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

/// Exercises `hw-access` and the auto-generated `hwif_in` port + per-field
/// always_ff cascade in the SystemVerilog target. The Rust target ignores
/// the HW-side attribute.
///
/// Root block of the HwWritableStatus driver
#[derive(Debug)]
pub struct HwWritableStatus<I> {
    pub(crate) interface: I,
    #[doc(hidden)]
    base_address: u8,
}
impl<I> HwWritableStatus<I> {
    /// Create a new instance of the block based on device interface
    pub const fn new(interface: I) -> Self {
        Self { interface, base_address: 0 }
    }
    /// Status register: HW updates the counter and overflow flag continuously,
    /// SW can clear the overflow flag (W1C) and observe both, plus a SW-only
    /// scratch field for parity testing of the cascade.
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
impl<I> ::device_driver::Block for HwWritableStatus<I> {
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
    bits: [u8; 2],
}
unsafe impl ::device_driver::Fieldset for StatusFields {
    const METADATA: ::device_driver::FieldsetMetadata = ::device_driver::FieldsetMetadata::new()
        .with_byte_order(::device_driver::ByteOrder::LE);
    const ZERO: Self = Self { bits: [0; 2] };
}
impl StatusFields {
    /// `7:0` - Read the `counter` field.
    ///
    /// HW updates the counter (RW), SW can read it. SW writes are
    /// allowed but lose to HW on the same cycle (HW > SW precedence).
    #[must_use]
    pub fn counter(&self) -> u8 {
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
    /// `bit 8` - Read the `overflow` field.
    ///
    /// Sticky overflow latched by HW, cleared by SW W1C.
    #[must_use]
    pub fn overflow(&self) -> bool {
        let start = 8;
        let end = 8;
        let raw = unsafe {
            ::device_driver::ops::load::<
                u8,
                ::device_driver::ops::LE,
            >(&self.bits, start, end)
        };
        raw > 0
    }
    /// `bit 9` - Read the `hw_armed` field.
    ///
    /// HW-driven status (HW writes, SW reads); SW cannot modify.
    #[must_use]
    pub fn hw_armed(&self) -> bool {
        let start = 9;
        let end = 9;
        let raw = unsafe {
            ::device_driver::ops::load::<
                u8,
                ::device_driver::ops::LE,
            >(&self.bits, start, end)
        };
        raw > 0
    }
    /// `15:12` - Read the `scratch` field.
    ///
    /// SW-only scratch field — no HW interaction.
    #[must_use]
    pub fn scratch(&self) -> u8 {
        let start = 12;
        let end = 15;
        let raw = unsafe {
            ::device_driver::ops::load::<
                u8,
                ::device_driver::ops::LE,
            >(&self.bits, start, end)
        };
        raw
    }
    /// `7:0` - Set the `counter` field.
    ///
    /// HW updates the counter (RW), SW can read it. SW writes are
    /// allowed but lose to HW on the same cycle (HW > SW precedence).
    pub fn set_counter(&mut self, value: u8) {
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
    /// `bit 8` - Set the `overflow` field.
    ///
    /// Sticky overflow latched by HW, cleared by SW W1C.
    pub fn set_overflow(&mut self, value: bool) {
        let start = 8;
        let end = 8;
        let raw = value as _;
        unsafe {
            ::device_driver::ops::store::<
                u8,
                ::device_driver::ops::LE,
            >(raw, start, end, &mut self.bits)
        };
    }
    /// `15:12` - Set the `scratch` field.
    ///
    /// SW-only scratch field — no HW interaction.
    pub fn set_scratch(&mut self, value: u8) {
        let start = 12;
        let end = 15;
        let raw = value;
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
impl From<[u8; 2]> for StatusFields {
    fn from(bits: [u8; 2]) -> Self {
        Self { bits }
    }
}
impl From<StatusFields> for [u8; 2] {
    fn from(val: StatusFields) -> Self {
        val.bits
    }
}
impl core::fmt::Debug for StatusFields {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> Result<(), core::fmt::Error> {
        let mut d = f.debug_struct("StatusFields");
        d.field("counter", &self.counter());
        d.field("overflow", &self.overflow());
        d.field("hw_armed", &self.hw_armed());
        d.field("scratch", &self.scratch());
        d.finish()
    }
}
#[cfg(feature = "defmt")]
impl defmt::Format for StatusFields {
    fn format(&self, f: defmt::Formatter) {
        defmt::write!(f, "StatusFields {{ ");
        defmt::write!(f, "counter: {=u8}, ", & self.counter());
        defmt::write!(f, "overflow: {=bool}, ", & self.overflow());
        defmt::write!(f, "hw_armed: {=bool}, ", & self.hw_armed());
        defmt::write!(f, "scratch: {=u8}, ", & self.scratch());
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
