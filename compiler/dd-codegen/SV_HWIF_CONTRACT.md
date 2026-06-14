# `hwif_in` / `hwif_out` Contract

Read this when you wire the generated `<dev>_regs` module into your
user RTL. It documents every signal on the HW-side struct interface,
what cycle semantics it has, and how the cascade arms interact.

For the DSL surface that produces these signals, read
`SV_USER_GUIDE.md`. For the architectural decisions behind the port
choices, read `SYSTEMVERILOG_DECISIONS.md`.

## Two structs

`<dev>_regs` exposes its HW-side interface as two strongly-typed
struct ports:

```sv
output <dev>__out_t hwif_out,
input  <dev>__in_t  hwif_in
```

* `hwif_out` is **everything the regblock tells user RTL** — the
  current value of each storage register, plus per-cycle strobes for
  externally-mapped registers and command/buffer pulses.
* `hwif_in` is **everything user RTL tells the regblock** — HW-driven
  field updates, HW set/clear strobes, external-register read data,
  interrupt sources.

Both are packed structs typed in `<dev>_pkg`. Field names use the
canonical `<reg>_<field>[_strobe]` convention so collisions across
registers are impossible.

## Clock domain

All `hwif_*` signals are synchronous to the single `clk` exposed on
the regblock module's port list. Multi-clock crossings are NOT done
inside the regblock — the user RTL is responsible for synchronizing
asynchronous sources before wiring them to `hwif_in`. Synchronizers
typically go in user wrappers that aggregate per-block hwif structs.

`rst_n` is active-low and synchronous. Storage flops reset to either
the DSL-declared `reset:` value (registers) or the field-level reset
value embedded in the cascade arm (synthesized companion registers
reset to 0).

## Read-side: what `hwif_out` carries

### Storage-backed registers

For every register with storage (i.e., NOT `external: allow`):

```sv
hwif_out.<reg>                   // raw <CPUIF_DATA_W>-bit storage value
```

This signal is **combinational** with respect to the storage flop —
it changes on the next `clk` edge after the storage update, and
holds for as many cycles as the storage holds. User RTL can sample
it at any time without coordination with the bus.

For named fields, project the storage word through the per-fieldset
typedef:

```sv
status_fields_t s = status_fields_t'(hwif_out.status);
if (s.overflow) ...
```

### `hw-access: WO` / `hw-access: RW` fields

`hw-access` controls whether a field is observable on `hwif_out`,
whether it's drivable on `hwif_in`, and how the cascade arm composes
with SW writes.

| `hw-access` | On `hwif_out`? | On `hwif_in`? | SW writable? |
|-------------|----------------|---------------|--------------|
| `RO` (default) | yes | no | yes |
| `RW` | yes | yes (gated by `_we`) | yes |
| `WO` | yes | yes (gated by `_we`) | no |

A `RW` or `WO` field gets a paired `<reg>_<field>_we` input on
`hwif_in`. When `_we` is high on the rising edge of `clk`, the
storage flop latches `hwif_in.<reg>_<field>`. The cascade arm runs
on the same `clk`; see `precedence` below for ordering relative to
SW writes.

### Companion-register storage

`<group>_intr_enable`, `<group>_intr_mask`, and `<buf>_status` are
storage registers like any other. `hwif_out.root_intr_enable` exposes
the raw 32-bit enable mask; user RTL typically does not read it
(software writes it; the regblock's internal `irq_<group>` OR-reduce
consumes it).

### External registers

`external: allow` registers have NO storage flop. Instead:

```sv
hwif_out.<reg>_ext_wr_hit        // 1-cycle pulse on bus write hit
hwif_out.<reg>_ext_rd_hit        // 1-cycle pulse on bus read hit
hwif_out.<reg>_ext_wr_data       // bus write data (valid while ext_wr_hit)
hwif_in.<reg>_ext_rd_data        // user RTL drives this every cycle
```

