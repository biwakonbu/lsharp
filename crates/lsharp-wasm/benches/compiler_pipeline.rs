//! コンパイラパイプライン ベンチマーク
//!
//! パイプラインの各ステージ (parse → infer → lower → codegen) を個別に計測し、
//! ボトルネックの特定と回帰検出に使用する。

use std::time::Duration;

use criterion::{BatchSize, Criterion, criterion_group, criterion_main};
use lsharp_ir::lower::Lower;
use lsharp_ir::{CompilationCache, compile_multi_file, compile_multi_file_incremental};
use lsharp_types::infer::Infer;
use lsharp_wasm::incremental_bench::SelfhostIncrementalBenchFixture;

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
            lower
                .lower_program(&factorial_ast, &factorial_types)
                .unwrap()
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
    let hello_module = hello_lower.lower_program(&hello_ast, &hello_types).unwrap();

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

struct IncrementalCompileBenchState {
    fixture: SelfhostIncrementalBenchFixture,
    cache: CompilationCache,
}

fn prepare_incremental_warm_state() -> IncrementalCompileBenchState {
    let fixture = SelfhostIncrementalBenchFixture::create().expect("fixture should be created");
    let mut cache = CompilationCache::new();
    compile_multi_file_incremental(fixture.entry_path(), &mut cache)
        .expect("warm-up incremental compile should succeed");
    IncrementalCompileBenchState { fixture, cache }
}

fn prepare_incremental_single_change_state() -> IncrementalCompileBenchState {
    let fixture = SelfhostIncrementalBenchFixture::create().expect("fixture should be created");
    let mut cache = CompilationCache::new();
    compile_multi_file_incremental(fixture.entry_path(), &mut cache)
        .expect("warm-up incremental compile should succeed");
    fixture
        .apply_changed_module_variant()
        .expect("single-module change should be staged");
    IncrementalCompileBenchState { fixture, cache }
}

fn bench_incremental_compile_selfhost(c: &mut Criterion) {
    let mut group = c.benchmark_group("incremental_compile_selfhost");
    group.sample_size(10);
    group.measurement_time(Duration::from_secs(20));

    group.bench_function("full_compile_app_main", |b| {
        b.iter_batched(
            || SelfhostIncrementalBenchFixture::create().expect("fixture should be created"),
            |fixture| {
                compile_multi_file(fixture.entry_path()).expect("full compile should succeed")
            },
            BatchSize::PerIteration,
        );
    });

    group.bench_function("incremental_cold_app_main", |b| {
        b.iter_batched(
            || {
                (
                    SelfhostIncrementalBenchFixture::create().expect("fixture should be created"),
                    CompilationCache::new(),
                )
            },
            |(fixture, mut cache)| {
                compile_multi_file_incremental(fixture.entry_path(), &mut cache)
                    .expect("cold incremental compile should succeed")
            },
            BatchSize::PerIteration,
        );
    });

    group.bench_function("incremental_warm_clean_rebuild_app_main", |b| {
        b.iter_batched_ref(
            prepare_incremental_warm_state,
            |state| {
                compile_multi_file_incremental(state.fixture.entry_path(), &mut state.cache)
                    .expect("warm clean rebuild should succeed")
            },
            BatchSize::PerIteration,
        );
    });

    group.bench_function("incremental_single_module_change_app_main", |b| {
        b.iter_batched_ref(
            prepare_incremental_single_change_state,
            |state| {
                compile_multi_file_incremental(state.fixture.entry_path(), &mut state.cache)
                    .expect("single-module incremental compile should succeed")
            },
            BatchSize::PerIteration,
        );
    });

    group.finish();
}

criterion_group!(
    benches,
    bench_parse,
    bench_infer,
    bench_lower,
    bench_codegen,
    bench_full_pipeline,
    bench_incremental_compile_selfhost
);
criterion_main!(benches);
