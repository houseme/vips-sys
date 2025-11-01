# vips-sys

[English](README.md) | [Chinese Simplified](README_CN.md)

[![Crates.io](https://img.shields.io/crates/v/vips-sys.svg)](https://crates.io/crates/vips-sys)
[![Rust](https://github.com/houseme/vips-sys/actions/workflows/rust.yml/badge.svg)](https://github.com/houseme/vips-sys/actions/workflows/rust.yml)
[![Code Coverage](https://codecov.io/gh/elbaro/vips-sys/branch/master/graph/badge.svg)](https://codecov.io/gh/elbaro/vips-sys)
[![docs.rs](https://docs.rs/vips-sys/badge.svg)](https://docs.rs/vips-sys/)
[![License](https://img.shields.io/crates/l/vips-sys)](./LICENSE)
[![Downloads](https://img.shields.io/crates/d/vips-sys)](https://crates.io/crates/vips-sys)

Low-level Rust FFI bindings for `libvips`. Provides the foundation for higher-level, safer wrappers.

- Docs: https://houseme.github.io/vips-sys/vips_sys/
- Requirement: `libvips >= 8.2` (validated with `8.17.2`)
- Goal: stable builds, cross-platform reuse, and in-sync with upstream `libvips`

## Installation

- macOS
    - `brew install vips pkg-config`
- Debian/Ubuntu
    - `sudo apt-get install -y libvips-dev pkg-config`
- Windows (MSVC)
    - `vcpkg install vips` and ensure the environment is visible to `cargo`

Make sure `pkg-config --cflags --libs vips` works (Windows linking via `vcpkg`).

## Features and versions

- Optional features
    - `static`: prefer static linking
    - `dynamic`: prefer dynamic linking (default)
    - `helpers`: minimal safe helpers (`init`/`shutdown`/`version`)
- Build-time exports
    - `LIBVIPS_VERSION`: detected `libvips` version string
    - `cfg(vips_8_17)`: set when version `>= 8.17` for conditional compilation upstream
- Versioned bindings
    - Default to `OUT_DIR/binding.rs`
    - Compatible path: include versioned outputs when needed (e.g. `binding_8_74.rs`)

## Usage example (with `helpers`)

```rust
use vips_sys::helpers;

fn main() {
    helpers::init("vips-sys-example").expect("vips init failed");
    let ver = helpers::version();
    println!("libvips version: {}.{}.{}", ver.0, ver.1, ver.2);
    helpers::shutdown();
}
```

Cargo snippet:

```toml
[dependencies]
vips-sys = { version = "0.1.3-beta.1", features = ["helpers"] }
```

## Build notes

This crate uses `bindgen` at build time:

- Discover include paths via `pkg-config` and pass them to `clang`
- Disable `layout_tests`, enable `rustified_enum(".*")`
- Use `blocklist_*` to avoid platform conflicts

Environment variables:

- `PKG_CONFIG_PATH`: search path for `pkg-config` (e.g. macOS M1: `/opt/homebrew/lib/pkgconfig`)
- `LIBVIPS_NO_BINDGEN`: skip bindgen and reuse existing output
- `BINDGEN_EXTRA_CLANG_ARGS`: extra `clang` args (e.g. additional `-I`)
- `LIBCLANG_PATH`: location of `libclang` (e.g. local LLVM)

## Troubleshooting

- `vips` not found:
    - Ensure `libvips` and `pkg-config` are installed (or `vcpkg` on Windows)
    - Check `PKG_CONFIG_PATH` contains the directory of `vips.pc`
- Bindings generation fails:
    - Set `LIBCLANG_PATH` to a valid `libclang`
    - Add missing `-I` paths via `BINDGEN_EXTRA_CLANG_ARGS`
- Static linking issues (macOS):
    - Prefer dynamic linking or review known Homebrew caveats

## License

[MIT](LICENSE)

## Changelog

See [`CHANGELOG.md`](CHANGELOG.md).