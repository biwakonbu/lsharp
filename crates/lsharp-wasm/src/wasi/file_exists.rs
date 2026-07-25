use wasm_encoder::{CodeSection, ValType};

/// __file_exists: String オブジェクトパスを受け取り、存在すれば 1、しなければ 0 を返す
pub(super) fn emit_file_exists_func(
    codes: &mut CodeSection,
    path_open_idx: u32,
    fd_close_idx: u32,
) {
    use wasm_encoder::Instruction as W;

    // locals: 0=path(i64), 1=path_offset(i32), 2=path_len(i32), 3=errno(i32)
    let mut f = wasm_encoder::Function::new(vec![
        (1, ValType::I32), // 1: path_offset (= path_addr + 8)
        (1, ValType::I32), // 2: path_len
        (1, ValType::I32), // 3: errno
    ]);

    // String オブジェクトからパス情報を取得
    // path_offset = path_addr + 8
    f.instruction(&W::LocalGet(0));
    f.instruction(&W::I32WrapI64);
    f.instruction(&W::I32Const(8));
    f.instruction(&W::I32Add);
    f.instruction(&W::LocalSet(1)); // path_offset

    // path_len = i32.load(path_addr + 4)
    f.instruction(&W::LocalGet(0));
    f.instruction(&W::I32WrapI64);
    f.instruction(&W::I32Load(wasm_encoder::MemArg {
        offset: 4,
        align: 2,
        memory_index: 0,
    }));
    f.instruction(&W::LocalSet(2)); // path_len

    // path_open(dirfd=3, 0, path, path_len, 0, rights, 0, 0, fd_ptr=280)
    f.instruction(&W::I32Const(3)); // dirfd = 3
    f.instruction(&W::I32Const(0)); // dirflags
    f.instruction(&W::LocalGet(1)); // path
    f.instruction(&W::LocalGet(2)); // path_len
    f.instruction(&W::I32Const(0)); // oflags = 0 (read)
    f.instruction(&W::I64Const(0x02)); // rights_base = fd_read
    f.instruction(&W::I64Const(0)); // rights_inheriting
    f.instruction(&W::I32Const(0)); // fdflags
    f.instruction(&W::I32Const(280)); // fd_ptr
    f.instruction(&W::Call(path_open_idx));
    f.instruction(&W::LocalSet(3)); // errno

    // errno == 0 → ファイル存在、fd_close して 1 を返す
    f.instruction(&W::LocalGet(3));
    f.instruction(&W::I32Eqz);
    f.instruction(&W::If(wasm_encoder::BlockType::Empty));
    // fd_close
    f.instruction(&W::I32Const(280));
    f.instruction(&W::I32Load(wasm_encoder::MemArg {
        offset: 0,
        align: 2,
        memory_index: 0,
    }));
    f.instruction(&W::Call(fd_close_idx));
    f.instruction(&W::LocalSet(3)); // close errno: open 成功でも close 失敗は存在判定を失敗にする
    f.instruction(&W::End);

    // 結果: errno == 0 なら 1、それ以外 0
    f.instruction(&W::LocalGet(3));
    f.instruction(&W::I32Eqz);
    f.instruction(&W::I64ExtendI32U);

    f.instruction(&W::End);
    codes.function(&f);
}
