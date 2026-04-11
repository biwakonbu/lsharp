//! P10-2/P10-3: マクロ展開エンジン
//!
//! defmacro で定義されたマクロを展開する。
//! パイプライン: parse → **macro_expand** → type inference → lowering → codegen
//!
//! ## 展開ルール
//! - `(macro-name arg1 arg2)` → マクロ本体の Quote 式を展開
//! - Quote 内の `~param` → 対応する引数に置換
//! - Quote 内の `~@param` → リストを展開 (splice)
//! - 再帰展開は深度制限 128 まで
//!
//! ## P10-3: 型付きマクロ
//! - `:type` シグネチャをパース・保存 (検証は型推論フェーズで実施)
//! - 展開トレースバック: マクロ展開の履歴を保持し、エラー時に展開元を表示

use std::collections::HashMap;

use crate::ast::{Decl, Expr, Program, TypeExpr};
use crate::span::Span;

/// P10-3: マクロ展開トレースの1エントリ
#[derive(Debug, Clone)]
pub struct MacroExpansionStep {
    /// 展開されたマクロ名
    pub macro_name: String,
    /// 呼び出し元のスパン
    pub call_span: Span,
    /// 展開の深さ
    pub depth: usize,
}

/// マクロ展開エラー
#[derive(Debug, Clone, thiserror::Error)]
pub enum MacroExpandError {
    #[error("未定義マクロ: {name}")]
    UndefinedMacro { name: String, span: Span },
    #[error(
        "マクロ引数の数が一致しません: {name} は {expected} 個の引数を受け取りますが、{actual} 個が渡されました"
    )]
    ArityMismatch {
        name: String,
        expected: usize,
        actual: usize,
        span: Span,
    },
    #[error("マクロ展開の再帰制限 ({limit}) を超えました: {name}")]
    RecursionLimit {
        name: String,
        limit: usize,
        span: Span,
    },
    #[error("unquote-splicing (~@) はリストコンテキストでのみ使用できます")]
    SpliceOutsideList { span: Span },
    /// P10-3: 展開トレースバック付きエラー
    #[error("マクロ展開エラー (トレースバック付き): {inner}")]
    WithTrace {
        inner: Box<MacroExpandError>,
        /// 展開トレース (呼び出し順)
        trace: Vec<MacroExpansionStep>,
    },
}

impl MacroExpandError {
    /// P10-3: トレースバックを付与してエラーを返す
    pub fn with_trace(self, trace: Vec<MacroExpansionStep>) -> Self {
        if trace.is_empty() {
            return self;
        }
        MacroExpandError::WithTrace {
            inner: Box::new(self),
            trace,
        }
    }

    /// P10-3: トレースバックのフォーマット済み文字列を取得
    pub fn format_traceback(&self) -> String {
        match self {
            MacroExpandError::WithTrace { inner, trace } => {
                let mut msg = format!("{inner}\n\nマクロ展開トレースバック:");
                for (i, step) in trace.iter().enumerate() {
                    msg.push_str(&format!(
                        "\n  [{i}] {} (depth={}, span={}..{})",
                        step.macro_name, step.depth, step.call_span.start, step.call_span.end
                    ));
                }
                msg
            }
            other => format!("{other}"),
        }
    }
}

/// マクロ定義
#[derive(Debug, Clone)]
struct MacroDef {
    params: Vec<String>,
    body: Expr,
    /// P10-3: オプションの型シグネチャ
    type_sig: Option<TypeExpr>,
}

/// マクロ展開器
pub struct MacroExpander {
    /// 登録済みマクロ
    macros: HashMap<String, MacroDef>,
    /// 再帰展開の深度制限
    max_depth: usize,
    /// gensym カウンター (簡易衛生性)
    gensym_counter: u64,
    /// P10-3: 展開トレースの記録
    expansion_trace: Vec<MacroExpansionStep>,
    /// P10-5: Computation Builder 登録情報 (ビルダー名 -> (bind関数名, return関数名))
    computation_builders: HashMap<String, (String, String)>,
}

impl MacroExpander {
    pub fn new() -> Self {
        Self {
            macros: HashMap::new(),
            max_depth: 128,
            gensym_counter: 0,
            expansion_trace: Vec::new(),
            computation_builders: HashMap::new(),
        }
    }

    /// 組み込みマクロを登録済みの展開器を作成
    pub fn with_builtins() -> Self {
        let mut expander = Self::new();
        expander.register_builtins();
        expander
    }

    /// 組み込みマクロを登録
    /// - when: (when test body) -> (if test body ())
    /// - unless: (unless test body) -> (if test () body)
    /// - assert: (assert expr) -> (if expr () (do (print "Assertion failed") 0))
    fn register_builtins(&mut self) {
        use crate::ast::Literal;
        let s = Span::new(0, 0); // ダミースパン

        // (when test body) -> (if test body ())
        self.macros.insert(
            "when".to_string(),
            MacroDef {
                params: vec!["test".to_string(), "body".to_string()],
                body: Expr::Quote(
                    s,
                    Box::new(Expr::If(
                        s,
                        Box::new(Expr::Unquote(s, Box::new(Expr::Var(s, "test".to_string())))),
                        Box::new(Expr::Unquote(s, Box::new(Expr::Var(s, "body".to_string())))),
                        Box::new(Expr::Lit(s, Literal::Unit)),
                    )),
                ),
                type_sig: None,
            },
        );

        // (unless test body) -> (if test () body)
        self.macros.insert(
            "unless".to_string(),
            MacroDef {
                params: vec!["test".to_string(), "body".to_string()],
                body: Expr::Quote(
                    s,
                    Box::new(Expr::If(
                        s,
                        Box::new(Expr::Unquote(s, Box::new(Expr::Var(s, "test".to_string())))),
                        Box::new(Expr::Lit(s, Literal::Unit)),
                        Box::new(Expr::Unquote(s, Box::new(Expr::Var(s, "body".to_string())))),
                    )),
                ),
                type_sig: None,
            },
        );

        // (assert expr) -> (if expr () (do (print "Assertion failed") 0))
        self.macros.insert(
            "assert".to_string(),
            MacroDef {
                params: vec!["expr".to_string()],
                body: Expr::Quote(
                    s,
                    Box::new(Expr::If(
                        s,
                        Box::new(Expr::Unquote(s, Box::new(Expr::Var(s, "expr".to_string())))),
                        Box::new(Expr::Lit(s, Literal::Unit)),
                        Box::new(Expr::Do(
                            s,
                            vec![
                                Expr::App(
                                    s,
                                    Box::new(Expr::Var(s, "print".to_string())),
                                    vec![Expr::Lit(
                                        s,
                                        Literal::String("Assertion failed".to_string()),
                                    )],
                                ),
                                Expr::Lit(s, Literal::Int(0)),
                            ],
                        )),
                    )),
                ),
                type_sig: None,
            },
        );
    }
    /// P10-5: cond 式の展開
    /// (cond c1 e1 c2 e2 ... default) -> (if c1 e1 (if c2 e2 ... default))
    fn expand_cond(
        &mut self,
        span: Span,
        args: Vec<Expr>,
        depth: usize,
    ) -> Result<Expr, MacroExpandError> {
        use crate::ast::Literal;
        if args.is_empty() {
            // 引数なし: () を返す
            return Ok(Expr::Lit(span, Literal::Unit));
        }
        if args.len() == 1 {
            // default 値のみ
            return self.expand_expr(args.into_iter().next().unwrap(), depth + 1);
        }
        // 条件-値のペアを再帰的に if-else チェーンに展開
        let mut iter = args.into_iter();
        let cond_expr = iter.next().unwrap();
        let then_expr = iter.next().unwrap();
        let rest: Vec<Expr> = iter.collect();

        let expanded_cond = self.expand_expr(cond_expr, depth + 1)?;
        let expanded_then = self.expand_expr(then_expr, depth + 1)?;
        let expanded_else = self.expand_cond(span, rest, depth + 1)?;

        Ok(Expr::If(
            span,
            Box::new(expanded_cond),
            Box::new(expanded_then),
            Box::new(expanded_else),
        ))
    }

