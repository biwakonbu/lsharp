
/// NATIVE-REAL-08f9: x86_64 の monolithic emit は後続関数の helper call でも絶対 helper offset を指すこと
#[test]
#[ignore]
fn test_native_codegen_x86_monolithic_emit_localizes_helper_offset_for_nonzero_caller() {
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

(defn print-bytes [bytes idx n]
  (if (>= idx n)
    0
    (do
      (print (vector-get bytes idx))
      (print-bytes bytes (+ idx 1) n))))

(defn main []
  (let [filler-ir (vector-push (vector-new 1) (make-instr 1 7))
        filler (make-function-meta 0 0 filler-ir)
        caller-ir (vector-push (vector-new 1) (make-instr 67 0))
        caller (make-function-meta 0 0 caller-ir)
        functions (vector-push (vector-push (vector-new 2) filler) caller)
        starts (collect-callable-function-starts-x86 functions 0)
        helper-offset (callable-user-total-size-x86 functions 0)
        target (make-target 1)
        native (emit-native-function-meta-bundle functions target)]
    (do
      (print (vector-get starts 1))
      (print helper-offset)
      (print-bytes native 0 (vector-length native))
      0)))"#,
    );
    let mut lines = output.trim().lines();
    let caller_start = lines
        .next()
        .expect("caller start output")
        .parse::<usize>()
        .expect("caller start parse");
    let helper_offset = lines
        .next()
        .expect("helper offset output")
        .parse::<isize>()
        .expect("helper offset parse");
    let bytes = lines
        .map(|line| {
            line.parse::<u8>()
                .unwrap_or_else(|_| panic!("x86 monolithic helper call byte parse 失敗: {line}"))
        })
        .collect::<Vec<_>>();
    let call_offset = (caller_start..bytes.len())
        .find(|idx| bytes[*idx] == 0xe8)
        .unwrap_or_else(|| panic!("x86 monolithic helper call が見つからない: bytes={bytes:?}"));
    let rel = i32::from_le_bytes([
        bytes[call_offset + 1],
        bytes[call_offset + 2],
        bytes[call_offset + 3],
        bytes[call_offset + 4],
    ]);
    let target = call_offset as isize + 5 + rel as isize;

    assert_eq!(
        target, helper_offset,
        "x86 monolithic helper call target は code 全体の helper offset を指すべき: call_offset={call_offset} rel={rel} target={target} helper_offset={helper_offset} bytes={bytes:?}"
    );
}

/// NATIVE-REAL-08g: x86_64 で 2 引数 direct call bundle が arg move + rel32 call bytes を持つこと
#[test]
#[ignore]
fn test_native_codegen_emits_x86_direct_call_two_arg_bundle_bytes() {
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
                      (vector-push (vector-new 3) (make-instr 3 40))
                      (make-instr 3 2))
                    (make-call 1))
        callee-ir (vector-push
                    (vector-push
                      (vector-push (vector-new 3) (make-local-get 0))
                      (make-local-get 1))
                      (make-instr 24 0))
        caller (make-function-meta 0 0 caller-ir)
        callee (make-function-meta 2 0 callee-ir)
        functions (vector-push (vector-push (vector-new 2) caller) callee)
        target (make-target 1)
        native (emit-native-function-meta-bundle functions target)]
    (do
      (print (vector-length native))
      (print (vector-get native 20))
      (print (vector-get native 21))
      (print (vector-get native 22))
      (print (vector-get native 23))
      (print (vector-get native 24))
      (print (vector-get native 25))
      (print (vector-get native 26))
      (print (vector-get native 27))
      (print (vector-get native 28))
      (print (vector-get native 29))
      (print (vector-get native 30))
      (print (vector-get native 44))
      (print (vector-get native 45))
      (print (vector-get native 46))
      (print (vector-get native 51))
      (print (vector-get native 52))
      (print (vector-get native 53))
      (print (vector-get native 87))
      (print (vector-get native 88))
      0)))"#,
    );
    let lines: Vec<&str> = output.trim().lines().collect();

    assert!(
        lines.len() >= 20,
        "x86 direct call two-arg bundle bytes 出力が不足: {:?}",
        lines
    );
    assert_eq!(
        lines[0], "89",
        "x86_64 direct call two-arg bundle payload は 89 bytes であるべき"
    );
    assert_eq!(lines[1], "72", "arg1 move 先頭は mov rsi, rax の 0x48");
    assert_eq!(lines[2], "137", "arg1 move 2 byte 目は 0x89");
    assert_eq!(lines[3], "198", "arg1 move 3 byte 目は ModRM 0xC6");
    assert_eq!(lines[4], "72", "arg0 move 先頭は mov rdi, rcx の 0x48");
    assert_eq!(lines[5], "137", "arg0 move 2 byte 目は 0x89");
    assert_eq!(lines[6], "207", "arg0 move 3 byte 目は ModRM 0xCF");
    assert_eq!(lines[7], "232", "direct call は call rel32 opcode 0xE8");
    assert_eq!(lines[8], "2", "forward call offset の下位 byte は 2");
    assert_eq!(lines[9], "0", "forward call offset byte1 は 0");
    assert_eq!(lines[10], "0", "forward call offset byte2 は 0");
    assert_eq!(lines[11], "0", "forward call offset byte3 は 0");
    assert_eq!(
        lines[12], "72",
        "callee param0 spill は mov [rbp-offset], rdi の 0x48"
    );
    assert_eq!(lines[13], "137", "callee param0 spill 2 byte 目は 0x89");
    assert_eq!(
        lines[14], "189",
        "callee param0 spill 3 byte 目は ModRM 0xBD"
    );
    assert_eq!(
        lines[15], "72",
        "callee param1 spill は mov [rbp-offset], rsi の 0x48"
    );
    assert_eq!(lines[16], "137", "callee param1 spill 2 byte 目は 0x89");
    assert_eq!(
        lines[17], "181",
        "callee param1 spill 3 byte 目は ModRM 0xB5"
    );
    assert_eq!(lines[18], "93", "payload 末尾手前は pop rbp");
    assert_eq!(lines[19], "195", "payload 末尾は ret");
}

