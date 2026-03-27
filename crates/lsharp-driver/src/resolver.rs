use crate::config;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct SemVersion {
    major: u64,
    minor: u64,
    patch: u64,
}

impl SemVersion {
    pub fn parse(input: &str) -> Result<Self, String> {
        let normalized = input.trim().trim_start_matches('v');
        let mut parts = normalized.split('.');
        let major = parts
            .next()
            .ok_or_else(|| format!("semver の形式が不正です: {input}"))?
            .parse()
            .map_err(|_| format!("major version の形式が不正です: {input}"))?;
        let minor = parts
            .next()
            .ok_or_else(|| format!("semver の形式が不正です: {input}"))?
            .parse()
            .map_err(|_| format!("minor version の形式が不正です: {input}"))?;
        let patch = parts
            .next()
            .ok_or_else(|| format!("semver の形式が不正です: {input}"))?
            .parse()
            .map_err(|_| format!("patch version の形式が不正です: {input}"))?;
        if parts.next().is_some() {
            return Err(format!("semver の形式が不正です: {input}"));
        }
        Ok(Self {
            major,
            minor,
            patch,
        })
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum VersionReq {
    Compatible(SemVersion),
    Exact(SemVersion),
    Minimum(SemVersion),
}

impl VersionReq {
    pub fn parse(input: &str) -> Result<Self, String> {
        let trimmed = input.trim();
        if let Some(version) = trimmed.strip_prefix(">=") {
            return Ok(Self::Minimum(SemVersion::parse(version)?));
        }
        if let Some(version) = trimmed.strip_prefix('=') {
            return Ok(Self::Exact(SemVersion::parse(version)?));
        }
        Ok(Self::Compatible(SemVersion::parse(trimmed)?))
    }

    pub fn matches(&self, version: &SemVersion) -> bool {
        match self {
            Self::Compatible(base) => matches_compatible(base, version),
            Self::Exact(base) => version == base,
            Self::Minimum(base) => version >= base,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CachedPackage {
    pub path: PathBuf,
    pub version: SemVersion,
    pub version_text: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ResolvedVersionDependency {
    pub package_dir: PathBuf,
    pub version: String,
}

pub fn resolve_cached_version_dependency(
    project_dir: &Path,
    name: &str,
    version_req: &str,
) -> Result<ResolvedVersionDependency, String> {
    let req = VersionReq::parse(version_req)?;
    let candidates = cached_packages_for_name(&project_dir.join(".lsharp").join("packages"), name)?;
    let selected = select_highest_matching_cached_package(&req, &candidates).ok_or_else(|| {
        format!(
            "依存 '{}' に一致する semver 候補が cache にありません: {}",
            name, version_req
        )
    })?;

    Ok(ResolvedVersionDependency {
        package_dir: selected.path.clone(),
        version: selected.version_text.clone(),
    })
}

pub fn package_version_text(package_dir: &Path) -> String {
    if !package_dir.join("lsharp.toml").exists() {
        return "0.0.0".to_string();
    }
    let config = config::load_config(package_dir);
    if config.project.version.is_empty() {
        "0.0.0".to_string()
    } else {
        config.project.version
    }
}

fn cached_packages_for_name(packages_dir: &Path, name: &str) -> Result<Vec<CachedPackage>, String> {
    let mut candidates = Vec::new();
    let prefix = format!("{name}-");
    let entries = match std::fs::read_dir(packages_dir) {
        Ok(entries) => entries,
        Err(_) => return Ok(candidates),
    };

    for entry in entries {
        let entry = entry.map_err(|e| format!("package cache の走査に失敗: {e}"))?;
        let path = entry.path();
        let Some(file_name) = path.file_name().and_then(|name| name.to_str()) else {
            continue;
        };
        if !file_name.starts_with(&prefix) {
            continue;
        }
        if !path.join("lsharp.toml").exists() {
            continue;
        }
        let metadata = match path.symlink_metadata() {
            Ok(metadata) => metadata,
            Err(_) => continue,
        };
        if !metadata.is_dir() && !metadata.file_type().is_symlink() {
            continue;
        }

        let version_text = package_version_text(&path);
        let Ok(version) = SemVersion::parse(&version_text) else {
            continue;
        };
        candidates.push(CachedPackage {
            path,
            version,
            version_text,
        });
    }

    candidates.sort_by(|left, right| left.version.cmp(&right.version));
    Ok(candidates)
}

fn matches_compatible(base: &SemVersion, version: &SemVersion) -> bool {
    if version < base {
        return false;
    }

    if base.major > 0 {
        return version.major == base.major;
    }
    if base.minor > 0 {
        return version.major == 0 && version.minor == base.minor;
    }

    version.major == 0 && version.minor == 0 && version.patch == base.patch
}

pub fn select_highest_matching_cached_package<'a>(
    req: &VersionReq,
    candidates: &'a [CachedPackage],
) -> Option<&'a CachedPackage> {
    candidates
        .iter()
        .filter(|candidate| req.matches(&candidate.version))
        .max_by(|left, right| left.version.cmp(&right.version))
}

#[cfg(test)]
mod tests {
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
}
