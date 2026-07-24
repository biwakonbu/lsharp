use lsharp_syntax::parse;
use lsharp_types::infer::{Infer, TypeError};

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