/// NATIVE-REAL-08h: x86_64 で 3 引数 direct call bundle が spill load + arg moves + rel32 call bytes を持つこと
#[test]
#[ignore]
fn test_native_codegen_emits_x86_direct_call_three_arg_bundle_bytes() {
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
                        (vector-push (vector-new 4) (make-instr 3 40))
                        (make-instr 3 2))
                      (make-instr 3 5))
                    (make-call 1))
        callee-ir (vector-push
                    (vector-push
                      (vector-push
                        (vector-push
                          (vector-push (vector-new 5) (make-local-get 0))
                          (make-local-get 1))
                        (make-instr 24 0))
                      (make-local-get 2))
                    (make-instr 24 0))
        caller (make-function-meta 0 0 caller-ir)
        callee (make-function-meta 3 0 callee-ir)
        functions (vector-push (vector-push (vector-new 2) caller) callee)
        target (make-target 1)
        native (emit-native-function-meta-bundle functions target)]
    (do
      (print (vector-length native))
      (print (vector-get native 42))
      (print (vector-get native 43))
      (print (vector-get native 44))
      (print (vector-get native 45))
      (print (vector-get native 46))
      (print (vector-get native 47))
      (print (vector-get native 48))
      (print (vector-get native 49))
      (print (vector-get native 50))
      (print (vector-get native 51))
      (print (vector-get native 52))
      (print (vector-get native 53))
      (print (vector-get native 54))
      (print (vector-get native 55))
      (print (vector-get native 56))
      (print (vector-get native 57))
      (print (vector-get native 58))
      (print (vector-get native 59))
      (print (vector-get native 80))
      (print (vector-get native 81))
      (print (vector-get native 82))
      (print (vector-get native 87))
      (print (vector-get native 88))
      (print (vector-get native 89))
      (print (vector-get native 94))
      (print (vector-get native 95))
      (print (vector-get native 96))
      (print (vector-get native 142))
      (print (vector-get native 143))
      0)))"#,
    );
    let lines: Vec<&str> = output.trim().lines().collect();

    assert!(
        lines.len() >= 30,
        "x86 direct call three-arg bundle bytes 出力が不足: {:?}",
        lines
    );
    assert_eq!(
        lines[0], "144",
        "x86_64 direct call three-arg bundle payload は 144 bytes であるべき"
    );
    assert_eq!(lines[1], "72", "arg2 move 先頭は mov rdx, rax の 0x48");
    assert_eq!(lines[2], "137", "arg2 move 2 byte 目は 0x89");
    assert_eq!(lines[3], "194", "arg2 move 3 byte 目は ModRM 0xC2");
    assert_eq!(lines[4], "72", "arg1 move 先頭は mov rsi, rcx の 0x48");
    assert_eq!(lines[5], "137", "arg1 move 2 byte 目は 0x89");
    assert_eq!(lines[6], "206", "arg1 move 3 byte 目は ModRM 0xCE");
    assert_eq!(
        lines[7], "72",
        "arg0 load 先頭は mov rdi, [rbp-offset] の 0x48"
    );
    assert_eq!(lines[8], "139", "arg0 load 2 byte 目は 0x8B");
    assert_eq!(lines[9], "189", "arg0 load 3 byte 目は ModRM 0xBD");
    assert_eq!(lines[10], "248", "arg0 spill load offset byte0 は -8");
    assert_eq!(lines[11], "255", "arg0 spill load offset byte1 は 0xFF");
    assert_eq!(lines[12], "255", "arg0 spill load offset byte2 は 0xFF");
    assert_eq!(lines[13], "255", "arg0 spill load offset byte3 は 0xFF");
    assert_eq!(lines[14], "232", "direct call は call rel32 opcode 0xE8");
    assert_eq!(lines[15], "9", "forward call offset の下位 byte は 9");
    assert_eq!(lines[16], "0", "forward call offset byte1 は 0");
    assert_eq!(lines[17], "0", "forward call offset byte2 は 0");
    assert_eq!(lines[18], "0", "forward call offset byte3 は 0");
    assert_eq!(lines[19], "72", "callee param0 spill 先頭は 0x48");
    assert_eq!(lines[20], "137", "callee param0 spill 2 byte 目は 0x89");
    assert_eq!(
        lines[21], "189",
        "callee param0 spill 3 byte 目は ModRM 0xBD"
    );
    assert_eq!(lines[22], "72", "callee param1 spill 先頭は 0x48");
    assert_eq!(lines[23], "137", "callee param1 spill 2 byte 目は 0x89");
    assert_eq!(
        lines[24], "181",
        "callee param1 spill 3 byte 目は ModRM 0xB5"
    );
    assert_eq!(lines[25], "72", "callee param2 spill 先頭は 0x48");
    assert_eq!(lines[26], "137", "callee param2 spill 2 byte 目は 0x89");
    assert_eq!(
        lines[27], "149",
        "callee param2 spill 3 byte 目は ModRM 0x95"
    );
    assert_eq!(lines[28], "93", "payload 末尾手前は pop rbp");
    assert_eq!(lines[29], "195", "payload 末尾は ret");
}

