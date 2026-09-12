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
- Requirement: `libvips >= 8.2` (bindings generated from **8.18.6**)
- Goals: fast builds, reliable linking, cross-platform reuse

## Highlights

| | Default | Feature `bindgen` |
|--|---------|-------------------|
| Bindings | Pregenerated `src/bindings/prebuilt.rs` | Live bindgen via clang |
| Needs libclang? | **No** | Yes |
| Typical `cargo build` | Seconds | Slower (first time) |
| Use when | Normal development / CI | Updating API surface / new libvips |

Normal builds **do not run bindgen**. You only need a libvips **library** to link against.

## Installation

### macOS

```bash
brew install vips pkg-config
# Apple Silicon:
export PKG_CONFIG_PATH=/opt/homebrew/lib/pkgconfig
```

### Debian / Ubuntu

```bash
sudo apt-get install -y libvips-dev pkg-config
```

### Windows (MSVC)

```bat
vcpkg install vips:x64-windows
set VCPKG_ROOT=C:\path\to\vcpkg
```

Static: `vcpkg install vips:x64-windows-static` and `set VCPKG_DEFAULT_TRIPLET=x64-windows-static`.

Verify: `pkg-config --cflags --libs vips` (MSVC uses vcpkg instead).

## Cargo

```toml
[dependencies]
vips-sys = { version = "0.1.3-beta.2", features = ["helpers"] }
```

### Features

| Feature | Default | Description |
|---------|---------|-------------|
| *(none)* | ✓ | Pregenerated bindings + link probe |
| `helpers` | | `init` / `shutdown` / `version` / `version_string` (cached) |
| `static` | | Prefer static linking (`LIBVIPS_STATIC=1` overrides) |
| `dynamic` | | Prefer dynamic linking |
| `bindgen` | | Regenerate bindings with clang against local headers |

### Static linking

```toml
vips-sys = { version = "0.1.3-beta.2", features = ["static"] }
```

```bash
LIBVIPS_STATIC=1 cargo build
```

Resolution order:

1. `pkg-config --static` / vcpkg static triplet if a static libvips is installed
2. Meson + ninja build of `vendor/libvips` into `OUT_DIR` (if tools are on `PATH`)
3. Clear error otherwise

### Vendored libvips (submodule)

`vendor/libvips` tracks [libvips](https://github.com/libvips/libvips) at **v8.18.6**
(see [rust-sys-crate](https://kornel.ski/rust-sys-crate)):

```bash
git clone --recurse-submodules <this-repo>
# or:
git submodule update --init --recursive
```

Link / header resolution:

1. `LIBVIPS_LIB_DIR` / `LIBVIPS_INCLUDE_DIR` (optional `LIBVIPS_STATIC=1`)
2. System library via `pkg-config` (Unix) or `vcpkg` (MSVC)
3. Static vendor build when `static` is requested
4. Vendored headers for include paths (still need a library to link)

## Refreshing pregenerated bindings

```bash
# Requires: libclang, glib headers, and vips headers (system or vendor submodule)
cargo build --features bindgen
# build.rs prints the OUT_DIR path — copy it over:
cp target/debug/build/vips-sys-*/out/binding.rs src/bindings/prebuilt.rs
```

`wrapper.h` is the single bindgen entry point and is **kept** in the repo.
`generate.sh` was removed in favor of the `bindgen` feature.

Build-time exports:

- `LIBVIPS_VERSION` — detected version string (from pkg-config when available)
- `cfg(vips_8_16)` / `cfg(vips_8_17)` — when version is new enough

## Example (`helpers`)

```rust
use vips_sys::helpers;

fn main() {
    helpers::init("vips-sys-example").expect("vips init failed");
    let (a, b, c) = helpers::version();
    println!("libvips {}.{}.{} ({})", a, b, c, helpers::version_string());
    helpers::shutdown();
}
```

## Environment variables

| Variable | Purpose |
|----------|---------|
| `PKG_CONFIG_PATH` | Find `vips.pc` |
| `LIBVIPS_LIB_DIR` / `LIBVIPS_INCLUDE_DIR` | Explicit library / headers |
| `LIBVIPS_STATIC` | `1` prefer static; `0`/`false`/`off` disable |
| `LIBVIPS_NO_VENDOR` | Ignore `vendor/libvips` |
| `LIBVIPS_NO_BINDGEN` | With feature `bindgen`, skip generation |
| `LIBVIPS_VERSION` | Override version string for cfg |
| `VCPKG_ROOT` / `VCPKG_DEFAULT_TRIPLET` | Windows vcpkg |
| `BINDGEN_EXTRA_CLANG_ARGS` | Extra clang flags (bindgen feature) |
| `LIBCLANG_PATH` | libclang location (bindgen feature) |

## Troubleshooting

- **link error, library not found** — install libvips (`libvips-dev` / `vcpkg install vips`)
- **bindgen feature fails** — install `libclang`, glib headers, and vips headers
- **static on macOS** — prefer dynamic (Homebrew); use vendor + meson for true static
- **Windows headers** — set `VCPKG_ROOT` so include paths resolve

## License

[MIT](LICENSE)

## Changelog

See [`CHANGELOG.md`](CHANGELOG.md).
