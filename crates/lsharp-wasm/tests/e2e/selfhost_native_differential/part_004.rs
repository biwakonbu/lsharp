
/// NATIVE-REAL-08f7: x86_64 の monolithic emit は後続関数の import call でも caller base を補正すること
#[test]
#[ignore]
fn test_native_codegen_x86_monolithic_emit_localizes_import_stub_for_nonzero_caller() {
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
  (let [import-ir (vector-new 0)
        import-func (make-function-meta 0 0 import-ir)
        filler-ir (vector-push (vector-new 1) (make-instr 1 7))
        filler (make-function-meta 0 0 filler-ir)
        caller-ir (vector-push (vector-new 1) (make-call 0))
        caller (make-function-meta 0 0 caller-ir)
        functions (vector-push (vector-push (vector-push (vector-new 3) import-func) filler) caller)
        starts (collect-callable-function-starts-x86 functions 1)
        import-stub-offset (callable-user-total-size-x86 functions 1)
        target (make-target 1)
        native (emit-native-function-meta-bundle-with-import-count functions 1 target)]
    (do
      (print (vector-get starts 1))
      (print import-stub-offset)
      (print-bytes native 0 (vector-length native))
      0)))"#,
    );
    let mut lines = output.trim().lines();
    let caller_start = lines
        .next()
        .expect("caller start output")
        .parse::<isize>()
        .expect("caller start parse");
    let import_stub_offset = lines
        .next()
        .expect("import stub output")
        .parse::<isize>()
        .expect("import stub parse");
    let bytes = lines
        .map(|line| {
            line.parse::<u8>()
                .unwrap_or_else(|_| panic!("x86 import call byte parse 失敗: {line}"))
        })
        .collect::<Vec<_>>();
    let call_search_end = import_stub_offset as usize;
    let call_offset = (caller_start as usize..call_search_end)
        .find(|idx| bytes.get(*idx).copied() == Some(0xe8) && *idx + 4 < bytes.len())
        .unwrap_or_else(|| {
            panic!(
                "x86 import caller 内に rel32 call が無い: caller_start={caller_start} import_stub_offset={import_stub_offset} bytes={bytes:?}"
            )
        });

    assert_eq!(
        bytes.get(call_offset).copied(),
        Some(0xe8),
        "x86 import caller body 先頭は rel32 call であるべき: caller_start={caller_start} bytes={bytes:?}"
    );
    let rel = i32::from_le_bytes([
        bytes[call_offset + 1],
        bytes[call_offset + 2],
        bytes[call_offset + 3],
        bytes[call_offset + 4],
    ]);
    let target = call_offset as isize + 5 + rel as isize;

    assert_eq!(
        target, import_stub_offset,
        "x86 monolithic import call target は code 全体の import stub offset を指すべき: call_offset={call_offset} rel={rel} target={target} import_stub_offset={import_stub_offset} bytes={bytes:?}"
    );
}

/// NATIVE-REAL-08f7: x86_64 の low-level function emit は既存 result 長を caller base として rel32 を計算すること
#[test]
#[ignore]
fn test_native_codegen_x86_function_emit_uses_existing_result_len_as_base() {
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

(defn push-zero-bytes [result idx len]
  (if (>= idx len)
    result
    (push-zero-bytes (vector-push result 144) (+ idx 1) len)))

(defn print-bytes [bytes idx n]
  (if (>= idx n)
    0
    (do
      (print (vector-get bytes idx))
      (print-bytes bytes (+ idx 1) n))))

(defn main []
  (let [filler-size 16
        caller-ir (vector-push (vector-new 1) (make-call 1))
        caller (make-function-meta 0 0 caller-ir)
        callee-ir (vector-push (vector-new 1) (make-instr 1 42))
        callee (make-function-meta 0 0 callee-ir)
        caller-size (native-function-size-x86 caller (vector-push (vector-push (vector-new 2) caller) callee))
        callee-start (+ filler-size caller-size)
        functions (vector-push (vector-push (vector-new 2) caller) callee)
        starts (vector-push (vector-push (vector-new 2) filler-size) callee-start)
        result (ref-new (push-zero-bytes (vector-new 0) 0 filler-size))]
    (do
      (generate-native-function-x86-64-bundle-with-import-count caller result starts functions 0 callee-start)
      (print filler-size)
      (print callee-start)
      (print-bytes (ref-get result) 0 (vector-length (ref-get result)))
      0)))"#,
    );
    let mut lines = output.trim().lines();
    let filler_size = lines
        .next()
        .expect("filler size output")
        .parse::<usize>()
        .expect("filler size parse");
    let callee_start = lines
        .next()
        .expect("callee start output")
        .parse::<isize>()
        .expect("callee start parse");
    let bytes = lines
        .map(|line| {
            line.parse::<u8>()
                .unwrap_or_else(|_| panic!("x86 existing-result call byte parse 失敗: {line}"))
        })
        .collect::<Vec<_>>();
    let call_offset = (filler_size..callee_start as usize)
        .find(|idx| bytes.get(*idx).copied() == Some(0xe8) && *idx + 4 < bytes.len())
        .unwrap_or_else(|| {
            panic!(
                "x86 existing-result caller 内に rel32 call が無い: filler_size={filler_size} callee_start={callee_start} bytes={bytes:?}"
            )
        });

    assert_eq!(
        bytes.get(call_offset).copied(),
        Some(0xe8),
        "x86 caller body 先頭は rel32 call であるべき: filler_size={filler_size} bytes={bytes:?}"
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
        "x86 low-level emit は caller base を result 長で補正すべき: call_offset={call_offset} rel={rel} target={target} callee_start={callee_start} bytes={bytes:?}"
    );
}

