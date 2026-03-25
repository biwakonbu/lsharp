#!/bin/bash
# L# ロールバックスクリプト
# selfhost コンパイラから Rust 実装への緊急ロールバック手順
#
# 使い方:
#   ./scripts/rollback.sh [--dry-run]
#
# 詳細な手順は docs/rollback-procedure.md を参照

set -euo pipefail

DRY_RUN=false
if [[ "${1:-}" == "--dry-run" ]]; then
    DRY_RUN=true
    echo "[DRY RUN] ロールバックのシミュレーションを実行します"
fi

echo "=== L# ロールバック手順 ==="
echo ""

# 1. 現在の状態を確認
echo "--- Step 1: 現在の状態確認 ---"
echo "  Git ブランチ: $(git branch --show-current)"
echo "  最新コミット: $(git log --oneline -1)"

# 2. legacy-rust-bootstrap の存在確認
echo ""
echo "--- Step 2: レガシーコードの確認 ---"
if [[ -d legacy-rust-bootstrap ]]; then
    echo "  PASS: legacy-rust-bootstrap/ が存在"
else
    echo "  WARN: legacy-rust-bootstrap/ が見つからない"
    echo "  ロールバック先のコードが利用できません"
    exit 1
fi

# 3. ロールバック実行
echo ""
echo "--- Step 3: ロールバック ---"
if $DRY_RUN; then
    echo "  [DRY RUN] crates/ を legacy-rust-bootstrap/ の内容で復元"
    echo "  [DRY RUN] Cargo.toml/Cargo.lock を復元"
    echo "  [DRY RUN] cargo build で検証"
    echo "  [DRY RUN] cargo test で検証"
else
    echo "  WARNING: 実際のロールバックは手動で実行してください"
    echo "  詳細: docs/rollback-procedure.md"
fi

echo ""
echo "=== ロールバック手順完了 ==="
echo "詳細な手順は docs/rollback-procedure.md を参照してください"
