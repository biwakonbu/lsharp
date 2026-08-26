fn write_fixed_input_set_self_feed_artifact(
    artifact_id: &str,
    report: &str,
    metadata: &serde_json::Value,
) -> std::path::PathBuf {
    let artifact_root = selfhost_project_root()
        .join("ci-artifacts/bootstrap-diff")
        .join(artifact_id);
    std::fs::create_dir_all(&artifact_root).unwrap_or_else(|e| {
        panic!(
            "CP-01 artifact ディレクトリ作成に失敗 {}: {}",
            artifact_root.display(),
            e
        )
    });

    std::fs::write(
        artifact_root.join("fixed-input-set-self-feed-report.txt"),
        report,
    )
    .unwrap_or_else(|e| panic!("CP-01 report 書き込み失敗: {e}"));
    std::fs::write(
        artifact_root.join("fixed-input-set-self-feed.json"),
        serde_json::to_vec_pretty(metadata).expect("CP-01 self-feed JSON serialize に失敗"),
    )
    .unwrap_or_else(|e| panic!("CP-01 metadata 書き込み失敗: {e}"));

    artifact_root
}

fn write_fixed_input_set_stage_chain_artifact(
    artifact_id: &str,
    report: &str,
    metadata: &serde_json::Value,
) -> std::path::PathBuf {
    let artifact_root = selfhost_project_root()
        .join("ci-artifacts/bootstrap-diff")
        .join(artifact_id);
    std::fs::create_dir_all(&artifact_root).unwrap_or_else(|e| {
        panic!(
            "BOOT-04 artifact ディレクトリ作成に失敗 {}: {}",
            artifact_root.display(),
            e
        )
    });

    std::fs::write(
        artifact_root.join("fixed-input-set-stage-chain-report.txt"),
        report,
    )
    .unwrap_or_else(|e| panic!("BOOT-04 stage-chain report 書き込み失敗: {e}"));
    std::fs::write(
        artifact_root.join("fixed-input-set-stage-chain.json"),
        serde_json::to_vec_pretty(metadata).expect("BOOT-04 stage-chain JSON serialize に失敗"),
    )
    .unwrap_or_else(|e| panic!("BOOT-04 stage-chain metadata 書き込み失敗: {e}"));

    artifact_root
}

fn write_fixed_input_set_incremental_compare_artifact(
    artifact_id: &str,
    report: &str,
    metadata: &serde_json::Value,
) -> std::path::PathBuf {
    let artifact_root = selfhost_project_root()
        .join("ci-artifacts/bootstrap-diff")
        .join(artifact_id);
    std::fs::create_dir_all(&artifact_root).unwrap_or_else(|e| {
        panic!(
            "INC-H1 artifact ディレクトリ作成に失敗 {}: {}",
            artifact_root.display(),
            e
        )
    });

    std::fs::write(
        artifact_root.join("fixed-input-set-incremental-compare-report.txt"),
        report,
    )
    .unwrap_or_else(|e| panic!("INC-H1 report 書き込み失敗: {e}"));
    std::fs::write(
        artifact_root.join("fixed-input-set-incremental-compare.json"),
        serde_json::to_vec_pretty(metadata)
            .expect("INC-H1 incremental compare JSON serialize に失敗"),
    )
    .unwrap_or_else(|e| panic!("INC-H1 metadata 書き込み失敗: {e}"));

    artifact_root
}

fn compile_fixed_input_target_with_rust_full(
    selfhost_root: &std::path::Path,
    repo_root: &std::path::Path,
    target: &FixedInputSetTarget,
) -> Result<Vec<u8>, String> {
    let root_dir = fixed_input_set_target_root(selfhost_root, repo_root, target);
    let target_path = root_dir.join(&target.path);
    let module = lsharp_ir::compile_multi_file(&target_path)
        .map_err(|e| format!("full compile failed for {}: {e}", target.path))?;
    lsharp_wasm::wasi::emit_wasm_wasi(&module)
        .map_err(|e| format!("full emit failed for {}: {e:?}", target.path))
}

