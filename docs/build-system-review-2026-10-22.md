# Build-system review and release hardening

## Design

The 0.3.0 release keeps dependency discovery and native build work in the build
script. Image operations do not pay for this policy at runtime.

| Layer | Responsibility |
| --- | --- |
| `build_support/config.rs` | One immutable environment/feature snapshot, link policy, version parsing and platform C++ runtime |
| `build.rs` | Explicit overrides, target dispatch, named library metadata, version cfgs and binding generation |
| `build_support/windows.rs` | Official vcpkg port, target/CRT/triplet selection and installation-specific headers |
| `build_support/unix.rs` | pkg-config discovery, native archive builds and incremental configuration |

An explicit installation is resolved once before platform dispatch. A matching
Unix `.pc` file supplies its complete dependency metadata, including GObject calls
used by the safe wrapper. The pre-generated binding mode avoids clang and unused
header-generation probes. Include vectors and package metadata transfer ownership
rather than being copied between providers.

MSVC continues to use the official `libvips` port and manifest-derived link names.
The vcpkg installation database and selected version header are tracked for
rebuilds. Target selection and explicit triplet/CRT overrides remain unchanged
from the issue #11 fix, whose
[native Windows CI](https://github.com/houseme/vips-sys/actions/runs/39640145662)
passed together with Linux, macOS and the header-free stub case.

## Native builds and linking

The previous native path could accept an existing archive indefinitely, and
could accept a dynamic system library while static linking was requested.
System static discovery now requires an archive. Vendor builds always let Ninja
check source dependencies; a configuration stamp resets Meson after changes to
compiler options, SDK, target or dependency-search configuration. The stamp
controls configuration invalidation only. It never replaces Ninja's source checks.

The native builder selects the configured Meson/Ninja executables, uses a fixed
libdir, compiles the archive target and installs changed development artifacts.
Tools, examples and fuzz executables are not needed for the Rust library build.
Dependency subprojects are not downloaded implicitly. Meson 0.60+, Ninja and
native development dependencies are required. Cross targets use prebuilt target
libraries instead of accidentally building a host archive.

Static libvips does not imply a fully static application: codec and platform
libraries can retain their normal linkage. pkg-config supplies the private
closure, while the libvips archive is linked explicitly. This preserves optional
codec availability without forcing every installed codec archive into the result.
The target's C++ runtime is selected explicitly; `CXXSTDLIB` supports custom
toolchains. Actual testing found an Apple link failure from `stdc++` in dependency
metadata, which this platform selection resolves with libc++.

Binding generation still uses CargoCallbacks for header invalidation. It
normalizes output in memory and writes only changed contents. The system-header
path no longer watches the entire unrelated vendor tree. Default builds retain
the pre-generated API surface and do not invoke bindgen.

## Executed evidence

Validated using stable Rust 1.99.1 and libvips 8.18.7:

- Full sys feature tests and Clippy with warnings denied.
- Eight isolated vcpkg parser scenarios and policy/CRT/version tests.
- Real vendor static archive build, native linking and execution.
- Nine real image/HDR tests through a consumer linked to that static archive.
- Nine real image/HDR tests through an explicit Unix installation, including its
  complete GObject dependencies.
- The native cache preserved the installed archive on an unchanged rerun.
  Touching a C source rebuilt it; changing `CFLAGS` reset configuration and rebuilt
  450 native objects.
- Candidate-local observations: initial invocation 12.749 s, unchanged invocation
  0.100 s, one-source rebuild 3.867 s. These timings describe this candidate in the
  local environment and include Cargo/test startup.
- Formatting, diff and workflow lint checks. Release verification includes native
  Windows linked tests and a header-free stub path, without duplicating the regular
  matrix on version-tag pushes. Default/system-library jobs skip the upstream
  submodule; a dedicated Linux release job checks out and executes the native
  static provider. Release tags bind to the exact verified commit.

The configured Unix provider and system codec dependency versions affect native
linking. Fully static OS/codec deployment and arbitrary custom C++ toolchains
require their own target validation. Normal MSVC discovery remains governed by
vcpkg-rs' global triplet and dynamic opt-in contract.

## Release sequence

Merge remote main, validate the candidate and packaged contents, commit the
release notes, then publish `vips-sys` 0.3.0. Update the high-level `vips` dependency
only after the registry version is available, removing the mandatory sibling path
and validating the downloaded published artifact. Publishing `vips` itself is a
separate action.
