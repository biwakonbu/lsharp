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

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
PROJECT_DIR="$(cd "$SCRIPT_DIR/.." && pwd)"
PROGRAMS_DIR="$SCRIPT_DIR/bench-programs"
TMP_DIR=$(mktemp -d)

cleanup() {
    rm -rf "$TMP_DIR"
}
trap cleanup EXIT

echo "=== L# 言語比較ベンチマーク (fibonacci 35) ==="
echo ""

# 結果格納用
declare -A TIMES
declare -A MEMORIES
declare -A SIZES

# --- Rust ---
echo "[1/4] Rust をコンパイル・実行中..."
if command -v rustc &>/dev/null; then
    rustc -O "$PROGRAMS_DIR/fib.rs" -o "$TMP_DIR/fib_rust" 2>/dev/null
    SIZES["Rust"]=$(wc -c < "$TMP_DIR/fib_rust" | tr -d ' ')

    # /usr/bin/time で計測 (macOS 形式)
    TIME_OUTPUT=$(/usr/bin/time -l "$TMP_DIR/fib_rust" 2>&1 >/dev/null || true)
    TIMES["Rust"]=$(echo "$TIME_OUTPUT" | grep "real" | awk '{print $1}' || echo "N/A")
    MEMORIES["Rust"]=$(echo "$TIME_OUTPUT" | grep "maximum resident set size" | awk '{print $1}' || echo "N/A")
else
    echo "  rustc が見つかりません。スキップ。"
    TIMES["Rust"]="N/A"
    MEMORIES["Rust"]="N/A"
    SIZES["Rust"]="N/A"
fi

# --- Go ---
echo "[2/4] Go をコンパイル・実行中..."
if command -v go &>/dev/null; then
    go build -o "$TMP_DIR/fib_go" "$PROGRAMS_DIR/fib.go" 2>/dev/null
    SIZES["Go"]=$(wc -c < "$TMP_DIR/fib_go" | tr -d ' ')

    TIME_OUTPUT=$(/usr/bin/time -l "$TMP_DIR/fib_go" 2>&1 >/dev/null || true)
    TIMES["Go"]=$(echo "$TIME_OUTPUT" | grep "real" | awk '{print $1}' || echo "N/A")
    MEMORIES["Go"]=$(echo "$TIME_OUTPUT" | grep "maximum resident set size" | awk '{print $1}' || echo "N/A")
else
    echo "  go が見つかりません。スキップ。"
    TIMES["Go"]="N/A"
    MEMORIES["Go"]="N/A"
    SIZES["Go"]="N/A"
fi

# --- JavaScript (Node.js) ---
echo "[3/4] JavaScript (Node.js) を実行中..."
if command -v node &>/dev/null; then
    SIZES["JS"]="N/A"

    TIME_OUTPUT=$(/usr/bin/time -l node "$PROGRAMS_DIR/fib.js" 2>&1 >/dev/null || true)
    TIMES["JS"]=$(echo "$TIME_OUTPUT" | grep "real" | awk '{print $1}' || echo "N/A")
    MEMORIES["JS"]=$(echo "$TIME_OUTPUT" | grep "maximum resident set size" | awk '{print $1}' || echo "N/A")
else
    echo "  node が見つかりません。スキップ。"
    TIMES["JS"]="N/A"
    MEMORIES["JS"]="N/A"
    SIZES["JS"]="N/A"
fi

# --- L# (Wasm via wasmtime) ---
echo "[4/4] L# をコンパイル・実行中..."
LSHARP_WASM="$TMP_DIR/fib_lsharp.wasm"

# コンパイル時間計測
COMPILE_OUTPUT=$(/usr/bin/time -l cargo run --quiet --manifest-path "$PROJECT_DIR/Cargo.toml" -- compile "$PROJECT_DIR/examples/fib.ls" -o "$LSHARP_WASM" 2>&1 || true)

if [[ -f "$LSHARP_WASM" ]]; then
    SIZES["L#"]=$(wc -c < "$LSHARP_WASM" | tr -d ' ')

    # 実行時間計測
    if command -v wasmtime &>/dev/null; then
        TIME_OUTPUT=$(/usr/bin/time -l wasmtime "$LSHARP_WASM" 2>&1 >/dev/null || true)
        TIMES["L#"]=$(echo "$TIME_OUTPUT" | grep "real" | awk '{print $1}' || echo "N/A")
        MEMORIES["L#"]=$(echo "$TIME_OUTPUT" | grep "maximum resident set size" | awk '{print $1}' || echo "N/A")
    else
        echo "  wasmtime が見つかりません。実行スキップ。"
        TIMES["L#"]="N/A"
        MEMORIES["L#"]="N/A"
    fi
else
    echo "  L# コンパイルに失敗しました。"
    TIMES["L#"]="N/A"
    MEMORIES["L#"]="N/A"
    SIZES["L#"]="N/A"
fi

# --- 結果表示 ---
echo ""
echo "=== 結果 ==="
echo ""
printf "%-8s %15s %15s %15s\n" "言語" "実行時間" "RSS メモリ" "バイナリサイズ"
printf "%-8s %15s %15s %15s\n" "--------" "---------------" "---------------" "---------------"

for lang in "Rust" "Go" "JS" "L#"; do
    time_val="${TIMES[$lang]:-N/A}"
    mem_val="${MEMORIES[$lang]:-N/A}"
    size_val="${SIZES[$lang]:-N/A}"

    # メモリを MB に変換 (macOS の maximum resident set size はバイト単位)
    if [[ "$mem_val" != "N/A" && "$mem_val" =~ ^[0-9]+$ ]]; then
        mem_mb=$(python3 -c "print(f'{$mem_val / 1024 / 1024:.1f} MB')" 2>/dev/null || echo "$mem_val B")
    else
        mem_mb="$mem_val"
    fi

    # サイズを KB に変換
    if [[ "$size_val" != "N/A" && "$size_val" =~ ^[0-9]+$ ]]; then
        size_kb=$(python3 -c "print(f'{$size_val / 1024:.1f} KB')" 2>/dev/null || echo "$size_val B")
    else
        size_kb="$size_val"
    fi

    printf "%-8s %15s %15s %15s\n" "$lang" "$time_val" "$mem_mb" "$size_kb"
done

echo ""
echo "注: 実行時間は /usr/bin/time の real 時間。"
echo "    hyperfine がインストールされている場合はより正確な計測が可能です。"
