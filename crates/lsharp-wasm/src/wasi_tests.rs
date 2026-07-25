#[cfg(test)]
mod tests {
    use super::*;
    use lsharp_ir::lower::Lower;
    use lsharp_types::infer::Infer;

    fn compile_wasi(source: &str) -> Vec<u8> {
        let program = lsharp_syntax::parse(source).unwrap();
        let mut infer = Infer::new();
        let type_results = infer.infer_program(&program).unwrap();
        let mut lower = Lower::new();
        let module = lower.lower_program(&program, &type_results).unwrap();
        emit_wasm_wasi(&module).unwrap()
    }

    fn compile_wasi_p2(source: &str) -> Vec<u8> {
        let program = lsharp_syntax::parse(source).unwrap();
        let mut infer = Infer::new();
        let type_results = infer.infer_program(&program).unwrap();
        let mut lower = Lower::new();
        let module = lower.lower_program(&program, &type_results).unwrap();
        emit_wasm_wasi_p2(&module).unwrap()
    }

    fn run_wasi(wasm_bytes: &[u8]) -> String {
        use wasmtime::*;
        use wasmtime_wasi::{WasiCtxBuilder, preview1::WasiP1Ctx};

        let engine = Engine::default();
        let mut linker = Linker::<WasiP1Ctx>::new(&engine);
        wasmtime_wasi::preview1::add_to_linker_sync(&mut linker, |t| t).unwrap();

        let stdout = wasmtime_wasi::pipe::MemoryOutputPipe::new(1024);
        let wasi = WasiCtxBuilder::new().stdout(stdout.clone()).build_p1();

        let mut store = Store::new(&engine, wasi);
        let module = wasmtime::Module::new(&engine, wasm_bytes).unwrap();
        let instance = linker.instantiate(&mut store, &module).unwrap();

        let start = instance
            .get_typed_func::<(), ()>(&mut store, "_start")
            .unwrap();
        start.call(&mut store, ()).unwrap();

        drop(store);
        let bytes = stdout.try_into_inner().unwrap();
        String::from_utf8(bytes.to_vec()).unwrap()
    }

    fn run_wasi_with_root_slot_failure_ledger(wasm_bytes: &[u8]) -> (String, i32, i32, i32) {
        use wasmtime::*;
        use wasmtime_wasi::{WasiCtxBuilder, preview1::WasiP1Ctx};

        let engine = Engine::default();
        let mut linker = Linker::<WasiP1Ctx>::new(&engine);
        wasmtime_wasi::preview1::add_to_linker_sync(&mut linker, |t| t).unwrap();
        let stdout = wasmtime_wasi::pipe::MemoryOutputPipe::new(1024);
        let wasi = WasiCtxBuilder::new().stdout(stdout).build_p1();
        let mut store = Store::new(&engine, wasi);
        let module = wasmtime::Module::new(&engine, wasm_bytes).unwrap();
        let instance = linker.instantiate(&mut store, &module).unwrap();
        let start = instance
            .get_typed_func::<(), ()>(&mut store, "_start")
            .unwrap();
        let error = start.call(&mut store, ()).unwrap_err();
        let failure_slot = instance
            .get_global(&mut store, "__lsharp_root_slot_failure_slot")
            .unwrap()
            .get(&mut store)
            .i32()
            .unwrap();
        let failure_top = instance
            .get_global(&mut store, "__lsharp_root_slot_failure_top")
            .unwrap()
            .get(&mut store)
            .i32()
            .unwrap();
        let failure_count = instance
            .get_global(&mut store, "__lsharp_root_slot_failure_count")
            .unwrap()
            .get(&mut store)
            .i32()
            .unwrap();
        (
            format!("{error:#}"),
            failure_slot,
            failure_top,
            failure_count,
        )
    }

    #[test]
    fn test_root_set_invalid_slot_records_failure_ledger_before_trap() {
        let wasm = compile_wasi("(defn main [] (root_set 0 42))");

        let (error, failure_slot, failure_top, failure_count) =
            run_wasi_with_root_slot_failure_ledger(&wasm);

        assert!(
            error.contains("<wasm function 24>"),
            "root_set trap: {error}"
        );
        assert_eq!(failure_slot, 0);
        assert_eq!(failure_top, 0);
        assert_eq!(failure_count, 1);
    }

