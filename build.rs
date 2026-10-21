// SPDX-License-Identifier: MIT
//
// Build script for vips-sys (MIT).
// Links against libvips, which is LGPL-2.1 — see README “License”.

use std::fs;
use std::process::Command;
use std::{collections::HashSet, env, path::PathBuf};

#[path = "build_support/windows.rs"]
mod windows;

type IncludePaths = Vec<PathBuf>;
type Defines = Vec<(String, Option<String>)>;
type VersionOpt = Option<String>;
type ProbeResult = Option<(IncludePaths, Defines, VersionOpt)>;

fn env_path(name: &str) -> Option<PathBuf> {
    env::var_os(name).map(PathBuf::from)
}

/// Prefer static linking when `LIBVIPS_STATIC` is set or the `static` feature is
/// enabled without `dynamic`. Env var wins over Cargo features (sys-crate convention).
fn prefer_static() -> bool {
    if let Ok(v) = env::var("LIBVIPS_STATIC") {
        let v = v.to_ascii_lowercase();
        return v != "0" && v != "false" && v != "off";
    }
    cfg!(feature = "static") && !cfg!(feature = "dynamic")
}

fn vendor_root() -> Option<PathBuf> {
    if env::var_os("LIBVIPS_NO_VENDOR").is_some() {
        return None;
    }
    let root = PathBuf::from(env::var_os("CARGO_MANIFEST_DIR")?).join("vendor/libvips");
    if root.is_dir() && root.join("meson.build").is_file() {
        Some(root)
    } else {
        None
    }
}

fn vendor_include_paths() -> Option<IncludePaths> {
    let root = vendor_root()?;
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
    let mut paths = Vec::new();
    if let Ok(glib) = pkg_config::Config::new().probe("glib-2.0") {
        paths = merge_includes(paths, glib.include_paths);
    }
    if let Ok(gobject) = pkg_config::Config::new().probe("gobject-2.0") {
        paths = merge_includes(paths, gobject.include_paths);
    }
    paths
}

/// meson/ninja helpers — Unix static vendor builds only.
fn which(tool: &str) -> Option<PathBuf> {
    if let Ok(path) = env::var(format!("{}_PATH", tool.to_ascii_uppercase())) {
        let p = PathBuf::from(path);
        if p.is_file() {
            return Some(p);
        }
    }
    let status = Command::new(tool).arg("--version").output().ok()?;
    if status.status.success() {
        Some(PathBuf::from(tool))
    } else {
        None
    }
}

/// Optional meson-based static build from `vendor/libvips`.
fn try_build_vendor_static() -> Option<(IncludePaths, PathBuf)> {
    let root = vendor_root()?;
    let out_dir = PathBuf::from(env::var_os("OUT_DIR")?).join("libvips-build");
    let prefix = out_dir.join("install");
    let lib_dir = prefix.join("lib");
    let archive = lib_dir.join("libvips.a");
    let archive_alt = lib_dir.join("libvips.lib");

    if archive.is_file() || archive_alt.is_file() {
        return Some((static_install_includes(&prefix), lib_dir));
    }

    if which("meson").is_none() || which("ninja").is_none() {
        println!(
            "cargo:warning=vips-sys: meson/ninja not found; cannot build static libvips from vendor"
        );
        return None;
    }

    fs::create_dir_all(&out_dir).ok()?;
    let builddir = out_dir.join("builddir");
    let status = Command::new("meson")
        .args([
            "setup",
            builddir.to_str()?,
            root.to_str()?,
            "--prefix",
            prefix.to_str()?,
            "--default-library=static",
            "--buildtype=release",
            "-Dintrospection=disabled",
            "-Dmodules=disabled",
            "-Ddeprecated=false",
        ])
        .status()
        .ok()?;
    if !status.success() {
        println!("cargo:warning=vips-sys: meson setup failed for vendored libvips");
        return None;
    }

    for args in [
        vec!["-C", builddir.to_str()?],
        vec!["-C", builddir.to_str()?, "install"],
    ] {
        let status = Command::new("ninja").args(&args).status().ok()?;
        if !status.success() {
            println!("cargo:warning=vips-sys: ninja step failed for vendored libvips");
            return None;
        }
    }

    if archive.is_file() || archive_alt.is_file() {
        Some((static_install_includes(&prefix), lib_dir))
    } else {
        None
    }
}

fn static_install_includes(prefix: &std::path::Path) -> IncludePaths {
    let mut includes = vec![prefix.join("include")];
    includes = merge_includes(includes, vendor_include_paths().unwrap_or_default());
    merge_includes(includes, glib_include_paths())
}

