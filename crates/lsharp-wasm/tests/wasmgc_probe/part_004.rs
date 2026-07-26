#[test]
fn wasm_gc_component_cli_fs_runner_creates_hard_link_and_drops_resources() {
    let core = emit_component_cli_link_file_probe_module();
    let wit_file = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("..")
        .join("wit")
        .join("lsharp-wasmgc-output.wit");
    let component = lsharp_wasm::component_adapter::componentize_core_module(
        &core,
        &wit_file,
        "wasmgc-cli-fs",
        &[],
    )
    .expect("link-at probe を componentize できる");

    let nonce = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("system clock は unix epoch より後であるべき")
        .as_nanos();
    let dir = std::env::temp_dir().join(format!("lsharp_wasmgc_link_file_{nonce}"));
    let extra_dir = std::env::temp_dir().join(format!("lsharp_wasmgc_link_file_extra_{nonce}"));
    std::fs::create_dir_all(&dir).expect("link-at fixture directory を作成できる");
    std::fs::create_dir_all(&extra_dir).expect("second link-at fixture directory を作成できる");
    std::fs::write(dir.join("source.txt"), b"hello").expect("link-at source file を作成できる");

    let preopen = lsharp_wasm::wasmgc_runner::Preview2Preopen::new(
        &dir,
        "data",
        lsharp_wasm::wasmgc_runner::Preview2PreopenRights::read_write(),
    );
    let extra_preopen = lsharp_wasm::wasmgc_runner::Preview2Preopen::new(
        &extra_dir,
        "extra",
        lsharp_wasm::wasmgc_runner::Preview2PreopenRights::read_write(),
    );
    let output = lsharp_wasm::wasmgc_runner::run_wasm_wasmgc_component_cli_with_preview2_stdout_and_preopens(
        &component,
        &[],
        "",
        &[preopen, extra_preopen],
    )
    .expect("descriptor link-at を実行できる");

    assert_eq!(output.stdout, "");
    assert_eq!(output.exit_code, 0);
    assert_eq!(
        std::fs::read(dir.join("source.txt")).expect("link-at source file を読める"),
        b"hello"
    );
    assert_eq!(
        std::fs::read(dir.join("hardlink.txt")).expect("link-at 後の hard link を読める"),
        b"hello"
    );
    std::fs::remove_dir_all(&dir).expect("link-at fixture directory を削除できる");
    std::fs::remove_dir_all(&extra_dir).expect("second link-at fixture directory を削除できる");
}

#[test]
fn wasm_gc_component_cli_fs_runner_compares_same_file_descriptors_and_drops_resources() {
    let core = emit_component_cli_same_object_probe_module();
    let wit_file = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("..")
        .join("wit")
        .join("lsharp-wasmgc-output.wit");
    let component = lsharp_wasm::component_adapter::componentize_core_module(
        &core,
        &wit_file,
        "wasmgc-cli-fs",
        &[],
    )
    .expect("is-same-object probe を componentize できる");

    let nonce = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("system clock は unix epoch より後であるべき")
        .as_nanos();
    let dir = std::env::temp_dir().join(format!("lsharp_wasmgc_same_object_{nonce}"));
    let extra_dir = std::env::temp_dir().join(format!("lsharp_wasmgc_same_object_extra_{nonce}"));
    std::fs::create_dir_all(&dir).expect("is-same-object fixture directory を作成できる");
    std::fs::create_dir_all(&extra_dir)
        .expect("second is-same-object fixture directory を作成できる");
    std::fs::write(dir.join("source.txt"), b"hello")
        .expect("is-same-object source file を作成できる");
    std::fs::hard_link(dir.join("source.txt"), dir.join("hardlink.txt"))
        .expect("is-same-object hard link fixture を作成できる");

    let preopen = lsharp_wasm::wasmgc_runner::Preview2Preopen::new(
        &dir,
        "data",
        lsharp_wasm::wasmgc_runner::Preview2PreopenRights::read_only(),
    );
    let extra_preopen = lsharp_wasm::wasmgc_runner::Preview2Preopen::new(
        &extra_dir,
        "extra",
        lsharp_wasm::wasmgc_runner::Preview2PreopenRights::read_only(),
    );
    let output = lsharp_wasm::wasmgc_runner::run_wasm_wasmgc_component_cli_with_preview2_stdout_and_preopens(
        &component,
        &[],
        "",
        &[preopen, extra_preopen],
    )
    .expect("descriptor is-same-object を実行できる");

    assert_eq!(output.stdout, "");
    assert_eq!(output.exit_code, 0);
    assert_eq!(
        std::fs::read(dir.join("source.txt")).expect("is-same-object source file を読める"),
        b"hello"
    );
    assert_eq!(
        std::fs::read(dir.join("hardlink.txt")).expect("is-same-object hard link を読める"),
        b"hello"
    );
    std::fs::remove_dir_all(&dir).expect("is-same-object fixture directory を削除できる");
    std::fs::remove_dir_all(&extra_dir)
        .expect("second is-same-object fixture directory を削除できる");
}

