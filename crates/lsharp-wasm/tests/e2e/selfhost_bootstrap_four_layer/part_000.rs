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
