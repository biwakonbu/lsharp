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

#[test]
fn atomic_write_removes_temporary_file_when_rename_fails() {
    let dir = unique_temp_dir("rename-failure");
    let destination = dir.join("intent-graph.json");
    fs::create_dir(&destination).expect("destination directory should be created");

    assert!(
        write_durable_atomic(&destination, b"new").is_err(),
        "writing over a directory should fail at the atomic rename boundary"
    );

    let entries = fs::read_dir(&dir)
        .expect("temporary directory should be readable")
        .collect::<std::result::Result<Vec<_>, _>>()
        .expect("directory entries should be readable");
    assert_eq!(entries.len(), 1, "failed writes must not leave a temp file");
    assert!(
        entries[0].path().is_dir(),
        "the original destination directory should remain"
    );

    fs::remove_dir_all(&dir).expect("temporary directory should be removed");
}
