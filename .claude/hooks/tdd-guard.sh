#!/bin/bash
# TDD ガード: 実装ファイル編集前にテスト状況を確認する
# PreToolUse (Edit|Write) で発火
#
# エラー時はツール実行をブロックしないよう、各コマンドの失敗を個別処理する
# エラーログ: /tmp/lsharp-hook-errors.log

LOG_FILE="/tmp/lsharp-hook-errors.log"

log_error() {
  echo "[$(date '+%Y-%m-%d %H:%M:%S')] [tdd-guard] $1" >> "$LOG_FILE"
}

INPUT=$(cat || true)
if [[ -z "$INPUT" ]]; then
  exit 0
fi

# jq が利用できない場合はログに記録してスキップ
if ! command -v jq &>/dev/null; then
  log_error "jq が見つかりません。brew install jq でインストールしてください"
  exit 0
fi

TOOL_NAME=$(echo "$INPUT" | jq -r '.tool_name // empty' 2>/dev/null)
if [[ "$TOOL_NAME" != "Edit" && "$TOOL_NAME" != "Write" ]]; then
  exit 0
fi

FILE_PATH=$(echo "$INPUT" | jq -r '.tool_input.file_path // empty' 2>/dev/null)
if [[ -z "$FILE_PATH" ]]; then
  exit 0
fi

# crates/*/src/*.rs の実装ファイルのみ対象
# テストファイル、examples、docs は除外
if [[ ! "$FILE_PATH" =~ crates/.*/src/.*\.rs$ ]]; then
  exit 0
fi

# 書き込み内容にテスト関連コードが含まれていれば OK (テストを書いている最中)
# Edit: new_string, Write: content
CONTENT=$(echo "$INPUT" | jq -r '.tool_input.content // .tool_input.new_string // empty' 2>/dev/null)
if echo "$CONTENT" | grep -qE '#\[cfg\(test\)\]|#\[test\]|mod tests'; then
  exit 0
fi

# 新規ファイル作成の場合はスキップ (まだテストモジュールがなくて当然)
if [[ ! -f "$FILE_PATH" ]]; then
  exit 0
fi

# 既存ファイルにテストモジュールがあるか確認
if grep -q '#\[cfg(test)\]' "$FILE_PATH"; then
  # テストモジュールが既にある -- OK
  exit 0
fi

# NOTE: crate の tests/ ディレクトリ存在チェックは意図的に省略
# tests/ に無関係なテストがあるだけでガードをスルーしてしまうため
# インラインテスト (#[cfg(test)]) の存在のみをチェックする

# テストが見つからない: stderr に警告を出力
# exit 0 で実行は許可 (ブロックはしない)
cat >&2 << 'EOF'
[TDD Guard] このファイルにテストモジュールが見つかりません。
TDD ワークフロー: テストを先に書いてから実装してください。
  1. #[cfg(test)] mod tests { ... } を追加
  2. テストが失敗することを確認
  3. 実装を書く
  4. テストが通ることを確認
EOF

exit 0
