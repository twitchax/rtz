fn main() {
    // Let the rust linter know about the possible custom configs.
    println!("cargo::rustc-check-cfg=cfg(host_family_windows)");
    println!("cargo::rustc-check-cfg=cfg(host_family_unix)");
    println!("cargo::rustc-check-cfg=cfg(host_family_wasm)");
    println!("cargo::rustc-check-cfg=cfg(wasm)");

    // Set special host configs.
    if cfg!(windows) {
        println!("cargo::rustc-cfg=host_family_windows");
    }
    if cfg!(unix) {
        println!("cargo::rustc-cfg=host_family_unix");
    }
    if cfg!(wasm) {
        println!("cargo::rustc-cfg=host_family_wasm");
        println!("cargo::rustc-cfg=wasm");
    }

    // Do not run the build script if the target is wasm.
    #[cfg(not(target_family = "wasm"))]
    rtz_build::main();
}
