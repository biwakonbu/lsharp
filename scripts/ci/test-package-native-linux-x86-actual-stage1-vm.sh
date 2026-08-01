#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
WRAPPER="$ROOT/scripts/ci/package-native-linux-x86-actual-stage1-vm.sh"
SOURCE_COMMIT="$(git -C "$ROOT" rev-parse --verify HEAD)"

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
STALE_STAGE1="$TMP_ROOT/stale-stage1"
OUTPUT_DIR="$TMP_ROOT/stage0"
STALE_OUTPUT_DIR="$TMP_ROOT/stale-stage0"
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
cat >"$ACTUAL_STAGE1/manifest.json" <<JSON
{
  "target": "x86_64-unknown-linux-gnu",
  "source_commit": "$SOURCE_COMMIT",
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
if [[ "$#" -eq 0 ]]; then
  printf 'fake-stage1-compiler\n'
  exit 0
fi
[[ "$#" -eq 5 ]] || exit 92
case "$2:$3:$4:$5" in
  0:64:1:0)
    cat <<'TRANSPORT'
9000000005
1
10
0
9000000006
9000000001
43
9000000002
491481697616312
9026096594944
364510094841526784
7019251490299464131
8314605285929872999
667706
TRANSPORT
    ;;
  1:1:0:1)
    printf '9000000003\n0\n9000000004\n'
    ;;
  *)
    printf 'unexpected compiler range: %s\n' "$*" >&2
    exit 93
    ;;
esac
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

cat >"$HOST_BIN/timeout" <<'SH'
#!/usr/bin/env bash
set -euo pipefail
[[ $# -ge 2 ]] || exit 97
shift
exec "$@"
SH
chmod +x "$HOST_BIN/timeout"

cat >"$HOST_BIN/cc" <<'SH'
#!/usr/bin/env bash
set -euo pipefail

if [[ "${1:-}" == "-c" ]]; then
  output=""
  for ((index = 1; index < $#; index += 1)); do
    if [[ "${!index}" == "-o" ]]; then
      next=$((index + 1))
      output="${!next}"
      break
    fi
  done
  [[ -n "$output" ]] || exit 94
  : >"$output"
  exit 0
fi

[[ "${1:-}" == "@linker-response.txt" ]] || exit 95
cat >program.native <<'PROGRAM'
#!/usr/bin/env bash
set -euo pipefail
printf 'Int\ndiagnostics:0\n'
PROGRAM
chmod +x program.native
SH
chmod +x "$HOST_BIN/cc"

run_wrapper() {
  PATH="$HOST_BIN:$PATH" \
    LSHARP_NATIVE_LINUX_X86_VM_NAME="$VM_NAME" \
    LSHARP_NATIVE_LINUX_X86_STAGE0_PACKAGE_TEST_LOG="$LOG" \
    "$WRAPPER" "$@"
}

expect_reject "missing stage1 artifact" run_wrapper \
  --actual-stage1-dir "$INVALID_STAGE1" \
  --output-dir "$TMP_ROOT/missing-stage1-output"

cp -a "$ACTUAL_STAGE1" "$STALE_STAGE1"
python3 - "$STALE_STAGE1/manifest.json" <<'PY'
import json
import sys

manifest_path = sys.argv[1]
manifest = json.load(open(manifest_path, encoding="utf-8"))
manifest["source_commit"] = "0" * 40
with open(manifest_path, "w", encoding="utf-8") as handle:
    json.dump(manifest, handle, indent=2)
    handle.write("\n")
PY

expect_reject "stale stage1 artifact" run_wrapper \
  --actual-stage1-dir "$STALE_STAGE1" \
  --output-dir "$STALE_OUTPUT_DIR"

run_wrapper \
  --actual-stage1-dir "$ACTUAL_STAGE1" \
  --output-dir "$OUTPUT_DIR"

python3 - "$OUTPUT_DIR/manifest.json" "$SOURCE_COMMIT" <<'PY'
import json
import sys

manifest = json.load(open(sys.argv[1], encoding="utf-8"))
expected = {
    "kind": "lsharp-native-selfhost-stage0",
    "target": "x86_64-unknown-linux-gnu",
    "source_commit": sys.argv[2],
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

RUNNER_SOURCE="$TMP_ROOT/runner-source"
RUNNER_STAGE="$TMP_ROOT/runner-stage"
RUNNER_INPUT="$RUNNER_SOURCE/input.ls"
mkdir -p "$RUNNER_SOURCE/src/App"
printf '(module App.Cli)\n' >"$RUNNER_SOURCE/src/App/Cli.ls"
printf '(defn main [] 42)\n' >"$RUNNER_INPUT"

runner_stderr="$TMP_ROOT/runner.stderr"
runner_output="$TMP_ROOT/runner.stdout"
set +e
(
  cd "$ROOT"
  PATH="$HOST_BIN:$PATH" \
    LSHARP_NATIVE_STAGE0_TRANSPORT_TEST_ALLOW_UNSUPPORTED_HOST=1 \
    NATIVE_STAGE0_DIR="$OUTPUT_DIR" \
    NATIVE_SOURCE_ROOT="$RUNNER_SOURCE" \
    NATIVE_STAGE_DIR="$RUNNER_STAGE" \
    "$ROOT/scripts/native-selfhost-dev.sh" check "$RUNNER_INPUT"
) >"$runner_output" 2>"$runner_stderr"
runner_status=$?
set -e
[[ "$runner_status" -eq 0 ]] || fail "packaged stage0 runner failed with exit=$runner_status: $(cat "$runner_stderr")"
[[ -z "$(cat "$runner_stderr")" ]] || fail "packaged stage0 runner emitted stderr: $(cat "$runner_stderr")"
grep -Fx 'Int' "$runner_output" >/dev/null || fail "packaged stage0 runner output is missing Int"
grep -Fx 'diagnostics:0' "$runner_output" >/dev/null \
  || fail "packaged stage0 runner output is missing diagnostics:0"
[[ -x "$RUNNER_STAGE/program.native" ]] || fail "packaged stage0 runner did not materialize program.native"
! grep -F -- 'forbidden|' "$LOG" >/dev/null || fail "packaged stage0 runner invoked a forbidden host tool"

echo "Linux x86 actual-stage1 stage0 package tests: OK"
