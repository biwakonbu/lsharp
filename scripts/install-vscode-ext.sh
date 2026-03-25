#!/usr/bin/env bash
# L# VSCode 拡張のビルド＆インストールスクリプト
# 使い方: bash scripts/install-vscode-ext.sh
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
ROOT_DIR="$(dirname "$SCRIPT_DIR")"

echo "==> cargo build --release (lsharp バイナリ)"
cd "$ROOT_DIR"
cargo build --release

echo "==> npm install & VSCode 拡張パッケージング"
cd "$ROOT_DIR/editors/vscode"
npm install
npm run install-ext

echo "==> 完了! VSCode を再起動して L# 拡張を有効化してください。"
