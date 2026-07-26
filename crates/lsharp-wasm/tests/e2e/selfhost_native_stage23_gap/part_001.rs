#[test]
fn test_native_codegen_int_to_string_helpers_emit_tagged_decimal_string_abi() {
    let lines = run_x86_selfhost_runtime_helper_harness(
        "native-stage23-int-to-string-helper-abi",
        r#"  (let [x86-helper (emit-x86-selfhost-int-to-string-helper)
        aarch64-helper (emit-aarch64-selfhost-int-to-string-helper)]
    (do
      (print (vector-length x86-helper))
      (print-bytes-loop x86-helper 0 20)
      (print-bytes-loop x86-helper (- (vector-length x86-helper) 20) (vector-length x86-helper))
      (print (vector-length aarch64-helper))
      (print-bytes-loop aarch64-helper 0 20)
      (print-bytes-loop aarch64-helper (- (vector-length aarch64-helper) 20) (vector-length aarch64-helper))
      0))"#,
    );

    assert_eq!(
        lines,
        vec![
            169, 83, 72, 137, 251, 72, 49, 255, 72, 199, 198, 32, 0, 0, 0, 72, 199, 194, 3, 0, 0,
            8, 72, 137, 209, 243, 164, 76, 137, 208, 72, 15, 186, 232, 63, 91, 195, 49, 192, 91,
            195, 176, 243, 83, 190, 169, 254, 11, 0, 249, 230, 3, 0, 170, 0, 4, 128, 210, 75, 253,
            255, 151, 192, 3, 95, 214, 31, 32, 3, 213, 31, 32, 3, 213, 31, 32, 3, 213, 31, 32, 3,
            213,
        ],
        "native int-to-string helper は tagged String の確保、十進 byte copy、callee-saved register の復元を target ごとの ABI で維持する必要がある",
    );
}

#[test]
fn test_native_codegen_aarch64_write_file_helpers_use_binary_file_abi() {
    let lines = run_x86_selfhost_runtime_helper_harness(
        "native-stage23-aarch64-write-file-helper-abi",
        r#"  (let [import-stub-offset 4096
        import-count 10
        current-offset 1024
        write-file-helper (emit-aarch64-selfhost-write-file-helper)
        write-file-bytes-helper (emit-aarch64-selfhost-write-file-bytes-helper)
        write-file-call (codegen-ir-instr-bundle-aarch64-with-import-count 89 0 current-offset (vector-new 0) (vector-new 0) import-count import-stub-offset 0 2)
        write-file-bytes-call (codegen-ir-instr-bundle-aarch64-with-import-count 90 0 current-offset (vector-new 0) (vector-new 0) import-count import-stub-offset 0 2)]
    (do
      (print (opcode-stack-delta 89 0 (vector-new 0)))
      (print (opcode-stack-delta 90 0 (vector-new 0)))
      (print (aarch64-selfhost-proc-exit-helper-offset import-stub-offset import-count))
      (print (aarch64-selfhost-write-file-helper-offset import-stub-offset import-count))
      (print (aarch64-selfhost-write-file-bytes-helper-offset import-stub-offset import-count))
      (print (aarch64-selfhost-helper-trailer-size import-count))
      (print (native-instr-size-aarch64 89 0 (vector-new 0) 2))
      (print (native-instr-size-aarch64 90 0 (vector-new 0) 2))
      (print (vector-length write-file-helper))
      (print-bytes-loop write-file-helper 0 8)
      (print-bytes-loop write-file-helper (- (vector-length write-file-helper) 4) (vector-length write-file-helper))
      (print (vector-length write-file-bytes-helper))
      (print-bytes-loop write-file-bytes-helper 0 8)
      (print-bytes-loop write-file-bytes-helper (- (vector-length write-file-bytes-helper) 4) (vector-length write-file-bytes-helper))
      (print (vector-length write-file-call))
      (print-bytes-loop write-file-call 0 (vector-length write-file-call))
      (print (vector-length write-file-bytes-call))
      (print-bytes-loop write-file-bytes-call 0 (vector-length write-file-bytes-call))
      0))"#,
    );

    assert_eq!(
        lines,
        vec![
            -1, -1, 6708, 6724, 6948, 3336, 12, 12, 224, 243, 83, 191, 169, 245, 91, 191, 169, 192,
            3, 95, 214, 308, 243, 83, 191, 169, 245, 91, 191, 169, 192, 3, 95, 214, 12, 225, 3, 0,
            170, 224, 3, 9, 170, 143, 5, 0, 148, 12, 225, 3, 0, 170, 224, 3, 9, 170, 199, 5, 0,
            148,
        ],
        "AArch64 write-file / write-file-bytes は Darwin ABI、trailer offset、raw byte write helper を一致させる必要がある",
    );
}

