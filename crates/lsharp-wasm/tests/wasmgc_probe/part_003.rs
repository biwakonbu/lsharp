#[test]
fn wasm_gc_component_cli_fs_runner_subscribes_and_polls_input_stream() {
    let core = emit_component_cli_pollable_probe_module();
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
    .expect("pollable probe を componentize できる");

    let nonce = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("system clock は unix epoch より後であるべき")
        .as_nanos();
    let dir = std::env::temp_dir().join(format!("lsharp_wasmgc_pollable_{nonce}"));
    let extra_dir = std::env::temp_dir().join(format!("lsharp_wasmgc_pollable_extra_{nonce}"));
    std::fs::create_dir_all(&dir).expect("pollable fixture directory を作成できる");
    std::fs::create_dir_all(&extra_dir).expect("second pollable fixture directory を作成できる");
    std::fs::write(dir.join("input.txt"), b"hello").expect("pollable fixture file を作成できる");

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
    .expect("input stream pollable を実行できる");

    assert_eq!(output.stdout, "R");
    assert_eq!(output.exit_code, 0);
    assert_eq!(
        std::fs::read(dir.join("input.txt")).expect("pollable fixture を読める"),
        b"hello"
    );
    std::fs::remove_dir_all(&dir).expect("pollable fixture directory を削除できる");
    std::fs::remove_dir_all(&extra_dir).expect("second pollable fixture directory を削除できる");
}

#[test]
fn wasm_gc_component_cli_fs_runner_polls_empty_input_stream_as_ready() {
    let core = emit_component_cli_pollable_probe_module();
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
    .expect("empty pollable probe を componentize できる");

    let nonce = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("system clock は unix epoch より後であるべき")
        .as_nanos();
    let dir = std::env::temp_dir().join(format!("lsharp_wasmgc_pollable_empty_{nonce}"));
    let extra_dir =
        std::env::temp_dir().join(format!("lsharp_wasmgc_pollable_empty_extra_{nonce}"));
    std::fs::create_dir_all(&dir).expect("empty pollable fixture directory を作成できる");
    std::fs::create_dir_all(&extra_dir)
        .expect("second empty pollable fixture directory を作成できる");
    std::fs::write(dir.join("input.txt"), b"").expect("empty pollable fixture file を作成できる");

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
    .expect("empty input stream pollable を実行できる");

    assert_eq!(output.stdout, "R");
    assert_eq!(output.exit_code, 0);
    std::fs::remove_dir_all(&dir).expect("empty pollable fixture directory を削除できる");
    std::fs::remove_dir_all(&extra_dir)
        .expect("second empty pollable fixture directory を削除できる");
}

#[test]
fn wasm_gc_component_cli_fs_runner_syncs_descriptor_data_and_drops_resources() {
    let core = emit_component_cli_sync_data_probe_module();
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
    .expect("sync-data probe を componentize できる");

    let nonce = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("system clock は unix epoch より後であるべき")
        .as_nanos();
    let dir = std::env::temp_dir().join(format!("lsharp_wasmgc_sync_data_{nonce}"));
    let extra_dir = std::env::temp_dir().join(format!("lsharp_wasmgc_sync_data_extra_{nonce}"));
    std::fs::create_dir_all(&dir).expect("sync-data fixture directory を作成できる");
    std::fs::create_dir_all(&extra_dir).expect("second sync-data fixture directory を作成できる");
    std::fs::write(dir.join("input.txt"), b"hello").expect("sync-data fixture file を作成できる");

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
    .expect("descriptor sync-data を実行できる");

    assert_eq!(output.stdout, "");
    assert_eq!(output.exit_code, 0);
    assert_eq!(
        std::fs::read(dir.join("input.txt")).expect("sync-data fixture を読める"),
        b"hello"
    );
    std::fs::remove_dir_all(&dir).expect("sync-data fixture directory を削除できる");
    std::fs::remove_dir_all(&extra_dir).expect("second sync-data fixture directory を削除できる");
}

