#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
ARTIFACT_ID="${NATIVE_LINUX_X86_HOSTGEN_VM_ARTIFACT_ID:-local}"
ARTIFACT_DIR_INPUT="${LSHARP_NATIVE_LINUX_X86_HOSTGEN_VM_ARTIFACT_DIR:-ci-artifacts/native-linux-x86-hostgen-vm/${ARTIFACT_ID}}"
VM_NAME="${LSHARP_NATIVE_LINUX_X86_VM_NAME:-lsharp-linux-x86}"
VM_WORK_DIR="${LSHARP_NATIVE_LINUX_X86_VM_WORK_DIR:-/tmp/lsharp-native-linux-x86-hostgen-vm-${ARTIFACT_ID}}"
KEEP_VM_WORK_DIR="${LSHARP_NATIVE_LINUX_X86_KEEP_VM_WORK_DIR:-0}"

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
OBJECT_ARTIFACT="${ARTIFACT_DIR}/program.o"
ARGV_OBJECT_ARTIFACT="${ARTIFACT_DIR}/argv-program.o"
ARGV_CHAR_OBJECT_ARTIFACT="${ARTIFACT_DIR}/argv-char-program.o"
PRINT_OBJECT_ARTIFACT="${ARTIFACT_DIR}/print-program.o"
VECTOR_OBJECT_ARTIFACT="${ARTIFACT_DIR}/vector-program.o"
REF_OBJECT_ARTIFACT="${ARTIFACT_DIR}/ref-program.o"
SUBSTRING_OBJECT_ARTIFACT="${ARTIFACT_DIR}/substring-program.o"
STRING_CONCAT_OBJECT_ARTIFACT="${ARTIFACT_DIR}/string-concat-program.o"
MAP_OBJECT_ARTIFACT="${ARTIFACT_DIR}/map-program.o"
MAP_SIZE_OBJECT_ARTIFACT="${ARTIFACT_DIR}/map-size-program.o"
FILE_EXISTS_OBJECT_ARTIFACT="${ARTIFACT_DIR}/file-exists-program.o"
ACTUAL_STAGE1_ARTIFACT_DIR="${ARTIFACT_DIR}/actual-stage1"

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

LSHARP_NATIVE_LINUX_X86_OBJECT_ARTIFACT="${OBJECT_ARTIFACT}" \
  cargo test -q -p lsharp-wasm --test e2e \
  e2e::selfhost_native_stage_chain::test_e2e_native_linux_x86_host_generates_selfhost_elf_object_artifact \
  -- --exact --ignored

if [[ ! -s "${OBJECT_ARTIFACT}" ]]; then
  echo "ERROR: host ELF object artifact was not generated: ${OBJECT_ARTIFACT}" >&2
  exit 1
fi

LSHARP_NATIVE_LINUX_X86_ARGV_OBJECT_ARTIFACT="${ARGV_OBJECT_ARTIFACT}" \
  cargo test -q -p lsharp-wasm --test e2e \
  e2e::selfhost_native_stage_chain::test_e2e_native_linux_x86_host_generates_argv_string_length_elf_object_artifact \
  -- --exact --ignored

if [[ ! -s "${ARGV_OBJECT_ARTIFACT}" ]]; then
  echo "ERROR: host argv ELF object artifact was not generated: ${ARGV_OBJECT_ARTIFACT}" >&2
  exit 1
fi

LSHARP_NATIVE_LINUX_X86_ARGV_CHAR_OBJECT_ARTIFACT="${ARGV_CHAR_OBJECT_ARTIFACT}" \
  cargo test -q -p lsharp-wasm --test e2e \
  e2e::selfhost_native_stage_chain::test_e2e_native_linux_x86_host_generates_argv_string_char_at_elf_object_artifact \
  -- --exact --ignored

if [[ ! -s "${ARGV_CHAR_OBJECT_ARTIFACT}" ]]; then
  echo "ERROR: host argv char ELF object artifact was not generated: ${ARGV_CHAR_OBJECT_ARTIFACT}" >&2
  exit 1
fi

LSHARP_NATIVE_LINUX_X86_PRINT_OBJECT_ARTIFACT="${PRINT_OBJECT_ARTIFACT}" \
  cargo test -q -p lsharp-wasm --test e2e \
  e2e::selfhost_native_stage_chain::test_e2e_native_linux_x86_host_generates_print_elf_object_artifact \
  -- --exact --ignored

if [[ ! -s "${PRINT_OBJECT_ARTIFACT}" ]]; then
  echo "ERROR: host print ELF object artifact was not generated: ${PRINT_OBJECT_ARTIFACT}" >&2
  exit 1
fi

LSHARP_NATIVE_LINUX_X86_VECTOR_OBJECT_ARTIFACT="${VECTOR_OBJECT_ARTIFACT}" \
  cargo test -q -p lsharp-wasm --test e2e \
  e2e::selfhost_native_stage_chain::test_e2e_native_linux_x86_host_generates_vector_elf_object_artifact \
  -- --exact --ignored

if [[ ! -s "${VECTOR_OBJECT_ARTIFACT}" ]]; then
  echo "ERROR: host vector ELF object artifact was not generated: ${VECTOR_OBJECT_ARTIFACT}" >&2
  exit 1
fi

LSHARP_NATIVE_LINUX_X86_REF_OBJECT_ARTIFACT="${REF_OBJECT_ARTIFACT}" \
  cargo test -q -p lsharp-wasm --test e2e \
  e2e::selfhost_native_stage_chain::test_e2e_native_linux_x86_host_generates_ref_elf_object_artifact \
  -- --exact --ignored

if [[ ! -s "${REF_OBJECT_ARTIFACT}" ]]; then
  echo "ERROR: host ref ELF object artifact was not generated: ${REF_OBJECT_ARTIFACT}" >&2
  exit 1
fi

LSHARP_NATIVE_LINUX_X86_SUBSTRING_OBJECT_ARTIFACT="${SUBSTRING_OBJECT_ARTIFACT}" \
  cargo test -q -p lsharp-wasm --test e2e \
  e2e::selfhost_native_stage_chain::test_e2e_native_linux_x86_host_generates_substring_elf_object_artifact \
  -- --exact --ignored

if [[ ! -s "${SUBSTRING_OBJECT_ARTIFACT}" ]]; then
  echo "ERROR: host substring ELF object artifact was not generated: ${SUBSTRING_OBJECT_ARTIFACT}" >&2
  exit 1
fi

LSHARP_NATIVE_LINUX_X86_STRING_CONCAT_OBJECT_ARTIFACT="${STRING_CONCAT_OBJECT_ARTIFACT}" \
  cargo test -q -p lsharp-wasm --test e2e \
  e2e::selfhost_native_stage_chain::test_e2e_native_linux_x86_host_generates_string_concat_elf_object_artifact \
  -- --exact --ignored

if [[ ! -s "${STRING_CONCAT_OBJECT_ARTIFACT}" ]]; then
  echo "ERROR: host string-concat ELF object artifact was not generated: ${STRING_CONCAT_OBJECT_ARTIFACT}" >&2
  exit 1
fi

LSHARP_NATIVE_LINUX_X86_MAP_OBJECT_ARTIFACT="${MAP_OBJECT_ARTIFACT}" \
  cargo test -q -p lsharp-wasm --test e2e \
  e2e::selfhost_native_stage_chain::test_e2e_native_linux_x86_host_generates_map_insert_get_elf_object_artifact \
  -- --exact --ignored

if [[ ! -s "${MAP_OBJECT_ARTIFACT}" ]]; then
  echo "ERROR: host map ELF object artifact was not generated: ${MAP_OBJECT_ARTIFACT}" >&2
  exit 1
fi

LSHARP_NATIVE_LINUX_X86_MAP_SIZE_OBJECT_ARTIFACT="${MAP_SIZE_OBJECT_ARTIFACT}" \
  cargo test -q -p lsharp-wasm --test e2e \
  e2e::selfhost_native_stage_chain::test_e2e_native_linux_x86_host_generates_map_size_elf_object_artifact \
  -- --exact --ignored

