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

    // Only link the C++ library when not using native mocks
    if !cfg!(feature = "native") {
        let lib_dir = manifest_dir
            .join("third_party/machine-asset-tools/build/riscv64")
            .canonicalize()
            .unwrap();
        println!("cargo:rustc-link-search=native={}", lib_dir.display());
        println!("cargo:rustc-link-lib=static=cma");
    }

    println!("cargo:rerun-if-changed=wrapper.h");
    println!("cargo:rerun-if-changed=third_party/machine-asset-tools/include/");
    println!("cargo:rerun-if-changed=third_party/machine-guest-tools/sys-utils/libcmt/include/");
}