fn compile_fixed_input_target_with_rust_incremental(
    selfhost_root: &std::path::Path,
    repo_root: &std::path::Path,
    target: &FixedInputSetTarget,
) -> Result<(Vec<u8>, Vec<u8>), String> {
    let root_dir = fixed_input_set_target_root(selfhost_root, repo_root, target);
    let target_path = root_dir.join(&target.path);
    let mut cache = lsharp_ir::CompilationCache::new();

    let cold_module = lsharp_ir::compile_multi_file_incremental(&target_path, &mut cache)
        .map_err(|e| format!("incremental cold compile failed for {}: {e}", target.path))?;
    let warm_module = lsharp_ir::compile_multi_file_incremental(&target_path, &mut cache)
        .map_err(|e| format!("incremental warm compile failed for {}: {e}", target.path))?;

    let cold_wasm = lsharp_wasm::wasi::emit_wasm_wasi(&cold_module)
        .map_err(|e| format!("incremental cold emit failed for {}: {e:?}", target.path))?;
    let warm_wasm = lsharp_wasm::wasi::emit_wasm_wasi(&warm_module)
        .map_err(|e| format!("incremental warm emit failed for {}: {e:?}", target.path))?;
    Ok((cold_wasm, warm_wasm))
}

#[test]
#[ignore]
fn test_e2e_bootstrap_stage2_self_feed_fixed_input_set() {
    run_bootstrap_acceptance_with_expanded_stack(|| {
        let (stage2, root) = build_stage2_self_compiler_from_main();
        let artifact_id = bootstrap_diff_artifact_id();
        let repo_root = selfhost_project_root();
        let targets = fixed_input_set_self_feed_targets();

        assert_eq!(
            targets.len(),
            54,
            "CP-01: fixed input set は selfhost/stdlib/examples の合計 54 件であるべき"
        );

        let mut compiled = Vec::new();
        let mut failures = Vec::new();
        for target in &targets {
            let root_dir = match target.root {
                FixedInputSetRoot::Selfhost => &root,
                FixedInputSetRoot::Repo => &repo_root,
            };
            let out_a = run_wasm_with_eleven_imports_compiler_mode_fs(
                &stage2,
                root_dir,
                &["compiler", target.path.as_str()],
            );
            let out_b = run_wasm_with_eleven_imports_compiler_mode_fs(
                &stage2,
                root_dir,
                &["compiler", target.path.as_str()],
            );
            match (out_a, out_b) {
                (Ok(output_a), Ok(output_b)) => {
                    if output_a != output_b {
                        failures.push(serde_json::json!({
                            "path": target.path,
                            "root": target.root.label(),
                            "error": "stage2 self-feed 出力が非決定的",
                        }));
                        continue;
                    }

                    let parsed =
                        std::panic::catch_unwind(|| parse_emitted_wasm_modules(&output_a, 1));
                    let Ok(modules_a) = parsed else {
                        failures.push(serde_json::json!({
                            "path": target.path,
                            "root": target.root.label(),
                            "error": "stage2 出力が単一 wasm モジュールとして復元できない",
                        }));
                        continue;
                    };
                    let parsed =
                        std::panic::catch_unwind(|| parse_emitted_wasm_modules(&output_b, 1));
                    let Ok(modules_b) = parsed else {
                        failures.push(serde_json::json!({
                            "path": target.path,
                            "root": target.root.label(),
                            "error": "stage2 2回目出力が単一 wasm モジュールとして復元できない",
                        }));
                        continue;
                    };
                    let wasm_a = &modules_a[0];
                    let wasm_b = &modules_b[0];
                    if std::panic::catch_unwind(|| assert_valid_wasm(wasm_a)).is_err() {
                        failures.push(serde_json::json!({
                            "path": target.path,
                            "root": target.root.label(),
                            "error": "stage2 出力 wasm の検証に失敗",
                        }));
                        continue;
                    }
                    if std::panic::catch_unwind(|| assert_valid_wasm(wasm_b)).is_err() {
                        failures.push(serde_json::json!({
                            "path": target.path,
                            "root": target.root.label(),
                            "error": "stage2 2回目出力 wasm の検証に失敗",
                        }));
                        continue;
                    }
                    if wasm_a != wasm_b {
                        failures.push(serde_json::json!({
                            "path": target.path,
                            "root": target.root.label(),
                            "error": "stage2 self-feed wasm が byte-identical でない",
                        }));
                        continue;
                    }
                    compiled.push(serde_json::json!({
                    "path": target.path,
                    "root": target.root.label(),
                    "output_wasm_bytes": wasm_a.len(),
                    "fingerprint": super::selfhost_bootstrap_four_layer::hash_fingerprint(wasm_a),
                }));
                }
                (Err(err), _) | (_, Err(err)) => failures.push(serde_json::json!({
                    "path": target.path,
                    "root": target.root.label(),
                    "error": err,
                })),
            }
        }

        let mut report_lines = vec![
            "Bootstrap Fixed Input Set Self-Feed Report".to_string(),
            "==========================================".to_string(),
            format!("commit: {artifact_id}"),
            "timestamp: 1970-01-01T00:00:00Z".to_string(),
            "test: test_e2e_bootstrap_stage2_self_feed_fixed_input_set".to_string(),
            format!("stage2_self_compiler_bytes: {}", stage2.len()),
            format!("target_count: {}", targets.len()),
            format!("compiled_count: {}", compiled.len()),
            format!("failed_count: {}", failures.len()),
            String::new(),
        ];
        report_lines.extend(compiled.iter().map(|entry| {
            format!(
                "PASS [{}] {} -> {} bytes",
                entry["root"].as_str().unwrap_or("unknown"),
                entry["path"].as_str().unwrap_or("<missing>"),
                entry["output_wasm_bytes"].as_u64().unwrap_or(0)
            )
        }));
        report_lines.extend(failures.iter().map(|entry| {
            format!(
                "FAIL [{}] {} -> {}",
                entry["root"].as_str().unwrap_or("unknown"),
                entry["path"].as_str().unwrap_or("<missing>"),
                entry["error"]
                    .as_str()
                    .unwrap_or("unknown error")
                    .lines()
                    .next()
                    .unwrap_or("unknown error")
            )
        }));
        let report = report_lines.join("\n");

        let metadata = serde_json::json!({
            "commit_sha": artifact_id,
            "timestamp": "1970-01-01T00:00:00Z",
            "test_name": "test_e2e_bootstrap_stage2_self_feed_fixed_input_set",
            "stage2_self_compiler_bytes": stage2.len(),
            "target_count": targets.len(),
            "compiled_count": compiled.len(),
            "failed_count": failures.len(),
            "compiled_targets": compiled,
            "failed_targets": failures,
        });
        let artifact_dir =
            write_fixed_input_set_self_feed_artifact(&artifact_id, &report, &metadata);

        let written_metadata: serde_json::Value = serde_json::from_str(
            &std::fs::read_to_string(artifact_dir.join("fixed-input-set-self-feed.json"))
                .expect("CP-01 self-feed artifact JSON の読み込みに失敗"),
        )
        .expect("CP-01 self-feed artifact JSON は JSON であること");
        assert_eq!(
            written_metadata["compiled_count"].as_u64(),
            Some(compiled.len() as u64),
            "CP-01 self-feed artifact は compiled_count を保持すること"
        );
        assert_eq!(
            written_metadata["failed_count"].as_u64(),
            Some(failures.len() as u64),
            "CP-01 self-feed artifact は failed_count を保持すること"
        );

        assert!(
            failures.is_empty(),
            "CP-01: stage2 self-feed fixed input set に失敗がある: {}",
            serde_json::to_string_pretty(&written_metadata["failed_targets"])
                .expect("CP-01 failure JSON serialize に失敗")
        );
        assert_eq!(
            compiled.len(),
            targets.len(),
            "CP-01: stage2 self compiler は fixed input set 全件を再生成できるべき"
        );
    });
}

