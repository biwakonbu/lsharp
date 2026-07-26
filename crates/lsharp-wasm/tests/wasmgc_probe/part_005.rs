#[test]
fn wasm_gc_runner_rejects_non_print_string_import_without_wasi_fallback() {
    let bytes = wat::parse_str(
        r#"
        (module
          (import "env" "unsupported" (func))
          (func (export "main") (result i64)
            i64.const 0))
        "#,
    )
    .expect("unsupported import module を生成できる");
    let error =
        lsharp_wasm::wasmgc_runner::run_wasm_wasmgc_with_stdout_sink(&bytes, |_bytes| Ok(()))
            .expect_err("unsupported import は WASI fallback せず拒否する");

    assert!(error.contains("未対応"), "{error}");
}

#[test]
fn wasm_gc_runner_write_adapter_retries_partial_writes_until_chunk_is_consumed() {
    let bytes = emit_print_string_probe_module(&[195, 169], 0);
    let output = Arc::new(Mutex::new(Vec::new()));
    let writer = OneByteWriter {
        output: Arc::clone(&output),
    };
    let exit_code = lsharp_wasm::wasmgc_runner::run_wasm_wasmgc_to_writer(&bytes, writer)
        .expect("partial writer adapter が chunk 全体を書き切れる");

    assert_eq!(exit_code, 0);
    assert_eq!(*output.lock().unwrap(), vec![195, 169]);
}

#[test]
fn wasm_gc_runner_write_adapter_propagates_write_error() {
    let bytes = emit_print_string_probe_module(&[65], 0);
    let error = lsharp_wasm::wasmgc_runner::run_wasm_wasmgc_to_writer(&bytes, FailingWriter)
        .expect_err("writer error は runner error になる");

    assert!(error.contains("stdout closed"), "{error}");
}

#[test]
fn wasm_gc_runner_write_adapter_rejects_write_zero() {
    let bytes = emit_print_string_probe_module(&[65], 0);
    let error = lsharp_wasm::wasmgc_runner::run_wasm_wasmgc_to_writer(&bytes, ZeroWriter)
        .expect_err("WriteZero は runner error になる");

    assert!(error.contains("failed"), "{error}");
}

#[test]
fn wasm_gc_runner_write_adapter_propagates_flush_error_after_execution() {
    let bytes = emit_print_string_probe_module(&[65], 0);
    let error = lsharp_wasm::wasmgc_runner::run_wasm_wasmgc_to_writer(&bytes, FlushFailingWriter)
        .expect_err("flush error は runner error になる");

    assert!(error.contains("flush failed"), "{error}");
}

#[test]
fn wasm_gc_component_output_writer_retries_partial_writes_until_chunk_is_consumed() {
    let bytes = emit_component_output_probe_module(&[195, 169], 13);
    let output = Arc::new(Mutex::new(Vec::new()));
    let writer = OneByteWriter {
        output: Arc::clone(&output),
    };
    let exit_code =
        lsharp_wasm::wasmgc_runner::run_wasm_wasmgc_component_output_to_writer(&bytes, writer)
            .expect("component output writer adapter が chunk 全体を書き切れる");

    assert_eq!(exit_code, 13);
    assert_eq!(*output.lock().unwrap(), vec![195, 169]);
}

#[test]
fn wasm_gc_component_output_writer_propagates_write_error() {
    let bytes = emit_component_output_probe_module(&[65], 0);
    let error = lsharp_wasm::wasmgc_runner::run_wasm_wasmgc_component_output_to_writer(
        &bytes,
        FailingWriter,
    )
    .expect_err("component output writer error は runner error になる");

    assert!(error.contains("stdout closed"), "{error}");
}

#[test]
fn wasm_gc_component_output_writer_rejects_write_zero() {
    let bytes = emit_component_output_probe_module(&[65], 0);
    let error =
        lsharp_wasm::wasmgc_runner::run_wasm_wasmgc_component_output_to_writer(&bytes, ZeroWriter)
            .expect_err("component output WriteZero は runner error になる");

    assert!(error.contains("failed"), "{error}");
}

#[test]
fn wasm_gc_component_output_writer_propagates_flush_error_after_execution() {
    let bytes = emit_component_output_probe_module(&[65], 0);
    let error = lsharp_wasm::wasmgc_runner::run_wasm_wasmgc_component_output_to_writer(
        &bytes,
        FlushFailingWriter,
    )
    .expect_err("component output flush error は runner error になる");

    assert!(error.contains("flush failed"), "{error}");
}

