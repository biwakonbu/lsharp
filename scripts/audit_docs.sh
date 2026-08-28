#!/bin/bash
# docs 正本監査: TODO.md/ISSUES.md/README.md/book/ と実装の差分を検出
# TODO.md は active-only とし、完了 evidence は ADR・仕様・運用記録で保持する
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

echo "=== docs 正本監査 ==="
echo ""

# =============================================================================
# 1. TODO.md の active-only 契約と ISSUES.md の未完項目反映
# =============================================================================
echo "--- [仕様差分] TODO.md: active-only backlog 契約 ---"
COMPLETED_TODO_COUNT=$(grep -cE '^[[:space:]]*-[[:space:]]+\[x\]' TODO.md 2>/dev/null) || COMPLETED_TODO_COUNT=0
if [ "$COMPLETED_TODO_COUNT" -eq 0 ]; then
    echo "  OK: TODO.md に完了済み [x] 項目なし"
else
    echo "  ERROR: TODO.md に完了済み [x] 項目が $COMPLETED_TODO_COUNT 件残っている"
    ERRORS=$((ERRORS + COMPLETED_TODO_COUNT))
fi

ACTIVE_ISSUE_IDS=$(awk -F'|' '
function trim(value) {
    gsub(/^[ \t]+|[ \t]+$/, "", value)
    return value
}
/^\| \[[DI][O0-9C-]*\]/ {
    issue = trim($2)
    status = trim($5)
    if (status == "open" || status == "in-design" || status == "documented-limitation") {
        sub(/^\[/, "", issue)
        sub(/\].*$/, "", issue)
        print issue
    }
}
' ISSUES.md)
MISSING_ACTIVE_ISSUES=0
while IFS= read -r issue_id; do
    [ -n "$issue_id" ] || continue
    if grep -qF "\`$issue_id\`" TODO.md; then
        echo "  OK: active issue $issue_id は TODO.md の aggregate に反映済み"
    else
        echo "  ERROR: active issue $issue_id が TODO.md に見つからない"
        MISSING_ACTIVE_ISSUES=$((MISSING_ACTIVE_ISSUES + 1))
    fi
done <<< "$ACTIVE_ISSUE_IDS"
if [ -z "$ACTIVE_ISSUE_IDS" ]; then
    echo "  ERROR: ISSUES.md から active issue を抽出できない"
    ERRORS=$((ERRORS + 1))
elif [ "$MISSING_ACTIVE_ISSUES" -gt 0 ]; then
    ERRORS=$((ERRORS + MISSING_ACTIVE_ISSUES))
fi

# =============================================================================
# 1b. current milestone と TODO.md の整合性
# =============================================================================
echo ""
echo "--- [仕様差分] v0.2 Milestone 2 の active task 同期 ---"
M2_PLAN="docs/development/planning/v0.2-milestone-02.md"
for TASK_ID in EC-M2-01 EC-M2-02 EC-M2-03; do
    if grep -qF "\`$TASK_ID\`" TODO.md && grep -qF "$TASK_ID" "$M2_PLAN"; then
        echo "  OK: $TASK_ID は TODO.md と Milestone 2 plan に存在"
    else
        echo "  ERROR: $TASK_ID を TODO.md と $M2_PLAN の両方へ記載すること"
        ERRORS=$((ERRORS + 1))
    fi
done

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
# 1d. branch protection / required check 契約の整合性
# =============================================================================
echo ""
echo "--- [運用差分] branch protection / required check 契約 ---"
REQUIRED_CHECK_JOB_ID="ci-gate-v2"
REQUIRED_CHECK_DISPLAY_NAME="CI Gate v2"
CI_DOC_FILE="docs/development/operations/CI.md"
BRANCH_PROTECTION_FILE="docs/development/operations/branch-protection-checklist.md"
CI_GRAPH_FILE="docs/development/operations/ci-gate-v2-job-graph.md"

if grep -qE "^  ${REQUIRED_CHECK_JOB_ID}:" .github/workflows/ci.yml; then
    echo "  OK: ci.yml に required check の job id (${REQUIRED_CHECK_JOB_ID}) あり"
else
    echo "  ERROR: ci.yml に required check の job id (${REQUIRED_CHECK_JOB_ID}) が見つからない"
    ERRORS=$((ERRORS + 1))
fi

if grep -q "name: ${REQUIRED_CHECK_DISPLAY_NAME}" .github/workflows/ci.yml; then
    echo "  OK: ci.yml に required check の Actions 表示名 (${REQUIRED_CHECK_DISPLAY_NAME}) あり"
else
    echo "  ERROR: ci.yml に required check の Actions 表示名 (${REQUIRED_CHECK_DISPLAY_NAME}) が見つからない"
    ERRORS=$((ERRORS + 1))
fi

for DOC in "$CI_DOC_FILE" "$BRANCH_PROTECTION_FILE" "$CI_GRAPH_FILE"; do
    if grep -q "$REQUIRED_CHECK_JOB_ID" "$DOC" && grep -q "$REQUIRED_CHECK_DISPLAY_NAME" "$DOC"; then
        echo "  OK: $DOC に required check の job id / 表示名の対応あり"
    else
        echo "  ERROR: $DOC に required check の job id (${REQUIRED_CHECK_JOB_ID}) と Actions 表示名 (${REQUIRED_CHECK_DISPLAY_NAME}) の両方が必要"
        ERRORS=$((ERRORS + 1))
    fi
done

# =============================================================================
# 2. 公開 CLI ドキュメントが compile 中心か確認 (P12-0)
# =============================================================================
echo ""
echo "--- [P12-0] 公開 CLI ドキュメントの compile 統一確認 ---"
for SPEC in \
    "README.md|lsharp compile|README compile 導線" \
    "README.md|checksums.txt|README checksum 導線" \
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
for f in \
    selfhost/src/Syntax/Lexer.ls \
    selfhost/src/Syntax/Parser.ls \
    selfhost/src/Syntax/AST.ls \
    selfhost/src/Types/Type.ls \
    selfhost/src/Types/TypeScheme.ls \
    selfhost/src/Backend/Wasm/Compiler.ls \
    selfhost/src/Backend/Wasm/WasmEmit.ls \
    selfhost/src/App/Main.ls
do
    if [ -f "$f" ]; then
        echo "  OK: $f"
    else
        echo "  MISSING: $f"
        MISSING_CORE=$((MISSING_CORE + 1))
        ERRORS=$((ERRORS + 1))
    fi
done

echo ""
echo "--- [実装欠落] selfhost: 追加コンポーネント確認 ---"
for f in selfhost/src/Syntax/MacroExpand.ls selfhost/src/Types/TypeInfer.ls; do
    if [ -f "$f" ]; then
        LINES=$(wc -l < "$f" | tr -d ' ')
        echo "  EXISTS: $f ($LINES 行)"
    else
        echo "  MISSING: $f"
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

# Documentation site
if [ -f "docs/site.toml" ] &&
   grep -q 'source = "docs/site.toml"' docs/site.toml &&
   grep -q 'docs/guides/quick-start.md' docs/site.toml &&
   grep -q 'book/ch01-introduction.md' docs/site.toml &&
   grep -q 'docs/development/operations/documentation-site.md' docs/site.toml; then
    echo "  OK: docs/site.toml が公開ドキュメントサイトの SSOT として存在"
else
    echo "  ERROR: docs/site.toml に必須ページまたは source 宣言が不足"
    ERRORS=$((ERRORS + 1))
fi

if [ -x "scripts/ci/build-doc-site.sh" ]; then
    echo "  OK: scripts/ci/build-doc-site.sh 存在 (実行権限あり)"
else
    echo "  ERROR: scripts/ci/build-doc-site.sh なし、または実行権限なし"
    ERRORS=$((ERRORS + 1))
fi

if [ -f ".github/workflows/docs.yml" ] &&
   grep -q 'actions/upload-pages-artifact@v4' .github/workflows/docs.yml &&
   grep -q 'actions/deploy-pages@v4' .github/workflows/docs.yml; then
    echo "  OK: Docs Site workflow が GitHub Pages artifact / deploy を使用"
else
    echo "  ERROR: .github/workflows/docs.yml の GitHub Pages 公開設定が不足"
    ERRORS=$((ERRORS + 1))
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
# 10. current milestone 完了条件とテスト/gate の紐付け確認
# =============================================================================
echo ""
echo "--- [仕様差分] current milestone の gate 紐付け ---"
CONDITION_LINES=$(sed -n '/^## Gate/,$p' "$M2_PLAN")
GATE_PATTERN='(test_|gate|E2E|bootstrap|smoke|verification|fixed.point|parser.*graph.*validate|Mac Apple Silicon|Linux x86_64)'
if echo "$CONDITION_LINES" | grep -qE "$GATE_PATTERN"; then
    echo "  OK: Milestone 2 の完了条件に runtime/target gate 参照あり"
else
    echo "  ERROR: $M2_PLAN の Gate 節に runtime/target gate 参照がない"
    ERRORS=$((ERRORS + 1))
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
echo "    - host launcher: Wasmtime と guest component を束ねる正式配布用バイナリ"
echo '    - guest component: host launcher に埋め込む `.component.wasm` の正本成果物'
echo "    - single binary distribution: host launcher + embedded guest component + stdlib の配布形態"

echo ""
echo "--- [I-104] completion-criteria が赤い test を達成根拠に名指ししていないか ---"
# completion-criteria.md の `test_...` と ignored lane 台帳の ::test_... の積を取る。
# 積が空でなければ ERROR。例外は同一行の `[赤: <引き取り先>]` 注記でのみ認める
# (裁定は docs/adr/decisions-completion-criteria-red-citation.md)。
CC_DOC="docs/development/planning/completion-criteria.md"
LANE_LEDGER="docs/development/validation/ignored-lane-expected-failures.txt"
if [ ! -f "$CC_DOC" ] || [ ! -f "$LANE_LEDGER" ]; then
    echo "  ERROR: 照合対象の file が見つからない ($CC_DOC / $LANE_LEDGER)"
    ERRORS=$((ERRORS + 1))
else
    RED_CITATIONS=$(awk -v ledger="$LANE_LEDGER" '
BEGIN {
    while ((getline line < ledger) > 0) {
        rest = line
        while (match(rest, /::test_[a-z0-9_]+/)) {
            name = substr(rest, RSTART + 2, RLENGTH - 2)
            red[name] = 1
            rest = substr(rest, RSTART + RLENGTH)
        }
    }
    close(ledger)
}
{
    # 引き取り先つきで赤と明記している行は例外
    if ($0 ~ /\[赤: [^]]+\]/) next
    rest = $0
    while (match(rest, /`test_[a-z0-9_]+`/)) {
        name = substr(rest, RSTART + 1, RLENGTH - 2)
        if (name in red) print FILENAME ":" FNR ": " name
        rest = substr(rest, RSTART + RLENGTH)
    }
}
' "$CC_DOC")
    if [ -z "$RED_CITATIONS" ]; then
        echo "  OK: 赤い test を無注記で名指ししている箇所なし"
    else
        RED_COUNT=$(printf "%s\n" "$RED_CITATIONS" | wc -l | tr -d " ")
        echo "  ERROR: 期待 FAIL の test を達成根拠に名指ししている箇所が $RED_COUNT 件"
        printf "%s\n" "$RED_CITATIONS" | sed "s/^/    /"
        echo "    -> 実装未達なら gate の状態マーカーを戻す。誤引用なら名指しを外す。"
        echo "       赤と分かったうえで名指しするなら同一行に [赤: <引き取り先>] を書く"
        ERRORS=$((ERRORS + RED_COUNT))
    fi
fi

# =============================================================================
# サマリ
# =============================================================================
echo ""
echo "=== 監査完了: エラー $ERRORS 件, 警告 $WARNINGS 件 ==="

if [ "$ERRORS" -gt 0 ]; then
    exit 1
fi
exit 0
