#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
ARTIFACT_ID="${NATIVE_MACOS_X86_ARTIFACT_ID:-local}"
ARTIFACT_DIR_INPUT="${LSHARP_NATIVE_MACOS_X86_ARTIFACT_DIR:-ci-artifacts/native-macos-x86-local-smoke/${ARTIFACT_ID}}"

if [[ "${ARTIFACT_DIR_INPUT}" = /* ]]; then
  if [[ "${ARTIFACT_DIR_INPUT}" != "${ROOT_DIR}"/* ]]; then
    echo "ERROR: LSHARP_NATIVE_MACOS_X86_ARTIFACT_DIR must be under repository root: ${ARTIFACT_DIR_INPUT}" >&2
    exit 1
  fi
  ARTIFACT_DIR="${ARTIFACT_DIR_INPUT}"
else
  ARTIFACT_DIR="${ROOT_DIR}/${ARTIFACT_DIR_INPUT}"
fi

HOST_OS="$(uname -s)"
HOST_ARCH="$(uname -m)"
if [[ "${HOST_OS}" != "Darwin" ]]; then
  echo "ERROR: native-macos-x86-local-smoke.sh requires Darwin with Rosetta; got ${HOST_OS}/${HOST_ARCH}" >&2
  exit 1
fi
if ! command -v clang >/dev/null 2>&1; then
  echo "ERROR: clang is required for x86_64-apple-darwin local smoke" >&2
  exit 1
fi
if ! arch -x86_64 /usr/bin/true >/dev/null 2>&1; then
  echo "ERROR: arch -x86_64 failed; Rosetta is required for x86_64-apple-darwin local smoke" >&2
  exit 1
fi

rm -rf "${ARTIFACT_DIR}"
mkdir -p "${ARTIFACT_DIR}"

cat > "${ARTIFACT_DIR}/program.s" <<'ASM'
.section __TEXT,__text
.globl _generated
_generated:
    .byte 0x48,0xc7,0xc0,0x2a,0x00,0x00,0x00,0xc3
.globl _main
_main:
    pushq %rbp
    movq %rsp, %rbp
    callq _generated
    popq %rbp
    retq
ASM

cat > "${ARTIFACT_DIR}/runtime.s" <<'ASM'
.section __TEXT,__text
.globl _lsharp_runtime_stub
_lsharp_runtime_stub:
    retq
ASM

(
  cd "${ARTIFACT_DIR}"
  clang -arch x86_64 -c program.s -o program.o
  clang -arch x86_64 -c runtime.s -o runtime.o
  cat > linker-response.txt <<'EOF'
-o
program.native
program.o
runtime.o
EOF
  clang -arch x86_64 -Wl,-stack_size,0x08000000 @linker-response.txt
)

file_output="$(file "${ARTIFACT_DIR}/program.native")"
if [[ "${file_output}" != *"Mach-O"* || "${file_output}" != *"x86_64"* ]]; then
  echo "ERROR: program.native is not an x86_64 Mach-O binary: ${file_output}" >&2
  exit 1
fi

set +e
arch -x86_64 "${ARTIFACT_DIR}/program.native" > "${ARTIFACT_DIR}/stdout.txt" 2> "${ARTIFACT_DIR}/stderr.txt"
actual_exit_code=$?
set -e

status="fail"
if [[ "${actual_exit_code}" -eq 42 ]]; then
  status="pass"
fi

cat > "${ARTIFACT_DIR}/summary.json" <<EOF
{
  "target": "x86_64-apple-darwin",
  "host_os": "${HOST_OS}",
  "host_arch": "${HOST_ARCH}",
  "status": "${status}",
  "expected_exit_code": 42,
  "actual_exit_code": ${actual_exit_code},
  "binary_kind": "${file_output}"
}
EOF

if [[ "${status}" != "pass" ]]; then
  echo "ERROR: x86_64-apple-darwin Rosetta smoke exit code mismatch: expected 42 got ${actual_exit_code}" >&2
  exit 1
fi

echo "native-macos-x86-local-smoke: OK (${ARTIFACT_DIR})"
