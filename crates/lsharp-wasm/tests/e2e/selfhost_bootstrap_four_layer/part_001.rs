
/// stage1 が stdout に出力した length-prefixed Wasm バイト列を復元するヘルパー
fn parse_emitted_wasm_modules(output: &str, expected_modules: usize) -> Vec<Vec<u8>> {
    let values: Vec<usize> = output
        .lines()
        .filter(|line| !line.trim().is_empty())
        .map(|line| {
            line.trim()
                .parse::<usize>()
                .unwrap_or_else(|_| panic!("数値でない stage1 出力: {line:?}"))
        })
        .collect();

    let mut pos = 0;
    let mut modules = Vec::with_capacity(expected_modules);
    for module_idx in 0..expected_modules {
        assert!(
            pos < values.len(),
            "module[{module_idx}] の長さ行が不足: {:?}",
            values
        );
        let len = values[pos];
        pos += 1;
        assert!(
            values.len() >= pos + len,
            "module[{module_idx}] の byte 数が不足: len={}, remaining={}",
            len,
            values.len().saturating_sub(pos)
        );

        let mut wasm = Vec::with_capacity(len);
        for &value in &values[pos..pos + len] {
            assert!(value <= u8::MAX as usize, "byte 値が範囲外: {value}");
            wasm.push(value as u8);
        }
        pos += len;
        modules.push(wasm);
    }

    assert_eq!(
        pos,
        values.len(),
        "想定外の trailing output が残っている: {:?}",
        &values[pos..]
    );
    modules
}

/// WASI ではなく素の Wasm export を呼び出し、i64 結果を確認するヘルパー
fn run_exported_i64(wasm: &[u8], export_name: &str) -> i64 {
    let engine = wasmtime::Engine::default();
    let module = wasmtime::Module::new(&engine, wasm).expect("stage2 Wasm の Module 構築に失敗");
    let mut store = wasmtime::Store::new(&engine, ());
    let instance = wasmtime::Instance::new(&mut store, &module, &[])
        .expect("stage2 Wasm のインスタンス化に失敗");
    let func = instance
        .get_typed_func::<(), i64>(&mut store, export_name)
        .unwrap_or_else(|e| panic!("{export_name} export の取得に失敗: {e}"));
    func.call(&mut store, ())
        .expect("stage2 Wasm の export 呼び出しに失敗")
}

