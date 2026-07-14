#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
WRAPPER="$ROOT/scripts/ci/package-native-linux-x86-actual-stage1-vm.sh"

fail() {
  echo "FAIL: $*" >&2
  exit 1
}

assert_file_contains() {
  local path="$1"
  local expected="$2"
  grep -F -- "$expected" "$path" >/dev/null || fail "$path does not contain: $expected"
}

expect_reject() {
  local label="$1"
  shift

  local output
  local status
  set +e
  output="$("$@" 2>&1)"
  status=$?
  set -e
  [[ "$status" -ne 0 ]] || fail "$label unexpectedly succeeded"
  [[ -n "$output" ]] || fail "$label did not report an error"
}

[[ -x "$WRAPPER" ]] || fail "Linux x86 actual-stage1 package wrapper is missing or not executable: $WRAPPER"

TMP_ROOT="$(mktemp -d "${TMPDIR:-/tmp}/lsharp-stage1-package-vm.XXXXXX")"
trap 'rm -rf "$TMP_ROOT"' EXIT

ACTUAL_STAGE1="$TMP_ROOT/actual-stage1"
INVALID_STAGE1="$TMP_ROOT/invalid-stage1"
OUTPUT_DIR="$TMP_ROOT/stage0"
HOST_BIN="$TMP_ROOT/host-bin"
LOG="$TMP_ROOT/invocations.log"
VM_NAME="lsharp-linux-x86-test"

mkdir -p "$ACTUAL_STAGE1" "$INVALID_STAGE1" "$HOST_BIN"
: >"$LOG"

printf '\303' >"$ACTUAL_STAGE1/stage1-code.bin"
printf '\0' >"$ACTUAL_STAGE1/stage1-data.bin"
printf '0\n' >"$ACTUAL_STAGE1/entrypoint-offset.txt"
printf '1\n' >"$ACTUAL_STAGE1/function-start-len.txt"
printf '10\n' >"$ACTUAL_STAGE1/main-func-idx.txt"
printf '(module App.Seed)\n' >"$ACTUAL_STAGE1/seed.ls"
cat >"$ACTUAL_STAGE1/manifest.json" <<'JSON'
{
  "target": "x86_64-unknown-linux-gnu",
  "code_len": 1,
  "data_len": 1,
  "entrypoint_offset": 0,
  "function_start_len": 1,
  "main_func_idx": 10
}
JSON

cat >"$HOST_BIN/limactl" <<'SH'
#!/usr/bin/env bash
set -euo pipefail

log="${LSHARP_NATIVE_LINUX_X86_STAGE0_PACKAGE_TEST_LOG:?}"

case "$1" in
  list)
    printf 'list|%s\n' "$*" >>"$log"
    printf 'Running\n'
    ;;
  shell)
    vm="$2"
    shift 3
    printf 'shell|%s|%s\n' "$vm" "$*" >>"$log"
    ;;
  copy)
    shift
    if [[ "${1:-}" == "--recursive" ]]; then
      shift
    fi
    source_path="$1"
    destination_path="$2"
    printf 'copy|%s|%s\n' "$source_path" "$destination_path" >>"$log"
    if [[ "$source_path" == *:* ]]; then
      mkdir -p "$(dirname "$destination_path")"
      cat >"$destination_path" <<'PROGRAM'
#!/usr/bin/env bash
set -euo pipefail
printf 'fake-stage1-compiler\n'
PROGRAM
      chmod +x "$destination_path"
    fi
    ;;
  *)
    printf 'unexpected limactl invocation: %s\n' "$*" >&2
    exit 91
    ;;
esac
SH
chmod +x "$HOST_BIN/limactl"

for forbidden in cargo lsharp rustc; do
  cat >"$HOST_BIN/$forbidden" <<'SH'
#!/usr/bin/env bash
set -euo pipefail
printf 'forbidden|%s\n' "$(basename "$0")" >>"${LSHARP_NATIVE_LINUX_X86_STAGE0_PACKAGE_TEST_LOG:?}"
exit 99
SH
  chmod +x "$HOST_BIN/$forbidden"
done

run_wrapper() {
  PATH="$HOST_BIN:$PATH" \
    LSHARP_NATIVE_LINUX_X86_VM_NAME="$VM_NAME" \
    LSHARP_NATIVE_LINUX_X86_STAGE0_PACKAGE_TEST_LOG="$LOG" \
    "$WRAPPER" "$@"
}

expect_reject "missing stage1 artifact" run_wrapper \
  --actual-stage1-dir "$INVALID_STAGE1" \
  --output-dir "$TMP_ROOT/missing-stage1-output"

run_wrapper \
  --actual-stage1-dir "$ACTUAL_STAGE1" \
  --output-dir "$OUTPUT_DIR"

python3 - "$OUTPUT_DIR/manifest.json" <<'PY'
import json
import sys

manifest = json.load(open(sys.argv[1], encoding="utf-8"))
expected = {
    "kind": "lsharp-native-selfhost-stage0",
    "target": "x86_64-unknown-linux-gnu",
    "compiler": "bin/compiler",
    "transport_driver": "bin/transport-driver",
    "materializer": "bin/materializer",
}
if manifest != expected:
    raise SystemExit(f"unexpected manifest: {manifest!r}")
PY

for executable in \
  "$OUTPUT_DIR/bin/compiler" \
  "$OUTPUT_DIR/bin/transport-driver" \
  "$OUTPUT_DIR/bin/materializer"; do
  [[ -x "$executable" ]] || fail "packaged executable is unavailable: $executable"
done
[[ -s "$OUTPUT_DIR/bin/materializer.py" ]] || fail "packaged materializer is unavailable"

assert_file_contains "$LOG" "stage1-code.bin entrypoint-offset.txt"
assert_file_contains "$LOG" "program.native"
! grep -F -- 'forbidden|' "$LOG" >/dev/null || fail "wrapper invoked a forbidden host tool"

compiler_output="$($OUTPUT_DIR/bin/compiler)"
[[ "$compiler_output" == "fake-stage1-compiler" ]] || fail "packaged compiler did not come from VM materialization"

echo "Linux x86 actual-stage1 stage0 package tests: OK"