#[test]
fn wasm_gc_component_cli_fs_runner_reads_stable_descriptor_metadata_hash_and_drops_resources() {
    let core = emit_component_cli_metadata_hash_probe_module();
    let wit_file = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("..")
        .join("wit")
        .join("lsharp-wasmgc-output.wit");
    let component = lsharp_wasm::component_adapter::componentize_core_module(
        &core,
        &wit_file,
        "wasmgc-cli-fs",
        &[],
    )
    .expect("metadata-hash probe を componentize できる");

    let nonce = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("system clock は unix epoch より後であるべき")
        .as_nanos();
    let dir = std::env::temp_dir().join(format!("lsharp_wasmgc_metadata_hash_{nonce}"));
    let extra_dir = std::env::temp_dir().join(format!("lsharp_wasmgc_metadata_hash_extra_{nonce}"));
    std::fs::create_dir_all(&dir).expect("metadata-hash fixture directory を作成できる");
    std::fs::create_dir_all(&extra_dir)
        .expect("second metadata-hash fixture directory を作成できる");
    std::fs::write(dir.join("source.txt"), b"hello")
        .expect("metadata-hash source file を作成できる");

    let preopen = lsharp_wasm::wasmgc_runner::Preview2Preopen::new(
        &dir,
        "data",
        lsharp_wasm::wasmgc_runner::Preview2PreopenRights::read_only(),
    );
    let extra_preopen = lsharp_wasm::wasmgc_runner::Preview2Preopen::new(
        &extra_dir,
        "extra",
        lsharp_wasm::wasmgc_runner::Preview2PreopenRights::read_only(),
    );
    let output = lsharp_wasm::wasmgc_runner::run_wasm_wasmgc_component_cli_with_preview2_stdout_and_preopens(
        &component,
        &[],
        "",
        &[preopen, extra_preopen],
    )
    .expect("descriptor metadata-hash を実行できる");

    assert_eq!(output.stdout, "");
    assert_eq!(output.exit_code, 0);
    assert_eq!(
        std::fs::read(dir.join("source.txt")).expect("metadata-hash source file を読める"),
        b"hello"
    );
    std::fs::remove_dir_all(&dir).expect("metadata-hash fixture directory を削除できる");
    std::fs::remove_dir_all(&extra_dir)
        .expect("second metadata-hash fixture directory を削除できる");
}