/// selfhost 10-import レイアウト (alloc/print/read-file/command-line-arg/string-concat/
/// substring/file-exists?/root_push/root_pop/root_set) を提供して i64 を返すヘルパー。
/// emit-import-section-runtime + compile-program-functions-with-base 10 で生成した
/// stage2 Wasm を実行するために使う。
fn run_exported_i64_with_runtime_imports(wasm: &[u8], export_name: &str) -> i64 {
    let engine = wasmtime::Engine::default();
    let module = wasmtime::Module::new(&engine, wasm)
        .expect("runtime 10-import 付き stage2 Wasm の Module 構築に失敗");

    struct State {
        next_alloc: i64,
        root_stack: Vec<i64>,
    }
    let mut store = wasmtime::Store::new(
        &engine,
        State {
            next_alloc: 1024,
            root_stack: Vec::new(),
        },
    );
    let alloc = wasmtime::Func::wrap(
        &mut store,
        |mut caller: wasmtime::Caller<'_, State>, size: i64| -> i64 {
            let base = caller.data().next_alloc;
            caller.data_mut().next_alloc = base + size;
            base
        },
    );
    let print = wasmtime::Func::wrap(&mut store, |_: wasmtime::Caller<'_, State>, _: i64| {});
    let read_file = wasmtime::Func::wrap(
        &mut store,
        |_: wasmtime::Caller<'_, State>, _: i64| -> i64 { 0 },
    );
    let command_line_arg = wasmtime::Func::wrap(
        &mut store,
        |_: wasmtime::Caller<'_, State>, _: i64| -> i64 { 0 },
    );
    let string_concat = wasmtime::Func::wrap(
        &mut store,
        |_: wasmtime::Caller<'_, State>, _: i64, _: i64| -> i64 { 0 },
    );
    let substring = wasmtime::Func::wrap(
        &mut store,
        |_: wasmtime::Caller<'_, State>, _: i64, _: i64, _: i64| -> i64 { 0 },
    );
    let file_exists = wasmtime::Func::wrap(
        &mut store,
        |_: wasmtime::Caller<'_, State>, _: i64| -> i64 { 0 },
    );
    let root_push = wasmtime::Func::wrap(
        &mut store,
        |mut caller: wasmtime::Caller<'_, State>, value: i64| -> i64 {
            let slot =
                i64::try_from(caller.data().root_stack.len()).expect("root_push: slot overflow");
            caller.data_mut().root_stack.push(value);
            slot
        },
    );
    let root_pop = wasmtime::Func::wrap(
        &mut store,
        |mut caller: wasmtime::Caller<'_, State>| -> i64 {
            caller.data_mut().root_stack.pop().unwrap_or(0)
        },
    );
    let root_set = wasmtime::Func::wrap(
        &mut store,
        |mut caller: wasmtime::Caller<'_, State>, slot: i64, value: i64| -> i64 {
            let idx = usize::try_from(slot).expect("root_set: slot must be non-negative");
            if idx < caller.data().root_stack.len() {
                caller.data_mut().root_stack[idx] = value;
            }
            slot
        },
    );
    let instance = wasmtime::Instance::new(
        &mut store,
        &module,
        &[
            alloc.into(),
            print.into(),
            read_file.into(),
            command_line_arg.into(),
            string_concat.into(),
            substring.into(),
            file_exists.into(),
            root_push.into(),
            root_pop.into(),
            root_set.into(),
        ],
    )
    .expect("runtime 10-import 付き stage2 Wasm のインスタンス化に失敗");
    let func = instance
        .get_typed_func::<(), i64>(&mut store, export_name)
        .unwrap_or_else(|e| panic!("{export_name} export の取得に失敗: {e}"));
    func.call(&mut store, ())
        .expect("runtime 10-import 付き stage2 Wasm の export 呼び出しに失敗")
}

/// `env.__alloc: (i64) -> i64` import を持つ stage2 Wasm を実行するヘルパー
fn run_exported_i64_with_alloc_import(wasm: &[u8], export_name: &str) -> i64 {
    let engine = wasmtime::Engine::default();
    let module = wasmtime::Module::new(&engine, wasm)
        .expect("alloc import 付き stage2 Wasm の Module 構築に失敗");
    let mut store = wasmtime::Store::new(&engine, 1024_i64);
    let alloc = wasmtime::Func::wrap(
        &mut store,
        |mut caller: wasmtime::Caller<'_, i64>, size: i64| -> i64 {
            let base = *caller.data();
            let end = base
                .checked_add(size)
                .unwrap_or_else(|| panic!("alloc import: end address が overflow"));
            ensure_memory_capacity(&mut caller, end, "alloc import");
            *caller.data_mut() = end;
            base
        },
    );
    let instance = wasmtime::Instance::new(&mut store, &module, &[alloc.into()])
        .expect("alloc import 付き stage2 Wasm のインスタンス化に失敗");
    let func = instance
        .get_typed_func::<(), i64>(&mut store, export_name)
        .unwrap_or_else(|e| panic!("{export_name} export の取得に失敗: {e}"));
    func.call(&mut store, ())
        .expect("alloc import 付き stage2 Wasm の export 呼び出しに失敗")
}

#[derive(Default)]
struct AllocPrintState {
    next_alloc: i64,
    printed: String,
}

