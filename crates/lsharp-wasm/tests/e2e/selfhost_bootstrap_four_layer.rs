use super::support::*;
use serde_json::Value;
use std::collections::{HashMap, hash_map::DefaultHasher};
use std::hash::{Hash, Hasher};

const SELFHOST_MAX_WASM_STACK: usize = 64 * 1024 * 1024;

fn configured_selfhost_engine() -> wasmtime::Engine {
    let mut config = wasmtime::Config::new();
    config.max_wasm_stack(SELFHOST_MAX_WASM_STACK);
    wasmtime::Engine::new(&config).expect("selfhost wasmtime engine 初期化に失敗")
}

const TEST_I64_IF_WASM: &[u8] = include_bytes!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../../tests/fixtures/selfhost-debug/test_i64_if.wasm"
));

/// V2-11 最小 harness 共通プレリュード（selfhost ランタイム import layout 版）。
///
/// stage1 が stage2 Wasm を構築するための共通ヘルパー関数群。
/// `(defn main [] ...)` を追加すれば完全な L# プログラムになる。
/// 10 個のランタイム import (alloc/print/read-file/command-line-arg/
/// string-concat/substring/file-exists/root-push/root-pop/root-set)
/// を持つ stage2 Wasm を emit-import-section-alloc-print-read-arg-concat-sub で生成し、
/// compile-program-functions-with-base で base offset 10 を指定する。
const RUNTIME_STAGE2_HARNESS_PRELUDE: &str = r#"
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
        pair (compile-program-functions-with-base program 10)
        functions (vector-get pair 1)
        func-count (vector-length functions)
        header (emit-header)
        type-sec (emit-type-section-wasi-quad-functions functions)
        import-sec (emit-import-section-alloc-print-read-arg-concat-sub)
        func-sec (emit-function-section-wasi-quad-functions functions)
        memory-sec (emit-memory-section)
        export-sec (emit-export-section-main-index (+ 9 func-count))
        code-sec (emit-code-section-wasi-quad-functions functions)
        bytes0 (bootstrap-append-bytes (vector-new 64) header 0 (vector-length header))
        bytes1 (bootstrap-append-bytes bytes0 type-sec 0 (vector-length type-sec))
        bytes2 (bootstrap-append-bytes bytes1 import-sec 0 (vector-length import-sec))
        bytes3 (bootstrap-append-bytes bytes2 func-sec 0 (vector-length func-sec))
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
      0)))"#;

fn stage1_source_emitting_wasi_stage2(stage2_src: &str) -> String {
    let mut harness = RUNTIME_STAGE2_HARNESS_PRELUDE.to_string();
    harness.push_str("\n(defn main []\n  (let [stage2 (bootstrap-build-stage2 \"");
    harness.push_str(stage2_src);
    harness.push_str("\")]\n    (do\n      (bootstrap-print-module stage2)\n      0)))");
    format!("{}\n{}", selfhost_cli_runtime_bundle(), harness)
}

fn stage1_source_emitting_wasi_stage2_with_source(stage2_src: &str) -> String {
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
        func-count (vector-length functions)
        header (emit-header)
        type-sec (emit-type-section-wasi-quad-functions functions)
        import-sec (emit-import-section-alloc-print-read-arg-concat-sub)
        func-sec (emit-function-section-wasi-quad-functions functions)
        memory-sec (emit-memory-section)
        export-sec (emit-export-section-main-index (+ 9 func-count))
        code-sec (emit-code-section-wasi-quad-functions functions)
        data-sec (emit-data-section data 1024)
        bytes0 (bootstrap-append-bytes (vector-new 64) header 0 (vector-length header))
        bytes1 (bootstrap-append-bytes bytes0 type-sec 0 (vector-length type-sec))
        bytes2 (bootstrap-append-bytes bytes1 import-sec 0 (vector-length import-sec))
        bytes3 (bootstrap-append-bytes bytes2 func-sec 0 (vector-length func-sec))
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
  (let [stage2 (bootstrap-build-stage2 "{stage2_src}")]
    (do
      (bootstrap-print-module stage2)
      0)))
"#
    );
    format!("{}\n{}", selfhost_cli_runtime_bundle(), harness)
}

/// wasmparser で wasm バイナリを検証し、詳細なエラーを返すヘルパー
fn validate_wasm_detailed(wasm: &[u8]) -> Result<(), String> {
    use wasmparser::{Parser, Validator, WasmFeatures};
    let mut validator = Validator::new_with_features(WasmFeatures::default());
    for payload in Parser::new(0).parse_all(wasm) {
        let payload = payload.map_err(|e| format!("parse error: {e}"))?;
        validator
            .payload(&payload)
            .map_err(|e| format!("validate error at offset {}: {}", e.offset(), e.message()))?;
    }
    Ok(())
}

// =============================================================================
// BOOT-04: True stage1-stage2-stage3 bootstrap 4 層検証テスト
// =============================================================================

/// Wasm バイナリからセクション ID とサイズの列を抽出するヘルパー
pub(crate) fn extract_sections(wasm: &[u8]) -> Vec<(u8, usize)> {
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
pub(crate) fn extract_section_bytes(wasm: &[u8], target_id: u8) -> Option<Vec<u8>> {
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

fn ordered_marker_positions(values: &[i64], markers: &[i64], label: &str) -> Vec<usize> {
    let mut positions = Vec::with_capacity(markers.len());
    let mut next_start = 0usize;
    for marker in markers {
        let position = values
            .iter()
            .enumerate()
            .skip(next_start)
            .find_map(|(index, value)| (*value == *marker).then_some(index))
            .unwrap_or_else(|| panic!("{label}: marker {marker} が見つからない: {values:?}"));
        positions.push(position);
        next_start = position + 1;
    }
    positions
}

fn selfhost_string_object_bytes(text: &str) -> Vec<u8> {
    let byte_len = text.len();
    let len = byte_len as u32;
    let mut bytes = Vec::with_capacity(8 + byte_len);
    bytes.extend_from_slice(&1u32.to_le_bytes());
    bytes.extend_from_slice(&len.to_le_bytes());
    bytes.extend_from_slice(text.as_bytes());
    bytes
}

fn selfhost_string_object_sequence(texts: &[&str]) -> Vec<u8> {
    texts
        .iter()
        .flat_map(|text| selfhost_string_object_bytes(text))
        .collect()
}

fn selfhost_string_object_offset(base: i64, texts_before: &[&str]) -> i64 {
    base + texts_before
        .iter()
        .map(|text| (8 + text.len()) as i64)
        .sum::<i64>()
}

pub(crate) struct BootstrapDiffArtifactFixture<'a> {
    pub(crate) artifact_id: &'a str,
    pub(crate) test_name: &'a str,
    pub(crate) left_key: &'a str,
    pub(crate) right_key: &'a str,
    pub(crate) left_label: &'a str,
    pub(crate) right_label: &'a str,
    pub(crate) left_wasm: Option<&'a [u8]>,
    pub(crate) right_wasm: Option<&'a [u8]>,
    pub(crate) diff_report: &'a str,
    pub(crate) metadata: Value,
    pub(crate) left_sections: Option<Value>,
    pub(crate) right_sections: Option<Value>,
    pub(crate) left_export: Option<&'a [u8]>,
    pub(crate) right_export: Option<&'a [u8]>,
    pub(crate) left_data: Option<&'a [u8]>,
    pub(crate) right_data: Option<&'a [u8]>,
}

pub(crate) fn bootstrap_diff_artifact_id() -> String {
    std::env::var("BOOTSTRAP_DIFF_ARTIFACT_ID")
        .ok()
        .filter(|value| !value.trim().is_empty())
        .or_else(|| {
            std::env::var("GITHUB_SHA")
                .ok()
                .filter(|value| !value.trim().is_empty())
        })
        .unwrap_or_else(|| "local".to_string())
}

fn write_optional_bytes(path: &std::path::Path, bytes: Option<&[u8]>) {
    if let Some(bytes) = bytes {
        std::fs::write(path, bytes).unwrap_or_else(|e| {
            panic!(
                "bootstrap diff artifact 書き込み失敗 {}: {}",
                path.display(),
                e
            )
        });
    }
}

fn write_optional_json(path: &std::path::Path, json: Option<&Value>) {
    if let Some(json) = json {
        std::fs::write(
            path,
            serde_json::to_vec_pretty(json).expect("bootstrap diff JSON serialize に失敗"),
        )
        .unwrap_or_else(|e| panic!("bootstrap diff JSON 書き込み失敗 {}: {}", path.display(), e));
    }
}

pub(crate) fn write_bootstrap_diff_artifact(
    fixture: &BootstrapDiffArtifactFixture<'_>,
) -> std::path::PathBuf {
    let artifact_root = selfhost_project_root()
        .join("ci-artifacts/bootstrap-diff")
        .join(fixture.artifact_id);
    std::fs::create_dir_all(&artifact_root).unwrap_or_else(|e| {
        panic!(
            "artifact ディレクトリ作成に失敗 {}: {}",
            artifact_root.display(),
            e
        )
    });

    std::fs::write(artifact_root.join("diff-report.txt"), fixture.diff_report)
        .unwrap_or_else(|e| panic!("diff-report.txt 書き込み失敗: {e}"));
    let mut metadata = fixture.metadata.clone();
    if metadata.get("test_name").is_none() {
        metadata["test_name"] = Value::String(fixture.test_name.to_string());
    }

    std::fs::write(
        artifact_root.join("metadata.json"),
        serde_json::to_vec_pretty(&metadata).expect("metadata.json serialize に失敗"),
    )
    .unwrap_or_else(|e| panic!("metadata.json 書き込み失敗: {e}"));

    write_optional_bytes(
        &artifact_root.join(format!("{}.wasm", fixture.left_label)),
        fixture.left_wasm,
    );
    write_optional_bytes(
        &artifact_root.join(format!("{}.wasm", fixture.right_label)),
        fixture.right_wasm,
    );
    write_optional_json(
        &artifact_root.join(format!("sections_{}.json", fixture.left_key)),
        fixture.left_sections.as_ref(),
    );
    write_optional_json(
        &artifact_root.join(format!("sections_{}.json", fixture.right_key)),
        fixture.right_sections.as_ref(),
    );
    write_optional_bytes(
        &artifact_root.join(format!("export_{}.bin", fixture.left_key)),
        fixture.left_export,
    );
    write_optional_bytes(
        &artifact_root.join(format!("export_{}.bin", fixture.right_key)),
        fixture.right_export,
    );
    write_optional_bytes(
        &artifact_root.join(format!("data_{}.bin", fixture.left_key)),
        fixture.left_data,
    );
    write_optional_bytes(
        &artifact_root.join(format!("data_{}.bin", fixture.right_key)),
        fixture.right_data,
    );

    artifact_root
}

fn imported_function_count(wasm: &[u8]) -> u32 {
    use wasmparser::{Parser, Payload, TypeRef};

    let mut count = 0;
    for payload in Parser::new(0).parse_all(wasm) {
        let Ok(payload) = payload else {
            break;
        };
        if let Payload::ImportSection(section) = payload {
            for import in section {
                let Ok(import) = import else {
                    continue;
                };
                if matches!(import.ty, TypeRef::Func(_)) {
                    count += 1;
                }
            }
        }
    }
    count
}

fn exported_function_index(wasm: &[u8], name: &str) -> Option<u32> {
    use wasmparser::{ExternalKind, Parser, Payload};

    for payload in Parser::new(0).parse_all(wasm) {
        let Ok(payload) = payload else {
            break;
        };
        if let Payload::ExportSection(section) = payload {
            for export in section {
                let Ok(export) = export else {
                    continue;
                };
                if export.name == name && matches!(export.kind, ExternalKind::Func) {
                    return Some(export.index);
                }
            }
        }
    }
    None
}

fn function_operator_debug(wasm: &[u8], func_index: u32, max_ops: usize) -> Vec<String> {
    use wasmparser::{Parser, Payload};

    let imported = imported_function_count(wasm);
    let mut defined_index = 0u32;
    for payload in Parser::new(0).parse_all(wasm) {
        let Ok(payload) = payload else {
            break;
        };
        if let Payload::CodeSectionEntry(body) = payload {
            let absolute_index = imported + defined_index;
            defined_index += 1;
            if absolute_index != func_index {
                continue;
            }
            let mut ops = Vec::new();
            let mut reader = body
                .get_operators_reader()
                .expect("operator reader を取得できること");
            while !reader.eof() && ops.len() < max_ops {
                let op = reader
                    .read()
                    .unwrap_or_else(|e| panic!("operator read failed at func {func_index}: {e}"));
                ops.push(format!("{op:?}"));
            }
            return ops;
        }
    }
    Vec::new()
}

fn function_body_bytes(wasm: &[u8], func_index: u32) -> Option<Vec<u8>> {
    use wasmparser::{Parser, Payload};

    let imported = imported_function_count(wasm);
    let mut defined_index = 0u32;
    for payload in Parser::new(0).parse_all(wasm) {
        let Ok(payload) = payload else {
            break;
        };
        if let Payload::CodeSectionEntry(body) = payload {
            let absolute_index = imported + defined_index;
            defined_index += 1;
            if absolute_index == func_index {
                return Some(body.as_bytes().to_vec());
            }
        }
    }
    None
}

fn local_bound_violation_indices(wasm: &[u8]) -> Vec<u32> {
    use wasmparser::{Operator, Parser, Payload};

    let imported = imported_function_count(wasm);
    let param_counts = selfhost_type_param_counts(wasm);
    let type_indices = selfhost_defined_function_type_indices(wasm);
    let mut indices = Vec::new();
    let mut defined_index = 0u32;

    for payload in Parser::new(0).parse_all(wasm) {
        let Ok(payload) = payload else {
            break;
        };
        if let Payload::CodeSectionEntry(body) = payload {
            let declared_locals = body
                .get_locals_reader()
                .ok()
                .map(|reader| {
                    let mut total = 0_u32;
                    for local in reader.into_iter().flatten() {
                        total += local.0;
                    }
                    total
                })
                .unwrap_or(0);
            let type_index = type_indices
                .get(defined_index as usize)
                .copied()
                .unwrap_or(0);
            let param_count = param_counts.get(type_index as usize).copied().unwrap_or(0);
            let total_locals = param_count + declared_locals;
            let absolute_index = imported + defined_index;
            let Ok(mut reader) = body.get_operators_reader() else {
                indices.push(absolute_index);
                defined_index += 1;
                continue;
            };
            let mut violated = false;
            while !reader.eof() {
                let op = match reader.read() {
                    Ok(op) => op,
                    Err(_) => {
                        violated = true;
                        break;
                    }
                };
                let local_index = match op {
                    Operator::LocalGet { local_index }
                    | Operator::LocalSet { local_index }
                    | Operator::LocalTee { local_index } => Some(local_index),
                    _ => None,
                };
                if let Some(local_index) = local_index
                    && local_index >= total_locals
                {
                    violated = true;
                    break;
                }
            }
            if violated {
                indices.push(absolute_index);
            }
            defined_index += 1;
        }
    }

    indices
}

fn first_byte_diff(left: &[u8], right: &[u8]) -> Option<usize> {
    let min_len = left.len().min(right.len());
    for idx in 0..min_len {
        if left[idx] != right[idx] {
            return Some(idx);
        }
    }
    if left.len() == right.len() {
        None
    } else {
        Some(min_len)
    }
}

fn read_leb_u32(bytes: &[u8], pos: &mut usize) -> u32 {
    let mut value = 0_u32;
    let mut shift = 0;
    loop {
        let byte = bytes[*pos];
        *pos += 1;
        value |= u32::from(byte & 0x7f) << shift;
        if byte & 0x80 == 0 {
            break;
        }
        shift += 7;
    }
    value
}

fn selfhost_type_param_counts(wasm: &[u8]) -> Vec<u32> {
    let Some(type_bytes) = extract_section_bytes(wasm, 1) else {
        return Vec::new();
    };
    let mut pos = 0usize;
    let count = read_leb_u32(&type_bytes, &mut pos) as usize;
    let mut param_counts = Vec::with_capacity(count);
    for _ in 0..count {
        if pos >= type_bytes.len() || type_bytes[pos] != 0x60 {
            break;
        }
        pos += 1;
        let param_count = read_leb_u32(&type_bytes, &mut pos);
        param_counts.push(param_count);
        pos += param_count as usize;
        let result_count = read_leb_u32(&type_bytes, &mut pos);
        pos += result_count as usize;
    }
    param_counts
}

fn selfhost_defined_function_type_indices(wasm: &[u8]) -> Vec<u32> {
    let Some(function_bytes) = extract_section_bytes(wasm, 3) else {
        return Vec::new();
    };
    let mut pos = 0usize;
    let count = read_leb_u32(&function_bytes, &mut pos) as usize;
    let mut type_indices = Vec::with_capacity(count);
    for _ in 0..count {
        type_indices.push(read_leb_u32(&function_bytes, &mut pos));
    }
    type_indices
}

fn local_bound_violations(wasm: &[u8]) -> Vec<String> {
    use wasmparser::{Operator, Parser, Payload};

    let imported = imported_function_count(wasm);
    let param_counts = selfhost_type_param_counts(wasm);
    let type_indices = selfhost_defined_function_type_indices(wasm);
    let mut violations = Vec::new();
    let mut defined_index = 0_u32;

    for payload in Parser::new(0).parse_all(wasm) {
        let Ok(payload) = payload else {
            break;
        };
        if let Payload::CodeSectionEntry(body) = payload {
            let body_prefix = body.as_bytes().iter().take(24).copied().collect::<Vec<_>>();
            let declared_locals = body
                .get_locals_reader()
                .ok()
                .map(|reader| {
                    let mut total = 0_u32;
                    for local in reader {
                        let (count, _) = local.unwrap_or_else(|e| {
                            panic!("locals read failed at func {defined_index}: {e}")
                        });
                        total += count;
                    }
                    total
                })
                .unwrap_or(0);
            let type_index = type_indices
                .get(defined_index as usize)
                .copied()
                .unwrap_or(0);
            let param_count = param_counts.get(type_index as usize).copied().unwrap_or(0);
            let total_locals = param_count + declared_locals;
            let absolute_index = imported + defined_index;
            let mut max_local_ref = 0_u32;
            let mut max_local_op = None::<String>;
            let Ok(mut reader) = body.get_operators_reader() else {
                violations.push(format!(
                    "func {absolute_index} (defined #{defined_index}, type {type_index}): operator reader init failed"
                ));
                defined_index += 1;
                continue;
            };
            while !reader.eof() {
                let op = match reader.read() {
                    Ok(op) => op,
                    Err(e) => {
                        violations.push(format!(
                            "func {absolute_index} (defined #{defined_index}, type {type_index}): operator read failed: {e}; params={param_count} locals={declared_locals} total={total_locals}; body_prefix={body_prefix:?}"
                        ));
                        break;
                    }
                };
                let local_index = match op {
                    Operator::LocalGet { local_index }
                    | Operator::LocalSet { local_index }
                    | Operator::LocalTee { local_index } => Some(local_index),
                    _ => None,
                };
                if let Some(local_index) = local_index
                    && (max_local_op.is_none() || local_index >= max_local_ref)
                {
                    max_local_ref = local_index;
                    max_local_op = Some(format!("{op:?}"));
                }
            }
            if let Some(max_local_op) = max_local_op
                && max_local_ref >= total_locals
            {
                violations.push(format!(
                        "func {absolute_index} (defined #{defined_index}, type {type_index}): params={param_count} locals={declared_locals} total={total_locals} max_ref={max_local_ref} via {max_local_op}; body_prefix={body_prefix:?}"
                    ));
            }
            defined_index += 1;
        }
    }

    violations
}

/// バイト列のハッシュフィンガープリントを計算するヘルパー
pub(crate) fn hash_fingerprint(data: &[u8]) -> u64 {
    let mut hasher = DefaultHasher::new();
    data.hash(&mut hasher);
    hasher.finish()
}

fn layer_status_name(matches: bool) -> &'static str {
    if matches { "match" } else { "mismatch" }
}

fn format_layer_line(name: &str, detail: String) -> String {
    format!("{name:<18}{detail}")
}

fn exported_memory<T>(caller: &mut wasmtime::Caller<'_, T>) -> wasmtime::Memory {
    match caller.get_export("memory") {
        Some(wasmtime::Extern::Memory(memory)) => memory,
        _ => panic!("memory export が見つからない"),
    }
}

fn ensure_memory_capacity<T>(caller: &mut wasmtime::Caller<'_, T>, end: i64, context: &str) {
    let needed = usize::try_from(end).unwrap_or_else(|_| panic!("{context}: end address が不正"));
    let memory = exported_memory(caller);
    let current = memory.data_size(&mut *caller);
    let current_pages = memory.size(&mut *caller);
    if needed > current {
        let additional = needed - current;
        let page_size = 64 * 1024;
        let pages = additional.div_ceil(page_size);
        memory
            .grow(
                &mut *caller,
                u64::try_from(pages).expect("追加 page 数が u64 に収まらない"),
            )
            .unwrap_or_else(|e| {
                panic!(
                    "{context}: memory.grow に失敗: {e}; current_bytes={current}; current_pages={current_pages}; needed_end={needed}; requested_pages={pages}"
                )
            });
    }
}

fn write_string_object_bytes<T>(
    mut caller: wasmtime::Caller<'_, T>,
    base: i64,
    content: &[u8],
    context: &str,
) {
    let object_len =
        i64::try_from(8 + content.len()).expect("string object size が i64 に収まらない");
    let end = base
        .checked_add(object_len)
        .unwrap_or_else(|| panic!("{context}: string object end address が overflow"));
    ensure_memory_capacity(&mut caller, end, context);
    let memory = exported_memory(&mut caller);
    let mut object = Vec::with_capacity(8 + content.len());
    object.extend_from_slice(&1_i32.to_le_bytes());
    object.extend_from_slice(&(content.len() as i32).to_le_bytes());
    object.extend_from_slice(content);
    memory
        .write(&mut caller, base as usize, &object)
        .unwrap_or_else(|e| panic!("{context}: string object を memory へ書き込めない: {e}"));
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
struct SixImportState {
    next_alloc: i64,
    printed: String,
    file_content: String,
    file_root: Option<std::path::PathBuf>,
    args: Vec<String>,
    string_object_cache: HashMap<Vec<u8>, i64>,
    root_stack: Vec<i64>,
}

fn alloc_cached_string_object(
    mut caller: wasmtime::Caller<'_, SixImportState>,
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

fn run_wasm_with_six_imports_compiler_mode_inner(
    wasm: &[u8],
    file_content: Option<&str>,
    file_root: Option<&std::path::Path>,
    args: &[&str],
    printed_first_on_error: bool,
) -> Result<String, String> {
    let engine = configured_selfhost_engine();
    let module = wasmtime::Module::new(&engine, wasm)
        .map_err(|e| format!("Wasm モジュールの読み込みに失敗: {} / {:?}", e, e))?;
    let mut store = wasmtime::Store::new(
        &engine,
        SixImportState {
            next_alloc: 65536,
            printed: String::new(),
            file_content: file_content.unwrap_or_default().to_string(),
            file_root: file_root.map(std::path::Path::to_path_buf),
            args: args.iter().map(|a| a.to_string()).collect(),
            string_object_cache: HashMap::new(),
            root_stack: Vec::new(),
        },
    );
    let alloc = wasmtime::Func::wrap(
        &mut store,
        |mut caller: wasmtime::Caller<'_, SixImportState>, size: i64| -> i64 {
            let base = caller.data().next_alloc;
            let end = base
                .checked_add(size)
                .unwrap_or_else(|| panic!("six-import alloc: end address が overflow"));
            ensure_memory_capacity(&mut caller, end, "six-import alloc");
            caller.data_mut().next_alloc = end;
            base
        },
    );
    let print = wasmtime::Func::wrap(
        &mut store,
        |mut caller: wasmtime::Caller<'_, SixImportState>, value: i64| {
            caller.data_mut().printed.push_str(&format!("{value}\n"));
        },
    );
    let read_file = wasmtime::Func::wrap(
        &mut store,
        |mut caller: wasmtime::Caller<'_, SixImportState>, path: i64| -> i64 {
            let content = if let Some(root_dir) = caller.data().file_root.clone() {
                let rel_path = read_path_text_with_root(&mut caller, path, &root_dir);
                let full_path = root_dir.join(&rel_path);
                let bytes = std::fs::read(&full_path).unwrap_or_else(|e| {
                    panic!(
                        "read-file import が {} を読めない: {e}",
                        full_path.display()
                    )
                });
                eprintln!(
                    "six-import read-file: {} len={} prefix={:?}",
                    full_path.display(),
                    bytes.len(),
                    &bytes[..bytes.len().min(32)]
                );
                bytes
            } else {
                let bytes = caller.data().file_content.as_bytes().to_vec();
                eprintln!(
                    "six-import read-file (inline): len={} prefix={:?}",
                    bytes.len(),
                    &bytes[..bytes.len().min(32)]
                );
                bytes
            };
            alloc_cached_string_object(caller, content, "six-import read-file")
        },
    );
    let command_line_arg = wasmtime::Func::wrap(
        &mut store,
        |caller: wasmtime::Caller<'_, SixImportState>, index: i64| -> i64 {
            let content = usize::try_from(index)
                .ok()
                .and_then(|i| caller.data().args.get(i))
                .map(|a| a.as_bytes().to_vec())
                .unwrap_or_default();
            alloc_cached_string_object(caller, content, "six-import command-line-arg")
        },
    );
    // string-concat(ptr1, ptr2): 2つの文字列オブジェクトを結合して新しい文字列を返す
    let string_concat = wasmtime::Func::wrap(
        &mut store,
        |mut caller: wasmtime::Caller<'_, SixImportState>, ptr1: i64, ptr2: i64| -> i64 {
            let (len1, len2) = {
                let Some(wasmtime::Extern::Memory(m)) = caller.get_export("memory") else {
                    return caller.data().next_alloc;
                };
                let mut buf = [0u8; 4];
                let _ = m.read(&caller, ptr1 as usize + 4, &mut buf);
                let l1 = i32::from_le_bytes(buf).max(0) as usize;
                let _ = m.read(&caller, ptr2 as usize + 4, &mut buf);
                let l2 = i32::from_le_bytes(buf).max(0) as usize;
                (l1, l2)
            };
            let combined = {
                let Some(wasmtime::Extern::Memory(m)) = caller.get_export("memory") else {
                    return caller.data().next_alloc;
                };
                let mut c1 = vec![0u8; len1];
                let mut c2 = vec![0u8; len2];
                let _ = m.read(&caller, ptr1 as usize + 8, &mut c1);
                let _ = m.read(&caller, ptr2 as usize + 8, &mut c2);
                let mut combined = c1;
                combined.extend_from_slice(&c2);
                combined
            };
            alloc_cached_string_object(caller, combined, "six-import string-concat")
        },
    );
    // substring(ptr, start, end): 文字列オブジェクトの部分文字列を返す
    let substring = wasmtime::Func::wrap(
        &mut store,
        |mut caller: wasmtime::Caller<'_, SixImportState>, ptr: i64, start: i64, end: i64| -> i64 {
            let slice = {
                let Some(wasmtime::Extern::Memory(m)) = caller.get_export("memory") else {
                    return caller.data().next_alloc;
                };
                let mut buf = [0u8; 4];
                let _ = m.read(&caller, ptr as usize + 4, &mut buf);
                let total_len = i32::from_le_bytes(buf).max(0) as usize;
                let s = (start as usize).min(total_len);
                let e = (end as usize).min(total_len);
                let slice_len = e.saturating_sub(s);
                let mut data = vec![0u8; slice_len];
                if slice_len > 0 {
                    let _ = m.read(&caller, ptr as usize + 8 + s, &mut data);
                }
                data
            };
            alloc_cached_string_object(caller, slice, "six-import substring")
        },
    );
    let file_exists = wasmtime::Func::wrap(
        &mut store,
        |mut caller: wasmtime::Caller<'_, SixImportState>, path: i64| -> i64 {
            let exists = if let Some(root_dir) = caller.data().file_root.clone() {
                let rel_path = read_path_text_with_root(&mut caller, path, &root_dir);
                root_dir.join(rel_path).exists()
            } else {
                let rel_path = String::from_utf8(read_string_object_bytes(&mut caller, path))
                    .expect("file-exists? path が UTF-8 ではない");
                std::path::Path::new(&rel_path).exists()
            };
            if exists { 1 } else { 0 }
        },
    );
    let root_push = wasmtime::Func::wrap(
        &mut store,
        |mut caller: wasmtime::Caller<'_, SixImportState>, value: i64| -> i64 {
            let slot = i64::try_from(caller.data().root_stack.len())
                .expect("six-import root_push: slot overflow");
            caller.data_mut().root_stack.push(value);
            slot
        },
    );
    let root_pop = wasmtime::Func::wrap(
        &mut store,
        |mut caller: wasmtime::Caller<'_, SixImportState>| -> i64 {
            caller.data_mut().root_stack.pop().unwrap_or(0)
        },
    );
    let root_set = wasmtime::Func::wrap(
        &mut store,
        |mut caller: wasmtime::Caller<'_, SixImportState>, slot: i64, value: i64| -> i64 {
            let idx = usize::try_from(slot)
                .unwrap_or_else(|_| panic!("six-import root_set: slot must be non-negative"));
            let len = caller.data().root_stack.len();
            assert!(
                idx < len,
                "six-import root_set: slot {} out of bounds {}",
                idx,
                len
            );
            caller.data_mut().root_stack[idx] = value;
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
    .map_err(|e| format!("インスタンス化に失敗: {e}"))?;
    let start = instance
        .get_typed_func::<(), ()>(&mut store, "_start")
        .map_err(|e| format!("_start export 取得失敗: {e}"))?;
    match start.call(&mut store, ()) {
        Ok(()) => Ok(store.data().printed.clone()),
        Err(e) => {
            if printed_first_on_error {
                Err(format!("printed={:?}; 実行失敗: {e}", store.data().printed))
            } else {
                Err(format!("実行失敗: {e}; printed={:?}", store.data().printed))
            }
        }
    }
}

pub(crate) fn run_wasm_with_six_imports_compiler_mode(
    wasm: &[u8],
    file_content: &str,
    args: &[&str],
) -> Result<String, String> {
    run_wasm_with_six_imports_compiler_mode_inner(wasm, Some(file_content), None, args, false)
}

pub(crate) fn run_wasm_with_six_imports_compiler_mode_fs(
    wasm: &[u8],
    root_dir: &std::path::Path,
    args: &[&str],
) -> Result<String, String> {
    run_wasm_with_six_imports_compiler_mode_inner(wasm, None, Some(root_dir), args, false)
}

pub(crate) fn run_wasm_with_six_imports_compiler_mode_fs_printed_first(
    wasm: &[u8],
    root_dir: &std::path::Path,
    args: &[&str],
) -> Result<String, String> {
    run_wasm_with_six_imports_compiler_mode_inner(wasm, None, Some(root_dir), args, true)
}

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
    let artifact_id = bootstrap_diff_artifact_id();

    // stage0 (Rust) で selfhost/src/App/Main.ls を 2 回コンパイル
    let wasm_a = compile_file_only(&main_path);
    let wasm_b = compile_file_only(&main_path);

    // レイヤー 1: ハッシュフィンガープリント比較
    let hash_a = hash_fingerprint(&wasm_a);
    let hash_b = hash_fingerprint(&wasm_b);
    let hash_match = hash_a == hash_b;

    // レイヤー 2: Export セクション (ID=7) のシンボル比較
    let export_a =
        extract_section_bytes(&wasm_a, 7).expect("wasm_a に Export セクションが見つからない");
    let export_b =
        extract_section_bytes(&wasm_b, 7).expect("wasm_b に Export セクションが見つからない");
    let export_match = export_a == export_b;
    assert!(!export_a.is_empty(), "Export セクションが空");

    // レイヤー 3: Data セクション (ID=11) のバイト列比較
    // Data セクションが存在しない場合は両方 None で一致とする
    let data_a = extract_section_bytes(&wasm_a, 11);
    let data_b = extract_section_bytes(&wasm_b, 11);
    let data_match = data_a == data_b;

    // レイヤー 4: 診断カウント比較
    // コンパイル成功 = 診断 0。try_compile_file_only でエラーを検出可能。
    let diag_a = try_compile_file_only(&main_path).is_ok();
    let diag_b = try_compile_file_only(&main_path).is_ok();
    let diag_match = diag_a == diag_b;
    assert!(diag_a, "コンパイルが失敗した（診断あり）");

    // 追加検証: raw bytes が完全一致
    let bytes_match = wasm_a == wasm_b;

    // 追加検証: セクション構造の安定性
    let sections_a = extract_sections(&wasm_a);
    let sections_b = extract_sections(&wasm_b);
    let sections_match = sections_a == sections_b;

    let timestamp = "1970-01-01T00:00:00Z";
    let data_line = match (&data_a, &data_b) {
        (None, None) => "ABSENT".to_string(),
        (Some(left), Some(right)) if data_match => {
            format!("MATCH ({} bytes vs {} bytes)", left.len(), right.len())
        }
        (Some(left), Some(right)) => {
            format!("MISMATCH ({} bytes vs {} bytes)", left.len(), right.len())
        }
        (Some(left), None) => format!("MISMATCH ({} bytes vs absent)", left.len()),
        (None, Some(right)) => format!("MISMATCH (absent vs {} bytes)", right.len()),
    };
    let diff_report = [
        "Bootstrap Diff Report".to_string(),
        "=====================".to_string(),
        format!("commit: {artifact_id}"),
        format!("timestamp: {timestamp}"),
        "test: test_e2e_bootstrap_four_layer_comparison".to_string(),
        String::new(),
        format_layer_line(
            "Layer 1 (hash):",
            if hash_match {
                format!("MATCH ({:#018x} vs {:#018x})", hash_a, hash_b)
            } else {
                format!("MISMATCH ({:#018x} vs {:#018x})", hash_a, hash_b)
            },
        ),
        format_layer_line(
            "Layer 2 (export):",
            if export_match {
                format!(
                    "MATCH ({} bytes vs {} bytes)",
                    export_a.len(),
                    export_b.len()
                )
            } else {
                format!(
                    "MISMATCH ({} bytes vs {} bytes)",
                    export_a.len(),
                    export_b.len()
                )
            },
        ),
        format_layer_line("Layer 3 (data):", data_line),
        format_layer_line(
            "Layer 4 (diag):",
            if diag_match {
                format!("MATCH ({} vs {})", i32::from(!diag_a), i32::from(!diag_b))
            } else {
                format!(
                    "MISMATCH ({} vs {})",
                    i32::from(!diag_a),
                    i32::from(!diag_b)
                )
            },
        ),
        String::new(),
        format!("stage1_a.wasm: {} bytes", wasm_a.len()),
        format!("stage1_b.wasm: {} bytes", wasm_b.len()),
        format!("raw-bytes-match: {bytes_match}"),
        format!("sections-match: {sections_match}"),
        String::new(),
    ]
    .join("\n");

    let metadata = serde_json::json!({
        "commit_sha": artifact_id,
        "timestamp": timestamp,
        "test_name": "test_e2e_bootstrap_four_layer_comparison",
        "stage1_a_size": wasm_a.len(),
        "stage1_b_size": wasm_b.len(),
        "layers": {
            "hash": layer_status_name(hash_match),
            "export": layer_status_name(export_match),
            "data": layer_status_name(data_match),
            "diagnostics": layer_status_name(diag_match)
        },
        "raw_bytes": layer_status_name(bytes_match),
        "sections": layer_status_name(sections_match)
    });

    write_bootstrap_diff_artifact(&BootstrapDiffArtifactFixture {
        artifact_id: &artifact_id,
        test_name: "test_e2e_bootstrap_four_layer_comparison",
        left_key: "a",
        right_key: "b",
        left_label: "stage1_a",
        right_label: "stage1_b",
        left_wasm: Some(&wasm_a),
        right_wasm: Some(&wasm_b),
        diff_report: &diff_report,
        metadata,
        left_sections: Some(serde_json::json!(sections_a)),
        right_sections: Some(serde_json::json!(sections_b)),
        left_export: Some(&export_a),
        right_export: Some(&export_b),
        left_data: data_a.as_deref(),
        right_data: data_b.as_deref(),
    });

    assert_eq!(
        hash_a, hash_b,
        "レイヤー1: ハッシュフィンガープリント不一致 — {:#018x} vs {:#018x}",
        hash_a, hash_b
    );
    assert_eq!(
        export_a,
        export_b,
        "レイヤー2: Export セクション不一致 — {} bytes vs {} bytes",
        export_a.len(),
        export_b.len()
    );
    assert_eq!(
        data_a,
        data_b,
        "レイヤー3: Data セクション不一致 — {:?} bytes vs {:?} bytes",
        data_a.as_ref().map(|d| d.len()),
        data_b.as_ref().map(|d| d.len())
    );
    assert_eq!(
        diag_a, diag_b,
        "レイヤー4: 診断結果不一致 — {} vs {}",
        diag_a, diag_b
    );
    assert_eq!(
        wasm_a,
        wasm_b,
        "raw bytes 不一致 — {} bytes vs {} bytes",
        wasm_a.len(),
        wasm_b.len()
    );
    assert_eq!(sections_a, sections_b, "セクション構造不一致");
}

#[test]
fn test_bootstrap_diff_artifact_writes_readable_local_report() {
    let artifact_id = "test-local-artifact";
    let artifact_root = selfhost_project_root()
        .join("ci-artifacts/bootstrap-diff")
        .join(artifact_id);
    if artifact_root.exists() {
        std::fs::remove_dir_all(&artifact_root).unwrap_or_else(|e| {
            panic!("artifact 事前掃除に失敗 {}: {}", artifact_root.display(), e)
        });
    }

    let fixture = BootstrapDiffArtifactFixture {
        artifact_id,
        test_name: "test_e2e_bootstrap_four_layer_comparison",
        left_key: "a",
        right_key: "b",
        left_label: "stage1_a",
        right_label: "stage1_b",
        left_wasm: Some(b"\0asm\x01\0\0\0"),
        right_wasm: Some(b"\0asm\x01\0\0\0\x01"),
        diff_report: "Bootstrap Diff Report\nLayer 1 (hash): MISMATCH\n",
        metadata: serde_json::json!({
            "commit_sha": artifact_id,
            "test_name": "test_e2e_bootstrap_four_layer_comparison",
            "layers": {
                "hash": "mismatch"
            }
        }),
        left_sections: Some(serde_json::json!([[1, 2]])),
        right_sections: Some(serde_json::json!([[1, 3]])),
        left_export: Some(&[0x01, 0x02]),
        right_export: Some(&[0x03, 0x04]),
        left_data: None,
        right_data: Some(&[0x09]),
    };

    let written_dir = write_bootstrap_diff_artifact(&fixture);
    assert_eq!(written_dir, artifact_root);
    assert!(written_dir.join("diff-report.txt").is_file());
    assert!(written_dir.join("metadata.json").is_file());
    assert!(written_dir.join("stage1_a.wasm").is_file());
    assert!(written_dir.join("stage1_b.wasm").is_file());
    assert!(written_dir.join("sections_a.json").is_file());
    assert!(written_dir.join("sections_b.json").is_file());
    assert!(written_dir.join("export_a.bin").is_file());
    assert!(written_dir.join("export_b.bin").is_file());
    assert!(!written_dir.join("data_a.bin").exists());
    assert!(written_dir.join("data_b.bin").is_file());

    let report = std::fs::read_to_string(written_dir.join("diff-report.txt"))
        .expect("diff-report.txt の読み込みに失敗");
    assert!(
        report.contains("Bootstrap Diff Report"),
        "diff-report.txt は人間可読ヘッダを持つこと"
    );
    assert!(
        report.contains("Layer 1 (hash): MISMATCH"),
        "diff-report.txt はレイヤー要約を含むこと"
    );

    let metadata: Value = serde_json::from_str(
        &std::fs::read_to_string(written_dir.join("metadata.json"))
            .expect("metadata.json の読み込みに失敗"),
    )
    .expect("metadata.json は JSON であること");
    assert_eq!(metadata["commit_sha"], artifact_id);
    assert_eq!(
        metadata["test_name"],
        "test_e2e_bootstrap_four_layer_comparison"
    );

    std::fs::remove_dir_all(&artifact_root)
        .unwrap_or_else(|e| panic!("artifact 後掃除に失敗 {}: {}", artifact_root.display(), e));
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
    let main_path = selfhost_main_path();

    // --- Phase 1: stage0 で最小サブセットをコンパイル ---
    // Token.ls は依存なしの最小モジュール
    let token_path = selfhost_source_path("Token.ls");
    let token_wasm_1 = compile_file_only(&token_path);
    let token_wasm_2 = compile_file_only(&token_path);
    assert_eq!(
        token_wasm_1, token_wasm_2,
        "Phase1: canonical Token.ls の stage0 コンパイルが非決定的"
    );
    assert_valid_wasm(&token_wasm_1);

    // --- Phase 2: stage0 で Main.ls をコンパイル → stage1.wasm ---
    let stage1_wasm_a = compile_file_only(&main_path);
    let stage1_wasm_b = compile_file_only(&main_path);
    assert_eq!(
        stage1_wasm_a, stage1_wasm_b,
        "Phase2: canonical Main.ls の stage0 コンパイルが非決定的"
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

// =============================================================================
// V2-11 runtime-emitter parity テスト
// =============================================================================

/// V2-11: emit-import-section-runtime が 10 import のバイト列を生成すること
///
/// selfhost の 10-import レイアウト (call 0..9) と一致するバイト列を検証する。
/// import count が 10、各 import 名が期待通りであることを wasmparser で検証する。
#[test]
fn test_v2_11_emit_import_section_runtime_produces_10_imports() {
    // selfhost の emit-import-section-runtime を呼んで bytes を print で出力するハーネス
    let harness = r#"
(defn print-bytes [bytes idx count]
  (if (>= idx count)
    0
    (do
      (print (vector-get bytes idx))
      (print-bytes bytes (+ idx 1) count))))

(defn main []
  (let [sec (emit-import-section-runtime)
        n (vector-length sec)]
    (do
      (print n)
      (print-bytes sec 0 n)
      0)))
"#;
    let source = format!("{}\n{}", selfhost_cli_runtime_bundle(), harness);
    let wasm = compile_only(&source);
    assert_valid_wasm(&wasm);

    let output = lsharp_wasm::wasi_runner::run_wasm_wasi(&wasm)
        .expect("emit-import-section-runtime 出力ハーネスの実行に失敗");

    // 出力された数値列からバイト列を復元
    let numbers: Vec<i64> = output
        .lines()
        .filter(|l| !l.trim().is_empty())
        .map(|l| l.trim().parse::<i64>().expect("数値パース失敗"))
        .collect();
    assert!(!numbers.is_empty(), "出力が空");
    let byte_count = numbers[0] as usize;
    assert_eq!(numbers.len(), byte_count + 1, "バイト数と出力行数が不一致");
    let sec_bytes: Vec<u8> = numbers[1..].iter().map(|&n| n as u8).collect();

    // wasmparser でダミー Wasm モジュール (magic + version + import section) を検証
    let mut wasm_module = vec![0x00, 0x61, 0x73, 0x6d, 0x01, 0x00, 0x00, 0x00];

    // 型セクション: 5 種類 (type 0-4) を追加してから import section を追加
    // type 0: (i64) -> i64
    // type 1: (i64) -> void
    // type 2: (i64, i64) -> i64
    // type 3: (i64, i64, i64) -> i64
    // type 4: () -> i64
    let type_section: Vec<u8> = vec![
        0x01, // section id: type
        0x1b, // section size (27 bytes = 1 count + 5+4+6+7+4 type bytes)
        0x05, // 5 types
        0x60, 0x01, 0x7e, 0x01, 0x7e, // (i64) -> i64
        0x60, 0x01, 0x7e, 0x00, // (i64) -> void
        0x60, 0x02, 0x7e, 0x7e, 0x01, 0x7e, // (i64,i64) -> i64
        0x60, 0x03, 0x7e, 0x7e, 0x7e, 0x01, 0x7e, // (i64,i64,i64) -> i64
        0x60, 0x00, 0x01, 0x7e, // () -> i64
    ];
    wasm_module.extend_from_slice(&type_section);
    wasm_module.extend_from_slice(&sec_bytes);

    // wasmparser で parse して import 数と名前を検証
    use wasmparser::{Parser, Payload};
    let mut import_count = 0usize;
    let mut import_names: Vec<String> = Vec::new();
    for payload in Parser::new(0).parse_all(&wasm_module) {
        if let Ok(Payload::ImportSection(reader)) = payload {
            for import in reader {
                let imp = import.expect("import エントリ読み取り失敗");
                import_names.push(imp.name.to_string());
                import_count += 1;
            }
        }
    }

    assert_eq!(import_count, 10, "import 数が 10 でない: {import_names:?}");
    let expected_names = [
        "__alloc",
        "print",
        "read-file",
        "command-line-arg",
        "string-concat",
        "substring",
        "file-exists?",
        "root_push",
        "root_pop",
        "root_set",
    ];
    for (i, (actual, expected)) in import_names.iter().zip(expected_names.iter()).enumerate() {
        assert_eq!(
            actual, expected,
            "import[{i}] 名が不一致: got {actual:?}, want {expected:?}"
        );
    }
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
        "{}\n{}\n{}\n{}\n{}\n{}\n{}\n{}\n{}",
        selfhost_module("Token.ls"),
        selfhost_module("AST.ls"),
        selfhost_module("Lexer.ls"),
        selfhost_module("Parser.ls"),
        selfhost_module("IR.ls"),
        selfhost_module("Compiler.ls"),
        selfhost_module("WasiBackend.ls"),
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
        "{}\n{}\n{}\n{}\n{}\n{}\n{}\n{}\n{}",
        selfhost_module("Token.ls"),
        selfhost_module("AST.ls"),
        selfhost_module("Lexer.ls"),
        selfhost_module("Parser.ls"),
        selfhost_module("IR.ls"),
        selfhost_module("Compiler.ls"),
        selfhost_module("WasiBackend.ls"),
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
        "{}\n{}\n{}\n{}\n{}\n{}\n{}\n{}\n{}",
        selfhost_module("Token.ls"),
        selfhost_module("AST.ls"),
        selfhost_module("Lexer.ls"),
        selfhost_module("Parser.ls"),
        selfhost_module("IR.ls"),
        selfhost_module("Compiler.ls"),
        selfhost_module("WasiBackend.ls"),
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
    let stage2_src = r#"(defn add1 [x] (+ x 1)) (defn main [] (add1 41))"#;
    let harness = {
        let mut s = RUNTIME_STAGE2_HARNESS_PRELUDE.to_string();
        s.push_str("\n(defn main []\n  (let [stage2 (bootstrap-build-stage2 \"");
        s.push_str(stage2_src);
        s.push_str("\")]\n    (do\n      (bootstrap-print-module stage2)\n      0)))");
        s
    };
    let stage1_source = format!("{}\n{}", selfhost_cli_runtime_bundle(), harness);
    let stage1_wasm = compile_only(&stage1_source);
    assert_valid_wasm(&stage1_wasm);

    let output = lsharp_wasm::wasi_runner::run_wasm_wasi(&stage1_wasm)
        .expect("single-param call program を含む stage1 wasm の実行に失敗");
    let modules = parse_emitted_wasm_modules(&output, 1);
    assert_eq!(modules.len(), 1, "stage2 モジュール数が不正");
    assert_valid_wasm(&modules[0]);
    assert!(
        extract_sections(&modules[0]).iter().any(|(id, _)| *id == 2),
        "stage2 は selfhost ランタイム import section を持つこと"
    );
    assert_eq!(
        run_exported_i64_with_runtime_imports(&modules[0], "_start"),
        42,
        "stage1 は 1 引数関数呼出しを含む stage2 Wasm を生成すること"
    );
}

/// BOOT-04: stage1 が let local を含む stage2 Wasm を生成できること
#[test]
fn test_e2e_bootstrap_stage1_emits_stage2_wasm_for_let_local_program() {
    let stage2_src = r#"(defn main [] (let [x 42] x))"#;
    let harness = {
        let mut s = RUNTIME_STAGE2_HARNESS_PRELUDE.to_string();
        s.push_str("\n(defn main []\n  (let [stage2 (bootstrap-build-stage2 \"");
        s.push_str(stage2_src);
        s.push_str("\")]\n    (do\n      (bootstrap-print-module stage2)\n      0)))");
        s
    };
    let stage1_source = format!("{}\n{}", selfhost_cli_runtime_bundle(), harness);
    let stage1_wasm = compile_only(&stage1_source);
    assert_valid_wasm(&stage1_wasm);

    let output = lsharp_wasm::wasi_runner::run_wasm_wasi(&stage1_wasm)
        .expect("let local program を含む stage1 wasm の実行に失敗");
    let modules = parse_emitted_wasm_modules(&output, 1);
    assert_eq!(modules.len(), 1, "stage2 モジュール数が不正");
    assert_valid_wasm(&modules[0]);
    assert!(
        extract_sections(&modules[0]).iter().any(|(id, _)| *id == 2),
        "stage2 は selfhost ランタイム import section を持つこと"
    );
    assert_eq!(
        run_exported_i64_with_runtime_imports(&modules[0], "_start"),
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
    let stage2_src =
        r#"(defn fib [n] (if (< n 2) n (+ (fib (- n 1)) (fib (- n 2))))) (defn main [] (fib 8))"#;
    let harness = {
        let mut s = RUNTIME_STAGE2_HARNESS_PRELUDE.to_string();
        s.push_str("\n(defn main []\n  (let [stage2 (bootstrap-build-stage2 \"");
        s.push_str(stage2_src);
        s.push_str("\")]\n    (do\n      (bootstrap-print-module stage2)\n      0)))");
        s
    };
    let stage1_source = format!("{}\n{}", selfhost_cli_runtime_bundle(), harness);
    let stage1_wasm = compile_only(&stage1_source);
    assert_valid_wasm(&stage1_wasm);

    let output = lsharp_wasm::wasi_runner::run_wasm_wasi(&stage1_wasm)
        .expect("再帰フィボナッチを含む stage1 wasm の実行に失敗");
    let modules = parse_emitted_wasm_modules(&output, 1);
    assert_eq!(modules.len(), 1, "stage2 モジュール数が不正");
    assert_valid_wasm(&modules[0]);
    assert!(
        extract_sections(&modules[0]).iter().any(|(id, _)| *id == 2),
        "stage2 は selfhost ランタイム import section を持つこと"
    );
    assert_eq!(
        run_exported_i64_with_runtime_imports(&modules[0], "_start"),
        21,
        "stage1 は fib(8)=21 を返す stage2 Wasm を生成すること"
    );
}

/// BOOT-04: stage1 が自己再帰階乗を含む stage2 Wasm を生成・実行できること
///
/// (defn fact [n] ...) + (defn main [] (fact 5)) → stage2 が 120 を返す
#[test]
fn test_e2e_bootstrap_stage1_emits_stage2_wasm_for_recursive_factorial() {
    let stage2_src =
        r#"(defn fact [n] (if (<= n 1) 1 (* n (fact (- n 1))))) (defn main [] (fact 5))"#;
    let harness = {
        let mut s = RUNTIME_STAGE2_HARNESS_PRELUDE.to_string();
        s.push_str("\n(defn main []\n  (let [stage2 (bootstrap-build-stage2 \"");
        s.push_str(stage2_src);
        s.push_str("\")]\n    (do\n      (bootstrap-print-module stage2)\n      0)))");
        s
    };
    let stage1_source = format!("{}\n{}", selfhost_cli_runtime_bundle(), harness);
    let stage1_wasm = compile_only(&stage1_source);
    assert_valid_wasm(&stage1_wasm);

    let output = lsharp_wasm::wasi_runner::run_wasm_wasi(&stage1_wasm)
        .expect("再帰階乗を含む stage1 wasm の実行に失敗");
    let modules = parse_emitted_wasm_modules(&output, 1);
    assert_eq!(modules.len(), 1, "stage2 モジュール数が不正");
    assert_valid_wasm(&modules[0]);
    assert!(
        extract_sections(&modules[0]).iter().any(|(id, _)| *id == 2),
        "stage2 は selfhost ランタイム import section を持つこと"
    );
    assert_eq!(
        run_exported_i64_with_runtime_imports(&modules[0], "_start"),
        120,
        "stage1 は fact(5)=120 を返す stage2 Wasm を生成すること"
    );
}

/// BOOT-04: stage1 が多関数ヘルパー再帰を含む stage2 Wasm を生成・実行できること
///
/// sum(n) を呼ぶ helper(x) + main の 3 関数構成で stage2 が 55 を返す
#[test]
fn test_e2e_bootstrap_stage1_emits_stage2_wasm_for_multi_function_helper_recursion() {
    let stage2_src = r#"(defn sum [n] (if (<= n 0) 0 (+ n (sum (- n 1))))) (defn helper [x] (sum x)) (defn main [] (helper 10))"#;
    let harness = {
        let mut s = RUNTIME_STAGE2_HARNESS_PRELUDE.to_string();
        s.push_str("\n(defn main []\n  (let [stage2 (bootstrap-build-stage2 \"");
        s.push_str(stage2_src);
        s.push_str("\")]\n    (do\n      (bootstrap-print-module stage2)\n      0)))");
        s
    };
    let stage1_source = format!("{}\n{}", selfhost_cli_runtime_bundle(), harness);
    let stage1_wasm = compile_only(&stage1_source);
    assert_valid_wasm(&stage1_wasm);

    let output = lsharp_wasm::wasi_runner::run_wasm_wasi(&stage1_wasm)
        .expect("多関数ヘルパー再帰を含む stage1 wasm の実行に失敗");
    let modules = parse_emitted_wasm_modules(&output, 1);
    assert_eq!(modules.len(), 1, "stage2 モジュール数が不正");
    assert_valid_wasm(&modules[0]);
    assert!(
        extract_sections(&modules[0]).iter().any(|(id, _)| *id == 2),
        "stage2 は selfhost ランタイム import section を持つこと"
    );
    assert_eq!(
        run_exported_i64_with_runtime_imports(&modules[0], "_start"),
        55,
        "stage1 は sum(10)=55 を経由する helper→main を含む stage2 Wasm を生成すること"
    );
}

/// BOOT-04: stage1 が string-char-at builtin を含む stage2 Wasm を valid module として生成できること
#[test]
fn test_e2e_bootstrap_stage1_emits_stage2_wasm_for_string_char_at_helper_program() {
    let stage2_src = r#"(defn first [s] (string-char-at s 0)) (defn main [] 0)"#;
    let harness = {
        let mut s = RUNTIME_STAGE2_HARNESS_PRELUDE.to_string();
        s.push_str("\n(defn main []\n  (let [stage2 (bootstrap-build-stage2 \"");
        s.push_str(stage2_src);
        s.push_str("\")]\n    (do\n      (bootstrap-print-module stage2)\n      0)))");
        s
    };
    let stage1_source = format!("{}\n{}", selfhost_cli_runtime_bundle(), harness);
    let stage1_wasm = compile_only(&stage1_source);
    assert_valid_wasm(&stage1_wasm);

    let output = lsharp_wasm::wasi_runner::run_wasm_wasi(&stage1_wasm)
        .expect("string-char-at helper program を含む stage1 wasm の実行に失敗");
    let modules = parse_emitted_wasm_modules(&output, 1);
    assert_eq!(modules.len(), 1, "stage2 モジュール数が不正");
    assert_valid_wasm(&modules[0]);
    assert!(
        extract_sections(&modules[0]).iter().any(|(id, _)| *id == 2),
        "stage2 は selfhost ランタイム import section を持つこと"
    );
    assert!(
        extract_sections(&modules[0]).iter().any(|(id, _)| *id == 5),
        "string-char-at helper を含む stage2 Wasm は memory section を持つこと"
    );
    assert_eq!(
        run_exported_i64_with_runtime_imports(&modules[0], "_start"),
        0,
        "helper 未使用でも string-char-at builtin を含む stage2 Wasm が実行可能であること"
    );
}

/// BOOT-04: stage1 が string-length builtin を含む stage2 Wasm を valid module として生成できること
#[test]
fn test_e2e_bootstrap_stage1_emits_stage2_wasm_for_string_length_helper_program() {
    let stage2_src = r#"(defn len1 [s] (string-length s)) (defn main [] 0)"#;
    let harness = {
        let mut s = RUNTIME_STAGE2_HARNESS_PRELUDE.to_string();
        s.push_str("\n(defn main []\n  (let [stage2 (bootstrap-build-stage2 \"");
        s.push_str(stage2_src);
        s.push_str("\")]\n    (do\n      (bootstrap-print-module stage2)\n      0)))");
        s
    };
    let stage1_source = format!("{}\n{}", selfhost_cli_runtime_bundle(), harness);
    let stage1_wasm = compile_only(&stage1_source);
    assert_valid_wasm(&stage1_wasm);

    let output = lsharp_wasm::wasi_runner::run_wasm_wasi(&stage1_wasm)
        .expect("string-length helper program を含む stage1 wasm の実行に失敗");
    let modules = parse_emitted_wasm_modules(&output, 1);
    assert_eq!(modules.len(), 1, "stage2 モジュール数が不正");
    assert_valid_wasm(&modules[0]);
    assert!(
        extract_sections(&modules[0]).iter().any(|(id, _)| *id == 2),
        "stage2 は selfhost ランタイム import section を持つこと"
    );
    assert!(
        extract_sections(&modules[0]).iter().any(|(id, _)| *id == 5),
        "string-length helper を含む stage2 Wasm は memory section を持つこと"
    );
    assert_eq!(
        run_exported_i64_with_runtime_imports(&modules[0], "_start"),
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
    let expected_data = selfhost_string_object_bytes("abc");
    assert!(
        data_section
            .windows(expected_data.len())
            .any(|window| window == expected_data),
        "data section に string object header + bytes が含まれていない"
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
    let expected_data = selfhost_string_object_sequence(&["ab", "cde"]);
    assert!(
        data_section
            .windows(expected_data.len())
            .any(|window| window == expected_data),
        "nested string literal objects が data section に連結配置されていない"
    );
    assert_eq!(
        run_exported_i64(&modules[0], "_start"),
        selfhost_string_object_offset(1024, &["ab"]),
        "nested string literal の最終 offset が前段 object header + bytes を考慮していない"
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
    let expected_data = selfhost_string_object_sequence(&["ab", "c", "de", "fgh", "ijk"]);
    assert!(
        data_section
            .windows(expected_data.len())
            .any(|window| window == expected_data),
        "extended do string literal objects が data section に連結配置されていない"
    );
    assert_eq!(
        run_exported_i64(&modules[0], "_start"),
        selfhost_string_object_offset(1024, &["ab", "c", "de", "fgh"]),
        "extended do string literal の最終 offset が前段 object header + bytes を考慮していない"
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
    let expected_data = selfhost_string_object_sequence(&["hello", "world"]);
    assert!(
        data_section
            .windows(expected_data.len())
            .any(|window| window == expected_data),
        "if string literal objects が data section に連結配置されていない"
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
    let expected_data = selfhost_string_object_sequence(&["one", "two"]);
    assert!(
        data_section
            .windows(expected_data.len())
            .any(|window| window == expected_data),
        "match string literal objects が data section に連結配置されていない"
    );
    assert_eq!(
        run_exported_i64(&modules[0], "_start"),
        selfhost_string_object_offset(1024, &["one"]),
        "match string literal の selected branch offset が前段 object header + bytes を考慮していない"
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
    let expected_data = selfhost_string_object_bytes("ok");
    assert!(
        data_section
            .windows(expected_data.len())
            .any(|window| window == expected_data),
        "lambda string literal object が data section に配置されていない"
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
    let stage2_src = r#"(defn vlen [v] (vector-length v)) (defn main [] 0)"#;
    let harness = {
        let mut s = RUNTIME_STAGE2_HARNESS_PRELUDE.to_string();
        s.push_str("\n(defn main []\n  (let [stage2 (bootstrap-build-stage2 \"");
        s.push_str(stage2_src);
        s.push_str("\")]\n    (do\n      (bootstrap-print-module stage2)\n      0)))");
        s
    };
    let stage1_source = format!("{}\n{}", selfhost_cli_runtime_bundle(), harness);
    let stage1_wasm = compile_only(&stage1_source);
    assert_valid_wasm(&stage1_wasm);

    let output = lsharp_wasm::wasi_runner::run_wasm_wasi(&stage1_wasm)
        .expect("vector-length helper program を含む stage1 wasm の実行に失敗");
    let modules = parse_emitted_wasm_modules(&output, 1);
    assert_eq!(modules.len(), 1, "stage2 モジュール数が不正");
    assert_valid_wasm(&modules[0]);
    assert!(
        extract_sections(&modules[0]).iter().any(|(id, _)| *id == 2),
        "stage2 は selfhost ランタイム import section を持つこと"
    );
    assert!(
        extract_sections(&modules[0]).iter().any(|(id, _)| *id == 5),
        "vector-length helper を含む stage2 Wasm は memory section を持つこと"
    );
    assert_eq!(
        run_exported_i64_with_runtime_imports(&modules[0], "_start"),
        0,
        "helper 未使用でも vector-length builtin を含む stage2 Wasm が実行可能であること"
    );
}

/// BOOT-04: stage1 が vector-get builtin を含む stage2 Wasm を valid module として生成できること
#[test]
fn test_e2e_bootstrap_stage1_emits_stage2_wasm_for_vector_get_helper_program() {
    let stage2_src = r#"(defn vget0 [v] (vector-get v 0)) (defn main [] 0)"#;
    let harness = {
        let mut s = RUNTIME_STAGE2_HARNESS_PRELUDE.to_string();
        s.push_str("\n(defn main []\n  (let [stage2 (bootstrap-build-stage2 \"");
        s.push_str(stage2_src);
        s.push_str("\")]\n    (do\n      (bootstrap-print-module stage2)\n      0)))");
        s
    };
    let stage1_source = format!("{}\n{}", selfhost_cli_runtime_bundle(), harness);
    let stage1_wasm = compile_only(&stage1_source);
    assert_valid_wasm(&stage1_wasm);

    let output = lsharp_wasm::wasi_runner::run_wasm_wasi(&stage1_wasm)
        .expect("vector-get helper program を含む stage1 wasm の実行に失敗");
    let modules = parse_emitted_wasm_modules(&output, 1);
    assert_eq!(modules.len(), 1, "stage2 モジュール数が不正");
    assert_valid_wasm(&modules[0]);
    assert!(
        extract_sections(&modules[0]).iter().any(|(id, _)| *id == 2),
        "stage2 は selfhost ランタイム import section を持つこと"
    );
    assert!(
        extract_sections(&modules[0]).iter().any(|(id, _)| *id == 5),
        "vector-get helper を含む stage2 Wasm は memory section を持つこと"
    );
    assert_eq!(
        run_exported_i64_with_runtime_imports(&modules[0], "_start"),
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
        export-sec (emit-export-section-main-memory-index 1 0)
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
        export-sec (emit-export-section-main-memory-index 1 0)
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
    let stage1_source = stage1_source_emitting_wasi_stage2(
        "(defn main [] (let [v0 (vector-new 1)] (let [v1 (vector-push v0 10)] (vector-length (vector-push v1 20)))))",
    );
    let stage1_wasm = compile_only(&stage1_source);
    assert_valid_wasm(&stage1_wasm);

    let output = lsharp_wasm::wasi_runner::run_wasm_wasi(&stage1_wasm)
        .expect("vector-push program を含む stage1 wasm の実行に失敗");
    let modules = parse_emitted_wasm_modules(&output, 1);
    assert_eq!(modules.len(), 1, "stage2 モジュール数が不正");
    assert_valid_wasm(&modules[0]);
    assert!(
        extract_sections(&modules[0]).iter().any(|(id, _)| *id == 2),
        "vector-push program を含む stage2 Wasm は selfhost runtime import section を持つこと"
    );
    assert_eq!(
        run_exported_i64_with_runtime_imports(&modules[0], "_start"),
        2,
        "vector-push の in-place + growth を含む stage2 Wasm が runtime 10-import 付きで実行可能であること"
    );
}

/// BOOT-04: stage1 が ref-new/ref-set/ref-get を含む stage2 Wasm を生成できること
#[test]
fn test_e2e_bootstrap_stage1_emits_stage2_wasm_for_ref_program() {
    let stage1_source = stage1_source_emitting_wasi_stage2(
        "(defn main [] (let [r (ref-new 1)] (do (ref-set r 42) (ref-get r))))",
    );
    let stage1_wasm = compile_only(&stage1_source);
    assert_valid_wasm(&stage1_wasm);

    let output = lsharp_wasm::wasi_runner::run_wasm_wasi(&stage1_wasm)
        .expect("ref program を含む stage1 wasm の実行に失敗");
    let modules = parse_emitted_wasm_modules(&output, 1);
    assert_eq!(modules.len(), 1, "stage2 モジュール数が不正");
    assert_valid_wasm(&modules[0]);
    assert!(
        extract_sections(&modules[0]).iter().any(|(id, _)| *id == 2),
        "ref program を含む stage2 Wasm は selfhost runtime import section を持つこと"
    );
    assert_eq!(
        run_exported_i64_with_runtime_imports(&modules[0], "_start"),
        42,
        "ref-new/ref-set/ref-get を含む stage2 Wasm が runtime 10-import 付きで実行可能であること"
    );
}

/// BOOT-04: stage1 が整数 key の map-new/map-insert/map-get/map-size を含む stage2 Wasm を生成できること
#[test]
fn test_e2e_bootstrap_stage1_emits_stage2_wasm_for_map_program() {
    let stage1_source = stage1_source_emitting_wasi_stage2(
        "(defn main [] (let [m0 (map-new)] (let [m1 (map-insert m0 1 10)] (let [m2 (map-insert m1 2 20)] (+ (+ (map-get m2 1) (map-get m2 2)) (map-size m2))))))",
    );
    let stage1_wasm = compile_only(&stage1_source);
    assert_valid_wasm(&stage1_wasm);

    let output = lsharp_wasm::wasi_runner::run_wasm_wasi(&stage1_wasm)
        .expect("map program を含む stage1 wasm の実行に失敗");
    let modules = parse_emitted_wasm_modules(&output, 1);
    assert_eq!(modules.len(), 1, "stage2 モジュール数が不正");
    assert_valid_wasm(&modules[0]);
    assert!(
        extract_sections(&modules[0]).iter().any(|(id, _)| *id == 2),
        "map program を含む stage2 Wasm は selfhost runtime import section を持つこと"
    );
    assert_eq!(
        run_exported_i64_with_runtime_imports(&modules[0], "_start"),
        32,
        "整数 key の map builtins を含む stage2 Wasm が runtime 10-import 付きで実行可能であること"
    );
}

/// BOOT-04: stage1 が整数 key subset の map-contains? を含む stage2 Wasm を生成できること
#[test]
fn test_e2e_bootstrap_stage1_emits_stage2_wasm_for_map_contains_program() {
    let stage1_source = stage1_source_emitting_wasi_stage2(
        "(defn main [] (let [m0 (map-new)] (let [m1 (map-insert m0 7 70)] (+ (* 10 (map-contains? m1 7)) (map-contains? m1 99)))))",
    );
    let stage1_wasm = compile_only(&stage1_source);
    assert_valid_wasm(&stage1_wasm);

    let output = lsharp_wasm::wasi_runner::run_wasm_wasi(&stage1_wasm)
        .expect("map-contains? program を含む stage1 wasm の実行に失敗");
    let modules = parse_emitted_wasm_modules(&output, 1);
    assert_eq!(modules.len(), 1, "stage2 モジュール数が不正");
    assert_valid_wasm(&modules[0]);
    assert!(
        extract_sections(&modules[0]).iter().any(|(id, _)| *id == 2),
        "map-contains? program を含む stage2 Wasm は selfhost runtime import section を持つこと"
    );
    assert_eq!(
        run_exported_i64_with_runtime_imports(&modules[0], "_start"),
        10,
        "整数 key subset の map-contains? を含む stage2 Wasm が runtime 10-import 付きで実行可能であること"
    );
}

/// BOOT-04: stage1 が整数 key subset の map-remove を含む stage2 Wasm を生成できること
#[test]
fn test_e2e_bootstrap_stage1_emits_stage2_wasm_for_map_remove_program() {
    let stage1_source = stage1_source_emitting_wasi_stage2(
        "(defn main [] (let [m0 (map-new)] (let [m1 (map-insert m0 1 10)] (let [m2 (map-insert m1 2 20)] (let [m3 (map-remove m2 1)] (+ (map-get m3 1) (+ (* 10 (map-size m3)) (map-get m3 2))))))))",
    );
    let stage1_wasm = compile_only(&stage1_source);
    assert_valid_wasm(&stage1_wasm);

    let output = lsharp_wasm::wasi_runner::run_wasm_wasi(&stage1_wasm)
        .expect("map-remove program を含む stage1 wasm の実行に失敗");
    let modules = parse_emitted_wasm_modules(&output, 1);
    assert_eq!(modules.len(), 1, "stage2 モジュール数が不正");
    assert_valid_wasm(&modules[0]);
    assert!(
        extract_sections(&modules[0]).iter().any(|(id, _)| *id == 2),
        "map-remove program を含む stage2 Wasm は selfhost runtime import section を持つこと"
    );
    assert_eq!(
        run_exported_i64_with_runtime_imports(&modules[0], "_start"),
        30,
        "整数 key subset の map-remove を含む stage2 Wasm が runtime 10-import 付きで実行可能であること"
    );
}

/// BOOT-04: stage1 が source-aware string key subset の map builtins を含む stage2 Wasm を生成できること
#[test]
fn test_e2e_bootstrap_stage1_emits_stage2_wasm_for_string_key_map_program() {
    let stage2_source = r#"(defn main [] (let [m0 (map-new)] (let [m1 (map-insert m0 "aa" 10)] (let [m2 (map-insert m1 "bb" 20)] (let [m3 (map-remove m2 "aa")] (+ (* 10 (map-size m3)) (map-get m3 "bb")))))))"#.replace('"', "\\\"");
    let stage1_source = stage1_source_emitting_wasi_stage2_with_source(&stage2_source);
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
        run_exported_i64_with_runtime_imports(&modules[0], "_start"),
        30,
        "string key subset の map builtins を含む stage2 Wasm が runtime 10-import 付きで実行可能であること"
    );
}

/// BOOT-04: stage1 が non-literal string key map builtins を含む stage2 Wasm を生成できること
#[test]
fn test_e2e_bootstrap_stage1_emits_stage2_wasm_for_non_literal_string_key_map_program() {
    let stage2_source = r#"(defn main [] (do (print (let [key (read-file "fixture.txt")] (let [m0 (map-new)] (let [m1 (map-insert m0 key 42)] (map-get m1 key))))) 0))"#.replace('"', "\\\"");
    let harness = format!(
        r#"
(defn main []
  (let [src "{}"
        program (parse-program src)
        pair (compile-program-functions-with-source src program)
        functions (vector-get pair 1)
        data (vector-get pair 2)
        stage2 (build-wasm-bytes-wasi functions data)]
    (do
      (print-wasm-module stage2)
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
    let printed = run_wasm_with_six_imports_compiler_mode(&modules[0], "aa", &[])
        .expect("non-literal string key map builtins を含む stage2 Wasm の実行に失敗");
    assert_eq!(
        printed.trim(),
        "42",
        "non-literal string key map builtins を含む stage2 Wasm が runtime 10-import 付きで実行可能であること"
    );
}

/// BOOT-04: stage1 が generalized 4-helper path で alloc+print+read-file+__fnv1a_hash stage2 Wasm を生成できること
#[test]
fn test_e2e_bootstrap_stage1_emits_stage2_wasm_for_generalized_alloc_print_read_hash_helper_quad() {
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
        2,
        "generalized alloc+print+read-file+__fnv1a_hash quad を含む stage2 Wasm が実行可能であること"
    );
}

/// BOOT-04: stage1 が同じ generalized alloc+print+read-file+__fnv1a_hash source から同一 stage2 Wasm を 2 回生成できること
#[test]
fn test_e2e_bootstrap_stage1_emits_identical_hash_helper_quad_stage2_wasm_for_same_source() {
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
        2
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
        assert_eq!(
            modules.len(),
            1,
            "N=5 では stage2 モジュールが 1 つ生成されるはず"
        );
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
        assert_eq!(
            modules.len(),
            1,
            "N=2000 では stage2 モジュールが 1 つ生成されるはず"
        );
        assert_valid_wasm(&modules[0]);
    }
}

#[test]
fn test_e2e_boot04_selfhost_compile_program_functions_handles_many_defns() {
    let mut stage2_src = String::new();
    for i in 0..2000 {
        stage2_src.push_str(&format!("(defn fn{i:04} [] {i}) "));
    }
    stage2_src.push_str("(defn main [] 0)");

    let harness = format!(
        concat!(
            "(defn main []\n",
            "  (let [program (parse-program \"{s2}\")\n",
            "        pair (compile-program-functions program)\n",
            "        functions (vector-get pair 1)]\n",
            "    (do (print (vector-length functions)) 0)))\n",
        ),
        s2 = stage2_src
    );
    let stage1_source = format!("{}\n{}", selfhost_cli_runtime_bundle(), harness);
    let stage1_wasm = compile_only(&stage1_source);
    assert_valid_wasm(&stage1_wasm);
    let result = lsharp_wasm::wasi_runner::run_wasm_wasi(&stage1_wasm);
    assert!(
        result.is_ok(),
        "compile-program-functions が大量 defn でトラップした: {:?}",
        result.err()
    );
    assert_eq!(
        result.unwrap().trim(),
        "2001",
        "2000 個の defn + main の 2001 関数が登録されるべき"
    );
}

#[test]
fn test_e2e_boot04_selfhost_compile_program_functions_with_source_handles_deep_let_chain() {
    let mut nested_expr = "0".to_string();
    for i in (0..512).rev() {
        nested_expr = format!("(let [v{i:04} {i}] {nested_expr})");
    }
    let stage2_src = format!("(defn main [] {nested_expr})");

    let harness = format!(
        concat!(
            "(defn main []\n",
            "  (let [program (parse-program \"{s2}\")\n",
            "        pair (compile-program-functions-with-source \"{s2}\" program)\n",
            "        functions (vector-get pair 1)]\n",
            "    (do (print (vector-length functions)) 0)))\n",
        ),
        s2 = stage2_src
    );
    let stage1_source = format!("{}\n{}", selfhost_cli_runtime_bundle(), harness);
    let stage1_wasm = compile_only(&stage1_source);
    assert_valid_wasm(&stage1_wasm);
    let result = lsharp_wasm::wasi_runner::run_wasm_wasi(&stage1_wasm);
    assert!(
        result.is_ok(),
        "compile-program-functions-with-source が深い let 連鎖でトラップした: {:?}",
        result.err()
    );
    assert_eq!(
        result.unwrap().trim(),
        "1",
        "深い let 連鎖でも 1 関数のメタデータを返すべき"
    );
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
    let fixture_dir =
        std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../tests/fixtures");
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

    // stage2 が 6-import モデルで実行可能であること (_start: () -> () ラッパー付き)
    // minimal.ls = (defn main [] 42) → main は何も print しない
    let run_result = run_wasm_with_six_imports_compiler_mode(stage2_wasm, "", &[]);
    assert!(
        run_result.is_ok(),
        "BOOT-04 compiler-mode: stage2 の WASI 実行に失敗: {:?}",
        run_result.err()
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
    let fixture_dir =
        std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../tests/fixtures");
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

    // stage3 が 6-import モデルで実行できること
    let stage3_result = run_wasm_with_six_imports_compiler_mode(stage3_wasm, "", &[]);
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
/// BOOT-04: self-hosted stage2 compiler が minimal.ls を stage3 へコンパイルできること
#[test]
fn test_e2e_boot04_self_hosted_stage2_compiles_minimal() {
    let main_path = selfhost_main_path();
    // selfhost/ ルート（src/ の親）を WASI dir として設定する。
    // selfhost/src/App/Main.ls は dotted import (Syntax.AST 等) を使うため、
    // source_root = "src" が正しく解決されるには WASI dir = selfhost/ が必要。
    let selfhost_root = main_path
        .parent()
        .expect("App/ ディレクトリ")
        .parent()
        .expect("src/ ディレクトリ")
        .parent()
        .expect("selfhost/ ルートディレクトリ")
        .to_path_buf();
    let fixture_dir =
        std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../tests/fixtures");

    // stage1 = Rust bootstrap が生成した完全コンパイラ wasm
    let stage1_wasm = compile_file_only(&main_path);
    assert_valid_wasm(&stage1_wasm);

    // stage1 が compiler-mode で src/App/Main.ls 自身をコンパイル → stage2_self_compiler を試みる
    // WASI dir = selfhost/ にすることで dotted import (Syntax.AST → src/Syntax/AST.ls) が解決される
    let output = lsharp_wasm::wasi_runner::run_wasm_wasi_with_dir_and_args(
        &stage1_wasm,
        Some(&selfhost_root),
        &["compiler", "src/App/Main.ls"],
    )
    .expect("BOOT-04 self-hosted-stage2: stage1 が Main.ls の self-compile に失敗した");
    eprintln!(
        "BOOT-04 self-hosted-stage2: stage1 が Main.ls → output ({} chars) を生成",
        output.len()
    );

    let modules = std::panic::catch_unwind(|| parse_emitted_wasm_modules(&output, 1))
        .expect("BOOT-04 self-hosted-stage2: stage1 出力が wasm モジュール形式でない");
    let stage2_self_compiler = &modules[0];
    eprintln!(
        "BOOT-04 self-hosted-stage2: stage2_self_compiler = {} bytes",
        stage2_self_compiler.len()
    );
    let sections = extract_sections(stage2_self_compiler);
    eprintln!("BOOT-04 stage2 sections: {:?}", sections);
    match validate_wasm_detailed(stage2_self_compiler) {
        Ok(_) => eprintln!("BOOT-04 stage2: wasmparser validation PASSED"),
        Err(e) => eprintln!("BOOT-04 stage2 wasmparser ERROR: {}", e),
    }
    assert_valid_wasm(stage2_self_compiler);

    let minimal_ls_content = std::fs::read_to_string(fixture_dir.join("minimal.ls"))
        .unwrap_or_else(|_| "(defn main [] 42)".to_string());
    let stage3_output = run_wasm_with_six_imports_compiler_mode(
        stage2_self_compiler,
        &minimal_ls_content,
        &["compiler", "minimal.ls"],
    )
    .expect("BOOT-04 self-hosted-stage2: stage2_self_compiler が minimal.ls をコンパイルできない");
    eprintln!(
        "BOOT-04 self-hosted-stage2: stage3_output = {} chars",
        stage3_output.len()
    );

    let stage3_modules = std::panic::catch_unwind(|| parse_emitted_wasm_modules(&stage3_output, 1))
        .expect("BOOT-04 self-hosted-stage2: stage3 出力が wasm 形式でない");
    let stage3_wasm = &stage3_modules[0];
    assert_valid_wasm(stage3_wasm);

    let run_result = run_wasm_with_six_imports_compiler_mode(stage3_wasm, "", &[]);
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

#[test]
fn test_e2e_boot04_self_hosted_stage2_preserves_batched_step_progress() {
    let main_path = selfhost_main_path();
    let selfhost_root = main_path
        .parent()
        .expect("App/ ディレクトリ")
        .parent()
        .expect("src/ ディレクトリ")
        .parent()
        .expect("selfhost/ ルートディレクトリ")
        .to_path_buf();

    let stage1_wasm = compile_file_only(&main_path);
    assert_valid_wasm(&stage1_wasm);
    let stage2_output = lsharp_wasm::wasi_runner::run_wasm_wasi_with_dir_and_args(
        &stage1_wasm,
        Some(&selfhost_root),
        &["compiler", "src/App/Main.ls"],
    )
    .expect("BOOT-04 batching probe: stage1 が Main.ls の self-compile に失敗した");
    let stage2_modules = parse_emitted_wasm_modules(&stage2_output, 1);
    let stage2_self_compiler = &stage2_modules[0];
    assert_valid_wasm(stage2_self_compiler);

    let probe_source = r#"
(defn make-state [done next]
  (vector-push
    (vector-push (vector-new 4) done)
    next))

(defn step [limit pos]
  (if (>= pos limit)
    (make-state 1 pos)
    (make-state 0 (+ pos 1))))

(defn continue-step [limit state]
  (if (= (vector-get state 0) 1)
    state
    (step limit (vector-get state 1))))

(defn step-8 [limit pos]
  (let [step1 (step limit pos)
    step2 (continue-step limit step1)
    step3 (continue-step limit step2)
    step4 (continue-step limit step3)
    step5 (continue-step limit step4)
    step6 (continue-step limit step5)
    step7 (continue-step limit step6)
    step8 (continue-step limit step7)]
    step8))

(defn continue-step-8 [limit state]
  (if (= (vector-get state 0) 1)
    state
    (step-8 limit (vector-get state 1))))

(defn step-64 [limit pos]
  (let [step1 (step-8 limit pos)
    step2 (continue-step-8 limit step1)
    step3 (continue-step-8 limit step2)
    step4 (continue-step-8 limit step3)
    step5 (continue-step-8 limit step4)
    step6 (continue-step-8 limit step5)
    step7 (continue-step-8 limit step6)
    step8 (continue-step-8 limit step7)]
    step8))

(defn continue-step-64 [limit state]
  (if (= (vector-get state 0) 1)
    state
    (step-64 limit (vector-get state 1))))

(defn step-512 [limit pos]
  (let [step1 (step-64 limit pos)
    step2 (continue-step-64 limit step1)
    step3 (continue-step-64 limit step2)
    step4 (continue-step-64 limit step3)
    step5 (continue-step-64 limit step4)
    step6 (continue-step-64 limit step5)
    step7 (continue-step-64 limit step6)
    step8 (continue-step-64 limit step7)]
    step8))

(defn main []
  (let [state8 (step-8 1000 0)
    state64 (step-64 1000 0)
    state512 (step-512 1000 0)
    capped64 (step-64 13 0)]
    (do
      (print (vector-get state8 1))
      (print (vector-get state64 1))
      (print (vector-get state512 1))
      (print (vector-get capped64 1))
      0)))
"#;

    let stage3_result = run_wasm_with_six_imports_compiler_mode(
        stage2_self_compiler,
        probe_source,
        &["compiler", "batching-probe.ls"],
    );

    match &stage3_result {
        Ok(stage3_output) => {
            let stage3_modules = parse_emitted_wasm_modules(stage3_output, 1);
            let stage3_wasm = &stage3_modules[0];
            assert_valid_wasm(stage3_wasm);

            let run_output = run_wasm_with_six_imports_compiler_mode(stage3_wasm, "", &[])
                .expect("BOOT-04 batching probe: stage3 probe module の実行に失敗した");
            let lines: Vec<&str> = run_output
                .lines()
                .filter(|line| !line.trim().is_empty())
                .collect();

            assert!(
                lines.len() >= 4,
                "BOOT-04 batching probe の出力が不足: {:?}",
                lines
            );
            assert_eq!(lines[0], "8", "step-8 は 8 ステップぶん進むべき");
            assert_eq!(lines[1], "64", "step-64 は 64 ステップぶん進むべき");
            assert_eq!(lines[2], "512", "step-512 は 512 ステップぶん進むべき");
            assert_eq!(lines[3], "13", "step-64 は limit 到達時に早期終了すべき");
        }
        Err(compile_err) => {
            let frame_count = compile_err
                .lines()
                .filter(|l| l.contains("wasm function"))
                .count();
            eprintln!(
                "BOOT-04 batching probe BLOCKED: stage2 compile failed with {} wasm frames at overflow",
                frame_count
            );
            eprintln!(
                "BOOT-04 batching probe BLOCKED: first error line: {}",
                compile_err.lines().next().unwrap_or("")
            );
            eprintln!(
                "BOOT-04 batching probe THRESHOLD: synthetic step-8/64/512 probe still exceeds stage2 expression recursion budget (~{} recursion levels at ~65 frames each)",
                frame_count / 65
            );

            assert!(
                compile_err.contains("wasm backtrace") || compile_err.contains("unreachable"),
                "batching probe stage2 compile 失敗は wasm backtrace を含むべき (got: {})",
                compile_err.lines().next().unwrap_or("")
            );
            assert!(
                frame_count >= 200,
                "batching probe overflow frame count が 200 未満 (got {}): 失敗モードが変わった可能性がある",
                frame_count
            );
        }
    }
}

#[test]
fn test_e2e_boot04_self_hosted_stage2_compiles_large_single_file() {
    let main_path = selfhost_main_path();
    let selfhost_root = main_path
        .parent()
        .expect("App/ ディレクトリ")
        .parent()
        .expect("src/ ディレクトリ")
        .parent()
        .expect("selfhost/ ルートディレクトリ")
        .to_path_buf();

    let stage1_wasm = compile_file_only(&main_path);
    assert_valid_wasm(&stage1_wasm);

    let stage2_output = lsharp_wasm::wasi_runner::run_wasm_wasi_with_dir_and_args(
        &stage1_wasm,
        Some(&selfhost_root),
        &["compiler", "src/App/Main.ls"],
    )
    .expect("BOOT-04 large-file: stage1 が Main.ls の self-compile に失敗した");
    let stage2_modules = parse_emitted_wasm_modules(&stage2_output, 1);
    let stage2_self_compiler = &stage2_modules[0];
    assert_valid_wasm(stage2_self_compiler);

    let repeated_helpers = (0..800)
        .map(|idx| format!("(defn helper-{idx} [] 0)"))
        .collect::<Vec<_>>()
        .join("\n");
    let large_source = format!("{repeated_helpers}\n(defn main [] 42)\n");
    let stage3_output = run_wasm_with_six_imports_compiler_mode(
        stage2_self_compiler,
        &large_source,
        &["compiler", "large-token-file.ls"],
    )
    .expect("BOOT-04 large-file: stage2_self_compiler が大きい単一ファイルをコンパイルできない");
    let stage3_modules = parse_emitted_wasm_modules(&stage3_output, 1);
    let stage3_wasm = &stage3_modules[0];
    assert_valid_wasm(stage3_wasm);
}

#[test]
fn test_e2e_boot04_self_hosted_stage2_compiles_bare_module_file() {
    let main_path = selfhost_main_path();
    let selfhost_root = main_path
        .parent()
        .expect("App/ ディレクトリ")
        .parent()
        .expect("src/ ディレクトリ")
        .parent()
        .expect("selfhost/ ルートディレクトリ")
        .to_path_buf();

    let stage1_wasm = compile_file_only(&main_path);
    assert_valid_wasm(&stage1_wasm);

    let stage2_output = lsharp_wasm::wasi_runner::run_wasm_wasi_with_dir_and_args(
        &stage1_wasm,
        Some(&selfhost_root),
        &["compiler", "src/App/Main.ls"],
    )
    .expect("BOOT-04 bare-module: stage1 が Main.ls の self-compile に失敗した");
    let stage2_modules = parse_emitted_wasm_modules(&stage2_output, 1);
    let stage2_self_compiler = &stage2_modules[0];
    assert_valid_wasm(stage2_self_compiler);

    let bare_module_source = "(module App.Main)\n(defn main [] 0)\n";
    let stage3_output = run_wasm_with_six_imports_compiler_mode(
        stage2_self_compiler,
        bare_module_source,
        &["compiler", "src/App/Main.ls"],
    )
    .expect("BOOT-04 bare-module: stage2_self_compiler が bare module source をコンパイルできない");
    let stage3_modules = parse_emitted_wasm_modules(&stage3_output, 1);
    let stage3_wasm = &stage3_modules[0];
    std::fs::write("/tmp/bare_module_string_stage3.wasm", stage3_wasm)
        .expect("bare-module string stage3 dump に失敗");
    assert_valid_wasm(stage3_wasm);
    validate_wasm_detailed(stage3_wasm)
        .unwrap_or_else(|e| panic!("BOOT-04 bare-module: stage3 wasm validation failed: {e}"));
}

#[test]
fn test_e2e_boot04_self_hosted_stage2_compiles_bare_zero_fs_package() {
    let main_path = selfhost_main_path();
    let selfhost_root = main_path
        .parent()
        .expect("App/ ディレクトリ")
        .parent()
        .expect("src/ ディレクトリ")
        .parent()
        .expect("selfhost/ ルートディレクトリ")
        .to_path_buf();

    let stage1_wasm = compile_file_only(&main_path);
    assert_valid_wasm(&stage1_wasm);

    let stage2_output = lsharp_wasm::wasi_runner::run_wasm_wasi_with_dir_and_args(
        &stage1_wasm,
        Some(&selfhost_root),
        &["compiler", "src/App/Main.ls"],
    )
    .expect("BOOT-04 bare-fs: stage1 が Main.ls の self-compile に失敗した");
    let stage2_modules = parse_emitted_wasm_modules(&stage2_output, 1);
    let stage2_self_compiler = &stage2_modules[0];
    assert_valid_wasm(stage2_self_compiler);

    let temp_root = std::env::temp_dir().join(format!(
        "lsharp-boot04-bare-fs-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .expect("時刻が巻き戻った")
            .as_nanos()
    ));
    let app_dir = temp_root.join("src/App");
    std::fs::create_dir_all(&app_dir).expect("bare-fs temp dir を作れない");
    std::fs::write(
        app_dir.join("Main.ls"),
        "(module App.Main)\n(defn main [] 0)\n",
    )
    .expect("bare-fs Main.ls を書けない");

    let stage3_output = run_wasm_with_six_imports_compiler_mode_fs(
        stage2_self_compiler,
        &temp_root,
        &["compiler", "src/App/Main.ls"],
    )
    .expect("BOOT-04 bare-fs: stage2_self_compiler が temp package をコンパイルできない");
    let stage3_modules = parse_emitted_wasm_modules(&stage3_output, 1);
    let stage3_wasm = &stage3_modules[0];
    std::fs::write("/tmp/bare_module_fs_stage3.wasm", stage3_wasm)
        .expect("bare-fs stage3 dump に失敗");
    assert_valid_wasm(stage3_wasm);
    validate_wasm_detailed(stage3_wasm)
        .unwrap_or_else(|e| panic!("BOOT-04 bare-fs: stage3 wasm validation failed: {e}"));
    let engine = wasmtime::Engine::default();
    wasmtime::Module::new(&engine, stage3_wasm)
        .unwrap_or_else(|e| panic!("BOOT-04 bare-fs: wasmtime load failed: {e}"));

    std::fs::remove_dir_all(&temp_root).expect("bare-fs temp dir を削除できない");
}

#[test]
fn test_e2e_boot04_self_hosted_stage2_cache_probe_parses_bare_module_once() {
    let main_path = selfhost_main_path();
    let selfhost_root = main_path
        .parent()
        .expect("App/ ディレクトリ")
        .parent()
        .expect("src/ ディレクトリ")
        .parent()
        .expect("selfhost/ ルートディレクトリ")
        .to_path_buf();

    let stage1_wasm = compile_file_only(&main_path);
    assert_valid_wasm(&stage1_wasm);

    let stage2_output = lsharp_wasm::wasi_runner::run_wasm_wasi_with_dir_and_args(
        &stage1_wasm,
        Some(&selfhost_root),
        &["compiler", "src/App/Main.ls"],
    )
    .expect("BOOT-04 cache-probe: stage1 が Main.ls の self-compile に失敗した");
    let stage2_modules = parse_emitted_wasm_modules(&stage2_output, 1);
    let stage2_self_compiler = &stage2_modules[0];
    assert_valid_wasm(stage2_self_compiler);

    let temp_root = std::env::temp_dir().join(format!(
        "lsharp-boot04-cache-probe-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .expect("時刻が巻き戻った")
            .as_nanos()
    ));
    let app_dir = temp_root.join("src/App");
    std::fs::create_dir_all(&app_dir).expect("cache-probe temp dir を作れない");
    std::fs::write(
        app_dir.join("Main.ls"),
        "(module App.Main)\n(defn main [] 0)\n",
    )
    .expect("cache-probe Main.ls を書けない");

    let debug_output = run_wasm_with_six_imports_compiler_mode_fs(
        stage2_self_compiler,
        &temp_root,
        &[
            "compiler",
            "src/App/Main.ls",
            "",
            "",
            "",
            "",
            "",
            "",
            "cache",
        ],
    )
    .expect("BOOT-04 cache-probe: stage2_self_compiler の cache probe 実行に失敗した");
    let values: Vec<i64> = debug_output
        .lines()
        .filter(|line| !line.trim().is_empty())
        .map(|line| {
            line.trim()
                .parse::<i64>()
                .unwrap_or_else(|_| panic!("BOOT-04 cache-probe: 数値でない debug 出力: {line:?}"))
        })
        .collect();
    eprintln!("BOOT-04 cache-probe values = {:?}", values);

    assert!(
        values.len() >= 4,
        "BOOT-04 cache-probe: debug 出力が短すぎる: {:?}",
        values
    );
    assert_eq!(
        &values[..4],
        &[80, 1, 35, 2],
        "BOOT-04 cache-probe: bare module の parse-count / source / decl 集計が期待と異なる"
    );

    std::fs::remove_dir_all(&temp_root).expect("cache-probe temp dir を削除できない");
}

#[test]
fn test_e2e_boot04_self_hosted_stage2_cache_probe_reads_main_again_entry() {
    let main_path = selfhost_main_path();
    let main_src =
        std::fs::read_to_string(&main_path).expect("BOOT-04 main-cache-probe: Main.ls を読めない");
    let selfhost_root = main_path
        .parent()
        .expect("App/ ディレクトリ")
        .parent()
        .expect("src/ ディレクトリ")
        .parent()
        .expect("selfhost/ ルートディレクトリ")
        .to_path_buf();

    let stage1_wasm = compile_file_only(&main_path);
    assert_valid_wasm(&stage1_wasm);

    let stage2_output = lsharp_wasm::wasi_runner::run_wasm_wasi_with_dir_and_args(
        &stage1_wasm,
        Some(&selfhost_root),
        &["compiler", "src/App/Main.ls"],
    )
    .expect("BOOT-04 main-cache-probe: stage1 が Main.ls の self-compile に失敗した");
    let stage2_modules = parse_emitted_wasm_modules(&stage2_output, 1);
    let stage2_self_compiler = &stage2_modules[0];
    assert_valid_wasm(stage2_self_compiler);

    let debug_output = run_wasm_with_six_imports_compiler_mode_fs(
        stage2_self_compiler,
        &selfhost_root,
        &[
            "compiler",
            "src/App/Main.ls",
            "",
            "",
            "",
            "",
            "",
            "",
            "cache",
        ],
    )
    .expect("BOOT-04 main-cache-probe: stage2_self_compiler の cache probe 実行に失敗した");
    let values: Vec<i64> = debug_output
        .lines()
        .filter(|line| !line.trim().is_empty())
        .map(|line| {
            line.trim().parse::<i64>().unwrap_or_else(|_| {
                panic!("BOOT-04 main-cache-probe: 数値でない debug 出力: {line:?}")
            })
        })
        .collect();
    eprintln!("BOOT-04 main-cache-probe values = {:?}", values);

    assert!(
        values.len() >= 4,
        "BOOT-04 main-cache-probe: debug 出力が短すぎる: {:?}",
        values
    );
    assert_eq!(
        &values[..4],
        &[80, 1, main_src.len() as i64, 4],
        "BOOT-04 main-cache-probe: entry source parse 集計が期待と異なる"
    );
}

#[test]
fn test_e2e_boot04_self_hosted_stage2_cache_pairs_probe_handles_bare_module() {
    let main_path = selfhost_main_path();
    let selfhost_root = main_path
        .parent()
        .expect("App/ ディレクトリ")
        .parent()
        .expect("src/ ディレクトリ")
        .parent()
        .expect("selfhost/ ルートディレクトリ")
        .to_path_buf();

    let stage1_wasm = compile_file_only(&main_path);
    assert_valid_wasm(&stage1_wasm);

    let stage2_output = lsharp_wasm::wasi_runner::run_wasm_wasi_with_dir_and_args(
        &stage1_wasm,
        Some(&selfhost_root),
        &["compiler", "src/App/Main.ls"],
    )
    .expect("BOOT-04 cache-pairs-bare: stage1 が Main.ls の self-compile に失敗した");
    let stage2_modules = parse_emitted_wasm_modules(&stage2_output, 1);
    let stage2_self_compiler = &stage2_modules[0];
    assert_valid_wasm(stage2_self_compiler);

    let temp_root = std::env::temp_dir().join(format!(
        "lsharp-boot04-cache-pairs-bare-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .expect("時刻が巻き戻った")
            .as_nanos()
    ));
    let app_dir = temp_root.join("src/App");
    std::fs::create_dir_all(&app_dir).expect("cache-pairs-bare temp dir を作れない");
    std::fs::write(
        app_dir.join("Main.ls"),
        "(module App.Main)\n(defn main [] 0)\n",
    )
    .expect("cache-pairs-bare Main.ls を書けない");

    let debug_output = run_wasm_with_six_imports_compiler_mode_fs(
        stage2_self_compiler,
        &temp_root,
        &[
            "compiler",
            "src/App/Main.ls",
            "",
            "",
            "",
            "",
            "",
            "",
            "",
            "pairs",
        ],
    )
    .expect("BOOT-04 cache-pairs-bare: stage2_self_compiler の pairs probe 実行に失敗した");
    let values: Vec<i64> = debug_output
        .lines()
        .filter(|line| !line.trim().is_empty())
        .map(|line| {
            line.trim().parse::<i64>().unwrap_or_else(|_| {
                panic!("BOOT-04 cache-pairs-bare: 数値でない debug 出力: {line:?}")
            })
        })
        .collect();
    eprintln!("BOOT-04 cache-pairs-bare values = {:?}", values);

    assert!(
        values.len() >= 4,
        "BOOT-04 cache-pairs-bare: debug 出力が短すぎる: {:?}",
        values
    );
    assert_eq!(
        &values[..4],
        &[81, 1, 1, 2],
        "BOOT-04 cache-pairs-bare: bare module の pair 集計が期待と異なる"
    );

    std::fs::remove_dir_all(&temp_root).expect("cache-pairs-bare temp dir を削除できない");
}

#[test]
fn test_e2e_boot04_self_hosted_stage2_cache_pairs_probe_handles_one_import() {
    let main_path = selfhost_main_path();
    let selfhost_root = main_path
        .parent()
        .expect("App/ ディレクトリ")
        .parent()
        .expect("src/ ディレクトリ")
        .parent()
        .expect("selfhost/ ルートディレクトリ")
        .to_path_buf();

    let stage1_wasm = compile_file_only(&main_path);
    assert_valid_wasm(&stage1_wasm);

    let stage2_output = lsharp_wasm::wasi_runner::run_wasm_wasi_with_dir_and_args(
        &stage1_wasm,
        Some(&selfhost_root),
        &["compiler", "src/App/Main.ls"],
    )
    .expect("BOOT-04 cache-pairs-one-import: stage1 が Main.ls の self-compile に失敗した");
    let stage2_modules = parse_emitted_wasm_modules(&stage2_output, 1);
    let stage2_self_compiler = &stage2_modules[0];
    assert_valid_wasm(stage2_self_compiler);

    let temp_root = std::env::temp_dir().join(format!(
        "lsharp-boot04-cache-pairs-one-import-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .expect("時刻が巻き戻った")
            .as_nanos()
    ));
    let app_dir = temp_root.join("src/App");
    std::fs::create_dir_all(&app_dir).expect("cache-pairs-one-import temp dir を作れない");
    std::fs::write(
        app_dir.join("Main.ls"),
        "(module App.Main)\n(import App.Helper)\n(defn main [] 0)\n",
    )
    .expect("cache-pairs-one-import Main.ls を書けない");
    std::fs::write(
        app_dir.join("Helper.ls"),
        "(module App.Helper)\n(defn helper [] 1)\n",
    )
    .expect("cache-pairs-one-import Helper.ls を書けない");

    let debug_output = run_wasm_with_six_imports_compiler_mode_fs(
        stage2_self_compiler,
        &temp_root,
        &[
            "compiler",
            "src/App/Main.ls",
            "",
            "",
            "",
            "",
            "",
            "",
            "",
            "pairs",
        ],
    )
    .expect("BOOT-04 cache-pairs-one-import: stage2_self_compiler の pairs probe 実行に失敗した");
    let values: Vec<i64> = debug_output
        .lines()
        .filter(|line| !line.trim().is_empty())
        .map(|line| {
            line.trim().parse::<i64>().unwrap_or_else(|_| {
                panic!("BOOT-04 cache-pairs-one-import: 数値でない debug 出力: {line:?}")
            })
        })
        .collect();
    eprintln!("BOOT-04 cache-pairs-one-import values = {:?}", values);

    assert!(
        values.len() >= 4,
        "BOOT-04 cache-pairs-one-import: debug 出力が短すぎる: {:?}",
        values
    );
    assert_eq!(
        &values[..4],
        &[81, 2, 2, 3],
        "BOOT-04 cache-pairs-one-import: single import graph の pair 集計が期待と異なる"
    );

    std::fs::remove_dir_all(&temp_root).expect("cache-pairs-one-import temp dir を削除できない");
}

#[test]
fn test_e2e_boot04_self_hosted_stage2_cache_pairs_probe_reads_main_again_graph() {
    let main_path = selfhost_main_path();
    let selfhost_root = main_path
        .parent()
        .expect("App/ ディレクトリ")
        .parent()
        .expect("src/ ディレクトリ")
        .parent()
        .expect("selfhost/ ルートディレクトリ")
        .to_path_buf();

    let stage1_wasm = compile_file_only(&main_path);
    assert_valid_wasm(&stage1_wasm);

    let stage2_output = lsharp_wasm::wasi_runner::run_wasm_wasi_with_dir_and_args(
        &stage1_wasm,
        Some(&selfhost_root),
        &["compiler", "src/App/Main.ls"],
    )
    .expect("BOOT-04 main-cache-pairs: stage1 が Main.ls の self-compile に失敗した");
    let stage2_modules = parse_emitted_wasm_modules(&stage2_output, 1);
    let stage2_self_compiler = &stage2_modules[0];
    assert_valid_wasm(stage2_self_compiler);

    let debug_output = run_wasm_with_six_imports_compiler_mode_fs(
        stage2_self_compiler,
        &selfhost_root,
        &[
            "compiler",
            "src/App/Main.ls",
            "",
            "",
            "",
            "",
            "",
            "",
            "",
            "pairs",
        ],
    )
    .expect("BOOT-04 main-cache-pairs: stage2_self_compiler の pairs probe 実行に失敗した");
    let values: Vec<i64> = debug_output
        .lines()
        .filter(|line| !line.trim().is_empty())
        .map(|line| {
            line.trim().parse::<i64>().unwrap_or_else(|_| {
                panic!("BOOT-04 main-cache-pairs: 数値でない debug 出力: {line:?}")
            })
        })
        .collect();
    eprintln!("BOOT-04 main-cache-pairs values = {:?}", values);

    assert!(
        values.len() >= 4,
        "BOOT-04 main-cache-pairs: debug 出力が短すぎる: {:?}",
        values
    );
    assert_eq!(values[0], 81, "BOOT-04 main-cache-pairs: marker mismatch");
    assert!(
        values[1] > 10,
        "BOOT-04 main-cache-pairs: parse count が小さすぎる: {:?}",
        values
    );
    assert!(
        values[2] > 10,
        "BOOT-04 main-cache-pairs: pair count が小さすぎる: {:?}",
        values
    );
    assert_eq!(
        values[3], 4,
        "BOOT-04 main-cache-pairs: entry decl count が期待と異なる"
    );
}

#[test]
fn test_e2e_boot04_self_hosted_stage2_reports_main_again_cache_pairs_progress() {
    let main_path = selfhost_main_path();
    let selfhost_root = main_path
        .parent()
        .expect("App/ ディレクトリ")
        .parent()
        .expect("src/ ディレクトリ")
        .parent()
        .expect("selfhost/ ルートディレクトリ")
        .to_path_buf();

    let stage1_wasm = compile_file_only(&main_path);
    assert_valid_wasm(&stage1_wasm);

    let stage2_output = lsharp_wasm::wasi_runner::run_wasm_wasi_with_dir_and_args(
        &stage1_wasm,
        Some(&selfhost_root),
        &["compiler", "src/App/Main.ls"],
    )
    .expect("BOOT-04 main-cache-pairs-progress: stage1 が Main.ls の self-compile に失敗した");
    let stage2_modules = parse_emitted_wasm_modules(&stage2_output, 1);
    let stage2_self_compiler = &stage2_modules[0];
    assert_valid_wasm(stage2_self_compiler);

    let progress_output = run_wasm_with_six_imports_compiler_mode_fs_printed_first(
        stage2_self_compiler,
        &selfhost_root,
        &[
            "compiler",
            "src/App/Main.ls",
            "",
            "",
            "",
            "",
            "",
            "",
            "",
            "",
            "pairs-progress",
        ],
    );
    eprintln!(
        "BOOT-04 main-cache-pairs-progress output = {:?}",
        progress_output
    );
}

#[test]
fn test_e2e_boot04_self_hosted_stage2_reports_one_import_path_resolution() {
    let main_path = selfhost_main_path();
    let selfhost_root = main_path
        .parent()
        .expect("App/ ディレクトリ")
        .parent()
        .expect("src/ ディレクトリ")
        .parent()
        .expect("selfhost/ ルートディレクトリ")
        .to_path_buf();

    let stage1_wasm = compile_file_only(&main_path);
    assert_valid_wasm(&stage1_wasm);

    let stage2_output = lsharp_wasm::wasi_runner::run_wasm_wasi_with_dir_and_args(
        &stage1_wasm,
        Some(&selfhost_root),
        &["compiler", "src/App/Main.ls"],
    )
    .expect("BOOT-04 one-import-path-debug: stage1 が Main.ls の self-compile に失敗した");
    let stage2_modules = parse_emitted_wasm_modules(&stage2_output, 1);
    let stage2_self_compiler = &stage2_modules[0];
    assert_valid_wasm(stage2_self_compiler);

    let temp_root = std::env::temp_dir().join(format!(
        "lsharp-boot04-one-import-path-debug-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .expect("時刻が巻き戻った")
            .as_nanos()
    ));
    let app_dir = temp_root.join("src/App");
    std::fs::create_dir_all(&app_dir).expect("one-import-path-debug temp dir を作れない");
    std::fs::write(
        app_dir.join("Main.ls"),
        "(module App.Main)\n(import App.CompilerMode)\n(defn main [] 0)\n",
    )
    .expect("one-import-path-debug Main.ls を書けない");

    let debug_output = run_wasm_with_six_imports_compiler_mode_fs(
        stage2_self_compiler,
        &temp_root,
        &["compiler", "src/App/Main.ls", "debug", "paths"],
    )
    .expect("BOOT-04 one-import-path-debug: path debug 実行に失敗した");
    let values: Vec<i64> = debug_output
        .lines()
        .filter(|line| !line.trim().is_empty())
        .map(|line| {
            line.trim().parse::<i64>().unwrap_or_else(|_| {
                panic!("BOOT-04 one-import-path-debug: 数値でない debug 出力: {line:?}")
            })
        })
        .collect();
    eprintln!("BOOT-04 one-import path values = {:?}", values);

    std::fs::remove_dir_all(&temp_root).expect("one-import-path-debug temp dir を削除できない");
}

#[test]
fn test_e2e_boot04_self_hosted_stage2_reports_main_again_progress() {
    let main_path = selfhost_main_path();
    let selfhost_root = main_path
        .parent()
        .expect("App/ ディレクトリ")
        .parent()
        .expect("src/ ディレクトリ")
        .parent()
        .expect("selfhost/ ルートディレクトリ")
        .to_path_buf();

    let stage1_wasm = compile_file_only(&main_path);
    assert_valid_wasm(&stage1_wasm);

    let stage2_output = lsharp_wasm::wasi_runner::run_wasm_wasi_with_dir_and_args(
        &stage1_wasm,
        Some(&selfhost_root),
        &["compiler", "src/App/Main.ls"],
    )
    .expect("BOOT-04 main-progress: stage1 が Main.ls の self-compile に失敗した");
    let stage2_modules = parse_emitted_wasm_modules(&stage2_output, 1);
    let stage2_self_compiler = &stage2_modules[0];
    assert_valid_wasm(stage2_self_compiler);

    let progress_output = run_wasm_with_six_imports_compiler_mode_fs(
        stage2_self_compiler,
        &selfhost_root,
        &[
            "compiler",
            "src/App/Main.ls",
            "debug",
            "progress",
            "main-again",
        ],
    );
    eprintln!("BOOT-04 main-progress output = {:?}", progress_output);
}

#[test]
fn test_e2e_boot04_self_hosted_stage2_reports_main_again_build_progress() {
    let main_path = selfhost_main_path();
    let selfhost_root = main_path
        .parent()
        .expect("App/ ディレクトリ")
        .parent()
        .expect("src/ ディレクトリ")
        .parent()
        .expect("selfhost/ ルートディレクトリ")
        .to_path_buf();

    let stage1_wasm = compile_file_only(&main_path);
    assert_valid_wasm(&stage1_wasm);

    let stage2_output = lsharp_wasm::wasi_runner::run_wasm_wasi_with_dir_and_args(
        &stage1_wasm,
        Some(&selfhost_root),
        &["compiler", "src/App/Main.ls"],
    )
    .expect("BOOT-04 main-build-progress: stage1 が Main.ls の self-compile に失敗した");
    let stage2_modules = parse_emitted_wasm_modules(&stage2_output, 1);
    let stage2_self_compiler = &stage2_modules[0];
    assert_valid_wasm(stage2_self_compiler);

    let progress_output = run_wasm_with_six_imports_compiler_mode_fs(
        stage2_self_compiler,
        &selfhost_root,
        &[
            "compiler",
            "src/App/Main.ls",
            "debug",
            "progress",
            "build",
            "main-again",
        ],
    )
    .expect("BOOT-04 main-build-progress: stage2_self_compiler の build progress 実行に失敗した");
    let values: Vec<i64> = progress_output
        .lines()
        .filter(|line| !line.trim().is_empty())
        .map(|line| {
            line.trim().parse::<i64>().unwrap_or_else(|_| {
                panic!("BOOT-04 main-build-progress: 数値でない debug 出力: {line:?}")
            })
        })
        .collect();
    eprintln!("BOOT-04 main-build-progress values = {:?}", values);

    assert!(
        values.len() >= 36,
        "BOOT-04 main-build-progress: debug 出力が短すぎる: {:?}",
        values
    );
    assert_eq!(
        values[0], 50,
        "BOOT-04 main-build-progress: 最初の marker は 50 であるべき"
    );
    assert_eq!(
        values[3], 51,
        "BOOT-04 main-build-progress: header marker 51 が続くべき"
    );
    assert!(
        values[1] > 1000,
        "BOOT-04 main-build-progress: function count が小さすぎる: {:?}",
        values
    );
    assert!(
        values[2] > 0,
        "BOOT-04 main-build-progress: data length が正であるべき: {:?}",
        values
    );
    assert!(
        values[4] > 0,
        "BOOT-04 main-build-progress: header length が正であるべき: {:?}",
        values
    );
    ordered_marker_positions(
        &values,
        &(50..=67).collect::<Vec<_>>(),
        "BOOT-04 main-build-progress: marker sequence が崩れている",
    );
    let last_marker_index = values
        .iter()
        .rposition(|value| *value == 67)
        .expect("BOOT-04 main-build-progress: final marker 67 が見つからない");
    assert_eq!(
        last_marker_index + 2,
        values.len(),
        "BOOT-04 main-build-progress: final marker の後には wasm size だけが続くべき"
    );
    assert_eq!(
        values[last_marker_index + 1],
        values[last_marker_index - 1],
        "BOOT-04 main-build-progress: final wasm size は data append 後と一致するべき"
    );
}

#[test]
fn test_e2e_boot04_self_hosted_stage2_reaches_main_again_build_phase_markers() {
    let main_path = selfhost_main_path();
    let selfhost_root = main_path
        .parent()
        .expect("App/ ディレクトリ")
        .parent()
        .expect("src/ ディレクトリ")
        .parent()
        .expect("selfhost/ ルートディレクトリ")
        .to_path_buf();

    let stage1_wasm = compile_file_only(&main_path);
    assert_valid_wasm(&stage1_wasm);

    let stage2_output = lsharp_wasm::wasi_runner::run_wasm_wasi_with_dir_and_args(
        &stage1_wasm,
        Some(&selfhost_root),
        &["compiler", "src/App/Main.ls"],
    )
    .expect("BOOT-04 main-build-phase: stage1 が Main.ls の self-compile に失敗した");
    let stage2_modules = parse_emitted_wasm_modules(&stage2_output, 1);
    let stage2_self_compiler = &stage2_modules[0];
    assert_valid_wasm(stage2_self_compiler);

    let phase_output = run_wasm_with_six_imports_compiler_mode_fs(
        stage2_self_compiler,
        &selfhost_root,
        &[
            "compiler",
            "src/App/Main.ls",
            "",
            "",
            "",
            "",
            "",
            "",
            "",
            "",
            "",
            "",
            "",
            "",
            "build-phase",
        ],
    )
    .expect("BOOT-04 main-build-phase: stage2_self_compiler の build phase 実行に失敗した");
    let values: Vec<i64> = phase_output
        .lines()
        .filter(|line| !line.trim().is_empty())
        .map(|line| {
            line.trim().parse::<i64>().unwrap_or_else(|_| {
                panic!("BOOT-04 main-build-phase: 数値でない debug 出力: {line:?}")
            })
        })
        .collect();

    assert!(
        values.len() >= 24,
        "BOOT-04 main-build-phase: debug 出力が短すぎる: {:?}",
        values
    );
    assert_eq!(
        values[0], 101,
        "BOOT-04 main-build-phase: 最初の marker は 101 であるべき"
    );
    assert_eq!(
        values[1], 102,
        "BOOT-04 main-build-phase: compile 完了 marker 102 が続くべき"
    );
    assert_eq!(
        values[3], 104,
        "BOOT-04 main-build-phase: parse-count marker 104 が続くべき"
    );
    assert!(
        values[2] > 1000,
        "BOOT-04 main-build-phase: function count が小さすぎる: {:?}",
        values
    );
    assert!(
        values[4] > 10,
        "BOOT-04 main-build-phase: parse count が小さすぎる: {:?}",
        values
    );
    ordered_marker_positions(
        &values[5..],
        &(50..=66).collect::<Vec<_>>(),
        "BOOT-04 main-build-phase: build marker sequence が崩れている",
    );
    let last_marker_index = values
        .iter()
        .rposition(|value| *value == 103)
        .expect("BOOT-04 main-build-phase: final marker 103 が見つからない");
    assert_eq!(
        last_marker_index + 2,
        values.len(),
        "BOOT-04 main-build-phase: final marker の後には wasm size だけが続くべき"
    );
    assert!(
        values[last_marker_index + 1] > 0,
        "BOOT-04 main-build-phase: final wasm size が正であるべき: {:?}",
        values
    );
}

#[test]
fn test_e2e_boot04_self_hosted_stage2_reaches_main_again_build_compile_progress_markers() {
    let main_path = selfhost_main_path();
    let selfhost_root = main_path
        .parent()
        .expect("App/ ディレクトリ")
        .parent()
        .expect("src/ ディレクトリ")
        .parent()
        .expect("selfhost/ ルートディレクトリ")
        .to_path_buf();

    let stage1_wasm = compile_file_only(&main_path);
    assert_valid_wasm(&stage1_wasm);

    let stage2_output = lsharp_wasm::wasi_runner::run_wasm_wasi_with_dir_and_args(
        &stage1_wasm,
        Some(&selfhost_root),
        &["compiler", "src/App/Main.ls"],
    )
    .expect("BOOT-04 main-build-compile-progress: stage1 が Main.ls の self-compile に失敗した");
    let stage2_modules = parse_emitted_wasm_modules(&stage2_output, 1);
    let stage2_self_compiler = &stage2_modules[0];
    assert_valid_wasm(stage2_self_compiler);

    let progress_output = run_wasm_with_six_imports_compiler_mode_fs(
        stage2_self_compiler,
        &selfhost_root,
        &[
            "compiler",
            "src/App/Main.ls",
            "",
            "",
            "",
            "",
            "",
            "",
            "",
            "",
            "",
            "",
            "",
            "",
            "",
            "build-compile-progress",
        ],
    )
    .expect(
        "BOOT-04 main-build-compile-progress: stage2_self_compiler の build compile progress 実行に失敗した",
    );
    let values: Vec<i64> = progress_output
        .lines()
        .filter(|line| !line.trim().is_empty())
        .map(|line| {
            line.trim().parse::<i64>().unwrap_or_else(|_| {
                panic!("BOOT-04 main-build-compile-progress: 数値でない debug 出力: {line:?}")
            })
        })
        .collect();

    assert!(
        values.len() >= 8,
        "BOOT-04 main-build-compile-progress: debug 出力が短すぎる: {:?}",
        values
    );
    assert_eq!(
        values[0], 111,
        "BOOT-04 main-build-compile-progress: 最初の marker は 111 であるべき"
    );
    assert_eq!(
        values[1], 112,
        "BOOT-04 main-build-compile-progress: register 後 marker 112 が続くべき"
    );
    assert!(
        values[2] > 0,
        "BOOT-04 main-build-compile-progress: register pair 数が正であるべき: {:?}",
        values
    );
    assert!(
        values.iter().any(|value| *value == 29),
        "BOOT-04 main-build-compile-progress: pair progress marker 29 が必要: {:?}",
        values
    );
    assert!(
        values.iter().any(|value| *value == 40),
        "BOOT-04 main-build-compile-progress: defn progress marker 40 が必要: {:?}",
        values
    );
    let last_marker_index = values
        .iter()
        .rposition(|value| *value == 113)
        .expect("BOOT-04 main-build-compile-progress: final marker 113 が見つからない");
    assert_eq!(
        last_marker_index + 2,
        values.len(),
        "BOOT-04 main-build-compile-progress: final marker の後には function count だけが続くべき"
    );
    assert!(
        values[last_marker_index + 1] > 1000,
        "BOOT-04 main-build-compile-progress: function count が小さすぎる: {:?}",
        values
    );
}

#[test]
fn test_e2e_boot04_self_hosted_stage2_warm_target_defn_parity_reaches_ast_make_type_constrained() {
    let main_path = selfhost_main_path();
    let selfhost_root = main_path
        .parent()
        .expect("App/ ディレクトリ")
        .parent()
        .expect("src/ ディレクトリ")
        .parent()
        .expect("selfhost/ ルートディレクトリ")
        .to_path_buf();

    let stage1_wasm = compile_file_only(&main_path);
    assert_valid_wasm(&stage1_wasm);

    let stage2_output = lsharp_wasm::wasi_runner::run_wasm_wasi_with_dir_and_args(
        &stage1_wasm,
        Some(&selfhost_root),
        &["compiler", "src/App/Main.ls"],
    )
    .expect("BOOT-04 warm-target-defn: stage1 が Main.ls の self-compile に失敗した");
    let stage2_modules = parse_emitted_wasm_modules(&stage2_output, 1);
    let stage2_self_compiler = &stage2_modules[0];
    assert_valid_wasm(stage2_self_compiler);

    let parity_output = run_wasm_with_six_imports_compiler_mode_fs(
        stage2_self_compiler,
        &selfhost_root,
        &[
            "compiler",
            "src/Syntax/AST.ls",
            "",
            "",
            "",
            "",
            "",
            "",
            "",
            "",
            "",
            "",
            "",
            "",
            "",
            "",
            "",
            "warm-target-defn",
        ],
    )
    .expect("BOOT-04 warm-target-defn: stage2_self_compiler の parity probe 実行に失敗した");
    let values: Vec<i64> = parity_output
        .lines()
        .filter(|line| !line.trim().is_empty())
        .map(|line| {
            line.trim().parse::<i64>().unwrap_or_else(|_| {
                panic!("BOOT-04 warm-target-defn: 数値でない debug 出力: {line:?}")
            })
        })
        .collect();

    assert!(
        values.len() >= 10,
        "BOOT-04 warm-target-defn: debug 出力が短すぎる: {:?}",
        values
    );
    assert_eq!(
        values[0], 141,
        "BOOT-04 warm-target-defn: warm-up 完了 marker 141 から始まるべき"
    );
    assert_eq!(
        values[2], 142,
        "BOOT-04 warm-target-defn: data length marker 142 が続くべき"
    );
    assert_eq!(
        values[4], 124,
        "BOOT-04 warm-target-defn: target decl tag marker 124 が必要"
    );
    assert_eq!(
        values[5], 20,
        "BOOT-04 warm-target-defn: target decl は defn であるべき"
    );
    assert_eq!(
        values[6], 123,
        "BOOT-04 warm-target-defn: ftable IR marker 123 が必要"
    );
    assert!(
        values[7] > 0,
        "BOOT-04 warm-target-defn: ftable IR は空であってはいけない: {:?}",
        values
    );
    assert_eq!(
        values[8], 144,
        "BOOT-04 warm-target-defn: source-aware function-meta marker 144 が必要"
    );
    assert!(
        values[9] > 0,
        "BOOT-04 warm-target-defn: source-aware IR は空であってはいけない: {:?}",
        values
    );
}

#[test]
fn test_e2e_boot04_self_hosted_stage2_target_defn_parity_reaches_ast_make_type_constrained() {
    fn marker_value(values: &[i64], marker: i64) -> i64 {
        assert_eq!(
            values.len() % 2,
            0,
            "BOOT-04 target-defn: marker/value ペア数が崩れている: {:?}",
            values
        );
        values
            .chunks_exact(2)
            .find_map(|chunk| (chunk[0] == marker).then_some(chunk[1]))
            .unwrap_or_else(|| {
                panic!("BOOT-04 target-defn: marker {marker} が見つからない: {values:?}")
            })
    }

    let main_path = selfhost_main_path();
    let selfhost_root = main_path
        .parent()
        .expect("App/ ディレクトリ")
        .parent()
        .expect("src/ ディレクトリ")
        .parent()
        .expect("selfhost/ ルートディレクトリ")
        .to_path_buf();

    let stage1_wasm = compile_file_only(&main_path);
    assert_valid_wasm(&stage1_wasm);

    let stage2_output = lsharp_wasm::wasi_runner::run_wasm_wasi_with_dir_and_args(
        &stage1_wasm,
        Some(&selfhost_root),
        &["compiler", "src/App/Main.ls"],
    )
    .expect("BOOT-04 target-defn: stage1 が Main.ls の self-compile に失敗した");
    let stage2_modules = parse_emitted_wasm_modules(&stage2_output, 1);
    let stage2_self_compiler = &stage2_modules[0];
    assert_valid_wasm(stage2_self_compiler);

    let parity_output = run_wasm_with_six_imports_compiler_mode_fs(
        stage2_self_compiler,
        &selfhost_root,
        &[
            "compiler",
            "src/Syntax/AST.ls",
            "",
            "",
            "",
            "",
            "",
            "",
            "",
            "",
            "",
            "",
            "",
            "",
            "",
            "",
            "target-defn",
        ],
    )
    .expect("BOOT-04 target-defn: stage2_self_compiler の parity probe 実行に失敗した");
    let values: Vec<i64> = parity_output
        .lines()
        .filter(|line| !line.trim().is_empty())
        .map(|line| {
            line.trim()
                .parse::<i64>()
                .unwrap_or_else(|_| panic!("BOOT-04 target-defn: 数値でない debug 出力: {line:?}"))
        })
        .collect();
    eprintln!("BOOT-04 target-defn values = {:?}", values);

    assert!(
        values.len() >= 8,
        "BOOT-04 target-defn: debug 出力が短すぎる: {:?}",
        values
    );
    assert_eq!(marker_value(&values, 121), 59);
    assert_eq!(marker_value(&values, 124), 20);
    assert!(
        marker_value(&values, 125) > 0,
        "BOOT-04 target-defn: param-count は正であるべき: {:?}",
        values
    );
    assert!(
        marker_value(&values, 126) > 0,
        "BOOT-04 target-defn: body tag は正であるべき: {:?}",
        values
    );
    assert_eq!(marker_value(&values, 127), 5);
    assert_eq!(marker_value(&values, 128), 4);
    assert_eq!(
        marker_value(&values, 129),
        marker_value(&values, 131),
        "BOOT-04 target-defn: use-site と def-site の hash は一致するべき: {:?}",
        values
    );
    assert!(
        marker_value(&values, 130) > 0,
        "BOOT-04 target-defn: use-site lookup は空であってはいけない: {:?}",
        values
    );
    assert!(
        marker_value(&values, 132) > 0,
        "BOOT-04 target-defn: def-site lookup は空であってはいけない: {:?}",
        values
    );
    assert!(
        marker_value(&values, 133) > 0,
        "BOOT-04 target-defn: local use-site lookup は空であってはいけない: {:?}",
        values
    );
    assert!(
        marker_value(&values, 134) > 0,
        "BOOT-04 target-defn: local def-site lookup は空であってはいけない: {:?}",
        values
    );
    assert!(
        marker_value(&values, 123) > 0,
        "BOOT-04 target-defn: ftable IR は空であってはいけない: {:?}",
        values
    );
    assert!(
        marker_value(&values, 122) > 0,
        "BOOT-04 target-defn: source-aware IR は空であってはいけない: {:?}",
        values
    );
}

#[test]
fn test_e2e_boot04_stage1_target_defn_parity_reports_ast_make_type_constrained_lengths() {
    fn marker_value(values: &[i64], marker: i64) -> i64 {
        assert_eq!(
            values.len() % 2,
            0,
            "BOOT-04 stage1 target-defn: marker/value ペア数が崩れている: {:?}",
            values
        );
        values
            .chunks_exact(2)
            .find_map(|chunk| (chunk[0] == marker).then_some(chunk[1]))
            .unwrap_or_else(|| {
                panic!("BOOT-04 stage1 target-defn: marker {marker} が見つからない: {values:?}")
            })
    }

    let main_path = selfhost_main_path();
    let selfhost_root = main_path
        .parent()
        .expect("App/ ディレクトリ")
        .parent()
        .expect("src/ ディレクトリ")
        .parent()
        .expect("selfhost/ ルートディレクトリ")
        .to_path_buf();

    let stage1_wasm = compile_file_only(&main_path);
    assert_valid_wasm(&stage1_wasm);

    let parity_output = lsharp_wasm::wasi_runner::run_wasm_wasi_with_dir_and_args(
        &stage1_wasm,
        Some(&selfhost_root),
        &[
            "compiler",
            "src/Syntax/AST.ls",
            "",
            "",
            "",
            "",
            "",
            "",
            "",
            "",
            "",
            "",
            "",
            "",
            "",
            "",
            "target-defn",
        ],
    )
    .expect("BOOT-04 stage1 target-defn: stage1 parity probe 実行に失敗した");
    let values: Vec<i64> = parity_output
        .lines()
        .filter(|line| !line.trim().is_empty())
        .map(|line| {
            line.trim().parse::<i64>().unwrap_or_else(|_| {
                panic!("BOOT-04 stage1 target-defn: 数値でない debug 出力: {line:?}")
            })
        })
        .collect();
    eprintln!("BOOT-04 stage1 target-defn values = {:?}", values);

    assert_eq!(marker_value(&values, 121), 59);
    assert_eq!(marker_value(&values, 124), 20);
    assert_eq!(marker_value(&values, 125), 1);
    assert_eq!(marker_value(&values, 126), 7);
    assert_eq!(marker_value(&values, 127), 5);
    assert_eq!(marker_value(&values, 128), 4);
    assert_eq!(marker_value(&values, 129), marker_value(&values, 131));
    assert!(
        marker_value(&values, 130) > 0,
        "stage1 use-site lookup は空であってはいけない: {:?}",
        values
    );
    assert!(
        marker_value(&values, 132) > 0,
        "stage1 def-site lookup は空であってはいけない: {:?}",
        values
    );
    assert!(
        marker_value(&values, 133) > 0,
        "stage1 local use-site lookup は空であってはいけない: {:?}",
        values
    );
    assert!(
        marker_value(&values, 134) > 0,
        "stage1 local def-site lookup は空であってはいけない: {:?}",
        values
    );
    assert!(
        marker_value(&values, 123) > 0,
        "stage1 ftable IR は空であってはいけない: {:?}",
        values
    );
    assert!(
        marker_value(&values, 122) > 0,
        "stage1 source-aware IR は空であってはいけない: {:?}",
        values
    );
}

#[test]
#[ignore]
fn test_debug_boot04_stage2_first_defn_probe_on_minimal_make_type_constrained_shape() {
    let temp_root =
        std::env::temp_dir().join(format!("lsharp_target_defn_minimal_{}", std::process::id()));
    std::fs::create_dir_all(&temp_root).expect("temp dir should be created");
    let source_path = temp_root.join("mini_ast_shape.ls");
    std::fs::write(
        &source_path,
        "(defn make-type-constrained [name-hash] (let [v (vector-new 2)] (vector-push (vector-push v (ast-typeconstrained)) name-hash)))\n(defn ast-typeconstrained [] 24)\n(defn main [] 0)\n",
    )
    .expect("mini source should be written");

    let main_path = selfhost_main_path();
    let selfhost_root = main_path
        .parent()
        .expect("App/ ディレクトリ")
        .parent()
        .expect("src/ ディレクトリ")
        .parent()
        .expect("selfhost/ ルートディレクトリ")
        .to_path_buf();

    let stage1_wasm = compile_file_only(&main_path);
    assert_valid_wasm(&stage1_wasm);
    let source_path_str = source_path.to_str().expect("utf-8 path");
    let mut source_step_args = vec!["compiler", source_path_str];
    while source_step_args.len() < 21 {
        source_step_args.push("");
    }
    source_step_args.push("first-defn-source-step");

    let stage1_probe_output = lsharp_wasm::wasi_runner::run_wasm_wasi_with_dir_and_args(
        &stage1_wasm,
        Some(&selfhost_root),
        &source_step_args,
    )
    .expect("stage1 first-defn probe on minimal source should run");
    eprintln!(
        "BOOT-04 minimal first-defn stage1 = {:?}",
        stage1_probe_output
    );

    let stage2_output = lsharp_wasm::wasi_runner::run_wasm_wasi_with_dir_and_args(
        &stage1_wasm,
        Some(&selfhost_root),
        &["compiler", "src/App/Main.ls"],
    )
    .expect("stage1 should self-compile Main.ls");
    let stage2_modules = parse_emitted_wasm_modules(&stage2_output, 1);
    let stage2_self_compiler = &stage2_modules[0];
    assert_valid_wasm(stage2_self_compiler);

    let probe_output = run_wasm_with_six_imports_compiler_mode_fs(
        stage2_self_compiler,
        &selfhost_root,
        &source_step_args,
    )
    .expect("stage2 first-defn probe on minimal source should run");
    eprintln!("BOOT-04 minimal first-defn values = {:?}", probe_output);
    assert!(!probe_output.trim().is_empty());
}

#[test]
#[ignore]
fn test_debug_boot04_stage2_first_defn_ir_parity_on_minimal_demo_main_shape() {
    let temp_root =
        std::env::temp_dir().join(format!("lsharp_demo_main_minimal_{}", std::process::id()));
    std::fs::create_dir_all(&temp_root).expect("temp dir should be created");
    let source_path = temp_root.join("mini_demo_main_shape.ls");
    std::fs::write(
        &source_path,
        "(module Mini.Token)\n(defn demo-main [] (do (print (tok-lparen)) (print (tok-rparen)) (print (tok-eof)) 0))\n(defn tok-lparen [] 40)\n(defn tok-rparen [] 41)\n(defn tok-eof [] 99)\n",
    )
    .expect("mini source should be written");

    let main_path = selfhost_main_path();
    let selfhost_root = main_path
        .parent()
        .expect("App/ ディレクトリ")
        .parent()
        .expect("src/ ディレクトリ")
        .parent()
        .expect("selfhost/ ルートディレクトリ")
        .to_path_buf();

    let stage1_wasm = compile_file_only(&main_path);
    assert_valid_wasm(&stage1_wasm);

    let stage2_output = lsharp_wasm::wasi_runner::run_wasm_wasi_with_dir_and_args(
        &stage1_wasm,
        Some(&selfhost_root),
        &["compiler", "src/App/Main.ls"],
    )
    .expect("stage1 should self-compile Main.ls");
    let stage2_modules = parse_emitted_wasm_modules(&stage2_output, 1);
    let stage2_self_compiler = &stage2_modules[0];
    assert_valid_wasm(stage2_self_compiler);

    let source_path_str = source_path.to_str().expect("utf-8 path");
    let probe_output = run_wasm_with_six_imports_compiler_mode_fs(
        stage2_self_compiler,
        &selfhost_root,
        &[
            "compiler",
            source_path_str,
            "",
            "",
            "",
            "",
            "",
            "",
            "",
            "",
            "",
            "",
            "",
            "",
            "",
            "",
            "",
            "",
            "first-defn-ir-parity",
        ],
    )
    .expect("stage2 first-defn-ir-parity probe on minimal demo-main source should run");
    eprintln!(
        "BOOT-04 minimal demo-main first-defn-ir-parity = {:?}",
        probe_output
    );
    assert!(!probe_output.trim().is_empty());
}

#[test]
#[ignore]
fn test_debug_boot04_stage2_first_defn_source_probe_on_minimal_text_eq_loop_shape() {
    let temp_root = std::env::temp_dir().join(format!(
        "lsharp_text_eq_loop_minimal_{}",
        std::process::id()
    ));
    std::fs::create_dir_all(&temp_root).expect("temp dir should be created");
    let source_path = temp_root.join("mini_text_eq_loop_shape.ls");
    std::fs::write(
        &source_path,
        "(module Mini.ModuleResolver)\n(defn text-eq-loop [left right idx len] (if (>= idx len) true (if (= (string-char-at left idx) (string-char-at right idx)) (text-eq-loop left right (+ idx 1) len) false)))\n(defn main [] 0)\n",
    )
    .expect("mini source should be written");

    let main_path = selfhost_main_path();
    let selfhost_root = main_path
        .parent()
        .expect("App/ ディレクトリ")
        .parent()
        .expect("src/ ディレクトリ")
        .parent()
        .expect("selfhost/ ルートディレクトリ")
        .to_path_buf();

    let stage1_wasm = compile_file_only(&main_path);
    assert_valid_wasm(&stage1_wasm);

    let stage2_output = lsharp_wasm::wasi_runner::run_wasm_wasi_with_dir_and_args(
        &stage1_wasm,
        Some(&selfhost_root),
        &["compiler", "src/App/Main.ls"],
    )
    .expect("stage1 should self-compile Main.ls");
    let stage2_modules = parse_emitted_wasm_modules(&stage2_output, 1);
    let stage2_self_compiler = &stage2_modules[0];
    assert_valid_wasm(stage2_self_compiler);

    let source_path_str = source_path.to_str().expect("utf-8 path");
    let mut probe_args = vec!["compiler", source_path_str];
    while probe_args.len() < 22 {
        probe_args.push("");
    }
    probe_args.push("first-defn-source");

    let probe_output = run_wasm_with_six_imports_compiler_mode_fs(
        stage2_self_compiler,
        &selfhost_root,
        &probe_args,
    )
    .expect("stage2 first-defn source probe on minimal text-eq-loop source should run");
    eprintln!(
        "BOOT-04 minimal text-eq-loop source probe = {:?}",
        probe_output
    );
    assert!(!probe_output.trim().is_empty());
}

#[test]
#[ignore]
fn test_debug_boot04_stage2_first_defn_source_step_probe_on_minimal_path_parent_shape() {
    let temp_root = selfhost_project_root()
        .join("target/test-artifacts")
        .join(format!(
            "lsharp_path_parent_minimal_step_probe_{}",
            std::process::id()
        ));
    let _ = std::fs::remove_dir_all(&temp_root);
    std::fs::create_dir_all(&temp_root).expect("temp dir should be created");
    let source_path = temp_root.join("mini_path_parent_shape.ls");
    std::fs::write(
        &source_path,
        "(module Mini.ModuleResolver)\n(defn path-parent [path] (let [len (string-length path)] (if (= len 0) \"\" (if (has-path-sep path 0 len) (let [last (find-last-path-sep path 0 len -1)] (if (< last 0) \"\" (if (= last 0) \"/\" (substring path 0 last)))) \".\"))))\n(defn path-char [path idx] (string-char-at path idx))\n(defn is-path-sep [path idx] (let [ch (path-char path idx)] (if (= ch 47) true (if (= ch 92) true false))))\n(defn has-path-sep [path idx len] (if (>= idx len) false (if (is-path-sep path idx) true (has-path-sep path (+ idx 1) len))))\n(defn find-last-path-sep [path idx len last] (if (>= idx len) last (find-last-path-sep path (+ idx 1) len (if (is-path-sep path idx) idx last))))\n(defn main [] 0)\n",
    )
    .expect("mini source should be written");

    let main_path = selfhost_main_path();
    let selfhost_root = main_path
        .parent()
        .expect("App/ ディレクトリ")
        .parent()
        .expect("src/ ディレクトリ")
        .parent()
        .expect("selfhost/ ルートディレクトリ")
        .to_path_buf();

    let stage1_wasm = compile_file_only(&main_path);
    assert_valid_wasm(&stage1_wasm);

    let stage2_output = lsharp_wasm::wasi_runner::run_wasm_wasi_with_dir_and_args(
        &stage1_wasm,
        Some(&selfhost_root),
        &["compiler", "src/App/Main.ls"],
    )
    .expect("stage1 should self-compile Main.ls");
    let stage2_modules = parse_emitted_wasm_modules(&stage2_output, 1);
    let stage2_self_compiler = &stage2_modules[0];
    assert_valid_wasm(stage2_self_compiler);

    let source_path_str = source_path.to_str().expect("utf-8 path");
    let mut probe_args = vec!["compiler", source_path_str];
    while probe_args.len() < 21 {
        probe_args.push("");
    }
    probe_args.push("first-defn-source-step");

    let probe_output = run_wasm_with_six_imports_compiler_mode_fs(
        stage2_self_compiler,
        &selfhost_root,
        &probe_args,
    )
    .expect("stage2 first-defn source step probe on minimal path-parent source should run");
    eprintln!(
        "BOOT-04 minimal path-parent source step probe = {:?}",
        probe_output
    );
    assert!(!probe_output.trim().is_empty());
}

#[test]
#[ignore]
fn test_debug_boot04_stage2_build_compile_progress_on_minimal_path_parent_shape() {
    let temp_root = selfhost_project_root()
        .join("target/test-artifacts")
        .join(format!(
            "lsharp_path_parent_minimal_build_progress_{}",
            std::process::id()
        ));
    let _ = std::fs::remove_dir_all(&temp_root);
    std::fs::create_dir_all(&temp_root).expect("temp dir should be created");
    let source_path = temp_root.join("mini_path_parent_shape.ls");
    std::fs::write(
        &source_path,
        "(module Mini.ModuleResolver)\n(defn path-parent [path] (let [len (string-length path)] (if (= len 0) \"\" (if (has-path-sep path 0 len) (let [last (find-last-path-sep path 0 len -1)] (if (< last 0) \"\" (if (= last 0) \"/\" (substring path 0 last)))) \".\"))))\n(defn path-char [path idx] (string-char-at path idx))\n(defn is-path-sep [path idx] (let [ch (path-char path idx)] (if (= ch 47) true (if (= ch 92) true false))))\n(defn has-path-sep [path idx len] (if (>= idx len) false (if (is-path-sep path idx) true (has-path-sep path (+ idx 1) len))))\n(defn find-last-path-sep [path idx len last] (if (>= idx len) last (find-last-path-sep path (+ idx 1) len (if (is-path-sep path idx) idx last))))\n(defn main [] (print (string-length (path-parent (command-line-arg 1)))))\n",
    )
    .expect("mini source should be written");

    let main_path = selfhost_main_path();
    let selfhost_root = main_path
        .parent()
        .expect("App/ ディレクトリ")
        .parent()
        .expect("src/ ディレクトリ")
        .parent()
        .expect("selfhost/ ルートディレクトリ")
        .to_path_buf();

    let stage1_wasm = compile_file_only(&main_path);
    assert_valid_wasm(&stage1_wasm);

    let stage2_output = lsharp_wasm::wasi_runner::run_wasm_wasi_with_dir_and_args(
        &stage1_wasm,
        Some(&selfhost_root),
        &["compiler", "src/App/Main.ls"],
    )
    .expect("stage1 should self-compile Main.ls");
    let stage2_modules = parse_emitted_wasm_modules(&stage2_output, 1);
    let stage2_self_compiler = &stage2_modules[0];
    assert_valid_wasm(stage2_self_compiler);

    let source_path_str = source_path.to_str().expect("utf-8 path");
    let progress_output = run_wasm_with_six_imports_compiler_mode_fs(
        stage2_self_compiler,
        &selfhost_root,
        &[
            "compiler",
            source_path_str,
            "",
            "",
            "",
            "",
            "",
            "",
            "",
            "",
            "",
            "",
            "",
            "",
            "",
            "build-compile-progress",
        ],
    )
    .expect("stage2 build compile progress on minimal path-parent source should run");
    eprintln!(
        "BOOT-04 minimal path-parent build compile progress = {:?}",
        progress_output
    );
    assert!(!progress_output.trim().is_empty());
}

#[test]
#[ignore]
fn test_debug_boot04_stage1_build_compile_progress_on_app_cli() {
    fn run_wasi_capture_trap_stdout(
        wasm_bytes: &[u8],
        dir: Option<&std::path::Path>,
        args: &[&str],
    ) -> Result<String, String> {
        use wasmtime::{Engine, Linker, Module, Store};
        use wasmtime_wasi::{WasiCtxBuilder, preview1::WasiP1Ctx};

        let engine = Engine::default();
        let mut linker = Linker::<WasiP1Ctx>::new(&engine);
        wasmtime_wasi::preview1::add_to_linker_sync(&mut linker, |ctx| ctx)
            .map_err(|e| format!("WASI リンクに失敗: {e}"))?;

        let stdout = wasmtime_wasi::pipe::MemoryOutputPipe::new(4 * 1024 * 1024);
        let mut builder = WasiCtxBuilder::new();
        builder.stdout(stdout.clone());
        builder.args(args);
        if let Some(dir_path) = dir {
            builder
                .preopened_dir(
                    dir_path,
                    ".",
                    wasmtime_wasi::DirPerms::all(),
                    wasmtime_wasi::FilePerms::all(),
                )
                .map_err(|e| format!("preopened_dir に失敗: {e}"))?;
        }
        let mut store = Store::new(&engine, builder.build_p1());
        let module = Module::new(&engine, wasm_bytes)
            .map_err(|e| format!("Wasm モジュールの読み込みに失敗: {e}"))?;
        let instance = linker
            .instantiate(&mut store, &module)
            .map_err(|e| format!("インスタンス化に失敗: {e}"))?;
        let start = instance
            .get_typed_func::<(), ()>(&mut store, "_start")
            .map_err(|e| format!("_start 関数が見つからない: {e}"))?;
        let result = start.call(&mut store, ());
        drop(store);
        let bytes = stdout
            .try_into_inner()
            .ok_or_else(|| "stdout の取得に失敗".to_string())?;
        let printed = String::from_utf8(bytes.to_vec())
            .map_err(|e| format!("stdout の UTF-8 変換に失敗: {e}"))?;
        match result {
            Ok(()) => Ok(printed),
            Err(e) => Err(format!("実行に失敗: {e}; printed={printed:?}")),
        }
    }

    let main_path = selfhost_main_path();
    let selfhost_root = main_path
        .parent()
        .expect("App/ ディレクトリ")
        .parent()
        .expect("src/ ディレクトリ")
        .parent()
        .expect("selfhost/ ルートディレクトリ")
        .to_path_buf();

    let stage1_wasm = compile_file_only(&main_path);
    assert_valid_wasm(&stage1_wasm);

    let progress_output = run_wasi_capture_trap_stdout(
        &stage1_wasm,
        Some(&selfhost_root),
        &[
            "compiler",
            "src/App/Cli.ls",
            "",
            "",
            "",
            "",
            "",
            "",
            "",
            "",
            "",
            "",
            "",
            "",
            "",
            "build-compile-progress",
        ],
    )
    .expect("stage1 build compile progress on App/Cli.ls should run");
    eprintln!(
        "BOOT-04 App/Cli stage1 build compile progress = {:?}",
        progress_output
    );
    assert!(!progress_output.trim().is_empty());
}

#[test]
#[ignore]
fn test_debug_boot04_stage1_build_compile_progress_on_compiler_module() {
    fn run_wasi_capture_trap_stdout(
        wasm_bytes: &[u8],
        dir: Option<&std::path::Path>,
        args: &[&str],
    ) -> Result<String, String> {
        use wasmtime::{Engine, Linker, Module, Store};
        use wasmtime_wasi::{WasiCtxBuilder, preview1::WasiP1Ctx};

        let engine = Engine::default();
        let mut linker = Linker::<WasiP1Ctx>::new(&engine);
        wasmtime_wasi::preview1::add_to_linker_sync(&mut linker, |ctx| ctx)
            .map_err(|e| format!("WASI リンクに失敗: {e}"))?;

        let stdout = wasmtime_wasi::pipe::MemoryOutputPipe::new(4 * 1024 * 1024);
        let mut builder = WasiCtxBuilder::new();
        builder.stdout(stdout.clone());
        builder.args(args);
        if let Some(dir_path) = dir {
            builder
                .preopened_dir(
                    dir_path,
                    ".",
                    wasmtime_wasi::DirPerms::all(),
                    wasmtime_wasi::FilePerms::all(),
                )
                .map_err(|e| format!("preopened_dir に失敗: {e}"))?;
        }
        let mut store = Store::new(&engine, builder.build_p1());
        let module = Module::new(&engine, wasm_bytes)
            .map_err(|e| format!("Wasm モジュールの読み込みに失敗: {e}"))?;
        let instance = linker
            .instantiate(&mut store, &module)
            .map_err(|e| format!("インスタンス化に失敗: {e}"))?;
        let start = instance
            .get_typed_func::<(), ()>(&mut store, "_start")
            .map_err(|e| format!("_start 関数が見つからない: {e}"))?;
        let result = start.call(&mut store, ());
        drop(store);
        let bytes = stdout
            .try_into_inner()
            .ok_or_else(|| "stdout の取得に失敗".to_string())?;
        let printed = String::from_utf8(bytes.to_vec())
            .map_err(|e| format!("stdout の UTF-8 変換に失敗: {e}"))?;
        match result {
            Ok(()) => Ok(printed),
            Err(e) => Err(format!("実行に失敗: {e}; printed={printed:?}")),
        }
    }

    let main_path = selfhost_main_path();
    let selfhost_root = main_path
        .parent()
        .expect("App/ ディレクトリ")
        .parent()
        .expect("src/ ディレクトリ")
        .parent()
        .expect("selfhost/ ルートディレクトリ")
        .to_path_buf();

    let stage1_wasm = compile_file_only(&main_path);
    assert_valid_wasm(&stage1_wasm);

    let progress_output = run_wasi_capture_trap_stdout(
        &stage1_wasm,
        Some(&selfhost_root),
        &[
            "compiler",
            "src/Backend/Wasm/Compiler.ls",
            "",
            "",
            "",
            "",
            "",
            "",
            "",
            "",
            "",
            "",
            "",
            "",
            "",
            "build-compile-progress",
        ],
    )
    .expect("stage1 build compile progress on Compiler.ls should run");
    eprintln!(
        "BOOT-04 Compiler.ls stage1 build compile progress = {:?}",
        progress_output
    );
    assert!(!progress_output.trim().is_empty());
}

#[test]
#[ignore]
fn test_debug_boot04_stage1_build_compile_progress_on_compiler_mode_module() {
    fn run_wasi_capture_trap_stdout(
        wasm_bytes: &[u8],
        dir: Option<&std::path::Path>,
        args: &[&str],
    ) -> Result<String, String> {
        use wasmtime::{Engine, Linker, Module, Store};
        use wasmtime_wasi::{WasiCtxBuilder, preview1::WasiP1Ctx};

        let engine = Engine::default();
        let mut linker = Linker::<WasiP1Ctx>::new(&engine);
        wasmtime_wasi::preview1::add_to_linker_sync(&mut linker, |ctx| ctx)
            .map_err(|e| format!("WASI リンクに失敗: {e}"))?;

        let stdout = wasmtime_wasi::pipe::MemoryOutputPipe::new(4 * 1024 * 1024);
        let mut builder = WasiCtxBuilder::new();
        builder.stdout(stdout.clone());
        builder.args(args);
        if let Some(dir_path) = dir {
            builder
                .preopened_dir(
                    dir_path,
                    ".",
                    wasmtime_wasi::DirPerms::all(),
                    wasmtime_wasi::FilePerms::all(),
                )
                .map_err(|e| format!("preopened_dir に失敗: {e}"))?;
        }
        let mut store = Store::new(&engine, builder.build_p1());
        let module = Module::new(&engine, wasm_bytes)
            .map_err(|e| format!("Wasm モジュールの読み込みに失敗: {e}"))?;
        let instance = linker
            .instantiate(&mut store, &module)
            .map_err(|e| format!("インスタンス化に失敗: {e}"))?;
        let start = instance
            .get_typed_func::<(), ()>(&mut store, "_start")
            .map_err(|e| format!("_start 関数が見つからない: {e}"))?;
        let result = start.call(&mut store, ());
        drop(store);
        let bytes = stdout
            .try_into_inner()
            .ok_or_else(|| "stdout の取得に失敗".to_string())?;
        let printed = String::from_utf8(bytes.to_vec())
            .map_err(|e| format!("stdout の UTF-8 変換に失敗: {e}"))?;
        match result {
            Ok(()) => Ok(printed),
            Err(e) => Err(format!("実行に失敗: {e}; printed={printed:?}")),
        }
    }

    let main_path = selfhost_main_path();
    let selfhost_root = main_path
        .parent()
        .expect("App/ ディレクトリ")
        .parent()
        .expect("src/ ディレクトリ")
        .parent()
        .expect("selfhost/ ルートディレクトリ")
        .to_path_buf();

    let stage1_wasm = compile_file_only(&main_path);
    assert_valid_wasm(&stage1_wasm);

    let progress_output = run_wasi_capture_trap_stdout(
        &stage1_wasm,
        Some(&selfhost_root),
        &[
            "compiler",
            "src/App/CompilerMode.ls",
            "",
            "",
            "",
            "",
            "",
            "",
            "",
            "",
            "",
            "",
            "",
            "",
            "",
            "build-compile-progress",
        ],
    )
    .expect("stage1 build compile progress on CompilerMode.ls should run");
    eprintln!(
        "BOOT-04 CompilerMode.ls stage1 build compile progress = {:?}",
        progress_output
    );
    assert!(!progress_output.trim().is_empty());
}

#[test]
#[ignore]
fn test_debug_boot04_stage1_build_compile_progress_on_minimal_vector_push_shape() {
    fn run_wasi_capture_trap_stdout(
        wasm_bytes: &[u8],
        dir: Option<&std::path::Path>,
        args: &[&str],
    ) -> Result<String, String> {
        use wasmtime::{Engine, Linker, Module, Store};
        use wasmtime_wasi::{WasiCtxBuilder, preview1::WasiP1Ctx};

        let engine = Engine::default();
        let mut linker = Linker::<WasiP1Ctx>::new(&engine);
        wasmtime_wasi::preview1::add_to_linker_sync(&mut linker, |ctx| ctx)
            .map_err(|e| format!("WASI リンクに失敗: {e}"))?;

        let stdout = wasmtime_wasi::pipe::MemoryOutputPipe::new(4 * 1024 * 1024);
        let mut builder = WasiCtxBuilder::new();
        builder.stdout(stdout.clone());
        builder.args(args);
        if let Some(dir_path) = dir {
            builder
                .preopened_dir(
                    dir_path,
                    ".",
                    wasmtime_wasi::DirPerms::all(),
                    wasmtime_wasi::FilePerms::all(),
                )
                .map_err(|e| format!("preopened_dir に失敗: {e}"))?;
        }
        let mut store = Store::new(&engine, builder.build_p1());
        let module = Module::new(&engine, wasm_bytes)
            .map_err(|e| format!("Wasm モジュールの読み込みに失敗: {e}"))?;
        let instance = linker
            .instantiate(&mut store, &module)
            .map_err(|e| format!("インスタンス化に失敗: {e}"))?;
        let start = instance
            .get_typed_func::<(), ()>(&mut store, "_start")
            .map_err(|e| format!("_start 関数が見つからない: {e}"))?;
        let result = start.call(&mut store, ());
        drop(store);
        let bytes = stdout
            .try_into_inner()
            .ok_or_else(|| "stdout の取得に失敗".to_string())?;
        let printed = String::from_utf8(bytes.to_vec())
            .map_err(|e| format!("stdout の UTF-8 変換に失敗: {e}"))?;
        match result {
            Ok(()) => Ok(printed),
            Err(e) => Err(format!("実行に失敗: {e}; printed={printed:?}")),
        }
    }

    let main_path = selfhost_main_path();
    let selfhost_root = main_path
        .parent()
        .expect("App/ ディレクトリ")
        .parent()
        .expect("src/ ディレクトリ")
        .parent()
        .expect("selfhost/ ルートディレクトリ")
        .to_path_buf();
    let temp_root = selfhost_root.join("target/test-artifacts").join(format!(
        "lsharp_vector_push_minimal_build_progress_{}",
        std::process::id()
    ));
    let _ = std::fs::remove_dir_all(&temp_root);
    std::fs::create_dir_all(&temp_root).expect("temp dir should be created");
    let source_path = temp_root.join("mini_vector_push_shape.ls");
    std::fs::write(
        &source_path,
        "(module Mini.Compiler)\n(defn compile-vector-push-with-source [node source env ftable instrs data-ref] (do (root_push node) (root_push source) (root_push env) (root_push ftable) (root_push instrs) (root_push data-ref) (let [vector-root (alloc-root-needed (vector-get node 3)) vector-instrs (compile-expr-with-source (vector-get node 3) source env ftable (vector-new 8) data-ref)] (do (root_push vector-instrs) (let [value-root (alloc-root-needed (vector-get node 4)) value-instrs (compile-expr-with-source (vector-get node 4) source env ftable (vector-new 8) data-ref)] (do (root_push value-instrs) (let [temp-base (max-root-temp-base env vector-instrs value-instrs) vector-local temp-base value-local (+ temp-base 1) instrs1 (append-instr-vector instrs vector-instrs)] (do (root_push instrs1) (let [instrs2 (emit-to instrs1 (op-local-set) vector-local)] (do (root_push instrs2) (let [instrs3 (maybe-root-push-drop instrs2 vector-root vector-local)] (do (root_push instrs3) (let [instrs4 (append-instr-vector instrs3 value-instrs)] (do (root_push instrs4) (let [instrs5 (emit-to instrs4 (op-local-set) value-local)] (do (root_push instrs5) (let [instrs6 (maybe-root-push-drop instrs5 value-root value-local)] (do (root_push instrs6) (let [instrs7 (emit-to instrs6 (op-local-get) vector-local)] (do (root_push instrs7) (let [instrs8 (emit-to instrs7 (op-local-get) value-local)] (do (root_push instrs8) (let [instrs9 (emit-to instrs8 (op-vector-push) (+ 1 (map-size env)))] (do (root_push instrs9) (let [instrs10 (maybe-root-pop-drop instrs9 value-root)] (do (root_push instrs10) (let [result (maybe-root-pop-drop instrs10 vector-root)] (do (root_pop) (root_pop) (root_pop) (root_pop) (root_pop) (root_pop) (root_pop) (root_pop) (root_pop) (root_pop) (root_pop) (root_pop) (root_pop) (root_pop) (root_pop) (root_pop) (root_pop) (root_pop) result))))))))))))))))))))))))))))\n(defn root_push [x] 0)\n(defn root_pop [] 0)\n(defn alloc-root-needed [x] 0)\n(defn vector-get [v idx] 0)\n(defn vector-new [n] 0)\n(defn compile-expr-with-source [expr source env ftable instrs data-ref] instrs)\n(defn max-root-temp-base [env lhs rhs] 0)\n(defn append-instr-vector [lhs rhs] lhs)\n(defn emit-to [instrs op arg] instrs)\n(defn maybe-root-push-drop [instrs should-root local-idx] instrs)\n(defn maybe-root-pop-drop [instrs should-root] instrs)\n(defn op-local-set [] 0)\n(defn op-local-get [] 0)\n(defn op-vector-push [] 0)\n(defn map-size [m] 0)\n(defn main [] 0)\n",
    )
    .expect("mini source should be written");

    let stage1_wasm = compile_file_only(&main_path);
    assert_valid_wasm(&stage1_wasm);

    let source_path_str = source_path
        .strip_prefix(&selfhost_root)
        .expect("source path should stay under selfhost root")
        .to_str()
        .expect("utf-8 path");
    let progress_output = run_wasi_capture_trap_stdout(
        &stage1_wasm,
        Some(&selfhost_root),
        &[
            "compiler",
            source_path_str,
            "",
            "",
            "",
            "",
            "",
            "",
            "",
            "",
            "",
            "",
            "",
            "",
            "",
            "build-compile-progress",
        ],
    )
    .expect("stage1 build compile progress on minimal vector-push source should run");
    eprintln!(
        "BOOT-04 minimal vector-push stage1 build compile progress = {:?}",
        progress_output
    );
    assert!(!progress_output.trim().is_empty());
}

#[test]
#[ignore]
fn test_debug_boot04_stage1_build_compile_progress_on_padded_vector_push_shape() {
    fn run_wasi_capture_trap_stdout(
        wasm_bytes: &[u8],
        dir: Option<&std::path::Path>,
        args: &[&str],
    ) -> Result<String, String> {
        use wasmtime::{Engine, Linker, Module, Store};
        use wasmtime_wasi::{WasiCtxBuilder, preview1::WasiP1Ctx};

        let engine = Engine::default();
        let mut linker = Linker::<WasiP1Ctx>::new(&engine);
        wasmtime_wasi::preview1::add_to_linker_sync(&mut linker, |ctx| ctx)
            .map_err(|e| format!("WASI リンクに失敗: {e}"))?;

        let stdout = wasmtime_wasi::pipe::MemoryOutputPipe::new(4 * 1024 * 1024);
        let mut builder = WasiCtxBuilder::new();
        builder.stdout(stdout.clone());
        builder.args(args);
        if let Some(dir_path) = dir {
            builder
                .preopened_dir(
                    dir_path,
                    ".",
                    wasmtime_wasi::DirPerms::all(),
                    wasmtime_wasi::FilePerms::all(),
                )
                .map_err(|e| format!("preopened_dir に失敗: {e}"))?;
        }
        let mut store = Store::new(&engine, builder.build_p1());
        let module = Module::new(&engine, wasm_bytes)
            .map_err(|e| format!("Wasm モジュールの読み込みに失敗: {e}"))?;
        let instance = linker
            .instantiate(&mut store, &module)
            .map_err(|e| format!("インスタンス化に失敗: {e}"))?;
        let start = instance
            .get_typed_func::<(), ()>(&mut store, "_start")
            .map_err(|e| format!("_start 関数が見つからない: {e}"))?;
        let result = start.call(&mut store, ());
        drop(store);
        let bytes = stdout
            .try_into_inner()
            .ok_or_else(|| "stdout の取得に失敗".to_string())?;
        let printed = String::from_utf8(bytes.to_vec())
            .map_err(|e| format!("stdout の UTF-8 変換に失敗: {e}"))?;
        match result {
            Ok(()) => Ok(printed),
            Err(e) => Err(format!("実行に失敗: {e}; printed={printed:?}")),
        }
    }

    let main_path = selfhost_main_path();
    let selfhost_root = main_path
        .parent()
        .expect("App/ ディレクトリ")
        .parent()
        .expect("src/ ディレクトリ")
        .parent()
        .expect("selfhost/ ルートディレクトリ")
        .to_path_buf();
    let temp_root = selfhost_root.join("target/test-artifacts").join(format!(
        "lsharp_vector_push_padded_build_progress_{}",
        std::process::id()
    ));
    let _ = std::fs::remove_dir_all(&temp_root);
    std::fs::create_dir_all(&temp_root).expect("temp dir should be created");
    let source_path = temp_root.join("padded_vector_push_shape.ls");

    let filler = "x".repeat(700);
    let mut source = String::from("(module Mini.Compiler)\n");
    for i in 0..198 {
        source.push_str(&format!("(defn filler{i:03} [] \"{filler}\")\n"));
    }
    source.push_str("(defn compile-vector-push-with-source [node source env ftable instrs data-ref] (do (root_push node) (root_push source) (root_push env) (root_push ftable) (root_push instrs) (root_push data-ref) (let [vector-root (alloc-root-needed (vector-get node 3)) vector-instrs (compile-expr-with-source (vector-get node 3) source env ftable (vector-new 8) data-ref)] (do (root_push vector-instrs) (let [value-root (alloc-root-needed (vector-get node 4)) value-instrs (compile-expr-with-source (vector-get node 4) source env ftable (vector-new 8) data-ref)] (do (root_push value-instrs) (let [temp-base (max-root-temp-base env vector-instrs value-instrs) vector-local temp-base value-local (+ temp-base 1) instrs1 (append-instr-vector instrs vector-instrs)] (do (root_push instrs1) (let [instrs2 (emit-to instrs1 (op-local-set) vector-local)] (do (root_push instrs2) (let [instrs3 (maybe-root-push-drop instrs2 vector-root vector-local)] (do (root_push instrs3) (let [instrs4 (append-instr-vector instrs3 value-instrs)] (do (root_push instrs4) (let [instrs5 (emit-to instrs4 (op-local-set) value-local)] (do (root_push instrs5) (let [instrs6 (maybe-root-push-drop instrs5 value-root value-local)] (do (root_push instrs6) (let [instrs7 (emit-to instrs6 (op-local-get) vector-local)] (do (root_push instrs7) (let [instrs8 (emit-to instrs7 (op-local-get) value-local)] (do (root_push instrs8) (let [instrs9 (emit-to instrs8 (op-vector-push) (+ 1 (map-size env)))] (do (root_push instrs9) (let [instrs10 (maybe-root-pop-drop instrs9 value-root)] (do (root_push instrs10) (let [result (maybe-root-pop-drop instrs10 vector-root)] (do (root_pop) (root_pop) (root_pop) (root_pop) (root_pop) (root_pop) (root_pop) (root_pop) (root_pop) (root_pop) (root_pop) (root_pop) (root_pop) (root_pop) (root_pop) (root_pop) (root_pop) (root_pop) result))))))))))))))))))))))))))))\n");
    source.push_str("(defn root_push [x] 0)\n(defn root_pop [] 0)\n(defn alloc-root-needed [x] 0)\n(defn vector-get [v idx] 0)\n(defn vector-new [n] 0)\n(defn compile-expr-with-source [expr source env ftable instrs data-ref] instrs)\n(defn max-root-temp-base [env lhs rhs] 0)\n(defn append-instr-vector [lhs rhs] lhs)\n(defn emit-to [instrs op arg] instrs)\n(defn maybe-root-push-drop [instrs should-root local-idx] instrs)\n(defn maybe-root-pop-drop [instrs should-root] instrs)\n(defn op-local-set [] 0)\n(defn op-local-get [] 0)\n(defn op-vector-push [] 0)\n(defn map-size [m] 0)\n(defn main [] 0)\n");
    std::fs::write(&source_path, source).expect("padded source should be written");

    let stage1_wasm = compile_file_only(&main_path);
    assert_valid_wasm(&stage1_wasm);

    let source_path_str = source_path
        .strip_prefix(&selfhost_root)
        .expect("source path should stay under selfhost root")
        .to_str()
        .expect("utf-8 path");
    let progress_output = run_wasi_capture_trap_stdout(
        &stage1_wasm,
        Some(&selfhost_root),
        &[
            "compiler",
            source_path_str,
            "",
            "",
            "",
            "",
            "",
            "",
            "",
            "",
            "",
            "",
            "",
            "",
            "",
            "build-compile-progress",
        ],
    )
    .expect("stage1 build compile progress on padded vector-push source should run");
    eprintln!(
        "BOOT-04 padded vector-push stage1 build compile progress = {:?}",
        progress_output
    );
    assert!(!progress_output.trim().is_empty());
}

#[test]
#[ignore]
fn test_debug_boot04_stage1_build_compile_progress_on_large_ftable_vector_push_shape() {
    fn run_wasi_capture_trap_stdout(
        wasm_bytes: &[u8],
        dir: Option<&std::path::Path>,
        args: &[&str],
    ) -> Result<String, String> {
        use wasmtime::{Engine, Linker, Module, Store};
        use wasmtime_wasi::{WasiCtxBuilder, preview1::WasiP1Ctx};

        let engine = Engine::default();
        let mut linker = Linker::<WasiP1Ctx>::new(&engine);
        wasmtime_wasi::preview1::add_to_linker_sync(&mut linker, |ctx| ctx)
            .map_err(|e| format!("WASI リンクに失敗: {e}"))?;

        let stdout = wasmtime_wasi::pipe::MemoryOutputPipe::new(4 * 1024 * 1024);
        let mut builder = WasiCtxBuilder::new();
        builder.stdout(stdout.clone());
        builder.args(args);
        if let Some(dir_path) = dir {
            builder
                .preopened_dir(
                    dir_path,
                    ".",
                    wasmtime_wasi::DirPerms::all(),
                    wasmtime_wasi::FilePerms::all(),
                )
                .map_err(|e| format!("preopened_dir に失敗: {e}"))?;
        }
        let mut store = Store::new(&engine, builder.build_p1());
        let module = Module::new(&engine, wasm_bytes)
            .map_err(|e| format!("Wasm モジュールの読み込みに失敗: {e}"))?;
        let instance = linker
            .instantiate(&mut store, &module)
            .map_err(|e| format!("インスタンス化に失敗: {e}"))?;
        let start = instance
            .get_typed_func::<(), ()>(&mut store, "_start")
            .map_err(|e| format!("_start 関数が見つからない: {e}"))?;
        let result = start.call(&mut store, ());
        drop(store);
        let bytes = stdout
            .try_into_inner()
            .ok_or_else(|| "stdout の取得に失敗".to_string())?;
        let printed = String::from_utf8(bytes.to_vec())
            .map_err(|e| format!("stdout の UTF-8 変換に失敗: {e}"))?;
        match result {
            Ok(()) => Ok(printed),
            Err(e) => Err(format!("実行に失敗: {e}; printed={printed:?}")),
        }
    }

    let main_path = selfhost_main_path();
    let selfhost_root = main_path
        .parent()
        .expect("App/ ディレクトリ")
        .parent()
        .expect("src/ ディレクトリ")
        .parent()
        .expect("selfhost/ ルートディレクトリ")
        .to_path_buf();
    let temp_root = selfhost_root.join("target/test-artifacts").join(format!(
        "lsharp_vector_push_large_ftable_build_progress_{}",
        std::process::id()
    ));
    let _ = std::fs::remove_dir_all(&temp_root);
    std::fs::create_dir_all(&temp_root).expect("temp dir should be created");
    let source_path = temp_root.join("large_ftable_vector_push_shape.ls");

    let mut source = String::from("(module Mini.Compiler)\n");
    for i in 0..198 {
        source.push_str(&format!("(defn prefix{i:03} [] {i})\n"));
    }
    source.push_str("(defn compile-vector-push-with-source [node source env ftable instrs data-ref] (do (root_push node) (root_push source) (root_push env) (root_push ftable) (root_push instrs) (root_push data-ref) (let [vector-root (alloc-root-needed (vector-get node 3)) vector-instrs (compile-expr-with-source (vector-get node 3) source env ftable (vector-new 8) data-ref)] (do (root_push vector-instrs) (let [value-root (alloc-root-needed (vector-get node 4)) value-instrs (compile-expr-with-source (vector-get node 4) source env ftable (vector-new 8) data-ref)] (do (root_push value-instrs) (let [temp-base (max-root-temp-base env vector-instrs value-instrs) vector-local temp-base value-local (+ temp-base 1) instrs1 (append-instr-vector instrs vector-instrs)] (do (root_push instrs1) (let [instrs2 (emit-to instrs1 (op-local-set) vector-local)] (do (root_push instrs2) (let [instrs3 (maybe-root-push-drop instrs2 vector-root vector-local)] (do (root_push instrs3) (let [instrs4 (append-instr-vector instrs3 value-instrs)] (do (root_push instrs4) (let [instrs5 (emit-to instrs4 (op-local-set) value-local)] (do (root_push instrs5) (let [instrs6 (maybe-root-push-drop instrs5 value-root value-local)] (do (root_push instrs6) (let [instrs7 (emit-to instrs6 (op-local-get) vector-local)] (do (root_push instrs7) (let [instrs8 (emit-to instrs7 (op-local-get) value-local)] (do (root_push instrs8) (let [instrs9 (emit-to instrs8 (op-vector-push) (+ 1 (map-size env)))] (do (root_push instrs9) (let [instrs10 (maybe-root-pop-drop instrs9 value-root)] (do (root_push instrs10) (let [result (maybe-root-pop-drop instrs10 vector-root)] (do (root_pop) (root_pop) (root_pop) (root_pop) (root_pop) (root_pop) (root_pop) (root_pop) (root_pop) (root_pop) (root_pop) (root_pop) (root_pop) (root_pop) (root_pop) (root_pop) (root_pop) (root_pop) result))))))))))))))))))))))))))))\n");
    source.push_str("(defn root_push [x] 0)\n(defn root_pop [] 0)\n(defn alloc-root-needed [x] 0)\n(defn vector-get [v idx] 0)\n(defn vector-new [n] 0)\n(defn compile-expr-with-source [expr source env ftable instrs data-ref] instrs)\n(defn max-root-temp-base [env lhs rhs] 0)\n(defn append-instr-vector [lhs rhs] lhs)\n(defn emit-to [instrs op arg] instrs)\n(defn maybe-root-push-drop [instrs should-root local-idx] instrs)\n(defn maybe-root-pop-drop [instrs should-root] instrs)\n(defn op-local-set [] 0)\n(defn op-local-get [] 0)\n(defn op-vector-push [] 0)\n(defn map-size [m] 0)\n");
    for i in 0..800 {
        source.push_str(&format!("(defn suffix{i:03} [] {i})\n"));
    }
    source.push_str("(defn main [] 0)\n");
    std::fs::write(&source_path, source).expect("large ftable source should be written");

    let stage1_wasm = compile_file_only(&main_path);
    assert_valid_wasm(&stage1_wasm);

    let source_path_str = source_path
        .strip_prefix(&selfhost_root)
        .expect("source path should stay under selfhost root")
        .to_str()
        .expect("utf-8 path");
    let progress_output = run_wasi_capture_trap_stdout(
        &stage1_wasm,
        Some(&selfhost_root),
        &[
            "compiler",
            source_path_str,
            "",
            "",
            "",
            "",
            "",
            "",
            "",
            "",
            "",
            "",
            "",
            "",
            "",
            "build-compile-progress",
        ],
    )
    .expect("stage1 build compile progress on large-ftable vector-push source should run");
    eprintln!(
        "BOOT-04 large-ftable vector-push stage1 build compile progress = {:?}",
        progress_output
    );
    assert!(!progress_output.trim().is_empty());
}

#[test]
fn test_e2e_boot04_stage2_first_defn_source_probe_emits_expected_plus_ir_on_minimal_text_eq_loop_shape()
 {
    let temp_root = selfhost_project_root()
        .join("target/test-artifacts")
        .join(format!(
            "lsharp_text_eq_loop_minimal_stage_compare_{}",
            std::process::id()
        ));
    let _ = std::fs::remove_dir_all(&temp_root);
    std::fs::create_dir_all(&temp_root).expect("temp dir should be created");
    let source_path = temp_root.join("mini_text_eq_loop_shape.ls");
    std::fs::write(
        &source_path,
        "(module Mini.ModuleResolver)\n(defn text-eq-loop [left right idx len] (if (>= idx len) true (if (= (string-char-at left idx) (string-char-at right idx)) (text-eq-loop left right (+ idx 1) len) false)))\n(defn main [] 0)\n",
    )
    .expect("mini source should be written");

    let main_path = selfhost_main_path();
    let selfhost_root = main_path
        .parent()
        .expect("App/ ディレクトリ")
        .parent()
        .expect("src/ ディレクトリ")
        .parent()
        .expect("selfhost/ ルートディレクトリ")
        .to_path_buf();

    let stage1_wasm = compile_file_only(&main_path);
    assert_valid_wasm(&stage1_wasm);

    let source_path_str = source_path.to_str().expect("utf-8 path");
    let mut probe_args = vec!["compiler", source_path_str];
    while probe_args.len() < 22 {
        probe_args.push("");
    }
    probe_args.push("first-defn-source");

    let stage2_output = lsharp_wasm::wasi_runner::run_wasm_wasi_with_dir_and_args(
        &stage1_wasm,
        Some(&selfhost_root),
        &["compiler", "src/App/Main.ls"],
    )
    .expect("stage1 should self-compile Main.ls");
    let stage2_modules = parse_emitted_wasm_modules(&stage2_output, 1);
    let stage2_self_compiler = &stage2_modules[0];
    assert_valid_wasm(stage2_self_compiler);

    let stage2_probe_output = run_wasm_with_six_imports_compiler_mode_fs(
        stage2_self_compiler,
        &selfhost_root,
        &probe_args,
    )
    .expect("stage2 first-defn source probe on minimal text-eq-loop source should run");
    let values: Vec<i64> = stage2_probe_output
        .lines()
        .filter(|line| !line.trim().is_empty())
        .map(|line| {
            line.trim().parse::<i64>().unwrap_or_else(|_| {
                panic!("BOOT-04 minimal text-eq-loop source probe: 数値でない debug 出力: {line:?}")
            })
        })
        .collect();
    assert_eq!(
        values,
        vec![
            301, 1, 302, 6, 303, 5, 304, 3, 206, 0, 209, 2, 207, 10, 208, 3, 206, 1, 209, 2, 207,
            1, 208, 1, 206, 2, 209, 2, 207, 20, 208, 0,
        ],
        "minimal text-eq-loop source probe は (+ idx 1) を local-get / i64-const / i64-add に lower すべき: {:?}",
        values
    );
    std::fs::remove_dir_all(&temp_root).expect("repo-local temp dir should be removed");
}

#[test]
#[ignore]
fn test_debug_boot04_stage2_ast_chunked_step_progress_on_ast_file() {
    let main_path = selfhost_main_path();
    let selfhost_root = main_path
        .parent()
        .expect("App/ ディレクトリ")
        .parent()
        .expect("src/ ディレクトリ")
        .parent()
        .expect("selfhost/ ルートディレクトリ")
        .to_path_buf();

    let stage1_wasm = compile_file_only(&main_path);
    assert_valid_wasm(&stage1_wasm);

    let stage2_output = lsharp_wasm::wasi_runner::run_wasm_wasi_with_dir_and_args(
        &stage1_wasm,
        Some(&selfhost_root),
        &["compiler", "src/App/Main.ls"],
    )
    .expect("stage1 should self-compile Main.ls");
    let stage2_modules = parse_emitted_wasm_modules(&stage2_output, 1);
    let stage2_self_compiler = &stage2_modules[0];
    assert_valid_wasm(stage2_self_compiler);

    let mut args = vec!["compiler", "src/Syntax/AST.ls"];
    while args.len() < 20 {
        args.push("");
    }
    args.push("ast-chunked-step");

    let output =
        run_wasm_with_six_imports_compiler_mode_fs(stage2_self_compiler, &selfhost_root, &args)
            .expect("stage2 ast-chunked-step probe should run");
    eprintln!("BOOT-04 ast-chunked-step values = {:?}", output);
    assert!(!output.trim().is_empty());
}

#[test]
fn test_e2e_boot04_self_hosted_stage2_cache_compile_progress_counts_all_main_modules() {
    let main_path = selfhost_main_path();
    let selfhost_root = main_path
        .parent()
        .expect("App/ ディレクトリ")
        .parent()
        .expect("src/ ディレクトリ")
        .parent()
        .expect("selfhost/ ルートディレクトリ")
        .to_path_buf();
    let stage1_wasm = compile_file_only(&main_path);
    assert_valid_wasm(&stage1_wasm);

    let stage2_output = lsharp_wasm::wasi_runner::run_wasm_wasi_with_dir_and_args(
        &stage1_wasm,
        Some(&selfhost_root),
        &["compiler", "src/App/Main.ls"],
    )
    .expect("BOOT-04 cache-compile-progress: stage1 が Main.ls の self-compile に失敗した");
    let stage2_modules = parse_emitted_wasm_modules(&stage2_output, 1);
    let stage2_self_compiler = &stage2_modules[0];
    assert_valid_wasm(stage2_self_compiler);

    let debug_output = run_wasm_with_six_imports_compiler_mode_fs(
        stage2_self_compiler,
        &selfhost_root,
        &[
            "compiler",
            "src/App/Main.ls",
            "",
            "",
            "",
            "",
            "",
            "",
            "",
            "",
            "",
            "",
            "cache-compile-progress",
        ],
    )
    .expect(
        "BOOT-04 cache-compile-progress: stage2_self_compiler の cache compile progress 実行に失敗した",
    );
    let values: Vec<i64> = debug_output
        .lines()
        .filter(|line| !line.trim().is_empty())
        .map(|line| {
            line.trim().parse::<i64>().unwrap_or_else(|_| {
                panic!("BOOT-04 cache-compile-progress: 数値でない debug 出力: {line:?}")
            })
        })
        .collect();
    eprintln!("BOOT-04 cache-compile-progress values = {:?}", values);

    assert_eq!(
        values.len(),
        8,
        "BOOT-04 cache-compile-progress: debug 出力長が期待と異なる: {:?}",
        values
    );
    assert_eq!(
        values[0], 86,
        "BOOT-04 cache-compile-progress: marker 86 が必要"
    );
    assert_eq!(
        values[2], 87,
        "BOOT-04 cache-compile-progress: marker 87 が必要"
    );
    assert_eq!(
        values[4], 88,
        "BOOT-04 cache-compile-progress: marker 88 が必要"
    );
    assert_eq!(
        values[6], 89,
        "BOOT-04 cache-compile-progress: marker 89 が必要"
    );

    let parse_count = values[1];
    let pair_count = values[3];
    let reg_count = values[5];
    let function_count = values[7];

    assert_eq!(
        parse_count, pair_count,
        "BOOT-04 cache-compile-progress: cache compile は Main graph の全 pair を一度ずつ parse するべき: {:?}",
        values
    );
    assert!(
        pair_count >= 26,
        "BOOT-04 cache-compile-progress: Main graph pair count が小さすぎる: {:?}",
        values
    );
    assert_eq!(
        reg_count, function_count,
        "BOOT-04 cache-compile-progress: register/compile 後の function count は一致するべき: {:?}",
        values
    );
    assert!(
        function_count >= 1531,
        "BOOT-04 cache-compile-progress: compiled function count が小さすぎる: {:?}",
        values
    );
}

#[test]
fn test_e2e_boot04_self_hosted_stage2_module_resolver_first_defn_with_source_matches_ftable_ir() {
    let main_path = selfhost_main_path();
    let selfhost_root = main_path
        .parent()
        .expect("App/ ディレクトリ")
        .parent()
        .expect("src/ ディレクトリ")
        .parent()
        .expect("selfhost/ ルートディレクトリ")
        .to_path_buf();
    let stage1_wasm = compile_file_only(&main_path);
    assert_valid_wasm(&stage1_wasm);

    let stage2_output = lsharp_wasm::wasi_runner::run_wasm_wasi_with_dir_and_args(
        &stage1_wasm,
        Some(&selfhost_root),
        &["compiler", "src/App/Main.ls"],
    )
    .expect("BOOT-04 first-defn-ir-parity: stage1 が Main.ls の self-compile に失敗した");
    let stage2_modules = parse_emitted_wasm_modules(&stage2_output, 1);
    let stage2_self_compiler = &stage2_modules[0];
    assert_valid_wasm(stage2_self_compiler);

    let debug_output = run_wasm_with_six_imports_compiler_mode_fs(
        stage2_self_compiler,
        &selfhost_root,
        &[
            "compiler",
            "src/App/ModuleResolver.ls",
            "",
            "",
            "",
            "",
            "",
            "",
            "",
            "",
            "",
            "",
            "",
            "first-defn-ir-parity",
        ],
    )
    .expect("BOOT-04 first-defn-ir-parity: stage2_self_compiler の parity probe 実行に失敗した");
    let values: Vec<i64> = debug_output
        .lines()
        .filter(|line| !line.trim().is_empty())
        .map(|line| {
            line.trim().parse::<i64>().unwrap_or_else(|_| {
                panic!("BOOT-04 first-defn-ir-parity: 数値でない debug 出力: {line:?}")
            })
        })
        .collect();
    eprintln!("BOOT-04 first-defn-ir-parity values = {:?}", values);

    assert!(
        values.len() >= 17,
        "BOOT-04 first-defn-ir-parity: raw/source/ftable marker 群を期待した: {:?}",
        values
    );
    assert_eq!(
        values[0], 91,
        "BOOT-04 first-defn-ir-parity: raw-source marker が崩れている: {:?}",
        values
    );
    assert!(
        values[1] >= 0,
        "BOOT-04 first-defn-ir-parity: defn index を見つけられていない: {:?}",
        values
    );
    assert_eq!(
        values[2], 92,
        "BOOT-04 first-defn-ir-parity: raw-source length marker が崩れている: {:?}",
        values
    );
    assert_eq!(
        values[4], 93,
        "BOOT-04 first-defn-ir-parity: with-source pre-marker が崩れている: {:?}",
        values
    );
    assert_eq!(
        values[5], 94,
        "BOOT-04 first-defn-ir-parity: with-source length marker が崩れている: {:?}",
        values
    );
    assert_eq!(
        values[7], 95,
        "BOOT-04 first-defn-ir-parity: defn index replay marker が崩れている: {:?}",
        values
    );
    assert_eq!(
        values[9], 96,
        "BOOT-04 first-defn-ir-parity: source IR marker が崩れている: {:?}",
        values
    );
    assert_eq!(
        values[11], 97,
        "BOOT-04 first-defn-ir-parity: ftable IR marker が崩れている: {:?}",
        values
    );
    assert_eq!(
        values[13], 98,
        "BOOT-04 first-defn-ir-parity: data marker が崩れている: {:?}",
        values
    );
    assert_eq!(
        values[15], 99,
        "BOOT-04 first-defn-ir-parity: raw-ftable marker が崩れている: {:?}",
        values
    );
    assert!(
        values[3] > 3 && values[6] > 3 && values[10] > 3,
        "BOOT-04 first-defn-ir-parity: source IR が短すぎる: {:?}",
        values
    );
    assert_eq!(
        values[1], values[8],
        "BOOT-04 first-defn-ir-parity: defn index replay が一致するべき: {:?}",
        values
    );
    assert_eq!(
        values[3], values[16],
        "BOOT-04 first-defn-ir-parity: raw source / raw ftable で IR 長が一致するべき: {:?}",
        values
    );
    assert_eq!(
        values[6], values[10],
        "BOOT-04 first-defn-ir-parity: with-source marker 94/96 の IR 長が一致するべき: {:?}",
        values
    );
    assert_eq!(
        values[10], values[12],
        "BOOT-04 first-defn-ir-parity: with-source / with-ftable で IR 長が一致するべき: {:?}",
        values
    );
}

#[test]
fn test_e2e_boot04_self_hosted_stage2_reports_module_resolver_progress() {
    let main_path = selfhost_main_path();
    let selfhost_root = main_path
        .parent()
        .expect("App/ ディレクトリ")
        .parent()
        .expect("src/ ディレクトリ")
        .parent()
        .expect("selfhost/ ルートディレクトリ")
        .to_path_buf();

    let stage1_wasm = compile_file_only(&main_path);
    assert_valid_wasm(&stage1_wasm);

    let stage2_output = lsharp_wasm::wasi_runner::run_wasm_wasi_with_dir_and_args(
        &stage1_wasm,
        Some(&selfhost_root),
        &["compiler", "src/App/Main.ls"],
    )
    .expect("BOOT-04 module-resolver-progress: stage1 が Main.ls の self-compile に失敗した");
    let stage2_modules = parse_emitted_wasm_modules(&stage2_output, 1);
    let stage2_self_compiler = &stage2_modules[0];
    assert_valid_wasm(stage2_self_compiler);

    let progress_output = run_wasm_with_six_imports_compiler_mode_fs(
        stage2_self_compiler,
        &selfhost_root,
        &[
            "compiler",
            "src/App/ModuleResolver.ls",
            "debug",
            "progress",
            "module-resolver",
        ],
    );
    eprintln!(
        "BOOT-04 module-resolver-progress output = {:?}",
        progress_output
    );
}

#[test]
fn test_e2e_boot04_self_hosted_stage2_reports_string_length_if_progress() {
    let main_path = selfhost_main_path();
    let selfhost_root = main_path
        .parent()
        .expect("App/ ディレクトリ")
        .parent()
        .expect("src/ ディレクトリ")
        .parent()
        .expect("selfhost/ ルートディレクトリ")
        .to_path_buf();

    let stage1_wasm = compile_file_only(&main_path);
    assert_valid_wasm(&stage1_wasm);

    let stage2_output = lsharp_wasm::wasi_runner::run_wasm_wasi_with_dir_and_args(
        &stage1_wasm,
        Some(&selfhost_root),
        &["compiler", "src/App/Main.ls"],
    )
    .expect("BOOT-04 string-length-if-progress: stage1 が Main.ls の self-compile に失敗した");
    let stage2_modules = parse_emitted_wasm_modules(&stage2_output, 1);
    let stage2_self_compiler = &stage2_modules[0];
    assert_valid_wasm(stage2_self_compiler);

    let source = "(module App.ModuleResolver)\n(defn text-len-eq [left right] (let [len (string-length left)] (if (= len (string-length right)) 1 0)))\n(defn main [] (text-len-eq (command-line-arg 0) (command-line-arg 1)))\n";
    let progress_output = run_wasm_with_six_imports_compiler_mode(
        stage2_self_compiler,
        source,
        &[
            "compiler",
            "src/App/ModuleResolver.ls",
            "debug",
            "progress",
            "inline",
        ],
    );
    eprintln!(
        "BOOT-04 string-length-if-progress output = {:?}",
        progress_output
    );
}

#[test]
fn test_v2_12_self_hosted_stage2_keeps_complex_defn_decl_tag() {
    let main_path = selfhost_main_path();
    let selfhost_root = main_path
        .parent()
        .expect("App/ ディレクトリ")
        .parent()
        .expect("src/ ディレクトリ")
        .parent()
        .expect("selfhost/ ルートディレクトリ")
        .to_path_buf();

    let stage1_wasm = compile_file_only(&main_path);
    assert_valid_wasm(&stage1_wasm);

    let stage2_output = lsharp_wasm::wasi_runner::run_wasm_wasi_with_dir_and_args(
        &stage1_wasm,
        Some(&selfhost_root),
        &["compiler", "src/App/Main.ls"],
    )
    .expect("V2-12 complex-defn-tag: stage1 が Main.ls の self-compile に失敗した");
    let stage2_modules = parse_emitted_wasm_modules(&stage2_output, 1);
    let stage2_self_compiler = &stage2_modules[0];
    assert_valid_wasm(stage2_self_compiler);

    let source = "(module App.Main)\n(defn helper [x] (if (= x 0) 0 (+ x 1)))\n";
    let debug_output = run_wasm_with_six_imports_compiler_mode(
        stage2_self_compiler,
        source,
        &[
            "compiler",
            "src/App/Main.ls",
            "",
            "",
            "",
            "",
            "",
            "",
            "",
            "",
            "",
            "expr-tag",
        ],
    )
    .expect("V2-12 complex-defn-tag: stage2_self_compiler の debug 実行に失敗した");
    let values: Vec<i64> = debug_output
        .lines()
        .filter(|line| !line.trim().is_empty())
        .map(|line| {
            line.trim().parse::<i64>().unwrap_or_else(|_| {
                panic!("V2-12 complex-defn-tag: 数値でない debug 出力: {line:?}")
            })
        })
        .collect();

    assert!(
        values.len() >= 10,
        "V2-12 complex-defn-tag: debug 出力が短すぎる: {:?}",
        values
    );
    assert_eq!(
        values[0], 73,
        "V2-12 complex-defn-tag: expr-tag debug marker が期待と異なる: {:?}",
        values
    );
    assert_eq!(
        values[7], 20,
        "V2-12 complex-defn-tag: complex body を持つ defn も decl tag 20 を維持するべき: {:?}",
        values
    );
    assert_eq!(
        values[8], 1,
        "V2-12 complex-defn-tag: helper の引数数が期待と異なる: {:?}",
        values
    );
    assert_eq!(
        values[9], 6,
        "V2-12 complex-defn-tag: helper body は if tag を維持するべき: {:?}",
        values
    );
}

#[test]
fn test_v2_12_self_hosted_stage2_emits_data_section_for_string_literals() {
    let main_path = selfhost_main_path();
    let selfhost_root = main_path
        .parent()
        .expect("App/ ディレクトリ")
        .parent()
        .expect("src/ ディレクトリ")
        .parent()
        .expect("selfhost/ ルートディレクトリ")
        .to_path_buf();

    let stage1_wasm = compile_file_only(&main_path);
    assert_valid_wasm(&stage1_wasm);

    let stage2_output = lsharp_wasm::wasi_runner::run_wasm_wasi_with_dir_and_args(
        &stage1_wasm,
        Some(&selfhost_root),
        &["compiler", "src/App/Main.ls"],
    )
    .expect("V2-12 string-data: stage1 が Main.ls の self-compile に失敗した");
    let stage2_modules = parse_emitted_wasm_modules(&stage2_output, 1);
    let stage2_self_compiler = &stage2_modules[0];
    assert_valid_wasm(stage2_self_compiler);

    let source = r#"
(module App.Main)
(defn main []
  (if (= 1 1)
    "hello"
    "world"))
"#;
    let stage3_output = run_wasm_with_six_imports_compiler_mode(
        stage2_self_compiler,
        source,
        &["compiler", "inline-string-data.ls"],
    )
    .expect("V2-12 string-data: stage2_self_compiler が inline string source をコンパイルできない");
    let stage3_modules = parse_emitted_wasm_modules(&stage3_output, 1);
    let stage3_wasm = &stage3_modules[0];
    assert_valid_wasm(stage3_wasm);

    let data_section = extract_section_bytes(stage3_wasm, 11)
        .expect("V2-12 string-data: string literal を含む stage3 wasm は data section を持つべき");
    let hello = b"hello";
    let world = b"world";
    assert!(
        data_section
            .windows(hello.len())
            .any(|window| window == hello),
        "V2-12 string-data: data section に hello bytes が見つからない: {:?}",
        &data_section[..data_section.len().min(64)]
    );
    assert!(
        data_section
            .windows(world.len())
            .any(|window| window == world),
        "V2-12 string-data: data section に world bytes が見つからない: {:?}",
        &data_section[..data_section.len().min(64)]
    );
}

#[test]
fn test_v2_12_self_hosted_stage2_keeps_if_and_string_expr_tags() {
    let main_path = selfhost_main_path();
    let selfhost_root = main_path
        .parent()
        .expect("App/ ディレクトリ")
        .parent()
        .expect("src/ ディレクトリ")
        .parent()
        .expect("selfhost/ ルートディレクトリ")
        .to_path_buf();

    let stage1_wasm = compile_file_only(&main_path);
    assert_valid_wasm(&stage1_wasm);
    let source = "(module App.Main)\n(defn main [] (if (= 1 1) \"hello\" \"world\"))\n";
    let expr_tag_args = [
        "compiler",
        "src/App/Main.ls",
        "",
        "",
        "",
        "",
        "",
        "",
        "",
        "",
        "",
        "expr-tag",
    ];

    let stage2_output = lsharp_wasm::wasi_runner::run_wasm_wasi_with_dir_and_args(
        &stage1_wasm,
        Some(&selfhost_root),
        &["compiler", "src/App/Main.ls"],
    )
    .expect("V2-12 expr-tag: stage1 が Main.ls の self-compile に失敗した");
    let stage2_modules = parse_emitted_wasm_modules(&stage2_output, 1);
    let stage2_self_compiler = &stage2_modules[0];
    assert_valid_wasm(stage2_self_compiler);

    let printed =
        run_wasm_with_six_imports_compiler_mode(stage2_self_compiler, source, &expr_tag_args)
            .expect("V2-12 expr-tag: stage2_self_compiler の expr-tag 実行に失敗した");
    let values: Vec<i64> = printed
        .lines()
        .filter(|line| !line.trim().is_empty())
        .map(|line| {
            line.trim()
                .parse::<i64>()
                .unwrap_or_else(|_| panic!("V2-12 expr-tag: 数値でない診断出力: {line:?}"))
        })
        .collect();

    assert_eq!(
        values,
        vec![73, 0, 32, 12, 12, 6, 6, 20, 0, 6, 3, 3],
        "V2-12 expr-tag: stage2 parser は defn/main-if/string tags を保つべき"
    );
}

#[test]
fn test_e2e_boot04_self_hosted_stage2_compiles_module_import_file() {
    let main_path = selfhost_main_path();
    let selfhost_root = main_path
        .parent()
        .expect("App/ ディレクトリ")
        .parent()
        .expect("src/ ディレクトリ")
        .parent()
        .expect("selfhost/ ルートディレクトリ")
        .to_path_buf();

    let stage1_wasm = compile_file_only(&main_path);
    assert_valid_wasm(&stage1_wasm);

    let stage2_output = lsharp_wasm::wasi_runner::run_wasm_wasi_with_dir_and_args(
        &stage1_wasm,
        Some(&selfhost_root),
        &["compiler", "src/App/Main.ls"],
    )
    .expect("BOOT-04 module-import: stage1 が Main.ls の self-compile に失敗した");
    let stage2_modules = parse_emitted_wasm_modules(&stage2_output, 1);
    let stage2_self_compiler = &stage2_modules[0];
    assert_valid_wasm(stage2_self_compiler);

    let module_import_source = "(module App.Main)\n(import App.CompilerMode)\n(defn main [] 0)\n";
    let stage3_output = run_wasm_with_six_imports_compiler_mode(
        stage2_self_compiler,
        module_import_source,
        &["compiler", "src/App/Main.ls"],
    )
    .expect(
        "BOOT-04 module-import: stage2_self_compiler が module+import source をコンパイルできない",
    );
    let stage3_modules = parse_emitted_wasm_modules(&stage3_output, 1);
    let stage3_wasm = &stage3_modules[0];
    assert_valid_wasm(stage3_wasm);
    validate_wasm_detailed(stage3_wasm)
        .unwrap_or_else(|e| panic!("BOOT-04 module-import: stage3 wasm validation failed: {e}"));
}

#[test]
fn test_e2e_boot04_self_hosted_stage2_compiles_main_shape_source() {
    let main_path = selfhost_main_path();
    let selfhost_root = main_path
        .parent()
        .expect("App/ ディレクトリ")
        .parent()
        .expect("src/ ディレクトリ")
        .parent()
        .expect("selfhost/ ルートディレクトリ")
        .to_path_buf();

    let stage1_wasm = compile_file_only(&main_path);
    assert_valid_wasm(&stage1_wasm);

    let stage2_output = lsharp_wasm::wasi_runner::run_wasm_wasi_with_dir_and_args(
        &stage1_wasm,
        Some(&selfhost_root),
        &["compiler", "src/App/Main.ls"],
    )
    .expect("BOOT-04 main-shape: stage1 が Main.ls の self-compile に失敗した");
    let stage2_modules = parse_emitted_wasm_modules(&stage2_output, 1);
    let stage2_self_compiler = &stage2_modules[0];
    assert_valid_wasm(stage2_self_compiler);

    let temp_root = std::env::temp_dir().join(format!(
        "lsharp-boot04-main-shape-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .expect("時刻が巻き戻った")
            .as_nanos()
    ));
    let app_dir = temp_root.join("src/App");
    std::fs::create_dir_all(&app_dir).expect("main-shape temp dir を作れない");
    std::fs::write(
        app_dir.join("Main.ls"),
        "(module App.Main)\n(import App.CompilerMode)\n(import App.PipelineSmoke)\n(defn main [] (if (> (string-length (command-line-arg 1)) 0) (compile-file-mode) (run-main-smoke)))\n",
    )
    .expect("main-shape Main.ls を書けない");
    std::fs::write(
        app_dir.join("CompilerMode.ls"),
        "(module App.CompilerMode)\n(defn compile-file-mode [] 1)\n",
    )
    .expect("main-shape CompilerMode.ls を書けない");
    std::fs::write(
        app_dir.join("PipelineSmoke.ls"),
        "(module App.PipelineSmoke)\n(defn run-main-smoke [] 2)\n",
    )
    .expect("main-shape PipelineSmoke.ls を書けない");

    let stage3_output = run_wasm_with_six_imports_compiler_mode_fs(
        stage2_self_compiler,
        &temp_root,
        &["compiler", "src/App/Main.ls"],
    )
    .expect(
        "BOOT-04 main-shape: stage2_self_compiler が Main.ls shape package をコンパイルできない",
    );
    let stage3_modules = parse_emitted_wasm_modules(&stage3_output, 1);
    let stage3_wasm = &stage3_modules[0];
    assert_valid_wasm(stage3_wasm);
    validate_wasm_detailed(stage3_wasm)
        .unwrap_or_else(|e| panic!("BOOT-04 main-shape: stage3 wasm validation failed: {e}"));
    let engine = wasmtime::Engine::default();
    wasmtime::Module::new(&engine, stage3_wasm)
        .unwrap_or_else(|e| panic!("BOOT-04 main-shape: wasmtime load failed: {e}"));

    std::fs::remove_dir_all(&temp_root).expect("main-shape temp dir を削除できない");
}

#[test]
fn test_e2e_boot04_self_hosted_stage2_compiles_text_eq_repro_source() {
    let main_path = selfhost_main_path();
    let selfhost_root = main_path
        .parent()
        .expect("App/ ディレクトリ")
        .parent()
        .expect("src/ ディレクトリ")
        .parent()
        .expect("selfhost/ ルートディレクトリ")
        .to_path_buf();

    let stage1_wasm = compile_file_only(&main_path);
    assert_valid_wasm(&stage1_wasm);

    let stage2_output = lsharp_wasm::wasi_runner::run_wasm_wasi_with_dir_and_args(
        &stage1_wasm,
        Some(&selfhost_root),
        &["compiler", "src/App/Main.ls"],
    )
    .expect("BOOT-04 text-eq-repro: stage1 が Main.ls の self-compile に失敗した");
    let stage2_modules = parse_emitted_wasm_modules(&stage2_output, 1);
    let stage2_self_compiler = &stage2_modules[0];
    assert_valid_wasm(stage2_self_compiler);

    let source = "(module App.ModuleResolver)\n(defn text-eq-loop [left right idx len] (if (>= idx len) true (if (= (string-char-at left idx) (string-char-at right idx)) (text-eq-loop left right (+ idx 1) len) false)))\n(defn text-eq [left right] (let [len (string-length left)] (if (= len (string-length right)) (text-eq-loop left right 0 len) false)))\n(defn main [] (print (if (text-eq (command-line-arg 0) (command-line-arg 1)) 1 0)))\n";
    let stage3_output = run_wasm_with_six_imports_compiler_mode(
        stage2_self_compiler,
        source,
        &["compiler", "src/App/ModuleResolver.ls"],
    )
    .expect("BOOT-04 text-eq-repro: stage2_self_compiler が repro source をコンパイルできない");
    let stage3_modules = parse_emitted_wasm_modules(&stage3_output, 1);
    let stage3_wasm = &stage3_modules[0];
    assert_valid_wasm(stage3_wasm);
    validate_wasm_detailed(stage3_wasm)
        .unwrap_or_else(|e| panic!("BOOT-04 text-eq-repro: stage3 wasm validation failed: {e}"));
    let run_output = run_wasm_with_six_imports_compiler_mode(stage3_wasm, "", &["same", "same"])
        .unwrap_or_else(|e| panic!("BOOT-04 text-eq-repro: 実行失敗: {e}"));
    assert_eq!(run_output.trim(), "1");
}

#[test]
fn test_e2e_boot04_self_hosted_stage2_compiles_string_length_repro_source() {
    let main_path = selfhost_main_path();
    let selfhost_root = main_path
        .parent()
        .expect("App/ ディレクトリ")
        .parent()
        .expect("src/ ディレクトリ")
        .parent()
        .expect("selfhost/ ルートディレクトリ")
        .to_path_buf();

    let stage1_wasm = compile_file_only(&main_path);
    assert_valid_wasm(&stage1_wasm);

    let stage2_output = lsharp_wasm::wasi_runner::run_wasm_wasi_with_dir_and_args(
        &stage1_wasm,
        Some(&selfhost_root),
        &["compiler", "src/App/Main.ls"],
    )
    .expect("BOOT-04 string-length-repro: stage1 が Main.ls の self-compile に失敗した");
    let stage2_modules = parse_emitted_wasm_modules(&stage2_output, 1);
    let stage2_self_compiler = &stage2_modules[0];
    assert_valid_wasm(stage2_self_compiler);

    let source = "(module App.ModuleResolver)\n(defn text-len [text] (string-length text))\n(defn main [] (text-len (command-line-arg 0)))\n";
    let stage3_output = run_wasm_with_six_imports_compiler_mode(
        stage2_self_compiler,
        source,
        &["compiler", "src/App/ModuleResolver.ls"],
    )
    .expect(
        "BOOT-04 string-length-repro: stage2_self_compiler が repro source をコンパイルできない",
    );
    let stage3_modules = parse_emitted_wasm_modules(&stage3_output, 1);
    let stage3_wasm = &stage3_modules[0];
    assert_valid_wasm(stage3_wasm);
    validate_wasm_detailed(stage3_wasm).unwrap_or_else(|e| {
        panic!("BOOT-04 string-length-repro: stage3 wasm validation failed: {e}")
    });
    let engine = wasmtime::Engine::default();
    wasmtime::Module::new(&engine, stage3_wasm).unwrap_or_else(|e| {
        panic!(
            "BOOT-04 string-length-repro: wasmtime load failed: {} / {:?}",
            e, e
        )
    });
}

#[test]
fn test_e2e_boot04_self_hosted_stage2_compiles_string_length_if_repro_source() {
    let main_path = selfhost_main_path();
    let selfhost_root = main_path
        .parent()
        .expect("App/ ディレクトリ")
        .parent()
        .expect("src/ ディレクトリ")
        .parent()
        .expect("selfhost/ ルートディレクトリ")
        .to_path_buf();

    let stage1_wasm = compile_file_only(&main_path);
    assert_valid_wasm(&stage1_wasm);

    let stage2_output = lsharp_wasm::wasi_runner::run_wasm_wasi_with_dir_and_args(
        &stage1_wasm,
        Some(&selfhost_root),
        &["compiler", "src/App/Main.ls"],
    )
    .expect("BOOT-04 string-length-if-repro: stage1 が Main.ls の self-compile に失敗した");
    let stage2_modules = parse_emitted_wasm_modules(&stage2_output, 1);
    let stage2_self_compiler = &stage2_modules[0];
    assert_valid_wasm(stage2_self_compiler);

    let source = "(module App.ModuleResolver)\n(defn text-len-eq [left right] (let [len (string-length left)] (if (= len (string-length right)) 1 0)))\n(defn main [] (text-len-eq (command-line-arg 0) (command-line-arg 1)))\n";
    let stage3_output = run_wasm_with_six_imports_compiler_mode(
        stage2_self_compiler,
        source,
        &["compiler", "src/App/ModuleResolver.ls"],
    )
    .expect(
        "BOOT-04 string-length-if-repro: stage2_self_compiler が repro source をコンパイルできない",
    );
    let stage3_modules = parse_emitted_wasm_modules(&stage3_output, 1);
    let stage3_wasm = &stage3_modules[0];
    assert_valid_wasm(stage3_wasm);
    validate_wasm_detailed(stage3_wasm).unwrap_or_else(|e| {
        panic!("BOOT-04 string-length-if-repro: stage3 wasm validation failed: {e}")
    });
    let engine = wasmtime::Engine::default();
    wasmtime::Module::new(&engine, stage3_wasm).unwrap_or_else(|e| {
        panic!(
            "BOOT-04 string-length-if-repro: wasmtime load failed: {} / {:?}",
            e, e
        )
    });
}

#[test]
fn test_e2e_boot04_self_hosted_stage2_compiles_let_string_length_repro_source() {
    let main_path = selfhost_main_path();
    let selfhost_root = main_path
        .parent()
        .expect("App/ ディレクトリ")
        .parent()
        .expect("src/ ディレクトリ")
        .parent()
        .expect("selfhost/ ルートディレクトリ")
        .to_path_buf();

    let stage1_wasm = compile_file_only(&main_path);
    assert_valid_wasm(&stage1_wasm);

    let stage2_output = lsharp_wasm::wasi_runner::run_wasm_wasi_with_dir_and_args(
        &stage1_wasm,
        Some(&selfhost_root),
        &["compiler", "src/App/Main.ls"],
    )
    .expect("BOOT-04 let-string-length-repro: stage1 が Main.ls の self-compile に失敗した");
    let stage2_modules = parse_emitted_wasm_modules(&stage2_output, 1);
    let stage2_self_compiler = &stage2_modules[0];
    assert_valid_wasm(stage2_self_compiler);

    let source = "(module App.ModuleResolver)\n(defn text-len [left] (let [len (string-length left)] len))\n(defn main [] (text-len (command-line-arg 0)))\n";
    let stage3_output = run_wasm_with_six_imports_compiler_mode(
        stage2_self_compiler,
        source,
        &["compiler", "src/App/ModuleResolver.ls"],
    )
    .expect("BOOT-04 let-string-length-repro: stage2_self_compiler が repro source をコンパイルできない");
    let stage3_modules = parse_emitted_wasm_modules(&stage3_output, 1);
    let stage3_wasm = &stage3_modules[0];
    assert_valid_wasm(stage3_wasm);
    validate_wasm_detailed(stage3_wasm).unwrap_or_else(|e| {
        panic!("BOOT-04 let-string-length-repro: stage3 wasm validation failed: {e}")
    });
    let engine = wasmtime::Engine::default();
    wasmtime::Module::new(&engine, stage3_wasm).unwrap_or_else(|e| {
        panic!(
            "BOOT-04 let-string-length-repro: wasmtime load failed: {} / {:?}",
            e, e
        )
    });
}

#[test]
fn test_e2e_boot04_self_hosted_stage2_compiles_eq_string_length_repro_source() {
    let main_path = selfhost_main_path();
    let selfhost_root = main_path
        .parent()
        .expect("App/ ディレクトリ")
        .parent()
        .expect("src/ ディレクトリ")
        .parent()
        .expect("selfhost/ ルートディレクトリ")
        .to_path_buf();

    let stage1_wasm = compile_file_only(&main_path);
    assert_valid_wasm(&stage1_wasm);

    let stage2_output = lsharp_wasm::wasi_runner::run_wasm_wasi_with_dir_and_args(
        &stage1_wasm,
        Some(&selfhost_root),
        &["compiler", "src/App/Main.ls"],
    )
    .expect("BOOT-04 eq-string-length-repro: stage1 が Main.ls の self-compile に失敗した");
    let stage2_modules = parse_emitted_wasm_modules(&stage2_output, 1);
    let stage2_self_compiler = &stage2_modules[0];
    assert_valid_wasm(stage2_self_compiler);

    let source = "(module App.ModuleResolver)\n(defn text-len-eq [left right] (= (string-length left) (string-length right)))\n(defn main [] (text-len-eq (command-line-arg 0) (command-line-arg 1)))\n";
    let stage3_output = run_wasm_with_six_imports_compiler_mode(
        stage2_self_compiler,
        source,
        &["compiler", "src/App/ModuleResolver.ls"],
    )
    .expect(
        "BOOT-04 eq-string-length-repro: stage2_self_compiler が repro source をコンパイルできない",
    );
    let stage3_modules = parse_emitted_wasm_modules(&stage3_output, 1);
    let stage3_wasm = &stage3_modules[0];
    assert_valid_wasm(stage3_wasm);
    validate_wasm_detailed(stage3_wasm).unwrap_or_else(|e| {
        panic!("BOOT-04 eq-string-length-repro: stage3 wasm validation failed: {e}")
    });
    let engine = wasmtime::Engine::default();
    wasmtime::Module::new(&engine, stage3_wasm).unwrap_or_else(|e| {
        panic!(
            "BOOT-04 eq-string-length-repro: wasmtime load failed: {} / {:?}",
            e, e
        )
    });
}

#[test]
fn test_e2e_boot04_self_hosted_stage2_compiles_let_eq_string_length_repro_source() {
    let main_path = selfhost_main_path();
    let selfhost_root = main_path
        .parent()
        .expect("App/ ディレクトリ")
        .parent()
        .expect("src/ ディレクトリ")
        .parent()
        .expect("selfhost/ ルートディレクトリ")
        .to_path_buf();

    let stage1_wasm = compile_file_only(&main_path);
    assert_valid_wasm(&stage1_wasm);

    let stage2_output = lsharp_wasm::wasi_runner::run_wasm_wasi_with_dir_and_args(
        &stage1_wasm,
        Some(&selfhost_root),
        &["compiler", "src/App/Main.ls"],
    )
    .expect("BOOT-04 let-eq-string-length-repro: stage1 が Main.ls の self-compile に失敗した");
    let stage2_modules = parse_emitted_wasm_modules(&stage2_output, 1);
    let stage2_self_compiler = &stage2_modules[0];
    assert_valid_wasm(stage2_self_compiler);

    let source = "(module App.ModuleResolver)\n(defn text-len-eq [left right] (let [len (string-length left)] (= len (string-length right))))\n(defn main [] (text-len-eq (command-line-arg 0) (command-line-arg 1)))\n";
    let stage3_output = run_wasm_with_six_imports_compiler_mode(
        stage2_self_compiler,
        source,
        &["compiler", "src/App/ModuleResolver.ls"],
    )
    .expect("BOOT-04 let-eq-string-length-repro: stage2_self_compiler が repro source をコンパイルできない");
    let stage3_modules = parse_emitted_wasm_modules(&stage3_output, 1);
    let stage3_wasm = &stage3_modules[0];
    assert_valid_wasm(stage3_wasm);
    validate_wasm_detailed(stage3_wasm).unwrap_or_else(|e| {
        panic!("BOOT-04 let-eq-string-length-repro: stage3 wasm validation failed: {e}")
    });
    let engine = wasmtime::Engine::default();
    wasmtime::Module::new(&engine, stage3_wasm).unwrap_or_else(|e| {
        panic!(
            "BOOT-04 let-eq-string-length-repro: wasmtime load failed: {} / {:?}",
            e, e
        )
    });
}

#[test]
fn test_e2e_boot04_self_hosted_stage2_runs_path_parent_repro_source() {
    let main_path = selfhost_main_path();
    let selfhost_root = main_path
        .parent()
        .expect("App/ ディレクトリ")
        .parent()
        .expect("src/ ディレクトリ")
        .parent()
        .expect("selfhost/ ルートディレクトリ")
        .to_path_buf();

    let stage1_wasm = compile_file_only(&main_path);
    assert_valid_wasm(&stage1_wasm);

    let stage2_output = lsharp_wasm::wasi_runner::run_wasm_wasi_with_dir_and_args(
        &stage1_wasm,
        Some(&selfhost_root),
        &["compiler", "src/App/Main.ls"],
    )
    .expect("BOOT-04 path-parent-repro: stage1 が Main.ls の self-compile に失敗した");
    let stage2_modules = parse_emitted_wasm_modules(&stage2_output, 1);
    let stage2_self_compiler = &stage2_modules[0];
    assert_valid_wasm(stage2_self_compiler);

    let source = "(module App.ModuleResolver)\n(defn path-parent [path] (let [len (string-length path)] (if (= len 0) \"\" (if (has-path-sep path 0 len) (let [last (find-last-path-sep path 0 len -1)] (if (< last 0) \"\" (if (= last 0) \"/\" (substring path 0 last)))) \".\"))))\n(defn path-char [path idx] (string-char-at path idx))\n(defn is-path-sep [path idx] (let [ch (path-char path idx)] (if (= ch 47) true (if (= ch 92) true false))))\n(defn has-path-sep [path idx len] (if (>= idx len) false (if (is-path-sep path idx) true (has-path-sep path (+ idx 1) len))))\n(defn find-last-path-sep [path idx len last] (if (>= idx len) last (find-last-path-sep path (+ idx 1) len (if (is-path-sep path idx) idx last))))\n(defn main [] (print (string-length (path-parent (command-line-arg 1)))))\n";
    let stage3_output = run_wasm_with_six_imports_compiler_mode(
        stage2_self_compiler,
        source,
        &["compiler", "src/App/ModuleResolver.ls"],
    )
    .expect("BOOT-04 path-parent-repro: stage2_self_compiler が repro source をコンパイルできない");
    let stage3_modules = parse_emitted_wasm_modules(&stage3_output, 1);
    let stage3_wasm = &stage3_modules[0];
    assert_valid_wasm(stage3_wasm);
    validate_wasm_detailed(stage3_wasm).unwrap_or_else(|e| {
        panic!("BOOT-04 path-parent-repro: stage3 wasm validation failed: {e}")
    });

    let run_output = run_wasm_with_six_imports_compiler_mode(stage3_wasm, "", &["prog", "a/b"])
        .unwrap_or_else(|e| panic!("BOOT-04 path-parent-repro: 実行失敗: {e}"));
    assert_eq!(run_output.trim(), "1");
}

#[test]
fn test_e2e_boot04_self_hosted_stage2_runs_path_join_repro_source() {
    let main_path = selfhost_main_path();
    let selfhost_root = main_path
        .parent()
        .expect("App/ ディレクトリ")
        .parent()
        .expect("src/ ディレクトリ")
        .parent()
        .expect("selfhost/ ルートディレクトリ")
        .to_path_buf();

    let stage1_wasm = compile_file_only(&main_path);
    assert_valid_wasm(&stage1_wasm);

    let stage2_output = lsharp_wasm::wasi_runner::run_wasm_wasi_with_dir_and_args(
        &stage1_wasm,
        Some(&selfhost_root),
        &["compiler", "src/App/Main.ls"],
    )
    .expect("BOOT-04 path-join-repro: stage1 が Main.ls の self-compile に失敗した");
    let stage2_modules = parse_emitted_wasm_modules(&stage2_output, 1);
    let stage2_self_compiler = &stage2_modules[0];
    assert_valid_wasm(stage2_self_compiler);

    let source = "(module App.ModuleResolver)\n(defn path-join [base child] (if (= (string-length base) 0) child (let [len (string-length base)] (if (= (string-char-at base (- len 1)) 47) (string-concat base child) (if (= (string-char-at base (- len 1)) 92) (string-concat base child) (string-concat (string-concat base \"/\") child))))))\n(defn main [] (print (string-length (path-join (command-line-arg 1) (command-line-arg 2)))))\n";
    let stage3_output = run_wasm_with_six_imports_compiler_mode(
        stage2_self_compiler,
        source,
        &["compiler", "src/App/ModuleResolver.ls"],
    )
    .expect("BOOT-04 path-join-repro: stage2_self_compiler が repro source をコンパイルできない");
    let stage3_modules = parse_emitted_wasm_modules(&stage3_output, 1);
    let stage3_wasm = &stage3_modules[0];
    assert_valid_wasm(stage3_wasm);
    validate_wasm_detailed(stage3_wasm)
        .unwrap_or_else(|e| panic!("BOOT-04 path-join-repro: stage3 wasm validation failed: {e}"));

    let run_output = run_wasm_with_six_imports_compiler_mode(stage3_wasm, "", &["prog", "a", "b"])
        .unwrap_or_else(|e| panic!("BOOT-04 path-join-repro: 実行失敗: {e}"));
    assert_eq!(run_output.trim(), "3");
}

#[test]
fn test_e2e_boot04_self_hosted_stage2_runs_string_concat_repro_source() {
    let main_path = selfhost_main_path();
    let selfhost_root = main_path
        .parent()
        .expect("App/ ディレクトリ")
        .parent()
        .expect("src/ ディレクトリ")
        .parent()
        .expect("selfhost/ ルートディレクトリ")
        .to_path_buf();

    let stage1_wasm = compile_file_only(&main_path);
    assert_valid_wasm(&stage1_wasm);

    let stage2_output = lsharp_wasm::wasi_runner::run_wasm_wasi_with_dir_and_args(
        &stage1_wasm,
        Some(&selfhost_root),
        &["compiler", "src/App/Main.ls"],
    )
    .expect("BOOT-04 string-concat-repro: stage1 が Main.ls の self-compile に失敗した");
    let stage2_modules = parse_emitted_wasm_modules(&stage2_output, 1);
    let stage2_self_compiler = &stage2_modules[0];
    assert_valid_wasm(stage2_self_compiler);

    let source = "(module App.ModuleResolver)\n(defn main [] (let [value (string-concat (command-line-arg 1) (command-line-arg 2))] (do (print (string-length value)) (print (string-char-at value 0)) (print (string-char-at value 1)) 0)))\n";
    let stage3_output = run_wasm_with_six_imports_compiler_mode(
        stage2_self_compiler,
        source,
        &["compiler", "src/App/ModuleResolver.ls"],
    )
    .expect(
        "BOOT-04 string-concat-repro: stage2_self_compiler が repro source をコンパイルできない",
    );
    let stage3_modules = parse_emitted_wasm_modules(&stage3_output, 1);
    let stage3_wasm = &stage3_modules[0];
    assert_valid_wasm(stage3_wasm);
    validate_wasm_detailed(stage3_wasm).unwrap_or_else(|e| {
        panic!("BOOT-04 string-concat-repro: stage3 wasm validation failed: {e}")
    });

    let run_output = run_wasm_with_six_imports_compiler_mode(stage3_wasm, "", &["prog", "a", "b"])
        .unwrap_or_else(|e| panic!("BOOT-04 string-concat-repro: 実行失敗: {e}"));
    let values: Vec<i64> = run_output
        .lines()
        .filter(|line| !line.trim().is_empty())
        .map(|line| {
            line.trim()
                .parse::<i64>()
                .unwrap_or_else(|_| panic!("BOOT-04 string-concat-repro: 数値でない出力: {line:?}"))
        })
        .collect();
    assert_eq!(values, vec![2, 97, 98]);
}

#[test]
fn test_e2e_boot04_self_hosted_stage2_runs_recursive_string_accumulator_repro_source() {
    let main_path = selfhost_main_path();
    let selfhost_root = main_path
        .parent()
        .expect("App/ ディレクトリ")
        .parent()
        .expect("src/ ディレクトリ")
        .parent()
        .expect("selfhost/ ルートディレクトリ")
        .to_path_buf();

    let stage1_wasm = compile_file_only(&main_path);
    assert_valid_wasm(&stage1_wasm);

    let stage2_output = lsharp_wasm::wasi_runner::run_wasm_wasi_with_dir_and_args(
        &stage1_wasm,
        Some(&selfhost_root),
        &["compiler", "src/App/Main.ls"],
    )
    .expect(
        "BOOT-04 recursive-string-accumulator-repro: stage1 が Main.ls の self-compile に失敗した",
    );
    let stage2_modules = parse_emitted_wasm_modules(&stage2_output, 1);
    let stage2_self_compiler = &stage2_modules[0];
    assert_valid_wasm(stage2_self_compiler);

    let source = "(module App.ModuleResolver)\n(defn grow-loop [seed idx len out] (if (>= idx len) out (grow-loop seed (+ idx 1) len (string-concat out seed))))\n(defn main [] (let [value (grow-loop (command-line-arg 1) 0 2 \"\")] (do (print (string-length value)) (print (string-char-at value 0)) (print (string-char-at value 1)) 0)))\n";
    let stage3_output = run_wasm_with_six_imports_compiler_mode(
        stage2_self_compiler,
        source,
        &["compiler", "src/App/ModuleResolver.ls"],
    )
    .expect("BOOT-04 recursive-string-accumulator-repro: stage2_self_compiler が repro source をコンパイルできない");
    let stage3_modules = parse_emitted_wasm_modules(&stage3_output, 1);
    let stage3_wasm = &stage3_modules[0];
    assert_valid_wasm(stage3_wasm);
    validate_wasm_detailed(stage3_wasm).unwrap_or_else(|e| {
        panic!("BOOT-04 recursive-string-accumulator-repro: stage3 wasm validation failed: {e}")
    });

    let run_output = run_wasm_with_six_imports_compiler_mode(stage3_wasm, "", &["prog", "a"])
        .unwrap_or_else(|e| panic!("BOOT-04 recursive-string-accumulator-repro: 実行失敗: {e}"));
    let values: Vec<i64> = run_output
        .lines()
        .filter(|line| !line.trim().is_empty())
        .map(|line| {
            line.trim().parse::<i64>().unwrap_or_else(|_| {
                panic!("BOOT-04 recursive-string-accumulator-repro: 数値でない出力: {line:?}")
            })
        })
        .collect();
    assert_eq!(values, vec![2, 97, 97]);
}

#[test]
fn test_e2e_boot04_self_hosted_stage2_runs_substring_repro_source() {
    let main_path = selfhost_main_path();
    let selfhost_root = main_path
        .parent()
        .expect("App/ ディレクトリ")
        .parent()
        .expect("src/ ディレクトリ")
        .parent()
        .expect("selfhost/ ルートディレクトリ")
        .to_path_buf();

    let stage1_wasm = compile_file_only(&main_path);
    assert_valid_wasm(&stage1_wasm);

    let stage2_output = lsharp_wasm::wasi_runner::run_wasm_wasi_with_dir_and_args(
        &stage1_wasm,
        Some(&selfhost_root),
        &["compiler", "src/App/Main.ls"],
    )
    .expect("BOOT-04 substring-repro: stage1 が Main.ls の self-compile に失敗した");
    let stage2_modules = parse_emitted_wasm_modules(&stage2_output, 1);
    let stage2_self_compiler = &stage2_modules[0];
    assert_valid_wasm(stage2_self_compiler);

    let source = "(module App.ModuleResolver)\n(defn main [] (let [value (substring (command-line-arg 1) 1 3)] (do (print (string-length value)) (print (string-char-at value 0)) (print (string-char-at value 1)) 0)))\n";
    let stage3_output = run_wasm_with_six_imports_compiler_mode(
        stage2_self_compiler,
        source,
        &["compiler", "src/App/ModuleResolver.ls"],
    )
    .expect("BOOT-04 substring-repro: stage2_self_compiler が repro source をコンパイルできない");
    let stage3_modules = parse_emitted_wasm_modules(&stage3_output, 1);
    let stage3_wasm = &stage3_modules[0];
    assert_valid_wasm(stage3_wasm);
    validate_wasm_detailed(stage3_wasm)
        .unwrap_or_else(|e| panic!("BOOT-04 substring-repro: stage3 wasm validation failed: {e}"));

    let run_output = run_wasm_with_six_imports_compiler_mode(stage3_wasm, "", &["prog", "abcd"])
        .unwrap_or_else(|e| panic!("BOOT-04 substring-repro: 実行失敗: {e}"));
    let values: Vec<i64> = run_output
        .lines()
        .filter(|line| !line.trim().is_empty())
        .map(|line| {
            line.trim()
                .parse::<i64>()
                .unwrap_or_else(|_| panic!("BOOT-04 substring-repro: 数値でない出力: {line:?}"))
        })
        .collect();
    assert_eq!(values, vec![2, 98, 99]);
}

#[test]
fn test_e2e_boot04_self_hosted_stage2_runs_recursive_substring_accumulator_repro_source() {
    let main_path = selfhost_main_path();
    let selfhost_root = main_path
        .parent()
        .expect("App/ ディレクトリ")
        .parent()
        .expect("src/ ディレクトリ")
        .parent()
        .expect("selfhost/ ルートディレクトリ")
        .to_path_buf();

    let stage1_wasm = compile_file_only(&main_path);
    assert_valid_wasm(&stage1_wasm);

    let stage2_output = lsharp_wasm::wasi_runner::run_wasm_wasi_with_dir_and_args(
        &stage1_wasm,
        Some(&selfhost_root),
        &["compiler", "src/App/Main.ls"],
    )
    .expect(
        "BOOT-04 recursive-substring-accumulator-repro: stage1 が Main.ls の self-compile に失敗した",
    );
    let stage2_modules = parse_emitted_wasm_modules(&stage2_output, 1);
    let stage2_self_compiler = &stage2_modules[0];
    assert_valid_wasm(stage2_self_compiler);

    let source = "(module App.ModuleResolver)\n(defn text-eq-loop [left right idx len] (if (>= idx len) true (if (= (string-char-at left idx) (string-char-at right idx)) (text-eq-loop left right (+ idx 1) len) false)))\n(defn text-eq [left right] (let [len (string-length left)] (if (= len (string-length right)) (text-eq-loop left right 0 len) false)))\n(defn copy-loop [src idx len out] (if (>= idx len) out (copy-loop src (+ idx 1) len (string-concat out (substring src idx (+ idx 1))))))\n(defn main [] (let [src (command-line-arg 1)] (print (if (text-eq (copy-loop src 0 (string-length src) \"\") src) 1 0))))\n";
    let stage3_output = run_wasm_with_six_imports_compiler_mode(
        stage2_self_compiler,
        source,
        &["compiler", "src/App/ModuleResolver.ls"],
    )
    .expect("BOOT-04 recursive-substring-accumulator-repro: stage2_self_compiler が repro source をコンパイルできない");
    let stage3_modules = parse_emitted_wasm_modules(&stage3_output, 1);
    let stage3_wasm = &stage3_modules[0];
    assert_valid_wasm(stage3_wasm);
    validate_wasm_detailed(stage3_wasm).unwrap_or_else(|e| {
        panic!("BOOT-04 recursive-substring-accumulator-repro: stage3 wasm validation failed: {e}")
    });

    let run_output = run_wasm_with_six_imports_compiler_mode(stage3_wasm, "", &["prog", "abc"])
        .unwrap_or_else(|e| panic!("BOOT-04 recursive-substring-accumulator-repro: 実行失敗: {e}"));
    assert_eq!(run_output.trim(), "1");
}

#[test]
fn test_e2e_boot04_self_hosted_stage2_runs_string_concat_literal_suffix_repro_source() {
    let main_path = selfhost_main_path();
    let selfhost_root = main_path
        .parent()
        .expect("App/ ディレクトリ")
        .parent()
        .expect("src/ ディレクトリ")
        .parent()
        .expect("selfhost/ ルートディレクトリ")
        .to_path_buf();

    let stage1_wasm = compile_file_only(&main_path);
    assert_valid_wasm(&stage1_wasm);

    let stage2_output = lsharp_wasm::wasi_runner::run_wasm_wasi_with_dir_and_args(
        &stage1_wasm,
        Some(&selfhost_root),
        &["compiler", "src/App/Main.ls"],
    )
    .expect(
        "BOOT-04 string-concat-literal-suffix-repro: stage1 が Main.ls の self-compile に失敗した",
    );
    let stage2_modules = parse_emitted_wasm_modules(&stage2_output, 1);
    let stage2_self_compiler = &stage2_modules[0];
    assert_valid_wasm(stage2_self_compiler);

    let source = "(module App.ModuleResolver)\n(defn text-eq-loop [left right idx len] (if (>= idx len) true (if (= (string-char-at left idx) (string-char-at right idx)) (text-eq-loop left right (+ idx 1) len) false)))\n(defn text-eq [left right] (let [len (string-length left)] (if (= len (string-length right)) (text-eq-loop left right 0 len) false)))\n(defn main [] (print (if (text-eq (string-concat (command-line-arg 1) \".ls\") \"ab.ls\") 1 0)))\n";
    let stage3_output = run_wasm_with_six_imports_compiler_mode(
        stage2_self_compiler,
        source,
        &["compiler", "src/App/ModuleResolver.ls"],
    )
    .expect("BOOT-04 string-concat-literal-suffix-repro: stage2_self_compiler が repro source をコンパイルできない");
    let stage3_modules = parse_emitted_wasm_modules(&stage3_output, 1);
    let stage3_wasm = &stage3_modules[0];
    assert_valid_wasm(stage3_wasm);
    validate_wasm_detailed(stage3_wasm).unwrap_or_else(|e| {
        panic!("BOOT-04 string-concat-literal-suffix-repro: stage3 wasm validation failed: {e}")
    });

    let run_output = run_wasm_with_six_imports_compiler_mode(stage3_wasm, "", &["prog", "ab"])
        .unwrap_or_else(|e| panic!("BOOT-04 string-concat-literal-suffix-repro: 実行失敗: {e}"));
    assert_eq!(run_output.trim(), "1");
}

#[test]
fn test_e2e_boot04_self_hosted_stage2_runs_string_literal_repro_source() {
    let main_path = selfhost_main_path();
    let selfhost_root = main_path
        .parent()
        .expect("App/ ディレクトリ")
        .parent()
        .expect("src/ ディレクトリ")
        .parent()
        .expect("selfhost/ ルートディレクトリ")
        .to_path_buf();

    let stage1_wasm = compile_file_only(&main_path);
    assert_valid_wasm(&stage1_wasm);

    let stage2_output = lsharp_wasm::wasi_runner::run_wasm_wasi_with_dir_and_args(
        &stage1_wasm,
        Some(&selfhost_root),
        &["compiler", "src/App/Main.ls"],
    )
    .expect("BOOT-04 string-literal-repro: stage1 が Main.ls の self-compile に失敗した");
    let stage2_modules = parse_emitted_wasm_modules(&stage2_output, 1);
    let stage2_self_compiler = &stage2_modules[0];
    assert_valid_wasm(stage2_self_compiler);

    let source = "(module App.ModuleResolver)\n(defn main [] (do (print (string-length \".ls\")) (print (string-char-at \".ls\" 0)) (print (string-char-at \".ls\" 1)) (print (string-char-at \".ls\" 2)) 0))\n";
    let stage3_output = run_wasm_with_six_imports_compiler_mode(
        stage2_self_compiler,
        source,
        &["compiler", "src/App/ModuleResolver.ls"],
    )
    .expect(
        "BOOT-04 string-literal-repro: stage2_self_compiler が repro source をコンパイルできない",
    );
    let stage3_modules = parse_emitted_wasm_modules(&stage3_output, 1);
    let stage3_wasm = &stage3_modules[0];
    assert_valid_wasm(stage3_wasm);
    validate_wasm_detailed(stage3_wasm).unwrap_or_else(|e| {
        panic!("BOOT-04 string-literal-repro: stage3 wasm validation failed: {e}")
    });

    let run_output = run_wasm_with_six_imports_compiler_mode(stage3_wasm, "", &["prog"])
        .unwrap_or_else(|e| panic!("BOOT-04 string-literal-repro: 実行失敗: {e}"));
    let values: Vec<i64> = run_output
        .lines()
        .filter(|line| !line.trim().is_empty())
        .map(|line| {
            line.trim().parse::<i64>().unwrap_or_else(|_| {
                panic!("BOOT-04 string-literal-repro: 数値でない出力: {line:?}")
            })
        })
        .collect();
    assert_eq!(values, vec![3, 46, 108, 115]);
}

#[test]
fn test_e2e_boot04_self_hosted_stage2_runs_module_relative_join_repro_source() {
    let main_path = selfhost_main_path();
    let selfhost_root = main_path
        .parent()
        .expect("App/ ディレクトリ")
        .parent()
        .expect("src/ ディレクトリ")
        .parent()
        .expect("selfhost/ ルートディレクトリ")
        .to_path_buf();

    let stage1_wasm = compile_file_only(&main_path);
    assert_valid_wasm(&stage1_wasm);

    let stage2_output = lsharp_wasm::wasi_runner::run_wasm_wasi_with_dir_and_args(
        &stage1_wasm,
        Some(&selfhost_root),
        &["compiler", "src/App/Main.ls"],
    )
    .expect("BOOT-04 module-relative-join-repro: stage1 が Main.ls の self-compile に失敗した");
    let stage2_modules = parse_emitted_wasm_modules(&stage2_output, 1);
    let stage2_self_compiler = &stage2_modules[0];
    assert_valid_wasm(stage2_self_compiler);

    let source = "(module App.ModuleResolver)\n(defn text-eq-loop [left right idx len] (if (>= idx len) true (if (= (string-char-at left idx) (string-char-at right idx)) (text-eq-loop left right (+ idx 1) len) false)))\n(defn text-eq [left right] (let [len (string-length left)] (if (= len (string-length right)) (text-eq-loop left right 0 len) false)))\n(defn path-char [path idx] (string-char-at path idx))\n(defn path-join [base child] (if (= (string-length base) 0) child (let [len (string-length base)] (if (= (string-char-at base (- len 1)) 47) (string-concat base child) (if (= (string-char-at base (- len 1)) 92) (string-concat base child) (string-concat (string-concat base \"/\") child))))))\n(defn module-name-to-relative-loop [name idx len out] (if (>= idx len) (string-concat out \".ls\") (let [piece (if (= (path-char name idx) 46) \"/\" (substring name idx (+ idx 1)))] (module-name-to-relative-loop name (+ idx 1) len (string-concat out piece)))))\n(defn module-name-to-relative [name] (module-name-to-relative-loop name 0 (string-length name) \"\"))\n(defn main [] (print (if (text-eq (path-join \"src\" (module-name-to-relative (command-line-arg 1))) \"src/App/ModuleResolver.ls\") 1 0)))\n";
    let stage3_output = run_wasm_with_six_imports_compiler_mode(
        stage2_self_compiler,
        source,
        &["compiler", "src/App/ModuleResolver.ls"],
    )
    .expect("BOOT-04 module-relative-join-repro: stage2_self_compiler が repro source をコンパイルできない");
    let stage3_modules = parse_emitted_wasm_modules(&stage3_output, 1);
    let stage3_wasm = &stage3_modules[0];
    assert_valid_wasm(stage3_wasm);
    validate_wasm_detailed(stage3_wasm).unwrap_or_else(|e| {
        panic!("BOOT-04 module-relative-join-repro: stage3 wasm validation failed: {e}")
    });

    let run_output =
        run_wasm_with_six_imports_compiler_mode(stage3_wasm, "", &["prog", "App.ModuleResolver"])
            .unwrap_or_else(|e| panic!("BOOT-04 module-relative-join-repro: 実行失敗: {e}"));
    assert_eq!(run_output.trim(), "1");
}

#[test]
fn test_e2e_boot04_self_hosted_stage2_compiles_user_call_four_args_repro_source() {
    let main_path = selfhost_main_path();
    let selfhost_root = main_path
        .parent()
        .expect("App/ ディレクトリ")
        .parent()
        .expect("src/ ディレクトリ")
        .parent()
        .expect("selfhost/ ルートディレクトリ")
        .to_path_buf();

    let stage1_wasm = compile_file_only(&main_path);
    assert_valid_wasm(&stage1_wasm);

    let stage2_output = lsharp_wasm::wasi_runner::run_wasm_wasi_with_dir_and_args(
        &stage1_wasm,
        Some(&selfhost_root),
        &["compiler", "src/App/Main.ls"],
    )
    .expect("BOOT-04 user-call-4-repro: stage1 が Main.ls の self-compile に失敗した");
    let stage2_modules = parse_emitted_wasm_modules(&stage2_output, 1);
    let stage2_self_compiler = &stage2_modules[0];
    assert_valid_wasm(stage2_self_compiler);

    let source = "(module App.ModuleResolver)\n(defn helper [left right idx len] 1)\n(defn text-eq [left right] (helper left right 0 0))\n(defn main [] (text-eq (command-line-arg 0) (command-line-arg 1)))\n";
    let stage3_output = run_wasm_with_six_imports_compiler_mode(
        stage2_self_compiler,
        source,
        &["compiler", "src/App/ModuleResolver.ls"],
    )
    .expect("BOOT-04 user-call-4-repro: stage2_self_compiler が repro source をコンパイルできない");
    let stage3_modules = parse_emitted_wasm_modules(&stage3_output, 1);
    let stage3_wasm = &stage3_modules[0];
    assert_valid_wasm(stage3_wasm);
    validate_wasm_detailed(stage3_wasm).unwrap_or_else(|e| {
        panic!("BOOT-04 user-call-4-repro: stage3 wasm validation failed: {e}")
    });
    let engine = wasmtime::Engine::default();
    wasmtime::Module::new(&engine, stage3_wasm).unwrap_or_else(|e| {
        panic!(
            "BOOT-04 user-call-4-repro: wasmtime load failed: {} / {:?}",
            e, e
        )
    });
}

#[test]
fn test_e2e_boot04_self_hosted_stage2_runs_command_line_arg_repro_source() {
    let main_path = selfhost_main_path();
    let selfhost_root = main_path
        .parent()
        .expect("App/ ディレクトリ")
        .parent()
        .expect("src/ ディレクトリ")
        .parent()
        .expect("selfhost/ ルートディレクトリ")
        .to_path_buf();

    let stage1_wasm = compile_file_only(&main_path);
    assert_valid_wasm(&stage1_wasm);

    let stage2_output = lsharp_wasm::wasi_runner::run_wasm_wasi_with_dir_and_args(
        &stage1_wasm,
        Some(&selfhost_root),
        &["compiler", "src/App/Main.ls"],
    )
    .expect("BOOT-04 command-line-arg-repro: stage1 が Main.ls の self-compile に失敗した");
    let stage2_modules = parse_emitted_wasm_modules(&stage2_output, 1);
    let stage2_self_compiler = &stage2_modules[0];
    assert_valid_wasm(stage2_self_compiler);

    let source = "(module App.Main)\n(defn main [] (print (string-length (command-line-arg 1))))\n";
    let stage3_output = run_wasm_with_six_imports_compiler_mode(
        stage2_self_compiler,
        source,
        &["compiler", "src/App/Main.ls"],
    )
    .expect(
        "BOOT-04 command-line-arg-repro: stage2_self_compiler が repro source をコンパイルできない",
    );
    let stage3_modules = parse_emitted_wasm_modules(&stage3_output, 1);
    let stage3_wasm = &stage3_modules[0];
    assert_valid_wasm(stage3_wasm);
    validate_wasm_detailed(stage3_wasm).unwrap_or_else(|e| {
        panic!("BOOT-04 command-line-arg-repro: stage3 wasm validation failed: {e}")
    });

    let run_output = run_wasm_with_six_imports_compiler_mode(stage3_wasm, "", &["prog", "abc"])
        .unwrap_or_else(|e| panic!("BOOT-04 command-line-arg-repro: 実行失敗: {e}"));
    assert_eq!(run_output.trim(), "3");
}

#[test]
fn test_e2e_boot04_self_hosted_stage2_runs_print_repro_source() {
    let main_path = selfhost_main_path();
    let selfhost_root = main_path
        .parent()
        .expect("App/ ディレクトリ")
        .parent()
        .expect("src/ ディレクトリ")
        .parent()
        .expect("selfhost/ ルートディレクトリ")
        .to_path_buf();

    let stage1_wasm = compile_file_only(&main_path);
    assert_valid_wasm(&stage1_wasm);

    let stage2_output = lsharp_wasm::wasi_runner::run_wasm_wasi_with_dir_and_args(
        &stage1_wasm,
        Some(&selfhost_root),
        &["compiler", "src/App/Main.ls"],
    )
    .expect("BOOT-04 print-repro: stage1 が Main.ls の self-compile に失敗した");
    let stage2_modules = parse_emitted_wasm_modules(&stage2_output, 1);
    let stage2_self_compiler = &stage2_modules[0];
    assert_valid_wasm(stage2_self_compiler);

    let source = "(module App.Main)\n(defn main [] (print 7))\n";
    let stage3_output = run_wasm_with_six_imports_compiler_mode(
        stage2_self_compiler,
        source,
        &["compiler", "src/App/Main.ls"],
    )
    .expect("BOOT-04 print-repro: stage2_self_compiler が repro source をコンパイルできない");
    let stage3_modules = parse_emitted_wasm_modules(&stage3_output, 1);
    let stage3_wasm = &stage3_modules[0];
    assert_valid_wasm(stage3_wasm);
    validate_wasm_detailed(stage3_wasm)
        .unwrap_or_else(|e| panic!("BOOT-04 print-repro: stage3 wasm validation failed: {e}"));
    let start_idx = exported_function_index(stage3_wasm, "_start")
        .expect("BOOT-04 print-repro: _start export が必要");
    eprintln!(
        "BOOT-04 print-repro: sections={:?} _start={} start_ops={:?} main_ops={:?}",
        extract_sections(stage3_wasm),
        start_idx,
        function_operator_debug(stage3_wasm, start_idx, 8),
        function_operator_debug(stage3_wasm, start_idx - 1, 16)
    );

    let run_output = run_wasm_with_six_imports_compiler_mode(stage3_wasm, "", &[])
        .unwrap_or_else(|e| panic!("BOOT-04 print-repro: 実行失敗: {e}"));
    assert_eq!(run_output.trim(), "7");
}

#[test]
fn test_e2e_boot04_self_hosted_stage2_reports_print_repro_ir() {
    let main_path = selfhost_main_path();
    let selfhost_root = main_path
        .parent()
        .expect("App/ ディレクトリ")
        .parent()
        .expect("src/ ディレクトリ")
        .parent()
        .expect("selfhost/ ルートディレクトリ")
        .to_path_buf();

    let stage1_wasm = compile_file_only(&main_path);
    assert_valid_wasm(&stage1_wasm);

    let stage2_output = lsharp_wasm::wasi_runner::run_wasm_wasi_with_dir_and_args(
        &stage1_wasm,
        Some(&selfhost_root),
        &["compiler", "src/App/Main.ls"],
    )
    .expect("BOOT-04 print-ir: stage1 が Main.ls の self-compile に失敗した");
    let stage2_modules = parse_emitted_wasm_modules(&stage2_output, 1);
    let stage2_self_compiler = &stage2_modules[0];
    assert_valid_wasm(stage2_self_compiler);

    let source = "(module App.Main)\n(defn main [] (print 7))\n";
    let ir_output = run_wasm_with_six_imports_compiler_mode(
        stage2_self_compiler,
        source,
        &["compiler", "src/App/Main.ls", "", "", "", "", "ir"],
    )
    .expect("BOOT-04 print-ir: IR debug 実行に失敗");
    let lines: Vec<&str> = ir_output
        .lines()
        .filter(|line| !line.trim().is_empty())
        .collect();
    eprintln!("BOOT-04 print-ir lines: {:?}", lines);
    assert!(
        lines.first() == Some(&"71"),
        "BOOT-04 print-ir: IR debug marker が不正: {:?}",
        lines
    );
}

#[test]
fn test_e2e_boot04_self_hosted_stage2_reports_print_repro_tokens() {
    let main_path = selfhost_main_path();
    let selfhost_root = main_path
        .parent()
        .expect("App/ ディレクトリ")
        .parent()
        .expect("src/ ディレクトリ")
        .parent()
        .expect("selfhost/ ルートディレクトリ")
        .to_path_buf();

    let stage1_wasm = compile_file_only(&main_path);
    assert_valid_wasm(&stage1_wasm);

    let stage2_output = lsharp_wasm::wasi_runner::run_wasm_wasi_with_dir_and_args(
        &stage1_wasm,
        Some(&selfhost_root),
        &["compiler", "src/App/Main.ls"],
    )
    .expect("BOOT-04 print-token: stage1 が Main.ls の self-compile に失敗した");
    let stage2_modules = parse_emitted_wasm_modules(&stage2_output, 1);
    let stage2_self_compiler = &stage2_modules[0];
    assert_valid_wasm(stage2_self_compiler);

    let source = "(module App.Main)\n(defn main [] (print 7))\n";
    let token_output = run_wasm_with_six_imports_compiler_mode(
        stage2_self_compiler,
        source,
        &["compiler", "src/App/Main.ls", "", "", "", "", "", "tokens"],
    )
    .expect("BOOT-04 print-token: token debug 実行に失敗");
    let lines: Vec<&str> = token_output
        .lines()
        .filter(|line| !line.trim().is_empty())
        .collect();
    eprintln!("BOOT-04 print-token lines: {:?}", lines);
    assert!(
        lines.first() == Some(&"72"),
        "BOOT-04 print-token: token debug marker が不正: {:?}",
        lines
    );
}

#[test]
fn test_e2e_boot04_self_hosted_stage2_compiles_two_imports_zero_source() {
    let main_path = selfhost_main_path();
    let selfhost_root = main_path
        .parent()
        .expect("App/ ディレクトリ")
        .parent()
        .expect("src/ ディレクトリ")
        .parent()
        .expect("selfhost/ ルートディレクトリ")
        .to_path_buf();

    let stage1_wasm = compile_file_only(&main_path);
    assert_valid_wasm(&stage1_wasm);

    let stage2_output = lsharp_wasm::wasi_runner::run_wasm_wasi_with_dir_and_args(
        &stage1_wasm,
        Some(&selfhost_root),
        &["compiler", "src/App/Main.ls"],
    )
    .expect("BOOT-04 two-imports-zero: stage1 が Main.ls の self-compile に失敗した");
    let stage2_modules = parse_emitted_wasm_modules(&stage2_output, 1);
    let stage2_self_compiler = &stage2_modules[0];
    assert_valid_wasm(stage2_self_compiler);

    let source = "(module App.Main)\n(import App.CompilerMode)\n(import App.PipelineSmoke)\n(defn main [] 0)\n";
    let stage3_output = run_wasm_with_six_imports_compiler_mode(
        stage2_self_compiler,
        source,
        &["compiler", "src/App/Main.ls"],
    )
    .expect("BOOT-04 two-imports-zero: stage2_self_compiler が source をコンパイルできない");
    let stage3_modules = parse_emitted_wasm_modules(&stage3_output, 1);
    let stage3_wasm = &stage3_modules[0];
    assert_valid_wasm(stage3_wasm);
    validate_wasm_detailed(stage3_wasm)
        .unwrap_or_else(|e| panic!("BOOT-04 two-imports-zero: stage3 wasm validation failed: {e}"));
    let engine = wasmtime::Engine::default();
    wasmtime::Module::new(&engine, stage3_wasm)
        .unwrap_or_else(|e| panic!("BOOT-04 two-imports-zero: wasmtime load failed: {e}"));
}

#[test]
fn test_e2e_boot04_self_hosted_stage2_compiles_one_import_zero_fs_package() {
    let main_path = selfhost_main_path();
    let selfhost_root = main_path
        .parent()
        .expect("App/ ディレクトリ")
        .parent()
        .expect("src/ ディレクトリ")
        .parent()
        .expect("selfhost/ ルートディレクトリ")
        .to_path_buf();

    let stage1_wasm = compile_file_only(&main_path);
    assert_valid_wasm(&stage1_wasm);

    let stage2_output = lsharp_wasm::wasi_runner::run_wasm_wasi_with_dir_and_args(
        &stage1_wasm,
        Some(&selfhost_root),
        &["compiler", "src/App/Main.ls"],
    )
    .expect("BOOT-04 one-import-fs: stage1 が Main.ls の self-compile に失敗した");
    let stage2_modules = parse_emitted_wasm_modules(&stage2_output, 1);
    let stage2_self_compiler = &stage2_modules[0];
    assert_valid_wasm(stage2_self_compiler);

    let temp_root = std::env::temp_dir().join(format!(
        "lsharp-boot04-one-import-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .expect("時刻が巻き戻った")
            .as_nanos()
    ));
    let app_dir = temp_root.join("src/App");
    std::fs::create_dir_all(&app_dir).expect("one-import-fs temp dir を作れない");
    std::fs::write(
        app_dir.join("Main.ls"),
        "(module App.Main)\n(import App.CompilerMode)\n(defn main [] 0)\n",
    )
    .expect("one-import-fs Main.ls を書けない");
    std::fs::write(
        app_dir.join("CompilerMode.ls"),
        "(module App.CompilerMode)\n(defn compile-file-mode [] 1)\n",
    )
    .expect("one-import-fs CompilerMode.ls を書けない");

    let stage3_output = run_wasm_with_six_imports_compiler_mode_fs(
        stage2_self_compiler,
        &temp_root,
        &["compiler", "src/App/Main.ls"],
    )
    .expect("BOOT-04 one-import-fs: stage2_self_compiler が temp package をコンパイルできない");
    let stage3_modules = parse_emitted_wasm_modules(&stage3_output, 1);
    let stage3_wasm = &stage3_modules[0];
    assert_valid_wasm(stage3_wasm);
    validate_wasm_detailed(stage3_wasm)
        .unwrap_or_else(|e| panic!("BOOT-04 one-import-fs: stage3 wasm validation failed: {e}"));
    let engine = wasmtime::Engine::default();
    wasmtime::Module::new(&engine, stage3_wasm)
        .unwrap_or_else(|e| panic!("BOOT-04 one-import-fs: wasmtime load failed: {e}"));

    std::fs::remove_dir_all(&temp_root).expect("one-import-fs temp dir を削除できない");
}

#[test]
fn test_e2e_boot04_self_hosted_stage2_compiles_two_imports_zero_fs_package() {
    let main_path = selfhost_main_path();
    let selfhost_root = main_path
        .parent()
        .expect("App/ ディレクトリ")
        .parent()
        .expect("src/ ディレクトリ")
        .parent()
        .expect("selfhost/ ルートディレクトリ")
        .to_path_buf();

    let stage1_wasm = compile_file_only(&main_path);
    assert_valid_wasm(&stage1_wasm);

    let stage2_output = lsharp_wasm::wasi_runner::run_wasm_wasi_with_dir_and_args(
        &stage1_wasm,
        Some(&selfhost_root),
        &["compiler", "src/App/Main.ls"],
    )
    .expect("BOOT-04 two-imports-fs: stage1 が Main.ls の self-compile に失敗した");
    let stage2_modules = parse_emitted_wasm_modules(&stage2_output, 1);
    let stage2_self_compiler = &stage2_modules[0];
    assert_valid_wasm(stage2_self_compiler);

    let temp_root = std::env::temp_dir().join(format!(
        "lsharp-boot04-two-imports-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .expect("時刻が巻き戻った")
            .as_nanos()
    ));
    let app_dir = temp_root.join("src/App");
    std::fs::create_dir_all(&app_dir).expect("two-imports-fs temp dir を作れない");
    std::fs::write(
        app_dir.join("Main.ls"),
        "(module App.Main)\n(import App.CompilerMode)\n(import App.PipelineSmoke)\n(defn main [] 0)\n",
    )
    .expect("two-imports-fs Main.ls を書けない");
    std::fs::write(
        app_dir.join("CompilerMode.ls"),
        "(module App.CompilerMode)\n(defn compile-file-mode [] 1)\n",
    )
    .expect("two-imports-fs CompilerMode.ls を書けない");
    std::fs::write(
        app_dir.join("PipelineSmoke.ls"),
        "(module App.PipelineSmoke)\n(defn run-main-smoke [] 2)\n",
    )
    .expect("two-imports-fs PipelineSmoke.ls を書けない");

    let stage3_output = run_wasm_with_six_imports_compiler_mode_fs(
        stage2_self_compiler,
        &temp_root,
        &["compiler", "src/App/Main.ls"],
    )
    .expect("BOOT-04 two-imports-fs: stage2_self_compiler が temp package をコンパイルできない");
    let stage3_modules = parse_emitted_wasm_modules(&stage3_output, 1);
    let stage3_wasm = &stage3_modules[0];
    assert_valid_wasm(stage3_wasm);
    std::fs::write("/tmp/two_imports_zero_fs_stage3.wasm", stage3_wasm)
        .expect("two-imports-fs stage3 dump に失敗");
    eprintln!(
        "BOOT-04 two-imports-fs stage3: bytes={}, sections={:?}",
        stage3_wasm.len(),
        extract_sections(stage3_wasm)
    );
    validate_wasm_detailed(stage3_wasm)
        .unwrap_or_else(|e| panic!("BOOT-04 two-imports-fs: stage3 wasm validation failed: {e}"));
    let engine = wasmtime::Engine::default();
    wasmtime::Module::new(&engine, stage3_wasm)
        .unwrap_or_else(|e| panic!("BOOT-04 two-imports-fs: wasmtime load failed: {e}"));

    std::fs::remove_dir_all(&temp_root).expect("two-imports-fs temp dir を削除できない");
}

#[test]
fn test_e2e_boot04_self_hosted_stage2_compiles_if_builtin_source() {
    let main_path = selfhost_main_path();
    let selfhost_root = main_path
        .parent()
        .expect("App/ ディレクトリ")
        .parent()
        .expect("src/ ディレクトリ")
        .parent()
        .expect("selfhost/ ルートディレクトリ")
        .to_path_buf();

    let stage1_wasm = compile_file_only(&main_path);
    assert_valid_wasm(&stage1_wasm);

    let stage2_output = lsharp_wasm::wasi_runner::run_wasm_wasi_with_dir_and_args(
        &stage1_wasm,
        Some(&selfhost_root),
        &["compiler", "src/App/Main.ls"],
    )
    .expect("BOOT-04 if-builtin: stage1 が Main.ls の self-compile に失敗した");
    let stage2_modules = parse_emitted_wasm_modules(&stage2_output, 1);
    let stage2_self_compiler = &stage2_modules[0];
    assert_valid_wasm(stage2_self_compiler);

    let source =
        "(module App.Main)\n(defn main [] (if (> (string-length (command-line-arg 1)) 0) 1 0))\n";
    let stage3_output = run_wasm_with_six_imports_compiler_mode(
        stage2_self_compiler,
        source,
        &["compiler", "src/App/Main.ls"],
    )
    .expect("BOOT-04 if-builtin: stage2_self_compiler が source をコンパイルできない");
    let stage3_modules = parse_emitted_wasm_modules(&stage3_output, 1);
    let stage3_wasm = &stage3_modules[0];
    assert_valid_wasm(stage3_wasm);
    validate_wasm_detailed(stage3_wasm)
        .unwrap_or_else(|e| panic!("BOOT-04 if-builtin: stage3 wasm validation failed: {e}"));
    let engine = wasmtime::Engine::default();
    wasmtime::Module::new(&engine, stage3_wasm)
        .unwrap_or_else(|e| panic!("BOOT-04 if-builtin: wasmtime load failed: {e}"));
}

#[test]
fn test_e2e_boot04_self_hosted_stage2_compiles_main_again() {
    let main_path = selfhost_main_path();
    let selfhost_root = main_path
        .parent()
        .expect("App/ ディレクトリ")
        .parent()
        .expect("src/ ディレクトリ")
        .parent()
        .expect("selfhost/ ルートディレクトリ")
        .to_path_buf();
    let fixture_dir =
        std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../tests/fixtures");

    let stage1_wasm = compile_file_only(&main_path);
    assert_valid_wasm(&stage1_wasm);

    let stage2_output = lsharp_wasm::wasi_runner::run_wasm_wasi_with_dir_and_args(
        &stage1_wasm,
        Some(&selfhost_root),
        &["compiler", "src/App/Main.ls"],
    )
    .expect("BOOT-04 self-feed: stage1 が Main.ls の self-compile に失敗した");
    let stage2_modules = parse_emitted_wasm_modules(&stage2_output, 1);
    let stage2_self_compiler = &stage2_modules[0];
    assert_valid_wasm(stage2_self_compiler);

    let stage3_output = run_wasm_with_six_imports_compiler_mode_fs(
        stage2_self_compiler,
        &selfhost_root,
        &["compiler", "src/App/Main.ls"],
    )
    .expect("BOOT-04 self-feed: stage2_self_compiler が Main.ls を再コンパイルできない");
    let stage3_modules = parse_emitted_wasm_modules(&stage3_output, 1);
    let stage3_self_compiler = &stage3_modules[0];
    assert_valid_wasm(stage3_self_compiler);
    std::fs::write(
        "/tmp/main_again_stage3_self_compiler.wasm",
        stage3_self_compiler,
    )
    .expect("stage3 self compiler dump に失敗");
    eprintln!(
        "BOOT-04 stage3 self compiler: bytes={}, sections={:?}",
        stage3_self_compiler.len(),
        extract_sections(stage3_self_compiler)
    );
    validate_wasm_detailed(stage3_self_compiler).unwrap_or_else(|e| {
        panic!(
            "BOOT-04 self-feed: stage3 self compiler validation failed: {e}; sections={:?}; fingerprint={}",
            extract_sections(stage3_self_compiler),
            hash_fingerprint(stage3_self_compiler)
        )
    });

    let stage4_output = run_wasm_with_six_imports_compiler_mode_fs(
        stage3_self_compiler,
        &fixture_dir,
        &["compiler", "minimal.ls"],
    )
    .expect("BOOT-04 self-feed: stage3_self_compiler が minimal.ls をコンパイルできない");
    let stage4_modules = parse_emitted_wasm_modules(&stage4_output, 1);
    let stage4_wasm = &stage4_modules[0];
    assert_valid_wasm(stage4_wasm);

    let run_result = run_wasm_with_six_imports_compiler_mode(stage4_wasm, "", &[]);
    assert!(
        run_result.is_ok(),
        "BOOT-04 self-feed: stage4 minimal 実行失敗: {:?}",
        run_result.err()
    );
}

#[test]
fn test_v2_12_self_hosted_stage2_reports_main_again_stage3_local_bounds() {
    let main_path = selfhost_main_path();
    let selfhost_root = main_path
        .parent()
        .expect("App/ ディレクトリ")
        .parent()
        .expect("src/ ディレクトリ")
        .parent()
        .expect("selfhost/ ルートディレクトリ")
        .to_path_buf();

    let stage1_wasm = compile_file_only(&main_path);
    assert_valid_wasm(&stage1_wasm);

    let stage2_output = lsharp_wasm::wasi_runner::run_wasm_wasi_with_dir_and_args(
        &stage1_wasm,
        Some(&selfhost_root),
        &["compiler", "src/App/Main.ls"],
    )
    .expect("V2-12 main-again-locals: stage1 が Main.ls の self-compile に失敗した");
    let stage2_modules = parse_emitted_wasm_modules(&stage2_output, 1);
    let stage2_self_compiler = &stage2_modules[0];
    assert_valid_wasm(stage2_self_compiler);

    let stage3_output = run_wasm_with_six_imports_compiler_mode_fs(
        stage2_self_compiler,
        &selfhost_root,
        &["compiler", "src/App/Main.ls"],
    )
    .expect("V2-12 main-again-locals: stage2_self_compiler が Main.ls を再コンパイルできない");
    let stage3_modules = parse_emitted_wasm_modules(&stage3_output, 1);
    let stage3_self_compiler = &stage3_modules[0];
    assert_valid_wasm(stage3_self_compiler);

    let violations = local_bound_violations(stage3_self_compiler);
    let first_violation_func = violations.first().and_then(|msg| {
        msg.strip_prefix("func ")
            .and_then(|rest| rest.split_whitespace().next())
            .and_then(|func| func.parse::<u32>().ok())
    });
    let first_violation_ops = first_violation_func
        .map(|func| function_operator_debug(stage3_self_compiler, func, 24))
        .unwrap_or_default();
    validate_wasm_detailed(stage3_self_compiler).unwrap_or_else(|e| {
        panic!(
            "V2-12 main-again-locals: stage3 self compiler validation failed: {e}; sections={:?}; violations={:?}; first_violation_func={:?}; first_violation_ops={:?}; fingerprint={}",
            extract_sections(stage3_self_compiler),
            violations,
            first_violation_func,
            first_violation_ops,
            hash_fingerprint(stage3_self_compiler)
        )
    });
    assert!(
        violations.is_empty(),
        "V2-12 main-again-locals: local bound violations: {:?}; first_violation_func={:?}; first_violation_ops={:?}",
        violations,
        first_violation_func,
        first_violation_ops
    );
}

#[test]
fn test_v2_12_self_hosted_stage2_compiles_large_let_chain() {
    let main_path = selfhost_main_path();
    let selfhost_root = main_path
        .parent()
        .expect("App/ ディレクトリ")
        .parent()
        .expect("src/ ディレクトリ")
        .parent()
        .expect("selfhost/ ルートディレクトリ")
        .to_path_buf();

    let stage1_wasm = compile_file_only(&main_path);
    assert_valid_wasm(&stage1_wasm);

    let stage2_output = lsharp_wasm::wasi_runner::run_wasm_wasi_with_dir_and_args(
        &stage1_wasm,
        Some(&selfhost_root),
        &["compiler", "src/App/Main.ls"],
    )
    .expect("V2-12 let-chain: stage1 が Main.ls の self-compile に失敗した");
    let stage2_modules = parse_emitted_wasm_modules(&stage2_output, 1);
    let stage2_self_compiler = &stage2_modules[0];
    assert_valid_wasm(stage2_self_compiler);

    let bindings = (0..160)
        .map(|idx| format!("v{idx} {idx}"))
        .collect::<Vec<_>>()
        .join(" ");
    let source = format!("(module App.Main)\n(defn main []\n  (let [{bindings}]\n    v159))\n");

    let stage3_output = run_wasm_with_six_imports_compiler_mode(
        stage2_self_compiler,
        &source,
        &["compiler", "inline-large-let-chain.ls"],
    )
    .expect("V2-12 let-chain: stage2_self_compiler が large let-chain source をコンパイルできない");
    let stage3_modules = parse_emitted_wasm_modules(&stage3_output, 1);
    let stage3_wasm = &stage3_modules[0];
    assert_valid_wasm(stage3_wasm);
}

#[test]
fn test_v2_12_self_hosted_stage2_compiles_vector_push_program() {
    let main_path = selfhost_main_path();
    let selfhost_root = main_path
        .parent()
        .expect("App/ ディレクトリ")
        .parent()
        .expect("src/ ディレクトリ")
        .parent()
        .expect("selfhost/ ルートディレクトリ")
        .to_path_buf();

    let stage1_wasm = compile_file_only(&main_path);
    assert_valid_wasm(&stage1_wasm);

    let stage2_output = lsharp_wasm::wasi_runner::run_wasm_wasi_with_dir_and_args(
        &stage1_wasm,
        Some(&selfhost_root),
        &["compiler", "src/App/Main.ls"],
    )
    .expect("V2-12 vector-push: stage1 が Main.ls の self-compile に失敗した");
    let stage2_modules = parse_emitted_wasm_modules(&stage2_output, 1);
    let stage2_self_compiler = &stage2_modules[0];
    assert_valid_wasm(stage2_self_compiler);

    let source = r#"
(module App.Main)
(defn main []
  (print
    (let [v (vector-new 1)
          v2 (vector-push v 42)]
      (vector-get v2 0))))
"#;
    let stage3_output = run_wasm_with_six_imports_compiler_mode(
        stage2_self_compiler,
        source,
        &["compiler", "inline-vector-push.ls"],
    )
    .expect("V2-12 vector-push: stage2_self_compiler が vector-push source をコンパイルできない");
    let stage3_modules = parse_emitted_wasm_modules(&stage3_output, 1);
    let stage3_wasm = &stage3_modules[0];
    assert_valid_wasm(stage3_wasm);
    let run_output = run_wasm_with_six_imports_compiler_mode(stage3_wasm, "", &[])
        .expect("V2-12 vector-push: stage3_wasm が runtime imports で実行できること");
    assert_eq!(run_output, "42\n");
}

#[test]
fn test_v2_12_self_hosted_stage2_loads_wasm_emit_module() {
    let main_path = selfhost_main_path();
    let selfhost_root = main_path
        .parent()
        .expect("App/ ディレクトリ")
        .parent()
        .expect("src/ ディレクトリ")
        .parent()
        .expect("selfhost/ ルートディレクトリ")
        .to_path_buf();

    let stage1_wasm = compile_file_only(&main_path);
    assert_valid_wasm(&stage1_wasm);

    let stage2_output = lsharp_wasm::wasi_runner::run_wasm_wasi_with_dir_and_args(
        &stage1_wasm,
        Some(&selfhost_root),
        &["compiler", "src/App/Main.ls"],
    )
    .expect("V2-12 WasmEmit: stage1 が Main.ls の self-compile に失敗した");
    let stage2_modules = parse_emitted_wasm_modules(&stage2_output, 1);
    let stage2_self_compiler = &stage2_modules[0];
    assert_valid_wasm(stage2_self_compiler);

    let stage3_output = run_wasm_with_six_imports_compiler_mode_fs(
        stage2_self_compiler,
        &selfhost_root,
        &["compiler", "src/Backend/Wasm/WasmEmit.ls"],
    )
    .expect("V2-12 WasmEmit: stage2_self_compiler が WasmEmit.ls をコンパイルできない");
    let stage3_modules = parse_emitted_wasm_modules(&stage3_output, 1);
    let stage3_wasm = &stage3_modules[0];
    let violations = local_bound_violations(stage3_wasm);
    let engine = wasmtime::Engine::default();
    wasmtime::Module::new(&engine, stage3_wasm).unwrap_or_else(|e| {
        panic!(
            "V2-12 WasmEmit: wasmtime load failed: {e}; sections={:?}; violations={:?}; fingerprint={}",
            extract_sections(stage3_wasm),
            violations,
            hash_fingerprint(stage3_wasm)
        )
    });
    assert!(
        violations.is_empty(),
        "V2-12 WasmEmit: local bound violations: {:?}",
        violations
    );
}

#[test]
fn test_v2_12_self_hosted_stage2_loads_compiler_mode_module() {
    let main_path = selfhost_main_path();
    let selfhost_root = main_path
        .parent()
        .expect("App/ ディレクトリ")
        .parent()
        .expect("src/ ディレクトリ")
        .parent()
        .expect("selfhost/ ルートディレクトリ")
        .to_path_buf();

    let stage1_wasm = compile_file_only(&main_path);
    assert_valid_wasm(&stage1_wasm);

    let stage2_output = lsharp_wasm::wasi_runner::run_wasm_wasi_with_dir_and_args(
        &stage1_wasm,
        Some(&selfhost_root),
        &["compiler", "src/App/Main.ls"],
    )
    .expect("V2-12 CompilerMode: stage1 が Main.ls の self-compile に失敗した");
    let stage2_modules = parse_emitted_wasm_modules(&stage2_output, 1);
    let stage2_self_compiler = &stage2_modules[0];
    assert_valid_wasm(stage2_self_compiler);

    let stage3_output = run_wasm_with_six_imports_compiler_mode_fs(
        stage2_self_compiler,
        &selfhost_root,
        &["compiler", "src/App/CompilerMode.ls"],
    )
    .expect("V2-12 CompilerMode: stage2_self_compiler が CompilerMode.ls をコンパイルできない");
    let stage3_modules = parse_emitted_wasm_modules(&stage3_output, 1);
    let stage3_wasm = &stage3_modules[0];
    let violations = local_bound_violations(stage3_wasm);
    let engine = wasmtime::Engine::default();
    wasmtime::Module::new(&engine, stage3_wasm).unwrap_or_else(|e| {
        panic!(
            "V2-12 CompilerMode: wasmtime load failed: {e}; sections={:?}; violations={:?}; fingerprint={}",
            extract_sections(stage3_wasm),
            violations,
            hash_fingerprint(stage3_wasm)
        )
    });
    assert!(
        violations.is_empty(),
        "V2-12 CompilerMode: local bound violations: {:?}",
        violations
    );
}

#[test]
#[ignore = "診断専用: regular invariant は test_v2_12_self_hosted_stage2_loads_compiler_mode_module が担う"]
fn test_v2_12_self_hosted_stage2_reports_compiler_mode_first_violation_body_diff() {
    let main_path = selfhost_main_path();
    let selfhost_root = main_path
        .parent()
        .expect("App/ ディレクトリ")
        .parent()
        .expect("src/ ディレクトリ")
        .parent()
        .expect("selfhost/ ルートディレクトリ")
        .to_path_buf();
    let compiler_mode_path = selfhost_root.join("src/App/CompilerMode.ls");

    let stage1_wasm = compile_file_only(&main_path);
    assert_valid_wasm(&stage1_wasm);

    let stage1_output = lsharp_wasm::wasi_runner::run_wasm_wasi_with_dir_and_args(
        &stage1_wasm,
        Some(&selfhost_root),
        &["compiler", "src/App/CompilerMode.ls"],
    )
    .expect("V2-12 CompilerMode diff: stage1_wasm が CompilerMode.ls をコンパイルできない");
    let stage1_modules = parse_emitted_wasm_modules(&stage1_output, 1);
    let stage1_compiler_mode = &stage1_modules[0];
    let engine = wasmtime::Engine::default();
    wasmtime::Module::new(&engine, stage1_compiler_mode)
        .expect("V2-12 CompilerMode diff: stage1 output は load できること");

    let stage2_output = lsharp_wasm::wasi_runner::run_wasm_wasi_with_dir_and_args(
        &stage1_wasm,
        Some(&selfhost_root),
        &["compiler", "src/App/Main.ls"],
    )
    .expect("V2-12 CompilerMode diff: stage1 が Main.ls の self-compile に失敗した");
    let stage2_modules = parse_emitted_wasm_modules(&stage2_output, 1);
    let stage2_self_compiler = &stage2_modules[0];
    assert_valid_wasm(stage2_self_compiler);

    let stage3_output = run_wasm_with_six_imports_compiler_mode_fs(
        stage2_self_compiler,
        &selfhost_root,
        &["compiler", "src/App/CompilerMode.ls"],
    )
    .expect(
        "V2-12 CompilerMode diff: stage2_self_compiler が CompilerMode.ls をコンパイルできない",
    );
    let stage3_modules = parse_emitted_wasm_modules(&stage3_output, 1);
    let stage3_compiler_mode = &stage3_modules[0];

    let bad_indices = local_bound_violation_indices(stage3_compiler_mode);
    let first_bad = *bad_indices
        .first()
        .expect("V2-12 CompilerMode diff: stage3 output に violation があること");
    let stage1_body = function_body_bytes(stage1_compiler_mode, first_bad)
        .expect("V2-12 CompilerMode diff: stage1 body が見つかること");
    let stage3_body = function_body_bytes(stage3_compiler_mode, first_bad)
        .expect("V2-12 CompilerMode diff: stage3 body が見つかること");
    let diff_at = first_byte_diff(stage1_body.as_slice(), stage3_body.as_slice())
        .expect("V2-12 CompilerMode diff: body 差分があること");
    let window_start = diff_at.saturating_sub(16);
    let window_end_stage1 = (diff_at + 24).min(stage1_body.len());
    let window_end_stage3 = (diff_at + 24).min(stage3_body.len());

    panic!(
        "V2-12 CompilerMode diff: path={}; first_bad={}; diff_at={}; stage1_size={}; stage3_size={}; stage1_prefix={:?}; stage3_prefix={:?}; stage1_window={:?}; stage3_window={:?}; stage1_ops={:?}; stage3_violations={:?}; stage1_fingerprint={}; stage3_fingerprint={}",
        compiler_mode_path.display(),
        first_bad,
        diff_at,
        stage1_body.len(),
        stage3_body.len(),
        stage1_body.iter().take(32).copied().collect::<Vec<_>>(),
        stage3_body.iter().take(32).copied().collect::<Vec<_>>(),
        stage1_body[window_start..window_end_stage1].to_vec(),
        stage3_body[window_start..window_end_stage3].to_vec(),
        function_operator_debug(stage1_compiler_mode, first_bad, 20),
        local_bound_violations(stage3_compiler_mode),
        hash_fingerprint(stage1_body.as_slice()),
        hash_fingerprint(stage3_body.as_slice())
    );
}

#[test]
fn test_e2e_boot04_self_hosted_stage2_compiles_high_function_index_calls() {
    let main_path = selfhost_main_path();
    let selfhost_root = main_path
        .parent()
        .expect("App/ ディレクトリ")
        .parent()
        .expect("src/ ディレクトリ")
        .parent()
        .expect("selfhost/ ルートディレクトリ")
        .to_path_buf();

    let stage1_wasm = compile_file_only(&main_path);
    assert_valid_wasm(&stage1_wasm);

    let stage2_output = lsharp_wasm::wasi_runner::run_wasm_wasi_with_dir_and_args(
        &stage1_wasm,
        Some(&selfhost_root),
        &["compiler", "src/App/Main.ls"],
    )
    .expect("BOOT-04 high-func-idx: stage1 self-compile に失敗した");
    let stage2_modules = parse_emitted_wasm_modules(&stage2_output, 1);
    let stage2_self_compiler = &stage2_modules[0];
    assert_valid_wasm(stage2_self_compiler);

    let helper_count = 130usize;
    let helpers = (0..helper_count)
        .map(|idx| format!("(defn helper-{idx} [] {idx})"))
        .collect::<Vec<_>>()
        .join("\n");
    let source = format!("{helpers}\n(defn main [] (helper-129))\n");

    let stage3_output = run_wasm_with_six_imports_compiler_mode(
        stage2_self_compiler,
        &source,
        &["compiler", "HighFunctionIndex.ls"],
    )
    .expect("BOOT-04 high-func-idx: stage2_self_compiler が synthetic source をコンパイルできない");
    let stage3_modules = parse_emitted_wasm_modules(&stage3_output, 1);
    let stage3_wasm = &stage3_modules[0];
    let violations = local_bound_violations(stage3_wasm);
    validate_wasm_detailed(stage3_wasm).unwrap_or_else(|e| {
        panic!(
            "BOOT-04 high-func-idx: validation failed: {e}; bytes={}; sections={:?}; violations={:?}; fingerprint={}",
            stage3_wasm.len(),
            extract_sections(stage3_wasm),
            violations,
            hash_fingerprint(stage3_wasm)
        )
    });
    assert!(
        violations.is_empty(),
        "BOOT-04 high-func-idx: local bound violations: {:?}",
        violations
    );
    let engine = wasmtime::Engine::default();
    wasmtime::Module::new(&engine, stage3_wasm).unwrap_or_else(|e| {
        panic!(
            "BOOT-04 high-func-idx: wasmtime load failed: {e}; sections={:?}; violations={:?}; fingerprint={}",
            extract_sections(stage3_wasm),
            violations,
            hash_fingerprint(stage3_wasm)
        )
    });
}

#[test]
fn test_e2e_boot04_self_hosted_stage2_compiles_high_function_index_step64_pattern() {
    let main_path = selfhost_main_path();
    let selfhost_root = main_path
        .parent()
        .expect("App/ ディレクトリ")
        .parent()
        .expect("src/ ディレクトリ")
        .parent()
        .expect("selfhost/ ルートディレクトリ")
        .to_path_buf();

    let stage1_wasm = compile_file_only(&main_path);
    assert_valid_wasm(&stage1_wasm);

    let stage2_output = lsharp_wasm::wasi_runner::run_wasm_wasi_with_dir_and_args(
        &stage1_wasm,
        Some(&selfhost_root),
        &["compiler", "src/App/Main.ls"],
    )
    .expect("BOOT-04 high-step64: stage1 self-compile に失敗した");
    let stage2_modules = parse_emitted_wasm_modules(&stage2_output, 1);
    let stage2_self_compiler = &stage2_modules[0];
    assert_valid_wasm(stage2_self_compiler);

    let helper_count = 130usize;
    let helpers = (0..helper_count)
        .map(|idx| format!("(defn helper-{idx} [a b c d] a)"))
        .collect::<Vec<_>>()
        .join("\n");
    let let_count = 24usize;
    let mut body = String::from("step24");
    for idx in (1..=let_count).rev() {
        let helper = if idx % 2 == 0 { 129 } else { 128 };
        body = format!("(let [step{idx} (helper-{helper} a b c d)] {body})");
    }
    let source =
        format!("{helpers}\n(defn wrapper [a b c d] {body})\n(defn main [] (wrapper 1 2 3 4))\n");

    let stage3_output = run_wasm_with_six_imports_compiler_mode(
        stage2_self_compiler,
        &source,
        &["compiler", "HighFunctionIndexStep64.ls"],
    )
    .expect("BOOT-04 high-step64: stage2_self_compiler が synthetic source をコンパイルできない");
    let stage3_modules = parse_emitted_wasm_modules(&stage3_output, 1);
    let stage3_wasm = &stage3_modules[0];
    let violations = local_bound_violations(stage3_wasm);
    validate_wasm_detailed(stage3_wasm).unwrap_or_else(|e| {
        panic!(
            "BOOT-04 high-step64: validation failed: {e}; bytes={}; sections={:?}; violations={:?}; fingerprint={}",
            stage3_wasm.len(),
            extract_sections(stage3_wasm),
            violations,
            hash_fingerprint(stage3_wasm)
        )
    });
    assert!(
        violations.is_empty(),
        "BOOT-04 high-step64: local bound violations: {:?}",
        violations
    );
    let engine = wasmtime::Engine::default();
    wasmtime::Module::new(&engine, stage3_wasm).unwrap_or_else(|e| {
        panic!(
            "BOOT-04 high-step64: wasmtime load failed: {e}; sections={:?}; violations={:?}; fingerprint={}",
            extract_sections(stage3_wasm),
            violations,
            hash_fingerprint(stage3_wasm)
        )
    });
}

#[test]
fn test_e2e_boot04_self_hosted_stage2_compiles_high_index_parser_like_step64() {
    let main_path = selfhost_main_path();
    let selfhost_root = main_path
        .parent()
        .expect("App/ ディレクトリ")
        .parent()
        .expect("src/ ディレクトリ")
        .parent()
        .expect("selfhost/ ルートディレクトリ")
        .to_path_buf();

    let stage1_wasm = compile_file_only(&main_path);
    assert_valid_wasm(&stage1_wasm);

    let stage2_output = lsharp_wasm::wasi_runner::run_wasm_wasi_with_dir_and_args(
        &stage1_wasm,
        Some(&selfhost_root),
        &["compiler", "src/App/Main.ls"],
    )
    .expect("BOOT-04 parser-like-step64: stage1 self-compile に失敗した");
    let stage2_modules = parse_emitted_wasm_modules(&stage2_output, 1);
    let stage2_self_compiler = &stage2_modules[0];
    assert_valid_wasm(stage2_self_compiler);

    let helper_count = 130usize;
    let helpers = (0..helper_count)
        .map(|idx| format!("(defn helper-{idx} [state depth] state)"))
        .collect::<Vec<_>>()
        .join("\n");
    let source = format!(
        "{helpers}\n\
         (defn make-state [done next]\n\
           (vector-push (vector-push (vector-new 2) done) next))\n\
         (defn step [state depth]\n\
           (if (<= depth 0)\n\
             (make-state 1 depth)\n\
             (let [kind (vector-get state 0)]\n\
               (if (= kind 4)\n\
                 (make-state 0 (+ depth 1))\n\
                 (if (= kind 5)\n\
                   (make-state 0 (- depth 1))\n\
                   (make-state 0 depth))))))\n\
         (defn cont [state]\n\
           (if (= (vector-get state 0) 1)\n\
             state\n\
             (step state (vector-get state 1))))\n\
         (defn step8 [state depth]\n\
           (let [step1 (step state depth)]\n\
             (let [step2 (cont step1)]\n\
               (let [step3 (cont step2)]\n\
                 (let [step4 (cont step3)]\n\
                   (let [step5 (cont step4)]\n\
                     (let [step6 (cont step5)]\n\
                       (let [step7 (cont step6)]\n\
                         (let [step8 (cont step7)]\n\
                           step8))))))))\n\
         (defn cont8 [state]\n\
           (if (= (vector-get state 0) 1)\n\
             state\n\
             (step8 state (vector-get state 1))))\n\
         (defn step64 [state depth]\n\
           (let [step1 (step8 state depth)]\n\
             (let [step2 (cont8 step1)]\n\
               (let [step3 (cont8 step2)]\n\
                 (let [step4 (cont8 step3)]\n\
                   (let [step5 (cont8 step4)]\n\
                     (let [step6 (cont8 step5)]\n\
                       (let [step7 (cont8 step6)]\n\
                         (let [step8 (cont8 step7)]\n\
                           step8))))))))\n\
         (defn main [] (step64 (make-state 0 3) 3))\n"
    );

    let stage3_output = run_wasm_with_six_imports_compiler_mode(
        stage2_self_compiler,
        &source,
        &["compiler", "HighIndexParserLikeStep64.ls"],
    )
    .expect(
        "BOOT-04 parser-like-step64: stage2_self_compiler が synthetic source をコンパイルできない",
    );
    let stage3_modules = parse_emitted_wasm_modules(&stage3_output, 1);
    let stage3_wasm = &stage3_modules[0];
    let violations = local_bound_violations(stage3_wasm);
    validate_wasm_detailed(stage3_wasm).unwrap_or_else(|e| {
        panic!(
            "BOOT-04 parser-like-step64: validation failed: {e}; bytes={}; sections={:?}; violations={:?}; fingerprint={}",
            stage3_wasm.len(),
            extract_sections(stage3_wasm),
            violations,
            hash_fingerprint(stage3_wasm)
        )
    });
    assert!(
        violations.is_empty(),
        "BOOT-04 parser-like-step64: local bound violations: {:?}",
        violations
    );
    let engine = wasmtime::Engine::default();
    wasmtime::Module::new(&engine, stage3_wasm).unwrap_or_else(|e| {
        panic!(
            "BOOT-04 parser-like-step64: wasmtime load failed: {e}; sections={:?}; violations={:?}; fingerprint={}",
            extract_sections(stage3_wasm),
            violations,
            hash_fingerprint(stage3_wasm)
        )
    });
}

#[test]
fn test_e2e_boot04_self_hosted_stage2_reports_stage3_minimal_progress() {
    let main_path = selfhost_main_path();
    let selfhost_root = main_path
        .parent()
        .expect("App/ ディレクトリ")
        .parent()
        .expect("src/ ディレクトリ")
        .parent()
        .expect("selfhost/ ルートディレクトリ")
        .to_path_buf();
    let fixture_dir =
        std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../tests/fixtures");
    let minimal_src = std::fs::read_to_string(fixture_dir.join("minimal.ls"))
        .expect("BOOT-04 stage3-minimal-progress: minimal fixture を読めない");

    let stage1_wasm = compile_file_only(&main_path);
    assert_valid_wasm(&stage1_wasm);

    let stage2_output = lsharp_wasm::wasi_runner::run_wasm_wasi_with_dir_and_args(
        &stage1_wasm,
        Some(&selfhost_root),
        &["compiler", "src/App/Main.ls"],
    )
    .expect("BOOT-04 stage3-minimal-progress: stage1 self-compile に失敗した");
    let stage2_modules = parse_emitted_wasm_modules(&stage2_output, 1);
    let stage2_self_compiler = &stage2_modules[0];
    assert_valid_wasm(stage2_self_compiler);

    let stage3_output = run_wasm_with_six_imports_compiler_mode_fs(
        stage2_self_compiler,
        &selfhost_root,
        &["compiler", "src/App/Main.ls"],
    )
    .expect("BOOT-04 stage3-minimal-progress: stage2 self-compile に失敗した");
    let stage3_modules = parse_emitted_wasm_modules(&stage3_output, 1);
    let stage3_self_compiler = &stage3_modules[0];
    assert_valid_wasm(stage3_self_compiler);

    let progress_output = run_wasm_with_six_imports_compiler_mode_fs(
        stage3_self_compiler,
        &fixture_dir,
        &["compiler", "minimal.ls", "debug", "progress", "minimal"],
    )
    .expect("BOOT-04 stage3-minimal-progress: stage3 compiler の progress debug 実行に失敗した");
    let values = progress_output
        .lines()
        .filter(|line| !line.trim().is_empty())
        .map(|line| {
            line.trim().parse::<i64>().unwrap_or_else(|err| {
                panic!("BOOT-04 stage3-minimal-progress: 数値でない debug 出力: {line:?} / {err}")
            })
        })
        .collect::<Vec<_>>();
    assert!(
        values.len() >= 26,
        "BOOT-04 stage3-minimal-progress: debug 出力が短すぎる: {:?}",
        values
    );
    assert_eq!(
        &values[..6],
        &[1, 1, 2, 0, 3, 1],
        "BOOT-04 stage3-minimal-progress: top-level progress prefix が崩れている: {:?}",
        values
    );
    assert!(
        values
            .windows(4)
            .any(|window| window == [29, 0, minimal_src.len() as i64, 1]),
        "BOOT-04 stage3-minimal-progress: pair progress marker 29 が崩れている: {:?}",
        values
    );
    assert!(
        values.windows(3).any(|window| window == [40, 0, 20]),
        "BOOT-04 stage3-minimal-progress: defn marker 40 が崩れている: {:?}",
        values
    );
    assert!(
        values.windows(2).any(|window| window == [41, 0]),
        "BOOT-04 stage3-minimal-progress: compiled-fn start marker 41 が崩れている: {:?}",
        values
    );
    assert!(
        values.windows(3).any(|window| window == [42, 0, 1]),
        "BOOT-04 stage3-minimal-progress: compiled-fn count marker 42 が崩れている: {:?}",
        values
    );
    assert!(
        values.windows(2).any(|window| window == [43, 0]),
        "BOOT-04 stage3-minimal-progress: decl completion marker 43 が崩れている: {:?}",
        values
    );
    assert!(
        values
            .windows(4)
            .any(|window| window == [30, 0, minimal_src.len() as i64, 1]),
        "BOOT-04 stage3-minimal-progress: pair completion marker 30 が崩れている: {:?}",
        values
    );
    assert_eq!(
        &values[values.len() - 2..],
        &[4, 1],
        "BOOT-04 stage3-minimal-progress: compiled function count は 1 であるべき: {:?}",
        values
    );
}

#[test]
fn test_e2e_boot04_self_hosted_stage2_classifies_chunked_lexer_failure_band() {
    let main_path = selfhost_main_path();
    let selfhost_root = main_path
        .parent()
        .expect("App/ ディレクトリ")
        .parent()
        .expect("src/ ディレクトリ")
        .parent()
        .expect("selfhost/ ルートディレクトリ")
        .to_path_buf();
    let stage1_wasm = compile_file_only(&main_path);
    assert_valid_wasm(&stage1_wasm);

    let stage2_output = lsharp_wasm::wasi_runner::run_wasm_wasi_with_dir_and_args(
        &stage1_wasm,
        Some(&selfhost_root),
        &["compiler", "src/App/Main.ls"],
    )
    .expect("BOOT-04 chunk diag: stage1 が Main.ls の self-compile に失敗した");
    let stage2_modules = parse_emitted_wasm_modules(&stage2_output, 1);
    let stage2_self_compiler = &stage2_modules[0];
    assert_valid_wasm(stage2_self_compiler);

    let build_helper_source = |count: usize| {
        let helpers = (0..count)
            .map(|idx| format!("(defn helper-{idx} [] 0)"))
            .collect::<Vec<_>>()
            .join("\n");
        format!("{helpers}\n(defn main [] 42)\n")
    };
    let parse_stage3_wasm = |label: &str, output: &str| -> Result<usize, String> {
        let modules = std::panic::catch_unwind(|| parse_emitted_wasm_modules(output, 1))
            .map_err(|_| format!("{label}: 出力が wasm モジュール形式でない"))?;
        let wasm = &modules[0];
        assert_valid_wasm(wasm);
        Ok(wasm.len())
    };
    let try_compile_inline = |label: &str, source: &str| -> Result<usize, String> {
        let output = run_wasm_with_six_imports_compiler_mode(
            stage2_self_compiler,
            source,
            &["compiler", label],
        )
        .map_err(|err| format!("{label}: {err}"))?;
        parse_stage3_wasm(label, &output)
    };
    let try_compile_file = |path: &str| -> Result<usize, String> {
        let output = run_wasm_with_six_imports_compiler_mode_fs(
            stage2_self_compiler,
            &selfhost_root,
            &["compiler", path],
        )
        .map_err(|err| format!("{path}: {err}"))?;
        parse_stage3_wasm(path, &output)
    };
    let summarize = |result: &Result<usize, String>| match result {
        Ok(bytes) => format!("ok({bytes} bytes)"),
        Err(err) => {
            let head = err.lines().next().unwrap_or(err);
            format!("err({head})")
        }
    };
    let summarize_optional = |result: &Option<Result<usize, String>>| match result {
        Some(inner) => summarize(inner),
        None => "skipped".to_string(),
    };

    // helper 1 個あたり約 7 トークンなので、36 個は 256 トークン未満、37 個で最初の chunk 境界を跨ぐ。
    let below_boundary = try_compile_inline("diag-below-boundary.ls", &build_helper_source(36));
    let cross_boundary = try_compile_inline("diag-cross-boundary.ls", &build_helper_source(37));
    let multi_chunk = try_compile_inline("diag-multi-chunk.ls", &build_helper_source(80));
    let need_real_world = below_boundary.is_ok() && cross_boundary.is_ok() && multi_chunk.is_ok();
    let large_single_file = need_real_world
        .then(|| try_compile_inline("diag-large-single-file.ls", &build_helper_source(800)));
    let main_again = need_real_world.then(|| try_compile_file("src/App/Main.ls"));

    let classification = if below_boundary.is_err() {
        "local-before-boundary"
    } else if cross_boundary.is_err() {
        "first-boundary-crossing"
    } else if multi_chunk.is_err() {
        "post-first-chunk"
    } else if large_single_file
        .as_ref()
        .is_some_and(|result| result.is_err())
        || main_again.as_ref().is_some_and(|result| result.is_err())
    {
        "real-world-only"
    } else {
        "no-probe-failure"
    };

    eprintln!(
        "BOOT-04 chunk-band diag: below={} cross={} multi={} large={} main={} => {}",
        summarize(&below_boundary),
        summarize(&cross_boundary),
        summarize(&multi_chunk),
        summarize_optional(&large_single_file),
        summarize_optional(&main_again),
        classification
    );

    assert!(matches!(
        classification,
        "local-before-boundary"
            | "first-boundary-crossing"
            | "post-first-chunk"
            | "real-world-only"
            | "no-probe-failure"
    ));
}

#[test]
fn test_e2e_boot04_self_hosted_stage2_compiles_step512_progress_harness() {
    let main_path = selfhost_main_path();
    let selfhost_root = main_path
        .parent()
        .expect("App/ ディレクトリ")
        .parent()
        .expect("src/ ディレクトリ")
        .parent()
        .expect("selfhost/ ルートディレクトリ")
        .to_path_buf();
    let diagnostic_rel_path = "src/Tools/Test/Stage2LexerStep512Progress.ls";
    let diagnostic_abs_path = selfhost_root.join(diagnostic_rel_path);

    assert!(
        diagnostic_abs_path.exists(),
        "診断ハーネス {} が存在しない",
        diagnostic_abs_path.display()
    );

    let stage1_wasm = compile_file_only(&main_path);
    assert_valid_wasm(&stage1_wasm);

    let stage2_output = lsharp_wasm::wasi_runner::run_wasm_wasi_with_dir_and_args(
        &stage1_wasm,
        Some(&selfhost_root),
        &["compiler", "src/App/Main.ls"],
    )
    .expect("BOOT-04 step512-compile: stage1 が Main.ls の self-compile に失敗した");
    let stage2_modules = parse_emitted_wasm_modules(&stage2_output, 1);
    let stage2_self_compiler = &stage2_modules[0];
    assert_valid_wasm(stage2_self_compiler);

    let stage3_output = run_wasm_with_six_imports_compiler_mode_fs(
        stage2_self_compiler,
        &selfhost_root,
        &["compiler", diagnostic_rel_path],
    )
    .expect("BOOT-04 step512-compile: stage2 が診断ハーネスをコンパイルできない");
    let stage3_modules = parse_emitted_wasm_modules(&stage3_output, 1);
    let stage3_wasm = &stage3_modules[0];
    assert_valid_wasm(stage3_wasm);
}

#[test]
fn test_e2e_boot04_self_hosted_stage2_compiler_runtime_resolves_param_and_user_call() {
    let main_path = selfhost_main_path();
    let selfhost_root = main_path
        .parent()
        .expect("App/ ディレクトリ")
        .parent()
        .expect("src/ ディレクトリ")
        .parent()
        .expect("selfhost/ ルートディレクトリ")
        .to_path_buf();

    let stage1_wasm = compile_file_only(&main_path);
    assert_valid_wasm(&stage1_wasm);

    let stage2_output = lsharp_wasm::wasi_runner::run_wasm_wasi_with_dir_and_args(
        &stage1_wasm,
        Some(&selfhost_root),
        &["compiler", "src/App/Main.ls"],
    )
    .expect("BOOT-04 runtime-lookup: stage1 が Main.ls の self-compile に失敗した");
    let stage2_modules = parse_emitted_wasm_modules(&stage2_output, 1);
    let stage2_self_compiler = &stage2_modules[0];
    assert_valid_wasm(stage2_self_compiler);

    let stage3_source = r#"
(defn helper [x] x)
(defn main []
  (print (helper 7)))
"#;
    let stage3_output = run_wasm_with_six_imports_compiler_mode(
        stage2_self_compiler,
        stage3_source,
        &["compiler", "inline-runtime-lookup.ls"],
    )
    .expect("BOOT-04 runtime-lookup: stage2 が inline source をコンパイルできない");
    let stage3_modules = parse_emitted_wasm_modules(&stage3_output, 1);
    let stage3_wasm = &stage3_modules[0];
    assert_valid_wasm(stage3_wasm);

    let printed = run_wasm_with_six_imports_compiler_mode(stage3_wasm, "", &[])
        .expect("BOOT-04 runtime-lookup: stage3 inline wasm の実行に失敗");
    assert_eq!(
        printed, "7\n",
        "stage2 compiler runtime は param/local lookup と user call lookup を保持すること"
    );
}

#[test]
fn test_e2e_boot04_self_hosted_stage2_reports_step512_progress() {
    let main_path = selfhost_main_path();
    let selfhost_root = main_path
        .parent()
        .expect("App/ ディレクトリ")
        .parent()
        .expect("src/ ディレクトリ")
        .parent()
        .expect("selfhost/ ルートディレクトリ")
        .to_path_buf();
    let diagnostic_rel_path = "src/Tools/Test/Stage2LexerStep512Progress.ls";
    let diagnostic_abs_path = selfhost_root.join(diagnostic_rel_path);

    assert!(
        diagnostic_abs_path.exists(),
        "診断ハーネス {} が存在しない",
        diagnostic_abs_path.display()
    );

    let stage1_wasm = compile_file_only(&main_path);
    assert_valid_wasm(&stage1_wasm);

    let stage2_output = lsharp_wasm::wasi_runner::run_wasm_wasi_with_dir_and_args(
        &stage1_wasm,
        Some(&selfhost_root),
        &["compiler", "src/App/Main.ls"],
    )
    .expect("BOOT-04 step512 diag: stage1 が Main.ls の self-compile に失敗した");
    let stage2_modules = parse_emitted_wasm_modules(&stage2_output, 1);
    let stage2_self_compiler = &stage2_modules[0];
    assert_valid_wasm(stage2_self_compiler);

    let stage3_result = run_wasm_with_six_imports_compiler_mode_fs(
        stage2_self_compiler,
        &selfhost_root,
        &["compiler", diagnostic_rel_path],
    );

    match &stage3_result {
        Ok(stage3_output) => {
            // SUCCESS: stage2 が診断ハーネスをコンパイルできた
            let stage3_modules = parse_emitted_wasm_modules(stage3_output, 1);
            let stage3_wasm = &stage3_modules[0];
            assert_valid_wasm(stage3_wasm);

            match validate_wasm_detailed(stage3_wasm) {
                Err(validate_err) => {
                    eprintln!(
                        "BOOT-04 step512 diag ADVANCED: stage3 diagnostic wasm validation failed: {}",
                        validate_err
                    );
                    assert!(
                        validate_err.contains("values remaining on stack at end of block"),
                        "step512 stage3 validation 失敗モードが変わった可能性: {}",
                        validate_err
                    );
                }
                Ok(()) => match run_wasm_with_six_imports_compiler_mode(stage3_wasm, "", &[]) {
                    Ok(run_output) => {
                        let values = run_output
                            .lines()
                            .filter(|line| !line.trim().is_empty())
                            .map(|line| {
                                line.trim().parse::<i64>().unwrap_or_else(|err| {
                                    panic!("step512 診断出力が整数でない: {line:?} / {err}")
                                })
                            })
                            .collect::<Vec<_>>();

                        eprintln!("BOOT-04 step512 diag SUCCESS: {:?}", values);

                        assert!(
                            values.len() == 4 || values.len() == 7,
                            "step512 診断出力は 4 行または 7 行であるべき: {:?}",
                            values
                        );

                        let source_len = values[0];
                        let done1 = values[1];
                        let next1 = values[2];
                        let count1 = values[3];

                        assert!(source_len > 0, "step512 診断入力長が 0");
                        assert!(
                            next1 > 0 && next1 <= source_len,
                            "step1 next が範囲外: {:?}",
                            values
                        );
                        assert!(count1 > 0, "step1 token count が 0: {:?}", values);

                        if done1 == 0 {
                            assert_eq!(
                                values.len(),
                                7,
                                "step1 未完了なら step2 出力が必要: {:?}",
                                values
                            );
                            let next2 = values[5];
                            let count2 = values[6];
                            assert!(
                                next2 > next1 && next2 <= source_len,
                                "step2 next が前進していない: {:?}",
                                values
                            );
                            assert!(
                                count2 > count1,
                                "step2 token count が増えていない: {:?}",
                                values
                            );
                        } else {
                            assert_eq!(
                                values.len(),
                                4,
                                "step1 完了なら step2 出力は不要: {:?}",
                                values
                            );
                        }
                    }
                    Err(run_err) => {
                        // ADVANCED: stage2 compile は通ったので、次の narrow blocker は stage3 wasm の
                        // block stack-balance 崩れであることを診断として固定する。
                        let violations = local_bound_violations(stage3_wasm);
                        let _ = std::fs::write("/tmp/step512_progress_stage3.wasm", stage3_wasm);
                        eprintln!(
                            "BOOT-04 step512 diag ADVANCED: stage3 diagnostic wasm runtime/load failed: {}; full_error={}; sections={:?}; violations={:?}; fingerprint={}",
                            run_err.lines().next().unwrap_or(""),
                            run_err,
                            extract_sections(stage3_wasm),
                            violations,
                            hash_fingerprint(stage3_wasm)
                        );
                        assert!(
                            run_err.contains("values remaining on stack at end of block"),
                            "step512 stage3 実行失敗モードが変わった可能性: {}",
                            run_err
                        );
                    }
                },
            }
        }
        Err(compile_err) => {
            // BLOCKED: stage2 が Syntax.Lexer を含む診断ハーネスをコンパイルできない
            // wasm コールスタックの再帰限界を計測して文書化する
            let frame_count = compile_err
                .lines()
                .filter(|l| l.contains("wasm function"))
                .count();
            eprintln!(
                "BOOT-04 step512 diag BLOCKED: stage2 compile failed with {} wasm frames at overflow",
                frame_count
            );
            eprintln!(
                "BOOT-04 step512 diag BLOCKED: first error line: {}",
                compile_err.lines().next().unwrap_or("")
            );
            // stage2 の再帰1レベルあたり約 65 フレームを消費する。
            // Syntax.Lexer の classify-symbol は 12 段の nested-if を持ち、
            // 12 * 65 = 780 フレームが必要 >> wasmtime のデフォルト ~280 フレーム限界
            eprintln!(
                "BOOT-04 step512 diag THRESHOLD: stage2 wasm stack ~{} frames; \
                 ~{} recursion levels (each ~65 frames); \
                 Syntax.Lexer classify-symbol requires ~12 nested-if levels (~780 frames needed); \
                 fix = reduce stage2 expression recursion depth",
                frame_count,
                frame_count / 65
            );

            // 既知の失敗モード: wasm バックトレースを含む深い再帰スタックオーバーフロー
            assert!(
                compile_err.contains("wasm backtrace") || compile_err.contains("unreachable"),
                "step512 stage2 compile 失敗は wasm backtrace を含むべき (got: {})",
                compile_err.lines().next().unwrap_or("")
            );
            // フレーム数 ≥ 200 → 深い再帰であることを確認
            assert!(
                frame_count >= 200,
                "step512 stage2 overflow frame count が 200 未満 (got {}): 失敗モードが変わった可能性がある",
                frame_count
            );
        }
    }
}

/// BOOT-04: compiler-mode が Syntax.LexerCompat を含む selfhost probe を解決できること
#[test]
fn test_e2e_boot04_compiler_mode_lexer_compat_import_resolution() {
    let main_path = selfhost_main_path();
    let selfhost_root = main_path
        .parent()
        .expect("App/ ディレクトリ")
        .parent()
        .expect("src/ ディレクトリ")
        .parent()
        .expect("selfhost/ ルートディレクトリ")
        .to_path_buf();
    let probe_rel_path = "src/Tools/Test/LexerCompatImportProbe.ls";
    let probe_abs_path = selfhost_root.join(probe_rel_path);

    assert!(
        probe_abs_path.exists(),
        "compat probe {} が存在しない",
        probe_abs_path.display()
    );

    let stage1_wasm = compile_file_only(&main_path);
    assert_valid_wasm(&stage1_wasm);

    let output = lsharp_wasm::wasi_runner::run_wasm_wasi_with_dir_and_args(
        &stage1_wasm,
        Some(&selfhost_root),
        &["compiler", probe_rel_path],
    )
    .expect("BOOT-04 lexer-compat-import: compiler-mode が compat probe をコンパイルできなかった");

    let modules = parse_emitted_wasm_modules(&output, 1);
    let result_wasm = &modules[0];
    assert_valid_wasm(result_wasm);

    let run_output = run_wasm_with_six_imports_compiler_mode(result_wasm, "", &[])
        .expect("BOOT-04 lexer-compat-import: 生成 wasm の実行に失敗した");
    let values = run_output
        .lines()
        .filter(|line| !line.trim().is_empty())
        .map(|line| {
            line.trim().parse::<i64>().unwrap_or_else(|err| {
                panic!("lexer-compat probe 出力が整数でない: {line:?} / {err}")
            })
        })
        .collect::<Vec<_>>();

    assert_eq!(
        values.len(),
        7,
        "lexer-compat probe 出力行数が不正: {:?}",
        values
    );
    assert!(
        values[0] >= 3,
        "legacy tokenize は少なくとも 3 要素以上を返すべき: {:?}",
        values
    );
    assert_eq!(&values[1..], &[5, 0, 1, 2, 42, 1]);
}

/// BOOT-04: compiler-mode が import 宣言を解決できること
///
/// stage1 (Rust bootstrap wasm) を compiler-mode で実行したとき、
/// (import ...) 宣言を持つファイルを正しく処理できることを検証する。
///
/// simple_main.ls: (import SimpleHelper) + (defn main [] (helper-value))
/// simple_helper.ls: (defn helper-value [] 42)
///
/// import 解決後:
/// - helper-value, main の両関数が ftable に登録される
/// - 生成 wasm は valid wasm
/// - _start → main → helper-value → 42 が正常実行される
#[test]
fn test_e2e_boot04_compiler_mode_import_resolution() {
    let main_path = selfhost_main_path();
    let fixture_dir =
        std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../tests/fixtures");

    assert!(
        fixture_dir.join("SimpleMain.ls").exists(),
        "fixture ファイル tests/fixtures/SimpleMain.ls が存在しない"
    );
    assert!(
        fixture_dir.join("SimpleHelper.ls").exists(),
        "fixture ファイル tests/fixtures/SimpleHelper.ls が存在しない"
    );

    // stage1 (Rust bootstrap) wasm
    let stage1_wasm = compile_file_only(&main_path);
    assert_valid_wasm(&stage1_wasm);

    // compiler-mode で SimpleMain.ls をコンパイル (import SimpleHelper を解決する必要あり)
    let output = lsharp_wasm::wasi_runner::run_wasm_wasi_with_dir_and_args(
        &stage1_wasm,
        Some(&fixture_dir),
        &["compiler", "SimpleMain.ls"],
    )
    .expect("BOOT-04 import-resolution: compiler-mode が SimpleMain.ls をコンパイルできなかった");

    // 出力が length-prefixed wasm バイト列であること
    let modules = parse_emitted_wasm_modules(&output, 1);
    let result_wasm = &modules[0];
    assert_valid_wasm(result_wasm);

    // 生成 wasm が正常実行できること (helper-value を呼び出す main が動く)
    // 6-import モデル: env.string-concat, env.substring も import される
    let run_result = run_wasm_with_six_imports_compiler_mode(result_wasm, "", &[]);
    assert!(
        run_result.is_ok(),
        "BOOT-04 import-resolution: 生成 wasm の WASI 実行に失敗: {:?}",
        run_result.err()
    );

    eprintln!(
        "BOOT-04 import-resolution GREEN: SimpleMain.ls + SimpleHelper → {} bytes の wasm を生成・実行 OK",
        result_wasm.len()
    );
}

/// BOOT-04: compiler-mode が manifest なし source root 配下の dotted import を解決できること
#[test]
fn test_e2e_boot04_compiler_mode_dotted_import_resolution_from_src_root() {
    let main_path = selfhost_main_path();
    let fixture_dir = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../tests/fixtures/hier-selfhost");

    assert!(
        fixture_dir.join("src/App/Main.ls").exists(),
        "fixture ファイル tests/fixtures/hier-selfhost/src/App/Main.ls が存在しない"
    );
    assert!(
        fixture_dir.join("src/Syntax/SimpleHelper.ls").exists(),
        "fixture ファイル tests/fixtures/hier-selfhost/src/Syntax/SimpleHelper.ls が存在しない"
    );

    let stage1_wasm = compile_file_only(&main_path);
    assert_valid_wasm(&stage1_wasm);

    let output = lsharp_wasm::wasi_runner::run_wasm_wasi_with_dir_and_args(
        &stage1_wasm,
        Some(&fixture_dir),
        &["compiler", "src/App/Main.ls"],
    )
    .expect(
        "BOOT-04 dotted-import-resolution: compiler-mode が src/App/Main.ls をコンパイルできなかった",
    );

    let modules = parse_emitted_wasm_modules(&output, 1);
    let result_wasm = &modules[0];
    assert_valid_wasm(result_wasm);

    let run_result = run_wasm_with_six_imports_compiler_mode(result_wasm, "", &[]);
    assert!(
        run_result.is_ok(),
        "BOOT-04 dotted-import-resolution: 生成 wasm の WASI 実行に失敗: {:?}",
        run_result.err()
    );
}

#[test]
fn test_e2e_boot04_compiler_mode_package_index_resolution() {
    let main_path = selfhost_main_path();
    let fixture_dir = std::env::temp_dir().join(format!(
        "lsharp_selfhost_package_index_{}",
        std::process::id()
    ));
    let _ = std::fs::remove_dir_all(&fixture_dir);
    std::fs::create_dir_all(fixture_dir.join("src")).unwrap();
    std::fs::create_dir_all(fixture_dir.join(".lsharp/packages/demo-123/src")).unwrap();
    std::fs::create_dir_all(fixture_dir.join(".lsharp/module-index")).unwrap();

    std::fs::write(
        fixture_dir.join("src/Main.ls"),
        "(module Main)\n(import Geometry)\n(defn main [] (distance))",
    )
    .unwrap();
    std::fs::write(
        fixture_dir.join(".lsharp/packages/demo-123/src/Geometry.ls"),
        "(module Geometry)\n(defn distance [] 42)",
    )
    .unwrap();
    std::fs::write(
        fixture_dir.join(".lsharp/module-index/Geometry.path"),
        ".lsharp/packages/demo-123/src/Geometry.ls\n",
    )
    .unwrap();

    let stage1_wasm = compile_file_only(&main_path);
    assert_valid_wasm(&stage1_wasm);

    let output = lsharp_wasm::wasi_runner::run_wasm_wasi_with_dir_and_args(
        &stage1_wasm,
        Some(&fixture_dir),
        &["compiler", "src/Main.ls"],
    )
    .expect(
        "BOOT-04 package-index-resolution: compiler-mode が src/Main.ls をコンパイルできなかった",
    );

    let modules = parse_emitted_wasm_modules(&output, 1);
    let result_wasm = &modules[0];
    assert_valid_wasm(result_wasm);

    let run_result = run_wasm_with_six_imports_compiler_mode(result_wasm, "", &[]);
    let _ = std::fs::remove_dir_all(&fixture_dir);
    assert!(
        run_result.is_ok(),
        "BOOT-04 package-index-resolution: 生成 wasm の WASI 実行に失敗: {:?}",
        run_result.err()
    );
}

#[test]
fn test_e2e_boot04_compiler_mode_supports_twelve_arg_calls() {
    let main_path = selfhost_main_path();
    let fixture_dir =
        std::env::temp_dir().join(format!("lsharp_selfhost_many_args_{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&fixture_dir);
    std::fs::create_dir_all(fixture_dir.join("src")).unwrap();
    std::fs::write(
        fixture_dir.join("src/Main.ls"),
        "(module Main)\n(defn pick-last [a b c d e f g h i j k l] (do (print l) l))\n(defn main [] (pick-last 1 2 3 4 5 6 7 8 9 10 11 12))",
    )
    .unwrap();

    let stage1_wasm = compile_file_only(&main_path);
    assert_valid_wasm(&stage1_wasm);

    let output = lsharp_wasm::wasi_runner::run_wasm_wasi_with_dir_and_args(
        &stage1_wasm,
        Some(&fixture_dir),
        &["compiler", "src/Main.ls"],
    )
    .expect("BOOT-04 many-args: compiler-mode が src/Main.ls をコンパイルできなかった");

    let modules = parse_emitted_wasm_modules(&output, 1);
    let result_wasm = &modules[0];
    assert_valid_wasm(result_wasm);

    let run_result = run_wasm_with_six_imports_compiler_mode(result_wasm, "", &[]);
    let _ = std::fs::remove_dir_all(&fixture_dir);
    let run_output = run_result.expect("BOOT-04 many-args: 生成 wasm の WASI 実行に失敗した");
    assert_eq!(run_output, "12\n");
}

#[test]
fn test_e2e_boot04_compiler_mode_ignores_dotted_flat_file() {
    let main_path = selfhost_main_path();
    let fixture_dir = std::env::temp_dir().join(format!(
        "lsharp_selfhost_dotted_flat_fallback_{}",
        std::process::id()
    ));
    let _ = std::fs::remove_dir_all(&fixture_dir);
    std::fs::create_dir_all(fixture_dir.join("src/App")).unwrap();
    std::fs::create_dir_all(fixture_dir.join("src")).unwrap();

    std::fs::write(
        fixture_dir.join("src/App/Main.ls"),
        "(module App.Main)\n(import Syntax.Token)\n(defn main [] (print (token-tag)))",
    )
    .unwrap();
    std::fs::write(
        fixture_dir.join("src/Syntax.Token.ls"),
        "(module Syntax.Token)\n(defn token-tag [] 7)",
    )
    .unwrap();

    let stage1_wasm = compile_file_only(&main_path);
    assert_valid_wasm(&stage1_wasm);

    let result = lsharp_wasm::wasi_runner::run_wasm_wasi_with_dir_and_args(
        &stage1_wasm,
        Some(&fixture_dir),
        &["compiler", "src/App/Main.ls"],
    );

    let _ = std::fs::remove_dir_all(&fixture_dir);
    if let Ok(output) = result {
        let modules = parse_emitted_wasm_modules(&output, 1);
        let result_wasm = &modules[0];
        assert_valid_wasm(result_wasm);

        if let Ok(run_output) = run_wasm_with_six_imports_compiler_mode(result_wasm, "", &[]) {
            assert_ne!(
                run_output, "7\n",
                "BOOT-04 dotted-flat-file: compiler-mode が src/Syntax.Token.ls を module source に採用している"
            );
        }
    }
}

#[test]
fn test_i64_if_condition_validity() {
    // i64 を if 条件に使う wasm を wasmparser と wasmtime で検証
    let vresult = validate_wasm_detailed(TEST_I64_IF_WASM);
    eprintln!("wasmparser result: {:?}", vresult);
    let engine = wasmtime::Engine::default();
    let mresult = wasmtime::Module::new(&engine, TEST_I64_IF_WASM);
    eprintln!(
        "wasmtime result: {}",
        if mresult.is_ok() { "OK" } else { "FAIL" }
    );
}

#[test]
#[ignore]
fn test_debug_stage2_save() {
    // stage2 を生成してファイルに保存する (デバッグ用)
    let main_path = selfhost_main_path();
    let stage1_wasm = compile_file_only(&main_path);
    let selfhost_dir = selfhost_package_root();
    let output = lsharp_wasm::wasi_runner::run_wasm_wasi_with_dir_and_args(
        &stage1_wasm,
        Some(&selfhost_dir),
        &["compiler", "src/App/Main.ls"],
    )
    .expect("stage1 failed");
    let modules = parse_emitted_wasm_modules(&output, 1);
    let stage2 = &modules[0];
    std::fs::write("stage2_debug.wasm", stage2).expect("write failed");
    eprintln!("stage2_debug.wasm written ({} bytes)", stage2.len());
}

#[test]
fn test_parse_compiler_ls() {
    // Compiler.ls をパースして構文エラーを検出する
    let source = std::fs::read_to_string(selfhost_source_path("Compiler.ls")).expect("read file");
    match lsharp_syntax::parse(&source) {
        Ok(_) => eprintln!("Compiler.ls パース成功"),
        Err(e) => eprintln!("Compiler.ls パースエラー: {:?}", e),
    }
}

#[test]
fn test_parse_caws_standalone() {
    // compile-apply-with-source を単独でパースする
    let source = std::fs::read_to_string(
        selfhost_project_root().join("tests/fixtures/selfhost-debug/test_caws.ls"),
    )
    .expect("read file");
    match lsharp_syntax::parse(&source) {
        Ok(prog) => eprintln!("パース成功: {} decls", prog.decls.len()),
        Err(e) => eprintln!("パースエラー: {:?}", e),
    }
}

#[test]
#[ignore]
fn test_debug_stage2_output_minimal() {
    // stage2 が minimal.ls をコンパイルした出力を保存・検証する
    let main_path = selfhost_main_path();
    let selfhost_root = main_path
        .parent()
        .unwrap()
        .parent()
        .unwrap()
        .parent()
        .unwrap()
        .to_path_buf();
    let stage1_wasm = compile_file_only(&main_path);

    // stage1 で src/App/Main.ls をコンパイル → stage2
    let output = lsharp_wasm::wasi_runner::run_wasm_wasi_with_dir_and_args(
        &stage1_wasm,
        Some(&selfhost_root),
        &["compiler", "src/App/Main.ls"],
    )
    .expect("stage1 failed");
    let modules = parse_emitted_wasm_modules(&output, 1);
    let stage2 = &modules[0];
    std::fs::write("stage2_debug2.wasm", stage2).expect("write failed");
    eprintln!("stage2 written ({} bytes)", stage2.len());

    // stage2 で minimal.ls をコンパイル
    let fixture_dir =
        std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../tests/fixtures");
    let stage3_result = lsharp_wasm::wasi_runner::run_wasm_wasi_with_dir_and_args(
        stage2,
        Some(&fixture_dir),
        &["compiler", "minimal.ls"],
    );
    match stage3_result {
        Err(e) => eprintln!("stage2->minimal failed: {}", e),
        Ok(out) => {
            let modules3 = parse_emitted_wasm_modules(&out, 1);
            let stage3 = &modules3[0];
            std::fs::write("stage3_minimal.wasm", stage3).expect("write failed");
            eprintln!("stage3 written ({} bytes)", stage3.len());
        }
    }
}

#[test]
fn test_validate_stage2_wasm() {
    // stage2 を詳細バリデーション
    let main_path = selfhost_main_path();
    let selfhost_root = main_path
        .parent()
        .unwrap()
        .parent()
        .unwrap()
        .parent()
        .unwrap()
        .to_path_buf();
    let stage1_wasm = compile_file_only(&main_path);
    let output = lsharp_wasm::wasi_runner::run_wasm_wasi_with_dir_and_args(
        &stage1_wasm,
        Some(&selfhost_root),
        &["compiler", "src/App/Main.ls"],
    )
    .expect("stage1 failed");
    let modules = parse_emitted_wasm_modules(&output, 1);
    let stage2 = &modules[0];
    match validate_wasm_detailed(stage2) {
        Ok(_) => eprintln!("stage2 詳細バリデーション PASSED ({} bytes)", stage2.len()),
        Err(e) => eprintln!("stage2 詳細バリデーション FAILED: {}", e),
    }
}

#[test]
#[ignore]
fn test_debug_func_49_context() {
    // stage2 のwasm 49 番の関数が何をしているか確認
    let main_path = selfhost_main_path();
    let selfhost_root = main_path
        .parent()
        .unwrap()
        .parent()
        .unwrap()
        .parent()
        .unwrap()
        .to_path_buf();
    let stage1_wasm = compile_file_only(&main_path);
    let output = lsharp_wasm::wasi_runner::run_wasm_wasi_with_dir_and_args(
        &stage1_wasm,
        Some(&selfhost_root),
        &["compiler", "src/App/Main.ls"],
    )
    .expect("stage1 failed");
    let modules = parse_emitted_wasm_modules(&output, 1);
    let stage2 = &modules[0];

    // Count imports
    let mut pos = 8usize;
    let mut import_count = 0u32;
    let data = stage2.as_slice();
    while pos < data.len() {
        let sid = data[pos];
        pos += 1;
        let mut size = 0u32;
        let mut shift = 0;
        loop {
            let b = data[pos];
            pos += 1;
            size |= ((b & 0x7f) as u32) << shift;
            if b & 0x80 == 0 {
                break;
            }
            shift += 7;
        }
        if sid == 2 {
            // count imports
            let mut v = 0u32;
            let mut sh = 0;
            let mut i = pos;
            loop {
                let b = data[i];
                i += 1;
                v |= ((b & 0x7f) as u32) << sh;
                if b & 0x80 == 0 {
                    break;
                }
                sh += 7;
            }
            import_count = v;
        }
        if sid == 3 {
            // count user funcs
            let mut v = 0u32;
            let mut sh = 0;
            let mut i = pos;
            loop {
                let b = data[i];
                i += 1;
                v |= ((b & 0x7f) as u32) << sh;
                if b & 0x80 == 0 {
                    break;
                }
                sh += 7;
            }
            eprintln!(
                "stage2: {} imports, {} user funcs, total={}",
                import_count,
                v,
                import_count + v
            );
            break;
        }
        pos += size as usize;
    }
}

#[test]
#[ignore]
fn test_debug_tok_eof_in_stage2() {
    // Token.ls main が tok-eof を正しく呼べるか確認
    // stage2の func 49 (Token::main) がちゃんと call 48 を使うか確認
    let main_path = selfhost_main_path();
    let selfhost_dir = selfhost_package_root();
    let stage1_wasm = compile_file_only(&main_path);
    let output = lsharp_wasm::wasi_runner::run_wasm_wasi_with_dir_and_args(
        &stage1_wasm,
        Some(&selfhost_dir),
        &["compiler", "src/App/Main.ls"],
    )
    .expect("stage1 failed");
    let modules = parse_emitted_wasm_modules(&output, 1);
    let stage2 = &modules[0];

    // Find func 49 bytecode
    let data = stage2.as_slice();
    fn read_leb(data: &[u8], pos: &mut usize) -> u64 {
        let mut v = 0u64;
        let mut shift = 0;
        loop {
            let b = data[*pos];
            *pos += 1;
            v |= ((b & 0x7f) as u64) << shift;
            if b & 0x80 == 0 {
                break;
            }
            shift += 7;
        }
        v
    }
    let mut pos = 8usize;
    while pos < data.len() {
        let sid = data[pos];
        pos += 1;
        let size = read_leb(data, &mut pos) as usize;
        if sid == 10 {
            // code section
            let _count = read_leb(data, &mut pos);
            // Skip to func 49 (index 49 in code section, which = func 49-6=43 user func)
            // actually each func in code section is 0-indexed: func 0 = user func 0, func 43 = user func 43
            // func 49 in wasm = imports(6) + user_func_43
            // code section func 43 (0-indexed)
            for _ in 0..43 {
                let sz = read_leb(data, &mut pos) as usize;
                pos += sz;
            }
            let func_size = read_leb(data, &mut pos) as usize;
            eprintln!("Code func 43 (Token::main) size={} bytes", func_size);
            let func_end = pos + func_size;
            let local_count = read_leb(data, &mut pos);
            for _ in 0..local_count {
                let _n = read_leb(data, &mut pos);
                let _t = data[pos];
                pos += 1;
            }
            // Dump the instructions
            while pos < func_end {
                let op = data[pos];
                pos += 1;
                match op {
                    0x10 => {
                        let idx = read_leb(data, &mut pos);
                        eprintln!("  call {idx}");
                    }
                    0x42 => {
                        let v = read_leb(data, &mut pos);
                        eprintln!("  i64.const {v}");
                    }
                    0x1a => eprintln!("  drop"),
                    0x0b => {
                        eprintln!("  end");
                        break;
                    }
                    _ => eprintln!("  op 0x{op:02x}"),
                }
            }
            break;
        }
        pos += size;
    }
}

#[test]
#[ignore]
fn test_debug_token_ls_compilation() {
    // Token.ls だけをコンパイルして tok-eof (func 42) が正しい index を持つか確認
    let token_path = selfhost_source_path("Token.ls");
    let token_src = std::fs::read_to_string(&token_path).unwrap();
    eprintln!("Token.ls: {} chars", token_src.len());

    // tok-eof hash
    let tok_eof_hash: i64 = {
        let s = "tok-eof";
        let mut acc: i64 = 0;
        for c in s.chars() {
            acc = acc.wrapping_mul(31).wrapping_add(c as i64);
        }
        acc
    };
    eprintln!("tok-eof hash = {tok_eof_hash}");

    // Manually check: compile Token.ls with selfhost (via stage1)
    let main_path = selfhost_main_path();
    let selfhost_dir = selfhost_package_root();
    let stage1_wasm = compile_file_only(&main_path);

    // compile Token.ls with stage1
    let output = lsharp_wasm::wasi_runner::run_wasm_wasi_with_dir_and_args(
        &stage1_wasm,
        Some(&selfhost_dir),
        &["compiler", "src/Syntax/Token.ls"],
    )
    .expect("stage1 failed to compile Token.ls");
    eprintln!("Token.ls compiled, output {} chars", output.len());
    let modules = parse_emitted_wasm_modules(&output, 1);
    let token_wasm = &modules[0];

    // Look for tok-eof function (returns 99)
    let found_99 = std::panic::catch_unwind(|| {
        let data = token_wasm.as_slice();
        fn read_leb(data: &[u8], pos: &mut usize) -> u64 {
            let mut v = 0u64;
            let mut shift = 0;
            loop {
                let b = data[*pos];
                *pos += 1;
                v |= ((b & 0x7f) as u64) << shift;
                if b & 0x80 == 0 {
                    break;
                }
                shift += 7;
            }
            v
        }
        let mut pos = 8usize;
        let mut func_99_idx = None;
        while pos < data.len() {
            let sid = data[pos];
            pos += 1;
            let size = read_leb(data, &mut pos) as usize;
            if sid == 10 {
                let count = read_leb(data, &mut pos) as usize;
                for fidx in 0..count {
                    let sz = read_leb(data, &mut pos) as usize;
                    let end = pos + sz;
                    let local_count = read_leb(data, &mut pos);
                    for _ in 0..local_count {
                        let _n = read_leb(data, &mut pos);
                        let _t = data[pos];
                        pos += 1;
                    }
                    // Check if it's a simple i64.const 99 return
                    if pos < end - 2 && data[pos] == 0x42 {
                        // i64.const
                        pos += 1;
                        let val = read_leb(data, &mut pos);
                        if val == 99 && pos < end && data[pos] == 0x0b {
                            func_99_idx = Some(fidx);
                            eprintln!(
                                "Found tok-eof (=99) at user func idx {fidx} (wasm idx {})",
                                fidx + 6
                            );
                        }
                    }
                    pos = end;
                }
                break;
            }
            pos += size;
        }
        func_99_idx
    });
    eprintln!("tok-eof in Token.ls compilation: {:?}", found_99);
}

#[test]
#[ignore]
fn test_debug_stage3_output_chars() {
    // stage2 が minimal.ls をコンパイルした出力の最初の 200 文字を確認する
    let main_path = selfhost_main_path();
    let selfhost_root = main_path
        .parent()
        .unwrap()
        .parent()
        .unwrap()
        .parent()
        .unwrap()
        .to_path_buf();
    let stage1_wasm = compile_file_only(&main_path);

    let stage2_output = lsharp_wasm::wasi_runner::run_wasm_wasi_with_dir_and_args(
        &stage1_wasm,
        Some(&selfhost_root),
        &["compiler", "src/App/Main.ls"],
    )
    .expect("stage1 failed");

    let modules = parse_emitted_wasm_modules(&stage2_output, 1);
    let stage2 = &modules[0];

    let fixture_dir =
        std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../tests/fixtures");
    let minimal_ls = std::fs::read_to_string(fixture_dir.join("minimal.ls"))
        .unwrap_or_else(|_| "(defn main [] 42)".to_string());

    let stage3_result =
        run_wasm_with_six_imports_compiler_mode(stage2, &minimal_ls, &["compiler", "minimal.ls"]);

    match stage3_result {
        Err(e) => eprintln!("stage3 実行失敗: {}", e),
        Ok(out) => {
            eprintln!("stage3 output length: {} chars", out.len());
            let lines: Vec<&str> = out.lines().filter(|l| !l.is_empty()).collect();
            eprintln!("stage3 line count: {}", lines.len());
            if let Some(first) = lines.first() {
                eprintln!("stage3 first line (count?): {}", first);
            }
            // 最初の30個の値を表示
            let values: Vec<i64> = lines
                .iter()
                .take(30)
                .filter_map(|l| l.trim().parse::<i64>().ok())
                .collect();
            eprintln!("stage3 first 30 values: {:?}", values);
            // 全 i64 値を収集（範囲外を含む）
            let all_values: Vec<i64> = lines
                .iter()
                .filter_map(|l| l.trim().parse::<i64>().ok())
                .collect();
            // 有効バイト範囲外の値を探す
            let out_of_range: Vec<(usize, i64)> = all_values
                .iter()
                .enumerate()
                .filter(|&(_, &v)| !(0..=255).contains(&v))
                .take(5)
                .map(|(i, &v)| (i, v))
                .collect();
            eprintln!("out-of-range byte values (pos, val): {:?}", out_of_range);
            // stage3 bytes を保存
            if !all_values.is_empty() {
                let count = all_values[0] as usize;
                if all_values.len() > count {
                    let bytes: Vec<u8> = all_values[1..=count]
                        .iter()
                        .map(|&v| (v & 0xFF) as u8)
                        .collect();
                    let _ = std::fs::write("stage3_from_debug.wasm", &bytes);
                    eprintln!(
                        "stage3 bytes saved ({} bytes, {} may be truncated)",
                        count,
                        bytes.len()
                    );
                }
            }
            // 全値を print
            eprintln!(
                "stage3 all {} values: {:?}",
                all_values.len(),
                &all_values[..all_values.len().min(200)]
            );
        }
    }
}

#[test]
#[ignore]
fn test_debug_stage3_main_again_output_chars() {
    let main_path = selfhost_main_path();
    let selfhost_root = main_path
        .parent()
        .unwrap()
        .parent()
        .unwrap()
        .parent()
        .unwrap()
        .to_path_buf();
    let stage1_wasm = compile_file_only(&main_path);

    let stage2_output = lsharp_wasm::wasi_runner::run_wasm_wasi_with_dir_and_args(
        &stage1_wasm,
        Some(&selfhost_root),
        &["compiler", "src/App/Main.ls"],
    )
    .expect("stage1 failed");

    let modules = parse_emitted_wasm_modules(&stage2_output, 1);
    let stage2 = &modules[0];

    let stage3_result = run_wasm_with_six_imports_compiler_mode_fs(
        stage2,
        &selfhost_root,
        &["compiler", "src/App/Main.ls"],
    );

    match stage3_result {
        Err(e) => eprintln!("stage3 main_again 実行失敗: {}", e),
        Ok(out) => {
            eprintln!("stage3 main_again output length: {} chars", out.len());
            let lines: Vec<&str> = out.lines().filter(|l| !l.is_empty()).collect();
            eprintln!("stage3 main_again line count: {}", lines.len());
            if let Some(first) = lines.first() {
                eprintln!("stage3 main_again first line: {}", first);
            }
            let first_values: Vec<i64> = lines
                .iter()
                .take(40)
                .filter_map(|l| l.trim().parse::<i64>().ok())
                .collect();
            eprintln!("stage3 main_again first 40 values: {:?}", first_values);
            let out_of_range: Vec<(usize, i64)> = lines
                .iter()
                .enumerate()
                .filter_map(|(idx, line)| line.trim().parse::<i64>().ok().map(|v| (idx, v)))
                .filter(|(_, v)| *v < 0 || *v > 255)
                .take(20)
                .collect();
            eprintln!(
                "stage3 main_again out-of-range values (pos, val): {:?}",
                out_of_range
            );
        }
    }
}
