# libvips upstream audit — 2026-10-22

## Revision and release boundary

The Rust repositories were pulled with `git pull --ff-only`; both were already
current. The dependency moves from `v8.18.6` (`426af3f44246fce9cfa8dd51a353aa4dfd48c553`)
to the latest published release, [`v8.18.7`](https://github.com/libvips/libvips/releases/tag/v8.18.7)
(`24ad4d042940e6bf99a68871ba886ca8847c9c82`, published 2026-09-26).

The [8.18.6…8.18.7 comparison](https://github.com/libvips/libvips/compare/v8.18.6...v8.18.7)
contains **14 commits and 13 changed files**. No public headers changed, so there
are **no newly exported public functions, enums or signatures in 8.18.7**.
The previous bindings already include 8.18's RAW/UltraHDR, gainmap, tile geometry,
Oklab/Oklch and interpretation helpers. Refreshing the stable API updates version
constants and fixes the GLib size aliases; it does not import development APIs.
Regenerating against configured release headers also restores 56 enum `*_get_type`
entry points missing from the prior Rust defaults: **1,100 → 1,156 declarations**,
with no removed functions. All 56 were verified in the compiled native export table.
These are newly exposed Rust declarations for existing native APIs, rather than
new functions introduced by the 8.18.7 patch release. The refresh script rejects
accidental removals, including regeneration against headers with deprecated APIs
disabled. The prebuilt file retains its generic `VIPS_CONFIG` placeholder.

Release fixes include TIFF colormap/JP2K/rgba validation (#5208, #5217), LUT size
limits (#5206), Hough bounds (#5209), PDFium buffer bounds (#5207), UltraHDR gainmap
rounding (#5221), untrusted JP2K saves (#5226), EXIF blob type checks (#5227), and
JP2K dimension validation (#5228). These are correctness/robustness improvements;
this audit does not establish an image-processing throughput improvement from
the upstream patch release itself.

## Merged development work

The [8.18.7…master header comparison](https://github.com/libvips/libvips/compare/v8.18.7...master)
is a comparison across diverged release/development branches, not simply the
post-release commit history. At audit time it reports 222 development commits.
Notable public API differences include:

| Area | Development API | Status for these Rust versions |
| --- | --- | --- |
| QOI | `vips_qoiload{,_buffer,_source}`, `vips_qoisave{,_buffer,_target}` | Deferred until a release carries them |
| HDR colour | `vips_CICP2scRGB`, `vips_scRGB2CICP`, CICP enum families and white-level constant | Deferred |
| Drawing | `vips_image_copy_draw`, `vips_check_draw` (#5200) | Deferred; stable drawing uses a fresh memory image instead |
| Statistics | `vips_percent_lum` | Deferred |
| Metadata | `vips_image_get_loop` | Deferred |
| Region | `vips_region_blend_over` | Deferred |
| Controls | `vips_unlimited_set`, `vips_unlimited_get` | Deferred |
| Options/enums | specific smartcrop interest, four additional blend modes | Deferred |
| RAW metadata | JPEG preview extraction (#5086) | Deferred; this is metadata/loader behavior, not a new C entry point |

Recent fixes include initialization/cache/threadpool races (#5092, #5165, #5212,
#5250), `sink_screen` lifetime locking (#5259), ICC blob reference ownership
(#5260), and total animated/multipage height bounds (#5257, #5261, #5262, #5265,
#5266). A PR merged to `8.18` after the tag is **not included in v8.18.7**.
The latest released dependency therefore does not claim to include those later
fixes. Recheck them when the next release becomes available.

## Rust binding/build review

- **Critical, fixed:** host `C long` aliases for GLib `gsize`/`gssize` were
  incorrect for 64-bit Windows (LLP64). Use `usize`/`isize` and compile-time checks
  for these aliases and `GType`. Bindgen uses the same explicit aliases.
- **Major, fixed:** `helpers::shutdown` had a safe signature despite destroying
  process-wide state still reachable through live native objects. It is now
  unsafe with explicit all-users-finished/no-later-calls requirements.
- **Major, fixed:** initialization cached failures, preventing recovery. Only a
  successful result is cached, and both Rust crates share that state and lock.
- **Major, fixed:** `LIBVIPS_NO_BINDGEN` skipped generation without providing the
  file included by `src/lib.rs`. It now copies the pregenerated declarations,
  including on a clean build without headers.
- **Minor, fixed:** the partial fingerprint omitted header contents/defines and
  could skip bindgen callbacks on a rebuild. CargoCallbacks now owns header
  invalidation; unchanged Cargo builds still skip the build script normally.
- **Minor, fixed:** `dynamic` could override the documented `LIBVIPS_STATIC`
  environment precedence. Windows vcpkg now uses its resolved include paths,
  instead of guessing another triplet's headers or treating headers as a library.

## Remaining improvements

1. Validate a fully static vendor build with a complete codec dependency closure.
   This audit does not certify static deployment: archive cache invalidation,
   private transitive linker flags, platform libdirs and tool-path overrides need
   a dedicated reproducible integration case before expanding that claim.
2. Add native Windows/vcpkg execution tests and per-target C/Rust layout checks.
   Cross-target Rust checking proves compilation and pointer-size assertions; it
   does not certify every generated struct, va_list ABI or native call on Windows.
   Use live bindgen against target headers for target-specific low-level layouts.
3. Automate comparison of new released public headers and generated symbols in
   CI. Keep stable and development API baselines separate.
4. Preserve codec build provenance when measuring performance: Highway, PNG/JPEG,
   libraw and libultrahdr availability determine which native paths are exercised.

## Merged PR inventory

Source: GitHub merged-PR query `repo:libvips/libvips is:pr is:merged merged:>=2026-08-25`,
limited to 200 results. **57** matched at audit time. The cutoff is inclusive and
includes the 8.18.6 release-day baseline PRs. Release classification is based on
merge commit membership in the stable comparison, with later maintenance and
master work marked separately.

| PR | Merged UTC | Base | Change | Delivery status |
| --- | --- | --- | --- | --- |
| [#5266](https://github.com/libvips/libvips/pull/5266) | 2026-10-20 | 8.18 | pdfload: improve height handling | 8.18 maintenance after 8.18.7 |
| [#5265](https://github.com/libvips/libvips/pull/5265) | 2026-10-20 | 8.18 | gifload: limit max height | 8.18 maintenance after 8.18.7 |
| [#5264](https://github.com/libvips/libvips/pull/5264) | 2026-10-20 | master | tsan: upgrade GLib subproject to 2.90.1 | development master |
| [#5263](https://github.com/libvips/libvips/pull/5263) | 2026-10-20 | master | tsan: remove redundant suppressions | development master |
| [#5262](https://github.com/libvips/libvips/pull/5262) | 2026-10-20 | 8.18 | heifload: limit total multipage height | 8.18 maintenance after 8.18.7 |
| [#5261](https://github.com/libvips/libvips/pull/5261) | 2026-10-20 | 8.18 | webpload: limit animated height | 8.18 maintenance after 8.18.7 |
| [#5260](https://github.com/libvips/libvips/pull/5260) | 2026-10-19 | 8.18 | icc_transform: copy blob, not raw pointer | 8.18 maintenance after 8.18.7 |
| [#5259](https://github.com/libvips/libvips/pull/5259) | 2026-10-20 | 8.18 | sink_screen: move free inside the global lock | 8.18 maintenance after 8.18.7 |
| [#5257](https://github.com/libvips/libvips/pull/5257) | 2026-10-20 | 8.18 | tiffload: limit total image height | 8.18 maintenance after 8.18.7 |
| [#5254](https://github.com/libvips/libvips/pull/5254) | 2026-10-16 | 8.18 | tiffload: add support for offset JP2K tiles | 8.18 maintenance after 8.18.7 |
| [#5253](https://github.com/libvips/libvips/pull/5253) | 2026-10-16 | 8.18 | histogram: double band count for complex images | 8.18 maintenance after 8.18.7 |
| [#5251](https://github.com/libvips/libvips/pull/5251) | 2026-10-16 | master | tiffload: remove unreachable code due to existing bits_per_sample limit of 32 | development master |
| [#5250](https://github.com/libvips/libvips/pull/5250) | 2026-10-16 | master | cache: entry pointer must be cleared inside mutex | development master |
| [#5249](https://github.com/libvips/libvips/pull/5249) | 2026-10-13 | master | pngsave: write text chunks as iTXt | development master |
| [#5247](https://github.com/libvips/libvips/pull/5247) | 2026-10-16 | 8.18 | magicksave: guard against non-integer frame delay | 8.18 maintenance after 8.18.7 |
| [#5246](https://github.com/libvips/libvips/pull/5246) | 2026-10-16 | 8.18 | popplerload: synchronise dimension check with pdfiumload | 8.18 maintenance after 8.18.7 |
| [#5245](https://github.com/libvips/libvips/pull/5245) | 2026-10-16 | 8.18 | temp_name: allow only one string substitution | 8.18 maintenance after 8.18.7 |
| [#5244](https://github.com/libvips/libvips/pull/5244) | 2026-10-15 | 8.18 | tiff{load,save}: honour block state for jp2k tiles | 8.18 maintenance after 8.18.7 |
| [#5243](https://github.com/libvips/libvips/pull/5243) | 2026-10-16 | 8.18 | uhdrsave: verify dimensions are within JPEG limits | 8.18 maintenance after 8.18.7 |
| [#5242](https://github.com/libvips/libvips/pull/5242) | 2026-10-16 | 8.18 | mosaic: add areas bounds check, required for small search size | 8.18 maintenance after 8.18.7 |
| [#5241](https://github.com/libvips/libvips/pull/5241) | 2026-10-09 | 8.18 | allow LAB interp for LABQ coding | 8.18 maintenance after 8.18.7 |
| [#5240](https://github.com/libvips/libvips/pull/5240) | 2026-10-08 | 8.18 | CI: explicitly install gobject-introspection | 8.18 maintenance after 8.18.7 |
| [#5239](https://github.com/libvips/libvips/pull/5239) | 2026-10-08 | 8.18 | jxlsave: don't save bad profiles | 8.18 maintenance after 8.18.7 |
| [#5236](https://github.com/libvips/libvips/pull/5236) | 2026-10-09 | 8.18 | invertlut: prevent possible integer overflow | 8.18 maintenance after 8.18.7 |
| [#5235](https://github.com/libvips/libvips/pull/5235) | 2026-10-02 | master | thumbnail: pass missing `fail_on` arg to loaders | development master |
| [#5234](https://github.com/libvips/libvips/pull/5234) | 2026-10-02 | master | Fix build against IM6/GM | development master |
| [#5233](https://github.com/libvips/libvips/pull/5233) | 2026-09-30 | master | magickload: scale and round Quantum values on unpack | development master |
| [#5232](https://github.com/libvips/libvips/pull/5232) | 2026-09-29 | master | qoiload: ensure a custom source retains existing flags | development master |
| [#5229](https://github.com/libvips/libvips/pull/5229) | 2026-10-20 | master | Use actual sRGB gamma by default | development master |
| [#5228](https://github.com/libvips/libvips/pull/5228) | 2026-09-26 | 8.18 | jp2kload: more size validation | 8.18.7 |
| [#5227](https://github.com/libvips/libvips/pull/5227) | 2026-09-26 | 8.18 | header: only parse EXIF metadata for blob values | 8.18.7 |
| [#5226](https://github.com/libvips/libvips/pull/5226) | 2026-09-26 | 8.18 | jp2ksave: tag as untrusted | 8.18.7 |
| [#5223](https://github.com/libvips/libvips/pull/5223) | 2026-09-24 | master | composite: ensure correct operator in scalar path | development master |
| [#5221](https://github.com/libvips/libvips/pull/5221) | 2026-09-24 | 8.18 | uhdrload: calculate gain map scale factor using round-to-nearest | 8.18.7 |
| [#5220](https://github.com/libvips/libvips/pull/5220) | 2026-09-23 | master | OSS-Fuzz: disable build of cruft | development master |
| [#5217](https://github.com/libvips/libvips/pull/5217) | 2026-09-16 | 8.18 | tiffload: check rgba between directories | 8.18.7 |
| [#5215](https://github.com/libvips/libvips/pull/5215) | 2026-09-13 | master | docs: add Rust binding (vips-rs) to README | development master |
| [#5214](https://github.com/libvips/libvips/pull/5214) | 2026-09-11 | master | CI: various upgrades | development master |
| [#5212](https://github.com/libvips/libvips/pull/5212) | 2026-09-25 | master | cache: ensure entry invalidation uses mutex | development master |
| [#5210](https://github.com/libvips/libvips/pull/5210) | 2026-09-24 | master | start a SECURITY.md | development master |
| [#5209](https://github.com/libvips/libvips/pull/5209) | 2026-09-05 | 8.18 | hough_line: add a featurespace bounds check | 8.18.7 |
| [#5208](https://github.com/libvips/libvips/pull/5208) | 2026-09-05 | 8.18 | tiffload: test for jp2k tiles that are too small | 8.18.7 |
| [#5207](https://github.com/libvips/libvips/pull/5207) | 2026-09-05 | 8.18 | pdfiumload: check for buffer too small | 8.18.7 |
| [#5206](https://github.com/libvips/libvips/pull/5206) | 2026-09-05 | 8.18 | buildlut: limit the output lut size | 8.18.7 |
| [#5205](https://github.com/libvips/libvips/pull/5205) | 2026-09-16 | master | svgload: improve dpi= and scale= behaviour | development master |
| [#5203](https://github.com/libvips/libvips/pull/5203) | 2026-09-03 | master | doc: ban use of generative AI | development master |
| [#5200](https://github.com/libvips/libvips/pull/5200) | 2026-09-13 | master | add vips_image_copy_draw(), vips_check_draw() | development master |
| [#5199](https://github.com/libvips/libvips/pull/5199) | 2026-08-31 | master | vips_target_finish: drop the two params it no longer takes | development master |
| [#5197](https://github.com/libvips/libvips/pull/5197) | 2026-09-13 | master | cplusplus: add support for pointer args | development master |
| [#5193](https://github.com/libvips/libvips/pull/5193) | 2026-08-25 | 8.18 | heifload: set untrusted for libheif before 1.23.2 | 8.18.6 baseline |
| [#5192](https://github.com/libvips/libvips/pull/5192) | 2026-08-25 | master | convi: sum using 128 bit integer where available | development master |
| [#5191](https://github.com/libvips/libvips/pull/5191) | 2026-08-25 | master | draw_line: add draw-point | development master |
| [#5188](https://github.com/libvips/libvips/pull/5188) | 2026-08-25 | master | affine: verify coordinates before adjusting margin | development master |
| [#5173](https://github.com/libvips/libvips/pull/5173) | 2026-08-25 | 8.18 | openexrload: more validation | 8.18.6 baseline |
| [#5165](https://github.com/libvips/libvips/pull/5165) | 2026-09-14 | master | threadpool: guard progress reporting with `allocate_lock` mutex | development master |
| [#5092](https://github.com/libvips/libvips/pull/5092) | 2026-10-20 | master | Assorted ThreadSanitizer fixes | development master |
| [#5086](https://github.com/libvips/libvips/pull/5086) | 2026-10-17 | master | dcrawload: add preview jpegs | development master |

## Executed validation

- Stable Rust 1.99.1; native libvips 8.18.7 built from the pinned source.
- `cargo test --features bindgen,helpers,v8_18`: passed against native headers/library.
- `cargo test --all-features` and `cargo clippy --all-targets --all-features -- -D warnings`: passed.
- The refresh guard checked 1,156 declarations and preserved every prior function.
  Native `nm` export verification found all 56 newly exposed type getter symbols.
- Windows MSVC and Linux cross-target checks passed for the pointer-size ABI assertions;
  these are compilation checks, not execution on those operating systems.
- A clean-target `LIBVIPS_NO_BINDGEN` + `bindgen,helpers,stub` build verifies the
  header-free fallback and stub link path. Format, diff and workflow lint checks passed.
- Source checkout locally omits large upstream image/fuzzer corpora to avoid repeated
  network transfer failures. All build sources are present, and the gitlink points
  to the official unmodified release. No upstream C source was patched.
- Full static linking, Windows/vcpkg runtime, absent codecs and upstream test corpora
  are not certified by these checks. No package was published.
