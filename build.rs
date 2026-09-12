use std::{env, path::PathBuf};

type IncludePaths = Vec<PathBuf>;
type Defines = Vec<(String, Option<String>)>;
type VersionOpt = Option<String>;
type ProbeResult = Option<(IncludePaths, Defines, VersionOpt)>;

fn env_path(name: &str) -> Option<PathBuf> {
    env::var_os(name).map(PathBuf::from)
}

fn prefer_static() -> bool {
    if env::var_os("LIBVIPS_STATIC").is_some() {
        return true;
    }
    cfg!(feature = "static") && !cfg!(feature = "dynamic")
}

fn vendor_include_paths() -> Option<IncludePaths> {
    if env::var_os("LIBVIPS_NO_VENDOR").is_some() {
        return None;
    }
    let root = PathBuf::from(env::var_os("CARGO_MANIFEST_DIR")?).join("vendor/libvips");
    if !root.is_dir() {
        return None;
    }

    let mut paths = Vec::new();
    // meson-style tree: vendor/libvips/libvips/include + generated config under build
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

#[cfg(target_env = "msvc")]
fn find_libvips() -> ProbeResult {
    // Env overrides first (sys-crate convention)
    if let (Some(lib_dir), Some(inc_dir)) = (env_path("LIBVIPS_LIB_DIR"), env_path("LIBVIPS_INCLUDE_DIR")) {
        println!("cargo:rustc-link-search=native={}", lib_dir.display());
        let kind = if prefer_static() { "static" } else { "dylib" };
        println!("cargo:rustc-link-lib={}=vips", kind);
        return Some((vec![inc_dir], Vec::new(), env_path("LIBVIPS_VERSION").and_then(|v| v.into_string().ok())));
    }

    // vcpkg for MSVC
    let _lib = vcpkg::find_package("vips").ok()?;
    let mut include_paths = vendor_include_paths().unwrap_or_default();
    if include_paths.is_empty() {
        // leave bindgen to system headers via INCLUDE if present
    }
    Some((include_paths, Vec::new(), None))
}

#[cfg(not(target_env = "msvc"))]
fn find_libvips() -> ProbeResult {
    // 1) Explicit env override
    if let Some(lib_dir) = env_path("LIBVIPS_LIB_DIR") {
        println!("cargo:rustc-link-search=native={}", lib_dir.display());
        let kind = if prefer_static() { "static" } else { "dylib" };
        println!("cargo:rustc-link-lib={}=vips", kind);
        let include_paths = env_path("LIBVIPS_INCLUDE_DIR")
            .map(|p| vec![p])
            .or_else(vendor_include_paths)
            .unwrap_or_default();
        let version = env_path("LIBVIPS_VERSION").and_then(|v| v.into_string().ok());
        return Some((include_paths, Vec::new(), version));
    }

    // 2) System library via pkg-config
    let mut cfg = pkg_config::Config::new();
    if prefer_static() {
        cfg.statik(true);
    }
    if cfg!(feature = "dynamic") {
        cfg.statik(false);
    }
    if let Ok(lib) = cfg.atleast_version("8.2").probe("vips") {
        let mut include_paths = lib.include_paths;
        if let Some(vendor) = vendor_include_paths() {
            for p in vendor {
                if !include_paths.contains(&p) {
                    include_paths.push(p);
                }
            }
        }
        return Some((include_paths, Vec::new(), Some(lib.version.clone())));
    }

    // 3) Vendored headers (link still needs a built/installed libvips).
    //    libvips headers depend on glib; pull glib include paths from pkg-config when available.
    if let Some(include_paths) = vendor_include_paths() {
        println!(
            "cargo:warning=vips-sys: using vendored libvips headers from vendor/libvips; \
             install a libvips library or set LIBVIPS_LIB_DIR to link"
        );
        println!("cargo:rustc-link-lib=vips");
        let mut include_paths = include_paths;
        if let Ok(glib) = pkg_config::Config::new().probe("glib-2.0") {
            for p in glib.include_paths {
                if !include_paths.contains(&p) {
                    include_paths.push(p);
                }
            }
            if let Ok(gobject) = pkg_config::Config::new().probe("gobject-2.0") {
                for p in gobject.include_paths {
                    if !include_paths.contains(&p) {
                        include_paths.push(p);
                    }
                }
            }
        }
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
    let (include_paths, defines, version) = find_libvips().expect(
        "vips-sys: libvips not found. Install libvips (pkg-config/vcpkg), \
         set LIBVIPS_LIB_DIR/LIBVIPS_INCLUDE_DIR, or initialize the vendor/libvips submodule",
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
