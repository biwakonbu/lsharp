#!/bin/bash
# ドキュメント同期ガード: 実装ファイル編集前に、この slice の正本ドキュメントが
# 更新されているかを確認する。PreToolUse (Edit|Write) で発火。
#
# tdd-guard.sh と同じ哲学で **ブロックしない** (常に exit 0)。
# 正当なリファクタ・調査・追試を止めないため、判断はエージェント側に委ねて警告のみ出す。
#
# エラーログ: /tmp/lsharp-hook-errors.log

LOG_FILE="/tmp/lsharp-hook-errors.log"

log_error() {
  echo "[$(date '+%Y-%m-%d %H:%M:%S')] [doc-guard] $1" >> "$LOG_FILE"
}

INPUT=$(cat || true)
if [[ -z "$INPUT" ]]; then
  exit 0
fi

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

# --- 対象の絞り込み ---------------------------------------------------------
# 「観測可能な挙動が変わりうる正本」だけを対象にする。
#   crates/*/src/*.rs      Rust 実装
#   selfhost/src/**/*.ls   L# 実装
#   scripts/*.sh /*.py     運用スクリプト (契約の一部)
#   Cargo.toml             ビルド契約
IS_TARGET=0
case "$FILE_PATH" in
  *"/crates/"*"/src/"*.rs | crates/*/src/*.rs) IS_TARGET=1 ;;
  *"/selfhost/src/"*.ls  | selfhost/src/*.ls)  IS_TARGET=1 ;;
  *"/scripts/"*.sh | scripts/*.sh)             IS_TARGET=1 ;;
  *"/scripts/"*.py | scripts/*.py)             IS_TARGET=1 ;;
  */Cargo.toml | Cargo.toml)                   IS_TARGET=1 ;;
esac
if [[ "$IS_TARGET" -eq 0 ]]; then
  exit 0
fi

# test 専用ファイルは除外する。テストの追加は TDD 側の規律であり、
# ドキュメント正本の更新を要求する変更ではない。
if [[ "$FILE_PATH" =~ _tests\.rs$ ]] \
  || [[ "$FILE_PATH" =~ /tests/ ]] \
  || [[ "$FILE_PATH" =~ /scripts/ci/test- ]] \
  || [[ "$FILE_PATH" =~ _test\.py$ ]]; then
  exit 0
fi

# --- repo root の解決 -------------------------------------------------------
# worktree で動くため cwd ではなく対象ファイル側から辿る。
SEARCH_DIR=$(dirname "$FILE_PATH")
while [[ -n "$SEARCH_DIR" && ! -d "$SEARCH_DIR" && "$SEARCH_DIR" != "/" ]]; do
  SEARCH_DIR=$(dirname "$SEARCH_DIR")
done
REPO_ROOT=$(git -C "$SEARCH_DIR" rev-parse --show-toplevel 2>/dev/null)
if [[ -z "$REPO_ROOT" ]]; then
  exit 0
fi

# --- 正本ドキュメントが既に触られているか ------------------------------------
# 「この slice でドキュメントを先に書いたか」を working tree の差分で判定する。
# 追跡外の新規 ADR も拾うため --porcelain の untracked を含めて見る。
DOC_CHANGES=$(git -C "$REPO_ROOT" status --porcelain -- \
  ISSUES.md TODO.md AGENTS.md CLAUDE.md docs 2>/dev/null)
if [[ -n "$DOC_CHANGES" ]]; then
  exit 0
fi

cat >&2 << 'EOF'
[Doc Guard] この slice でまだ正本ドキュメントを更新していません。
実装より先に「何を決めたか」を書いてください (doc-RED -> 実装 -> doc-GREEN)。

  ISSUES.md   問題台帳。何が問題か・根拠・状態。チェックボックスは置かない
  TODO.md     未完了タスクの正本。[x] は使わず [ ] / [~] / [BLOCKED: 理由]
  docs/adr/   判断とその根拠。何を採り、何を却下したか
  docs/development/operations/  計測値・運用手順
  AGENTS.md   日常運用の手順が変わる場合

判断が要らない機械的な変更 (test split、rename、typo) ならこの警告は無視して構いません。
詳細: .claude/rules/doc-sync.md
EOF

exit 0
