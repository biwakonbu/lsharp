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
    /// 利用者向けの安定した診断コードを返す。
    pub fn code(&self) -> &'static str {
        "LS0201"
    }

    /// 診断に対応する source span を返す。
    pub fn span(&self) -> Option<Span> {
        match self {
            Self::UndefinedMacro { span, .. }
            | Self::ArityMismatch { span, .. }
            | Self::RecursionLimit { span, .. }
            | Self::SpliceOutsideList { span } => Some(*span),
            Self::WithTrace { inner, .. } => inner.span(),
        }
    }

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
mod tests;

#[cfg(test)]
mod tests_p10_5;

#[cfg(test)]
mod tests_computation_macro;