    /// P10-5: |> パイプライン演算子の展開
    /// (|> val f1 f2 f3) -> (f3 (f2 (f1 val)))
    fn expand_pipe_forward(
        &mut self,
        span: Span,
        args: Vec<Expr>,
        depth: usize,
    ) -> Result<Expr, MacroExpandError> {
        if args.is_empty() {
            use crate::ast::Literal;
            return Ok(Expr::Lit(span, Literal::Unit));
        }
        let mut iter = args.into_iter();
        let mut acc = self.expand_expr(iter.next().unwrap(), depth + 1)?;

        for func_expr in iter {
            let expanded_func = self.expand_expr(func_expr, depth + 1)?;
            // (func acc) の形に変換
            // func が App の場合 (部分適用): (f arg1 arg2) + val -> (f arg1 arg2 val)
            match expanded_func {
                Expr::App(s, f, mut existing_args) => {
                    existing_args.push(acc);
                    acc = Expr::App(s, f, existing_args);
                }
                other => {
                    acc = Expr::App(span, Box::new(other), vec![acc]);
                }
            }
        }

        Ok(acc)
    }

    /// P10-5: Computation Expression の脱糖
    /// (computation builder (let! x e1) (return e2))
    ///   => (bind-fn e1 (fn [x] (return-fn e2)))
    /// (computation builder (do! e1) (return e2))
    ///   => (bind-fn e1 (fn [_] (return-fn e2)))
    /// (computation builder (return e))
    ///   => (return-fn e)
    fn desugar_computation(
        &mut self,
        span: Span,
        bind_fn: &str,
        return_fn: &str,
        steps: Vec<crate::ast::ComputationStep>,
        depth: usize,
    ) -> Result<Expr, MacroExpandError> {
        use crate::ast::{ComputationStep, Param, Pattern};

        if steps.is_empty() {
            use crate::ast::Literal;
            return Ok(Expr::Lit(span, Literal::Unit));
        }

        // 最後のステップから逆順に脱糖を構築
        let mut steps_vec: Vec<ComputationStep> = steps;
        let last = steps_vec.pop().unwrap();

        // 最後のステップを展開
        let mut result = match last {
            ComputationStep::Return(_, e) => {
                // (return e) => (return-fn e)
                let expanded_e = self.expand_expr(e, depth + 1)?;
                Expr::App(
                    span,
                    Box::new(Expr::Var(span, return_fn.to_string())),
                    vec![expanded_e],
                )
            }
            ComputationStep::Expr(e) => {
                // 末尾の通常式はそのまま
                self.expand_expr(e, depth + 1)?
            }
            ComputationStep::LetBang(_, pat, e) => {
                // 末尾の let! は bind + unit return
                let expanded_e = self.expand_expr(e, depth + 1)?;
                let var_name = match &pat {
                    Pattern::Var(_, name) => name.clone(),
                    _ => self.gensym("comp"),
                };
                let unit_return = Expr::App(
                    span,
                    Box::new(Expr::Var(span, return_fn.to_string())),
                    vec![Expr::Var(span, var_name.clone())],
                );
                Expr::App(
                    span,
                    Box::new(Expr::Var(span, bind_fn.to_string())),
                    vec![
                        expanded_e,
                        Expr::Lambda(
                            span,
                            vec![Param {
                                span,
                                name: var_name,
                                ty: None,
                            }],
                            Box::new(unit_return),
                        ),
                    ],
                )
            }
            ComputationStep::DoBang(_, e) => {
                // 末尾の do! はそのまま実行
                self.expand_expr(e, depth + 1)?
            }
        };

        // 残りのステップを逆順に fold
        for step in steps_vec.into_iter().rev() {
            result = match step {
                ComputationStep::LetBang(_, pat, e) => {
                    // (let! x e rest) => (bind-fn e (fn [x] rest))
                    let expanded_e = self.expand_expr(e, depth + 1)?;
                    let var_name = match &pat {
                        Pattern::Var(_, name) => name.clone(),
                        _ => self.gensym("comp"),
                    };
                    Expr::App(
                        span,
                        Box::new(Expr::Var(span, bind_fn.to_string())),
                        vec![
                            expanded_e,
                            Expr::Lambda(
                                span,
                                vec![Param {
                                    span,
                                    name: var_name,
                                    ty: None,
                                }],
                                Box::new(result),
                            ),
                        ],
                    )
                }
                ComputationStep::DoBang(_, e) => {
                    // (do! e rest) => (bind-fn e (fn [_] rest))
                    let expanded_e = self.expand_expr(e, depth + 1)?;
                    let ignore_var = self.gensym("_");
                    Expr::App(
                        span,
                        Box::new(Expr::Var(span, bind_fn.to_string())),
                        vec![
                            expanded_e,
                            Expr::Lambda(
                                span,
                                vec![Param {
                                    span,
                                    name: ignore_var,
                                    ty: None,
                                }],
                                Box::new(result),
                            ),
                        ],
                    )
                }
                ComputationStep::Return(_, e) => {
                    // return が途中にある場合 (通常は最後)
                    let expanded_e = self.expand_expr(e, depth + 1)?;
                    Expr::App(
                        span,
                        Box::new(Expr::Var(span, return_fn.to_string())),
                        vec![expanded_e],
                    )
                }
                ComputationStep::Expr(e) => {
                    // 通常式は単に展開
                    let _expanded = self.expand_expr(e, depth + 1)?;
                    result
                }
            };
        }

        Ok(result)
    }

    /// gensym: ユニークなシンボル名を生成 (簡易衛生性)
    pub fn gensym(&mut self, prefix: &str) -> String {
        self.gensym_counter += 1;
        format!("__gensym_{}_{}", prefix, self.gensym_counter)
    }

    /// P10-3: 展開トレースを取得
    pub fn expansion_trace(&self) -> &[MacroExpansionStep] {
        &self.expansion_trace
    }

    /// P10-3: マクロの型シグネチャを取得
    pub fn macro_type_sig(&self, name: &str) -> Option<&TypeExpr> {
        self.macros.get(name).and_then(|m| m.type_sig.as_ref())
    }

