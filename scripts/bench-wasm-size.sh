#!/bin/bash
# Wasm バイナリサイズ計測スクリプト
#
# 使い方:
#   scripts/bench-wasm-size.sh              # サイズ計測・ベースライン比較
#   scripts/bench-wasm-size.sh --save-baseline  # 結果をベースラインとして保存
#
# 出力: 各 example の Wasm バイナリサイズ (bytes)
# ベースラインがある場合は差分 (%) も表示

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
PROJECT_DIR="$(cd "$SCRIPT_DIR/.." && pwd)"
BASELINE_FILE="$SCRIPT_DIR/bench-baseline.json"
TMP_DIR=$(mktemp -d)
SAVE_BASELINE=false

if [[ "${1:-}" == "--save-baseline" ]]; then
    SAVE_BASELINE=true
fi

cleanup() {
    rm -rf "$TMP_DIR"
}
trap cleanup EXIT

echo "=== L# Wasm バイナリサイズ計測 ==="
echo ""

# ベースラインを読み込む
declare -A BASELINE
if [[ -f "$BASELINE_FILE" ]]; then
    while IFS='=' read -r key value; do
        BASELINE["$key"]="$value"
    done < <(python3 -c "
import json, sys
with open('$BASELINE_FILE') as f:
    data = json.load(f)
for k, v in data.get('wasm_sizes', {}).items():
    print(f'{k}={v}')
" 2>/dev/null || true)
fi

# 結果格納用
declare -A RESULTS
HAS_WARNING=false

printf "%-25s %10s %10s\n" "ファイル" "サイズ" "ベースライン比"
printf "%-25s %10s %10s\n" "-------------------------" "----------" "----------"

for src in "$PROJECT_DIR"/examples/*.ls; do
    name=$(basename "$src" .ls)
    wasm_path="$TMP_DIR/${name}.wasm"

    # コンパイル (エラーの場合はスキップ)
    if ! cargo run --quiet --manifest-path "$PROJECT_DIR/Cargo.toml" -- compile "$src" -o "$wasm_path" 2>/dev/null; then
        printf "%-25s %10s %10s\n" "$name.ls" "SKIP" "(コンパイルエラー)"
        continue
    fi

    if [[ ! -f "$wasm_path" ]]; then
        printf "%-25s %10s %10s\n" "$name.ls" "SKIP" "(出力なし)"
        continue
    fi

    size=$(wc -c < "$wasm_path" | tr -d ' ')
    RESULTS["$name"]=$size

    # ベースラインとの比較
    diff_str="-"
    if [[ -n "${BASELINE[$name]:-}" ]]; then
        baseline_size=${BASELINE[$name]}
        if [[ $baseline_size -gt 0 ]]; then
            diff_pct=$(python3 -c "print(f'{($size - $baseline_size) / $baseline_size * 100:+.1f}%')" 2>/dev/null || echo "N/A")

            # 10% 以上の増加で WARNING
            increase=$(python3 -c "print('YES' if ($size - $baseline_size) / $baseline_size * 100 > 10 else 'NO')" 2>/dev/null || echo "NO")
            if [[ "$increase" == "YES" ]]; then
                diff_str="$diff_pct WARNING"
                HAS_WARNING=true
            else
                diff_str="$diff_pct"
            fi
        fi
    fi

    printf "%-25s %8s B %10s\n" "$name.ls" "$size" "$diff_str"
done

echo ""

# ベースライン保存
if [[ "$SAVE_BASELINE" == "true" ]]; then
    # JSON 生成
    json="{ \"timestamp\": \"$(date -u '+%Y-%m-%dT%H:%M:%SZ')\", \"wasm_sizes\": {"
    first=true
    for name in "${!RESULTS[@]}"; do
        if [[ "$first" == "true" ]]; then
            first=false
        else
            json+=","
        fi
        json+=" \"$name\": ${RESULTS[$name]}"
    done
    json+=" } }"

    echo "$json" | python3 -m json.tool > "$BASELINE_FILE"
    echo "ベースラインを保存しました: $BASELINE_FILE"
fi

if [[ "$HAS_WARNING" == "true" ]]; then
    echo ""
    echo "WARNING: 10% 以上のサイズ増加が検出されました。"
    echo "原因を確認し、意図的な変更であればベースラインを更新してください:"
    echo "  scripts/bench-wasm-size.sh --save-baseline"
fi
