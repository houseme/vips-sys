# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](http://keepachangelog.com/en/1.0.0/)
and this project adheres to [Semantic Versioning](http://semver.org/spec/v2.0.0.html) as implemented by Cargo.

---

## [Unreleased]

### Performance

- **Pregenerated bindings** (`src/bindings/prebuilt.rs`) are used by default: no
  clang/bindgen required for normal `cargo build`. Enable feature `bindgen` only
  when refreshing bindings against local headers.
- Bindgen (feature `bindgen`) allowlists `vips_*` / `Vips*` / `VIPS_*` plus a few
  GObject helpers, uses `use_core()` + `core::ffi`, and fingerprint-caches output
  in `OUT_DIR`.
- `helpers::init` / `version` / `version_string` use `OnceLock` (no repeated FFI).
- `bindgen` is an optional build-dependency (`default-features = false`).
- `merge_includes` dedups with a `HashSet`.

### Added

- Vendored libvips as a git submodule at `vendor/libvips` (pinned to **v8.18.6**).
- Static linking: `pkg-config --static` / vcpkg static triplet; optional meson+ninja
  build from vendored sources (`static` feature or `LIBVIPS_STATIC=1`).
- Windows/vcpkg: `VCPKG_ROOT`, `VCPKG_DEFAULT_TRIPLET`; document
  `vcpkg install vips:x64-windows[-static]`.
- Env overrides: `LIBVIPS_LIB_DIR`, `LIBVIPS_INCLUDE_DIR`, `LIBVIPS_NO_VENDOR`,
  `LIBVIPS_STATIC` (`0`/`false`/`off` disable).
- Export `cargo:include` for dependent sys crates.
- `cfg(vips_8_16)` and `cfg(vips_8_17)`.
- `helpers::version_string()`.
- Feature `bindgen` to regenerate FFI from local headers.

### Changed

- Default compile path no longer runs bindgen; `src/bindings/prebuilt.rs` is
  `include!`d unless feature `bindgen` is enabled.
- CI workflows use `actions/checkout@v7` with `submodules: recursive` and
  `dtolnay/rust-toolchain@stable`.
- docs.rs metadata enables `helpers` only (not `bindgen`).
- Removed `generate.sh` — use `cargo build --features bindgen` instead.
  `wrapper.h` is kept as the bindgen entry header.

### Fixed

- `prefer_static()` matches `env::var` as `Ok` (was incorrectly `Some`).

## [0.1.3-beta.2] - 2025-11-02

- Fix: disable doctests for auto-generated bindings to avoid rustdoc failures.
- Chore: refactor build script types to reduce `clippy::type-complexity`.
- Docs: rewrite `README.md` in English and add `README_CN.md` with language switch.
- Build: ensure bindgen does not generate comments to keep bindings lean.

## [0.1.3-beta.1] - 2025-11-01

### Added

- Added optional feature 'helpers' to provide a minimum security package: 'init'/'shutdown'/'version' for quick
  verification and single testing.
- Export the 'LIBVIPS_VERSION' environment variable during build time to make it easier for upper-level to see installed
  libvips versions.
- Build period condition: When '>= 8.17.x' is detected, 'cfg(vips_8_17)' is output to pave the way for conditional
  compilation at the upper layer.

### Changed

- Centralize binding generation to 'build.rs', use 'bindgen' 'blocklist_*' API, 'CargoCallbacks', turn off '
  layout_tests' for improved stability and repeatable builds.
- 'pkg-config' is responsible for the header file path detection, and automatically injects '-I' to 'bindgen'.
- macOS/Apple Silicon does not require manual hardcoding of the '-I' path, which is handled by 'pkg-config' by default.
- Unified feature switches to control static/dynamic links ('static'/'dynamic') to reduce platform branch scattering.

### Fixed

- Fixed 'find_libvips' return type mismatch in non-MSVC environments and compilation errors caused by misuse of '
  String'.
  -Improved compatibility on different versions of 'pkg-config' to avoid vulnerabilities in parsing '--cflags'.

### Notes

- Runtime requirements 'libvips >= 8.2', validated locally to '8.17.2'.
- Windows (MSVC) does link probe via 'vcpkg'.