#[test]
fn wasm_gc_component_output_writer_flushes_after_nonzero_exit() {
    let bytes = emit_component_output_probe_module(&[65], 7);
    let events = Arc::new(Mutex::new(Vec::new()));
    let exit_code = lsharp_wasm::wasmgc_runner::run_wasm_wasmgc_component_output_to_writer(
        &bytes,
        EventWriter {
            events: Arc::clone(&events),
        },
    )
    .expect("nonzero exit 後も component output writer を flush できる");

    assert_eq!(exit_code, 7);
    assert_eq!(*events.lock().unwrap(), vec!["write", "flush"]);
}

#[test]
fn wasm_gc_component_output_fd_write_retries_partial_writes() {
    let bytes = emit_component_output_probe_module(&[195, 169], 17);
    let output = Arc::new(Mutex::new(Vec::new()));
    let calls = Arc::new(Mutex::new(Vec::<(u32, usize)>::new()));
    let output_for_write = Arc::clone(&output);
    let calls_for_write = Arc::clone(&calls);
    let exit_code = lsharp_wasm::wasmgc_runner::run_wasm_wasmgc_component_output_to_fd_write(
        &bytes,
        1,
        move |fd, chunk| {
            calls_for_write.lock().unwrap().push((fd, chunk.len()));
            if let Some(byte) = chunk.first() {
                output_for_write.lock().unwrap().push(*byte);
                Ok(1)
            } else {
                Ok(0)
            }
        },
    )
    .expect("component output fd_write adapter が partial write を再試行できる");

    assert_eq!(exit_code, 17);
    assert_eq!(*output.lock().unwrap(), vec![195, 169]);
    assert_eq!(*calls.lock().unwrap(), vec![(1, 2), (1, 1)]);
}

#[test]
fn wasm_gc_component_output_fd_write_propagates_errno() {
    let bytes = emit_component_output_probe_module(&[65], 0);
    let error = lsharp_wasm::wasmgc_runner::run_wasm_wasmgc_component_output_to_fd_write(
        &bytes,
        1,
        |_fd, _chunk| Err(28),
    )
    .expect_err("component output fd_write errno は runner error になる");

    assert!(error.contains("28"), "{error}");
}

#[test]
fn wasm_gc_component_output_fd_write_rejects_zero_and_overreported_counts() {
    let bytes = emit_component_output_probe_module(&[65], 0);
    let zero_error = lsharp_wasm::wasmgc_runner::run_wasm_wasmgc_component_output_to_fd_write(
        &bytes,
        1,
        |_fd, _chunk| Ok(0),
    )
    .expect_err("component output fd_write zero は拒否する");
    assert!(zero_error.contains("failed"), "{zero_error}");

    let overreported_error =
        lsharp_wasm::wasmgc_runner::run_wasm_wasmgc_component_output_to_fd_write(
            &bytes,
            1,
            |_fd, chunk| Ok(chunk.len() + 1),
        )
        .expect_err("component output fd_write over-report は拒否する");
    assert!(
        overreported_error.contains("over-reported"),
        "{overreported_error}"
    );
}

struct OneByteWriter {
    output: Arc<Mutex<Vec<u8>>>,
}

impl Write for OneByteWriter {
    fn write(&mut self, bytes: &[u8]) -> io::Result<usize> {
        let count = usize::from(!bytes.is_empty());
        if count != 0 {
            self.output.lock().unwrap().push(bytes[0]);
        }
        Ok(count)
    }

    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}

struct FailingWriter;

impl Write for FailingWriter {
    fn write(&mut self, _bytes: &[u8]) -> io::Result<usize> {
        Err(io::Error::new(io::ErrorKind::BrokenPipe, "stdout closed"))
    }

    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}

struct ZeroWriter;

impl Write for ZeroWriter {
    fn write(&mut self, _bytes: &[u8]) -> io::Result<usize> {
        Ok(0)
    }

    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}

struct FlushFailingWriter;

impl Write for FlushFailingWriter {
    fn write(&mut self, bytes: &[u8]) -> io::Result<usize> {
        Ok(bytes.len())
    }

    fn flush(&mut self) -> io::Result<()> {
        Err(io::Error::other("flush closed"))
    }
}

