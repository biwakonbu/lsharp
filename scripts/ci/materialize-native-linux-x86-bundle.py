import os
import pathlib
import subprocess
import sys

stage_dir = pathlib.Path(sys.argv[1])
code_name = sys.argv[2]
entrypoint = int((stage_dir / sys.argv[3]).read_text().strip())
actual_heap_bytes = int(os.environ.get("LSHARP_NATIVE_LINUX_X86_ACTUAL_HEAP_BYTES", "4294967296"))
skip_argv0 = os.environ.get("LSHARP_NATIVE_LINUX_X86_SKIP_ARGV0", "0") == "1"
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
argv_adjust = "    dec %r14\n    add $8, %r15\n" if skip_argv0 else ""
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
{argv_adjust}    mov %r14, %r12
    movabs ${actual_heap_bytes}, %rdi
    mov $1, %rsi
    call calloc@PLT
    test %rax, %rax
    je .Lalloc_fail
    mov %rax, %r14
    movabs ${actual_heap_bytes}, %rcx
    mov %rcx, 8(%r14)
    mov $8192, %rcx
    mov %rcx, (%r14)
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
