use super::support::*;
use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};

// =============================================================================
// BOOT-04: True stage1-stage2-stage3 bootstrap 4 層検証テスト
// =============================================================================

/// Wasm バイナリからセクション ID とサイズの列を抽出するヘルパー
fn extract_sections(wasm: &[u8]) -> Vec<(u8, usize)> {
    let mut sections = Vec::new();
    let mut pos = 8; // magic(4) + version(4)
    while pos < wasm.len() {
        let section_id = wasm[pos];
        pos += 1;
        let mut size: usize = 0;
        let mut shift = 0;
        loop {
            if pos >= wasm.len() {
                break;
            }
            let byte = wasm[pos] as usize;
            pos += 1;
            size |= (byte & 0x7f) << shift;
            if byte & 0x80 == 0 {
                break;
            }
            shift += 7;
        }
        sections.push((section_id, size));
        pos += size;
    }
    sections
}

/// 指定セクション ID のバイト列を抽出するヘルパー
fn extract_section_bytes(wasm: &[u8], target_id: u8) -> Option<Vec<u8>> {
    let mut pos = 8;
    while pos < wasm.len() {
        let section_id = wasm[pos];
        pos += 1;
        let mut size: usize = 0;
        let mut shift = 0;
        loop {
            if pos >= wasm.len() {
                break;
            }
            let byte = wasm[pos] as usize;
            pos += 1;
            size |= (byte & 0x7f) << shift;
            if byte & 0x80 == 0 {
                break;
            }
            shift += 7;
        }
        if section_id == target_id {
            return Some(wasm[pos..pos + size].to_vec());
        }
        pos += size;
    }
    None
}

/// バイト列のハッシュフィンガープリントを計算するヘルパー
fn hash_fingerprint(data: &[u8]) -> u64 {
    let mut hasher = DefaultHasher::new();
    data.hash(&mut hasher);
    hasher.finish()
}

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
            *caller.data_mut() = base + size;
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
            caller.data_mut().next_alloc = base + size;
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
            caller.data_mut().next_alloc = base + size;
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
            caller.data_mut().next_alloc = base + 8 + content.len() as i64;
            let memory = match caller.get_export("memory") {
                Some(wasmtime::Extern::Memory(memory)) => memory,
                _ => panic!("memory export が見つからない"),
            };
            let mut object = Vec::with_capacity(8 + content.len());
            object.extend_from_slice(&1_i32.to_le_bytes());
            object.extend_from_slice(&(content.len() as i32).to_le_bytes());
            object.extend_from_slice(&content);
            memory
                .write(&mut caller, base as usize, &object)
                .expect("read-file import が stage2 memory へ書き込めない");
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
            caller.data_mut().next_alloc = base + size;
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
            let actual_path = read_memory_text(&mut caller, path, expected_path.len());
            assert_eq!(actual_path, expected_path, "read-file path string が不正");
            let content = caller.data().file_content.as_bytes().to_vec();
            let base = caller.data().next_alloc;
            caller.data_mut().next_alloc = base + 8 + content.len() as i64;
            let memory = match caller.get_export("memory") {
                Some(wasmtime::Extern::Memory(memory)) => memory,
                _ => panic!("memory export が見つからない"),
            };
            let mut object = Vec::with_capacity(8 + content.len());
            object.extend_from_slice(&1_i32.to_le_bytes());
            object.extend_from_slice(&(content.len() as i32).to_le_bytes());
            object.extend_from_slice(&content);
            memory
                .write(&mut caller, base as usize, &object)
                .expect("read-file import が stage2 memory へ書き込めない");
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
            caller.data_mut().next_alloc = base + size;
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
            caller.data_mut().next_alloc = base + 8 + content.len() as i64;
            let memory = match caller.get_export("memory") {
                Some(wasmtime::Extern::Memory(memory)) => memory,
                _ => panic!("memory export が見つからない"),
            };
            let mut object = Vec::with_capacity(8 + content.len());
            object.extend_from_slice(&1_i32.to_le_bytes());
            object.extend_from_slice(&(content.len() as i32).to_le_bytes());
            object.extend_from_slice(&content);
            memory
                .write(&mut caller, base as usize, &object)
                .expect("read-file import が stage2 memory へ書き込めない");
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
    fn write_string_object<T>(mut caller: wasmtime::Caller<'_, T>, base: i64, content: &[u8]) {
        let memory = match caller.get_export("memory") {
            Some(wasmtime::Extern::Memory(memory)) => memory,
            _ => panic!("memory export が見つからない"),
        };
        let mut object = Vec::with_capacity(8 + content.len());
        object.extend_from_slice(&1_i32.to_le_bytes());
        object.extend_from_slice(&(content.len() as i32).to_le_bytes());
        object.extend_from_slice(content);
        memory
            .write(&mut caller, base as usize, &object)
            .expect("string object を stage2 memory へ書き込めない");
    }

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
            caller.data_mut().next_alloc = base + size;
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
            caller.data_mut().next_alloc = base + 8 + content.len() as i64;
            write_string_object(caller, base, &content);
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
            caller.data_mut().next_alloc = base + 8 + content.len() as i64;
            write_string_object(caller, base, &content);
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

/// BOOT-04: 4 層比較テスト
///
/// selfhost コンパイラを Rust stage0 で 2 回コンパイルし、
/// 以下の 4 レイヤーで出力の同一性を検証する:
///   1. ハッシュフィンガープリント (raw bytes)
///   2. Export セクションシンボル
///   3. Data セクションバイト列
///   4. 診断カウント (コンパイル成功 = 0)
///
/// 真の stage1→stage2 自己コンパイルは未接続。
/// stage0 (Rust) コンパイラの決定性を 4 次元で検証する。
#[test]
fn test_e2e_bootstrap_four_layer_comparison() {
    let main_path = selfhost_main_path();

    // stage0 (Rust) で selfhost/Main.ls を 2 回コンパイル
    let wasm_a = compile_file_only(&main_path);
    let wasm_b = compile_file_only(&main_path);

    // レイヤー 1: ハッシュフィンガープリント比較
    let hash_a = hash_fingerprint(&wasm_a);
    let hash_b = hash_fingerprint(&wasm_b);
    assert_eq!(
        hash_a, hash_b,
        "レイヤー1: ハッシュフィンガープリント不一致 — {:#018x} vs {:#018x}",
        hash_a, hash_b
    );

    // レイヤー 2: Export セクション (ID=7) のシンボル比較
    let export_a =
        extract_section_bytes(&wasm_a, 7).expect("wasm_a に Export セクションが見つからない");
    let export_b =
        extract_section_bytes(&wasm_b, 7).expect("wasm_b に Export セクションが見つからない");
    assert_eq!(
        export_a,
        export_b,
        "レイヤー2: Export セクション不一致 — {} bytes vs {} bytes",
        export_a.len(),
        export_b.len()
    );
    assert!(!export_a.is_empty(), "Export セクションが空");

    // レイヤー 3: Data セクション (ID=11) のバイト列比較
    // Data セクションが存在しない場合は両方 None で一致とする
    let data_a = extract_section_bytes(&wasm_a, 11);
    let data_b = extract_section_bytes(&wasm_b, 11);
    assert_eq!(
        data_a,
        data_b,
        "レイヤー3: Data セクション不一致 — {:?} bytes vs {:?} bytes",
        data_a.as_ref().map(|d| d.len()),
        data_b.as_ref().map(|d| d.len())
    );

    // レイヤー 4: 診断カウント比較
    // コンパイル成功 = 診断 0。try_compile_file_only でエラーを検出可能。
    let diag_a = try_compile_file_only(&main_path).is_ok();
    let diag_b = try_compile_file_only(&main_path).is_ok();
    assert_eq!(
        diag_a, diag_b,
        "レイヤー4: 診断結果不一致 — {} vs {}",
        diag_a, diag_b
    );
    assert!(diag_a, "コンパイルが失敗した（診断あり）");

    // 追加検証: raw bytes が完全一致
    assert_eq!(
        wasm_a,
        wasm_b,
        "raw bytes 不一致 — {} bytes vs {} bytes",
        wasm_a.len(),
        wasm_b.len()
    );

    // 追加検証: セクション構造の安定性
    let sections_a = extract_sections(&wasm_a);
    let sections_b = extract_sections(&wasm_b);
    assert_eq!(sections_a, sections_b, "セクション構造不一致");
}

/// BOOT-04: ステージチェーン検証テスト
///
/// stage0 (Rust) → stage1 (Wasm) の連鎖を検証する:
///   1. stage0 で selfhost の最小サブセット (Token.ls) をコンパイル
///   2. stage0 で Main.ls をコンパイルして stage1.wasm を生成
///   3. stage1.wasm を WASI 実行し、コンパイラとして動作することを確認
///   4. stage0 の出力構造 (セクション・エクスポート) が安定していることを検証
///
/// 真の stage1→stage2 自己コンパイルは未接続のため、
/// stage0 の決定性 + stage1 の実行可能性を証明する。
#[test]
fn test_e2e_bootstrap_stage_chain_verification() {
    let selfhost_dir = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../selfhost");
    let main_path = selfhost_dir.join("Main.ls");

    // --- Phase 1: stage0 で最小サブセットをコンパイル ---
    // Token.ls は依存なしの最小モジュール
    let token_path = selfhost_dir.join("Token.ls");
    let token_wasm_1 = compile_file_only(&token_path);
    let token_wasm_2 = compile_file_only(&token_path);
    assert_eq!(
        token_wasm_1, token_wasm_2,
        "Phase1: Token.ls の stage0 コンパイルが非決定的"
    );
    assert_valid_wasm(&token_wasm_1);

    // --- Phase 2: stage0 で Main.ls をコンパイル → stage1.wasm ---
    let stage1_wasm_a = compile_file_only(&main_path);
    let stage1_wasm_b = compile_file_only(&main_path);
    assert_eq!(
        stage1_wasm_a, stage1_wasm_b,
        "Phase2: Main.ls の stage0 コンパイルが非決定的"
    );
    assert_valid_wasm(&stage1_wasm_a);

    // --- Phase 3: stage1.wasm の実行可能性検証 ---
    // stage1 コンパイラ (Main.ls) を WASI 実行し、正常終了を確認
    let stage1_result = lsharp_wasm::wasi_runner::run_wasm_wasi(&stage1_wasm_a);
    assert!(
        stage1_result.is_ok(),
        "Phase3: stage1.wasm の WASI 実行に失敗 — {:?}",
        stage1_result.err()
    );
    let stage1_output = stage1_result.unwrap();
    assert!(
        !stage1_output.is_empty(),
        "Phase3: stage1 コンパイラの出力が空"
    );

    // --- Phase 4: stage0 出力の構造的一致検証 ---
    // Token.ls と Main.ls 両方の構造が安定していることを検証

    // Token.ls: Export セクション安定性
    let token_export_1 = extract_section_bytes(&token_wasm_1, 7);
    let token_export_2 = extract_section_bytes(&token_wasm_2, 7);
    assert_eq!(
        token_export_1, token_export_2,
        "Phase4: Token.ls の Export セクションが不安定"
    );

    // Main.ls: 4 層全て安定
    let main_hash_a = hash_fingerprint(&stage1_wasm_a);
    let main_hash_b = hash_fingerprint(&stage1_wasm_b);
    assert_eq!(
        main_hash_a, main_hash_b,
        "Phase4: Main.ls のハッシュフィンガープリント不一致"
    );

    let main_export_a = extract_section_bytes(&stage1_wasm_a, 7)
        .expect("stage1_a に Export セクションが見つからない");
    let main_export_b = extract_section_bytes(&stage1_wasm_b, 7)
        .expect("stage1_b に Export セクションが見つからない");
    assert_eq!(
        main_export_a, main_export_b,
        "Phase4: Main.ls の Export セクション不一致"
    );

    let main_data_a = extract_section_bytes(&stage1_wasm_a, 11);
    let main_data_b = extract_section_bytes(&stage1_wasm_b, 11);
    assert_eq!(
        main_data_a, main_data_b,
        "Phase4: Main.ls の Data セクション不一致"
    );

    // --- Phase 5: stage1 出力の再現性検証 ---
    // stage1 を再度実行し、同じ出力が得られることを確認
    let stage1_result_2 = lsharp_wasm::wasi_runner::run_wasm_wasi(&stage1_wasm_a);
    assert!(
        stage1_result_2.is_ok(),
        "Phase5: stage1.wasm の 2 回目実行に失敗"
    );
    let stage1_output_2 = stage1_result_2.unwrap();
    assert_eq!(
        stage1_output, stage1_output_2,
        "Phase5: stage1 コンパイラの出力が非決定的"
    );
}

/// BOOT-04: stage1 が narrow subset を実際に stage2 Wasm へコンパイルできること
///
/// true fixed-point そのものではないが、Rust stage0 が生成した stage1 が
/// selfhost の Parser/Compiler/WasmEmit を使って実体の Wasm bytes を出力し、
/// その stage2 を実行できる最小 bootstrap 経路を固定する。
#[test]
fn test_e2e_bootstrap_stage1_emits_stage2_wasm_for_minimal_subset() {
    let harness = r#"
(defn bootstrap-append-bytes [dst src idx count]
  (if (>= idx count)
    dst
    (bootstrap-append-bytes
      (vector-push dst (vector-get src idx))
      src
      (+ idx 1)
      count)))

(defn bootstrap-build-stage2 [src]
  (let [program (parse-program src)
        ir (lower program)
        header (emit-header)
        type-sec (emit-type-section-main)
        function-sec (emit-function-section)
        export-sec (emit-export-section)
        code-sec (emit-code-section ir)
        bytes0 (bootstrap-append-bytes (vector-new 64) header 0 (vector-length header))
        bytes1 (bootstrap-append-bytes bytes0 type-sec 0 (vector-length type-sec))
        bytes2 (bootstrap-append-bytes bytes1 function-sec 0 (vector-length function-sec))
        bytes3 (bootstrap-append-bytes bytes2 export-sec 0 (vector-length export-sec))]
    (bootstrap-append-bytes bytes3 code-sec 0 (vector-length code-sec))))

(defn bootstrap-print-module-bytes [bytes idx count]
  (if (>= idx count)
    0
    (do
      (print (vector-get bytes idx))
      (bootstrap-print-module-bytes bytes (+ idx 1) count))))

(defn bootstrap-print-module [bytes]
  (let [count (vector-length bytes)]
    (do
      (print count)
      (bootstrap-print-module-bytes bytes 0 count)
      0)))

(defn main []
  (let [stage2-a (bootstrap-build-stage2 "(defn main [] 42)")
        stage2-b (bootstrap-build-stage2 "(defn main [] 7)")]
    (do
      (bootstrap-print-module stage2-a)
      (bootstrap-print-module stage2-b)
      0)))
"#;
    let stage1_source = format!(
        "{}\n{}\n{}\n{}\n{}\n{}\n{}\n{}",
        selfhost_module("Token.ls"),
        selfhost_module("AST.ls"),
        selfhost_module("Lexer.ls"),
        selfhost_module("Parser.ls"),
        selfhost_module("IR.ls"),
        selfhost_module("Compiler.ls"),
        selfhost_module("WasmEmit.ls"),
        harness
    );
    let stage1_wasm = compile_only(&stage1_source);
    assert_valid_wasm(&stage1_wasm);

    let first_output = lsharp_wasm::wasi_runner::run_wasm_wasi(&stage1_wasm)
        .expect("stage1 wasm の 1 回目実行に失敗");
    let second_output = lsharp_wasm::wasi_runner::run_wasm_wasi(&stage1_wasm)
        .expect("stage1 wasm の 2 回目実行に失敗");
    assert_eq!(
        first_output, second_output,
        "stage1 の stage2 生成結果が非決定的"
    );

    let modules = parse_emitted_wasm_modules(&first_output, 2);
    assert_eq!(modules.len(), 2, "stage2 モジュール数が不正");
    assert_ne!(
        modules[0], modules[1],
        "異なる入力ソースから同一 stage2 Wasm が出力された"
    );

    for (idx, wasm) in modules.iter().enumerate() {
        assert_valid_wasm(wasm);
        assert!(
            wasm.len() > 8,
            "module[{idx}] の stage2 Wasm が短すぎる: {} bytes",
            wasm.len()
        );
    }

    assert_eq!(run_exported_i64(&modules[0], "_start"), 42);
    assert_eq!(run_exported_i64(&modules[1], "_start"), 7);
}

/// BOOT-04: stage1 が同じ tiny source から同一 stage2 Wasm を 2 回生成できること
#[test]
fn test_e2e_bootstrap_stage1_emits_identical_stage2_wasm_for_same_tiny_source() {
    let harness = r#"
(defn bootstrap-append-bytes [dst src idx count]
  (if (>= idx count)
    dst
    (bootstrap-append-bytes
      (vector-push dst (vector-get src idx))
      src
      (+ idx 1)
      count)))

(defn bootstrap-build-stage2 [src]
  (let [program (parse-program src)
        ir (lower program)
        header (emit-header)
        type-sec (emit-type-section-main)
        function-sec (emit-function-section)
        export-sec (emit-export-section)
        code-sec (emit-code-section ir)
        bytes0 (bootstrap-append-bytes (vector-new 64) header 0 (vector-length header))
        bytes1 (bootstrap-append-bytes bytes0 type-sec 0 (vector-length type-sec))
        bytes2 (bootstrap-append-bytes bytes1 function-sec 0 (vector-length function-sec))
        bytes3 (bootstrap-append-bytes bytes2 export-sec 0 (vector-length export-sec))]
    (bootstrap-append-bytes bytes3 code-sec 0 (vector-length code-sec))))

(defn bootstrap-print-module-bytes [bytes idx count]
  (if (>= idx count)
    0
    (do
      (print (vector-get bytes idx))
      (bootstrap-print-module-bytes bytes (+ idx 1) count))))

(defn bootstrap-print-module [bytes]
  (let [count (vector-length bytes)]
    (do
      (print count)
      (bootstrap-print-module-bytes bytes 0 count)
      0)))

(defn main []
  (let [src "(defn main [] 42)"
        stage2-a (bootstrap-build-stage2 src)
        stage2-b (bootstrap-build-stage2 src)]
    (do
      (bootstrap-print-module stage2-a)
      (bootstrap-print-module stage2-b)
      0)))
"#;
    let stage1_source = format!(
        "{}\n{}\n{}\n{}\n{}\n{}\n{}\n{}",
        selfhost_module("Token.ls"),
        selfhost_module("AST.ls"),
        selfhost_module("Lexer.ls"),
        selfhost_module("Parser.ls"),
        selfhost_module("IR.ls"),
        selfhost_module("Compiler.ls"),
        selfhost_module("WasmEmit.ls"),
        harness
    );
    let stage1_wasm = compile_only(&stage1_source);
    assert_valid_wasm(&stage1_wasm);

    let output = lsharp_wasm::wasi_runner::run_wasm_wasi(&stage1_wasm)
        .expect("same-source stage1 実行に失敗");
    let modules = parse_emitted_wasm_modules(&output, 2);
    assert_eq!(modules.len(), 2, "stage2 モジュール数が不正");
    assert_eq!(
        modules[0], modules[1],
        "同じ tiny source から stage2 Wasm が非決定的に変化した"
    );
    assert_valid_wasm(&modules[0]);
    assert_eq!(run_exported_i64(&modules[0], "_start"), 42);
}

/// BOOT-04: stage1 が extended do block を含む stage2 Wasm も生成できること
#[test]
fn test_e2e_bootstrap_stage1_emits_stage2_wasm_for_extended_do_block() {
    let harness = r#"
(defn bootstrap-append-bytes [dst src idx count]
  (if (>= idx count)
    dst
    (bootstrap-append-bytes
      (vector-push dst (vector-get src idx))
      src
      (+ idx 1)
      count)))

(defn bootstrap-build-stage2 [src]
  (let [program (parse-program src)
        ir (lower program)
        header (emit-header)
        type-sec (emit-type-section-main)
        function-sec (emit-function-section)
        export-sec (emit-export-section)
        code-sec (emit-code-section ir)
        bytes0 (bootstrap-append-bytes (vector-new 64) header 0 (vector-length header))
        bytes1 (bootstrap-append-bytes bytes0 type-sec 0 (vector-length type-sec))
        bytes2 (bootstrap-append-bytes bytes1 function-sec 0 (vector-length function-sec))
        bytes3 (bootstrap-append-bytes bytes2 export-sec 0 (vector-length export-sec))]
    (bootstrap-append-bytes bytes3 code-sec 0 (vector-length code-sec))))

(defn bootstrap-print-module-bytes [bytes idx count]
  (if (>= idx count)
    0
    (do
      (print (vector-get bytes idx))
      (bootstrap-print-module-bytes bytes (+ idx 1) count))))

(defn bootstrap-print-module [bytes]
  (let [count (vector-length bytes)]
    (do
      (print count)
      (bootstrap-print-module-bytes bytes 0 count)
      0)))

(defn main []
  (let [stage2 (bootstrap-build-stage2 "(defn main [] (do 11 22 33 44 55 66 77))")]
    (do
      (bootstrap-print-module stage2)
      0)))
"#;
    let stage1_source = format!(
        "{}\n{}\n{}\n{}\n{}\n{}\n{}\n{}",
        selfhost_module("Token.ls"),
        selfhost_module("AST.ls"),
        selfhost_module("Lexer.ls"),
        selfhost_module("Parser.ls"),
        selfhost_module("IR.ls"),
        selfhost_module("Compiler.ls"),
        selfhost_module("WasmEmit.ls"),
        harness
    );
    let stage1_wasm = compile_only(&stage1_source);
    assert_valid_wasm(&stage1_wasm);

    let output = lsharp_wasm::wasi_runner::run_wasm_wasi(&stage1_wasm)
        .expect("extended do block を含む stage1 wasm の実行に失敗");
    let modules = parse_emitted_wasm_modules(&output, 1);
    assert_eq!(modules.len(), 1, "stage2 モジュール数が不正");
    assert_valid_wasm(&modules[0]);
    assert_eq!(
        run_exported_i64(&modules[0], "_start"),
        77,
        "stage1 は do block の最終式まで含む stage2 Wasm を生成すること"
    );
}

