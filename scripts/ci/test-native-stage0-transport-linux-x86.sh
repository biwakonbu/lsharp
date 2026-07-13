#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
DRIVER="$ROOT/scripts/ci/native-stage0-transport-linux-x86.sh"

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

assert_invocations() {
  local expected="$1"
  local actual
  actual="$(<"$INVOCATION_LOG")"
  assert_eq "$expected" "$actual"
}

reset_logs() {
  : >"$INVOCATION_LOG"
  : >"$HOST_LOG"
  : >"$TIMEOUT_LOG"
}

write_source() {
  local line_count="$1"
  local trailing_newline="$2"
  local idx=1

  : >"$SOURCE_ROOT/src/App/Seed.ls"
  while (( idx <= line_count )); do
    if (( idx == line_count && trailing_newline == 0 )); then
      printf '(line %s)' "$idx" >>"$SOURCE_ROOT/src/App/Seed.ls"
    else
      printf '(line %s)\n' "$idx" >>"$SOURCE_ROOT/src/App/Seed.ls"
    fi
    idx=$((idx + 1))
  done
}

run_driver() {
  local total="$1"
  local output="$2"

  NATIVE_STAGE0_TEST_SOURCE_ROOT="$SOURCE_ROOT" \
    NATIVE_STAGE0_TEST_INVOCATIONS="$INVOCATION_LOG" \
  NATIVE_STAGE0_TEST_HOST_LOG="$HOST_LOG" \
    NATIVE_STAGE0_TEST_TIMEOUT_LOG="$TIMEOUT_LOG" \
    NATIVE_STAGE0_FAKE_TOTAL="$total" \
    NATIVE_STAGE0_TRANSPORT_CHUNK_SIZE="${NATIVE_STAGE0_TRANSPORT_CHUNK_SIZE:-}" \
    NATIVE_STAGE0_TRANSPORT_TIMEOUT_SECONDS="${NATIVE_STAGE0_TRANSPORT_TIMEOUT_SECONDS:-}" \
    NATIVE_STAGE0_FAKE_FAIL_START="${NATIVE_STAGE0_FAKE_FAIL_START:-}" \
    NATIVE_STAGE0_FAKE_EMPTY_START="${NATIVE_STAGE0_FAKE_EMPTY_START:-}" \
    NATIVE_STAGE0_FAKE_TIMEOUT_START="${NATIVE_STAGE0_FAKE_TIMEOUT_START:-}" \
    NATIVE_STAGE0_FAKE_UNAME_SYSTEM="${NATIVE_STAGE0_FAKE_UNAME_SYSTEM:-}" \
    NATIVE_STAGE0_FAKE_UNAME_MACHINE="${NATIVE_STAGE0_FAKE_UNAME_MACHINE:-}" \
    PATH="$HOST_BIN:$PATH" \
    env "${HOST_CHECK_OVERRIDE[@]}" \
      "$DRIVER" "$COMPILER" "$SOURCE_ROOT" "src/App/Seed.ls" "$output"
}

run_driver_expect_failure() {
  local total="$1"
  local output="$2"
  local stderr_path="$3"
  local status=0

  set +e
  run_driver "$total" "$output" >"$TMP_ROOT/failure.stdout" 2>"$stderr_path"
  status=$?
  set -e
  [[ "$status" -ne 0 ]] || fail "driver unexpectedly succeeded"
}

[[ -x "$DRIVER" ]] || fail "native stage0 transport driver is missing or not executable: $DRIVER"

TMP_ROOT="$(mktemp -d "${TMPDIR:-/tmp}/lsharp-native-stage0-transport.XXXXXX")"
trap 'rm -rf "$TMP_ROOT"' EXIT

SOURCE_ROOT="$TMP_ROOT/source"
OUTSIDE_ROOT="$TMP_ROOT/outside"
COMPILER="$TMP_ROOT/compiler.native"
HOST_BIN="$TMP_ROOT/host-bin"
INVOCATION_LOG="$TMP_ROOT/invocations.log"
HOST_LOG="$TMP_ROOT/host.log"
TIMEOUT_LOG="$TMP_ROOT/timeout.log"

