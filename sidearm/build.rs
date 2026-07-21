use std::env;
use std::fs::{self, File};
use std::io::Write;
use std::path::PathBuf;

fn main() {
    let manifest_dir = PathBuf::from(env::var("CARGO_MANIFEST_DIR").unwrap());
    let memory_name = if env::var("CARGO_FEATURE_MEMORY_ITCM").is_ok() {
        "memory-itcm.x"
    } else {
        "memory-ddr.x"
    };
    let memory = fs::read(manifest_dir.join(memory_name)).unwrap();

    let out = PathBuf::from(env::var("OUT_DIR").unwrap());
    File::create(out.join("memory.x"))
        .unwrap()
        .write_all(&memory)
        .unwrap();

    println!("cargo:rustc-link-search={}", out.display());
    // Only bare-metal (firmware) links get the RSC linker script; injecting it
    // into a host x86_64 link corrupts the test binaries.
    if env::var("CARGO_CFG_TARGET_OS").as_deref() == Ok("none") {
        println!(
            "cargo:rustc-link-arg=-T{}/link-rsc.x",
            manifest_dir.display()
        );
    }
    println!("cargo:rerun-if-changed=memory-ddr.x");
    println!("cargo:rerun-if-changed=memory-itcm.x");
    println!("cargo:rerun-if-changed=build.rs");
}