/// NATIVE-REAL-08f7b1: x86_64 の low-level function emit は明示 caller base を rel32 に使うこと
#[test]
#[ignore]
fn test_native_codegen_x86_function_emit_honors_explicit_function_start() {
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
        target (make-function-meta 0 0 target-ir)
        caller-ir (vector-push (vector-new 1) (make-call 0))
        caller (make-function-meta 0 0 caller-ir)
        functions (vector-push (vector-push (vector-new 2) target) caller)
        starts (vector-push (vector-push (vector-new 2) 0) 16)
        function-start 16
        result (ref-new (vector-new 0))]
    (do
      (generate-native-function-x86-64-bundle caller result starts functions function-start)
      (print function-start)
      (print-bytes (ref-get result) 0 (vector-length (ref-get result)))
      0)))"#,
    );
    let mut lines = output.trim().lines();
    let function_start = lines
        .next()
        .expect("explicit function start output")
        .parse::<isize>()
        .expect("explicit function start parse");
    let bytes = lines
        .map(|line| {
            line.parse::<u8>().unwrap_or_else(|_| {
                panic!("x86 explicit function start call byte parse 失敗: {line}")
            })
        })
        .collect::<Vec<_>>();
    let call_offset = bytes
        .iter()
        .position(|byte| *byte == 0xe8)
        .unwrap_or_else(|| panic!("x86 explicit function start call が無い: bytes={bytes:?}"));
    let rel = i32::from_le_bytes([
        bytes[call_offset + 1],
        bytes[call_offset + 2],
        bytes[call_offset + 3],
        bytes[call_offset + 4],
    ]);
    let target = call_offset as isize + 5 + rel as isize;

    assert_eq!(
        target, -function_start,
        "x86 low-level emit は result 配置と独立した明示 caller base を target 相対値へ使うべき: call_offset={call_offset} rel={rel} target={target} function_start={function_start} bytes={bytes:?}"
    );
}

/// NATIVE-REAL-08f7a: x86_64 の 55+ 引数 call は rel32 next offset を実 emit と一致させること
#[test]
#[ignore]
fn test_native_codegen_x86_high_arity_call_rel_next_offsets_match_emitters() {
    let output = run_native_codegen_harness(
        r#"(module Main)
(import NativeCodegen)

(defn find-call-offset [bytes idx len]
  (if (>= (+ idx 4) len)
    -1
    (if (and (= (vector-get bytes idx) 232)
             (and (= (vector-get bytes (+ idx 1)) 16)
                  (and (= (vector-get bytes (+ idx 2)) 0)
                       (and (= (vector-get bytes (+ idx 3)) 0)
                            (= (vector-get bytes (+ idx 4)) 0)))))
      idx
      (find-call-offset bytes (+ idx 1) len))))

(defn print-next-offset-check [arity bytes]
  (let [call-offset (find-call-offset bytes 0 (vector-length bytes))]
    (do
      (print arity)
      (print (vector-length bytes))
      (print (native-call-bundle-size-x86 arity 0))
      (print (+ call-offset 5))
      (print (native-call-rel-next-offset-x86 arity 0)))))

(defn main []
  (do
    (print-next-offset-check 55 (emit-fifty-five-arg-call-x86 16 0))
    (print-next-offset-check 56 (emit-fifty-six-arg-call-x86 16 0))
    (print-next-offset-check 57 (emit-fifty-seven-arg-call-x86 16 0))
    (print-next-offset-check 58 (emit-fifty-eight-arg-call-x86 16 0))
    (print-next-offset-check 59 (emit-fifty-nine-arg-call-x86 16 0))
    (print-next-offset-check 60 (emit-sixty-arg-call-x86 16 0))
    (print-next-offset-check 61 (emit-twenty-plus-arg-call-x86 61 16 0))
    0))"#,
    );

    let lines = output
        .trim()
        .lines()
        .map(|line| {
            line.parse::<isize>()
                .unwrap_or_else(|_| panic!("x86 high-arity rel next offset parse 失敗: {line}"))
        })
        .collect::<Vec<_>>();
    assert_eq!(
        lines.len() % 5,
        0,
        "x86 high-arity rel next offset 出力は arity/actual-size/declared-size/actual-next/declared-next の5列であるべき: {lines:?}"
    );

    for chunk in lines.chunks(5) {
        let arity = chunk[0];
        let actual_size = chunk[1];
        let declared_size = chunk[2];
        let actual_next_offset = chunk[3];
        let declared_next_offset = chunk[4];
        assert_eq!(
            declared_size, actual_size,
            "x86 {arity}-arg call の size table は実 emit 長と一致すべき"
        );
        assert_eq!(
            declared_next_offset, actual_next_offset,
            "x86 {arity}-arg call の rel32 next offset は emit 済み call 終端と一致すべき"
        );
    }
}