/// `env.__alloc: (i64) -> i64` と `env.print: (i64) -> ()` import を持つ stage2 Wasm を実行するヘルパー
fn run_exported_i64_with_alloc_print_imports(wasm: &[u8], export_name: &str) -> (i64, String) {
    let engine = wasmtime::Engine::default();
    let module = wasmtime::Module::new(&engine, wasm)
        .expect("alloc/print import 付き stage2 Wasm の Module 構築に失敗");
    let mut store = wasmtime::Store::new(
        &engine,
        AllocPrintState {
            next_alloc: 1024,
            printed: String::new(),
        },
    );
    let alloc = wasmtime::Func::wrap(
        &mut store,
        |mut caller: wasmtime::Caller<'_, AllocPrintState>, size: i64| -> i64 {
            let base = caller.data().next_alloc;
            let end = base
                .checked_add(size)
                .unwrap_or_else(|| panic!("alloc/print import: end address が overflow"));
            ensure_memory_capacity(&mut caller, end, "alloc/print import");
            caller.data_mut().next_alloc = end;
            base
        },
    );
    let print = wasmtime::Func::wrap(
        &mut store,
        |mut caller: wasmtime::Caller<'_, AllocPrintState>, value: i64| {
            caller.data_mut().printed.push_str(&format!("{value}\n"));
        },
    );
    let instance = wasmtime::Instance::new(&mut store, &module, &[alloc.into(), print.into()])
        .expect("alloc/print import 付き stage2 Wasm のインスタンス化に失敗");
    let func = instance
        .get_typed_func::<(), i64>(&mut store, export_name)
        .unwrap_or_else(|e| panic!("{export_name} export の取得に失敗: {e}"));
    let value = func
        .call(&mut store, ())
        .expect("alloc/print import 付き stage2 Wasm の export 呼び出しに失敗");
    let printed = store.data().printed.clone();
    (value, printed)
}

#[derive(Default)]
struct AllocPrintReadState {
    next_alloc: i64,
    printed: String,
    file_content: String,
}

/// `env.__alloc`, `env.print`, `env.read-file` import を持つ stage2 Wasm を実行するヘルパー
fn run_exported_i64_with_alloc_print_read_imports(
    wasm: &[u8],
    export_name: &str,
    file_content: &str,
) -> (i64, String) {
    let engine = wasmtime::Engine::default();
    let module = wasmtime::Module::new(&engine, wasm)
        .expect("alloc/print/read-file import 付き stage2 Wasm の Module 構築に失敗");
    let mut store = wasmtime::Store::new(
        &engine,
        AllocPrintReadState {
            next_alloc: 1024,
            printed: String::new(),
            file_content: file_content.to_string(),
        },
    );
    let alloc = wasmtime::Func::wrap(
        &mut store,
        |mut caller: wasmtime::Caller<'_, AllocPrintReadState>, size: i64| -> i64 {
            let base = caller.data().next_alloc;
            let end = base
                .checked_add(size)
                .unwrap_or_else(|| panic!("alloc/print/read import: end address が overflow"));
            ensure_memory_capacity(&mut caller, end, "alloc/print/read import");
            caller.data_mut().next_alloc = end;
            base
        },
    );
    let print = wasmtime::Func::wrap(
        &mut store,
        |mut caller: wasmtime::Caller<'_, AllocPrintReadState>, value: i64| {
            caller.data_mut().printed.push_str(&format!("{value}\n"));
        },
    );
    let read_file = wasmtime::Func::wrap(
        &mut store,
        |mut caller: wasmtime::Caller<'_, AllocPrintReadState>, _path: i64| -> i64 {
            let content = caller.data().file_content.as_bytes().to_vec();
            let base = caller.data().next_alloc;
            let end = base
                .checked_add(
                    i64::try_from(8 + content.len()).expect("read-file object size overflow"),
                )
                .unwrap_or_else(|| {
                    panic!("alloc/print/read import: read-file end address が overflow")
                });
            caller.data_mut().next_alloc = end;
            write_string_object_bytes(caller, base, &content, "alloc/print/read import");
            base
        },
    );
    let instance = wasmtime::Instance::new(
        &mut store,
        &module,
        &[alloc.into(), print.into(), read_file.into()],
    )
    .expect("alloc/print/read-file import 付き stage2 Wasm のインスタンス化に失敗");
    let func = instance
        .get_typed_func::<(), i64>(&mut store, export_name)
        .unwrap_or_else(|e| panic!("{export_name} export の取得に失敗: {e}"));
    let value = func
        .call(&mut store, ())
        .expect("alloc/print/read-file import 付き stage2 Wasm の export 呼び出しに失敗");
    let printed = store.data().printed.clone();
    (value, printed)
}