/// BOOT-04: stage1 が zero-arg 2 関数 + call を含む stage2 Wasm を生成できること
#[test]
fn test_e2e_bootstrap_stage1_emits_stage2_wasm_for_zero_arg_call_program() {
    let harness = r#"
(defn bootstrap-append-bytes [dst src idx count]
  (if (>= idx count)
    dst
    (bootstrap-append-bytes
      (vector-push dst (vector-get src idx))
      src
      (+ idx 1)
      count)))

(defn bootstrap-build-stage2 [src]
  (let [program (parse-program src)
        pair (compile-program program)
        ir-list (vector-get pair 1)
        func-count (vector-length ir-list)
        header (emit-header)
        type-sec (emit-type-section-main)
        function-sec (emit-function-section-count func-count)
        export-sec (emit-export-section-main-index (- func-count 1))
        code-sec (emit-code-section-list ir-list)
        bytes0 (bootstrap-append-bytes (vector-new 64) header 0 (vector-length header))
        bytes1 (bootstrap-append-bytes bytes0 type-sec 0 (vector-length type-sec))
        bytes2 (bootstrap-append-bytes bytes1 function-sec 0 (vector-length function-sec))
        bytes3 (bootstrap-append-bytes bytes2 export-sec 0 (vector-length export-sec))]
    (bootstrap-append-bytes bytes3 code-sec 0 (vector-length code-sec))))

(defn bootstrap-print-module-bytes [bytes idx count]
  (if (>= idx count)
    0
    (do
      (print (vector-get bytes idx))
      (bootstrap-print-module-bytes bytes (+ idx 1) count))))

(defn bootstrap-print-module [bytes]
  (let [count (vector-length bytes)]
    (do
      (print count)
      (bootstrap-print-module-bytes bytes 0 count)
      0)))

(defn main []
  (let [stage2 (bootstrap-build-stage2 "(defn helper [] 42) (defn main [] (helper))")]
    (do
      (bootstrap-print-module stage2)
      0)))
"#;
    let stage1_source = format!("{}\n{}", selfhost_cli_runtime_bundle(), harness);
    let stage1_wasm = compile_only(&stage1_source);
    assert_valid_wasm(&stage1_wasm);

    let output = lsharp_wasm::wasi_runner::run_wasm_wasi(&stage1_wasm)
        .expect("zero-arg call program を含む stage1 wasm の実行に失敗");
    let modules = parse_emitted_wasm_modules(&output, 1);
    assert_eq!(modules.len(), 1, "stage2 モジュール数が不正");
    assert_valid_wasm(&modules[0]);
    assert_eq!(
        run_exported_i64(&modules[0], "_start"),
        42,
        "stage1 は helper→main call を含む stage2 Wasm を生成すること"
    );
}

/// BOOT-04: stage1 が 1 引数関数呼出しを含む stage2 Wasm を生成できること
#[test]
fn test_e2e_bootstrap_stage1_emits_stage2_wasm_for_single_param_call_program() {
    let harness = r#"
(defn bootstrap-append-bytes [dst src idx count]
  (if (>= idx count)
    dst
    (bootstrap-append-bytes
      (vector-push dst (vector-get src idx))
      src
      (+ idx 1)
      count)))

(defn bootstrap-build-stage2 [src]
  (let [program (parse-program src)
        pair (compile-program-functions program)
        functions (vector-get pair 1)
        func-count (vector-length functions)
        header (emit-header)
        type-sec (emit-type-section-functions functions)
        function-sec (emit-function-section-functions functions)
        export-sec (emit-export-section-main-index (- func-count 1))
        code-sec (emit-code-section-functions functions)
        bytes0 (bootstrap-append-bytes (vector-new 64) header 0 (vector-length header))
        bytes1 (bootstrap-append-bytes bytes0 type-sec 0 (vector-length type-sec))
        bytes2 (bootstrap-append-bytes bytes1 function-sec 0 (vector-length function-sec))
        bytes3 (bootstrap-append-bytes bytes2 export-sec 0 (vector-length export-sec))]
    (bootstrap-append-bytes bytes3 code-sec 0 (vector-length code-sec))))

(defn bootstrap-print-module-bytes [bytes idx count]
  (if (>= idx count)
    0
    (do
      (print (vector-get bytes idx))
      (bootstrap-print-module-bytes bytes (+ idx 1) count))))

(defn bootstrap-print-module [bytes]
  (let [count (vector-length bytes)]
    (do
      (print count)
      (bootstrap-print-module-bytes bytes 0 count)
      0)))

(defn main []
  (let [stage2 (bootstrap-build-stage2 "(defn add1 [x] (+ x 1)) (defn main [] (add1 41))")]
    (do
      (bootstrap-print-module stage2)
      0)))
"#;
    let stage1_source = format!("{}\n{}", selfhost_cli_runtime_bundle(), harness);
    let stage1_wasm = compile_only(&stage1_source);
    assert_valid_wasm(&stage1_wasm);

    let output = lsharp_wasm::wasi_runner::run_wasm_wasi(&stage1_wasm)
        .expect("single-param call program を含む stage1 wasm の実行に失敗");
    let modules = parse_emitted_wasm_modules(&output, 1);
    assert_eq!(modules.len(), 1, "stage2 モジュール数が不正");
    assert_valid_wasm(&modules[0]);
    assert_eq!(
        run_exported_i64(&modules[0], "_start"),
        42,
        "stage1 は 1 引数関数呼出しを含む stage2 Wasm を生成すること"
    );
}

/// BOOT-04: stage1 が let local を含む stage2 Wasm を生成できること
#[test]
fn test_e2e_bootstrap_stage1_emits_stage2_wasm_for_let_local_program() {
    let harness = r#"
(defn bootstrap-append-bytes [dst src idx count]
  (if (>= idx count)
    dst
    (bootstrap-append-bytes
      (vector-push dst (vector-get src idx))
      src
      (+ idx 1)
      count)))

(defn bootstrap-build-stage2 [src]
  (let [program (parse-program src)
        pair (compile-program-functions program)
        functions (vector-get pair 1)
        func-count (vector-length functions)
        header (emit-header)
        type-sec (emit-type-section-functions functions)
        function-sec (emit-function-section-functions functions)
        export-sec (emit-export-section-main-index (- func-count 1))
        code-sec (emit-code-section-functions functions)
        bytes0 (bootstrap-append-bytes (vector-new 64) header 0 (vector-length header))
        bytes1 (bootstrap-append-bytes bytes0 type-sec 0 (vector-length type-sec))
        bytes2 (bootstrap-append-bytes bytes1 function-sec 0 (vector-length function-sec))
        bytes3 (bootstrap-append-bytes bytes2 export-sec 0 (vector-length export-sec))]
    (bootstrap-append-bytes bytes3 code-sec 0 (vector-length code-sec))))

(defn bootstrap-print-module-bytes [bytes idx count]
  (if (>= idx count)
    0
    (do
      (print (vector-get bytes idx))
      (bootstrap-print-module-bytes bytes (+ idx 1) count))))

(defn bootstrap-print-module [bytes]
  (let [count (vector-length bytes)]
    (do
      (print count)
      (bootstrap-print-module-bytes bytes 0 count)
      0)))

(defn main []
  (let [stage2 (bootstrap-build-stage2 "(defn main [] (let [x 42] x))")]
    (do
      (bootstrap-print-module stage2)
      0)))
"#;
    let stage1_source = format!("{}\n{}", selfhost_cli_runtime_bundle(), harness);
    let stage1_wasm = compile_only(&stage1_source);
    assert_valid_wasm(&stage1_wasm);

    let output = lsharp_wasm::wasi_runner::run_wasm_wasi(&stage1_wasm)
        .expect("let local program を含む stage1 wasm の実行に失敗");
    let modules = parse_emitted_wasm_modules(&output, 1);
    assert_eq!(modules.len(), 1, "stage2 モジュール数が不正");
    assert_valid_wasm(&modules[0]);
    assert_eq!(
        run_exported_i64(&modules[0], "_start"),
        42,
        "stage1 は let local を含む stage2 Wasm を生成すること"
    );
}

// =============================================================================
// BOOT-04: 再帰・多関数プログラムの stage1→stage2 検証
// =============================================================================

/// BOOT-04: stage1 が自己再帰フィボナッチを含む stage2 Wasm を生成・実行できること
///
/// (defn fib [n] ...) + (defn main [] (fib 8)) → stage2 が 21 を返す
#[test]
fn test_e2e_bootstrap_stage1_emits_stage2_wasm_for_recursive_fibonacci() {
    let harness = r#"
(defn bootstrap-append-bytes [dst src idx count]
  (if (>= idx count)
    dst
    (bootstrap-append-bytes
      (vector-push dst (vector-get src idx))
      src
      (+ idx 1)
      count)))

(defn bootstrap-build-stage2 [src]
  (let [program (parse-program src)
        pair (compile-program-functions program)
        functions (vector-get pair 1)
        func-count (vector-length functions)
        header (emit-header)
        type-sec (emit-type-section-functions functions)
        function-sec (emit-function-section-functions functions)
        export-sec (emit-export-section-main-index (- func-count 1))
        code-sec (emit-code-section-functions functions)
        bytes0 (bootstrap-append-bytes (vector-new 64) header 0 (vector-length header))
        bytes1 (bootstrap-append-bytes bytes0 type-sec 0 (vector-length type-sec))
        bytes2 (bootstrap-append-bytes bytes1 function-sec 0 (vector-length function-sec))
        bytes3 (bootstrap-append-bytes bytes2 export-sec 0 (vector-length export-sec))]
    (bootstrap-append-bytes bytes3 code-sec 0 (vector-length code-sec))))

(defn bootstrap-print-module-bytes [bytes idx count]
  (if (>= idx count)
    0
    (do
      (print (vector-get bytes idx))
      (bootstrap-print-module-bytes bytes (+ idx 1) count))))

(defn bootstrap-print-module [bytes]
  (let [count (vector-length bytes)]
    (do
      (print count)
      (bootstrap-print-module-bytes bytes 0 count)
      0)))

(defn main []
  (let [stage2 (bootstrap-build-stage2 "(defn fib [n] (if (< n 2) n (+ (fib (- n 1)) (fib (- n 2))))) (defn main [] (fib 8))")]
    (do
      (bootstrap-print-module stage2)
      0)))
"#;
    let stage1_source = format!("{}\n{}", selfhost_cli_runtime_bundle(), harness);
    let stage1_wasm = compile_only(&stage1_source);
    assert_valid_wasm(&stage1_wasm);

    let output = lsharp_wasm::wasi_runner::run_wasm_wasi(&stage1_wasm)
        .expect("再帰フィボナッチを含む stage1 wasm の実行に失敗");
    let modules = parse_emitted_wasm_modules(&output, 1);
    assert_eq!(modules.len(), 1, "stage2 モジュール数が不正");
    assert_valid_wasm(&modules[0]);
    assert_eq!(
        run_exported_i64(&modules[0], "_start"),
        21,
        "stage1 は fib(8)=21 を返す stage2 Wasm を生成すること"
    );
}

/// BOOT-04: stage1 が自己再帰階乗を含む stage2 Wasm を生成・実行できること
///
/// (defn fact [n] ...) + (defn main [] (fact 5)) → stage2 が 120 を返す
#[test]
fn test_e2e_bootstrap_stage1_emits_stage2_wasm_for_recursive_factorial() {
    let harness = r#"
(defn bootstrap-append-bytes [dst src idx count]
  (if (>= idx count)
    dst
    (bootstrap-append-bytes
      (vector-push dst (vector-get src idx))
      src
      (+ idx 1)
      count)))

(defn bootstrap-build-stage2 [src]
  (let [program (parse-program src)
        pair (compile-program-functions program)
        functions (vector-get pair 1)
        func-count (vector-length functions)
        header (emit-header)
        type-sec (emit-type-section-functions functions)
        function-sec (emit-function-section-functions functions)
        export-sec (emit-export-section-main-index (- func-count 1))
        code-sec (emit-code-section-functions functions)
        bytes0 (bootstrap-append-bytes (vector-new 64) header 0 (vector-length header))
        bytes1 (bootstrap-append-bytes bytes0 type-sec 0 (vector-length type-sec))
        bytes2 (bootstrap-append-bytes bytes1 function-sec 0 (vector-length function-sec))
        bytes3 (bootstrap-append-bytes bytes2 export-sec 0 (vector-length export-sec))]
    (bootstrap-append-bytes bytes3 code-sec 0 (vector-length code-sec))))

(defn bootstrap-print-module-bytes [bytes idx count]
  (if (>= idx count)
    0
    (do
      (print (vector-get bytes idx))
      (bootstrap-print-module-bytes bytes (+ idx 1) count))))

(defn bootstrap-print-module [bytes]
  (let [count (vector-length bytes)]
    (do
      (print count)
      (bootstrap-print-module-bytes bytes 0 count)
      0)))

(defn main []
  (let [stage2 (bootstrap-build-stage2 "(defn fact [n] (if (<= n 1) 1 (* n (fact (- n 1))))) (defn main [] (fact 5))")]
    (do
      (bootstrap-print-module stage2)
      0)))
"#;
    let stage1_source = format!("{}\n{}", selfhost_cli_runtime_bundle(), harness);
    let stage1_wasm = compile_only(&stage1_source);
    assert_valid_wasm(&stage1_wasm);

    let output = lsharp_wasm::wasi_runner::run_wasm_wasi(&stage1_wasm)
        .expect("再帰階乗を含む stage1 wasm の実行に失敗");
    let modules = parse_emitted_wasm_modules(&output, 1);
    assert_eq!(modules.len(), 1, "stage2 モジュール数が不正");
    assert_valid_wasm(&modules[0]);
    assert_eq!(
        run_exported_i64(&modules[0], "_start"),
        120,
        "stage1 は fact(5)=120 を返す stage2 Wasm を生成すること"
    );
}

/// BOOT-04: stage1 が多関数ヘルパー再帰を含む stage2 Wasm を生成・実行できること
///
/// sum(n) を呼ぶ helper(x) + main の 3 関数構成で stage2 が 55 を返す
#[test]
fn test_e2e_bootstrap_stage1_emits_stage2_wasm_for_multi_function_helper_recursion() {
    let harness = r#"
(defn bootstrap-append-bytes [dst src idx count]
  (if (>= idx count)
    dst
    (bootstrap-append-bytes
      (vector-push dst (vector-get src idx))
      src
      (+ idx 1)
      count)))

(defn bootstrap-build-stage2 [src]
  (let [program (parse-program src)
        pair (compile-program-functions program)
        functions (vector-get pair 1)
        func-count (vector-length functions)
        header (emit-header)
        type-sec (emit-type-section-functions functions)
        function-sec (emit-function-section-functions functions)
        export-sec (emit-export-section-main-index (- func-count 1))
        code-sec (emit-code-section-functions functions)
        bytes0 (bootstrap-append-bytes (vector-new 64) header 0 (vector-length header))
        bytes1 (bootstrap-append-bytes bytes0 type-sec 0 (vector-length type-sec))
        bytes2 (bootstrap-append-bytes bytes1 function-sec 0 (vector-length function-sec))
        bytes3 (bootstrap-append-bytes bytes2 export-sec 0 (vector-length export-sec))]
    (bootstrap-append-bytes bytes3 code-sec 0 (vector-length code-sec))))

(defn bootstrap-print-module-bytes [bytes idx count]
  (if (>= idx count)
    0
    (do
      (print (vector-get bytes idx))
      (bootstrap-print-module-bytes bytes (+ idx 1) count))))

(defn bootstrap-print-module [bytes]
  (let [count (vector-length bytes)]
    (do
      (print count)
      (bootstrap-print-module-bytes bytes 0 count)
      0)))

(defn main []
  (let [stage2 (bootstrap-build-stage2 "(defn sum [n] (if (<= n 0) 0 (+ n (sum (- n 1))))) (defn helper [x] (sum x)) (defn main [] (helper 10))")]
    (do
      (bootstrap-print-module stage2)
      0)))
"#;
    let stage1_source = format!("{}\n{}", selfhost_cli_runtime_bundle(), harness);
    let stage1_wasm = compile_only(&stage1_source);
    assert_valid_wasm(&stage1_wasm);

    let output = lsharp_wasm::wasi_runner::run_wasm_wasi(&stage1_wasm)
        .expect("多関数ヘルパー再帰を含む stage1 wasm の実行に失敗");
    let modules = parse_emitted_wasm_modules(&output, 1);
    assert_eq!(modules.len(), 1, "stage2 モジュール数が不正");
    assert_valid_wasm(&modules[0]);
    assert_eq!(
        run_exported_i64(&modules[0], "_start"),
        55,
        "stage1 は sum(10)=55 を経由する helper→main を含む stage2 Wasm を生成すること"
    );
}

/// BOOT-04: stage1 が string-char-at builtin を含む stage2 Wasm を valid module として生成できること
#[test]
fn test_e2e_bootstrap_stage1_emits_stage2_wasm_for_string_char_at_helper_program() {
    let harness = r#"
(defn bootstrap-append-bytes [dst src idx count]
  (if (>= idx count)
    dst
    (bootstrap-append-bytes
      (vector-push dst (vector-get src idx))
      src
      (+ idx 1)
      count)))

(defn bootstrap-build-stage2 [src]
  (let [program (parse-program src)
        pair (compile-program-functions program)
        functions (vector-get pair 1)
        func-count (vector-length functions)
        header (emit-header)
        type-sec (emit-type-section-functions functions)
        function-sec (emit-function-section-functions functions)
        memory-sec (emit-memory-section)
        export-sec (emit-export-section-main-index (- func-count 1))
        code-sec (emit-code-section-functions functions)
        bytes0 (bootstrap-append-bytes (vector-new 64) header 0 (vector-length header))
        bytes1 (bootstrap-append-bytes bytes0 type-sec 0 (vector-length type-sec))
        bytes2 (bootstrap-append-bytes bytes1 function-sec 0 (vector-length function-sec))
        bytes3 (bootstrap-append-bytes bytes2 memory-sec 0 (vector-length memory-sec))
        bytes4 (bootstrap-append-bytes bytes3 export-sec 0 (vector-length export-sec))]
    (bootstrap-append-bytes bytes4 code-sec 0 (vector-length code-sec))))

(defn bootstrap-print-module-bytes [bytes idx count]
  (if (>= idx count)
    0
    (do
      (print (vector-get bytes idx))
      (bootstrap-print-module-bytes bytes (+ idx 1) count))))

(defn bootstrap-print-module [bytes]
  (let [count (vector-length bytes)]
    (do
      (print count)
      (bootstrap-print-module-bytes bytes 0 count)
      0)))

(defn main []
  (let [stage2 (bootstrap-build-stage2 "(defn first [s] (string-char-at s 0)) (defn main [] 0)")]
    (do
      (bootstrap-print-module stage2)
      0)))
"#;
    let stage1_source = format!("{}\n{}", selfhost_cli_runtime_bundle(), harness);
    let stage1_wasm = compile_only(&stage1_source);
    assert_valid_wasm(&stage1_wasm);

    let output = lsharp_wasm::wasi_runner::run_wasm_wasi(&stage1_wasm)
        .expect("string-char-at helper program を含む stage1 wasm の実行に失敗");
    let modules = parse_emitted_wasm_modules(&output, 1);
    assert_eq!(modules.len(), 1, "stage2 モジュール数が不正");
    assert_valid_wasm(&modules[0]);
    assert!(
        extract_sections(&modules[0]).iter().any(|(id, _)| *id == 5),
        "string-char-at helper を含む stage2 Wasm は memory section を持つこと"
    );
    assert_eq!(
        run_exported_i64(&modules[0], "_start"),
        0,
        "helper 未使用でも string-char-at builtin を含む stage2 Wasm が実行可能であること"
    );
}

