#!/bin/bash
# L# ロールバックスクリプト
# host launcher + guest component の緊急ロールバック案内
#
# 使い方:
#   ./scripts/rollback.sh [--dry-run] <last-known-good-tag> [guest-component-asset]
#
# 詳細な手順は docs/development/operations/rollback-procedure.md を参照

set -euo pipefail

DRY_RUN=false
if [[ "${1:-}" == "--dry-run" ]]; then
    DRY_RUN=true
    echo "[DRY RUN] ロールバックのシミュレーションを実行します"
    shift
fi

LKG_TAG="${1:-}"
LKG_COMPONENT_ASSET="${2:-}"

echo "=== L# ロールバック手順 ==="
echo ""

# 1. 現在の状態を確認
echo "--- Step 1: 現在の状態確認 ---"
echo "  Git ブランチ: $(git branch --show-current)"
echo "  最新コミット: $(git log --oneline -1)"
if [[ -n "$(git status --short)" ]]; then
    echo "  WARN: 作業ツリーが dirty です。rollback 前に退避または別ブランチ化してください"
else
    echo "  PASS: 作業ツリーは clean"
fi

# 2. last-known-good anchor の確認
echo ""
echo "--- Step 2: last-known-good anchor の確認 ---"
if [[ -z "$LKG_TAG" ]]; then
    echo "  ERROR: last-known-good release tag を指定してください"
    echo "  例: $0 --dry-run v0.2.3 target/release-playbook/lsharp-component.wasm"
    exit 1
fi

echo "  LKG tag: $LKG_TAG"
if git rev-parse -q --verify "refs/tags/$LKG_TAG" >/dev/null; then
    echo "  PASS: ローカルに tag $LKG_TAG が存在"
else
    echo "  WARN: ローカルに tag $LKG_TAG が見つかりません"
    echo "  GitHub Release notes の Rollback anchor と asset 名を確認してください"
fi

if [[ -n "$LKG_COMPONENT_ASSET" ]]; then
    if [[ -e "$LKG_COMPONENT_ASSET" ]]; then
        echo "  PASS: guest component asset が存在: $LKG_COMPONENT_ASSET"
    else
        echo "  WARN: guest component asset が見つからない: $LKG_COMPONENT_ASSET"
        echo "  GitHub Release asset から再取得してください"
    fi
else
    echo "  INFO: guest component asset 未指定。Rollback anchor に記録された asset 名を参照してください"
fi

# 3. ロールバック計画
echo ""
echo "--- Step 3: host launcher / guest component ロールバック ---"
if $DRY_RUN; then
    echo "  [DRY RUN] git checkout $LKG_TAG -- ."
    echo "  [DRY RUN] Rollback anchor の host launcher asset / guest component asset を復元"
    echo "  [DRY RUN] cargo build --release で再ビルド"
    echo "  [DRY RUN] cargo test で検証"
    echo "  [DRY RUN] LSHARP_BIN=target/release/lsharp bash scripts/ci/default-path-smoke.sh"
else
    echo "  WARNING: 実際のロールバックは手動で実行してください"
    echo "  1. GitHub Release notes の Rollback anchor で tag / asset 名 / checksum を確認"
    echo "  2. git checkout $LKG_TAG -- ."
    echo "  3. host launcher / guest component を同じ anchor の asset set に戻す"
    echo "  4. cargo build --release && cargo test"
    echo "  5. LSHARP_BIN=target/release/lsharp bash scripts/ci/default-path-smoke.sh"
    echo "  詳細: docs/development/operations/rollback-procedure.md"
fi

echo ""
echo "=== ロールバック手順完了 ==="
echo "詳細な手順は docs/development/operations/rollback-procedure.md を参照してください"