#[test]
fn wasm_gc_component_cli_fs_runner_reads_stable_metadata_hash_at_and_drops_resources() {
    let core = emit_component_cli_metadata_hash_at_probe_module();
    let wit_file = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("..")
        .join("wit")
        .join("lsharp-wasmgc-output.wit");
    let component = lsharp_wasm::component_adapter::componentize_core_module(
        &core,
        &wit_file,
        "wasmgc-cli-fs",
        &[],
    )
    .expect("metadata-hash-at probe を componentize できる");

    let nonce = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("system clock は unix epoch より後であるべき")
        .as_nanos();
    let dir = std::env::temp_dir().join(format!("lsharp_wasmgc_metadata_hash_at_{nonce}"));
    let extra_dir =
        std::env::temp_dir().join(format!("lsharp_wasmgc_metadata_hash_at_extra_{nonce}"));
    std::fs::create_dir_all(&dir).expect("metadata-hash-at fixture directory を作成できる");
    std::fs::create_dir_all(&extra_dir)
        .expect("second metadata-hash-at fixture directory を作成できる");
    std::fs::write(dir.join("source.txt"), b"hello")
        .expect("metadata-hash-at source file を作成できる");

    let preopen = lsharp_wasm::wasmgc_runner::Preview2Preopen::new(
        &dir,
        "data",
        lsharp_wasm::wasmgc_runner::Preview2PreopenRights::read_only(),
    );
    let extra_preopen = lsharp_wasm::wasmgc_runner::Preview2Preopen::new(
        &extra_dir,
        "extra",
        lsharp_wasm::wasmgc_runner::Preview2PreopenRights::read_only(),
    );
    let output = lsharp_wasm::wasmgc_runner::run_wasm_wasmgc_component_cli_with_preview2_stdout_and_preopens(
        &component,
        &[],
        "",
        &[preopen, extra_preopen],
    )
    .expect("descriptor metadata-hash-at を実行できる");

    assert_eq!(output.stdout, "");
    assert_eq!(output.exit_code, 0);
    assert_eq!(
        std::fs::read(dir.join("source.txt")).expect("metadata-hash-at source file を読める"),
        b"hello"
    );
    std::fs::remove_dir_all(&dir).expect("metadata-hash-at fixture directory を削除できる");
    std::fs::remove_dir_all(&extra_dir)
        .expect("second metadata-hash-at fixture directory を削除できる");
}

#[test]
fn wasm_gc_component_cli_fs_runner_stats_file_at_and_drops_resources() {
    let core = emit_component_cli_stat_at_probe_module();
    let wit_file = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("..")
        .join("wit")
        .join("lsharp-wasmgc-output.wit");
    let component = lsharp_wasm::component_adapter::componentize_core_module(
        &core,
        &wit_file,
        "wasmgc-cli-fs",
        &[],
    )
    .expect("stat-at probe を componentize できる");

    let nonce = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("system clock は unix epoch より後であるべき")
        .as_nanos();
    let dir = std::env::temp_dir().join(format!("lsharp_wasmgc_stat_at_{nonce}"));
    let extra_dir = std::env::temp_dir().join(format!("lsharp_wasmgc_stat_at_extra_{nonce}"));
    std::fs::create_dir_all(&dir).expect("stat-at fixture directory を作成できる");
    std::fs::create_dir_all(&extra_dir).expect("second stat-at fixture directory を作成できる");
    std::fs::write(dir.join("source.txt"), b"hello").expect("stat-at source file を作成できる");

    let preopen = lsharp_wasm::wasmgc_runner::Preview2Preopen::new(
        &dir,
        "data",
        lsharp_wasm::wasmgc_runner::Preview2PreopenRights::read_only(),
    );
    let extra_preopen = lsharp_wasm::wasmgc_runner::Preview2Preopen::new(
        &extra_dir,
        "extra",
        lsharp_wasm::wasmgc_runner::Preview2PreopenRights::read_only(),
    );
    let output = lsharp_wasm::wasmgc_runner::run_wasm_wasmgc_component_cli_with_preview2_stdout_and_preopens(
        &component,
        &[],
        "",
        &[preopen, extra_preopen],
    )
    .expect("descriptor stat-at を実行できる");

    assert_eq!(output.stdout, "");
    assert_eq!(output.exit_code, 0);
    assert_eq!(
        std::fs::read(dir.join("source.txt")).expect("stat-at source file を読める"),
        b"hello"
    );
    std::fs::remove_dir_all(&dir).expect("stat-at fixture directory を削除できる");
    std::fs::remove_dir_all(&extra_dir).expect("second stat-at fixture directory を削除できる");
}