fn persist_and_reload_wasmgc_component_artifact(component: &[u8]) -> Result<Vec<u8>, String> {
    let nonce = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_err(|error| format!("artifact nonce を取得できない: {error}"))?
        .as_nanos();
    let dir = std::env::temp_dir().join(format!(
        "lsharp_wasmgc_component_artifact_{}_{}",
        std::process::id(),
        nonce
    ));
    std::fs::create_dir_all(&dir)
        .map_err(|error| format!("Component artifact の一時ディレクトリを作成できない: {error}"))?;
    let path = dir.join("output.component.wasm");
    let result = (|| {
        lsharp_wasm::component_adapter::write_component_artifact(&path, component)
            .map_err(|error| format!("Component artifact を保存できない: {error}"))?;
        lsharp_wasm::component_adapter::read_component_artifact(&path)
            .map_err(|error| format!("Component artifact を再読込できない: {error}"))
    })();
    let cleanup = std::fs::remove_dir_all(&dir);
    match (result, cleanup) {
        (Ok(bytes), Ok(())) => Ok(bytes),
        (Err(error), Ok(())) => Err(error),
        (Ok(_), Err(error)) => Err(format!(
            "Component artifact は再読込できたが一時ディレクトリを削除できない: {error}"
        )),
        (Err(error), Err(cleanup_error)) => Err(format!(
            "{error}; 一時ディレクトリの削除にも失敗した: {cleanup_error}"
        )),
    }
}

struct EventWriter {
    events: Arc<Mutex<Vec<&'static str>>>,
}

impl Write for EventWriter {
    fn write(&mut self, bytes: &[u8]) -> io::Result<usize> {
        self.events.lock().unwrap().push("write");
        Ok(bytes.len())
    }

    fn flush(&mut self) -> io::Result<()> {
        self.events.lock().unwrap().push("flush");
        Ok(())
    }
}

