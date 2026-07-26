
/// NATIVE-REAL-08c9: x86_64 で plain if/else/end が dedicated bytes を持つこと。
#[test]
#[ignore]
fn test_native_codegen_emits_x86_if_else_bytes() {
    assert_x86_plain_control_tail(
        "if/else",
        &[(3, 1), (41, 0), (3, 42), (79, 0), (3, 7), (43, 0)],
        25,
        &[
            0, 0, 72, 137, 193, 184, 42, 0, 0, 0, 233, 8, 0, 0, 0, 72, 137, 193, 184, 7, 0, 0, 0,
            93, 195,
        ],
    );
}

fn assert_x86_i64_compare_tail(name: &str, opcode: u32, setcc_opcode: u32) {
    let output = run_native_codegen_harness(&format!(
        r#"(module Main)
(import NativeTarget)
(import NativeCodegen)

(defn make-instr [opcode operand]
  (vector-push (vector-push (vector-new 2) opcode) operand))

(defn main []
  (let [instr1 (make-instr 1 42)
        instr2 (make-instr 11 0)
        instr3 (make-instr 1 2)
        instr4 (make-instr 11 1)
        instr5 (make-instr 10 0)
        instr6 (make-instr 10 1)
        instr7 (make-instr {opcode} 0)
        ir (vector-push
             (vector-push
               (vector-push
                 (vector-push
                   (vector-push
                     (vector-push
                       (vector-push (vector-new 7) instr1)
                       instr2)
                     instr3)
                   instr4)
                 instr5)
               instr6)
             instr7)
        target (make-target 1)
        native (emit-native ir target)
        n (vector-length native)]
    (do
      (print n)
      (print (vector-get native (- n 18)))
      (print (vector-get native (- n 17)))
      (print (vector-get native (- n 16)))
      (print (vector-get native (- n 15)))
      (print (vector-get native (- n 14)))
      (print (vector-get native (- n 13)))
      (print (vector-get native (- n 12)))
      (print (vector-get native (- n 11)))
      (print (vector-get native (- n 10)))
      (print (vector-get native (- n 9)))
      (print (vector-get native (- n 8)))
      (print (vector-get native (- n 7)))
      (print (vector-get native (- n 6)))
      (print (vector-get native (- n 5)))
      (print (vector-get native (- n 4)))
      (print (vector-get native (- n 3)))
      (print (vector-get native (- n 2)))
      (print (vector-get native (- n 1)))
      0)))"#,
    ));
    let values: Vec<u32> = output
        .trim()
        .lines()
        .map(|line| {
            line.parse::<u32>()
                .unwrap_or_else(|_| panic!("{name}: 数値出力であるべきだが `{line}` を得た"))
        })
        .collect();

    assert!(
        values.len() >= 19,
        "{name}: compare tail 出力が不足: {values:?}"
    );
    assert!(
        values[0] >= 18,
        "{name}: payload 長が短すぎるため compare tail を検査できない: {values:?}"
    );
    assert_eq!(
        &values[1..19],
        &[
            72,
            57,
            193,
            15,
            setcc_opcode,
            192,
            15,
            182,
            192,
            72,
            129,
            196,
            16,
            0,
            0,
            0,
            93,
            195,
        ],
        "{name}: x86 compare tail は cmp + setcc + movzx + add rsp,16 + epilogue であるべき"
    );
}

/// NATIVE-REAL-08c2: x86_64 で i64 compare 群が cmp + setcc + movzx bytes を持つこと
#[test]
#[ignore]
fn test_native_codegen_emits_x86_i64_compare_bytes() {
    for (name, opcode, setcc_opcode) in [
        ("i64.eq", 30, 148),
        ("i64.ne", 31, 149),
        ("i64.lt_s", 32, 156),
        ("i64.gt_s", 33, 159),
        ("i64.le_s", 34, 158),
        ("i64.ge_s", 35, 157),
    ] {
        assert_x86_i64_compare_tail(name, opcode, setcc_opcode);
    }
}