#[test]
fn wasm_gc_component_cli_fs_runner_sets_file_times_at_without_changing_no_change_values() {
    let core = emit_component_cli_set_times_at_probe_module();
    let wit_file = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("..")
        .join("wit")
        .join("lsharp-wasmgc-output.wit");
    let component = lsharp_wasm::component_adapter::componentize_core_module(
        &core,
        &wit_file,
        "wasmgc-cli-fs",
        &[],
    )
    .expect("set-times-at probe を componentize できる");

    let nonce = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("system clock は unix epoch より後であるべき")
        .as_nanos();
    let dir = std::env::temp_dir().join(format!("lsharp_wasmgc_set_times_at_{nonce}"));
    let extra_dir = std::env::temp_dir().join(format!("lsharp_wasmgc_set_times_at_extra_{nonce}"));
    std::fs::create_dir_all(&dir).expect("set-times-at fixture directory を作成できる");
    std::fs::create_dir_all(&extra_dir)
        .expect("second set-times-at fixture directory を作成できる");
    std::fs::write(dir.join("source.txt"), b"hello")
        .expect("set-times-at source file を作成できる");

    let preopen = lsharp_wasm::wasmgc_runner::Preview2Preopen::new(
        &dir,
        "data",
        lsharp_wasm::wasmgc_runner::Preview2PreopenRights::read_write(),
    );
    let extra_preopen = lsharp_wasm::wasmgc_runner::Preview2Preopen::new(
        &extra_dir,
        "extra",
        lsharp_wasm::wasmgc_runner::Preview2PreopenRights::read_write(),
    );
    let output = lsharp_wasm::wasmgc_runner::run_wasm_wasmgc_component_cli_with_preview2_stdout_and_preopens(
        &component,
        &[],
        "",
        &[preopen, extra_preopen],
    )
    .expect("descriptor set-times-at を実行できる");

    assert_eq!(output.stdout, "");
    assert_eq!(output.exit_code, 0);
    assert_eq!(
        std::fs::read(dir.join("source.txt")).expect("set-times-at source file を読める"),
        b"hello"
    );
    std::fs::remove_dir_all(&dir).expect("set-times-at fixture directory を削除できる");
    std::fs::remove_dir_all(&extra_dir)
        .expect("second set-times-at fixture directory を削除できる");
}

#[test]
fn wasm_gc_component_cli_fs_runner_polls_subscribed_input_stream_list() {
    let core = emit_component_cli_poll_list_probe_module();
    let wit_file = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("..")
        .join("wit")
        .join("lsharp-wasmgc-output.wit");
    let component = lsharp_wasm::component_adapter::componentize_core_module(
        &core,
        &wit_file,
        "wasmgc-cli-fs-streams",
        &[],
    )
    .expect("poll list probe を componentize できる");

    let nonce = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("system clock は unix epoch より後であるべき")
        .as_nanos();
    let dir = std::env::temp_dir().join(format!("lsharp_wasmgc_poll_list_{nonce}"));
    let extra_dir = std::env::temp_dir().join(format!("lsharp_wasmgc_poll_list_extra_{nonce}"));
    std::fs::create_dir_all(&dir).expect("poll list fixture directory を作成できる");
    std::fs::create_dir_all(&extra_dir).expect("second poll list fixture directory を作成できる");
    std::fs::write(dir.join("input.txt"), b"hello").expect("poll list fixture file を作成できる");

    let preopen = lsharp_wasm::wasmgc_runner::Preview2Preopen::new(
        &dir,
        "data",
        lsharp_wasm::wasmgc_runner::Preview2PreopenRights::read_only(),
    );
    let extra_preopen = lsharp_wasm::wasmgc_runner::Preview2Preopen::new(
        &extra_dir,
        "extra",
        lsharp_wasm::wasmgc_runner::Preview2PreopenRights::read_only(),
    );
    let output = lsharp_wasm::wasmgc_runner::run_wasm_wasmgc_component_cli_with_preview2_stdout_and_preopens(
        &component,
        &[],
        "",
        &[preopen, extra_preopen],
    )
    .expect("poll list を実行できる");

    assert_eq!(output.stdout, "P");
    assert_eq!(output.exit_code, 0);
    assert_eq!(
        std::fs::read(dir.join("input.txt")).expect("poll list fixture を読める"),
        b"hello"
    );
    std::fs::remove_dir_all(&dir).expect("poll list fixture directory を削除できる");
    std::fs::remove_dir_all(&extra_dir).expect("second poll list fixture directory を削除できる");
}

