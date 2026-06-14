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

/// Exercises the field-level `on-read` modifier (read-clear / read-set)
/// landing in the SystemVerilog target. Rust target ignores it.
///
/// Root block of the OnReadModifiers driver
#[derive(Debug)]
pub struct OnReadModifiers<I> {
    pub(crate) interface: I,
    #[doc(hidden)]
    base_address: u8,
}
impl<I> OnReadModifiers<I> {
    /// Create a new instance of the block based on device interface
    pub const fn new(interface: I) -> Self {
        Self { interface, base_address: 0 }
    }
    /// Status register: a mix of plain, read-clear and read-set fields.
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
}
impl<I> ::device_driver::Block for OnReadModifiers<I> {
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
    bits: [u8; 1],
}
unsafe impl ::device_driver::Fieldset for StatusFields {
    const METADATA: ::device_driver::FieldsetMetadata = ::device_driver::FieldsetMetadata::new()
        .with_byte_order(::device_driver::ByteOrder::LE);
    const ZERO: Self = Self { bits: [0; 1] };
}
impl StatusFields {
    /// `bit 0` - Read the `flag` field.
    ///
    /// Plain (default) — read is purely combinational.
    #[must_use]
    pub fn flag(&self) -> bool {
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
    /// `bit 1` - Read the `sticky_overflow` field.
    ///
    /// Read-clear (RC): bit clears after SW reads the register.
    #[must_use]
    pub fn sticky_overflow(&self) -> bool {
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
    /// `bit 2` - Read the `self_arming` field.
    ///
    /// Read-set (RS): bit sets after SW reads the register.
    #[must_use]
    pub fn self_arming(&self) -> bool {
        let start = 2;
        let end = 2;
        let raw = unsafe {
            ::device_driver::ops::load::<
                u8,
                ::device_driver::ops::LE,
            >(&self.bits, start, end)
        };
        raw > 0
    }
    /// `bit 3` - Read the `combo` field.
    ///
    /// Combined with on-write: W1C and RC on the same field.
    #[must_use]
    pub fn combo(&self) -> bool {
        let start = 3;
        let end = 3;
        let raw = unsafe {
            ::device_driver::ops::load::<
                u8,
                ::device_driver::ops::LE,
            >(&self.bits, start, end)
        };
        raw > 0
    }
    /// `bit 0` - Set the `flag` field.
    ///
    /// Plain (default) — read is purely combinational.
    pub fn set_flag(&mut self, value: bool) {
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
    /// `bit 1` - Set the `sticky_overflow` field.
    ///
    /// Read-clear (RC): bit clears after SW reads the register.
    pub fn set_sticky_overflow(&mut self, value: bool) {
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
    /// `bit 2` - Set the `self_arming` field.
    ///
    /// Read-set (RS): bit sets after SW reads the register.
    pub fn set_self_arming(&mut self, value: bool) {
        let start = 2;
        let end = 2;
        let raw = value as _;
        unsafe {
            ::device_driver::ops::store::<
                u8,
                ::device_driver::ops::LE,
            >(raw, start, end, &mut self.bits)
        };
    }
    /// `bit 3` - Set the `combo` field.
    ///
    /// Combined with on-write: W1C and RC on the same field.
    pub fn set_combo(&mut self, value: bool) {
        let start = 3;
        let end = 3;
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
impl From<[u8; 1]> for StatusFields {
    fn from(bits: [u8; 1]) -> Self {
        Self { bits }
    }
}
impl From<StatusFields> for [u8; 1] {
    fn from(val: StatusFields) -> Self {
        val.bits
    }
}
impl core::fmt::Debug for StatusFields {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> Result<(), core::fmt::Error> {
        let mut d = f.debug_struct("StatusFields");
        d.field("flag", &self.flag());
        d.field("sticky_overflow", &self.sticky_overflow());
        d.field("self_arming", &self.self_arming());
        d.field("combo", &self.combo());
        d.finish()
    }
}
#[cfg(feature = "defmt")]
impl defmt::Format for StatusFields {
    fn format(&self, f: defmt::Formatter) {
        defmt::write!(f, "StatusFields {{ ");
        defmt::write!(f, "flag: {=bool}, ", & self.flag());
        defmt::write!(f, "sticky_overflow: {=bool}, ", & self.sticky_overflow());
        defmt::write!(f, "self_arming: {=bool}, ", & self.self_arming());
        defmt::write!(f, "combo: {=bool}, ", & self.combo());
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
