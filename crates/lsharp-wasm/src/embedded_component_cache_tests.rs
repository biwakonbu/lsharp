use super::*;

// ---------------------------------------------------------------------------
// key 導出
// ---------------------------------------------------------------------------

#[test]
fn test_embedded_component_key_changes_when_source_bytes_change() {
    let root = unique_temp_dir("source-change");
    write_source(&root, "App/EmbeddedCli.ls", "(module App.EmbeddedCli)\n");

    let emitter = SourceFingerprint::from_bytes(b"emitter-v1");
    let before = EmbeddedComponentKey::from_parts(
        &collect_source_entries("selfhost/src", &root).unwrap(),
        &emitter,
    );

    write_source(&root, "App/EmbeddedCli.ls", "(module App.EmbeddedCli)\n\n");
    let after = EmbeddedComponentKey::from_parts(
        &collect_source_entries("selfhost/src", &root).unwrap(),
        &emitter,
    );

    assert_ne!(
        before, after,
        "source が 1 バイト変わったら embedded component key も変わるべき"
    );

    std::fs::remove_dir_all(&root).unwrap();
}

#[test]
fn test_embedded_component_key_changes_when_emitter_fingerprint_changes() {
    // emitter (= build script binary) だけが変わったケース。source fingerprint は不変なので、
    // emitter を key に含めないと「古い emitter の bytes を新しい emitter の成果物として
    // 埋め込む」stale hit が黙って起きる。
    let root = unique_temp_dir("emitter-change");
    write_source(&root, "App/EmbeddedCli.ls", "(module App.EmbeddedCli)\n");
    let entries = collect_source_entries("selfhost/src", &root).unwrap();

    let before =
        EmbeddedComponentKey::from_parts(&entries, &SourceFingerprint::from_bytes(b"emitter-v1"));
    let after =
        EmbeddedComponentKey::from_parts(&entries, &SourceFingerprint::from_bytes(b"emitter-v2"));

    assert_ne!(
        before, after,
        "emitter が変わったら source が同一でも key は変わるべき"
    );

    std::fs::remove_dir_all(&root).unwrap();
}

#[test]
fn test_embedded_component_key_changes_when_source_is_renamed() {
    // 内容が同じでも module path が変われば別の program になる。
    let root = unique_temp_dir("rename");
    write_source(&root, "App/EmbeddedCli.ls", "(module App.EmbeddedCli)\n");
    let emitter = SourceFingerprint::from_bytes(b"emitter-v1");
    let before = EmbeddedComponentKey::from_parts(
        &collect_source_entries("selfhost/src", &root).unwrap(),
        &emitter,
    );

    std::fs::remove_file(root.join("App/EmbeddedCli.ls")).unwrap();
    write_source(&root, "App/OtherCli.ls", "(module App.EmbeddedCli)\n");
    let after = EmbeddedComponentKey::from_parts(
        &collect_source_entries("selfhost/src", &root).unwrap(),
        &emitter,
    );

    assert_ne!(before, after, "file 名が変わったら key も変わるべき");

    std::fs::remove_dir_all(&root).unwrap();
}

#[test]
fn test_embedded_component_key_is_independent_of_root_location() {
    // worktree ごとに絶対 path が違っても、同じ source なら同じ key になる。
    // ここが絶対 path 依存だと worktree 間で cache が全く共有されない。
    let first = unique_temp_dir("root-a");
    let second = unique_temp_dir("root-b");
    for root in [&first, &second] {
        write_source(root, "App/EmbeddedCli.ls", "(module App.EmbeddedCli)\n");
        write_source(root, "Core/List.ls", "(module Core.List)\n");
    }

    let emitter = SourceFingerprint::from_bytes(b"emitter-v1");
    assert_eq!(
        EmbeddedComponentKey::from_parts(
            &collect_source_entries("selfhost/src", &first).unwrap(),
            &emitter
        ),
        EmbeddedComponentKey::from_parts(
            &collect_source_entries("selfhost/src", &second).unwrap(),
            &emitter
        ),
        "同一 source なら worktree の位置に依らず同じ key になるべき"
    );

    std::fs::remove_dir_all(&first).unwrap();
    std::fs::remove_dir_all(&second).unwrap();
}

