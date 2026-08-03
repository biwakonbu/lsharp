use super::support::*;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};

#[derive(Debug, Default, Clone, PartialEq, Eq)]
struct PartialFdWriteCapture {
    stdout: Vec<u8>,
    file: Vec<u8>,
    file_write_calls: usize,
    fd_write_calls: usize,
    fd_close_calls: usize,
    last_fd: i32,
    last_iovs_len: i32,
    last_iov_len: i32,
}

#[derive(Debug, Default, Clone, PartialEq, Eq)]
struct PartialFdReadCapture {
    stdout: Vec<u8>,
    read_payload: Vec<u8>,
    fd_read_calls: usize,
    path_open_calls: usize,
    fd_close_calls: usize,
}

fn fixture_dir(name: &str) -> PathBuf {
    std::env::temp_dir().join(format!(
        "lsharp_selfhost_standalone_io_{}_{}",
        name,
        std::process::id()
    ))
}

fn read_i32<T>(memory: wasmtime::Memory, caller: &wasmtime::Caller<'_, T>, addr: i32) -> i32 {
    let start = addr as usize;
    let end = start + 4;
    let bytes: [u8; 4] = memory.data(caller)[start..end]
        .try_into()
        .expect("i32 を読めるべき");
    i32::from_le_bytes(bytes)
}

fn write_i32<T>(
    memory: wasmtime::Memory,
    caller: &mut wasmtime::Caller<'_, T>,
    addr: i32,
    value: i32,
) {
    let start = addr as usize;
    let end = start + 4;
    memory.data_mut(caller)[start..end].copy_from_slice(&value.to_le_bytes());
}

fn run_with_partial_fd_write(wasm: &[u8], dir: &Path) -> Result<PartialFdWriteCapture, String> {
    run_with_partial_fd_write_with_close_errno(wasm, dir, 0)
}

fn run_with_partial_fd_write_with_close_errno(
    wasm: &[u8],
    dir: &Path,
    close_errno: i32,
) -> Result<PartialFdWriteCapture, String> {
    use wasmtime::{Engine, Linker, Module, Store};
    use wasmtime_wasi::{preview1::WasiP1Ctx, WasiCtxBuilder};

    let engine = Engine::default();
    let mut linker = Linker::<WasiP1Ctx>::new(&engine);
    linker.allow_shadowing(true);
    wasmtime_wasi::preview1::add_to_linker_sync(&mut linker, |ctx| ctx)
        .map_err(|err| format!("WASI linker 構築に失敗: {err}"))?;

    let capture = Arc::new(Mutex::new(PartialFdWriteCapture::default()));
    let fd_write_capture = Arc::clone(&capture);
    linker
        .func_wrap(
            "wasi_snapshot_preview1",
            "fd_write",
            move |mut caller: wasmtime::Caller<'_, WasiP1Ctx>,
                  fd: i32,
                  iovs: i32,
                  iovs_len: i32,
                  nwritten: i32|
                  -> i32 {
                let memory = caller
                    .get_export("memory")
                    .and_then(|export| export.into_memory())
                    .expect("standalone Wasm は memory export を持つべき");
                let mut output = Vec::new();
                for index in 0..iovs_len {
                    let base = iovs + index * 8;
                    let ptr = read_i32(memory, &caller, base);
                    let len = read_i32(memory, &caller, base + 4);
                    let start = ptr as usize;
                    let end = start + len as usize;
                    output.extend_from_slice(&memory.data(&caller)[start..end]);
                }

                let mut state = fd_write_capture.lock().expect("fd_write capture lock");
                state.fd_write_calls += 1;
                state.last_fd = fd;
                state.last_iovs_len = iovs_len;
                state.last_iov_len = output.len() as i32;
                let written = if fd == 1 || fd == 2 {
                    state.stdout.extend_from_slice(&output);
                    output.len()
                } else {
                    let write_len = if state.file_write_calls == 0 {
                        output.len().min(2)
                    } else {
                        output.len()
                    };
                    state.file.extend_from_slice(&output[..write_len]);
                    state.file_write_calls += 1;
                    write_len
                };
                write_i32(memory, &mut caller, nwritten, written as i32);
                0
            },
        )
        .map_err(|err| format!("partial fd_write shim 登録に失敗: {err}"))?;

    let fd_close_capture = Arc::clone(&capture);
    linker
        .func_wrap(
            "wasi_snapshot_preview1",
            "fd_close",
            move |_caller: wasmtime::Caller<'_, WasiP1Ctx>, _fd: i32| -> i32 {
                let mut state = fd_close_capture.lock().expect("fd_close capture lock");
                state.fd_close_calls += 1;
                close_errno
            },
        )
        .map_err(|err| format!("fd_close shim 登録に失敗: {err}"))?;

    let mut builder = WasiCtxBuilder::new();
    builder
        .preopened_dir(
            dir,
            ".",
            wasmtime_wasi::DirPerms::all(),
            wasmtime_wasi::FilePerms::all(),
        )
        .map_err(|err| format!("preopened_dir に失敗: {err}"))?;
    let mut store = Store::new(&engine, builder.build_p1());
    let module = Module::new(&engine, wasm).map_err(|err| format!("Wasm 構築に失敗: {err:#}"))?;
    let instance = linker
        .instantiate(&mut store, &module)
        .map_err(|err| format!("Wasm instance 化に失敗: {err}"))?;
    let start = instance
        .get_typed_func::<(), ()>(&mut store, "_start")
        .map_err(|err| format!("_start export が見つからない: {err}"))?;
    start
        .call(&mut store, ())
        .map_err(|err| format!("standalone Wasm 実行に失敗: {err}"))?;

    capture
        .lock()
        .map(|state| state.clone())
        .map_err(|_| "fd_write capture lock が poison された".to_string())
}

