# SystemVerilog Target — User Guide

This guide answers the questions a first-time user of the
SystemVerilog code-generation target asks. For the architectural
"why," read `SYSTEMVERILOG_DECISIONS.md`. For project status and
outstanding work, read `SYSTEMVERILOG_ROADMAP.md`. For the runtime
contract between the regblock and your user RTL, read
`SV_HWIF_CONTRACT.md`.

## What you write

A single `*.ddsl` source describes both the software and hardware
view of the register map:

```ddsl
device Status {
    byte-order: LE,
    register-address-type: u8,
    sv-bus: apb3,           // emit an APB3 wrapper alongside the regblock
    sv-data-width: 32,      // CPUIF data width — 32 or 64

    register Ctrl {
        address: 0,
        access: RW,
        fields: fieldset CtrlFields {
            size-bytes: 4,
            field enable 0 -> bool,
            field mode 3:1 -> uint,
        }
    },

    register Status {
        address: 4,
        access: RO,
        fields: fieldset StatusFields {
            size-bytes: 4,
            field overflow 0 -> bool {
                on-write: clear,           // W1C
                intr-trigger: level,
                intr-enable: allow,
                intr-mask: allow,
            },
        }
    }
}
```

## What you get

The compiler runs the same source through two backends:

* **Rust HAL** — typed accessors (`Ctrl::Field::enable`,
  `Status::Field::overflow`) talking to whatever transport you wire
  up (SPI, I2C, MMIO, simulated). Ships in any crate that depends on
  `device-driver`.
* **SystemVerilog regblock** — a synthesizable module set you drop
  into a chip's register-bank hierarchy.

The two are guaranteed identical at the register-map level: addresses,
field bit positions, reset values, access modifiers, and interrupt
plumbing come from the same DSL source. Cross-side bugs caused by
hand-translating SW driver headers from RTL no longer exist.

## How you invoke the SV target

The CLI binary is `ddc` (built from the `device-driver-cli` crate).
Multi-file SV output is selected by passing a directory as `-o`:

```bash
$ cargo build -p device-driver-cli
$ ./target/debug/ddc build \
    -t sv \
    -o ./generated/sv/ \
    path/to/your.ddsl
```

The directory will contain one or more files depending on which
optional features you enabled in the DSL:

| File                       | When | Contents |
|----------------------------|------|----------|
| `<dev>.sv`                 | always | Package + regs module (combined, single file) |
| `<dev>_apb3.sv`            | `sv-bus: apb3` | APB3 wrapper |
| `<dev>_apb4.sv`            | `sv-bus: apb4` | APB4 wrapper (adds `pprot` + `pstrb`) |
| `<dev>_axi4lite.sv`        | `sv-bus: axi4lite` | AXI4-Lite wrapper |
| `<dev>_ahblite.sv`         | `sv-bus: ahblite` | AHB-Lite v1.1 wrapper (32-bit data) |
| `<dev>_native.sv`          | `sv-bus: native` | Structural passthrough |
| `<dev>_sva.sv`             | any `sv-assert-*: allow` | Bound SVA checker module |
| `<dev>_sva_bind.svh`       | any `sv-assert-*: allow` | `bind` snippet for the checker |
| `<dev>_ral_pkg.sv`         | `sv-ral: allow` | UVM register model |
| `<dev>_ral_smoke_seq.sv`   | `sv-ral: allow` | UVM smoke sequence (RW round-trip per register) |

Naming follows snake_case — a DSL `device MyChip` produces files
named `my_chip*.sv`. Address constants follow `<DEV>_<REG>_ADDR`
upper-case convention.

## Module hierarchy

The single combined `<dev>.sv` defines two SystemVerilog modules:

* `<dev>_pkg` (package) — typedefs for each fieldset, address
  constants, top-level `hwif_in_t` / `hwif_out_t` struct types.
* `<dev>_regs` (module) — the actual regblock. Has a native CPUIF
  port set + `hwif_out` + `hwif_in` typed structs.

When you opt in to a bus wrapper, the wrapper module instantiates
`<dev>_regs` and exposes the standard bus interface on its own ports.
Typical integration:

```sv
my_chip_apb3 u_regs (
    .clk        (clk),
    .rst_n      (rst_n),
    // APB3 ports
    .psel       (apb_psel),
    .penable    (apb_penable),
    .pwrite     (apb_pwrite),
    .paddr      (apb_paddr),
    .pwdata     (apb_pwdata),
    .prdata     (apb_prdata),
    .pready     (apb_pready),
    .pslverr    (apb_pslverr),
    // hwif passthrough (see SV_HWIF_CONTRACT.md)
    .hwif_out   (regs_to_user),
    .hwif_in    (user_to_regs)
);
```

You drive your user RTL from `regs_to_user.<reg>.<field>`, and you
report HW-sourced status into `user_to_regs.<reg>_<field>` /
`user_to_regs.<reg>_<field>_we` strobes.

## Quick reference — DSL surface added by the SV target

All of the below are inert under the Rust target. Adding them to a
DDSL source never changes the generated Rust HAL.

