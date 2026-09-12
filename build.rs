use std::{env, path::PathBuf, process::Command};

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
    if let Some(v) = env::var("LIBVIPS_STATIC") {
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
        println!("cargo:rustc-link-lib=vips");
    }
}

fn merge_includes(mut paths: IncludePaths, extra: IncludePaths) -> IncludePaths {
    for p in extra {
        if !paths.contains(&p) {
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

/// Optional meson-based static build from `vendor/libvips`.
/// Returns Some((include_paths, lib_dir)) when a static archive is produced.
fn try_build_vendor_static() -> Option<(IncludePaths, PathBuf)> {
    let root = vendor_root()?;
    let out_dir = PathBuf::from(env::var_os("OUT_DIR")?).join("libvips-build");
    let prefix = out_dir.join("install");
    let lib_dir = prefix.join("lib");
    let archive = lib_dir.join("libvips.a");
    let archive_alt = lib_dir.join("libvips.lib");

    if archive.is_file() || archive_alt.is_file() {
        let include = prefix.join("include");
        let mut includes = vec![include];
        includes = merge_includes(includes, vendor_include_paths().unwrap_or_default());
        includes = merge_includes(includes, glib_include_paths());
        return Some((includes, lib_dir));
    }

    if which("meson").is_none() || which("ninja").is_none() {
        println!(
            "cargo:warning=vips-sys: meson/ninja not found; cannot build static libvips from vendor"
        );
        return None;
    }

    std::fs::create_dir_all(&out_dir).ok()?;
    let status = Command::new("meson")
        .args([
            "setup",
            out_dir.join("builddir").to_str()?,
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

    let status = Command::new("ninja")
        .args(["-C", out_dir.join("builddir").to_str()?])
        .status()
        .ok()?;
    if !status.success() {
        println!("cargo:warning=vips-sys: ninja build failed for vendored libvips");
        return None;
    }

    let status = Command::new("ninja")
        .args(["-C", out_dir.join("builddir").to_str()?, "install"])
        .status()
        .ok()?;
    if !status.success() {
        println!("cargo:warning=vips-sys: ninja install failed for vendored libvips");
        return None;
    }

    if archive.is_file() || archive_alt.is_file() {
        let include = prefix.join("include");
        let mut includes = vec![include];
        includes = merge_includes(includes, vendor_include_paths().unwrap_or_default());
        includes = merge_includes(includes, glib_include_paths());
        Some((includes, lib_dir))
    } else {
        None
    }
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

#[cfg(target_env = "msvc")]
fn find_libvips() -> ProbeResult {
    // 1) Env overrides (work for both static and dynamic prebuilt trees)
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

    // 2) vcpkg (recommended Windows path; set VCPKG_ROOT, use triplet x64-windows / x64-windows-static)
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
            // vcpkg crate already emits cargo:rustc-link-* metadata
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

    // 2) System library via pkg-config
    //    `.statik(true)` asks pkg-config for `--static` libs and dependency flags.
    let mut cfg = pkg_config::Config::new();
    if prefer_static() {
        cfg.statik(true);
    }
    if cfg!(feature = "dynamic") {
        cfg.statik(false);
    }
    if let Ok(lib) = cfg.atleast_version("8.2").probe("vips") {
        let include_paths = merge_includes(lib.include_paths, vendor_include_paths().unwrap_or_default());
        return Some((include_paths, Vec::new(), Some(lib.version.clone())));
    }

    // 3) Build static libvips from the vendored submodule (needs meson + ninja)
    if prefer_static() {
        if let Some((includes, lib_dir)) = try_build_vendor_static() {
            println!("cargo:rustc-link-search=native={}", lib_dir.display());
            emit_link("static");
            return Some((includes, Vec::new(), None));
        }
    }

    // 4) Vendored headers only (link still needs a built/installed libvips)
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

fn generate_bindings(include_paths: &[PathBuf], defines: &Defines) {
    println!("cargo:rerun-if-changed=wrapper.h");
    println!("cargo:rerun-if-changed=build.rs");
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

    let mut builder = bindgen::Builder::default()
        .header("wrapper.h")
        .parse_callbacks(Box::new(bindgen::CargoCallbacks::new()))
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

    let out = PathBuf::from(env::var_os("OUT_DIR").unwrap());
    bindings
        .write_to_file(out.join("binding.rs"))
        .expect("Couldn't write bindings");
}

fn apply_version_cfg(version: &str) {
    println!("cargo:rustc-env=LIBVIPS_VERSION={}", version);
    let ver_parts: Vec<_> = version.split('.').collect();
    if ver_parts.len() >= 2 {
        if let (Ok(major), Ok(minor)) = (ver_parts[0].parse::<u32>(), ver_parts[1].parse::<u32>()) {
            if major > 8 || (major == 8 && minor >= 17) {
                println!("cargo:rustc-cfg=vips_8_17");
            }
        }
    }
}

fn main() {
    println!("cargo:rerun-if-env-changed=LIBVIPS_NO_VENDOR");
    println!("cargo:rerun-if-env-changed=VCPKG_ROOT");
    println!("cargo:rerun-if-env-changed=VCPKG_DEFAULT_TRIPLET");

    let (include_paths, defines, version) = find_libvips().expect(
        "vips-sys: libvips not found.\n\
         - Unix/macOS: install libvips-dev and pkg-config\n\
         - Windows MSVC: `vcpkg install vips` (set VCPKG_ROOT)\n\
         - Static: enable `static` feature or LIBVIPS_STATIC=1\n\
         - Or set LIBVIPS_LIB_DIR / LIBVIPS_INCLUDE_DIR\n\
         - Or `git submodule update --init` for vendor/libvips",
    );

    if let Some(v) = version {
        apply_version_cfg(&v);
    }

    println!(
        "cargo:include={}",
        include_paths
            .iter()
            .map(|p| p.display().to_string())
            .collect::<Vec<_>>()
            .join(":")
    );

    generate_bindings(&include_paths, &defines);
}