#[test]
fn wasm_gc_component_cli_fs_runner_polls_empty_input_stream_list_as_ready() {
    let core = emit_component_cli_poll_list_probe_module();
    let wit_file = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("..")
        .join("wit")
        .join("lsharp-wasmgc-output.wit");
    let component = lsharp_wasm::component_adapter::componentize_core_module(
        &core,
        &wit_file,
        "wasmgc-cli-fs-streams",
        &[],
    )
    .expect("empty poll list probe を componentize できる");

    let nonce = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("system clock は unix epoch より後であるべき")
        .as_nanos();
    let dir = std::env::temp_dir().join(format!("lsharp_wasmgc_poll_list_empty_{nonce}"));
    let extra_dir =
        std::env::temp_dir().join(format!("lsharp_wasmgc_poll_list_empty_extra_{nonce}"));
    std::fs::create_dir_all(&dir).expect("empty poll list fixture directory を作成できる");
    std::fs::create_dir_all(&extra_dir)
        .expect("second empty poll list fixture directory を作成できる");
    std::fs::write(dir.join("input.txt"), b"").expect("empty poll list fixture file を作成できる");

    let preopen = lsharp_wasm::wasmgc_runner::Preview2Preopen::new(
        &dir,
        "data",
        lsharp_wasm::wasmgc_runner::Preview2PreopenRights::read_only(),
    );
    let extra_preopen = lsharp_wasm::wasmgc_runner::Preview2Preopen::new(
        &extra_dir,
        "extra",
        lsharp_wasm::wasmgc_runner::Preview2PreopenRights::read_only(),
    );
    let output = lsharp_wasm::wasmgc_runner::run_wasm_wasmgc_component_cli_with_preview2_stdout_and_preopens(
        &component,
        &[],
        "",
        &[preopen, extra_preopen],
    )
    .expect("empty poll list を実行できる");

    assert_eq!(output.stdout, "P");
    assert_eq!(output.exit_code, 0);
    std::fs::remove_dir_all(&dir).expect("empty poll list fixture directory を削除できる");
    std::fs::remove_dir_all(&extra_dir)
        .expect("second empty poll list fixture directory を削除できる");
}

#[test]
fn wasm_gc_component_cli_fs_runner_polls_multiple_input_stream_pollables_as_ready() {
    let core = emit_component_cli_poll_list_probe_module_with_list_len(2);
    let wit_file = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("..")
        .join("wit")
        .join("lsharp-wasmgc-output.wit");
    let component = lsharp_wasm::component_adapter::componentize_core_module(
        &core,
        &wit_file,
        "wasmgc-cli-fs-streams",
        &[],
    )
    .expect("multiple poll list probe を componentize できる");

    let nonce = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("system clock は unix epoch より後であるべき")
        .as_nanos();
    let dir = std::env::temp_dir().join(format!("lsharp_wasmgc_poll_list_multiple_{nonce}"));
    let extra_dir =
        std::env::temp_dir().join(format!("lsharp_wasmgc_poll_list_multiple_extra_{nonce}"));
    std::fs::create_dir_all(&dir).expect("multiple poll list fixture directory を作成できる");
    std::fs::create_dir_all(&extra_dir)
        .expect("second multiple poll list fixture directory を作成できる");
    std::fs::write(dir.join("input.txt"), b"")
        .expect("multiple poll list fixture file を作成できる");

    let preopen = lsharp_wasm::wasmgc_runner::Preview2Preopen::new(
        &dir,
        "data",
        lsharp_wasm::wasmgc_runner::Preview2PreopenRights::read_only(),
    );
    let extra_preopen = lsharp_wasm::wasmgc_runner::Preview2Preopen::new(
        &extra_dir,
        "extra",
        lsharp_wasm::wasmgc_runner::Preview2PreopenRights::read_only(),
    );
    let output = lsharp_wasm::wasmgc_runner::run_wasm_wasmgc_component_cli_with_preview2_stdout_and_preopens(
        &component,
        &[],
        "",
        &[preopen, extra_preopen],
    )
    .expect("multiple poll list を実行できる");

    assert_eq!(output.stdout, "P");
    assert_eq!(output.exit_code, 0);
    std::fs::remove_dir_all(&dir).expect("multiple poll list fixture directory を削除できる");
    std::fs::remove_dir_all(&extra_dir)
        .expect("second multiple poll list fixture directory を削除できる");
}