#[test]
fn test_native_codegen_aarch64_write_file_bytes_helper_preserves_heap_base() {
    let lines = run_x86_selfhost_runtime_helper_harness(
        "native-stage23-aarch64-write-file-bytes-heap-base",
        r#"  (let [helper (emit-aarch64-selfhost-write-file-bytes-helper)]
    (do
      (print (vector-length helper))
      (print-bytes-loop helper 20 32)
      (print-bytes-loop helper 116 124)
      (print-bytes-loop helper 140 144)
      (print-bytes-loop helper 168 172)
      (print-bytes-loop helper 196 200)
      (print-bytes-loop helper 232 236)
      (print-bytes-loop helper 264 268)
      0))"#,
    );

    assert_eq!(
        lines,
        vec![
            308, 23, 0, 128, 210, 32, 2, 248, 182, 160, 2, 0, 139, 244, 4, 248, 182, 180, 2, 20,
            139, 31, 32, 3, 213, 224, 3, 0, 145, 225, 3, 0, 145, 247, 2, 0, 139, 224, 3, 23, 170,
        ],
        "AArch64 write-file-bytes helper は tagged heap offset を heap base へ戻し、base register を書込件数で破壊してはならない",
    );
}

#[test]
fn test_native_codegen_aarch64_write_file_helper_preserves_heap_base() {
    let lines = run_x86_selfhost_runtime_helper_harness(
        "native-stage23-aarch64-write-file-heap-base",
        r#"  (let [helper (emit-aarch64-selfhost-write-file-helper)]
    (do
      (print (vector-length helper))
      (print-bytes-loop helper 16 28)
      (print-bytes-loop helper 112 124)
      (print-bytes-loop helper 164 168)
      (print-bytes-loop helper 184 188)
      0))"#,
    );

    assert_eq!(
        lines,
        vec![
            224, 17, 0, 128, 210, 32, 2, 248, 182, 160, 2, 0, 139, 148, 2, 248, 182, 161, 2, 20,
            139, 33, 248, 64, 146, 49, 2, 0, 139, 224, 3, 17, 170,
        ],
        "AArch64 write-file helper は path/content の tagged heap offset を heap base へ戻し、書込件数で base register を破壊してはならない",
    );
}

#[test]
fn test_native_codegen_x86_vector_and_ref_helper_call_sites_resolve_offsets() {
    let lines = run_x86_selfhost_runtime_helper_harness(
        "native-stage23-x86-vector-ref-helper-call-sites",
        r#"  (let [import-stub-offset 4096
        import-count 10
        current-offset 2048
        vector-new-bytes (codegen-ir-instr-bundle-x86-with-import-count 54 0 current-offset (vector-new 0) (vector-new 0) import-count import-stub-offset 0 1)
        vector-length-bytes (codegen-ir-instr-bundle-x86-with-import-count 52 0 current-offset (vector-new 0) (vector-new 0) import-count import-stub-offset 0 1)
        vector-get-bytes (codegen-ir-instr-bundle-x86-with-import-count 53 0 current-offset (vector-new 0) (vector-new 0) import-count import-stub-offset 0 2)
        vector-push-bytes (codegen-ir-instr-bundle-x86-with-import-count 55 0 current-offset (vector-new 0) (vector-new 0) import-count import-stub-offset 0 2)
        ref-new-bytes (codegen-ir-instr-bundle-x86-with-import-count 56 0 current-offset (vector-new 0) (vector-new 0) import-count import-stub-offset 0 1)
        ref-get-bytes (codegen-ir-instr-bundle-x86-with-import-count 57 0 current-offset (vector-new 0) (vector-new 0) import-count import-stub-offset 0 1)
        ref-set-bytes (codegen-ir-instr-bundle-x86-with-import-count 58 0 current-offset (vector-new 0) (vector-new 0) import-count import-stub-offset 0 2)]
    (do
      (print (x86-selfhost-vector-new-helper-offset import-stub-offset import-count))
      (print (x86-selfhost-vector-length-helper-offset import-stub-offset import-count))
      (print (x86-selfhost-vector-get-helper-offset import-stub-offset import-count))
      (print (x86-selfhost-vector-push-helper-offset import-stub-offset import-count))
      (print (x86-selfhost-ref-new-helper-offset import-stub-offset import-count))
      (print (x86-selfhost-ref-get-helper-offset import-stub-offset import-count))
      (print (x86-selfhost-ref-set-helper-offset import-stub-offset import-count))
      (print (vector-length vector-new-bytes))
      (print-bytes-loop vector-new-bytes 0 (vector-length vector-new-bytes))
      (print (vector-length vector-length-bytes))
      (print-bytes-loop vector-length-bytes 0 (vector-length vector-length-bytes))
      (print (vector-length vector-get-bytes))
      (print-bytes-loop vector-get-bytes 0 (vector-length vector-get-bytes))
      (print (vector-length vector-push-bytes))
      (print-bytes-loop vector-push-bytes 0 (vector-length vector-push-bytes))
      (print (vector-length ref-new-bytes))
      (print-bytes-loop ref-new-bytes 0 (vector-length ref-new-bytes))
      (print (vector-length ref-get-bytes))
      (print-bytes-loop ref-get-bytes 0 (vector-length ref-get-bytes))
      (print (vector-length ref-set-bytes))
      (print-bytes-loop ref-set-bytes 0 (vector-length ref-set-bytes))
      0))"#,
    );

    assert_eq!(
        lines,
        vec![
            4549, 4666, 4683, 4730, 4935, 5008, 5026, 7, 81, 232, 191, 9, 0, 0, 89, 7, 81, 232, 52,
            10, 0, 0, 89, 5, 232, 70, 10, 0, 0, 5, 232, 117, 10, 0, 0, 7, 81, 232, 65, 11, 0, 0,
            89, 7, 81, 232, 138, 11, 0, 0, 89, 5, 232, 157, 11, 0, 0,
        ],
        "x86_64 vector/ref helper call sites は trailer offset を指す call を出す必要がある"
    );
}

