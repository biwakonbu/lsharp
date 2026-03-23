//! IR -> Wasm バイナリ生成

use lsharp_ir::{Instruction, Module};
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
    types.ty().function(vec![ValType::I64, ValType::I64], vec![ValType::I64]);

    // Type 3: __string_eq 関数の型 (i64, i64) -> (i64)
    let string_eq_type_idx = types.len();
    types.ty().function(vec![ValType::I64, ValType::I64], vec![ValType::I64]);

    // Type 4: print-string 関数の型 (i64) -> ()
    let print_string_type_idx = types.len();
    types.ty().function(vec![ValType::I64], vec![]);

    // Type 5: proc-exit 関数の型 (i32) -> ()
    let proc_exit_type_idx = types.len();
    types.ty().function(vec![ValType::I32], vec![]);

    // ユーザー定義関数の型を追加
    let mut func_type_indices: Vec<u32> = Vec::new();
    for func in &module.functions {
        let type_idx = types.len();
        let params: Vec<ValType> = func.params.iter().map(|t| crate::emit::ir_to_wasm_valtype(*t)).collect();
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
    imports.import("env", "__string_concat", EntityType::Function(string_concat_type_idx));
    // __string_eq: (i64, i64) -> (i64) - 文字列比較
    imports.import("env", "__string_eq", EntityType::Function(string_eq_type_idx));
    // print-string: (i64) -> () - 文字列出力
    imports.import("env", "print-string", EntityType::Function(print_string_type_idx));
    // proc-exit: (i32) -> () - プロセス終了
    imports.import("env", "proc-exit", EntityType::Function(proc_exit_type_idx));
    // __int_to_string: (i64) -> (i64) - 整数→文字列変換
    imports.import("env", "__int_to_string", EntityType::Function(alloc_type_idx));
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
    let import_count: u32 = 7; // print + __alloc + __string_concat + __string_eq + print-string + proc-exit + __int_to_string
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
mod tests {
    use super::*;
    use lsharp_ir::lower::Lower;
    use lsharp_types::infer::Infer;

    fn compile(source: &str) -> Vec<u8> {
        let program = lsharp_syntax::parse(source).unwrap();
        let mut infer = Infer::new();
        let type_results = infer.infer_program(&program).unwrap();
        let mut lower = Lower::new();
        let module = lower.lower_program(&program, &type_results).unwrap();
        emit_wasm(&module).unwrap()
    }

    fn run_main(wasm_bytes: &[u8]) -> i64 {
        use wasmtime::*;

        let engine = Engine::default();
        let module = wasmtime::Module::new(&engine, wasm_bytes).unwrap();
        let mut store = Store::new(&engine, ());

        // print 関数のスタブ
        let print_ty = FuncType::new(&engine, [ValType::I64], []);
        let print_func = Func::new(&mut store, print_ty, |_caller, params, _results| {
            if let Some(Val::I64(n)) = params.first() {
                println!("{n}");
            }
            Ok(())
        });

        // __alloc 関数のスタブ（テスト用ダミー）
        let alloc_ty = FuncType::new(&engine, [ValType::I64], [ValType::I64]);
        let alloc_func = Func::new(&mut store, alloc_ty, |_caller, _params, results| {
            results[0] = Val::I64(1024); // ダミーアドレス
            Ok(())
        });

        // __string_concat 関数のスタブ
        let string_concat_ty = FuncType::new(&engine, [ValType::I64, ValType::I64], [ValType::I64]);
        let string_concat_func = Func::new(&mut store, string_concat_ty, |_caller, _params, results| {
            results[0] = Val::I64(0); // ダミー
            Ok(())
        });

        // __string_eq 関数のスタブ
        let string_eq_ty = FuncType::new(&engine, [ValType::I64, ValType::I64], [ValType::I64]);
        let string_eq_func = Func::new(&mut store, string_eq_ty, |_caller, _params, results| {
            results[0] = Val::I64(0); // ダミー
            Ok(())
        });

        // print-string 関数のスタブ
        let print_string_ty = FuncType::new(&engine, [ValType::I64], []);
        let print_string_func = Func::new(&mut store, print_string_ty, |_caller, _params, _results| {
            Ok(())
        });

        // proc-exit 関数のスタブ
        let proc_exit_ty = FuncType::new(&engine, [ValType::I32], []);
        let proc_exit_func = Func::new(&mut store, proc_exit_ty, |_caller, _params, _results| {
            Ok(())
        });

        // __int_to_string 関数のスタブ
        let int_to_string_ty = FuncType::new(&engine, [ValType::I64], [ValType::I64]);
        let int_to_string_func = Func::new(&mut store, int_to_string_ty, |_caller, _params, results| {
            results[0] = Val::I64(0); // ダミー
            Ok(())
        });

        let instance = Instance::new(
            &mut store,
            &module,
            &[print_func.into(), alloc_func.into(), string_concat_func.into(), string_eq_func.into(), print_string_func.into(), proc_exit_func.into(), int_to_string_func.into()],
        ).unwrap();

        let main = instance
            .get_typed_func::<(), i64>(&mut store, "main")
            .unwrap();

        main.call(&mut store, ()).unwrap()
    }

    #[test]
    fn test_compile_fib() {
        let wasm = compile(
            "(defn fib [n]
               (if (<= n 1)
                 n
                 (+ (fib (- n 1)) (fib (- n 2)))))
             (defn main [] (fib 10))",
        );
        let result = run_main(&wasm);
        assert_eq!(result, 55);
    }

    #[test]
    fn test_compile_arithmetic() {
        let wasm = compile("(defn main [] (+ (* 3 4) 5))");
        let result = run_main(&wasm);
        assert_eq!(result, 17);
    }

    #[test]
    fn test_compile_if() {
        let wasm = compile("(defn main [] (if (< 1 2) 42 0))");
        let result = run_main(&wasm);
        assert_eq!(result, 42);
    }

    #[test]
    fn test_compile_let() {
        let wasm = compile("(defn main [] (let [x 10 y 20] (+ x y)))");
        let result = run_main(&wasm);
        assert_eq!(result, 30);
    }

    #[test]
    fn test_compile_recursive_factorial() {
        let wasm = compile(
            "(defn fact [n]
               (if (<= n 1)
                 1
                 (* n (fact (- n 1)))))
             (defn main [] (fact 10))",
        );
        let result = run_main(&wasm);
        assert_eq!(result, 3628800);
    }

    #[test]
    fn test_compile_nested_let() {
        let wasm = compile(
            "(defn main []
               (let [a 5
                     b (+ a 3)]
                 (* a b)))",
        );
        let result = run_main(&wasm);
        assert_eq!(result, 40);
    }

    #[test]
    fn test_compile_multi_function() {
        let wasm = compile(
            "(defn double [x] (* x 2))
             (defn main [] (double 21))",
        );
        let result = run_main(&wasm);
        assert_eq!(result, 42);
    }
}
