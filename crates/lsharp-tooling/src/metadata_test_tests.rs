use super::*;
use std::{
    fs,
    path::PathBuf,
    time::{SystemTime, UNIX_EPOCH},
};

fn unique_temp_dir(name: &str) -> PathBuf {
    let nonce = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("system time before unix epoch")
        .as_nanos();
    let dir = std::env::temp_dir().join(format!(
        "lsharp_tooling_metadata_{name}_{}_{}",
        std::process::id(),
        nonce
    ));
    fs::create_dir_all(&dir).expect("temp dir creation failed");
    dir
}

include!("metadata_test_tests/basic.rs");
include!("metadata_test_tests/canonical.rs");
include!("metadata_test_tests/property.rs");