#[test]
#[ignore]
fn test_e2e_bootstrap_fixed_input_set_stage_chain_match_cli_module() {
    run_bootstrap_acceptance_with_expanded_stack(|| {
        let (stage1, stage2, selfhost_root) = build_stage1_and_stage2_self_compilers_from_main();
        let repo_root = selfhost_project_root();
        let target = fixed_input_set_target_by_path("src/App/Cli.ls");
        let stage2_target =
            compile_fixed_input_target_with_stage1(&stage1, &selfhost_root, &repo_root, &target)
                .expect("BOOT-04: stage1 compiler が App/Cli.ls の self-feed compile に失敗");
        let stage3_target =
            compile_fixed_input_target_with_stage2(&stage2, &selfhost_root, &repo_root, &target)
                .expect("BOOT-04: stage2 compiler が App/Cli.ls の self-feed compile に失敗");
        assert_eq!(
        stage2_target,
        stage3_target,
        "BOOT-04: App/Cli.ls の stage chain mismatch: {}",
        serde_json::to_string_pretty(&serde_json::json!({
            "path": target.path,
            "stage2_output_wasm_bytes": stage2_target.len(),
            "stage3_output_wasm_bytes": stage3_target.len(),
            "stage2_fingerprint": super::selfhost_bootstrap_four_layer::hash_fingerprint(&stage2_target),
            "stage3_fingerprint": super::selfhost_bootstrap_four_layer::hash_fingerprint(&stage3_target),
            "export_match": extract_section_bytes(&stage2_target, 7) == extract_section_bytes(&stage3_target, 7),
            "data_match": extract_section_bytes(&stage2_target, 11) == extract_section_bytes(&stage3_target, 11),
            "first_diff": first_diff_index(&stage2_target, &stage3_target),
        }))
        .expect("BOOT-04: App/Cli.ls mismatch JSON serialize に失敗")
    );
    });
}

