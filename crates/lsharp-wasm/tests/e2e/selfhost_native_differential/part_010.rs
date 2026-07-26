
/// NATIVE-REAL-08t: x86_64 で 14 引数 direct call bundle が 8 stack arg を持つこと
#[test]
#[ignore]
fn test_native_codegen_emits_x86_direct_call_fourteen_arg_bundle_bytes() {
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
                                      (vector-push
                                        (vector-push
                                          (vector-push
                                            (vector-push
                                              (vector-push (vector-new 15) (make-instr 3 31))
                                              (make-instr 3 2))
                                            (make-instr 3 3))
                                          (make-instr 3 5))
                                        (make-instr 3 7))
                                      (make-instr 3 11))
                                    (make-instr 3 13))
                                  (make-instr 3 14))
                                (make-instr 3 17))
                              (make-instr 3 19))
                            (make-instr 3 23))
                          (make-instr 3 29))
                        (make-instr 3 31))
                      (make-instr 3 37))
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
                                             (vector-push
                                               (vector-push (vector-new 27) (make-local-get 0))
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
                         (make-instr 24 0))
        callee-ir-mid (vector-push
                        (vector-push callee-ir-head (make-local-get 7))
                        (make-instr 24 0))
        callee-ir-tail (vector-push
                         (vector-push callee-ir-mid (make-local-get 8))
                         (make-instr 24 0))
        callee-ir-more (vector-push
                         (vector-push callee-ir-tail (make-local-get 9))
                         (make-instr 24 0))
        callee-ir-last (vector-push
                         (vector-push callee-ir-more (make-local-get 10))
                         (make-instr 24 0))
        callee-ir-next (vector-push
                         (vector-push callee-ir-last (make-local-get 11))
                         (make-instr 24 0))
        callee-ir-next2 (vector-push
                          (vector-push callee-ir-next (make-local-get 12))
                          (make-instr 24 0))
        callee-ir (vector-push
                    (vector-push callee-ir-next2 (make-local-get 13))
                    (make-instr 24 0))
        caller (make-function-meta 0 0 caller-ir)
        callee (make-function-meta 14 0 callee-ir)
        functions (vector-push (vector-push (vector-new 2) caller) callee)
        starts (collect-function-starts-x86 functions)
        caller-end (vector-get starts 1)
        call-start (- caller-end 151)
        spill-start (+ caller-end 11)
        target (make-target 1)
        native (emit-native-function-meta-bundle functions target)
        n (vector-length native)]
    (do
      (print n)
      (print-range native call-start (+ call-start 142))
      (print-range native spill-start (+ spill-start 130))
      (print (vector-get native (- n 2)))
      (print (vector-get native (- n 1)))
      0)))"#,
    );

    let lines: Vec<&str> = output.trim().lines().collect();
    let expected = [
        "1598", "72", "129", "236", "64", "0", "0", "0", "72", "137", "68", "36", "56", "72",
        "137", "76", "36", "48", "72", "139", "141", "248", "255", "255", "255", "72", "137", "76",
        "36", "40", "72", "139", "141", "240", "255", "255", "255", "72", "137", "76", "36", "32",
        "72", "139", "141", "232", "255", "255", "255", "72", "137", "76", "36", "24", "72", "139",
        "141", "224", "255", "255", "255", "72", "137", "76", "36", "16", "72", "139", "141",
        "216", "255", "255", "255", "72", "137", "76", "36", "8", "76", "139", "141", "208", "255",
        "255", "255", "76", "137", "12", "36", "76", "139", "141", "200", "255", "255", "255",
        "76", "139", "133", "192", "255", "255", "255", "72", "139", "141", "184", "255", "255",
        "255", "72", "139", "149", "176", "255", "255", "255", "72", "139", "181", "168", "255",
        "255", "255", "72", "139", "189", "160", "255", "255", "255", "232", "16", "0", "0", "0",
        "72", "129", "196", "64", "0", "0", "0", "72", "137", "189", "248", "255", "255", "255",
        "72", "137", "181", "240", "255", "255", "255", "72", "137", "149", "232", "255", "255",
        "255", "72", "137", "141", "224", "255", "255", "255", "76", "137", "133", "216", "255",
        "255", "255", "76", "137", "141", "208", "255", "255", "255", "72", "139", "69", "16",
        "72", "137", "133", "200", "255", "255", "255", "72", "139", "69", "24", "72", "137",
        "133", "192", "255", "255", "255", "72", "139", "69", "32", "72", "137", "133", "184",
        "255", "255", "255", "72", "139", "69", "40", "72", "137", "133", "176", "255", "255",
        "255", "72", "139", "69", "48", "72", "137", "133", "168", "255", "255", "255", "72",
        "139", "69", "56", "72", "137", "133", "160", "255", "255", "255", "72", "139", "69", "64",
        "72", "137", "133", "152", "255", "255", "255", "72", "139", "69", "72", "72", "137",
        "133", "144", "255", "255", "255", "93", "195",
    ];

    assert!(
        lines.len() >= expected.len(),
        "x86 direct call fourteen-arg bundle bytes 出力が不足: {:?}",
        lines
    );
    assert_eq!(
        &lines[..expected.len()],
        expected,
        "x86_64 direct call fourteen-arg bundle payload/call-layout exact bytes が一致しない"
    );
}

