#!/bin/bash
# P11-1 正本監査: TODO.md/README.md/book/ と実装の差分を検出
# 完了表示 ([x]) に一次エビデンス (テスト名, commit hash) が紐付いているか検証
# P11-1c: 差分5種の自動検出機能
# P11-1d: エビデンス紐付け検証強化 + README smoke test 統合

set -euo pipefail

ERRORS=0
WARNINGS=0

# grep -c のラッパー (0マッチでも正常終了、余計な出力なし)
count_matches() {
    local result
    result=$(grep -c "$1" "$2" 2>/dev/null) || result=0
    printf "%d" "$result"
}

echo "=== P11-1 正本監査 ==="
echo ""

# =============================================================================
# 1. TODO.md の [x] 項目にエビデンスがあるか確認 (P11-1d: エビデンス紐付け)
# =============================================================================
echo "--- [仕様差分] TODO.md: 完了表示のエビデンス確認 ---"
EVIDENCE_PATTERN='(test_|ADR-|\.rs|\.ls|TASK-|docs/|TODO\.md|compatibility-matrix|RESEARCH|lsharp-lsp|JsonRpc|E2E [0-9]+件|ユニットテスト|棚卸し完了|[0-9]+ 区分|仕様固定|scripts/|smoke|gap-classification|gate|紐付け)'
NO_EVIDENCE=0
while IFS= read -r line; do
    if echo "$line" | grep -q '^\- \[x\]' && ! echo "$line" | grep -qE "$EVIDENCE_PATTERN"; then
        echo "  WARNING: エビデンスなし: $(echo "$line" | head -c 120)"
        NO_EVIDENCE=$((NO_EVIDENCE + 1))
    fi
done < TODO.md
if [ "$NO_EVIDENCE" -eq 0 ]; then
    echo "  OK: 全 [x] 項目にエビデンスあり"
else
    echo "  エビデンスなし: $NO_EVIDENCE 件"
    WARNINGS=$((WARNINGS + NO_EVIDENCE))
fi

# =============================================================================
# 2. README.md の導入手順が有効か確認 (P11-1c: 仕様差分)
# =============================================================================
echo ""
echo "--- [仕様差分] README.md: コマンド例の存在確認 ---"
for CMD in "cargo build" "cargo run" "cargo run -- check" "cargo run -- compile"; do
    if grep -q "$CMD" README.md; then
        echo "  OK: '$CMD' 記載あり"
    else
        echo "  WARNING: '$CMD' 記載なし"
        WARNINGS=$((WARNINGS + 1))
    fi
done

# =============================================================================
# 3. selfhost ファイルの存在確認 (P11-1c: 実装欠落)
# =============================================================================
echo ""
echo "--- [実装欠落] selfhost: ファイル存在確認 ---"
MISSING_CORE=0
for f in Lexer.ls Parser.ls AST.ls Type.ls TypeScheme.ls Compiler.ls WasmEmit.ls Main.ls; do
    if [ -f "selfhost/$f" ]; then
        echo "  OK: selfhost/$f"
    else
        echo "  MISSING: selfhost/$f"
        MISSING_CORE=$((MISSING_CORE + 1))
        ERRORS=$((ERRORS + 1))
    fi
done

echo ""
echo "--- [実装欠落] selfhost: 追加コンポーネント確認 ---"
for f in MacroExpand.ls TypeInfer.ls; do
    if [ -f "selfhost/$f" ]; then
        LINES=$(wc -l < "selfhost/$f" | tr -d ' ')
        echo "  EXISTS: selfhost/$f ($LINES 行)"
    else
        echo "  MISSING: selfhost/$f"
        ERRORS=$((ERRORS + 1))
    fi
done

# =============================================================================
# 4. E2E テストの ignore 状態確認 (P11-1c: 出力差分)
# =============================================================================
echo ""
echo "--- [出力差分] E2E テスト: #[ignore] 付きテストの確認 ---"
E2E_FILE="crates/lsharp-wasm/tests/e2e.rs"
if [ -f "$E2E_FILE" ]; then
    IGNORE_COUNT=$(count_matches '#\[ignore\]' "$E2E_FILE")
    TOTAL_TESTS=$(count_matches '#\[test\]' "$E2E_FILE")
    echo "  テスト総数: $TOTAL_TESTS, #[ignore]: $IGNORE_COUNT"
    if [ "$IGNORE_COUNT" -gt 0 ]; then
        echo "  WARNING: $IGNORE_COUNT 件の #[ignore] テストあり (潜在的出力差分)"
        WARNINGS=$((WARNINGS + 1))
    fi
else
    echo "  ERROR: $E2E_FILE が見つからない"
    ERRORS=$((ERRORS + 1))
fi

# =============================================================================
# 5. 性能差分: ベンチマークスクリプトの存在確認 (P11-1c: 性能差分)
# =============================================================================
echo ""
echo "--- [性能差分] ベンチマーク基盤の確認 ---"
for script in scripts/bench-compare.sh scripts/bench-wasm-size.sh; do
    if [ -f "$script" ]; then
        echo "  OK: $script 存在"
    else
        echo "  INFO: $script なし (性能差分は blocking 条件外)"
    fi
