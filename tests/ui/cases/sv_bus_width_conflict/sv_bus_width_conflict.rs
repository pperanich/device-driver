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

/// Exercises the `bus_compat_checked` MIR pass — `sv-bus: ahblite`
/// combined with `sv-data-width: 64` is an error because the AHB-Lite
/// v1.1 wrapper template is fixed at 32-bit `HRDATA`/`HWDATA`. Pass
/// emits an `IncompatibleSvBusWidth` error.
///
/// `sv-bus: native` + `sv-data-width: 64` is a warning, not an error —
/// the native CPUIF is parameterized but few downstream interconnects
/// honor 64-bit. Both diagnostics should appear in the snapshot.
///
/// Root block of the SvBusWidthConflict driver
#[derive(Debug)]
pub struct SvBusWidthConflict<I> {
    pub(crate) interface: I,
    #[doc(hidden)]
    base_address: u8,
}
impl<I> SvBusWidthConflict<I> {
    /// Create a new instance of the block based on device interface
    pub const fn new(interface: I) -> Self {
        Self { interface, base_address: 0 }
    }
    #[doc(alias = "Foo")]
    pub fn foo(
        &mut self,
    ) -> ::device_driver::RegisterOperation<
        '_,
        Self,
        FooFields,
        u8,
        ::device_driver::RW,
        (),
    >
    where
        I: ::device_driver::RegisterInterfaceBase<AddressType = u8>,
    {
        let address = self.base_address + 0;
        ::device_driver::RegisterOperation::new(self, address as u8, FooFields::default)
    }
}
impl<I> ::device_driver::Block for SvBusWidthConflict<I> {
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
pub struct FooFields {
    /// The internal bits
    bits: [u8; 8],
}
unsafe impl ::device_driver::Fieldset for FooFields {
    const METADATA: ::device_driver::FieldsetMetadata = ::device_driver::FieldsetMetadata::new()
        .with_byte_order(::device_driver::ByteOrder::LE);
    const ZERO: Self = Self { bits: [0; 8] };
}
impl FooFields {
    /// `63:0` - Read the `value` field.
    ///
    #[must_use]
    pub fn value(&self) -> u64 {
        let start = 0;
        let end = 63;
        let raw = unsafe {
            ::device_driver::ops::load::<
                u64,
                ::device_driver::ops::LE,
            >(&self.bits, start, end)
        };
        raw
    }
    /// `63:0` - Set the `value` field.
    ///
    pub fn set_value(&mut self, value: u64) {
        let start = 0;
        let end = 63;
        let raw = value;
        unsafe {
            ::device_driver::ops::store::<
                u64,
                ::device_driver::ops::LE,
            >(raw, start, end, &mut self.bits)
        };
    }
}
impl Default for FooFields {
    fn default() -> Self {
        <Self as ::device_driver::Fieldset>::ZERO
    }
}
impl From<[u8; 8]> for FooFields {
    fn from(bits: [u8; 8]) -> Self {
        Self { bits }
    }
}
impl From<FooFields> for [u8; 8] {
    fn from(val: FooFields) -> Self {
        val.bits
    }
}
impl core::fmt::Debug for FooFields {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> Result<(), core::fmt::Error> {
        let mut d = f.debug_struct("FooFields");
        d.field("value", &self.value());
        d.finish()
    }
}
#[cfg(feature = "defmt")]
impl defmt::Format for FooFields {
    fn format(&self, f: defmt::Formatter) {
        defmt::write!(f, "FooFields {{ ");
        defmt::write!(f, "value: {=u64}, ", & self.value());
        defmt::write!(f, "}}");
    }
}
impl core::ops::BitAnd for FooFields {
    type Output = Self;
    fn bitand(mut self, rhs: Self) -> Self::Output {
        self &= rhs;
        self
    }
}
impl core::ops::BitAndAssign for FooFields {
    fn bitand_assign(&mut self, rhs: Self) {
        for (l, r) in self.bits.iter_mut().zip(&rhs.bits) {
            *l &= *r;
        }
    }
}
impl core::ops::BitOr for FooFields {
    type Output = Self;
    fn bitor(mut self, rhs: Self) -> Self::Output {
        self |= rhs;
        self
    }
}
impl core::ops::BitOrAssign for FooFields {
    fn bitor_assign(&mut self, rhs: Self) {
        for (l, r) in self.bits.iter_mut().zip(&rhs.bits) {
            *l |= *r;
        }
    }
}
impl core::ops::BitXor for FooFields {
    type Output = Self;
    fn bitxor(mut self, rhs: Self) -> Self::Output {
        self ^= rhs;
        self
    }
}
impl core::ops::BitXorAssign for FooFields {
    fn bitxor_assign(&mut self, rhs: Self) {
        for (l, r) in self.bits.iter_mut().zip(&rhs.bits) {
            *l ^= *r;
        }
    }
}
impl core::ops::Not for FooFields {
    type Output = Self;
    fn not(mut self) -> Self::Output {
        for val in self.bits.iter_mut() {
            *val = !*val;
        }
        self
    }
}
compile_error!("The device driver input has errors that need to be solved!");
