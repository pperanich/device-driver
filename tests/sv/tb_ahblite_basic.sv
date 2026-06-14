// =============================================================================
// Functional testbench for the `ahblite_basic_ahblite` wrapper (M7e).
//
// Exercises (per `tests/ui/cases/ahblite_basic/input.ddsl`):
//   - AHB-Lite address-phase + data-phase transactions
//   - Write transaction (NONSEQ) into the Ctrl register
//   - Read transaction back via HRDATA
//   - ERROR response (HRESP=1) on access to an unmapped address
//
// Returns nonzero exit code on any failure ($error increments fail_count,
// $fatal at end propagates).
// =============================================================================
`timescale 1ns / 1ps

module tb_ahblite_basic;
  import ahblite_basic_pkg::*;

  // ---- Clock & reset ---------------------------------------------------------
  logic hclk = 0;
  always #5 hclk = ~hclk;
  logic hresetn = 0;

  // ---- AHB-Lite --------------------------------------------------------------
  logic                      hsel = 0;
  logic                      hwrite = 0;
  logic [1:0]                htrans = 2'b00;
  logic [2:0]                hsize = 3'b010;
  logic [2:0]                hburst = 3'b000;
  logic [CPUIF_ADDR_W-1:0]   haddr = 0;

  logic [CPUIF_DATA_W-1:0]   hwdata = 0;
  logic                      hready = 1;
  logic                      hreadyout;
  logic [CPUIF_DATA_W-1:0]   hrdata;
  logic                      hresp;

  // ---- Hwif ------------------------------------------------------------------
  ahblite_basic__out_t hwif_out;

  // ---- DUT -------------------------------------------------------------------
  ahblite_basic_ahblite dut (
    .hclk     (hclk),
    .hresetn  (hresetn),
    .hsel     (hsel),
    .hwrite   (hwrite),
    .htrans   (htrans),
    .hsize    (hsize),
    .hburst   (hburst),
    .haddr    (haddr),
    .hwdata   (hwdata),
    .hready   (hready),
    .hreadyout(hreadyout),
    .hrdata   (hrdata),
    .hresp    (hresp),
    .hwif_out (hwif_out)
  );

  // Sample hready as the master would: when hreadyout returns high, the data
  // phase completed and we can launch the next address phase.
  assign hready = hreadyout;

  // ---- Helpers ---------------------------------------------------------------
  int unsigned fail_count = 0;

  task automatic check(input string label, input bit cond);
    if (!cond) begin
      $error("FAIL [%s] @ t=%0t", label, $time);
      fail_count++;
    end
  endtask

  // Drive a NONSEQ transfer. Address phase one cycle, data phase next.
  // Returns the latched HRESP after the data-phase ack.
  task automatic ahb_write(input [CPUIF_ADDR_W-1:0] addr,
                           input [CPUIF_DATA_W-1:0] data,
                           output logic              resp);
    // Address phase
    @(posedge hclk);
    hsel    <= 1;
    hwrite  <= 1;
    htrans  <= 2'b10;   // NONSEQ
    haddr   <= addr;
    // Drop SEL/TRANS for next cycle to go IDLE
    @(posedge hclk);
    hsel    <= 0;
    hwrite  <= 0;
    htrans  <= 2'b00;
    // Drive data on data-phase cycle
    hwdata  <= data;
    // Wait for slave to ack via hreadyout (we sampled hready=hreadyout above)
    do @(posedge hclk); while (!hreadyout);
    resp = hresp;
    hwdata <= '0;
  endtask

  task automatic ahb_read(input [CPUIF_ADDR_W-1:0]  addr,
                          output [CPUIF_DATA_W-1:0] data,
                          output logic              resp);
    @(posedge hclk);
    hsel    <= 1;
    hwrite  <= 0;
    htrans  <= 2'b10;
    haddr   <= addr;
    @(posedge hclk);
    hsel    <= 0;
    htrans  <= 2'b00;
    do @(posedge hclk); while (!hreadyout);
    data = hrdata;
    resp = hresp;
  endtask

  // ---- Stimulus --------------------------------------------------------------
  logic [CPUIF_DATA_W-1:0] rd_val;
  logic                    resp;
  initial begin
    // Reset
    hresetn = 0;
    repeat (4) @(posedge hclk);
    hresetn = 1;
    @(posedge hclk);

    // 1. Write Ctrl.
    ahb_write(8'h00, 32'hABCD_0001, resp);
    check("write Ctrl OKAY", resp == 1'b0);

    // 2. Read back.
    ahb_read(8'h00, rd_val, resp);
    check("read Ctrl OKAY",   resp == 1'b0);
    check("read Ctrl enable", rd_val[0] == 1'b1);
    check("read Ctrl count",  rd_val[31:16] == 16'hABCD);

    // 3. Address miss → HRESP=1.
    ahb_write(8'hFF, 32'hDEAD_BEEF, resp);
    check("write ERROR on bad addr", resp == 1'b1);

    ahb_read(8'hFF, rd_val, resp);
    check("read ERROR on bad addr", resp == 1'b1);

    // Summary
    if (fail_count == 0) begin
      $display("[tb_ahblite_basic] PASS");
      $finish;
    end else begin
      $display("[tb_ahblite_basic] FAIL — %0d assertions failed", fail_count);
      $fatal;
    end
  end

  // Watchdog
  initial begin
    #20000;
    $display("[tb_ahblite_basic] timeout");
    $fatal;
  end

endmodule
