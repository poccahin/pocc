use std::env;
use std::path::PathBuf;
use std::process::Command;

fn main() {
    let out_dir = env::var("OUT_DIR").expect("❌ [FATAL] OUT_DIR not set by Cargo.");
    let target_os = env::var("CARGO_CFG_TARGET_OS").unwrap_or_else(|_| "linux".to_string());

    println!("cargo:warning=🌌 [LIFE++] Initiating Planetary Cross-Compilation Matrix...");

    println!("cargo:warning=⚡ [ZIG] Forging L0 Kinetic Trust Root...");

    let zig_out_lib = PathBuf::from(&out_dir).join("libopenclaw_kinetic.a");

    let zig_status = Command::new("zig")
        .args([
            "build-lib",
            "openclaw-kinetic/src/pokw_generator.zig",
            "-O",
            "ReleaseFast",
            "-femit-bin",
            zig_out_lib.to_str().expect("❌ [FATAL] Invalid output path for Zig lib."),
            "-fPIC",
        ])
        .status()
        .expect("❌ [FATAL] Failed to invoke Zig compiler. Is Zig (>=0.13) installed?");

    if !zig_status.success() {
        panic!("❌ [FATAL] Zig compilation of Kinetic layer failed!");
    }

    println!("cargo:warning=🚀 [C++] Compiling L2.5 Tensor Wind Tunnel via CMake...");

    let cpp_dst = cmake::Config::new("openclaw-tensor")
        .profile("Release")
        .build();

    println!("cargo:rustc-link-search=native={}", out_dir);
    println!("cargo:rustc-link-search=native={}/lib", cpp_dst.display());

    println!("cargo:rustc-link-lib=static=openclaw_kinetic");
    println!("cargo:rustc-link-lib=static=openclaw_tensor");

    if target_os == "macos" {
        println!("cargo:rustc-link-lib=dylib=c++");
    } else {
        println!("cargo:rustc-link-lib=dylib=stdc++");
    }

    println!("cargo:rerun-if-changed=openclaw-kinetic/src/");
    println!("cargo:rerun-if-changed=openclaw-tensor/src/");
    println!("cargo:rerun-if-changed=openclaw-tensor/CMakeLists.txt");

    println!("cargo:warning=✅ [MATRIX] Subsystems compiled and fused. Passing control to Rustc.");
}
