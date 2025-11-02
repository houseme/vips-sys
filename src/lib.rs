#[allow(non_upper_case_globals, non_camel_case_types, non_snake_case)]
mod ffi {
    include!(concat!(env!("OUT_DIR"), "/binding.rs"));
}

pub use ffi::*;

// Optional minimum safety package for easy initialization and shutdown (without changing the positioning of -sys)
#[cfg(feature = "helpers")]
pub mod helpers {
    use super::*;
    pub fn init(argv0: &str) -> Result<(), i32> {
        let c = std::ffi::CString::new(argv0).unwrap();
        let rc = unsafe { vips_init(c.as_ptr()) };
        if rc == 0 {
            Ok(())
        } else {
            Err(rc)
        }
    }
    pub fn shutdown() {
        unsafe { vips_shutdown() }
    }
    pub fn version() -> (i32, i32, i32) {
        unsafe { (vips_version(0), vips_version(1), vips_version(2)) }
    }
}
