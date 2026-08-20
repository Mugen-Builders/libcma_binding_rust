use std::{env, path::Path, path::PathBuf, process::Command};

/// Pinned SHA-256 of the nlohmann/json v3.12.0 single-header release asset (`json.hpp`).
/// Verified out-of-band against the upstream GitHub release download. A mismatch means the
/// fetched header was corrupted or tampered with — the build must refuse to proceed.
const NLOHMANN_JSON_SHA256: &str =
    "aaf127c04cb31c406e5b04a63f1ae89369fccde6d8fa7cdda1ed4f32dfc5de63";

fn main() {
    let out_dir = PathBuf::from(env::var("OUT_DIR").unwrap());
    let manifest_dir = PathBuf::from(env::var("CARGO_MANIFEST_DIR").unwrap());

    let mat = manifest_dir.join("third_party/machine-asset-tools");

    // A fresh `git clone` (without --recurse-submodules) leaves third_party/ empty, which would
    // make the include-path lookups below fail. Pull the submodules in automatically so the crate
    // builds with nothing more than `cargo build` — no out-of-band setup step required.
    if !mat.join("include").exists() {
        run(
            "git",
            &["submodule", "update", "--init", "--recursive"],
            &manifest_dir,
        );
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

    // MOCK backend active: shout about it so a fake in-memory ledger can never be shipped to
    // production unnoticed. NOTE also that everything in the `!mock` block below (the wget of
    // nlohmann/json, `make third-party` which fetches Boost et al., and the C++ compile of
    // libcma.a) is gated OFF here — so a `mock` build downloads no third-party C++ sources and
    // needs no C++ toolchain.
    if cfg!(feature = "mock") {
        println!(
            "cargo:warning=libcma_binding_rust: building with the MOCK ledger (feature `mock`) — \
             this is NOT real libcma; never use in production. For a real build use \
             default-features = false, features = [\"host-real\"] (or \"riscv64\")."
        );
    }

    // Link the real C++ libcma when the MOCK is NOT selected. Two targets:
    //   - `riscv64`   → cross-compile for the Cartesi machine (the non-mock default).
    //   - `host-real` → compile for the host (x86_64), so an off-chain consumer runs the
    //                   *same* ledger the machine will. The DEFS in machine-asset-tools force
    //                   SIMD-free/generic paths so the record bytes match across arches.
    //
    // Everything inside this block fetches third-party C++ sources from the NETWORK and invokes a
    // C++ COMPILER; none of it runs for a `mock` build. Keeping the fetch/compile confined here is
    // what keeps the default `mock` path hermetic (no network, no toolchain, docs.rs/offline-safe).
    if !cfg!(feature = "mock") {
        // Real builds are NOT hermetic: they require network access and a C++ toolchain (g++ >= 14
        // for C++20/23). Make that requirement visible in the build log up front.
        println!(
            "cargo:warning=libcma_binding_rust: building REAL libcma from C++ source — this build \
             requires network access and a C++ toolchain (g++ >= 14). See build.rs for details."
        );

        let host = cfg!(feature = "host-real");
        // Distinct object dirs so a host build and a cross build never clobber each other.
        let obj_subdir = if host { "build/host" } else { "build/riscv64" };
        let lib_dir = mat.join(obj_subdir);
        let lib_path = lib_dir.join("libcma.a");
        // libcma.a archives ONLY libcma's own objects (ledger*, parser*, utils). `parser_impl.o`
        // references libcmt's C ABI helpers (cmt_abi_*, cmt_buf_*) and those symbols are NOT in
        // the archive, so libcmt must be linked alongside it. Where libcmt comes from is
        // arch-dependent — see `cmt_lib_path` below.
        //
        // This is easy to miss because archive members are pulled lazily: nothing in this crate's
        // own code path calls the C parser (parser.rs is pure Rust over alloy), so `parser_impl.o`
        // is never pulled and the missing symbols stay invisible until a DOWNSTREAM consumer calls
        // cma_parser_decode_advance/inspect or cma_parser_encode_voucher — at which point the link
        // fails with a dozen `undefined symbol: cmt_abi_*`. tests/c_parser_link.rs forces exactly
        // that pull so the regression cannot come back unnoticed.
        //
        // riscv64: upstream deliberately does NOT build libcmt from source (its real io backend
        // needs the cartesi Linux kernel headers, which exist only inside the guest). It comes
        // from the machine-guest-tools package installed in the app image, so we emit a plain
        // `-lcmt` and let the image supply it.
        // host: the Makefile builds it from the vendored guest-tools sources with the mock io
        // backend, so we build and statically link that archive.
        let cmt_lib_path = lib_dir.join("libcmt.a");

        // Build libcma.a from source if it isn't already present. This is what lets the crate be
        // consumed as a plain `git`/`crates.io` dependency WITHOUT vendoring a prebuilt archive.
        //
        // Build-environment requirements (the Cartesi SDK / app Dockerfile provide the cross set):
        //   - GNU make, wget, and network access
        //   - riscv64: the RISC-V GCC 14 cross toolchain (g++-14-riscv64-linux-gnu / gcc-14-…).
        //   - host-real: a host C++ toolchain with g++ >= 14 (C++20/C++23) and Boost is fetched.
        // Override the compiler names with CMA_RISCV64_CXX/CC or CMA_HOST_CXX/CC.
        //
        // Presence alone is NOT a sufficient cache key. `libcma.a` is compiled from the
        // machine-asset-tools submodule, but nothing about the archive records WHICH revision it
        // came from, so bumping the submodule used to leave a stale archive in place and silently
        // link yesterday's libcma against today's headers — exactly the kind of skew the bindgen
        // layout assertions cannot catch, because they only ever see the headers. The stamp below
        // records the inputs that determine the archive's contents; any change forces a rebuild.
        let stamp_path = lib_dir.join(".cma-build-stamp");
        let stamp = build_stamp(&manifest_dir, host);
        let stamp_matches = std::fs::read_to_string(&stamp_path)
            .map(|recorded| recorded.trim() == stamp.trim())
            .unwrap_or(false);
        let artifacts_present = lib_path.exists() && (!host || cmt_lib_path.exists());

        if !artifacts_present || !stamp_matches {
            // A stale stamp means the tree the objects were compiled from is gone. Make cannot be
            // trusted to notice: a `git checkout` of the submodule can leave source files with
            // OLDER mtimes than the objects built from the previous revision, so an incremental
            // make would consider them up to date. Drop the whole object dir and rebuild clean.
            if lib_dir.exists() && !stamp_matches {
                println!(
                    "cargo:warning=libcma_binding_rust: build inputs changed since {} was compiled \
                     — rebuilding from scratch.",
                    lib_dir.display()
                );
                std::fs::remove_dir_all(&lib_dir)
                    .expect("failed to clear the stale libcma object directory");
            }
            let (toolchain_prefix, cxx, cc, ar) = if host {
                (
                    String::new(),
                    env::var("CMA_HOST_CXX").unwrap_or_else(|_| "g++".into()),
                    env::var("CMA_HOST_CC").unwrap_or_else(|_| "gcc".into()),
                    "ar".to_string(),
                )
            } else {
                (
                    "riscv64-linux-gnu-".to_string(),
                    env::var("CMA_RISCV64_CXX")
                        .unwrap_or_else(|_| "riscv64-linux-gnu-g++-14".into()),
                    env::var("CMA_RISCV64_CC")
                        .unwrap_or_else(|_| "riscv64-linux-gnu-gcc-14".into()),
                    "riscv64-linux-gnu-ar".to_string(),
                )
            };

            // machine-asset-tools' `third-party` target fetches Boost/emulator/guest-tools but not
            // nlohmann/json, so fetch that single header first.
            let nlohmann = mat.join("third-party/nlohmann/json.hpp");
            if !nlohmann.exists() {
                std::fs::create_dir_all(mat.join("third-party/nlohmann")).ok();
                // GNU `wget` has no `--checksum` flag (only wget2 does), so we download here and
                // verify the SHA-256 against a pinned constant below rather than at fetch time.
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
            // Supply-chain gate: pin + verify the one header the Makefile does not fetch itself.
            // This runs on BOTH a fresh download and a pre-existing/vendored copy, so a tampered or
            // corrupted cache is caught too. A mismatch is a hard, un-ignorable build failure.
            verify_sha256(&nlohmann, NLOHMANN_JSON_SHA256);

            // Self-heal a partially-staged libcmt. Upstream's `third-party/libcmt` make target is a
            // DIRECTORY, and a directory's mtime is trivially newer than the tarball it was
            // extracted from, so make considers any existing copy up to date and never repairs it.
            // A checkout that was staged by an older revision therefore keeps a headers-only
            // directory forever, and the libcmt compile dies on
            // `third-party/libcmt/src/buf.c: No such file or directory`. Removing it forces a clean
            // re-extraction of both the headers and the sources.
            let staged_cmt = mat.join("third-party/libcmt");
            if staged_cmt.exists() && !staged_cmt.join("src/buf.c").exists() {
                std::fs::remove_dir_all(&staged_cmt)
                    .expect("failed to clear the partially-staged third-party/libcmt directory");
            }

            // Download + stage the third-party deps, then compile the static archive. The Makefile
            // hardcodes `libcma_OBJDIR := build/riscv64`; override it so the host build lands in its
            // own dir (command-line assignments beat the Makefile's `:=`).
            //
            // Build ONLY the pieces libcma.a actually needs — Boost, the guest-tools libcmt
            // headers, and nlohmann/json. The blanket `make third-party` also downloads and
            // extracts the prebuilt cartesi-machine emulator .deb (~57 MB, xz-compressed), which is
            // linked ONLY by the host-side `account-driver-reader` tool, NOT by libcma.a. Pulling it
            // in needs `xz` (and downloads tens of MB) for nothing, and breaks minimal build
            // environments such as the Cartesi machine cross-build image (which ships no `xz`).
            run(
                "make",
                &[
                    "third-party-boost",
                    "third-party-guest-tools",
                    "third-party-nlohmann-json",
                    &format!("TOOLCHAIN_PREFIX={toolchain_prefix}"),
                ],
                &mat,
            );
            // On the host, build libcmt.a in the same invocation. On riscv64 the Makefile leaves
            // `libcmt_LIB` empty on purpose (see the comment on `cmt_lib_path`), so asking for the
            // target there would fail with "no rule to make target".
            let mut targets = vec![format!("{obj_subdir}/libcma.a")];
            if host {
                targets.push(format!("{obj_subdir}/libcmt.a"));
            }
            let mut make_args: Vec<String> = targets;
            make_args.extend([
                format!("libcma_OBJDIR={obj_subdir}"),
                format!("TOOLCHAIN_PREFIX={toolchain_prefix}"),
                format!("CXX={cxx}"),
                format!("CC={cc}"),
                format!("AR={ar}"),
            ]);
            run(
                "make",
                &make_args.iter().map(String::as_str).collect::<Vec<_>>(),
                &mat,
            );
            assert!(
                lib_path.exists(),
                "libcma.a build did not produce {}",
                lib_path.display()
            );
            assert!(
                !host || cmt_lib_path.exists(),
                "libcmt.a build did not produce {}",
                cmt_lib_path.display()
            );

            // Written only after the archives exist, so an interrupted or failed build leaves no
            // stamp and the next run rebuilds rather than trusting a half-built object dir.
            std::fs::write(&stamp_path, &stamp).expect("failed to write the libcma build stamp");
        }

        println!(
            "cargo:rustc-link-search=native={}",
            lib_dir.canonicalize().unwrap().display()
        );
        println!("cargo:rustc-link-lib=static=cma");
        // MUST come after `cma`: static archives are resolved left-to-right, and it is libcma's
        // parser_impl.o that references the cmt_abi_*/cmt_buf_* symbols, not the other way round.
        if host {
            println!("cargo:rustc-link-lib=static=cmt");
        } else {
            // Plain `-lcmt`, matching what upstream's own sample apps pass: resolves against the
            // libcmt shipped by the machine-guest-tools package in the Cartesi app image.
            println!("cargo:rustc-link-lib=cmt");
        }
        // libcma is C++ (Boost.Interprocess/Unordered); link the C++ runtime after it so its
        // vtables / __cxxabiv1 ABI symbols resolve. Dynamic so the cross gcc locates libstdc++.so
        // automatically (the machine rootfs must provide libstdc++6).
        println!("cargo:rustc-link-lib=dylib=stdc++");
    }

    println!("cargo:rerun-if-changed=wrapper.h");
    println!("cargo:rerun-if-changed=third_party/machine-asset-tools/include/");
    println!("cargo:rerun-if-changed=third_party/machine-guest-tools/sys-utils/libcmt/include/");
}

/// Identity of everything that determines the contents of the prebuilt `libcma.a`/`libcmt.a`.
///
/// Compared against the stamp stored beside the archives to decide whether a cached build is
/// still valid. Covers:
///   - the checked-out revision of each submodule the C++ is compiled from (the reason this
///     exists: a submodule bump must invalidate the archive);
///   - the target arch, since host and cross objects are not interchangeable;
///   - the compiler overrides, since switching toolchains changes the objects.
///
/// The submodule revisions come from `git -C <path> rev-parse HEAD` rather than from the
/// gitlink, so a locally checked-out or dirty submodule is also distinguished. A checkout with
/// no usable git (a vendored tarball, say) reports `unknown`, which keeps the stamp stable and
/// falls back to plain presence-caching rather than rebuilding on every single run.
fn build_stamp(manifest_dir: &Path, host: bool) -> String {
    let rev = |sub: &str| -> String {
        Command::new("git")
            .args([
                "-C",
                manifest_dir.join(sub).to_str().unwrap_or("."),
                "rev-parse",
                "HEAD",
            ])
            .output()
            .ok()
            .filter(|o| o.status.success())
            .and_then(|o| String::from_utf8(o.stdout).ok())
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty())
            .unwrap_or_else(|| "unknown".to_string())
    };

    let (cxx, cc) = if host {
        (
            env::var("CMA_HOST_CXX").unwrap_or_else(|_| "g++".into()),
            env::var("CMA_HOST_CC").unwrap_or_else(|_| "gcc".into()),
        )
    } else {
        (
            env::var("CMA_RISCV64_CXX").unwrap_or_else(|_| "riscv64-linux-gnu-g++-14".into()),
            env::var("CMA_RISCV64_CC").unwrap_or_else(|_| "riscv64-linux-gnu-gcc-14".into()),
        )
    };

    // Line-oriented and human-readable on purpose: when a rebuild is triggered unexpectedly, the
    // stamp file is the first thing anyone will `cat`.
    format!(
        "machine-asset-tools={}\nmachine-guest-tools={}\narch={}\ncxx={cxx}\ncc={cc}\n",
        rev("third_party/machine-asset-tools"),
        rev("third_party/machine-guest-tools"),
        if host { "host" } else { "riscv64" },
    )
}

/// Return the directory holding GCC's builtin headers (stdbool.h, stddef.h…), if discoverable.
/// Used as a fallback so bindgen works even when libclang ships without its own resource headers.
fn gcc_builtin_include() -> Option<String> {
    let cc = env::var("CC").unwrap_or_else(|_| "cc".into());
    let out = Command::new(cc)
        .arg("-print-file-name=include")
        .output()
        .ok()?;
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

/// Verify a file against a pinned SHA-256, failing the build loudly on mismatch. Used as the
/// supply-chain gate for the nlohmann/json header fetched over the network. Shells out to a
/// system hashing tool so no extra crate dependency is required.
fn verify_sha256(path: &Path, expected: &str) {
    let actual = sha256_of(path).unwrap_or_else(|| {
        panic!(
            "cannot verify {}: no usable SHA-256 tool found (need `sha256sum`, `shasum`, or \
             `openssl` on PATH). Refusing to build against an unverified nlohmann/json header.",
            path.display()
        )
    });
    assert!(
        actual.eq_ignore_ascii_case(expected),
        "SHA-256 mismatch for {}: expected {expected}, got {actual}. Refusing to build against an \
         unverified nlohmann/json header (possible supply-chain tampering or a corrupted download). \
         Delete the file and re-fetch, or update the pin in build.rs if the upstream release changed.",
        path.display()
    );
}

/// Compute the lowercase hex SHA-256 of `path` using whatever system tool is available
/// (`sha256sum`, then `shasum -a 256`, then `openssl dgst -sha256 -r`). Returns None if none
/// produced a valid 64-char hex digest. Avoids pulling in a hashing crate as a build-dependency.
fn sha256_of(path: &Path) -> Option<String> {
    let p = path.to_str()?;
    // (command, args preceding the file path); each tool prints the digest as the first token.
    let candidates: [(&str, &[&str]); 3] = [
        ("sha256sum", &[]),
        ("shasum", &["-a", "256"]),
        ("openssl", &["dgst", "-sha256", "-r"]),
    ];
    for (cmd, pre) in candidates {
        let mut args: Vec<&str> = pre.to_vec();
        args.push(p);
        if let Ok(out) = Command::new(cmd).args(&args).output() {
            if out.status.success() {
                if let Ok(s) = String::from_utf8(out.stdout) {
                    if let Some(tok) = s.split_whitespace().next() {
                        if tok.len() == 64 && tok.bytes().all(|b| b.is_ascii_hexdigit()) {
                            return Some(tok.to_ascii_lowercase());
                        }
                    }
                }
            }
        }
    }
    None
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
