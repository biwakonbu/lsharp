#!/usr/bin/env bash
# C-1: build-time embedded guest component を積んだ `lsharp` バイナリの default path smoke
set -euo pipefail
ROOT="$(cd "$(dirname "$0")/../.." && pwd)"
cd "$ROOT"

LSHARP_BIN="${LSHARP_BIN:-$ROOT/target/debug/lsharp}"
OUT_DIR="${OUT_DIR:-$ROOT/target/ci/default-path-smoke}"

mkdir -p "$OUT_DIR"

if [[ ! -x "$LSHARP_BIN" ]]; then
  echo "=== default-path-smoke: build lsharp binary ==="
  cargo build -p lsharp-driver -q
fi

if [[ ! -x "$LSHARP_BIN" ]]; then
  echo "ERROR: lsharp binary not executable: $LSHARP_BIN"
  exit 1
fi

echo "=== default-path-smoke: compile (examples/fib.ls) ==="
OUT_COMPONENT="$OUT_DIR/examples_fib.component.wasm"
rm -f "$OUT_COMPONENT"
"$LSHARP_BIN" compile examples/fib.ls -o "$OUT_COMPONENT"
if [[ ! -s "$OUT_COMPONENT" ]]; then
  echo "ERROR: component output empty: $OUT_COMPONENT"
  exit 1
fi

echo "=== default-path-smoke: embedded guest default path ==="
EMBED_SMOKE_DIR="$OUT_DIR/embedded-smoke"
EMBED_COMPONENT_ARTIFACT="$EMBED_SMOKE_DIR/selfhost-embedded.component.wasm"
EMBED_SMOKE_INPUT="$EMBED_SMOKE_DIR/smoke_input.ls"
rm -rf "$EMBED_SMOKE_DIR"
mkdir -p "$EMBED_SMOKE_DIR"
python3 - <<'PY' > "$EMBED_SMOKE_INPUT"
print(";; " + "x" * 5000)
print("(defn main [] 42)")
PY

PARSE_OUTPUT="$(cd "$EMBED_SMOKE_DIR" && "$LSHARP_BIN" parse smoke_input.ls)"
if [[ "$PARSE_OUTPUT" != *"decls:1"* ]] || [[ "$PARSE_OUTPUT" != *"diagnostics:0"* ]]; then
  echo "ERROR: embedded parse output mismatch"
  echo "$PARSE_OUTPUT"
  exit 1
fi

CHECK_OUTPUT="$(cd "$EMBED_SMOKE_DIR" && "$LSHARP_BIN" check smoke_input.ls)"
if [[ "$CHECK_OUTPUT" != *"diagnostics:0"* ]]; then
  echo "ERROR: embedded check output mismatch"
  echo "$CHECK_OUTPUT"
  exit 1
fi

cat > "$EMBED_SMOKE_DIR/test_input.ls" <<'EOF'
(defn abs
  [x]
  :example [(= (abs 5) 5)]
  :invariant (>= result 0)
  (if (< x 0) (- 0 x) x))
EOF

TEST_OUTPUT="$(cd "$EMBED_SMOKE_DIR" && "$LSHARP_BIN" test test_input.ls)"
if [[ "$TEST_OUTPUT" != *"examples:1"* ]] || [[ "$TEST_OUTPUT" != *"invariants:1"* ]] || [[ "$TEST_OUTPUT" != *"failures:0"* ]]; then
  echo "ERROR: embedded test output mismatch"
  echo "$TEST_OUTPUT"
  exit 1
fi

FMT_OUTPUT="$(cd "$EMBED_SMOKE_DIR" && "$LSHARP_BIN" fmt smoke_input.ls)"
if [[ "$FMT_OUTPUT" != "$(cat "$EMBED_SMOKE_INPUT")" ]]; then
  echo "ERROR: embedded fmt output mismatch"
  printf 'expected:\n%s\nactual:\n%s\n' "$(cat "$EMBED_SMOKE_INPUT")" "$FMT_OUTPUT"
  exit 1
fi

set +e
DISABLE_PARSE_OUTPUT="$(cd "$EMBED_SMOKE_DIR" && LSHARP_DISABLE_EMBEDDED_COMPONENT=1 "$LSHARP_BIN" parse smoke_input.ls 2>&1)"
DISABLE_PARSE_STATUS=$?
set -e
if [[ $DISABLE_PARSE_STATUS -eq 0 ]]; then
  echo "ERROR: embedded disable flag unexpectedly allowed parse"
  exit 1