/// NATIVE-REAL-08u: x86_64 で 15 引数 direct call bundle が 9 stack arg を持つこと
#[test]
#[ignore]
fn test_native_codegen_emits_x86_direct_call_fifteen_arg_bundle_bytes() {
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
                                      (vector-push
                                        (vector-push
                                          (vector-push
                                            (vector-push
                                              (vector-push
                                                (vector-push (vector-new 16) (make-instr 3 31))
                                                (make-instr 3 2))
                                              (make-instr 3 3))
                                            (make-instr 3 5))
                                          (make-instr 3 7))
                                        (make-instr 3 11))
                                      (make-instr 3 13))
                                    (make-instr 3 14))
                                  (make-instr 3 17))
                                (make-instr 3 19))
                              (make-instr 3 23))
                            (make-instr 3 29))
                          (make-instr 3 31))
                        (make-instr 3 37))
                      (make-instr 3 1))
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
                                             (vector-push
                                               (vector-push (vector-new 29) (make-local-get 0))
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
                         (make-instr 24 0))
        callee-ir-mid (vector-push
                        (vector-push callee-ir-head (make-local-get 7))
                        (make-instr 24 0))
        callee-ir-tail (vector-push
                         (vector-push callee-ir-mid (make-local-get 8))
                         (make-instr 24 0))
        callee-ir-more (vector-push
                         (vector-push callee-ir-tail (make-local-get 9))
                         (make-instr 24 0))
        callee-ir-last (vector-push
                         (vector-push callee-ir-more (make-local-get 10))
                         (make-instr 24 0))
        callee-ir-next (vector-push
                         (vector-push callee-ir-last (make-local-get 11))
                         (make-instr 24 0))
        callee-ir-next2 (vector-push
                          (vector-push callee-ir-next (make-local-get 12))
                          (make-instr 24 0))
        callee-ir-next3 (vector-push
                          (vector-push callee-ir-next2 (make-local-get 13))
                          (make-instr 24 0))
        callee-ir (vector-push
                    (vector-push callee-ir-next3 (make-local-get 14))
                    (make-instr 24 0))
        caller (make-function-meta 0 0 caller-ir)
        callee (make-function-meta 15 0 callee-ir)
        functions (vector-push (vector-push (vector-new 2) caller) callee)
        starts (collect-function-starts-x86 functions)
        caller-end (vector-get starts 1)
        call-start (- caller-end 163)
        spill-start (+ caller-end 11)
        target (make-target 1)
        native (emit-native-function-meta-bundle functions target)
        n (vector-length native)]
    (do
      (print n)
      (print-range native call-start (+ call-start 154))
      (print-range native spill-start (+ spill-start 141))
      (print (vector-get native (- n 2)))
      (print (vector-get native (- n 1)))
      0)))"#,
    );

    let lines: Vec<&str> = output.trim().lines().collect();
    let expected = [
        "1816", "72", "129", "236", "80", "0", "0", "0", "72", "137", "68", "36", "64", "72",
        "137", "76", "36", "56", "72", "139", "141", "248", "255", "255", "255", "72", "137", "76",
        "36", "48", "72", "139", "141", "240", "255", "255", "255", "72", "137", "76", "36", "40",
        "72", "139", "141", "232", "255", "255", "255", "72", "137", "76", "36", "32", "72", "139",
        "141", "224", "255", "255", "255", "72", "137", "76", "36", "24", "72", "139", "141",
        "216", "255", "255", "255", "72", "137", "76", "36", "16", "72", "139", "141", "208",
        "255", "255", "255", "72", "137", "76", "36", "8", "76", "139", "141", "200", "255", "255",
        "255", "76", "137", "12", "36", "76", "139", "141", "192", "255", "255", "255", "76",
        "139", "133", "184", "255", "255", "255", "72", "139", "141", "176", "255", "255", "255",
        "72", "139", "149", "168", "255", "255", "255", "72", "139", "181", "160", "255", "255",
        "255", "72", "139", "189", "152", "255", "255", "255", "232", "16", "0", "0", "0", "72",
        "129", "196", "80", "0", "0", "0", "72", "137", "189", "248", "255", "255", "255", "72",
        "137", "181", "240", "255", "255", "255", "72", "137", "149", "232", "255", "255", "255",
        "72", "137", "141", "224", "255", "255", "255", "76", "137", "133", "216", "255", "255",
        "255", "76", "137", "141", "208", "255", "255", "255", "72", "139", "69", "16", "72",
        "137", "133", "200", "255", "255", "255", "72", "139", "69", "24", "72", "137", "133",
        "192", "255", "255", "255", "72", "139", "69", "32", "72", "137", "133", "184", "255",
        "255", "255", "72", "139", "69", "40", "72", "137", "133", "176", "255", "255", "255",
        "72", "139", "69", "48", "72", "137", "133", "168", "255", "255", "255", "72", "139", "69",
        "56", "72", "137", "133", "160", "255", "255", "255", "72", "139", "69", "64", "72", "137",
        "133", "152", "255", "255", "255", "72", "139", "69", "72", "72", "137", "133", "144",
        "255", "255", "255", "72", "139", "69", "80", "72", "137", "133", "136", "255", "255",
        "255", "93", "195",
    ];

    assert!(
        lines.len() >= expected.len(),
        "x86 direct call fifteen-arg bundle bytes 出力が不足: {:?}",
        lines
    );
    assert_eq!(
        &lines[..expected.len()],
        expected,
        "x86_64 direct call fifteen-arg bundle payload/call-layout exact bytes が一致しない"
    );
}

