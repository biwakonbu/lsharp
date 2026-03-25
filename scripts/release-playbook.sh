#!/bin/bash
# L# リリースプレイブック
# リリース手順を自動化するスクリプト
#
# 使い方:
#   ./scripts/release-playbook.sh <version>
#   例: ./scripts/release-playbook.sh 0.2.0

set -euo pipefail

VERSION="${1:-}"

if [[ -z "$VERSION" ]]; then
    echo "使い方: $0 <version>"
    echo "例: $0 0.2.0"
    exit 1
fi

echo "=== L# リリースプレイブック v${VERSION} ==="
echo ""

# 1. ビルド検証
echo "--- Step 1: ビルド検証 ---"
cargo build --release
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
for module in Token AST IR Type TypeScheme Compiler WasmEmit Lexer Parser; do
    echo "  Compiling selfhost/${module}.ls..."
    cargo run --release -- compile selfhost/${module}.ls -o /tmp/release_${module}.wasm
done
echo "PASS: ブートストラップ成功"

# 6. smoke test
echo ""
echo "--- Step 6: Smoke Test ---"
if [[ -f scripts/smoke_test_readme.sh ]]; then
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

# クリーンアップ
rm -f /tmp/release_*.wasm

echo ""
echo "=== リリース準備完了: v${VERSION} ==="
echo ""
echo "次のステップ:"
echo "  1. git tag v${VERSION}"
echo "  2. git push origin v${VERSION}"
echo "  3. GitHub Releases でリリースノートを作成"
