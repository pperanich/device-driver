use device_driver_diagnostics::{DynError, ResultExt};

mod lowering;
pub mod model;
mod synthesis;

pub fn lower_mir(manifest: device_driver_mir::model::Manifest) -> Result<model::Driver, DynError> {
    let enums = lowering::transform_enums(&manifest);
    let field_sets = lowering::transform_field_sets(&manifest)
        .with_message(|| "could not transform fieldsets")?;
    let devices =
        lowering::transform_devices(&manifest).with_message(|| "could not transform devices")?;

    let mut driver = model::Driver {
        devices,
        field_sets,
        enums,
    };
    // Reject explicit-bit conflicts BEFORE synthesis so the user sees
    // a single root-cause diagnostic instead of a downstream
    // collision. If two fields both claim bit 5, the synthesis pass
    // would silently overwrite one in the BTreeMap of declaration
    // order; the collision check catches it head-on.
    synthesis::check_explicit_intr_bits(&driver)
        .with_message(|| "duplicate explicit intr companion bit positions")?;
    let trace = synthesis::synthesize_companion_registers(&mut driver);
    synthesis::check_companion_collisions(&driver, &trace)
        .with_message(|| "synthesized companion registers collide with user-declared ones")?;
    Ok(driver)
}
