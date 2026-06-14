use std::{fmt::Display, num::NonZeroU32, str::FromStr};

use crate::{
    identifier::{IdentifierRef, Type},
    span::Spanned,
};

pub trait VariantNames {
    /// Names of the variants of this enum
    const VARIANTS: &'static [&'static str];
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
#[cfg_attr(feature = "arbitrary", derive(arbitrary::Arbitrary))]
pub enum Integer {
    U8,
    U16,
    U32,
    U64,
    I8,
    I16,
    #[default]
    I32,
    I64,
}

impl VariantNames for Integer {
    const VARIANTS: &[&'static str] = &["u8", "u16", "u32", "u64", "i8", "i16", "i32", "i64"];
}

impl Display for Integer {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", Self::VARIANTS[*self as usize])
    }
}

impl FromStr for Integer {
    type Err = ();

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "u8" => Ok(Self::U8),
            "u16" => Ok(Self::U16),
            "u32" => Ok(Self::U32),
            "u64" => Ok(Self::U64),
            "i8" => Ok(Self::I8),
            "i16" => Ok(Self::I16),
            "i32" => Ok(Self::I32),
            "i64" => Ok(Self::I64),
            _ => Err(()),
        }
    }
}

impl Integer {
    #[must_use]
    pub const fn is_signed(&self) -> bool {
        self.min_value() != 0
    }

    #[must_use]
    pub const fn min_value(&self) -> i128 {
        match self {
            Integer::U8 => u8::MIN as i128,
            Integer::U16 => u16::MIN as i128,
            Integer::U32 => u32::MIN as i128,
            Integer::U64 => u64::MIN as i128,
            Integer::I8 => i8::MIN as i128,
            Integer::I16 => i16::MIN as i128,
            Integer::I32 => i32::MIN as i128,
            Integer::I64 => i64::MIN as i128,
        }
    }

    #[must_use]
    pub const fn max_value(&self) -> i128 {
        match self {
            Integer::U8 => u8::MAX as i128,
            Integer::U16 => u16::MAX as i128,
            Integer::U32 => u32::MAX as i128,
            Integer::U64 => u64::MAX as i128,
            Integer::I8 => i8::MAX as i128,
            Integer::I16 => i16::MAX as i128,
            Integer::I32 => i32::MAX as i128,
            Integer::I64 => i64::MAX as i128,
        }
    }

    #[must_use]
    pub const fn size_bits(&self) -> u32 {
        match self {
            Integer::U8 => 8,
            Integer::U16 => 16,
            Integer::U32 => 32,
            Integer::U64 => 64,
            Integer::I8 => 8,
            Integer::I16 => 16,
            Integer::I32 => 32,
            Integer::I64 => 64,
        }
    }

    /// Find the smallest integer type that can fully contain the min and max
    /// and is equal or larger than the given `size_bits`.
    ///
    /// This function has a preference for unsigned integers.
    /// You can force a signed integer by making the min be negative (e.g. -1)
    #[must_use]
    pub const fn find_smallest(min: i128, max: i128, size_bits: u64) -> Option<Integer> {
        Some(match (min, max, size_bits) {
            (0.., ..0x1_00, ..=8) => Integer::U8,
            (0.., ..0x1_0000, ..=16) => Integer::U16,
            (0.., ..0x1_0000_0000, ..=32) => Integer::U32,
            (0.., ..0x1_0000_0000_0000_0000, ..=64) => Integer::U64,
            (-0x80.., ..0x80, ..=8) => Integer::I8,
            (-0x8000.., ..0x8000, ..=16) => Integer::I16,
            (-0x8000_00000.., ..0x8000_0000, ..=32) => Integer::I32,
            (-0x8000_0000_0000_0000.., ..0x8000_0000_0000_0000, ..=64) => Integer::I64,
            _ => return None,
        })
    }

