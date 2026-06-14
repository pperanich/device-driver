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

/// Exercises `sv-bus: apb3` device-level property — requests the SystemVerilog
/// target emit an APB3 wrapper module around the regs CPUIF. Rust target
/// ignores `sv-bus`.
///
/// Root block of the Apb3Basic driver
#[derive(Debug)]
pub struct Apb3Basic<I> {
    pub(crate) interface: I,
    #[doc(hidden)]
    base_address: u8,
}
impl<I> Apb3Basic<I> {
    /// Create a new instance of the block based on device interface
    pub const fn new(interface: I) -> Self {
        Self { interface, base_address: 0 }
    }
    /// Plain RW register, just enough to validate the wrapper signal path.
    #[doc(alias = "Ctrl")]
    pub fn ctrl(
        &mut self,
    ) -> ::device_driver::RegisterOperation<
        '_,
        Self,
        CtrlFields,
        u8,
        ::device_driver::RW,
        (),
    >
    where
        I: ::device_driver::RegisterInterfaceBase<AddressType = u8>,
    {
        let address = self.base_address + 0;
        ::device_driver::RegisterOperation::new(self, address as u8, CtrlFields::default)
    }
}
impl<I> ::device_driver::Block for Apb3Basic<I> {
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
pub struct CtrlFields {
    /// The internal bits
    bits: [u8; 1],
}
unsafe impl ::device_driver::Fieldset for CtrlFields {
    const METADATA: ::device_driver::FieldsetMetadata = ::device_driver::FieldsetMetadata::new()
        .with_byte_order(::device_driver::ByteOrder::LE);
    const ZERO: Self = Self { bits: [0; 1] };
}
impl CtrlFields {
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
    /// `3:1` - Read the `mode` field.
    ///
    #[must_use]
    pub fn mode(&self) -> u8 {
        let start = 1;
        let end = 3;
        let raw = unsafe {
            ::device_driver::ops::load::<
                u8,
                ::device_driver::ops::LE,
            >(&self.bits, start, end)
        };
        raw
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
    /// `3:1` - Set the `mode` field.
    ///
    pub fn set_mode(&mut self, value: u8) {
        let start = 1;
        let end = 3;
        let raw = value;
        unsafe {
            ::device_driver::ops::store::<
                u8,
                ::device_driver::ops::LE,
            >(raw, start, end, &mut self.bits)
        };
    }
}
impl Default for CtrlFields {
    fn default() -> Self {
        <Self as ::device_driver::Fieldset>::ZERO
    }
}
impl From<[u8; 1]> for CtrlFields {
    fn from(bits: [u8; 1]) -> Self {
        Self { bits }
    }
}
impl From<CtrlFields> for [u8; 1] {
    fn from(val: CtrlFields) -> Self {
        val.bits
    }
}
impl core::fmt::Debug for CtrlFields {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> Result<(), core::fmt::Error> {
        let mut d = f.debug_struct("CtrlFields");
        d.field("enable", &self.enable());
        d.field("mode", &self.mode());
        d.finish()
    }
}
#[cfg(feature = "defmt")]
impl defmt::Format for CtrlFields {
    fn format(&self, f: defmt::Formatter) {
        defmt::write!(f, "CtrlFields {{ ");
        defmt::write!(f, "enable: {=bool}, ", & self.enable());
        defmt::write!(f, "mode: {=u8}, ", & self.mode());
        defmt::write!(f, "}}");
    }
}
impl core::ops::BitAnd for CtrlFields {
    type Output = Self;
    fn bitand(mut self, rhs: Self) -> Self::Output {
        self &= rhs;
        self
    }
}
impl core::ops::BitAndAssign for CtrlFields {
    fn bitand_assign(&mut self, rhs: Self) {
        for (l, r) in self.bits.iter_mut().zip(&rhs.bits) {
            *l &= *r;
        }
    }
}
impl core::ops::BitOr for CtrlFields {
    type Output = Self;
    fn bitor(mut self, rhs: Self) -> Self::Output {
        self |= rhs;
        self
    }
}
impl core::ops::BitOrAssign for CtrlFields {
    fn bitor_assign(&mut self, rhs: Self) {
        for (l, r) in self.bits.iter_mut().zip(&rhs.bits) {
            *l |= *r;
        }
    }
}
impl core::ops::BitXor for CtrlFields {
    type Output = Self;
    fn bitxor(mut self, rhs: Self) -> Self::Output {
        self ^= rhs;
        self
    }
}
impl core::ops::BitXorAssign for CtrlFields {
    fn bitxor_assign(&mut self, rhs: Self) {
        for (l, r) in self.bits.iter_mut().zip(&rhs.bits) {
            *l ^= *r;
        }
    }
}
impl core::ops::Not for CtrlFields {
    type Output = Self;
    fn not(mut self) -> Self::Output {
        for val in self.bits.iter_mut() {
            *val = !*val;
        }
        self
    }
}
