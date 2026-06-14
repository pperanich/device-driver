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

/// Exercises advanced HW-side strobes and the singlepulse modifier on top of
/// the per-field always_ff cascade. Rust target ignores these attributes.
///
/// Root block of the AdvancedHwModifiers driver
#[derive(Debug)]
pub struct AdvancedHwModifiers<I> {
    pub(crate) interface: I,
    #[doc(hidden)]
    base_address: u8,
}
impl<I> AdvancedHwModifiers<I> {
    /// Create a new instance of the block based on device interface
    pub const fn new(interface: I) -> Self {
        Self { interface, base_address: 0 }
    }
    /// Mixed control + status register stressing the full cascade.
    #[doc(alias = "Mixed")]
    pub fn mixed(
        &mut self,
    ) -> ::device_driver::RegisterOperation<
        '_,
        Self,
        MixedFields,
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
            MixedFields::default,
        )
    }
}
impl<I> ::device_driver::Block for AdvancedHwModifiers<I> {
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
pub struct MixedFields {
    /// The internal bits
    bits: [u8; 1],
}
unsafe impl ::device_driver::Fieldset for MixedFields {
    const METADATA: ::device_driver::FieldsetMetadata = ::device_driver::FieldsetMetadata::new()
        .with_byte_order(::device_driver::ByteOrder::LE);
    const ZERO: Self = Self { bits: [0; 1] };
}
impl MixedFields {
    /// `bit 0` - Read the `alarm` field.
    ///
    /// Sticky alarm: HW pulses `hwset` when an event occurs, SW
    /// observes via read, SW W1C acknowledges and clears.
    #[must_use]
    pub fn alarm(&self) -> bool {
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
    /// `bit 1` - Read the `busy` field.
    ///
    /// HW-driven busy flag with an explicit HW clear strobe — HW can
    /// asynchronously force the flag low without re-reading storage.
    #[must_use]
    pub fn busy(&self) -> bool {
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
    /// `bit 2` - Read the `doorbell` field.
    ///
    /// Doorbell: SW writes 1 to fire a one-cycle pulse to HW, then the
    /// field auto-clears next cycle (single-cycle command strobe).
    #[must_use]
    pub fn doorbell(&self) -> bool {
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
    /// `7:4` - Read the `scratch` field.
    ///
    /// Plain SW RW for cascade-parity coverage.
    #[must_use]
    pub fn scratch(&self) -> u8 {
        let start = 4;
        let end = 7;
        let raw = unsafe {
            ::device_driver::ops::load::<
                u8,
                ::device_driver::ops::LE,
            >(&self.bits, start, end)
        };
        raw
    }
    /// `bit 0` - Set the `alarm` field.
    ///
    /// Sticky alarm: HW pulses `hwset` when an event occurs, SW
    /// observes via read, SW W1C acknowledges and clears.
    pub fn set_alarm(&mut self, value: bool) {
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
    /// `bit 1` - Set the `busy` field.
    ///
    /// HW-driven busy flag with an explicit HW clear strobe — HW can
    /// asynchronously force the flag low without re-reading storage.
    pub fn set_busy(&mut self, value: bool) {
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
    /// `bit 2` - Set the `doorbell` field.
    ///
    /// Doorbell: SW writes 1 to fire a one-cycle pulse to HW, then the
    /// field auto-clears next cycle (single-cycle command strobe).
    pub fn set_doorbell(&mut self, value: bool) {
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
    /// `7:4` - Set the `scratch` field.
    ///
    /// Plain SW RW for cascade-parity coverage.
    pub fn set_scratch(&mut self, value: u8) {
        let start = 4;
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
impl Default for MixedFields {
    fn default() -> Self {
        <Self as ::device_driver::Fieldset>::ZERO
    }
}
impl From<[u8; 1]> for MixedFields {
    fn from(bits: [u8; 1]) -> Self {
        Self { bits }
    }
}
impl From<MixedFields> for [u8; 1] {
    fn from(val: MixedFields) -> Self {
        val.bits
    }
}
impl core::fmt::Debug for MixedFields {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> Result<(), core::fmt::Error> {
        let mut d = f.debug_struct("MixedFields");
        d.field("alarm", &self.alarm());
        d.field("busy", &self.busy());
        d.field("doorbell", &self.doorbell());
        d.field("scratch", &self.scratch());
        d.finish()
    }
}
#[cfg(feature = "defmt")]
impl defmt::Format for MixedFields {
    fn format(&self, f: defmt::Formatter) {
        defmt::write!(f, "MixedFields {{ ");
        defmt::write!(f, "alarm: {=bool}, ", & self.alarm());
        defmt::write!(f, "busy: {=bool}, ", & self.busy());
        defmt::write!(f, "doorbell: {=bool}, ", & self.doorbell());
        defmt::write!(f, "scratch: {=u8}, ", & self.scratch());
        defmt::write!(f, "}}");
    }
}
impl core::ops::BitAnd for MixedFields {
    type Output = Self;
    fn bitand(mut self, rhs: Self) -> Self::Output {
        self &= rhs;
        self
    }
}
impl core::ops::BitAndAssign for MixedFields {
    fn bitand_assign(&mut self, rhs: Self) {
        for (l, r) in self.bits.iter_mut().zip(&rhs.bits) {
            *l &= *r;
        }
    }
}
impl core::ops::BitOr for MixedFields {
    type Output = Self;
    fn bitor(mut self, rhs: Self) -> Self::Output {
        self |= rhs;
        self
    }
}
impl core::ops::BitOrAssign for MixedFields {
    fn bitor_assign(&mut self, rhs: Self) {
        for (l, r) in self.bits.iter_mut().zip(&rhs.bits) {
            *l |= *r;
        }
    }
}
impl core::ops::BitXor for MixedFields {
    type Output = Self;
    fn bitxor(mut self, rhs: Self) -> Self::Output {
        self ^= rhs;
        self
    }
}
impl core::ops::BitXorAssign for MixedFields {
    fn bitxor_assign(&mut self, rhs: Self) {
        for (l, r) in self.bits.iter_mut().zip(&rhs.bits) {
            *l ^= *r;
        }
    }
}
impl core::ops::Not for MixedFields {
    type Output = Self;
    fn not(mut self) -> Self::Output {
        for val in self.bits.iter_mut() {
            *val = !*val;
        }
        self
    }
}
