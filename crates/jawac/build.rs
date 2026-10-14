//! Generates and compiles the C++ half of the cxx bridge.
//!
//! When CMake drives the build it sets `JAWA_CXXBRIDGE_DIR`, and the headers
//! the C++ backend needs (`rust/cxx.h` and the generated `bridge.rs.h`) are
//! copied there.

use std::env;
use std::fs;
use std::path::{Path, PathBuf};

fn main() {
    let root = PathBuf::from(env::var("CARGO_MANIFEST_DIR").unwrap()).join("../..");
    cxx_build::bridge("src/bridge.rs")
        .include(root.join("include"))
        .std("c++17")
        .compile("jawac-bridge");

    if let Ok(dir) = env::var("JAWA_CXXBRIDGE_DIR") {
        let generated = PathBuf::from(env::var("OUT_DIR").unwrap()).join("cxxbridge/include");
        let dir = PathBuf::from(dir);
        copy(&generated.join("rust/cxx.h"), &dir.join("rust/cxx.h"));
        copy(
            &generated.join("jawac/src/bridge.rs.h"),
            &dir.join("jawac/src/bridge.rs.h"),
        );
    }

    println!("cargo:rerun-if-changed=src/bridge.rs");
    println!("cargo:rerun-if-changed=../../include/jawa/Bridge/Bridge.h");
    for var in [
        "JAWA_CXXBRIDGE_DIR",
        "JAWA_DEFAULT_CC",
        "JAWA_RUNTIME_LIB",
        "JAWA_LINK_ARGS",
    ] {
        println!("cargo:rerun-if-env-changed={var}");
    }
}

/// Copies a file unless the destination is already identical, so CMake does
/// not rebuild the backend for nothing.
fn copy(from: &Path, to: &Path) {
    let contents = fs::read(from).unwrap_or_else(|e| panic!("reading {}: {e}", from.display()));
    if fs::read(to).ok().as_deref() == Some(contents.as_slice()) {
        return;
    }
    fs::create_dir_all(to.parent().unwrap()).unwrap();
    fs::write(to, contents).unwrap_or_else(|e| panic!("writing {}: {e}", to.display()));
}