mkdir -p "$SOURCE_ROOT/src/App" "$OUTSIDE_ROOT" "$HOST_BIN"
SOURCE_ROOT="$(cd "$SOURCE_ROOT" && pwd -P)"
OUTSIDE_ROOT="$(cd "$OUTSIDE_ROOT" && pwd -P)"
: >"$INVOCATION_LOG"
: >"$HOST_LOG"
: >"$TIMEOUT_LOG"

cat >"$COMPILER" <<'SH'
#!/usr/bin/env bash
set -euo pipefail

[[ $# -eq 5 ]] || exit 90
entry="$1"
start="$2"
end="$3"
include_header="$4"
include_tail="$5"

[[ "$PWD" == "$NATIVE_STAGE0_TEST_SOURCE_ROOT" ]] || exit 91
[[ "$entry" == "src/App/Seed.ls" ]] || exit 92
[[ -f "$entry" ]] || exit 93
[[ "$start" =~ ^[0-9]+$ && "$end" =~ ^[0-9]+$ ]] || exit 94
[[ "$include_header" == "0" || "$include_header" == "1" ]] || exit 95
[[ "$include_tail" == "0" || "$include_tail" == "1" ]] || exit 96
printf '%s|%s|%s|%s|%s\n' "$entry" "$start" "$end" "$include_header" "$include_tail" >>"$NATIVE_STAGE0_TEST_INVOCATIONS"

if [[ "${NATIVE_STAGE0_FAKE_FAIL_START:-}" == "$start" ]]; then
  echo "fake compiler failure at range $start-$end" >&2
  exit 47
fi
if [[ "${NATIVE_STAGE0_FAKE_EMPTY_START:-}" == "$start" ]]; then
  exit 0
fi

if [[ "$include_header" == "1" ]]; then
  printf '9000000005\n%s\n10\n0\n9000000006\n9000000001\n%s\n9000000002\n' \
    "$NATIVE_STAGE0_FAKE_TOTAL" "$NATIVE_STAGE0_FAKE_TOTAL"
fi

index="$start"
while (( index < end && index < NATIVE_STAGE0_FAKE_TOTAL )); do
  printf '9000000010\n1\n%s\n' "$((index + 1))"
  index=$((index + 1))
done

if [[ "$include_tail" == "1" ]]; then
  printf '9000000003\n0\n9000000004\n'
fi
SH
chmod +x "$COMPILER"

for forbidden in cargo lsharp curl; do
  cat >"$HOST_BIN/$forbidden" <<'SH'
#!/usr/bin/env bash
set -euo pipefail
printf '%s\n' "$(basename "$0")" >>"$NATIVE_STAGE0_TEST_HOST_LOG"
exit 99
SH
  chmod +x "$HOST_BIN/$forbidden"
done

cat >"$HOST_BIN/uname" <<'SH'
#!/usr/bin/env bash
set -euo pipefail

if [[ -z "${NATIVE_STAGE0_FAKE_UNAME_SYSTEM:-}" ]]; then
  exec /usr/bin/uname "$@"
fi

case "$1" in
  -s) printf '%s\n' "$NATIVE_STAGE0_FAKE_UNAME_SYSTEM" ;;
  -m) printf '%s\n' "$NATIVE_STAGE0_FAKE_UNAME_MACHINE" ;;
  *) exit 98 ;;
esac
SH
chmod +x "$HOST_BIN/uname"

cat >"$HOST_BIN/timeout" <<'SH'
#!/usr/bin/env bash
set -euo pipefail

timeout_seconds="$1"
shift
entry="$2"
start="$3"
end="$4"
include_header="$5"
include_tail="$6"

