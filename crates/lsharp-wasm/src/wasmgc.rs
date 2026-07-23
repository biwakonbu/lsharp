//! WasmGC バックエンドの実行可能性を検証する最小スパイク。
//!
//! 本モジュールは L# の IR をまだ変換しない。`wasm-encoder` で GC struct と
//! `struct.new` / `struct.get` を含む core module を生成し、Wasmtime が実行できる
//! ことを固定する Stage 0 の契約である。実 backend はこの契約を土台に追加する。

use wasm_encoder::{
    CodeSection, ExportKind, ExportSection, FieldType, Function, FunctionSection, StorageType,
    TypeSection, ValType,
};

/// WasmGC の struct 命令を含む最小の self-contained module を生成する。
///
/// `read-field` は immutable な `struct { i64 }` を生成し、フィールド 0 を読み出して
/// `42` を返す。型セクションの 0 番目は GC struct、1 番目は関数型である。
pub fn emit_minimal_struct_probe() -> Vec<u8> {
    let mut module = wasm_encoder::Module::new();

    let mut types = TypeSection::new();
    types.ty().struct_([FieldType {
        element_type: StorageType::Val(ValType::I64),
        mutable: false,
    }]);
    types.ty().function([], [ValType::I64]);
    module.section(&types);

    let mut functions = FunctionSection::new();
    functions.function(1);
    module.section(&functions);

    let mut exports = ExportSection::new();
    exports.export("read-field", ExportKind::Func, 0);
    module.section(&exports);

    let mut code = CodeSection::new();
    let mut function = Function::new([]);
    function.instruction(&wasm_encoder::Instruction::I64Const(42));
    function.instruction(&wasm_encoder::Instruction::StructNew(0));
    function.instruction(&wasm_encoder::Instruction::StructGet {
        struct_type_index: 0,
        field_index: 0,
    });
    function.instruction(&wasm_encoder::Instruction::End);
    code.function(&function);
    module.section(&code);

    module.finish()
}