#[test]
fn test_native_codegen_x86_local_get_preserves_depth_one_stack_value() {
    let lines = run_x86_selfhost_runtime_helper_harness(
        "native-stage23-x86-local-get-depth-one-preserve",
        r#"  (let [frame-base-slot-count 2
        current-depth 1
        local-bytes (codegen-ir-instr-bundle-x86-with-import-count 10 0 2048 (vector-new 0) (vector-new 0) 10 4096 frame-base-slot-count current-depth)
        local-size (native-instr-size-x86 10 0 (vector-new 0) current-depth)]
    (do
      (print local-size)
      (print (vector-length local-bytes))
      (print-bytes-loop local-bytes 0 (vector-length local-bytes))
      0))"#,
    );

    assert_eq!(
        &lines[..5],
        &[10, 10, 72, 137, 193],
        "x86_64 LocalGet at stack depth 1 must preserve the previous top value in rcx before loading the local"
    );
}

#[test]
fn test_native_codegen_x86_string_slice_concat_helper_call_sites_resolve_offsets() {
    let lines = run_x86_selfhost_runtime_helper_harness(
        "native-stage23-x86-string-slice-concat-helper-call-sites",
        r#"  (let [import-stub-offset 4096
        import-count 10
        current-offset 3072
        substring-bytes (codegen-ir-instr-bundle-x86-with-import-count 69 0 current-offset (vector-new 0) (vector-new 0) import-count import-stub-offset 0 3)
        concat-bytes (codegen-ir-instr-bundle-x86-with-import-count 70 0 current-offset (vector-new 0) (vector-new 0) import-count import-stub-offset 0 2)]
    (do
      (print (x86-selfhost-substring-helper-offset import-stub-offset import-count))
      (print (x86-selfhost-string-concat-helper-offset import-stub-offset import-count))
      (print (vector-length substring-bytes))
      (print-bytes-loop substring-bytes 0 (vector-length substring-bytes))
      (print (vector-length concat-bytes))
      (print-bytes-loop concat-bytes 0 (vector-length concat-bytes))
      0))"#,
    );

    assert_eq!(
        lines,
        vec![
            5046, 5191, 12, 72, 139, 149, 248, 255, 255, 255, 232, 170, 7, 0, 0, 5, 232, 66, 8, 0,
            0,
        ],
        "x86_64 substring/string-concat helper call sites は trailer offset を指す call を出す必要がある"
    );
}

