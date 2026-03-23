//! WASI 対応の Wasm コード生成
//!
//! wasmtime で直接実行可能な Wasm バイナリを生成する。
//! print 関数を WASI の fd_write で実装し、_start エントリポイントを生成。

use lsharp_ir::{GcTypeKind, Instruction, Module};
use wasm_encoder::{
    ArrayType, CodeSection, CompositeInnerType, CompositeType, DataSection, EntityType,
    ExportKind, ExportSection, FieldType, FunctionSection, GlobalSection, GlobalType,
    ImportSection, MemorySection, MemoryType, StorageType, StructType, SubType, TypeSection,
    ValType,
};

use crate::codegen::CodegenError;

/// メモリレイアウト定数
const NEWLINE_ADDR: i32 = 0;     // '\n' の格納位置
const IOV_ADDR: i32 = 16;        // iovec 構造体 (8 bytes: base + len)
const NWRITTEN_ADDR: i32 = 24;   // nwritten (4 bytes)
const BUF_END: i32 = 276;        // 数値変換バッファ末尾 (21桁分: i64の最大桁数+符号)

/// WASI モードで Wasm バイナリを生成
pub fn emit_wasm_wasi(module: &Module) -> Result<Vec<u8>, CodegenError> {
    let mut wasm_module = wasm_encoder::Module::new();

    // 関数インデックス:
    // 0: fd_write (import)
    // 1: __print_i64 (内部ヘルパー)
    // 2: __alloc (Bump Allocator)
    // 3: __string_concat (スタブ)
    // 4: __string_eq (スタブ)
    // 5..5+N-1: ユーザー関数
    // 5+N: _start
    let print_helper_idx: u32 = 1;
    let alloc_func_idx: u32 = 2;
    let string_concat_idx: u32 = 3;
    let string_eq_idx: u32 = 4;
    let user_func_base: u32 = 5;
    let start_func_idx: u32 = user_func_base + module.functions.len() as u32;

    // GC 型の数（TypeSection でのオフセット計算に使用）
    let gc_type_count = module.gc_types.len() as u32;

    // === Type Section ===
    let mut types = TypeSection::new();

    // GC 型定義を TypeSection に登録（type index 0..gc_type_count-1）
    for gc_type in &module.gc_types {
        match &gc_type.kind {
            GcTypeKind::Struct(fields) => {
                let wasm_fields: Vec<FieldType> = fields
                    .iter()
                    .map(|f| FieldType {
                        element_type: StorageType::Val(crate::emit::ir_to_wasm_valtype(f.ty)),
                        mutable: f.mutable,
                    })
                    .collect();
                types.ty().subtype(&SubType {
                    is_final: true,
                    supertype_idx: None,
                    composite_type: CompositeType {
                        inner: CompositeInnerType::Struct(StructType {
                            fields: wasm_fields.into_boxed_slice(),
                        }),
                        shared: false,
                        descriptor: None,
                        describes: None,
                    },
                });
            }
            GcTypeKind::Array(elem_ty) => {
                types.ty().subtype(&SubType {
                    is_final: true,
                    supertype_idx: None,
                    composite_type: CompositeType {
                        inner: CompositeInnerType::Array(ArrayType(FieldType {
                            element_type: StorageType::Val(crate::emit::ir_to_wasm_valtype(*elem_ty)),
                            mutable: true,
                        })),
                        shared: false,
                        descriptor: None,
                        describes: None,
                    },
                });
            }
        }
    }

    // 関数型のインデックスは gc_type_count だけオフセット

    // fd_write 関数型: (i32, i32, i32, i32) -> i32
    let fd_write_type_idx = types.len();
    types.ty().function(vec![ValType::I32; 4], vec![ValType::I32]);

    // __print_i64 関数型: (i64) -> ()
    let print_type_idx = types.len();
    types.ty().function(vec![ValType::I64], vec![]);

    // __alloc 関数型: (i64) -> i64 (サイズを受け取りアドレスを返す)
    let alloc_type_idx = types.len();
    types.ty().function(vec![ValType::I64], vec![ValType::I64]);

    // __string_concat 関数型: (i64, i64) -> i64
    let string_concat_type_idx = types.len();
    types.ty().function(vec![ValType::I64, ValType::I64], vec![ValType::I64]);

    // __string_eq 関数型: (i64, i64) -> i64
    let string_eq_type_idx = types.len();
    types.ty().function(vec![ValType::I64, ValType::I64], vec![ValType::I64]);

    // ユーザー関数の型
    let mut user_type_indices = Vec::new();
    for func in &module.functions {
        let type_idx = types.len();
        let params: Vec<ValType> = func.params.iter().map(|t| crate::emit::ir_to_wasm_valtype(*t)).collect();
        let results = vec![crate::emit::ir_to_wasm_valtype(func.result)];
        types.ty().function(params, results);
        user_type_indices.push(type_idx);
    }

    // _start 関数型: () -> ()
    let start_type_idx = types.len();
    types.ty().function(vec![], vec![]);

    wasm_module.section(&types);

    // === Import Section ===
    let mut imports = ImportSection::new();
    imports.import("wasi_snapshot_preview1", "fd_write", EntityType::Function(fd_write_type_idx));
    wasm_module.section(&imports);

    // === Function Section ===
    let mut functions = FunctionSection::new();
    functions.function(print_type_idx); // __print_i64
    functions.function(alloc_type_idx); // __alloc
    functions.function(string_concat_type_idx); // __string_concat
    functions.function(string_eq_type_idx); // __string_eq
    for &type_idx in &user_type_indices {
        functions.function(type_idx);
    }
    functions.function(start_type_idx); // _start
    wasm_module.section(&functions);

    // === Memory Section ===
    let mut memories = MemorySection::new();
    memories.memory(MemoryType {
        minimum: 1,
        maximum: None,
        memory64: false,
        shared: false,
        page_size_log2: None,
    });
    wasm_module.section(&memories);

    // === Global Section ===
    // $heap_ptr: ヒープポインタ（文字列定数データの末尾から開始）
    let total_string_data_size: i32 = module.string_data.iter()
        .map(|(_, bytes)| bytes.len() as i32)
        .sum();
    // 8バイトアラインメントに切り上げ
    let heap_start = ((512 + total_string_data_size) + 7) & !7;
    let mut globals = GlobalSection::new();
    globals.global(
        GlobalType {
            val_type: ValType::I32,
            mutable: true,
            shared: false,
        },
        &wasm_encoder::ConstExpr::i32_const(heap_start),
    );
    wasm_module.section(&globals);

    // === Export Section ===
    let mut exports = ExportSection::new();
    exports.export("memory", ExportKind::Memory, 0);
    exports.export("_start", ExportKind::Func, start_func_idx);
    wasm_module.section(&exports);

    // === Code Section ===
    let mut codes = CodeSection::new();

    // __print_i64
    emit_print_i64_func(&mut codes);

    // __alloc (Bump Allocator)
    emit_alloc_func(&mut codes);

    // __string_concat スタブ: 第1引数をそのまま返す
    {
        use wasm_encoder::Instruction as W;
        let mut f = wasm_encoder::Function::new(vec![]);
        f.instruction(&W::LocalGet(0));
        f.instruction(&W::End);
        codes.function(&f);
    }

    // __string_eq スタブ: 0 (false) を返す
    {
        use wasm_encoder::Instruction as W;
        let mut f = wasm_encoder::Function::new(vec![]);
        f.instruction(&W::I64Const(0));
        f.instruction(&W::End);
        codes.function(&f);
    }

    // ユーザー関数
    for func in &module.functions {
        let mut f = wasm_encoder::Function::new(
            func.locals
                .iter()
                .map(|t| (1, crate::emit::ir_to_wasm_valtype(*t)))
                .collect::<Vec<_>>(),
        );
        emit_instructions_wasi(
            &mut f, &func.body,
            print_helper_idx, alloc_func_idx,
            string_concat_idx, string_eq_idx,
            user_func_base,
        )?;
        f.instruction(&wasm_encoder::Instruction::End);
        codes.function(&f);
    }

    // _start
    {
        let mut f = wasm_encoder::Function::new(vec![]);
        if let Some(main_idx) = module.functions.iter().position(|f| f.name == "main") {
            f.instruction(&wasm_encoder::Instruction::Call(user_func_base + main_idx as u32));
            f.instruction(&wasm_encoder::Instruction::Drop);
        }
        f.instruction(&wasm_encoder::Instruction::End);
        codes.function(&f);
    }

    wasm_module.section(&codes);

    // === Data Section ===
    let mut data = DataSection::new();
    data.active(0, &wasm_encoder::ConstExpr::i32_const(NEWLINE_ADDR), b"\n".iter().copied());
    // 文字列定数データをデータセクションに格納
    let mut str_offset = 512i32;
    for (_label, bytes) in &module.string_data {
        data.active(0, &wasm_encoder::ConstExpr::i32_const(str_offset), bytes.iter().copied());
        str_offset += bytes.len() as i32;
    }
    wasm_module.section(&data);

    Ok(wasm_module.finish())
}

