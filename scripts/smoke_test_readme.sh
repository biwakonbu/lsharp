#!/bin/bash
# P11-1d: README.md の導入手順が現行 mainline で再現できることを smoke test で確認する
# README.md と book/ に記載されたコマンド例を抽出して実行し、再現性を検証する

set -euo pipefail

ERRORS=0
PASS=0

echo "=== README / book smoke test ==="
echo ""

# 1. cargo build が通ること
echo "--- cargo build ---"
if cargo build 2>&1 | tail -1; then
    echo "PASS: cargo build"
    PASS=$((PASS + 1))
else
    echo "FAIL: cargo build"
    ERRORS=$((ERRORS + 1))
fi

# 2. cargo run -- check examples/fib.ls が通ること (README Quick Start)
echo ""
echo "--- cargo run -- check examples/fib.ls ---"
if cargo run -- check examples/fib.ls 2>&1 | tail -3; then
    echo "PASS: cargo run -- check examples/fib.ls"
    PASS=$((PASS + 1))
else
    echo "FAIL: cargo run -- check examples/fib.ls"
    ERRORS=$((ERRORS + 1))
fi

# 3. cargo run -- compile examples/fib.ls -o /tmp/fib_smoke.wasm が通ること
echo ""
echo "--- cargo run -- compile examples/fib.ls ---"
if cargo run -- compile examples/fib.ls -o /tmp/fib_smoke.wasm 2>&1 | tail -3; then
    echo "PASS: cargo run -- compile examples/fib.ls"
    PASS=$((PASS + 1))
else
    echo "FAIL: cargo run -- compile examples/fib.ls"
    ERRORS=$((ERRORS + 1))
fi

# 4. wasmtime で実行できること (wasmtime が利用可能な場合のみ)
echo ""
echo "--- wasmtime /tmp/fib_smoke.wasm ---"
if command -v wasmtime &> /dev/null; then
    OUTPUT=$(wasmtime /tmp/fib_smoke.wasm 2>&1 || true)
    if echo "$OUTPUT" | grep -q "55"; then
        echo "PASS: wasmtime fib.wasm => 55"
        PASS=$((PASS + 1))
    else
        echo "FAIL: wasmtime fib.wasm の出力が期待値と異なる: $OUTPUT"
        ERRORS=$((ERRORS + 1))
    fi
else
    echo "SKIP: wasmtime が見つからない (任意依存)"
fi

# 5. cargo run -- parse examples/fib.ls --ast が通ること (README Architecture 例)
echo ""
echo "--- cargo run -- parse examples/fib.ls --ast ---"
if cargo run -- parse examples/fib.ls --ast 2>&1 | head -5; then
    echo "PASS: cargo run -- parse examples/fib.ls --ast"
    PASS=$((PASS + 1))
else
    echo "FAIL: cargo run -- parse examples/fib.ls --ast"
    ERRORS=$((ERRORS + 1))
fi

# 6. cargo test が通ること
echo ""
echo "--- cargo test (quick check) ---"
if cargo test 2>&1 | tail -3; then
    echo "PASS: cargo test"
    PASS=$((PASS + 1))
else
    echo "FAIL: cargo test"
    ERRORS=$((ERRORS + 1))
fi

# クリーンアップ
rm -f /tmp/fib_smoke.wasm

echo ""
echo "=== smoke test 完了: PASS=$PASS, FAIL=$ERRORS ==="
exit $ERRORS