printf '%s\n' "$timeout_seconds" >>"$NATIVE_STAGE0_TEST_TIMEOUT_LOG"
if [[ "${NATIVE_STAGE0_FAKE_TIMEOUT_START:-}" == "$start" ]]; then
  printf 'fake timeout at range %s-%s\n' "$start" "$end" >&2
  exit 124
fi

exec "$@"
SH
chmod +x "$HOST_BIN/timeout"

write_source 1 1
set +e
NATIVE_STAGE0_FAKE_UNAME_SYSTEM=Darwin \
  NATIVE_STAGE0_FAKE_UNAME_MACHINE=arm64 \
  PATH="$HOST_BIN:$PATH" \
  "$DRIVER" "$COMPILER" "$SOURCE_ROOT" "src/App/Seed.ls" "$TMP_ROOT/host-guard.transport" \
  >"$TMP_ROOT/host-guard.stdout" 2>"$TMP_ROOT/host-guard.stderr"
HOST_GUARD_STATUS=$?
set -e
[[ "$HOST_GUARD_STATUS" -ne 0 ]] || fail "driver accepted an unsupported host"
assert_file_contains "$TMP_ROOT/host-guard.stderr" "requires Linux x86_64"
HOST_CHECK_OVERRIDE=(LSHARP_NATIVE_STAGE0_TRANSPORT_TEST_ALLOW_UNSUPPORTED_HOST=1)

write_source 130 1
reset_logs
DEFAULT_OUTPUT="$TMP_ROOT/default.transport"
run_driver 130 "$DEFAULT_OUTPUT"
assert_invocations $'src/App/Seed.ls|0|64|1|0\nsrc/App/Seed.ls|64|128|0|0\nsrc/App/Seed.ls|128|130|0|1'
assert_eq "1" "$(grep -c '^9000000005$' "$DEFAULT_OUTPUT")"
assert_eq "1" "$(grep -c '^9000000003$' "$DEFAULT_OUTPUT")"
assert_eq "130" "$(grep -c '^9000000010$' "$DEFAULT_OUTPUT")"
assert_eq "1 2 3 4 5 6 7 8 9 10" "$(awk '$0 == "9000000010" { getline; getline; printf "%s ", $0 }' "$DEFAULT_OUTPUT" | awk '{$1=$1; print $0}' | cut -d ' ' -f 1-10)"
assert_eq "130" "$(awk '$0 == "9000000010" { getline; getline; value = $0 } END { print value }' "$DEFAULT_OUTPUT")"
assert_eq $'900\n900\n900' "$(<"$TIMEOUT_LOG")"
[[ ! -s "$HOST_LOG" ]] || fail "driver invoked a forbidden host command"

write_source 5 1
reset_logs
OVERRIDE_OUTPUT="$TMP_ROOT/override.transport"
NATIVE_STAGE0_TRANSPORT_CHUNK_SIZE=2 \
  NATIVE_STAGE0_TRANSPORT_TIMEOUT_SECONDS=17 \
  run_driver 5 "$OVERRIDE_OUTPUT"
assert_invocations $'src/App/Seed.ls|0|2|1|0\nsrc/App/Seed.ls|2|4|0|0\nsrc/App/Seed.ls|4|5|0|1'
assert_eq "5" "$(grep -c '^9000000010$' "$OVERRIDE_OUTPUT")"
assert_eq $'17\n17\n17' "$(<"$TIMEOUT_LOG")"

write_source 65 1
reset_logs
INVALID_OVERRIDE_OUTPUT="$TMP_ROOT/invalid-override.transport"
NATIVE_STAGE0_TRANSPORT_CHUNK_SIZE=invalid run_driver 65 "$INVALID_OVERRIDE_OUTPUT"
assert_invocations $'src/App/Seed.ls|0|64|1|0\nsrc/App/Seed.ls|64|65|0|1'
assert_eq $'900\n900' "$(<"$TIMEOUT_LOG")"

