
/// NATIVE-REAL-08o: x86_64 で 9 引数 direct call bundle が 3 stack arg を持つこと
#[test]
#[ignore]
fn test_native_codegen_emits_x86_direct_call_nine_arg_bundle_bytes() {
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
                                  (vector-push
                                    (vector-push (vector-new 10) (make-instr 3 40))
                                    (make-instr 3 2))
                                  (make-instr 3 5))
                                (make-instr 3 7))
                              (make-instr 3 11))
                            (make-instr 3 14))
                          (make-instr 3 17))
                        (make-instr 3 19))
                      (make-instr 3 23))
                    (make-call 1))
        callee-ir-head (vector-push
                         (vector-push
                           (vector-push
                             (vector-push
                               (vector-push
                                 (vector-push
                                   (vector-push
                                     (vector-push
                                       (vector-push
                                         (vector-push
                                           (vector-push
                                             (vector-push (vector-new 17) (make-local-get 0))
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
                         (make-local-get 6))
        callee-ir-mid (vector-push
                        (vector-push callee-ir-head (make-instr 24 0))
                        (make-local-get 7))
        callee-ir-tail (vector-push
                         (vector-push callee-ir-mid (make-instr 24 0))
                         (make-local-get 8))
        callee-ir (vector-push callee-ir-tail (make-instr 24 0))
        caller (make-function-meta 0 0 caller-ir)
        callee (make-function-meta 9 0 callee-ir)
        functions (vector-push (vector-push (vector-new 2) caller) callee)
        target (make-target 1)
        native (emit-native-function-meta-bundle functions target)]
    (do
      (print (vector-length native))
      (print (vector-get native 426))
      (print (vector-get native 427))
      (print (vector-get native 428))
      (print (vector-get native 429))
      (print (vector-get native 430))
      (print (vector-get native 431))
      (print (vector-get native 432))
      (print (vector-get native 433))
      (print (vector-get native 434))
      (print (vector-get native 435))
      (print (vector-get native 436))
      (print (vector-get native 437))
      (print (vector-get native 438))
      (print (vector-get native 439))
      (print (vector-get native 440))
      (print (vector-get native 441))
      (print (vector-get native 442))
      (print (vector-get native 443))
      (print (vector-get native 444))
      (print (vector-get native 445))
      (print (vector-get native 446))
      (print (vector-get native 447))
      (print (vector-get native 448))
      (print (vector-get native 449))
      (print (vector-get native 450))
      (print (vector-get native 451))
      (print (vector-get native 452))
      (print (vector-get native 453))
      (print (vector-get native 454))
      (print (vector-get native 455))
      (print (vector-get native 456))
      (print (vector-get native 457))
      (print (vector-get native 458))
      (print (vector-get native 459))
      (print (vector-get native 460))
      (print (vector-get native 461))
      (print (vector-get native 462))
      (print (vector-get native 463))
      (print (vector-get native 464))
      (print (vector-get native 465))
      (print (vector-get native 466))
      (print (vector-get native 467))
      (print (vector-get native 468))
      (print (vector-get native 469))
      (print (vector-get native 470))
      (print (vector-get native 471))
      (print (vector-get native 472))
      (print (vector-get native 473))
      (print (vector-get native 474))
      (print (vector-get native 475))
      (print (vector-get native 476))
      (print (vector-get native 477))
      (print (vector-get native 478))
      (print (vector-get native 479))
      (print (vector-get native 480))
      (print (vector-get native 481))
      (print (vector-get native 482))
      (print (vector-get native 483))
      (print (vector-get native 484))
      (print (vector-get native 485))
      (print (vector-get native 486))
      (print (vector-get native 487))
      (print (vector-get native 488))
      (print (vector-get native 489))
      (print (vector-get native 490))
      (print (vector-get native 491))
      (print (vector-get native 492))
      (print (vector-get native 493))
      (print (vector-get native 494))
      (print (vector-get native 495))
      (print (vector-get native 496))
      (print (vector-get native 497))
      (print (vector-get native 498))
      (print (vector-get native 499))
      (print (vector-get native 500))
      (print (vector-get native 501))
      (print (vector-get native 502))
      (print (vector-get native 503))
      (print (vector-get native 504))
      (print (vector-get native 505))
      (print (vector-get native 506))
      (print (vector-get native 507))
      (print (vector-get native 528))
      (print (vector-get native 529))
      (print (vector-get native 530))
      (print (vector-get native 535))
      (print (vector-get native 536))
      (print (vector-get native 537))
      (print (vector-get native 542))
      (print (vector-get native 543))
      (print (vector-get native 544))
      (print (vector-get native 549))
      (print (vector-get native 550))
      (print (vector-get native 551))
      (print (vector-get native 556))
      (print (vector-get native 557))
      (print (vector-get native 558))
      (print (vector-get native 563))
      (print (vector-get native 564))
      (print (vector-get native 565))
      (print (vector-get native 570))
      (print (vector-get native 571))
      (print (vector-get native 572))
      (print (vector-get native 573))
      (print (vector-get native 574))
      (print (vector-get native 575))
      (print (vector-get native 576))
      (print (vector-get native 581))
      (print (vector-get native 582))
      (print (vector-get native 583))
      (print (vector-get native 584))
      (print (vector-get native 585))
      (print (vector-get native 586))
      (print (vector-get native 587))
      (print (vector-get native 592))
      (print (vector-get native 593))
      (print (vector-get native 594))
      (print (vector-get native 595))
      (print (vector-get native 596))
      (print (vector-get native 597))
      (print (vector-get native 598))
      (print (vector-get native 716))
      (print (vector-get native 717))
      0)))"#,
    );
    std::fs::write(
        "/Users/biwakonbu/.copilot/session-state/263086ba-9594-429c-a3e4-f0d13815c738/files/native30.txt",
        &output,
    )
    .unwrap();

    let lines: Vec<&str> = output.trim().lines().collect();

    assert!(
        lines.len() >= 123,
        "x86 direct call nine-arg bundle bytes 出力が不足: {:?}",
        lines
    );
    assert_eq!(
        lines[0], "718",
        "x86_64 direct call nine-arg bundle payload は 718 bytes であるべき"
    );
    assert_eq!(
        lines[1], "72",
        "stack arg reserve 先頭は sub rsp, 32 の 0x48"
    );
    assert_eq!(lines[2], "129", "stack arg reserve 2 byte 目は 0x81");
    assert_eq!(lines[3], "236", "stack arg reserve 3 byte 目は ModRM 0xEC");
    assert_eq!(lines[4], "32", "stack arg reserve imm byte0 は 32");
    assert_eq!(lines[5], "0", "stack arg reserve imm byte1 は 0");
    assert_eq!(lines[6], "0", "stack arg reserve imm byte2 は 0");
    assert_eq!(lines[7], "0", "stack arg reserve imm byte3 は 0");
    assert_eq!(
        lines[8], "72",
        "stack arg8 spill 先頭は mov [rsp+16], rax の 0x48"
    );
    assert_eq!(lines[9], "137", "stack arg8 spill 2 byte 目は 0x89");
    assert_eq!(lines[10], "68", "stack arg8 spill 3 byte 目は ModRM 0x44");
    assert_eq!(lines[11], "36", "stack arg8 spill 4 byte 目は SIB 0x24");
    assert_eq!(lines[12], "16", "stack arg8 spill disp8 は 16");
    assert_eq!(
        lines[13], "72",
        "stack arg7 spill 先頭は mov [rsp+8], rcx の 0x48"
    );
    assert_eq!(lines[14], "137", "stack arg7 spill 2 byte 目は 0x89");
    assert_eq!(lines[15], "76", "stack arg7 spill 3 byte 目は ModRM 0x4C");
    assert_eq!(lines[16], "36", "stack arg7 spill 4 byte 目は SIB 0x24");
    assert_eq!(lines[17], "8", "stack arg7 spill disp8 は 8");
    assert_eq!(
        lines[18], "76",
        "arg6 load 先頭は mov r9, [rbp-offset] の 0x4C"
    );
    assert_eq!(lines[19], "139", "arg6 load 2 byte 目は 0x8B");
    assert_eq!(lines[20], "141", "arg6 load 3 byte 目は ModRM 0x8D");
    assert_eq!(lines[21], "248", "arg6 spill load offset byte0 は -8");
    assert_eq!(lines[22], "255", "arg6 spill load offset byte1 は 0xFF");
    assert_eq!(lines[23], "255", "arg6 spill load offset byte2 は 0xFF");
    assert_eq!(lines[24], "255", "arg6 spill load offset byte3 は 0xFF");
    assert_eq!(
        lines[25], "76",
        "stack arg6 spill 先頭は mov [rsp], r9 の 0x4C"
    );
    assert_eq!(lines[26], "137", "stack arg6 spill 2 byte 目は 0x89");
    assert_eq!(lines[27], "12", "stack arg6 spill 3 byte 目は ModRM 0x0C");
    assert_eq!(lines[28], "36", "stack arg6 spill 4 byte 目は SIB 0x24");
    assert_eq!(
        lines[29], "76",
        "arg5 load 先頭は mov r9, [rbp-offset] の 0x4C"
    );
    assert_eq!(lines[30], "139", "arg5 load 2 byte 目は 0x8B");
    assert_eq!(lines[31], "141", "arg5 load 3 byte 目は ModRM 0x8D");
    assert_eq!(lines[32], "240", "arg5 spill load offset byte0 は -16");
    assert_eq!(lines[33], "255", "arg5 spill load offset byte1 は 0xFF");
    assert_eq!(lines[34], "255", "arg5 spill load offset byte2 は 0xFF");
    assert_eq!(lines[35], "255", "arg5 spill load offset byte3 は 0xFF");
    assert_eq!(
        lines[36], "76",
        "arg4 load 先頭は mov r8, [rbp-offset] の 0x4C"
    );
    assert_eq!(lines[37], "139", "arg4 load 2 byte 目は 0x8B");
    assert_eq!(lines[38], "133", "arg4 load 3 byte 目は ModRM 0x85");
    assert_eq!(lines[39], "232", "arg4 spill load offset byte0 は -24");
    assert_eq!(lines[40], "255", "arg4 spill load offset byte1 は 0xFF");
    assert_eq!(lines[41], "255", "arg4 spill load offset byte2 は 0xFF");
    assert_eq!(lines[42], "255", "arg4 spill load offset byte3 は 0xFF");
    assert_eq!(
        lines[43], "72",
        "arg3 load 先頭は mov rcx, [rbp-offset] の 0x48"
    );
    assert_eq!(lines[44], "139", "arg3 load 2 byte 目は 0x8B");
    assert_eq!(lines[45], "141", "arg3 load 3 byte 目は ModRM 0x8D");
    assert_eq!(lines[46], "224", "arg3 spill load offset byte0 は -32");
    assert_eq!(lines[47], "255", "arg3 spill load offset byte1 は 0xFF");
    assert_eq!(lines[48], "255", "arg3 spill load offset byte2 は 0xFF");
    assert_eq!(lines[49], "255", "arg3 spill load offset byte3 は 0xFF");
    assert_eq!(
        lines[50], "72",
        "arg2 load 先頭は mov rdx, [rbp-offset] の 0x48"
    );
    assert_eq!(lines[51], "139", "arg2 load 2 byte 目は 0x8B");
    assert_eq!(lines[52], "149", "arg2 load 3 byte 目は ModRM 0x95");
    assert_eq!(lines[53], "216", "arg2 spill load offset byte0 は -40");
    assert_eq!(lines[54], "255", "arg2 spill load offset byte1 は 0xFF");
    assert_eq!(lines[55], "255", "arg2 spill load offset byte2 は 0xFF");
    assert_eq!(lines[56], "255", "arg2 spill load offset byte3 は 0xFF");
    assert_eq!(
        lines[57], "72",
        "arg1 load 先頭は mov rsi, [rbp-offset] の 0x48"
    );
    assert_eq!(lines[58], "139", "arg1 load 2 byte 目は 0x8B");
    assert_eq!(lines[59], "181", "arg1 load 3 byte 目は ModRM 0xB5");
    assert_eq!(lines[60], "208", "arg1 spill load offset byte0 は -48");
    assert_eq!(lines[61], "255", "arg1 spill load offset byte1 は 0xFF");
    assert_eq!(lines[62], "255", "arg1 spill load offset byte2 は 0xFF");
    assert_eq!(lines[63], "255", "arg1 spill load offset byte3 は 0xFF");
    assert_eq!(
        lines[64], "72",
        "arg0 load 先頭は mov rdi, [rbp-offset] の 0x48"
    );
    assert_eq!(lines[65], "139", "arg0 load 2 byte 目は 0x8B");
    assert_eq!(lines[66], "189", "arg0 load 3 byte 目は ModRM 0xBD");
    assert_eq!(lines[67], "200", "arg0 spill load offset byte0 は -56");
    assert_eq!(lines[68], "255", "arg0 spill load offset byte1 は 0xFF");
    assert_eq!(lines[69], "255", "arg0 spill load offset byte2 は 0xFF");
    assert_eq!(lines[70], "255", "arg0 spill load offset byte3 は 0xFF");
    assert_eq!(lines[71], "232", "direct call は call rel32 opcode 0xE8");
    assert_eq!(lines[72], "16", "forward call offset の下位 byte は 16");
    assert_eq!(lines[73], "0", "forward call offset byte1 は 0");
    assert_eq!(lines[74], "0", "forward call offset byte2 は 0");
    assert_eq!(lines[75], "0", "forward call offset byte3 は 0");
    assert_eq!(
        lines[76], "72",
        "post-call stack restore 先頭は add rsp, 32 の 0x48"
    );
    assert_eq!(lines[77], "129", "post-call stack restore 2 byte 目は 0x81");
    assert_eq!(
        lines[78], "196",
        "post-call stack restore 3 byte 目は ModRM 0xC4"
    );
    assert_eq!(lines[79], "32", "post-call stack restore imm byte0 は 32");
    assert_eq!(lines[80], "0", "post-call stack restore imm byte1 は 0");
    assert_eq!(lines[81], "0", "post-call stack restore imm byte2 は 0");
    assert_eq!(lines[82], "0", "post-call stack restore imm byte3 は 0");
    assert_eq!(lines[83], "72", "callee param0 spill 先頭は 0x48");
    assert_eq!(lines[84], "137", "callee param0 spill 2 byte 目は 0x89");
    assert_eq!(
        lines[85], "189",
        "callee param0 spill 3 byte 目は ModRM 0xBD"
    );
    assert_eq!(lines[86], "72", "callee param1 spill 先頭は 0x48");
    assert_eq!(lines[87], "137", "callee param1 spill 2 byte 目は 0x89");
    assert_eq!(
        lines[88], "181",
        "callee param1 spill 3 byte 目は ModRM 0xB5"
    );
    assert_eq!(lines[89], "72", "callee param2 spill 先頭は 0x48");
    assert_eq!(lines[90], "137", "callee param2 spill 2 byte 目は 0x89");
    assert_eq!(
        lines[91], "149",
        "callee param2 spill 3 byte 目は ModRM 0x95"
    );
    assert_eq!(lines[92], "72", "callee param3 spill 先頭は 0x48");
    assert_eq!(lines[93], "137", "callee param3 spill 2 byte 目は 0x89");
    assert_eq!(
        lines[94], "141",
        "callee param3 spill 3 byte 目は ModRM 0x8D"
    );
    assert_eq!(
        lines[95], "76",
        "callee param4 spill 先頭は mov [rbp-offset], r8 の 0x4C"
    );
    assert_eq!(lines[96], "137", "callee param4 spill 2 byte 目は 0x89");
    assert_eq!(
        lines[97], "133",
        "callee param4 spill 3 byte 目は ModRM 0x85"
    );
    assert_eq!(
        lines[98], "76",
        "callee param5 spill 先頭は mov [rbp-offset], r9 の 0x4C"
    );
    assert_eq!(lines[99], "137", "callee param5 spill 2 byte 目は 0x89");
    assert_eq!(
        lines[100], "141",
        "callee param5 spill 3 byte 目は ModRM 0x8D"
    );
    assert_eq!(
        lines[101], "72",
        "callee stack arg6 load 先頭は mov rax, [rbp+16] の 0x48"
    );
    assert_eq!(lines[102], "139", "callee stack arg6 load 2 byte 目は 0x8B");
    assert_eq!(
        lines[103], "69",
        "callee stack arg6 load 3 byte 目は ModRM 0x45"
    );
    assert_eq!(lines[104], "16", "callee stack arg6 load disp8 は 16");
    assert_eq!(lines[105], "72", "callee param6 spill 先頭は 0x48");
    assert_eq!(lines[106], "137", "callee param6 spill 2 byte 目は 0x89");
    assert_eq!(
        lines[107], "133",
        "callee param6 spill 3 byte 目は ModRM 0x85"
    );
    assert_eq!(
        lines[108], "72",
        "callee stack arg7 load 先頭は mov rax, [rbp+24] の 0x48"
    );
    assert_eq!(lines[109], "139", "callee stack arg7 load 2 byte 目は 0x8B");
    assert_eq!(
        lines[110], "69",
        "callee stack arg7 load 3 byte 目は ModRM 0x45"
    );
    assert_eq!(lines[111], "24", "callee stack arg7 load disp8 は 24");
    assert_eq!(lines[112], "72", "callee param7 spill 先頭は 0x48");
    assert_eq!(lines[113], "137", "callee param7 spill 2 byte 目は 0x89");
    assert_eq!(
        lines[114], "133",
        "callee param7 spill 3 byte 目は ModRM 0x85"
    );
    assert_eq!(
        lines[115], "72",
        "callee stack arg8 load 先頭は mov rax, [rbp+32] の 0x48"
    );
    assert_eq!(lines[116], "139", "callee stack arg8 load 2 byte 目は 0x8B");
    assert_eq!(
        lines[117], "69",
        "callee stack arg8 load 3 byte 目は ModRM 0x45"
    );
    assert_eq!(lines[118], "32", "callee stack arg8 load disp8 は 32");
    assert_eq!(lines[119], "72", "callee param8 spill 先頭は 0x48");
    assert_eq!(lines[120], "137", "callee param8 spill 2 byte 目は 0x89");
    assert_eq!(
        lines[121], "133",
        "callee param8 spill 3 byte 目は ModRM 0x85"
    );
    assert_eq!(lines[122], "93", "payload 末尾手前は pop rbp");
    assert_eq!(lines[123], "195", "payload 末尾は ret");
}

/// NATIVE-REAL-08p: x86_64 で 10 引数 direct call bundle が 4 stack arg を持つこと
#[test]
#[ignore]
fn test_native_codegen_emits_x86_direct_call_ten_arg_bundle_bytes() {
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
                                  (vector-push
                                    (vector-push
                                      (vector-push (vector-new 11) (make-instr 3 40))
                                      (make-instr 3 2))
                                    (make-instr 3 5))
                                  (make-instr 3 7))
                                (make-instr 3 11))
                              (make-instr 3 14))
                            (make-instr 3 17))
                          (make-instr 3 19))
                        (make-instr 3 23))
                      (make-instr 3 29))
                    (make-call 1))
        callee-ir-head (vector-push
                         (vector-push
                           (vector-push
                             (vector-push
                               (vector-push
                                 (vector-push
                                   (vector-push
                                     (vector-push
                                       (vector-push
                                         (vector-push
                                           (vector-push (vector-new 17) (make-local-get 0))
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
                        (vector-push callee-ir-head (make-local-get 6))
                        (make-instr 24 0))
        callee-ir-tail (vector-push
                         (vector-push callee-ir-mid (make-local-get 7))
                         (make-instr 24 0))
        callee-ir-more (vector-push
                         (vector-push callee-ir-tail (make-local-get 8))
                         (make-instr 24 0))
        callee-ir (vector-push
                    (vector-push callee-ir-more (make-local-get 9))
                    (make-instr 24 0))
        caller (make-function-meta 0 0 caller-ir)
        callee (make-function-meta 10 0 callee-ir)
        functions (vector-push (vector-push (vector-new 2) caller) callee)
        target (make-target 1)
        native (emit-native-function-meta-bundle functions target)]
    (do
      (print (vector-length native))
      (print (vector-get native 539))
      (print (vector-get native 540))
      (print (vector-get native 541))
      (print (vector-get native 542))
      (print (vector-get native 543))
      (print (vector-get native 544))
      (print (vector-get native 545))
      (print (vector-get native 546))
      (print (vector-get native 547))
      (print (vector-get native 548))
      (print (vector-get native 549))
      (print (vector-get native 550))
      (print (vector-get native 551))
      (print (vector-get native 552))
      (print (vector-get native 553))
      (print (vector-get native 554))
      (print (vector-get native 555))
      (print (vector-get native 556))
      (print (vector-get native 557))
      (print (vector-get native 558))
      (print (vector-get native 559))
      (print (vector-get native 560))
      (print (vector-get native 561))
      (print (vector-get native 562))
      (print (vector-get native 563))
      (print (vector-get native 564))
      (print (vector-get native 565))
      (print (vector-get native 566))
      (print (vector-get native 567))
      (print (vector-get native 568))
      (print (vector-get native 569))
      (print (vector-get native 570))
      (print (vector-get native 571))
      (print (vector-get native 572))
      (print (vector-get native 573))
      (print (vector-get native 574))
      (print (vector-get native 575))
      (print (vector-get native 576))
      (print (vector-get native 577))
      (print (vector-get native 578))
      (print (vector-get native 579))
      (print (vector-get native 580))
      (print (vector-get native 581))
      (print (vector-get native 582))
      (print (vector-get native 583))
      (print (vector-get native 584))
      (print (vector-get native 585))
      (print (vector-get native 586))
      (print (vector-get native 587))
      (print (vector-get native 588))
      (print (vector-get native 589))
      (print (vector-get native 590))
      (print (vector-get native 591))
      (print (vector-get native 592))
      (print (vector-get native 593))
      (print (vector-get native 594))
      (print (vector-get native 595))
      (print (vector-get native 596))
      (print (vector-get native 597))
      (print (vector-get native 598))
      (print (vector-get native 599))
      (print (vector-get native 600))
      (print (vector-get native 601))
      (print (vector-get native 602))
      (print (vector-get native 603))
      (print (vector-get native 604))
      (print (vector-get native 605))
      (print (vector-get native 606))
      (print (vector-get native 607))
      (print (vector-get native 608))
      (print (vector-get native 609))
      (print (vector-get native 610))
      (print (vector-get native 611))
      (print (vector-get native 612))
      (print (vector-get native 613))
      (print (vector-get native 614))
      (print (vector-get native 615))
      (print (vector-get native 616))
      (print (vector-get native 617))
      (print (vector-get native 618))
      (print (vector-get native 619))
      (print (vector-get native 620))
      (print (vector-get native 621))
      (print (vector-get native 622))
      (print (vector-get native 623))
      (print (vector-get native 624))
      (print (vector-get native 625))
      (print (vector-get native 626))
      (print (vector-get native 627))
      (print (vector-get native 628))
      (print (vector-get native 629))
      (print (vector-get native 630))
      (print (vector-get native 631))
      (print (vector-get native 632))
      (print (vector-get native 653))
      (print (vector-get native 654))
      (print (vector-get native 655))
      (print (vector-get native 660))
      (print (vector-get native 661))
      (print (vector-get native 662))
      (print (vector-get native 667))
      (print (vector-get native 668))
      (print (vector-get native 669))
      (print (vector-get native 674))
      (print (vector-get native 675))
      (print (vector-get native 676))
      (print (vector-get native 681))
      (print (vector-get native 682))
      (print (vector-get native 683))
      (print (vector-get native 688))
      (print (vector-get native 689))
      (print (vector-get native 690))
      (print (vector-get native 695))
      (print (vector-get native 696))
      (print (vector-get native 697))
      (print (vector-get native 698))
      (print (vector-get native 699))
      (print (vector-get native 700))
      (print (vector-get native 701))
      (print (vector-get native 706))
      (print (vector-get native 707))
      (print (vector-get native 708))
      (print (vector-get native 709))
      (print (vector-get native 710))
      (print (vector-get native 711))
      (print (vector-get native 712))
      (print (vector-get native 717))
      (print (vector-get native 718))
      (print (vector-get native 719))
      (print (vector-get native 720))
      (print (vector-get native 721))
      (print (vector-get native 722))
      (print (vector-get native 723))
      (print (vector-get native 728))
      (print (vector-get native 729))
      (print (vector-get native 730))
      (print (vector-get native 731))
      (print (vector-get native 732))
      (print (vector-get native 733))
      (print (vector-get native 734))
      (print (vector-get native 864))
      (print (vector-get native 865))
      0)))"#,
    );

    let lines: Vec<&str> = output.trim().lines().collect();
    let expected = [
        "866", "72", "129", "236", "32", "0", "0", "0", "72", "137", "68", "36", "24", "72", "137",
        "76", "36", "16", "72", "139", "141", "248", "255", "255", "255", "72", "137", "76", "36",
        "8", "76", "139", "141", "240", "255", "255", "255", "76", "137", "12", "36", "76", "139",
        "141", "232", "255", "255", "255", "76", "139", "133", "224", "255", "255", "255", "72",
        "139", "141", "216", "255", "255", "255", "72", "139", "149", "208", "255", "255", "255",
        "72", "139", "181", "200", "255", "255", "255", "72", "139", "189", "192", "255", "255",
        "255", "232", "16", "0", "0", "0", "72", "129", "196", "32", "0", "0", "0", "72", "137",
        "189", "72", "137", "181", "72", "137", "149", "72", "137", "141", "76", "137", "133",
        "76", "137", "141", "72", "139", "69", "16", "72", "137", "133", "72", "139", "69", "24",
        "72", "137", "133", "72", "139", "69", "32", "72", "137", "133", "72", "139", "69", "40",
        "72", "137", "133", "93", "195",
    ];

    assert!(
        lines.len() >= expected.len(),
        "x86 direct call ten-arg bundle bytes 出力が不足: {:?}",
        lines
    );
    assert_eq!(
        &lines[..expected.len()],
        expected,
        "x86_64 direct call ten-arg bundle payload/call-layout exact bytes が一致しない"
    );
}
