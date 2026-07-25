use wasm_encoder::{CodeSection, ValType};

use super::emit_tagged_pointer_from_i32_local;

/// __read_file: String オブジェクトパスを受け取り、ファイル内容を String オブジェクトで返す
/// path_open → fd_filestat_get → __alloc → fd_read → fd_close
pub(super) fn emit_read_file_func(
    codes: &mut CodeSection,
    alloc_func_idx: u32,
    path_open_idx: u32,
    fd_read_idx: u32,
    fd_close_idx: u32,
    fd_filestat_get_idx: u32,
) {
    use wasm_encoder::Instruction as W;

    // locals: 0=path(i64 param), 1=path_offset(i32), 2=path_len(i32), 3=fd(i32),
    //         4=file_size(i32), 5=buf_addr(i32), 6=fd_read_errno(i32), 7=nread(i32),
    //         8=path_open_errno(i32), 9=fd_filestat_get_errno(i32)
    let mut f = wasm_encoder::Function::new(vec![
        (1, ValType::I32), // 1: path_offset (bytes の開始アドレス = path_addr + 8)
        (1, ValType::I32), // 2: path_len
        (1, ValType::I32), // 3: fd
        (1, ValType::I32), // 4: file_size
        (1, ValType::I32), // 5: buf_addr (String オブジェクトのアドレス)
        (1, ValType::I32), // 6: fd_read_errno
        (1, ValType::I32), // 7: nread
        (1, ValType::I32), // 8: path_open_errno
        (1, ValType::I32), // 9: fd_filestat_get_errno
    ]);

    // String オブジェクトからパス情報を取得
    // path_offset = path_addr + 8 (bytes の開始位置)
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

    // fd を格納するスクラッチ領域 (アドレス 280)
    // path_open(dirfd=3, dirflags=0, path, path_len, oflags=0, rights_base, rights_inheriting, fdflags=0, fd_ptr)
    f.instruction(&W::I32Const(3)); // dirfd = 3 (preopened dir)
    f.instruction(&W::I32Const(0)); // dirflags = 0
    f.instruction(&W::LocalGet(1)); // path
    f.instruction(&W::LocalGet(2)); // path_len
    f.instruction(&W::I32Const(0)); // oflags = 0 (read only)
    f.instruction(&W::I64Const(0x42)); // rights_base = fd_read | fd_seek | fd_filestat_get
    f.instruction(&W::I64Const(0)); // rights_inheriting
    f.instruction(&W::I32Const(0)); // fdflags = 0
    f.instruction(&W::I32Const(280)); // fd_ptr (スクラッチ領域)
    f.instruction(&W::Call(path_open_idx));
    f.instruction(&W::LocalSet(8)); // path_open errno

    // open 失敗時は未初期化の fd を使わず、空文字列を返す。
    f.instruction(&W::LocalGet(8));
    f.instruction(&W::I32Const(0));
    f.instruction(&W::I32Ne);
    f.instruction(&W::If(wasm_encoder::BlockType::Empty));
    f.instruction(&W::I64Const(8));
    f.instruction(&W::Call(alloc_func_idx));
    f.instruction(&W::I32WrapI64);
    f.instruction(&W::LocalSet(5));
    f.instruction(&W::LocalGet(5));
    f.instruction(&W::I32Const(1));
    f.instruction(&W::I32Store(wasm_encoder::MemArg {
        offset: 0,
        align: 2,
        memory_index: 0,
    }));
    f.instruction(&W::LocalGet(5));
    f.instruction(&W::I32Const(0));
    f.instruction(&W::I32Store(wasm_encoder::MemArg {
        offset: 4,
        align: 2,
        memory_index: 0,
    }));
    emit_tagged_pointer_from_i32_local(&mut f, 5);
    f.instruction(&W::Return);
    f.instruction(&W::End);

    // fd を読み出し
    f.instruction(&W::I32Const(280));
    f.instruction(&W::I32Load(wasm_encoder::MemArg {
        offset: 0,
        align: 2,
        memory_index: 0,
    }));
    f.instruction(&W::LocalSet(3)); // fd

    // fd_filestat_get でファイルサイズ取得 (stat バッファは 288 から 64 バイト)
    f.instruction(&W::LocalGet(3)); // fd
    f.instruction(&W::I32Const(288)); // stat buf (288..352)
    f.instruction(&W::Call(fd_filestat_get_idx));
    f.instruction(&W::LocalSet(9)); // fd_filestat_get errno

    // stat 失敗時は開いた fd を閉じ、空文字列を返す。
    f.instruction(&W::LocalGet(9));
    f.instruction(&W::I32Const(0));
    f.instruction(&W::I32Ne);
    f.instruction(&W::If(wasm_encoder::BlockType::Empty));
    f.instruction(&W::LocalGet(3));
    f.instruction(&W::Call(fd_close_idx));
    f.instruction(&W::LocalSet(8)); // close errno は結果を返さず fail-closed
    f.instruction(&W::I64Const(8));
    f.instruction(&W::Call(alloc_func_idx));
    f.instruction(&W::I32WrapI64);
    f.instruction(&W::LocalSet(5));
    f.instruction(&W::LocalGet(5));
    f.instruction(&W::I32Const(1));
    f.instruction(&W::I32Store(wasm_encoder::MemArg {
        offset: 0,
        align: 2,
        memory_index: 0,
    }));
    f.instruction(&W::LocalGet(5));
    f.instruction(&W::I32Const(0));
    f.instruction(&W::I32Store(wasm_encoder::MemArg {
        offset: 4,
        align: 2,
        memory_index: 0,
    }));
    emit_tagged_pointer_from_i32_local(&mut f, 5);
    f.instruction(&W::Return);
    f.instruction(&W::End);

    // file_size = stat[32..40] の下位 32bit (filesize は offset 32 の i64)
    f.instruction(&W::I32Const(288));
    f.instruction(&W::I32Load(wasm_encoder::MemArg {
        offset: 32,
        align: 2,
        memory_index: 0,
    })); // stat.st_size の下位 32bit
    f.instruction(&W::LocalSet(4)); // file_size

    // String オブジェクト確保: __alloc(8 + file_size)
    f.instruction(&W::LocalGet(4));
    f.instruction(&W::I32Const(8));
    f.instruction(&W::I32Add);
    f.instruction(&W::I64ExtendI32U);
    f.instruction(&W::Call(alloc_func_idx));
    f.instruction(&W::I32WrapI64);
    f.instruction(&W::LocalSet(5)); // buf_addr = String オブジェクトのアドレス
    // tag = 1
    f.instruction(&W::LocalGet(5));
    f.instruction(&W::I32Const(1));
    f.instruction(&W::I32Store(wasm_encoder::MemArg {
        offset: 0,
        align: 2,
        memory_index: 0,
    }));
    // len = file_size (後で nread に更新)
    f.instruction(&W::LocalGet(5));
    f.instruction(&W::LocalGet(4));
    f.instruction(&W::I32Store(wasm_encoder::MemArg {
        offset: 4,
        align: 2,
        memory_index: 0,
    }));

    // iov を設定: iov[0].buf = buf_addr + 8, iov[0].len = file_size (スクラッチ 352)
    f.instruction(&W::I32Const(352));
    f.instruction(&W::LocalGet(5));
    f.instruction(&W::I32Const(8));
    f.instruction(&W::I32Add);
    f.instruction(&W::I32Store(wasm_encoder::MemArg {
        offset: 0,
        align: 2,
        memory_index: 0,
    })); // iov.buf

    f.instruction(&W::I32Const(352));
    f.instruction(&W::LocalGet(4));
    f.instruction(&W::I32Store(wasm_encoder::MemArg {
        offset: 4,
        align: 2,
        memory_index: 0,
    })); // iov.len

    // fd_read(fd, iov_ptr=352, iov_count=1, nread_ptr=360)
    f.instruction(&W::LocalGet(3)); // fd
    f.instruction(&W::I32Const(352)); // iovs
    f.instruction(&W::I32Const(1)); // iovs_len
    f.instruction(&W::I32Const(360)); // nread ptr
    f.instruction(&W::Call(fd_read_idx));
    f.instruction(&W::LocalSet(6)); // errno

    // nread を読み取り
    f.instruction(&W::I32Const(360));
    f.instruction(&W::I32Load(wasm_encoder::MemArg {
        offset: 0,
        align: 2,
        memory_index: 0,
    }));
    f.instruction(&W::LocalSet(7)); // nread

    // fd_read errno は payload を公開せず fail-closed にする。
    f.instruction(&W::LocalGet(6));
    f.instruction(&W::I32Const(0));
    f.instruction(&W::I32Ne);
    f.instruction(&W::If(wasm_encoder::BlockType::Empty));
    f.instruction(&W::I32Const(0));
    f.instruction(&W::LocalSet(7));
    f.instruction(&W::End);

    // fd_close の errno を保持し、close 失敗時は payload を公開しない。
    f.instruction(&W::LocalGet(3));
    f.instruction(&W::Call(fd_close_idx));
    f.instruction(&W::LocalSet(8));

    f.instruction(&W::LocalGet(8));
    f.instruction(&W::I32Const(0));
    f.instruction(&W::I32Ne);
    f.instruction(&W::If(wasm_encoder::BlockType::Empty));
    f.instruction(&W::I32Const(0));
    f.instruction(&W::LocalSet(7));
    f.instruction(&W::End);

    // String オブジェクトの len を nread に更新
    f.instruction(&W::LocalGet(5));
    f.instruction(&W::LocalGet(7));
    f.instruction(&W::I32Store(wasm_encoder::MemArg {
        offset: 4,
        align: 2,
        memory_index: 0,
    }));
    // タグ付き String handle を返す
    emit_tagged_pointer_from_i32_local(&mut f, 5);

    f.instruction(&W::End);
    codes.function(&f);
}