fn find_libvips_msvc() -> ProbeResult {
    if let Some(lib_dir) = env_path("LIBVIPS_LIB_DIR") {
        let include = env_path("LIBVIPS_INCLUDE_DIR")
            .map(|p| vec![p])
            .or_else(vendor_include_paths)
            .unwrap_or_default();
        println!("cargo:rustc-link-search=native={}", lib_dir.display());
        emit_link(if prefer_static() { "static" } else { "dylib" });
        let version = env::var("LIBVIPS_VERSION").ok();
        return Some((include, Vec::new(), version));
    }

    let mut config = windows::config(prefer_static());
    match windows::probe(&mut config) {
        Ok(lib) => {
            // vcpkg already resolved the selected triplet. Do not guess another
            // installation's headers or merge in an unrelated vendor version.
            Some((
                windows::include_paths(&lib),
                Vec::new(),
                env::var("LIBVIPS_VERSION").ok(),
            ))
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

fn find_libvips_unix() -> ProbeResult {
    // 1) Explicit env override
    if let Some(lib_dir) = env_path("LIBVIPS_LIB_DIR") {
        println!("cargo:rustc-link-search=native={}", lib_dir.display());
        emit_link(if prefer_static() { "static" } else { "dylib" });
        let include_paths = env_path("LIBVIPS_INCLUDE_DIR")
            .map(|p| vec![p])
            .or_else(vendor_include_paths)
            .unwrap_or_default();
        // glib headers are only required when bindgen parses vendor/system headers.
        #[cfg(feature = "bindgen")]
        let include_paths = merge_includes(include_paths, glib_include_paths());
        let version = env::var("LIBVIPS_VERSION").ok();
        return Some((include_paths, Vec::new(), version));
    }

    // 2) System library via pkg-config (fast path)
    //    Do not merge vendor headers here: system includes match the linked
    //    library, and skipping FS probes keeps the build script cheap.
    let mut cfg = pkg_config::Config::new();
    if prefer_static() {
        cfg.statik(true);
    }
    if let Ok(lib) = cfg
        .atleast_version(if cfg!(feature = "v8_18") {
            "8.18"
        } else {
            "8.2"
        })
        .probe("vips")
    {
        return Some((lib.include_paths, Vec::new(), Some(lib.version.clone())));
    }

    // 3) Build static libvips from the vendored submodule
    if prefer_static()
        && let Some((includes, lib_dir)) = try_build_vendor_static()
    {
        println!("cargo:rustc-link-search=native={}", lib_dir.display());
        emit_link("static");
        return Some((includes, Vec::new(), None));
    }

    // 4) No linkable library. Headers alone are not enough to satisfy the linker.
    //    Return None so main() can pick stub / warn / bindgen-only as appropriate.
    None
}

fn find_libvips() -> ProbeResult {
    // Build-script cfg describes HOST. Dependency discovery must follow TARGET.
    if env::var("CARGO_CFG_TARGET_ENV").as_deref() == Ok("msvc") {
        find_libvips_msvc()
    } else {
        find_libvips_unix()
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
    println!("cargo:rerun-if-changed=vendor/libvips");

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

    bindings
        .write_to_file(&binding_rs)
        .expect("Couldn't write bindings");
    // Prepend license notice so copies into src/bindings/prebuilt.rs keep it.
    if let Ok(body) = fs::read_to_string(&binding_rs) {
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
        fs::write(&binding_rs, format!("{header}{normalized}"))
            .expect("Couldn't normalize bindings");
    }

    // Convenience for maintainers: print where to copy when refreshing prebuilt.rs
    println!(
        "cargo:warning=vips-sys: wrote {} — copy to src/bindings/prebuilt.rs to refresh defaults",
        binding_rs.display()
    );
}

fn apply_version_cfg(version: &str) {
    println!("cargo:rustc-env=LIBVIPS_VERSION={}", version);
    let ver_parts: Vec<_> = version.split('.').collect();
    if ver_parts.len() >= 2
        && let (Ok(major), Ok(minor)) = (ver_parts[0].parse::<u32>(), ver_parts[1].parse::<u32>())
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
    for variable in [
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
    let found = find_libvips();

    let (include_paths, defines, version) = match found {
        Some(triple) => triple,
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
                    emit_link(if prefer_static() { "static" } else { "dylib" });
                }
            }

            #[cfg(feature = "bindgen")]
            {
                if env::var_os("LIBVIPS_NO_BINDGEN").is_some() {
                    // This mode copies prebuilt.rs and needs no headers.
                    (Vec::new(), Vec::new(), None)
                } else {
                    // Headers for bindgen: vendor tree or nothing.
                    let mut inc = vendor_include_paths().unwrap_or_default();
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
                    (inc, Vec::new(), None)
                }
            }
            #[cfg(not(feature = "bindgen"))]
            {
                (Vec::new(), Vec::new(), None)
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
