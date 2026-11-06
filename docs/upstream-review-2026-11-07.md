# libvips upstream and binding review — 2026-11-07

## Baseline and scope

Both Rust repositories completed `git pull --ff-only` from clean `main` branches.
The source baseline was libvips 8.18.7 (`24ad4d042940e6bf99a68871ba886ca8847c9c82`).
The selected update is **8.19.0-rc1** (`115df84b47247fb3d644de16bbb997f9edc4b5e3`),
published November 1. Its configured version is 8.19.0. The latest stable native
release remains 8.18.7. The Rust 0.4.0 version is prepared in Git, with publication
deferred. [Release](https://github.com/libvips/libvips/releases/tag/v8.19.0-rc1)

The audit compared public headers at both tags, generated declarations, native
exported symbols, recent merged PRs and post-RC headers at master
`12c9899a42ab37970f89626999a9856eea3ffad1`. Release and development branches have
separate histories, so commit-count subtraction is unsuitable as an API inventory.

## API synchronization

The pregenerated surface grows from **1,156 to 1,175 functions**. The refresh guard
preserved all previous function names, and `nm` verified every added symbol in
the compiled RC library. Existing public C structure fields retain their layouts
in the tag-to-tag header diff. Enum sentinel values move as upstream adds variants.

```text
vips_CICP2scRGB
vips_check_draw
vips_cicp_colour_primaries_get_type
vips_cicp_matrix_coefficients_get_type
vips_cicp_transfer_characteristics_get_type
vips_image_copy_draw
vips_image_get_blob_value
vips_image_get_loop
vips_percent_lum
vips_qoiload
vips_qoiload_buffer
vips_qoiload_source
vips_qoisave
vips_qoisave_buffer
vips_qoisave_target
vips_region_blend_over
vips_scRGB2CICP
vips_unlimited_get
vips_unlimited_set
```

Additional declarations include the three CICP enum families, six CICP/CLLI
metadata names, `VIPS_UHDR_TO_SCRGB`, `VIPS_INTERESTING_SPECIFIC`, and the hue,
saturation, colour and luminosity blend modes. `v8_19` includes `v8_18` and checks
the detected native version. Low-level callers must match the symbols they call
to their native library. Runtime version helpers describe the linked library.

The safe wrapper adds QOI file/buffer IO, CICP/scRGB, luminance percentiles,
loop/blob metadata and specific-point crops/thumbnails. QOI source/target handles,
region blending, unlimited controls and drawable-image primitives are exposed
through FFI. They require source callback ownership, prepared writable regions,
process-global configuration, or aliasing guarantees to support a safe wrapper.
`vips_image_copy_draw` can return a reference to foreign memory. The safe drawing
path therefore continues to detach borrowed pixels before modification.

## Functional and performance changes upstream

- Point-of-interest cropping is available for decoded images and encoded-source
  thumbnails. The thumbnail path can use shrink-on-load. [#4846](https://github.com/libvips/libvips/pull/4846), [#4860](https://github.com/libvips/libvips/pull/4860)
- CICP conversion and metadata handling enable HDR-aware pipelines. The current
  wrapper offers typed primaries/transfer selection and 8/16-bit output. [#4928](https://github.com/libvips/libvips/pull/4928), [#4979](https://github.com/libvips/libvips/pull/4979)
- QOI and APNG extend native codec support. APNG uses existing PNG entry points
  and animation metadata. The QOI wrappers expose native-owned encoded buffers. [#5128](https://github.com/libvips/libvips/pull/5128), [#5118](https://github.com/libvips/libvips/pull/5118), [#5122](https://github.com/libvips/libvips/pull/5122)
- Vertical reduction adds vectorizable block processing. The upstream release
  describes gains up to about 3x for affected cases. A controlled local
  8.18-versus-8.19 resize benchmark remains necessary to quantify this workload. [#5112](https://github.com/libvips/libvips/pull/5112), [#5114](https://github.com/libvips/libvips/pull/5114)

## Review findings

### Major — native feature version check bypass, fixed

`build_support/unix.rs::find` previously filtered pkg-config by the requested
feature version. Finding an older library produced `None`, and `build.rs::main`
could emit bare `-lvips`. That path skipped `apply_version_cfg`, deferring failure
to missing symbols during linking. Discovery now obtains the installed version
before enforcing the requested feature. An actual 8.18.7 library with `v8_19`
produced the expected build error. Malformed detected versions also fail validation.

### Major — QOI thumbnail source reuse in RC1, tracked upstream boundary

A valid 2x2 RGB QOI produced by `vips_qoisave_buffer` loads successfully through
`vips_qoiload_buffer`, but `vips_thumbnail_buffer` returns `bad QOI magic`.
A standalone C reproducer confirmed the same result outside Rust.
`libvips/foreign/qoiload.c::vips_foreign_load_qoi_parse_header` reads 14 bytes at
the current source position. Thumbnail processing reopens the loader using its
source. Rewind handling needs upstream investigation and a native regression.
QOI load/save and PNG thumbnail tests pass locally. Applications can explicitly
decode QOI before applying decoded-image geometry while this RC is in use.

### Post-RC follow-up

Master changes HDR reference-white normalization (#5295), lossless HEIF CICP
signaling (#5288), and Deep Zoom parent-directory validation (#5286). The RC
baseline predates these changes. Post-RC headers also add three foreign flag-mask
macros and change the UHDR scaling constant. Recheck colour reference values,
metadata and generated constants when selecting the next upstream revision.

## Merged PR inventory since the previous review

GitHub's 100 most recently updated closed PRs were read, then filtered to merged
PRs since 2026-10-22. This query returned 20 matching PRs. Master merge commits
were checked for RC ancestry. Maintenance-branch rows are classified separately.
The bounded query is a recent-update inventory.

| PR | Merged UTC | Base | Change | Revision boundary |
| --- | --- | --- | --- | --- |
| [#5300](https://github.com/libvips/libvips/pull/5300) | 2026-11-05 | master | Security policy: note that vips_system is outside scope | After RC1 |
| [#5295](https://github.com/libvips/libvips/pull/5295) | 2026-11-05 | master | colour: map hdr reference white to scrgb 1.0 | After RC1 |
| [#5294](https://github.com/libvips/libvips/pull/5294) | 2026-11-05 | master | Silence `glib-mkenums` warnings | After RC1 |
| [#5286](https://github.com/libvips/libvips/pull/5286) | 2026-11-04 | master | dzsave: reject imagename referencing parent directory | After RC1 |
| [#5292](https://github.com/libvips/libvips/pull/5292) | 2026-11-04 | gh-pages | Add missing links to API docs in news posts | Website |
| [#5289](https://github.com/libvips/libvips/pull/5289) | 2026-11-03 | master | Minor documentation improvements | After RC1 |
| [#5288](https://github.com/libvips/libvips/pull/5288) | 2026-11-03 | master | heifsave: force identity matrix for lossless cicp | After RC1 |
| [#5285](https://github.com/libvips/libvips/pull/5285) | 2026-11-01 | master | dzsave: reject imagename containing path separator | In RC1 |
| [#5284](https://github.com/libvips/libvips/pull/5284) | 2026-11-01 | master | thumbnail_buffer: remove unreachable code | In RC1 |
| [#5282](https://github.com/libvips/libvips/pull/5282) | 2026-11-01 | master | revise code using buffer loaders | In RC1 |
| [#5281](https://github.com/libvips/libvips/pull/5281) | 2026-10-31 | master | tiff: add rounding to premultiplied alpha handling | In RC1 |
| [#5279](https://github.com/libvips/libvips/pull/5279) | 2026-10-29 | master | use hypot where possible | In RC1 |
| [#5277](https://github.com/libvips/libvips/pull/5277) | 2026-10-29 | master | check the colour funcs for HDR | In RC1 |
| [#5278](https://github.com/libvips/libvips/pull/5278) | 2026-10-29 | master | clang-tidy: avoid implicit float to double promotions | In RC1 |
| [#5276](https://github.com/libvips/libvips/pull/5276) | 2026-10-29 | master | tiffsave: disable tiling for labq jpeg images | In RC1 |
| [#5275](https://github.com/libvips/libvips/pull/5275) | 2026-10-29 | master | Docs: add cicp chapter | In RC1 |
| [#5274](https://github.com/libvips/libvips/pull/5274) | 2026-10-28 | 8.18 | pngload: allow PNG images containing invalid CMF | Maintenance branch |
| [#5202](https://github.com/libvips/libvips/pull/5202) | 2026-10-26 | master | thumbnail: respect output profile and linear mode for cicp images | In RC1 |
| [#5271](https://github.com/libvips/libvips/pull/5271) | 2026-10-26 | 8.18 | libnsgif: update from upstream | Maintenance branch |
| [#5201](https://github.com/libvips/libvips/pull/5201) | 2026-10-22 | master | colour: do not strip cicp or clli signaling | In RC1 |

## Validation and further work

- Stable Rust 1.99.1 and native release-mode builds of 8.19.0-rc1 and 8.18.7.
- `cargo test --features bindgen,helpers,v8_19`, `cargo test --all-features` and
  `cargo clippy --all-targets --all-features -- -D warnings` passed on native RC1.
- All 19 added FFI symbols were found in the native export table.
- Native 8.18.7 with `v8_19` fails with the expected minimum-version diagnostic.
- The companion wrapper's `v8_18` suite passes against native 8.18.7.
- Header-free `bindgen,helpers,stub` tests pass in a clean target directory.
- Native vendor `static,helpers,v8_19` tests pass, compiling the pinned source
  through the actual Cargo builder. Codec dependencies use platform-default linkage.
- Formatting, diff checks, workflow lint and documentation checks pass.
- CI now has a pinned Linux 8.19 job. Distro jobs select compatible features.

Priorities for follow-up are a QOI source rewind regression/fix, native Windows
8.19 execution and per-target C layout checks, APNG multi-frame test fixtures,
coverage with more optional codecs, and measured vertical-reduce benchmarks with
fixed SIMD/codec/compiler configuration. A full all-codec deployment matrix and
performance comparison across native versions remain open validation work.
