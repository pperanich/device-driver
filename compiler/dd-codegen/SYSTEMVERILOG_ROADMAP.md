# SystemVerilog Backend — Roadmap

Living document. Captures the multi-milestone plan for the SystemVerilog code-generation backend, current status, and the work remaining. Intended to be readable cold, without context from any prior conversation. Pairs with `SYSTEMVERILOG_DECISIONS.md` (architectural choices locked in M1).

## North Star

One DDSL source → six artifacts:

1. Rust HAL (unchanged behaviour — the original target)
2. SV regblock module (`<dev>_regs.sv`)
3. SV package (`<dev>_pkg.sv`) with typedefs + address consts
4. Bus adapter wrapper(s) (`<dev>_<bus>.sv`) per requested protocol
5. SVA checker module (`<dev>_sva.sv`) + `bind` snippet
6. UVM RAL package (`<dev>_ral_pkg.sv`)

**Zero regression on Rust target.** All HW semantics opt-in via `hw-*` / `sv-*` / new field-level attrs. The Rust template carries the new LIR fields but ignores them; existing Rust tests stay green at every milestone.

## Status board

| Milestone | Scope | Commit | UI test pairs |
|-----------|-------|--------|---------------|
| **M1** | Spec doc + Target enum + CLI wiring + first templates | `ef3d272` | — (decisions only) |
| **M2** | Native CPUIF; package + module templates; reset values; per-field access masks; register array unroll; address-miss SLVERR; snapshot test harness; verilator-lint CI | `ef3d272` | `basic_register` |
| **M3a** | `on-write: store/clear/set/toggle` (W1C/W1S/W1T) | `ef3d272` | `on_write_modifiers` |
| **M3b** | `on-read: store/clear/set` (RC/RS) | `ef3d272` | `on_read_modifiers` |
| **M3c** | `hw-access: RW/RO/WO` + `hwif_in` port + per-field always_ff cascade refactor | `ef3d272` | `hw_writable_status` |
| **M3d** | `hw-clr` / `hw-set` strobes; `singlepulse` auto-clear | `ef3d272` | `advanced_hw_modifiers` |
| **M3e** | `precedence: hw|sw` override; Verilator functional TB + CI sim job | `39fb980` | `precedence_override` |
| **M4a** | Multi-file output API + native + APB3 wrapper | `e6dfb37` | `apb3_basic` |
| **M4b** | APB4 + AXI4-Lite wrappers + Verilator TBs | `f91b4fc` | `apb4_basic`, `axi4lite_basic` |
| **M5a** | `intr-trigger` + `intr-group` interrupt aggregation + IRQ OR-reduce outputs | `b53761c` | `interrupt_aggregation` |
| **M5b** | `sv-assert-*` SVA checker module + bind snippet | `0d252d0` | `sva_basic` |
| **M6a** | `command` `hw-handshake: strobe` HW mapping | `01dd037` | `command_strobe` |
| **M6b** | `buffer` `hw-kind: fifo` HW mapping (bus-side handshake; no storage) | `7659fc4` | `buffer_fifo` |
| **M6c** | UVM RAL package (`<dev>_ral_pkg.sv`) | `85aeaaf` | `ral_basic` |
| **M7a** | AXI4-Lite asserts, `intr-no-aggregate`, `sv-data-width` DSL, RC/RS UVM | `9f621d2` | — |
| **M7b** | `reserved-behavior` (3 policies) + `external` register | `a701db9` | `reserved_external` |
| **M7c** | Buffer `status-address`/`words` + `bus_compat_checked` MIR pass | `8282092` | — |
| **M7d** | Companion register synthesis (IRQ enable/mask + FIFO status) | `2faee23` | `companion_regs` |
| **M7e** | AHB-Lite v1.1 wrapper | `504a379` | `ahblite_basic` |
| **M7f** | UVM RAL smoke sequence (`<dev>_ral_smoke_seq.sv`) | `3cd30f9` | — |
| **M7g** | SymbiYosys CI job for SVA snapshot | — | — |
| **M7h** | LIR-level collision detection for synthesized companion registers | `fb9ada1` | — |
| **U1**  | Real `bus_compat_checked` (width validation, ahblite+64 conflict) + spanned data-width diagnostic | `a9c155a` | `sv_data_width_invalid`, `sv_bus_width_conflict` |
| **U2**  | Functional TBs for M7b (reserved/external) + M7d (companion regs) | `bd5e94e` | — (TBs only) |
| **U3**  | Explicit `intr-enable-bit:` / `intr-mask-bit:` (item 8 option a) | `f1b127c` | `intr_explicit_bits` |
| **U4**  | SV user guide + hwif contract + e2e blinky example | `2ae40a2` | — (docs + example) |
| **U5**  | pkg/regs file split + sv-ral default flip to opt-out | this commit | — |