#[test]
fn test_collect_source_entries_is_sorted_and_label_relative() {
    let root = unique_temp_dir("entries");
    write_source(&root, "Core/List.ls", "(module Core.List)\n");
    write_source(&root, "App/EmbeddedCli.ls", "(module App.EmbeddedCli)\n");

    let names = collect_source_entries("selfhost/src", &root)
        .unwrap()
        .into_iter()
        .map(|(name, _)| name)
        .collect::<Vec<_>>();

    assert_eq!(
        names,
        vec![
            "selfhost/src/App/EmbeddedCli.ls".to_string(),
            "selfhost/src/Core/List.ls".to_string(),
        ],
        "entry は label 相対かつ byte 順で安定していること"
    );

    std::fs::remove_dir_all(&root).unwrap();
}

// ---------------------------------------------------------------------------
// envelope
// ---------------------------------------------------------------------------

#[test]
fn test_embedded_component_cache_stores_and_loads_bytes_for_matching_key() {
    let dir = unique_temp_dir("roundtrip");
    let cache = EmbeddedComponentCache::new(&dir);
    let key = test_key("emitter-v1");

    cache
        .store(&key, b"component-bytes")
        .expect("embedded component cache は bytes を保存できるべき");
    assert_eq!(
        cache
            .load(&key)
            .expect("embedded component cache は bytes を読み込めるべき"),
        Some(b"component-bytes".to_vec())
    );

    std::fs::remove_dir_all(&dir).unwrap();
}

#[test]
fn test_embedded_component_cache_misses_when_key_changes() {
    let dir = unique_temp_dir("key-miss");
    let cache = EmbeddedComponentCache::new(&dir);

    cache.store(&test_key("emitter-v1"), b"component-bytes").unwrap();
    assert_eq!(
        cache
            .load(&test_key("emitter-v2"))
            .expect("別 key の lookup は失敗扱いにしない"),
        None,
        "emitter が変わった component を再利用してはいけない"
    );

    std::fs::remove_dir_all(&dir).unwrap();
}

#[test]
fn test_embedded_component_cache_rejects_corrupt_envelope_and_leaves_no_temp_file() {
    let dir = unique_temp_dir("corrupt");
    let cache = EmbeddedComponentCache::new(&dir);
    let key = test_key("emitter-v1");

    cache.store(&key, b"component-bytes").unwrap();
    let path = cache.path_for(&key);
    let mut corrupt = std::fs::read(&path).unwrap();
    *corrupt
        .last_mut()
        .expect("envelope は payload を含む") ^= 1;
    std::fs::write(&path, corrupt).unwrap();

    assert_eq!(
        cache
            .load(&key)
            .expect("破損 cache は fresh compile へ戻れるべき"),
        None,
        "破損 envelope を component bytes として返してはいけない"
    );
    let entries = std::fs::read_dir(dir.join(EMBEDDED_COMPONENT_CACHE_SCHEMA))
        .expect("cache directory を列挙できる")
        .collect::<std::result::Result<Vec<_>, _>>()
        .unwrap();
    assert_eq!(
        entries.len(),
        1,
        "破損 cache の lookup で一時 file を残さない"
    );

    std::fs::remove_dir_all(&dir).unwrap();
}

#[test]
fn test_embedded_component_cache_misses_when_entry_is_absent() {
    let dir = unique_temp_dir("absent");
    let cache = EmbeddedComponentCache::new(&dir);

    assert_eq!(
        cache
            .load(&test_key("emitter-v1"))
            .expect("未生成 cache は error ではなく miss"),
        None
    );

    let _ = std::fs::remove_dir_all(&dir);
}

// ---------------------------------------------------------------------------
// eviction
// ---------------------------------------------------------------------------

#[test]
fn test_embedded_component_cache_trims_to_the_requested_entry_count() {
    // 1 entry が 1.2MB 前後あるので、source を編集するたび target 配下が膨らむ。
    // store のたびに上限まで刈り込む。
    let dir = unique_temp_dir("trim");
    let cache = EmbeddedComponentCache::new(&dir);

    for index in 0..5 {
        cache
            .store(&test_key(&format!("emitter-v{index}")), b"component-bytes")
            .unwrap();
    }
    cache.trim_to_entries(2).expect("trim は成功するべき");

    assert_eq!(
        std::fs::read_dir(dir.join(EMBEDDED_COMPONENT_CACHE_SCHEMA))
            .unwrap()
            .count(),
        2,
        "entry 数が上限まで刈り込まれるべき"
    );

    std::fs::remove_dir_all(&dir).unwrap();
}

