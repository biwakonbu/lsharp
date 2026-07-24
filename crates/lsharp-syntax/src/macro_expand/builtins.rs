use super::{MacroDef, MacroExpandError, MacroExpander};
use crate::ast::Expr;
use crate::span::Span;

impl MacroExpander {
    /// 組み込みマクロを登録
    /// - when: (when test body) -> (if test body ())
    /// - unless: (unless test body) -> (if test () body)
    /// - assert: (assert expr) -> (if expr () (do (print "Assertion failed") 0))
    pub(super) fn register_builtins(&mut self) {
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
    pub(super) fn expand_cond(
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
    pub(super) fn expand_pipe_forward(
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
    pub(super) fn desugar_computation(
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
}