#[test]
fn test_native_codegen_x86_map_and_file_helper_call_sites_resolve_offsets() {
    let lines = run_x86_selfhost_runtime_helper_harness(
        "native-stage23-x86-map-file-helper-call-sites",
        r#"  (let [import-stub-offset 4096
        import-count 10
        current-offset 4096
        map-new-bytes (codegen-ir-instr-bundle-x86-with-import-count 60 0 current-offset (vector-new 0) (vector-new 0) import-count import-stub-offset 0 0)
        map-size-bytes (codegen-ir-instr-bundle-x86-with-import-count 61 0 current-offset (vector-new 0) (vector-new 0) import-count import-stub-offset 0 1)
        map-insert-bytes (codegen-ir-instr-bundle-x86-with-import-count 62 0 current-offset (vector-new 0) (vector-new 0) import-count import-stub-offset 0 3)
        map-get-bytes (codegen-ir-instr-bundle-x86-with-import-count 63 0 current-offset (vector-new 0) (vector-new 0) import-count import-stub-offset 0 2)
        file-exists-bytes (codegen-ir-instr-bundle-x86-with-import-count 73 0 current-offset (vector-new 0) (vector-new 0) import-count import-stub-offset 0 1)]
    (do
      (print (x86-selfhost-map-new-helper-offset import-stub-offset import-count))
      (print (x86-selfhost-map-size-helper-offset import-stub-offset import-count))
      (print (x86-selfhost-map-insert-helper-offset import-stub-offset import-count))
      (print (x86-selfhost-map-get-helper-offset import-stub-offset import-count))
      (print (x86-selfhost-file-exists-helper-offset import-stub-offset import-count))
      (print (vector-length map-new-bytes))
      (print-bytes-loop map-new-bytes 0 (vector-length map-new-bytes))
      (print (vector-length map-size-bytes))
      (print-bytes-loop map-size-bytes 0 (vector-length map-size-bytes))
      (print (vector-length map-insert-bytes))
      (print-bytes-loop map-insert-bytes 0 (vector-length map-insert-bytes))
      (print (vector-length map-get-bytes))
      (print-bytes-loop map-get-bytes 0 (vector-length map-get-bytes))
      (print (vector-length file-exists-bytes))
      (print-bytes-loop file-exists-bytes 0 (vector-length file-exists-bytes))
      0))"#,
    );

    assert_eq!(
        lines,
        vec![
            5386, 5458, 5475, 5536, 5598, 5, 232, 5, 5, 0, 0, 7, 81, 232, 76, 5, 0, 0, 89, 12, 72,
            139, 149, 248, 255, 255, 255, 232, 87, 5, 0, 0, 5, 232, 155, 5, 0, 0, 7, 81, 232, 216,
            5, 0, 0, 89,
        ],
        "x86_64 map/file helper call sites は trailer offset を指す call を出す必要がある"
    );
}

#[test]
fn test_native_codegen_x86_runtime_helper_emitters_return_executable_byte_vectors() {
    let lines = run_x86_selfhost_runtime_helper_harness(
        "native-stage23-x86-runtime-helper-bytes",
        r#"  (let [command-helper (emit-x86-selfhost-command-line-arg-helper)
        read-helper (emit-x86-selfhost-read-file-helper)
        strlen-helper (emit-x86-selfhost-string-length-helper)
        char-at-helper (emit-x86-selfhost-string-char-at-helper)
        print-helper (emit-x86-selfhost-print-helper)]
    (do
      (print (vector-length command-helper))
      (print-bytes-loop command-helper 0 (vector-length command-helper))
      (print (vector-length read-helper))
      (print-bytes-loop read-helper 0 8)
      (print-bytes-loop read-helper (- (vector-length read-helper) 8) (vector-length read-helper))
      (print (vector-length strlen-helper))
      (print-bytes-loop strlen-helper 0 (vector-length strlen-helper))
      (print (vector-length char-at-helper))
      (print-bytes-loop char-at-helper 0 (vector-length char-at-helper))
      (print (vector-length print-helper))
      (print-bytes-loop print-helper 0 8)
      (print-bytes-loop print-helper (- (vector-length print-helper) 8) (vector-length print-helper))
      0))"#,
    );

    assert_eq!(
        lines,
        vec![
            18, 72, 133, 192, 124, 10, 76, 57, 224, 125, 5, 73, 139, 4, 199, 195, 49, 192, 195,
            207, 83, 65, 84, 65, 85, 69, 49, 237, 49, 192, 65, 93, 65, 92, 91, 195, 52, 72, 133,
            192, 116, 44, 121, 9, 72, 15, 186, 240, 63, 139, 64, 4, 195, 72, 61, 0, 0, 0, 64, 115,
            7, 76, 1, 240, 139, 64, 4, 195, 72, 49, 201, 128, 60, 8, 0, 116, 5, 72, 255, 193, 235,
            245, 72, 137, 200, 195, 49, 192, 195, 71, 72, 133, 192, 120, 63, 72, 133, 201, 116, 58,
            72, 133, 201, 121, 16, 72, 15, 186, 241, 63, 59, 65, 4, 115, 43, 15, 182, 68, 1, 8,
            195, 72, 129, 249, 0, 0, 0, 64, 115, 14, 76, 1, 241, 59, 65, 4, 115, 20, 15, 182, 68,
            1, 8, 195, 72, 129, 249, 0, 16, 0, 0, 114, 5, 15, 182, 4, 1, 195, 49, 192, 195, 102,
            83, 72, 131, 236, 32, 72, 137, 195, 49, 192, 72, 131, 196, 32, 91, 195,
        ],
        "x86_64 runtime helper emitters は実行可能な prologue/epilogue を持つ byte vector を返す必要がある"
    );
    assert!(
        supported_selfhost_native_opcodes_x86_64().contains("ReadFile"),
        "selfhost x86_64 gap supported set から ReadFile を外したまま"
    );
    assert!(
        supported_selfhost_native_opcodes_x86_64().contains("StringLength"),
        "selfhost x86_64 gap supported set から StringLength を外したまま"
    );
    assert!(
        supported_selfhost_native_opcodes_x86_64().contains("StringCharAt"),
        "selfhost x86_64 gap supported set から StringCharAt を外したまま"
    );
    assert!(
        supported_selfhost_native_opcodes_x86_64().contains("Print"),
        "selfhost x86_64 gap supported set から Print を外したまま"
    );
}