| Property | Scope | Values | Effect |
|----------|-------|--------|--------|
| `sv-bus` | device | `native`, `apb3`, `apb4`, `axi4lite`, `ahblite` (list) | Emit one wrapper per entry |
| `sv-data-width` | device | `32`, `64` | CPUIF data width (default 32) |
| `sv-ral` | device | `allow` | Emit UVM RAL package |
| `sv-hdl-path-prefix` | device | `"u_top.u_regs"` | RAL backdoor prefix |
| `sv-assert-reset` | device | `allow` | SVA: reset-value invariance |
| `sv-assert-decode-mutex` | device | `allow` | SVA: `$onehot0` address decode |
| `sv-assert-w1c` | device | `allow` | SVA: W1C clears exactly what was written |
| `sv-assert-ro-invariance` | device | `allow` | SVA: RO fields can't be SW-written |
| `intr-aggregate` | device | `true`/`false` (default `true`) | OR-reduce root irq |
| `intr-enable-address-base` | device | u32 | Base address for synth `<group>_intr_enable` |
| `intr-mask-address-base` | device | u32 | Base address for synth `<group>_intr_mask` |
| `reserved-behavior` | register | `ro_zero` (default), `ro_preserve`, `rw_storage` | What unnamed bits do under SW writes |
| `external` | register | `allow` | No storage; bus passes through to user RTL |
| `hw-handshake` | command | `strobe` | Bus write pulses `cmd_valid_<n>` |
| `hw-kind` | buffer | `fifo` | Bus reads pop / writes push the FIFO |
| `depth` | buffer | u32 | FIFO depth (default 16) |
| `status-address` | buffer | u32 | Synthesize `<buf>_status` companion |
| `on-write` | field | `store`, `clear`, `set`, `toggle` | W1C/W1S/W1T behavior |
| `on-read` | field | `store`, `clear`, `set` | RC/RS behavior |
| `hw-access` | field | `RO`, `RW`, `WO` | HW visibility (default RO) |
| `hw-clr` / `hw-set` | field | `allow` | HW strobe-driven 0/1 |
| `singlepulse` | field | `allow` | Auto-clear next cycle |
| `precedence` | field | `hw` (default), `sw` | Cascade arm ordering |
| `intr-trigger` | field | `level`, `posedge`, `negedge`, `bothedge` | Interrupt source detection |
| `intr-sticky` | field | `allow` | Latch raw to status |
| `intr-group` | field | `"<name>"` | Partition into `irq_<group>` |
| `intr-enable` | field | `allow` | Add to `<group>_intr_enable` register |
| `intr-mask` | field | `allow` | Add to `<group>_intr_mask` register |
| `intr-enable-bit` | field | u32 | Pin to a specific bit (ABI-stable) |
| `intr-mask-bit` | field | u32 | Pin to a specific bit (ABI-stable) |

## End-to-end example

See `examples/sv_e2e/` — a minimal DDSL source, both generated
artifacts, and a Verilator co-simulation harness that exercises the
same field from both sides (Rust HAL writes via a CPUIF stub; SV
testbench reads). Build with `make` from inside `examples/sv_e2e/`.

## Verification artifact set

Every register-map change reproduces deterministically across the
artifact set:

* Snapshot tests catch any change in `<dev>.sv` byte-for-byte.
* Verilator-lint sweeps every snapshot SV file.
* Functional Verilator TBs in `tests/sv/tb_*.sv` exercise the
  representative features (per-field cascade, HW>SW precedence, W1C,
  intr aggregation, FIFO status, external register passthrough).
* SymbiYosys job (best-effort) formally proves the SVA assertion set
  on the M5b checker snapshot.

## Common pitfalls

* **`sv-data-width: 64` + `sv-bus: ahblite`** is rejected at MIR-pass
  time. AHB-Lite v1.1's emitted wrapper is fixed at 32-bit
  `HRDATA`/`HWDATA`.
* **`sv-data-width: 64` + `sv-bus: native`** is allowed but produces
  a warning — most downstream interconnects do not consume 64-bit.
* **Adding a new `intr-enable: allow` field mid-list silently shifts
  every subsequent bit position** in `<group>_intr_enable`. Pin
  `intr-enable-bit: N` on any field whose bit must remain stable.
* **`external: allow` registers have no storage flop**. Bus reads
  return `hwif_in.<reg>_ext_rd_data` directly; user RTL must drive a
  valid value every cycle, not just when `_ext_rd_hit` is asserted.

## Where to look in the codebase

* `compiler/dd-codegen/src/systemverilog.rs` — codegen helpers
* `compiler/dd-codegen/templates/systemverilog/` — Askama templates
* `compiler/dd-lir/src/synthesis.rs` — companion register synthesis
* `compiler/dd-mir/src/passes/bus_compat_checked.rs` — bus / width
  compatibility checks
* `tests/ui/cases/*` — one DDSL case per feature; the `.sv` snapshot
  alongside each one is the expected output
* `tests/sv/tb_*.sv` — Verilator functional testbenches