/// NATIVE-REAL-08f7b: x86_64 segmented emit の 6 引数 call は caller base を差し引いて rel32 を計算すること
#[test]
#[ignore]
fn test_native_codegen_x86_function_emit_localizes_six_arg_call_for_existing_result() {
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

(defn push-zero-bytes [result idx len]
  (if (>= idx len)
    result
    (push-zero-bytes (vector-push result 144) (+ idx 1) len)))

(defn print-bytes [bytes idx n]
  (if (>= idx n)
    0
    (do
      (print (vector-get bytes idx))
      (print-bytes bytes (+ idx 1) n))))

(defn main []
  (let [filler-size 16
        caller-ir (vector-push
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
        callee-ir (vector-push (vector-new 1) (make-local-get 0))
        caller (make-function-meta 0 0 caller-ir)
        callee (make-function-meta 6 0 callee-ir)
        functions (vector-push (vector-push (vector-new 2) caller) callee)
        caller-size (native-function-size-x86 caller functions)
        callee-start (+ filler-size caller-size)
        helper-offset (+ callee-start (native-function-size-x86 callee functions))
        starts (vector-push (vector-push (vector-new 2) filler-size) callee-start)
        result (ref-new (push-zero-bytes (vector-new 0) 0 filler-size))
        layout (make-x86-function-emit-layout 0 helper-offset filler-size filler-size)]
    (do
      (generate-native-function-x86-64-bundle-with-layout caller result starts functions layout)
      (print filler-size)
      (print callee-start)
      (print-bytes (ref-get result) 0 (vector-length (ref-get result)))
      0)))"#,
    );
    let mut lines = output.trim().lines();
    let filler_size = lines
        .next()
        .expect("filler size output")
        .parse::<usize>()
        .expect("filler size parse");
    let callee_start = lines
        .next()
        .expect("callee start output")
        .parse::<isize>()
        .expect("callee start parse");
    let bytes = lines
        .map(|line| {
            line.parse::<u8>().unwrap_or_else(|_| {
                panic!("x86 six-arg existing-result call byte parse 失敗: {line}")
            })
        })
        .collect::<Vec<_>>();
    let call_offset = (filler_size..bytes.len())
        .rev()
        .find(|idx| bytes[*idx] == 0xe8)
        .unwrap_or_else(|| panic!("x86 six-arg call が見つからない: bytes={bytes:?}"));
    let rel = i32::from_le_bytes([
        bytes[call_offset + 1],
        bytes[call_offset + 2],
        bytes[call_offset + 3],
        bytes[call_offset + 4],
    ]);
    let target = call_offset as isize + 5 + rel as isize;

    assert_eq!(
        target, callee_start,
        "x86 six-arg segmented emit は caller base を補正して callee start を指すべき: call_offset={call_offset} rel={rel} target={target} callee_start={callee_start} bytes={bytes:?}"
    );
}

