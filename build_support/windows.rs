//! MSVC dependency discovery, shared with filesystem-based regression tests.
use std::{env, path::PathBuf};

pub fn config(prefer_static: bool) -> vcpkg::Config {
    let mut config = vcpkg::Config::new();
    // Honor the Rust helper's global override before the vcpkg CLI default.
    if let Ok(triplet) = env::var("VCPKGRS_TRIPLET").or_else(|_| env::var("VCPKG_DEFAULT_TRIPLET"))
    {
        config.target_triplet(triplet);
    } else if prefer_static {
        let arch = match env::var("CARGO_CFG_TARGET_ARCH").as_deref() {
            Ok("x86_64") => Some("x64"),
            Ok("x86") => Some("x86"),
            Ok("aarch64") => Some("arm64"),
            _ => None,
        };
        if let Some(arch) = arch {
            let features = env::var("CARGO_CFG_TARGET_FEATURE").unwrap_or_default();
            let suffix = if features.split(',').any(|feature| feature == "crt-static") {
                "static"
            } else {
                "static-md"
            };
            config.target_triplet(format!("{arch}-windows-{suffix}"));
        }
    }
    config
}

pub fn probe(config: &mut vcpkg::Config) -> Result<vcpkg::Library, vcpkg::Error> {
    config.find_package("libvips")
}

pub fn include_paths(lib: vcpkg::Library) -> Vec<PathBuf> {
    let mut paths = lib.include_paths;
    // GLib headers are split across the selected triplet's include and lib
    // directories. Never mix in system headers or another triplet's glibconfig.h.
    for index in 0..paths.len() {
        let glib = paths[index].join("glib-2.0");
        if glib.is_dir() {
            paths.push(glib);
        }
    }
    for link in &lib.link_paths {
        let glib_config = link.join("glib-2.0/include");
        if glib_config.is_dir() {
            paths.push(glib_config);
        }
    }
    paths
}
