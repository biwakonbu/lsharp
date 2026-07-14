#!/usr/bin/env bash
# C-1: build-time embedded guest component を積んだ `lsharp` バイナリの default path smoke
set -euo pipefail
ROOT="$(cd "$(dirname "$0")/../.." && pwd)"
cd "$ROOT"

LSHARP_BIN="${LSHARP_BIN:-$ROOT/target/debug/lsharp}"
OUT_DIR_INPUT="${OUT_DIR:-target/ci/default-path-smoke}"

if [[ "$OUT_DIR_INPUT" = /* ]]; then
  if [[ "$OUT_DIR_INPUT" != "$ROOT"/* ]]; then
    echo "ERROR: OUT_DIR must be under repository root for embedded compile/build smoke: $OUT_DIR_INPUT"
    exit 1
  fi
  OUT_DIR_REL="${OUT_DIR_INPUT#"$ROOT"/}"
  OUT_DIR="$OUT_DIR_INPUT"
else
  OUT_DIR_REL="$OUT_DIR_INPUT"
  OUT_DIR="$ROOT/$OUT_DIR_INPUT"
fi

mkdir -p "$OUT_DIR"

if [[ ! -x "$LSHARP_BIN" ]]; then
  echo "ERROR: lsharp binary is required: $LSHARP_BIN" >&2
  echo "Set LSHARP_BIN to a packaged lsharp/program.native before running this smoke." >&2
  exit 1
fi

echo "=== default-path-smoke: embedded compile/build default path ==="
OUT_COMPONENT="$OUT_DIR/examples_fib.component.wasm"
OUT_BUILD="$OUT_DIR/examples_fib_build.component.wasm"
REL_OUT_COMPONENT="$OUT_DIR_REL/examples_fib.component.wasm"
REL_OUT_BUILD="$OUT_DIR_REL/examples_fib_build.component.wasm"
rm -f "$OUT_COMPONENT"
rm -f "$OUT_BUILD"

COMPILE_OUTPUT="$("$LSHARP_BIN" compile examples/fib.ls -o "$REL_OUT_COMPONENT")"
if [[ "$COMPILE_OUTPUT" != *"wasm-size:"* ]]; then
  echo "ERROR: embedded compile output mismatch"
  echo "$COMPILE_OUTPUT"
  exit 1
fi
if [[ ! -s "$OUT_COMPONENT" ]]; then
  echo "ERROR: component output empty: $OUT_COMPONENT"
  exit 1
fi
if ! xxd -p -l 4 "$OUT_COMPONENT" | grep -qi '^0061736d$'; then
  echo "ERROR: component output is not a Wasm binary: $OUT_COMPONENT"
  exit 1
fi

BUILD_OUTPUT="$("$LSHARP_BIN" build examples/fib.ls --output "$REL_OUT_BUILD")"
if [[ "$BUILD_OUTPUT" != *"wasm-size:"* ]]; then
  echo "ERROR: embedded build output mismatch"
  echo "$BUILD_OUTPUT"
  exit 1
fi
if [[ ! -s "$OUT_BUILD" ]]; then
  echo "ERROR: build output empty: $OUT_BUILD"
  exit 1
fi
if ! xxd -p -l 4 "$OUT_BUILD" | grep -qi '^0061736d$'; then
  echo "ERROR: build output is not a Wasm binary: $OUT_BUILD"
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

cat > "$EMBED_SMOKE_DIR/review_input.ls" <<'EOF'
(defn main [] (let [x 42] 0))
EOF

REVIEW_OUTPUT="$(cd "$EMBED_SMOKE_DIR" && "$LSHARP_BIN" review review_input.ls)"
if [[ "$REVIEW_OUTPUT" != *"unused-let"* ]] || [[ "$REVIEW_OUTPUT" != *"diagnostics:1,first-body:let binding x is not used"* ]] || [[ "$REVIEW_OUTPUT" != *"warning"* ]] || [[ "$REVIEW_OUTPUT" != *"L0001@1:1"* ]]; then
  echo "ERROR: embedded review output mismatch"
  echo "$REVIEW_OUTPUT"
  exit 1
fi

REVIEW_JSON_OUTPUT="$(cd "$EMBED_SMOKE_DIR" && "$LSHARP_BIN" review review_input.ls --json)"
if [[ "$REVIEW_JSON_OUTPUT" != *'"source":"source-200"'* ]] || [[ "$REVIEW_JSON_OUTPUT" != *'"title":"unused-let"'* ]] || [[ "$REVIEW_JSON_OUTPUT" != *'"code":"L0001"'* ]]; then
  echo "ERROR: embedded review --json output mismatch"
  echo "$REVIEW_JSON_OUTPUT"
  exit 1
fi

REVIEW_FORMAT_JSON_OUTPUT="$(cd "$EMBED_SMOKE_DIR" && "$LSHARP_BIN" review review_input.ls --format json)"
if [[ "$REVIEW_FORMAT_JSON_OUTPUT" != *'"source":"source-200"'* ]] || [[ "$REVIEW_FORMAT_JSON_OUTPUT" != *'"title":"unused-let"'* ]] || [[ "$REVIEW_FORMAT_JSON_OUTPUT" != *'"code":"L0001"'* ]]; then
  echo "ERROR: embedded review --format json output mismatch"
  echo "$REVIEW_FORMAT_JSON_OUTPUT"
  exit 1
fi

DOC_ACK_OUTPUT="$(cd "$EMBED_SMOKE_DIR" && "$LSHARP_BIN" doc-ack review_input.ls)"
if [[ "$DOC_ACK_OUTPUT" != *"ack:recorded"* ]] || [[ "$DOC_ACK_OUTPUT" != *"module-global"* ]] || [[ "$DOC_ACK_OUTPUT" != *"functions:1,types:0,first-fn:main"* ]] || [[ "$DOC_ACK_OUTPUT" != *"Doc-Reviewed-By: anonymous"* ]]; then
  echo "ERROR: embedded doc-ack output mismatch"
  echo "$DOC_ACK_OUTPUT"
  exit 1
fi

DOC_ACK_TRAILER_OUTPUT="$(cd "$EMBED_SMOKE_DIR" && "$LSHARP_BIN" doc-ack review_input.ls --trailer)"
if [[ "$DOC_ACK_TRAILER_OUTPUT" != "; Doc-Reviewed-By: anonymous" ]]; then
  echo "ERROR: embedded doc-ack --trailer output mismatch"
  echo "$DOC_ACK_TRAILER_OUTPUT"
  exit 1
fi

DOC_CHECK_OUTPUT="$(cd "$EMBED_SMOKE_DIR" && "$LSHARP_BIN" doc-check review_input.ls)"
if [[ "$DOC_CHECK_OUTPUT" != *"status:ok"* ]] || [[ "$DOC_CHECK_OUTPUT" != *"module-global"* ]] || [[ "$DOC_CHECK_OUTPUT" != *"functions:1,types:0,first-fn:main"* ]] || [[ "$DOC_CHECK_OUTPUT" != *"Doc-Review-Status: Passed"* ]] || [[ "$DOC_CHECK_OUTPUT" != *"Doc-Reviewed-By: anonymous"* ]]; then
  echo "ERROR: embedded doc-check output mismatch"
  echo "$DOC_CHECK_OUTPUT"
  exit 1
fi

cat > "$EMBED_SMOKE_DIR/review_input_strict.ls" <<'EOF'
(defn main [] 42)
; Doc-Review-Status: Passed
; Doc-Reviewed-By: anonymous
EOF

DOC_CHECK_STRICT_OUTPUT="$(cd "$EMBED_SMOKE_DIR" && "$LSHARP_BIN" doc-check review_input_strict.ls --strict)"
if [[ "$DOC_CHECK_STRICT_OUTPUT" != *"status:ok"* ]] || [[ "$DOC_CHECK_STRICT_OUTPUT" != *"module-global"* ]] || [[ "$DOC_CHECK_STRICT_OUTPUT" != *"functions:1,types:0,first-fn:main"* ]] || [[ "$DOC_CHECK_STRICT_OUTPUT" != *"Doc-Review-Status: Passed"* ]] || [[ "$DOC_CHECK_STRICT_OUTPUT" != *"Doc-Reviewed-By: anonymous"* ]]; then
  echo "ERROR: embedded doc-check --strict output mismatch"
  echo "$DOC_CHECK_STRICT_OUTPUT"
  exit 1
fi

set +e
DOC_CHECK_STRICT_FAIL_OUTPUT="$(cd "$EMBED_SMOKE_DIR" && "$LSHARP_BIN" doc-check review_input.ls --strict 2>&1)"
DOC_CHECK_STRICT_FAIL_STATUS=$?
set -e
if [[ $DOC_CHECK_STRICT_FAIL_STATUS -eq 0 ]]; then
  echo "ERROR: embedded doc-check --strict unexpectedly accepted missing trailer"
  exit 1
fi
if [[ "$DOC_CHECK_STRICT_FAIL_OUTPUT" != *"error: invalid doc trailer: expected trailing comment lines"* ]]; then
  echo "ERROR: embedded doc-check --strict missing-trailer error mismatch"
  echo "$DOC_CHECK_STRICT_FAIL_OUTPUT"
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
DISABLE_REVIEW_OUTPUT="$(cd "$EMBED_SMOKE_DIR" && LSHARP_DISABLE_EMBEDDED_COMPONENT=1 "$LSHARP_BIN" review review_input.ls 2>&1)"
DISABLE_REVIEW_STATUS=$?
DISABLE_DOC_ACK_OUTPUT="$(cd "$EMBED_SMOKE_DIR" && LSHARP_DISABLE_EMBEDDED_COMPONENT=1 "$LSHARP_BIN" doc-ack review_input.ls 2>&1)"
DISABLE_DOC_ACK_STATUS=$?
DISABLE_DOC_ACK_TRAILER_OUTPUT="$(cd "$EMBED_SMOKE_DIR" && LSHARP_DISABLE_EMBEDDED_COMPONENT=1 "$LSHARP_BIN" doc-ack review_input.ls --trailer 2>&1)"
DISABLE_DOC_ACK_TRAILER_STATUS=$?
DISABLE_DOC_CHECK_OUTPUT="$(cd "$EMBED_SMOKE_DIR" && LSHARP_DISABLE_EMBEDDED_COMPONENT=1 "$LSHARP_BIN" doc-check review_input.ls 2>&1)"
DISABLE_DOC_CHECK_STATUS=$?
DISABLE_DOC_CHECK_STRICT_OUTPUT="$(cd "$EMBED_SMOKE_DIR" && LSHARP_DISABLE_EMBEDDED_COMPONENT=1 "$LSHARP_BIN" doc-check review_input_strict.ls --strict 2>&1)"
DISABLE_DOC_CHECK_STRICT_STATUS=$?
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
if [[ $DISABLE_REVIEW_STATUS -eq 0 ]]; then
  echo "ERROR: embedded disable flag unexpectedly allowed review"
  exit 1
fi
if [[ "$DISABLE_REVIEW_OUTPUT" != *"LSHARP_PATH"* ]]; then
  echo "ERROR: embedded disable flag did not restore review delegation hint"
  echo "$DISABLE_REVIEW_OUTPUT"
  exit 1
fi
if [[ $DISABLE_DOC_ACK_STATUS -eq 0 ]]; then
  echo "ERROR: embedded disable flag unexpectedly allowed doc-ack"
  exit 1
fi
if [[ "$DISABLE_DOC_ACK_OUTPUT" != *"LSHARP_PATH"* ]]; then
  echo "ERROR: embedded disable flag did not restore doc-ack delegation hint"
  echo "$DISABLE_DOC_ACK_OUTPUT"
  exit 1
fi
if [[ $DISABLE_DOC_ACK_TRAILER_STATUS -eq 0 ]]; then
  echo "ERROR: embedded disable flag unexpectedly allowed doc-ack --trailer"
  exit 1
fi
if [[ "$DISABLE_DOC_ACK_TRAILER_OUTPUT" != *"LSHARP_PATH"* ]]; then
  echo "ERROR: embedded disable flag did not restore doc-ack --trailer hint"
  echo "$DISABLE_DOC_ACK_TRAILER_OUTPUT"
  exit 1
fi
if [[ $DISABLE_DOC_CHECK_STATUS -eq 0 ]]; then
  echo "ERROR: embedded disable flag unexpectedly allowed doc-check"
  exit 1
fi
if [[ "$DISABLE_DOC_CHECK_OUTPUT" != *"LSHARP_PATH"* ]]; then
  echo "ERROR: embedded disable flag did not restore doc-check delegation hint"
  echo "$DISABLE_DOC_CHECK_OUTPUT"
  exit 1
fi
if [[ $DISABLE_DOC_CHECK_STRICT_STATUS -eq 0 ]]; then
  echo "ERROR: embedded disable flag unexpectedly allowed doc-check --strict"
  exit 1
fi
if [[ "$DISABLE_DOC_CHECK_STRICT_OUTPUT" != *"LSHARP_PATH"* ]]; then
  echo "ERROR: embedded disable flag did not restore doc-check --strict hint"
  echo "$DISABLE_DOC_CHECK_STRICT_OUTPUT"
  exit 1
fi

LSHARP_DISABLE_EMBEDDED_COMPONENT=1 "$LSHARP_BIN" compile selfhost/src/App/SmokeCli.ls -o "$EMBED_COMPONENT_ARTIFACT"
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
