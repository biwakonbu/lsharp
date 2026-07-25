use std::{
    fs,
    io::{self, Write},
    path::Path,
    sync::atomic::{AtomicU64, Ordering},
    time::{SystemTime, UNIX_EPOCH},
};

static TEMP_FILE_COUNTER: AtomicU64 = AtomicU64::new(0);

/// 同じ親 directory 内の一時ファイルを durable に保存してから rename する。
pub(crate) fn write_durable_atomic(path: &Path, bytes: &[u8]) -> io::Result<()> {
    let parent = path
        .parent()
        .filter(|parent| !parent.as_os_str().is_empty())
        .unwrap_or_else(|| Path::new("."));
    let file_name = path
        .file_name()
        .and_then(|name| name.to_str())
        .ok_or_else(|| {
            io::Error::new(
                io::ErrorKind::InvalidInput,
                format!(
                    "出力 path の file name を取得できません: {}",
                    path.display()
                ),
            )
        })?;
    let nonce = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_err(|error| io::Error::other(format!("一時 path 用時刻取得に失敗しました: {error}")))?
        .as_nanos();
    let sequence = TEMP_FILE_COUNTER.fetch_add(1, Ordering::Relaxed);
    let temporary_path = parent.join(format!(
        ".{file_name}.tmp-{}-{nonce}-{sequence}",
        std::process::id()
    ));

    let result = (|| {
        let mut file = fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&temporary_path)?;
        file.write_all(bytes)?;
        file.sync_all()?;
        drop(file);
        fs::rename(&temporary_path, path)?;
        sync_parent_directory(parent)
    })();

    if result.is_err() {
        let _ = fs::remove_file(&temporary_path);
    }
    result
}

#[cfg(unix)]
fn sync_parent_directory(parent: &Path) -> io::Result<()> {
    fs::File::open(parent)?.sync_all()
}

#[cfg(not(unix))]
fn sync_parent_directory(_parent: &Path) -> io::Result<()> {
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::write_durable_atomic;
    use std::fs;
    use std::path::PathBuf;
    use std::time::{SystemTime, UNIX_EPOCH};

    fn unique_temp_dir(name: &str) -> PathBuf {
        let nonce = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("system time before unix epoch")
            .as_nanos();
        let dir = std::env::temp_dir().join(format!(
            "lsharp_driver_atomic_write_{name}_{}_{}",
            std::process::id(),
            nonce
        ));
        fs::create_dir_all(&dir).expect("temporary directory should be created");
        dir
    }

    #[cfg(unix)]
    #[test]
    fn atomic_write_replaces_destination_without_following_symlink() {
        use std::os::unix::fs::symlink;

        let dir = unique_temp_dir("symlink");
        let sentinel = dir.join("sentinel.json");
        let destination = dir.join("intent-graph.json");
        fs::write(&sentinel, b"old").expect("sentinel should be written");
        symlink(&sentinel, &destination).expect("destination symlink should be created");

        write_durable_atomic(&destination, b"new").expect("atomic write should succeed");

        assert_eq!(
            fs::read(&sentinel).expect("sentinel should remain readable"),
            b"old"
        );
        assert_eq!(
            fs::read(&destination).expect("destination should contain new bytes"),
            b"new"
        );
        assert!(
            !fs::symlink_metadata(&destination)
                .expect("destination metadata should be readable")
                .file_type()
                .is_symlink(),
            "atomic replacement should replace the symlink itself"
        );
        let entries = fs::read_dir(&dir)
            .expect("temporary directory should be readable")
            .collect::<std::result::Result<Vec<_>, _>>()
            .expect("directory entries should be readable");
        assert_eq!(entries.len(), 2, "temporary file should not remain");

        fs::remove_dir_all(&dir).expect("temporary directory should be removed");
    }
}
