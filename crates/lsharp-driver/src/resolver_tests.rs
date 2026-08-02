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

#[test]
fn test_cached_candidates_require_safe_manifests_and_deterministic_ties() {
    let base_dir = std::env::temp_dir().join(format!(
        "lsharp_resolver_candidate_provenance_{}",
        std::process::id()
    ));
    let _ = std::fs::remove_dir_all(&base_dir);
    let packages_dir = base_dir.join(".lsharp/packages");
    std::fs::create_dir_all(&packages_dir).unwrap();

    for candidate in ["demo-alpha", "demo-zeta"] {
        let package_dir = packages_dir.join(candidate);
        std::fs::create_dir_all(&package_dir).unwrap();
        std::fs::write(
            package_dir.join("lsharp.toml"),
            "[project]\nname = \"demo\"\nversion = \"1.2.3\"\n",
        )
        .unwrap();
    }

    let result = resolve_cached_version_dependency(&base_dir, "demo", "1.0.0").unwrap();
    assert_eq!(result.package_dir, packages_dir.join("demo-zeta"));

    let external = base_dir.join("external");
    std::fs::create_dir_all(external.join("src")).unwrap();
    std::fs::write(
        external.join("lsharp.toml"),
        "[project]\nname = \"demo\"\nversion = \"9.0.0\"\n",
    )
    .unwrap();
    #[cfg(unix)]
    std::os::unix::fs::symlink(&external, packages_dir.join("demo-root-symlink")).unwrap();

    let nested = packages_dir.join("demo-nested-symlink");
    std::fs::create_dir_all(nested.join("src")).unwrap();
    std::fs::write(
        nested.join("lsharp.toml"),
        "[project]\nname = \"demo\"\nversion = \"8.0.0\"\n",
    )
    .unwrap();
    #[cfg(unix)]
    std::os::unix::fs::symlink(&external, nested.join("src/linked-source")).unwrap();

    let error = resolve_cached_version_dependency(&base_dir, "demo", "1.0.0")
        .expect_err("unsafe cache candidates must fail closed");
    assert!(
        error.contains("cached candidate"),
        "unexpected error: {error}"
    );

    std::fs::remove_dir_all(&base_dir).unwrap();
}