/// __print_i64: i64 の値を10進文字列に変換して stdout に出力
fn emit_print_i64_func(codes: &mut CodeSection) {
    use wasm_encoder::Instruction as W;
    use wasm_encoder::MemArg;

    let mem = |offset: u64| MemArg { offset, align: 0, memory_index: 0 };
    let mem32 = |offset: u64| MemArg { offset, align: 2, memory_index: 0 };

    let mut f = wasm_encoder::Function::new(vec![
        (1, ValType::I32), // local 1: pos
        (1, ValType::I32), // local 2: is_neg
        (1, ValType::I64), // local 3: abs_val
    ]);

    // pos = BUF_END
    f.instruction(&W::I32Const(BUF_END));
    f.instruction(&W::LocalSet(1));

    // abs_val = value
    f.instruction(&W::LocalGet(0));
    f.instruction(&W::LocalSet(3));

    // is_neg = 0
    f.instruction(&W::I32Const(0));
    f.instruction(&W::LocalSet(2));

    // if (value < 0) { is_neg = 1; abs_val = -value; }
    f.instruction(&W::LocalGet(0));
    f.instruction(&W::I64Const(0));
    f.instruction(&W::I64LtS);
    f.instruction(&W::If(wasm_encoder::BlockType::Empty));
    {
        f.instruction(&W::I32Const(1));
        f.instruction(&W::LocalSet(2));
        f.instruction(&W::I64Const(0));
        f.instruction(&W::LocalGet(0));
        f.instruction(&W::I64Sub);
        f.instruction(&W::LocalSet(3));
    }
    f.instruction(&W::End);

    // 特殊ケース: value == 0
    f.instruction(&W::LocalGet(3));
    f.instruction(&W::I64Eqz);
    f.instruction(&W::If(wasm_encoder::BlockType::Empty));
    {
        f.instruction(&W::LocalGet(1));
        f.instruction(&W::I32Const(1));
        f.instruction(&W::I32Sub);
        f.instruction(&W::LocalSet(1));
        f.instruction(&W::LocalGet(1));
        f.instruction(&W::I32Const(48));
        f.instruction(&W::I32Store8(mem(0)));
    }
    f.instruction(&W::Else);
    {
        f.instruction(&W::Block(wasm_encoder::BlockType::Empty));
        f.instruction(&W::Loop(wasm_encoder::BlockType::Empty));
        {
            f.instruction(&W::LocalGet(3));
            f.instruction(&W::I64Eqz);
            f.instruction(&W::BrIf(1));

            f.instruction(&W::LocalGet(1));
            f.instruction(&W::I32Const(1));
            f.instruction(&W::I32Sub);
            f.instruction(&W::LocalSet(1));

            f.instruction(&W::LocalGet(1));
            f.instruction(&W::LocalGet(3));
            f.instruction(&W::I64Const(10));
            f.instruction(&W::I64RemU);
            f.instruction(&W::I32WrapI64);
            f.instruction(&W::I32Const(48));
            f.instruction(&W::I32Add);
            f.instruction(&W::I32Store8(mem(0)));

            f.instruction(&W::LocalGet(3));
            f.instruction(&W::I64Const(10));
            f.instruction(&W::I64DivU);
            f.instruction(&W::LocalSet(3));

            f.instruction(&W::Br(0));
        }
        f.instruction(&W::End);
        f.instruction(&W::End);
    }
    f.instruction(&W::End);

    // 負号
    f.instruction(&W::LocalGet(2));
    f.instruction(&W::If(wasm_encoder::BlockType::Empty));
    {
        f.instruction(&W::LocalGet(1));
        f.instruction(&W::I32Const(1));
        f.instruction(&W::I32Sub);
        f.instruction(&W::LocalSet(1));
        f.instruction(&W::LocalGet(1));
        f.instruction(&W::I32Const(45));
        f.instruction(&W::I32Store8(mem(0)));
    }
    f.instruction(&W::End);

    // === fd_write: 数値出力 ===
    f.instruction(&W::I32Const(IOV_ADDR));
    f.instruction(&W::LocalGet(1));
    f.instruction(&W::I32Store(mem32(0)));
    f.instruction(&W::I32Const(IOV_ADDR + 4));
    f.instruction(&W::I32Const(BUF_END));
    f.instruction(&W::LocalGet(1));
    f.instruction(&W::I32Sub);
    f.instruction(&W::I32Store(mem32(0)));

    f.instruction(&W::I32Const(1));
    f.instruction(&W::I32Const(IOV_ADDR));
    f.instruction(&W::I32Const(1));
    f.instruction(&W::I32Const(NWRITTEN_ADDR));
    f.instruction(&W::Call(0));
    f.instruction(&W::Drop);

    // === fd_write: 改行出力 ===
    f.instruction(&W::I32Const(IOV_ADDR));
    f.instruction(&W::I32Const(NEWLINE_ADDR));
    f.instruction(&W::I32Store(mem32(0)));
    f.instruction(&W::I32Const(IOV_ADDR + 4));
    f.instruction(&W::I32Const(1));
    f.instruction(&W::I32Store(mem32(0)));

    f.instruction(&W::I32Const(1));
    f.instruction(&W::I32Const(IOV_ADDR));
    f.instruction(&W::I32Const(1));
    f.instruction(&W::I32Const(NWRITTEN_ADDR));
    f.instruction(&W::Call(0));
    f.instruction(&W::Drop);

    f.instruction(&W::End);
    codes.function(&f);
}