/// NATIVE-REAL-08i: x86_64 の 3-value window で drop;drop が spilled previous を復元すること
#[test]
#[ignore]
fn test_native_codegen_emits_x86_three_value_double_drop_bytes() {
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
  (let [ir (vector-push
             (vector-push
               (vector-push
                 (vector-push
                   (vector-push (vector-new 5) (make-instr 3 7))
                   (make-instr 3 40))
                 (make-instr 3 2))
               (make-instr 44 0))
             (make-instr 44 0))
        func (make-function-meta 0 0 ir)
        functions (vector-push (vector-new 1) func)
        target (make-target 1)
        native (emit-native-function-meta-bundle functions target)]
    (do
      (print (vector-length native))
      (print (vector-get native 27))
      (print (vector-get native 28))
      (print (vector-get native 29))
      (print (vector-get native 30))
      (print (vector-get native 31))
      (print (vector-get native 32))
      (print (vector-get native 33))
      (print (vector-get native 34))
      (print (vector-get native 35))
      (print (vector-get native 36))
      (print (vector-get native 37))
      (print (vector-get native 38))
      (print (vector-get native 39))
      (print (vector-get native 40))
      (print (vector-get native 41))
      (print (vector-get native 42))
      (print (vector-get native 43))
      (print (vector-get native 44))
      (print (vector-get native 45))
      (print (vector-get native 46))
      (print (vector-get native 47))
      (print (vector-get native 48))
      (print (vector-get native 49))
      (print (vector-get native 50))
      (print (vector-get native 51))
      (print (vector-get native 52))
      (print (vector-get native 53))
      (print (vector-get native 54))
      (print (vector-get native 62))
      (print (vector-get native 63))
      0)))"#,
    );
    let lines: Vec<&str> = output.trim().lines().collect();

    assert!(
        lines.len() >= 31,
        "x86 three-value double-drop bytes 出力が不足: {:?}",
        lines
    );
    assert_eq!(
        lines[0], "64",
        "x86_64 three-value double-drop payload は 64 bytes であるべき"
    );
    assert_eq!(lines[1], "72", "third push spill store 先頭は 0x48");
    assert_eq!(lines[2], "137", "third push spill store 2 byte 目は 0x89");
    assert_eq!(
        lines[3], "141",
        "third push spill store 3 byte 目は ModRM 0x8D"
    );
    assert_eq!(lines[4], "248", "spill store offset byte0 は -8");
    assert_eq!(lines[5], "255", "spill store offset byte1 は 0xFF");
    assert_eq!(lines[6], "255", "spill store offset byte2 は 0xFF");
    assert_eq!(lines[7], "255", "spill store offset byte3 は 0xFF");
    assert_eq!(lines[8], "72", "third push で current->previous の 0x48");
    assert_eq!(
        lines[9], "137",
        "third push で current->previous 2 byte 目は 0x89"
    );
    assert_eq!(
        lines[10], "193",
        "third push で current->previous 3 byte 目は ModRM 0xC1"
    );
    assert_eq!(lines[11], "184", "third push の mov eax, imm32 opcode");
    assert_eq!(lines[12], "2", "third push 即値の下位 byte は 2");
    assert_eq!(lines[13], "0", "third push 即値 byte1 は 0");
    assert_eq!(lines[14], "0", "third push 即値 byte2 は 0");
    assert_eq!(lines[15], "0", "third push 即値 byte3 は 0");
    assert_eq!(lines[16], "72", "first drop の mov rax, rcx 先頭は 0x48");
    assert_eq!(lines[17], "137", "first drop 2 byte 目は 0x89");
    assert_eq!(lines[18], "200", "first drop 3 byte 目は ModRM 0xC8");
    assert_eq!(lines[19], "72", "first drop restore spill 先頭は 0x48");
    assert_eq!(
        lines[20], "139",
        "first drop restore spill 2 byte 目は 0x8B"
    );
    assert_eq!(
        lines[21], "141",
        "first drop restore spill 3 byte 目は ModRM 0x8D"
    );
    assert_eq!(
        lines[22], "248",
        "first drop restore spill offset byte0 は -8"
    );
    assert_eq!(
        lines[23], "255",
        "first drop restore spill offset byte1 は 0xFF"
    );
    assert_eq!(
        lines[24], "255",
        "first drop restore spill offset byte2 は 0xFF"
    );
    assert_eq!(
        lines[25], "255",
        "first drop restore spill offset byte3 は 0xFF"
    );
    assert_eq!(lines[26], "72", "second drop の mov rax, rcx 先頭は 0x48");
    assert_eq!(lines[27], "137", "second drop 2 byte 目は 0x89");
    assert_eq!(lines[28], "200", "second drop 3 byte 目は ModRM 0xC8");
    assert_eq!(lines[29], "93", "payload 末尾手前は pop rbp");
    assert_eq!(lines[30], "195", "payload 末尾は ret");
}

