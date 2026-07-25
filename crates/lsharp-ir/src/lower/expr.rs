//! 式の lowering (lower_expr 関連)

use std::collections::HashMap;

use lsharp_syntax::ast::*;
use lsharp_types::{infer::ExprTypeKey, types::Type};

use crate::{Function, Instruction, IrType};

use super::{FuncCtx, Lower, LowerBackend, LowerError, type_to_name};

mod application;
mod application_calls;
mod application_map;
mod application_ref_vector;
mod application_scalar;
#[cfg(test)]
mod application_tests;
mod helpers;
#[cfg(test)]
mod helpers_tests;
mod wasmgc_lambda;
#[cfg(test)]
mod wasmgc_lambda_tests;

#[derive(Debug, Clone, Copy)]
struct WasmGcCapturedLambdaInfo {
    env_type_index: u32,
    call_ref_type_index: u32,
}

impl Lower {
    /// 式を IR 命令に変換（スタックマシン方式）
    pub(crate) fn lower_expr(&mut self, ctx: &mut FuncCtx, expr: &Expr) -> Result<(), LowerError> {
        match expr {
            Expr::Lit(expr_span, lit) => match lit {
                Literal::Int(n) => ctx.emit(Instruction::I64Const(*n)),
                Literal::Float(n) => ctx.emit(Instruction::F64Const(*n)),
                Literal::Bool(b) => ctx.emit(Instruction::I64Const(if *b { 1 } else { 0 })),
                Literal::String(s) => {
                    if self.backend == LowerBackend::WasmGc {
                        let type_index = self.string_array_type_index.ok_or_else(|| {
                            LowerError::Unsupported {
                                msg: "WasmGC String の GC array type が登録されていません"
                                    .to_string(),
                                span: Some(*expr_span),
                            }
                        })?;
                        for byte in s.as_bytes() {
                            ctx.emit(Instruction::I32Const(i32::from(*byte)));
                        }
                        ctx.emit(Instruction::ArrayNewFixed(
                            type_index,
                            s.len().try_into().map_err(|_| LowerError::Unsupported {
                                msg:
                                    "WasmGC String literal が array.new_fixed の長さを超えています"
                                        .to_string(),
                                span: Some(*expr_span),
                            })?,
                        ));
                        return Ok(());
                    }

                    // 文字列リテラル: データセクションにバイト列を格納し、
                    // ランタイムでヒープ上に String オブジェクト [tag=1, len, bytes] を確保
                    let bytes = s.as_bytes().to_vec();
                    let len = bytes.len() as u32;
                    let data_offset = self.string_offset;
                    let label = format!("$str{}", self.string_data.len());
                    self.string_data.push((label, bytes));
                    self.string_offset += len;

                    let alloc_idx = *self.func_indices.get("__alloc").ok_or_else(|| {
                        LowerError::UndefinedFunction {
                            name: "__alloc".to_string(),
                            span: Some(*expr_span),
                        }
                    })?;

                    // __alloc(8 + len) でヒープ領域を確保
                    ctx.emit(Instruction::I64Const((8 + len) as i64));
                    ctx.emit(Instruction::Call(alloc_idx));
                    // 戻り値 = ヒープオブジェクトのアドレス (i64)
                    let obj_local = ctx.alloc_local("_str_obj".to_string());
                    ctx.emit(Instruction::LocalSet(obj_local));

                    // tag = String を書き込み (obj + 0)
                    ctx.emit(Instruction::LocalGet(obj_local));
                    ctx.emit(Instruction::I32WrapI64);
                    ctx.emit(Instruction::I32Const(super::HEAP_TAG_STRING));
                    ctx.emit(Instruction::I32Store { offset: 0 });

                    // len を書き込み (obj + 4)
                    ctx.emit(Instruction::LocalGet(obj_local));
                    ctx.emit(Instruction::I32WrapI64);
                    ctx.emit(Instruction::I32Const(len as i32));
                    ctx.emit(Instruction::I32Store { offset: 4 });

                    if len > 0 {
                        // memory.copy(obj + 8, data_offset, len)
                        // dst: obj + 8
                        ctx.emit(Instruction::LocalGet(obj_local));
                        ctx.emit(Instruction::I32WrapI64);
                        ctx.emit(Instruction::I32Const(8));
                        ctx.emit(Instruction::I32Add);
                        // src: data_offset (データセクション上のアドレス)
                        ctx.emit(Instruction::I32Const(data_offset as i32));
                        // len
                        ctx.emit(Instruction::I32Const(len as i32));
                        ctx.emit(Instruction::MemoryCopy);
                    }

                    // タグ付き String handle をスタックに積む
                    ctx.emit(Instruction::LocalGet(obj_local));
                    ctx.emit(Instruction::I64Const(1i64 << 63));
                    ctx.emit(Instruction::I64Add);
                }
                Literal::Unit => ctx.emit(Instruction::I64Const(0)),
            },

            Expr::Var(expr_span, name) => {
                if let Some(&idx) = ctx.locals_map.get(name) {
                    ctx.emit(Instruction::LocalGet(idx));
                } else if let Some(&func_idx) = self.func_indices.get(name) {
                    // 引数なし ADT コンストラクタ（または引数なし関数）を呼び出し
                    ctx.emit(Instruction::Call(func_idx));
                } else if let Some(&func_idx) = self.lifted_func_indices.get(name) {
                    // Lambda Lifting で生成された関数の呼び出し
                    ctx.emit(Instruction::Call(func_idx));
                } else {
                    return Err(LowerError::UndefinedFunction {
                        name: name.clone(),
                        span: Some(*expr_span),
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
                let mut scoped_bindings = Vec::new();
                let result = (|| -> Result<(), LowerError> {
                    for (pat, val) in bindings {
                        let inferred_type_name = self.infer_expr_type_name_with_ctx(ctx, val);
                        let is_captured_lambda = if self.backend == super::LowerBackend::WasmGc
                            && let Expr::Lambda(lambda_span, params, body) = val
                        {
                            let free_var_list = self.wasmgc_lambda_free_vars(params, body);
                            if free_var_list.is_empty() {
                                false
                            } else {
                                self.lower_wasmgc_captured_lambda_value(
                                    ctx,
                                    *lambda_span,
                                    params,
                                    body,
                                    &free_var_list,
                                )?;
                                true
                            }
                        } else {
                            false
                        };
                        if !is_captured_lambda {
                            self.lower_expr(ctx, val)?;
                        }
                        let lambda_func_index = if self.backend == super::LowerBackend::WasmGc
                            && matches!(val, Expr::Lambda(_, _, _))
                        {
                            match ctx.instructions.last() {
                                Some(Instruction::RefFunc(function_index)) => Some(*function_index),
                                _ => None,
                            }
                        } else {
                            None
                        };
                        let lambda_func_type_index =
                            lambda_func_index.map(|index| self.gc_types.len() as u32 + index);
                        let lambda_env_type_index = if self.backend == super::LowerBackend::WasmGc
                            && matches!(val, Expr::Lambda(_, _, _))
                        {
                            match ctx.instructions.last() {
                                Some(Instruction::StructNew(type_index)) => Some(*type_index),
                                _ => None,
                            }
                        } else {
                            None
                        };
                        match pat {
                            Pattern::Var(_, name) => {
                                let previous_local = ctx.locals_map.get(name).copied();
                                let previous_type = ctx.local_type_names.get(name).cloned();
                                let binding_ir_type = lambda_env_type_index
                                    .map(IrType::Ref)
                                    .or_else(|| lambda_func_type_index.map(IrType::TypedFuncRef))
                                    .or_else(|| {
                                        inferred_type_name
                                            .as_deref()
                                            .map(|type_name| self.ir_type_for_type_name(type_name))
                                            .filter(|ty| {
                                                self.backend == super::LowerBackend::WasmGc
                                                    && matches!(ty, IrType::Ref(_))
                                            })
                                    })
                                    .unwrap_or(IrType::I64);
                                let idx =
                                    ctx.alloc_scoped_local_typed(name.clone(), binding_ir_type);
                                if let Some(type_name) = inferred_type_name {
                                    ctx.local_type_names.insert(name.clone(), type_name);
                                } else {
                                    ctx.local_type_names.remove(name);
                                }
                                scoped_bindings.push((name.clone(), previous_local, previous_type));
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
                    self.lower_expr(ctx, body)
                })();
                for (name, previous_local, previous_type) in scoped_bindings.into_iter().rev() {
                    ctx.restore_local_binding(name, previous_local, previous_type);
                }
                result?;
            }
            Expr::App(expr_span, func, args) => {
                self.lower_app(ctx, *expr_span, func, args)?;
            }
            Expr::Match(_, scrutinee, arms) => {
                // scrutinee を評価してローカルに保存
                self.lower_expr(ctx, scrutinee)?;
                let scrut_type_name = self.infer_expr_type_name_with_ctx(ctx, scrutinee);
                let scrut_ir_type = scrut_type_name
                    .as_deref()
                    .map(|name| self.ir_type_for_type_name(name))
                    .unwrap_or(IrType::I64);
                let scrut_local = ctx.alloc_local_typed("_match".to_string(), scrut_ir_type);
                if let Some(type_name) = scrut_type_name {
                    ctx.local_type_names.insert("_match".to_string(), type_name);
                }
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

            Expr::Lambda(_, params, body) => {
                // Lambda Lifting: Lambda 式をトップレベル関数にリフト
                let lambda_name = self.fresh_lambda_name();

                // 自由変数を検出
                let free_var_list = self.wasmgc_lambda_free_vars(params, body);

                if self.backend == LowerBackend::WasmGc && !free_var_list.is_empty() {
                    return Err(LowerError::Unsupported {
                        msg: "WasmGC captured closure は typed funcref/env struct への変換が未実装です"
                            .to_string(),
                        span: Some(expr.span()),
                    });
                }

                if self.backend == LowerBackend::WasmGc {
                    let lambda_type = self
                        .expr_type_results
                        .get(&ExprTypeKey::new(&ctx.type_scope_key, expr.span()))
                        .cloned();
                    let (param_types, param_type_names, inferred_result_type) = match lambda_type {
                        Some(Type::Fun(inferred_params, ret))
                            if inferred_params.len() == params.len() =>
                        {
                            (
                                inferred_params
                                    .iter()
                                    .map(|ty| self.ir_type_for_type(ty))
                                    .collect::<Vec<_>>(),
                                inferred_params.iter().map(type_to_name).collect::<Vec<_>>(),
                                Some(self.ir_type_for_type(&ret)),
                            )
                        }
                        _ => (
                            params
                                .iter()
                                .map(|param| {
                                    param
                                        .ty
                                        .as_ref()
                                        .map(|ty| self.type_expr_to_ir(ty))
                                        .unwrap_or(IrType::I64)
                                })
                                .collect::<Vec<_>>(),
                            params
                                .iter()
                                .map(|param| param.ty.as_ref().and_then(super::type_expr_to_name))
                                .collect::<Vec<_>>(),
                            None,
                        ),
                    };

                    let mut lifted_ctx =
                        FuncCtx::with_type_scope(lambda_name.clone(), ctx.type_scope_key.clone());
                    for (param_idx, param) in params.iter().enumerate() {
                        let idx = lifted_ctx.next_local;
                        lifted_ctx.locals_map.insert(param.name.clone(), idx);
                        if let Some(type_name) = param_type_names.get(param_idx).cloned().flatten()
                        {
                            lifted_ctx
                                .local_type_names
                                .insert(param.name.clone(), type_name);
                        }
                        lifted_ctx.param_count += 1;
                        lifted_ctx.next_local += 1;
                        lifted_ctx
                            .local_types
                            .push(param_types.get(param_idx).copied().unwrap_or(IrType::I64));
                    }

                    self.lower_expr(&mut lifted_ctx, body)?;
                    let result_type = inferred_result_type
                        .or_else(|| {
                            self.infer_expr_type_name_with_ctx(&lifted_ctx, body)
                                .map(|name| self.ir_type_for_type_name(&name))
                        })
                        .unwrap_or(IrType::I64);
                    let extra_local_count =
                        (lifted_ctx.next_local - lifted_ctx.param_count) as usize;
                    let extra_locals = lifted_ctx
                        .local_types
                        .get(lifted_ctx.param_count as usize..)
                        .filter(|types| types.len() == extra_local_count)
                        .map_or_else(
                            || vec![IrType::I64; extra_local_count],
                            |types| types.to_vec(),
                        );
                    let lifted_func = Function {
                        name: lambda_name.clone(),
                        params: param_types,
                        result: result_type,
                        locals: extra_locals,
                        body: lifted_ctx.instructions,
                        is_export: false,
                    };
                    let func_idx = self.next_func_idx + self.lifted_functions.len() as u32;
                    let local_func_idx =
                        func_idx.checked_sub(self.import_count).ok_or_else(|| {
                            LowerError::Unsupported {
                            msg: "WasmGC lambda の function index が runtime import 境界より前です"
                                .to_string(),
                            span: Some(expr.span()),
                        }
                        })?;
                    self.lifted_func_indices.insert(lambda_name, func_idx);
                    self.lifted_functions.push(lifted_func);
                    ctx.emit(Instruction::RefFunc(local_func_idx));
                    return Ok(());
                }

                // リフト先関数を構築:
                // 統一呼び出し規約: (元パラメータ..., closure_ptr) -> result
                // リフト関数内部で closure_ptr からキャプチャ値を読み出す
                let mut lifted_ctx =
                    FuncCtx::with_type_scope(lambda_name.clone(), ctx.type_scope_key.clone());
                // 元のパラメータを登録
                for p in params {
                    let idx = lifted_ctx.next_local;
                    lifted_ctx.locals_map.insert(p.name.clone(), idx);
                    lifted_ctx.param_count += 1;
                    lifted_ctx.next_local += 1;
                }
                // closure_ptr パラメータを追加（常に最後のパラメータ）
                let closure_ptr_idx = lifted_ctx.next_local;
                lifted_ctx
                    .locals_map
                    .insert("__closure_ptr".to_string(), closure_ptr_idx);
                lifted_ctx.param_count += 1;
                lifted_ctx.next_local += 1;

                // 自由変数をローカルに読み出すプロローグを生成
                let mut prologue = Vec::new();
                for (i, fv) in free_var_list.iter().enumerate() {
                    let fv_local = lifted_ctx.alloc_local(fv.clone());
                    // closure_ptr からキャプチャ値を読み出す:
                    // fv_local = i64.load(i32.wrap(closure_ptr) + 8 + i*8)
                    prologue.push(Instruction::LocalGet(closure_ptr_idx));
                    prologue.push(Instruction::I32WrapI64);
                    prologue.push(Instruction::I64Load {
                        offset: 8 + (i as u32) * 8,
                    });
                    prologue.push(Instruction::LocalSet(fv_local));
                }

                // Lambda の本体を変換
                self.lower_expr(&mut lifted_ctx, body)?;

                // プロローグを本体の先頭に挿入
                let mut full_body = prologue;
                full_body.extend(lifted_ctx.instructions);

                // リフト先関数のパラメータ型: (元パラメータ..., closure_ptr)
                let total_params = params.len() + 1; // +1 は closure_ptr
                let extra_locals =
                    vec![IrType::I64; (lifted_ctx.next_local - lifted_ctx.param_count) as usize];

                let lifted_func = Function {
                    name: lambda_name.clone(),
                    params: vec![IrType::I64; total_params],
                    result: IrType::I64,
                    locals: extra_locals,
                    body: full_body,
                    is_export: false,
                };

                // リフトされた関数のインデックスを割り当て
                let func_idx = self.next_func_idx + self.lifted_functions.len() as u32;
                self.lifted_func_indices.insert(lambda_name, func_idx);
                self.lifted_functions.push(lifted_func);

                // クロージャオブジェクトをヒープに確保（自由変数の有無に関わらず）
                // レイアウト: [heap_tag=4: i32, func_idx: i32, captured_0: i64, ...]
                {
                    let n_captures = free_var_list.len();
                    let alloc_size = 8 + (n_captures as i64) * 8; // 最低 8 バイト (ヘッダのみ)

                    // __alloc(size) でメモリ確保
                    ctx.emit(Instruction::I64Const(alloc_size));
                    let alloc_idx = *self.func_indices.get("__alloc").unwrap_or(&1);
                    ctx.emit(Instruction::Call(alloc_idx));
                    // __alloc は i64 を返す → i64 のままローカルに保存
                    let addr_local = ctx.alloc_local("_closure_addr".to_string());
                    ctx.emit(Instruction::LocalSet(addr_local));

                    // heap_tag=4 (CLOSURE) を offset 0 に書き込む
                    ctx.emit(Instruction::LocalGet(addr_local));
                    ctx.emit(Instruction::I32WrapI64);
                    ctx.emit(Instruction::I32Const(super::HEAP_TAG_CLOSURE));
                    ctx.emit(Instruction::I32Store { offset: 0 });

                    // func_idx を offset 4 に書き込む
                    ctx.emit(Instruction::LocalGet(addr_local));
                    ctx.emit(Instruction::I32WrapI64);
                    ctx.emit(Instruction::FuncIdx(func_idx));
                    ctx.emit(Instruction::I32Store { offset: 4 });

                    // キャプチャ値を書き込む: mem[addr + 8 + i*8] = captured_i
                    for (i, fv) in free_var_list.iter().enumerate() {
                        ctx.emit(Instruction::LocalGet(addr_local));
                        ctx.emit(Instruction::I32WrapI64);
                        if let Some(&fv_local) = ctx.locals_map.get(fv) {
                            ctx.emit(Instruction::LocalGet(fv_local));
                        } else {
                            // フォールバック: 0 を書き込む
                            ctx.emit(Instruction::I64Const(0));
                        }
                        ctx.emit(Instruction::I64Store {
                            offset: 8 + (i as u32) * 8,
                        });
                    }

                    // タグ付きポインタを返す: addr は i64 のまま
                    // 最上位ビットをセット: addr | (1 << 63)
                    ctx.emit(Instruction::LocalGet(addr_local));
                    ctx.emit(Instruction::I64Const(1i64 << 63));
                    ctx.emit(Instruction::I64Add);
                }
            }

            Expr::Ann(_, expr, _) => {
                // 型注釈は無視して中身を変換
                self.lower_expr(ctx, expr)?;
            }

            Expr::RecordLit(_, type_name, fields) => {
                if let Some(&gc_type_idx) = self.record_type_indices.get(type_name) {
                    // レコード定義のフィールド順序に従って値をスタックに積む
                    if let Some(field_order) = self.record_fields.get(type_name).cloned() {
                        let field_map: HashMap<&str, &Expr> =
                            fields.iter().map(|(n, e)| (n.as_str(), e)).collect();
                        for field_name in &field_order {
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

            Expr::FieldAccess(expr_span, expr, field_name) => {
                // 式を評価してスタックにレコード値を積む
                self.lower_expr(ctx, expr)?;

                // 型推論結果から型名を取得して正確にフィールドを解決 (R-M5)
                let type_name_hint = self.infer_expr_type_name(expr);
                let mut resolved = false;

                if let Some(ref tn) = type_name_hint {
                    // 型名が判明: 正確に解決
                    if let Some(fields) = self.record_fields.get(tn).cloned() {
                        if let Some(field_idx) = fields.iter().position(|f| f == field_name) {
                            if let Some(&gc_type_idx) = self.record_type_indices.get(tn) {
                                ctx.emit(Instruction::StructGet(gc_type_idx, field_idx as u32));
                                resolved = true;
                            }
                        } else {
                            return Err(LowerError::Unsupported {
                                msg: format!(
                                    "レコード型 '{tn}' にフィールド '{field_name}' が存在しません"
                                ),
                                span: Some(*expr_span),
                            });
                        }
                    }
                }

                if !resolved {
                    // フォールバック: フィールド名で全レコード型を走査
                    // record_fields を一時的にクローンして借用問題を回避
                    let record_fields_snapshot: Vec<(String, Vec<String>)> = self
                        .record_fields
                        .iter()
                        .map(|(k, v)| (k.clone(), v.clone()))
                        .collect();
                    for (type_name, fields) in &record_fields_snapshot {
                        if let Some(field_idx) = fields.iter().position(|f| f == field_name)
                            && let Some(&gc_type_idx) = self.record_type_indices.get(type_name)
                        {
                            ctx.emit(Instruction::StructGet(gc_type_idx, field_idx as u32));
                            resolved = true;
                            break;
                        }
                    }
                }

                if !resolved {
                    return Err(LowerError::Unsupported {
                        msg: format!("フィールド '{field_name}' を解決できません"),
                        span: Some(*expr_span),
                    });
                }
            }

            Expr::RecordUpdate(_, base, update_fields) => {
                // ベースレコードを評価してローカルに保存
                self.lower_expr(ctx, base)?;
                let base_type_name = self.infer_expr_type_name_with_ctx(ctx, base);
                let base_ir_type = base_type_name
                    .as_deref()
                    .and_then(|type_name| {
                        (self.backend == super::LowerBackend::WasmGc)
                            .then(|| {
                                self.record_type_indices
                                    .get(type_name)
                                    .copied()
                                    .map(IrType::Ref)
                            })
                            .flatten()
                    })
                    .unwrap_or(IrType::I64);
                let base_local = ctx.alloc_local_typed("_record_base".to_string(), base_ir_type);
                ctx.emit(Instruction::LocalSet(base_local));

                // 型推論結果からベース式の型名を取得 (R-m3)
                let type_name_hint = base_type_name;
                let mut found_type = None;

                if let Some(ref tn) = type_name_hint {
                    // 型名が判明: 正確に解決
                    if let Some(fields) = self.record_fields.get(tn).cloned() {
                        found_type = Some((tn.clone(), fields));
                    }
                }

                if found_type.is_none() {
                    // フォールバック: フィールド名で全レコード型を走査
                    let record_fields_snapshot: Vec<(String, Vec<String>)> = self
                        .record_fields
                        .iter()
                        .map(|(k, v)| (k.clone(), v.clone()))
                        .collect();
                    for (type_name, fields) in record_fields_snapshot {
                        let all_match = update_fields.iter().all(|(n, _)| fields.contains(n));
                        if all_match {
                            found_type = Some((type_name, fields));
                            break;
                        }
                    }
                }

                if let Some((type_name, field_order)) = found_type {
                    if let Some(&gc_type_idx) = self.record_type_indices.get(&type_name) {
                        let update_map: HashMap<&str, &Expr> =
                            update_fields.iter().map(|(n, e)| (n.as_str(), e)).collect();
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
            Expr::Computation(span, builder_name, steps) => {
                if self.backend == LowerBackend::WasmGc
                    && steps.iter().any(|step| {
                        matches!(
                            step,
                            ComputationStep::LetBang(..) | ComputationStep::DoBang(..)
                        )
                    })
                {
                    return Err(LowerError::Unsupported {
                        msg: "WasmGC backend の computation let!/do! は GC closure を使う bind が未対応です"
                            .to_string(),
                        span: Some(*span),
                    });
                }

                // Computation Expression: bind/return 関数呼び出しに脱糖
                let builder_info = self.computation_builders.get(builder_name).cloned();

                for (i, step) in steps.iter().enumerate() {
                    match step {
                        ComputationStep::LetBang(_, pat, expr) => {
                            // let! x = expr -> bind(expr, fn [x] rest)
                            // MVP: bind 関数を呼び出す（簡易版: 式を評価してローカルに格納）
                            self.lower_expr(ctx, expr)?;
                            if let Some((ref bind_fn, _)) = builder_info
                                && let Some(&idx) = self.func_indices.get(bind_fn.as_str())
                            {
                                // bind 関数の第1引数（モナド値）は既にスタック上
                                // 残りのステップは後続で評価される
                                // MVP: 式の結果をそのまま変数に束縛
                                let _ = idx; // 将来的に bind 呼び出しに使用
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
                            if let Some((_, ref return_fn)) = builder_info
                                && let Some(&idx) = self.func_indices.get(return_fn.as_str())
                            {
                                ctx.emit(Instruction::Call(idx));
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
            // P10-1: Quote/Unquote/UnquoteSplice はマクロ展開後には残らない
            Expr::Quote(expr_span, _)
            | Expr::Unquote(expr_span, _)
            | Expr::UnquoteSplice(expr_span, _) => {
                return Err(LowerError::Unsupported {
                    msg: "quote/unquote はマクロ展開後に使用できません".to_string(),
                    span: Some(*expr_span),
                });
            }
        }

        Ok(())
    }
}
