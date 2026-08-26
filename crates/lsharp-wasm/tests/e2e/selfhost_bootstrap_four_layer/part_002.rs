
fn run_wasm_with_eleven_imports_compiler_mode_inner(
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
        ElevenImportState {
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
        |mut caller: wasmtime::Caller<'_, ElevenImportState>, size: i64| -> i64 {
            let base = caller.data().next_alloc;
            let end = base
                .checked_add(size)
                .unwrap_or_else(|| panic!("eleven-import alloc: end address が overflow"));
            ensure_memory_capacity(&mut caller, end, "eleven-import alloc");
            caller.data_mut().next_alloc = end;
            base
        },
    );
    let print = wasmtime::Func::wrap(
        &mut store,
        |mut caller: wasmtime::Caller<'_, ElevenImportState>, value: i64| {
            caller.data_mut().printed.push_str(&format!("{value}\n"));
        },
    );
    let read_file = wasmtime::Func::wrap(
        &mut store,
        |mut caller: wasmtime::Caller<'_, ElevenImportState>, path: i64| -> i64 {
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
                    "eleven-import read-file: {} len={} prefix={:?}",
                    full_path.display(),
                    bytes.len(),
                    &bytes[..bytes.len().min(32)]
                );
                bytes
            } else {
                let bytes = caller.data().file_content.as_bytes().to_vec();
                eprintln!(
                    "eleven-import read-file (inline): len={} prefix={:?}",
                    bytes.len(),
                    &bytes[..bytes.len().min(32)]
                );
                bytes
            };
            alloc_cached_string_object(caller, content, "eleven-import read-file")
        },
    );
    let command_line_arg = wasmtime::Func::wrap(
        &mut store,
        |caller: wasmtime::Caller<'_, ElevenImportState>, index: i64| -> i64 {
            let content = usize::try_from(index)
                .ok()
                .and_then(|i| caller.data().args.get(i))
                .map(|a| a.as_bytes().to_vec())
                .unwrap_or_default();
            alloc_cached_string_object(caller, content, "eleven-import command-line-arg")
        },
    );
    // string-concat(ptr1, ptr2): 2つの文字列オブジェクトを結合して新しい文字列を返す
    let string_concat = wasmtime::Func::wrap(
        &mut store,
        |mut caller: wasmtime::Caller<'_, ElevenImportState>, ptr1: i64, ptr2: i64| -> i64 {
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
            alloc_cached_string_object(caller, combined, "eleven-import string-concat")
        },
    );
    // substring(ptr, start, end): 文字列オブジェクトの部分文字列を返す
    let substring = wasmtime::Func::wrap(
        &mut store,
        |mut caller: wasmtime::Caller<'_, ElevenImportState>, ptr: i64, start: i64, end: i64| -> i64 {
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
            alloc_cached_string_object(caller, slice, "eleven-import substring")
        },
    );
    let file_exists = wasmtime::Func::wrap(
        &mut store,
        |mut caller: wasmtime::Caller<'_, ElevenImportState>, path: i64| -> i64 {
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
        |mut caller: wasmtime::Caller<'_, ElevenImportState>, value: i64| -> i64 {
            let slot = i64::try_from(caller.data().root_stack.len())
                .expect("eleven-import root_push: slot overflow");
            caller.data_mut().root_stack.push(value);
            slot
        },
    );
    let root_pop = wasmtime::Func::wrap(
        &mut store,
        |mut caller: wasmtime::Caller<'_, ElevenImportState>| -> i64 {
            caller.data_mut().root_stack.pop().unwrap_or(0)
        },
    );
    let root_set = wasmtime::Func::wrap(
        &mut store,
        |mut caller: wasmtime::Caller<'_, ElevenImportState>, slot: i64, value: i64| -> i64 {
            let idx = usize::try_from(slot)
                .unwrap_or_else(|_| panic!("eleven-import root_set: slot must be non-negative"));
            let len = caller.data().root_stack.len();
            assert!(
                idx < len,
                "eleven-import root_set: slot {} out of bounds {}",
                idx,
                len
            );
            caller.data_mut().root_stack[idx] = value;
            slot
        },
    );
    let print_string = wasmtime::Func::wrap(
        &mut store,
        |mut caller: wasmtime::Caller<'_, ElevenImportState>, value: i64| {
            let bytes = read_string_object_bytes(&mut caller, value);
            let text = String::from_utf8(bytes).expect("print-string の文字列が UTF-8 ではない");
            caller.data_mut().printed.push_str(&text);
        },
    );
    let mut imports = vec![
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
    ];
    imports.push(print_string.into());
    let instance = wasmtime::Instance::new(
        &mut store,
        &module,
        &imports,
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

pub(crate) fn run_wasm_with_eleven_imports_compiler_mode(
    wasm: &[u8],
    file_content: &str,
    args: &[&str],
) -> Result<String, String> {
    run_wasm_with_eleven_imports_compiler_mode_inner(wasm, Some(file_content), None, args, false)
}

pub(crate) fn run_wasm_with_eleven_imports_compiler_mode_fs(
    wasm: &[u8],
    root_dir: &std::path::Path,
    args: &[&str],
) -> Result<String, String> {
    run_wasm_with_eleven_imports_compiler_mode_inner(wasm, None, Some(root_dir), args, false)
}

pub(crate) fn run_wasm_with_eleven_imports_compiler_mode_fs_printed_first(
    wasm: &[u8],
    root_dir: &std::path::Path,
    args: &[&str],
) -> Result<String, String> {
    run_wasm_with_eleven_imports_compiler_mode_inner(wasm, None, Some(root_dir), args, true)
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
#[ignore]
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
#[ignore]
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
#[ignore]
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
