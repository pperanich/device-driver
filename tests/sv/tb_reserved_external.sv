// =============================================================================
// Functional testbench for the SystemVerilog `reserved_external` case.
//
// Exercises (per `tests/ui/cases/reserved_external/input.ddsl`):
//   - `reserved-behavior: ro_zero` (default) — reserved bits read as 0,
//     SW writes to reserved bits ignored.
//   - `reserved-behavior: ro_preserve` — reserved bits keep their reset
//     value (0xDEADBEEF >> 1 in this case, since bit 0 is the `enable`
//     field).
//   - `reserved-behavior: rw_storage` — reserved bits accept SW writes
//     like a hidden RW field and round-trip on subsequent reads.
//   - `external: allow` — bus reads/writes pass through to user RTL via
//     `hwif_out.<reg>_ext_wr_hit`/`ext_wr_data` and
//     `hwif_in.<reg>_ext_rd_data`. No storage flop inside the regblock.
//
// Returns nonzero exit code on any failure.
// =============================================================================
`timescale 1ns / 1ps

module tb_reserved_external;
  import reserved_external_pkg::*;

  // ---- Clock & reset ---------------------------------------------------------
  logic clk = 0;
  always #5 clk = ~clk;
  logic rst_n = 0;

  // ---- CPUIF -----------------------------------------------------------------
  logic                      cpuif_req       = 0;
  logic                      cpuif_req_is_wr = 0;
  logic [CPUIF_ADDR_W-1:0]   cpuif_addr      = 0;
  logic [CPUIF_DATA_W-1:0]   cpuif_wr_data   = 0;
  logic [CPUIF_DATA_W-1:0]   cpuif_wr_biten  = 0;

  logic                      cpuif_rd_ack;
  logic                      cpuif_rd_err;
  logic [CPUIF_DATA_W-1:0]   cpuif_rd_data;
  logic                      cpuif_wr_ack;
  logic                      cpuif_wr_err;

  // ---- Hwif ------------------------------------------------------------------
  reserved_external__out_t hwif_out;
  reserved_external__in_t  hwif_in;

  initial hwif_in = '0;

  // ---- DUT -------------------------------------------------------------------
  reserved_external_regs dut (
    .clk            (clk),
    .rst_n          (rst_n),
    .cpuif_req      (cpuif_req),
    .cpuif_req_is_wr(cpuif_req_is_wr),
    .cpuif_addr     (cpuif_addr),
    .cpuif_wr_data  (cpuif_wr_data),
    .cpuif_wr_biten (cpuif_wr_biten),
    .cpuif_rd_ack   (cpuif_rd_ack),
    .cpuif_rd_err   (cpuif_rd_err),
    .cpuif_rd_data  (cpuif_rd_data),
    .cpuif_wr_ack   (cpuif_wr_ack),
    .cpuif_wr_err   (cpuif_wr_err),
    .hwif_out       (hwif_out),
    .hwif_in        (hwif_in)
  );

  // ---- Helpers ---------------------------------------------------------------
  int unsigned fail_count = 0;

  task automatic check(input string label, input bit cond);
    if (!cond) begin
      $error("FAIL [%s] @ t=%0t", label, $time);
      fail_count++;
    end
  endtask

  task automatic sw_write(input [CPUIF_ADDR_W-1:0] addr,
                          input [CPUIF_DATA_W-1:0] data,
                          input [CPUIF_DATA_W-1:0] biten);
    @(posedge clk);
    cpuif_req       <= 1;
    cpuif_req_is_wr <= 1;
    cpuif_addr      <= addr;
    cpuif_wr_data   <= data;
    cpuif_wr_biten  <= biten;
    @(posedge clk);
    cpuif_req       <= 0;
    cpuif_req_is_wr <= 0;
    cpuif_wr_data   <= '0;
    cpuif_wr_biten  <= '0;
  endtask

  task automatic sw_read(input [CPUIF_ADDR_W-1:0] addr,
                         output [CPUIF_DATA_W-1:0] data);
    @(posedge clk);
    cpuif_req       <= 1;
    cpuif_req_is_wr <= 0;
    cpuif_addr      <= addr;
    @(posedge clk);
    cpuif_req       <= 0;
    data            = cpuif_rd_data;
  endtask

  // ---- Stimulus --------------------------------------------------------------
  logic [CPUIF_DATA_W-1:0] rd_val;
  initial begin
    // Reset
    rst_n = 0;
    repeat (4) @(posedge clk);
    rst_n = 1;
    @(posedge clk);

    // ------------------------------------------------------------------
    // 1. ro_zero (CtrlZero): reset value 0; reserved bits 31:1 read as 0
    //    even if SW tried to set them. Reset value is 0 so this is
    //    trivially correct — focus on the write-then-read round-trip.
    // ------------------------------------------------------------------
    sw_write(RESERVED_EXTERNAL_CTRL_ZERO_ADDR, 32'hFFFF_FFFF, 32'hFFFF_FFFF);
    @(posedge clk);
    sw_read(RESERVED_EXTERNAL_CTRL_ZERO_ADDR, rd_val);
    check("ro_zero: enable wrote",          rd_val[0]    == 1'b1);
    check("ro_zero: reserved reads as 0",   rd_val[31:1] == 31'h0);

    // ------------------------------------------------------------------
    // 2. ro_preserve (CtrlPreserve): reset 0xDEADBEEF. Reserved bits keep
    //    that pattern except for bit 0 (the `enable` field). SW write to
    //    reserved bits should be ignored.
    // ------------------------------------------------------------------
    sw_read(RESERVED_EXTERNAL_CTRL_PRESERVE_ADDR, rd_val);
    check("ro_preserve: enable @ reset", rd_val[0]    == 1'b1);
    // 0xDEADBEEF[31:1] == 0x6F56DF77
    check("ro_preserve: reserved @ reset", rd_val[31:1] == 31'h6F56DF77);

    sw_write(RESERVED_EXTERNAL_CTRL_PRESERVE_ADDR, 32'h0000_0000, 32'hFFFF_FFFE);
    @(posedge clk);
    sw_read(RESERVED_EXTERNAL_CTRL_PRESERVE_ADDR, rd_val);
    check("ro_preserve: reserved unchanged after sw write",
          rd_val[31:1] == 31'h6F56DF77);

    // ------------------------------------------------------------------
    // 3. rw_storage (Scratch): reset 0. `tag` is 7:0; reserved bits 31:8
    //    accept SW writes like a hidden RW field.
    // ------------------------------------------------------------------
    sw_write(RESERVED_EXTERNAL_SCRATCH_ADDR, 32'hCAFE_BA42, 32'hFFFF_FFFF);
    @(posedge clk);
    sw_read(RESERVED_EXTERNAL_SCRATCH_ADDR, rd_val);
    check("rw_storage: tag round-trip",        rd_val[7:0]  == 8'h42);
    check("rw_storage: reserved round-trip",   rd_val[31:8] == 24'hCAFEBA);

    // Write with biten masking reserved bits — they hold; tag changes.
    sw_write(RESERVED_EXTERNAL_SCRATCH_ADDR, 32'h0000_0099, 32'h0000_00FF);
    @(posedge clk);
    sw_read(RESERVED_EXTERNAL_SCRATCH_ADDR, rd_val);
    check("rw_storage: biten masks reserved",  rd_val[31:8] == 24'hCAFEBA);
    check("rw_storage: tag updated",           rd_val[7:0]  == 8'h99);

    // ------------------------------------------------------------------
    // 4. external (ExtCtrl): no storage. Bus writes pulse ext_wr_hit +
    //    drive ext_wr_data; bus reads sample user-driven ext_rd_data.
    // ------------------------------------------------------------------
    hwif_in.ext_ctrl_ext_rd_data = 32'h1234_5678;
    sw_read(RESERVED_EXTERNAL_EXT_CTRL_ADDR, rd_val);
    check("external: read returns ext_rd_data", rd_val == 32'h1234_5678);

    // Drive a bus write and check the wr_hit + wr_data appear on hwif_out
    // for one cycle (the strobe pulse).
    @(posedge clk);
    cpuif_req       <= 1;
    cpuif_req_is_wr <= 1;
    cpuif_addr      <= RESERVED_EXTERNAL_EXT_CTRL_ADDR;
    cpuif_wr_data   <= 32'hA5A5_5A5A;
    cpuif_wr_biten  <= 32'hFFFF_FFFF;
    @(posedge clk);
    // Sample combinationally on the same cycle the bus presents the request:
    // ext_wr_hit is a continuous assignment of wr_hit_ext_ctrl.
    check("external: ext_wr_hit pulses",   hwif_out.ext_ctrl_ext_wr_hit  == 1'b1);
    check("external: ext_wr_data forwards", hwif_out.ext_ctrl_ext_wr_data == 32'hA5A5_5A5A);
    cpuif_req       <= 0;
    cpuif_req_is_wr <= 0;
    cpuif_wr_data   <= '0;
    cpuif_wr_biten  <= '0;
    @(posedge clk);
    check("external: ext_wr_hit de-asserts", hwif_out.ext_ctrl_ext_wr_hit == 1'b0);

    // ------------------------------------------------------------------
    // 5. Address miss → SLVERR on the unused address 0xFE.
    // ------------------------------------------------------------------
    sw_write(8'hFE, 32'hDEAD_BEEF, 32'hFFFF_FFFF);
    @(posedge clk);
    check("addr-miss SLVERR (wr_err)", cpuif_wr_err == 1'b1);

    // ---- Summary -------------------------------------------------------------
    if (fail_count == 0) begin
      $display("[tb_reserved_external] PASS");
      $finish;
    end else begin
      $display("[tb_reserved_external] FAIL — %0d assertions failed", fail_count);
      $fatal;
    end
  end

  // Hang-up watchdog
  initial begin
    #20000;
    $display("[tb_reserved_external] timeout");
    $fatal;
  end

endmodule
