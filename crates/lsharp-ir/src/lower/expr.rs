//! 式の lowering (lower_expr 関連)

use std::collections::HashMap;

use lsharp_syntax::ast::*;

use crate::{Instruction, IrType};

use super::{is_builtin_binop, FuncCtx, Lower, LowerError};

impl Lower {
    /// 式を IR 命令に変換（スタックマシン方式）
    pub(crate) fn lower_expr(&self, ctx: &mut FuncCtx, expr: &Expr) -> Result<(), LowerError> {
        match expr {
            Expr::Lit(_, lit) => match lit {
                Literal::Int(n) => ctx.emit(Instruction::I64Const(*n)),
                Literal::Float(n) => ctx.emit(Instruction::F64Const(*n)),
                Literal::Bool(b) => ctx.emit(Instruction::I64Const(if *b { 1 } else { 0 })),
                Literal::String(s) => {
                    // 文字列リテラル: データセクションに格納し、(offset << 32 | len) でエンコード
                    let bytes = s.as_bytes().to_vec();
                    let len = bytes.len() as u32;
                    let offset = self.string_offset.get();
                    let label = format!("$str{}", self.string_data.borrow().len());
                    self.string_data.borrow_mut().push((label, bytes));
                    self.string_offset.set(offset + len);
                    // offset << 32 | len として i64 にパック
                    let packed = ((offset as i64) << 32) | (len as i64);
                    ctx.emit(Instruction::I64Const(packed));
                }
                Literal::Unit => ctx.emit(Instruction::I64Const(0)),
            },

            Expr::Var(_, name) => {
                if let Some(&idx) = ctx.locals_map.get(name) {
                    ctx.emit(Instruction::LocalGet(idx));
                } else if let Some(&func_idx) = self.func_indices.get(name) {
                    // 引数なし ADT コンストラクタ（または引数なし関数）を呼び出し
                    ctx.emit(Instruction::Call(func_idx));
                } else {
                    return Err(LowerError::UndefinedFunction {
                        name: name.clone(),
                    });
                }
            }

            Expr::If(_, cond, then, else_) => {
                // 条件式
                self.lower_expr(ctx, cond)?;
                // Bool (i64) -> i32 に変換
                ctx.emit(Instruction::I32WrapI64);
                // if-then-else
                ctx.emit(Instruction::If(IrType::I64));
                self.lower_expr(ctx, then)?;
                ctx.emit(Instruction::Else);
                self.lower_expr(ctx, else_)?;
                ctx.emit(Instruction::End);
            }

            Expr::Let(_, bindings, body) => {
                for (pat, val) in bindings {
                    self.lower_expr(ctx, val)?;
                    match pat {
                        Pattern::Var(_, name) => {
                            let idx = ctx.alloc_local(name.clone());
                            ctx.emit(Instruction::LocalSet(idx));
                        }
                        Pattern::Wildcard(_) => {
                            ctx.emit(Instruction::Drop);
                        }
                        _ => {
                            // MVP: 複雑なパターンは未サポート
                            let idx = ctx.alloc_local("_pat".to_string());
                            ctx.emit(Instruction::LocalSet(idx));
                        }
                    }
                }
                self.lower_expr(ctx, body)?;
            }

            Expr::App(_, func, args) => {
                match func.as_ref() {
                    // and/or 論理演算子（i64 -> i32 変換が必要）
                    Expr::Var(_, op) if (op == "and" || op == "or") && args.len() == 2 => {
                        // 左オペランド: i64 -> i32
                        self.lower_expr(ctx, &args[0])?;
                        ctx.emit(Instruction::I32WrapI64);
                        // 右オペランド: i64 -> i32
                        self.lower_expr(ctx, &args[1])?;
                        ctx.emit(Instruction::I32WrapI64);
                        // i32 レベルで and/or
                        if op == "and" {
                            ctx.emit(Instruction::I32And);
                        } else {
                            ctx.emit(Instruction::I32Or);
                        }
                        // 結果を i64 に拡張
                        ctx.emit(Instruction::I64ExtendI32S);
                    }
                    // 組み込み二項演算子
                    Expr::Var(_, op) if is_builtin_binop(op) && args.len() == 2 => {
                        self.lower_expr(ctx, &args[0])?;
                        self.lower_expr(ctx, &args[1])?;
                        self.emit_binop(ctx, op)?;
                    }
                    // not 演算子
                    Expr::Var(_, op) if op == "not" && args.len() == 1 => {
                        self.lower_expr(ctx, &args[0])?;
                        ctx.emit(Instruction::I64Const(0));
                        ctx.emit(Instruction::I64Eq);
                        ctx.emit(Instruction::I64ExtendI32S);
                    }
                    // print 関数
                    Expr::Var(_, name) if name == "print" => {
                        if let Some(arg) = args.first() {
                            self.lower_expr(ctx, arg)?;
                        }
                        let idx = *self.func_indices.get("print").ok_or_else(|| {
                            LowerError::UndefinedFunction { name: "print".to_string() }
                        })?;
                        ctx.emit(Instruction::Call(idx));
                        // print は Unit を返す
                        ctx.emit(Instruction::I64Const(0));
                    }
                    // __alloc 関数 (Bump Allocator)
                    Expr::Var(_, name) if name == "__alloc" => {
                        if let Some(arg) = args.first() {
                            self.lower_expr(ctx, arg)?;
                        }
                        let idx = *self.func_indices.get("__alloc").ok_or_else(|| {
                            LowerError::UndefinedFunction { name: "__alloc".to_string() }
                        })?;
                        ctx.emit(Instruction::Call(idx));
                    }
                    // TypeName.field アクセサ呼び出し
                    Expr::Var(_, name) if name.contains('.') && name.starts_with(|c: char| c.is_ascii_uppercase()) => {
                        // 引数（レコード）を評価
                        for arg in args {
                            self.lower_expr(ctx, arg)?;
                        }
                        if let Some(&idx) = self.func_indices.get(name.as_str()) {
                            ctx.emit(Instruction::Call(idx));
                        } else {
                            return Err(LowerError::UndefinedFunction {
                                name: name.clone(),
                            });
                        }
                    }
                    // ユーザー定義関数呼び出し（トレイト静的ディスパッチ対応）
                    Expr::Var(_, name) => {
                        // 引数を評価
                        for arg in args {
                            self.lower_expr(ctx, arg)?;
                        }
                        if let Some(&idx) = self.func_indices.get(name.as_str()) {
                            ctx.emit(Instruction::Call(idx));
                        } else if let Some(idx) = self.resolve_trait_dispatch(name, args) {
                            // P5-6: トレイトメソッドの静的ディスパッチ自動解決
                            ctx.emit(Instruction::Call(idx));
                        } else {
                            return Err(LowerError::UndefinedFunction {
                                name: name.clone(),
                            });
                        }
                    }
                    _ => {
                        return Err(LowerError::Unsupported {
                            msg: "間接的な関数呼び出し".to_string(),
                        });
                    }
                }
            }

            Expr::Match(_, scrutinee, arms) => {
                // MVP: 簡易パターンマッチ（ADT なし、リテラル/変数のみ）
                // scrutinee を評価してローカルに保存
                self.lower_expr(ctx, scrutinee)?;
                let scrut_local = ctx.alloc_local("_match".to_string());
                ctx.emit(Instruction::LocalSet(scrut_local));

                // ネストした if-else チェインで変換
                self.lower_match_arms(ctx, scrut_local, arms, 0)?;
            }

            Expr::Do(_, exprs) => {
                for (i, expr) in exprs.iter().enumerate() {
                    self.lower_expr(ctx, expr)?;
                    // 最後の式以外は結果を捨てる
                    if i < exprs.len() - 1 {
                        ctx.emit(Instruction::Drop);
                    }
                }
                if exprs.is_empty() {
                    ctx.emit(Instruction::I64Const(0)); // unit
                }
            }

            Expr::Lambda(_, _, _) => {
                // MVP: ラムダ（クロージャ）は未サポート
                return Err(LowerError::Unsupported {
                    msg: "ラムダ式（クロージャ）".to_string(),
                });
            }

            Expr::Ann(_, expr, _) => {
                // 型注釈は無視して中身を変換
                self.lower_expr(ctx, expr)?;
            }

            Expr::RecordLit(_, type_name, fields) => {
                if let Some(&gc_type_idx) = self.record_type_indices.get(type_name) {
                    // レコード定義のフィールド順序に従って値をスタックに積む
                    if let Some(field_order) = self.record_fields.get(type_name) {
                        let field_map: HashMap<&str, &Expr> = fields
                            .iter()
                            .map(|(n, e)| (n.as_str(), e))
                            .collect();
                        for field_name in field_order {
                            if let Some(expr) = field_map.get(field_name.as_str()) {
                                self.lower_expr(ctx, expr)?;
                            } else {
                                // フィールドが見つからない場合はデフォルト値
                                ctx.emit(Instruction::I64Const(0));
                            }
                        }
                    } else {
                        // フィールド順序不明の場合は指定順に積む
                        for (_, field_expr) in fields {
                            self.lower_expr(ctx, field_expr)?;
                        }
                    }
                    ctx.emit(Instruction::StructNew(gc_type_idx));
                } else {
                    // GC 型が見つからない場合はフォールバック
                    if let Some((_, first_field)) = fields.first() {
                        self.lower_expr(ctx, first_field)?;
                    } else {
                        ctx.emit(Instruction::I64Const(0));
                    }
                }
            }

            Expr::FieldAccess(_, expr, field_name) => {
                // 式を評価してスタックにレコード値を積む
                self.lower_expr(ctx, expr)?;

                // 型推論結果から型名を取得して正確にフィールドを解決 (R-M5)
                let type_name_hint = self.infer_expr_type_name(expr);
                let mut resolved = false;

                if let Some(ref tn) = type_name_hint {
                    // 型名が判明: 正確に解決
                    if let Some(fields) = self.record_fields.get(tn) {
                        if let Some(field_idx) = fields.iter().position(|f| f == field_name) {
                            if let Some(&gc_type_idx) = self.record_type_indices.get(tn) {
                                ctx.emit(Instruction::StructGet(gc_type_idx, field_idx as u32));
                                resolved = true;
                            }
                        } else {
                            return Err(LowerError::Unsupported {
                                msg: format!("レコード型 '{tn}' にフィールド '{field_name}' が存在しません"),
                            });
                        }
                    }
                }

                if !resolved {
                    // フォールバック: フィールド名で全レコード型を走査
                    for (type_name, fields) in &self.record_fields {
                        if let Some(field_idx) = fields.iter().position(|f| f == field_name) {
                            if let Some(&gc_type_idx) = self.record_type_indices.get(type_name) {
                                ctx.emit(Instruction::StructGet(gc_type_idx, field_idx as u32));
                                resolved = true;
                                break;
                            }
                        }
                    }
                }

                if !resolved {
                    return Err(LowerError::Unsupported {
                        msg: format!("フィールド '{field_name}' を解決できません"),
                    });
                }
            }

            Expr::RecordUpdate(_, base, update_fields) => {
                // ベースレコードを評価してローカルに保存
                self.lower_expr(ctx, base)?;
                let base_local = ctx.alloc_local("_record_base".to_string());
                ctx.emit(Instruction::LocalSet(base_local));

                // 型推論結果からベース式の型名を取得 (R-m3)
                let type_name_hint = self.infer_expr_type_name(base);
                let mut found_type = None;

                if let Some(ref tn) = type_name_hint {
                    // 型名が判明: 正確に解決
                    if let Some(fields) = self.record_fields.get(tn) {
                        found_type = Some((tn.clone(), fields.clone()));
                    }
                }

                if found_type.is_none() {
                    // フォールバック: フィールド名で全レコード型を走査
                    for (type_name, fields) in &self.record_fields {
                        let all_match = update_fields.iter().all(|(n, _)| fields.contains(n));
                        if all_match {
                            found_type = Some((type_name.clone(), fields.clone()));
                            break;
                        }
                    }
                }

                if let Some((type_name, field_order)) = found_type {
                    if let Some(&gc_type_idx) = self.record_type_indices.get(&type_name) {
                        let update_map: HashMap<&str, &Expr> = update_fields
                            .iter()
                            .map(|(n, e)| (n.as_str(), e))
                            .collect();
                        // 各フィールドについて、更新値があればそれを、なければベースから取得
                        for (field_idx, field_name) in field_order.iter().enumerate() {
                            if let Some(expr) = update_map.get(field_name.as_str()) {
                                self.lower_expr(ctx, expr)?;
                            } else {
                                ctx.emit(Instruction::LocalGet(base_local));
                                ctx.emit(Instruction::StructGet(gc_type_idx, field_idx as u32));
                            }
                        }
                        ctx.emit(Instruction::StructNew(gc_type_idx));
                    } else {
                        ctx.emit(Instruction::LocalGet(base_local));
                    }
                } else {
                    // フォールバック: ベースをそのまま返す
                    ctx.emit(Instruction::LocalGet(base_local));
                }
            }
            Expr::Computation(_, builder_name, steps) => {
                // Computation Expression: bind/return 関数呼び出しに脱糖
                let builder_info = self.computation_builders.get(builder_name).cloned();

                for (i, step) in steps.iter().enumerate() {
                    match step {
                        ComputationStep::LetBang(_, pat, expr) => {
                            // let! x = expr -> bind(expr, fn [x] rest)
                            // MVP: bind 関数を呼び出す（簡易版: 式を評価してローカルに格納）
                            self.lower_expr(ctx, expr)?;
                            if let Some((ref bind_fn, _)) = builder_info {
                                if let Some(&idx) = self.func_indices.get(bind_fn.as_str()) {
                                    // bind 関数の第1引数（モナド値）は既にスタック上
                                    // 残りのステップは後続で評価される
                                    // MVP: 式の結果をそのまま変数に束縛
                                    let _ = idx; // 将来的に bind 呼び出しに使用
                                }
                            }
                            // パターン変数をローカルに格納
                            if let Pattern::Var(_, var_name) = pat {
                                let var_local = ctx.alloc_local(var_name.clone());
                                ctx.emit(Instruction::LocalSet(var_local));
                            }
                        }
                        ComputationStep::DoBang(_, expr) => {
                            // do! expr -> bind(expr, fn [_] rest)
                            self.lower_expr(ctx, expr)?;
                            // 結果を捨てる（最後のステップでなければ）
                            if i < steps.len() - 1 {
                                ctx.emit(Instruction::Drop);
                            }
                        }
                        ComputationStep::Return(_, expr) => {
                            // return expr -> return_fn(expr)
                            self.lower_expr(ctx, expr)?;
                            if let Some((_, ref return_fn)) = builder_info {
                                if let Some(&idx) = self.func_indices.get(return_fn.as_str()) {
                                    ctx.emit(Instruction::Call(idx));
                                }
                            }
                        }
                        ComputationStep::Expr(expr) => {
                            self.lower_expr(ctx, expr)?;
                            // 中間式の結果を捨てる（最後のステップでなければ）
                            if i < steps.len() - 1 {
                                ctx.emit(Instruction::Drop);
                            }
                        }
                    }
                }
            }
        }

        Ok(())
    }

