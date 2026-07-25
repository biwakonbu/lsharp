use wasm_encoder::{CodeSection, ValType};

/// __write_file: String オブジェクトパスと String オブジェクト内容を受け取り、書き込みバイト数を返す
pub(super) fn emit_write_file_func(
    codes: &mut CodeSection,
    path_open_idx: u32,
    fd_write_idx: u32,
    fd_close_idx: u32,
) {
    use wasm_encoder::Instruction as W;

    // locals: 0=path(i64), 1=content(i64), 2=path_offset(i32), 3=path_len(i32),
    //         4=content_offset(i32), 5=content_len(i32), 6=fd(i32), 7=nwritten(i32)
    let mut f = wasm_encoder::Function::new(vec![
        (1, ValType::I32), // 2: path_offset (= path_addr + 8)
        (1, ValType::I32), // 3: path_len
        (1, ValType::I32), // 4: content_offset (= content_addr + 8)
        (1, ValType::I32), // 5: content_len
        (1, ValType::I32), // 6: fd
        (1, ValType::I32), // 7: nwritten
    ]);

    // パスの bytes を取得: path_offset = path_addr + 8
    f.instruction(&W::LocalGet(0));
    f.instruction(&W::I32WrapI64);
    f.instruction(&W::I32Const(8));
    f.instruction(&W::I32Add);
    f.instruction(&W::LocalSet(2)); // path_offset

    // path_len = i32.load(path_addr + 4)
    f.instruction(&W::LocalGet(0));
    f.instruction(&W::I32WrapI64);
    f.instruction(&W::I32Load(wasm_encoder::MemArg {
        offset: 4,
        align: 2,
        memory_index: 0,
    }));
    f.instruction(&W::LocalSet(3)); // path_len

    // 内容の bytes を取得: content_offset = content_addr + 8
    f.instruction(&W::LocalGet(1));
    f.instruction(&W::I32WrapI64);
    f.instruction(&W::I32Const(8));
    f.instruction(&W::I32Add);
    f.instruction(&W::LocalSet(4)); // content_offset

    // content_len = i32.load(content_addr + 4)
    f.instruction(&W::LocalGet(1));
    f.instruction(&W::I32WrapI64);
    f.instruction(&W::I32Load(wasm_encoder::MemArg {
        offset: 4,
        align: 2,
        memory_index: 0,
    }));
    f.instruction(&W::LocalSet(5)); // content_len

    // path_open(dirfd=3, dirflags=0, path, path_len, oflags=1(creat)|4(trunc), rights, 0, 0, fd_ptr=280)
    f.instruction(&W::I32Const(3)); // dirfd = 3
    f.instruction(&W::I32Const(0)); // dirflags
    f.instruction(&W::LocalGet(2)); // path
    f.instruction(&W::LocalGet(3)); // path_len
    f.instruction(&W::I32Const(5)); // oflags = O_CREAT(1) | O_TRUNC(4)
    f.instruction(&W::I64Const(0x40)); // rights_base = fd_write
    f.instruction(&W::I64Const(0)); // rights_inheriting
    f.instruction(&W::I32Const(0)); // fdflags
    f.instruction(&W::I32Const(280)); // fd_ptr
    f.instruction(&W::Call(path_open_idx));
    f.instruction(&W::LocalSet(3)); // path_open errno (path_len は以後不要)

    // open 失敗時は fd_write / fd_close を呼ばず、-1 を返す。
    f.instruction(&W::LocalGet(3));
    f.instruction(&W::I32Const(0));
    f.instruction(&W::I32Ne);
    f.instruction(&W::If(wasm_encoder::BlockType::Empty));
    f.instruction(&W::I32Const(-1));
    f.instruction(&W::LocalSet(7));
    f.instruction(&W::LocalGet(7));
    f.instruction(&W::I64ExtendI32S);
    f.instruction(&W::Return);
    f.instruction(&W::End);

    // fd を読み出し
    f.instruction(&W::I32Const(280));
    f.instruction(&W::I32Load(wasm_encoder::MemArg {
        offset: 0,
        align: 2,
        memory_index: 0,
    }));
    f.instruction(&W::LocalSet(6)); // fd

    // iov 設定 (スクラッチ 352)
    f.instruction(&W::I32Const(352));
    f.instruction(&W::LocalGet(4));
    f.instruction(&W::I32Store(wasm_encoder::MemArg {
        offset: 0,
        align: 2,
        memory_index: 0,
    })); // iov.buf

    f.instruction(&W::I32Const(352));
    f.instruction(&W::LocalGet(5));
    f.instruction(&W::I32Store(wasm_encoder::MemArg {
        offset: 4,
        align: 2,
        memory_index: 0,
    })); // iov.len

    // fd_write(fd, iovs=352, iovs_len=1, nwritten_ptr=360)
    f.instruction(&W::LocalGet(6)); // fd
    f.instruction(&W::I32Const(352)); // iovs
    f.instruction(&W::I32Const(1)); // iovs_len
    f.instruction(&W::I32Const(360)); // nwritten
    f.instruction(&W::Call(fd_write_idx));
    f.instruction(&W::Drop);

    // nwritten を読み取り
    f.instruction(&W::I32Const(360));
    f.instruction(&W::I32Load(wasm_encoder::MemArg {
        offset: 0,
        align: 2,
        memory_index: 0,
    }));
    f.instruction(&W::LocalSet(7));

    // fd_close の errno を path_len local に保存する。
    f.instruction(&W::LocalGet(6));
    f.instruction(&W::Call(fd_close_idx));
    f.instruction(&W::LocalSet(3));
    f.instruction(&W::LocalGet(3));
    f.instruction(&W::I32Const(0));
    f.instruction(&W::I32Ne);
    f.instruction(&W::If(wasm_encoder::BlockType::Empty));
    f.instruction(&W::I32Const(-1));
    f.instruction(&W::LocalSet(7));
    f.instruction(&W::End);

    // 書き込みバイト数を返す
    f.instruction(&W::LocalGet(7));
    f.instruction(&W::I64ExtendI32S);

    f.instruction(&W::End);
    codes.function(&f);
}
