# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](http://keepachangelog.com/en/1.0.0/)
and this project adheres to [Semantic Versioning](http://semver.org/spec/v2.0.0.html) as implemented by Cargo.

---

## [Unreleased]

- Documentation: Supplementary installation, build, and troubleshooting instructions.
- CI: It is recommended to override the minimum smoke test for macOS and Linux ('helpers::init()').

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