    /// Given the min and the max and the sign of the integer,
    /// how many bits are required to fit the min and max? (inclusive)
    #[must_use]
    pub const fn bits_required(&self, min: i128, max: i128) -> u32 {
        assert!(max >= min);

        if self.is_signed() {
            let min_bits = if min.is_negative() {
                i128::BITS - (min.abs() - 1).leading_zeros() + 1
            } else {
                0
            };
            let max_bits = if max.is_positive() {
                i128::BITS - max.leading_zeros() + 1
            } else {
                0
            };

            if min_bits > max_bits {
                min_bits
            } else {
                max_bits
            }
        } else {
            assert!(min >= 0);
            i128::BITS - max.leading_zeros()
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Hash)]
#[cfg_attr(feature = "arbitrary", derive(arbitrary::Arbitrary))]
pub enum Access {
    #[default]
    RW,
    RO,
    WO,
}

impl Access {
    #[must_use]
    pub fn is_readable(&self) -> bool {
        match self {
            Access::RW => true,
            Access::RO => true,
            Access::WO => false,
        }
    }
}

impl VariantNames for Access {
    const VARIANTS: &[&'static str] = &["RW", "RO", "WO"];
}

impl Display for Access {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", Self::VARIANTS[*self as usize])
    }
}

impl FromStr for Access {
    type Err = ();

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "RW" => Ok(Self::RW),
            "RO" => Ok(Self::RO),
            "WO" => Ok(Self::WO),
            _ => Err(()),
        }
    }
}

/// Side-effect of a software write to a field.
/// Layers on top of `Access` to express W1C/W1S/W1T patterns common in CSRs.
///
/// - `Store` (default): write replaces affected bits with `wdata & wr_biten`
/// - `Clear` (W1C): bits where `wdata & wr_biten == 1` are cleared in storage
/// - `Set`   (W1S): bits where `wdata & wr_biten == 1` are set in storage
/// - `Toggle`(W1T): bits where `wdata & wr_biten == 1` are toggled in storage
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
#[cfg_attr(feature = "arbitrary", derive(arbitrary::Arbitrary))]
pub enum OnWrite {
    #[default]
    Store,
    Clear,
    Set,
    Toggle,
}

impl VariantNames for OnWrite {
    const VARIANTS: &[&'static str] = &["store", "clear", "set", "toggle"];
}

impl Display for OnWrite {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", Self::VARIANTS[*self as usize])
    }
}

impl FromStr for OnWrite {
    type Err = ();

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "store" => Ok(Self::Store),
            "clear" => Ok(Self::Clear),
            "set" => Ok(Self::Set),
            "toggle" => Ok(Self::Toggle),
            _ => Err(()),
        }
    }
}

/// Side-effect of a software read of a field.
/// Layered on top of `Access`. Read-clear (RC) and read-set (RS) are common in
/// status registers where the act of reading acknowledges the event.
///
/// - `Store` (default): read is purely combinational, storage unchanged
/// - `Clear` (RC): the field's storage bits clear after a successful read
/// - `Set`   (RS): the field's storage bits set after a successful read
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
#[cfg_attr(feature = "arbitrary", derive(arbitrary::Arbitrary))]
pub enum OnRead {
    #[default]
    Store,
    Clear,
    Set,
}

impl VariantNames for OnRead {
    const VARIANTS: &[&'static str] = &["store", "clear", "set"];
}

impl Display for OnRead {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", Self::VARIANTS[*self as usize])
    }
}

impl FromStr for OnRead {
    type Err = ();

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "store" => Ok(Self::Store),
            "clear" => Ok(Self::Clear),
            "set" => Ok(Self::Set),
            _ => Err(()),
        }
    }
}

/// Hardware-side access mode for a field. Orthogonal to `Access` (which is
/// software-side). Defaults to `RO` — HW observes storage via `hwif_out` only.
///
/// - `RO` (default): HW reads storage via `hwif_out.<field>`
/// - `RW`: HW also writes via `hwif_in.<field>` gated by a `_we` strobe
/// - `WO`: HW writes only; storage is HW-driven, not observable by HW
///
/// Reuses the `Access` lexer token for `RW`/`RO`/`WO`. The property setter
/// disambiguates by key (`hw-access` vs `access`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
#[cfg_attr(feature = "arbitrary", derive(arbitrary::Arbitrary))]
pub enum HwAccess {
    #[default]
    RO,
    RW,
    WO,
}

impl HwAccess {
    #[must_use]
    pub fn is_writable(&self) -> bool {
        matches!(self, HwAccess::RW | HwAccess::WO)
    }

