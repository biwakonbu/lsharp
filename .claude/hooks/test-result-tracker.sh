#!/bin/bash
# テスト実行結果トラッカー: cargo test 実行後にテスト結果を記録する
# PostToolUse (Bash) で発火
#
# エラー時はツール実行をブロックしないよう、各コマンドの失敗を個別処理する
# エラーログ: /tmp/lsharp-hook-errors.log

LOG_FILE="/tmp/lsharp-hook-errors.log"

log_error() {
  echo "[$(date '+%Y-%m-%d %H:%M:%S')] [test-tracker] $1" >> "$LOG_FILE"
}

INPUT=$(cat || true)
if [[ -z "$INPUT" ]]; then
  log_error "stdin が空 (入力なし)"
  exit 0
fi

# jq が利用できない場合はログに記録してスキップ
if ! command -v jq &>/dev/null; then
  log_error "jq が見つかりません。brew install jq でインストールしてください"
  exit 0
fi

TOOL_NAME=$(echo "$INPUT" | jq -r '.tool_name // empty')
if [[ "$TOOL_NAME" != "Bash" ]]; then
  exit 0
fi

# cargo test コマンドかどうか確認
COMMAND=$(echo "$INPUT" | jq -r '.tool_input.command // empty')
if [[ ! "$COMMAND" =~ cargo[[:space:]]+test ]]; then
  exit 0
fi

# テスト結果を確認
# PostToolUse の出力フィールドは stdout / output のいずれかの可能性がある
STDOUT=$(echo "$INPUT" | jq -r '.tool_output.stdout // .tool_output.output // .stdout // .output // empty')

# 出力が空の場合はスキップ (スキーマ不明時のフォールバック)
if [[ -z "$STDOUT" ]]; then
  exit 0
fi

# テスト失敗を検出
if echo "$STDOUT" | grep -q 'test result: FAILED'; then
  # macOS 互換: grep -oE を使用 (grep -oP は BSD grep 非対応)
  FAILED_COUNT=$(echo "$STDOUT" | grep -oE '[0-9]+ failed' | head -1)
  cat >&2 << EOF
[TDD Tracker] テスト失敗を検出: ${FAILED_COUNT:-unknown}
次のアクション:
  - 失敗したテストを確認
  - 実装を修正 (テストの期待値を変更しない)
  - 再度 cargo test を実行
EOF
fi

# テスト成功を検出
if echo "$STDOUT" | grep -q 'test result: ok'; then
  PASS_INFO=$(echo "$STDOUT" | grep 'test result: ok' | tail -1)
  cat >&2 << EOF
[TDD Tracker] テスト成功: ${PASS_INFO}
次のアクション:
  - TODO.md の該当項目を更新 (テスト数を注記)
  - cargo clippy で警告確認
EOF
fi

exit 0