if [[ ! -s "${MAP_SIZE_OBJECT_ARTIFACT}" ]]; then
  echo "ERROR: host map-size ELF object artifact was not generated: ${MAP_SIZE_OBJECT_ARTIFACT}" >&2
  exit 1
fi

LSHARP_NATIVE_LINUX_X86_FILE_EXISTS_OBJECT_ARTIFACT="${FILE_EXISTS_OBJECT_ARTIFACT}" \
  cargo test -q -p lsharp-wasm --test e2e \
  e2e::selfhost_native_stage_chain::test_e2e_native_linux_x86_host_generates_file_exists_elf_object_artifact \
  -- --exact --ignored

if [[ ! -s "${FILE_EXISTS_OBJECT_ARTIFACT}" ]]; then
  echo "ERROR: host file-exists ELF object artifact was not generated: ${FILE_EXISTS_OBJECT_ARTIFACT}" >&2
  exit 1
fi

LSHARP_NATIVE_LINUX_X86_ACTUAL_STAGE1_ARTIFACT_DIR="${ACTUAL_STAGE1_ARTIFACT_DIR}" \
  cargo test -q -p lsharp-wasm --test e2e \
  e2e::selfhost_native_stage_chain::test_e2e_native_linux_x86_host_generates_actual_selfregen_stage1_bundle_artifact \
  -- --exact --ignored

for file in stage1-code.bin entrypoint-offset.txt seed.ls manifest.json; do
  if [[ ! -s "${ACTUAL_STAGE1_ARTIFACT_DIR}/${file}" ]]; then
    echo "ERROR: actual stage1 artifact was not generated: ${ACTUAL_STAGE1_ARTIFACT_DIR}/${file}" >&2
    exit 1
  fi
done

limactl shell "${VM_NAME}" -- rm -rf "${VM_WORK_DIR}"
limactl shell "${VM_NAME}" -- mkdir -p "${VM_WORK_DIR}"
limactl copy "${CODE_ARTIFACT}" "${VM_NAME}:${VM_WORK_DIR}/code.bin"
limactl copy "${OBJECT_ARTIFACT}" "${VM_NAME}:${VM_WORK_DIR}/program.o"
limactl copy "${ARGV_OBJECT_ARTIFACT}" "${VM_NAME}:${VM_WORK_DIR}/argv-program.o"
limactl copy "${ARGV_CHAR_OBJECT_ARTIFACT}" "${VM_NAME}:${VM_WORK_DIR}/argv-char-program.o"
limactl copy "${PRINT_OBJECT_ARTIFACT}" "${VM_NAME}:${VM_WORK_DIR}/print-program.o"
limactl copy "${VECTOR_OBJECT_ARTIFACT}" "${VM_NAME}:${VM_WORK_DIR}/vector-program.o"
limactl copy "${REF_OBJECT_ARTIFACT}" "${VM_NAME}:${VM_WORK_DIR}/ref-program.o"
limactl copy "${SUBSTRING_OBJECT_ARTIFACT}" "${VM_NAME}:${VM_WORK_DIR}/substring-program.o"
limactl copy "${STRING_CONCAT_OBJECT_ARTIFACT}" "${VM_NAME}:${VM_WORK_DIR}/string-concat-program.o"
limactl copy "${MAP_OBJECT_ARTIFACT}" "${VM_NAME}:${VM_WORK_DIR}/map-program.o"
limactl copy "${MAP_SIZE_OBJECT_ARTIFACT}" "${VM_NAME}:${VM_WORK_DIR}/map-size-program.o"
limactl copy "${FILE_EXISTS_OBJECT_ARTIFACT}" "${VM_NAME}:${VM_WORK_DIR}/file-exists-program.o"
limactl shell "${VM_NAME}" -- rm -rf "${VM_WORK_DIR}/actual-stage1"
limactl shell "${VM_NAME}" -- mkdir -p "${VM_WORK_DIR}/actual-stage1/src/App"
limactl copy "${ACTUAL_STAGE1_ARTIFACT_DIR}/stage1-code.bin" "${VM_NAME}:${VM_WORK_DIR}/actual-stage1/stage1-code.bin"
limactl copy "${ACTUAL_STAGE1_ARTIFACT_DIR}/stage1-data.bin" "${VM_NAME}:${VM_WORK_DIR}/actual-stage1/stage1-data.bin"
limactl copy "${ACTUAL_STAGE1_ARTIFACT_DIR}/entrypoint-offset.txt" "${VM_NAME}:${VM_WORK_DIR}/actual-stage1/entrypoint-offset.txt"
limactl copy "${ACTUAL_STAGE1_ARTIFACT_DIR}/function-start-len.txt" "${VM_NAME}:${VM_WORK_DIR}/actual-stage1/function-start-len.txt"
limactl copy "${ACTUAL_STAGE1_ARTIFACT_DIR}/main-func-idx.txt" "${VM_NAME}:${VM_WORK_DIR}/actual-stage1/main-func-idx.txt"
limactl copy "${ACTUAL_STAGE1_ARTIFACT_DIR}/manifest.json" "${VM_NAME}:${VM_WORK_DIR}/actual-stage1/manifest.json"
limactl copy "${ACTUAL_STAGE1_ARTIFACT_DIR}/seed.ls" "${VM_NAME}:${VM_WORK_DIR}/actual-stage1/src/App/Seed.ls"
tar -C "${ROOT_DIR}/selfhost" --exclude '._*' -cf - src | limactl shell "${VM_NAME}" -- tar -C "${VM_WORK_DIR}/actual-stage1" -xf -
limactl copy "${ACTUAL_STAGE1_ARTIFACT_DIR}/seed.ls" "${VM_NAME}:${VM_WORK_DIR}/actual-stage1/src/App/Seed.ls"

set +e
limactl shell "${VM_NAME}" -- env \
  LSHARP_NATIVE_LINUX_X86_ACTUAL_TIMEOUT="${LSHARP_NATIVE_LINUX_X86_ACTUAL_TIMEOUT:-900}" \
  LSHARP_NATIVE_LINUX_X86_ACTUAL_CHUNK_SIZE="${LSHARP_NATIVE_LINUX_X86_ACTUAL_CHUNK_SIZE:-4}" \
  LSHARP_NATIVE_LINUX_X86_ACTUAL_CHUNK_RETRIES="${LSHARP_NATIVE_LINUX_X86_ACTUAL_CHUNK_RETRIES:-1}" \
  LSHARP_NATIVE_LINUX_X86_ACTUAL_HEAP_BYTES="${LSHARP_NATIVE_LINUX_X86_ACTUAL_HEAP_BYTES:-4294967296}" \
  LSHARP_NATIVE_LINUX_X86_VM_REPLAY_LOCK_DIR="${LSHARP_NATIVE_LINUX_X86_VM_REPLAY_LOCK_DIR:-/tmp/lsharp-native-linux-x86-hostgen-vm-replay.lock}" \
  LSHARP_NATIVE_LINUX_X86_STAGE2_METADATA_START="${LSHARP_NATIVE_LINUX_X86_STAGE2_METADATA_START:-}" \
  LSHARP_NATIVE_LINUX_X86_STAGE2_METADATA_END="${LSHARP_NATIVE_LINUX_X86_STAGE2_METADATA_END:-}" \
  LSHARP_NATIVE_LINUX_X86_STAGE3_PROGRESS="${LSHARP_NATIVE_LINUX_X86_STAGE3_PROGRESS:-}" \
  LSHARP_NATIVE_LINUX_X86_STAGE3_PROGRESS_ONLY="${LSHARP_NATIVE_LINUX_X86_STAGE3_PROGRESS_ONLY:-}" \
  bash -s -- "${VM_WORK_DIR}" <<'VM_SCRIPT'
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

cc -c program.s -o code-program.o
cc -c runtime.s -o runtime.o

cat >linker-response.txt <<'EOF_RESPONSE'
-o
program.native
code-program.o
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
    "code-program.o",
    "runtime.o",
    "linker-response.txt",
    "program.native"
  ]
}
JSON

