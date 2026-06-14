// =============================================================================
// Functional testbench for the `buffer_fifo_apb3` wrapper (M6b).
//
// DUT (per `tests/ui/cases/buffer_fifo/input.ddsl`):
//   buffer DataFifo — address 0x10, access RW, hw-kind: fifo, depth: 16
//
// The regblock itself does not contain FIFO storage — the TB provides a
// trivial 16-deep synchronous FIFO behind the hwif. The TB then drives
// the bus and verifies push/pop ordering and value preservation.
//
// Exercises:
//   - APB3 write at the buffer address pushes onto the user FIFO
//   - APB3 read at the buffer address pops the head value
//   - FIFO returns values in-order (verifies push/pop wiring + decoder)
//   - Bus access at an unmapped address still raises PSLVERR
// =============================================================================
`timescale 1ns / 1ps

module tb_buffer_fifo;
  import buffer_fifo_pkg::*;

  // ---- Clock & reset ---------------------------------------------------------
  logic pclk = 0;
  always #5 pclk = ~pclk;
  logic presetn = 0;

  // ---- APB3 ------------------------------------------------------------------
  logic                      psel = 0;
  logic                      penable = 0;
  logic                      pwrite = 0;
  logic [CPUIF_ADDR_W-1:0]   paddr = 0;
  logic [CPUIF_DATA_W-1:0]   pwdata = 0;

  logic [CPUIF_DATA_W-1:0]   prdata;
  logic                      pready;
  logic                      pslverr;

  // ---- Hwif ------------------------------------------------------------------
  buffer_fifo__out_t hwif_out;
  buffer_fifo__in_t  hwif_in;

  // ---- DUT -------------------------------------------------------------------
  buffer_fifo_apb3 dut (
    .pclk    (pclk),
    .presetn (presetn),
    .psel    (psel),
    .penable (penable),
    .pwrite  (pwrite),
    .paddr   (paddr),
    .pwdata  (pwdata),
    .prdata  (prdata),
    .pready  (pready),
    .pslverr (pslverr),
    .hwif_out(hwif_out),
    .hwif_in (hwif_in)
  );

  // ---- User-side 16-deep FIFO (the regblock does not own storage) ----------
  localparam int FIFO_DEPTH = 16;
  logic [CPUIF_DATA_W-1:0] fifo_mem [0:FIFO_DEPTH-1];
  logic [4:0]              wr_ptr = 5'd0;
  logic [4:0]              rd_ptr = 5'd0;
  wire  [4:0]              level  = wr_ptr - rd_ptr;
  wire                     empty  = (level == 5'd0);
  wire                     full   = (level == FIFO_DEPTH);

  always_ff @(posedge pclk) begin
    if (!presetn) begin
      wr_ptr <= 5'd0;
      rd_ptr <= 5'd0;
    end else begin
      if (hwif_out.buf_data_fifo_push && !full) begin
        fifo_mem[wr_ptr[3:0]] <= hwif_out.buf_data_fifo_wdata;
        wr_ptr <= wr_ptr + 5'd1;
      end
      if (hwif_out.buf_data_fifo_pop && !empty) begin
        rd_ptr <= rd_ptr + 5'd1;
      end
    end
  end

  // Combinational read head — what the bus sees on the read cycle.
  assign hwif_in.buf_data_fifo_rdata = fifo_mem[rd_ptr[3:0]];

  // ---- Helpers ---------------------------------------------------------------
  int unsigned fail_count = 0;

  task automatic check(input string label, input bit cond);
    if (!cond) begin
      $error("FAIL [%s] @ t=%0t", label, $time);
      fail_count++;
    end
  endtask

  task automatic apb_write(input [CPUIF_ADDR_W-1:0] addr,
                           input [CPUIF_DATA_W-1:0] data);
    @(posedge pclk);
    psel    <= 1;
    penable <= 0;
    pwrite  <= 1;
    paddr   <= addr;
    pwdata  <= data;
    @(posedge pclk);
    penable <= 1;
    do @(posedge pclk); while (!pready);
    psel    <= 0;
    penable <= 0;
    pwrite  <= 0;
  endtask

  task automatic apb_read(input [CPUIF_ADDR_W-1:0]  addr,
                          output [CPUIF_DATA_W-1:0] data);
    @(posedge pclk);
    psel    <= 1;
    penable <= 0;
    pwrite  <= 0;
    paddr   <= addr;
    @(posedge pclk);
    penable <= 1;
    do @(posedge pclk); while (!pready);
    data = prdata;
    psel    <= 0;
    penable <= 0;
  endtask

  // ---- Stimulus --------------------------------------------------------------
  logic [CPUIF_DATA_W-1:0] rd_val;
  initial begin
    // Reset
    presetn = 0;
    repeat (4) @(posedge pclk);
    presetn = 1;
    @(posedge pclk);

    // 1. Push three values via the bus.
    apb_write(8'h10, 32'hDEAD_0001);
    check("push #1 no err", pslverr == 1'b0);
    apb_write(8'h10, 32'hDEAD_0002);
    check("push #2 no err", pslverr == 1'b0);
    apb_write(8'h10, 32'hDEAD_0003);
    check("push #3 no err", pslverr == 1'b0);
    repeat (2) @(posedge pclk);
    check("level after 3 pushes", level == 5'd3);

    // 2. Pop them back in FIFO order.
    apb_read(8'h10, rd_val);
    check("pop #1 no err", pslverr == 1'b0);
    check("pop #1 value",  rd_val == 32'hDEAD_0001);
    apb_read(8'h10, rd_val);
    check("pop #2 value",  rd_val == 32'hDEAD_0002);
    apb_read(8'h10, rd_val);
    check("pop #3 value",  rd_val == 32'hDEAD_0003);
    repeat (2) @(posedge pclk);
    check("level after drain", level == 5'd0);

    // 3. Address miss → PSLVERR.
    apb_write(8'hFF, 32'hBADC_AFE);
    check("PSLVERR on bad addr write", pslverr == 1'b1);
    apb_read(8'hFF, rd_val);
    check("PSLVERR on bad addr read",  pslverr == 1'b1);

    // Summary
    if (fail_count == 0) begin
      $display("[tb_buffer_fifo] PASS");
      $finish;
    end else begin
      $display("[tb_buffer_fifo] FAIL — %0d assertions failed", fail_count);
      $fatal;
    end
  end

  // Watchdog
  initial begin
    #20000;
    $display("[tb_buffer_fifo] timeout");
    $fatal;
  end

endmodule
