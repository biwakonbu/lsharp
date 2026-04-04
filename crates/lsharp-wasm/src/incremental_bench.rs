use std::{
    fs,
    path::{Path, PathBuf},
    time::{SystemTime, UNIX_EPOCH},
};

pub struct SelfhostIncrementalBenchFixture {
    workspace_root: PathBuf,
    entry_path: PathBuf,
    changed_module_path: PathBuf,
    original_changed_module_source: String,
    changed_module_variant_source: String,
}

impl SelfhostIncrementalBenchFixture {
    pub fn create() -> Result<Self, String> {
        let repo_root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..");
        let source_root = repo_root.join("selfhost/src");
        let workspace_root = benchmark_workspace_root();
        let staged_source_root = workspace_root.join("selfhost/src");
        copy_dir_all(&source_root, &staged_source_root)?;

        let entry_path = staged_source_root.join("App/Main.ls");
        let changed_module_path = staged_source_root.join("App/CompilerMode.ls");
        let original_changed_module_source = fs::read_to_string(&changed_module_path)
            .map_err(|err| format!("{}: {err}", changed_module_path.display()))?;
        let changed_module_variant_source = format!(
            "; incremental benchmark single-module change\n{}",
            original_changed_module_source
        );

        Ok(Self {
            workspace_root,
            entry_path,
            changed_module_path,
            original_changed_module_source,
            changed_module_variant_source,
        })
    }

    pub fn entry_path(&self) -> &Path {
        &self.entry_path
    }

    pub fn apply_changed_module_variant(&self) -> Result<(), String> {
        fs::write(
            &self.changed_module_path,
            &self.changed_module_variant_source,
        )
        .map_err(|err| format!("{}: {err}", self.changed_module_path.display()))
    }

    pub fn restore_changed_module(&self) -> Result<(), String> {
        fs::write(
            &self.changed_module_path,
            &self.original_changed_module_source,
        )
        .map_err(|err| format!("{}: {err}", self.changed_module_path.display()))
    }
}

impl Drop for SelfhostIncrementalBenchFixture {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.workspace_root);
    }
}

fn benchmark_workspace_root() -> PathBuf {
    let nonce = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_nanos();
    std::env::temp_dir().join(format!(
        "lsharp_incremental_bench_{}_{}",
        std::process::id(),
        nonce
    ))
}

fn copy_dir_all(source: &Path, dest: &Path) -> Result<(), String> {
    fs::create_dir_all(dest).map_err(|err| format!("{}: {err}", dest.display()))?;

    for entry in fs::read_dir(source).map_err(|err| format!("{}: {err}", source.display()))? {
        let entry = entry.map_err(|err| format!("{}: {err}", source.display()))?;
        let path = entry.path();
        let target = dest.join(entry.file_name());
        if path.is_dir() {
            copy_dir_all(&path, &target)?;
        } else {
            fs::copy(&path, &target).map_err(|err| format!("{}: {err}", path.display()))?;
        }
    }

    Ok(())
}
