//! Cannonball!
//!
//! This library provides a Rust APi for writing QEMU plugins. It does *not* provide a safe
//! API for doing so (yet!), and instead aims to provide a thin wrapper around the QEMU plugin
//! API. This allows for writing plugins in Rust without having to write any C code!
//!
//! This allows very cool things like creating a plugin that can be loaded into QEMU (which
//! can be installed as a crate with the [qemu](https://crates.io/crates/qemu) crate) and
//! run scalably, all from pure Rust code.

#![allow(non_upper_case_globals)]

use libc::c_int;

pub mod api;
pub mod args;
pub mod callbacks;
pub mod install;

use api::QEMU_PLUGIN_VERSION;

#[no_mangle]
pub static qemu_plugin_version: c_int = QEMU_PLUGIN_VERSION as c_int;