Current: **39 UI snapshot tests pass.** Rust target unchanged. Verilator CI jobs: `verilator-lint` (lint every snapshot SV) + `verilator-sim` (build + run TBs for `hw_writable_status`, `apb3_basic`, `axi4lite_basic`, `interrupt_aggregation`, `command_strobe`, `buffer_fifo`, `ahblite_basic`). Best-effort `sva-formal` job runs SymbiYosys against the M5b checker.

## Locked architectural decisions

See `SYSTEMVERILOG_DECISIONS.md`. Summary:

| # | Decision |
|---|----------|
| 1 | Internal CPUIF uses per-bit `cpuif_wr_biten`, not byte `wstrb` |
| 2 | Split `rd_ack` / `wr_ack`, in-order, no transaction IDs |
| 3 | Synchronous active-low `rst_n` |
| 4 | `<dev>_regs`, `<dev>_pkg`, `<dev>_sva`, `<dev>_ral_pkg`, `<dev>_reg_block`, `<dev>__in_t` / `<dev>__out_t` |
| 5 | v1 buses: APB3 + APB4 + AXI4-Lite + native (AHB-Lite v1.1; AXI4-full out) |
| 6 | Error response: SLVERR only; never DECERR from regblock |
| 7 | Interrupts: separate `mask` and `enable`; multi-group output |
| 8 | SVA: separate checker module, bound; granular opt-in |
| 9 | UVM RAL: separate file, snapshot + smoke seq, no coverage |
| 10 | AXI4-full not in v1; macro stays Rust-only; CLI multi-target |

## Architecture as built (post M3)

### Compiler pipeline

```
DDSL source
   ↓ dd-lexer (logos)               adds Token::OnWrite, Token::Precedence
   ↓ dd-parser (chumsky)            adds Expression::OnWrite, Precedence + accessors
   ↓ dd-mir/lowering                Field gains optional on_write/on_read/hw_access/
                                     hw_clr/hw_set/singlepulse/precedence; PropertyInfo
                                     setters dispatch by `name: value`
   ↓ dd-mir/passes                  on_write_semantics_valid, on_read_semantics_valid
   ↓ dd-lir/lowering                LIR Field gets concrete defaults applied
   ↓ dd-codegen
      ├─ rust.rs (unchanged)        emits Rust HAL ignoring new fields
      └─ systemverilog.rs           emits SV (3 askama templates)
```

### Generated SV file structure (M3 output)

Single output stream containing both package and module:

```
// banner
package <dev>_pkg;
  localparam CPUIF_ADDR_W, CPUIF_DATA_W
  typedef <fs>_t (per fieldset)             // field-name-typed view
  localparam <DEV>_<REG>_<idx>_ADDR (per instance)
  typedef <dev>__out_t                       // raw register storage view
  typedef <dev>__in_t (conditional)          // per HW-writable field + strobes
endpackage

module <dev>_regs import <dev>_pkg::*; (
  clk, rst_n,
  cpuif_req, cpuif_req_is_wr, cpuif_addr, cpuif_wr_data, cpuif_wr_biten,
  cpuif_rd_ack, cpuif_rd_err, cpuif_rd_data,
  cpuif_wr_ack, cpuif_wr_err,
  hwif_out, hwif_in (conditional)
);
  // RD_MASK_<REG> per register (WO bits zero on read)
  // storage_<reg>[_<idx>] (per instance)
  // wr_hit_/rd_hit_<reg>[_<idx>] (per instance)
  // one always_ff PER FIELD, precedence-ordered cascade
  // any_hit OR-reduce → SLVERR on miss
  // read mux + ack/err pipelining
  // hwif_out assignments
endmodule
```

### Per-field cascade (heart of M3)

For each field, `field_flop` in `systemverilog.rs` emits one `always_ff` whose arms are chosen and ordered per field properties:

**`precedence: hw` (default):**
```
reset → hwclr → hwset → hw_we → sw_write → sw_read → singlepulse
```

**`precedence: sw`:**
```
reset → sw_write → sw_read → hwclr → hwset → hw_we → singlepulse
```