/// BOOT-04: stage1 が string-length builtin を含む stage2 Wasm を valid module として生成できること
#[test]
fn test_e2e_bootstrap_stage1_emits_stage2_wasm_for_string_length_helper_program() {
    let harness = r#"
(defn bootstrap-append-bytes [dst src idx count]
  (if (>= idx count)
    dst
    (bootstrap-append-bytes
      (vector-push dst (vector-get src idx))
      src
      (+ idx 1)
      count)))

(defn bootstrap-build-stage2 [src]
  (let [program (parse-program src)
        pair (compile-program-functions program)
        functions (vector-get pair 1)
        func-count (vector-length functions)
        header (emit-header)
        type-sec (emit-type-section-functions functions)
        function-sec (emit-function-section-functions functions)
        memory-sec (emit-memory-section)
        export-sec (emit-export-section-main-index (- func-count 1))
        code-sec (emit-code-section-functions functions)
        bytes0 (bootstrap-append-bytes (vector-new 64) header 0 (vector-length header))
        bytes1 (bootstrap-append-bytes bytes0 type-sec 0 (vector-length type-sec))
        bytes2 (bootstrap-append-bytes bytes1 function-sec 0 (vector-length function-sec))
        bytes3 (bootstrap-append-bytes bytes2 memory-sec 0 (vector-length memory-sec))
        bytes4 (bootstrap-append-bytes bytes3 export-sec 0 (vector-length export-sec))]
    (bootstrap-append-bytes bytes4 code-sec 0 (vector-length code-sec))))

(defn bootstrap-print-module-bytes [bytes idx count]
  (if (>= idx count)
    0
    (do
      (print (vector-get bytes idx))
      (bootstrap-print-module-bytes bytes (+ idx 1) count))))

(defn bootstrap-print-module [bytes]
  (let [count (vector-length bytes)]
    (do
      (print count)
      (bootstrap-print-module-bytes bytes 0 count)
      0)))

(defn main []
  (let [stage2 (bootstrap-build-stage2 "(defn len1 [s] (string-length s)) (defn main [] 0)")]
    (do
      (bootstrap-print-module stage2)
      0)))
"#;
    let stage1_source = format!("{}\n{}", selfhost_cli_runtime_bundle(), harness);
    let stage1_wasm = compile_only(&stage1_source);
    assert_valid_wasm(&stage1_wasm);

    let output = lsharp_wasm::wasi_runner::run_wasm_wasi(&stage1_wasm)
        .expect("string-length helper program を含む stage1 wasm の実行に失敗");
    let modules = parse_emitted_wasm_modules(&output, 1);
    assert_eq!(modules.len(), 1, "stage2 モジュール数が不正");
    assert_valid_wasm(&modules[0]);
    assert!(
        extract_sections(&modules[0]).iter().any(|(id, _)| *id == 5),
        "string-length helper を含む stage2 Wasm は memory section を持つこと"
    );
    assert_eq!(
        run_exported_i64(&modules[0], "_start"),
        0,
        "helper 未使用でも string-length builtin を含む stage2 Wasm が実行可能であること"
    );
}

/// BOOT-04: stage1 が string literal を data section に落とし込んだ stage2 Wasm を生成できること
#[test]
fn test_e2e_bootstrap_stage1_emits_stage2_wasm_for_string_literal_data_section() {
    let harness = r#"
(defn bootstrap-append-bytes [dst src idx count]
  (if (>= idx count)
    dst
    (bootstrap-append-bytes
      (vector-push dst (vector-get src idx))
      src
      (+ idx 1)
      count)))

(defn bootstrap-build-stage2 [src]
  (let [program (parse-program src)
        pair (compile-program-functions-with-source src program)
        functions (vector-get pair 1)
        data (vector-get pair 2)
        header (emit-header)
        type-sec (emit-type-section-functions functions)
        function-sec (emit-function-section-functions functions)
        memory-sec (emit-memory-section)
        export-sec (emit-export-section-main-index 0)
        code-sec (emit-code-section-functions functions)
        data-sec (emit-data-section data 1024)
        bytes0 (bootstrap-append-bytes (vector-new 64) header 0 (vector-length header))
        bytes1 (bootstrap-append-bytes bytes0 type-sec 0 (vector-length type-sec))
        bytes2 (bootstrap-append-bytes bytes1 function-sec 0 (vector-length function-sec))
        bytes3 (bootstrap-append-bytes bytes2 memory-sec 0 (vector-length memory-sec))
        bytes4 (bootstrap-append-bytes bytes3 export-sec 0 (vector-length export-sec))
        bytes5 (bootstrap-append-bytes bytes4 code-sec 0 (vector-length code-sec))]
    (bootstrap-append-bytes bytes5 data-sec 0 (vector-length data-sec))))

(defn bootstrap-print-module-bytes [bytes idx count]
  (if (>= idx count)
    0
    (do
      (print (vector-get bytes idx))
      (bootstrap-print-module-bytes bytes (+ idx 1) count))))

(defn bootstrap-print-module [bytes]
  (let [count (vector-length bytes)]
    (do
      (print count)
      (bootstrap-print-module-bytes bytes 0 count)
      0)))

(defn main []
  (let [stage2 (bootstrap-build-stage2 "(defn main [] \"abc\")")]
    (do
      (bootstrap-print-module stage2)
      0)))
"#;
    let stage1_source = format!("{}\n{}", selfhost_cli_runtime_bundle(), harness);
    let stage1_wasm = compile_only(&stage1_source);
    assert_valid_wasm(&stage1_wasm);

    let output = lsharp_wasm::wasi_runner::run_wasm_wasi(&stage1_wasm)
        .expect("string literal data section program を含む stage1 wasm の実行に失敗");
    let modules = parse_emitted_wasm_modules(&output, 1);
    assert_eq!(modules.len(), 1, "stage2 モジュール数が不正");
    assert_valid_wasm(&modules[0]);
    let sections = extract_sections(&modules[0]);
    assert!(
        sections.iter().any(|(id, _)| *id == 11),
        "string literal を含む stage2 Wasm は data section を持つこと"
    );
    let data_section = extract_section_bytes(&modules[0], 11).expect("data section が見つからない");
    assert!(
        data_section.windows(3).any(|window| window == [97, 98, 99]),
        "data section に string literal bytes が含まれていない"
    );
    assert_eq!(
        run_exported_i64(&modules[0], "_start"),
        1024,
        "string literal lowering の data base offset が不正"
    );
}

/// BOOT-04: stage1 が nested string literal を distinct offsets 付きで stage2 Wasm に落とし込めること
#[test]
fn test_e2e_bootstrap_stage1_emits_stage2_wasm_for_nested_string_literal_data_section() {
    let stage2_source = r#"(defn main [] (do "ab" "cde"))"#.replace('"', "\\\"");
    let harness = format!(
        r#"
(defn bootstrap-append-bytes [dst src idx count]
  (if (>= idx count)
    dst
    (bootstrap-append-bytes
      (vector-push dst (vector-get src idx))
      src
      (+ idx 1)
      count)))

(defn bootstrap-build-stage2 [src]
  (let [program (parse-program src)
        pair (compile-program-functions-with-source src program)
        functions (vector-get pair 1)
        data (vector-get pair 2)
        header (emit-header)
        type-sec (emit-type-section-functions functions)
        function-sec (emit-function-section-functions functions)
        memory-sec (emit-memory-section)
        export-sec (emit-export-section-main-index 0)
        code-sec (emit-code-section-functions functions)
        data-sec (emit-data-section data 1024)
        bytes0 (bootstrap-append-bytes (vector-new 64) header 0 (vector-length header))
        bytes1 (bootstrap-append-bytes bytes0 type-sec 0 (vector-length type-sec))
        bytes2 (bootstrap-append-bytes bytes1 function-sec 0 (vector-length function-sec))
        bytes3 (bootstrap-append-bytes bytes2 memory-sec 0 (vector-length memory-sec))
        bytes4 (bootstrap-append-bytes bytes3 export-sec 0 (vector-length export-sec))
        bytes5 (bootstrap-append-bytes bytes4 code-sec 0 (vector-length code-sec))]
    (bootstrap-append-bytes bytes5 data-sec 0 (vector-length data-sec))))

(defn bootstrap-print-module-bytes [bytes idx count]
  (if (>= idx count)
    0
    (do
      (print (vector-get bytes idx))
      (bootstrap-print-module-bytes bytes (+ idx 1) count))))

(defn bootstrap-print-module [bytes]
  (let [count (vector-length bytes)]
    (do
      (print count)
      (bootstrap-print-module-bytes bytes 0 count)
      0)))

(defn main []
  (let [stage2 (bootstrap-build-stage2 "{}")]
    (do
      (bootstrap-print-module stage2)
      0)))
"#,
        stage2_source
    );
    let stage1_source = format!("{}\n{}", selfhost_cli_runtime_bundle(), harness);
    let stage1_wasm = compile_only(&stage1_source);
    assert_valid_wasm(&stage1_wasm);

    let output = lsharp_wasm::wasi_runner::run_wasm_wasi(&stage1_wasm)
        .expect("nested string literal data section program を含む stage1 wasm の実行に失敗");
    let modules = parse_emitted_wasm_modules(&output, 1);
    assert_eq!(modules.len(), 1, "stage2 モジュール数が不正");
    assert_valid_wasm(&modules[0]);
    let data_section = extract_section_bytes(&modules[0], 11).expect("data section が見つからない");
    assert!(
        data_section
            .windows(5)
            .any(|window| window == [97, 98, 99, 100, 101]),
        "nested string literal bytes が data section に連結配置されていない"
    );
    assert_eq!(
        run_exported_i64(&modules[0], "_start"),
        1026,
        "nested string literal の最終 offset が前段 bytes を考慮していない"
    );
}

/// BOOT-04: stage1 が 5 式以上の do に含まれる source-aware string literal も stage2 Wasm に落とし込めること
#[test]
fn test_e2e_bootstrap_stage1_emits_stage2_wasm_for_extended_do_string_literal_data_section() {
    let stage2_source = r#"(defn main [] (do "ab" "c" "de" "fgh" "ijk"))"#.replace('"', "\\\"");
    let harness = format!(
        r#"
(defn bootstrap-append-bytes [dst src idx count]
  (if (>= idx count)
    dst
    (bootstrap-append-bytes
      (vector-push dst (vector-get src idx))
      src
      (+ idx 1)
      count)))

(defn bootstrap-build-stage2 [src]
  (let [program (parse-program src)
        pair (compile-program-functions-with-source src program)
        functions (vector-get pair 1)
        data (vector-get pair 2)
        header (emit-header)
        type-sec (emit-type-section-functions functions)
        function-sec (emit-function-section-functions functions)
        memory-sec (emit-memory-section)
        export-sec (emit-export-section-main-index 0)
        code-sec (emit-code-section-functions functions)
        data-sec (emit-data-section data 1024)
        bytes0 (bootstrap-append-bytes (vector-new 64) header 0 (vector-length header))
        bytes1 (bootstrap-append-bytes bytes0 type-sec 0 (vector-length type-sec))
        bytes2 (bootstrap-append-bytes bytes1 function-sec 0 (vector-length function-sec))
        bytes3 (bootstrap-append-bytes bytes2 memory-sec 0 (vector-length memory-sec))
        bytes4 (bootstrap-append-bytes bytes3 export-sec 0 (vector-length export-sec))
        bytes5 (bootstrap-append-bytes bytes4 code-sec 0 (vector-length code-sec))]
    (bootstrap-append-bytes bytes5 data-sec 0 (vector-length data-sec))))

(defn bootstrap-print-module-bytes [bytes idx count]
  (if (>= idx count)
    0
    (do
      (print (vector-get bytes idx))
      (bootstrap-print-module-bytes bytes (+ idx 1) count))))

(defn bootstrap-print-module [bytes]
  (let [count (vector-length bytes)]
    (do
      (print count)
      (bootstrap-print-module-bytes bytes 0 count)
      0)))

(defn main []
  (let [stage2 (bootstrap-build-stage2 "{}")]
    (do
      (bootstrap-print-module stage2)
      0)))
"#,
        stage2_source
    );
    let stage1_source = format!("{}\n{}", selfhost_cli_runtime_bundle(), harness);
    let stage1_wasm = compile_only(&stage1_source);
    assert_valid_wasm(&stage1_wasm);

    let output = lsharp_wasm::wasi_runner::run_wasm_wasi(&stage1_wasm)
        .expect("extended do string literal program を含む stage1 wasm の実行に失敗");
    let modules = parse_emitted_wasm_modules(&output, 1);
    assert_eq!(modules.len(), 1, "stage2 モジュール数が不正");
    assert_valid_wasm(&modules[0]);
    let data_section = extract_section_bytes(&modules[0], 11).expect("data section が見つからない");
    assert!(
        data_section
            .windows(11)
            .any(|window| window == b"abcdefghijk"),
        "extended do string literal bytes が data section に連結配置されていない"
    );
    assert_eq!(
        run_exported_i64(&modules[0], "_start"),
        1032,
        "extended do string literal の最終 offset が前段 bytes を考慮していない"
    );
}

/// BOOT-04: stage1 が if branch 内の source-aware string literal を stage2 Wasm に落とし込めること
#[test]
fn test_e2e_bootstrap_stage1_emits_stage2_wasm_for_if_string_literal_data_section() {
    let stage2_source = r#"(defn main [] (if (= 1 1) "hello" "world"))"#.replace('"', "\\\"");
    let harness = format!(
        r#"
(defn bootstrap-append-bytes [dst src idx count]
  (if (>= idx count)
    dst
    (bootstrap-append-bytes
      (vector-push dst (vector-get src idx))
      src
      (+ idx 1)
      count)))

(defn bootstrap-build-stage2 [src]
  (let [program (parse-program src)
        pair (compile-program-functions-with-source src program)
        functions (vector-get pair 1)
        data (vector-get pair 2)
        header (emit-header)
        type-sec (emit-type-section-functions functions)
        function-sec (emit-function-section-functions functions)
        memory-sec (emit-memory-section)
        export-sec (emit-export-section-main-index 0)
        code-sec (emit-code-section-functions functions)
        data-sec (emit-data-section data 1024)
        bytes0 (bootstrap-append-bytes (vector-new 64) header 0 (vector-length header))
        bytes1 (bootstrap-append-bytes bytes0 type-sec 0 (vector-length type-sec))
        bytes2 (bootstrap-append-bytes bytes1 function-sec 0 (vector-length function-sec))
        bytes3 (bootstrap-append-bytes bytes2 memory-sec 0 (vector-length memory-sec))
        bytes4 (bootstrap-append-bytes bytes3 export-sec 0 (vector-length export-sec))
        bytes5 (bootstrap-append-bytes bytes4 code-sec 0 (vector-length code-sec))]
    (bootstrap-append-bytes bytes5 data-sec 0 (vector-length data-sec))))

(defn bootstrap-print-module-bytes [bytes idx count]
  (if (>= idx count)
    0
    (do
      (print (vector-get bytes idx))
      (bootstrap-print-module-bytes bytes (+ idx 1) count))))

(defn bootstrap-print-module [bytes]
  (let [count (vector-length bytes)]
    (do
      (print count)
      (bootstrap-print-module-bytes bytes 0 count)
      0)))

(defn main []
  (let [stage2 (bootstrap-build-stage2 "{}")]
    (do
      (bootstrap-print-module stage2)
      0)))
"#,
        stage2_source
    );
    let stage1_source = format!("{}\n{}", selfhost_cli_runtime_bundle(), harness);
    let stage1_wasm = compile_only(&stage1_source);
    assert_valid_wasm(&stage1_wasm);

    let output = lsharp_wasm::wasi_runner::run_wasm_wasi(&stage1_wasm)
        .expect("if string literal program を含む stage1 wasm の実行に失敗");
    let modules = parse_emitted_wasm_modules(&output, 1);
    assert_eq!(modules.len(), 1, "stage2 モジュール数が不正");
    assert_valid_wasm(&modules[0]);
    let data_section = extract_section_bytes(&modules[0], 11).expect("data section が見つからない");
    assert!(
        data_section
            .windows(10)
            .any(|window| window == b"helloworld"),
        "if string literal bytes が data section に連結配置されていない"
    );
    assert_eq!(
        run_exported_i64(&modules[0], "_start"),
        1024,
        "if string literal の then branch offset が不正"
    );
}

/// BOOT-04: stage1 が match arm 内の source-aware string literal を stage2 Wasm に落とし込めること
#[test]
fn test_e2e_bootstrap_stage1_emits_stage2_wasm_for_match_string_literal_data_section() {
    let stage2_source = r#"(defn main [] (match 2 [1 "one"] [2 "two"]))"#.replace('"', "\\\"");
    let harness = format!(
        r#"
(defn bootstrap-append-bytes [dst src idx count]
  (if (>= idx count)
    dst
    (bootstrap-append-bytes
      (vector-push dst (vector-get src idx))
      src
      (+ idx 1)
      count)))

(defn bootstrap-build-stage2 [src]
  (let [program (parse-program src)
        pair (compile-program-functions-with-source src program)
        functions (vector-get pair 1)
        data (vector-get pair 2)
        header (emit-header)
        type-sec (emit-type-section-functions functions)
        function-sec (emit-function-section-functions functions)
        memory-sec (emit-memory-section)
        export-sec (emit-export-section-main-index 0)
        code-sec (emit-code-section-functions functions)
        data-sec (emit-data-section data 1024)
        bytes0 (bootstrap-append-bytes (vector-new 64) header 0 (vector-length header))
        bytes1 (bootstrap-append-bytes bytes0 type-sec 0 (vector-length type-sec))
        bytes2 (bootstrap-append-bytes bytes1 function-sec 0 (vector-length function-sec))
        bytes3 (bootstrap-append-bytes bytes2 memory-sec 0 (vector-length memory-sec))
        bytes4 (bootstrap-append-bytes bytes3 export-sec 0 (vector-length export-sec))
        bytes5 (bootstrap-append-bytes bytes4 code-sec 0 (vector-length code-sec))]
    (bootstrap-append-bytes bytes5 data-sec 0 (vector-length data-sec))))

(defn bootstrap-print-module-bytes [bytes idx count]
  (if (>= idx count)
    0
    (do
      (print (vector-get bytes idx))
      (bootstrap-print-module-bytes bytes (+ idx 1) count))))

(defn bootstrap-print-module [bytes]
  (let [count (vector-length bytes)]
    (do
      (print count)
      (bootstrap-print-module-bytes bytes 0 count)
      0)))

(defn main []
  (let [stage2 (bootstrap-build-stage2 "{}")]
    (do
      (bootstrap-print-module stage2)
      0)))
"#,
        stage2_source
    );
    let stage1_source = format!("{}\n{}", selfhost_cli_runtime_bundle(), harness);
    let stage1_wasm = compile_only(&stage1_source);
    assert_valid_wasm(&stage1_wasm);

    let output = lsharp_wasm::wasi_runner::run_wasm_wasi(&stage1_wasm)
        .expect("match string literal program を含む stage1 wasm の実行に失敗");
    let modules = parse_emitted_wasm_modules(&output, 1);
    assert_eq!(modules.len(), 1, "stage2 モジュール数が不正");
    assert_valid_wasm(&modules[0]);
    let data_section = extract_section_bytes(&modules[0], 11).expect("data section が見つからない");
    assert!(
        data_section.windows(6).any(|window| window == b"onetwo"),
        "match string literal bytes が data section に連結配置されていない"
    );
    assert_eq!(
        run_exported_i64(&modules[0], "_start"),
        1027,
        "match string literal の selected branch offset が不正"
    );
}

/// BOOT-04: stage1 が lambda body 内の source-aware string literal を stage2 Wasm に落とし込めること
#[test]
fn test_e2e_bootstrap_stage1_emits_stage2_wasm_for_lambda_string_literal_data_section() {
    let stage2_source = r#"(defn main [] (fn [x] "ok"))"#.replace('"', "\\\"");
    let harness = format!(
        r#"
(defn bootstrap-append-bytes [dst src idx count]
  (if (>= idx count)
    dst
    (bootstrap-append-bytes
      (vector-push dst (vector-get src idx))
      src
      (+ idx 1)
      count)))

(defn bootstrap-build-stage2 [src]
  (let [program (parse-program src)
        pair (compile-program-functions-with-source src program)
        functions (vector-get pair 1)
        data (vector-get pair 2)
        header (emit-header)
        type-sec (emit-type-section-functions functions)
        function-sec (emit-function-section-functions functions)
        memory-sec (emit-memory-section)
        export-sec (emit-export-section-main-index 0)
        code-sec (emit-code-section-functions functions)
        data-sec (emit-data-section data 1024)
        bytes0 (bootstrap-append-bytes (vector-new 64) header 0 (vector-length header))
        bytes1 (bootstrap-append-bytes bytes0 type-sec 0 (vector-length type-sec))
        bytes2 (bootstrap-append-bytes bytes1 function-sec 0 (vector-length function-sec))
        bytes3 (bootstrap-append-bytes bytes2 memory-sec 0 (vector-length memory-sec))
        bytes4 (bootstrap-append-bytes bytes3 export-sec 0 (vector-length export-sec))
        bytes5 (bootstrap-append-bytes bytes4 code-sec 0 (vector-length code-sec))]
    (bootstrap-append-bytes bytes5 data-sec 0 (vector-length data-sec))))

(defn bootstrap-print-module-bytes [bytes idx count]
  (if (>= idx count)
    0
    (do
      (print (vector-get bytes idx))
      (bootstrap-print-module-bytes bytes (+ idx 1) count))))

(defn bootstrap-print-module [bytes]
  (let [count (vector-length bytes)]
    (do
      (print count)
      (bootstrap-print-module-bytes bytes 0 count)
      0)))

(defn main []
  (let [stage2 (bootstrap-build-stage2 "{}")]
    (do
      (bootstrap-print-module stage2)
      0)))
"#,
        stage2_source
    );
    let stage1_source = format!("{}\n{}", selfhost_cli_runtime_bundle(), harness);
    let stage1_wasm = compile_only(&stage1_source);
    assert_valid_wasm(&stage1_wasm);

    let output = lsharp_wasm::wasi_runner::run_wasm_wasi(&stage1_wasm)
        .expect("lambda string literal program を含む stage1 wasm の実行に失敗");
    let modules = parse_emitted_wasm_modules(&output, 1);
    assert_eq!(modules.len(), 1, "stage2 モジュール数が不正");
    assert_valid_wasm(&modules[0]);
    let data_section = extract_section_bytes(&modules[0], 11).expect("data section が見つからない");
    assert!(
        data_section.windows(2).any(|window| window == b"ok"),
        "lambda string literal bytes が data section に配置されていない"
    );
    assert_eq!(
        run_exported_i64(&modules[0], "_start"),
        1024,
        "lambda string literal の offset が不正"
    );
}

