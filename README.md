# vips-sys

[English](README.md) \| [简体中文](README_CN.md)

[![Crates.io](https://img.shields.io/crates/v/vips-sys.svg)](https://crates.io/crates/vips-sys)
[![Rust](https://github.com/houseme/vips-sys/actions/workflows/rust.yml/badge.svg)](https://github.com/houseme/vips-sys/actions/workflows/rust.yml)
[![Docs](https://img.shields.io/badge/docs-online-blue)](https://houseme.github.io/vips-sys/vips-sys/)
[![docs.rs](https://docs.rs/vips-sys/badge.svg)](https://docs.rs/vips-sys/)
[![License](https://img.shields.io/crates/l/vips-sys)](./LICENSE)
[![Downloads](https://img.shields.io/crates/d/vips-sys)](https://crates.io/crates/vips-sys)

Low-level Rust FFI bindings for `libvips`. Designed to be stable, minimal, and a foundation for higher-level wrappers.

- Docs: https://houseme.github.io/vips-sys/vips_sys/
- Requirement: `libvips >= 8.2` (validated on `8.17.2`)
- Goals: reliable builds, cross-platform reuse, in-sync with upstream

## Installation

- macOS
    - `brew install vips pkg-config`
    - Apple Silicon: ensure `PKG_CONFIG_PATH=/opt/homebrew/lib/pkgconfig`
- Debian/Ubuntu
    - `sudo apt-get install -y libvips-dev pkg-config`
- Windows (MSVC)
    - `vcpkg install vips` and ensure the environment is visible to `cargo`

Verify `pkg-config --cflags --libs vips` works (MSVC links via `vcpkg`).

## Features

- `static`: prefer static linking
- `dynamic`: prefer dynamic linking (default)
- `helpers`: minimal helpers for init/shutdown/version

Build-time exports:

- `LIBVIPS_VERSION`: detected `libvips` version string
- `cfg(vips_8_17)`: enabled when version `>= 8.17`

## Example (`helpers`)

```rust
use vips_sys::helpers;

fn main() {
    helpers::init("vips-sys-example").expect("vips init failed");
    let (a, b, c) = helpers::version();
    println!("libvips version: {}.{}.{}", a, b, c);
    helpers::shutdown();
}
```

Cargo:

```toml
[dependencies]
vips-sys = { version = "0.1.3-beta.2", features = ["helpers"] }
```

## Build notes

This crate uses `bindgen`:

- Include paths from `pkg-config` and pass to `clang`
- `layout_tests` disabled, `rustified_enum(".*")` enabled
- Comments disabled to avoid doctest noise
- Some items blocklisted for portability

Environment:

- `PKG_CONFIG_PATH`: path for `vips.pc`
- `LIBVIPS_NO_BINDGEN`: skip bindgen and reuse generated output
- `BINDGEN_EXTRA_CLANG_ARGS`: extra `-I` or flags
- `LIBCLANG_PATH`: path to `libclang` if needed

## Troubleshooting

- Not found `vips`:
    - Install `libvips` and `pkg-config` (or `vcpkg` on Windows)
    - Check `PKG_CONFIG_PATH`
- Bindgen failed:
    - Set `LIBCLANG_PATH`, or add include dirs via `BINDGEN_EXTRA_CLANG_ARGS`
- Static linking on macOS:
    - Prefer dynamic linking due to Homebrew constraints

## License

[MIT](LICENSE)

## Changelog

See [`CHANGELOG.md`](CHANGELOG.md).