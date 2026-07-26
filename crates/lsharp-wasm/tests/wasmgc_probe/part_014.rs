fn emit_component_cli_pending_output_stream_failure_probe_module() -> Vec<u8> {
    wat::parse_str(
        r#"
(module
  (type (func (param i32 i32)))
  (type (func (param i32)))
  (type (func (result i32)))
  (type (func (param i32 i32 i32 i32 i32 i32 i32)))
  (type (func (param i32 i64 i32)))
  (type (func (param i32 i32)))
  (type (func (param i32 i32 i32 i32)))
  (type (func (param i32) (result i32)))
  (import "lsharp:wasmgc-output/stdout@0.1.0" "write" (func $write-stdout (type 0)))
  (import "wasi:filesystem/preopens@0.2.3" "get-directories" (func $get-directories (type 1)))
  (import "wasi:filesystem/types@0.2.3" "[method]descriptor.open-at" (func $open-at (type 3)))
  (import "wasi:filesystem/types@0.2.3" "[method]descriptor.write-via-stream" (func $write-via-stream (type 4)))
  (import "wasi:io/streams@0.2.3" "[method]output-stream.check-write" (func $check-write (type 5)))
  (import "wasi:io/streams@0.2.3" "[method]output-stream.write" (func $write (type 6)))
  (import "wasi:io/streams@0.2.3" "[method]output-stream.subscribe" (func $subscribe (type 7)))
  (import "wasi:io/poll@0.2.3" "[method]pollable.block" (func $block (type 1)))
  (import "wasi:filesystem/types@0.2.3" "filesystem-error-code" (func $filesystem-error-code (type 5)))
  (import "wasi:io/streams@0.2.3" "[resource-drop]output-stream" (func $drop-output-stream (param i32)))
  (import "wasi:io/poll@0.2.3" "[resource-drop]pollable" (func $drop-pollable (param i32)))
  (import "wasi:filesystem/types@0.2.3" "[resource-drop]descriptor" (func $drop-descriptor (param i32)))
  (import "wasi:io/error@0.2.3" "[resource-drop]error" (func $drop-error (param i32)))
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
  (data (i32.const 128) "output.txt")
  (data (i32.const 144) "x")
  (data (i32.const 160) "C")
  (func (export "wasi:cli/run@0.2.3#run") (type 2)
    (local $preopen i32)
    (local $descriptor i32)
    (local $stream i32)
    (local $pollable i32)
    (local $error i32)
    i32.const 16
    call $get-directories
    i32.const 20
    i32.load
    i32.const 1
    i32.ne
    if
      i32.const 1
      return
    end
    i32.const 16
    i32.load
    i32.load
    local.set $preopen
    local.get $preopen
    i32.const 0
    i32.const 128
    i32.const 10
    i32.const 5
    i32.const 2
    i32.const 32
    call $open-at
    i32.const 32
    i32.load8_u
    if
      local.get $preopen
      call $drop-descriptor
      i32.const 1
      return
    end
    i32.const 36
    i32.load
    local.set $descriptor
    local.get $descriptor
    i64.const -1
    i32.const 40
    call $write-via-stream
    i32.const 40
    i32.load8_u
    if
      local.get $descriptor
      call $drop-descriptor
      local.get $preopen
      call $drop-descriptor
      i32.const 1
      return
    end
    i32.const 44
    i32.load
    local.set $stream
    local.get $stream
    i32.const 48
    call $check-write
    i32.const 48
    i32.load8_u
    if
      local.get $stream
      call $drop-output-stream
      local.get $descriptor
      call $drop-descriptor
      local.get $preopen
      call $drop-descriptor
      i32.const 1
      return
    end
    i32.const 56
    i64.load
    i64.eqz
    if
      local.get $stream
      call $drop-output-stream
      local.get $descriptor
      call $drop-descriptor
      local.get $preopen
      call $drop-descriptor
      i32.const 1
      return
    end
    local.get $stream
    i32.const 144
    i32.const 1
    i32.const 64
    call $write
    i32.const 64
    i32.load8_u
    i32.eqz
    if
      nop
    else
      i32.const 68
      i32.load8_u
      i32.eqz
      if
        i32.const 72
        i32.load
        call $drop-error
      end
      local.get $stream
      call $drop-output-stream
      local.get $descriptor
      call $drop-descriptor
      local.get $preopen
      call $drop-descriptor
      i32.const 1
      return
    end
    local.get $stream
    call $subscribe
    local.set $pollable
    local.get $pollable
    call $block
    local.get $stream
    i32.const 80
    call $check-write
    i32.const 80
    i32.load8_u
    i32.eqz
    if
      local.get $pollable
      call $drop-pollable
      local.get $stream
      call $drop-output-stream
      local.get $descriptor
      call $drop-descriptor
      local.get $preopen
      call $drop-descriptor
      i32.const 1
      return
    end
    i32.const 88
    i32.load8_u
    i32.const 0
    i32.ne
    if
      local.get $pollable
      call $drop-pollable
      local.get $stream
      call $drop-output-stream
      local.get $descriptor
      call $drop-descriptor
      local.get $preopen
      call $drop-descriptor
      i32.const 1
      return
    end
    i32.const 92
    i32.load
    local.set $error
    local.get $error
    i32.const 96
    call $filesystem-error-code
    i32.const 96
    i32.load8_u
    i32.const 1
    i32.ne
    if
      local.get $error
      call $drop-error
      local.get $pollable
      call $drop-pollable
      local.get $stream
      call $drop-output-stream
      local.get $descriptor
      call $drop-descriptor
      local.get $preopen
      call $drop-descriptor
      i32.const 1
      return
    end
    i32.const 97
    i32.load8_u
    i32.const 12
    i32.ne
    if
      local.get $error
      call $drop-error
      local.get $pollable
      call $drop-pollable
      local.get $stream
      call $drop-output-stream
      local.get $descriptor
      call $drop-descriptor
      local.get $preopen
      call $drop-descriptor
      i32.const 1
      return
    end
    i32.const 160
    i32.const 1
    call $write-stdout
    local.get $error
    call $drop-error
    local.get $pollable
    call $drop-pollable
    local.get $stream
    call $drop-output-stream
    local.get $descriptor
    call $drop-descriptor
    local.get $preopen
    call $drop-descriptor
    i32.const 0)
)
"#,
    )
    .expect("pending output stream failure probe module を生成できる")
}

