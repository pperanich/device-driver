use std::collections::HashSet;

use crate::{
    model::{LendingIterator, Manifest, UniqueId},
    passes::{Assumption, Pass},
};
use device_driver_common::specifiers::{Access, OnWrite};
use device_driver_diagnostics::{Diagnostics, DynError, errors::OnWriteOnReadOnly};

pub struct OnWriteSemanticsValid;

impl Pass for OnWriteSemanticsValid {
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

/// Reject `on-write: clear|set|toggle` modifier on RO fields — a write
/// side-effect on a non-writable field is contradictory and would silently
/// be dead code in the SV target.
pub fn run_pass(manifest: &mut Manifest, diagnostics: &mut Diagnostics) {
    let mut iter = manifest.iter_objects_with_config_mut();
    while let Some((object, _)) = iter.next() {
        let Some(field_set) = object.as_field_set_mut() else {
            continue;
        };
        let field_set_span = field_set.name.span;

        for field in field_set.fields.iter_mut() {
            let Some(on_write) = field.on_write else {
                continue;
            };
            if matches!(on_write, OnWrite::Store) {
                continue;
            }
            if field.access == Access::RO {
                diagnostics.add(OnWriteOnReadOnly {
                    field_name: field.name.span,
                    on_write_value: on_write,
                    field_set_context: field_set_span,
                });
                // Repair: drop the modifier so later passes don't trip over it.
                field.on_write = None;
            }
        }
    }
}
