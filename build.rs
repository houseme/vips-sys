// SPDX-License-Identifier: MIT
//
// Build script for vips-sys (MIT).
// Links against libvips, which is LGPL-2.1 — see README “License”.

use std::{collections::HashSet, env, fs, path::PathBuf, process::Command};

type IncludePaths = Vec<PathBuf>;
type Defines = Vec<(String, Option<String>)>;
type VersionOpt = Option<String>;
type ProbeResult = Option<(IncludePaths, Defines, VersionOpt)>;

/// Bump when bindgen options / allowlist change so cached bindings are invalidated.
#[cfg(feature = "bindgen")]
const BINDINGS_SCHEMA: &str = "vips-sys-bindings-v2";

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
    if paths.is_empty() {
        None
    } else {
        Some(paths)
    }
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

#[cfg(target_env = "msvc")]
fn find_libvips() -> ProbeResult {
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

    let mut config = vcpkg::Config::new();
    if prefer_static() {
        if let Ok(triplet) = env::var("VCPKG_DEFAULT_TRIPLET") {
            config = config.target_triplet(&triplet);
        } else {
            config = config.target_triplet("x64-windows-static");
        }
    }
    match config.find_package("vips") {
        Ok(_lib) => {
            let mut includes = Vec::new();
            if let Ok(inc) = env::var("DEP_VIPS_INCLUDE") {
                for p in env::split_paths(&inc) {
                    includes.push(p);
                }
            }
            if let Ok(vcpkg_root) = env::var("VCPKG_ROOT") {
                let base = PathBuf::from(vcpkg_root).join("installed");
                let triplets = if prefer_static() {
                    vec!["x64-windows-static", "x64-windows"]
                } else {
                    vec!["x64-windows", "x64-windows-static"]
                };
                for triplet in triplets {
                    let p = base.join(triplet).join("include");
                    if p.is_dir() {
                        includes.push(p);
                        break;
                    }
                }
            }
            includes = merge_includes(includes, vendor_include_paths().unwrap_or_default());
            Some((includes, Vec::new(), None))
        }
        Err(err) => {
            println!(
                "cargo:warning=vips-sys: vcpkg could not find vips ({err}); \
                 install with `vcpkg install vips` or set LIBVIPS_LIB_DIR"
            );
            let includes = vendor_include_paths().unwrap_or_default();
            if includes.is_empty() {
                None
            } else {
                emit_link(if prefer_static() { "static" } else { "dylib" });
                Some((includes, Vec::new(), None))
            }
        }
    }
}

#[cfg(not(target_env = "msvc"))]
fn find_libvips() -> ProbeResult {
    // 1) Explicit env override
    if let Some(lib_dir) = env_path("LIBVIPS_LIB_DIR") {
        println!("cargo:rustc-link-search=native={}", lib_dir.display());
        emit_link(if prefer_static() { "static" } else { "dylib" });
        let include_paths = env_path("LIBVIPS_INCLUDE_DIR")
            .map(|p| vec![p])
            .or_else(vendor_include_paths)
            .unwrap_or_default();
        let include_paths = merge_includes(include_paths, glib_include_paths());
        let version = env::var("LIBVIPS_VERSION").ok();
        return Some((include_paths, Vec::new(), version));
    }

    // 2) System library via pkg-config (fast path — covers include dirs for glib too)
    let mut cfg = pkg_config::Config::new();
    if prefer_static() {
        cfg.statik(true);
    }
    if cfg!(feature = "dynamic") {
        cfg.statik(false);
    }
    if let Ok(lib) = cfg.atleast_version("8.2").probe("vips") {
        let include_paths = merge_includes(
            lib.include_paths,
            vendor_include_paths().unwrap_or_default(),
        );
        return Some((include_paths, Vec::new(), Some(lib.version.clone())));
    }

    // 3) Build static libvips from the vendored submodule
    if prefer_static()
        && let Some((includes, lib_dir)) = try_build_vendor_static()
    {
        println!("cargo:rustc-link-search=native={}", lib_dir.display());
        emit_link("static");
        return Some((includes, Vec::new(), None));
    }

    // 4) Vendored headers only
    if let Some(include_paths) = vendor_include_paths() {
        println!(
            "cargo:warning=vips-sys: using vendored libvips headers from vendor/libvips; \
             install a libvips library, enable static with meson/ninja, or set LIBVIPS_LIB_DIR"
        );
        emit_link(if prefer_static() { "static" } else { "dylib" });
        let include_paths = merge_includes(include_paths, glib_include_paths());
        return Some((include_paths, Vec::new(), None));
    }

    None
}

