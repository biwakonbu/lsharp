//! V2-11 (ii-b) reference test: selfhost layout の 9-import / 5-type の
//! WASM section bytes を wasm-encoder で生成し、selfhost L# 側の
//! emit-import-section-runtime / emit-type-section-runtime を写経する
//! 際の正本として hex dump で記録する。
//!
//! selfhost layout (commit 5be506d で逆算済み):
//!   slot 0 alloc            (i64) -> i64   type A
//!   slot 1 print            (i64) -> ()    type B
//!   slot 2 read-file        (i64) -> i64   type A
//!   slot 3 command-line-arg (i64) -> i64   type A
//!   slot 4 string-concat    (i64,i64) -> i64       type C
//!   slot 5 substring        (i64,i64,i64) -> i64   type D
//!   slot 6 file-exists?     (i64) -> i64   type A
//!   slot 7 root_push        (i64) -> i64   type A
//!   slot 8 root_pop         () -> i64      type E
//!
//! root_set (slot 9) は現行 emit path で未使用のため省略。

use wasm_encoder::{EntityType, ImportSection, TypeSection, ValType};

fn build_runtime_type_section() -> TypeSection {
    let mut types = TypeSection::new();
    // type 0 = A: (i64) -> i64
    types.ty().function(vec![ValType::I64], vec![ValType::I64]);
    // type 1 = B: (i64) -> ()
    types.ty().function(vec![ValType::I64], vec![]);
    // type 2 = C: (i64, i64) -> i64
    types
        .ty()
        .function(vec![ValType::I64, ValType::I64], vec![ValType::I64]);
    // type 3 = D: (i64, i64, i64) -> i64
    types.ty().function(
        vec![ValType::I64, ValType::I64, ValType::I64],
        vec![ValType::I64],
    );
    // type 4 = E: () -> i64
    types.ty().function(vec![], vec![ValType::I64]);
    types
}

fn build_runtime_import_section() -> ImportSection {
    let mut imports = ImportSection::new();
    imports.import("env", "__alloc", EntityType::Function(0));
    imports.import("env", "print", EntityType::Function(1));
    imports.import("env", "read-file", EntityType::Function(0));
    imports.import("env", "command-line-arg", EntityType::Function(0));
    imports.import("env", "string-concat", EntityType::Function(2));
    imports.import("env", "substring", EntityType::Function(3));
    imports.import("env", "file-exists?", EntityType::Function(0));
    imports.import("env", "root_push", EntityType::Function(0));
    imports.import("env", "root_pop", EntityType::Function(4));
    imports
}

fn hex_dump(label: &str, bytes: &[u8]) {
    println!("=== {} ({} bytes) ===", label, bytes.len());
    for (i, chunk) in bytes.chunks(16).enumerate() {
        let hex: Vec<String> = chunk.iter().map(|b| format!("{:02x}", b)).collect();
        println!("{:04x}: {}", i * 16, hex.join(" "));
    }
}

#[test]
fn dump_runtime_type_section_reference_bytes() {
    use wasm_encoder::Section;
    let types = build_runtime_type_section();
    let mut bytes = Vec::new();
    types.append_to(&mut bytes);
    hex_dump("type section (5 types)", &bytes);
    assert!(!bytes.is_empty(), "type section bytes must be non-empty");
}

#[test]
fn dump_runtime_import_section_reference_bytes() {
    use wasm_encoder::Section;
    let imports = build_runtime_import_section();
    let mut bytes = Vec::new();
    imports.append_to(&mut bytes);
    hex_dump("import section (9 imports)", &bytes);
    assert_eq!(imports.len(), 9, "must encode exactly 9 imports");
}

#[test]
fn validate_runtime_sections_compose_into_valid_module() {
    use wasm_encoder::{FunctionSection, Module};
    let types = build_runtime_type_section();
    let imports = build_runtime_import_section();
    let funcs = FunctionSection::new();
    let mut module = Module::new();
    module.section(&types);
    module.section(&imports);
    module.section(&funcs);
    let bytes = module.finish();
    wasmparser::validate(&bytes).expect("9-import / 5-type module must validate");
}
