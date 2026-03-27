//! L# コンパイラパイプラインのベンチマーク
//!
//! 各フェーズ（パース、型推論、IR lowering、Wasm codegen）と
//! フルパイプラインの実行時間を計測する。
//!
//! 実行: `cargo bench -p lsharp-wasm`

use criterion::{Criterion, black_box, criterion_group, criterion_main};
use lsharp_ir::lower::Lower;
use lsharp_types::infer::Infer;

/// ベンチマーク用の簡単な L# プログラム
const SIMPLE_SOURCE: &str = "(defn main [] (+ 1 2))";

/// ベンチマーク用のやや複雑な L# プログラム（フィボナッチ）
const FIBONACCI_SOURCE: &str = r#"
(defn fib [n : Int] : Int
  (if (<= n 1)
    n
    (+ (fib (- n 1)) (fib (- n 2)))))

(defn main []
  (print (fib 10)))
"#;

/// パースのベンチマーク: ソースコードから AST を生成
fn bench_parse(c: &mut Criterion) {
    c.bench_function("parse_simple", |b| {
        b.iter(|| lsharp_syntax::parse(black_box(SIMPLE_SOURCE)).unwrap())
    });

    c.bench_function("parse_fibonacci", |b| {
        b.iter(|| lsharp_syntax::parse(black_box(FIBONACCI_SOURCE)).unwrap())
    });
}

/// 型推論のベンチマーク: AST に対して Hindley-Milner 型推論を実行
fn bench_infer(c: &mut Criterion) {
    let simple_prog = lsharp_syntax::parse(SIMPLE_SOURCE).unwrap();
    let fib_prog = lsharp_syntax::parse(FIBONACCI_SOURCE).unwrap();

    c.bench_function("infer_simple", |b| {
        b.iter(|| {
            let mut infer = Infer::new();
            infer.infer_program(black_box(&simple_prog)).unwrap()
        })
    });

    c.bench_function("infer_fibonacci", |b| {
        b.iter(|| {
            let mut infer = Infer::new();
            infer.infer_program(black_box(&fib_prog)).unwrap()
        })
    });
}

/// IR lowering のベンチマーク: 型チェック済み AST を IR に変換
fn bench_lower(c: &mut Criterion) {
    let simple_prog = lsharp_syntax::parse(SIMPLE_SOURCE).unwrap();
    let mut infer = Infer::new();
    let simple_types = infer.infer_program(&simple_prog).unwrap();

    let fib_prog = lsharp_syntax::parse(FIBONACCI_SOURCE).unwrap();
    let mut infer = Infer::new();
    let fib_types = infer.infer_program(&fib_prog).unwrap();

    c.bench_function("lower_simple", |b| {
        b.iter(|| {
            let mut lower = Lower::new();
            lower
                .lower_program(black_box(&simple_prog), black_box(&simple_types))
                .unwrap()
        })
    });

    c.bench_function("lower_fibonacci", |b| {
        b.iter(|| {
            let mut lower = Lower::new();
            lower
                .lower_program(black_box(&fib_prog), black_box(&fib_types))
                .unwrap()
        })
    });
}

/// Wasm コード生成のベンチマーク: IR から Wasm バイナリを生成
fn bench_codegen(c: &mut Criterion) {
    let simple_prog = lsharp_syntax::parse(SIMPLE_SOURCE).unwrap();
    let mut infer = Infer::new();
    let simple_types = infer.infer_program(&simple_prog).unwrap();
    let mut lower = Lower::new();
    let simple_module = lower.lower_program(&simple_prog, &simple_types).unwrap();

    let fib_prog = lsharp_syntax::parse(FIBONACCI_SOURCE).unwrap();
    let mut infer = Infer::new();
    let fib_types = infer.infer_program(&fib_prog).unwrap();
    let mut lower = Lower::new();
    let fib_module = lower.lower_program(&fib_prog, &fib_types).unwrap();

    c.bench_function("codegen_simple", |b| {
        b.iter(|| lsharp_wasm::wasi::emit_wasm_wasi(black_box(&simple_module)).unwrap())
    });

    c.bench_function("codegen_fibonacci", |b| {
        b.iter(|| lsharp_wasm::wasi::emit_wasm_wasi(black_box(&fib_module)).unwrap())
    });
}

/// フルパイプラインのベンチマーク: パース → 型推論 → IR → Wasm
fn bench_full_pipeline(c: &mut Criterion) {
    c.bench_function("full_pipeline_simple", |b| {
        b.iter(|| {
            let prog = lsharp_syntax::parse(black_box(SIMPLE_SOURCE)).unwrap();
            let mut infer = Infer::new();
            let types = infer.infer_program(&prog).unwrap();
            let mut lower = Lower::new();
            let module = lower.lower_program(&prog, &types).unwrap();
            lsharp_wasm::wasi::emit_wasm_wasi(&module).unwrap()
        })
    });

    c.bench_function("full_pipeline_fibonacci", |b| {
        b.iter(|| {
            let prog = lsharp_syntax::parse(black_box(FIBONACCI_SOURCE)).unwrap();
            let mut infer = Infer::new();
            let types = infer.infer_program(&prog).unwrap();
            let mut lower = Lower::new();
            let module = lower.lower_program(&prog, &types).unwrap();
            lsharp_wasm::wasi::emit_wasm_wasi(&module).unwrap()
        })
    });
}

criterion_group!(
    benches,
    bench_parse,
    bench_infer,
    bench_lower,
    bench_codegen,
    bench_full_pipeline,
);
criterion_main!(benches);
