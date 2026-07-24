use criterion::{black_box, criterion_group, criterion_main, Criterion};
use lsharp_syntax::parse;
use lsharp_types::infer::Infer;

fn nested_type_annotation_source(depth: usize) -> String {
    assert!(depth > 0, "the deep type benchmark needs a constructor");

    let nested = (0..depth).fold(String::from("Int"), |inner, _| format!("(Box {inner})"));
    format!("(defn identity [(: value {nested})] : {nested} value)")
}

fn wide_record_type_annotation_source(field_count: usize) -> String {
    assert!(field_count > 0, "the wide record benchmark needs a field");

    let fields = (0..field_count)
        .map(|index| format!("(: field{index} Int)"))
        .collect::<Vec<_>>()
        .join(" ");
    format!("(type Wide (record {fields})) (defn identity [(: value Wide)] : Wide value)")
}

fn bench_deep_type_annotation(c: &mut Criterion) {
    let source = nested_type_annotation_source(128);
    c.bench_function("infer_deep_type_annotation_128", |b| {
        b.iter(|| {
            let program = parse(black_box(&source)).expect("deep type fixture must parse");
            let mut infer = Infer::new();
            black_box(
                infer
                    .infer_program(&program)
                    .expect("deep type inference must succeed"),
            );
        });
    });
}

fn bench_wide_record_annotation(c: &mut Criterion) {
    let source = wide_record_type_annotation_source(256);
    c.bench_function("infer_wide_record_annotation_256", |b| {
        b.iter(|| {
            let program = parse(black_box(&source)).expect("wide record fixture must parse");
            let mut infer = Infer::new();
            black_box(
                infer
                    .infer_program(&program)
                    .expect("wide record inference must succeed"),
            );
        });
    });
}

criterion_group!(
    benches,
    bench_deep_type_annotation,
    bench_wide_record_annotation
);
criterion_main!(benches);