/// __alloc: Bump Allocator (i64 サイズ) -> i64 アドレス
///
/// 疑似コード:
///   let aligned_size = ((size as i32) + 7) & !7
///   let ptr = global.get $heap_ptr
///   let new_ptr = ptr + aligned_size
///   if new_ptr > memory.size * 65536:
///     memory.grow((new_ptr - available + 65535) / 65536)
///   global.set $heap_ptr = new_ptr
///   return ptr as i64
fn emit_alloc_func(codes: &mut CodeSection) {
    use wasm_encoder::Instruction as W;

    let mut f = wasm_encoder::Function::new(vec![
        (1, ValType::I32), // local 1: aligned_size
        (1, ValType::I32), // local 2: ptr (旧 heap_ptr)
        (1, ValType::I32), // local 3: new_ptr
    ]);

    // aligned_size = (i32.wrap(size) + 7) & ~7
    f.instruction(&W::LocalGet(0));    // size (i64)
    f.instruction(&W::I32WrapI64);     // i32 に変換
    f.instruction(&W::I32Const(7));
    f.instruction(&W::I32Add);
    f.instruction(&W::I32Const(-8));   // !7 = 0xFFFF_FFF8 = -8 as i32
    f.instruction(&W::I32And);
    f.instruction(&W::LocalSet(1));    // aligned_size

    // ptr = global.get $heap_ptr
    f.instruction(&W::GlobalGet(0));   // $heap_ptr
    f.instruction(&W::LocalSet(2));    // ptr

    // new_ptr = ptr + aligned_size
    f.instruction(&W::LocalGet(2));
    f.instruction(&W::LocalGet(1));
    f.instruction(&W::I32Add);
    f.instruction(&W::LocalSet(3));    // new_ptr

    // ページ不足チェック: if new_ptr > memory.size * 65536
    f.instruction(&W::LocalGet(3));              // new_ptr
    f.instruction(&W::MemorySize(0));            // memory.size (ページ数)
    f.instruction(&W::I32Const(65536));
    f.instruction(&W::I32Mul);                   // available = pages * 65536
    f.instruction(&W::I32GtU);                   // new_ptr > available?
    f.instruction(&W::If(wasm_encoder::BlockType::Empty));
    {
        // needed_pages = (new_ptr - available + 65535) / 65536
        f.instruction(&W::LocalGet(3));          // new_ptr
        f.instruction(&W::MemorySize(0));
        f.instruction(&W::I32Const(65536));
        f.instruction(&W::I32Mul);               // available
        f.instruction(&W::I32Sub);               // new_ptr - available
        f.instruction(&W::I32Const(65535));
        f.instruction(&W::I32Add);               // + 65535
        f.instruction(&W::I32Const(65536));
        f.instruction(&W::I32DivU);              // / 65536
        f.instruction(&W::MemoryGrow(0));
        f.instruction(&W::Drop);                 // memory.grow の戻り値を捨てる
    }
    f.instruction(&W::End);

    // global.set $heap_ptr = new_ptr
    f.instruction(&W::LocalGet(3));
    f.instruction(&W::GlobalSet(0));

    // return ptr as i64
    f.instruction(&W::LocalGet(2));
    f.instruction(&W::I64ExtendI32U);

    f.instruction(&W::End);
    codes.function(&f);
}

