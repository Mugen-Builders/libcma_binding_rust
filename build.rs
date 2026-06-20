use std::{env, path::Path, path::PathBuf, process::Command};

fn main() {
    let out_dir = PathBuf::from(env::var("OUT_DIR").unwrap());
    let manifest_dir = PathBuf::from(env::var("CARGO_MANIFEST_DIR").unwrap());

    let mat = manifest_dir.join("third_party/machine-asset-tools");

    // A fresh `git clone` (without --recurse-submodules) leaves third_party/ empty, which would
    // make the include-path lookups below fail. Pull the submodules in automatically so the crate
    // builds with nothing more than `cargo build` — no out-of-band setup step required.
    if !mat.join("include").exists() {
        run("git", &["submodule", "update", "--init", "--recursive"], &manifest_dir);
    }

    let cma_include_dir = mat.join("include").canonicalize().expect(
        "third_party/machine-asset-tools/include is still missing after submodule init — \
         is this a git checkout? otherwise run `git submodule update --init --recursive`",
    );
    let cmt_include_dir = manifest_dir
        .join("third_party/machine-guest-tools/sys-utils/libcmt/include")
        .canonicalize()
        .expect(
            "third_party/machine-guest-tools is still missing after submodule init — \
             is this a git checkout? otherwise run `git submodule update --init --recursive`",
        );

    let mut builder = bindgen::Builder::default()
        .header(manifest_dir.join("wrapper.h").to_str().unwrap())
        .clang_arg(format!("-I{}", cma_include_dir.display()))
        .clang_arg(format!("-I{}", cmt_include_dir.display()));

    // bindgen embeds libclang, which needs the compiler's builtin headers (stdbool.h, stddef.h…).
    // Some installs ship libclang WITHOUT its resource-dir headers (e.g. only `libclang1` and no
    // `libclang-common-*-dev`), so bindgen fails with "stdbool.h file not found". If the caller
    // hasn't already supplied include args, fall back to GCC's builtin header dir, which is always
    // present wherever a C compiler is. Harmless when clang has its own headers (its resource dir
    // is searched first; this is only an -isystem fallback).
    if env::var_os("BINDGEN_EXTRA_CLANG_ARGS").is_none() {
        if let Some(gcc_inc) = gcc_builtin_include() {
            builder = builder.clang_arg("-isystem").clang_arg(gcc_inc);
        }
    }

    let bindings = builder
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

    // Link the real C++ libcma when not using the native mock.
    if !cfg!(feature = "native") {
        let lib_dir = mat.join("build/riscv64");
        let lib_path = lib_dir.join("libcma.a");

        // Build libcma.a from source if it isn't already present. This is what lets the crate be
        // consumed as a plain `git`/`crates.io` dependency WITHOUT vendoring a prebuilt archive.
        //
        // Build-environment requirements (the Cartesi SDK / app Dockerfile provides these):
        //   - GNU make, wget, and network access
        //   - the RISC-V GCC 14 cross toolchain: g++-14-riscv64-linux-gnu / gcc-14-riscv64-linux-gnu
        //     (libcma's C++ source requires GCC >= 14).
        // Override the compiler names with CMA_RISCV64_CXX / CMA_RISCV64_CC if your toolchain
        // differs, or skip this whole path by pre-building build/riscv64/libcma.a yourself.
        if !lib_path.exists() {
            let cxx =
                env::var("CMA_RISCV64_CXX").unwrap_or_else(|_| "riscv64-linux-gnu-g++-14".into());
            let cc =
                env::var("CMA_RISCV64_CC").unwrap_or_else(|_| "riscv64-linux-gnu-gcc-14".into());

            // machine-asset-tools' `third-party` target fetches Boost/emulator/guest-tools but not
            // nlohmann/json, so fetch that single header first.
            let nlohmann = mat.join("third-party/nlohmann/json.hpp");
            if !nlohmann.exists() {
                std::fs::create_dir_all(mat.join("third-party/nlohmann")).ok();
                run(
                    "wget",
                    &[
                        "-qO",
                        nlohmann.to_str().unwrap(),
                        "https://github.com/nlohmann/json/releases/download/v3.12.0/json.hpp",
                    ],
                    &mat,
                );
            }

            // Download + stage the third-party deps, then cross-compile the static archive.
            run("make", &["third-party", "TOOLCHAIN_PREFIX=riscv64-linux-gnu-"], &mat);
            run(
                "make",
                &[
                    "build/riscv64/libcma.a",
                    "TOOLCHAIN_PREFIX=riscv64-linux-gnu-",
                    &format!("CXX={cxx}"),
                    &format!("CC={cc}"),
                    "AR=riscv64-linux-gnu-ar",
                ],
                &mat,
            );
            assert!(
                lib_path.exists(),
                "libcma.a build did not produce {}",
                lib_path.display()
            );
        }

        println!(
            "cargo:rustc-link-search=native={}",
            lib_dir.canonicalize().unwrap().display()
        );
        println!("cargo:rustc-link-lib=static=cma");
        // libcma is C++ (Boost.Interprocess/Unordered); link the C++ runtime after it so its
        // vtables / __cxxabiv1 ABI symbols resolve. Dynamic so the cross gcc locates libstdc++.so
        // automatically (the machine rootfs must provide libstdc++6).
        println!("cargo:rustc-link-lib=dylib=stdc++");
    }

    println!("cargo:rerun-if-changed=wrapper.h");
    println!("cargo:rerun-if-changed=third_party/machine-asset-tools/include/");
    println!("cargo:rerun-if-changed=third_party/machine-guest-tools/sys-utils/libcmt/include/");
}

/// Return the directory holding GCC's builtin headers (stdbool.h, stddef.h…), if discoverable.
/// Used as a fallback so bindgen works even when libclang ships without its own resource headers.
fn gcc_builtin_include() -> Option<String> {
    let cc = env::var("CC").unwrap_or_else(|_| "cc".into());
    let out = Command::new(cc).arg("-print-file-name=include").output().ok()?;
    if !out.status.success() {
        return None;
    }
    let dir = String::from_utf8(out.stdout).ok()?.trim().to_string();
    if !dir.is_empty() && Path::new(&dir).join("stdbool.h").exists() {
        Some(dir)
    } else {
        None
    }
}

/// Run a command in `cwd`, panicking with a helpful message if it is missing or fails.
fn run(cmd: &str, args: &[&str], cwd: &Path) {
    let status = Command::new(cmd)
        .args(args)
        .current_dir(cwd)
        .status()
        .unwrap_or_else(|e| panic!("failed to spawn `{cmd}` ({e}); is it installed?"));
    assert!(status.success(), "`{cmd} {}` failed", args.join(" "));
}
