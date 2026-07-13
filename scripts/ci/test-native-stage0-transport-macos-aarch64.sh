#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
DRIVER="$ROOT/scripts/ci/native-stage0-transport-macos-aarch64.sh"

fail() {
  echo "FAIL: $*" >&2
  exit 1
}

assert_eq() {
  [[ "$1" == "$2" ]] || fail "expected '$1', got '$2'"
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

[[ -x "$DRIVER" ]] || fail "native stage0 transport driver is missing or not executable: $DRIVER"

TMP_ROOT="$(mktemp -d "${TMPDIR:-/tmp}/lsharp-native-stage0-transport-macos-aarch64.XXXXXX")"
TMP_ROOT="$(cd "$TMP_ROOT" && pwd -P)"
trap 'rm -rf "$TMP_ROOT"' EXIT

SOURCE_ROOT="$TMP_ROOT/source root"
ENTRY="src/App/Cli.ls"
COMPILER="$TMP_ROOT/compiler.native"
NON_EXECUTABLE_COMPILER="$TMP_ROOT/non-executable-compiler.native"
TRANSPORT_OUTPUT="$TMP_ROOT/transport-output.txt"
COMPILER_LOG="$TMP_ROOT/compiler.log"
COMPILER_INVOCATION_LOG="$TMP_ROOT/compiler-invocations.log"
HOST_BIN="$TMP_ROOT/host-bin"
HOST_TOOL_LOG="$TMP_ROOT/host-tools.log"

mkdir -p "$SOURCE_ROOT/src/App" "$HOST_BIN"
: >"$COMPILER_LOG"
: >"$COMPILER_INVOCATION_LOG"
: >"$HOST_TOOL_LOG"

cat >"$SOURCE_ROOT/$ENTRY" <<'LS'
(module App.Cli)
LS
cat >"$SOURCE_ROOT/src/Cli.ls" <<'LS'
(module Cli)
LS
cat >"$TMP_ROOT/outside.ls" <<'LS'
(module Outside)
LS

cat >"$COMPILER" <<'SH'
#!/usr/bin/env bash
set -euo pipefail

expected_source_root="${NATIVE_STAGE0_TRANSPORT_TEST_SOURCE_ROOT:?}"
expected_entry="${NATIVE_STAGE0_TRANSPORT_TEST_ENTRY:?}"
expected_output="${NATIVE_STAGE0_TRANSPORT_TEST_OUTPUT:?}"

printf 'invoked|%s|%s|%s\n' "$PWD" "$#" "${1:-}" \
  >>"${NATIVE_STAGE0_TRANSPORT_TEST_INVOCATION_LOG:?}"

[[ "$PWD" == "$expected_source_root" ]] || {
  printf 'unexpected current directory: %s\n' "$PWD" >&2
  exit 81
}
[[ $# -eq 1 ]] || {
  printf 'expected one source entry argument, got %s\n' "$#" >&2
  exit 82
}
[[ "$1" == "$expected_entry" ]] || {
  printf 'unexpected source entry argument: %s\n' "$1" >&2
  exit 83
}
[[ ! -e "$expected_output" ]] || {
  printf 'stale transport output was visible to compiler: %s\n' "$expected_output" >&2
  exit 84
}

printf 'cwd|%s\n' "$PWD" >>"${NATIVE_STAGE0_TRANSPORT_TEST_LOG:?}"
printf 'argc|%s\n' "$#" >>"${NATIVE_STAGE0_TRANSPORT_TEST_LOG:?}"
printf 'entry|%s\n' "$1" >>"${NATIVE_STAGE0_TRANSPORT_TEST_LOG:?}"

case "${NATIVE_STAGE0_TRANSPORT_TEST_MODE:-success}" in
  success)
    printf 'transport-payload\n'
    ;;
  stderr)
    printf 'compiler diagnostic\n' >&2
    printf 'transport-payload\n'
    ;;
  failure)
    printf 'compiler failed\n' >&2
    exit 85
    ;;
  empty)
    ;;
  *)
    printf 'unknown test mode: %s\n' "$NATIVE_STAGE0_TRANSPORT_TEST_MODE" >&2
    exit 86
    ;;
esac
SH
chmod +x "$COMPILER"

cat >"$NON_EXECUTABLE_COMPILER" <<'SH'
#!/usr/bin/env bash
exit 0
SH
chmod 0644 "$NON_EXECUTABLE_COMPILER"

for host_tool in cargo lsharp rustc curl wget; do
  cat >"$HOST_BIN/$host_tool" <<'SH'
