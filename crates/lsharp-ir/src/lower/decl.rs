//! 宣言の lowering (関数、ADT、レコード、制約付き型等)

use lsharp_syntax::ast::*;
use lsharp_types::types::Type;

use crate::{Function, Instruction, IrType};

use super::{type_to_ir, type_to_name, type_expr_to_name, FuncCtx, Lower, LowerError};

impl Lower {
    /// レコード型のフィールドインデックスを解決
    pub(crate) fn resolve_field_index(&self, type_name: &str, field_name: &str) -> Option<u32> {
        if let Some(fields) = self.record_fields.get(type_name) {
            fields.iter().position(|f| f == field_name).map(|i| i as u32)
        } else {
            None
        }
    }

    /// トレイトメソッド呼び出しを静的ディスパッチで解決（P5-6）
    ///
    /// 関数名がトレイトメソッドに該当する場合、第一引数の型情報から
    /// 具体的な実装（マングル名）を解決して関数インデックスを返す。
    pub(crate) fn resolve_trait_dispatch(&self, method_name: &str, args: &[Expr]) -> Option<u32> {
        // メソッド名がトレイトメソッドとして登録されているか確認
        let trait_names = self.trait_method_names.get(method_name)?;

        // 第一引数の型名を推定
        let first_arg_type = if let Some(arg) = args.first() {
            self.infer_expr_type_name(arg)
        } else {
            None
        };

        if let Some(type_name) = first_arg_type {
            // (trait_name, type_name, method_name) でマングル名を検索
            for trait_name in trait_names {
                let key = (trait_name.clone(), type_name.clone(), method_name.to_string());
                if let Some(mangled) = self.trait_method_impls.get(&key) {
                    return self.func_indices.get(mangled).copied();
                }
            }
        }

        // 型が不明な場合、実装が1つだけならそれを使う（一意解決）
        for trait_name in trait_names {
            let matching: Vec<_> = self.trait_method_impls.iter()
                .filter(|((t, _, m), _)| t == trait_name && m == method_name)
                .collect();
            if matching.len() == 1 {
                let (_, mangled) = matching[0];
                return self.func_indices.get(mangled).copied();
            }
        }

        None
    }

    /// 式の型名を推定する（静的ディスパッチ用の簡易推定）
    pub(crate) fn infer_expr_type_name(&self, expr: &Expr) -> Option<String> {
        match expr {
            // リテラルから型を推定
            Expr::Lit(_, Literal::Int(_)) => Some("Int".to_string()),
            Expr::Lit(_, Literal::Float(_)) => Some("Float".to_string()),
            Expr::Lit(_, Literal::Bool(_)) => Some("Bool".to_string()),
            Expr::Lit(_, Literal::String(_)) => Some("String".to_string()),
            Expr::Lit(_, Literal::Unit) => Some("Unit".to_string()),
            // 変数の場合、型推論結果から型名を取得
            Expr::Var(_, name) => {
                if let Some(ty) = self.type_results.get(name) {
                    type_to_name(ty)
                } else {
                    None
                }
            }
            // 型注釈がある場合
            Expr::Ann(_, _, type_expr) => type_expr_to_name(type_expr),
            // レコードリテラルの場合、型名が明示的
            Expr::RecordLit(_, type_name, _) => Some(type_name.clone()),
            // 関数呼び出しの場合、戻り値型を推定
            Expr::App(_, func, _) => {
                if let Expr::Var(_, func_name) = func.as_ref() {
                    if let Some(ty) = self.type_results.get(func_name) {
                        match ty {
                            Type::Fun(_, ret) => type_to_name(ret),
                            _ => None,
                        }
                    } else {
                        None
                    }
                } else {
                    None
                }
            }
            _ => None,
        }
    }

    /// フィールドアクセサ関数を生成
    pub(crate) fn generate_field_accessor(
        &self,
        type_name: &str,
        field_name: &str,
        field_idx: u32,
        _field_type: &lsharp_syntax::ast::TypeExpr,
    ) -> Function {
        let accessor_name = format!("{type_name}.{field_name}");

        // MVP: レコードはフラットに i64 として扱う
        // 将来的に WasmGC struct.get に変換
        let mut body = Vec::new();
        if let Some(&gc_type_idx) = self.record_type_indices.get(type_name) {
            body.push(Instruction::LocalGet(0));
            body.push(Instruction::StructGet(gc_type_idx, field_idx));
        } else {
            // フォールバック: 引数をそのまま返す
            body.push(Instruction::LocalGet(0));
        }

        Function {
            name: accessor_name,
            params: vec![IrType::I64],
            result: IrType::I64,
            locals: Vec::new(),
            body,
            is_export: false,
        }
    }