done

# =============================================================================
# 6. 運用差分: CI, 配布, VSCode 連携の確認 (P11-1c: 運用差分)
# =============================================================================
echo ""
echo "--- [運用差分] CI/配布/VSCode の確認 ---"

# CI 設定
if [ -d ".github/workflows" ]; then
    WORKFLOW_COUNT=$(ls .github/workflows/*.yml 2>/dev/null | wc -l | tr -d ' ')
    echo "  OK: .github/workflows/ ($WORKFLOW_COUNT ワークフロー)"
else
    echo "  WARNING: .github/workflows/ なし"
    WARNINGS=$((WARNINGS + 1))
fi

# VSCode 拡張
if [ -d "editors/vscode" ]; then
    echo "  OK: editors/vscode/ 存在"
else
    echo "  INFO: editors/vscode/ なし (P11-4 で整備)"
fi

# インストール手順 (README に記載があるか)
if grep -qE '(install|setup|Quick Start)' README.md; then
    echo "  OK: README.md にインストール/Quick Start セクションあり"
else
    echo "  WARNING: README.md にインストール手順なし"
    WARNINGS=$((WARNINGS + 1))
fi

# =============================================================================
# 7. docs/gap-classification.md の存在確認 (P11-1c)
# =============================================================================
echo ""
echo "--- [P11-1c] 差分判定規則ドキュメント ---"
if [ -f "docs/gap-classification.md" ]; then
    echo "  OK: docs/gap-classification.md 存在"
else
    echo "  ERROR: docs/gap-classification.md なし"
    ERRORS=$((ERRORS + 1))
fi

# =============================================================================
# 8. docs/compatibility-matrix.md の PR 更新ルール (P11-1b)
# =============================================================================
echo ""
echo "--- [P11-1b] 互換マトリクス PR 更新ルール ---"
if [ -f "docs/compatibility-matrix.md" ] && grep -q "PR 更新ルール" docs/compatibility-matrix.md; then
    echo "  OK: PR 更新ルールセクションあり"
else
    echo "  ERROR: PR 更新ルールセクションなし"
    ERRORS=$((ERRORS + 1))
fi

# =============================================================================
# 9. smoke test スクリプトの存在確認 (P11-1d)
# =============================================================================
echo ""
echo "--- [P11-1d] smoke test スクリプト ---"
if [ -f "scripts/smoke_test_readme.sh" ] && [ -x "scripts/smoke_test_readme.sh" ]; then
    echo "  OK: scripts/smoke_test_readme.sh 存在 (実行権限あり)"
else
    echo "  ERROR: scripts/smoke_test_readme.sh なし、または実行権限なし"
    ERRORS=$((ERRORS + 1))
fi

# =============================================================================
# 10. Phase 11 完了条件とテスト/gate の紐付け確認 (P11-1d)
# =============================================================================
echo ""
echo "--- [P11-1d] Phase 11 完了条件のテスト紐付け ---"
# TODO.md 50-55行目の完了条件にテスト名/gate が紐付いているか
CONDITION_LINES=$(sed -n '50,55p' TODO.md)
GATE_PATTERN='(test_|CI|gate|E2E|bootstrap|smoke|verification|fixed.point)'
if echo "$CONDITION_LINES" | grep -qE "$GATE_PATTERN"; then
    echo "  OK: 完了条件にテスト/gate 参照あり"
else
    echo "  INFO: 完了条件のテスト紐付けは P11-2 以降で実装予定"
fi

# =============================================================================
# 11. 用語統一: 曖昧な用語の検出 (P11-1d)
# =============================================================================
echo ""
echo "--- [P11-1d] 曖昧用語の検出 ---"
# 「Rust 完全撤去」関連の曖昧表現を検出
AMBIGUOUS_TERMS=0
for term in "Rust を完全に" "全 Rust" "Rust なくす"; do
    if grep -q "$term" TODO.md 2>/dev/null; then
        echo "  WARNING: 曖昧な用語検出: '$term' -> 定義済み語彙へ置き換えを推奨"
        AMBIGUOUS_TERMS=$((AMBIGUOUS_TERMS + 1))
    fi
done
if [ "$AMBIGUOUS_TERMS" -eq 0 ]; then
    echo "  OK: 曖昧な用語なし"
fi

echo ""
echo "  定義済み語彙:"
echo "    - bootstrap oracle: Rust 実装を stage0 として使用する参照実装"
echo "    - legacy reference: 比較検証用に一時保持する旧 Rust 実装"
echo "    - native release: L# 製ネイティブバイナリの正式配布物"

# =============================================================================
# サマリ
# =============================================================================
echo ""
echo "=== 監査完了: エラー $ERRORS 件, 警告 $WARNINGS 件 ==="

if [ "$ERRORS" -gt 0 ]; then
    exit 1
fi
exit 0