/// NATIVE-REAL-08d: x86_64 で direct call bundle が rel32 call bytes を持つこと
#[test]
#[ignore]
fn test_native_codegen_emits_x86_direct_call_bundle_bytes() {
    let output = run_native_codegen_harness(
        r#"(module Main)
(import NativeTarget)
(import NativeCodegen)
(import IR.IR)

(defn main []
  (let [caller-ir (vector-push (vector-new 1) (make-call 1))
        callee-ir (vector-push (vector-new 1) (make-instr 3 42))
        functions (vector-push (vector-push (vector-new 2) caller-ir) callee-ir)
        target (make-target 1)
        native (emit-native-bundle functions target)]
    (do
      (print (vector-length native))
      (print (vector-get native 4))
      (print (vector-get native 5))
      (print (vector-get native 6))
      (print (vector-get native 7))
      (print (vector-get native 8))
      (print (vector-get native 23))
      (print (vector-get native 24))
      0)))"#,
    );
    let lines: Vec<&str> = output.trim().lines().collect();

    assert!(
        lines.len() >= 8,
        "x86 direct call bundle bytes 出力が不足: {:?}",
        lines
    );
    assert_eq!(
        lines[0], "25",
        "x86_64 direct call bundle payload は 25 bytes であるべき"
    );
    assert_eq!(lines[1], "232", "direct call は call rel32 opcode 0xE8");
    assert_eq!(lines[2], "2", "forward call offset の下位 byte は 2");
    assert_eq!(lines[3], "0", "forward call offset byte1 は 0");
    assert_eq!(lines[4], "0", "forward call offset byte2 は 0");
    assert_eq!(lines[5], "0", "forward call offset byte3 は 0");
    assert_eq!(lines[6], "93", "callee epilogue 先頭は pop rbp");
    assert_eq!(lines[7], "195", "callee epilogue 末尾は ret");
}

/// NATIVE-REAL-08e: AArch64 で direct call bundle が BL + callee bytes を持つこと
#[test]
#[ignore]
fn test_native_codegen_emits_aarch64_direct_call_bundle_bytes() {
    let output = run_native_codegen_harness(
        r#"(module Main)
(import NativeTarget)
(import NativeCodegen)
(import IR.IR)

(defn main []
  (let [caller-ir (vector-push (vector-new 1) (make-call 1))
        callee-ir (vector-push (vector-new 1) (make-instr 3 42))
        functions (vector-push (vector-push (vector-new 2) caller-ir) callee-ir)
        target (make-target 2)
        native (emit-native-bundle functions target)]
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
      (print (vector-get native 24))
      (print (vector-get native 25))
      (print (vector-get native 26))
      (print (vector-get native 27))
      0)))"#,
    );
    let lines: Vec<&str> = output.trim().lines().collect();

    assert!(
        lines.len() >= 13,
        "aarch64 direct call bundle bytes 出力が不足: {:?}",
        lines
    );
    assert_eq!(
        lines[0], "28",
        "aarch64 direct call bundle payload は 28 bytes であるべき"
    );
    assert_eq!(
        lines[1], "253",
        "direct call bundle 先頭は save fp/lr byte 0"
    );
    assert_eq!(
        lines[2], "123",
        "direct call bundle 先頭は save fp/lr byte 1"
    );
    assert_eq!(
        lines[3], "191",
        "direct call bundle 先頭は save fp/lr byte 2"
    );
    assert_eq!(
        lines[4], "169",
        "direct call bundle 先頭は save fp/lr byte 3"
    );
    assert_eq!(lines[5], "3", "direct call bundle の BL byte 0 は 3");
    assert_eq!(lines[6], "0", "direct call bundle の BL byte 1 は 0");
    assert_eq!(lines[7], "0", "direct call bundle の BL byte 2 は 0");
    assert_eq!(lines[8], "148", "direct call bundle の BL byte 3 は 148");
    assert_eq!(lines[9], "192", "callee epilogue 先頭は RET byte 0");
    assert_eq!(lines[10], "3", "callee epilogue 2 byte 目は RET byte 1");
    assert_eq!(lines[11], "95", "callee epilogue 3 byte 目は RET byte 2");
    assert_eq!(lines[12], "214", "callee epilogue 末尾は RET byte 3");
}

