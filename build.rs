// SPDX-License-Identifier: MIT
//
// Build script for vips-sys (MIT).
// Links against libvips, which is LGPL-2.1 — see README “License”.

use std::fs;
use std::{collections::HashSet, env, path::PathBuf};

#[path = "build_support/config.rs"]
mod config;
use config::BuildOptions;

#[path = "build_support/unix.rs"]
mod unix;
#[path = "build_support/windows.rs"]
mod windows;

type IncludePaths = Vec<PathBuf>;
type Defines = Vec<(String, Option<String>)>;
#[derive(Default)]
struct LibraryInfo {
    include_paths: IncludePaths,
    defines: Defines,
    version: Option<String>,
}

type ProbeResult = Option<LibraryInfo>;

fn pkg_info(lib: pkg_config::Library, options: &BuildOptions) -> LibraryInfo {
    let defines = if options.generate_bindings {
        lib.defines.into_iter().collect()
    } else {
        Vec::new()
    };
    LibraryInfo {
        include_paths: lib.include_paths,
        defines,
        version: Some(lib.version),
    }
}

fn vendor_root(options: &BuildOptions) -> Option<PathBuf> {
    if options.no_vendor {
        return None;
    }
    let root = PathBuf::from(env::var_os("CARGO_MANIFEST_DIR")?).join("vendor/libvips");
    if root.is_dir() && root.join("meson.build").is_file() {
        Some(root)
    } else {
        None
    }
}

fn vendor_include_paths(options: &BuildOptions) -> Option<IncludePaths> {
    let root = vendor_root(options)?;
    let mut paths = Vec::new();
    let candidates = [
        root.join("libvips/include"),
        root.join("libvips/include/vips"),
        root.join("libvips"),
        root.join("libvips/build/vips"),
    ];
    for p in candidates {
        if p.is_dir() {
            paths.push(p);
        }
    }
    if paths.is_empty() { None } else { Some(paths) }
}

fn emit_link(kind: &str) {
    if kind == "static" {
        println!("cargo:rustc-link-lib=static=vips");
    } else {
        // Explicit dylib keeps the dynamic path obvious and avoids accidental static pull-in.
        println!("cargo:rustc-link-lib=dylib=vips");
    }
}

fn merge_includes(mut paths: IncludePaths, extra: IncludePaths) -> IncludePaths {
    let mut seen: HashSet<PathBuf> = paths.iter().cloned().collect();
    for p in extra {
        if seen.insert(p.clone()) {
            paths.push(p);
        }
    }
    paths
}

/// Used by the non-MSVC probe path and by feature `bindgen` (vendor headers).
fn glib_include_paths() -> IncludePaths {
    // GObject's .pc file already requires GLib; one probe supplies both.
    pkg_config::Config::new()
        .probe("gobject-2.0")
        .map(|lib| lib.include_paths)
        .unwrap_or_default()
}

fn find_libvips_msvc(options: &BuildOptions) -> ProbeResult {
    let mut config = windows::config(options.static_linking);
    if let Ok(root) = vcpkg::find_vcpkg_root(&config) {
        for path in [
            root.join("installed/vcpkg/status"),
            root.join("installed/vcpkg/updates"),
        ] {
            println!("cargo:rerun-if-changed={}", path.display());
        }
    }
    match windows::probe(&mut config) {
        Ok(lib) => {
            // vcpkg already resolved the selected triplet. Do not guess another
            // installation's headers or merge in an unrelated vendor version.
            let version = options.version.clone().or_else(|| {
                lib.include_paths.iter().find_map(|include| {
                    let header = include.join("vips/version.h");
                    if header.is_file() {
                        println!("cargo:rerun-if-changed={}", header.display());
                        fs::read_to_string(header)
                            .ok()
                            .and_then(|text| config::header_version(&text))
                    } else {
                        None
                    }
                })
            });
            Some(LibraryInfo {
                include_paths: windows::include_paths(lib),
                defines: Vec::new(),
                version,
            })
        }
        Err(err) => {
            println!(
                "cargo:warning=vips-sys: vcpkg could not find the libvips port ({err}); \
                 install `libvips` for the selected triplet; dynamic builds also need VCPKGRS_DYNAMIC=1. Set VCPKG_ROOT or LIBVIPS_LIB_DIR if needed"
            );
            // A header directory alone is not a linkable Windows library.
            None
        }
    }
}