fn read_memory_text<T>(caller: &mut wasmtime::Caller<'_, T>, addr: i64, len: usize) -> String {
    let memory = match caller.get_export("memory") {
        Some(wasmtime::Extern::Memory(memory)) => memory,
        _ => panic!("memory export が見つからない"),
    };
    let mut bytes = vec![0_u8; len];
    memory
        .read(&mut *caller, addr as usize, &mut bytes)
        .expect("memory text bytes を読めない");
    String::from_utf8(bytes).expect("string object bytes が UTF-8 ではない")
}

fn read_string_object_bytes<T>(caller: &mut wasmtime::Caller<'_, T>, addr: i64) -> Vec<u8> {
    let memory = match caller.get_export("memory") {
        Some(wasmtime::Extern::Memory(memory)) => memory,
        _ => panic!("memory export が見つからない"),
    };
    let mut len_bytes = [0_u8; 4];
    memory
        .read(&mut *caller, addr as usize + 4, &mut len_bytes)
        .expect("string object length を読めない");
    let len = i32::from_le_bytes(len_bytes);
    let len = usize::try_from(len).expect("string object length が負");
    let mut bytes = vec![0_u8; len];
    memory
        .read(&mut *caller, addr as usize + 8, &mut bytes)
        .expect("string object bytes を読めない");
    bytes
}

fn read_path_text<T>(
    caller: &mut wasmtime::Caller<'_, T>,
    addr: i64,
    expected_len: usize,
) -> String {
    let memory = exported_memory(caller);
    let mut header = [0_u8; 8];
    if memory
        .read(&mut *caller, addr as usize, &mut header)
        .is_ok()
    {
        let tag = i32::from_le_bytes(header[0..4].try_into().expect("tag header 長が不正"));
        let len = i32::from_le_bytes(header[4..8].try_into().expect("len header 長が不正"));
        if tag == 1 && usize::try_from(len).ok() == Some(expected_len) {
            return String::from_utf8(read_string_object_bytes(caller, addr))
                .expect("path string object bytes が UTF-8 ではない");
        }
    }
    read_memory_text(caller, addr, expected_len)
}

fn read_path_text_with_root<T>(
    caller: &mut wasmtime::Caller<'_, T>,
    addr: i64,
    root_dir: &std::path::Path,
) -> String {
    let memory = exported_memory(caller);
    let data_size = memory.data_size(&mut *caller);
    let addr_usize = usize::try_from(addr).expect("path addr が負");
    assert!(addr_usize < data_size, "path addr が memory 範囲外: {addr}");

    let mut header = [0_u8; 8];
    if addr_usize + 8 <= data_size && memory.read(&mut *caller, addr_usize, &mut header).is_ok() {
        let tag = i32::from_le_bytes(header[0..4].try_into().expect("tag header 長が不正"));
        let len = i32::from_le_bytes(header[4..8].try_into().expect("len header 長が不正"));
        if tag == 1
            && let Ok(len) = usize::try_from(len)
            && addr_usize + 8 + len <= data_size
        {
            let text = String::from_utf8(read_string_object_bytes(caller, addr))
                .expect("path string object bytes が UTF-8 ではない");
            return text;
        }
    }

    let max_len = (data_size - addr_usize).min(512);
    let mut raw = vec![0_u8; max_len];
    memory
        .read(&mut *caller, addr_usize, &mut raw)
        .expect("raw path bytes を読めない");
    for len in 1..=max_len {
        let Ok(text) = std::str::from_utf8(&raw[..len]) else {
            continue;
        };
        if !(text.ends_with(".ls") || text.ends_with(".path")) {
            continue;
        }
        let full_path = root_dir.join(text);
        if full_path.exists() {
            return text.to_string();
        }
    }

    panic!(
        "path decode に失敗: addr={addr}, header={:?}, raw_prefix={:?}",
        header,
        &raw[..raw.len().min(32)]
    );
}