cat >object-runtime.s <<'ASM'
.text
.extern calloc
.extern malloc
.extern memcpy
.extern strlen
.extern generated
.globl main
main:
    push %rbp
    mov %rsp, %rbp
    push %rbx
    push %r12
    push %r13
    push %r14
    push %r15
    sub $24, %rsp

    mov %rdi, %r14
    mov %rsi, %r12
    mov %r14, %rdi
    mov $8, %rsi
    call calloc@PLT
    test %rax, %rax
    je .Lalloc_fail
    mov %rax, %r15
    xor %r13d, %r13d

.Largv_loop:
    cmp %r14, %r13
    jge .Largv_done
    mov (%r12,%r13,8), %rbx
    mov %rbx, %rdi
    call strlen@PLT
    mov %rax, -48(%rbp)
    lea 8(%rax), %rdi
    call malloc@PLT
    test %rax, %rax
    je .Lalloc_fail
    mov %rax, -56(%rbp)
    movl $1, (%rax)
    mov -48(%rbp), %rcx
    mov %ecx, 4(%rax)
    lea 8(%rax), %rdi
    mov %rbx, %rsi
    mov %rcx, %rdx
    call memcpy@PLT
    mov -56(%rbp), %rax
    movabs $0x8000000000000000, %rcx
    or %rcx, %rax
    mov %rax, (%r15,%r13,8)
    inc %r13
    jmp .Largv_loop

.Largv_done:
    mov %r14, %r12
    xor %r14d, %r14d
    mov %r14, %rdi
    mov %r15, %rsi
    call generated
    jmp .Ldone

.Lalloc_fail:
    mov $1, %eax

.Ldone:
    add $24, %rsp
    pop %r15
    pop %r14
    pop %r13
    pop %r12
    pop %rbx
    pop %rbp
    ret

.section .note.GNU-stack,"",@progbits
ASM

cc -c object-runtime.s -o object-runtime.o

cat >object-linker-response.txt <<'EOF_RESPONSE'
-o
object-program.native
program.o
object-runtime.o
EOF_RESPONSE

cc @object-linker-response.txt

set +e
./object-program.native >object-stdout.txt 2>object-stderr.txt
object_actual_exit_code=$?
set -e

if [[ "${object_actual_exit_code}" -ne "${expected_exit_code}" ]]; then
  echo "ERROR: object-program.native actual_exit_code=${object_actual_exit_code}, expected ${expected_exit_code}" >&2
  exit 1
fi

cat >object-summary.json <<JSON
{
  "target": "x86_64-unknown-linux-gnu",
  "host_os": "${HOST_OS}",
  "host_arch": "${HOST_ARCH}",
  "status": "pass",
  "scope": "host-side selfhost-generated ELF object linked and executed in local Linux x86_64 VM",
  "expected_exit_code": ${expected_exit_code},
  "actual_exit_code": ${object_actual_exit_code},
  "canonical_files": [
    "program.o",
    "object-runtime.o",
    "object-linker-response.txt",
    "object-program.native"
  ]
}
JSON

cat >argv-object-linker-response.txt <<'EOF_RESPONSE'
-o
argv-object-program.native
argv-program.o
object-runtime.o
EOF_RESPONSE

cc @argv-object-linker-response.txt

set +e
./argv-object-program.native seedling >argv-object-stdout.txt 2>argv-object-stderr.txt
argv_object_actual_exit_code=$?
set -e

argv_expected_exit_code=8
if [[ "${argv_object_actual_exit_code}" -ne "${argv_expected_exit_code}" ]]; then
  echo "ERROR: argv-object-program.native actual_exit_code=${argv_object_actual_exit_code}, expected ${argv_expected_exit_code}" >&2
  exit 1
fi

cat >argv-object-summary.json <<JSON
{
  "target": "x86_64-unknown-linux-gnu",
  "host_os": "${HOST_OS}",
  "host_arch": "${HOST_ARCH}",
  "status": "pass",
  "scope": "host-side selfhost-generated ELF object executes command-line-arg plus string-length helper in local Linux x86_64 VM",
  "argv": ["seedling"],
  "expected_exit_code": ${argv_expected_exit_code},
  "actual_exit_code": ${argv_object_actual_exit_code},
  "canonical_files": [
    "argv-program.o",
    "object-runtime.o",
    "argv-object-linker-response.txt",
    "argv-object-program.native"
  ]
}
JSON

set +e
./argv-object-program.native >argv-object-noarg-stdout.txt 2>argv-object-noarg-stderr.txt
argv_object_noarg_actual_exit_code=$?
set -e

argv_noarg_expected_exit_code=0
if [[ "${argv_object_noarg_actual_exit_code}" -ne "${argv_noarg_expected_exit_code}" ]]; then
  echo "ERROR: argv-object-program.native no-arg actual_exit_code=${argv_object_noarg_actual_exit_code}, expected ${argv_noarg_expected_exit_code}" >&2
  exit 1
fi

cat >argv-object-noarg-summary.json <<JSON
{
  "target": "x86_64-unknown-linux-gnu",
  "host_os": "${HOST_OS}",
  "host_arch": "${HOST_ARCH}",
  "status": "pass",
  "scope": "host-side selfhost-generated ELF object treats missing argv string-length as zero in local Linux x86_64 VM",
  "argv": [],
  "expected_exit_code": ${argv_noarg_expected_exit_code},
  "actual_exit_code": ${argv_object_noarg_actual_exit_code},
  "canonical_files": [
    "argv-program.o",
    "object-runtime.o",
    "argv-object-linker-response.txt",
    "argv-object-program.native"
  ]
}
JSON

cat >argv-char-object-linker-response.txt <<'EOF_RESPONSE'
-o
argv-char-object-program.native
argv-char-program.o
object-runtime.o
EOF_RESPONSE

cc @argv-char-object-linker-response.txt

set +e
./argv-char-object-program.native seedling >argv-char-object-stdout.txt 2>argv-char-object-stderr.txt
argv_char_object_actual_exit_code=$?
set -e

argv_char_expected_exit_code=101
if [[ "${argv_char_object_actual_exit_code}" -ne "${argv_char_expected_exit_code}" ]]; then
  echo "ERROR: argv-char-object-program.native actual_exit_code=${argv_char_object_actual_exit_code}, expected ${argv_char_expected_exit_code}" >&2
  exit 1
fi

cat >argv-char-object-summary.json <<JSON
{
  "target": "x86_64-unknown-linux-gnu",
  "host_os": "${HOST_OS}",
  "host_arch": "${HOST_ARCH}",
  "status": "pass",
  "scope": "host-side selfhost-generated ELF object executes command-line-arg plus string-char-at helper in local Linux x86_64 VM",
  "argv": ["seedling"],
  "expected_exit_code": ${argv_char_expected_exit_code},
  "actual_exit_code": ${argv_char_object_actual_exit_code},
  "canonical_files": [
    "argv-char-program.o",
    "object-runtime.o",
    "argv-char-object-linker-response.txt",
    "argv-char-object-program.native"
  ]
}
JSON

cat >print-object-linker-response.txt <<'EOF_RESPONSE'
-o
print-object-program.native
print-program.o
object-runtime.o
EOF_RESPONSE

cc @print-object-linker-response.txt

set +e
./print-object-program.native >print-object-stdout.txt 2>print-object-stderr.txt
print_object_actual_exit_code=$?
set -e

print_expected_stdout="42"
print_actual_stdout="$(tr -d '\r' < print-object-stdout.txt)"
if [[ "${print_object_actual_exit_code}" -ne 0 ]]; then
  echo "ERROR: print-object-program.native actual_exit_code=${print_object_actual_exit_code}, expected 0" >&2
  exit 1
fi
if [[ "${print_actual_stdout}" != "${print_expected_stdout}" ]]; then
  echo "ERROR: print-object-program.native stdout='${print_actual_stdout}', expected '${print_expected_stdout}'" >&2
  exit 1
fi

