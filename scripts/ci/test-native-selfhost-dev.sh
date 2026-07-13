#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
RUNNER="$ROOT/scripts/native-selfhost-dev.sh"

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

assert_file_not_contains() {
  local path="$1"
  local unexpected="$2"
  ! grep -F -- "$unexpected" "$path" >/dev/null || fail "$path unexpectedly contains: $unexpected"
}

[[ -x "$RUNNER" ]] || fail "native selfhost dev runner is missing or not executable: $RUNNER"

TMP_ROOT="$(mktemp -d "${TMPDIR:-/tmp}/lsharp-native-selfhost-dev.XXXXXX")"
trap 'rm -rf "$TMP_ROOT"' EXIT

TEST_ROOT="$TMP_ROOT/repo"
STAGE0_DIR="$TMP_ROOT/stage0"
SOURCE_ROOT="$TMP_ROOT/source"
STAGE_DIR="$TMP_ROOT/stage"
LOG_FILE="$TMP_ROOT/invocations.log"
HOST_BIN="$TMP_ROOT/host-bin"

mkdir -p "$TEST_ROOT/scripts/ci" "$STAGE0_DIR/bin" "$SOURCE_ROOT/src/App" "$HOST_BIN"
cp "$RUNNER" "$TEST_ROOT/scripts/native-selfhost-dev.sh"
chmod +x "$TEST_ROOT/scripts/native-selfhost-dev.sh"
cp "$ROOT/scripts/ci/decode-native-selfhost-transport.py" \
  "$TEST_ROOT/scripts/ci/decode-native-selfhost-transport.py"
chmod +x "$TEST_ROOT/scripts/ci/decode-native-selfhost-transport.py"

cat >"$STAGE0_DIR/manifest.json" <<'JSON'
{
  "kind": "lsharp-native-selfhost-stage0",
  "target": "x86_64-unknown-linux-gnu",
  "compiler": "bin/compiler",
  "transport_driver": "bin/transport-driver",
  "materializer": "bin/materializer"
}
JSON

cat >"$STAGE0_DIR/bin/compiler" <<'SH'
#!/usr/bin/env bash
set -euo pipefail
printf 'compiler|%s\n' "$*" >>"$NATIVE_TEST_LOG"
SH
chmod +x "$STAGE0_DIR/bin/compiler"

cat >"$STAGE0_DIR/bin/transport-driver" <<'SH'
#!/usr/bin/env bash
set -euo pipefail
[[ $# -eq 4 ]] || exit 91
compiler="$1"
source_root="$2"
entry="$3"
transport_output="$4"
[[ "$source_root" == */source ]] || exit 92
[[ -f "$source_root/$entry" ]] || exit 93
"$compiler" "$source_root" "$entry"
printf 'transport|%s|%s\n' "$source_root" "$entry" >>"$NATIVE_TEST_LOG"
cat >"$transport_output" <<'TRANSPORT'
9000000005
0
10
0
9000000006
9000000001
1
9000000002
0
9000000003
0
9000000004
TRANSPORT
SH
chmod +x "$STAGE0_DIR/bin/transport-driver"

cat >"$STAGE0_DIR/bin/materializer" <<'SH'
#!/usr/bin/env bash
set -euo pipefail
[[ $# -eq 3 ]] || exit 94
stage_dir="$1"
[[ -s "$2" ]] || exit 95
[[ -f "$3" ]] || exit 96
[[ "${LSHARP_NATIVE_LINUX_X86_SKIP_ARGV0:-}" == "1" ]] || exit 99
printf 'materializer|%s\n' "$stage_dir" >>"$NATIVE_TEST_LOG"
cat >"$stage_dir/program.native" <<'PROGRAM'
#!/usr/bin/env bash
set -euo pipefail
[[ -z "${LSHARP_PATH+x}" ]] || exit 97
[[ -z "${LSHARP_DISABLE_EMBEDDED_COMPONENT+x}" ]] || exit 98
printf 'program|%s\n' "$*" >>"$NATIVE_TEST_LOG"
PROGRAM
chmod +x "$stage_dir/program.native"
SH
chmod +x "$STAGE0_DIR/bin/materializer"

cat >"$SOURCE_ROOT/src/App/Cli.ls" <<'LS'
(module App.Cli)
LS

cat >"$HOST_BIN/cargo" <<'SH'
#!/usr/bin/env bash
printf 'host-cargo\n' >>"$NATIVE_TEST_LOG"
exit 99
SH
chmod +x "$HOST_BIN/cargo"

cat >"$HOST_BIN/lsharp" <<'SH'
#!/usr/bin/env bash
printf 'host-lsharp\n' >>"$NATIVE_TEST_LOG"
exit 99
SH
chmod +x "$HOST_BIN/lsharp"

run_runner() {
  NATIVE_TEST_LOG="$LOG_FILE" \
    LSHARP_PATH="$HOST_BIN/lsharp" \
    LSHARP_DISABLE_EMBEDDED_COMPONENT=1 \
    PATH="$HOST_BIN:$PATH" \
    "$TEST_ROOT/scripts/native-selfhost-dev.sh" \
      --stage0-dir "$STAGE0_DIR" \
      --source-root "$SOURCE_ROOT" \
      --stage-dir "$STAGE_DIR" \
      "$@"
}

run_runner alpha beta
assert_eq "1" "$(grep -c '^transport|' "$LOG_FILE")"
assert_eq "1" "$(grep -c '^materializer|' "$LOG_FILE")"
assert_file_contains "$LOG_FILE" "program|alpha beta"
assert_file_not_contains "$LOG_FILE" "host-cargo"
assert_file_not_contains "$LOG_FILE" "host-lsharp"

run_runner reuse
assert_eq "1" "$(grep -c '^transport|' "$LOG_FILE")"
assert_file_contains "$LOG_FILE" "program|reuse"

printf '\n;; source refresh\n' >>"$SOURCE_ROOT/src/App/Cli.ls"
run_runner changed
assert_eq "2" "$(grep -c '^transport|' "$LOG_FILE")"
assert_file_contains "$LOG_FILE" "program|changed"

run_runner --bootstrap bootstrap
assert_eq "3" "$(grep -c '^transport|' "$LOG_FILE")"
assert_file_contains "$LOG_FILE" "program|bootstrap"

assert_file_not_contains "$RUNNER" 'cargo '
assert_file_not_contains "$RUNNER" 'command -v lsharp'
assert_file_not_contains "$RUNNER" 'which lsharp'

echo "native selfhost dev runner tests: OK"
