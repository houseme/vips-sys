# vips-sys

[English](README.md) | [简体中文](README_CN.md)

[![Crates.io](https://img.shields.io/crates/v/vips-sys.svg)](https://crates.io/crates/vips-sys)
[![Rust](https://github.com/houseme/vips-sys/actions/workflows/rust.yml/badge.svg)](https://github.com/houseme/vips-sys/actions/workflows/rust.yml)
[![Code Coverage](https://codecov.io/gh/elbaro/vips-sys/branch/master/graph/badge.svg)](https://codecov.io/gh/elbaro/vips-sys)
[![docs.rs](https://docs.rs/vips-sys/badge.svg)](https://docs.rs/vips-sys/)
[![License](https://img.shields.io/crates/l/vips-sys)](./LICENSE)
[![Downloads](https://img.shields.io/crates/d/vips-sys)](https://crates.io/crates/vips-sys)

Rust 对 `libvips` 的低层 FFI 绑定。面向上层封装（如更安全的 API）提供基础设施。

- 文档：`https://houseme.github.io/vips-sys/vips_sys/`
- 要求：`libvips >= 8.2`（已在 `8.17.2` 上验证）
- 目标：构建稳定、跨平台可复用、与上游 `libvips` 同步升级

## 安装

- macOS
    - `brew install vips pkg-config`
- Debian/Ubuntu
    - `sudo apt-get install -y libvips-dev pkg-config`
- Windows (MSVC)
    - `vcpkg install vips`，并确保环境可被 `cargo` 检测

确保 `pkg-config --cflags --libs vips` 可正确输出（Windows 由 `vcpkg` 负责链接）。

## 特性与版本

- 可选特性
    - `static`：静态链接偏好
    - `dynamic`：动态链接偏好（默认）
    - `helpers`：启用最小安全封装（`init`／`shutdown`／`version`）
- 构建期导出
    - `LIBVIPS_VERSION`：探测到的 `libvips` 版本字符串
    - 当版本 `>= 8.17` 时导出 `cfg(vips_8_17)`（供上层 crate 做条件编译）
- 版本化绑定
    - 默认包含 `OUT_DIR/binding.rs`
    - 兼容路径：如开启 `vips_8_74` 时包含 `binding_8_74.rs`

## 使用示例（启用 `helpers`）

```rust
use vips_sys::helpers;

fn main() {
    helpers::init("vips-sys-example").expect("vips init failed");
    let ver = helpers::version();
    println!("libvips version: {}.{}.{}", ver.0, ver.1, ver.2);
    helpers::shutdown();
}
```

Cargo 依赖示例：

```toml
[dependencies]
vips-sys = { version = "0.1.3-beta.1", features = ["helpers"] }
```

## 构建说明

本 crate 在构建时使用 `bindgen` 自动生成绑定：

- 通过 `pkg-config` 探测 `include` 路径并传递给 `clang`
- 关闭 `layout_tests`，使用 `rustified_enum(".*")`
- 使用 `blocklist_*` 避免与平台定义冲突

相关环境变量：

- `PKG_CONFIG_PATH`：`pkg-config` 搜索路径（如 macOS M1：`/opt/homebrew/lib/pkgconfig`）
- `LIBVIPS_NO_BINDGEN`：设置后跳过绑定生成（使用已有产物）
- `BINDGEN_EXTRA_CLANG_ARGS`：为 `clang` 追加自定义参数（如额外 `-I`）
- `LIBCLANG_PATH`：`libclang` 动态库位置（如本地 LLVM 安装）

## 常见问题

- 无法找到 `vips`：
    - 确认系统已安装 `libvips` 与 `pkg-config`（或 Windows 的 `vcpkg`）
    - 检查 `PKG_CONFIG_PATH` 是否包含 `vips.pc` 所在目录
- 绑定生成失败：
    - 设置 `LIBCLANG_PATH` 指向可用的 `libclang`
    - 通过 `BINDGEN_EXTRA_CLANG_ARGS` 传入缺失的 `-I` 路径
- 静态链接异常（macOS）：
    - 优先使用动态链接或参考 Homebrew 已知链接问题

## 许可

[`MIT`](LICENSE)

## 变更日志

参见 [`CHANGELOG.md`](CHANGELOG.md)。