Reset is always first. Singlepulse is always last (catch-all auto-clear). Arms whose conditions don't apply for the field are omitted entirely; the storage slice holds if no arm fires.

## Current DSL surface

### Existing (pre-SystemVerilog work)

```
device <Name> {
    byte-order: LE|BE,
    register-address-type: u8|u16|u32|u64|i8|i16|i32|i64,
    command-address-type: ...,
    buffer-address-type: ...,
    register-address-mode: mapped|indexed,
    word-boundaries: "...",

    register <Name>[<count>*<stride>] {
        address: <n>,
        access: RW|RO|WO,
        reset: <n>,
        fields: fieldset <Name> { size-bytes: <n>, field <name> <bits> -> <type>, ... }
    },
    command <Name> { ... in/out fieldsets ... },
    buffer <Name> { access: RW|RO|WO, address: <n> }
}
```

### Added by M2–M3e (all optional, SV-target-effective, Rust-target-inert)

Field-level (in the `{ ... }` after `field <name> [access] <bits> -> <type>`):

```
field foo 0 -> bool {
    on-write:  store | clear | set | toggle   // W1C/W1S/W1T
    on-read:   store | clear | set            // RC/RS (toggle rejected with diagnostic)
    hw-access: RW | RO | WO                   // default RO; RW/WO emit hwif_in + _we strobe
    hw-clr:    allow                          // adds hwif_in.<f>_hwclr strobe
    hw-set:    allow                          // adds hwif_in.<f>_hwset strobe
    singlepulse: allow                        // auto-clear next cycle (catch-all arm)
    precedence: hw | sw                       // cascade arm ordering (default hw)
}
```

### Pending (M4+)

Device-level:
- `sv-bus: [apb3, apb4, axi4-lite, native]` — list of adapter wrappers to emit
- `sv-data-width: 32|64` — CPUIF data bus width (default 32)
- `intr-aggregate: true|false` — OR-reduce irq output (default true)
- `sv-assertions: { reset: bool, decode_mutex: bool, w1c: bool, ro_invariance: bool }`
- `sv-ral: bool` — emit UVM RAL package (default true)
- `sv-hdl-path-prefix: "..."` — RAL backdoor path

Field-level:
- `intr: { trigger: level|posedge|negedge|bothedge, sticky: bool, group: <ident>, mask: bool, enable: bool }` — interrupt source

Register-level:
- `reserved-behavior: ro_zero | ro_preserve | rw_storage` (default `ro_zero`)
- `external` — no storage, expose handshake to user RTL

Command/Buffer-level:
- `command Foo { hw-handshake: strobe | valid-ready | fifo, depth-in: N, depth-out: N }`
- `buffer Foo { hw-kind: fifo | stream | bram, depth: N, status-address: 0xNN, words: N }`

## M4 — Bus adapters

**Goal.** Templated wrappers around the regblock's native CPUIF, so the user can drop the regblock into APB3/APB4/AXI4-Lite/native interconnect without hand-writing glue.

**v1 protocols:** APB3 + APB4 + AXI4-Lite + native passthrough. AHB-Lite slips to v1.1. AXI4-full uses external Xilinx/ARM `axi_to_axi_lite` shim.

**Sub-tasks (proposed):**

| # | Item | Notes |
|---|------|-------|
| M4.1 | DSL: `sv-bus: [<list>]` device-level property accepting a list of identifiers from `apb3|apb4|axi4lite|native` | Reuses identifier-list parsing; default `[native]` |
| M4.2 | MIR field on `DeviceConfig`: `sv_bus: Vec<SvBus>` | LIR mirrors as `Vec<SvBus>` on the block |
| M4.3 | MIR pass `bus_compat_checked` | Width + addr-width must match; refuse unsupported combos |
| M4.4 | Codegen: emit one `<dev>_<bus>.sv` per requested adapter | New `cpuif/<bus>.sv.j2` templates |
| M4.5 | APB3 wrapper template | psel/penable/pready/pslverr handshake → CPUIF |
| M4.6 | APB4 wrapper template | APB3 + pprot + pstrb (pstrb → wr_biten fanout) |
| M4.7 | AXI4-Lite wrapper template | AW/W/B + AR/R channels; in-order; SLVERR on unmapped |
| M4.8 | Native passthrough | Trivial — just rename. Documented |
| M4.9 | Burst rejection on AXI4-Lite | Already protocol-forbidden; emit assert to catch violations |
| M4.10 | Multi-file output API | `dd-codegen::codegen` returns `Vec<(String, String)>` or `HashMap<String, String>`. CLI writes each to an output dir |
| M4.11 | CLI: `-o <dir>` becomes a directory for multi-file SV output | Keep single-file behaviour for Rust target |
| M4.12 | Snapshot tests for each bus | Per case, accept emits `<name>__<bus>.sv` alongside `<name>.sv` |
| M4.13 | Verilator TB for APB3 wrapper | Drive PADDR/PWDATA, verify same scenarios as native CPUIF |
| M4.14 | CI: extend `verilator-lint` + `verilator-sim` to cover wrapper files | Sweep over each adapter |

