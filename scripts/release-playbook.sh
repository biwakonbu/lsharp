#!/bin/bash
# L# リリースプレイブック
# リリース手順を自動化するスクリプト
#
# 使い方:
#   ./scripts/release-playbook.sh <version>
#   例: ./scripts/release-playbook.sh 0.2.0

set -euo pipefail

SCRIPT_DIR=$(cd "$(dirname "$0")" && pwd)
ROOT_DIR=$(cd "$SCRIPT_DIR/.." && pwd)
PLAYBOOK_DIR="${PLAYBOOK_DIR:-$ROOT_DIR/target/release-playbook}"
VERSION="${1:-}"
LSHARP_BIN="${LSHARP_BIN:-$ROOT_DIR/target/release/lsharp}"

if [[ -z "$VERSION" ]]; then
    echo "使い方: $0 <version>"
    echo "例: $0 0.2.0"
    exit 1
fi

mkdir -p "$PLAYBOOK_DIR"
cd "$ROOT_DIR"

echo "=== L# リリースプレイブック v${VERSION} ==="
echo ""

# 1. ビルド検証
echo "--- Step 1: ビルド検証 ---"
cargo build --release
if [[ ! -x "$LSHARP_BIN" ]]; then
    echo "ERROR: release binary not executable: $LSHARP_BIN"
    exit 1
fi
echo "PASS: ビルド成功"

# 2. テスト実行
echo ""
echo "--- Step 2: テスト実行 ---"
cargo test
echo "PASS: 全テスト通過"

# 3. Clippy リント
echo ""
echo "--- Step 3: Clippy リント ---"
cargo clippy -- -D warnings
echo "PASS: リント通過"

# 4. フォーマット確認
echo ""
echo "--- Step 4: フォーマット確認 ---"
cargo fmt --check
echo "PASS: フォーマット OK"

# 5. ブートストラップ検証
echo ""
echo "--- Step 5: ブートストラップ検証 ---"
OUT_DIR="$PLAYBOOK_DIR/bootstrap" \
LSHARP_BIN="$LSHARP_BIN" \
    bash scripts/ci/compile-phase11-inputs.sh
echo "PASS: ブートストラップ成功"

# 6. smoke test
echo ""
echo "--- Step 6: Smoke Test ---"
OUT_DIR="$PLAYBOOK_DIR/default-path-smoke" \
LSHARP_BIN="$LSHARP_BIN" \
    bash scripts/ci/default-path-smoke.sh
if [[ -f scripts/smoke_test_readme.sh ]]; then
    SMOKE_DIR="$PLAYBOOK_DIR/readme-smoke" \
    LSHARP_BIN="$LSHARP_BIN" \
        bash scripts/smoke_test_readme.sh
    echo "PASS: Smoke test 成功"
else
    echo "SKIP: smoke_test_readme.sh が見つからない"
fi

# 7. チェックサム生成
echo ""
echo "--- Step 7: チェックサム生成 ---"
if [[ -f scripts/checksum.sh ]]; then
    echo "チェックサムスクリプトが利用可能"
else
    echo "WARN: checksum.sh が見つからない"
fi

echo ""
echo "=== リリース準備完了: v${VERSION} ==="
echo "検証成果物: $PLAYBOOK_DIR"
echo ""
echo "次のステップ:"
echo "  1. git tag v${VERSION}"
echo "  2. git push origin v${VERSION}"
echo "  3. GitHub Releases でリリースノートを作成"
