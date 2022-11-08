extern crate cbindgen;

use bindgen::builder;
use qemu::include_qemu_plugin_h;

use std::{env::var, fs::write, path::PathBuf};

fn main() {
    let out_dir = PathBuf::from(var("OUT_DIR").unwrap());
    let qemu_plugin_header = out_dir.join("qemu-plugin.h");
    let qemu_plugin_bindings = out_dir.join("qemu_plugin_bindings.rs");

    // Write the qemu plugin header
    let qemu_plugin_header_contents = include_qemu_plugin_h();

    write(&qemu_plugin_header, &qemu_plugin_header_contents)
        .expect("Failed to write qemu-plugin.h");

    let rust_bindings = builder()
        .header(qemu_plugin_header.to_str().unwrap())
        .blocklist_function("qemu_plugin_install")
        .blocklist_item("qemu_plugin_version")
        .generate()
        .expect("Unable to generate bindings for qemu-plugin.h");

    rust_bindings
        .write_to_file(qemu_plugin_bindings)
        .expect("Couldn't write bindings for qemu-plugin.h");
}
