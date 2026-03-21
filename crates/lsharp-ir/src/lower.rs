//! Typed AST → IR 変換 (Lowering)

use std::collections::HashMap;

use lsharp_syntax::ast::*;
use lsharp_types::types::Type;

use crate::{Function, Instruction, IrType, Module};

/// Lowering エラー
#[derive(Debug, Clone, thiserror::Error)]
pub enum LowerError {
    #[error("未サポートの式: {msg}")]
    Unsupported { msg: String },

    #[error("未定義の関数: {name}")]
    UndefinedFunction { name: String },
}

/// Lowering コンテキスト
pub struct Lower {
    /// 関数名 → 関数インデックスのマッピング
    func_indices: HashMap<String, u32>,
    /// import 関数の数（ユーザー関数のインデックスオフセット）
    import_count: u32,
    /// 型推論結果
    type_results: HashMap<String, Type>,
}

impl Lower {
    pub fn new() -> Self {
        Self {
            func_indices: HashMap::new(),
            import_count: 0,
            type_results: HashMap::new(),
        }
    }

    /// プログラム全体を IR に変換
    pub fn lower_program(
        &mut self,
        program: &Program,
        type_results: &[(String, lsharp_types::types::TypeScheme)],
    ) -> Result<Module, LowerError> {
        // 型推論結果を保存
        for (name, scheme) in type_results {
            self.type_results.insert(name.clone(), scheme.ty.clone());
        }

        // import 関数を登録 (print = index 0)
        self.func_indices.insert("print".to_string(), 0);
        self.import_count = 1;

        // ユーザー定義関数のインデックスを事前登録
        let mut func_idx = self.import_count;
        for decl in &program.decls {
            if let Decl::Defn { name, .. } = decl {
                self.func_indices.insert(name.clone(), func_idx);
                func_idx += 1;
            }
        }

        // 各関数を IR に変換
        let mut functions = Vec::new();
        for decl in &program.decls {
            if let Decl::Defn {
                name, params, body, ..
            } = decl
            {
                let func = self.lower_function(name, params, body)?;
                functions.push(func);
            }
        }

        Ok(Module { functions })
    }

