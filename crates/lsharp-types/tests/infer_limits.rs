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

/// depth を持つ `ref-new` の入れ子越しに self-application させる。
/// 診断は LS1003 (InfiniteType) で、深さによらず安定して返るべき。
fn occur_check_program(depth: usize) -> String {
    assert!(depth > 0, "the occur-check fixture needs at least one wrap");

    let nested = (0..depth).fold(String::from("value"), |inner, _| format!("(ref-new {inner})"));
    format!("(defn occur [value] (value {nested}))")
}

#[test]
fn occur_check_reports_infinite_type_at_documented_depths() {
    for depth in [32, 64, 128] {
        let source = occur_check_program(depth);
        let program = parse(&source).expect("occur-check fixture must parse");
        let mut infer = Infer::new();
        let error = infer
            .infer_program(&program)
            .expect_err("nested self-application must fail the occurs check");

        assert!(
            matches!(&error, TypeError::InfiniteType { .. }),
            "depth {depth} should expose InfiniteType, got {error:?}"
        );
        assert_eq!(error.code(), "LS1003", "unexpected code at depth {depth}");
    }
}

/// 多引数適用では、先行引数が確定させた substitution が後続引数の環境へ伝わる。
/// Apply の環境更新を最適化しても、この契約は落としてはならない。
#[test]
fn multi_argument_application_propagates_prior_argument_substitution() {
    let program = parse("(defn pair [left right] left) (defn inconsistent [f] (pair (f 1) (f true)))")
        .expect("multi-argument substitution fixture must parse");
    let mut infer = Infer::new();
    let error = infer
        .infer_program(&program)
        .expect_err("the first argument must constrain f before the second argument");

    assert_eq!(error.code(), "LS1004");
}
