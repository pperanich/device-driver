// =============================================================================
// Functional testbench for the `apb_3_basic_apb3` wrapper (M4a/M4b).
//
// Exercises (per `tests/ui/cases/apb3_basic/input.ddsl`):
//   - APB3 write of the Ctrl register
//   - APB3 read-back round-trip
//   - PSLVERR raised on access to an unmapped address
//
// Returns nonzero exit code on any failure ($error increments fail_count,
// $fatal at end propagates).
// =============================================================================
`timescale 1ns / 1ps

module tb_apb3_basic;
  import apb_3_basic_pkg::*;

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
  apb_3_basic__out_t hwif_out;

  // ---- DUT (uses APB3 wrapper) -----------------------------------------------
  apb_3_basic_apb3 dut (
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
    .hwif_out(hwif_out)
  );

  // ---- Helpers ---------------------------------------------------------------
  int unsigned fail_count = 0;

  task automatic check(input string label, input bit cond);
    if (!cond) begin
      $error("FAIL [%s] @ t=%0t", label, $time);
      fail_count++;
    end
  endtask

  // Standard 2-phase APB3 transfer: SETUP cycle (psel asserted, penable
  // low) → ACCESS cycle (penable asserted, slave responds with pready).
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

  task automatic apb_read(input [CPUIF_ADDR_W-1:0] addr,
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

    // 1. Write Ctrl: enable=1, mode=0b100, threshold=0xABCD.
    //    Layout: bit 0 = enable, bits 3:1 = mode, bits 15:4 reserved,
    //            bits 31:16 = threshold.
    apb_write(8'h00, 32'hABCD_0009);
    check("write Ctrl no err", pslverr == 1'b0);

    // 2. Read back via APB; storage should hold what we wrote in the named
    //    bit positions. (Reserved bits read as 0 — RD_MASK gates them.)
    apb_read(8'h00, rd_val);
    check("read Ctrl no err", pslverr == 1'b0);
    check("read Ctrl enable", rd_val[0]  == 1'b1);
    check("read Ctrl mode",   rd_val[3:1] == 3'b100);
    check("read Ctrl thresh", rd_val[31:16] == 16'hABCD);

    // 3. Address miss → PSLVERR.
    apb_write(8'hFF, 32'hDEAD_BEEF);
    check("PSLVERR on bad addr", pslverr == 1'b1);

    // Summary
    if (fail_count == 0) begin
      $display("[tb_apb3_basic] PASS");
      $finish;
    end else begin
      $display("[tb_apb3_basic] FAIL — %0d assertions failed", fail_count);
      $fatal;
    end
  end

  // Watchdog
  initial begin
    #20000;
    $display("[tb_apb3_basic] timeout");
    $fatal;
  end

endmodule