/// IR の import_count (print=0, __alloc=1, __string_concat=2, __string_eq=3)
const IR_IMPORT_COUNT: u32 = 4;

/// IR 命令を WASI 用にリマップして出力
fn emit_instructions_wasi(
    func: &mut wasm_encoder::Function,
    instructions: &[Instruction],
    print_helper_idx: u32,
    alloc_func_idx: u32,
    str_concat_idx: u32,
    str_eq_idx: u32,
    user_func_base: u32,
) -> Result<(), CodegenError> {
    use wasm_encoder::Instruction as W;
    crate::emit::emit_instructions_common(func, instructions, |f, i| {
        if i == 0 {
            // print -> __print_i64
            f.instruction(&W::Call(print_helper_idx));
        } else if i == 1 {
            // __alloc -> Bump Allocator
            f.instruction(&W::Call(alloc_func_idx));
        } else if i == 2 {
            // __string_concat
            f.instruction(&W::Call(str_concat_idx));
        } else if i == 3 {
            // __string_eq
            f.instruction(&W::Call(str_eq_idx));
        } else {
            // ユーザー関数: IR_IMPORT_COUNT 分ずらす
            f.instruction(&W::Call(user_func_base + (i - IR_IMPORT_COUNT)));
        }
        Ok(())
    })
}


