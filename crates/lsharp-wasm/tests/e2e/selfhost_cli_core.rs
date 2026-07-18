use super::support::*;
use serde_json::Value;
use std::path::PathBuf;
use std::sync::atomic::{AtomicUsize, Ordering};

static CLI_TEST_FIXTURE_COUNTER: AtomicUsize = AtomicUsize::new(0);

fn lsp_stdio_snapshot(name: &str) -> Vec<Value> {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../tests/snapshots/lsp/stdio")
        .join(name);
    let snapshot = std::fs::read_to_string(&path)
        .unwrap_or_else(|e| panic!("snapshot 読み込み失敗 {}: {}", path.display(), e));
    serde_json::from_str(&snapshot)
        .unwrap_or_else(|e| panic!("snapshot JSON parse 失敗 {}: {}", path.display(), e))
}

fn parse_lsp_stdio_frames(output: &str) -> Vec<Value> {
    let bytes = output.as_bytes();
    let mut cursor = 0;
    let mut frames = Vec::new();

    while cursor < bytes.len() {
        let header_end = bytes[cursor..]
            .windows(4)
            .position(|window| window == b"\r\n\r\n")
            .map(|offset| cursor + offset)
            .unwrap_or_else(|| {
                panic!(
                    "LSP frame header terminator が見つからない: cursor={} output={:?}",
                    cursor, output
                )
            });
        let header = std::str::from_utf8(&bytes[cursor..header_end])
            .unwrap_or_else(|e| panic!("LSP frame header は UTF-8 であるべき: {}", e));
        let content_length = header
            .strip_prefix("Content-Length: ")
            .unwrap_or_else(|| {
                panic!(
                    "LSP frame header は Content-Length で始まるべき: {:?}",
                    header
                )
            })
            .parse::<usize>()
            .unwrap_or_else(|e| panic!("Content-Length parse 失敗 {:?}: {}", header, e));
        let body_start = header_end + 4;
        let body_end = body_start + content_length;
        assert!(
            body_end <= bytes.len(),
            "LSP frame body が途中で切れている: header={:?} bytes={} output={:?}",
            header,
            bytes.len(),
            output
        );
        let body = std::str::from_utf8(&bytes[body_start..body_end])
            .unwrap_or_else(|e| panic!("LSP frame body は UTF-8 であるべき: {}", e));
        let payload = serde_json::from_str(body).unwrap_or_else(|e| {
            panic!(
                "LSP frame body は valid JSON であるべき: {}\nbody={:?}",
                e, body
            )
        });
        frames.push(payload);
        cursor = body_end;
    }

    frames
}

fn assert_lsp_stdio_snapshot(output: &str, snapshot_name: &str, message: &str) {
    let actual = parse_lsp_stdio_frames(output);
    let expected = lsp_stdio_snapshot(snapshot_name);
    assert_eq!(actual, expected, "{}", message);
}

fn cli_text_snapshot(name: &str) -> String {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../tests/snapshots/cli")
        .join(name);
    std::fs::read_to_string(&path)
        .unwrap_or_else(|e| panic!("cli snapshot 読み込み失敗 {}: {}", path.display(), e))
}

fn assert_cli_text_snapshot(output: &str, snapshot_name: &str, message: &str) {
    let expected = cli_text_snapshot(snapshot_name);
    assert_eq!(output, expected, "{}", message);
}

fn doctools_json_snapshot(name: &str) -> Value {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../tests/snapshots/doctools")
        .join(name);
    let snapshot = std::fs::read_to_string(&path)
        .unwrap_or_else(|e| panic!("doctools snapshot 読み込み失敗 {}: {}", path.display(), e));
    serde_json::from_str(&snapshot).unwrap_or_else(|e| {
        panic!(
            "doctools snapshot JSON parse 失敗 {}: {}",
            path.display(),
            e
        )
    })
}

fn cli_test_fixture_dir(prefix: &str) -> PathBuf {
    std::env::temp_dir().join(format!(
        "lsharp_test_cli_core_{}_{}_{}",
        prefix,
        std::process::id(),
        CLI_TEST_FIXTURE_COUNTER.fetch_add(1, Ordering::Relaxed)
    ))
}

fn write_cli_fixture_files(dir: &std::path::Path, files: &[(&str, &str)]) {
    let _ = std::fs::remove_dir_all(dir);
    std::fs::create_dir_all(dir).expect("fixture directory の作成に失敗");
    for (relative, source) in files {
        let path = dir.join(relative);
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent).unwrap_or_else(|e| {
                panic!("fixture parent の作成に失敗 {}: {}", parent.display(), e)
            });
        }
        std::fs::write(&path, source)
            .unwrap_or_else(|e| panic!("fixture file の書き込みに失敗 {}: {}", path.display(), e));
    }
}

fn take_lsharp_toplevel_forms(source: &str, form_count: usize) -> String {
    assert!(form_count > 0, "top-level form 数は 1 以上であるべき");

    let mut forms = 0usize;
    let mut depth = 0usize;
    let mut in_string = false;
    let mut escaped = false;

    for (idx, ch) in source.char_indices() {
        if in_string {
            if escaped {
                escaped = false;
            } else if ch == '\\' {
                escaped = true;
            } else if ch == '"' {
                in_string = false;
            }
            continue;
        }

        match ch {
            '"' => in_string = true,
            '(' => depth += 1,
            ')' => {
                depth = depth.checked_sub(1).expect("top-level form の括弧が不正");
                if depth == 0 {
                    forms += 1;
                    if forms == form_count {
                        return source[..idx + ch.len_utf8()].to_string();
                    }
                }
            }
            _ => {}
        }
    }

    panic!(
        "top-level form を {} 個切り出せない: 実際は {} 個",
        form_count, forms
    );
}

fn stack_safe_wasm_bytes_eq_helpers() -> &'static str {
    r#"
(defn make-wasm-bytes-eq-state [done next-idx mismatch]
  (push-int-vector-local
    (push-int-vector-local
      (push-int-vector-local (vector-new 3) done)
      next-idx)
    mismatch))
(defn wasm-bytes-eq-step [left right idx n]
  (if (>= idx n)
    (make-wasm-bytes-eq-state 1 idx 0)
    (if (= (vector-get left idx) (vector-get right idx))
      (make-wasm-bytes-eq-state 0 (+ idx 1) 0)
      (make-wasm-bytes-eq-state 1 (+ idx 1) 1))))
(defn continue-wasm-bytes-eq-step [left right n state]
  (if (= (vector-get state 0) 1)
    state
    (wasm-bytes-eq-step left right (vector-get state 1) n)))
(defn wasm-bytes-eq-step-8 [left right idx n]
  (let [step1 (wasm-bytes-eq-step left right idx n)
        step2 (continue-wasm-bytes-eq-step left right n step1)
        step3 (continue-wasm-bytes-eq-step left right n step2)
        step4 (continue-wasm-bytes-eq-step left right n step3)
        step5 (continue-wasm-bytes-eq-step left right n step4)
        step6 (continue-wasm-bytes-eq-step left right n step5)
        step7 (continue-wasm-bytes-eq-step left right n step6)
        step8 (continue-wasm-bytes-eq-step left right n step7)]
    step8))
(defn continue-wasm-bytes-eq-step-8 [left right n state]
  (if (= (vector-get state 0) 1)
    state
    (wasm-bytes-eq-step-8 left right (vector-get state 1) n)))
(defn wasm-bytes-eq-step-64 [left right idx n]
  (let [step1 (wasm-bytes-eq-step-8 left right idx n)
        step2 (continue-wasm-bytes-eq-step-8 left right n step1)
        step3 (continue-wasm-bytes-eq-step-8 left right n step2)
        step4 (continue-wasm-bytes-eq-step-8 left right n step3)
        step5 (continue-wasm-bytes-eq-step-8 left right n step4)
        step6 (continue-wasm-bytes-eq-step-8 left right n step5)
        step7 (continue-wasm-bytes-eq-step-8 left right n step6)
        step8 (continue-wasm-bytes-eq-step-8 left right n step7)]
    step8))
(defn wasm-bytes-eq-loop [left right idx n]
  (let [state (wasm-bytes-eq-step-64 left right idx n)]
    (if (= (vector-get state 2) 1)
      0
      (if (= (vector-get state 0) 1)
        1
        (wasm-bytes-eq-loop left right (vector-get state 1) n)))))
(defn wasm-bytes-eq [left right]
  (if (= (vector-length left) (vector-length right))
    (wasm-bytes-eq-loop left right 0 (vector-length left))
    0))
"#
}

fn with_stack_safe_wasm_bytes_eq_helpers(body: &str) -> String {
    format!("{}\n{}", stack_safe_wasm_bytes_eq_helpers(), body)
}

fn assert_selfhost_direct_fixture_with_func_idx_is_deterministic(
    fixture_prefix: &str,
    file_name: &str,
    source: &str,
    label: &str,
    func_idx: usize,
) {
    let dir = cli_test_fixture_dir(fixture_prefix);
    write_cli_fixture_files(&dir, &[(file_name, source)]);
    let fixture_path = dir.join(file_name).to_string_lossy().replace('\\', "\\\\");
    let wasm_bytes_eq_helpers = stack_safe_wasm_bytes_eq_helpers();

    let harness = format!(
        r#"
{wasm_bytes_eq_helpers}
(defn compile-file-state [path]
  (let [src (read-file path)
        program (parse-program src)
        n (vector-length program)
        reg-result (register-defns-chunked program 0 n (ftable-new) {func_idx})
        ftable (vector-get reg-result 2)
        data-ref (ref-new (vector-new 8))
        functions (compile-defn-functions-with-source program 0 n src ftable data-ref (vector-new 8))
        data (ref-get data-ref)]
    (push-object-vector (vector-push (vector-push (vector-new 2) functions) data) program)))
(defn main []
  (let [state1 (compile-file-state "{fixture_path}")
        state2 (compile-file-state "{fixture_path}")
        functions1 (vector-get state1 0)
        data1 (vector-get state1 1)
        functions2 (vector-get state2 0)
        data2 (vector-get state2 1)
        wasm1 (build-wasm-bytes-wasi functions1 data1)
        wasm2 (build-wasm-bytes-wasi functions2 data2)]
    (do
      (print (vector-length wasm1))
      (print (vector-length wasm2))
      (print (wasm-bytes-eq wasm1 wasm2))
      0)))
"#
    );

    let combined = format!("{}\n{}", selfhost_cli_runtime_bundle(), harness);
    let output = compile_and_run(&combined);
    let _ = std::fs::remove_dir_all(&dir);
    let lines: Vec<&str> = output.trim().lines().collect();

    assert!(
        lines.len() >= 3,
        "{} determinism 出力が不足: {:?}",
        label,
        lines
    );
    assert_eq!(
        lines[0], lines[1],
        "2回の {} compile で Wasm 長は一致するべき",
        label
    );
    assert_eq!(
        lines[2], "1",
        "2回の {} compile は byte-identical であるべき: {:?}",
        label, lines
    );
}

fn assert_selfhost_direct_fixture_is_deterministic(
    fixture_prefix: &str,
    file_name: &str,
    source: &str,
    label: &str,
) {
    assert_selfhost_direct_fixture_with_func_idx_is_deterministic(
        fixture_prefix,
        file_name,
        source,
        label,
        0,
    );
}

fn assert_selfhost_direct_fixture_code_section_is_deterministic(
    fixture_prefix: &str,
    file_name: &str,
    source: &str,
    label: &str,
) {
    let dir = cli_test_fixture_dir(fixture_prefix);
    write_cli_fixture_files(&dir, &[(file_name, source)]);
    let fixture_path = dir.join(file_name).to_string_lossy().replace('\\', "\\\\");
    let wasm_bytes_eq_helpers = stack_safe_wasm_bytes_eq_helpers();

    let harness = format!(
        r#"
{wasm_bytes_eq_helpers}
(defn make-byte-fingerprint-state [done next-pos next-acc]
  (push-int-vector-local
    (push-int-vector-local
      (push-int-vector-local (vector-new 3) done)
      next-pos)
    next-acc))
(defn wasm-bytes-fingerprint-step [bytes pos end acc]
  (if (>= pos end)
    (make-byte-fingerprint-state 1 pos acc)
    (make-byte-fingerprint-state 0 (+ pos 1) (+ (* acc 31) (vector-get bytes pos)))))
(defn continue-wasm-bytes-fingerprint-step [bytes end state]
  (if (= (vector-get state 0) 1)
    state
    (wasm-bytes-fingerprint-step bytes (vector-get state 1) end (vector-get state 2))))
(defn wasm-bytes-fingerprint-step-8 [bytes pos end acc]
  (let [step1 (wasm-bytes-fingerprint-step bytes pos end acc)
        step2 (continue-wasm-bytes-fingerprint-step bytes end step1)
        step3 (continue-wasm-bytes-fingerprint-step bytes end step2)
        step4 (continue-wasm-bytes-fingerprint-step bytes end step3)
        step5 (continue-wasm-bytes-fingerprint-step bytes end step4)
        step6 (continue-wasm-bytes-fingerprint-step bytes end step5)
        step7 (continue-wasm-bytes-fingerprint-step bytes end step6)
        step8 (continue-wasm-bytes-fingerprint-step bytes end step7)]
    step8))
(defn continue-wasm-bytes-fingerprint-step-8 [bytes end state]
  (if (= (vector-get state 0) 1)
    state
    (wasm-bytes-fingerprint-step-8 bytes (vector-get state 1) end (vector-get state 2))))
(defn wasm-bytes-fingerprint-step-64 [bytes pos end acc]
  (let [step1 (wasm-bytes-fingerprint-step-8 bytes pos end acc)
        step2 (continue-wasm-bytes-fingerprint-step-8 bytes end step1)
        step3 (continue-wasm-bytes-fingerprint-step-8 bytes end step2)
        step4 (continue-wasm-bytes-fingerprint-step-8 bytes end step3)
        step5 (continue-wasm-bytes-fingerprint-step-8 bytes end step4)
        step6 (continue-wasm-bytes-fingerprint-step-8 bytes end step5)
        step7 (continue-wasm-bytes-fingerprint-step-8 bytes end step6)
        step8 (continue-wasm-bytes-fingerprint-step-8 bytes end step7)]
    step8))
(defn wasm-bytes-fingerprint-loop [bytes pos end acc]
  (let [step (wasm-bytes-fingerprint-step-64 bytes pos end acc)]
    (if (= (vector-get step 0) 1)
      (vector-get step 2)
      (wasm-bytes-fingerprint-loop bytes (vector-get step 1) end (vector-get step 2)))))
(defn wasm-bytes-fingerprint [bytes]
  (wasm-bytes-fingerprint-loop bytes 0 (vector-length bytes) 0))
(defn compile-file-code-section [path]
  (let [src (read-file path)
        program (parse-program src)
        n (vector-length program)
        reg-result (register-defns-chunked program 0 n (ftable-new) 0)
        ftable (vector-get reg-result 2)
        data-ref (ref-new (vector-new 8))
        functions (compile-defn-functions-with-source program 0 n src ftable data-ref (vector-new 8))]
    (emit-code-section-wasi-quad-functions functions)))
(defn main []
  (let [code1 (compile-file-code-section "{fixture_path}")
        code2 (compile-file-code-section "{fixture_path}")]
    (do
      (print (vector-length code1))
      (print (vector-length code2))
      (print (wasm-bytes-fingerprint code1))
      (print (wasm-bytes-fingerprint code2))
      (print (wasm-bytes-eq code1 code2))
      0)))
"#
    );

    let combined = format!("{}\n{}", selfhost_cli_runtime_bundle(), harness);
    let output = compile_and_run(&combined);
    let _ = std::fs::remove_dir_all(&dir);
    let lines: Vec<&str> = output.trim().lines().collect();

    assert!(
        lines.len() >= 5,
        "{} code section determinism 出力が不足: {:?}",
        label,
        lines
    );
    assert_eq!(
        lines[0], lines[1],
        "2回の {} direct compile で code section 長は一致するべき",
        label
    );
    assert_eq!(
        lines[2], lines[3],
        "2回の {} direct compile で code section fingerprint は一致するべき: {:?}",
        label, lines
    );
    assert_eq!(
        lines[4], "1",
        "2回の {} direct compile で code section bytes は一致するべき: {:?}",
        label, lines
    );
}

fn assert_selfhost_direct_fixture_code_section_survives_allocation_history(
    fixture_prefix: &str,
    file_name: &str,
    source: &str,
    label: &str,
) {
    let dir = cli_test_fixture_dir(fixture_prefix);
    write_cli_fixture_files(&dir, &[(file_name, source)]);
    let fixture_path = dir.join(file_name).to_string_lossy().replace('\\', "\\\\");
    let wasm_bytes_eq_helpers = stack_safe_wasm_bytes_eq_helpers();

    let harness = format!(
        r#"
{wasm_bytes_eq_helpers}
(defn make-byte-fingerprint-state [done next-pos next-acc]
  (push-int-vector-local
    (push-int-vector-local
      (push-int-vector-local (vector-new 3) done)
      next-pos)
    next-acc))
(defn wasm-bytes-fingerprint-step [bytes pos end acc]
  (if (>= pos end)
    (make-byte-fingerprint-state 1 pos acc)
    (make-byte-fingerprint-state 0 (+ pos 1) (+ (* acc 31) (vector-get bytes pos)))))
(defn continue-wasm-bytes-fingerprint-step [bytes end state]
  (if (= (vector-get state 0) 1)
    state
    (wasm-bytes-fingerprint-step bytes (vector-get state 1) end (vector-get state 2))))
(defn wasm-bytes-fingerprint-step-8 [bytes pos end acc]
  (let [step1 (wasm-bytes-fingerprint-step bytes pos end acc)
        step2 (continue-wasm-bytes-fingerprint-step bytes end step1)
        step3 (continue-wasm-bytes-fingerprint-step bytes end step2)
        step4 (continue-wasm-bytes-fingerprint-step bytes end step3)
        step5 (continue-wasm-bytes-fingerprint-step bytes end step4)
        step6 (continue-wasm-bytes-fingerprint-step bytes end step5)
        step7 (continue-wasm-bytes-fingerprint-step bytes end step6)
        step8 (continue-wasm-bytes-fingerprint-step bytes end step7)]
    step8))
(defn continue-wasm-bytes-fingerprint-step-8 [bytes end state]
  (if (= (vector-get state 0) 1)
    state
    (wasm-bytes-fingerprint-step-8 bytes (vector-get state 1) end (vector-get state 2))))
(defn wasm-bytes-fingerprint-step-64 [bytes pos end acc]
  (let [step1 (wasm-bytes-fingerprint-step-8 bytes pos end acc)
        step2 (continue-wasm-bytes-fingerprint-step-8 bytes end step1)
        step3 (continue-wasm-bytes-fingerprint-step-8 bytes end step2)
        step4 (continue-wasm-bytes-fingerprint-step-8 bytes end step3)
        step5 (continue-wasm-bytes-fingerprint-step-8 bytes end step4)
        step6 (continue-wasm-bytes-fingerprint-step-8 bytes end step5)
        step7 (continue-wasm-bytes-fingerprint-step-8 bytes end step6)
        step8 (continue-wasm-bytes-fingerprint-step-8 bytes end step7)]
    step8))
(defn wasm-bytes-fingerprint-loop [bytes pos end acc]
  (let [step (wasm-bytes-fingerprint-step-64 bytes pos end acc)]
    (if (= (vector-get step 0) 1)
      (vector-get step 2)
      (wasm-bytes-fingerprint-loop bytes (vector-get step 1) end (vector-get step 2)))))
(defn wasm-bytes-fingerprint [bytes]
  (wasm-bytes-fingerprint-loop bytes 0 (vector-length bytes) 0))
(defn allocation-history-warmup-step [seed]
  (let [items (push-int-vector-local
                (push-int-vector-local
                  (push-int-vector-local (vector-new 3) seed)
                  (+ seed 1))
                (+ seed 2))]
    (+ (+ (vector-get items 0) (vector-get items 1)) (vector-get items 2))))
(defn allocation-history-warmup-loop [idx limit acc]
  (if (>= idx limit)
    acc
    (allocation-history-warmup-loop (+ idx 1) limit (+ acc (allocation-history-warmup-step idx)))))
(defn compile-file-code-section [path]
  (let [src (read-file path)
        program (parse-program src)
        n (vector-length program)
        reg-result (register-defns-chunked program 0 n (ftable-new) 0)
        ftable (vector-get reg-result 2)
        data-ref (ref-new (vector-new 8))
        functions (compile-defn-functions-with-source program 0 n src ftable data-ref (vector-new 8))]
    (emit-code-section-wasi-quad-functions functions)))
(defn main []
  (let [code1 (compile-file-code-section "{fixture_path}")
        warmup (allocation-history-warmup-loop 0 128 0)
        code2 (compile-file-code-section "{fixture_path}")]
    (do
      (print warmup)
      (print (vector-length code1))
      (print (vector-length code2))
      (print (wasm-bytes-fingerprint code1))
      (print (wasm-bytes-fingerprint code2))
      (print (wasm-bytes-eq code1 code2))
      0)))
"#
    );

    let combined = format!("{}\n{}", selfhost_cli_runtime_bundle(), harness);
    let output = compile_and_run(&combined);
    let _ = std::fs::remove_dir_all(&dir);
    let lines: Vec<&str> = output.trim().lines().collect();

    assert!(
        lines.len() >= 6,
        "{} allocation-history code section 出力が不足: {:?}",
        label,
        lines
    );
    assert_ne!(
        lines[0], "0",
        "{} allocation warmup は fixture の割当履歴を変更するべき: {:?}",
        label, lines
    );
    assert_eq!(
        lines[1], lines[2],
        "allocation warmup 後も {} direct compile の code section 長は一致するべき: {:?}",
        label, lines
    );
    assert_eq!(
        lines[3], lines[4],
        "allocation warmup 後も {} direct compile の code section fingerprint は一致するべき: {:?}",
        label, lines
    );
    assert_eq!(
        lines[5], "1",
        "allocation warmup 後も {} direct compile の code section bytes は一致するべき: {:?}",
        label, lines
    );
}

fn assert_selfhost_inline_fixture_with_func_idx_is_deterministic(
    fixture_prefix: &str,
    source: &str,
    label: &str,
    func_idx: usize,
) {
    let dir = cli_test_fixture_dir(fixture_prefix);
    write_cli_fixture_files(
        &dir,
        &[("lsharp.toml", ""), ("src/App/ModuleResolver.ls", source)],
    );
    let wasm_bytes_eq_helpers = stack_safe_wasm_bytes_eq_helpers();

    let harness = format!(
        r#"
{wasm_bytes_eq_helpers}
(defn compile-inline-file-state [path func-idx]
  (let [src (read-file path)
        program (parse-program src)
        source-root (resolve-source-root path)
        package-root (resolve-package-root path)
        seen-ref (ref-new (map-new))
        imported-pairs (load-imports-from-decls program src 0 (vector-length program) seen-ref (vector-new 8) source-root package-root)
        all-pairs (append-src-decl-pair imported-pairs src program)
        n (vector-length all-pairs)
        reg-result (register-all-pairs all-pairs 0 n (ftable-new) {func_idx})
        ftable (vector-get reg-result 0)
        data-ref (ref-new (vector-new 8))
        functions (compile-all-src-decl-pairs all-pairs 0 n ftable data-ref (vector-new 8))
        data (ref-get data-ref)]
    (push-object-vector (vector-push (vector-push (vector-new 2) functions) data) program)))
(defn main []
  (let [state1 (compile-inline-file-state "src/App/ModuleResolver.ls" {func_idx})
        state2 (compile-inline-file-state "src/App/ModuleResolver.ls" {func_idx})
        functions1 (vector-get state1 0)
        data1 (vector-get state1 1)
        functions2 (vector-get state2 0)
        data2 (vector-get state2 1)
        wasm1 (build-wasm-bytes-wasi functions1 data1)
        wasm2 (build-wasm-bytes-wasi functions2 data2)]
    (do
      (print (vector-length wasm1))
      (print (vector-length wasm2))
      (print (wasm-bytes-eq wasm1 wasm2))
      0)))
"#,
    );

    let combined = format!("{}\n{}", selfhost_cli_runtime_bundle(), harness);
    let output = compile_and_run_with_dir(&combined, &dir);
    let _ = std::fs::remove_dir_all(&dir);
    let lines: Vec<&str> = output.trim().lines().collect();

    assert!(
        lines.len() >= 3,
        "{} determinism 出力が不足: {:?}",
        label,
        lines
    );
    assert_eq!(
        lines[0], lines[1],
        "2回の {} compile で Wasm 長は一致するべき",
        label
    );
    assert_eq!(
        lines[2], "1",
        "2回の {} compile は byte-identical であるべき: {:?}",
        label, lines
    );
}

fn assert_selfhost_inline_fixture_is_deterministic(
    fixture_prefix: &str,
    source: &str,
    label: &str,
) {
    assert_selfhost_inline_fixture_with_func_idx_is_deterministic(fixture_prefix, source, label, 7);
}

fn cli_multifile_nested_fixture_files() -> [(&'static str, &'static str); 3] {
    [
        (
            "Support/Base.ls",
            "(module Support.Base)\n(defn base-val [] 10)",
        ),
        (
            "Support/Mid.ls",
            "(module Support.Mid)\n(import Support.Base)\n(defn mid-val [] (* (base-val) 2))",
        ),
        (
            "main.ls",
            "(module Main)\n(import Support.Mid)\n(defn main [] (mid-val))",
        ),
    ]
}

fn cli_lsp_nested_fixture_files() -> [(&'static str, &'static str); 3] {
    [
        (
            "src/Support/Base.ls",
            "(module Support.Base) (defn base-val [] 10)",
        ),
        (
            "src/Support/Mid.ls",
            "(module Support.Mid) (import Support.Base) (defn mid-val [] (base-val))",
        ),
        (
            "src/Main.ls",
            "(module Main) (import Support.Mid) (defn main [] (mid-val))",
        ),
    ]
}

fn make_lsp_did_open_with_path(uri: u32, path: &str, source: &str) -> String {
    format!(
        r#"{{"jsonrpc":"2.0","method":"textDocument/didOpen","params":{{"uri":{},"path":"{}","source":"{}"}}}}"#,
        uri, path, source
    )
}

fn lsp_frame(body: &str) -> String {
    format!("Content-Length: {}\r\n\r\n{}", body.len(), body)
}

fn run_lsp_stdio_with_dir(stdin: &str, dir: &std::path::Path) -> String {
    let wasm = compile_only(selfhost_cli_runtime_bundle());
    lsharp_wasm::wasi_runner::run_wasm_wasi_with_dir_args_and_stdin(
        &wasm,
        Some(dir),
        &["lsp", "--stdio"],
        stdin,
    )
    .expect("filesystem-backed lsp stdio 実行に失敗")
}

fn run_lsp_filesystem_snapshot_request(
    prefix: &str,
    open_uri: u32,
    open_path: &str,
    open_source: &str,
    request_body: &str,
) -> String {
    let dir = cli_test_fixture_dir(prefix);
    write_cli_fixture_files(&dir, &cli_lsp_nested_fixture_files());
    let open_body = make_lsp_did_open_with_path(open_uri, open_path, open_source);
    let stdin = format!("{}{}", lsp_frame(&open_body), lsp_frame(request_body));
    let output = run_lsp_stdio_with_dir(&stdin, &dir);
    let _ = std::fs::remove_dir_all(&dir);
    output
}

fn parse_wasm_size_line(line: &str, context: &str) -> i64 {
    assert!(
        line.starts_with("wasm-size:"),
        "{}: wasm-size:<n> 形式であるべき: {:?}",
        context,
        line
    );
    line["wasm-size:".len()..]
        .parse()
        .unwrap_or_else(|e| panic!("{}: wasm size parse 失敗 {:?}: {}", context, line, e))
}

fn parse_i64_line(line: &str, context: &str) -> i64 {
    line.parse()
        .unwrap_or_else(|e| panic!("{}: integer parse 失敗 {:?}: {}", context, line, e))
}

fn run_cli_multifile_helper_size(dir: &std::path::Path, file_path: &str, target: i64) -> i64 {
    let harness = format!(
        r#"
(defn main []
  (print (compile-file-wasm-size "{file_path}" {target})))
"#
    );
    let combined = format!("{}\n{}", selfhost_cli_runtime_bundle(), harness);
    let output = compile_and_run_with_dir(&combined, dir);
    parse_i64_line(
        output
            .trim()
            .lines()
            .next()
            .expect("compile-file-wasm-size output が必要"),
        "compile-file-wasm-size helper output",
    )
}

/// TEST-CLI-02-C: selfhost/src/App/Cli.ls に repl/lsp/fmt/doc コマンド定義
///
/// T4-4 AC-013: ユーティリティコマンドが L# 実装で動作すること
/// Red Phase: selfhost/src/App/Cli.ls が未作成のため FAIL する。
#[test]
#[ignore]
fn test_e2e_selfhost_cli_repl_lsp_fmt() {
    let cli_path = selfhost_source_path("Cli.ls");
    assert!(cli_path.exists(), "selfhost/src/App/Cli.ls が存在しない");
    let source =
        std::fs::read_to_string(&cli_path).expect("selfhost/src/App/Cli.ls の読み込みに失敗");

    // ユーティリティコマンドの定義を確認 (T4-4 AC-013)
    let commands = ["repl", "lsp", "fmt", "doc"];
    for cmd in &commands {
        assert!(
            source.contains(cmd),
            "selfhost/src/App/Cli.ls に '{}' コマンドの定義がない (AC-013)",
            cmd
        );
    }
}

/// TEST-CLI-02-C2: canonical App/Cli.ls が file-path compile gate を通過すること
#[test]
#[ignore]
fn test_e2e_selfhost_cli_canonical_file_compile() {
    let wasm = compile_file_only(&selfhost_source_path("Cli.ls"));
    assert!(
        wasm.len() > 1000,
        "canonical Cli.ls の Wasm が小さすぎる: {} bytes",
        wasm.len()
    );
}

/// TEST-CLI-01-B: selfhost/src/App/Cli.ls の --help 相当出力が主要コマンドを列挙できること
///
/// T4a-2 AC-104/AC-106: help 出力が usage とサブコマンド一覧を含むこと
#[test]
#[ignore]
fn test_e2e_selfhost_cli_help_output() {
    let harness = r#"
(defn main []
  (do
    (show-help)
    0))
"#;

    let combined = format!("{}\n{}", selfhost_cli_runtime_bundle(), harness);
    let output = compile_and_run(&combined);

    assert!(
        output.contains("Usage: lsharp <command>"),
        "help 出力に usage 行が必要: {:?}",
        output
    );
    for cmd in [
        "parse",
        "check",
        "compile",
        "build",
        "test",
        "review",
        "doc-ack",
        "doc-check",
        "install",
        "repl",
        "lsp",
        "fmt",
        "doc",
    ] {
        assert!(
            output.contains(cmd),
            "help 出力に '{}' が必要: {:?}",
            cmd,
            output
        );
    }
}

/// TEST-CLI-01-B2: selfhost/src/App/Cli.ls の compile target parser helper が preview1/component/alias を区別できること
#[test]
#[ignore]
fn test_e2e_selfhost_cli_compile_target_parser_helper() {
    let harness = r#"
(defn main []
  (do
    (print (parse-compile-target-name "wasi-preview1"))
    (print (parse-compile-target-name "wasi-component"))
    (print (parse-compile-target-name "wasm"))
    (print (parse-compile-target-name "bogus"))
    0))
"#;

    let combined = format!("{}\n{}", selfhost_cli_runtime_bundle(), harness);
    let output = compile_and_run(&combined);
    let lines: Vec<&str> = output.trim().lines().collect();

    assert_eq!(
        lines,
        vec!["0", "1", "1", "-1"],
        "compile target parser helper は preview1/component/alias/invalid を区別するべき: {:?}",
        lines
    );
}

/// TEST-CLI-01-B3: compile/build subcommand help に target option が明示されること
#[test]
#[ignore]
fn test_e2e_selfhost_cli_compile_help_mentions_target_option() {
    let harness = r#"
(defn main []
  (do
    (print-string (format-subcommand-help "compile"))
    (print-string "
")
    (print-string (format-subcommand-help "build"))
    (print-string "
")
    0))
"#;

    let combined = format!("{}\n{}", selfhost_cli_runtime_bundle(), harness);
    let output = compile_and_run(&combined);
    let lines: Vec<&str> = output.trim().lines().collect();

    assert!(lines.len() >= 2, "subcommand help 出力が不足: {:?}", lines);
    assert!(
        lines[0].contains("--target"),
        "compile help は --target option を案内するべき: {:?}",
        lines[0]
    );
    assert!(
        lines[1].contains("--target"),
        "build help は --target option を案内するべき: {:?}",
        lines[1]
    );
}

/// TEST-CLI-01-C: selfhost/src/App/Cli.ls の --version 相当出力が `lsharp x.y.z` 形式であること
///
/// T4a-2 AC-105: version 出力形式を固定する
#[test]
#[ignore]
fn test_e2e_selfhost_cli_version_output() {
    let harness = r#"
(defn main []
  (do
    (show-version)
    0))
"#;

    let combined = format!("{}\n{}", selfhost_cli_runtime_bundle(), harness);
    let output = compile_and_run(&combined);

    assert_eq!(output.trim(), "lsharp 0.1.0");
}

/// TEST-CLI-02-D: selfhost/src/App/Cli.ls の parse core helper が source を parse できること
///
/// CLI-02 の最小 tranche として、file I/O 抜きで parse-program を CLI helper へ接続する。
#[test]
#[ignore]
fn test_e2e_selfhost_cli_parse_source_core() {
    let harness = r#"
(defn main []
  (do
    (print (run-parse-source "(defn main [] 42)" 0))
    0))
"#;

    let combined = format!("{}\n{}", selfhost_cli_runtime_bundle(), harness);
    let output = compile_and_run(&combined);
    let lines: Vec<&str> = output.trim().lines().collect();

    assert!(lines.len() >= 5, "cli parse core 出力が不足: {:?}", lines);
    assert_eq!(
        lines[0], "decls:1",
        "program decl-count text は 1 であるべき"
    );
    assert_eq!(
        lines[1], "first-decl:defn",
        "先頭 decl は defn text であるべき"
    );
    assert_eq!(
        lines[2], "first-body:int",
        "defn body は int text であるべき"
    );
    assert_eq!(
        lines[3], "diagnostics:0",
        "parse diagnostics summary は 0 件であるべき"
    );
    assert_eq!(
        lines[4], "0",
        "run-parse-source の終了コードは success であるべき"
    );
}

/// TEST-CLI-02-E: selfhost/src/App/Cli.ls の check core helper が source を型推論できること
///
/// CLI-02 の最小 tranche として、file I/O 抜きで TypeInfer.infer を CLI helper へ接続する。
#[test]
#[ignore]
fn test_e2e_selfhost_cli_check_source_core() {
    let harness = r#"
(defn main []
  (do
    (print (run-check-source "(defn main [] 42)" 0))
    0))
"#;

    let combined = format!("{}\n{}", selfhost_cli_runtime_bundle(), harness);
    let output = compile_and_run(&combined);
    let lines: Vec<&str> = output.trim().lines().collect();

    assert!(lines.len() >= 3, "cli check core 出力が不足: {:?}", lines);
    assert_eq!(lines[0], "Int", "check 結果は型名 Int を返すべき");
    assert_eq!(
        lines[1], "diagnostics:0",
        "check diagnostics summary は 0 件であるべき"
    );
    assert_eq!(
        lines[2], "0",
        "run-check-source の終了コードは success であるべき"
    );
}

/// TEST-CLI-02-F: selfhost/src/App/Cli.ls の run-parse が file-path から source を読めること
#[test]
#[ignore]
fn test_e2e_selfhost_cli_parse_file_handler() {
    let dir =
        std::env::temp_dir().join(format!("lsharp_test_cli_parse_file_{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    std::fs::write(dir.join("input.ls"), "(defn main [] 42)").unwrap();

    let harness = r#"
(defn main []
  (do
    (print (run-parse "input.ls" 0))
    0))
"#;

    let combined = format!("{}\n{}", selfhost_cli_runtime_bundle(), harness);
    let output = compile_and_run_with_dir(&combined, &dir);
    let _ = std::fs::remove_dir_all(&dir);
    let lines: Vec<&str> = output.trim().lines().collect();

    assert!(
        lines.len() >= 5,
        "cli parse file handler 出力が不足: {:?}",
        lines
    );
    assert_eq!(
        lines[0], "decls:1",
        "program decl-count text は 1 であるべき"
    );
    assert_eq!(
        lines[1], "first-decl:defn",
        "先頭 decl は defn text であるべき"
    );
    assert_eq!(
        lines[2], "first-body:int",
        "defn body は int text であるべき"
    );
    assert_eq!(
        lines[3], "diagnostics:0",
        "parse diagnostics summary は 0 件であるべき"
    );
    assert_eq!(lines[4], "0", "run-parse の終了コードは success であるべき");
}

/// TEST-CLI-02-G: selfhost/src/App/Cli.ls の run-check が file-path から source を読めること
#[test]
#[ignore]
fn test_e2e_selfhost_cli_check_file_handler() {
    let dir =
        std::env::temp_dir().join(format!("lsharp_test_cli_check_file_{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    std::fs::write(dir.join("input.ls"), "(defn main [] 42)").unwrap();

    let harness = r#"
(defn main []
  (do
    (print (run-check "input.ls" 0))
    0))
"#;

    let combined = format!("{}\n{}", selfhost_cli_runtime_bundle(), harness);
    let output = compile_and_run_with_dir(&combined, &dir);
    let _ = std::fs::remove_dir_all(&dir);
    let lines: Vec<&str> = output.trim().lines().collect();

    assert!(
        lines.len() >= 3,
        "cli check file handler 出力が不足: {:?}",
        lines
    );
    assert_eq!(lines[0], "Int", "check 結果は型名 Int を返すべき");
    assert_eq!(
        lines[1], "diagnostics:0",
        "check diagnostics summary は 0 件であるべき"
    );
    assert_eq!(lines[2], "0", "run-check の終了コードは success であるべき");
}

/// TEST-CLI-02-G2: run-parse-source が recovery 入力でも diagnostics summary を返すこと
#[test]
#[ignore]
fn test_e2e_selfhost_cli_parse_source_recovery_summary() {
    let harness = r#"
(defn main []
  (do
    (print (run-parse-source ")" 0))
    0))
"#;

    let combined = format!("{}\n{}", selfhost_cli_runtime_bundle(), harness);
    let output = compile_and_run(&combined);
    let lines: Vec<&str> = output.trim().lines().collect();

    assert!(
        lines.len() >= 2,
        "cli parse recovery 出力が不足: {:?}",
        lines
    );
    assert!(
        lines.contains(&"diagnostics:1,P0001@1:1,first-body:unexpected token )"),
        "parse recovery summary は code/location を含むべき: {:?}",
        lines
    );
    assert_eq!(
        lines.last(),
        Some(&"0"),
        "run-parse-source は recovery summary 後も success を返すべき"
    );
}

/// TEST-CLI-02-G2b: run-parse-source が `]` recovery でも token 別 diagnostics body を返すこと
#[test]
#[ignore]
fn test_e2e_selfhost_cli_parse_source_recovery_unexpected_bracket_summary() {
    let harness = r#"
(defn main []
  (do
    (print (run-parse-source "]" 0))
    0))
"#;

    let combined = format!("{}\n{}", selfhost_cli_runtime_bundle(), harness);
    let output = compile_and_run(&combined);
    let lines: Vec<&str> = output.trim().lines().collect();

    assert!(
        lines.len() >= 2,
        "cli parse recovery `]` 出力が不足: {:?}",
        lines
    );
    assert!(
        lines.contains(&"diagnostics:1,P0001@1:1,first-body:unexpected token ]"),
        "parse recovery summary は unexpected token ] を含むべき: {:?}",
        lines
    );
    assert_eq!(
        lines.last(),
        Some(&"0"),
        "run-parse-source は recovery summary 後も success を返すべき"
    );
}

/// TEST-CLI-02-G3: run-check-source が型エラー入力でも diagnostics summary を返すこと
#[test]
#[ignore]
fn test_e2e_selfhost_cli_check_source_type_error_summary() {
    let harness = r#"
(defn main []
  (do
    (print (run-check-source "(defn main [] (if 42 1 0))" 0))
    0))
"#;

    let combined = format!("{}\n{}", selfhost_cli_runtime_bundle(), harness);
    let output = compile_and_run(&combined);
    let lines: Vec<&str> = output.trim().lines().collect();

    assert!(
        lines.len() >= 3,
        "cli check type-error 出力が不足: {:?}",
        lines
    );
    assert!(
        lines.contains(&"diagnostics:1,T0001@1:1,first-body:if condition must be Bool"),
        "check type-error summary は code/location を含むべき: {:?}",
        lines
    );
    assert_eq!(
        lines.last(),
        Some(&"1"),
        "run-check-source は type-error summary 後に compile error を返すべき"
    );
}

/// TEST-CLI-02-G3b: run-check-source が未定義シンボルでも code 別 diagnostics body を返すこと
#[test]
#[ignore]
fn test_e2e_selfhost_cli_check_source_undefined_symbol_summary() {
    let harness = r#"
(defn main []
  (do
    (print (run-check-source "(defn main [] missing)" 0))
    0))
"#;

    let combined = format!("{}\n{}", selfhost_cli_runtime_bundle(), harness);
    let output = compile_and_run(&combined);
    let lines: Vec<&str> = output.trim().lines().collect();

    assert!(
        lines.len() >= 3,
        "cli check undefined-symbol 出力が不足: {:?}",
        lines
    );
    assert!(
        lines.contains(&"diagnostics:1,T0001@1:1,first-body:undefined symbol"),
        "check undefined-symbol summary は code 別 body を含むべき: {:?}",
        lines
    );
    assert_eq!(
        lines.last(),
        Some(&"1"),
        "run-check-source は diagnostics summary 後に compile error を返すべき"
    );
}

/// TEST-CLI-02-H: selfhost/src/App/Cli.ls の file-path handler は missing file を compile error で返す
#[test]
#[ignore]
fn test_e2e_selfhost_cli_file_handler_missing_file() {
    let dir = std::env::temp_dir().join(format!(
        "lsharp_test_cli_missing_file_{}",
        std::process::id()
    ));
    std::fs::create_dir_all(&dir).unwrap();

    let harness = r#"
(defn main []
  (do
    (print (run-parse "missing.ls" 0))
    (print (run-check "missing.ls" 0))
    (print (run-build "missing.ls" 0))
    (print (run-test "missing.ls" 0))
    (print (run-review "missing.ls" 0))
    (print (run-fmt "missing.ls" 0))
    (print (run-compile "missing.ls" 0))
    (print (run-doc-ack "missing.ls" 0))
    (print (run-doc-check "missing.ls" 0))
    (print (run-doc "missing.ls" 0))
    0))
"#;

    let combined = format!("{}\n{}", selfhost_cli_runtime_bundle(), harness);
    let output = compile_and_run_with_dir(&combined, &dir);
    let _ = std::fs::remove_dir_all(&dir);
    let lines: Vec<&str> = output.trim().lines().collect();

    assert_eq!(
        lines,
        vec!["1", "1", "1", "1", "1", "1", "1", "1", "1", "1"],
        "missing file は parse/check/build/test/review/fmt/compile/doc-ack/doc-check/doc とも compile error=1 を返すべき"
    );
}

/// TEST-CLI-02-I: selfhost/src/App/Cli.ls の arg-parse がコマンド文字列を command id へ変換できること
#[test]
#[ignore]
fn test_e2e_selfhost_cli_arg_parse_strings() {
    let harness = r#"
(defn main []
  (do
    (print (arg-parse "parse"))
    (print (arg-parse "check"))
    (print (arg-parse "compile"))
    (print (arg-parse "doc"))
    (print (arg-parse "unknown"))
    0))
"#;

    let combined = format!("{}\n{}", selfhost_cli_runtime_bundle(), harness);
    let output = compile_and_run(&combined);
    let lines: Vec<&str> = output.trim().lines().collect();

    assert_eq!(
        lines,
        vec!["1", "2", "3", "13", "0"],
        "arg-parse は既知コマンドを対応する id へ変換し、未知コマンドは 0 を返すべき"
    );
}

/// TEST-CLI-02-J: selfhost/src/App/Cli.ls の run-fmt-source が format-program を呼べること
#[test]
#[ignore]
fn test_e2e_selfhost_cli_fmt_source_core() {
    let harness = r#"
(defn main []
  (do
    (print (run-fmt-source "(defn a [] 42)" 0))
    0))
"#;

    let combined = format!("{}\n{}", selfhost_cli_runtime_bundle(), harness);
    let output = compile_and_run(&combined);
    let lines: Vec<&str> = output.trim().lines().collect();

    assert_eq!(
        lines.len(),
        2,
        "run-fmt-source は 1 つの fmt 出力と success code を返すべき"
    );
    assert_eq!(
        lines[0], "(defn a [] 42)",
        "run-fmt-source は format-program の canonical text を stdout へ返すべき"
    );
    assert_eq!(lines[1], "0", "run-fmt-source は success=0 を返すべき");
}

/// TEST-CLI-02-J2: run-fmt-source が string literal を fallback せず返すこと
#[test]
#[ignore]
fn test_e2e_selfhost_cli_fmt_source_string_literal() {
    let harness = r#"
(defn main []
  (do
    (print (run-fmt-source "\"abc\"" 0))
    0))
"#;

    let combined = format!("{}\n{}", selfhost_cli_runtime_bundle(), harness);
    let output = compile_and_run(&combined);
    let lines: Vec<&str> = output.trim().lines().collect();

    assert_eq!(
        lines.len(),
        2,
        "run-fmt-source string literal は fmt 出力と success code を返すべき"
    );
    assert_eq!(
        lines[0], "\"abc\"",
        "run-fmt-source は string literal を source-aware formatter で返すべき"
    );
    assert_eq!(lines[1], "0", "run-fmt-source は success=0 を返すべき");
}

/// TEST-CLI-02-J3: run-fmt-source string literal output を snapshot に固定すること
#[test]
#[ignore]
fn test_e2e_selfhost_cli_fmt_source_string_literal_snapshot() {
    let harness = r#"
(defn main []
  (do
    (print (run-fmt-source "\"abc\"" 0))
    0))
"#;

    let combined = format!("{}\n{}", selfhost_cli_runtime_bundle(), harness);
    let output = compile_and_run(&combined);

    assert_cli_text_snapshot(
        &output,
        "fmt-source-string-literal.txt",
        "run-fmt-source string literal output は representative text snapshot と一致するべき",
    );
}

/// TEST-CLI-02-J4: run-fmt-source が defn metadata を canonical 順で保持すること
#[test]
#[ignore]
fn test_e2e_selfhost_cli_fmt_source_defn_metadata() {
    let harness = r#"
(defn main []
  (do
    (print (run-fmt-source "(defn add [x y] :doc \"Add two ints\" :params [(x \"left\") (y \"right\")] :returns \"sum\" :example [(add 1 2)] (+ x y))" 0))
    0))
"#;

    let combined = format!("{}\n{}", selfhost_cli_runtime_bundle(), harness);
    let output = compile_and_run(&combined);
    let lines: Vec<&str> = output.trim().lines().collect();

    assert_eq!(
        lines.len(),
        2,
        "run-fmt-source metadata は fmt 出力と success code を返すべき"
    );
    assert_eq!(
        lines[0],
        "(defn add [x y] :params [(x \"left\") (y \"right\")] :returns \"sum\" :doc \"Add two ints\" :example [(add 1 2)] (+ x y))",
        "run-fmt-source は defn metadata を canonical 順で返すべき"
    );
    assert_eq!(lines[1], "0", "run-fmt-source は success=0 を返すべき");
}

/// TEST-CLI-02-K: selfhost/src/App/Cli.ls の run-fmt が file-path から source を読めること
#[test]
#[ignore]
fn test_e2e_selfhost_cli_fmt_file_handler() {
    let dir = std::env::temp_dir().join(format!("lsharp_test_cli_fmt_file_{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    std::fs::write(dir.join("input.ls"), "(defn a [] 42)").unwrap();

    let harness = r#"
(defn main []
  (do
    (print (run-fmt "input.ls" 0))
    0))
"#;

    let combined = format!("{}\n{}", selfhost_cli_runtime_bundle(), harness);
    let output = compile_and_run_with_dir(&combined, &dir);
    let _ = std::fs::remove_dir_all(&dir);
    let lines: Vec<&str> = output.trim().lines().collect();

    assert_eq!(
        lines.len(),
        2,
        "run-fmt は 1 つの fmt 出力と success code を返すべき"
    );
    assert_eq!(
        lines[0], "(defn a [] 42)",
        "run-fmt は file-path 経由でも canonical text を stdout へ返すべき"
    );
    assert_eq!(lines[1], "0", "run-fmt は success=0 を返すべき");
}

/// TEST-CLI-02-L: selfhost/src/App/Cli.ls の run-compile-source が compile PoC を呼べること
#[test]
#[ignore]
fn test_e2e_selfhost_cli_compile_source_core() {
    let harness = r#"
(defn main []
  (do
    (print (run-compile-source "(defn main [] 42)" 0))
    0))
"#;

    let combined = format!("{}\n{}", selfhost_cli_runtime_bundle(), harness);
    let output = compile_and_run(&combined);
    let lines: Vec<&str> = output.trim().lines().collect();

    assert!(
        lines.len() >= 2,
        "run-compile-source 出力が不足: {:?}",
        lines
    );
    assert!(
        lines[0].starts_with("wasm-size:"),
        "run-compile-source は wasm-size:<n> を返すべき: {:?}",
        lines
    );
    let wasm_size: i64 = lines[0]["wasm-size:".len()..]
        .parse()
        .expect("wasm size は整数であるべき");
    assert!(
        wasm_size > 8,
        "wasm size は header 超であるべき: {}",
        wasm_size
    );
    assert_eq!(
        lines[1], "0",
        "run-compile-source の終了コードは success であるべき"
    );
}

/// TEST-CLI-02-L2: emit-wasm-with-target が preview1/component で size を切り替えること
#[test]
#[ignore]
fn test_e2e_selfhost_cli_emit_wasm_with_target_changes_wasm_size() {
    let harness = r#"
(defn main []
  (let [program (parse-program "(defn main [] 42)")
    ir (lower program)]
    (do
      (print (emit-wasm-with-target ir (compile-target-preview1)))
      (print (emit-wasm-with-target ir (compile-target-component)))
      0)))
"#;

    let combined = format!("{}\n{}", selfhost_cli_runtime_bundle(), harness);
    let output = compile_and_run(&combined);
    let lines: Vec<&str> = output.trim().lines().collect();

    assert_eq!(
        lines.len(),
        2,
        "target 別 wasm size が 2 行必要: {:?}",
        lines
    );
    let preview1_size: i64 = lines[0]
        .parse()
        .expect("preview1 wasm size は整数であるべき");
    let component_size: i64 = lines[1]
        .parse()
        .expect("component wasm size は整数であるべき");
    assert!(
        preview1_size > component_size,
        "preview1 target は component target より大きい import layout を持つべき: preview1={preview1_size}, component={component_size}"
    );
}

/// TEST-CLI-02-M: selfhost/src/App/Cli.ls の run-compile が file-path から source を読めること
#[test]
#[ignore]
fn test_e2e_selfhost_cli_compile_file_handler() {
    let dir = std::env::temp_dir().join(format!(
        "lsharp_test_cli_compile_file_{}",
        std::process::id()
    ));
    std::fs::create_dir_all(&dir).unwrap();
    std::fs::write(dir.join("input.ls"), "(defn main [] 42)").unwrap();

    let harness = r#"
(defn main []
  (do
    (print (run-compile "input.ls" 0))
    0))
"#;

    let combined = format!("{}\n{}", selfhost_cli_runtime_bundle(), harness);
    let output = compile_and_run_with_dir(&combined, &dir);
    let _ = std::fs::remove_dir_all(&dir);
    let lines: Vec<&str> = output.trim().lines().collect();

    assert!(lines.len() >= 2, "run-compile 出力が不足: {:?}", lines);
    assert!(
        lines[0].starts_with("wasm-size:"),
        "run-compile は wasm-size:<n> を返すべき: {:?}",
        lines
    );
    let wasm_size: i64 = lines[0]["wasm-size:".len()..]
        .parse()
        .expect("wasm size は整数であるべき");
    assert!(
        wasm_size > 8,
        "wasm size は header 超であるべき: {}",
        wasm_size
    );
    assert_eq!(
        lines[1], "0",
        "run-compile の終了コードは success であるべき"
    );
}

/// TEST-CLI-02-M1B: selfhost/src/App/Cli.ls の run-compile は nested import fixture を import-aware helper 経由で解決すること
#[test]
#[ignore]
fn test_e2e_selfhost_cli_compile_file_handler_multifile_nested_imports() {
    let dir = cli_test_fixture_dir("compile_multifile_nested");
    write_cli_fixture_files(&dir, &cli_multifile_nested_fixture_files());

    let harness = r#"
(defn main []
  (let [src (read-file "main.ls")]
    (do
      (print (run-compile "main.ls" 0))
      (print (compile-file-wasm-size "main.ls" 0))
      (print (run-compile-source src 0))
      0)))
"#;

    let combined = format!("{}\n{}", selfhost_cli_runtime_bundle(), harness);
    let output = compile_and_run_with_dir(&combined, &dir);
    let _ = std::fs::remove_dir_all(&dir);
    let lines: Vec<&str> = output.trim().lines().collect();

    assert!(
        lines.len() >= 5,
        "run-compile multi-file nested fixture 出力が不足: {:?}",
        lines
    );
    let file_size = parse_wasm_size_line(lines[0], "run-compile multi-file nested fixture");
    let helper_size = parse_i64_line(lines[2], "compile-file-wasm-size nested fixture");
    let source_only_size =
        parse_wasm_size_line(lines[3], "run-compile-source nested fixture baseline");
    assert_eq!(lines[1], "0", "run-compile は success=0 を返すべき");
    assert_eq!(
        lines[4], "0",
        "run-compile-source baseline は success=0 を返すべき"
    );
    assert!(
        file_size == helper_size,
        "run-compile は import-aware helper と同じ wasm-size を返すべき: cli={file_size}, helper={helper_size}"
    );
    assert!(
        helper_size > source_only_size,
        "compile-file-wasm-size helper は source-only baseline より大きい wasm-size を返すべき: helper={helper_size}, source-only={source_only_size}"
    );
}

/// TEST-CLI-02-M1C: selfhost/src/App/Cli.ls は shared cache helper 経由で clean hit 時の再 parse を避けること
#[test]
#[ignore]
fn test_e2e_selfhost_cli_compile_functions_data_with_cache_reuses_clean_hit() {
    let dir = cli_test_fixture_dir("compile_functions_data_cache");
    write_cli_fixture_files(&dir, &cli_multifile_nested_fixture_files());

    let harness = r#"
(defn main []
  (let [cache-ref (ref-new (map-new))
        parse-count-ref (ref-new 0)
        pair1 (compile-file-functions-data-with-cache "main.ls" cache-ref parse-count-ref)
        count1 (ref-get parse-count-ref)
        pair2 (compile-file-functions-data-with-cache "main.ls" cache-ref parse-count-ref)
        count2 (ref-get parse-count-ref)
        functions1 (vector-get pair1 0)
        data1 (vector-get pair1 1)
        functions2 (vector-get pair2 0)
        data2 (vector-get pair2 1)]
    (do
      (print count1)
      (print count2)
      (print (vector-length functions1))
      (print (vector-length functions2))
      (print (vector-length data1))
      (print (vector-length data2))
      0)))
"#;

    let combined = format!("{}\n{}", selfhost_cli_runtime_bundle(), harness);
    let output = compile_and_run_with_dir(&combined, &dir);
    let _ = std::fs::remove_dir_all(&dir);
    let lines: Vec<&str> = output.trim().lines().collect();

    assert!(
        lines.len() >= 6,
        "compile-file-functions-data-with-cache 出力が不足: {:?}",
        lines
    );
    assert_eq!(
        lines[0], "3",
        "初回 compile では main/mid/base の 3 モジュールを parse するべき"
    );
    assert_eq!(lines[1], "3", "clean hit では parse-count が増えないべき");
    assert_eq!(lines[2], "3", "functions1 は 3 個保持するべき");
    assert_eq!(lines[3], "3", "functions2 は 3 個保持するべき");
    assert_eq!(
        lines[4], lines[5],
        "data section 長は cache hit 前後で一致するべき"
    );
}

/// TEST-CLI-02-M1D: selfhost/src/App/Cli.ls は shared cache helper で module path invalidation を反映すること
#[test]
#[ignore]
fn test_e2e_selfhost_cli_compile_functions_data_with_cache_invalidates_changed_module_path() {
    let dir = cli_test_fixture_dir("compile_functions_data_cache_invalidation");
    write_cli_fixture_files(
        &dir,
        &[
            (
                "src/Main.ls",
                "(module Main)\n(import App.Lib)\n(defn main [] (helper))",
            ),
            ("vendor/App/Lib.ls", "(module App.Lib)\n(defn helper [] 7)"),
            (".lsharp/module-index/App/Lib.path", "vendor/App/Lib.ls"),
            (
                "src/App/Placeholder.ls",
                "(module App.Placeholder)\n(defn unused [] 0)",
            ),
        ],
    );

    let harness = r#"
(defn main []
  (let [cache-ref (ref-new (map-new))
        parse-count-ref (ref-new 0)
        pair1 (compile-file-functions-data-with-cache "src/Main.ls" cache-ref parse-count-ref)
        count1 (ref-get parse-count-ref)
        _ (write-file "src/App/Lib.ls" "(module App.Lib) (defn helper [] 9)")
        pair2 (compile-file-functions-data-with-cache "src/Main.ls" cache-ref parse-count-ref)
        count2 (ref-get parse-count-ref)
        functions1 (vector-get pair1 0)
        functions2 (vector-get pair2 0)]
    (do
      (print count1)
      (print count2)
      (print (vector-length functions1))
      (print (vector-length functions2))
      0)))
"#;

    let combined = format!("{}\n{}", selfhost_cli_runtime_bundle(), harness);
    let output = compile_and_run_with_dir(&combined, &dir);
    let _ = std::fs::remove_dir_all(&dir);
    let lines: Vec<&str> = output.trim().lines().collect();

    assert!(
        lines.len() >= 4,
        "compile-file-functions-data-with-cache invalidation 出力が不足: {:?}",
        lines
    );
    assert_eq!(
        lines[0], "2",
        "初回 compile では main と vendor lib を parse するべき"
    );
    assert_eq!(
        lines[1], "3",
        "module path 更新後は local lib だけ再 parse するべき"
    );
    assert_eq!(lines[2], "2", "functions1 は 2 個保持するべき");
    assert_eq!(lines[3], "2", "functions2 も 2 個保持するべき");
}

/// TEST-CLI-02-M1D: selfhost cached helper は changed module 後も fresh compile と同じ Wasm を返すこと
#[test]
#[ignore]
fn test_e2e_selfhost_cli_compile_functions_data_with_cache_matches_fresh_compile_after_change() {
    let dir = cli_test_fixture_dir("compile_functions_data_cache_dirty_parity");
    write_cli_fixture_files(
        &dir,
        &[
            (
                "src/Main.ls",
                "(module Main)\n(import App.Lib)\n(defn main [] (helper))",
            ),
            ("vendor/App/Lib.ls", "(module App.Lib)\n(defn helper [] 7)"),
            (".lsharp/module-index/App/Lib.path", "vendor/App/Lib.ls"),
            (
                "src/App/Placeholder.ls",
                "(module App.Placeholder)\n(defn unused [] 0)",
            ),
        ],
    );

    let harness = with_stack_safe_wasm_bytes_eq_helpers(
        r#"
(defn main []
  (let [cache-ref (ref-new (map-new))
        parse-count-ref (ref-new 0)
        payload1 (compile-file-functions-data-with-cache "src/Main.ls" cache-ref parse-count-ref)
        count1 (ref-get parse-count-ref)
        functions1 (vector-get payload1 0)
        data1 (vector-get payload1 1)
        wasm1 (build-wasm-bytes-wasi functions1 data1)
        _ (write-file "src/App/Lib.ls" "(module App.Lib) (defn helper [] 9)")
        payload2 (compile-file-functions-data-with-cache "src/Main.ls" cache-ref parse-count-ref)
        count2 (ref-get parse-count-ref)
        functions2 (vector-get payload2 0)
        data2 (vector-get payload2 1)
        wasm2 (build-wasm-bytes-wasi functions2 data2)
        fresh-payload (compile-file-functions-data "src/Main.ls")
        fresh-functions (vector-get fresh-payload 0)
        fresh-data (vector-get fresh-payload 1)
        fresh-wasm (build-wasm-bytes-wasi fresh-functions fresh-data)]
    (do
      (print count1)
      (print count2)
      (print (wasm-bytes-eq wasm1 wasm2))
      (print (wasm-bytes-eq wasm2 fresh-wasm))
      0)))
"#,
    );

    let combined = format!("{}\n{}", selfhost_cli_runtime_bundle(), harness);
    let output = compile_and_run_with_dir(&combined, &dir);
    let _ = std::fs::remove_dir_all(&dir);
    let lines: Vec<&str> = output.trim().lines().collect();

    assert!(
        lines.len() >= 4,
        "compile-file-functions-data-with-cache dirty parity 出力が不足: {:?}",
        lines
    );
    assert_eq!(
        lines[0], "2",
        "初回 compile では main と vendor lib を parse するべき"
    );
    assert_eq!(
        lines[1], "3",
        "changed module 後は local lib だけ追加で再 parse するべき"
    );
    assert_eq!(
        lines[2], "0",
        "changed module 後の cached compile は initial output から変化するべき"
    );
    assert_eq!(
        lines[3], "1",
        "changed module 後の cached compile は fresh compile と byte-identical であるべき"
    );
}

/// TEST-CLI-02-M1E: selfhost direct multi-file compile は同じ入力を 2 回与えても同じ Wasm を返すこと
#[test]
#[ignore]
fn test_e2e_selfhost_cli_direct_multifile_compile_is_deterministic() {
    let dir = cli_test_fixture_dir("compile_functions_data_direct_determinism");
    write_cli_fixture_files(
        &dir,
        &[
            (
                "src/Main.ls",
                "(module Main)\n(import App.Lib)\n(defn main [] (helper))",
            ),
            ("vendor/App/Lib.ls", "(module App.Lib)\n(defn helper [] 7)"),
            (".lsharp/module-index/App/Lib.path", "vendor/App/Lib.ls"),
            (
                "src/App/Placeholder.ls",
                "(module App.Placeholder)\n(defn unused [] 0)",
            ),
        ],
    );

    let harness = with_stack_safe_wasm_bytes_eq_helpers(
        r#"
(defn make-ir-fingerprint-state [done next-idx next-acc]
  (push-int-vector-local
    (push-int-vector-local
      (push-int-vector-local (vector-new 3) done)
      next-idx)
    next-acc))
(defn ir-fingerprint-step [ir idx count acc]
  (if (>= idx count)
    (make-ir-fingerprint-state 1 idx acc)
    (let [instr (vector-get ir idx)
          opcode (vector-get instr 0)
          operand (vector-get instr 1)]
      (make-ir-fingerprint-state 0 (+ idx 1) (+ (* (+ (* acc 31) opcode) 31) operand)))))
(defn continue-ir-fingerprint-step [ir count state]
  (if (= (vector-get state 0) 1)
    state
    (ir-fingerprint-step ir (vector-get state 1) count (vector-get state 2))))
(defn ir-fingerprint-step-8 [ir idx count acc]
  (let [step1 (ir-fingerprint-step ir idx count acc)
        step2 (continue-ir-fingerprint-step ir count step1)
        step3 (continue-ir-fingerprint-step ir count step2)
        step4 (continue-ir-fingerprint-step ir count step3)
        step5 (continue-ir-fingerprint-step ir count step4)
        step6 (continue-ir-fingerprint-step ir count step5)
        step7 (continue-ir-fingerprint-step ir count step6)
        step8 (continue-ir-fingerprint-step ir count step7)]
    step8))
(defn continue-ir-fingerprint-step-8 [ir count state]
  (if (= (vector-get state 0) 1)
    state
    (ir-fingerprint-step-8 ir (vector-get state 1) count (vector-get state 2))))
(defn ir-fingerprint-step-64 [ir idx count acc]
  (let [step1 (ir-fingerprint-step-8 ir idx count acc)
        step2 (continue-ir-fingerprint-step-8 ir count step1)
        step3 (continue-ir-fingerprint-step-8 ir count step2)
        step4 (continue-ir-fingerprint-step-8 ir count step3)
        step5 (continue-ir-fingerprint-step-8 ir count step4)
        step6 (continue-ir-fingerprint-step-8 ir count step5)
        step7 (continue-ir-fingerprint-step-8 ir count step6)
        step8 (continue-ir-fingerprint-step-8 ir count step7)]
    step8))
(defn ir-fingerprint-loop [ir idx count acc]
  (let [step (ir-fingerprint-step-64 ir idx count acc)]
    (if (= (vector-get step 0) 1)
      (vector-get step 2)
      (ir-fingerprint-loop ir (vector-get step 1) count (vector-get step 2)))))
(defn ir-fingerprint [ir]
  (ir-fingerprint-loop ir 0 (vector-length ir) 0))
(defn function-fingerprint [func]
  (let [param-count (vector-get func 0)
        local-count (vector-get func 1)
        ir (vector-get func 2)]
    (+ (* (+ (* param-count 31) local-count) 31) (ir-fingerprint ir))))
(defn first-function-mismatch [functions1 functions2 idx count]
  (if (>= idx count)
    -1
    (if (= (function-fingerprint (vector-get functions1 idx)) (function-fingerprint (vector-get functions2 idx)))
      (first-function-mismatch functions1 functions2 (+ idx 1) count)
      idx)))
(defn first-ir-mismatch [ir1 ir2 idx count]
  (if (>= idx count)
    -1
    (let [instr1 (vector-get ir1 idx)
          instr2 (vector-get ir2 idx)]
      (if (and (= (vector-get instr1 0) (vector-get instr2 0)) (= (vector-get instr1 1) (vector-get instr2 1)))
        (first-ir-mismatch ir1 ir2 (+ idx 1) count)
        idx))))
(defn print-ir-instr [ir idx]
  (if (< idx 0)
    (do
      (print -1)
      (print -1)
      (print -1)
      0)
    (if (>= idx (vector-length ir))
      (do
        (print -1)
        (print -1)
        (print -1)
        0)
      (let [instr (vector-get ir idx)]
        (do
          (print idx)
          (print (vector-get instr 0))
          (print (vector-get instr 1))
          0)))))
(defn nth-defn [program idx seen target]
  (if (>= idx (vector-length program))
    (vector-new 5)
    (let [decl (vector-get program idx)]
      (if (= (vector-get decl 0) 20)
        (if (= seen target)
          decl
          (nth-defn program (+ idx 1) (+ seen 1) target))
        (nth-defn program (+ idx 1) seen target)))))
(defn nth-defn-name-hash [program idx seen target]
  (if (>= idx (vector-length program))
    -1
    (let [decl (vector-get program idx)]
      (if (= (vector-get decl 0) 20)
        (if (= seen target)
          (vector-get decl 1)
          (nth-defn-name-hash program (+ idx 1) (+ seen 1) target))
        (nth-defn-name-hash program (+ idx 1) seen target)))))
(defn nth-defn [program idx seen target]
  (if (>= idx (vector-length program))
    (vector-new 5)
    (let [decl (vector-get program idx)]
      (if (= (vector-get decl 0) 20)
        (if (= seen target)
          decl
          (nth-defn program (+ idx 1) (+ seen 1) target))
        (nth-defn program (+ idx 1) seen target)))))
(defn compile-standalone-rooted [expr]
  (do
    (root_push expr)
    (let [base (vector-new 8)]
      (do
        (root_push base)
        (let [result (compile-expr-with-ftable expr (env-new) (ftable-new) base)]
          (do
            (root_pop)
            (root_pop)
            result))))))
(defn compile-inline-file-state [path func-idx]
  (let [src (read-file path)
        program (parse-program src)
        source-root (resolve-source-root path)
        package-root (resolve-package-root path)
        seen-ref (ref-new (map-new))
        imported-pairs (load-imports-from-decls program src 0 (vector-length program) seen-ref (vector-new 8) source-root package-root)
        all-pairs (vector-push imported-pairs (make-src-decl-pair src program))
        n (vector-length all-pairs)
        reg-result (register-all-pairs all-pairs 0 n (ftable-new) func-idx)
        ftable (vector-get reg-result 0)
        data-ref (ref-new (vector-new 8))
        functions (compile-all-src-decl-pairs all-pairs 0 n ftable data-ref (vector-new 8))
        data (ref-get data-ref)]
    (vector-push (vector-push (vector-new 2) functions) data)))
(defn main []
  (let [payload1 (compile-inline-file-state "src/Main.ls" 7)
        payload2 (compile-inline-file-state "src/Main.ls" 7)
        functions1 (vector-get payload1 0)
        data1 (vector-get payload1 1)
        functions2 (vector-get payload2 0)
        data2 (vector-get payload2 1)
        wasm1 (build-wasm-bytes-wasi functions1 data1)
        wasm2 (build-wasm-bytes-wasi functions2 data2)]
    (do
      (print (vector-length wasm1))
      (print (vector-length wasm2))
      (print (wasm-bytes-eq wasm1 wasm2))
      0)))
"#,
    );

    let combined = format!("{}\n{}", selfhost_cli_runtime_bundle(), harness);
    let output = compile_and_run_with_dir(&combined, &dir);
    let _ = std::fs::remove_dir_all(&dir);
    let lines: Vec<&str> = output.trim().lines().collect();

    assert!(
        lines.len() >= 3,
        "direct multi-file determinism 出力が不足: {:?}",
        lines
    );
    assert_eq!(
        lines[0], lines[1],
        "2回の direct compile で Wasm 長は一致するべき"
    );
    assert_eq!(
        lines[2], "1",
        "2回の direct compile は byte-identical であるべき"
    );
}

/// TEST-CLI-02-M1F0: selfhost App.ModuleResolver の direct compile は 2 回連続でも同じ Wasm を返すこと
#[test]
#[ignore]
fn test_e2e_selfhost_cli_direct_module_resolver_compile_is_deterministic() {
    let dir = selfhost_package_root();

    let harness = with_stack_safe_wasm_bytes_eq_helpers(
        r#"
(defn make-ir-fingerprint-state [done next-idx next-acc]
  (push-int-vector-local
    (push-int-vector-local
      (push-int-vector-local (vector-new 3) done)
      next-idx)
    next-acc))
(defn ir-fingerprint-step [ir idx count acc]
  (if (>= idx count)
    (make-ir-fingerprint-state 1 idx acc)
    (let [instr (vector-get ir idx)
          opcode (vector-get instr 0)
          operand (vector-get instr 1)]
      (make-ir-fingerprint-state 0 (+ idx 1) (+ (* (+ (* acc 31) opcode) 31) operand)))))
(defn continue-ir-fingerprint-step [ir count state]
  (if (= (vector-get state 0) 1)
    state
    (ir-fingerprint-step ir (vector-get state 1) count (vector-get state 2))))
(defn ir-fingerprint-step-8 [ir idx count acc]
  (let [step1 (ir-fingerprint-step ir idx count acc)
        step2 (continue-ir-fingerprint-step ir count step1)
        step3 (continue-ir-fingerprint-step ir count step2)
        step4 (continue-ir-fingerprint-step ir count step3)
        step5 (continue-ir-fingerprint-step ir count step4)
        step6 (continue-ir-fingerprint-step ir count step5)
        step7 (continue-ir-fingerprint-step ir count step6)
        step8 (continue-ir-fingerprint-step ir count step7)]
    step8))
(defn continue-ir-fingerprint-step-8 [ir count state]
  (if (= (vector-get state 0) 1)
    state
    (ir-fingerprint-step-8 ir (vector-get state 1) count (vector-get state 2))))
(defn ir-fingerprint-step-64 [ir idx count acc]
  (let [step1 (ir-fingerprint-step-8 ir idx count acc)
        step2 (continue-ir-fingerprint-step-8 ir count step1)
        step3 (continue-ir-fingerprint-step-8 ir count step2)
        step4 (continue-ir-fingerprint-step-8 ir count step3)
        step5 (continue-ir-fingerprint-step-8 ir count step4)
        step6 (continue-ir-fingerprint-step-8 ir count step5)
        step7 (continue-ir-fingerprint-step-8 ir count step6)
        step8 (continue-ir-fingerprint-step-8 ir count step7)]
    step8))
(defn ir-fingerprint-loop [ir idx count acc]
  (let [step (ir-fingerprint-step-64 ir idx count acc)]
    (if (= (vector-get step 0) 1)
      (vector-get step 2)
      (ir-fingerprint-loop ir (vector-get step 1) count (vector-get step 2)))))
(defn ir-fingerprint [ir]
  (ir-fingerprint-loop ir 0 (vector-length ir) 0))
(defn function-fingerprint [func]
  (let [param-count (vector-get func 0)
        local-count (vector-get func 1)
        ir (vector-get func 2)]
    (+ (* (+ (* param-count 31) local-count) 31) (ir-fingerprint ir))))
(defn first-function-mismatch [functions1 functions2 idx count]
  (if (>= idx count)
    -1
    (if (= (function-fingerprint (vector-get functions1 idx)) (function-fingerprint (vector-get functions2 idx)))
      (first-function-mismatch functions1 functions2 (+ idx 1) count)
      idx)))
(defn first-ir-mismatch [ir1 ir2 idx count]
  (if (>= idx count)
    -1
    (let [instr1 (vector-get ir1 idx)
          instr2 (vector-get ir2 idx)]
      (if (and (= (vector-get instr1 0) (vector-get instr2 0)) (= (vector-get instr1 1) (vector-get instr2 1)))
        (first-ir-mismatch ir1 ir2 (+ idx 1) count)
        idx))))
(defn print-ir-instr [ir idx]
  (if (< idx 0)
    (do
      (print -1)
      (print -1)
      (print -1)
      0)
    (if (>= idx (vector-length ir))
      (do
        (print -1)
        (print -1)
        (print -1)
        0)
      (let [instr (vector-get ir idx)]
        (do
          (print idx)
          (print (vector-get instr 0))
          (print (vector-get instr 1))
          0)))))
(defn nth-defn [program idx seen target]
  (if (>= idx (vector-length program))
    (vector-new 5)
    (let [decl (vector-get program idx)]
      (if (= (vector-get decl 0) 20)
        (if (= seen target)
          decl
          (nth-defn program (+ idx 1) (+ seen 1) target))
        (nth-defn program (+ idx 1) seen target)))))
(defn nth-defn-name-hash [program idx seen target]
  (if (>= idx (vector-length program))
    -1
    (let [decl (vector-get program idx)]
      (if (= (vector-get decl 0) 20)
        (if (= seen target)
          (vector-get decl 1)
          (nth-defn-name-hash program (+ idx 1) (+ seen 1) target))
        (nth-defn-name-hash program (+ idx 1) seen target)))))
(defn nth-defn [program idx seen target]
  (if (>= idx (vector-length program))
    (vector-new 5)
    (let [decl (vector-get program idx)]
      (if (= (vector-get decl 0) 20)
        (if (= seen target)
          decl
          (nth-defn program (+ idx 1) (+ seen 1) target))
        (nth-defn program (+ idx 1) seen target)))))
(defn compile-standalone-rooted [expr]
  (do
    (root_push expr)
    (let [base (vector-new 8)]
      (do
        (root_push base)
        (let [result (compile-expr-with-ftable expr (env-new) (ftable-new) base)]
          (do
            (root_pop)
            (root_pop)
            result))))))
(defn compile-inline-file-state [path func-idx]
  (let [src (read-file path)
        program (parse-program src)
        source-root (resolve-source-root path)
        package-root (resolve-package-root path)
        seen-ref (ref-new (map-new))
        imported-pairs (load-imports-from-decls program src 0 (vector-length program) seen-ref (vector-new 8) source-root package-root)
        all-pairs (append-src-decl-pair imported-pairs src program)
        n (vector-length all-pairs)
        reg-result (register-all-pairs all-pairs 0 n (ftable-new) func-idx)
        ftable (vector-get reg-result 0)
        data-ref (ref-new (vector-new 8))
        functions (compile-all-src-decl-pairs all-pairs 0 n ftable data-ref (vector-new 8))
        data (ref-get data-ref)]
    (push-object-vector (vector-push (vector-push (vector-new 2) functions) data) program)))
(defn main []
  (let [state1 (compile-inline-file-state "src/App/ModuleResolver.ls" 7)
        state2 (compile-inline-file-state "src/App/ModuleResolver.ls" 7)
        functions1 (vector-get state1 0)
        data1 (vector-get state1 1)
        program1 (vector-get state1 2)
        functions2 (vector-get state2 0)
        data2 (vector-get state2 1)
        program2 (vector-get state2 2)
        wasm1 (build-wasm-bytes-wasi functions1 data1)
        wasm2 (build-wasm-bytes-wasi functions2 data2)
        mismatch-idx (first-function-mismatch functions1 functions2 0 (vector-length functions1))
        mismatch-decl1 (if (< mismatch-idx 0) (vector-new 5) (nth-defn program1 0 0 mismatch-idx))
        mismatch-decl2 (if (< mismatch-idx 0) (vector-new 5) (nth-defn program2 0 0 mismatch-idx))
        mismatch-name-hash (if (< mismatch-idx 0) -1 (nth-defn-name-hash program1 0 0 mismatch-idx))
        mismatch-func1 (if (< mismatch-idx 0) (vector-new 3) (vector-get functions1 mismatch-idx))
        mismatch-func2 (if (< mismatch-idx 0) (vector-new 3) (vector-get functions2 mismatch-idx))
        mismatch-ir-vec1 (if (< mismatch-idx 0) (vector-new 2) (vector-get mismatch-func1 2))
        mismatch-ir-vec2 (if (< mismatch-idx 0) (vector-new 2) (vector-get mismatch-func2 2))
        mismatch-body1 (if (< mismatch-idx 0) (vector-new 4) (vector-get mismatch-decl1 (+ 3 (vector-get mismatch-decl1 2))))
        mismatch-body2 (if (< mismatch-idx 0) (vector-new 4) (vector-get mismatch-decl2 (+ 3 (vector-get mismatch-decl2 2))))
        outer-if1 (if (< mismatch-idx 0) (vector-new 4) (vector-get mismatch-body1 3))
        outer-if2 (if (< mismatch-idx 0) (vector-new 4) (vector-get mismatch-body2 3))
        has-path-call1 (if (< mismatch-idx 0) (vector-new 6) (vector-get (vector-get outer-if1 3) 1))
        has-path-call2 (if (< mismatch-idx 0) (vector-new 6) (vector-get (vector-get outer-if2 3) 1))
        find-last-call1 (if (< mismatch-idx 0) (vector-new 7) (vector-get (vector-get (vector-get outer-if1 3) 2) 2))
        find-last-call2 (if (< mismatch-idx 0) (vector-new 7) (vector-get (vector-get (vector-get outer-if2 3) 2) 2))
        mismatch-ir-idx (if (< mismatch-idx 0) -1 (first-ir-mismatch mismatch-ir-vec1 mismatch-ir-vec2 0 (vector-length mismatch-ir-vec1)))]
    (do
      (print (vector-length wasm1))
      (print (vector-length wasm2))
      (print (wasm-bytes-eq wasm1 wasm2))
      (print mismatch-idx)
      (print mismatch-name-hash)
      (print (if (< mismatch-idx 0) -1 (vector-get mismatch-func1 0)))
      (print (if (< mismatch-idx 0) -1 (vector-get mismatch-func2 0)))
      (print (if (< mismatch-idx 0) -1 (vector-get mismatch-func1 1)))
      (print (if (< mismatch-idx 0) -1 (vector-get mismatch-func2 1)))
      (print (if (< mismatch-idx 0) -1 (vector-get (vector-get has-path-call1 4) 0)))
      (print (if (< mismatch-idx 0) -1 (vector-get (vector-get has-path-call1 4) 1)))
      (print (if (< mismatch-idx 0) -1 (vector-get (vector-get has-path-call2 4) 0)))
      (print (if (< mismatch-idx 0) -1 (vector-get (vector-get has-path-call2 4) 1)))
      (print (if (< mismatch-idx 0) -1 (vector-get (vector-get find-last-call1 4) 0)))
      (print (if (< mismatch-idx 0) -1 (vector-get (vector-get find-last-call1 4) 1)))
      (print (if (< mismatch-idx 0) -1 (vector-get (vector-get find-last-call2 4) 0)))
      (print (if (< mismatch-idx 0) -1 (vector-get (vector-get find-last-call2 4) 1)))
      (print (function-fingerprint mismatch-func1))
      (print (function-fingerprint (vector-get functions1 10)))
      (print (function-fingerprint (vector-get functions1 11)))
      (print (function-fingerprint (vector-get functions1 12)))
      (print (function-fingerprint (vector-get functions1 13)))
      (print (function-fingerprint (vector-get functions1 14)))
      (print mismatch-ir-idx)
      (print (if (< mismatch-ir-idx 0) -1 (vector-get (vector-get mismatch-ir-vec1 mismatch-ir-idx) 0)))
      (print (if (< mismatch-ir-idx 0) -1 (vector-get (vector-get mismatch-ir-vec1 mismatch-ir-idx) 1)))
      (print (if (< mismatch-ir-idx 0) -1 (vector-get (vector-get mismatch-ir-vec2 mismatch-ir-idx) 0)))
      (print (if (< mismatch-ir-idx 0) -1 (vector-get (vector-get mismatch-ir-vec2 mismatch-ir-idx) 1)))
      (print-ir-instr mismatch-ir-vec1 (- mismatch-ir-idx 4))
      (print-ir-instr mismatch-ir-vec1 (- mismatch-ir-idx 3))
      (print-ir-instr mismatch-ir-vec1 (- mismatch-ir-idx 2))
      (print-ir-instr mismatch-ir-vec1 (- mismatch-ir-idx 1))
      (print-ir-instr mismatch-ir-vec1 mismatch-ir-idx)
      (print-ir-instr mismatch-ir-vec1 (+ mismatch-ir-idx 1))
      (print-ir-instr mismatch-ir-vec1 (+ mismatch-ir-idx 2))
      (print-ir-instr mismatch-ir-vec1 (+ mismatch-ir-idx 3))
      (print-ir-instr mismatch-ir-vec1 (+ mismatch-ir-idx 4))
      (print-ir-instr mismatch-ir-vec2 (- mismatch-ir-idx 4))
      (print-ir-instr mismatch-ir-vec2 (- mismatch-ir-idx 3))
      (print-ir-instr mismatch-ir-vec2 (- mismatch-ir-idx 2))
      (print-ir-instr mismatch-ir-vec2 (- mismatch-ir-idx 1))
      (print-ir-instr mismatch-ir-vec2 mismatch-ir-idx)
      (print-ir-instr mismatch-ir-vec2 (+ mismatch-ir-idx 1))
      (print-ir-instr mismatch-ir-vec2 (+ mismatch-ir-idx 2))
      (print-ir-instr mismatch-ir-vec2 (+ mismatch-ir-idx 3))
      (print-ir-instr mismatch-ir-vec2 (+ mismatch-ir-idx 4))
      0)))
"#,
    );

    let combined = format!("{}\n{}", selfhost_cli_runtime_bundle(), harness);
    let output = compile_and_run_with_dir(&combined, &dir);
    let lines: Vec<&str> = output.trim().lines().collect();

    assert!(
        lines.len() >= 10,
        "module resolver direct determinism 出力が不足: {:?}",
        lines
    );
    assert_eq!(
        lines[0], lines[1],
        "2回の module resolver compile で Wasm 長は一致するべき: {:?}",
        lines
    );
    assert_eq!(
        lines[2], "1",
        "2回の module resolver compile は byte-identical であるべき: {:?}",
        lines
    );
}

/// TEST-CLI-02-M1F0B: path-parent 最小 fixture の direct compile は 2 回連続でも同じ Wasm を返すこと
#[test]
#[ignore]
fn test_e2e_selfhost_cli_direct_path_parent_fixture_is_deterministic() {
    assert_selfhost_direct_fixture_is_deterministic(
        "path_parent_direct_determinism",
        "PathParentMini.ls",
        "(defn path-char [path idx] (string-char-at path idx))\n\
         (defn is-path-sep [path idx] (let [ch (path-char path idx)] (if (= ch 47) true (if (= ch 92) true false))))\n\
         (defn has-path-sep [path idx len] (if (>= idx len) false (if (is-path-sep path idx) true (has-path-sep path (+ idx 1) len))))\n\
         (defn find-last-path-sep [path idx len last] (if (>= idx len) last (find-last-path-sep path (+ idx 1) len (if (is-path-sep path idx) idx last))))\n\
         (defn path-parent [path] (let [len (string-length path)] (if (= len 0) \"\" (if (has-path-sep path 0 len) (let [last (find-last-path-sep path 0 len -1)] (if (< last 0) \"\" (if (= last 0) \"/\" (substring path 0 last)))) \".\"))))",
        "path-parent fixture",
    );
}

/// TEST-CLI-02-M1F0B1: path-parent 最小 fixture の direct compile は code section も 2 回連続で同じ bytes を返すこと
#[test]
#[ignore]
fn test_e2e_selfhost_cli_direct_path_parent_code_section_is_deterministic() {
    assert_selfhost_direct_fixture_code_section_is_deterministic(
        "path_parent_direct_code_section_determinism",
        "PathParentMini.ls",
        "(defn path-char [path idx] (string-char-at path idx))\n\
         (defn is-path-sep [path idx] (let [ch (path-char path idx)] (if (= ch 47) true (if (= ch 92) true false))))\n\
         (defn has-path-sep [path idx len] (if (>= idx len) false (if (is-path-sep path idx) true (has-path-sep path (+ idx 1) len))))\n\
         (defn find-last-path-sep [path idx len last] (if (>= idx len) last (find-last-path-sep path (+ idx 1) len (if (is-path-sep path idx) idx last))))\n\
         (defn path-parent [path] (let [len (string-length path)] (if (= len 0) \"\" (if (has-path-sep path 0 len) (let [last (find-last-path-sep path 0 len -1)] (if (< last 0) \"\" (if (= last 0) \"/\" (substring path 0 last)))) \".\"))))",
        "path-parent code section fixture",
    );
}

/// TEST-CLI-02-M1F0B1A: path-parent 最小 fixture の direct compile code section は allocation warmup 後も同じ bytes を返すこと
#[test]
#[ignore]
fn test_e2e_selfhost_cli_direct_path_parent_code_section_survives_allocation_history() {
    assert_selfhost_direct_fixture_code_section_survives_allocation_history(
        "path_parent_direct_code_section_allocation_history",
        "PathParentMini.ls",
        "(defn path-char [path idx] (string-char-at path idx))\n\
         (defn is-path-sep [path idx] (let [ch (path-char path idx)] (if (= ch 47) true (if (= ch 92) true false))))\n\
         (defn has-path-sep [path idx len] (if (>= idx len) false (if (is-path-sep path idx) true (has-path-sep path (+ idx 1) len))))\n\
         (defn find-last-path-sep [path idx len last] (if (>= idx len) last (find-last-path-sep path (+ idx 1) len (if (is-path-sep path idx) idx last))))\n\
         (defn path-parent [path] (let [len (string-length path)] (if (= len 0) \"\" (if (has-path-sep path 0 len) (let [last (find-last-path-sep path 0 len -1)] (if (< last 0) \"\" (if (= last 0) \"/\" (substring path 0 last)))) \".\"))))",
        "path-parent allocation-history code section fixture",
    );
}

/// TEST-CLI-02-M1F0B2: path-parent 最小 fixture の inline compile は 2 回連続でも同じ Wasm を返すこと
#[test]
#[ignore]
fn test_e2e_selfhost_cli_inline_path_parent_fixture_is_deterministic() {
    let source = "(defn path-char [path idx] (string-char-at path idx))\n\
                  (defn is-path-sep [path idx] (let [ch (path-char path idx)] (if (= ch 47) true (if (= ch 92) true false))))\n\
                  (defn has-path-sep [path idx len] (if (>= idx len) false (if (is-path-sep path idx) true (has-path-sep path (+ idx 1) len))))\n\
                  (defn find-last-path-sep [path idx len last] (if (>= idx len) last (find-last-path-sep path (+ idx 1) len (if (is-path-sep path idx) idx last))))\n\
                  (defn path-parent [path] (let [len (string-length path)] (if (= len 0) \"\" (if (has-path-sep path 0 len) (let [last (find-last-path-sep path 0 len -1)] (if (< last 0) \"\" (if (= last 0) \"/\" (substring path 0 last)))) \".\"))))";
    assert_selfhost_inline_fixture_with_func_idx_is_deterministic(
        "path_parent_inline_determinism",
        source,
        "path-parent inline fixture",
        7,
    );
}

/// TEST-CLI-02-M1F0C: ModuleResolver prefix (find-src-ancestor まで) の direct compile は 2 回連続でも同じ Wasm を返すこと
#[test]
#[ignore]
fn test_e2e_selfhost_cli_direct_module_resolver_prefix_is_deterministic() {
    let prefix_source = take_lsharp_toplevel_forms(selfhost_module("ModuleResolver.ls"), 13);
    assert_selfhost_direct_fixture_is_deterministic(
        "module_resolver_prefix_direct_determinism",
        "ModuleResolverPrefix.ls",
        &prefix_source,
        "module resolver prefix",
    );
}

/// TEST-CLI-02-M1F0D: ModuleResolver 19 form prefix の direct compile は 2 回連続でも同じ Wasm を返すこと
#[test]
#[ignore]
fn test_e2e_selfhost_cli_direct_module_resolver_prefix_19_forms_is_deterministic() {
    let prefix_source = take_lsharp_toplevel_forms(selfhost_module("ModuleResolver.ls"), 19);
    assert_selfhost_direct_fixture_is_deterministic(
        "module_resolver_prefix_19_direct_determinism",
        "ModuleResolverPrefix19.ls",
        &prefix_source,
        "module resolver 19-form prefix",
    );
}

/// TEST-CLI-02-M1F0E: ModuleResolver 26 form prefix の direct compile は 2 回連続でも同じ Wasm を返すこと
#[test]
#[ignore]
fn test_e2e_selfhost_cli_direct_module_resolver_prefix_26_forms_is_deterministic() {
    let prefix_source = take_lsharp_toplevel_forms(selfhost_module("ModuleResolver.ls"), 26);
    assert_selfhost_direct_fixture_is_deterministic(
        "module_resolver_prefix_26_direct_determinism",
        "ModuleResolverPrefix26.ls",
        &prefix_source,
        "module resolver 26-form prefix",
    );
}

/// TEST-CLI-02-M1F0F: ModuleResolver 33 form prefix の direct compile は 2 回連続でも同じ Wasm を返すこと
#[test]
#[ignore]
fn test_e2e_selfhost_cli_direct_module_resolver_prefix_33_forms_is_deterministic() {
    let prefix_source = take_lsharp_toplevel_forms(selfhost_module("ModuleResolver.ls"), 33);
    assert_selfhost_direct_fixture_is_deterministic(
        "module_resolver_prefix_33_direct_determinism",
        "ModuleResolverPrefix33.ls",
        &prefix_source,
        "module resolver 33-form prefix",
    );
}

/// TEST-CLI-02-M1F0G: ModuleResolver full source の single-file direct compile は 2 回連続でも同じ Wasm を返すこと
#[test]
#[ignore]
fn test_e2e_selfhost_cli_direct_module_resolver_full_fixture_is_deterministic() {
    assert_selfhost_direct_fixture_is_deterministic(
        "module_resolver_full_fixture_direct_determinism",
        "ModuleResolverFull.ls",
        selfhost_module("ModuleResolver.ls"),
        "module resolver full fixture",
    );
}

/// TEST-CLI-02-M1F0H: ModuleResolver full source の inline direct compile は 2 回連続でも同じ Wasm を返すこと
#[test]
#[ignore]
fn test_e2e_selfhost_cli_direct_module_resolver_full_inline_fixture_is_deterministic() {
    assert_selfhost_inline_fixture_is_deterministic(
        "module_resolver_full_inline_fixture_direct_determinism",
        selfhost_module("ModuleResolver.ls"),
        "module resolver full inline fixture",
    );
}

#[test]
#[ignore]
fn test_e2e_selfhost_cli_direct_module_resolver_full_inline_without_import_scan_is_deterministic() {
    let dir = cli_test_fixture_dir("module_resolver_full_inline_without_import_scan");
    write_cli_fixture_files(
        &dir,
        &[
            ("lsharp.toml", ""),
            (
                "src/App/ModuleResolver.ls",
                selfhost_module("ModuleResolver.ls"),
            ),
        ],
    );

    let harness = with_stack_safe_wasm_bytes_eq_helpers(
        r#"
(defn compile-inline-file-state-no-import-scan [path func-idx]
  (let [src (read-file path)
        program (parse-program src)
        pair (make-src-decl-pair src program)
        all-pairs (push-object-vector (vector-new 8) pair)
        n (vector-length all-pairs)
        reg-result (register-all-pairs all-pairs 0 n (ftable-new) func-idx)
        ftable (vector-get reg-result 0)
        data-ref (ref-new (vector-new 8))
        functions (compile-all-src-decl-pairs all-pairs 0 n ftable data-ref (vector-new 8))
        data (ref-get data-ref)]
    (push-object-vector (vector-push (vector-push (vector-new 2) functions) data) program)))
(defn main []
  (let [state1 (compile-inline-file-state-no-import-scan "src/App/ModuleResolver.ls" 7)
        state2 (compile-inline-file-state-no-import-scan "src/App/ModuleResolver.ls" 7)
        functions1 (vector-get state1 0)
        data1 (vector-get state1 1)
        functions2 (vector-get state2 0)
        data2 (vector-get state2 1)
        wasm1 (build-wasm-bytes-wasi functions1 data1)
        wasm2 (build-wasm-bytes-wasi functions2 data2)]
    (do
      (print (vector-length wasm1))
      (print (vector-length wasm2))
      (print (wasm-bytes-eq wasm1 wasm2))
      0)))
"#,
    );

    let combined = format!("{}\n{}", selfhost_cli_runtime_bundle(), harness);
    let output = compile_and_run_with_dir(&combined, &dir);
    let _ = std::fs::remove_dir_all(&dir);
    let lines: Vec<&str> = output.trim().lines().collect();

    assert!(
        lines.len() >= 3,
        "module resolver full inline no-import-scan determinism 出力が不足: {:?}",
        lines
    );
    assert_eq!(lines[0], lines[1]);
    assert_eq!(
        lines[2], "1",
        "no-import-scan inline path は deterministic であるべき"
    );
}

#[test]
#[ignore]
fn test_e2e_selfhost_cli_direct_module_resolver_pair_registration_direct_compile_is_deterministic()
{
    let dir = cli_test_fixture_dir("module_resolver_pair_registration_direct_compile");
    write_cli_fixture_files(
        &dir,
        &[
            ("lsharp.toml", ""),
            (
                "src/App/ModuleResolver.ls",
                selfhost_module("ModuleResolver.ls"),
            ),
        ],
    );

    let harness = with_stack_safe_wasm_bytes_eq_helpers(
        r#"
(defn compile-pair-registration-direct-state [path func-idx]
  (let [src (read-file path)
        program (parse-program src)
        pair (make-src-decl-pair src program)
        all-pairs (push-object-vector (vector-new 8) pair)
        reg-result (register-all-pairs all-pairs 0 (vector-length all-pairs) (ftable-new) func-idx)
        ftable (vector-get reg-result 0)
        data-ref (ref-new (vector-new 8))
        functions (compile-defn-functions-with-source program 0 (vector-length program) src ftable data-ref (vector-new 8))
        data (ref-get data-ref)]
    (push-object-vector (vector-push (vector-push (vector-new 2) functions) data) program)))
(defn main []
  (let [state1 (compile-pair-registration-direct-state "src/App/ModuleResolver.ls" 7)
        state2 (compile-pair-registration-direct-state "src/App/ModuleResolver.ls" 7)
        functions1 (vector-get state1 0)
        data1 (vector-get state1 1)
        functions2 (vector-get state2 0)
        data2 (vector-get state2 1)
        wasm1 (build-wasm-bytes-wasi functions1 data1)
        wasm2 (build-wasm-bytes-wasi functions2 data2)]
    (do
      (print (vector-length wasm1))
      (print (vector-length wasm2))
      (print (wasm-bytes-eq wasm1 wasm2))
      0)))
"#,
    );

    let combined = format!("{}\n{}", selfhost_cli_runtime_bundle(), harness);
    let output = compile_and_run_with_dir(&combined, &dir);
    let _ = std::fs::remove_dir_all(&dir);
    let lines: Vec<&str> = output.trim().lines().collect();

    assert!(
        lines.len() >= 3,
        "module resolver pair-registration direct-compile determinism 出力が不足: {:?}",
        lines
    );
    assert_eq!(lines[0], lines[1]);
    assert_eq!(
        lines[2], "1",
        "pair registration + direct compile path は deterministic であるべき"
    );
}

#[test]
#[ignore]
fn test_e2e_selfhost_cli_direct_module_resolver_pair_creation_direct_compile_is_deterministic() {
    let dir = cli_test_fixture_dir("module_resolver_pair_creation_direct_compile");
    write_cli_fixture_files(
        &dir,
        &[
            ("lsharp.toml", ""),
            (
                "src/App/ModuleResolver.ls",
                selfhost_module("ModuleResolver.ls"),
            ),
        ],
    );
    let wasm_bytes_eq_helpers = stack_safe_wasm_bytes_eq_helpers();

    let harness = format!(
        r#"
{wasm_bytes_eq_helpers}
(defn compile-pair-creation-direct-state [path func-idx]
  (let [src (read-file path)
        program (parse-program src)
        pair (make-src-decl-pair src program)
        all-pairs (push-object-vector (vector-new 8) pair)
        reg-result (register-defns-chunked program 0 (vector-length program) (ftable-new) func-idx)
        ftable (vector-get reg-result 0)
        data-ref (ref-new (vector-new 8))
        functions (compile-defn-functions-with-source program 0 (vector-length program) src ftable data-ref (vector-new 8))
        data (ref-get data-ref)]
    (push-object-vector (vector-push (vector-push (vector-new 2) functions) data) program)))
(defn main []
  (let [state1 (compile-pair-creation-direct-state "src/App/ModuleResolver.ls" 7)
        state2 (compile-pair-creation-direct-state "src/App/ModuleResolver.ls" 7)
        functions1 (vector-get state1 0)
        data1 (vector-get state1 1)
        functions2 (vector-get state2 0)
        data2 (vector-get state2 1)
        wasm1 (build-wasm-bytes-wasi functions1 data1)
        wasm2 (build-wasm-bytes-wasi functions2 data2)]
    (do
      (print (vector-length wasm1))
      (print (vector-length wasm2))
      (print (wasm-bytes-eq wasm1 wasm2))
      0)))
"#
    );

    let combined = format!("{}\n{}", selfhost_cli_runtime_bundle(), harness);
    let output = compile_and_run_with_dir(&combined, &dir);
    let _ = std::fs::remove_dir_all(&dir);
    let lines: Vec<&str> = output.trim().lines().collect();

    assert!(
        lines.len() >= 3,
        "module resolver pair-creation direct-compile determinism 出力が不足: {:?}",
        lines
    );
    assert_eq!(lines[0], lines[1]);
    assert_eq!(
        lines[2], "1",
        "pair creation + direct compile path は deterministic であるべき"
    );
}

#[test]
#[ignore = "temporary diagnostic harness for local mismatch inspection"]
fn test_e2e_selfhost_cli_direct_module_resolver_full_inline_mismatch_probe() {
    let dir = cli_test_fixture_dir("module_resolver_full_inline_mismatch_probe");
    write_cli_fixture_files(
        &dir,
        &[
            ("lsharp.toml", ""),
            (
                "src/App/ModuleResolver.ls",
                selfhost_module("ModuleResolver.ls"),
            ),
        ],
    );

    let harness = with_stack_safe_wasm_bytes_eq_helpers(
        r#"
(defn make-ir-fingerprint-state [done next-idx next-acc]
  (push-int-vector-local
    (push-int-vector-local
      (push-int-vector-local (vector-new 3) done)
      next-idx)
    next-acc))
(defn ir-fingerprint-step [ir idx count acc]
  (if (>= idx count)
    (make-ir-fingerprint-state 1 idx acc)
    (let [instr (vector-get ir idx)
          opcode (vector-get instr 0)
          operand (vector-get instr 1)]
      (make-ir-fingerprint-state 0 (+ idx 1) (+ (* (+ (* acc 31) opcode) 31) operand)))))
(defn continue-ir-fingerprint-step [ir count state]
  (if (= (vector-get state 0) 1)
    state
    (ir-fingerprint-step ir (vector-get state 1) count (vector-get state 2))))
(defn ir-fingerprint-step-8 [ir idx count acc]
  (let [step1 (ir-fingerprint-step ir idx count acc)
        step2 (continue-ir-fingerprint-step ir count step1)
        step3 (continue-ir-fingerprint-step ir count step2)
        step4 (continue-ir-fingerprint-step ir count step3)
        step5 (continue-ir-fingerprint-step ir count step4)
        step6 (continue-ir-fingerprint-step ir count step5)
        step7 (continue-ir-fingerprint-step ir count step6)
        step8 (continue-ir-fingerprint-step ir count step7)]
    step8))
(defn continue-ir-fingerprint-step-8 [ir count state]
  (if (= (vector-get state 0) 1)
    state
    (ir-fingerprint-step-8 ir (vector-get state 1) count (vector-get state 2))))
(defn ir-fingerprint-step-64 [ir idx count acc]
  (let [step1 (ir-fingerprint-step-8 ir idx count acc)
        step2 (continue-ir-fingerprint-step-8 ir count step1)
        step3 (continue-ir-fingerprint-step-8 ir count step2)
        step4 (continue-ir-fingerprint-step-8 ir count step3)
        step5 (continue-ir-fingerprint-step-8 ir count step4)
        step6 (continue-ir-fingerprint-step-8 ir count step5)
        step7 (continue-ir-fingerprint-step-8 ir count step6)
        step8 (continue-ir-fingerprint-step-8 ir count step7)]
    step8))
(defn ir-fingerprint-loop [ir idx count acc]
  (let [step (ir-fingerprint-step-64 ir idx count acc)]
    (if (= (vector-get step 0) 1)
      (vector-get step 2)
      (ir-fingerprint-loop ir (vector-get step 1) count (vector-get step 2)))))
(defn ir-fingerprint [ir]
  (ir-fingerprint-loop ir 0 (vector-length ir) 0))
(defn function-fingerprint [func]
  (let [param-count (vector-get func 0)
        local-count (vector-get func 1)
        ir (vector-get func 2)]
    (+ (* (+ (* param-count 31) local-count) 31) (ir-fingerprint ir))))
(defn first-function-mismatch [functions1 functions2 idx count]
  (if (>= idx count)
    -1
    (if (= (function-fingerprint (vector-get functions1 idx)) (function-fingerprint (vector-get functions2 idx)))
      (first-function-mismatch functions1 functions2 (+ idx 1) count)
      idx)))
(defn first-ir-mismatch [ir1 ir2 idx count]
  (if (>= idx count)
    -1
    (let [instr1 (vector-get ir1 idx)
          instr2 (vector-get ir2 idx)]
      (if (and (= (vector-get instr1 0) (vector-get instr2 0)) (= (vector-get instr1 1) (vector-get instr2 1)))
        (first-ir-mismatch ir1 ir2 (+ idx 1) count)
        idx))))
(defn nth-defn-name-hash [program idx seen target]
  (if (>= idx (vector-length program))
    -1
    (let [decl (vector-get program idx)]
      (if (= (vector-get decl 0) 20)
        (if (= seen target)
          (vector-get decl 1)
          (nth-defn-name-hash program (+ idx 1) (+ seen 1) target))
        (nth-defn-name-hash program (+ idx 1) seen target)))))
(defn compile-inline-file-state [path func-idx]
  (let [src (read-file path)
        program (parse-program src)
        source-root (resolve-source-root path)
        package-root (resolve-package-root path)
        seen-ref (ref-new (map-new))
        imported-pairs (load-imports-from-decls program src 0 (vector-length program) seen-ref (vector-new 8) source-root package-root)
        all-pairs (append-src-decl-pair imported-pairs src program)
        n (vector-length all-pairs)
        reg-result (register-all-pairs all-pairs 0 n (ftable-new) 7)
        ftable (vector-get reg-result 0)
        data-ref (ref-new (vector-new 8))
        functions (compile-all-src-decl-pairs all-pairs 0 n ftable data-ref (vector-new 8))
        data (ref-get data-ref)
        state1 (push-object-vector (vector-new 3) functions)
        state2 (push-object-vector state1 data)]
    (push-object-vector state2 program)))
(defn main []
  (let [state1 (compile-inline-file-state "src/App/ModuleResolver.ls" 7)
        state2 (compile-inline-file-state "src/App/ModuleResolver.ls" 7)
        functions1 (vector-get state1 0)
        data1 (vector-get state1 1)
        program1 (vector-get state1 2)
        functions2 (vector-get state2 0)
        data2 (vector-get state2 1)
        wasm1 (build-wasm-bytes-wasi functions1 data1)
        wasm2 (build-wasm-bytes-wasi functions2 data2)
        mismatch-idx (first-function-mismatch functions1 functions2 0 (vector-length functions1))
        mismatch-name-hash (if (< mismatch-idx 0) -1 (nth-defn-name-hash program1 0 0 mismatch-idx))
        mismatch-func1 (if (< mismatch-idx 0) (vector-new 3) (vector-get functions1 mismatch-idx))
        mismatch-func2 (if (< mismatch-idx 0) (vector-new 3) (vector-get functions2 mismatch-idx))
        mismatch-ir1 (if (< mismatch-idx 0) (vector-new 2) (vector-get mismatch-func1 2))
        mismatch-ir2 (if (< mismatch-idx 0) (vector-new 2) (vector-get mismatch-func2 2))
        mismatch-ir-idx (if (< mismatch-idx 0) -1 (first-ir-mismatch mismatch-ir1 mismatch-ir2 0 (vector-length mismatch-ir1)))]
    (do
      (print (vector-length wasm1))
      (print (vector-length wasm2))
      (print (wasm-bytes-eq wasm1 wasm2))
      (print mismatch-idx)
      (print mismatch-name-hash)
      (print (if (< mismatch-idx 0) -1 (vector-get mismatch-func1 0)))
      (print (if (< mismatch-idx 0) -1 (vector-get mismatch-func1 1)))
      (print mismatch-ir-idx)
      (print (if (< mismatch-ir-idx 0) -1 (vector-get (vector-get mismatch-ir1 mismatch-ir-idx) 0)))
      (print (if (< mismatch-ir-idx 0) -1 (vector-get (vector-get mismatch-ir1 mismatch-ir-idx) 1)))
      (print (if (< mismatch-ir-idx 0) -1 (vector-get (vector-get mismatch-ir2 mismatch-ir-idx) 0)))
      (print (if (< mismatch-ir-idx 0) -1 (vector-get (vector-get mismatch-ir2 mismatch-ir-idx) 1)))
      0)))
"#,
    );

    let combined = format!("{}\n{}", selfhost_cli_runtime_bundle(), harness);
    let output = compile_and_run_with_dir(&combined, &dir);
    let _ = std::fs::remove_dir_all(&dir);
    let lines: Vec<&str> = output.trim().lines().collect();

    assert!(
        lines.len() >= 12,
        "module resolver full inline mismatch probe 出力が不足: {:?}",
        lines
    );
    panic!("mismatch probe: {:?}", lines);
}

/// TEST-CLI-02-M1F0I: ModuleResolver 13 form prefix の inline direct compile は 2 回連続でも同じ Wasm を返すこと
#[test]
#[ignore]
fn test_e2e_selfhost_cli_direct_module_resolver_inline_prefix_13_forms_is_deterministic() {
    let prefix_source = take_lsharp_toplevel_forms(selfhost_module("ModuleResolver.ls"), 13);
    assert_selfhost_inline_fixture_is_deterministic(
        "module_resolver_inline_prefix_13_direct_determinism",
        &prefix_source,
        "module resolver 13-form inline prefix",
    );
}

/// TEST-CLI-02-M1F0J: ModuleResolver 26 form prefix の inline direct compile は 2 回連続でも同じ Wasm を返すこと
#[test]
#[ignore]
fn test_e2e_selfhost_cli_direct_module_resolver_inline_prefix_26_forms_is_deterministic() {
    let prefix_source = take_lsharp_toplevel_forms(selfhost_module("ModuleResolver.ls"), 26);
    assert_selfhost_inline_fixture_is_deterministic(
        "module_resolver_inline_prefix_26_direct_determinism",
        &prefix_source,
        "module resolver 26-form inline prefix",
    );
}

/// TEST-CLI-02-M1F0K: ModuleResolver 33 form prefix の inline direct compile は 2 回連続でも同じ Wasm を返すこと
#[test]
#[ignore]
fn test_e2e_selfhost_cli_direct_module_resolver_inline_prefix_33_forms_is_deterministic() {
    let prefix_source = take_lsharp_toplevel_forms(selfhost_module("ModuleResolver.ls"), 33);
    assert_selfhost_inline_fixture_is_deterministic(
        "module_resolver_inline_prefix_33_direct_determinism",
        &prefix_source,
        "module resolver 33-form inline prefix",
    );
}

#[test]
#[ignore]
fn test_e2e_selfhost_cli_direct_module_resolver_inline_prefix_post33_bisect() {
    for form_count in [34usize, 35, 36, 37, 38, 39] {
        println!("bisect form_count={form_count}");
        let prefix_source =
            take_lsharp_toplevel_forms(selfhost_module("ModuleResolver.ls"), form_count);
        let fixture_prefix = format!("module_resolver_inline_prefix_{form_count}_bisect");
        let label = format!("module resolver {form_count}-form inline prefix");
        assert_selfhost_inline_fixture_is_deterministic(&fixture_prefix, &prefix_source, &label);
    }
}

/// TEST-CLI-02-M1F0L: ModuleResolver full source の single-file direct compile は func_idx=7 でも deterministic であること
#[test]
#[ignore]
fn test_e2e_selfhost_cli_direct_module_resolver_full_fixture_func_idx_7_is_deterministic() {
    assert_selfhost_direct_fixture_with_func_idx_is_deterministic(
        "module_resolver_full_fixture_func_idx_7_direct_determinism",
        "ModuleResolverFullFuncIdx7.ls",
        selfhost_module("ModuleResolver.ls"),
        "module resolver full fixture func_idx=7",
        7,
    );
}

/// TEST-CLI-02-M1F0M: ModuleResolver full source の inline direct compile は func_idx=0 でも deterministic であること
#[test]
#[ignore]
fn test_e2e_selfhost_cli_direct_module_resolver_full_inline_fixture_func_idx_0_is_deterministic() {
    assert_selfhost_inline_fixture_with_func_idx_is_deterministic(
        "module_resolver_full_inline_fixture_func_idx_0_direct_determinism",
        selfhost_module("ModuleResolver.ls"),
        "module resolver full inline fixture func_idx=0",
        0,
    );
}

/// TEST-CLI-02-M1F0N: ModuleResolver full source の compile-file-functions-data は 2 回連続でも同じ Wasm を返すこと
#[test]
#[ignore]
fn test_e2e_selfhost_cli_compile_file_functions_data_module_resolver_is_deterministic() {
    let dir = cli_test_fixture_dir("module_resolver_compile_file_functions_data_determinism");
    write_cli_fixture_files(
        &dir,
        &[
            ("lsharp.toml", ""),
            (
                "src/App/ModuleResolver.ls",
                selfhost_module("ModuleResolver.ls"),
            ),
        ],
    );

    let harness = with_stack_safe_wasm_bytes_eq_helpers(
        r#"
(defn main []
  (let [payload1 (compile-file-functions-data "src/App/ModuleResolver.ls")
        payload2 (compile-file-functions-data "src/App/ModuleResolver.ls")
        functions1 (vector-get payload1 0)
        data1 (vector-get payload1 1)
        functions2 (vector-get payload2 0)
        data2 (vector-get payload2 1)
        wasm1 (build-wasm-bytes-wasi functions1 data1)
        wasm2 (build-wasm-bytes-wasi functions2 data2)]
    (do
      (print (vector-length wasm1))
      (print (vector-length wasm2))
      (print (wasm-bytes-eq wasm1 wasm2))
      0)))
"#,
    );

    let combined = format!("{}\n{}", selfhost_cli_runtime_bundle(), harness);
    let output = compile_and_run_with_dir(&combined, &dir);
    let _ = std::fs::remove_dir_all(&dir);
    let lines: Vec<&str> = output.trim().lines().collect();

    assert!(
        lines.len() >= 3,
        "compile-file-functions-data ModuleResolver determinism 出力が不足: {:?}",
        lines
    );
    assert_eq!(
        lines[0], lines[1],
        "2回の compile-file-functions-data ModuleResolver compile で Wasm 長は一致するべき"
    );
    assert_eq!(
        lines[2], "1",
        "2回の compile-file-functions-data ModuleResolver compile は byte-identical であるべき: {:?}",
        lines
    );
}

/// TEST-CLI-02-M1F0O: ModuleResolver full source の register-defns direct compile は 2 回連続でも同じ Wasm を返すこと
#[test]
#[ignore]
fn test_e2e_selfhost_cli_direct_module_resolver_register_defns_is_deterministic() {
    let dir = cli_test_fixture_dir("module_resolver_register_defns_direct_determinism");
    write_cli_fixture_files(
        &dir,
        &[(
            "ModuleResolverRegisterDefns.ls",
            selfhost_module("ModuleResolver.ls"),
        )],
    );
    let fixture_path = dir
        .join("ModuleResolverRegisterDefns.ls")
        .to_string_lossy()
        .replace('\\', "\\\\");

    let harness = with_stack_safe_wasm_bytes_eq_helpers(&format!(
        r#"
(defn compile-file-state [path]
  (let [src (read-file path)
        program (parse-program src)
        n (vector-length program)
        reg-result (register-defns program 0 n (ftable-new) 0)
        ftable (vector-get reg-result 0)
        data-ref (ref-new (vector-new 8))
        functions (compile-defn-functions-with-source program 0 n src ftable data-ref (vector-new 8))
        data (ref-get data-ref)]
    (push-object-vector (vector-push (vector-push (vector-new 2) functions) data) program)))
(defn main []
  (let [state1 (compile-file-state "{fixture_path}")
        state2 (compile-file-state "{fixture_path}")
        functions1 (vector-get state1 0)
        data1 (vector-get state1 1)
        functions2 (vector-get state2 0)
        data2 (vector-get state2 1)
        wasm1 (build-wasm-bytes-wasi functions1 data1)
        wasm2 (build-wasm-bytes-wasi functions2 data2)]
    (do
      (print (vector-length wasm1))
      (print (vector-length wasm2))
      (print (wasm-bytes-eq wasm1 wasm2))
      0)))
"#
    ));

    let combined = format!("{}\n{}", selfhost_cli_runtime_bundle(), harness);
    let output = compile_and_run(&combined);
    let _ = std::fs::remove_dir_all(&dir);
    let lines: Vec<&str> = output.trim().lines().collect();

    assert!(
        lines.len() >= 3,
        "register-defns ModuleResolver determinism 出力が不足: {:?}",
        lines
    );
    assert_eq!(
        lines[0], lines[1],
        "2回の register-defns ModuleResolver compile で Wasm 長は一致するべき"
    );
    assert_eq!(
        lines[2], "1",
        "2回の register-defns ModuleResolver compile は byte-identical であるべき: {:?}",
        lines
    );
}

/// TEST-CLI-02-M1F0P: ModuleResolver full source の single-pair pipeline compile は 2 回連続でも同じ Wasm を返すこと
#[test]
#[ignore]
fn test_e2e_selfhost_cli_direct_module_resolver_single_pair_pipeline_is_deterministic() {
    let dir = cli_test_fixture_dir("module_resolver_single_pair_pipeline_determinism");
    write_cli_fixture_files(
        &dir,
        &[(
            "ModuleResolverSinglePair.ls",
            selfhost_module("ModuleResolver.ls"),
        )],
    );
    let fixture_path = dir
        .join("ModuleResolverSinglePair.ls")
        .to_string_lossy()
        .replace('\\', "\\\\");

    let harness = with_stack_safe_wasm_bytes_eq_helpers(&format!(
        r#"
(defn compile-file-state [path]
  (let [src (read-file path)
        program (parse-program src)
        pair (make-src-decl-pair src program)]
    (do
      (root_push pair)
      (let [all-pairs (push-object-vector (vector-new 8) pair)]
        (do
          (root_push all-pairs)
          (let [n (vector-length all-pairs)
                reg-result (register-all-pairs all-pairs 0 n (ftable-new) 0)]
            (do
              (root_push reg-result)
              (let [ftable (vector-get reg-result 0)
                    data-ref (ref-new (vector-new 8))
                    functions (compile-all-src-decl-pairs all-pairs 0 n ftable data-ref (vector-new 8))
                    data (ref-get data-ref)]
                (do
                  (root_push functions)
                  (root_push data)
                  (let [payload1 (vector-push (vector-new 2) functions)]
                    (do
                      (root_push payload1)
                      (let [payload2 (vector-push payload1 data)]
                        (do
                          (root_pop)
                          (root_pop)
                          (root_pop)
                          (root_pop)
                          (root_pop)
                          payload2)))))))))))))
(defn main []
  (let [state1 (compile-file-state "{fixture_path}")
        state2 (compile-file-state "{fixture_path}")
        functions1 (vector-get state1 0)
        data1 (vector-get state1 1)
        functions2 (vector-get state2 0)
        data2 (vector-get state2 1)
        wasm1 (build-wasm-bytes-wasi functions1 data1)
        wasm2 (build-wasm-bytes-wasi functions2 data2)]
    (do
      (print (vector-length wasm1))
      (print (vector-length wasm2))
      (print (wasm-bytes-eq wasm1 wasm2))
      0)))
"#
    ));

    let combined = format!("{}\n{}", selfhost_cli_runtime_bundle(), harness);
    let output = compile_and_run(&combined);
    let _ = std::fs::remove_dir_all(&dir);
    let lines: Vec<&str> = output.trim().lines().collect();

    assert!(
        lines.len() >= 3,
        "single-pair pipeline ModuleResolver determinism 出力が不足: {:?}",
        lines
    );
    assert_eq!(
        lines[0], lines[1],
        "2回の single-pair pipeline ModuleResolver compile で Wasm 長は一致するべき"
    );
    assert_eq!(
        lines[2], "1",
        "2回の single-pair pipeline ModuleResolver compile は byte-identical であるべき: {:?}",
        lines
    );
}

/// TEST-CLI-02-M1F0Q: ModuleResolver full source の compile-file-pairs-with-cache pipeline は 2 回連続でも同じ Wasm を返すこと
#[test]
#[ignore]
fn test_e2e_selfhost_cli_compile_file_pairs_with_cache_pipeline_is_deterministic() {
    let dir = cli_test_fixture_dir("module_resolver_pairs_cache_pipeline_determinism");
    write_cli_fixture_files(
        &dir,
        &[
            ("lsharp.toml", ""),
            (
                "src/App/ModuleResolver.ls",
                selfhost_module("ModuleResolver.ls"),
            ),
        ],
    );

    let harness = with_stack_safe_wasm_bytes_eq_helpers(
        r#"
(defn compile-file-state [path]
  (let [cache-ref (ref-new (map-new))
        parse-count-ref (ref-new 0)
        all-pairs (compile-file-pairs-with-cache path cache-ref parse-count-ref)]
    (do
      (root_push all-pairs)
      (let [n (vector-length all-pairs)
            reg-result (register-all-pairs all-pairs 0 n (ftable-new) 0)]
        (do
          (root_push reg-result)
          (let [ftable (vector-get reg-result 0)
                data-ref (ref-new (vector-new 8))
                functions (compile-all-src-decl-pairs all-pairs 0 n ftable data-ref (vector-new 8))
                data (ref-get data-ref)]
            (do
              (root_push functions)
              (root_push data)
              (let [payload1 (vector-push (vector-new 2) functions)]
                (do
                  (root_push payload1)
                  (let [payload2 (vector-push payload1 data)]
                    (do
                      (root_pop)
                      (root_pop)
                      (root_pop)
                      (root_pop)
                      payload2)))))))))))
(defn main []
  (let [state1 (compile-file-state "src/App/ModuleResolver.ls")
        state2 (compile-file-state "src/App/ModuleResolver.ls")
        functions1 (vector-get state1 0)
        data1 (vector-get state1 1)
        functions2 (vector-get state2 0)
        data2 (vector-get state2 1)
        wasm1 (build-wasm-bytes-wasi functions1 data1)
        wasm2 (build-wasm-bytes-wasi functions2 data2)]
    (do
      (print (vector-length wasm1))
      (print (vector-length wasm2))
      (print (wasm-bytes-eq wasm1 wasm2))
      0)))
"#,
    );

    let combined = format!("{}\n{}", selfhost_cli_runtime_bundle(), harness);
    let output = compile_and_run_with_dir(&combined, &dir);
    let _ = std::fs::remove_dir_all(&dir);
    let lines: Vec<&str> = output.trim().lines().collect();

    assert!(
        lines.len() >= 3,
        "compile-file-pairs-with-cache pipeline ModuleResolver determinism 出力が不足: {:?}",
        lines
    );
    assert_eq!(
        lines[0], lines[1],
        "2回の compile-file-pairs-with-cache pipeline ModuleResolver compile で Wasm 長は一致するべき"
    );
    assert_eq!(
        lines[2], "1",
        "2回の compile-file-pairs-with-cache pipeline ModuleResolver compile は byte-identical であるべき: {:?}",
        lines
    );
}

/// TEST-CLI-02-M1F0R: ModuleResolver full source の load-src-decl-pair-with-cache pipeline は 2 回連続でも同じ Wasm を返すこと
#[test]
#[ignore]
fn test_e2e_selfhost_cli_load_src_decl_pair_with_cache_pipeline_is_deterministic() {
    let dir = cli_test_fixture_dir("module_resolver_src_decl_pair_cache_pipeline_determinism");
    write_cli_fixture_files(
        &dir,
        &[
            ("lsharp.toml", ""),
            (
                "src/App/ModuleResolver.ls",
                selfhost_module("ModuleResolver.ls"),
            ),
        ],
    );

    let harness = with_stack_safe_wasm_bytes_eq_helpers(
        r#"
(defn compile-file-state [path]
  (let [cache-ref (ref-new (map-new))
        parse-count-ref (ref-new 0)
        pair (load-src-decl-pair-with-cache path cache-ref parse-count-ref)]
    (do
      (root_push pair)
      (let [all-pairs (push-object-vector (vector-new 8) pair)]
        (do
          (root_push all-pairs)
          (let [n (vector-length all-pairs)
                reg-result (register-all-pairs all-pairs 0 n (ftable-new) 0)]
            (do
              (root_push reg-result)
              (let [ftable (vector-get reg-result 0)
                    data-ref (ref-new (vector-new 8))
                    functions (compile-all-src-decl-pairs all-pairs 0 n ftable data-ref (vector-new 8))
                    data (ref-get data-ref)]
                (do
                  (root_push functions)
                  (root_push data)
                  (let [payload1 (vector-push (vector-new 2) functions)]
                    (do
                      (root_push payload1)
                      (let [payload2 (vector-push payload1 data)]
                        (do
                          (root_pop)
                          (root_pop)
                          (root_pop)
                          (root_pop)
                          (root_pop)
                          payload2)))))))))))))
(defn main []
  (let [state1 (compile-file-state "src/App/ModuleResolver.ls")
        state2 (compile-file-state "src/App/ModuleResolver.ls")
        functions1 (vector-get state1 0)
        data1 (vector-get state1 1)
        functions2 (vector-get state2 0)
        data2 (vector-get state2 1)
        wasm1 (build-wasm-bytes-wasi functions1 data1)
        wasm2 (build-wasm-bytes-wasi functions2 data2)]
    (do
      (print (vector-length wasm1))
      (print (vector-length wasm2))
      (print (wasm-bytes-eq wasm1 wasm2))
      0)))
"#,
    );

    let combined = format!("{}\n{}", selfhost_cli_runtime_bundle(), harness);
    let output = compile_and_run_with_dir(&combined, &dir);
    let _ = std::fs::remove_dir_all(&dir);
    let lines: Vec<&str> = output.trim().lines().collect();

    assert!(
        lines.len() >= 3,
        "load-src-decl-pair-with-cache pipeline ModuleResolver determinism 出力が不足: {:?}",
        lines
    );
    assert_eq!(
        lines[0], lines[1],
        "2回の load-src-decl-pair-with-cache pipeline ModuleResolver compile で Wasm 長は一致するべき"
    );
    assert_eq!(
        lines[2], "1",
        "2回の load-src-decl-pair-with-cache pipeline ModuleResolver compile は byte-identical であるべき: {:?}",
        lines
    );
}

/// TEST-CLI-02-M1F0R1: load-src-decl-pair-with-cache は ModuleResolver の src/decls を壊さず返すこと
#[test]
#[ignore]
fn test_e2e_selfhost_cli_load_src_decl_pair_with_cache_returns_expected_pair_shape() {
    let dir = cli_test_fixture_dir("module_resolver_src_decl_pair_cache_shape");
    write_cli_fixture_files(
        &dir,
        &[
            ("lsharp.toml", ""),
            (
                "src/App/ModuleResolver.ls",
                selfhost_module("ModuleResolver.ls"),
            ),
        ],
    );

    let harness = r#"
(defn main []
  (let [cache-ref (ref-new (map-new))
        parse-count-ref (ref-new 0)
        pair (load-src-decl-pair-with-cache "src/App/ModuleResolver.ls" cache-ref parse-count-ref)
        src (vector-get pair 0)
        decls (vector-get pair 1)]
    (do
      (print (ref-get parse-count-ref))
      (print (string-length src))
      (print (vector-length decls))
      0)))
"#;

    let combined = format!("{}\n{}", selfhost_cli_runtime_bundle(), harness);
    let output = compile_and_run_with_dir(&combined, &dir);
    let _ = std::fs::remove_dir_all(&dir);
    let lines: Vec<&str> = output.trim().lines().collect();
    let expected_decls = parse_for_pipeline(selfhost_module("ModuleResolver.ls"))
        .decls
        .len();

    assert!(
        lines.len() >= 3,
        "load-src-decl-pair-with-cache shape 出力が不足: {:?}",
        lines
    );
    assert_eq!(lines[0], "1", "初回 load では parse は 1 回だけ走るべき");
    assert_eq!(
        lines[1],
        selfhost_module("ModuleResolver.ls").len().to_string(),
        "返却された src length は canonical ModuleResolver と一致するべき"
    );
    assert_eq!(
        lines[2],
        expected_decls.to_string(),
        "返却された decl count は canonical ModuleResolver parse 結果と一致するべき"
    );
}

/// TEST-CLI-02-M1F0R2: load-src-decl-pair-with-cache の single-pair compile は manual pair path と一致すること
#[test]
#[ignore]
fn test_e2e_selfhost_cli_load_src_decl_pair_with_cache_matches_manual_pair_pipeline() {
    let dir = cli_test_fixture_dir("module_resolver_src_decl_pair_cache_matches_manual");
    write_cli_fixture_files(
        &dir,
        &[
            ("lsharp.toml", ""),
            (
                "src/App/ModuleResolver.ls",
                selfhost_module("ModuleResolver.ls"),
            ),
        ],
    );

    let harness = with_stack_safe_wasm_bytes_eq_helpers(
        r#"
(defn compile-pair-state [pair]
  (do
    (root_push pair)
    (let [all-pairs (push-object-vector (vector-new 8) pair)]
      (do
        (root_push all-pairs)
        (let [n (vector-length all-pairs)
              reg-result (register-all-pairs all-pairs 0 n (ftable-new) 0)]
          (do
            (root_push reg-result)
            (let [ftable (vector-get reg-result 0)
                  data-ref (ref-new (vector-new 8))
                  functions (compile-all-src-decl-pairs all-pairs 0 n ftable data-ref (vector-new 8))
                  data (ref-get data-ref)]
              (do
                (root_push functions)
                (root_push data)
                (let [payload1 (vector-push (vector-new 2) functions)]
                  (do
                    (root_push payload1)
                    (let [payload2 (vector-push payload1 data)]
                      (do
                        (root_pop)
                        (root_pop)
                        (root_pop)
                        (root_pop)
                        (root_pop)
                        payload2))))))))))))
(defn main []
  (let [cache-ref (ref-new (map-new))
        parse-count-ref (ref-new 0)
        loader-pair (load-src-decl-pair-with-cache "src/App/ModuleResolver.ls" cache-ref parse-count-ref)
        src (read-file "src/App/ModuleResolver.ls")
        program (parse-program src)
        manual-pair (make-src-decl-pair src program)
        loader-state (compile-pair-state loader-pair)
        manual-state (compile-pair-state manual-pair)
        loader-functions (vector-get loader-state 0)
        loader-data (vector-get loader-state 1)
        manual-functions (vector-get manual-state 0)
        manual-data (vector-get manual-state 1)
        loader-wasm (build-wasm-bytes-wasi loader-functions loader-data)
        manual-wasm (build-wasm-bytes-wasi manual-functions manual-data)]
    (do
      (print (vector-length loader-wasm))
      (print (vector-length manual-wasm))
      (print (wasm-bytes-eq loader-wasm manual-wasm))
      0)))
"#,
    );

    let combined = format!("{}\n{}", selfhost_cli_runtime_bundle(), harness);
    let output = compile_and_run_with_dir(&combined, &dir);
    let _ = std::fs::remove_dir_all(&dir);
    let lines: Vec<&str> = output.trim().lines().collect();

    assert!(
        lines.len() >= 3,
        "load-src-decl-pair-with-cache vs manual pair 出力が不足: {:?}",
        lines
    );
    assert_eq!(
        lines[0], lines[1],
        "loader pair と manual pair compile の Wasm 長は一致するべき"
    );
    assert_eq!(
        lines[2], "1",
        "loader pair compile は manual pair compile と byte-identical であるべき: {:?}",
        lines
    );
}

/// TEST-CLI-02-M1F0R3: cache miss side effects 後の manual pair compile は plain manual pair と一致すること
#[test]
#[ignore]
fn test_e2e_selfhost_cli_cache_miss_side_effects_match_manual_pair_pipeline() {
    let dir = cli_test_fixture_dir("module_resolver_cache_side_effects_match_manual");
    write_cli_fixture_files(
        &dir,
        &[
            ("lsharp.toml", ""),
            (
                "src/App/ModuleResolver.ls",
                selfhost_module("ModuleResolver.ls"),
            ),
        ],
    );

    let harness = with_stack_safe_wasm_bytes_eq_helpers(
        r#"
(defn compile-pair-state [pair]
  (do
    (root_push pair)
    (let [all-pairs (push-object-vector (vector-new 8) pair)]
      (do
        (root_push all-pairs)
        (let [n (vector-length all-pairs)
              reg-result (register-all-pairs all-pairs 0 n (ftable-new) 0)]
          (do
            (root_push reg-result)
            (let [ftable (vector-get reg-result 0)
                  data-ref (ref-new (vector-new 8))
                  functions (compile-all-src-decl-pairs all-pairs 0 n ftable data-ref (vector-new 8))
                  data (ref-get data-ref)]
              (do
                (root_push functions)
                (root_push data)
                (let [payload1 (vector-push (vector-new 2) functions)]
                  (do
                    (root_push payload1)
                    (let [payload2 (vector-push payload1 data)]
                      (do
                        (root_pop)
                        (root_pop)
                        (root_pop)
                        (root_pop)
                        (root_pop)
                        payload2))))))))))))
(defn compile-manual-state [path]
  (let [src (read-file path)
        program (parse-program src)
        pair (make-src-decl-pair src program)]
    (compile-pair-state pair)))
(defn compile-manual-after-cache-miss-side-effects [path]
  (let [cache-ref (ref-new (map-new))
        parse-count-ref (ref-new 0)
        src (read-file path)
        fingerprint (source-fingerprint src)
        cache-key (src-decl-cache-key path)
        pair (parse-src-decl-pair src)
        entry (make-src-decl-cache-entry fingerprint pair)]
    (do
      (root_push entry)
      (ref-set parse-count-ref (+ (ref-get parse-count-ref) 1))
      (ref-set cache-ref (ref-map-insert-object-safe cache-ref cache-key entry))
      (root_pop)
      (let [program (parse-program src)
            manual-pair (make-src-decl-pair src program)]
        (compile-pair-state manual-pair)))))
(defn main []
  (let [plain-state (compile-manual-state "src/App/ModuleResolver.ls")
        sidefx-state (compile-manual-after-cache-miss-side-effects "src/App/ModuleResolver.ls")
        plain-functions (vector-get plain-state 0)
        plain-data (vector-get plain-state 1)
        sidefx-functions (vector-get sidefx-state 0)
        sidefx-data (vector-get sidefx-state 1)
        plain-wasm (build-wasm-bytes-wasi plain-functions plain-data)
        sidefx-wasm (build-wasm-bytes-wasi sidefx-functions sidefx-data)]
    (do
      (print (vector-length plain-wasm))
      (print (vector-length sidefx-wasm))
      (print (wasm-bytes-eq plain-wasm sidefx-wasm))
      0)))
"#,
    );

    let combined = format!("{}\n{}", selfhost_cli_runtime_bundle(), harness);
    let output = compile_and_run_with_dir(&combined, &dir);
    let _ = std::fs::remove_dir_all(&dir);
    let lines: Vec<&str> = output.trim().lines().collect();

    assert!(
        lines.len() >= 3,
        "cache miss side effects vs manual pair 出力が不足: {:?}",
        lines
    );
    assert_eq!(
        lines[0], lines[1],
        "cache miss side effects 後の manual pair compile でも Wasm 長は一致するべき"
    );
    assert_eq!(
        lines[2], "1",
        "cache miss side effects 後の manual pair compile は plain manual pair と byte-identical であるべき: {:?}",
        lines
    );
}

/// TEST-CLI-02-M1F0R4: parse-src-decl-pair side effect 単独では manual pair compile が揺れないこと
#[test]
#[ignore]
fn test_e2e_selfhost_cli_parse_src_decl_pair_side_effect_matches_manual_pair_pipeline() {
    let dir = cli_test_fixture_dir("module_resolver_parse_src_decl_pair_side_effect");
    write_cli_fixture_files(
        &dir,
        &[
            ("lsharp.toml", ""),
            (
                "src/App/ModuleResolver.ls",
                selfhost_module("ModuleResolver.ls"),
            ),
        ],
    );

    let harness = with_stack_safe_wasm_bytes_eq_helpers(
        r#"
(defn compile-pair-state [pair]
  (do
    (root_push pair)
    (let [all-pairs (push-object-vector (vector-new 8) pair)]
      (do
        (root_push all-pairs)
        (let [n (vector-length all-pairs)
              reg-result (register-all-pairs all-pairs 0 n (ftable-new) 0)]
          (do
            (root_push reg-result)
            (let [ftable (vector-get reg-result 0)
                  data-ref (ref-new (vector-new 8))
                  functions (compile-all-src-decl-pairs all-pairs 0 n ftable data-ref (vector-new 8))
                  data (ref-get data-ref)]
              (do
                (root_push functions)
                (root_push data)
                (let [payload1 (vector-push (vector-new 2) functions)]
                  (do
                    (root_push payload1)
                    (let [payload2 (vector-push payload1 data)]
                      (do
                        (root_pop)
                        (root_pop)
                        (root_pop)
                        (root_pop)
                        (root_pop)
                        payload2))))))))))))
(defn compile-manual-state [path]
  (let [src (read-file path)
        program (parse-program src)
        pair (make-src-decl-pair src program)]
    (compile-pair-state pair)))
(defn compile-manual-after-parse-side-effect [path]
  (let [src (read-file path)
        parsed-pair (parse-src-decl-pair src)]
    (do
      (root_push parsed-pair)
      (let [program (parse-program src)
            manual-pair (make-src-decl-pair src program)]
        (do
          (root_pop)
          (compile-pair-state manual-pair))))))
(defn main []
  (let [plain-state (compile-manual-state "src/App/ModuleResolver.ls")
        parsed-state (compile-manual-after-parse-side-effect "src/App/ModuleResolver.ls")
        plain-functions (vector-get plain-state 0)
        plain-data (vector-get plain-state 1)
        parsed-functions (vector-get parsed-state 0)
        parsed-data (vector-get parsed-state 1)
        plain-wasm (build-wasm-bytes-wasi plain-functions plain-data)
        parsed-wasm (build-wasm-bytes-wasi parsed-functions parsed-data)]
    (do
      (print (vector-length plain-wasm))
      (print (vector-length parsed-wasm))
      (print (wasm-bytes-eq plain-wasm parsed-wasm))
      0)))
"#,
    );

    let combined = format!("{}\n{}", selfhost_cli_runtime_bundle(), harness);
    let output = compile_and_run_with_dir(&combined, &dir);
    let _ = std::fs::remove_dir_all(&dir);
    let lines: Vec<&str> = output.trim().lines().collect();

    assert!(
        lines.len() >= 3,
        "parse-src-decl-pair side effect vs manual pair 出力が不足: {:?}",
        lines
    );
    assert_eq!(
        lines[0], lines[1],
        "parse-src-decl-pair side effect 後の manual pair compile でも Wasm 長は一致するべき"
    );
    assert_eq!(
        lines[2], "1",
        "parse-src-decl-pair side effect 後の manual pair compile は plain manual pair と byte-identical であるべき: {:?}",
        lines
    );
}

/// TEST-CLI-02-M1F0R2: ModuleResolver の single-pair compile 後に root stack が空へ戻ること
#[test]
#[ignore]
fn test_e2e_selfhost_cli_manual_pair_compile_restores_root_stack() {
    let dir = cli_test_fixture_dir("module_resolver_manual_pair_root_stack");
    write_cli_fixture_files(
        &dir,
        &[
            ("lsharp.toml", ""),
            (
                "src/App/ModuleResolver.ls",
                selfhost_module("ModuleResolver.ls"),
            ),
        ],
    );

    let harness = r#"
(defn compile-pair-state [pair]
  (do
    (root_push pair)
    (let [all-pairs (push-object-vector (vector-new 8) pair)]
      (do
        (root_push all-pairs)
        (let [n (vector-length all-pairs)
              reg-result (register-all-pairs all-pairs 0 n (ftable-new) 0)]
          (do
            (root_push reg-result)
            (let [ftable (vector-get reg-result 0)
                  data-ref (ref-new (vector-new 8))
                  functions (compile-all-src-decl-pairs all-pairs 0 n ftable data-ref (vector-new 8))
                  data (ref-get data-ref)]
              (do
                (root_push functions)
                (root_push data)
                (let [payload1 (vector-push (vector-new 2) functions)]
                  (do
                    (root_push payload1)
                    (let [payload2 (vector-push payload1 data)]
                      (do
                        (root_pop)
                        (root_pop)
                        (root_pop)
                        (root_pop)
                        (root_pop)
                        (root_pop)
                        payload2))))))))))))
(defn compile-manual-state [path]
  (let [src (read-file path)
        program (parse-program src)
        pair (make-src-decl-pair src program)]
    (compile-pair-state pair)))
(defn main []
  (let [state (compile-manual-state "src/App/ModuleResolver.ls")]
    (do
      (print (vector-length (vector-get state 0)))
      0)))
"#;

    let combined = format!("{}\n{}", selfhost_cli_runtime_bundle(), harness);
    let (output, telemetry) = compile_and_capture_runtime_telemetry_with_dir(&combined, &dir);
    let _ = std::fs::remove_dir_all(&dir);
    let lines: Vec<&str> = output.trim().lines().collect();

    assert!(
        !lines.is_empty(),
        "manual pair compile root telemetry 出力が不足: {:?}",
        lines
    );
    assert_eq!(
        telemetry.root_stack_top, 0,
        "manual pair compile 後に root stack は空であるべき: {:?}",
        telemetry
    );
}

/// TEST-CLI-02-M1F0R3: path-parent 最小 fixture の single-pair compile 後に root stack が空へ戻ること
#[test]
#[ignore]
fn test_e2e_selfhost_cli_path_parent_manual_pair_compile_restores_root_stack() {
    let dir = cli_test_fixture_dir("path_parent_manual_pair_root_stack");
    write_cli_fixture_files(
        &dir,
        &[
            ("lsharp.toml", ""),
            (
                "src/App/ModuleResolver.ls",
                "(defn path-char [path idx] (string-char-at path idx))\n\
                 (defn is-path-sep [path idx] (let [ch (path-char path idx)] (if (= ch 47) true (if (= ch 92) true false))))\n\
                 (defn has-path-sep [path idx len] (if (>= idx len) false (if (is-path-sep path idx) true (has-path-sep path (+ idx 1) len))))\n\
                 (defn find-last-path-sep [path idx len last] (if (>= idx len) last (find-last-path-sep path (+ idx 1) len (if (is-path-sep path idx) idx last))))\n\
                 (defn path-parent [path] (let [len (string-length path)] (if (= len 0) \"\" (if (has-path-sep path 0 len) (let [last (find-last-path-sep path 0 len -1)] (if (< last 0) \"\" (if (= last 0) \"/\" (substring path 0 last)))) \".\"))))",
            ),
        ],
    );

    let harness = r#"
(defn compile-pair-state [pair]
  (do
    (root_push pair)
    (let [all-pairs (push-object-vector (vector-new 8) pair)]
      (do
        (root_push all-pairs)
        (let [n (vector-length all-pairs)
              reg-result (register-all-pairs all-pairs 0 n (ftable-new) 0)]
          (do
            (root_push reg-result)
            (let [ftable (vector-get reg-result 0)
                  data-ref (ref-new (vector-new 8))
                  functions (compile-all-src-decl-pairs all-pairs 0 n ftable data-ref (vector-new 8))
                  data (ref-get data-ref)]
              (do
                (root_push functions)
                (root_push data)
                (let [payload1 (vector-push (vector-new 2) functions)]
                  (do
                    (root_push payload1)
                    (let [payload2 (vector-push payload1 data)]
                      (do
                        (root_pop)
                        (root_pop)
                        (root_pop)
                        (root_pop)
                        (root_pop)
                        (root_pop)
                        payload2))))))))))))
(defn compile-manual-state [path]
  (let [src (read-file path)
        program (parse-program src)
        pair (make-src-decl-pair src program)]
    (compile-pair-state pair)))
(defn main []
  (let [state (compile-manual-state "src/App/ModuleResolver.ls")]
    (do
      (print (vector-length (vector-get state 0)))
      0)))
"#;

    let combined = format!("{}\n{}", selfhost_cli_runtime_bundle(), harness);
    let (output, telemetry) = compile_and_capture_runtime_telemetry_with_dir(&combined, &dir);
    let _ = std::fs::remove_dir_all(&dir);
    let lines: Vec<&str> = output.trim().lines().collect();

    assert!(
        !lines.is_empty(),
        "path-parent manual pair compile root telemetry 出力が不足: {:?}",
        lines
    );
    assert_eq!(
        telemetry.root_stack_top, 0,
        "path-parent manual pair compile 後に root stack は空であるべき: {:?}",
        telemetry
    );
}

/// TEST-CLI-02-M1F0R4: 最小 fixture の single-pair compile 後に root stack が空へ戻ること
#[test]
#[ignore]
fn test_e2e_selfhost_cli_trivial_manual_pair_compile_restores_root_stack() {
    let dir = cli_test_fixture_dir("trivial_manual_pair_root_stack");
    write_cli_fixture_files(
        &dir,
        &[
            ("lsharp.toml", ""),
            ("src/App/ModuleResolver.ls", "(defn main [] 1)"),
        ],
    );

    let harness = r#"
(defn compile-pair-state [pair]
  (do
    (root_push pair)
    (let [all-pairs (push-object-vector (vector-new 8) pair)]
      (do
        (root_push all-pairs)
        (let [n (vector-length all-pairs)
              reg-result (register-all-pairs all-pairs 0 n (ftable-new) 0)]
          (do
            (root_push reg-result)
            (let [ftable (vector-get reg-result 0)
                  data-ref (ref-new (vector-new 8))
                  functions (compile-all-src-decl-pairs all-pairs 0 n ftable data-ref (vector-new 8))
                  data (ref-get data-ref)]
              (do
                (root_push functions)
                (root_push data)
                (let [payload1 (vector-push (vector-new 2) functions)]
                  (do
                    (root_push payload1)
                    (let [payload2 (vector-push payload1 data)]
                      (do
                        (root_pop)
                        (root_pop)
                        (root_pop)
                        (root_pop)
                        (root_pop)
                        (root_pop)
                        payload2))))))))))))
(defn compile-manual-state [path]
  (let [src (read-file path)
        program (parse-program src)
        pair (make-src-decl-pair src program)]
    (compile-pair-state pair)))
(defn main []
  (let [state (compile-manual-state "src/App/ModuleResolver.ls")]
    (do
      (print (vector-length (vector-get state 0)))
      0)))
"#;

    let combined = format!("{}\n{}", selfhost_cli_runtime_bundle(), harness);
    let (output, telemetry) = compile_and_capture_runtime_telemetry_with_dir(&combined, &dir);
    let _ = std::fs::remove_dir_all(&dir);
    let lines: Vec<&str> = output.trim().lines().collect();

    assert!(
        !lines.is_empty(),
        "trivial manual pair compile root telemetry 出力が不足: {:?}",
        lines
    );
    assert_eq!(
        telemetry.root_stack_top, 0,
        "trivial manual pair compile 後に root stack は空であるべき: {:?}",
        telemetry
    );
}

/// TEST-CLI-02-M1F0S: ModuleResolver full source は source-fingerprint 実行後でも direct compile が deterministic であること
#[test]
#[ignore]
fn test_e2e_selfhost_cli_module_resolver_after_source_fingerprint_is_deterministic() {
    let dir = cli_test_fixture_dir("module_resolver_after_source_fingerprint_determinism");
    write_cli_fixture_files(
        &dir,
        &[(
            "ModuleResolverAfterFingerprint.ls",
            selfhost_module("ModuleResolver.ls"),
        )],
    );
    let fixture_path = dir
        .join("ModuleResolverAfterFingerprint.ls")
        .to_string_lossy()
        .replace('\\', "\\\\");

    let harness = with_stack_safe_wasm_bytes_eq_helpers(&format!(
        r#"
(defn compile-file-state [path]
  (let [src (read-file path)
        fingerprint (source-fingerprint src)
        program (parse-program src)
        pair (make-src-decl-pair src program)]
    (do
      (print fingerprint)
      (root_push pair)
      (let [all-pairs (push-object-vector (vector-new 8) pair)]
        (do
          (root_push all-pairs)
          (let [n (vector-length all-pairs)
                reg-result (register-all-pairs all-pairs 0 n (ftable-new) 0)]
            (do
              (root_push reg-result)
              (let [ftable (vector-get reg-result 0)
                    data-ref (ref-new (vector-new 8))
                    functions (compile-all-src-decl-pairs all-pairs 0 n ftable data-ref (vector-new 8))
                    data (ref-get data-ref)]
                (do
                  (root_push functions)
                  (root_push data)
                  (let [payload1 (vector-push (vector-new 2) functions)]
                    (do
                      (root_push payload1)
                      (let [payload2 (vector-push payload1 data)]
                        (do
                          (root_pop)
                          (root_pop)
                          (root_pop)
                          (root_pop)
                          (root_pop)
                          payload2)))))))))))))
(defn main []
  (let [state1 (compile-file-state "{fixture_path}")
        state2 (compile-file-state "{fixture_path}")
        functions1 (vector-get state1 0)
        data1 (vector-get state1 1)
        functions2 (vector-get state2 0)
        data2 (vector-get state2 1)
        wasm1 (build-wasm-bytes-wasi functions1 data1)
        wasm2 (build-wasm-bytes-wasi functions2 data2)]
    (do
      (print (vector-length wasm1))
      (print (vector-length wasm2))
      (print (wasm-bytes-eq wasm1 wasm2))
      0)))
"#
    ));

    let combined = format!("{}\n{}", selfhost_cli_runtime_bundle(), harness);
    let output = compile_and_run(&combined);
    let _ = std::fs::remove_dir_all(&dir);
    let lines: Vec<&str> = output.trim().lines().collect();

    assert!(
        lines.len() >= 5,
        "source-fingerprint 後 ModuleResolver determinism 出力が不足: {:?}",
        lines
    );
    assert_eq!(
        lines[0], lines[1],
        "2回の source-fingerprint 値は一致するべき"
    );
    assert_eq!(
        lines[2], lines[3],
        "2回の source-fingerprint 後 compile で Wasm 長は一致するべき"
    );
    assert_eq!(
        lines[4], "1",
        "2回の source-fingerprint 後 compile は byte-identical であるべき: {:?}",
        lines
    );
}

/// TEST-CLI-02-M1F0T: ModuleResolver full source は src-decl cache entry insert 後でも direct compile が deterministic であること
#[test]
#[ignore]
fn test_e2e_selfhost_cli_module_resolver_after_src_decl_cache_insert_is_deterministic() {
    let dir = cli_test_fixture_dir("module_resolver_after_src_decl_cache_insert_determinism");
    write_cli_fixture_files(
        &dir,
        &[(
            "ModuleResolverAfterCacheInsert.ls",
            selfhost_module("ModuleResolver.ls"),
        )],
    );
    let fixture_path = dir
        .join("ModuleResolverAfterCacheInsert.ls")
        .to_string_lossy()
        .replace('\\', "\\\\");

    let harness = with_stack_safe_wasm_bytes_eq_helpers(&format!(
        r#"
(defn compile-file-state [path]
  (let [src (read-file path)
        fingerprint (source-fingerprint src)
        cache-key (src-decl-cache-key path)
        cache-ref (ref-new (map-new))
        parse-count-ref (ref-new 0)
        pair (parse-src-decl-pair src)]
    (do
      (root_push pair)
      (let [entry (make-src-decl-cache-entry fingerprint pair)]
        (do
          (root_push entry)
          (ref-set parse-count-ref (+ (ref-get parse-count-ref) 1))
          (ref-set cache-ref (ref-map-insert-object-safe cache-ref cache-key entry))
          (root_pop)
          (let [all-pairs (push-object-vector (vector-new 8) pair)]
            (do
              (root_push all-pairs)
              (let [n (vector-length all-pairs)
                    reg-result (register-all-pairs all-pairs 0 n (ftable-new) 0)]
                (do
                  (root_push reg-result)
                  (let [ftable (vector-get reg-result 0)
                        data-ref (ref-new (vector-new 8))
                        functions (compile-all-src-decl-pairs all-pairs 0 n ftable data-ref (vector-new 8))
                        data (ref-get data-ref)]
                    (do
                      (root_push functions)
                      (root_push data)
                      (let [payload1 (vector-push (vector-new 2) functions)]
                        (do
                          (root_push payload1)
                          (let [payload2 (vector-push payload1 data)]
                            (do
                              (root_pop)
                              (root_pop)
                              (root_pop)
                              (root_pop)
                              (root_pop)
                              payload2)))))))))))))))
(defn main []
  (let [state1 (compile-file-state "{fixture_path}")
        state2 (compile-file-state "{fixture_path}")
        functions1 (vector-get state1 0)
        data1 (vector-get state1 1)
        functions2 (vector-get state2 0)
        data2 (vector-get state2 1)
        wasm1 (build-wasm-bytes-wasi functions1 data1)
        wasm2 (build-wasm-bytes-wasi functions2 data2)]
    (do
      (print (vector-length wasm1))
      (print (vector-length wasm2))
      (print (wasm-bytes-eq wasm1 wasm2))
      0)))
"#
    ));

    let combined = format!("{}\n{}", selfhost_cli_runtime_bundle(), harness);
    let output = compile_and_run(&combined);
    let _ = std::fs::remove_dir_all(&dir);
    let lines: Vec<&str> = output.trim().lines().collect();

    assert!(
        lines.len() >= 3,
        "src-decl cache entry insert 後 ModuleResolver determinism 出力が不足: {:?}",
        lines
    );
    assert_eq!(
        lines[0], lines[1],
        "2回の src-decl cache entry insert 後 compile で Wasm 長は一致するべき"
    );
    assert_eq!(
        lines[2], "1",
        "2回の src-decl cache entry insert 後 compile は byte-identical であるべき: {:?}",
        lines
    );
}

/// TEST-CLI-02-M1F0U: ModuleResolver full source は empty cache lookup 後でも direct compile が deterministic であること
#[test]
#[ignore]
fn test_e2e_selfhost_cli_module_resolver_after_src_decl_cache_lookup_is_deterministic() {
    let dir = cli_test_fixture_dir("module_resolver_after_src_decl_cache_lookup_determinism");
    write_cli_fixture_files(
        &dir,
        &[(
            "ModuleResolverAfterCacheLookup.ls",
            selfhost_module("ModuleResolver.ls"),
        )],
    );
    let fixture_path = dir
        .join("ModuleResolverAfterCacheLookup.ls")
        .to_string_lossy()
        .replace('\\', "\\\\");

    let harness = with_stack_safe_wasm_bytes_eq_helpers(&format!(
        r#"
(defn compile-file-state [path]
  (let [src (read-file path)
        fingerprint (source-fingerprint src)
        cache-ref (ref-new (map-new))
        cache-key (src-decl-cache-key path)
        cached-entry (ref-map-get-safe cache-ref cache-key)
        program (parse-program src)
        pair (make-src-decl-pair src program)]
    (do
      (print fingerprint)
      (print cached-entry)
      (root_push pair)
      (let [all-pairs (push-object-vector (vector-new 8) pair)]
        (do
          (root_push all-pairs)
          (let [n (vector-length all-pairs)
                reg-result (register-all-pairs all-pairs 0 n (ftable-new) 0)]
            (do
              (root_push reg-result)
              (let [ftable (vector-get reg-result 0)
                    data-ref (ref-new (vector-new 8))
                    functions (compile-all-src-decl-pairs all-pairs 0 n ftable data-ref (vector-new 8))
                    data (ref-get data-ref)]
                (do
                  (root_push functions)
                  (root_push data)
                  (let [payload1 (vector-push (vector-new 2) functions)]
                    (do
                      (root_push payload1)
                      (let [payload2 (vector-push payload1 data)]
                        (do
                          (root_pop)
                          (root_pop)
                          (root_pop)
                          (root_pop)
                          (root_pop)
                          payload2)))))))))))))
(defn main []
  (let [state1 (compile-file-state "{fixture_path}")
        state2 (compile-file-state "{fixture_path}")
        functions1 (vector-get state1 0)
        data1 (vector-get state1 1)
        functions2 (vector-get state2 0)
        data2 (vector-get state2 1)
        wasm1 (build-wasm-bytes-wasi functions1 data1)
        wasm2 (build-wasm-bytes-wasi functions2 data2)]
    (do
      (print (vector-length wasm1))
      (print (vector-length wasm2))
      (print (wasm-bytes-eq wasm1 wasm2))
      0)))
"#
    ));

    let combined = format!("{}\n{}", selfhost_cli_runtime_bundle(), harness);
    let output = compile_and_run(&combined);
    let _ = std::fs::remove_dir_all(&dir);
    let lines: Vec<&str> = output.trim().lines().collect();

    assert!(
        lines.len() >= 7,
        "empty cache lookup 後 ModuleResolver determinism 出力が不足: {:?}",
        lines
    );
    assert_eq!(
        lines[0], lines[2],
        "2回の source-fingerprint 値は一致するべき"
    );
    assert_eq!(lines[1], "0", "初回 empty cache lookup は 0 を返すべき");
    assert_eq!(lines[3], "0", "2回目 empty cache lookup も 0 を返すべき");
    assert_eq!(
        lines[4], lines[5],
        "2回の empty cache lookup 後 compile で Wasm 長は一致するべき"
    );
    assert_eq!(
        lines[6], "1",
        "2回の empty cache lookup 後 compile は byte-identical であるべき: {:?}",
        lines
    );
}

/// TEST-CLI-02-M1F: selfhost App.Main の direct compile は 2 回連続でも同じ Wasm を返すこと
#[test]
#[ignore]
fn test_e2e_selfhost_cli_direct_selfhost_main_compile_is_deterministic() {
    let dir = selfhost_package_root();

    let harness = r#"
(defn make-byte-fingerprint-state [done next-pos next-acc]
  (push-int-vector-local
    (push-int-vector-local
      (push-int-vector-local (vector-new 3) done)
      next-pos)
    next-acc))
(defn wasm-bytes-fingerprint-step [bytes pos end acc]
  (if (>= pos end)
    (make-byte-fingerprint-state 1 pos acc)
    (make-byte-fingerprint-state 0 (+ pos 1) (+ (* acc 31) (vector-get bytes pos)))))
(defn continue-wasm-bytes-fingerprint-step [bytes end state]
  (if (= (vector-get state 0) 1)
    state
    (wasm-bytes-fingerprint-step bytes (vector-get state 1) end (vector-get state 2))))
(defn wasm-bytes-fingerprint-step-8 [bytes pos end acc]
  (let [step1 (wasm-bytes-fingerprint-step bytes pos end acc)
        step2 (continue-wasm-bytes-fingerprint-step bytes end step1)
        step3 (continue-wasm-bytes-fingerprint-step bytes end step2)
        step4 (continue-wasm-bytes-fingerprint-step bytes end step3)
        step5 (continue-wasm-bytes-fingerprint-step bytes end step4)
        step6 (continue-wasm-bytes-fingerprint-step bytes end step5)
        step7 (continue-wasm-bytes-fingerprint-step bytes end step6)
        step8 (continue-wasm-bytes-fingerprint-step bytes end step7)]
    step8))
(defn continue-wasm-bytes-fingerprint-step-8 [bytes end state]
  (if (= (vector-get state 0) 1)
    state
    (wasm-bytes-fingerprint-step-8 bytes (vector-get state 1) end (vector-get state 2))))
(defn wasm-bytes-fingerprint-step-64 [bytes pos end acc]
  (let [step1 (wasm-bytes-fingerprint-step-8 bytes pos end acc)
        step2 (continue-wasm-bytes-fingerprint-step-8 bytes end step1)
        step3 (continue-wasm-bytes-fingerprint-step-8 bytes end step2)
        step4 (continue-wasm-bytes-fingerprint-step-8 bytes end step3)
        step5 (continue-wasm-bytes-fingerprint-step-8 bytes end step4)
        step6 (continue-wasm-bytes-fingerprint-step-8 bytes end step5)
        step7 (continue-wasm-bytes-fingerprint-step-8 bytes end step6)
        step8 (continue-wasm-bytes-fingerprint-step-8 bytes end step7)]
    step8))
(defn wasm-bytes-fingerprint-loop [bytes pos end acc]
  (let [step (wasm-bytes-fingerprint-step-64 bytes pos end acc)]
    (if (= (vector-get step 0) 1)
      (vector-get step 2)
      (wasm-bytes-fingerprint-loop bytes (vector-get step 1) end (vector-get step 2)))))
(defn wasm-bytes-fingerprint [bytes]
  (wasm-bytes-fingerprint-loop bytes 0 (vector-length bytes) 0))
(defn make-ir-fingerprint-state [done next-idx next-acc]
  (push-int-vector-local
    (push-int-vector-local
      (push-int-vector-local (vector-new 3) done)
      next-idx)
    next-acc))
(defn ir-fingerprint-step [ir idx count acc]
  (if (>= idx count)
    (make-ir-fingerprint-state 1 idx acc)
    (let [instr (vector-get ir idx)
          opcode (vector-get instr 0)
          operand (vector-get instr 1)]
      (make-ir-fingerprint-state 0 (+ idx 1) (+ (* (+ (* acc 31) opcode) 31) operand)))))
(defn continue-ir-fingerprint-step [ir count state]
  (if (= (vector-get state 0) 1)
    state
    (ir-fingerprint-step ir (vector-get state 1) count (vector-get state 2))))
(defn ir-fingerprint-step-8 [ir idx count acc]
  (let [step1 (ir-fingerprint-step ir idx count acc)
        step2 (continue-ir-fingerprint-step ir count step1)
        step3 (continue-ir-fingerprint-step ir count step2)
        step4 (continue-ir-fingerprint-step ir count step3)
        step5 (continue-ir-fingerprint-step ir count step4)
        step6 (continue-ir-fingerprint-step ir count step5)
        step7 (continue-ir-fingerprint-step ir count step6)
        step8 (continue-ir-fingerprint-step ir count step7)]
    step8))
(defn continue-ir-fingerprint-step-8 [ir count state]
  (if (= (vector-get state 0) 1)
    state
    (ir-fingerprint-step-8 ir (vector-get state 1) count (vector-get state 2))))
(defn ir-fingerprint-step-64 [ir idx count acc]
  (let [step1 (ir-fingerprint-step-8 ir idx count acc)
        step2 (continue-ir-fingerprint-step-8 ir count step1)
        step3 (continue-ir-fingerprint-step-8 ir count step2)
        step4 (continue-ir-fingerprint-step-8 ir count step3)
        step5 (continue-ir-fingerprint-step-8 ir count step4)
        step6 (continue-ir-fingerprint-step-8 ir count step5)
        step7 (continue-ir-fingerprint-step-8 ir count step6)
        step8 (continue-ir-fingerprint-step-8 ir count step7)]
    step8))
(defn ir-fingerprint-loop [ir idx count acc]
  (let [step (ir-fingerprint-step-64 ir idx count acc)]
    (if (= (vector-get step 0) 1)
      (vector-get step 2)
      (ir-fingerprint-loop ir (vector-get step 1) count (vector-get step 2)))))
(defn ir-fingerprint [ir]
  (ir-fingerprint-loop ir 0 (vector-length ir) 0))
(defn function-fingerprint [func]
  (let [param-count (vector-get func 0)
        local-count (vector-get func 1)
        ir (vector-get func 2)]
    (+ (* (+ (* param-count 31) local-count) 31) (ir-fingerprint ir))))
(defn first-function-mismatch [functions1 functions2 idx count]
  (if (>= idx count)
    -1
    (if (= (function-fingerprint (vector-get functions1 idx)) (function-fingerprint (vector-get functions2 idx)))
      (first-function-mismatch functions1 functions2 (+ idx 1) count)
      idx)))
(defn first-ir-mismatch [ir1 ir2 idx count]
  (if (>= idx count)
    -1
    (let [instr1 (vector-get ir1 idx)
          instr2 (vector-get ir2 idx)]
      (if (and (= (vector-get instr1 0) (vector-get instr2 0)) (= (vector-get instr1 1) (vector-get instr2 1)))
        (first-ir-mismatch ir1 ir2 (+ idx 1) count)
        idx))))
(defn instr-op-at [ir idx]
  (if (or (< idx 0) (>= idx (vector-length ir)))
    -1
    (vector-get (vector-get ir idx) 0)))
(defn instr-operand-at [ir idx]
  (if (or (< idx 0) (>= idx (vector-length ir)))
    -1
    (vector-get (vector-get ir idx) 1)))
(defn count-defns-in-decls [decls idx n]
  (if (>= idx n)
    0
    (+ (if (= (vector-get (vector-get decls idx) 0) 20) 1 0)
       (count-defns-in-decls decls (+ idx 1) n))))
(defn count-defns-in-pair [pair]
  (let [decls (vector-get pair 1)]
    (count-defns-in-decls decls 0 (vector-length decls))))
(defn find-owner-pair-index [pairs pair-idx pair-count target base]
  (if (>= pair-idx pair-count)
    -1
    (let [pair (vector-get pairs pair-idx)
          pair-defn-count (count-defns-in-pair pair)]
      (if (< target (+ base pair-defn-count))
        pair-idx
        (find-owner-pair-index pairs (+ pair-idx 1) pair-count target (+ base pair-defn-count))))))
(defn find-owner-pair-base [pairs pair-idx pair-count target base]
  (if (>= pair-idx pair-count)
    -1
    (let [pair (vector-get pairs pair-idx)
          pair-defn-count (count-defns-in-pair pair)]
      (if (< target (+ base pair-defn-count))
        base
        (find-owner-pair-base pairs (+ pair-idx 1) pair-count target (+ base pair-defn-count))))))
(defn find-line-end [src idx len]
  (if (>= idx len)
    idx
    (if (= (string-char-at src idx) 10)
      idx
      (find-line-end src (+ idx 1) len))))
(defn source-first-line [src]
  (let [len (string-length src)
        end (find-line-end src 0 len)]
    (substring src 0 end)))
(defn make-functions-data-pair [functions data]
  (push-object-vector (push-object-vector (vector-new 2) functions) data))
(defn compile-inline-file-state [path func-idx]
  (let [src (read-file path)
        program (parse-program src)
        source-root (resolve-source-root path)
        package-root (resolve-package-root path)
        seen-ref (ref-new (map-new))
        imported-pairs (load-imports-from-decls program src 0 (vector-length program) seen-ref (vector-new 8) source-root package-root)
        all-pairs (append-src-decl-pair imported-pairs src program)
        n (vector-length all-pairs)
        reg-result (register-all-pairs all-pairs 0 n (ftable-new) func-idx)
        ftable (vector-get reg-result 0)
        data-ref (ref-new (vector-new 8))
        functions (compile-all-src-decl-pairs all-pairs 0 n ftable data-ref (vector-new 8))
        data (ref-get data-ref)]
    (push-object-vector (make-functions-data-pair functions data) all-pairs)))
(defn main []
  (let [state1 (compile-inline-file-state "src/App/Main.ls" 7)
        state2 (compile-inline-file-state "src/App/Main.ls" 7)
        functions1 (vector-get state1 0)
        data1 (vector-get state1 1)
        pairs1 (vector-get state1 2)
        functions2 (vector-get state2 0)
        data2 (vector-get state2 1)
        function-count (vector-length functions1)
        mismatch-idx (first-function-mismatch functions1 functions2 0 function-count)
        owner-pair-idx (find-owner-pair-index pairs1 0 (vector-length pairs1) mismatch-idx 0)
        owner-pair-base (find-owner-pair-base pairs1 0 (vector-length pairs1) mismatch-idx 0)
        owner-local-idx (- mismatch-idx owner-pair-base)
        owner-source (if (< owner-pair-idx 0) "" (vector-get (vector-get pairs1 owner-pair-idx) 0))
        owner-module-line (source-first-line owner-source)
        mismatch-func1 (if (< mismatch-idx 0) (vector-new 3) (vector-get functions1 mismatch-idx))
        mismatch-func2 (if (< mismatch-idx 0) (vector-new 3) (vector-get functions2 mismatch-idx))
        mismatch-ir-vec1 (if (< mismatch-idx 0) (vector-new 2) (vector-get mismatch-func1 2))
        mismatch-ir-vec2 (if (< mismatch-idx 0) (vector-new 2) (vector-get mismatch-func2 2))
        mismatch-ir-idx (if (< mismatch-idx 0) -1 (first-ir-mismatch mismatch-ir-vec1 mismatch-ir-vec2 0 (vector-length mismatch-ir-vec1)))
        mismatch-instr1 (if (< mismatch-ir-idx 0) (vector-new 2) (vector-get mismatch-ir-vec1 mismatch-ir-idx))
        mismatch-instr2 (if (< mismatch-ir-idx 0) (vector-new 2) (vector-get mismatch-ir-vec2 mismatch-ir-idx))
        mismatch-local1 (if (< mismatch-idx 0) -1 (vector-get mismatch-func1 1))
        mismatch-local2 (if (< mismatch-idx 0) -1 (vector-get mismatch-func2 1))
        mismatch-ir1 (if (< mismatch-idx 0) -1 (ir-fingerprint (vector-get mismatch-func1 2)))
        mismatch-ir2 (if (< mismatch-idx 0) -1 (ir-fingerprint (vector-get mismatch-func2 2)))
        type1 (emit-type-section-wasi-quad-functions functions1)
        type2 (emit-type-section-wasi-quad-functions functions2)
        func1 (emit-function-section-wasi-quad-functions functions1)
        func2 (emit-function-section-wasi-quad-functions functions2)
        code1 (emit-code-section-wasi-quad-functions functions1)
        code2 (emit-code-section-wasi-quad-functions functions2)
        data-sec1 (emit-data-section data1 1024)
        data-sec2 (emit-data-section data2 1024)
        wasm1 (build-wasm-bytes-wasi functions1 data1)
        wasm2 (build-wasm-bytes-wasi functions2 data2)]
    (do
      (print (vector-length wasm1))
      (print (vector-length wasm2))
      (print (wasm-bytes-fingerprint type1))
      (print (wasm-bytes-fingerprint type2))
      (print (wasm-bytes-fingerprint func1))
      (print (wasm-bytes-fingerprint func2))
      (print (wasm-bytes-fingerprint code1))
      (print (wasm-bytes-fingerprint code2))
      (print (wasm-bytes-fingerprint data-sec1))
      (print (wasm-bytes-fingerprint data-sec2))
      (print mismatch-idx)
      (print owner-pair-idx)
      (print owner-local-idx)
      (print mismatch-local1)
      (print mismatch-local2)
      (print mismatch-ir1)
      (print mismatch-ir2)
      (print mismatch-ir-idx)
      (print (if (< mismatch-ir-idx 0) -1 (vector-get mismatch-instr1 0)))
      (print (if (< mismatch-ir-idx 0) -1 (vector-get mismatch-instr1 1)))
      (print (if (< mismatch-ir-idx 0) -1 (vector-get mismatch-instr2 0)))
      (print (if (< mismatch-ir-idx 0) -1 (vector-get mismatch-instr2 1)))
      (print (instr-op-at mismatch-ir-vec1 38))
      (print (instr-operand-at mismatch-ir-vec1 38))
      (print (instr-op-at mismatch-ir-vec1 39))
      (print (instr-operand-at mismatch-ir-vec1 39))
      (print (instr-op-at mismatch-ir-vec1 40))
      (print (instr-operand-at mismatch-ir-vec1 40))
      (print (instr-op-at mismatch-ir-vec1 41))
      (print (instr-operand-at mismatch-ir-vec1 41))
      (print (instr-op-at mismatch-ir-vec1 42))
      (print (instr-operand-at mismatch-ir-vec1 42))
      (print (instr-op-at mismatch-ir-vec1 43))
      (print (instr-operand-at mismatch-ir-vec1 43))
      (print (instr-op-at mismatch-ir-vec1 44))
      (print (instr-operand-at mismatch-ir-vec1 44))
      (print (instr-op-at mismatch-ir-vec1 45))
      (print (instr-operand-at mismatch-ir-vec1 45))
      (print (instr-op-at mismatch-ir-vec1 46))
      (print (instr-operand-at mismatch-ir-vec1 46))
      (print (instr-op-at mismatch-ir-vec2 38))
      (print (instr-operand-at mismatch-ir-vec2 38))
      (print (instr-op-at mismatch-ir-vec2 39))
      (print (instr-operand-at mismatch-ir-vec2 39))
      (print (instr-op-at mismatch-ir-vec2 40))
      (print (instr-operand-at mismatch-ir-vec2 40))
      (print (instr-op-at mismatch-ir-vec2 41))
      (print (instr-operand-at mismatch-ir-vec2 41))
      (print (instr-op-at mismatch-ir-vec2 42))
      (print (instr-operand-at mismatch-ir-vec2 42))
      (print (instr-op-at mismatch-ir-vec2 43))
      (print (instr-operand-at mismatch-ir-vec2 43))
      (print (instr-op-at mismatch-ir-vec2 44))
      (print (instr-operand-at mismatch-ir-vec2 44))
      (print (instr-op-at mismatch-ir-vec2 45))
      (print (instr-operand-at mismatch-ir-vec2 45))
      (print (instr-op-at mismatch-ir-vec2 46))
      (print (instr-operand-at mismatch-ir-vec2 46))
      (print owner-module-line)
      (print (wasm-bytes-fingerprint wasm1))
      (print (wasm-bytes-fingerprint wasm2))
      0)))
"#;

    let combined = format!("{}\n{}", selfhost_cli_runtime_bundle(), harness);
    let output = compile_and_run_with_dir(&combined, &dir);
    let lines: Vec<&str> = output.trim().lines().collect();

    assert!(
        lines.len() >= 61,
        "selfhost main direct determinism 出力が不足: {:?}",
        lines
    );
    assert_eq!(
        lines[0], lines[1],
        "2回の selfhost main compile で Wasm 長は一致するべき"
    );
    assert_eq!(
        lines[2], lines[3],
        "type section は一致するべき: {:?}",
        lines
    );
    assert_eq!(
        lines[4], lines[5],
        "function section は一致するべき: {:?}",
        lines
    );
    assert_eq!(
        lines[6], lines[7],
        "code section は一致するべき: {:?}",
        lines
    );
    assert_eq!(
        lines[8], lines[9],
        "data section は一致するべき: {:?}",
        lines
    );
    assert_eq!(
        lines[59], lines[60],
        "2回の selfhost main direct compile は fingerprint 一致であるべき: {:?}",
        lines
    );
}

/// TEST-CLI-02-M2: selfhost/src/App/Cli.ls の run-build が file-path から source を読めること
#[test]
#[ignore]
fn test_e2e_selfhost_cli_build_file_handler() {
    let dir =
        std::env::temp_dir().join(format!("lsharp_test_cli_build_file_{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    std::fs::write(dir.join("input.ls"), "(defn main [] 42)").unwrap();

    let harness = r#"
(defn main []
  (do
    (print (run-build "input.ls" 0))
    0))
"#;

    let combined = format!("{}\n{}", selfhost_cli_runtime_bundle(), harness);
    let output = compile_and_run_with_dir(&combined, &dir);
    let _ = std::fs::remove_dir_all(&dir);
    let lines: Vec<&str> = output.trim().lines().collect();

    assert!(lines.len() >= 2, "run-build 出力が不足: {:?}", lines);
    assert!(
        lines[0].starts_with("wasm-size:"),
        "run-build は wasm-size:<n> を返すべき: {:?}",
        lines
    );
    let wasm_size: i64 = lines[0]["wasm-size:".len()..]
        .parse()
        .expect("wasm size は整数であるべき");
    assert!(
        wasm_size > 8,
        "wasm size は header 超であるべき: {}",
        wasm_size
    );
    assert_eq!(lines[1], "0", "run-build の終了コードは success であるべき");
}

/// TEST-CLI-02-M2B: selfhost/src/App/Cli.ls の run-build は nested import fixture を import-aware helper 経由で解決すること
#[test]
#[ignore]
fn test_e2e_selfhost_cli_build_file_handler_multifile_nested_imports() {
    let dir = cli_test_fixture_dir("build_multifile_nested");
    write_cli_fixture_files(&dir, &cli_multifile_nested_fixture_files());

    let harness = r#"
(defn main []
  (let [src (read-file "main.ls")]
    (do
      (print (run-build "main.ls" 0))
      (print (compile-file-wasm-size "main.ls" 0))
      (print (run-compile-source src 0))
      0)))
"#;

    let combined = format!("{}\n{}", selfhost_cli_runtime_bundle(), harness);
    let output = compile_and_run_with_dir(&combined, &dir);
    let _ = std::fs::remove_dir_all(&dir);
    let lines: Vec<&str> = output.trim().lines().collect();

    assert!(
        lines.len() >= 5,
        "run-build multi-file nested fixture 出力が不足: {:?}",
        lines
    );
    let file_size = parse_wasm_size_line(lines[0], "run-build multi-file nested fixture");
    let helper_size = parse_i64_line(lines[2], "compile-file-wasm-size nested fixture");
    let source_only_size =
        parse_wasm_size_line(lines[3], "run-compile-source nested fixture baseline");
    assert_eq!(lines[1], "0", "run-build は success=0 を返すべき");
    assert_eq!(
        lines[4], "0",
        "run-compile-source baseline は success=0 を返すべき"
    );
    assert!(
        file_size == helper_size,
        "run-build は import-aware helper と同じ wasm-size を返すべき: cli={file_size}, helper={helper_size}"
    );
    assert!(
        helper_size > source_only_size,
        "compile-file-wasm-size helper は source-only baseline より大きい wasm-size を返すべき: helper={helper_size}, source-only={source_only_size}"
    );
}

/// TEST-CLI-02-M3: selfhost/src/App/Cli.ls の run-install が install plan text を返せること
#[test]
#[ignore]
fn test_e2e_selfhost_cli_install_package_core() {
    let harness = r#"
(defn main []
  (do
    (print (run-install "core" 0))
    0))
"#;

    let combined = format!("{}\n{}", selfhost_cli_runtime_bundle(), harness);
    let output = compile_and_run(&combined);
    let lines: Vec<&str> = output.trim().lines().collect();

    assert_eq!(
        lines,
        vec!["package:core", "status:planned", "0"],
        "run-install は package install plan text と success=0 を返すべき"
    );
}

/// TEST-CLI-02-M4: selfhost/src/App/Cli.ls の run-install は空 package を compile error にする
#[test]
#[ignore]
fn test_e2e_selfhost_cli_install_empty_package() {
    let harness = r#"
(defn main []
  (print (run-install "" 0)))
"#;

    let combined = format!("{}\n{}", selfhost_cli_runtime_bundle(), harness);
    let output = compile_and_run(&combined);
    let lines: Vec<&str> = output.trim().lines().collect();

    assert_eq!(
        lines,
        vec!["1"],
        "run-install は空 package に compile error=1 を返すべき"
    );
}

/// TEST-CLI-02-M5: selfhost/src/App/Cli.ls の run-repl が warmup session summary を返せること
#[test]
#[ignore]
fn test_e2e_selfhost_cli_repl_core() {
    let harness = r#"
(defn main []
  (do
    (print (run-repl 0))
    0))
"#;

    let combined = format!("{}\n{}", selfhost_cli_runtime_bundle(), harness);
    let output = compile_and_run(&combined);
    let lines: Vec<&str> = output.trim().lines().collect();

    assert_eq!(
        lines,
        vec!["type:Int", "evals:1", "input-bytes:17", "0"],
        "run-repl は warmup session summary と success=0 を返すべき"
    );
}

/// TEST-CLI-02-M6: selfhost/src/App/Cli.ls の run-lsp が capability summary text を返せること
#[test]
#[ignore]
fn test_e2e_selfhost_cli_lsp_core() {
    let harness = r#"
(defn main []
  (do
    (print (run-lsp 0))
    0))
"#;

    let combined = format!("{}\n{}", selfhost_cli_runtime_bundle(), harness);
    let output = compile_and_run(&combined);
    let lines: Vec<&str> = output.trim().lines().collect();

    assert_eq!(
        lines,
        vec![
            "sync:full",
            "hover:true",
            "completion:true",
            "definition:true",
            "references:true",
            "rename:true",
            "formatting:true",
            "requests:1",
            "documents:0",
            "source-bytes:0",
            "0",
        ],
        "run-lsp は capability + shared-state summary text と success=0 を返すべき"
    );
}

/// TEST-CLI-02-M7: selfhost/src/App/Cli.ls の LSP transport helper が initialize request を frame response にできること
#[test]
#[ignore]
fn test_e2e_selfhost_cli_lsp_transport_initialize_frame() {
    let body = r#"{"jsonrpc":"2.0","id":7,"result":[1,1,1,1,1,1,1]}"#;
    let expected = format!("Content-Length: {}\r\n\r\n{}", body.len(), body);
    let harness = r#"
(defn main []
  (let [request
          (vector-push
            (vector-push
              (vector-push
                (vector-push (vector-new 4) 2)
                7)
              (lsp-method-initialize))
            0)]
    (print-string (run-lsp-transport-request request))))
"#;

    let combined = format!("{}\n{}", selfhost_cli_runtime_bundle(), harness);
    let output = compile_and_run(&combined);

    assert_eq!(
        output, expected,
        "run-lsp-transport-request は initialize request を framed response に変換すべき"
    );
}

/// TEST-CLI-02-M8: selfhost/src/App/Cli.ls の LSP transport helper が未知メソッドを JSON-RPC error frame にできること
#[test]
#[ignore]
fn test_e2e_selfhost_cli_lsp_transport_unknown_method_error() {
    let body = r#"{"jsonrpc":"2.0","id":9,"error":{"code":-32601,"message":"Method not found"}}"#;
    let expected = format!("Content-Length: {}\r\n\r\n{}", body.len(), body);
    let harness = r#"
(defn main []
  (let [request
          (vector-push
            (vector-push
              (vector-push
                (vector-push (vector-new 4) 2)
                9)
              999)
            0)]
    (print-string (run-lsp-transport-request request))))
"#;

    let combined = format!("{}\n{}", selfhost_cli_runtime_bundle(), harness);
    let output = compile_and_run(&combined);

    assert_eq!(
        output, expected,
        "run-lsp-transport-request は未知メソッドに Method not found frame を返すべき"
    );
}

/// TEST-CLI-02-M8b: selfhost/src/App/Cli.ls の LSP transport helper は shutdown 後 request を error frame にすること
#[test]
#[ignore]
fn test_e2e_selfhost_cli_lsp_transport_request_after_shutdown_error() {
    let body = r#"{"jsonrpc":"2.0","id":10,"error":{"code":-32600,"message":"Invalid Request"}}"#;
    let expected = format!("Content-Length: {}\r\n\r\n{}", body.len(), body);
    let harness = r#"
(defn make-request [id method-id params]
  (vector-push
    (vector-push
      (vector-push
        (vector-push (vector-new 4) 2)
        id)
      method-id)
    params))

(defn main []
  (let [shutdown-request
          (make-request 9 (lsp-method-shutdown) 0)
        hover-request
          (make-request
            10
            (lsp-method-hover)
            (vector-push
              (vector-push
                (vector-push (vector-new 3) 42)
                1)
              1))
        requests
          (vector-push
            (vector-push (vector-new 2) shutdown-request)
            hover-request)
        summary (run-lsp-transport-sequence requests)
        frames (vector-get summary 0)]
    (print-string (vector-get frames 1))))
"#;

    let combined = format!("{}\n{}", selfhost_cli_runtime_bundle(), harness);
    let output = compile_and_run(&combined);

    assert_eq!(
        output, expected,
        "run-lsp-transport-request は shutdown 後 request を Invalid Request frame で拒否するべき"
    );
}

/// TEST-CLI-02-M9: selfhost/src/App/Cli.ls の LSP transport helper sequence が shared-state で複数 request を捌けること
#[test]
#[ignore]
fn test_e2e_selfhost_cli_lsp_transport_goto_definition_frame() {
    let body = r#"{"jsonrpc":"2.0","id":7,"result":[10,1,7]}"#;
    let expected = format!("Content-Length: {}\r\n\r\n{}", body.len(), body);
    let source = "(defn helper [x] x)\n(defn main [] (helper 1))";
    let harness = format!(
        r#"
(defn main []
  (let [params
          (vector-push
            (vector-push
              (vector-push
                (vector-push (vector-new 4) 10)
                2)
              16)
            "{source}")
        request
          (vector-push
            (vector-push
              (vector-push
                (vector-push (vector-new 4) 2)
                7)
              (lsp-method-goto-def))
            params)]
    (print-string (run-lsp-transport-request request))))
"#
    );

    let combined = format!("{}\n{}", selfhost_cli_runtime_bundle(), harness);
    let output = compile_and_run(&combined);

    assert_eq!(
        output, expected,
        "run-lsp-transport-request は goto-definition request を framed response に変換すべき"
    );
}

/// TEST-CLI-02-M9b: selfhost/src/App/Cli.ls の LSP transport helper が hover request を framed response にできること
#[test]
#[ignore]
fn test_e2e_selfhost_cli_lsp_transport_hover_frame() {
    let body =
        r#"{"jsonrpc":"2.0","id":8,"result":{"range":[2,16,2,22],"contents":"defn square"}}"#;
    let expected = format!("Content-Length: {}\r\n\r\n{}", body.len(), body);
    let source = "(defn square [x] x)\n(defn main [] (square 1) (square 2))";
    let harness = format!(
        r#"
(defn main []
  (let [params
          (vector-push
            (vector-push
              (vector-push
                (vector-push (vector-new 4) 99)
                2)
              17)
            "{source}")
        request
          (vector-push
            (vector-push
              (vector-push
                (vector-push (vector-new 4) 2)
                8)
              (lsp-method-hover))
            params)]
    (print-string (run-lsp-transport-request request))))
"#
    );

    let combined = format!("{}\n{}", selfhost_cli_runtime_bundle(), harness);
    let output = compile_and_run(&combined);

    assert_eq!(
        output, expected,
        "run-lsp-transport-request は hover request を framed response に変換すべき"
    );
}

/// TEST-CLI-02-M9c: selfhost/src/App/Cli.ls の LSP transport helper が references request を framed response にできること
#[test]
#[ignore]
fn test_e2e_selfhost_cli_lsp_transport_references_frame() {
    let body = r#"{"jsonrpc":"2.0","id":10,"result":[[99,1,7],[99,2,16],[99,2,27]]}"#;
    let expected = format!("Content-Length: {}\r\n\r\n{}", body.len(), body);
    let source = "(defn square [x] x)\n(defn main [] (square 1) (square 2))";
    let harness = format!(
        r#"
(defn main []
  (let [params
          (vector-push
            (vector-push
              (vector-push
                (vector-push (vector-new 4) 99)
                2)
              17)
            "{source}")
        request
          (vector-push
            (vector-push
              (vector-push
                (vector-push (vector-new 4) 2)
                10)
              (lsp-method-references))
            params)]
    (print-string (run-lsp-transport-request request))))
"#
    );

    let combined = format!("{}\n{}", selfhost_cli_runtime_bundle(), harness);
    let output = compile_and_run(&combined);

    assert_eq!(
        output, expected,
        "run-lsp-transport-request は references request を framed response に変換すべき"
    );
}

/// TEST-CLI-02-M9d: selfhost/src/App/Cli.ls の LSP transport helper が completion request を framed response にできること
#[test]
#[ignore]
fn test_e2e_selfhost_cli_lsp_transport_completion_frame() {
    let body = r#"{"jsonrpc":"2.0","id":11,"result":[["defn",14,"defn"],["let",14,"let"],["if",14,"if"],["match",14,"match"],["do",14,"do"],["fn",14,"fn"],["module",14,"module"]]}"#;
    let expected = format!("Content-Length: {}\r\n\r\n{}", body.len(), body);
    let harness = r#"
(defn main []
  (let [request
          (vector-push
            (vector-push
              (vector-push
                (vector-push (vector-new 4) 2)
                11)
              (lsp-method-completion))
            0)]
    (print-string (run-lsp-transport-request request))))
"#;

    let combined = format!("{}\n{}", selfhost_cli_runtime_bundle(), harness);
    let output = compile_and_run(&combined);

    assert_eq!(
        output, expected,
        "run-lsp-transport-request は completion request を framed response に変換すべき"
    );
}

/// TEST-CLI-02-M9e: selfhost/src/App/Cli.ls の LSP transport helper が formatting request を framed response にできること
#[test]
#[ignore]
fn test_e2e_selfhost_cli_lsp_transport_formatting_frame() {
    let body = "{\"jsonrpc\":\"2.0\",\"id\":12,\"result\":[[1,1,2,4,\"(defn main [] 1)\\n\"]]}";
    let expected = format!("Content-Length: {}\r\n\r\n{}", body.len(), body);
    let source = "(defn main []\n 1)";
    let harness = format!(
        r#"
(defn main []
  (let [params
          (vector-push
            (vector-push (vector-new 2) 77)
            "{source}")
        request
          (vector-push
            (vector-push
              (vector-push
                (vector-push (vector-new 4) 2)
                12)
              (lsp-method-formatting))
            params)]
    (print-string (run-lsp-transport-request request))))
"#
    );

    let combined = format!("{}\n{}", selfhost_cli_runtime_bundle(), harness);
    let output = compile_and_run(&combined);

    assert_eq!(
        output, expected,
        "run-lsp-transport-request は formatting request を framed response に変換すべき"
    );
}

/// TEST-CLI-02-M9f: selfhost/src/App/Cli.ls の LSP transport helper が rename request を framed response にできること
#[test]
#[ignore]
fn test_e2e_selfhost_cli_lsp_transport_rename_frame() {
    let body = r#"{"jsonrpc":"2.0","id":13,"result":[[99,[[1,7,1,13,"cube"],[2,16,2,22,"cube"],[2,27,2,33,"cube"]]]]}"#;
    let expected = format!("Content-Length: {}\r\n\r\n{}", body.len(), body);
    let source = "(defn square [x] x)\n(defn main [] (square 1) (square 2))";
    let harness = format!(
        r#"
(defn main []
  (let [params
          (vector-push
            (vector-push
              (vector-push
                (vector-push
                  (vector-push (vector-new 5) 99)
                  2)
                17)
              "{source}")
            "cube")
        request
          (vector-push
            (vector-push
              (vector-push
                (vector-push (vector-new 4) 2)
                13)
              (lsp-method-rename))
            params)]
    (print-string (run-lsp-transport-request request))))
"#
    );

    let combined = format!("{}\n{}", selfhost_cli_runtime_bundle(), harness);
    let output = compile_and_run(&combined);

    assert_eq!(
        output, expected,
        "run-lsp-transport-request は rename request を framed response に変換すべき"
    );
}

/// TEST-CLI-02-M9: selfhost/src/App/Cli.ls の LSP transport helper sequence が shared-state で複数 request を捌けること
#[test]
#[ignore]
fn test_e2e_selfhost_cli_lsp_transport_sequence_summary() {
    let init_body = r#"{"jsonrpc":"2.0","id":3,"result":[1,1,1,1,1,1,1]}"#;
    let init_frame = format!("Content-Length: {}\r\n\r\n{}", init_body.len(), init_body);
    let shutdown_body = r#"{"jsonrpc":"2.0","id":4,"result":0}"#;
    let shutdown_frame = format!(
        "Content-Length: {}\r\n\r\n{}",
        shutdown_body.len(),
        shutdown_body
    );
    let harness = r#"
(defn main []
  (let [init-request
          (vector-push
            (vector-push
              (vector-push
                (vector-push (vector-new 4) 2)
                3)
              (lsp-method-initialize))
            0)
        shutdown-request
          (vector-push
            (vector-push
              (vector-push
                (vector-push (vector-new 4) 2)
                4)
              (lsp-method-shutdown))
            0)
        requests
          (vector-push
            (vector-push (vector-new 2) init-request)
            shutdown-request)
        summary (run-lsp-transport-sequence requests)
        frames (vector-get summary 0)]
    (do
      (print-string (vector-get frames 0))
      (print-string "\n---\n")
      (print-string (vector-get frames 1))
      (print-string "\n---\n")
      (print (vector-length frames))
      (print (vector-get summary 2)))))
"#;

    let combined = format!("{}\n{}", selfhost_cli_runtime_bundle(), harness);
    let output = compile_and_run(&combined);
    let parts: Vec<&str> = output.split("\n---\n").collect();

    assert_eq!(
        parts.len(),
        3,
        "transport sequence output format が不正: {:?}",
        output
    );
    assert_eq!(
        parts[0], init_frame,
        "frame0 は initialize response であるべき"
    );
    assert_eq!(
        parts[1], shutdown_frame,
        "frame1 は shutdown response であるべき"
    );
    assert_eq!(
        parts[2].trim().lines().collect::<Vec<_>>(),
        vec!["2", "2"],
        "sequence summary は frame-count=2 / request-count=2 を返すべき"
    );
}

/// TEST-CLI-02-M10: publishDiagnostics notification が deterministic JSON/frame と request-count を返すこと
#[test]
#[ignore]
fn test_e2e_selfhost_cli_lsp_transport_publish_diagnostics_frame() {
    let diagnostics_json =
        r#"[{"source":1,"severity":1,"rule":203,"line":2,"col":4,"messageHash":7003}]"#;
    let notification = format!(
        r#"{{"jsonrpc":"2.0","method":"textDocument/publishDiagnostics","params":{{"uri":42,"diagnostics":{}}}}}"#,
        diagnostics_json
    );
    let expected_frame = format!(
        "Content-Length: {}\r\n\r\n{}",
        notification.len(),
        notification
    );
    let harness = r#"
(defn main []
  (let [state (server-state-new)
        diag (vector-push
               (vector-push
                 (vector-push
                   (vector-push
                     (vector-push
                       (vector-push (vector-new 6) 1)
                       203)
                     2)
                   4)
                 7003)
               1)
        diags (vector-push (vector-new 1) diag)
        params (vector-push (vector-push (vector-new 2) 42) diags)
        result (json-rpc-dispatch (lsp-method-publish-diagnostics) params state)]
    (do
      (print-string (vector-get result 1))
      (print-string "\n---\n")
      (print-string (lsp-render-publish-diagnostics-frame 42 diags))
      (print-string "\n---\n")
      (print (server-state-request-count state)))))
"#;

    let combined = format!("{}\n{}", selfhost_cli_runtime_bundle(), harness);
    let output = compile_and_run(&combined);
    let parts: Vec<&str> = output.split("\n---\n").collect();

    assert_eq!(
        parts.len(),
        3,
        "publishDiagnostics output format が不正: {:?}",
        output
    );
    assert_eq!(
        parts[0], diagnostics_json,
        "handle-publish-diagnostics は deterministic diagnostics JSON を返すべき"
    );
    assert_eq!(
        parts[1], expected_frame,
        "lsp-render-publish-diagnostics-frame は notification frame を返すべき"
    );
    assert_eq!(
        parts[2].trim(),
        "1",
        "publishDiagnostics dispatch は request-count を 1 増やすべき"
    );
}

/// TEST-CLI-02-M11: didOpen dispatch + frame helper が deterministic に動くこと
#[test]
#[ignore]
fn test_e2e_selfhost_cli_lsp_transport_didopen_frame() {
    let payload =
        r#"{"jsonrpc":"2.0","method":"textDocument/didOpen","params":{"uri":42,"sourceBytes":16}}"#;
    let expected = format!("Content-Length: {}\r\n\r\n{}", payload.len(), payload);
    let harness = r#"
(defn main []
  (let [state (server-state-new)
        params (vector-push (vector-push (vector-new 2) 42) "(defn main [] 0)")
        result (json-rpc-dispatch (lsp-method-did-open) params state)]
    (do
      (print result)
      (print-string "\n---\n")
      (print-string (lsp-render-didopen-frame 42 result)))))
"#;

    let combined = format!("{}\n{}", selfhost_cli_runtime_bundle(), harness);
    let output = compile_and_run(&combined);
    let parts: Vec<&str> = output.split("\n---\n").collect();

    assert_eq!(
        parts.len(),
        2,
        "didOpen helper output format が不正: {:?}",
        output
    );
    assert_eq!(
        parts[0].trim(),
        "16",
        "didOpen dispatch は source length=16 を返すべき"
    );
    assert_eq!(
        parts[1], expected,
        "didOpen frame は deterministic であるべき"
    );
}

/// TEST-CLI-02-M12: didOpen -> didChange shared-state sequence が framed notifications と state summary を返すこと
#[test]
#[ignore]
fn test_e2e_selfhost_cli_lsp_transport_document_sequence() {
    let open_payload =
        r#"{"jsonrpc":"2.0","method":"textDocument/didOpen","params":{"uri":42,"sourceBytes":16}}"#;
    let open_frame = format!(
        "Content-Length: {}\r\n\r\n{}",
        open_payload.len(),
        open_payload
    );
    let change_payload = r#"{"jsonrpc":"2.0","method":"textDocument/didChange","params":{"uri":42,"sourceBytes":22}}"#;
    let change_frame = format!(
        "Content-Length: {}\r\n\r\n{}",
        change_payload.len(),
        change_payload
    );
    let harness = r#"
(defn main []
  (let [state (server-state-new)
        open-params (vector-push (vector-push (vector-new 2) 42) "(defn main [] 0)")
        change-params (vector-push (vector-push (vector-new 2) 42) "(defn main [] (+ 0 1))")
        open-result (json-rpc-dispatch (lsp-method-did-open) open-params state)
        change-result (json-rpc-dispatch (lsp-method-did-change) change-params state)]
    (do
      (print-string (lsp-render-didopen-frame 42 open-result))
      (print-string "\n---\n")
      (print-string (lsp-render-didchange-frame 42 change-result))
      (print-string "\n---\n")
      (print (server-state-doc-count state))
      (print (server-state-request-count state))
      (print (server-state-source-length state)))))
"#;

    let combined = format!("{}\n{}", selfhost_cli_runtime_bundle(), harness);
    let output = compile_and_run(&combined);
    let parts: Vec<&str> = output.split("\n---\n").collect();

    assert_eq!(
        parts.len(),
        3,
        "document sequence output format が不正: {:?}",
        output
    );
    assert_eq!(
        parts[0], open_frame,
        "frame0 は didOpen notification であるべき"
    );
    assert_eq!(
        parts[1], change_frame,
        "frame1 は didChange notification であるべき"
    );
    assert_eq!(
        parts[2].trim().lines().collect::<Vec<_>>(),
        vec!["1", "2", "22"],
        "sequence summary は doc-count=1 / request-count=2 / source-bytes=22 を返すべき"
    );
}

/// TEST-CLI-02-M12b: raw stdio frame helper が Content-Length header 付き initialize request を捌けること
#[test]
#[ignore]
fn test_e2e_selfhost_cli_lsp_stdio_frame_initialize() {
    let body = r#"{"jsonrpc":"2.0","id":14,"result":[1,1,1,1,1,1,1]}"#;
    let expected = format!("Content-Length: {}\r\n\r\n{}", body.len(), body);
    let header = format!("Content-Length: {}\r\n\r\n", body.len());
    let harness = format!(
        r#"
(defn main []
  (let [msg
          (vector-push
            (vector-push
              (vector-push
                (vector-push (vector-new 4) 2)
                14)
              (lsp-method-initialize))
            0)
        frame (vector-push (vector-push (vector-new 2) "{header}") msg)
        result (run-lsp-stdio-frame frame)]
    (do
      (print-string (vector-get result 0))
      (print-string "\n---\n")
      (print (vector-get result 1)))))
"#
    );

    let combined = format!("{}\n{}", selfhost_cli_runtime_bundle(), harness);
    let output = compile_and_run(&combined);
    let parts: Vec<&str> = output.split("\n---\n").collect();

    assert_eq!(
        parts.len(),
        2,
        "stdio frame output format が不正: {:?}",
        output
    );
    assert_eq!(
        parts[0], expected,
        "run-lsp-stdio-frame は initialize frame を返すべき"
    );
    assert_eq!(
        parts[1].trim(),
        body.len().to_string(),
        "run-lsp-stdio-frame は parsed Content-Length を返すべき"
    );
}

/// TEST-CLI-02-M12c: raw stdio frame sequence helper が shared-state で didOpen -> didChange を捌けること
#[test]
#[ignore]
fn test_e2e_selfhost_cli_lsp_stdio_frame_sequence() {
    let open_payload =
        r#"{"jsonrpc":"2.0","method":"textDocument/didOpen","params":{"uri":42,"sourceBytes":16}}"#;
    let open_frame = format!(
        "Content-Length: {}\r\n\r\n{}",
        open_payload.len(),
        open_payload
    );
    let change_payload = r#"{"jsonrpc":"2.0","method":"textDocument/didChange","params":{"uri":42,"sourceBytes":22}}"#;
    let change_frame = format!(
        "Content-Length: {}\r\n\r\n{}",
        change_payload.len(),
        change_payload
    );
    let open_header = format!("Content-Length: {}\r\n\r\n", open_payload.len());
    let change_header = format!("Content-Length: {}\r\n\r\n", change_payload.len());
    let harness = format!(
        r#"
(defn make-wire-msg [id method-id params]
  (vector-push
    (vector-push
      (vector-push
        (vector-push (vector-new 4) 2)
        id)
      method-id)
    params))

(defn make-wire-frame [header msg]
  (vector-push (vector-push (vector-new 2) header) msg))

(defn main []
  (let [open-params (vector-push (vector-push (vector-new 2) 42) "(defn main [] 0)")
        change-params (vector-push (vector-push (vector-new 2) 42) "(defn main [] (+ 0 1))")
        open-frame (make-wire-frame "{open_header}" (make-wire-msg 0 (lsp-method-did-open) open-params))
        change-frame (make-wire-frame "{change_header}" (make-wire-msg 0 (lsp-method-did-change) change-params))
        frames (vector-push (vector-push (vector-new 2) open-frame) change-frame)
        summary (run-lsp-stdio-sequence frames)
        rendered (vector-get summary 0)]
    (do
      (print-string (vector-get rendered 0))
      (print-string "\n---\n")
      (print-string (vector-get rendered 1))
      (print-string "\n---\n")
      (print (vector-get summary 1))
      (print (vector-get summary 2))
      (print (vector-get summary 3)))))
"#
    );

    let combined = format!("{}\n{}", selfhost_cli_runtime_bundle(), harness);
    let output = compile_and_run(&combined);
    let parts: Vec<&str> = output.split("\n---\n").collect();

    assert_eq!(
        parts.len(),
        3,
        "stdio frame sequence output format が不正: {:?}",
        output
    );
    assert_eq!(
        parts[0], open_frame,
        "frame0 は didOpen notification であるべき"
    );
    assert_eq!(
        parts[1], change_frame,
        "frame1 は didChange notification であるべき"
    );
    assert_eq!(
        parts[2].trim().lines().collect::<Vec<_>>(),
        vec!["2", "22", &change_payload.len().to_string()],
        "stdio frame sequence summary は request-count=2 / source-length=22 / last-content-length を返すべき"
    );
}

/// TEST-CLI-02-M12e: didOpen/didChange は parse diagnostics refresh を publishDiagnostics frame で返すこと
#[test]
#[ignore]
fn test_e2e_selfhost_cli_lsp_transport_document_sequence_publishes_diagnostics_refresh() {
    let open_payload =
        r#"{"jsonrpc":"2.0","method":"textDocument/didOpen","params":{"uri":42,"sourceBytes":1}}"#;
    let open_diagnostics = r#"{"jsonrpc":"2.0","method":"textDocument/publishDiagnostics","params":{"uri":42,"diagnostics":[{"source":1,"severity":1,"rule":1001,"line":1,"col":1,"messageHash":0}]}}"#;
    let change_payload = r#"{"jsonrpc":"2.0","method":"textDocument/didChange","params":{"uri":42,"sourceBytes":16}}"#;
    let change_diagnostics = r#"{"jsonrpc":"2.0","method":"textDocument/publishDiagnostics","params":{"uri":42,"diagnostics":[]}}"#;
    let open_frame = format!(
        "Content-Length: {}\r\n\r\n{}Content-Length: {}\r\n\r\n{}",
        open_payload.len(),
        open_payload,
        open_diagnostics.len(),
        open_diagnostics
    );
    let change_frame = format!(
        "Content-Length: {}\r\n\r\n{}Content-Length: {}\r\n\r\n{}",
        change_payload.len(),
        change_payload,
        change_diagnostics.len(),
        change_diagnostics
    );
    let harness = r#"
(defn make-request [id method-id params]
  (vector-push
    (vector-push
      (vector-push
        (vector-push (vector-new 4) 2)
        id)
      method-id)
    params))

(defn main []
  (let [state (server-state-new)
        open-params (vector-push (vector-push (vector-new 2) 42) ")")
        change-params (vector-push (vector-push (vector-new 2) 42) "(defn main [] 0)")
        open-frame (lsp-transport-dispatch-request state (make-request 0 (lsp-method-did-open) open-params))
        change-frame (lsp-transport-dispatch-request state (make-request 0 (lsp-method-did-change) change-params))]
    (do
      (print-string open-frame)
      (print-string "\n---\n")
      (print-string change-frame)
      (print-string "\n---\n")
      (print (server-state-request-count state))
      (print (server-state-source-length state)))))
"#;

    let combined = format!("{}\n{}", selfhost_cli_runtime_bundle(), harness);
    let output = compile_and_run(&combined);
    let parts: Vec<&str> = output.split("\n---\n").collect();

    assert_eq!(
        parts.len(),
        3,
        "transport diagnostics refresh output format が不正: {:?}",
        output
    );
    assert_eq!(
        parts[0], open_frame,
        "didOpen は parse diagnostics frame を後続させるべき"
    );
    assert_eq!(
        parts[1], change_frame,
        "didChange は diagnostics clear frame を後続させるべき"
    );
    assert_eq!(
        parts[2].trim().lines().collect::<Vec<_>>(),
        vec!["2", "16"],
        "transport diagnostics refresh summary は request-count=2 / latest-source-bytes=16 を返すべき"
    );
}

/// TEST-CLI-02-M12f: stdio body parser は spec 寄り didOpen/didChange params でも diagnostics refresh を返すこと
#[test]
#[ignore]
fn test_e2e_selfhost_cli_lsp_stdio_body_document_sequence_spec_params_publishes_diagnostics_refresh()
 {
    let open_body = r#"{"jsonrpc":"2.0","method":"textDocument/didOpen","params":{"textDocument":{"uri":42,"languageId":"lsharp","version":1,"text":")"}}}"#;
    let change_body = r#"{"jsonrpc":"2.0","method":"textDocument/didChange","params":{"textDocument":{"uri":42,"version":2},"contentChanges":[{"text":"(defn main [] 0)"}]}}"#;
    let open_payload =
        r#"{"jsonrpc":"2.0","method":"textDocument/didOpen","params":{"uri":42,"sourceBytes":1}}"#;
    let open_diagnostics = r#"{"jsonrpc":"2.0","method":"textDocument/publishDiagnostics","params":{"uri":42,"diagnostics":[{"source":1,"severity":1,"rule":1001,"line":1,"col":1,"messageHash":0}]}}"#;
    let change_payload = r#"{"jsonrpc":"2.0","method":"textDocument/didChange","params":{"uri":42,"sourceBytes":16}}"#;
    let change_diagnostics = r#"{"jsonrpc":"2.0","method":"textDocument/publishDiagnostics","params":{"uri":42,"diagnostics":[]}}"#;
    let open_frame = format!(
        "Content-Length: {}\r\n\r\n{}Content-Length: {}\r\n\r\n{}",
        open_payload.len(),
        open_payload,
        open_diagnostics.len(),
        open_diagnostics
    );
    let change_frame = format!(
        "Content-Length: {}\r\n\r\n{}Content-Length: {}\r\n\r\n{}",
        change_payload.len(),
        change_payload,
        change_diagnostics.len(),
        change_diagnostics
    );
    let open_body_lsharp = open_body.replace('\\', "\\\\").replace('"', "\\\"");
    let change_body_lsharp = change_body.replace('\\', "\\\\").replace('"', "\\\"");
    let harness = format!(
        r#"
(defn main []
  (let [state (server-state-new)
        open-body "{open_body_lsharp}"
        change-body "{change_body_lsharp}"
        open-frame (lsp-transport-dispatch-request state (lsp-stdio-message-request (lsp-stdio-body-message open-body)))
        change-frame (lsp-transport-dispatch-request state (lsp-stdio-message-request (lsp-stdio-body-message change-body)))]
    (do
      (print-string open-frame)
      (print-string "\n---\n")
      (print-string change-frame)
      (print-string "\n---\n")
      (print (server-state-request-count state))
      (print (server-state-source-length state)))))
"#
    );

    let combined = format!("{}\n{}", selfhost_cli_runtime_bundle(), harness);
    let output = compile_and_run(&combined);
    let parts: Vec<&str> = output.split("\n---\n").collect();

    assert_eq!(
        parts.len(),
        3,
        "spec document params diagnostics refresh output format が不正: {:?}",
        output
    );
    assert_eq!(
        parts[0], open_frame,
        "spec didOpen params でも parse diagnostics frame を後続させるべき"
    );
    assert_eq!(
        parts[1], change_frame,
        "spec didChange params でも diagnostics clear frame を後続させるべき"
    );
    assert_eq!(
        parts[2].trim().lines().collect::<Vec<_>>(),
        vec!["2", "16"],
        "spec document params summary は request-count=2 / latest-source-bytes=16 を返すべき"
    );
}

/// TEST-CLI-02-M12f2: didOpen/didChange は type diagnostics refresh を publishDiagnostics frame で返すこと
#[test]
#[ignore]
fn test_e2e_selfhost_cli_lsp_transport_document_sequence_publishes_type_diagnostics_refresh() {
    let open_payload =
        r#"{"jsonrpc":"2.0","method":"textDocument/didOpen","params":{"uri":42,"sourceBytes":26}}"#;
    let open_diagnostics = r#"{"jsonrpc":"2.0","method":"textDocument/publishDiagnostics","params":{"uri":42,"diagnostics":[{"source":2,"severity":1,"rule":2,"line":1,"col":1,"messageHash":2}]}}"#;
    let change_payload = r#"{"jsonrpc":"2.0","method":"textDocument/didChange","params":{"uri":42,"sourceBytes":16}}"#;
    let change_diagnostics = r#"{"jsonrpc":"2.0","method":"textDocument/publishDiagnostics","params":{"uri":42,"diagnostics":[]}}"#;
    let open_frame = format!(
        "Content-Length: {}\r\n\r\n{}Content-Length: {}\r\n\r\n{}",
        open_payload.len(),
        open_payload,
        open_diagnostics.len(),
        open_diagnostics
    );
    let change_frame = format!(
        "Content-Length: {}\r\n\r\n{}Content-Length: {}\r\n\r\n{}",
        change_payload.len(),
        change_payload,
        change_diagnostics.len(),
        change_diagnostics
    );
    let harness = r#"
(defn make-request [id method-id params]
  (vector-push
    (vector-push
      (vector-push
        (vector-push (vector-new 4) 2)
        id)
      method-id)
    params))

(defn main []
  (let [state (server-state-new)
        open-params (vector-push (vector-push (vector-new 2) 42) "(defn main [] (if 42 1 0))")
        change-params (vector-push (vector-push (vector-new 2) 42) "(defn main [] 0)")
        open-frame (lsp-transport-dispatch-request state (make-request 0 (lsp-method-did-open) open-params))
        change-frame (lsp-transport-dispatch-request state (make-request 0 (lsp-method-did-change) change-params))]
    (do
      (print-string open-frame)
      (print-string "\n---\n")
      (print-string change-frame)
      (print-string "\n---\n")
      (print (server-state-request-count state))
      (print (server-state-source-length state)))))
"#;

    let combined = format!("{}\n{}", selfhost_cli_runtime_bundle(), harness);
    let output = compile_and_run(&combined);
    let parts: Vec<&str> = output.split("\n---\n").collect();

    assert_eq!(
        parts.len(),
        3,
        "transport type diagnostics refresh output format が不正: {:?}",
        output
    );
    assert_eq!(
        parts[0], open_frame,
        "didOpen は type diagnostics frame を後続させるべき"
    );
    assert_eq!(
        parts[1], change_frame,
        "didChange は type diagnostics clear frame を後続させるべき"
    );
    assert_eq!(
        parts[2].trim().lines().collect::<Vec<_>>(),
        vec!["2", "16"],
        "transport type diagnostics refresh summary は request-count=2 / latest-source-bytes=16 を返すべき"
    );
}

/// TEST-CLI-02-M12f3: didOpen/didChange は lint diagnostics refresh を publishDiagnostics frame で返すこと
#[test]
#[ignore]
fn test_e2e_selfhost_cli_lsp_transport_document_sequence_publishes_lint_diagnostics_refresh() {
    let open_payload =
        r#"{"jsonrpc":"2.0","method":"textDocument/didOpen","params":{"uri":42,"sourceBytes":29}}"#;
    let open_diagnostics = r#"{"jsonrpc":"2.0","method":"textDocument/publishDiagnostics","params":{"uri":42,"diagnostics":[{"source":3,"severity":2,"rule":100,"line":1,"col":1,"messageHash":100}]}}"#;
    let change_payload = r#"{"jsonrpc":"2.0","method":"textDocument/didChange","params":{"uri":42,"sourceBytes":16}}"#;
    let change_diagnostics = r#"{"jsonrpc":"2.0","method":"textDocument/publishDiagnostics","params":{"uri":42,"diagnostics":[]}}"#;
    let open_frame = format!(
        "Content-Length: {}\r\n\r\n{}Content-Length: {}\r\n\r\n{}",
        open_payload.len(),
        open_payload,
        open_diagnostics.len(),
        open_diagnostics
    );
    let change_frame = format!(
        "Content-Length: {}\r\n\r\n{}Content-Length: {}\r\n\r\n{}",
        change_payload.len(),
        change_payload,
        change_diagnostics.len(),
        change_diagnostics
    );
    let harness = r#"
(defn make-request [id method-id params]
  (vector-push
    (vector-push
      (vector-push
        (vector-push (vector-new 4) 2)
        id)
      method-id)
    params))

(defn main []
  (let [state (server-state-new)
        open-params (vector-push (vector-push (vector-new 2) 42) "(defn main [] (let [x 42] 0))")
        change-params (vector-push (vector-push (vector-new 2) 42) "(defn main [] 0)")
        open-frame (lsp-transport-dispatch-request state (make-request 0 (lsp-method-did-open) open-params))
        change-frame (lsp-transport-dispatch-request state (make-request 0 (lsp-method-did-change) change-params))]
    (do
      (print-string open-frame)
      (print-string "\n---\n")
      (print-string change-frame)
      (print-string "\n---\n")
      (print (server-state-request-count state))
      (print (server-state-source-length state)))))
"#;

    let combined = format!("{}\n{}", selfhost_cli_runtime_bundle(), harness);
    let output = compile_and_run(&combined);
    let parts: Vec<&str> = output.split("\n---\n").collect();

    assert_eq!(
        parts.len(),
        3,
        "transport lint diagnostics refresh output format が不正: {:?}",
        output
    );
    assert_eq!(
        parts[0], open_frame,
        "didOpen は lint diagnostics frame を後続させるべき"
    );
    assert_eq!(
        parts[1], change_frame,
        "didChange は lint diagnostics clear frame を後続させるべき"
    );
    assert_eq!(
        parts[2].trim().lines().collect::<Vec<_>>(),
        vec!["2", "16"],
        "transport lint diagnostics refresh summary は request-count=2 / latest-source-bytes=16 を返すべき"
    );
}

/// TEST-CLI-02-M12f4: stdio body parser は spec 寄り didOpen/didChange params でも type diagnostics refresh を返すこと
#[test]
#[ignore]
fn test_e2e_selfhost_cli_lsp_stdio_body_document_sequence_spec_params_publishes_type_diagnostics_refresh()
 {
    let open_body = r#"{"jsonrpc":"2.0","method":"textDocument/didOpen","params":{"textDocument":{"uri":42,"languageId":"lsharp","version":1,"text":"(defn main [] (if 42 1 0))"}}}"#;
    let change_body = r#"{"jsonrpc":"2.0","method":"textDocument/didChange","params":{"textDocument":{"uri":42,"version":2},"contentChanges":[{"text":"(defn main [] 0)"}]}}"#;
    let open_payload =
        r#"{"jsonrpc":"2.0","method":"textDocument/didOpen","params":{"uri":42,"sourceBytes":26}}"#;
    let open_diagnostics = r#"{"jsonrpc":"2.0","method":"textDocument/publishDiagnostics","params":{"uri":42,"diagnostics":[{"source":2,"severity":1,"rule":2,"line":1,"col":1,"messageHash":2}]}}"#;
    let change_payload = r#"{"jsonrpc":"2.0","method":"textDocument/didChange","params":{"uri":42,"sourceBytes":16}}"#;
    let change_diagnostics = r#"{"jsonrpc":"2.0","method":"textDocument/publishDiagnostics","params":{"uri":42,"diagnostics":[]}}"#;
    let open_frame = format!(
        "Content-Length: {}\r\n\r\n{}Content-Length: {}\r\n\r\n{}",
        open_payload.len(),
        open_payload,
        open_diagnostics.len(),
        open_diagnostics
    );
    let change_frame = format!(
        "Content-Length: {}\r\n\r\n{}Content-Length: {}\r\n\r\n{}",
        change_payload.len(),
        change_payload,
        change_diagnostics.len(),
        change_diagnostics
    );
    let open_body_lsharp = open_body.replace('\\', "\\\\").replace('"', "\\\"");
    let change_body_lsharp = change_body.replace('\\', "\\\\").replace('"', "\\\"");
    let harness = format!(
        r#"
(defn main []
  (let [state (server-state-new)
        open-body "{open_body_lsharp}"
        change-body "{change_body_lsharp}"
        open-frame (lsp-transport-dispatch-request state (lsp-stdio-message-request (lsp-stdio-body-message open-body)))
        change-frame (lsp-transport-dispatch-request state (lsp-stdio-message-request (lsp-stdio-body-message change-body)))]
    (do
      (print-string open-frame)
      (print-string "\n---\n")
      (print-string change-frame)
      (print-string "\n---\n")
      (print (server-state-request-count state))
      (print (server-state-source-length state)))))
"#
    );

    let combined = format!("{}\n{}", selfhost_cli_runtime_bundle(), harness);
    let output = compile_and_run(&combined);
    let parts: Vec<&str> = output.split("\n---\n").collect();

    assert_eq!(
        parts.len(),
        3,
        "spec document params type diagnostics refresh output format が不正: {:?}",
        output
    );
    assert_eq!(
        parts[0], open_frame,
        "spec didOpen params でも type diagnostics frame を後続させるべき"
    );
    assert_eq!(
        parts[1], change_frame,
        "spec didChange params でも type diagnostics clear frame を後続させるべき"
    );
    assert_eq!(
        parts[2].trim().lines().collect::<Vec<_>>(),
        vec!["2", "16"],
        "spec document params type summary は request-count=2 / latest-source-bytes=16 を返すべき"
    );
}

/// TEST-CLI-02-M12f5: stdio body parser は spec 寄り didOpen/didChange params でも lint diagnostics refresh を返すこと
#[test]
#[ignore]
fn test_e2e_selfhost_cli_lsp_stdio_body_document_sequence_spec_params_publishes_lint_diagnostics_refresh()
 {
    let open_body = r#"{"jsonrpc":"2.0","method":"textDocument/didOpen","params":{"textDocument":{"uri":42,"languageId":"lsharp","version":1,"text":"(defn main [] (let [x 42] 0))"}}}"#;
    let change_body = r#"{"jsonrpc":"2.0","method":"textDocument/didChange","params":{"textDocument":{"uri":42,"version":2},"contentChanges":[{"text":"(defn main [] 0)"}]}}"#;
    let open_payload =
        r#"{"jsonrpc":"2.0","method":"textDocument/didOpen","params":{"uri":42,"sourceBytes":29}}"#;
    let open_diagnostics = r#"{"jsonrpc":"2.0","method":"textDocument/publishDiagnostics","params":{"uri":42,"diagnostics":[{"source":3,"severity":2,"rule":100,"line":1,"col":1,"messageHash":100}]}}"#;
    let change_payload = r#"{"jsonrpc":"2.0","method":"textDocument/didChange","params":{"uri":42,"sourceBytes":16}}"#;
    let change_diagnostics = r#"{"jsonrpc":"2.0","method":"textDocument/publishDiagnostics","params":{"uri":42,"diagnostics":[]}}"#;
    let open_frame = format!(
        "Content-Length: {}\r\n\r\n{}Content-Length: {}\r\n\r\n{}",
        open_payload.len(),
        open_payload,
        open_diagnostics.len(),
        open_diagnostics
    );
    let change_frame = format!(
        "Content-Length: {}\r\n\r\n{}Content-Length: {}\r\n\r\n{}",
        change_payload.len(),
        change_payload,
        change_diagnostics.len(),
        change_diagnostics
    );
    let open_body_lsharp = open_body.replace('\\', "\\\\").replace('"', "\\\"");
    let change_body_lsharp = change_body.replace('\\', "\\\\").replace('"', "\\\"");
    let harness = format!(
        r#"
(defn main []
  (let [state (server-state-new)
        open-body "{open_body_lsharp}"
        change-body "{change_body_lsharp}"
        open-frame (lsp-transport-dispatch-request state (lsp-stdio-message-request (lsp-stdio-body-message open-body)))
        change-frame (lsp-transport-dispatch-request state (lsp-stdio-message-request (lsp-stdio-body-message change-body)))]
    (do
      (print-string open-frame)
      (print-string "\n---\n")
      (print-string change-frame)
      (print-string "\n---\n")
      (print (server-state-request-count state))
      (print (server-state-source-length state)))))
"#
    );

    let combined = format!("{}\n{}", selfhost_cli_runtime_bundle(), harness);
    let output = compile_and_run(&combined);
    let parts: Vec<&str> = output.split("\n---\n").collect();

    assert_eq!(
        parts.len(),
        3,
        "spec document params lint diagnostics refresh output format が不正: {:?}",
        output
    );
    assert_eq!(
        parts[0], open_frame,
        "spec didOpen params でも lint diagnostics frame を後続させるべき"
    );
    assert_eq!(
        parts[1], change_frame,
        "spec didChange params でも lint diagnostics clear frame を後続させるべき"
    );
    assert_eq!(
        parts[2].trim().lines().collect::<Vec<_>>(),
        vec!["2", "16"],
        "spec document params lint summary は request-count=2 / latest-source-bytes=16 を返すべき"
    );
}

/// TEST-CLI-02-M12g: stdio body parser は spec 寄り hover params の position.character を col として読むこと
#[test]
#[ignore]
fn test_e2e_selfhost_cli_lsp_stdio_body_hover_spec_position_character_params() {
    let hover_body = r#"{"jsonrpc":"2.0","id":66,"method":"textDocument/hover","params":{"textDocument":{"uri":42},"position":{"line":1,"character":38}}}"#;
    let hover_body_lsharp = hover_body.replace('\\', "\\\\").replace('"', "\\\"");
    let harness = format!(
        r#"
(defn main []
  (let [msg (lsp-stdio-body-message "{hover_body_lsharp}")
        params (vector-get msg 3)]
    (do
      (print (vector-get params 0))
      (print (vector-get params 1))
      (print (vector-get params 2)))))
"#
    );

    let combined = format!("{}\n{}", selfhost_cli_runtime_bundle(), harness);
    let output = compile_and_run(&combined);
    let lines: Vec<&str> = output.trim().lines().collect();

    assert_eq!(
        lines,
        vec!["42", "1", "38"],
        "spec hover params は [uri,line,col]=[42,1,38] として読まれるべき: {:?}",
        lines
    );
}

/// TEST-CLI-02-M12h: stdio body parser は spec 寄り rename params の position.character と newName を読むこと
#[test]
#[ignore]
fn test_e2e_selfhost_cli_lsp_stdio_body_rename_spec_position_character_params() {
    let rename_body = r#"{"jsonrpc":"2.0","id":70,"method":"textDocument/rename","params":{"textDocument":{"uri":42},"position":{"line":1,"character":38},"newName":"cube"}}"#;
    let rename_body_lsharp = rename_body.replace('\\', "\\\\").replace('"', "\\\"");
    let harness = format!(
        r#"
(defn main []
  (let [msg (lsp-stdio-body-message "{rename_body_lsharp}")
        params (vector-get msg 3)]
    (do
      (print (vector-get params 0))
      (print (vector-get params 1))
      (print (vector-get params 2))
      (print-string (vector-get params 4))
      (print-string "\n"))))
"#
    );

    let combined = format!("{}\n{}", selfhost_cli_runtime_bundle(), harness);
    let output = compile_and_run(&combined);
    let lines: Vec<&str> = output.trim().lines().collect();

    assert_eq!(
        lines,
        vec!["42", "1", "38", "cube"],
        "spec rename params は [uri,line,col,newName]=[42,1,38,cube] として読まれるべき: {:?}",
        lines
    );
}

/// TEST-CLI-02-M12d: raw stdio wire helper が長めの open/hover/change/completion/formatting 系列を最後まで捌けること
#[test]
#[ignore]
fn test_e2e_selfhost_cli_lsp_stdio_wire_repeated_sequence() {
    let render_lsp_wire_frame =
        |body: &str| format!("Content-Length: {}\r\n\r\n{}", body.len(), body);
    let repeat_rendered_frames = |frames: &[String], iterations: usize| {
        let mut rendered = String::new();
        for _ in 0..iterations {
            for frame in frames {
                rendered.push_str(frame);
            }
        }
        rendered
    };

    let open_source = "(defn helper [] 1)\n(defn main [] (helper 1))";
    let change_source = "(defn helper [] 1)\n(defn main []  (he))";
    let iterations = 12usize;

    let init_body = r#"{"jsonrpc":"2.0","id":80,"method":"initialize","params":0}"#;
    let open_body = format!(
        r#"{{"jsonrpc":"2.0","method":"textDocument/didOpen","params":{{"uri":42,"source":"{}"}}}}"#,
        open_source
    );
    let hover_body = r#"{"jsonrpc":"2.0","id":81,"method":"textDocument/hover","params":{"uri":42,"line":1,"col":21}}"#;
    let change_body = format!(
        r#"{{"jsonrpc":"2.0","method":"textDocument/didChange","params":{{"uri":42,"source":"{}"}}}}"#,
        change_source
    );
    let completion_body = r#"{"jsonrpc":"2.0","id":82,"method":"textDocument/completion","params":{"uri":42,"line":1,"col":23}}"#;
    let formatting_body =
        r#"{"jsonrpc":"2.0","id":83,"method":"textDocument/formatting","params":{"uri":42}}"#;

    let stdin = format!(
        "{}{}",
        render_lsp_wire_frame(init_body),
        repeat_rendered_frames(
            &[
                render_lsp_wire_frame(&open_body),
                render_lsp_wire_frame(hover_body),
                render_lsp_wire_frame(&change_body),
                render_lsp_wire_frame(completion_body),
                render_lsp_wire_frame(formatting_body),
            ],
            iterations
        )
    );

    let harness = format!(
        r#"
(defn main []
  (let [wire {stdin:?}]
    (print-string (run-lsp-stdio-wire wire))))
"#
    );

    let combined = format!("{}\n{}", selfhost_cli_runtime_bundle(), harness);
    let output = compile_and_run(&combined);
    let frames = parse_lsp_stdio_frames(&output);
    let init_response = serde_json::json!({
        "jsonrpc": "2.0",
        "id": 80,
        "result": [1, 1, 1, 1, 1, 1, 1]
    });
    let open_response = serde_json::json!({
        "jsonrpc": "2.0",
        "method": "textDocument/didOpen",
        "params": {
            "uri": 42,
            "sourceBytes": open_source.len()
        }
    });
    let change_response = serde_json::json!({
        "jsonrpc": "2.0",
        "method": "textDocument/didChange",
        "params": {
            "uri": 42,
            "sourceBytes": change_source.len()
        }
    });
    let first_open_diagnostics = frames
        .get(2)
        .cloned()
        .expect("1 回目 didOpen diagnostics frame が必要");
    let first_hover_response = frames.get(3).cloned().expect("1 回目 hover frame が必要");
    let first_change_diagnostics = frames
        .get(5)
        .cloned()
        .expect("1 回目 didChange diagnostics frame が必要");
    let first_completion_response = frames
        .get(6)
        .cloned()
        .expect("1 回目 completion frame が必要");
    let first_formatting_response = frames
        .get(7)
        .cloned()
        .expect("1 回目 formatting frame が必要");

    assert_eq!(
        frames.len(),
        1 + (iterations * 7),
        "raw stdio wire helper は initialize + 各反復 7 frame を返すべき"
    );
    assert_eq!(
        frames[0], init_response,
        "frame0 は initialize response であるべき"
    );

    assert_eq!(
        first_open_diagnostics["method"],
        serde_json::json!("textDocument/publishDiagnostics"),
        "didOpen 後は publishDiagnostics frame を返すべき"
    );
    assert_eq!(
        first_open_diagnostics["params"]["uri"],
        serde_json::json!(42),
        "didOpen diagnostics は uri=42 を対象にすべき"
    );
    assert!(
        first_open_diagnostics["params"]["diagnostics"].is_array(),
        "didOpen diagnostics は配列であるべき"
    );
    assert_eq!(
        first_change_diagnostics["method"],
        serde_json::json!("textDocument/publishDiagnostics"),
        "didChange 後は publishDiagnostics frame を返すべき"
    );
    assert_eq!(
        first_change_diagnostics["params"]["uri"],
        serde_json::json!(42),
        "didChange diagnostics は uri=42 を対象にすべき"
    );
    assert!(
        first_change_diagnostics["params"]["diagnostics"].is_array(),
        "didChange diagnostics は配列であるべき"
    );
    assert_eq!(
        first_hover_response["id"],
        serde_json::json!(81),
        "hover frame は id=81 を保持すべき"
    );
    assert!(
        first_hover_response["result"].is_object(),
        "hover frame は result object を返すべき"
    );
    assert_eq!(
        first_completion_response["id"],
        serde_json::json!(82),
        "completion frame は id=82 を保持すべき"
    );
    assert!(
        first_completion_response["result"].is_array(),
        "completion frame は result array を返すべき"
    );
    assert_eq!(
        first_formatting_response["id"],
        serde_json::json!(83),
        "formatting frame は id=83 を保持すべき"
    );
    assert!(
        first_formatting_response["result"].is_array(),
        "formatting frame は result array を返すべき"
    );

    for iteration in 0..iterations {
        let base = 1 + (iteration * 7);
        assert_eq!(
            frames[base], open_response,
            "iteration {} の didOpen response が不正",
            iteration
        );
        assert_eq!(
            frames[base + 1],
            first_open_diagnostics,
            "iteration {} の didOpen diagnostics は決定的であるべき",
            iteration
        );
        assert_eq!(
            frames[base + 2],
            first_hover_response,
            "iteration {} の hover response が不正",
            iteration
        );
        assert_eq!(
            frames[base + 3],
            change_response,
            "iteration {} の didChange response が不正",
            iteration
        );
        assert_eq!(
            frames[base + 4],
            first_change_diagnostics,
            "iteration {} の didChange diagnostics は決定的であるべき",
            iteration
        );
        assert_eq!(
            frames[base + 5],
            first_completion_response,
            "iteration {} の completion response が不正",
            iteration
        );
        assert_eq!(
            frames[base + 6],
            first_formatting_response,
            "iteration {} の formatting response が不正",
            iteration
        );
    }
}

/// TEST-CLI-02-N: selfhost/src/App/Cli.ls の run-test-source が TestRunner.generate-tests を呼べること
#[test]
#[ignore]
fn test_e2e_selfhost_cli_test_source_core() {
    let harness = r#"
(defn main []
  (do
    (print (run-test-source "(defn main [] 42)" 0))
    0))
"#;

    let combined = format!("{}\n{}", selfhost_cli_runtime_bundle(), harness);
    let output = compile_and_run(&combined);
    let lines: Vec<&str> = output.trim().lines().collect();

    assert_eq!(
        lines,
        vec!["examples:0", "invariants:0", "failures:0", "0"],
        "run-test-source は labeled summary と success=0 を返すべき"
    );
}

/// TEST-CLI-02-O: selfhost/src/App/Cli.ls の run-test が file-path から source を読めること
#[test]
#[ignore]
fn test_e2e_selfhost_cli_test_file_handler() {
    let dir =
        std::env::temp_dir().join(format!("lsharp_test_cli_test_file_{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    std::fs::write(dir.join("input.ls"), "(defn main [] 42)").unwrap();

    let harness = r#"
(defn main []
  (do
    (print (run-test "input.ls" 0))
    0))
"#;

    let combined = format!("{}\n{}", selfhost_cli_runtime_bundle(), harness);
    let output = compile_and_run_with_dir(&combined, &dir);
    let _ = std::fs::remove_dir_all(&dir);
    let lines: Vec<&str> = output.trim().lines().collect();

    assert_eq!(
        lines,
        vec!["examples:0", "invariants:0", "failures:0", "0"],
        "run-test は labeled summary と success=0 を返すべき"
    );
}

/// TEST-CLI-02-O2: selfhost/src/Tools/Test/TestRunner.ls が supported subset の metadata suite を実行できること
#[test]
#[ignore]
fn test_e2e_selfhost_test_runner_extracts_supported_metadata_suite() {
    let harness = r#"
(defn main []
  (let [src "(defn abs [x] :example [(= (abs 5) 5) (= (abs (- 0 7)) 7)] :invariant (>= result 0) (if (< x 0) (- 0 x) x))"
        examples (extract-examples src)
        invariants (extract-invariants src)]
    (do
      (print (vector-length examples))
      (print (vector-length invariants))
      0)))
"#;

    let combined = format!("{}\n{}", selfhost_cli_runtime_bundle(), harness);
    let output = compile_and_run(&combined);
    let lines: Vec<&str> = output.trim().lines().collect();

    assert_eq!(
        lines,
        vec!["2", "1"],
        "extract-examples / extract-invariants は supported metadata を 2 / 1 件抽出できるべき"
    );
}

/// EC-M1-02: selfhost runner が parser の invariant AST を test case へ直接投影すること
#[test]
fn test_e2e_selfhost_test_runner_extracts_invariant_from_parser_ast() {
    let harness = r#"
(defn main []
  (let [src "(defn succ [x] :invariant (= result (+ x 1)) (+ x 1))"
        program (parse-program src)
        invariants (extract-invariants-from-program program)
        test-case (vector-get invariants 0)
        predicate (vector-get test-case 2)
        suite (generate-tests src)
        invariant-results (vector-get suite 1)
        invariant-result (vector-get invariant-results 0)]
    (do
      (print (vector-length invariants))
      (print (if (= (vector-get predicate 0) (ast-apply)) 1 0))
      (print (if (= (vector-get (vector-get predicate 1) 1) (hash-eq)) 1 0))
      (print (vector-length invariant-results))
      (print (vector-get invariant-result 1))
      0)))
"#;

    let combined = format!("{}\n{}", selfhost_cli_runtime_bundle(), harness);
    let output = run_with_expanded_stack(NATIVE_HARNESS_STACK_BYTES, move || {
        compile_and_run(&combined)
    });
    let lines: Vec<&str> = output.trim().lines().collect();

    assert_eq!(
        lines,
        vec!["1", "1", "1", "1", "1"],
        "invariant は parser AST から抽出され、generate-tests で実行されるべき"
    );
}

/// EC-M1-02: selfhost runner が parser の defn metadata から example を投影すること
#[test]
fn test_e2e_selfhost_test_runner_projects_examples_from_parser_metadata() {
    let harness = r#"
(defn main []
  (let [src "(defn succ [x] :example [(= (succ 1) 2) (= (succ 5) 6)] (+ x 1))"
        program (parse-program src)
        decl (vector-get program 0)
        meta (vector-get decl 5)
        examples (extract-examples-from-program program)
        suite (generate-tests src)
        results (vector-get suite 0)
        result0 (vector-get results 0)
        result1 (vector-get results 1)]
    (do
      (print (if (> (string-length (vector-get meta 1)) 0) 1 0))
      (print (vector-length examples))
      (print (vector-length results))
      (print (vector-get result0 1))
      (print (vector-get result1 1))
      0)))
"#;

    let combined = format!("{}\n{}", selfhost_cli_runtime_bundle(), harness);
    let output = run_with_expanded_stack(NATIVE_HARNESS_STACK_BYTES, move || {
        compile_and_run(&combined)
    });
    let lines: Vec<&str> = output.trim().lines().collect();

    assert_eq!(
        lines,
        vec!["1", "2", "2", "1", "1"],
        "example は parser metadata から投影され、generate-tests で 2 件とも成功するべき"
    );
}

/// EC-M1-02: selfhost parser metadata が複数 directive と typed defn で保持されること
#[test]
fn test_e2e_selfhost_test_runner_preserves_example_metadata_across_defn_shapes() {
    let harness = r#"
(defn main []
  (let [multi-src "(defn succ [x] :example [(= (succ 1) 2)] :example [(= (succ 5) 6)] (+ x 1))"
        multi-program (parse-program multi-src)
        multi-examples (extract-examples-from-program multi-program)
        typed-src "(defn typed-succ [(: x Int)] : Int :example [(= (typed-succ 1) 2)] (+ x 1))"
        typed-program (parse-program typed-src)
        typed-examples (extract-examples-from-program typed-program)
        typed-suite (generate-tests typed-src)
        typed-results (vector-get typed-suite 0)
        typed-result (vector-get typed-results 0)]
    (do
      (print (vector-length multi-examples))
      (print (vector-length typed-examples))
      (print (vector-length typed-results))
      (print (vector-get typed-result 1))
      0)))
"#;

    let combined = format!("{}\n{}", selfhost_cli_runtime_bundle(), harness);
    let output = run_with_expanded_stack(NATIVE_HARNESS_STACK_BYTES, move || {
        compile_and_run(&combined)
    });
    let lines: Vec<&str> = output.trim().lines().collect();

    assert_eq!(
        lines,
        vec!["2", "1", "1", "1"],
        "複数 example directive の順序と typed defn の metadata projection を維持するべき"
    );
}

/// EC-M1-02: selfhost runner が parser-owned ordered forms から example を投影すること
#[test]
fn test_e2e_selfhost_test_runner_projects_ordered_example_forms() {
    let harness = r#"
(defn main []
  (let [src "(defn succ [x] :example [(= (succ 1) 2)] :invariant (= result (+ x 1)) :example [(= (succ 5) 6)] (+ x 1))"
        program (parse-program src)
        decl (vector-get program 0)
        forms (test-defn-ordered-forms decl)
        examples (extract-examples-from-program program)
        suite (generate-tests src)
        results (vector-get suite 0)
        result0 (vector-get results 0)
        result1 (vector-get results 1)]
    (do
      (print (vector-length forms))
      (print (vector-get (vector-get forms 0) 0))
      (print (vector-get (vector-get forms 1) 0))
      (print (vector-get (vector-get forms 2) 0))
      (print (vector-length examples))
      (print (vector-length results))
      (print (vector-get result0 1))
      (print (vector-get result1 1))
      0)))
"#;

    let combined = format!("{}\n{}", selfhost_cli_runtime_bundle(), harness);
    let output = run_with_expanded_stack(NATIVE_HARNESS_STACK_BYTES, move || {
        compile_and_run(&combined)
    });
    let lines: Vec<&str> = output.trim().lines().collect();

    assert_eq!(
        lines,
        vec!["3", "1", "2", "1", "2", "2", "1", "1"],
        "runner は parser-owned ordered forms から example のみを順序どおり投影するべき"
    );
}

/// EC-M1-02: selfhost runner が複数の parser-owned invariant forms を保持すること
#[test]
fn test_e2e_selfhost_test_runner_projects_ordered_invariant_forms() {
    let harness = r#"
(defn main []
  (let [src "(defn succ [x] :invariant (>= result 0) :invariant (= result (+ x 1)) (+ x 1))"
        program (parse-program src)
        decl (vector-get program 0)
        forms (test-defn-ordered-forms decl)
        invariants (extract-invariants-from-program program)
        suite (generate-tests src)
        results (vector-get suite 1)
        result0 (vector-get results 0)
        result1 (vector-get results 1)]
    (do
      (print (vector-length forms))
      (print (vector-get (vector-get forms 0) 0))
      (print (vector-get (vector-get forms 1) 0))
      (print (vector-length invariants))
      (print (vector-length results))
      (print (vector-get result0 1))
      (print (vector-get result1 1))
      0)))
"#;

    let combined = format!("{}\n{}", selfhost_test_runner_runtime_bundle(), harness);
    let output = run_with_expanded_stack(NATIVE_HARNESS_STACK_BYTES, move || {
        compile_and_run(&combined)
    });
    let lines: Vec<&str> = output.trim().lines().collect();

    assert_eq!(
        lines,
        vec!["2", "2", "2", "2", "2", "1", "1"],
        "runner は複数の parser-owned invariant forms を順序どおり実行するべき"
    );
}

/// EC-M1-03: selfhost runner が canonical :assert を独立 bucket へ投影・実行すること
#[test]
fn test_e2e_selfhost_test_runner_projects_and_runs_ordered_assertion_forms() {
    let harness = r#"
(defn main []
  (let [src "(defn positive [] :assert [(> 1 0) (= 1 1)] true)"
        program (parse-program src)
        decl (vector-get program 0)
        forms (test-defn-ordered-forms decl)
        form (vector-get forms 0)
        suite (generate-tests src)
        assertions (vector-get suite 2)
        result0 (vector-get assertions 0)
        result1 (vector-get assertions 1)]
    (do
      (print (vector-length forms))
      (print (vector-get form 0))
      (print (vector-length (vector-get form 1)))
      (print (vector-length assertions))
      (print (vector-get result0 1))
      (print (vector-get result1 1))
      0)))
"#;

    let combined = format!("{}\n{}", selfhost_test_runner_runtime_bundle(), harness);
    let output = run_with_expanded_stack(NATIVE_HARNESS_STACK_BYTES, move || {
        compile_and_run(&combined)
    });
    let lines: Vec<&str> = output.trim().lines().collect();

    assert_eq!(
        lines,
        vec!["1", "3", "2", "2", "1", "1"],
        "canonical :assert は predicate の grouping と順序を保ったまま独立 bucket で実行されるべき"
    );
}

/// EC-M1-02: selfhost parser が canonical :case の expectation を ordered form へ保持すること
#[test]
fn test_e2e_selfhost_parser_preserves_ordered_case_forms() {
    let harness = r#"
(defn main []
  (let [src "(defn succ [x] :case [(expect (succ 1) 2) (expect (succ 2) 4)] (+ x 1))"
        program (parse-program src)
        decl (vector-get program 0)
        forms (test-defn-ordered-forms decl)
        form (vector-get forms 0)
        expectations (vector-get form 1)
        pair0 (vector-get expectations 0)
        pair1 (vector-get expectations 1)]
    (do
      (print (vector-length forms))
      (print (vector-get form 0))
      (print (vector-length expectations))
      (print (vector-get (vector-get pair0 0) 0))
      (print (vector-get (vector-get pair0 1) 0))
      (print (vector-get (vector-get pair1 0) 0))
      (print (vector-get (vector-get pair1 1) 0))
      0)))
"#;

    let combined = format!("{}\n{}", selfhost_test_runner_runtime_bundle(), harness);
    let output = run_with_expanded_stack(NATIVE_HARNESS_STACK_BYTES, move || {
        compile_and_run(&combined)
    });
    let lines: Vec<&str> = output.trim().lines().collect();

    assert_eq!(
        lines,
        vec!["1", "4", "2", "5", "1", "5", "1"],
        "selfhost parser は canonical :case を expectation 順に保持するべき"
    );
}

/// EC-M1-03: selfhost parser が canonical :property payload を bracket-aware に保持すること
#[test]
fn test_e2e_selfhost_parser_preserves_ordered_property_forms() {
    let harness = r#"
(defn main []
  (let [src "(defn abs [x] :property [(for-all [x Int] :cases 12 :seed 81042 :shrink false :precondition [(>= x -100)] :postcondition (>= result 0))] (if (< x 0) (- 0 x) x))"
        program (parse-program src)
        decl (vector-get program 0)
        forms (test-defn-ordered-forms decl)
        form (vector-get forms 0)
        payload (vector-get form 1)]
    (do
      (print (vector-length forms))
      (print (vector-get form 0))
      (print-string payload)
      (print-string "\n")
      0)))
"#;

    let combined = format!("{}\n{}", selfhost_test_runner_runtime_bundle(), harness);
    let output = run_with_expanded_stack(NATIVE_HARNESS_STACK_BYTES, move || {
        compile_and_run(&combined)
    });
    let lines: Vec<&str> = output.trim().lines().collect();

    assert_eq!(
        lines,
        vec![
            "1",
            "5",
            "(for-all [x Int] :cases 12 :seed 81042 :shrink false :precondition [(>= x -100)] :postcondition (>= result 0))",
        ],
        "selfhost parser は canonical :property payload の括弧構造と source order を保持するべき"
    );
}

/// EC-M1-02: selfhost parser-owned contract form が directive source span を保持すること
#[test]
fn test_e2e_selfhost_parser_contract_forms_keep_directive_spans() {
    let harness = r#"
(defn main []
  (let [src "(defn identity [x] :property [(for-all [x Int] :cases 1 :postcondition (= result x))] x)"
        program (parse-program src)
        decl (vector-get program 0)
        forms (test-defn-ordered-forms decl)
        form (vector-get forms 0)]
    (do
      (print (vector-length form))
      (print (if (and (> (vector-length form) 3) (= (vector-get form 2) 19)) 1 0))
      (print (if (and (> (vector-length form) 3) (< (vector-get form 2) (vector-get form 3))) 1 0))
      0)))
"#;

    let combined = format!("{}\n{}", selfhost_test_runner_runtime_bundle(), harness);
    let output = run_with_expanded_stack(NATIVE_HARNESS_STACK_BYTES, move || {
        compile_and_run(&combined)
    });
    let lines: Vec<&str> = output.trim().lines().collect();

    assert_eq!(
        lines,
        vec!["4", "1", "1"],
        "parser-owned contract form は directive の source span を payload と一緒に保持するべき"
    );
}

/// EC-M1-02: parser-owned ContractSuite が raw inventory と directive span を共有すること
#[test]
fn test_e2e_selfhost_parser_contract_suite_preserves_property_directive_span() {
    let harness = r#"
(defn main []
  (let [src "(defn identity [x] :property [(for-all [x Int] :cases 1 :postcondition (= result x))] x)"
        raw (vector-get (extract-contract-forms src) 0)
        suites (extract-parser-contract-suites src)
        ordered (vector-get (vector-get suites 0) 1)
        suite-form (vector-get ordered 0)]
    (do
      (print (if (and (> (vector-length raw) 4) (and (> (vector-length suite-form) 3) (= (vector-get raw 3) (vector-get suite-form 2)))) 1 0))
      (print (if (and (> (vector-length raw) 4) (and (> (vector-length suite-form) 3) (= (vector-get raw 4) (vector-get suite-form 3)))) 1 0))
      0)))
"#;

    let combined = format!("{}\n{}", selfhost_test_runner_runtime_bundle(), harness);
    let output = run_with_expanded_stack(NATIVE_HARNESS_STACK_BYTES, move || {
        compile_and_run(&combined)
    });
    let lines: Vec<&str> = output.trim().lines().collect();

    assert_eq!(
        lines,
        vec!["1", "1"],
        "parser-owned ContractSuite は raw inventory と同じ directive span を保持するべき"
    );
}

/// EC-M1-02: selfhost parser が deterministic property を typed contract shape へ投影すること
#[test]
fn test_e2e_selfhost_parser_projects_typed_property_sampling_contract() {
    let harness = r#"
(defn main []
  (let [program (parse-program "(defn identity [x] :property [(for-all [value Int] :cases 3 :postcondition (= result value))] x)")
        contracts (extract-parser-typed-property-contracts program)
        contract (vector-get contracts 0)
        binders (vector-get contract 1)
        binder (vector-get binders 0)
        preconditions (vector-get contract 2)
        postcondition (vector-get contract 3)
        sampling (vector-get contract 4)]
    (do
      (print (vector-length contracts))
      (print (if (= (vector-get contract 0) (name-hash "identity" 0 8)) 1 0))
      (print (vector-length binders))
      (print (if (= (vector-get binder 0) (name-hash "value" 0 5)) 1 0))
      (print (if (= (vector-get binder 1) (name-hash "Int" 0 3)) 1 0))
      (print (vector-get binder 2))
      (print (vector-length preconditions))
      (print (vector-get postcondition 0))
      (print (vector-get sampling 0))
      (print (vector-get sampling 1))
      (print-string (vector-get sampling 2))
      (print-string "\n")
      (print (vector-get sampling 3))
      (print (vector-get sampling 4))
      (print (vector-get contract 5))
      0)))
"#;

    let combined = format!("{}\n{}", selfhost_test_runner_runtime_bundle(), harness);
    let output = run_with_expanded_stack(NATIVE_HARNESS_STACK_BYTES, move || {
        compile_and_run(&combined)
    });
    let lines: Vec<&str> = output.trim().lines().collect();

    assert_eq!(
        lines,
        vec![
            "1", "1", "1", "1", "1", "1", "0", "5", "3", "0",
            "type-directed-splitmix64-v1", "1", "0", "0",
        ],
        "selfhost property は owner/binder/predicate/sampling の typed contract shape を Rust canonical IR に対応付けるべき"
    );
}

/// EC-M1-02: selfhost typed projection が未対応 sampling option を明示拒否すること
#[test]
fn test_e2e_selfhost_parser_keeps_typed_property_profile_boundary() {
    let harness = r#"
(defn main []
  (let [program (parse-program "(defn identity [x] :property [(for-all [value Int] :cases 3 :seed 42 :postcondition (= result value))] x)")
        contracts (extract-parser-typed-property-contracts program)
        contract (vector-get contracts 0)]
    (do
      (print (vector-length contracts))
      (print (vector-length (vector-get contract 1)))
      (print (vector-length (vector-get contract 4)))
      (print (vector-get contract 5))
      0)))
"#;

    let combined = format!("{}\n{}", selfhost_test_runner_runtime_bundle(), harness);
    let output = run_with_expanded_stack(NATIVE_HARNESS_STACK_BYTES, move || {
        compile_and_run(&combined)
    });
    let lines: Vec<&str> = output.trim().lines().collect();

    assert_eq!(
        lines,
        vec!["1", "0", "0", "3002"],
        "selfhost typed property projection は未対応 sampling option を default 値へ丸めず明示拒否するべき"
    );
}

/// EC-M1-03: selfhost metadata runner が未接続の canonical :property を検出すること
#[test]
fn test_e2e_selfhost_runner_reports_unimplemented_property_boundary() {
    let harness = r#"
(defn main []
  (do
    (print (metadata-test-runner-boundary-code (parse-program "(defn abs [x] :property [(for-all [x Int] :cases 12 :seed 81042 :shrink false :precondition [(>= x -100)] :postcondition (>= result 0))] (if (< x 0) (- 0 x) x))")))
    0))
"#;

    let combined = format!("{}\n{}", selfhost_test_runner_runtime_bundle(), harness);
    let output = run_with_expanded_stack(NATIVE_HARNESS_STACK_BYTES, move || {
        compile_and_run(&combined)
    });
    let lines: Vec<&str> = output.trim().lines().collect();

    assert_eq!(
        lines,
        vec!["3002"],
        "selfhost runner は未接続の property を LS3002 境界へ送るべき"
    );
}

/// EC-M1-02: selfhost runner が単一 precondition の false sample を skip して評価すること
#[test]
fn test_e2e_selfhost_runner_executes_property_precondition_and_skips_false_samples() {
    let harness = r#"
(defn main []
  (let [src "(defn identity [x] :property [(for-all [value Int] :cases 5 :precondition [(>= value 0)] :postcondition (= result value))] x)"
        suite (generate-tests-from-source src)
        properties (vector-get suite 4)
        result0 (vector-get properties 0)]
    (do
      (print (vector-length properties))
      (print (vector-get result0 1))
      (print (vector-get result0 2))
      (print (vector-get result0 3))
      0)))
"#;

    let combined = format!("{}\n{}", selfhost_test_runner_runtime_bundle(), harness);
    let output = run_with_expanded_stack(NATIVE_HARNESS_STACK_BYTES, move || {
        compile_and_run(&combined)
    });
    let lines: Vec<&str> = output.trim().lines().collect();

    assert_eq!(
        lines,
        vec!["1", "1", "4", "0"],
        "selfhost runner は precondition が false の sample を skip し、実行対象だけを評価すべき"
    );
}

/// EC-M1-02: selfhost runner が複数 precondition を conjunction として評価すること
#[test]
fn test_e2e_selfhost_runner_executes_all_property_preconditions_as_conjunction() {
    let harness = r#"
(defn main []
  (let [src "(defn identity [x] :property [(for-all [value Int] :cases 5 :precondition [(>= value 0) (< value 42)] :postcondition (= result value))] x)"
        suite (generate-tests-from-source src)
        properties (vector-get suite 4)
        result0 (vector-get properties 0)]
    (do
      (print (vector-length properties))
      (print (vector-get result0 1))
      (print (vector-get result0 2))
      (print (vector-get result0 3))
      0)))
"#;

    let combined = format!("{}\n{}", selfhost_test_runner_runtime_bundle(), harness);
    let output = run_with_expanded_stack(NATIVE_HARNESS_STACK_BYTES, move || {
        compile_and_run(&combined)
    });
    let lines: Vec<&str> = output.trim().lines().collect();

    assert_eq!(
        lines,
        vec!["1", "1", "3", "0"],
        "selfhost runner は複数 precondition を全て満たす sample だけ評価すべき"
    );
}

/// EC-M1-04: selfhost runner が全 sample skip の property を vacuous success にしないこと
#[test]
fn test_e2e_selfhost_runner_rejects_vacuous_property_precondition() {
    let harness = r#"
(defn main []
  (let [src "(defn identity [x] :property [(for-all [value Int] :cases 5 :precondition [(= value 999)] :postcondition (= result value))] x)"
        suite (generate-tests-from-source src)
        properties (vector-get suite 4)
        result0 (vector-get properties 0)]
    (do
      (print (vector-length properties))
      (print (vector-get result0 1))
      (print (vector-get result0 2))
      (print (vector-get result0 3))
      0)))
"#;

    let combined = format!("{}\n{}", selfhost_test_runner_runtime_bundle(), harness);
    let output = run_with_expanded_stack(NATIVE_HARNESS_STACK_BYTES, move || {
        compile_and_run(&combined)
    });
    let lines: Vec<&str> = output.trim().lines().collect();

    assert_eq!(
        lines,
        vec!["1", "0", "0", "2005"],
        "selfhost runner は全 sample skip の property を vacuous success にしてはならない"
    );
}

/// EC-M1-04: selfhost runner が literal true の property postcondition を成功扱いしないこと
#[test]
fn test_e2e_selfhost_runner_rejects_vacuous_property_postcondition() {
    let harness = r#"
(defn main []
  (let [src "(defn identity [x] :property [(for-all [value Int] :cases 1 :postcondition true)] x)"
        suite (generate-tests-from-source src)
        properties (vector-get suite 4)
        result0 (vector-get properties 0)]
    (do
      (print (vector-length properties))
      (print (vector-get result0 1))
      (print (vector-get result0 2))
      (print (vector-get result0 3))
      0)))
"#;

    let combined = format!("{}\n{}", selfhost_test_runner_runtime_bundle(), harness);
    let output = run_with_expanded_stack(NATIVE_HARNESS_STACK_BYTES, move || {
        compile_and_run(&combined)
    });
    let lines: Vec<&str> = output.trim().lines().collect();

    assert_eq!(
        lines,
        vec!["1", "0", "0", "2005"],
        "selfhost runner は literal true postcondition を vacuous success にしてはならない"
    );
}

/// EC-M1-04: selfhost runner が静的に true な integer comparison postcondition を成功扱いしないこと
#[test]
fn test_e2e_selfhost_runner_rejects_statically_true_property_postcondition() {
    let harness = r#"
(defn main []
  (let [src "(defn identity [x] :property [(for-all [value Int] :cases 1 :postcondition (= 1 1))] x)"
        suite (generate-tests-from-source src)
        properties (vector-get suite 4)
        result0 (vector-get properties 0)]
    (do
      (print (vector-length properties))
      (print (vector-get result0 1))
      (print (vector-get result0 2))
      (print (vector-get result0 3))
      0)))
"#;

    let combined = format!("{}\n{}", selfhost_test_runner_runtime_bundle(), harness);
    let output = run_with_expanded_stack(NATIVE_HARNESS_STACK_BYTES, move || {
        compile_and_run(&combined)
    });
    let lines: Vec<&str> = output.trim().lines().collect();

    assert_eq!(
        lines,
        vec!["1", "0", "0", "2005"],
        "selfhost runner は静的に true な integer comparison を vacuous success にしてはならない"
    );
}

/// EC-M1-02: selfhost runner が二つの Int binder を pair prefix として実行すること
#[test]
fn test_e2e_selfhost_runner_executes_two_int_property_binders() {
    let harness = r#"
(defn main []
  (let [src "(defn sum [left right] :property [(for-all [a Int b Int] :cases 5 :precondition [(< b 5)] :postcondition (= result (+ a b)))] (+ left right))"
        suite (generate-tests-from-source src)
        properties (vector-get suite 4)
        result0 (vector-get properties 0)]
    (do
      (print (vector-length properties))
      (print (vector-get result0 1))
      (print (vector-get result0 2))
      (print (vector-get result0 3))
      0)))
"#;

    let combined = format!("{}\n{}", selfhost_test_runner_runtime_bundle(), harness);
    let output = run_with_expanded_stack(NATIVE_HARNESS_STACK_BYTES, move || {
        compile_and_run(&combined)
    });
    let lines: Vec<&str> = output.trim().lines().collect();

    assert_eq!(
        lines,
        vec!["1", "1", "4", "0"],
        "selfhost runner は二つの Int binder を同じ pair prefix と precondition で評価すべき"
    );
}

/// EC-M1-05: selfhost runner が単一 Bool binder を false/true prefix として実行すること
#[test]
fn test_e2e_selfhost_runner_executes_bool_property_binder() {
    let harness = r#"
(defn main []
  (let [src "(defn identity [x] :property [(for-all [value Bool] :cases 2 :postcondition (or value (not value)))] x)"
        suite (generate-tests-from-source src)
        properties (vector-get suite 4)
        result0 (vector-get properties 0)]
    (do
      (print (vector-length properties))
      (print (vector-get result0 1))
      (print (vector-get result0 2))
      (print (vector-get result0 3))
      0)))
"#;

    let combined = format!("{}\n{}", selfhost_test_runner_runtime_bundle(), harness);
    let output = run_with_expanded_stack(NATIVE_HARNESS_STACK_BYTES, move || {
        compile_and_run(&combined)
    });
    let lines: Vec<&str> = output.trim().lines().collect();

    assert_eq!(
        lines,
        vec!["1", "1", "2", "0"],
        "selfhost runner は単一 Bool binder を false/true の 2 cases として評価すべき"
    );
}

/// EC-M1-05: selfhost runner が Int/Bool mixed binder を source-order prefix として実行すること
#[test]
fn test_e2e_selfhost_runner_executes_mixed_int_bool_property_binders() {
    let harness = r#"
(defn main []
  (let [src "(defn choose [input enabled] :property [(for-all [value Int flag Bool] :cases 2 :postcondition (and (>= value 0) (or flag (not flag))))] enabled)"
        suite (generate-tests-from-source src)
        properties (vector-get suite 4)
        result0 (vector-get properties 0)]
    (do
      (print (vector-length properties))
      (print (vector-get result0 1))
      (print (vector-get result0 2))
      (print (vector-get result0 3))
      0)))
"#;

    let combined = format!("{}\n{}", selfhost_test_runner_runtime_bundle(), harness);
    let output = run_with_expanded_stack(NATIVE_HARNESS_STACK_BYTES, move || {
        compile_and_run(&combined)
    });
    let lines: Vec<&str> = output.trim().lines().collect();

    assert_eq!(
        lines,
        vec!["1", "1", "2", "0"],
        "selfhost runner は Int/Bool mixed binder を [0,false]/[1,true] として評価すべき"
    );
}

/// EC-M1-05: selfhost runner が二つの Bool binder を source-order prefix として実行すること
#[test]
fn test_e2e_selfhost_runner_executes_two_bool_property_binders() {
    let harness = r#"
(defn main []
  (let [src "(defn choose [left right] :property [(for-all [a Bool b Bool] :cases 2 :postcondition (= result (if (or a b) 1 0)))] (if (or left right) 1 0))"
        suite (generate-tests-from-source src)
        properties (vector-get suite 4)
        result0 (vector-get properties 0)]
    (do
      (print (vector-length properties))
      (print (vector-get result0 1))
      (print (vector-get result0 2))
      (print (vector-get result0 3))
      0)))
"#;

    let combined = format!("{}\n{}", selfhost_test_runner_runtime_bundle(), harness);
    let output = run_with_expanded_stack(NATIVE_HARNESS_STACK_BYTES, move || {
        compile_and_run(&combined)
    });
    let lines: Vec<&str> = output.trim().lines().collect();

    assert_eq!(
        lines,
        vec!["1", "1", "2", "0"],
        "selfhost runner は二つの Bool binder を [false,false]/[true,true] として評価すべき"
    );
}

/// EC-M1-05: selfhost runner が三つの Bool binder を source-order prefix として実行すること
#[test]
fn test_e2e_selfhost_runner_executes_three_bool_property_binders() {
    let harness = r#"
(defn main []
  (let [src "(defn choose [left middle right] :property [(for-all [a Bool b Bool c Bool] :cases 2 :postcondition (= result (if (or a (or b c)) 1 0)))] (if (or left (or middle right)) 1 0))"
        suite (generate-tests-from-source src)
        properties (vector-get suite 4)
        result0 (vector-get properties 0)]
    (do
      (print (vector-length properties))
      (print (vector-get result0 1))
      (print (vector-get result0 2))
      (print (vector-get result0 3))
      0)))
"#;

    let combined = format!("{}\n{}", selfhost_test_runner_runtime_bundle(), harness);
    let output = run_with_expanded_stack(NATIVE_HARNESS_STACK_BYTES, move || {
        compile_and_run(&combined)
    });
    let lines: Vec<&str> = output.trim().lines().collect();

    assert_eq!(
        lines,
        vec!["1", "1", "2", "0"],
        "selfhost runner は三つの Bool binder を [false,false,false]/[true,true,true] として評価すべき"
    );
}

/// EC-M1-05: selfhost runner が三つの Bool binder の cases 上限を明示拒否すること
#[test]
fn test_e2e_selfhost_runner_rejects_three_bool_property_above_two_cases() {
    let harness = r#"
(defn main []
  (let [src "(defn choose [left middle right] :property [(for-all [a Bool b Bool c Bool] :cases 3 :postcondition (= result (if (or a (or b c)) 1 0)))] (if (or left (or middle right)) 1 0))"
        suite (generate-tests-from-source src)
        properties (vector-get suite 4)
        result0 (vector-get properties 0)]
    (do
      (print (vector-length properties))
      (print (vector-get result0 1))
      (print (vector-get result0 2))
      (print (vector-get result0 3))
      0)))
"#;

    let combined = format!("{}\n{}", selfhost_test_runner_runtime_bundle(), harness);
    let output = run_with_expanded_stack(NATIVE_HARNESS_STACK_BYTES, move || {
        compile_and_run(&combined)
    });
    let lines: Vec<&str> = output.trim().lines().collect();

    assert_eq!(
        lines,
        vec!["1", "0", "0", "3002"],
        "selfhost runner は三つの Bool binder の cases 3 を narrow profile 外として拒否すべき"
    );
}

/// EC-M1-05: selfhost runner が二つの Bool binder の cases 上限を明示拒否すること
#[test]
fn test_e2e_selfhost_runner_rejects_two_bool_property_above_two_cases() {
    let harness = r#"
(defn main []
  (let [src "(defn choose [left right] :property [(for-all [a Bool b Bool] :cases 3 :postcondition (= result (if (or a b) 1 0)))] (if (or left right) 1 0))"
        suite (generate-tests-from-source src)
        properties (vector-get suite 4)
        result0 (vector-get properties 0)]
    (do
      (print (vector-length properties))
      (print (vector-get result0 1))
      (print (vector-get result0 2))
      (print (vector-get result0 3))
      0)))
"#;

    let combined = format!("{}\n{}", selfhost_test_runner_runtime_bundle(), harness);
    let output = run_with_expanded_stack(NATIVE_HARNESS_STACK_BYTES, move || {
        compile_and_run(&combined)
    });
    let lines: Vec<&str> = output.trim().lines().collect();

    assert_eq!(
        lines,
        vec!["1", "0", "0", "3002"],
        "selfhost runner は二つの Bool binder の cases 3 を narrow profile 外として拒否すべき"
    );
}

/// EC-M1-05: selfhost runner が mixed binder の cases 上限を明示拒否すること
#[test]
fn test_e2e_selfhost_runner_rejects_mixed_int_bool_property_above_two_cases() {
    let harness = r#"
(defn main []
  (let [src "(defn choose [input enabled] :property [(for-all [value Int flag Bool] :cases 3 :postcondition (and (>= value 0) (or flag (not flag))))] enabled)"
        suite (generate-tests-from-source src)
        properties (vector-get suite 4)
        result0 (vector-get properties 0)]
    (do
      (print (vector-length properties))
      (print (vector-get result0 1))
      (print (vector-get result0 2))
      (print (vector-get result0 3))
      0)))
"#;

    let combined = format!("{}\n{}", selfhost_test_runner_runtime_bundle(), harness);
    let output = run_with_expanded_stack(NATIVE_HARNESS_STACK_BYTES, move || {
        compile_and_run(&combined)
    });
    let lines: Vec<&str> = output.trim().lines().collect();

    assert_eq!(
        lines,
        vec!["1", "0", "0", "3002"],
        "selfhost runner は mixed binder の cases 3 を narrow profile 外として拒否すべき"
    );
}

/// EC-M1-05: selfhost runner が Bool の cases 上限を明示拒否すること
#[test]
fn test_e2e_selfhost_runner_rejects_bool_property_above_two_cases() {
    let harness = r#"
(defn main []
  (let [src "(defn identity [x] :property [(for-all [value Bool] :cases 3 :postcondition (or value (not value)))] x)"
        suite (generate-tests-from-source src)
        properties (vector-get suite 4)
        result0 (vector-get properties 0)]
    (do
      (print (vector-length properties))
      (print (vector-get result0 1))
      (print (vector-get result0 2))
      (print (vector-get result0 3))
      0)))
"#;

    let combined = format!("{}\n{}", selfhost_test_runner_runtime_bundle(), harness);
    let output = run_with_expanded_stack(NATIVE_HARNESS_STACK_BYTES, move || {
        compile_and_run(&combined)
    });
    let lines: Vec<&str> = output.trim().lines().collect();

    assert_eq!(
        lines,
        vec!["1", "0", "0", "3002"],
        "selfhost runner は Bool の cases 3 を narrow profile 外として拒否すべき"
    );
}

/// EC-M1-05: selfhost runner が三つの Int binder を cases 1 として実行すること
#[test]
fn test_e2e_selfhost_runner_executes_three_int_property_binders() {
    let harness = r#"
(defn main []
  (let [src "(defn sum3 [left middle right] :property [(for-all [a Int b Int c Int] :cases 1 :postcondition (= result (+ a (+ b c))))] (+ left (+ middle right)))"
        suite (generate-tests-from-source src)
        properties (vector-get suite 4)
        result0 (vector-get properties 0)]
    (do
      (print (vector-length properties))
      (print (vector-get result0 1))
      (print (vector-get result0 2))
      (print (vector-get result0 3))
      0)))
"#;

    let combined = format!("{}\n{}", selfhost_test_runner_runtime_bundle(), harness);
    let output = run_with_expanded_stack(NATIVE_HARNESS_STACK_BYTES, move || {
        compile_and_run(&combined)
    });
    let lines: Vec<&str> = output.trim().lines().collect();

    assert_eq!(
        lines,
        vec!["1", "1", "1", "0"],
        "selfhost runner は三つの Int binder を [0,0,0] の 1 case として評価すべき"
    );
}

/// EC-M1-05: selfhost runner が 3 Int binder の cases 上限を明示拒否すること
#[test]
fn test_e2e_selfhost_runner_rejects_three_int_property_binders_above_two_cases() {
    let harness = r#"
(defn main []
  (let [src "(defn sum3 [left middle right] :property [(for-all [a Int b Int c Int] :cases 3 :postcondition (= result (+ a (+ b c))))] (+ left (+ middle right)))"
        suite (generate-tests-from-source src)
        properties (vector-get suite 4)
        result0 (vector-get properties 0)]
    (do
      (print (vector-length properties))
      (print (vector-get result0 1))
      (print (vector-get result0 2))
      (print (vector-get result0 3))
      0)))
"#;

    let combined = format!("{}\n{}", selfhost_test_runner_runtime_bundle(), harness);
    let output = run_with_expanded_stack(NATIVE_HARNESS_STACK_BYTES, move || {
        compile_and_run(&combined)
    });
    let lines: Vec<&str> = output.trim().lines().collect();

    assert_eq!(
        lines,
        vec!["1", "0", "0", "3002"],
        "selfhost runner は 3 Int binder の cases 3 を narrow profile 外として拒否すべき"
    );
}

/// EC-M1-05: selfhost runner が 3 binder mixed Int/Bool を source-order prefix として実行すること
#[test]
fn test_e2e_selfhost_runner_executes_three_mixed_int_bool_property_binders() {
    let harness = r#"
(defn main []
  (let [src "(defn choose [input enabled offset] :property [(for-all [left Int flag Bool right Int] :cases 2 :postcondition (= result (if flag (+ left right) left)))] (if enabled (+ input offset) input))"
        suite (generate-tests-from-source src)
        properties (vector-get suite 4)
        result0 (vector-get properties 0)]
    (do
      (print (vector-length properties))
      (print (vector-get result0 1))
      (print (vector-get result0 2))
      (print (vector-get result0 3))
      0)))
"#;

    let combined = format!("{}\n{}", selfhost_test_runner_runtime_bundle(), harness);
    let output = run_with_expanded_stack(NATIVE_HARNESS_STACK_BYTES, move || {
        compile_and_run(&combined)
    });
    let lines: Vec<&str> = output.trim().lines().collect();

    assert_eq!(
        lines,
        vec!["1", "1", "2", "0"],
        "selfhost runner は 3 binder mixed を [0,false,0]/[1,true,1] として評価すべき"
    );
}

/// EC-M1-05: selfhost runner が 3 binder mixed の cases 上限を明示拒否すること
#[test]
fn test_e2e_selfhost_runner_rejects_three_mixed_int_bool_property_above_two_cases() {
    let harness = r#"
(defn main []
  (let [src "(defn choose [input enabled offset] :property [(for-all [left Int flag Bool right Int] :cases 3 :postcondition (= result (if flag (+ left right) left)))] (if enabled (+ input offset) input))"
        suite (generate-tests-from-source src)
        properties (vector-get suite 4)
        result0 (vector-get properties 0)]
    (do
      (print (vector-length properties))
      (print (vector-get result0 1))
      (print (vector-get result0 2))
      (print (vector-get result0 3))
      0)))
"#;

    let combined = format!("{}\n{}", selfhost_test_runner_runtime_bundle(), harness);
    let output = run_with_expanded_stack(NATIVE_HARNESS_STACK_BYTES, move || {
        compile_and_run(&combined)
    });
    let lines: Vec<&str> = output.trim().lines().collect();

    assert_eq!(
        lines,
        vec!["1", "0", "0", "3002"],
        "selfhost runner は 3 binder mixed の cases 3 を narrow profile 外として拒否すべき"
    );
}

/// EC-M1-05: selfhost runner が 4 binder mixed Int/Bool を source-order prefix として実行すること
#[test]
fn test_e2e_selfhost_runner_executes_four_mixed_int_bool_property_binders() {
    let harness = r#"
(defn main []
  (let [src "(defn choose [left enabled right ready] :property [(for-all [first Int flag Bool second Int again Bool] :cases 2 :postcondition (= result (if (and flag again) (+ first second) first)))] (if (and enabled ready) (+ left right) left))"
        suite (generate-tests-from-source src)
        properties (vector-get suite 4)
        result0 (vector-get properties 0)]
    (do
      (print (vector-length properties))
      (print (vector-get result0 1))
      (print (vector-get result0 2))
      (print (vector-get result0 3))
      0)))
"#;

    let combined = format!("{}\n{}", selfhost_test_runner_runtime_bundle(), harness);
    let output = run_with_expanded_stack(NATIVE_HARNESS_STACK_BYTES, move || {
        compile_and_run(&combined)
    });
    let lines: Vec<&str> = output.trim().lines().collect();

    assert_eq!(
        lines,
        vec!["1", "1", "2", "0"],
        "selfhost runner は 4 binder mixed を [0,false,0,false]/[1,true,1,true] として評価すべき"
    );
}

/// EC-M1-05: selfhost runner が 4 binder mixed の cases 上限を明示拒否すること
#[test]
fn test_e2e_selfhost_runner_rejects_four_mixed_int_bool_property_above_two_cases() {
    let harness = r#"
(defn main []
  (let [src "(defn choose [left enabled right ready] :property [(for-all [first Int flag Bool second Int again Bool] :cases 3 :postcondition (= result (if (and flag again) (+ first second) first)))] (if (and enabled ready) (+ left right) left))"
        suite (generate-tests-from-source src)
        properties (vector-get suite 4)
        result0 (vector-get properties 0)]
    (do
      (print (vector-length properties))
      (print (vector-get result0 1))
      (print (vector-get result0 2))
      (print (vector-get result0 3))
      0)))
"#;

    let combined = format!("{}\n{}", selfhost_test_runner_runtime_bundle(), harness);
    let output = run_with_expanded_stack(NATIVE_HARNESS_STACK_BYTES, move || {
        compile_and_run(&combined)
    });
    let lines: Vec<&str> = output.trim().lines().collect();

    assert_eq!(
        lines,
        vec!["1", "0", "0", "3002"],
        "selfhost runner は 4 binder mixed の cases 3 を narrow profile 外として拒否すべき"
    );
}

/// EC-M1-04: selfhost runner が property binder の名前衝突を明示拒否すること
#[test]
fn test_e2e_selfhost_runner_rejects_property_binder_name_collisions() {
    let harness = r#"
(defn main []
  (do
    (print (metadata-test-runner-boundary-code (parse-program "(defn pair [left right] :property [(for-all [value Int value Int] :cases 1 :postcondition (= result value))] (+ left right))")))
    (print (metadata-test-runner-boundary-code (parse-program "(defn identity [value] :property [(for-all [result Int] :cases 1 :postcondition (= result 0))] value)")))
    0))
"#;

    let combined = format!("{}\n{}", selfhost_test_runner_runtime_bundle(), harness);
    let output = run_with_expanded_stack(NATIVE_HARNESS_STACK_BYTES, move || {
        compile_and_run(&combined)
    });
    let lines: Vec<&str> = output.trim().lines().collect();

    assert_eq!(
        lines,
        vec!["3002", "3002"],
        "selfhost runner は binder 名の衝突を profile 外として拒否すべき"
    );
}

/// EC-M1-05: selfhost property smoke profile が seed を暗黙に受け入れないこと
#[test]
fn test_e2e_selfhost_runner_rejects_property_seed_option() {
    let harness = r#"
(defn main []
  (do
    (print (metadata-test-runner-boundary-code (parse-program "(defn identity [x] :property [(for-all [x Int] :cases 1 :seed 42 :postcondition (= result x))] x)")))
    0))
"#;

    let combined = format!("{}\n{}", selfhost_test_runner_runtime_bundle(), harness);
    let output = run_with_expanded_stack(NATIVE_HARNESS_STACK_BYTES, move || {
        compile_and_run(&combined)
    });
    let lines: Vec<&str> = output.trim().lines().collect();

    assert_eq!(
        lines,
        vec!["3002"],
        "selfhost runner は deterministic smoke profile 外の seed を明示拒否すべき"
    );
}

/// EC-M1-05: 移行期の deterministic property smoke profile を実行すること
#[test]
fn test_e2e_selfhost_runner_executes_deterministic_property_smoke() {
    let harness = r#"
(defn main []
  (let [src "(defn identity [x] :property [(for-all [x Int] :cases 5 :postcondition (= result x))] x)"
        suite (generate-tests-from-source src)
        properties (vector-get suite 4)
        result0 (vector-get properties 0)]
    (do
      (print (vector-length properties))
      (print (vector-get result0 1))
      (print (vector-get result0 2))
      (print (vector-get result0 3))
      (print (metadata-test-runner-boundary-code (parse-program src)))
      0)))
"#;

    let combined = format!("{}\n{}", selfhost_test_runner_runtime_bundle(), harness);
    let output = run_with_expanded_stack(NATIVE_HARNESS_STACK_BYTES, move || {
        compile_and_run(&combined)
    });
    let lines: Vec<&str> = output.trim().lines().collect();

    assert_eq!(
        lines,
        vec!["1", "1", "5", "0", "0"],
        "selfhost runner は deterministic property smoke を 5 cases 実行して成功扱いすべき"
    );
}

/// EC-M1-05: selfhost runner が単一 String binder を deterministic prefix として実行すること
#[test]
fn test_e2e_selfhost_runner_executes_string_property_binder() {
    let harness = r#"
(defn main []
  (let [src "(defn identity [x] :property [(for-all [sample String] :cases 5 :postcondition (string-eq result sample))] x)"
        suite (generate-tests-from-source src)
        properties (vector-get suite 4)
        result0 (vector-get properties 0)]
    (do
      (print (vector-length properties))
      (print (vector-get result0 1))
      (print (vector-get result0 2))
      (print (vector-get result0 3))
      0)))
"#;

    let combined = format!("{}\n{}", selfhost_test_runner_runtime_bundle(), harness);
    let output = run_with_expanded_stack(NATIVE_HARNESS_STACK_BYTES, move || {
        compile_and_run(&combined)
    });
    let lines: Vec<&str> = output.trim().lines().collect();

    assert_eq!(
        lines,
        vec!["1", "1", "5", "0"],
        "selfhost runner は単一 String binder を 5 cases の string-eq property として評価すべき"
    );
}

/// EC-M1-05: selfhost CLI が deterministic property smoke を 0 件へ丸めないこと
#[test]
fn test_e2e_selfhost_cli_reports_deterministic_property_smoke() {
    let harness = r#"
(defn main []
  (do
    (print (run-test-source "(defn identity [x] :property [(for-all [x Int] :cases 5 :postcondition (= result x))] x)" 0))
    0))
"#;

    let combined = format!("{}\n{}", selfhost_cli_runtime_bundle(), harness);
    let output = run_with_expanded_stack(NATIVE_HARNESS_STACK_BYTES, move || {
        compile_and_run(&combined)
    });
    let lines: Vec<&str> = output.trim().lines().collect();

    assert_eq!(
        lines,
        vec!["examples:0", "invariants:0", "properties:1", "failures:0", "0"],
        "selfhost CLI は deterministic property を properties:1 として集計すべき"
    );
}

/// EC-M1-02: selfhost CLI が二つの Int binder と precondition conjunction を実行すること
#[test]
fn test_e2e_selfhost_cli_reports_two_int_property_binders() {
    let harness = r#"
(defn main []
  (do
    (print (run-test-source "(defn sum [left right] :property [(for-all [a Int b Int] :cases 5 :precondition [(< b 5)] :postcondition (= result (+ a b)))] (+ left right))" 0))
    0))
"#;

    let combined = format!("{}\n{}", selfhost_cli_runtime_bundle(), harness);
    let output = run_with_expanded_stack(NATIVE_HARNESS_STACK_BYTES, move || {
        compile_and_run(&combined)
    });
    let lines: Vec<&str> = output.trim().lines().collect();

    assert_eq!(
        lines,
        vec!["examples:0", "invariants:0", "properties:1", "failures:0", "0"],
        "selfhost CLI は二つの Int binder と precondition conjunction を実行すべき"
    );
}

/// EC-M1-05: selfhost CLI が non-Bool property を実行せず preflight 診断にすること
#[test]
fn test_e2e_selfhost_cli_rejects_non_bool_deterministic_property() {
    let harness = r#"
(defn main []
  (do
    (print (run-test-source "(defn identity [x] :property [(for-all [x Int] :cases 1 :postcondition (+ result 1))] x)" 0))
    0))
"#;

    let combined = format!("{}\n{}", selfhost_cli_runtime_bundle(), harness);
    let output = run_with_expanded_stack(NATIVE_HARNESS_STACK_BYTES, move || {
        compile_and_run(&combined)
    });
    let lines: Vec<&str> = output.trim().lines().collect();

    assert_eq!(
        lines,
        vec![
            "examples:0",
            "invariants:0",
            "properties:1",
            "failures:1",
            "diagnostics:1,LS1002",
            "2",
        ],
        "selfhost CLI は non-Bool property を truthy な整数として実行してはならない"
    );
}

/// EC-M1-03: embedded/native CLI が同じ property runner 境界を呼び出すこと
#[test]
fn test_selfhost_cli_sources_route_property_runner_boundary() {
    for file_name in ["Cli.ls", "EmbeddedCli.ls"] {
        let path = if file_name == "Cli.ls" {
            selfhost_source_path(file_name)
        } else {
            selfhost_project_root().join("selfhost/src/App/EmbeddedCli.ls")
        };
        let source = std::fs::read_to_string(path)
            .unwrap_or_else(|error| panic!("{} の読み込みに失敗: {}", file_name, error));
        assert!(
            source.contains("metadata-test-runner-boundary-code"),
            "{} は未接続の property runner を明示的な境界へ送るべき",
            file_name
        );
        assert!(
            source.contains("check-canonical-properties-with-analysis"),
            "{} の check は canonical property preflight を呼び出すべき",
            file_name
        );
        assert!(
            source.contains("check-property-diagnostic-body-from-code"),
            "{} の check は property 専用診断本文を持つべき",
            file_name
        );
    }
}

/// EC-M1-02: selfhost runner が canonical :case を actual/expected として実行すること
#[test]
fn test_e2e_selfhost_test_runner_materializes_canonical_cases() {
    let harness = r#"
(defn main []
  (let [src "(defn succ [x] :case [(expect (succ 1) 2) (expect (succ 2) 4)] (+ x 1))"
        program (parse-program src)
        cases (extract-cases-from-program program)
        case0 (vector-get cases 0)
        case1 (vector-get cases 1)
        suite (generate-tests src)
        results (vector-get suite 3)
        result0 (vector-get results 0)
        result1 (vector-get results 1)]
    (do
      (print (vector-length cases))
      (print (vector-get case0 0))
      (print (vector-get (vector-get case0 1) 0))
      (print (vector-get (vector-get case0 2) 0))
      (print (vector-get case1 0))
      (print (vector-get (vector-get case1 1) 0))
      (print (vector-get (vector-get case1 2) 0))
      (print (vector-get result0 1))
      (print (vector-get result1 1))
      0)))
"#;

    let combined = format!("{}\n{}", selfhost_test_runner_runtime_bundle(), harness);
    let output = run_with_expanded_stack(NATIVE_HARNESS_STACK_BYTES, move || {
        compile_and_run(&combined)
    });
    let lines: Vec<&str> = output.trim().lines().collect();

    assert_eq!(
        lines,
        vec!["2", "0", "5", "1", "1", "5", "1", "1", "0"],
        "selfhost runner は canonical :case の actual/expected を順序どおり実行するべき"
    );
}

/// EC-M1-02: selfhost runner が空の canonical :case を LS2006 で拒否すること
#[test]
fn test_e2e_selfhost_test_runner_rejects_empty_canonical_case() {
    let harness = r#"
(defn main []
  (let [src "(defn succ [x] :case [] (+ x 1))"
        program (parse-program src)
        cases (extract-cases-from-program program)
        suite (generate-tests src)
        results (vector-get suite 3)
        result0 (vector-get results 0)
        summary (test-diagnostics-summary-with-cases
          (vector-new 0)
          (vector-new 0)
          (vector-new 0)
          results)]
    (do
      (print (vector-length cases))
      (print (vector-length results))
      (print (vector-get result0 1))
      (print (vector-get result0 3))
      (print-string summary)
      (print-string "\n")
      0)))
"#;

    let combined = format!("{}\n{}", selfhost_test_runner_runtime_bundle(), harness);
    let output = run_with_expanded_stack(NATIVE_HARNESS_STACK_BYTES, move || {
        compile_and_run(&combined)
    });
    let lines: Vec<&str> = output.trim().lines().collect();

    assert_eq!(
        lines,
        vec!["1", "1", "0", "2006", "diagnostics:1,LS2006"],
        "selfhost runner は空の canonical :case を 0 件の成功として扱わず LS2006 にするべき"
    );
}

/// EC-M1-02: selfhost runner が :case の未知変数を Unit に丸めないこと
#[test]
fn test_e2e_selfhost_test_runner_rejects_unknown_case_variable() {
    let harness = r#"
(defn main []
  (let [src "(defn identity [x] :case [(expect missing 1) (expect x x) (expect result result)] x)"
        suite (generate-tests src)
        results (vector-get suite 3)
        result0 (vector-get results 0)
        result1 (vector-get results 1)
        result2 (vector-get results 2)
        summary (test-diagnostics-summary-with-cases
          (vector-new 0)
          (vector-new 0)
          (vector-new 0)
          results)]
    (do
      (print (vector-length results))
      (print (vector-get result0 1))
      (print (vector-get result0 3))
      (print (vector-get result1 1))
      (print (vector-get result1 3))
      (print (vector-get result2 1))
      (print (vector-get result2 3))
      (print-string summary)
      (print-string "\n")
      0)))
"#;

    let combined = format!("{}\n{}", selfhost_test_runner_runtime_bundle(), harness);
    let output = run_with_expanded_stack(NATIVE_HARNESS_STACK_BYTES, move || {
        compile_and_run(&combined)
    });
    let lines: Vec<&str> = output.trim().lines().collect();

    assert_eq!(
        lines,
        vec!["3", "0", "1", "0", "1", "0", "1", "diagnostics:3,LS1001"],
        "selfhost runner は未知変数を Unit に丸めず、case の implicit scope を許可するべきではない"
    );
}

/// EC-M1-02: selfhost CLI が canonical :case の件数・失敗を summary へ反映すること
#[test]
fn test_e2e_selfhost_cli_reports_canonical_cases() {
    let harness = r#"
(defn main []
  (let [src "(defn succ [x] :case [(expect (succ 1) 2) (expect (succ 2) 4)] (+ x 1))"]
    (do
      (print-string "BEGIN\n")
      (print (run-test-source src 0))
      0)))
"#;

    let combined = format!("{}\n{}", selfhost_cli_runtime_bundle(), harness);
    let output = run_with_expanded_stack(NATIVE_HARNESS_STACK_BYTES, move || {
        compile_and_run(&combined)
    });
    let lines: Vec<&str> = output.trim().lines().collect();

    assert_eq!(
        lines,
        vec![
            "BEGIN",
            "examples:0",
            "invariants:0",
            "cases:2",
            "failures:1",
            "2",
        ],
        "selfhost CLI は canonical :case を silent success にせず件数と失敗数へ反映するべき"
    );
}

#[test]
fn test_e2e_selfhost_cli_check_reports_legacy_migration_summary() {
    let harness = r#"
(defn main []
  (let [src "(defn succ [x] :example [(succ 0) (= (succ 1) 2)] :invariant (= result (+ x 1)) (+ x 1))"]
    (do
      (print (run-check-source src 0))
      0)))
"#;

    let combined = format!("{}\n{}", selfhost_cli_runtime_bundle(), harness);
    let output = run_with_expanded_stack(NATIVE_HARNESS_STACK_BYTES, move || {
        compile_and_run(&combined)
    });
    let lines: Vec<&str> = output.trim().lines().collect();

    assert_eq!(lines.len(), 5, "selfhost check は migration detail 行を返すべき");
    assert_eq!(lines[0], "Fn");
    assert_eq!(lines[1], "diagnostics:0");
    assert_eq!(
        lines[2],
        "migration:3,LS2001:docs-only-example,LS2001:assertion,LS2002:property-postcondition"
    );
    assert!(lines[3].starts_with("migration-detail:LS2001|owner="));
    assert!(lines[3].contains("|selected=legacy-example-truthiness|disposition=docs-only-example|"));
    assert!(lines[3].contains("|selected=legacy-example-truthiness|disposition=assertion|"));
    assert!(lines[3].contains("|selected=legacy-invariant-deterministic-smoke|disposition=property-postcondition|"));
    assert!(lines[3].contains("|message=non-Bool (Int) legacy :example は docs-only :example として保持する候補です"));
    assert!(lines[3].contains("|message=Bool legacy :example は strict :assert への移行候補です"));
    assert!(lines[3].contains("|message=legacy :invariant は :property / :postcondition への移行候補です"));
    assert_eq!(lines[4], "0");
}

/// EC-M1-03: selfhost check の JSON option が構造化 migration report を返すこと
#[test]
fn test_e2e_selfhost_cli_check_source_json_returns_structured_migration_report() {
    let harness = r#"
(defn main []
  (let [src "(defn succ [x] :example [(succ 0) (= (succ 1) 2)] :invariant (= result (+ x 1)) (+ x 1))"]
    (do
      (print (run-check-source src 1))
      0)))
"#;

    let combined = format!("{}\n{}", selfhost_cli_runtime_bundle(), harness);
    let output = run_with_expanded_stack(NATIVE_HARNESS_STACK_BYTES, move || {
        compile_and_run(&combined)
    });
    let lines: Vec<&str> = output.trim().lines().collect();

    assert_eq!(lines.len(), 2, "check --json は JSON report と終了コードだけを返すべき");
    let report: Value = serde_json::from_str(lines[0])
        .expect("check --json の report は valid JSON であるべき");
    assert_eq!(report["command"], "check");
    assert_eq!(report["type"], "Fn");
    assert_eq!(report["diagnostics"]["count"], 0);
    assert_eq!(report["diagnostics"]["firstErrorCode"], 0);
    assert_eq!(report["diagnostics"]["message"], "");
    assert_eq!(report["migration"].as_array().unwrap().len(), 3);
    assert_eq!(lines[1], "0");
}

/// EC-M1-03: selfhost check JSON が診断時に non-zero exit を返すこと
#[test]
fn test_e2e_selfhost_cli_check_source_json_returns_diagnostic_exit() {
    let harness = r#"
(defn main []
  (let [result (run-check-source "(defn main [] (if 42 1 0))" 1)]
    (do
      (print result)
      0)))
"#;

    let combined = format!("{}\n{}", selfhost_cli_runtime_bundle(), harness);
    let output = run_with_expanded_stack(NATIVE_HARNESS_STACK_BYTES, move || {
        compile_and_run(&combined)
    });
    let lines: Vec<&str> = output.trim().lines().collect();

    assert_eq!(lines.len(), 2, "診断付き check --json は report と exit code を返すべき");
    let report: Value = serde_json::from_str(lines[0])
        .expect("診断付き check --json の report は valid JSON であるべき");
    assert!(report["diagnostics"]["count"].as_i64().unwrap() > 0);
    assert!(report["diagnostics"]["firstErrorCode"].as_i64().unwrap() > 0);
    assert!(!report["diagnostics"]["message"].as_str().unwrap().is_empty());
    assert_eq!(lines[1], "1");
}

/// EC-M1-03: selfhost migration JSON の文字列値が JSON の escape 規則を守ること
#[test]
fn test_e2e_selfhost_migration_json_quote_escapes_delimiters_and_controls() {
    let harness = r#"
(defn main []
  (let [value (string-concat "quote: \" slash: \\" "\n\t")]
    (do
      (print-string (legacy-json-quote value))
      0)))
"#;

    let combined = format!("{}\n{}", selfhost_migration_runtime_bundle(), harness);
    let output = run_with_expanded_stack(NATIVE_HARNESS_STACK_BYTES, move || {
        compile_and_run(&combined)
    });

    assert_eq!(
        output.trim(),
        r#""quote: \" slash: \\\n\t""#,
        "legacy migration JSON の文字列値は quote/backslash/control を escape するべき"
    );
    let parsed: Value = serde_json::from_str(output.trim())
        .expect("legacy migration JSON の quoted string は valid JSON であるべき");
    assert_eq!(
        parsed,
        Value::String("quote: \" slash: \\\n\t".to_owned()),
        "legacy migration JSON の escape は元の文字列へ round-trip するべき"
    );
}

/// EC-M1-03: selfhost migration row が parser-owned owner と directive span を保持すること
#[test]
fn test_e2e_selfhost_migration_rows_preserve_legacy_owner_and_directive_spans() {
    let harness = r#"
(defn main []
  (let [src "(defn succ [x] :example [(succ 0) (= (succ 1) 2)] :invariant (= result (+ x 1)) (+ x 1))"
        program (parse-program src)
        rows (classify-legacy-contracts program)
        row0 (vector-get rows 0)
        row1 (vector-get rows 1)
        row2 (vector-get rows 2)
        raw (extract-contract-forms src)
        raw0 (vector-get raw 0)
        raw1 (vector-get raw 1)
        detail0 (legacy-migration-row-detail-text row0)
        detail1 (legacy-migration-row-detail-text row1)
        detail2 (legacy-migration-row-detail-text row2)
        detail-summary (legacy-migration-detail-summary rows)
        detail-json0 (legacy-migration-row-detail-json row0)
        detail-json1 (legacy-migration-row-detail-json row1)
        detail-json2 (legacy-migration-row-detail-json row2)
        detail-json-summary (legacy-migration-detail-json-summary rows)
        expected-detail-summary (string-concat
          "migration-detail:"
          (string-concat
            detail0
            (string-concat "," (string-concat detail1 (string-concat "," detail2)))))
        expected-detail-json0 (string-concat
          "{\"code\":\"LS2001\",\"ownerHash\":"
          (string-concat
            (int-to-string (vector-get raw0 1))
            (string-concat
              ",\"selectedSemantics\":\"legacy-example-truthiness\",\"disposition\":\"docs-only-example\",\"span\":{\"start\":"
              (string-concat
                (int-to-string (vector-get raw0 3))
                (string-concat
                  ",\"end\":"
                  (string-concat
                    (int-to-string (vector-get raw0 4))
                    (string-concat
                      "},\"message\":\""
                      (string-concat
                        (vector-get row0 5)
                        (string-concat
                          "\",\"expressionSpan\":{\"start\":"
                          (string-concat
                            (int-to-string (vector-get row0 7))
                            (string-concat
                              ",\"end\":"
                              (string-concat
                                (int-to-string (vector-get row0 8))
                                "}}"))))))))))))
        expected-detail-json-summary (string-concat
          "["
          (string-concat
            detail-json0
            (string-concat "," (string-concat detail-json1 (string-concat "," (string-concat detail-json2 "]"))))))
        expected-detail0 (string-concat
          "LS2001|owner="
          (string-concat
            (int-to-string (vector-get raw0 1))
            (string-concat
              "|selected=legacy-example-truthiness|disposition=docs-only-example|span="
              (string-concat
                (int-to-string (vector-get raw0 3))
                (string-concat
                  "-"
                  (string-concat
                    (int-to-string (vector-get raw0 4))
                    (string-concat "|message=" (vector-get row0 5))))))))]
    (do
      (print (vector-length rows))
      (print (vector-length row0))
      (print (vector-length row1))
      (print (vector-length row2))
      (print (= (vector-get row0 2) (vector-get raw0 3)))
      (print (= (vector-get row0 3) (vector-get raw0 4)))
      (print (= (vector-get row1 2) (vector-get raw0 3)))
      (print (= (vector-get row1 3) (vector-get raw0 4)))
      (print (= (vector-get row2 2) (vector-get raw1 3)))
      (print (= (vector-get row2 3) (vector-get raw1 4)))
      (print (= (vector-get row0 4) (vector-get raw0 1)))
      (print (= (vector-get row1 4) (vector-get raw0 1)))
      (print (= (vector-get row2 4) (vector-get raw1 1)))
      (print (string-eq
        (vector-get row0 5)
        "non-Bool (Int) legacy :example は docs-only :example として保持する候補です"))
      (print (string-eq
        (vector-get row1 5)
        "Bool legacy :example は strict :assert への移行候補です"))
      (print (string-eq
        (vector-get row2 5)
        "legacy :invariant は :property / :postcondition への移行候補です"))
      (print (= (vector-get row0 6) 1))
      (print (= (vector-get row1 6) 1))
      (print (= (vector-get row2 6) 2))
      (print (string-eq detail0 expected-detail0))
      (print (string-eq detail-summary expected-detail-summary))
      (print (string-eq detail-json0 expected-detail-json0))
      (print (string-eq detail-json-summary expected-detail-json-summary))
      (print-string detail-json0)
      (print-string "\n")
      (print-string detail-json-summary)
      (print-string "\n")
      0)))
"#;

    let combined = format!("{}\n{}", selfhost_migration_runtime_bundle(), harness);
    let output = run_with_expanded_stack(NATIVE_HARNESS_STACK_BYTES, move || {
        compile_and_run(&combined)
    });
    let lines: Vec<&str> = output.trim().lines().collect();

    let expected_check_lines = vec![
        "3", "9", "9", "9", "1", "1", "1", "1", "1", "1", "1", "1", "1", "1", "1", "1", "1", "1", "1", "1", "1", "1", "1",
    ];
    assert_eq!(lines.len(), expected_check_lines.len() + 2);
    assert_eq!(
        &lines[..expected_check_lines.len()],
        expected_check_lines.as_slice(),
        "selfhost migration row は raw inventory の owner と directive span を共有するべき"
    );
    let detail_json: Value = serde_json::from_str(lines[expected_check_lines.len()])
        .expect("selfhost migration detail は valid JSON であるべき");
    assert_eq!(detail_json["code"], "LS2001");
    assert_eq!(detail_json["selectedSemantics"], "legacy-example-truthiness");
    assert_eq!(detail_json["disposition"], "docs-only-example");
    assert_eq!(
        detail_json["message"],
        "non-Bool (Int) legacy :example は docs-only :example として保持する候補です"
    );
    assert!(detail_json["ownerHash"].is_i64());
    assert!(detail_json["span"]["start"].is_i64());
    assert!(detail_json["span"]["end"].is_i64());
    assert!(detail_json["expressionSpan"]["start"].is_i64());
    assert!(detail_json["expressionSpan"]["end"].is_i64());
    let detail_json_summary: Value = serde_json::from_str(lines[expected_check_lines.len() + 1])
        .expect("selfhost migration detail summary は valid JSON であるべき");
    assert_eq!(detail_json_summary.as_array().map(Vec::len), Some(3));
}

/// EC-M1-03: selfhost migration row が各 legacy expression の source span を保持すること
#[test]
fn test_e2e_selfhost_migration_rows_preserve_expression_spans() {
    let source =
        "(defn succ [x] :example [(succ 0) (= (succ 1) 2)] :invariant (= result (+ x 1)) (+ x 1))";
    let harness = format!(
        r#"
(defn main []
  (let [program (parse-program "{source}")
        rows (classify-legacy-contracts program)
        row0 (vector-get rows 0)
        row1 (vector-get rows 1)
        row2 (vector-get rows 2)]
    (do
      (print (vector-length row0))
      (print (if (> (vector-length row0) 8) (vector-get row0 7) -1))
      (print (if (> (vector-length row0) 8) (vector-get row0 8) -1))
      (print (vector-length row1))
      (print (if (> (vector-length row1) 8) (vector-get row1 7) -1))
      (print (if (> (vector-length row1) 8) (vector-get row1 8) -1))
      (print (vector-length row2))
      (print (if (> (vector-length row2) 8) (vector-get row2 7) -1))
      (print (if (> (vector-length row2) 8) (vector-get row2 8) -1))
      0)))
"#
    );

    let combined = format!("{}\n{}", selfhost_migration_runtime_bundle(), harness);
    let output = run_with_expanded_stack(NATIVE_HARNESS_STACK_BYTES, move || {
        compile_and_run(&combined)
    });
    let actual: Vec<String> = output
        .trim()
        .lines()
        .map(std::string::ToString::to_string)
        .collect();

    let expected_spans = ["(succ 0)", "(= (succ 1) 2)", "(= result (+ x 1))"]
        .map(|expr| {
            let start = source.find(expr).expect("expression span fixture が見つかる");
            (start.to_string(), (start + expr.len()).to_string())
        });
    let expected = vec![
        "9".to_owned(),
        expected_spans[0].0.clone(),
        expected_spans[0].1.clone(),
        "9".to_owned(),
        expected_spans[1].0.clone(),
        expected_spans[1].1.clone(),
        "9".to_owned(),
        expected_spans[2].0.clone(),
        expected_spans[2].1.clone(),
    ];
    assert_eq!(
        actual, expected,
        "legacy migration row は directive span だけでなく各 expression span を保持するべき"
    );
}

/// EC-M1-03: selfhost CLI が canonical :assert の件数を結果へ反映すること
#[test]
#[ignore]
fn test_e2e_selfhost_cli_reports_canonical_assertions() {
    let harness = r#"
(defn main []
  (let [src "(defn positive [] :assert [(> 1 0) (= 1 1)] true)"]
    (do
      (print-string "BEGIN\n")
      (print (run-test-source src 0))
      0)))
"#;

    let combined = format!("{}\n{}", selfhost_cli_runtime_bundle(), harness);
    let output = run_with_expanded_stack(NATIVE_HARNESS_STACK_BYTES, move || {
        compile_and_run(&combined)
    });
    let lines: Vec<&str> = output.trim().lines().collect();

    assert_eq!(
        lines,
        vec![
            "BEGIN",
            "examples:0",
            "invariants:0",
            "assertions:2",
            "failures:0",
            "0"
        ],
        "selfhost CLI は canonical :assert を silent success にせず件数へ反映するべき"
    );
}

/// EC-M1-02: selfhost runner が module/private declaration 内の invariant を投影すること
#[test]
fn test_e2e_selfhost_test_runner_projects_nested_invariant_forms() {
    let harness = r#"
(defn main []
  (let [src "(module Math (defn succ [x] :invariant (= result (+ x 1)) (+ x 1))) (private (defn pred [x] :invariant (= result (- x 1)) (- x 1)))"
        program (parse-program src)
        invariants (extract-invariants-from-program program)
        suite (generate-tests src)
        results (vector-get suite 1)
        result0 (vector-get results 0)
        result1 (vector-get results 1)]
    (do
      (print (vector-length invariants))
      (print (vector-length results))
      (print (vector-get result0 1))
      (print (vector-get result1 1))
      0)))
"#;

    let combined = format!("{}\n{}", selfhost_test_runner_runtime_bundle(), harness);
    let output = run_with_expanded_stack(NATIVE_HARNESS_STACK_BYTES, move || {
        compile_and_run(&combined)
    });
    let lines: Vec<&str> = output.trim().lines().collect();

    assert_eq!(
        lines,
        vec!["2", "2", "1", "1"],
        "runner は module/private 内の invariant を順序どおり実行するべき"
    );
}

/// EC-M1-01: Rust oracle と selfhost runner が invariant parameter scope の結果を揃えること
#[test]
fn test_e2e_selfhost_test_runner_matches_rust_oracle_for_invariant_scope() {
    let source = "(defn succ [x] :invariant (= result (+ x 1)) (+ x 1))";
    let oracle = run_metadata_tests(source);
    assert_eq!(oracle.len(), 1, "Rust oracle は invariant 1 件を生成するべき");
    assert!(oracle[0].passed, "Rust oracle の invariant は全 sample で pass するべき");

    let harness = r#"
(defn main []
  (let [src "(defn succ [x] :invariant (= result (+ x 1)) (+ x 1))"
        suite (generate-tests src)
        results (vector-get suite 1)
        result0 (vector-get results 0)]
    (do
      (print (vector-length results))
      (print (vector-get result0 1))
      (print (vector-get result0 2))
      (print (vector-get result0 3))
      0)))
"#;

    let combined = format!("{}\n{}", selfhost_test_runner_runtime_bundle(), harness);
    let output = run_with_expanded_stack(NATIVE_HARNESS_STACK_BYTES, move || {
        compile_and_run(&combined)
    });
    let lines: Vec<&str> = output.trim().lines().collect();
    assert_eq!(lines, vec!["1", "1", "5", "0"]);
}

/// EC-M1-01: Rust oracle と selfhost runner が invariant 内の local-let scope を揃えること
#[test]
fn test_e2e_selfhost_test_runner_matches_rust_oracle_for_invariant_local_let_scope() {
    let source =
        "(defn succ [x] :invariant (let [delta 1] (= result (+ x delta))) (+ x 1))";
    let oracle = run_metadata_tests(source);
    assert_eq!(oracle.len(), 1, "Rust oracle は invariant 1 件を生成するべき");
    assert!(
        oracle[0].passed,
        "Rust oracle の local-let invariant は全 sample で pass するべき"
    );

    let harness = r#"
(defn main []
  (let [src "(defn succ [x] :invariant (let [delta 1] (= result (+ x delta))) (+ x 1))"
        suite (generate-tests src)
        results (vector-get suite 1)
        result0 (vector-get results 0)]
    (do
      (print (vector-length results))
      (print (vector-get result0 1))
      (print (vector-get result0 2))
      (print (vector-get result0 3))
      0)))
"#;

    let combined = format!("{}\n{}", selfhost_test_runner_runtime_bundle(), harness);
    let output = run_with_expanded_stack(NATIVE_HARNESS_STACK_BYTES, move || {
        compile_and_run(&combined)
    });
    let lines: Vec<&str> = output.trim().lines().collect();
    assert_eq!(
        lines,
        vec!["1", "1", "5", "0"],
        "selfhost invariant は local-let binding を含む x/result scope と 5 サンプルを Rust oracle と揃えるべき"
    );
}

/// EC-M1-01: invariant 内 lambda の未知変数を Rust oracle と selfhost が診断すること
#[test]
fn test_e2e_selfhost_test_runner_reports_unknown_invariant_lambda_variable() {
    let source = "(defn succ [x] :invariant (let [check (fn [delta] (= result (+ x missing)))] true) (+ x 1))";
    let program = lsharp_syntax::parse(source).expect("lambda を含む invariant fixture は parse できるべき");
    let diagnostics = lsharp_types::metadata_check::check_metadata(&program);
    assert!(
        diagnostics
            .iter()
            .any(|diagnostic| diagnostic.message.contains("未定義")),
        "Rust oracle は lambda 本体の未知変数を診断するべき: {diagnostics:?}"
    );

    let harness = r#"
(defn main []
  (let [src "(defn succ [x] :invariant (let [check (fn [delta] (= result (+ x missing)))] true) (+ x 1))"
        suite (generate-tests src)
        results (vector-get suite 1)
        result0 (vector-get results 0)]
    (do
      (print (vector-length results))
      (print (vector-get result0 1))
      (print (vector-get result0 2))
      (print (vector-get result0 3))
      0)))
"#;

    let combined = format!("{}\n{}", selfhost_test_runner_runtime_bundle(), harness);
    let output = run_with_expanded_stack(NATIVE_HARNESS_STACK_BYTES, move || {
        compile_and_run(&combined)
    });
    let lines: Vec<&str> = output.trim().lines().collect();
    assert_eq!(
        lines,
        vec!["1", "0", "0", "1"],
        "selfhost contract path は lambda 本体の未知変数を silent Unit/0 fallback にしないべき"
    );
}

/// EC-M1-01: invariant 内 computation binding の未知変数を Rust oracle と selfhost が診断すること
#[test]
fn test_e2e_selfhost_test_runner_reports_unknown_invariant_computation_variable() {
    let source = "(defn succ [x] :invariant (computation maybe-builder (let! delta missing) (return (= result (+ x delta)))) (+ x 1))";
    let program = lsharp_syntax::parse(source)
        .expect("computation を含む invariant fixture は parse できるべき");
    let diagnostics = lsharp_types::metadata_check::check_metadata(&program);
    assert!(
        diagnostics
            .iter()
            .any(|diagnostic| diagnostic.message.contains("未定義")),
        "Rust oracle は computation step の未知変数を診断するべき: {diagnostics:?}"
    );

    let harness = r#"
(defn main []
  (let [src "(defn succ [x] :invariant (computation maybe-builder (let! delta missing) (return (= result (+ x delta)))) (+ x 1))"
        suite (generate-tests src)
        results (vector-get suite 1)
        result0 (vector-get results 0)]
    (do
      (print (vector-length results))
      (print (vector-get result0 1))
      (print (vector-get result0 2))
      (print (vector-get result0 3))
      0)))
"#;

    let combined = format!("{}\n{}", selfhost_test_runner_runtime_bundle(), harness);
    let output = run_with_expanded_stack(NATIVE_HARNESS_STACK_BYTES, move || {
        compile_and_run(&combined)
    });
    let lines: Vec<&str> = output.trim().lines().collect();
    assert_eq!(
        lines,
        vec!["1", "0", "0", "1"],
        "selfhost contract path は computation step の未知変数を LS1002 にすり替えず LS1001 として報告するべき"
    );
}

/// EC-M1-01: invariant 内 computation の let! binding を Rust oracle と selfhost が評価すること
#[test]
fn test_e2e_selfhost_test_runner_matches_rust_oracle_for_valid_invariant_computation() {
    let source = r#"
(computation-builder maybe-builder mb identity)
(defn identity [x] x)
(defn mb [m f] (f m))
(defn succ [x]
  :invariant (computation maybe-builder (let! delta 1) (return (= result (+ x delta))))
  (+ x 1))
"#;
    let oracle = run_metadata_tests(source);
    assert_eq!(oracle.len(), 1, "Rust oracle は computation invariant 1 件を生成するべき");
    assert!(oracle[0].passed, "Rust oracle の computation invariant は pass するべき");

    let harness = r#"
(defn main []
  (let [src "(computation-builder maybe-builder mb identity) (defn identity [x] x) (defn mb [m f] (f m)) (defn succ [x] :invariant (computation maybe-builder (let! delta 1) (return (= result (+ x delta)))) (+ x 1))"
        suite (generate-tests src)
        results (vector-get suite 1)
        result0 (vector-get results 0)]
    (do
      (print (vector-length results))
      (print (vector-get result0 1))
      (print (vector-get result0 2))
      (print (vector-get result0 3))
      0)))
"#;

    let combined = format!("{}\n{}", selfhost_test_runner_runtime_bundle(), harness);
    let output = run_with_expanded_stack(NATIVE_HARNESS_STACK_BYTES, move || {
        compile_and_run(&combined)
    });
    let lines: Vec<&str> = output.trim().lines().collect();
    assert_eq!(
        lines,
        vec!["1", "1", "5", "0"],
        "selfhost computation evaluator は let! binding を保持し Rust oracle と揃えるべき"
    );
}

/// EC-M1-01: invariant 内 match の variable pattern を Rust oracle と selfhost が評価すること
#[test]
fn test_e2e_selfhost_test_runner_matches_rust_oracle_for_valid_invariant_match() {
    let source = r#"
(defn succ [x]
  :invariant (match x [value (= result (+ value 1))])
  (+ x 1))
"#;
    let oracle = run_metadata_tests(source);
    assert_eq!(oracle.len(), 1, "Rust oracle は match invariant 1 件を生成するべき");
    assert!(oracle[0].passed, "Rust oracle の match invariant は pass するべき");

    let harness = r#"
(defn main []
  (let [src "(defn succ [x] :invariant (match x [value (= result (+ value 1))]) (+ x 1))"
        suite (generate-tests src)
        results (vector-get suite 1)
        result0 (vector-get results 0)]
    (do
      (print (vector-length results))
      (print (vector-get result0 1))
      (print (vector-get result0 2))
      (print (vector-get result0 3))
      0)))
"#;

    let combined = format!("{}\n{}", selfhost_test_runner_runtime_bundle(), harness);
    let output = run_with_expanded_stack(NATIVE_HARNESS_STACK_BYTES, move || {
        compile_and_run(&combined)
    });
    let lines: Vec<&str> = output.trim().lines().collect();
    assert_eq!(
        lines,
        vec!["1", "1", "5", "0"],
        "selfhost match evaluator は variable pattern binding を保持し Rust oracle と揃えるべき"
    );
}

/// EC-M1-01: invariant 内 match の literal / wildcard pattern を Rust oracle と selfhost が評価すること
#[test]
fn test_e2e_selfhost_test_runner_matches_rust_oracle_for_literal_and_wildcard_match() {
    let source = r#"
(defn literal-match [x]
  :invariant (match true [true (= result result)] [_ false])
  (+ x 1))
(defn wildcard-match [x]
  :invariant (match x [_ (= result (+ x 1))])
  (+ x 1))
"#;
    let oracle = run_metadata_tests(source);
    assert_eq!(oracle.len(), 2, "Rust oracle は match invariant 2 件を生成するべき");
    assert!(oracle.iter().all(|result| result.passed));

    let harness = r#"
(defn main []
  (let [src "(defn literal-match [x] :invariant (match true [true (= result result)] [_ false]) (+ x 1)) (defn wildcard-match [x] :invariant (match x [_ (= result (+ x 1))]) (+ x 1))"
        suite (generate-tests src)
        results (vector-get suite 1)
        result0 (vector-get results 0)
        result1 (vector-get results 1)]
    (do
      (print (vector-length results))
      (print (vector-get result0 1))
      (print (vector-get result0 2))
      (print (vector-get result0 3))
      (print (vector-get result1 1))
      (print (vector-get result1 2))
      (print (vector-get result1 3))
      0)))
"#;

    let combined = format!("{}\n{}", selfhost_test_runner_runtime_bundle(), harness);
    let output = run_with_expanded_stack(NATIVE_HARNESS_STACK_BYTES, move || {
        compile_and_run(&combined)
    });
    let lines: Vec<&str> = output.trim().lines().collect();
    assert_eq!(
        lines,
        vec!["2", "1", "5", "0", "1", "5", "0"],
        "selfhost match evaluator は literal/wildcard pattern を Rust oracle と揃えるべき"
    );
}

/// EC-M1-01: invariant 内 match arm の未知変数を Rust oracle と selfhost が診断すること
#[test]
fn test_e2e_selfhost_test_runner_reports_unknown_invariant_match_variable() {
    let source =
        "(defn succ [x] :invariant (match x [value (= result missing)] [_ true]) (+ x 1))";
    let program =
        lsharp_syntax::parse(source).expect("match を含む invariant fixture は parse できるべき");
    let diagnostics = lsharp_types::metadata_check::check_metadata(&program);
    assert!(
        diagnostics
            .iter()
            .any(|diagnostic| diagnostic.message.contains("未定義")),
        "Rust oracle は match arm body の未知変数を診断するべき: {diagnostics:?}"
    );

    let harness = r#"
(defn main []
  (let [src "(defn succ [x] :invariant (match x [value (= result missing)] [_ true]) (+ x 1))"
        suite (generate-tests src)
        results (vector-get suite 1)
        result0 (vector-get results 0)]
    (do
      (print (vector-length results))
      (print (vector-get result0 1))
      (print (vector-get result0 2))
      (print (vector-get result0 3))
      0)))
"#;

    let combined = format!("{}\n{}", selfhost_test_runner_runtime_bundle(), harness);
    let output = run_with_expanded_stack(NATIVE_HARNESS_STACK_BYTES, move || {
        compile_and_run(&combined)
    });
    let lines: Vec<&str> = output.trim().lines().collect();
    assert_eq!(
        lines,
        vec!["1", "0", "0", "1"],
        "selfhost contract path は match arm body の未知変数を LS1002 にすり替えず LS1001 として報告するべき"
    );
}

/// TEST-CLI-02-O2b: selfhost/src/Tools/Test/TestRunner.ls が supported subset の metadata suite を実行できること
#[test]
#[ignore]
fn test_e2e_selfhost_test_runner_executes_examples_only() {
    let harness = r#"
(defn main []
  (let [src "(defn abs [x] :example [(= (abs 5) 5) (= (abs (- 0 7)) 7)] (if (< x 0) (- 0 x) x))"
        program (parse-program src)
        results (run-examples program (extract-examples src))
        example0 (vector-get results 0)
        example1 (vector-get results 1)]
    (do
      (print (vector-length results))
      (print (vector-get example0 1))
      (print (vector-get example1 1))
      0)))
"#;

    let combined = format!("{}\n{}", selfhost_cli_runtime_bundle(), harness);
    let output = compile_and_run(&combined);
    let lines: Vec<&str> = output.trim().lines().collect();

    assert_eq!(
        lines,
        vec!["2", "1", "1"],
        "run-examples は supported examples を 2 件成功として実行できるべき"
    );
}

/// TEST-CLI-02-O2c: selfhost/src/Tools/Test/TestRunner.ls が supported invariant suite を materialize できること
#[test]
#[ignore]
fn test_e2e_selfhost_test_runner_executes_invariant_only() {
    let harness = r#"
(defn main []
  (let [src "(defn abs [x] :invariant (>= result 0) (if (< x 0) (- 0 x) x))"
        suite (generate-tests-from-source src)
        results (vector-get suite 1)
        invariant0 (vector-get results 0)]
    (do
      (print (vector-length results))
      (print (vector-get invariant0 1))
      (print (vector-get invariant0 2))
      0)))
"#;

    let combined = format!("{}\n{}", selfhost_cli_runtime_bundle(), harness);
    let output = compile_and_run(&combined);
    let lines: Vec<&str> = output.trim().lines().collect();

    assert_eq!(
        lines,
        vec!["1", "1", "5"],
        "run-invariants は supported invariant を 5 サンプル計画付きで materialize できるべき"
    );
}

/// TEST-CLI-02-O2e: selfhost runner が invariant の元関数引数を scope に束縛すること
#[test]
fn test_e2e_selfhost_test_runner_binds_invariant_parameters() {
    let harness = r#"
(defn main []
  (let [src "(defn succ [x] :invariant (= result (+ x 1)) (+ x 1))"
        suite (generate-tests-from-source src)
        results (vector-get suite 1)
        invariant0 (vector-get results 0)]
    (do
      (print (vector-length results))
      (print (vector-get invariant0 1))
      (print (vector-get invariant0 2))
      0)))
"#;

    let combined = format!("{}\n{}", selfhost_cli_runtime_bundle(), harness);
    let output = compile_and_run(&combined);
    let lines: Vec<&str> = output.trim().lines().collect();

    assert_eq!(
        lines,
        vec!["1", "1", "5"],
        "selfhost invariant は x/result scope と 5 サンプルを Rust oracle と揃えるべき"
    );
}

/// TEST-CLI-02-O2f: selfhost runner が invariant の未定義変数を LS1001 として報告すること
#[test]
fn test_e2e_selfhost_test_runner_reports_unknown_invariant_variable() {
    let harness = r#"
(defn main []
  (let [src "(defn succ [x] :invariant (= result (+ missing 1)) (+ x 1))"]
    (do
      (print-string "BEGIN\n")
      (print (run-test-source src 0))
      0)))
"#;

    let combined = format!("{}\n{}", selfhost_cli_runtime_bundle(), harness);
    let output = run_with_expanded_stack(NATIVE_HARNESS_STACK_BYTES, move || {
        compile_and_run(&combined)
    });
    let lines: Vec<&str> = output.trim().lines().collect();

    assert_eq!(
        lines,
        vec![
            "BEGIN",
            "examples:0",
            "invariants:1",
            "failures:1",
            "diagnostics:1,LS1001",
            "2",
        ],
        "selfhost contract path は未定義 invariant 変数を silent Unit/0 fallback にしないべき"
    );
}

/// TEST-CLI-02-O2g1: selfhost runner が Bool でない invariant を LS1002 として報告すること
#[test]
fn test_e2e_selfhost_test_runner_rejects_non_bool_invariant() {
    let harness = r#"
(defn main []
  (let [src "(defn succ [x] :invariant (+ x 1) (+ x 1))"]
    (do
      (print-string "BEGIN\n")
      (print (run-test-source src 0))
      0)))
"#;

    let combined = format!("{}\n{}", selfhost_cli_runtime_bundle(), harness);
    let output = run_with_expanded_stack(NATIVE_HARNESS_STACK_BYTES, move || {
        compile_and_run(&combined)
    });
    let lines: Vec<&str> = output.trim().lines().collect();

    assert_eq!(
        lines,
        vec![
            "BEGIN",
            "examples:0",
            "invariants:1",
            "failures:1",
            "diagnostics:1,LS1002",
            "2",
        ],
        "selfhost contract path は Bool でない invariant を truthy として成功扱いしないべき"
    );
}

/// EC-M1-02: selfhost check が canonical :case の型エラーを実行前に報告すること
#[test]
fn test_e2e_selfhost_cli_check_reports_invalid_canonical_case() {
    let harness = r#"
(defn main []
  (do
    (print-string "BEGIN\n")
    (print (run-check-source "(defn noop [] :case [(expect \"a\" \"a\")] true)" 0))
    0))
"#;

    let combined = format!("{}\n{}", selfhost_cli_runtime_bundle(), harness);
    let output = run_with_expanded_stack(NATIVE_HARNESS_STACK_BYTES, move || {
        compile_and_run(&combined)
    });
    let lines: Vec<&str> = output.trim().lines().collect();

    assert_eq!(
        lines,
        vec![
            "BEGIN",
            "Bool",
            "diagnostics:1,T0001@1:1,first-body:case actual and expected types must be Int or Bool",
            "0",
        ],
        "selfhost check は canonical case の unsupported type を実行前に報告するべき"
    );
}

/// EC-M1-04: selfhost check が canonical :property の non-Bool predicate を報告すること。
#[test]
fn test_e2e_selfhost_cli_check_rejects_non_bool_canonical_property() {
    let harness = r#"
(defn main []
  (let [source "(defn identity [x] :property [(for-all [x Int] :postcondition (+ result 1))] x)"
        program (parse-program source)
        analysis (infer-program-analysis program)
        result (check-canonical-properties-with-analysis program analysis)]
    (do
      (print-string "BEGIN\n")
      (print (vector-get result 0))
      (print (vector-get result 1))
      0)))
"#;

    let combined = format!(
        "{}\n{}",
        selfhost_parser_typeinfer_runtime_bundle(),
        harness
    );
    let output = run_with_expanded_stack(NATIVE_HARNESS_STACK_BYTES, move || {
        compile_and_run(&combined)
    });
    let lines: Vec<&str> = output.trim().lines().collect();

    assert_eq!(
        lines,
        vec![
            "BEGIN",
            "1",
            "1002",
        ],
        "selfhost property checker は non-Bool predicate を実行前に報告するべき"
    );
}

/// EC-M1-04: selfhost check が typed property binder を postcondition scope へ投影すること。
#[test]
fn test_e2e_selfhost_cli_check_accepts_typed_property_binder() {
    let harness = r#"
(defn main []
  (let [source "(defn abs [x] :property [(for-all [value Int] :precondition [(>= value 0)] :postcondition (>= result value))] x)"
        program (parse-program source)
        analysis (infer-program-analysis program)
        result (check-canonical-properties-with-analysis program analysis)]
    (do
      (print-string "BEGIN\n")
      (print (vector-get result 0))
      (print (vector-get result 1))
      0)))
"#;

    let combined = format!(
        "{}\n{}",
        selfhost_parser_typeinfer_runtime_bundle(),
        harness
    );
    let output = run_with_expanded_stack(NATIVE_HARNESS_STACK_BYTES, move || {
        compile_and_run(&combined)
    });
    let lines: Vec<&str> = output.trim().lines().collect();

    assert_eq!(
        lines,
        vec!["BEGIN", "0", "0"],
        "selfhost property checker は typed binder を postcondition scope へ投影するべき"
    );
}

/// EC-M1-04: selfhost check が precondition も Bool preflight の対象にすること。
#[test]
fn test_e2e_selfhost_cli_check_rejects_non_bool_property_precondition() {
    let harness = r#"
(defn main []
  (let [source "(defn identity [x] :property [(for-all [value Int] :precondition [(+ value 1)] :postcondition (>= result 0))] x)"
        program (parse-program source)
        analysis (infer-program-analysis program)
        result (check-canonical-properties-with-analysis program analysis)]
    (do
      (print-string "BEGIN\n")
      (print (vector-get result 0))
      (print (vector-get result 1))
      0)))
"#;

    let combined = format!(
        "{}\n{}",
        selfhost_parser_typeinfer_runtime_bundle(),
        harness
    );
    let output = run_with_expanded_stack(NATIVE_HARNESS_STACK_BYTES, move || {
        compile_and_run(&combined)
    });
    let lines: Vec<&str> = output.trim().lines().collect();

    assert_eq!(
        lines,
        vec!["BEGIN", "1", "1002"],
        "selfhost property checker は non-Bool precondition を報告するべき"
    );
}

/// EC-M1-04: selfhost check が複数の typed property binder を同じ scope へ投影すること。
#[test]
fn test_e2e_selfhost_cli_check_accepts_multiple_typed_property_binders() {
    let harness = r#"
(defn main []
  (let [source "(defn pair [x] :property [(for-all [left Int right Int] :precondition [(>= left 0) (>= right 0)] :postcondition (>= result (+ left right)))] x)"
        program (parse-program source)
        analysis (infer-program-analysis program)
        result (check-canonical-properties-with-analysis program analysis)]
    (do
      (print-string "BEGIN\n")
      (print (vector-get result 0))
      (print (vector-get result 1))
      0)))
"#;

    let combined = format!(
        "{}\n{}",
        selfhost_parser_typeinfer_runtime_bundle(),
        harness
    );
    let output = run_with_expanded_stack(NATIVE_HARNESS_STACK_BYTES, move || {
        compile_and_run(&combined)
    });
    let lines: Vec<&str> = output.trim().lines().collect();

    assert_eq!(
        lines,
        vec!["BEGIN", "0", "0"],
        "selfhost property checker は複数の typed binder を同じ scope へ投影するべき"
    );
}

/// EC-M1-04: selfhost check が複数 precondition の全てを Bool preflight すること。
#[test]
fn test_e2e_selfhost_cli_check_rejects_non_bool_second_property_precondition() {
    let harness = r#"
(defn main []
  (let [source "(defn identity [x] :property [(for-all [value Int] :precondition [(>= value 0) (+ value 1)] :postcondition (>= result value))] x)"
        program (parse-program source)
        analysis (infer-program-analysis program)
        result (check-canonical-properties-with-analysis program analysis)]
    (do
      (print-string "BEGIN\n")
      (print (vector-get result 0))
      (print (vector-get result 1))
      0)))
"#;

    let combined = format!(
        "{}\n{}",
        selfhost_parser_typeinfer_runtime_bundle(),
        harness
    );
    let output = run_with_expanded_stack(NATIVE_HARNESS_STACK_BYTES, move || {
        compile_and_run(&combined)
    });
    let lines: Vec<&str> = output.trim().lines().collect();

    assert_eq!(
        lines,
        vec!["BEGIN", "1", "1002"],
        "selfhost property checker は全ての precondition を Bool 必須にするべき"
    );
}

/// EC-M1-04: selfhost check が空の canonical property を成功扱いしないこと。
#[test]
fn test_e2e_selfhost_cli_check_rejects_empty_canonical_property() {
    let harness = r#"
(defn main []
  (let [source "(defn noop [] :property [] true)"
        program (parse-program source)
        analysis (infer-program-analysis program)
        result (check-canonical-properties-with-analysis program analysis)]
    (do
      (print-string "BEGIN\n")
      (print (vector-get result 0))
      (print (vector-get result 1))
      0)))
"#;

    let combined = format!(
        "{}\n{}",
        selfhost_parser_typeinfer_runtime_bundle(),
        harness
    );
    let output = run_with_expanded_stack(NATIVE_HARNESS_STACK_BYTES, move || {
        compile_and_run(&combined)
    });
    let lines: Vec<&str> = output.trim().lines().collect();

    assert_eq!(
        lines,
        vec!["BEGIN", "1", "2007"],
        "selfhost property checker は空の property を成功扱いするべきではない"
    );
}

/// EC-M1-04: selfhost check が literal true の property postcondition を拒否すること。
#[test]
fn test_e2e_selfhost_cli_check_rejects_vacuous_property_postcondition() {
    let harness = r#"
(defn main []
  (let [source "(defn identity [x] :property [(for-all [x Int] :postcondition true)] x)"
        program (parse-program source)
        analysis (infer-program-analysis program)
        result (check-canonical-properties-with-analysis program analysis)]
    (do
      (print-string "BEGIN\n")
      (print (vector-get result 0))
      (print (vector-get result 1))
      0)))
"#;

    let combined = format!(
        "{}\n{}",
        selfhost_parser_typeinfer_runtime_bundle(),
        harness
    );
    let output = run_with_expanded_stack(NATIVE_HARNESS_STACK_BYTES, move || {
        compile_and_run(&combined)
    });
    let lines: Vec<&str> = output.trim().lines().collect();

    assert_eq!(
        lines,
        vec!["BEGIN", "1", "2005"],
        "selfhost property checker は literal true postcondition を成功扱いするべきではない"
    );
}

/// EC-M1-04: selfhost check が typed binder のない property を成功扱いしないこと。
#[test]
fn test_e2e_selfhost_cli_check_rejects_property_without_typed_binder() {
    let harness = r#"
(defn main []
  (let [source "(defn noop [] :property [(for-all [] :postcondition (>= result 0))] true)"
        program (parse-program source)
        analysis (infer-program-analysis program)
        result (check-canonical-properties-with-analysis program analysis)]
    (do
      (print-string "BEGIN\n")
      (print (vector-get result 0))
      (print (vector-get result 1))
      0)))
"#;

    let combined = format!(
        "{}\n{}",
        selfhost_parser_typeinfer_runtime_bundle(),
        harness
    );
    let output = run_with_expanded_stack(NATIVE_HARNESS_STACK_BYTES, move || {
        compile_and_run(&combined)
    });
    let lines: Vec<&str> = output.trim().lines().collect();

    assert_eq!(
        lines,
        vec!["BEGIN", "1", "2007"],
        "selfhost property checker は typed binder のない property を成功扱いするべきではない"
    );
}

/// EC-M1-04: selfhost check が cases=0 の property を成功扱いしないこと。
#[test]
fn test_e2e_selfhost_cli_check_rejects_zero_case_property() {
    let harness = r#"
(defn main []
  (let [source "(defn identity [x] :property [(for-all [x Int] :cases 0 :postcondition (= result x))] x)"
        program (parse-program source)
        analysis (infer-program-analysis program)
        result (check-canonical-properties-with-analysis program analysis)]
    (do
      (print-string "BEGIN\n")
      (print (vector-get result 0))
      (print (vector-get result 1))
      0)))
"#;

    let combined = format!(
        "{}\n{}",
        selfhost_parser_typeinfer_runtime_bundle(),
        harness
    );
    let output = run_with_expanded_stack(NATIVE_HARNESS_STACK_BYTES, move || {
        compile_and_run(&combined)
    });
    let lines: Vec<&str> = output.trim().lines().collect();

    assert_eq!(
        lines,
        vec!["BEGIN", "1", "2007"],
        "selfhost property checker は cases=0 の property を成功扱いするべきではない"
    );
}

/// EC-M1-04: selfhost check が静的に true な integer comparison property を拒否すること。
#[test]
fn test_e2e_selfhost_cli_check_rejects_statically_true_property_postcondition() {
    let harness = r#"
(defn main []
  (let [source "(defn identity [x] :property [(for-all [x Int] :postcondition (= 1 1))] x)"
        program (parse-program source)
        analysis (infer-program-analysis program)
        result (check-canonical-properties-with-analysis program analysis)]
    (do
      (print-string "BEGIN\n")
      (print (vector-get result 0))
      (print (vector-get result 1))
      0)))
"#;

    let combined = format!(
        "{}\n{}",
        selfhost_parser_typeinfer_runtime_bundle(),
        harness
    );
    let output = run_with_expanded_stack(NATIVE_HARNESS_STACK_BYTES, move || {
        compile_and_run(&combined)
    });
    let lines: Vec<&str> = output.trim().lines().collect();

    assert_eq!(
        lines,
        vec!["BEGIN", "1", "2005"],
        "selfhost property checker は静的に true な integer comparison を成功扱いするべきではない"
    );
}

/// EC-M1-04: selfhost check が property binder の名前衝突を拒否すること。
#[test]
fn test_e2e_selfhost_cli_check_rejects_property_binder_name_collisions() {
    let harness = r#"
(defn print-check-result [source]
  (let [program (parse-program source)
        analysis (infer-program-analysis program)
        result (check-canonical-properties-with-analysis program analysis)]
    (do
      (print (vector-get result 0))
      (print (vector-get result 1))
      0)))

(defn main []
  (do
    (print-string "BEGIN\n")
    (print-check-result "(defn pair [left right] :property [(for-all [value Int value Int] :postcondition (= result value))] (+ left right))")
    (print-check-result "(defn identity [value] :property [(for-all [result Int] :postcondition (= result 0))] value)")
    0))
"#;

    let combined = format!(
        "{}\n{}",
        selfhost_parser_typeinfer_runtime_bundle(),
        harness
    );
    let output = run_with_expanded_stack(NATIVE_HARNESS_STACK_BYTES, move || {
        compile_and_run(&combined)
    });
    let lines: Vec<&str> = output.trim().lines().collect();

    assert_eq!(
        lines,
        vec!["BEGIN", "1", "2007", "1", "2007"],
        "selfhost check は binder 名の衝突を structural property diagnostic にするべき"
    );
}

/// EC-M1-04: selfhost check が到達不能な literal false precondition を拒否すること。
#[test]
fn test_e2e_selfhost_cli_check_rejects_unreachable_literal_false_precondition() {
    let harness = r#"
(defn main []
  (let [source "(defn identity [x] :property [(for-all [x Int] :precondition [false] :postcondition (>= result 0))] x)"
        program (parse-program source)
        analysis (infer-program-analysis program)
        result (check-canonical-properties-with-analysis program analysis)]
    (do
      (print-string "BEGIN\n")
      (print (vector-get result 0))
      (print (vector-get result 1))
      0)))
"#;

    let combined = format!(
        "{}\n{}",
        selfhost_parser_typeinfer_runtime_bundle(),
        harness
    );
    let output = run_with_expanded_stack(NATIVE_HARNESS_STACK_BYTES, move || {
        compile_and_run(&combined)
    });
    let lines: Vec<&str> = output.trim().lines().collect();

    assert_eq!(
        lines,
        vec!["BEGIN", "1", "2005"],
        "selfhost property checker は到達不能な literal false precondition を成功扱いするべきではない"
    );
}

/// EC-M1-04: selfhost check が静的に false な integer precondition を拒否すること。
#[test]
fn test_e2e_selfhost_cli_check_rejects_statically_false_property_precondition() {
    let harness = r#"
(defn main []
  (let [source "(defn identity [x] :property [(for-all [x Int] :precondition [(= 1 2)] :postcondition (>= result 0))] x)"
        program (parse-program source)
        analysis (infer-program-analysis program)
        result (check-canonical-properties-with-analysis program analysis)]
    (do
      (print-string "BEGIN\n")
      (print (vector-get result 0))
      (print (vector-get result 1))
      0)))
"#;

    let combined = format!(
        "{}\n{}",
        selfhost_parser_typeinfer_runtime_bundle(),
        harness
    );
    let output = run_with_expanded_stack(NATIVE_HARNESS_STACK_BYTES, move || {
        compile_and_run(&combined)
    });
    let lines: Vec<&str> = output.trim().lines().collect();

    assert_eq!(
        lines,
        vec!["BEGIN", "1", "2005"],
        "selfhost property checker は静的に false な precondition を成功扱いするべきではない"
    );
}

/// EC-M1-04: selfhost check が annotation 付き false precondition を拒否すること。
#[test]
fn test_e2e_selfhost_cli_check_rejects_annotated_false_property_precondition() {
    let harness = r#"
(defn main []
  (let [source "(defn identity [x] :property [(for-all [x Int] :precondition [(: false Bool)] :postcondition (>= result 0))] x)"
        program (parse-program source)
        analysis (infer-program-analysis program)
        result (check-canonical-properties-with-analysis program analysis)]
    (do
      (print-string "BEGIN\n")
      (print (vector-get result 0))
      (print (vector-get result 1))
      0)))
"#;

    let combined = format!(
        "{}\n{}",
        selfhost_parser_typeinfer_runtime_bundle(),
        harness
    );
    let output = run_with_expanded_stack(NATIVE_HARNESS_STACK_BYTES, move || {
        compile_and_run(&combined)
    });
    let lines: Vec<&str> = output.trim().lines().collect();

    assert_eq!(
        lines,
        vec!["BEGIN", "1", "2005"],
        "selfhost property checker は annotation 付き false precondition を成功扱いするべきではない"
    );
}

/// EC-M1-04: selfhost check が compound false property precondition を拒否すること。
#[test]
fn test_e2e_selfhost_cli_check_rejects_compound_false_property_precondition() {
    let harness = r#"
(defn main []
  (let [source "(defn identity [x] :property [(for-all [x Int] :precondition [(and false true)] :postcondition (>= result 0))] x)"
        program (parse-program source)
        analysis (infer-program-analysis program)
        result (check-canonical-properties-with-analysis program analysis)]
    (do
      (print-string "BEGIN\n")
      (print (vector-get result 0))
      (print (vector-get result 1))
      0)))
"#;

    let combined = format!(
        "{}\n{}",
        selfhost_parser_typeinfer_runtime_bundle(),
        harness
    );
    let output = run_with_expanded_stack(NATIVE_HARNESS_STACK_BYTES, move || {
        compile_and_run(&combined)
    });
    let lines: Vec<&str> = output.trim().lines().collect();

    assert_eq!(
        lines,
        vec!["BEGIN", "1", "2005"],
        "selfhost property checker は compound false precondition を成功扱いするべきではない"
    );
}

/// EC-M1-04: selfhost check が unary not の false property precondition を拒否すること。
#[test]
fn test_e2e_selfhost_cli_check_rejects_unary_not_true_property_precondition() {
    let harness = r#"
(defn main []
  (let [source "(defn identity [x] :property [(for-all [x Int] :precondition [(not true)] :postcondition (>= result 0))] x)"
        program (parse-program source)
        analysis (infer-program-analysis program)
        result (check-canonical-properties-with-analysis program analysis)]
    (do
      (print-string "BEGIN\n")
      (print (vector-get result 0))
      (print (vector-get result 1))
      0)))
"#;

    let combined = format!(
        "{}\n{}",
        selfhost_parser_typeinfer_runtime_bundle(),
        harness
    );
    let output = run_with_expanded_stack(NATIVE_HARNESS_STACK_BYTES, move || {
        compile_and_run(&combined)
    });
    let lines: Vec<&str> = output.trim().lines().collect();

    assert_eq!(
        lines,
        vec!["BEGIN", "1", "2005"],
        "selfhost property checker は unary not の false precondition を成功扱いするべきではない"
    );
}

/// EC-M1-04: selfhost check が負の cases option を成功扱いしないこと。
#[test]
fn test_e2e_selfhost_cli_check_rejects_negative_property_cases() {
    let harness = r#"
(defn main []
  (let [source "(defn identity [x] :property [(for-all [x Int] :cases -1 :postcondition (= result x))] x)"
        program (parse-program source)
        analysis (infer-program-analysis program)
        result (check-canonical-properties-with-analysis program analysis)]
    (do
      (print-string "BEGIN\n")
      (print (vector-get result 0))
      (print (vector-get result 1))
      0)))
"#;

    let combined = format!(
        "{}\n{}",
        selfhost_parser_typeinfer_runtime_bundle(),
        harness
    );
    let output = run_with_expanded_stack(NATIVE_HARNESS_STACK_BYTES, move || {
        compile_and_run(&combined)
    });
    let lines: Vec<&str> = output.trim().lines().collect();

    assert_eq!(
        lines,
        vec!["BEGIN", "1", "2007"],
        "selfhost property checker は負の cases option を成功扱いするべきではない"
    );
}

/// EC-M1-04: selfhost check が非数値の cases option を成功扱いしないこと。
#[test]
fn test_e2e_selfhost_cli_check_rejects_non_numeric_property_cases() {
    let harness = r#"
(defn main []
  (let [source "(defn identity [x] :property [(for-all [x Int] :cases false :postcondition (= result x))] x)"
        program (parse-program source)
        analysis (infer-program-analysis program)
        result (check-canonical-properties-with-analysis program analysis)]
    (do
      (print-string "BEGIN\n")
      (print (vector-get result 0))
      (print (vector-get result 1))
      0)))
"#;

    let combined = format!(
        "{}\n{}",
        selfhost_parser_typeinfer_runtime_bundle(),
        harness
    );
    let output = run_with_expanded_stack(NATIVE_HARNESS_STACK_BYTES, move || {
        compile_and_run(&combined)
    });
    let lines: Vec<&str> = output.trim().lines().collect();

    assert_eq!(
        lines,
        vec!["BEGIN", "1", "2007"],
        "selfhost property checker は非数値の cases option を成功扱いするべきではない"
    );
}

/// EC-M1-04: selfhost check が未知の property option を成功扱いしないこと。
#[test]
fn test_e2e_selfhost_cli_check_rejects_unknown_property_option() {
    let harness = r#"
(defn main []
  (let [source "(defn identity [x] :property [(for-all [x Int] :unknown true :cases 1 :postcondition (= result x))] x)"
        program (parse-program source)
        analysis (infer-program-analysis program)
        result (check-canonical-properties-with-analysis program analysis)]
    (do
      (print-string "BEGIN\n")
      (print (vector-get result 0))
      (print (vector-get result 1))
      0)))
"#;

    let combined = format!(
        "{}\n{}",
        selfhost_parser_typeinfer_runtime_bundle(),
        harness
    );
    let output = run_with_expanded_stack(NATIVE_HARNESS_STACK_BYTES, move || {
        compile_and_run(&combined)
    });
    let lines: Vec<&str> = output.trim().lines().collect();

    assert_eq!(
        lines,
        vec!["BEGIN", "1", "2007"],
        "selfhost property checker は未知の option を成功扱いするべきではない"
    );
}

/// EC-M1-04: selfhost check が各 property option 境界の未知 token を拒否すること。
#[test]
fn test_e2e_selfhost_cli_check_rejects_unknown_property_option_at_each_boundary() {
    let harness = r#"
(defn print-check-result [source]
    (let [program (parse-program source)
        analysis (infer-program-analysis program)
        result (check-canonical-properties-with-analysis program analysis)]
    (do
      (print (vector-get result 0))
      (print (vector-get result 1))
      0)))

(defn main []
  (do
    (print-string "BEGIN\n")
    (print-check-result "(defn identity [x] :property [(for-all [x Int] :unknown true :cases 1 :postcondition (= result x))] x)")
    (print-check-result "(defn identity [x] :property [(for-all [x Int] :cases 1 :unknown true :postcondition (= result x))] x)")
    (print-check-result "(defn identity [x] :property [(for-all [x Int] :cases 1 :precondition [(>= x 0)] :unknown true :postcondition (= result x))] x)")
    (print-check-result "(defn identity [x] :property [(for-all [x Int] :cases 1 :seed 7 :unknown true :postcondition (= result x))] x)")
    (print-check-result "(defn identity [x] :property [(for-all [x Int] :cases 1 :postcondition (= result x) :unknown true)] x)")
    (print-check-result "(defn identity [x] :property [(for-all [x Int] :cases-extra true :postcondition (= result x))] x)")
    0))
"#;

    let combined = format!(
        "{}\n{}",
        selfhost_parser_typeinfer_runtime_bundle(),
        harness
    );
    let output = run_with_expanded_stack(NATIVE_HARNESS_STACK_BYTES, move || {
        compile_and_run(&combined)
    });
    let lines: Vec<&str> = output.trim().lines().collect();

    assert_eq!(
        lines,
        vec![
            "BEGIN", "1", "2007", "1", "2007", "1", "2007", "1", "2007", "1", "2007", "1", "2007",
        ],
        "selfhost property checker は各 option 境界の未知 token を拒否するべき"
    );
}

/// EC-M1-04: selfhost check が property option の値欠落を成功扱いしないこと。
#[test]
fn test_e2e_selfhost_cli_check_rejects_missing_property_option_value() {
    let harness = r#"
(defn print-check-result [source]
  (let [program (parse-program source)
        analysis (infer-program-analysis program)
        result (check-canonical-properties-with-analysis program analysis)]
    (do
      (print (vector-get result 0))
      (print (vector-get result 1))
      0)))

(defn main []
  (do
    (print-string "BEGIN\n")
    (print-check-result "(defn identity [x] :property [(for-all [x Int] :cases 1 :seed :postcondition (= result x))] x)")
    (print-check-result "(defn identity [x] :property [(for-all [x Int] :cases 1 :shrink :postcondition (= result x))] x)")
    (print-check-result "(defn identity [x] :property [(for-all [x Int] :precondition :postcondition (= result x))] x)")
    (print-check-result "(defn identity [x] :property [(for-all [x Int] :postcondition :cases 1)] x)")
    0))
"#;

    let combined = format!(
        "{}\n{}",
        selfhost_parser_typeinfer_runtime_bundle(),
        harness
    );
    let output = run_with_expanded_stack(NATIVE_HARNESS_STACK_BYTES, move || {
        compile_and_run(&combined)
    });
    let lines: Vec<&str> = output.trim().lines().collect();

    assert_eq!(
        lines,
        vec!["BEGIN", "1", "2007", "1", "2007", "1", "2007", "1", "2007"],
        "selfhost property checker は property option の値欠落を成功扱いするべきではない"
    );
}

/// EC-M1-04: selfhost parser の delimiter diagnostics が unclosed property expression を見落とさないこと。
#[test]
fn test_e2e_selfhost_parser_delimiter_diagnostics_rejects_unclosed_property_expression() {
    let harness = r#"
(defn main []
  (let [source "(defn identity [x] :property [(for-all [x Int] :postcondition (= result x))] x"
        diagnostics (parse-delimiter-diagnostics (tokenize-with-spans source) source)]
    (do
      (print (vector-length diagnostics))
      (print (vector-get (vector-get diagnostics 0) 1))
      0)))
"#;

    let combined = format!("{}\n{}", selfhost_parser_runtime_bundle(), harness);
    let output = run_with_expanded_stack(NATIVE_HARNESS_STACK_BYTES, move || {
        compile_and_run(&combined)
    });
    let lines: Vec<&str> = output.trim().lines().collect();

    assert_eq!(lines, vec!["1", "1001"], "selfhost delimiter diagnostics は unclosed property expression を拒否するべき");
}

/// EC-M1-02: selfhost test が canonical :case の型エラーを実行前に拒否すること
#[test]
fn test_e2e_selfhost_cli_test_rejects_invalid_canonical_case() {
    let harness = r#"
(defn main []
  (let [src "(defn noop [] :case [(expect \"a\" \"a\")] true)"]
    (do
      (print-string "BEGIN\n")
      (print (run-test-source src 0))
      0)))
"#;

    let combined = format!("{}\n{}", selfhost_cli_runtime_bundle(), harness);
    let output = run_with_expanded_stack(NATIVE_HARNESS_STACK_BYTES, move || {
        compile_and_run(&combined)
    });
    let lines: Vec<&str> = output.trim().lines().collect();

    assert_eq!(
        lines,
        vec![
            "BEGIN",
            "examples:0",
            "invariants:0",
            "cases:1",
            "failures:1",
            "diagnostics:1,LS1002",
            "2",
        ],
        "selfhost test は canonical case の unsupported type を評価せず拒否するべき"
    );
}

/// TEST-CLI-02-O2g: selfhost runner が legacy contract の順序と source span を保持すること
#[test]
fn test_e2e_selfhost_test_runner_preserves_contract_form_order_and_spans() {
    let harness = r#"
(defn main []
  (let [src "(defn succ [x] :example [(succ 0)] :invariant (= result (+ x 1)) :example [(succ 1)] (+ x 1))"
        forms (extract-contract-forms src)
        form0 (vector-get forms 0)
        form1 (vector-get forms 1)
        form2 (vector-get forms 2)]
    (do
      (print (vector-length forms))
      (print (vector-get form0 0))
      (print (vector-length (vector-get form0 2)))
      (print (vector-get form0 3))
      (print (vector-get form0 4))
      (print (vector-get form1 0))
      (print (vector-length (vector-get form1 2)))
      (print (vector-get form1 3))
      (print (vector-get form1 4))
      (print (vector-get form2 0))
      (print (vector-length (vector-get form2 2)))
      (print (vector-get form2 3))
      (print (vector-get form2 4))
      0)))
"#;

    let combined = format!("{}\n{}", selfhost_test_runner_runtime_bundle(), harness);
    let output = run_with_expanded_stack(NATIVE_HARNESS_STACK_BYTES, move || {
        compile_and_run(&combined)
    });
    let lines: Vec<&str> = output.trim().lines().collect();

    assert_eq!(
        lines,
        vec!["3", "1", "1", "15", "34", "2", "1", "35", "64", "1", "1", "65", "84"],
        "selfhost contract inventory は legacy directive の順序・grouping・source span を保持すべき"
    );
}

/// EC-M1-02: selfhost raw inventory が legacy/canonical contract の順序と span を保持すること
#[test]
fn test_e2e_selfhost_contract_inventory_includes_canonical_forms() {
    let harness = r#"
(defn main []
  (let [src "(defn f [x] :example [(f 1)] :case [(expect (f 1) 2)] :assert [(= 1 1)] :property [(for-all [x Int] :cases 3 :postcondition (= result x))] :invariant (= result (+ x 1)) (+ x 1))"
        forms (extract-contract-forms src)
        form0 (vector-get forms 0)
        form1 (vector-get forms 1)
        form2 (vector-get forms 2)
        form3 (vector-get forms 3)
        form4 (vector-get forms 4)]
    (do
      (print (vector-length forms))
      (print (vector-get form0 0))
      (print (vector-get form1 0))
      (print (vector-get form2 0))
      (print (vector-get form3 0))
      (print (vector-get form4 0))
      (print-string (vector-get (vector-get form1 2) 0))
      (print-string "\n")
      (print-string (vector-get (vector-get form2 2) 0))
      (print-string "\n")
      (print-string (vector-get (vector-get form3 2) 0))
      (print-string "\n")
      (print (vector-length (vector-get form4 2)))
      (print (if (< (vector-get form0 3) (vector-get form0 4)) 1 0))
      (print (if (< (vector-get form1 3) (vector-get form1 4)) 1 0))
      (print (if (< (vector-get form2 3) (vector-get form2 4)) 1 0))
      (print (if (< (vector-get form3 3) (vector-get form3 4)) 1 0))
      (print (if (< (vector-get form4 3) (vector-get form4 4)) 1 0))
      0)))
"#;

    let combined = format!("{}\n{}", selfhost_test_runner_runtime_bundle(), harness);
    let output = run_with_expanded_stack(NATIVE_HARNESS_STACK_BYTES, move || {
        compile_and_run(&combined)
    });
    let lines: Vec<&str> = output.trim().lines().collect();
    assert_eq!(
        lines,
        vec![
            "5",
            "1",
            "4",
            "3",
            "5",
            "2",
            "(expect (f 1) 2)",
            "(= 1 1)",
            "(for-all [x Int] :cases 3 :postcondition (= result x))",
            "1",
            "1",
            "1",
            "1",
            "1",
            "1",
        ],
        "raw contract inventory は全 form kind の順序、payload、source span を保持するべき"
    );
}

/// EC-M1-02: parser-owned ordered forms を canonical/pending suite projection に分けること
#[test]
fn test_e2e_selfhost_parser_contract_suite_projection_separates_legacy_forms() {
    let harness = r#"
(defn main []
  (let [src "(defn f [x] :example [(f 1)] :case [(expect (f 1) 2)] :assert [(= 1 1)] :property [(for-all [x Int] :cases 3 :postcondition (= result x))] :invariant (= result (+ x 1)) (+ x 1))"
        suites (extract-parser-contract-suites src)
        suite (vector-get suites 0)
        ordered (vector-get suite 1)
        executable (vector-get suite 2)
        pending (vector-get suite 3)]
    (do
      (print (vector-length suites))
      (print (vector-length ordered))
      (print (vector-length executable))
      (print (vector-length pending))
      (print (vector-get (vector-get executable 0) 0))
      (print (vector-get (vector-get executable 1) 0))
      (print (vector-get (vector-get executable 2) 0))
      (print (vector-get (vector-get pending 0) 0))
      (print (vector-get (vector-get pending 1) 0))
      (print (vector-length (vector-get (vector-get executable 0) 1)))
      (print (vector-length (vector-get (vector-get executable 1) 1)))
      (print-string (vector-get (vector-get ordered 3) 1))
      (print-string "\n")
      (print (vector-length (vector-get (vector-get executable 2) 1)))
      (print (vector-get (vector-get (vector-get executable 2) 1) 4))
      (print (vector-get (vector-get (vector-get pending 1) 1) 0))
      0)))
"#;

    let combined = format!("{}\n{}", selfhost_test_runner_runtime_bundle(), harness);
    let output = run_with_expanded_stack(NATIVE_HARNESS_STACK_BYTES, move || {
        compile_and_run(&combined)
    });
    let lines: Vec<&str> = output.trim().lines().collect();
    assert_eq!(
        lines,
        vec![
            "1",
            "5",
            "3",
            "2",
            "4",
            "3",
            "5",
            "1",
            "2",
            "1",
            "1",
            "(for-all [x Int] :cases 3 :postcondition (= result x))",
            "5",
            "0",
            "5",
        ],
        "parser-owned contract suite は canonical forms と legacy migration forms を分離し、順序を保持するべき"
    );
}

/// EC-M1-02: parser-owned ContractSuite が canonical :property payload を typed shape へ投影すること
#[test]
fn test_e2e_selfhost_parser_contract_suite_projects_typed_property_payload() {
    let harness = r#"
(defn main []
  (let [src "(defn identity [x] :property [(for-all [value Int] :cases 3 :postcondition (= result value))] x)"
        suites (extract-parser-contract-suites src)
        suite (vector-get suites 0)
        executable (vector-get suite 2)
        property (vector-get executable 0)
        payload (vector-get property 1)
        binders (vector-get payload 0)
        binder (vector-get binders 0)
        preconditions (vector-get payload 1)
        postcondition (vector-get payload 2)
        sampling (vector-get payload 3)]
    (do
      (print (vector-length suites))
      (print (vector-length executable))
      (print (vector-get property 0))
      (print (vector-length binders))
      (print (if (= (vector-get binder 0) (name-hash "value" 0 5)) 1 0))
      (print (if (= (vector-get binder 1) (name-hash "Int" 0 3)) 1 0))
      (print (vector-get binder 2))
      (print (vector-length preconditions))
      (print (vector-get postcondition 0))
      (print (vector-get sampling 0))
      (print (vector-get sampling 1))
      (print-string (vector-get sampling 2))
      (print-string "\n")
      (print (vector-get sampling 3))
      (print (vector-get sampling 4))
      0)))
"#;

    let combined = format!("{}\n{}", selfhost_test_runner_runtime_bundle(), harness);
    let output = run_with_expanded_stack(NATIVE_HARNESS_STACK_BYTES, move || {
        compile_and_run(&combined)
    });
    let lines: Vec<&str> = output.trim().lines().collect();

    assert_eq!(
        lines,
        vec![
            "1", "1", "5", "1", "1", "1", "1", "0", "5", "3", "0",
            "type-directed-splitmix64-v1", "1", "0",
        ],
        "parser-owned ContractSuite は canonical property の typed binder/predicate/sampling を raw string のまま返すべきではない"
    );
}

/// EC-M1-02: parser-owned ContractSuite が canonical property precondition を AST へ投影すること
#[test]
fn test_e2e_selfhost_parser_contract_suite_projects_property_precondition() {
    let harness = r#"
(defn main []
  (let [src "(defn identity [x] :property [(for-all [value Int] :cases 3 :precondition [(>= value 0)] :postcondition (= result value))] x)"
        suites (extract-parser-contract-suites src)
        property (vector-get (vector-get (vector-get suites 0) 2) 0)
        payload (vector-get property 1)
        preconditions (vector-get payload 1)
        predicate (vector-get preconditions 0)]
    (do
      (print (vector-length suites))
      (print (vector-length preconditions))
      (print (vector-get predicate 0))
      (print (vector-get (vector-get payload 3) 0))
      (print (vector-get payload 4))
      0)))
"#;

    let combined = format!("{}\n{}", selfhost_test_runner_runtime_bundle(), harness);
    let output = run_with_expanded_stack(NATIVE_HARNESS_STACK_BYTES, move || {
        compile_and_run(&combined)
    });
    let lines: Vec<&str> = output.trim().lines().collect();

    assert_eq!(
        lines,
        vec!["1", "1", "5", "3", "0"],
        "parser-owned ContractSuite は property precondition を raw payload のまま破棄せず AST として保持するべき"
    );
}

/// EC-M1-02: parser-owned ContractSuite が複数の typed property binder を source 順に保持すること
#[test]
fn test_e2e_selfhost_parser_contract_suite_projects_multiple_property_binders() {
    let harness = r#"
(defn main []
  (let [src "(defn pair [x] :property [(for-all [left Int right Int] :cases 2 :precondition [(>= left 0)] :postcondition (= result left))] x)"
        suites (extract-parser-contract-suites src)
        suite (vector-get suites 0)
        suite-executable (vector-get suite 2)
        suite-transformed (vector-get suite-executable 0)
        payload (vector-get suite-transformed 1)
        binders (vector-get payload 0)
        left (vector-get binders 0)
        right (vector-get binders 1)
        sampling (vector-get payload 3)]
    (do
      (print (vector-length suite-executable))
      (print (vector-get suite-transformed 0))
      (print (vector-length binders))
      (print (if (= (vector-get left 0) (name-hash "left" 0 4)) 1 0))
      (print (if (= (vector-get left 1) (name-hash "Int" 0 3)) 1 0))
      (print (if (= (vector-get right 0) (name-hash "right" 0 5)) 1 0))
      (print (if (= (vector-get right 1) (name-hash "Int" 0 3)) 1 0))
      (print (vector-length (vector-get payload 1)))
      (print (vector-get (vector-get payload 3) 0))
      (print (vector-get payload 4))
      0)))
"#;

    let combined = format!("{}\n{}", selfhost_test_runner_runtime_bundle(), harness);
    let output = run_with_expanded_stack(NATIVE_HARNESS_STACK_BYTES, move || {
        compile_and_run(&combined)
    });
    let lines: Vec<&str> = output.trim().lines().collect();

    assert_eq!(
        lines,
        vec!["1", "5", "2", "1", "1", "1", "1", "1", "2", "0"],
        "parser-owned ContractSuite は複数 typed binder の source order と sampling を保持するべき"
    );
}

/// TEST-CLI-02-O2d: selfhost/src/Tools/Test/TestRunner.ls が supported subset の metadata suite を実行できること
#[test]
#[ignore]
fn test_e2e_selfhost_test_runner_executes_supported_metadata_suite() {
    let harness = r#"
(defn main []
  (let [src "(defn abs [x] :example [(= (abs 5) 5) (= (abs (- 0 7)) 7)] :invariant (>= result 0) (if (< x 0) (- 0 x) x))"
        suite (generate-tests-from-source src)
        examples (vector-get suite 0)
        invariants (vector-get suite 1)
        example0 (vector-get examples 0)
        example1 (vector-get examples 1)
        invariant0 (vector-get invariants 0)]
    (do
      (print (vector-length examples))
      (print (vector-length invariants))
      (print (vector-get example0 1))
      (print (vector-get example1 1))
      (print (vector-get invariant0 1))
      (print (vector-get invariant0 2))
      0)))
"#;

    let combined = format!("{}\n{}", selfhost_cli_runtime_bundle(), harness);
    let output = compile_and_run(&combined);
    let lines: Vec<&str> = output.trim().lines().collect();

    assert_eq!(
        lines,
        vec!["2", "1", "1", "1", "1", "5"],
        "generate-tests-from-source は 2 example + 1 invariant を実行し、invariant は 5 サンプル通過を返すべき"
    );
}

/// TEST-CLI-02-O3: selfhost/src/App/Cli.ls の run-test-source が supported subset の metadata を成功終了できること
#[test]
#[ignore]
fn test_e2e_selfhost_cli_test_source_metadata_pass() {
    let harness = r#"
(defn main []
  (let [src "(defn abs [x] :example [(= (abs 5) 5) (= (abs (- 0 7)) 7)] :invariant (>= result 0) (if (< x 0) (- 0 x) x))"]
    (do
      (print (run-test-source src 0))
      0)))
"#;

    let combined = format!("{}\n{}", selfhost_cli_runtime_bundle(), harness);
    let output = compile_and_run(&combined);
    let lines: Vec<&str> = output.trim().lines().collect();

    assert_eq!(
        lines,
        vec!["examples:2", "invariants:1", "failures:0", "0"],
        "run-test-source は passing metadata suite に labeled summary と success=0 を返すべき"
    );
}

/// TEST-CLI-02-O4: selfhost/src/App/Cli.ls の run-test-source が failing example を runtime error にできること
#[test]
#[ignore]
fn test_e2e_selfhost_cli_test_source_metadata_fail() {
    let harness = r#"
(defn main []
  (let [src "(defn dec [x] :example [(= (dec 2) 3)] (- x 1))"]
    (do
      (print (run-test-source src 0))
      0)))
"#;

    let combined = format!("{}\n{}", selfhost_cli_runtime_bundle(), harness);
    let output = compile_and_run(&combined);
    let lines: Vec<&str> = output.trim().lines().collect();

    assert_eq!(
        lines,
        vec!["examples:1", "invariants:0", "failures:1", "2"],
        "run-test-source は failing example に labeled summary と runtime-error=2 を返すべき"
    );
}

/// TEST-CLI-02-O5: selfhost/src/App/Cli.ls の run-test が file-path 経由の metadata suite も実行できること
#[test]
#[ignore]
fn test_e2e_selfhost_cli_test_file_handler_metadata_pass() {
    let project_root = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..");
    let dir = project_root.join("target").join(format!(
        "e2e_selfhost_cli_test_metadata_{}",
        std::process::id()
    ));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    std::fs::write(
        dir.join("input.ls"),
        "(defn abs [x] :example [(= (abs 5) 5) (= (abs (- 0 7)) 7)] :invariant (>= result 0) (if (< x 0) (- 0 x) x))",
    )
    .unwrap();

    let harness = r#"
(defn main []
  (do
    (print (run-test "input.ls" 0))
    0))
"#;

    let combined = format!("{}\n{}", selfhost_cli_runtime_bundle(), harness);
    let output = compile_and_run_with_dir(&combined, &dir);
    let _ = std::fs::remove_dir_all(&dir);
    let lines: Vec<&str> = output.trim().lines().collect();

    assert_eq!(
        lines,
        vec!["examples:2", "invariants:1", "failures:0", "0"],
        "run-test は file-path 経由でも labeled summary を返せるべき"
    );
}

/// TEST-CLI-02-P: selfhost/src/App/Cli.ls の run-review-source が review title/body を返せること
#[test]
#[ignore]
fn test_e2e_selfhost_cli_review_source_core() {
    let harness = r#"
(defn main []
  (do
    (print (run-review-source "(defn main [] (let [x 42] 0))" 0))
    0))
"#;

    let combined = format!("{}\n{}", selfhost_cli_runtime_bundle(), harness);
    let output = compile_and_run(&combined);
    let lines: Vec<&str> = output.trim().lines().collect();

    assert_eq!(
        lines,
        vec![
            "1",
            "unused-let",
            "diagnostics:1,first-body:let binding x is not used",
            "warning",
            "L0001@1:1",
            "0"
        ],
        "run-review-source は review count/title/body/severity/code-location と success=0 を返すべき"
    );
}

/// TEST-CLI-02-Q: selfhost/src/App/Cli.ls の run-review が file-path から review title/body を返せること
#[test]
#[ignore]
fn test_e2e_selfhost_cli_review_file_handler() {
    let dir = std::env::temp_dir().join(format!(
        "lsharp_test_cli_review_file_{}",
        std::process::id()
    ));
    std::fs::create_dir_all(&dir).unwrap();
    std::fs::write(dir.join("input.ls"), "(defn main [] (let [x 42] 0))").unwrap();

    let harness = r#"
(defn main []
  (do
    (print (run-review "input.ls" 0))
    0))
"#;

    let combined = format!("{}\n{}", selfhost_cli_runtime_bundle(), harness);
    let output = compile_and_run_with_dir(&combined, &dir);
    let _ = std::fs::remove_dir_all(&dir);
    let lines: Vec<&str> = output.trim().lines().collect();

    assert_eq!(
        lines,
        vec![
            "1",
            "unused-let",
            "diagnostics:1,first-body:let binding x is not used",
            "warning",
            "L0001@1:1",
            "0"
        ],
        "run-review は review count/title/body/severity/code-location と success=0 を返すべき"
    );
}

/// TEST-CLI-02-Q2: selfhost/src/App/Cli.ls の run-review-source が empty-do rule も返せること
#[test]
#[ignore]
fn test_e2e_selfhost_cli_review_source_empty_do() {
    let harness = r#"
(defn main []
  (do
    (print (run-review-source "(defn main [] (do))" 0))
    0))
"#;

    let combined = format!("{}\n{}", selfhost_cli_runtime_bundle(), harness);
    let output = compile_and_run(&combined);
    let lines: Vec<&str> = output.trim().lines().collect();

    assert_eq!(
        lines,
        vec![
            "1",
            "empty-do",
            "diagnostics:1,first-body:do block has no expressions",
            "warning",
            "L0002@1:1",
            "0",
        ],
        "run-review-source は empty-do rule でも review summary/severity/code-location を返すべき"
    );
}

/// TEST-CLI-02-Q3: selfhost/src/App/Cli.ls の run-review-source が schema-object JSON を返せること
#[test]
#[ignore]
fn test_e2e_selfhost_cli_review_source_json_snapshot() {
    let harness = r#"
(defn main []
  (do
    (print (run-review-source "(defn first [] (let [unused 42] 0)) (defn second [] (do))" 1))
    0))
"#;

    let combined = format!("{}\n{}", selfhost_cli_runtime_bundle(), harness);
    let output = compile_and_run(&combined);
    let lines: Vec<&str> = output.trim().lines().collect();
    let actual: Value =
        serde_json::from_str(lines[0]).expect("run-review-source json line は valid JSON");

    assert_eq!(
        actual,
        doctools_json_snapshot("review-schema-object.json"),
        "run-review-source json output は representative review schema snapshot と一致するべき"
    );
    assert_eq!(
        lines[1], "0",
        "run-review-source json mode は success=0 を返すべき"
    );
}

/// TEST-CLI-02-R: selfhost/src/App/Cli.ls の run-doc-source が DocTools.generate を呼べること
#[test]
#[ignore]
fn test_e2e_selfhost_cli_doc_source_core() {
    let harness = r#"
(defn main []
  (do
    (print (run-doc-source "(defn main [] 42)" 0))
    0))
"#;

    let combined = format!("{}\n{}", selfhost_cli_runtime_bundle(), harness);
    let output = compile_and_run(&combined);
    let lines: Vec<&str> = output.trim().lines().collect();

    assert_eq!(
        lines,
        vec!["module-global", "functions:1,types:0,first-fn:main", "0"],
        "run-doc-source は deterministic な title/body と success=0 を返すべき"
    );
}

/// TEST-CLI-02-R2: run-doc-source output を snapshot に固定すること
#[test]
#[ignore]
fn test_e2e_selfhost_cli_doc_source_snapshot() {
    let harness = r#"
(defn main []
  (do
    (print (run-doc-source "(defn main [] 42)" 0))
    0))
"#;

    let combined = format!("{}\n{}", selfhost_cli_runtime_bundle(), harness);
    let output = compile_and_run(&combined);

    assert_cli_text_snapshot(
        &output,
        "doc-source-basic.txt",
        "run-doc-source output は representative text snapshot と一致するべき",
    );
}

/// TEST-CLI-02-R3: selfhost/src/App/Cli.ls の run-doc-source が schema-object JSON を返せること
#[test]
#[ignore]
fn test_e2e_selfhost_cli_doc_source_json_snapshot() {
    let harness = r#"
(defn main []
  (do
    (print (run-doc-source "(module Demo (defn add [x y] :params [(x \"left\") (y \"right\")] :returns \"sum\" :doc \"Add two ints\" :example [(add 1 2)] (+ x y)) (type Doc Int) (type-alias Alias Int))" 1))
    0))
"#;

    let combined = format!("{}\n{}", selfhost_cli_runtime_bundle(), harness);
    let output = compile_and_run(&combined);
    let lines: Vec<&str> = output.trim().lines().collect();
    let actual: Value =
        serde_json::from_str(lines[0]).expect("run-doc-source json line は valid JSON");

    assert_eq!(
        actual,
        doctools_json_snapshot("doc-output-schema-object.json"),
        "run-doc-source json output は representative doc-output schema snapshot と一致するべき"
    );
    assert_eq!(
        lines[1], "0",
        "run-doc-source json mode は success=0 を返すべき"
    );
}

/// TEST-CLI-02-S: selfhost/src/App/Cli.ls の run-doc が file-path から source を読めること
#[test]
#[ignore]
fn test_e2e_selfhost_cli_doc_file_handler() {
    let dir = std::env::temp_dir().join(format!("lsharp_test_cli_doc_file_{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    std::fs::write(dir.join("input.ls"), "(defn main [] 42)").unwrap();

    let harness = r#"
(defn main []
  (do
    (print (run-doc "input.ls" 0))
    0))
"#;

    let combined = format!("{}\n{}", selfhost_cli_runtime_bundle(), harness);
    let output = compile_and_run_with_dir(&combined, &dir);
    let _ = std::fs::remove_dir_all(&dir);
    let lines: Vec<&str> = output.trim().lines().collect();

    assert_eq!(
        lines,
        vec!["module-global", "functions:1,types:0,first-fn:main", "0"],
        "run-doc は file-path 経由でも deterministic な title/body と success=0 を返すべき"
    );
}

/// TEST-CLI-02-T: selfhost/src/App/Cli.ls の run-doc-ack が file-path から source を読めること
#[test]
#[ignore]
fn test_e2e_selfhost_cli_doc_ack_file_handler() {
    let dir = std::env::temp_dir().join(format!(
        "lsharp_test_cli_doc_ack_file_{}",
        std::process::id()
    ));
    std::fs::create_dir_all(&dir).unwrap();
    std::fs::write(dir.join("input.ls"), "(defn main [] 42)").unwrap();

    let harness = r#"
(defn main []
  (do
    (print (run-doc-ack "input.ls" 0))
    0))
"#;

    let combined = format!("{}\n{}", selfhost_cli_runtime_bundle(), harness);
    let output = compile_and_run_with_dir(&combined, &dir);
    let _ = std::fs::remove_dir_all(&dir);
    let lines: Vec<&str> = output.trim().lines().collect();

    assert_eq!(
        lines,
        vec![
            "ack:recorded",
            "module-global",
            "functions:1,types:0,first-fn:main",
            "; Doc-Reviewed-By: anonymous",
            "0",
        ],
        "run-doc-ack は ack status と title/body と trailer と success=0 を返すべき"
    );
}

/// TEST-CLI-02-T2: selfhost/src/App/Cli.ls の run-doc-ack が trailer-only mode を返せること
#[test]
#[ignore]
fn test_e2e_selfhost_cli_doc_ack_file_handler_trailer_only() {
    let dir = std::env::temp_dir().join(format!(
        "lsharp_test_cli_doc_ack_trailer_only_{}",
        std::process::id()
    ));
    std::fs::create_dir_all(&dir).unwrap();
    std::fs::write(dir.join("input.ls"), "(defn main [] 42)").unwrap();

    let harness = r#"
(defn main []
  (do
    (print (run-doc-ack "input.ls" 1))
    0))
"#;

    let combined = format!("{}\n{}", selfhost_cli_runtime_bundle(), harness);
    let output = compile_and_run_with_dir(&combined, &dir);
    let _ = std::fs::remove_dir_all(&dir);
    let lines: Vec<&str> = output.trim().lines().collect();

    assert_eq!(
        lines,
        vec!["; Doc-Reviewed-By: anonymous", "0"],
        "run-doc-ack trailer-only mode は comment trailer のみを返すべき"
    );
}

/// TEST-CLI-02-U: selfhost/src/App/Cli.ls の run-doc-check が file-path から source を読めること
#[test]
#[ignore]
fn test_e2e_selfhost_cli_doc_check_file_handler() {
    let dir = std::env::temp_dir().join(format!(
        "lsharp_test_cli_doc_check_file_{}",
        std::process::id()
    ));
    std::fs::create_dir_all(&dir).unwrap();
    std::fs::write(dir.join("input.ls"), "(defn main [] 42)").unwrap();

    let harness = r#"
(defn main []
  (do
    (print (run-doc-check "input.ls" 0))
    0))
"#;

    let combined = format!("{}\n{}", selfhost_cli_runtime_bundle(), harness);
    let output = compile_and_run_with_dir(&combined, &dir);
    let _ = std::fs::remove_dir_all(&dir);
    let lines: Vec<&str> = output.trim().lines().collect();

    assert_eq!(
        lines,
        vec![
            "status:ok",
            "module-global",
            "functions:1,types:0,first-fn:main",
            "; Doc-Review-Status: Passed",
            "; Doc-Reviewed-By: anonymous",
            "0",
        ],
        "run-doc-check は status と title/body と trailer と success=0 を返すべき"
    );
}

/// TEST-CLI-02-U2: selfhost/src/App/Cli.ls の run-doc-check strict mode が valid trailer を受理すること
#[test]
#[ignore]
fn test_e2e_selfhost_cli_doc_check_file_handler_strict_success() {
    let dir = std::env::temp_dir().join(format!(
        "lsharp_test_cli_doc_check_strict_success_{}",
        std::process::id()
    ));
    std::fs::create_dir_all(&dir).unwrap();
    std::fs::write(
        dir.join("input.ls"),
        "(defn main [] 42)\n; Doc-Review-Status: Passed\n; Doc-Reviewed-By: anonymous\n",
    )
    .unwrap();

    let harness = r#"
(defn main []
  (do
    (print (run-doc-check "input.ls" 1))
    0))
"#;

    let combined = format!("{}\n{}", selfhost_cli_runtime_bundle(), harness);
    let output = compile_and_run_with_dir(&combined, &dir);
    let _ = std::fs::remove_dir_all(&dir);
    let lines: Vec<&str> = output.trim().lines().collect();

    assert_eq!(
        lines,
        vec![
            "status:ok",
            "module-global",
            "functions:1,types:0,first-fn:main",
            "; Doc-Review-Status: Passed",
            "; Doc-Reviewed-By: anonymous",
            "0",
        ],
        "run-doc-check strict mode は valid trailer comment を受理するべき"
    );
}

/// TEST-CLI-02-U3: selfhost/src/App/Cli.ls の run-doc-check strict mode が invalid trailer を拒否すること
#[test]
#[ignore]
fn test_e2e_selfhost_cli_doc_check_file_handler_strict_missing_trailer_fails() {
    let dir = std::env::temp_dir().join(format!(
        "lsharp_test_cli_doc_check_strict_fail_{}",
        std::process::id()
    ));
    std::fs::create_dir_all(&dir).unwrap();
    std::fs::write(dir.join("input.ls"), "(defn main [] 42)\n").unwrap();

    let harness = r#"
(defn main []
  (do
    (print (run-doc-check "input.ls" 1))
    0))
"#;

    let combined = format!("{}\n{}", selfhost_cli_runtime_bundle(), harness);
    let output = compile_and_run_with_dir(&combined, &dir);
    let _ = std::fs::remove_dir_all(&dir);
    let lines: Vec<&str> = output.trim().lines().collect();

    assert_eq!(
        lines,
        vec![
            "error: invalid doc trailer: expected trailing comment lines",
            "1"
        ],
        "run-doc-check strict mode は trailer 欠落時に compile error を返すべき"
    );
}

/// TEST-CLI-02-V: exit-code-success が 0 を返すこと
///
/// CLI-02 contract parity: 終了コードの公開 API を検証
#[test]
#[ignore]
fn test_e2e_selfhost_cli_exit_code_success() {
    let harness = r#"
(defn main []
  (do
    (print (exit-code-success))
    0))
"#;

    let combined = format!("{}\n{}", selfhost_cli_runtime_bundle(), harness);
    let output = compile_and_run(&combined);
    let lines: Vec<&str> = output.trim().lines().collect();

    assert_eq!(
        lines.last().unwrap(),
        &"0",
        "exit-code-success は 0 であるべき"
    );
}

/// TEST-CLI-02-W: exit-code-compile-error が 1 を返すこと
///
/// CLI-02 contract parity: コンパイルエラー終了コード
#[test]
#[ignore]
fn test_e2e_selfhost_cli_exit_code_compile_error() {
    let harness = r#"
(defn main []
  (do
    (print (exit-code-compile-error))
    0))
"#;

    let combined = format!("{}\n{}", selfhost_cli_runtime_bundle(), harness);
    let output = compile_and_run(&combined);
    let lines: Vec<&str> = output.trim().lines().collect();

    assert_eq!(
        lines.last().unwrap(),
        &"1",
        "exit-code-compile-error は 1 であるべき"
    );
}

/// TEST-CLI-02-X: 不明コマンドで run-command が 127 を返すこと
///
/// CLI-02 contract parity: 不明コマンドの終了コード
#[test]
#[ignore]
fn test_e2e_selfhost_cli_exit_code_unknown_command() {
    let harness = r#"
(defn main []
  (let [code (run-command "nonexistent" "" 0)]
    (do
      (print code)
      0)))
"#;

    let combined = format!("{}\n{}", selfhost_cli_runtime_bundle(), harness);
    let output = compile_and_run(&combined);
    let lines: Vec<&str> = output.trim().lines().collect();

    // run-command は cli-stderr でエラーを出力してから 127 を返す
    assert_eq!(
        lines.last().unwrap(),
        &"127",
        "不明コマンドの終了コードは 127 であるべき"
    );
    assert!(
        output.contains("error: unknown command: nonexistent"),
        "不明コマンドでエラーメッセージが出力されるべき: {:?}",
        output
    );
}

/// TEST-CLI-02-Y: help-text が 13 コマンドすべてを列挙すること
///
/// CLI-02 contract parity: ヘルプ出力の完全性
#[test]
#[ignore]
fn test_e2e_selfhost_cli_help_lists_all_commands() {
    let harness = r#"
(defn main []
  (do
    (print-string (help-text))
    0))
"#;

    let combined = format!("{}\n{}", selfhost_cli_runtime_bundle(), harness);
    let output = compile_and_run(&combined);

    let commands = [
        "parse",
        "check",
        "compile",
        "build",
        "test",
        "review",
        "doc-ack",
        "doc-check",
        "install",
        "repl",
        "lsp",
        "fmt",
        "doc",
    ];
    let mut count = 0;
    for cmd in &commands {
        if output.contains(cmd) {
            count += 1;
        }
    }
    assert_eq!(
        count, 13,
        "help テキストは 13 コマンドすべてを列挙すべき (found {})",
        count
    );
}

/// TEST-CLI-02-Z: version-text が `lsharp x.y.z` 形式であること
///
/// CLI-02 contract parity: バージョン出力形式
#[test]
#[ignore]
fn test_e2e_selfhost_cli_version_format() {
    let harness = r#"
(defn main []
  (do
    (print-string (version-text))
    0))
"#;

    let combined = format!("{}\n{}", selfhost_cli_runtime_bundle(), harness);
    let output = compile_and_run(&combined);
    let trimmed = output.trim();

    assert!(
        trimmed.starts_with("lsharp "),
        "バージョンは 'lsharp ' で始まるべき: {:?}",
        trimmed
    );
    let version_part = trimmed.strip_prefix("lsharp ").unwrap();
    let parts: Vec<&str> = version_part.split('.').collect();
    assert_eq!(
        parts.len(),
        3,
        "バージョンは x.y.z 形式であるべき: {}",
        version_part
    );
}

/// TEST-CLI-02-AA: cli-stdout / cli-stderr の出力チャネル分離
///
/// CLI-02 contract parity: stdout は結果出力、stderr は "error: " プレフィックス付き
#[test]
#[ignore]
fn test_e2e_selfhost_cli_stdout_stderr_separation() {
    let harness = r#"
(defn main []
  (do
    (cli-stdout "program output")
    (cli-stderr "diagnostic message")
    0))
"#;

    let combined = format!("{}\n{}", selfhost_cli_runtime_bundle(), harness);
    let output = compile_and_run(&combined);

    assert!(
        output.contains("program output"),
        "cli-stdout の出力が含まれるべき: {:?}",
        output
    );
    assert!(
        output.contains("error: diagnostic message"),
        "cli-stderr の出力は 'error: ' プレフィックスを持つべき: {:?}",
        output
    );
    // stdout と stderr が別行に出力されることを確認
    let lines: Vec<&str> = output.trim().lines().collect();
    assert!(
        lines.len() >= 2,
        "cli-stdout と cli-stderr は別行に出力されるべき: {:?}",
        lines
    );
}

/// TEST-CLI-02-AB: main-dispatch が parse file handler を entrypoint helper 経由で呼べること
#[test]
#[ignore]
fn test_e2e_selfhost_cli_main_dispatch_parse_file() {
    let dir = std::env::temp_dir().join(format!(
        "lsharp_test_cli_main_dispatch_parse_{}",
        std::process::id()
    ));
    std::fs::create_dir_all(&dir).unwrap();
    std::fs::write(dir.join("input.ls"), "(defn main [] 42)").unwrap();

    let harness = r#"
(defn main []
  (do
    (print (main-dispatch "parse" "input.ls" 0))
    0))
"#;

    let combined = format!("{}\n{}", selfhost_cli_runtime_bundle(), harness);
    let output = compile_and_run_with_dir(&combined, &dir);
    let _ = std::fs::remove_dir_all(&dir);
    let lines: Vec<&str> = output.trim().lines().collect();

    assert!(
        lines.len() >= 5,
        "main-dispatch parse 出力が不足: {:?}",
        lines
    );
    assert_eq!(lines[0], "decls:1");
    assert_eq!(lines[1], "first-decl:defn");
    assert_eq!(lines[2], "first-body:int");
    assert_eq!(lines[3], "diagnostics:0");
    assert_eq!(lines[4], "0");
}

/// TEST-CLI-02-AC: main-dispatch が help/version/unknown surface を保つこと
#[test]
#[ignore]
fn test_e2e_selfhost_cli_main_dispatch_command_surface() {
    let harness = r#"
(defn main []
  (let [help-code (main-dispatch "--help" "" 0)
        version-code (main-dispatch "--version" "" 0)
        unknown-code (main-dispatch "nonexistent" "" 0)]
    (do
      (print-string "\nhelp-code:")
      (print help-code)
      (print-string "version-code:")
      (print version-code)
      (print-string "unknown-code:")
      (print unknown-code)
      0)))
"#;

    let combined = format!("{}\n{}", selfhost_cli_runtime_bundle(), harness);
    let output = compile_and_run(&combined);
    assert!(
        output.contains("Usage: lsharp <command>"),
        "main-dispatch help は usage を出力すべき: {:?}",
        output
    );
    assert!(
        output.contains("lsharp 0.1.0"),
        "main-dispatch version は version text を出力すべき: {:?}",
        output
    );
    assert!(
        output.contains("error: unknown command: nonexistent"),
        "main-dispatch unknown は error surface を保つべき: {:?}",
        output
    );
    assert!(
        output.contains("help-code:0"),
        "help は success=0 を返すべき: {:?}",
        output
    );
    assert!(
        output.contains("version-code:0"),
        "version は success=0 を返すべき: {:?}",
        output
    );
    assert!(
        output.contains("unknown-code:127"),
        "unknown command は 127 を返すべき: {:?}",
        output
    );
}

/// TEST-CLI-02-AD: actual Cli main は引数なし実行で help surface を返すこと
#[test]
#[ignore]
fn test_e2e_selfhost_cli_main_no_args_shows_help() {
    let output = compile_and_run(selfhost_cli_runtime_bundle());

    assert!(
        output.contains("Usage: lsharp <command>"),
        "Cli main の no-args 実行は help usage を返すべき: {:?}",
        output
    );
    assert!(
        output.contains("Commands:"),
        "Cli main の no-args 実行は command list を返すべき: {:?}",
        output
    );
}

/// TEST-CLI-02-AE: actual Cli main は argv 経由で --version を処理できること
#[test]
#[ignore]
fn test_e2e_selfhost_cli_main_with_args_version() {
    let output = compile_and_run_with_args(selfhost_cli_runtime_bundle(), &["--version"]);

    assert!(
        output.contains("lsharp 0.1.0"),
        "Cli main の argv 実行は --version を処理すべき: {:?}",
        output
    );
}

/// TEST-CLI-02-AE2: actual Cli main は argv 経由で -v alias を処理できること
#[test]
#[ignore]
fn test_e2e_selfhost_cli_main_with_args_short_version() {
    let output = compile_and_run_with_args(selfhost_cli_runtime_bundle(), &["-v"]);

    assert!(
        output.contains("lsharp 0.1.0"),
        "Cli main の argv 実行は -v alias を処理すべき: {:?}",
        output
    );
}

/// TEST-CLI-02-AF: actual Cli main は argv 経由で parse file command を処理できること
#[test]
#[ignore]
fn test_e2e_selfhost_cli_main_with_args_parse_file() {
    let dir = std::env::temp_dir().join(format!(
        "lsharp_test_cli_main_args_parse_{}",
        std::process::id()
    ));
    std::fs::create_dir_all(&dir).unwrap();
    std::fs::write(dir.join("input.ls"), "(defn main [] 42)").unwrap();

    let output = compile_and_run_with_dir_and_args(
        selfhost_cli_runtime_bundle(),
        &dir,
        &["parse", "input.ls"],
    );
    let _ = std::fs::remove_dir_all(&dir);
    let lines: Vec<&str> = output.trim().lines().collect();

    assert!(
        lines.len() >= 4,
        "Cli main parse argv 出力が不足: {:?}",
        lines
    );
    assert_eq!(lines[0], "decls:1");
    assert_eq!(lines[1], "first-decl:defn");
    assert_eq!(lines[2], "first-body:int");
    assert_eq!(lines[3], "diagnostics:0");
}

/// TEST-CLI-02-AF2: actual Cli main は argv 経由で compile file command を処理できること
#[test]
#[ignore]
fn test_e2e_selfhost_cli_main_with_args_compile_file() {
    let dir = std::env::temp_dir().join(format!(
        "lsharp_test_cli_main_args_compile_{}",
        std::process::id()
    ));
    std::fs::create_dir_all(&dir).unwrap();
    std::fs::write(dir.join("input.ls"), "(defn main [] 42)").unwrap();

    let output = compile_and_run_with_dir_and_args(
        selfhost_cli_runtime_bundle(),
        &dir,
        &["compile", "input.ls"],
    );
    let _ = std::fs::remove_dir_all(&dir);
    let lines: Vec<&str> = output.trim().lines().collect();

    assert!(
        !lines.is_empty(),
        "Cli main compile argv 出力が不足: {:?}",
        lines
    );
    assert!(
        lines[0].starts_with("wasm-size:"),
        "Cli main compile argv は wasm-size:<n> を返すべき: {:?}",
        lines
    );
}

/// TEST-CLI-02-AF2B: actual Cli main は nested import fixture の compile を import-aware helper と同じ summary で返すこと
#[test]
#[ignore]
fn test_e2e_selfhost_cli_main_with_args_compile_file_multifile_nested_imports() {
    let dir = cli_test_fixture_dir("main_compile_multifile_nested");
    write_cli_fixture_files(&dir, &cli_multifile_nested_fixture_files());
    let expected_size = run_cli_multifile_helper_size(&dir, "main.ls", 0);

    let output = compile_and_run_with_dir_and_args(
        selfhost_cli_runtime_bundle(),
        &dir,
        &["compile", "main.ls"],
    );
    let _ = std::fs::remove_dir_all(&dir);

    let output_line = output
        .trim()
        .lines()
        .next()
        .expect("Cli main compile multi-file output が必要");
    let output_size =
        parse_wasm_size_line(output_line, "Cli main compile multi-file nested fixture");
    assert!(
        output_size == expected_size,
        "Cli main compile は import-aware helper と同じ wasm-size を返すべき: cli={output_size}, helper={expected_size}"
    );
}

/// TEST-CLI-02-AF3: actual Cli main は argv 経由で build file command を処理できること
#[test]
#[ignore]
fn test_e2e_selfhost_cli_main_with_args_build_file() {
    let dir = std::env::temp_dir().join(format!(
        "lsharp_test_cli_main_args_build_{}",
        std::process::id()
    ));
    std::fs::create_dir_all(&dir).unwrap();
    std::fs::write(dir.join("input.ls"), "(defn main [] 42)").unwrap();

    let output = compile_and_run_with_dir_and_args(
        selfhost_cli_runtime_bundle(),
        &dir,
        &["build", "input.ls"],
    );
    let _ = std::fs::remove_dir_all(&dir);
    let lines: Vec<&str> = output.trim().lines().collect();

    assert!(
        !lines.is_empty(),
        "Cli main build argv 出力が不足: {:?}",
        lines
    );
    assert!(
        lines[0].starts_with("wasm-size:"),
        "Cli main build argv は wasm-size:<n> を返すべき: {:?}",
        lines
    );
}

/// TEST-CLI-02-AF3B: actual Cli main は nested import fixture の build を import-aware helper と同じ summary で返すこと
#[test]
#[ignore]
fn test_e2e_selfhost_cli_main_with_args_build_file_multifile_nested_imports() {
    let dir = cli_test_fixture_dir("main_build_multifile_nested");
    write_cli_fixture_files(&dir, &cli_multifile_nested_fixture_files());
    let expected_size = run_cli_multifile_helper_size(&dir, "main.ls", 0);

    let output = compile_and_run_with_dir_and_args(
        selfhost_cli_runtime_bundle(),
        &dir,
        &["build", "main.ls"],
    );
    let _ = std::fs::remove_dir_all(&dir);

    let output_line = output
        .trim()
        .lines()
        .next()
        .expect("Cli main build multi-file output が必要");
    let output_size = parse_wasm_size_line(output_line, "Cli main build multi-file nested fixture");
    assert!(
        output_size == expected_size,
        "Cli main build は import-aware helper と同じ wasm-size を返すべき: cli={output_size}, helper={expected_size}"
    );
}

/// TEST-CLI-02-AF4: actual Cli main は compile <file> -o <path> で output file を書けること
#[test]
#[ignore]
fn test_e2e_selfhost_cli_main_with_args_compile_output_path() {
    let dir = std::env::temp_dir().join(format!(
        "lsharp_test_cli_main_args_compile_output_{}",
        std::process::id()
    ));
    std::fs::create_dir_all(&dir).unwrap();
    std::fs::write(dir.join("input.ls"), "(defn main [] 42)").unwrap();

    let output = compile_and_run_with_dir_and_args(
        selfhost_cli_runtime_bundle(),
        &dir,
        &["compile", "input.ls", "-o", "out.txt"],
    );
    let written = std::fs::read_to_string(dir.join("out.txt")).unwrap();
    let _ = std::fs::remove_dir_all(&dir);
    let lines: Vec<&str> = output.trim().lines().collect();

    assert!(
        !lines.is_empty(),
        "Cli main compile -o 出力が不足: {:?}",
        lines
    );
    assert!(
        lines[0].starts_with("wasm-size:"),
        "Cli main compile -o は wasm-size:<n> を返すべき: {:?}",
        lines
    );
    assert_eq!(
        written.trim(),
        lines[0],
        "compile -o は stdout summary を output file にも書くべき"
    );
}

/// TEST-CLI-02-AF5: actual Cli main は build <file> --output <path> で output file を書けること
#[test]
#[ignore]
fn test_e2e_selfhost_cli_main_with_args_build_output_path() {
    let dir = std::env::temp_dir().join(format!(
        "lsharp_test_cli_main_args_build_output_{}",
        std::process::id()
    ));
    std::fs::create_dir_all(&dir).unwrap();
    std::fs::write(dir.join("input.ls"), "(defn main [] 42)").unwrap();

    let output = compile_and_run_with_dir_and_args(
        selfhost_cli_runtime_bundle(),
        &dir,
        &["build", "input.ls", "--output", "build.txt"],
    );
    let written = std::fs::read_to_string(dir.join("build.txt")).unwrap();
    let _ = std::fs::remove_dir_all(&dir);
    let lines: Vec<&str> = output.trim().lines().collect();

    assert!(
        !lines.is_empty(),
        "Cli main build --output 出力が不足: {:?}",
        lines
    );
    assert!(
        lines[0].starts_with("wasm-size:"),
        "Cli main build --output は wasm-size:<n> を返すべき: {:?}",
        lines
    );
    assert_eq!(
        written.trim(),
        lines[0],
        "build --output は stdout summary を output file にも書くべき"
    );
}

/// TEST-CLI-02-AF6: actual Cli main は compile <file> --target ... -o <path> を併用できること
#[test]
#[ignore]
fn test_e2e_selfhost_cli_main_with_args_compile_target_and_output_path() {
    let dir = std::env::temp_dir().join(format!(
        "lsharp_test_cli_main_args_compile_target_output_{}",
        std::process::id()
    ));
    std::fs::create_dir_all(&dir).unwrap();
    std::fs::write(dir.join("input.ls"), "(defn main [] 42)").unwrap();

    let output = compile_and_run_with_dir_and_args(
        selfhost_cli_runtime_bundle(),
        &dir,
        &[
            "compile",
            "input.ls",
            "--target",
            "wasi-component",
            "-o",
            "targeted.txt",
        ],
    );
    let written = std::fs::read_to_string(dir.join("targeted.txt")).unwrap();
    let _ = std::fs::remove_dir_all(&dir);
    let lines: Vec<&str> = output.trim().lines().collect();

    assert!(
        !lines.is_empty(),
        "Cli main compile --target ... -o 出力が不足: {:?}",
        lines
    );
    assert!(
        lines[0].starts_with("wasm-size:"),
        "Cli main compile --target ... -o は wasm-size:<n> を返すべき: {:?}",
        lines
    );
    assert_eq!(
        written.trim(),
        lines[0],
        "compile --target ... -o は stdout summary を output file にも書くべき"
    );
}

/// TEST-CLI-02-AF6B: actual Cli main は preview1/component target ごとに異なる wasm-size を返すこと
#[test]
#[ignore]
fn test_e2e_selfhost_cli_main_with_args_compile_target_changes_wasm_size() {
    let dir = std::env::temp_dir().join(format!(
        "lsharp_test_cli_main_args_compile_target_size_{}",
        std::process::id()
    ));
    std::fs::create_dir_all(&dir).unwrap();
    std::fs::write(dir.join("input.ls"), "(defn main [] 42)").unwrap();

    let preview1_output = compile_and_run_with_dir_and_args(
        selfhost_cli_runtime_bundle(),
        &dir,
        &["compile", "input.ls", "--target", "wasi-preview1"],
    );
    let component_output = compile_and_run_with_dir_and_args(
        selfhost_cli_runtime_bundle(),
        &dir,
        &["compile", "input.ls", "--target", "wasi-component"],
    );
    let _ = std::fs::remove_dir_all(&dir);

    let preview1_line = preview1_output
        .trim()
        .lines()
        .next()
        .expect("preview1 compile output が必要");
    let component_line = component_output
        .trim()
        .lines()
        .next()
        .expect("component compile output が必要");
    assert!(
        preview1_line.starts_with("wasm-size:"),
        "preview1 compile output は wasm-size:<n> を返すべき: {:?}",
        preview1_output
    );
    assert!(
        component_line.starts_with("wasm-size:"),
        "component compile output は wasm-size:<n> を返すべき: {:?}",
        component_output
    );

    let preview1_size: i64 = preview1_line["wasm-size:".len()..]
        .parse()
        .expect("preview1 wasm size は整数であるべき");
    let component_size: i64 = component_line["wasm-size:".len()..]
        .parse()
        .expect("component wasm size は整数であるべき");
    assert!(
        preview1_size > component_size,
        "Cli main compile は preview1/component target を size に反映するべき: preview1={preview1_size}, component={component_size}"
    );
}

/// TEST-CLI-02-AF7: actual Cli main は build <file> --output <path> --target wasm を併用できること
#[test]
#[ignore]
fn test_e2e_selfhost_cli_main_with_args_build_output_path_and_target_alias() {
    let dir = std::env::temp_dir().join(format!(
        "lsharp_test_cli_main_args_build_output_target_{}",
        std::process::id()
    ));
    std::fs::create_dir_all(&dir).unwrap();
    std::fs::write(dir.join("input.ls"), "(defn main [] 42)").unwrap();

    let output = compile_and_run_with_dir_and_args(
        selfhost_cli_runtime_bundle(),
        &dir,
        &[
            "build",
            "input.ls",
            "--output",
            "build-target.txt",
            "--target",
            "wasm",
        ],
    );
    let written = std::fs::read_to_string(dir.join("build-target.txt")).unwrap();
    let _ = std::fs::remove_dir_all(&dir);
    let lines: Vec<&str> = output.trim().lines().collect();

    assert!(
        !lines.is_empty(),
        "Cli main build --output ... --target 出力が不足: {:?}",
        lines
    );
    assert!(
        lines[0].starts_with("wasm-size:"),
        "Cli main build --output ... --target は wasm-size:<n> を返すべき: {:?}",
        lines
    );
    assert_eq!(
        written.trim(),
        lines[0],
        "build --output ... --target は stdout summary を output file にも書くべき"
    );
}

/// TEST-CLI-02-AG: actual Cli main は subcommand --help を処理できること
#[test]
#[ignore]
fn test_e2e_selfhost_cli_main_with_args_subcommand_help() {
    let output = compile_and_run_with_args(selfhost_cli_runtime_bundle(), &["parse", "--help"]);

    assert!(
        output.contains("parse <file> - Parse source and show AST"),
        "Cli main は subcommand help text を返すべき: {:?}",
        output
    );
}

/// TEST-CLI-02-AH: actual Cli main は `-h` alias で global help を返せること
#[test]
#[ignore]
fn test_e2e_selfhost_cli_main_with_args_short_help() {
    let output = compile_and_run_with_args(selfhost_cli_runtime_bundle(), &["-h"]);

    assert!(
        output.contains("Usage: lsharp <command>"),
        "Cli main は -h alias で global help を返すべき: {:?}",
        output
    );
}

/// TEST-CLI-02-AI: actual Cli main は `help <subcommand>` を処理できること
#[test]
#[ignore]
fn test_e2e_selfhost_cli_main_with_help_command() {
    let output = compile_and_run_with_args(selfhost_cli_runtime_bundle(), &["help", "parse"]);

    assert!(
        output.contains("parse <file> - Parse source and show AST"),
        "Cli main は help subcommand surface を返すべき: {:?}",
        output
    );
}

/// TEST-CLI-02-AJ: actual Cli main は help compile に output option surface を含めること
#[test]
#[ignore]
fn test_e2e_selfhost_cli_main_with_help_compile_output_option() {
    let output = compile_and_run_with_args(selfhost_cli_runtime_bundle(), &["help", "compile"]);

    assert!(
        output.contains("compile <file> [-o <file>]"),
        "Cli main は compile help に output option surface を含めるべき: {:?}",
        output
    );
}

/// TEST-CLI-02-AK: actual Cli main は build --help に output option surface を含めること
#[test]
#[ignore]
fn test_e2e_selfhost_cli_main_with_build_subcommand_help_output_option() {
    let output = compile_and_run_with_args(selfhost_cli_runtime_bundle(), &["build", "--help"]);

    assert!(
        output.contains("build <file> [--output <file>]"),
        "Cli main は build help に output option surface を含めるべき: {:?}",
        output
    );
}

/// TEST-CLI-02-AL: actual Cli main は `lsp --stdio` で stdin の initialize frame を処理できること
#[test]
#[ignore]
fn test_e2e_selfhost_cli_main_with_lsp_stdio_initialize() {
    let request_body = r#"{"jsonrpc":"2.0","id":21,"method":"initialize","params":0}"#;
    let stdin = format!(
        "Content-Length: {}\r\n\r\n{}",
        request_body.len(),
        request_body
    );
    let response_body = r#"{"jsonrpc":"2.0","id":21,"result":[1,1,1,1,1,1,1]}"#;
    let expected = format!(
        "Content-Length: {}\r\n\r\n{}",
        response_body.len(),
        response_body
    );

    let output = compile_and_run_with_args_and_stdin(
        selfhost_cli_runtime_bundle(),
        &["lsp", "--stdio"],
        &stdin,
    );

    assert_eq!(
        output, expected,
        "Cli main は lsp --stdio で initialize frame をそのまま返すべき"
    );
}

/// TEST-CLI-02-AM: actual Cli main は `lsp --stdio` で連続 frame を順に処理できること
#[test]
#[ignore]
fn test_e2e_selfhost_cli_main_with_lsp_stdio_initialize_shutdown_sequence() {
    let init_body = r#"{"jsonrpc":"2.0","id":31,"method":"initialize","params":0}"#;
    let shutdown_body = r#"{"jsonrpc":"2.0","id":32,"method":"shutdown","params":0}"#;
    let stdin = format!(
        "Content-Length: {}\r\n\r\n{}Content-Length: {}\r\n\r\n{}",
        init_body.len(),
        init_body,
        shutdown_body.len(),
        shutdown_body
    );
    let init_response = r#"{"jsonrpc":"2.0","id":31,"result":[1,1,1,1,1,1,1]}"#;
    let shutdown_response = r#"{"jsonrpc":"2.0","id":32,"result":0}"#;
    let expected = format!(
        "Content-Length: {}\r\n\r\n{}Content-Length: {}\r\n\r\n{}",
        init_response.len(),
        init_response,
        shutdown_response.len(),
        shutdown_response
    );

    let output = compile_and_run_with_args_and_stdin(
        selfhost_cli_runtime_bundle(),
        &["lsp", "--stdio"],
        &stdin,
    );

    assert_eq!(
        output, expected,
        "Cli main は lsp --stdio で initialize→shutdown frame を順に返すべき"
    );
}

/// TEST-CLI-02-AN: actual Cli main は `lsp --stdio` で unknown method を Method not found frame にできること
#[test]
#[ignore]
fn test_e2e_selfhost_cli_main_with_lsp_stdio_unknown_method() {
    let request_body = r#"{"jsonrpc":"2.0","id":41,"method":"workspace/unknown","params":0}"#;
    let stdin = format!(
        "Content-Length: {}\r\n\r\n{}",
        request_body.len(),
        request_body
    );
    let response_body =
        r#"{"jsonrpc":"2.0","id":41,"error":{"code":-32601,"message":"Method not found"}}"#;
    let expected = format!(
        "Content-Length: {}\r\n\r\n{}",
        response_body.len(),
        response_body
    );

    let output = compile_and_run_with_args_and_stdin(
        selfhost_cli_runtime_bundle(),
        &["lsp", "--stdio"],
        &stdin,
    );

    assert_eq!(
        output, expected,
        "Cli main は lsp --stdio で unknown method を error frame にすべき"
    );
}

/// TEST-CLI-02-AN2: actual Cli main は `lsp --stdio` で completion request を framed response にできること
#[test]
#[ignore]
fn test_e2e_selfhost_cli_main_with_lsp_stdio_completion() {
    let request_body = r#"{"jsonrpc":"2.0","id":51,"method":"textDocument/completion","params":0}"#;
    let stdin = format!(
        "Content-Length: {}\r\n\r\n{}",
        request_body.len(),
        request_body
    );
    let response_body = r#"{"jsonrpc":"2.0","id":51,"result":[["defn",14,"defn"],["let",14,"let"],["if",14,"if"],["match",14,"match"],["do",14,"do"],["fn",14,"fn"],["module",14,"module"]]}"#;
    let expected = format!(
        "Content-Length: {}\r\n\r\n{}",
        response_body.len(),
        response_body
    );

    let output = compile_and_run_with_args_and_stdin(
        selfhost_cli_runtime_bundle(),
        &["lsp", "--stdio"],
        &stdin,
    );

    assert_eq!(
        output, expected,
        "Cli main は lsp --stdio で completion frame を返すべき"
    );
}

/// TEST-CLI-02-AN3: actual Cli main は `lsp --stdio` で definition request を framed response にできること
#[test]
#[ignore]
fn test_e2e_selfhost_cli_main_with_lsp_stdio_goto_definition() {
    let request_body = r#"{"jsonrpc":"2.0","id":61,"method":"textDocument/definition","params":{"uri":10,"line":1,"col":38,"source":"(defn helper [x] x) (defn main [] (helper 1))"}}"#;
    let stdin = format!(
        "Content-Length: {}\r\n\r\n{}",
        request_body.len(),
        request_body
    );
    let response_body = r#"{"jsonrpc":"2.0","id":61,"result":[10,1,7]}"#;
    let expected = format!(
        "Content-Length: {}\r\n\r\n{}",
        response_body.len(),
        response_body
    );

    let output = compile_and_run_with_args_and_stdin(
        selfhost_cli_runtime_bundle(),
        &["lsp", "--stdio"],
        &stdin,
    );

    assert_eq!(
        output, expected,
        "Cli main は lsp --stdio で definition frame を返すべき"
    );
}

/// TEST-CLI-02-AN4: actual Cli main は `lsp --stdio` で hover request を framed response にできること
#[test]
#[ignore]
fn test_e2e_selfhost_cli_main_with_lsp_stdio_hover() {
    let request_body = r#"{"jsonrpc":"2.0","id":62,"method":"textDocument/hover","params":{"uri":10,"line":1,"col":38,"source":"(defn helper [x] x) (defn main [] (helper 1))"}}"#;
    let stdin = format!(
        "Content-Length: {}\r\n\r\n{}",
        request_body.len(),
        request_body
    );
    let response_body =
        r#"{"jsonrpc":"2.0","id":62,"result":{"range":[1,36,1,42],"contents":"defn helper"}}"#;
    let expected = format!(
        "Content-Length: {}\r\n\r\n{}",
        response_body.len(),
        response_body
    );

    let output = compile_and_run_with_args_and_stdin(
        selfhost_cli_runtime_bundle(),
        &["lsp", "--stdio"],
        &stdin,
    );

    assert_eq!(
        output, expected,
        "Cli main は lsp --stdio で hover frame を返すべき"
    );
}

/// TEST-CLI-02-AN5: actual Cli main は `lsp --stdio` で references request を framed response にできること
#[test]
#[ignore]
fn test_e2e_selfhost_cli_main_with_lsp_stdio_references() {
    let request_body = r#"{"jsonrpc":"2.0","id":63,"method":"textDocument/references","params":{"uri":10,"line":1,"col":38,"source":"(defn square [x] x) (defn main [] (square 1) (square 2))"}}"#;
    let stdin = format!(
        "Content-Length: {}\r\n\r\n{}",
        request_body.len(),
        request_body
    );
    let response_body = r#"{"jsonrpc":"2.0","id":63,"result":[[10,1,7],[10,1,36],[10,1,47]]}"#;
    let expected = format!(
        "Content-Length: {}\r\n\r\n{}",
        response_body.len(),
        response_body
    );

    let output = compile_and_run_with_args_and_stdin(
        selfhost_cli_runtime_bundle(),
        &["lsp", "--stdio"],
        &stdin,
    );

    assert_eq!(
        output, expected,
        "Cli main は lsp --stdio で references frame を返すべき"
    );
}

/// TEST-CLI-02-AN6: actual Cli main は `lsp --stdio` で formatting request を framed response にできること
#[test]
#[ignore]
fn test_e2e_selfhost_cli_main_with_lsp_stdio_formatting() {
    let request_body = r#"{"jsonrpc":"2.0","id":64,"method":"textDocument/formatting","params":{"uri":10,"source":"(defn main [] 1)"}}"#;
    let stdin = format!(
        "Content-Length: {}\r\n\r\n{}",
        request_body.len(),
        request_body
    );
    let response_body =
        "{\"jsonrpc\":\"2.0\",\"id\":64,\"result\":[[1,1,1,17,\"(defn main [] 1)\\n\"]]}";
    let expected = format!(
        "Content-Length: {}\r\n\r\n{}",
        response_body.len(),
        response_body
    );

    let output = compile_and_run_with_args_and_stdin(
        selfhost_cli_runtime_bundle(),
        &["lsp", "--stdio"],
        &stdin,
    );

    assert_eq!(
        output, expected,
        "Cli main は lsp --stdio で formatting frame を返すべき"
    );
}

/// TEST-CLI-02-AN7: actual Cli main は `lsp --stdio` で rename request を framed response にできること
#[test]
#[ignore]
fn test_e2e_selfhost_cli_main_with_lsp_stdio_rename() {
    let request_body = r#"{"jsonrpc":"2.0","id":65,"method":"textDocument/rename","params":{"uri":10,"line":1,"col":38,"source":"(defn square [x] x) (defn main [] (square 1) (square 2))","newName":"cube"}}"#;
    let stdin = format!(
        "Content-Length: {}\r\n\r\n{}",
        request_body.len(),
        request_body
    );
    let response_body = r#"{"jsonrpc":"2.0","id":65,"result":[[10,[[1,7,1,13,"cube"],[1,36,1,42,"cube"],[1,47,1,53,"cube"]]]]}"#;
    let expected = format!(
        "Content-Length: {}\r\n\r\n{}",
        response_body.len(),
        response_body
    );

    let output = compile_and_run_with_args_and_stdin(
        selfhost_cli_runtime_bundle(),
        &["lsp", "--stdio"],
        &stdin,
    );

    assert_eq!(
        output, expected,
        "Cli main は lsp --stdio で rename frame を返すべき"
    );
}

/// TEST-CLI-02-AN8: actual Cli main は `lsp --stdio` で didOpen -> didChange sequence を順に処理できること
#[test]
#[ignore]
fn test_e2e_selfhost_cli_main_with_lsp_stdio_document_sequence() {
    let open_body = r#"{"jsonrpc":"2.0","method":"textDocument/didOpen","params":{"uri":42,"source":"(defn main [] 0)"}}"#;
    let change_body = r#"{"jsonrpc":"2.0","method":"textDocument/didChange","params":{"uri":42,"source":"(defn main [] (+ 0 1))"}}"#;
    let stdin = format!(
        "Content-Length: {}\r\n\r\n{}Content-Length: {}\r\n\r\n{}",
        open_body.len(),
        open_body,
        change_body.len(),
        change_body
    );
    let open_response =
        r#"{"jsonrpc":"2.0","method":"textDocument/didOpen","params":{"uri":42,"sourceBytes":16}}"#;
    let change_response = r#"{"jsonrpc":"2.0","method":"textDocument/didChange","params":{"uri":42,"sourceBytes":22}}"#;
    let expected = format!(
        "Content-Length: {}\r\n\r\n{}Content-Length: {}\r\n\r\n{}",
        open_response.len(),
        open_response,
        change_response.len(),
        change_response
    );

    let output = compile_and_run_with_args_and_stdin(
        selfhost_cli_runtime_bundle(),
        &["lsp", "--stdio"],
        &stdin,
    );

    assert_eq!(
        output, expected,
        "Cli main は lsp --stdio で didOpen -> didChange frame を順に返すべき"
    );
}

/// TEST-CLI-02-AN9: actual Cli main は `lsp --stdio` で didOpen 後に source なし hover request も open document state から処理できること
#[test]
#[ignore]
fn test_e2e_selfhost_cli_main_with_lsp_stdio_hover_uses_open_document() {
    let open_body = r#"{"jsonrpc":"2.0","method":"textDocument/didOpen","params":{"uri":42,"source":"(defn helper [x] x) (defn main [] (helper 1))"}}"#;
    let hover_body = r#"{"jsonrpc":"2.0","id":66,"method":"textDocument/hover","params":{"uri":42,"line":1,"col":38}}"#;
    let stdin = format!(
        "Content-Length: {}\r\n\r\n{}Content-Length: {}\r\n\r\n{}",
        open_body.len(),
        open_body,
        hover_body.len(),
        hover_body
    );
    let open_response =
        r#"{"jsonrpc":"2.0","method":"textDocument/didOpen","params":{"uri":42,"sourceBytes":45}}"#;
    let hover_response =
        r#"{"jsonrpc":"2.0","id":66,"result":{"range":[1,36,1,42],"contents":"defn helper"}}"#;
    let expected = format!(
        "Content-Length: {}\r\n\r\n{}Content-Length: {}\r\n\r\n{}",
        open_response.len(),
        open_response,
        hover_response.len(),
        hover_response
    );

    let output = compile_and_run_with_args_and_stdin(
        selfhost_cli_runtime_bundle(),
        &["lsp", "--stdio"],
        &stdin,
    );

    assert_eq!(
        output, expected,
        "Cli main は didOpen 後の source なし hover で open document state を使うべき"
    );
}

/// TEST-CLI-02-AN9b: actual Cli main は spec 寄り hover params でも open document state から処理できること
#[test]
#[ignore]
fn test_e2e_selfhost_cli_main_with_lsp_stdio_hover_uses_open_document_spec_params() {
    let open_body = r#"{"jsonrpc":"2.0","method":"textDocument/didOpen","params":{"uri":42,"source":"(defn helper [x] x) (defn main [] (helper 1))"}}"#;
    let hover_body = r#"{"jsonrpc":"2.0","id":66,"method":"textDocument/hover","params":{"textDocument":{"uri":42},"position":{"line":1,"character":38}}}"#;
    let stdin = format!(
        "Content-Length: {}\r\n\r\n{}Content-Length: {}\r\n\r\n{}",
        open_body.len(),
        open_body,
        hover_body.len(),
        hover_body
    );
    let open_response =
        r#"{"jsonrpc":"2.0","method":"textDocument/didOpen","params":{"uri":42,"sourceBytes":45}}"#;
    let hover_response =
        r#"{"jsonrpc":"2.0","id":66,"result":{"range":[1,36,1,42],"contents":"defn helper"}}"#;
    let expected = format!(
        "Content-Length: {}\r\n\r\n{}Content-Length: {}\r\n\r\n{}",
        open_response.len(),
        open_response,
        hover_response.len(),
        hover_response
    );

    let output = compile_and_run_with_args_and_stdin(
        selfhost_cli_runtime_bundle(),
        &["lsp", "--stdio"],
        &stdin,
    );

    assert_eq!(
        output, expected,
        "Cli main は spec 寄り hover params でも open document state を使うべき"
    );
}

/// TEST-CLI-02-AN10: actual Cli main は `lsp --stdio` で didOpen 後に source なし definition request も open document state から処理できること
#[test]
#[ignore]
fn test_e2e_selfhost_cli_main_with_lsp_stdio_definition_uses_open_document() {
    let source = "(defn helper [x] x) (defn main [] (helper 1))";
    let open_body = format!(
        r#"{{"jsonrpc":"2.0","method":"textDocument/didOpen","params":{{"uri":42,"source":"{}"}}}}"#,
        source
    );
    let definition_body = r#"{"jsonrpc":"2.0","id":67,"method":"textDocument/definition","params":{"uri":42,"line":1,"col":38}}"#;
    let stdin = format!(
        "Content-Length: {}\r\n\r\n{}Content-Length: {}\r\n\r\n{}",
        open_body.len(),
        open_body,
        definition_body.len(),
        definition_body
    );
    let open_response = format!(
        r#"{{"jsonrpc":"2.0","method":"textDocument/didOpen","params":{{"uri":42,"sourceBytes":{}}}}}"#,
        source.len()
    );
    let definition_response = r#"{"jsonrpc":"2.0","id":67,"result":[42,1,7]}"#;
    let expected = format!(
        "Content-Length: {}\r\n\r\n{}Content-Length: {}\r\n\r\n{}",
        open_response.len(),
        open_response,
        definition_response.len(),
        definition_response
    );

    let output = compile_and_run_with_args_and_stdin(
        selfhost_cli_runtime_bundle(),
        &["lsp", "--stdio"],
        &stdin,
    );

    assert_eq!(
        output, expected,
        "Cli main は didOpen 後の source なし definition で open document state を使うべき"
    );
}

/// TEST-CLI-02-AN10b: actual Cli main は spec 寄り definition params でも open document state から処理できること
#[test]
#[ignore]
fn test_e2e_selfhost_cli_main_with_lsp_stdio_definition_uses_open_document_spec_params() {
    let source = "(defn helper [x] x) (defn main [] (helper 1))";
    let open_body = format!(
        r#"{{"jsonrpc":"2.0","method":"textDocument/didOpen","params":{{"uri":42,"source":"{}"}}}}"#,
        source
    );
    let definition_body = r#"{"jsonrpc":"2.0","id":67,"method":"textDocument/definition","params":{"textDocument":{"uri":42},"position":{"line":1,"character":38}}}"#;
    let stdin = format!(
        "Content-Length: {}\r\n\r\n{}Content-Length: {}\r\n\r\n{}",
        open_body.len(),
        open_body,
        definition_body.len(),
        definition_body
    );
    let open_response = format!(
        r#"{{"jsonrpc":"2.0","method":"textDocument/didOpen","params":{{"uri":42,"sourceBytes":{}}}}}"#,
        source.len()
    );
    let definition_response = r#"{"jsonrpc":"2.0","id":67,"result":[42,1,7]}"#;
    let expected = format!(
        "Content-Length: {}\r\n\r\n{}Content-Length: {}\r\n\r\n{}",
        open_response.len(),
        open_response,
        definition_response.len(),
        definition_response
    );

    let output = compile_and_run_with_args_and_stdin(
        selfhost_cli_runtime_bundle(),
        &["lsp", "--stdio"],
        &stdin,
    );

    assert_eq!(
        output, expected,
        "Cli main は spec 寄り definition params でも open document state を使うべき"
    );
}

/// TEST-CLI-02-AN11: actual Cli main は `lsp --stdio` で didOpen 後に source なし references request も open document state から処理できること
#[test]
#[ignore]
fn test_e2e_selfhost_cli_main_with_lsp_stdio_references_uses_open_document() {
    let source = "(defn square [x] x) (defn main [] (square 1) (square 2))";
    let open_body = format!(
        r#"{{"jsonrpc":"2.0","method":"textDocument/didOpen","params":{{"uri":42,"source":"{}"}}}}"#,
        source
    );
    let references_body = r#"{"jsonrpc":"2.0","id":68,"method":"textDocument/references","params":{"uri":42,"line":1,"col":38}}"#;
    let stdin = format!(
        "Content-Length: {}\r\n\r\n{}Content-Length: {}\r\n\r\n{}",
        open_body.len(),
        open_body,
        references_body.len(),
        references_body
    );
    let open_response = format!(
        r#"{{"jsonrpc":"2.0","method":"textDocument/didOpen","params":{{"uri":42,"sourceBytes":{}}}}}"#,
        source.len()
    );
    let open_diagnostics = r#"{"jsonrpc":"2.0","method":"textDocument/publishDiagnostics","params":{"uri":42,"diagnostics":[{"source":1,"severity":1,"rule":1001,"line":1,"col":56,"messageHash":0}]}}"#;
    let references_response =
        r#"{"jsonrpc":"2.0","id":68,"result":[[42,1,7],[42,1,36],[42,1,47]]}"#;
    let expected = format!(
        "Content-Length: {}\r\n\r\n{}Content-Length: {}\r\n\r\n{}Content-Length: {}\r\n\r\n{}",
        open_response.len(),
        open_response,
        open_diagnostics.len(),
        open_diagnostics,
        references_response.len(),
        references_response
    );

    let output = compile_and_run_with_args_and_stdin(
        selfhost_cli_runtime_bundle(),
        &["lsp", "--stdio"],
        &stdin,
    );

    assert_eq!(
        output, expected,
        "Cli main は didOpen 後の source なし references で open document state を使うべき"
    );
}

/// TEST-CLI-02-AN11b: actual Cli main は spec 寄り references params でも open document state から処理できること
#[test]
#[ignore]
fn test_e2e_selfhost_cli_main_with_lsp_stdio_references_uses_open_document_spec_params() {
    let source = "(defn square [x] x) (defn main [] (square 1) (square 2))";
    let open_body = format!(
        r#"{{"jsonrpc":"2.0","method":"textDocument/didOpen","params":{{"uri":42,"source":"{}"}}}}"#,
        source
    );
    let references_body = r#"{"jsonrpc":"2.0","id":68,"method":"textDocument/references","params":{"textDocument":{"uri":42},"position":{"line":1,"character":38}}}"#;
    let stdin = format!(
        "Content-Length: {}\r\n\r\n{}Content-Length: {}\r\n\r\n{}",
        open_body.len(),
        open_body,
        references_body.len(),
        references_body
    );
    let open_response = format!(
        r#"{{"jsonrpc":"2.0","method":"textDocument/didOpen","params":{{"uri":42,"sourceBytes":{}}}}}"#,
        source.len()
    );
    let open_diagnostics = r#"{"jsonrpc":"2.0","method":"textDocument/publishDiagnostics","params":{"uri":42,"diagnostics":[{"source":1,"severity":1,"rule":1001,"line":1,"col":56,"messageHash":0}]}}"#;
    let references_response =
        r#"{"jsonrpc":"2.0","id":68,"result":[[42,1,7],[42,1,36],[42,1,47]]}"#;
    let expected = format!(
        "Content-Length: {}\r\n\r\n{}Content-Length: {}\r\n\r\n{}Content-Length: {}\r\n\r\n{}",
        open_response.len(),
        open_response,
        open_diagnostics.len(),
        open_diagnostics,
        references_response.len(),
        references_response
    );

    let output = compile_and_run_with_args_and_stdin(
        selfhost_cli_runtime_bundle(),
        &["lsp", "--stdio"],
        &stdin,
    );

    assert_eq!(
        output, expected,
        "Cli main は spec 寄り references params でも open document state を使うべき"
    );
}

/// TEST-CLI-02-AN12: actual Cli main は `lsp --stdio` で didOpen 後に source なし formatting request も open document state から処理できること
#[test]
#[ignore]
fn test_e2e_selfhost_cli_main_with_lsp_stdio_formatting_uses_open_document() {
    let source = "(defn main [] 1)";
    let open_body = format!(
        r#"{{"jsonrpc":"2.0","method":"textDocument/didOpen","params":{{"uri":42,"source":"{}"}}}}"#,
        source
    );
    let formatting_body =
        r#"{"jsonrpc":"2.0","id":69,"method":"textDocument/formatting","params":{"uri":42}}"#;
    let stdin = format!(
        "Content-Length: {}\r\n\r\n{}Content-Length: {}\r\n\r\n{}",
        open_body.len(),
        open_body,
        formatting_body.len(),
        formatting_body
    );
    let open_response = format!(
        r#"{{"jsonrpc":"2.0","method":"textDocument/didOpen","params":{{"uri":42,"sourceBytes":{}}}}}"#,
        source.len()
    );
    let formatting_response =
        "{\"jsonrpc\":\"2.0\",\"id\":69,\"result\":[[1,1,1,17,\"(defn main [] 1)\\n\"]]}";
    let expected = format!(
        "Content-Length: {}\r\n\r\n{}Content-Length: {}\r\n\r\n{}",
        open_response.len(),
        open_response,
        formatting_response.len(),
        formatting_response
    );

    let output = compile_and_run_with_args_and_stdin(
        selfhost_cli_runtime_bundle(),
        &["lsp", "--stdio"],
        &stdin,
    );

    assert_eq!(
        output, expected,
        "Cli main は didOpen 後の source なし formatting で open document state を使うべき"
    );
}

/// TEST-CLI-02-AN13: actual Cli main は `lsp --stdio` で didOpen 後に source なし rename request も open document state から処理できること
#[test]
#[ignore]
fn test_e2e_selfhost_cli_main_with_lsp_stdio_rename_uses_open_document() {
    let source = "(defn square [x] x) (defn main [] (square 1) (square 2))";
    let open_body = format!(
        r#"{{"jsonrpc":"2.0","method":"textDocument/didOpen","params":{{"uri":42,"source":"{}"}}}}"#,
        source
    );
    let rename_body = r#"{"jsonrpc":"2.0","id":70,"method":"textDocument/rename","params":{"uri":42,"line":1,"col":38,"newName":"cube"}}"#;
    let stdin = format!(
        "Content-Length: {}\r\n\r\n{}Content-Length: {}\r\n\r\n{}",
        open_body.len(),
        open_body,
        rename_body.len(),
        rename_body
    );
    let open_response = format!(
        r#"{{"jsonrpc":"2.0","method":"textDocument/didOpen","params":{{"uri":42,"sourceBytes":{}}}}}"#,
        source.len()
    );
    let open_diagnostics = r#"{"jsonrpc":"2.0","method":"textDocument/publishDiagnostics","params":{"uri":42,"diagnostics":[{"source":1,"severity":1,"rule":1001,"line":1,"col":56,"messageHash":0}]}}"#;
    let rename_response = r#"{"jsonrpc":"2.0","id":70,"result":[[42,[[1,7,1,13,"cube"],[1,36,1,42,"cube"],[1,47,1,53,"cube"]]]]}"#;
    let expected = format!(
        "Content-Length: {}\r\n\r\n{}Content-Length: {}\r\n\r\n{}Content-Length: {}\r\n\r\n{}",
        open_response.len(),
        open_response,
        open_diagnostics.len(),
        open_diagnostics,
        rename_response.len(),
        rename_response
    );

    let output = compile_and_run_with_args_and_stdin(
        selfhost_cli_runtime_bundle(),
        &["lsp", "--stdio"],
        &stdin,
    );

    assert_eq!(
        output, expected,
        "Cli main は didOpen 後の source なし rename で open document state を使うべき"
    );
}

/// TEST-CLI-02-AN13b: actual Cli main は spec 寄り rename params でも open document state から処理できること
#[test]
#[ignore]
fn test_e2e_selfhost_cli_main_with_lsp_stdio_rename_uses_open_document_spec_params() {
    let source = "(defn square [x] x) (defn main [] (square 1) (square 2))";
    let open_body = format!(
        r#"{{"jsonrpc":"2.0","method":"textDocument/didOpen","params":{{"uri":42,"source":"{}"}}}}"#,
        source
    );
    let rename_body = r#"{"jsonrpc":"2.0","id":70,"method":"textDocument/rename","params":{"textDocument":{"uri":42},"position":{"line":1,"character":38},"newName":"cube"}}"#;
    let stdin = format!(
        "Content-Length: {}\r\n\r\n{}Content-Length: {}\r\n\r\n{}",
        open_body.len(),
        open_body,
        rename_body.len(),
        rename_body
    );
    let open_response = format!(
        r#"{{"jsonrpc":"2.0","method":"textDocument/didOpen","params":{{"uri":42,"sourceBytes":{}}}}}"#,
        source.len()
    );
    let open_diagnostics = r#"{"jsonrpc":"2.0","method":"textDocument/publishDiagnostics","params":{"uri":42,"diagnostics":[{"source":1,"severity":1,"rule":1001,"line":1,"col":56,"messageHash":0}]}}"#;
    let rename_response = r#"{"jsonrpc":"2.0","id":70,"result":[[42,[[1,7,1,13,"cube"],[1,36,1,42,"cube"],[1,47,1,53,"cube"]]]]}"#;
    let expected = format!(
        "Content-Length: {}\r\n\r\n{}Content-Length: {}\r\n\r\n{}Content-Length: {}\r\n\r\n{}",
        open_response.len(),
        open_response,
        open_diagnostics.len(),
        open_diagnostics,
        rename_response.len(),
        rename_response
    );

    let output = compile_and_run_with_args_and_stdin(
        selfhost_cli_runtime_bundle(),
        &["lsp", "--stdio"],
        &stdin,
    );

    assert_eq!(
        output, expected,
        "Cli main は spec 寄り rename params でも open document state を使うべき"
    );
}

/// TEST-CLI-02-AN14: actual Cli main は `lsp --stdio` で didOpen 後に source なし completion request も open document state から処理できること
#[test]
#[ignore]
fn test_e2e_selfhost_cli_main_with_lsp_stdio_completion_uses_open_document() {
    let source = "(defn helper [] 1) (he)";
    let open_body = format!(
        r#"{{"jsonrpc":"2.0","method":"textDocument/didOpen","params":{{"uri":42,"source":"{}"}}}}"#,
        source
    );
    let completion_body = r#"{"jsonrpc":"2.0","id":71,"method":"textDocument/completion","params":{"uri":42,"line":1,"col":23}}"#;
    let stdin = format!(
        "Content-Length: {}\r\n\r\n{}Content-Length: {}\r\n\r\n{}",
        open_body.len(),
        open_body,
        completion_body.len(),
        completion_body
    );
    let open_response = format!(
        r#"{{"jsonrpc":"2.0","method":"textDocument/didOpen","params":{{"uri":42,"sourceBytes":{}}}}}"#,
        source.len()
    );
    let completion_response = r#"{"jsonrpc":"2.0","id":71,"result":[["helper",3,"helper"]]}"#;
    let expected = format!(
        "Content-Length: {}\r\n\r\n{}Content-Length: {}\r\n\r\n{}",
        open_response.len(),
        open_response,
        completion_response.len(),
        completion_response
    );

    let output = compile_and_run_with_args_and_stdin(
        selfhost_cli_runtime_bundle(),
        &["lsp", "--stdio"],
        &stdin,
    );

    assert_eq!(
        output, expected,
        "Cli main は didOpen 後の source なし completion で open document state を使うべき"
    );
}

/// TEST-CLI-02-AN14b: actual Cli main は spec 寄り completion params でも open document state から処理できること
#[test]
#[ignore]
fn test_e2e_selfhost_cli_main_with_lsp_stdio_completion_uses_open_document_spec_params() {
    let source = "(defn helper [] 1) (he)";
    let open_body = format!(
        r#"{{"jsonrpc":"2.0","method":"textDocument/didOpen","params":{{"uri":42,"source":"{}"}}}}"#,
        source
    );
    let completion_body = r#"{"jsonrpc":"2.0","id":71,"method":"textDocument/completion","params":{"textDocument":{"uri":42},"position":{"line":1,"character":23}}}"#;
    let stdin = format!(
        "Content-Length: {}\r\n\r\n{}Content-Length: {}\r\n\r\n{}",
        open_body.len(),
        open_body,
        completion_body.len(),
        completion_body
    );
    let open_response = format!(
        r#"{{"jsonrpc":"2.0","method":"textDocument/didOpen","params":{{"uri":42,"sourceBytes":{}}}}}"#,
        source.len()
    );
    let completion_response = r#"{"jsonrpc":"2.0","id":71,"result":[["helper",3,"helper"]]}"#;
    let expected = format!(
        "Content-Length: {}\r\n\r\n{}Content-Length: {}\r\n\r\n{}",
        open_response.len(),
        open_response,
        completion_response.len(),
        completion_response
    );

    let output = compile_and_run_with_args_and_stdin(
        selfhost_cli_runtime_bundle(),
        &["lsp", "--stdio"],
        &stdin,
    );

    assert_eq!(
        output, expected,
        "Cli main は spec 寄り completion params でも open document state を使うべき"
    );
}

/// TEST-CLI-02-AN14c: actual Cli main は spec 寄り didOpen `textDocument.text` の
/// escaped quote を含む source でも formatting へ正しく渡せること
#[test]
#[ignore]
fn test_e2e_selfhost_cli_main_with_lsp_stdio_formatting_uses_spec_document_text_with_escaped_quote()
{
    let source = r#"(defn main [] "a\"b")"#;
    let open_body = serde_json::json!({
        "jsonrpc": "2.0",
        "method": "textDocument/didOpen",
        "params": {
            "textDocument": {
                "uri": 42,
                "languageId": "lsharp",
                "version": 1,
                "text": source
            }
        }
    })
    .to_string();
    let formatting_body = serde_json::json!({
        "jsonrpc": "2.0",
        "id": 169,
        "method": "textDocument/formatting",
        "params": {
            "uri": 42
        }
    })
    .to_string();
    let stdin = format!("{}{}", lsp_frame(&open_body), lsp_frame(&formatting_body));

    let output = compile_and_run_with_args_and_stdin(
        selfhost_cli_runtime_bundle(),
        &["lsp", "--stdio"],
        &stdin,
    );

    let formatted = format!("{source}\n");
    let frames = parse_lsp_stdio_frames(&output);
    let expected = vec![
        serde_json::json!({
            "jsonrpc": "2.0",
            "method": "textDocument/didOpen",
            "params": {
                "uri": 42,
                "sourceBytes": source.len()
            }
        }),
        serde_json::json!({
            "jsonrpc": "2.0",
            "id": 169,
            "result": [[1, 1, 1, source.len() + 1, formatted]]
        }),
    ];

    assert_eq!(
        frames, expected,
        "Cli main は escaped quote を含む spec document text でも formatting へ同じ source を渡すべき"
    );
}

/// TEST-CLI-02-AN14d: actual Cli main は spec 寄り didOpen `textDocument.text` の
/// unicode escaped quote (`\u0022`) でも formatting へ正しく渡せること
#[test]
#[ignore]
fn test_e2e_selfhost_cli_main_with_lsp_stdio_formatting_uses_spec_document_text_with_unicode_escaped_quote()
 {
    let source = r#"(defn main [] "ab")"#;
    let open_body = r#"{"jsonrpc":"2.0","method":"textDocument/didOpen","params":{"textDocument":{"uri":42,"languageId":"lsharp","version":1,"text":"(defn main [] \u0022ab\u0022)"}}}"#;
    let formatting_body = serde_json::json!({
        "jsonrpc": "2.0",
        "id": 171,
        "method": "textDocument/formatting",
        "params": {
            "uri": 42
        }
    })
    .to_string();
    let stdin = format!("{}{}", lsp_frame(open_body), lsp_frame(&formatting_body));

    let output = compile_and_run_with_args_and_stdin(
        selfhost_cli_runtime_bundle(),
        &["lsp", "--stdio"],
        &stdin,
    );

    let formatted = format!("{source}\n");
    let frames = parse_lsp_stdio_frames(&output);
    let expected = vec![
        serde_json::json!({
            "jsonrpc": "2.0",
            "method": "textDocument/didOpen",
            "params": {
                "uri": 42,
                "sourceBytes": source.len()
            }
        }),
        serde_json::json!({
            "jsonrpc": "2.0",
            "id": 171,
            "result": [[1, 1, 1, source.len() + 1, formatted]]
        }),
    ];

    assert_eq!(
        frames, expected,
        "Cli main は unicode escaped quote を含む spec document text でも formatting へ同じ source を渡すべき"
    );
}

/// TEST-CLI-02-AN14e: actual Cli main は didOpen 後の formatting で defn metadata を保持すること
#[test]
#[ignore]
fn test_e2e_selfhost_cli_main_with_lsp_stdio_formatting_preserves_defn_metadata() {
    let source = r#"(defn add [x y] :doc "Add two ints" :params [(x "left") (y "right")] :returns "sum" :example [(add 1 2)] (+ x y))"#;
    let open_body = serde_json::json!({
        "jsonrpc": "2.0",
        "method": "textDocument/didOpen",
        "params": {
            "textDocument": {
                "uri": 42,
                "languageId": "lsharp",
                "version": 1,
                "text": source
            }
        }
    })
    .to_string();
    let formatting_body = serde_json::json!({
        "jsonrpc": "2.0",
        "id": 173,
        "method": "textDocument/formatting",
        "params": {
            "uri": 42
        }
    })
    .to_string();
    let stdin = format!("{}{}", lsp_frame(&open_body), lsp_frame(&formatting_body));

    let output = compile_and_run_with_args_and_stdin(
        selfhost_cli_runtime_bundle(),
        &["lsp", "--stdio"],
        &stdin,
    );

    let formatted = "(defn add [x y] :params [(x \"left\") (y \"right\")] :returns \"sum\" :doc \"Add two ints\" :example [(add 1 2)] (+ x y))\n";
    let frames = parse_lsp_stdio_frames(&output);
    let expected = vec![
        serde_json::json!({
            "jsonrpc": "2.0",
            "method": "textDocument/didOpen",
            "params": {
                "uri": 42,
                "sourceBytes": source.len()
            }
        }),
        serde_json::json!({
            "jsonrpc": "2.0",
            "id": 173,
            "result": [[1, 1, 1, source.len() + 1, formatted]]
        }),
    ];

    assert_eq!(
        frames, expected,
        "Cli main は LSP formatting でも defn metadata を canonical 順で保持するべき"
    );
}

/// TEST-CLI-02-AN15: actual Cli main は `lsp --stdio` で open 済み別 document から source なし definition を解決できること
#[test]
#[ignore]
fn test_e2e_selfhost_cli_main_with_lsp_stdio_definition_resolves_open_document() {
    let helper_source = "(defn helper [x] x)";
    let main_source = "(helper 1)";
    let open_helper_body = format!(
        r#"{{"jsonrpc":"2.0","method":"textDocument/didOpen","params":{{"uri":11,"source":"{}"}}}}"#,
        helper_source
    );
    let open_main_body = format!(
        r#"{{"jsonrpc":"2.0","method":"textDocument/didOpen","params":{{"uri":10,"source":"{}"}}}}"#,
        main_source
    );
    let definition_body = r#"{"jsonrpc":"2.0","id":72,"method":"textDocument/definition","params":{"uri":10,"line":1,"col":2}}"#;
    let stdin = format!(
        "Content-Length: {}\r\n\r\n{}Content-Length: {}\r\n\r\n{}Content-Length: {}\r\n\r\n{}",
        open_helper_body.len(),
        open_helper_body,
        open_main_body.len(),
        open_main_body,
        definition_body.len(),
        definition_body
    );
    let open_helper_response = format!(
        r#"{{"jsonrpc":"2.0","method":"textDocument/didOpen","params":{{"uri":11,"sourceBytes":{}}}}}"#,
        helper_source.len()
    );
    let open_main_response = format!(
        r#"{{"jsonrpc":"2.0","method":"textDocument/didOpen","params":{{"uri":10,"sourceBytes":{}}}}}"#,
        main_source.len()
    );
    let definition_response = r#"{"jsonrpc":"2.0","id":72,"result":[11,1,7]}"#;
    let expected = format!(
        "Content-Length: {}\r\n\r\n{}Content-Length: {}\r\n\r\n{}Content-Length: {}\r\n\r\n{}",
        open_helper_response.len(),
        open_helper_response,
        open_main_response.len(),
        open_main_response,
        definition_response.len(),
        definition_response
    );

    let output = compile_and_run_with_args_and_stdin(
        selfhost_cli_runtime_bundle(),
        &["lsp", "--stdio"],
        &stdin,
    );

    assert_eq!(
        output, expected,
        "Cli main は open 済み別 document から source なし definition を解決すべき"
    );
}

/// TEST-CLI-02-AN16: actual Cli main は `lsp --stdio` で open 済み別 document から source なし hover contents を解決できること
#[test]
#[ignore]
fn test_e2e_selfhost_cli_main_with_lsp_stdio_hover_resolves_open_document() {
    let helper_source = "(defn helper [x] x)";
    let main_source = "(helper 1)";
    let open_helper_body = format!(
        r#"{{"jsonrpc":"2.0","method":"textDocument/didOpen","params":{{"uri":11,"source":"{}"}}}}"#,
        helper_source
    );
    let open_main_body = format!(
        r#"{{"jsonrpc":"2.0","method":"textDocument/didOpen","params":{{"uri":10,"source":"{}"}}}}"#,
        main_source
    );
    let hover_body = r#"{"jsonrpc":"2.0","id":73,"method":"textDocument/hover","params":{"uri":10,"line":1,"col":2}}"#;
    let stdin = format!(
        "Content-Length: {}\r\n\r\n{}Content-Length: {}\r\n\r\n{}Content-Length: {}\r\n\r\n{}",
        open_helper_body.len(),
        open_helper_body,
        open_main_body.len(),
        open_main_body,
        hover_body.len(),
        hover_body
    );
    let open_helper_response = format!(
        r#"{{"jsonrpc":"2.0","method":"textDocument/didOpen","params":{{"uri":11,"sourceBytes":{}}}}}"#,
        helper_source.len()
    );
    let open_main_response = format!(
        r#"{{"jsonrpc":"2.0","method":"textDocument/didOpen","params":{{"uri":10,"sourceBytes":{}}}}}"#,
        main_source.len()
    );
    let hover_response =
        r#"{"jsonrpc":"2.0","id":73,"result":{"range":[1,2,1,8],"contents":"defn helper"}}"#;
    let expected = format!(
        "Content-Length: {}\r\n\r\n{}Content-Length: {}\r\n\r\n{}Content-Length: {}\r\n\r\n{}",
        open_helper_response.len(),
        open_helper_response,
        open_main_response.len(),
        open_main_response,
        hover_response.len(),
        hover_response
    );

    let output = compile_and_run_with_args_and_stdin(
        selfhost_cli_runtime_bundle(),
        &["lsp", "--stdio"],
        &stdin,
    );

    assert_eq!(
        output, expected,
        "Cli main は open 済み別 document から source なし hover contents を解決すべき"
    );
}

/// TEST-CLI-02-AN17: actual Cli main は `lsp --stdio` で didChange 後の source なし completion に最新 document state を使うこと
#[test]
#[ignore]
fn test_e2e_selfhost_cli_main_with_lsp_stdio_completion_uses_changed_document() {
    let open_source = "(defn alpha [] 1) (al)";
    let changed_source = "(defn helper [] 1) (he)";
    let open_body = format!(
        r#"{{"jsonrpc":"2.0","method":"textDocument/didOpen","params":{{"uri":42,"source":"{}"}}}}"#,
        open_source
    );
    let change_body = format!(
        r#"{{"jsonrpc":"2.0","method":"textDocument/didChange","params":{{"uri":42,"source":"{}"}}}}"#,
        changed_source
    );
    let completion_body = r#"{"jsonrpc":"2.0","id":74,"method":"textDocument/completion","params":{"uri":42,"line":1,"col":23}}"#;
    let stdin = format!(
        "Content-Length: {}\r\n\r\n{}Content-Length: {}\r\n\r\n{}Content-Length: {}\r\n\r\n{}",
        open_body.len(),
        open_body,
        change_body.len(),
        change_body,
        completion_body.len(),
        completion_body
    );
    let open_response = format!(
        r#"{{"jsonrpc":"2.0","method":"textDocument/didOpen","params":{{"uri":42,"sourceBytes":{}}}}}"#,
        open_source.len()
    );
    let change_response = format!(
        r#"{{"jsonrpc":"2.0","method":"textDocument/didChange","params":{{"uri":42,"sourceBytes":{}}}}}"#,
        changed_source.len()
    );
    let completion_response = r#"{"jsonrpc":"2.0","id":74,"result":[["helper",3,"helper"]]}"#;
    let expected = format!(
        "Content-Length: {}\r\n\r\n{}Content-Length: {}\r\n\r\n{}Content-Length: {}\r\n\r\n{}",
        open_response.len(),
        open_response,
        change_response.len(),
        change_response,
        completion_response.len(),
        completion_response
    );

    let output = compile_and_run_with_args_and_stdin(
        selfhost_cli_runtime_bundle(),
        &["lsp", "--stdio"],
        &stdin,
    );

    assert_eq!(
        output, expected,
        "Cli main は didChange 後の source なし completion で最新 document state を使うべき"
    );
}

/// TEST-CLI-02-AN17b: actual Cli main は spec 寄り `contentChanges[0].text` の
/// escaped newline でも didChange 後の最新 source を使うこと
#[test]
#[ignore]
fn test_e2e_selfhost_cli_main_with_lsp_stdio_completion_uses_spec_changed_document_with_escaped_newline()
 {
    let open_source = "(defn alpha [] 1) (al)";
    let changed_source = "(defn helper [] 1)\n(he)";
    let open_body = serde_json::json!({
        "jsonrpc": "2.0",
        "method": "textDocument/didOpen",
        "params": {
            "textDocument": {
                "uri": 42,
                "languageId": "lsharp",
                "version": 1,
                "text": open_source
            }
        }
    })
    .to_string();
    let change_body = serde_json::json!({
        "jsonrpc": "2.0",
        "method": "textDocument/didChange",
        "params": {
            "textDocument": {
                "uri": 42,
                "version": 2
            },
            "contentChanges": [
                {
                    "text": changed_source
                }
            ]
        }
    })
    .to_string();
    let completion_body = serde_json::json!({
        "jsonrpc": "2.0",
        "id": 170,
        "method": "textDocument/completion",
        "params": {
            "textDocument": {
                "uri": 42
            },
            "position": {
                "line": 2,
                "character": 4
            }
        }
    })
    .to_string();
    let stdin = format!(
        "{}{}{}",
        lsp_frame(&open_body),
        lsp_frame(&change_body),
        lsp_frame(&completion_body)
    );

    let output = compile_and_run_with_args_and_stdin(
        selfhost_cli_runtime_bundle(),
        &["lsp", "--stdio"],
        &stdin,
    );

    let frames = parse_lsp_stdio_frames(&output);
    let expected = vec![
        serde_json::json!({
            "jsonrpc": "2.0",
            "method": "textDocument/didOpen",
            "params": {
                "uri": 42,
                "sourceBytes": open_source.len()
            }
        }),
        serde_json::json!({
            "jsonrpc": "2.0",
            "method": "textDocument/didChange",
            "params": {
                "uri": 42,
                "sourceBytes": changed_source.len()
            }
        }),
        serde_json::json!({
            "jsonrpc": "2.0",
            "id": 170,
            "result": [["helper", 3, "helper"]]
        }),
    ];

    assert_eq!(
        frames, expected,
        "Cli main は escaped newline を含む spec didChange text でも最新 completion source を使うべき"
    );
}

/// TEST-CLI-02-AN17c: actual Cli main は spec 寄り `contentChanges[0].text` の
/// unicode escaped newline (`\u000a`) でも didChange 後の最新 source を使うこと
#[test]
#[ignore]
fn test_e2e_selfhost_cli_main_with_lsp_stdio_completion_uses_spec_changed_document_with_unicode_escaped_newline()
 {
    let open_source = "(defn alpha [] 1) (al)";
    let changed_source = "(defn helper [] 1)\n(he)";
    let open_body = serde_json::json!({
        "jsonrpc": "2.0",
        "method": "textDocument/didOpen",
        "params": {
            "textDocument": {
                "uri": 42,
                "languageId": "lsharp",
                "version": 1,
                "text": open_source
            }
        }
    })
    .to_string();
    let change_body = r#"{"jsonrpc":"2.0","method":"textDocument/didChange","params":{"textDocument":{"uri":42,"version":2},"contentChanges":[{"text":"(defn helper [] 1)\u000a(he)"}]}}"#;
    let completion_body = serde_json::json!({
        "jsonrpc": "2.0",
        "id": 172,
        "method": "textDocument/completion",
        "params": {
            "textDocument": {
                "uri": 42
            },
            "position": {
                "line": 2,
                "character": 4
            }
        }
    })
    .to_string();
    let stdin = format!(
        "{}{}{}",
        lsp_frame(&open_body),
        lsp_frame(change_body),
        lsp_frame(&completion_body)
    );

    let output = compile_and_run_with_args_and_stdin(
        selfhost_cli_runtime_bundle(),
        &["lsp", "--stdio"],
        &stdin,
    );

    let frames = parse_lsp_stdio_frames(&output);
    let expected = vec![
        serde_json::json!({
            "jsonrpc": "2.0",
            "method": "textDocument/didOpen",
            "params": {
                "uri": 42,
                "sourceBytes": open_source.len()
            }
        }),
        serde_json::json!({
            "jsonrpc": "2.0",
            "method": "textDocument/didChange",
            "params": {
                "uri": 42,
                "sourceBytes": changed_source.len()
            }
        }),
        serde_json::json!({
            "jsonrpc": "2.0",
            "id": 172,
            "result": [["helper", 3, "helper"]]
        }),
    ];

    assert_eq!(
        frames, expected,
        "Cli main は unicode escaped newline を含む spec didChange text でも最新 completion source を使うべき"
    );
}

/// TEST-CLI-02-AN18: actual Cli main は `lsp --stdio` で same-URI repeated didOpen 後に最新 source を使うこと
#[test]
#[ignore]
fn test_e2e_selfhost_cli_main_with_lsp_stdio_repeated_didopen_keeps_latest_source() {
    let first_source = "(defn alpha [] 1) (al)";
    let latest_source = "(defn beta [] 1) (be)";
    let first_open_body = format!(
        r#"{{"jsonrpc":"2.0","method":"textDocument/didOpen","params":{{"uri":42,"source":"{}"}}}}"#,
        first_source
    );
    let second_open_body = format!(
        r#"{{"jsonrpc":"2.0","method":"textDocument/didOpen","params":{{"uri":42,"source":"{}"}}}}"#,
        latest_source
    );
    let completion_body = r#"{"jsonrpc":"2.0","id":75,"method":"textDocument/completion","params":{"uri":42,"line":1,"col":21}}"#;
    let stdin = format!(
        "Content-Length: {}\r\n\r\n{}Content-Length: {}\r\n\r\n{}Content-Length: {}\r\n\r\n{}",
        first_open_body.len(),
        first_open_body,
        second_open_body.len(),
        second_open_body,
        completion_body.len(),
        completion_body
    );
    let first_open_response = format!(
        r#"{{"jsonrpc":"2.0","method":"textDocument/didOpen","params":{{"uri":42,"sourceBytes":{}}}}}"#,
        first_source.len()
    );
    let second_open_response = format!(
        r#"{{"jsonrpc":"2.0","method":"textDocument/didOpen","params":{{"uri":42,"sourceBytes":{}}}}}"#,
        latest_source.len()
    );
    let completion_response = r#"{"jsonrpc":"2.0","id":75,"result":[["beta",3,"beta"]]}"#;
    let expected = format!(
        "Content-Length: {}\r\n\r\n{}Content-Length: {}\r\n\r\n{}Content-Length: {}\r\n\r\n{}",
        first_open_response.len(),
        first_open_response,
        second_open_response.len(),
        second_open_response,
        completion_response.len(),
        completion_response
    );

    let output = compile_and_run_with_args_and_stdin(
        selfhost_cli_runtime_bundle(),
        &["lsp", "--stdio"],
        &stdin,
    );

    assert_eq!(
        output, expected,
        "Cli main は same-URI repeated didOpen 後に最新 source を保持するべき"
    );
}

/// TEST-CLI-02-AN19: actual Cli main は `lsp --stdio` で didChange 後の source なし hover に最新 document state を使うこと
#[test]
#[ignore]
fn test_e2e_selfhost_cli_main_with_lsp_stdio_hover_uses_changed_document() {
    let open_source = "(defn alpha [x] x) (defn main [] (alpha 1))";
    let changed_source = "(defn helper [x] x) (defn main [] (helper 1))";
    let open_body = format!(
        r#"{{"jsonrpc":"2.0","method":"textDocument/didOpen","params":{{"uri":42,"source":"{}"}}}}"#,
        open_source
    );
    let change_body = format!(
        r#"{{"jsonrpc":"2.0","method":"textDocument/didChange","params":{{"uri":42,"source":"{}"}}}}"#,
        changed_source
    );
    let hover_body = r#"{"jsonrpc":"2.0","id":76,"method":"textDocument/hover","params":{"uri":42,"line":1,"col":38}}"#;
    let stdin = format!(
        "Content-Length: {}\r\n\r\n{}Content-Length: {}\r\n\r\n{}Content-Length: {}\r\n\r\n{}",
        open_body.len(),
        open_body,
        change_body.len(),
        change_body,
        hover_body.len(),
        hover_body
    );
    let open_response = format!(
        r#"{{"jsonrpc":"2.0","method":"textDocument/didOpen","params":{{"uri":42,"sourceBytes":{}}}}}"#,
        open_source.len()
    );
    let change_response = format!(
        r#"{{"jsonrpc":"2.0","method":"textDocument/didChange","params":{{"uri":42,"sourceBytes":{}}}}}"#,
        changed_source.len()
    );
    let hover_response =
        r#"{"jsonrpc":"2.0","id":76,"result":{"range":[1,36,1,42],"contents":"defn helper"}}"#;
    let expected = format!(
        "Content-Length: {}\r\n\r\n{}Content-Length: {}\r\n\r\n{}Content-Length: {}\r\n\r\n{}",
        open_response.len(),
        open_response,
        change_response.len(),
        change_response,
        hover_response.len(),
        hover_response
    );

    let output = compile_and_run_with_args_and_stdin(
        selfhost_cli_runtime_bundle(),
        &["lsp", "--stdio"],
        &stdin,
    );

    assert_eq!(
        output, expected,
        "Cli main は didChange 後の source なし hover で最新 document state を使うべき"
    );
}

/// TEST-CLI-02-AN20: actual Cli main は `lsp --stdio` で same-URI repeated didOpen 後の source なし definition に最新 source を使うこと
#[test]
#[ignore]
fn test_e2e_selfhost_cli_main_with_lsp_stdio_definition_uses_latest_reopened_document() {
    let first_source = "(defn alpha [x] x) (defn main [] (alpha 1))";
    let latest_source = "(defn helper [x] x) (defn main [] (helper 1))";
    let first_open_body = format!(
        r#"{{"jsonrpc":"2.0","method":"textDocument/didOpen","params":{{"uri":42,"source":"{}"}}}}"#,
        first_source
    );
    let second_open_body = format!(
        r#"{{"jsonrpc":"2.0","method":"textDocument/didOpen","params":{{"uri":42,"source":"{}"}}}}"#,
        latest_source
    );
    let definition_body = r#"{"jsonrpc":"2.0","id":77,"method":"textDocument/definition","params":{"uri":42,"line":1,"col":38}}"#;
    let stdin = format!(
        "Content-Length: {}\r\n\r\n{}Content-Length: {}\r\n\r\n{}Content-Length: {}\r\n\r\n{}",
        first_open_body.len(),
        first_open_body,
        second_open_body.len(),
        second_open_body,
        definition_body.len(),
        definition_body
    );
    let first_open_response = format!(
        r#"{{"jsonrpc":"2.0","method":"textDocument/didOpen","params":{{"uri":42,"sourceBytes":{}}}}}"#,
        first_source.len()
    );
    let second_open_response = format!(
        r#"{{"jsonrpc":"2.0","method":"textDocument/didOpen","params":{{"uri":42,"sourceBytes":{}}}}}"#,
        latest_source.len()
    );
    let definition_response = r#"{"jsonrpc":"2.0","id":77,"result":[42,1,7]}"#;
    let expected = format!(
        "Content-Length: {}\r\n\r\n{}Content-Length: {}\r\n\r\n{}Content-Length: {}\r\n\r\n{}",
        first_open_response.len(),
        first_open_response,
        second_open_response.len(),
        second_open_response,
        definition_response.len(),
        definition_response
    );

    let output = compile_and_run_with_args_and_stdin(
        selfhost_cli_runtime_bundle(),
        &["lsp", "--stdio"],
        &stdin,
    );

    assert_eq!(
        output, expected,
        "Cli main は same-URI repeated didOpen 後の source なし definition で最新 source を使うべき"
    );
}

/// TEST-CLI-02-AN21: actual Cli main は `lsp --stdio` で didChange 後の source なし definition に最新 document state を使うこと
#[test]
#[ignore]
fn test_e2e_selfhost_cli_main_with_lsp_stdio_definition_uses_changed_document() {
    let open_source = "(defn alpha [x] x) (defn main [] (alpha 1))";
    let changed_source = "(defn helper [x] x) (defn main [] (helper 1))";
    let open_body = format!(
        r#"{{"jsonrpc":"2.0","method":"textDocument/didOpen","params":{{"uri":42,"source":"{}"}}}}"#,
        open_source
    );
    let change_body = format!(
        r#"{{"jsonrpc":"2.0","method":"textDocument/didChange","params":{{"uri":42,"source":"{}"}}}}"#,
        changed_source
    );
    let definition_body = r#"{"jsonrpc":"2.0","id":78,"method":"textDocument/definition","params":{"uri":42,"line":1,"col":38}}"#;
    let stdin = format!(
        "Content-Length: {}\r\n\r\n{}Content-Length: {}\r\n\r\n{}Content-Length: {}\r\n\r\n{}",
        open_body.len(),
        open_body,
        change_body.len(),
        change_body,
        definition_body.len(),
        definition_body
    );
    let open_response = format!(
        r#"{{"jsonrpc":"2.0","method":"textDocument/didOpen","params":{{"uri":42,"sourceBytes":{}}}}}"#,
        open_source.len()
    );
    let change_response = format!(
        r#"{{"jsonrpc":"2.0","method":"textDocument/didChange","params":{{"uri":42,"sourceBytes":{}}}}}"#,
        changed_source.len()
    );
    let definition_response = r#"{"jsonrpc":"2.0","id":78,"result":[42,1,7]}"#;
    let expected = format!(
        "Content-Length: {}\r\n\r\n{}Content-Length: {}\r\n\r\n{}Content-Length: {}\r\n\r\n{}",
        open_response.len(),
        open_response,
        change_response.len(),
        change_response,
        definition_response.len(),
        definition_response
    );

    let output = compile_and_run_with_args_and_stdin(
        selfhost_cli_runtime_bundle(),
        &["lsp", "--stdio"],
        &stdin,
    );

    assert_eq!(
        output, expected,
        "Cli main は didChange 後の source なし definition で最新 document state を使うべき"
    );
}

/// TEST-CLI-02-AN21b: actual Cli main は `lsp --stdio` で didChange 後の source なし references に最新 document state を使うこと
#[test]
#[ignore]
fn test_e2e_selfhost_cli_main_with_lsp_stdio_references_uses_changed_document() {
    let open_source = "(defn alpha [x] x) (defn main [] (alpha 1))";
    let changed_source = "(defn helper [x] x) (defn main [] (do (helper 1) (helper 2)))";
    let open_body = format!(
        r#"{{"jsonrpc":"2.0","method":"textDocument/didOpen","params":{{"uri":42,"source":"{}"}}}}"#,
        open_source
    );
    let change_body = format!(
        r#"{{"jsonrpc":"2.0","method":"textDocument/didChange","params":{{"uri":42,"source":"{}"}}}}"#,
        changed_source
    );
    let references_body = r#"{"jsonrpc":"2.0","id":82,"method":"textDocument/references","params":{"uri":42,"line":1,"col":40}}"#;
    let stdin = format!(
        "Content-Length: {}\r\n\r\n{}Content-Length: {}\r\n\r\n{}Content-Length: {}\r\n\r\n{}",
        open_body.len(),
        open_body,
        change_body.len(),
        change_body,
        references_body.len(),
        references_body
    );
    let open_response = format!(
        r#"{{"jsonrpc":"2.0","method":"textDocument/didOpen","params":{{"uri":42,"sourceBytes":{}}}}}"#,
        open_source.len()
    );
    let change_response = format!(
        r#"{{"jsonrpc":"2.0","method":"textDocument/didChange","params":{{"uri":42,"sourceBytes":{}}}}}"#,
        changed_source.len()
    );
    let references_response =
        r#"{"jsonrpc":"2.0","id":82,"result":[[42,1,7],[42,1,40],[42,1,51]]}"#;
    let expected = format!(
        "Content-Length: {}\r\n\r\n{}Content-Length: {}\r\n\r\n{}Content-Length: {}\r\n\r\n{}",
        open_response.len(),
        open_response,
        change_response.len(),
        change_response,
        references_response.len(),
        references_response
    );

    let output = compile_and_run_with_args_and_stdin(
        selfhost_cli_runtime_bundle(),
        &["lsp", "--stdio"],
        &stdin,
    );

    assert_eq!(
        output, expected,
        "Cli main は didChange 後の source なし references で最新 document state を使うべき"
    );
}

/// TEST-CLI-02-AN21c: actual Cli main は `lsp --stdio` で didChange 後の source なし rename に最新 document state を使うこと
#[test]
#[ignore]
fn test_e2e_selfhost_cli_main_with_lsp_stdio_rename_uses_changed_document() {
    let open_source = "(defn alpha [x] x) (defn main [] (alpha 1))";
    let changed_source = "(defn helper [x] x) (defn main [] (do (helper 1) (helper 2)))";
    let open_body = format!(
        r#"{{"jsonrpc":"2.0","method":"textDocument/didOpen","params":{{"uri":42,"source":"{}"}}}}"#,
        open_source
    );
    let change_body = format!(
        r#"{{"jsonrpc":"2.0","method":"textDocument/didChange","params":{{"uri":42,"source":"{}"}}}}"#,
        changed_source
    );
    let rename_body = r#"{"jsonrpc":"2.0","id":84,"method":"textDocument/rename","params":{"uri":42,"line":1,"col":40,"newName":"cube"}}"#;
    let stdin = format!(
        "Content-Length: {}\r\n\r\n{}Content-Length: {}\r\n\r\n{}Content-Length: {}\r\n\r\n{}",
        open_body.len(),
        open_body,
        change_body.len(),
        change_body,
        rename_body.len(),
        rename_body
    );
    let open_response = format!(
        r#"{{"jsonrpc":"2.0","method":"textDocument/didOpen","params":{{"uri":42,"sourceBytes":{}}}}}"#,
        open_source.len()
    );
    let change_response = format!(
        r#"{{"jsonrpc":"2.0","method":"textDocument/didChange","params":{{"uri":42,"sourceBytes":{}}}}}"#,
        changed_source.len()
    );
    let rename_response = r#"{"jsonrpc":"2.0","id":84,"result":[[42,[[1,7,1,13,"cube"],[1,40,1,46,"cube"],[1,51,1,57,"cube"]]]]}"#;
    let expected = format!(
        "Content-Length: {}\r\n\r\n{}Content-Length: {}\r\n\r\n{}Content-Length: {}\r\n\r\n{}",
        open_response.len(),
        open_response,
        change_response.len(),
        change_response,
        rename_response.len(),
        rename_response
    );

    let output = compile_and_run_with_args_and_stdin(
        selfhost_cli_runtime_bundle(),
        &["lsp", "--stdio"],
        &stdin,
    );

    assert_eq!(
        output, expected,
        "Cli main は didChange 後の source なし rename で最新 document state を使うべき"
    );
}

/// TEST-CLI-02-AN22: actual Cli main は `lsp --stdio` で same-URI repeated didOpen 後の source なし hover に最新 source を使うこと
#[test]
#[ignore]
fn test_e2e_selfhost_cli_main_with_lsp_stdio_hover_uses_latest_reopened_document() {
    let first_source = "(defn alpha [x] x) (defn main [] (alpha 1))";
    let latest_source = "(defn helper [x] x) (defn main [] (helper 1))";
    let first_open_body = format!(
        r#"{{"jsonrpc":"2.0","method":"textDocument/didOpen","params":{{"uri":42,"source":"{}"}}}}"#,
        first_source
    );
    let second_open_body = format!(
        r#"{{"jsonrpc":"2.0","method":"textDocument/didOpen","params":{{"uri":42,"source":"{}"}}}}"#,
        latest_source
    );
    let hover_body = r#"{"jsonrpc":"2.0","id":79,"method":"textDocument/hover","params":{"uri":42,"line":1,"col":38}}"#;
    let stdin = format!(
        "Content-Length: {}\r\n\r\n{}Content-Length: {}\r\n\r\n{}Content-Length: {}\r\n\r\n{}",
        first_open_body.len(),
        first_open_body,
        second_open_body.len(),
        second_open_body,
        hover_body.len(),
        hover_body
    );
    let first_open_response = format!(
        r#"{{"jsonrpc":"2.0","method":"textDocument/didOpen","params":{{"uri":42,"sourceBytes":{}}}}}"#,
        first_source.len()
    );
    let second_open_response = format!(
        r#"{{"jsonrpc":"2.0","method":"textDocument/didOpen","params":{{"uri":42,"sourceBytes":{}}}}}"#,
        latest_source.len()
    );
    let hover_response =
        r#"{"jsonrpc":"2.0","id":79,"result":{"range":[1,36,1,42],"contents":"defn helper"}}"#;
    let expected = format!(
        "Content-Length: {}\r\n\r\n{}Content-Length: {}\r\n\r\n{}Content-Length: {}\r\n\r\n{}",
        first_open_response.len(),
        first_open_response,
        second_open_response.len(),
        second_open_response,
        hover_response.len(),
        hover_response
    );

    let output = compile_and_run_with_args_and_stdin(
        selfhost_cli_runtime_bundle(),
        &["lsp", "--stdio"],
        &stdin,
    );

    assert_eq!(
        output, expected,
        "Cli main は same-URI repeated didOpen 後の source なし hover で最新 source を使うべき"
    );
}

/// TEST-CLI-02-AN22b: actual Cli main は `lsp --stdio` で same-URI repeated didOpen 後の source なし references に最新 source を使うこと
#[test]
#[ignore]
fn test_e2e_selfhost_cli_main_with_lsp_stdio_references_uses_latest_reopened_document() {
    let first_source = "(defn alpha [x] x) (defn main [] (alpha 1))";
    let latest_source = "(defn helper [x] x) (defn main [] (do (helper 1) (helper 2)))";
    let first_open_body = format!(
        r#"{{"jsonrpc":"2.0","method":"textDocument/didOpen","params":{{"uri":42,"source":"{}"}}}}"#,
        first_source
    );
    let second_open_body = format!(
        r#"{{"jsonrpc":"2.0","method":"textDocument/didOpen","params":{{"uri":42,"source":"{}"}}}}"#,
        latest_source
    );
    let references_body = r#"{"jsonrpc":"2.0","id":83,"method":"textDocument/references","params":{"uri":42,"line":1,"col":40}}"#;
    let stdin = format!(
        "Content-Length: {}\r\n\r\n{}Content-Length: {}\r\n\r\n{}Content-Length: {}\r\n\r\n{}",
        first_open_body.len(),
        first_open_body,
        second_open_body.len(),
        second_open_body,
        references_body.len(),
        references_body
    );
    let first_open_response = format!(
        r#"{{"jsonrpc":"2.0","method":"textDocument/didOpen","params":{{"uri":42,"sourceBytes":{}}}}}"#,
        first_source.len()
    );
    let second_open_response = format!(
        r#"{{"jsonrpc":"2.0","method":"textDocument/didOpen","params":{{"uri":42,"sourceBytes":{}}}}}"#,
        latest_source.len()
    );
    let references_response =
        r#"{"jsonrpc":"2.0","id":83,"result":[[42,1,7],[42,1,40],[42,1,51]]}"#;
    let expected = format!(
        "Content-Length: {}\r\n\r\n{}Content-Length: {}\r\n\r\n{}Content-Length: {}\r\n\r\n{}",
        first_open_response.len(),
        first_open_response,
        second_open_response.len(),
        second_open_response,
        references_response.len(),
        references_response
    );

    let output = compile_and_run_with_args_and_stdin(
        selfhost_cli_runtime_bundle(),
        &["lsp", "--stdio"],
        &stdin,
    );

    assert_eq!(
        output, expected,
        "Cli main は same-URI repeated didOpen 後の source なし references で最新 source を使うべき"
    );
}

/// TEST-CLI-02-AN22c: actual Cli main は `lsp --stdio` で same-URI repeated didOpen 後の source なし rename に最新 source を使うこと
#[test]
#[ignore]
fn test_e2e_selfhost_cli_main_with_lsp_stdio_rename_uses_latest_reopened_document() {
    let first_source = "(defn alpha [x] x) (defn main [] (alpha 1))";
    let latest_source = "(defn helper [x] x) (defn main [] (do (helper 1) (helper 2)))";
    let first_open_body = format!(
        r#"{{"jsonrpc":"2.0","method":"textDocument/didOpen","params":{{"uri":42,"source":"{}"}}}}"#,
        first_source
    );
    let second_open_body = format!(
        r#"{{"jsonrpc":"2.0","method":"textDocument/didOpen","params":{{"uri":42,"source":"{}"}}}}"#,
        latest_source
    );
    let rename_body = r#"{"jsonrpc":"2.0","id":85,"method":"textDocument/rename","params":{"uri":42,"line":1,"col":40,"newName":"cube"}}"#;
    let stdin = format!(
        "Content-Length: {}\r\n\r\n{}Content-Length: {}\r\n\r\n{}Content-Length: {}\r\n\r\n{}",
        first_open_body.len(),
        first_open_body,
        second_open_body.len(),
        second_open_body,
        rename_body.len(),
        rename_body
    );
    let first_open_response = format!(
        r#"{{"jsonrpc":"2.0","method":"textDocument/didOpen","params":{{"uri":42,"sourceBytes":{}}}}}"#,
        first_source.len()
    );
    let second_open_response = format!(
        r#"{{"jsonrpc":"2.0","method":"textDocument/didOpen","params":{{"uri":42,"sourceBytes":{}}}}}"#,
        latest_source.len()
    );
    let rename_response = r#"{"jsonrpc":"2.0","id":85,"result":[[42,[[1,7,1,13,"cube"],[1,40,1,46,"cube"],[1,51,1,57,"cube"]]]]}"#;
    let expected = format!(
        "Content-Length: {}\r\n\r\n{}Content-Length: {}\r\n\r\n{}Content-Length: {}\r\n\r\n{}",
        first_open_response.len(),
        first_open_response,
        second_open_response.len(),
        second_open_response,
        rename_response.len(),
        rename_response
    );

    let output = compile_and_run_with_args_and_stdin(
        selfhost_cli_runtime_bundle(),
        &["lsp", "--stdio"],
        &stdin,
    );

    assert_eq!(
        output, expected,
        "Cli main は same-URI repeated didOpen 後の source なし rename で最新 source を使うべき"
    );
}

/// TEST-CLI-02-AN23: actual Cli main は `lsp --stdio` completion response を schema snapshot に一致させること
#[test]
#[ignore]
fn test_e2e_selfhost_cli_main_with_lsp_stdio_completion_schema_snapshot() {
    let request_body = r#"{"jsonrpc":"2.0","id":51,"method":"textDocument/completion","params":0}"#;
    let stdin = format!(
        "Content-Length: {}\r\n\r\n{}",
        request_body.len(),
        request_body
    );

    let output = compile_and_run_with_args_and_stdin(
        selfhost_cli_runtime_bundle(),
        &["lsp", "--stdio"],
        &stdin,
    );

    assert_lsp_stdio_snapshot(
        &output,
        "completion.json",
        "Cli main は lsp --stdio completion response schema を snapshot と一致させるべき",
    );
}

/// TEST-CLI-02-AN24: actual Cli main は `lsp --stdio` の open 済み別 document definition response を schema snapshot に一致させること
#[test]
#[ignore]
fn test_e2e_selfhost_cli_main_with_lsp_stdio_definition_open_document_schema_snapshot() {
    let helper_source = "(defn helper [x] x)";
    let main_source = "(helper 1)";
    let open_helper_body = format!(
        r#"{{"jsonrpc":"2.0","method":"textDocument/didOpen","params":{{"uri":11,"source":"{}"}}}}"#,
        helper_source
    );
    let open_main_body = format!(
        r#"{{"jsonrpc":"2.0","method":"textDocument/didOpen","params":{{"uri":10,"source":"{}"}}}}"#,
        main_source
    );
    let definition_body = r#"{"jsonrpc":"2.0","id":72,"method":"textDocument/definition","params":{"uri":10,"line":1,"col":2}}"#;
    let stdin = format!(
        "Content-Length: {}\r\n\r\n{}Content-Length: {}\r\n\r\n{}Content-Length: {}\r\n\r\n{}",
        open_helper_body.len(),
        open_helper_body,
        open_main_body.len(),
        open_main_body,
        definition_body.len(),
        definition_body
    );

    let output = compile_and_run_with_args_and_stdin(
        selfhost_cli_runtime_bundle(),
        &["lsp", "--stdio"],
        &stdin,
    );

    assert_lsp_stdio_snapshot(
        &output,
        "definition-open-document.json",
        "Cli main は lsp --stdio definition open-document response schema を snapshot と一致させるべき",
    );
}

/// TEST-CLI-02-AN25: actual Cli main は `lsp --stdio` formatting response を valid JSON schema snapshot に一致させること
#[test]
#[ignore]
fn test_e2e_selfhost_cli_main_with_lsp_stdio_formatting_open_document_schema_snapshot() {
    let source = "(defn main [] 1)";
    let open_body = format!(
        r#"{{"jsonrpc":"2.0","method":"textDocument/didOpen","params":{{"uri":42,"source":"{}"}}}}"#,
        source
    );
    let formatting_body =
        r#"{"jsonrpc":"2.0","id":69,"method":"textDocument/formatting","params":{"uri":42}}"#;
    let stdin = format!(
        "Content-Length: {}\r\n\r\n{}Content-Length: {}\r\n\r\n{}",
        open_body.len(),
        open_body,
        formatting_body.len(),
        formatting_body
    );

    let output = compile_and_run_with_args_and_stdin(
        selfhost_cli_runtime_bundle(),
        &["lsp", "--stdio"],
        &stdin,
    );

    assert_lsp_stdio_snapshot(
        &output,
        "formatting-open-document.json",
        "Cli main は lsp --stdio formatting response schema を snapshot と一致させるべき",
    );
}

/// TEST-CLI-02-AN26: actual Cli main は `lsp --stdio` hover response を schema snapshot に一致させること
#[test]
#[ignore]
fn test_e2e_selfhost_cli_main_with_lsp_stdio_hover_schema_snapshot() {
    let request_body = r#"{"jsonrpc":"2.0","id":62,"method":"textDocument/hover","params":{"uri":10,"line":1,"col":38,"source":"(defn helper [x] x) (defn main [] (helper 1))"}}"#;
    let stdin = format!(
        "Content-Length: {}\r\n\r\n{}",
        request_body.len(),
        request_body
    );

    let output = compile_and_run_with_args_and_stdin(
        selfhost_cli_runtime_bundle(),
        &["lsp", "--stdio"],
        &stdin,
    );

    assert_lsp_stdio_snapshot(
        &output,
        "hover.json",
        "Cli main は lsp --stdio hover response schema を snapshot と一致させるべき",
    );
}

/// TEST-CLI-02-AN27: actual Cli main は `lsp --stdio` references response を schema snapshot に一致させること
#[test]
#[ignore]
fn test_e2e_selfhost_cli_main_with_lsp_stdio_references_schema_snapshot() {
    let request_body = r#"{"jsonrpc":"2.0","id":63,"method":"textDocument/references","params":{"uri":10,"line":1,"col":38,"source":"(defn square [x] x) (defn main [] (square 1) (square 2))"}}"#;
    let stdin = format!(
        "Content-Length: {}\r\n\r\n{}",
        request_body.len(),
        request_body
    );

    let output = compile_and_run_with_args_and_stdin(
        selfhost_cli_runtime_bundle(),
        &["lsp", "--stdio"],
        &stdin,
    );

    assert_lsp_stdio_snapshot(
        &output,
        "references.json",
        "Cli main は lsp --stdio references response schema を snapshot と一致させるべき",
    );
}

/// TEST-CLI-02-AN28: actual Cli main は `lsp --stdio` rename response を schema snapshot に一致させること
#[test]
#[ignore]
fn test_e2e_selfhost_cli_main_with_lsp_stdio_rename_schema_snapshot() {
    let request_body = r#"{"jsonrpc":"2.0","id":65,"method":"textDocument/rename","params":{"uri":10,"line":1,"col":38,"source":"(defn square [x] x) (defn main [] (square 1) (square 2))","newName":"cube"}}"#;
    let stdin = format!(
        "Content-Length: {}\r\n\r\n{}",
        request_body.len(),
        request_body
    );

    let output = compile_and_run_with_args_and_stdin(
        selfhost_cli_runtime_bundle(),
        &["lsp", "--stdio"],
        &stdin,
    );

    assert_lsp_stdio_snapshot(
        &output,
        "rename.json",
        "Cli main は lsp --stdio rename response schema を snapshot と一致させるべき",
    );
}

/// TEST-CLI-02-AN29: actual Cli main は `lsp --stdio` initialize response を schema snapshot に一致させること
#[test]
#[ignore]
fn test_e2e_selfhost_cli_main_with_lsp_stdio_initialize_schema_snapshot() {
    let request_body = r#"{"jsonrpc":"2.0","id":21,"method":"initialize","params":0}"#;
    let stdin = format!(
        "Content-Length: {}\r\n\r\n{}",
        request_body.len(),
        request_body
    );

    let output = compile_and_run_with_args_and_stdin(
        selfhost_cli_runtime_bundle(),
        &["lsp", "--stdio"],
        &stdin,
    );

    assert_lsp_stdio_snapshot(
        &output,
        "initialize.json",
        "Cli main は lsp --stdio initialize response schema を snapshot と一致させるべき",
    );
}

/// TEST-CLI-02-AN30: actual Cli main は `lsp --stdio` initialize→shutdown sequence を schema snapshot に一致させること
#[test]
#[ignore]
fn test_e2e_selfhost_cli_main_with_lsp_stdio_initialize_shutdown_schema_snapshot() {
    let init_body = r#"{"jsonrpc":"2.0","id":31,"method":"initialize","params":0}"#;
    let shutdown_body = r#"{"jsonrpc":"2.0","id":32,"method":"shutdown","params":0}"#;
    let stdin = format!(
        "Content-Length: {}\r\n\r\n{}Content-Length: {}\r\n\r\n{}",
        init_body.len(),
        init_body,
        shutdown_body.len(),
        shutdown_body
    );

    let output = compile_and_run_with_args_and_stdin(
        selfhost_cli_runtime_bundle(),
        &["lsp", "--stdio"],
        &stdin,
    );

    assert_lsp_stdio_snapshot(
        &output,
        "initialize-shutdown-sequence.json",
        "Cli main は lsp --stdio initialize→shutdown schema を snapshot と一致させるべき",
    );
}

/// TEST-CLI-02-AN31: actual Cli main は `lsp --stdio` unknown method error を schema snapshot に一致させること
#[test]
#[ignore]
fn test_e2e_selfhost_cli_main_with_lsp_stdio_unknown_method_schema_snapshot() {
    let request_body = r#"{"jsonrpc":"2.0","id":41,"method":"workspace/unknown","params":0}"#;
    let stdin = format!(
        "Content-Length: {}\r\n\r\n{}",
        request_body.len(),
        request_body
    );

    let output = compile_and_run_with_args_and_stdin(
        selfhost_cli_runtime_bundle(),
        &["lsp", "--stdio"],
        &stdin,
    );

    assert_lsp_stdio_snapshot(
        &output,
        "unknown-method.json",
        "Cli main は lsp --stdio unknown method error schema を snapshot と一致させるべき",
    );
}

/// TEST-CLI-02-AN31b: actual Cli main は `lsp --stdio` shutdown 後 request error を schema snapshot に一致させること
#[test]
#[ignore]
fn test_e2e_selfhost_cli_main_with_lsp_stdio_request_after_shutdown_schema_snapshot() {
    let shutdown_body = r#"{"jsonrpc":"2.0","id":51,"method":"shutdown","params":0}"#;
    let hover_body = r#"{"jsonrpc":"2.0","id":52,"method":"textDocument/hover","params":{"uri":42,"line":1,"col":1}}"#;
    let stdin = format!(
        "Content-Length: {}\r\n\r\n{}Content-Length: {}\r\n\r\n{}",
        shutdown_body.len(),
        shutdown_body,
        hover_body.len(),
        hover_body
    );

    let output = compile_and_run_with_args_and_stdin(
        selfhost_cli_runtime_bundle(),
        &["lsp", "--stdio"],
        &stdin,
    );

    assert_lsp_stdio_snapshot(
        &output,
        "shutdown-request-after-error.json",
        "Cli main は lsp --stdio shutdown 後 request error schema を snapshot と一致させるべき",
    );
}

/// TEST-CLI-02-AN32: actual Cli main は `lsp --stdio` didOpen→didChange sequence を schema snapshot に一致させること
#[test]
#[ignore]
fn test_e2e_selfhost_cli_main_with_lsp_stdio_document_sequence_schema_snapshot() {
    let open_body = r#"{"jsonrpc":"2.0","method":"textDocument/didOpen","params":{"uri":42,"source":"(defn main [] 0)"}}"#;
    let change_body = r#"{"jsonrpc":"2.0","method":"textDocument/didChange","params":{"uri":42,"source":"(defn main [] (+ 0 1))"}}"#;
    let stdin = format!(
        "Content-Length: {}\r\n\r\n{}Content-Length: {}\r\n\r\n{}",
        open_body.len(),
        open_body,
        change_body.len(),
        change_body
    );

    let output = compile_and_run_with_args_and_stdin(
        selfhost_cli_runtime_bundle(),
        &["lsp", "--stdio"],
        &stdin,
    );

    assert_lsp_stdio_snapshot(
        &output,
        "document-sequence.json",
        "Cli main は lsp --stdio didOpen→didChange schema を snapshot と一致させるべき",
    );
}

/// TEST-CLI-02-AN33: actual Cli main は `lsp --stdio` publishDiagnostics notification を schema snapshot に一致させること
#[test]
#[ignore]
fn test_e2e_selfhost_cli_main_with_lsp_stdio_publish_diagnostics_schema_snapshot() {
    let request_body = r#"{"jsonrpc":"2.0","method":"textDocument/publishDiagnostics","params":{"uri":42,"diagnostics":[{"source":1,"severity":1,"rule":203,"line":2,"col":4,"messageHash":7003}]}}"#;
    let stdin = format!(
        "Content-Length: {}\r\n\r\n{}",
        request_body.len(),
        request_body
    );

    let output = compile_and_run_with_args_and_stdin(
        selfhost_cli_runtime_bundle(),
        &["lsp", "--stdio"],
        &stdin,
    );

    assert_lsp_stdio_snapshot(
        &output,
        "publish-diagnostics.json",
        "Cli main は lsp --stdio publishDiagnostics notification schema を snapshot と一致させるべき",
    );
}

/// TEST-CLI-02-AN34: actual Cli main は `lsp --stdio` の didChange 後 hover fallback を schema snapshot に一致させること
#[test]
#[ignore]
fn test_e2e_selfhost_cli_main_with_lsp_stdio_hover_changed_document_schema_snapshot() {
    let open_source = "(defn alpha [x] x) (defn main [] (alpha 1))";
    let changed_source = "(defn helper [x] x) (defn main [] (helper 1))";
    let open_body = format!(
        r#"{{"jsonrpc":"2.0","method":"textDocument/didOpen","params":{{"uri":42,"source":"{}"}}}}"#,
        open_source
    );
    let change_body = format!(
        r#"{{"jsonrpc":"2.0","method":"textDocument/didChange","params":{{"uri":42,"source":"{}"}}}}"#,
        changed_source
    );
    let hover_body = r#"{"jsonrpc":"2.0","id":76,"method":"textDocument/hover","params":{"uri":42,"line":1,"col":38}}"#;
    let stdin = format!(
        "Content-Length: {}\r\n\r\n{}Content-Length: {}\r\n\r\n{}Content-Length: {}\r\n\r\n{}",
        open_body.len(),
        open_body,
        change_body.len(),
        change_body,
        hover_body.len(),
        hover_body
    );

    let output = compile_and_run_with_args_and_stdin(
        selfhost_cli_runtime_bundle(),
        &["lsp", "--stdio"],
        &stdin,
    );

    assert_lsp_stdio_snapshot(
        &output,
        "hover-changed-document.json",
        "Cli main は lsp --stdio didChange 後 hover fallback schema を snapshot と一致させるべき",
    );
}

/// TEST-CLI-02-AN34c: actual Cli main は `lsp --stdio` の didChange 後 completion fallback を schema snapshot に一致させること
#[test]
#[ignore]
fn test_e2e_selfhost_cli_main_with_lsp_stdio_completion_changed_document_schema_snapshot() {
    let open_source = "(defn alpha [] 1) (al)";
    let changed_source = "(defn helper [] 1) (he)";
    let open_body = format!(
        r#"{{"jsonrpc":"2.0","method":"textDocument/didOpen","params":{{"uri":42,"source":"{}"}}}}"#,
        open_source
    );
    let change_body = format!(
        r#"{{"jsonrpc":"2.0","method":"textDocument/didChange","params":{{"uri":42,"source":"{}"}}}}"#,
        changed_source
    );
    let completion_body = r#"{"jsonrpc":"2.0","id":74,"method":"textDocument/completion","params":{"uri":42,"line":1,"col":23}}"#;
    let stdin = format!(
        "Content-Length: {}\r\n\r\n{}Content-Length: {}\r\n\r\n{}Content-Length: {}\r\n\r\n{}",
        open_body.len(),
        open_body,
        change_body.len(),
        change_body,
        completion_body.len(),
        completion_body
    );

    let output = compile_and_run_with_args_and_stdin(
        selfhost_cli_runtime_bundle(),
        &["lsp", "--stdio"],
        &stdin,
    );

    assert_lsp_stdio_snapshot(
        &output,
        "completion-changed-document.json",
        "Cli main は lsp --stdio didChange 後 completion fallback schema を snapshot と一致させるべき",
    );
}

/// TEST-CLI-02-AN34b: actual Cli main は `lsp --stdio` の didChange 後 references fallback を schema snapshot に一致させること
#[test]
#[ignore]
fn test_e2e_selfhost_cli_main_with_lsp_stdio_references_changed_document_schema_snapshot() {
    let open_source = "(defn alpha [x] x) (defn main [] (alpha 1))";
    let changed_source = "(defn helper [x] x) (defn main [] (do (helper 1) (helper 2)))";
    let open_body = format!(
        r#"{{"jsonrpc":"2.0","method":"textDocument/didOpen","params":{{"uri":42,"source":"{}"}}}}"#,
        open_source
    );
    let change_body = format!(
        r#"{{"jsonrpc":"2.0","method":"textDocument/didChange","params":{{"uri":42,"source":"{}"}}}}"#,
        changed_source
    );
    let references_body = r#"{"jsonrpc":"2.0","id":82,"method":"textDocument/references","params":{"uri":42,"line":1,"col":40}}"#;
    let stdin = format!(
        "Content-Length: {}\r\n\r\n{}Content-Length: {}\r\n\r\n{}Content-Length: {}\r\n\r\n{}",
        open_body.len(),
        open_body,
        change_body.len(),
        change_body,
        references_body.len(),
        references_body
    );

    let output = compile_and_run_with_args_and_stdin(
        selfhost_cli_runtime_bundle(),
        &["lsp", "--stdio"],
        &stdin,
    );

    assert_lsp_stdio_snapshot(
        &output,
        "references-changed-document.json",
        "Cli main は lsp --stdio didChange 後 references fallback schema を snapshot と一致させるべき",
    );
}

/// TEST-CLI-02-AN34d: actual Cli main は `lsp --stdio` の didChange 後 definition fallback を schema snapshot に一致させること
#[test]
#[ignore]
fn test_e2e_selfhost_cli_main_with_lsp_stdio_definition_changed_document_schema_snapshot() {
    let open_source = "(defn alpha [x] x) (defn main [] (alpha 1))";
    let changed_source = "(defn helper [x] x) (defn main [] (helper 1))";
    let open_body = format!(
        r#"{{"jsonrpc":"2.0","method":"textDocument/didOpen","params":{{"uri":42,"source":"{}"}}}}"#,
        open_source
    );
    let change_body = format!(
        r#"{{"jsonrpc":"2.0","method":"textDocument/didChange","params":{{"uri":42,"source":"{}"}}}}"#,
        changed_source
    );
    let definition_body = r#"{"jsonrpc":"2.0","id":78,"method":"textDocument/definition","params":{"uri":42,"line":1,"col":38}}"#;
    let stdin = format!(
        "Content-Length: {}\r\n\r\n{}Content-Length: {}\r\n\r\n{}Content-Length: {}\r\n\r\n{}",
        open_body.len(),
        open_body,
        change_body.len(),
        change_body,
        definition_body.len(),
        definition_body
    );

    let output = compile_and_run_with_args_and_stdin(
        selfhost_cli_runtime_bundle(),
        &["lsp", "--stdio"],
        &stdin,
    );

    assert_lsp_stdio_snapshot(
        &output,
        "definition-changed-document.json",
        "Cli main は lsp --stdio didChange 後 definition fallback schema を snapshot と一致させるべき",
    );
}

/// TEST-CLI-02-AN34e: actual Cli main は `lsp --stdio` の didChange 後 rename fallback を schema snapshot に一致させること
#[test]
#[ignore]
fn test_e2e_selfhost_cli_main_with_lsp_stdio_rename_changed_document_schema_snapshot() {
    let open_source = "(defn alpha [x] x) (defn main [] (alpha 1))";
    let changed_source = "(defn helper [x] x) (defn main [] (do (helper 1) (helper 2)))";
    let open_body = format!(
        r#"{{"jsonrpc":"2.0","method":"textDocument/didOpen","params":{{"uri":42,"source":"{}"}}}}"#,
        open_source
    );
    let change_body = format!(
        r#"{{"jsonrpc":"2.0","method":"textDocument/didChange","params":{{"uri":42,"source":"{}"}}}}"#,
        changed_source
    );
    let rename_body = r#"{"jsonrpc":"2.0","id":84,"method":"textDocument/rename","params":{"uri":42,"line":1,"col":40,"newName":"cube"}}"#;
    let stdin = format!(
        "Content-Length: {}\r\n\r\n{}Content-Length: {}\r\n\r\n{}Content-Length: {}\r\n\r\n{}",
        open_body.len(),
        open_body,
        change_body.len(),
        change_body,
        rename_body.len(),
        rename_body
    );

    let output = compile_and_run_with_args_and_stdin(
        selfhost_cli_runtime_bundle(),
        &["lsp", "--stdio"],
        &stdin,
    );

    assert_lsp_stdio_snapshot(
        &output,
        "rename-changed-document.json",
        "Cli main は lsp --stdio didChange 後 rename fallback schema を snapshot と一致させるべき",
    );
}

/// TEST-CLI-02-AN35: actual Cli main は `lsp --stdio` の repeated didOpen 後 definition fallback を schema snapshot に一致させること
#[test]
#[ignore]
fn test_e2e_selfhost_cli_main_with_lsp_stdio_definition_latest_reopened_schema_snapshot() {
    let first_source = "(defn alpha [x] x) (defn main [] (alpha 1))";
    let latest_source = "(defn helper [x] x) (defn main [] (helper 1))";
    let first_open_body = format!(
        r#"{{"jsonrpc":"2.0","method":"textDocument/didOpen","params":{{"uri":42,"source":"{}"}}}}"#,
        first_source
    );
    let second_open_body = format!(
        r#"{{"jsonrpc":"2.0","method":"textDocument/didOpen","params":{{"uri":42,"source":"{}"}}}}"#,
        latest_source
    );
    let definition_body = r#"{"jsonrpc":"2.0","id":77,"method":"textDocument/definition","params":{"uri":42,"line":1,"col":38}}"#;
    let stdin = format!(
        "Content-Length: {}\r\n\r\n{}Content-Length: {}\r\n\r\n{}Content-Length: {}\r\n\r\n{}",
        first_open_body.len(),
        first_open_body,
        second_open_body.len(),
        second_open_body,
        definition_body.len(),
        definition_body
    );

    let output = compile_and_run_with_args_and_stdin(
        selfhost_cli_runtime_bundle(),
        &["lsp", "--stdio"],
        &stdin,
    );

    assert_lsp_stdio_snapshot(
        &output,
        "definition-latest-reopened.json",
        "Cli main は lsp --stdio repeated didOpen 後 definition fallback schema を snapshot と一致させるべき",
    );
}

/// TEST-CLI-02-AN35c: actual Cli main は `lsp --stdio` の repeated didOpen 後 hover fallback を schema snapshot に一致させること
#[test]
#[ignore]
fn test_e2e_selfhost_cli_main_with_lsp_stdio_hover_latest_reopened_schema_snapshot() {
    let first_source = "(defn alpha [x] x) (defn main [] (alpha 1))";
    let latest_source = "(defn helper [x] x) (defn main [] (helper 1))";
    let first_open_body = format!(
        r#"{{"jsonrpc":"2.0","method":"textDocument/didOpen","params":{{"uri":42,"source":"{}"}}}}"#,
        first_source
    );
    let second_open_body = format!(
        r#"{{"jsonrpc":"2.0","method":"textDocument/didOpen","params":{{"uri":42,"source":"{}"}}}}"#,
        latest_source
    );
    let hover_body = r#"{"jsonrpc":"2.0","id":79,"method":"textDocument/hover","params":{"uri":42,"line":1,"col":38}}"#;
    let stdin = format!(
        "Content-Length: {}\r\n\r\n{}Content-Length: {}\r\n\r\n{}Content-Length: {}\r\n\r\n{}",
        first_open_body.len(),
        first_open_body,
        second_open_body.len(),
        second_open_body,
        hover_body.len(),
        hover_body
    );

    let output = compile_and_run_with_args_and_stdin(
        selfhost_cli_runtime_bundle(),
        &["lsp", "--stdio"],
        &stdin,
    );

    assert_lsp_stdio_snapshot(
        &output,
        "hover-latest-reopened.json",
        "Cli main は lsp --stdio repeated didOpen 後 hover fallback schema を snapshot と一致させるべき",
    );
}

/// TEST-CLI-02-AN35b: actual Cli main は `lsp --stdio` の repeated didOpen 後 references fallback を schema snapshot に一致させること
#[test]
#[ignore]
fn test_e2e_selfhost_cli_main_with_lsp_stdio_references_latest_reopened_schema_snapshot() {
    let first_source = "(defn alpha [x] x) (defn main [] (alpha 1))";
    let latest_source = "(defn helper [x] x) (defn main [] (do (helper 1) (helper 2)))";
    let first_open_body = format!(
        r#"{{"jsonrpc":"2.0","method":"textDocument/didOpen","params":{{"uri":42,"source":"{}"}}}}"#,
        first_source
    );
    let second_open_body = format!(
        r#"{{"jsonrpc":"2.0","method":"textDocument/didOpen","params":{{"uri":42,"source":"{}"}}}}"#,
        latest_source
    );
    let references_body = r#"{"jsonrpc":"2.0","id":83,"method":"textDocument/references","params":{"uri":42,"line":1,"col":40}}"#;
    let stdin = format!(
        "Content-Length: {}\r\n\r\n{}Content-Length: {}\r\n\r\n{}Content-Length: {}\r\n\r\n{}",
        first_open_body.len(),
        first_open_body,
        second_open_body.len(),
        second_open_body,
        references_body.len(),
        references_body
    );

    let output = compile_and_run_with_args_and_stdin(
        selfhost_cli_runtime_bundle(),
        &["lsp", "--stdio"],
        &stdin,
    );

    assert_lsp_stdio_snapshot(
        &output,
        "references-latest-reopened.json",
        "Cli main は lsp --stdio repeated didOpen 後 references fallback schema を snapshot と一致させるべき",
    );
}

/// TEST-CLI-02-AN35d: actual Cli main は `lsp --stdio` の repeated didOpen 後 completion fallback を schema snapshot に一致させること
#[test]
#[ignore]
fn test_e2e_selfhost_cli_main_with_lsp_stdio_completion_latest_reopened_schema_snapshot() {
    let first_source = "(defn alpha [] 1) (al)";
    let latest_source = "(defn beta [] 1) (be)";
    let first_open_body = format!(
        r#"{{"jsonrpc":"2.0","method":"textDocument/didOpen","params":{{"uri":42,"source":"{}"}}}}"#,
        first_source
    );
    let second_open_body = format!(
        r#"{{"jsonrpc":"2.0","method":"textDocument/didOpen","params":{{"uri":42,"source":"{}"}}}}"#,
        latest_source
    );
    let completion_body = r#"{"jsonrpc":"2.0","id":75,"method":"textDocument/completion","params":{"uri":42,"line":1,"col":21}}"#;
    let stdin = format!(
        "Content-Length: {}\r\n\r\n{}Content-Length: {}\r\n\r\n{}Content-Length: {}\r\n\r\n{}",
        first_open_body.len(),
        first_open_body,
        second_open_body.len(),
        second_open_body,
        completion_body.len(),
        completion_body
    );

    let output = compile_and_run_with_args_and_stdin(
        selfhost_cli_runtime_bundle(),
        &["lsp", "--stdio"],
        &stdin,
    );

    assert_lsp_stdio_snapshot(
        &output,
        "completion-latest-reopened.json",
        "Cli main は lsp --stdio repeated didOpen 後 completion fallback schema を snapshot と一致させるべき",
    );
}

/// TEST-CLI-02-AN35e: actual Cli main は `lsp --stdio` の repeated didOpen 後 rename fallback を schema snapshot に一致させること
#[test]
#[ignore]
fn test_e2e_selfhost_cli_main_with_lsp_stdio_rename_latest_reopened_schema_snapshot() {
    let first_source = "(defn alpha [x] x) (defn main [] (alpha 1))";
    let latest_source = "(defn helper [x] x) (defn main [] (do (helper 1) (helper 2)))";
    let first_open_body = format!(
        r#"{{"jsonrpc":"2.0","method":"textDocument/didOpen","params":{{"uri":42,"source":"{}"}}}}"#,
        first_source
    );
    let second_open_body = format!(
        r#"{{"jsonrpc":"2.0","method":"textDocument/didOpen","params":{{"uri":42,"source":"{}"}}}}"#,
        latest_source
    );
    let rename_body = r#"{"jsonrpc":"2.0","id":85,"method":"textDocument/rename","params":{"uri":42,"line":1,"col":40,"newName":"cube"}}"#;
    let stdin = format!(
        "Content-Length: {}\r\n\r\n{}Content-Length: {}\r\n\r\n{}Content-Length: {}\r\n\r\n{}",
        first_open_body.len(),
        first_open_body,
        second_open_body.len(),
        second_open_body,
        rename_body.len(),
        rename_body
    );

    let output = compile_and_run_with_args_and_stdin(
        selfhost_cli_runtime_bundle(),
        &["lsp", "--stdio"],
        &stdin,
    );

    assert_lsp_stdio_snapshot(
        &output,
        "rename-latest-reopened.json",
        "Cli main は lsp --stdio repeated didOpen 後 rename fallback schema を snapshot と一致させるべき",
    );
}

/// TEST-CLI-02-AN36: actual Cli main は `lsp --stdio` で didChange 時に diagnostics refresh frame を返すこと
#[test]
#[ignore]
fn test_e2e_selfhost_cli_main_with_lsp_stdio_document_sequence_diagnostics_refresh_snapshot() {
    let open_body =
        r#"{"jsonrpc":"2.0","method":"textDocument/didOpen","params":{"uri":42,"source":")"}}"#;
    let change_body = r#"{"jsonrpc":"2.0","method":"textDocument/didChange","params":{"uri":42,"source":"(defn main [] 0)"}}"#;
    let stdin = format!(
        "Content-Length: {}\r\n\r\n{}Content-Length: {}\r\n\r\n{}",
        open_body.len(),
        open_body,
        change_body.len(),
        change_body
    );

    let output = compile_and_run_with_args_and_stdin(
        selfhost_cli_runtime_bundle(),
        &["lsp", "--stdio"],
        &stdin,
    );

    assert_lsp_stdio_snapshot(
        &output,
        "document-sequence-diagnostics-refresh.json",
        "Cli main は lsp --stdio didChange diagnostics refresh schema を snapshot と一致させるべき",
    );
}

/// TEST-CLI-02-AN37: actual Cli main は spec 寄り didOpen/didChange params でも diagnostics refresh snapshot に一致すること
#[test]
#[ignore]
fn test_e2e_selfhost_cli_main_with_lsp_stdio_document_sequence_spec_params_diagnostics_refresh_snapshot()
 {
    let open_body = r#"{"jsonrpc":"2.0","method":"textDocument/didOpen","params":{"textDocument":{"uri":42,"languageId":"lsharp","version":1,"text":")"}}}"#;
    let change_body = r#"{"jsonrpc":"2.0","method":"textDocument/didChange","params":{"textDocument":{"uri":42,"version":2},"contentChanges":[{"text":"(defn main [] 0)"}]}}"#;
    let stdin = format!(
        "Content-Length: {}\r\n\r\n{}Content-Length: {}\r\n\r\n{}",
        open_body.len(),
        open_body,
        change_body.len(),
        change_body
    );

    let output = compile_and_run_with_args_and_stdin(
        selfhost_cli_runtime_bundle(),
        &["lsp", "--stdio"],
        &stdin,
    );

    assert_lsp_stdio_snapshot(
        &output,
        "document-sequence-diagnostics-refresh.json",
        "Cli main は spec 寄り lsp --stdio didChange diagnostics refresh でも snapshot と一致させるべき",
    );
}

/// TEST-CLI-02-AN37a: actual Cli main は `lsp --stdio` で type diagnostics refresh frame を返すこと
#[test]
#[ignore]
fn test_e2e_selfhost_cli_main_with_lsp_stdio_document_sequence_type_diagnostics_refresh_snapshot() {
    let open_body = r#"{"jsonrpc":"2.0","method":"textDocument/didOpen","params":{"uri":42,"source":"(defn main [] (if 42 1 0))"}}"#;
    let change_body = r#"{"jsonrpc":"2.0","method":"textDocument/didChange","params":{"uri":42,"source":"(defn main [] 0)"}}"#;
    let stdin = format!(
        "Content-Length: {}\r\n\r\n{}Content-Length: {}\r\n\r\n{}",
        open_body.len(),
        open_body,
        change_body.len(),
        change_body
    );

    let output = compile_and_run_with_args_and_stdin(
        selfhost_cli_runtime_bundle(),
        &["lsp", "--stdio"],
        &stdin,
    );

    assert_lsp_stdio_snapshot(
        &output,
        "document-sequence-type-diagnostics-refresh.json",
        "Cli main は lsp --stdio didChange type diagnostics refresh schema を snapshot と一致させるべき",
    );
}

/// TEST-CLI-02-AN37b: actual Cli main は `lsp --stdio` で lint diagnostics refresh frame を返すこと
#[test]
#[ignore]
fn test_e2e_selfhost_cli_main_with_lsp_stdio_document_sequence_lint_diagnostics_refresh_snapshot() {
    let open_body = r#"{"jsonrpc":"2.0","method":"textDocument/didOpen","params":{"uri":42,"source":"(defn main [] (let [x 42] 0))"}}"#;
    let change_body = r#"{"jsonrpc":"2.0","method":"textDocument/didChange","params":{"uri":42,"source":"(defn main [] 0)"}}"#;
    let stdin = format!(
        "Content-Length: {}\r\n\r\n{}Content-Length: {}\r\n\r\n{}",
        open_body.len(),
        open_body,
        change_body.len(),
        change_body
    );

    let output = compile_and_run_with_args_and_stdin(
        selfhost_cli_runtime_bundle(),
        &["lsp", "--stdio"],
        &stdin,
    );

    assert_lsp_stdio_snapshot(
        &output,
        "document-sequence-lint-diagnostics-refresh.json",
        "Cli main は lsp --stdio didChange lint diagnostics refresh schema を snapshot と一致させるべき",
    );
}

/// TEST-CLI-02-AN37c: actual Cli main は spec 寄り didOpen/didChange params でも type diagnostics refresh snapshot に一致すること
#[test]
#[ignore]
fn test_e2e_selfhost_cli_main_with_lsp_stdio_document_sequence_spec_params_type_diagnostics_refresh_snapshot()
 {
    let open_body = r#"{"jsonrpc":"2.0","method":"textDocument/didOpen","params":{"textDocument":{"uri":42,"languageId":"lsharp","version":1,"text":"(defn main [] (if 42 1 0))"}}}"#;
    let change_body = r#"{"jsonrpc":"2.0","method":"textDocument/didChange","params":{"textDocument":{"uri":42,"version":2},"contentChanges":[{"text":"(defn main [] 0)"}]}}"#;
    let stdin = format!(
        "Content-Length: {}\r\n\r\n{}Content-Length: {}\r\n\r\n{}",
        open_body.len(),
        open_body,
        change_body.len(),
        change_body
    );

    let output = compile_and_run_with_args_and_stdin(
        selfhost_cli_runtime_bundle(),
        &["lsp", "--stdio"],
        &stdin,
    );

    assert_lsp_stdio_snapshot(
        &output,
        "document-sequence-type-diagnostics-refresh.json",
        "Cli main は spec 寄り lsp --stdio type diagnostics refresh でも snapshot と一致させるべき",
    );
}

/// TEST-CLI-02-AN37d: actual Cli main は spec 寄り didOpen/didChange params でも lint diagnostics refresh snapshot に一致すること
#[test]
#[ignore]
fn test_e2e_selfhost_cli_main_with_lsp_stdio_document_sequence_spec_params_lint_diagnostics_refresh_snapshot()
 {
    let open_body = r#"{"jsonrpc":"2.0","method":"textDocument/didOpen","params":{"textDocument":{"uri":42,"languageId":"lsharp","version":1,"text":"(defn main [] (let [x 42] 0))"}}}"#;
    let change_body = r#"{"jsonrpc":"2.0","method":"textDocument/didChange","params":{"textDocument":{"uri":42,"version":2},"contentChanges":[{"text":"(defn main [] 0)"}]}}"#;
    let stdin = format!(
        "Content-Length: {}\r\n\r\n{}Content-Length: {}\r\n\r\n{}",
        open_body.len(),
        open_body,
        change_body.len(),
        change_body
    );

    let output = compile_and_run_with_args_and_stdin(
        selfhost_cli_runtime_bundle(),
        &["lsp", "--stdio"],
        &stdin,
    );

    assert_lsp_stdio_snapshot(
        &output,
        "document-sequence-lint-diagnostics-refresh.json",
        "Cli main は spec 寄り lsp --stdio lint diagnostics refresh でも snapshot と一致させるべき",
    );
}

/// TEST-CLI-02-AN38: actual Cli main は document path 付き hover の filesystem import response を snapshot に一致させること
#[test]
#[ignore]
fn test_e2e_selfhost_cli_main_with_lsp_stdio_hover_filesystem_import_schema_snapshot() {
    let main_source = "(module Main) (import Support.Mid) (defn main [] (mid-val))";
    let hover_col = main_source.find("(mid-val)").expect("mid-val call") + 2;
    let hover_body = format!(
        r#"{{"jsonrpc":"2.0","id":191,"method":"textDocument/hover","params":{{"uri":200,"line":1,"col":{hover_col}}}}}"#
    );

    let output = run_lsp_filesystem_snapshot_request(
        "hover_filesystem_snapshot",
        200,
        "src/Main.ls",
        main_source,
        &hover_body,
    );

    assert_lsp_stdio_snapshot(
        &output,
        "hover-filesystem-import.json",
        "Cli main は document path 付き hover の filesystem import response を snapshot と一致させるべき",
    );
}

/// TEST-CLI-02-AN39: actual Cli main は document path 付き completion の filesystem import response を snapshot に一致させること
#[test]
#[ignore]
fn test_e2e_selfhost_cli_main_with_lsp_stdio_completion_filesystem_import_schema_snapshot() {
    let main_source = "(module Main) (import Support.Mid) (defn main [] (mid-va))";
    let completion_col = main_source.find("mid-va").expect("mid-va call") + "mid-va".len() + 1;
    let completion_body = format!(
        r#"{{"jsonrpc":"2.0","id":192,"method":"textDocument/completion","params":{{"uri":200,"line":1,"col":{completion_col}}}}}"#
    );

    let output = run_lsp_filesystem_snapshot_request(
        "completion_filesystem_snapshot",
        200,
        "src/Main.ls",
        main_source,
        &completion_body,
    );

    assert_lsp_stdio_snapshot(
        &output,
        "completion-filesystem-import.json",
        "Cli main は document path 付き completion の filesystem import response を snapshot と一致させるべき",
    );
}

/// TEST-CLI-02-AN40: actual Cli main は document path 付き definition の filesystem import response を snapshot に一致させること
#[test]
#[ignore]
fn test_e2e_selfhost_cli_main_with_lsp_stdio_definition_filesystem_import_schema_snapshot() {
    let main_source = "(module Main) (import Support.Mid) (defn main [] (mid-val))";
    let definition_col = main_source.find("(mid-val)").expect("mid-val call") + 2;
    let definition_body = format!(
        r#"{{"jsonrpc":"2.0","id":193,"method":"textDocument/definition","params":{{"uri":200,"line":1,"col":{definition_col}}}}}"#
    );

    let output = run_lsp_filesystem_snapshot_request(
        "definition_filesystem_snapshot",
        200,
        "src/Main.ls",
        main_source,
        &definition_body,
    );

    assert_lsp_stdio_snapshot(
        &output,
        "definition-filesystem-import.json",
        "Cli main は document path 付き definition の filesystem import response を snapshot と一致させるべき",
    );
}

/// TEST-CLI-02-AN41: actual Cli main は document path 付き references の filesystem import response を snapshot に一致させること
#[test]
#[ignore]
fn test_e2e_selfhost_cli_main_with_lsp_stdio_references_filesystem_import_schema_snapshot() {
    let main_source = "(module Main) (import Support.Mid) (defn main [] (mid-val))";
    let references_col = main_source.find("(mid-val)").expect("mid-val call") + 2;
    let references_body = format!(
        r#"{{"jsonrpc":"2.0","id":194,"method":"textDocument/references","params":{{"uri":200,"line":1,"col":{references_col}}}}}"#
    );

    let output = run_lsp_filesystem_snapshot_request(
        "references_filesystem_snapshot",
        200,
        "src/Main.ls",
        main_source,
        &references_body,
    );

    assert_lsp_stdio_snapshot(
        &output,
        "references-filesystem-import.json",
        "Cli main は document path 付き references の filesystem import response を snapshot と一致させるべき",
    );
}

/// TEST-CLI-02-AN42: actual Cli main は document path 付き rename の filesystem import response を snapshot に一致させること
#[test]
#[ignore]
fn test_e2e_selfhost_cli_main_with_lsp_stdio_rename_filesystem_import_schema_snapshot() {
    let main_source = "(module Main) (import Support.Mid) (defn main [] (mid-val))";
    let rename_col = main_source.find("(mid-val)").expect("mid-val call") + 2;
    let rename_body = format!(
        r#"{{"jsonrpc":"2.0","id":195,"method":"textDocument/rename","params":{{"uri":200,"line":1,"col":{rename_col},"newName":"mid-next"}}}}"#
    );

    let output = run_lsp_filesystem_snapshot_request(
        "rename_filesystem_snapshot",
        200,
        "src/Main.ls",
        main_source,
        &rename_body,
    );

    assert_lsp_stdio_snapshot(
        &output,
        "rename-filesystem-import.json",
        "Cli main は document path 付き rename の filesystem import response を snapshot と一致させるべき",
    );
}

/// TEST-CLI-02-AN43: actual Cli main は filesystem-backed path state を
/// 複数 request と didChange を跨いで保持し、代表 sequence snapshot に一致すること
#[test]
#[ignore]
fn test_e2e_selfhost_cli_main_with_lsp_stdio_filesystem_document_sequence_schema_snapshot() {
    let dir = cli_test_fixture_dir("filesystem_document_sequence_snapshot");
    write_cli_fixture_files(&dir, &cli_lsp_nested_fixture_files());

    let open_source = "(module Main) (import Support.Mid) (defn main [] (mid-val))";
    let changed_source = "(module Main) (import Support.Mid) (defn main [] (mid-va))";
    let symbol_col = open_source.find("(mid-val)").expect("mid-val call") + 2;
    let completion_col = changed_source.find("mid-va").expect("mid-va call") + "mid-va".len() + 1;

    let open_body = make_lsp_did_open_with_path(200, "src/Main.ls", open_source);
    let hover_body = format!(
        r#"{{"jsonrpc":"2.0","id":196,"method":"textDocument/hover","params":{{"uri":200,"line":1,"col":{symbol_col}}}}}"#
    );
    let definition_body = format!(
        r#"{{"jsonrpc":"2.0","id":197,"method":"textDocument/definition","params":{{"uri":200,"line":1,"col":{symbol_col}}}}}"#
    );
    let references_body = format!(
        r#"{{"jsonrpc":"2.0","id":198,"method":"textDocument/references","params":{{"uri":200,"line":1,"col":{symbol_col}}}}}"#
    );
    let rename_body = format!(
        r#"{{"jsonrpc":"2.0","id":199,"method":"textDocument/rename","params":{{"uri":200,"line":1,"col":{symbol_col},"newName":"mid-next"}}}}"#
    );
    let change_body = format!(
        r#"{{"jsonrpc":"2.0","method":"textDocument/didChange","params":{{"uri":200,"source":"{}"}}}}"#,
        changed_source
    );
    let completion_body = format!(
        r#"{{"jsonrpc":"2.0","id":200,"method":"textDocument/completion","params":{{"uri":200,"line":1,"col":{completion_col}}}}}"#
    );
    let stdin = format!(
        "{}{}{}{}{}{}{}",
        lsp_frame(&open_body),
        lsp_frame(&hover_body),
        lsp_frame(&definition_body),
        lsp_frame(&references_body),
        lsp_frame(&rename_body),
        lsp_frame(&change_body),
        lsp_frame(&completion_body)
    );

    let output = run_lsp_stdio_with_dir(&stdin, &dir);
    let _ = std::fs::remove_dir_all(&dir);

    assert_lsp_stdio_snapshot(
        &output,
        "filesystem-document-sequence.json",
        "Cli main は filesystem-backed long-lived document sequence を snapshot と一致させるべき",
    );
}

/// TEST-CLI-02-AN43b: actual Cli main は filesystem-backed path state を
/// spec 寄り request shape + didChange を跨いでも保持し、同じ representative snapshot に収束すること
#[test]
#[ignore]
fn test_e2e_selfhost_cli_main_with_lsp_stdio_filesystem_document_sequence_spec_style_snapshot() {
    let dir = cli_test_fixture_dir("filesystem_document_sequence_spec_style_snapshot");
    write_cli_fixture_files(&dir, &cli_lsp_nested_fixture_files());

    let open_source = "(module Main) (import Support.Mid) (defn main [] (mid-val))";
    let changed_source = "(module Main) (import Support.Mid) (defn main [] (mid-va))";
    let symbol_col = open_source.find("(mid-val)").expect("mid-val call") + 2;
    let completion_col = changed_source.find("mid-va").expect("mid-va call") + "mid-va".len();

    let open_body = serde_json::json!({
        "jsonrpc": "2.0",
        "method": "textDocument/didOpen",
        "params": {
            "textDocument": {
                "uri": 200,
                "languageId": "lsharp",
                "version": 1,
                "text": open_source
            },
            "path": "src/Main.ls"
        }
    })
    .to_string();
    let hover_body = serde_json::json!({
        "jsonrpc": "2.0",
        "id": 196,
        "method": "textDocument/hover",
        "params": {
            "textDocument": {
                "uri": 200
            },
            "position": {
                "line": 1,
                "character": symbol_col
            }
        }
    })
    .to_string();
    let definition_body = serde_json::json!({
        "jsonrpc": "2.0",
        "id": 197,
        "method": "textDocument/definition",
        "params": {
            "textDocument": {
                "uri": 200
            },
            "position": {
                "line": 1,
                "character": symbol_col
            }
        }
    })
    .to_string();
    let references_body = serde_json::json!({
        "jsonrpc": "2.0",
        "id": 198,
        "method": "textDocument/references",
        "params": {
            "textDocument": {
                "uri": 200
            },
            "position": {
                "line": 1,
                "character": symbol_col
            }
        }
    })
    .to_string();
    let rename_body = serde_json::json!({
        "jsonrpc": "2.0",
        "id": 199,
        "method": "textDocument/rename",
        "params": {
            "textDocument": {
                "uri": 200
            },
            "position": {
                "line": 1,
                "character": symbol_col
            },
            "newName": "mid-next"
        }
    })
    .to_string();
    let change_body = serde_json::json!({
        "jsonrpc": "2.0",
        "method": "textDocument/didChange",
        "params": {
            "textDocument": {
                "uri": 200,
                "version": 2
            },
            "contentChanges": [
                {
                    "text": changed_source
                }
            ]
        }
    })
    .to_string();
    let completion_body = serde_json::json!({
        "jsonrpc": "2.0",
        "id": 200,
        "method": "textDocument/completion",
        "params": {
            "textDocument": {
                "uri": 200
            },
            "position": {
                "line": 1,
                "character": completion_col
            }
        }
    })
    .to_string();
    let stdin = format!(
        "{}{}{}{}{}{}{}",
        lsp_frame(&open_body),
        lsp_frame(&hover_body),
        lsp_frame(&definition_body),
        lsp_frame(&references_body),
        lsp_frame(&rename_body),
        lsp_frame(&change_body),
        lsp_frame(&completion_body)
    );

    let output = run_lsp_stdio_with_dir(&stdin, &dir);
    let _ = std::fs::remove_dir_all(&dir);

    assert_lsp_stdio_snapshot(
        &output,
        "filesystem-document-sequence.json",
        "Cli main は filesystem-backed long-lived document sequence を spec params でも同じ snapshot に収束させるべき",
    );
}

/// TEST-CLI-02-AO: actual Cli main は help lsp に `--stdio` surface を含めること
#[test]
#[ignore]
fn test_e2e_selfhost_cli_main_with_help_lsp_stdio_option() {
    let output = compile_and_run_with_args(selfhost_cli_runtime_bundle(), &["help", "lsp"]);

    assert!(
        output.contains("lsp [--stdio] - Start LSP server"),
        "Cli main は lsp help に --stdio surface を含めるべき: {:?}",
        output
    );
}

/// TEST-LSP-01: selfhost/src/Tools/Lsp/LspServer.ls 存在 + JSON-RPC dispatch 構造
///
/// T4-2: L# 製 LSP の正式化 -- LspServer.ls が存在し JSON-RPC dispatch を持つこと
/// Red Phase: selfhost/src/Tools/Lsp/LspServer.ls が未作成のため FAIL する。
#[test]
fn test_e2e_selfhost_lsp_skeleton_v2() {
    let lsp_path = selfhost_source_path("LspServer.ls");
    assert!(
        lsp_path.exists(),
        "selfhost/src/Tools/Lsp/LspServer.ls が存在しない (T4-2: L# 製 LSP の正式化)"
    );
    let source = std::fs::read_to_string(&lsp_path)
        .expect("selfhost/src/Tools/Lsp/LspServer.ls の読み込みに失敗");

    // JSON-RPC dispatch 構造を確認
    assert!(
        source.contains("jsonrpc")
            || source.contains("json-rpc")
            || source.contains("JsonRpc")
            || source.contains("dispatch"),
        "selfhost/src/Tools/Lsp/LspServer.ls に JSON-RPC dispatch 構造がない"
    );
    // module 宣言
    assert!(
        source.contains("(module Tools.Lsp.LspServer)") || source.contains("(module Tools.Lsp"),
        "selfhost/src/Tools/Lsp/LspServer.ls に module 宣言がない"
    );
}

/// TEST-LSP-02: selfhost/src/Tools/Lsp/LspServer.ls に LSP 3.17 の 10 メソッドが定義されていること
///
/// T4-2 AC-005: initialize/shutdown/didOpen/didChange/hover/goto_definition/
///              references/rename/formatting/completion の 10 メソッド
/// Red Phase: selfhost/src/Tools/Lsp/LspServer.ls が未作成のため FAIL する。
#[test]
fn test_e2e_selfhost_lsp_10_methods() {
    let lsp_path = selfhost_source_path("LspServer.ls");
    assert!(
        lsp_path.exists(),
        "selfhost/src/Tools/Lsp/LspServer.ls が存在しない"
    );
    let source = std::fs::read_to_string(&lsp_path)
        .expect("selfhost/src/Tools/Lsp/LspServer.ls の読み込みに失敗");

    // T4-2 AC-005: 10 メソッドが LSP 3.17 仕様に準拠
    let methods = [
        "initialize",
        "shutdown",
        "didOpen",
        "didChange",
        "hover",
        "goto_definition",
        "references",
        "rename",
        "formatting",
        "completion",
    ];
    // メソッド名のバリエーション (キャメルケース / スネークケース / ハイフン区切り)
    for method in &methods {
        let snake = method.to_string();
        let kebab = snake.replace('_', "-");
        let found = source.contains(&snake) || source.contains(&kebab);
        assert!(
            found,
            "selfhost/src/Tools/Lsp/LspServer.ls に LSP メソッド '{}' の定義がない (AC-005)",
            method
        );
    }
}
