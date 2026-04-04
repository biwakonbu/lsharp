//! メタデータ検証テスト: ドキュメント構造の正当性を検証
//!
//! TEST-META-01: compatibility matrix 8列拡張の検証
//! TEST-META-04: gap backlog classification の5分類検証

/// ../../../docs/development/planning/compatibility-matrix.md のヘッダーが 8 列であること、
/// 必須列名が存在することを検証する。
///
/// 8列: Feature, Rust source, L# source, Parity test, Default path, Deletion gate, + 2列拡張
///
/// 現状は 6 列なので、8 列拡張が完了するまで FAIL する (Red Phase)。
#[test]
fn test_meta_01_compatibility_matrix_8_columns() {
    let content = std::fs::read_to_string(
        std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../../docs/development/planning/compatibility-matrix.md"),
    )
    .expect("docs/development/planning/compatibility-matrix.md が読み込めない");

    // ヘッダー行を探す (最初の Markdown テーブルヘッダー)
    let header_line = content
        .lines()
        .find(|line| line.starts_with('|') && line.contains("Rust source"))
        .expect("テーブルヘッダー行 (Rust source を含む) が見つからない");

    // パイプで分割して列数をカウント (先頭・末尾の空要素を除く)
    let columns: Vec<&str> = header_line
        .split('|')
        .map(|s| s.trim())
        .filter(|s| !s.is_empty())
        .collect();

    // 8 列であること
    assert_eq!(
        columns.len(),
        8,
        "互換マトリクスのヘッダーは 8 列であるべき (現在 {} 列): {:?}",
        columns.len(),
        columns
    );

    // 必須列名の存在チェック
    let required_columns = [
        "Feature",
        "Rust source",
        "L# source",
        "Parity test",
        "Default path",
        "Deletion gate",
    ];

    for required in &required_columns {
        assert!(
            columns.iter().any(|c| c.contains(required)),
            "必須列 '{}' がヘッダーに存在しない: {:?}",
            required,
            columns
        );
    }
}

/// ../../../docs/development/planning/gap-classification.md に 5 分類のセクション/ラベルが
/// 全て存在することを検証する。
///
/// 5 分類: spec-diff, impl-missing, output-diff, perf-diff, ops-diff
///
/// 現状はセクション見出しが日本語名 (仕様差分, 実装欠落, ...) なので、
/// 英語ラベル (spec-diff 等) が追加されるまで FAIL する (Red Phase)。
#[test]
fn test_meta_04_gap_backlog_5_categories() {
    let content = std::fs::read_to_string(
        std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../../docs/development/planning/gap-classification.md"),
    )
    .expect("docs/development/planning/gap-classification.md が読み込めない");

    // 5 分類の英語ラベルが全て存在すること
    let categories = [
        "spec-diff",
        "impl-missing",
        "output-diff",
        "perf-diff",
        "ops-diff",
    ];

    let mut missing: Vec<&str> = Vec::new();
    for cat in &categories {
        if !content.contains(cat) {
            missing.push(cat);
        }
    }

    assert!(
        missing.is_empty(),
        "../../../docs/development/planning/gap-classification.md に以下の分類ラベルが不足: {:?} (全 5 分類が必要)",
        missing
    );

    // 各分類がセクション (### または ## レベル) として定義されていること
    for cat in &categories {
        let section_pattern = format!("# {}", cat);
        let has_section = content.lines().any(|line| {
            let trimmed = line.trim();
            trimmed.contains(cat) && (trimmed.starts_with('#') || trimmed.starts_with("- "))
        });
        assert!(
            has_section || content.contains(&section_pattern),
            "分類 '{}' がセクション見出しまたはリスト項目として定義されていない",
            cat
        );
    }
}

