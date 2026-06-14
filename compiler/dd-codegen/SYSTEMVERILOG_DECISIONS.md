# SystemVerilog Backend — Locked Decisions (v1)

This document fixes the architectural decisions for the SystemVerilog code-generation backend. Each decision is locked before implementation begins; revisiting any of them mid-build is expensive and requires explicit sign-off.

Scope: the regblock RTL, bus adapter wrappers, SVA checker, and UVM RAL package emitted from a single DDSL source. Companion to `book/` design docs.

---

## Decision 1 — Internal CPUIF: per-bit `wr_biten`, no `wstrb`

**Decision.** The canonical internal control-port bundle used by the generated regblock is:

```
input  logic                          clk,
input  logic                          rst_n,

// Request channel
input  logic                          cpuif_req,
input  logic                          cpuif_req_is_wr,
input  logic [CPUIF_ADDR_W-1:0]       cpuif_addr,
input  logic [CPUIF_DATA_W-1:0]       cpuif_wr_data,
input  logic [CPUIF_DATA_W-1:0]       cpuif_wr_biten,    // per-BIT enable, not byte strobe

// Read response
output logic                          cpuif_rd_ack,
output logic                          cpuif_rd_err,
output logic [CPUIF_DATA_W-1:0]       cpuif_rd_data,

// Write response
output logic                          cpuif_wr_ack,
output logic                          cpuif_wr_err,
```

**Rationale.** Per-bit `wr_biten` makes field-level masking a single `wr_biten[hi:lo]` slice in the template. With `wstrb` we would have to expand byte-strobes to bit-masks inside every field's update expression — multiply that by W1C / W1S / W1T variants and the template combinatorial explosion is painful. Bus adapter wrappers fan out `wstrb` to `wr_biten` (replicate each `wstrb[i]` across 8 bits) in their own thin wrapper. This is the peakrdl-regblock convention and the prior art is battle-tested.

**Alternative considered.** Byte-strobe (`wstrb`) at the regblock boundary. Rejected — pushes byte-to-bit fanout into every field template.

---

## Decision 2 — Split `rd_ack` / `wr_ack`, in-order responses only

**Decision.** Read and write responses use separate single-cycle ack pulses (`cpuif_rd_ack`, `cpuif_wr_ack`) with paired error flags (`cpuif_rd_err`, `cpuif_wr_err`). No transaction IDs. No out-of-order responses. At most one outstanding request per direction.

**Rationale.** A CSR block has no meaningful concurrency between reads and writes. Split-ack lets us pipeline a read behind a write (or vice versa) without an ID-tracking FIFO. In-order with no IDs keeps the generator's combinational decode logic trivial and keeps bus adapters thin.

**Alternative considered.** Single `cpuif_ack` for both directions, or AXI-style ID-tracked responses. Rejected — single ack forces serialization at the adapter; ID tracking is gratuitous for register access patterns.

---

## Decision 3 — Reset: synchronous active-low `rst_n`

**Decision.** Generated regblock uses a single clock `clk` and a single active-low synchronous reset `rst_n`. All flops use `always_ff @(posedge clk) if (!rst_n) ... else ...`.

**Rationale.** Active-low is the de-facto industry default; matches APB/AHB/AXI conventions out of the box (no inverter in adapters). Synchronous reset is simpler to analyze for static timing and more portable across FPGA/ASIC targets than async reset with explicit deassertion synchronizer.

**Alternative considered.** Async-reset (`always_ff @(posedge clk or negedge rst_n)`). Deferred to v1.x as a `sv-reset-style:` opt-in. Active-high reset is rejected outright as a v1 option.

---

## Decision 4 — Naming conventions

**Decision.** Lock the following identifiers, derived from the DDSL `device` name in `snake_case` unless otherwise stated:

| Artifact | Identifier |
|----------|------------|
| Top regblock module | `<dev>_regs` |
| Package | `<dev>_pkg` |
| Bus adapter wrapper | `<dev>_<bus>` (e.g. `foo_apb3`, `foo_axi4lite`) |
| SVA checker module | `<dev>_sva` |
| Bind site | `bind <dev>_regs <dev>_sva u_chk (.*);` |
| UVM RAL package | `<dev>_ral_pkg` |
| UVM register block class | `<dev>_reg_block` |
| Hwif structs | `<dev>__in_t`, `<dev>__out_t` (double underscore reserves namespace) |
| Address constants | `<DEV>_<REG>_ADDR` (SHOUTY_SNAKE) |

**Rationale.** The generator owns both RTL and verification artifacts. One naming source of truth eliminates `add_hdl_path_slice` drift in RAL. Double-underscore separator on hwif structs is a peakrdl idiom that nests cleanly through blocks.

**Alternative considered.** Per-target name overrides via DDSL attribute. Deferred — adds DSL surface for marginal value.

---

## Decision 5 — Bus adapters: APB3 + APB4 + AXI4-Lite + native (v1)

**Decision.** v1 ships four bus adapter templates: `apb3`, `apb4`, `axi4lite`, `native` (no adapter — pass CPUIF through). The DDSL `sv-bus:` property accepts a list; one wrapper file is emitted per requested adapter.

```
sv-bus: [apb3, axi4lite]
```

**Rationale.** APB3/APB4 are the lowest-effort to template (no out-of-order channels). AXI4-Lite is the SoC standard. `native` covers the case where the user instantiates the regblock under an existing bus wrapper of their own. AHB-Lite slips to v1.1 — its `HREADY` shared-bus semantics complicate testing and the protocol is in slow decline. AXI4-full is rejected from v1; user instantiates Xilinx/ARM `axi_to_axi_lite` shim.

**Alternative considered.** Single bus per generator invocation. Rejected — common case is a regblock that needs to be portable across two interconnects on one chip.

---

## Decision 6 — Error response: SLVERR only

**Decision.** Bus adapters return SLVERR (APB) / SLVERR (AXI) on:
- Write to RO field bits (when entire field is RO and `wr_biten` overlaps)
- Read from WO field bits
- Access to an address inside the regblock range that decodes to no register
- Burst on AXI4-Lite (protocol violation)

DECERR is never asserted by the regblock — that response belongs to the interconnect for out-of-range addresses outside the regblock's allocation.

**Rationale.** Single error class simplifies adapter state machines and matches what most synthesis-clean CSR blocks do in practice. DECERR is interconnect-layer.

**Alternative considered.** Silent acceptance of writes to RO. Rejected — masks software bugs; explicit error is more debuggable.

---

## Decision 7 — Interrupts: separate `mask` and `enable`, multi-group output

**Decision.** Interrupt-source fields generate the canonical six-signal chain: `raw_status` → `status` (latch, optionally sticky) → `enable` (gates latching) → `mask` (gates output) → `pending` (RO, `status & ~mask`) → `irq_out` (OR-reduce of `pending`). `mask` and `enable` are kept distinct and each get an auto-generated companion register.

DSL allows `group:` attribute on `intr` fields to partition into multiple output IRQ lines:

```
intr: { trigger: level, group: "err" }
intr: { trigger: posedge, group: "normal" }
```

Codegen emits one `irq_<group>` output per distinct group plus `irq` for ungrouped sources. Top-level device knob `intr-aggregate: false` disables the OR-reduce and exposes individual pending bits instead.

**Rationale.** The mask-vs-enable distinction is semantically important (mask hides the IRQ but lets events accumulate; enable prevents events from being recorded). Collapsing them is the most common source of post-silicon firmware-vs-RTL disagreements; we will not propagate that confusion.

**Alternative considered.** Single combined `enable_mask`. Rejected on correctness grounds.

---

## Decision 8 — SVA: separate checker module, bound; granular opt-in

**Decision.** Assertions are emitted in `<dev>_sva.sv` as a standalone module bound to the regblock:

```sv
bind <dev>_regs <dev>_sva u_chk (.*);
```

