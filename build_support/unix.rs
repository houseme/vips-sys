//! Unix dependency discovery and incremental native-source builds.
use super::{BuildOptions, ProbeResult, config, pkg_info, vendor_root};
use std::{env, fs, path::PathBuf, process::Command};

pub(super) fn explicit(options: &BuildOptions, directory: &std::path::Path) -> ProbeResult {
    let pc = directory.join("pkgconfig/vips.pc");
    if !pc.is_file() {
        return None;
    }
    let lib = pkg_config::Config::new()
        .cargo_metadata(!options.static_linking)
        .statik(options.static_linking)
        .probe(pc.to_str()?)
        .ok()?;
    if options.static_linking {
        emit_static_metadata(&lib);
    }
    let mut info = pkg_info(lib, options);
    if let Some(include) = &options.include_dir {
        info.include_paths.insert(0, include.clone());
    }
    if options.version.is_some() {
        info.version.clone_from(&options.version);
    }
    Some(info)
}

/// The static preference selects a static libvips archive. Dependencies retain
/// platform-default linkage, as in Meson's native build; --static supplies the
/// private dependency closure without forcing every installed codec archive.
fn emit_static_metadata(lib: &pkg_config::Library) {
    for path in &lib.link_paths {
        println!("cargo:rustc-link-search=native={}", path.display());
    }
    for path in &lib.framework_paths {
        println!("cargo:rustc-link-search=framework={}", path.display());
    }
    for name in &lib.libs {
        match name.as_str() {
            "vips" => println!("cargo:rustc-link-lib=static=vips"),
            // Apple uses libc++; some third-party .pc files report stdc++.
            "stdc++" | "c++" | "c++_shared" => {}
            _ => println!("cargo:rustc-link-lib={name}"),
        }
    }
    for framework in &lib.frameworks {
        println!("cargo:rustc-link-lib=framework={framework}");
    }
    for path in &lib.link_files {
        if let (Some(parent), Some(file)) = (path.parent(), path.file_name()) {
            println!("cargo:rustc-link-search=native={}", parent.display());
            let kind = if path.extension().is_some_and(|s| s == "a") {
                "static"
            } else {
                "dylib"
            };
            println!(
                "cargo:rustc-link-lib={kind}:+verbatim={}",
                file.to_string_lossy()
            );
        }
    }
    for args in &lib.ld_args {
        println!("cargo:rustc-link-arg=-Wl,{}", args.join(","));
    }
    if let Some(cpp) = config::cpp_stdlib(
        &env::var("TARGET").unwrap_or_default(),
        env::var("CXXSTDLIB").ok(),
    ) {
        println!("cargo:rustc-link-lib={cpp}");
    }
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
fn try_build_vendor_static(options: &BuildOptions) -> ProbeResult {
    let root = vendor_root(options)?;
    let out_dir = PathBuf::from(env::var_os("OUT_DIR")?).join("libvips-build");
    let prefix = out_dir.join("install");
    let lib_dir = prefix.join("lib");
    let archive = lib_dir.join("libvips.a");
    let (Some(meson), Some(ninja)) = (which("meson"), which("ninja")) else {
        println!(
            "cargo:warning=vips-sys: vendor builds require Meson >= 0.60 and Ninja; set MESON_PATH/NINJA_PATH or add them to PATH"
        );
        return None;
    };
    if env::var_os("HOST") != env::var_os("TARGET") {
        println!(
            "cargo:warning=vips-sys: vendor builds require a native target; provide target libraries with pkg-config or LIBVIPS_LIB_DIR"
        );
        return None;
    }
    // Native inputs are watched only when this backend is actually selected.
    for path in [
        root.join("meson.build"),
        root.join("meson_options.txt"),
        root.join("libvips"),
    ] {
        println!("cargo:rerun-if-changed={}", path.display());
    }

    fs::create_dir_all(&out_dir).ok()?;
    let mut fingerprint = String::new();
    for key in [
        "HOST",
        "TARGET",
        "MESON_PATH",
        "NINJA_PATH",
        "CC",
        "CXX",
        "AR",
        "RANLIB",
        "CFLAGS",
        "CPPFLAGS",
        "CXXFLAGS",
        "LDFLAGS",
        "SDKROOT",
        "MACOSX_DEPLOYMENT_TARGET",
        "PKG_CONFIG_PATH",
        "PKG_CONFIG_LIBDIR",
        "PKG_CONFIG_SYSROOT_DIR",
    ] {
        println!("cargo:rerun-if-env-changed={key}");
        // This private OUT_DIR stamp governs configuration, never source freshness.
        use std::fmt::Write;
        writeln!(&mut fingerprint, "{key}={:?}", env::var_os(key)).ok()?;
    }
    let stamp = out_dir.join("configuration");
    let builddir = out_dir.join("builddir");
    if stamp.is_file()
        && fs::read_to_string(&stamp).ok().as_deref() != Some(fingerprint.as_str())
        && builddir.is_dir()
    {
        fs::remove_dir_all(&builddir).ok()?;
    }
    let mut setup = Command::new(&meson);
    if builddir.join("meson-private/coredata.dat").is_file() {
        setup.arg("setup").arg("--reconfigure");
    } else {
        setup.arg("setup");
    }
    let status = setup
        .args([
            builddir.to_str()?,
            root.to_str()?,
            "--prefix",
            prefix.to_str()?,
            "--libdir=lib",
            "--default-library=static",
            "--buildtype=release",
            "--wrap-mode=nofallback",
            "-Dintrospection=disabled",
            "-Dmodules=disabled",
            "-Ddeprecated=true",
            "-Dcplusplus=false",
            "-Dexamples=false",
        ])
        .status()
        .ok()?;
    if !status.success() {
        println!("cargo:warning=vips-sys: meson setup failed for vendored libvips");
        return None;
    }

    fs::write(stamp, fingerprint).ok()?;

    // Build only the library: tools, examples and fuzz executables are not
    // needed by Rust consumers. Meson devel tags install headers/archive/.pc.
    if !Command::new(ninja)
        .args(["-C", builddir.to_str()?, "libvips/libvips.a"])
        .status()
        .ok()?
        .success()
        || !Command::new(meson)
            .args([
                "install",
                "-C",
                builddir.to_str()?,
                "--no-rebuild",
                "--only-changed",
                "--tags",
                "devel",
            ])
            .status()
            .ok()?
            .success()
    {
        println!("cargo:warning=vips-sys: vendored libvips build/install failed");
        return None;
    }
    if !archive.is_file() {
        return None;
    }
    // Resolve the complete dependency closure from the generated .pc file.
    let pc = lib_dir.join("pkgconfig/vips.pc");
    let library = pkg_config::Config::new()
        .cargo_metadata(false)
        .env_metadata(false)
        .statik(true)
        .probe(pc.to_str()?);
    match library {
        Ok(lib) => {
            emit_static_metadata(&lib);
            Some(pkg_info(lib, options))
        }
        Err(error) => {
            println!(
                "cargo:warning=vips-sys: cannot resolve vendored libvips dependencies: {error}"
            );
            None
        }
    }
}

pub(super) fn find(options: &BuildOptions) -> ProbeResult {
    // 2) System library via pkg-config (fast path)
    //    Do not merge vendor headers here: system includes match the linked
    //    library, and skipping FS probes keeps the build script cheap.
    let mut cfg = pkg_config::Config::new();
    if options.static_linking {
        // pkg-config --static can silently fall back to a dylib when the
        // requested archive is absent. Only accept a real static libvips.
        let installed_archive = pkg_config::get_variable("vips", "libdir")
            .ok()
            .is_some_and(|dir| PathBuf::from(dir).join("libvips.a").is_file());
        if !installed_archive {
            return try_build_vendor_static(options);
        }
        cfg.statik(true);
        cfg.cargo_metadata(false);
    }
    if let Ok(lib) = cfg
        .atleast_version(if cfg!(feature = "v8_18") {
            "8.18"
        } else {
            "8.2"
        })
        .probe("vips")
    {
        if options.static_linking {
            emit_static_metadata(&lib);
        }
        return Some(pkg_info(lib, options));
    }

    if options.static_linking {
        return try_build_vendor_static(options);
    }
    None
}