#[test]
fn test_native_codegen_x86_cli_runtime_helper_emitters_return_linux_syscall_bytes() {
    let lines = run_x86_selfhost_runtime_helper_harness(
        "native-stage23-x86-cli-runtime-helper-bytes",
        r#"  (let [argc-helper (emit-x86-selfhost-command-line-args-helper)
        print-string-helper (emit-x86-selfhost-print-string-helper)
        proc-exit-helper (emit-x86-selfhost-proc-exit-helper)]
    (do
      (print (vector-length argc-helper))
      (print-bytes-loop argc-helper 0 (vector-length argc-helper))
      (print (vector-length print-string-helper))
      (print-bytes-loop print-string-helper 0 (vector-length print-string-helper))
      (print (vector-length proc-exit-helper))
      (print-bytes-loop proc-exit-helper 0 (vector-length proc-exit-helper))
      0))"#,
    );

    assert_eq!(
        lines,
        vec![
            4, 76, 137, 224, 195, 51, 72, 137, 198, 72, 133, 246, 116, 40, 121, 7, 72, 15, 186,
            246, 63, 235, 12, 72, 129, 254, 0, 0, 0, 64, 115, 3, 76, 1, 246, 139, 86, 4, 72, 131,
            198, 8, 191, 1, 0, 0, 0, 184, 1, 0, 0, 0, 15, 5, 49, 192, 195, 12, 72, 137, 199, 184,
            60, 0, 0, 0, 15, 5, 15, 11,
        ],
        "x86_64 CLI runtime helper は Linux write/exit syscall と保持済み argc register の実バイトを返す必要がある"
    );
}

