use askama::Template;
use convert_case::{Case, Casing};
use device_driver_common::{
    identifier::Identifier,
    specifiers::{
        Access, AddressRange, HwAccess, HwHandshake, HwKind, IntrTrigger, OnRead, OnWrite,
        Precedence, ReservedBehavior, SvBus,
    },
};
use device_driver_lir::model::{
    Block, BlockMethod, BlockMethodType, Driver, Field, FieldSet, Repeat, SvAssertOpts,
};

use crate::SvCodegenOptions;

#[derive(Template)]
#[template(path = "systemverilog/device.sv.j2", escape = "none")]
pub struct DeviceTemplateSv<'a> {
    driver: &'a Driver,
    #[allow(dead_code)]
    source: &'a str,
    compile_options: &'a SvCodegenOptions,
}

impl<'a> DeviceTemplateSv<'a> {
    pub fn new(driver: &'a Driver, source: &'a str, compile_options: &'a SvCodegenOptions) -> Self {
        Self {
            driver,
            source,
            compile_options,
        }
    }

    fn data_width(&self) -> u32 {
        if let Some(w) = self
            .driver
            .devices
            .first()
            .and_then(|d| d.blocks.iter().find(|b| b.root))
            .and_then(|b| b.sv_data_width)
        {
            return w;
        }
        self.compile_options.data_width.unwrap_or(32)
    }

    fn addr_width(&self) -> u32 {
        if let Some(w) = self.compile_options.addr_width {
            return w;
        }
        self.driver
            .devices
            .first()
            .map(|d| d.internal_address_type.size_bits())
            .unwrap_or(32)
    }

    fn find_field_set(
        &self,
        name: &Identifier<device_driver_common::identifier::Type>,
    ) -> Option<&FieldSet> {
        self.driver.field_sets.iter().find(|fs| fs.name == *name)
    }

    fn read_mask_lit(&self, method: &BlockMethod) -> Option<String> {
        let fs = self.method_field_set(method)?;
        let reg_access = self.register_access(method)?;
        let reg_readable = matches!(reg_access, Access::RW | Access::RO);
        // `RoPreserve` / `RwStorage` expose reserved (non-field-covered)
        // bits on reads — widen the mask to the full storage word in
        // those modes. `RoZero` (default) keeps the per-field mask.
        let reserved = self.register_reserved_behavior(method);
        if reg_readable
            && matches!(
                reserved,
                ReservedBehavior::RoPreserve | ReservedBehavior::RwStorage
            )
        {
            let width_bits = fs.size_bytes * 8;
            let cap: u128 = if width_bits >= 128 {
                u128::MAX
            } else {
                (1u128 << width_bits) - 1
            };
            return Some(format!("{}'h{:x}", width_bits, cap));
        }
        Some(field_mask_lit(fs, |a| {
            reg_readable && matches!(a, Access::RW | Access::RO)
        }))
    }

    fn register_access(&self, method: &BlockMethod) -> Option<Access> {
        match &method.method_type {
            BlockMethodType::Register { access, .. } => Some(*access),
            _ => None,
        }
    }

    /// `RoZero` for non-Register methods; the actual policy otherwise.
    fn register_reserved_behavior(&self, method: &BlockMethod) -> ReservedBehavior {
        match &method.method_type {
            BlockMethodType::Register {
                reserved_behavior, ..
            } => *reserved_behavior,
            _ => ReservedBehavior::RoZero,
        }
    }

    /// `true` iff the register is marked `external: allow` — codegen
    /// suppresses storage + cascade and routes reads through `hwif_in`.
    fn register_is_external(&self, method: &BlockMethod) -> bool {
        matches!(
            &method.method_type,
            BlockMethodType::Register { external: true, .. }
        )
    }

    /// For `reserved-behavior: rw_storage`, emit one `always_ff` per
    /// contiguous range of reserved bits. Each `always_ff` drives only
    /// its own `storage_<reg>[hi:lo]` slice, so synthesis sees no
    /// multiple drivers between the named-field cascades and the
    /// reserved-bit cascade. Returns an empty string for `ro_zero` and
    /// `ro_preserve` since the existing per-field cascade handles them.
    fn reserved_bits_flop(
        &self,
        method: &BlockMethod,
        fs: &FieldSet,
        inst: &RegInstance,
    ) -> String {
        if !matches!(
            self.register_reserved_behavior(method),
            ReservedBehavior::RwStorage
        ) {
            return String::new();
        }
        let storage = format!("{}{}", sv_reg_storage_name(&method.name), inst.suffix_snake);
        let reg = sv_reg_name(&method.name);
        // Build a field-coverage map: any bit covered by a declared field
        // is excluded; only the reserved bits land here.
        let width_bits = fs.size_bytes * 8;
        let mut covered = vec![false; width_bits as usize];
        for f in &fs.fields {
            for bit in f.address.start..=f.address.end {
                if (bit as usize) < covered.len() {
                    covered[bit as usize] = true;
                }
            }
        }
        // Reset-value byte stream → bit-level lookup.
        let reset_bytes = self.method_reset_bytes(method).unwrap_or_default();
        let reset_bit = |bit: u32| -> u32 {
            let byte_idx = (bit / 8) as usize;
            let bit_in_byte = bit % 8;
            if byte_idx < reset_bytes.len() {
                ((reset_bytes[byte_idx] >> bit_in_byte) & 1) as u32
            } else {
                0
            }
        };
        // Walk contiguous unreserved runs.
        let mut out = String::new();
        let mut emitted = false;
        let mut bit = 0u32;
        while bit < width_bits {
            if covered[bit as usize] {
                bit += 1;
                continue;
            }
            let lo = bit;
            while bit < width_bits && !covered[bit as usize] {
                bit += 1;
            }
            let hi = bit - 1;
            let slice = if hi == lo {
                format!("[{lo}]")
            } else {
                format!("[{hi}:{lo}]")
            };
            let run_width = hi - lo + 1;
            let mut reset_val: u128 = 0;
            for b in lo..=hi {
                reset_val |= (reset_bit(b) as u128) << (b - lo);
            }
            let reset_lit = format!("{run_width}'h{reset_val:x}");
            if !emitted {
                out.push_str("  // reserved bits (rw_storage policy)\n");
                emitted = true;
            }
            out.push_str(&format!(
                "  always_ff @(posedge clk) begin\n    \
                   if (!rst_n) begin\n      \
                     {storage}{slice} <= {reset_lit};\n    \
                   end else if (wr_hit_{reg}{}) begin\n      \
                     {storage}{slice} <= (cpuif_wr_data{slice} & cpuif_wr_biten{slice}) | \
                                          ({storage}{slice} & ~cpuif_wr_biten{slice});\n    \
                   end\n  end\n",
                inst.suffix_snake,
            ));
        }
        out
    }

    /// Enumerate concrete register instances for a method.
    /// Repeat::None → one instance with empty suffix.
    /// Repeat::Count → N instances at strided addresses, suffixed `_0`, `_1`, ...
    /// Repeat::Enum → one per enum variant, suffixed `_<variant>`, addressed by discriminant.
    fn instances(&self, method: &BlockMethod) -> Vec<RegInstance> {
        match &method.repeat {
            Repeat::None => vec![RegInstance {
                suffix_snake: String::new(),
                suffix_upper: String::new(),
                address: method.address,
            }],
            Repeat::Count { count, stride } => (0..*count)
                .map(|i| RegInstance {
                    suffix_snake: format!("_{}", i),
                    suffix_upper: format!("_{}", i),
                    address: method.address + (i as i128) * stride,
                })
                .collect(),
            Repeat::Enum {
                enum_name,
                enum_variants,
                stride,
            } => {
                let enum_def = self.driver.enums.iter().find(|e| e.name == *enum_name);
                enum_variants
                    .iter()
                    .map(|var_name| {
                        let discriminant = enum_def
                            .and_then(|e| e.variants.iter().find(|v| v.name == *var_name))
                            .map(|v| v.discriminant)
                            .unwrap_or(0);
                        RegInstance {
                            suffix_snake: format!("_{}", var_name.to_case(Case::Snake)),
                            suffix_upper: format!("_{}", var_name.to_case(Case::UpperSnake)),
                            address: method.address + discriminant * stride,
                        }
                    })
                    .collect()
            }
        }
    }
}

