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
    - Install [vcpkg](https://github.com/microsoft/vcpkg), then `vcpkg install vips:x64-windows`
      (or `vips:x64-windows-static` for static)
    - Set `VCPKG_ROOT` so the `vcpkg` crate can locate the installed tree
    - Optional: `set VCPKG_DEFAULT_TRIPLET=x64-windows-static` when using the `static` feature

Verify `pkg-config --cflags --libs vips` works (MSVC links via `vcpkg`).

### Vendored libvips (submodule)

This crate vendors [libvips](https://github.com/libvips/libvips) as a git submodule at
`vendor/libvips` (pinned to `v8.18.6`), following common `*-sys` crate practice
(see [kornel.ski/rust-sys-crate](https://kornel.ski/rust-sys-crate)):

```bash
git clone --recurse-submodules <this-repo>
# or after clone:
git submodule update --init --recursive
```

Build resolution order:

1. `LIBVIPS_LIB_DIR` / `LIBVIPS_INCLUDE_DIR` (and optional `LIBVIPS_STATIC=1`)
2. System library via `pkg-config` (Linux/BSD/macOS) or `vcpkg` (MSVC)
3. When `static` is requested: attempt a meson/ninja static build from `vendor/libvips`
4. Vendored headers under `vendor/libvips` (linking still requires a libvips library)

## Features

- `static`: prefer static linking (also `LIBVIPS_STATIC=1`; env wins over features)
- `dynamic`: prefer dynamic linking (default)
- `helpers`: minimal helpers for init/shutdown/version

### Static linking

```toml
[dependencies]
vips-sys = { version = "0.1.3-beta.2", features = ["static"] }
```

or

```bash
LIBVIPS_STATIC=1 cargo build
```

Resolution for static builds:

1. `pkg-config --static` / vcpkg static triplet when a static `libvips` is already installed
2. Meson build of `vendor/libvips` into `OUT_DIR` when `meson` + `ninja` are on `PATH`
3. Clear error otherwise

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
- `LIBVIPS_LIB_DIR` / `LIBVIPS_INCLUDE_DIR`: explicit library/include override
- `LIBVIPS_STATIC=1`: prefer static linking (`0`/`false` disables)
- `LIBVIPS_NO_BINDGEN`: skip bindgen and reuse generated output
- `LIBVIPS_NO_VENDOR`: ignore vendored headers
- `VCPKG_ROOT` / `VCPKG_DEFAULT_TRIPLET`: Windows vcpkg discovery
- `BINDGEN_EXTRA_CLANG_ARGS`: extra `-I` or flags
- `LIBCLANG_PATH`: path to `libclang` if needed

## Windows notes (MSVC)

```bat
git clone https://github.com/microsoft/vcpkg %VCPKG_ROOT%
%VCPKG_ROOT%\vcpkg install vips:x64-windows
set VCPKG_ROOT=%VCPKG_ROOT%
cargo build
```

Static:

```bat
vcpkg install vips:x64-windows-static
set VCPKG_DEFAULT_TRIPLET=x64-windows-static
set LIBVIPS_STATIC=1
cargo build --features static
```

## Troubleshooting

- Not found `vips`:
    - Install `libvips` and `pkg-config` (or `vcpkg` on Windows)
    - Check `PKG_CONFIG_PATH` / `VCPKG_ROOT`
- Bindgen failed:
    - Set `LIBCLANG_PATH`, or add include dirs via `BINDGEN_EXTRA_CLANG_ARGS`
- Static linking on macOS:
    - Prefer dynamic linking due to Homebrew constraints; use vendor + meson for static
- Windows bindgen cannot find headers:
    - Ensure `VCPKG_ROOT` is set so include paths are discovered

## License

[MIT](LICENSE)

## Changelog

See [`CHANGELOG.md`](CHANGELOG.md).