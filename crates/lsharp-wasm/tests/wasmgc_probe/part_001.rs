#[test]
fn wasm_gc_component_output_cli_backend_emits_canonical_run_export() {
    let module = IrModule {
        functions: vec![Function {
            name: "main".to_string(),
            params: vec![],
            result: IrType::I64,
            locals: vec![],
            body: vec![
                Instruction::I32Const(67),
                Instruction::I32Const(76),
                Instruction::ArrayNewFixed(0, 2),
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
    let core = lsharp_wasm::wasmgc::emit_wasm_wasmgc_component_cli(&module)
        .expect("WasmGC CLI backend が canonical run export を生成できる");
    let wit_file = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("..")
        .join("wit")
        .join("lsharp-wasmgc-output.wit");

    let component = lsharp_wasm::component_adapter::componentize_core_module(
        &core,
        &wit_file,
        "wasmgc-cli",
        &[],
    )
    .expect("WasmGC CLI backend の core を componentize できる");
    wasmparser::Validator::new()
        .validate_all(&component)
        .expect("WasmGC CLI component が validation に成功する");
}

#[test]
fn wasm_gc_component_cli_runner_executes_wasi_cli_run_with_preview2_stdout() {
    let module = IrModule {
        functions: vec![Function {
            name: "main".to_string(),
            params: vec![],
            result: IrType::I64,
            locals: vec![],
            body: vec![
                Instruction::I32Const(67),
                Instruction::I32Const(76),
                Instruction::ArrayNewFixed(0, 2),
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
    let core = lsharp_wasm::wasmgc::emit_wasm_wasmgc_component_cli(&module)
        .expect("WasmGC CLI backend が core を生成できる");
    let wit_file = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("..")
        .join("wit")
        .join("lsharp-wasmgc-output.wit");
    let component = lsharp_wasm::component_adapter::componentize_core_module(
        &core,
        &wit_file,
        "wasmgc-cli",
        &[],
    )
    .expect("WasmGC CLI core を componentize できる");

    let output = lsharp_wasm::wasmgc_runner::run_wasm_wasmgc_component_cli_with_preview2_stdout(
        &component,
        None,
        &[],
        "",
    )
    .expect("WASI Preview2 wasi:cli/run で WasmGC Component を実行できる");

    assert_eq!(output.stdout, "CL");
    assert_eq!(output.exit_code, 0);
}

#[test]
fn wasm_gc_component_cli_artifact_round_trip_preserves_wasi_cli_run() {
    let module = IrModule {
        functions: vec![Function {
            name: "main".to_string(),
            params: vec![],
            result: IrType::I64,
            locals: vec![],
            body: vec![
                Instruction::I32Const(67),
                Instruction::I32Const(76),
                Instruction::ArrayNewFixed(0, 2),
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
    let core = lsharp_wasm::wasmgc::emit_wasm_wasmgc_component_cli(&module)
        .expect("WasmGC CLI backend が core を生成できる");
    let wit_file = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("..")
        .join("wit")
        .join("lsharp-wasmgc-output.wit");
    let component = lsharp_wasm::component_adapter::componentize_core_module(
        &core,
        &wit_file,
        "wasmgc-cli",
        &[],
    )
    .expect("WasmGC CLI core を componentize できる");
    let direct = lsharp_wasm::wasmgc_runner::run_wasm_wasmgc_component_cli_with_preview2_stdout(
        &component,
        None,
        &[],
        "",
    )
    .expect("in-memory wasi:cli/run を実行できる");

    let nonce = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("system clock は unix epoch より後であるべき")
        .as_nanos();
    let dir = std::env::temp_dir().join(format!(
        "lsharp_wasmgc_cli_component_artifact_{}_{}",
        std::process::id(),
        nonce
    ));
    std::fs::create_dir_all(&dir).expect("CLI Component artifact directory を作成できる");
    let path = dir.join("Main.component.wasm");
    lsharp_wasm::component_adapter::write_component_artifact(&path, &component)
        .expect("CLI Component artifact を atomic に保存できる");
    let artifact = lsharp_wasm::component_adapter::read_component_artifact(&path)
        .expect("CLI Component artifact を再読込できる");
    assert_eq!(
        artifact, component,
        "保存・再読込で Component bytes を変質させない"
    );
    wasmparser::Validator::new()
        .validate_all(&artifact)
        .expect("再読込した CLI Component artifact が validation に成功する");
    let mut config = Config::new();
    config.wasm_gc(true);
    let engine = Engine::new(&config).expect("CLI artifact runtime 用 WasmGC engine を作成できる");
    wasmtime::component::Component::new(&engine, &artifact)
        .expect("再読込した CLI Component artifact を検証できる");

    let round_trip =
        lsharp_wasm::wasmgc_runner::run_wasm_wasmgc_component_cli_with_preview2_stdout(
            &artifact,
            None,
            &[],
            "",
        )
        .expect("再読込した CLI Component artifact を同じ Preview2 runtime で実行できる");
    assert_eq!(round_trip.stdout, direct.stdout);
    assert_eq!(round_trip.exit_code, direct.exit_code);
    assert_eq!(round_trip.stdout, "CL");
    assert_eq!(round_trip.exit_code, 0);
    std::fs::remove_dir_all(&dir).expect("CLI Component artifact directory を削除できる");
}

#[test]
fn wasm_gc_component_cli_runner_maps_wasi_cli_exit_to_exit_status() {
    let core = emit_component_output_cli_exit_probe_module(1);
    let wit_file = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("..")
        .join("wit")
        .join("lsharp-wasmgc-output.wit");
    let component = lsharp_wasm::component_adapter::componentize_core_module(
        &core,
        &wit_file,
        "wasmgc-cli",
        &[],
    )
    .expect("wasi:cli/exit を使う WasmGC CLI core を componentize できる");

    let output = lsharp_wasm::wasmgc_runner::run_wasm_wasmgc_component_cli_with_preview2_stdout(
        &component,
        None,
        &[],
        "",
    )
    .expect("wasi:cli/exit は終了コードとして扱える");

    assert_eq!(output.stdout, "");
    assert_eq!(output.exit_code, 1);
}

#[test]
fn wasm_gc_component_cli_runner_maps_failed_wasi_cli_run_result_to_exit_status() {
    let core = emit_component_output_cli_run_probe_module_with_result(1);
    let wit_file = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("..")
        .join("wit")
        .join("lsharp-wasmgc-output.wit");
    let component = lsharp_wasm::component_adapter::componentize_core_module(
        &core,
        &wit_file,
        "wasmgc-cli",
        &[],
    )
    .expect("失敗 result を返す WasmGC CLI core を componentize できる");

    let output = lsharp_wasm::wasmgc_runner::run_wasm_wasmgc_component_cli_with_preview2_stdout(
        &component,
        None,
        &[],
        "",
    )
    .expect("wasi:cli/run の失敗 result は終了コードとして扱える");

    assert_eq!(output.stdout, "");
    assert_eq!(output.exit_code, 1);
}

#[test]
fn wasm_gc_component_cli_fs_runner_enforces_preopen_rights() {
    let core = emit_component_cli_preopen_write_probe_module();
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
    .expect("filesystem capability を持つ WasmGC CLI core を componentize できる");

    let nonce = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("system clock は unix epoch より後であるべき")
        .as_nanos();
    let dir = std::env::temp_dir().join(format!("lsharp_wasmgc_fd_rights_{nonce}"));
    std::fs::create_dir_all(&dir).expect("fd rights fixture directory を作成できる");
    let probe_file = dir.join("rights.txt");

    let no_preopen = lsharp_wasm::wasmgc_runner::run_wasm_wasmgc_component_cli_with_preview2_stdout_and_preopen_rights(
        &component,
        None,
        &[],
        "",
        lsharp_wasm::wasmgc_runner::Preview2PreopenRights::read_write(),
    )
    .expect("preopen がない Component も明示的な失敗 result を返せる");
    assert_eq!(no_preopen.exit_code, 1);

    let read_only = lsharp_wasm::wasmgc_runner::run_wasm_wasmgc_component_cli_with_preview2_stdout_and_preopen_rights(
        &component,
        Some(&dir),
        &[],
        "",
        lsharp_wasm::wasmgc_runner::Preview2PreopenRights::read_only(),
    )
    .expect("read-only preopen は Component を実行できる");
    assert_eq!(read_only.exit_code, 1);
    assert!(
        !probe_file.exists(),
        "read-only preopen は create を許可しない"
    );

    let read_write = lsharp_wasm::wasmgc_runner::run_wasm_wasmgc_component_cli_with_preview2_stdout_and_preopen_rights(
        &component,
        Some(&dir),
        &[],
        "",
        lsharp_wasm::wasmgc_runner::Preview2PreopenRights::read_write(),
    )
    .expect("read-write preopen は Component を実行できる");
    assert_eq!(read_write.exit_code, 0);
    assert!(
        probe_file.exists(),
        "read-write preopen は create を許可する"
    );

    std::fs::remove_dir_all(&dir).expect("fd rights fixture directory を削除できる");
}

#[test]
fn wasm_gc_component_cli_fs_runner_reads_named_preopen_stream_and_drops_resources() {
    let core = emit_component_cli_named_preopen_stream_probe_module();
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
    .expect("named preopen stream probe を componentize できる");

    let nonce = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("system clock は unix epoch より後であるべき")
        .as_nanos();
    let dir = std::env::temp_dir().join(format!("lsharp_wasmgc_named_preopen_{nonce}"));
    let extra_dir = std::env::temp_dir().join(format!("lsharp_wasmgc_named_preopen_extra_{nonce}"));
    std::fs::create_dir_all(&dir).expect("named preopen fixture directory を作成できる");
    std::fs::create_dir_all(&extra_dir)
        .expect("second named preopen fixture directory を作成できる");
    std::fs::write(dir.join("input.txt"), b"hello").expect("stream fixture file を作成できる");

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
    .expect("named preopen の input stream を実行できる");

    assert_eq!(output.stdout, "hello");
    assert_eq!(output.exit_code, 0);
    std::fs::remove_dir_all(&dir).expect("named preopen fixture directory を削除できる");
    std::fs::remove_dir_all(&extra_dir)
        .expect("second named preopen fixture directory を削除できる");
}

#[test]
fn wasm_gc_component_cli_fs_runner_maps_nonblocking_input_stream_failure_to_filesystem_error_code()
{
    let core = emit_component_cli_nonblocking_input_stream_failure_probe_module();
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
    .expect("non-blocking input stream failure probe を componentize できる");

    let nonce = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("system clock は unix epoch より後であるべき")
        .as_nanos();
    let dir = std::env::temp_dir().join(format!("lsharp_wasmgc_nonblocking_read_failure_{nonce}"));
    std::fs::create_dir_all(&dir)
        .expect("non-blocking input stream failure fixture directory を作成できる");
    std::fs::write(dir.join("input.txt"), b"hello")
        .expect("non-blocking input stream failure fixture file を作成できる");

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
    .expect("non-blocking input stream failure を実行できる");

    assert_eq!(output.stdout, "R");
    assert_eq!(output.exit_code, 0);
    std::fs::remove_dir_all(&dir)
        .expect("non-blocking input stream failure fixture directory を削除できる");
}

#[test]
fn wasm_gc_component_cli_fs_runner_maps_nonblocking_input_skip_failure_to_filesystem_error_code() {
    let core = emit_component_cli_nonblocking_input_skip_failure_probe_module();
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
    .expect("non-blocking input skip failure probe を componentize できる");

    let nonce = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("system clock は unix epoch より後であるべき")
        .as_nanos();
    let dir = std::env::temp_dir().join(format!("lsharp_wasmgc_nonblocking_skip_failure_{nonce}"));
    std::fs::create_dir_all(&dir)
        .expect("non-blocking input skip failure fixture directory を作成できる");
    std::fs::write(dir.join("input.txt"), b"hello")
        .expect("non-blocking input skip failure fixture file を作成できる");

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
    .expect("non-blocking input skip failure を実行できる");

    assert_eq!(output.stdout, "S");
    assert_eq!(output.exit_code, 0);
    std::fs::remove_dir_all(&dir)
        .expect("non-blocking input skip failure fixture directory を削除できる");
}

#[test]
fn wasm_gc_component_cli_fs_runner_splices_input_into_output_and_drops_resources() {
    let core = emit_component_cli_splice_stream_probe_module();
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
    .expect("splice stream probe を componentize できる");

    let nonce = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("system clock は unix epoch より後であるべき")
        .as_nanos();
    let dir = std::env::temp_dir().join(format!("lsharp_wasmgc_splice_{nonce}"));
    let extra_dir = std::env::temp_dir().join(format!("lsharp_wasmgc_splice_extra_{nonce}"));
    std::fs::create_dir_all(&dir).expect("splice fixture directory を作成できる");
    std::fs::create_dir_all(&extra_dir).expect("second splice fixture directory を作成できる");
    std::fs::write(dir.join("input.txt"), b"hello").expect("splice input fixture を作成できる");

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
    .expect("output-stream splice を実行できる");

    assert_eq!(output.stdout, "");
    assert_eq!(output.exit_code, 0);
    assert_eq!(
        std::fs::read(dir.join("spliced.txt")).expect("splice の成果物を読める"),
        b"hello"
    );
    std::fs::remove_dir_all(&dir).expect("splice fixture directory を削除できる");
    std::fs::remove_dir_all(&extra_dir).expect("second splice fixture directory を削除できる");
}

#[test]
fn wasm_gc_component_cli_fs_runner_skips_input_stream_then_reads_remaining_bytes() {
    let core = emit_component_cli_skip_stream_probe_module();
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
    .expect("skip stream probe を componentize できる");

    let nonce = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("system clock は unix epoch より後であるべき")
        .as_nanos();
    let dir = std::env::temp_dir().join(format!("lsharp_wasmgc_skip_{nonce}"));
    let extra_dir = std::env::temp_dir().join(format!("lsharp_wasmgc_skip_extra_{nonce}"));
    std::fs::create_dir_all(&dir).expect("skip fixture directory を作成できる");
    std::fs::create_dir_all(&extra_dir).expect("second skip fixture directory を作成できる");
    std::fs::write(dir.join("input.txt"), b"hello!").expect("skip input fixture を作成できる");

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
    .expect("input-stream skip/blocking-skip を実行できる");

    assert_eq!(output.stdout, "llo!");
    assert_eq!(output.exit_code, 0);
    std::fs::remove_dir_all(&dir).expect("skip fixture directory を削除できる");
    std::fs::remove_dir_all(&extra_dir).expect("second skip fixture directory を削除できる");
}

#[test]
fn wasm_gc_component_cli_fs_runner_reads_nonblocking_input_stream_and_completes_remaining_bytes_and_reports_eof()
 {
    let core = emit_component_cli_read_stream_probe_module();
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
    .expect("read stream probe を componentize できる");

    let nonce = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("system clock は unix epoch より後であるべき")
        .as_nanos();
    let dir = std::env::temp_dir().join(format!("lsharp_wasmgc_read_{nonce}"));
    let extra_dir = std::env::temp_dir().join(format!("lsharp_wasmgc_read_extra_{nonce}"));
    std::fs::create_dir_all(&dir).expect("read fixture directory を作成できる");
    std::fs::create_dir_all(&extra_dir).expect("second read fixture directory を作成できる");
    std::fs::write(dir.join("input.txt"), b"hello").expect("read input fixture を作成できる");

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
    .expect("input-stream read を実行できる");

    assert_eq!(output.stdout, "helloE");
    assert_eq!(output.exit_code, 0);
    std::fs::remove_dir_all(&dir).expect("read fixture directory を削除できる");
    std::fs::remove_dir_all(&extra_dir).expect("second read fixture directory を削除できる");
}

#[test]
fn wasm_gc_component_cli_fs_runner_reads_empty_input_stream_as_empty_success() {
    let core = emit_component_cli_empty_read_stream_probe_module();
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
    .expect("empty read stream probe を componentize できる");

    let nonce = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("system clock は unix epoch より後であるべき")
        .as_nanos();
    let dir = std::env::temp_dir().join(format!("lsharp_wasmgc_empty_read_{nonce}"));
    let extra_dir = std::env::temp_dir().join(format!("lsharp_wasmgc_empty_read_extra_{nonce}"));
    std::fs::create_dir_all(&dir).expect("empty read fixture directory を作成できる");
    std::fs::create_dir_all(&extra_dir).expect("second empty read fixture directory を作成できる");
    std::fs::write(dir.join("input.txt"), b"").expect("empty read fixture file を作成できる");

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
    .expect("empty input-stream read を実行できる");

    assert_eq!(output.stdout, "Z");
    assert_eq!(output.exit_code, 0);
    std::fs::remove_dir_all(&dir).expect("empty read fixture directory を削除できる");
    std::fs::remove_dir_all(&extra_dir).expect("second empty read fixture directory を削除できる");
}

#[test]
fn wasm_gc_component_cli_fs_runner_blocking_reads_empty_input_stream_reports_closed() {
    let core = emit_component_cli_empty_blocking_read_stream_probe_module();
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
    .expect("empty blocking-read stream probe を componentize できる");

    let nonce = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("system clock は unix epoch より後であるべき")
        .as_nanos();
    let dir = std::env::temp_dir().join(format!("lsharp_wasmgc_empty_blocking_read_{nonce}"));
    let extra_dir =
        std::env::temp_dir().join(format!("lsharp_wasmgc_empty_blocking_read_extra_{nonce}"));
    std::fs::create_dir_all(&dir).expect("empty blocking-read fixture directory を作成できる");
    std::fs::create_dir_all(&extra_dir)
        .expect("second empty blocking-read fixture directory を作成できる");
    std::fs::write(dir.join("input.txt"), b"")
        .expect("empty blocking-read fixture file を作成できる");

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
    .expect("empty input-stream blocking-read を実行できる");

    assert_eq!(output.stdout, "C");
    assert_eq!(output.exit_code, 0);
    std::fs::remove_dir_all(&dir).expect("empty blocking-read fixture directory を削除できる");
    std::fs::remove_dir_all(&extra_dir)
        .expect("second empty blocking-read fixture directory を削除できる");
}