#[test]
fn test_embedded_component_cache_trim_keeps_the_most_recently_stored_entry() {
    // 直前に書いた entry を捨てると、その build が即座に cache miss になり意味が無い。
    let dir = unique_temp_dir("trim-recent");
    let cache = EmbeddedComponentCache::new(&dir);

    for index in 0..4 {
        cache
            .store(&test_key(&format!("emitter-v{index}")), b"component-bytes")
            .unwrap();
    }
    let newest = test_key("emitter-v3");
    cache.trim_to_entries(1).unwrap();

    assert_eq!(
        cache.load(&newest).unwrap(),
        Some(b"component-bytes".to_vec()),
        "最後に store した entry は残るべき"
    );

    std::fs::remove_dir_all(&dir).unwrap();
}

#[test]
fn test_embedded_component_cache_trim_is_noop_when_under_the_limit() {
    let dir = unique_temp_dir("trim-noop");
    let cache = EmbeddedComponentCache::new(&dir);
    let key = test_key("emitter-v1");
    cache.store(&key, b"component-bytes").unwrap();

    cache.trim_to_entries(8).unwrap();

    assert_eq!(cache.load(&key).unwrap(), Some(b"component-bytes".to_vec()));

    std::fs::remove_dir_all(&dir).unwrap();
}

#[test]
fn test_embedded_component_cache_trim_ignores_a_missing_directory() {
    // 一度も store していない状態で trim しても error にしない。
    let dir = unique_temp_dir("trim-absent");
    EmbeddedComponentCache::new(&dir)
        .trim_to_entries(4)
        .expect("未生成 cache の trim は成功扱い");

    let _ = std::fs::remove_dir_all(&dir);
}

// ---------------------------------------------------------------------------
// cache root の逆算
// ---------------------------------------------------------------------------

#[test]
fn test_cache_root_from_out_dir_resolves_the_target_directory() {
    assert_eq!(
        cache_root_from_out_dir(std::path::Path::new(
            "/repo/target/debug/build/lsharp-driver-1a2b3c/out"
        )),
        Some(std::path::PathBuf::from("/repo/target/lsharp-embed-cache")),
        "OUT_DIR から target dir を逆算して cache 置き場を決めるべき"
    );
}

#[test]
fn test_cache_root_from_out_dir_honors_a_custom_target_dir_name() {
    // CARGO_TARGET_DIR で別名にしていても `build` 祖先からの相対で成立する。
    assert_eq!(
        cache_root_from_out_dir(std::path::Path::new(
            "/tmp/scratch-target/release/build/lsharp-driver-9f/out"
        )),
        Some(std::path::PathBuf::from(
            "/tmp/scratch-target/lsharp-embed-cache"
        ))
    );
}

#[test]
fn test_cache_root_from_out_dir_gives_up_without_a_build_ancestor() {
    // 想定外の layout で推測した場所に書き込むより、cache を諦めるほうが安全。
    assert_eq!(
        cache_root_from_out_dir(std::path::Path::new("/somewhere/else/out")),
        None
    );
}

// ---------------------------------------------------------------------------
// helpers
// ---------------------------------------------------------------------------

fn test_key(emitter: &str) -> EmbeddedComponentKey {
    EmbeddedComponentKey::from_parts(
        &[(
            "selfhost/src/App/EmbeddedCli.ls".to_string(),
            SourceFingerprint::from_bytes(b"(module App.EmbeddedCli)\n"),
        )],
        &SourceFingerprint::from_bytes(emitter.as_bytes()),
    )
}

fn write_source(root: &std::path::Path, relative: &str, contents: &str) {
    let path = root.join(relative);
    std::fs::create_dir_all(path.parent().expect("source は parent を持つ")).unwrap();
    std::fs::write(path, contents).unwrap();
}

fn unique_temp_dir(name: &str) -> std::path::PathBuf {
    std::env::temp_dir().join(format!(
        "lsharp_wasm_embedded_component_cache_{name}_{}_{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .expect("system clock は unix epoch より後であるべき")
            .as_nanos()
    ))
}
