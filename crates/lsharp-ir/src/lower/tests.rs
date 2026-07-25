//! lower モジュールのテスト

use std::collections::HashMap;

use super::*;
use crate::{
    Function, Instruction, IrType, Module,
    root_lifetime::{RootLifetimeError, validate_function, validate_module},
};
use lsharp_syntax::span::Span;
use lsharp_types::{
    infer::Infer,
    types::{Type, TypeScheme},
};

/// ソースコードから IR モジュールを生成するヘルパー
fn lower(source: &str) -> Module {
    let program = lsharp_syntax::parse(source).unwrap();
    let mut infer = Infer::new();
    let type_results = infer.infer_program(&program).unwrap();
    let expr_type_results = infer.expr_type_results_snapshot();
    let mut lowerer = Lower::new();
    let module = lowerer
        .lower_program_with_expr_types(&program, &type_results, &expr_type_results)
        .unwrap();
    validate_module(&module).unwrap_or_else(|error| {
        panic!("lower helper produced invalid root lifetime: source={source:?}, error={error:?}")
    });
    module
}

/// IR のテキストダンプをスナップショットテストで検証
fn assert_ir(source: &str, snapshot_name: &str) {
    let module = lower(source);
    insta::assert_snapshot!(snapshot_name, module.dump());
}

fn count_call_instr(body: &[Instruction], idx: u32) -> usize {
    body.iter()
        .filter(|instr| matches!(instr, Instruction::Call(call_idx) if *call_idx == idx))
        .count()
}

fn call_positions(body: &[Instruction], idx: u32) -> Vec<usize> {
    body.iter()
        .enumerate()
        .filter_map(|(i, instr)| {
            matches!(instr, Instruction::Call(call_idx) if *call_idx == idx).then_some(i)
        })
        .collect()
}

const ALLOC_IDX: u32 = 1;
const ROOT_PUSH_IDX: u32 = 14;
const ROOT_POP_IDX: u32 = 15;
const ROOT_SET_IDX: u32 = 16;
const USER_FUNC_BASE_IDX: u32 = 17;

fn function_index(module: &Module, name: &str) -> u32 {
    USER_FUNC_BASE_IDX
        + module
            .functions
            .iter()
            .position(|func| func.name == name)
            .unwrap_or_else(|| panic!("function not found: {name}")) as u32
}

#[test]
fn lower_context_reuse_matches_a_fresh_context() {
    let first_program = lsharp_syntax::parse(
        r#"
        (type User (record (: name String)))
        (defn first [] "first")
        "#,
    )
    .unwrap();
    let second_program = lsharp_syntax::parse(r#"(defn second [] 42)"#).unwrap();

    let mut first_infer = Infer::new();
    let first_type_results = first_infer.infer_program(&first_program).unwrap();
    let first_expr_type_results = first_infer.expr_type_results_snapshot();
    let mut second_infer = Infer::new();
    let second_type_results = second_infer.infer_program(&second_program).unwrap();
    let second_expr_type_results = second_infer.expr_type_results_snapshot();

    let mut reused = Lower::with_backend(LowerBackend::WasmGc);
    reused
        .lower_program_with_expr_types(
            &first_program,
            &first_type_results,
            &first_expr_type_results,
        )
        .unwrap();
    let reused_module = reused
        .lower_program_with_expr_types(
            &second_program,
            &second_type_results,
            &second_expr_type_results,
        )
        .unwrap();

    let mut fresh = Lower::with_backend(LowerBackend::WasmGc);
    let fresh_module = fresh
        .lower_program_with_expr_types(
            &second_program,
            &second_type_results,
            &second_expr_type_results,
        )
        .unwrap();

    assert_eq!(reused_module.dump(), fresh_module.dump());
    assert_eq!(reused_module.string_data, fresh_module.string_data);
    assert_eq!(reused_module.gc_types.len(), fresh_module.gc_types.len());
}

fn call_position(body: &[Instruction], idx: u32) -> usize {
    body.iter()
        .position(|instr| matches!(instr, Instruction::Call(call_idx) if *call_idx == idx))
        .unwrap_or_else(|| panic!("call {idx} not found in body: {body:?}"))
}

fn call_indirect_positions(body: &[Instruction]) -> Vec<usize> {
    body.iter()
        .enumerate()
        .filter_map(|(idx, instr)| matches!(instr, Instruction::CallIndirect(_)).then_some(idx))
        .collect()
}

fn assert_roots_balanced(body: &[Instruction], context: &str) {
    assert_eq!(
        count_call_instr(body, ROOT_PUSH_IDX),
        count_call_instr(body, ROOT_POP_IDX),
        "{context}: root_push/root_pop が釣り合っていない: {body:?}"
    );
}

fn assert_rooted_safe_point(body: &[Instruction], safe_point_pos: usize, context: &str) {
    let has_push_before = body[..safe_point_pos]
        .iter()
        .any(|instr| matches!(instr, Instruction::Call(ROOT_PUSH_IDX)));
    let has_pop_after = body[safe_point_pos + 1..]
        .iter()
        .any(|instr| matches!(instr, Instruction::Call(ROOT_POP_IDX)));

    assert!(
        has_push_before,
        "{context}: safe point 前に root_push が必要: {body:?}"
    );
    assert!(
        has_pop_after,
        "{context}: safe point 後に root_pop が必要: {body:?}"
    );
}

fn assert_root_push_between(body: &[Instruction], after: usize, before: usize, context: &str) {
    assert!(
        body[after + 1..before]
            .iter()
            .any(|instr| matches!(instr, Instruction::Call(ROOT_PUSH_IDX))),
        "{context}: safe point 間の値を再 root_push するべき: {body:?}"
    );
}

#[cfg(test)]
mod closure_calls;
#[cfg(test)]
mod core_lowering;
#[cfg(test)]
mod heap_and_adt;
#[cfg(test)]
mod language_and_traits;
#[cfg(test)]
mod module_and_lambdas;
#[cfg(test)]
mod records_and_adt;
#[cfg(test)]
mod rooting_calls;
#[cfg(test)]
mod rooting_loops;
#[cfg(test)]
mod wasm_gc_and_roots;
