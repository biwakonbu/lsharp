use std::path::Path;

#[cfg(all(target_os = "macos", target_arch = "aarch64"))]
use std::path::PathBuf;

use lsharp_ir::Module;

const NATIVE_BACKEND_ERROR_CODE: &str = "LS4001";

#[path = "native_emitter.rs"]
mod native_emitter;

fn native_backend_error(message: impl std::fmt::Display) -> miette::Report {
    miette::miette!("[{NATIVE_BACKEND_ERROR_CODE}] {message}")
}

#[cfg(all(target_os = "macos", target_arch = "aarch64"))]
pub(crate) fn compile_native_executable(module: &Module, output_path: &Path) -> miette::Result<()> {
    let asm = native_module_assembly(module).map_err(native_backend_error)?;
    let asm_path = native_temp_asm_path(output_path).map_err(native_backend_error)?;
    let temporary_output_path =
        native_temp_output_path(output_path).map_err(native_backend_error)?;

    std::fs::write(&asm_path, asm)
        .map_err(|e| native_backend_error(format!("{}: {}", asm_path.display(), e)))?;
    let linker_output = match std::process::Command::new("cc")
        .arg(&asm_path)
        .arg("-o")
        .arg(&temporary_output_path)
        .output()
    {
        Ok(output) => output,
        Err(error) => {
            let _ = std::fs::remove_file(&asm_path);
            let _ = std::fs::remove_file(&temporary_output_path);
            return Err(native_backend_error(format!(
                "native linker 起動失敗: {error}"
            )));
        }
    };
    let _ = std::fs::remove_file(&asm_path);

    if !linker_output.status.success() {
        let _ = std::fs::remove_file(&temporary_output_path);
        let stderr = String::from_utf8_lossy(&linker_output.stderr);
        return Err(native_backend_error(format!(
            "native linker が失敗しました: status={}; stderr={stderr}",
            linker_output.status
        )));
    }

    if let Err(error) = lsharp_wasm::component_adapter::sync_artifact_file(&temporary_output_path) {
        let _ = std::fs::remove_file(&temporary_output_path);
        return Err(native_backend_error(format!(
            "native artifact file の同期に失敗しました ({}): {error}",
            temporary_output_path.display()
        )));
    }

    std::fs::rename(&temporary_output_path, output_path).map_err(|error| {
        let _ = std::fs::remove_file(&temporary_output_path);
        native_backend_error(format!(
            "native artifact の atomic replacement に失敗しました ({}): {error}",
            output_path.display()
        ))
    })?;

    lsharp_wasm::component_adapter::sync_artifact_parent(output_path).map_err(|error| {
        native_backend_error(format!(
            "native artifact parent directory の同期に失敗しました ({}): {error}",
            output_path.display()
        ))
    })?;

    Ok(())
}

#[cfg(not(all(target_os = "macos", target_arch = "aarch64")))]
pub(crate) fn compile_native_executable(
    _module: &Module,
    output_path: &Path,
) -> miette::Result<()> {
    Err(native_backend_error(format!(
        "native backend は未サポートです。この host の Rust driver native path は aarch64-apple-darwin のみ生成できます: {}",
        output_path.display()
    )))
}

#[cfg(all(target_os = "macos", target_arch = "aarch64"))]
fn native_main_index(module: &Module) -> miette::Result<usize> {
    module
        .functions
        .iter()
        .position(|func| func.name == "main" && func.is_export)
        .or_else(|| module.functions.iter().position(|func| func.name == "main"))
        .ok_or_else(|| miette::miette!("native backend には main 関数が必要です"))
}

#[cfg(all(target_os = "macos", target_arch = "aarch64"))]
const NATIVE_IR_IMPORT_COUNT: u32 = 17;

#[cfg(all(target_os = "macos", target_arch = "aarch64"))]
fn native_module_assembly(module: &Module) -> miette::Result<String> {
    let reachable_functions = native_reachable_function_indexes(module)?;
    let mut asm = String::from(
        ".section __TEXT,__cstring\n\
         L_lsharp_fmt_i64:\n\
             .asciz \"%lld\\n\"\n\
         .data\n\
         .p2align 3\n\
         _lsharp_heap_next:\n\
             .quad 0\n\
         .zerofill __DATA,__bss,_lsharp_heap,1048576,4\n\
         .text\n",
    );
    for function_index in reachable_functions {
        let function = &module.functions[function_index];
        native_emitter::NativeFunctionEmitter::new(module, function_index, function)
            .emit(&mut asm)?;
    }
    Ok(asm)
}

#[cfg(all(target_os = "macos", target_arch = "aarch64"))]
fn native_reachable_function_indexes(module: &Module) -> miette::Result<Vec<usize>> {
    let main_index = native_main_index(module)?;
    let mut reachable = vec![false; module.functions.len()];
    let mut pending = vec![main_index];

    while let Some(function_index) = pending.pop() {
        if reachable[function_index] {
            continue;
        }
        reachable[function_index] = true;

        for instruction in &module.functions[function_index].body {
            let lsharp_ir::Instruction::Call(index) = instruction else {
                continue;
            };
            if *index < NATIVE_IR_IMPORT_COUNT {
                continue;
            }
            let callee_index = (*index - NATIVE_IR_IMPORT_COUNT) as usize;
            if callee_index >= module.functions.len() {
                continue;
            }
            pending.push(callee_index);
        }
    }

    Ok(reachable
        .into_iter()
        .enumerate()
        .filter_map(|(index, is_reachable)| is_reachable.then_some(index))
        .collect())
}