/// TEST-META-03: CI ワークフローの audit-docs ゲートジョブ検証
///
/// 以下を検証:
/// 1. `.github/workflows/ci.yml` に `audit_docs` (または `audit-docs`) を実行するジョブが存在する
/// 2. そのジョブが `ci-gate` の `needs` に含まれている (required check)
/// 3. `scripts/audit_docs.sh` が存在し実行可能である
#[test]
fn test_meta_03_audit_docs_ci_gate() {
    let project_root = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .canonicalize()
        .expect("プロジェクトルートが見つからない");

    // 1. CI ワークフローファイルの読み込み
    let ci_yml_path = project_root.join(".github/workflows/ci.yml");
    assert!(
        ci_yml_path.exists(),
        ".github/workflows/ci.yml が存在しない"
    );
    let ci_content = std::fs::read_to_string(&ci_yml_path).expect("ci.yml の読み込みに失敗");

    // 2. audit-docs ジョブが存在すること
    let has_audit_docs_job =
        ci_content.contains("audit-docs:") || ci_content.contains("audit_docs:");
    assert!(
        has_audit_docs_job,
        "ci.yml に audit-docs (または audit_docs) ジョブが定義されていない"
    );

    // 3. audit-docs ジョブ内で audit_docs.sh を実行していること
    let has_audit_script_run = ci_content.contains("audit_docs.sh");
    assert!(
        has_audit_script_run,
        "ci.yml の audit-docs ジョブが scripts/audit_docs.sh を実行していない"
    );

    // 4. ci-gate の needs に audit-docs が含まれていること
    let gate_needs_audit = ci_content
        .lines()
        .any(|line| line.contains("needs:") && line.contains("audit-docs"));
    assert!(
        gate_needs_audit,
        "ci-gate ジョブの needs に audit-docs が含まれていない。\n\
         audit-docs は CI の required check として ci-gate に統合される必要がある"
    );

    // 5. scripts/audit_docs.sh が存在すること
    let audit_script_path = project_root.join("scripts/audit_docs.sh");
    assert!(
        audit_script_path.exists(),
        "scripts/audit_docs.sh が存在しない"
    );

    // 6. scripts/audit_docs.sh が実行可能であること
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let metadata =
            std::fs::metadata(&audit_script_path).expect("audit_docs.sh のメタデータ取得に失敗");
        let permissions = metadata.permissions();
        let is_executable = permissions.mode() & 0o111 != 0;
        assert!(
            is_executable,
            "scripts/audit_docs.sh に実行権限が付与されていない (mode: {:o})",
            permissions.mode()
        );
    }
}

/// TEST-META-05: branch protection の required check 契約が docs / workflow / audit で同期している
///
/// 以下を検証:
/// 1. `.github/workflows/ci.yml` に `ci-gate-v2` job と `CI Gate v2` 表示名が存在する
/// 2. `docs/development/operations/CI.md` が job id と Actions 表示名の両方を案内している
/// 3. `docs/development/operations/branch-protection-checklist.md` が required check 名を具体的に記載している
/// 4. `scripts/audit_docs.sh` が branch protection 正本の整合性を機械検証している
#[test]
fn test_meta_05_branch_protection_required_check_contract() {
    let project_root = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .canonicalize()
        .expect("プロジェクトルートが見つからない");

    let ci_yml_path = project_root.join(".github/workflows/ci.yml");
    let ci_content = std::fs::read_to_string(&ci_yml_path).expect("ci.yml の読み込みに失敗");
    assert!(
        ci_content.contains("ci-gate-v2:"),
        "ci.yml に ci-gate-v2 job id が存在しない"
    );
    assert!(
        ci_content.contains("name: CI Gate v2"),
        "ci.yml に CI Gate v2 表示名が存在しない"
    );

    let ci_doc_path = project_root.join("docs/development/operations/CI.md");
    let ci_doc_content = std::fs::read_to_string(&ci_doc_path).expect("CI.md の読み込みに失敗");
    assert!(
        ci_doc_content.contains("ci-gate-v2"),
        "CI.md は required check の job id (`ci-gate-v2`) を説明すること"
    );
    assert!(
        ci_doc_content.contains("CI Gate v2"),
        "CI.md は required check の Actions 表示名 (`CI Gate v2`) を説明すること"
    );

    let checklist_path =
        project_root.join("docs/development/operations/branch-protection-checklist.md");
    let checklist_content = std::fs::read_to_string(&checklist_path)
        .expect("branch-protection-checklist.md の読み込みに失敗");
    assert!(
        checklist_content.contains("ci-gate-v2"),
        "branch-protection-checklist.md は required check の job id (`ci-gate-v2`) を含むこと"
    );
    assert!(
        checklist_content.contains("CI Gate v2"),
        "branch-protection-checklist.md は required check の Actions 表示名 (`CI Gate v2`) を含むこと"
    );

    let audit_script_path = project_root.join("scripts/audit_docs.sh");
    let audit_script_content = std::fs::read_to_string(&audit_script_path)
        .expect("scripts/audit_docs.sh の読み込みに失敗");
    assert!(
        audit_script_content.contains("branch-protection-checklist.md"),
        "scripts/audit_docs.sh は branch-protection-checklist.md の整合性を確認すること"
    );
    assert!(
        audit_script_content.contains("ci-gate-v2"),
        "scripts/audit_docs.sh は required check の job id (`ci-gate-v2`) を検証すること"
    );
    assert!(
        audit_script_content.contains("CI Gate v2"),
        "scripts/audit_docs.sh は required check の Actions 表示名 (`CI Gate v2`) を検証すること"
    );
}