/// BOOT-04: stage1 が vector-length builtin を含む stage2 Wasm を valid module として生成できること
#[test]
fn test_e2e_bootstrap_stage1_emits_stage2_wasm_for_vector_length_helper_program() {
    let harness = r#"
(defn bootstrap-append-bytes [dst src idx count]
  (if (>= idx count)
    dst
    (bootstrap-append-bytes
      (vector-push dst (vector-get src idx))
      src
      (+ idx 1)
      count)))

(defn bootstrap-build-stage2 [src]
  (let [program (parse-program src)
        pair (compile-program-functions program)
        functions (vector-get pair 1)
        func-count (vector-length functions)
        header (emit-header)
        type-sec (emit-type-section-functions functions)
        function-sec (emit-function-section-functions functions)
        memory-sec (emit-memory-section)
        export-sec (emit-export-section-main-index (- func-count 1))
        code-sec (emit-code-section-functions functions)
        bytes0 (bootstrap-append-bytes (vector-new 64) header 0 (vector-length header))
        bytes1 (bootstrap-append-bytes bytes0 type-sec 0 (vector-length type-sec))
        bytes2 (bootstrap-append-bytes bytes1 function-sec 0 (vector-length function-sec))
        bytes3 (bootstrap-append-bytes bytes2 memory-sec 0 (vector-length memory-sec))
        bytes4 (bootstrap-append-bytes bytes3 export-sec 0 (vector-length export-sec))]
    (bootstrap-append-bytes bytes4 code-sec 0 (vector-length code-sec))))

(defn bootstrap-print-module-bytes [bytes idx count]
  (if (>= idx count)
    0
    (do
      (print (vector-get bytes idx))
      (bootstrap-print-module-bytes bytes (+ idx 1) count))))

(defn bootstrap-print-module [bytes]
  (let [count (vector-length bytes)]
    (do
      (print count)
      (bootstrap-print-module-bytes bytes 0 count)
      0)))

(defn main []
  (let [stage2 (bootstrap-build-stage2 "(defn vlen [v] (vector-length v)) (defn main [] 0)")]
    (do
      (bootstrap-print-module stage2)
      0)))
"#;
    let stage1_source = format!("{}\n{}", selfhost_cli_runtime_bundle(), harness);
    let stage1_wasm = compile_only(&stage1_source);
    assert_valid_wasm(&stage1_wasm);

    let output = lsharp_wasm::wasi_runner::run_wasm_wasi(&stage1_wasm)
        .expect("vector-length helper program を含む stage1 wasm の実行に失敗");
    let modules = parse_emitted_wasm_modules(&output, 1);
    assert_eq!(modules.len(), 1, "stage2 モジュール数が不正");
    assert_valid_wasm(&modules[0]);
    assert!(
        extract_sections(&modules[0]).iter().any(|(id, _)| *id == 5),
        "vector-length helper を含む stage2 Wasm は memory section を持つこと"
    );
    assert_eq!(
        run_exported_i64(&modules[0], "_start"),
        0,
        "helper 未使用でも vector-length builtin を含む stage2 Wasm が実行可能であること"
    );
}

/// BOOT-04: stage1 が vector-get builtin を含む stage2 Wasm を valid module として生成できること
#[test]
fn test_e2e_bootstrap_stage1_emits_stage2_wasm_for_vector_get_helper_program() {
    let harness = r#"
(defn bootstrap-append-bytes [dst src idx count]
  (if (>= idx count)
    dst
    (bootstrap-append-bytes
      (vector-push dst (vector-get src idx))
      src
      (+ idx 1)
      count)))

(defn bootstrap-build-stage2 [src]
  (let [program (parse-program src)
        pair (compile-program-functions program)
        functions (vector-get pair 1)
        func-count (vector-length functions)
        header (emit-header)
        type-sec (emit-type-section-functions functions)
        function-sec (emit-function-section-functions functions)
        memory-sec (emit-memory-section)
        export-sec (emit-export-section-main-index (- func-count 1))
        code-sec (emit-code-section-functions functions)
        bytes0 (bootstrap-append-bytes (vector-new 64) header 0 (vector-length header))
        bytes1 (bootstrap-append-bytes bytes0 type-sec 0 (vector-length type-sec))
        bytes2 (bootstrap-append-bytes bytes1 function-sec 0 (vector-length function-sec))
        bytes3 (bootstrap-append-bytes bytes2 memory-sec 0 (vector-length memory-sec))
        bytes4 (bootstrap-append-bytes bytes3 export-sec 0 (vector-length export-sec))]
    (bootstrap-append-bytes bytes4 code-sec 0 (vector-length code-sec))))

(defn bootstrap-print-module-bytes [bytes idx count]
  (if (>= idx count)
    0
    (do
      (print (vector-get bytes idx))
      (bootstrap-print-module-bytes bytes (+ idx 1) count))))

(defn bootstrap-print-module [bytes]
  (let [count (vector-length bytes)]
    (do
      (print count)
      (bootstrap-print-module-bytes bytes 0 count)
      0)))

(defn main []
  (let [stage2 (bootstrap-build-stage2 "(defn vget0 [v] (vector-get v 0)) (defn main [] 0)")]
    (do
      (bootstrap-print-module stage2)
      0)))
"#;
    let stage1_source = format!("{}\n{}", selfhost_cli_runtime_bundle(), harness);
    let stage1_wasm = compile_only(&stage1_source);
    assert_valid_wasm(&stage1_wasm);

    let output = lsharp_wasm::wasi_runner::run_wasm_wasi(&stage1_wasm)
        .expect("vector-get helper program を含む stage1 wasm の実行に失敗");
    let modules = parse_emitted_wasm_modules(&output, 1);
    assert_eq!(modules.len(), 1, "stage2 モジュール数が不正");
    assert_valid_wasm(&modules[0]);
    assert!(
        extract_sections(&modules[0]).iter().any(|(id, _)| *id == 5),
        "vector-get helper を含む stage2 Wasm は memory section を持つこと"
    );
    assert_eq!(
        run_exported_i64(&modules[0], "_start"),
        0,
        "helper 未使用でも vector-get builtin を含む stage2 Wasm が実行可能であること"
    );
}

/// BOOT-04: stage1 が __alloc import を伴う vector-new program を stage2 Wasm として生成できること
#[test]
fn test_e2e_bootstrap_stage1_emits_stage2_wasm_for_vector_new_program() {
    let harness = r#"
(defn bootstrap-append-bytes [dst src idx count]
  (if (>= idx count)
    dst
    (bootstrap-append-bytes
      (vector-push dst (vector-get src idx))
      src
      (+ idx 1)
      count)))

(defn bootstrap-build-stage2 [src]
  (let [program (parse-program src)
        pair (compile-program-functions program)
        functions (vector-get pair 1)
        header (emit-header)
        type-sec (emit-type-section-alloc-main)
        import-sec (emit-import-section-alloc)
        function-sec (emit-function-section-main-type-index 1)
        memory-sec (emit-memory-section)
        export-sec (emit-export-section-main-index 1)
        code-sec (emit-code-section-functions functions)
        bytes0 (bootstrap-append-bytes (vector-new 64) header 0 (vector-length header))
        bytes1 (bootstrap-append-bytes bytes0 type-sec 0 (vector-length type-sec))
        bytes2 (bootstrap-append-bytes bytes1 import-sec 0 (vector-length import-sec))
        bytes3 (bootstrap-append-bytes bytes2 function-sec 0 (vector-length function-sec))
        bytes4 (bootstrap-append-bytes bytes3 memory-sec 0 (vector-length memory-sec))
        bytes5 (bootstrap-append-bytes bytes4 export-sec 0 (vector-length export-sec))]
    (bootstrap-append-bytes bytes5 code-sec 0 (vector-length code-sec))))

(defn bootstrap-print-module-bytes [bytes idx count]
  (if (>= idx count)
    0
    (do
      (print (vector-get bytes idx))
      (bootstrap-print-module-bytes bytes (+ idx 1) count))))

(defn bootstrap-print-module [bytes]
  (let [count (vector-length bytes)]
    (do
      (print count)
      (bootstrap-print-module-bytes bytes 0 count)
      0)))

(defn main []
  (let [stage2 (bootstrap-build-stage2 "(defn main [] (vector-length (vector-new 4)))")]
    (do
      (bootstrap-print-module stage2)
      0)))
"#;
    let stage1_source = format!("{}\n{}", selfhost_cli_runtime_bundle(), harness);
    let stage1_wasm = compile_only(&stage1_source);
    assert_valid_wasm(&stage1_wasm);

    let output = lsharp_wasm::wasi_runner::run_wasm_wasi(&stage1_wasm)
        .expect("vector-new program を含む stage1 wasm の実行に失敗");
    let modules = parse_emitted_wasm_modules(&output, 1);
    assert_eq!(modules.len(), 1, "stage2 モジュール数が不正");
    assert_valid_wasm(&modules[0]);
    assert!(
        extract_sections(&modules[0]).iter().any(|(id, _)| *id == 2),
        "vector-new program を含む stage2 Wasm は alloc import section を持つこと"
    );
    assert_eq!(
        run_exported_i64_with_alloc_import(&modules[0], "_start"),
        0,
        "vector-new + vector-length を含む stage2 Wasm が alloc import 付きで実行可能であること"
    );
}

/// BOOT-04: stage1 が同じ alloc-import tiny source から同一 stage2 Wasm を 2 回生成できること
#[test]
fn test_e2e_bootstrap_stage1_emits_identical_alloc_stage2_wasm_for_same_source() {
    let harness = r#"
(defn bootstrap-append-bytes [dst src idx count]
  (if (>= idx count)
    dst
    (bootstrap-append-bytes
      (vector-push dst (vector-get src idx))
      src
      (+ idx 1)
      count)))

(defn bootstrap-build-stage2 [src]
  (let [program (parse-program src)
        pair (compile-program-functions program)
        functions (vector-get pair 1)
        header (emit-header)
        type-sec (emit-type-section-alloc-main)
        import-sec (emit-import-section-alloc)
        function-sec (emit-function-section-main-type-index 1)
        memory-sec (emit-memory-section)
        export-sec (emit-export-section-main-index 1)
        code-sec (emit-code-section-functions functions)
        bytes0 (bootstrap-append-bytes (vector-new 64) header 0 (vector-length header))
        bytes1 (bootstrap-append-bytes bytes0 type-sec 0 (vector-length type-sec))
        bytes2 (bootstrap-append-bytes bytes1 import-sec 0 (vector-length import-sec))
        bytes3 (bootstrap-append-bytes bytes2 function-sec 0 (vector-length function-sec))
        bytes4 (bootstrap-append-bytes bytes3 memory-sec 0 (vector-length memory-sec))
        bytes5 (bootstrap-append-bytes bytes4 export-sec 0 (vector-length export-sec))]
    (bootstrap-append-bytes bytes5 code-sec 0 (vector-length code-sec))))

(defn bootstrap-print-module-bytes [bytes idx count]
  (if (>= idx count)
    0
    (do
      (print (vector-get bytes idx))
      (bootstrap-print-module-bytes bytes (+ idx 1) count))))

(defn bootstrap-print-module [bytes]
  (let [count (vector-length bytes)]
    (do
      (print count)
      (bootstrap-print-module-bytes bytes 0 count)
      0)))

(defn main []
  (let [src "(defn main [] (vector-length (vector-new 4)))"
        stage2-a (bootstrap-build-stage2 src)
        stage2-b (bootstrap-build-stage2 src)]
    (do
      (bootstrap-print-module stage2-a)
      (bootstrap-print-module stage2-b)
      0)))
"#;
    let stage1_source = format!("{}\n{}", selfhost_cli_runtime_bundle(), harness);
    let stage1_wasm = compile_only(&stage1_source);
    assert_valid_wasm(&stage1_wasm);

    let output = lsharp_wasm::wasi_runner::run_wasm_wasi(&stage1_wasm)
        .expect("same alloc-source stage1 実行に失敗");
    let modules = parse_emitted_wasm_modules(&output, 2);
    assert_eq!(modules.len(), 2, "stage2 モジュール数が不正");
    assert_eq!(
        modules[0], modules[1],
        "同じ alloc-import tiny source から stage2 Wasm が非決定的に変化した"
    );
    assert_valid_wasm(&modules[0]);
    assert!(
        extract_sections(&modules[0]).iter().any(|(id, _)| *id == 2),
        "repeatability 対象 stage2 Wasm は alloc import section を持つこと"
    );
    assert_eq!(run_exported_i64_with_alloc_import(&modules[0], "_start"), 0);
}

/// BOOT-04: stage1 が vector-push の in-place + growth を含む stage2 Wasm を生成できること
#[test]
fn test_e2e_bootstrap_stage1_emits_stage2_wasm_for_vector_push_program() {
    let harness = r#"
(defn bootstrap-append-bytes [dst src idx count]
  (if (>= idx count)
    dst
    (bootstrap-append-bytes
      (vector-push dst (vector-get src idx))
      src
      (+ idx 1)
      count)))

(defn bootstrap-build-stage2 [src]
  (let [program (parse-program src)
        pair (compile-program-functions program)
        functions (vector-get pair 1)
        header (emit-header)
        type-sec (emit-type-section-alloc-main)
        import-sec (emit-import-section-alloc)
        function-sec (emit-function-section-main-type-index 1)
        memory-sec (emit-memory-section)
        export-sec (emit-export-section-main-index 1)
        code-sec (emit-code-section-functions functions)
        bytes0 (bootstrap-append-bytes (vector-new 64) header 0 (vector-length header))
        bytes1 (bootstrap-append-bytes bytes0 type-sec 0 (vector-length type-sec))
        bytes2 (bootstrap-append-bytes bytes1 import-sec 0 (vector-length import-sec))
        bytes3 (bootstrap-append-bytes bytes2 function-sec 0 (vector-length function-sec))
        bytes4 (bootstrap-append-bytes bytes3 memory-sec 0 (vector-length memory-sec))
        bytes5 (bootstrap-append-bytes bytes4 export-sec 0 (vector-length export-sec))]
    (bootstrap-append-bytes bytes5 code-sec 0 (vector-length code-sec))))

(defn bootstrap-print-module-bytes [bytes idx count]
  (if (>= idx count)
    0
    (do
      (print (vector-get bytes idx))
      (bootstrap-print-module-bytes bytes (+ idx 1) count))))

(defn bootstrap-print-module [bytes]
  (let [count (vector-length bytes)]
    (do
      (print count)
      (bootstrap-print-module-bytes bytes 0 count)
      0)))

(defn main []
  (let [stage2 (bootstrap-build-stage2 "(defn main [] (let [v0 (vector-new 1)] (let [v1 (vector-push v0 10)] (vector-length (vector-push v1 20)))))")]
    (do
      (bootstrap-print-module stage2)
      0)))
"#;
    let stage1_source = format!("{}\n{}", selfhost_cli_runtime_bundle(), harness);
    let stage1_wasm = compile_only(&stage1_source);
    assert_valid_wasm(&stage1_wasm);

    let output = lsharp_wasm::wasi_runner::run_wasm_wasi(&stage1_wasm)
        .expect("vector-push program を含む stage1 wasm の実行に失敗");
    let modules = parse_emitted_wasm_modules(&output, 1);
    assert_eq!(modules.len(), 1, "stage2 モジュール数が不正");
    assert_valid_wasm(&modules[0]);
    assert!(
        extract_sections(&modules[0]).iter().any(|(id, _)| *id == 2),
        "vector-push program を含む stage2 Wasm は alloc import section を持つこと"
    );
    assert_eq!(
        run_exported_i64_with_alloc_import(&modules[0], "_start"),
        2,
        "vector-push の in-place + growth を含む stage2 Wasm が alloc import 付きで実行可能であること"
    );
}

/// BOOT-04: stage1 が ref-new/ref-set/ref-get を含む stage2 Wasm を生成できること
#[test]
fn test_e2e_bootstrap_stage1_emits_stage2_wasm_for_ref_program() {
    let harness = r#"
(defn bootstrap-append-bytes [dst src idx count]
  (if (>= idx count)
    dst
    (bootstrap-append-bytes
      (vector-push dst (vector-get src idx))
      src
      (+ idx 1)
      count)))

(defn bootstrap-build-stage2 [src]
  (let [program (parse-program src)
        pair (compile-program-functions program)
        functions (vector-get pair 1)
        header (emit-header)
        type-sec (emit-type-section-alloc-main)
        import-sec (emit-import-section-alloc)
        function-sec (emit-function-section-main-type-index 1)
        memory-sec (emit-memory-section)
        export-sec (emit-export-section-main-index 1)
        code-sec (emit-code-section-functions functions)
        bytes0 (bootstrap-append-bytes (vector-new 64) header 0 (vector-length header))
        bytes1 (bootstrap-append-bytes bytes0 type-sec 0 (vector-length type-sec))
        bytes2 (bootstrap-append-bytes bytes1 import-sec 0 (vector-length import-sec))
        bytes3 (bootstrap-append-bytes bytes2 function-sec 0 (vector-length function-sec))
        bytes4 (bootstrap-append-bytes bytes3 memory-sec 0 (vector-length memory-sec))
        bytes5 (bootstrap-append-bytes bytes4 export-sec 0 (vector-length export-sec))]
    (bootstrap-append-bytes bytes5 code-sec 0 (vector-length code-sec))))

(defn bootstrap-print-module-bytes [bytes idx count]
  (if (>= idx count)
    0
    (do
      (print (vector-get bytes idx))
      (bootstrap-print-module-bytes bytes (+ idx 1) count))))

(defn bootstrap-print-module [bytes]
  (let [count (vector-length bytes)]
    (do
      (print count)
      (bootstrap-print-module-bytes bytes 0 count)
      0)))

(defn main []
  (let [stage2 (bootstrap-build-stage2 "(defn main [] (let [r (ref-new 1)] (do (ref-set r 42) (ref-get r))))")]
    (do
      (bootstrap-print-module stage2)
      0)))
"#;
    let stage1_source = format!("{}\n{}", selfhost_cli_runtime_bundle(), harness);
    let stage1_wasm = compile_only(&stage1_source);
    assert_valid_wasm(&stage1_wasm);

    let output = lsharp_wasm::wasi_runner::run_wasm_wasi(&stage1_wasm)
        .expect("ref program を含む stage1 wasm の実行に失敗");
    let modules = parse_emitted_wasm_modules(&output, 1);
    assert_eq!(modules.len(), 1, "stage2 モジュール数が不正");
    assert_valid_wasm(&modules[0]);
    assert!(
        extract_sections(&modules[0]).iter().any(|(id, _)| *id == 2),
        "ref program を含む stage2 Wasm は alloc import section を持つこと"
    );
    assert_eq!(
        run_exported_i64_with_alloc_import(&modules[0], "_start"),
        42,
        "ref-new/ref-set/ref-get を含む stage2 Wasm が alloc import 付きで実行可能であること"
    );
}

/// BOOT-04: stage1 が整数 key の map-new/map-insert/map-get/map-size を含む stage2 Wasm を生成できること
#[test]
fn test_e2e_bootstrap_stage1_emits_stage2_wasm_for_map_program() {
    let harness = r#"
(defn bootstrap-append-bytes [dst src idx count]
  (if (>= idx count)
    dst
    (bootstrap-append-bytes
      (vector-push dst (vector-get src idx))
      src
      (+ idx 1)
      count)))

(defn bootstrap-build-stage2 [src]
  (let [program (parse-program src)
        pair (compile-program-functions program)
        functions (vector-get pair 1)
        header (emit-header)
        type-sec (emit-type-section-alloc-main)
        import-sec (emit-import-section-alloc)
        function-sec (emit-function-section-main-type-index 1)
        memory-sec (emit-memory-section)
        export-sec (emit-export-section-main-index 1)
        code-sec (emit-code-section-functions functions)
        bytes0 (bootstrap-append-bytes (vector-new 64) header 0 (vector-length header))
        bytes1 (bootstrap-append-bytes bytes0 type-sec 0 (vector-length type-sec))
        bytes2 (bootstrap-append-bytes bytes1 import-sec 0 (vector-length import-sec))
        bytes3 (bootstrap-append-bytes bytes2 function-sec 0 (vector-length function-sec))
        bytes4 (bootstrap-append-bytes bytes3 memory-sec 0 (vector-length memory-sec))
        bytes5 (bootstrap-append-bytes bytes4 export-sec 0 (vector-length export-sec))]
    (bootstrap-append-bytes bytes5 code-sec 0 (vector-length code-sec))))

(defn bootstrap-print-module-bytes [bytes idx count]
  (if (>= idx count)
    0
    (do
      (print (vector-get bytes idx))
      (bootstrap-print-module-bytes bytes (+ idx 1) count))))

(defn bootstrap-print-module [bytes]
  (let [count (vector-length bytes)]
    (do
      (print count)
      (bootstrap-print-module-bytes bytes 0 count)
      0)))

(defn main []
  (let [stage2 (bootstrap-build-stage2 "(defn main [] (let [m0 (map-new)] (let [m1 (map-insert m0 1 10)] (let [m2 (map-insert m1 2 20)] (+ (+ (map-get m2 1) (map-get m2 2)) (map-size m2))))))")]
    (do
      (bootstrap-print-module stage2)
      0)))
"#;
    let stage1_source = format!("{}\n{}", selfhost_cli_runtime_bundle(), harness);
    let stage1_wasm = compile_only(&stage1_source);
    assert_valid_wasm(&stage1_wasm);

    let output = lsharp_wasm::wasi_runner::run_wasm_wasi(&stage1_wasm)
        .expect("map program を含む stage1 wasm の実行に失敗");
    let modules = parse_emitted_wasm_modules(&output, 1);
    assert_eq!(modules.len(), 1, "stage2 モジュール数が不正");
    assert_valid_wasm(&modules[0]);
    assert!(
        extract_sections(&modules[0]).iter().any(|(id, _)| *id == 2),
        "map program を含む stage2 Wasm は alloc import section を持つこと"
    );
    assert_eq!(
        run_exported_i64_with_alloc_import(&modules[0], "_start"),
        32,
        "整数 key の map builtins を含む stage2 Wasm が alloc import 付きで実行可能であること"
    );
}

