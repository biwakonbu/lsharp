#!/bin/bash
# P12-0: compile 中心の公開 CLI ドキュメントが現行 mainline で再現できることを smoke test で確認する
# README.md / AGENTS.md / CLAUDE.md で案内する compile / test / lsp / mcp-server の導線を軽量に検証する

set -euo pipefail

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
cd "$ROOT"

ERRORS=0
PASS=0
DEFAULT_LSHARP_BIN="$ROOT/target/debug/lsharp"
LSHARP_BIN="${LSHARP_BIN:-$DEFAULT_LSHARP_BIN}"
SMOKE_DIR="${SMOKE_DIR:-ci-artifacts/readme-smoke}"
SMOKE_SOURCE="$SMOKE_DIR/fib_smoke.ls"
SMOKE_WASM="$SMOKE_DIR/fib_smoke.wasm"

mkdir -p "$SMOKE_DIR"
trap 'rm -f "$SMOKE_SOURCE" "$SMOKE_WASM"' EXIT
cp examples/fib.ls "$SMOKE_SOURCE"

echo "=== README / docs smoke test ==="
echo ""

# 1. 開発用 CLI または配布済み binary を利用できること
if [[ -x "$LSHARP_BIN" ]]; then
    echo "--- prebuilt lsharp binary: $LSHARP_BIN ---"
    echo "PASS: using prebuilt lsharp binary ($LSHARP_BIN)"
    PASS=$((PASS + 1))
elif [[ "$LSHARP_BIN" != "$DEFAULT_LSHARP_BIN" ]]; then
    echo "--- prebuilt lsharp binary: $LSHARP_BIN ---"
    echo "FAIL: lsharp binary not executable ($LSHARP_BIN)"
    ERRORS=$((ERRORS + 1))
else
    echo "--- cargo build -p lsharp-driver ---"
    if cargo build -p lsharp-driver 2>&1 | tail -3; then
        echo "PASS: cargo build -p lsharp-driver"
        PASS=$((PASS + 1))
    else
        echo "FAIL: cargo build -p lsharp-driver"
        ERRORS=$((ERRORS + 1))
    fi
fi

# 2. README Quick Start の compile 導線が通ること
echo ""
echo "--- $LSHARP_BIN compile $SMOKE_SOURCE ---"
if "$LSHARP_BIN" compile "$SMOKE_SOURCE" -o "$SMOKE_WASM" 2>&1 | tail -3; then
    echo "PASS: $LSHARP_BIN compile $SMOKE_SOURCE"
    PASS=$((PASS + 1))
else
    echo "FAIL: $LSHARP_BIN compile $SMOKE_SOURCE"
    ERRORS=$((ERRORS + 1))
fi

# 3. Wasm artifact が生成されること
echo ""
echo "--- artifact check: $SMOKE_WASM ---"
if [ -s "$SMOKE_WASM" ]; then
    echo "PASS: wasm artifact generated ($SMOKE_WASM)"
    PASS=$((PASS + 1))
else
    echo "FAIL: wasm artifact not generated ($SMOKE_WASM)"
    ERRORS=$((ERRORS + 1))
fi

# 4. README の metadata test 導線が通ること
echo ""
echo "--- $LSHARP_BIN test examples/metadata.ls ---"
if "$LSHARP_BIN" test examples/metadata.ls 2>&1 | tail -6; then
    echo "PASS: $LSHARP_BIN test examples/metadata.ls"
    PASS=$((PASS + 1))
else
    echo "FAIL: $LSHARP_BIN test examples/metadata.ls"
    ERRORS=$((ERRORS + 1))
fi

# 5. LSP backend の入口が存在すること
echo ""
echo "--- $LSHARP_BIN lsp --help ---"
if "$LSHARP_BIN" lsp --help 2>&1 | head -5; then
    echo "PASS: $LSHARP_BIN lsp --help"
    PASS=$((PASS + 1))
else
    echo "FAIL: $LSHARP_BIN lsp --help"
    ERRORS=$((ERRORS + 1))
fi

# 6. MCP backend の入口が存在すること
echo ""
echo "--- $LSHARP_BIN mcp-server --help ---"
if "$LSHARP_BIN" mcp-server --help 2>&1 | head -5; then
    echo "PASS: $LSHARP_BIN mcp-server --help"
    PASS=$((PASS + 1))
else
    echo "FAIL: $LSHARP_BIN mcp-server --help"
    ERRORS=$((ERRORS + 1))
fi

# 7. wasmtime で実行できること (wasmtime が利用可能な場合のみ)
echo ""
echo "--- wasmtime $SMOKE_WASM ---"
if command -v wasmtime &> /dev/null; then
    OUTPUT=$(wasmtime "$SMOKE_WASM" 2>&1 || true)
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

echo ""
echo "=== smoke test 完了: PASS=$PASS, FAIL=$ERRORS ==="
exit $ERRORS
