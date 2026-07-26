#[test]
fn wasm_gc_component_cli_fs_runner_maps_stream_failure_to_filesystem_error_code() {
    let core = emit_component_cli_stream_failure_probe_module();
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
    .expect("stream failure probe を componentize できる");

    let nonce = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("system clock は unix epoch より後であるべき")
        .as_nanos();
    let dir = std::env::temp_dir().join(format!("lsharp_wasmgc_stream_failure_{nonce}"));
    std::fs::create_dir_all(&dir).expect("stream failure fixture directory を作成できる");
    std::fs::write(dir.join("input.txt"), b"seed")
        .expect("stream failure fixture file を作成できる");

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
    .expect("stream failure を実行できる");

    assert_eq!(output.stdout, "E");
    assert_eq!(output.exit_code, 0);
    std::fs::remove_dir_all(&dir).expect("stream failure fixture directory を削除できる");
}

#[test]
fn wasm_gc_component_cli_fs_runner_maps_output_stream_failure_to_filesystem_error_code() {
    let core = emit_component_cli_output_stream_failure_probe_module();
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
    .expect("output stream failure probe を componentize できる");

    let nonce = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("system clock は unix epoch より後であるべき")
        .as_nanos();
    let dir = std::env::temp_dir().join(format!("lsharp_wasmgc_output_stream_failure_{nonce}"));
    std::fs::create_dir_all(&dir).expect("output stream failure fixture directory を作成できる");

    let preopen = lsharp_wasm::wasmgc_runner::Preview2Preopen::new(
        &dir,
        "data",
        lsharp_wasm::wasmgc_runner::Preview2PreopenRights::read_write(),
    );
    let output = lsharp_wasm::wasmgc_runner::run_wasm_wasmgc_component_cli_with_preview2_stdout_and_preopens(
        &component,
        &[],
        "",
        &[preopen],
    )
    .expect("output stream failure を実行できる");

    assert_eq!(output.stdout, "O");
    assert_eq!(output.exit_code, 0);
    std::fs::remove_dir_all(&dir).expect("output stream failure fixture directory を削除できる");
}

#[test]
fn wasm_gc_component_cli_fs_runner_maps_async_output_stream_failure_to_filesystem_error_code() {
    let core = emit_component_cli_async_output_stream_failure_probe_module();
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
    .expect("async output stream failure probe を componentize できる");

    let nonce = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("system clock は unix epoch より後であるべき")
        .as_nanos();
    let dir = std::env::temp_dir().join(format!("lsharp_wasmgc_async_output_failure_{nonce}"));
    std::fs::create_dir_all(&dir)
        .expect("async output stream failure fixture directory を作成できる");

    let preopen = lsharp_wasm::wasmgc_runner::Preview2Preopen::new(
        &dir,
        "data",
        lsharp_wasm::wasmgc_runner::Preview2PreopenRights::read_write(),
    );
    let output = lsharp_wasm::wasmgc_runner::run_wasm_wasmgc_component_cli_with_preview2_stdout_and_preopens(
        &component,
        &[],
        "",
        &[preopen],
    )
    .expect("async output stream failure を実行できる");

    assert_eq!(output.stdout, "A");
    assert_eq!(output.exit_code, 0);
    std::fs::remove_dir_all(&dir)
        .expect("async output stream failure fixture directory を削除できる");
}

#[test]
fn wasm_gc_component_cli_fs_runner_maps_pending_output_stream_failure_to_filesystem_error_code() {
    let core = emit_component_cli_pending_output_stream_failure_probe_module();
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
    .expect("pending output stream failure probe を componentize できる");

    let nonce = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("system clock は unix epoch より後であるべき")
        .as_nanos();
    let dir = std::env::temp_dir().join(format!("lsharp_wasmgc_pending_output_failure_{nonce}"));
    std::fs::create_dir_all(&dir)
        .expect("pending output stream failure fixture directory を作成できる");

    let preopen = lsharp_wasm::wasmgc_runner::Preview2Preopen::new(
        &dir,
        "data",
        lsharp_wasm::wasmgc_runner::Preview2PreopenRights::read_write(),
    );
    let output = lsharp_wasm::wasmgc_runner::run_wasm_wasmgc_component_cli_with_preview2_stdout_and_preopens(
        &component,
        &[],
        "",
        &[preopen],
    )
    .expect("pending output stream failure を実行できる");

    assert_eq!(output.stdout, "C");
    assert_eq!(output.exit_code, 0);
    std::fs::remove_dir_all(&dir)
        .expect("pending output stream failure fixture directory を削除できる");
}

