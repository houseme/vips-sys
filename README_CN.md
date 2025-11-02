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
    - `vcpkg install vips` 并确保 `cargo` 可见

请验证 `pkg-config --cflags --libs vips` 可用（MSVC 链接通过 `vcpkg`）。

## 可选特性

- `static`：优先静态链接
- `dynamic`：优先动态链接（默认）
- `helpers`：提供 `init`/`shutdown`/`version` 的最小安全辅助

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
- `LIBVIPS_NO_BINDGEN`：跳过绑定生成，复用已有输出
- `BINDGEN_EXTRA_CLANG_ARGS`：额外传递给 `clang` 的参数（如 `-I`）
- `LIBCLANG_PATH`：`libclang` 路径

## 故障排查

- 未找到 `vips`：
    - 安装 `libvips` 与 `pkg-config`（Windows 使用 `vcpkg`）
    - 检查 `PKG_CONFIG_PATH`
- 绑定生成失败：
    - 设置 `LIBCLANG_PATH` 或通过 `BINDGEN_EXTRA_CLANG_ARGS` 添加缺失的包含目录
- macOS 静态链接：
    - 建议优先动态链接，避免 Homebrew 限制

## 许可证

[MIT](LICENSE)

## 更新日志

参见 [`CHANGELOG.md`](CHANGELOG.md)。