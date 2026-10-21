//! Low-level FFI bindings to `libvips`.
//!
//! By default this crate uses **pregenerated** bindings (`src/bindings/prebuilt.rs`)
//! so consumers do not need clang/bindgen at build time.
//!
//! Enable the `bindgen` feature to regenerate bindings against the headers found
//! on the machine (system libvips or `vendor/libvips`).
//!
//! # License
//!
//! This crate’s Rust source is **MIT**. The linked [`libvips`](https://www.libvips.org/)
//! library is **LGPL-2.1** — distributing binaries that link libvips requires
//! complying with LGPL-2.1 for that library. See the README “License” section.

#![allow(non_upper_case_globals, non_camel_case_types, non_snake_case)]
#![allow(clippy::missing_safety_doc)]

/// Pregenerated declarations from libvips 8.18.7. Callers must ensure that each
/// symbol they use exists in the linked library; newer APIs need newer libvips.
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

// These must hold even for no-link Windows CI builds.
const _: () = {
    assert!(core::mem::size_of::<gsize>() == core::mem::size_of::<usize>());
    assert!(core::mem::size_of::<gssize>() == core::mem::size_of::<isize>());
    assert!(core::mem::size_of::<GType>() == core::mem::size_of::<usize>());
};

/// Optional minimal helpers for init / shutdown / version.
#[cfg(feature = "helpers")]
pub mod helpers {
    use super::*;
    use std::ffi::CString;
    use std::sync::{Mutex, OnceLock};

    static INIT: OnceLock<()> = OnceLock::new();
    static INIT_LOCK: Mutex<()> = Mutex::new(());

    /// Whether this crate has successfully initialized libvips.
    pub fn is_initialized() -> bool {
        INIT.get().is_some()
    }

    /// Initialize libvips. Later calls reuse the first successful result.
    pub fn init(argv0: &str) -> Result<(), i32> {
        // Avoid CString allocation when init already succeeded.
        if INIT.get().is_some() {
            return Ok(());
        }
        let _guard = INIT_LOCK.lock().unwrap_or_else(|e| e.into_inner());
        if INIT.get().is_some() {
            return Ok(());
        }
        let c = CString::new(argv0).map_err(|_| -1)?;
        let rc = unsafe { vips_init(c.as_ptr()) };
        if rc != 0 {
            return Err(rc);
        }
        let _ = INIT.set(());
        Ok(())
    }

    /// Shut down libvips permanently. It cannot be initialized again.
    ///
    /// # Safety
    /// All images, regions, interpolators, workers and other libvips users
    /// must have finished and been destroyed. No later libvips calls, including
    /// `init`, may occur. Prefer leaving cleanup to process exit.
    pub unsafe fn shutdown() {
        // SAFETY: the caller guarantees that all libvips use has ended.
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
        // Deref-coerce `&'static &'static str` → `&'static str` (no explicit `*`).
        STR.get_or_init(|| unsafe {
            let p = vips_version_string();
            if p.is_null() {
                ""
            } else {
                std::ffi::CStr::from_ptr(p).to_str().unwrap_or("")
            }
        })
    }
}

#[cfg(all(test, feature = "helpers"))]
mod tests {
    use super::helpers;

    #[test]
    fn init_and_version() {
        assert_eq!(helpers::init("invalid\0name"), Err(-1));
        helpers::init("vips-sys-test").expect("vips_init");
        let (maj, min, mic) = helpers::version();
        assert!(maj >= 0 && min >= 0 && mic >= 0);
        assert!(!helpers::version_string().is_empty());
        // Cached: second call is free and consistent.
        assert_eq!(helpers::version(), (maj, min, mic));
        std::thread::scope(|scope| {
            for _ in 0..8 {
                scope.spawn(|| helpers::init("parallel-init").unwrap());
            }
        });
    }
}
