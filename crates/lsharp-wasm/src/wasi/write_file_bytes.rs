use wasm_encoder::{CodeSection, ValType};

/// __write_file_bytes: Vector の各要素の下位 8 bit を raw bytes として書き込む。
///
/// 呼び出し元は path/vector を root stack に保持してからこの helper を呼ぶため、
/// packed buffer の確保中に GC が走っても入力オブジェクトは回収されない。
pub(super) fn emit_write_file_bytes_func(
    codes: &mut CodeSection,
    alloc_func_idx: u32,
    path_open_idx: u32,
    fd_write_idx: u32,
    fd_close_idx: u32,
) {
    use wasm_encoder::{Instruction as W, MemArg};

    let mem32 = |offset: u64| MemArg {
        offset,
        align: 2,
        memory_index: 0,
    };
    let mem64 = |offset: u64| MemArg {
        offset,
        align: 3,
        memory_index: 0,
    };

    // params: 0=path(i64), 1=Vector(i64)
    // locals: 2=path_offset, 3=path_len, 4=vector_addr, 5=vector_len,
    //         6=buffer_addr, 7=index, 8=fd, 9=nwritten (all i32)
    let mut f = wasm_encoder::Function::new(vec![(8, ValType::I32)]);

    // String path の bytes を取得する。
    f.instruction(&W::LocalGet(0));
    f.instruction(&W::I32WrapI64);
    f.instruction(&W::I32Const(8));
    f.instruction(&W::I32Add);
    f.instruction(&W::LocalSet(2));
    f.instruction(&W::LocalGet(0));
    f.instruction(&W::I32WrapI64);
    f.instruction(&W::I32Load(mem32(4)));
    f.instruction(&W::LocalSet(3));

    // Vector layout: [tag:i32, capacity:i32, length:i32, pad:i32, i64 elements...]
    f.instruction(&W::LocalGet(1));
    f.instruction(&W::I32WrapI64);
    f.instruction(&W::LocalSet(4));
    f.instruction(&W::LocalGet(4));
    f.instruction(&W::I32Load(mem32(8)));
    f.instruction(&W::LocalSet(5));

    // Vector の i64 要素を packed bytes へ詰めるための一時バッファを確保する。
    f.instruction(&W::LocalGet(5));
    f.instruction(&W::I64ExtendI32U);
    f.instruction(&W::Call(alloc_func_idx));
    f.instruction(&W::I32WrapI64);
    f.instruction(&W::LocalSet(6));

    f.instruction(&W::I32Const(0));
    f.instruction(&W::LocalSet(7));
    f.instruction(&W::Block(wasm_encoder::BlockType::Empty));
    f.instruction(&W::Loop(wasm_encoder::BlockType::Empty));
    f.instruction(&W::LocalGet(7));
    f.instruction(&W::LocalGet(5));
    f.instruction(&W::I32GeU);
    f.instruction(&W::BrIf(1));

    // buffer[index] = low_byte(vector[index])
    f.instruction(&W::LocalGet(6));
    f.instruction(&W::LocalGet(7));
    f.instruction(&W::I32Add);
    f.instruction(&W::LocalGet(4));
    f.instruction(&W::I32Const(16));
    f.instruction(&W::I32Add);
    f.instruction(&W::LocalGet(7));
    f.instruction(&W::I32Const(3));
    f.instruction(&W::I32Shl);
    f.instruction(&W::I32Add);
    f.instruction(&W::I64Load(mem64(0)));
    f.instruction(&W::I32WrapI64);
    f.instruction(&W::I32Store8(MemArg {
        offset: 0,
        align: 0,
        memory_index: 0,
    }));

    f.instruction(&W::LocalGet(7));
    f.instruction(&W::I32Const(1));
    f.instruction(&W::I32Add);
    f.instruction(&W::LocalSet(7));
    f.instruction(&W::Br(0));
    f.instruction(&W::End);
    f.instruction(&W::End);

    // path_open(dirfd=3, dirflags=0, path, path_len, oflags=CREAT|TRUNC,
    //           rights=fd_write, rights_inheriting=0, fdflags=0, fd_ptr=280)
    f.instruction(&W::I32Const(3));
    f.instruction(&W::I32Const(0));
    f.instruction(&W::LocalGet(2));
    f.instruction(&W::LocalGet(3));
    f.instruction(&W::I32Const(5));
    f.instruction(&W::I64Const(0x40));
    f.instruction(&W::I64Const(0));
    f.instruction(&W::I32Const(0));
    f.instruction(&W::I32Const(280));
    f.instruction(&W::Call(path_open_idx));
    f.instruction(&W::LocalSet(3)); // path_open errno (path_len は以後不要)

    // open 失敗時は fd_write / fd_close を呼ばず、-1 を返す。
    f.instruction(&W::LocalGet(3));
    f.instruction(&W::I32Const(0));
    f.instruction(&W::I32Ne);
    f.instruction(&W::If(wasm_encoder::BlockType::Empty));
    f.instruction(&W::I32Const(-1));
    f.instruction(&W::LocalSet(9));
    f.instruction(&W::LocalGet(9));
    f.instruction(&W::I64ExtendI32S);
    f.instruction(&W::Return);
    f.instruction(&W::End);

    f.instruction(&W::I32Const(280));
    f.instruction(&W::I32Load(mem32(0)));
    f.instruction(&W::LocalSet(8));

    // iovec = { buffer_addr, vector_len } at scratch 352.
    f.instruction(&W::I32Const(352));
    f.instruction(&W::LocalGet(6));
    f.instruction(&W::I32Store(mem32(0)));
    f.instruction(&W::I32Const(352));
    f.instruction(&W::LocalGet(5));
    f.instruction(&W::I32Store(mem32(4)));

    f.instruction(&W::LocalGet(8));
    f.instruction(&W::I32Const(352));
    f.instruction(&W::I32Const(1));
    f.instruction(&W::I32Const(360));
    f.instruction(&W::Call(fd_write_idx));
    f.instruction(&W::Drop);

    f.instruction(&W::I32Const(360));
    f.instruction(&W::I32Load(mem32(0)));
    f.instruction(&W::LocalSet(9));
    f.instruction(&W::LocalGet(8));
    f.instruction(&W::Call(fd_close_idx));
    f.instruction(&W::LocalSet(7));
    f.instruction(&W::LocalGet(7));
    f.instruction(&W::I32Const(0));
    f.instruction(&W::I32Ne);
    f.instruction(&W::If(wasm_encoder::BlockType::Empty));
    f.instruction(&W::I32Const(-1));
    f.instruction(&W::LocalSet(9));
    f.instruction(&W::End);

    f.instruction(&W::LocalGet(9));
    f.instruction(&W::I64ExtendI32S);
    f.instruction(&W::End);
    codes.function(&f);
}
