//! block 形式の module body (`(module M (defn f ...) ...)`) を compile 経路で拒否する。
//!
//! parser も型推論も検査系もこの形を受理するが、**lowering は `Decl::ModuleDecl` を
//! 一度も見ない**。`lower/program.rs` は `Decl::Defn` だけを拾い、multi-file 経路の
//! `compile_entrypoints.rs` は `ModuleDecl` を丸ごと捨てる。結果として body 内の宣言は
//! IR に到達せず、症状が 2 つに分かれる。
//!
//! 1. body 内に sibling 参照があると `未定義の変数` という誤診断で止まる
//! 2. sibling 参照が無いと **compile が成功し、何も起きないバイナリが exit 0 で完成する**
//!
//! 2 の方が重い。失敗を知らせるものが何も無いためである。
//!
//! この検査は **infer より前**に置く。infer より後ろだと 1 の側が先に誤診断で止まり、
//! この診断が出ない。
//!
//! 判断と却下案 (実装 / parser で reject / lowering で reject) は
//! `docs/adr/decisions-module-body-form-rejection.md`。

use lsharp_syntax::ast::{Decl, Program};

/// 未対応の構文としての診断 code。
pub const UNSUPPORTED_MODULE_BODY_CODE: &str = "LS0105";

/// block 形式の module body を検出したら診断文字列を返す。
///
/// 入れ子 (`(module A (module B ...))`) は最外の body が非空なので、
/// top-level の宣言を 1 度走査すれば足りる。
pub fn reject_block_form_module_body(program: &Program) -> Result<(), String> {
    for decl in &program.decls {
        if let Decl::ModuleDecl { span, name, body } = decl
            && !body.is_empty()
        {
            return Err(format!(
                "[{UNSUPPORTED_MODULE_BODY_CODE}] 未対応の構文: \
                 module 本体を括弧の中に置く形 `(module {name} ...)` はまだ \
                 lowering されません ({}..{})。\
                 `(module {name})` と宣言して、宣言は top-level へ並べてください。",
                span.start, span.end
            ));
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    include!("module_body_form_tests.rs");
}