cat >print-object-summary.json <<JSON
{
  "target": "x86_64-unknown-linux-gnu",
  "host_os": "${HOST_OS}",
  "host_arch": "${HOST_ARCH}",
  "status": "pass",
  "scope": "host-side selfhost-generated ELF object executes print helper in local Linux x86_64 VM",
  "expected_exit_code": 0,
  "actual_exit_code": ${print_object_actual_exit_code},
  "expected_stdout": "${print_expected_stdout}",
  "actual_stdout": "${print_actual_stdout}",
  "canonical_files": [
    "print-program.o",
    "object-runtime.o",
    "print-object-linker-response.txt",
    "print-object-program.native"
  ]
}
JSON

cat >vector-object-linker-response.txt <<'EOF_RESPONSE'
-o
vector-object-program.native
vector-program.o
object-runtime.o
EOF_RESPONSE

cc @vector-object-linker-response.txt

set +e
./vector-object-program.native >vector-object-stdout.txt 2>vector-object-stderr.txt
vector_object_actual_exit_code=$?
set -e

vector_expected_exit_code=42
if [[ "${vector_object_actual_exit_code}" -ne "${vector_expected_exit_code}" ]]; then
  echo "ERROR: vector-object-program.native actual_exit_code=${vector_object_actual_exit_code}, expected ${vector_expected_exit_code}" >&2
  exit 1
fi

cat >vector-object-summary.json <<JSON
{
  "target": "x86_64-unknown-linux-gnu",
  "host_os": "${HOST_OS}",
  "host_arch": "${HOST_ARCH}",
  "status": "pass",
  "scope": "host-side selfhost-generated ELF object executes vector-new/vector-push/vector-get helpers in local Linux x86_64 VM",
  "expected_exit_code": ${vector_expected_exit_code},
  "actual_exit_code": ${vector_object_actual_exit_code},
  "canonical_files": [
    "vector-program.o",
    "object-runtime.o",
    "vector-object-linker-response.txt",
    "vector-object-program.native"
  ]
}
JSON

cat >ref-object-linker-response.txt <<'EOF_RESPONSE'
-o
ref-object-program.native
ref-program.o
object-runtime.o
EOF_RESPONSE

cc @ref-object-linker-response.txt

set +e
./ref-object-program.native >ref-object-stdout.txt 2>ref-object-stderr.txt
ref_object_actual_exit_code=$?
set -e

ref_expected_exit_code=99
if [[ "${ref_object_actual_exit_code}" -ne "${ref_expected_exit_code}" ]]; then
  echo "ERROR: ref-object-program.native actual_exit_code=${ref_object_actual_exit_code}, expected ${ref_expected_exit_code}" >&2
  exit 1
fi

cat >ref-object-summary.json <<JSON
{
  "target": "x86_64-unknown-linux-gnu",
  "host_os": "${HOST_OS}",
  "host_arch": "${HOST_ARCH}",
  "status": "pass",
  "scope": "host-side selfhost-generated ELF object executes ref-new/ref-set/ref-get helpers in local Linux x86_64 VM",
  "expected_exit_code": ${ref_expected_exit_code},
  "actual_exit_code": ${ref_object_actual_exit_code},
  "canonical_files": [
    "ref-program.o",
    "object-runtime.o",
    "ref-object-linker-response.txt",
    "ref-object-program.native"
  ]
}
JSON

cat >substring-object-linker-response.txt <<'EOF_RESPONSE'
-o
substring-object-program.native
substring-program.o
object-runtime.o
EOF_RESPONSE

cc @substring-object-linker-response.txt

set +e
./substring-object-program.native seedling >substring-object-stdout.txt 2>substring-object-stderr.txt
substring_object_actual_exit_code=$?
set -e

substring_expected_exit_code=3
if [[ "${substring_object_actual_exit_code}" -ne "${substring_expected_exit_code}" ]]; then
  echo "ERROR: substring-object-program.native actual_exit_code=${substring_object_actual_exit_code}, expected ${substring_expected_exit_code}" >&2
  exit 1
fi

cat >substring-object-summary.json <<JSON
{
  "target": "x86_64-unknown-linux-gnu",
  "host_os": "${HOST_OS}",
  "host_arch": "${HOST_ARCH}",
  "status": "pass",
  "scope": "host-side selfhost-generated ELF object executes substring helper in local Linux x86_64 VM",
  "argv": ["seedling"],
  "expected_exit_code": ${substring_expected_exit_code},
  "actual_exit_code": ${substring_object_actual_exit_code},
  "canonical_files": [
    "substring-program.o",
    "object-runtime.o",
    "substring-object-linker-response.txt",
    "substring-object-program.native"
  ]
}
JSON

cat >string-concat-object-linker-response.txt <<'EOF_RESPONSE'
-o
string-concat-object-program.native
string-concat-program.o
object-runtime.o
EOF_RESPONSE

cc @string-concat-object-linker-response.txt

set +e
./string-concat-object-program.native seed ling >string-concat-object-stdout.txt 2>string-concat-object-stderr.txt
string_concat_object_actual_exit_code=$?
set -e

string_concat_expected_exit_code=8
if [[ "${string_concat_object_actual_exit_code}" -ne "${string_concat_expected_exit_code}" ]]; then
  echo "ERROR: string-concat-object-program.native actual_exit_code=${string_concat_object_actual_exit_code}, expected ${string_concat_expected_exit_code}" >&2
  exit 1
fi

cat >string-concat-object-summary.json <<JSON
{
  "target": "x86_64-unknown-linux-gnu",
  "host_os": "${HOST_OS}",
  "host_arch": "${HOST_ARCH}",
  "status": "pass",
  "scope": "host-side selfhost-generated ELF object executes string-concat helper in local Linux x86_64 VM",
  "argv": ["seed", "ling"],
  "expected_exit_code": ${string_concat_expected_exit_code},
  "actual_exit_code": ${string_concat_object_actual_exit_code},
  "canonical_files": [
    "string-concat-program.o",
    "object-runtime.o",
    "string-concat-object-linker-response.txt",
    "string-concat-object-program.native"
  ]
}
JSON

cat >map-object-linker-response.txt <<'EOF_RESPONSE'
-o
map-object-program.native
map-program.o
object-runtime.o
EOF_RESPONSE

cc @map-object-linker-response.txt

set +e
./map-object-program.native >map-object-stdout.txt 2>map-object-stderr.txt
map_object_actual_exit_code=$?
set -e

map_expected_exit_code=42
if [[ "${map_object_actual_exit_code}" -ne "${map_expected_exit_code}" ]]; then
  echo "ERROR: map-object-program.native actual_exit_code=${map_object_actual_exit_code}, expected ${map_expected_exit_code}" >&2
  exit 1
fi

cat >map-object-summary.json <<JSON
{
  "target": "x86_64-unknown-linux-gnu",
  "host_os": "${HOST_OS}",
  "host_arch": "${HOST_ARCH}",
  "status": "pass",
  "scope": "host-side selfhost-generated ELF object executes map-new/map-insert/map-get helpers in local Linux x86_64 VM",
  "expected_exit_code": ${map_expected_exit_code},
  "actual_exit_code": ${map_object_actual_exit_code},
  "canonical_files": [
    "map-program.o",
    "object-runtime.o",
    "map-object-linker-response.txt",
    "map-object-program.native"
  ]
}
JSON

cat >map-size-object-linker-response.txt <<'EOF_RESPONSE'
-o
map-size-object-program.native
map-size-program.o
object-runtime.o
EOF_RESPONSE

cc @map-size-object-linker-response.txt

set +e
./map-size-object-program.native >map-size-object-stdout.txt 2>map-size-object-stderr.txt
map_size_object_actual_exit_code=$?
set -e

map_size_expected_exit_code=1
if [[ "${map_size_object_actual_exit_code}" -ne "${map_size_expected_exit_code}" ]]; then
  echo "ERROR: map-size-object-program.native actual_exit_code=${map_size_object_actual_exit_code}, expected ${map_size_expected_exit_code}" >&2
  exit 1
fi

cat >map-size-object-summary.json <<JSON
{
  "target": "x86_64-unknown-linux-gnu",
  "host_os": "${HOST_OS}",
  "host_arch": "${HOST_ARCH}",
  "status": "pass",
  "scope": "host-side selfhost-generated ELF object executes map-size helper in local Linux x86_64 VM",
  "expected_exit_code": ${map_size_expected_exit_code},
  "actual_exit_code": ${map_size_object_actual_exit_code},
  "canonical_files": [
    "map-size-program.o",
    "object-runtime.o",
    "map-size-object-linker-response.txt",
    "map-size-object-program.native"
  ]
}
JSON

