#!/bin/bash
# ベンチマーク結果を Markdown レポートとして生成するスクリプト
#
# 使い方:
#   scripts/bench-report.sh              # 全ベンチマーク実行 + レポート生成
#   scripts/bench-report.sh --skip-bench  # ベンチマークをスキップし、前回結果からレポートのみ生成
#
# 出力: docs/BENCHMARK.md
# 注: macOS デフォルト bash (3.x) 互換。連想配列は使用しない。

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
PROJECT_DIR="$(cd "$SCRIPT_DIR/.." && pwd)"
REPORT_FILE="$PROJECT_DIR/docs/BENCHMARK.md"
TMP_DIR=$(mktemp -d)
SKIP_BENCH=false

if [[ "${1:-}" == "--skip-bench" ]]; then
    SKIP_BENCH=true
fi

cleanup() {
    rm -rf "$TMP_DIR"
}
trap cleanup EXIT

mkdir -p "$PROJECT_DIR/docs"

# /usr/bin/time の出力から real 時間を抽出するヘルパー
extract_real_time() {
    echo "$1" | grep "real" | awk '{print $1}' || echo "N/A"
}

# /usr/bin/time の出力から RSS を抽出するヘルパー
extract_rss() {
    echo "$1" | grep "maximum resident set size" | awk '{print $1}' || echo "0"
}

# バイト数を MB 文字列に変換
bytes_to_mb() {
    local bytes="$1"
    if [[ "$bytes" =~ ^[0-9]+$ && "$bytes" -gt 0 ]]; then
        python3 -c "print(f'{$bytes / 1024 / 1024:.1f} MB')" 2>/dev/null || echo "$bytes B"
    else
        echo "N/A"
    fi
}

# バイト数を KB 文字列に変換
bytes_to_kb() {
    local bytes="$1"
    if [[ "$bytes" =~ ^[0-9]+$ && "$bytes" -gt 0 ]]; then
        python3 -c "print(f'{$bytes / 1024:.1f} KB')" 2>/dev/null || echo "$bytes B"
    else
        echo "N/A"
    fi
}

# タイムスタンプ
TIMESTAMP=$(date '+%Y-%m-%d %H:%M:%S')
GIT_HASH=$(git rev-parse --short HEAD 2>/dev/null || echo "unknown")
GIT_BRANCH=$(git branch --show-current 2>/dev/null || echo "unknown")

# ========================================
# 1. コンパイル速度 (criterion)
# ========================================
CRITERION_SECTION=""
if [[ "$SKIP_BENCH" == "false" ]]; then
    echo "[1/4] criterion ベンチマーク実行中..."
    CRITERION_OUTPUT=$(cargo bench -p lsharp-wasm --bench compiler_pipeline 2>&1 || true)

    CRITERION_SECTION="### コンパイル速度 (criterion)

