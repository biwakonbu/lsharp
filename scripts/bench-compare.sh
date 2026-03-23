#!/bin/bash
# 言語比較ベンチマークスクリプト
#
# fibonacci(35) を Rust / Go / JS / L# で実行し、
# 実行時間・メモリ使用量・バイナリサイズを比較する。
#
# 使い方:
#   scripts/bench-compare.sh
#
# 前提:
#   - rustc, go, node がインストール済み
#   - cargo run -- compile が動作すること
# 注: macOS デフォルト bash (3.x) 互換。連想配列は使用しない。

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
PROJECT_DIR="$(cd "$SCRIPT_DIR/.." && pwd)"
PROGRAMS_DIR="$SCRIPT_DIR/bench-programs"
TMP_DIR=$(mktemp -d)

cleanup() {
    rm -rf "$TMP_DIR"
}
trap cleanup EXIT

# /usr/bin/time の出力からデータを抽出するヘルパー
extract_real_time() {
    echo "$1" | grep "real" | awk '{print $1}' || echo "N/A"
}

extract_rss() {
    echo "$1" | grep "maximum resident set size" | awk '{print $1}' || echo "N/A"
}

bytes_to_mb() {
    local bytes="$1"
    if [[ "$bytes" != "N/A" && "$bytes" =~ ^[0-9]+$ && "$bytes" -gt 0 ]]; then
        python3 -c "print(f'{$bytes / 1024 / 1024:.1f} MB')" 2>/dev/null || echo "$bytes B"
    else
        echo "N/A"
    fi
}

bytes_to_kb() {
    local bytes="$1"
    if [[ "$bytes" != "N/A" && "$bytes" =~ ^[0-9]+$ && "$bytes" -gt 0 ]]; then
        python3 -c "print(f'{$bytes / 1024:.1f} KB')" 2>/dev/null || echo "$bytes B"
    else
        echo "N/A"
    fi
}

echo "=== L# 言語比較ベンチマーク (fibonacci 35) ==="
echo ""

# 各言語の結果を個別変数で管理
RUST_TIME="N/A"; RUST_MEM="N/A"; RUST_SIZE="N/A"
GO_TIME="N/A"; GO_MEM="N/A"; GO_SIZE="N/A"
JS_TIME="N/A"; JS_MEM="N/A"; JS_SIZE="N/A"
LSHARP_TIME="N/A"; LSHARP_MEM="N/A"; LSHARP_SIZE="N/A"

# --- Rust ---
echo "[1/4] Rust をコンパイル・実行中..."
if command -v rustc &>/dev/null; then
    rustc -O "$PROGRAMS_DIR/fib.rs" -o "$TMP_DIR/fib_rust" 2>/dev/null
    RUST_SIZE=$(wc -c < "$TMP_DIR/fib_rust" | tr -d ' ')

    TIME_OUTPUT=$(/usr/bin/time -l "$TMP_DIR/fib_rust" 2>&1 >/dev/null || true)
    RUST_TIME=$(extract_real_time "$TIME_OUTPUT")
    RUST_MEM=$(extract_rss "$TIME_OUTPUT")
else
    echo "  rustc が見つかりません。スキップ。"
fi

# --- Go ---
echo "[2/4] Go をコンパイル・実行中..."
if command -v go &>/dev/null; then
    go build -o "$TMP_DIR/fib_go" "$PROGRAMS_DIR/fib.go" 2>/dev/null
    GO_SIZE=$(wc -c < "$TMP_DIR/fib_go" | tr -d ' ')

    TIME_OUTPUT=$(/usr/bin/time -l "$TMP_DIR/fib_go" 2>&1 >/dev/null || true)
    GO_TIME=$(extract_real_time "$TIME_OUTPUT")
    GO_MEM=$(extract_rss "$TIME_OUTPUT")
else
    echo "  go が見つかりません。スキップ。"
fi

# --- JavaScript (Node.js) ---
echo "[3/4] JavaScript (Node.js) を実行中..."
if command -v node &>/dev/null; then
    TIME_OUTPUT=$(/usr/bin/time -l node "$PROGRAMS_DIR/fib.js" 2>&1 >/dev/null || true)
    JS_TIME=$(extract_real_time "$TIME_OUTPUT")
    JS_MEM=$(extract_rss "$TIME_OUTPUT")
else
    echo "  node が見つかりません。スキップ。"
fi

# --- L# (Wasm via wasmtime) ---
echo "[4/4] L# をコンパイル・実行中..."
LSHARP_WASM="$TMP_DIR/fib_lsharp.wasm"

cargo run --quiet --manifest-path "$PROJECT_DIR/Cargo.toml" -- compile "$PROJECT_DIR/examples/fib.ls" -o "$LSHARP_WASM" 2>/dev/null || true

if [[ -f "$LSHARP_WASM" ]]; then
    LSHARP_SIZE=$(wc -c < "$LSHARP_WASM" | tr -d ' ')

    if command -v wasmtime &>/dev/null; then
        TIME_OUTPUT=$(/usr/bin/time -l wasmtime "$LSHARP_WASM" 2>&1 >/dev/null || true)
        LSHARP_TIME=$(extract_real_time "$TIME_OUTPUT")
        LSHARP_MEM=$(extract_rss "$TIME_OUTPUT")
    else
        echo "  wasmtime が見つかりません。実行スキップ。"
    fi
else
    echo "  L# コンパイルに失敗しました。"
fi

# --- 結果表示 ---
echo ""
echo "=== 結果 ==="
echo ""
printf "%-12s %15s %15s %15s\n" "言語" "実行時間" "RSS メモリ" "バイナリサイズ"
printf "%-12s %15s %15s %15s\n" "------------" "---------------" "---------------" "---------------"
printf "%-12s %15s %15s %15s\n" "Rust" "$RUST_TIME" "$(bytes_to_mb "$RUST_MEM")" "$(bytes_to_kb "$RUST_SIZE")"
printf "%-12s %15s %15s %15s\n" "Go" "$GO_TIME" "$(bytes_to_mb "$GO_MEM")" "$(bytes_to_kb "$GO_SIZE")"
printf "%-12s %15s %15s %15s\n" "JS (Node)" "$JS_TIME" "$(bytes_to_mb "$JS_MEM")" "N/A"
printf "%-12s %15s %15s %15s\n" "L# (Wasm)" "$LSHARP_TIME" "$(bytes_to_mb "$LSHARP_MEM")" "$(bytes_to_kb "$LSHARP_SIZE")"

echo ""
echo "注: 実行時間は /usr/bin/time の real 時間。"
echo "    L# は wasmtime 経由の実行のため、ランタイム起動オーバーヘッドを含む。"
echo "    hyperfine がインストールされている場合はより正確な計測が可能です。"
