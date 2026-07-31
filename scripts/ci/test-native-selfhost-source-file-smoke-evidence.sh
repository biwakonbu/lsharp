#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
SOURCE_SMOKE="$ROOT/scripts/ci/native-selfhost-dev-source-file-smoke.sh"
WRITER="$ROOT/scripts/ci/write-native-source-smoke-evidence.py"

fail() {
  echo "FAIL: $*" >&2
  exit 1
}

assert_file_contains() {
  local path="$1"
  local expected="$2"
  grep -F -- "$expected" "$path" >/dev/null || fail "$path does not contain: $expected"
}

[[ -f "$SOURCE_SMOKE" ]] || fail "source smoke is missing: $SOURCE_SMOKE"
[[ -f "$WRITER" ]] || fail "source smoke evidence writer is missing: $WRITER"
assert_file_contains "$SOURCE_SMOKE" 'NATIVE_SELFHOST_SOURCE_SMOKE_EVIDENCE_DIR'
assert_file_contains "$SOURCE_SMOKE" 'write-native-source-smoke-evidence.py'
python3 - "$SOURCE_SMOKE" <<'PY'
import pathlib
import sys

source = pathlib.Path(sys.argv[1]).read_text(encoding="utf-8")
broken = 'python3 \\\n  "$VALIDATION_ATTESTATION_SOURCE"'
if broken in source:
    raise SystemExit("attestation fixture generator must pass '-' to python3 stdin")
PY

TMP_ROOT="$(mktemp -d "${TMPDIR:-/tmp}/lsharp-native-source-smoke-evidence.XXXXXX")"
trap 'rm -rf "$TMP_ROOT"' EXIT

WORK_DIR="$TMP_ROOT/work"
EVIDENCE_DIR="$TMP_ROOT/evidence"
STAGE0_MANIFEST="$TMP_ROOT/stage0-manifest.json"
mkdir -p "$WORK_DIR"
printf 'decls:1\n' >"$WORK_DIR/parse.stdout"
printf 'compile-bytes' >"$WORK_DIR/compile.wasm"
printf 'build-bytes' >"$WORK_DIR/build.wasm"
cat >"$STAGE0_MANIFEST" <<'JSON'
{
  "kind": "lsharp-native-selfhost-stage0",
  "target": "aarch64-apple-darwin",
  "source_commit": "0123456789012345678901234567890123456789"
}
JSON

python3 "$WRITER" \
  --evidence-dir "$EVIDENCE_DIR" \
  --work-dir "$WORK_DIR" \
  --stage0-manifest "$STAGE0_MANIFEST" \
  --target aarch64-apple-darwin \
  --exit-code 7

[[ "$(cat "$EVIDENCE_DIR/exit.code")" == "7" ]] || fail "exit code evidence was not preserved"
[[ "$(cat "$EVIDENCE_DIR/work/parse.stdout")" == "decls:1" ]] || fail "stdout evidence was not preserved"
cmp -s "$WORK_DIR/compile.wasm" "$EVIDENCE_DIR/work/compile.wasm" \
  || fail "Wasm evidence was not preserved"
[[ -s "$EVIDENCE_DIR/stage0-manifest.json" ]] || fail "stage0 manifest evidence is missing"

python3 - "$EVIDENCE_DIR/manifest.json" <<'PY'
import json
import pathlib
import sys

manifest = json.loads(pathlib.Path(sys.argv[1]).read_text(encoding="utf-8"))
assert manifest["kind"] == "lsharp-native-selfhost-source-smoke-evidence"
assert manifest["target"] == "aarch64-apple-darwin"
assert manifest["source_commit"] == "0123456789012345678901234567890123456789"
assert manifest["exit_code"] == 7
assert manifest["artifacts"]["compile.wasm"]["size"] == len(b"compile-bytes")
PY

if python3 "$WRITER" \
  --evidence-dir "$EVIDENCE_DIR" \
  --work-dir "$WORK_DIR" \
  --stage0-manifest "$STAGE0_MANIFEST" \
  --target aarch64-apple-darwin \
  --exit-code 0; then
  fail "evidence writer overwrote an existing evidence directory"
fi

echo "native source smoke evidence contract test passed"