#[test]
fn test_native_codegen_aarch64_cli_runtime_helpers_preserve_offsets_and_branch_targets() {
    let lines = run_x86_selfhost_runtime_helper_harness(
        "native-stage23-aarch64-cli-runtime-helpers",
        r#"  (let [import-stub-offset 4096
        import-count 10
        current-offset 1024
        argc-helper (emit-aarch64-selfhost-command-line-args-helper)
        print-string-helper (emit-aarch64-selfhost-print-string-helper)
        proc-exit-helper (emit-aarch64-selfhost-proc-exit-helper)
        argc-depth0 (codegen-ir-instr-bundle-aarch64-with-import-count 86 0 current-offset (vector-new 0) (vector-new 0) import-count import-stub-offset 0 0)
        argc-depth2 (codegen-ir-instr-bundle-aarch64-with-import-count 86 0 current-offset (vector-new 0) (vector-new 0) import-count import-stub-offset 0 2)]
    (do
      (print (aarch64-selfhost-command-line-args-helper-offset import-stub-offset import-count))
      (print (aarch64-selfhost-print-string-helper-offset import-stub-offset import-count))
      (print (aarch64-selfhost-proc-exit-helper-offset import-stub-offset import-count))
      (print (aarch64-selfhost-helper-trailer-size import-count))
      (print (native-instr-size-aarch64 86 0 (vector-new 0) 0))
      (print (native-instr-size-aarch64 86 0 (vector-new 0) 1))
      (print (native-instr-size-aarch64 86 0 (vector-new 0) 2))
      (print (native-instr-size-aarch64 86 0 (vector-new 0) 5))
      (print (native-instr-size-aarch64 87 0 (vector-new 0) 1))
      (print (native-instr-size-aarch64 88 0 (vector-new 0) 1))
      (print (vector-length argc-helper))
      (print-bytes-loop argc-helper 0 (vector-length argc-helper))
      (print (vector-length print-string-helper))
      (print-bytes-loop print-string-helper 0 (vector-length print-string-helper))
      (print (vector-length proc-exit-helper))
      (print-bytes-loop proc-exit-helper 0 (vector-length proc-exit-helper))
      (print (vector-length argc-depth0))
      (print-bytes-loop argc-depth0 0 (vector-length argc-depth0))
      (print (vector-length argc-depth2))
      (print-bytes-loop argc-depth2 0 (vector-length argc-depth2))
      0))"#,
    );
    let mut cursor = 0;
    let scalars = &lines[cursor..cursor + 10];
    cursor += 10;
    assert_eq!(
        scalars,
        &[6616, 6624, 6708, 3160, 12, 16, 20, 44, 12, 12],
        "AArch64 CLI runtime helper の offset/trailer/stack-depth size が不正"
    );

    let take_bytes = |values: &[i64], cursor: &mut usize| {
        let len = values[*cursor] as usize;
        *cursor += 1;
        let bytes = values[*cursor..*cursor + len]
            .iter()
            .map(|value| *value as u8)
            .collect::<Vec<_>>();
        *cursor += len;
        bytes
    };
    let argc_helper = take_bytes(&lines, &mut cursor);
    let print_string_helper = take_bytes(&lines, &mut cursor);
    let proc_exit_helper = take_bytes(&lines, &mut cursor);
    let argc_depth0 = take_bytes(&lines, &mut cursor);
    let argc_depth2 = take_bytes(&lines, &mut cursor);
    assert_eq!(
        cursor,
        lines.len(),
        "AArch64 helper harness に未消費出力がある"
    );

    assert_eq!(argc_helper, [224, 3, 19, 170, 192, 3, 95, 214]);
    assert_eq!(
        print_string_helper,
        [
            235, 3, 0, 170, 75, 2, 0, 180, 43, 1, 248, 183, 127, 1, 22, 235, 3, 1, 0, 84, 225, 3,
            11, 170, 2, 0, 128, 210, 44, 104, 98, 56, 236, 0, 0, 52, 66, 4, 0, 145, 253, 255, 255,
            23, 107, 249, 64, 146, 171, 2, 11, 139, 98, 5, 64, 185, 97, 33, 0, 145, 130, 0, 0, 180,
            32, 0, 128, 210, 144, 0, 128, 210, 1, 16, 0, 212, 224, 3, 31, 170, 192, 3, 95, 214,
        ],
        "AArch64 print-string helper は Darwin write syscall の検証済み bytes と一致する必要がある"
    );
    assert_eq!(
        proc_exit_helper,
        [
            48, 0, 128, 210, 1, 16, 0, 212, 224, 3, 31, 170, 192, 3, 95, 214
        ]
    );

    let decode_bl_target = |bundle: &[u8], bundle_offset: usize, absolute_offset: i64| {
        let word = u32::from_le_bytes(
            bundle[bundle_offset..bundle_offset + 4]
                .try_into()
                .expect("BL instruction は4 bytes"),
        );
        assert_eq!(word >> 26, 0b100101, "対象 instruction は BL であるべき");
        let imm26 = (word & 0x03ff_ffff) as i32;
        let signed = (imm26 << 6) >> 6;
        absolute_offset + bundle_offset as i64 + i64::from(signed) * 4
    };
    assert_eq!(decode_bl_target(&argc_depth0, 4, 1024), 6616);
    assert_eq!(decode_bl_target(&argc_depth2, 12, 1024), 6616);
}

#[test]
fn test_native_codegen_x86_string_slice_concat_helper_emitters_return_executable_byte_vectors() {
    let lines = run_x86_selfhost_runtime_helper_harness(
        "native-stage23-x86-string-slice-concat-helper-bytes",
        r#"  (let [substring-helper (emit-x86-selfhost-substring-helper)
        concat-helper (emit-x86-selfhost-string-concat-helper)]
    (do
      (print (vector-length substring-helper))
      (print-bytes-loop substring-helper 0 8)
      (print-bytes-loop substring-helper (- (vector-length substring-helper) 8) (vector-length substring-helper))
      (print (vector-length concat-helper))
      (print-bytes-loop concat-helper 0 8)
      (print-bytes-loop concat-helper (- (vector-length concat-helper) 8) (vector-length concat-helper))
      0))"#,
    );

    assert_eq!(
        lines,
        vec![
            145, 83, 65, 84, 65, 85, 65, 86, 65, 65, 94, 65, 93, 65, 92, 91, 195, 195, 72, 133,
            201, 120, 18, 72, 129, 249, 93, 65, 92, 91, 195, 49, 192, 195,
        ],
        "x86_64 substring/string-concat helper emitters は実行可能な prologue/epilogue を持つ byte vector を返す必要がある"
    );
}