fn fnv1a_hash_bytes(bytes: &[u8]) -> i64 {
    let mut hash: u64 = 0xcbf29ce484222325;
    for &byte in bytes {
        hash ^= u64::from(byte);
        hash = hash.wrapping_mul(0x100000001b3);
    }
    hash as i64
}

fn run_exported_i64_with_alloc_print_read_path_imports(
    wasm: &[u8],
    export_name: &str,
    expected_path: &str,
    file_content: &str,
) -> (i64, String) {
    let engine = wasmtime::Engine::default();
    let module = wasmtime::Module::new(&engine, wasm)
        .expect("alloc/print/read-file import 付き stage2 Wasm の Module 構築に失敗");
    let mut store = wasmtime::Store::new(
        &engine,
        AllocPrintReadState {
            next_alloc: 1024,
            printed: String::new(),
            file_content: file_content.to_string(),
        },
    );
    let alloc = wasmtime::Func::wrap(
        &mut store,
        |mut caller: wasmtime::Caller<'_, AllocPrintReadState>, size: i64| -> i64 {
            let base = caller.data().next_alloc;
            let end = base
                .checked_add(size)
                .unwrap_or_else(|| panic!("alloc/print/read/path import: end address が overflow"));
            ensure_memory_capacity(&mut caller, end, "alloc/print/read/path import");
            caller.data_mut().next_alloc = end;
            base
        },
    );
    let print = wasmtime::Func::wrap(
        &mut store,
        |mut caller: wasmtime::Caller<'_, AllocPrintReadState>, value: i64| {
            caller.data_mut().printed.push_str(&format!("{value}\n"));
        },
    );
    let expected_path = expected_path.to_string();
    let read_file = wasmtime::Func::wrap(
        &mut store,
        move |mut caller: wasmtime::Caller<'_, AllocPrintReadState>, path: i64| -> i64 {
            let actual_path = read_path_text(&mut caller, path, expected_path.len());
            assert_eq!(actual_path, expected_path, "read-file path string が不正");
            let content = caller.data().file_content.as_bytes().to_vec();
            let base = caller.data().next_alloc;
            let end = base
                .checked_add(
                    i64::try_from(8 + content.len()).expect("read-file object size overflow"),
                )
                .unwrap_or_else(|| {
                    panic!("alloc/print/read/path import: read-file end address が overflow")
                });
            caller.data_mut().next_alloc = end;
            write_string_object_bytes(caller, base, &content, "alloc/print/read/path import");
            base
        },
    );
    let instance = wasmtime::Instance::new(
        &mut store,
        &module,
        &[alloc.into(), print.into(), read_file.into()],
    )
    .expect("alloc/print/read-file import 付き stage2 Wasm のインスタンス化に失敗");
    let func = instance
        .get_typed_func::<(), i64>(&mut store, export_name)
        .unwrap_or_else(|e| panic!("{export_name} export の取得に失敗: {e}"));
    let value = func
        .call(&mut store, ())
        .expect("alloc/print/read-file import 付き stage2 Wasm の export 呼び出しに失敗");
    let printed = store.data().printed.clone();
    (value, printed)
}

