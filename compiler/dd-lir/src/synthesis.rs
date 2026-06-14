//! Post-lowering synthesis pass that materialises companion registers
//! the user opted into via field-level `intr-enable` / `intr-mask` or
//! buffer-level `status-address`. Companion registers are appended to
//! the root `Block` alongside user-declared ones; companion fieldsets
//! are appended to `Driver.field_sets`. Codegen treats them like any
//! other register / fieldset.

use device_driver_common::{
    identifier::{Identifier, IdentifierType},
    specifiers::{
        Access, AddressRange, BaseType, ByteOrder, HwAccess, OnRead, OnWrite, Precedence,
    },
};
use device_driver_diagnostics::DynError;

use crate::model::{
    BlockMethod, BlockMethodType, Driver, Field, FieldConversionMethod, FieldSet, Repeat,
};

/// Build an `Identifier` with default boundaries applied. Synthesis runs
/// after MIR passes, so newly-created identifiers must apply boundaries
/// themselves before codegen calls `to_case` on them.
fn ident<T: IdentifierType + Default>(name: &str) -> Identifier<T> {
    let mut id: Identifier<T> = Identifier::try_parse(name).unwrap();
    id.apply_boundaries(&const { convert_case::Boundary::defaults() });
    id
}

/// Trace of what `synthesize_companion_registers` appended to the driver,
/// so the post-synthesis collision check can target only those entries
/// (vs. flagging existing user overlaps that earlier MIR passes already
/// vetted).
#[derive(Default)]
pub struct SynthesisTrace {
    pub synth_method_names: Vec<String>,
    pub synth_field_set_names: Vec<String>,
}

pub fn synthesize_companion_registers(driver: &mut Driver) -> SynthesisTrace {
    let mut trace = SynthesisTrace::default();
    synthesize_intr_companions(driver, &mut trace);
    synthesize_buffer_status_companions(driver, &mut trace);
    trace
}

/// Reject duplicate explicit `intr-enable-bit:` / `intr-mask-bit:`
/// assignments within the same IRQ group + side. Two opt-in fields
/// can't both claim bit 5 of `<group>_intr_enable`. Declaration-order
/// allocation also slots around the explicit pins (so an unpinned
/// field can't accidentally collide either).
pub fn check_explicit_intr_bits(driver: &Driver) -> Result<(), DynError> {
    use std::collections::BTreeMap;
    #[derive(Default)]
    struct GroupBits {
        // bit -> first field that claimed it
        enable: BTreeMap<u32, String>,
        mask: BTreeMap<u32, String>,
    }
    let mut by_group: BTreeMap<String, GroupBits> = BTreeMap::new();
    for dev in &driver.devices {
        for block in dev.blocks.iter().filter(|b| b.root) {
            for method in &block.methods {
                let BlockMethodType::Register { field_set_name, .. } = &method.method_type else {
                    continue;
                };
                let Some(fs) = driver
                    .field_sets
                    .iter()
                    .find(|fs| fs.name == *field_set_name)
                else {
                    continue;
                };
                for f in &fs.fields {
                    if f.intr_trigger.is_none() {
                        continue;
                    }
                    let group = f.intr_group.clone().unwrap_or_default();
                    let entry = by_group.entry(group.clone()).or_default();
                    if f.intr_enable
                        && let Some(bit) = f.intr_enable_bit
                        && let Some(prev) = entry.enable.insert(bit, f.name.original().to_string())
                    {
                        let group_label = if group.is_empty() {
                            "root".to_string()
                        } else {
                            group
                        };
                        return Err(DynError::new(format!(
                            "duplicate explicit `intr-enable-bit: {bit}` in IRQ group `{group_label}`: \
                             fields `{prev}` and `{}` both claim bit {bit}. Pick distinct \
                             bit positions, or remove one of the `intr-enable-bit:` knobs to fall \
                             back to declaration order.",
                            f.name.original(),
                        )));
                    }
                    if f.intr_mask
                        && let Some(bit) = f.intr_mask_bit
                        && let Some(prev) = entry.mask.insert(bit, f.name.original().to_string())
                    {
                        let group_label = if group.is_empty() {
                            "root".to_string()
                        } else {
                            group
                        };
                        return Err(DynError::new(format!(
                            "duplicate explicit `intr-mask-bit: {bit}` in IRQ group `{group_label}`: \
                             fields `{prev}` and `{}` both claim bit {bit}. Pick distinct \
                             bit positions, or remove one of the `intr-mask-bit:` knobs to fall \
                             back to declaration order.",
                            f.name.original(),
                        )));
                    }
                }
            }
        }
    }
    Ok(())
}

