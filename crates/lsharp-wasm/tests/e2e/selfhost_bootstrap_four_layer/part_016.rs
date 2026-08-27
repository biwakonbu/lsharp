#[test]
#[ignore]
fn test_debug_func_49_context() {
    // stage2 のwasm 49 番の関数が何をしているか確認
    let main_path = selfhost_main_path();
    let selfhost_root = main_path
        .parent()
        .unwrap()
        .parent()
        .unwrap()
        .parent()
        .unwrap()
        .to_path_buf();
    let stage1_wasm = compile_file_only(&main_path);
    let output = lsharp_wasm::wasi_runner::run_wasm_wasi_with_dir_and_args(
        &stage1_wasm,
        Some(&selfhost_root),
        &["compiler", "src/App/Main.ls"],
    )
    .expect("stage1 failed");
    let modules = parse_emitted_wasm_modules(&output, 1);
    let stage2 = &modules[0];

    // Count imports
    let mut pos = 8usize;
    let mut import_count = 0u32;
    let data = stage2.as_slice();
    while pos < data.len() {
        let sid = data[pos];
        pos += 1;
        let mut size = 0u32;
        let mut shift = 0;
        loop {
            let b = data[pos];
            pos += 1;
            size |= ((b & 0x7f) as u32) << shift;
            if b & 0x80 == 0 {
                break;
            }
            shift += 7;
        }
        if sid == 2 {
            // count imports
            let mut v = 0u32;
            let mut sh = 0;
            let mut i = pos;
            loop {
                let b = data[i];
                i += 1;
                v |= ((b & 0x7f) as u32) << sh;
                if b & 0x80 == 0 {
                    break;
                }
                sh += 7;
            }
            import_count = v;
        }
        if sid == 3 {
            // count user funcs
            let mut v = 0u32;
            let mut sh = 0;
            let mut i = pos;
            loop {
                let b = data[i];
                i += 1;
                v |= ((b & 0x7f) as u32) << sh;
                if b & 0x80 == 0 {
                    break;
                }
                sh += 7;
            }
            eprintln!(
                "stage2: {} imports, {} user funcs, total={}",
                import_count,
                v,
                import_count + v
            );
            break;
        }
        pos += size as usize;
    }
}

#[test]
#[ignore]
fn test_debug_tok_eof_in_stage2() {
    // Token.ls main が tok-eof を正しく呼べるか確認
    // stage2の func 49 (Token::main) がちゃんと call 48 を使うか確認
    let main_path = selfhost_main_path();
    let selfhost_dir = selfhost_package_root();
    let stage1_wasm = compile_file_only(&main_path);
    let output = lsharp_wasm::wasi_runner::run_wasm_wasi_with_dir_and_args(
        &stage1_wasm,
        Some(&selfhost_dir),
        &["compiler", "src/App/Main.ls"],
    )
    .expect("stage1 failed");
    let modules = parse_emitted_wasm_modules(&output, 1);
    let stage2 = &modules[0];

    // Find func 49 bytecode
    let data = stage2.as_slice();
    fn read_leb(data: &[u8], pos: &mut usize) -> u64 {
        let mut v = 0u64;
        let mut shift = 0;
        loop {
            let b = data[*pos];
            *pos += 1;
            v |= ((b & 0x7f) as u64) << shift;
            if b & 0x80 == 0 {
                break;
            }
            shift += 7;
        }
        v
    }
    let mut pos = 8usize;
    while pos < data.len() {
        let sid = data[pos];
        pos += 1;
        let size = read_leb(data, &mut pos) as usize;
        if sid == 10 {
            // code section
            let _count = read_leb(data, &mut pos);
            // Skip to func 49 (index 49 in code section, which = func 49-6=43 user func)
            // actually each func in code section is 0-indexed: func 0 = user func 0, func 43 = user func 43
            // func 49 in wasm = imports(6) + user_func_43
            // code section func 43 (0-indexed)
            for _ in 0..43 {
                let sz = read_leb(data, &mut pos) as usize;
                pos += sz;
            }
            let func_size = read_leb(data, &mut pos) as usize;
            eprintln!("Code func 43 (Token::main) size={} bytes", func_size);
            let func_end = pos + func_size;
            let local_count = read_leb(data, &mut pos);
            for _ in 0..local_count {
                let _n = read_leb(data, &mut pos);
                let _t = data[pos];
                pos += 1;
            }
            // Dump the instructions
            while pos < func_end {
                let op = data[pos];
                pos += 1;
                match op {
                    0x10 => {
                        let idx = read_leb(data, &mut pos);
                        eprintln!("  call {idx}");
                    }
                    0x42 => {
                        let v = read_leb(data, &mut pos);
                        eprintln!("  i64.const {v}");
                    }
                    0x1a => eprintln!("  drop"),
                    0x0b => {
                        eprintln!("  end");
                        break;
                    }
                    _ => eprintln!("  op 0x{op:02x}"),
                }
            }
            break;
        }
        pos += size;
    }
}