The bind statement is emitted as a separate `<dev>_sva_bind.svh` snippet for the user to `include` in their testbench. Granular opt-in via:

```
sv-assertions: {
    reset: true,
    decode_mutex: true,
    w1c: true,
    ro_invariance: false
}
```

v1 default-on assertions: `reset`, `decode_mutex`. v1 default-off: `w1c`, `ro_invariance` (high churn, opt-in once user has codegen confidence).

**Rationale.** `bind` keeps the RTL synthesizable and lint-clean without `ifdef SVA_ON` clutter. Separating the bind snippet from the checker module lets users place the bind in a testbench package without touching delivered RTL.

**Alternative considered.** Inline `ifdef` blocks in the regblock. Rejected — clutters generated RTL and forces every downstream consumer to handle the ifdef.

**Portability subset.** Assertions restrict to: `assert property (@(posedge clk) disable iff (!rst_n) <ant> |=> <con>)` using only `$past`, `$stable`, `$onehot`, `$onehot0`, `$countones`. No `$isunknown` in FV-targeted properties. No `first_match`, no recursive properties, no local variables.

---

## Decision 9 — UVM RAL: separate file, snapshot + smoke seq, no coverage

**Decision.** RAL package emitted to `<dev>_ral_pkg.sv`, opt-out via `sv-ral: false`. Package contains `uvm_reg_field` / `uvm_reg` / `uvm_reg_block` subclasses; backdoor paths set via `add_hdl_path_slice` using the locked naming convention from Decision 4. Coverage groups (`uvm_reg_field::include_coverage`) are out of v1.

CI strategy: golden-file snapshot only. No UVM-aware sim in OSS CI (license cost). Smoke sequence emitted as `<dev>_ral_smoke_seq.sv` documents the intended exercise pattern; runs under user's local UVM sim.

**Rationale.** Compile-only verification with a UVM-aware sim requires Questa/Xcelium/VCS. Snapshot tests catch structural regressions; UVM correctness is the user's testbench concern.

**Alternative considered.** Bundle RAL with RTL in single file. Rejected — RAL is verification-only; downstream synth flows reject UVM imports.

---

## Decision 10 — AXI4-full is not in v1; macro stays Rust-only

**Decision.**
- AXI4-full bus adapter is **out of v1**. Document the external shim path (`axi_to_axi_lite` from ARM / Xilinx / open implementations).
- `device-driver-macros` (proc-macro entry) remains Rust-only. Proc-macro cannot meaningfully emit SV files alongside Rust into the consumer's build tree.
- CLI multi-target: `dd-cli build -t rust -t systemverilog <src>` compiles once, emits both. Output writer must handle the multi-file case (SV target emits ≥2 files: package + module + optional adapter/checker/RAL).

**Rationale.** AXI4-full bursts, transaction IDs, narrow-burst byte-lane logic, and 4KB-boundary checks roughly double the wrapper LOC for marginal benefit at the CSR boundary. WASM target similarly extended to multi-file output (`HashMap<String, String>` return type).

**Alternative considered.** Emit AXI4-full with bursts-rejected stub. Deferred — half-implementations age badly.

---

## Open issues not locked here

- **`sv-data-width` granularity** — 32 vs 64. v1 default 32. Wider data buses (128/256) deferred.
- **Multi-clock domains** — not in v1. All registers on `clk`.
- **CDC for HW-side interfaces** — not in v1. User RTL handles CDC outside the regblock.
- **Companion FIFO status register address allocation** — explicit-or-next-free with diagnostic on collision (see Decision E in plan).

## Tooling assumptions

- **Lint:** Verilator ≥ 5.0 in CI (`verilator --lint-only -Wall -sv`). Not currently installed in the dev environment — CI workflow update is a prerequisite to Milestone M2.
- **FV (opt-in):** SymbiYosys + Yosys via the `sv-assertions` opt-in CI job.
- **UVM sim:** out of OSS CI scope; snapshot-only.
