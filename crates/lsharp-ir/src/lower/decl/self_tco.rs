//! 自己末尾呼び出し最適化 (Self TCO) の lowering helper

use std::collections::HashMap;

use crate::{Instruction, IrType};

/// 自己末尾呼び出し最適化 (Self TCO) を適用する
///
/// 関数本体の命令列を解析し、自己再帰末尾呼び出しをループ+ジャンプに変換する。
///
/// 変換例 (`append-byte-vector` の場合):
/// ```text
/// ;; 変換前 (再帰)
/// if (i64)
///   local.get 0   ;; base case
/// else
///   ... (新しい引数を計算)
///   call self
/// end
///
/// ;; 変換後 (ループ)
/// loop (i64)
///   if (i64)
///     local.get 0
///   else
///     ...
///     local.set 3, local.set 2, local.set 1, local.set 0
///     br 1   ;; loop 再起動
///   end
/// end (loop)
/// ```
///
/// 検出条件: `Call(self_idx)` の後続命令が全て `End` のみである場合を末尾呼び出しとみなす。
/// 既存の Loop/Block 命令が含まれる関数には適用しない。
pub(super) struct SelfTcoRootOps<'a> {
    pub(super) rooted_params: &'a [(u32, u32)],
    pub(super) root_push_idx: u32,
    pub(super) root_pop_idx: u32,
    pub(super) root_set_idx: u32,
}

pub(super) fn apply_self_tco(
    instructions: Vec<Instruction>,
    self_idx: u32,
    param_count: u32,
    result_type: IrType,
    root_ops: &SelfTcoRootOps<'_>,
) -> Vec<Instruction> {
    // 既存のループ/ブロック命令がある場合はスキップ (安全のため)
    let has_loop_or_block = instructions.iter().any(|i| {
        matches!(
            i,
            Instruction::Loop(_)
                | Instruction::LoopEmpty
                | Instruction::Block(_)
                | Instruction::BlockEmpty
        )
    });
    if has_loop_or_block {
        return instructions;
    }

    // 自己末尾呼び出し候補を収集: position → depth at call site
    let tail_calls = find_simple_self_tail_calls(&instructions, self_idx);

    if tail_calls.is_empty() {
        return instructions;
    }

    // 変換: Loop(result_type) でラップし、各 Call(self) を LocalSets + Br に置換
    let mut result = Vec::with_capacity(
        instructions.len()
            + 2
            + root_ops.rooted_params.len() * 5
            + tail_calls.len() * (param_count as usize + 1 + root_ops.rooted_params.len() * 4),
    );
    for (param_idx, slot_local) in root_ops.rooted_params {
        result.push(Instruction::LocalGet(*param_idx));
        result.push(Instruction::Call(root_ops.root_push_idx));
        result.push(Instruction::LocalSet(*slot_local));
    }
    result.push(Instruction::Loop(result_type));

    for (i, instr) in instructions.into_iter().enumerate() {
        if let Some(&depth) = tail_calls.get(&i) {
            // Call(self) を引数ローカルへの LocalSet + Br に置き換える
            // スタック上の引数は LIFO のため、最後の引数から逆順に pop する
            for p in (0..param_count).rev() {
                result.push(Instruction::LocalSet(p));
            }
            for (param_idx, slot_local) in root_ops.rooted_params {
                result.push(Instruction::LocalGet(*slot_local));
                result.push(Instruction::LocalGet(*param_idx));
                result.push(Instruction::Call(root_ops.root_set_idx));
                result.push(Instruction::Drop);
            }
            result.push(Instruction::Br(depth));
            // Call 命令自体は出力しない (replace)
        } else {
            result.push(instr);
        }
    }

    result.push(Instruction::End); // Loop を閉じる
    for _ in root_ops.rooted_params {
        result.push(Instruction::Call(root_ops.root_pop_idx));
        result.push(Instruction::Drop);
    }
    result
}

/// 単純な自己末尾呼び出しを検出する
///
/// `Call(self_idx)` の後続命令が全て `End` のみの場合を末尾呼び出しとみなす。
/// 戻り値: position → depth (呼び出し時点の if/else ネスト深度) のマップ
fn find_simple_self_tail_calls(instructions: &[Instruction], self_idx: u32) -> HashMap<usize, u32> {
    let mut result = HashMap::new();
    let mut depth = 0i32;

    for (pos, instr) in instructions.iter().enumerate() {
        match instr {
            Instruction::If(_) | Instruction::IfEmpty => depth += 1,
            Instruction::Else => {} // depth は変化しない
            Instruction::End => depth -= 1,
            Instruction::Call(idx) if *idx == self_idx => {
                let d = depth;
                // 後続命令が全て End かつ数が depth と一致すれば末尾呼び出し
                let remaining = &instructions[pos + 1..];
                if remaining.len() == d as usize
                    && remaining.iter().all(|i| matches!(i, Instruction::End))
                {
                    result.insert(pos, d as u32);
                }
            }
            _ => {}
        }
    }
    result
}