/// NATIVE-REAL-08v: x86_64 で 16 引数 direct call bundle が 10 stack arg を持つこと
#[test]
#[ignore]
fn test_native_codegen_emits_x86_direct_call_sixteen_arg_bundle_bytes() {
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
                                      (vector-push
                                        (vector-push
                                          (vector-push
                                            (vector-push
                                              (vector-push
                                                (vector-push
                                                  (vector-push (vector-new 17) (make-instr 3 31))
                                                  (make-instr 3 2))
                                                (make-instr 3 3))
                                              (make-instr 3 5))
                                            (make-instr 3 7))
                                          (make-instr 3 11))
                                        (make-instr 3 13))
                                      (make-instr 3 14))
                                    (make-instr 3 17))
                                  (make-instr 3 19))
                                (make-instr 3 23))
                              (make-instr 3 29))
                            (make-instr 3 31))
                          (make-instr 3 37))
                        (make-instr 3 1))
                      (make-instr 3 2))
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
                                             (vector-push
                                               (vector-push (vector-new 31) (make-local-get 0))
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
                         (make-instr 24 0))
        callee-ir-mid (vector-push
                        (vector-push callee-ir-head (make-local-get 7))
                        (make-instr 24 0))
        callee-ir-tail (vector-push
                         (vector-push callee-ir-mid (make-local-get 8))
                         (make-instr 24 0))
        callee-ir-more (vector-push
                         (vector-push callee-ir-tail (make-local-get 9))
                         (make-instr 24 0))
        callee-ir-last (vector-push
                         (vector-push callee-ir-more (make-local-get 10))
                         (make-instr 24 0))
        callee-ir-next (vector-push
                         (vector-push callee-ir-last (make-local-get 11))
                         (make-instr 24 0))
        callee-ir-next2 (vector-push
                          (vector-push callee-ir-next (make-local-get 12))
                          (make-instr 24 0))
        callee-ir-next3 (vector-push
                          (vector-push callee-ir-next2 (make-local-get 13))
                          (make-instr 24 0))
        callee-ir-next4 (vector-push
                          (vector-push callee-ir-next3 (make-local-get 14))
                          (make-instr 24 0))
        callee-ir (vector-push
                    (vector-push callee-ir-next4 (make-local-get 15))
                    (make-instr 24 0))
        caller (make-function-meta 0 0 caller-ir)
        callee (make-function-meta 16 0 callee-ir)
        functions (vector-push (vector-push (vector-new 2) caller) callee)
        starts (collect-function-starts-x86 functions)
        caller-end (vector-get starts 1)
        call-start (- caller-end 175)
        spill-start (+ caller-end 11)
        target (make-target 1)
        native (emit-native-function-meta-bundle functions target)
        n (vector-length native)]
    (do
      (print n)
      (print-range native call-start (+ call-start 166))
      (print-range native spill-start (+ spill-start 152))
      (print (vector-get native (- n 2)))
      (print (vector-get native (- n 1)))
      0)))"#,
    );

    let lines: Vec<&str> = output.trim().lines().collect();
    let expected = [
        "2048", "72", "129", "236", "80", "0", "0", "0", "72", "137", "68", "36", "72", "72",
        "137", "76", "36", "64", "72", "139", "141", "248", "255", "255", "255", "72", "137", "76",
        "36", "56", "72", "139", "141", "240", "255", "255", "255", "72", "137", "76", "36", "48",
        "72", "139", "141", "232", "255", "255", "255", "72", "137", "76", "36", "40", "72", "139",
        "141", "224", "255", "255", "255", "72", "137", "76", "36", "32", "72", "139", "141",
        "216", "255", "255", "255", "72", "137", "76", "36", "24", "72", "139", "141", "208",
        "255", "255", "255", "72", "137", "76", "36", "16", "72", "139", "141", "200", "255",
        "255", "255", "72", "137", "76", "36", "8", "76", "139", "141", "192", "255", "255", "255",
        "76", "137", "12", "36", "76", "139", "141", "184", "255", "255", "255", "76", "139",
        "133", "176", "255", "255", "255", "72", "139", "141", "168", "255", "255", "255", "72",
        "139", "149", "160", "255", "255", "255", "72", "139", "181", "152", "255", "255", "255",
        "72", "139", "189", "144", "255", "255", "255", "232", "16", "0", "0", "0", "72", "129",
        "196", "80", "0", "0", "0", "72", "137", "189", "248", "255", "255", "255", "72", "137",
        "181", "240", "255", "255", "255", "72", "137", "149", "232", "255", "255", "255", "72",
        "137", "141", "224", "255", "255", "255", "76", "137", "133", "216", "255", "255", "255",
        "76", "137", "141", "208", "255", "255", "255", "72", "139", "69", "16", "72", "137",
        "133", "200", "255", "255", "255", "72", "139", "69", "24", "72", "137", "133", "192",
        "255", "255", "255", "72", "139", "69", "32", "72", "137", "133", "184", "255", "255",
        "255", "72", "139", "69", "40", "72", "137", "133", "176", "255", "255", "255", "72",
        "139", "69", "48", "72", "137", "133", "168", "255", "255", "255", "72", "139", "69", "56",
        "72", "137", "133", "160", "255", "255", "255", "72", "139", "69", "64", "72", "137",
        "133", "152", "255", "255", "255", "72", "139", "69", "72", "72", "137", "133", "144",
        "255", "255", "255", "72", "139", "69", "80", "72", "137", "133", "136", "255", "255",
        "255", "72", "139", "69", "88", "72", "137", "133", "128", "255", "255", "255", "93",
        "195",
    ];

    assert!(
        lines.len() >= expected.len(),
        "x86 direct call sixteen-arg bundle bytes 出力が不足: {:?}",
        lines
    );
    assert_eq!(
        &lines[..expected.len()],
        expected,
        "x86_64 direct call sixteen-arg bundle payload/call-layout exact bytes が一致しない"
    );
}