**Acceptance:** A single DDSL case can request `sv-bus: [apb3, axi4lite]`, regenerate accept-mode emits both wrapper files, both lint clean, both pass per-bus functional TB.

**Architectural risk:** Multi-file output is a real API change to `dd-codegen`. The current `String` return type bakes single-file. Suggested approach: add a parallel `codegen_files() -> Vec<(filename, content)>` keeping `codegen()` as a back-compat sugar that joins them with banner comments. Update CLI to call `codegen_files` when target = SV.

## M5 — Interrupt aggregation + SVA

**Goal.** Capture the canonical IRQ pattern (raw_status → status → enable → mask → pending → irq_out) plus an opt-in formal-friendly SVA checker module bound to the regblock.

### Interrupt portion

**DSL surface:**
```
field overflow 0 -> bool {
    intr: { trigger: level | posedge | negedge | bothedge,
            sticky: true,           // latched, cleared via on-write
            group: "err",           // optional partition
            mask: true,             // generate mask companion
            enable: true            // generate enable companion
    }
}
```

**Codegen:** for each `intr` field:
- Auto-synthesize companion `<group>_enable` and `<group>_mask` registers (allocate addresses next-free or via explicit `intr-address:` knob)
- Storage cascade gains a new arm: latch raw → status when enabled
- `irq_<group>` output assigned `|(<group>_status & ~<group>_mask)`
- Top-level `irq` output is OR over ungrouped intr fields

**Sub-tasks:**

| # | Item |
|---|------|
| M5.1 | DSL: `intr` sub-property bag on Field with trigger/sticky/group/mask/enable |
| M5.2 | Lexer tokens for `level`, `posedge`, `negedge`, `bothedge` |
| M5.3 | MIR: `Field.intr: Option<IntrSpec>`, validation pass `interrupt_fields_valid` |
| M5.4 | MIR pass `intr_groups_resolved` — distinct groups, allocate companion regs |
| M5.5 | MIR pass `companion_regs_synthesized` — add enable/mask regs |
| M5.6 | LIR mirrors |
| M5.7 | Codegen: `intr.sv.j2` snippet handling latch, mask, OR-reduce |
| M5.8 | UI test case `interrupt_aggregation` |
| M5.9 | Verilator TB exercising IRQ raise/clear/mask |

### SVA portion

**DSL surface:**
```
sv-assertions: {
    reset: true,
    decode_mutex: true,
    w1c: false,
    ro_invariance: false
}
```

**Codegen:** `<dev>_sva.sv` checker module with a `bind <dev>_regs <dev>_sva u_chk (.*);` snippet emitted to `<dev>_sva_bind.svh`.

Property set (portable subset, no `$isunknown` in FV-targeted assertions):

| Assertion | Default |
|-----------|---------|
| Reset values correct after `!rst_n` | on |
| Address decode mutex (`$onehot0`) | on |
| RO field invariance under SW write | off |
| W1C clears exactly the written bits | off |

**Sub-tasks:**

| # | Item |
|---|------|
| M5.10 | DSL: device-level `sv-assertions: { ... }` sub-property bag |
| M5.11 | New tokens for sub-keys (`reset`, `decode_mutex`, ...) — use sub-node? |
| M5.12 | MIR: `DeviceConfig.sv_assertions: SvAssertOpts` |
| M5.13 | Codegen: `checker.sv.j2` template emits `assert property` set per enabled |
| M5.14 | Codegen: `bind.svh.j2` template for include-snippet |
| M5.15 | Optional SymbiYosys CI job (skip if no demand) |

**Acceptance:** A case with `intr` fields emits enable/mask/pending companion regs + working irq output. SVA assertions can be enabled per category; checker file lints with verilator and (if SymbiYosys job enabled) proves the must-have set.