fi
if [[ "$DISABLE_PARSE_OUTPUT" != *"LSHARP_PATH"* ]]; then
  echo "ERROR: embedded disable flag did not restore shadow command hint"
  echo "$DISABLE_PARSE_OUTPUT"
  exit 1
fi

"$LSHARP_BIN" compile selfhost/src/App/EmbeddedCli.ls -o "$EMBED_COMPONENT_ARTIFACT"
if [[ ! -s "$EMBED_COMPONENT_ARTIFACT" ]]; then
  echo "ERROR: selfhost embedded component artifact empty: $EMBED_COMPONENT_ARTIFACT"
  exit 1
fi

COMPONENT_CHECK_OUTPUT="$(cd "$EMBED_SMOKE_DIR" && LSHARP_PATH="$EMBED_COMPONENT_ARTIFACT" "$LSHARP_BIN" check smoke_input.ls)"
if [[ "$COMPONENT_CHECK_OUTPUT" != *"diagnostics:0"* ]]; then
  echo "ERROR: external component delegation output mismatch"
  echo "$COMPONENT_CHECK_OUTPUT"
  exit 1
fi

echo "=== default-path-smoke: LSHARP_PATH delegation ==="
DELEGATE_ROOT="$OUT_DIR/delegation"
DELEGATE_EXEC="$DELEGATE_ROOT/delegate-exec.sh"
DELEGATE_DIR="$DELEGATE_ROOT/dir"
DELEGATE_DIR_BIN="$DELEGATE_DIR/lsharp"
rm -rf "$DELEGATE_ROOT"
mkdir -p "$DELEGATE_DIR"

cat > "$DELEGATE_EXEC" <<'EOF'
#!/usr/bin/env bash
set -euo pipefail
echo "delegated-exec:$*"
exit 23
EOF
chmod +x "$DELEGATE_EXEC"

cat > "$DELEGATE_DIR_BIN" <<'EOF'
#!/usr/bin/env bash
set -euo pipefail
echo "delegated-dir:$*"
exit 29
EOF
chmod +x "$DELEGATE_DIR_BIN"

set +e
DELEGATE_EXEC_OUTPUT="$(LSHARP_PATH="$DELEGATE_EXEC" "$LSHARP_BIN" --version 2>&1)"
DELEGATE_EXEC_STATUS=$?
DELEGATE_DIR_OUTPUT="$(LSHARP_PATH="$DELEGATE_DIR" "$LSHARP_BIN" --version 2>&1)"
DELEGATE_DIR_STATUS=$?
DELEGATE_MISSING_OUTPUT="$(LSHARP_PATH="$DELEGATE_ROOT/missing" "$LSHARP_BIN" --version 2>&1)"
DELEGATE_MISSING_STATUS=$?
set -e

if [[ $DELEGATE_EXEC_STATUS -ne 23 ]]; then
  echo "ERROR: executable-path delegation status mismatch: $DELEGATE_EXEC_STATUS"
  echo "$DELEGATE_EXEC_OUTPUT"
  exit 1
fi
if [[ "$DELEGATE_EXEC_OUTPUT" != *"delegated-exec:--version"* ]]; then
  echo "ERROR: executable-path delegation output mismatch"
  echo "$DELEGATE_EXEC_OUTPUT"
  exit 1
fi

if [[ $DELEGATE_DIR_STATUS -ne 29 ]]; then
  echo "ERROR: directory-path delegation status mismatch: $DELEGATE_DIR_STATUS"
  echo "$DELEGATE_DIR_OUTPUT"
  exit 1
fi
if [[ "$DELEGATE_DIR_OUTPUT" != *"delegated-dir:--version"* ]]; then
  echo "ERROR: directory-path delegation output mismatch"
  echo "$DELEGATE_DIR_OUTPUT"
  exit 1
fi

if [[ $DELEGATE_MISSING_STATUS -eq 0 ]]; then
  echo "ERROR: invalid LSHARP_PATH unexpectedly succeeded"
  exit 1
fi
if [[ "$DELEGATE_MISSING_OUTPUT" != *"LSHARP_PATH"* ]]; then
  echo "ERROR: invalid LSHARP_PATH did not surface an explicit error"
  echo "$DELEGATE_MISSING_OUTPUT"
  exit 1
fi

echo "default-path-smoke: OK"
