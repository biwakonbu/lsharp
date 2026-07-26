
/// NATIVE-REAL-08k: x86_64 で 5 引数 direct call bundle が 3-spill load + arg moves + rel32 call bytes を持つこと
#[test]
#[ignore]
fn test_native_codegen_emits_x86_direct_call_five_arg_bundle_bytes() {
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
                            (vector-push (vector-new 6) (make-instr 3 40))
                            (make-instr 3 2))
                          (make-instr 3 5))
                        (make-instr 3 7))
                      (make-instr 3 11))
                    (make-call 1))
        callee-ir-base (vector-push
                         (vector-push
                           (vector-push
                             (vector-push
                               (vector-push
                                 (vector-push
                                   (vector-push
                                     (vector-push (vector-new 9) (make-local-get 0))
                                     (make-local-get 1))
                                   (make-instr 24 0))
                                 (make-local-get 2))
                               (make-instr 24 0))
                             (make-local-get 3))
                           (make-instr 24 0))
                         (make-local-get 4))
        callee-ir (vector-push callee-ir-base (make-instr 24 0))
        caller (make-function-meta 0 0 caller-ir)
        callee (make-function-meta 5 0 callee-ir)
        functions (vector-push (vector-push (vector-new 2) caller) callee)
        target (make-target 1)
        native (emit-native-function-meta-bundle functions target)]
    (do
      (print (vector-length native))
      (print (vector-get native 114))
      (print (vector-get native 115))
      (print (vector-get native 116))
      (print (vector-get native 117))
      (print (vector-get native 118))
      (print (vector-get native 119))
      (print (vector-get native 120))
      (print (vector-get native 121))
      (print (vector-get native 122))
      (print (vector-get native 123))
      (print (vector-get native 124))
      (print (vector-get native 125))
      (print (vector-get native 126))
      (print (vector-get native 127))
      (print (vector-get native 128))
      (print (vector-get native 129))
      (print (vector-get native 130))
      (print (vector-get native 131))
      (print (vector-get native 132))
      (print (vector-get native 133))
      (print (vector-get native 134))
      (print (vector-get native 135))
      (print (vector-get native 136))
      (print (vector-get native 137))
      (print (vector-get native 138))
      (print (vector-get native 139))
      (print (vector-get native 140))
      (print (vector-get native 141))
      (print (vector-get native 142))
      (print (vector-get native 163))
      (print (vector-get native 164))
      (print (vector-get native 165))
      (print (vector-get native 170))
      (print (vector-get native 171))
      (print (vector-get native 172))
      (print (vector-get native 177))
      (print (vector-get native 178))
      (print (vector-get native 179))
      (print (vector-get native 184))
      (print (vector-get native 185))
      (print (vector-get native 186))
      (print (vector-get native 191))
      (print (vector-get native 192))
      (print (vector-get native 193))
      (print (vector-get native 263))
      (print (vector-get native 264))
      0)))"#,
    );
    let lines: Vec<&str> = output.trim().lines().collect();

    assert!(
        lines.len() >= 47,
        "x86 direct call five-arg bundle bytes 出力が不足: {:?}",
        lines
    );
    assert_eq!(
        lines[0], "265",
        "x86_64 direct call five-arg bundle payload は 265 bytes であるべき"
    );
    assert_eq!(lines[1], "73", "arg4 move 先頭は mov r8, rax の 0x49");
    assert_eq!(lines[2], "137", "arg4 move 2 byte 目は 0x89");
    assert_eq!(lines[3], "192", "arg4 move 3 byte 目は ModRM 0xC0");
    assert_eq!(
        lines[4], "72",
        "arg2 load 先頭は mov rdx, [rbp-offset] の 0x48"
    );
    assert_eq!(lines[5], "139", "arg2 load 2 byte 目は 0x8B");
    assert_eq!(lines[6], "149", "arg2 load 3 byte 目は ModRM 0x95");
    assert_eq!(lines[7], "248", "arg2 spill load offset byte0 は -8");
    assert_eq!(lines[8], "255", "arg2 spill load offset byte1 は 0xFF");
    assert_eq!(lines[9], "255", "arg2 spill load offset byte2 は 0xFF");
    assert_eq!(lines[10], "255", "arg2 spill load offset byte3 は 0xFF");
    assert_eq!(
        lines[11], "72",
        "arg1 load 先頭は mov rsi, [rbp-offset] の 0x48"
    );
    assert_eq!(lines[12], "139", "arg1 load 2 byte 目は 0x8B");
    assert_eq!(lines[13], "181", "arg1 load 3 byte 目は ModRM 0xB5");
    assert_eq!(lines[14], "240", "arg1 spill load offset byte0 は -16");
    assert_eq!(lines[15], "255", "arg1 spill load offset byte1 は 0xFF");
    assert_eq!(lines[16], "255", "arg1 spill load offset byte2 は 0xFF");
    assert_eq!(lines[17], "255", "arg1 spill load offset byte3 は 0xFF");
    assert_eq!(
        lines[18], "72",
        "arg0 load 先頭は mov rdi, [rbp-offset] の 0x48"
    );
    assert_eq!(lines[19], "139", "arg0 load 2 byte 目は 0x8B");
    assert_eq!(lines[20], "189", "arg0 load 3 byte 目は ModRM 0xBD");
    assert_eq!(lines[21], "232", "arg0 spill load offset byte0 は -24");
    assert_eq!(lines[22], "255", "arg0 spill load offset byte1 は 0xFF");
    assert_eq!(lines[23], "255", "arg0 spill load offset byte2 は 0xFF");
    assert_eq!(lines[24], "255", "arg0 spill load offset byte3 は 0xFF");
    assert_eq!(lines[25], "232", "direct call は call rel32 opcode 0xE8");
    assert_eq!(lines[26], "9", "forward call offset の下位 byte は 9");
    assert_eq!(lines[27], "0", "forward call offset byte1 は 0");
    assert_eq!(lines[28], "0", "forward call offset byte2 は 0");
    assert_eq!(lines[29], "0", "forward call offset byte3 は 0");
    assert_eq!(lines[30], "72", "callee param0 spill 先頭は 0x48");
    assert_eq!(lines[31], "137", "callee param0 spill 2 byte 目は 0x89");
    assert_eq!(
        lines[32], "189",
        "callee param0 spill 3 byte 目は ModRM 0xBD"
    );
    assert_eq!(lines[33], "72", "callee param1 spill 先頭は 0x48");
    assert_eq!(lines[34], "137", "callee param1 spill 2 byte 目は 0x89");
    assert_eq!(
        lines[35], "181",
        "callee param1 spill 3 byte 目は ModRM 0xB5"
    );
    assert_eq!(lines[36], "72", "callee param2 spill 先頭は 0x48");
    assert_eq!(lines[37], "137", "callee param2 spill 2 byte 目は 0x89");
    assert_eq!(
        lines[38], "149",
        "callee param2 spill 3 byte 目は ModRM 0x95"
    );
    assert_eq!(lines[39], "72", "callee param3 spill 先頭は 0x48");
    assert_eq!(lines[40], "137", "callee param3 spill 2 byte 目は 0x89");
    assert_eq!(
        lines[41], "141",
        "callee param3 spill 3 byte 目は ModRM 0x8D"
    );
    assert_eq!(
        lines[42], "76",
        "callee param4 spill 先頭は mov [rbp-offset], r8 の 0x4C"
    );
    assert_eq!(lines[43], "137", "callee param4 spill 2 byte 目は 0x89");
    assert_eq!(
        lines[44], "133",
        "callee param4 spill 3 byte 目は ModRM 0x85"
    );
    assert_eq!(lines[45], "93", "payload 末尾手前は pop rbp");
    assert_eq!(lines[46], "195", "payload 末尾は ret");
}