#[cfg(test)]
mod tests {
    use super::*;
    use lsharp_ir::lower::Lower;
    use lsharp_types::infer::Infer;

    /// ソースコードから WASI 対応 Wasm バイナリを生成
    fn compile_wasi(source: &str) -> Vec<u8> {
        let program = lsharp_syntax::parse(source).unwrap();
        let mut infer = Infer::new();
        let type_results = infer.infer_program(&program).unwrap();
        let mut lower = Lower::new();
        let module = lower.lower_program(&program, &type_results).unwrap();
        emit_wasm_wasi(&module).unwrap()
    }

    /// Wasm バイナリを WASI 環境で実行し、stdout 出力を返す
    fn run_wasi(wasm_bytes: &[u8]) -> String {
        use wasmtime::*;
        use wasmtime_wasi::{WasiCtxBuilder, preview1::WasiP1Ctx};

        let engine = Engine::default();
        let mut linker = Linker::<WasiP1Ctx>::new(&engine);
        wasmtime_wasi::preview1::add_to_linker_sync(&mut linker, |t| t).unwrap();

        let stdout = wasmtime_wasi::pipe::MemoryOutputPipe::new(1024);

        let wasi = WasiCtxBuilder::new()
            .stdout(stdout.clone())
            .build_p1();

        let mut store = Store::new(&engine, wasi);
        let module = wasmtime::Module::new(&engine, wasm_bytes).unwrap();
        let instance = linker.instantiate(&mut store, &module).unwrap();

        let start = instance.get_typed_func::<(), ()>(&mut store, "_start").unwrap();
        start.call(&mut store, ()).unwrap();

        drop(store);
        let bytes = stdout.try_into_inner().unwrap();
        String::from_utf8(bytes.to_vec()).unwrap()
    }