## M6 — Command/Buffer HW mapping + UVM RAL

**Goal.** Give DDSL `Command` and `Buffer` block-method types a concrete HW mapping. Emit UVM RAL package for verification-team consumption.

### Command/Buffer

**v1 DSL surface (conservative; reserve namespace, refuse other variants):**
```
command Foo {
    hw-handshake: strobe                         // default; v1
    // hw-handshake: valid-ready | fifo         // parsed but rejected v1
}
buffer Foo {
    hw-kind: fifo,
    depth: 16,
    status-address: 0x20                         // optional; otherwise auto-allocated
}
```

**Codegen:**
- Command (strobe): bus write to the address pulses `cmd_valid_<n>` for one cycle, latching `in` fields onto outputs. Response payload latched on `resp_valid_<n>`. Read of the address returns the response.
- Buffer (fifo): bus reads pop, writes push; auto-generated `<n>_status` companion register exposes `level`, `full`, `empty`, `almost_full`.

**Sub-tasks (Command):**

| # | Item |
|---|------|
| M6.1 | DSL: `hw-handshake: strobe` (only accepted value v1) |
| M6.2 | MIR: `BlockMethodType::Command` gains `hw_hints: Option<CommandHwHints>` |
| M6.3 | LIR mirrors |
| M6.4 | Codegen: emit per-command strobe + payload ports; latched response storage |
| M6.5 | UI test case `command_strobe` |

**Sub-tasks (Buffer):**

| # | Item |
|---|------|
| M6.6 | DSL: `hw-kind: fifo` (only accepted), `depth: N`, optional `status-address` |
| M6.7 | MIR: `BlockMethodType::Buffer` gains `hw_hints: Option<BufferHwHints>` |
| M6.8 | MIR pass `companion_regs_synthesized` extends to FIFO status |
| M6.9 | Codegen: FIFO instance + bus push/pop integration |
| M6.10 | UI test case `buffer_fifo` |

### UVM RAL

**v1 DSL surface:**
```
sv-ral: true                  // device-level (default true)
sv-hdl-path-prefix: "u_chip.u_regs"
```

**Output file:** `<dev>_ral_pkg.sv` containing `uvm_reg_field` / `uvm_reg` / `uvm_reg_block` subclasses. Backdoor paths set via `add_hdl_path_slice` referencing the locked naming convention from Decision 4.

**Coverage:** out of v1.

**Sub-tasks:**

| # | Item |
|---|------|
| M6.11 | DSL: `sv-ral` + `sv-hdl-path-prefix` device-level properties |
| M6.12 | Codegen: `ral_pkg.sv.j2` template |
| M6.13 | Multi-file output for RAL (paired with M4.10) |
| M6.14 | Snapshot test for `<dev>_ral_pkg.sv` |
| M6.15 | Optional UVM smoke seq emitted to `<dev>_ral_smoke_seq.sv` (no CI sim) |

**Acceptance:** Any case with a register emits a RAL package by default; backdoor paths line up with RTL naming; snapshot tests catch structural regressions.

## Risk register

| # | Risk | Mitigation |
|---|------|------------|
| 1 | Multi-file output API change (M4) | Add `codegen_files()` alongside `codegen()`; keep back-compat |
| 2 | Companion-reg address allocation collisions (M5/M6) | New MIR pass after synthesis, before non-overlap; user can override with explicit `<companion>-address:` |
| 3 | UVM HDL backdoor path drifts from RTL naming | Generator owns both — snapshot tests catch drift |
| 4 | Verilator SVA support gaps (some `$past` forms partial) | Document portable subset; gate FV-only via `ifdef FORMAL` |
| 5 | AXI4-Lite skid-buffer / B+R channel ordering bugs | Steal peakrdl-regblock's AXI4-Lite template wholesale |
| 6 | DSL `intr` mask-vs-enable conflation in user mental model | Default both companion regs; document distinction |
| 7 | Multi-word registers > `sv-data-width` not yet validated | Add MIR pass once M4 lands width parameterization |
| 8 | UVM package can't be functionally tested in OSS CI (Verilator no UVM) | Snapshot tests only; optional Questa/Xcelium job for users |
| 9 | Dead helpers from earlier milestones (`write_arm_statements`, `read_arm_statements`, `write_mask_lit`, `sv_reset_lit`, `sv_field_slice`, `sv_hwif_in_struct`) | Sweep cleanup at M4 start before adding new code |