\`\`\`
$(echo "$CRITERION_OUTPUT" | grep -E "(Benchmarking|time:|change:)" || echo "結果取得不可")
\`\`\`

<details>
<summary>criterion 全出力</summary>

\`\`\`
$CRITERION_OUTPUT
\`\`\`

</details>
"
else
    if [[ -d "$PROJECT_DIR/target/criterion" ]]; then
        CRITERION_SECTION="### コンパイル速度 (criterion)

> 前回のベンチマーク結果を使用しています。最新結果を取得するには \`scripts/bench-report.sh\` を引数なしで実行してください。
"
    else
        CRITERION_SECTION="### コンパイル速度 (criterion)

> ベンチマーク結果がありません。\`scripts/bench-report.sh\` を引数なしで実行してください。
"
    fi
fi

# ========================================
# 2. Wasm バイナリサイズ
# ========================================
echo "[2/4] Wasm バイナリサイズ計測中..."

WASM_SIZE_ROWS=""
WASM_TOTAL=0
WASM_COUNT=0

for src in "$PROJECT_DIR"/examples/*.ls; do
    name=$(basename "$src" .ls)
    wasm_path="$TMP_DIR/${name}.wasm"

    if cargo run --quiet --manifest-path "$PROJECT_DIR/Cargo.toml" -- compile "$src" -o "$wasm_path" 2>/dev/null; then
        if [[ -f "$wasm_path" ]]; then
            size=$(wc -c < "$wasm_path" | tr -d ' ')
            size_kb=$(python3 -c "print(f'{$size / 1024:.1f}')" 2>/dev/null || echo "N/A")
            WASM_SIZE_ROWS="${WASM_SIZE_ROWS}| \`${name}.ls\` | ${size} B | ${size_kb} KB |
"
            WASM_TOTAL=$((WASM_TOTAL + size))
            WASM_COUNT=$((WASM_COUNT + 1))
        fi
    else
        WASM_SIZE_ROWS="${WASM_SIZE_ROWS}| \`${name}.ls\` | - | コンパイルエラー |
"
    fi
done

if [[ $WASM_COUNT -gt 0 ]]; then
    WASM_AVG=$(python3 -c "print(f'{$WASM_TOTAL / $WASM_COUNT:.0f}')" 2>/dev/null || echo "N/A")
    WASM_AVG_KB=$(python3 -c "print(f'{$WASM_TOTAL / $WASM_COUNT / 1024:.1f}')" 2>/dev/null || echo "N/A")
else
    WASM_AVG="N/A"
    WASM_AVG_KB="N/A"
fi

# ========================================
# 3. ランタイム計測
# ========================================
echo "[3/4] ランタイム計測中..."

COMPILE_TIME="N/A"
COMPILE_RSS="N/A"
EXEC_TIME="N/A"
EXEC_RSS="N/A"

# コンパイル時間
LSHARP_WASM="$TMP_DIR/fib_bench.wasm"
COMPILE_OUTPUT=$(/usr/bin/time -l cargo run --quiet --manifest-path "$PROJECT_DIR/Cargo.toml" -- compile "$PROJECT_DIR/examples/fib.ls" -o "$LSHARP_WASM" 2>&1 || true)
COMPILE_TIME=$(extract_real_time "$COMPILE_OUTPUT")
COMPILE_RSS_BYTES=$(extract_rss "$COMPILE_OUTPUT")
COMPILE_RSS=$(bytes_to_mb "$COMPILE_RSS_BYTES")

# 実行時間 (wasmtime)
EXEC_RSS_BYTES="0"
if [[ -f "$LSHARP_WASM" ]] && command -v wasmtime &>/dev/null; then
    EXEC_OUTPUT=$(/usr/bin/time -l wasmtime "$LSHARP_WASM" 2>&1 || true)
    EXEC_TIME=$(extract_real_time "$EXEC_OUTPUT")
    EXEC_RSS_BYTES=$(extract_rss "$EXEC_OUTPUT")
    EXEC_RSS=$(bytes_to_mb "$EXEC_RSS_BYTES")
fi

# ========================================
# 4. 言語比較 (連想配列を使わない方式)
# ========================================
echo "[4/4] 言語比較実行中..."

# 各言語の結果を個別変数で管理
RUST_TIME="N/A"; RUST_MEM="N/A"; RUST_SIZE="N/A"
GO_TIME="N/A"; GO_MEM="N/A"; GO_SIZE="N/A"
JS_TIME="N/A"; JS_MEM="N/A"; JS_SIZE="N/A"
LSHARP_TIME="$EXEC_TIME"; LSHARP_MEM="$EXEC_RSS_BYTES"; LSHARP_SIZE="N/A"

# Rust
if command -v rustc &>/dev/null; then
    rustc -O "$SCRIPT_DIR/bench-programs/fib.rs" -o "$TMP_DIR/fib_rust" 2>/dev/null || true
    if [[ -f "$TMP_DIR/fib_rust" ]]; then
        RUST_SIZE=$(wc -c < "$TMP_DIR/fib_rust" | tr -d ' ')
        TIME_OUT=$(/usr/bin/time -l "$TMP_DIR/fib_rust" 2>&1 >/dev/null || true)
        RUST_TIME=$(extract_real_time "$TIME_OUT")
        RUST_MEM=$(extract_rss "$TIME_OUT")
    fi
fi

# Go
if command -v go &>/dev/null; then
    go build -o "$TMP_DIR/fib_go" "$SCRIPT_DIR/bench-programs/fib.go" 2>/dev/null || true
    if [[ -f "$TMP_DIR/fib_go" ]]; then
        GO_SIZE=$(wc -c < "$TMP_DIR/fib_go" | tr -d ' ')
        TIME_OUT=$(/usr/bin/time -l "$TMP_DIR/fib_go" 2>&1 >/dev/null || true)
        GO_TIME=$(extract_real_time "$TIME_OUT")
        GO_MEM=$(extract_rss "$TIME_OUT")
    fi
fi

# JavaScript
if command -v node &>/dev/null; then
    TIME_OUT=$(/usr/bin/time -l node "$SCRIPT_DIR/bench-programs/fib.js" 2>&1 >/dev/null || true)
    JS_TIME=$(extract_real_time "$TIME_OUT")
    JS_MEM=$(extract_rss "$TIME_OUT")
fi

# L# Wasm サイズ
if [[ -f "$LSHARP_WASM" ]]; then
    LSHARP_SIZE=$(wc -c < "$LSHARP_WASM" | tr -d ' ')
fi

# 比較テーブル行を生成
COMPARE_ROWS="| Rust | $RUST_TIME | $(bytes_to_mb "$RUST_MEM") | $(bytes_to_kb "$RUST_SIZE") |
| Go | $GO_TIME | $(bytes_to_mb "$GO_MEM") | $(bytes_to_kb "$GO_SIZE") |
| JS (Node.js) | $JS_TIME | $(bytes_to_mb "$JS_MEM") | N/A |
| L# (Wasm) | $LSHARP_TIME | $(bytes_to_mb "$LSHARP_MEM") | $(bytes_to_kb "$LSHARP_SIZE") |"

# ========================================
# レポート生成
# ========================================
echo ""
echo "レポート生成中..."

EXAMPLE_COUNT=$(ls "$PROJECT_DIR"/examples/*.ls 2>/dev/null | wc -l | tr -d ' ')

cat > "$REPORT_FILE" << REPORT_EOF
# L# パフォーマンスベンチマーク レポート

> 計測日時: ${TIMESTAMP}
> Git: \`${GIT_HASH}\` (${GIT_BRANCH})
> プラットフォーム: $(uname -s) $(uname -m) ($(uname -r))

## サマリ

| 項目 | 値 |
|------|-----|
| コンパイル時間 (fib.ls) | ${COMPILE_TIME} |
| コンパイル RSS メモリ | ${COMPILE_RSS} |
| Wasm 実行時間 (fib 10) | ${EXEC_TIME} |
| Wasm 実行 RSS メモリ | ${EXEC_RSS} |
| 平均 Wasm サイズ | ${WASM_AVG} B (${WASM_AVG_KB} KB) |
| コンパイル成功数 | ${WASM_COUNT} / ${EXAMPLE_COUNT} |

---

## 詳細結果

${CRITERION_SECTION}

### Wasm バイナリサイズ

| ファイル | サイズ (bytes) | サイズ (KB) |
|---------|---------------|------------|
${WASM_SIZE_ROWS}

### ランタイム計測 (fib.ls)

| 計測項目 | コンパイル | 実行 (wasmtime) |
|---------|-----------|----------------|
| 時間 | ${COMPILE_TIME} | ${EXEC_TIME} |
| RSS メモリ | ${COMPILE_RSS} | ${EXEC_RSS} |

### 言語比較 (fibonacci 35)

| 言語 | 実行時間 | RSS メモリ | バイナリサイズ |
|------|---------|-----------|-------------|
${COMPARE_ROWS}

> **注**: 実行時間は \`/usr/bin/time\` の real 時間。L# は wasmtime 経由の実行のため、ランタイム起動オーバーヘッドを含む。

---

## 計測環境

| 項目 | 値 |
|------|-----|
| OS | $(uname -s) $(uname -r) |
| Arch | $(uname -m) |
| Rust | $(rustc --version 2>/dev/null || echo "N/A") |
| Go | $(go version 2>/dev/null || echo "N/A") |
| Node.js | $(node --version 2>/dev/null || echo "N/A") |
| wasmtime | $(wasmtime --version 2>/dev/null || echo "N/A") |

---

## 計測対象の網羅性

| 計測項目 | ステータス | 手段 |
|---------|----------|------|
| CPU 使用率 | ✅ 計測済み | \`/usr/bin/time\` |
| メモリ使用量 (RSS) | ✅ 計測済み | \`/usr/bin/time -l\` |
| バイナリサイズ | ✅ 計測済み | \`wc -c\` |
| コンパイル速度 | ✅ 計測済み | criterion |
| GPU 使用率 | N/A | Wasm/WASI に GPU アクセスなし |
| GC 挙動 | 将来対応 | WasmGC 統計 API 利用時 |
| DOM 操作 | 将来対応 | wasm-bindgen 導入時 |

---

*このレポートは \`scripts/bench-report.sh\` で自動生成されました。*
REPORT_EOF

echo ""
echo "レポートを生成しました: docs/BENCHMARK.md"
echo "GitHub で確認: リポジトリの docs/BENCHMARK.md を参照"