pub struct RegInstance {
    pub suffix_snake: String,
    pub suffix_upper: String,
    pub address: i128,
}

/// One strobe-handshake command. Generated by `command_strobe_entries` and
/// consumed by both `package.sv.j2` (to widen `hwif_in` / `hwif_out`
/// structs) and `module.sv.j2` (to drive the decoder + storage logic).
pub struct CmdStrobeEntry {
    /// e.g. `read_id` — matches the user's command name in snake_case.
    pub name: String,
    /// e.g. `CHIP_READ_ID_ADDR` — matches what the package emits.
    pub addr_const: String,
    /// Width of the `cmd_<n>_in` payload bits sent to HW. `0` ⇒ no
    /// `fields-in` declared.
    pub in_width: u32,
    /// Width of the `cmd_<n>_resp` payload bits captured from HW. `0` ⇒
    /// no `fields-out` declared; bus reads at this command's address
    /// return zero.
    pub out_width: u32,
    /// Combinational address of this command — embedded directly into
    /// the address-constant declaration in the package.
    pub address: i128,
}

impl CmdStrobeEntry {
    #[must_use]
    pub fn has_resp(&self) -> bool {
        self.out_width > 0
    }
    #[must_use]
    pub fn has_payload(&self) -> bool {
        self.in_width > 0
    }
}

/// One FIFO-mapped buffer. Generated by `buffer_fifo_entries` and consumed
/// by both `package.sv.j2` (hwif struct widening) and `module.sv.j2`
/// (decoder + push/pop strobes).
pub struct BufFifoEntry {
    pub name: String,
    pub addr_const: String,
    pub address: i128,
    /// Capacity hint surfaced in doc comments; the regblock itself does
    /// not instantiate FIFO storage in v1.
    pub depth: Option<u32>,
    /// If `true`, bus writes push (i.e. the buffer is SW-writable).
    pub bus_pushes: bool,
    /// If `true`, bus reads pop (i.e. the buffer is SW-readable).
    pub bus_pops: bool,
}

/// One emitted interrupt-source. Carries the strings the SV templates need
/// without re-deriving them at every callsite. Generated by
/// `DeviceTemplateSv::intr_sources`.
/// Companion-register side. Mirrors `dd-lir::synthesis::Side`.
#[derive(Clone, Copy)]
enum Side {
    Enable,
    Mask,
}

pub struct IntrSource {
    /// e.g. `status_0_overflow` — matches `sv_hwif_field_name` for this field.
    pub hwif_field_name: String,
    /// Storage bit expression, e.g. `storage_status_0[3]`. Drives the OR-reduce
    /// into `irq` / `irq_<group>`.
    pub storage_bit: String,
    /// Combinational event signal, e.g. `intr_event_status_0_overflow`. The
    /// field cascade gates its `'1` arm on this.
    pub event_signal: String,
    /// Previous-cycle reg name when edge detection is needed. `None` for
    /// `IntrTrigger::Level`.
    pub prev_signal: Option<String>,
    pub trigger: IntrTrigger,
    /// `None` ⇒ contributes to root `irq`. `Some(g)` ⇒ contributes to
    /// `irq_<g>` only.
    pub group: Option<String>,
    /// Original source-field name (used to resolve the synthesized
    /// companion enable/mask bit position, which must match
    /// `dd-lir::synthesis::synthesize_intr_companions`'s allocation).
    pub source_field_name: String,
    /// True iff this source opted into the per-group `intr_enable`
    /// companion register.
    pub has_enable_opt_in: bool,
    /// True iff this source opted into the per-group `intr_mask`
    /// companion register.
    pub has_mask_opt_in: bool,
    /// Explicit pinning of the enable bit position. `None` ⇒ falls back
    /// to declaration-order allocation around any pins.
    pub explicit_enable_bit: Option<u32>,
    /// Same for the mask companion.
    pub explicit_mask_bit: Option<u32>,
}

impl<'a> DeviceTemplateSv<'a> {
    fn method_field_set(&self, method: &BlockMethod) -> Option<&FieldSet> {
        let name = match &method.method_type {
            BlockMethodType::Register { field_set_name, .. } => field_set_name,
            _ => return None,
        };
        self.find_field_set(name)
    }

    /// True iff any field in any register of this block needs HW-driven input —
    /// either a `_we`+value pair (hw-access RW/WO) or a `_hwclr` / `_hwset`
    /// strobe. Drives whether the `<dev>__in_t` struct and `hwif_in` module
    /// port are emitted at all.
    fn has_hw_in_port(&self, block: &Block) -> bool {
        for method in &block.methods {
            // Strobe-handshake commands with a response payload always
            // require HW → regs inputs (`cmd_<n>_resp_valid` + payload).
            if let BlockMethodType::Command {
                field_set_name_out,
                hw_handshake,
                ..
            } = &method.method_type
            {
                if matches!(hw_handshake, Some(HwHandshake::Strobe))
                    && field_set_name_out
                        .as_ref()
                        .and_then(|n| self.find_field_set(n))
                        .is_some_and(|fs| fs.size_bytes > 0)
                {
                    return true;
                }
                continue;
            }
            // FIFO buffers that can be popped (RW or RO) need a HW → regs
            // `buf_<n>_rdata` input.
            if let BlockMethodType::Buffer {
                access, hw_kind, ..
            } = &method.method_type
            {
                if matches!(hw_kind, Some(HwKind::Fifo))
                    && matches!(access, Access::RW | Access::RO)
                {
                    return true;
                }
                continue;
            }
            // External registers always require `<reg>_ext_rd_data`
            // from user RTL.
            if self.register_is_external(method) {
                return true;
            }
            let Some(fs) = self.method_field_set(method) else {
                continue;
            };
            if fs.fields.iter().any(|f| {
                matches!(f.hw_access, HwAccess::RW | HwAccess::WO)
                    || f.hw_clr
                    || f.hw_set
                    || f.is_interrupt()
            }) {
                return true;
            }
        }
        false
    }