/// NATIVE-REAL-08f7c: x86_64 segmented emit の 7 引数 call は深い value window でも rel32 を補正すること
#[test]
#[ignore]
fn test_native_codegen_x86_function_emit_localizes_seven_arg_call_for_deep_stack() {
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

(defn push-zero-bytes [result idx len]
  (if (>= idx len)
    result
    (push-zero-bytes (vector-push result 144) (+ idx 1) len)))

(defn print-bytes [bytes idx n]
  (if (>= idx n)
    0
    (do
      (print (vector-get bytes idx))
      (print-bytes bytes (+ idx 1) n))))

(defn main []
  (let [filler-size 16
        caller-ir (vector-push
                    (vector-push
                      (vector-push
                        (vector-push
                          (vector-push
                            (vector-push
                              (vector-push
                                (vector-push
                                  (vector-push
                                    (vector-push (vector-new 10) (make-instr 3 101))
                                    (make-instr 3 102))
                                  (make-instr 3 1))
                                (make-instr 3 2))
                              (make-instr 3 3))
                            (make-instr 3 4))
                          (make-instr 3 5))
                        (make-instr 3 6))
                      (make-instr 3 7))
                    (make-call 1))
        callee-ir (vector-push (vector-new 1) (make-local-get 0))
        caller (make-function-meta 0 0 caller-ir)
        callee (make-function-meta 7 0 callee-ir)
        functions (vector-push (vector-push (vector-new 2) caller) callee)
        caller-size (native-function-size-x86 caller functions)
        callee-start (+ filler-size caller-size)
        helper-offset (+ callee-start (native-function-size-x86 callee functions))
        starts (vector-push (vector-push (vector-new 2) filler-size) callee-start)
        result (ref-new (push-zero-bytes (vector-new 0) 0 filler-size))
        layout (make-x86-function-emit-layout 0 helper-offset filler-size filler-size)]
    (do
      (generate-native-function-x86-64-bundle-with-layout caller result starts functions layout)
      (print filler-size)
      (print callee-start)
      (print-bytes (ref-get result) 0 (vector-length (ref-get result)))
      0)))"#,
    );
    let mut lines = output.trim().lines();
    let filler_size = lines
        .next()
        .expect("filler size output")
        .parse::<usize>()
        .expect("filler size parse");
    let callee_start = lines
        .next()
        .expect("callee start output")
        .parse::<isize>()
        .expect("callee start parse");
    let bytes = lines
        .map(|line| {
            line.parse::<u8>()
                .unwrap_or_else(|_| panic!("x86 seven-arg deep-stack call byte parse 失敗: {line}"))
        })
        .collect::<Vec<_>>();
    let call_offset = (filler_size..bytes.len())
        .rev()
        .find(|idx| bytes[*idx] == 0xe8)
        .unwrap_or_else(|| panic!("x86 seven-arg call が見つからない: bytes={bytes:?}"));
    let rel = i32::from_le_bytes([
        bytes[call_offset + 1],
        bytes[call_offset + 2],
        bytes[call_offset + 3],
        bytes[call_offset + 4],
    ]);
    let target = call_offset as isize + 5 + rel as isize;

    assert_eq!(
        target, callee_start,
        "x86 seven-arg segmented emit は deep value window の prefix を含めて callee start を指すべき: call_offset={call_offset} rel={rel} target={target} callee_start={callee_start} bytes={bytes:?}"
    );
}

