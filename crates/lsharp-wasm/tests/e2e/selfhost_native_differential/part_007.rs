
/// NATIVE-REAL-08n: x86_64 で 8 引数 direct call bundle が 2 stack arg を持つこと
#[test]
#[ignore]
fn test_native_codegen_emits_x86_direct_call_eight_arg_bundle_bytes() {
    let output = run_native_codegen_harness(
        r#"(module Main)
(import NativeTarget)
(import NativeCodegen)
(import IR.IR)

(defn make-function-meta [param-count local-count ir]
  (vector-push
    (vector-push
      (vector-push (vector-new 3) param-count)
      local-count)
    ir))

(defn main []
  (let [caller-ir (vector-push
                    (vector-push
                      (vector-push
                        (vector-push
                          (vector-push
                            (vector-push
                              (vector-push
                                (vector-push
                                  (vector-push (vector-new 9) (make-instr 3 40))
                                  (make-instr 3 2))
                                (make-instr 3 5))
                              (make-instr 3 7))
                            (make-instr 3 11))
                          (make-instr 3 14))
                        (make-instr 3 17))
                      (make-instr 3 19))
                    (make-call 1))
        callee-ir-base (vector-push
                         (vector-push
                           (vector-push
                             (vector-push
                               (vector-push
                                 (vector-push
                                   (vector-push
                                     (vector-push
                                       (vector-push
                                         (vector-push
                                           (vector-push (vector-new 15) (make-local-get 0))
                                           (make-local-get 1))
                                         (make-instr 24 0))
                                       (make-local-get 2))
                                     (make-instr 24 0))
                                   (make-local-get 3))
                                 (make-instr 24 0))
                               (make-local-get 4))
                             (make-instr 24 0))
                           (make-local-get 5))
                         (make-instr 24 0))
        callee-ir-mid (vector-push
                        (vector-push callee-ir-base (make-local-get 6))
                        (make-instr 24 0))
        callee-ir (vector-push
                    (vector-push callee-ir-mid (make-local-get 7))
                    (make-instr 24 0))
        caller (make-function-meta 0 0 caller-ir)
        callee (make-function-meta 8 0 callee-ir)
        functions (vector-push (vector-push (vector-new 2) caller) callee)
        target (make-target 1)
        native (emit-native-function-meta-bundle functions target)]
    (do
      (print (vector-length native))
      (print (vector-get native 327))
      (print (vector-get native 328))
      (print (vector-get native 329))
      (print (vector-get native 330))
      (print (vector-get native 331))
      (print (vector-get native 332))
      (print (vector-get native 333))
      (print (vector-get native 334))
      (print (vector-get native 335))
      (print (vector-get native 336))
      (print (vector-get native 337))
      (print (vector-get native 338))
      (print (vector-get native 339))
      (print (vector-get native 340))
      (print (vector-get native 341))
      (print (vector-get native 342))
      (print (vector-get native 343))
      (print (vector-get native 344))
      (print (vector-get native 345))
      (print (vector-get native 346))
      (print (vector-get native 347))
      (print (vector-get native 348))
      (print (vector-get native 349))
      (print (vector-get native 350))
      (print (vector-get native 351))
      (print (vector-get native 352))
      (print (vector-get native 353))
      (print (vector-get native 354))
      (print (vector-get native 355))
      (print (vector-get native 356))
      (print (vector-get native 357))
      (print (vector-get native 358))
      (print (vector-get native 359))
      (print (vector-get native 360))
      (print (vector-get native 361))
      (print (vector-get native 362))
      (print (vector-get native 363))
      (print (vector-get native 364))
      (print (vector-get native 365))
      (print (vector-get native 366))
      (print (vector-get native 367))
      (print (vector-get native 368))
      (print (vector-get native 369))
      (print (vector-get native 370))
      (print (vector-get native 371))
      (print (vector-get native 372))
      (print (vector-get native 373))
      (print (vector-get native 374))
      (print (vector-get native 375))
      (print (vector-get native 376))
      (print (vector-get native 377))
      (print (vector-get native 378))
      (print (vector-get native 379))
      (print (vector-get native 380))
      (print (vector-get native 381))
      (print (vector-get native 382))
      (print (vector-get native 383))
      (print (vector-get native 384))
      (print (vector-get native 385))
      (print (vector-get native 386))
      (print (vector-get native 387))
      (print (vector-get native 388))
      (print (vector-get native 389))
      (print (vector-get native 390))
      (print (vector-get native 391))
      (print (vector-get native 392))
      (print (vector-get native 393))
      (print (vector-get native 394))
      (print (vector-get native 395))
      (print (vector-get native 396))
      (print (vector-get native 417))
      (print (vector-get native 418))
      (print (vector-get native 419))
      (print (vector-get native 424))
      (print (vector-get native 425))
      (print (vector-get native 426))
      (print (vector-get native 431))
      (print (vector-get native 432))
      (print (vector-get native 433))
      (print (vector-get native 438))
      (print (vector-get native 439))
      (print (vector-get native 440))
      (print (vector-get native 445))
      (print (vector-get native 446))
      (print (vector-get native 447))
      (print (vector-get native 452))
      (print (vector-get native 453))
      (print (vector-get native 454))
      (print (vector-get native 459))
      (print (vector-get native 460))
      (print (vector-get native 461))
      (print (vector-get native 462))
      (print (vector-get native 463))
      (print (vector-get native 464))
      (print (vector-get native 465))
      (print (vector-get native 470))
      (print (vector-get native 471))
      (print (vector-get native 472))
      (print (vector-get native 473))
      (print (vector-get native 474))
      (print (vector-get native 475))
      (print (vector-get native 476))
      (print (vector-get native 582))
      (print (vector-get native 583))
       0)))"#,
    );
    let lines: Vec<&str> = output.trim().lines().collect();

    assert!(
        lines.len() >= 105,
        "x86 direct call eight-arg bundle bytes 出力が不足: {:?}",
        lines
    );
    assert_eq!(
        lines[0], "584",
        "x86_64 direct call eight-arg bundle payload は 584 bytes であるべき"
    );
    assert_eq!(
        lines[1], "72",
        "stack arg reserve 先頭は sub rsp, 16 の 0x48"
    );
    assert_eq!(lines[2], "129", "stack arg reserve 2 byte 目は 0x81");
    assert_eq!(lines[3], "236", "stack arg reserve 3 byte 目は ModRM 0xEC");
    assert_eq!(lines[4], "16", "stack arg reserve imm byte0 は 16");
    assert_eq!(lines[5], "0", "stack arg reserve imm byte1 は 0");
    assert_eq!(lines[6], "0", "stack arg reserve imm byte2 は 0");
    assert_eq!(lines[7], "0", "stack arg reserve imm byte3 は 0");
    assert_eq!(
        lines[8], "72",
        "stack arg7 spill 先頭は mov [rsp+8], rax の 0x48"
    );
    assert_eq!(lines[9], "137", "stack arg7 spill 2 byte 目は 0x89");
    assert_eq!(lines[10], "68", "stack arg7 spill 3 byte 目は ModRM 0x44");
    assert_eq!(lines[11], "36", "stack arg7 spill 4 byte 目は SIB 0x24");
    assert_eq!(lines[12], "8", "stack arg7 spill disp8 は 8");
    assert_eq!(
        lines[13], "72",
        "stack arg6 spill 先頭は mov [rsp], rcx の 0x48"
    );
    assert_eq!(lines[14], "137", "stack arg6 spill 2 byte 目は 0x89");
    assert_eq!(lines[15], "12", "stack arg6 spill 3 byte 目は ModRM 0x0C");
    assert_eq!(lines[16], "36", "stack arg6 spill 4 byte 目は SIB 0x24");
    assert_eq!(
        lines[17], "76",
        "arg5 load 先頭は mov r9, [rbp-offset] の 0x4C"
    );
    assert_eq!(lines[18], "139", "arg5 load 2 byte 目は 0x8B");
    assert_eq!(lines[19], "141", "arg5 load 3 byte 目は ModRM 0x8D");
    assert_eq!(lines[20], "248", "arg5 spill load offset byte0 は -8");
    assert_eq!(lines[21], "255", "arg5 spill load offset byte1 は 0xFF");
    assert_eq!(lines[22], "255", "arg5 spill load offset byte2 は 0xFF");
    assert_eq!(lines[23], "255", "arg5 spill load offset byte3 は 0xFF");
    assert_eq!(
        lines[24], "76",
        "arg4 load 先頭は mov r8, [rbp-offset] の 0x4C"
    );
    assert_eq!(lines[25], "139", "arg4 load 2 byte 目は 0x8B");
    assert_eq!(lines[26], "133", "arg4 load 3 byte 目は ModRM 0x85");
    assert_eq!(lines[27], "240", "arg4 spill load offset byte0 は -16");
    assert_eq!(lines[28], "255", "arg4 spill load offset byte1 は 0xFF");
    assert_eq!(lines[29], "255", "arg4 spill load offset byte2 は 0xFF");
    assert_eq!(lines[30], "255", "arg4 spill load offset byte3 は 0xFF");
    assert_eq!(
        lines[31], "72",
        "arg3 load 先頭は mov rcx, [rbp-offset] の 0x48"
    );
    assert_eq!(lines[32], "139", "arg3 load 2 byte 目は 0x8B");
    assert_eq!(lines[33], "141", "arg3 load 3 byte 目は ModRM 0x8D");
    assert_eq!(lines[34], "232", "arg3 spill load offset byte0 は -24");
    assert_eq!(lines[35], "255", "arg3 spill load offset byte1 は 0xFF");
    assert_eq!(lines[36], "255", "arg3 spill load offset byte2 は 0xFF");
    assert_eq!(lines[37], "255", "arg3 spill load offset byte3 は 0xFF");
    assert_eq!(
        lines[38], "72",
        "arg2 load 先頭は mov rdx, [rbp-offset] の 0x48"
    );
    assert_eq!(lines[39], "139", "arg2 load 2 byte 目は 0x8B");
    assert_eq!(lines[40], "149", "arg2 load 3 byte 目は ModRM 0x95");
    assert_eq!(lines[41], "224", "arg2 spill load offset byte0 は -32");
    assert_eq!(lines[42], "255", "arg2 spill load offset byte1 は 0xFF");
    assert_eq!(lines[43], "255", "arg2 spill load offset byte2 は 0xFF");
    assert_eq!(lines[44], "255", "arg2 spill load offset byte3 は 0xFF");
    assert_eq!(
        lines[45], "72",
        "arg1 load 先頭は mov rsi, [rbp-offset] の 0x48"
    );
    assert_eq!(lines[46], "139", "arg1 load 2 byte 目は 0x8B");
    assert_eq!(lines[47], "181", "arg1 load 3 byte 目は ModRM 0xB5");
    assert_eq!(lines[48], "216", "arg1 spill load offset byte0 は -40");
    assert_eq!(lines[49], "255", "arg1 spill load offset byte1 は 0xFF");
    assert_eq!(lines[50], "255", "arg1 spill load offset byte2 は 0xFF");
    assert_eq!(lines[51], "255", "arg1 spill load offset byte3 は 0xFF");
    assert_eq!(
        lines[52], "72",
        "arg0 load 先頭は mov rdi, [rbp-offset] の 0x48"
    );
    assert_eq!(lines[53], "139", "arg0 load 2 byte 目は 0x8B");
    assert_eq!(lines[54], "189", "arg0 load 3 byte 目は ModRM 0xBD");
    assert_eq!(lines[55], "208", "arg0 spill load offset byte0 は -48");
    assert_eq!(lines[56], "255", "arg0 spill load offset byte1 は 0xFF");
    assert_eq!(lines[57], "255", "arg0 spill load offset byte2 は 0xFF");
    assert_eq!(lines[58], "255", "arg0 spill load offset byte3 は 0xFF");
    assert_eq!(lines[59], "232", "direct call は call rel32 opcode 0xE8");
    assert_eq!(lines[60], "16", "forward call offset の下位 byte は 16");
    assert_eq!(lines[61], "0", "forward call offset byte1 は 0");
    assert_eq!(lines[62], "0", "forward call offset byte2 は 0");
    assert_eq!(lines[63], "0", "forward call offset byte3 は 0");
    assert_eq!(
        lines[64], "72",
        "post-call stack restore 先頭は add rsp, 16 の 0x48"
    );
    assert_eq!(lines[65], "129", "post-call stack restore 2 byte 目は 0x81");
    assert_eq!(
        lines[66], "196",
        "post-call stack restore 3 byte 目は ModRM 0xC4"
    );
    assert_eq!(lines[67], "16", "post-call stack restore imm byte0 は 16");
    assert_eq!(lines[68], "0", "post-call stack restore imm byte1 は 0");
    assert_eq!(lines[69], "0", "post-call stack restore imm byte2 は 0");
    assert_eq!(lines[70], "0", "post-call stack restore imm byte3 は 0");
    assert_eq!(lines[71], "72", "callee param0 spill 先頭は 0x48");
    assert_eq!(lines[72], "137", "callee param0 spill 2 byte 目は 0x89");
    assert_eq!(
        lines[73], "189",
        "callee param0 spill 3 byte 目は ModRM 0xBD"
    );
    assert_eq!(lines[74], "72", "callee param1 spill 先頭は 0x48");
    assert_eq!(lines[75], "137", "callee param1 spill 2 byte 目は 0x89");
    assert_eq!(
        lines[76], "181",
        "callee param1 spill 3 byte 目は ModRM 0xB5"
    );
    assert_eq!(lines[77], "72", "callee param2 spill 先頭は 0x48");
    assert_eq!(lines[78], "137", "callee param2 spill 2 byte 目は 0x89");
    assert_eq!(
        lines[79], "149",
        "callee param2 spill 3 byte 目は ModRM 0x95"
    );
    assert_eq!(lines[80], "72", "callee param3 spill 先頭は 0x48");
    assert_eq!(lines[81], "137", "callee param3 spill 2 byte 目は 0x89");
    assert_eq!(
        lines[82], "141",
        "callee param3 spill 3 byte 目は ModRM 0x8D"
    );
    assert_eq!(
        lines[83], "76",
        "callee param4 spill 先頭は mov [rbp-offset], r8 の 0x4C"
    );
    assert_eq!(lines[84], "137", "callee param4 spill 2 byte 目は 0x89");
    assert_eq!(
        lines[85], "133",
        "callee param4 spill 3 byte 目は ModRM 0x85"
    );
    assert_eq!(
        lines[86], "76",
        "callee param5 spill 先頭は mov [rbp-offset], r9 の 0x4C"
    );
    assert_eq!(lines[87], "137", "callee param5 spill 2 byte 目は 0x89");
    assert_eq!(
        lines[88], "141",
        "callee param5 spill 3 byte 目は ModRM 0x8D"
    );
    assert_eq!(
        lines[89], "72",
        "callee stack arg6 load 先頭は mov rax, [rbp+16] の 0x48"
    );
    assert_eq!(lines[90], "139", "callee stack arg6 load 2 byte 目は 0x8B");
    assert_eq!(
        lines[91], "69",
        "callee stack arg6 load 3 byte 目は ModRM 0x45"
    );
    assert_eq!(lines[92], "16", "callee stack arg6 load disp8 は 16");
    assert_eq!(lines[93], "72", "callee param6 spill 先頭は 0x48");
    assert_eq!(lines[94], "137", "callee param6 spill 2 byte 目は 0x89");
    assert_eq!(
        lines[95], "133",
        "callee param6 spill 3 byte 目は ModRM 0x85"
    );
    assert_eq!(
        lines[96], "72",
        "callee stack arg7 load 先頭は mov rax, [rbp+24] の 0x48"
    );
    assert_eq!(lines[97], "139", "callee stack arg7 load 2 byte 目は 0x8B");
    assert_eq!(
        lines[98], "69",
        "callee stack arg7 load 3 byte 目は ModRM 0x45"
    );
    assert_eq!(lines[99], "24", "callee stack arg7 load disp8 は 24");
    assert_eq!(lines[100], "72", "callee param7 spill 先頭は 0x48");
    assert_eq!(lines[101], "137", "callee param7 spill 2 byte 目は 0x89");
    assert_eq!(
        lines[102], "133",
        "callee param7 spill 3 byte 目は ModRM 0x85"
    );
    assert_eq!(lines[103], "93", "payload 末尾手前は pop rbp");
    assert_eq!(lines[104], "195", "payload 末尾は ret");
}