#[test]
fn wasm_gc_component_cli_fs_runner_maps_nonblocking_flush_pending_output_stream_failure_to_filesystem_error_code()
 {
    let core = emit_component_cli_nonblocking_flush_pending_output_stream_failure_probe_module();
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
    .expect("non-blocking flush pending output stream failure probe を componentize できる");

    let nonce = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("system clock は unix epoch より後であるべき")
        .as_nanos();
    let dir = std::env::temp_dir().join(format!("lsharp_wasmgc_nonblocking_flush_failure_{nonce}"));
    std::fs::create_dir_all(&dir)
        .expect("non-blocking flush pending output stream failure fixture directory を作成できる");

    let preopen = lsharp_wasm::wasmgc_runner::Preview2Preopen::new(
        &dir,
        "data",
        lsharp_wasm::wasmgc_runner::Preview2PreopenRights::read_write(),
    );
    let output = lsharp_wasm::wasmgc_runner::run_wasm_wasmgc_component_cli_with_preview2_stdout_and_preopens(
        &component,
        &[],
        "",
        &[preopen],
    )
    .expect("non-blocking flush pending output stream failure を実行できる");

    assert_eq!(output.stdout, "F");
    assert_eq!(output.exit_code, 0);
    std::fs::remove_dir_all(&dir)
        .expect("non-blocking flush pending output stream failure fixture directory を削除できる");
}

#[test]
fn wasm_gc_component_cli_fs_runner_reads_descriptor_directly_and_reports_eof() {
    let core = emit_component_cli_direct_read_probe_module();
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
    .expect("descriptor direct read probe を componentize できる");
    let nonce = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("system clock は unix epoch より後であるべき")
        .as_nanos();
    let dir = std::env::temp_dir().join(format!("lsharp_wasmgc_direct_read_{nonce}"));
    let extra_dir = std::env::temp_dir().join(format!("lsharp_wasmgc_direct_read_extra_{nonce}"));
    std::fs::create_dir_all(&dir).expect("direct read fixture directory を作成できる");
    std::fs::create_dir_all(&extra_dir).expect("second direct read fixture directory を作成できる");
    std::fs::write(dir.join("input.txt"), b"hello").expect("direct read fixture file を作成できる");

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
    .expect("descriptor direct read を実行できる");

    assert_eq!(output.stdout, "hello");
    assert_eq!(output.exit_code, 0);
    std::fs::remove_dir_all(&dir).expect("direct read fixture directory を削除できる");
    std::fs::remove_dir_all(&extra_dir).expect("second direct read fixture directory を削除できる");
}

#[test]
fn wasm_gc_component_cli_fs_runner_writes_and_appends_streams_then_drops_resources() {
    let core = emit_component_cli_write_stream_probe_module();
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
    .expect("write/append stream probe を componentize できる");

    let nonce = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("system clock は unix epoch より後であるべき")
        .as_nanos();
    let dir = std::env::temp_dir().join(format!("lsharp_wasmgc_write_stream_{nonce}"));
    let extra_dir = std::env::temp_dir().join(format!("lsharp_wasmgc_write_stream_extra_{nonce}"));
    std::fs::create_dir_all(&dir).expect("write stream fixture directory を作成できる");
    std::fs::create_dir_all(&extra_dir)
        .expect("second write stream fixture directory を作成できる");

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
    .expect("write/append stream を実行できる");

    assert_eq!(output.stdout, "");
    assert_eq!(output.exit_code, 0);
    assert_eq!(
        std::fs::read(dir.join("output.txt")).expect("write stream の成果物を読める"),
        b"hello!"
    );
    std::fs::remove_dir_all(&dir).expect("write stream fixture directory を削除できる");
    std::fs::remove_dir_all(&extra_dir)
        .expect("second write stream fixture directory を削除できる");
}