/// NATIVE-REAL-08f: x86_64 で 1 引数 direct call bundle が arg move + rel32 call bytes を持つこと
#[test]
#[ignore]
fn test_native_codegen_emits_x86_direct_call_arg_bundle_bytes() {
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
                    (vector-push (vector-new 2) (make-instr 1 42))
                    (make-call 1))
        callee-ir (vector-push (vector-new 1) (make-local-get 0))
        caller (make-function-meta 0 0 caller-ir)
        callee (make-function-meta 1 0 callee-ir)
        functions (vector-push (vector-push (vector-new 2) caller) callee)
        target (make-target 1)
        native (emit-native-function-meta-bundle functions target)]
    (do
      (print (vector-length native))
      (print (vector-get native 14))
      (print (vector-get native 15))
      (print (vector-get native 16))
      (print (vector-get native 17))
      (print (vector-get native 18))
      (print (vector-get native 19))
      (print (vector-get native 20))
      (print (vector-get native 21))
      (print (vector-get native 22))
      (print (vector-get native 23))
      (print (vector-get native 37))
      (print (vector-get native 38))
      (print (vector-get native 39))
      (print (vector-get native 61))
      (print (vector-get native 62))
      0)))"#,
    );
    let lines: Vec<&str> = output.trim().lines().collect();

    assert!(
        lines.len() >= 16,
        "x86 direct call arg bundle bytes 出力が不足: {:?}",
        lines
    );
    assert_eq!(
        lines[0], "63",
        "x86_64 direct call arg bundle payload は 63 bytes であるべき"
    );
    assert_eq!(lines[1], "72", "arg move 先頭は mov rdi, rax の 0x48");
    assert_eq!(lines[2], "137", "arg move 2 byte 目は 0x89");
    assert_eq!(lines[3], "199", "arg move 3 byte 目は ModRM 0xC7");
    assert_eq!(
        lines[4], "81",
        "1 引数 call 前に previous-value 用 rcx を push する"
    );
    assert_eq!(lines[5], "232", "direct call は call rel32 opcode 0xE8");
    assert_eq!(lines[6], "3", "forward call offset の下位 byte は 3");
    assert_eq!(lines[7], "0", "forward call offset byte1 は 0");
    assert_eq!(lines[8], "0", "forward call offset byte2 は 0");
    assert_eq!(lines[9], "0", "forward call offset byte3 は 0");
    assert_eq!(
        lines[10], "89",
        "1 引数 call 後に previous-value 用 rcx を pop する"
    );
    assert_eq!(
        lines[11], "72",
        "callee param spill は mov [rbp-offset], rdi の 0x48"
    );
    assert_eq!(lines[12], "137", "callee param spill 2 byte 目は 0x89");
    assert_eq!(
        lines[13], "189",
        "callee param spill 3 byte 目は ModRM 0xBD"
    );
    assert_eq!(lines[14], "93", "payload 末尾手前は pop rbp");
    assert_eq!(lines[15], "195", "payload 末尾は ret");
}

