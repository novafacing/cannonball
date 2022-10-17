extern crate cbindgen;

use std::{env::var, fs::create_dir_all, path::PathBuf};

fn target_dir() -> PathBuf {
    if let Ok(target) = var("CARGO_TARGET_DIR") {
        PathBuf::from(target)
    } else {
        PathBuf::from(var("CARGO_MANIFEST_DIR").unwrap()).join("target")
    }
}

fn main() {
    let crate_dir = var("CARGO_MANIFEST_DIR").unwrap();

    let ffi_outdir = target_dir().join("ffi");

    // create the ffi directory if it doesn't exist
    if !ffi_outdir.exists() {
        create_dir_all(&ffi_outdir).expect(
            format!(
                "Unable to create directory: {}",
                ffi_outdir.as_os_str().to_string_lossy()
            )
            .as_str(),
        );
    }

    let config = cbindgen::Config {
        language: cbindgen::Language::C,
        macro_expansion: cbindgen::MacroExpansionConfig { bitflags: true },
        ..Default::default()
    };

    // Generate the C header bindings for the library
    cbindgen::Builder::new()
        .with_crate(crate_dir)
        .with_config(config)
        .generate()
        .expect("Unable to generate bindings")
        .write_to_file(format!(
            "{}/cannonball-client.h",
            ffi_outdir.as_os_str().to_string_lossy()
        ));
}
