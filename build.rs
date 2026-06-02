// Copies pdfium.dll from the project root into the build output directory
// so that `cargo run` works without manual copying.
use std::{env, fs, path::PathBuf};

fn main() {
    println!("cargo:rerun-if-changed=pdfium.dll");

    let manifest = PathBuf::from(env::var("CARGO_MANIFEST_DIR").unwrap());
    let dll_src  = manifest.join("pdfium.dll");

    if !dll_src.exists() {
        return; // Not downloaded yet — the binary will still build, just won't run.
    }

    // OUT_DIR is target/{debug|release}/build/<crate>-<hash>/out
    // Three parents up lands us at target/{debug|release}/
    let out_dir   = PathBuf::from(env::var("OUT_DIR").unwrap());
    let build_dir = out_dir.ancestors().nth(3).unwrap().to_path_buf();
    let dll_dst   = build_dir.join("pdfium.dll");

    if !dll_dst.exists() {
        fs::copy(&dll_src, &dll_dst)
            .expect("build.rs: failed to copy pdfium.dll to build output");
        println!("cargo:warning=Copied pdfium.dll → {}", dll_dst.display());
    }
}