/// NATIVE-REAL-08f2: x86_64 で import prefix を含む actual module index space の 1 引数 call が同じ rel32 bytes を持つこと
#[test]
#[ignore]
fn test_native_codegen_emits_x86_import_prefixed_direct_call_arg_bundle_bytes() {
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
  (let [import-meta (make-function-meta 0 0 (vector-new 0))
        caller-ir (vector-push
                    (vector-push (vector-new 2) (make-instr 1 42))
                    (make-call 2))
        callee-ir (vector-push (vector-new 1) (make-local-get 0))
        caller (make-function-meta 0 0 caller-ir)
        callee (make-function-meta 1 0 callee-ir)
        functions (vector-push
                    (vector-push
                      (vector-push (vector-new 3) import-meta)
                      caller)
                    callee)
        target (make-target 1)
        native (emit-native-function-meta-bundle-with-import-count functions 1 target)]
    (do
      (print (vector-length native))
      (print (vector-get native 14))
      (print (vector-get native 15))
      (print (vector-get native 16))
      (print (vector-get native 17))
      (print (vector-get native 18))
      (print (vector-get native 19))
      (print (vector-get native 20))
      (print (vector-get native 21))
      (print (vector-get native 22))
      (print (vector-get native 23))
      (print (vector-get native 37))
      (print (vector-get native 38))
      (print (vector-get native 39))
      (print (vector-get native 61))
      (print (vector-get native 62))
      (print (vector-get native 63))
      0)))"#,
    );
    let lines: Vec<&str> = output.trim().lines().collect();

    assert!(
        lines.len() >= 17,
        "x86 import-prefixed direct call arg bundle bytes 出力が不足: {:?}",
        lines
    );
    assert_eq!(
        lines[0], "64",
        "x86_64 import-prefixed direct call arg bundle payload は 64 bytes であるべき"
    );
    assert_eq!(lines[1], "72", "arg move 先頭は mov rdi, rax の 0x48");
    assert_eq!(lines[2], "137", "arg move 2 byte 目は 0x89");
    assert_eq!(lines[3], "199", "arg move 3 byte 目は ModRM 0xC7");
    assert_eq!(
        lines[4], "81",
        "1 引数 call 前に previous-value 用 rcx を push する"
    );
    assert_eq!(lines[5], "232", "direct call は call rel32 opcode 0xE8");
    assert_eq!(lines[6], "3", "forward call offset の下位 byte は 3");
    assert_eq!(lines[7], "0", "forward call offset byte1 は 0");
    assert_eq!(lines[8], "0", "forward call offset byte2 は 0");
    assert_eq!(lines[9], "0", "forward call offset byte3 は 0");
    assert_eq!(
        lines[10], "89",
        "1 引数 call 後に previous-value 用 rcx を pop する"
    );
    assert_eq!(
        lines[11], "72",
        "callee param spill は mov [rbp-offset], rdi の 0x48"
    );
    assert_eq!(lines[12], "137", "callee param spill 2 byte 目は 0x89");
    assert_eq!(
        lines[13], "189",
        "callee param spill 3 byte 目は ModRM 0xBD"
    );
    assert_eq!(lines[14], "93", "callee epilogue 手前は pop rbp");
    assert_eq!(lines[15], "195", "callee epilogue 末尾は ret");
    assert_eq!(lines[16], "195", "import stub 末尾は ret");
}

/// NATIVE-REAL-08f3: x86_64 の import call は import index に関係なく user code 直後の shared ret stub を指すこと
#[test]
#[ignore]
fn test_native_codegen_emits_x86_import_call_targets_trailing_ret_stub() {
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
   (let [import-meta (make-function-meta 0 0 (vector-new 0))
         imports (vector-push
                   (vector-push
                     (vector-push
                       (vector-push
                         (vector-push
                           (vector-push (vector-new 6) import-meta)
                           import-meta)
                         import-meta)
                       import-meta)
                     import-meta)
                   import-meta)
         caller-ir (vector-push
                     (vector-push (vector-new 2) (make-instr 1 42))
                     (make-call 5))
         caller (make-function-meta 0 0 caller-ir)
         functions (vector-push imports caller)
         target (make-target 1)
         native (emit-native-function-meta-bundle-with-import-count functions 6 target)]
    (do
      (print-bytes native 0 (vector-length native))
      0)))"#,
    );
    let bytes = output
        .trim()
        .lines()
        .map(|line| {
            line.parse::<u8>()
                .unwrap_or_else(|_| panic!("x86 import call byte parse 失敗: {line}"))
        })
        .collect::<Vec<_>>();

    let call_offset = bytes
        .iter()
        .position(|byte| *byte == 0xe8)
        .unwrap_or_else(|| panic!("x86 import call に rel32 call opcode が無い: {bytes:?}"));
    let rel = i32::from_le_bytes([
        bytes[call_offset + 1],
        bytes[call_offset + 2],
        bytes[call_offset + 3],
        bytes[call_offset + 4],
    ]);
    let target = call_offset as isize + 5 + rel as isize;

    assert_eq!(
        bytes.get(target as usize).copied(),
        Some(0xc3),
        "x86 import call target は user code 直後の ret stub を指すべき: call_offset={call_offset} rel={rel} target={target} len={} bytes={bytes:?}",
        bytes.len()
    );
}