#[test]
fn wasm_gc_component_cli_fs_runner_writes_zeroes_and_drops_resources() {
    let core = emit_component_cli_zeroes_stream_probe_module();
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
    .expect("write-zeroes probe を componentize できる");

    let nonce = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("system clock は unix epoch より後であるべき")
        .as_nanos();
    let dir = std::env::temp_dir().join(format!("lsharp_wasmgc_zeroes_{nonce}"));
    let extra_dir = std::env::temp_dir().join(format!("lsharp_wasmgc_zeroes_extra_{nonce}"));
    std::fs::create_dir_all(&dir).expect("write-zeroes fixture directory を作成できる");
    std::fs::create_dir_all(&extra_dir)
        .expect("second write-zeroes fixture directory を作成できる");

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
    .expect("output-stream blocking-write-zeroes-and-flush を実行できる");

    assert_eq!(output.stdout, "");
    assert_eq!(output.exit_code, 0);
    assert_eq!(
        std::fs::read(dir.join("zeros.bin")).expect("write-zeroes の成果物を読める"),
        [0, 0, 0]
    );
    std::fs::remove_dir_all(&dir).expect("write-zeroes fixture directory を削除できる");
    std::fs::remove_dir_all(&extra_dir)
        .expect("second write-zeroes fixture directory を削除できる");
}

#[test]
fn wasm_gc_component_cli_fs_runner_checks_writes_and_flushes_stream_then_drops_resources() {
    let core = emit_component_cli_check_write_stream_probe_module();
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
    .expect("check-write stream probe を componentize できる");

    let nonce = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("system clock は unix epoch より後であるべき")
        .as_nanos();
    let dir = std::env::temp_dir().join(format!("lsharp_wasmgc_check_write_{nonce}"));
    let extra_dir = std::env::temp_dir().join(format!("lsharp_wasmgc_check_write_extra_{nonce}"));
    std::fs::create_dir_all(&dir).expect("check-write fixture directory を作成できる");
    std::fs::create_dir_all(&extra_dir).expect("second check-write fixture directory を作成できる");

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
    .expect("output-stream check-write/write/flush を実行できる");

    assert_eq!(output.stdout, "");
    assert_eq!(output.exit_code, 0);
    assert_eq!(
        std::fs::read(dir.join("checked.txt")).expect("check-write の成果物を読める"),
        b"hello"
    );
    std::fs::remove_dir_all(&dir).expect("check-write fixture directory を削除できる");
    std::fs::remove_dir_all(&extra_dir).expect("second check-write fixture directory を削除できる");
}

#[test]
fn wasm_gc_component_cli_fs_runner_writes_zeroes_after_check_write_then_drops_resources() {
    let core = emit_component_cli_direct_write_zeroes_stream_probe_module();
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
    .expect("direct write-zeroes stream probe を componentize できる");

    let nonce = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("system clock は unix epoch より後であるべき")
        .as_nanos();
    let dir = std::env::temp_dir().join(format!("lsharp_wasmgc_direct_zeroes_{nonce}"));
    let extra_dir = std::env::temp_dir().join(format!("lsharp_wasmgc_direct_zeroes_extra_{nonce}"));
    std::fs::create_dir_all(&dir).expect("direct write-zeroes fixture directory を作成できる");
    std::fs::create_dir_all(&extra_dir)
        .expect("second direct write-zeroes fixture directory を作成できる");

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
    .expect("output-stream check-write/write-zeroes/blocking-flush を実行できる");

    assert_eq!(output.stdout, "");
    assert_eq!(output.exit_code, 0);
    assert_eq!(
        std::fs::read(dir.join("direct-zeroes.bin")).expect("direct write-zeroes の成果物を読める"),
        [0, 0, 0, 0]
    );
    std::fs::remove_dir_all(&dir).expect("direct write-zeroes fixture directory を削除できる");
    std::fs::remove_dir_all(&extra_dir)
        .expect("second direct write-zeroes fixture directory を削除できる");
}

#[test]
fn wasm_gc_component_cli_fs_runner_writes_descriptor_directly_and_stats_file() {
    let core = emit_component_cli_direct_write_stat_probe_module();
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
    .expect("direct write/stat probe を componentize できる");

    let nonce = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("system clock は unix epoch より後であるべき")
        .as_nanos();
    let dir = std::env::temp_dir().join(format!("lsharp_wasmgc_direct_write_{nonce}"));
    let extra_dir = std::env::temp_dir().join(format!("lsharp_wasmgc_direct_write_extra_{nonce}"));
    std::fs::create_dir_all(&dir).expect("direct write fixture directory を作成できる");
    std::fs::create_dir_all(&extra_dir)
        .expect("second direct write fixture directory を作成できる");

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
    .expect("descriptor direct write/stat を実行できる");

    assert_eq!(output.stdout, "");
    assert_eq!(output.exit_code, 0);
    assert_eq!(
        std::fs::read(dir.join("output.txt")).expect("direct write の成果物を読める"),
        b"hello"
    );
    std::fs::remove_dir_all(&dir).expect("direct write fixture directory を削除できる");
    std::fs::remove_dir_all(&extra_dir)
        .expect("second direct write fixture directory を削除できる");
}

