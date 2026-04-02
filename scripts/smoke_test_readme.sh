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
SMOKE_SOURCE="$SMOKE_DIR/quickstart.ls"
SMOKE_METADATA_SOURCE="$SMOKE_DIR/quickstart-metadata.ls"
SMOKE_WASM="$SMOKE_DIR/quickstart.wasm"
SMOKE_DOC_HTML="$SMOKE_DIR/quickstart-metadata.html"

hash_file() {
    local path="$1"
    if command -v sha256sum >/dev/null 2>&1; then
        sha256sum "$path" | awk '{print $1}'
    elif command -v shasum >/dev/null 2>&1; then
        shasum -a 256 "$path" | awk '{print $1}'
    else
        echo "FAIL: sha256sum / shasum が見つからない"
        ERRORS=$((ERRORS + 1))
        return 1
    fi
}

mkdir -p "$SMOKE_DIR"
trap 'rm -f "$SMOKE_SOURCE" "$SMOKE_METADATA_SOURCE" "$SMOKE_WASM" "$SMOKE_DOC_HTML"' EXIT

cat > "$SMOKE_SOURCE" <<'EOF'
(defn main [] 42)
EOF

cat > "$SMOKE_METADATA_SOURCE" <<'EOF'
(defn abs
  [x]
  :doc "整数の絶対値を返す。"
  :params [(x "対象の整数")]
  :returns "x の絶対値"
  :example [(= (abs 5) 5)]
  :invariant (>= result 0)
  (if (< x 0) (- 0 x) x))
EOF

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

# 2. README Quick Start の checksum 導線が packaged archive で通ること
echo ""
echo "--- packaged checksum verification ---"
PACKAGED_ROOT="$(cd "$(dirname "$LSHARP_BIN")" && pwd)"
if [[ -f "$PACKAGED_ROOT/checksums.txt" ]]; then
    CHECKSUM_FAILED=0
    while read -r expected relpath _; do
        [[ -n "${expected:-}" ]] || continue
        TARGET_PATH="$PACKAGED_ROOT/$relpath"
        if [[ ! -f "$TARGET_PATH" ]]; then
            echo "FAIL: checksum target missing ($relpath)"
            CHECKSUM_FAILED=1
            break
        fi
        ACTUAL="$(hash_file "$TARGET_PATH")" || {
            CHECKSUM_FAILED=1
            break
        }
        if [[ "$ACTUAL" != "$expected" ]]; then
            echo "FAIL: checksum mismatch ($relpath)"
            CHECKSUM_FAILED=1
            break
        fi
    done < "$PACKAGED_ROOT/checksums.txt"
    if [[ "$CHECKSUM_FAILED" == "0" ]]; then
        echo "PASS: packaged checksums.txt"
        PASS=$((PASS + 1))
    else
        ERRORS=$((ERRORS + 1))
    fi
else
    echo "SKIP: packaged checksums.txt not found next to $LSHARP_BIN"
fi

# 3. README Quick Start の compile 導線が通ること
echo ""
echo "--- $LSHARP_BIN compile $SMOKE_SOURCE ---"
if "$LSHARP_BIN" compile "$SMOKE_SOURCE" -o "$SMOKE_WASM" 2>&1 | tail -3; then
    echo "PASS: $LSHARP_BIN compile $SMOKE_SOURCE"
    PASS=$((PASS + 1))
else
    echo "FAIL: $LSHARP_BIN compile $SMOKE_SOURCE"
    ERRORS=$((ERRORS + 1))
fi

# 4. Wasm artifact が生成されること
echo ""
echo "--- artifact check: $SMOKE_WASM ---"
if [ -s "$SMOKE_WASM" ] && xxd -p -l 4 "$SMOKE_WASM" | grep -qi '^0061736d$'; then
    echo "PASS: wasm artifact generated ($SMOKE_WASM)"
    PASS=$((PASS + 1))
else
    echo "FAIL: wasm artifact not generated ($SMOKE_WASM)"
    ERRORS=$((ERRORS + 1))
fi

# 5. README の metadata test 導線が通ること
echo ""
echo "--- $LSHARP_BIN test $SMOKE_METADATA_SOURCE ---"
if "$LSHARP_BIN" test "$SMOKE_METADATA_SOURCE" 2>&1 | tail -6; then
    echo "PASS: $LSHARP_BIN test $SMOKE_METADATA_SOURCE"
    PASS=$((PASS + 1))
else
    echo "FAIL: $LSHARP_BIN test $SMOKE_METADATA_SOURCE"
    ERRORS=$((ERRORS + 1))
fi

# 6. LSP backend の入口が存在すること
echo ""
echo "--- $LSHARP_BIN lsp --help ---"
if "$LSHARP_BIN" lsp --help 2>&1 | head -5; then
    echo "PASS: $LSHARP_BIN lsp --help"
    PASS=$((PASS + 1))
else
    echo "FAIL: $LSHARP_BIN lsp --help"
    ERRORS=$((ERRORS + 1))
fi

# 7. metadata-driven docs の入口が通ること
echo ""
echo "--- $LSHARP_BIN doc $SMOKE_METADATA_SOURCE ---"
if "$LSHARP_BIN" doc "$SMOKE_METADATA_SOURCE" -o "$SMOKE_DOC_HTML" 2>&1 | tail -3; then
    if [ -s "$SMOKE_DOC_HTML" ]; then
        echo "PASS: $LSHARP_BIN doc $SMOKE_METADATA_SOURCE"
        PASS=$((PASS + 1))
    else
        echo "FAIL: doc artifact not generated ($SMOKE_DOC_HTML)"
        ERRORS=$((ERRORS + 1))
    fi
else
    echo "FAIL: $LSHARP_BIN doc $SMOKE_METADATA_SOURCE"
    ERRORS=$((ERRORS + 1))
fi

# 8. MCP backend の入口が存在すること
echo ""
echo "--- $LSHARP_BIN mcp-server --help ---"
if "$LSHARP_BIN" mcp-server --help 2>&1 | head -5; then
    echo "PASS: $LSHARP_BIN mcp-server --help"
    PASS=$((PASS + 1))
else
    echo "FAIL: $LSHARP_BIN mcp-server --help"
    ERRORS=$((ERRORS + 1))
fi

echo ""
echo "=== smoke test 完了: PASS=$PASS, FAIL=$ERRORS ==="
exit $ERRORS
