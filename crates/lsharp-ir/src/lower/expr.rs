//! 式の lowering (lower_expr 関連)

use std::collections::HashMap;

use lsharp_syntax::ast::*;

use crate::{closure, Function, Instruction, IrType};

use super::{is_builtin_binop, FuncCtx, Lower, LowerError};

impl Lower {
    /// 式を IR 命令に変換（スタックマシン方式）
    pub(crate) fn lower_expr(&mut self, ctx: &mut FuncCtx, expr: &Expr) -> Result<(), LowerError> {
        match expr {
            Expr::Lit(_, lit) => match lit {
                Literal::Int(n) => ctx.emit(Instruction::I64Const(*n)),
                Literal::Float(n) => ctx.emit(Instruction::F64Const(*n)),
                Literal::Bool(b) => ctx.emit(Instruction::I64Const(if *b { 1 } else { 0 })),
                Literal::String(s) => {
                    // 文字列リテラル: データセクションにバイト列を格納し、
                    // ランタイムでヒープ上に String オブジェクト [tag=1, len, bytes] を確保
                    let bytes = s.as_bytes().to_vec();
                    let len = bytes.len() as u32;
                    let data_offset = self.string_offset;
                    let label = format!("$str{}", self.string_data.len());
                    self.string_data.push((label, bytes));
                    self.string_offset += len;

                    let alloc_idx = *self.func_indices.get("__alloc").ok_or_else(|| {
                        LowerError::UndefinedFunction { name: "__alloc".to_string() }
                    })?;

                    // __alloc(8 + len) でヒープ領域を確保
                    ctx.emit(Instruction::I64Const((8 + len) as i64));
                    ctx.emit(Instruction::Call(alloc_idx));
                    // 戻り値 = ヒープオブジェクトのアドレス (i64)
                    let obj_local = ctx.alloc_local("_str_obj".to_string());
                    ctx.emit(Instruction::LocalSet(obj_local));

                    // tag = 1 を書き込み (obj + 0)
                    ctx.emit(Instruction::LocalGet(obj_local));
                    ctx.emit(Instruction::I32WrapI64);
                    ctx.emit(Instruction::I32Const(1));
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

                    // ヒープオブジェクトのアドレスをスタックに積む
                    ctx.emit(Instruction::LocalGet(obj_local));
                }
                Literal::Unit => ctx.emit(Instruction::I64Const(0)),
            },

            Expr::Var(_, name) => {
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
                    // print 関数 (多相: 引数型に応じて print-int / print-string を呼び分け)
                    Expr::Var(_, name) if name == "print" => {
                        if let Some(arg) = args.first() {
                            // 引数の型を推定して適切な print 関数を選択
                            let is_string = self.infer_expr_type_name(arg)
                                .map(|t| t == "String")
                                .unwrap_or(false);
                            self.lower_expr(ctx, arg)?;
                            if is_string {
                                // 文字列の場合: print-string (改行なし) + 改行出力
                                let idx = *self.func_indices.get("print-string").ok_or_else(|| {
                                    LowerError::UndefinedFunction { name: "print-string".to_string() }
                                })?;
                                ctx.emit(Instruction::Call(idx));
                            } else {
                                // 整数の場合: print (改行付き)
                                let idx = *self.func_indices.get("print").ok_or_else(|| {
                                    LowerError::UndefinedFunction { name: "print".to_string() }
                                })?;
                                ctx.emit(Instruction::Call(idx));
                            }
                        }
                        // print は Unit を返す
                        ctx.emit(Instruction::I64Const(0));
                    }
                    // print-string: 文字列を出力 (改行なし)
                    Expr::Var(_, name) if name == "print-string" => {
                        if let Some(arg) = args.first() {
                            self.lower_expr(ctx, arg)?;
                        }
                        let idx = *self.func_indices.get("print-string").ok_or_else(|| {
                            LowerError::UndefinedFunction { name: "print-string".to_string() }
                        })?;
                        ctx.emit(Instruction::Call(idx));
                        // print-string は Unit を返す
                        ctx.emit(Instruction::I64Const(0));
                    }
                    // proc-exit: プロセス終了 (Int -> Unit)
                    Expr::Var(_, name) if name == "proc-exit" => {
                        if let Some(arg) = args.first() {
                            self.lower_expr(ctx, arg)?;
                        }
                        // i64 -> i32 に変換して proc_exit を呼ぶ
                        ctx.emit(Instruction::I32WrapI64);
                        let idx = *self.func_indices.get("proc-exit").ok_or_else(|| {
                            LowerError::UndefinedFunction { name: "proc-exit".to_string() }
                        })?;
                        ctx.emit(Instruction::Call(idx));
                        // proc-exit は Unit を返す（実際にはここに到達しないが型整合のため）
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
                    // string-char-at: ヒープ上 String オブジェクトのバイト値を返す
                    // String オブジェクト: [tag:i32=1][len:i32][bytes:u8*]
                    Expr::Var(_, name) if name == "string-char-at" => {
                        if args.len() >= 2 {
                            self.lower_expr(ctx, &args[0])?; // s (String オブジェクトアドレス)
                            self.lower_expr(ctx, &args[1])?; // i (index)
                        }
                        // i を一時ローカルに保存 (i64 のまま)
                        let idx_local = ctx.alloc_local("_char_at_idx".to_string());
                        ctx.emit(Instruction::LocalSet(idx_local));
                        // s はスタックトップ (i64) = ヒープオブジェクトのアドレス
                        // bytes_addr = s + 8 (tag=4bytes, len=4bytes をスキップ)
                        ctx.emit(Instruction::I32WrapI64);
                        ctx.emit(Instruction::I32Const(8));
                        ctx.emit(Instruction::I32Add);
                        // addr = bytes_addr + i
                        ctx.emit(Instruction::LocalGet(idx_local));
                        ctx.emit(Instruction::I32WrapI64);
                        ctx.emit(Instruction::I32Add);
                        // バイト値を読み出し
                        ctx.emit(Instruction::I32Load8U { offset: 0 });
                        // i32 -> i64 に拡張
                        ctx.emit(Instruction::I64ExtendI32U);
                    }
                    // substring: ヒープ上 String オブジェクトの [start, end) を部分文字列として返す
                    // 新しい String オブジェクト [tag=1, len, bytes] をヒープに確保
                    Expr::Var(_, name) if name == "substring" => {
                        if args.len() >= 3 {
                            self.lower_expr(ctx, &args[0])?; // s (String オブジェクトアドレス): i64
                            self.lower_expr(ctx, &args[1])?; // start: i64
                            self.lower_expr(ctx, &args[2])?; // end: i64
                        }
                        // end, start, s を一時ローカルに保存
                        let end_local = ctx.alloc_local("_substr_end".to_string());
                        ctx.emit(Instruction::LocalSet(end_local));
                        let start_local = ctx.alloc_local("_substr_start".to_string());
                        ctx.emit(Instruction::LocalSet(start_local));
                        let str_local = ctx.alloc_local("_substr_str".to_string());
                        ctx.emit(Instruction::LocalSet(str_local));
                        // new_len = end - start (i64)
                        let new_len_local = ctx.alloc_local("_substr_len".to_string());
                        ctx.emit(Instruction::LocalGet(end_local));
                        ctx.emit(Instruction::LocalGet(start_local));
                        ctx.emit(Instruction::I64Sub);
                        ctx.emit(Instruction::LocalSet(new_len_local));
                        // new_obj = __alloc(8 + new_len) -- tag(4) + len(4) + bytes
                        let obj_local = ctx.alloc_local("_substr_obj".to_string());
                        ctx.emit(Instruction::LocalGet(new_len_local));
                        ctx.emit(Instruction::I64Const(8));
                        ctx.emit(Instruction::I64Add);
                        let alloc_idx = *self.func_indices.get("__alloc").ok_or_else(|| {
                            LowerError::UndefinedFunction { name: "__alloc".to_string() }
                        })?;
                        ctx.emit(Instruction::Call(alloc_idx));
                        ctx.emit(Instruction::LocalSet(obj_local));
                        // tag = 1 を書き込み (obj + 0)
                        ctx.emit(Instruction::LocalGet(obj_local));
                        ctx.emit(Instruction::I32WrapI64);
                        ctx.emit(Instruction::I32Const(1));
                        ctx.emit(Instruction::I32Store { offset: 0 });
                        // len を書き込み (obj + 4)
                        ctx.emit(Instruction::LocalGet(obj_local));
                        ctx.emit(Instruction::I32WrapI64);
                        ctx.emit(Instruction::LocalGet(new_len_local));
                        ctx.emit(Instruction::I32WrapI64);
                        ctx.emit(Instruction::I32Store { offset: 4 });
                        // memory.copy(obj + 8, src + 8 + start, new_len)
                        // dst: obj + 8
                        ctx.emit(Instruction::LocalGet(obj_local));
                        ctx.emit(Instruction::I32WrapI64);
                        ctx.emit(Instruction::I32Const(8));
                        ctx.emit(Instruction::I32Add);
                        // src: s + 8 + start
                        ctx.emit(Instruction::LocalGet(str_local));
                        ctx.emit(Instruction::I32WrapI64);
                        ctx.emit(Instruction::I32Const(8));
                        ctx.emit(Instruction::I32Add);
                        ctx.emit(Instruction::LocalGet(start_local));
                        ctx.emit(Instruction::I32WrapI64);
                        ctx.emit(Instruction::I32Add);
                        // len: new_len (i32)
                        ctx.emit(Instruction::LocalGet(new_len_local));
                        ctx.emit(Instruction::I32WrapI64);
                        ctx.emit(Instruction::MemoryCopy);
                        // 新しい String オブジェクトのアドレスを返す
                        ctx.emit(Instruction::LocalGet(obj_local));
                    }
                    // string-length: ヒープ上 String オブジェクトの len フィールドを取得
                    // String オブジェクト: [tag:i32=1][len:i32][bytes:u8*]
                    Expr::Var(_, name) if name == "string-length" => {
                        if let Some(arg) = args.first() {
                            self.lower_expr(ctx, arg)?;
                        }
                        // s はスタックトップ (i64) = ヒープオブジェクトのアドレス
                        // len = i32.load(s + 4)
                        ctx.emit(Instruction::I32WrapI64);
                        ctx.emit(Instruction::I32Load { offset: 4 });
                        ctx.emit(Instruction::I64ExtendI32U);
                    }
                    // string-concat: 2 つの文字列を結合
                    Expr::Var(_, name) if name == "string-concat" => {
                        if args.len() >= 2 {
                            self.lower_expr(ctx, &args[0])?;
                            self.lower_expr(ctx, &args[1])?;
                        }
                        let idx = *self.func_indices.get("__string_concat").ok_or_else(|| {
                            LowerError::UndefinedFunction { name: "__string_concat".to_string() }
                        })?;
                        ctx.emit(Instruction::Call(idx));
                    }
                    // string-eq: 2 つの文字列を比較
                    Expr::Var(_, name) if name == "string-eq" => {
                        if args.len() >= 2 {
                            self.lower_expr(ctx, &args[0])?;
                            self.lower_expr(ctx, &args[1])?;
                        }
                        let idx = *self.func_indices.get("__string_eq").ok_or_else(|| {
                            LowerError::UndefinedFunction { name: "__string_eq".to_string() }
                        })?;
                        ctx.emit(Instruction::Call(idx));
                    }
                    // int-to-string: 整数を文字列に変換
                    Expr::Var(_, name) if name == "int-to-string" => {
                        if let Some(arg) = args.first() {
                            self.lower_expr(ctx, arg)?;
                        }
                        let idx = *self.func_indices.get("__int_to_string").ok_or_else(|| {
                            LowerError::UndefinedFunction { name: "__int_to_string".to_string() }
                        })?;
                        ctx.emit(Instruction::Call(idx));
                    }
                    // ref-new: ヒープに Ref Cell を確保して値を格納
                    // レイアウト: [tag=7: i32, _pad: i32, value: i64]
                    // 合計 16 バイト
                    Expr::Var(_, name) if name == "ref-new" => {
                        // 引数 (値) を評価
                        if let Some(arg) = args.first() {
                            self.lower_expr(ctx, arg)?;
                        }
                        // 値を一時ローカルに保存
                        let val_local = ctx.alloc_local("_ref_val".to_string());
                        ctx.emit(Instruction::LocalSet(val_local));
                        // __alloc(16) でヒープ確保
                        ctx.emit(Instruction::I64Const(16));
                        let alloc_idx = *self.func_indices.get("__alloc").ok_or_else(|| {
                            LowerError::UndefinedFunction { name: "__alloc".to_string() }
                        })?;
                        ctx.emit(Instruction::Call(alloc_idx));
                        // アドレスを i64 のままローカルに保存
                        let addr_local = ctx.alloc_local("_ref_addr".to_string());
                        ctx.emit(Instruction::LocalSet(addr_local));
                        // ヘッダ書き込み: mem[addr+0] = tag=7
                        ctx.emit(Instruction::LocalGet(addr_local));
                        ctx.emit(Instruction::I32WrapI64);
                        ctx.emit(Instruction::I32Const(super::HEAP_TAG_REF));
                        ctx.emit(Instruction::I32Store { offset: 0 });
                        // ヘッダ書き込み: mem[addr+4] = size=16
                        ctx.emit(Instruction::LocalGet(addr_local));
                        ctx.emit(Instruction::I32WrapI64);
                        ctx.emit(Instruction::I32Const(16));
                        ctx.emit(Instruction::I32Store { offset: 4 });
                        // 値書き込み: mem[addr+8] = value
                        ctx.emit(Instruction::LocalGet(addr_local));
                        ctx.emit(Instruction::I32WrapI64);
                        ctx.emit(Instruction::LocalGet(val_local));
                        ctx.emit(Instruction::I64Store { offset: 8 });
                        // タグ付きポインタを返す (addr は既に i64)
                        // 最上位ビットをセット: addr | (1 << 63)
                        ctx.emit(Instruction::LocalGet(addr_local));
                        ctx.emit(Instruction::I64Const(1i64 << 63));
                        ctx.emit(Instruction::I64Add);
                    }
                    // ref-get: Ref Cell から値を読み出す
                    Expr::Var(_, name) if name == "ref-get" => {
                        // 引数 (Ref ポインタ) を評価
                        if let Some(arg) = args.first() {
                            self.lower_expr(ctx, arg)?;
                        }
                        // タグ解除してアドレスを取得
                        super::emit_untag_pointer(&mut ctx.instructions);
                        // mem[addr+8] から値を読み出す
                        ctx.emit(Instruction::I64Load { offset: 8 });
                    }
                    // ref-set: Ref Cell に値を書き込む
                    Expr::Var(_, name) if name == "ref-set" => {
                        if args.len() >= 2 {
                            // 第1引数 (Ref ポインタ) を評価
                            self.lower_expr(ctx, &args[0])?;
                            // タグ解除してアドレスを取得
                            super::emit_untag_pointer(&mut ctx.instructions);
                            // 第2引数 (新しい値) を評価
                            self.lower_expr(ctx, &args[1])?;
                            // mem[addr+8] = new_value
                            ctx.emit(Instruction::I64Store { offset: 8 });
                            // Unit を返す
                            ctx.emit(Instruction::I64Const(0));
                        }
                    }
                    // === Vector (可変長配列) ビルトイン ===

                    // vector-new: 指定 capacity で空ベクタを確保
                    // レイアウト: [tag=5: i32, capacity: i32, length: i32, padding: i32, elem_0: i64, ...]
                    // ヘッダ 16 バイト + 各要素 8 バイト
                    Expr::Var(_, name) if name == "vector-new" => {
                        // 引数: capacity (i64)
                        if let Some(arg) = args.first() {
                            self.lower_expr(ctx, arg)?;
                        }
                        // capacity を i64 のままローカルに保存
                        let cap_local = ctx.alloc_local("_vec_cap".to_string());
                        ctx.emit(Instruction::LocalSet(cap_local));
                        // 割り当てサイズ = 16 + capacity * 8 (i64 算術)
                        ctx.emit(Instruction::I64Const(16));
                        ctx.emit(Instruction::LocalGet(cap_local));
                        ctx.emit(Instruction::I64Const(8));
                        ctx.emit(Instruction::I64Mul);
                        ctx.emit(Instruction::I64Add);
                        // __alloc 呼び出し (i64 引数)
                        let alloc_idx = *self.func_indices.get("__alloc").ok_or_else(|| {
                            LowerError::UndefinedFunction { name: "__alloc".to_string() }
                        })?;
                        ctx.emit(Instruction::Call(alloc_idx));
                        // アドレスをローカルに保存 (i64)
                        let addr_local = ctx.alloc_local("_vec_addr".to_string());
                        ctx.emit(Instruction::LocalSet(addr_local));
                        // tag=5 書き込み: mem[addr+0] = HEAP_TAG_VECTOR
                        ctx.emit(Instruction::LocalGet(addr_local));
                        ctx.emit(Instruction::I32WrapI64);
                        ctx.emit(Instruction::I32Const(super::HEAP_TAG_VECTOR));
                        ctx.emit(Instruction::I32Store { offset: 0 });
                        // capacity 書き込み: mem[addr+4] = capacity
                        ctx.emit(Instruction::LocalGet(addr_local));
                        ctx.emit(Instruction::I32WrapI64);
                        ctx.emit(Instruction::LocalGet(cap_local));
                        ctx.emit(Instruction::I32WrapI64);
                        ctx.emit(Instruction::I32Store { offset: 4 });
                        // length=0 書き込み: mem[addr+8] = 0
                        ctx.emit(Instruction::LocalGet(addr_local));
                        ctx.emit(Instruction::I32WrapI64);
                        ctx.emit(Instruction::I32Const(0));
                        ctx.emit(Instruction::I32Store { offset: 8 });
                        // padding=0 書き込み: mem[addr+12] = 0
                        ctx.emit(Instruction::LocalGet(addr_local));
                        ctx.emit(Instruction::I32WrapI64);
                        ctx.emit(Instruction::I32Const(0));
                        ctx.emit(Instruction::I32Store { offset: 12 });
                        // タグ付きポインタを返す
                        ctx.emit(Instruction::LocalGet(addr_local));
                        ctx.emit(Instruction::I64Const(1i64 << 63));
                        ctx.emit(Instruction::I64Add);
                    }

                    // vector-length: ベクタの現在の長さを返す
                    Expr::Var(_, name) if name == "vector-length" => {
                        if let Some(arg) = args.first() {
                            self.lower_expr(ctx, arg)?;
                        }
                        // タグ解除してアドレスを取得
                        ctx.emit(Instruction::I32WrapI64);
                        // mem[addr+8] から length を読み出す
                        ctx.emit(Instruction::I32Load { offset: 8 });
                        // i32 -> i64 に拡張
                        ctx.emit(Instruction::I64ExtendI32U);
                    }

                    // vector-get: インデックス指定で要素を取得
                    // vector-get [v i] -> a
                    Expr::Var(_, name) if name == "vector-get" => {
                        if args.len() >= 2 {
                            // 第1引数: ベクタ (tagged pointer) → i64 のまま保持
                            self.lower_expr(ctx, &args[0])?;
                            let addr_local = ctx.alloc_local("_vget_addr".to_string());
                            ctx.emit(Instruction::LocalSet(addr_local));
                            // 第2引数: インデックス (i64) → i32 に変換して計算
                            self.lower_expr(ctx, &args[1])?;
                            ctx.emit(Instruction::I32WrapI64);
                            // 要素のオフセット = 16 + i * 8
                            ctx.emit(Instruction::I32Const(8));
                            ctx.emit(Instruction::I32Mul);
                            ctx.emit(Instruction::I32Const(16));
                            ctx.emit(Instruction::I32Add);
                            // addr (i64 → i32) + offset
                            ctx.emit(Instruction::LocalGet(addr_local));
                            ctx.emit(Instruction::I32WrapI64);
                            ctx.emit(Instruction::I32Add);
                            // i64 値を読み出す
                            ctx.emit(Instruction::I64Load { offset: 0 });
                        }
                    }

                    // vector-set: インデックス指定で要素を上書き (ミューテーション)
                    // vector-set [v i x] -> Vector (同じベクタを返す)
                    Expr::Var(_, name) if name == "vector-set" => {
                        if args.len() >= 3 {
                            // 第1引数: ベクタ (tagged pointer) → i64 のまま保持
                            self.lower_expr(ctx, &args[0])?;
                            let tagged_local = ctx.alloc_local("_vset_tagged".to_string());
                            ctx.emit(Instruction::LocalSet(tagged_local));
                            // 第2引数: インデックス (i64) → i32 に変換
                            self.lower_expr(ctx, &args[1])?;
                            ctx.emit(Instruction::I32WrapI64);
                            // 要素のオフセット = 16 + i * 8
                            ctx.emit(Instruction::I32Const(8));
                            ctx.emit(Instruction::I32Mul);
                            ctx.emit(Instruction::I32Const(16));
                            ctx.emit(Instruction::I32Add);
                            // addr (i64 → i32) + offset
                            ctx.emit(Instruction::LocalGet(tagged_local));
                            ctx.emit(Instruction::I32WrapI64);
                            ctx.emit(Instruction::I32Add);
                            // 第3引数: 新しい値 (i64)
                            self.lower_expr(ctx, &args[2])?;
                            // mem[elem_addr] = value
                            ctx.emit(Instruction::I64Store { offset: 0 });
                            // 同じタグ付きポインタを返す
                            ctx.emit(Instruction::LocalGet(tagged_local));
                        }
                    }

                    // vector-push: 要素を末尾に追加 (capacity 超過時は再割り当て)
                    // vector-push [v x] -> Vector
                    // 注意: すべてのローカル変数は i64 型で保持
                    Expr::Var(_, name) if name == "vector-push" => {
                        if args.len() >= 2 {
                            // 第1引数: ベクタ (tagged pointer) → i64
                            self.lower_expr(ctx, &args[0])?;
                            let tagged_local = ctx.alloc_local("_vpush_tagged".to_string());
                            ctx.emit(Instruction::LocalSet(tagged_local));
                            // 第2引数: 追加する値 → i64
                            self.lower_expr(ctx, &args[1])?;
                            let val_local = ctx.alloc_local("_vpush_val".to_string());
                            ctx.emit(Instruction::LocalSet(val_local));

                            // length を読み出して i64 で保存
                            let len_local = ctx.alloc_local("_vpush_len".to_string());
                            ctx.emit(Instruction::LocalGet(tagged_local));
                            ctx.emit(Instruction::I32WrapI64); // untag
                            ctx.emit(Instruction::I32Load { offset: 8 });
                            ctx.emit(Instruction::I64ExtendI32U);
                            ctx.emit(Instruction::LocalSet(len_local));

                            // capacity を読み出して i64 で保存
                            let cap_local = ctx.alloc_local("_vpush_cap".to_string());
                            ctx.emit(Instruction::LocalGet(tagged_local));
                            ctx.emit(Instruction::I32WrapI64); // untag
                            ctx.emit(Instruction::I32Load { offset: 4 });
                            ctx.emit(Instruction::I64ExtendI32U);
                            ctx.emit(Instruction::LocalSet(cap_local));

                            // if length >= capacity then 再割り当て else 既存バッファに追加
                            ctx.emit(Instruction::LocalGet(len_local));
                            ctx.emit(Instruction::LocalGet(cap_local));
                            ctx.emit(Instruction::I64GeS);
                            ctx.emit(Instruction::If(IrType::I64)); // 結果: 新しいタグ付きポインタ (i64)

                            // === 再割り当てブランチ ===
                            {
                                // new_cap = max(capacity * 2, 4) (i64 演算)
                                let new_cap_local = ctx.alloc_local("_vpush_newcap".to_string());
                                ctx.emit(Instruction::LocalGet(cap_local));
                                ctx.emit(Instruction::I64Const(2));
                                ctx.emit(Instruction::I64Mul);
                                let tmp_local = ctx.alloc_local("_vpush_tmp".to_string());
                                ctx.emit(Instruction::LocalSet(tmp_local));
                                ctx.emit(Instruction::LocalGet(tmp_local));
                                ctx.emit(Instruction::I64Const(4));
                                ctx.emit(Instruction::I64GtS);
                                ctx.emit(Instruction::If(IrType::I64));
                                ctx.emit(Instruction::LocalGet(tmp_local));
                                ctx.emit(Instruction::Else);
                                ctx.emit(Instruction::I64Const(4));
                                ctx.emit(Instruction::End);
                                ctx.emit(Instruction::LocalSet(new_cap_local));

                                // alloc_size = 16 + new_cap * 8 (i64)
                                ctx.emit(Instruction::I64Const(16));
                                ctx.emit(Instruction::LocalGet(new_cap_local));
                                ctx.emit(Instruction::I64Const(8));
                                ctx.emit(Instruction::I64Mul);
                                ctx.emit(Instruction::I64Add);
                                let alloc_idx = *self.func_indices.get("__alloc").ok_or_else(|| {
                                    LowerError::UndefinedFunction { name: "__alloc".to_string() }
                                })?;
                                ctx.emit(Instruction::Call(alloc_idx));
                                let new_addr_local = ctx.alloc_local("_vpush_newaddr".to_string());
                                ctx.emit(Instruction::LocalSet(new_addr_local));

                                // 新しいヘッダを書き込む
                                // tag=5
                                ctx.emit(Instruction::LocalGet(new_addr_local));
                                ctx.emit(Instruction::I32WrapI64);
                                ctx.emit(Instruction::I32Const(super::HEAP_TAG_VECTOR));
                                ctx.emit(Instruction::I32Store { offset: 0 });
                                // capacity = new_cap
                                ctx.emit(Instruction::LocalGet(new_addr_local));
                                ctx.emit(Instruction::I32WrapI64);
                                ctx.emit(Instruction::LocalGet(new_cap_local));
                                ctx.emit(Instruction::I32WrapI64);
                                ctx.emit(Instruction::I32Store { offset: 4 });
                                // length = old_len + 1
                                ctx.emit(Instruction::LocalGet(new_addr_local));
                                ctx.emit(Instruction::I32WrapI64);
                                ctx.emit(Instruction::LocalGet(len_local));
                                ctx.emit(Instruction::I32WrapI64);
                                ctx.emit(Instruction::I32Const(1));
                                ctx.emit(Instruction::I32Add);
                                ctx.emit(Instruction::I32Store { offset: 8 });
                                // padding = 0
                                ctx.emit(Instruction::LocalGet(new_addr_local));
                                ctx.emit(Instruction::I32WrapI64);
                                ctx.emit(Instruction::I32Const(0));
                                ctx.emit(Instruction::I32Store { offset: 12 });

                                // 既存要素をコピー: memory.copy(dst, src, byte_count)
                                // dst = new_addr + 16
                                ctx.emit(Instruction::LocalGet(new_addr_local));
                                ctx.emit(Instruction::I32WrapI64);
                                ctx.emit(Instruction::I32Const(16));
                                ctx.emit(Instruction::I32Add);
                                // src = old_addr + 16 (untag)
                                ctx.emit(Instruction::LocalGet(tagged_local));
                                ctx.emit(Instruction::I32WrapI64); // untag
                                ctx.emit(Instruction::I32Const(16));
                                ctx.emit(Instruction::I32Add);
                                // count = len * 8
                                ctx.emit(Instruction::LocalGet(len_local));
                                ctx.emit(Instruction::I32WrapI64);
                                ctx.emit(Instruction::I32Const(8));
                                ctx.emit(Instruction::I32Mul);
                                ctx.emit(Instruction::MemoryCopy);

                                // 新要素を書き込み: mem[new_addr + 16 + len * 8] = val
                                ctx.emit(Instruction::LocalGet(new_addr_local));
                                ctx.emit(Instruction::I32WrapI64);
                                ctx.emit(Instruction::LocalGet(len_local));
                                ctx.emit(Instruction::I32WrapI64);
                                ctx.emit(Instruction::I32Const(8));
                                ctx.emit(Instruction::I32Mul);
                                ctx.emit(Instruction::I32Const(16));
                                ctx.emit(Instruction::I32Add);
                                ctx.emit(Instruction::I32Add);
                                ctx.emit(Instruction::LocalGet(val_local));
                                ctx.emit(Instruction::I64Store { offset: 0 });

                                // 新しいタグ付きポインタを返す
                                ctx.emit(Instruction::LocalGet(new_addr_local));
                                ctx.emit(Instruction::I64Const(1i64 << 63));
                                ctx.emit(Instruction::I64Add);
                            }

                            ctx.emit(Instruction::Else);

                            // === 既存バッファに追加ブランチ ===
                            {
                                // 新要素を書き込み: mem[untag(addr) + 16 + len * 8] = val
                                ctx.emit(Instruction::LocalGet(tagged_local));
                                ctx.emit(Instruction::I32WrapI64); // untag
                                ctx.emit(Instruction::LocalGet(len_local));
                                ctx.emit(Instruction::I32WrapI64);
                                ctx.emit(Instruction::I32Const(8));
                                ctx.emit(Instruction::I32Mul);
                                ctx.emit(Instruction::I32Const(16));
                                ctx.emit(Instruction::I32Add);
                                ctx.emit(Instruction::I32Add);
                                ctx.emit(Instruction::LocalGet(val_local));
                                ctx.emit(Instruction::I64Store { offset: 0 });

                                // length を更新: mem[untag(addr)+8] = len + 1
                                ctx.emit(Instruction::LocalGet(tagged_local));
                                ctx.emit(Instruction::I32WrapI64); // untag
                                ctx.emit(Instruction::LocalGet(len_local));
                                ctx.emit(Instruction::I32WrapI64);
                                ctx.emit(Instruction::I32Const(1));
                                ctx.emit(Instruction::I32Add);
                                ctx.emit(Instruction::I32Store { offset: 8 });

                                // 同じタグ付きポインタを返す
                                ctx.emit(Instruction::LocalGet(tagged_local));
                            }

                            ctx.emit(Instruction::End);
                        }
                    }

                    // === HashMap ビルトイン ===

                    // map-new: デフォルト容量 16 で空のハッシュマップを作成
                    // レイアウト: [tag=6: i32, capacity: i32, size: i32, padding: i32, entries...]
                    // エントリ: [key: i64, value: i64] (16バイト) — key=0 は空スロット
                    // 合計: 16 + 16 * 16 = 272 bytes
                    Expr::Var(_, name) if name == "map-new" => {
                        let default_cap: i32 = 16;
                        let alloc_size: i64 = 16 + (default_cap as i64) * 16; // 272
                        ctx.emit(Instruction::I64Const(alloc_size));
                        let alloc_idx = *self.func_indices.get("__alloc").ok_or_else(|| {
                            LowerError::UndefinedFunction { name: "__alloc".to_string() }
                        })?;
                        ctx.emit(Instruction::Call(alloc_idx));
                        let addr_local = ctx.alloc_local("_map_addr".to_string());
                        ctx.emit(Instruction::LocalSet(addr_local));
                        // tag=6
                        ctx.emit(Instruction::LocalGet(addr_local));
                        ctx.emit(Instruction::I32WrapI64);
                        ctx.emit(Instruction::I32Const(super::HEAP_TAG_HASHMAP));
                        ctx.emit(Instruction::I32Store { offset: 0 });
                        // capacity
                        ctx.emit(Instruction::LocalGet(addr_local));
                        ctx.emit(Instruction::I32WrapI64);
                        ctx.emit(Instruction::I32Const(default_cap));
                        ctx.emit(Instruction::I32Store { offset: 4 });
                        // size=0
                        ctx.emit(Instruction::LocalGet(addr_local));
                        ctx.emit(Instruction::I32WrapI64);
                        ctx.emit(Instruction::I32Const(0));
                        ctx.emit(Instruction::I32Store { offset: 8 });
                        // エントリ領域をゼロ初期化 (key=0 は空スロット)
                        ctx.emit(Instruction::LocalGet(addr_local));
                        ctx.emit(Instruction::I32WrapI64);
                        ctx.emit(Instruction::I32Const(16)); // ヘッダスキップ
                        ctx.emit(Instruction::I32Add);
                        ctx.emit(Instruction::I32Const(0)); // fill value = 0
                        ctx.emit(Instruction::I32Const((default_cap * 16) as i32)); // 256 bytes
                        ctx.emit(Instruction::MemoryFill);
                        // タグ付きポインタを返す
                        ctx.emit(Instruction::LocalGet(addr_local));
                        ctx.emit(Instruction::I64Const(1i64 << 63));
                        ctx.emit(Instruction::I64Add);
                    }

                    // map-size: ハッシュマップのエントリ数を返す
                    Expr::Var(_, name) if name == "map-size" => {
                        if let Some(arg) = args.first() {
                            self.lower_expr(ctx, arg)?;
                        }
                        // タグ解除してアドレスを取得
                        ctx.emit(Instruction::I32WrapI64);
                        // mem[addr+8] から size を読み出す
                        ctx.emit(Instruction::I32Load { offset: 8 });
                        // i32 -> i64 に拡張
                        ctx.emit(Instruction::I64ExtendI32U);
                    }

                    // map-insert [m key value] -> Map
                    // 線形探索: key=0 は空スロット、16バイト/エントリ
                    Expr::Var(_, name) if name == "map-insert" => {
                        if args.len() >= 3 {
                            self.lower_expr(ctx, &args[0])?;
                            let tagged_local = ctx.alloc_local("_mi_tagged".to_string());
                            ctx.emit(Instruction::LocalSet(tagged_local));
                            let addr_local = ctx.alloc_local("_mi_addr".to_string());
                            ctx.emit(Instruction::LocalGet(tagged_local));
                            ctx.emit(Instruction::I32WrapI64);
                            ctx.emit(Instruction::I64ExtendI32U);
                            ctx.emit(Instruction::LocalSet(addr_local));
                            self.lower_expr(ctx, &args[1])?;
                            // 文字列キーの場合は FNV-1a ハッシュに変換
                            self.emit_string_key_hash(ctx, &args[1])?;
                            let key_local = ctx.alloc_local("_mi_key".to_string());
                            ctx.emit(Instruction::LocalSet(key_local));
                            self.lower_expr(ctx, &args[2])?;
                            let val_local = ctx.alloc_local("_mi_val".to_string());
                            ctx.emit(Instruction::LocalSet(val_local));
                            let cap_local = ctx.alloc_local("_mi_cap".to_string());
                            ctx.emit(Instruction::LocalGet(addr_local));
                            ctx.emit(Instruction::I32WrapI64);
                            ctx.emit(Instruction::I32Load { offset: 4 });
                            ctx.emit(Instruction::I64ExtendI32U);
                            ctx.emit(Instruction::LocalSet(cap_local));
                            let i_local = ctx.alloc_local("_mi_i".to_string());
                            ctx.emit(Instruction::I64Const(0));
                            ctx.emit(Instruction::LocalSet(i_local));
                            ctx.emit(Instruction::BlockEmpty);
                            ctx.emit(Instruction::LoopEmpty);
                            // if i >= cap → break
                            ctx.emit(Instruction::LocalGet(i_local));
                            ctx.emit(Instruction::LocalGet(cap_local));
                            ctx.emit(Instruction::I64GeS);
                            ctx.emit(Instruction::BrIf(1));
                            // entry_addr = addr + 16 + i * 16
                            let ea_local = ctx.alloc_local("_mi_ea".to_string());
                            ctx.emit(Instruction::LocalGet(addr_local));
                            ctx.emit(Instruction::I64Const(16));
                            ctx.emit(Instruction::I64Add);
                            ctx.emit(Instruction::LocalGet(i_local));
                            ctx.emit(Instruction::I64Const(16));
                            ctx.emit(Instruction::I64Mul);
                            ctx.emit(Instruction::I64Add);
                            ctx.emit(Instruction::LocalSet(ea_local));
                            let ek_local = ctx.alloc_local("_mi_ek".to_string());
                            ctx.emit(Instruction::LocalGet(ea_local));
                            ctx.emit(Instruction::I32WrapI64);
                            ctx.emit(Instruction::I64Load { offset: 0 });
                            ctx.emit(Instruction::LocalSet(ek_local));
                            // if entry_key == 0 → 新規挿入
                            ctx.emit(Instruction::LocalGet(ek_local));
                            ctx.emit(Instruction::I64Const(0));
                            ctx.emit(Instruction::I64Eq);
                            ctx.emit(Instruction::IfEmpty);
                            ctx.emit(Instruction::LocalGet(ea_local));
                            ctx.emit(Instruction::I32WrapI64);
                            ctx.emit(Instruction::LocalGet(key_local));
                            ctx.emit(Instruction::I64Store { offset: 0 });
                            ctx.emit(Instruction::LocalGet(ea_local));
                            ctx.emit(Instruction::I32WrapI64);
                            ctx.emit(Instruction::LocalGet(val_local));
                            ctx.emit(Instruction::I64Store { offset: 8 });
                            // size++
                            ctx.emit(Instruction::LocalGet(addr_local));
                            ctx.emit(Instruction::I32WrapI64);
                            ctx.emit(Instruction::LocalGet(addr_local));
                            ctx.emit(Instruction::I32WrapI64);
                            ctx.emit(Instruction::I32Load { offset: 8 });
                            ctx.emit(Instruction::I32Const(1));
                            ctx.emit(Instruction::I32Add);
                            ctx.emit(Instruction::I32Store { offset: 8 });
                            ctx.emit(Instruction::Br(2)); // block を抜ける
                            ctx.emit(Instruction::End); // end if
                            // if entry_key == key → 上書き
                            ctx.emit(Instruction::LocalGet(ek_local));
                            ctx.emit(Instruction::LocalGet(key_local));
                            ctx.emit(Instruction::I64Eq);
                            ctx.emit(Instruction::IfEmpty);
                            ctx.emit(Instruction::LocalGet(ea_local));
                            ctx.emit(Instruction::I32WrapI64);
                            ctx.emit(Instruction::LocalGet(val_local));
                            ctx.emit(Instruction::I64Store { offset: 8 });
                            ctx.emit(Instruction::Br(2)); // block を抜ける
                            ctx.emit(Instruction::End); // end if
                            // i++
                            ctx.emit(Instruction::LocalGet(i_local));
                            ctx.emit(Instruction::I64Const(1));
                            ctx.emit(Instruction::I64Add);
                            ctx.emit(Instruction::LocalSet(i_local));
                            ctx.emit(Instruction::Br(0)); // loop continue
                            ctx.emit(Instruction::End); // end loop
                            ctx.emit(Instruction::End); // end block
                            ctx.emit(Instruction::LocalGet(tagged_local));
                        }
                    }

                    // map-get [m key] -> a (未存在時は 0)
                    Expr::Var(_, name) if name == "map-get" => {
                        if args.len() >= 2 {
                            self.lower_expr(ctx, &args[0])?;
                            let addr_local = ctx.alloc_local("_mg_addr".to_string());
                            ctx.emit(Instruction::I32WrapI64);
                            ctx.emit(Instruction::I64ExtendI32U);
                            ctx.emit(Instruction::LocalSet(addr_local));
                            self.lower_expr(ctx, &args[1])?;
                            // 文字列キーの場合は FNV-1a ハッシュに変換
                            self.emit_string_key_hash(ctx, &args[1])?;
                            let key_local = ctx.alloc_local("_mg_key".to_string());
                            ctx.emit(Instruction::LocalSet(key_local));
                            let cap_local = ctx.alloc_local("_mg_cap".to_string());
                            ctx.emit(Instruction::LocalGet(addr_local));
                            ctx.emit(Instruction::I32WrapI64);
                            ctx.emit(Instruction::I32Load { offset: 4 });
                            ctx.emit(Instruction::I64ExtendI32U);
                            ctx.emit(Instruction::LocalSet(cap_local));
                            let result_local = ctx.alloc_local("_mg_result".to_string());
                            ctx.emit(Instruction::I64Const(0));
                            ctx.emit(Instruction::LocalSet(result_local));
                            let i_local = ctx.alloc_local("_mg_i".to_string());
                            ctx.emit(Instruction::I64Const(0));
                            ctx.emit(Instruction::LocalSet(i_local));
                            ctx.emit(Instruction::BlockEmpty);
                            ctx.emit(Instruction::LoopEmpty);
                            ctx.emit(Instruction::LocalGet(i_local));
                            ctx.emit(Instruction::LocalGet(cap_local));
                            ctx.emit(Instruction::I64GeS);
                            ctx.emit(Instruction::BrIf(1));
                            let ea_local = ctx.alloc_local("_mg_ea".to_string());
                            ctx.emit(Instruction::LocalGet(addr_local));
                            ctx.emit(Instruction::I64Const(16));
                            ctx.emit(Instruction::I64Add);
                            ctx.emit(Instruction::LocalGet(i_local));
                            ctx.emit(Instruction::I64Const(16));
                            ctx.emit(Instruction::I64Mul);
                            ctx.emit(Instruction::I64Add);
                            ctx.emit(Instruction::LocalSet(ea_local));
                            let ek_local = ctx.alloc_local("_mg_ek".to_string());
                            ctx.emit(Instruction::LocalGet(ea_local));
                            ctx.emit(Instruction::I32WrapI64);
                            ctx.emit(Instruction::I64Load { offset: 0 });
                            ctx.emit(Instruction::LocalSet(ek_local));
                            // found
                            ctx.emit(Instruction::LocalGet(ek_local));
                            ctx.emit(Instruction::LocalGet(key_local));
                            ctx.emit(Instruction::I64Eq);
                            ctx.emit(Instruction::IfEmpty);
                            ctx.emit(Instruction::LocalGet(ea_local));
                            ctx.emit(Instruction::I32WrapI64);
                            ctx.emit(Instruction::I64Load { offset: 8 });
                            ctx.emit(Instruction::LocalSet(result_local));
                            ctx.emit(Instruction::Br(2));
                            ctx.emit(Instruction::End);
                            // i++ (key=0 の空スロットもスキップ — 削除後の穴を越えて探索)
                            ctx.emit(Instruction::LocalGet(i_local));
                            ctx.emit(Instruction::I64Const(1));
                            ctx.emit(Instruction::I64Add);
                            ctx.emit(Instruction::LocalSet(i_local));
                            ctx.emit(Instruction::Br(0));
                            ctx.emit(Instruction::End); // loop
                            ctx.emit(Instruction::End); // block
                            ctx.emit(Instruction::LocalGet(result_local));
                        }
                    }

                    // map-contains? [m key] -> Bool (1=存在, 0=不存在)
                    Expr::Var(_, name) if name == "map-contains?" => {
                        if args.len() >= 2 {
                            self.lower_expr(ctx, &args[0])?;
                            let addr_local = ctx.alloc_local("_mc_addr".to_string());
                            ctx.emit(Instruction::I32WrapI64);
                            ctx.emit(Instruction::I64ExtendI32U);
                            ctx.emit(Instruction::LocalSet(addr_local));
                            self.lower_expr(ctx, &args[1])?;
                            // 文字列キーの場合は FNV-1a ハッシュに変換
                            self.emit_string_key_hash(ctx, &args[1])?;
                            let key_local = ctx.alloc_local("_mc_key".to_string());
                            ctx.emit(Instruction::LocalSet(key_local));
                            let cap_local = ctx.alloc_local("_mc_cap".to_string());
                            ctx.emit(Instruction::LocalGet(addr_local));
                            ctx.emit(Instruction::I32WrapI64);
                            ctx.emit(Instruction::I32Load { offset: 4 });
                            ctx.emit(Instruction::I64ExtendI32U);
                            ctx.emit(Instruction::LocalSet(cap_local));
                            let result_local = ctx.alloc_local("_mc_result".to_string());
                            ctx.emit(Instruction::I64Const(0));
                            ctx.emit(Instruction::LocalSet(result_local));
                            let i_local = ctx.alloc_local("_mc_i".to_string());
                            ctx.emit(Instruction::I64Const(0));
                            ctx.emit(Instruction::LocalSet(i_local));
                            ctx.emit(Instruction::BlockEmpty);
                            ctx.emit(Instruction::LoopEmpty);
                            ctx.emit(Instruction::LocalGet(i_local));
                            ctx.emit(Instruction::LocalGet(cap_local));
                            ctx.emit(Instruction::I64GeS);
                            ctx.emit(Instruction::BrIf(1));
                            let ea_local = ctx.alloc_local("_mc_ea".to_string());
                            ctx.emit(Instruction::LocalGet(addr_local));
                            ctx.emit(Instruction::I64Const(16));
                            ctx.emit(Instruction::I64Add);
                            ctx.emit(Instruction::LocalGet(i_local));
                            ctx.emit(Instruction::I64Const(16));
                            ctx.emit(Instruction::I64Mul);
                            ctx.emit(Instruction::I64Add);
                            ctx.emit(Instruction::LocalSet(ea_local));
                            let ek_local = ctx.alloc_local("_mc_ek".to_string());
                            ctx.emit(Instruction::LocalGet(ea_local));
                            ctx.emit(Instruction::I32WrapI64);
                            ctx.emit(Instruction::I64Load { offset: 0 });
                            ctx.emit(Instruction::LocalSet(ek_local));
                            // found
                            ctx.emit(Instruction::LocalGet(ek_local));
                            ctx.emit(Instruction::LocalGet(key_local));
                            ctx.emit(Instruction::I64Eq);
                            ctx.emit(Instruction::IfEmpty);
                            ctx.emit(Instruction::I64Const(1));
                            ctx.emit(Instruction::LocalSet(result_local));
                            ctx.emit(Instruction::Br(2));
                            ctx.emit(Instruction::End);
                            // i++ (全スロット走査)
                            ctx.emit(Instruction::LocalGet(i_local));
                            ctx.emit(Instruction::I64Const(1));
                            ctx.emit(Instruction::I64Add);
                            ctx.emit(Instruction::LocalSet(i_local));
                            ctx.emit(Instruction::Br(0));
                            ctx.emit(Instruction::End); // loop
                            ctx.emit(Instruction::End); // block
                            ctx.emit(Instruction::LocalGet(result_local));
                        }
                    }

                    // map-remove [m key] -> Map
                    Expr::Var(_, name) if name == "map-remove" => {
                        if args.len() >= 2 {
                            self.lower_expr(ctx, &args[0])?;
                            let tagged_local = ctx.alloc_local("_mr_tagged".to_string());
                            ctx.emit(Instruction::LocalSet(tagged_local));
                            let addr_local = ctx.alloc_local("_mr_addr".to_string());
                            ctx.emit(Instruction::LocalGet(tagged_local));
                            ctx.emit(Instruction::I32WrapI64);
                            ctx.emit(Instruction::I64ExtendI32U);
                            ctx.emit(Instruction::LocalSet(addr_local));
                            self.lower_expr(ctx, &args[1])?;
                            // 文字列キーの場合は FNV-1a ハッシュに変換
                            self.emit_string_key_hash(ctx, &args[1])?;
                            let key_local = ctx.alloc_local("_mr_key".to_string());
                            ctx.emit(Instruction::LocalSet(key_local));
                            let cap_local = ctx.alloc_local("_mr_cap".to_string());
                            ctx.emit(Instruction::LocalGet(addr_local));
                            ctx.emit(Instruction::I32WrapI64);
                            ctx.emit(Instruction::I32Load { offset: 4 });
                            ctx.emit(Instruction::I64ExtendI32U);
                            ctx.emit(Instruction::LocalSet(cap_local));
                            let i_local = ctx.alloc_local("_mr_i".to_string());
                            ctx.emit(Instruction::I64Const(0));
                            ctx.emit(Instruction::LocalSet(i_local));
                            ctx.emit(Instruction::BlockEmpty);
                            ctx.emit(Instruction::LoopEmpty);
                            ctx.emit(Instruction::LocalGet(i_local));
                            ctx.emit(Instruction::LocalGet(cap_local));
                            ctx.emit(Instruction::I64GeS);
                            ctx.emit(Instruction::BrIf(1));
                            let ea_local = ctx.alloc_local("_mr_ea".to_string());
                            ctx.emit(Instruction::LocalGet(addr_local));
                            ctx.emit(Instruction::I64Const(16));
                            ctx.emit(Instruction::I64Add);
                            ctx.emit(Instruction::LocalGet(i_local));
                            ctx.emit(Instruction::I64Const(16));
                            ctx.emit(Instruction::I64Mul);
                            ctx.emit(Instruction::I64Add);
                            ctx.emit(Instruction::LocalSet(ea_local));
                            let ek_local = ctx.alloc_local("_mr_ek".to_string());
                            ctx.emit(Instruction::LocalGet(ea_local));
                            ctx.emit(Instruction::I32WrapI64);
                            ctx.emit(Instruction::I64Load { offset: 0 });
                            ctx.emit(Instruction::LocalSet(ek_local));
                            // found → key=-1 (tombstone), size--
                            ctx.emit(Instruction::LocalGet(ek_local));
                            ctx.emit(Instruction::LocalGet(key_local));
                            ctx.emit(Instruction::I64Eq);
                            ctx.emit(Instruction::IfEmpty);
                            ctx.emit(Instruction::LocalGet(ea_local));
                            ctx.emit(Instruction::I32WrapI64);
                            ctx.emit(Instruction::I64Const(-1i64)); // tombstone marker
                            ctx.emit(Instruction::I64Store { offset: 0 });
                            // size--
                            ctx.emit(Instruction::LocalGet(addr_local));
                            ctx.emit(Instruction::I32WrapI64);
                            ctx.emit(Instruction::LocalGet(addr_local));
                            ctx.emit(Instruction::I32WrapI64);
                            ctx.emit(Instruction::I32Load { offset: 8 });
                            ctx.emit(Instruction::I32Const(1));
                            ctx.emit(Instruction::I32Sub);
                            ctx.emit(Instruction::I32Store { offset: 8 });
                            ctx.emit(Instruction::Br(2));
                            ctx.emit(Instruction::End);
                            // empty (key==0) → not found, break
                            ctx.emit(Instruction::LocalGet(ek_local));
                            ctx.emit(Instruction::I64Const(0));
                            ctx.emit(Instruction::I64Eq);
                            ctx.emit(Instruction::BrIf(1));
                            // tombstone (key==-1) → skip, continue probing
                            ctx.emit(Instruction::LocalGet(i_local));
                            ctx.emit(Instruction::I64Const(1));
                            ctx.emit(Instruction::I64Add);
                            ctx.emit(Instruction::LocalSet(i_local));
                            ctx.emit(Instruction::Br(0));
                            ctx.emit(Instruction::End); // loop
                            ctx.emit(Instruction::End); // block
                            ctx.emit(Instruction::LocalGet(tagged_local));
                        }
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
                        if let Some(&idx) = self.func_indices.get(name.as_str()) {
                            // 既知の関数: 引数を評価して直接呼び出し
                            for arg in args {
                                self.lower_expr(ctx, arg)?;
                            }
                            ctx.emit(Instruction::Call(idx));
                        } else if let Some(&idx) = self.lifted_func_indices.get(name.as_str()) {
                            // Lambda Lifting で生成された関数の呼び出し
                            for arg in args {
                                self.lower_expr(ctx, arg)?;
                            }
                            ctx.emit(Instruction::Call(idx));
                        } else if let Some(idx) = self.resolve_trait_dispatch(name, args) {
                            // P5-6: トレイトメソッドの静的ディスパッチ自動解決
                            for arg in args {
                                self.lower_expr(ctx, arg)?;
                            }
                            ctx.emit(Instruction::Call(idx));
                        } else if let Some(&local_idx) = ctx.locals_map.get(name) {
                            // ローカル変数に格納されたクロージャの間接呼び出し
                            // 統一呼び出し規約: (元引数..., closure_ptr) -> result
                            // call_indirect のスタック: [arg0, ..., argN, closure_ptr, table_idx]

                            // 1. 元引数を評価してスタックに積む
                            for arg in args {
                                self.lower_expr(ctx, arg)?;
                            }
                            // 2. クロージャポインタをスタックに積む（リフト関数の最後のパラメータ）
                            ctx.emit(Instruction::LocalGet(local_idx));
                            // 3. テーブルインデックス (func_idx) を取得してスタックに積む
                            //    クロージャポインタからタグ解除して func_idx を読み出す
                            ctx.emit(Instruction::LocalGet(local_idx));
                            ctx.emit(Instruction::I32WrapI64);
                            ctx.emit(Instruction::I32Load { offset: 4 });
                            // 4. call_indirect: 型は (i64 * (args.len() + 1)) -> i64
                            let call_type_id = args.len() as u32 + 1; // 元引数 + closure_ptr
                            ctx.emit(Instruction::CallIndirect(call_type_id));
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

            Expr::Lambda(_, params, body) => {
                // Lambda Lifting: Lambda 式をトップレベル関数にリフト
                let lambda_name = self.fresh_lambda_name();

                // 自由変数を検出
                let param_names: Vec<String> = params.iter().map(|p| p.name.clone()).collect();
                let free_vars = closure::free_variables(&param_names, body);

                // 自由変数をソートして順序を安定させる
                // 組み込み関数や既存関数は自由変数としてキャプチャしない
                let mut free_var_list: Vec<String> = free_vars.into_iter()
                    .filter(|v| {
                        !is_builtin_binop(v)
                            && v != "not" && v != "print" && v != "__alloc" && v != "proc-exit"
                            && !self.func_indices.contains_key(v)
                    })
                    .collect();
                free_var_list.sort();

                // リフト先関数を構築:
                // 統一呼び出し規約: (元パラメータ..., closure_ptr) -> result
                // リフト関数内部で closure_ptr からキャプチャ値を読み出す
                let mut lifted_ctx = FuncCtx::new(lambda_name.clone());
                // 元のパラメータを登録
                for p in params {
                    let idx = lifted_ctx.next_local;
                    lifted_ctx.locals_map.insert(p.name.clone(), idx);
                    lifted_ctx.param_count += 1;
                    lifted_ctx.next_local += 1;
                }
                // closure_ptr パラメータを追加（常に最後のパラメータ）
                let closure_ptr_idx = lifted_ctx.next_local;
                lifted_ctx.locals_map.insert("__closure_ptr".to_string(), closure_ptr_idx);
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
                    prologue.push(Instruction::I64Load { offset: 8 + (i as u32) * 8 });
                    prologue.push(Instruction::LocalSet(fv_local));
                }

                // Lambda の本体を変換
                self.lower_expr(&mut lifted_ctx, body)?;

                // プロローグを本体の先頭に挿入
                let mut full_body = prologue;
                full_body.extend(lifted_ctx.instructions);

                // リフト先関数のパラメータ型: (元パラメータ..., closure_ptr)
                let total_params = params.len() + 1; // +1 は closure_ptr
                let extra_locals = vec![IrType::I64; (lifted_ctx.next_local - lifted_ctx.param_count) as usize];

                let lifted_func = Function {
                    name: lambda_name.clone(),
                    params: vec![IrType::I64; total_params],
                    result: IrType::I64,
                    locals: extra_locals,
                    body: full_body,
                    is_export: false,
                };

                // リフトされた関数のインデックスを割り当て
                let func_idx = self.next_func_idx
                    + self.lifted_functions.len() as u32;
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
                        let field_map: HashMap<&str, &Expr> = fields
                            .iter()
                            .map(|(n, e)| (n.as_str(), e))
                            .collect();
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

            Expr::FieldAccess(_, expr, field_name) => {
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
                                msg: format!("レコード型 '{tn}' にフィールド '{field_name}' が存在しません"),
                            });
                        }
                    }
                }

                if !resolved {
                    // フォールバック: フィールド名で全レコード型を走査
                    // record_fields を一時的にクローンして借用問題を回避
                    let record_fields_snapshot: Vec<(String, Vec<String>)> = self.record_fields
                        .iter()
                        .map(|(k, v)| (k.clone(), v.clone()))
                        .collect();
                    for (type_name, fields) in &record_fields_snapshot {
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
                    if let Some(fields) = self.record_fields.get(tn).cloned() {
                        found_type = Some((tn.clone(), fields));
                    }
                }

                if found_type.is_none() {
                    // フォールバック: フィールド名で全レコード型を走査
                    let record_fields_snapshot: Vec<(String, Vec<String>)> = self.record_fields
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
    pub(crate) fn emit_binop(&mut self, ctx: &mut FuncCtx, op: &str) -> Result<(), LowerError> {
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

    /// 文字列キーの場合に FNV-1a ハッシュ呼び出しを挿入する
    fn emit_string_key_hash(&self, ctx: &mut FuncCtx, key_expr: &Expr) -> Result<(), LowerError> {
        let is_string_key = self.infer_expr_type_name(key_expr)
            .map(|t| t == "String")
            .unwrap_or(false);
        if is_string_key {
            let hash_idx = *self.func_indices.get("__fnv1a_hash").ok_or_else(|| {
                LowerError::UndefinedFunction { name: "__fnv1a_hash".to_string() }
            })?;
            ctx.emit(Instruction::Call(hash_idx));
        }
        Ok(())
    }
}