The user-side contract:
* `_ext_wr_data` is meaningful **only during the cycle `_ext_wr_hit`
  is asserted**. Latch it if you need to retain the value.
* `_ext_rd_data` must be **valid combinationally during the cycle
  `_ext_rd_hit` is asserted**. The cycle of `_ext_rd_hit` feeds the
  internal `cpuif_rd_data_next` mux; the regblock captures it into
  the registered `cpuif_rd_data` on the next posedge, which is also
  when `cpuif_rd_ack` rises. Driving `_ext_rd_data` only AFTER
  observing `_ext_rd_hit` is too late — the regblock has already
  flopped '0.

### Buffer FIFO interface

For `buffer Foo { hw-kind: fifo, depth: N }`:

```sv
hwif_out.foo_push_valid          // bus wrote → push this data
hwif_out.foo_push_data
hwif_in.foo_push_ready           // user FIFO has room

hwif_out.foo_pop_ready           // bus read → pop one entry
hwif_in.foo_pop_data             // top of FIFO
hwif_in.foo_pop_valid            // FIFO non-empty (and ready)
```

Push/pop handshakes follow valid/ready convention: a transaction
completes when both `_valid` and `_ready` are high on a `clk` edge.
If the user FIFO can't accept a push (`_push_ready` low), the bus
transaction sets `cpuif_wr_err` (SLVERR). Same for a read on an
empty FIFO.

### Command strobes

For `command Foo { hw-handshake: strobe }`:

```sv
hwif_out.foo_cmd_valid           // 1-cycle pulse on bus write hit
hwif_out.foo_cmd_in              // payload (latched from cpuif_wr_data)
hwif_in.foo_resp_valid           // user latches a response
hwif_in.foo_resp_out             // payload (read on subsequent bus read)
```

## Write-side: what `hwif_in` carries

### HW-driven field updates

For each `RW` or `WO` field:

```sv
hwif_in.<reg>_<field>            // data
hwif_in.<reg>_<field>_we         // write enable (1-cycle pulse)
```

The cascade arm samples both on the same `clk` edge. If `_we` is
high, the storage flop takes `hwif_in.<reg>_<field>` instead of the
SW-write value (under default `precedence: hw`).

### `hw-clr` and `hw-set` strobes

For fields opted into HW-driven clear/set:

```sv
hwif_in.<reg>_<field>_hwclr      // 1-cycle pulse → storage <= 0
hwif_in.<reg>_<field>_hwset      // 1-cycle pulse → storage <= 1 / max
```

These are distinct from the `_we` data-latch path. `hwclr` and
`hwset` ignore the data value entirely; they're pure storage
operations. The cascade arm orders them as:

```
reset → hwclr → hwset → hw_we → sw_write → sw_read → singlepulse
```

So a simultaneous `_hwclr` and `_we` results in `_we` winning (it
runs later in the cascade).

### Interrupt sources

For fields with `intr-trigger: ...`:

```sv
hwif_in.<reg>_<field>_intr       // raw HW signal
```

The interpretation depends on the trigger style:
* `level` — storage latches while `_intr` is held high.
* `posedge` — storage latches on the rising edge of `_intr`.
* `negedge` — storage latches on the falling edge.
* `bothedge` — storage latches on either edge.

The latched bit becomes the `<reg>` storage bit visible on
`hwif_out`. SW W1C (`on-write: clear`) is the only way to clear it.

## Per-field cascade ordering

The cascade arm is the heart of the cycle semantics. For each field,
exactly one assignment fires per `clk` edge based on which arm's
condition is true. From earliest to latest:

```
1. reset (rst_n == 0)
2. hwclr (hwif_in.<f>_hwclr)
3. hwset (hwif_in.<f>_hwset)
4. hw_we (hwif_in.<f>_we)
5. sw_write (wr_hit_<reg> && cpuif_wr_biten[bit])
6. sw_read (rd_hit_<reg> && on-read modifier)
7. singlepulse (auto-clear; fires next cycle after any other arm)
```