/// NATIVE-REAL-08l: x86_64 で 6 引数 direct call bundle が 4-spill load + arg moves + rel32 call bytes を持つこと
#[test]
#[ignore]
fn test_native_codegen_emits_x86_direct_call_six_arg_bundle_bytes() {
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
                              (vector-push (vector-new 7) (make-instr 3 40))
                              (make-instr 3 2))
                            (make-instr 3 5))
                          (make-instr 3 7))
                        (make-instr 3 11))
                      (make-instr 3 14))
                    (make-call 1))
        callee-ir-base (vector-push
                         (vector-push
                           (vector-push
                             (vector-push
                               (vector-push
                                 (vector-push
                                   (vector-push
                                     (vector-push
                                       (vector-push (vector-new 11) (make-local-get 0))
                                       (make-local-get 1))
                                     (make-instr 24 0))
                                   (make-local-get 2))
                                 (make-instr 24 0))
                               (make-local-get 3))
                             (make-instr 24 0))
                           (make-local-get 4))
                         (make-instr 24 0))
        callee-ir (vector-push
                    (vector-push callee-ir-base (make-local-get 5))
                    (make-instr 24 0))
        caller (make-function-meta 0 0 caller-ir)
        callee (make-function-meta 6 0 callee-ir)
        functions (vector-push (vector-push (vector-new 2) caller) callee)
        target (make-target 1)
        native (emit-native-function-meta-bundle functions target)]
    (do
      (print (vector-length native))
      (print (vector-get native 171))
      (print (vector-get native 172))
      (print (vector-get native 173))
      (print (vector-get native 174))
      (print (vector-get native 175))
      (print (vector-get native 176))
      (print (vector-get native 177))
      (print (vector-get native 178))
      (print (vector-get native 179))
      (print (vector-get native 180))
      (print (vector-get native 181))
      (print (vector-get native 182))
      (print (vector-get native 183))
      (print (vector-get native 184))
      (print (vector-get native 185))
      (print (vector-get native 186))
      (print (vector-get native 187))
      (print (vector-get native 188))
      (print (vector-get native 189))
      (print (vector-get native 190))
      (print (vector-get native 191))
      (print (vector-get native 192))
      (print (vector-get native 193))
      (print (vector-get native 194))
      (print (vector-get native 195))
      (print (vector-get native 196))
      (print (vector-get native 197))
      (print (vector-get native 198))
      (print (vector-get native 199))
      (print (vector-get native 200))
      (print (vector-get native 201))
      (print (vector-get native 202))
      (print (vector-get native 203))
      (print (vector-get native 204))
      (print (vector-get native 205))
      (print (vector-get native 206))
      (print (vector-get native 207))
      (print (vector-get native 208))
      (print (vector-get native 209))
      (print (vector-get native 230))
      (print (vector-get native 231))
      (print (vector-get native 232))
      (print (vector-get native 237))
      (print (vector-get native 238))
      (print (vector-get native 239))
      (print (vector-get native 244))
      (print (vector-get native 245))
      (print (vector-get native 246))
      (print (vector-get native 251))
      (print (vector-get native 252))
      (print (vector-get native 253))
      (print (vector-get native 258))
      (print (vector-get native 259))
      (print (vector-get native 260))
      (print (vector-get native 265))
      (print (vector-get native 266))
      (print (vector-get native 267))
      (print (vector-get native 349))
      (print (vector-get native 350))
      0)))"#,
    );
    let lines: Vec<&str> = output.trim().lines().collect();

    assert!(
        lines.len() >= 60,
        "x86 direct call six-arg bundle bytes 出力が不足: {:?}",
        lines
    );
    assert_eq!(
        lines[0], "351",
        "x86_64 direct call six-arg bundle payload は 351 bytes であるべき"
    );
    assert_eq!(lines[1], "73", "arg5 move 先頭は mov r9, rax の 0x49");
    assert_eq!(lines[2], "137", "arg5 move 2 byte 目は 0x89");
    assert_eq!(lines[3], "193", "arg5 move 3 byte 目は ModRM 0xC1");
    assert_eq!(lines[4], "73", "arg4 move 先頭は mov r8, rcx の 0x49");
    assert_eq!(lines[5], "137", "arg4 move 2 byte 目は 0x89");
    assert_eq!(lines[6], "200", "arg4 move 3 byte 目は ModRM 0xC8");
    assert_eq!(
        lines[7], "72",
        "arg3 load 先頭は mov rcx, [rbp-offset] の 0x48"
    );
    assert_eq!(lines[8], "139", "arg3 load 2 byte 目は 0x8B");
    assert_eq!(lines[9], "141", "arg3 load 3 byte 目は ModRM 0x8D");
    assert_eq!(lines[10], "248", "arg3 spill load offset byte0 は -8");
    assert_eq!(lines[11], "255", "arg3 spill load offset byte1 は 0xFF");
    assert_eq!(lines[12], "255", "arg3 spill load offset byte2 は 0xFF");
    assert_eq!(lines[13], "255", "arg3 spill load offset byte3 は 0xFF");
    assert_eq!(
        lines[14], "72",
        "arg2 load 先頭は mov rdx, [rbp-offset] の 0x48"
    );
    assert_eq!(lines[15], "139", "arg2 load 2 byte 目は 0x8B");
    assert_eq!(lines[16], "149", "arg2 load 3 byte 目は ModRM 0x95");
    assert_eq!(lines[17], "240", "arg2 spill load offset byte0 は -16");
    assert_eq!(lines[18], "255", "arg2 spill load offset byte1 は 0xFF");
    assert_eq!(lines[19], "255", "arg2 spill load offset byte2 は 0xFF");
    assert_eq!(lines[20], "255", "arg2 spill load offset byte3 は 0xFF");
    assert_eq!(
        lines[21], "72",
        "arg1 load 先頭は mov rsi, [rbp-offset] の 0x48"
    );
    assert_eq!(lines[22], "139", "arg1 load 2 byte 目は 0x8B");
    assert_eq!(lines[23], "181", "arg1 load 3 byte 目は ModRM 0xB5");
    assert_eq!(lines[24], "232", "arg1 spill load offset byte0 は -24");
    assert_eq!(lines[25], "255", "arg1 spill load offset byte1 は 0xFF");
    assert_eq!(lines[26], "255", "arg1 spill load offset byte2 は 0xFF");
    assert_eq!(lines[27], "255", "arg1 spill load offset byte3 は 0xFF");
    assert_eq!(
        lines[28], "72",
        "arg0 load 先頭は mov rdi, [rbp-offset] の 0x48"
    );
    assert_eq!(lines[29], "139", "arg0 load 2 byte 目は 0x8B");
    assert_eq!(lines[30], "189", "arg0 load 3 byte 目は ModRM 0xBD");
    assert_eq!(lines[31], "224", "arg0 spill load offset byte0 は -32");
    assert_eq!(lines[32], "255", "arg0 spill load offset byte1 は 0xFF");
    assert_eq!(lines[33], "255", "arg0 spill load offset byte2 は 0xFF");
    assert_eq!(lines[34], "255", "arg0 spill load offset byte3 は 0xFF");
    assert_eq!(lines[35], "232", "direct call は call rel32 opcode 0xE8");
    assert_eq!(lines[36], "9", "forward call offset の下位 byte は 9");
    assert_eq!(lines[37], "0", "forward call offset byte1 は 0");
    assert_eq!(lines[38], "0", "forward call offset byte2 は 0");
    assert_eq!(lines[39], "0", "forward call offset byte3 は 0");
    assert_eq!(lines[40], "72", "callee param0 spill 先頭は 0x48");
    assert_eq!(lines[41], "137", "callee param0 spill 2 byte 目は 0x89");
    assert_eq!(
        lines[42], "189",
        "callee param0 spill 3 byte 目は ModRM 0xBD"
    );
    assert_eq!(lines[43], "72", "callee param1 spill 先頭は 0x48");
    assert_eq!(lines[44], "137", "callee param1 spill 2 byte 目は 0x89");
    assert_eq!(
        lines[45], "181",
        "callee param1 spill 3 byte 目は ModRM 0xB5"
    );
    assert_eq!(lines[46], "72", "callee param2 spill 先頭は 0x48");
    assert_eq!(lines[47], "137", "callee param2 spill 2 byte 目は 0x89");
    assert_eq!(
        lines[48], "149",
        "callee param2 spill 3 byte 目は ModRM 0x95"
    );
    assert_eq!(lines[49], "72", "callee param3 spill 先頭は 0x48");
    assert_eq!(lines[50], "137", "callee param3 spill 2 byte 目は 0x89");
    assert_eq!(
        lines[51], "141",
        "callee param3 spill 3 byte 目は ModRM 0x8D"
    );
    assert_eq!(
        lines[52], "76",
        "callee param4 spill 先頭は mov [rbp-offset], r8 の 0x4C"
    );
    assert_eq!(lines[53], "137", "callee param4 spill 2 byte 目は 0x89");
    assert_eq!(
        lines[54], "133",
        "callee param4 spill 3 byte 目は ModRM 0x85"
    );
    assert_eq!(
        lines[55], "76",
        "callee param5 spill 先頭は mov [rbp-offset], r9 の 0x4C"
    );
    assert_eq!(lines[56], "137", "callee param5 spill 2 byte 目は 0x89");
    assert_eq!(
        lines[57], "141",
        "callee param5 spill 3 byte 目は ModRM 0x8D"
    );
    assert_eq!(lines[58], "93", "payload 末尾手前は pop rbp");
    assert_eq!(lines[59], "195", "payload 末尾は ret");
}