## Phasing strategy (forward-looking)

**Update 2026-06-14:** all 9 items below have been worked through. The
SV target is now usability-shaped, not just snapshot-green: 43 UI
cases + 8 LIR unit tests pass; the multi-file CLI output, real
bus_compat_checked, functional TBs for M7b/d, explicit
`intr-enable-bit:` knobs, full doc set + e2e example, package/regs +
per-bus file splits via `codegen_files`, and RAL default flip are all
landed. The single remaining manual action is item 4 — pushing the
branch to origin and verifying CI is green.

### Cosmetic remainders (truly optional)

### Cosmetic remainders (truly optional)

These three are roadmap-tracked as cosmetic. None blocks usability; they only change file layout or default ergonomics.

1. **Pkg/regs file split.** Split the combined `<dev>.sv` output into separate `<dev>_pkg.sv` + `<dev>_regs.sv` files.
   - **Current state:** `codegen_files()` already returns the multi-file `Vec<(filename, content)>` API. The combined `<dev>.sv` artifact is a single entry that concatenates package + regs together.
   - **Why deferred:** Splitting requires adding standalone Askama templates for `package_file.sv.j2` and `module_file.sv.j2` (the scaffolds are checked in but unused). Existing snapshots and TBs reference `<case>.sv` as the combined file; splitting churns every snapshot and every Verilator TB compile command for zero functional gain.
   - **When to do it:** Only if a user explicitly asks for the per-file layout (typical regblock IP delivery conventions).

2. **Per-bus snapshot files (M4.12).** Emit each wrapper as its own `<case>__<bus>.sv` snapshot rather than appending to the combined file.
   - **Current state:** `codegen_files()` emits separate `<dev>_<bus>.sv` entries already. The test harness concatenates them into the single `<case>.sv` snapshot for diffing.
   - **Why deferred:** Per-bus snapshots would require reworking the 7 Verilator TB compile commands (`.github/workflows/ci.yaml`) to consume `<case>.sv` + `<case>__<bus>.sv` instead of the combined file. Pure layout churn — no behavior change.
   - **When to do it:** Bundled with the pkg/regs split above. They share the same harness rework.

3. **`sv-ral` default flip from opt-in to opt-out.** Currently `sv-ral: allow` is required to emit the RAL package. Flipping the default to "on, opt-out via `sv-no-ral: allow`" matches verification team expectations (RAL package is normally always wanted).
   - **Current state:** Opt-in default for snapshot stability. Of the 39 UI cases, only `ral_basic` and `companion_regs` set `sv-ral`; all 37 others would suddenly emit a RAL package after the flip, dragging in 35 snapshot regenerations of zero-content-change-but-massive-diff size.
   - **Why deferred:** The artifact set is identical either way (RAL is always *available*, just gated by a flag). The default is a cosmetic preference for verification teams vs. SW teams.
   - **When to do it:** When the verification-team consumer count exceeds the SW-only-target consumer count.

### Usability gaps (the real "is this shippable" list)

These are not roadmap-tracked because the original roadmap stopped at "snapshot tests green." They are the work between that milestone and a tool an external engineer can actually use.