#[test]
fn wasm_gc_component_cli_fs_runner_drops_descriptor_after_direct_write_error() {
    let core = emit_component_cli_direct_write_error_probe_module();
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
    .expect("direct write error probe を componentize できる");

    let nonce = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("system clock は unix epoch より後であるべき")
        .as_nanos();
    let dir = std::env::temp_dir().join(format!("lsharp_wasmgc_write_error_{nonce}"));
    let extra_dir = std::env::temp_dir().join(format!("lsharp_wasmgc_write_error_extra_{nonce}"));
    std::fs::create_dir_all(&dir).expect("write error fixture directory を作成できる");
    std::fs::create_dir_all(&extra_dir).expect("second write error fixture directory を作成できる");
    std::fs::write(dir.join("input.txt"), b"seed").expect("write error fixture file を作成できる");

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
    .expect("read-only descriptor の direct write error を実行できる");

    assert_eq!(output.stdout, "");
    assert_eq!(output.exit_code, 0);
    assert_eq!(
        std::fs::read(dir.join("input.txt")).expect("write error fixture の成果物を読める"),
        b"seed"
    );
    std::fs::remove_dir_all(&dir).expect("write error fixture directory を削除できる");
    std::fs::remove_dir_all(&extra_dir).expect("second write error fixture directory を削除できる");
}

#[test]
fn wasm_gc_component_cli_fs_runner_reads_directory_entries_and_drops_stream() {
    let core = emit_component_cli_read_directory_probe_module();
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
    .expect("read-directory probe を componentize できる");

    let nonce = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("system clock は unix epoch より後であるべき")
        .as_nanos();
    let dir = std::env::temp_dir().join(format!("lsharp_wasmgc_read_directory_{nonce}"));
    let extra_dir =
        std::env::temp_dir().join(format!("lsharp_wasmgc_read_directory_extra_{nonce}"));
    std::fs::create_dir_all(&dir).expect("read-directory fixture directory を作成できる");
    std::fs::create_dir_all(&extra_dir)
        .expect("second read-directory fixture directory を作成できる");
    std::fs::write(dir.join("input.txt"), b"hello")
        .expect("read-directory fixture file を作成できる");

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
    .expect("read-directory と directory-entry stream を実行できる");

    assert_eq!(output.stdout, "input.txt");
    assert_eq!(output.exit_code, 0);
    std::fs::remove_dir_all(&dir).expect("read-directory fixture directory を削除できる");
    std::fs::remove_dir_all(&extra_dir)
        .expect("second read-directory fixture directory を削除できる");
}

#[test]
fn wasm_gc_component_cli_fs_runner_reports_descriptor_type_and_flags() {
    let core = emit_component_cli_descriptor_type_flags_probe_module();
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
    .expect("descriptor type/flags probe を componentize できる");

    let nonce = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("system clock は unix epoch より後であるべき")
        .as_nanos();
    let dir = std::env::temp_dir().join(format!("lsharp_wasmgc_descriptor_flags_{nonce}"));
    let extra_dir =
        std::env::temp_dir().join(format!("lsharp_wasmgc_descriptor_flags_extra_{nonce}"));
    std::fs::create_dir_all(&dir).expect("descriptor flags fixture directory を作成できる");
    std::fs::create_dir_all(&extra_dir)
        .expect("second descriptor flags fixture directory を作成できる");
    std::fs::write(dir.join("input.txt"), b"hello")
        .expect("descriptor flags fixture file を作成できる");

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
    .expect("descriptor type/flags を実行できる");
    assert_eq!(output.stdout, "");
    assert_eq!(output.exit_code, 0);
    assert_eq!(
        std::fs::read(dir.join("input.txt")).expect("descriptor flags fixture を読める"),
        b"hello"
    );
    std::fs::remove_dir_all(&dir).expect("descriptor flags fixture directory を削除できる");
    std::fs::remove_dir_all(&extra_dir)
        .expect("second descriptor flags fixture directory を削除できる");
}
