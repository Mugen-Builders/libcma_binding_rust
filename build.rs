use std::{env, path::PathBuf};

fn main() {
    let out_dir = PathBuf::from(env::var("OUT_DIR").unwrap());
    let manifest_dir = PathBuf::from(env::var("CARGO_MANIFEST_DIR").unwrap());
    let cma_include_dir = manifest_dir
        .join("third_party/machine-asset-tools/include")
        .canonicalize()
        .unwrap();
    let cmt_include_dir = manifest_dir
        .join("third_party/machine-guest-tools/sys-utils/libcmt/include")
        .canonicalize()
        .unwrap();

    let bindings = bindgen::Builder::default()
        .header(manifest_dir.join("wrapper.h").to_str().unwrap())
        .clang_arg(format!("-I{}", cma_include_dir.display()))
        .clang_arg(format!("-I{}", cmt_include_dir.display()))
        .allowlist_function("cma_.*")
        .allowlist_type("cma_.*")
        .allowlist_type("cmt_.*")
        .allowlist_var("CMT_.*")
        .allowlist_var("CMA_.*")
        .parse_callbacks(Box::new(bindgen::CargoCallbacks::new()))
        .generate()
        .expect("bindgen failed");

    bindings
        .write_to_file(out_dir.join("bindings.rs"))
        .expect("Failed to write bindings");

    // Without `native`, prefer linking the real static `libcma.a`. If it is missing (typical on a
    // host that has not run the machine-asset-tools Docker build), fall back to the same Rust mocks
    // as `native` so `cargo build --no-default-features` still works for CI and local dev.
    if !cfg!(feature = "native") {
        println!("cargo:rustc-check-cfg=cfg(cma_host_mocks)");
        println!("cargo:rerun-if-env-changed=CMA_LIB_DIR");
        let lib_dir = env::var("CMA_LIB_DIR")
            .map(PathBuf::from)
            .unwrap_or_else(|_| {
                manifest_dir.join("third_party/machine-asset-tools/build/riscv64")
            });
        let lib_a = lib_dir.join("libcma.a");
        println!("cargo:rerun-if-changed={}", lib_a.display());

        if lib_a.is_file() {
            let lib_dir = lib_dir.canonicalize().unwrap_or_else(|e| {
                panic!("failed to resolve path {}: {e}", lib_dir.display());
            });
            println!("cargo:rustc-link-search=native={}", lib_dir.display());
            println!("cargo:rustc-link-lib=static=cma");
        } else {
            println!(
                "cargo:warning=libcma.a not found at {}; compiling host mocks instead. \
                 Set CMA_LIB_DIR or build machine-asset-tools to link the static library.",
                lib_a.display()
            );
            println!("cargo:rustc-cfg=cma_host_mocks");
        }
    }

    println!("cargo:rerun-if-changed=wrapper.h");
    println!("cargo:rerun-if-changed=third_party/machine-asset-tools/include/");
    println!("cargo:rerun-if-changed=third_party/machine-guest-tools/sys-utils/libcmt/include/");
}
