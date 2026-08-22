//! computation builder の診断契約 (`I-44` / `COMP-BUILDER-01`)
//!
//! - 未登録の builder 名は `LS1001` (`UndefinedVar`) で拒否する
//! - builder が登録済みでも bind / return の member が環境に無ければ拒否する
//! - builder が完全なら、最後のステップの型が結果型として保たれる

use lsharp_types::{
    infer::{Infer, TypeError},
    types::Type,
};

#[test]
fn unknown_computation_builder_reports_stable_diagnostic() {
    const SOURCE: &str = "(defn main [] (computation missing (return 42)))";
    let program = lsharp_syntax::parse(SOURCE)
        .expect("unknown computation builder も diagnostic のため parse できるべき");

    let error = Infer::new()
        .infer_program(&program)
        .expect_err("unknown computation builder を fresh type で通してはならない");

    assert_eq!(error.code(), "LS1001");
    let TypeError::UndefinedVar { name, span } = error else {
        panic!("unknown computation builder は UndefinedVar であるべき: {error:?}");
    };
    assert_eq!(name, "missing");
    assert_eq!(
        &SOURCE[span.start..span.end],
        "(computation missing (return 42))"
    );
}

#[test]
fn known_computation_builder_preserves_plain_expression_result_type() {
    const SOURCE: &str = r#"
        (computation-builder identity identity-bind identity-return)
        (defn identity-return [x] x)
        (defn identity-bind [m f] (f m))
        (defn main [] (computation identity (+ 1 2)))
    "#;
    let program =
        lsharp_syntax::parse(SOURCE).expect("known computation builder は parse できるべき");

    let inferred = Infer::new()
        .infer_program(&program)
        .expect("known computation builder は型推論できるべき");
    let main = inferred
        .iter()
        .find(|(name, _)| name == "main")
        .expect("main の推論結果が存在するべき");

    assert_eq!(main.1.ty, Type::Fun(Vec::new(), Box::new(Type::int())));
}

#[test]
fn computation_builder_missing_return_function_reports_stable_diagnostic() {
    const SOURCE: &str = r#"
        (computation-builder identity identity-bind missing-return)
        (defn identity-bind [m f] (f m))
        (defn main [] (computation identity (return 42)))
    "#;
    let program = lsharp_syntax::parse(SOURCE).expect("incomplete builder は parse できるべき");

    let error = Infer::new()
        .infer_program(&program)
        .expect_err("missing return function を黙って通してはならない");

    assert_eq!(error.code(), "LS1001");
    let TypeError::UndefinedVar { name, span } = error else {
        panic!("missing return function は UndefinedVar であるべき: {error:?}");
    };
    assert_eq!(name, "missing-return");
    assert_eq!(
        &SOURCE[span.start..span.end],
        "(computation identity (return 42))"
    );
}

#[test]
fn computation_builder_missing_bind_function_reports_stable_diagnostic() {
    const SOURCE: &str = r#"
        (computation-builder identity missing-bind identity-return)
        (defn identity-return [x] x)
        (defn main [] (computation identity (return 42)))
    "#;
    let program = lsharp_syntax::parse(SOURCE).expect("incomplete builder は parse できるべき");

    let error = Infer::new()
        .infer_program(&program)
        .expect_err("missing bind function を黙って通してはならない");

    assert_eq!(error.code(), "LS1001");
    let TypeError::UndefinedVar { name, span } = error else {
        panic!("missing bind function は UndefinedVar であるべき: {error:?}");
    };
    assert_eq!(name, "missing-bind");
    assert_eq!(
        &SOURCE[span.start..span.end],
        "(computation identity (return 42))"
    );
}

/// member の存在検査は「宣言順」ではなく「前方参照込みの環境」に対して行う。
/// `infer_decl_functions` のパス 1 が全 defn を仮登録するので、
/// builder の member が使用箇所より後ろに書かれていても incomplete 扱いにしてはならない。
///
/// 結果型そのものはここでは要求しない。前方参照時点の member は placeholder 型変数であり、
/// `Int` へは解決されない (`I-46`)。本 test が守るのは「誤って拒否しないこと」だけである。
#[test]
fn computation_builder_members_resolve_when_declared_after_use() {
    const SOURCE: &str = r#"
        (computation-builder identity identity-bind identity-return)
        (defn main [] (computation identity (return 42)))
        (defn identity-return [x] x)
        (defn identity-bind [m f] (f m))
    "#;
    let program = lsharp_syntax::parse(SOURCE).expect("前方参照でも parse できるべき");

    let inferred = Infer::new()
        .infer_program(&program)
        .expect("builder の member が使用箇所より後ろにあっても incomplete 扱いにしてはならない");
    assert!(
        inferred.iter().any(|(name, _)| name == "main"),
        "main の推論結果が存在するべき"
    );
}