#[test]
fn wasm_gc_component_cli_fs_runner_polls_multiple_input_sources_as_ready() {
    let core = emit_component_cli_poll_list_probe_module_from_two_input_streams();
    let wit_file = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("..")
        .join("wit")
        .join("lsharp-wasmgc-output.wit");
    let component = lsharp_wasm::component_adapter::componentize_core_module(
        &core,
        &wit_file,
        "wasmgc-cli-fs-streams",
        &[],
    )
    .expect("multiple input source poll list probe を componentize できる");

    let nonce = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("system clock は unix epoch より後であるべき")
        .as_nanos();
    let dir = std::env::temp_dir().join(format!("lsharp_wasmgc_poll_sources_{nonce}"));
    std::fs::create_dir_all(&dir).expect("multiple input source fixture directory を作成できる");
    std::fs::write(dir.join("source-a.txt"), b"")
        .expect("multiple input source fixture file を作成できる");
    std::fs::write(dir.join("source-b.txt"), b"")
        .expect("second multiple input source fixture file を作成できる");

    let preopen = lsharp_wasm::wasmgc_runner::Preview2Preopen::new(
        &dir,
        "data",
        lsharp_wasm::wasmgc_runner::Preview2PreopenRights::read_only(),
    );
    let output = lsharp_wasm::wasmgc_runner::run_wasm_wasmgc_component_cli_with_preview2_stdout_and_preopens(
        &component,
        &[],
        "",
        &[preopen],
    )
    .expect("multiple input source poll list を実行できる");

    assert_eq!(output.stdout, "P");
    assert_eq!(output.exit_code, 0);
    std::fs::remove_dir_all(&dir).expect("multiple input source fixture directory を削除できる");
}

#[test]
fn wasm_gc_component_cli_fs_runner_traps_on_empty_poll_list() {
    let core = emit_component_cli_poll_list_probe_module_with_list_len(0);
    let wit_file = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("..")
        .join("wit")
        .join("lsharp-wasmgc-output.wit");
    let component = lsharp_wasm::component_adapter::componentize_core_module(
        &core,
        &wit_file,
        "wasmgc-cli-fs-streams",
        &[],
    )
    .expect("empty poll list trap probe を componentize できる");

    let nonce = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("system clock は unix epoch より後であるべき")
        .as_nanos();
    let dir = std::env::temp_dir().join(format!("lsharp_wasmgc_poll_list_trap_{nonce}"));
    let extra_dir =
        std::env::temp_dir().join(format!("lsharp_wasmgc_poll_list_trap_extra_{nonce}"));
    std::fs::create_dir_all(&dir).expect("empty poll list trap fixture directory を作成できる");
    std::fs::create_dir_all(&extra_dir)
        .expect("second empty poll list trap fixture directory を作成できる");
    std::fs::write(dir.join("input.txt"), b"")
        .expect("empty poll list trap fixture file を作成できる");

    let preopen = lsharp_wasm::wasmgc_runner::Preview2Preopen::new(
        &dir,
        "data",
        lsharp_wasm::wasmgc_runner::Preview2PreopenRights::read_only(),
    );
    let extra_preopen = lsharp_wasm::wasmgc_runner::Preview2Preopen::new(
        &extra_dir,
        "extra",
        lsharp_wasm::wasmgc_runner::Preview2PreopenRights::read_only(),
    );
    let error = lsharp_wasm::wasmgc_runner::run_wasm_wasmgc_component_cli_with_preview2_stdout_and_preopens(
        &component,
        &[],
        "",
        &[preopen, extra_preopen],
    )
    .expect_err("empty poll list は trap になるべき");

    assert!(
        error.contains("poll"),
        "empty poll list trap の境界を示すべき: {error}"
    );
    std::fs::remove_dir_all(&dir).expect("empty poll list trap fixture directory を削除できる");
    std::fs::remove_dir_all(&extra_dir)
        .expect("second empty poll list trap fixture directory を削除できる");
}