/// For every distinct intr group that has at least one field opted into
/// `intr-enable` / `intr-mask`, emit a packed companion register whose
/// bits line up with the source fields' position in the group's status
/// register.
fn synthesize_intr_companions(driver: &mut Driver, trace: &mut SynthesisTrace) {
    // Collect (group, source_reg, source_field, bit) per opt-in side.
    struct Source {
        group: String,
        field_name: String,
        // Storage-bit position in the synthesized companion register.
        // Explicit `intr-enable-bit:` / `intr-mask-bit:` wins; otherwise
        // we fall back to declaration order per group.
        bit: u32,
    }
    fn collect(driver: &Driver, side: Side) -> Vec<Source> {
        // Two-pass per group: first place explicit bits, then fill the
        // remaining declaration-order entries into the unclaimed bits
        // starting at 0. This way an explicit `intr-enable-bit: 5`
        // pins to bit 5 and a later un-pinned field gets bit 0 (or the
        // next free slot below 5).
        struct Pending {
            field_name: String,
            explicit_bit: Option<u32>,
        }
        let mut by_group: std::collections::BTreeMap<String, Vec<Pending>> =
            std::collections::BTreeMap::new();
        for dev in &driver.devices {
            for block in dev.blocks.iter().filter(|b| b.root) {
                for method in &block.methods {
                    let BlockMethodType::Register { field_set_name, .. } = &method.method_type
                    else {
                        continue;
                    };
                    let Some(fs) = driver
                        .field_sets
                        .iter()
                        .find(|fs| fs.name == *field_set_name)
                    else {
                        continue;
                    };
                    for f in &fs.fields {
                        if f.intr_trigger.is_none() {
                            continue;
                        }
                        let opt_in = match side {
                            Side::Enable => f.intr_enable,
                            Side::Mask => f.intr_mask,
                        };
                        if !opt_in {
                            continue;
                        }
                        let explicit_bit = match side {
                            Side::Enable => f.intr_enable_bit,
                            Side::Mask => f.intr_mask_bit,
                        };
                        let group = f.intr_group.clone().unwrap_or_default();
                        by_group.entry(group).or_default().push(Pending {
                            field_name: f.name.original().to_string(),
                            explicit_bit,
                        });
                    }
                }
            }
        }
        let mut out = Vec::new();
        for (group, mut pendings) in by_group {
            // Place explicit bits first; they own those positions.
            let mut claimed: std::collections::BTreeSet<u32> = std::collections::BTreeSet::new();
            for p in &pendings {
                if let Some(bit) = p.explicit_bit {
                    claimed.insert(bit);
                }
            }
            // Stable iterator over the un-pinned ones; allocate the
            // lowest unclaimed bit to each in declaration order.
            let mut next = 0u32;
            for p in pendings.iter_mut() {
                if let Some(bit) = p.explicit_bit {
                    out.push(Source {
                        group: group.clone(),
                        field_name: std::mem::take(&mut p.field_name),
                        bit,
                    });
                } else {
                    while claimed.contains(&next) {
                        next += 1;
                    }
                    claimed.insert(next);
                    out.push(Source {
                        group: group.clone(),
                        field_name: std::mem::take(&mut p.field_name),
                        bit: next,
                    });
                    next += 1;
                }
            }
        }
        out
    }

    #[derive(Clone, Copy)]
    enum Side {
        Enable,
        Mask,
    }

    for side in [Side::Enable, Side::Mask] {
        let sources = collect(driver, side);
        if sources.is_empty() {
            continue;
        }
        // Resolve base address from device config; without it we can't
        // assign concrete addresses, so skip.
        let Some(block) = driver
            .devices
            .iter_mut()
            .flat_map(|d| d.blocks.iter_mut())
            .find(|b| b.root)
        else {
            continue;
        };
        let (base, side_label, side_default) = match side {
            Side::Enable => (block.intr_enable_address_base, "intr_enable", Access::RW),
            Side::Mask => (block.intr_mask_address_base, "intr_mask", Access::RW),
        };
        let Some(base_addr) = base else {
            continue;
        };

        // Bucket sources by group, in deterministic order.
        let mut by_group: std::collections::BTreeMap<String, Vec<&Source>> =
            std::collections::BTreeMap::new();
        for s in &sources {
            by_group.entry(s.group.clone()).or_default().push(s);
        }

        for (group_idx, (group, srcs)) in by_group.into_iter().enumerate() {
            let group_name = if group.is_empty() {
                "root".to_string()
            } else {
                group
            };
            let fs_name = ident(&format!("{group_name}_{side_label}_fields"));
            let reg_name = ident(&format!("{group_name}_{side_label}"));
            let size_bytes = 4u32;
            let mut fields = Vec::new();
            for s in &srcs {
                fields.push(Field {
                    description: format!(
                        "Synthesized {side_label} bit for `{}` in group `{group_name}`",
                        s.field_name
                    ),
                    name: ident(&s.field_name),
                    address: AddressRange {
                        start: s.bit,
                        end: s.bit,
                    },
                    base_type: BaseType::Bool.to_string(),
                    conversion_method: FieldConversionMethod::Bool,
                    access: side_default,
                    on_write: OnWrite::Store,
                    on_read: OnRead::Store,
                    hw_access: HwAccess::RO,
                    hw_clr: false,
                    hw_set: false,
                    singlepulse: false,
                    precedence: Precedence::Hw,
                    intr_trigger: None,
                    intr_group: None,
                    intr_sticky: false,
                    intr_enable: false,
                    intr_mask: false,
                    intr_enable_bit: None,
                    intr_mask_bit: None,
                    repeat: Repeat::None,
                });
            }
            let fs = FieldSet {
                description: format!(
                    "Auto-synthesized {side_label} companion for IRQ group `{group_name}`."
                ),
                name: fs_name.clone(),
                byte_order: ByteOrder::LE,
                size_bytes,
                fields,
            };
            let bm = BlockMethod {
                description: format!(
                    "Auto-synthesized {side_label} companion for IRQ group `{group_name}`."
                ),
                name: reg_name,
                address: base_addr + (group_idx as i128) * (size_bytes as i128),
                repeat: Repeat::None,
                method_type: BlockMethodType::Register {
                    field_set_name: fs_name,
                    access: side_default,
                    reset_value: None,
                    reserved_behavior: Default::default(),
                    external: false,
                },
            };
            trace
                .synth_method_names
                .push(bm.name.original().to_string());
            trace
                .synth_field_set_names
                .push(fs.name.original().to_string());
            driver.field_sets.push(fs);
            block.methods.push(bm);
        }
    }
}