    #[must_use]
    pub fn is_readable(&self) -> bool {
        matches!(self, HwAccess::RW | HwAccess::RO)
    }
}

impl VariantNames for HwAccess {
    const VARIANTS: &[&'static str] = &["RO", "RW", "WO"];
}

impl Display for HwAccess {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", Self::VARIANTS[*self as usize])
    }
}

impl From<Access> for HwAccess {
    fn from(a: Access) -> HwAccess {
        match a {
            Access::RW => HwAccess::RW,
            Access::RO => HwAccess::RO,
            Access::WO => HwAccess::WO,
        }
    }
}

/// Which side wins when a hardware write and a software write collide on the
/// same clock edge. Default `Hw` matches SystemRDL convention and matches
/// what every prior milestone of the SV target assumed.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
#[cfg_attr(feature = "arbitrary", derive(arbitrary::Arbitrary))]
pub enum Precedence {
    #[default]
    Hw,
    Sw,
}

impl VariantNames for Precedence {
    const VARIANTS: &[&'static str] = &["hw", "sw"];
}

impl Display for Precedence {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", Self::VARIANTS[*self as usize])
    }
}

impl FromStr for Precedence {
    type Err = ();

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "hw" => Ok(Self::Hw),
            "sw" => Ok(Self::Sw),
            _ => Err(()),
        }
    }
}

/// Behaviour of bits that aren't covered by any declared field inside a
/// register. Affects both the read mask (what bits appear on
/// `cpuif_rd_data`) and whether SW writes are allowed to land in those
/// slots.
///
/// - `RoZero` (default): reserved bits read as 0; SW writes to them are
///   silently dropped.
/// - `RoPreserve`: reserved bits keep whatever sat in storage (typically
///   reset value); SW writes are dropped but the bits remain visible.
/// - `RwStorage`: reserved bits behave like a hidden RW field — SW writes
///   land in storage, reads return the latest written value.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
#[cfg_attr(feature = "arbitrary", derive(arbitrary::Arbitrary))]
pub enum ReservedBehavior {
    #[default]
    RoZero,
    RoPreserve,
    RwStorage,
}

impl VariantNames for ReservedBehavior {
    const VARIANTS: &[&'static str] = &["ro_zero", "ro_preserve", "rw_storage"];
}

impl Display for ReservedBehavior {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", Self::VARIANTS[*self as usize])
    }
}

impl FromStr for ReservedBehavior {
    type Err = ();

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "ro_zero" => Ok(Self::RoZero),
            "ro_preserve" => Ok(Self::RoPreserve),
            "rw_storage" => Ok(Self::RwStorage),
            _ => Err(()),
        }
    }
}

/// Hardware-side mapping for a `buffer` block-method. v1 supports only
/// `Fifo`; reserved space for future `Stream` / `Bram` modes.
///
/// - `Fifo`: bus writes push, bus reads pop. The regblock exposes
///   `hwif_out.buf_<n>_push` / `_wdata` and `hwif_out.buf_<n>_pop`
///   strobes; the actual FIFO storage lives in user RTL and feeds back
///   `hwif_in.buf_<n>_rdata`. v1 deliberately does not synthesize a
///   status companion register — that pass lands later.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
#[cfg_attr(feature = "arbitrary", derive(arbitrary::Arbitrary))]
pub enum HwKind {
    #[default]
    Fifo,
}

impl VariantNames for HwKind {
    const VARIANTS: &[&'static str] = &["fifo"];
}

impl Display for HwKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", Self::VARIANTS[*self as usize])
    }
}

impl FromStr for HwKind {
    type Err = ();

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "fifo" => Ok(Self::Fifo),
            _ => Err(()),
        }
    }
}

/// Hardware-side handshake mode for a `command` block-method. Picks how the
/// command's payload + completion semantics map to RTL ports on the regs
/// module. v1 supports only `Strobe`; reserved space for future
/// `ValidReady` and `Fifo` modes.
///
/// - `Strobe`: bus write at the command address pulses
///   `hwif_out.cmd_<n>_strobe` for one cycle and drives
///   `hwif_out.cmd_<n>_in` to a snapshot of the bus write data. If the
///   command declares `fields-out`, the regblock latches
///   `hwif_in.cmd_<n>_resp` whenever `hwif_in.cmd_<n>_resp_valid`
///   asserts; the latched value is returned on bus reads. Bus reads when
///   no response payload exists return zero.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
#[cfg_attr(feature = "arbitrary", derive(arbitrary::Arbitrary))]
pub enum HwHandshake {
    #[default]
    Strobe,
}

