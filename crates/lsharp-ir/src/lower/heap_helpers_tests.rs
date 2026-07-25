use crate::Instruction;

use super::{
    HEAP_TAG_ADT, HEAP_TAG_CLOSURE, HEAP_TAG_HASHMAP, HEAP_TAG_RECORD, HEAP_TAG_REF,
    HEAP_TAG_STRING, HEAP_TAG_VECTOR, emit_tag_pointer, emit_untag_pointer, emit_write_heap_header,
};

#[test]
fn heap_helper_module_preserves_pointer_and_tag_contract() {
    let mut body = Vec::new();
    emit_tag_pointer(&mut body, 0);
    emit_untag_pointer(&mut body);
    emit_write_heap_header(&mut body, HEAP_TAG_STRING, 16);

    assert_eq!(body.len(), 8);
    assert!(matches!(body[0], Instruction::I64ExtendI32U));
    assert!(matches!(body[1], Instruction::I64Const(v) if v == (1i64 << 63)));
    assert!(matches!(body[2], Instruction::I64Add));
    assert!(matches!(body[3], Instruction::I32WrapI64));
    assert!(matches!(body[4], Instruction::I32Const(v) if v == HEAP_TAG_STRING));
    assert!(matches!(body[5], Instruction::I32Store { offset: 0 }));
    assert!(matches!(body[6], Instruction::I32Const(16)));
    assert!(matches!(body[7], Instruction::I32Store { offset: 4 }));

    assert_eq!(HEAP_TAG_RECORD, 2);
    assert_eq!(HEAP_TAG_ADT, 3);
    assert_eq!(HEAP_TAG_CLOSURE, 4);
    assert_eq!(HEAP_TAG_VECTOR, 5);
    assert_eq!(HEAP_TAG_HASHMAP, 6);
    assert_eq!(HEAP_TAG_REF, 7);
}
