# TODO 完了 (品質改善) - 設計書

> 最終更新: 2026-03-25

## 概要

既存のコードベースに対する品質改善を実施した。新規アーキテクチャの変更は含まず、既存クレート内のリント修正、テスト復活、テスト実装化の 3カテゴリで構成される。全変更は既存ファイルの修正で完結している。

## アーキテクチャ

### 変更カテゴリ構成

```
カテゴリ 1 (clippy 修正)  : 11 ファイルのリント警告修正
カテゴリ 2 (テスト復活)   : E2E テストファイルの ignore 解除 + ヘルパー追加
カテゴリ 3 (テスト実装化) : Bootstrap テストの panic! -> 検証ロジック置換
```

### 変更範囲

変更は全て既存クレートのソースファイルに閉じており、クレート間の依存関係やパイプライン構造に影響しない。

## コンポーネント

### clippy 警告修正 (11 ファイル)

- **責務**: Rust の静的解析ツール clippy が検出した 43件の警告を修正
- **修正パターン**:
  - `allow` 属性追加 (dead_code, result_large_err 等のモジュールレベル抑制)
  - 不要な括弧の除去
  - if-let-in-match パターンの統合
  - doc コメントのインデント修正
  - collapsible_if の修正
- **対象ファイル**:
  - `crates/lsharp-types/src/infer.rs`
  - `crates/lsharp-types/src/constraints.rs`
  - `crates/lsharp-ir/src/lower/decl.rs`
  - `crates/lsharp-ir/src/lower/expr.rs`
  - `crates/lsharp-ir/src/lower/mod.rs`
  - `crates/lsharp-ir/src/lower/pattern.rs`
  - `crates/lsharp-wasm/src/wasi.rs`
  - `crates/lsharp-driver/src/commands/fmt.rs`
  - `crates/lsharp-driver/src/error.rs`
  - `crates/lsharp-driver/src/lockfile.rs`
  - `crates/lsharp-driver/src/main.rs`

### ignored テスト復活 (16 件)

- **責務**: `#[ignore]` 属性が付いた E2E テストを復活させ、テストカバレッジを回復
- **変更箇所**: `crates/lsharp-wasm/tests/e2e.rs`
- **復活内訳**:
  - MacroExpand テスト 5件: `compile_and_run_with_macros` ヘルパー関数を新規追加し、マクロ展開を含むパイプラインでのテストを可能にした
  - TypeInfer テスト 5件: テスト対象の型推論パスが安定したため ignore を解除
  - Pipeline テスト 3件: パイプライン統合テストの前提条件が満たされたため解除
  - Bootstrap テスト 3件: `panic!` プレースホルダーを実際の検証ロジックに置換して解除

### 残存 ignored テスト (2 件)

- **テスト名**: `selfhost_typeinfer_unification`, `selfhost_typeinfer_pattern_match`
- **理由**: Wasm codegen が未対応の言語機能に依存 (高階関数の値渡し、Int match + String 返却)
- **解除条件**: codegen の対応言語機能が拡張された時点で解除可能

## データ設計

該当なし。データモデル・データフローの変更は含まない。

## インターフェース

### API

該当なし。公開 API の変更は含まない。

### イベント

該当なし。

## エラーハンドリング

- `error.rs` のモジュールレベル `#[allow(dead_code)]` は将来のリファクタリング時に個別関数単位への細分化を推奨
- `infer.rs` のモジュールレベル `#[allow(clippy::result_large_err)]` は `Box<TypeError>` へのリファクタリングを推奨

## テスト戦略

### 復活テストの検証

全 16件の復活テストが `cargo test` でパスすることを確認:

| カテゴリ | 件数 | 検証方法 |
|----------|------|----------|
| MacroExpand | 5件 | compile_and_run_with_macros ヘルパーによるフルパイプライン実行 |
| TypeInfer | 5件 | 型推論結果の正確性検証 |
| Pipeline | 3件 | parse -> infer -> lower -> codegen のフルパイプライン検証 |
| Bootstrap | 3件 | panic! から実際の検証ロジック (出力値の assert) に置換 |

### テスト結果

- 合計: 868 pass, 0 failed, 2 ignored
- ignored 2件は既知の codegen 制限であり、対応不要

### レビュー指摘事項

| ID | 重要度 | 内容 | 対応 |
|----|--------|------|------|
| ISSUE-001 | 軽微 | error.rs のモジュールレベル dead_code allow | 将来リファクタリング時に個別化を推奨 |
| ISSUE-002 | 軽微 | infer.rs のモジュールレベル result_large_err allow | Box<TypeError> へのリファクタリングを推奨 |
| ISSUE-003 | 提案 | let-chain 構文の使用 | Rust 1.87+ で問題なし |

## 関連ドキュメント

- [要件定義書](requirements.md)
- [TODO 全残タスク完了 設計書](../todo-complete/design.md)
