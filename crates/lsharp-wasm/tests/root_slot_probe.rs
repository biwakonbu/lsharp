//! selfhost 型推論と canonical `:case` preflight の root slot 回帰。
//!
//! 外側の root slot を保持した Wasm harness で、nested allocation 後も
//! `root_top` が caller-owned slot を消費しないことを確認する。

use std::path::PathBuf;

use lsharp_ir::lower::Lower;
use lsharp_syntax::parse;
use lsharp_types::infer::Infer;
use wasmtime::{Config, Engine, Linker, Module, Store};
use wasmtime_wasi::{preview1::WasiP1Ctx, WasiCtxBuilder};

const MODULES: &[&str] = &[
    "Token.ls",
    "AST.ls",
    "Lexer.ls",
    "LexerCompat.ls",
    "Parser.ls",
    "Type.ls",
    "TypeScheme.ls",
    "TypeInferCore.ls",
    "TypeInferFunctions.ls",
    "TypeInferBuiltins.ls",
    "TypeInfer.ls",
    "TypeInferApply.ls",
    "TypeInferBlock.ls",
    "TypeInferPattern.ls",
    "TypeInferRecord.ls",
    "TypeInferRecordDecl.ls",
    "TypeInferAdt.ls",
    "TypeInferAssertions.ls",
    "PropertyRunner.ls",
    "TestRunner.ls",
];

fn run_on_large_stack<T, F>(work: F) -> T
where
    T: Send + 'static,
    F: FnOnce() -> T + Send + 'static,
{
    std::thread::Builder::new()
        .name("root-slot-wasm-probe".into())
        .stack_size(128 * 1024 * 1024)
        .spawn(work)
        .expect("root-slot probe thread を起動できるべき")
        .join()
        .expect("root-slot probe thread が panic しないべき")
}

fn module_path(root: &std::path::Path, name: &str) -> PathBuf {
    match name {
        "Token.ls" | "AST.ls" | "Lexer.ls" | "LexerCompat.ls" | "Parser.ls" => {
            root.join("Syntax").join(name)
        }
        name if name.starts_with("Type") => root.join("Types").join(name),
        "PropertyRunner.ls" | "TestRunner.ls" => root.join("Tools/Test").join(name),
        _ => panic!("unknown selfhost module: {name}"),
    }
}

fn compile_bundle(harness: &str) -> Vec<u8> {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../selfhost/src");
    let bundle = MODULES
        .iter()
        .map(|name| {
            std::fs::read_to_string(module_path(&root, name))
                .expect("selfhost module source should be readable")
        })
        .collect::<Vec<_>>()
        .join("\n")
        .replace("(import Types.TypeInfer)\n", "");
    let source = format!("{bundle}\n{harness}");
    let program = parse(&source).expect("root-slot probe source should parse");
    let mut infer = Infer::new();
    let type_results = infer
        .infer_program(&program)
        .expect("root-slot probe source should typecheck");
    let mut lower = Lower::new();
    let module = lower
        .lower_program(&program, &type_results)
        .expect("root-slot probe source should lower");
    lsharp_wasm::wasi::emit_wasm_wasi(&module).expect("root-slot probe should emit Wasm")
}

fn run_with_root_telemetry(wasm: &[u8]) -> (Result<(), String>, i32, String) {
    let mut config = Config::new();
    config.max_wasm_stack(64 * 1024 * 1024);
    let engine = Engine::new(&config).expect("Wasmtime engine should build");
    let mut linker = Linker::<WasiP1Ctx>::new(&engine);
    wasmtime_wasi::preview1::add_to_linker_sync(&mut linker, |ctx| ctx)
        .expect("WASI linker should build");
    let stdout = wasmtime_wasi::pipe::MemoryOutputPipe::new(1024 * 1024);
    let mut builder = WasiCtxBuilder::new();
    builder.stdout(stdout.clone());
    builder.args(&["root-slot-probe"]);
    let mut store = Store::new(&engine, builder.build_p1());
    let module = Module::new(&engine, wasm).expect("probe Wasm should instantiate");
    let instance = linker
        .instantiate(&mut store, &module)
        .expect("probe instance should instantiate");
    let start = instance
        .get_typed_func::<(), ()>(&mut store, "_start")
        .expect("_start should be exported");
    let result = start
        .call(&mut store, ())
        .map_err(|error| format!("{error:#}"));
    let top = instance
        .get_global(&mut store, "__lsharp_root_stack_top")
        .expect("root stack top should be exported")
        .get(&mut store)
        .i32()
        .expect("root stack top should be i32");
    drop(store);
    let output = stdout
        .try_into_inner()
        .expect("stdout pipe should be uniquely owned");
    (result, top, String::from_utf8_lossy(&output).into_owned())
}

#[test]
fn test_generate_tests_preserves_outer_root_slots() {
    let harness = r#"
(defn main []
  (let [src "(defn succ [x] :case [(expect 1 3)] (+ x 1))"
        source-slot (root_push src)
        program (parse-program src)
        program-slot (root_push program)
        suite (generate-tests-from-source src)]
    (do
      (print (vector-length (vector-get suite 3)))
      0)))
"#;
    run_on_large_stack(move || {
        let wasm = compile_bundle(harness);
        let (result, root_top, output) = run_with_root_telemetry(&wasm);
        assert!(
            result.is_ok(),
            "generate-tests は外側 root slot を保持したまま完了するべき: result={result:?}, root_top={root_top}, stdout={output:?}"
        );
        assert_eq!(output.trim(), "1");
        assert_eq!(
            root_top, 2,
            "outer root slot は generate-tests 後も残るべき"
        );
    });
}

#[test]
fn test_analysis_and_case_check_preserve_outer_root_slots() {
    let harness = r#"
(defn main []
  (let [src "(defn succ [x] :case [(expect (succ 1) 3)] (+ x 1))"
        source-slot (root_push src)
        program (parse-program src)
        program-slot (root_push program)
        analysis (infer-program-analysis program)
        analysis-slot (root_push analysis)
        case-check (check-canonical-cases-with-analysis program analysis)
        case-check-slot (root_push case-check)]
    (do
      (print-string "AFTER-CHECK\n")
      (print (vector-get case-check 0))
      0)))
"#;
    run_on_large_stack(move || {
        let wasm = compile_bundle(harness);
        let (result, root_top, output) = run_with_root_telemetry(&wasm);
        assert!(
            result.is_ok(),
            "analysis/case check は外側 root slot を保持したまま完了するべき: result={result:?}, root_top={root_top}, stdout={output:?}"
        );
        assert_eq!(output.trim().lines().collect::<Vec<_>>().len(), 2);
        assert_eq!(
            root_top, 4,
            "analysis/case check 後も外側 root slot は残るべき"
        );
    });
}