/// NATIVE-REAL-08m: x86_64 で 7 引数 direct call bundle が stack arg + 5-spill load + rel32 call bytes を持つこと
#[test]
#[ignore]
fn test_native_codegen_emits_x86_direct_call_seven_arg_bundle_bytes() {
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
                                (vector-push (vector-new 8) (make-instr 3 40))
                                (make-instr 3 2))
                              (make-instr 3 5))
                            (make-instr 3 7))
                          (make-instr 3 11))
                        (make-instr 3 14))
                      (make-instr 3 17))
                    (make-call 1))
        callee-ir-base (vector-push
                         (vector-push
                           (vector-push
                             (vector-push
                               (vector-push
                                 (vector-push
                                  (vector-push
                                    (vector-push
                                      (vector-push (vector-new 13) (make-local-get 0))
                                      (make-local-get 1))
                                    (make-instr 24 0))
                                     (make-local-get 2))
                                   (make-instr 24 0))
                                 (make-local-get 3))
                               (make-instr 24 0))
                             (make-local-get 4))
                           (make-instr 24 0))
        callee-ir-mid (vector-push
                        (vector-push callee-ir-base (make-local-get 5))
                        (make-instr 24 0))
        callee-ir (vector-push
                    (vector-push callee-ir-mid (make-local-get 6))
                    (make-instr 24 0))
        caller (make-function-meta 0 0 caller-ir)
        callee (make-function-meta 7 0 callee-ir)
        functions (vector-push (vector-push (vector-new 2) caller) callee)
        target (make-target 1)
        native (emit-native-function-meta-bundle functions target)]
    (do
      (print (vector-length native))
      (print (vector-get native 242))
      (print (vector-get native 243))
      (print (vector-get native 244))
      (print (vector-get native 245))
      (print (vector-get native 246))
      (print (vector-get native 247))
      (print (vector-get native 248))
      (print (vector-get native 249))
      (print (vector-get native 250))
      (print (vector-get native 251))
      (print (vector-get native 252))
      (print (vector-get native 253))
      (print (vector-get native 254))
      (print (vector-get native 255))
      (print (vector-get native 256))
      (print (vector-get native 257))
      (print (vector-get native 258))
      (print (vector-get native 259))
      (print (vector-get native 260))
      (print (vector-get native 261))
      (print (vector-get native 262))
      (print (vector-get native 263))
      (print (vector-get native 264))
      (print (vector-get native 265))
      (print (vector-get native 266))
      (print (vector-get native 267))
      (print (vector-get native 268))
      (print (vector-get native 269))
      (print (vector-get native 270))
      (print (vector-get native 271))
      (print (vector-get native 272))
      (print (vector-get native 273))
      (print (vector-get native 274))
      (print (vector-get native 275))
      (print (vector-get native 276))
      (print (vector-get native 277))
      (print (vector-get native 278))
      (print (vector-get native 279))
      (print (vector-get native 280))
      (print (vector-get native 281))
      (print (vector-get native 282))
      (print (vector-get native 283))
      (print (vector-get native 284))
      (print (vector-get native 285))
      (print (vector-get native 286))
      (print (vector-get native 287))
      (print (vector-get native 288))
      (print (vector-get native 289))
      (print (vector-get native 290))
      (print (vector-get native 291))
      (print (vector-get native 292))
      (print (vector-get native 293))
      (print (vector-get native 294))
      (print (vector-get native 295))
      (print (vector-get native 296))
      (print (vector-get native 297))
      (print (vector-get native 298))
      (print (vector-get native 299))
      (print (vector-get native 300))
      (print (vector-get native 301))
      (print (vector-get native 302))
      (print (vector-get native 323))
      (print (vector-get native 324))
      (print (vector-get native 325))
      (print (vector-get native 330))
      (print (vector-get native 331))
      (print (vector-get native 332))
      (print (vector-get native 337))
      (print (vector-get native 338))
      (print (vector-get native 339))
      (print (vector-get native 344))
      (print (vector-get native 345))
      (print (vector-get native 346))
      (print (vector-get native 351))
      (print (vector-get native 352))
      (print (vector-get native 353))
      (print (vector-get native 358))
      (print (vector-get native 359))
      (print (vector-get native 360))
      (print (vector-get native 365))
      (print (vector-get native 366))
      (print (vector-get native 367))
      (print (vector-get native 368))
      (print (vector-get native 369))
      (print (vector-get native 370))
      (print (vector-get native 371))
      (print (vector-get native 465))
      (print (vector-get native 466))
      0)))"#,
    );
    let lines: Vec<&str> = output.trim().lines().collect();

    assert!(
        lines.len() >= 89,
        "x86 direct call seven-arg bundle bytes 出力が不足: {:?}",
        lines
    );
    assert_eq!(
        lines[0], "467",
        "x86_64 direct call seven-arg bundle payload は 467 bytes であるべき"
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
        "stack arg spill 先頭は mov [rsp], rax の 0x48"
    );
    assert_eq!(lines[9], "137", "stack arg spill 2 byte 目は 0x89");
    assert_eq!(lines[10], "4", "stack arg spill 3 byte 目は ModRM 0x04");
    assert_eq!(lines[11], "36", "stack arg spill 4 byte 目は SIB 0x24");
    assert_eq!(lines[12], "73", "arg5 move 先頭は mov r9, rcx の 0x49");
    assert_eq!(lines[13], "137", "arg5 move 2 byte 目は 0x89");
    assert_eq!(lines[14], "201", "arg5 move 3 byte 目は ModRM 0xC9");
    assert_eq!(
        lines[15], "76",
        "arg4 load 先頭は mov r8, [rbp-offset] の 0x4C"
    );
    assert_eq!(lines[16], "139", "arg4 load 2 byte 目は 0x8B");
    assert_eq!(lines[17], "133", "arg4 load 3 byte 目は ModRM 0x85");
    assert_eq!(lines[18], "248", "arg4 spill load offset byte0 は -8");
    assert_eq!(lines[19], "255", "arg4 spill load offset byte1 は 0xFF");
    assert_eq!(lines[20], "255", "arg4 spill load offset byte2 は 0xFF");
    assert_eq!(lines[21], "255", "arg4 spill load offset byte3 は 0xFF");
    assert_eq!(
        lines[22], "72",
        "arg3 load 先頭は mov rcx, [rbp-offset] の 0x48"
    );
    assert_eq!(lines[23], "139", "arg3 load 2 byte 目は 0x8B");
    assert_eq!(lines[24], "141", "arg3 load 3 byte 目は ModRM 0x8D");
    assert_eq!(lines[25], "240", "arg3 spill load offset byte0 は -16");
    assert_eq!(lines[26], "255", "arg3 spill load offset byte1 は 0xFF");
    assert_eq!(lines[27], "255", "arg3 spill load offset byte2 は 0xFF");
    assert_eq!(lines[28], "255", "arg3 spill load offset byte3 は 0xFF");
    assert_eq!(
        lines[29], "72",
        "arg2 load 先頭は mov rdx, [rbp-offset] の 0x48"
    );
    assert_eq!(lines[30], "139", "arg2 load 2 byte 目は 0x8B");
    assert_eq!(lines[31], "149", "arg2 load 3 byte 目は ModRM 0x95");
    assert_eq!(lines[32], "232", "arg2 spill load offset byte0 は -24");
    assert_eq!(lines[33], "255", "arg2 spill load offset byte1 は 0xFF");
    assert_eq!(lines[34], "255", "arg2 spill load offset byte2 は 0xFF");
    assert_eq!(lines[35], "255", "arg2 spill load offset byte3 は 0xFF");
    assert_eq!(
        lines[36], "72",
        "arg1 load 先頭は mov rsi, [rbp-offset] の 0x48"
    );
    assert_eq!(lines[37], "139", "arg1 load 2 byte 目は 0x8B");
    assert_eq!(lines[38], "181", "arg1 load 3 byte 目は ModRM 0xB5");
    assert_eq!(lines[39], "224", "arg1 spill load offset byte0 は -32");
    assert_eq!(lines[40], "255", "arg1 spill load offset byte1 は 0xFF");
    assert_eq!(lines[41], "255", "arg1 spill load offset byte2 は 0xFF");
    assert_eq!(lines[42], "255", "arg1 spill load offset byte3 は 0xFF");
    assert_eq!(
        lines[43], "72",
        "arg0 load 先頭は mov rdi, [rbp-offset] の 0x48"
    );
    assert_eq!(lines[44], "139", "arg0 load 2 byte 目は 0x8B");
    assert_eq!(lines[45], "189", "arg0 load 3 byte 目は ModRM 0xBD");
    assert_eq!(lines[46], "216", "arg0 spill load offset byte0 は -40");
    assert_eq!(lines[47], "255", "arg0 spill load offset byte1 は 0xFF");
    assert_eq!(lines[48], "255", "arg0 spill load offset byte2 は 0xFF");
    assert_eq!(lines[49], "255", "arg0 spill load offset byte3 は 0xFF");
    assert_eq!(lines[50], "232", "direct call は call rel32 opcode 0xE8");
    assert_eq!(lines[51], "16", "forward call offset の下位 byte は 16");
    assert_eq!(lines[52], "0", "forward call offset byte1 は 0");
    assert_eq!(lines[53], "0", "forward call offset byte2 は 0");
    assert_eq!(lines[54], "0", "forward call offset byte3 は 0");
    assert_eq!(
        lines[55], "72",
        "post-call stack restore 先頭は add rsp, 16 の 0x48"
    );
    assert_eq!(lines[56], "129", "post-call stack restore 2 byte 目は 0x81");
    assert_eq!(
        lines[57], "196",
        "post-call stack restore 3 byte 目は ModRM 0xC4"
    );
    assert_eq!(lines[58], "16", "post-call stack restore imm byte0 は 16");
    assert_eq!(lines[59], "0", "post-call stack restore imm byte1 は 0");
    assert_eq!(lines[60], "0", "post-call stack restore imm byte2 は 0");
    assert_eq!(lines[61], "0", "post-call stack restore imm byte3 は 0");
    assert_eq!(lines[62], "72", "callee param0 spill 先頭は 0x48");
    assert_eq!(lines[63], "137", "callee param0 spill 2 byte 目は 0x89");
    assert_eq!(
        lines[64], "189",
        "callee param0 spill 3 byte 目は ModRM 0xBD"
    );
    assert_eq!(lines[65], "72", "callee param1 spill 先頭は 0x48");
    assert_eq!(lines[66], "137", "callee param1 spill 2 byte 目は 0x89");
    assert_eq!(
        lines[67], "181",
        "callee param1 spill 3 byte 目は ModRM 0xB5"
    );
    assert_eq!(lines[68], "72", "callee param2 spill 先頭は 0x48");
    assert_eq!(lines[69], "137", "callee param2 spill 2 byte 目は 0x89");
    assert_eq!(
        lines[70], "149",
        "callee param2 spill 3 byte 目は ModRM 0x95"
    );
    assert_eq!(lines[71], "72", "callee param3 spill 先頭は 0x48");
    assert_eq!(lines[72], "137", "callee param3 spill 2 byte 目は 0x89");
    assert_eq!(
        lines[73], "141",
        "callee param3 spill 3 byte 目は ModRM 0x8D"
    );
    assert_eq!(
        lines[74], "76",
        "callee param4 spill 先頭は mov [rbp-offset], r8 の 0x4C"
    );
    assert_eq!(lines[75], "137", "callee param4 spill 2 byte 目は 0x89");
    assert_eq!(
        lines[76], "133",
        "callee param4 spill 3 byte 目は ModRM 0x85"
    );
    assert_eq!(
        lines[77], "76",
        "callee param5 spill 先頭は mov [rbp-offset], r9 の 0x4C"
    );
    assert_eq!(lines[78], "137", "callee param5 spill 2 byte 目は 0x89");
    assert_eq!(
        lines[79], "141",
        "callee param5 spill 3 byte 目は ModRM 0x8D"
    );
    assert_eq!(
        lines[80], "72",
        "callee stack arg load 先頭は mov rax, [rbp+16] の 0x48"
    );
    assert_eq!(lines[81], "139", "callee stack arg load 2 byte 目は 0x8B");
    assert_eq!(
        lines[82], "69",
        "callee stack arg load 3 byte 目は ModRM 0x45"
    );
    assert_eq!(lines[83], "16", "callee stack arg load disp8 は 16");
    assert_eq!(lines[84], "72", "callee param6 spill 先頭は 0x48");
    assert_eq!(lines[85], "137", "callee param6 spill 2 byte 目は 0x89");
    assert_eq!(
        lines[86], "133",
        "callee param6 spill 3 byte 目は ModRM 0x85"
    );
    assert_eq!(lines[87], "93", "payload 末尾手前は pop rbp");
    assert_eq!(lines[88], "195", "payload 末尾は ret");
}
