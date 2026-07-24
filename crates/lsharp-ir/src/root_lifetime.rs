//! Compiler が生成する root stack 操作の lifetime 契約。
//!
//! `root_push` が返す slot は、対応する `root_pop` まで有効でなければならない。
//! Lowering の個数検査だけでは、pop 済み slot を `root_set` に渡す事故や、分岐ごとの
//! lifetime のずれを検出できないため、ここでは軽量な抽象実行で検査する。

use std::collections::HashMap;

use crate::{Function, Instruction, Module};

/// Lowering が使用する runtime import index。
pub const ROOT_PUSH_INDEX: u32 = 14;
pub const ROOT_POP_INDEX: u32 = 15;
pub const ROOT_SET_INDEX: u32 = 16;

#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum RootLifetimeError {
    #[error("{function}: instruction {instruction_index}: root_pop without an active root slot")]
    RootPopUnderflow {
        function: String,
        instruction_index: usize,
    },

    #[error("{function}: instruction {instruction_index}: root_set without an active root slot")]
    RootSetWithoutActiveSlot {
        function: String,
        instruction_index: usize,
    },

    #[error("{function}: instruction {instruction_index}: root_set used stale root slot {slot_id}")]
    StaleSlot {
        function: String,
        instruction_index: usize,
        slot_id: u64,
    },

    #[error(
        "{function}: instruction {instruction_index}: branch root depth differs ({then_depth} vs {else_depth})"
    )]
    BranchDepthMismatch {
        function: String,
        instruction_index: usize,
        then_depth: usize,
        else_depth: usize,
    },

    #[error("{function}: function exits with {depth} active root slots")]
    ImbalancedExit { function: String, depth: usize },
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum Value {
    Unknown,
    Constant(i64),
    RootSlot(u64),
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct State {
    stack: Vec<Value>,
    locals: HashMap<u32, Value>,
    /// `None` は、分岐 merge 後など slot identity を証明できない状態を表す。
    roots: Vec<Option<u64>>,
    next_slot: u64,
}

impl State {
    fn new(function: &Function) -> Self {
        let mut locals = HashMap::new();
        for index in 0..function.params.len() as u32 {
            locals.insert(index, Value::Unknown);
        }
        Self {
            stack: Vec::new(),
            locals,
            roots: Vec::new(),
            next_slot: 0,
        }
    }

    fn pop_value(&mut self) -> Value {
        self.stack.pop().unwrap_or(Value::Unknown)
    }

    fn pop_values(&mut self, count: usize) {
        for _ in 0..count {
            self.pop_value();
        }
    }

    fn has_root_slot(&self, slot_id: u64) -> bool {
        self.roots.iter().flatten().any(|active| *active == slot_id)
    }

    fn merge(
        mut then_state: Self,
        else_state: Self,
        instruction_index: usize,
        function: &str,
    ) -> Result<Self, RootLifetimeError> {
        if then_state.roots.len() != else_state.roots.len() {
            return Err(RootLifetimeError::BranchDepthMismatch {
                function: function.to_string(),
                instruction_index,
                then_depth: then_state.roots.len(),
                else_depth: else_state.roots.len(),
            });
        }

        for (then_root, else_root) in then_state.roots.iter_mut().zip(else_state.roots) {
            if *then_root != Some(else_root.unwrap_or(u64::MAX)) {
                *then_root = None;
            }
        }

        if then_state.stack.len() != else_state.stack.len() {
            then_state.stack.clear();
        } else {
            for (then_value, else_value) in then_state.stack.iter_mut().zip(else_state.stack) {
                if *then_value != else_value {
                    *then_value = Value::Unknown;
                }
            }
        }

        for (local, then_value) in &mut then_state.locals {
            if else_state.locals.get(local) != Some(then_value) {
                *then_value = Value::Unknown;
            }
        }
        for (local, else_value) in else_state.locals {
            then_state.locals.entry(local).or_insert(Value::Unknown);
            if then_state.locals.get(&local) != Some(&else_value) {
                then_state.locals.insert(local, Value::Unknown);
            }
        }
        then_state.next_slot = then_state.next_slot.max(else_state.next_slot);
        Ok(then_state)
    }
}

/// 1 関数の root slot lifetime を検証する。
pub fn validate_function(function: &Function) -> Result<(), RootLifetimeError> {
    let state = State::new(function);
    let state = validate_range(
        &function.body,
        0,
        function.body.len(),
        state,
        &function.name,
    )?;
    if !state.roots.is_empty() {
        return Err(RootLifetimeError::ImbalancedExit {
            function: function.name.clone(),
            depth: state.roots.len(),
        });
    }
    Ok(())
}

/// モジュール内の全関数を検証する。
pub fn validate_module(module: &Module) -> Result<(), RootLifetimeError> {
    for function in &module.functions {
        validate_function(function)?;
    }
    Ok(())
}

fn validate_range(
    body: &[Instruction],
    start: usize,
    end: usize,
    mut state: State,
    function: &str,
) -> Result<State, RootLifetimeError> {
    let mut index = start;
    while index < end {
        match &body[index] {
            Instruction::I64Const(value) => state.stack.push(Value::Constant(*value)),
            Instruction::LocalGet(local) => {
                state
                    .stack
                    .push(state.locals.get(local).cloned().unwrap_or(Value::Unknown));
            }
            Instruction::LocalSet(local) => {
                let value = state.pop_value();
                state.locals.insert(*local, value);
            }
            Instruction::LocalTee(local) => {
                let value = state.stack.last().cloned().unwrap_or(Value::Unknown);
                state.locals.insert(*local, value);
            }
            Instruction::Drop => {
                state.pop_value();
            }
            Instruction::Call(call_index) => match *call_index {
                ROOT_PUSH_INDEX => {
                    state.pop_value();
                    let slot_id = state.next_slot;
                    state.next_slot += 1;
                    state.roots.push(Some(slot_id));
                    state.stack.push(Value::RootSlot(slot_id));
                }
                ROOT_POP_INDEX => {
                    if state.roots.pop().is_none() {
                        return Err(RootLifetimeError::RootPopUnderflow {
                            function: function.to_string(),
                            instruction_index: index,
                        });
                    }
                    state.stack.push(Value::Unknown);
                }
                ROOT_SET_INDEX => {
                    state.pop_value();
                    let slot = state.pop_value();
                    if let Value::RootSlot(slot_id) = slot
                        && !state.has_root_slot(slot_id)
                    {
                        return Err(RootLifetimeError::StaleSlot {
                            function: function.to_string(),
                            instruction_index: index,
                            slot_id,
                        });
                    }
                    if state.roots.is_empty() {
                        return Err(RootLifetimeError::RootSetWithoutActiveSlot {
                            function: function.to_string(),
                            instruction_index: index,
                        });
                    }
                    state.stack.push(Value::Unknown);
                }
                0 => state.pop_values(1),
                1 | 6 | 7 | 9 | 11 | 13 => {
                    state.pop_values(1);
                    state.stack.push(Value::Unknown);
                }
                2 | 3 | 8 => {
                    state.pop_values(2);
                    state.stack.push(Value::Unknown);
                }
                4 | 5 => state.pop_values(1),
                10 | 12 => state.stack.push(Value::Unknown),
                _ => {}
            },
            Instruction::If(_) | Instruction::IfEmpty => {
                state.pop_value();
                let (else_index, end_index) = find_control_end(body, index)?;
                let then_end = else_index.unwrap_or(end_index);
                let then_state =
                    validate_range(body, index + 1, then_end, state.clone(), function)?;
                let else_state = if let Some(else_index) = else_index {
                    validate_range(body, else_index + 1, end_index, state.clone(), function)?
                } else {
                    state.clone()
                };
                state = State::merge(then_state, else_state, index, function)?;
                index = end_index;
            }
            Instruction::Block(_)
            | Instruction::BlockEmpty
            | Instruction::Loop(_)
            | Instruction::LoopEmpty => {
                let (_, end_index) = find_control_end(body, index)?;
                state = validate_range(body, index + 1, end_index, state, function)?;
                index = end_index;
            }
            Instruction::Return | Instruction::Unreachable => return Ok(state),
            _ => {}
        }
        index += 1;
    }
    Ok(state)
}

fn find_control_end(
    body: &[Instruction],
    start: usize,
) -> Result<(Option<usize>, usize), RootLifetimeError> {
    let mut depth = 0usize;
    let mut else_index = None;
    for (index, instruction) in body.iter().enumerate().skip(start) {
        match instruction {
            Instruction::If(_)
            | Instruction::IfEmpty
            | Instruction::Block(_)
            | Instruction::BlockEmpty
            | Instruction::Loop(_)
            | Instruction::LoopEmpty => depth += 1,
            Instruction::Else if depth == 1 => else_index = Some(index),
            Instruction::End => {
                depth = depth.saturating_sub(1);
                if depth == 0 {
                    return Ok((else_index, index));
                }
            }
            _ => {}
        }
    }
    Err(RootLifetimeError::BranchDepthMismatch {
        function: "<malformed-ir>".to_string(),
        instruction_index: start,
        then_depth: 0,
        else_depth: 0,
    })
}
