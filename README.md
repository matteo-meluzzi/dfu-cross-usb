dfu-cross-usb
=============

A Rust crate for performing USB Device Firmware Update (DFU) operations based on the [`cross_usb`](https://crates.io/crates/cross_usb) crate.

**Note: This crate currently only works for WASM targets.**

## Overview

This crate provides an implementation of the USB DFU protocol that works in web browsers through WebAssembly. It depends on:

- [`cross_usb`](https://crates.io/crates/cross_usb) - Cross-platform USB library with WASM support
- [`dfu-core`](https://crates.io/crates/dfu-core) - Core DFU protocol implementation

## Usage

Add this to your `Cargo.toml`:

```toml
[dependencies]
dfu-cross-usb = "0.1.0"
```

This crate is designed to work in web browsers where you can perform DFU updates on USB devices through the WebUSB API.

## Target Support

- ✅ `wasm32-unknown-unknown` - Primary target for web browsers
- ❌ Native targets - Not currently supported

## License

MIT OR Apache-2.0