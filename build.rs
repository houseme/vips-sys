use std::{env, path::PathBuf};

type IncludePaths = Vec<PathBuf>;
type Defines = Vec<(String, Option<String>)>;
type VersionOpt = Option<String>;
type ProbeResult = Option<(IncludePaths, Defines, VersionOpt)>;

#[cfg(target_env = "msvc")]
fn find_libvips() -> ProbeResult {
    // On MSVC, only links are handled, and headers are left to the pre-installed pkg-config or user-customized
    // No impact on macOS/Linux users
    let _lib = vcpkg::find_package("vips").ok()?;
    // vcpkg does not provide include_paths directly; If you need automatic bindgen under MSVC, you can extend the probe logic here
    Some((Vec::new(), Vec::new(), None))
}

#[cfg(not(target_env = "msvc"))]
fn find_libvips() -> ProbeResult {
    let mut cfg = pkg_config::Config::new();
    if cfg!(feature = "static") {
        cfg.statik(true);
    }
    if cfg!(feature = "dynamic") {
        cfg.statik(false);
    }
    let lib = cfg.atleast_version("8.2").probe("vips").ok()?;

    // Normalize version to Option<String>
    let version: VersionOpt = Some(lib.version.clone());

    // Keep defines empty for cross-version compatibility
    // To be compatible with different implementations, empty defines are returned first;
    // If you want to enable it, you can change this line to 'lib.defines'
    let defines: Defines = Vec::new();

    Some((lib.include_paths, defines, version))
}

fn generate_bindings(include_paths: &[PathBuf], defines: &Defines) {
    println!("cargo:rerun-if-changed='wrapper.h'");
    println!("cargo:rerun-if-changed='build.rs'");
    println!("cargo:rerun-if-env-changed=LIBVIPS_NO_BINDGEN");

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

fn main() {
    let (include_paths, defines, version) =
        find_libvips().expect("libvips not found via pkg-config/vcpkg");

    // Expose cfg based on installed version (conditional compilation is required at the upper level)
    if let Some(v) = version {
        println!("cargo:rustc-env=LIBVIPS_VERSION={}", v);
        // Simple example: 8.17.x and above
        let ver_parts: Vec<_> = v.split('.').collect();
        if ver_parts.len() >= 2 {
            if let (Ok(major), Ok(minor)) =
                (ver_parts[0].parse::<u32>(), ver_parts[1].parse::<u32>())
            {
                if major > 8 || (major == 8 && minor >= 17) {
                    println!("cargo:rustc-cfg=vips_8_17");
                }
            }
        }
    }

    generate_bindings(&include_paths, &defines);
}