    /// True iff any field of the register carries a non-trivial on-read modifier.
    /// Drives whether the storage flop gets an `else if (rd_hit_<reg>) ...` arm.
    /// Bytes of the reset value for the register this method declares, or
    /// `None` when no reset is set.
    fn method_reset_bytes<'m>(&self, method: &'m BlockMethod) -> Option<&'m [u8]> {
        match &method.method_type {
            BlockMethodType::Register { reset_value, .. } => {
                reset_value.as_ref().map(|rv| rv.value.as_slice())
            }
            _ => None,
        }
    }

    /// Compute the slice of the register reset value that covers `field`.
    /// Defaults to `'0` when no reset is set on the register.
    fn field_reset_lit(&self, method: &BlockMethod, field: &Field) -> String {
        let width = field.address.end - field.address.start + 1;
        let Some(bytes) = self.method_reset_bytes(method) else {
            return "'0".to_string();
        };
        let mut value: u128 = 0;
        for (i, b) in bytes.iter().enumerate().take(16) {
            value |= (*b as u128) << (i * 8);
        }
        let mask: u128 = if width >= 128 {
            u128::MAX
        } else {
            (1u128 << width) - 1
        };
        let slice_val = (value >> field.address.start) & mask;
        format!("{}'h{:x}", width, slice_val)
    }

    /// Whether this field accepts software writes, accounting for both
    /// register-level and field-level access.
    fn field_sw_writable(&self, method: &BlockMethod, field: &Field) -> bool {
        let reg_writable = self
            .register_access(method)
            .map(|a| matches!(a, Access::RW | Access::WO))
            .unwrap_or(false);
        reg_writable && matches!(field.access, Access::RW | Access::WO)
    }

    /// Render the full `always_ff` block for one field of one register
    /// instance. Arm order is governed by `field.precedence`:
    ///
    ///   `Hw` (default):
    ///     reset → hw-clr → hw-set → hw_we write → sw write → sw on-read
    ///           → singlepulse auto-clear
    ///
    ///   `Sw` (override):
    ///     reset → sw write → sw on-read → hw-clr → hw-set → hw_we write
    ///           → singlepulse auto-clear
    ///
    /// Singlepulse always sits as the lowest-precedence catch-all so it can't
    /// rob another arm of the cycle. Reset is always first. Missing arms are
    /// omitted entirely.
    fn field_flop(&self, method: &BlockMethod, field: &Field, inst: &RegInstance) -> String {
        let storage = format!("{}{}", sv_reg_storage_name(&method.name), inst.suffix_snake);
        let slice = sv_field_slice(&field.address);
        let q = format!("{storage}{slice}");
        let reset_lit = self.field_reset_lit(method, field);
        let reg_name = sv_reg_name(&method.name);
        let hwif = sv_hwif_field_name(&method.name, &inst.suffix_snake, field);

        // ---- Per-arm bodies (each emits the `else if (...) begin ... end`
        // fragment when applicable, or None when the field doesn't use it). ----
        let hwclr_arm = field
            .hw_clr
            .then(|| format!(" else if (hwif_in.{hwif}_hwclr) begin\n      {q} <= '0;\n    end"));
        let hwset_arm = field
            .hw_set
            .then(|| format!(" else if (hwif_in.{hwif}_hwset) begin\n      {q} <= '1;\n    end"));
        // Interrupt-source arm. Treated as a high-precedence hw-set: when the
        // detected event asserts the storage latches to '1' (sticky). SW
        // clears via `on-write: clear` (W1C). The DSL forces sticky=true when
        // `intr-trigger` is set; non-sticky/transparent latches are not
        // expressible in v1.
        let intr_arm = field.intr_trigger.is_some().then(|| {
            let event = format!("intr_event_{hwif}");
            format!(" else if ({event}) begin\n      {q} <= '1;\n    end")
        });
        let hw_we_arm = matches!(field.hw_access, HwAccess::RW | HwAccess::WO).then(|| {
            format!(" else if (hwif_in.{hwif}_we) begin\n      {q} <= hwif_in.{hwif};\n    end")
        });
        let sw_write_arm = self.field_sw_writable(method, field).then(|| {
            let biten = format!("cpuif_wr_biten{slice}");
            let wdata = format!("cpuif_wr_data{slice}");
            let rhs = match field.on_write {
                OnWrite::Store => format!("({wdata} & {biten}) | ({q} & ~{biten})"),
                OnWrite::Clear => format!("{q} & ~({wdata} & {biten})"),
                OnWrite::Set => format!("{q} | ({wdata} & {biten})"),
                OnWrite::Toggle => format!("{q} ^ ({wdata} & {biten})"),
            };
            format!(
                " else if (wr_hit_{reg_name}{}) begin\n      {q} <= {rhs};\n    end",
                inst.suffix_snake
            )
        });
        let sw_read_arm = (!matches!(field.on_read, OnRead::Store)
            && matches!(field.access, Access::RW | Access::RO))
        .then(|| {
            let rhs = match field.on_read {
                OnRead::Clear => "'0",
                OnRead::Set => "'1",
                OnRead::Store => unreachable!(),
            };
            format!(
                " else if (rd_hit_{reg_name}{}) begin\n      {q} <= {rhs};\n    end",
                inst.suffix_snake
            )
        });
        let singlepulse_arm = field
            .singlepulse
            .then(|| format!(" else begin\n      {q} <= '0;\n    end"));

        // ---- Order arms by precedence and concatenate. ----
        let mut body = format!(
            "  always_ff @(posedge clk) begin\n    if (!rst_n) begin\n      {q} <= {reset_lit};\n    end"
        );
        // `intr_arm` sits right alongside `hw_set` — both are HW-side strobes
        // that latch the field to '1'. Precedence layout matches Decision 7:
        // HW-side strobes win over both SW writes and SW read modifiers.
        let arms: [Option<String>; 6] = match field.precedence {
            Precedence::Hw => [
                hwclr_arm,
                hwset_arm,
                intr_arm,
                hw_we_arm,
                sw_write_arm,
                sw_read_arm,
            ],
            Precedence::Sw => [
                sw_write_arm,
                sw_read_arm,
                hwclr_arm,
                hwset_arm,
                intr_arm,
                hw_we_arm,
            ],
        };
        for arm in arms.into_iter().flatten() {
            body.push_str(&arm);
        }
        if let Some(sp) = singlepulse_arm {
            body.push_str(&sp);
        }
        body.push_str("\n  end\n");
        body
    }

    /// Enumerate every interrupt-source field across every register instance
    /// of `block`. Used by the module template to declare prev regs, event
    /// signals, irq output ports, and the OR-reduces that drive them.
    fn intr_sources(&self, block: &Block) -> Vec<IntrSource> {
        let mut out = Vec::new();
        for method in &block.methods {
            let Some(fs) = self.method_field_set(method) else {
                continue;
            };
            for inst in self.instances(method) {
                for field in &fs.fields {
                    let Some(trigger) = field.intr_trigger else {
                        continue;
                    };
                    let hwif_field_name =
                        sv_hwif_field_name(&method.name, &inst.suffix_snake, field);
                    let storage_name = sv_reg_storage_name(&method.name);
                    let storage = format!("{}{}", storage_name, inst.suffix_snake);
                    let slice = sv_field_slice(&field.address);
                    let event_signal = format!("intr_event_{}", hwif_field_name);
                    let prev_signal = (!matches!(trigger, IntrTrigger::Level))
                        .then(|| format!("intr_prev_{}", hwif_field_name));
                    out.push(IntrSource {
                        hwif_field_name,
                        storage_bit: format!("{storage}{slice}"),
                        event_signal,
                        prev_signal,
                        trigger,
                        group: field.intr_group.as_ref().and_then(|g| {
                            let s = g.to_case(Case::Snake);
                            if s.is_empty() { None } else { Some(s) }
                        }),
                        source_field_name: field.name.original().to_string(),
                        has_enable_opt_in: field.intr_enable,
                        has_mask_opt_in: field.intr_mask,
                        explicit_enable_bit: field.intr_enable_bit,
                        explicit_mask_bit: field.intr_mask_bit,
                    });
                }
            }
        }
        out
    }

    /// Distinct named groups in deterministic order. Drives one `irq_<group>`
    /// output port per entry.
    fn intr_groups(&self, block: &Block) -> Vec<String> {
        let mut groups: Vec<String> = self
            .intr_sources(block)
            .into_iter()
            .filter_map(|s| s.group)
            .collect();
        groups.sort();
        groups.dedup();
        groups
    }

    /// `true` iff at least one intr field has no `intr-group` set — drives
    /// emission of the root `irq` output port. Also gated by the device's
    /// `intr-no-aggregate` flag (default on; set to suppress).
    fn has_root_irq(&self, block: &Block) -> bool {
        block.intr_aggregate && self.intr_sources(block).iter().any(|s| s.group.is_none())
    }

    /// `true` iff any field of any register in the block declares `intr-trigger`.
    /// Used to gate emission of interrupt plumbing in the templates.
    fn has_any_intr(&self, block: &Block) -> bool {
        !self.intr_sources(block).is_empty()
    }

    /// OR-reduce expression over every sticky storage bit whose group matches
    /// `group` (`None` ⇒ ungrouped → root `irq`). Each source bit is gated by
    /// its companion `<group>_intr_enable` (when present) and inverse of its
    /// companion `<group>_intr_mask` (when present). Used by the `irq` /
    /// `irq_<group>` assigns. Returns `"1'b0"` when the set is empty.
    ///
    /// The enable/mask bit indices must match the synthesis pass's
    /// allocation (`dd-lir::synthesis::synthesize_intr_companions`),
    /// which honors explicit `intr-enable-bit:` / `intr-mask-bit:`
    /// pinning before filling un-pinned fields into the lowest free
    /// bits in declaration order. Without this resolution the formula
    /// would index by source-field bit instead of synthesized-companion
    /// bit, silently breaking IRQ gating whenever a user pinned an
    /// enable/mask bit different from the source-field position.
    fn intr_group_or(&self, block: &Block, group: Option<&str>) -> String {
        let group_label = group.unwrap_or("root").to_string();
        let enable_reg = format!("{group_label}_intr_enable");
        let mask_reg = format!("{group_label}_intr_mask");
        let has_enable = self.has_companion_register(block, &enable_reg);
        let has_mask = self.has_companion_register(block, &mask_reg);
        let sources_in_group: Vec<IntrSource> = self
            .intr_sources(block)
            .into_iter()
            .filter(|s| s.group.as_deref() == group)
            .collect();
        let enable_bits = Self::allocate_companion_bits(&sources_in_group, Side::Enable);
        let mask_bits = Self::allocate_companion_bits(&sources_in_group, Side::Mask);
        let mut terms: Vec<String> = Vec::new();
        for s in &sources_in_group {
            let mut term = s.storage_bit.clone();
            if has_enable && let Some(bit) = enable_bits.get(s.source_field_name.as_str()) {
                term = format!("({term} & storage_{enable_reg}[{bit}])");
            }
            if has_mask && let Some(bit) = mask_bits.get(s.source_field_name.as_str()) {
                term = format!("({term} & ~storage_{mask_reg}[{bit}])");
            }
            terms.push(term);
        }
        if terms.is_empty() {
            return "1'b0".to_string();
        }
        terms.join(" | ")
    }

    /// Replicates `dd-lir::synthesis::synthesize_intr_companions`'s
    /// two-pass allocation: explicit pins claim their bit first, then
    /// un-pinned opt-ins fill the lowest unclaimed bit in declaration
    /// order. Returns a map keyed by source-field name. Fields that
    /// didn't opt in to the given side are absent from the map.
    fn allocate_companion_bits(
        sources: &[IntrSource],
        side: Side,
    ) -> std::collections::HashMap<&str, u32> {
        let mut out: std::collections::HashMap<&str, u32> = std::collections::HashMap::new();
        let mut claimed: std::collections::BTreeSet<u32> = std::collections::BTreeSet::new();
        // Pass 1: explicit pins.
        for s in sources {
            let (opt_in, explicit) = match side {
                Side::Enable => (s.has_enable_opt_in, s.explicit_enable_bit),
                Side::Mask => (s.has_mask_opt_in, s.explicit_mask_bit),
            };
            if !opt_in {
                continue;
            }
            if let Some(bit) = explicit {
                claimed.insert(bit);
                out.insert(s.source_field_name.as_str(), bit);
            }
        }
        // Pass 2: declaration-order fill into lowest unclaimed bit.
        let mut next = 0u32;
        for s in sources {
            let (opt_in, explicit) = match side {
                Side::Enable => (s.has_enable_opt_in, s.explicit_enable_bit),
                Side::Mask => (s.has_mask_opt_in, s.explicit_mask_bit),
            };
            if !opt_in || explicit.is_some() {
                continue;
            }
            while claimed.contains(&next) {
                next += 1;
            }
            claimed.insert(next);
            out.insert(s.source_field_name.as_str(), next);
            next += 1;
        }
        out
    }

    /// `true` iff the block has a method named `<name>` that is a
    /// Register. Used by `intr_group_or` to decide whether the
    /// enable/mask companion exists.
    fn has_companion_register(&self, block: &Block, name: &str) -> bool {
        block.methods.iter().any(|m| {
            matches!(&m.method_type, BlockMethodType::Register { .. }) && m.name.original() == name
        })
    }

    /// OR-reduce of all root-bound (un-grouped) intr storage bits. Template
    /// shim around `intr_group_or` because askama doesn't sugar `None`.
    fn intr_root_or(&self, block: &Block) -> String {
        self.intr_group_or(block, None)
    }

    /// OR-reduce of intr storage bits in the named group. Template shim
    /// around `intr_group_or` because askama doesn't sugar `Some(...)`.
    fn intr_named_group_or(&self, block: &Block, group: &str) -> String {
        self.intr_group_or(block, Some(group))
    }

    /// Enumerate every strobe-handshake command on `block`. Each entry
    /// gives the package + module templates enough to emit decoder,
    /// payload, response storage, and read-mux logic without re-walking
    /// the LIR every time.
    fn command_strobe_entries(&self, block: &Block) -> Vec<CmdStrobeEntry> {
        let mut out = Vec::new();
        for method in &block.methods {
            let BlockMethodType::Command {
                field_set_name_in,
                field_set_name_out,
                hw_handshake,
            } = &method.method_type
            else {
                continue;
            };
            if !matches!(hw_handshake, Some(HwHandshake::Strobe)) {
                continue;
            }
            let in_width = field_set_name_in
                .as_ref()
                .and_then(|n| self.find_field_set(n))
                .map(|fs| fs.size_bytes * 8)
                .unwrap_or(0);
            let out_width = field_set_name_out
                .as_ref()
                .and_then(|n| self.find_field_set(n))
                .map(|fs| fs.size_bytes * 8)
                .unwrap_or(0);
            out.push(CmdStrobeEntry {
                name: sv_reg_name(&method.name),
                addr_const: sv_reg_addr_const(&block.name, &method.name),
                in_width,
                out_width,
                address: method.address,
            });
        }
        out
    }

    /// `true` iff the block has any strobe command — drives a section
    /// switch in the module template.
    fn has_strobe_commands(&self, block: &Block) -> bool {
        !self.command_strobe_entries(block).is_empty()
    }

    /// Enumerate every FIFO-mapped buffer on `block`.
    fn buffer_fifo_entries(&self, block: &Block) -> Vec<BufFifoEntry> {
        let mut out = Vec::new();
        for method in &block.methods {
            let BlockMethodType::Buffer {
                access,
                hw_kind,
                depth,
                ..
            } = &method.method_type
            else {
                continue;
            };
            if !matches!(hw_kind, Some(HwKind::Fifo)) {
                continue;
            }
            out.push(BufFifoEntry {
                name: sv_reg_name(&method.name),
                addr_const: sv_reg_addr_const(&block.name, &method.name),
                address: method.address,
                depth: *depth,
                bus_pushes: matches!(access, Access::RW | Access::WO),
                bus_pops: matches!(access, Access::RW | Access::RO),
            });
        }
        out
    }

    /// `true` iff the block has any FIFO buffer — gates a section in the
    /// module template.
    fn has_fifo_buffers(&self, block: &Block) -> bool {
        !self.buffer_fifo_entries(block).is_empty()
    }

    /// Combinational expression for the event signal of one intr source.
    /// `Level` => raw input, `Posedge` => `~prev & raw`, etc.
    fn intr_event_expr(&self, src: &IntrSource) -> String {
        let raw = format!("hwif_in.{}_intr", src.hwif_field_name);
        match src.trigger {
            IntrTrigger::Level => raw,
            IntrTrigger::Posedge => {
                let prev = src.prev_signal.as_ref().unwrap();
                format!("~{prev} & {raw}")
            }
            IntrTrigger::Negedge => {
                let prev = src.prev_signal.as_ref().unwrap();
                format!("{prev} & ~{raw}")
            }
            IntrTrigger::Bothedge => {
                let prev = src.prev_signal.as_ref().unwrap();
                format!("{prev} ^ {raw}")
            }
        }
    }
}

