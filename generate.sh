#!/usr/bin/env bash
# Offline helper: regenerate bindings with the same allowlist as build.rs.
# Prefer `cargo build` — build.rs applies include paths, caching, and platform probes.
set -euo pipefail
cd "$(dirname "$0")"

bindgen wrapper.h -o src/binding.rs \
  --use-core \
  --ctypes-prefix core::ffi \
  --rustified-enum ".*" \
  --no-layout-tests \
  --no-doc-comments \
  --allowlist-function 'vips_.*' \
  --allowlist-type 'Vips.*' \
  --allowlist-var 'VIPS_.*' \
  --allowlist-function 'g_object_unref' \
  --allowlist-function 'g_object_ref' \
  --allowlist-function 'g_free' \
  --allowlist-function 'g_signal_connect_data' \
  --allowlist-type 'GConnectFlags' \
  --blocklist-type 'max_align_t' \
  --blocklist-item 'FP_NAN' \
  --blocklist-item 'FP_INFINITE' \
  --blocklist-item 'FP_ZERO' \
  --blocklist-item 'FP_SUBNORMAL' \
  --blocklist-item 'FP_NORMAL' \
  -- $(pkg-config --cflags vips 2>/dev/null || true)
