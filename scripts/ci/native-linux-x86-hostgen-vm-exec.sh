#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
ARTIFACT_ID="${NATIVE_LINUX_X86_HOSTGEN_VM_ARTIFACT_ID:-local}"
ARTIFACT_DIR_INPUT="${LSHARP_NATIVE_LINUX_X86_HOSTGEN_VM_ARTIFACT_DIR:-ci-artifacts/native-linux-x86-hostgen-vm/${ARTIFACT_ID}}"
VM_NAME="${LSHARP_NATIVE_LINUX_X86_VM_NAME:-lsharp-linux-x86}"
VM_WORK_DIR="${LSHARP_NATIVE_LINUX_X86_VM_WORK_DIR:-/tmp/lsharp-native-linux-x86-hostgen-vm-${ARTIFACT_ID}}"

if [[ "${ARTIFACT_DIR_INPUT}" = /* ]]; then
  if [[ "${ARTIFACT_DIR_INPUT}" != "${ROOT_DIR}"/* ]]; then
    echo "ERROR: LSHARP_NATIVE_LINUX_X86_HOSTGEN_VM_ARTIFACT_DIR must be under repository root: ${ARTIFACT_DIR_INPUT}" >&2
    exit 1
  fi
  ARTIFACT_DIR="${ARTIFACT_DIR_INPUT}"
else
  ARTIFACT_DIR="${ROOT_DIR}/${ARTIFACT_DIR_INPUT}"
fi

cd "${ROOT_DIR}"

if ! command -v limactl >/dev/null 2>&1; then
  echo "ERROR: limactl is required for hostgen->VM Linux x86_64 native execution smoke" >&2
  exit 1
fi

rm -rf "${ARTIFACT_DIR}"
mkdir -p "${ARTIFACT_DIR}"

CODE_ARTIFACT="${ARTIFACT_DIR}/code.bin"

echo "=== native Linux x86_64 hostgen -> VM exec smoke ==="
echo "artifact dir: ${ARTIFACT_DIR}"
echo "VM: ${VM_NAME}"
echo "scope: host-side selfhost-generated Linux x86_64 code artifact linked and executed inside local VM."

LSHARP_NATIVE_LINUX_X86_CODE_ARTIFACT="${CODE_ARTIFACT}" \
  cargo test -q -p lsharp-wasm --test e2e \
  e2e::selfhost_native_stage_chain::test_e2e_native_linux_x86_host_generates_const_42_code_artifact \
  -- --exact --ignored

if [[ ! -s "${CODE_ARTIFACT}" ]]; then
  echo "ERROR: host code artifact was not generated: ${CODE_ARTIFACT}" >&2
  exit 1
fi

limactl shell "${VM_NAME}" -- mkdir -p "${VM_WORK_DIR}"
limactl copy "${CODE_ARTIFACT}" "${VM_NAME}:${VM_WORK_DIR}/code.bin"

limactl shell "${VM_NAME}" -- bash -s -- "${VM_WORK_DIR}" <<'VM_SCRIPT'
set -euo pipefail

VM_WORK_DIR="$1"
cd "${VM_WORK_DIR}"

HOST_OS="$(uname -s)"
HOST_ARCH="$(uname -m)"
if [[ "${HOST_OS}" != "Linux" || "${HOST_ARCH}" != "x86_64" ]]; then
  echo "ERROR: VM execution requires Linux/x86_64; got ${HOST_OS}/${HOST_ARCH}" >&2
  exit 1
fi

bytes="$(od -An -tx1 -v code.bin | tr -s '[:space:]' ' ' | sed 's/^ //; s/ $//; s/ /, 0x/g; s/^/0x/')"
if [[ -z "${bytes}" ]]; then
  echo "ERROR: code.bin is empty" >&2
  exit 1
fi

cat >program.s <<ASM
.text
.globl generated
generated:
    .byte ${bytes}

.globl main
main:
    push %rbp
    mov %rsp, %rbp
    call generated
    pop %rbp
    ret

.section .note.GNU-stack,"",@progbits
ASM

cat >runtime.s <<'ASM'
.text
.globl lsharp_runtime_stub
lsharp_runtime_stub:
    ret

.section .note.GNU-stack,"",@progbits
ASM

cc -c program.s -o program.o
cc -c runtime.s -o runtime.o

cat >linker-response.txt <<'EOF_RESPONSE'
-o
program.native
program.o
runtime.o
EOF_RESPONSE

cc @linker-response.txt

set +e
./program.native >stdout.txt 2>stderr.txt
actual_exit_code=$?
set -e

expected_exit_code=42
if [[ "${actual_exit_code}" -ne "${expected_exit_code}" ]]; then
  echo "ERROR: program.native actual_exit_code=${actual_exit_code}, expected ${expected_exit_code}" >&2
  exit 1
fi

cat >summary.json <<JSON
{
  "target": "x86_64-unknown-linux-gnu",
  "host_os": "${HOST_OS}",
  "host_arch": "${HOST_ARCH}",
  "status": "pass",
  "scope": "host-side selfhost-generated code artifact linked and executed in local Linux x86_64 VM",
  "expected_exit_code": ${expected_exit_code},
  "actual_exit_code": ${actual_exit_code},
  "canonical_files": [
    "code.bin",
    "program.o",
    "runtime.o",
    "linker-response.txt",
    "program.native"
  ]
}
JSON
VM_SCRIPT

for file in program.s runtime.s program.o runtime.o linker-response.txt program.native stdout.txt stderr.txt summary.json; do
  limactl copy "${VM_NAME}:${VM_WORK_DIR}/${file}" "${ARTIFACT_DIR}/${file}"
done

echo "native Linux x86_64 hostgen -> VM exec evidence collected."
