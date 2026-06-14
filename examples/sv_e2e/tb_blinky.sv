// =============================================================================
// Co-simulation harness for the SV `blinky` example.
//
// Drives the regblock directly via its native CPUIF (the APB3 wrapper
// would work too, but the CPUIF path is the smallest reproducible
// scenario for a doc example).
//
// Scenario:
//   1. SW reads Status before HW runs → 0.
//   2. SW writes Ctrl.enable=1.
//   3. User RTL pulses tick_we for 5 cycles, with tick values 1..5.
//   4. SW reads Status, checks tick == 5.
//   5. User RTL pulses overflow_hwset.
//   6. SW reads Status, checks overflow == 1.
//   7. SW writes Status with bit 0 set (W1C) → overflow clears.
//   8. SW reads Status, checks overflow == 0.
//
// PASS exits 0; any FAIL increments fail_count and the testbench
// $fatals at the end so CI catches it.
// =============================================================================
`timescale 1ns / 1ps

module tb_blinky;
  import blinky_pkg::*;

  logic clk = 0;
  always #5 clk = ~clk;
  logic rst_n = 0;

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

  blinky__out_t hwif_out;
  blinky__in_t  hwif_in;
  initial hwif_in = '0;

  blinky_regs dut (
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

  logic [CPUIF_DATA_W-1:0] rd_val;
  initial begin
    rst_n = 0;
    repeat (4) @(posedge clk);
    rst_n = 1;
    @(posedge clk);

    // 1. Read Status before HW runs.
    sw_read(BLINKY_STATUS_ADDR, rd_val);
    check("status @ reset", rd_val == 32'h0);

    // Period defaults to 0x100.
    sw_read(BLINKY_PERIOD_ADDR, rd_val);
    check("period reset value", rd_val == 32'h100);

    // 2. SW writes Ctrl.enable=1.
    sw_write(BLINKY_CTRL_ADDR, 32'h0000_0001, 32'h0000_0001);
    @(posedge clk);
    check("enable observed by user RTL", hwif_out.ctrl[0] == 1'b1);

    // 3. Drive 5 ticks (HW write into tick field, bits 31:1).
    for (int i = 1; i <= 5; i++) begin
      @(posedge clk);
      hwif_in.status_tick    <= 31'(i);
      hwif_in.status_tick_we <= 1'b1;
    end
    @(posedge clk);
    hwif_in.status_tick_we <= 1'b0;
    @(posedge clk);

    // 4. Read Status — tick should be 5 (bits 31:1).
    sw_read(BLINKY_STATUS_ADDR, rd_val);
    check("tick == 5", rd_val[31:1] == 31'h5);
    check("overflow still 0", rd_val[0] == 1'b0);

    // 5. Pulse overflow_hwset.
    @(posedge clk);
    hwif_in.status_overflow_hwset <= 1'b1;
    @(posedge clk);
    hwif_in.status_overflow_hwset <= 1'b0;
    @(posedge clk);

    // 6. SW sees overflow == 1.
    sw_read(BLINKY_STATUS_ADDR, rd_val);
    check("overflow set by hwset", rd_val[0] == 1'b1);

    // 7. SW W1C clears overflow.
    sw_write(BLINKY_STATUS_ADDR, 32'h0000_0001, 32'h0000_0001);
    @(posedge clk);

    // 8. Verify cleared.
    sw_read(BLINKY_STATUS_ADDR, rd_val);
    check("overflow cleared by W1C", rd_val[0] == 1'b0);

    if (fail_count == 0) begin
      $display("[tb_blinky] PASS");
      $finish;
    end else begin
      $display("[tb_blinky] FAIL — %0d assertions failed", fail_count);
      $fatal;
    end
  end

  initial begin
    #20000;
    $display("[tb_blinky] timeout");
    $fatal;
  end

endmodule