    /// ADT コンストラクタ関数を生成 (リニアメモリ版)
    ///
    /// ヒープレイアウト: [heap_tag=3: i32, variant_tag: i32, field_0: i64, ...]
    /// __alloc でメモリ確保 → ヘッダ書き込み → フィールド書き込み → タグ付きポインタ返却
    pub(crate) fn generate_adt_constructor(
        &self,
        variant_name: &str,
        _gc_type_idx: u32,
        tag_val: i32,
        field_count: usize,
    ) -> Function {
        let mut body = Vec::new();
        // ローカル変数の割り当て:
        // 0..field_count: パラメータ (フィールド値)
        // field_count: _addr (i32, ヒープアドレス)
        let addr_local = field_count as u32;
        let alloc_size = 8 + (field_count as i32) * 8; // ヘッダ 8 バイト + フィールド各 8 バイト

        // __alloc(size) でメモリ確保
        body.push(Instruction::I64Const(alloc_size as i64));
        let alloc_idx = *self.func_indices.get("__alloc").unwrap_or(&1);
        body.push(Instruction::Call(alloc_idx));
        // __alloc は i64 を返す → i32 に変換してローカルに保存
        body.push(Instruction::I32WrapI64);
        body.push(Instruction::LocalSet(addr_local));

        // heap_tag=3 (ADT) を offset 0 に書き込む
        body.push(Instruction::LocalGet(addr_local));
        body.push(Instruction::I32Const(super::HEAP_TAG_ADT));
        body.push(Instruction::I32Store { offset: 0 });

        // variant_tag を offset 4 に書き込む
        body.push(Instruction::LocalGet(addr_local));
        body.push(Instruction::I32Const(tag_val));
        body.push(Instruction::I32Store { offset: 4 });

        // 各フィールドを書き込む: mem[addr + 8 + i*8] = field_i
        for i in 0..field_count {
            body.push(Instruction::LocalGet(addr_local));
            body.push(Instruction::LocalGet(i as u32));
            body.push(Instruction::I64Store { offset: 8 + (i as u32) * 8 });
        }

        // タグ付きポインタを返す: addr | (1 << 63)
        body.push(Instruction::LocalGet(addr_local));
        super::emit_tag_pointer(&mut body, addr_local);

        Function {
            name: variant_name.to_string(),
            params: vec![IrType::I64; field_count],
            result: IrType::I64,
            locals: vec![IrType::I32], // addr_local (i32)
            body,
            is_export: false,
        }
    }

    /// 制約付き型のスマートコンストラクタ (Name.new) を生成
    /// 制約を満たさない場合は unreachable (トラップ) する
    pub(crate) fn generate_constraint_check(
        &self,
        type_name: &str,
        constraints: &[Constraint],
    ) -> Function {
        let func_name = format!("{type_name}.new");
        let mut body = Vec::new();

        for constraint in constraints {
            match constraint {
                Constraint::Gte(expr) => {
                    if let Expr::Lit(_, Literal::Int(threshold)) = expr {
                        body.push(Instruction::LocalGet(0));
                        body.push(Instruction::I64Const(*threshold));
                        body.push(Instruction::I64GeS);
                        body.push(Instruction::I32Eqz);
                        body.push(Instruction::If(IrType::I64));
                        body.push(Instruction::Unreachable);
                        body.push(Instruction::Else);
                        body.push(Instruction::I64Const(0));
                        body.push(Instruction::End);
                        body.push(Instruction::Drop);
                    }
                }
                Constraint::Lte(expr) => {
                    if let Expr::Lit(_, Literal::Int(threshold)) = expr {
                        body.push(Instruction::LocalGet(0));
                        body.push(Instruction::I64Const(*threshold));
                        body.push(Instruction::I64LeS);
                        body.push(Instruction::I32Eqz);
                        body.push(Instruction::If(IrType::I64));
                        body.push(Instruction::Unreachable);
                        body.push(Instruction::Else);
                        body.push(Instruction::I64Const(0));
                        body.push(Instruction::End);
                        body.push(Instruction::Drop);
                    }
                }
                Constraint::Range(lo_expr, hi_expr) => {
                    if let (Expr::Lit(_, Literal::Int(lo)), Expr::Lit(_, Literal::Int(hi))) =
                        (lo_expr, hi_expr)
                    {
                        body.push(Instruction::LocalGet(0));
                        body.push(Instruction::I64Const(*lo));
                        body.push(Instruction::I64GeS);
                        body.push(Instruction::I32Eqz);
                        body.push(Instruction::If(IrType::I64));
                        body.push(Instruction::Unreachable);
                        body.push(Instruction::Else);
                        body.push(Instruction::I64Const(0));
                        body.push(Instruction::End);
                        body.push(Instruction::Drop);
                        body.push(Instruction::LocalGet(0));
                        body.push(Instruction::I64Const(*hi));
                        body.push(Instruction::I64LeS);
                        body.push(Instruction::I32Eqz);
                        body.push(Instruction::If(IrType::I64));
                        body.push(Instruction::Unreachable);
                        body.push(Instruction::Else);
                        body.push(Instruction::I64Const(0));
                        body.push(Instruction::End);
                        body.push(Instruction::Drop);
                    }
                }
                _ => {}
            }
        }

        body.push(Instruction::LocalGet(0));

        Function {
            name: func_name,
            params: vec![IrType::I64],
            result: IrType::I64,
            locals: Vec::new(),
            body,
            is_export: false,
        }
    }

