use std::collections::HashSet;

use crate::{
    model::{LendingIterator, Manifest, Object, UniqueId},
    passes::{Assumption, Pass},
};
use device_driver_diagnostics::{Diagnostics, DynError, errors::MissingIntrCompanionBase};

/// Reject `intr-enable: allow` / `intr-mask: allow` opt-ins on fields
/// whose owning device does not set the matching `intr-enable-
/// address-base` / `intr-mask-address-base` property. Without the
/// base, the LIR synthesis pass silently skips emission of the
/// companion register and the IRQ formula loses its gating term. The
/// user sees a configurable IRQ output that ignores the configuration
/// — a near-impossible bug to diagnose from generated SV alone.
///
/// Surfacing the failure at MIR time turns the silent footgun into a
/// loud, source-localized error.
pub struct IntrCompanionsAddressable;

impl Pass for IntrCompanionsAddressable {
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

fn run_pass(manifest: &mut Manifest, diagnostics: &mut Diagnostics) {
    // Walk every device + its child registers' fields. For each field
    // that opted into intr-enable / intr-mask, confirm the owning
    // device has the matching address base.
    let mut iter = manifest.iter_objects_with_config_mut();
    while let Some((object, _)) = iter.next() {
        let Object::Device(device) = object else {
            continue;
        };
        let enable_base = device.device_config.intr_enable_address_base;
        let mask_base = device.device_config.intr_mask_address_base;
        if enable_base.is_some() && mask_base.is_some() {
            // Both bases present — every opt-in resolves; skip walk.
            continue;
        }
        for child in collect_field_sets(device) {
            for field in child {
                if field.intr_enable && enable_base.is_none() {
                    diagnostics.add(MissingIntrCompanionBase {
                        field_span: field.name.span,
                        side: "intr-enable",
                        address_base_property: "intr-enable-address-base",
                    });
                }
                if field.intr_mask && mask_base.is_none() {
                    diagnostics.add(MissingIntrCompanionBase {
                        field_span: field.name.span,
                        side: "intr-mask",
                        address_base_property: "intr-mask-address-base",
                    });
                }
            }
        }
    }
}

/// Recurse into the device's children and yield every field set's
/// fields. Used immutably so we can borrow `device.device_config`
/// alongside.
fn collect_field_sets(device: &crate::model::Device) -> Vec<&Vec<crate::model::Field>> {
    let mut out = Vec::new();
    walk(&device.objects, &mut out);
    out
}

fn walk<'a>(objects: &'a [Object], out: &mut Vec<&'a Vec<crate::model::Field>>) {
    for obj in objects {
        if let Some(fs) = obj.as_field_set() {
            out.push(&fs.fields);
        }
        let children = obj.child_objects();
        if !children.is_empty() {
            walk(children, out);
        }
    }
}
