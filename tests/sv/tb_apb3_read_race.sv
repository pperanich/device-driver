// =============================================================================
// APB3 wrapper read-path stress.
//
// Targets the wrapper race fixed by registering `cpuif_rd_data` inside
// the regs module: with a combinational read mux and
// `cpuif_req = psel & penable & ~pready`, when pready rises cpuif_req
// drops to 0 → all rd_hit_* collapse → cpuif_rd_data goes to '0 the
// same cycle the master samples prdata.
//
// Three scenarios chase that race:
//   (1) Three back-to-back reads of distinct registers, expect each
//       prdata to match the corresponding register's reset value
//       (0xAAAAAAAA / 0xBBBBBBBB / 0xCCCCCCCC).
//   (2) Read each register with one idle cycle between transactions —
//       previous read's leftover data must not bleed into the next.
//   (3) Write-then-read of the same register, no idle cycles, expect
//       the just-written value (validates that the registered output
//       does not stale-fetch).
// =============================================================================
`timescale 1ns / 1ps

module tb_apb3_read_race;
  import apb_3_read_race_pkg::*;

  logic pclk = 0;
  always #5 pclk = ~pclk;
  logic presetn = 0;

  logic                      psel    = 0;
  logic                      penable = 0;
  logic                      pwrite  = 0;
  logic [CPUIF_ADDR_W-1:0]   paddr   = 0;
  logic [CPUIF_DATA_W-1:0]   pwdata  = 0;
  logic [CPUIF_DATA_W-1:0]   prdata;
  logic                      pready;
  logic                      pslverr;

  apb_3_read_race_apb3 dut (
    .pclk    (pclk),
    .presetn (presetn),
    .psel    (psel),
    .penable (penable),
    .pwrite  (pwrite),
    .paddr   (paddr),
    .pwdata  (pwdata),
    .prdata  (prdata),
    .pready  (pready),
    .pslverr (pslverr)
  );

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

  logic [CPUIF_DATA_W-1:0] rd_val;
  initial begin
    presetn = 0;
    repeat (4) @(posedge pclk);
    presetn = 1;
    @(posedge pclk);

    // ------------------------------------------------------------------
    // (1) Back-to-back reads of three registers. Each must return its
    //     own reset value. Before the fix, the first read would have
    //     returned '0 because cpuif_rd_data collapsed mid-cycle.
    // ------------------------------------------------------------------
    apb_read(APB_3_READ_RACE_REG_A_ADDR, rd_val);
    check("RegA b2b reset",    rd_val == 32'hAAAA_AAAA);
    apb_read(APB_3_READ_RACE_REG_B_ADDR, rd_val);
    check("RegB b2b reset",    rd_val == 32'hBBBB_BBBB);
    apb_read(APB_3_READ_RACE_REG_C_ADDR, rd_val);
    check("RegC b2b reset",    rd_val == 32'hCCCC_CCCC);

    // ------------------------------------------------------------------
    // (2) Reads with idle cycles between. Tests that the registered
    //     output isn't holding stale data from a previous transaction.
    // ------------------------------------------------------------------
    apb_read(APB_3_READ_RACE_REG_C_ADDR, rd_val);
    check("RegC idle-spaced", rd_val == 32'hCCCC_CCCC);
    repeat (5) @(posedge pclk);
    apb_read(APB_3_READ_RACE_REG_A_ADDR, rd_val);
    check("RegA after idle",  rd_val == 32'hAAAA_AAAA);
    repeat (3) @(posedge pclk);
    apb_read(APB_3_READ_RACE_REG_B_ADDR, rd_val);
    check("RegB after idle",  rd_val == 32'hBBBB_BBBB);

    // ------------------------------------------------------------------
    // (3) Write-then-read of the same register, no idle cycles.
    //     The bus must surface the just-written value, not the previous
    //     reset or '0.
    // ------------------------------------------------------------------
    apb_write(APB_3_READ_RACE_REG_A_ADDR, 32'hDEAD_BEEF);
    apb_read (APB_3_READ_RACE_REG_A_ADDR, rd_val);
    check("RegA write-then-read", rd_val == 32'hDEAD_BEEF);

    apb_write(APB_3_READ_RACE_REG_B_ADDR, 32'h1234_5678);
    apb_read (APB_3_READ_RACE_REG_B_ADDR, rd_val);
    check("RegB write-then-read", rd_val == 32'h1234_5678);

    if (fail_count == 0) begin
      $display("[tb_apb3_read_race] PASS");
      $finish;
    end else begin
      $display("[tb_apb3_read_race] FAIL — %0d assertions failed", fail_count);
      $fatal;
    end
  end

  initial begin
    #20000;
    $display("[tb_apb3_read_race] timeout");
    $fatal;
  end

endmodule