touch file-exists-target.txt
cat >file-exists-object-linker-response.txt <<'EOF_RESPONSE'
-o
file-exists-object-program.native
file-exists-program.o
object-runtime.o
EOF_RESPONSE

cc @file-exists-object-linker-response.txt

set +e
./file-exists-object-program.native file-exists-target.txt >file-exists-object-stdout.txt 2>file-exists-object-stderr.txt
file_exists_object_actual_exit_code=$?
set -e

file_exists_expected_exit_code=1
if [[ "${file_exists_object_actual_exit_code}" -ne "${file_exists_expected_exit_code}" ]]; then
  echo "ERROR: file-exists-object-program.native actual_exit_code=${file_exists_object_actual_exit_code}, expected ${file_exists_expected_exit_code}" >&2
  exit 1
fi

cat >file-exists-object-summary.json <<JSON
{
  "target": "x86_64-unknown-linux-gnu",
  "host_os": "${HOST_OS}",
  "host_arch": "${HOST_ARCH}",
  "status": "pass",
  "scope": "host-side selfhost-generated ELF object executes file-exists helper in local Linux x86_64 VM",
  "argv": ["file-exists-target.txt"],
  "expected_exit_code": ${file_exists_expected_exit_code},
  "actual_exit_code": ${file_exists_object_actual_exit_code},
  "canonical_files": [
    "file-exists-program.o",
    "object-runtime.o",
    "file-exists-object-linker-response.txt",
    "file-exists-object-program.native"
  ]
}
JSON

cat >materialize-actual-bundle.py <<'PY'
import os
import pathlib
import subprocess
import sys

stage_dir = pathlib.Path(sys.argv[1])
code_name = sys.argv[2]
entrypoint = int((stage_dir / sys.argv[3]).read_text().strip())
actual_heap_bytes = int(os.environ.get("LSHARP_NATIVE_LINUX_X86_ACTUAL_HEAP_BYTES", "4294967296"))
code_path = stage_dir / code_name
code_len = code_path.stat().st_size
data_name = "stage1-data.bin" if (stage_dir / "stage1-data.bin").exists() else "stage-data.bin"
data_path = stage_dir / data_name
if not data_path.exists():
    data_path.write_bytes(b"")
data_len = data_path.stat().st_size if data_path.exists() else 0
if entrypoint < 0 or entrypoint >= code_len:
    raise SystemExit(f"entrypoint out of range: offset={entrypoint} len={code_len}")

prefix = f'    .incbin "{code_name}", 0, {entrypoint}\n' if entrypoint else ""
suffix = f'    .incbin "{code_name}", {entrypoint}\n'
program_asm = f""".text
.extern calloc
.extern malloc
.extern memcpy
.extern strlen
.globl main
main:
    push %rbp
    mov %rsp, %rbp
    push %rbx
    push %r12
    push %r13
    push %r14
    push %r15
    sub $24, %rsp
    mov %rdi, %r14
    mov %rsi, %r12
    mov %r14, %rdi
    mov $8, %rsi
    call calloc@PLT
    test %rax, %rax
    je .Lalloc_fail
    mov %rax, %r15
    xor %r13d, %r13d
.Largv_loop:
    cmp %r14, %r13
    jge .Largv_done
    mov (%r12,%r13,8), %rbx
    mov %rbx, %rdi
    call strlen@PLT
    mov %rax, -48(%rbp)
    lea 8(%rax), %rdi
    call malloc@PLT
    test %rax, %rax
    je .Lalloc_fail
    mov %rax, -56(%rbp)
    movl $1, (%rax)
    mov -48(%rbp), %rcx
    mov %ecx, 4(%rax)
    lea 8(%rax), %rdi
    mov %rbx, %rsi
    mov %rcx, %rdx
    call memcpy@PLT
    mov -56(%rbp), %rax
    movabs $0x8000000000000000, %rcx
    or %rcx, %rax
    mov %rax, (%r15,%r13,8)
    inc %r13
    jmp .Largv_loop
.Largv_done:
    mov %r14, %r12
    movabs ${actual_heap_bytes}, %rdi
    mov $1, %rsi
    call calloc@PLT
    test %rax, %rax
    je .Lalloc_fail
    mov %rax, %r14
    lea 1024(%r14), %rdi
    lea lsharp_data(%rip), %rsi
    mov ${data_len}, %rdx
    call memcpy@PLT
    call generated
    jmp .Ldone
.Lalloc_fail:
    mov $1, %eax
.Ldone:
    add $24, %rsp
    pop %r15
    pop %r14
    pop %r13
    pop %r12
    pop %rbx
    pop %rbp
    ret
.globl lsharp_bundle
lsharp_bundle:
{prefix}.globl generated
generated:
{suffix}.section .rodata
lsharp_data:
    .incbin "{data_name}"
.section .note.GNU-stack,"",@progbits
"""
(stage_dir / "program.s").write_text(program_asm)
(stage_dir / "runtime.s").write_text(
    '.text\n.globl lsharp_runtime_stub\nlsharp_runtime_stub:\n    ret\n.section .note.GNU-stack,"",@progbits\n'
)
subprocess.run(["cc", "-c", "program.s", "-o", "program.o"], cwd=stage_dir, check=True)
subprocess.run(["cc", "-c", "runtime.s", "-o", "runtime.o"], cwd=stage_dir, check=True)
(stage_dir / "linker-response.txt").write_text("-o\nprogram.native\nprogram.o\nruntime.o\n")
subprocess.run(["cc", "@linker-response.txt"], cwd=stage_dir, check=True)
PY

cat >decode-actual-transport.py <<'PY'
import pathlib
import sys

stdout_path = pathlib.Path(sys.argv[1])
out_dir = pathlib.Path(sys.argv[2])
out_dir.mkdir(parents=True, exist_ok=True)
lines = [line for line in stdout_path.read_bytes().replace(b"\0", b"\n").splitlines() if line]

def parse_int(line: bytes) -> int:
    return int(line.decode("utf-8"))

def expect(index: int, sentinel: int) -> int:
    if index >= len(lines) or parse_int(lines[index]) != sentinel:
        got = lines[index][:80] if index < len(lines) else b"<eof>"
        raise SystemExit(f"missing sentinel {sentinel} at line {index}: {got!r}")
    return index + 1

def decode_packed_flat(packed_lines, declared_len: int) -> bytes:
    decoded = bytearray()
    mask = (1 << 64) - 1
    for raw in packed_lines:
        packed = parse_int(raw) & mask
        for byte_idx in range(8):
            if len(decoded) >= declared_len:
                return bytes(decoded)
            decoded.append((packed >> (byte_idx * 8)) & 0xff)
    if len(decoded) != declared_len:
        raise SystemExit(f"decoded length mismatch: declared={declared_len} actual={len(decoded)}")
    return bytes(decoded)

def packed_line_count(byte_len: int) -> int:
    return (byte_len + 7) // 8

def decode_packed_payload(packed_lines, declared_len: int) -> bytes:
    if not packed_lines or parse_int(packed_lines[0]) != 9000000010:
        return decode_packed_flat(packed_lines, declared_len)
    decoded = bytearray()
    idx = 0
    while idx < len(packed_lines) and len(decoded) < declared_len:
        if parse_int(packed_lines[idx]) != 9000000010:
            raise SystemExit(f"missing segment marker at packed payload line {idx}")
        idx += 1
        if idx >= len(packed_lines):
            raise SystemExit("missing segment length after segment marker")
        segment_len = parse_int(packed_lines[idx])
        idx += 1
        count = packed_line_count(segment_len)
        segment = decode_packed_flat(packed_lines[idx:idx + count], segment_len)
        decoded.extend(segment)
        idx += count
    if len(decoded) != declared_len:
        raise SystemExit(f"decoded segmented length mismatch: declared={declared_len} actual={len(decoded)}")
    return bytes(decoded)