fn run_with_partial_fd_read(wasm: &[u8], dir: &Path) -> Result<PartialFdReadCapture, String> {
    run_with_partial_fd_read_with_errors(wasm, dir, 0, 0)
}

fn run_with_partial_fd_read_with_close_errno(
    wasm: &[u8],
    dir: &Path,
    close_errno: i32,
) -> Result<PartialFdReadCapture, String> {
    run_with_partial_fd_read_with_errors(wasm, dir, 0, close_errno)
}

fn run_with_partial_fd_read_with_fd_read_errno(
    wasm: &[u8],
    dir: &Path,
    fd_read_errno: i32,
) -> Result<PartialFdReadCapture, String> {
    run_with_partial_fd_read_with_errors(wasm, dir, fd_read_errno, 0)
}

fn run_with_partial_fd_read_with_path_open_errno(
    wasm: &[u8],
    dir: &Path,
    path_open_errno: i32,
) -> Result<PartialFdReadCapture, String> {
    run_with_partial_fd_read_with_errors_and_path_open(wasm, dir, 0, 0, Some(path_open_errno))
}

fn run_with_partial_fd_read_with_errors(
    wasm: &[u8],
    dir: &Path,
    fd_read_errno: i32,
    close_errno: i32,
) -> Result<PartialFdReadCapture, String> {
    run_with_partial_fd_read_with_errors_and_path_open(wasm, dir, fd_read_errno, close_errno, None)
}