#[cfg(all(target_os = "macos", target_arch = "aarch64"))]
fn native_function_label(module: &Module, function_index: usize) -> String {
    let function = &module.functions[function_index];
    if function.name == "main" && function.is_export {
        "_main".to_string()
    } else {
        format!(
            "_lsharp_fn_{function_index}_{}",
            native_sanitize_symbol(&function.name)
        )
    }
}

#[cfg(all(target_os = "macos", target_arch = "aarch64"))]
fn native_sanitize_symbol(name: &str) -> String {
    name.chars()
        .map(|ch| {
            if ch.is_ascii_alphanumeric() || ch == '_' {
                ch
            } else {
                '_'
            }
        })
        .collect()
}

#[cfg(all(target_os = "macos", target_arch = "aarch64"))]
fn native_frame_size(function: &lsharp_ir::Function) -> usize {
    let slots = function.params.len() + function.locals.len();
    let bytes = slots * 8;
    bytes.div_ceil(16) * 16
}

#[cfg(all(target_os = "macos", target_arch = "aarch64"))]
fn native_local_offset(index: u32) -> i32 {
    -8 * (index as i32 + 1)
}

#[cfg(all(target_os = "macos", target_arch = "aarch64"))]
fn native_emit_i64_const(asm: &mut String, register: &str, value: i64) {
    let raw = value as u64;
    let chunks = [
        raw & 0xffff,
        (raw >> 16) & 0xffff,
        (raw >> 32) & 0xffff,
        (raw >> 48) & 0xffff,
    ];
    asm.push_str(&format!("    movz {register}, #{}\n", chunks[0]));
    for (idx, chunk) in chunks.iter().enumerate().skip(1) {
        if *chunk != 0 {
            asm.push_str(&format!(
                "    movk {register}, #{chunk}, lsl #{}\n",
                idx * 16
            ));
        }
    }
}

#[cfg(all(target_os = "macos", target_arch = "aarch64"))]
fn native_emit_heap_alloc(asm: &mut String) {
    asm.push_str("    add x9, x9, #7\n");
    asm.push_str("    lsr x9, x9, #3\n");
    asm.push_str("    lsl x9, x9, #3\n");
    asm.push_str("    adrp x10, _lsharp_heap_next@PAGE\n");
    asm.push_str("    add x10, x10, _lsharp_heap_next@PAGEOFF\n");
    asm.push_str("    ldr x11, [x10]\n");
    asm.push_str("    add x12, x11, x9\n");
    asm.push_str("    str x12, [x10]\n");
    asm.push_str("    mov x9, x11\n");
}

#[cfg(all(target_os = "macos", target_arch = "aarch64"))]
fn native_emit_heap_base_plus_offset(asm: &mut String, offset_register: &str, target: &str) {
    let wide_register = offset_register;
    let narrow_register = offset_register.replacen('x', "w", 1);
    asm.push_str(&format!("    uxtw {wide_register}, {narrow_register}\n"));
    asm.push_str(&format!("    adrp {target}, _lsharp_heap@PAGE\n"));
    asm.push_str(&format!(
        "    add {target}, {target}, _lsharp_heap@PAGEOFF\n"
    ));
    asm.push_str(&format!("    add {target}, {target}, {wide_register}\n"));
}

#[cfg(all(target_os = "macos", target_arch = "aarch64"))]
fn native_emit_heap_base_plus_offset_and_memarg(
    asm: &mut String,
    offset_register: &str,
    target: &str,
    memarg_offset: u32,
) {
    native_emit_heap_base_plus_offset(asm, offset_register, target);
    if memarg_offset > 0 {
        native_emit_i64_const(asm, "x12", i64::from(memarg_offset));
        asm.push_str(&format!("    add {target}, {target}, x12\n"));
    }
}

#[cfg(all(target_os = "macos", target_arch = "aarch64"))]
fn native_emit_push(asm: &mut String, register: &str) {
    asm.push_str("    sub sp, sp, #16\n");
    asm.push_str(&format!("    str {register}, [sp]\n"));
}

#[cfg(all(target_os = "macos", target_arch = "aarch64"))]
fn native_emit_pop(asm: &mut String, register: &str) {
    asm.push_str(&format!("    ldr {register}, [sp]\n"));
    asm.push_str("    add sp, sp, #16\n");
}

#[cfg(all(target_os = "macos", target_arch = "aarch64"))]
fn native_temp_asm_path(output_path: &Path) -> miette::Result<PathBuf> {
    let timestamp = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_err(|e| miette::miette!("native temp path 作成失敗: {e}"))?
        .as_nanos();
    let output_name = output_path
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or("a.out");

    Ok(std::env::temp_dir().join(format!(
        "lsharp-native-{}-{timestamp}-{output_name}.s",
        std::process::id()
    )))
}

#[cfg(all(target_os = "macos", target_arch = "aarch64"))]
fn native_temp_output_path(output_path: &Path) -> miette::Result<PathBuf> {
    let timestamp = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_err(|e| miette::miette!("native output temp path 作成失敗: {e}"))?
        .as_nanos();
    let parent = output_path.parent().unwrap_or_else(|| Path::new("."));
    let output_name = output_path
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or("a.out");

    Ok(parent.join(format!(
        ".{output_name}.tmp-{}-{timestamp}",
        std::process::id()
    )))
}

#[cfg(all(test, target_os = "macos", target_arch = "aarch64"))]
#[path = "native_tests.rs"]
mod tests;
