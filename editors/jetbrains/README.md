# L# JetBrains IDE 設定

IntelliJ IDEA / CLion / WebStorm 等の JetBrains IDE 向け L# LSP 統合。

## 前提条件

- JetBrains IDE 2024.1+
- [LSP4IJ](https://plugins.jetbrains.com/plugin/23257-lsp4ij) プラグイン
- `lsharp` バイナリが PATH に存在すること

## セットアップ

### 1. LSP4IJ プラグインのインストール

Settings > Plugins > Marketplace で「LSP4IJ」を検索してインストール。

### 2. L# 言語サーバーの登録

Settings > Languages & Frameworks > Language Servers で新規サーバーを追加:

| 項目 | 値 |
|------|------|
| Name | L# |
| Command | `lsharp lsp` |
| File patterns | `*.ls` |

### 3. ファイルタイプの関連付け

Settings > Editor > File Types で新規ファイルタイプを追加:

| 項目 | 値 |
|------|------|
| Name | L# |
| Line comment | `;` |
| File name patterns | `*.ls` |

## 提供機能

LSP4IJ 経由で以下の機能が利用可能:

- Diagnostics (構文エラー、型エラー)
- Hover (型情報、ドキュメント)
- Completion (関数、キーワード、モジュール)
- Go to Definition (Ctrl+Click / Ctrl+B)
- Find References (Alt+F7)
- Rename (Shift+F6)
- Document Formatting (Ctrl+Alt+L)

## 制限事項

- シンタックスハイライトは LSP の semantic tokens ではなく、ファイルタイプ設定に依存
- 高度なハイライトが必要な場合は TextMate bundle のインポートを検討