fn emit_component_cli_nonblocking_flush_pending_output_stream_failure_probe_module() -> Vec<u8> {
    wat::parse_str(
        r#"
(module
  (type (func (param i32 i32)))
  (type (func (param i32)))
  (type (func (result i32)))
  (type (func (param i32 i32 i32 i32 i32 i32 i32)))
  (type (func (param i32 i64 i32)))
  (type (func (param i32 i32)))
  (type (func (param i32 i32 i32 i32)))
  (type (func (param i32) (result i32)))
  (import "lsharp:wasmgc-output/stdout@0.1.0" "write" (func $write-stdout (type 0)))
  (import "wasi:filesystem/preopens@0.2.3" "get-directories" (func $get-directories (type 1)))
  (import "wasi:filesystem/types@0.2.3" "[method]descriptor.open-at" (func $open-at (type 3)))
  (import "wasi:filesystem/types@0.2.3" "[method]descriptor.write-via-stream" (func $write-via-stream (type 4)))
  (import "wasi:io/streams@0.2.3" "[method]output-stream.check-write" (func $check-write (type 5)))
  (import "wasi:io/streams@0.2.3" "[method]output-stream.write" (func $write (type 6)))
  (import "wasi:io/streams@0.2.3" "[method]output-stream.flush" (func $flush (type 5)))
  (import "wasi:io/streams@0.2.3" "[method]output-stream.subscribe" (func $subscribe (type 7)))
  (import "wasi:io/poll@0.2.3" "[method]pollable.block" (func $block (type 1)))
  (import "wasi:filesystem/types@0.2.3" "filesystem-error-code" (func $filesystem-error-code (type 5)))
  (import "wasi:io/streams@0.2.3" "[resource-drop]output-stream" (func $drop-output-stream (param i32)))
  (import "wasi:io/poll@0.2.3" "[resource-drop]pollable" (func $drop-pollable (param i32)))
  (import "wasi:filesystem/types@0.2.3" "[resource-drop]descriptor" (func $drop-descriptor (param i32)))
  (import "wasi:io/error@0.2.3" "[resource-drop]error" (func $drop-error (param i32)))
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
  (data (i32.const 128) "output.txt")
  (data (i32.const 144) "x")
  (data (i32.const 160) "F")
  (func (export "wasi:cli/run@0.2.3#run") (type 2)
    (local $preopen i32)
    (local $descriptor i32)
    (local $stream i32)
    (local $pollable i32)
    (local $error i32)
    i32.const 16
    call $get-directories
    i32.const 20
    i32.load
    i32.const 1
    i32.ne
    if
      i32.const 1
      return
    end
    i32.const 16
    i32.load
    i32.load
    local.set $preopen
    local.get $preopen
    i32.const 0
    i32.const 128
    i32.const 10
    i32.const 5
    i32.const 2
    i32.const 32
    call $open-at
    i32.const 32
    i32.load8_u
    if
      local.get $preopen
      call $drop-descriptor
      i32.const 1
      return
    end
    i32.const 36
    i32.load
    local.set $descriptor
    local.get $descriptor
    i64.const -1
    i32.const 40
    call $write-via-stream
    i32.const 40
    i32.load8_u
    if
      local.get $descriptor
      call $drop-descriptor
      local.get $preopen
      call $drop-descriptor
      i32.const 1
      return
    end
    i32.const 44
    i32.load
    local.set $stream
    local.get $stream
    i32.const 48
    call $check-write
    i32.const 48
    i32.load8_u
    if
      local.get $stream
      call $drop-output-stream
      local.get $descriptor
      call $drop-descriptor
      local.get $preopen
      call $drop-descriptor
      i32.const 1
      return
    end
    i32.const 56
    i64.load
    i64.eqz
    if
      local.get $stream
      call $drop-output-stream
      local.get $descriptor
      call $drop-descriptor
      local.get $preopen
      call $drop-descriptor
      i32.const 1
      return
    end
    local.get $stream
    i32.const 144
    i32.const 1
    i32.const 64
    call $write
    i32.const 64
    i32.load8_u
    i32.eqz
    if
      nop
    else
      i32.const 68
      i32.load8_u
      i32.eqz
      if
        i32.const 72
        i32.load
        call $drop-error
      end
      local.get $stream
      call $drop-output-stream
      local.get $descriptor
      call $drop-descriptor
      local.get $preopen
      call $drop-descriptor
      i32.const 1
      return
    end
    local.get $stream
    i32.const 72
    call $flush
    i32.const 72
    i32.load8_u
    i32.eqz
    if
      nop
    else
      i32.const 76
      i32.load8_u
      i32.eqz
      if
        i32.const 80
        i32.load
        call $drop-error
      end
      local.get $stream
      call $drop-output-stream
      local.get $descriptor
      call $drop-descriptor
      local.get $preopen
      call $drop-descriptor
      i32.const 1
      return
    end
    local.get $stream
    call $subscribe
    local.set $pollable
    local.get $pollable
    call $block
    local.get $stream
    i32.const 88
    call $check-write
    i32.const 88
    i32.load8_u
    i32.eqz
    if
      local.get $pollable
      call $drop-pollable
      local.get $stream
      call $drop-output-stream
      local.get $descriptor
      call $drop-descriptor
      local.get $preopen
      call $drop-descriptor
      i32.const 1
      return
    end
    i32.const 96
    i32.load8_u
    i32.const 0
    i32.ne
    if
      local.get $pollable
      call $drop-pollable
      local.get $stream
      call $drop-output-stream
      local.get $descriptor
      call $drop-descriptor
      local.get $preopen
      call $drop-descriptor
      i32.const 1
      return
    end
    i32.const 100
    i32.load
    local.set $error
    local.get $error
    i32.const 104
    call $filesystem-error-code
    i32.const 104
    i32.load8_u
    i32.const 1
    i32.ne
    if
      local.get $error
      call $drop-error
      local.get $pollable
      call $drop-pollable
      local.get $stream
      call $drop-output-stream
      local.get $descriptor
      call $drop-descriptor
      local.get $preopen
      call $drop-descriptor
      i32.const 1
      return
    end
    i32.const 105
    i32.load8_u
    i32.const 12
    i32.ne
    if
      local.get $error
      call $drop-error
      local.get $pollable
      call $drop-pollable
      local.get $stream
      call $drop-output-stream
      local.get $descriptor
      call $drop-descriptor
      local.get $preopen
      call $drop-descriptor
      i32.const 1
      return
    end
    i32.const 160
    i32.const 1
    call $write-stdout
    local.get $error
    call $drop-error
    local.get $pollable
    call $drop-pollable
    local.get $stream
    call $drop-output-stream
    local.get $descriptor
    call $drop-descriptor
    local.get $preopen
    call $drop-descriptor
    i32.const 0)
)
"#,
    )
    .expect("non-blocking flush pending output stream failure probe module を生成できる")
}

