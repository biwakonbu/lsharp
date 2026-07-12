#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
VM_NAME="${LSHARP_NATIVE_LINUX_X86_LIMA_VM:-lsharp-linux-x86}"
KEEP_ARTIFACTS="${LSHARP_NATIVE_LINUX_X86_KEEP_WRITE_BYTES_ARTIFACTS:-0}"
HOST_WORK_DIR="$(mktemp -d "${TMPDIR:-/tmp}/lsharp-linux-x86-write-bytes.XXXXXX")"
VM_WORK_DIR="/tmp/lsharp-linux-x86-write-bytes-$$"
VM_WORK_DIR_CREATED=0

cleanup() {
  if [[ "${VM_WORK_DIR_CREATED}" -eq 1 && "${KEEP_ARTIFACTS}" != "1" ]] && command -v limactl >/dev/null 2>&1; then
    limactl shell "${VM_NAME}" -- rm -rf "${VM_WORK_DIR}" >/dev/null 2>&1 || true
  fi
  if [[ "${KEEP_ARTIFACTS}" != "1" ]]; then
    rm -rf "${HOST_WORK_DIR}"
  fi
}
trap cleanup EXIT

if [[ "$(uname -s)" != "Darwin" || "$(uname -m)" != "arm64" ]]; then
  echo "ERROR: this local smoke requires macOS arm64" >&2
  exit 1
fi
if ! command -v limactl >/dev/null 2>&1; then
  echo "ERROR: limactl is required for the Linux x86_64 local smoke" >&2
  exit 1
fi

cd "${ROOT_DIR}"
vm_status="$(limactl list "${VM_NAME}" --format '{{.Status}}' 2>/dev/null || true)"
if [[ "${vm_status}" != "Running" ]]; then
  limactl start --tty=false "${VM_NAME}"
fi

OBJECT_PATH="${HOST_WORK_DIR}/write-file-bytes.o"
LSHARP_NATIVE_LINUX_X86_WRITE_FILE_BYTES_OBJECT_ARTIFACT="${OBJECT_PATH}" \
  cargo test -q -p lsharp-wasm --test e2e \
    native_linux_x86_host_generates_write_file_bytes_elf_object_artifact \
    -- --ignored --nocapture

limactl shell "${VM_NAME}" -- rm -rf "${VM_WORK_DIR}"
limactl shell "${VM_NAME}" -- mkdir -p "${VM_WORK_DIR}"
VM_WORK_DIR_CREATED=1
limactl copy "${OBJECT_PATH}" "${VM_NAME}:${VM_WORK_DIR}/program.o"

limactl shell "${VM_NAME}" -- bash -s -- "${VM_WORK_DIR}" <<'VM_SCRIPT'
set -euo pipefail
work_dir="$1"
cd "${work_dir}"

cat > object-runtime.s <<'ASM'
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
cc program.o object-runtime.o -o program.native

set +e
./program.native output.bin >stdout.txt 2>stderr.txt
actual_exit_code=$?
set -e
if [[ "${actual_exit_code}" -ne 0 ]]; then
  echo "ERROR: write-file-bytes exit=${actual_exit_code}, expected 0" >&2
  cat stdout.txt >&2
  cat stderr.txt >&2
  exit 1
fi
if [[ -s stderr.txt ]]; then
  echo "ERROR: write-file-bytes emitted stderr" >&2
  cat stderr.txt >&2
  exit 1
fi
actual_bytes="$(od -An -v -t x1 output.bin | tr -s ' ' | tr '\n' ' ' | sed 's/^ //; s/ $//')"
if [[ "${actual_bytes}" != "00 61 73 6d" ]]; then
  echo "ERROR: write-file-bytes payload='${actual_bytes}', expected '00 61 73 6d'" >&2
  exit 1
fi
printf 'Linux x86_64 write-file-bytes smoke passed: exit=%s bytes=%s\n' "${actual_exit_code}" "${actual_bytes}"
VM_SCRIPT
