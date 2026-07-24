use super::*;

#[test]
fn test_version_req_plain_string_means_compatible_range() {
    let req = VersionReq::parse("1.0.0").unwrap();
    assert!(req.matches(&SemVersion::parse("1.0.0").unwrap()));
    assert!(req.matches(&SemVersion::parse("1.9.9").unwrap()));
    assert!(!req.matches(&SemVersion::parse("2.0.0").unwrap()));
}

#[test]
fn test_version_req_exact_match() {
    let req = VersionReq::parse("=1.2.3").unwrap();
    assert!(req.matches(&SemVersion::parse("1.2.3").unwrap()));
    assert!(!req.matches(&SemVersion::parse("1.2.4").unwrap()));
}

#[test]
fn test_version_req_minimum_match() {
    let req = VersionReq::parse(">=1.2.3").unwrap();
    assert!(req.matches(&SemVersion::parse("1.2.3").unwrap()));
    assert!(req.matches(&SemVersion::parse("2.0.0").unwrap()));
    assert!(!req.matches(&SemVersion::parse("1.2.2").unwrap()));
}

#[test]
fn test_select_highest_matching_cached_package() {
    let req = VersionReq::parse("1.0.0").unwrap();
    let candidates = vec![
        CachedPackage {
            path: std::path::PathBuf::from("/tmp/pkg-a"),
            version: SemVersion::parse("1.0.1").unwrap(),
            version_text: "1.0.1".to_string(),
        },
        CachedPackage {
            path: std::path::PathBuf::from("/tmp/pkg-b"),
            version: SemVersion::parse("1.4.0").unwrap(),
            version_text: "1.4.0".to_string(),
        },
        CachedPackage {
            path: std::path::PathBuf::from("/tmp/pkg-c"),
            version: SemVersion::parse("2.0.0").unwrap(),
            version_text: "2.0.0".to_string(),
        },
    ];

    let selected = select_highest_matching_cached_package(&req, &candidates).unwrap();
    assert_eq!(selected.version_text, "1.4.0");
}