    /// プログラム全体のマクロ展開
    /// - DefMacro 宣言を収集してマクロテーブルに登録
    /// - 残りの宣言内の式を展開
    /// - DefMacro 宣言自体は出力から除去
    pub fn expand_program(&mut self, program: Program) -> Result<Program, MacroExpandError> {
        self.expansion_trace.clear();
        let mut expanded_decls = Vec::new();

        for decl in program.decls {
            match decl {
                Decl::ComputationBuilder {
                    name,
                    bind_fn,
                    return_fn,
                    ..
                } => {
                    // P10-5: Computation Builder を登録 (マクロ展開時に脱糖するため)
                    self.computation_builders
                        .insert(name.clone(), (bind_fn.clone(), return_fn.clone()));
                    // ComputationBuilder 宣言は出力に残す (型推論でも参照するため)
                    expanded_decls.push(Decl::ComputationBuilder {
                        span: Span::new(0, 0),
                        name,
                        bind_fn,
                        return_fn,
                    });
                }
                Decl::DefMacro {
                    name,
                    params,
                    macro_type,
                    body,
                    ..
                } => {
                    // マクロ定義を登録 (P10-3: 型シグネチャも保存)
                    let param_names: Vec<String> = params.iter().map(|p| p.name.clone()).collect();
                    self.macros.insert(
                        name,
                        MacroDef {
                            params: param_names,
                            body,
                            type_sig: macro_type,
                        },
                    );
                }
                other => {
                    let expanded = self.expand_decl(other).map_err(|e| {
                        // P10-3: エラーにトレースバックを付与
                        e.with_trace(self.expansion_trace.clone())
                    })?;
                    expanded_decls.push(expanded);
                }
            }
        }

        Ok(Program {
            decls: expanded_decls,
        })
    }

    /// 宣言内の式を展開
    fn expand_decl(&mut self, decl: Decl) -> Result<Decl, MacroExpandError> {
        match decl {
            Decl::Defn {
                span,
                name,
                params,
                return_ty,
                body,
                where_clauses,
                metadata,
            } => {
                let expanded_body = self.expand_expr(body, 0)?;
                Ok(Decl::Defn {
                    span,
                    name,
                    params,
                    return_ty,
                    body: expanded_body,
                    where_clauses,
                    metadata,
                })
            }
            // 他の宣言は再帰的にボディ内を展開する必要はない（現時点では）
            other => Ok(other),
        }
    }

    /// 式のマクロ展開 (再帰)
    fn expand_expr(&mut self, expr: Expr, depth: usize) -> Result<Expr, MacroExpandError> {
        if depth > self.max_depth {
            return Err(MacroExpandError::RecursionLimit {
                name: "<unknown>".to_string(),
                limit: self.max_depth,
                span: expr.span(),
            });
        }

        match expr {
            // 関数適用: マクロ呼び出しかチェック
            Expr::App(span, func, args) => {
                // func が Var で、組み込み特殊形式 or マクロかチェック
                if let Expr::Var(_, ref name) = *func {
                    // P10-5: cond マクロ -- (cond c1 e1 c2 e2 ... default)
                    // if-else チェーンに展開: (if c1 e1 (if c2 e2 ... default))
                    if name == "cond" {
                        self.expansion_trace.push(MacroExpansionStep {
                            macro_name: "cond".to_string(),
                            call_span: span,
                            depth,
                        });
                        return self.expand_cond(span, args, depth);
                    }

                    // P10-5: |> パイプラインマクロ -- (|> val f1 f2 f3)
                    // スレッディングに展開: (f3 (f2 (f1 val)))
                    if name == "|>" {
                        self.expansion_trace.push(MacroExpansionStep {
                            macro_name: "|>".to_string(),
                            call_span: span,
                            depth,
                        });
                        return self.expand_pipe_forward(span, args, depth);
                    }

                    if let Some(macro_def) = self.macros.get(name).cloned() {
                        // マクロ呼び出し
                        if args.len() != macro_def.params.len() {
                            return Err(MacroExpandError::ArityMismatch {
                                name: name.clone(),
                                expected: macro_def.params.len(),
                                actual: args.len(),
                                span,
                            });
                        }

                        // P10-3: 展開トレースに記録
                        self.expansion_trace.push(MacroExpansionStep {
                            macro_name: name.clone(),
                            call_span: span,
                            depth,
                        });

                        // 引数を展開済みにする
                        let expanded_args: Vec<Expr> = args
                            .into_iter()
                            .map(|a| self.expand_expr(a, depth + 1))
                            .collect::<Result<_, _>>()?;

                        // パラメータ → 引数のマッピング
                        let mut bindings = HashMap::new();
                        for (param, arg) in macro_def.params.iter().zip(expanded_args.iter()) {
                            bindings.insert(param.clone(), arg.clone());
                        }

                        // マクロ本体を展開
                        let expanded = self.substitute_expr(&macro_def.body, &bindings)?;

                        // 再帰展開 (マクロが別のマクロを呼ぶ場合)
                        return self.expand_expr(expanded, depth + 1);
                    }
                }

                // 通常の関数適用: 子要素を再帰展開
                let expanded_func = self.expand_expr(*func, depth + 1)?;
                let expanded_args: Vec<Expr> = args
                    .into_iter()
                    .map(|a| self.expand_expr(a, depth + 1))
                    .collect::<Result<_, _>>()?;
                Ok(Expr::App(span, Box::new(expanded_func), expanded_args))
            }

            // if 式
            Expr::If(span, cond, then_br, else_br) => {
                let c = self.expand_expr(*cond, depth + 1)?;
                let t = self.expand_expr(*then_br, depth + 1)?;
                let e = self.expand_expr(*else_br, depth + 1)?;
                Ok(Expr::If(span, Box::new(c), Box::new(t), Box::new(e)))
            }

            // let 束縛
            Expr::Let(span, bindings, body) => {
                let expanded_bindings: Vec<_> = bindings
                    .into_iter()
                    .map(|(pat, expr)| {
                        let e = self.expand_expr(expr, depth + 1)?;
                        Ok((pat, e))
                    })
                    .collect::<Result<_, MacroExpandError>>()?;
                let expanded_body = self.expand_expr(*body, depth + 1)?;
                Ok(Expr::Let(span, expanded_bindings, Box::new(expanded_body)))
            }

            // ラムダ
            Expr::Lambda(span, params, body) => {
                let expanded_body = self.expand_expr(*body, depth + 1)?;
                Ok(Expr::Lambda(span, params, Box::new(expanded_body)))
            }

            // do ブロック
            Expr::Do(span, exprs) => {
                let expanded: Vec<Expr> = exprs
                    .into_iter()
                    .map(|e| self.expand_expr(e, depth + 1))
                    .collect::<Result<_, _>>()?;
                Ok(Expr::Do(span, expanded))
            }

            // match 式
            Expr::Match(span, scrutinee, arms) => {
                use crate::ast::MatchArm;
                let expanded_scrutinee = self.expand_expr(*scrutinee, depth + 1)?;
                let expanded_arms: Vec<MatchArm> = arms
                    .into_iter()
                    .map(|arm| {
                        let body = self.expand_expr(arm.body, depth + 1)?;
                        Ok(MatchArm {
                            span: arm.span,
                            pattern: arm.pattern,
                            guard: arm.guard,
                            body,
                        })
                    })
                    .collect::<Result<_, MacroExpandError>>()?;
                Ok(Expr::Match(
                    span,
                    Box::new(expanded_scrutinee),
                    expanded_arms,
                ))
            }

            // Quote は展開しない (マクロ本体としてのみ使用)
            Expr::Quote(_, _) => Ok(expr),

            // Unquote/UnquoteSplice は quote 外では展開しない
            Expr::Unquote(_, _) | Expr::UnquoteSplice(_, _) => Ok(expr),

            // リテラル・変数・その他はそのまま
            Expr::Lit(_, _) | Expr::Var(_, _) => Ok(expr),

            // 型注釈
            Expr::Ann(span, inner, ty) => {
                let expanded = self.expand_expr(*inner, depth + 1)?;
                Ok(Expr::Ann(span, Box::new(expanded), ty))
            }

            // レコード系
            Expr::RecordLit(span, name, fields) => {
                let expanded_fields: Vec<_> = fields
                    .into_iter()
                    .map(|(n, e)| {
                        let expanded = self.expand_expr(e, depth + 1)?;
                        Ok((n, expanded))
                    })
                    .collect::<Result<_, MacroExpandError>>()?;
                Ok(Expr::RecordLit(span, name, expanded_fields))
            }

            Expr::FieldAccess(span, inner, field) => {
                let expanded = self.expand_expr(*inner, depth + 1)?;
                Ok(Expr::FieldAccess(span, Box::new(expanded), field))
            }

            Expr::RecordUpdate(span, inner, fields) => {
                let expanded_base = self.expand_expr(*inner, depth + 1)?;
                let expanded_fields: Vec<_> = fields
                    .into_iter()
                    .map(|(n, e)| {
                        let expanded = self.expand_expr(e, depth + 1)?;
                        Ok((n, expanded))
                    })
                    .collect::<Result<_, MacroExpandError>>()?;
                Ok(Expr::RecordUpdate(
                    span,
                    Box::new(expanded_base),
                    expanded_fields,
                ))
            }

            // P10-5: Computation Expression のマクロ展開 (脱糖)
            // ビルダーが登録されている場合: let!/do!/return を bind/return 関数呼び出しに変換
            // 未登録の場合: Computation ノードをそのまま残す (型推論で処理)
            Expr::Computation(span, builder, steps) => {
                use crate::ast::ComputationStep;
                if let Some((bind_fn, return_fn)) = self.computation_builders.get(&builder).cloned()
                {
                    // P10-5: Computation Expression を bind/return 関数呼び出しに脱糖
                    self.expansion_trace.push(MacroExpansionStep {
                        macro_name: format!("computation:{}", builder),
                        call_span: span,
                        depth,
                    });
                    self.desugar_computation(span, &bind_fn, &return_fn, steps, depth)
                } else {
                    // ビルダー未登録: 内部式のみ展開して Computation ノードを保持
                    let expanded_steps: Vec<_> = steps
                        .into_iter()
                        .map(|step| match step {
                            ComputationStep::LetBang(s, pat, e) => {
                                let expanded = self.expand_expr(e, depth + 1)?;
                                Ok(ComputationStep::LetBang(s, pat, expanded))
                            }
                            ComputationStep::DoBang(s, e) => {
                                let expanded = self.expand_expr(e, depth + 1)?;
                                Ok(ComputationStep::DoBang(s, expanded))
                            }
                            ComputationStep::Return(s, e) => {
                                let expanded = self.expand_expr(e, depth + 1)?;
                                Ok(ComputationStep::Return(s, expanded))
                            }
                            ComputationStep::Expr(e) => {
                                let expanded = self.expand_expr(e, depth + 1)?;
                                Ok(ComputationStep::Expr(expanded))
                            }
                        })
                        .collect::<Result<_, MacroExpandError>>()?;
                    Ok(Expr::Computation(span, builder, expanded_steps))
                }
            }
        }
    }