#[test]
fn wasm_gc_emitter_maps_reference_typed_struct_fields() {
    let module = IrModule {
        functions: vec![Function {
            name: "read-nested-field".to_string(),
            params: vec![],
            result: IrType::I64,
            locals: vec![],
            body: vec![
                Instruction::I64Const(42),
                Instruction::StructNew(0),
                Instruction::StructNew(1),
                Instruction::StructGet(1, 0),
                Instruction::StructGet(0, 0),
            ],
            is_export: true,
        }],
        gc_types: vec![
            GcTypeDef {
                name: "Point".to_string(),
                kind: GcTypeKind::Struct(vec![GcField {
                    name: "value".to_string(),
                    ty: IrType::I64,
                    mutable: false,
                }]),
            },
            GcTypeDef {
                name: "Box".to_string(),
                kind: GcTypeKind::Struct(vec![GcField {
                    name: "point".to_string(),
                    ty: IrType::Ref(0),
                    mutable: false,
                }]),
            },
        ],
        imports: vec![],
        globals: vec![],
        string_data: vec![],
    };
    let bytes = lsharp_wasm::wasmgc::emit_wasm_wasmgc(&module)
        .expect("reference typed field を含む IR module を生成できる");

    let mut config = Config::new();
    config.wasm_gc(true);
    let engine = Engine::new(&config).expect("WasmGC engine を作成できる");
    let module = Module::new(&engine, bytes).expect("nested struct module を検証できる");
    let mut store = Store::new(&engine, ());
    let instance =
        Instance::new(&mut store, &module, &[]).expect("IR module を instantiate できる");
    let read_nested_field = instance
        .get_typed_func::<(), i64>(&mut store, "read-nested-field")
        .expect("read-nested-field export が存在する");

    assert_eq!(read_nested_field.call(&mut store, ()).unwrap(), 42);
}

