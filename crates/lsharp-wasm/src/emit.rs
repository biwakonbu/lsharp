//! 共通命令エミッション
//!
//! `codegen` と `wasi` の両モジュールで共有する命令変換ロジック。

use lsharp_ir::{Instruction, IrType};
use wasm_encoder::ValType;

use crate::codegen::CodegenError;

/// IR 型 → Wasm 型変換
pub fn ir_to_wasm_valtype(ty: IrType) -> ValType {
    match ty {
        IrType::I64 => ValType::I64,
        IrType::F64 => ValType::F64,
        IrType::I32 => ValType::I32,
        IrType::Ref(_) => ValType::I64, // MVP: GC 参照は i64 にフォールバック
        IrType::FuncRef => ValType::FUNCREF,
    }
}

/// IR 命令列を Wasm 命令に変換（Call 処理はコールバックで差し込み）
pub fn emit_instructions_common<F>(
    func: &mut wasm_encoder::Function,
    instructions: &[Instruction],
    mut call_handler: F,
) -> Result<(), CodegenError>
where
    F: FnMut(&mut wasm_encoder::Function, u32) -> Result<(), CodegenError>,
{
    emit_instructions_common_with_handler(func, instructions, &mut call_handler, |_, _| Ok(false))
}

/// IR 命令列を Wasm 命令に変換する。
/// バックエンド固有の命令は `custom_handler` が true を返して差し替えられる。
pub fn emit_instructions_common_with_handler<F, H>(
    func: &mut wasm_encoder::Function,
    instructions: &[Instruction],
    mut call_handler: F,
    mut custom_handler: H,
) -> Result<(), CodegenError>
where
    F: FnMut(&mut wasm_encoder::Function, u32) -> Result<(), CodegenError>,
    H: FnMut(&mut wasm_encoder::Function, &Instruction) -> Result<bool, CodegenError>,
{
    use wasm_encoder::Instruction as W;

    for instr in instructions {
        if custom_handler(func, instr)? {
            continue;
        }
        match instr {
            // 定数
            Instruction::I64Const(n) => {
                func.instruction(&W::I64Const(*n));
            }
            Instruction::F64Const(n) => {
                func.instruction(&W::F64Const((*n).into()));
            }
            Instruction::I32Const(n) => {
                func.instruction(&W::I32Const(*n));
            }

            // ローカル変数
            Instruction::LocalGet(i) => {
                func.instruction(&W::LocalGet(*i));
            }
            Instruction::LocalSet(i) => {
                func.instruction(&W::LocalSet(*i));
            }
            Instruction::LocalTee(i) => {
                func.instruction(&W::LocalTee(*i));
            }

            // 整数演算
            Instruction::I64Add => {
                func.instruction(&W::I64Add);
            }
            Instruction::I64Sub => {
                func.instruction(&W::I64Sub);
            }
            Instruction::I64Mul => {
                func.instruction(&W::I64Mul);
            }
            Instruction::I64Div => {
                func.instruction(&W::I64DivS);
            }
            Instruction::I64Rem => {
                func.instruction(&W::I64RemS);
            }

            // 浮動小数点演算
            Instruction::F64Add => {
                func.instruction(&W::F64Add);
            }
            Instruction::F64Sub => {
                func.instruction(&W::F64Sub);
            }
            Instruction::F64Mul => {
                func.instruction(&W::F64Mul);
            }
            Instruction::F64Div => {
                func.instruction(&W::F64Div);
            }

            // 比較
            Instruction::I64Eq => {
                func.instruction(&W::I64Eq);
            }
            Instruction::I64Ne => {
                func.instruction(&W::I64Ne);
            }
            Instruction::I64LtS => {
                func.instruction(&W::I64LtS);
            }
            Instruction::I64GtS => {
                func.instruction(&W::I64GtS);
            }
            Instruction::I64LeS => {
                func.instruction(&W::I64LeS);
            }
            Instruction::I64GeS => {
                func.instruction(&W::I64GeS);
            }

            // 論理演算
            Instruction::I32Eqz => {
                func.instruction(&W::I32Eqz);
            }
            Instruction::I32And => {
                func.instruction(&W::I32And);
            }
            Instruction::I32Or => {
                func.instruction(&W::I32Or);
            }

            // 型変換
            Instruction::I64ExtendI32S => {
                func.instruction(&W::I64ExtendI32S);
            }
            Instruction::I32WrapI64 => {
                func.instruction(&W::I32WrapI64);
            }

            // 制御フロー — Call はコールバックに委譲
            Instruction::Call(i) => {
                call_handler(func, *i)?;
            }
            Instruction::If(ty) => {
                func.instruction(&W::If(wasm_encoder::BlockType::Result(ir_to_wasm_valtype(
                    *ty,
                ))));
            }
            Instruction::Else => {
                func.instruction(&W::Else);
            }
            Instruction::End => {
                func.instruction(&W::End);
            }
            Instruction::Block(ty) => {
                func.instruction(&W::Block(wasm_encoder::BlockType::Result(
                    ir_to_wasm_valtype(*ty),
                )));
            }
            Instruction::Loop(ty) => {
                func.instruction(&W::Loop(wasm_encoder::BlockType::Result(
                    ir_to_wasm_valtype(*ty),
                )));
            }
            Instruction::BlockEmpty => {
                func.instruction(&W::Block(wasm_encoder::BlockType::Empty));
            }
            Instruction::LoopEmpty => {
                func.instruction(&W::Loop(wasm_encoder::BlockType::Empty));
            }
            Instruction::IfEmpty => {
                func.instruction(&W::If(wasm_encoder::BlockType::Empty));
            }
            Instruction::Br(i) => {
                func.instruction(&W::Br(*i));
            }
            Instruction::BrIf(i) => {
                func.instruction(&W::BrIf(*i));
            }
            Instruction::Return => {
                func.instruction(&W::Return);
            }
            Instruction::Unreachable => {
                func.instruction(&W::Unreachable);
            }

            Instruction::CallImport(i) => {
                func.instruction(&W::Call(*i));
            }
            Instruction::WriteFileBytes => {
                return Err(CodegenError::Error {
                    msg: "write-file-bytes は target 固有の runtime helper を必要とします"
                        .to_string(),
                });
            }
            Instruction::Drop => {
                func.instruction(&W::Drop);
            }

            // GC 命令は MVP ではフォールバック
            // TODO: WasmGC 本格実装時に削除。スタック操作はフォールバック用。
            Instruction::StructNew(_) => {
                func.instruction(&W::I64Const(0));
            }
            // TODO: WasmGC 本格実装時に削除。スタック操作はフォールバック用。
            Instruction::StructGet(_, _) => { /* nop */ }
            // TODO: WasmGC 本格実装時に削除。スタック操作はフォールバック用。
            Instruction::StructSet(_, _) => {
                func.instruction(&W::Drop);
                func.instruction(&W::Drop);
                func.instruction(&W::I64Const(0));
            }
            // TODO: WasmGC 本格実装時に削除。スタック操作はフォールバック用。
            Instruction::RefCast(_) => { /* nop */ }
            // TODO: WasmGC 本格実装時に削除。linear backend では null reference を i64 0 にする。
            Instruction::RefNull(_) => {
                func.instruction(&W::I64Const(0));
            }
            Instruction::ArrayNewFixed(_, _)
            | Instruction::ArrayNewDefault(_)
            | Instruction::ArrayGet(_)
            | Instruction::ArraySet(_)
            | Instruction::ArrayLen(_) => {
                return Err(CodegenError::Error {
                    msg: "GC array 命令は WasmGC backend でのみ利用できます".to_string(),
                });
            }

            // 関数参照
            Instruction::RefFunc(idx) => {
                func.instruction(&W::RefFunc(*idx));
            }
            Instruction::CallRef(type_idx) => {
                func.instruction(&W::CallRef(*type_idx));
            }

            // グローバル変数
            Instruction::GlobalGet(idx) => {
                func.instruction(&W::GlobalGet(*idx));
            }
            Instruction::GlobalSet(idx) => {
                func.instruction(&W::GlobalSet(*idx));
            }

            // メモリ操作
            Instruction::I32Load { offset } => {
                func.instruction(&W::I32Load(wasm_encoder::MemArg {
                    offset: *offset as u64,
                    align: 2, // 4バイトアライン
                    memory_index: 0,
                }));
            }
            Instruction::I32Store { offset } => {
                func.instruction(&W::I32Store(wasm_encoder::MemArg {
                    offset: *offset as u64,
                    align: 2,
                    memory_index: 0,
                }));
            }
            Instruction::I32Load8U { offset } => {
                func.instruction(&W::I32Load8U(wasm_encoder::MemArg {
                    offset: *offset as u64,
                    align: 0, // 1バイトアライン
                    memory_index: 0,
                }));
            }
            Instruction::I32Store8 { offset } => {
                func.instruction(&W::I32Store8(wasm_encoder::MemArg {
                    offset: *offset as u64,
                    align: 0,
                    memory_index: 0,
                }));
            }
            Instruction::I64Load { offset } => {
                func.instruction(&W::I64Load(wasm_encoder::MemArg {
                    offset: *offset as u64,
                    align: 3, // 8バイトアライン
                    memory_index: 0,
                }));
            }
            Instruction::I64Store { offset } => {
                func.instruction(&W::I64Store(wasm_encoder::MemArg {
                    offset: *offset as u64,
                    align: 3,
                    memory_index: 0,
                }));
            }
            // 型変換
            Instruction::I64ExtendI32U => {
                func.instruction(&W::I64ExtendI32U);
            }
            // i32 算術演算
            Instruction::I32Add => {
                func.instruction(&W::I32Add);
            }
            Instruction::I32Sub => {
                func.instruction(&W::I32Sub);
            }
            Instruction::I32Mul => {
                func.instruction(&W::I32Mul);
            }
            // i32 比較
            Instruction::I32GtU => {
                func.instruction(&W::I32GtU);
            }
            Instruction::I32GeU => {
                func.instruction(&W::I32GeU);
            }
            // ビット操作
            Instruction::I32Shl => {
                func.instruction(&W::I32Shl);
            }
            Instruction::I32ShrU => {
                func.instruction(&W::I32ShrU);
            }
            Instruction::I64Shl => {
                func.instruction(&W::I64Shl);
            }
            Instruction::I64ShrU => {
                func.instruction(&W::I64ShrU);
            }
            Instruction::I64And => {
                func.instruction(&W::I64And);
            }
            Instruction::I64Or => {
                func.instruction(&W::I64Or);
            }
            Instruction::I64Xor => {
                func.instruction(&W::I64Xor);
            }
            // メモリ管理
            Instruction::MemoryGrow => {
                func.instruction(&W::MemoryGrow(0));
            }
            Instruction::MemorySize => {
                func.instruction(&W::MemorySize(0));
            }
            Instruction::MemoryCopy => {
                func.instruction(&W::MemoryCopy {
                    src_mem: 0,
                    dst_mem: 0,
                });
            }
            Instruction::MemoryFill => {
                func.instruction(&W::MemoryFill(0));
            }
            // 間接呼び出し (クロージャ用)
            Instruction::CallIndirect(type_idx) => {
                func.instruction(&W::CallIndirect {
                    type_index: *type_idx,
                    table_index: 0,
                });
            }
            // 関数インデックスを i32 値として積む (codegen でリマップ済み)
            Instruction::FuncIdx(idx) => {
                // Call のリマップコールバックは使わず、直接 i32.const にする
                // ここに来る時点で既に Wasm インデックスにリマップされていない
                // → wasi.rs の emit_instructions_wasi で処理する
                func.instruction(&W::I32Const(*idx as i32));
            }
            // StringConst は lowering 段階でインライン展開済みのはず
            Instruction::StringConst(_) => {
                panic!("StringConst should be expanded in lowering stage");
            }
        };
    }

    Ok(())
}
