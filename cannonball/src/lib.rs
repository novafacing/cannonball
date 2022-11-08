#![allow(non_upper_case_globals)]

use libc::c_int;

pub mod api;
pub mod args;
pub mod callbacks;
pub mod install;

use api::QEMU_PLUGIN_VERSION;

#[no_mangle]
pub static qemu_plugin_version: c_int = QEMU_PLUGIN_VERSION as c_int;