/// NATIVE-REAL-08f7d: x86_64 segmented emit の 4 引数 call は post-consume tail を rel32 に含めないこと
#[test]
#[ignore]
fn test_native_codegen_x86_function_emit_localizes_four_arg_call_with_existing_stack() {
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

(defn push-zero-bytes [result idx len]
  (if (>= idx len)
    result
    (push-zero-bytes (vector-push result 144) (+ idx 1) len)))

(defn print-bytes [bytes idx n]
  (if (>= idx n)
    0
    (do
      (print (vector-get bytes idx))
      (print-bytes bytes (+ idx 1) n))))

(defn main []
  (let [filler-size 16
        caller-ir (vector-push
                    (vector-push
                      (vector-push
                        (vector-push
                          (vector-push
                            (vector-push (vector-new 7) (make-instr 3 101))
                            (make-instr 3 102))
                          (make-instr 3 103))
                        (make-instr 3 104))
                      (make-instr 3 105))
                    (make-call 1))
        callee-ir (vector-push (vector-new 1) (make-local-get 0))
        caller (make-function-meta 0 0 caller-ir)
        callee (make-function-meta 4 0 callee-ir)
        functions (vector-push (vector-push (vector-new 2) caller) callee)
        caller-size (native-function-size-x86 caller functions)
        callee-start (+ filler-size caller-size)
        helper-offset (+ callee-start (native-function-size-x86 callee functions))
        starts (vector-push (vector-push (vector-new 2) filler-size) callee-start)
        result (ref-new (push-zero-bytes (vector-new 0) 0 filler-size))
        layout (make-x86-function-emit-layout 0 helper-offset filler-size filler-size)]
    (do
      (generate-native-function-x86-64-bundle-with-layout caller result starts functions layout)
      (print filler-size)
      (print callee-start)
      (print-bytes (ref-get result) 0 (vector-length (ref-get result)))
      0)))"#,
    );
    let mut lines = output.trim().lines();
    let filler_size = lines
        .next()
        .expect("filler size output")
        .parse::<usize>()
        .expect("filler size parse");
    let callee_start = lines
        .next()
        .expect("callee start output")
        .parse::<isize>()
        .expect("callee start parse");
    let bytes = lines
        .map(|line| {
            line.parse::<u8>().unwrap_or_else(|_| {
                panic!("x86 four-arg existing-stack call byte parse 失敗: {line}")
            })
        })
        .collect::<Vec<_>>();
    let call_offset = (filler_size..bytes.len())
        .find(|idx| {
            if bytes[*idx] != 0xe8 || *idx + 4 >= bytes.len() {
                return false;
            }
            let rel = i32::from_le_bytes([
                bytes[*idx + 1],
                bytes[*idx + 2],
                bytes[*idx + 3],
                bytes[*idx + 4],
            ]);
            let target = *idx as isize + 5 + rel as isize;
            target >= 0 && target <= bytes.len() as isize
        })
        .unwrap_or_else(|| panic!("x86 four-arg call が見つからない: bytes={bytes:?}"));
    let rel = i32::from_le_bytes([
        bytes[call_offset + 1],
        bytes[call_offset + 2],
        bytes[call_offset + 3],
        bytes[call_offset + 4],
    ]);
    let target = call_offset as isize + 5 + rel as isize;

    assert_eq!(
        target, callee_start,
        "x86 four-arg segmented emit は post-consume tail ではなく call 直後を基準に callee start を指すべき: call_offset={call_offset} rel={rel} target={target} callee_start={callee_start} bytes={bytes:?}"
    );
}

/// NATIVE-REAL-08f8: x86_64 の low-level function emit は helper offset も caller base でローカル化すること
#[test]
#[ignore]
fn test_native_codegen_x86_function_emit_localizes_helper_offset_for_existing_result() {
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

(defn push-zero-bytes [result idx len]
  (if (>= idx len)
    result
    (push-zero-bytes (vector-push result 144) (+ idx 1) len)))

(defn print-bytes [bytes idx n]
  (if (>= idx n)
    0
    (do
      (print (vector-get bytes idx))
      (print-bytes bytes (+ idx 1) n))))

(defn main []
  (let [filler-size 16
        caller-ir (vector-push (vector-new 1) (make-instr 67 0))
        caller (make-function-meta 0 0 caller-ir)
        functions (vector-push (vector-new 1) caller)
        starts (vector-push (vector-new 1) filler-size)
        helper-offset (+ filler-size (native-function-size-x86 caller functions))
        result (ref-new (push-zero-bytes (vector-new 0) 0 filler-size))]
    (do
      (generate-native-function-x86-64-bundle-with-import-count caller result starts functions 0 helper-offset)
      (print filler-size)
      (print helper-offset)
      (print-bytes (ref-get result) 0 (vector-length (ref-get result)))
      0)))"#,
    );
    let mut lines = output.trim().lines();
    let filler_size = lines
        .next()
        .expect("filler size output")
        .parse::<usize>()
        .expect("filler size parse");
    let helper_offset = lines
        .next()
        .expect("helper offset output")
        .parse::<isize>()
        .expect("helper offset parse");
    let bytes = lines
        .map(|line| {
            line.parse::<u8>()
                .unwrap_or_else(|_| panic!("x86 helper call byte parse 失敗: {line}"))
        })
        .collect::<Vec<_>>();
    let call_offset = (filler_size..bytes.len())
        .find(|idx| bytes[*idx] == 0xe8)
        .unwrap_or_else(|| panic!("x86 helper call が見つからない: bytes={bytes:?}"));
    let rel = i32::from_le_bytes([
        bytes[call_offset + 1],
        bytes[call_offset + 2],
        bytes[call_offset + 3],
        bytes[call_offset + 4],
    ]);
    let target = call_offset as isize + 5 + rel as isize;

    assert_eq!(
        target, helper_offset,
        "x86 helper call target は code 全体の helper offset を指すべき: call_offset={call_offset} rel={rel} target={target} helper_offset={helper_offset} bytes={bytes:?}"
    );
}