reset_logs
ZERO_OVERRIDE_OUTPUT="$TMP_ROOT/zero-override.transport"
NATIVE_STAGE0_TRANSPORT_CHUNK_SIZE=0 run_driver 65 "$ZERO_OVERRIDE_OUTPUT"
assert_invocations $'src/App/Seed.ls|0|64|1|0\nsrc/App/Seed.ls|64|65|0|1'
assert_eq $'900\n900' "$(<"$TIMEOUT_LOG")"

reset_logs
ZERO_TIMEOUT_OUTPUT="$TMP_ROOT/zero-timeout.transport"
NATIVE_STAGE0_TRANSPORT_TIMEOUT_SECONDS=0 run_driver 65 "$ZERO_TIMEOUT_OUTPUT"
assert_invocations $'src/App/Seed.ls|0|64|1|0\nsrc/App/Seed.ls|64|65|0|1'
assert_eq $'900\n900' "$(<"$TIMEOUT_LOG")"

reset_logs
INVALID_TIMEOUT_OUTPUT="$TMP_ROOT/invalid-timeout.transport"
NATIVE_STAGE0_TRANSPORT_TIMEOUT_SECONDS=invalid run_driver 65 "$INVALID_TIMEOUT_OUTPUT"
assert_invocations $'src/App/Seed.ls|0|64|1|0\nsrc/App/Seed.ls|64|65|0|1'
assert_eq $'900\n900' "$(<"$TIMEOUT_LOG")"

write_source 1 0
reset_logs
NO_NEWLINE_OUTPUT="$TMP_ROOT/no-newline.transport"
run_driver 65 "$NO_NEWLINE_OUTPUT"
assert_invocations $'src/App/Seed.ls|0|64|1|0\nsrc/App/Seed.ls|64|65|0|1'
assert_eq "65" "$(grep -c '^9000000010$' "$NO_NEWLINE_OUTPUT")"

write_source 5 1
reset_logs
FAILURE_OUTPUT="$TMP_ROOT/failure.transport"
printf 'preserved\n' >"$FAILURE_OUTPUT"
FAILURE_STDERR="$TMP_ROOT/failure.stderr"
NATIVE_STAGE0_TRANSPORT_CHUNK_SIZE=2 \
  NATIVE_STAGE0_FAKE_FAIL_START=2 \
  run_driver_expect_failure 5 "$FAILURE_OUTPUT" "$FAILURE_STDERR"
assert_invocations $'src/App/Seed.ls|0|2|1|0\nsrc/App/Seed.ls|2|4|0|0'
assert_eq $'900\n900' "$(<"$TIMEOUT_LOG")"
assert_eq "preserved" "$(<"$FAILURE_OUTPUT")"
assert_file_contains "$FAILURE_STDERR" "native compiler failed"
assert_file_not_contains "$HOST_LOG" "cargo"
assert_file_not_contains "$HOST_LOG" "lsharp"
assert_file_not_contains "$HOST_LOG" "curl"

reset_logs
NATIVE_STAGE0_TRANSPORT_CHUNK_SIZE=2 \
  run_driver 5 "$FAILURE_OUTPUT"
assert_invocations $'src/App/Seed.ls|2|4|0|0\nsrc/App/Seed.ls|4|5|0|1'
assert_eq "5" "$(grep -c '^9000000010$' "$FAILURE_OUTPUT")"
[[ ! -e "${FAILURE_OUTPUT}.resume" ]] || fail "completed transport left a resume checkpoint"

SOURCE_CHANGE_OUTPUT="$TMP_ROOT/source-change.transport"
printf 'preserved\n' >"$SOURCE_CHANGE_OUTPUT"
NATIVE_STAGE0_TRANSPORT_CHUNK_SIZE=2 \
  NATIVE_STAGE0_FAKE_FAIL_START=2 \
  run_driver_expect_failure 5 "$SOURCE_CHANGE_OUTPUT" "$TMP_ROOT/source-change.stderr"
write_source 6 1
reset_logs
NATIVE_STAGE0_TRANSPORT_CHUNK_SIZE=2 \
  run_driver 5 "$SOURCE_CHANGE_OUTPUT"