/// NATIVE-REAL-08ia: x86_64 の 24-value drop helper が spill21 まで low->high に詰め直すこと。
#[test]
#[ignore]
fn test_native_codegen_emits_x86_deep_drop_helper_bytes() {
    let output = run_native_codegen_harness(
        r#"(module Main)
(import NativeCodegen)

(defn main []
  (let [native (emit-drop-bundle-x86 0 24)]
    (do
      (print (vector-length native))
      (print (vector-get native 0))
      (print (vector-get native 1))
      (print (vector-get native 2))
      (print (vector-get native 3))
      (print (vector-get native 4))
      (print (vector-get native 5))
      (print (vector-get native 6))
      (print (vector-get native 7))
      (print (vector-get native 8))
      (print (vector-get native 9))
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
      (print (vector-get native 303))
      0)))"#,
    );
    let lines: Vec<&str> = output.trim().lines().collect();

    assert!(
        lines.len() >= 25,
        "x86 deep-drop helper bytes 出力が不足: {:?}",
        lines
    );
    assert_eq!(
        lines[0], "304",
        "24-value deep drop helper は 304 bytes であるべき"
    );
    assert_eq!(lines[1], "72", "deep drop 先頭は mov rax, rcx の 0x48");
    assert_eq!(lines[2], "137", "deep drop 2 byte 目は 0x89");
    assert_eq!(lines[3], "200", "deep drop 3 byte 目は ModRM 0xC8");
    assert_eq!(lines[4], "72", "deep drop restore spill0 先頭は 0x48");
    assert_eq!(lines[5], "139", "deep drop restore spill0 2 byte 目は 0x8B");
    assert_eq!(
        lines[6], "141",
        "deep drop restore spill0 3 byte 目は ModRM 0x8D"
    );
    assert_eq!(
        lines[7], "248",
        "deep drop restore spill0 offset byte0 は -8"
    );
    assert_eq!(
        lines[8], "255",
        "deep drop restore spill0 offset byte1 は 0xFF"
    );
    assert_eq!(
        lines[9], "255",
        "deep drop restore spill0 offset byte2 は 0xFF"
    );
    assert_eq!(
        lines[10], "255",
        "deep drop restore spill0 offset byte3 は 0xFF"
    );
    assert_eq!(lines[11], "72", "末尾 shift load 先頭は 0x48");
    assert_eq!(lines[12], "139", "末尾 shift load 2 byte 目は 0x8B");
    assert_eq!(lines[13], "181", "末尾 shift load 3 byte 目は ModRM 0xB5");
    assert_eq!(lines[14], "80", "末尾 shift load offset byte0 は -176");
    assert_eq!(lines[15], "255", "末尾 shift load offset byte1 は 0xFF");
    assert_eq!(lines[16], "255", "末尾 shift load offset byte2 は 0xFF");
    assert_eq!(lines[17], "255", "末尾 shift load offset byte3 は 0xFF");
    assert_eq!(lines[18], "72", "末尾 shift store 先頭は 0x48");
    assert_eq!(lines[19], "137", "末尾 shift store 2 byte 目は 0x89");
    assert_eq!(lines[20], "181", "末尾 shift store 3 byte 目は ModRM 0xB5");
    assert_eq!(lines[21], "88", "末尾 shift store offset byte0 は -168");
    assert_eq!(lines[22], "255", "末尾 shift store offset byte1 は 0xFF");
    assert_eq!(lines[23], "255", "末尾 shift store offset byte2 は 0xFF");
    assert_eq!(lines[24], "255", "末尾 shift store offset byte3 は 0xFF");
}