/// TEST-META-06: Deferred / v2 native 項目の正本同期
///
/// 以下を検証:
/// 1. `TODO.md` の「現在の残タスク一覧（正本）」に Deferred / v2 項目を混ぜない
/// 2. `TODO.md` の Deferred / v2 節が V2-08 / V2-09 / V2-10 を保持する
/// 3. `TODO.md` の Deferred / v2 注記が V2-01〜V2-10 と `v2-designs/` を参照している
/// 4. `phase11-implementation-plan.md` に V2-08 / V2-09 / V2-10 節が存在する
/// 5. V2-08 / V2-09 / V2-10 の設計 docs が存在し、Deferred 方針を説明している
#[test]
fn test_meta_06_deferred_v2_native_docs_are_synced() {
    let project_root = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .canonicalize()
        .expect("プロジェクトルートが見つからない");

    let todo_path = project_root.join("TODO.md");
    let todo_content = std::fs::read_to_string(&todo_path).expect("TODO.md の読み込みに失敗");
    let current_remaining_section = todo_content
        .split("## 現在の残タスク一覧（正本）")
        .nth(1)
        .and_then(|rest| rest.split("## Phase 11: Rust 完全撤去").next())
        .expect("TODO.md の現在の残タスク一覧 section が見つからない");
    let deferred_section = todo_content
        .split("### Deferred / v2")
        .nth(1)
        .expect("TODO.md の Deferred / v2 section が見つからない");
    assert!(
        !current_remaining_section
            .contains("- [~] `V2-08` Native backend self-regeneration（Deferred）"),
        "TODO.md の残タスク一覧に Deferred の V2-08 を混ぜないこと"
    );
    assert!(
        !current_remaining_section
            .contains("- [~] `V2-09` Wasm/native differential zero（Deferred）"),
        "TODO.md の残タスク一覧に Deferred の V2-09 を混ぜないこと"
    );
    assert!(
        !current_remaining_section
            .contains("- [ ] `V2-10` Native-only RC distribution（Deferred）"),
        "TODO.md の残タスク一覧に Deferred の V2-10 を混ぜないこと"
    );
    assert!(
        todo_content.contains("V2-01〜V2-10")
            && todo_content.contains("docs/development/planning/v2-designs/"),
        "TODO.md の Deferred / v2 注記は plan 節と v2-designs 配下の両方を参照すること"
    );
    assert!(
        deferred_section.contains("- [DEFERRED] V2-08 Native backend self-regeneration"),
        "TODO.md の Deferred / v2 節で V2-08 の詳細が存在すること"
    );
    assert!(
        deferred_section.contains("- [DEFERRED] V2-09 Wasm/native differential zero"),
        "TODO.md の Deferred / v2 節で V2-09 の詳細が存在すること"
    );
    assert!(
        deferred_section.contains("- [DEFERRED] V2-10 Native-only RC distribution"),
        "TODO.md では V2-10 は Deferred 項目として [DEFERRED] を維持すること"
    );

    let plan_path = project_root.join("docs/development/planning/phase11-implementation-plan.md");
    let plan_content = std::fs::read_to_string(&plan_path)
        .expect("phase11-implementation-plan.md の読み込みに失敗");
    for anchor in [
        "v2-08-native-backend-self-regeneration",
        "v2-09-wasm-native-differential-zero",
        "v2-10-native-only-rc-distribution",
    ] {
        assert!(
            plan_content.contains(anchor),
            "phase11-implementation-plan.md に V2 節 anchor `{anchor}` が必要"
        );
    }

    let v2_08_doc = project_root
        .join("docs/development/planning/v2-designs/v2-08-native-backend-self-regeneration.md");
    let v2_09_doc = project_root
        .join("docs/development/planning/v2-designs/v2-09-wasm-native-differential-zero.md");
    let v2_10_doc = project_root
        .join("docs/development/planning/v2-designs/v2-10-native-only-rc-distribution.md");
    for doc in [&v2_08_doc, &v2_09_doc, &v2_10_doc] {
        assert!(doc.is_file(), "{} が存在しない", doc.display());
        let content = std::fs::read_to_string(doc)
            .unwrap_or_else(|e| panic!("{} の読み込みに失敗: {}", doc.display(), e));
        assert!(
            content.contains("Deferred")
                || content.contains("Phase 11 後")
                || content.contains("Component Model pivot"),
            "{} は Deferred / post-Phase11 方針を説明すること",
            doc.display()
        );
    }
}
