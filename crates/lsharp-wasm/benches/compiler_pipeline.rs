//! コンパイラパイプライン ベンチマーク
//!
//! パイプラインの各ステージ (parse → infer → lower → codegen) を個別に計測し、
//! ボトルネックの特定と回帰検出に使用する。

use criterion::{criterion_group, criterion_main, Criterion};
use lsharp_ir::lower::Lower;
use lsharp_types::infer::Infer;

// ベンチマーク用フィクスチャ
const FIB_SOURCE: &str = include_str!("../../../examples/fib.ls");
const FACTORIAL_SOURCE: &str = include_str!("../../../examples/factorial.ls");
const HELLO_SOURCE: &str = include_str!("../../../examples/hello.ls");

/// パース: ソースコード → AST
fn bench_parse(c: &mut Criterion) {
    let mut group = c.benchmark_group("parse");

    group.bench_function("fib", |b| {
        b.iter(|| lsharp_syntax::parse(FIB_SOURCE).unwrap());
    });

    group.bench_function("factorial", |b| {
        b.iter(|| lsharp_syntax::parse(FACTORIAL_SOURCE).unwrap());
    });

    group.bench_function("hello", |b| {
        b.iter(|| lsharp_syntax::parse(HELLO_SOURCE).unwrap());
    });

    group.finish();
}

/// 型推論: AST → 型チェック済み AST
fn bench_infer(c: &mut Criterion) {
    let fib_ast = lsharp_syntax::parse(FIB_SOURCE).unwrap();
    let factorial_ast = lsharp_syntax::parse(FACTORIAL_SOURCE).unwrap();
    let hello_ast = lsharp_syntax::parse(HELLO_SOURCE).unwrap();

    let mut group = c.benchmark_group("infer");

    group.bench_function("fib", |b| {
        b.iter(|| {
            let mut infer = Infer::new();
            infer.infer_program(&fib_ast).unwrap()
        });
    });

    group.bench_function("factorial", |b| {
        b.iter(|| {
            let mut infer = Infer::new();
            infer.infer_program(&factorial_ast).unwrap()
        });
    });

    group.bench_function("hello", |b| {
        b.iter(|| {
            let mut infer = Infer::new();
            infer.infer_program(&hello_ast).unwrap()
        });
    });

    group.finish();
}

/// IR 変換: AST → IR
fn bench_lower(c: &mut Criterion) {
    let fib_ast = lsharp_syntax::parse(FIB_SOURCE).unwrap();
    let factorial_ast = lsharp_syntax::parse(FACTORIAL_SOURCE).unwrap();
    let hello_ast = lsharp_syntax::parse(HELLO_SOURCE).unwrap();

    let mut fib_infer = Infer::new();
    let fib_types = fib_infer.infer_program(&fib_ast).unwrap();

    let mut factorial_infer = Infer::new();
    let factorial_types = factorial_infer.infer_program(&factorial_ast).unwrap();

    let mut hello_infer = Infer::new();
    let hello_types = hello_infer.infer_program(&hello_ast).unwrap();

    let mut group = c.benchmark_group("lower");

    group.bench_function("fib", |b| {
        b.iter(|| {
            let mut lower = Lower::new();
            lower.lower_program(&fib_ast, &fib_types).unwrap()
        });
    });

    group.bench_function("factorial", |b| {
        b.iter(|| {
            let mut lower = Lower::new();
            lower.lower_program(&factorial_ast, &factorial_types).unwrap()
        });
    });

    group.bench_function("hello", |b| {
        b.iter(|| {
            let mut lower = Lower::new();
            lower.lower_program(&hello_ast, &hello_types).unwrap()
        });
    });

    group.finish();
}

/// コード生成: IR → Wasm バイナリ
fn bench_codegen(c: &mut Criterion) {
    let fib_ast = lsharp_syntax::parse(FIB_SOURCE).unwrap();
    let mut fib_infer = Infer::new();
    let fib_types = fib_infer.infer_program(&fib_ast).unwrap();
    let mut fib_lower = Lower::new();
    let fib_module = fib_lower.lower_program(&fib_ast, &fib_types).unwrap();

    let factorial_ast = lsharp_syntax::parse(FACTORIAL_SOURCE).unwrap();
    let mut factorial_infer = Infer::new();
    let factorial_types = factorial_infer.infer_program(&factorial_ast).unwrap();
    let mut factorial_lower = Lower::new();
    let factorial_module = factorial_lower
        .lower_program(&factorial_ast, &factorial_types)
        .unwrap();

    let hello_ast = lsharp_syntax::parse(HELLO_SOURCE).unwrap();
    let mut hello_infer = Infer::new();
    let hello_types = hello_infer.infer_program(&hello_ast).unwrap();
    let mut hello_lower = Lower::new();
    let hello_module = hello_lower
        .lower_program(&hello_ast, &hello_types)
        .unwrap();

    let mut group = c.benchmark_group("codegen");

    group.bench_function("fib", |b| {
        b.iter(|| lsharp_wasm::wasi::emit_wasm_wasi(&fib_module).unwrap());
    });

    group.bench_function("factorial", |b| {
        b.iter(|| lsharp_wasm::wasi::emit_wasm_wasi(&factorial_module).unwrap());
    });

    group.bench_function("hello", |b| {
        b.iter(|| lsharp_wasm::wasi::emit_wasm_wasi(&hello_module).unwrap());
    });

    group.finish();
}

/// フルパイプライン: ソースコード → Wasm バイナリ (E2E)
fn bench_full_pipeline(c: &mut Criterion) {
    let mut group = c.benchmark_group("full_pipeline");

    group.bench_function("fib", |b| {
        b.iter(|| {
            let ast = lsharp_syntax::parse(FIB_SOURCE).unwrap();
            let mut infer = Infer::new();
            let types = infer.infer_program(&ast).unwrap();
            let mut lower = Lower::new();
            let module = lower.lower_program(&ast, &types).unwrap();
            lsharp_wasm::wasi::emit_wasm_wasi(&module).unwrap()
        });
    });

    group.bench_function("factorial", |b| {
        b.iter(|| {
            let ast = lsharp_syntax::parse(FACTORIAL_SOURCE).unwrap();
            let mut infer = Infer::new();
            let types = infer.infer_program(&ast).unwrap();
            let mut lower = Lower::new();
            let module = lower.lower_program(&ast, &types).unwrap();
            lsharp_wasm::wasi::emit_wasm_wasi(&module).unwrap()
        });
    });

    group.bench_function("hello", |b| {
        b.iter(|| {
            let ast = lsharp_syntax::parse(HELLO_SOURCE).unwrap();
            let mut infer = Infer::new();
            let types = infer.infer_program(&ast).unwrap();
            let mut lower = Lower::new();
            let module = lower.lower_program(&ast, &types).unwrap();
            lsharp_wasm::wasi::emit_wasm_wasi(&module).unwrap()
        });
    });

    group.finish();
}

criterion_group!(
    benches,
    bench_parse,
    bench_infer,
    bench_lower,
    bench_codegen,
    bench_full_pipeline
);
criterion_main!(benches);
