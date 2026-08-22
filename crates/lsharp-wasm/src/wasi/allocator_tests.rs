use wasm_encoder::CodeSection;

use super::{AllocatorGlobals, allocator::emit_alloc_func};

#[test]
fn allocator_module_emits_function_body() {
    let mut codes = CodeSection::new();
    emit_alloc_func(
        &mut codes,
        AllocatorGlobals {
            heap_ptr_global_idx: 0,
            alloc_count_global_idx: 1,
            object_count_global_idx: 2,
            free_list_count_global_idx: 3,
            object_table_base_global_idx: 5,
            object_table_capacity_global_idx: 6,
            free_class_heads_base_global_idx: 7,
            free_list_scan_steps_global_idx: 8,
        },
    );

    assert_eq!(codes.len(), 1);
}

/// `Block` を開いた直後に無条件 `Br(0)` でその block を抜ける区間を残さない。
///
/// この形は「区間を丸ごと到達不能にして残す」ときにだけ現れる。到達不能な命令列は
/// wasm validator を通ってしまうので、中に誤りがあっても実行では気付けない
/// (`ISSUES.md` `I-35`: legacy free-list search の `Br(0)` は `Br(1)` であるべきだった)。
/// 到達不能区間そのものを禁止して、この形のバグが再び入らないようにする。
#[test]
fn allocator_body_has_no_unreachable_block_prologue() {
    use wasm_encoder::Encode;

    let mut codes = CodeSection::new();
    emit_alloc_func(
        &mut codes,
        AllocatorGlobals {
            heap_ptr_global_idx: 0,
            alloc_count_global_idx: 1,
            object_count_global_idx: 2,
            free_list_count_global_idx: 3,
            object_table_base_global_idx: 5,
            object_table_capacity_global_idx: 6,
            free_class_heads_base_global_idx: 7,
            free_list_scan_steps_global_idx: 8,
        },
    );

    let mut raw = Vec::new();
    codes.encode(&mut raw);

    // block (empty) = 0x02 0x40、br 0 = 0x0C 0x00
    const DEAD_PROLOGUE: [u8; 4] = [0x02, 0x40, 0x0C, 0x00];
    assert!(
        !raw.windows(DEAD_PROLOGUE.len()).any(|w| w == DEAD_PROLOGUE),
        "__alloc の body に到達不能 block 区間が残っている"
    );
}