#[test]
fn wasm_gc_component_cli_fs_runner_syncs_descriptor_and_drops_resources() {
    let core = emit_component_cli_sync_probe_module();
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
    .expect("sync probe を componentize できる");

    let nonce = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("system clock は unix epoch より後であるべき")
        .as_nanos();
    let dir = std::env::temp_dir().join(format!("lsharp_wasmgc_sync_{nonce}"));
    let extra_dir = std::env::temp_dir().join(format!("lsharp_wasmgc_sync_extra_{nonce}"));
    std::fs::create_dir_all(&dir).expect("sync fixture directory を作成できる");
    std::fs::create_dir_all(&extra_dir).expect("second sync fixture directory を作成できる");
    std::fs::write(dir.join("input.txt"), b"hello").expect("sync fixture file を作成できる");

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
    .expect("descriptor sync を実行できる");

    assert_eq!(output.stdout, "");
    assert_eq!(output.exit_code, 0);
    assert_eq!(
        std::fs::read(dir.join("input.txt")).expect("sync fixture を読める"),
        b"hello"
    );
    std::fs::remove_dir_all(&dir).expect("sync fixture directory を削除できる");
    std::fs::remove_dir_all(&extra_dir).expect("second sync fixture directory を削除できる");
}

#[test]
fn wasm_gc_component_cli_fs_runner_sets_descriptor_size_and_drops_resources() {
    let core = emit_component_cli_set_size_probe_module();
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
    .expect("set-size probe を componentize できる");

    let nonce = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("system clock は unix epoch より後であるべき")
        .as_nanos();
    let dir = std::env::temp_dir().join(format!("lsharp_wasmgc_set_size_{nonce}"));
    let extra_dir = std::env::temp_dir().join(format!("lsharp_wasmgc_set_size_extra_{nonce}"));
    std::fs::create_dir_all(&dir).expect("set-size fixture directory を作成できる");
    std::fs::create_dir_all(&extra_dir).expect("second set-size fixture directory を作成できる");
    std::fs::write(dir.join("input.txt"), b"hello").expect("set-size fixture file を作成できる");

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
    .expect("descriptor set-size を実行できる");

    assert_eq!(output.stdout, "");
    assert_eq!(output.exit_code, 0);
    assert_eq!(
        std::fs::read(dir.join("input.txt")).expect("set-size fixture を読める"),
        b"hello\0\0"
    );
    std::fs::remove_dir_all(&dir).expect("set-size fixture directory を削除できる");
    std::fs::remove_dir_all(&extra_dir).expect("second set-size fixture directory を削除できる");
}

#[test]
fn wasm_gc_component_cli_fs_runner_sets_descriptor_times_without_changing_no_change_values() {
    let core = emit_component_cli_set_times_probe_module();
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
    .expect("set-times probe を componentize できる");

    let nonce = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("system clock は unix epoch より後であるべき")
        .as_nanos();
    let dir = std::env::temp_dir().join(format!("lsharp_wasmgc_set_times_{nonce}"));
    let extra_dir = std::env::temp_dir().join(format!("lsharp_wasmgc_set_times_extra_{nonce}"));
    std::fs::create_dir_all(&dir).expect("set-times fixture directory を作成できる");
    std::fs::create_dir_all(&extra_dir).expect("second set-times fixture directory を作成できる");
    std::fs::write(dir.join("input.txt"), b"hello").expect("set-times fixture file を作成できる");

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
    .expect("descriptor set-times を実行できる");

    assert_eq!(output.stdout, "");
    assert_eq!(output.exit_code, 0);
    assert_eq!(
        std::fs::read(dir.join("input.txt")).expect("set-times fixture を読める"),
        b"hello"
    );
    std::fs::remove_dir_all(&dir).expect("set-times fixture directory を削除できる");
    std::fs::remove_dir_all(&extra_dir).expect("second set-times fixture directory を削除できる");
}

#[test]
fn wasm_gc_component_cli_fs_runner_advises_descriptor_and_drops_resources() {
    let core = emit_component_cli_advise_probe_module();
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
    .expect("advise probe を componentize できる");

    let nonce = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("system clock は unix epoch より後であるべき")
        .as_nanos();
    let dir = std::env::temp_dir().join(format!("lsharp_wasmgc_advise_{nonce}"));
    let extra_dir = std::env::temp_dir().join(format!("lsharp_wasmgc_advise_extra_{nonce}"));
    std::fs::create_dir_all(&dir).expect("advise fixture directory を作成できる");
    std::fs::create_dir_all(&extra_dir).expect("second advise fixture directory を作成できる");
    std::fs::write(dir.join("input.txt"), b"hello").expect("advise fixture file を作成できる");

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
    .expect("descriptor advise を実行できる");

    assert_eq!(output.stdout, "");
    assert_eq!(output.exit_code, 0);
    assert_eq!(
        std::fs::read(dir.join("input.txt")).expect("advise fixture を読める"),
        b"hello"
    );
    std::fs::remove_dir_all(&dir).expect("advise fixture directory を削除できる");
    std::fs::remove_dir_all(&extra_dir).expect("second advise fixture directory を削除できる");
}

