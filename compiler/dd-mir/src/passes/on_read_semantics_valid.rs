use std::collections::HashSet;

use crate::{
    model::{LendingIterator, Manifest, UniqueId},
    passes::{Assumption, Pass},
};
use device_driver_common::specifiers::{Access, OnRead};
use device_driver_diagnostics::{Diagnostics, DynError, errors::OnReadOnWriteOnly};

pub struct OnReadSemanticsValid;

impl Pass for OnReadSemanticsValid {
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

/// Reject `on-read: clear|set` modifier on WO fields — a read side-effect on a
/// non-readable field is contradictory.
pub fn run_pass(manifest: &mut Manifest, diagnostics: &mut Diagnostics) {
    let mut iter = manifest.iter_objects_with_config_mut();
    while let Some((object, _)) = iter.next() {
        let Some(field_set) = object.as_field_set_mut() else {
            continue;
        };
        let field_set_span = field_set.name.span;

        for field in field_set.fields.iter_mut() {
            let Some(on_read) = field.on_read else {
                continue;
            };
            if matches!(on_read, OnRead::Store) {
                continue;
            }
            if field.access == Access::WO {
                diagnostics.add(OnReadOnWriteOnly {
                    field_name: field.name.span,
                    on_read_value: on_read,
                    field_set_context: field_set_span,
                });
                // Repair: drop the modifier so later passes don't trip over it.
                field.on_read = None;
            }
        }
    }
}