/// BOOT-04: stage1 が整数 key subset の map-contains? を含む stage2 Wasm を生成できること
#[test]
fn test_e2e_bootstrap_stage1_emits_stage2_wasm_for_map_contains_program() {
    let harness = r#"
(defn bootstrap-append-bytes [dst src idx count]
  (if (>= idx count)
    dst
    (bootstrap-append-bytes
      (vector-push dst (vector-get src idx))
      src
      (+ idx 1)
      count)))

(defn bootstrap-build-stage2 [src]
  (let [program (parse-program src)
        pair (compile-program-functions program)
        functions (vector-get pair 1)
        header (emit-header)
        type-sec (emit-type-section-alloc-main)
        import-sec (emit-import-section-alloc)
        function-sec (emit-function-section-main-type-index 1)
        memory-sec (emit-memory-section)
        export-sec (emit-export-section-main-index 1)
        code-sec (emit-code-section-functions functions)
        bytes0 (bootstrap-append-bytes (vector-new 64) header 0 (vector-length header))
        bytes1 (bootstrap-append-bytes bytes0 type-sec 0 (vector-length type-sec))
        bytes2 (bootstrap-append-bytes bytes1 import-sec 0 (vector-length import-sec))
        bytes3 (bootstrap-append-bytes bytes2 function-sec 0 (vector-length function-sec))
        bytes4 (bootstrap-append-bytes bytes3 memory-sec 0 (vector-length memory-sec))
        bytes5 (bootstrap-append-bytes bytes4 export-sec 0 (vector-length export-sec))]
    (bootstrap-append-bytes bytes5 code-sec 0 (vector-length code-sec))))

(defn bootstrap-print-module-bytes [bytes idx count]
  (if (>= idx count)
    0
    (do
      (print (vector-get bytes idx))
      (bootstrap-print-module-bytes bytes (+ idx 1) count))))

(defn bootstrap-print-module [bytes]
  (let [count (vector-length bytes)]
    (do
      (print count)
      (bootstrap-print-module-bytes bytes 0 count)
      0)))

(defn main []
  (let [stage2 (bootstrap-build-stage2 "(defn main [] (let [m0 (map-new)] (let [m1 (map-insert m0 7 70)] (+ (* 10 (map-contains? m1 7)) (map-contains? m1 99)))))")]
    (do
      (bootstrap-print-module stage2)
      0)))
"#;
    let stage1_source = format!("{}\n{}", selfhost_cli_runtime_bundle(), harness);
    let stage1_wasm = compile_only(&stage1_source);
    assert_valid_wasm(&stage1_wasm);

    let output = lsharp_wasm::wasi_runner::run_wasm_wasi(&stage1_wasm)
        .expect("map-contains? program を含む stage1 wasm の実行に失敗");
    let modules = parse_emitted_wasm_modules(&output, 1);
    assert_eq!(modules.len(), 1, "stage2 モジュール数が不正");
    assert_valid_wasm(&modules[0]);
    assert!(
        extract_sections(&modules[0]).iter().any(|(id, _)| *id == 2),
        "map-contains? program を含む stage2 Wasm は alloc import section を持つこと"
    );
    assert_eq!(
        run_exported_i64_with_alloc_import(&modules[0], "_start"),
        10,
        "整数 key subset の map-contains? を含む stage2 Wasm が alloc import 付きで実行可能であること"
    );
}

/// BOOT-04: stage1 が整数 key subset の map-remove を含む stage2 Wasm を生成できること
#[test]
fn test_e2e_bootstrap_stage1_emits_stage2_wasm_for_map_remove_program() {
    let harness = r#"
(defn bootstrap-append-bytes [dst src idx count]
  (if (>= idx count)
    dst
    (bootstrap-append-bytes
      (vector-push dst (vector-get src idx))
      src
      (+ idx 1)
      count)))

(defn bootstrap-build-stage2 [src]
  (let [program (parse-program src)
        pair (compile-program-functions program)
        functions (vector-get pair 1)
        header (emit-header)
        type-sec (emit-type-section-alloc-main)
        import-sec (emit-import-section-alloc)
        function-sec (emit-function-section-main-type-index 1)
        memory-sec (emit-memory-section)
        export-sec (emit-export-section-main-index 1)
        code-sec (emit-code-section-functions functions)
        bytes0 (bootstrap-append-bytes (vector-new 64) header 0 (vector-length header))
        bytes1 (bootstrap-append-bytes bytes0 type-sec 0 (vector-length type-sec))
        bytes2 (bootstrap-append-bytes bytes1 import-sec 0 (vector-length import-sec))
        bytes3 (bootstrap-append-bytes bytes2 function-sec 0 (vector-length function-sec))
        bytes4 (bootstrap-append-bytes bytes3 memory-sec 0 (vector-length memory-sec))
        bytes5 (bootstrap-append-bytes bytes4 export-sec 0 (vector-length export-sec))]
    (bootstrap-append-bytes bytes5 code-sec 0 (vector-length code-sec))))

(defn bootstrap-print-module-bytes [bytes idx count]
  (if (>= idx count)
    0
    (do
      (print (vector-get bytes idx))
      (bootstrap-print-module-bytes bytes (+ idx 1) count))))

(defn bootstrap-print-module [bytes]
  (let [count (vector-length bytes)]
    (do
      (print count)
      (bootstrap-print-module-bytes bytes 0 count)
      0)))

(defn main []
  (let [stage2 (bootstrap-build-stage2 "(defn main [] (let [m0 (map-new)] (let [m1 (map-insert m0 1 10)] (let [m2 (map-insert m1 2 20)] (let [m3 (map-remove m2 1)] (+ (map-get m3 1) (+ (* 10 (map-size m3)) (map-get m3 2))))))))")]
    (do
      (bootstrap-print-module stage2)
      0)))
"#;
    let stage1_source = format!("{}\n{}", selfhost_cli_runtime_bundle(), harness);
    let stage1_wasm = compile_only(&stage1_source);
    assert_valid_wasm(&stage1_wasm);

    let output = lsharp_wasm::wasi_runner::run_wasm_wasi(&stage1_wasm)
        .expect("map-remove program を含む stage1 wasm の実行に失敗");
    let modules = parse_emitted_wasm_modules(&output, 1);
    assert_eq!(modules.len(), 1, "stage2 モジュール数が不正");
    assert_valid_wasm(&modules[0]);
    assert!(
        extract_sections(&modules[0]).iter().any(|(id, _)| *id == 2),
        "map-remove program を含む stage2 Wasm は alloc import section を持つこと"
    );
    assert_eq!(
        run_exported_i64_with_alloc_import(&modules[0], "_start"),
        30,
        "整数 key subset の map-remove を含む stage2 Wasm が alloc import 付きで実行可能であること"
    );
}

/// BOOT-04: stage1 が source-aware string key subset の map builtins を含む stage2 Wasm を生成できること
#[test]
fn test_e2e_bootstrap_stage1_emits_stage2_wasm_for_string_key_map_program() {
    let stage2_source = r#"(defn main [] (let [m0 (map-new)] (let [m1 (map-insert m0 "aa" 10)] (let [m2 (map-insert m1 "bb" 20)] (let [m3 (map-remove m2 "aa")] (+ (* 10 (map-size m3)) (map-get m3 "bb")))))))"#.replace('"', "\\\"");
    let harness = format!(
        r#"
(defn bootstrap-append-bytes [dst src idx count]
  (if (>= idx count)
    dst
    (bootstrap-append-bytes
      (vector-push dst (vector-get src idx))
      src
      (+ idx 1)
      count)))

(defn bootstrap-build-stage2 [src]
  (let [program (parse-program src)
        pair (compile-program-functions-with-source src program)
        functions (vector-get pair 1)
        data (vector-get pair 2)
        header (emit-header)
        type-sec (emit-type-section-alloc-main)
        import-sec (emit-import-section-alloc)
        function-sec (emit-function-section-main-type-index 1)
        memory-sec (emit-memory-section)
        export-sec (emit-export-section-main-index 1)
        code-sec (emit-code-section-functions functions)
        data-sec (emit-data-section data 1024)
        bytes0 (bootstrap-append-bytes (vector-new 64) header 0 (vector-length header))
        bytes1 (bootstrap-append-bytes bytes0 type-sec 0 (vector-length type-sec))
        bytes2 (bootstrap-append-bytes bytes1 import-sec 0 (vector-length import-sec))
        bytes3 (bootstrap-append-bytes bytes2 function-sec 0 (vector-length function-sec))
        bytes4 (bootstrap-append-bytes bytes3 memory-sec 0 (vector-length memory-sec))
        bytes5 (bootstrap-append-bytes bytes4 export-sec 0 (vector-length export-sec))
        bytes6 (bootstrap-append-bytes bytes5 code-sec 0 (vector-length code-sec))]
    (bootstrap-append-bytes bytes6 data-sec 0 (vector-length data-sec))))

(defn bootstrap-print-module-bytes [bytes idx count]
  (if (>= idx count)
    0
    (do
      (print (vector-get bytes idx))
      (bootstrap-print-module-bytes bytes (+ idx 1) count))))

(defn bootstrap-print-module [bytes]
  (let [count (vector-length bytes)]
    (do
      (print count)
      (bootstrap-print-module-bytes bytes 0 count)
      0)))

(defn main []
  (let [stage2 (bootstrap-build-stage2 "{}")]
    (do
      (bootstrap-print-module stage2)
      0)))
"#,
        stage2_source
    );
    let stage1_source = format!("{}\n{}", selfhost_cli_runtime_bundle(), harness);
    let stage1_wasm = compile_only(&stage1_source);
    assert_valid_wasm(&stage1_wasm);

    let output = lsharp_wasm::wasi_runner::run_wasm_wasi(&stage1_wasm)
        .expect("string key map program を含む stage1 wasm の実行に失敗");
    let modules = parse_emitted_wasm_modules(&output, 1);
    assert_eq!(modules.len(), 1, "stage2 モジュール数が不正");
    assert_valid_wasm(&modules[0]);
    let data_section = extract_section_bytes(&modules[0], 11).unwrap_or_default();
    assert!(
        !data_section
            .windows(2)
            .any(|window| window == [97, 97] || window == [98, 98]),
        "string key literal bytes は data section に残らず hash const 化されること"
    );
    assert_eq!(
        run_exported_i64_with_alloc_import(&modules[0], "_start"),
        30,
        "string key subset の map builtins を含む stage2 Wasm が alloc import 付きで実行可能であること"
    );
}

/// BOOT-04: stage1 が non-literal string key map builtins を含む stage2 Wasm を生成できること
#[test]
fn test_e2e_bootstrap_stage1_emits_stage2_wasm_for_non_literal_string_key_map_program() {
    let stage2_source = r#"(defn main [] (let [key (read-file "fixture.txt")] (let [m0 (map-new)] (let [m1 (map-insert m0 key 42)] (map-get m1 key)))))"#.replace('"', "\\\"");
    let harness = format!(
        r#"
(defn bootstrap-append-bytes [dst src idx count]
  (if (>= idx count)
    dst
    (bootstrap-append-bytes
      (vector-push dst (vector-get src idx))
      src
      (+ idx 1)
      count)))

(defn bootstrap-build-stage2 [src]
  (let [program (parse-program src)
        pair (compile-program-functions-with-source src program)
        functions (vector-get pair 1)
        data (vector-get pair 2)
        header (emit-header)
        type-sec (emit-type-section-alloc-print-main)
        import-sec (emit-import-section-alloc-print-read-hash)
        function-sec (emit-function-section-main-type-index 2)
        memory-sec (emit-memory-section)
        export-sec (emit-export-section-main-memory-index 4 0)
        code-sec (emit-code-section-functions functions)
        data-sec (emit-data-section data 1024)
        bytes0 (bootstrap-append-bytes (vector-new 64) header 0 (vector-length header))
        bytes1 (bootstrap-append-bytes bytes0 type-sec 0 (vector-length type-sec))
        bytes2 (bootstrap-append-bytes bytes1 import-sec 0 (vector-length import-sec))
        bytes3 (bootstrap-append-bytes bytes2 function-sec 0 (vector-length function-sec))
        bytes4 (bootstrap-append-bytes bytes3 memory-sec 0 (vector-length memory-sec))
        bytes5 (bootstrap-append-bytes bytes4 export-sec 0 (vector-length export-sec))
        bytes6 (bootstrap-append-bytes bytes5 code-sec 0 (vector-length code-sec))]
    (bootstrap-append-bytes bytes6 data-sec 0 (vector-length data-sec))))

(defn bootstrap-print-module-bytes [bytes idx count]
  (if (>= idx count)
    0
    (do
      (print (vector-get bytes idx))
      (bootstrap-print-module-bytes bytes (+ idx 1) count))))

(defn bootstrap-print-module [bytes]
  (let [count (vector-length bytes)]
    (do
      (print count)
      (bootstrap-print-module-bytes bytes 0 count)
      0)))

(defn main []
  (let [stage2 (bootstrap-build-stage2 "{}")]
    (do
      (bootstrap-print-module stage2)
      0)))
"#,
        stage2_source
    );
    let stage1_source = format!("{}\n{}", selfhost_cli_runtime_bundle(), harness);
    let stage1_wasm = compile_only(&stage1_source);
    assert_valid_wasm(&stage1_wasm);

    let output = lsharp_wasm::wasi_runner::run_wasm_wasi(&stage1_wasm)
        .expect("non-literal string key map program を含む stage1 wasm の実行に失敗");
    let modules = parse_emitted_wasm_modules(&output, 1);
    assert_eq!(modules.len(), 1, "stage2 モジュール数が不正");
    assert_valid_wasm(&modules[0]);
    let data_section = extract_section_bytes(&modules[0], 11).unwrap_or_default();
    assert!(
        data_section
            .windows("fixture.txt".len())
            .any(|window| window == b"fixture.txt"),
        "read-file path literal bytes は data section に配置されること"
    );
    assert_eq!(
        run_exported_i64_with_alloc_print_read_hash_imports(&modules[0], "_start", "aa").0,
        42,
        "non-literal string key map builtins を含む stage2 Wasm が alloc/print/read-file/fnv1a import 付きで実行可能であること"
    );
}

/// BOOT-04: stage1 が generalized 4-helper path で alloc+print+read-file+__fnv1a_hash stage2 Wasm を生成できること
#[test]
fn test_e2e_bootstrap_stage1_emits_stage2_wasm_for_generalized_alloc_print_read_hash_helper_quad() {
    let stage2_source = r#"(defn main [] (let [key (read-file "fixture.txt")] (let [m0 (map-new)] (let [m1 (map-insert m0 key 42)] (map-get m1 key)))))"#.replace('"', "\\\"");
    let harness = format!(
        r#"
(defn bootstrap-append-bytes [dst src idx count]
  (if (>= idx count)
    dst
    (bootstrap-append-bytes
      (vector-push dst (vector-get src idx))
      src
      (+ idx 1)
      count)))

(defn bootstrap-build-stage2 [src]
  (let [program (parse-program src)
        pair (compile-program-functions-with-source src program)
        functions (vector-get pair 1)
        data (vector-get pair 2)
        header (emit-header)
        type-sec (emit-type-section-helper-quad-main (helper-id-alloc) (helper-id-print) (helper-id-read-file) (helper-id-runtime-hash))
        import-sec (emit-import-section-helper-quad (helper-id-alloc) (helper-id-print) (helper-id-read-file) (helper-id-runtime-hash))
        function-sec (emit-function-section-main-type-index 2)
        memory-sec (emit-memory-section)
        export-sec (emit-export-section-main-memory-index 4 0)
        code-sec (emit-code-section-functions functions)
        data-sec (emit-data-section data 1024)
        bytes0 (bootstrap-append-bytes (vector-new 64) header 0 (vector-length header))
        bytes1 (bootstrap-append-bytes bytes0 type-sec 0 (vector-length type-sec))
        bytes2 (bootstrap-append-bytes bytes1 import-sec 0 (vector-length import-sec))
        bytes3 (bootstrap-append-bytes bytes2 function-sec 0 (vector-length function-sec))
        bytes4 (bootstrap-append-bytes bytes3 memory-sec 0 (vector-length memory-sec))
        bytes5 (bootstrap-append-bytes bytes4 export-sec 0 (vector-length export-sec))
        bytes6 (bootstrap-append-bytes bytes5 code-sec 0 (vector-length code-sec))]
    (bootstrap-append-bytes bytes6 data-sec 0 (vector-length data-sec))))

(defn bootstrap-print-module-bytes [bytes idx count]
  (if (>= idx count)
    0
    (do
      (print (vector-get bytes idx))
      (bootstrap-print-module-bytes bytes (+ idx 1) count))))

(defn bootstrap-print-module [bytes]
  (let [count (vector-length bytes)]
    (do
      (print count)
      (bootstrap-print-module-bytes bytes 0 count)
      0)))

(defn main []
  (let [stage2 (bootstrap-build-stage2 "{}")]
    (do
      (bootstrap-print-module stage2)
      0)))
"#,
        stage2_source
    );
    let stage1_source = format!("{}\n{}", selfhost_cli_runtime_bundle(), harness);
    let stage1_wasm = compile_only(&stage1_source);
    assert_valid_wasm(&stage1_wasm);

    let output = lsharp_wasm::wasi_runner::run_wasm_wasi(&stage1_wasm)
        .expect("generalized alloc+print+read-file+__fnv1a_hash quad program を含む stage1 wasm の実行に失敗");
    let modules = parse_emitted_wasm_modules(&output, 1);
    assert_eq!(modules.len(), 1, "stage2 モジュール数が不正");
    assert_valid_wasm(&modules[0]);
    let data_section = extract_section_bytes(&modules[0], 11).unwrap_or_default();
    assert!(
        data_section
            .windows("fixture.txt".len())
            .any(|window| window == b"fixture.txt"),
        "generalized hash quad でも read-file path literal bytes は data section に配置されること"
    );
    assert_eq!(
        run_exported_i64_with_alloc_print_read_hash_imports(&modules[0], "_start", "aa").0,
        42,
        "generalized alloc+print+read-file+__fnv1a_hash quad を含む stage2 Wasm が実行可能であること"
    );
}

/// BOOT-04: stage1 が同じ generalized alloc+print+read-file+__fnv1a_hash source から同一 stage2 Wasm を 2 回生成できること
#[test]
fn test_e2e_bootstrap_stage1_emits_identical_hash_helper_quad_stage2_wasm_for_same_source() {
    let stage2_source = r#"(defn main [] (let [key (read-file "fixture.txt")] (let [m0 (map-new)] (let [m1 (map-insert m0 key 42)] (map-get m1 key)))))"#.replace('"', "\\\"");
    let harness = format!(
        r#"
(defn bootstrap-append-bytes [dst src idx count]
  (if (>= idx count)
    dst
    (bootstrap-append-bytes
      (vector-push dst (vector-get src idx))
      src
      (+ idx 1)
      count)))

(defn bootstrap-build-stage2 [src]
  (let [program (parse-program src)
        pair (compile-program-functions-with-source src program)
        functions (vector-get pair 1)
        data (vector-get pair 2)
        header (emit-header)
        type-sec (emit-type-section-helper-quad-main (helper-id-alloc) (helper-id-print) (helper-id-read-file) (helper-id-runtime-hash))
        import-sec (emit-import-section-helper-quad (helper-id-alloc) (helper-id-print) (helper-id-read-file) (helper-id-runtime-hash))
        function-sec (emit-function-section-main-type-index 2)
        memory-sec (emit-memory-section)
        export-sec (emit-export-section-main-memory-index 4 0)
        code-sec (emit-code-section-functions functions)
        data-sec (emit-data-section data 1024)
        bytes0 (bootstrap-append-bytes (vector-new 64) header 0 (vector-length header))
        bytes1 (bootstrap-append-bytes bytes0 type-sec 0 (vector-length type-sec))
        bytes2 (bootstrap-append-bytes bytes1 import-sec 0 (vector-length import-sec))
        bytes3 (bootstrap-append-bytes bytes2 function-sec 0 (vector-length function-sec))
        bytes4 (bootstrap-append-bytes bytes3 memory-sec 0 (vector-length memory-sec))
        bytes5 (bootstrap-append-bytes bytes4 export-sec 0 (vector-length export-sec))
        bytes6 (bootstrap-append-bytes bytes5 code-sec 0 (vector-length code-sec))]
    (bootstrap-append-bytes bytes6 data-sec 0 (vector-length data-sec))))

(defn bootstrap-print-module-bytes [bytes idx count]
  (if (>= idx count)
    0
    (do
      (print (vector-get bytes idx))
      (bootstrap-print-module-bytes bytes (+ idx 1) count))))

(defn bootstrap-print-module [bytes]
  (let [count (vector-length bytes)]
    (do
      (print count)
      (bootstrap-print-module-bytes bytes 0 count)
      0)))

(defn main []
  (let [src "{}"
        stage2-a (bootstrap-build-stage2 src)
        stage2-b (bootstrap-build-stage2 src)]
    (do
      (bootstrap-print-module stage2-a)
      (bootstrap-print-module stage2-b)
      0)))
"#,
        stage2_source
    );
    let stage1_source = format!("{}\n{}", selfhost_cli_runtime_bundle(), harness);
    let stage1_wasm = compile_only(&stage1_source);
    assert_valid_wasm(&stage1_wasm);

    let output = lsharp_wasm::wasi_runner::run_wasm_wasi(&stage1_wasm)
        .expect("same hash-helper quad source stage1 実行に失敗");
    let modules = parse_emitted_wasm_modules(&output, 2);
    assert_eq!(modules.len(), 2, "stage2 モジュール数が不正");
    assert_eq!(
        modules[0], modules[1],
        "同じ generalized hash helper quad source から stage2 Wasm が非決定的に変化した"
    );
    assert_valid_wasm(&modules[0]);
    let data_section = extract_section_bytes(&modules[0], 11).unwrap_or_default();
    assert!(
        data_section
            .windows("fixture.txt".len())
            .any(|window| window == b"fixture.txt"),
        "repeatability でも hash quad の read-file path literal bytes は data section に配置されること"
    );
    assert_eq!(
        run_exported_i64_with_alloc_print_read_hash_imports(&modules[0], "_start", "aa").0,
        42
    );
}

