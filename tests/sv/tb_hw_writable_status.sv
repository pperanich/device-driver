// =============================================================================
// Functional testbench for the SystemVerilog `hw_writable_status` case.
//
// Exercises (per `tests/ui/cases/hw_writable_status/input.ddsl`):
//   - reset values
//   - SW write + read of the `counter` field
//   - HW write to `counter` via the `_we` strobe
//   - HW>SW precedence on the same cycle
//   - HW set + SW W1C on the `overflow` sticky flag
//   - HW write to `hw_armed` (HW-WO + SW-RO) — visible on hwif_out, not via SW
//   - SW-only RW on `scratch`
//   - Address-miss SLVERR on `cpuif_*_err`
//
// Returns nonzero exit code on any failure ($error increments $error count,
// $finish at the end propagates it).
// =============================================================================
`timescale 1ns / 1ps

module tb_hw_writable_status;
  import hw_writable_status_pkg::*;

  // ---- Clock & reset ---------------------------------------------------------
  logic clk = 0;
  always #5 clk = ~clk;
  logic rst_n = 0;

  // ---- CPUIF -----------------------------------------------------------------
  logic                      cpuif_req = 0;
  logic                      cpuif_req_is_wr = 0;
  logic [CPUIF_ADDR_W-1:0]   cpuif_addr = 0;
  logic [CPUIF_DATA_W-1:0]   cpuif_wr_data = 0;
  logic [CPUIF_DATA_W-1:0]   cpuif_wr_biten = 0;

  logic                      cpuif_rd_ack;
  logic                      cpuif_rd_err;
  logic [CPUIF_DATA_W-1:0]   cpuif_rd_data;
  logic                      cpuif_wr_ack;
  logic                      cpuif_wr_err;

  // ---- Hwif ------------------------------------------------------------------
  hw_writable_status__out_t hwif_out;
  hw_writable_status__in_t  hwif_in;

  // Default-zero everything; tasks drive specific strobes for one cycle.
  initial hwif_in = '0;

  // ---- DUT -------------------------------------------------------------------
  hw_writable_status_regs dut (
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

    // 1. After reset, storage should be zero (counter, overflow, hw_armed,
    //    scratch). hwif_out reflects raw storage.
    check("reset hwif_out", hwif_out.status == '0);

    // 2. SW writes counter = 0x42 (bits 7:0). overflow + hw_armed are masked
    //    out by per-field writability (overflow is W1C — write 0 clears
    //    nothing; hw_armed is SW-RO — write ignored).
    sw_write(HW_WRITABLE_STATUS_STATUS_ADDR, 32'h0000_0042, 32'h0000_00FF);
    @(posedge clk);
    check("sw counter sticks", hwif_out.status[7:0] == 8'h42);
    check("sw didn't touch overflow", hwif_out.status[8] == 1'b0);
    check("sw didn't touch hw_armed", hwif_out.status[9] == 1'b0);

    // 3. SW reads back: counter visible, hw_armed is SW-RO so still visible
    //    (RO != WO), scratch == 0.
    sw_read(HW_WRITABLE_STATUS_STATUS_ADDR, rd_val);
    check("sw read counter", rd_val[7:0] == 8'h42);
    check("sw read scratch", rd_val[15:12] == 4'h0);

    // 4. HW asserts the counter _we strobe with value 0xAA — should override
    //    the prior storage on the next clk edge (HW write arm in cascade).
    @(posedge clk);
    hwif_in.status_counter    <= 8'hAA;
    hwif_in.status_counter_we <= 1'b1;
    @(posedge clk);
    hwif_in.status_counter_we <= 1'b0;
    @(posedge clk);
    check("hw write counter", hwif_out.status[7:0] == 8'hAA);

    // 5. Same-cycle SW+HW write — HW>SW precedence: HW value wins.
    @(posedge clk);
    cpuif_req                 <= 1;
    cpuif_req_is_wr           <= 1;
    cpuif_addr                <= HW_WRITABLE_STATUS_STATUS_ADDR;
    cpuif_wr_data             <= 32'h0000_0055;
    cpuif_wr_biten            <= 32'h0000_00FF;
    hwif_in.status_counter    <= 8'hCC;
    hwif_in.status_counter_we <= 1'b1;
    @(posedge clk);
    cpuif_req                 <= 0;
    cpuif_req_is_wr           <= 0;
    cpuif_wr_data             <= '0;
    cpuif_wr_biten            <= '0;
    hwif_in.status_counter_we <= 1'b0;
    @(posedge clk);
    check("HW>SW precedence", hwif_out.status[7:0] == 8'hCC);

    // 6. HW sets overflow via _we; SW W1C clears it.
    @(posedge clk);
    hwif_in.status_overflow    <= 1'b1;
    hwif_in.status_overflow_we <= 1'b1;
    @(posedge clk);
    hwif_in.status_overflow_we <= 1'b0;
    @(posedge clk);
    check("hw set overflow", hwif_out.status[8] == 1'b1);

    // SW W1C — write 1 to overflow bit clears it (on-write: clear).
    sw_write(HW_WRITABLE_STATUS_STATUS_ADDR, 32'h0000_0100, 32'h0000_0100);
    @(posedge clk);
    check("sw W1C clears overflow", hwif_out.status[8] == 1'b0);

    // 7. HW writes hw_armed (HW-WO + SW-RO). SW write to bit 9 must NOT change it.
    @(posedge clk);
    hwif_in.status_hw_armed    <= 1'b1;
    hwif_in.status_hw_armed_we <= 1'b1;
    @(posedge clk);
    hwif_in.status_hw_armed_we <= 1'b0;
    @(posedge clk);
    check("hw write hw_armed", hwif_out.status[9] == 1'b1);

    sw_write(HW_WRITABLE_STATUS_STATUS_ADDR, 32'h0000_0000, 32'h0000_0200);
    @(posedge clk);
    check("sw can't clear hw_armed", hwif_out.status[9] == 1'b1);

    // SW read includes hw_armed (sw-RO is readable).
    sw_read(HW_WRITABLE_STATUS_STATUS_ADDR, rd_val);
    check("sw read hw_armed", rd_val[9] == 1'b1);

    // 8. SW writes scratch bits 15:12 = 0xA; should stick.
    sw_write(HW_WRITABLE_STATUS_STATUS_ADDR, 32'h0000_A000, 32'h0000_F000);
    @(posedge clk);
    check("sw scratch sticks", hwif_out.status[15:12] == 4'hA);

    // 9. Address miss → SLVERR.
    sw_write(8'hFF, 32'hDEAD_BEEF, 32'hFFFF_FFFF);
    @(posedge clk);
    check("addr-miss SLVERR (wr_err)", cpuif_wr_err == 1'b1);

    // Summary
    if (fail_count == 0) begin
      $display("[tb_hw_writable_status] PASS");
      $finish;
    end else begin
      $display("[tb_hw_writable_status] FAIL — %0d assertions failed", fail_count);
      $fatal;
    end
  end

  // Hang-up watchdog
  initial begin
    #20000;
    $display("[tb_hw_writable_status] timeout");
    $fatal;
  end

endmodule