/// NATIVE-REAL-08f4: x86_64 の per-function emit は 5-arg import layout で local stub へ import call を向けること
#[test]
#[ignore]
fn test_native_codegen_x86_function_emit_uses_import_layout_stub_offset() {
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

(defn push-imports [idx count result import-meta]
  (if (>= idx count)
    result
    (push-imports (+ idx 1) count (vector-push result import-meta) import-meta)))

(defn print-bytes [bytes idx n]
  (if (>= idx n)
    0
    (do
      (print (vector-get bytes idx))
      (print-bytes bytes (+ idx 1) n))))

(defn main []
  (let [import-meta (make-function-meta 0 0 (vector-new 0))
        imports (push-imports 0 6 (vector-new 8) import-meta)
        caller-ir (vector-push
                    (vector-push
                      (vector-push (vector-new 3) (make-instr 1 42))
                      (make-instr 11 0))
                    (make-call 5))
        caller (make-function-meta 0 1 caller-ir)
        functions (vector-push imports caller)
        starts (vector-push (vector-new 1) 0)
        result (ref-new (vector-new 0))
        import-stub-offset (native-function-size-x86 caller functions)]
    (do
      (generate-native-function-x86-64-bundle-with-import-count caller result starts functions 6 import-stub-offset)
      (print-bytes (ref-get result) 0 (vector-length (ref-get result)))
      0)))"#,
    );
    let bytes = output
        .trim()
        .lines()
        .map(|line| {
            line.parse::<u8>()
                .unwrap_or_else(|_| panic!("x86 per-function import call byte parse 失敗: {line}"))
        })
        .collect::<Vec<_>>();
    let call_offset = bytes
        .iter()
        .position(|byte| *byte == 0xe8)
        .unwrap_or_else(|| {
            panic!("x86 per-function import call に rel32 call opcode が無い: {bytes:?}")
        });
    let rel = i32::from_le_bytes([
        bytes[call_offset + 1],
        bytes[call_offset + 2],
        bytes[call_offset + 3],
        bytes[call_offset + 4],
    ]);
    let target = call_offset as isize + 5 + rel as isize;

    assert_eq!(
        target as usize,
        bytes.len(),
        "x86 per-function import call target は派生 local stub offset を指すべき: call_offset={call_offset} rel={rel} target={target} len={} bytes={bytes:?}",
        bytes.len()
    );
}