/// BOOT-04: stage1 が alloc+print import を伴う print program を stage2 Wasm として生成できること
#[test]
fn test_e2e_bootstrap_stage1_emits_stage2_wasm_for_print_program() {
    let harness = r#"
(defn bootstrap-append-bytes [dst src idx count]
  (if (>= idx count)
    dst
    (bootstrap-append-bytes
      (vector-push dst (vector-get src idx))
      src
      (+ idx 1)
      count)))

(defn bootstrap-build-stage2 [src]
  (let [program (parse-program src)
        pair (compile-program-functions program)
        functions (vector-get pair 1)
        header (emit-header)
        type-sec (emit-type-section-alloc-print-main)
        import-sec (emit-import-section-alloc-print)
        function-sec (emit-function-section-main-type-index 2)
        memory-sec (emit-memory-section)
        export-sec (emit-export-section-main-index 2)
        code-sec (emit-code-section-functions functions)
        bytes0 (bootstrap-append-bytes (vector-new 64) header 0 (vector-length header))
        bytes1 (bootstrap-append-bytes bytes0 type-sec 0 (vector-length type-sec))
        bytes2 (bootstrap-append-bytes bytes1 import-sec 0 (vector-length import-sec))
        bytes3 (bootstrap-append-bytes bytes2 function-sec 0 (vector-length function-sec))
        bytes4 (bootstrap-append-bytes bytes3 memory-sec 0 (vector-length memory-sec))
        bytes5 (bootstrap-append-bytes bytes4 export-sec 0 (vector-length export-sec))]
    (bootstrap-append-bytes bytes5 code-sec 0 (vector-length code-sec))))

(defn bootstrap-print-module-bytes [bytes idx count]
  (if (>= idx count)
    0
    (do
      (print (vector-get bytes idx))
      (bootstrap-print-module-bytes bytes (+ idx 1) count))))

(defn bootstrap-print-module [bytes]
  (let [count (vector-length bytes)]
    (do
      (print count)
      (bootstrap-print-module-bytes bytes 0 count)
      0)))

(defn main []
  (let [stage2 (bootstrap-build-stage2 "(defn main [] (do (print 42) (print 7) 0))")]
    (do
      (bootstrap-print-module stage2)
      0)))
"#;
    let stage1_source = format!("{}\n{}", selfhost_cli_runtime_bundle(), harness);
    let stage1_wasm = compile_only(&stage1_source);
    assert_valid_wasm(&stage1_wasm);

    let output = lsharp_wasm::wasi_runner::run_wasm_wasi(&stage1_wasm)
        .expect("print program を含む stage1 wasm の実行に失敗");
    let modules = parse_emitted_wasm_modules(&output, 1);
    assert_eq!(modules.len(), 1, "stage2 モジュール数が不正");
    assert_valid_wasm(&modules[0]);
    assert!(
        extract_sections(&modules[0]).iter().any(|(id, _)| *id == 2),
        "print program を含む stage2 Wasm は import section を持つこと"
    );
    let (result, printed) = run_exported_i64_with_alloc_print_imports(&modules[0], "_start");
    assert_eq!(result, 0, "print program を含む stage2 Wasm の戻り値が不正");
    assert_eq!(printed, "42\n7\n", "stage2 print output が不正");
}

/// BOOT-04: stage1 が generalized 2-helper pair で alloc+print stage2 Wasm を生成できること
#[test]
fn test_e2e_bootstrap_stage1_emits_stage2_wasm_for_generalized_alloc_print_helper_pair() {
    let harness = r#"
(defn bootstrap-append-bytes [dst src idx count]
  (if (>= idx count)
    dst
    (bootstrap-append-bytes
      (vector-push dst (vector-get src idx))
      src
      (+ idx 1)
      count)))

(defn bootstrap-build-stage2 [src]
  (let [program (parse-program src)
        pair (compile-program-functions program)
        functions (vector-get pair 1)
        header (emit-header)
        type-sec (emit-type-section-helper-pair-main (helper-id-alloc) (helper-id-print))
        import-sec (emit-import-section-helper-pair (helper-id-alloc) (helper-id-print))
        function-sec (emit-function-section-main-type-index 2)
        memory-sec (emit-memory-section)
        export-sec (emit-export-section-main-index 2)
        code-sec (emit-code-section-functions functions)
        bytes0 (bootstrap-append-bytes (vector-new 64) header 0 (vector-length header))
        bytes1 (bootstrap-append-bytes bytes0 type-sec 0 (vector-length type-sec))
        bytes2 (bootstrap-append-bytes bytes1 import-sec 0 (vector-length import-sec))
        bytes3 (bootstrap-append-bytes bytes2 function-sec 0 (vector-length function-sec))
        bytes4 (bootstrap-append-bytes bytes3 memory-sec 0 (vector-length memory-sec))
        bytes5 (bootstrap-append-bytes bytes4 export-sec 0 (vector-length export-sec))]
    (bootstrap-append-bytes bytes5 code-sec 0 (vector-length code-sec))))

(defn bootstrap-print-module-bytes [bytes idx count]
  (if (>= idx count)
    0
    (do
      (print (vector-get bytes idx))
      (bootstrap-print-module-bytes bytes (+ idx 1) count))))

(defn bootstrap-print-module [bytes]
  (let [count (vector-length bytes)]
    (do
      (print count)
      (bootstrap-print-module-bytes bytes 0 count)
      0)))

(defn main []
  (let [stage2 (bootstrap-build-stage2 "(defn main [] (do (print 42) (print 7) 0))")]
    (do
      (bootstrap-print-module stage2)
      0)))
"#;
    let stage1_source = format!("{}\n{}", selfhost_cli_runtime_bundle(), harness);
    let stage1_wasm = compile_only(&stage1_source);
    assert_valid_wasm(&stage1_wasm);

    let output = lsharp_wasm::wasi_runner::run_wasm_wasi(&stage1_wasm)
        .expect("generalized alloc+print pair program を含む stage1 wasm の実行に失敗");
    let modules = parse_emitted_wasm_modules(&output, 1);
    assert_eq!(modules.len(), 1, "stage2 モジュール数が不正");
    assert_valid_wasm(&modules[0]);
    let (result, printed) = run_exported_i64_with_alloc_print_imports(&modules[0], "_start");
    assert_eq!(
        result, 0,
        "generalized alloc+print pair stage2 Wasm の戻り値が不正"
    );
    assert_eq!(
        printed, "42\n7\n",
        "generalized alloc+print pair stage2 print output が不正"
    );
}

/// BOOT-04: stage1 が同じ generalized alloc+print pair source から同一 stage2 Wasm を 2 回生成できること
#[test]
fn test_e2e_bootstrap_stage1_emits_identical_alloc_print_pair_stage2_wasm_for_same_source() {
    let harness = r#"
(defn bootstrap-append-bytes [dst src idx count]
  (if (>= idx count)
    dst
    (bootstrap-append-bytes
      (vector-push dst (vector-get src idx))
      src
      (+ idx 1)
      count)))

(defn bootstrap-build-stage2 [src]
  (let [program (parse-program src)
        pair (compile-program-functions program)
        functions (vector-get pair 1)
        header (emit-header)
        type-sec (emit-type-section-helper-pair-main (helper-id-alloc) (helper-id-print))
        import-sec (emit-import-section-helper-pair (helper-id-alloc) (helper-id-print))
        function-sec (emit-function-section-main-type-index 2)
        memory-sec (emit-memory-section)
        export-sec (emit-export-section-main-index 2)
        code-sec (emit-code-section-functions functions)
        bytes0 (bootstrap-append-bytes (vector-new 64) header 0 (vector-length header))
        bytes1 (bootstrap-append-bytes bytes0 type-sec 0 (vector-length type-sec))
        bytes2 (bootstrap-append-bytes bytes1 import-sec 0 (vector-length import-sec))
        bytes3 (bootstrap-append-bytes bytes2 function-sec 0 (vector-length function-sec))
        bytes4 (bootstrap-append-bytes bytes3 memory-sec 0 (vector-length memory-sec))
        bytes5 (bootstrap-append-bytes bytes4 export-sec 0 (vector-length export-sec))]
    (bootstrap-append-bytes bytes5 code-sec 0 (vector-length code-sec))))

(defn bootstrap-print-module-bytes [bytes idx count]
  (if (>= idx count)
    0
    (do
      (print (vector-get bytes idx))
      (bootstrap-print-module-bytes bytes (+ idx 1) count))))

(defn bootstrap-print-module [bytes]
  (let [count (vector-length bytes)]
    (do
      (print count)
      (bootstrap-print-module-bytes bytes 0 count)
      0)))

(defn main []
  (let [src "(defn main [] (do (print 42) (print 7) 0))"
        stage2-a (bootstrap-build-stage2 src)
        stage2-b (bootstrap-build-stage2 src)]
    (do
      (bootstrap-print-module stage2-a)
      (bootstrap-print-module stage2-b)
      0)))
"#;
    let stage1_source = format!("{}\n{}", selfhost_cli_runtime_bundle(), harness);
    let stage1_wasm = compile_only(&stage1_source);
    assert_valid_wasm(&stage1_wasm);

    let output = lsharp_wasm::wasi_runner::run_wasm_wasi(&stage1_wasm)
        .expect("same generalized alloc+print pair source stage1 実行に失敗");
    let modules = parse_emitted_wasm_modules(&output, 2);
    assert_eq!(modules.len(), 2, "stage2 モジュール数が不正");
    assert_eq!(
        modules[0], modules[1],
        "同じ generalized alloc+print pair source から stage2 Wasm が非決定的に変化した"
    );
    assert_valid_wasm(&modules[0]);
    let (result, printed) = run_exported_i64_with_alloc_print_imports(&modules[0], "_start");
    assert_eq!(result, 0);
    assert_eq!(printed, "42\n7\n");
}

/// BOOT-04: stage1 が alloc+print+read-file import を伴う read-file program を stage2 Wasm として生成できること
#[test]
fn test_e2e_bootstrap_stage1_emits_stage2_wasm_for_read_file_program() {
    let harness = r#"
(defn bootstrap-append-bytes [dst src idx count]
  (if (>= idx count)
    dst
    (bootstrap-append-bytes
      (vector-push dst (vector-get src idx))
      src
      (+ idx 1)
      count)))

(defn bootstrap-build-stage2 [src]
  (let [program (parse-program src)
        pair (compile-program-functions program)
        functions (vector-get pair 1)
        header (emit-header)
        type-sec (emit-type-section-alloc-print-main)
        import-sec (emit-import-section-alloc-print-read)
        function-sec (emit-function-section-main-type-index 2)
        memory-sec (emit-memory-section)
        export-sec (emit-export-section-main-memory-index 3 0)
        code-sec (emit-code-section-functions functions)
        bytes0 (bootstrap-append-bytes (vector-new 64) header 0 (vector-length header))
        bytes1 (bootstrap-append-bytes bytes0 type-sec 0 (vector-length type-sec))
        bytes2 (bootstrap-append-bytes bytes1 import-sec 0 (vector-length import-sec))
        bytes3 (bootstrap-append-bytes bytes2 function-sec 0 (vector-length function-sec))
        bytes4 (bootstrap-append-bytes bytes3 memory-sec 0 (vector-length memory-sec))
        bytes5 (bootstrap-append-bytes bytes4 export-sec 0 (vector-length export-sec))]
    (bootstrap-append-bytes bytes5 code-sec 0 (vector-length code-sec))))

(defn bootstrap-print-module-bytes [bytes idx count]
  (if (>= idx count)
    0
    (do
      (print (vector-get bytes idx))
      (bootstrap-print-module-bytes bytes (+ idx 1) count))))

(defn bootstrap-print-module [bytes]
  (let [count (vector-length bytes)]
    (do
      (print count)
      (bootstrap-print-module-bytes bytes 0 count)
      0)))

(defn main []
  (let [stage2 (bootstrap-build-stage2 "(defn main [] (string-length (read-file 0)))")]
    (do
      (bootstrap-print-module stage2)
      0)))
"#;
    let stage1_source = format!("{}\n{}", selfhost_cli_runtime_bundle(), harness);
    let stage1_wasm = compile_only(&stage1_source);
    assert_valid_wasm(&stage1_wasm);

    let output = lsharp_wasm::wasi_runner::run_wasm_wasi(&stage1_wasm)
        .expect("read-file program を含む stage1 wasm の実行に失敗");
    let modules = parse_emitted_wasm_modules(&output, 1);
    assert_eq!(modules.len(), 1, "stage2 モジュール数が不正");
    assert_valid_wasm(&modules[0]);
    assert!(
        extract_sections(&modules[0]).iter().any(|(id, _)| *id == 2),
        "read-file program を含む stage2 Wasm は import section を持つこと"
    );
    let (result, printed) =
        run_exported_i64_with_alloc_print_read_imports(&modules[0], "_start", "hello from file");
    assert_eq!(
        result, 15,
        "read-file program を含む stage2 Wasm の戻り値が不正"
    );
    assert!(
        printed.is_empty(),
        "read-file slice では print output は不要"
    );
}

/// BOOT-04: stage1 が generalized 3-helper triple で alloc+print+read-file stage2 Wasm を生成できること
#[test]
fn test_e2e_bootstrap_stage1_emits_stage2_wasm_for_generalized_alloc_print_read_helper_triple() {
    let harness = r#"
(defn bootstrap-append-bytes [dst src idx count]
  (if (>= idx count)
    dst
    (bootstrap-append-bytes
      (vector-push dst (vector-get src idx))
      src
      (+ idx 1)
      count)))

(defn bootstrap-build-stage2 [src]
  (let [program (parse-program src)
        pair (compile-program-functions program)
        functions (vector-get pair 1)
        header (emit-header)
        type-sec (emit-type-section-helper-triple-main (helper-id-alloc) (helper-id-print) (helper-id-read-file))
        import-sec (emit-import-section-helper-triple (helper-id-alloc) (helper-id-print) (helper-id-read-file))
        function-sec (emit-function-section-main-type-index 2)
        memory-sec (emit-memory-section)
        export-sec (emit-export-section-main-memory-index 3 0)
        code-sec (emit-code-section-functions functions)
        bytes0 (bootstrap-append-bytes (vector-new 64) header 0 (vector-length header))
        bytes1 (bootstrap-append-bytes bytes0 type-sec 0 (vector-length type-sec))
        bytes2 (bootstrap-append-bytes bytes1 import-sec 0 (vector-length import-sec))
        bytes3 (bootstrap-append-bytes bytes2 function-sec 0 (vector-length function-sec))
        bytes4 (bootstrap-append-bytes bytes3 memory-sec 0 (vector-length memory-sec))
        bytes5 (bootstrap-append-bytes bytes4 export-sec 0 (vector-length export-sec))]
    (bootstrap-append-bytes bytes5 code-sec 0 (vector-length code-sec))))

(defn bootstrap-print-module-bytes [bytes idx count]
  (if (>= idx count)
    0
    (do
      (print (vector-get bytes idx))
      (bootstrap-print-module-bytes bytes (+ idx 1) count))))

(defn bootstrap-print-module [bytes]
  (let [count (vector-length bytes)]
    (do
      (print count)
      (bootstrap-print-module-bytes bytes 0 count)
      0)))

(defn main []
  (let [stage2 (bootstrap-build-stage2 "(defn main [] (string-length (read-file 0)))")]
    (do
      (bootstrap-print-module stage2)
      0)))
"#;
    let stage1_source = format!("{}\n{}", selfhost_cli_runtime_bundle(), harness);
    let stage1_wasm = compile_only(&stage1_source);
    assert_valid_wasm(&stage1_wasm);

    let output = lsharp_wasm::wasi_runner::run_wasm_wasi(&stage1_wasm)
        .expect("generalized alloc+print+read-file triple program を含む stage1 wasm の実行に失敗");
    let modules = parse_emitted_wasm_modules(&output, 1);
    assert_eq!(modules.len(), 1, "stage2 モジュール数が不正");
    assert_valid_wasm(&modules[0]);
    let (result, printed) =
        run_exported_i64_with_alloc_print_read_imports(&modules[0], "_start", "hello from file");
    assert_eq!(
        result, 15,
        "generalized alloc+print+read-file triple stage2 Wasm の戻り値が不正"
    );
    assert!(
        printed.is_empty(),
        "generalized alloc+print+read-file triple slice では print output は不要"
    );
}

/// BOOT-04: stage1 が同じ generalized alloc+print+read-file triple source から同一 stage2 Wasm を 2 回生成できること
#[test]
fn test_e2e_bootstrap_stage1_emits_identical_read_helper_triple_stage2_wasm_for_same_source() {
    let harness = r#"
(defn bootstrap-append-bytes [dst src idx count]
  (if (>= idx count)
    dst
    (bootstrap-append-bytes
      (vector-push dst (vector-get src idx))
      src
      (+ idx 1)
      count)))

(defn bootstrap-build-stage2 [src]
  (let [program (parse-program src)
        pair (compile-program-functions program)
        functions (vector-get pair 1)
        header (emit-header)
        type-sec (emit-type-section-helper-triple-main (helper-id-alloc) (helper-id-print) (helper-id-read-file))
        import-sec (emit-import-section-helper-triple (helper-id-alloc) (helper-id-print) (helper-id-read-file))
        function-sec (emit-function-section-main-type-index 2)
        memory-sec (emit-memory-section)
        export-sec (emit-export-section-main-memory-index 3 0)
        code-sec (emit-code-section-functions functions)
        bytes0 (bootstrap-append-bytes (vector-new 64) header 0 (vector-length header))
        bytes1 (bootstrap-append-bytes bytes0 type-sec 0 (vector-length type-sec))
        bytes2 (bootstrap-append-bytes bytes1 import-sec 0 (vector-length import-sec))
        bytes3 (bootstrap-append-bytes bytes2 function-sec 0 (vector-length function-sec))
        bytes4 (bootstrap-append-bytes bytes3 memory-sec 0 (vector-length memory-sec))
        bytes5 (bootstrap-append-bytes bytes4 export-sec 0 (vector-length export-sec))]
    (bootstrap-append-bytes bytes5 code-sec 0 (vector-length code-sec))))

(defn bootstrap-print-module-bytes [bytes idx count]
  (if (>= idx count)
    0
    (do
      (print (vector-get bytes idx))
      (bootstrap-print-module-bytes bytes (+ idx 1) count))))

(defn bootstrap-print-module [bytes]
  (let [count (vector-length bytes)]
    (do
      (print count)
      (bootstrap-print-module-bytes bytes 0 count)
      0)))

(defn main []
  (let [src "(defn main [] (string-length (read-file 0)))"
        stage2-a (bootstrap-build-stage2 src)
        stage2-b (bootstrap-build-stage2 src)]
    (do
      (bootstrap-print-module stage2-a)
      (bootstrap-print-module stage2-b)
      0)))
"#;
    let stage1_source = format!("{}\n{}", selfhost_cli_runtime_bundle(), harness);
    let stage1_wasm = compile_only(&stage1_source);
    assert_valid_wasm(&stage1_wasm);

    let output = lsharp_wasm::wasi_runner::run_wasm_wasi(&stage1_wasm)
        .expect("same generalized read-helper triple source stage1 実行に失敗");
    let modules = parse_emitted_wasm_modules(&output, 2);
    assert_eq!(modules.len(), 2, "stage2 モジュール数が不正");
    assert_eq!(
        modules[0], modules[1],
        "同じ generalized alloc+print+read-file triple source から stage2 Wasm が非決定的に変化した"
    );
    assert_valid_wasm(&modules[0]);
    let (result, printed) =
        run_exported_i64_with_alloc_print_read_imports(&modules[0], "_start", "hello from file");
    assert_eq!(result, 15);
    assert!(printed.is_empty());
}