    /// 制約付き型の検証関数 (Name.valid?) を生成
    pub(crate) fn generate_constraint_valid(
        &self,
        type_name: &str,
        constraints: &[Constraint],
    ) -> Function {
        let func_name = format!("{type_name}.valid?");
        let mut body = Vec::new();

        body.push(Instruction::I64Const(1));

        for constraint in constraints {
            match constraint {
                Constraint::Gte(expr) => {
                    if let Expr::Lit(_, Literal::Int(threshold)) = expr {
                        body.push(Instruction::LocalGet(0));
                        body.push(Instruction::I64Const(*threshold));
                        body.push(Instruction::I64GeS);
                        body.push(Instruction::I64ExtendI32S);
                        body.push(Instruction::I32WrapI64);
                        body.push(Instruction::I32And);
                        body.push(Instruction::I64ExtendI32S);
                    }
                }
                Constraint::Lte(expr) => {
                    if let Expr::Lit(_, Literal::Int(threshold)) = expr {
                        body.push(Instruction::LocalGet(0));
                        body.push(Instruction::I64Const(*threshold));
                        body.push(Instruction::I64LeS);
                        body.push(Instruction::I64ExtendI32S);
                        body.push(Instruction::I32WrapI64);
                        body.push(Instruction::I32And);
                        body.push(Instruction::I64ExtendI32S);
                    }
                }
                Constraint::Range(lo_expr, hi_expr) => {
                    if let (Expr::Lit(_, Literal::Int(lo)), Expr::Lit(_, Literal::Int(hi))) =
                        (lo_expr, hi_expr)
                    {
                        body.push(Instruction::LocalGet(0));
                        body.push(Instruction::I64Const(*lo));
                        body.push(Instruction::I64GeS);
                        body.push(Instruction::I64ExtendI32S);
                        body.push(Instruction::I32WrapI64);
                        body.push(Instruction::I32And);
                        body.push(Instruction::I64ExtendI32S);
                        body.push(Instruction::LocalGet(0));
                        body.push(Instruction::I64Const(*hi));
                        body.push(Instruction::I64LeS);
                        body.push(Instruction::I64ExtendI32S);
                        body.push(Instruction::I32WrapI64);
                        body.push(Instruction::I32And);
                        body.push(Instruction::I64ExtendI32S);
                    }
                }
                _ => {}
            }
        }

        Function {
            name: func_name,
            params: vec![IrType::I64],
            result: IrType::I64,
            locals: Vec::new(),
            body,
            is_export: false,
        }
    }

    /// 関数を IR に変換
    pub(crate) fn lower_function(
        &self,
        name: &str,
        params: &[Param],
        body: &Expr,
    ) -> Result<Function, LowerError> {
        let mut ctx = FuncCtx::new(name.to_string());

        // パラメータをローカル変数として登録
        for param in params {
            let idx = ctx.next_local;
            ctx.locals_map.insert(param.name.clone(), idx);
            ctx.param_count += 1;
            ctx.next_local += 1;
        }

        // 本体を変換
        self.lower_expr(&mut ctx, body)?;

        // 関数型を推論結果から取得
        let (param_types, result_type) = if let Some(ty) = self.type_results.get(name) {
            match ty {
                Type::Fun(params, ret) => {
                    let p: Vec<IrType> = params.iter().map(type_to_ir).collect();
                    let r = type_to_ir(ret);
                    (p, r)
                }
                _ => (Vec::new(), type_to_ir(ty)),
            }
        } else {
            let p = vec![IrType::I64; params.len()];
            (p, IrType::I64)
        };

        // ローカル変数（パラメータ以外）
        let extra_locals = vec![IrType::I64; (ctx.next_local - ctx.param_count) as usize];

        Ok(Function {
            name: name.to_string(),
            params: param_types,
            result: result_type,
            locals: extra_locals,
            body: ctx.instructions,
            is_export: name == "main",
        })
    }
}