fn run_with_partial_fd_read_with_errors_and_path_open(
    wasm: &[u8],
    dir: &Path,
    fd_read_errno: i32,
    close_errno: i32,
    path_open_errno: Option<i32>,
) -> Result<PartialFdReadCapture, String> {
    use wasmtime::{Engine, Linker, Module, Store};
    use wasmtime_wasi::{preview1::WasiP1Ctx, WasiCtxBuilder};

    let engine = Engine::default();
    let mut linker = Linker::<WasiP1Ctx>::new(&engine);
    linker.allow_shadowing(true);
    wasmtime_wasi::preview1::add_to_linker_sync(&mut linker, |ctx| ctx)
        .map_err(|err| format!("WASI linker 構築に失敗: {err}"))?;

    let capture = Arc::new(Mutex::new(PartialFdReadCapture::default()));
    if let Some(path_open_errno) = path_open_errno {
        let path_open_capture = Arc::clone(&capture);
        linker
            .func_wrap(
                "wasi_snapshot_preview1",
                "path_open",
                move |_caller: wasmtime::Caller<'_, WasiP1Ctx>,
                      _dirfd: i32,
                      _dirflags: i32,
                      _path: i32,
                      _path_len: i32,
                      _oflags: i32,
                      _fs_rights_base: i64,
                      _fs_rights_inheriting: i64,
                      _fdflags: i32,
                      _fd_ptr: i32|
                      -> i32 {
                    let mut state = path_open_capture.lock().expect("path_open capture lock");
                    state.path_open_calls += 1;
                    path_open_errno
                },
            )
            .map_err(|err| format!("path_open shim 登録に失敗: {err}"))?;
    }
    let fd_read_capture = Arc::clone(&capture);
    linker
        .func_wrap(
            "wasi_snapshot_preview1",
            "fd_read",
            move |mut caller: wasmtime::Caller<'_, WasiP1Ctx>,
                  _fd: i32,
                  iovs: i32,
                  iovs_len: i32,
                  nread: i32|
                  -> i32 {
                let memory = caller
                    .get_export("memory")
                    .and_then(|export| export.into_memory())
                    .expect("standalone Wasm は memory export を持つべき");
                let mut state = fd_read_capture.lock().expect("fd_read capture lock");
                state.fd_read_calls += 1;
                let payload = b"payload";
                let remaining = payload.len().saturating_sub(state.read_payload.len());
                let mut requested = 0usize;
                for index in 0..iovs_len {
                    let base = iovs + index * 8;
                    requested += read_i32(memory, &caller, base + 4) as usize;
                }
                let write_len = remaining.min(requested).min(if state.fd_read_calls == 1 {
                    2
                } else {
                    usize::MAX
                });
                let mut copied = 0usize;
                for index in 0..iovs_len {
                    let base = iovs + index * 8;
                    let ptr = read_i32(memory, &caller, base);
                    let len = read_i32(memory, &caller, base + 4) as usize;
                    let chunk_len = (write_len - copied).min(len);
                    if chunk_len == 0 {
                        break;
                    }
                    let start = ptr as usize;
                    let end = start + chunk_len;
                    let payload_start = state.read_payload.len();
                    memory.data_mut(&mut caller)[start..end]
                        .copy_from_slice(&payload[payload_start..payload_start + chunk_len]);
                    state
                        .read_payload
                        .extend_from_slice(&payload[payload_start..payload_start + chunk_len]);
                    copied += chunk_len;
                }
                write_i32(memory, &mut caller, nread, copied as i32);
                if state.fd_read_calls == 1 {
                    fd_read_errno
                } else {
                    0
                }
            },
        )
        .map_err(|err| format!("partial fd_read shim 登録に失敗: {err}"))?;

    let fd_close_capture = Arc::clone(&capture);
    linker
        .func_wrap(
            "wasi_snapshot_preview1",
            "fd_close",
            move |_caller: wasmtime::Caller<'_, WasiP1Ctx>, _fd: i32| -> i32 {
                let mut state = fd_close_capture.lock().expect("fd_close capture lock");
                state.fd_close_calls += 1;
                close_errno
            },
        )
        .map_err(|err| format!("fd_close shim 登録に失敗: {err}"))?;

    let stdout = wasmtime_wasi::pipe::MemoryOutputPipe::new(16 * 1024 * 1024);
    let mut builder = WasiCtxBuilder::new();
    builder.stdout(stdout.clone());
    builder
        .preopened_dir(
            dir,
            ".",
            wasmtime_wasi::DirPerms::all(),
            wasmtime_wasi::FilePerms::all(),
        )
        .map_err(|err| format!("preopened_dir に失敗: {err}"))?;
    let mut store = Store::new(&engine, builder.build_p1());
    let module = Module::new(&engine, wasm).map_err(|err| format!("Wasm 構築に失敗: {err:#}"))?;
    let instance = linker
        .instantiate(&mut store, &module)
        .map_err(|err| format!("Wasm instance 化に失敗: {err}"))?;
    let start = instance
        .get_typed_func::<(), ()>(&mut store, "_start")
        .map_err(|err| format!("_start export が見つからない: {err}"))?;
    start
        .call(&mut store, ())
        .map_err(|err| format!("standalone Wasm 実行に失敗: {err}"))?;
    drop(store);

    let stdout = stdout
        .try_into_inner()
        .ok_or_else(|| "stdout capture lock が poison された".to_string())?
        .to_vec();
    capture
        .lock()
        .map(|state| PartialFdReadCapture {
            stdout,
            ..state.clone()
        })
        .map_err(|_| "fd_read capture lock が poison された".to_string())
}

fn compile_standalone_source(source: &str, dir: &Path, cli_cache_env: &str) -> Vec<u8> {
    std::fs::create_dir_all(dir).expect("standalone source fixture directory の作成に失敗");
    std::fs::write(dir.join("input.ls"), source).expect("standalone input.ls の書き込みに失敗");
    let harness = r#"
(defn main []
  (let [src (read-file "input.ls")
        program (parse-program src)
        pair (compile-program-functions-with-source-base src program 12)
        functions (vector-get pair 1)
        data (vector-get pair 2)
        wasm (build-wasm-bytes-wasi-standalone functions data)]
    (do
      (write-file-bytes "output.wasm" wasm)
      0)))
"#;
    let cli_source = format!("{}\n{}", selfhost_cli_runtime_bundle(), harness);
    let cli_wasm = if let Some(cache_path) = std::env::var_os(cli_cache_env) {
        let cache_path = PathBuf::from(cache_path);
        if cache_path.exists() {
            std::fs::read(cache_path).expect("cached selfhost CLI Wasm の読み込みに失敗")
        } else {
            let bytes = compile_only(&cli_source);
            std::fs::write(cache_path, &bytes).expect("cached selfhost CLI Wasm の書き込みに失敗");
            bytes
        }
    } else {
        compile_only(&cli_source)
    };
    let output = lsharp_wasm::wasi_runner::run_wasm_wasi_with_dir_args_and_stdin_capture(
        &cli_wasm,
        Some(dir),
        &["compile", "input.ls", "-o", "output.wasm"],
        "",
    )
    .expect("selfhost standalone source compile の実行に失敗");
    assert_eq!(
        output.exit_code, 0,
        "selfhost source compile は成功するべき"
    );
    std::fs::read(dir.join("output.wasm")).expect("standalone output.wasm の読み込みに失敗")
}

