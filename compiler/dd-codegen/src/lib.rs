use clap::{Parser, Subcommand};
use convert_case::{Case, Casing};
use device_driver_common::specifiers::SvBus;
use device_driver_lir::model::Driver;
use itertools::Itertools;

pub use crate::rust::RustCodegenOptions;

mod rust;
mod systemverilog;

#[derive(Debug, Clone, Subcommand)]
pub enum Target {
    /// Generate Rust code
    Rust(RustCodegenOptions),
    /// Generate SystemVerilog code (regblock + per-bus wrappers + SVA + UVM RAL)
    #[command(name = "systemverilog", alias = "sv")]
    SystemVerilog(SvCodegenOptions),
}

/// Knobs the SystemVerilog target reads at codegen time. These do not
/// affect the Rust target; they're only meaningful when invoking the
/// `systemverilog` subcommand.
#[derive(Parser, Debug, Clone, Default)]
#[command(no_binary_name = true)]
pub struct SvCodegenOptions {
    /// Override the CPUIF data-bus width in bits. Accepted values: 32, 64.
    /// When omitted, the DSL `sv-data-width:` device property wins; if
    /// also unset, defaults to 32.
    #[arg(long = "sv-data-width", value_name = "BITS", require_equals = true)]
    pub data_width: Option<u32>,
    /// Override the CPUIF address-bus width in bits. When omitted, the
    /// width is derived from the DSL `register-address-type:` property.
    #[arg(long = "sv-addr-width", value_name = "BITS", require_equals = true)]
    pub addr_width: Option<u32>,
}

impl Target {
    pub fn create_error_message(&self) -> &'static str {
        match self {
            Target::Rust(_) => {
                "compile_error!(\"The device driver input has errors that need to be solved!\");"
            }
            Target::SystemVerilog(_) => {
                "// ERROR: The device driver input has errors that need to be solved!"
            }
        }
    }

    /// Converts the multiline text to comments that work for the target
    pub fn to_comments(&self, text: &str) -> String {
        match self {
            Target::Rust(_) => text.lines().map(|line| format!("// {line}")).join("\n"),
            Target::SystemVerilog(_) => text.lines().map(|line| format!("// {line}")).join("\n"),
        }
    }

    pub fn should_format_as_rust(&self) -> bool {
        matches!(self, Target::Rust(_))
    }
}

pub fn codegen(target: &Target, lir_driver: &Driver, source: &str) -> String {
    match target {
        Target::Rust(codegen_options) => {
            rust::DriverTemplateRust::new(lir_driver, source, codegen_options).to_string()
        }
        Target::SystemVerilog(_) => {
            // Back-compat: concatenate all per-file outputs separated by a
            // blank line. Snapshot tests compare against this stream and
            // stdout output (no `-o`) sends one combined dump. Directory-style
            // output uses `codegen_files` directly so the separator is moot.
            let mut out = String::new();
            for (_, content) in codegen_files(target, lir_driver, source) {
                out.push_str(&content);
                if !out.ends_with('\n') {
                    out.push('\n');
                }
                out.push('\n');
            }
            out
        }
    }
}

/// Multi-file codegen. Returns one `(filename, content)` per artifact the
/// target produces. The Rust target always returns exactly one entry. The SV
/// target returns the package + regs module, plus one wrapper per requested
/// `sv-bus:` adapter, plus SVA / UVM RAL files when opted in.
pub fn codegen_files(target: &Target, lir_driver: &Driver, source: &str) -> Vec<(String, String)> {
    match target {
        Target::Rust(codegen_options) => {
            let device_name = lir_driver
                .devices
                .first()
                .and_then(|d| d.blocks.iter().find(|b| b.root).map(|b| b.name.original()))
                .unwrap_or("device");
            let filename = format!("{}.rs", device_name.to_case(Case::Snake));
            let content =
                rust::DriverTemplateRust::new(lir_driver, source, codegen_options).to_string();
            vec![(filename, content)]
        }
        Target::SystemVerilog(sv_options) => {
            let mut files = Vec::new();
            let combined =
                systemverilog::DeviceTemplateSv::new(lir_driver, source, sv_options).to_string();
            let dev_name = lir_driver
                .devices
                .first()
                .and_then(|d| d.blocks.iter().find(|b| b.root).map(|b| b.name.original()))
                .unwrap_or("device")
                .to_case(Case::Snake);
            if let Some(end_idx) = combined.find("endpackage") {
                let line_end = combined[end_idx..]
                    .find('\n')
                    .map_or(combined.len(), |off| end_idx + off + 1);
                let (pkg_part, mod_part) = combined.split_at(line_end);
                files.push((format!("{dev_name}_pkg.sv"), pkg_part.to_string()));
                let mod_trimmed = mod_part.trim_start_matches('\n');
                files.push((format!("{dev_name}_regs.sv"), mod_trimmed.to_string()));
            } else {
                files.push((format!("{dev_name}.sv"), combined));
            }

            let buses: Vec<SvBus> = lir_driver
                .devices
                .iter()
                .flat_map(|d| d.blocks.iter().filter(|b| b.root))
                .flat_map(|b| b.sv_bus.iter().copied())
                .collect();
            for bus in buses {
                let filename = format!("{dev_name}_{}.sv", bus.suffix());
                let content =
                    systemverilog::render_bus_wrapper(lir_driver, bus, &dev_name, sv_options);
                files.push((filename, content));
            }
            if let Some((sva, bind)) = systemverilog::render_sva_files(lir_driver) {
                files.push((format!("{dev_name}_sva.sv"), sva));
                files.push((format!("{dev_name}_sva_bind.svh"), bind));
            }
            if let Some((ral, smoke)) = systemverilog::render_ral_files(lir_driver, sv_options) {
                files.push((format!("{dev_name}_ral_pkg.sv"), ral));
                files.push((format!("{dev_name}_ral_smoke_seq.sv"), smoke));
            }
            files
        }
    }
}
