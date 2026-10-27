//! Unit tests for data/updates.rs.

use super::*;

#[test]
fn reads_the_version_and_page_of_the_latest_release() {
    let json = r#"{"tag_name":"v1.2.0","html_url":"https://github.com/anggiedwarsa/pawpal/releases/tag/v1.2.0","draft":false,"prerelease":false}"#;
    assert_eq!(
        parse_latest(json),
        Some(Release { version: "1.2.0".into(), url: "https://github.com/anggiedwarsa/pawpal/releases/tag/v1.2.0".into() })
    );
    let bare = parse_latest(r#"{"tag_name":"2.0"}"#).unwrap();
    assert_eq!((bare.version.as_str(), bare.url.as_str()), ("2.0", "https://github.com/anggiedwarsa/pawpal/releases/latest"));
}

#[test]
fn skips_drafts_pre_releases_and_errors() {
    assert_eq!(parse_latest(r#"{"tag_name":"v9.0.0","prerelease":true}"#), None);
    assert_eq!(parse_latest(r#"{"tag_name":"v9.0.0","draft":true}"#), None);
    assert_eq!(parse_latest(r#"{"message":"Not Found"}"#), None);
    assert_eq!(parse_latest("<html>"), None);
}

#[test]
fn compares_versions_number_by_number() {
    assert!(is_newer("1.0.1", "1.0.0"));
    assert!(is_newer("1.10.0", "1.9.3"));
    assert!(is_newer("2", "1.9.9"));
    assert!(!is_newer("1.0.0", "1.0.0"));
    assert!(!is_newer("1.0", "1.0.0"), "a missing part counts as 0");
    assert!(!is_newer("0.9.9", "1.0.0"));
    assert!(!is_newer("1.0.0-beta", "1.0.0"));
}

#[test]
fn only_a_later_release_counts_as_an_update() {
    let r = |v: &str| Some(Release { version: v.into(), url: String::new() });
    assert_eq!(newer_than_this(r("999.0.0")).map(|r| r.version), Some("999.0.0".into()));
    assert_eq!(newer_than_this(r(env!("CARGO_PKG_VERSION"))), None);
    assert_eq!(newer_than_this(None), None);
}
