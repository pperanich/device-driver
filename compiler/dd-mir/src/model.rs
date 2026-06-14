use std::{fmt::Display, rc::Rc};

use convert_case::Boundary;
#[cfg(test)]
use device_driver_common::identifier::IdentifierType;
use device_driver_common::{
    identifier::{All, Identifier, IdentifierRef, Operation, RuntimeType, Type},
    span::{Span, SpanExt, Spanned},
    specifiers::{
        Access, AddressMode, AddressRange, BaseType, ByteOrder, HwAccess, HwHandshake, HwKind,
        Integer, IntrTrigger, NodeType, OnRead, OnWrite, Precedence, Repeat, ReservedBehavior,
        ResetValue, SvBus, TypeConversion,
    },
};

#[derive(Debug, Clone, Default, PartialEq)]
pub struct Manifest {
    pub description: String,
    pub name: Spanned<Identifier<All>>,
    pub config: DeviceConfig,
    pub objects: Vec<Object>,
    pub span: Span,
}

impl Manifest {
    pub fn iter_objects_with_config_mut(&mut self) -> ObjectIterMut<'_> {
        ObjectIterMut {
            children: &mut self.objects,
            parent: None,
            collection_object_returned: false,
            current_device_config: Rc::new(self.config.clone()),
        }
    }

    pub fn iter_objects(&self) -> impl Iterator<Item = &Object> {
        ObjectIter {
            children: &self.objects,
            parent: None,
            collection_object_returned: false,
            current_device_config: Rc::new(self.config.clone()),
        }
        .map(|(object, _)| object)
    }

    #[must_use]
    pub fn iter_objects_with_config(&self) -> ObjectIter<'_> {
        ObjectIter {
            children: &self.objects,
            parent: None,
            collection_object_returned: false,
            current_device_config: Rc::new(self.config.clone()),
        }
    }

    pub fn iter_enums(&self) -> impl Iterator<Item = &'_ Enum> {
        self.iter_objects_with_config().filter_map(|(o, _)| {
            if let Object::Enum(e) = o {
                Some(e)
            } else {
                None
            }
        })
    }

    pub fn iter_enums_with_config(&self) -> impl Iterator<Item = (&'_ Enum, Rc<DeviceConfig>)> {
        self.iter_objects_with_config().filter_map(|(o, config)| {
            if let Object::Enum(e) = o {
                Some((e, config))
            } else {
                None
            }
        })
    }

    pub fn iter_devices_with_config(&self) -> impl Iterator<Item = (&'_ Device, Rc<DeviceConfig>)> {
        self.iter_objects_with_config().filter_map(|(o, config)| {
            if let Object::Device(d) = o {
                Some((d, config))
            } else {
                None
            }
        })
    }
}

#[derive(Default)]
pub struct ObjectIterMut<'a> {
    children: &'a mut [Object],
    parent: Option<Box<ObjectIterMut<'a>>>,
    collection_object_returned: bool,
    current_device_config: Rc<DeviceConfig>,
}

/// A GAT based lending iterator.
/// Can't do anything fancy with it yet though.
pub trait LendingIterator {
    type Item<'a>
    where
        Self: 'a;

    fn next(&mut self) -> Option<Self::Item<'_>>;
}

impl LendingIterator for ObjectIterMut<'_> {
    type Item<'b>
        = (&'b mut Object, Rc<DeviceConfig>)
    where
        Self: 'b;

    fn next(&mut self) -> Option<Self::Item<'_>> {
        if self.children.is_empty() {
            match self.parent.take() {
                Some(parent) => {
                    // continue with the parent node
                    *self = *parent;
                    self.next()
                }
                None => None,
            }
        } else if self.children[0].child_objects_mut().is_empty() {
            let (first, rest) = std::mem::take(&mut self.children)
                .split_first_mut()
                .expect("Already checked not empty");
            self.children = rest;
            Some((first, self.current_device_config.clone()))
        } else if !self.collection_object_returned {
            self.collection_object_returned = true;

            let next_device_config = if let Some(new_config) = self.children[0].device_config() {
                Rc::new(self.current_device_config.override_with(new_config))
            } else {
                self.current_device_config.clone()
            };

            Some((&mut self.children[0], next_device_config))
        } else {
            self.collection_object_returned = false;

            let next_device_config = if let Some(new_config) = self.children[0].device_config() {
                Rc::new(self.current_device_config.override_with(new_config))
            } else {
                self.current_device_config.clone()
            };

            let (first, rest) = std::mem::take(&mut self.children)
                .split_first_mut()
                .expect("Already checked not empty");
            self.children = rest;

            *self = ObjectIterMut {
                children: first.child_objects_mut(),
                parent: Some(Box::new(std::mem::take(self))),
                collection_object_returned: false,
                current_device_config: next_device_config,
            };
            self.next()
        }
    }
}

#[derive(Default)]
pub struct ObjectIter<'a> {
    children: &'a [Object],
    parent: Option<Box<ObjectIter<'a>>>,
    collection_object_returned: bool,
    current_device_config: Rc<DeviceConfig>,
}

