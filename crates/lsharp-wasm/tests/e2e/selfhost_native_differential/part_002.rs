
fn assert_x86_i32_logic_tail(name: &str, opcode: u32, expected_instr: [u32; 2]) {
    let output = run_native_codegen_harness(&format!(
        r#"(module Main)
(import NativeTarget)
(import NativeCodegen)

(defn make-instr [opcode operand]
  (vector-push (vector-push (vector-new 2) opcode) operand))

(defn main []
  (let [instr1 (make-instr 3 12)
        instr2 (make-instr 11 0)
        instr3 (make-instr 3 10)
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
        values.len() >= 12,
        "{name}: logic tail 出力が不足: {values:?}"
    );
    assert!(
        values[0] >= 11,
        "{name}: payload 長が短すぎるため logic tail を検査できない: {values:?}"
    );
    assert_eq!(
        &values[1..12],
        &[
            expected_instr[0],
            expected_instr[1],
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
        "{name}: x86 logic tail は opcode + add rsp,16 + epilogue であるべき"
    );
}

/// NATIVE-REAL-08c1: x86_64 で i32.and / i32.or が distinct bytes を持つこと
#[test]
#[ignore]
fn test_native_codegen_emits_x86_i32_logic_bytes() {
    assert_x86_i32_logic_tail("i32.and", 26, [33, 200]);
    assert_x86_i32_logic_tail("i32.or", 27, [9, 200]);
}

/// NATIVE-REAL-08c3: x86_64 で i64.mul が distinct bytes を持つこと
#[test]
#[ignore]
fn test_native_codegen_emits_x86_i64_mul_bytes() {
    let output = run_native_codegen_harness(
        r#"(module Main)
(import NativeTarget)
(import NativeCodegen)

(defn make-instr [opcode operand]
  (vector-push (vector-push (vector-new 2) opcode) operand))

(defn main []
  (let [instr1 (make-instr 1 21)
        instr2 (make-instr 11 0)
        instr3 (make-instr 1 2)
        instr4 (make-instr 11 1)
        instr5 (make-instr 10 0)
        instr6 (make-instr 10 1)
        instr7 (make-instr 22 0)
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
    );
    let values: Vec<u32> = output
        .trim()
        .lines()
        .map(|line| {
            line.parse::<u32>()
                .unwrap_or_else(|_| panic!("i64.mul: 数値出力であるべきだが `{line}` を得た"))
        })
        .collect();

    assert!(values.len() >= 14, "i64.mul tail 出力が不足: {values:?}");
    assert!(
        values[0] >= 13,
        "i64.mul payload 長が短すぎるため tail を検査できない: {values:?}"
    );
    assert_eq!(
        &values[1..14],
        &[72, 15, 175, 193, 72, 129, 196, 16, 0, 0, 0, 93, 195],
        "x86 i64.mul tail は imul + add rsp,16 + epilogue であるべき"
    );
}

/// NATIVE-REAL-08c3b: x86_64 で i64.sub が lhs - rhs の順序を保つ byte列を持つこと
#[test]
#[ignore]
fn test_native_codegen_emits_x86_i64_sub_order_bytes() {
    let output = run_native_codegen_harness(
        r#"(module Main)
(import NativeTarget)
(import NativeCodegen)

(defn make-instr [opcode operand]
  (vector-push (vector-push (vector-new 2) opcode) operand))

(defn main []
  (let [instr1 (make-instr 1 47)
        instr2 (make-instr 11 0)
        instr3 (make-instr 1 5)
        instr4 (make-instr 11 1)
        instr5 (make-instr 10 0)
        instr6 (make-instr 10 1)
        instr7 (make-instr 21 0)
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
    );
    let values: Vec<u32> = output
        .trim()
        .lines()
        .map(|line| {
            line.parse::<u32>()
                .unwrap_or_else(|_| panic!("i64.sub: 数値出力であるべきだが `{line}` を得た"))
        })
        .collect();

    assert!(values.len() >= 16, "i64.sub tail 出力が不足: {values:?}");
    assert!(
        values[0] >= 15,
        "i64.sub payload 長が短すぎるため tail を検査できない: {values:?}"
    );
    assert_eq!(
        &values[1..16],
        &[
            72, 41, 193, 72, 137, 200, 72, 129, 196, 16, 0, 0, 0, 93, 195
        ],
        "x86 i64.sub tail は sub rcx, rax; mov rax, rcx + epilogue であるべき"
    );
}

/// NATIVE-REAL-08c4: x86_64 で i64.div が distinct bytes を持つこと
#[test]
#[ignore]
fn test_native_codegen_emits_x86_i64_div_bytes() {
    let output = run_native_codegen_harness(
        r#"(module Main)
(import NativeTarget)
(import NativeCodegen)

(defn make-instr [opcode operand]
  (vector-push (vector-push (vector-new 2) opcode) operand))

(defn main []
  (let [instr1 (make-instr 1 84)
        instr2 (make-instr 11 0)
        instr3 (make-instr 1 2)
        instr4 (make-instr 11 1)
        instr5 (make-instr 10 0)
        instr6 (make-instr 10 1)
        instr7 (make-instr 23 0)
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
      (print (vector-get native (- n 20)))
      (print (vector-get native (- n 19)))
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
    );
    let values: Vec<u32> = output
        .trim()
        .lines()
        .map(|line| {
            line.parse::<u32>()
                .unwrap_or_else(|_| panic!("i64.div: 数値出力であるべきだが `{line}` を得た"))
        })
        .collect();

    assert!(values.len() >= 21, "i64.div tail 出力が不足: {values:?}");
    assert!(
        values[0] >= 20,
        "i64.div payload 長が短すぎるため tail を検査できない: {values:?}"
    );
    assert_eq!(
        &values[1..21],
        &[
            72, 137, 198, 72, 137, 200, 72, 153, 72, 247, 254, 72, 129, 196, 16, 0, 0, 0, 93, 195,
        ],
        "x86 i64.div tail は divisor save + dividend restore + cqo/idiv + add rsp,16 + epilogue であるべき"
    );
}

/// NATIVE-REAL-08c5: x86_64 で i64.rem が distinct bytes を持つこと
#[test]
#[ignore]
fn test_native_codegen_emits_x86_i64_rem_bytes() {
    let output = run_native_codegen_harness(
        r#"(module Main)
(import NativeTarget)
(import NativeCodegen)

(defn make-instr [opcode operand]
  (vector-push (vector-push (vector-new 2) opcode) operand))

(defn main []
  (let [instr1 (make-instr 1 85)
        instr2 (make-instr 11 0)
        instr3 (make-instr 1 43)
        instr4 (make-instr 11 1)
        instr5 (make-instr 10 0)
        instr6 (make-instr 10 1)
        instr7 (make-instr 28 0)
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
      (print (vector-get native (- n 23)))
      (print (vector-get native (- n 22)))
      (print (vector-get native (- n 21)))
      (print (vector-get native (- n 20)))
      (print (vector-get native (- n 19)))
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
    );
    let values: Vec<u32> = output
        .trim()
        .lines()
        .map(|line| {
            line.parse::<u32>()
                .unwrap_or_else(|_| panic!("i64.rem: 数値出力であるべきだが `{line}` を得た"))
        })
        .collect();

    assert!(values.len() >= 24, "i64.rem tail 出力が不足: {values:?}");
    assert!(
        values[0] >= 23,
        "i64.rem payload 長が短すぎるため tail を検査できない: {values:?}"
    );
    assert_eq!(
        &values[1..24],
        &[
            72, 137, 198, 72, 137, 200, 72, 153, 72, 247, 254, 72, 137, 208, 72, 129, 196, 16, 0,
            0, 0, 93, 195,
        ],
        "x86 i64.rem tail は divisor save + dividend restore + cqo/idiv + mov rax,rdx + add rsp,16 + epilogue であるべき"
    );
}

fn assert_x86_memory_load_tail(
    name: &str,
    opcode: u32,
    offset: u32,
    tail_len: usize,
    expected_tail: &[u32],
) {
    let output = run_native_codegen_harness(&format!(
        r#"(module Main)
(import NativeTarget)
(import NativeCodegen)

(defn make-instr [opcode operand]
  (vector-push (vector-push (vector-new 2) opcode) operand))

(defn print-tail [native idx n]
  (if (>= idx n)
    0
    (do
      (print (vector-get native idx))
      (print-tail native (+ idx 1) n))))

(defn main []
  (let [instr1 (make-instr 1 0)
        instr2 (make-instr {opcode} {offset})
        ir (vector-push
             (vector-push (vector-new 2) instr1)
             instr2)
        target (make-target 1)
        native (emit-native ir target)
        n (vector-length native)]
    (do
      (print n)
      (print-tail native (- n {tail_len}) n)
      0)))"#
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
        values.len() >= (tail_len + 1),
        "{name}: tail 出力が不足: {values:?}"
    );
    assert!(
        values[0] >= tail_len as u32,
        "{name}: payload 長が短すぎるため tail を検査できない: {values:?}"
    );
    assert_eq!(
        &values[1..(tail_len + 1)],
        expected_tail,
        "{name}: x86 load tail が期待と異なる"
    );
}

/// NATIVE-REAL-08c6: x86_64 で i64/i32 load 系が distinct bytes を持つこと
#[test]
#[ignore]
fn test_native_codegen_emits_x86_memory_load_bytes() {
    assert_x86_memory_load_tail("i64.load", 48, 8, 6, &[72, 139, 64, 8, 93, 195]);
    assert_x86_memory_load_tail("i32.load", 45, 4, 5, &[139, 64, 4, 93, 195]);
    assert_x86_memory_load_tail("i32.load8_u", 47, 1, 6, &[15, 182, 64, 1, 93, 195]);
}

fn assert_x86_memory_store_tail(
    name: &str,
    value_opcode: u32,
    value_operand: u32,
    store_opcode: u32,
    store_offset: u32,
    tail_len: usize,
    expected_tail: &[u32],
) {
    let output = run_native_codegen_harness(&format!(
        r#"(module Main)
(import NativeTarget)
(import NativeCodegen)

(defn make-instr [opcode operand]
  (vector-push (vector-push (vector-new 2) opcode) operand))

(defn print-tail [native idx n]
  (if (>= idx n)
    0
    (do
      (print (vector-get native idx))
      (print-tail native (+ idx 1) n))))

(defn main []
  (let [instr1 (make-instr 1 0)
        instr2 (make-instr 11 0)
        instr3 (make-instr {value_opcode} {value_operand})
        instr4 (make-instr 11 1)
        instr5 (make-instr 10 0)
        instr6 (make-instr 10 1)
        instr7 (make-instr {store_opcode} {store_offset})
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
      (print-tail native (- n {tail_len}) n)
      0)))"#
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
        values.len() >= (tail_len + 1),
        "{name}: tail 出力が不足: {values:?}"
    );
    assert!(
        values[0] >= tail_len as u32,
        "{name}: payload 長が短すぎるため tail を検査できない: {values:?}"
    );
    assert_eq!(
        &values[1..(tail_len + 1)],
        expected_tail,
        "{name}: x86 store tail が期待と異なる"
    );
}

/// NATIVE-REAL-08c7: x86_64 で i64/i32 store 系が distinct bytes を持つこと
#[test]
#[ignore]
fn test_native_codegen_emits_x86_memory_store_bytes() {
    assert_x86_memory_store_tail(
        "i64.store",
        1,
        42,
        49,
        8,
        13,
        &[72, 137, 65, 8, 72, 129, 196, 16, 0, 0, 0, 93, 195],
    );
    assert_x86_memory_store_tail(
        "i32.store",
        3,
        42,
        46,
        4,
        12,
        &[137, 65, 4, 72, 129, 196, 16, 0, 0, 0, 93, 195],
    );
}

fn assert_x86_memory_bulk_tail(
    name: &str,
    instrs: &[(u32, u32)],
    tail_len: usize,
    expected_tail: &[u32],
) {
    let instr_bindings = instrs
        .iter()
        .enumerate()
        .map(|(idx, (opcode, operand))| format!("instr{idx} (make-instr {opcode} {operand})"))
        .collect::<Vec<_>>()
        .join("\n        ");
    let ir_expr = (0..instrs.len()).fold(format!("(vector-new {})", instrs.len()), |expr, idx| {
        format!("(vector-push {expr} instr{idx})")
    });
    let output = run_native_codegen_harness(&format!(
        r#"(module Main)
(import NativeTarget)
(import NativeCodegen)

(defn make-instr [opcode operand]
  (vector-push (vector-push (vector-new 2) opcode) operand))

(defn make-function-meta [param-count local-count ir]
  (vector-push
    (vector-push
      (vector-push (vector-new 3) param-count)
      local-count)
    ir))

(defn print-tail [native idx n]
  (if (>= idx n)
    0
    (do
      (print (vector-get native idx))
      (print-tail native (+ idx 1) n))))

(defn main []
  (let [{instr_bindings}
        ir {ir_expr}
        func (make-function-meta 1 0 ir)
        functions (vector-push (vector-new 1) func)
        target (make-target 1)
        native (emit-native-function-meta-bundle functions target)
        n (vector-length native)]
    (do
      (print n)
      (print-tail native (- n {tail_len}) n)
      0)))"#
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
        values.len() >= (tail_len + 1),
        "{name}: tail 出力が不足: {values:?}"
    );
    assert!(
        values[0] >= tail_len as u32,
        "{name}: payload 長が短すぎるため tail を検査できない: {values:?}"
    );
    assert_eq!(
        &values[1..(tail_len + 1)],
        expected_tail,
        "{name}: x86 memory bulk tail が期待と異なる"
    );
}

fn assert_x86_plain_control_tail(
    name: &str,
    instrs: &[(u32, i64)],
    tail_len: usize,
    expected_tail: &[u32],
) {
    let instr_bindings = instrs
        .iter()
        .enumerate()
        .map(|(idx, (opcode, operand))| format!("instr{idx} (make-instr {opcode} {operand})"))
        .collect::<Vec<_>>()
        .join("\n        ");
    let ir_expr = (0..instrs.len()).fold(format!("(vector-new {})", instrs.len()), |expr, idx| {
        format!("(vector-push {expr} instr{idx})")
    });
    let output = run_native_codegen_harness(&format!(
        r#"(module Main)
(import NativeTarget)
(import NativeCodegen)

(defn make-instr [opcode operand]
  (vector-push (vector-push (vector-new 2) opcode) operand))

(defn print-tail [native idx n]
  (if (>= idx n)
    0
    (do
      (print (vector-get native idx))
      (print-tail native (+ idx 1) n))))

(defn main []
  (let [{instr_bindings}
        ir {ir_expr}
        target (make-target 1)
        native (emit-native ir target)
        n (vector-length native)]
    (do
      (print n)
      (print-tail native (- n {tail_len}) n)
      0)))"#
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
        values.len() >= (tail_len + 1),
        "{name}: tail 出力が不足: {values:?}"
    );
    assert!(
        values[0] >= tail_len as u32,
        "{name}: payload 長が短すぎるため tail を検査できない: {values:?}"
    );
    assert_eq!(
        &values[1..(tail_len + 1)],
        expected_tail,
        "{name}: x86 control-flow tail が期待と異なる"
    );
}

/// NATIVE-REAL-08c8: x86_64 で memory.copy / memory.fill が dedicated bytes を持つこと。
#[test]
#[ignore]
fn test_native_codegen_emits_x86_memory_bulk_bytes() {
    assert_x86_memory_bulk_tail(
        "memory.copy",
        &[(10, 0), (10, 0), (3, 5), (77, 0)],
        24,
        &[
            72, 139, 189, 240, 255, 255, 255, 72, 137, 206, 72, 137, 193, 243, 164, 72, 129, 196,
            16, 0, 0, 0, 93, 195,
        ],
    );
    assert_x86_memory_bulk_tail(
        "memory.fill",
        &[(10, 0), (3, 42), (3, 5), (78, 0)],
        27,
        &[
            72, 137, 202, 72, 139, 189, 240, 255, 255, 255, 72, 137, 193, 72, 137, 208, 243, 170,
            72, 129, 196, 16, 0, 0, 0, 93, 195,
        ],
    );
}
