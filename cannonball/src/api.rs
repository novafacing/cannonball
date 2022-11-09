//! Rust bindings for the QEMU plugin API.
//!
//! This module provides raw sys-level bindings to the QEMU plugin API. It also provides
//! some helper functions for working with the API to build a plugin written entirely in Rust.

#![allow(non_upper_case_globals)]
#![allow(non_camel_case_types)]
#![allow(non_snake_case)]

include!(concat!(env!("OUT_DIR"), "/qemu_plugin_bindings.rs"));
