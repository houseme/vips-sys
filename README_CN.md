# vips-sys

[English](README.md) \| [简体中文](README_CN.md)

[![Crates.io](https://img.shields.io/crates/v/vips-sys.svg)](https://crates.io/crates/vips-sys)
[![Rust](https://github.com/houseme/vips-sys/actions/workflows/rust.yml/badge.svg)](https://github.com/houseme/vips-sys/actions/workflows/rust.yml)
[![Docs](https://img.shields.io/badge/docs-online-blue)](https://houseme.github.io/vips-sys/vips-sys/)
[![docs.rs](https://docs.rs/vips-sys/badge.svg)](https://docs.rs/vips-sys/)
[![License](https://img.shields.io/crates/l/vips-sys)](./LICENSE)
[![Downloads](https://img.shields.io/crates/d/vips-sys)](https://crates.io/crates/vips-sys)

`libvips` 的 Rust 低层 FFI 绑定。追求稳定、精简，可作为更高层安全封装的基础。

- 文档：https://houseme.github.io/vips-sys/vips_sys/
- 依赖：`libvips >= 8.2`（已在 `8.17.2` 验证）
- 目标：构建稳定、跨平台复用、跟踪上游

## 安装

- macOS
    - `brew install vips pkg-config`
    - Apple Silicon: 确认 `PKG_CONFIG_PATH=/opt/homebrew/lib/pkgconfig`
- Debian/Ubuntu
    - `sudo apt-get install -y libvips-dev pkg-config`
- Windows（MSVC）
    - 安装 [vcpkg](https://github.com/microsoft/vcpkg)，然后 `vcpkg install vips:x64-windows`
      （静态链接用 `vips:x64-windows-static`）
    - 设置 `VCPKG_ROOT`，以便 `vcpkg` crate 定位安装树
    - 启用 `static` 特性时可设置 `VCPKG_DEFAULT_TRIPLET=x64-windows-static`

请验证 `pkg-config --cflags --libs vips` 可用（MSVC 链接通过 `vcpkg`）。

### 子模块中的 libvips

本仓库通过 git 子模块在 `vendor/libvips` 引入
[libvips](https://github.com/libvips/libvips)（固定 `v8.18.6`），
遵循常见 `*-sys` crate 做法（参见 [rust-sys-crate](https://kornel.ski/rust-sys-crate)）。

构建解析顺序：

1. `LIBVIPS_LIB_DIR` / `LIBVIPS_INCLUDE_DIR`（可选 `LIBVIPS_STATIC=1`）
2. 系统库：`pkg-config`（Linux/BSD/macOS）或 `vcpkg`（MSVC）
3. 请求静态链接时：尝试用 meson/ninja 从 `vendor/libvips` 构建
4. 回退到 `vendor/libvips` 头文件（链接仍需要 libvips 库）

## 可选特性

- `static`：优先静态链接（也可用 `LIBVIPS_STATIC=1`；环境变量优先于特性）
- `dynamic`：优先动态链接（默认）
- `helpers`：提供 `init`/`shutdown`/`version` 的最小安全辅助

### 静态链接

```toml
[dependencies]
vips-sys = { version = "0.1.3-beta.2", features = ["static"] }
```

或：

```bash
LIBVIPS_STATIC=1 cargo build
```

静态构建解析：

1. 已安装静态 `libvips` 时走 `pkg-config --static` / vcpkg static triplet
2. `PATH` 中有 `meson` + `ninja` 时，从 `vendor/libvips` 构建到 `OUT_DIR`
3. 否则给出明确错误提示

构建期导出：

- `LIBVIPS_VERSION`：检测到的 `libvips` 版本
- `cfg(vips_8_17)`：当版本 `>= 8.17` 启用

## 示例（启用 `helpers`）

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

## 构建说明

本仓库在构建时使用 `bindgen`：

- 通过 `pkg-config` 获取包含目录并传递给 `clang`
- 关闭 `layout_tests`，开启 `rustified_enum(".*")`
- 关闭注释生成，避免 doctest 误报
- 屏蔽部分条目以提升可移植性

环境变量：

- `PKG_CONFIG_PATH`：`vips.pc` 搜索路径
- `LIBVIPS_LIB_DIR` / `LIBVIPS_INCLUDE_DIR`：显式指定库/头文件目录
- `LIBVIPS_STATIC=1`：优先静态链接（`0`/`false` 关闭）
- `LIBVIPS_NO_BINDGEN`：跳过绑定生成，复用已有输出
- `LIBVIPS_NO_VENDOR`：忽略 vendor 头文件
- `VCPKG_ROOT` / `VCPKG_DEFAULT_TRIPLET`：Windows vcpkg 探测
- `BINDGEN_EXTRA_CLANG_ARGS`：额外传递给 `clang` 的参数（如 `-I`）
- `LIBCLANG_PATH`：`libclang` 路径

## Windows 说明（MSVC）

```bat
vcpkg install vips:x64-windows
set VCPKG_ROOT=C:\path\to\vcpkg
cargo build
```

静态：

```bat
vcpkg install vips:x64-windows-static
set VCPKG_DEFAULT_TRIPLET=x64-windows-static
set LIBVIPS_STATIC=1
cargo build --features static
```

## 故障排查

- 未找到 `vips`：
    - 安装 `libvips` 与 `pkg-config`（Windows 使用 `vcpkg`）
    - 检查 `PKG_CONFIG_PATH` / `VCPKG_ROOT`
- 绑定生成失败：
    - 设置 `LIBCLANG_PATH` 或通过 `BINDGEN_EXTRA_CLANG_ARGS` 添加缺失的包含目录
- macOS 静态链接：
    - 建议优先动态链接；需要静态时用 vendor + meson
- Windows bindgen 找不到头文件：
    - 确认已设置 `VCPKG_ROOT`

## 许可证

[MIT](LICENSE)

## 更新日志

参见 [`CHANGELOG.md`](CHANGELOG.md)。