fn sv_logic_width(bits: u32) -> String {
    if bits <= 1 {
        String::new()
    } else {
        format!("[{}:0]", bits - 1)
    }
}

fn sv_field_slice(range: &AddressRange) -> String {
    if range.start == range.end {
        format!("[{}]", range.start)
    } else {
        format!("[{}:{}]", range.end, range.start)
    }
}

fn sv_field_width(range: &AddressRange) -> u32 {
    range.end - range.start + 1
}

fn sv_module_name<T: device_driver_common::identifier::IdentifierType>(
    device_name: &Identifier<T>,
) -> String {
    format!("{}_regs", device_name.to_case(Case::Snake))
}

fn sv_package_name<T: device_driver_common::identifier::IdentifierType>(
    device_name: &Identifier<T>,
) -> String {
    format!("{}_pkg", device_name.to_case(Case::Snake))
}

fn sv_reg_addr_const<
    T: device_driver_common::identifier::IdentifierType,
    U: device_driver_common::identifier::IdentifierType,
>(
    device_name: &Identifier<T>,
    reg_name: &Identifier<U>,
) -> String {
    format!(
        "{}_{}_ADDR",
        device_name.to_case(Case::UpperSnake),
        reg_name.to_case(Case::UpperSnake)
    )
}

fn sv_hwif_in_struct<T: device_driver_common::identifier::IdentifierType>(
    device_name: &Identifier<T>,
) -> String {
    format!("{}__in_t", device_name.to_case(Case::Snake))
}