fn run_exported_i64_with_alloc_print_read_hash_imports(
    wasm: &[u8],
    export_name: &str,
    file_content: &str,
) -> (i64, String) {
    let engine = wasmtime::Engine::default();
    let module = wasmtime::Module::new(&engine, wasm)
        .expect("alloc/print/read-file/fnv1a import 付き stage2 Wasm の Module 構築に失敗");
    let mut store = wasmtime::Store::new(
        &engine,
        AllocPrintReadState {
            next_alloc: 1024,
            printed: String::new(),
            file_content: file_content.to_string(),
        },
    );
    let alloc = wasmtime::Func::wrap(
        &mut store,
        |mut caller: wasmtime::Caller<'_, AllocPrintReadState>, size: i64| -> i64 {
            let base = caller.data().next_alloc;
            let end = base
                .checked_add(size)
                .unwrap_or_else(|| panic!("alloc/print/read/hash import: end address が overflow"));
            ensure_memory_capacity(&mut caller, end, "alloc/print/read/hash import");
            caller.data_mut().next_alloc = end;
            base
        },
    );
    let print = wasmtime::Func::wrap(
        &mut store,
        |mut caller: wasmtime::Caller<'_, AllocPrintReadState>, value: i64| {
            caller.data_mut().printed.push_str(&format!("{value}\n"));
        },
    );
    let read_file = wasmtime::Func::wrap(
        &mut store,
        |mut caller: wasmtime::Caller<'_, AllocPrintReadState>, _path: i64| -> i64 {
            let content = caller.data().file_content.as_bytes().to_vec();
            let base = caller.data().next_alloc;
            let end = base
                .checked_add(
                    i64::try_from(8 + content.len()).expect("read-file object size overflow"),
                )
                .unwrap_or_else(|| {
                    panic!("alloc/print/read/hash import: read-file end address が overflow")
                });
            caller.data_mut().next_alloc = end;
            write_string_object_bytes(caller, base, &content, "alloc/print/read/hash import");
            base
        },
    );
    let fnv1a_hash = wasmtime::Func::wrap(
        &mut store,
        |mut caller: wasmtime::Caller<'_, AllocPrintReadState>, value: i64| -> i64 {
            fnv1a_hash_bytes(&read_string_object_bytes(&mut caller, value))
        },
    );
    let instance = wasmtime::Instance::new(
        &mut store,
        &module,
        &[
            alloc.into(),
            print.into(),
            read_file.into(),
            fnv1a_hash.into(),
        ],
    )
    .expect("alloc/print/read-file/fnv1a import 付き stage2 Wasm のインスタンス化に失敗");
    let func = instance
        .get_typed_func::<(), i64>(&mut store, export_name)
        .unwrap_or_else(|e| panic!("{export_name} export の取得に失敗: {e}"));
    let value = func
        .call(&mut store, ())
        .expect("alloc/print/read-file/fnv1a import 付き stage2 Wasm の export 呼び出しに失敗");
    let printed = store.data().printed.clone();
    (value, printed)
}

#[derive(Default)]
struct AllocPrintReadArgState {
    next_alloc: i64,
    printed: String,
    file_content: String,
    args: Vec<String>,
}

