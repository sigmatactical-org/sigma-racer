fn main() {
    // Only the firmware binary links against the embedded runtime; host
    // builds (cargo test) must not see these flags. rustc-link-arg-bins
    // applies to bins only, and the scripts resolve only for the thumb
    // target where embassy-stm32 (memory-x) and defmt are in the graph.
    let target = std::env::var("TARGET").unwrap_or_default();
    if target.starts_with("thumbv7em") {
        println!("cargo:rustc-link-arg-bins=--nmagic");
        println!("cargo:rustc-link-arg-bins=-Tlink.x");
        println!("cargo:rustc-link-arg-bins=-Tdefmt.x");
    }
}