/// FIFO status companion register synthesis. For each buffer with
/// `hw-kind: fifo` AND a `status-address`, emit an RO register at the
/// status address with `level`, `full`, `empty`, `almost_full` fields.
/// User RTL drives these via the existing HW-input plumbing (hw-access
/// RW + _we strobes already supported).
fn synthesize_buffer_status_companions(driver: &mut Driver, trace: &mut SynthesisTrace) {
    use device_driver_common::specifiers::HwKind;

    // First collect what we need to add without holding a mutable
    // borrow on the driver fields the loop reads.
    struct ToAdd {
        fs: FieldSet,
        method: BlockMethod,
    }
    let mut additions: Vec<ToAdd> = Vec::new();

    for dev in &driver.devices {
        for block in dev.blocks.iter().filter(|b| b.root) {
            for method in &block.methods {
                let BlockMethodType::Buffer {
                    hw_kind,
                    depth,
                    status_address: Some(addr),
                    ..
                } = &method.method_type
                else {
                    continue;
                };
                if !matches!(hw_kind, Some(HwKind::Fifo)) {
                    continue;
                }
                let buf_name = method.name.original();
                let fs_name = ident(&format!("{buf_name}_status_fields"));
                let reg_name = ident(&format!("{buf_name}_status"));
                // `level` width comes from log2(depth + 1) to count
                // {0..depth} inclusive. Default to 16 if not set.
                let depth = depth.unwrap_or(16);
                let level_width = depth
                    .saturating_add(1)
                    .next_power_of_two()
                    .trailing_zeros()
                    .max(1);
                let mut bit = 0u32;
                let level_range = AddressRange {
                    start: bit,
                    end: bit + level_width - 1,
                };
                bit += level_width;
                let full_range = AddressRange {
                    start: bit,
                    end: bit,
                };
                bit += 1;
                let empty_range = AddressRange {
                    start: bit,
                    end: bit,
                };
                bit += 1;
                let almost_full_range = AddressRange {
                    start: bit,
                    end: bit,
                };

                let mk_bool = |name: &str, r: AddressRange| Field {
                    description: format!(
                        "FIFO status — {name} (HW-driven by user RTL via `hwif_in.{}_{name}`)",
                        reg_name.original()
                    ),
                    name: ident(name),
                    address: r,
                    base_type: BaseType::Bool.to_string(),
                    conversion_method: FieldConversionMethod::Bool,
                    access: Access::RO,
                    on_write: OnWrite::Store,
                    on_read: OnRead::Store,
                    hw_access: HwAccess::WO,
                    hw_clr: false,
                    hw_set: false,
                    singlepulse: false,
                    precedence: Precedence::Hw,
                    intr_trigger: None,
                    intr_group: None,
                    intr_sticky: false,
                    intr_enable: false,
                    intr_mask: false,
                    intr_enable_bit: None,
                    intr_mask_bit: None,
                    repeat: Repeat::None,
                };
                let level_field = Field {
                    description: format!(
                        "FIFO status — current level (HW-driven by user RTL via `hwif_in.{}_level`)",
                        reg_name.original()
                    ),
                    name: ident("level"),
                    address: level_range,
                    base_type: "u32".to_string(),
                    conversion_method: FieldConversionMethod::None,
                    access: Access::RO,
                    on_write: OnWrite::Store,
                    on_read: OnRead::Store,
                    hw_access: HwAccess::WO,
                    hw_clr: false,
                    hw_set: false,
                    singlepulse: false,
                    precedence: Precedence::Hw,
                    intr_trigger: None,
                    intr_group: None,
                    intr_sticky: false,
                    intr_enable: false,
                    intr_mask: false,
                    intr_enable_bit: None,
                    intr_mask_bit: None,
                    repeat: Repeat::None,
                };

                let fs = FieldSet {
                    description: format!(
                        "Auto-synthesized status companion for FIFO buffer `{buf_name}`."
                    ),
                    name: fs_name.clone(),
                    byte_order: ByteOrder::LE,
                    size_bytes: 4,
                    fields: vec![
                        level_field,
                        mk_bool("full", full_range),
                        mk_bool("empty", empty_range),
                        mk_bool("almost_full", almost_full_range),
                    ],
                };
                let method = BlockMethod {
                    description: format!(
                        "Auto-synthesized status companion for FIFO buffer `{buf_name}`."
                    ),
                    name: reg_name,
                    address: *addr,
                    repeat: Repeat::None,
                    method_type: BlockMethodType::Register {
                        field_set_name: fs_name,
                        access: Access::RO,
                        reset_value: None,
                        reserved_behavior: Default::default(),
                        external: false,
                    },
                };
                additions.push(ToAdd { fs, method });
            }
        }
    }

    let root = driver
        .devices
        .iter_mut()
        .flat_map(|d| d.blocks.iter_mut())
        .find(|b| b.root);
    if let Some(block) = root {
        for ToAdd { fs, method } in additions {
            trace
                .synth_method_names
                .push(method.name.original().to_string());
            trace
                .synth_field_set_names
                .push(fs.name.original().to_string());
            driver.field_sets.push(fs);
            block.methods.push(method);
        }
    }
}

