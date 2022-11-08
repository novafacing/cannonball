//! Plugin installation
//!
//! This module will handle installation and registration with QEMU. It exports the
//! `qemu_plugin_install` function which is called by QEMU when the plugin is loaded. This
//! function will run setup callbacks and register static callbacks with QEMU.

use inventory;
use libc::{c_char, c_int};

use crate::{
    api::{qemu_info_t, qemu_plugin_id_t},
    args::Args,
    callbacks::{Register, SetupCallbackType, StaticCallbackType},
};

const PLUGIN_INSTALL_SUCCESS: c_int = 0;

inventory::collect!(SetupCallbackType);
inventory::collect!(StaticCallbackType);

#[no_mangle]
/// Global entry point. This function will be called by QEMU when the plugin is loaded
/// using `dlopen`.
pub extern "C" fn qemu_plugin_install(
    id: qemu_plugin_id_t,
    info: *const qemu_info_t,
    argc: c_int,
    argv: *const *const c_char,
) -> c_int {
    let args = Args::new(argc, argv);

    for setup_cb in inventory::iter::<SetupCallbackType> {
        match setup_cb {
            SetupCallbackType::Setup(setup_cb) => {
                (setup_cb.cb)(info, &args);
            }
        }
    }

    for callback in inventory::iter::<StaticCallbackType> {
        callback.register(id);
    }

    PLUGIN_INSTALL_SUCCESS
}