#[test]
fn test_native_codegen_x86_map_and_file_helper_emitters_return_executable_byte_vectors() {
    let lines = run_x86_selfhost_runtime_helper_harness(
        "native-stage23-x86-map-file-helper-bytes",
        r#"  (let [map-new-helper (emit-x86-selfhost-map-new-helper)
        map-size-helper (emit-x86-selfhost-map-size-helper)
        map-insert-helper (emit-x86-selfhost-map-insert-helper)
        map-get-helper (emit-x86-selfhost-map-get-helper)
        file-exists-helper (emit-x86-selfhost-file-exists-helper)]
    (do
      (print (vector-length map-new-helper))
      (print-bytes-loop map-new-helper 0 8)
      (print-bytes-loop map-new-helper (- (vector-length map-new-helper) 8) (vector-length map-new-helper))
      (print (vector-length map-size-helper))
      (print-bytes-loop map-size-helper 0 (vector-length map-size-helper))
      (print (vector-length map-insert-helper))
      (print-bytes-loop map-insert-helper 0 8)
      (print-bytes-loop map-insert-helper (- (vector-length map-insert-helper) 8) (vector-length map-insert-helper))
      (print (vector-length map-get-helper))
      (print-bytes-loop map-get-helper 0 8)
      (print-bytes-loop map-get-helper (- (vector-length map-get-helper) 8) (vector-length map-get-helper))
      (print (vector-length file-exists-helper))
      (print-bytes-loop file-exists-helper 0 8)
      (print-bytes-loop file-exists-helper (- (vector-length file-exists-helper) 8) (vector-length file-exists-helper))
      0))"#,
    );

    assert_eq!(
        lines,
        vec![
            72, 81, 73, 139, 6, 72, 137, 199, 185, 49, 192, 89, 195, 144, 144, 144, 144, 17, 72, 133,
            192, 121, 9, 72, 15, 186, 240, 63, 139, 64, 8, 195, 49, 192, 195, 104, 83, 72, 137, 211,
            72, 133, 219, 121, 232, 63, 91, 195, 49, 192, 91, 195, 62, 83, 65, 84, 72, 133, 201,
            121, 48, 91, 195, 49, 192, 65, 92, 91, 195, 84, 83, 65, 84, 72, 137, 227, 72, 133, 192,
            72, 137, 220, 65, 92, 91, 195,
        ],
        "x86_64 map/file helper emitters は実行可能な prologue/epilogue を持つ byte vector を返す必要がある"
    );
}

#[test]
fn test_native_codegen_x86_read_file_helper_uses_linux_syscalls() {
    let lines = run_x86_selfhost_runtime_helper_harness(
        "native-stage23-x86-read-file-linux-syscalls",
        r#"  (let [read-helper (emit-x86-selfhost-read-file-helper)]
    (do
      (print (vector-get read-helper 56))
      (print (vector-get read-helper 57))
      (print (vector-get read-helper 58))
      (print (vector-get read-helper 59))
      (print (vector-get read-helper 60))
      (print (vector-get read-helper 98))
      (print (vector-get read-helper 99))
      (print (vector-get read-helper 100))
      (print (vector-get read-helper 101))
      (print (vector-get read-helper 111))
      (print (vector-get read-helper 112))
      (print (vector-get read-helper 113))
      (print (vector-get read-helper 114))
      (print (vector-get read-helper 115))
      (print (vector-get read-helper 140))
      (print (vector-get read-helper 141))
      (print (vector-get read-helper 142))
      (print (vector-get read-helper 143))
      (print (vector-get read-helper 144))
      (print (vector-get read-helper 169))
      (print (vector-get read-helper 170))
      (print (vector-get read-helper 171))
      (print (vector-get read-helper 172))
      (print (vector-get read-helper 173))
      (print (vector-get read-helper 192))
      (print (vector-get read-helper 193))
      (print (vector-get read-helper 194))
      (print (vector-get read-helper 195))
      (print (vector-get read-helper 196))
      0))"#,
    );

    assert_eq!(
        lines,
        vec![
            184, 2, 0, 0, 0, // open
            34, 0, 0, 0, // MAP_PRIVATE | MAP_ANONYMOUS
            184, 9, 0, 0, 0, // mmap
            184, 0, 0, 0, 0, // read
            184, 3, 0, 0, 0, // close
            184, 3, 0, 0, 0, // close on failure
        ],
        "x86_64 Linux read-file helper は Linux syscall 番号と mmap flags を使う必要がある"
    );
}