/// Generate bindings with bindgen (feature `bindgen` only).
/// Default builds use `src/bindings/prebuilt.rs` and skip this path entirely.
#[cfg(feature = "bindgen")]
fn generate_bindings(include_paths: &[PathBuf], defines: &Defines, version: Option<&str>) {
    println!("cargo:rerun-if-changed=wrapper.h");
    println!("cargo:rerun-if-env-changed=LIBVIPS_NO_BINDGEN");
    println!("cargo:rerun-if-env-changed=LIBVIPS_LIB_DIR");
    println!("cargo:rerun-if-env-changed=LIBVIPS_INCLUDE_DIR");
    println!("cargo:rerun-if-env-changed=LIBVIPS_STATIC");
    println!("cargo:rerun-if-env-changed=LIBVIPS_NO_VENDOR");
    println!("cargo:rerun-if-env-changed=VCPKG_ROOT");
    println!("cargo:rerun-if-changed=vendor/libvips");

    if env::var_os("LIBVIPS_NO_BINDGEN").is_some() {
        return;
    }

    let out = PathBuf::from(env::var_os("OUT_DIR").expect("OUT_DIR"));
    let binding_rs = out.join("binding.rs");
    let stamp = out.join("binding.fingerprint");
    let fp = bindings_fingerprint(include_paths, version);

    // Skip the expensive clang/bindgen pass when inputs are unchanged.
    if binding_rs.is_file()
        && let Ok(prev) = fs::read_to_string(&stamp)
        && prev.trim() == fp
    {
        return;
    }

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
        let _ = fs::write(&binding_rs, format!("{header}{body}"));
    }
    let _ = fs::write(&stamp, &fp);

    // Convenience for maintainers: print where to copy when refreshing prebuilt.rs
    println!(
        "cargo:warning=vips-sys: wrote {} — copy to src/bindings/prebuilt.rs to refresh defaults",
        binding_rs.display()
    );
}

/// Fingerprint helper is only needed when regenerating bindings.
#[cfg(feature = "bindgen")]
fn bindings_fingerprint(include_paths: &[PathBuf], version: Option<&str>) -> String {
    let mut acc = String::with_capacity(256);
    acc.push_str(BINDINGS_SCHEMA);
    acc.push('\n');
    acc.push_str(version.unwrap_or("unknown"));
    acc.push('\n');
    for p in include_paths {
        acc.push_str(&p.display().to_string());
        acc.push('\n');
    }
    if let Ok(w) = fs::read_to_string("wrapper.h") {
        acc.push_str(&w);
    }
    let mut hash: u64 = 0xcbf29ce484222325;
    for b in acc.as_bytes() {
        hash ^= *b as u64;
        hash = hash.wrapping_mul(0x100000001b3);
    }
    format!("{hash:016x}")
}

fn apply_version_cfg(version: &str) {
    println!("cargo:rustc-env=LIBVIPS_VERSION={}", version);
    let ver_parts: Vec<_> = version.split('.').collect();
    if ver_parts.len() >= 2
        && let (Ok(major), Ok(minor)) = (ver_parts[0].parse::<u32>(), ver_parts[1].parse::<u32>())
    {
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
    println!("cargo:rerun-if-env-changed=LIBVIPS_NO_VENDOR");
    println!("cargo:rerun-if-env-changed=VCPKG_ROOT");
    println!("cargo:rerun-if-env-changed=VCPKG_DEFAULT_TRIPLET");
    println!("cargo:rerun-if-env-changed=LIBVIPS_VERSION");

    let (include_paths, defines, version) = find_libvips().expect(
        "vips-sys: libvips not found.\n\
         - Unix/macOS: install libvips-dev and pkg-config\n\
         - Windows MSVC: `vcpkg install vips` (set VCPKG_ROOT)\n\
         - Static: enable `static` feature or LIBVIPS_STATIC=1\n\
         - Or set LIBVIPS_LIB_DIR / LIBVIPS_INCLUDE_DIR\n\
         - Or `git submodule update --init` for vendor/libvips",
    );

    if let Some(v) = version.as_deref() {
        apply_version_cfg(v);
    }

    println!(
        "cargo:include={}",
        include_paths
            .iter()
            .map(|p| p.display().to_string())
            .collect::<Vec<_>>()
            .join(":")
    );

    // Default builds compile `src/bindings/prebuilt.rs` — no clang required.
    #[cfg(feature = "bindgen")]
    generate_bindings(&include_paths, &defines, version.as_deref());

    // Silence unused-parameter warnings when bindgen is disabled.
    #[cfg(not(feature = "bindgen"))]
    let _ = (&include_paths, &defines);
}