fn sv_hwif_out_struct<T: device_driver_common::identifier::IdentifierType>(
    device_name: &Identifier<T>,
) -> String {
    format!("{}__out_t", device_name.to_case(Case::Snake))
}

fn sv_field_name(field: &Field) -> String {
    field.name.to_case(Case::Snake)
}

fn sv_reg_name<T: device_driver_common::identifier::IdentifierType>(
    method_name: &Identifier<T>,
) -> String {
    method_name.to_case(Case::Snake)
}

fn field_mask_lit<F: Fn(Access) -> bool>(field_set: &FieldSet, accept: F) -> String {
    let width_bits = field_set.size_bytes * 8;
    let mut mask: u128 = 0;
    for field in &field_set.fields {
        if !accept(field.access) {
            continue;
        }
        for bit in field.address.start..=field.address.end {
            if bit < 128 {
                mask |= 1u128 << bit;
            }
        }
    }
    let cap: u128 = if width_bits >= 128 {
        u128::MAX
    } else {
        (1u128 << width_bits) - 1
    };
    mask &= cap;
    format!("{}'h{:x}", width_bits, mask)
}

fn sv_access_comment(access: &Access) -> &'static str {
    match access {
        Access::RW => "RW",
        Access::RO => "RO",
        Access::WO => "WO",
    }
}

fn sv_hex_addr(addr: &i128) -> String {
    format!("'h{:x}", addr)
}

fn sv_field_set_struct_name(field_set: &FieldSet) -> String {
    format!("{}_t", field_set.name.to_case(Case::Snake))
}

fn sv_hw_writable(hw: &HwAccess) -> bool {
    matches!(hw, HwAccess::RW | HwAccess::WO)
}

fn sv_hwif_field_name<T: device_driver_common::identifier::IdentifierType>(
    method_name: &Identifier<T>,
    suffix_snake: &str,
    field: &Field,
) -> String {
    format!(
        "{}{}_{}",
        method_name.to_case(Case::Snake),
        suffix_snake,
        field.name.to_case(Case::Snake)
    )
}

fn sv_reg_storage_width(field_set: &FieldSet) -> u32 {
    field_set.size_bytes * 8
}

fn sv_reg_storage_name<T: device_driver_common::identifier::IdentifierType>(
    method_name: &Identifier<T>,
) -> String {
    format!("storage_{}", method_name.to_case(Case::Snake))
}

fn sv_sva_module_name<T: device_driver_common::identifier::IdentifierType>(
    device_name: &Identifier<T>,
) -> String {
    format!("{}_sva", device_name.to_case(Case::Snake))
}

fn sv_sva_bind_filename<T: device_driver_common::identifier::IdentifierType>(
    device_name: &Identifier<T>,
) -> String {
    format!("{}_sva_bind.svh", device_name.to_case(Case::Snake))
}

fn sv_sva_bind_guard<T: device_driver_common::identifier::IdentifierType>(
    device_name: &Identifier<T>,
) -> String {
    format!("_{}_SVA_BIND_SVH", device_name.to_case(Case::UpperSnake))
}

fn sv_ral_pkg_name<T: device_driver_common::identifier::IdentifierType>(
    device_name: &Identifier<T>,
) -> String {
    format!("{}_ral_pkg", device_name.to_case(Case::Snake))
}

fn sv_ral_pkg_guard<T: device_driver_common::identifier::IdentifierType>(
    device_name: &Identifier<T>,
) -> String {
    format!("{}_RAL_PKG_SV", device_name.to_case(Case::UpperSnake))
}

fn sv_ral_block_class<T: device_driver_common::identifier::IdentifierType>(
    device_name: &Identifier<T>,
) -> String {
    format!("{}_reg_block", device_name.to_case(Case::Snake))
}

fn sv_ral_reg_class<
    T: device_driver_common::identifier::IdentifierType,
    U: device_driver_common::identifier::IdentifierType,
>(
    device_name: &Identifier<T>,
    reg_name: &Identifier<U>,
) -> String {
    format!(
        "{}_{}_reg",
        device_name.to_case(Case::Snake),
        reg_name.to_case(Case::Snake)
    )
}

fn sv_ral_smoke_pkg<T: device_driver_common::identifier::IdentifierType>(
    device_name: &Identifier<T>,
) -> String {
    format!("{}_ral_smoke_pkg", device_name.to_case(Case::Snake))
}

fn sv_ral_smoke_class<T: device_driver_common::identifier::IdentifierType>(
    device_name: &Identifier<T>,
) -> String {
    format!("{}_ral_smoke_seq", device_name.to_case(Case::Snake))
}

fn sv_ral_smoke_guard<T: device_driver_common::identifier::IdentifierType>(
    device_name: &Identifier<T>,
) -> String {
    format!("{}_RAL_SMOKE_SEQ_SV", device_name.to_case(Case::UpperSnake))
}

// =============================================================================
// Bus wrapper rendering
// =============================================================================
//
// Each requested `sv-bus:` value produces one wrapper `module <dev>_<bus>`
// that surrounds the native `<dev>_regs` CPUIF port set in the matching
// standard interface. M4a ships:
//   - `native`  → trivial structural passthrough (exposes the CPUIF as-is)
//   - `apb3`    → APB3 handshake (PSEL/PENABLE/PREADY/PSLVERR)
// `apb4` and `axi4lite` land in M4b.

#[derive(Template)]
#[template(path = "systemverilog/native.sv.j2", escape = "none")]
pub struct NativeWrapperSv<'a> {
    dev_name: &'a str,
    has_hw_in_port: bool,
    has_root_irq: bool,
    intr_groups: Vec<String>,
}

#[derive(Template)]
#[template(path = "systemverilog/apb3.sv.j2", escape = "none")]
pub struct Apb3WrapperSv<'a> {
    dev_name: &'a str,
    has_hw_in_port: bool,
    has_root_irq: bool,
    intr_groups: Vec<String>,
}

#[derive(Template)]
#[template(path = "systemverilog/apb4.sv.j2", escape = "none")]
pub struct Apb4WrapperSv<'a> {
    dev_name: &'a str,
    has_hw_in_port: bool,
    has_root_irq: bool,
    intr_groups: Vec<String>,
}

#[derive(Template)]
#[template(path = "systemverilog/axi4lite.sv.j2", escape = "none")]
pub struct Axi4LiteWrapperSv<'a> {
    dev_name: &'a str,
    has_hw_in_port: bool,
    has_root_irq: bool,
    intr_groups: Vec<String>,
}

