/* Minimal libvips stand-in for `cargo test` without a system library.
 * Enabled by the `stub` feature; never used when a real libvips is found.
 * SPDX-License-Identifier: MIT
 */

#ifdef _WIN32
#define VIPS_STUB_API __declspec(dllexport)
#else
#define VIPS_STUB_API
#endif

VIPS_STUB_API int vips_init(const char *argv0) {
  (void)argv0;
  return 0;
}

VIPS_STUB_API void vips_shutdown(void) {}

VIPS_STUB_API int vips_version(int field) {
  switch (field) {
  case 0:
    return 8;
  case 1:
    return 18;
  case 2:
    return 6;
  default:
    return 0;
  }
}

VIPS_STUB_API const char *vips_version_string(void) { return "8.18.6-stub"; }
