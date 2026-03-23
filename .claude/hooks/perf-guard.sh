#!/bin/bash
# パフォーマンスガード: パフォーマンス影響ファイルの変更を検知し通知する
# PostToolUse (Edit|Write) で発火
#
# ブロックは行わない (常に exit 0)
# stderr に [Perf Guard] 接頭辞で情報通知のみ出力

LOG_FILE="/tmp/lsharp-hook-errors.log"

log_error() {
  echo "[$(date '+%Y-%m-%d %H:%M:%S')] [perf-guard] $1" >> "$LOG_FILE"
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

# パフォーマンスクリティカルなファイルパターン
PERF_PATTERNS=(
  "crates/lsharp-wasm/src/"
  "crates/lsharp-ir/src/"
  "crates/lsharp-types/src/infer.rs"
  "crates/lsharp-syntax/src/parser.rs"
  "crates/lsharp-syntax/src/lexer.rs"
)

MATCHED=false
for pattern in "${PERF_PATTERNS[@]}"; do
  if [[ "$FILE_PATH" == *"$pattern"* ]]; then
    MATCHED=true
    break
  fi
done

if [[ "$MATCHED" == "true" ]]; then
  cat >&2 << 'EOF'
[Perf Guard] パフォーマンスに影響しうるファイルが変更されました。
変更完了後に `/bench` でベンチマークを実行することを推奨します。
  計測対象: compile | wasm-size | runtime | memory | compare | all
EOF
fi

exit 0
