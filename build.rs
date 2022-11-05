extern crate cbindgen;

use cc::Build;
use pkg_config;
use qemu::include_qemu_plugin_h;

use std::{env::var, fs::write, path::PathBuf};

fn main() {
    let crate_dir = var("CARGO_MANIFEST_DIR").unwrap();

    let out_dir = PathBuf::from(var("OUT_DIR").unwrap());
    let cannonball_header = out_dir.join("cannonball.h");
    let qemu_plugin_header = out_dir.join("qemu-plugin.h");
    let libcannonball = out_dir.join("libcannonball.a");

    // Write the qemu plugin header
    let qemu_plugin_header_contents = include_qemu_plugin_h();
    write(&qemu_plugin_header, &qemu_plugin_header_contents)
        .expect("Failed to write qemu-plugin.h");

    let config = cbindgen::Config {
        language: cbindgen::Language::C,
        macro_expansion: cbindgen::MacroExpansionConfig { bitflags: true },
        ..Default::default()
    };

    // Generate the C header bindings for the library
    let bindings = cbindgen::Builder::new()
        .with_crate(crate_dir)
        .with_config(config)
        .generate()
        .expect("Unable to generate bindings");

    bindings.write_to_file(cannonball_header);

    let libglib = pkg_config::Config::new()
        .atleast_version("2.0")
        .probe("glib-2.0")
        .unwrap();

    // Tell cargo to link with libglib
    println!(
        "cargo:rustc-link-search=native={}",
        libglib.link_paths[0].display()
    );
    println!("cargo:rustc-link-lib=glib-2.0");

    Build::new()
        .file("plugin/src/args.c")
        .file("plugin/src/callback.c")
        .file("plugin/src/install.c")
        .file("plugin/src/logging.c")
        .include("plugin/include")
        .include(&out_dir)
        .includes(libglib.include_paths)
        .out_dir(&out_dir)
        .cargo_metadata(true)
        .compile("cannonball");

    // Make sure the library exists
    if !libcannonball.exists() {
        panic!("Failed to compile cannonball library");
    }
}