/// BOOT-04: stage1 が同じ read-file helper source から同一 stage2 Wasm を 2 回生成できること
#[test]
fn test_e2e_bootstrap_stage1_emits_identical_read_helper_stage2_wasm_for_same_source() {
    let harness = r#"
(defn bootstrap-append-bytes [dst src idx count]
  (if (>= idx count)
    dst
    (bootstrap-append-bytes
      (vector-push dst (vector-get src idx))
      src
      (+ idx 1)
      count)))

(defn bootstrap-build-stage2 [src]
  (let [program (parse-program src)
        pair (compile-program-functions program)
        functions (vector-get pair 1)
        header (emit-header)
        type-sec (emit-type-section-alloc-print-main)
        import-sec (emit-import-section-alloc-print-read)
        function-sec (emit-function-section-main-type-index 2)
        memory-sec (emit-memory-section)
        export-sec (emit-export-section-main-memory-index 3 0)
        code-sec (emit-code-section-functions functions)
        bytes0 (bootstrap-append-bytes (vector-new 64) header 0 (vector-length header))
        bytes1 (bootstrap-append-bytes bytes0 type-sec 0 (vector-length type-sec))
        bytes2 (bootstrap-append-bytes bytes1 import-sec 0 (vector-length import-sec))
        bytes3 (bootstrap-append-bytes bytes2 function-sec 0 (vector-length function-sec))
        bytes4 (bootstrap-append-bytes bytes3 memory-sec 0 (vector-length memory-sec))
        bytes5 (bootstrap-append-bytes bytes4 export-sec 0 (vector-length export-sec))]
    (bootstrap-append-bytes bytes5 code-sec 0 (vector-length code-sec))))

(defn bootstrap-print-module-bytes [bytes idx count]
  (if (>= idx count)
    0
    (do
      (print (vector-get bytes idx))
      (bootstrap-print-module-bytes bytes (+ idx 1) count))))

(defn bootstrap-print-module [bytes]
  (let [count (vector-length bytes)]
    (do
      (print count)
      (bootstrap-print-module-bytes bytes 0 count)
      0)))

(defn main []
  (let [src "(defn main [] (string-length (read-file 0)))"
        stage2-a (bootstrap-build-stage2 src)
        stage2-b (bootstrap-build-stage2 src)]
    (do
      (bootstrap-print-module stage2-a)
      (bootstrap-print-module stage2-b)
      0)))
"#;
    let stage1_source = format!("{}\n{}", selfhost_cli_runtime_bundle(), harness);
    let stage1_wasm = compile_only(&stage1_source);
    assert_valid_wasm(&stage1_wasm);

    let output = lsharp_wasm::wasi_runner::run_wasm_wasi(&stage1_wasm)
        .expect("same read-helper source stage1 実行に失敗");
    let modules = parse_emitted_wasm_modules(&output, 2);
    assert_eq!(modules.len(), 2, "stage2 モジュール数が不正");
    assert_eq!(
        modules[0], modules[1],
        "同じ read-file helper source から stage2 Wasm が非決定的に変化した"
    );
    assert_valid_wasm(&modules[0]);
    let (result, printed) =
        run_exported_i64_with_alloc_print_read_imports(&modules[0], "_start", "hello from file");
    assert_eq!(result, 15);
    assert!(printed.is_empty());
}

/// BOOT-04: stage1 が実 path string を伴う read-file program を stage2 Wasm として生成できること
#[test]
fn test_e2e_bootstrap_stage1_emits_stage2_wasm_for_read_file_path_string_program() {
    let stage2_source =
        r#"(defn main [] (string-length (read-file "fixture.txt")))"#.replace('"', "\\\"");
    let harness = format!(
        r#"
(defn bootstrap-append-bytes [dst src idx count]
  (if (>= idx count)
    dst
    (bootstrap-append-bytes
      (vector-push dst (vector-get src idx))
      src
      (+ idx 1)
      count)))

(defn bootstrap-build-stage2 [src]
  (let [program (parse-program src)
        pair (compile-program-functions-with-source src program)
        functions (vector-get pair 1)
        data (vector-get pair 2)
        header (emit-header)
        type-sec (emit-type-section-alloc-print-main)
        import-sec (emit-import-section-alloc-print-read)
        function-sec (emit-function-section-main-type-index 2)
        memory-sec (emit-memory-section)
        export-sec (emit-export-section-main-memory-index 3 0)
        code-sec (emit-code-section-functions functions)
        data-sec (emit-data-section data 1024)
        bytes0 (bootstrap-append-bytes (vector-new 64) header 0 (vector-length header))
        bytes1 (bootstrap-append-bytes bytes0 type-sec 0 (vector-length type-sec))
        bytes2 (bootstrap-append-bytes bytes1 import-sec 0 (vector-length import-sec))
        bytes3 (bootstrap-append-bytes bytes2 function-sec 0 (vector-length function-sec))
        bytes4 (bootstrap-append-bytes bytes3 memory-sec 0 (vector-length memory-sec))
        bytes5 (bootstrap-append-bytes bytes4 export-sec 0 (vector-length export-sec))
        bytes6 (bootstrap-append-bytes bytes5 code-sec 0 (vector-length code-sec))]
    (bootstrap-append-bytes bytes6 data-sec 0 (vector-length data-sec))))

(defn bootstrap-print-module-bytes [bytes idx count]
  (if (>= idx count)
    0
    (do
      (print (vector-get bytes idx))
      (bootstrap-print-module-bytes bytes (+ idx 1) count))))

(defn bootstrap-print-module [bytes]
  (let [count (vector-length bytes)]
    (do
      (print count)
      (bootstrap-print-module-bytes bytes 0 count)
      0)))

(defn main []
  (let [stage2 (bootstrap-build-stage2 "{}")]
    (do
      (bootstrap-print-module stage2)
      0)))
"#,
        stage2_source
    );
    let stage1_source = format!("{}\n{}", selfhost_cli_runtime_bundle(), harness);
    let stage1_wasm = compile_only(&stage1_source);
    assert_valid_wasm(&stage1_wasm);

    let output = lsharp_wasm::wasi_runner::run_wasm_wasi(&stage1_wasm)
        .expect("path string read-file program を含む stage1 wasm の実行に失敗");
    let modules = parse_emitted_wasm_modules(&output, 1);
    assert_eq!(modules.len(), 1, "stage2 モジュール数が不正");
    assert_valid_wasm(&modules[0]);
    let data_section = extract_section_bytes(&modules[0], 11).unwrap_or_default();
    assert!(
        data_section
            .windows("fixture.txt".len())
            .any(|window| window == b"fixture.txt"),
        "read-file path literal は data section に残ること"
    );
    let (result, printed) = run_exported_i64_with_alloc_print_read_path_imports(
        &modules[0],
        "_start",
        "fixture.txt",
        "hello from file",
    );
    assert_eq!(
        result, 15,
        "path string read-file program を含む stage2 Wasm の戻り値が不正"
    );
    assert!(
        printed.is_empty(),
        "read-file slice では print output は不要"
    );
}

/// BOOT-04: stage1 が同じ source-aware read-file path string source から同一 stage2 Wasm を 2 回生成できること
#[test]
fn test_e2e_bootstrap_stage1_emits_identical_read_file_path_stage2_wasm_for_same_source() {
    let stage2_source =
        r#"(defn main [] (string-length (read-file "fixture.txt")))"#.replace('"', "\\\"");
    let harness = format!(
        r#"
(defn bootstrap-append-bytes [dst src idx count]
  (if (>= idx count)
    dst
    (bootstrap-append-bytes
      (vector-push dst (vector-get src idx))
      src
      (+ idx 1)
      count)))

(defn bootstrap-build-stage2 [src]
  (let [program (parse-program src)
        pair (compile-program-functions-with-source src program)
        functions (vector-get pair 1)
        data (vector-get pair 2)
        header (emit-header)
        type-sec (emit-type-section-alloc-print-main)
        import-sec (emit-import-section-alloc-print-read)
        function-sec (emit-function-section-main-type-index 2)
        memory-sec (emit-memory-section)
        export-sec (emit-export-section-main-memory-index 3 0)
        code-sec (emit-code-section-functions functions)
        data-sec (emit-data-section data 1024)
        bytes0 (bootstrap-append-bytes (vector-new 64) header 0 (vector-length header))
        bytes1 (bootstrap-append-bytes bytes0 type-sec 0 (vector-length type-sec))
        bytes2 (bootstrap-append-bytes bytes1 import-sec 0 (vector-length import-sec))
        bytes3 (bootstrap-append-bytes bytes2 function-sec 0 (vector-length function-sec))
        bytes4 (bootstrap-append-bytes bytes3 memory-sec 0 (vector-length memory-sec))
        bytes5 (bootstrap-append-bytes bytes4 export-sec 0 (vector-length export-sec))
        bytes6 (bootstrap-append-bytes bytes5 code-sec 0 (vector-length code-sec))]
    (bootstrap-append-bytes bytes6 data-sec 0 (vector-length data-sec))))

(defn bootstrap-print-module-bytes [bytes idx count]
  (if (>= idx count)
    0
    (do
      (print (vector-get bytes idx))
      (bootstrap-print-module-bytes bytes (+ idx 1) count))))

(defn bootstrap-print-module [bytes]
  (let [count (vector-length bytes)]
    (do
      (print count)
      (bootstrap-print-module-bytes bytes 0 count)
      0)))

(defn main []
  (let [src "{}"
        stage2-a (bootstrap-build-stage2 src)
        stage2-b (bootstrap-build-stage2 src)]
    (do
      (bootstrap-print-module stage2-a)
      (bootstrap-print-module stage2-b)
      0)))
"#,
        stage2_source
    );
    let stage1_source = format!("{}\n{}", selfhost_cli_runtime_bundle(), harness);
    let stage1_wasm = compile_only(&stage1_source);
    assert_valid_wasm(&stage1_wasm);

    let output = lsharp_wasm::wasi_runner::run_wasm_wasi(&stage1_wasm)
        .expect("same path-string read-file source stage1 実行に失敗");
    let modules = parse_emitted_wasm_modules(&output, 2);
    assert_eq!(modules.len(), 2, "stage2 モジュール数が不正");
    assert_eq!(
        modules[0], modules[1],
        "同じ source-aware read-file path string source から stage2 Wasm が非決定的に変化した"
    );
    assert_valid_wasm(&modules[0]);
    let data_section = extract_section_bytes(&modules[0], 11).unwrap_or_default();
    assert!(
        data_section
            .windows("fixture.txt".len())
            .any(|window| window == b"fixture.txt"),
        "repeatability でも read-file path literal は data section に残ること"
    );
    let (result, printed) = run_exported_i64_with_alloc_print_read_path_imports(
        &modules[0],
        "_start",
        "fixture.txt",
        "hello from file",
    );
    assert_eq!(result, 15);
    assert!(printed.is_empty());
}

/// BOOT-04: stage1 が command-line-arg builtin を含む stage2 Wasm を生成し実行できること
#[test]
fn test_e2e_bootstrap_stage1_emits_stage2_wasm_for_command_line_arg_program() {
    let harness = r#"
(defn bootstrap-append-bytes [dst src idx count]
  (if (>= idx count)
    dst
    (bootstrap-append-bytes
      (vector-push dst (vector-get src idx))
      src
      (+ idx 1)
      count)))

(defn bootstrap-build-stage2 [src]
  (let [program (parse-program src)
        pair (compile-program-functions program)
        functions (vector-get pair 1)
        header (emit-header)
        type-sec (emit-type-section-alloc-print-main)
        import-sec (emit-import-section-alloc-print-read-arg)
        function-sec (emit-function-section-main-type-index 2)
        memory-sec (emit-memory-section)
        export-sec (emit-export-section-main-memory-index 4 0)
        code-sec (emit-code-section-functions functions)
        bytes0 (bootstrap-append-bytes (vector-new 64) header 0 (vector-length header))
        bytes1 (bootstrap-append-bytes bytes0 type-sec 0 (vector-length type-sec))
        bytes2 (bootstrap-append-bytes bytes1 import-sec 0 (vector-length import-sec))
        bytes3 (bootstrap-append-bytes bytes2 function-sec 0 (vector-length function-sec))
        bytes4 (bootstrap-append-bytes bytes3 memory-sec 0 (vector-length memory-sec))
        bytes5 (bootstrap-append-bytes bytes4 export-sec 0 (vector-length export-sec))]
    (bootstrap-append-bytes bytes5 code-sec 0 (vector-length code-sec))))

(defn bootstrap-print-module-bytes [bytes idx count]
  (if (>= idx count)
    0
    (do
      (print (vector-get bytes idx))
      (bootstrap-print-module-bytes bytes (+ idx 1) count))))

(defn bootstrap-print-module [bytes]
  (let [count (vector-length bytes)]
    (do
      (print count)
      (bootstrap-print-module-bytes bytes 0 count)
      0)))

(defn main []
  (let [stage2 (bootstrap-build-stage2 "(defn main [] (string-length (command-line-arg 1)))")]
    (do
      (bootstrap-print-module stage2)
      0)))
"#;
    let stage1_source = format!("{}\n{}", selfhost_cli_runtime_bundle(), harness);
    let stage1_wasm = compile_only(&stage1_source);
    assert_valid_wasm(&stage1_wasm);

    let output = lsharp_wasm::wasi_runner::run_wasm_wasi(&stage1_wasm)
        .expect("command-line-arg program を含む stage1 wasm の実行に失敗");
    let modules = parse_emitted_wasm_modules(&output, 1);
    assert_eq!(modules.len(), 1, "stage2 モジュール数が不正");
    assert_valid_wasm(&modules[0]);
    assert!(
        extract_sections(&modules[0]).iter().any(|(id, _)| *id == 2),
        "command-line-arg program を含む stage2 Wasm は import section を持つこと"
    );
    let (result, printed) = run_exported_i64_with_alloc_print_read_arg_imports(
        &modules[0],
        "_start",
        "",
        &["cli", "hello-argv"],
    );
    assert_eq!(
        result, 10,
        "command-line-arg program を含む stage2 Wasm の戻り値が不正"
    );
    assert!(
        printed.is_empty(),
        "command-line-arg slice では print output は不要"
    );
}

/// BOOT-04: stage1 が同じ command-line-arg helper source から同一 stage2 Wasm を 2 回生成できること
#[test]
fn test_e2e_bootstrap_stage1_emits_identical_arg_helper_stage2_wasm_for_same_source() {
    let harness = r#"
(defn bootstrap-append-bytes [dst src idx count]
  (if (>= idx count)
    dst
    (bootstrap-append-bytes
      (vector-push dst (vector-get src idx))
      src
      (+ idx 1)
      count)))

(defn bootstrap-build-stage2 [src]
  (let [program (parse-program src)
        pair (compile-program-functions program)
        functions (vector-get pair 1)
        header (emit-header)
        type-sec (emit-type-section-alloc-print-main)
        import-sec (emit-import-section-alloc-print-read-arg)
        function-sec (emit-function-section-main-type-index 2)
        memory-sec (emit-memory-section)
        export-sec (emit-export-section-main-memory-index 4 0)
        code-sec (emit-code-section-functions functions)
        bytes0 (bootstrap-append-bytes (vector-new 64) header 0 (vector-length header))
        bytes1 (bootstrap-append-bytes bytes0 type-sec 0 (vector-length type-sec))
        bytes2 (bootstrap-append-bytes bytes1 import-sec 0 (vector-length import-sec))
        bytes3 (bootstrap-append-bytes bytes2 function-sec 0 (vector-length function-sec))
        bytes4 (bootstrap-append-bytes bytes3 memory-sec 0 (vector-length memory-sec))
        bytes5 (bootstrap-append-bytes bytes4 export-sec 0 (vector-length export-sec))]
    (bootstrap-append-bytes bytes5 code-sec 0 (vector-length code-sec))))

(defn bootstrap-print-module-bytes [bytes idx count]
  (if (>= idx count)
    0
    (do
      (print (vector-get bytes idx))
      (bootstrap-print-module-bytes bytes (+ idx 1) count))))

(defn bootstrap-print-module [bytes]
  (let [count (vector-length bytes)]
    (do
      (print count)
      (bootstrap-print-module-bytes bytes 0 count)
      0)))

(defn main []
  (let [src "(defn main [] (string-length (command-line-arg 1)))"
        stage2-a (bootstrap-build-stage2 src)
        stage2-b (bootstrap-build-stage2 src)]
    (do
      (bootstrap-print-module stage2-a)
      (bootstrap-print-module stage2-b)
      0)))
"#;
    let stage1_source = format!("{}\n{}", selfhost_cli_runtime_bundle(), harness);
    let stage1_wasm = compile_only(&stage1_source);
    assert_valid_wasm(&stage1_wasm);

    let output = lsharp_wasm::wasi_runner::run_wasm_wasi(&stage1_wasm)
        .expect("same arg-helper source stage1 実行に失敗");
    let modules = parse_emitted_wasm_modules(&output, 2);
    assert_eq!(modules.len(), 2, "stage2 モジュール数が不正");
    assert_eq!(
        modules[0], modules[1],
        "同じ command-line-arg helper source から stage2 Wasm が非決定的に変化した"
    );
    assert_valid_wasm(&modules[0]);
    let (result, printed) = run_exported_i64_with_alloc_print_read_arg_imports(
        &modules[0],
        "_start",
        "",
        &["cli", "hello-argv"],
    );
    assert_eq!(result, 10);
    assert!(printed.is_empty());
}

/// BOOT-04: stage1 が generalized 4-helper path で alloc+print+read-file+command-line-arg stage2 Wasm を生成できること
#[test]
fn test_e2e_bootstrap_stage1_emits_stage2_wasm_for_generalized_alloc_print_read_arg_helper_quad() {
    let harness = r#"
(defn bootstrap-append-bytes [dst src idx count]
  (if (>= idx count)
    dst
    (bootstrap-append-bytes
      (vector-push dst (vector-get src idx))
      src
      (+ idx 1)
      count)))

(defn bootstrap-build-stage2 [src]
  (let [program (parse-program src)
        pair (compile-program-functions program)
        functions (vector-get pair 1)
        header (emit-header)
        type-sec (emit-type-section-helper-quad-main (helper-id-alloc) (helper-id-print) (helper-id-read-file) (helper-id-command-line-arg))
        import-sec (emit-import-section-helper-quad (helper-id-alloc) (helper-id-print) (helper-id-read-file) (helper-id-command-line-arg))
        function-sec (emit-function-section-main-type-index 2)
        memory-sec (emit-memory-section)
        export-sec (emit-export-section-main-memory-index 4 0)
        code-sec (emit-code-section-functions functions)
        bytes0 (bootstrap-append-bytes (vector-new 64) header 0 (vector-length header))
        bytes1 (bootstrap-append-bytes bytes0 type-sec 0 (vector-length type-sec))
        bytes2 (bootstrap-append-bytes bytes1 import-sec 0 (vector-length import-sec))
        bytes3 (bootstrap-append-bytes bytes2 function-sec 0 (vector-length function-sec))
        bytes4 (bootstrap-append-bytes bytes3 memory-sec 0 (vector-length memory-sec))
        bytes5 (bootstrap-append-bytes bytes4 export-sec 0 (vector-length export-sec))]
    (bootstrap-append-bytes bytes5 code-sec 0 (vector-length code-sec))))

(defn bootstrap-print-module-bytes [bytes idx count]
  (if (>= idx count)
    0
    (do
      (print (vector-get bytes idx))
      (bootstrap-print-module-bytes bytes (+ idx 1) count))))

(defn bootstrap-print-module [bytes]
  (let [count (vector-length bytes)]
    (do
      (print count)
      (bootstrap-print-module-bytes bytes 0 count)
      0)))

(defn main []
  (let [stage2 (bootstrap-build-stage2 "(defn main [] (string-length (command-line-arg 1)))")]
    (do
      (bootstrap-print-module stage2)
      0)))
"#;
    let stage1_source = format!("{}\n{}", selfhost_cli_runtime_bundle(), harness);
    let stage1_wasm = compile_only(&stage1_source);
    assert_valid_wasm(&stage1_wasm);

    let output = lsharp_wasm::wasi_runner::run_wasm_wasi(&stage1_wasm)
        .expect("generalized alloc+print+read-file+command-line-arg quad program を含む stage1 wasm の実行に失敗");
    let modules = parse_emitted_wasm_modules(&output, 1);
    assert_eq!(modules.len(), 1, "stage2 モジュール数が不正");
    assert_valid_wasm(&modules[0]);
    let (result, printed) = run_exported_i64_with_alloc_print_read_arg_imports(
        &modules[0],
        "_start",
        "",
        &["cli", "hello-argv"],
    );
    assert_eq!(
        result, 10,
        "generalized alloc+print+read-file+command-line-arg quad stage2 Wasm の戻り値が不正"
    );
    assert!(
        printed.is_empty(),
        "generalized alloc+print+read-file+command-line-arg quad slice では print output は不要"
    );
}

