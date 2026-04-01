# L# Language Support for VS Code

S 式構文 + Hindley-Milner 型推論の言語 L# の VS Code / Cursor / Windsurf 拡張。

## 機能

- シンタックスハイライト (TextMate grammar)
- リアルタイム診断 (構文エラー・型エラー)
- Hover (型情報、`:doc` メタデータ)
- 補完 (関数、キーワード、モジュール)
- 定義ジャンプ (Cmd+Click / F12)
- 全参照検索 (Shift+F12)
- シンボルリネーム (F2)
- ドキュメントフォーマット (Shift+Alt+F)

## Cursor / Windsurf での利用

この拡張は VS Code fork である Cursor と Windsurf でもそのまま動作する。
GitHub Copilot や Codex CLI もエディタ内の LSP を利用するため、追加設定なしで L# の補完・診断が有効になる。

Cursor の場合:
```bash
cursor --install-extension lsharp-vscode-0.1.0.vsix
```

## インストール

### ワンコマンド (推奨)

プロジェクトルートから:

```bash
bash scripts/install-vscode-ext.sh
```

`cargo build --release` → `npm install` → VSIX パッケージング → VSCode インストールを一括実行します。

### 手動インストール

```bash
# 1. lsharp バイナリをビルド
cargo build --release

# 2. VSCode 拡張をビルド & インストール
cd editors/vscode
npm install
npm run install-ext
```

### 拡張のみ再ビルド (lsharp バイナリ変更なし)

```bash
cd editors/vscode
npm run install-ext
```

## 前提条件

- `lsharp` バイナリが PATH に含まれていること
- Node.js & npm
- VSCode (または Cursor)

## 設定

| 設定 | 説明 | デフォルト |
|------|------|-----------|
| `lsharp.lspPath` | lsharp バイナリのパス | `""` (PATH から自動検索) |

`lsharp` バイナリが PATH にない場合は、VSCode 設定で `lsharp.lspPath` にフルパスを指定してください。

## 開発

```bash
cd editors/vscode
npm install
npm run watch    # ファイル変更を監視して自動ビルド
```

F5 キーで Extension Development Host を起動してデバッグできます。