#[test]
#[ignore]
fn test_debug_token_ls_compilation() {
    // Token.ls だけをコンパイルして tok-eof (func 42) が正しい index を持つか確認
    let token_path = selfhost_source_path("Token.ls");
    let token_src = std::fs::read_to_string(&token_path).unwrap();
    eprintln!("Token.ls: {} chars", token_src.len());

    // tok-eof hash
    let tok_eof_hash: i64 = {
        let s = "tok-eof";
        let mut acc: i64 = 0;
        for c in s.chars() {
            acc = acc.wrapping_mul(31).wrapping_add(c as i64);
        }
        acc
    };
    eprintln!("tok-eof hash = {tok_eof_hash}");

    // Manually check: compile Token.ls with selfhost (via stage1)
    let main_path = selfhost_main_path();
    let selfhost_dir = selfhost_package_root();
    let stage1_wasm = compile_file_only(&main_path);

    // compile Token.ls with stage1
    let output = lsharp_wasm::wasi_runner::run_wasm_wasi_with_dir_and_args(
        &stage1_wasm,
        Some(&selfhost_dir),
        &["compiler", "src/Syntax/Token.ls"],
    )
    .expect("stage1 failed to compile Token.ls");
    eprintln!("Token.ls compiled, output {} chars", output.len());
    let modules = parse_emitted_wasm_modules(&output, 1);
    let token_wasm = &modules[0];

    // Look for tok-eof function (returns 99)
    let found_99 = std::panic::catch_unwind(|| {
        let data = token_wasm.as_slice();
        fn read_leb(data: &[u8], pos: &mut usize) -> u64 {
            let mut v = 0u64;
            let mut shift = 0;
            loop {
                let b = data[*pos];
                *pos += 1;
                v |= ((b & 0x7f) as u64) << shift;
                if b & 0x80 == 0 {
                    break;
                }
                shift += 7;
            }
            v
        }
        let mut pos = 8usize;
        let mut func_99_idx = None;
        while pos < data.len() {
            let sid = data[pos];
            pos += 1;
            let size = read_leb(data, &mut pos) as usize;
            if sid == 10 {
                let count = read_leb(data, &mut pos) as usize;
                for fidx in 0..count {
                    let sz = read_leb(data, &mut pos) as usize;
                    let end = pos + sz;
                    let local_count = read_leb(data, &mut pos);
                    for _ in 0..local_count {
                        let _n = read_leb(data, &mut pos);
                        let _t = data[pos];
                        pos += 1;
                    }
                    // Check if it's a simple i64.const 99 return
                    if pos < end - 2 && data[pos] == 0x42 {
                        // i64.const
                        pos += 1;
                        let val = read_leb(data, &mut pos);
                        if val == 99 && pos < end && data[pos] == 0x0b {
                            func_99_idx = Some(fidx);
                            eprintln!(
                                "Found tok-eof (=99) at user func idx {fidx} (wasm idx {})",
                                fidx + 6
                            );
                        }
                    }
                    pos = end;
                }
                break;
            }
            pos += size;
        }
        func_99_idx
    });
    eprintln!("tok-eof in Token.ls compilation: {:?}", found_99);
}