    /// 二項演算子の IR 命令を出力
    pub(crate) fn emit_binop(&self, ctx: &mut FuncCtx, op: &str) -> Result<(), LowerError> {
        match op {
            "+" => ctx.emit(Instruction::I64Add),
            "-" => ctx.emit(Instruction::I64Sub),
            "*" => ctx.emit(Instruction::I64Mul),
            "/" => ctx.emit(Instruction::I64Div),
            "%" => ctx.emit(Instruction::I64Rem),
            "+." => ctx.emit(Instruction::F64Add),
            "-." => ctx.emit(Instruction::F64Sub),
            "*." => ctx.emit(Instruction::F64Mul),
            "/." => ctx.emit(Instruction::F64Div),
            "==" | "=" => {
                ctx.emit(Instruction::I64Eq);
                ctx.emit(Instruction::I64ExtendI32S);
            }
            "!=" => {
                ctx.emit(Instruction::I64Ne);
                ctx.emit(Instruction::I64ExtendI32S);
            }
            "<" => {
                ctx.emit(Instruction::I64LtS);
                ctx.emit(Instruction::I64ExtendI32S);
            }
            ">" => {
                ctx.emit(Instruction::I64GtS);
                ctx.emit(Instruction::I64ExtendI32S);
            }
            "<=" => {
                ctx.emit(Instruction::I64LeS);
                ctx.emit(Instruction::I64ExtendI32S);
            }
            ">=" => {
                ctx.emit(Instruction::I64GeS);
                ctx.emit(Instruction::I64ExtendI32S);
            }
            "and" => {
                ctx.emit(Instruction::I32WrapI64);
                ctx.emit(Instruction::I32And);
                ctx.emit(Instruction::I64ExtendI32S);
            }
            "or" => {
                ctx.emit(Instruction::I32WrapI64);
                ctx.emit(Instruction::I32Or);
                ctx.emit(Instruction::I64ExtendI32S);
            }
            _ => {
                return Err(LowerError::Unsupported {
                    msg: format!("未知の二項演算子: {}", op),
                });
            }
        }
        Ok(())
    }
}
