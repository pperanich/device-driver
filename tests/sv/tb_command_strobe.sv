// =============================================================================
// Functional testbench for the `command_strobe_apb3` wrapper (M6a).
//
// DUT (per `tests/ui/cases/command_strobe/input.ddsl`):
//   command ReadId — address 0x04, hw-handshake: strobe
//     fields-in:  op  [7:0]   (1 byte)
//     fields-out: id  [31:0]  (4 bytes)
//
// Exercises:
//   - APB3 write at the command address pulses `hwif_out.cmd_read_id_strobe`
//     for one cycle and surfaces `hwif_out.cmd_read_id_in` to the bus's
//     wdata[7:0] during that cycle
//   - HW asserting `hwif_in.cmd_read_id_resp_valid` + `cmd_read_id_resp`
//     latches the value into the response storage word
//   - APB3 read at the same address returns the latched response
//   - Address miss raises PSLVERR
//
// Returns nonzero exit code on any failure ($error increments fail_count,
// $fatal at end propagates).
// =============================================================================
`timescale 1ns / 1ps

module tb_command_strobe;
  import command_strobe_pkg::*;

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
  command_strobe__out_t hwif_out;
  command_strobe__in_t  hwif_in;

  initial hwif_in = '0;

  // ---- DUT -------------------------------------------------------------------
  command_strobe_apb3 dut (
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

  // ---- Helpers ---------------------------------------------------------------
  int unsigned fail_count = 0;

  task automatic check(input string label, input bit cond);
    if (!cond) begin
      $error("FAIL [%s] @ t=%0t", label, $time);
      fail_count++;
    end
  endtask

  // Snapshot strobe + cmd_in on each cycle so we can verify they pulsed
  // exactly when we expected.
  logic strobe_seen;
  logic [7:0] last_cmd_in;
  always_ff @(posedge pclk) begin
    if (!presetn) begin
      strobe_seen <= 1'b0;
      last_cmd_in <= 8'h00;
    end else if (hwif_out.cmd_read_id_strobe) begin
      strobe_seen <= 1'b1;
      last_cmd_in <= hwif_out.cmd_read_id_in;
    end
  end

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

    // 1. APB write at command address (0x04) with op = 0xAB. Should pulse
    //    strobe and expose op on cmd_in.
    apb_write(8'h04, 32'h0000_00AB);
    check("write to cmd no err", pslverr == 1'b0);
    repeat (2) @(posedge pclk);
    check("strobe seen", strobe_seen == 1'b1);
    check("cmd_in latched", last_cmd_in == 8'hAB);

    // 2. HW returns response payload. Pulse resp_valid for one cycle.
    @(posedge pclk);
    hwif_in.cmd_read_id_resp       <= 32'hCAFE_BABE;
    hwif_in.cmd_read_id_resp_valid <= 1'b1;
    @(posedge pclk);
    hwif_in.cmd_read_id_resp_valid <= 1'b0;
    hwif_in.cmd_read_id_resp       <= '0;
    repeat (2) @(posedge pclk);

    // 3. APB read at command address returns the latched response.
    apb_read(8'h04, rd_val);
    check("read cmd no err",   pslverr == 1'b0);
    check("read cmd response", rd_val == 32'hCAFE_BABE);

    // 4. Address miss → PSLVERR.
    apb_write(8'hFF, 32'hDEAD_BEEF);
    check("PSLVERR on bad addr write", pslverr == 1'b1);
    apb_read(8'hFF, rd_val);
    check("PSLVERR on bad addr read", pslverr == 1'b1);

    // Summary
    if (fail_count == 0) begin
      $display("[tb_command_strobe] PASS");
      $finish;
    end else begin
      $display("[tb_command_strobe] FAIL — %0d assertions failed", fail_count);
      $fatal;
    end
  end

  // Watchdog
  initial begin
    #20000;
    $display("[tb_command_strobe] timeout");
    $fatal;
  end

endmodule