fn find_libvips(options: &BuildOptions) -> ProbeResult {
    // An explicit installation is the common fast path for every target.
    if let Some(lib_dir) = &options.lib_dir {
        // Keep the installation's complete native dependency metadata when
        // available, including GObject symbols used by the safe wrapper.
        if !options.msvc
            && let Some(info) = unix::explicit(options, lib_dir)
        {
            return Some(info);
        }
        println!("cargo:rustc-link-search=native={}", lib_dir.display());
        emit_link(if options.static_linking {
            "static"
        } else {
            "dylib"
        });
        let mut include_paths = options
            .include_dir
            .clone()
            .map(|p| vec![p])
            .unwrap_or_default();
        if options.generate_bindings && include_paths.is_empty() {
            include_paths = vendor_include_paths(options).unwrap_or_default();
        }
        // The safe wrapper also calls GObject helpers directly. Their link
        // metadata is required even when bindings are pre-generated.
        if !options.msvc {
            include_paths = merge_includes(include_paths, glib_include_paths());
        }
        return Some(LibraryInfo {
            include_paths,
            defines: Vec::new(),
            version: options.version.clone(),
        });
    }
    if options.msvc {
        find_libvips_msvc(options)
    } else {
        unix::find(options)
    }
}

/// Generate bindings with bindgen (feature `bindgen` only).
/// Default builds use `src/bindings/prebuilt.rs` and skip this path entirely.
#[cfg(feature = "bindgen")]
fn generate_bindings(include_paths: &[PathBuf], defines: &Defines) {
    println!("cargo:rerun-if-changed=wrapper.h");
    println!("cargo:rerun-if-env-changed=LIBVIPS_NO_BINDGEN");
    println!("cargo:rerun-if-env-changed=LIBVIPS_LIB_DIR");
    println!("cargo:rerun-if-env-changed=LIBVIPS_INCLUDE_DIR");
    println!("cargo:rerun-if-env-changed=LIBVIPS_STATIC");
    println!("cargo:rerun-if-env-changed=LIBVIPS_NO_VENDOR");
    println!("cargo:rerun-if-env-changed=VCPKG_ROOT");

    let out = PathBuf::from(env::var_os("OUT_DIR").expect("OUT_DIR"));
    let binding_rs = out.join("binding.rs");
    if env::var_os("LIBVIPS_NO_BINDGEN").is_some() {
        fs::copy("src/bindings/prebuilt.rs", &binding_rs).expect("copy pregenerated bindings");
        return;
    }

    // CargoCallbacks registers every included header. Let Cargo track those
    // dependencies rather than skipping a rebuild with a partial fingerprint.
    let mut builder = bindgen::Builder::default()
        .header("wrapper.h")
        .parse_callbacks(Box::new(bindgen::CargoCallbacks::new()))
        // Allowlist keeps clang work and generated Rust small: only the libvips
        // surface plus the few GObject helpers the high-level crate needs.
        .allowlist_function("vips_.*")
        .allowlist_type("Vips.*")
        .allowlist_var("VIPS_.*")
        .allowlist_function("g_object_unref")
        .allowlist_function("g_object_ref")
        .allowlist_function("g_free")
        .allowlist_function("g_signal_connect_data")
        .allowlist_type("GConnectFlags")
        .use_core()
        .ctypes_prefix("core::ffi")
        // Only allowlisted enums are rustified; keep GConnectFlags as an enum
        // because high-level crates match on G_CONNECT_* variants.
        .rustified_enum(".*")
        .layout_tests(false)
        .generate_comments(false)
        .blocklist_type("max_align_t")
        .blocklist_item("FP_NAN")
        .blocklist_item("FP_INFINITE")
        .blocklist_item("FP_ZERO")
        .blocklist_item("FP_SUBNORMAL")
        .blocklist_item("FP_NORMAL");

    for p in include_paths {
        builder = builder.clang_arg(format!("-I{}", p.display()));
    }
    for (name, val) in defines {
        match val {
            Some(v) => builder = builder.clang_arg(format!("-D{}={}", name, v)),
            None => builder = builder.clang_arg(format!("-D{}", name)),
        }
    }

    let bindings = builder
        .generate()
        .expect("Unable to generate libvips bindings");

    // Normalize in memory and write once; unchanged contents retain their mtime.
    {
        let body = bindings.to_string();
        let header = concat!(
            "/* automatically generated by rust-bindgen (vips-sys)\n",
            " *\n",
            " * Pregenerated FFI declarations for the public libvips C API.\n",
            " * Part of vips-sys (MIT). The libvips library itself is LGPL-2.1;\n",
            " * linking/distributing against it requires LGPL-2.1 compliance.\n",
            " * See README “License”.\n",
            " */\n\n"
        );
        // Drop bindgen's own short banner if present, keep one notice.
        let body = body
            .strip_prefix("/* automatically generated by rust-bindgen")
            .and_then(|s| s.find("*/").map(|i| &s[i + 2..]))
            .map(|s| s.trim_start_matches('\n'))
            .unwrap_or(&body);
        // Normalize after generation so bindgen can still derive Copy/Clone
        // using the known scalar typedefs. GLib sizes are pointer-sized on
        // LLP64 Windows as well as Unix; C long does not have that guarantee.
        let mut normalized = String::with_capacity(body.len());
        for line in body.lines() {
            normalized.push_str(if line.starts_with("pub type gsize = ") {
                "pub type gsize = usize;"
            } else if line.starts_with("pub type gssize = ") {
                "pub type gssize = isize;"
            } else {
                line
            });
            normalized.push('\n');
        }
        let output = format!("{header}{normalized}");
        if fs::read_to_string(&binding_rs).ok().as_deref() != Some(output.as_str()) {
            fs::write(&binding_rs, output).expect("Couldn't write bindings");
        }
    }

    // Convenience for maintainers: print where to copy when refreshing prebuilt.rs
    println!(
        "cargo:warning=vips-sys: bindings ready at {} — use scripts/refresh-prebuilt.py to refresh defaults",
        binding_rs.display()
    );
}

