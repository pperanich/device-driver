#!/usr/bin/env cargo
---
[package]
edition = "2024"
[dependencies]
device-driver = { path="../../../../device-driver", default-features=false }
---
#![deny(warnings)]
#![allow(unexpected_cfgs)]
fn main() {}

// This code was generated using device-driver `2.0.0-alpha.1` (xxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxx),
// a tool distributed under MIT OR Apache-2.0 by Dion Dokter <dev@diondokter.nl>
// This version was built for xxxx-xxxx-xxxx using rustc 1.xx.x (xxxxxxxxx xxxx-xx-xx)
// 
// For more information about device-driver, visit the website: https://device-driver.com

/// Exercises register-level `reserved-behavior` (three variants) and the
/// `external: allow` opt-in. Rust target ignores both.
///
/// Root block of the ReservedExternal driver
#[derive(Debug)]
pub struct ReservedExternal<I> {
    pub(crate) interface: I,
    #[doc(hidden)]
    base_address: u8,
}
impl<I> ReservedExternal<I> {
    /// Create a new instance of the block based on device interface
    pub const fn new(interface: I) -> Self {
        Self { interface, base_address: 0 }
    }
    /// Default ro_zero — reserved bits read as 0.
    #[doc(alias = "CtrlZero")]
    pub fn ctrl_zero(
        &mut self,
    ) -> ::device_driver::RegisterOperation<
        '_,
        Self,
        CtrlZeroFields,
        u8,
        ::device_driver::RW,
        (),
    >
    where
        I: ::device_driver::RegisterInterfaceBase<AddressType = u8>,
    {
        let address = self.base_address + 0;
        ::device_driver::RegisterOperation::new(
            self,
            address as u8,
            CtrlZeroFields::default,
        )
    }
    /// ro_preserve — reserved bits keep reset value, readable.
    ///
    /// Reset value: `0xDEADBEEF`
    #[doc(alias = "CtrlPreserve")]
    pub fn ctrl_preserve(
        &mut self,
    ) -> ::device_driver::RegisterOperation<
        '_,
        Self,
        CtrlPreserveFields,
        u8,
        ::device_driver::RW,
        (),
    >
    where
        I: ::device_driver::RegisterInterfaceBase<AddressType = u8>,
    {
        let address = self.base_address + 4;
        ::device_driver::RegisterOperation::new(
            self,
            address as u8,
            || CtrlPreserveFields::from([239, 190, 173, 222]),
        )
    }
    /// rw_storage — reserved bits accept SW writes like a hidden RW field.
    ///
    /// Reset value: `0`
    #[doc(alias = "Scratch")]
    pub fn scratch(
        &mut self,
    ) -> ::device_driver::RegisterOperation<
        '_,
        Self,
        ScratchFields,
        u8,
        ::device_driver::RW,
        (),
    >
    where
        I: ::device_driver::RegisterInterfaceBase<AddressType = u8>,
    {
        let address = self.base_address + 8;
        ::device_driver::RegisterOperation::new(
            self,
            address as u8,
            || ScratchFields::from([0, 0, 0, 0]),
        )
    }
    /// External register — no storage flop. Bus reads/writes pass through
    /// to user RTL via `hwif_out.ctrl_ext_*` + `hwif_in.ctrl_ext_rd_data`.
    #[doc(alias = "ExtCtrl")]
    pub fn ext_ctrl(
        &mut self,
    ) -> ::device_driver::RegisterOperation<
        '_,
        Self,
        ExtCtrlFields,
        u8,
        ::device_driver::RW,
        (),
    >
    where
        I: ::device_driver::RegisterInterfaceBase<AddressType = u8>,
    {
        let address = self.base_address + 12;
        ::device_driver::RegisterOperation::new(
            self,
            address as u8,
            ExtCtrlFields::default,
        )
    }
}
impl<I> ::device_driver::Block for ReservedExternal<I> {
    type Interface = I;
    type RegisterAddressType = u8;
    type CommandAddressType = u8;
    type BufferAddressType = u8;
    type RegisterAddressMode = ();
    fn interface(&mut self) -> &mut Self::Interface {
        &mut self.interface
    }
}
#[derive(Copy, Clone, Eq, PartialEq)]
#[repr(transparent)]
pub struct ExtCtrlFields {
    /// The internal bits
    bits: [u8; 4],
}
unsafe impl ::device_driver::Fieldset for ExtCtrlFields {
    const METADATA: ::device_driver::FieldsetMetadata = ::device_driver::FieldsetMetadata::new()
        .with_byte_order(::device_driver::ByteOrder::LE);
    const ZERO: Self = Self { bits: [0; 4] };
}
impl ExtCtrlFields {
    /// `31:0` - Read the `cmd` field.
    ///
    #[must_use]
    pub fn cmd(&self) -> u32 {
        let start = 0;
        let end = 31;
        let raw = unsafe {
            ::device_driver::ops::load::<
                u32,
                ::device_driver::ops::LE,
            >(&self.bits, start, end)
        };
        raw
    }
    /// `31:0` - Set the `cmd` field.
    ///
    pub fn set_cmd(&mut self, value: u32) {
        let start = 0;
        let end = 31;
        let raw = value;
        unsafe {
            ::device_driver::ops::store::<
                u32,
                ::device_driver::ops::LE,
            >(raw, start, end, &mut self.bits)
        };
    }
}
impl Default for ExtCtrlFields {
    fn default() -> Self {
        <Self as ::device_driver::Fieldset>::ZERO
    }
}
impl From<[u8; 4]> for ExtCtrlFields {
    fn from(bits: [u8; 4]) -> Self {
        Self { bits }
    }
}
impl From<ExtCtrlFields> for [u8; 4] {
    fn from(val: ExtCtrlFields) -> Self {
        val.bits
    }
}
impl core::fmt::Debug for ExtCtrlFields {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> Result<(), core::fmt::Error> {
        let mut d = f.debug_struct("ExtCtrlFields");
        d.field("cmd", &self.cmd());
        d.finish()
    }
}
#[cfg(feature = "defmt")]
impl defmt::Format for ExtCtrlFields {
    fn format(&self, f: defmt::Formatter) {
        defmt::write!(f, "ExtCtrlFields {{ ");
        defmt::write!(f, "cmd: {=u32}, ", & self.cmd());
        defmt::write!(f, "}}");
    }
}
impl core::ops::BitAnd for ExtCtrlFields {
    type Output = Self;
    fn bitand(mut self, rhs: Self) -> Self::Output {
        self &= rhs;
        self
    }
}
impl core::ops::BitAndAssign for ExtCtrlFields {
    fn bitand_assign(&mut self, rhs: Self) {
        for (l, r) in self.bits.iter_mut().zip(&rhs.bits) {
            *l &= *r;
        }
    }
}
impl core::ops::BitOr for ExtCtrlFields {
    type Output = Self;
    fn bitor(mut self, rhs: Self) -> Self::Output {
        self |= rhs;
        self
    }
}
impl core::ops::BitOrAssign for ExtCtrlFields {
    fn bitor_assign(&mut self, rhs: Self) {
        for (l, r) in self.bits.iter_mut().zip(&rhs.bits) {
            *l |= *r;
        }
    }
}
impl core::ops::BitXor for ExtCtrlFields {
    type Output = Self;
    fn bitxor(mut self, rhs: Self) -> Self::Output {
        self ^= rhs;
        self
    }
}
impl core::ops::BitXorAssign for ExtCtrlFields {
    fn bitxor_assign(&mut self, rhs: Self) {
        for (l, r) in self.bits.iter_mut().zip(&rhs.bits) {
            *l ^= *r;
        }
    }
}
impl core::ops::Not for ExtCtrlFields {
    type Output = Self;
    fn not(mut self) -> Self::Output {
        for val in self.bits.iter_mut() {
            *val = !*val;
        }
        self
    }
}
#[derive(Copy, Clone, Eq, PartialEq)]
#[repr(transparent)]
pub struct ScratchFields {
    /// The internal bits
    bits: [u8; 4],
}
unsafe impl ::device_driver::Fieldset for ScratchFields {
    const METADATA: ::device_driver::FieldsetMetadata = ::device_driver::FieldsetMetadata::new()
        .with_byte_order(::device_driver::ByteOrder::LE);
    const ZERO: Self = Self { bits: [0; 4] };
}
impl ScratchFields {
    /// `7:0` - Read the `tag` field.
    ///
    #[must_use]
    pub fn tag(&self) -> u8 {
        let start = 0;
        let end = 7;
        let raw = unsafe {
            ::device_driver::ops::load::<
                u8,
                ::device_driver::ops::LE,
            >(&self.bits, start, end)
        };
        raw
    }
    /// `7:0` - Set the `tag` field.
    ///
    pub fn set_tag(&mut self, value: u8) {
        let start = 0;
        let end = 7;
        let raw = value;
        unsafe {
            ::device_driver::ops::store::<
                u8,
                ::device_driver::ops::LE,
            >(raw, start, end, &mut self.bits)
        };
    }
}
impl Default for ScratchFields {
    fn default() -> Self {
        <Self as ::device_driver::Fieldset>::ZERO
    }
}
impl From<[u8; 4]> for ScratchFields {
    fn from(bits: [u8; 4]) -> Self {
        Self { bits }
    }
}
impl From<ScratchFields> for [u8; 4] {
    fn from(val: ScratchFields) -> Self {
        val.bits
    }
}
impl core::fmt::Debug for ScratchFields {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> Result<(), core::fmt::Error> {
        let mut d = f.debug_struct("ScratchFields");
        d.field("tag", &self.tag());
        d.finish()
    }
}
#[cfg(feature = "defmt")]
impl defmt::Format for ScratchFields {
    fn format(&self, f: defmt::Formatter) {
        defmt::write!(f, "ScratchFields {{ ");
        defmt::write!(f, "tag: {=u8}, ", & self.tag());
        defmt::write!(f, "}}");
    }
}
impl core::ops::BitAnd for ScratchFields {
    type Output = Self;
    fn bitand(mut self, rhs: Self) -> Self::Output {
        self &= rhs;
        self
    }
}
impl core::ops::BitAndAssign for ScratchFields {
    fn bitand_assign(&mut self, rhs: Self) {
        for (l, r) in self.bits.iter_mut().zip(&rhs.bits) {
            *l &= *r;
        }
    }
}
impl core::ops::BitOr for ScratchFields {
    type Output = Self;
    fn bitor(mut self, rhs: Self) -> Self::Output {
        self |= rhs;
        self
    }
}
impl core::ops::BitOrAssign for ScratchFields {
    fn bitor_assign(&mut self, rhs: Self) {
        for (l, r) in self.bits.iter_mut().zip(&rhs.bits) {
            *l |= *r;
        }
    }
}
impl core::ops::BitXor for ScratchFields {
    type Output = Self;
    fn bitxor(mut self, rhs: Self) -> Self::Output {
        self ^= rhs;
        self
    }
}
impl core::ops::BitXorAssign for ScratchFields {
    fn bitxor_assign(&mut self, rhs: Self) {
        for (l, r) in self.bits.iter_mut().zip(&rhs.bits) {
            *l ^= *r;
        }
    }
}
impl core::ops::Not for ScratchFields {
    type Output = Self;
    fn not(mut self) -> Self::Output {
        for val in self.bits.iter_mut() {
            *val = !*val;
        }
        self
    }
}
#[derive(Copy, Clone, Eq, PartialEq)]
#[repr(transparent)]
pub struct CtrlPreserveFields {
    /// The internal bits
    bits: [u8; 4],
}
unsafe impl ::device_driver::Fieldset for CtrlPreserveFields {
    const METADATA: ::device_driver::FieldsetMetadata = ::device_driver::FieldsetMetadata::new()
        .with_byte_order(::device_driver::ByteOrder::LE);
    const ZERO: Self = Self { bits: [0; 4] };
}
impl CtrlPreserveFields {
    /// `bit 0` - Read the `enable` field.
    ///
    #[must_use]
    pub fn enable(&self) -> bool {
        let start = 0;
        let end = 0;
        let raw = unsafe {
            ::device_driver::ops::load::<
                u8,
                ::device_driver::ops::LE,
            >(&self.bits, start, end)
        };
        raw > 0
    }
    /// `bit 0` - Set the `enable` field.
    ///
    pub fn set_enable(&mut self, value: bool) {
        let start = 0;
        let end = 0;
        let raw = value as _;
        unsafe {
            ::device_driver::ops::store::<
                u8,
                ::device_driver::ops::LE,
            >(raw, start, end, &mut self.bits)
        };
    }
}
impl Default for CtrlPreserveFields {
    fn default() -> Self {
        <Self as ::device_driver::Fieldset>::ZERO
    }
}
impl From<[u8; 4]> for CtrlPreserveFields {
    fn from(bits: [u8; 4]) -> Self {
        Self { bits }
    }
}
impl From<CtrlPreserveFields> for [u8; 4] {
    fn from(val: CtrlPreserveFields) -> Self {
        val.bits
    }
}
impl core::fmt::Debug for CtrlPreserveFields {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> Result<(), core::fmt::Error> {
        let mut d = f.debug_struct("CtrlPreserveFields");
        d.field("enable", &self.enable());
        d.finish()
    }
}
#[cfg(feature = "defmt")]
impl defmt::Format for CtrlPreserveFields {
    fn format(&self, f: defmt::Formatter) {
        defmt::write!(f, "CtrlPreserveFields {{ ");
        defmt::write!(f, "enable: {=bool}, ", & self.enable());
        defmt::write!(f, "}}");
    }
}
impl core::ops::BitAnd for CtrlPreserveFields {
    type Output = Self;
    fn bitand(mut self, rhs: Self) -> Self::Output {
        self &= rhs;
        self
    }
}
impl core::ops::BitAndAssign for CtrlPreserveFields {
    fn bitand_assign(&mut self, rhs: Self) {
        for (l, r) in self.bits.iter_mut().zip(&rhs.bits) {
            *l &= *r;
        }
    }
}
impl core::ops::BitOr for CtrlPreserveFields {
    type Output = Self;
    fn bitor(mut self, rhs: Self) -> Self::Output {
        self |= rhs;
        self
    }
}
impl core::ops::BitOrAssign for CtrlPreserveFields {
    fn bitor_assign(&mut self, rhs: Self) {
        for (l, r) in self.bits.iter_mut().zip(&rhs.bits) {
            *l |= *r;
        }
    }
}
impl core::ops::BitXor for CtrlPreserveFields {
    type Output = Self;
    fn bitxor(mut self, rhs: Self) -> Self::Output {
        self ^= rhs;
        self
    }
}
impl core::ops::BitXorAssign for CtrlPreserveFields {
    fn bitxor_assign(&mut self, rhs: Self) {
        for (l, r) in self.bits.iter_mut().zip(&rhs.bits) {
            *l ^= *r;
        }
    }
}
impl core::ops::Not for CtrlPreserveFields {
    type Output = Self;
    fn not(mut self) -> Self::Output {
        for val in self.bits.iter_mut() {
            *val = !*val;
        }
        self
    }
}
#[derive(Copy, Clone, Eq, PartialEq)]
#[repr(transparent)]
pub struct CtrlZeroFields {
    /// The internal bits
    bits: [u8; 4],
}
unsafe impl ::device_driver::Fieldset for CtrlZeroFields {
    const METADATA: ::device_driver::FieldsetMetadata = ::device_driver::FieldsetMetadata::new()
        .with_byte_order(::device_driver::ByteOrder::LE);
    const ZERO: Self = Self { bits: [0; 4] };
}
impl CtrlZeroFields {
    /// `bit 0` - Read the `enable` field.
    ///
    #[must_use]
    pub fn enable(&self) -> bool {
        let start = 0;
        let end = 0;
        let raw = unsafe {
            ::device_driver::ops::load::<
                u8,
                ::device_driver::ops::LE,
            >(&self.bits, start, end)
        };
        raw > 0
    }
    /// `bit 0` - Set the `enable` field.
    ///
    pub fn set_enable(&mut self, value: bool) {
        let start = 0;
        let end = 0;
        let raw = value as _;
        unsafe {
            ::device_driver::ops::store::<
                u8,
                ::device_driver::ops::LE,
            >(raw, start, end, &mut self.bits)
        };
    }
}
impl Default for CtrlZeroFields {
    fn default() -> Self {
        <Self as ::device_driver::Fieldset>::ZERO
    }
}
impl From<[u8; 4]> for CtrlZeroFields {
    fn from(bits: [u8; 4]) -> Self {
        Self { bits }
    }
}
impl From<CtrlZeroFields> for [u8; 4] {
    fn from(val: CtrlZeroFields) -> Self {
        val.bits
    }
}
impl core::fmt::Debug for CtrlZeroFields {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> Result<(), core::fmt::Error> {
        let mut d = f.debug_struct("CtrlZeroFields");
        d.field("enable", &self.enable());
        d.finish()
    }
}
#[cfg(feature = "defmt")]
impl defmt::Format for CtrlZeroFields {
    fn format(&self, f: defmt::Formatter) {
        defmt::write!(f, "CtrlZeroFields {{ ");
        defmt::write!(f, "enable: {=bool}, ", & self.enable());
        defmt::write!(f, "}}");
    }
}
impl core::ops::BitAnd for CtrlZeroFields {
    type Output = Self;
    fn bitand(mut self, rhs: Self) -> Self::Output {
        self &= rhs;
        self
    }
}
impl core::ops::BitAndAssign for CtrlZeroFields {
    fn bitand_assign(&mut self, rhs: Self) {
        for (l, r) in self.bits.iter_mut().zip(&rhs.bits) {
            *l &= *r;
        }
    }
}
impl core::ops::BitOr for CtrlZeroFields {
    type Output = Self;
    fn bitor(mut self, rhs: Self) -> Self::Output {
        self |= rhs;
        self
    }
}
impl core::ops::BitOrAssign for CtrlZeroFields {
    fn bitor_assign(&mut self, rhs: Self) {
        for (l, r) in self.bits.iter_mut().zip(&rhs.bits) {
            *l |= *r;
        }
    }
}
impl core::ops::BitXor for CtrlZeroFields {
    type Output = Self;
    fn bitxor(mut self, rhs: Self) -> Self::Output {
        self ^= rhs;
        self
    }
}
impl core::ops::BitXorAssign for CtrlZeroFields {
    fn bitxor_assign(&mut self, rhs: Self) {
        for (l, r) in self.bits.iter_mut().zip(&rhs.bits) {
            *l ^= *r;
        }
    }
}
impl core::ops::Not for CtrlZeroFields {
    type Output = Self;
    fn not(mut self) -> Self::Output {
        for val in self.bits.iter_mut() {
            *val = !*val;
        }
        self
    }
}