def decode_packed_payload_at(index: int, declared_len: int, end_sentinel):
    if index < len(lines) and parse_int(lines[index]) == 9000000010:
        decoded = bytearray()
        segments = []
        segment_index = 0
        while index < len(lines) and len(decoded) < declared_len:
            if parse_int(lines[index]) != 9000000010:
                raise SystemExit(f"missing segment marker at line {index}")
            index += 1
            if index >= len(lines):
                raise SystemExit("missing segment length after segment marker")
            segment_len = parse_int(lines[index])
            index += 1
            count = packed_line_count(segment_len)
            segment = decode_packed_flat(lines[index:index + count], segment_len)
            segments.append((segment_index, len(decoded), segment_len, bytes(segment[:32])))
            decoded.extend(segment)
            index += count
            segment_index += 1
        if len(decoded) != declared_len:
            raise SystemExit(f"decoded segmented length mismatch: declared={declared_len} actual={len(decoded)}")
        return bytes(decoded), index, segments
    if end_sentinel is None:
        payload = decode_packed_flat(lines[index:], declared_len)
        return payload, len(lines), [(0, 0, declared_len, payload[:32])]
    start = index
    while index < len(lines) and parse_int(lines[index]) != end_sentinel:
        index += 1
    payload = decode_packed_flat(lines[start:index], declared_len)
    return payload, index, [(0, 0, declared_len, payload[:32])]

def write_code_segment_table(out_path: pathlib.Path, segments, function_start_len: int) -> None:
    rows = ["segment_index\tfunction_idx\tkind\tstart\tlen\tend\tfirst_32_bytes"]
    for segment_index, start, segment_len, first_bytes in segments:
        function_idx = segment_index + 10 if segment_index < function_start_len else -1
        kind = "function" if segment_index < function_start_len else "trailer"
        first_32_bytes = " ".join(f"{byte:02x}" for byte in first_bytes)
        rows.append(
            f"{segment_index}\t{function_idx}\t{kind}\t{start}\t{segment_len}\t{start + segment_len}\t{first_32_bytes}"
        )
    out_path.write_text("\n".join(rows) + "\n")

idx = 0
while idx < len(lines) and parse_int(lines[idx]) != 9000000005:
    idx += 1
idx = expect(idx, 9000000005)
function_start_len = parse_int(lines[idx]); idx += 1
main_func_idx = parse_int(lines[idx]); idx += 1
entrypoint_offset = parse_int(lines[idx]); idx += 1
idx = expect(idx, 9000000006)
idx = expect(idx, 9000000001)
code_len = parse_int(lines[idx]); idx += 1
idx = expect(idx, 9000000002)
# split execution contract: code, idx = decode_packed_payload_at(idx, code_len, 9000000003)
code, idx, code_segments = decode_packed_payload_at(idx, code_len, 9000000003)
idx = expect(idx, 9000000003)
data_len = parse_int(lines[idx]); idx += 1
idx = expect(idx, 9000000004)
data, idx, _data_segments = decode_packed_payload_at(idx, data_len, None)

(out_dir / "stage-code.bin").write_bytes(code)
(out_dir / "stage-data.bin").write_bytes(data)
(out_dir / "entrypoint-offset.txt").write_text(f"{entrypoint_offset}\n")
(out_dir / "function-start-len.txt").write_text(f"{function_start_len}\n")
(out_dir / "main-func-idx.txt").write_text(f"{main_func_idx}\n")
write_code_segment_table(out_dir / "stage-code-segments.tsv", code_segments, function_start_len)
(out_dir / "manifest.json").write_text(
    "{\n"
    '  "target": "x86_64-unknown-linux-gnu",\n'
    f'  "code_len": {len(code)},\n'
    f'  "data_len": {len(data)},\n'
    f'  "entrypoint_offset": {entrypoint_offset},\n'
    f'  "function_start_len": {function_start_len},\n'
    f'  "main_func_idx": {main_func_idx}\n'
    "}\n"
)
PY

ACTUAL_TIMEOUT="${LSHARP_NATIVE_LINUX_X86_ACTUAL_TIMEOUT:-900}"
ACTUAL_CHUNK_SIZE="${LSHARP_NATIVE_LINUX_X86_ACTUAL_CHUNK_SIZE:-4}"
ACTUAL_CHUNK_RETRIES="${LSHARP_NATIVE_LINUX_X86_ACTUAL_CHUNK_RETRIES:-1}"
STAGE2_METADATA_START="${LSHARP_NATIVE_LINUX_X86_STAGE2_METADATA_START:-}"
STAGE2_METADATA_END="${LSHARP_NATIVE_LINUX_X86_STAGE2_METADATA_END:-}"
STAGE3_PROGRESS="${LSHARP_NATIVE_LINUX_X86_STAGE3_PROGRESS:-}"
STAGE3_PROGRESS_ONLY="${LSHARP_NATIVE_LINUX_X86_STAGE3_PROGRESS_ONLY:-}"
VM_REPLAY_LOCK_DIR="${LSHARP_NATIVE_LINUX_X86_VM_REPLAY_LOCK_DIR:-/tmp/lsharp-native-linux-x86-hostgen-vm-replay.lock}"
REPLAY_LOCK_ACQUIRED=0
if [[ -n "${STAGE3_PROGRESS_ONLY}" ]]; then
  STAGE3_PROGRESS=1
fi
mkdir -p actual-stage2 actual-stage3
cp -a actual-stage1/src actual-stage2/src
cp -a actual-stage1/src actual-stage3/src

release_actual_replay_lock() {
  local holder_pid=""
  if [[ "${REPLAY_LOCK_ACQUIRED}" -ne 1 ]]; then
    return 0
  fi
  holder_pid="$(cat "${VM_REPLAY_LOCK_DIR}/pid" 2>/dev/null || true)"
  if [[ "${holder_pid}" = "$$" ]]; then
    rm -rf "${VM_REPLAY_LOCK_DIR}"
  fi
}

acquire_actual_replay_lock() {
  local current_pid=$$
  local holder_pid=""
  local holder_work_dir=""
  while ! mkdir "${VM_REPLAY_LOCK_DIR}" 2>/dev/null; do
    holder_pid="$(cat "${VM_REPLAY_LOCK_DIR}/pid" 2>/dev/null || true)"
    holder_work_dir="$(cat "${VM_REPLAY_LOCK_DIR}/work_dir" 2>/dev/null || true)"
    if [[ "${holder_pid}" =~ ^[0-9]+$ ]] && ps -p "${holder_pid}" >/dev/null 2>&1; then
      echo "ERROR: VM actual replay lock is held: current_pid=${current_pid} holder_pid=${holder_pid} holder_work_dir=${holder_work_dir}" >&2
      ps -p "${holder_pid}" -o pid,ppid,etime,%mem,%cpu,comm,args >&2 || true
      return 90
    fi
    echo "WARN: removing stale VM actual replay lock: ${VM_REPLAY_LOCK_DIR} holder_pid=${holder_pid} current_pid=${current_pid}" >&2
    rm -rf "${VM_REPLAY_LOCK_DIR}"
  done
  REPLAY_LOCK_ACQUIRED=1
  printf '%s\n' "$$" >"${VM_REPLAY_LOCK_DIR}/pid"
  printf '%s\n' "${VM_WORK_DIR}" >"${VM_REPLAY_LOCK_DIR}/work_dir"
  date -u '+%Y-%m-%dT%H:%M:%SZ' >"${VM_REPLAY_LOCK_DIR}/started_at"
  trap release_actual_replay_lock EXIT
}

write_actual_selfregen_failure_summary() {
  local phase="$1"
  local exit_code="$2"
  local stdout_file="$3"
  local stderr_file="$4"
  local stdout_bytes=0
  local stderr_bytes=0
  if [[ -e "${stdout_file}" ]]; then
    stdout_bytes="$(wc -c <"${stdout_file}")"
  fi
  if [[ -e "${stderr_file}" ]]; then
    stderr_bytes="$(wc -c <"${stderr_file}")"
  fi
  cat >actual-selfregen-summary.json <<JSON
{
  "target": "x86_64-unknown-linux-gnu",
  "host_os": "${HOST_OS}",
  "host_arch": "${HOST_ARCH}",
  "status": "fail",
  "phase": "${phase}",
  "exit_code": ${exit_code},
  "stdout_bytes": ${stdout_bytes},
  "stderr_bytes": ${stderr_bytes}
}
JSON
}