    fn assert_close_errno_is_saved(wasm_bytes: &[u8], code_ordinal: usize) {
        use wasmparser::{Operator, Parser, Payload};

        let mut ordinal = 0usize;
        let mut found_saved_errno = false;
        let mut found_dropped_errno = false;
        for payload in Parser::new(0).parse_all(wasm_bytes) {
            let payload = payload.expect("Wasm payload の読み取りに失敗");
            let Payload::CodeSectionEntry(body) = payload else {
                continue;
            };
            if ordinal != code_ordinal {
                ordinal += 1;
                continue;
            }

            let mut close_call = false;
            for operator in body
                .get_operators_reader()
                .expect("helper body の operator reader 作成に失敗")
            {
                match operator.expect("helper body の operator 読み取りに失敗") {
                    Operator::Call { function_index: 5 } => close_call = true,
                    Operator::LocalSet { .. } if close_call => {
                        found_saved_errno = true;
                        close_call = false;
                    }
                    Operator::Drop if close_call => {
                        found_dropped_errno = true;
                        close_call = false;
                    }
                    _ => close_call = false,
                }
            }
            break;
        }

        assert!(
            found_saved_errno,
            "fd_close errno を local へ保存する必要がある"
        );
        assert!(
            !found_dropped_errno,
            "fd_close errno を drop してはいけない"
        );
    }

    fn assert_fd_read_errno_is_saved(wasm_bytes: &[u8]) {
        use wasmparser::{Operator, Parser, Payload};

        let mut found_saved_errno = false;
        for payload in Parser::new(0).parse_all(wasm_bytes) {
            let payload = payload.expect("Wasm payload の読み取りに失敗");
            let Payload::CodeSectionEntry(body) = payload else {
                continue;
            };

            let mut read_call = false;
            for operator in body
                .get_operators_reader()
                .expect("helper body の operator reader 作成に失敗")
            {
                match operator.expect("helper body の operator 読み取りに失敗") {
                    Operator::Call { function_index: 4 } => read_call = true,
                    Operator::LocalSet { .. } if read_call => {
                        found_saved_errno = true;
                        read_call = false;
                    }
                    _ => {}
                }
            }
        }

        assert!(
            found_saved_errno,
            "fd_read errno を local へ保存する必要がある"
        );
    }

    fn assert_call_result_is_saved(
        wasm_bytes: &[u8],
        code_ordinal: usize,
        function_index: u32,
        result_name: &str,
    ) {
        use wasmparser::{Operator, Parser, Payload};

        let mut ordinal = 0usize;
        let mut found_saved_result = false;
        let mut found_dropped_result = false;
        for payload in Parser::new(0).parse_all(wasm_bytes) {
            let payload = payload.expect("Wasm payload の読み取りに失敗");
            let Payload::CodeSectionEntry(body) = payload else {
                continue;
            };
            if ordinal != code_ordinal {
                ordinal += 1;
                continue;
            }

            let mut call_result_pending = false;
            for operator in body
                .get_operators_reader()
                .expect("helper body の operator reader 作成に失敗")
            {
                match operator.expect("helper body の operator 読み取りに失敗") {
                    Operator::Call {
                        function_index: current_index,
                    } if current_index == function_index => call_result_pending = true,
                    Operator::LocalSet { .. } if call_result_pending => {
                        found_saved_result = true;
                        call_result_pending = false;
                    }
                    Operator::Drop if call_result_pending => {
                        found_dropped_result = true;
                        call_result_pending = false;
                    }
                    _ => call_result_pending = false,
                }
            }
            break;
        }

        assert!(
            found_saved_result,
            "{result_name} の errno を local へ保存する必要がある"
        );
        assert!(
            !found_dropped_result,
            "{result_name} の errno を drop してはいけない"
        );
    }

    #[test]
    fn test_wasi_write_helpers_preserve_fd_close_errno() {
        let wasm = compile_wasi(
            r#"
            (defn main []
              (do
                (write-file "output.txt" "payload")
                (let [bytes (vector-push (vector-new 1) 97)]
                  (write-file-bytes "raw.bin" bytes))
                0))
            "#,
        );
        assert_close_errno_is_saved(&wasm, 7);
        assert_close_errno_is_saved(&wasm, 16);
    }

