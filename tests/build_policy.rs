#[path = "../build_support/config.rs"]
#[allow(dead_code)]
mod config;

#[test]
fn explicit_link_policy_overrides_cargo_features() {
    assert!(!config::static_linking(None, true, true));
    assert!(config::static_linking(None, true, false));
    assert!(!config::static_linking(None, false, false));
    for disabled in ["0", "false", "OFF", "False"] {
        assert!(!config::static_linking(Some(disabled), true, false));
    }
    assert!(config::static_linking(Some("1"), false, true));
}

#[test]
fn version_comes_from_the_selected_headers() {
    assert_eq!(
        config::header_version("#define VIPS_VERSION_H\n#define VIPS_VERSION \"8.18.7\"\r\n"),
        Some("8.18.7".into())
    );
    assert_eq!(
        config::header_version("#define VIPS_VERSION_STRING \"8.18.7\""),
        None
    );
    assert_eq!(config::header_version("#define VIPS_VERSION unknown"), None);
}

#[test]
fn api_version_requirements_accept_rc_headers_and_reject_old_libraries() {
    assert!(config::version_at_least("8.19.0", 8, 19));
    assert!(config::version_at_least("8.19.0-rc1", 8, 19));
    assert!(config::version_at_least("9.0.0", 8, 19));
    assert!(!config::version_at_least("8.18.7", 8, 19));
    assert!(!config::version_at_least("8.17.0", 8, 18));
    assert!(!config::version_at_least("unknown", 8, 19));
}

#[test]
fn static_vendor_uses_the_platform_cpp_runtime() {
    assert_eq!(
        config::cpp_stdlib("aarch64-apple-darwin", None).as_deref(),
        Some("c++")
    );
    assert_eq!(
        config::cpp_stdlib("x86_64-unknown-linux-gnu", None).as_deref(),
        Some("stdc++")
    );
    assert_eq!(config::cpp_stdlib("x86_64-pc-windows-msvc", None), None);
    assert_eq!(
        config::cpp_stdlib("aarch64-apple-darwin", Some("custom".into())).as_deref(),
        Some("custom")
    );
    assert_eq!(
        config::cpp_stdlib("aarch64-apple-darwin", Some(String::new())),
        None
    );
}