#[test]
#[ignore]
fn test_e2e_bootstrap_fixed_input_set_stage_chain_match_lsp_server_module() {
    run_bootstrap_acceptance_with_expanded_stack(|| {
        let (stage1, stage2, selfhost_root) = build_stage1_and_stage2_self_compilers_from_main();
        let repo_root = selfhost_project_root();
        let target = fixed_input_set_target_by_path("src/Tools/Lsp/LspServer.ls");
        let stage2_target = compile_fixed_input_target_with_stage1(
            &stage1,
            &selfhost_root,
            &repo_root,
            &target,
        )
        .expect("BOOT-04: stage1 compiler が Tools/Lsp/LspServer.ls の self-feed compile に失敗");
        let stage3_target = compile_fixed_input_target_with_stage2(
            &stage2,
            &selfhost_root,
            &repo_root,
            &target,
        )
        .expect("BOOT-04: stage2 compiler が Tools/Lsp/LspServer.ls の self-feed compile に失敗");
        assert_eq!(
        stage2_target,
        stage3_target,
        "BOOT-04: Tools/Lsp/LspServer.ls の stage chain mismatch: {}",
        serde_json::to_string_pretty(&serde_json::json!({
            "path": target.path,
            "stage2_output_wasm_bytes": stage2_target.len(),
            "stage3_output_wasm_bytes": stage3_target.len(),
            "stage2_fingerprint": super::selfhost_bootstrap_four_layer::hash_fingerprint(&stage2_target),
            "stage3_fingerprint": super::selfhost_bootstrap_four_layer::hash_fingerprint(&stage3_target),
            "export_match": extract_section_bytes(&stage2_target, 7) == extract_section_bytes(&stage3_target, 7),
            "data_match": extract_section_bytes(&stage2_target, 11) == extract_section_bytes(&stage3_target, 11),
            "first_diff": first_diff_index(&stage2_target, &stage3_target),
        }))
        .expect("BOOT-04: Tools/Lsp/LspServer.ls mismatch JSON serialize に失敗")
    );
    });
}

