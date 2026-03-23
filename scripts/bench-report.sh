#!/bin/bash
# ベンチマーク結果を Markdown レポートとして生成するスクリプト
#
# 使い方:
#   scripts/bench-report.sh              # 全ベンチマーク実行 + レポート生成
#   scripts/bench-report.sh --skip-bench  # criterion をスキップし、それ以外を実行
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

# ========================================
# ヘルパー関数
# ========================================

# /usr/bin/time の出力から real 時間を抽出 (秒単位、単位付き)
extract_real_time() {
    local raw
    raw=$(echo "$1" | grep "real" | awk '{print $1}' || echo "")
    if [[ -n "$raw" && "$raw" != "N/A" ]]; then
        echo "${raw} s"
    else
        echo "N/A"
    fi
}

# /usr/bin/time の出力から user 時間を抽出 (秒単位、単位付き)
extract_user_time() {
    local raw
    raw=$(echo "$1" | grep "user" | awk '{print $1}' || echo "")
    if [[ -n "$raw" && "$raw" != "N/A" ]]; then
        echo "${raw} s"
    else
        echo "N/A"
    fi
}

# /usr/bin/time の出力から CPU% を抽出
extract_cpu_percent() {
    local raw
    raw=$(echo "$1" | grep "CPU" | grep -oE '[0-9]+%' | head -1 || echo "")
    if [[ -n "$raw" ]]; then
        echo "$raw"
    else
        echo "N/A"
    fi
}

# /usr/bin/time の出力から RSS (バイト数) を抽出
extract_rss() {
    echo "$1" | grep "maximum resident set size" | awk '{print $1}' || echo "0"
}

# バイト数を人間が読みやすい形式に変換 (単位付き)
format_bytes() {
    local bytes="$1"
    if [[ "$bytes" =~ ^[0-9]+$ && "$bytes" -gt 0 ]]; then
        python3 -c "
b = $bytes
if b >= 1024 * 1024:
    print(f'{b / 1024 / 1024:.1f} MB')
elif b >= 1024:
    print(f'{b / 1024:.1f} KB')
else:
    print(f'{b} B')
" 2>/dev/null || echo "$bytes B"
    else
        echo "N/A"
    fi
}

# コンパイル + 実行の計測を一括で行うヘルパー
# 引数: コマンド
# 出力: /usr/bin/time の stderr 出力全体
measure_time() {
    /usr/bin/time -l "$@" 2>&1 || true
}

# タイムスタンプ
TIMESTAMP=$(date '+%Y-%m-%d %H:%M:%S')
GIT_HASH=$(git rev-parse --short HEAD 2>/dev/null || echo "unknown")
GIT_BRANCH=$(git branch --show-current 2>/dev/null || echo "unknown")

# ========================================
# 1. criterion ベンチマーク (L# パイプライン内部)
# ========================================
CRITERION_SECTION=""
if [[ "$SKIP_BENCH" == "false" ]]; then
    echo "[1/5] criterion ベンチマーク実行中..."
    CRITERION_OUTPUT=$(cargo bench -p lsharp-wasm --bench compiler_pipeline 2>&1 || true)

    CRITERION_SECTION="### L# パイプライン内部速度 (criterion)

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
    CRITERION_SECTION="### L# パイプライン内部速度 (criterion)