/// `env.__alloc`, `env.print`, `env.read-file`, `env.command-line-arg` import を持つ stage2 Wasm を実行するヘルパー
fn run_exported_i64_with_alloc_print_read_arg_imports(
    wasm: &[u8],
    export_name: &str,
    file_content: &str,
    args: &[&str],
) -> (i64, String) {
    let engine = wasmtime::Engine::default();
    let module = wasmtime::Module::new(&engine, wasm).expect(
        "alloc/print/read-file/command-line-arg import 付き stage2 Wasm の Module 構築に失敗",
    );
    let mut store = wasmtime::Store::new(
        &engine,
        AllocPrintReadArgState {
            next_alloc: 1024,
            printed: String::new(),
            file_content: file_content.to_string(),
            args: args.iter().map(|arg| (*arg).to_string()).collect(),
        },
    );
    let alloc = wasmtime::Func::wrap(
        &mut store,
        |mut caller: wasmtime::Caller<'_, AllocPrintReadArgState>, size: i64| -> i64 {
            let base = caller.data().next_alloc;
            let end = base
                .checked_add(size)
                .unwrap_or_else(|| panic!("alloc/print/read/arg import: end address が overflow"));
            ensure_memory_capacity(&mut caller, end, "alloc/print/read/arg import");
            caller.data_mut().next_alloc = end;
            base
        },
    );
    let print = wasmtime::Func::wrap(
        &mut store,
        |mut caller: wasmtime::Caller<'_, AllocPrintReadArgState>, value: i64| {
            caller.data_mut().printed.push_str(&format!("{value}\n"));
        },
    );
    let read_file = wasmtime::Func::wrap(
        &mut store,
        |mut caller: wasmtime::Caller<'_, AllocPrintReadArgState>, _path: i64| -> i64 {
            let content = caller.data().file_content.as_bytes().to_vec();
            let base = caller.data().next_alloc;
            let end = base
                .checked_add(
                    i64::try_from(8 + content.len()).expect("read-file object size overflow"),
                )
                .unwrap_or_else(|| {
                    panic!("alloc/print/read/arg import: read-file end address が overflow")
                });
            caller.data_mut().next_alloc = end;
            write_string_object_bytes(caller, base, &content, "alloc/print/read/arg import");
            base
        },
    );
    let command_line_arg = wasmtime::Func::wrap(
        &mut store,
        |mut caller: wasmtime::Caller<'_, AllocPrintReadArgState>, index: i64| -> i64 {
            let content = usize::try_from(index)
                .ok()
                .and_then(|idx| caller.data().args.get(idx))
                .map(|arg| arg.as_bytes().to_vec())
                .unwrap_or_default();
            let base = caller.data().next_alloc;
            let end = base
                .checked_add(
                    i64::try_from(8 + content.len())
                        .expect("command-line-arg object size overflow"),
                )
                .unwrap_or_else(|| {
                    panic!("alloc/print/read/arg import: command-line-arg end address が overflow")
                });
            caller.data_mut().next_alloc = end;
            write_string_object_bytes(caller, base, &content, "alloc/print/read/arg import");
            base
        },
    );
    let instance = wasmtime::Instance::new(
        &mut store,
        &module,
        &[
            alloc.into(),
            print.into(),
            read_file.into(),
            command_line_arg.into(),
        ],
    )
    .expect(
        "alloc/print/read-file/command-line-arg import 付き stage2 Wasm のインスタンス化に失敗",
    );
    let func = instance
        .get_typed_func::<(), i64>(&mut store, export_name)
        .unwrap_or_else(|e| panic!("{export_name} export の取得に失敗: {e}"));
    let value = func.call(&mut store, ()).expect(
        "alloc/print/read-file/command-line-arg import 付き stage2 Wasm の export 呼び出しに失敗",
    );
    let printed = store.data().printed.clone();
    (value, printed)
}

/// legacy 名だが、現在は root runtime helper まで含む import モデルで wasm を実行する
///
/// stage2 以降の wasm は env.string-concat, env.substring, env.file-exists? も import するため、
/// 4-import ハーネスの代わりにこちらを使用する。
/// さらに selfhost parity のため env.root_push/env.root_pop/env.root_set も提供する。
struct ElevenImportState {
    next_alloc: i64,
    printed: String,
    file_content: String,
    file_root: Option<std::path::PathBuf>,
    args: Vec<String>,
    string_object_cache: HashMap<Vec<u8>, i64>,
    root_stack: Vec<i64>,
}

fn alloc_cached_string_object(
    mut caller: wasmtime::Caller<'_, ElevenImportState>,
    content: Vec<u8>,
    context: &str,
) -> i64 {
    if let Some(addr) = caller.data().string_object_cache.get(&content).copied() {
        return addr;
    }
    let base = caller.data().next_alloc;
    let end = base
        .checked_add(i64::try_from(8 + content.len()).expect("cached string object size overflow"))
        .unwrap_or_else(|| panic!("{context}: cached string object end address が overflow"));
    {
        let state = caller.data_mut();
        state.next_alloc = end;
        state.string_object_cache.insert(content.clone(), base);
    }
    write_string_object_bytes(caller, base, &content, context);
    base
}
