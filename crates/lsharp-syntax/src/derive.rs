//! P10-5: derive マクロ -- 型定義から関数を自動生成
//!
//! ADT (TypeDef) やレコード (RecordDef) の定義から、
//! 以下の関数を自動生成する:
//!
//! - `derive-show`: 値を文字列表現に変換する `show` 関数
//! - `derive-eq`: 構造的等値比較の `eq?` 関数
//!
//! ## 使用方法
//! ```lisp
//! (type (Maybe a) (Just a) Nothing)
//! ;; derive-show が以下を生成:
//! ;; (defn show-Maybe [x] (match x (Just v) (string-concat "Just(" (show v) ")") Nothing "Nothing"))
//! ;; derive-eq が以下を生成:
//! ;; (defn eq-Maybe? [a b] (match a (Just va) (match b (Just vb) (eq? va vb) _ false) Nothing (match b Nothing true _ false)))
//! ```

use crate::ast::{Decl, Expr, Literal, MatchArm, Param, Pattern, Program, Variant};
use crate::span::Span;

/// derive 対象のトレイト
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DeriveKind {
    /// show: 値を文字列に変換
    Show,
    /// eq: 構造的等値比較
    Eq,
}

/// derive マクロの展開結果
#[derive(Debug, Clone)]
pub struct DeriveResult {
    /// 生成された関数宣言
    pub decls: Vec<Decl>,
}

/// derive-show: ADT の各バリアントを文字列化する関数を生成
///
/// 入力: (type (Maybe a) (Just a) Nothing)
/// 出力: (defn show-Maybe [x]
///         (match x
///           (Just v0) (string-concat "Just(" (string-concat (show v0) ")"))
///           Nothing "Nothing"))
pub fn derive_show_adt(name: &str, variants: &[Variant]) -> Decl {
    let s = Span::new(0, 0);
    let func_name = format!("show-{name}");
    let param_name = "x";

    let arms: Vec<MatchArm> = variants
        .iter()
        .map(|variant| {
            let field_count = variant.fields.len();
            if field_count == 0 {
                // 引数なしバリアント: パターンは名前のみ、ボディは文字列リテラル
                MatchArm {
                    span: s,
                    pattern: Pattern::Constructor(s, variant.name.clone(), vec![]),
                    guard: None,
                    body: Expr::Lit(s, Literal::String(variant.name.clone())),
                }
            } else {
                // 引数ありバリアント: パターンは (Variant v0 v1 ...)
                let var_names: Vec<String> = (0..field_count).map(|i| format!("v{i}")).collect();
                let patterns: Vec<Pattern> = var_names
                    .iter()
                    .map(|v| Pattern::Var(s, v.clone()))
                    .collect();

                // ボディ: string-concat で結合
                // "VariantName(" ++ show(v0) ++ ", " ++ show(v1) ++ ")"
                let mut body = Expr::Lit(s, Literal::String(format!("{}(", variant.name)));

                for (i, var_name) in var_names.iter().enumerate() {
                    // (show vi)
                    let show_call = Expr::App(
                        s,
                        Box::new(Expr::Var(s, "show".to_string())),
                        vec![Expr::Var(s, var_name.clone())],
                    );

                    // string-concat
                    body = Expr::App(
                        s,
                        Box::new(Expr::Var(s, "string-concat".to_string())),
                        vec![body, show_call],
                    );

                    if i < field_count - 1 {
                        // カンマ区切り
                        body = Expr::App(
                            s,
                            Box::new(Expr::Var(s, "string-concat".to_string())),
                            vec![body, Expr::Lit(s, Literal::String(", ".to_string()))],
                        );
                    }
                }

                // 閉じ括弧
                body = Expr::App(
                    s,
                    Box::new(Expr::Var(s, "string-concat".to_string())),
                    vec![body, Expr::Lit(s, Literal::String(")".to_string()))],
                );

                MatchArm {
                    span: s,
                    pattern: Pattern::Constructor(s, variant.name.clone(), patterns),
                    guard: None,
                    body,
                }
            }
        })
        .collect();

    Decl::Defn {
        span: s,
        name: func_name,
        params: vec![Param {
            span: s,
            name: param_name.to_string(),
            ty: None,
        }],
        return_ty: None,
        body: Expr::Match(s, Box::new(Expr::Var(s, param_name.to_string())), arms),
        where_clauses: vec![],
        metadata: None,
    }
}

