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

/// Exercises `hw-kind: fifo` on a buffer — the SV target emits bus-side
/// push/pop strobes + wdata to user RTL and routes user-provided rdata
/// back through the read mux. The regblock does NOT contain FIFO storage
/// in v1. Rust target ignores `hw-kind` / `depth`.
///
/// Root block of the BufferFifo driver
#[derive(Debug)]
pub struct BufferFifo<I> {
    pub(crate) interface: I,
    #[doc(hidden)]
    base_address: u8,
}
impl<I> BufferFifo<I> {
    /// Create a new instance of the block based on device interface
    pub const fn new(interface: I) -> Self {
        Self { interface, base_address: 0 }
    }
    #[doc(alias = "DataFifo")]
    pub fn data_fifo(
        &mut self,
    ) -> ::device_driver::BufferOperation<'_, Self, u8, ::device_driver::RW>
    where
        I: ::device_driver::BufferInterfaceBase<AddressType = u8>,
    {
        let address = self.base_address + 16;
        ::device_driver::BufferOperation::new(self, address as u8)
    }
}
impl<I> ::device_driver::Block for BufferFifo<I> {
    type Interface = I;
    type RegisterAddressType = u8;
    type CommandAddressType = u8;
    type BufferAddressType = u8;
    type RegisterAddressMode = ();
    fn interface(&mut self) -> &mut Self::Interface {
        &mut self.interface
    }
}