    /// Quote 式内の Unquote を引数に置換する
    fn substitute_expr(
        &self,
        expr: &Expr,
        bindings: &HashMap<String, Expr>,
    ) -> Result<Expr, MacroExpandError> {
        match expr {
            // Quote: 中身を再帰的に置換（ただし Quote 自体は除去して中身を返す）
            Expr::Quote(_, inner) => self.substitute_expr(inner, bindings),

            // Unquote: 引数に置換
            Expr::Unquote(span, inner) => {
                if let Expr::Var(_, name) = inner.as_ref() {
                    if let Some(replacement) = bindings.get(name) {
                        Ok(replacement.clone())
                    } else {
                        // 束縛にない変数はそのまま
                        Ok(Expr::Var(*span, name.clone()))
                    }
                } else {
                    // ~(expr) の場合はそのまま返す（将来的には評価が必要）
                    self.substitute_expr(inner, bindings)
                }
            }

            // UnquoteSplice はリストコンテキスト（App）内で処理
            Expr::UnquoteSplice(span, _) => {
                Err(MacroExpandError::SpliceOutsideList { span: *span })
            }

            // App: ~@ の splice を処理
            Expr::App(span, func, args) => {
                let substituted_func = self.substitute_expr(func, bindings)?;
                let mut substituted_args = Vec::new();

                for arg in args {
                    match arg {
                        Expr::UnquoteSplice(_, inner) => {
                            if let Expr::Var(_, name) = inner.as_ref()
                                && let Some(replacement) = bindings.get(name)
                            {
                                // リストを展開: App の引数をフラットに追加
                                if let Expr::App(_, _, splice_args) = replacement {
                                    substituted_args.extend(splice_args.clone());
                                } else {
                                    // 単一値の場合はそのまま追加
                                    substituted_args.push(replacement.clone());
                                }
                            }
                        }
                        other => {
                            substituted_args.push(self.substitute_expr(other, bindings)?);
                        }
                    }
                }

                Ok(Expr::App(
                    *span,
                    Box::new(substituted_func),
                    substituted_args,
                ))
            }

            // If
            Expr::If(span, cond, then_br, else_br) => {
                let c = self.substitute_expr(cond, bindings)?;
                let t = self.substitute_expr(then_br, bindings)?;
                let e = self.substitute_expr(else_br, bindings)?;
                Ok(Expr::If(*span, Box::new(c), Box::new(t), Box::new(e)))
            }

            // Let
            Expr::Let(span, let_bindings, body) => {
                let substituted_bindings: Vec<_> = let_bindings
                    .iter()
                    .map(|(pat, expr)| {
                        let e = self.substitute_expr(expr, bindings)?;
                        Ok((pat.clone(), e))
                    })
                    .collect::<Result<_, MacroExpandError>>()?;
                let substituted_body = self.substitute_expr(body, bindings)?;
                Ok(Expr::Let(
                    *span,
                    substituted_bindings,
                    Box::new(substituted_body),
                ))
            }

            // Lambda
            Expr::Lambda(span, params, body) => {
                let substituted_body = self.substitute_expr(body, bindings)?;
                Ok(Expr::Lambda(
                    *span,
                    params.clone(),
                    Box::new(substituted_body),
                ))
            }

            // Do
            Expr::Do(span, exprs) => {
                let substituted: Vec<Expr> = exprs
                    .iter()
                    .map(|e| self.substitute_expr(e, bindings))
                    .collect::<Result<_, _>>()?;
                Ok(Expr::Do(*span, substituted))
            }

            // Match
            Expr::Match(span, scrutinee, arms) => {
                use crate::ast::MatchArm;
                let substituted_scrutinee = self.substitute_expr(scrutinee, bindings)?;
                let substituted_arms: Vec<MatchArm> = arms
                    .iter()
                    .map(|arm| {
                        let body = self.substitute_expr(&arm.body, bindings)?;
                        Ok(MatchArm {
                            span: arm.span,
                            pattern: arm.pattern.clone(),
                            guard: arm.guard.clone(),
                            body,
                        })
                    })
                    .collect::<Result<_, MacroExpandError>>()?;
                Ok(Expr::Match(
                    *span,
                    Box::new(substituted_scrutinee),
                    substituted_arms,
                ))
            }

            // リテラル・変数はそのまま
            Expr::Lit(span, lit) => Ok(Expr::Lit(*span, lit.clone())),
            Expr::Var(span, name) => Ok(Expr::Var(*span, name.clone())),

            // 型注釈
            Expr::Ann(span, inner, ty) => {
                let substituted = self.substitute_expr(inner, bindings)?;
                Ok(Expr::Ann(*span, Box::new(substituted), ty.clone()))
            }

            // レコード系はそのまま (マクロ本体では通常使われない)
            Expr::RecordLit(span, name, fields) => {
                let substituted_fields: Vec<_> = fields
                    .iter()
                    .map(|(n, e)| {
                        let substituted = self.substitute_expr(e, bindings)?;
                        Ok((n.clone(), substituted))
                    })
                    .collect::<Result<_, MacroExpandError>>()?;
                Ok(Expr::RecordLit(*span, name.clone(), substituted_fields))
            }

            Expr::FieldAccess(span, inner, field) => {
                let substituted = self.substitute_expr(inner, bindings)?;
                Ok(Expr::FieldAccess(
                    *span,
                    Box::new(substituted),
                    field.clone(),
                ))
            }

            Expr::RecordUpdate(span, inner, fields) => {
                let substituted_base = self.substitute_expr(inner, bindings)?;
                let substituted_fields: Vec<_> = fields
                    .iter()
                    .map(|(n, e)| {
                        let substituted = self.substitute_expr(e, bindings)?;
                        Ok((n.clone(), substituted))
                    })
                    .collect::<Result<_, MacroExpandError>>()?;
                Ok(Expr::RecordUpdate(
                    *span,
                    Box::new(substituted_base),
                    substituted_fields,
                ))
            }

            // Computation
            Expr::Computation(span, builder, steps) => {
                use crate::ast::ComputationStep;
                let substituted_steps: Vec<_> = steps
                    .iter()
                    .map(|step| match step {
                        ComputationStep::LetBang(s, pat, e) => {
                            let substituted = self.substitute_expr(e, bindings)?;
                            Ok(ComputationStep::LetBang(*s, pat.clone(), substituted))
                        }
                        ComputationStep::DoBang(s, e) => {
                            let substituted = self.substitute_expr(e, bindings)?;
                            Ok(ComputationStep::DoBang(*s, substituted))
                        }
                        ComputationStep::Return(s, e) => {
                            let substituted = self.substitute_expr(e, bindings)?;
                            Ok(ComputationStep::Return(*s, substituted))
                        }
                        ComputationStep::Expr(e) => {
                            let substituted = self.substitute_expr(e, bindings)?;
                            Ok(ComputationStep::Expr(substituted))
                        }
                    })
                    .collect::<Result<_, MacroExpandError>>()?;
                Ok(Expr::Computation(*span, builder.clone(), substituted_steps))
            }
        }
    }
}