#[test]
fn test_e2e_selfhost_standalone_not_preserves_bool_semantics() {
    let capture = run_with_expanded_stack(NATIVE_HARNESS_STACK_BYTES, || {
        let dir = fixture_dir("not_bool_semantics");
        let _ = std::fs::remove_dir_all(&dir);
        let standalone_wasm = compile_standalone_source(
            "(defn main [] (if (not true) (print 1) (print 2)))",
            &dir,
            "LSHARP_STANDALONE_NOT_BOOL_CLI_ARTIFACT",
        );
        let capture = run_with_partial_fd_write(&standalone_wasm, &dir)
            .expect("selfhost standalone not 実行に失敗");
        let _ = std::fs::remove_dir_all(&dir);
        capture
    });

    assert_eq!(capture.stdout, b"2\n", "not true は false 側の枝を選ぶべき");
}

#[test]
fn test_e2e_selfhost_standalone_user_call_after_preview1_import() {
    run_with_expanded_stack(NATIVE_HARNESS_STACK_BYTES, || {
        let dir = fixture_dir("user_call_after_preview1_import");
        let _ = std::fs::remove_dir_all(&dir);
        let standalone_wasm = compile_standalone_source(
            "(defn helper [] (print-string \"helper\"))\n(defn main [] (helper))",
            &dir,
            "LSHARP_STANDALONE_USER_CALL_AFTER_PREVIEW1_IMPORT_CLI_ARTIFACT",
        );
        let capture = run_with_partial_fd_write(&standalone_wasm, &dir)
            .expect("selfhost standalone user call の実行に失敗");
        let _ = std::fs::remove_dir_all(&dir);

        assert_eq!(capture.stdout, b"helper");
    });
}

#[test]
fn test_e2e_selfhost_standalone_read_stdin_runtime() {
    run_with_expanded_stack(NATIVE_HARNESS_STACK_BYTES, || {
        let dir = fixture_dir("read_stdin");
        let _ = std::fs::remove_dir_all(&dir);
        let standalone_wasm = compile_standalone_source(
            r#"(defn main [] (print-string (read-stdin)))"#,
            &dir,
            "LSHARP_STANDALONE_IO_READ_STDIN_CLI_ARTIFACT",
        );
        if let Some(save_path) = std::env::var_os("LSHARP_STANDALONE_IO_READ_STDIN_SAVE_ARTIFACT") {
            std::fs::write(save_path, &standalone_wasm)
                .expect("read-stdin standalone artifact の保存に失敗");
        }
        let boundary_4096 = format!("{}b", "a".repeat(4095));
        let boundary_over_4096 = format!("{}b", "a".repeat(4096));
        let cases = vec![
            ("payload", "payload".to_string()),
            ("empty", String::new()),
            ("4096", boundary_4096),
            ("over-4096", boundary_over_4096),
        ];
        for (label, stdin) in cases {
            let output = lsharp_wasm::wasi_runner::run_wasm_wasi_with_dir_args_and_stdin_capture(
                &standalone_wasm,
                Some(&dir),
                &[],
                &stdin,
            )
            .unwrap_or_else(|err| panic!("read-stdin standalone {label} 実行に失敗: {err}"));
            assert_eq!(output.exit_code, 0, "read-stdin {label} exit code");
            assert_eq!(
                output.stdout.as_bytes(),
                stdin.as_bytes(),
                "read-stdin {label} stdout"
            );
        }

        let errno_capture = run_with_partial_fd_read_with_fd_read_errno(&standalone_wasm, &dir, 1)
            .expect("read-stdin fd_read errno 下の standalone 実行に失敗");
        let _ = std::fs::remove_dir_all(&dir);

        assert_eq!(errno_capture.stdout, b"payload");
        assert_eq!(errno_capture.read_payload, b"payload");
        assert_eq!(errno_capture.fd_read_calls, 3);
        assert_eq!(errno_capture.fd_close_calls, 0);
    });
}