4. **CI has never run any of the SV / sby work.** The 18 commits in this branch (M5a → M7h) have never been pushed to origin. The 7 Verilator TBs, the `verilator-lint` sweep, the SymbiYosys formal-check job, and the new LIR unit tests are all validated locally only.
   - **Why it matters:** Snapshot tests prove the Rust codegen runs deterministically, not that the generated SystemVerilog actually compiles + simulates correctly under Verilator. Any of the 7 TBs could be broken in a way the local development loop missed (e.g., a `_unused_*` wire collision warning escalated to error in CI's `-Wno-fatal` config).
   - **What to do:** `git push origin master`, watch CI, fix anything red. The SymbiYosys job is `continue-on-error: true`, so even a green CI doesn't guarantee `sva-formal` passes — verify the Action log explicitly.

5. **No functional TBs for M7b (reserved/external) or M7d (companion regs).** Both are behaviorally complex features that ship only with snapshot tests.
   - **M7b gap.** `reserved-behavior: rw_storage` introduces a per-register `always_ff` for reserved bits with biten gating + reset-value preservation. `external: allow` removes the storage flop entirely and routes reads through `hwif_in.<reg>_ext_rd_data`. Neither has a TB that proves the bus-side behavior is correct under SW writes and concurrent HW activity.
     - **Suggested TB:** `tb_reserved_external.sv` driving APB3 writes to the `Scratch` register (rw_storage), reading back to verify reserved bits preserved; driving SW writes to `ExtCtrl` (external), verifying `ext_wr_hit` pulses + `ext_wr_data` matches `cpuif_wr_data`, and that a user-driven `ext_rd_data` mock appears on bus reads.
   - **M7d gap.** Companion register synthesis is the heaviest LIR-level codegen change. IRQ enable/mask gating on `irq_<group>` outputs depends on the OR-reduce `(storage[bit] & enable_storage[i]) & ~mask_storage[i]` actually firing correctly. FIFO status companion register is `hw-access: WO` — user RTL must drive `_we` strobes for it to update.
     - **Suggested TB:** `tb_companion_regs.sv` exercising: SW writes 0 to enable bit → IRQ output goes low even though intr storage is set; SW writes 1 to mask bit → IRQ output goes low; user-driven FIFO `level_we` strobe → bus read of status register reflects the new level value.
   - **What to do:** Each TB is ~150 lines following the existing `tb_*.sv` pattern. Add CI workflow step per TB. Estimated 1-2 days of work.

6. **Multi-file CLI output unverified.** The compiler crate exposes `codegen_files() -> Vec<(filename, content)>` but `dd-cli` was last verified to write multi-file output during M4a — when the only multi-file outputs were `<dev>.sv` + per-bus wrappers. Since then, M5b added `<dev>_sva.sv` + `<dev>_sva_bind.svh`, M6c added `<dev>_ral_pkg.sv`, M7e added `<dev>_ahblite.sv`, and M7f added `<dev>_ral_smoke_seq.sv`. None of these have been verified end-to-end via the CLI.
   - **Concrete test:** Run `cargo run --bin dd-cli -- build -t sv -o /tmp/out tests/ui/cases/ral_basic/input.ddsl`. Expected: `/tmp/out/` contains `ral_basic.sv`, `ral_basic_apb3.sv`, `ral_basic_ral_pkg.sv`, `ral_basic_ral_smoke_seq.sv`. If the CLI only writes the first entry of the `Vec`, the other artifacts are silently lost.
   - **Likely failure mode:** `dd-cli/src/main.rs` calls the back-compat `codegen()` (single-string concat) rather than `codegen_files()` (multi-file). Read the CLI source to confirm; fix if needed.

7. **No user-facing documentation, no end-to-end example.** The repository currently ships `SYSTEMVERILOG_DECISIONS.md` (architect's reference) and `SYSTEMVERILOG_ROADMAP.md` (status board, this document). Neither answers the questions a new user asks first.
   - **Missing artifacts:**
     - A "how to use the SV target" guide: what DSL surface does it support, how do I invoke it, what files do I get, where do I put them in my RTL hierarchy.
     - A runtime contract document for `hwif_in` / `hwif_out` — what does each port mean, what cycles must the user RTL guarantee, how do strobes interact with storage.
     - An end-to-end example: a small DDSL source, the generated Rust HAL talking to the generated SV regblock through a co-simulation harness, showing the same `Status.error` bit being written from Rust and read from a SystemVerilog testbench. Without this, the "matched SW + HW surfaces" pitch is theoretical.
   - **Concrete output:** Three new files in `compiler/dd-codegen/docs/` (or similar): `SV_USER_GUIDE.md`, `SV_HWIF_CONTRACT.md`, `examples/sv_e2e/` (DDSL + Rust + SV + Makefile + a 20-line testbench).
   - **Effort:** ~1 day to write; ~half a day to verify the example actually builds and runs.

8. **Companion-register bit positions shift on field reorder.** The synthesis pass in `dd-lir/src/synthesis.rs::synthesize_intr_companions` allocates bits in declaration order per group. If the user adds a new `intr-enable: allow` field between two existing ones, every subsequent bit's position in `<group>_intr_enable` shifts, silently breaking any compiled software that assumed the old layout.
   - **Why it matters:** Once a chip tapes out, the SW driver references `enable.write(1 << OVERFLOW_BIT)` where `OVERFLOW_BIT` is a constant. If a future revision of the DDSL reorders fields and synthesizes a different `OVERFLOW_BIT`, the SW silently writes to the wrong bit.
   - **Fix options:**
     - **(a)** Declarative bit positions: require each opt-in field to specify `intr-enable-bit: N` explicitly. Most explicit, most user-friction.
     - **(b)** Hash-stable bit positions: derive each bit from a hash of `(group, field_name)`. No DSL change but sparse bit layouts in the companion reg waste storage.
     - **(c)** Diagnostic on bit-position drift: emit a build-time warning when the LIR sees the same `(group, field_name)` mapped to a different bit than the previous compile (requires persisting a `.lock` file).
     - **(d)** Append-only ordering: stick with declaration order but document the convention; warn the user during code-review of any `intr-enable: allow` reorder.
   - **Recommendation:** (a) is the right answer for a real release. (d) is acceptable while the tool is still pre-1.0.

9. **`bus_compat_checked` MIR pass is mistitled.** The original M4.3 spec called for a width/protocol-compatibility check: "if `sv-bus: ahblite` and `sv-data-width: 16`, reject with diagnostic." The pass shipped in M7c does only duplicate detection on the `sv-bus:` list.
   - **What's missing:**
     - Reject `sv-data-width` values outside {32, 64} for AXI4-Lite and AHB-Lite.
     - Reject `sv-bus: ahblite` combined with `register-address-type: u128` (out-of-spec addressing).
     - Warn on `sv-bus: native` combined with `sv-data-width: 64` (the native CPUIF supports it but few downstream consumers do).
   - **Fix:** Extend `compiler/dd-mir/src/passes/bus_compat_checked.rs` with the three checks above. Each is a 10-line addition + a new diagnostic variant in `dd-diagnostics/src/errors.rs`.
   - **Effort:** ~half a day including UI test cases.

### Recommended sequence to "actually usable"

In strict dependency order:

1. **Push to origin** (5 min). Let CI run. Fix anything red. This is non-negotiable — every other item below assumes the snapshot baseline is also CI-verified.
2. **Verify multi-file CLI output** (1-2 hours). Item 6 above. Cheap to test, blocks every downstream consumer.
3. **Two functional TBs** (1-2 days). Item 5 above. Without these, "M7b/d work" is unproven for real users.
4. **User guide + E2E example** (1.5 days). Item 7 above. Without this, the tool is opaque.
5. **Companion bit-position stability fix** (option (a), 1 day). Item 8 above. Without this, ABI breaks are silent.
6. **Real `bus_compat_checked`** (half day). Item 9 above. Catches DSL misuse early.

Cosmetic items (1, 2, 3) ship whenever someone explicitly asks. No effort estimate needed.

## Pre-M4 housekeeping (resolved)

The dead-helper sweep originally listed here is now complete, except
where the helper is still actively used:
* `write_arm_statements`, `read_arm_statements`, `write_mask_lit`,
  `sv_reset_lit`, `has_hw_writable_fields` — all already deleted.
* `sv_field_slice` is still called from the cascade-arm emitter and
  `field_flop` (3 call sites); intentionally kept.
* `sv_hwif_in_struct` is still referenced from `module.sv.j2` and
  `package.sv.j2`; intentionally kept.

## Cross-references

- `compiler/dd-codegen/SYSTEMVERILOG_DECISIONS.md` — locked architectural decisions
- `compiler/dd-codegen/src/systemverilog.rs` — codegen helpers
- `compiler/dd-codegen/templates/systemverilog/{device,package,module}.sv.j2` — templates
- `compiler/dd-common/src/specifiers.rs` — DSL enums (Access, OnWrite, OnRead, HwAccess, Precedence)
- `compiler/dd-mir/src/passes/{on_write,on_read}_semantics_valid.rs` — MIR validation
- `tests/ui/cases/*` — UI snapshot cases, one per DSL feature exercised
- `tests/sv/tb_hw_writable_status.sv` — functional Verilator TB
- `.github/workflows/ci.yaml` — `verilator-lint` + `verilator-sim` jobs

## How to resume cold

If a future session picks this up with no chat context:

1. Read this file, then `SYSTEMVERILOG_DECISIONS.md`.
2. `cargo test -p device-driver-tests` — should show 17/17 ui tests pass.
3. `git log --oneline -- compiler/dd-codegen` — confirm `ef3d272` + `39fb980` present.
4. `find compiler/dd-codegen/templates/systemverilog -name '*.j2'` — confirm 3 templates.
5. Open `compiler/dd-codegen/src/systemverilog.rs:field_flop` — that's the heart of M3.
6. Pick the next milestone from the status board.
7. For M4: start with the multi-file output API in `dd-codegen::codegen` before any template work.
