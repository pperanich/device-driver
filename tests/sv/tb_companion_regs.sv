// =============================================================================
// Functional testbench for the SystemVerilog `companion_regs` case.
//
// Exercises (per `tests/ui/cases/companion_regs/input.ddsl`):
//   - Companion register synthesis: `<group>_intr_enable` and
//     `<group>_intr_mask` for both the root group and `err` group.
//   - IRQ output gating: `irq_<group> = storage_status[bit] &
//     enable[bit] & ~mask[bit]`. Disabling enable kills the IRQ; setting
//     mask kills the IRQ even with enable still on.
//   - Level vs posedge intr-trigger arms: `overflow` is level (latches on
//     `hwif_in.status_overflow_intr` while held); `underflow` is posedge
//     (latches on the rising edge only).
//   - SW W1C of the latched storage bit clears the IRQ.
//   - FIFO status companion register: user RTL drives `_we` strobes for
//     `level`, `full`, `empty`, `almost_full` fields; SW reads return
//     the latched composite value.
//
// Returns nonzero exit code on any failure.
// =============================================================================
`timescale 1ns / 1ps

module tb_companion_regs;
  import companion_regs_pkg::*;

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
  companion_regs__out_t hwif_out;
  companion_regs__in_t  hwif_in;
  initial hwif_in = '0;

  // ---- IRQ outputs -----------------------------------------------------------
  logic irq, irq_err;

  // ---- DUT -------------------------------------------------------------------
  companion_regs_regs dut (
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
    .hwif_in        (hwif_in),
    .irq            (irq),
    .irq_err        (irq_err)
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

  logic [CPUIF_DATA_W-1:0] rd_val;
  initial begin
    // Reset
    rst_n = 0;
    repeat (4) @(posedge clk);
    rst_n = 1;
    @(posedge clk);

    // ------------------------------------------------------------------
    // After reset: storage all 0; all IRQ outputs low; companion enable/
    // mask both 0 so even a fired intr stays gated.
    // ------------------------------------------------------------------
    check("reset: irq low",     irq     == 1'b0);
    check("reset: irq_err low", irq_err == 1'b0);
    check("reset: enable=0",    hwif_out.root_intr_enable[0] == 1'b0);
    check("reset: mask=0",      hwif_out.root_intr_mask[0]   == 1'b0);

    // ------------------------------------------------------------------
    // 1. Fire `overflow` (level) — storage[0] latches on the next cycle,
    //    but enable[0] = 0 so irq still 0.
    // ------------------------------------------------------------------
    hwif_in.status_overflow_intr = 1'b1;
    @(posedge clk);
    @(posedge clk);
    check("overflow latched", hwif_out.status[0] == 1'b1);
    check("irq gated (enable=0)", irq == 1'b0);

    // ------------------------------------------------------------------
    // 2. SW sets root_intr_enable[0] = 1 → irq goes high.
    // ------------------------------------------------------------------
    sw_write(COMPANION_REGS_ROOT_INTR_ENABLE_ADDR, 32'h0000_0001, 32'h0000_0001);
    @(posedge clk);
    check("root enable=1, irq high", irq == 1'b1);

    // ------------------------------------------------------------------
    // 3. SW sets root_intr_mask[0] = 1 → irq forced low even though
    //    storage and enable still 1.
    // ------------------------------------------------------------------
    sw_write(COMPANION_REGS_ROOT_INTR_MASK_ADDR, 32'h0000_0001, 32'h0000_0001);
    @(posedge clk);
    check("root mask=1, irq forced low", irq == 1'b0);
    check("storage still set under mask", hwif_out.status[0] == 1'b1);

    // ------------------------------------------------------------------
    // 4. Clear the mask, then W1C the storage bit → irq returns to 0.
    // ------------------------------------------------------------------
    sw_write(COMPANION_REGS_ROOT_INTR_MASK_ADDR, 32'h0000_0000, 32'h0000_0001);
    @(posedge clk);
    check("mask cleared, irq high again", irq == 1'b1);

    // Drop the source so the level-trigger doesn't re-latch immediately.
    hwif_in.status_overflow_intr = 1'b0;
    @(posedge clk);
    // SW W1C — write 1 to bit 0 clears it (on-write: clear).
    sw_write(COMPANION_REGS_STATUS_ADDR, 32'h0000_0001, 32'h0000_0001);
    @(posedge clk);
    check("storage cleared via W1C", hwif_out.status[0] == 1'b0);
    check("irq low after W1C",       irq == 1'b0);

    // ------------------------------------------------------------------
    // 5. `underflow` (posedge) — a sustained high level should latch the
    //    bit exactly once on the rising edge. After clearing the bit, a
    //    second rising edge re-latches it; holding high without a new
    //    edge should NOT relatch.
    // ------------------------------------------------------------------
    // First rising edge:
    hwif_in.status_underflow_intr = 1'b1;
    @(posedge clk);
    @(posedge clk);
    check("underflow posedge latches",  hwif_out.status[1] == 1'b1);

    // err group enable + mask separately exposed:
    sw_write(COMPANION_REGS_ERR_INTR_ENABLE_ADDR, 32'h0000_0001, 32'h0000_0001);
    @(posedge clk);
    check("irq_err high (err enable=1, mask=0)", irq_err == 1'b1);
    check("root irq unaffected by err group",     irq == 1'b0);

    sw_write(COMPANION_REGS_ERR_INTR_MASK_ADDR, 32'h0000_0001, 32'h0000_0001);
    @(posedge clk);
    check("err mask=1 gates irq_err", irq_err == 1'b0);

    // ------------------------------------------------------------------
    // 6. FIFO status companion register: user RTL drives _we strobes for
    //    each field; SW read returns the composite value.
    // ------------------------------------------------------------------
    @(posedge clk);
    hwif_in.data_fifo_status_level       <= 5'h0B;
    hwif_in.data_fifo_status_level_we    <= 1'b1;
    hwif_in.data_fifo_status_full        <= 1'b0;
    hwif_in.data_fifo_status_full_we     <= 1'b1;
    hwif_in.data_fifo_status_empty       <= 1'b0;
    hwif_in.data_fifo_status_empty_we    <= 1'b1;
    hwif_in.data_fifo_status_almost_full <= 1'b1;
    hwif_in.data_fifo_status_almost_full_we <= 1'b1;
    @(posedge clk);
    hwif_in.data_fifo_status_level_we       <= 1'b0;
    hwif_in.data_fifo_status_full_we        <= 1'b0;
    hwif_in.data_fifo_status_empty_we       <= 1'b0;
    hwif_in.data_fifo_status_almost_full_we <= 1'b0;
    @(posedge clk);

    sw_read(COMPANION_REGS_DATA_FIFO_STATUS_ADDR, rd_val);
    check("fifo status: level",         rd_val[4:0] == 5'h0B);
    check("fifo status: full",          rd_val[5]   == 1'b0);
    check("fifo status: empty",         rd_val[6]   == 1'b0);
    check("fifo status: almost_full",   rd_val[7]   == 1'b1);

    // Now flip full=1 + drop almost_full; expect new composite value.
    @(posedge clk);
    hwif_in.data_fifo_status_full           <= 1'b1;
    hwif_in.data_fifo_status_full_we        <= 1'b1;
    hwif_in.data_fifo_status_almost_full    <= 1'b0;
    hwif_in.data_fifo_status_almost_full_we <= 1'b1;
    @(posedge clk);
    hwif_in.data_fifo_status_full_we        <= 1'b0;
    hwif_in.data_fifo_status_almost_full_we <= 1'b0;
    @(posedge clk);

    sw_read(COMPANION_REGS_DATA_FIFO_STATUS_ADDR, rd_val);
    check("fifo status: full=1",        rd_val[5] == 1'b1);
    check("fifo status: almost_full=0", rd_val[7] == 1'b0);

    // ---- Summary -------------------------------------------------------------
    if (fail_count == 0) begin
      $display("[tb_companion_regs] PASS");
      $finish;
    end else begin
      $display("[tb_companion_regs] FAIL — %0d assertions failed", fail_count);
      $fatal;
    end
  end

  // Hang-up watchdog
  initial begin
    #20000;
    $display("[tb_companion_regs] timeout");
    $fatal;
  end

endmodule
