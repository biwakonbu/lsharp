---
description: TDD ワークフロー強制ルール - 実装ファイル編集時に適用
globs:
  - "crates/**/src/**/*.rs"
---

# TDD ワークフロー (テスト駆動開発)

このプロジェクトでは TDD を必須とする。実装ファイルを編集する前に、必ずテストを先に書くこと。

## 必須フロー

1. **RED**: テストを書く → `cargo test` で失敗を確認
2. **GREEN**: 実装を書く → `cargo test` で成功を確認
3. **REFACTOR**: リファクタリング → `cargo test` で成功を維持

## 禁止事項

- テストなしで実装コードを書くこと
- テストなしで TODO.md の項目を `[x]` にすること
- テストが失敗した状態で次のタスクに移ること
- テストの期待値を実装に合わせて変更すること (テストの設計ミスを除く)

## テスト配置ルール

- 型推論テスト: `crates/lsharp-types/src/infer.rs` 内の `#[cfg(test)] mod tests`
- 制約テスト: `crates/lsharp-types/src/constraints.rs` 内の `#[cfg(test)] mod tests`
- メタデータテスト: `crates/lsharp-types/src/metadata_check.rs` 内の `#[cfg(test)] mod tests`
- IR テスト: `crates/lsharp-ir/src/lower.rs` 内の `#[cfg(test)] mod tests`
- E2E テスト: `crates/lsharp-wasm/tests/e2e.rs`
- スナップショットテスト: `insta` クレートの `assert_snapshot!` / `assert_ir!` マクロ

## TODO 更新ルール

実装完了時に TODO.md を更新する際:
- `[x]` マークにはテスト内容を注記する (例: `-- ユニットテスト 5 個、E2E 2 個追加`)
- テスト 0 個の項目は `[x]` にしない (`[~]` で留める)
- 検証債務セクションの該当行も更新する
