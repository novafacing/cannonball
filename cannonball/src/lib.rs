//! Cannonball! 💣
//!
//! This library provides a Rust APi for writing QEMU plugins. It does *not* provide a safe
//! API for doing so (yet!), and instead aims to provide a thin wrapper around the QEMU plugin
//! API. This allows for writing plugins in Rust without having to write any C code!
//!
//! This allows very cool things like creating a plugin that can be loaded into QEMU (which
//! can be installed as a crate with the [qemu](https://crates.io/crates/qemu) crate) and
//! run scalably, all from pure Rust code. For an example plugin and driver binary, see
//! [Jaivana](https://github.com/novafacing/cannonball/tree/examples/jaivana), an example
//! putting together:
//!
//! - [Cannonball](https://crates.io/crates/cannonball) - This plugin API
//! - [QEMU](https://crates.io/crates/qemu) - A crate for installing and running QEMU
//! - [Memfd-exec](https://crates.io/crates/memfd-exec) - A crate for executing binaries in memory

#![allow(non_upper_case_globals)]

use libc::c_int;

pub mod api;
pub mod args;
pub mod callbacks;
pub mod install;

use api::QEMU_PLUGIN_VERSION;

#[no_mangle]
/// QEMU requires the API version to be exported as a global symbol. This symbol is checked
/// before loading the plugin.
pub static qemu_plugin_version: c_int = QEMU_PLUGIN_VERSION as c_int;
