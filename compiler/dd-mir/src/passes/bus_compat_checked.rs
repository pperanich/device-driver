use crate::{
    model::{LendingIterator, Manifest, Object, UniqueId},
    passes::{Assumption, Pass},
};
use device_driver_common::specifiers::SvBus;
use device_driver_diagnostics::{
    Diagnostics, DynError,
    errors::{DuplicateProperty, IncompatibleSvBusWidth, InvalidSvDataWidth, SvBusCompatLevel},
};
use std::collections::HashSet;

/// Pass-trait wrapper around the SV-target compat checks.
pub struct BusCompatChecked;

impl Pass for BusCompatChecked {
    const ASSUMPTIONS_MADE: &[Assumption] = &[];
    const ASSUMPTIONS_RELEASED: &[Assumption] = &[];

    fn run_pass(
        manifest: &mut Manifest,
        diagnostics: &mut Diagnostics,
    ) -> Result<HashSet<UniqueId>, DynError> {
        run_pass(manifest, diagnostics);
        Ok(HashSet::new())
    }
}

/// Validate the SV-target bus + CPUIF configuration on every device.
///
/// `sv-bus:` and `sv-data-width:` are device-level properties (their
/// setters write to `Device.device_config`, not to `Manifest.config`),
/// so this pass walks each `Object::Device` in the manifest and
/// validates the local config.
///
/// Catches:
/// * Duplicate `sv-bus:` entries (e.g. `sv-bus: apb3, sv-bus: apb3`) on
///   the same device — the parser accepts the list verbatim; this pass
///   collapses dups and surfaces a diagnostic on each repeat so the user
///   sees an error instead of two identical wrapper files appearing in
///   the output.
/// * `sv-data-width:` values outside `{32, 64}` — the SV target only
///   models those two CPUIF widths today.
/// * `sv-bus:` entries whose protocol contradicts the configured
///   `sv-data-width:` (e.g. AHB-Lite v1.1 has no 64-bit data variant in
///   our wrapper, so `sv-bus: ahblite` + `sv-data-width: 64` is an
///   error; `sv-bus: native` + `sv-data-width: 64` is legal but rare so
///   it is emitted as a warning, not an error).
pub fn run_pass(manifest: &mut Manifest, diagnostics: &mut Diagnostics) {
    let mut iter = manifest.iter_objects_with_config_mut();
    while let Some((object, _)) = iter.next() {
        let Object::Device(device) = object else {
            continue;
        };
        dedupe_sv_bus(&mut device.device_config.sv_bus, diagnostics);
        let Some(data_width) = device.device_config.sv_data_width else {
            continue;
        };
        if data_width.value != 32 && data_width.value != 64 {
            diagnostics.add(InvalidSvDataWidth {
                span: data_width.span,
                value: data_width.value,
            });
            // Don't run cross-checks against an invalid width — the
            // cascade would only confuse the user. The single root-cause
            // diagnostic is enough.
            continue;
        }
        for entry in &device.device_config.sv_bus {
            if let Some((reason, level)) = bus_width_compat(entry.value, data_width.value) {
                diagnostics.add(IncompatibleSvBusWidth {
                    bus_span: entry.span,
                    width_span: data_width.span,
                    bus_name: entry.value.suffix(),
                    width: data_width.value,
                    reason: reason.into(),
                    level,
                });
            }
        }
    }
}

fn dedupe_sv_bus(
    bus_list: &mut Vec<device_driver_common::span::Spanned<SvBus>>,
    diagnostics: &mut Diagnostics,
) {
    let taken = std::mem::take(bus_list);
    let mut seen: HashSet<SvBus> = HashSet::new();
    let mut first_span_of: std::collections::HashMap<SvBus, _> = std::collections::HashMap::new();
    for entry in taken {
        if seen.insert(entry.value) {
            first_span_of.insert(entry.value, entry.span);
            bus_list.push(entry);
        } else if let Some(&first) = first_span_of.get(&entry.value) {
            diagnostics.add(DuplicateProperty {
                original: first,
                duplicate: entry.span,
            });
        }
    }
}

/// Returns `(reason, level)` if the (bus, width) pair is incompatible or
/// strongly unusual. `None` ⇒ the pair is fine.
fn bus_width_compat(bus: SvBus, width: u32) -> Option<(&'static str, SvBusCompatLevel)> {
    match (bus, width) {
        // AHB-Lite v1.1's emitted wrapper only models a 32-bit `HRDATA`
        // / `HWDATA` data path. The protocol itself allows 64-bit, but
        // we don't synthesize the wider data-bus parameterization yet.
        (SvBus::AhbLite, 64) => Some((
            "AHB-Lite v1.1 wrapper is fixed at 32-bit `HRDATA`/`HWDATA`. \
             Re-run with `sv-data-width: 32` or drop the AHB-Lite wrapper.",
            SvBusCompatLevel::Error,
        )),
        // Native is the internal CPUIF — fully parameterized — but few
        // downstream consumers wire up a 64-bit interconnect. Flag it as
        // a warning so the user sees the choice was deliberate.
        (SvBus::Native, 64) => Some((
            "the native CPUIF supports a 64-bit data path, but few \
             downstream interconnects do. Confirm this is intended.",
            SvBusCompatLevel::Warning,
        )),
        _ => None,
    }
}
