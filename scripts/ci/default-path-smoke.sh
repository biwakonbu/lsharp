#!/usr/bin/env bash
# OPS-05: `cargo run` ではなくビルド済み `lsharp` バイナリが compile を実行できること（default path 移行の第1段）
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
OUT_WASM="$OUT_DIR/examples_fib.wasm"
rm -f "$OUT_WASM"
"$LSHARP_BIN" compile examples/fib.ls -o "$OUT_WASM"
if [[ ! -s "$OUT_WASM" ]]; then
  echo "ERROR: wasm output empty: $OUT_WASM"
  exit 1
fi

echo "=== default-path-smoke: selfhost wasm smoke ==="
WASM_SMOKE_DIR="$OUT_DIR/wasm-smoke"
WASM_SMOKE_ARTIFACT="$WASM_SMOKE_DIR/selfhost-smoke.wasm"
WASM_SMOKE_INPUT="$WASM_SMOKE_DIR/smoke_input.ls"
WASM_SMOKE_OUTPUT="$WASM_SMOKE_DIR/smoke_output.txt"
rm -rf "$WASM_SMOKE_DIR"
mkdir -p "$WASM_SMOKE_DIR"
printf '(defn main [] 42)\n' > "$WASM_SMOKE_INPUT"
"$LSHARP_BIN" compile selfhost/src/App/SmokeCli.ls -o "$WASM_SMOKE_ARTIFACT"
if [[ ! -s "$WASM_SMOKE_ARTIFACT" ]]; then
  echo "ERROR: selfhost wasm artifact empty: $WASM_SMOKE_ARTIFACT"
  exit 1
fi

WASM_CHECK_OUTPUT="$(cd "$WASM_SMOKE_DIR" && LSHARP_PATH="$WASM_SMOKE_ARTIFACT" "$LSHARP_BIN" check smoke_input.ls)"
if [[ "$WASM_CHECK_OUTPUT" != *"check:ok"* ]] || [[ "$WASM_CHECK_OUTPUT" != *"diagnostics:0"* ]]; then
  echo "ERROR: selfhost wasm check output mismatch"
  echo "$WASM_CHECK_OUTPUT"
  exit 1
fi

WASM_FMT_OUTPUT="$(cd "$WASM_SMOKE_DIR" && LSHARP_PATH="$WASM_SMOKE_ARTIFACT" "$LSHARP_BIN" fmt smoke_input.ls)"
if [[ "$WASM_FMT_OUTPUT" != "$(cat "$WASM_SMOKE_INPUT")" ]]; then
  echo "ERROR: selfhost wasm fmt output mismatch"
  printf 'expected:\n%s\nactual:\n%s\n' "$(cat "$WASM_SMOKE_INPUT")" "$WASM_FMT_OUTPUT"
  exit 1
fi

WASM_COMPILE_OUTPUT="$(cd "$WASM_SMOKE_DIR" && LSHARP_PATH="$WASM_SMOKE_ARTIFACT" "$LSHARP_BIN" compile smoke_input.ls -o smoke_output.txt)"
if [[ "$WASM_COMPILE_OUTPUT" != *"wasm-size:"* ]]; then
  echo "ERROR: selfhost wasm compile output mismatch"
  echo "$WASM_COMPILE_OUTPUT"
  exit 1
fi
if [[ ! -s "$WASM_SMOKE_OUTPUT" ]]; then
  echo "ERROR: selfhost wasm compile output file empty: $WASM_SMOKE_OUTPUT"
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