#[test]
fn wasm_gc_component_output_propagates_sink_failure_as_trap() {
    let module = IrModule {
        functions: vec![Function {
            name: "main".to_string(),
            params: vec![],
            result: IrType::I64,
            locals: vec![],
            body: vec![
                Instruction::I32Const(65),
                Instruction::ArrayNewFixed(0, 1),
                Instruction::Call(4),
                Instruction::I64Const(0),
            ],
            is_export: true,
        }],
        gc_types: vec![GcTypeDef {
            name: "StringBytes".to_string(),
            kind: GcTypeKind::PackedByteArray,
        }],
        imports: vec![],
        globals: vec![],
        string_data: vec![],
    };
    let core = lsharp_wasm::wasmgc::emit_wasm_wasmgc_component_output(&module)
        .expect("component output sink failure module を生成できる");
    let error = lsharp_wasm::wasmgc_runner::run_wasm_wasmgc_component_output_with_stdout_sink(
        &core,
        |_bytes| Err("stdout closed".to_string()),
    )
    .expect_err("component output sink error は trap になる");
    assert!(error.contains("stdout closed"), "{error}");
}

#[test]
fn wasm_gc_component_output_rejects_invalid_linear_memory_range() {
    let core = wat::parse_str(
        r#"
(module
  (type (func (param i32 i32)))
  (type (func (result i64)))
  (import "lsharp:wasmgc-output/stdout@0.1.0" "write" (func $write (type 0)))
  (memory (export "memory") 1)
  (func (export "main") (type 1)
    i32.const 65536
    i32.const 1
    call $write
    i64.const 0)
)
"#,
    )
    .expect("invalid range module を生成できる");
    let error = lsharp_wasm::wasmgc_runner::run_wasm_wasmgc_component_output_with_stdout_sink(
        &core,
        |_bytes| Ok(()),
    )
    .expect_err("linear memory 外の canonical pair は拒否する");
    assert!(error.contains("linear memory 外"), "{error}");
}

#[test]
fn wasm_gc_runner_connects_print_string_to_stdout_sink() {
    let module = IrModule {
        functions: vec![Function {
            name: "main".to_string(),
            params: vec![],
            result: IrType::I64,
            locals: vec![],
            body: vec![
                Instruction::I32Const(195),
                Instruction::I32Const(169),
                Instruction::ArrayNewFixed(0, 2),
                Instruction::Call(4),
                Instruction::I64Const(7),
            ],
            is_export: true,
        }],
        gc_types: vec![GcTypeDef {
            name: "StringBytes".to_string(),
            kind: GcTypeKind::PackedByteArray,
        }],
        imports: vec![],
        globals: vec![],
        string_data: vec![],
    };
    let bytes =
        lsharp_wasm::wasmgc::emit_wasm_wasmgc(&module).expect("runner sink module を生成できる");
    let printed = Arc::new(Mutex::new(Vec::<Vec<u8>>::new()));
    let printed_for_sink = Arc::clone(&printed);
    let exit_code =
        lsharp_wasm::wasmgc_runner::run_wasm_wasmgc_with_stdout_sink(&bytes, move |bytes| {
            printed_for_sink.lock().unwrap().push(bytes.to_vec());
            Ok(())
        })
        .expect("runner が print-string sink を接続できる");

    assert_eq!(exit_code, 7);
    assert_eq!(*printed.lock().unwrap(), vec![vec![195, 169]]);

    let captured = lsharp_wasm::wasmgc_runner::run_wasm_wasmgc_capture(&bytes)
        .expect("runner が stdout と exit code を capture できる");
    assert_eq!(captured.stdout, "é");
    assert_eq!(captured.exit_code, 7);
}

#[test]
fn wasm_gc_runner_propagates_stdout_sink_failure() {
    let module = IrModule {
        functions: vec![Function {
            name: "main".to_string(),
            params: vec![],
            result: IrType::I64,
            locals: vec![],
            body: vec![
                Instruction::I32Const(65),
                Instruction::ArrayNewFixed(0, 1),
                Instruction::Call(4),
                Instruction::I64Const(0),
            ],
            is_export: true,
        }],
        gc_types: vec![GcTypeDef {
            name: "StringBytes".to_string(),
            kind: GcTypeKind::PackedByteArray,
        }],
        imports: vec![],
        globals: vec![],
        string_data: vec![],
    };
    let bytes = lsharp_wasm::wasmgc::emit_wasm_wasmgc(&module)
        .expect("runner sink failure module を生成できる");
    let error = lsharp_wasm::wasmgc_runner::run_wasm_wasmgc_with_stdout_sink(&bytes, |_bytes| {
        Err("stdout closed".to_string())
    })
    .expect_err("sink failure は runner error になる");

    assert!(error.contains("stdout closed"), "{error}");
}