impl Default for MacroExpander {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ast::{Decl, Expr, Literal};
    use crate::lexer::Lexer;
    use crate::parser::Parser;

    fn parse(input: &str) -> Program {
        let mut lexer = Lexer::new(input);
        let tokens = lexer.tokenize().unwrap();
        let mut parser = Parser::new(tokens);
        parser.parse_program().unwrap()
    }

    #[test]
    fn test_expand_simple_macro() {
        let prog = parse(
            "(defmacro when [test body] '(if ~test ~body ()))\n\
             (defn f [x] (when (> x 0) (+ x 1)))",
        );

        let mut expander = MacroExpander::new();
        let expanded = expander.expand_program(prog).unwrap();

        assert_eq!(expanded.decls.len(), 1);

        if let Decl::Defn { body, .. } = &expanded.decls[0] {
            assert!(matches!(body, Expr::If(_, _, _, _)));
        } else {
            panic!("Expected Defn");
        }
    }

    #[test]
    fn test_defmacro_removed_from_output() {
        let prog = parse("(defmacro noop [x] '~x)");
        let mut expander = MacroExpander::new();
        let expanded = expander.expand_program(prog).unwrap();
        assert_eq!(expanded.decls.len(), 0);
    }

    #[test]
    fn test_macro_arity_mismatch() {
        let prog = parse(
            "(defmacro add2 [a b] '(+ ~a ~b))\n\
             (defn f [] (add2 1))",
        );
        let mut expander = MacroExpander::new();
        let result = expander.expand_program(prog);
        assert!(result.is_err());
        // P10-3: WithTrace でラップされることがある
        match result {
            Err(MacroExpandError::WithTrace { inner, .. }) => {
                if let MacroExpandError::ArityMismatch {
                    expected, actual, ..
                } = *inner
                {
                    assert_eq!(expected, 2);
                    assert_eq!(actual, 1);
                } else {
                    panic!("Expected ArityMismatch inside WithTrace");
                }
            }
            Err(MacroExpandError::ArityMismatch {
                expected, actual, ..
            }) => {
                assert_eq!(expected, 2);
                assert_eq!(actual, 1);
            }
            _ => panic!("Expected ArityMismatch"),
        }
    }

    #[test]
    fn test_identity_macro() {
        let prog = parse(
            "(defmacro id [x] '~x)\n\
             (defn f [] (id 42))",
        );
        let mut expander = MacroExpander::new();
        let expanded = expander.expand_program(prog).unwrap();
        assert_eq!(expanded.decls.len(), 1);
        if let Decl::Defn { body, .. } = &expanded.decls[0] {
            assert!(matches!(body, Expr::Lit(_, Literal::Int(42))));
        } else {
            panic!("Expected Defn");
        }
    }

    #[test]
    fn test_nested_macro_call() {
        let prog = parse(
            "(defmacro unless [test body] '(if ~test () ~body))\n\
             (defn f [x] (unless (> x 0) (- x 1)))",
        );
        let mut expander = MacroExpander::new();
        let expanded = expander.expand_program(prog).unwrap();

        if let Decl::Defn { body, .. } = &expanded.decls[0] {
            if let Expr::If(_, cond, then_br, else_br) = body {
                assert!(matches!(cond.as_ref(), Expr::App(_, _, _)));
                assert!(matches!(then_br.as_ref(), Expr::Lit(_, Literal::Unit)));
                assert!(matches!(else_br.as_ref(), Expr::App(_, _, _)));
            } else {
                panic!("Expected If, got {:?}", body);
            }
        }
    }

    #[test]
    fn test_gensym() {
        let mut expander = MacroExpander::new();
        let s1 = expander.gensym("tmp");
        let s2 = expander.gensym("tmp");
        assert_ne!(s1, s2);
        assert!(s1.starts_with("__gensym_tmp_"));
    }

    #[test]
    fn test_no_macro_passthrough() {
        let prog = parse("(defn add [x y] (+ x y))");
        let mut expander = MacroExpander::new();
        let expanded = expander.expand_program(prog).unwrap();
        assert_eq!(expanded.decls.len(), 1);
    }