/// NATIVE-REAL-08n2: x86_64 で 8 引数 direct call 後も outer value を rcx に復元すること
#[test]
#[ignore]
fn test_native_codegen_emits_x86_nested_eight_arg_call_restores_outer_value() {
    let output = run_native_codegen_harness(
        r#"(module Main)
(import NativeTarget)
(import NativeCodegen)
(import IR.IR)

(defn make-function-meta [param-count local-count ir]
  (vector-push
    (vector-push
      (vector-push (vector-new 3) param-count)
      local-count)
    ir))

(defn print-range [bytes idx end]
  (if (>= idx end)
    0
    (do
      (print (vector-get bytes idx))
      (print-range bytes (+ idx 1) end))))

(defn call-depth-loop [ir funcs idx end depth]
  (if (>= idx end)
    -1
    (let [instr (vector-get ir idx)
      opcode (vector-get instr 0)
      operand (vector-get instr 1)]
      (if (= opcode 40)
        depth
        (call-depth-loop ir funcs (+ idx 1) end (apply-stack-delta depth (opcode-stack-delta opcode operand funcs)))))))

(defn main []
  (let [caller-ir (vector-push
                    (vector-push
                      (vector-push
                        (vector-push
                          (vector-push
                            (vector-push
                              (vector-push
                                (vector-push
                                  (vector-push
                                    (vector-push
                                      (vector-push (vector-new 11) (make-instr 3 100))
                                      (make-instr 3 2))
                                    (make-instr 3 5))
                                  (make-instr 3 7))
                                (make-instr 3 11))
                              (make-instr 3 14))
                            (make-instr 3 17))
                          (make-instr 3 19))
                        (make-instr 3 23))
                      (make-call 1))
                    (make-instr 24 0))
        callee-ir (vector-push (vector-new 1) (make-local-get 0))
        caller (make-function-meta 0 0 caller-ir)
        callee (make-function-meta 8 0 callee-ir)
        functions (vector-push (vector-push (vector-new 2) caller) callee)
        starts (collect-function-starts-x86 functions)
        caller-end (vector-get starts 1)
        call-depth (call-depth-loop caller-ir functions 0 (vector-length caller-ir) 0)
        target (make-target 1)
        native (emit-native-function-meta-bundle functions target)]
    (do
      (print call-depth)
      (print caller-end)
      (print-range native 0 caller-end)
      0)))"#,
    );
    let values: Vec<u32> = output
        .trim()
        .lines()
        .map(|line| {
            line.parse::<u32>().unwrap_or_else(|_| {
                panic!("x86 nested eight-arg restore: 数値出力であるべきだが `{line}` を得た")
            })
        })
        .collect();
    assert!(
        values.len() > 2,
        "x86 nested eight-arg restore: caller bytes 出力が不足: {values:?}"
    );
    assert_eq!(
        values[0], 9,
        "x86 nested eight-arg restore: call 時 depth は outer+8 args の 9"
    );
    let caller_bytes = &values[2..];
    let call_idx = caller_bytes
        .windows(12)
        .position(|window| window[0] == 232 && window[5..12] == [72, 129, 196, 16, 0, 0, 0])
        .expect(
            "x86 nested eight-arg restore: stack restore 付き call rel32 opcode が見つからない",
        );
    let after_call = call_idx + 5;
    assert_eq!(
        &caller_bytes[after_call..after_call + 7],
        &[72, 129, 196, 16, 0, 0, 0],
        "x86 nested eight-arg restore: call 後はまず stack arg を解放する"
    );
    assert_eq!(
        &caller_bytes[after_call + 7..after_call + 10],
        &[72, 139, 141],
        "x86 nested eight-arg restore: Drop 前に outer value を rcx へ復元する"
    );
}