#[test]
fn wasm_gc_emitter_remaps_lowered_user_call_indices() {
    let module = IrModule {
        functions: vec![
            Function {
                name: "callee".to_string(),
                params: vec![],
                result: IrType::I64,
                locals: vec![],
                body: vec![Instruction::I64Const(7)],
                is_export: false,
            },
            Function {
                name: "main".to_string(),
                params: vec![],
                result: IrType::I64,
                locals: vec![],
                // Lower は runtime import 17 個の後ろを user function index として持つ。
                body: vec![Instruction::Call(17)],
                is_export: true,
            },
        ],
        gc_types: vec![],
        imports: vec![],
        globals: vec![],
        string_data: vec![],
    };

    let bytes = lsharp_wasm::wasmgc::emit_wasm_wasmgc(&module)
        .expect("lowered user call index を core Wasm index へ変換できる");
    let mut config = Config::new();
    config.wasm_gc(true);
    let engine = Engine::new(&config).expect("WasmGC engine を作成できる");
    let module = Module::new(&engine, bytes).expect("user call を含む module を検証できる");
    let mut store = Store::new(&engine, ());
    let instance = Instance::new(&mut store, &module, &[]).expect("module を instantiate できる");
    let main = instance
        .get_typed_func::<(), i64>(&mut store, "main")
        .expect("main export が存在する");

    assert_eq!(main.call(&mut store, ()).unwrap(), 7);
}