impl VariantNames for HwHandshake {
    const VARIANTS: &[&'static str] = &["strobe"];
}

impl Display for HwHandshake {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", Self::VARIANTS[*self as usize])
    }
}

impl FromStr for HwHandshake {
    type Err = ();

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "strobe" => Ok(Self::Strobe),
            _ => Err(()),
        }
    }
}

/// Trigger semantics for an interrupt-source field. The raw HW input
/// `hwif_in.<field>_intr` is sampled each cycle; this enum picks how the
/// detection event is derived from that input.
///
/// - `Level` (default): event = raw input high (transparent latch)
/// - `Posedge`: event = `~prev & raw`
/// - `Negedge`: event = `prev & ~raw`
/// - `Bothedge`: event = `prev ^ raw`
///
/// When `intr_sticky` is true (default) the event sets the field's storage,
/// and SW must clear it via `on-write: clear` (W1C). Detected events are
/// OR-reduced per-group into the `irq_<group>` output.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
#[cfg_attr(feature = "arbitrary", derive(arbitrary::Arbitrary))]
pub enum IntrTrigger {
    #[default]
    Level,
    Posedge,
    Negedge,
    Bothedge,
}

impl VariantNames for IntrTrigger {
    const VARIANTS: &[&'static str] = &["level", "posedge", "negedge", "bothedge"];
}

impl Display for IntrTrigger {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", Self::VARIANTS[*self as usize])
    }
}

impl FromStr for IntrTrigger {
    type Err = ();

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "level" => Ok(Self::Level),
            "posedge" => Ok(Self::Posedge),
            "negedge" => Ok(Self::Negedge),
            "bothedge" => Ok(Self::Bothedge),
            _ => Err(()),
        }
    }
}

/// Bus protocol that DSL asks the SV target to emit a wrapper for. Each
/// requested value produces one `<dev>_<bus>.sv` file that wraps the native
/// `<dev>_regs` CPUIF port set in the matching standard interface. `Native`
/// is just a structural pass-through retained for symmetry.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
#[cfg_attr(feature = "arbitrary", derive(arbitrary::Arbitrary))]
pub enum SvBus {
    #[default]
    Native,
    Apb3,
    Apb4,
    Axi4Lite,
    AhbLite,
}

impl SvBus {
    /// File-name suffix used when emitting the wrapper module.
    /// Lowercase `snake_case`, matches the user-facing DSL keyword.
    #[must_use]
    pub fn suffix(&self) -> &'static str {
        match self {
            SvBus::Native => "native",
            SvBus::Apb3 => "apb3",
            SvBus::Apb4 => "apb4",
            SvBus::Axi4Lite => "axi4lite",
            SvBus::AhbLite => "ahblite",
        }
    }
}

impl VariantNames for SvBus {
    const VARIANTS: &[&'static str] = &["native", "apb3", "apb4", "axi4lite", "ahblite"];
}

impl Display for SvBus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", Self::VARIANTS[*self as usize])
    }
}

impl FromStr for SvBus {
    type Err = ();

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "native" => Ok(Self::Native),
            "apb3" => Ok(Self::Apb3),
            "apb4" => Ok(Self::Apb4),
            "axi4lite" => Ok(Self::Axi4Lite),
            "ahblite" => Ok(Self::AhbLite),
            _ => Err(()),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
#[cfg_attr(feature = "arbitrary", derive(arbitrary::Arbitrary))]
pub enum ByteOrder {
    #[default]
    LE,
    BE,
}

impl VariantNames for ByteOrder {
    const VARIANTS: &[&'static str] = &["LE", "BE"];
}

impl Display for ByteOrder {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", Self::VARIANTS[*self as usize])
    }
}

impl FromStr for ByteOrder {
    type Err = ();

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "LE" => Ok(Self::LE),
            "BE" => Ok(Self::BE),
            _ => Err(()),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Hash)]
