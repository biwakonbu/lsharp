#!/bin/bash
# 言語比較ベンチマークスクリプト
#
# fibonacci(35) を Rust / Go / JS / L# で実行し、
# コンパイル時間・実行時間・メモリ使用量・バイナリサイズを比較する。
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

# ヘルパー
extract_real_time() {
    local raw
    raw=$(echo "$1" | grep "real" | awk '{print $1}' || echo "")
    if [[ -n "$raw" ]]; then echo "${raw} s"; else echo "N/A"; fi
}

extract_rss() {
    echo "$1" | grep "maximum resident set size" | awk '{print $1}' || echo "0"
}

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

# ウォームアップ付き実行計測
# 初回はコールドキャッシュ (ページフォルト) で計測値が不安定になるため、
# 1回ウォームアップしてから計測する。
measure_time_warm() {
    "$@" >/dev/null 2>&1 || true
    /usr/bin/time -l "$@" 2>&1 || true
}

echo "=== L# 言語比較ベンチマーク (fibonacci 35) ==="
echo "    (Rust / Go / JS / MoonBit / L# を比較)"
echo ""

# --- Rust ---
echo "[1/4] Rust をコンパイル・実行中..."
RUST_CTIME="N/A"; RUST_CMEM="N/A"; RUST_ETIME="N/A"; RUST_EMEM="N/A"; RUST_SIZE="N/A"
if command -v rustc &>/dev/null; then
    COUT=$(/usr/bin/time -l rustc -O "$PROGRAMS_DIR/fib.rs" -o "$TMP_DIR/fib_rust" 2>&1 || true)
    RUST_CTIME=$(extract_real_time "$COUT")
    RUST_CMEM=$(extract_rss "$COUT")
    if [[ -f "$TMP_DIR/fib_rust" ]]; then
        RUST_SIZE=$(wc -c < "$TMP_DIR/fib_rust" | tr -d ' ')
        EOUT=$(measure_time_warm "$TMP_DIR/fib_rust" 2>/dev/null)
        RUST_ETIME=$(extract_real_time "$EOUT")
        RUST_EMEM=$(extract_rss "$EOUT")
    fi
else
    echo "  rustc が見つかりません。スキップ。"
fi

# --- Go ---
echo "[2/4] Go をコンパイル・実行中..."
GO_CTIME="N/A"; GO_CMEM="N/A"; GO_ETIME="N/A"; GO_EMEM="N/A"; GO_SIZE="N/A"
if command -v go &>/dev/null; then
    COUT=$(/usr/bin/time -l go build -o "$TMP_DIR/fib_go" "$PROGRAMS_DIR/fib.go" 2>&1 || true)
    GO_CTIME=$(extract_real_time "$COUT")
    GO_CMEM=$(extract_rss "$COUT")
    if [[ -f "$TMP_DIR/fib_go" ]]; then
        GO_SIZE=$(wc -c < "$TMP_DIR/fib_go" | tr -d ' ')
        EOUT=$(measure_time_warm "$TMP_DIR/fib_go" 2>/dev/null)
        GO_ETIME=$(extract_real_time "$EOUT")
        GO_EMEM=$(extract_rss "$EOUT")
    fi
else
    echo "  go が見つかりません。スキップ。"
fi

# --- JavaScript ---
echo "[3/4] JavaScript (Node.js) を実行中..."
JS_ETIME="N/A"; JS_EMEM="N/A"
if command -v node &>/dev/null; then
    EOUT=$(measure_time_warm node "$PROGRAMS_DIR/fib.js" 2>/dev/null)
    JS_ETIME=$(extract_real_time "$EOUT")
    JS_EMEM=$(extract_rss "$EOUT")
else
    echo "  node が見つかりません。スキップ。"
fi

# --- MoonBit ---
echo "[4/5] MoonBit をコンパイル・実行中..."
MOONBIT_CTIME="N/A"; MOONBIT_CMEM="N/A"; MOONBIT_ETIME="N/A"; MOONBIT_EMEM="N/A"; MOONBIT_SIZE="N/A"
MOONBIT_DIR="$SCRIPT_DIR/bench-programs/moonbit"
if command -v moon &>/dev/null && [[ -d "$MOONBIT_DIR" ]]; then
    (cd "$MOONBIT_DIR" && moon clean 2>/dev/null)
    COUT=$(/usr/bin/time -l bash -c "cd '$MOONBIT_DIR' && moon build --target wasm --release" 2>&1 || true)
    MOONBIT_CTIME=$(extract_real_time "$COUT")
    MOONBIT_CMEM=$(extract_rss "$COUT")
    MOONBIT_WASM="$MOONBIT_DIR/_build/wasm/release/build/cmd/main/main.wasm"
    if [[ -f "$MOONBIT_WASM" ]]; then
        MOONBIT_SIZE=$(wc -c < "$MOONBIT_WASM" | tr -d ' ')
        EOUT=$(measure_time_warm bash -c "cd '$MOONBIT_DIR' && moon run cmd/main --target wasm" 2>/dev/null)
        MOONBIT_ETIME=$(extract_real_time "$EOUT")
        MOONBIT_EMEM=$(extract_rss "$EOUT")
    fi