impl<'a> Iterator for ObjectIter<'a> {
    type Item = (&'a Object, Rc<DeviceConfig>);

    fn next(&mut self) -> Option<Self::Item> {
        let children = std::mem::take(&mut self.children);

        match children.split_first() {
            None => match self.parent.take() {
                Some(parent) => {
                    // continue with the parent node
                    *self = *parent;
                    self.next()
                }
                None => None,
            },
            Some((first, rest)) => {
                self.children = rest;

                if first.child_objects().is_empty() {
                    Some((first, self.current_device_config.clone()))
                } else if !self.collection_object_returned {
                    self.collection_object_returned = true;

                    let next_device_config = if let Some(new_config) = first.device_config() {
                        Rc::new(self.current_device_config.override_with(new_config))
                    } else {
                        self.current_device_config.clone()
                    };

                    self.children = children;

                    Some((&children[0], next_device_config))
                } else {
                    self.collection_object_returned = false;

                    let next_device_config = if let Some(new_config) = first.device_config() {
                        Rc::new(self.current_device_config.override_with(new_config))
                    } else {
                        self.current_device_config.clone()
                    };

                    *self = ObjectIter {
                        children: first.child_objects(),
                        parent: Some(Box::new(std::mem::take(self))),
                        collection_object_returned: false,
                        current_device_config: next_device_config,
                    };
                    self.next()
                }
            }
        }
    }
}

/// Implementation meant for testing to easily create a manifest with just one device
impl From<Device> for Manifest {
    fn from(value: Device) -> Self {
        Self {
            description: String::new(),
            name: value
                .name
                .value
                .clone()
                .cast_unchecked()
                .with_span(value.name.span),
            span: value.span,
            objects: vec![Object::Device(value)],
            config: DeviceConfig::default(),
        }
    }
}

#[derive(Debug, Clone, Default, PartialEq)]
pub struct Device {
    pub description: String,
    pub name: Spanned<Identifier<Type>>,
    pub device_config: DeviceConfig,
    pub objects: Vec<Object>,
    /// Span of the whole object
    pub span: Span,
}

impl Device {
    pub fn iter_objects(&self) -> impl Iterator<Item = &Object> {
        ObjectIter {
            children: &self.objects,
            parent: None,
            collection_object_returned: false,
            // Note: We can't give the config from here because there might be a config in the manifest we don't know about
            current_device_config: Rc::new(DeviceConfig::default()),
        }
        .map(|(object, _)| object)
    }
}
#[derive(Debug, Clone, PartialEq)]
pub struct DeviceConfig {
    /// The id of the device that owns this config. If None, then this is a manifest config
    pub owner: Option<UniqueId>,
    pub byte_order: Option<ByteOrder>,
    pub register_address_type: Option<Spanned<Integer>>,
    pub command_address_type: Option<Spanned<Integer>>,
    pub buffer_address_type: Option<Spanned<Integer>>,
    pub name_word_boundaries: Option<Vec<Boundary>>,
    pub register_address_mode: Option<Spanned<AddressMode>>,
    /// SystemVerilog bus adapters the user wants emitted alongside the regblock.
    /// Empty Vec means no wrappers are produced (the bare `_regs` module ships
    /// alone). Each entry produces a `<dev>_<bus>.sv` file.
    pub sv_bus: Vec<Spanned<SvBus>>,
    /// SystemVerilog Assertion (SVA) categories the user wants emitted in
    /// `<dev>_sva.sv`. If all flags are off, no checker module / bind snippet
    /// is produced.
    pub sv_assertions: SvAssertOpts,
    /// When `true`, emit `<dev>_ral_pkg.sv` containing a UVM register model
    /// (`uvm_reg_field` / `uvm_reg` / `uvm_reg_block` subclasses) for
    /// verification-team consumption. Defaults to `true` (item 120 from
    /// the SV-backend roadmap — RAL is normally always wanted by
    /// verification teams; opt out via `sv-no-ral: allow` only if
    /// snapshot surface area matters more than RAL availability).
    /// `sv-ral: allow` is kept as a no-op for back-compat with DSL
    /// sources that pin the value explicitly.
    pub sv_ral: bool,
    /// HDL path prefix wired into the RAL package via
    /// `add_hdl_path_slice`. e.g. `"u_chip.u_regs"`. None ⇒ paths
    /// reference the regs module directly with no prefix.
    pub sv_hdl_path_prefix: Option<String>,
    /// CPUIF data bus width in bits. Default 32 when unset. Picked up by
    /// the SV target's `data_width()` helper; overrides the CLI option of
    /// the same name when both are provided. Spanned so
    /// `bus_compat_checked` can point a diagnostic at the source if the
    /// value or its combination with `sv-bus:` is unsupported.
    pub sv_data_width: Option<Spanned<u32>>,
    /// When `false`, the regblock suppresses the root `irq` OR-reduce
    /// output port even if ungrouped intr fields exist. Per-group
    /// `irq_<group>` outputs are unaffected. Default `true`.
    pub intr_aggregate: Option<bool>,
    /// Base address for auto-synthesized per-group enable companion
    /// registers. Each distinct group with at least one
    /// `intr-enable: allow` field occupies one CPUIF-data-width slot,
    /// starting at this base. None ⇒ no auto-synthesis (an error if any
    /// field has `intr-enable` set).
    pub intr_enable_address_base: Option<i128>,
    /// Same idea for mask companion registers.
    pub intr_mask_address_base: Option<i128>,
}

/// Opt-in SVA assertion categories. Each maps to a class of
/// `assert property` constructs in the emitted `<dev>_sva` checker module
/// (see SYSTEMVERILOG_DECISIONS.md, Decision 8).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Hash)]
pub struct SvAssertOpts {
    /// Each storage register equals its reset value while `!rst_n`.
    pub reset: bool,
    /// At most one `rd_hit_*` / `wr_hit_*` asserts per cycle (decode mutex).
    pub decode_mutex: bool,
    /// Every `on-write: clear` (W1C) field clears exactly the bits the
    /// transaction set in `cpuif_wr_data`.
    pub w1c: bool,
    /// RO fields' storage never changes from a SW write.
    pub ro_invariance: bool,
}

impl SvAssertOpts {
    /// `true` iff any category is enabled — gates emission of the entire SVA
    /// file pair.
    #[must_use]
    pub fn any(&self) -> bool {
        self.reset || self.decode_mutex || self.w1c || self.ro_invariance
    }
}

impl Default for DeviceConfig {
    fn default() -> Self {
        Self {
            owner: None,
            byte_order: None,
            register_address_type: None,
            command_address_type: None,
            buffer_address_type: None,
            name_word_boundaries: None,
            register_address_mode: None,
            sv_bus: Vec::new(),
            sv_assertions: SvAssertOpts::default(),
            // Default `sv_ral` to true — `sv-no-ral: allow` opts out.
            sv_ral: true,
            sv_hdl_path_prefix: None,
            sv_data_width: None,
            intr_aggregate: None,
            intr_enable_address_base: None,
            intr_mask_address_base: None,
        }
    }
}

impl DeviceConfig {
    #[must_use]
    pub fn override_with(&self, other: &Self) -> DeviceConfig {
        Self {
            owner: other.owner.clone().or(self.owner.clone()),
            byte_order: other.byte_order.or(self.byte_order),
            register_address_type: other.register_address_type.or(self.register_address_type),
            command_address_type: other.command_address_type.or(self.command_address_type),
            buffer_address_type: other.buffer_address_type.or(self.buffer_address_type),
            name_word_boundaries: other
                .name_word_boundaries
                .as_ref()
                .or(self.name_word_boundaries.as_ref())
                .cloned(),
            register_address_mode: other.register_address_mode.or(self.register_address_mode),
            sv_bus: if other.sv_bus.is_empty() {
                self.sv_bus.clone()
            } else {
                other.sv_bus.clone()
            },
            sv_assertions: SvAssertOpts {
                reset: self.sv_assertions.reset || other.sv_assertions.reset,
                decode_mutex: self.sv_assertions.decode_mutex || other.sv_assertions.decode_mutex,
                w1c: self.sv_assertions.w1c || other.sv_assertions.w1c,
                ro_invariance: self.sv_assertions.ro_invariance
                    || other.sv_assertions.ro_invariance,
            },
            // RAL is opt-out: any layer that says "no" wins. Two
            // defaults-of-true AND together to true; a `sv-no-ral` at
            // either layer wins and suppresses the package.
            sv_ral: self.sv_ral && other.sv_ral,
            sv_hdl_path_prefix: other
                .sv_hdl_path_prefix
                .as_ref()
                .or(self.sv_hdl_path_prefix.as_ref())
                .cloned(),
            sv_data_width: other.sv_data_width.or(self.sv_data_width),
            intr_aggregate: other.intr_aggregate.or(self.intr_aggregate),
            intr_enable_address_base: other
                .intr_enable_address_base
                .or(self.intr_enable_address_base),
            intr_mask_address_base: other.intr_mask_address_base.or(self.intr_mask_address_base),
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum Object {
    Device(Device),
    Block(Block),
    Register(Register),
    Command(Command),
    Buffer(Buffer),
    FieldSet(FieldSet),
    Enum(Enum),
    Extern(Extern),
    Field(Field),
}

impl Object {
    pub(crate) fn device_config(&self) -> Option<&DeviceConfig> {
        match self {
            Object::Device(device) => Some(&device.device_config),
            _ => None,
        }
    }

    pub(crate) fn child_objects_mut(&mut self) -> &mut [Object] {
        match self {
            Object::Device(device) => &mut device.objects,
            Object::Block(block) => &mut block.objects,
            _ => &mut [],
        }
    }

    pub(crate) fn child_objects_vec(&mut self) -> Option<&mut Vec<Object>> {
        match self {
            Object::Device(device) => Some(&mut device.objects),
            Object::Block(block) => Some(&mut block.objects),
            _ => None,
        }
    }

    pub(crate) fn child_objects(&self) -> &[Object] {
        match self {
            Object::Device(device) => &device.objects,
            Object::Block(block) => &block.objects,
            _ => &[],
        }
    }

    /// Get a mutable reference to the name of the specific object
    pub(crate) fn name_mut(&mut self) -> &mut Identifier<RuntimeType> {
        match self {
            Object::Device(val) => val.name.as_runtime_type_mut(),
            Object::Block(val) => val.name.as_runtime_type_mut(),
            Object::Register(val) => val.name.as_runtime_type_mut(),
            Object::Command(val) => val.name.as_runtime_type_mut(),
            Object::Buffer(val) => val.name.as_runtime_type_mut(),
            Object::FieldSet(val) => val.name.as_runtime_type_mut(),
            Object::Enum(val) => val.name.as_runtime_type_mut(),
            Object::Extern(val) => val.name.as_runtime_type_mut(),
            Object::Field(val) => val.name.as_runtime_type_mut(),
        }
    }

    /// Get a reference to the name of the specific object
    pub fn name(&self) -> &Identifier<RuntimeType> {
        match self {
            Object::Device(val) => val.name.as_runtime_type(),
            Object::Block(val) => val.name.as_runtime_type(),
            Object::Register(val) => val.name.as_runtime_type(),
            Object::Command(val) => val.name.as_runtime_type(),
            Object::Buffer(val) => val.name.as_runtime_type(),
            Object::FieldSet(val) => val.name.as_runtime_type(),
            Object::Enum(val) => val.name.as_runtime_type(),
            Object::Extern(val) => val.name.as_runtime_type(),
            Object::Field(val) => val.name.as_runtime_type(),
        }
    }

    /// Get the span of the name of the object
    pub(crate) fn name_span(&self) -> Span {
        match self {
            Object::Device(val) => val.name.span,
            Object::Block(val) => val.name.span,
            Object::Register(val) => val.name.span,
            Object::Command(val) => val.name.span,
            Object::Buffer(val) => val.name.span,
            Object::FieldSet(val) => val.name.span,
            Object::Enum(val) => val.name.span,
            Object::Extern(val) => val.name.span,
            Object::Field(val) => val.name.span,
        }
    }

    /// Return the address if it is specified.
    pub(crate) fn address(&self) -> Option<Spanned<i128>> {
        match self {
            Object::Device(_) => None,
            Object::Block(block) => Some(block.address_offset),
            Object::Register(register) => Some(register.address),
            Object::Command(command) => Some(command.address),
            Object::Buffer(buffer) => Some(buffer.address),
            Object::FieldSet(_) => None,
            Object::Enum(_) => None,
            Object::Extern(_) => None,
            Object::Field(_) => None,
        }
    }

    /// Return the repeat value if it exists
    pub(crate) fn repeat(&self) -> Option<&Repeat> {
        match self {
            Object::Device(_) => None,
            Object::Block(block) => block.repeat.as_ref(),
            Object::Register(register) => register.repeat.as_ref(),
            Object::Command(command) => command.repeat.as_ref(),
            Object::Buffer(_) => None,
            Object::FieldSet(_) => None,
            Object::Enum(_) => None,
            Object::Extern(_) => None,
            Object::Field(field) => field.repeat.as_ref(),
        }
    }

    /// Return the repeat value if it exists
    pub(crate) fn repeat_mut(&mut self) -> Option<&mut Repeat> {
        match self {
            Object::Device(_) => None,
            Object::Block(block) => block.repeat.as_mut(),
            Object::Register(register) => register.repeat.as_mut(),
            Object::Command(command) => command.repeat.as_mut(),
            Object::Buffer(_) => None,
            Object::FieldSet(_) => None,
            Object::Enum(_) => None,
            Object::Extern(_) => None,
            Object::Field(field) => field.repeat.as_mut(),
        }
    }

    pub(crate) fn as_field_set(&self) -> Option<&FieldSet> {
        if let Self::FieldSet(v) = self {
            Some(v)
        } else {
            None
        }
    }

    pub(crate) fn as_field_set_mut(&mut self) -> Option<&mut FieldSet> {
        if let Self::FieldSet(v) = self {
            Some(v)
        } else {
            None
        }
    }

    pub fn as_enum(&self) -> Option<&Enum> {
        if let Self::Enum(v) = self {
            Some(v)
        } else {
            None
        }
    }

    pub(crate) fn allow_address_overlap(&self) -> bool {
        match self {
            Object::Device(_) => false,
            Object::Block(_) => false,
            Object::Register(register) => register.allow_address_overlap,
            Object::Command(command) => command.allow_address_overlap,
            Object::Buffer(_) => false,
            Object::FieldSet(_) => false,
            Object::Enum(_) => false,
            Object::Extern(_) => false,
            Object::Field(_) => false,
        }
    }

    /// The span of the entire object
    pub(crate) fn span(&self) -> Span {
        match self {
            Object::Device(val) => val.span,
            Object::Block(val) => val.span,
            Object::Register(val) => val.span,
            Object::Command(val) => val.span,
            Object::Buffer(val) => val.span,
            Object::FieldSet(val) => val.span,
            Object::Enum(val) => val.span,
            Object::Extern(val) => val.span,
            Object::Field(val) => val.span,
        }
    }

    pub(crate) fn node_type(&self) -> NodeType {
        match self {
            Object::Device(_) => NodeType::Device,
            Object::Block(_) => NodeType::Block,
            Object::Register(_) => NodeType::Register,
            Object::Command(_) => NodeType::Command,
            Object::Buffer(_) => NodeType::Buffer,
            Object::FieldSet(_) => NodeType::FieldSet,
            Object::Enum(_) => NodeType::Enum,
            Object::Extern(_) => NodeType::Extern,
            Object::Field(_) => NodeType::Field,
        }
    }

    /// Get the fieldset refs of the object. Only returns non-zero for registers and commands
    pub(crate) fn fieldset_refs(&self) -> Vec<Spanned<IdentifierRef<Type>>> {
        match self {
            Object::Device(_) => Vec::new(),
            Object::Block(_) => Vec::new(),
            Object::Register(r) => vec![r.field_set_ref.clone()],
            Object::Command(c) => [c.field_set_ref_in.clone(), c.field_set_ref_out.clone()]
                .into_iter()
                .flatten()
                .collect(),
            Object::Buffer(_) => Vec::new(),
            Object::FieldSet(_) => Vec::new(),
            Object::Enum(_) => Vec::new(),
            Object::Extern(_) => Vec::new(),
            Object::Field(_) => Vec::new(),
        }
    }
}

#[derive(Debug, Clone, Default, PartialEq)]
pub struct Block {
    pub description: String,
    pub name: Spanned<Identifier<All>>,
    pub address_offset: Spanned<i128>,
    pub repeat: Option<Repeat>,
    pub objects: Vec<Object>,
    /// Span of the whole object
    pub span: Span,
}

#[derive(Debug, Clone, Default, PartialEq)]
pub struct Register {
    pub description: String,
    pub name: Spanned<Identifier<Operation>>,
    pub access: Access,
    pub allow_address_overlap: bool,
    pub address: Spanned<i128>,
    pub reset_value: Option<Spanned<ResetValue>>,
    pub repeat: Option<Repeat>,
    pub field_set_ref: Spanned<IdentifierRef<Type>>,
    /// How bits not covered by any field behave under SW access. Default
    /// `RoZero` (read as 0, writes dropped).
    pub reserved_behavior: ReservedBehavior,
    /// When `true`, the register has no internal storage flop. The
    /// regblock instead exposes `<reg>_ext_wr_hit` / `_ext_rd_hit` /
    /// `_ext_wr_data` ports on `hwif_out` and reads
    /// `hwif_in.<reg>_ext_rd_data`. SW transactions are translated
    /// directly into user RTL handshake.
    pub external: bool,
    /// Span of the whole object
    pub span: Span,
}

#[derive(Debug, Clone, Default, PartialEq)]
pub struct FieldSet {
    pub description: String,
    pub name: Spanned<Identifier<Type>>,
    pub size_bytes: Spanned<u32>,
    pub byte_order: Option<ByteOrder>,
    pub allow_bit_overlap: bool,
    pub fields: Vec<Field>,
    /// Span of the whole object
    pub span: Span,
}

impl FieldSet {
    pub fn size_bits(&self) -> u32 {
        self.size_bytes.value * 8
    }
}

#[derive(Debug, Clone, Default, PartialEq)]
pub struct Field {
    pub description: String,
    pub name: Spanned<Identifier<All>>,
    pub access: Access,
    pub on_write: Option<OnWrite>,
    pub on_read: Option<OnRead>,
    pub hw_access: Option<HwAccess>,
    pub hw_clr: bool,
    pub hw_set: bool,
    pub singlepulse: bool,
    pub precedence: Option<Precedence>,
    pub intr_trigger: Option<IntrTrigger>,
    pub intr_group: Option<String>,
    pub intr_sticky: bool,
    /// When true, the field opts into a per-group `<group>_intr_enable`
    /// companion register; its bit gates whether this source's sticky
    /// storage contributes to `irq_<group>`.
    pub intr_enable: bool,
    /// When true, the field opts into a per-group `<group>_intr_mask`
    /// companion register; setting the mask bit suppresses this source
    /// from `irq_<group>`.
    pub intr_mask: bool,
    /// Optional explicit bit position in the synthesized
    /// `<group>_intr_enable` companion register. When unset, the
    /// synthesis pass falls back to declaration order. Set this for
    /// any field whose enable bit must be ABI-stable across DSL
    /// reorders (the typical case once a chip tapes out). Spanned so
    /// the LIR-level collision check can point at the source.
    pub intr_enable_bit: Option<Spanned<u32>>,
    /// Same as `intr_enable_bit` but for the `<group>_intr_mask`
    /// companion register.
    pub intr_mask_bit: Option<Spanned<u32>>,
    pub base_type: Spanned<BaseType>,
    pub field_conversion: Option<TypeConversion>,
    pub field_address: Spanned<AddressRange>,
    pub repeat: Option<Repeat>,
    /// Span of the whole object
    pub span: Span,
}

impl Field {
    #[must_use]
    pub fn get_type_specifier_string(&self) -> String {
        match &self.field_conversion {
            Some(fc) => {
                format!(
                    "{}:{}{}",
                    self.base_type,
                    fc.type_name.original(),
                    if fc.fallible { "?" } else { "" }
                )
            }
            None => self.base_type.to_string(),
        }
    }
}

#[derive(Debug, Clone, Default, PartialEq)]
pub struct Enum {
    pub description: String,
    pub name: Spanned<Identifier<Type>>,
    pub variants: Vec<EnumVariant>,
    pub base_type: Spanned<BaseType>,
    pub size_bits: Option<u32>,
    pub generation_style: Option<EnumGenerationStyle>,
    /// Span of the whole object
    pub span: Span,
}

impl Enum {
    #[must_use]
    pub fn new(
        description: String,
        name: Spanned<Identifier<Type>>,
        variants: Vec<EnumVariant>,
        base_type: Spanned<BaseType>,
        size_bits: Option<u32>,
        span: Span,
    ) -> Self {
        Self {
            description,
            name,
            variants,
            base_type,
            size_bits,
            generation_style: None,
            span,
        }
    }

    #[cfg(test)]
    pub fn new_with_style(
        description: String,
        name: Spanned<Identifier<Type>>,
        variants: Vec<EnumVariant>,
        base_type: Spanned<BaseType>,
        size_bits: Option<u32>,
        generation_style: EnumGenerationStyle,
        span: Span,
    ) -> Self {
        Self {
            description,
            name,
            variants,
            base_type,
            size_bits,
            generation_style: Some(generation_style),
            span,
        }
    }

    /// Get an iterator over the variants, but with an extra counter to get the specified discriminant for each.
    ///
    /// *Note:* The validity of this is checked in the [`passes::enum_values_checked`] pass. If this function is run
    /// before that pass, there might be weird results.
    pub fn iter_variants_with_discriminant(&self) -> impl Iterator<Item = (i128, &EnumVariant)> {
        let mut next_discriminant = 0;
        self.variants.iter().map(move |variant| {
            if let Some(discriminant) = variant.value.specified_discriminant() {
                next_discriminant = discriminant + 1;
                (discriminant, variant)
            } else {
                let discriminant = next_discriminant;
                next_discriminant += 1;
                (discriminant, variant)
            }
        })
    }

    /// Get an iterator over the variants, but with an extra counter to get the specified discriminant for each.
    ///
    /// *Note:* The validity of this is checked in the [`passes::enum_values_checked`] pass. If this function is run
    /// before that pass, there might be weird results.
    pub fn iter_variants_with_discriminant_mut(
        &mut self,
    ) -> impl Iterator<Item = (i128, &mut EnumVariant)> {
        let mut next_discriminant = 0;
        self.variants.iter_mut().map(move |variant| {
            if let Some(discriminant) = variant.value.specified_discriminant() {
                next_discriminant = discriminant + 1;
                (discriminant, variant)
            } else {
                let discriminant = next_discriminant;
                next_discriminant += 1;
                (discriminant, variant)
            }
        })
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum EnumGenerationStyle {
    /// Not all basetype values can be converted to a variant
    Fallible,
    /// All bitpatterns within bits 0..size-bits are covered.
    /// The general interface is fallible, but this special knowledge can be used for safety guarantees
    InfallibleWithinRange,
    /// There's a fallback, so it's always safe
    Fallback,
}

impl EnumGenerationStyle {
    /// Returns `true` if the enum generation style is [`Fallible`].
    ///
    /// [`Fallible`]: EnumGenerationStyle::Fallible
    #[must_use]
    pub fn is_fallible(&self) -> bool {
        matches!(self, Self::Fallible)
    }
}

#[derive(Debug, Clone, Default, PartialEq)]
pub struct EnumVariant {
    pub description: String,
    pub name: Spanned<Identifier<All>>,
    pub value: EnumValue,
    /// Span of the whole object
    pub span: Span,
}

#[derive(Debug, Clone, PartialEq, Eq, Default, Hash)]
pub enum EnumValue {
    #[default]
    Unspecified,
    Specified(i128),
    Default(i128),
    UnspecifiedDefault,
    CatchAll(i128),
    UnspecifiedCatchAll,
}

impl EnumValue {
    #[must_use]
    pub fn is_default(&self) -> bool {
        matches!(self, Self::Default(_) | Self::UnspecifiedDefault)
    }

    #[must_use]
    pub fn is_catch_all(&self) -> bool {
        matches!(self, Self::CatchAll(_) | Self::UnspecifiedCatchAll)
    }

    pub fn specified_discriminant(&self) -> Option<i128> {
        match self {
            Self::Unspecified | Self::UnspecifiedDefault | Self::UnspecifiedCatchAll => None,
            Self::Specified(val) | EnumValue::Default(val) | EnumValue::CatchAll(val) => Some(*val),
        }
    }

    pub fn specify(&mut self, num: i128) {
        *self = match self {
            EnumValue::Unspecified | EnumValue::Specified(_) => Self::Specified(num),
            EnumValue::Default(_) | EnumValue::UnspecifiedDefault => Self::Default(num),
            EnumValue::CatchAll(_) | EnumValue::UnspecifiedCatchAll => Self::CatchAll(num),
        };
    }
}

#[derive(Debug, Clone, Default, PartialEq)]
pub struct Command {
    pub description: String,
    pub name: Spanned<Identifier<Operation>>,
    pub address: Spanned<i128>,
    pub allow_address_overlap: bool,
    pub repeat: Option<Repeat>,

    pub field_set_ref_in: Option<Spanned<IdentifierRef<Type>>>,
    pub field_set_ref_out: Option<Spanned<IdentifierRef<Type>>>,
    /// HW-side handshake mode. `None` means the command has no SV target
    /// emission (current default — preserves backward compatibility with
    /// pre-M6 cases that defined commands purely as a Rust HAL concept).
    pub hw_handshake: Option<HwHandshake>,

    /// Span of the whole object
    pub span: Span,
}

#[derive(Debug, Clone, Default, PartialEq)]
pub struct Buffer {
    pub description: String,
    pub name: Spanned<Identifier<Operation>>,
    pub access: Access,
    pub address: Spanned<i128>,
    /// HW mapping for the buffer. `None` keeps the buffer out of the SV
    /// target (Rust-target-only behaviour, unchanged from pre-M6b).
    pub hw_kind: Option<HwKind>,
    /// Capacity hint for the user's FIFO instance. Carried verbatim on the
    /// generated `<dev>__out_t` struct doc comment; the regblock itself
    /// does not instantiate the storage.
    pub depth: Option<u32>,
    /// Address at which a future companion status register (with
    /// `level`/`full`/`empty`/`almost_full` fields) will live once
    /// synthesis lands. Currently surfaced only in doc comments.
    pub status_address: Option<i128>,
    /// Number of data words behind a single bus access for streaming
    /// modes. Currently informational only — emitted in doc comments.
    pub words: Option<u32>,
    /// Span of the whole object
    pub span: Span,
}

#[derive(Debug, Clone, Default, PartialEq)]
pub struct Extern {
    pub description: String,
    pub name: Spanned<Identifier<Type>>,
    /// From/into what base type can this extern be converted?
    pub base_type: Spanned<BaseType>,
    /// If true, this extern can be converted infallibly too
    pub supports_infallible: bool,
    /// The user-specified size of the max value of the base type that should be expected
    pub size_bits: Option<Spanned<u64>>,
    /// Span of the whole object
    pub span: Span,
}

#[derive(Debug, Clone, Hash, PartialEq, Eq)]
pub enum UniqueId {
    Object {
        object_name: Spanned<Identifier<RuntimeType>>,
    },
    Field {
        parent_id: Box<UniqueId>,
        field_name: Spanned<Identifier<RuntimeType>>,
    },
    Variant {
        parent_id: Box<UniqueId>,
        variant_name: Spanned<Identifier<RuntimeType>>,
    },
}

impl UniqueId {
    #[must_use]
    pub fn span(&self) -> Span {
        match self {
            UniqueId::Object { object_name } => object_name.span,
            UniqueId::Field { field_name, .. } => field_name.span,
            UniqueId::Variant { variant_name, .. } => variant_name.span,
        }
    }

    pub fn identifier(&self) -> &Identifier<RuntimeType> {
        match self {
            UniqueId::Object { object_name } => object_name,
            UniqueId::Field { field_name, .. } => field_name,
            UniqueId::Variant { variant_name, .. } => variant_name,
        }
    }

    /// *Only for tests:* Create a new instance with a dummy span.
    #[cfg(test)]
    pub fn new_test<T: IdentifierType>(identifier: Identifier<T>) -> Self {
        use device_driver_common::span::SpanExt;

        Self::Object {
            object_name: identifier.to_runtime_type().with_dummy_span(),
        }
    }
}

impl Display for UniqueId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            UniqueId::Object { object_name } => write!(f, "{}", object_name.original()),
            UniqueId::Field {
                parent_id,
                field_name,
            } => write!(f, "{parent_id} {{ {} }}", field_name.original()),
            UniqueId::Variant {
                parent_id,
                variant_name,
            } => write!(f, "{parent_id} {{ {} }}", variant_name.original()),
        }
    }
}

pub trait Unique {
    type Metadata;

    fn id(&self) -> UniqueId
    where
        Self::Metadata: Empty;
    fn id_with(&self, meta: Self::Metadata) -> UniqueId;

    fn has_id(&self, id: &UniqueId) -> bool
    where
        Self::Metadata: Empty;
    fn has_id_with(&self, meta: Self::Metadata, id: &UniqueId) -> bool {
        self.id_with(meta) == *id
    }
}

pub trait Empty {}
impl Empty for () {}

macro_rules! impl_unique_object {
    ($t:ty) => {
        impl Unique for $t {
            type Metadata = ();

            fn id(&self) -> UniqueId {
                UniqueId::Object {
                    object_name: self
                        .name
                        .value
                        .clone()
                        .to_runtime_type()
                        .with_span(self.name.span),
                }
            }

            fn id_with(&self, _: Self::Metadata) -> UniqueId {
                self.id()
            }

            fn has_id(&self, id: &UniqueId) -> bool {
                match id {
                    UniqueId::Object { object_name } => {
                        self.name.as_runtime_type() == &object_name.value
                    }
                    _ => false,
                }
            }
        }
    };
}

impl_unique_object!(Device);
impl_unique_object!(Register);
impl_unique_object!(Command);
impl_unique_object!(Buffer);
impl_unique_object!(Block);
impl_unique_object!(Enum);
impl_unique_object!(FieldSet);
impl_unique_object!(Extern);

impl Unique for Field {
    type Metadata = UniqueId;

    fn id(&self) -> UniqueId {
        unreachable!()
    }

    fn id_with(&self, parent: Self::Metadata) -> UniqueId {
        UniqueId::Field {
            parent_id: Box::new(parent),
            field_name: self
                .name
                .value
                .clone()
                .to_runtime_type()
                .with_span(self.name.span),
        }
    }

    fn has_id(&self, _id: &UniqueId) -> bool {
        unreachable!()
    }
}

impl Unique for EnumVariant {
    type Metadata = UniqueId;

    fn id(&self) -> UniqueId {
        unreachable!()
    }

    fn id_with(&self, parent: Self::Metadata) -> UniqueId {
        UniqueId::Variant {
            parent_id: Box::new(parent),
            variant_name: self
                .name
                .value
                .clone()
                .to_runtime_type()
                .with_span(self.name.span),
        }
    }

    fn has_id(&self, _id: &UniqueId) -> bool {
        unreachable!()
    }
}

impl Unique for Object {
    type Metadata = ();

    fn id(&self) -> UniqueId {
        match self {
            Object::Device(val) => val.id(),
            Object::Block(val) => val.id(),
            Object::Register(val) => val.id(),
            Object::Command(val) => val.id(),
            Object::Buffer(val) => val.id(),
            Object::FieldSet(val) => val.id(),
            Object::Enum(val) => val.id(),
            Object::Extern(val) => val.id(),
            // Special
            Object::Field(_) => unimplemented!(),
        }
    }

    fn id_with(&self, (): Self::Metadata) -> UniqueId {
        self.id()
    }

    fn has_id(&self, id: &UniqueId) -> bool {
        match self {
            Object::Device(val) => val.has_id(id),
            Object::Block(val) => val.has_id(id),
            Object::Register(val) => val.has_id(id),
            Object::Command(val) => val.has_id(id),
            Object::Buffer(val) => val.has_id(id),
            Object::FieldSet(val) => val.has_id(id),
            Object::Enum(val) => val.has_id(id),
            Object::Extern(val) => val.has_id(id),
            // Special
            Object::Field(_) => unimplemented!(),
        }
    }
}

#[cfg(test)]
mod tests {
    use device_driver_common::span::SpanExt;

    use super::*;

    #[test]
    fn iter_works() {
        const NAME_ORDER: &[&str] = &["a", "b", "c", "d"];

        let mut manifest = Manifest {
            description: Default::default(),
            name: Default::default(),
            objects: vec![
                Object::Device(Device {
                    description: String::new(),
                    name: Identifier::try_parse("a").unwrap().with_dummy_span(),
                    device_config: DeviceConfig {
                        ..Default::default()
                    },
                    objects: vec![
                        Object::Extern(Extern {
                            name: Identifier::try_parse("b").unwrap().with_dummy_span(),
                            ..Default::default()
                        }),
                        Object::Extern(Extern {
                            name: Identifier::try_parse("c").unwrap().with_dummy_span(),
                            ..Default::default()
                        }),
                    ],
                    span: Span::default(),
                }),
                Object::Extern(Extern {
                    name: Identifier::try_parse("d").unwrap().with_dummy_span(),
                    ..Default::default()
                }),
            ],
            config: Default::default(),
            span: Default::default(),
        };

        let names: Vec<_> = manifest
            .iter_objects()
            .map(|o| o.name().original())
            .collect();
        assert_eq!(&names, NAME_ORDER);

        let mut names = Vec::new();
        let mut lender = manifest.iter_objects_with_config_mut();
        while let Some((object, _)) = lender.next() {
            names.push(object.name().original().to_string());
        }
        assert_eq!(&names, NAME_ORDER);
    }

    #[test]
    fn correct_integer_size_bits() {
        assert_eq!(Integer::U8.bits_required(0, 0), 0);
        assert_eq!(Integer::U8.bits_required(0, 1), 1);
        assert_eq!(Integer::U8.bits_required(0, 2), 2);
        assert_eq!(Integer::U8.bits_required(0, 3), 2);
        assert_eq!(Integer::U8.bits_required(0, 4), 3);

        assert_eq!(Integer::I8.bits_required(0, 0), 0);
        assert_eq!(Integer::I8.bits_required(-1, 0), 1);
        assert_eq!(Integer::I8.bits_required(-1, 1), 2);
        assert_eq!(Integer::I8.bits_required(0, 1), 2);
        assert_eq!(Integer::I8.bits_required(-2, 1), 2);
        assert_eq!(Integer::I8.bits_required(0, 2), 3);
        assert_eq!(Integer::I8.bits_required(-128, 0), 8);
        assert_eq!(Integer::I8.bits_required(-129, 0), 9);
        assert_eq!(Integer::I8.bits_required(0, 127), 8);
        assert_eq!(Integer::I8.bits_required(0, 128), 9);
        assert_eq!(Integer::I8.bits_required(-16, 15), 5);
        assert_eq!(Integer::I8.bits_required(-16, 16), 6);
        assert_eq!(Integer::I8.bits_required(-17, 15), 6);
    }
}