#[derive(Template)]
#[template(path = "systemverilog/ahblite.sv.j2", escape = "none")]
pub struct AhbLiteWrapperSv<'a> {
    dev_name: &'a str,
    has_hw_in_port: bool,
    has_root_irq: bool,
    intr_groups: Vec<String>,
}

/// Compute whether the root block emits an `hwif_in` port — used by the
/// wrapper templates to mirror the conditional port on the inner regs module.
fn root_has_hw_in_port(driver: &Driver) -> bool {
    for device in &driver.devices {
        for block in device.blocks.iter().filter(|b| b.root) {
            for method in &block.methods {
                if let BlockMethodType::Command {
                    field_set_name_out,
                    hw_handshake,
                    ..
                } = &method.method_type
                {
                    if matches!(hw_handshake, Some(HwHandshake::Strobe))
                        && field_set_name_out
                            .as_ref()
                            .and_then(|n| driver.field_sets.iter().find(|fs| fs.name == *n))
                            .is_some_and(|fs| fs.size_bytes > 0)
                    {
                        return true;
                    }
                    continue;
                }
                if let BlockMethodType::Buffer {
                    access, hw_kind, ..
                } = &method.method_type
                {
                    if matches!(hw_kind, Some(HwKind::Fifo))
                        && matches!(access, Access::RW | Access::RO)
                    {
                        return true;
                    }
                    continue;
                }
                if matches!(
                    &method.method_type,
                    BlockMethodType::Register { external: true, .. }
                ) {
                    return true;
                }
                let Some(fs) = driver.field_sets.iter().find(|fs| {
                    matches!(
                        &method.method_type,
                        BlockMethodType::Register { field_set_name, .. }
                            if fs.name == *field_set_name
                    )
                }) else {
                    continue;
                };
                if fs.fields.iter().any(|f| {
                    matches!(f.hw_access, HwAccess::RW | HwAccess::WO)
                        || f.hw_clr
                        || f.hw_set
                        || f.is_interrupt()
                }) {
                    return true;
                }
            }
        }
    }
    false
}

/// Collect the irq-output presence + distinct group list off the root block.
/// Wrappers mirror these on their inner regs instantiation.
fn root_irq_info(driver: &Driver) -> (bool, Vec<String>) {
    let mut has_root = false;
    let mut groups: Vec<String> = Vec::new();
    for device in &driver.devices {
        for block in device.blocks.iter().filter(|b| b.root) {
            let aggregate = block.intr_aggregate;
            for method in &block.methods {
                let Some(fs) = driver.field_sets.iter().find(|fs| {
                    matches!(
                        &method.method_type,
                        BlockMethodType::Register { field_set_name, .. }
                            if fs.name == *field_set_name
                    )
                }) else {
                    continue;
                };
                for f in &fs.fields {
                    if !f.is_interrupt() {
                        continue;
                    }
                    match &f.intr_group {
                        Some(g) => {
                            let s = g.to_case(Case::Snake);
                            if !s.is_empty() && !groups.contains(&s) {
                                groups.push(s);
                            } else if s.is_empty() && aggregate {
                                has_root = true;
                            }
                        }
                        None => {
                            if aggregate {
                                has_root = true;
                            }
                        }
                    }
                }
            }
        }
    }
    groups.sort();
    (has_root, groups)
}

pub fn render_bus_wrapper(
    driver: &Driver,
    bus: SvBus,
    dev_name: &str,
    _opts: &SvCodegenOptions,
) -> String {
    let has_hw_in_port = root_has_hw_in_port(driver);
    let (has_root_irq, intr_groups) = root_irq_info(driver);
    match bus {
        SvBus::Native => NativeWrapperSv {
            dev_name,
            has_hw_in_port,
            has_root_irq,
            intr_groups,
        }
        .to_string(),
        SvBus::Apb3 => Apb3WrapperSv {
            dev_name,
            has_hw_in_port,
            has_root_irq,
            intr_groups,
        }
        .to_string(),
        SvBus::Apb4 => Apb4WrapperSv {
            dev_name,
            has_hw_in_port,
            has_root_irq,
            intr_groups,
        }
        .to_string(),
        SvBus::Axi4Lite => Axi4LiteWrapperSv {
            dev_name,
            has_hw_in_port,
            has_root_irq,
            intr_groups,
        }
        .to_string(),
        SvBus::AhbLite => AhbLiteWrapperSv {
            dev_name,
            has_hw_in_port,
            has_root_irq,
            intr_groups,
        }
        .to_string(),
    }
}

// =============================================================================
// SVA checker rendering (M5b)
// =============================================================================

/// One non-clock/non-reset port on the SVA checker module. Either a per-reg
/// hit signal or a per-reg storage word; rendered into the SVA module's
/// port list and tied to `verilog _unused_sva` at the bottom.
pub struct SvaPort {
    pub name: String,
    /// SV type fragment without the trailing port name, e.g. `logic` or
    /// `logic [31:0]`.
    pub kind: String,
}

pub struct SvaResetEntry {
    pub suffix: String,
    pub storage: String,
    pub reset_lit: String,
}

pub struct SvaW1cEntry {
    pub suffix: String,
    pub wr_hit: String,
    pub storage_bit: String,
    /// Expression for the mask of bits the SW write asked to clear, e.g.
    /// `cpuif_wr_data[3] & cpuif_wr_biten[3]` for a single-bit field.
    pub cleared_mask: String,
}

pub struct SvaRoEntry {
    pub suffix: String,
    pub storage_bit: String,
    pub reset_lit: String,
}

#[derive(Template)]
#[template(path = "systemverilog/sva.sv.j2", escape = "none")]
pub struct SvaCheckerSv<'a> {
    block: &'a Block,
    driver: &'a Driver,
    opts: SvAssertOpts,
}

#[derive(Template)]
#[template(path = "systemverilog/sva_bind.svh.j2", escape = "none")]
pub struct SvaBindSv<'a> {
    block: &'a Block,
    #[allow(dead_code)]
    driver: &'a Driver,
}

impl<'a> SvaCheckerSv<'a> {
    fn find_field_set(
        &self,
        name: &Identifier<device_driver_common::identifier::Type>,
    ) -> Option<&FieldSet> {
        self.driver.field_sets.iter().find(|fs| fs.name == *name)
    }

    fn method_field_set(&self, method: &BlockMethod) -> Option<&FieldSet> {
        let name = match &method.method_type {
            BlockMethodType::Register { field_set_name, .. } => field_set_name,
            _ => return None,
        };
        self.find_field_set(name)
    }

    fn instances(&self, method: &BlockMethod) -> Vec<RegInstance> {
        match &method.repeat {
            Repeat::None => vec![RegInstance {
                suffix_snake: String::new(),
                suffix_upper: String::new(),
                address: method.address,
            }],
            Repeat::Count { count, stride } => (0..*count)
                .map(|i| RegInstance {
                    suffix_snake: format!("_{}", i),
                    suffix_upper: format!("_{}", i),
                    address: method.address + (i as i128) * stride,
                })
                .collect(),
            Repeat::Enum {
                enum_name,
                enum_variants,
                stride,
            } => {
                let enum_def = self.driver.enums.iter().find(|e| e.name == *enum_name);
                enum_variants
                    .iter()
                    .map(|var_name| {
                        let discriminant = enum_def
                            .and_then(|e| e.variants.iter().find(|v| v.name == *var_name))
                            .map(|v| v.discriminant)
                            .unwrap_or(0);
                        RegInstance {
                            suffix_snake: format!("_{}", var_name.to_case(Case::Snake)),
                            suffix_upper: format!("_{}", var_name.to_case(Case::UpperSnake)),
                            address: method.address + discriminant * stride,
                        }
                    })
                    .collect()
            }
        }
    }