#[cfg_attr(feature = "arbitrary", derive(arbitrary::Arbitrary))]
pub enum BaseType {
    #[default]
    Unspecified,
    Bool,
    Uint,
    Int,
    FixedSize(Integer),
}

impl BaseType {
    /// Returns `true` if the base type is [`Unspecified`].
    ///
    /// [`Unspecified`]: BaseType::Unspecified
    #[must_use]
    pub fn is_unspecified(&self) -> bool {
        matches!(self, Self::Unspecified)
    }

    /// Returns `true` if the base type is [`FixedSize`].
    ///
    /// [`FixedSize`]: BaseType::FixedSize
    #[must_use]
    pub fn is_fixed_size(&self) -> bool {
        matches!(self, Self::FixedSize(..))
    }

    pub fn as_fixed_size(&self) -> Option<Integer> {
        if let Self::FixedSize(v) = self {
            Some(*v)
        } else {
            None
        }
    }
}

impl Display for BaseType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            BaseType::Unspecified => write!(f, "unspecified"),
            BaseType::Bool => write!(f, "bool"),
            BaseType::Uint => write!(f, "uint"),
            BaseType::Int => write!(f, "int"),
            BaseType::FixedSize(integer) => write!(f, "{integer}"),
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct TypeConversion {
    /// The name of the type we're converting to
    pub type_name: Spanned<IdentifierRef<Type>>,
    /// True when we want to use the fallible interface (like a Result<type, error>)
    pub fallible: bool,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Repeat {
    pub source: RepeatSource,
    pub stride: Spanned<i128>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum RepeatSource {
    Count(NonZeroU32),
    Enum(Spanned<IdentifierRef<Type>>),
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum ResetValue {
    Integer(u128),
    Array(Vec<u8>),
}

impl ResetValue {
    #[must_use]
    pub fn as_array(&self) -> Option<&Vec<u8>> {
        if let Self::Array(v) = self {
            Some(v)
        } else {
            None
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NodeType {
    Manifest,
    Device,
    Block,
    Register,
    Command,
    Buffer,
    FieldSet,
    Enum,
    Extern,
    Field,
}

impl FromStr for NodeType {
    type Err = ();

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "manifest" => Ok(Self::Manifest),
            "device" => Ok(Self::Device),
            "block" => Ok(Self::Block),
            "register" => Ok(Self::Register),
            "command" => Ok(Self::Command),
            "buffer" => Ok(Self::Buffer),
            "fieldset" => Ok(Self::FieldSet),
            "enum" => Ok(Self::Enum),
            "extern" => Ok(Self::Extern),
            "field" => Ok(Self::Field),
            _ => Err(()),
        }
    }
}

impl VariantNames for NodeType {
    const VARIANTS: &'static [&'static str] = &[
        "manifest", "device", "block", "register", "command", "buffer", "fieldset", "enum",
        "extern", "field",
    ];
}

impl Display for NodeType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", Self::VARIANTS[*self as usize])
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Hash)]
pub struct AddressRange {
    pub start: u32,
    /// Inclusive end
    pub end: u32,
}

impl AddressRange {
    #[allow(clippy::len_without_is_empty, reason = "Range can never be empty")]
    /// The amount of bits this range covers
    pub fn len(&self) -> u64 {
        self.end as u64 - self.start as u64 + 1
    }
}

/// Type to specify how addresses work
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "arbitrary", derive(arbitrary::Arbitrary))]
pub enum AddressMode {
    /// Objects are memory-mapped.
    ///
    /// If object `A` has address `X` and is `Y` bytes big, then object `B` (if it exists) will have the address `X+Y`.
    Mapped,
    /// Objects are sequentially indexed.
    ///
    /// If object `A` has address `X`, then object `B` (if it exists) will have the address `X+1`.
    Indexed,
}

impl VariantNames for AddressMode {
    const VARIANTS: &[&'static str] = &["mapped", "indexed"];
}

impl Display for AddressMode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", Self::VARIANTS[*self as usize])
    }
}

impl FromStr for AddressMode {
    type Err = ();

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "mapped" => Ok(Self::Mapped),
            "indexed" => Ok(Self::Indexed),
            _ => Err(()),
        }
    }
}
