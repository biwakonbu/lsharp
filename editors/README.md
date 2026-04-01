# L# エディタ統合

L# LSP サーバー (`lsharp lsp`) を各エディタ / AI コーディングツールで利用するための設定。

## 対応状況

| エディタ / ツール | 方式 | ディレクトリ | 状態 |
|-------------------|------|-------------|------|
| VS Code | 拡張機能 (VSIX) | [vscode/](vscode/) | 実装済み |
| Cursor | VS Code 拡張を共用 | [vscode/](vscode/) | 実装済み |
| Windsurf | VS Code 拡張を共用 | [vscode/](vscode/) | 実装済み |
| GitHub Copilot | エディタ側 LSP を利用 | - | エディタ設定で対応 |
| Codex CLI | エディタ側 LSP を利用 | - | エディタ設定で対応 |
| Claude Code | プラグイン | [claude-code/](claude-code/) | 実装済み |
| Neovim | lspconfig レシピ | [neovim/](neovim/) | 実装済み |
| JetBrains IDE | LSP4IJ 設定ガイド | [jetbrains/](jetbrains/) | 設定ガイド |

## 対応プラットフォーム

| プラットフォーム | 状態 |
|-----------------|------|
| macOS (aarch64 / x86_64) | 対応 |
| Linux (x86_64) | 対応 |
| Windows | 非対応 |

> Windows は現在動作保証の対象外。`lsharp` バイナリのビルドおよび LSP サーバーの動作は macOS / Linux のみで検証している。

## 前提条件

全エディタ共通で `lsharp` バイナリが PATH に必要:

```bash
cargo install --path crates/lsharp-driver
```

## LSP 機能一覧

L# LSP サーバーが提供する機能:

| 機能 | 説明 |
|------|------|
| Diagnostics | 構文エラー、型エラーのリアルタイム表示 |
| Hover | 型情報と `:doc` メタデータの表示 |
| Completion | 関数、キーワード、モジュールの補完 |
| Go to Definition | 定義元へのジャンプ |
| Find References | シンボルの使用箇所検索 |
| Rename | シンボルのリネーム |
| Document Formatting | S 式の整形 |

## アーキテクチャ

```
Editor/Tool
    |
    | stdio (JSON-RPC 2.0)
    |
    v
lsharp lsp (tower-lsp 0.20)
    |
    +-- lsharp-syntax (Lexer + Parser)
    +-- lsharp-types  (Hindley-Milner 型推論)
```

各エディタプラグインは `lsharp lsp` プロセスを spawn し、stdio 経由で LSP プロトコルを通信する薄いラッパー。