#[test]
fn test_e2e_selfhost_standalone_command_line_runtime() {
    run_with_expanded_stack(NATIVE_HARNESS_STACK_BYTES, || {
        let dir = fixture_dir("command_line");
        let _ = std::fs::remove_dir_all(&dir);
        let standalone_wasm = compile_standalone_source(
            r#"(defn main []
  (do
    (print-string (command-line-arg 0))
    (print-string (command-line-arg 1))
    (print-string (command-line-arg 2))
    (print (command-line-args))))"#,
            &dir,
            "LSHARP_STANDALONE_COMMAND_LINE_CLI_ARTIFACT",
        );
        let with_args = lsharp_wasm::wasi_runner::run_wasm_wasi_with_dir_args_and_stdin_capture(
            &standalone_wasm,
            Some(&dir),
            &["alpha", "beta"],
            "",
        )
        .expect("standalone command-line runtime with args の実行に失敗");
        let without_args =
            lsharp_wasm::wasi_runner::run_wasm_wasi_with_dir_args_and_stdin_capture(
                &standalone_wasm,
                Some(&dir),
                &[],
                "",
            )
            .expect("standalone command-line runtime without args の実行に失敗");
        let _ = std::fs::remove_dir_all(&dir);

        assert_eq!(with_args.exit_code, 0);
        assert_eq!(with_args.stdout.as_bytes(), b"alphabeta2\n");
        assert_eq!(without_args.exit_code, 0);
        assert_eq!(without_args.stdout.as_bytes(), b"0\n");
    });
}

#[test]
fn test_wasi_fd_write_shim_is_used_for_standalone_import() {
    let dir = fixture_dir("shim");
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).expect("fixture directory の作成に失敗");
    let wasm = wat::parse_str(
        r#"
        (module
          (import "wasi_snapshot_preview1" "fd_write"
            (func $fd_write (param i32 i32 i32 i32) (result i32)))
          (memory (export "memory") 1)
          (data (i32.const 100) "payload")
          (func (export "_start")
            (i32.const 0)
            (i32.const 100)
            (i32.store)
            (i32.const 4)
            (i32.const 7)
            (i32.store)
            (i32.const 1)
            (i32.const 0)
            (i32.const 1)
            (i32.const 8)
            (call $fd_write)
            (drop)))
        "#,
    )
    .expect("fd_write shim 用 Wasm の構築に失敗");

    let capture = run_with_partial_fd_write(&wasm, &dir).expect("fd_write shim の実行に失敗");
    let _ = std::fs::remove_dir_all(&dir);
    assert_eq!(capture.fd_write_calls, 1, "fd_write shim が呼ばれるべき");
    assert_eq!(capture.stdout, b"payload");
}

#[test]
fn test_e2e_selfhost_standalone_write_file_retries_partial_fd_write() {
    run_with_expanded_stack(NATIVE_HARNESS_STACK_BYTES, || {
        let dir = fixture_dir("partial_write");
        let cached_artifact =
            std::env::var_os("LSHARP_STANDALONE_IO_ARTIFACT").map(std::path::PathBuf::from);
        let (standalone_wasm, execution_dir) = if let Some(artifact) = cached_artifact {
            let _ = std::fs::remove_dir_all(&dir);
            std::fs::create_dir_all(&dir).expect("cached artifact 用 directory の作成に失敗");
            let standalone_wasm =
                std::fs::read(&artifact).expect("LSHARP_STANDALONE_IO_ARTIFACT の読み込みに失敗");
            (standalone_wasm, dir.clone())
        } else {
            let _ = std::fs::remove_dir_all(&dir);
            std::fs::create_dir_all(&dir).expect("fixture directory の作成に失敗");
            std::fs::write(
                dir.join("input.ls"),
                r#"(defn main [] (write-file "written.txt" "payload"))"#,
            )
            .expect("input.ls の書き込みに失敗");

            let harness = r#"
(defn main []
  (let [src (read-file "input.ls")
        program (parse-program src)
        pair (compile-program-functions-with-source-base src program 12)
        functions (vector-get pair 1)
        data (vector-get pair 2)
        wasm (build-wasm-bytes-wasi-standalone functions data)]
    (do
      (write-file-bytes "output.wasm" wasm)
      0)))
"#;
            let cli_source = format!("{}\n{}", selfhost_cli_runtime_bundle(), harness);
            let cli_wasm_cache =
                std::env::var_os("LSHARP_STANDALONE_IO_CLI_ARTIFACT").map(std::path::PathBuf::from);
            let cli_wasm = if let Some(cache_path) = cli_wasm_cache.as_ref() {
                if cache_path.exists() {
                    std::fs::read(cache_path).expect("cached selfhost CLI Wasm の読み込みに失敗")
                } else {
                    let bytes = compile_only(&cli_source);
                    std::fs::write(cache_path, &bytes)
                        .expect("cached selfhost CLI Wasm の書き込みに失敗");
                    bytes
                }
            } else {
                compile_only(&cli_source)
            };
            let output = lsharp_wasm::wasi_runner::run_wasm_wasi_with_dir_args_and_stdin_capture(
                &cli_wasm,
                Some(&dir),
                &["compile", "input.ls", "-o", "output.wasm"],
                "",
            )
            .expect("selfhost harness の compile 実行に失敗");
            assert_eq!(output.exit_code, 0, "selfhost compile は成功するべき");
            let standalone_wasm = std::fs::read(dir.join("output.wasm"))
                .expect("standalone output.wasm の読み込みに失敗");
            if let Some(save_path) = std::env::var_os("LSHARP_STANDALONE_IO_SAVE_ARTIFACT") {
                std::fs::write(save_path, &standalone_wasm)
                    .expect("standalone artifact の保存に失敗");
            }
            (standalone_wasm, dir.clone())
        };

        let capture = run_with_partial_fd_write(&standalone_wasm, &execution_dir)
            .expect("partial fd_write 下の standalone 実行に失敗");
        let _ = std::fs::remove_dir_all(&dir);

        assert_eq!(capture.stdout, b"");
        assert_eq!(
            capture.file, b"payload",
            "partial fd_write capture: {capture:?}"
        );
        assert_eq!(capture.file_write_calls, 2);
    });
}