    #[test]
    fn test_wasi_read_file_preserves_fd_read_errno() {
        let wasm = compile_wasi(r#"(defn main [] (print-string (read-file "input.txt")))"#);
        assert_fd_read_errno_is_saved(&wasm);
    }

    #[test]
    fn test_wasi_read_file_preserves_fd_close_errno() {
        let wasm = compile_wasi(r#"(defn main [] (print-string (read-file "input.txt")))"#);
        assert_close_errno_is_saved(&wasm, 6);
    }

    #[test]
    fn test_wasi_file_helpers_preserve_path_open_errno() {
        let wasm = compile_wasi(
            r#"
            (defn main []
              (do
                (read-file "input.txt")
                (write-file "output.txt" "payload")
                (let [bytes (vector-push (vector-new 1) 97)]
                  (write-file-bytes "raw.bin" bytes))
                0))
            "#,
        );
        assert_call_result_is_saved(&wasm, 6, 6, "read-file path_open");
        assert_call_result_is_saved(&wasm, 6, 8, "read-file fd_filestat_get");
        assert_call_result_is_saved(&wasm, 7, 6, "write-file path_open");
        assert_call_result_is_saved(&wasm, 16, 6, "write-file-bytes path_open");
    }

    #[test]
    fn test_wasi_file_helpers_fail_closed_on_path_open_errno() {
        let wasm = compile_wasi(
            r#"
            (defn main []
              (let [bytes (vector-push (vector-new 1) 97)]
                (do
                  (print-string (read-file "input.txt"))
                  (print (write-file "output.txt" "payload"))
                  (print (write-file-bytes "raw.bin" bytes))
                  0)))
            "#,
        );
        assert_eq!(run_wasi(&wasm), "-1\n-1\n");
    }

    #[test]
    fn test_wasi_file_exists_preserves_fd_close_errno() {
        let wasm = compile_wasi(r#"(defn main [] (print (file-exists? "input.txt")))"#);
        assert_call_result_is_saved(&wasm, 8, 5, "file-exists fd_close");
    }

    #[test]
    fn test_wasi_print_positive() {
        let wasm = compile_wasi("(defn main [] (print 42))");
        assert_eq!(run_wasi(&wasm), "42\n");
    }

    #[test]
    fn test_wasi_print_zero() {
        let wasm = compile_wasi("(defn main [] (print 0))");
        assert_eq!(run_wasi(&wasm), "0\n");
    }

    #[test]
    fn test_wasi_print_large_number() {
        let wasm = compile_wasi("(defn main [] (print 1234567890))");
        assert_eq!(run_wasi(&wasm), "1234567890\n");
    }

    #[test]
    fn test_wasi_print_one() {
        let wasm = compile_wasi("(defn main [] (print 1))");
        assert_eq!(run_wasi(&wasm), "1\n");
    }

    #[test]
    fn test_wasi_print_arithmetic_result() {
        let wasm = compile_wasi("(defn main [] (print (+ (* 3 4) 5)))");
        assert_eq!(run_wasi(&wasm), "17\n");
    }

    #[test]
    fn test_wasi_multiple_prints() {
        let wasm = compile_wasi("(defn main [] (do (print 1) (print 2) (print 3) 0))");
        assert_eq!(run_wasi(&wasm), "1\n2\n3\n");
    }

    #[test]
    fn test_wasi_print_function_result() {
        let wasm = compile_wasi(
            "(defn double [x] (* x 2))
             (defn main [] (print (double 21)))",
        );
        assert_eq!(run_wasi(&wasm), "42\n");
    }

    #[test]
    fn test_wasi_write_file_bytes_writes_vector_low_bytes() {
        let wasm = compile_wasi(
            r#"
            (defn main []
              (let [bytes (vector-push
                            (vector-push
                              (vector-push
                                (vector-push (vector-new 4) 0)
                                97)
                              115)
                            109)]
                (write-file-bytes "raw.wasm" bytes)))
            "#,
        );
        let dir = std::env::temp_dir().join(format!(
            "lsharp_wasi_write_file_bytes_{}",
            std::process::id()
        ));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).expect("fixture directory の作成に失敗");

        let result = (|| {
            let output = crate::wasi_runner::run_wasm_wasi_with_dir(&wasm, Some(&dir))
                .expect("write-file-bytes program の実行に失敗");
            assert_eq!(output, "");
            assert_eq!(
                std::fs::read(dir.join("raw.wasm")).expect("raw.wasm の読み込みに失敗"),
                b"\0asm"
            );
        })();
        let _ = std::fs::remove_dir_all(&dir);
        result
    }

    #[test]
    fn test_wasi_print_fib() {
        let wasm = compile_wasi(
            "(defn fib [n]
               (if (<= n 1) n (+ (fib (- n 1)) (fib (- n 2)))))
             (defn main [] (print (fib 10)))",
        );
        assert_eq!(run_wasi(&wasm), "55\n");
    }

    #[test]
    fn test_wasi_record_access_uses_linear_memory_fallback() {
        let wasm = compile_wasi(
            "(type Point (record (: x Int) (: y Int)))
             (defn make-point [x y] {Point x x y y})
             (defn main [] (print (Point.x (make-point 10 20))))",
        );
        assert_eq!(run_wasi(&wasm), "10\n");
    }

    #[test]
    fn test_emit_wasm_wasi_p2_basic_program_compiles() {
        let component = compile_wasi_p2("(defn main [] (print 42))");
        assert!(component.len() > 8);
        assert_eq!(&component[0..4], b"\0asm");

        let engine = wasmtime::Engine::default();
        wasmtime::component::Component::new(&engine, &component)
            .expect("P2 entrypoint は valid component を生成するべき");
    }

    #[test]
    fn test_emit_wasm_wasi_p2_runs_print_via_component_runner() {
        let component = compile_wasi_p2("(defn main [] (print 42))");

        let output = crate::wasi_runner::run_wasm_component(&component)
            .expect("P2 component は preview2 runner で実行できるべき");
        assert_eq!(output, "42\n");
    }

    #[test]
    fn test_emit_wasm_wasi_p2_supports_stdin_and_args() {
        let component = compile_wasi_p2(
            r#"
            (defn main []
              (do
                (print-string (command-line-arg 0))
                (print-string ":")
                (print-string (read-stdin))
                0))
            "#,
        );

        let output = crate::wasi_runner::run_wasm_component_with_args_and_stdin(
            &component,
            &["alpha"],
            "stdin-smoke",
        )
        .expect("P2 component は argv/stdin bridge を使えるべき");
        assert_eq!(output, "alpha:stdin-smoke");
    }

    #[test]
    fn test_emit_wasm_wasi_p2_supports_large_stdout_write() {
        let payload = "x".repeat(4097);
        let component = compile_wasi_p2(&format!(
            r#"
            (defn main []
              (do
                (print-string "{payload}")
                0))
            "#
        ));

        let output = crate::wasi_runner::run_wasm_component(&component)
            .expect("P2 component は 4KiB 超の stdout write を処理できるべき");
        assert_eq!(output, payload);
    }

    #[test]
    fn test_emit_wasm_wasi_p2_supports_file_roundtrip() {
        let component = compile_wasi_p2(
            r#"
            (defn main []
              (do
                (write-file "roundtrip.txt" "hello component")
                (print-string (read-file "roundtrip.txt"))
                0))
            "#,
        );

        let dir = std::env::temp_dir().join("lsharp_wasi_p2_file_roundtrip");
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();

        let output = crate::wasi_runner::run_wasm_component_with_dir_args_and_stdin(
            &component,
            Some(&dir),
            &[],
            "",
        )
        .expect("P2 component は preview2 filesystem bridge 経由で file roundtrip できるべき");
        assert_eq!(output, "hello component");

        std::fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn test_wasi_proc_exit_type_check() {
        // proc-exit が型チェックを通ること (Int -> Unit)
        let source = "(defn main [] (do (proc-exit 0) 0))";
        let program = lsharp_syntax::parse(source).unwrap();
        let mut infer = Infer::new();
        let result = infer.infer_program(&program);
        assert!(
            result.is_ok(),
            "proc-exit の型チェックが失敗: {:?}",
            result.err()
        );
    }

    #[test]
    fn test_wasi_proc_exit_compile() {
        // proc-exit を含むコードがコンパイルでき、wasmtime で検証できること
        let wasm = compile_wasi("(defn main [] (do (proc-exit 0) 0))");
        assert!(wasm.len() > 8);
        assert_eq!(&wasm[0..4], b"\0asm");

        // wasmtime でモジュールを読み込めるか検証
        use wasmtime::Engine;
        let engine = Engine::default();
        wasmtime::Module::new(&engine, &wasm).expect("proc-exit を含むモジュールの読み込みに失敗");
    }

    #[test]
    fn test_wasi_proc_exit_run() {
        // proc-exit(0) を呼ぶと正常終了すること
        // wasmtime では proc_exit(0) は Trap ではなく正常終了として扱われる
        let wasm = compile_wasi("(defn main [] (do (print 42) (proc-exit 0) 0))");

        use wasmtime::*;
        use wasmtime_wasi::{WasiCtxBuilder, preview1::WasiP1Ctx};

        let engine = Engine::default();
        let mut linker = Linker::<WasiP1Ctx>::new(&engine);
        wasmtime_wasi::preview1::add_to_linker_sync(&mut linker, |t| t).unwrap();

        let stdout = wasmtime_wasi::pipe::MemoryOutputPipe::new(1024);
        let wasi = WasiCtxBuilder::new().stdout(stdout.clone()).build_p1();

        let mut store = Store::new(&engine, wasi);
        let module = wasmtime::Module::new(&engine, &wasm).unwrap();
        let instance = linker.instantiate(&mut store, &module).unwrap();

        let start = instance
            .get_typed_func::<(), ()>(&mut store, "_start")
            .unwrap();
        // proc_exit(0) は I32Exit(0) をトラップするが、exit code 0 は成功
        let result = start.call(&mut store, ());
        match result {
            Ok(()) => {} // 正常終了
            Err(e) => {
                // wasmtime は proc_exit を I32Exit として Trap する
                let exit_status = e.downcast_ref::<wasmtime_wasi::I32Exit>();
                assert!(exit_status.is_some(), "予期しないエラー: {e}");
                assert_eq!(exit_status.unwrap().0, 0, "exit code が 0 でない");
            }
        }

        drop(store);
        let bytes = stdout.try_into_inner().unwrap();
        let output = String::from_utf8(bytes.to_vec()).unwrap();
        assert_eq!(output, "42\n", "proc-exit 前の print 出力が正しくない");
    }

    #[test]
    fn test_wasi_additional_imports_validate() {
        // 新しい WASI import が追加されていても既存のコードが正しく動くことを検証
        let wasm = compile_wasi(
            "(defn fib [n]
               (if (<= n 1) n (+ (fib (- n 1)) (fib (- n 2)))))
             (defn main [] (print (fib 10)))",
        );
        assert_eq!(run_wasi(&wasm), "55\n");
    }

    #[test]
    fn test_wasi_import_section_count() {
        // Import Section に 9 つの WASI 関数が含まれていることを検証
        // (fd_write, proc_exit, args_get, args_sizes_get, fd_read, fd_close, path_open, fd_seek, fd_filestat_get)
        let wasm = compile_wasi("(defn main [] (print 42))");

        // wasmtime でモジュールを読み込んで import 数を検証
        use wasmtime::Engine;
        let engine = Engine::default();
        let module = wasmtime::Module::new(&engine, &wasm).unwrap();
        let imports: Vec<_> = module.imports().collect();
        assert_eq!(
            imports.len(),
            9,
            "WASI import 数が 9 でない: {:?}",
            imports
                .iter()
                .map(|i| i.name().to_string())
                .collect::<Vec<_>>()
        );

        // 各 import 名を検証
        let import_names: Vec<_> = imports.iter().map(|i| i.name().to_string()).collect();
        assert!(import_names.contains(&"fd_write".to_string()));
        assert!(import_names.contains(&"proc_exit".to_string()));
        assert!(import_names.contains(&"args_get".to_string()));
        assert!(import_names.contains(&"args_sizes_get".to_string()));
        assert!(import_names.contains(&"fd_read".to_string()));
        assert!(import_names.contains(&"fd_close".to_string()));
        assert!(import_names.contains(&"path_open".to_string()));
        assert!(import_names.contains(&"fd_seek".to_string()));
        assert!(import_names.contains(&"fd_filestat_get".to_string()));
    }

    #[test]
    fn test_wasi_closure_module_validates() {
        // クロージャを含むモジュールが wasmtime で読み込めることを検証
        let source = r#"
            (defn make-inc [] (fn [x] (+ x 1)))
            (defn apply [f x] (f x))
            (defn main [] (print (apply (make-inc) 41)))
        "#;
        let program = lsharp_syntax::parse(source).unwrap();
        let mut infer = Infer::new();
        let type_results = infer.infer_program(&program).unwrap();
        let mut lower = Lower::new();
        let module = lower.lower_program(&program, &type_results).unwrap();
        eprintln!("IR dump:\n{}", module.dump());
        for (i, f) in module.functions.iter().enumerate() {
            eprintln!(
                "func[{}] = {} ({} params, {} locals)",
                i,
                f.name,
                f.params.len(),
                f.locals.len()
            );
            for (j, instr) in f.body.iter().enumerate() {
                eprintln!("  [{j}] {instr:?}");
            }
        }
        let wasm_bytes = emit_wasm_wasi(&module).unwrap();
        eprintln!("Wasm bytes: {} bytes", wasm_bytes.len());

        // wasmtime でモジュールを読み込めるか
        use wasmtime::Engine;
        let engine = Engine::default();
        match wasmtime::Module::new(&engine, &wasm_bytes) {
            Ok(_) => eprintln!("Module loaded successfully"),
            Err(e) => panic!("Module load error: {e}"),
        }
    }
}