run_actual_stage_range() {
  local stage_dir="$1"
  local stdout_file="$2"
  local stderr_file="$3"
  local chunk_start="$4"
  local chunk_end="$5"
  local include_header="$6"
  local include_tail="$7"
  local chunk_attempt=0
  local chunk_stdout=""
  local chunk_stderr=""
  local chunk_exit_code=0
  local split_mid=0

  if [[ "${chunk_end}" -le "${chunk_start}" ]]; then
    return 0
  fi

  while :; do
    chunk_stdout="$(mktemp "${stage_dir}.chunk.${chunk_start}.XXXXXX.stdout")"
    chunk_stderr="$(mktemp "${stage_dir}.chunk.${chunk_start}.XXXXXX.stderr")"
    set +e
    (cd "${stage_dir}" && timeout "${ACTUAL_TIMEOUT}" ./program.native src/App/Seed.ls "${chunk_start}" "${chunk_end}" "${include_header}" "${include_tail}" >"../${chunk_stdout}" 2>"../${chunk_stderr}")
    chunk_exit_code=$?
    set -e
    if [[ "${chunk_exit_code}" -eq 0 ]]; then
      cat "${chunk_stdout}" >>"${stdout_file}"
      cat "${chunk_stderr}" >>"${stderr_file}"
      rm -f "${chunk_stdout}" "${chunk_stderr}"
      return 0
    fi
    if [[ "${chunk_attempt}" -lt "${ACTUAL_CHUNK_RETRIES}" ]]; then
      echo "WARN: retrying chunked native run for ${stage_dir} ${chunk_start}-${chunk_end} after exit ${chunk_exit_code}" >&2
      rm -f "${chunk_stdout}" "${chunk_stderr}"
      chunk_attempt=$((chunk_attempt + 1))
      continue
    fi
    rm -f "${chunk_stdout}" "${chunk_stderr}"
    break
  done

  if [[ $((chunk_end - chunk_start)) -gt 1 ]]; then
    split_mid=$(((chunk_start + chunk_end) / 2))
    echo "WARN: splitting chunked native run for ${stage_dir} ${chunk_start}-${chunk_end} at ${split_mid} after exit ${chunk_exit_code}" >&2
    run_actual_stage_range "${stage_dir}" "${stdout_file}" "${stderr_file}" "${chunk_start}" "${split_mid}" "${include_header}" 0 || return $?
    run_actual_stage_range "${stage_dir}" "${stdout_file}" "${stderr_file}" "${split_mid}" "${chunk_end}" 0 "${include_tail}" || return $?
    return 0
  fi

  cat "${chunk_stdout}" >>"${stdout_file}" 2>/dev/null || true
  cat "${chunk_stderr}" >>"${stderr_file}" 2>/dev/null || true
  rm -f "${chunk_stdout}" "${chunk_stderr}" 2>/dev/null || true
  return "${chunk_exit_code}"
}

run_actual_stage_chunked() {
  local stage_dir="$1"
  local stdout_file="$2"
  local stderr_file="$3"
  local chunk_start=0
  local function_start_len=""
  local chunk_end=0
  local include_header=0
  local include_tail=0
  : >"${stdout_file}"
  : >"${stderr_file}"

  while :; do
    if [[ -n "${function_start_len}" && "${chunk_start}" -ge "${function_start_len}" ]]; then
      break
    fi
    chunk_end=$((chunk_start + ACTUAL_CHUNK_SIZE))
    include_header=0
    include_tail=0
    if [[ "${chunk_start}" -eq 0 ]]; then
      include_header=1
    fi
    if [[ -n "${function_start_len}" && "${chunk_end}" -ge "${function_start_len}" ]]; then
      chunk_end="${function_start_len}"
      include_tail=1
    fi

    run_actual_stage_range "${stage_dir}" "${stdout_file}" "${stderr_file}" "${chunk_start}" "${chunk_end}" "${include_header}" "${include_tail}"

    if [[ -z "${function_start_len}" ]]; then
      function_start_len="$(awk 'NR == 2 { print; exit }' "${stdout_file}")"
      if [[ -z "${function_start_len}" ]]; then
        echo "ERROR: ${stage_dir} chunked run did not emit function_start_len" >&2
        return 1
      fi
      if [[ "${chunk_end}" -ge "${function_start_len}" ]]; then
        chunk_start="${chunk_end}"
      else
        chunk_start="${chunk_end}"
      fi
    else
      chunk_start="${chunk_end}"
    fi

    if [[ "${include_tail}" -eq 1 ]]; then
      break
    fi
  done
}

collect_stage2_metadata_range() {
  local metadata_start="${STAGE2_METADATA_START}"
  local metadata_end="${STAGE2_METADATA_END}"
  local metadata_exit_code=0
  if [[ -z "${metadata_start}" ]]; then
    return 0
  fi
  if [[ -z "${metadata_end}" ]]; then
    metadata_end="$((metadata_start + 1))"
  fi
  set +e
  (cd actual-stage1 && timeout "${ACTUAL_TIMEOUT}" ./program.native src/App/Seed.ls "${metadata_start}" "${metadata_end}" 0 0 metadata >"../actual-stage2-metadata.txt" 2>"../actual-stage2-metadata-stderr.txt")
  metadata_exit_code=$?
  set -e
  if [[ "${metadata_exit_code}" -ne 0 ]]; then
    echo "ERROR: actual stage2 metadata range ${metadata_start}-${metadata_end} failed with status ${metadata_exit_code}" >&2
    return "${metadata_exit_code}"
  fi
}

collect_stage3_progress_markers() {
  local progress_exit_code=0
  if [[ -z "${STAGE3_PROGRESS}" ]]; then
    return 0
  fi
  set +e
  (cd actual-stage2 && timeout "${ACTUAL_TIMEOUT}" ./program.native src/App/Seed.ls 0 1 0 0 "" "" progress >"../actual-stage3-progress.txt" 2>"../actual-stage3-progress-stderr.txt")
  progress_exit_code=$?
  set -e
  if [[ "${progress_exit_code}" -ne 0 ]]; then
    echo "ERROR: actual stage3 progress failed with status ${progress_exit_code}" >&2
    return "${progress_exit_code}"
  fi
}

acquire_actual_replay_lock
python3 materialize-actual-bundle.py actual-stage1 stage1-code.bin entrypoint-offset.txt
set +e
run_actual_stage_chunked actual-stage1 actual-stage2-stdout.txt actual-stage2-stderr.txt
actual_stage2_exit_code=$?
set -e
if [[ "${actual_stage2_exit_code}" -ne 0 ]]; then
  write_actual_selfregen_failure_summary "stage2-run" "${actual_stage2_exit_code}" actual-stage2-stdout.txt actual-stage2-stderr.txt
  exit "${actual_stage2_exit_code}"
fi
set +e
collect_stage2_metadata_range
actual_stage2_metadata_exit_code=$?
set -e
if [[ "${actual_stage2_metadata_exit_code}" -ne 0 ]]; then
  write_actual_selfregen_failure_summary "stage2-metadata" "${actual_stage2_metadata_exit_code}" actual-stage2-metadata.txt actual-stage2-metadata-stderr.txt
  exit "${actual_stage2_metadata_exit_code}"
fi
set +e
python3 decode-actual-transport.py actual-stage2-stdout.txt actual-stage2
actual_stage2_decode_exit_code=$?
set -e
if [[ "${actual_stage2_decode_exit_code}" -ne 0 ]]; then
  write_actual_selfregen_failure_summary "stage2-decode" "${actual_stage2_decode_exit_code}" actual-stage2-stdout.txt actual-stage2-stderr.txt
  exit "${actual_stage2_decode_exit_code}"
