//! メタ検証テスト: ドキュメントとコードの整合性を検証する。
//!
//! プロジェクトのメタ情報 (completion-criteria, TODO, 仕様ドキュメント等) が
//! コードの実装状態と一致していることを自動検証する。

/// TEST-META-02: completion marker が 3状態 (pending/in-progress/done) で管理されていることを検証。
///
/// docs/completion-criteria.md を読み込み、各完了条件マーカーが
/// 3状態で管理されていることを assert する。
/// Red Phase: completion-criteria.md に 3状態マーカーが未導入のため FAIL する。
#[test]
fn test_meta_02_completion_marker_3_states() {
    let criteria_source = std::fs::read_to_string("docs/completion-criteria.md")
        .expect("docs/completion-criteria.md が存在しない");

    // 3状態マーカーの定義が存在すること
    // 期待: [pending], [in-progress], [done] またはそれに相当するマーカー形式
    let has_pending_marker = criteria_source.contains("[pending]")
        || criteria_source.contains("- [ ]")  // チェックボックス形式
        || criteria_source.contains("status: pending");
    let has_in_progress_marker = criteria_source.contains("[in-progress]")
        || criteria_source.contains("- [~]")  // 部分完了形式
        || criteria_source.contains("status: in-progress");
    let has_done_marker = criteria_source.contains("[done]")
        || criteria_source.contains("- [x]")  // 完了形式
        || criteria_source.contains("status: done");

    // 3状態全てが文書内に存在すること
    assert!(
        has_pending_marker,
        "completion-criteria.md に pending 状態のマーカーがない"
    );
    assert!(
        has_in_progress_marker,
        "completion-criteria.md に in-progress 状態のマーカーがない"
    );
    assert!(
        has_done_marker,
        "completion-criteria.md に done 状態のマーカーがない"
    );

    // 各セクション (技術/ドキュメント/撤去前ゲート) にマーカーが付与されていること
    let sections = [
        ("P11-2e-1", "技術完了条件"),
        ("P11-2e-2", "ドキュメント完了条件"),
        ("P11-2e-3", "撤去前ゲート"),
    ];

    for (section_id, section_name) in &sections {
        // セクション内に状態マーカーが存在すること
        let section_start = criteria_source
            .find(section_id)
            .unwrap_or_else(|| panic!("セクション {} ({}) が見つからない", section_id, section_name));

        // セクション末尾を次のセクション or EOF で区切る
        let section_end = criteria_source[section_start + section_id.len()..]
            .find("\n## ")
            .map(|pos| section_start + section_id.len() + pos)
            .unwrap_or(criteria_source.len());

        let section_text = &criteria_source[section_start..section_end];

        // セクション内に条件ごとの状態マーカーが存在すること
        let has_any_marker = section_text.contains("[pending]")
            || section_text.contains("[in-progress]")
            || section_text.contains("[done]")
            || section_text.contains("- [ ]")
            || section_text.contains("- [~]")
            || section_text.contains("- [x]");

        assert!(
            has_any_marker,
            "セクション {} ({}) に完了状態マーカーがない。\
             各条件に [pending]/[in-progress]/[done] を付与してください。",
            section_id, section_name
        );
    }

    // 全条件数が 0 でないこと (条件が定義されていることの検証)
    let condition_count = criteria_source.matches("### 条件").count()
        + criteria_source.matches("### ゲート").count();
    assert!(
        condition_count >= 7,
        "completion-criteria.md の条件数が不足: {} (7以上必要)",
        condition_count
    );

    // 各条件に状態マーカーが個別に付与されていることを検証
    // 「### 条件 N:」の直後に状態マーカーが含まれること
    for i in 1..=4 {
        let marker = format!("### 条件 {}", i);
        if criteria_source.contains(&marker) {
            let pos = criteria_source.find(&marker).unwrap();
            let next_section = criteria_source[pos + marker.len()..]
                .find("### ")
                .map(|p| pos + marker.len() + p)
                .unwrap_or(criteria_source.len());
            let condition_text = &criteria_source[pos..next_section];
            let has_state = condition_text.contains("[pending]")
                || condition_text.contains("[in-progress]")
                || condition_text.contains("[done]");
            assert!(
                has_state,
                "{} に個別の状態マーカーがない", marker
            );
        }
    }
}