#[test]
fn test_e2e_selfhost_standalone_write_file_bytes_retries_partial_fd_write() {
    run_with_expanded_stack(NATIVE_HARNESS_STACK_BYTES, || {
        let dir = fixture_dir("partial_write_bytes");
        let _ = std::fs::remove_dir_all(&dir);
        let standalone_wasm = if let Some(artifact) =
            std::env::var_os("LSHARP_STANDALONE_IO_RAW_ARTIFACT")
        {
            std::fs::create_dir_all(&dir).expect("cached raw artifact 用 directory の作成に失敗");
            std::fs::read(artifact).expect("LSHARP_STANDALONE_IO_RAW_ARTIFACT の読み込みに失敗")
        } else {
            let standalone_wasm = compile_standalone_source(
                r#"(defn main []
  (let [bytes (vector-push
                (vector-push
                  (vector-push
                    (vector-push
                      (vector-push (vector-new 5) 0)
                      97)
                    115)
                  109)
                33)]
    (write-file-bytes "raw.bin" bytes)))
"#,
                &dir,
                "LSHARP_STANDALONE_IO_RAW_CLI_ARTIFACT",
            );
            if let Some(save_path) = std::env::var_os("LSHARP_STANDALONE_IO_RAW_SAVE_ARTIFACT") {
                std::fs::write(save_path, &standalone_wasm)
                    .expect("raw standalone artifact の保存に失敗");
            }
            standalone_wasm
        };

        let capture = run_with_partial_fd_write(&standalone_wasm, &dir)
            .expect("partial fd_write 下の raw standalone 実行に失敗");
        let _ = std::fs::remove_dir_all(&dir);

        assert_eq!(capture.stdout, b"");
        assert_eq!(
            capture.file, b"\0asm!",
            "partial raw fd_write capture: {capture:?}"
        );
        assert_eq!(capture.file_write_calls, 2);
    });
}

#[test]
fn test_e2e_selfhost_standalone_write_helpers_return_fd_close_errno() {
    run_with_expanded_stack(NATIVE_HARNESS_STACK_BYTES, || {
        let dir = fixture_dir("close_errno_write");
        let _ = std::fs::remove_dir_all(&dir);
        let standalone_wasm = if let Some(artifact) =
            std::env::var_os("LSHARP_STANDALONE_IO_CLOSE_WRITE_ARTIFACT")
        {
            std::fs::create_dir_all(&dir).expect("cached close artifact 用 directory の作成に失敗");
            std::fs::read(artifact)
                .expect("LSHARP_STANDALONE_IO_CLOSE_WRITE_ARTIFACT の読み込みに失敗")
        } else {
            let standalone_wasm = compile_standalone_source(
                r#"(defn main []
  (let [bytes (vector-push
                (vector-push
                  (vector-push
                    (vector-push
                      (vector-push (vector-new 5) 0)
                      97)
                    115)
                  109)
                33)]
    (do
      (print (write-file "written.txt" "payload"))
      (print (write-file-bytes "raw.bin" bytes))
      0)))
"#,
                &dir,
                "LSHARP_STANDALONE_IO_CLOSE_WRITE_CLI_ARTIFACT",
            );
            if let Some(save_path) =
                std::env::var_os("LSHARP_STANDALONE_IO_CLOSE_WRITE_SAVE_ARTIFACT")
            {
                std::fs::write(save_path, &standalone_wasm)
                    .expect("close errno standalone artifact の保存に失敗");
            }
            standalone_wasm
        };

        let capture = run_with_partial_fd_write_with_close_errno(&standalone_wasm, &dir, 1)
            .expect("fd_close errno 下の standalone write 実行に失敗");
        let _ = std::fs::remove_dir_all(&dir);

        assert_eq!(capture.stdout, b"-1\n-1\n");
        assert_eq!(capture.file, b"payload\0asm!");
        assert_eq!(capture.fd_close_calls, 2);
    });
}

