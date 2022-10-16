extern crate cbindgen;

use std::env::var;

fn main() {
    let crate_dir = var("CARGO_MANIFEST_DIR").unwrap();

    let config = cbindgen::Config {
        language: cbindgen::Language::C,
        macro_expansion: cbindgen::MacroExpansionConfig { bitflags: true },
        ..Default::default()
    };

    cbindgen::Builder::new()
        .with_crate(crate_dir)
        .with_config(config)
        .generate()
        .expect("Unable to generate bindings")
        .write_to_file("ffi/cannonball-client.h");
}
