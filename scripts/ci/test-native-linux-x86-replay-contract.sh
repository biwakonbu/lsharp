#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
SCRIPT="$ROOT/scripts/ci/native-linux-x86-hostgen-vm-exec.sh"

fail() {
  echo "FAIL: $*" >&2
  exit 1
}

assert_contains() {
  local expected="$1"
  grep -F -- "$expected" "$SCRIPT" >/dev/null \
    || fail "hostgen VM script does not contain: $expected"
}

[[ -s "$SCRIPT" ]] || fail "hostgen VM script is missing: $SCRIPT"

assert_contains 'LSHARP_NATIVE_LINUX_X86_FAIL_FAST_ON_OOM="${LSHARP_NATIVE_LINUX_X86_FAIL_FAST_ON_OOM:-1}"'
assert_contains 'if [[ "${chunk_exit_code}" -eq 137 && "${FAIL_FAST_ON_OOM}" = "1" ]]; then'
assert_contains 'source_commit = os.environ.get("LSHARP_NATIVE_LINUX_X86_SOURCE_COMMIT", "unknown")'
assert_contains '"source_commit": "{source_commit}",'

echo "native Linux x86 replay contract passed"
