//! One immutable snapshot of build policy for all discovery backends.
use std::{env, path::PathBuf};

#[derive(Debug)]
pub struct BuildOptions {
    pub static_linking: bool,
    pub generate_bindings: bool,
    pub msvc: bool,
    pub lib_dir: Option<PathBuf>,
    pub include_dir: Option<PathBuf>,
    pub version: Option<String>,
    pub no_vendor: bool,
}

pub fn static_linking(
    override_value: Option<&str>,
    static_feature: bool,
    dynamic_feature: bool,
) -> bool {
    override_value.map_or(static_feature && !dynamic_feature, |value| {
        !matches!(value.to_ascii_lowercase().as_str(), "0" | "false" | "off")
    })
}

impl BuildOptions {
    pub fn from_env() -> Self {
        Self {
            static_linking: static_linking(
                env::var("LIBVIPS_STATIC").ok().as_deref(),
                cfg!(feature = "static"),
                cfg!(feature = "dynamic"),
            ),
            generate_bindings: cfg!(feature = "bindgen")
                && env::var_os("LIBVIPS_NO_BINDGEN").is_none(),
            msvc: env::var("CARGO_CFG_TARGET_ENV").as_deref() == Ok("msvc"),
            lib_dir: env::var_os("LIBVIPS_LIB_DIR").map(PathBuf::from),
            include_dir: env::var_os("LIBVIPS_INCLUDE_DIR").map(PathBuf::from),
            version: env::var("LIBVIPS_VERSION").ok(),
            no_vendor: env::var_os("LIBVIPS_NO_VENDOR").is_some(),
        }
    }
}

pub fn header_version(header: &str) -> Option<String> {
    header.lines().find_map(|line| {
        let mut parts = line.split_whitespace();
        if parts.next()? != "#define" || parts.next()? != "VIPS_VERSION" {
            return None;
        }
        Some(
            parts
                .next()?
                .strip_prefix('"')?
                .strip_suffix('"')?
                .to_owned(),
        )
    })
}

pub fn cpp_stdlib(target: &str, override_value: Option<String>) -> Option<String> {
    if let Some(value) = override_value {
        return (!value.is_empty()).then_some(value);
    }
    if target.contains("msvc") {
        None
    } else if ["apple", "freebsd", "openbsd", "aix", "wasi", "ohos"]
        .iter()
        .any(|os| target.contains(os))
    {
        Some("c++".into())
    } else if target.contains("android") {
        Some("c++_shared".into())
    } else {
        Some("stdc++".into())
    }
}