/// NATIVE-REAL-08j: x86_64 で 4 引数 direct call bundle が 2-spill load + arg moves + rel32 call bytes を持つこと
#[test]
#[ignore]
fn test_native_codegen_emits_x86_direct_call_four_arg_bundle_bytes() {
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
                          (vector-push (vector-new 5) (make-instr 3 40))
                          (make-instr 3 2))
                        (make-instr 3 5))
                      (make-instr 3 7))
                    (make-call 1))
        callee-ir (vector-push
                    (vector-push
                      (vector-push
                        (vector-push
                          (vector-push
                            (vector-push
                              (vector-push (vector-new 7) (make-local-get 0))
                              (make-local-get 1))
                            (make-instr 24 0))
                          (make-local-get 2))
                        (make-instr 24 0))
                      (make-local-get 3))
                    (make-instr 24 0))
        caller (make-function-meta 0 0 caller-ir)
        callee (make-function-meta 4 0 callee-ir)
        functions (vector-push (vector-push (vector-new 2) caller) callee)
        target (make-target 1)
        native (emit-native-function-meta-bundle functions target)]
    (do
      (print (vector-length native))
      (print (vector-get native 71))
      (print (vector-get native 72))
      (print (vector-get native 73))
      (print (vector-get native 74))
      (print (vector-get native 75))
      (print (vector-get native 76))
      (print (vector-get native 77))
      (print (vector-get native 78))
      (print (vector-get native 79))
      (print (vector-get native 80))
      (print (vector-get native 81))
      (print (vector-get native 82))
      (print (vector-get native 83))
      (print (vector-get native 84))
      (print (vector-get native 85))
      (print (vector-get native 86))
      (print (vector-get native 87))
      (print (vector-get native 88))
      (print (vector-get native 89))
      (print (vector-get native 90))
      (print (vector-get native 91))
      (print (vector-get native 92))
      (print (vector-get native 93))
      (print (vector-get native 94))
      (print (vector-get native 95))
      (print (vector-get native 116))
      (print (vector-get native 117))
      (print (vector-get native 118))
      (print (vector-get native 123))
      (print (vector-get native 124))
      (print (vector-get native 125))
      (print (vector-get native 130))
      (print (vector-get native 131))
      (print (vector-get native 132))
      (print (vector-get native 137))
      (print (vector-get native 138))
      (print (vector-get native 139))
      (print (vector-get native 197))
      (print (vector-get native 198))
      0)))"#,
    );
    let lines: Vec<&str> = output.trim().lines().collect();

    assert!(
        lines.len() >= 40,
        "x86 direct call four-arg bundle bytes 出力が不足: {:?}",
        lines
    );
    assert_eq!(
        lines[0], "199",
        "x86_64 direct call four-arg bundle payload は 199 bytes であるべき"
    );
    assert_eq!(lines[1], "72", "arg2 move 先頭は mov rdx, rcx の 0x48");
    assert_eq!(lines[2], "137", "arg2 move 2 byte 目は 0x89");
    assert_eq!(lines[3], "202", "arg2 move 3 byte 目は ModRM 0xCA");
    assert_eq!(
        lines[4], "72",
        "arg1 load 先頭は mov rsi, [rbp-offset] の 0x48"
    );
    assert_eq!(lines[5], "139", "arg1 load 2 byte 目は 0x8B");
    assert_eq!(lines[6], "181", "arg1 load 3 byte 目は ModRM 0xB5");
    assert_eq!(lines[7], "248", "arg1 spill load offset byte0 は -8");
    assert_eq!(lines[8], "255", "arg1 spill load offset byte1 は 0xFF");
    assert_eq!(lines[9], "255", "arg1 spill load offset byte2 は 0xFF");
    assert_eq!(lines[10], "255", "arg1 spill load offset byte3 は 0xFF");
    assert_eq!(
        lines[11], "72",
        "arg0 load 先頭は mov rdi, [rbp-offset] の 0x48"
    );
    assert_eq!(lines[12], "139", "arg0 load 2 byte 目は 0x8B");
    assert_eq!(lines[13], "189", "arg0 load 3 byte 目は ModRM 0xBD");
    assert_eq!(lines[14], "240", "arg0 spill load offset byte0 は -16");
    assert_eq!(lines[15], "255", "arg0 spill load offset byte1 は 0xFF");
    assert_eq!(lines[16], "255", "arg0 spill load offset byte2 は 0xFF");
    assert_eq!(lines[17], "255", "arg0 spill load offset byte3 は 0xFF");
    assert_eq!(lines[18], "72", "arg3 move 先頭は mov rcx, rax の 0x48");
    assert_eq!(lines[19], "137", "arg3 move 2 byte 目は 0x89");
    assert_eq!(lines[20], "193", "arg3 move 3 byte 目は ModRM 0xC1");
    assert_eq!(lines[21], "232", "direct call は call rel32 opcode 0xE8");
    assert_eq!(lines[22], "9", "forward call offset の下位 byte は 9");
    assert_eq!(lines[23], "0", "forward call offset byte1 は 0");
    assert_eq!(lines[24], "0", "forward call offset byte2 は 0");
    assert_eq!(lines[25], "0", "forward call offset byte3 は 0");
    assert_eq!(lines[26], "72", "callee param0 spill 先頭は 0x48");
    assert_eq!(lines[27], "137", "callee param0 spill 2 byte 目は 0x89");
    assert_eq!(
        lines[28], "189",
        "callee param0 spill 3 byte 目は ModRM 0xBD"
    );
    assert_eq!(lines[29], "72", "callee param1 spill 先頭は 0x48");
    assert_eq!(lines[30], "137", "callee param1 spill 2 byte 目は 0x89");
    assert_eq!(
        lines[31], "181",
        "callee param1 spill 3 byte 目は ModRM 0xB5"
    );
    assert_eq!(lines[32], "72", "callee param2 spill 先頭は 0x48");
    assert_eq!(lines[33], "137", "callee param2 spill 2 byte 目は 0x89");
    assert_eq!(
        lines[34], "149",
        "callee param2 spill 3 byte 目は ModRM 0x95"
    );
    assert_eq!(lines[35], "72", "callee param3 spill 先頭は 0x48");
    assert_eq!(lines[36], "137", "callee param3 spill 2 byte 目は 0x89");
    assert_eq!(
        lines[37], "141",
        "callee param3 spill 3 byte 目は ModRM 0x8D"
    );
    assert_eq!(lines[38], "93", "payload 末尾手前は pop rbp");
    assert_eq!(lines[39], "195", "payload 末尾は ret");
}
