use lsharp_syntax::parse;
use lsharp_types::infer::{Infer, TypeError};

fn nested_type_annotation_program(depth: usize) -> String {
    assert!(
        depth > 0,
        "the limit fixture needs at least one constructor"
    );

    let nested = (0..depth).fold(String::from("Int"), |inner, _| format!("(Box {inner})"));
    format!("(defn identity [(: value {nested})] : {nested} value)")
}

fn wide_record_type_annotation_program(field_count: usize) -> String {
    assert!(
        field_count > 0,
        "the record fixture needs at least one field"
    );

    let fields = (0..field_count)
        .map(|index| format!("(: field{index} Int)"))
        .collect::<Vec<_>>()
        .join(" ");
    format!("(type Wide (record {fields})) (defn identity [(: value Wide)] : Wide value)")
}

#[test]
fn self_application_reports_infinite_type() {
    let program = parse("(defn omega [f] (f f))").expect("self-application fixture must parse");
    let mut infer = Infer::new();
    let error = infer
        .infer_program(&program)
        .expect_err("self-application must fail the occurs check");

    assert!(
        matches!(&error, TypeError::InfiniteType { .. }),
        "self-application should expose InfiniteType, got {error:?}"
    );
    assert_eq!(error.code(), "LS1003");
}

#[test]
fn wide_record_type_annotations_do_not_panic() {
    for field_count in [128, 256] {
        let source = wide_record_type_annotation_program(field_count);
        let outcome = std::panic::catch_unwind(|| {
            let program = parse(&source).expect("wide record fixture must parse");
            let mut infer = Infer::new();
            infer.infer_program(&program)
        });

        let result =
            outcome.unwrap_or_else(|_| panic!("type inference panicked at {field_count} fields"));
        assert!(
            result.is_ok(),
            "type inference failed at {field_count} fields: {result:?}"
        );
    }
}

#[test]
fn deeply_nested_type_annotations_do_not_panic() {
    for depth in [32, 64, 128] {
        let source = nested_type_annotation_program(depth);
        let outcome = std::panic::catch_unwind(|| {
            let program = parse(&source).expect("deep type fixture must parse");
            let mut infer = Infer::new();
            infer.infer_program(&program)
        });

        let result = outcome.unwrap_or_else(|_| panic!("type inference panicked at depth {depth}"));
        assert!(
            result.is_ok(),
            "type inference failed at depth {depth}: {result:?}"
        );
    }
}
