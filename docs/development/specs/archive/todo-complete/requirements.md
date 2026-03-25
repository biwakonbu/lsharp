# TODO 全残タスク完了 - 要件定義書

> 最終更新: 2026-03-24

## 概要

TODO.md に残存していた全未完了タスク (47件完了 / 51件中) を対象とし、構造的バグ修正、リファクタリング検証、テスト・品質基盤整備、LSP 全機能実装、文字列ヒープ化、セルフホスティング統合、CI/CD 構築を実施した。実装可能な全タスクは完了済み。残り 4件はセルフコンパイル能力の不足による技術的ブロックであり、大規模な新規開発を要する。

## 機能要件

### 必須機能 (バグ修正)

- **FR-001**: BUG-1 (ADR-064) -- FieldAccess の型解決で `infer_expr_type_name()` を拡張し、FieldAccess/Let/Match 式の型推定を追加。フォールバック時の誤選択を排除する
- **FR-002**: BUG-2 (ADR-067) -- RecordUpdate の型推定で FR-001 と同じ `infer_expr_type_name()` 拡張を適用し、フォールバック時の誤選択を排除する
- **FR-003**: BUG-3 (ADR-073) -- `lower_match_arms` のコンストラクタパターンで、最後の腕でもタグ比較を行い、不一致時に unreachable を出力する

### 必須機能 (実装済み検証)

- **FR-004**: IMP-1 (ADR-054) -- パーサーのエラーリカバリ (複数エラー一括報告) が動作することをテストで検証
- **FR-005**: IMP-2 (ADR-055) -- 制約階層の互換性チェック (親子制約の包含判定) が動作することをテストで検証
- **FR-006**: IMP-3 (ADR-056) -- config.rs の `load_config_result()` が Result を返すことをテストで検証
- **FR-007**: IMP-4 (ADR-066) -- `run_wasm_wasi` ヘルパーが `wasi_runner.rs` に統合されていることを確認

### 必須機能 (品質基盤)

- **FR-008**: QA-1 (ADR-057) -- 正規表現エンジンを NFA->DFA 変換方式に置換。Unicode 文字クラス (`\p{L}` 等) をサポート。constraints.rs のモジュール分割を含む
- **FR-009**: QA-2 (ADR-072) -- `parse_test_output` の `generate_sample_args` 重複呼び出しを解消。結果を事前計算して引数渡しにする
- **FR-010**: QA-3 (ADR-075) -- 型推論結果の受け渡しを HashMap 化し、線形探索を O(1) に改善
- **FR-011**: QA-4 (ADR-077) -- codegen/wasi の Wasm バイナリ出力に対する insta スナップショットテストを追加
- **FR-012**: QA-5 (ADR-078) -- criterion ベンチマークを追加 (parse, infer, lower, codegen の各ステージ)

### 必須機能 (エコシステム)

- **FR-013**: P9-2 -- LSP の定義ジャンプ・型ホバーを完成させ、completion/references/rename/formatting を実装する。ドキュメントキャッシュ (`HashMap<Url, String>`) を追加
- **FR-014**: CI/CD -- GitHub Actions ワークフロー作成 (`cargo test` + `cargo clippy` + `cargo fmt --check`)。PR トリガー + push トリガー。PR マージブロック設定を含む

### 必須機能 (大規模)

- **FR-015**: P1-2 -- 文字列リテラルを data section offset からヒープ上 String オブジェクト (tag=1, len, bytes) に変換。全文字列関連ビルトインの更新
- **FR-016**: P8-5 -- セルフホスティング統合。全モジュール (Token/Lexer/AST/Parser/IR/Type/TypeScheme/Compiler/WasmEmit) を結合した統合コンパイラを構築し、stage1.wasm を生成

### 追加実装

- **FR-017**: パターンマッチ改善 -- ネストしたコンストラクタパターンおよびガード条件を実装

## 非機能要件

### パフォーマンス

- **NFR-PERF-001**: QA-3 の HashMap 化により、型推論結果の参照を O(1) に改善
- **NFR-PERF-002**: QA-1 の NFA->DFA 変換により、正規表現マッチングのパフォーマンスを改善

### 保守性

- **NFR-MAINT-001**: constraints.rs (1811行) を QA-1 実装前にモジュール分割し、500-800行/ファイルの制限を満たす
- **NFR-MAINT-002**: 全変更に対してテストを追加し、TDD ワークフローに従う
- **NFR-MAINT-003**: 既存テストを壊さない (P1-2 の破壊的変更時はテストも同時更新)

### 互換性

- **NFR-COMPAT-001**: P1-2 の文字列ヒープ化は破壊的変更。全文字列関連テスト (string-char-at, substring, string-concat, string-eq, string-length, print-string) の修正が必須
- **NFR-COMPAT-002**: 既存の E2E テストを全て維持する

## 制約条件

- **CON-001**: ファイルサイズ制限 500-800行/ファイル。新規・既存ファイルともに遵守
- **CON-002**: TDD 必須。テストなしの実装は完了と見なさない
- **CON-003**: 日本語コメント、英語変数名
- **CON-004**: P8-5 は P1-2 (文字列ヒープ化) が前提条件

## 受入条件

- **AC-001**: 実装可能な全タスクが完了していること (47/47 達成)
- **AC-002**: `cargo test` が全テストパスすること (701件パス)
- **AC-003**: `cargo clippy` がエラーなしでパスすること
- **AC-004**: 各タスクに対応するテストが追加されていること (約 130件追加)
- **AC-005**: BUG-1/2 の再現テスト (同名フィールドの別レコード型) がパスすること
- **AC-006**: BUG-3 の再現テスト (非網羅パターンマッチ) がパスすること
- **AC-007**: LSP の hover/goto_definition が実際にソースから型情報・定義位置を返すこと
- **AC-008**: GitHub Actions の CI が `cargo test` + `cargo clippy` + `cargo fmt --check` を実行すること
- **AC-009**: P8-5 で stage1.wasm が生成でき、最低限の L# プログラムをコンパイルできること

## 除外事項

- WasmGC バックエンドの実装 (リニアメモリが正式基盤)
- Region GC の実装
- REPL の実装
- P8-5 の stage2.wasm 生成・固定点検証 (セルフコンパイル能力の拡張が前提)

## 実績サマリー

| 区分 | 完了数 | テスト数 |
|------|--------|---------|
| バグ修正 (BUG-1/2/3) | 3件 | 10件 |
| リファクタリング検証 (IMP-1/2/3/4) | 4件 | 12件 |
| 品質基盤 (QA-1/2/3/4/5) | 5件 | 71件 |
| LSP (P9-2) | 1件 | 27件 |
| 文字列ヒープ化 (P1-2) | 1件 | 8件 |
| パターンマッチ改善 | 1件 | 4件 |
| セルフホスト (P8-5 stage1) | 1件 | 2件 |
| CI/CD | 1件 | - |
| **合計** | **17件** | **約 130件** |

## 関連ドキュメント

- [設計書](design.md)
- [TODO 未完了タスク並列実装 仕様](../todo-parallel-implementation/requirements.md)