fi
cp actual-stage2/stage-code.bin actual-stage2/stage2-code.bin
cp actual-stage2/stage-data.bin actual-stage2/stage2-data.bin
python3 materialize-actual-bundle.py actual-stage2 stage-code.bin entrypoint-offset.txt
set +e
collect_stage3_progress_markers
actual_stage3_progress_exit_code=$?
set -e
if [[ "${actual_stage3_progress_exit_code}" -ne 0 ]]; then
  write_actual_selfregen_failure_summary "stage3-progress" "${actual_stage3_progress_exit_code}" actual-stage3-progress.txt actual-stage3-progress-stderr.txt
  exit "${actual_stage3_progress_exit_code}"
fi
if [[ -n "${STAGE3_PROGRESS_ONLY}" ]]; then
  cat >actual-selfregen-summary.json <<JSON
{
  "target": "x86_64-unknown-linux-gnu",
  "host_os": "${HOST_OS}",
  "host_arch": "${HOST_ARCH}",
  "status": "diagnostic",
  "phase": "stage3-progress",
  "progress_stdout_bytes": $(wc -c <actual-stage3-progress.txt),
  "progress_stderr_bytes": $(wc -c <actual-stage3-progress-stderr.txt)
}
JSON
  exit 0
fi
set +e
run_actual_stage_chunked actual-stage2 actual-stage3-stdout.txt actual-stage3-stderr.txt
actual_stage3_exit_code=$?
set -e
if [[ "${actual_stage3_exit_code}" -ne 0 ]]; then
  write_actual_selfregen_failure_summary "stage3-run" "${actual_stage3_exit_code}" actual-stage3-stdout.txt actual-stage3-stderr.txt
  exit "${actual_stage3_exit_code}"
fi
set +e
python3 decode-actual-transport.py actual-stage3-stdout.txt actual-stage3
actual_stage3_decode_exit_code=$?
set -e
if [[ "${actual_stage3_decode_exit_code}" -ne 0 ]]; then
  write_actual_selfregen_failure_summary "stage3-decode" "${actual_stage3_decode_exit_code}" actual-stage3-stdout.txt actual-stage3-stderr.txt
  exit "${actual_stage3_decode_exit_code}"
fi

if [[ -s actual-stage2-stderr.txt || -s actual-stage3-stderr.txt ]]; then
  echo "ERROR: actual self-regeneration stderr is not empty" >&2
  exit 1
fi
if ! cmp -s actual-stage2-stdout.txt actual-stage3-stdout.txt; then
  echo "ERROR: actual stage2/stage3 transport payload mismatch" >&2
  exit 1
fi

actual_stage2_stdout_sha="$(sha256sum actual-stage2-stdout.txt | awk '{print $1}')"
actual_stage3_stdout_sha="$(sha256sum actual-stage3-stdout.txt | awk '{print $1}')"
cat >actual-selfregen-summary.json <<JSON
{
  "target": "x86_64-unknown-linux-gnu",
  "host_os": "${HOST_OS}",
  "host_arch": "${HOST_ARCH}",
  "status": "pass",
  "scope": "host-generated stage1 x86 payload executes stage2/stage3 native self-regeneration in local Linux x86_64 VM",
  "stage2_stdout_sha256": "${actual_stage2_stdout_sha}",
  "stage3_stdout_sha256": "${actual_stage3_stdout_sha}",
  "stage2_code_len": $(wc -c <actual-stage2/stage-code.bin),
  "stage3_code_len": $(wc -c <actual-stage3/stage-code.bin)
}
JSON
VM_SCRIPT
vm_exec_status=$?
set -e

for file in program.s runtime.s program.o argv-program.o argv-char-program.o print-program.o vector-program.o ref-program.o substring-program.o string-concat-program.o map-program.o map-size-program.o file-exists-program.o code-program.o runtime.o linker-response.txt program.native stdout.txt stderr.txt summary.json object-runtime.s object-runtime.o object-linker-response.txt object-program.native object-stdout.txt object-stderr.txt object-summary.json argv-object-linker-response.txt argv-object-program.native argv-object-stdout.txt argv-object-stderr.txt argv-object-summary.json argv-char-object-linker-response.txt argv-char-object-program.native argv-char-object-stdout.txt argv-char-object-stderr.txt argv-char-object-summary.json print-object-linker-response.txt print-object-program.native print-object-stdout.txt print-object-stderr.txt print-object-summary.json vector-object-linker-response.txt vector-object-program.native vector-object-stdout.txt vector-object-stderr.txt vector-object-summary.json ref-object-linker-response.txt ref-object-program.native ref-object-stdout.txt ref-object-stderr.txt ref-object-summary.json substring-object-linker-response.txt substring-object-program.native substring-object-stdout.txt substring-object-stderr.txt substring-object-summary.json string-concat-object-linker-response.txt string-concat-object-program.native string-concat-object-stdout.txt string-concat-object-stderr.txt string-concat-object-summary.json map-object-linker-response.txt map-object-program.native map-object-stdout.txt map-object-stderr.txt map-object-summary.json map-size-object-linker-response.txt map-size-object-program.native map-size-object-stdout.txt map-size-object-stderr.txt map-size-object-summary.json file-exists-target.txt file-exists-object-linker-response.txt file-exists-object-program.native file-exists-object-stdout.txt file-exists-object-stderr.txt file-exists-object-summary.json actual-stage2-stdout.txt actual-stage2-stderr.txt actual-stage2-metadata.txt actual-stage2-metadata-stderr.txt actual-stage3-progress.txt actual-stage3-progress-stderr.txt actual-stage3-stdout.txt actual-stage3-stderr.txt actual-selfregen-summary.json; do
  if limactl shell "${VM_NAME}" -- test -e "${VM_WORK_DIR}/${file}"; then
    limactl copy "${VM_NAME}:${VM_WORK_DIR}/${file}" "${ARTIFACT_DIR}/${file}"
  fi
done

copy_actual_stage_debug_artifact() {
  local stage_dir="$1"
  local debug_dir="$2"
  if ! limactl shell "${VM_NAME}" -- test -d "${VM_WORK_DIR}/${stage_dir}"; then
    return 0
  fi
  rm -rf "${ARTIFACT_DIR}/${debug_dir}"
  mkdir -p "${ARTIFACT_DIR}/${debug_dir}"
  for file in manifest.json stage-code.bin stage-data.bin stage-code-segments.tsv stage1-code.bin stage2-code.bin stage2-data.bin entrypoint-offset.txt function-start-len.txt main-func-idx.txt stage-entry-ir-trace.txt program.s runtime.s program.o runtime.o linker-response.txt program.native; do
    if limactl shell "${VM_NAME}" -- test -e "${VM_WORK_DIR}/${stage_dir}/${file}"; then
      limactl copy "${VM_NAME}:${VM_WORK_DIR}/${stage_dir}/${file}" "${ARTIFACT_DIR}/${debug_dir}/${file}"
    fi
  done
  if limactl shell "${VM_NAME}" -- test -d "${VM_WORK_DIR}/${stage_dir}/src"; then
    limactl copy "${VM_NAME}:${VM_WORK_DIR}/${stage_dir}/src" "${ARTIFACT_DIR}/${debug_dir}"
  fi
}

copy_actual_stage_debug_artifact actual-stage1 stage1-debug
copy_actual_stage_debug_artifact actual-stage2 stage2-debug
copy_actual_stage_debug_artifact actual-stage3 stage3-debug

if [[ "${vm_exec_status}" -ne 0 ]]; then
  echo "ERROR: native Linux x86_64 hostgen -> VM exec smoke failed with status ${vm_exec_status}" >&2
  echo "VM workdir kept for failure diagnostics: ${VM_WORK_DIR}" >&2
  exit "${vm_exec_status}"
fi

if [[ "${KEEP_VM_WORK_DIR}" = "1" ]]; then
  echo "VM workdir kept by LSHARP_NATIVE_LINUX_X86_KEEP_VM_WORK_DIR=1: ${VM_WORK_DIR}"
else
  limactl shell "${VM_NAME}" -- rm -rf "${VM_WORK_DIR}"
  echo "VM workdir removed after successful evidence copy: ${VM_WORK_DIR}"
fi

echo "native Linux x86_64 hostgen -> VM exec evidence collected."