#[test]
fn test_native_codegen_x86_string_char_at_oob_jumps_to_return_zero() {
    let lines = run_x86_selfhost_runtime_helper_harness(
        "native-stage23-x86-string-char-at-oob-jump",
        r#"  (let [helper (emit-x86-selfhost-string-char-at-helper)]
    (do
      (print (vector-get helper 23))
      (print (vector-get helper 24))
      (let [target (+ 25 (vector-get helper 24))]
        (do
          (print (vector-get helper target))
          (print (vector-get helper (+ target 1)))
          (print (vector-get helper (+ target 2)))))
      0))"#,
    );

    assert_eq!(
        lines,
        vec![115, 43, 49, 192, 195],
        "x86_64 string-char-at の tagged out-of-bounds 分岐は return-zero へ飛ぶ必要がある"
    );
}

#[test]
fn test_native_codegen_x86_vector_push_helper_has_growth_path() {
    let lines = run_x86_selfhost_runtime_helper_harness(
        "native-stage23-x86-vector-push-helper-growth-path",
        r#"  (let [helper (emit-x86-selfhost-vector-push-helper)]
    (do
      (print (vector-length helper))
      (print (vector-get helper 0))
      (print (vector-get helper 1))
      (print (vector-get helper 2))
      (print (vector-get helper 3))
      (print (vector-get helper 89))
      (print (vector-get helper 90))
      (print (vector-get helper 91))
      (print (vector-get helper 92))
      (print (vector-get helper 93))
      (print (vector-get helper 94))
      (print (vector-get helper 95))
      0))"#,
    );

    assert_eq!(
        lines,
        vec![205, 65, 84, 65, 85, 72, 15, 71, 194, 72, 133, 192],
        "x86_64 vector-push helper は capacity 超過時に bounded native heap で grow できる必要がある"
    );
}

#[test]
fn test_native_codegen_x86_vector_push_helper_reads_capacity_from_rcx_header() {
    let lines = run_x86_selfhost_runtime_helper_harness(
        "native-stage23-x86-vector-push-helper-capacity-header",
        r#"  (let [helper (emit-x86-selfhost-vector-push-helper)]
    (do
      (print-bytes-loop helper 21 25)
      0))"#,
    );

    assert_eq!(
        lines,
        vec![68, 139, 97, 4],
        "x86_64 vector-push helper は caller frame ではなく untagged vector の [rcx+4] から capacity を読む必要がある"
    );
}

#[test]
fn test_native_codegen_x86_vector_push_helper_compares_length_to_capacity_register() {
    let lines = run_x86_selfhost_runtime_helper_harness(
        "native-stage23-x86-vector-push-helper-capacity-compare",
        r#"  (let [helper (emit-x86-selfhost-vector-push-helper)]
    (do
      (print-bytes-loop helper 21 28)
      0))"#,
    );

    assert_eq!(
        lines,
        vec![68, 139, 97, 4, 68, 57, 226],
        "x86_64 vector-push helper は length と capacity を cmp edx, r12d で比較する必要がある"
    );
}

#[test]
fn test_native_codegen_x86_vector_push_helper_saves_capacity_before_length_for_growth() {
    let lines = run_x86_selfhost_runtime_helper_harness(
        "native-stage23-x86-vector-push-helper-growth-register-order",
        r#"  (let [helper (emit-x86-selfhost-vector-push-helper)]
    (do
      (print-bytes-loop helper 36 45)
      0))"#,
    );

    assert_eq!(
        lines,
        vec![69, 137, 229, 65, 137, 212, 69, 133, 237],
        "x86_64 vector-push helper は grow 時に capacity を r13d へ退避してから length を r12d へ保存する必要がある"
    );
}

#[test]
fn test_native_codegen_x86_i64_sub_depth_three_restores_previous_window() {
    let lines = run_x86_selfhost_runtime_helper_harness(
        "native-stage23-x86-i64-sub-depth-three",
        r#"  (let [bytes (codegen-ir-instr-bundle-x86-with-import-count 21 0 1024 (vector-new 0) (vector-new 0) 0 0 0 3)]
    (do
      (print (vector-length bytes))
      (print (vector-get bytes 0))
      (print (vector-get bytes 1))
      (print (vector-get bytes 2))
      (print (vector-get bytes 3))
      (print (vector-get bytes 4))
      (print (vector-get bytes 5))
      (print (vector-get bytes 6))
      (print (vector-get bytes 7))
      0))"#,
    );

    assert_eq!(
        lines,
        vec![13, 72, 41, 193, 72, 137, 200, 72, 139],
        "x86_64 i64.sub bundle は depth>=3 で演算後に下段 stack window を rcx へ復元する必要がある"
    );
}