/// NATIVE-REAL-08f5: x86_64 の per-function emit は backward user call を関数先頭基準へ補正すること
#[test]
#[ignore]
fn test_native_codegen_x86_function_emit_uses_relative_start_for_backward_user_call() {
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
  (let [target-ir (vector-push (vector-new 1) (make-instr 1 7))
        target (make-function-meta 2 0 target-ir)
        caller-ir (vector-push
                    (vector-push
                      (vector-push (vector-new 3) (make-instr 1 1))
                      (make-instr 1 2))
                    (make-call 0))
        caller (make-function-meta 0 0 caller-ir)
        functions (vector-push (vector-push (vector-new 2) target) caller)
        target-size (native-function-size-x86 target functions)
        starts (vector-push (vector-push (vector-new 2) (- 0 target-size)) 0)
        result (ref-new (vector-new 0))]
    (do
      (generate-native-function-x86-64-bundle-with-import-count caller result starts functions 0 0)
      (print target-size)
      (print-bytes (ref-get result) 0 (vector-length (ref-get result)))
      0)))"#,
    );
    let mut lines = output.trim().lines();
    let target_size = lines
        .next()
        .expect("target size output")
        .parse::<isize>()
        .expect("target size parse");
    let bytes = lines
        .map(|line| {
            line.parse::<u8>().unwrap_or_else(|_| {
                panic!("x86 per-function backward call byte parse 失敗: {line}")
            })
        })
        .collect::<Vec<_>>();
    let call_offset = bytes
        .iter()
        .position(|byte| *byte == 0xe8)
        .unwrap_or_else(|| {
            panic!("x86 per-function backward call に rel32 call opcode が無い: {bytes:?}")
        });
    let rel = i32::from_le_bytes([
        bytes[call_offset + 1],
        bytes[call_offset + 2],
        bytes[call_offset + 3],
        bytes[call_offset + 4],
    ]);
    let target = call_offset as isize + 5 + rel as isize;

    assert_eq!(
        target, -target_size,
        "x86 per-function backward user call target は関数先頭基準の負 offset を指すべき: call_offset={call_offset} rel={rel} target={target} target_size={target_size} bytes={bytes:?}"
    );
}

/// NATIVE-REAL-08f6: x86_64 の monolithic emit は後続関数の user call でも絶対 function start を使うこと
#[test]
#[ignore]
fn test_native_codegen_x86_monolithic_emit_keeps_absolute_start_for_nonzero_caller() {
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
        caller-ir (vector-push (vector-new 1) (make-call 2))
        caller (make-function-meta 0 0 caller-ir)
        callee-ir (vector-push (vector-new 1) (make-instr 1 42))
        callee (make-function-meta 0 0 callee-ir)
        functions (vector-push (vector-push (vector-push (vector-new 3) filler) caller) callee)
        starts (collect-callable-function-starts-x86 functions 0)
        target (make-target 1)
        native (emit-native-function-meta-bundle functions target)]
    (do
      (print (vector-get starts 1))
      (print (vector-get starts 2))
      (print-bytes native 0 (vector-length native))
      0)))"#,
    );
    let mut lines = output.trim().lines();
    let caller_start = lines
        .next()
        .expect("caller start output")
        .parse::<isize>()
        .expect("caller start parse");
    let callee_start = lines
        .next()
        .expect("callee start output")
        .parse::<isize>()
        .expect("callee start parse");
    let bytes = lines
        .map(|line| {
            line.parse::<u8>()
                .unwrap_or_else(|_| panic!("x86 monolithic call byte parse 失敗: {line}"))
        })
        .collect::<Vec<_>>();
    let call_search_end = callee_start as usize;
    let call_offset = (caller_start as usize..call_search_end)
        .find(|idx| bytes.get(*idx).copied() == Some(0xe8) && *idx + 4 < bytes.len())
        .unwrap_or_else(|| {
            panic!(
                "x86 monolithic caller 内に rel32 call が無い: caller_start={caller_start} callee_start={callee_start} bytes={bytes:?}"
            )
        });

    assert_eq!(
        bytes.get(call_offset).copied(),
        Some(0xe8),
        "x86 monolithic caller body 先頭は rel32 call であるべき: caller_start={caller_start} bytes={bytes:?}"
    );
    let rel = i32::from_le_bytes([
        bytes[call_offset + 1],
        bytes[call_offset + 2],
        bytes[call_offset + 3],
        bytes[call_offset + 4],
    ]);
    let target = call_offset as isize + 5 + rel as isize;

    assert_eq!(
        target, callee_start,
        "x86 monolithic user call target は code 全体の絶対 callee start を指すべき: call_offset={call_offset} rel={rel} target={target} callee_start={callee_start} bytes={bytes:?}"
    );
}
