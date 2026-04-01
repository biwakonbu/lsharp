# L# Claude Code Plugin

Claude Code 向けの L# LSP 統合プラグイン。

## 対応プラットフォーム

macOS / Linux のみ。Windows は動作保証の対象外。

## 前提条件

`lsharp` バイナリが PATH に存在すること。

```bash
cargo install --path crates/lsharp-driver
```

## インストール

```bash
claude --plugin-dir path/to/lsharp/editors/claude-code
```

## 提供機能

L# LSP サーバー (`lsharp lsp`) を stdio 経由で接続し、以下の機能を提供:

- Diagnostics (構文エラー、型エラー)
- Hover (型情報、`:doc` メタデータ)
- Completion (関数、キーワード、モジュール)
- Go to Definition
- Find References
- Rename
- Document Formatting

## 対象ファイル

`.ls` 拡張子のファイルで自動的に有効化される。