    #[test]
    fn test_multiple_macros() {
        let prog = parse(
            "(defmacro when [test body] '(if ~test ~body ()))\n\
             (defmacro unless [test body] '(if ~test () ~body))\n\
             (defn f [x] (when (> x 0) 1))\n\
             (defn g [x] (unless (> x 0) 2))",
        );
        let mut expander = MacroExpander::new();
        let expanded = expander.expand_program(prog).unwrap();
        assert_eq!(expanded.decls.len(), 2);
    }

    // --- 組み込みマクロテスト ---

    #[test]
    fn test_builtin_when() {
        let prog = parse("(defn f [x] (when (> x 0) (+ x 1)))");
        let mut expander = MacroExpander::with_builtins();
        let expanded = expander.expand_program(prog).unwrap();
        assert_eq!(expanded.decls.len(), 1);
        if let Decl::Defn { body, .. } = &expanded.decls[0] {
            assert!(matches!(body, Expr::If(_, _, _, _)));
            if let Expr::If(_, _, _, else_br) = body {
                assert!(matches!(else_br.as_ref(), Expr::Lit(_, Literal::Unit)));
            }
        } else {
            panic!("Expected Defn");
        }
    }

    #[test]
    fn test_builtin_unless() {
        let prog = parse("(defn f [x] (unless (> x 0) (- x 1)))");
        let mut expander = MacroExpander::with_builtins();
        let expanded = expander.expand_program(prog).unwrap();
        assert_eq!(expanded.decls.len(), 1);
        if let Decl::Defn { body, .. } = &expanded.decls[0] {
            assert!(matches!(body, Expr::If(_, _, _, _)));
            if let Expr::If(_, _, then_br, _) = body {
                assert!(matches!(then_br.as_ref(), Expr::Lit(_, Literal::Unit)));
            }
        } else {
            panic!("Expected Defn");
        }
    }

    #[test]
    fn test_user_macro_overrides_builtin() {
        let prog = parse(
            "(defmacro when [test body] '(if ~test (+ ~body 100) ()))
             (defn f [x] (when (> x 0) x))",
        );
        let mut expander = MacroExpander::with_builtins();
        let expanded = expander.expand_program(prog).unwrap();
        assert_eq!(expanded.decls.len(), 1);
        if let Decl::Defn { body, .. } = &expanded.decls[0]
            && let Expr::If(_, _, then_br, _) = body
        {
            assert!(matches!(then_br.as_ref(), Expr::App(_, _, _)));
        }
    }

    // --- P10-3: 再帰マクロテスト ---

    #[test]
    fn test_recursive_macro_depth_limit() {
        let prog = parse(
            "(defmacro loop [x] '(loop ~x))
             (defn f [] (loop 1))",
        );
        let mut expander = MacroExpander::new();
        let result = expander.expand_program(prog);
        assert!(result.is_err());
        match &result {
            Err(MacroExpandError::RecursionLimit { limit, .. }) => {
                assert_eq!(*limit, 128);
            }
            Err(MacroExpandError::WithTrace { inner, .. }) => {
                if let MacroExpandError::RecursionLimit { limit, .. } = inner.as_ref() {
                    assert_eq!(*limit, 128);
                } else {
                    panic!("Expected RecursionLimit, got {:?}", inner);
                }
            }
            _ => panic!("Expected RecursionLimit, got {:?}", result),
        }
    }