/// derive-eq: ADT の構造的等値比較関数を生成
///
/// 入力: (type (Maybe a) (Just a) Nothing)
/// 出力: (defn eq-Maybe? [a b]
///         (match a
///           (Just va) (match b (Just vb) (eq? va vb) _ false)
///           Nothing (match b Nothing true _ false)))
pub fn derive_eq_adt(name: &str, variants: &[Variant]) -> Decl {
    let s = Span::new(0, 0);
    let func_name = format!("eq-{name}?");

    let arms: Vec<MatchArm> = variants
        .iter()
        .map(|variant| {
            let field_count = variant.fields.len();
            let a_vars: Vec<String> = (0..field_count).map(|i| format!("a{i}")).collect();
            let b_vars: Vec<String> = (0..field_count).map(|i| format!("b{i}")).collect();

            let a_patterns: Vec<Pattern> =
                a_vars.iter().map(|v| Pattern::Var(s, v.clone())).collect();

            let b_patterns: Vec<Pattern> =
                b_vars.iter().map(|v| Pattern::Var(s, v.clone())).collect();

            // 内側の match: b に対して同じバリアントかチェック
            let inner_match_body = if field_count == 0 {
                // 引数なし: 同じバリアントなら true
                Expr::Lit(s, Literal::Bool(true))
            } else {
                // 引数あり: 各フィールドを eq? で比較し && で結合
                // (eq? a0 b0) && (eq? a1 b1) && ...
                // 簡略化: 最後のフィールドから畳み込み
                let mut eq_expr = Expr::App(
                    s,
                    Box::new(Expr::Var(s, "==".to_string())),
                    vec![
                        Expr::Var(s, a_vars[field_count - 1].clone()),
                        Expr::Var(s, b_vars[field_count - 1].clone()),
                    ],
                );

                for i in (0..field_count - 1).rev() {
                    let field_eq = Expr::App(
                        s,
                        Box::new(Expr::Var(s, "==".to_string())),
                        vec![
                            Expr::Var(s, a_vars[i].clone()),
                            Expr::Var(s, b_vars[i].clone()),
                        ],
                    );
                    // (if (== ai bi) <rest> false)
                    eq_expr = Expr::If(
                        s,
                        Box::new(field_eq),
                        Box::new(eq_expr),
                        Box::new(Expr::Lit(s, Literal::Bool(false))),
                    );
                }

                eq_expr
            };

            // 内側 match の腕
            let inner_arms = vec![
                MatchArm {
                    span: s,
                    pattern: Pattern::Constructor(s, variant.name.clone(), b_patterns),
                    guard: None,
                    body: inner_match_body,
                },
                MatchArm {
                    span: s,
                    pattern: Pattern::Wildcard(s),
                    guard: None,
                    body: Expr::Lit(s, Literal::Bool(false)),
                },
            ];

            MatchArm {
                span: s,
                pattern: Pattern::Constructor(s, variant.name.clone(), a_patterns),
                guard: None,
                body: Expr::Match(s, Box::new(Expr::Var(s, "b".to_string())), inner_arms),
            }
        })
        .collect();

    Decl::Defn {
        span: s,
        name: func_name,
        params: vec![
            Param {
                span: s,
                name: "a".to_string(),
                ty: None,
            },
            Param {
                span: s,
                name: "b".to_string(),
                ty: None,
            },
        ],
        return_ty: None,
        body: Expr::Match(s, Box::new(Expr::Var(s, "a".to_string())), arms),
        where_clauses: vec![],
        metadata: None,
    }
}

/// derive-show: レコードの文字列化関数を生成
///
/// 入力: (type Point (record (: x Int) (: y Int)))
/// 出力: (defn show-Point [r]
///         (string-concat "Point{x=" (string-concat (show (Point.x r))
///           (string-concat ", y=" (string-concat (show (Point.y r)) "}")))))
pub fn derive_show_record(name: &str, fields: &[(String, crate::ast::TypeExpr)]) -> Decl {
    let s = Span::new(0, 0);
    let func_name = format!("show-{name}");
    let param_name = "r";

    let mut body = Expr::Lit(s, Literal::String(format!("{name}{{")));

    for (i, (field_name, _)) in fields.iter().enumerate() {
        // フィールドラベル
        let label = if i == 0 {
            format!("{field_name}=")
        } else {
            format!(", {field_name}=")
        };

        body = Expr::App(
            s,
            Box::new(Expr::Var(s, "string-concat".to_string())),
            vec![body, Expr::Lit(s, Literal::String(label))],
        );

        // (show (TypeName.field r))
        let field_access = Expr::FieldAccess(
            s,
            Box::new(Expr::Var(s, param_name.to_string())),
            field_name.clone(),
        );
        let show_call = Expr::App(
            s,
            Box::new(Expr::Var(s, "show".to_string())),
            vec![field_access],
        );

        body = Expr::App(
            s,
            Box::new(Expr::Var(s, "string-concat".to_string())),
            vec![body, show_call],
        );
    }

    // 閉じ括弧
    body = Expr::App(
        s,
        Box::new(Expr::Var(s, "string-concat".to_string())),
        vec![body, Expr::Lit(s, Literal::String("}".to_string()))],
    );

    Decl::Defn {
        span: s,
        name: func_name,
        params: vec![Param {
            span: s,
            name: param_name.to_string(),
            ty: None,
        }],
        return_ty: None,
        body,
        where_clauses: vec![],
        metadata: None,
    }
}

/// プログラム全体に derive を適用
/// 型定義に `:derive` メタデータがあれば、対応する関数を生成して追加
pub fn apply_derives(program: &Program) -> Vec<Decl> {
    let mut derived = Vec::new();

    for decl in &program.decls {
        match decl {
            Decl::TypeDef { name, variants, .. } => {
                // メタデータから derive 対象を取得
                // 現在は全 TypeDef に対して show と eq を生成
                // (将来的には :derive [Show Eq] メタデータで制御)
                {
                    // デフォルトで show と eq を derive
                    derived.push(derive_show_adt(name, variants));
                    derived.push(derive_eq_adt(name, variants));
                }
            }
            Decl::RecordDef { name, fields, .. } => {
                derived.push(derive_show_record(name, fields));
            }
            _ => {}
        }
    }

    derived
}

#[cfg(test)]
mod tests;
