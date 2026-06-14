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

/// Exercises `sv-bus: axi4lite` device-level property — emits an AXI4-Lite
/// wrapper with separate W/R FSMs. Rust target ignores `sv-bus`.
///
/// Root block of the Axi4LiteBasic driver
#[derive(Debug)]
pub struct Axi4LiteBasic<I> {
    pub(crate) interface: I,
    #[doc(hidden)]
    base_address: u8,
}
impl<I> Axi4LiteBasic<I> {
    /// Create a new instance of the block based on device interface
    pub const fn new(interface: I) -> Self {
        Self { interface, base_address: 0 }
    }
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
impl<I> ::device_driver::Block for Axi4LiteBasic<I> {
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
    bits: [u8; 4],
}
unsafe impl ::device_driver::Fieldset for CtrlFields {
    const METADATA: ::device_driver::FieldsetMetadata = ::device_driver::FieldsetMetadata::new()
        .with_byte_order(::device_driver::ByteOrder::LE);
    const ZERO: Self = Self { bits: [0; 4] };
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
    /// `31:16` - Read the `count` field.
    ///
    #[must_use]
    pub fn count(&self) -> u16 {
        let start = 16;
        let end = 31;
        let raw = unsafe {
            ::device_driver::ops::load::<
                u16,
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
    /// `31:16` - Set the `count` field.
    ///
    pub fn set_count(&mut self, value: u16) {
        let start = 16;
        let end = 31;
        let raw = value;
        unsafe {
            ::device_driver::ops::store::<
                u16,
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
impl From<[u8; 4]> for CtrlFields {
    fn from(bits: [u8; 4]) -> Self {
        Self { bits }
    }
}
impl From<CtrlFields> for [u8; 4] {
    fn from(val: CtrlFields) -> Self {
        val.bits
    }
}
impl core::fmt::Debug for CtrlFields {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> Result<(), core::fmt::Error> {
        let mut d = f.debug_struct("CtrlFields");
        d.field("enable", &self.enable());
        d.field("count", &self.count());
        d.finish()
    }
}
#[cfg(feature = "defmt")]
impl defmt::Format for CtrlFields {
    fn format(&self, f: defmt::Formatter) {
        defmt::write!(f, "CtrlFields {{ ");
        defmt::write!(f, "enable: {=bool}, ", & self.enable());
        defmt::write!(f, "count: {=u16}, ", & self.count());
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
