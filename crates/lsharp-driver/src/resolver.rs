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
        let major_text = parts
            .next()
            .ok_or_else(|| format!("semver の形式が不正です: {input}"))?;
        if !major_text.bytes().all(|byte| byte.is_ascii_digit()) {
            return Err(format!("major version の形式が不正です: {input}"));
        }
        let major = major_text
            .parse()
            .map_err(|_| format!("major version の形式が不正です: {input}"))?;
        let minor_text = parts
            .next()
            .ok_or_else(|| format!("semver の形式が不正です: {input}"))?;
        if !minor_text.bytes().all(|byte| byte.is_ascii_digit()) {
            return Err(format!("minor version の形式が不正です: {input}"));
        }
        let minor = minor_text
            .parse()
            .map_err(|_| format!("minor version の形式が不正です: {input}"))?;
        let patch_text = parts
            .next()
            .ok_or_else(|| format!("semver の形式が不正です: {input}"))?;
        if !patch_text.bytes().all(|byte| byte.is_ascii_digit()) {
            return Err(format!("patch version の形式が不正です: {input}"));
        }
        let patch = patch_text
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
        candidates.push(validate_cached_candidate(&path, name)?);
    }

    candidates.sort_by(|left, right| left.version.cmp(&right.version));
    Ok(candidates)
}

fn validate_cached_candidate(path: &Path, expected_name: &str) -> Result<CachedPackage, String> {
    let metadata = path.symlink_metadata().map_err(|error| {
        format!(
            "cached candidate {} cannot be inspected: {error}",
            path.display()
        )
    })?;
    if metadata.file_type().is_symlink() || !metadata.is_dir() {
        return Err(format!(
            "cached candidate {} must be a regular symlink-free directory",
            path.display()
        ));
    }

    let mut pending = vec![path.to_path_buf()];
    while let Some(directory) = pending.pop() {
        for entry in std::fs::read_dir(&directory).map_err(|error| {
            format!(
                "cached candidate {} cannot be scanned: {error}",
                path.display()
            )
        })? {
            let entry = entry.map_err(|error| {
                format!(
                    "cached candidate {} cannot be scanned: {error}",
                    path.display()
                )
            })?;
            let child = entry.path();
            let child_metadata = child.symlink_metadata().map_err(|error| {
                format!(
                    "cached candidate {} cannot inspect {}: {error}",
                    path.display(),
                    child.display()
                )
            })?;
            if child_metadata.file_type().is_symlink() {
                return Err(format!(
                    "cached candidate {} must be symlink-free: {}",
                    path.display(),
                    child.display()
                ));
            }
            if child_metadata.is_dir() {
                pending.push(child);
            }
        }
    }

    let manifest = path.join("lsharp.toml");
    let manifest_metadata = manifest.symlink_metadata().map_err(|error| {
        format!(
            "cached candidate {} manifest-valid check failed: {error}",
            path.display()
        )
    })?;
    if !manifest_metadata.is_file() || manifest_metadata.file_type().is_symlink() {
        return Err(format!(
            "cached candidate {} manifest must be a regular file",
            path.display()
        ));
    }
    let content = std::fs::read_to_string(&manifest).map_err(|error| {
        format!(
            "cached candidate {} manifest-valid check failed: {error}",
            path.display()
        )
    })?;
    let document: toml::Value = toml::from_str(&content).map_err(|error| {
        format!(
            "cached candidate {} manifest is invalid: {error}",
            path.display()
        )
    })?;
    let project = document
        .get("project")
        .and_then(toml::Value::as_table)
        .ok_or_else(|| {
            format!(
                "cached candidate {} manifest project table is invalid",
                path.display()
            )
        })?;
    let manifest_name = project.get("name").and_then(toml::Value::as_str);
    if manifest_name != Some(expected_name) {
        return Err(format!(
            "cached candidate {} manifest name does not match dependency {}",
            path.display(),
            expected_name
        ));
    }
    let version_text = project
        .get("version")
        .and_then(toml::Value::as_str)
        .ok_or_else(|| {
            format!(
                "cached candidate {} manifest version is invalid",
                path.display()
            )
        })?
        .to_string();
    let version = SemVersion::parse(&version_text).map_err(|error| {
        format!(
            "cached candidate {} manifest version is invalid: {error}",
            path.display()
        )
    })?;
    Ok(CachedPackage {
        path: path.to_path_buf(),
        version,
        version_text,
    })
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
        .max_by(|left, right| match left.version.cmp(&right.version) {
            std::cmp::Ordering::Equal => cached_package_name(left).cmp(cached_package_name(right)),
            ordering => ordering,
        })
}

fn cached_package_name(candidate: &CachedPackage) -> &str {
    candidate
        .path
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or("")
}

#[cfg(test)]
#[path = "resolver_tests.rs"]
mod tests;