fn apply_version_cfg(version: &str) {
    println!("cargo:rustc-env=LIBVIPS_VERSION={}", version);
    let mut parts = version.split('.');
    if let (Some(major), Some(minor)) = (parts.next(), parts.next())
        && let (Ok(major), Ok(minor)) = (major.parse::<u32>(), minor.parse::<u32>())
    {
        if cfg!(feature = "v8_18") && (major, minor) < (8, 18) {
            panic!("vips-sys: feature v8_18 requires libvips >= 8.18, found {version}");
        }
        if major > 8 || (major == 8 && minor >= 18) {
            println!("cargo:rustc-cfg=vips_8_18");
        }
        if major > 8 || (major == 8 && minor >= 17) {
            println!("cargo:rustc-cfg=vips_8_17");
        }
        if major > 8 || (major == 8 && minor >= 16) {
            println!("cargo:rustc-cfg=vips_8_16");
        }
    }
}

fn main() {
    println!("cargo:rerun-if-changed=build.rs");
    println!("cargo:rerun-if-changed=build_support/windows.rs");
    println!("cargo:rerun-if-changed=build_support/config.rs");
    println!("cargo:rerun-if-changed=build_support/unix.rs");
    for variable in [
        "MESON_PATH",
        "NINJA_PATH",
        "CXXSTDLIB",
        "VCPKGRS_TRIPLET",
        "VCPKGRS_DYNAMIC",
        "VCPKGRS_DISABLE",
        "NO_VCPKG",
        "VCPKGRS_NO_LIBVIPS",
        "LIBVIPS_NO_VCPKG",
    ] {
        println!("cargo:rerun-if-env-changed={variable}");
    }
    println!("cargo:rerun-if-env-changed=LIBVIPS_NO_VENDOR");
    println!("cargo:rerun-if-env-changed=LIBVIPS_LIB_DIR");
    println!("cargo:rerun-if-env-changed=LIBVIPS_INCLUDE_DIR");
    println!("cargo:rerun-if-env-changed=LIBVIPS_STATIC");
    println!("cargo:rerun-if-env-changed=PKG_CONFIG_PATH");
    println!("cargo:rerun-if-env-changed=VCPKG_ROOT");
    println!("cargo:rerun-if-env-changed=VCPKG_DEFAULT_TRIPLET");
    println!("cargo:rerun-if-env-changed=LIBVIPS_VERSION");
    println!("cargo:rerun-if-changed=stub/vips_stub.c");

    for cfg in ["vips_8_16", "vips_8_17", "vips_8_18"] {
        println!("cargo:rustc-check-cfg=cfg({cfg})");
    }
    println!("cargo:rerun-if-changed=src/bindings/prebuilt.rs");
    let options = BuildOptions::from_env();
    let found = find_libvips(&options);

    let LibraryInfo {
        include_paths,
        defines,
        version,
    } = match found {
        Some(library) => library,
        None => {
            // Prefer a real library whenever find_libvips succeeds; only fall
            // through here when nothing is installed.
            #[cfg(feature = "stub")]
            {
                println!(
                    "cargo:warning=vips-sys: no libvips library; linking stub/vips_stub.c \
                     (feature `stub`). Suitable for unit tests only — not for production."
                );
                cc::Build::new()
                    .file("stub/vips_stub.c")
                    .warnings(false)
                    .compile("vips");
            }
            #[cfg(not(feature = "stub"))]
            {
                #[cfg(not(feature = "bindgen"))]
                {
                    println!(
                        "cargo:warning=vips-sys: libvips not found at build time. \
                         Using pregenerated bindings; install libvips before linking binaries \
                         (pkg-config/vcpkg or LIBVIPS_LIB_DIR), or enable feature `stub` for tests."
                    );
                    emit_link(if options.static_linking {
                        "static"
                    } else {
                        "dylib"
                    });
                }
            }

            #[cfg(feature = "bindgen")]
            {
                if env::var_os("LIBVIPS_NO_BINDGEN").is_some() {
                    // This mode copies prebuilt.rs and needs no headers.
                    LibraryInfo::default()
                } else {
                    // Headers for bindgen: vendor tree or nothing.
                    let mut inc = vendor_include_paths(&options).unwrap_or_default();
                    if inc.is_empty() {
                        panic!(
                            "vips-sys: libvips headers not found (feature `bindgen`).\n\
                         - Unix/macOS: install libvips-dev and pkg-config\n\
                         - Windows MSVC: `vcpkg install libvips:x64-windows` (set VCPKG_ROOT)\n\
                         - Or set LIBVIPS_LIB_DIR / LIBVIPS_INCLUDE_DIR\n\
                         - Or `git submodule update --init` for vendor/libvips"
                        );
                    }
                    inc = merge_includes(inc, glib_include_paths());
                    LibraryInfo {
                        include_paths: inc,
                        ..LibraryInfo::default()
                    }
                }
            }
            #[cfg(not(feature = "bindgen"))]
            {
                LibraryInfo::default()
            }
        }
    };

    if let Some(v) = version.as_deref() {
        apply_version_cfg(v);
    }

    if !include_paths.is_empty() {
        let joined = env::join_paths(&include_paths).expect("valid include paths");
        println!("cargo:include={}", joined.to_string_lossy());
    }

    // Default builds compile `src/bindings/prebuilt.rs` — no clang required.
    #[cfg(feature = "bindgen")]
    generate_bindings(&include_paths, &defines);

    // Silence unused-parameter warnings when bindgen is disabled.
    #[cfg(not(feature = "bindgen"))]
    let _ = (&include_paths, &defines);
}
