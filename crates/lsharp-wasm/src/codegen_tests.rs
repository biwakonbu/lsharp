use super::*;
use lsharp_ir::lower::Lower;
use lsharp_types::infer::Infer;

#[test]
fn codegen_errors_expose_stable_code_without_source_span() {
    let error = CodegenError::Error {
        msg: "invalid instruction".to_string(),
    };

    assert_eq!(error.code(), "LS4001");
    assert_eq!(error.span(), None);
}

fn compile(source: &str) -> Vec<u8> {
    let program = lsharp_syntax::parse(source).unwrap();
    let mut infer = Infer::new();
    let type_results = infer.infer_program(&program).unwrap();
    let expr_type_results = infer.expr_type_results_snapshot();
    let mut lower = Lower::new();
    let module = lower
        .lower_program_with_expr_types(&program, &type_results, &expr_type_results)
        .unwrap();
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
    let string_concat_func =
        Func::new(&mut store, string_concat_ty, |_caller, _params, results| {
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
    let proc_exit_func = Func::new(
        &mut store,
        proc_exit_ty,
        |_caller, _params, _results| Ok(()),
    );

    // __int_to_string 関数のスタブ
    let int_to_string_ty = FuncType::new(&engine, [ValType::I64], [ValType::I64]);
    let int_to_string_func =
        Func::new(&mut store, int_to_string_ty, |_caller, _params, results| {
            results[0] = Val::I64(0); // ダミー
            Ok(())
        });

    // read-file 関数のスタブ
    let read_file_ty = FuncType::new(&engine, [ValType::I64], [ValType::I64]);
    let read_file_func = Func::new(&mut store, read_file_ty, |_caller, _params, results| {
        results[0] = Val::I64(0); // ダミー
        Ok(())
    });

    // write-file 関数のスタブ
    let write_file_ty = FuncType::new(&engine, [ValType::I64, ValType::I64], [ValType::I64]);
    let write_file_func = Func::new(&mut store, write_file_ty, |_caller, _params, results| {
        results[0] = Val::I64(0); // ダミー
        Ok(())
    });

    // file-exists? 関数のスタブ
    let file_exists_ty = FuncType::new(&engine, [ValType::I64], [ValType::I64]);
    let file_exists_func = Func::new(&mut store, file_exists_ty, |_caller, _params, results| {
        results[0] = Val::I64(0); // ダミー
        Ok(())
    });

    // command-line-args 関数のスタブ
    let command_line_args_ty = FuncType::new(&engine, [], [ValType::I64]);
    let command_line_args_func = Func::new(
        &mut store,
        command_line_args_ty,
        |_caller, _params, results| {
            results[0] = Val::I64(0);
            Ok(())
        },
    );

    // command-line-arg 関数のスタブ
    let command_line_arg_ty = FuncType::new(&engine, [ValType::I64], [ValType::I64]);
    let command_line_arg_func = Func::new(
        &mut store,
        command_line_arg_ty,
        |_caller, _params, results| {
            results[0] = Val::I64(0);
            Ok(())
        },
    );

    // read-stdin 関数のスタブ
    let read_stdin_ty = FuncType::new(&engine, [], [ValType::I64]);
    let read_stdin_func = Func::new(&mut store, read_stdin_ty, |_caller, _params, results| {
        results[0] = Val::I64(0);
        Ok(())
    });

    // __fnv1a_hash 関数のスタブ
    let fnv1a_hash_ty = FuncType::new(&engine, [ValType::I64], [ValType::I64]);
    let fnv1a_hash_func = Func::new(&mut store, fnv1a_hash_ty, |_caller, _params, results| {
        results[0] = Val::I64(0); // ダミー
        Ok(())
    });

    let root_stack = std::sync::Arc::new(std::sync::Mutex::new(Vec::<i64>::new()));

    let root_push_ty = FuncType::new(&engine, [ValType::I64], [ValType::I64]);
    let root_push_stack = root_stack.clone();
    let root_push_func = Func::new(&mut store, root_push_ty, move |_caller, params, results| {
        let value = params[0].i64().unwrap_or(0);
        let mut stack = root_push_stack.lock().unwrap();
        let slot = stack.len() as i64;
        stack.push(value);
        results[0] = Val::I64(slot);
        Ok(())
    });

    let root_pop_ty = FuncType::new(&engine, [], [ValType::I64]);
    let root_pop_stack = root_stack.clone();
    let root_pop_func = Func::new(&mut store, root_pop_ty, move |_caller, _params, results| {
        let mut stack = root_pop_stack.lock().unwrap();
        results[0] = Val::I64(stack.pop().unwrap_or(0));
        Ok(())
    });

    let root_set_ty = FuncType::new(&engine, [ValType::I64, ValType::I64], [ValType::I64]);
    let root_set_stack = root_stack.clone();
    let root_set_func = Func::new(&mut store, root_set_ty, move |_caller, params, results| {
        let slot = params[0].i64().unwrap_or(0);
        let value = params[1].i64().unwrap_or(0);
        let mut stack = root_set_stack.lock().unwrap();
        if let Some(entry) = stack.get_mut(slot.max(0) as usize) {
            *entry = value;
            results[0] = Val::I64(slot.max(0));
        } else {
            results[0] = Val::I64(0);
        }
        Ok(())
    });

    let instance = Instance::new(
        &mut store,
        &module,
        &[
            print_func.into(),
            alloc_func.into(),
            string_concat_func.into(),
            string_eq_func.into(),
            print_string_func.into(),
            proc_exit_func.into(),
            int_to_string_func.into(),
            read_file_func.into(),
            write_file_func.into(),
            file_exists_func.into(),
            command_line_args_func.into(),
            command_line_arg_func.into(),
            read_stdin_func.into(),
            fnv1a_hash_func.into(),
            root_push_func.into(),
            root_pop_func.into(),
            root_set_func.into(),
        ],
    )
    .unwrap();

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