#[test]
fn test_e2e_selfhost_standalone_read_file_retries_partial_fd_read() {
    run_with_expanded_stack(NATIVE_HARNESS_STACK_BYTES, || {
        let dir = fixture_dir("partial_read");
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).expect("read fixture directory の作成に失敗");
        std::fs::write(dir.join("input.txt"), b"payload")
            .expect("read fixture input.txt の書き込みに失敗");
        let standalone_wasm = if let Some(artifact) =
            std::env::var_os("LSHARP_STANDALONE_IO_READ_ARTIFACT")
        {
            std::fs::read(artifact).expect("LSHARP_STANDALONE_IO_READ_ARTIFACT の読み込みに失敗")
        } else {
            let standalone_wasm = compile_standalone_source(
                r#"(defn main [] (print-string (read-file "input.txt")))"#,
                &dir,
                "LSHARP_STANDALONE_IO_READ_CLI_ARTIFACT",
            );
            if let Some(save_path) = std::env::var_os("LSHARP_STANDALONE_IO_READ_SAVE_ARTIFACT") {
                std::fs::write(save_path, &standalone_wasm)
                    .expect("read standalone artifact の保存に失敗");
            }
            standalone_wasm
        };

        let capture = run_with_partial_fd_read(&standalone_wasm, &dir)
            .expect("partial fd_read 下の standalone 実行に失敗");
        let _ = std::fs::remove_dir_all(&dir);

        assert_eq!(capture.stdout, b"payload");
        assert_eq!(capture.read_payload, b"payload");
        assert_eq!(capture.fd_read_calls, 3);
    });
}

#[test]
fn test_e2e_selfhost_standalone_read_file_returns_all_bytes_at_4096() {
    run_with_expanded_stack(NATIVE_HARNESS_STACK_BYTES, || {
        let dir = fixture_dir("read_at_4096");
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).expect("4096-byte read fixture directory の作成に失敗");
        let mut payload = vec![b'a'; 4095];
        payload.push(b'b');
        std::fs::write(dir.join("input.txt"), &payload)
            .expect("4096-byte read fixture input.txt の書き込みに失敗");

        let standalone_wasm = compile_standalone_source(
            r#"(defn main [] (print-string (read-file "input.txt")))"#,
            &dir,
            "LSHARP_STANDALONE_IO_LARGE_READ_CLI_ARTIFACT",
        );
        let output = lsharp_wasm::wasi_runner::run_wasm_wasi_with_dir_args_and_stdin_capture(
            &standalone_wasm,
            Some(&dir),
            &[],
            "",
        )
        .expect("4096-byte read の standalone 実行に失敗");
        let _ = std::fs::remove_dir_all(&dir);

        assert_eq!(output.exit_code, 0);
        assert_eq!(output.stdout.as_bytes(), payload.as_slice());
    });
}

#[test]
fn test_e2e_selfhost_standalone_read_file_returns_all_bytes_over_4096() {
    run_with_expanded_stack(NATIVE_HARNESS_STACK_BYTES, || {
        let dir = fixture_dir("read_over_4096");
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).expect("4097-byte read fixture directory の作成に失敗");
        let mut payload = vec![b'a'; 4096];
        payload.push(b'b');
        std::fs::write(dir.join("input.txt"), &payload)
            .expect("4097-byte read fixture input.txt の書き込みに失敗");

        let standalone_wasm = compile_standalone_source(
            r#"(defn main [] (print-string (read-file "input.txt")))"#,
            &dir,
            "LSHARP_STANDALONE_IO_READ_OVER_4096_CLI_ARTIFACT",
        );
        let output = lsharp_wasm::wasi_runner::run_wasm_wasi_with_dir_args_and_stdin_capture(
            &standalone_wasm,
            Some(&dir),
            &[],
            "",
        )
        .expect("4097-byte read の standalone 実行に失敗");
        let _ = std::fs::remove_dir_all(&dir);

        assert_eq!(output.exit_code, 0);
        assert_eq!(output.stdout.as_bytes(), payload.as_slice());
    });
}

#[test]
fn test_e2e_selfhost_standalone_read_file_returns_fd_read_errno_after_partial_read() {
    run_with_expanded_stack(NATIVE_HARNESS_STACK_BYTES, || {
        let dir = fixture_dir("fd_read_errno");
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).expect("read fd_read errno fixture directory の作成に失敗");
        std::fs::write(dir.join("input.txt"), b"payload")
            .expect("read fd_read errno input.txt の書き込みに失敗");
        let standalone_wasm =
            if let Some(artifact) = std::env::var_os("LSHARP_STANDALONE_IO_READ_ERRNO_ARTIFACT") {
                std::fs::read(artifact)
                    .expect("LSHARP_STANDALONE_IO_READ_ERRNO_ARTIFACT の読み込みに失敗")
            } else {
                compile_standalone_source(
                    r#"(defn main [] (print-string (read-file "input.txt")))"#,
                    &dir,
                    "LSHARP_STANDALONE_IO_READ_ERRNO_CLI_ARTIFACT",
                )
            };

        let capture = run_with_partial_fd_read_with_fd_read_errno(&standalone_wasm, &dir, 1)
            .expect("partial fd_read errno 下の standalone 実行に失敗");
        let _ = std::fs::remove_dir_all(&dir);

        assert_eq!(capture.stdout, b"");
        assert_eq!(capture.read_payload, b"pa");
        assert_eq!(capture.fd_read_calls, 1);
        assert_eq!(capture.fd_close_calls, 1);
    });
}

