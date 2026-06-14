// =============================================================================
// Functional testbench for the `axi_4_lite_basic_axi4lite` wrapper (M4b).
//
// Exercises (per `tests/ui/cases/axi4lite_basic/input.ddsl`):
//   - AXI4-Lite write transaction (AW + W → B)
//   - AXI4-Lite read transaction (AR → R) with prior write's value
//   - SLVERR (2'b10) on access to an unmapped address
//   - WSTRB fanout: only strobed bytes update storage
//
// Returns nonzero exit code on any failure ($error increments fail_count,
// $fatal at end propagates).
// =============================================================================
`timescale 1ns / 1ps

module tb_axi4lite_basic;
  import axi_4_lite_basic_pkg::*;

  // ---- Clock & reset ---------------------------------------------------------
  logic aclk = 0;
  always #5 aclk = ~aclk;
  logic aresetn = 0;

  // ---- AXI4-Lite -------------------------------------------------------------
  logic                          awvalid = 0;
  logic                          awready;
  logic [CPUIF_ADDR_W-1:0]       awaddr = 0;
  logic [2:0]                    awprot = 0;

  logic                          wvalid = 0;
  logic                          wready;
  logic [CPUIF_DATA_W-1:0]       wdata = 0;
  logic [CPUIF_DATA_W/8-1:0]     wstrb = 0;

  logic                          bvalid;
  logic                          bready = 0;
  logic [1:0]                    bresp;

  logic                          arvalid = 0;
  logic                          arready;
  logic [CPUIF_ADDR_W-1:0]       araddr = 0;
  logic [2:0]                    arprot = 0;

  logic                          rvalid;
  logic                          rready = 0;
  logic [CPUIF_DATA_W-1:0]       rdata;
  logic [1:0]                    rresp;

  // ---- Hwif ------------------------------------------------------------------
  axi_4_lite_basic__out_t hwif_out;

  // ---- DUT -------------------------------------------------------------------
  axi_4_lite_basic_axi4lite dut (
    .aclk    (aclk),
    .aresetn (aresetn),
    .awvalid (awvalid),
    .awready (awready),
    .awaddr  (awaddr),
    .awprot  (awprot),
    .wvalid  (wvalid),
    .wready  (wready),
    .wdata   (wdata),
    .wstrb   (wstrb),
    .bvalid  (bvalid),
    .bready  (bready),
    .bresp   (bresp),
    .arvalid (arvalid),
    .arready (arready),
    .araddr  (araddr),
    .arprot  (arprot),
    .rvalid  (rvalid),
    .rready  (rready),
    .rdata   (rdata),
    .rresp   (rresp),
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

  task automatic axi_write(input [CPUIF_ADDR_W-1:0]   addr,
                           input [CPUIF_DATA_W-1:0]   data,
                           input [CPUIF_DATA_W/8-1:0] strb,
                           output logic [1:0]         resp);
    // Drive AW + W together (in-order, single outstanding).
    @(posedge aclk);
    awvalid <= 1;
    awaddr  <= addr;
    wvalid  <= 1;
    wdata   <= data;
    wstrb   <= strb;
    do @(posedge aclk); while (!(awvalid && awready));
    awvalid <= 0;
    wvalid  <= 0;

    // Wait for B and capture resp.
    bready <= 1;
    do @(posedge aclk); while (!bvalid);
    resp = bresp;
    bready <= 0;
  endtask

  task automatic axi_read(input [CPUIF_ADDR_W-1:0]  addr,
                          output [CPUIF_DATA_W-1:0] data,
                          output logic [1:0]        resp);
    @(posedge aclk);
    arvalid <= 1;
    araddr  <= addr;
    do @(posedge aclk); while (!(arvalid && arready));
    arvalid <= 0;

    rready <= 1;
    do @(posedge aclk); while (!rvalid);
    data = rdata;
    resp = rresp;
    rready <= 0;
  endtask

  // ---- Stimulus --------------------------------------------------------------
  logic [CPUIF_DATA_W-1:0] rd_val;
  logic [1:0]              resp;
  initial begin
    // Reset
    aresetn = 0;
    repeat (4) @(posedge aclk);
    aresetn = 1;
    @(posedge aclk);

    // 1. Write Ctrl: enable=1, count=0xABCD (bits 31:16). All strobes
    //    enabled — full-word write.
    axi_write(8'h00, 32'hABCD_0001, 4'hF, resp);
    check("write Ctrl OKAY", resp == 2'b00);

    // 2. Read back.
    axi_read(8'h00, rd_val, resp);
    check("read Ctrl OKAY",   resp == 2'b00);
    check("read Ctrl enable", rd_val[0] == 1'b1);
    check("read Ctrl count",  rd_val[31:16] == 16'hABCD);

    // 3. Partial-strobe write — only the top half should change.
    //    Mark wstrb = 4'b1100 (bytes 2 and 3 = bits 31:16).
    axi_write(8'h00, 32'h1234_FFFF, 4'b1100, resp);
    check("partial write OKAY",      resp == 2'b00);
    axi_read(8'h00, rd_val, resp);
    check("partial preserved lo",    rd_val[0] == 1'b1);
    check("partial updated hi",      rd_val[31:16] == 16'h1234);

    // 4. Address miss → SLVERR on B.
    axi_write(8'hFF, 32'hDEAD_BEEF, 4'hF, resp);
    check("write SLVERR on bad addr", resp == 2'b10);

    // 5. Address miss → SLVERR on R.
    axi_read(8'hFF, rd_val, resp);
    check("read SLVERR on bad addr", resp == 2'b10);

    // Summary
    if (fail_count == 0) begin
      $display("[tb_axi4lite_basic] PASS");
      $finish;
    end else begin
      $display("[tb_axi4lite_basic] FAIL — %0d assertions failed", fail_count);
      $fatal;
    end
  end

  // Watchdog
  initial begin
    #20000;
    $display("[tb_axi4lite_basic] timeout");
    $fatal;
  end

endmodule
