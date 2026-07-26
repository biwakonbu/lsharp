const ARTIFACT_CACHE_DIR_ENV: &str = "LSHARP_ARTIFACT_CACHE_DIR";
const ARTIFACT_CACHE_MAX_ENTRIES_ENV: &str = "LSHARP_ARTIFACT_CACHE_MAX_ENTRIES";
const ARTIFACT_CACHE_MAX_BYTES_ENV: &str = "LSHARP_ARTIFACT_CACHE_MAX_BYTES";

fn resolve_artifact_cache_dir(explicit: Option<PathBuf>) -> miette::Result<Option<PathBuf>> {
    resolve_artifact_cache_dir_from_values(explicit, std::env::var_os(ARTIFACT_CACHE_DIR_ENV))
}

fn resolve_artifact_cache_dir_from_values(
    explicit: Option<PathBuf>,
    environment: Option<std::ffi::OsString>,
) -> miette::Result<Option<PathBuf>> {
    if explicit.is_some() {
        return Ok(explicit);
    }

    match environment {
        None => Ok(None),
        Some(value) if value.is_empty() => Err(miette::miette!(
            "{ARTIFACT_CACHE_DIR_ENV} が空です。cache root の path を指定してください"
        )),
        Some(value) => Ok(Some(PathBuf::from(value))),
    }
}

fn resolve_artifact_cache_limits(
    explicit_max_entries: Option<usize>,
    explicit_max_bytes: Option<u64>,
) -> miette::Result<(Option<usize>, Option<u64>)> {
    resolve_artifact_cache_limits_from_values(
        explicit_max_entries,
        explicit_max_bytes,
        std::env::var_os(ARTIFACT_CACHE_MAX_ENTRIES_ENV),
        std::env::var_os(ARTIFACT_CACHE_MAX_BYTES_ENV),
    )
}

fn resolve_artifact_cache_limits_from_values(
    explicit_max_entries: Option<usize>,
    explicit_max_bytes: Option<u64>,
    environment_max_entries: Option<std::ffi::OsString>,
    environment_max_bytes: Option<std::ffi::OsString>,
) -> miette::Result<(Option<usize>, Option<u64>)> {
    let max_entries = match explicit_max_entries {
        Some(value) => Some(value),
        None => {
            parse_artifact_cache_limit(ARTIFACT_CACHE_MAX_ENTRIES_ENV, environment_max_entries)?
        }
    };
    let max_bytes = match explicit_max_bytes {
        Some(value) => Some(value),
        None => parse_artifact_cache_limit(ARTIFACT_CACHE_MAX_BYTES_ENV, environment_max_bytes)?,
    };
    Ok((max_entries, max_bytes))
}

fn parse_artifact_cache_limit<T>(
    environment_name: &str,
    value: Option<std::ffi::OsString>,
) -> miette::Result<Option<T>>
where
    T: std::str::FromStr,
    T::Err: std::fmt::Display,
{
    let Some(value) = value else {
        return Ok(None);
    };
    if value.is_empty() {
        return Err(miette::miette!(
            "{environment_name} が空です。数値の limit を指定してください"
        ));
    }
    let value = value
        .to_str()
        .ok_or_else(|| miette::miette!("{environment_name} は UTF-8 の数値で指定してください"))?;
    value.parse::<T>().map(Some).map_err(|error| {
        miette::miette!("{environment_name} の値 '{value}' を数値として解釈できません: {error}")
    })
}

fn validate_artifact_cache_options(
    artifact_cache_dir: Option<&Path>,
    max_entries: Option<usize>,
    max_bytes: Option<u64>,
) -> miette::Result<()> {
    if (max_entries.is_some() || max_bytes.is_some()) && artifact_cache_dir.is_none() {
        return Err(miette::miette!(
            "--artifact-cache-max-entries / --artifact-cache-max-bytes は --artifact-cache-dir と併用してください"
        ));
    }
    Ok(())
}

fn maintain_artifact_cache(
    artifact_cache_dir: Option<&Path>,
    max_entries: Option<usize>,
    max_bytes: Option<u64>,
) -> miette::Result<usize> {
    validate_artifact_cache_options(artifact_cache_dir, max_entries, max_bytes)?;
    let Some(root) = artifact_cache_dir else {
        return Ok(0);
    };
    let cache = lsharp_tooling::artifact_cache::ArtifactCache::new(root.to_path_buf());
    let mut removed = 0;
    if let Some(max_entries) = max_entries {
        removed += cache.trim_to_entries(max_entries)?;
    }
    if let Some(max_bytes) = max_bytes {
        removed += cache.trim_to_bytes(max_bytes)?;
    }
    Ok(removed)
}