/// NATIVE-REAL-08w: x86_64 で 17 引数 direct call bundle が 11 stack arg を持つこと
#[test]
#[ignore]
fn test_native_codegen_emits_x86_direct_call_seventeen_arg_bundle_bytes() {
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
                                      (vector-push
                                        (vector-push
                                          (vector-push
                                            (vector-push
                                              (vector-push
                                                (vector-push
                                                  (vector-push
                                                    (vector-push (vector-new 18) (make-instr 3 31))
                                                    (make-instr 3 2))
                                                  (make-instr 3 3))
                                                (make-instr 3 5))
                                              (make-instr 3 7))
                                            (make-instr 3 11))
                                          (make-instr 3 13))
                                        (make-instr 3 14))
                                      (make-instr 3 17))
                                    (make-instr 3 19))
                                  (make-instr 3 23))
                                (make-instr 3 29))
                              (make-instr 3 31))
                            (make-instr 3 37))
                          (make-instr 3 1))
                        (make-instr 3 2))
                      (make-instr 3 4))
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
                                             (vector-push
                                               (vector-push (vector-new 33) (make-local-get 0))
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
                         (make-instr 24 0))
        callee-ir-mid (vector-push
                        (vector-push callee-ir-head (make-local-get 7))
                        (make-instr 24 0))
        callee-ir-tail (vector-push
                         (vector-push callee-ir-mid (make-local-get 8))
                         (make-instr 24 0))
        callee-ir-more (vector-push
                         (vector-push callee-ir-tail (make-local-get 9))
                         (make-instr 24 0))
        callee-ir-last (vector-push
                         (vector-push callee-ir-more (make-local-get 10))
                         (make-instr 24 0))
        callee-ir-next (vector-push
                         (vector-push callee-ir-last (make-local-get 11))
                         (make-instr 24 0))
        callee-ir-next2 (vector-push
                          (vector-push callee-ir-next (make-local-get 12))
                          (make-instr 24 0))
        callee-ir-next3 (vector-push
                          (vector-push callee-ir-next2 (make-local-get 13))
                          (make-instr 24 0))
        callee-ir-next4 (vector-push
                          (vector-push callee-ir-next3 (make-local-get 14))
                          (make-instr 24 0))
        callee-ir-next5 (vector-push
                          (vector-push callee-ir-next4 (make-local-get 15))
                          (make-instr 24 0))
        callee-ir (vector-push
                    (vector-push callee-ir-next5 (make-local-get 16))
                    (make-instr 24 0))
        caller (make-function-meta 0 0 caller-ir)
        callee (make-function-meta 17 0 callee-ir)
        functions (vector-push (vector-push (vector-new 2) caller) callee)
        starts (collect-function-starts-x86 functions)
        caller-end (vector-get starts 1)
        call-start (- caller-end 187)
        spill-start (+ caller-end 11)
        target (make-target 1)
        native (emit-native-function-meta-bundle functions target)
        n (vector-length native)]
    (do
      (print n)
      (print-range native call-start (+ call-start 178))
      (print-range native spill-start (+ spill-start 163))
      (print (vector-get native (- n 2)))
      (print (vector-get native (- n 1)))
      0)))"#,
    );

    let lines: Vec<&str> = output.trim().lines().collect();
    let expected = [
        "2294", "72", "129", "236", "96", "0", "0", "0", "72", "137", "68", "36", "80", "72",
        "137", "76", "36", "72", "72", "139", "141", "248", "255", "255", "255", "72", "137", "76",
        "36", "64", "72", "139", "141", "240", "255", "255", "255", "72", "137", "76", "36", "56",
        "72", "139", "141", "232", "255", "255", "255", "72", "137", "76", "36", "48", "72", "139",
        "141", "224", "255", "255", "255", "72", "137", "76", "36", "40", "72", "139", "141",
        "216", "255", "255", "255", "72", "137", "76", "36", "32", "72", "139", "141", "208",
        "255", "255", "255", "72", "137", "76", "36", "24", "72", "139", "141", "200", "255",
        "255", "255", "72", "137", "76", "36", "16", "72", "139", "141", "192", "255", "255",
        "255", "72", "137", "76", "36", "8", "76", "139", "141", "184", "255", "255", "255", "76",
        "137", "12", "36", "76", "139", "141", "176", "255", "255", "255", "76", "139", "133",
        "168", "255", "255", "255", "72", "139", "141", "160", "255", "255", "255", "72", "139",
        "149", "152", "255", "255", "255", "72", "139", "181", "144", "255", "255", "255", "72",
        "139", "189", "136", "255", "255", "255", "232", "16", "0", "0", "0", "72", "129", "196",
        "96", "0", "0", "0", "72", "137", "189", "248", "255", "255", "255", "72", "137", "181",
        "240", "255", "255", "255", "72", "137", "149", "232", "255", "255", "255", "72", "137",
        "141", "224", "255", "255", "255", "76", "137", "133", "216", "255", "255", "255", "76",
        "137", "141", "208", "255", "255", "255", "72", "139", "69", "16", "72", "137", "133",
        "200", "255", "255", "255", "72", "139", "69", "24", "72", "137", "133", "192", "255",
        "255", "255", "72", "139", "69", "32", "72", "137", "133", "184", "255", "255", "255",
        "72", "139", "69", "40", "72", "137", "133", "176", "255", "255", "255", "72", "139", "69",
        "48", "72", "137", "133", "168", "255", "255", "255", "72", "139", "69", "56", "72", "137",
        "133", "160", "255", "255", "255", "72", "139", "69", "64", "72", "137", "133", "152",
        "255", "255", "255", "72", "139", "69", "72", "72", "137", "133", "144", "255", "255",
        "255", "72", "139", "69", "80", "72", "137", "133", "136", "255", "255", "255", "72",
        "139", "69", "88", "72", "137", "133", "128", "255", "255", "255", "72", "139", "69", "96",
        "72", "137", "133", "120", "255", "255", "255", "93", "195",
    ];

    assert!(
        lines.len() >= expected.len(),
        "x86 direct call seventeen-arg bundle bytes 出力が不足: {:?}",
        lines
    );
    assert_eq!(
        &lines[..expected.len()],
        expected,
        "x86_64 direct call seventeen-arg bundle payload/call-layout exact bytes が一致しない"
    );
}
