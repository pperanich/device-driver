use device_driver_common::{
    identifier::{All, Identifier, Operation, Type},
    span::Spanned,
    specifiers::{
        Access, AddressMode, AddressRange, ByteOrder, HwAccess, HwHandshake, HwKind, Integer,
        IntrTrigger, OnRead, OnWrite, Precedence, ReservedBehavior, SvBus,
    },
};

pub struct Driver {
    pub devices: Vec<Device>,
    pub field_sets: Vec<FieldSet>,
    pub enums: Vec<Enum>,
}

pub struct Device {
    pub internal_address_type: Integer,
    pub blocks: Vec<Block>,
}

pub struct Block {
    pub description: String,
    /// True for the root (top-level) block
    pub root: bool,
    pub name: Identifier<Type>,
    pub register_address_type: Integer,
    pub command_address_type: Integer,
    pub buffer_address_type: Integer,
    pub register_address_mode: Option<AddressMode>,
    pub methods: Vec<BlockMethod>,
    /// SV bus adapter wrappers to emit alongside this block's `<dev>_regs`
    /// module. Empty Vec means no wrappers (bare regs module only).
    pub sv_bus: Vec<SvBus>,
    /// SVA assertion categories to include in the `<dev>_sva` checker. When
    /// `SvAssertOpts::any()` is false, no checker file is emitted at all.
    pub sv_assertions: SvAssertOpts,
    /// When true, emit a `<dev>_ral_pkg.sv` UVM register model.
    pub sv_ral: bool,
    /// HDL path prefix for backdoor wiring in the RAL package.
    pub sv_hdl_path_prefix: Option<String>,
    /// CPUIF data bus width override (32 or 64). `None` ⇒ codegen falls
    /// back to the CLI option or 32.
    pub sv_data_width: Option<u32>,
    /// `false` suppresses the root `irq` OR-reduce output port.
    /// `true` (default) emits it when ungrouped intr fields exist.
    pub intr_aggregate: bool,
    /// Base addresses for auto-synthesized per-group enable/mask
    /// companion registers. `None` on either side disables synthesis for
    /// that side even if fields opt in.
    pub intr_enable_address_base: Option<i128>,
    pub intr_mask_address_base: Option<i128>,
}

/// Mirror of `device_driver_mir::model::SvAssertOpts`. Lives here so the
/// codegen layer (which only depends on LIR) can read it without pulling
/// MIR into scope.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Hash)]
pub struct SvAssertOpts {
    pub reset: bool,
    pub decode_mutex: bool,
    pub w1c: bool,
    pub ro_invariance: bool,
}

impl SvAssertOpts {
    #[must_use]
    pub fn any(&self) -> bool {
        self.reset || self.decode_mutex || self.w1c || self.ro_invariance
    }
}

pub struct BlockMethod {
    pub description: String,
    pub name: Identifier<Operation>,
    pub address: i128,
    pub repeat: Repeat,
    pub method_type: BlockMethodType,
}

pub enum Repeat {
    None,
    Count {
        count: u32,
        stride: i128,
    },
    Enum {
        enum_name: Identifier<Type>,
        enum_variants: Vec<Identifier<All>>,
        stride: i128,
    },
}

pub enum BlockMethodType {
    Block {
        name: Identifier<Type>,
    },
    Register {
        field_set_name: Identifier<Type>,
        access: Access,
        reset_value: Option<Spanned<Vec<u8>>>,
        /// Reserved-bit policy (default `RoZero`).
        reserved_behavior: ReservedBehavior,
        /// `true` ⇒ no internal storage flop; user RTL drives `<reg>_ext_rd_data`.
        external: bool,
    },
    Command {
        field_set_name_in: Option<Identifier<Type>>,
        field_set_name_out: Option<Identifier<Type>>,
        /// HW-side handshake mode. `None` skips SV emission for this
        /// command (current default for backward compat). `Some(Strobe)`
        /// opts the command into M6a strobe codegen.
        hw_handshake: Option<HwHandshake>,
    },
    Buffer {
        access: Access,
        /// HW mapping. `None` keeps the buffer SV-skipped.
        hw_kind: Option<HwKind>,
        /// Capacity hint surfaced in generated SV doc comments.
        depth: Option<u32>,
        /// Future-companion-status-register address; doc-comment only in v1.
        status_address: Option<i128>,
        /// Multi-word streaming hint; doc-comment only in v1.
        words: Option<u32>,
    },
}

/// A set of fields, like a register or command in/out
pub struct FieldSet {
    pub description: String,
    pub name: Identifier<Type>,
    pub byte_order: ByteOrder,
    pub size_bytes: u32,
    pub fields: Vec<Field>,
}

pub struct Field {
    pub description: String,
    pub name: Identifier<All>,
    pub address: AddressRange,
    pub base_type: String,
    pub conversion_method: FieldConversionMethod,
    pub access: Access,
    pub on_write: OnWrite,
    pub on_read: OnRead,
    pub hw_access: HwAccess,
    pub hw_clr: bool,
    pub hw_set: bool,
    pub singlepulse: bool,
    pub precedence: Precedence,
    pub intr_trigger: Option<IntrTrigger>,
    pub intr_group: Option<String>,
    pub intr_sticky: bool,
    pub intr_enable: bool,
    pub intr_mask: bool,
    /// Explicit bit position in the synthesized
    /// `<group>_intr_enable` companion register. Set by the user via
    /// `intr-enable-bit: N`. When `None`, the synthesis pass falls back
    /// to declaration order (current default — ABI-fragile but
    /// convenient).
    pub intr_enable_bit: Option<u32>,
    /// Same as `intr_enable_bit` for the `<group>_intr_mask` companion.
    pub intr_mask_bit: Option<u32>,
    pub repeat: Repeat,
}

impl Field {
    pub fn address_text(&self) -> String {
        if self.address.len() <= 1 {
            format!("bit {}", self.address.start)
        } else {
            format!("{}:{}", self.address.end, self.address.start)
        }
    }

    #[must_use]
    pub fn is_interrupt(&self) -> bool {
        self.intr_trigger.is_some()
    }
}

pub enum FieldConversionMethod {
    None,
    Into(Identifier<Type>),
    UnsafeInto(Identifier<Type>),
    TryInto(Identifier<Type>),
    Bool,
}

impl FieldConversionMethod {
    pub fn conversion_type(&self) -> Option<&Identifier<Type>> {
        match self {
            FieldConversionMethod::None => None,
            FieldConversionMethod::Into(type_path) => Some(type_path),
            FieldConversionMethod::UnsafeInto(type_path) => Some(type_path),
            FieldConversionMethod::TryInto(type_path) => Some(type_path),
            FieldConversionMethod::Bool => None,
        }
    }
}

pub struct Enum {
    pub description: String,
    pub name: Identifier<Type>,
    pub base_type: String,
    pub variants: Vec<EnumVariant>,
}

impl Enum {
    pub fn default_variant(&self) -> Option<&EnumVariant> {
        self.variants.iter().find(|v| v.default)
    }

    pub fn catch_all_variant(&self) -> Option<&EnumVariant> {
        self.variants.iter().find(|v| v.catch_all)
    }
}

pub struct EnumVariant {
    pub description: String,
    pub name: Identifier<All>,
    pub discriminant: i128,
    pub default: bool,
    pub catch_all: bool,
}