    /// Reset literal slice for a single field, mirroring the codegen helper in
    /// `DeviceTemplateSv` so the SVA module checks the same value.
    fn field_reset_lit(&self, method: &BlockMethod, field: &Field) -> String {
        let width = field.address.end - field.address.start + 1;
        let reset_bytes = match &method.method_type {
            BlockMethodType::Register { reset_value, .. } => {
                reset_value.as_ref().map(|rv| rv.value.as_slice())
            }
            _ => None,
        };
        let Some(bytes) = reset_bytes else {
            return "'0".to_string();
        };
        let mut value: u128 = 0;
        for (i, b) in bytes.iter().enumerate().take(16) {
            value |= (*b as u128) << (i * 8);
        }
        let mask: u128 = if width >= 128 {
            u128::MAX
        } else {
            (1u128 << width) - 1
        };
        let slice_val = (value >> field.address.start) & mask;
        format!("{}'h{:x}", width, slice_val)
    }

    fn reg_reset_word(&self, method: &BlockMethod, fs: &FieldSet) -> String {
        let width = fs.size_bytes * 8;
        let bytes = match &method.method_type {
            BlockMethodType::Register { reset_value, .. } => {
                reset_value.as_ref().map(|rv| rv.value.as_slice())
            }
            _ => None,
        };
        let Some(bytes) = bytes else {
            return "'0".to_string();
        };
        let mut value: u128 = 0;
        for (i, b) in bytes.iter().enumerate().take(16) {
            value |= (*b as u128) << (i * 8);
        }
        let cap: u128 = if width >= 128 {
            u128::MAX
        } else {
            (1u128 << width) - 1
        };
        value &= cap;
        format!("{}'h{:x}", width, value)
    }

    /// Per-register hit + storage ports, in stable order. Used by both the
    /// port-list template loop and the `_unused_sva` reduction.
    fn sva_signal_ports(&self, block: &Block) -> Vec<SvaPort> {
        let mut out = Vec::new();
        for method in &block.methods {
            if !matches!(method.method_type, BlockMethodType::Register { .. }) {
                continue;
            }
            let Some(fs) = self.method_field_set(method) else {
                continue;
            };
            for inst in self.instances(method) {
                let reg = sv_reg_name(&method.name);
                let storage = sv_reg_storage_name(&method.name);
                out.push(SvaPort {
                    name: format!("wr_hit_{reg}{}", inst.suffix_snake),
                    kind: "logic".to_string(),
                });
                out.push(SvaPort {
                    name: format!("rd_hit_{reg}{}", inst.suffix_snake),
                    kind: "logic".to_string(),
                });
                out.push(SvaPort {
                    name: format!("{storage}{}", inst.suffix_snake),
                    kind: format!("logic [{}:0]", sv_reg_storage_width(fs) - 1),
                });
            }
        }
        out
    }

    /// Just the `{wr,rd}_hit_*` names — fed into the `$onehot0` argument in
    /// the decode-mutex assertion.
    fn sva_hit_signal_names(&self, block: &Block) -> Vec<String> {
        let mut out = Vec::new();
        for method in &block.methods {
            if !matches!(method.method_type, BlockMethodType::Register { .. }) {
                continue;
            }
            for inst in self.instances(method) {
                let reg = sv_reg_name(&method.name);
                out.push(format!("wr_hit_{reg}{}", inst.suffix_snake));
                out.push(format!("rd_hit_{reg}{}", inst.suffix_snake));
            }
        }
        out
    }

    fn sva_reset_entries(&self, block: &Block) -> Vec<SvaResetEntry> {
        let mut out = Vec::new();
        for method in &block.methods {
            if !matches!(method.method_type, BlockMethodType::Register { .. }) {
                continue;
            }
            let Some(fs) = self.method_field_set(method) else {
                continue;
            };
            for inst in self.instances(method) {
                let reg = sv_reg_name(&method.name);
                let storage = sv_reg_storage_name(&method.name);
                out.push(SvaResetEntry {
                    suffix: format!("{reg}{}", inst.suffix_snake),
                    storage: format!("{storage}{}", inst.suffix_snake),
                    reset_lit: self.reg_reset_word(method, fs),
                });
            }
        }
        out
    }

    fn sva_w1c_entries(&self, block: &Block) -> Vec<SvaW1cEntry> {
        let mut out = Vec::new();
        for method in &block.methods {
            if !matches!(method.method_type, BlockMethodType::Register { .. }) {
                continue;
            }
            let Some(fs) = self.method_field_set(method) else {
                continue;
            };
            for inst in self.instances(method) {
                for field in &fs.fields {
                    if !matches!(field.on_write, OnWrite::Clear) {
                        continue;
                    }
                    let reg = sv_reg_name(&method.name);
                    let storage = sv_reg_storage_name(&method.name);
                    let slice = sv_field_slice(&field.address);
                    let storage_bit = format!("{storage}{}{slice}", inst.suffix_snake);
                    let cleared_mask = format!("cpuif_wr_data{slice} & cpuif_wr_biten{slice}");
                    out.push(SvaW1cEntry {
                        suffix: format!("{reg}{}_{}", inst.suffix_snake, sv_field_name(field)),
                        wr_hit: format!("wr_hit_{reg}{}", inst.suffix_snake),
                        storage_bit,
                        cleared_mask,
                    });
                }
            }
        }
        out
    }

    fn sva_ro_entries(&self, block: &Block) -> Vec<SvaRoEntry> {
        let mut out = Vec::new();
        for method in &block.methods {
            if !matches!(method.method_type, BlockMethodType::Register { .. }) {
                continue;
            }
            let Some(fs) = self.method_field_set(method) else {
                continue;
            };
            for inst in self.instances(method) {
                for field in &fs.fields {
                    // Only fully-RO fields qualify — anything that can be
                    // legitimately written by either side breaks `$stable`.
                    let fully_ro = matches!(field.access, Access::RO)
                        && matches!(field.hw_access, HwAccess::RO)
                        && !field.hw_clr
                        && !field.hw_set
                        && !field.is_interrupt()
                        && matches!(field.on_read, OnRead::Store);
                    if !fully_ro {
                        continue;
                    }
                    let reg = sv_reg_name(&method.name);
                    let storage = sv_reg_storage_name(&method.name);
                    let slice = sv_field_slice(&field.address);
                    out.push(SvaRoEntry {
                        suffix: format!("{reg}{}_{}", inst.suffix_snake, sv_field_name(field)),
                        storage_bit: format!("{storage}{}{slice}", inst.suffix_snake),
                        reset_lit: self.field_reset_lit(method, field),
                    });
                }
            }
        }
        out
    }
}

// =============================================================================
// UVM RAL package rendering (M6c)
// =============================================================================

pub struct RalField {
    pub snake_name: String,
    pub width: u32,
    pub lsb: u32,
    /// UVM access string — derived from the field's combined SW access and
    /// on-write modifier. See `ral_uvm_access` for the mapping table.
    pub uvm_access: String,
    /// `1'b1` if HW can change the storage out from under SW (volatile),
    /// `1'b0` otherwise. Maps directly to the `volatile` bit on
    /// `uvm_reg_field::configure`.
    pub volatile_lit: String,
    pub reset_lit: String,
}

pub struct RalRegister {
    pub snake_name: String,
    pub storage_name: String,
    pub uvm_class: String,
    pub width_bits: u32,
    /// Lowercase hex, no width prefix — embedded inside the template's
    /// `<W>'h<X>` literal.
    pub addr_hex: String,
    /// UVM register-level access (`"RW"`, `"RO"`, `"WO"`).
    pub uvm_access: String,
    pub fields: Vec<RalField>,
    /// Lowercase-hex write pattern used by the smoke sequence. Derived
    /// from the register's storage width — high half 0xA, low half 0x5
    /// (alternating nibbles), capped to the field width.
    pub smoke_pattern: String,
}

#[derive(Template)]
#[template(path = "systemverilog/ral_pkg.sv.j2", escape = "none")]
pub struct RalPkgSv<'a> {
    block: &'a Block,
    driver: &'a Driver,
    compile_options: &'a SvCodegenOptions,
}

