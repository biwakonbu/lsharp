#!/usr/bin/env python3

import os
import pathlib
import platform
import subprocess
import sys


if len(sys.argv) != 4:
    raise SystemExit(f"usage: {sys.argv[0]} <stage-dir> <code-file> <entrypoint-file>")
if platform.system() != "Darwin" or platform.machine() != "arm64":
    raise SystemExit("macOS arm64 native bundle materialization requires Darwin arm64")

stage_dir = pathlib.Path(sys.argv[1])
code_name = sys.argv[2]
entrypoint = int((stage_dir / sys.argv[3]).read_text(encoding="utf-8").strip())
skip_argv0 = os.environ.get("LSHARP_NATIVE_MACOS_AARCH64_SKIP_ARGV0", "0") == "1"
code_path = stage_dir / code_name
code_bytes = code_path.read_bytes()
code_len = len(code_bytes)
data_name = "stage1-data.bin" if (stage_dir / "stage1-data.bin").exists() else "stage-data.bin"
data_path = stage_dir / data_name
if not data_path.exists():
    data_path.write_bytes(b"")
data_bytes = data_path.read_bytes()

if entrypoint < 0 or entrypoint > code_len:
    raise SystemExit(f"entrypoint out of range: offset={entrypoint} len={code_len}")


def align_up(value, alignment):
    if value == 0:
        return 0
    return ((value + alignment - 1) // alignment) * alignment


def byte_text(data, empty):
    if not data:
        return empty
    return ", ".join(f"0x{byte:02x}" for byte in data)


data_base = 1024
data_frontier = align_up(data_base + len(data_bytes), 8)
alloc_size = max(data_frontier, 0x0001_0000) + 0x1_0000_0000
alloc_size = max(alloc_size, 0x1_0000_0000)
prefix_text = byte_text(code_bytes[:entrypoint], "0x1f, 0x20, 0x03, 0xd5")
suffix_text = byte_text(code_bytes[entrypoint:], "")
data_text = byte_text(data_bytes, "0x00")
argv_projection = "    sub x19, x19, #1\n    add x20, x20, #8\n" if skip_argv0 else ""

program_asm = f""".section __TEXT,__text
.extern _calloc
.extern _memcpy
.extern _strlen
.globl _main
_main:
    stp x29, x30, [sp, #-112]!
    mov x29, sp
    stp x19, x20, [sp, #16]
    stp x21, x22, [sp, #32]
    stp x23, x24, [sp, #48]
    stp x25, x26, [sp, #64]
    stp x27, x28, [sp, #80]
    mov x19, x0
    mov x20, x1
    mov x0, #1
    adrp x8, _lsharp_alloc_size@PAGE
    ldr x1, [x8, _lsharp_alloc_size@PAGEOFF]
    bl _calloc
    cbz x0, _lsharp_alloc_fail
    mov x21, x0
    adrp x8, _lsharp_heap_frontier@PAGE
    ldr x22, [x8, _lsharp_heap_frontier@PAGEOFF]
    adrp x8, _lsharp_data_len@PAGE
    ldr x2, [x8, _lsharp_data_len@PAGEOFF]
    cbz x2, _lsharp_copy_argv
    add x0, x21, #{data_base}
    adrp x1, _lsharp_data@PAGE
    add x1, x1, _lsharp_data@PAGEOFF
    bl _memcpy
_lsharp_copy_argv:
    mov x23, x20
    add x25, x21, x22
    lsl x8, x19, #3
    add x8, x8, #7
    lsr x8, x8, #3
    lsl x8, x8, #3
    add x22, x22, x8
    mov x26, #0
_lsharp_copy_argv_loop:
    cmp x26, x19
    b.ge _lsharp_copy_argv_done
    ldr x24, [x23, x26, lsl #3]
    mov x0, x24
    bl _strlen
    mov x27, x0
    add x8, x27, #8
    add x8, x8, #7
    lsr x8, x8, #3
    lsl x8, x8, #3
    mov x28, x22
    add x22, x22, x8
    add x3, x21, x28
    mov w4, #1
    str w4, [x3]
    str w27, [x3, #4]
    add x0, x3, #8
    mov x1, x24
    mov x2, x27
    bl _memcpy
    mov x8, #-0x8000000000000000
    orr x8, x28, x8
    str x8, [x25, x26, lsl #3]
    add x26, x26, #1
    b _lsharp_copy_argv_loop
_lsharp_copy_argv_done:
    mov x20, x25
{argv_projection}_lsharp_call_entry:
    mov x0, x19
    mov x1, x20
    mov x2, #0
    mov x3, #0
    mov x4, #0
    mov x5, #0
    mov x6, #0
    mov x7, #0
    mov x9, #0
    mov x10, #0
    adrp x27, _lsharp_root_stack@PAGE
    add x27, x27, _lsharp_root_stack@PAGEOFF
    mov x28, x27
    bl _lsharp_entry
    ldp x27, x28, [sp, #80]
    ldp x25, x26, [sp, #64]
    ldp x23, x24, [sp, #48]
    ldp x21, x22, [sp, #32]
    ldp x19, x20, [sp, #16]
    ldp x29, x30, [sp], #112
    ret
_lsharp_alloc_fail:
    mov w0, #1
    ldp x27, x28, [sp, #80]
    ldp x25, x26, [sp, #64]
    ldp x23, x24, [sp, #48]
    ldp x21, x22, [sp, #32]
    ldp x19, x20, [sp, #16]
    ldp x29, x30, [sp], #112
    ret
.section __TEXT,__const
.p2align 3
_lsharp_alloc_size:
    .quad {alloc_size}
_lsharp_heap_frontier:
    .quad {data_frontier}
_lsharp_data_len:
    .quad {len(data_bytes)}
.section __TEXT,__text
.globl _lsharp_bundle
_lsharp_bundle:
    .byte {prefix_text}
.globl _lsharp_entry
_lsharp_entry:
    .byte {suffix_text}
.section __DATA,__data
.p2align 3
_lsharp_data:
    .byte {data_text}
.zerofill __DATA,__bss,_lsharp_root_stack,0x800000,3
"""

(stage_dir / "program.s").write_text(program_asm, encoding="utf-8")
(stage_dir / "runtime.s").write_text(
    ".section __TEXT,__text\n.globl _lsharp_runtime_stub\n_lsharp_runtime_stub:\n    ret\n",
    encoding="utf-8",
)
subprocess.run(
    ["clang", "-arch", "arm64", "-c", "program.s", "-o", "program.o"],
    cwd=stage_dir,
    check=True,
)
subprocess.run(
    ["clang", "-arch", "arm64", "-c", "runtime.s", "-o", "runtime.o"],
    cwd=stage_dir,
    check=True,
)
(stage_dir / "linker-response.txt").write_text(
    "-o\nprogram.native\nprogram.o\nruntime.o\n",
    encoding="utf-8",
)
for _ in range(2):
    subprocess.run(
        ["clang", "-Wl,-stack_size,0x08000000", "@linker-response.txt"],
        cwd=stage_dir,
        check=True,
    )

codesign_identity = os.environ.get(
    "LSHARP_NATIVE_MACOS_AARCH64_CODESIGN_IDENTITY", ""
).strip()
if codesign_identity:
    try:
        subprocess.run(
            [
                "codesign",
                "--force",
                "--sign",
                codesign_identity,
                "--timestamp=none",
                "program.native",
            ],
            cwd=stage_dir,
            check=True,
            capture_output=True,
        )
    except subprocess.CalledProcessError as error:
        diagnostic = error.stderr.decode("utf-8", "replace").strip()
        if diagnostic:
            raise SystemExit(f"codesign failed: {diagnostic}") from error
        raise SystemExit(f"codesign failed with exit={error.returncode}") from error