#[test]
fn wasm_gc_emitter_executes_typed_funcref_call_ref() {
    let module = IrModule {
        functions: vec![
            Function {
                name: "identity".to_string(),
                params: vec![IrType::I64],
                result: IrType::I64,
                locals: vec![],
                body: vec![Instruction::LocalGet(0)],
                is_export: false,
            },
            Function {
                name: "main".to_string(),
                params: vec![],
                result: IrType::I64,
                locals: vec![],
                body: vec![
                    Instruction::I64Const(41),
                    Instruction::RefFunc(0),
                    Instruction::CallRef(0),
                ],
                is_export: true,
            },
        ],
        gc_types: vec![],
        imports: vec![],
        globals: vec![],
        string_data: vec![],
    };

    let bytes = lsharp_wasm::wasmgc::emit_wasm_wasmgc(&module)
        .expect("typed funcref と call_ref を含む WasmGC module を生成できる");
    let mut config = Config::new();
    config.wasm_gc(true);
    config.wasm_reference_types(true);
    config.wasm_function_references(true);
    let engine = Engine::new(&config).expect("WasmGC engine を作成できる");
    let module = Module::new(&engine, bytes).expect("typed funcref module を検証できる");
    let mut store = Store::new(&engine, ());
    let instance = Instance::new(&mut store, &module, &[]).expect("module を instantiate できる");
    let main = instance
        .get_typed_func::<(), i64>(&mut store, "main")
        .expect("main export が存在する");

    assert_eq!(main.call(&mut store, ()).unwrap(), 41);
}

#[test]
fn wasm_gc_emitter_accepts_lowered_non_capturing_lambda_funcref() {
    let program = lsharp_syntax::parse(
        r#"
        (defn make-inc [] (fn [x] (+ x 1)))
        (defn main [] 0)
        "#,
    )
    .expect("non-capturing lambda source を parse できる");
    let mut infer = lsharp_types::infer::Infer::new();
    let type_results = infer
        .infer_program(&program)
        .expect("non-capturing lambda source を型推論できる");
    let expr_type_results = infer.expr_type_results_snapshot();
    let mut lowerer = lsharp_ir::lower::Lower::with_backend(lsharp_ir::lower::LowerBackend::WasmGc);
    let ir = lowerer
        .lower_program_with_expr_types(&program, &type_results, &expr_type_results)
        .expect("non-capturing lambda を WasmGC funcref IR へ lowering できる");
    let make_inc = ir
        .functions
        .iter()
        .find(|function| function.name == "make-inc")
        .expect("make-inc が存在する");
    assert_eq!(make_inc.result, IrType::FuncRef);
    assert!(
        make_inc
            .body
            .iter()
            .any(|instruction| matches!(instruction, Instruction::RefFunc(_)))
    );

    let bytes = lsharp_wasm::wasmgc::emit_wasm_wasmgc(&ir)
        .expect("lowered non-capturing lambda module を生成できる");
    let mut config = Config::new();
    config.wasm_gc(true);
    config.wasm_reference_types(true);
    config.wasm_function_references(true);
    let engine = Engine::new(&config).expect("typed funcref を有効化した engine を作成できる");
    Module::new(&engine, bytes).expect("lowered non-capturing lambda module を検証できる");
}