#[test]
fn test_e2e_selfhost_standalone_read_file_returns_fd_close_errno() {
    run_with_expanded_stack(NATIVE_HARNESS_STACK_BYTES, || {
        let dir = fixture_dir("close_errno_read");
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).expect("read close errno fixture directory の作成に失敗");
        std::fs::write(dir.join("input.txt"), b"payload")
            .expect("read close errno input.txt の書き込みに失敗");
        let standalone_wasm =
            if let Some(artifact) = std::env::var_os("LSHARP_STANDALONE_IO_CLOSE_READ_ARTIFACT") {
                std::fs::read(artifact)
                    .expect("LSHARP_STANDALONE_IO_CLOSE_READ_ARTIFACT の読み込みに失敗")
            } else {
                let standalone_wasm = compile_standalone_source(
                    r#"(defn main [] (print-string (read-file "input.txt")))"#,
                    &dir,
                    "LSHARP_STANDALONE_IO_CLOSE_READ_CLI_ARTIFACT",
                );
                if let Some(save_path) =
                    std::env::var_os("LSHARP_STANDALONE_IO_CLOSE_READ_SAVE_ARTIFACT")
                {
                    std::fs::write(save_path, &standalone_wasm)
                        .expect("close errno read standalone artifact の保存に失敗");
                }
                standalone_wasm
            };

        let capture = run_with_partial_fd_read_with_close_errno(&standalone_wasm, &dir, 1)
            .expect("fd_close errno 下の standalone read 実行に失敗");
        let _ = std::fs::remove_dir_all(&dir);

        assert_eq!(capture.stdout, b"");
        assert_eq!(capture.read_payload, b"payload");
        assert_eq!(capture.fd_read_calls, 3);
        assert_eq!(capture.fd_close_calls, 1);
    });
}

#[test]
fn test_e2e_selfhost_standalone_file_exists_returns_false_on_fd_close_errno() {
    run_with_expanded_stack(NATIVE_HARNESS_STACK_BYTES, || {
        let dir = fixture_dir("file_exists_close_errno");
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir)
            .expect("file-exists close errno fixture directory の作成に失敗");
        std::fs::write(dir.join("exists.txt"), b"payload")
            .expect("file-exists close errno fixture の作成に失敗");

        let standalone_wasm = compile_standalone_source(
            r#"(defn main [] (if (file-exists? "exists.txt") (print-string "true") (print-string "false")))"#,
            &dir,
            "LSHARP_STANDALONE_IO_FILE_EXISTS_CLOSE_CLI_ARTIFACT",
        );
        let capture = run_with_partial_fd_write_with_close_errno(&standalone_wasm, &dir, 1)
            .expect("fd_close errno 下の file-exists standalone 実行に失敗");
        let _ = std::fs::remove_dir_all(&dir);

        assert_eq!(capture.stdout, b"false");
        assert_eq!(capture.fd_close_calls, 1);
    });
}

#[test]
fn test_e2e_selfhost_standalone_read_file_returns_empty_on_path_open_errno() {
    run_with_expanded_stack(NATIVE_HARNESS_STACK_BYTES, || {
        let dir = fixture_dir("path_open_errno");
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).expect("read path_open errno fixture directory の作成に失敗");
        std::fs::write(dir.join("input.txt"), b"payload")
            .expect("read path_open errno input.txt の書き込みに失敗");

        let standalone_wasm = compile_standalone_source(
            r#"(defn main [] (print-string (read-file "input.txt")))"#,
            &dir,
            "LSHARP_STANDALONE_IO_FILE_EXISTS_CLOSE_CLI_ARTIFACT",
        );
        let capture = run_with_partial_fd_read_with_path_open_errno(&standalone_wasm, &dir, 1)
            .expect("path_open errno 下の standalone read 実行に失敗");
        let _ = std::fs::remove_dir_all(&dir);

        assert_eq!(capture.stdout, b"");
        assert_eq!(capture.path_open_calls, 1);
        assert_eq!(capture.fd_read_calls, 0);
        assert_eq!(capture.fd_close_calls, 0);
    });
}