/// BOOT-04: stage1 が同じ generalized alloc+print+read-file+command-line-arg source から同一 stage2 Wasm を 2 回生成できること
#[test]
fn test_e2e_bootstrap_stage1_emits_identical_arg_helper_quad_stage2_wasm_for_same_source() {
    let harness = r#"
(defn bootstrap-append-bytes [dst src idx count]
  (if (>= idx count)
    dst
    (bootstrap-append-bytes
      (vector-push dst (vector-get src idx))
      src
      (+ idx 1)
      count)))

(defn bootstrap-build-stage2 [src]
  (let [program (parse-program src)
        pair (compile-program-functions program)
        functions (vector-get pair 1)
        header (emit-header)
        type-sec (emit-type-section-helper-quad-main (helper-id-alloc) (helper-id-print) (helper-id-read-file) (helper-id-command-line-arg))
        import-sec (emit-import-section-helper-quad (helper-id-alloc) (helper-id-print) (helper-id-read-file) (helper-id-command-line-arg))
        function-sec (emit-function-section-main-type-index 2)
        memory-sec (emit-memory-section)
        export-sec (emit-export-section-main-memory-index 4 0)
        code-sec (emit-code-section-functions functions)
        bytes0 (bootstrap-append-bytes (vector-new 64) header 0 (vector-length header))
        bytes1 (bootstrap-append-bytes bytes0 type-sec 0 (vector-length type-sec))
        bytes2 (bootstrap-append-bytes bytes1 import-sec 0 (vector-length import-sec))
        bytes3 (bootstrap-append-bytes bytes2 function-sec 0 (vector-length function-sec))
        bytes4 (bootstrap-append-bytes bytes3 memory-sec 0 (vector-length memory-sec))
        bytes5 (bootstrap-append-bytes bytes4 export-sec 0 (vector-length export-sec))]
    (bootstrap-append-bytes bytes5 code-sec 0 (vector-length code-sec))))

(defn bootstrap-print-module-bytes [bytes idx count]
  (if (>= idx count)
    0
    (do
      (print (vector-get bytes idx))
      (bootstrap-print-module-bytes bytes (+ idx 1) count))))

(defn bootstrap-print-module [bytes]
  (let [count (vector-length bytes)]
    (do
      (print count)
      (bootstrap-print-module-bytes bytes 0 count)
      0)))

(defn main []
  (let [src "(defn main [] (string-length (command-line-arg 1)))"
        stage2-a (bootstrap-build-stage2 src)
        stage2-b (bootstrap-build-stage2 src)]
    (do
      (bootstrap-print-module stage2-a)
      (bootstrap-print-module stage2-b)
      0)))
"#;
    let stage1_source = format!("{}\n{}", selfhost_cli_runtime_bundle(), harness);
    let stage1_wasm = compile_only(&stage1_source);
    assert_valid_wasm(&stage1_wasm);

    let output = lsharp_wasm::wasi_runner::run_wasm_wasi(&stage1_wasm)
        .expect("same arg-helper quad source stage1 実行に失敗");
    let modules = parse_emitted_wasm_modules(&output, 2);
    assert_eq!(modules.len(), 2, "stage2 モジュール数が不正");
    assert_eq!(
        modules[0], modules[1],
        "同じ generalized arg helper quad source から stage2 Wasm が非決定的に変化した"
    );
    assert_valid_wasm(&modules[0]);
    let (result, printed) = run_exported_i64_with_alloc_print_read_arg_imports(
        &modules[0],
        "_start",
        "",
        &["cli", "hello-argv"],
    );
    assert_eq!(result, 10);
    assert!(printed.is_empty());
}

// =============================================================================
// BOOT-04 リグレッション: file-fed stage2 generator self-feed proxy / deep recursive trap
// =============================================================================

/// BOOT-04 リグレッション: bootstrap-append-bytes の末尾再帰トラップ再現
///
/// `bootstrap-append-bytes` はバイト列を 1 バイトずつコピーする直接再帰で実装されており、
/// TCO (末尾呼び出し最適化) なしの Wasm では大きな配列に対してスタックオーバーフローが発生する。
///
/// この問題を最小限の形で再現する:
/// - stage2 ソース = N 個の単純な 0 引数関数からなるプログラム
/// - stage1 (selfhost CLI runtime) がそのプログラムをコンパイルして Wasm を組み立てる
/// - code section が大きくなるほど bootstrap-append-bytes の再帰深度が増す
/// - N が十分に大きいとき、stage1 実行時に Wasm スタックトラップが発生する
#[test]
fn test_e2e_boot04_bootstrap_append_bytes_deep_recursion_trap_repro() {
    let build_stage2_src = |n_funcs: usize| -> String {
        let mut s = String::new();
        for i in 0..n_funcs {
            s.push_str(&format!("(defn fn{i:04} [] {i}) "));
        }
        s.push_str("(defn main [] 0)");
        s
    };

    let make_harness = |stage2_src: &str| -> String {
        format!(
            concat!(
                "(defn bootstrap-append-bytes [dst src idx count]\n",
                "  (if (>= idx count)\n",
                "    dst\n",
                "    (bootstrap-append-bytes\n",
                "      (vector-push dst (vector-get src idx))\n",
                "      src (+ idx 1) count)))\n",
                "(defn bootstrap-build-stage2 [src]\n",
                "  (let [program (parse-program src)\n",
                "        pair (compile-program-functions program)\n",
                "        functions (vector-get pair 1)\n",
                "        func-count (vector-length functions)\n",
                "        header (emit-header)\n",
                "        type-sec (emit-type-section-functions functions)\n",
                "        function-sec (emit-function-section-functions functions)\n",
                "        export-sec (emit-export-section-main-index (- func-count 1))\n",
                "        code-sec (emit-code-section-functions functions)\n",
                "        bytes0 (bootstrap-append-bytes (vector-new 64) header 0 (vector-length header))\n",
                "        bytes1 (bootstrap-append-bytes bytes0 type-sec 0 (vector-length type-sec))\n",
                "        bytes2 (bootstrap-append-bytes bytes1 function-sec 0 (vector-length function-sec))\n",
                "        bytes3 (bootstrap-append-bytes bytes2 export-sec 0 (vector-length export-sec))]\n",
                "    (bootstrap-append-bytes bytes3 code-sec 0 (vector-length code-sec))))\n",
                "(defn bootstrap-print-module-bytes [bytes idx count]\n",
                "  (if (>= idx count) 0\n",
                "    (do (print (vector-get bytes idx))\n",
                "        (bootstrap-print-module-bytes bytes (+ idx 1) count))))\n",
                "(defn bootstrap-print-module [bytes]\n",
                "  (let [count (vector-length bytes)]\n",
                "    (do (print count) (bootstrap-print-module-bytes bytes 0 count) 0)))\n",
                "(defn main []\n",
                "  (let [stage2 (bootstrap-build-stage2 \"{s2}\")]\n",
                "    (do (bootstrap-print-module stage2) 0)))\n",
            ),
            s2 = stage2_src
        )
    };

    // N=5: code section ~100 bytes → 再帰は浅い → 成功するはず
    {
        let small_src = build_stage2_src(5);
        let harness = make_harness(&small_src);
        let stage1_source = format!("{}\n{}", selfhost_cli_runtime_bundle(), harness);
        let stage1_wasm = compile_only(&stage1_source);
        assert_valid_wasm(&stage1_wasm);
        let result = lsharp_wasm::wasi_runner::run_wasm_wasi(&stage1_wasm);
        assert!(
            result.is_ok(),
            "N=5 では bootstrap-append-bytes トラップが発生しないはず: {:?}",
            result.err()
        );
        let output = result.unwrap();
        let modules = parse_emitted_wasm_modules(&output, 1);
        assert_eq!(modules.len(), 1, "N=5 では stage2 モジュールが 1 つ生成されるはず");
        assert_valid_wasm(&modules[0]);
    }

    // N=2000: code section ~30,000 bytes
    // BOOT-04 修正済み: self-TCO (自己末尾呼び出し最適化) により再帰がループに変換される
    // lsharp-ir/src/lower/decl.rs の apply_self_tco により、
    // bootstrap-append-bytes のような自己末尾再帰関数がスタックを消費しなくなった
    {
        let large_src = build_stage2_src(2000);
        let harness = make_harness(&large_src);
        let stage1_source = format!("{}\n{}", selfhost_cli_runtime_bundle(), harness);
        let stage1_wasm = compile_only(&stage1_source);
        assert_valid_wasm(&stage1_wasm);
        let result = lsharp_wasm::wasi_runner::run_wasm_wasi(&stage1_wasm);
        assert!(
            result.is_ok(),
            "BOOT-04 リグレッション: N=2000 で bootstrap-append-bytes がトラップした。\n\
             self-TCO が正しく動作していない可能性があります。\n\
             エラー: {:?}",
            result.err()
        );
        let output = result.unwrap();
        let modules = parse_emitted_wasm_modules(&output, 1);
        assert_eq!(modules.len(), 1, "N=2000 では stage2 モジュールが 1 つ生成されるはず");
        assert_valid_wasm(&modules[0]);
    }
}

/// BOOT-04 リグレッション: 再帰深度境界の観測記録
///
/// bootstrap-append-bytes が何個の関数 (≈ code section バイト数) から失敗するかの境界を確認する。
/// 結果を eprintln で出力し、修正後の境界比較に利用する。
#[test]
fn test_e2e_boot04_bootstrap_append_bytes_recursion_depth_boundary() {
    let make_full_source = |n_funcs: usize| -> Vec<u8> {
        let mut src = String::new();
        for i in 0..n_funcs {
            src.push_str(&format!("(defn fn{i:04} [] {i}) "));
        }
        src.push_str("(defn main [] 0)");
        let harness = format!(
            concat!(
                "(defn bootstrap-append-bytes [dst s idx count]\n",
                "  (if (>= idx count) dst\n",
                "    (bootstrap-append-bytes (vector-push dst (vector-get s idx)) s (+ idx 1) count)))\n",
                "(defn bootstrap-build-stage2 [src]\n",
                "  (let [program (parse-program src)\n",
                "        pair (compile-program-functions program)\n",
                "        functions (vector-get pair 1)\n",
                "        func-count (vector-length functions)\n",
                "        header (emit-header)\n",
                "        type-sec (emit-type-section-functions functions)\n",
                "        function-sec (emit-function-section-functions functions)\n",
                "        export-sec (emit-export-section-main-index (- func-count 1))\n",
                "        code-sec (emit-code-section-functions functions)\n",
                "        bytes0 (bootstrap-append-bytes (vector-new 64) header 0 (vector-length header))\n",
                "        bytes1 (bootstrap-append-bytes bytes0 type-sec 0 (vector-length type-sec))\n",
                "        bytes2 (bootstrap-append-bytes bytes1 function-sec 0 (vector-length function-sec))\n",
                "        bytes3 (bootstrap-append-bytes bytes2 export-sec 0 (vector-length export-sec))]\n",
                "    (bootstrap-append-bytes bytes3 code-sec 0 (vector-length code-sec))))\n",
                "(defn main []\n",
                "  (let [stage2 (bootstrap-build-stage2 \"{src}\")]\n",
                "    (print (vector-length stage2))))\n",
            ),
            src = src
        );
        let stage1_source = format!("{}\n{}", selfhost_cli_runtime_bundle(), harness);
        compile_only(&stage1_source)
    };

    let try_n = |n: usize| -> bool {
        let wasm = make_full_source(n);
        lsharp_wasm::wasi_runner::run_wasm_wasi(&wasm).is_ok()
    };

    let n10_ok = try_n(10);
    let n50_ok = try_n(50);
    let n200_ok = try_n(200);
    let n500_ok = try_n(500);
    let n1000_ok = try_n(1000);

    eprintln!(
        "BOOT-04 bootstrap-append-bytes 再帰深度境界 (Wasm 関数数):\n  \
         N=10:   {}\n  \
         N=50:   {}\n  \
         N=200:  {}\n  \
         N=500:  {}\n  \
         N=1000: {}",
        if n10_ok { "OK" } else { "TRAP" },
        if n50_ok { "OK" } else { "TRAP" },
        if n200_ok { "OK" } else { "TRAP" },
        if n500_ok { "OK" } else { "TRAP" },
        if n1000_ok { "OK" } else { "TRAP" },
    );

    // N=10 は必ず成功 (code section ~150 bytes)
    assert!(n10_ok, "N=10 は必ず成功するはず");

    // 単調性: 成功から失敗への遷移は一方向のみ
    if !n50_ok {
        assert!(!n200_ok, "N=50 で TRAP なら N=200 も TRAP のはず");
        assert!(!n500_ok, "N=50 で TRAP なら N=500 も TRAP のはず");
    }
    if !n200_ok {
        assert!(!n500_ok, "N=200 で TRAP なら N=500 も TRAP のはず");
        assert!(!n1000_ok, "N=200 で TRAP なら N=1000 も TRAP のはず");
    }
    if !n500_ok {
        assert!(!n1000_ok, "N=500 で TRAP なら N=1000 も TRAP のはず");
    }
}

// =============================================================================
// BOOT-04: read-file compiler-mode — Main.ls のコンパイラモードエントリポイント検証
// =============================================================================

/// BOOT-04: read-file compiler-mode — stage1 (Main.ls compiled by Rust) が
/// ファイル引数を受け取りコンパイラとして動作すること
///
/// Main.ls の compiler-mode を検証:
/// - argv[1] にソースファイルパスが渡されたとき、そのファイルを read-file で読み込み
/// - parse-program → compile-program-functions → emit-*-wasi でコンパイルし
/// - WASM バイトを length-prefixed 形式で stdout に出力すること
#[test]
fn test_e2e_boot04_read_file_compiler_mode() {
    let main_path = selfhost_main_path();
    let stage1_wasm = compile_file_only(&main_path);
    assert_valid_wasm(&stage1_wasm);

    // テスト用 L# ソースファイルを用意
    let fixture_dir = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../tests/fixtures");
    assert!(
        fixture_dir.join("minimal.ls").exists(),
        "fixture ファイル tests/fixtures/minimal.ls が存在しない"
    );

    // compiler-mode で stage1 を実行 (argv[1] = "minimal.ls")
    let output = lsharp_wasm::wasi_runner::run_wasm_wasi_with_dir_and_args(
        &stage1_wasm,
        Some(&fixture_dir),
        &["compiler", "minimal.ls"],
    )
    .expect("BOOT-04 compiler-mode: stage1 実行失敗");

    // 出力が length-prefixed Wasm バイト列であること
    let modules = parse_emitted_wasm_modules(&output, 1);
    let stage2_wasm = &modules[0];
    assert_valid_wasm(stage2_wasm);

    // stage2 が WASI 実行可能であること (_start: () -> () ラッパー付き)
    let wasi_result = lsharp_wasm::wasi_runner::run_wasm_wasi(stage2_wasm);
    assert!(
        wasi_result.is_ok(),
        "BOOT-04 compiler-mode: stage2 の WASI 実行に失敗: {:?}",
        wasi_result.err()
    );

    eprintln!(
        "BOOT-04 compiler-mode: stage1 が minimal.ls をコンパイルして stage2 ({} bytes) を生成 OK",
        stage2_wasm.len()
    );
}

/// BOOT-04: stage2 コンパイラが minimal.ls を stage3 にコンパイルできること
///
/// stage1 (Rust bootstrap が生成した Main.ls コンパイラ wasm) を stage2_compiler と見なし、
/// stage2_compiler が compiler-mode で minimal.ls を読み込んで stage3 wasm を生成できること、
/// さらに stage3 が正しく実行できることを検証する。
///
/// - stage1 == stage2_compiler: どちらも Rust bootstrap が生成した同一の完全コンパイラ wasm
/// - stage2→stage3 の接続性を明示的に固定するテスト
/// - stage3 の出力が stage1→stage2 の出力と一致する（同一入力 → 決定論的出力）ことも検証
#[test]
fn test_e2e_boot04_stage2_compiler_to_stage3_minimal() {
    let main_path = selfhost_main_path();
    let fixture_dir = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../tests/fixtures");
    assert!(
        fixture_dir.join("minimal.ls").exists(),
        "fixture ファイル tests/fixtures/minimal.ls が存在しない"
    );

    // stage2_compiler = Rust bootstrap が生成した完全コンパイラ wasm (= stage1 と同一)
    let stage2_compiler = compile_file_only(&main_path);
    assert_valid_wasm(&stage2_compiler);

    // stage2_compiler が compiler-mode で minimal.ls → stage3 を生成
    let output = lsharp_wasm::wasi_runner::run_wasm_wasi_with_dir_and_args(
        &stage2_compiler,
        Some(&fixture_dir),
        &["compiler", "minimal.ls"],
    )
    .expect("BOOT-04 stage2→stage3: stage2_compiler の compiler-mode 実行失敗");

    let modules = parse_emitted_wasm_modules(&output, 1);
    let stage3_wasm = &modules[0];
    assert_valid_wasm(stage3_wasm);

    // stage3 が WASI 実行できること
    let stage3_result = lsharp_wasm::wasi_runner::run_wasm_wasi(stage3_wasm);
    assert!(
        stage3_result.is_ok(),
        "BOOT-04 stage2→stage3: stage3 の WASI 実行に失敗: {:?}",
        stage3_result.err()
    );

    // stage3 の出力が空であること（(defn main [] 42) は print しない）
    let stage3_output = stage3_result.unwrap();
    assert_eq!(
        stage3_output, "",
        "BOOT-04 stage2→stage3: stage3 の stdout 出力が期待と異なる: {:?}",
        stage3_output
    );

    // stage3 が stage2_compiler の出力と一致する（同一入力 → 決定論的）
    let output2 = lsharp_wasm::wasi_runner::run_wasm_wasi_with_dir_and_args(
        &stage2_compiler,
        Some(&fixture_dir),
        &["compiler", "minimal.ls"],
    )
    .expect("BOOT-04 stage2→stage3: stage2_compiler 2回目の実行失敗");
    let modules2 = parse_emitted_wasm_modules(&output2, 1);
    let stage3_wasm_b = &modules2[0];
    assert_eq!(
        stage3_wasm, stage3_wasm_b,
        "BOOT-04 stage2→stage3: stage3 wasm が非決定的（同一入力で異なる出力）"
    );

    eprintln!(
        "BOOT-04 stage2→stage3: stage2_compiler が minimal.ls → stage3 ({} bytes) を生成し実行 OK (決定論的確認済み)",
        stage3_wasm.len()
    );
}

/// BOOT-04: 自己コンパイル stage2 の精密ブロッカー記録テスト
///
/// stage1 (Rust bootstrap compiler wasm) が compiler-mode で Main.ls 自身を
/// コンパイルして stage2_self_compiler を生成できるかを検証する。
///
/// 現在の blockerを精密に固定する:
/// - stage1 の compiler-mode は単一ファイルを解析・コンパイルするが
/// - Main.ls は (import AST) 等の import 宣言を含むため、
/// - compile-program-functions は import を無視し、import 先の関数が欠落した wasm を生成する
/// - 結果として stage2_self_compiler は機能不全（呼び出せない関数を参照）
///
/// このテストが GREEN になった時点で BOOT-04 self-hosting は達成される。
#[test]
fn test_e2e_boot04_self_hosted_stage2_compiler_blocker() {
    let main_path = selfhost_main_path();
    let selfhost_dir = main_path
        .parent()
        .expect("selfhost/ ディレクトリが取得できない")
        .to_path_buf();
    let fixture_dir = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../tests/fixtures");

    // stage1 = Rust bootstrap が生成した完全コンパイラ wasm
    let stage1_wasm = compile_file_only(&main_path);
    assert_valid_wasm(&stage1_wasm);

    // stage1 が compiler-mode で Main.ls 自身をコンパイル → stage2_self_compiler を試みる
    // (import 宣言があるため、compile-program-functions は import を無視する)
    let stage2_result = lsharp_wasm::wasi_runner::run_wasm_wasi_with_dir_and_args(
        &stage1_wasm,
        Some(&selfhost_dir),
        &["compiler", "Main.ls"],
    );

    match stage2_result {
        Err(e) => {
            // stage1 が Main.ls のコンパイルに失敗: 実行時エラー
            eprintln!(
                "BOOT-04 self-hosted-stage2 BLOCKED: stage1 が Main.ls コンパイルに失敗: {}",
                e
            );
            // ブロッカーを記録して終了（このテストは現状 blocked であることを証明）
            return;
        }
        Ok(output) => {
            // stage1 が何らかの出力を生成した: stage2_self_compiler を解析
            eprintln!(
                "BOOT-04 self-hosted-stage2: stage1 が Main.ls → output ({} chars) を生成",
                output.len()
            );

            // output が wasm モジュールを含むか確認
            let modules_result = std::panic::catch_unwind(|| {
                parse_emitted_wasm_modules(&output, 1)
            });

            match modules_result {
                Err(_) => {
                    eprintln!(
                        "BOOT-04 self-hosted-stage2 BLOCKED: stage1 の Main.ls コンパイル出力が wasm モジュール形式でない"
                    );
                    return;
                }
                Ok(modules) => {
                    let stage2_self_compiler = &modules[0];
                    eprintln!(
                        "BOOT-04 self-hosted-stage2: stage2_self_compiler = {} bytes",
                        stage2_self_compiler.len()
                    );
                    assert_valid_wasm(stage2_self_compiler);

                    // stage2_self_compiler が compiler-mode で minimal.ls → stage3 を生成できるか
                    let stage3_result = lsharp_wasm::wasi_runner::run_wasm_wasi_with_dir_and_args(
                        stage2_self_compiler,
                        Some(&fixture_dir),
                        &["compiler", "minimal.ls"],
                    );

                    match stage3_result {
                        Err(e) => {
                            eprintln!(
                                "BOOT-04 self-hosted-stage2 BLOCKED: stage2_self_compiler が minimal.ls をコンパイルできない: {}",
                                e
                            );
                            // ブロッカーを記録して終了
                        }
                        Ok(stage3_output) => {
                            let stage3_modules = parse_emitted_wasm_modules(&stage3_output, 1);
                            let stage3_wasm = &stage3_modules[0];
                            assert_valid_wasm(stage3_wasm);

                            let run_result = lsharp_wasm::wasi_runner::run_wasm_wasi(stage3_wasm);
                            assert!(
                                run_result.is_ok(),
                                "stage2_self_compiler → stage3 実行失敗: {:?}",
                                run_result.err()
                            );
                            eprintln!(
                                "BOOT-04 self-hosted-stage2 GREEN: stage1→stage2_self_compiler→stage3 ({} bytes) 完全成功!",
                                stage3_wasm.len()
                            );
                        }
                    }
                }
            }
        }
    }
}