#[test]
#[ignore]
fn test_e2e_bootstrap_fixed_input_set_stage_chain_match() {
    run_bootstrap_acceptance_with_expanded_stack(|| {
        let (stage1, stage2, selfhost_root) = build_stage1_and_stage2_self_compilers_from_main();
        let repo_root = selfhost_project_root();
        let artifact_id = bootstrap_diff_artifact_id();
        let targets = fixed_input_set_self_feed_targets();

        assert_eq!(
            targets.len(),
            54,
            "BOOT-04: fixed input set は selfhost/stdlib/examples の合計 54 件であるべき"
        );

        let mut matched = Vec::new();
        let mut failures = Vec::new();
        for target in &targets {
            match (
            compile_fixed_input_target_with_stage1(&stage1, &selfhost_root, &repo_root, target),
            compile_fixed_input_target_with_stage2(&stage2, &selfhost_root, &repo_root, target),
        ) {
            (Ok(stage2_target), Ok(stage3_target)) => {
                let export_a = extract_section_bytes(&stage2_target, 7);
                let export_b = extract_section_bytes(&stage3_target, 7);
                let data_a = extract_section_bytes(&stage2_target, 11);
                let data_b = extract_section_bytes(&stage3_target, 11);
                let first_diff = first_diff_index(&stage2_target, &stage3_target);
                if stage2_target != stage3_target {
                    failures.push(serde_json::json!({
                        "path": target.path,
                        "root": target.root.label(),
                        "error": "stage1->stage2 と stage2->stage3 の出力 wasm が一致しない",
                        "stage2_output_wasm_bytes": stage2_target.len(),
                        "stage3_output_wasm_bytes": stage3_target.len(),
                        "stage2_fingerprint": super::selfhost_bootstrap_four_layer::hash_fingerprint(&stage2_target),
                        "stage3_fingerprint": super::selfhost_bootstrap_four_layer::hash_fingerprint(&stage3_target),
                        "export_match": export_a == export_b,
                        "data_match": data_a == data_b,
                        "first_diff": first_diff,
                    }));
                    continue;
                }
                matched.push(serde_json::json!({
                    "path": target.path,
                    "root": target.root.label(),
                    "output_wasm_bytes": stage2_target.len(),
                    "fingerprint": super::selfhost_bootstrap_four_layer::hash_fingerprint(&stage2_target),
                }));
            }
            (Err(stage1_err), Ok(_)) => failures.push(serde_json::json!({
                "path": target.path,
                "root": target.root.label(),
                "error": stage1_err,
            })),
            (Ok(_), Err(stage2_err)) => failures.push(serde_json::json!({
                "path": target.path,
                "root": target.root.label(),
                "error": stage2_err,
            })),
            (Err(stage1_err), Err(stage2_err)) => failures.push(serde_json::json!({
                "path": target.path,
                "root": target.root.label(),
                "error": format!("stage1 compiler: {stage1_err}; stage2 compiler: {stage2_err}"),
            })),
        }
        }

        let mut report_lines = vec![
            "Bootstrap Fixed Input Set Stage Chain Report".to_string(),
            "===========================================".to_string(),
            format!("commit: {artifact_id}"),
            "timestamp: 1970-01-01T00:00:00Z".to_string(),
            "test: test_e2e_bootstrap_fixed_input_set_stage_chain_match".to_string(),
            format!("stage1_self_compiler_bytes: {}", stage1.len()),
            format!("stage2_self_compiler_bytes: {}", stage2.len()),
            format!("target_count: {}", targets.len()),
            format!("matched_count: {}", matched.len()),
            format!("failed_count: {}", failures.len()),
            String::new(),
        ];
        report_lines.extend(matched.iter().map(|entry| {
            format!(
                "MATCH [{}] {} -> {} bytes",
                entry["root"].as_str().unwrap_or("unknown"),
                entry["path"].as_str().unwrap_or("<missing>"),
                entry["output_wasm_bytes"].as_u64().unwrap_or(0)
            )
        }));
        report_lines.extend(failures.iter().map(|entry| {
            format!(
                "FAIL [{}] {} -> {}",
                entry["root"].as_str().unwrap_or("unknown"),
                entry["path"].as_str().unwrap_or("<missing>"),
                entry["error"]
                    .as_str()
                    .unwrap_or("unknown error")
                    .lines()
                    .next()
                    .unwrap_or("unknown error")
            )
        }));
        let report = report_lines.join("\n");

        let metadata = serde_json::json!({
            "commit_sha": artifact_id,
            "timestamp": "1970-01-01T00:00:00Z",
            "test_name": "test_e2e_bootstrap_fixed_input_set_stage_chain_match",
            "stage1_self_compiler_bytes": stage1.len(),
            "stage2_self_compiler_bytes": stage2.len(),
            "target_count": targets.len(),
            "matched_count": matched.len(),
            "failed_count": failures.len(),
            "matched_targets": matched,
            "failed_targets": failures,
        });
        let artifact_dir =
            write_fixed_input_set_stage_chain_artifact(&artifact_id, &report, &metadata);

        let written_metadata: serde_json::Value = serde_json::from_str(
            &std::fs::read_to_string(artifact_dir.join("fixed-input-set-stage-chain.json"))
                .expect("BOOT-04 stage-chain artifact JSON の読み込みに失敗"),
        )
        .expect("BOOT-04 stage-chain artifact JSON は JSON であること");
        assert_eq!(
            written_metadata["matched_count"].as_u64(),
            Some(matched.len() as u64),
            "BOOT-04 stage-chain artifact は matched_count を保持すること"
        );
        assert_eq!(
            written_metadata["failed_count"].as_u64(),
            Some(failures.len() as u64),
            "BOOT-04 stage-chain artifact は failed_count を保持すること"
        );
        assert!(
            failures.is_empty(),
            "BOOT-04: full fixed input set stage chain compare に失敗がある: {}",
            serde_json::to_string_pretty(&written_metadata["failed_targets"])
                .expect("BOOT-04 failure JSON serialize に失敗")
        );
        assert_eq!(
            matched.len(),
            targets.len(),
            "BOOT-04: full fixed input set の stage2/stage3 compare は全件一致するべき"
        );
    });
}