#!/usr/bin/env bash
set -euo pipefail
printf 'host-tool|%s\n' "$(basename "$0")" >>"${NATIVE_STAGE0_TRANSPORT_TEST_HOST_LOG:?}"
exit 99
SH
  chmod +x "$HOST_BIN/$host_tool"
done

HOST_CHECK_OVERRIDE=()
if [[ "$(uname -s)" != "Darwin" || "$(uname -m)" != "arm64" ]]; then
  expect_reject "unsupported host without test override" \
    "$DRIVER" "$COMPILER" "$SOURCE_ROOT" "$ENTRY" "$TRANSPORT_OUTPUT"
  HOST_CHECK_OVERRIDE=(LSHARP_NATIVE_STAGE0_TRANSPORT_TEST_ALLOW_UNSUPPORTED_HOST=1)
fi

run_driver() {
  local mode="$1"
  shift

  env "${HOST_CHECK_OVERRIDE[@]}" \
    NATIVE_STAGE0_TRANSPORT_TEST_MODE="$mode" \
    NATIVE_STAGE0_TRANSPORT_TEST_SOURCE_ROOT="$SOURCE_ROOT" \
    NATIVE_STAGE0_TRANSPORT_TEST_ENTRY="$ENTRY" \
    NATIVE_STAGE0_TRANSPORT_TEST_OUTPUT="$TRANSPORT_OUTPUT" \
    NATIVE_STAGE0_TRANSPORT_TEST_LOG="$COMPILER_LOG" \
    NATIVE_STAGE0_TRANSPORT_TEST_INVOCATION_LOG="$COMPILER_INVOCATION_LOG" \
    NATIVE_STAGE0_TRANSPORT_TEST_HOST_LOG="$HOST_TOOL_LOG" \
    PATH="$HOST_BIN:$PATH" \
    "$DRIVER" "$@"
}

expect_reject "wrong argument count" \
  run_driver success "$COMPILER" "$SOURCE_ROOT" "$ENTRY"
expect_reject "missing compiler" \
  run_driver success "$TMP_ROOT/missing-compiler.native" "$SOURCE_ROOT" "$ENTRY" "$TRANSPORT_OUTPUT"
expect_reject "non-executable compiler" \
  run_driver success "$NON_EXECUTABLE_COMPILER" "$SOURCE_ROOT" "$ENTRY" "$TRANSPORT_OUTPUT"
expect_reject "missing source root" \
  run_driver success "$COMPILER" "$TMP_ROOT/missing-source" "$ENTRY" "$TRANSPORT_OUTPUT"
expect_reject "missing entry" \
  run_driver success "$COMPILER" "$SOURCE_ROOT" "src/App/Missing.ls" "$TRANSPORT_OUTPUT"
expect_reject "absolute entry" \
  run_driver success "$COMPILER" "$SOURCE_ROOT" "$SOURCE_ROOT/$ENTRY" "$TRANSPORT_OUTPUT"
expect_reject "parent traversal entry" \
  run_driver success "$COMPILER" "$SOURCE_ROOT" "../outside.ls" "$TRANSPORT_OUTPUT"
expect_reject "nested traversal entry" \
  run_driver success "$COMPILER" "$SOURCE_ROOT" "src/App/../Cli.ls" "$TRANSPORT_OUTPUT"
ln -s "$TMP_ROOT/outside.ls" "$SOURCE_ROOT/src/App/escaped.ls"
expect_reject "entry symlink outside source root" \
  run_driver success "$COMPILER" "$SOURCE_ROOT" "src/App/escaped.ls" "$TRANSPORT_OUTPUT"
assert_eq "" "$(cat "$COMPILER_INVOCATION_LOG")"

run_driver success "$COMPILER" "$SOURCE_ROOT" "$ENTRY" "$TRANSPORT_OUTPUT"
assert_eq "transport-payload" "$(cat "$TRANSPORT_OUTPUT")"
assert_file_contains "$COMPILER_LOG" "cwd|$SOURCE_ROOT"
assert_file_contains "$COMPILER_LOG" "argc|1"
assert_file_contains "$COMPILER_LOG" "entry|$ENTRY"

for mode in failure stderr empty; do
  printf 'stale transport output\n' >"$TRANSPORT_OUTPUT"
  expect_reject "$mode compiler output" \
    run_driver "$mode" "$COMPILER" "$SOURCE_ROOT" "$ENTRY" "$TRANSPORT_OUTPUT"
  [[ ! -e "$TRANSPORT_OUTPUT" ]] || fail "$mode compiler output retained stale transport output"
done

[[ ! -s "$HOST_TOOL_LOG" ]] || fail "transport driver invoked a host tool"

echo "native stage0 transport macos aarch64 tests: OK"
