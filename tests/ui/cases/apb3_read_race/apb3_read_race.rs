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

/// Stress case for the APB3 wrapper read path. Three registers at
/// distinct addresses with distinct content patterns lets the TB walk
/// back-to-back reads, insert idle cycles, and exercise read-after-write
/// — all paths that would have returned `'0` before
/// `cpuif_rd_data` was registered (the wrapper drops `cpuif_req` when
/// pready rises, which collapses the combinational read mux on the
/// same cycle the master samples prdata).
///
/// Root block of the Apb3ReadRace driver
#[derive(Debug)]
pub struct Apb3ReadRace<I> {
    pub(crate) interface: I,
    #[doc(hidden)]
    base_address: u8,
}
impl<I> Apb3ReadRace<I> {
    /// Create a new instance of the block based on device interface
    pub const fn new(interface: I) -> Self {
        Self { interface, base_address: 0 }
    }
    ///
    /// Reset value: `0xAAAA_AAAA`
    #[doc(alias = "RegA")]
    pub fn reg_a(
        &mut self,
    ) -> ::device_driver::RegisterOperation<
        '_,
        Self,
        RegAFields,
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
            || RegAFields::from([170, 170, 170, 170]),
        )
    }
    ///
    /// Reset value: `0xBBBB_BBBB`
    #[doc(alias = "RegB")]
    pub fn reg_b(
        &mut self,
    ) -> ::device_driver::RegisterOperation<
        '_,
        Self,
        RegBFields,
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
            || RegBFields::from([187, 187, 187, 187]),
        )
    }
    ///
    /// Reset value: `0xCCCC_CCCC`
    #[doc(alias = "RegC")]
    pub fn reg_c(
        &mut self,
    ) -> ::device_driver::RegisterOperation<
        '_,
        Self,
        RegCFields,
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
            || RegCFields::from([204, 204, 204, 204]),
        )
    }
}
impl<I> ::device_driver::Block for Apb3ReadRace<I> {
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
pub struct RegCFields {
    /// The internal bits
    bits: [u8; 4],
}
unsafe impl ::device_driver::Fieldset for RegCFields {
    const METADATA: ::device_driver::FieldsetMetadata = ::device_driver::FieldsetMetadata::new()
        .with_byte_order(::device_driver::ByteOrder::LE);
    const ZERO: Self = Self { bits: [0; 4] };
}
impl RegCFields {
    /// `31:0` - Read the `value` field.
    ///
    #[must_use]
    pub fn value(&self) -> u32 {
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
    /// `31:0` - Set the `value` field.
    ///
    pub fn set_value(&mut self, value: u32) {
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
impl Default for RegCFields {
    fn default() -> Self {
        <Self as ::device_driver::Fieldset>::ZERO
    }
}
impl From<[u8; 4]> for RegCFields {
    fn from(bits: [u8; 4]) -> Self {
        Self { bits }
    }
}
impl From<RegCFields> for [u8; 4] {
    fn from(val: RegCFields) -> Self {
        val.bits
    }
}
impl core::fmt::Debug for RegCFields {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> Result<(), core::fmt::Error> {
        let mut d = f.debug_struct("RegCFields");
        d.field("value", &self.value());
        d.finish()
    }
}
#[cfg(feature = "defmt")]
impl defmt::Format for RegCFields {
    fn format(&self, f: defmt::Formatter) {
        defmt::write!(f, "RegCFields {{ ");
        defmt::write!(f, "value: {=u32}, ", & self.value());
        defmt::write!(f, "}}");
    }
}
impl core::ops::BitAnd for RegCFields {
    type Output = Self;
    fn bitand(mut self, rhs: Self) -> Self::Output {
        self &= rhs;
        self
    }
}
impl core::ops::BitAndAssign for RegCFields {
    fn bitand_assign(&mut self, rhs: Self) {
        for (l, r) in self.bits.iter_mut().zip(&rhs.bits) {
            *l &= *r;
        }
    }
}
impl core::ops::BitOr for RegCFields {
    type Output = Self;
    fn bitor(mut self, rhs: Self) -> Self::Output {
        self |= rhs;
        self
    }
}
impl core::ops::BitOrAssign for RegCFields {
    fn bitor_assign(&mut self, rhs: Self) {
        for (l, r) in self.bits.iter_mut().zip(&rhs.bits) {
            *l |= *r;
        }
    }
}
impl core::ops::BitXor for RegCFields {
    type Output = Self;
    fn bitxor(mut self, rhs: Self) -> Self::Output {
        self ^= rhs;
        self
    }
}
impl core::ops::BitXorAssign for RegCFields {
    fn bitxor_assign(&mut self, rhs: Self) {
        for (l, r) in self.bits.iter_mut().zip(&rhs.bits) {
            *l ^= *r;
        }
    }
}
impl core::ops::Not for RegCFields {
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
pub struct RegBFields {
    /// The internal bits
    bits: [u8; 4],
}
unsafe impl ::device_driver::Fieldset for RegBFields {
    const METADATA: ::device_driver::FieldsetMetadata = ::device_driver::FieldsetMetadata::new()
        .with_byte_order(::device_driver::ByteOrder::LE);
    const ZERO: Self = Self { bits: [0; 4] };
}
impl RegBFields {
    /// `31:0` - Read the `value` field.
    ///
    #[must_use]
    pub fn value(&self) -> u32 {
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
    /// `31:0` - Set the `value` field.
    ///
    pub fn set_value(&mut self, value: u32) {
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
impl Default for RegBFields {
    fn default() -> Self {
        <Self as ::device_driver::Fieldset>::ZERO
    }
}
impl From<[u8; 4]> for RegBFields {
    fn from(bits: [u8; 4]) -> Self {
        Self { bits }
    }
}
impl From<RegBFields> for [u8; 4] {
    fn from(val: RegBFields) -> Self {
        val.bits
    }
}
impl core::fmt::Debug for RegBFields {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> Result<(), core::fmt::Error> {
        let mut d = f.debug_struct("RegBFields");
        d.field("value", &self.value());
        d.finish()
    }
}
#[cfg(feature = "defmt")]
impl defmt::Format for RegBFields {
    fn format(&self, f: defmt::Formatter) {
        defmt::write!(f, "RegBFields {{ ");
        defmt::write!(f, "value: {=u32}, ", & self.value());
        defmt::write!(f, "}}");
    }
}
impl core::ops::BitAnd for RegBFields {
    type Output = Self;
    fn bitand(mut self, rhs: Self) -> Self::Output {
        self &= rhs;
        self
    }
}
impl core::ops::BitAndAssign for RegBFields {
    fn bitand_assign(&mut self, rhs: Self) {
        for (l, r) in self.bits.iter_mut().zip(&rhs.bits) {
            *l &= *r;
        }
    }
}
impl core::ops::BitOr for RegBFields {
    type Output = Self;
    fn bitor(mut self, rhs: Self) -> Self::Output {
        self |= rhs;
        self
    }
}
impl core::ops::BitOrAssign for RegBFields {
    fn bitor_assign(&mut self, rhs: Self) {
        for (l, r) in self.bits.iter_mut().zip(&rhs.bits) {
            *l |= *r;
        }
    }
}
impl core::ops::BitXor for RegBFields {
    type Output = Self;
    fn bitxor(mut self, rhs: Self) -> Self::Output {
        self ^= rhs;
        self
    }
}
impl core::ops::BitXorAssign for RegBFields {
    fn bitxor_assign(&mut self, rhs: Self) {
        for (l, r) in self.bits.iter_mut().zip(&rhs.bits) {
            *l ^= *r;
        }
    }
}
impl core::ops::Not for RegBFields {
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
pub struct RegAFields {
    /// The internal bits
    bits: [u8; 4],
}
unsafe impl ::device_driver::Fieldset for RegAFields {
    const METADATA: ::device_driver::FieldsetMetadata = ::device_driver::FieldsetMetadata::new()
        .with_byte_order(::device_driver::ByteOrder::LE);
    const ZERO: Self = Self { bits: [0; 4] };
}
impl RegAFields {
    /// `31:0` - Read the `value` field.
    ///
    #[must_use]
    pub fn value(&self) -> u32 {
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
    /// `31:0` - Set the `value` field.
    ///
    pub fn set_value(&mut self, value: u32) {
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
impl Default for RegAFields {
    fn default() -> Self {
        <Self as ::device_driver::Fieldset>::ZERO
    }
}
impl From<[u8; 4]> for RegAFields {
    fn from(bits: [u8; 4]) -> Self {
        Self { bits }
    }
}
impl From<RegAFields> for [u8; 4] {
    fn from(val: RegAFields) -> Self {
        val.bits
    }
}
impl core::fmt::Debug for RegAFields {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> Result<(), core::fmt::Error> {
        let mut d = f.debug_struct("RegAFields");
        d.field("value", &self.value());
        d.finish()
    }
}
#[cfg(feature = "defmt")]
impl defmt::Format for RegAFields {
    fn format(&self, f: defmt::Formatter) {
        defmt::write!(f, "RegAFields {{ ");
        defmt::write!(f, "value: {=u32}, ", & self.value());
        defmt::write!(f, "}}");
    }
}
impl core::ops::BitAnd for RegAFields {
    type Output = Self;
    fn bitand(mut self, rhs: Self) -> Self::Output {
        self &= rhs;
        self
    }
}
impl core::ops::BitAndAssign for RegAFields {
    fn bitand_assign(&mut self, rhs: Self) {
        for (l, r) in self.bits.iter_mut().zip(&rhs.bits) {
            *l &= *r;
        }
    }
}
impl core::ops::BitOr for RegAFields {
    type Output = Self;
    fn bitor(mut self, rhs: Self) -> Self::Output {
        self |= rhs;
        self
    }
}
impl core::ops::BitOrAssign for RegAFields {
    fn bitor_assign(&mut self, rhs: Self) {
        for (l, r) in self.bits.iter_mut().zip(&rhs.bits) {
            *l |= *r;
        }
    }
}
impl core::ops::BitXor for RegAFields {
    type Output = Self;
    fn bitxor(mut self, rhs: Self) -> Self::Output {
        self ^= rhs;
        self
    }
}
impl core::ops::BitXorAssign for RegAFields {
    fn bitxor_assign(&mut self, rhs: Self) {
        for (l, r) in self.bits.iter_mut().zip(&rhs.bits) {
            *l ^= *r;
        }
    }
}
impl core::ops::Not for RegAFields {
    type Output = Self;
    fn not(mut self) -> Self::Output {
        for val in self.bits.iter_mut() {
            *val = !*val;
        }
        self
    }
}