#[test]
#[ignore]
fn test_e2e_incremental_compile_matches_full_compile_fixed_input_set() {
    run_bootstrap_acceptance_with_expanded_stack(|| {
        let artifact_id = bootstrap_diff_artifact_id();
        let selfhost_root = selfhost_package_root();
        let repo_root = selfhost_project_root();
        let targets = fixed_input_set_self_feed_targets();

        assert_eq!(
            targets.len(),
            54,
            "INC-H1: fixed input set は selfhost/stdlib/examples の合計 54 件であるべき"
        );

        let mut matched = Vec::new();
        let mut failures = Vec::new();
        for target in &targets {
            match (
            compile_fixed_input_target_with_rust_full(&selfhost_root, &repo_root, target),
            compile_fixed_input_target_with_rust_incremental(&selfhost_root, &repo_root, target),
        ) {
            (Ok(full_wasm), Ok((incremental_cold, incremental_warm))) => {
                if std::panic::catch_unwind(|| assert_valid_wasm(&full_wasm)).is_err() {
                    failures.push(serde_json::json!({
                        "path": target.path,
                        "root": target.root.label(),
                        "error": "full compile output wasm の検証に失敗",
                    }));
                    continue;
                }
                if std::panic::catch_unwind(|| assert_valid_wasm(&incremental_cold)).is_err() {
                    failures.push(serde_json::json!({
                        "path": target.path,
                        "root": target.root.label(),
                        "error": "incremental cold output wasm の検証に失敗",
                    }));
                    continue;
                }
                if std::panic::catch_unwind(|| assert_valid_wasm(&incremental_warm)).is_err() {
                    failures.push(serde_json::json!({
                        "path": target.path,
                        "root": target.root.label(),
                        "error": "incremental warm output wasm の検証に失敗",
                    }));
                    continue;
                }
                if full_wasm != incremental_cold || full_wasm != incremental_warm {
                    failures.push(serde_json::json!({
                        "path": target.path,
                        "root": target.root.label(),
                        "error": "full / incremental cold / incremental warm の wasm が byte-identical でない",
                        "full_wasm_bytes": full_wasm.len(),
                        "incremental_cold_wasm_bytes": incremental_cold.len(),
                        "incremental_warm_wasm_bytes": incremental_warm.len(),
                        "full_fingerprint": super::selfhost_bootstrap_four_layer::hash_fingerprint(&full_wasm),
                        "incremental_cold_fingerprint": super::selfhost_bootstrap_four_layer::hash_fingerprint(&incremental_cold),
                        "incremental_warm_fingerprint": super::selfhost_bootstrap_four_layer::hash_fingerprint(&incremental_warm),
                        "full_vs_incremental_cold_first_diff": first_diff_index(&full_wasm, &incremental_cold),
                        "cold_vs_warm_first_diff": first_diff_index(&incremental_cold, &incremental_warm),
                    }));
                    continue;
                }
                matched.push(serde_json::json!({
                    "path": target.path,
                    "root": target.root.label(),
                    "output_wasm_bytes": full_wasm.len(),
                    "fingerprint": super::selfhost_bootstrap_four_layer::hash_fingerprint(&full_wasm),
                }));
            }
            (Err(full_err), Ok(_)) => failures.push(serde_json::json!({
                "path": target.path,
                "root": target.root.label(),
                "error": full_err,
            })),
            (Ok(_), Err(incremental_err)) => failures.push(serde_json::json!({
                "path": target.path,
                "root": target.root.label(),
                "error": incremental_err,
            })),
            (Err(full_err), Err(incremental_err)) => failures.push(serde_json::json!({
                "path": target.path,
                "root": target.root.label(),
                "error": format!("full compile: {full_err}; incremental compile: {incremental_err}"),
            })),
        }
        }

        let mut report_lines = vec![
            "Incremental Fixed Input Set Compare Report".to_string(),
            "=========================================".to_string(),
            format!("commit: {artifact_id}"),
            "timestamp: 1970-01-01T00:00:00Z".to_string(),
            "test: test_e2e_incremental_compile_matches_full_compile_fixed_input_set".to_string(),
            format!("target_count: {}", targets.len()),
            format!("matched_count: {}", matched.len()),
            format!("failed_count: {}", failures.len()),
            String::new(),
        ];
        report_lines.extend(matched.iter().map(|entry| {
            format!(
                "MATCH [{}] {} -> {} bytes",
                entry["root"].as_str().unwrap_or("unknown"),
                entry["path"].as_str().unwrap_or("<missing>"),
                entry["output_wasm_bytes"].as_u64().unwrap_or(0)
            )
        }));
        report_lines.extend(failures.iter().map(|entry| {
            format!(
                "FAIL [{}] {} -> {}",
                entry["root"].as_str().unwrap_or("unknown"),
                entry["path"].as_str().unwrap_or("<missing>"),
                entry["error"]
                    .as_str()
                    .unwrap_or("unknown error")
                    .lines()
                    .next()
                    .unwrap_or("unknown error")
            )
        }));
        let report = report_lines.join("\n");

        let metadata = serde_json::json!({
            "commit_sha": artifact_id,
            "timestamp": "1970-01-01T00:00:00Z",
            "test_name": "test_e2e_incremental_compile_matches_full_compile_fixed_input_set",
            "target_count": targets.len(),
            "matched_count": matched.len(),
            "failed_count": failures.len(),
            "matched_targets": matched,
            "failed_targets": failures,
        });
        let artifact_dir =
            write_fixed_input_set_incremental_compare_artifact(&artifact_id, &report, &metadata);

        let written_metadata: serde_json::Value = serde_json::from_str(
            &std::fs::read_to_string(artifact_dir.join("fixed-input-set-incremental-compare.json"))
                .expect("INC-H1 artifact JSON の読み込みに失敗"),
        )
        .expect("INC-H1 artifact JSON は JSON であること");
        assert_eq!(
            written_metadata["matched_count"].as_u64(),
            Some(matched.len() as u64),
            "INC-H1 artifact は matched_count を保持すること"
        );
        assert_eq!(
            written_metadata["failed_count"].as_u64(),
            Some(failures.len() as u64),
            "INC-H1 artifact は failed_count を保持すること"
        );
        assert!(
            failures.is_empty(),
            "INC-H1: fixed input set の full vs incremental compare に失敗がある: {}",
            serde_json::to_string_pretty(&written_metadata["failed_targets"])
                .expect("INC-H1 failure JSON serialize に失敗")
        );
        assert_eq!(
            matched.len(),
            targets.len(),
            "INC-H1: fixed input set の full / incremental compare は全件一致するべき"
        );
    });
}

// =============================================================================
// Test 3: test_e2e_bootstrap_stage1_section_stability
// =============================================================================
