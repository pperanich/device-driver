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

/// Exercises the `intr-enable-bit` / `intr-mask-bit` explicit bit
/// position knobs (item 8 from the SV-backend usability gap list).
/// Without these, the synthesis pass falls back to declaration order,
/// which silently shifts bit positions if the user reorders fields.
/// This case pins each opt-in field to a specific bit so the layout is
/// ABI-stable: `overflow` always at bit 0, `underflow` always at bit 3.
/// A new field added between them at a later DSL revision can be given
/// bit 1 or 2 without disturbing the pinned ones.
///
/// Root block of the IntrExplicitBits driver
#[derive(Debug)]
pub struct IntrExplicitBits<I> {
    pub(crate) interface: I,
    #[doc(hidden)]
    base_address: u8,
}
impl<I> IntrExplicitBits<I> {
    /// Create a new instance of the block based on device interface
    pub const fn new(interface: I) -> Self {
        Self { interface, base_address: 0 }
    }
    #[doc(alias = "Status")]
    pub fn status(
        &mut self,
    ) -> ::device_driver::RegisterOperation<
        '_,
        Self,
        StatusFields,
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
            StatusFields::default,
        )
    }
    /// Auto-synthesized intr_enable companion for IRQ group `root`.
    pub fn root_intr_enable(
        &mut self,
    ) -> ::device_driver::RegisterOperation<
        '_,
        Self,
        RootIntrEnableFields,
        u8,
        ::device_driver::RW,
        (),
    >
    where
        I: ::device_driver::RegisterInterfaceBase<AddressType = u8>,
    {
        let address = self.base_address + 64;
        ::device_driver::RegisterOperation::new(
            self,
            address as u8,
            RootIntrEnableFields::default,
        )
    }
    /// Auto-synthesized intr_mask companion for IRQ group `root`.
    pub fn root_intr_mask(
        &mut self,
    ) -> ::device_driver::RegisterOperation<
        '_,
        Self,
        RootIntrMaskFields,
        u8,
        ::device_driver::RW,
        (),
    >
    where
        I: ::device_driver::RegisterInterfaceBase<AddressType = u8>,
    {
        let address = self.base_address + 96;
        ::device_driver::RegisterOperation::new(
            self,
            address as u8,
            RootIntrMaskFields::default,
        )
    }
}
impl<I> ::device_driver::Block for IntrExplicitBits<I> {
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
pub struct StatusFields {
    /// The internal bits
    bits: [u8; 4],
}
unsafe impl ::device_driver::Fieldset for StatusFields {
    const METADATA: ::device_driver::FieldsetMetadata = ::device_driver::FieldsetMetadata::new()
        .with_byte_order(::device_driver::ByteOrder::LE);
    const ZERO: Self = Self { bits: [0; 4] };
}
impl StatusFields {
    /// `bit 0` - Read the `overflow` field.
    ///
    #[must_use]
    pub fn overflow(&self) -> bool {
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
    /// `bit 1` - Read the `underflow` field.
    ///
    #[must_use]
    pub fn underflow(&self) -> bool {
        let start = 1;
        let end = 1;
        let raw = unsafe {
            ::device_driver::ops::load::<
                u8,
                ::device_driver::ops::LE,
            >(&self.bits, start, end)
        };
        raw > 0
    }
    /// `bit 0` - Set the `overflow` field.
    ///
    pub fn set_overflow(&mut self, value: bool) {
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
    /// `bit 1` - Set the `underflow` field.
    ///
    pub fn set_underflow(&mut self, value: bool) {
        let start = 1;
        let end = 1;
        let raw = value as _;
        unsafe {
            ::device_driver::ops::store::<
                u8,
                ::device_driver::ops::LE,
            >(raw, start, end, &mut self.bits)
        };
    }
}
impl Default for StatusFields {
    fn default() -> Self {
        <Self as ::device_driver::Fieldset>::ZERO
    }
}
impl From<[u8; 4]> for StatusFields {
    fn from(bits: [u8; 4]) -> Self {
        Self { bits }
    }
}
impl From<StatusFields> for [u8; 4] {
    fn from(val: StatusFields) -> Self {
        val.bits
    }
}
impl core::fmt::Debug for StatusFields {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> Result<(), core::fmt::Error> {
        let mut d = f.debug_struct("StatusFields");
        d.field("overflow", &self.overflow());
        d.field("underflow", &self.underflow());
        d.finish()
    }
}
#[cfg(feature = "defmt")]
impl defmt::Format for StatusFields {
    fn format(&self, f: defmt::Formatter) {
        defmt::write!(f, "StatusFields {{ ");
        defmt::write!(f, "overflow: {=bool}, ", & self.overflow());
        defmt::write!(f, "underflow: {=bool}, ", & self.underflow());
        defmt::write!(f, "}}");
    }
}
impl core::ops::BitAnd for StatusFields {
    type Output = Self;
    fn bitand(mut self, rhs: Self) -> Self::Output {
        self &= rhs;
        self
    }
}
impl core::ops::BitAndAssign for StatusFields {
    fn bitand_assign(&mut self, rhs: Self) {
        for (l, r) in self.bits.iter_mut().zip(&rhs.bits) {
            *l &= *r;
        }
    }
}
impl core::ops::BitOr for StatusFields {
    type Output = Self;
    fn bitor(mut self, rhs: Self) -> Self::Output {
        self |= rhs;
        self
    }
}
impl core::ops::BitOrAssign for StatusFields {
    fn bitor_assign(&mut self, rhs: Self) {
        for (l, r) in self.bits.iter_mut().zip(&rhs.bits) {
            *l |= *r;
        }
    }
}
impl core::ops::BitXor for StatusFields {
    type Output = Self;
    fn bitxor(mut self, rhs: Self) -> Self::Output {
        self ^= rhs;
        self
    }
}
impl core::ops::BitXorAssign for StatusFields {
    fn bitxor_assign(&mut self, rhs: Self) {
        for (l, r) in self.bits.iter_mut().zip(&rhs.bits) {
            *l ^= *r;
        }
    }
}
impl core::ops::Not for StatusFields {
    type Output = Self;
    fn not(mut self) -> Self::Output {
        for val in self.bits.iter_mut() {
            *val = !*val;
        }
        self
    }
}
/// Auto-synthesized intr_enable companion for IRQ group `root`.
#[doc(alias = "root_intr_enable_fields")]
#[derive(Copy, Clone, Eq, PartialEq)]
#[repr(transparent)]
pub struct RootIntrEnableFields {
    /// The internal bits
    bits: [u8; 4],
}
unsafe impl ::device_driver::Fieldset for RootIntrEnableFields {
    const METADATA: ::device_driver::FieldsetMetadata = ::device_driver::FieldsetMetadata::new()
        .with_byte_order(::device_driver::ByteOrder::LE);
    const ZERO: Self = Self { bits: [0; 4] };
}
impl RootIntrEnableFields {
    /// `bit 0` - Read the `overflow` field.
    ///
    /// Synthesized intr_enable bit for `overflow` in group `root`
    #[must_use]
    pub fn overflow(&self) -> bool {
        let start = 0;
        let end = 0;
        let raw = unsafe {
            ::device_driver::ops::load::<
                bool,
                ::device_driver::ops::LE,
            >(&self.bits, start, end)
        };
        raw > 0
    }
    /// `bit 3` - Read the `underflow` field.
    ///
    /// Synthesized intr_enable bit for `underflow` in group `root`
    #[must_use]
    pub fn underflow(&self) -> bool {
        let start = 3;
        let end = 3;
        let raw = unsafe {
            ::device_driver::ops::load::<
                bool,
                ::device_driver::ops::LE,
            >(&self.bits, start, end)
        };
        raw > 0
    }
    /// `bit 0` - Set the `overflow` field.
    ///
    /// Synthesized intr_enable bit for `overflow` in group `root`
    pub fn set_overflow(&mut self, value: bool) {
        let start = 0;
        let end = 0;
        let raw = value as _;
        unsafe {
            ::device_driver::ops::store::<
                bool,
                ::device_driver::ops::LE,
            >(raw, start, end, &mut self.bits)
        };
    }
    /// `bit 3` - Set the `underflow` field.
    ///
    /// Synthesized intr_enable bit for `underflow` in group `root`
    pub fn set_underflow(&mut self, value: bool) {
        let start = 3;
        let end = 3;
        let raw = value as _;
        unsafe {
            ::device_driver::ops::store::<
                bool,
                ::device_driver::ops::LE,
            >(raw, start, end, &mut self.bits)
        };
    }
}
impl Default for RootIntrEnableFields {
    fn default() -> Self {
        <Self as ::device_driver::Fieldset>::ZERO
    }
}
impl From<[u8; 4]> for RootIntrEnableFields {
    fn from(bits: [u8; 4]) -> Self {
        Self { bits }
    }
}
impl From<RootIntrEnableFields> for [u8; 4] {
    fn from(val: RootIntrEnableFields) -> Self {
        val.bits
    }
}
impl core::fmt::Debug for RootIntrEnableFields {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> Result<(), core::fmt::Error> {
        let mut d = f.debug_struct("RootIntrEnableFields");
        d.field("overflow", &self.overflow());
        d.field("underflow", &self.underflow());
        d.finish()
    }
}
#[cfg(feature = "defmt")]
impl defmt::Format for RootIntrEnableFields {
    fn format(&self, f: defmt::Formatter) {
        defmt::write!(f, "RootIntrEnableFields {{ ");
        defmt::write!(f, "overflow: {=bool}, ", & self.overflow());
        defmt::write!(f, "underflow: {=bool}, ", & self.underflow());
        defmt::write!(f, "}}");
    }
}
impl core::ops::BitAnd for RootIntrEnableFields {
    type Output = Self;
    fn bitand(mut self, rhs: Self) -> Self::Output {
        self &= rhs;
        self
    }
}
impl core::ops::BitAndAssign for RootIntrEnableFields {
    fn bitand_assign(&mut self, rhs: Self) {
        for (l, r) in self.bits.iter_mut().zip(&rhs.bits) {
            *l &= *r;
        }
    }
}
impl core::ops::BitOr for RootIntrEnableFields {
    type Output = Self;
    fn bitor(mut self, rhs: Self) -> Self::Output {
        self |= rhs;
        self
    }
}
impl core::ops::BitOrAssign for RootIntrEnableFields {
    fn bitor_assign(&mut self, rhs: Self) {
        for (l, r) in self.bits.iter_mut().zip(&rhs.bits) {
            *l |= *r;
        }
    }
}
impl core::ops::BitXor for RootIntrEnableFields {
    type Output = Self;
    fn bitxor(mut self, rhs: Self) -> Self::Output {
        self ^= rhs;
        self
    }
}
impl core::ops::BitXorAssign for RootIntrEnableFields {
    fn bitxor_assign(&mut self, rhs: Self) {
        for (l, r) in self.bits.iter_mut().zip(&rhs.bits) {
            *l ^= *r;
        }
    }
}
impl core::ops::Not for RootIntrEnableFields {
    type Output = Self;
    fn not(mut self) -> Self::Output {
        for val in self.bits.iter_mut() {
            *val = !*val;
        }
        self
    }
}
/// Auto-synthesized intr_mask companion for IRQ group `root`.
#[doc(alias = "root_intr_mask_fields")]
#[derive(Copy, Clone, Eq, PartialEq)]
#[repr(transparent)]
pub struct RootIntrMaskFields {
    /// The internal bits
    bits: [u8; 4],
}
unsafe impl ::device_driver::Fieldset for RootIntrMaskFields {
    const METADATA: ::device_driver::FieldsetMetadata = ::device_driver::FieldsetMetadata::new()
        .with_byte_order(::device_driver::ByteOrder::LE);
    const ZERO: Self = Self { bits: [0; 4] };
}
impl RootIntrMaskFields {
    /// `bit 0` - Read the `overflow` field.
    ///
    /// Synthesized intr_mask bit for `overflow` in group `root`
    #[must_use]
    pub fn overflow(&self) -> bool {
        let start = 0;
        let end = 0;
        let raw = unsafe {
            ::device_driver::ops::load::<
                bool,
                ::device_driver::ops::LE,
            >(&self.bits, start, end)
        };
        raw > 0
    }
    /// `bit 3` - Read the `underflow` field.
    ///
    /// Synthesized intr_mask bit for `underflow` in group `root`
    #[must_use]
    pub fn underflow(&self) -> bool {
        let start = 3;
        let end = 3;
        let raw = unsafe {
            ::device_driver::ops::load::<
                bool,
                ::device_driver::ops::LE,
            >(&self.bits, start, end)
        };
        raw > 0
    }
    /// `bit 0` - Set the `overflow` field.
    ///
    /// Synthesized intr_mask bit for `overflow` in group `root`
    pub fn set_overflow(&mut self, value: bool) {
        let start = 0;
        let end = 0;
        let raw = value as _;
        unsafe {
            ::device_driver::ops::store::<
                bool,
                ::device_driver::ops::LE,
            >(raw, start, end, &mut self.bits)
        };
    }
    /// `bit 3` - Set the `underflow` field.
    ///
    /// Synthesized intr_mask bit for `underflow` in group `root`
    pub fn set_underflow(&mut self, value: bool) {
        let start = 3;
        let end = 3;
        let raw = value as _;
        unsafe {
            ::device_driver::ops::store::<
                bool,
                ::device_driver::ops::LE,
            >(raw, start, end, &mut self.bits)
        };
    }
}
impl Default for RootIntrMaskFields {
    fn default() -> Self {
        <Self as ::device_driver::Fieldset>::ZERO
    }
}
impl From<[u8; 4]> for RootIntrMaskFields {
    fn from(bits: [u8; 4]) -> Self {
        Self { bits }
    }
}
impl From<RootIntrMaskFields> for [u8; 4] {
    fn from(val: RootIntrMaskFields) -> Self {
        val.bits
    }
}
impl core::fmt::Debug for RootIntrMaskFields {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> Result<(), core::fmt::Error> {
        let mut d = f.debug_struct("RootIntrMaskFields");
        d.field("overflow", &self.overflow());
        d.field("underflow", &self.underflow());
        d.finish()
    }
}
#[cfg(feature = "defmt")]
impl defmt::Format for RootIntrMaskFields {
    fn format(&self, f: defmt::Formatter) {
        defmt::write!(f, "RootIntrMaskFields {{ ");
        defmt::write!(f, "overflow: {=bool}, ", & self.overflow());
        defmt::write!(f, "underflow: {=bool}, ", & self.underflow());
        defmt::write!(f, "}}");
    }
}
impl core::ops::BitAnd for RootIntrMaskFields {
    type Output = Self;
    fn bitand(mut self, rhs: Self) -> Self::Output {
        self &= rhs;
        self
    }
}
impl core::ops::BitAndAssign for RootIntrMaskFields {
    fn bitand_assign(&mut self, rhs: Self) {
        for (l, r) in self.bits.iter_mut().zip(&rhs.bits) {
            *l &= *r;
        }
    }
}
impl core::ops::BitOr for RootIntrMaskFields {
    type Output = Self;
    fn bitor(mut self, rhs: Self) -> Self::Output {
        self |= rhs;
        self
    }
}
impl core::ops::BitOrAssign for RootIntrMaskFields {
    fn bitor_assign(&mut self, rhs: Self) {
        for (l, r) in self.bits.iter_mut().zip(&rhs.bits) {
            *l |= *r;
        }
    }
}
impl core::ops::BitXor for RootIntrMaskFields {
    type Output = Self;
    fn bitxor(mut self, rhs: Self) -> Self::Output {
        self ^= rhs;
        self
    }
}
impl core::ops::BitXorAssign for RootIntrMaskFields {
    fn bitxor_assign(&mut self, rhs: Self) {
        for (l, r) in self.bits.iter_mut().zip(&rhs.bits) {
            *l ^= *r;
        }
    }
}
impl core::ops::Not for RootIntrMaskFields {
    type Output = Self;
    fn not(mut self) -> Self::Output {
        for val in self.bits.iter_mut() {
            *val = !*val;
        }
        self
    }
}
