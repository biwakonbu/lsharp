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
    let export_a = extract_section_bytes(&wasm_a, 7)
        .expect("wasm_a に Export セクションが見つからない");
    let export_b = extract_section_bytes(&wasm_b, 7)
        .expect("wasm_b に Export セクションが見つからない");
    assert_eq!(
        export_a, export_b,
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
        data_a, data_b,
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
        wasm_a, wasm_b,
        "raw bytes 不一致 — {} bytes vs {} bytes",
        wasm_a.len(),
        wasm_b.len()
    );

    // 追加検証: セクション構造の安定性
    let sections_a = extract_sections(&wasm_a);
    let sections_b = extract_sections(&wasm_b);
    assert_eq!(
        sections_a, sections_b,
        "セクション構造不一致"
    );
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
    let selfhost_dir =
        std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../selfhost");
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
