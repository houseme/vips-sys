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
- 依赖：Rust **1.85+**（edition 2024）；`libvips >= 8.2`（绑定按 **8.18.6** 生成）
- 目标：构建快、链接可靠、跨平台复用

## 核心特性

| | 默认 | feature `bindgen` |
|--|------|-------------------|
| 绑定来源 | 预生成 `src/bindings/prebuilt.rs` | 现场 bindgen + clang |
| 需要 libclang？ | **否** | 是 |
| 典型 `cargo build` | 秒级 | 首次较慢 |
| 适用场景 | 日常开发 / CI | 更新 API / 跟进新版 libvips |

**默认构建不跑 bindgen**，只需要能链接到 libvips **库**。

## 安装

### macOS

```bash
brew install vips pkg-config
# Apple Silicon：
export PKG_CONFIG_PATH=/opt/homebrew/lib/pkgconfig
```

### Debian / Ubuntu

```bash
sudo apt-get install -y libvips-dev pkg-config
```

### Windows（MSVC）

```bat
vcpkg install vips:x64-windows
set VCPKG_ROOT=C:\path\to\vcpkg
```

静态：`vcpkg install vips:x64-windows-static`，并设置 `VCPKG_DEFAULT_TRIPLET=x64-windows-static`。

验证：`pkg-config --cflags --libs vips`（MSVC 走 vcpkg）。

## Cargo 用法

```toml
[dependencies]
vips-sys = { version = "0.2.0", features = ["helpers"] }
```

### Features

| Feature | 默认 | 说明 |
|---------|------|------|
| *(无)* | ✓ | 预生成绑定 + 链接探测 |
| `helpers` | | `init` / `shutdown` / `version` / `version_string`（带缓存） |
| `static` | | 优先静态链接（可用 `LIBVIPS_STATIC=1` 覆盖） |
| `dynamic` | | 优先动态链接 |
| `bindgen` | | 用 clang 按本机头文件重新生成绑定 |
| `stub` | | 无系统库时链接 `stub/vips_stub.c`（**仅测试**） |

### 无系统 libvips 时跑测试

```bash
cargo test --features helpers,stub
```

找不到真实库时，`stub` 会编译最小 C 桩（`vips_init` / `vips_version` 等）以便链接单测。
**有真实库时永远优先用真实库**；生产二进制不要带 `stub` 构建。

### 静态链接

```toml
vips-sys = { version = "0.2.0", features = ["static"] }
```

```bash
LIBVIPS_STATIC=1 cargo build
```

解析顺序：

1. 已装静态库时走 `pkg-config --static` / vcpkg static triplet
2. 有 `meson` + `ninja` 时从 `vendor/libvips` 构建到 `OUT_DIR`
3. 否则给出明确错误

### 子模块中的 libvips

`vendor/libvips` 固定 [libvips](https://github.com/libvips/libvips) **v8.18.6**
（参见 [rust-sys-crate](https://kornel.ski/rust-sys-crate)）：

```bash
git clone --recurse-submodules <本仓库>
# 或：
git submodule update --init --recursive
```

链接 / 头文件解析：

1. `LIBVIPS_LIB_DIR` / `LIBVIPS_INCLUDE_DIR`（可选 `LIBVIPS_STATIC=1`）
2. 系统库：`pkg-config`（Unix）或 `vcpkg`（MSVC）
3. 请求静态时尝试 vendor + meson
4. 回退 vendor 头文件（仍需可链接的库）

## 刷新预生成绑定

```bash
# 需要：libclang、glib 头文件，以及 vips 头文件（系统或 vendor 子模块）
cargo build --features bindgen
# build.rs 会打印 OUT_DIR 路径，复制覆盖即可：
cp target/debug/build/vips-sys-*/out/binding.rs src/bindings/prebuilt.rs
```

`wrapper.h` 作为唯一 bindgen 入口**保留**；`generate.sh` 已移除，改用 `bindgen` feature。

构建期导出：

- `LIBVIPS_VERSION`：检测到的版本（有 pkg-config 时）
- `cfg(vips_8_16)` / `cfg(vips_8_17)`：版本满足时启用

## 示例（`helpers`）

```rust
use vips_sys::helpers;

fn main() {
    helpers::init("vips-sys-example").expect("vips init failed");
    let (a, b, c) = helpers::version();
    println!("libvips {}.{}.{} ({})", a, b, c, helpers::version_string());
    helpers::shutdown();
}
```

## 环境变量

| 变量 | 作用 |
|------|------|
| `PKG_CONFIG_PATH` | 查找 `vips.pc` |
| `LIBVIPS_LIB_DIR` / `LIBVIPS_INCLUDE_DIR` | 显式指定库 / 头文件 |
| `LIBVIPS_STATIC` | `1` 优先静态；`0`/`false`/`off` 关闭 |
| `LIBVIPS_NO_VENDOR` | 忽略 `vendor/libvips` |
| `LIBVIPS_NO_BINDGEN` | 与 `bindgen` feature 合用时跳过生成 |
| `LIBVIPS_VERSION` | 覆盖用于 cfg 的版本字符串 |
| `VCPKG_ROOT` / `VCPKG_DEFAULT_TRIPLET` | Windows vcpkg |
| `BINDGEN_EXTRA_CLANG_ARGS` | 额外 clang 参数（`bindgen` feature） |
| `LIBCLANG_PATH` | libclang 路径（`bindgen` feature） |

## 故障排查

- **链接找不到库** — 安装 libvips（`libvips-dev` / `vcpkg install vips`）
- **`bindgen` feature 失败** — 安装 `libclang`、glib 头文件与 vips 头文件
- **macOS 静态链接** — 建议动态；真要静态用 vendor + meson
- **Windows 找不到头文件** — 设置 `VCPKG_ROOT`

## 许可证

本 crate 的 **Rust 源码**（含 `build.rs`、`src/`）采用 [MIT](LICENSE) 许可。

**`libvips` 本体为 [LGPL-2.1](https://github.com/libvips/libvips/blob/master/LICENSE)**
（并依赖 GLib 等兼容组件）。该许可**不是** MIT，也**不会**把本 crate 改成 LGPL。

### 对使用者的含义

| 产物 | 许可 |
|------|------|
| `vips-sys` 源码 / 预生成 FFI 声明 | MIT |
| 链接的 libvips 库（系统包、vcpkg 或 `vendor/` 构建） | LGPL-2.1 |
| 你最终链接 libvips 的二进制 | 必须满足 libvips 的 LGPL-2.1 义务 |

实务建议：

- **动态链接**（默认）：保证 libvips 可替换/可重链，并按 LGPL-2.1 提供许可以及获取源码的途径。
- **静态链接**（`static` 或 vendor meson 构建）：LGPL 义务更重（例如允许用修改后的 libvips 重新链接），发布前请自行评估。
- **`vendor/libvips` 子模块**源码为 LGPL-2.1。已通过 `exclude = ["vendor/"]` **排除**在 crates.io 包之外；若你自行再分发该目录，须按 LGPL-2.1 进行。
- 预生成绑定是对 C 公开 API 的 FFI 声明，不含 libvips 实现代码。

以上为工程实践说明，不构成法律意见；商用合规请咨询律师。

## 更新日志

参见 [`CHANGELOG.md`](CHANGELOG.md)。
