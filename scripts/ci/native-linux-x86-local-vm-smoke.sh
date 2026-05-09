#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
ARTIFACT_ID="${NATIVE_LINUX_X86_LOCAL_ARTIFACT_ID:-local}"
ARTIFACT_DIR_INPUT="${LSHARP_NATIVE_LINUX_X86_LOCAL_ARTIFACT_DIR:-ci-artifacts/native-linux-x86-local/${ARTIFACT_ID}}"

if [[ "${ARTIFACT_DIR_INPUT}" = /* ]]; then
  if [[ "${ARTIFACT_DIR_INPUT}" != "${ROOT_DIR}"/* ]]; then
    echo "ERROR: LSHARP_NATIVE_LINUX_X86_LOCAL_ARTIFACT_DIR must be under repository root: ${ARTIFACT_DIR_INPUT}" >&2
    exit 1
  fi
  ARTIFACT_DIR="${ARTIFACT_DIR_INPUT}"
else
  ARTIFACT_DIR="${ROOT_DIR}/${ARTIFACT_DIR_INPUT}"
fi

cd "${ROOT_DIR}"

HOST_OS="$(uname -s)"
HOST_ARCH="$(uname -m)"
if [[ "${HOST_OS}" != "Linux" || "${HOST_ARCH}" != "x86_64" ]]; then
  echo "ERROR: native-linux-x86-local-vm-smoke.sh requires Linux/x86_64; got ${HOST_OS}/${HOST_ARCH}" >&2
  exit 1
fi

rm -rf "${ARTIFACT_DIR}"
mkdir -p "${ARTIFACT_DIR}"

echo "=== native Linux x86_64 local VM smoke ==="
echo "artifact dir: ${ARTIFACT_DIR}"
echo "scope: fast local VM descriptor / ELF / runtime-link smoke; selfhost exact-byte suites stay out of the local inner loop."

cargo test -q -p lsharp-wasm --test e2e \
  e2e::selfhost_gc_runtime_bootstrap::test_e2e_selfhost_native_target_descriptors \
  -- --exact
cargo test -q -p lsharp-wasm --test e2e \
  e2e::selfhost_gc_runtime_bootstrap::test_e2e_selfhost_native_object_emitter \
  -- --exact
cargo test -q -p lsharp-wasm --test e2e \
  e2e::selfhost_native_stage_chain::test_e2e_native_linux_x86_const_42_link_and_execute \
  -- --exact --ignored

RUNTIME_DIR="${ARTIFACT_DIR}/runtime-link"
mkdir -p "${RUNTIME_DIR}"

cat >"${RUNTIME_DIR}/program.s" <<'ASM'
.text
.globl generated
generated:
    mov $42, %eax
    ret

.globl main
main:
    call generated
    ret

.section .note.GNU-stack,"",@progbits
ASM

cat >"${RUNTIME_DIR}/runtime.s" <<'ASM'
.text
.globl lsharp_runtime_stub
lsharp_runtime_stub:
    ret

.section .note.GNU-stack,"",@progbits
ASM

cc -c "${RUNTIME_DIR}/program.s" -o "${RUNTIME_DIR}/program.o"
cc -c "${RUNTIME_DIR}/runtime.s" -o "${RUNTIME_DIR}/runtime.o"

cat >"${RUNTIME_DIR}/linker-response.txt" <<'EOF_RESPONSE'
-o
program.native
program.o
runtime.o
EOF_RESPONSE

(
  cd "${RUNTIME_DIR}"
  cc @linker-response.txt
)

set +e
"${RUNTIME_DIR}/program.native" >"${RUNTIME_DIR}/stdout.txt" 2>"${RUNTIME_DIR}/stderr.txt"
exit_code=$?
set -e

expected_exit_code=42
if [[ "${exit_code}" -ne "${expected_exit_code}" ]]; then
  echo "ERROR: program.native exit_code=${exit_code}, expected ${expected_exit_code}" >&2
  exit 1
fi

cat >"${ARTIFACT_DIR}/summary.json" <<JSON
{
  "target": "x86_64-unknown-linux-gnu",
  "host_os": "${HOST_OS}",
  "host_arch": "${HOST_ARCH}",
  "status": "pass",
  "scope": "fast local VM descriptor / ELF / runtime-link smoke",
  "expected_exit_code": ${expected_exit_code},
  "actual_exit_code": ${exit_code},
  "canonical_files": [
    "program.o",
    "runtime.o",
    "linker-response.txt",
    "program.native"
  ],
  "actual_self_regeneration": "pending"
}
JSON

echo "native Linux x86_64 local VM smoke evidence collected."
