#!/bin/bash
# P11-1 正本監査: TODO.md/README.md/book/ と実装の差分を検出
# 完了表示 ([x]) に一次エビデンス (テスト名, commit hash) が紐付いているか検証
# P11-1c: 差分5種の自動検出機能
# P11-1d: エビデンス紐付け検証強化 + README smoke test 統合
# P12-0: 公開 CLI の docs/script を compile 中心に統一しているか確認

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
        echo "  ERROR: エビデンスなし: $(echo "$line" | head -c 120)"
        NO_EVIDENCE=$((NO_EVIDENCE + 1))
    fi
done < TODO.md
if [ "$NO_EVIDENCE" -eq 0 ]; then
    echo "  OK: 全 [x] 項目にエビデンスあり"
else
    echo "  エビデンスなし: $NO_EVIDENCE 件"
    ERRORS=$((ERRORS + NO_EVIDENCE))
fi

# =============================================================================
# 1b. Phase 11 完了主張と completion criteria の整合性
# =============================================================================
echo ""
echo "--- [仕様差分] Phase 11 完了主張の整合性確認 ---"
P11_CLAIM_COMPLETE=0
if grep -qE '完了済みフェーズ.*P11|Phase 11 全[0-9]+タスク完了|Phase 11 実装完了 ADR' TODO.md; then
    P11_CLAIM_COMPLETE=1
fi
P11_PENDING_COUNT=$(grep -cE '\[(pending|in-progress)\]' docs/development/planning/completion-criteria.md 2>/dev/null) || P11_PENDING_COUNT=0
if [ "$P11_CLAIM_COMPLETE" -eq 1 ] && [ "$P11_PENDING_COUNT" -gt 0 ]; then
    echo "  ERROR: TODO.md が Phase 11 完了を主張しているが、docs/development/planning/completion-criteria.md に pending/in-progress が $P11_PENDING_COUNT 件残っている"
    ERRORS=$((ERRORS + 1))
else
    echo "  OK: TODO.md の Phase 11 状態表示と completion criteria に矛盾なし"
fi

# =============================================================================
# 1c. compatibility-matrix の active row が証跡を持つか確認
# =============================================================================
echo ""
echo "--- [仕様差分] compatibility-matrix: active row の証跡確認 ---"
ACTIVE_ROW_GAPS=$(awk -F'|' '
function trim(value) {
    gsub(/^[ \t]+|[ \t]+$/, "", value)
    return value
}
/^\|/ {
    feature = trim($2)
    rust = trim($3)
    lsharp = trim($4)
    evidence = trim($8)
    if (feature == "" || feature == "Feature" || feature ~ /^-+$/) {
        next
    }
    if (rust != "" && rust != "-" && lsharp != "" && lsharp != "-" &&
        lsharp != "なし" && lsharp != "設計のみ" && evidence == "-") {
        count += 1
    }
}
END {
    print count + 0
}
' docs/development/planning/compatibility-matrix.md)
if [ "$ACTIVE_ROW_GAPS" -eq 0 ]; then
    echo "  OK: active row に evidence 欠落なし"
else
    echo "  ERROR: evidence が空の active row が $ACTIVE_ROW_GAPS 件ある"
    ERRORS=$((ERRORS + ACTIVE_ROW_GAPS))
fi

# =============================================================================
# 2. 公開 CLI ドキュメントが compile 中心か確認 (P12-0)
# =============================================================================
echo ""
echo "--- [P12-0] 公開 CLI ドキュメントの compile 統一確認 ---"
for SPEC in \
    "README.md|target/debug/lsharp compile|README compile 導線" \
    "README.md|LSP / MCP|README の内部 API 説明" \
    "AGENTS.md|cargo run -- compile|AGENTS compile 導線" \
    "AGENTS.md|LSP / MCP|AGENTS の内部 API 説明" \
    "CLAUDE.md|cargo run -- compile|CLAUDE compile 導線" \
    "CLAUDE.md|LSP / MCP|CLAUDE の内部 API 説明"; do
    FILE=$(echo "$SPEC" | cut -d'|' -f1)
    PATTERN=$(echo "$SPEC" | cut -d'|' -f2)
    LABEL=$(echo "$SPEC" | cut -d'|' -f3)
    if grep -q "$PATTERN" "$FILE"; then
        echo "  OK: $LABEL"
    else
        echo "  WARNING: $LABEL が見つからない"
        WARNINGS=$((WARNINGS + 1))
    fi
done

echo ""
echo "--- [P12-0] 非推奨の公開 CLI 例が残っていないか確認 ---"
DEPRECATED_PUBLIC_DOCS=0
DEPRECATED_PATTERN='(cargo run -- (parse|check|fmt)|target/debug/lsharp (parse|check|fmt))'
for FILE in README.md AGENTS.md CLAUDE.md book/ch03-parser.md book/ch04-type-inference.md scripts/smoke_test_readme.sh; do
    if grep -qE "$DEPRECATED_PATTERN" "$FILE"; then
        echo "  ERROR: 非推奨の公開 CLI 例が残存: $FILE"
        grep -nE "$DEPRECATED_PATTERN" "$FILE" | sed 's/^/    /'
        DEPRECATED_PUBLIC_DOCS=$((DEPRECATED_PUBLIC_DOCS + 1))
    fi
done
if [ "$DEPRECATED_PUBLIC_DOCS" -eq 0 ]; then
    echo "  OK: 直接影響する docs/scripts から旧 CLI 例を除去済み"
else
    ERRORS=$((ERRORS + DEPRECATED_PUBLIC_DOCS))
fi

if grep -q '/tmp/' scripts/smoke_test_readme.sh; then
    echo "  ERROR: scripts/smoke_test_readme.sh に /tmp 依存が残っている"
    ERRORS=$((ERRORS + 1))
else
    echo "  OK: scripts/smoke_test_readme.sh に /tmp 依存なし"
fi

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
# 7. docs/development/planning/gap-classification.md の存在確認 (P11-1c)
# =============================================================================
echo ""
echo "--- [P11-1c] 差分判定規則ドキュメント ---"
if [ -f "docs/development/planning/gap-classification.md" ]; then
    echo "  OK: docs/development/planning/gap-classification.md 存在"
else
    echo "  ERROR: docs/development/planning/gap-classification.md なし"
    ERRORS=$((ERRORS + 1))
fi

# =============================================================================
# 8. docs/development/planning/compatibility-matrix.md の PR 更新ルール (P11-1b)
# =============================================================================
echo ""
echo "--- [P11-1b] 互換マトリクス PR 更新ルール ---"
if [ -f "docs/development/planning/compatibility-matrix.md" ] && grep -q "PR 更新ルール" docs/development/planning/compatibility-matrix.md; then
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