    #[test]
    fn test_mutual_recursive_macros() {
        let prog = parse(
            "(defmacro mac-a [x] '(mac-b ~x))
             (defmacro mac-b [x] '(mac-a ~x))
             (defn f [] (mac-a 1))",
        );
        let mut expander = MacroExpander::new();
        let result = expander.expand_program(prog);
        assert!(result.is_err());
    }

    #[test]
    fn test_finite_recursive_macro() {
        let prog = parse(
            "(defmacro double [x] '(+ ~x ~x))
             (defmacro quad [x] '(double (double ~x)))
             (defn f [] (quad 5))",
        );
        let mut expander = MacroExpander::new();
        let expanded = expander.expand_program(prog).unwrap();
        assert_eq!(expanded.decls.len(), 1);
        if let Decl::Defn { body, .. } = &expanded.decls[0] {
            assert!(matches!(body, Expr::App(_, _, _)));
        }
    }

    // --- P10-3: ~@ splice 展開テスト ---

    #[test]
    fn test_splice_in_apply() {
        let prog = parse(
            "(defmacro wrap [f a b] '(~f ~a ~b))
             (defn test [] (wrap + 1 2))",
        );
        let mut expander = MacroExpander::new();
        let expanded = expander.expand_program(prog).unwrap();
        assert_eq!(expanded.decls.len(), 1);
        if let Decl::Defn { body, .. } = &expanded.decls[0] {
            if let Expr::App(_, func, args) = body {
                assert!(matches!(func.as_ref(), Expr::Var(_, _)));
                assert_eq!(args.len(), 2);
            } else {
                panic!("Expected App, got {:?}", body);
            }
        }
    }

    // --- P10-3: 型シグネチャテスト ---

    #[test]
    fn test_macro_type_sig_stored() {
        // (defmacro typed-when [test body] : (-> Bool Int Int) '(if ~test ~body ()))
        let prog =
            parse("(defmacro typed-when [test body] : (-> Bool Int Int) '(if ~test ~body ()))");
        let mut expander = MacroExpander::new();
        let _expanded = expander.expand_program(prog).unwrap();
        // 型シグネチャが保存されていることを確認
        let sig = expander.macro_type_sig("typed-when");
        assert!(sig.is_some(), "型シグネチャが保存されているべき");
    }

    #[test]
    fn test_macro_without_type_sig() {
        let prog = parse("(defmacro noop [x] '~x)");
        let mut expander = MacroExpander::new();
        let _expanded = expander.expand_program(prog).unwrap();
        let sig = expander.macro_type_sig("noop");
        assert!(sig.is_none(), "型シグネチャなしの場合は None");
    }

    // --- P10-3: 展開トレースバックテスト ---

    #[test]
    fn test_expansion_trace_recorded() {
        let prog = parse(
            "(defmacro double [x] '(+ ~x ~x))
             (defn f [] (double 5))",
        );
        let mut expander = MacroExpander::new();
        let _expanded = expander.expand_program(prog).unwrap();
        let trace = expander.expansion_trace();
        assert_eq!(trace.len(), 1);
        assert_eq!(trace[0].macro_name, "double");
        assert_eq!(trace[0].depth, 0);
    }

    #[test]
    fn test_nested_expansion_trace() {
        let prog = parse(
            "(defmacro double [x] '(+ ~x ~x))
             (defmacro quad [x] '(double (double ~x)))
             (defn f [] (quad 5))",
        );
        let mut expander = MacroExpander::new();
        let _expanded = expander.expand_program(prog).unwrap();
        let trace = expander.expansion_trace();
        // quad -> double -> double の3段階
        assert!(
            trace.len() >= 2,
            "トレースは少なくとも2エントリ: {:?}",
            trace
        );
        assert_eq!(trace[0].macro_name, "quad");
    }

    #[test]
    fn test_builtin_assert() {
        let prog = parse("(defn f [x] (assert (> x 0)))");
        let mut expander = MacroExpander::with_builtins();
        let expanded = expander.expand_program(prog).unwrap();
        assert_eq!(expanded.decls.len(), 1);
        if let Decl::Defn { body, .. } = &expanded.decls[0] {
            // assert は if に展開される
            assert!(matches!(body, Expr::If(_, _, _, _)));
            if let Expr::If(_, _, then_br, else_br) = body {
                // then: ()
                assert!(matches!(then_br.as_ref(), Expr::Lit(_, Literal::Unit)));
                // else: (do (print "Assertion failed") 0)
                assert!(matches!(else_br.as_ref(), Expr::Do(_, _)));
            }
        } else {
            panic!("Expected Defn");
        }
    }

    #[test]
    fn test_traceback_format() {
        let err = MacroExpandError::RecursionLimit {
            name: "loop".to_string(),
            limit: 128,
            span: Span::new(0, 10),
        };
        let trace = vec![
            MacroExpansionStep {
                macro_name: "loop".to_string(),
                call_span: Span::new(0, 10),
                depth: 0,
            },
            MacroExpansionStep {
                macro_name: "loop".to_string(),
                call_span: Span::new(0, 10),
                depth: 1,
            },
        ];
        let with_trace = err.with_trace(trace);
        let formatted = with_trace.format_traceback();
        assert!(
            formatted.contains("トレースバック"),
            "フォーマットにトレースバックが含まれるべき: {formatted}"
        );
        assert!(
            formatted.contains("loop"),
            "マクロ名が含まれるべき: {formatted}"
        );
    }
}

#[cfg(test)]
mod tests_p10_5 {
    use super::*;
    use crate::ast::{Decl, Expr, Literal};
    use crate::lexer::Lexer;
    use crate::parser::Parser;

    fn parse(input: &str) -> Program {
        let mut lexer = Lexer::new(input);
        let tokens = lexer.tokenize().unwrap();
        let mut parser = Parser::new(tokens);
        parser.parse_program().unwrap()
    }

    // --- P10-5: cond マクロテスト ---

    #[test]
    fn test_builtin_cond_two_branches() {
        // (cond (> x 0) 1 (< x 0) -1 0) -> (if (> x 0) 1 (if (< x 0) -1 0))
        let prog = parse("(defn f [x] (cond (> x 0) 1 (< x 0) (- 0 1) 0))");
        let mut expander = MacroExpander::with_builtins();
        let expanded = expander.expand_program(prog).unwrap();
        assert_eq!(expanded.decls.len(), 1);
        if let Decl::Defn { body, .. } = &expanded.decls[0] {
            // 外側の if
            if let Expr::If(_, _, _, else_br) = body {
                // 内側も if
                assert!(
                    matches!(else_br.as_ref(), Expr::If(_, _, _, _)),
                    "cond の else 分岐が if に展開されるべき: {:?}",
                    else_br
                );
            } else {
                panic!("Expected If, got {:?}", body);
            }
        } else {
            panic!("Expected Defn");
        }
    }

    #[test]
    fn test_builtin_cond_single_branch() {
        // (cond true 42 0) -> (if true 42 0)
        let prog = parse("(defn f [] (cond true 42 0))");
        let mut expander = MacroExpander::with_builtins();
        let expanded = expander.expand_program(prog).unwrap();
        assert_eq!(expanded.decls.len(), 1);
        if let Decl::Defn { body, .. } = &expanded.decls[0] {
            assert!(
                matches!(body, Expr::If(_, _, _, _)),
                "cond は if に展開されるべき: {:?}",
                body
            );
        }
    }

    #[test]
    fn test_builtin_cond_default_only() {
        // (cond 42) -> 42
        let prog = parse("(defn f [] (cond 42))");
        let mut expander = MacroExpander::with_builtins();
        let expanded = expander.expand_program(prog).unwrap();
        assert_eq!(expanded.decls.len(), 1);
        if let Decl::Defn { body, .. } = &expanded.decls[0] {
            assert!(
                matches!(body, Expr::Lit(_, Literal::Int(42))),
                "cond のデフォルト値のみの場合はそのまま返すべき: {:?}",
                body
            );
        }
    }

    // --- P10-5: |> パイプラインマクロテスト ---

    #[test]
    fn test_builtin_pipe_forward_single() {
        // (|> 42 print) -> (print 42)
        let prog = parse("(defn f [] (|> 42 print))");
        let mut expander = MacroExpander::with_builtins();
        let expanded = expander.expand_program(prog).unwrap();
        assert_eq!(expanded.decls.len(), 1);
        if let Decl::Defn { body, .. } = &expanded.decls[0] {
            if let Expr::App(_, func, args) = body {
                assert!(
                    matches!(func.as_ref(), Expr::Var(_, name) if name == "print"),
                    "pipe の関数が print であるべき: {:?}",
                    func
                );
                assert_eq!(args.len(), 1);
                assert!(
                    matches!(&args[0], Expr::Lit(_, Literal::Int(42))),
                    "pipe の引数が 42 であるべき: {:?}",
                    args
                );
            } else {
                panic!("Expected App, got {:?}", body);
            }
        }
    }

    #[test]
    fn test_builtin_pipe_forward_chain() {
        // (|> 1 (+ 2) (+ 3)) -> (+ 3 (+ 2 1))
        // つまり (+ (+ 1 2) 3) のようなネスト
        let prog = parse("(defn f [] (|> 1 (+ 2) (+ 3)))");
        let mut expander = MacroExpander::with_builtins();
        let expanded = expander.expand_program(prog).unwrap();
        assert_eq!(expanded.decls.len(), 1);
        if let Decl::Defn { body, .. } = &expanded.decls[0] {
            // 最外の App: (+ 3 ...)
            if let Expr::App(_, func, args) = body {
                assert!(
                    matches!(func.as_ref(), Expr::Var(_, name) if name == "+"),
                    "最外の関数が + であるべき: {:?}",
                    func
                );
                // 引数は [2, (+ 2 1)] の2つ (部分適用 + パイプ引数)
                assert_eq!(args.len(), 2, "引数が2つあるべき: {:?}", args);
            } else {
                panic!("Expected App, got {:?}", body);
            }
        }
    }

    #[test]
    fn test_builtin_pipe_forward_value_only() {
        // (|> 42) -> 42
        let prog = parse("(defn f [] (|> 42))");
        let mut expander = MacroExpander::with_builtins();
        let expanded = expander.expand_program(prog).unwrap();
        assert_eq!(expanded.decls.len(), 1);
        if let Decl::Defn { body, .. } = &expanded.decls[0] {
            assert!(
                matches!(body, Expr::Lit(_, Literal::Int(42))),
                "値のみの場合はそのまま返すべき: {:?}",
                body
            );
        }
    }

    #[test]
    fn test_cond_trace_recorded() {
        let prog = parse("(defn f [x] (cond (> x 0) 1 0))");
        let mut expander = MacroExpander::with_builtins();
        let _expanded = expander.expand_program(prog).unwrap();
        let trace = expander.expansion_trace();
        assert!(!trace.is_empty(), "cond 展開のトレースが記録されるべき");
        assert_eq!(trace[0].macro_name, "cond");
    }

    #[test]
    fn test_pipe_trace_recorded() {
        let prog = parse("(defn f [] (|> 42 print))");
        let mut expander = MacroExpander::with_builtins();
        let _expanded = expander.expand_program(prog).unwrap();
        let trace = expander.expansion_trace();
        assert!(!trace.is_empty(), "|> 展開のトレースが記録されるべき");
        assert_eq!(trace[0].macro_name, "|>");
    }
}

#[cfg(test)]
mod tests_computation_macro {
    use super::*;
    use crate::ast::{Decl, Expr};
    use crate::lexer::Lexer;
    use crate::parser::Parser;

    fn parse(input: &str) -> Program {
        let mut lexer = Lexer::new(input);
        let tokens = lexer.tokenize().unwrap();
        let mut parser = Parser::new(tokens);
        parser.parse_program().unwrap()
    }

    // --- P10-5: Computation Expression マクロ化テスト ---

    #[test]
    fn test_computation_return_desugared() {
        // (computation maybe (return 42)) => (maybe-return 42)
        let prog = parse(
            "(computation-builder maybe maybe-bind maybe-return)\n\
             (defn test [] (computation maybe (return 42)))",
        );
        let mut expander = MacroExpander::with_builtins();
        let expanded = expander.expand_program(prog).unwrap();
        // computation-builder + defn = 2 decls
        assert_eq!(expanded.decls.len(), 2);
        if let Decl::Defn { body, .. } = &expanded.decls[1] {
            // (maybe-return 42) に脱糖されるべき
            if let Expr::App(_, func, args) = body {
                assert!(
                    matches!(func.as_ref(), Expr::Var(_, name) if name == "maybe-return"),
                    "return は maybe-return に脱糖されるべき: {:?}",
                    func
                );
                assert_eq!(args.len(), 1);
            } else {
                panic!("Expected App (maybe-return 42), got {:?}", body);
            }
        } else {
            panic!("Expected Defn");
        }
    }

    #[test]
    fn test_computation_let_bang_desugared() {
        // (computation maybe (let! x (get-value)) (return x))
        //   => (maybe-bind (get-value) (fn [x] (maybe-return x)))
        let prog = parse(
            "(computation-builder maybe maybe-bind maybe-return)\n\
             (defn test [] (computation maybe (let! x (get-value)) (return x)))",
        );
        let mut expander = MacroExpander::with_builtins();
        let expanded = expander.expand_program(prog).unwrap();
        assert_eq!(expanded.decls.len(), 2);
        if let Decl::Defn { body, .. } = &expanded.decls[1] {
            // (maybe-bind (get-value) (fn [x] (maybe-return x)))
            if let Expr::App(_, func, args) = body {
                assert!(
                    matches!(func.as_ref(), Expr::Var(_, name) if name == "maybe-bind"),
                    "let! は maybe-bind に脱糖されるべき: {:?}",
                    func
                );
                assert_eq!(args.len(), 2, "bind は2引数であるべき");
                // 第2引数が Lambda であること
                assert!(
                    matches!(&args[1], Expr::Lambda(_, params, _) if params.len() == 1),
                    "bind の第2引数は Lambda であるべき: {:?}",
                    args[1]
                );
            } else {
                panic!("Expected App (maybe-bind ...), got {:?}", body);
            }
        } else {
            panic!("Expected Defn");
        }
    }

    #[test]
    fn test_computation_do_bang_desugared() {
        // (computation maybe (do! (side-effect)) (return 42))
        //   => (maybe-bind (side-effect) (fn [_] (maybe-return 42)))
        let prog = parse(
            "(computation-builder maybe maybe-bind maybe-return)\n\
             (defn test [] (computation maybe (do! (side-effect)) (return 42)))",
        );
        let mut expander = MacroExpander::with_builtins();
        let expanded = expander.expand_program(prog).unwrap();
        assert_eq!(expanded.decls.len(), 2);
        if let Decl::Defn { body, .. } = &expanded.decls[1] {
            if let Expr::App(_, func, args) = body {
                assert!(
                    matches!(func.as_ref(), Expr::Var(_, name) if name == "maybe-bind"),
                    "do! は maybe-bind に脱糖されるべき: {:?}",
                    func
                );
                assert_eq!(args.len(), 2);
                // 第2引数の Lambda パラメータ名が gensym (無視用) であること
                if let Expr::Lambda(_, params, _) = &args[1] {
                    assert!(
                        params[0].name.starts_with("__gensym_"),
                        "do! の Lambda パラメータは gensym であるべき: {}",
                        params[0].name
                    );
                }
            } else {
                panic!("Expected App, got {:?}", body);
            }
        }
    }

    #[test]
    fn test_computation_chain_desugared() {
        // (computation maybe (let! x (get-value)) (let! y (process x)) (return y))
        //   => (maybe-bind (get-value) (fn [x] (maybe-bind (process x) (fn [y] (maybe-return y)))))
        let prog = parse(
            "(computation-builder maybe maybe-bind maybe-return)\n\
             (defn test [] (computation maybe (let! x (get-value)) (let! y (process x)) (return y)))",
        );
        let mut expander = MacroExpander::with_builtins();
        let expanded = expander.expand_program(prog).unwrap();
        assert_eq!(expanded.decls.len(), 2);
        if let Decl::Defn { body, .. } = &expanded.decls[1] {
            // 外側: (maybe-bind (get-value) (fn [x] ...))
            if let Expr::App(_, func, args) = body {
                assert!(matches!(func.as_ref(), Expr::Var(_, name) if name == "maybe-bind"));
                // 内側 Lambda 内: (maybe-bind (process x) (fn [y] (maybe-return y)))
                if let Expr::Lambda(_, _, inner_body) = &args[1] {
                    if let Expr::App(_, inner_func, _) = inner_body.as_ref() {
                        assert!(
                            matches!(inner_func.as_ref(), Expr::Var(_, name) if name == "maybe-bind"),
                            "チェーンの内側も maybe-bind であるべき: {:?}",
                            inner_func
                        );
                    } else {
                        panic!("Expected inner App, got {:?}", inner_body);
                    }
                }
            } else {
                panic!("Expected App, got {:?}", body);
            }
        }
    }

    #[test]
    fn test_computation_trace_recorded() {
        let prog = parse(
            "(computation-builder maybe maybe-bind maybe-return)\n\
             (defn test [] (computation maybe (return 42)))",
        );
        let mut expander = MacroExpander::with_builtins();
        let _expanded = expander.expand_program(prog).unwrap();
        let trace = expander.expansion_trace();
        assert!(
            !trace.is_empty(),
            "computation 展開のトレースが記録されるべき"
        );
        assert_eq!(trace[0].macro_name, "computation:maybe");
    }

    #[test]
    fn test_computation_without_builder_preserved() {
        // ビルダー未登録の場合は Computation ノードをそのまま残す
        let prog = parse("(defn test [] (computation unknown (return 42)))");
        let mut expander = MacroExpander::with_builtins();
        let expanded = expander.expand_program(prog).unwrap();
        assert_eq!(expanded.decls.len(), 1);
        if let Decl::Defn { body, .. } = &expanded.decls[0] {
            assert!(
                matches!(body, Expr::Computation(_, name, _) if name == "unknown"),
                "未登録ビルダーの場合は Computation を保持: {:?}",
                body
            );
        }
    }
}