else
    echo "  moon が見つかりません。スキップ。"
fi

# --- L# ---
echo "[5/5] L# をコンパイル・実行中..."
LSHARP_CTIME="N/A"; LSHARP_CMEM="N/A"; LSHARP_ETIME="N/A"; LSHARP_EMEM="N/A"; LSHARP_SIZE="N/A"
LSHARP_WASM="$TMP_DIR/fib_lsharp.wasm"
# ベンチ用の fib(35) を使用 (examples/fib.ls は fib(10) なので比較不可)
BENCH_FIB="$SCRIPT_DIR/bench-programs/fib.ls"
COUT=$(/usr/bin/time -l cargo run --quiet --manifest-path "$PROJECT_DIR/Cargo.toml" -- compile "$BENCH_FIB" -o "$LSHARP_WASM" 2>&1 || true)
LSHARP_CTIME=$(extract_real_time "$COUT")
LSHARP_CMEM=$(extract_rss "$COUT")
if [[ -f "$LSHARP_WASM" ]]; then
    LSHARP_SIZE=$(wc -c < "$LSHARP_WASM" | tr -d ' ')
    if command -v wasmtime &>/dev/null; then
        EOUT=$(measure_time_warm wasmtime "$LSHARP_WASM" 2>/dev/null)
        LSHARP_ETIME=$(extract_real_time "$EOUT")
        LSHARP_EMEM=$(extract_rss "$EOUT")
    fi
fi

# --- 結果表示 ---
echo ""
echo "=== コンパイル速度比較 ==="
echo ""
printf "%-15s %15s %15s\n" "言語" "コンパイル時間" "コンパイルRSS"
printf "%-15s %15s %15s\n" "---------------" "---------------" "---------------"
printf "%-15s %15s %15s\n" "Rust" "$RUST_CTIME" "$(format_bytes "$RUST_CMEM")"
printf "%-15s %15s %15s\n" "Go" "$GO_CTIME" "$(format_bytes "$GO_CMEM")"
printf "%-15s %15s %15s\n" "MoonBit (→Wasm)" "$MOONBIT_CTIME" "$(format_bytes "$MOONBIT_CMEM")"
printf "%-15s %15s %15s\n" "L# (→Wasm)" "$LSHARP_CTIME" "$(format_bytes "$LSHARP_CMEM")"
printf "%-15s %15s %15s\n" "JS (Node)" "N/A" "N/A"

echo ""
echo "=== 実行速度比較 ==="
echo ""
printf "%-15s %15s %15s %15s\n" "言語" "実行時間" "実行RSS" "バイナリサイズ"
printf "%-15s %15s %15s %15s\n" "---------------" "---------------" "---------------" "---------------"
printf "%-15s %15s %15s %15s\n" "Rust" "$RUST_ETIME" "$(format_bytes "$RUST_EMEM")" "$(format_bytes "$RUST_SIZE")"
printf "%-15s %15s %15s %15s\n" "Go" "$GO_ETIME" "$(format_bytes "$GO_EMEM")" "$(format_bytes "$GO_SIZE")"
printf "%-15s %15s %15s %15s\n" "MoonBit (moon)" "$MOONBIT_ETIME" "$(format_bytes "$MOONBIT_EMEM")" "$(format_bytes "$MOONBIT_SIZE")"
printf "%-15s %15s %15s %15s\n" "L# (wasmtime)" "$LSHARP_ETIME" "$(format_bytes "$LSHARP_EMEM")" "$(format_bytes "$LSHARP_SIZE")"
printf "%-15s %15s %15s %15s\n" "JS (Node)" "$JS_ETIME" "$(format_bytes "$JS_EMEM")" "N/A"

echo ""
echo "注: 時間の単位は秒 (s)。メモリの単位は MB/KB。"
echo "    L# は wasmtime 経由の実行のため、ランタイム起動オーバーヘッドを含む。"
