//! Low-level FFI bindings to `libvips`.
//!
//! By default this crate uses **pregenerated** bindings (`src/bindings/prebuilt.rs`)
//! so consumers do not need clang/bindgen at build time.
//!
//! Enable the `bindgen` feature to regenerate bindings against the headers found
//! on the machine (system libvips or `vendor/libvips`).

#![allow(non_upper_case_globals, non_camel_case_types, non_snake_case)]
#![allow(clippy::missing_safety_doc)]

/// Pregenerated bindings (default). Shared ABI for libvips 8.2+.
#[cfg(not(feature = "bindgen"))]
#[allow(clippy::all)]
mod ffi {
    include!("bindings/prebuilt.rs");
}

/// Live bindgen output from `OUT_DIR` (feature `bindgen`).
#[cfg(feature = "bindgen")]
#[allow(clippy::all)]
mod ffi {
    include!(concat!(env!("OUT_DIR"), "/binding.rs"));
}

pub use ffi::*;

/// Optional minimal helpers for init / shutdown / version.
#[cfg(feature = "helpers")]
pub mod helpers {
    use super::*;
    use std::ffi::CString;
    use std::sync::OnceLock;

    /// Initialize libvips. Later calls reuse the first successful result.
    pub fn init(argv0: &str) -> Result<(), i32> {
        static INIT: OnceLock<Result<(), i32>> = OnceLock::new();
        let c = CString::new(argv0).map_err(|_| -1)?;
        *INIT.get_or_init(|| {
            let rc = unsafe { vips_init(c.as_ptr()) };
            if rc == 0 {
                Ok(())
            } else {
                Err(rc)
            }
        })
    }

    /// Shut down libvips. Prefer calling once at process exit.
    pub fn shutdown() {
        unsafe { vips_shutdown() }
    }

    /// `(major, minor, micro)` of the linked libvips. Cached after first call.
    pub fn version() -> (i32, i32, i32) {
        static VER: OnceLock<(i32, i32, i32)> = OnceLock::new();
        *VER.get_or_init(|| unsafe { (vips_version(0), vips_version(1), vips_version(2)) })
    }

    /// `libvips` version string, e.g. `"8.18.6"`.
    ///
    /// The returned `&'static str` points at libvips-owned memory that lives
    /// for the process lifetime.
    pub fn version_string() -> &'static str {
        static STR: OnceLock<&'static str> = OnceLock::new();
        *STR.get_or_init(|| unsafe {
            let p = vips_version_string();
            if p.is_null() {
                ""
            } else {
                std::ffi::CStr::from_ptr(p).to_str().unwrap_or("")
            }
        })
    }
}