#[test]
fn wasm_gc_component_cli_fs_runner_creates_directory_and_drops_resources() {
    let core = emit_component_cli_create_directory_probe_module();
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
    .expect("create-directory-at probe を componentize できる");

    let nonce = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("system clock は unix epoch より後であるべき")
        .as_nanos();
    let dir = std::env::temp_dir().join(format!("lsharp_wasmgc_create_directory_{nonce}"));
    let extra_dir =
        std::env::temp_dir().join(format!("lsharp_wasmgc_create_directory_extra_{nonce}"));
    std::fs::create_dir_all(&dir).expect("create-directory-at fixture directory を作成できる");
    std::fs::create_dir_all(&extra_dir)
        .expect("second create-directory-at fixture directory を作成できる");

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
    .expect("descriptor create-directory-at を実行できる");

    assert_eq!(output.stdout, "");
    assert_eq!(output.exit_code, 0);
    assert!(dir.join("created").is_dir());
    std::fs::remove_dir_all(&dir).expect("create-directory-at fixture directory を削除できる");
    std::fs::remove_dir_all(&extra_dir)
        .expect("second create-directory-at fixture directory を削除できる");
}

#[test]
fn wasm_gc_component_cli_fs_runner_removes_directory_and_drops_resources() {
    let core = emit_component_cli_remove_directory_probe_module();
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
    .expect("remove-directory-at probe を componentize できる");

    let nonce = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("system clock は unix epoch より後であるべき")
        .as_nanos();
    let dir = std::env::temp_dir().join(format!("lsharp_wasmgc_remove_directory_{nonce}"));
    let extra_dir =
        std::env::temp_dir().join(format!("lsharp_wasmgc_remove_directory_extra_{nonce}"));
    std::fs::create_dir_all(dir.join("to-remove"))
        .expect("remove-directory-at fixture directory を作成できる");
    std::fs::create_dir_all(&extra_dir)
        .expect("second remove-directory-at fixture directory を作成できる");

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
    .expect("descriptor remove-directory-at を実行できる");

    assert_eq!(output.stdout, "");
    assert_eq!(output.exit_code, 0);
    assert!(!dir.join("to-remove").exists());
    std::fs::remove_dir_all(&dir).expect("remove-directory-at fixture directory を削除できる");
    std::fs::remove_dir_all(&extra_dir)
        .expect("second remove-directory-at fixture directory を削除できる");
}

#[test]
fn wasm_gc_component_cli_fs_runner_unlinks_file_and_drops_resources() {
    let core = emit_component_cli_unlink_file_probe_module();
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
    .expect("unlink-file-at probe を componentize できる");

    let nonce = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("system clock は unix epoch より後であるべき")
        .as_nanos();
    let dir = std::env::temp_dir().join(format!("lsharp_wasmgc_unlink_file_{nonce}"));
    let extra_dir = std::env::temp_dir().join(format!("lsharp_wasmgc_unlink_file_extra_{nonce}"));
    std::fs::create_dir_all(&dir).expect("unlink-file-at fixture directory を作成できる");
    std::fs::create_dir_all(&extra_dir)
        .expect("second unlink-file-at fixture directory を作成できる");
    std::fs::write(dir.join("to-unlink.txt"), b"hello")
        .expect("unlink-file-at fixture file を作成できる");

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
    .expect("descriptor unlink-file-at を実行できる");

    assert_eq!(output.stdout, "");
    assert_eq!(output.exit_code, 0);
    assert!(!dir.join("to-unlink.txt").exists());
    std::fs::remove_dir_all(&dir).expect("unlink-file-at fixture directory を削除できる");
    std::fs::remove_dir_all(&extra_dir)
        .expect("second unlink-file-at fixture directory を削除できる");
}