#[derive(Template)]
#[template(path = "systemverilog/ral_smoke_seq.sv.j2", escape = "none")]
pub struct RalSmokeSv<'a> {
    block: &'a Block,
    driver: &'a Driver,
    compile_options: &'a SvCodegenOptions,
}

impl<'a> RalSmokeSv<'a> {
    // Reuses the same field-walking machinery as `RalPkgSv` — wrap
    // through the underlying `RalPkgSv` to avoid duplicating logic.
    fn ral_registers(&self) -> Vec<RalRegister> {
        RalPkgSv {
            block: self.block,
            driver: self.driver,
            compile_options: self.compile_options,
        }
        .ral_registers()
    }
}

impl<'a> RalPkgSv<'a> {
    fn data_width(&self) -> u32 {
        if let Some(w) = self.block.sv_data_width {
            return w;
        }
        self.compile_options.data_width.unwrap_or(32)
    }

    fn ral_addr_width(&self) -> u32 {
        if let Some(w) = self.compile_options.addr_width {
            return w;
        }
        self.driver
            .devices
            .first()
            .map(|d| d.internal_address_type.size_bits())
            .unwrap_or(32)
    }

    fn find_field_set(
        &self,
        name: &Identifier<device_driver_common::identifier::Type>,
    ) -> Option<&FieldSet> {
        self.driver.field_sets.iter().find(|fs| fs.name == *name)
    }

    fn has_hdl_path_prefix(&self) -> bool {
        self.block
            .sv_hdl_path_prefix
            .as_deref()
            .is_some_and(|s| !s.is_empty())
    }

    fn ral_hdl_path_prefix(&self) -> String {
        self.block.sv_hdl_path_prefix.clone().unwrap_or_default()
    }

    /// Walk every Register method on the root block, materialise the data
    /// the template needs once per render.
    fn ral_registers(&self) -> Vec<RalRegister> {
        let mut out = Vec::new();
        for method in &self.block.methods {
            let BlockMethodType::Register {
                field_set_name,
                access,
                reset_value,
                ..
            } = &method.method_type
            else {
                continue;
            };
            let Some(fs) = self.find_field_set(field_set_name) else {
                continue;
            };
            let reg_access = match access {
                Access::RW => "RW",
                Access::RO => "RO",
                Access::WO => "WO",
            };
            let reset_bytes: Option<&[u8]> = reset_value.as_ref().map(|rv| rv.value.as_slice());
            let mut reset_word: u128 = 0;
            if let Some(bytes) = reset_bytes {
                for (i, b) in bytes.iter().enumerate().take(16) {
                    reset_word |= (*b as u128) << (i * 8);
                }
            }
            let mut fields = Vec::new();
            for field in &fs.fields {
                let width = field.address.end - field.address.start + 1;
                let mask: u128 = if width >= 128 {
                    u128::MAX
                } else {
                    (1u128 << width) - 1
                };
                let reset_slice = (reset_word >> field.address.start) & mask;
                let volatile = matches!(field.hw_access, HwAccess::RW | HwAccess::WO)
                    || field.hw_clr
                    || field.hw_set
                    || field.is_interrupt();
                fields.push(RalField {
                    snake_name: sv_field_name(field),
                    width,
                    lsb: field.address.start,
                    uvm_access: ral_uvm_access(field, *access),
                    volatile_lit: if volatile { "1'b1" } else { "1'b0" }.to_string(),
                    reset_lit: format!("{}'h{:x}", width, reset_slice),
                });
            }
            // Smoke pattern: alternating `0xA5` bytes capped to the
            // register's storage width. Avoids both all-zero (would
            // mirror reset) and all-ones (which conflicts with most
            // reserved-bit policies); shifts naturally for non-power-of-2
            // widths.
            let width_bits = sv_reg_storage_width(fs);
            let cap: u128 = if width_bits >= 128 {
                u128::MAX
            } else {
                (1u128 << width_bits) - 1
            };
            let mut pat: u128 = 0;
            for byte_idx in 0..(width_bits.div_ceil(8)) {
                let byte_val = if byte_idx & 1 == 0 {
                    0xA5u128
                } else {
                    0x5Au128
                };
                pat |= byte_val << (byte_idx * 8);
            }
            pat &= cap;
            let smoke_pattern = format!("{:x}", pat);
            out.push(RalRegister {
                snake_name: sv_reg_name(&method.name),
                storage_name: sv_reg_storage_name(&method.name),
                uvm_class: sv_ral_reg_class(&self.block.name, &method.name),
                width_bits,
                addr_hex: format!("{:x}", method.address),
                uvm_access: reg_access.to_string(),
                fields,
                smoke_pattern,
            });
        }
        out
    }
}

/// Map a DDSL field's combined SW access + on-write + on-read modifiers onto
/// a UVM access string. Covers the W1C/W1S/W1T family from `on-write` and
/// the RC/RS read-side modifiers from `on-read`, plus the combined WRC/WRS
/// forms when both sides act. Falls back to the bare register-level access
/// string for everything else.
fn ral_uvm_access(field: &Field, reg_access: Access) -> String {
    let effective_access = match (reg_access, field.access) {
        (Access::RO, _) | (_, Access::RO) => Access::RO,
        (Access::WO, _) | (_, Access::WO) => Access::WO,
        _ => Access::RW,
    };
    let read_side = field.on_read;
    match (effective_access, field.on_write, read_side) {
        // Read-only with read-side modifier
        (Access::RO, _, OnRead::Clear) => "RC".to_string(),
        (Access::RO, _, OnRead::Set) => "RS".to_string(),
        (Access::RO, _, OnRead::Store) => "RO".to_string(),
        // RW with both write- and read-side modifiers
        (Access::RW, OnWrite::Clear, OnRead::Clear) => "WRC".to_string(),
        (Access::RW, OnWrite::Set, OnRead::Set) => "WRS".to_string(),
        (Access::RW, OnWrite::Store, OnRead::Clear) => "WRC".to_string(),
        (Access::RW, OnWrite::Store, OnRead::Set) => "WRS".to_string(),
        // RW write-side only
        (Access::RW, OnWrite::Clear, _) => "W1C".to_string(),
        (Access::RW, OnWrite::Set, _) => "W1S".to_string(),
        (Access::RW, OnWrite::Toggle, _) => "W1T".to_string(),
        (Access::RW, OnWrite::Store, OnRead::Store) => "RW".to_string(),
        (Access::WO, _, _) => "WO".to_string(),
    }
}

/// Render the UVM RAL package + smoke sequence for a driver. Returns
/// `(ral_pkg.sv, ral_smoke_seq.sv)` when `sv-ral` is enabled on the root
/// block, `None` otherwise.
pub fn render_ral_files(
    driver: &Driver,
    compile_options: &SvCodegenOptions,
) -> Option<(String, String)> {
    let root = driver
        .devices
        .iter()
        .flat_map(|d| d.blocks.iter())
        .find(|b| b.root)?;
    if !root.sv_ral {
        return None;
    }
    let pkg = RalPkgSv {
        block: root,
        driver,
        compile_options,
    }
    .to_string();
    let smoke = RalSmokeSv {
        block: root,
        driver,
        compile_options,
    }
    .to_string();
    Some((pkg, smoke))
}

/// Render the SVA pair for a driver. Returns `(sva.sv, sva_bind.svh)` when
/// any assertion category is enabled on the root block, `None` otherwise.
pub fn render_sva_files(driver: &Driver) -> Option<(String, String)> {
    let root = driver
        .devices
        .iter()
        .flat_map(|d| d.blocks.iter())
        .find(|b| b.root)?;
    if !root.sv_assertions.any() {
        return None;
    }
    let sva = SvaCheckerSv {
        block: root,
        driver,
        opts: root.sv_assertions,
    }
    .to_string();
    let bind = SvaBindSv {
        block: root,
        driver,
    }
    .to_string();
    Some((sva, bind))
}
