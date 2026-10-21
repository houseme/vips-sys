//! Exercise the real vcpkg parser with isolated installation metadata.
//! Placeholder libraries test discovery/metadata, not Windows machine code.
#[path = "../build_support/windows.rs"]
mod windows;

use std::{fs, path::PathBuf, process::Command};

struct Fixture(PathBuf);

impl Fixture {
    fn new(case: &str, triplet: &str) -> Self {
        let root = std::env::temp_dir().join(format!("vips-vcpkg-{}-{case}", std::process::id()));
        fs::create_dir_all(root.join("installed/vcpkg/info")).unwrap();
        fs::create_dir_all(root.join("installed/vcpkg/updates")).unwrap();
        fs::write(root.join(".vcpkg-root"), "").unwrap();
        let mut status = String::new();
        for (port, libs, deps) in [
            ("libvips", &["libvips-42"][..], "Depends: glib\n"),
            ("glib", &["gobject-2.0", "glib-2.0"][..], ""),
        ] {
            status.push_str(&format!("Package: {port}\nVersion: 8.18.7\nArchitecture: {triplet}\n{deps}Status: install ok installed\n\n"));
            let mut manifest = String::new();
            for library in libs {
                for extension in ["lib", "dll"] {
                    if extension == "dll" && triplet.contains("-static") {
                        continue;
                    }
                    let directory = if extension == "lib" { "lib" } else { "bin" };
                    let path = format!("{triplet}/{directory}/{library}.{extension}");
                    let installed = root.join("installed").join(&path);
                    fs::create_dir_all(installed.parent().unwrap()).unwrap();
                    fs::write(installed, "placeholder").unwrap();
                    manifest.push_str(&format!("{path}\n"));
                }
            }
            fs::write(
                root.join(format!("installed/vcpkg/info/{port}_8.18.7_{triplet}.list")),
                manifest,
            )
            .unwrap();
        }
        fs::write(root.join("installed/vcpkg/status"), status).unwrap();
        for directory in ["include/glib-2.0", "lib/glib-2.0/include"] {
            fs::create_dir_all(root.join("installed").join(triplet).join(directory)).unwrap();
        }
        Self(root)
    }
}

impl Drop for Fixture {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

#[test]
#[ignore = "entry point invoked by isolated parent tests"]
fn subprocess_probe() {
    let triplet = std::env::var("VIPS_TEST_TRIPLET").expect("parent test supplies the triplet");
    let mut config = windows::config(std::env::var_os("VIPS_TEST_STATIC").is_some());
    config.cargo_metadata(false).copy_dlls(false);
    let result = windows::probe(&mut config);
    if std::env::var_os("VIPS_TEST_DYNAMIC_MISSING").is_some() {
        assert!(
            matches!(result, Err(vcpkg::Error::RequiredEnvMissing(ref name)) if name == "VCPKGRS_DYNAMIC")
        );
        return;
    }
    let library = result.expect("official libvips port should be discovered");
    assert_eq!(library.vcpkg_triplet, triplet);
    assert_eq!(library.is_static, triplet.contains("-static"));
    assert!(library.ports.contains(&"libvips".to_owned()));
    for name in ["libvips-42", "gobject-2.0", "glib-2.0"] {
        assert!(
            library.found_names.iter().any(|found| found == name),
            "missing manifest-derived library {name}"
        );
    }
    let prefix = PathBuf::from(std::env::var_os("VCPKG_ROOT").unwrap())
        .join("installed")
        .join(triplet);
    let includes = windows::include_paths(&library);
    for directory in ["include", "include/glib-2.0", "lib/glib-2.0/include"] {
        assert!(includes.contains(&prefix.join(directory)));
    }
}

fn run_case(name: &str, triplet: &str, arch: &str, static_link: bool, settings: &[(&str, &str)]) {
    let fixture = Fixture::new(name, triplet);
    let mut command = Command::new(std::env::current_exe().unwrap());
    command.args(["--ignored", "--exact", "subprocess_probe", "--nocapture"]);
    for key in [
        "VCPKG_ROOT",
        "VCPKGRS_TRIPLET",
        "VCPKG_DEFAULT_TRIPLET",
        "VCPKGRS_DYNAMIC",
        "VCPKGRS_DISABLE",
        "NO_VCPKG",
        "VCPKGRS_NO_LIBVIPS",
        "LIBVIPS_NO_VCPKG",
        "VIPS_TEST_STATIC",
        "VIPS_TEST_DYNAMIC_MISSING",
    ] {
        command.env_remove(key);
    }
    let target = match arch {
        "x86_64" => "x86_64-pc-windows-msvc",
        "aarch64" => "aarch64-pc-windows-msvc",
        _ => "i686-pc-windows-msvc",
    };
    command
        .env("TARGET", target)
        .env("CARGO_CFG_TARGET_ARCH", arch)
        .env("CARGO_CFG_TARGET_FEATURE", "")
        .env("VCPKG_ROOT", &fixture.0)
        .env("VIPS_TEST_TRIPLET", triplet);
    if static_link {
        command.env("VIPS_TEST_STATIC", "1");
    }
    for (key, value) in settings {
        command.env(key, value);
    }
    let output = command.output().unwrap();
    assert!(
        output.status.success(),
        "{name}: {}\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
}

#[test]
fn official_port_dynamic_and_transitive_libraries() {
    run_case(
        "dynamic",
        "x64-windows",
        "x86_64",
        false,
        &[("VCPKGRS_DYNAMIC", "1")],
    );
}

#[test]
fn static_linking_matches_target_architecture_and_crt() {
    for (name, triplet, arch, crt) in [
        ("static-md", "x64-windows-static-md", "x86_64", ""),
        ("static-crt", "x64-windows-static", "x86_64", "crt-static"),
        ("arm64", "arm64-windows-static-md", "aarch64", ""),
        ("x86", "x86-windows-static-md", "x86", ""),
    ] {
        run_case(
            name,
            triplet,
            arch,
            true,
            &[("CARGO_CFG_TARGET_FEATURE", crt), ("VCPKGRS_DYNAMIC", "1")],
        );
    }
}

#[test]
fn explicit_triplet_overrides_are_respected() {
    run_case(
        "cli-default",
        "x64-windows",
        "x86_64",
        false,
        &[
            ("VCPKG_DEFAULT_TRIPLET", "x64-windows"),
            ("VCPKGRS_DYNAMIC", "1"),
        ],
    );
    run_case(
        "rust-override",
        "x64-windows-static",
        "x86_64",
        true,
        &[
            ("VCPKGRS_TRIPLET", "x64-windows-static"),
            ("VCPKG_DEFAULT_TRIPLET", "wrong-triplet"),
        ],
    );
}

#[test]
fn dynamic_linking_requires_explicit_opt_in() {
    run_case(
        "missing-dynamic",
        "x64-windows",
        "x86_64",
        false,
        &[
            ("VCPKGRS_TRIPLET", "x64-windows"),
            ("VIPS_TEST_DYNAMIC_MISSING", "1"),
        ],
    );
}
