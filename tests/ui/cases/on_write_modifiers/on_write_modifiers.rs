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

/// Exercises the field-level `on-write` modifier (W1C / W1S / W1T / store)
/// landing in the SystemVerilog target. The Rust target ignores the modifier.
///
/// Root block of the OnWriteModifiers driver
#[derive(Debug)]
pub struct OnWriteModifiers<I> {
    pub(crate) interface: I,
    #[doc(hidden)]
    base_address: u8,
}
impl<I> OnWriteModifiers<I> {
    /// Create a new instance of the block based on device interface
    pub const fn new(interface: I) -> Self {
        Self { interface, base_address: 0 }
    }
    /// Status register: every field demonstrates a different write semantic.
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
impl<I> ::device_driver::Block for OnWriteModifiers<I> {
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
    /// `bit 0` - Read the `plain` field.
    ///
    /// Standard mask-merge write (default; matches plain RW).
    #[must_use]
    pub fn plain(&self) -> bool {
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
    /// `bit 1` - Read the `overflow` field.
    ///
    /// Write-1-to-clear: writing `1` clears the bit, writing `0` is a no-op.
    #[must_use]
    pub fn overflow(&self) -> bool {
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
    /// `bit 2` - Read the `arm` field.
    ///
    /// Write-1-to-set: writing `1` sets the bit, writing `0` is a no-op.
    #[must_use]
    pub fn arm(&self) -> bool {
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
    /// `bit 3` - Read the `flip` field.
    ///
    /// Write-1-to-toggle: writing `1` flips the bit, writing `0` is a no-op.
    #[must_use]
    pub fn flip(&self) -> bool {
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
    /// `bit 0` - Set the `plain` field.
    ///
    /// Standard mask-merge write (default; matches plain RW).
    pub fn set_plain(&mut self, value: bool) {
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
    /// `bit 1` - Set the `overflow` field.
    ///
    /// Write-1-to-clear: writing `1` clears the bit, writing `0` is a no-op.
    pub fn set_overflow(&mut self, value: bool) {
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
    /// `bit 2` - Set the `arm` field.
    ///
    /// Write-1-to-set: writing `1` sets the bit, writing `0` is a no-op.
    pub fn set_arm(&mut self, value: bool) {
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
    /// `bit 3` - Set the `flip` field.
    ///
    /// Write-1-to-toggle: writing `1` flips the bit, writing `0` is a no-op.
    pub fn set_flip(&mut self, value: bool) {
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
        d.field("plain", &self.plain());
        d.field("overflow", &self.overflow());
        d.field("arm", &self.arm());
        d.field("flip", &self.flip());
        d.finish()
    }
}
#[cfg(feature = "defmt")]
impl defmt::Format for StatusFields {
    fn format(&self, f: defmt::Formatter) {
        defmt::write!(f, "StatusFields {{ ");
        defmt::write!(f, "plain: {=bool}, ", & self.plain());
        defmt::write!(f, "overflow: {=bool}, ", & self.overflow());
        defmt::write!(f, "arm: {=bool}, ", & self.arm());
        defmt::write!(f, "flip: {=bool}, ", & self.flip());
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
