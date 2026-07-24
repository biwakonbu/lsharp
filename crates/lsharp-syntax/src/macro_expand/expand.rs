use std::collections::HashMap;

use super::{MacroExpandError, MacroExpander, MacroExpansionStep};
use crate::ast::Expr;

impl MacroExpander {
    /// 式のマクロ展開 (再帰)
    pub(super) fn expand_expr(
        &mut self,
        expr: Expr,
        depth: usize,
    ) -> Result<Expr, MacroExpandError> {
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
    pub(super) fn substitute_expr(
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