    /// 関数を IR に変換
    fn lower_function(
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
                    let p: Vec<IrType> = params.iter().map(|t| type_to_ir(t)).collect();
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

    /// 式を IR 命令に変換（スタックマシン方式）
    fn lower_expr(&self, ctx: &mut FuncCtx, expr: &Expr) -> Result<(), LowerError> {
        match expr {
            Expr::Lit(_, lit) => match lit {
                Literal::Int(n) => ctx.emit(Instruction::I64Const(*n)),
                Literal::Float(n) => ctx.emit(Instruction::F64Const(*n)),
                Literal::Bool(b) => ctx.emit(Instruction::I64Const(if *b { 1 } else { 0 })),
                Literal::String(_) => {
                    // MVP: 文字列は未サポート、0 を返す
                    ctx.emit(Instruction::I64Const(0));
                }
                Literal::Unit => ctx.emit(Instruction::I64Const(0)),
            },

            Expr::Var(_, name) => {
                if let Some(&idx) = ctx.locals_map.get(name) {
                    ctx.emit(Instruction::LocalGet(idx));
                } else {
                    return Err(LowerError::UndefinedFunction {
                        name: name.clone(),
                    });
                }
            }

            Expr::If(_, cond, then, else_) => {
                // 条件式
                self.lower_expr(ctx, cond)?;
                // Bool (i64) → i32 に変換
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
                    // 組み込み二項演算子
                    Expr::Var(_, op) if is_builtin_binop(op) && args.len() == 2 => {
                        self.lower_expr(ctx, &args[0])?;
                        self.lower_expr(ctx, &args[1])?;
                        self.emit_binop(ctx, op);
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
                        let idx = self.func_indices["print"];
                        ctx.emit(Instruction::Call(idx));
                        // print は Unit を返す
                        ctx.emit(Instruction::I64Const(0));
                    }
                    // ユーザー定義関数呼び出し
                    Expr::Var(_, name) => {
                        // 引数を評価
                        for arg in args {
                            self.lower_expr(ctx, arg)?;
                        }
                        if let Some(&idx) = self.func_indices.get(name.as_str()) {
                            ctx.emit(Instruction::Call(idx));
                        } else {
                            // ローカル変数（クロージャ呼び出し）は MVP では未サポート
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
        }

        Ok(())
    }

    /// match の腕を if-else チェインに変換
    fn lower_match_arms(
        &self,
        ctx: &mut FuncCtx,
        scrut_local: u32,
        arms: &[MatchArm],
        idx: usize,
    ) -> Result<(), LowerError> {
        if idx >= arms.len() {
            // 到達不能（網羅性チェック済みの前提）
            ctx.emit(Instruction::Unreachable);
            return Ok(());
        }

        let arm = &arms[idx];

        match &arm.pattern {
            // ワイルドカードや変数パターンは常にマッチ
            Pattern::Wildcard(_) => {
                self.lower_expr(ctx, &arm.body)?;
            }
            Pattern::Var(_, name) => {
                // scrutinee を変数に束縛
                ctx.emit(Instruction::LocalGet(scrut_local));
                let var_local = ctx.alloc_local(name.clone());
                ctx.emit(Instruction::LocalSet(var_local));
                self.lower_expr(ctx, &arm.body)?;
            }
            Pattern::Lit(_, Literal::Int(n)) => {
                // scrutinee == n なら本体を実行
                ctx.emit(Instruction::LocalGet(scrut_local));
                ctx.emit(Instruction::I64Const(*n));
                ctx.emit(Instruction::I64Eq);
                ctx.emit(Instruction::If(IrType::I64));
                self.lower_expr(ctx, &arm.body)?;
                ctx.emit(Instruction::Else);
                self.lower_match_arms(ctx, scrut_local, arms, idx + 1)?;
                ctx.emit(Instruction::End);
            }
            Pattern::Lit(_, Literal::Bool(b)) => {
                ctx.emit(Instruction::LocalGet(scrut_local));
                ctx.emit(Instruction::I64Const(if *b { 1 } else { 0 }));
                ctx.emit(Instruction::I64Eq);
                ctx.emit(Instruction::If(IrType::I64));
                self.lower_expr(ctx, &arm.body)?;
                ctx.emit(Instruction::Else);
                self.lower_match_arms(ctx, scrut_local, arms, idx + 1)?;
                ctx.emit(Instruction::End);
            }
            Pattern::Constructor(_, name, sub_pats) if sub_pats.is_empty() => {
                // 引数なしコンストラクタ: タグ比較（MVP: None=0 等の簡易マッピング）
                // MVP ではワイルドカードとして扱う
                let _ = name;
                if idx == arms.len() - 1 {
                    // 最後の腕はデフォルトとして扱う
                    self.lower_expr(ctx, &arm.body)?;
                } else {
                    // 次の腕と if-else
                    ctx.emit(Instruction::If(IrType::I64));
                    self.lower_expr(ctx, &arm.body)?;
                    ctx.emit(Instruction::Else);
                    self.lower_match_arms(ctx, scrut_local, arms, idx + 1)?;
                    ctx.emit(Instruction::End);
                }
            }
            Pattern::Constructor(_, _name, sub_pats) => {
                // MVP: 引数付きコンストラクタは変数パターンに退化
                // サブパターンの最初の変数に scrutinee を束縛
                if let Some(Pattern::Var(_, var_name)) = sub_pats.first() {
                    ctx.emit(Instruction::LocalGet(scrut_local));
                    let var_local = ctx.alloc_local(var_name.clone());
                    ctx.emit(Instruction::LocalSet(var_local));
                }
                if idx == arms.len() - 1 {
                    self.lower_expr(ctx, &arm.body)?;
                } else {
                    self.lower_expr(ctx, &arm.body)?;
                }
            }
            _ => {
                return Err(LowerError::Unsupported {
                    msg: format!("パターン: {:?}", arm.pattern),
                });
            }
        }

        Ok(())
    }

    /// 二項演算子の IR 命令を出力
    fn emit_binop(&self, ctx: &mut FuncCtx, op: &str) {
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
            "==" => {
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
                // 2つ目の値もラップが必要だが、スタック順の問題で省略
                ctx.emit(Instruction::I32And);
                ctx.emit(Instruction::I64ExtendI32S);
            }
            "or" => {
                ctx.emit(Instruction::I32WrapI64);
                ctx.emit(Instruction::I32Or);
                ctx.emit(Instruction::I64ExtendI32S);
            }
            _ => {} // 未知の演算子は無視
        }
    }
}

impl Default for Lower {
    fn default() -> Self {
        Self::new()
    }
}

/// 関数変換コンテキスト
struct FuncCtx {
    #[allow(dead_code)]
    name: String,
    instructions: Vec<Instruction>,
    locals_map: HashMap<String, u32>,
    param_count: u32,
    next_local: u32,
}

impl FuncCtx {
    fn new(name: String) -> Self {
        Self {
            name,
            instructions: Vec::new(),
            locals_map: HashMap::new(),
            param_count: 0,
            next_local: 0,
        }
    }

    fn emit(&mut self, instr: Instruction) {
        self.instructions.push(instr);
    }

    fn alloc_local(&mut self, name: String) -> u32 {
        if let Some(&idx) = self.locals_map.get(&name) {
            return idx;
        }
        let idx = self.next_local;
        self.locals_map.insert(name, idx);
        self.next_local += 1;
        idx
    }
}

/// 組み込み二項演算子か判定
fn is_builtin_binop(name: &str) -> bool {
    matches!(
        name,
        "+" | "-"
            | "*"
            | "/"
            | "%"
            | "+."
            | "-."
            | "*."
            | "/."
            | "=="
            | "!="
            | "<"
            | ">"
            | "<="
            | ">="
            | "and"
            | "or"
    )
}

/// L# 型 → IR 型
pub fn type_to_ir(ty: &Type) -> IrType {
    match ty {
        Type::Con(name) => match name.as_str() {
            "Int" => IrType::I64,
            "Float" => IrType::F64,
            "Bool" => IrType::I64, // Bool は i64 (0/1)
            "Unit" => IrType::I64, // Unit も i64 (0)
            "String" => IrType::I64, // MVP: 文字列はポインタ (i64)
            _ => IrType::I64,
        },
        Type::Var(_) => IrType::I64, // 未解決の型変数はデフォルト i64
        Type::Fun(_, _) => IrType::I64, // 関数ポインタ
        Type::App(_, _) => IrType::I64, // ADT ポインタ
    }
}
