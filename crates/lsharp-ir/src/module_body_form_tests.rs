use super::*;

fn parse(source: &str) -> lsharp_syntax::ast::Program {
    lsharp_syntax::parse(source).expect("fixture は parse できるべき")
}

#[test]
fn flat_marker_module_is_accepted() {
    let program = parse("(module Main)\n(defn main [] (print 42))\n");
    assert!(reject_block_form_module_body(&program).is_ok());
}

#[test]
fn program_without_module_decl_is_accepted() {
    let program = parse("(defn main [] (print 42))\n");
    assert!(reject_block_form_module_body(&program).is_ok());
}

#[test]
fn block_form_module_body_is_rejected_with_code_and_span() {
    let source = "(module Main (defn main [] (print 42)))\n";
    let program = parse(source);

    let error = reject_block_form_module_body(&program)
        .expect_err("block 形式 module body は拒否されるべき");

    assert!(error.contains(UNSUPPORTED_MODULE_BODY_CODE), "code: {error}");
    assert!(error.contains("未対応の構文"), "wording: {error}");
    assert!(error.contains("Main"), "module 名: {error}");
    assert!(
        error.contains(&format!("(0..{})", source.trim_end().len())),
        "span: {error}"
    );
}

/// sibling 参照が無い形は compile が成功して無出力バイナリになる経路。
/// `I-39` で最も重いのはこちらなので、単独で pin する。
#[test]
fn block_form_without_sibling_reference_is_rejected() {
    let program = parse("(module Main (defn helper [] 1))\n(defn main [] (print 42))\n");
    assert!(reject_block_form_module_body(&program).is_err());
}

/// `private` で包んだ body も同じ形である。
#[test]
fn block_form_with_private_wrapper_is_rejected() {
    let program = parse("(module Main (private (defn helper [] 1)))\n");
    assert!(reject_block_form_module_body(&program).is_err());
}

/// 入れ子は最外の body が非空なので、最外だけ見れば足りる。
#[test]
fn nested_block_form_is_rejected_at_outermost_module() {
    let program = parse("(module App (module Sub (defn succ [x] (+ x 1))))\n");
    let error = reject_block_form_module_body(&program)
        .expect_err("入れ子 block 形式も拒否されるべき");
    assert!(error.contains("App"), "最外 module 名を指すべき: {error}");
}