#[test]
fn wasm_gc_component_cli_fs_runner_renames_file_and_drops_resources() {
    let core = emit_component_cli_rename_file_probe_module();
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
    .expect("rename-at probe を componentize できる");

    let nonce = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("system clock は unix epoch より後であるべき")
        .as_nanos();
    let dir = std::env::temp_dir().join(format!("lsharp_wasmgc_rename_file_{nonce}"));
    let extra_dir = std::env::temp_dir().join(format!("lsharp_wasmgc_rename_file_extra_{nonce}"));
    std::fs::create_dir_all(&dir).expect("rename-at fixture directory を作成できる");
    std::fs::create_dir_all(&extra_dir).expect("second rename-at fixture directory を作成できる");
    std::fs::write(dir.join("old.txt"), b"hello").expect("rename-at fixture file を作成できる");

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
    .expect("descriptor rename-at を実行できる");

    assert_eq!(output.stdout, "");
    assert_eq!(output.exit_code, 0);
    assert!(!dir.join("old.txt").exists());
    assert_eq!(
        std::fs::read(dir.join("renamed.txt")).expect("rename-at 後の file を読める"),
        b"hello"
    );
    std::fs::remove_dir_all(&dir).expect("rename-at fixture directory を削除できる");
    std::fs::remove_dir_all(&extra_dir).expect("second rename-at fixture directory を削除できる");
}

#[test]
fn wasm_gc_component_cli_fs_runner_creates_symlink_and_drops_resources() {
    let core = emit_component_cli_symlink_file_probe_module();
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
    .expect("symlink-at probe を componentize できる");

    let nonce = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("system clock は unix epoch より後であるべき")
        .as_nanos();
    let dir = std::env::temp_dir().join(format!("lsharp_wasmgc_symlink_file_{nonce}"));
    let extra_dir = std::env::temp_dir().join(format!("lsharp_wasmgc_symlink_file_extra_{nonce}"));
    std::fs::create_dir_all(&dir).expect("symlink-at fixture directory を作成できる");
    std::fs::create_dir_all(&extra_dir).expect("second symlink-at fixture directory を作成できる");
    std::fs::write(dir.join("target.txt"), b"hello").expect("symlink-at target file を作成できる");

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
    .expect("descriptor symlink-at を実行できる");

    assert_eq!(output.stdout, "");
    assert_eq!(output.exit_code, 0);
    assert_eq!(
        std::fs::read_link(dir.join("link.txt")).expect("symlink-at 後の link を読める"),
        PathBuf::from("target.txt")
    );
    assert_eq!(
        std::fs::read(dir.join("link.txt")).expect("symlink-at 経由で target を読める"),
        b"hello"
    );
    std::fs::remove_dir_all(&dir).expect("symlink-at fixture directory を削除できる");
    std::fs::remove_dir_all(&extra_dir).expect("second symlink-at fixture directory を削除できる");
}

#[test]
fn wasm_gc_component_cli_fs_runner_reads_symlink_target_and_drops_resources() {
    let core = emit_component_cli_readlink_file_probe_module();
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
    .expect("readlink-at probe を componentize できる");

    let nonce = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("system clock は unix epoch より後であるべき")
        .as_nanos();
    let dir = std::env::temp_dir().join(format!("lsharp_wasmgc_readlink_file_{nonce}"));
    let extra_dir = std::env::temp_dir().join(format!("lsharp_wasmgc_readlink_file_extra_{nonce}"));
    std::fs::create_dir_all(&dir).expect("readlink-at fixture directory を作成できる");
    std::fs::create_dir_all(&extra_dir).expect("second readlink-at fixture directory を作成できる");
    std::fs::write(dir.join("target.txt"), b"hello").expect("readlink-at target file を作成できる");
    std::os::unix::fs::symlink("target.txt", dir.join("link.txt"))
        .expect("readlink-at fixture symlink を作成できる");

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
    .expect("descriptor readlink-at を実行できる");

    assert_eq!(output.stdout, "target.txt");
    assert_eq!(output.exit_code, 0);
    assert_eq!(
        std::fs::read_link(dir.join("link.txt")).expect("readlink-at 後の link を読める"),
        PathBuf::from("target.txt")
    );
    std::fs::remove_dir_all(&dir).expect("readlink-at fixture directory を削除できる");
    std::fs::remove_dir_all(&extra_dir).expect("second readlink-at fixture directory を削除できる");
}