    #[test]
    fn test_wasi_print_positive() {
        let wasm = compile_wasi("(defn main [] (print 42))");
        let output = run_wasi(&wasm);
        assert_eq!(output, "42\n");
    }

    #[test]
    fn test_wasi_print_zero() {
        let wasm = compile_wasi("(defn main [] (print 0))");
        let output = run_wasi(&wasm);
        assert_eq!(output, "0\n");
    }

    #[test]
    fn test_wasi_print_large_number() {
        let wasm = compile_wasi("(defn main [] (print 1234567890))");
        let output = run_wasi(&wasm);
        assert_eq!(output, "1234567890\n");
    }

    #[test]
    fn test_wasi_print_one() {
        let wasm = compile_wasi("(defn main [] (print 1))");
        let output = run_wasi(&wasm);
        assert_eq!(output, "1\n");
    }

    #[test]
    fn test_wasi_print_arithmetic_result() {
        let wasm = compile_wasi("(defn main [] (print (+ (* 3 4) 5)))");
        let output = run_wasi(&wasm);
        assert_eq!(output, "17\n");
    }

    #[test]
    fn test_wasi_multiple_prints() {
        let wasm = compile_wasi(
            "(defn main [] (do (print 1) (print 2) (print 3) 0))",
        );
        let output = run_wasi(&wasm);
        assert_eq!(output, "1\n2\n3\n");
    }

    #[test]
    fn test_wasi_print_function_result() {
        let wasm = compile_wasi(
            "(defn double [x] (* x 2))
             (defn main [] (print (double 21)))",
        );
        let output = run_wasi(&wasm);
        assert_eq!(output, "42\n");
    }

    #[test]
    fn test_wasi_print_fib() {
        let wasm = compile_wasi(
            "(defn fib [n]
               (if (<= n 1)
                 n
                 (+ (fib (- n 1)) (fib (- n 2)))))
             (defn main [] (print (fib 10)))",
        );
        let output = run_wasi(&wasm);
        assert_eq!(output, "55\n");
    }

    #[test]
    fn test_wasi_gc_type_section_with_record() {
        // レコード型がある場合、TypeSection に GC 型定義が出力される
        // wasmtime の GC feature が未有効のため、バイナリ生成のみ検証
        let wasm = compile_wasi(
            "(type Point (record (: x Int) (: y Int)))
             (defn main [] (print 42))",
        );
        // Wasm バイナリが正常に生成されること
        assert!(wasm.len() > 8);
        // Wasm マジックナンバーの確認
        assert_eq!(&wasm[0..4], b"\0asm");
        // GC 型定義が含まれている場合、バイナリサイズが通常より大きいこと
        let wasm_no_gc = compile_wasi("(defn main [] (print 42))");
        assert!(
            wasm.len() > wasm_no_gc.len(),
            "GC 型を含むバイナリ({})は通常のバイナリ({})より大きいはず",
            wasm.len(),
            wasm_no_gc.len()
        );
    }
}