fn emit_print_string_probe_module(bytes: &[i32], exit_code: i64) -> Vec<u8> {
    let mut body = bytes
        .iter()
        .copied()
        .map(Instruction::I32Const)
        .collect::<Vec<_>>();
    body.push(Instruction::ArrayNewFixed(0, bytes.len() as u32));
    body.push(Instruction::Call(4));
    body.push(Instruction::I64Const(exit_code));
    let module = IrModule {
        functions: vec![Function {
            name: "main".to_string(),
            params: vec![],
            result: IrType::I64,
            locals: vec![],
            body,
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
    lsharp_wasm::wasmgc::emit_wasm_wasmgc(&module).expect("writer adapter module を生成できる")
}

fn emit_component_output_probe_module(bytes: &[i32], exit_code: i64) -> Vec<u8> {
    let mut body = bytes
        .iter()
        .copied()
        .map(Instruction::I32Const)
        .collect::<Vec<_>>();
    body.push(Instruction::ArrayNewFixed(0, bytes.len() as u32));
    body.push(Instruction::Call(4));
    body.push(Instruction::I64Const(exit_code));
    let module = IrModule {
        functions: vec![Function {
            name: "main".to_string(),
            params: vec![],
            result: IrType::I64,
            locals: vec![],
            body,
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
    lsharp_wasm::wasmgc::emit_wasm_wasmgc_component_output(&module)
        .expect("component output writer adapter module を生成できる")
}

fn emit_component_output_cli_run_probe_module() -> Vec<u8> {
    emit_component_output_cli_run_probe_module_with_result(0)
}

fn emit_component_output_cli_run_probe_module_with_result(result: i32) -> Vec<u8> {
    wat::parse_str(format!(
        r#"
(module
  (type (func (param i32 i32)))
  (type (func (result i32)))
  (import "lsharp:wasmgc-output/stdout@0.1.0" "write" (func $write (type 0)))
  (memory (export "memory") 1)
  (func (export "wasi:cli/run@0.2.3#run") (type 1)
    i32.const {result})
)
"#
    ))
    .expect("canonical wasi:cli/run probe module を生成できる")
}

fn emit_component_output_cli_exit_probe_module(exit_code: i32) -> Vec<u8> {
    wat::parse_str(format!(
        r#"
(module
  (type (func (param i32 i32)))
  (type (func (result i32)))
  (import "lsharp:wasmgc-output/stdout@0.1.0" "write" (func $write (type 0)))
  (import "wasi:cli/exit@0.2.3" "exit" (func $exit (param i32)))
  (memory (export "memory") 1)
  (func (export "wasi:cli/run@0.2.3#run") (type 1)
    i32.const {exit_code}
    call $exit
    i32.const 0)
)
"#
    ))
    .expect("wasi:cli/exit probe module を生成できる")
}

fn emit_component_cli_preopen_write_probe_module() -> Vec<u8> {
    wat::parse_str(
        r#"
(module
  (type (func (param i32 i32)))
  (type (func (param i32)))
  (type (func (result i32)))
  (type (func (param i32 i32 i32 i32 i32 i32 i32)))
  (import "lsharp:wasmgc-output/stdout@0.1.0" "write" (func $write (type 0)))
  (import "wasi:filesystem/preopens@0.2.3" "get-directories" (func $get-directories (type 1)))
  (import "wasi:filesystem/types@0.2.3" "[method]descriptor.open-at" (func $open-at (type 3)))
  (memory (export "memory") 2)
  (global $heap (mut i32) (i32.const 1024))
  (func (export "cabi_realloc")
    (param $old i32) (param $old-len i32) (param $align i32) (param $new-len i32)
    (result i32)
    (local $mask i32)
    (local $ptr i32)
    local.get $align
    i32.const 1
    i32.sub
    local.set $mask
    global.get $heap
    local.get $mask
    i32.add
    local.get $mask
    i32.const -1
    i32.xor
    i32.and
    local.set $ptr
    local.get $ptr
    local.get $new-len
    i32.add
    global.set $heap
    local.get $ptr)
  (data (i32.const 128) "rights.txt")
  (func (export "wasi:cli/run@0.2.3#run") (type 2)
    i32.const 16
    call $get-directories
    i32.const 20
    i32.load
    i32.eqz
    if (result i32)
      i32.const 1
    else
      i32.const 16
      i32.load
      i32.load
      i32.const 0
      i32.const 128
      i32.const 10
      i32.const 1
      i32.const 2
      i32.const 32
      call $open-at
      i32.const 32
      i32.load
    end)
)
"#,
    )
    .expect("preopen rights probe module を生成できる")
}

fn emit_component_cli_named_preopen_stream_probe_module() -> Vec<u8> {
    wat::parse_str(
        r#"
(module
  (type (func (param i32 i32)))
  (type (func (param i32)))
  (type (func (result i32)))
  (type (func (param i32 i32 i32 i32 i32 i32 i32)))
  (type (func (param i32 i64 i32)))
  (import "lsharp:wasmgc-output/stdout@0.1.0" "write" (func $write (type 0)))
  (import "wasi:filesystem/preopens@0.2.3" "get-directories" (func $get-directories (type 1)))
  (import "wasi:filesystem/types@0.2.3" "[method]descriptor.open-at" (func $open-at (type 3)))
  (import "wasi:filesystem/types@0.2.3" "[method]descriptor.read-via-stream" (func $read-via-stream (type 4)))
  (import "wasi:io/streams@0.2.3" "[method]input-stream.blocking-read" (func $blocking-read (param i32 i64 i32)))
  (import "wasi:io/streams@0.2.3" "[resource-drop]input-stream" (func $drop-input-stream (param i32)))
  (import "wasi:filesystem/types@0.2.3" "[resource-drop]descriptor" (func $drop-descriptor (param i32)))
  (memory (export "memory") 2)
  (global $heap (mut i32) (i32.const 1024))
  (func (export "cabi_realloc")
    (param $old i32) (param $old-len i32) (param $align i32) (param $new-len i32)
    (result i32)
    (local $mask i32)
    (local $ptr i32)
    local.get $align
    i32.const 1
    i32.sub
    local.set $mask
    global.get $heap
    local.get $mask
    i32.add
    local.get $mask
    i32.const -1
    i32.xor
    i32.and
    local.set $ptr
    local.get $ptr
    local.get $new-len
    i32.add
    global.set $heap
    local.get $ptr)
  (data (i32.const 128) "input.txt")
  (func (export "wasi:cli/run@0.2.3#run") (type 2)
    (local $preopen i32)
    (local $descriptor i32)
    (local $stream i32)
    i32.const 16
    call $get-directories
    i32.const 20
    i32.load
    i32.const 2
    i32.ne
    if (result i32)
      i32.const 1
    else
      i32.const 16
      i32.load
      i32.load
      local.set $preopen
      local.get $preopen
      i32.const 0
      i32.const 128
      i32.const 9
      i32.const 0
      i32.const 1
      i32.const 32
      call $open-at
      i32.const 32
      i32.load8_u
      if
        local.get $preopen
        call $drop-descriptor
        i32.const 2
        return
      end
      local.get $preopen
      call $drop-descriptor
      i32.const 36
      i32.load
      local.set $descriptor
      local.get $descriptor
      i64.const 0
      i32.const 40
      call $read-via-stream
      i32.const 40
      i32.load8_u
      if
        i32.const 3
        return
      end
      i32.const 44
      i32.load
      local.set $stream
      local.get $stream
      i64.const 5
      i32.const 48
      call $blocking-read
      i32.const 48
      i32.load8_u
      if
        i32.const 4
        return
      end
      i32.const 52
      i32.load
      i32.const 56
      i32.load
      call $write
      local.get $stream
      call $drop-input-stream
      local.get $descriptor
      call $drop-descriptor
      i32.const 0
      return
    end)
)
"#,
    )
    .expect("named preopen stream probe module を生成できる")
}

fn emit_component_cli_direct_read_probe_module() -> Vec<u8> {
    wat::parse_str(
        r#"
(module
  (type (func (param i32 i32)))
  (type (func (param i32)))
  (type (func (result i32)))
  (type (func (param i32 i32 i32 i32 i32 i32 i32)))
  (type (func (param i32 i64 i64 i32)))
  (import "lsharp:wasmgc-output/stdout@0.1.0" "write" (func $write (type 0)))
  (import "wasi:filesystem/preopens@0.2.3" "get-directories" (func $get-directories (type 1)))
  (import "wasi:filesystem/types@0.2.3" "[method]descriptor.open-at" (func $open-at (type 3)))
  (import "wasi:filesystem/types@0.2.3" "[method]descriptor.read" (func $read (type 4)))
  (import "wasi:filesystem/types@0.2.3" "[resource-drop]descriptor" (func $drop-descriptor (param i32)))
  (memory (export "memory") 2)
  (global $heap (mut i32) (i32.const 1024))
  (func (export "cabi_realloc")
    (param $old i32) (param $old-len i32) (param $align i32) (param $new-len i32)
    (result i32)
    (local $mask i32)
    (local $ptr i32)
    local.get $align
    i32.const 1
    i32.sub
    local.set $mask
    global.get $heap
    local.get $mask
    i32.add
    local.get $mask
    i32.const -1
    i32.xor
    i32.and
    local.set $ptr
    local.get $ptr
    local.get $new-len
    i32.add
    global.set $heap
    local.get $ptr)
  (data (i32.const 128) "input.txt")
  (func (export "wasi:cli/run@0.2.3#run") (type 2)
    (local $preopen i32)
    (local $descriptor i32)
    i32.const 16
    call $get-directories
    i32.const 20
    i32.load
    i32.const 2
    i32.ne
    if (result i32)
      i32.const 1
    else
      i32.const 16
      i32.load
      i32.load
      local.set $preopen
      local.get $preopen
      i32.const 0
      i32.const 128
      i32.const 9
      i32.const 0
      i32.const 1
      i32.const 32
      call $open-at
      i32.const 32
      i32.load8_u
      if
        local.get $preopen
        call $drop-descriptor
        i32.const 2
        return
      end
      local.get $preopen
      call $drop-descriptor
      i32.const 36
      i32.load
      local.set $descriptor
      local.get $descriptor
      i64.const 5
      i64.const 0
      i32.const 40
      call $read
      i32.const 40
      i32.load8_u
      if
        local.get $descriptor
        call $drop-descriptor
        i32.const 1
        return
      end
      i32.const 48
      i32.load
      i32.const 5
      i32.ne
      if
        local.get $descriptor
        call $drop-descriptor
        i32.const 1
        return
      end
      i32.const 52
      i32.load8_u
      if
        local.get $descriptor
        call $drop-descriptor
        i32.const 1
        return
      end
      i32.const 44
      i32.load
      i32.const 48
      i32.load
      call $write
      local.get $descriptor
      i64.const 1
      i64.const 5
      i32.const 40
      call $read
      i32.const 40
      i32.load8_u
      if
        local.get $descriptor
        call $drop-descriptor
        i32.const 1
        return
      end
      i32.const 48
      i32.load
      i32.const 0
      i32.ne
      if
        local.get $descriptor
        call $drop-descriptor
        i32.const 1
        return
      end
      i32.const 52
      i32.load8_u
      i32.eqz
      if
        local.get $descriptor
        call $drop-descriptor
        i32.const 1
        return
      end
      local.get $descriptor
      call $drop-descriptor
      i32.const 0
      return
    end)
)
"#,
    )
    .expect("descriptor direct read probe module を生成できる")
}