This is the default `precedence: hw` order. With `precedence: sw`,
arms 5 + 6 move ahead of arms 2-4 so SW writes win during contention.

Arms whose condition is structurally impossible for a field (e.g.,
`hwset` on a field without `hw-set: allow`) are pruned at codegen
time — they don't appear in the generated `always_ff`.

## Bus handshake guarantees

The regblock's CPUIF is a fixed-latency, registered handshake:

* Reads: `cpuif_req` is presented at cycle N (combinational decode
  produces `rd_hit_*` and a `cpuif_rd_data_next` mux value the same
  cycle). At posedge N+1, `cpuif_rd_ack` and `cpuif_rd_data` both
  flop and become valid for the consumer. They stay aligned —
  `cpuif_rd_data` is captured into a registered output, not driven
  combinationally, so the consumer may sample `{rd_ack, rd_data}`
  together without racing the `cpuif_req` gating that bus wrappers
  apply on `~pready`.
* Writes: `cpuif_wr_ack` asserts **the same cycle** as the storage
  flop latches the write. The cascade arm runs that cycle; on the
  following cycle, `hwif_out.<reg>` reflects the new value.
* SLVERR: `cpuif_rd_err` / `cpuif_wr_err` asserts the same cycle as
  the corresponding ack if no register decoded the address (the
  `any_hit` OR-reduce is low). Address-miss errors are SLVERR only;
  the regblock never emits DECERR.

Bus wrappers (APB3/APB4/AXI4-Lite/AHB-Lite) translate this internal
handshake into the standard protocol's `pready` / `bvalid` / etc.
Wrapper-level skid buffers exist where the protocol requires them
(AXI4-Lite W-channel for write-data hold while waiting on B); they
do not perturb the internal CPUIF timing.

## What user RTL is NOT allowed to do

* Drive `hwif_in.<reg>_<field>` to a different value while `_we`
  remains high across multiple cycles AND expect the cascade arm to
  pick up the second value cleanly. If you need to hold a write
  enable for N cycles, drive `hwif_in.<reg>_<field>` to the SAME
  value across all N cycles. Treat `_we` as a single-cycle strobe.
* Drive `hwif_in.<reg>_ext_rd_data` only on `_ext_rd_hit`. It must
  hold a valid value combinationally every cycle — the bus reads
  it directly without latching.
* Assert two mutually-exclusive cascade arm inputs simultaneously
  (e.g., `_hwclr` and `_hwset` on the same cycle). The cascade
  resolves to the later one in the precedence order, but the
  reverse direction's intent is silently lost. Consider it a user
  bug.
* Wire asynchronous signals directly to `hwif_in`. The regblock
  assumes single-clock-domain operation; metastability from
  un-synchronized sources surfaces as bit-flip storage corruption.

## Worked example

A `Status` register with one HW-driven `overflow` flag (sticky,
level-triggered intr source) plus a SW-readable `count`:

```sv
// User RTL pushes a "we just had an overflow" event:
hwif_in.status_overflow_intr <= overflow_in_user_domain;

// User RTL tells the SW driver how many events have happened:
hwif_in.status_count    <= event_counter;
hwif_in.status_count_we <= event_counter_changed;
```

* Cycle 1: `overflow_in_user_domain` rises high. On `clk`'s posedge,
  the cascade arm latches `storage_status[overflow_bit] <= 1`.
* Cycle 2: SW issues a read of `STATUS_ADDR`. The read mux returns
  `storage_status & RD_MASK_STATUS`. SW sees `overflow = 1`.
* Cycle 3: SW issues a write of `0x1` (W1C) to clear `overflow`.
  Same cycle: `wr_hit_status` asserts, the cascade arm's W1C arm
  fires, `storage_status[overflow_bit] <= 0`.
* Cycle 4: `hwif_out.status[overflow_bit]` is now 0. The user RTL
  observation port matches the SW-visible state.

The same sequence under `precedence: sw` would let a same-cycle SW
W1C beat a `_we` strobe from user RTL.
