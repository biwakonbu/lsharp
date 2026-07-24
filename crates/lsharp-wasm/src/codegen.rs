//! IR -> Wasm バイナリ生成

use lsharp_ir::{Instruction, Module};
use lsharp_syntax::span::Span;
use wasm_encoder::{
    CodeSection, EntityType, ExportKind, ExportSection, FunctionSection, ImportSection,
    MemorySection, MemoryType, TypeSection, ValType,
};

/// Wasm コード生成エラー
#[derive(Debug, Clone, thiserror::Error)]
pub enum CodegenError {
    #[error("コード生成エラー: {msg}")]
    Error { msg: String },
}

impl CodegenError {
    pub fn code(&self) -> &'static str {
        "LS4001"
    }

    pub fn span(&self) -> Option<Span> {
        None
    }
}

/// IR モジュールを Wasm バイナリに変換
pub fn emit_wasm(module: &Module) -> Result<Vec<u8>, CodegenError> {
    let mut wasm_module = wasm_encoder::Module::new();

    // === Type Section ===
    let mut types = TypeSection::new();

    // Type 0: print 関数の型 (i64) -> ()
    types.ty().function(vec![ValType::I64], vec![]);

    // Type 1: __alloc 関数の型 (i64) -> (i64)
    let alloc_type_idx = types.len();
    types.ty().function(vec![ValType::I64], vec![ValType::I64]);

    // Type 2: __string_concat 関数の型 (i64, i64) -> (i64)
    let string_concat_type_idx = types.len();
    types
        .ty()
        .function(vec![ValType::I64, ValType::I64], vec![ValType::I64]);

    // Type 3: __string_eq 関数の型 (i64, i64) -> (i64)
    let string_eq_type_idx = types.len();
    types
        .ty()
        .function(vec![ValType::I64, ValType::I64], vec![ValType::I64]);

    // Type 4: print-string 関数の型 (i64) -> ()
    let print_string_type_idx = types.len();
    types.ty().function(vec![ValType::I64], vec![]);

    // Type 5: proc-exit 関数の型 (i32) -> ()
    let proc_exit_type_idx = types.len();
    types.ty().function(vec![ValType::I32], vec![]);

    // Type 6: command-line-args 関数の型 () -> (i64)
    let command_line_args_type_idx = types.len();
    types.ty().function(vec![], vec![ValType::I64]);

    // Type 7: command-line-arg 関数の型 (i64) -> (i64)
    let command_line_arg_type_idx = types.len();
    types.ty().function(vec![ValType::I64], vec![ValType::I64]);

    // Type 8: read-stdin 関数の型 () -> (i64)
    let read_stdin_type_idx = types.len();
    types.ty().function(vec![], vec![ValType::I64]);

    // ユーザー定義関数の型を追加
    let mut func_type_indices: Vec<u32> = Vec::new();
    for func in &module.functions {
        let type_idx = types.len();
        let params: Vec<ValType> = func
            .params
            .iter()
            .map(|t| crate::emit::ir_to_wasm_valtype(*t))
            .collect();
        let results = vec![crate::emit::ir_to_wasm_valtype(func.result)];
        types.ty().function(params, results);
        func_type_indices.push(type_idx);
    }
    wasm_module.section(&types);

    // === Import Section ===
    let mut imports = ImportSection::new();
    // print: (i64) -> ()  (type index 0)
    imports.import("env", "print", EntityType::Function(0));
    // __alloc: (i64) -> (i64)  (type index 1) - Bump Allocator スタブ
    imports.import("env", "__alloc", EntityType::Function(alloc_type_idx));
    // __string_concat: (i64, i64) -> (i64) - 文字列結合
    imports.import(
        "env",
        "__string_concat",
        EntityType::Function(string_concat_type_idx),
    );
    // __string_eq: (i64, i64) -> (i64) - 文字列比較
    imports.import(
        "env",
        "__string_eq",
        EntityType::Function(string_eq_type_idx),
    );
    // print-string: (i64) -> () - 文字列出力
    imports.import(
        "env",
        "print-string",
        EntityType::Function(print_string_type_idx),
    );
    // proc-exit: (i32) -> () - プロセス終了
    imports.import("env", "proc-exit", EntityType::Function(proc_exit_type_idx));
    // __int_to_string: (i64) -> (i64) - 整数→文字列変換
    imports.import(
        "env",
        "__int_to_string",
        EntityType::Function(alloc_type_idx),
    );
    // read-file: (i64) -> (i64) - ファイル読み込み
    imports.import("env", "read-file", EntityType::Function(alloc_type_idx));
    // write-file: (i64, i64) -> (i64) - ファイル書き込み
    imports.import(
        "env",
        "write-file",
        EntityType::Function(string_concat_type_idx),
    );
    // file-exists?: (i64) -> (i64) - ファイル存在確認
    imports.import("env", "file-exists?", EntityType::Function(alloc_type_idx));
    // command-line-args: () -> (i64) - コマンドライン引数数
    imports.import(
        "env",
        "command-line-args",
        EntityType::Function(command_line_args_type_idx),
    );
    // command-line-arg: (i64) -> (i64) - 指定 index のコマンドライン引数
    imports.import(
        "env",
        "command-line-arg",
        EntityType::Function(command_line_arg_type_idx),
    );
    // read-stdin: () -> (i64) - stdin 全体を返す
    imports.import(
        "env",
        "read-stdin",
        EntityType::Function(read_stdin_type_idx),
    );
    // __fnv1a_hash: (i64) -> (i64) - FNV-1a ハッシュ
    imports.import("env", "__fnv1a_hash", EntityType::Function(alloc_type_idx));
    // root_push: (i64) -> (i64) - root stack へ push して slot を返す
    imports.import("env", "root_push", EntityType::Function(alloc_type_idx));
    // root_pop: () -> (i64) - root stack の末尾値を返しながら pop する
    imports.import("env", "root_pop", EntityType::Function(read_stdin_type_idx));
    // root_set: (i64, i64) -> (i64) - 既存 root slot を更新する
    imports.import(
        "env",
        "root_set",
        EntityType::Function(string_concat_type_idx),
    );
    wasm_module.section(&imports);

    // === Function Section ===
    let mut functions = FunctionSection::new();
    for &type_idx in &func_type_indices {
        functions.function(type_idx);
    }
    wasm_module.section(&functions);

    // === Memory Section ===
    let mut memories = MemorySection::new();
    memories.memory(MemoryType {
        minimum: 1,
        maximum: None,
        memory64: false,
        shared: false,
        page_size_log2: None,
    });
    wasm_module.section(&memories);

    // === Export Section ===
    let mut exports = ExportSection::new();
    exports.export("memory", ExportKind::Memory, 0);

    // main 関数とその他の export
    let import_count: u32 = 17; // print + __alloc + __string_concat + __string_eq + print-string + proc-exit + __int_to_string + read-file + write-file + file-exists? + command-line-args + command-line-arg + read-stdin + __fnv1a_hash + root_push + root_pop + root_set
    for (i, func) in module.functions.iter().enumerate() {
        if func.is_export {
            exports.export(&func.name, ExportKind::Func, import_count + i as u32);
        }
    }
    // 全関数を export（デバッグ用）
    for (i, func) in module.functions.iter().enumerate() {
        if !func.is_export {
            exports.export(&func.name, ExportKind::Func, import_count + i as u32);
        }
    }
    wasm_module.section(&exports);

    // === Code Section ===
    let mut codes = CodeSection::new();
    for func in &module.functions {
        let mut f = wasm_encoder::Function::new(
            func.locals
                .iter()
                .map(|t| (1, crate::emit::ir_to_wasm_valtype(*t)))
                .collect::<Vec<_>>(),
        );

        // 命令を変換
        emit_instructions(&mut f, &func.body)?;

        // 暗黙の end
        f.instruction(&wasm_encoder::Instruction::End);

        codes.function(&f);
    }
    wasm_module.section(&codes);

    Ok(wasm_module.finish())
}

/// IR 命令列を Wasm 命令に変換
fn emit_instructions(
    func: &mut wasm_encoder::Function,
    instructions: &[Instruction],
) -> Result<(), CodegenError> {
    use wasm_encoder::Instruction as W;
    crate::emit::emit_instructions_common(func, instructions, |f, i| {
        f.instruction(&W::Call(i));
        Ok(())
    })
}

/// IR 型 -> Wasm 型
#[cfg(test)]
#[path = "codegen_tests.rs"]
mod tests;