assert_invocations $'src/App/Seed.ls|0|2|1|0\nsrc/App/Seed.ls|2|4|0|0\nsrc/App/Seed.ls|4|5|0|1'
assert_eq "5" "$(grep -c '^9000000010$' "$SOURCE_CHANGE_OUTPUT")"
[[ ! -e "${SOURCE_CHANGE_OUTPUT}.resume" ]] || fail "source-changed transport left a resume checkpoint"

reset_logs
TIMEOUT_FAILURE_OUTPUT="$TMP_ROOT/timeout-failure.transport"
printf 'preserved\n' >"$TIMEOUT_FAILURE_OUTPUT"
TIMEOUT_FAILURE_STDERR="$TMP_ROOT/timeout-failure.stderr"
NATIVE_STAGE0_TRANSPORT_CHUNK_SIZE=2 \
  NATIVE_STAGE0_TRANSPORT_TIMEOUT_SECONDS=3 \
  NATIVE_STAGE0_FAKE_TIMEOUT_START=2 \
  run_driver_expect_failure 5 "$TIMEOUT_FAILURE_OUTPUT" "$TIMEOUT_FAILURE_STDERR"
assert_invocations 'src/App/Seed.ls|0|2|1|0'
assert_eq $'3\n3' "$(<"$TIMEOUT_LOG")"
assert_eq "preserved" "$(<"$TIMEOUT_FAILURE_OUTPUT")"
assert_file_contains "$TIMEOUT_FAILURE_STDERR" "native compiler failed"
assert_file_contains "$TIMEOUT_FAILURE_STDERR" "fake timeout"

reset_logs
EMPTY_OUTPUT="$TMP_ROOT/empty.transport"
EMPTY_STDERR="$TMP_ROOT/empty.stderr"
NATIVE_STAGE0_FAKE_EMPTY_START=0 \
  run_driver_expect_failure 5 "$EMPTY_OUTPUT" "$EMPTY_STDERR"
assert_invocations 'src/App/Seed.ls|0|64|1|0'
assert_file_contains "$EMPTY_STDERR" "empty output"

printf '(outside)\n' >"$OUTSIDE_ROOT/outside.ls"
reset_logs
OUTSIDE_STDERR="$TMP_ROOT/outside.stderr"
set +e
env "${HOST_CHECK_OVERRIDE[@]}" PATH="$HOST_BIN:$PATH" \
  "$DRIVER" "$COMPILER" "$SOURCE_ROOT" "../outside/outside.ls" "$TMP_ROOT/outside.transport" \
  >"$TMP_ROOT/outside.stdout" 2>"$OUTSIDE_STDERR"
OUTSIDE_STATUS=$?
set -e
[[ "$OUTSIDE_STATUS" -ne 0 ]] || fail "driver accepted a traversal entry"
assert_eq "" "$(<"$INVOCATION_LOG")"
assert_file_contains "$OUTSIDE_STDERR" "entry path must stay within source root"

ln -s "$OUTSIDE_ROOT/outside.ls" "$SOURCE_ROOT/src/App/escaped.ls"
reset_logs
SYMLINK_STDERR="$TMP_ROOT/symlink.stderr"
set +e
env "${HOST_CHECK_OVERRIDE[@]}" PATH="$HOST_BIN:$PATH" \
  "$DRIVER" "$COMPILER" "$SOURCE_ROOT" "src/App/escaped.ls" "$TMP_ROOT/symlink.transport" \
  >"$TMP_ROOT/symlink.stdout" 2>"$SYMLINK_STDERR"
SYMLINK_STATUS=$?
set -e
[[ "$SYMLINK_STATUS" -ne 0 ]] || fail "driver accepted an entry symlink outside source root"
assert_eq "" "$(<"$INVOCATION_LOG")"
assert_file_contains "$SYMLINK_STDERR" "entry path must stay within source root"

echo "native stage0 transport driver tests: OK"
