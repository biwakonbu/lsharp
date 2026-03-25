# TODO 完了 (品質改善) - 要件定義書

> 最終更新: 2026-03-25

## 概要

TODO.md の対応不足分を全て完了するための品質改善タスク。clippy 警告の全件修正、ignored テストの削減、Bootstrap テストの実装化を実施し、コードベース全体の品質基盤を強化した。

## 機能要件

### 必須機能

- **FR-001**: clippy 警告全修正 -- 43件の clippy 警告を 0件に修正する。allow 属性追加、不要な括弧除去、if-let-in-match パターン統合、doc インデント修正、collapsible_if 修正を含む
- **FR-002**: ignored テスト削減 -- 18件の ignored テストを 2件に削減する。MacroExpand 5件、TypeInfer 5件、Pipeline 3件、Bootstrap 3件を復活させる
- **FR-003**: Bootstrap テスト実装化 -- Bootstrap テスト 3件の `panic!` プレースホルダーを実際の検証ロジックに置換する

### 補助機能

- **FR-004**: MacroExpand テスト復活用の `compile_and_run_with_macros` ヘルパー関数を追加する

## 非機能要件

### パフォーマンス

- **NFR-PERF-001**: 品質改善による実行時パフォーマンスの劣化がないこと

### 保守性

- **NFR-MAINT-001**: clippy 警告 0件を維持し、CI での clippy チェックを継続可能な状態とする
- **NFR-MAINT-002**: ignored テストの理由を明確にドキュメント化する

### 互換性

- **NFR-COMPAT-001**: 既存テストの期待値を変更しないこと (テスト設計ミスを除く)
- **NFR-COMPAT-002**: 既存の公開 API に影響を与えないこと

## 受入条件

- **AC-001**: `cargo clippy` が警告 0件でパスすること
- **AC-002**: `cargo test` が全テストパスすること (868 pass, 0 failed)
- **AC-003**: ignored テストが 2件以下であること
- **AC-004**: 残存 ignored テストに対して理由が明記されていること
- **AC-005**: Bootstrap テストが `panic!` ではなく実際の検証ロジックで動作すること

## 制約条件

- **CON-001**: ファイルサイズ制限 500-800行/ファイルを遵守
- **CON-002**: TDD ワークフローに従う
- **CON-003**: 日本語コメント、英語変数名

## 除外事項

- Wasm codegen 未対応の言語機能 (高階関数の値渡し、Int match + String 返却) の実装
- 新規言語機能の追加
- パフォーマンス最適化

## 実績サマリー

| 指標 | 改善前 | 改善後 |
|------|--------|--------|
| clippy 警告 | 43件 | 0件 |
| ignored テスト | 18件 | 2件 |
| Bootstrap テスト (panic!) | 3件 | 0件 |
| テスト結果 | - | 868 pass, 0 failed, 2 ignored |

### 残存 ignored テストの理由

| テスト名 | 理由 |
|----------|------|
| selfhost_typeinfer_unification | 高階関数の値渡しが Wasm codegen 未対応 |
| selfhost_typeinfer_pattern_match | Int match + String 返却が codegen 未対応 |

## 関連ドキュメント

- [設計書](./design.md)
- [TODO 全残タスク完了 仕様](../todo-complete/requirements.md)
