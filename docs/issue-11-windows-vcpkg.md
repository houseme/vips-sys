# Issue #11: Windows vcpkg discovery

## Conclusion and scope

[Issue #11](https://github.com/houseme/vips-sys/issues/11) is valid and still affects
revision `bb729b7`, as well as the reported published 0.2.0. The Rust crate is
edition 2024 with no declared MSRV; validation uses current stable Rust.

The package manager's port name is `libvips`. The Unix pkg-config module remains
`vips`, and native link names come from installed manifests rather than either
package-manager identifier. Changing the Unix probe to `libvips` would introduce
a regression, so only the vcpkg port lookup changes.

Sources: [official port](https://github.com/microsoft/vcpkg/blob/master/ports/libvips/vcpkg.json),
[vcpkg-rs Config](https://docs.rs/vcpkg/0.2.15/vcpkg/struct.Config.html),
[vcpkg-rs linking policy](https://docs.rs/vcpkg/0.2.15/vcpkg/), and
[Cargo build-script target configuration](https://doc.rust-lang.org/cargo/reference/build-scripts.html#inputs-to-the-build-script).

## Findings and fixes

`build_support/windows.rs:30` — Correct the package identifier.
Severity: Major. With only the official `libvips` port installed, the old `vips`
lookup fails before resolving the library, DLLs or transitive dependencies. The
production helper now requests `libvips`. vcpkg-rs reads actual link names and
transitive dependencies from its installation database; no `lib_name` override
or incorrect fallback to a second package identifier is added.

`build_support/windows.rs:4` — Preserve triplet, architecture and CRT choices.
Severity: Major. The previous static branch always forced `x64-windows-static`
unless the CLI default was set, including for ARM64/x86 and Rust's default dynamic
CRT. It could also override `VCPKGRS_TRIPLET`. Selection now follows:

1. `VCPKGRS_TRIPLET` (the Rust helper's explicit global override).
2. `VCPKG_DEFAULT_TRIPLET` (the vcpkg CLI default).
3. For a static request, target architecture plus `static` for `crt-static`,
   otherwise `static-md`.
4. Otherwise vcpkg-rs' own target/CRT/dynamic policy.

Dynamic triplets require `VCPKGRS_DYNAMIC=1` even when explicitly selected; this
vcpkg-rs contract is documented rather than bypassed by mutating global process
environment. DLLs must remain discoverable through `PATH` at runtime.

`build.rs:143` — Select the backend from the compilation target.
Severity: Major. `#[cfg(target_env = "msvc")]` in a build script tests its host,
so a non-Windows host targeting MSVC previously entered the Unix probe. Runtime
selection now reads `CARGO_CFG_TARGET_ENV`; the tiny vcpkg build dependency is
available to every host. Unix/pkg-config discovery remains the Unix target path.

`build_support/windows.rs:34` and `build.rs:421` — Preserve target headers and paths.
Severity: Minor. Add GLib's split include directories from the selected triplet;
never borrow another installation's `glibconfig.h`. Include metadata now uses
`env::join_paths`, which preserves drive-letter paths on a Windows host.
Relevant vcpkg environment variables and the shared helper are watched for rebuilds.

The English/Chinese READMEs, build warnings and fallback error instructions use
the correct port and complete dynamic/static examples. Changelog tracks this fix
under Unreleased; the prepared 0.3.0 version is unchanged.

## Reproduction and verification

The regression fixture creates an isolated installation database with the
`libvips` and `glib` ports, versioned library names, DLLs and transitive dependency
records. It invokes the real vcpkg-rs parser used by production. With the old port
identifier the dynamic case failed with `LibNotFound("package vips is not installed
for vcpkg triplet x64-windows")`; after the correction it passed.

Four parent tests execute eight isolated child probes: dynamic discovery and
transitive libraries, x64 static/dynamic CRT, x86, ARM64, the two explicit override
levels, and the missing dynamic opt-in error. The ignored subprocess entry point
is executed by these parent tests. No tests mutate environment variables inside
the parallel Rust test process. Fixture libraries are placeholders: these tests
verify discovery and metadata, not Windows machine-code compatibility.

A separate macOS-host/MSVC-target Cargo check used an isolated `libvips` fixture
and inspected the actual production build-script output. It selected the fixture's
manifest-derived library name and GLib include directories, with no Unix fallback.
This validates target dispatch, not cross-linking or native Windows execution.

Executed checks:

- `cargo test --all-features` and `cargo clippy --all-targets --all-features -- -D warnings`.
- Header-free `cargo test --features helpers,stub` with system probing disabled.
- Consumer `vips` all-feature tests, including real image/HDR operations and lifetime cases.
- Formatting, diff checks and workflow lint; package file listing includes the new build helper.

The Windows matrix now installs the official `libvips:x64-windows` port, enables
dynamic discovery, sets the DLL path, and runs linked helper/integration tests and
a release build. The runner's installation root is documented in the
[official Windows image inventory](https://github.com/actions/runner-images/blob/main/images/windows/Windows2025-Readme.md#environment-variables).
Native Windows CI results must be checked after push; local metadata fixtures and
cross-target checks do not substitute for that execution. Native ARM64 and fully
static Windows execution remain outside the executed local validation.

## Delivery boundary

This fixes the current Git source. It does not replace the already published
0.2.0 artifact; consumers requiring 0.2 cannot automatically resolve to the
prepared 0.3.0. Use the paired current source checkouts for development; a 0.2
backport or package publication is a separate release decision. No package or
release tag is created as part of this fix.

## Release hardening follow-up

The linked [native Windows CI run](https://github.com/houseme/vips-sys/actions/runs/39640145662)
completed successfully on Windows, Linux, macOS and the no-library stub job.
The subsequent provider refactor is described in
[build-system review](build-system-review-2026-10-22.md). The release candidate
folds this fix into the 0.3.0 changelog rather than leaving it Unreleased.
