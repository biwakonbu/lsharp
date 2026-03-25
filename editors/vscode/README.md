# L# Language Support for VSCode

S 式構文 + Hindley-Milner 型推論の言語 L# の VSCode 拡張。

## 機能

- シンタックスハイライト (TextMate grammar)
- リアルタイム診断 (構文エラー・型エラー)
- 定義ジャンプ (Cmd+Click / F12)
- 全参照検索 (Shift+F12)
- シンボルリネーム (F2)
- ドキュメントフォーマット (Shift+Alt+F)

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
