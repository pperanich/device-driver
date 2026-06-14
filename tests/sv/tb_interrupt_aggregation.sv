// =============================================================================
// Functional testbench for the `interrupt_aggregation_apb3` wrapper (M5a).
//
// DUT (per `tests/ui/cases/interrupt_aggregation/input.ddsl`):
//   Status[3:0] = { link_down,  // bit3, negedge, group "link"
//                   link_up,    // bit2, bothedge, group "link"
//                   underflow,  // bit1, posedge, root group
//                   overflow }  // bit0, level,   root group
//   All fields are W1C (`on-write: clear`).
//
// Exercises:
//   - Level latches into storage and drives root `irq`
//   - Posedge fires only on a 0→1 transition (not steady-state high)
//   - Bothedge fires on either transition (group `link` → `irq_link`)
//   - Negedge fires only on a 1→0 transition
//   - Group routing: link_up + link_down feed `irq_link`, not root `irq`
//   - SW W1C clears the latched bit and the corresponding irq line drops
//
// Returns nonzero exit code on any failure ($error increments fail_count,
// $fatal at end propagates).
// =============================================================================
`timescale 1ns / 1ps

module tb_interrupt_aggregation;
  import interrupt_aggregation_pkg::*;

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

  // ---- Hwif + IRQs -----------------------------------------------------------
  interrupt_aggregation__out_t hwif_out;
  interrupt_aggregation__in_t  hwif_in;
  logic                        irq;
  logic                        irq_link;

  initial hwif_in = '0;

  // ---- DUT -------------------------------------------------------------------
  interrupt_aggregation_apb3 dut (
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
    .hwif_in (hwif_in),
    .irq     (irq),
    .irq_link(irq_link)
  );

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
    @(posedge pclk);

    check("post-reset irq low",      irq      == 1'b0);
    check("post-reset irq_link low", irq_link == 1'b0);

    // ---------------------------------------------------------------------
    // 1. Level trigger: overflow_intr held high → status[0] latches → root irq
    // ---------------------------------------------------------------------
    hwif_in.status_overflow_intr <= 1'b1;
    repeat (2) @(posedge pclk);
    check("overflow level irq",   irq      == 1'b1);
    check("overflow link clean",  irq_link == 1'b0);
    apb_read(8'h00, rd_val);
    check("overflow latched bit", rd_val[0] == 1'b1);

    // Clear it via SW W1C; raw still high but storage clears next cycle
    // until the level reasserts. We deassert raw first then clear, to
    // verify the W1C path itself.
    hwif_in.status_overflow_intr <= 1'b0;
    repeat (2) @(posedge pclk);
    apb_write(8'h00, 32'h0000_0001);
    repeat (2) @(posedge pclk);
    check("overflow cleared", irq == 1'b0);

    // ---------------------------------------------------------------------
    // 2. Posedge trigger: underflow_intr 0→1 fires once
    // ---------------------------------------------------------------------
    hwif_in.status_underflow_intr <= 1'b1;
    repeat (3) @(posedge pclk);
    check("underflow posedge irq", irq == 1'b1);
    apb_read(8'h00, rd_val);
    check("underflow latched bit", rd_val[1] == 1'b1);

    // Holding it high should not re-fire after clear (sticky bit stays
    // until W1C; once W1C clears the bit, posedge edge detector should not
    // re-fire because the raw input is still high without a 0→1).
    apb_write(8'h00, 32'h0000_0002);  // W1C bit 1
    repeat (3) @(posedge pclk);
    check("underflow holds clear with raw high", irq == 1'b0);

    hwif_in.status_underflow_intr <= 1'b0;
    repeat (2) @(posedge pclk);

    // ---------------------------------------------------------------------
    // 3. Bothedge group routing: link_up 0→1 fires on irq_link only
    // ---------------------------------------------------------------------
    hwif_in.status_link_up_intr <= 1'b1;
    repeat (3) @(posedge pclk);
    check("link_up bothedge irq_link",   irq_link == 1'b1);
    check("link_up does not pollute root", irq    == 1'b0);

    apb_write(8'h00, 32'h0000_0004);  // W1C bit 2
    repeat (2) @(posedge pclk);
    check("link_up cleared", irq_link == 1'b0);

    // 1→0 also fires bothedge
    hwif_in.status_link_up_intr <= 1'b0;
    repeat (3) @(posedge pclk);
    check("link_up bothedge on 1->0", irq_link == 1'b1);

    apb_write(8'h00, 32'h0000_0004);  // W1C bit 2
    repeat (2) @(posedge pclk);

    // ---------------------------------------------------------------------
    // 4. Negedge trigger: link_down fires only on 1→0
    // ---------------------------------------------------------------------
    hwif_in.status_link_down_intr <= 1'b1;
    repeat (3) @(posedge pclk);
    check("link_down posedge does NOT fire (negedge)",
          irq_link == 1'b0 && irq == 1'b0);

    hwif_in.status_link_down_intr <= 1'b0;
    repeat (3) @(posedge pclk);
    check("link_down negedge fires irq_link", irq_link == 1'b1);
    check("link_down stays out of root irq",  irq      == 1'b0);

    apb_write(8'h00, 32'h0000_0008);  // W1C bit 3
    repeat (2) @(posedge pclk);
    check("link_down cleared", irq_link == 1'b0);

    // Summary
    if (fail_count == 0) begin
      $display("[tb_interrupt_aggregation] PASS");
      $finish;
    end else begin
      $display("[tb_interrupt_aggregation] FAIL — %0d assertions failed", fail_count);
      $fatal;
    end
  end

  // Watchdog
  initial begin
    #50000;
    $display("[tb_interrupt_aggregation] timeout");
    $fatal;
  end

endmodule