> \`--skip-bench\` 指定のためスキップ。\`scripts/bench-report.sh\` を引数なしで実行してください。
"
fi

# ========================================
# 2. 言語別コンパイル速度比較
# ========================================
echo "[2/5] コンパイル速度比較中..."

# --- Rust コンパイル ---
RUST_COMPILE_TIME="N/A"; RUST_COMPILE_RSS="N/A"; RUST_COMPILE_CPU="N/A"
if command -v rustc &>/dev/null; then
    COUT=$(measure_time rustc -O "$SCRIPT_DIR/bench-programs/fib.rs" -o "$TMP_DIR/fib_rust" 2>/dev/null)
    RUST_COMPILE_TIME=$(extract_real_time "$COUT")
    RUST_COMPILE_RSS=$(extract_rss "$COUT")
    RUST_COMPILE_CPU=$(extract_cpu_percent "$COUT")
fi

# --- Go コンパイル ---
GO_COMPILE_TIME="N/A"; GO_COMPILE_RSS="N/A"; GO_COMPILE_CPU="N/A"
if command -v go &>/dev/null; then
    COUT=$(measure_time go build -o "$TMP_DIR/fib_go" "$SCRIPT_DIR/bench-programs/fib.go" 2>/dev/null)
    GO_COMPILE_TIME=$(extract_real_time "$COUT")
    GO_COMPILE_RSS=$(extract_rss "$COUT")
    GO_COMPILE_CPU=$(extract_cpu_percent "$COUT")
fi

# --- L# コンパイル ---
LSHARP_COMPILE_TIME="N/A"; LSHARP_COMPILE_RSS="N/A"; LSHARP_COMPILE_CPU="N/A"
LSHARP_WASM="$TMP_DIR/fib_lsharp.wasm"
COUT=$(/usr/bin/time -l cargo run --quiet --manifest-path "$PROJECT_DIR/Cargo.toml" -- compile "$PROJECT_DIR/examples/fib.ls" -o "$LSHARP_WASM" 2>&1 || true)
LSHARP_COMPILE_TIME=$(extract_real_time "$COUT")
LSHARP_COMPILE_RSS=$(extract_rss "$COUT")
LSHARP_COMPILE_CPU=$(extract_cpu_percent "$COUT")

# ========================================
# 3. 言語別実行速度比較
# ========================================
echo "[3/5] 実行速度比較中..."

# --- Rust 実行 ---
RUST_EXEC_TIME="N/A"; RUST_EXEC_RSS="N/A"; RUST_EXEC_CPU="N/A"; RUST_BIN_SIZE="N/A"
if [[ -f "$TMP_DIR/fib_rust" ]]; then
    RUST_BIN_SIZE=$(wc -c < "$TMP_DIR/fib_rust" | tr -d ' ')
    EOUT=$(/usr/bin/time -l "$TMP_DIR/fib_rust" 2>&1 >/dev/null || true)
    RUST_EXEC_TIME=$(extract_real_time "$EOUT")
    RUST_EXEC_RSS=$(extract_rss "$EOUT")
    RUST_EXEC_CPU=$(extract_cpu_percent "$EOUT")
fi

# --- Go 実行 ---
GO_EXEC_TIME="N/A"; GO_EXEC_RSS="N/A"; GO_EXEC_CPU="N/A"; GO_BIN_SIZE="N/A"
if [[ -f "$TMP_DIR/fib_go" ]]; then
    GO_BIN_SIZE=$(wc -c < "$TMP_DIR/fib_go" | tr -d ' ')
    EOUT=$(/usr/bin/time -l "$TMP_DIR/fib_go" 2>&1 >/dev/null || true)
    GO_EXEC_TIME=$(extract_real_time "$EOUT")
    GO_EXEC_RSS=$(extract_rss "$EOUT")
    GO_EXEC_CPU=$(extract_cpu_percent "$EOUT")
fi

# --- JavaScript 実行 ---
JS_EXEC_TIME="N/A"; JS_EXEC_RSS="N/A"; JS_EXEC_CPU="N/A"
if command -v node &>/dev/null; then
    EOUT=$(/usr/bin/time -l node "$SCRIPT_DIR/bench-programs/fib.js" 2>&1 >/dev/null || true)
    JS_EXEC_TIME=$(extract_real_time "$EOUT")
    JS_EXEC_RSS=$(extract_rss "$EOUT")
    JS_EXEC_CPU=$(extract_cpu_percent "$EOUT")
fi

# --- L# (Wasm) 実行 ---
LSHARP_EXEC_TIME="N/A"; LSHARP_EXEC_RSS="N/A"; LSHARP_EXEC_CPU="N/A"; LSHARP_BIN_SIZE="N/A"
if [[ -f "$LSHARP_WASM" ]]; then
    LSHARP_BIN_SIZE=$(wc -c < "$LSHARP_WASM" | tr -d ' ')
    if command -v wasmtime &>/dev/null; then
        EOUT=$(/usr/bin/time -l wasmtime "$LSHARP_WASM" 2>&1 >/dev/null || true)
        LSHARP_EXEC_TIME=$(extract_real_time "$EOUT")
        LSHARP_EXEC_RSS=$(extract_rss "$EOUT")
        LSHARP_EXEC_CPU=$(extract_cpu_percent "$EOUT")
    fi
fi

# ========================================
# 4. Wasm バイナリサイズ一覧
# ========================================
echo "[4/5] Wasm バイナリサイズ計測中..."

WASM_SIZE_ROWS=""
WASM_TOTAL=0
WASM_COUNT=0

for src in "$PROJECT_DIR"/examples/*.ls; do
    name=$(basename "$src" .ls)
    wasm_path="$TMP_DIR/${name}.wasm"

    if cargo run --quiet --manifest-path "$PROJECT_DIR/Cargo.toml" -- compile "$src" -o "$wasm_path" 2>/dev/null; then
        if [[ -f "$wasm_path" ]]; then
            size=$(wc -c < "$wasm_path" | tr -d ' ')
            WASM_SIZE_ROWS="${WASM_SIZE_ROWS}| \`${name}.ls\` | $(format_bytes "$size") |
"
            WASM_TOTAL=$((WASM_TOTAL + size))
            WASM_COUNT=$((WASM_COUNT + 1))
        fi
    else
        WASM_SIZE_ROWS="${WASM_SIZE_ROWS}| \`${name}.ls\` | コンパイルエラー |
"
    fi
done

if [[ $WASM_COUNT -gt 0 ]]; then
    WASM_AVG=$(python3 -c "print(f'{$WASM_TOTAL / $WASM_COUNT:.0f}')" 2>/dev/null || echo "N/A")
    WASM_AVG_FMT=$(format_bytes "$WASM_AVG")
else
    WASM_AVG_FMT="N/A"
fi

# ========================================
# 5. レポート生成
# ========================================
echo "[5/5] レポート生成中..."

EXAMPLE_COUNT=$(ls "$PROJECT_DIR"/examples/*.ls 2>/dev/null | wc -l | tr -d ' ')

cat > "$REPORT_FILE" << REPORT_EOF
# L# パフォーマンスベンチマーク レポート

> 計測日時: ${TIMESTAMP}
> Git: \`${GIT_HASH}\` (${GIT_BRANCH})
> プラットフォーム: $(uname -s) $(uname -m) ($(uname -r))

---

## サマリ

| 項目 | 値 |
|------|-----|
| L# コンパイル時間 (fib.ls) | ${LSHARP_COMPILE_TIME} |
| L# コンパイル RSS メモリ | $(format_bytes "$LSHARP_COMPILE_RSS") |
| L# Wasm 実行時間 (fib 10) | ${LSHARP_EXEC_TIME} |
| L# Wasm 実行 RSS メモリ | $(format_bytes "$LSHARP_EXEC_RSS") |
| L# Wasm 平均サイズ | ${WASM_AVG_FMT} |
| コンパイル成功数 | ${WASM_COUNT} / ${EXAMPLE_COUNT} |

---

## 言語比較 (fibonacci 35)

### コンパイル速度

| 言語 | コンパイル時間 | コンパイル RSS メモリ | CPU 使用率 |
|------|-------------|-------------------|-----------|
| Rust (\`rustc -O\`) | ${RUST_COMPILE_TIME} | $(format_bytes "$RUST_COMPILE_RSS") | ${RUST_COMPILE_CPU} |
| Go (\`go build\`) | ${GO_COMPILE_TIME} | $(format_bytes "$GO_COMPILE_RSS") | ${GO_COMPILE_CPU} |
| L# (\`lsharp compile\`) | ${LSHARP_COMPILE_TIME} | $(format_bytes "$LSHARP_COMPILE_RSS") | ${LSHARP_COMPILE_CPU} |
| JS (Node.js) | N/A (インタプリタ) | N/A | N/A |

### 実行速度

| 言語 | 実行時間 | 実行 RSS メモリ | CPU 使用率 |
|------|---------|---------------|-----------|
| Rust (ネイティブ) | ${RUST_EXEC_TIME} | $(format_bytes "$RUST_EXEC_RSS") | ${RUST_EXEC_CPU} |
| Go (ネイティブ) | ${GO_EXEC_TIME} | $(format_bytes "$GO_EXEC_RSS") | ${GO_EXEC_CPU} |
| L# (wasmtime) | ${LSHARP_EXEC_TIME} | $(format_bytes "$LSHARP_EXEC_RSS") | ${LSHARP_EXEC_CPU} |
| JS (Node.js) | ${JS_EXEC_TIME} | $(format_bytes "$JS_EXEC_RSS") | ${JS_EXEC_CPU} |

### バイナリサイズ

| 言語 | バイナリサイズ |
|------|-------------|
| Rust | $(format_bytes "$RUST_BIN_SIZE") |
| Go | $(format_bytes "$GO_BIN_SIZE") |
| L# (Wasm) | $(format_bytes "$LSHARP_BIN_SIZE") |
| JS | N/A (ソースコード実行) |

> **注**:
> - 時間は \`/usr/bin/time\` の real 時間 (壁時計時間)。単位は秒 (s)。
> - RSS メモリは maximum resident set size。単位は MB。
> - L# の実行時間は wasmtime ランタイム起動オーバーヘッドを含む。
> - L# のコンパイル時間は cargo の起動オーバーヘッドを含む (純粋なコンパイル時間は criterion を参照)。

---

## 詳細結果

${CRITERION_SECTION}

### Wasm バイナリサイズ一覧

| ファイル | サイズ |
|---------|--------|
${WASM_SIZE_ROWS}
| **平均** | **${WASM_AVG_FMT}** |

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
| コンパイル速度 | ✅ 全言語比較 | \`/usr/bin/time\` + criterion |
| 実行速度 | ✅ 全言語比較 | \`/usr/bin/time\` |
| CPU 使用率 | ✅ 全言語比較 | \`/usr/bin/time\` |
| メモリ使用量 (RSS) | ✅ 全言語比較 | \`/usr/bin/time -l\` |
| バイナリサイズ | ✅ 全言語比較 | \`wc -c\` |
| GPU 使用率 | N/A | Wasm/WASI に GPU アクセスなし |
| GC 挙動 | 将来対応 | WasmGC 統計 API 利用時 |
| DOM 操作 | 将来対応 | wasm-bindgen 導入時 |

---

*このレポートは \`scripts/bench-report.sh\` で自動生成されました。*
REPORT_EOF

echo ""
echo "レポートを生成しました: docs/BENCHMARK.md"
echo "GitHub で確認: リポジトリの docs/BENCHMARK.md を参照"
