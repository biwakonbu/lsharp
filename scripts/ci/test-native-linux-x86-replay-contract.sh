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
assert_contains 'mov %rax, %r14'
assert_contains 'mov %rcx, 8(%r14)'
assert_contains 'mov $8192, %rcx'
assert_contains 'mov %rcx, (%r14)'
assert_contains 'OBJECT_ONLY="${LSHARP_NATIVE_LINUX_X86_OBJECT_ONLY:-0}"'
assert_contains 'if [[ "${OBJECT_ONLY}" = "1" ]]; then'
assert_contains 'MAP_REF_OBJECT_ARTIFACT="${ARTIFACT_DIR}/map-ref-program.o"'
assert_contains 'test_e2e_native_linux_x86_host_generates_map_ref_get_elf_object_artifact'
assert_contains 'scope": "host-side selfhost-generated ELF object preserves map-new across ref-new/ref-get'

echo "native Linux x86 replay contract passed"