/// Catch silent collisions between synthesized companion registers and
/// user-declared ones. The standard MIR `names_unique` and
/// `addresses_non_overlapping` passes run before synthesis, so they
/// miss anything the synthesis pass adds. Re-check at LIR level after
/// synthesis has appended its work.
///
/// Three categories of collision are surfaced:
///   * Two methods on the root block share a name.
///   * Two field sets share a name.
///   * Two registers on the root block occupy overlapping address ranges
///     (using each register's fieldset `size_bytes` to derive the range).
///
/// The diagnostic includes a hint that synthesis is likely the culprit,
/// since these collisions are rare in hand-written DSL alone — the
/// MIR passes would have caught them earlier.
pub fn check_companion_collisions(driver: &Driver, trace: &SynthesisTrace) -> Result<(), DynError> {
    let Some(root_block) = driver
        .devices
        .iter()
        .flat_map(|d| &d.blocks)
        .find(|b| b.root)
    else {
        return Ok(());
    };
    if trace.synth_method_names.is_empty() && trace.synth_field_set_names.is_empty() {
        return Ok(());
    }

    let synth_methods: std::collections::HashSet<&str> = trace
        .synth_method_names
        .iter()
        .map(|s| s.as_str())
        .collect();
    let synth_field_sets: std::collections::HashSet<&str> = trace
        .synth_field_set_names
        .iter()
        .map(|s| s.as_str())
        .collect();

    // ---- 1. Method-name collision involving a synthesized entry ------
    let mut method_name_count: std::collections::HashMap<&str, usize> =
        std::collections::HashMap::new();
    for method in &root_block.methods {
        *method_name_count.entry(method.name.original()).or_insert(0) += 1;
    }
    for synth_name in &synth_methods {
        if method_name_count.get(synth_name).copied().unwrap_or(0) > 1 {
            return Err(DynError::new(format!(
                "synthesized companion register `{synth_name}` collides with a \
                 user-declared register of the same name. Companion registers \
                 come from `intr-enable-address-base` / `intr-mask-address-base` \
                 (named `<group>_intr_enable` / `<group>_intr_mask`) or from a \
                 buffer's `status-address` (named `<buf>_status`). Pick a \
                 different name on the user-declared side or remove the \
                 synthesis trigger."
            )));
        }
    }

    // ---- 2. Field-set-name collision involving a synthesized entry ---
    let mut fs_name_count: std::collections::HashMap<&str, usize> =
        std::collections::HashMap::new();
    for fs in &driver.field_sets {
        *fs_name_count.entry(fs.name.original()).or_insert(0) += 1;
    }
    for synth_name in &synth_field_sets {
        if fs_name_count.get(synth_name).copied().unwrap_or(0) > 1 {
            return Err(DynError::new(format!(
                "synthesized companion fieldset `{synth_name}` collides with a \
                 user-declared fieldset of the same name. Synthesized fieldsets \
                 are named `<group>_intr_enable_fields`, \
                 `<group>_intr_mask_fields`, or `<buf>_status_fields` — rename \
                 the user fieldset or remove the synthesis trigger."
            )));
        }
    }

    // ---- 3. Address-range overlap where at least one register is -----
    //         synthesized (existing user-vs-user overlaps were already
    //         validated by the MIR `addresses_non_overlapping` pass).
    let mut ranges: Vec<(i128, i128, &str)> = Vec::new();
    for method in &root_block.methods {
        let BlockMethodType::Register { field_set_name, .. } = &method.method_type else {
            continue;
        };
        let Some(fs) = driver
            .field_sets
            .iter()
            .find(|fs| fs.name == *field_set_name)
        else {
            continue;
        };
        let start = method.address;
        let end = method.address + (fs.size_bytes as i128) - 1;
        let name = method.name.original();
        let is_synth = synth_methods.contains(name);
        for (other_start, other_end, other_name) in &ranges {
            let overlaps = start <= *other_end && end >= *other_start;
            if !overlaps {
                continue;
            }
            let other_is_synth = synth_methods.contains(other_name);
            if !is_synth && !other_is_synth {
                // User-only overlap — already validated upstream; skip.
                continue;
            }
            let (synth_name, synth_range, user_name, user_range) = if is_synth {
                (name, (start, end), *other_name, (*other_start, *other_end))
            } else {
                (*other_name, (*other_start, *other_end), name, (start, end))
            };
            return Err(DynError::new(format!(
                "synthesized companion register `{synth_name}` at \
                 0x{:x}..=0x{:x} overlaps the user-declared register \
                 `{user_name}` at 0x{:x}..=0x{:x}. Move the user register or \
                 change `intr-enable-address-base` / `intr-mask-address-base` / \
                 buffer `status-address` so they don't collide.",
                synth_range.0, synth_range.1, user_range.0, user_range.1,
            )));
        }
        ranges.push((start, end, name));
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::{Block, Device};
    use device_driver_common::specifiers::{AddressMode, ByteOrder, Integer};

    fn empty_block() -> Block {
        Block {
            description: String::new(),
            root: true,
            name: ident("root_block"),
            register_address_type: Integer::U8,
            command_address_type: Integer::U8,
            buffer_address_type: Integer::U8,
            register_address_mode: Some(AddressMode::Mapped),
            methods: Vec::new(),
            sv_bus: Vec::new(),
            sv_assertions: Default::default(),
            sv_ral: false,
            sv_hdl_path_prefix: None,
            sv_data_width: None,
            intr_aggregate: true,
            intr_enable_address_base: None,
            intr_mask_address_base: None,
        }
    }

    fn empty_driver(block: Block) -> Driver {
        Driver {
            devices: vec![Device {
                internal_address_type: Integer::U8,
                blocks: vec![block],
            }],
            field_sets: Vec::new(),
            enums: Vec::new(),
        }
    }

    fn mk_register(name: &str, addr: i128, fs_name: &str) -> BlockMethod {
        BlockMethod {
            description: String::new(),
            name: ident(name),
            address: addr,
            repeat: Repeat::None,
            method_type: BlockMethodType::Register {
                field_set_name: ident(fs_name),
                access: Access::RW,
                reset_value: None,
                reserved_behavior: Default::default(),
                external: false,
            },
        }
    }

    fn mk_field_set(name: &str, size_bytes: u32) -> FieldSet {
        FieldSet {
            description: String::new(),
            name: ident(name),
            byte_order: ByteOrder::LE,
            size_bytes,
            fields: Vec::new(),
        }
    }

    #[test]
    fn no_synthesis_no_check() {
        let block = empty_block();
        let driver = empty_driver(block);
        let trace = SynthesisTrace::default();
        assert!(check_companion_collisions(&driver, &trace).is_ok());
    }

    #[test]
    fn name_collision_rejected() {
        let mut block = empty_block();
        block.methods.push(mk_register(
            "root_intr_enable",
            0,
            "root_intr_enable_fields",
        ));
        // Simulate synthesis appending a register with the SAME name.
        block.methods.push(mk_register(
            "root_intr_enable",
            0x40,
            "root_intr_enable_fields_dup",
        ));
        let mut driver = empty_driver(block);
        driver
            .field_sets
            .push(mk_field_set("root_intr_enable_fields", 4));
        driver
            .field_sets
            .push(mk_field_set("root_intr_enable_fields_dup", 4));

        let trace = SynthesisTrace {
            synth_method_names: vec!["root_intr_enable".to_string()],
            synth_field_set_names: vec!["root_intr_enable_fields_dup".to_string()],
        };
        let err = check_companion_collisions(&driver, &trace).unwrap_err();
        assert!(
            err.to_string().contains("`root_intr_enable`"),
            "diagnostic should name the synthesized register: {err}"
        );
    }

    #[test]
    fn address_collision_rejected() {
        let mut block = empty_block();
        // User register at 0x40, 4 bytes — occupies 0x40..=0x43.
        block.methods.push(mk_register("ctrl", 0x40, "ctrl_fields"));
        // Synthesized register also at 0x40 — would overlap.
        block.methods.push(mk_register(
            "root_intr_enable",
            0x40,
            "root_intr_enable_fields",
        ));
        let mut driver = empty_driver(block);
        driver.field_sets.push(mk_field_set("ctrl_fields", 4));
        driver
            .field_sets
            .push(mk_field_set("root_intr_enable_fields", 4));

        let trace = SynthesisTrace {
            synth_method_names: vec!["root_intr_enable".to_string()],
            synth_field_set_names: vec!["root_intr_enable_fields".to_string()],
        };
        let err = check_companion_collisions(&driver, &trace).unwrap_err();
        let msg = err.to_string();
        assert!(
            msg.contains("overlaps"),
            "expected overlap diagnostic: {msg}"
        );
        assert!(
            msg.contains("root_intr_enable") && msg.contains("ctrl"),
            "diagnostic should name both registers: {msg}"
        );
    }

    #[test]
    fn user_only_overlap_not_flagged() {
        // Existing MIR pass `addresses_non_overlapping` is responsible for
        // user-vs-user overlaps; the LIR check should not duplicate that work.
        let mut block = empty_block();
        block.methods.push(mk_register("foo", 0, "foo_fields"));
        block.methods.push(mk_register("bar", 1, "foo_fields"));
        let mut driver = empty_driver(block);
        driver.field_sets.push(mk_field_set("foo_fields", 3));

        let trace = SynthesisTrace::default();
        assert!(
            check_companion_collisions(&driver, &trace).is_ok(),
            "user-vs-user overlap must not be flagged here"
        );
    }

    #[test]
    fn field_set_name_collision_rejected() {
        let mut block = empty_block();
        block.methods.push(mk_register("dummy", 0, "fs_a"));
        let mut driver = empty_driver(block);
        // User fieldset
        driver.field_sets.push(mk_field_set("fs_a", 4));
        // Synthesized fieldset with the same name
        driver.field_sets.push(mk_field_set("fs_a", 4));

        let trace = SynthesisTrace {
            synth_method_names: Vec::new(),
            synth_field_set_names: vec!["fs_a".to_string()],
        };
        let err = check_companion_collisions(&driver, &trace).unwrap_err();
        assert!(err.to_string().contains("fieldset"));
    }

    // ---- check_explicit_intr_bits -----------------------------------------

    use device_driver_common::specifiers::IntrTrigger;

    fn intr_field(
        name: &str,
        group: Option<&str>,
        enable_bit: Option<u32>,
        mask_bit: Option<u32>,
    ) -> Field {
        Field {
            description: String::new(),
            name: ident(name),
            address: AddressRange { start: 0, end: 0 },
            base_type: "u8".to_string(),
            conversion_method: FieldConversionMethod::Bool,
            access: Access::RW,
            on_write: OnWrite::Clear,
            on_read: OnRead::Store,
            hw_access: HwAccess::RO,
            hw_clr: false,
            hw_set: false,
            singlepulse: false,
            precedence: Precedence::Hw,
            intr_trigger: Some(IntrTrigger::Level),
            intr_group: group.map(str::to_string),
            intr_sticky: true,
            intr_enable: enable_bit.is_some(),
            intr_mask: mask_bit.is_some(),
            intr_enable_bit: enable_bit,
            intr_mask_bit: mask_bit,
            repeat: Repeat::None,
        }
    }

    #[test]
    fn explicit_enable_bit_collision_rejected() {
        let mut block = empty_block();
        block
            .methods
            .push(mk_register("status", 0, "status_fields"));
        let mut driver = empty_driver(block);
        let mut fs = mk_field_set("status_fields", 4);
        fs.fields.push(intr_field("overflow", None, Some(2), None));
        fs.fields.push(intr_field("underflow", None, Some(2), None));
        driver.field_sets.push(fs);

        let err = check_explicit_intr_bits(&driver).unwrap_err();
        let msg = err.to_string();
        assert!(msg.contains("intr-enable-bit: 2"), "msg = {msg}");
        assert!(
            msg.contains("overflow") && msg.contains("underflow"),
            "msg = {msg}"
        );
    }

    #[test]
    fn explicit_bits_distinct_groups_ok() {
        // Same bit number in two DIFFERENT groups is fine — they end up
        // in different companion registers.
        let mut block = empty_block();
        block
            .methods
            .push(mk_register("status", 0, "status_fields"));
        let mut driver = empty_driver(block);
        let mut fs = mk_field_set("status_fields", 4);
        fs.fields
            .push(intr_field("overflow", Some("a"), Some(2), None));
        fs.fields
            .push(intr_field("underflow", Some("b"), Some(2), None));
        driver.field_sets.push(fs);

        assert!(check_explicit_intr_bits(&driver).is_ok());
    }

    #[test]
    fn explicit_mask_bit_collision_rejected_distinct_from_enable() {
        // Two fields both pin `intr-mask-bit: 7` → reject. (Even though
        // their `intr-enable-bit` is unset / different.)
        let mut block = empty_block();
        block
            .methods
            .push(mk_register("status", 0, "status_fields"));
        let mut driver = empty_driver(block);
        let mut fs = mk_field_set("status_fields", 4);
        fs.fields.push(intr_field("a", None, None, Some(7)));
        fs.fields.push(intr_field("b", None, None, Some(7)));
        driver.field_sets.push(fs);

        let err = check_explicit_intr_bits(&driver).unwrap_err();
        assert!(err.to_string().contains("intr-mask-bit: 7"));
    }
}
