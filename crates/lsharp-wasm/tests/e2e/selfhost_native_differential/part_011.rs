
/// NATIVE-REAL-08x: x86_64 で 18 引数 direct call bundle が 12 stack arg を持つこと
#[test]
#[ignore]
fn test_native_codegen_emits_x86_direct_call_eighteen_arg_bundle_bytes() {
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
                                                    (vector-push
                                                      (vector-push (vector-new 19) (make-instr 3 31))
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
                      (make-instr 3 3))
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
                                               (vector-push (vector-new 35) (make-local-get 0))
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
        callee-ir-next6 (vector-push
                          (vector-push callee-ir-next5 (make-local-get 16))
                          (make-instr 24 0))
        callee-ir (vector-push
                    (vector-push callee-ir-next6 (make-local-get 17))
                    (make-instr 24 0))
        caller (make-function-meta 0 0 caller-ir)
        callee (make-function-meta 18 0 callee-ir)
        functions (vector-push (vector-push (vector-new 2) caller) callee)
        starts (collect-function-starts-x86 functions)
        caller-end (vector-get starts 1)
        call-start (- caller-end 199)
        spill-start (+ caller-end 11)
        target (make-target 1)
        native (emit-native-function-meta-bundle functions target)
        n (vector-length native)]
    (do
      (print n)
      (print-range native call-start (+ call-start 190))
      (print-range native spill-start (+ spill-start 174))
      (print (vector-get native (- n 2)))
      (print (vector-get native (- n 1)))
      0)))"#,
    );

    let lines: Vec<&str> = output.trim().lines().collect();
    let expected = [
        "2554", "72", "129", "236", "96", "0", "0", "0", "72", "137", "68", "36", "88", "72",
        "137", "76", "36", "80", "72", "139", "141", "248", "255", "255", "255", "72", "137", "76",
        "36", "72", "72", "139", "141", "240", "255", "255", "255", "72", "137", "76", "36", "64",
        "72", "139", "141", "232", "255", "255", "255", "72", "137", "76", "36", "56", "72", "139",
        "141", "224", "255", "255", "255", "72", "137", "76", "36", "48", "72", "139", "141",
        "216", "255", "255", "255", "72", "137", "76", "36", "40", "72", "139", "141", "208",
        "255", "255", "255", "72", "137", "76", "36", "32", "72", "139", "141", "200", "255",
        "255", "255", "72", "137", "76", "36", "24", "72", "139", "141", "192", "255", "255",
        "255", "72", "137", "76", "36", "16", "72", "139", "141", "184", "255", "255", "255", "72",
        "137", "76", "36", "8", "76", "139", "141", "176", "255", "255", "255", "76", "137", "12",
        "36", "76", "139", "141", "168", "255", "255", "255", "76", "139", "133", "160", "255",
        "255", "255", "72", "139", "141", "152", "255", "255", "255", "72", "139", "149", "144",
        "255", "255", "255", "72", "139", "181", "136", "255", "255", "255", "72", "139", "189",
        "128", "255", "255", "255", "232", "16", "0", "0", "0", "72", "129", "196", "96", "0", "0",
        "0", "72", "137", "189", "248", "255", "255", "255", "72", "137", "181", "240", "255",
        "255", "255", "72", "137", "149", "232", "255", "255", "255", "72", "137", "141", "224",
        "255", "255", "255", "76", "137", "133", "216", "255", "255", "255", "76", "137", "141",
        "208", "255", "255", "255", "72", "139", "69", "16", "72", "137", "133", "200", "255",
        "255", "255", "72", "139", "69", "24", "72", "137", "133", "192", "255", "255", "255",
        "72", "139", "69", "32", "72", "137", "133", "184", "255", "255", "255", "72", "139", "69",
        "40", "72", "137", "133", "176", "255", "255", "255", "72", "139", "69", "48", "72", "137",
        "133", "168", "255", "255", "255", "72", "139", "69", "56", "72", "137", "133", "160",
        "255", "255", "255", "72", "139", "69", "64", "72", "137", "133", "152", "255", "255",
        "255", "72", "139", "69", "72", "72", "137", "133", "144", "255", "255", "255", "72",
        "139", "69", "80", "72", "137", "133", "136", "255", "255", "255", "72", "139", "69", "88",
        "72", "137", "133", "128", "255", "255", "255", "72", "139", "69", "96", "72", "137",
        "133", "120", "255", "255", "255", "72", "139", "69", "104", "72", "137", "133", "112",
        "255", "255", "255", "93", "195",
    ];

    assert!(
        lines.len() >= expected.len(),
        "x86 direct call eighteen-arg bundle bytes 出力が不足: {:?}",
        lines
    );
    assert_eq!(
        &lines[..expected.len()],
        expected,
        "x86_64 direct call eighteen-arg bundle payload/call-layout exact bytes が一致しない"
    );
}

/// NATIVE-REAL-08y: x86_64 で 19 引数 direct call bundle が 13 stack arg を持つこと
#[test]
#[ignore]
fn test_native_codegen_emits_x86_direct_call_nineteen_arg_bundle_bytes() {
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
                                                    (vector-push
                                                      (vector-push
                                                        (vector-push (vector-new 20) (make-instr 3 31))
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
                        (make-instr 3 3))
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
                                               (vector-push (vector-new 37) (make-local-get 0))
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
        callee-ir-next6 (vector-push
                          (vector-push callee-ir-next5 (make-local-get 16))
                          (make-instr 24 0))
        callee-ir-next7 (vector-push
                          (vector-push callee-ir-next6 (make-local-get 17))
                          (make-instr 24 0))
        callee-ir (vector-push
                    (vector-push callee-ir-next7 (make-local-get 18))
                    (make-instr 24 0))
        caller (make-function-meta 0 0 caller-ir)
        callee (make-function-meta 19 0 callee-ir)
        functions (vector-push (vector-push (vector-new 2) caller) callee)
        starts (collect-function-starts-x86 functions)
        caller-end (vector-get starts 1)
        call-start (- caller-end 211)
        spill-start (+ caller-end 11)
        target (make-target 1)
        native (emit-native-function-meta-bundle functions target)
        n (vector-length native)]
    (do
      (print n)
      (print-range native call-start (+ call-start 202))
      (print-range native spill-start (+ spill-start 185))
      (print (vector-get native (- n 2)))
      (print (vector-get native (- n 1)))
      0)))"#,
    );

    let lines: Vec<&str> = output.trim().lines().collect();
    let expected = [
        "2828", "72", "129", "236", "112", "0", "0", "0", "72", "137", "68", "36", "96", "72",
        "137", "76", "36", "88", "72", "139", "141", "248", "255", "255", "255", "72", "137", "76",
        "36", "80", "72", "139", "141", "240", "255", "255", "255", "72", "137", "76", "36", "72",
        "72", "139", "141", "232", "255", "255", "255", "72", "137", "76", "36", "64", "72", "139",
        "141", "224", "255", "255", "255", "72", "137", "76", "36", "56", "72", "139", "141",
        "216", "255", "255", "255", "72", "137", "76", "36", "48", "72", "139", "141", "208",
        "255", "255", "255", "72", "137", "76", "36", "40", "72", "139", "141", "200", "255",
        "255", "255", "72", "137", "76", "36", "32", "72", "139", "141", "192", "255", "255",
        "255", "72", "137", "76", "36", "24", "72", "139", "141", "184", "255", "255", "255", "72",
        "137", "76", "36", "16", "72", "139", "141", "176", "255", "255", "255", "72", "137", "76",
        "36", "8", "76", "139", "141", "168", "255", "255", "255", "76", "137", "12", "36", "76",
        "139", "141", "160", "255", "255", "255", "76", "139", "133", "152", "255", "255", "255",
        "72", "139", "141", "144", "255", "255", "255", "72", "139", "149", "136", "255", "255",
        "255", "72", "139", "181", "128", "255", "255", "255", "72", "139", "189", "120", "255",
        "255", "255", "232", "16", "0", "0", "0", "72", "129", "196", "112", "0", "0", "0", "72",
        "137", "189", "248", "255", "255", "255", "72", "137", "181", "240", "255", "255", "255",
        "72", "137", "149", "232", "255", "255", "255", "72", "137", "141", "224", "255", "255",
        "255", "76", "137", "133", "216", "255", "255", "255", "76", "137", "141", "208", "255",
        "255", "255", "72", "139", "69", "16", "72", "137", "133", "200", "255", "255", "255",
        "72", "139", "69", "24", "72", "137", "133", "192", "255", "255", "255", "72", "139", "69",
        "32", "72", "137", "133", "184", "255", "255", "255", "72", "139", "69", "40", "72", "137",
        "133", "176", "255", "255", "255", "72", "139", "69", "48", "72", "137", "133", "168",
        "255", "255", "255", "72", "139", "69", "56", "72", "137", "133", "160", "255", "255",
        "255", "72", "139", "69", "64", "72", "137", "133", "152", "255", "255", "255", "72",
        "139", "69", "72", "72", "137", "133", "144", "255", "255", "255", "72", "139", "69", "80",
        "72", "137", "133", "136", "255", "255", "255", "72", "139", "69", "88", "72", "137",
        "133", "128", "255", "255", "255", "72", "139", "69", "96", "72", "137", "133", "120",
        "255", "255", "255", "72", "139", "69", "104", "72", "137", "133", "112", "255", "255",
        "255", "72", "139", "69", "112", "72", "137", "133", "104", "255", "255", "255", "93",
        "195",
    ];

    assert!(
        lines.len() >= expected.len(),
        "x86 direct call nineteen-arg bundle bytes 出力が不足: {:?}",
        lines
    );
    assert_eq!(
        &lines[..expected.len()],
        expected,
        "x86_64 direct call nineteen-arg bundle payload/call-layout exact bytes が一致しない"
    );
}

/// NATIVE-REAL-08z: x86_64 で 20 引数 direct call bundle が 14 stack arg を持つこと
#[test]
#[ignore]
fn test_native_codegen_emits_x86_direct_call_twenty_arg_bundle_bytes() {
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
  (let [caller-ir0 (vector-push (vector-new 21) (make-instr 3 31))
        caller-ir1 (vector-push caller-ir0 (make-instr 3 2))
        caller-ir2 (vector-push caller-ir1 (make-instr 3 3))
        caller-ir3 (vector-push caller-ir2 (make-instr 3 5))
        caller-ir4 (vector-push caller-ir3 (make-instr 3 7))
        caller-ir5 (vector-push caller-ir4 (make-instr 3 11))
        caller-ir6 (vector-push caller-ir5 (make-instr 3 13))
        caller-ir7 (vector-push caller-ir6 (make-instr 3 14))
        caller-ir8 (vector-push caller-ir7 (make-instr 3 17))
        caller-ir9 (vector-push caller-ir8 (make-instr 3 19))
        caller-ir10 (vector-push caller-ir9 (make-instr 3 23))
        caller-ir11 (vector-push caller-ir10 (make-instr 3 29))
        caller-ir12 (vector-push caller-ir11 (make-instr 3 31))
        caller-ir13 (vector-push caller-ir12 (make-instr 3 37))
        caller-ir14 (vector-push caller-ir13 (make-instr 3 1))
        caller-ir15 (vector-push caller-ir14 (make-instr 3 2))
        caller-ir16 (vector-push caller-ir15 (make-instr 3 4))
        caller-ir17 (vector-push caller-ir16 (make-instr 3 3))
        caller-ir18 (vector-push caller-ir17 (make-instr 3 1))
        caller-ir19 (vector-push caller-ir18 (make-instr 3 1))
        caller-ir (vector-push caller-ir19 (make-call 1))
        callee-ir0 (vector-push (vector-new 39) (make-local-get 0))
        callee-ir1 (vector-push callee-ir0 (make-local-get 1))
        callee-ir2 (vector-push callee-ir1 (make-instr 24 0))
        callee-ir3 (vector-push callee-ir2 (make-local-get 2))
        callee-ir4 (vector-push callee-ir3 (make-instr 24 0))
        callee-ir5 (vector-push callee-ir4 (make-local-get 3))
        callee-ir6 (vector-push callee-ir5 (make-instr 24 0))
        callee-ir7 (vector-push callee-ir6 (make-local-get 4))
        callee-ir8 (vector-push callee-ir7 (make-instr 24 0))
        callee-ir9 (vector-push callee-ir8 (make-local-get 5))
        callee-ir10 (vector-push callee-ir9 (make-instr 24 0))
        callee-ir11 (vector-push callee-ir10 (make-local-get 6))
        callee-ir12 (vector-push callee-ir11 (make-instr 24 0))
        callee-ir13 (vector-push callee-ir12 (make-local-get 7))
        callee-ir14 (vector-push callee-ir13 (make-instr 24 0))
        callee-ir15 (vector-push callee-ir14 (make-local-get 8))
        callee-ir16 (vector-push callee-ir15 (make-instr 24 0))
        callee-ir17 (vector-push callee-ir16 (make-local-get 9))
        callee-ir18 (vector-push callee-ir17 (make-instr 24 0))
        callee-ir19 (vector-push callee-ir18 (make-local-get 10))
        callee-ir20 (vector-push callee-ir19 (make-instr 24 0))
        callee-ir21 (vector-push callee-ir20 (make-local-get 11))
        callee-ir22 (vector-push callee-ir21 (make-instr 24 0))
        callee-ir23 (vector-push callee-ir22 (make-local-get 12))
        callee-ir24 (vector-push callee-ir23 (make-instr 24 0))
        callee-ir25 (vector-push callee-ir24 (make-local-get 13))
        callee-ir26 (vector-push callee-ir25 (make-instr 24 0))
        callee-ir27 (vector-push callee-ir26 (make-local-get 14))
        callee-ir28 (vector-push callee-ir27 (make-instr 24 0))
        callee-ir29 (vector-push callee-ir28 (make-local-get 15))
        callee-ir30 (vector-push callee-ir29 (make-instr 24 0))
        callee-ir31 (vector-push callee-ir30 (make-local-get 16))
        callee-ir32 (vector-push callee-ir31 (make-instr 24 0))
        callee-ir33 (vector-push callee-ir32 (make-local-get 17))
        callee-ir34 (vector-push callee-ir33 (make-instr 24 0))
        callee-ir35 (vector-push callee-ir34 (make-local-get 18))
        callee-ir36 (vector-push callee-ir35 (make-instr 24 0))
        callee-ir37 (vector-push callee-ir36 (make-local-get 19))
        callee-ir (vector-push callee-ir37 (make-instr 24 0))
        caller (make-function-meta 0 0 caller-ir)
        callee (make-function-meta 20 0 callee-ir)
        functions (vector-push (vector-push (vector-new 2) caller) callee)
        starts (collect-function-starts-x86 functions)
        caller-end (vector-get starts 1)
        call-start (- caller-end 223)
        spill-start (+ caller-end 11)
        target (make-target 1)
        native (emit-native-function-meta-bundle functions target)
        n (vector-length native)]
    (do
      (print n)
      (print-range native call-start (+ call-start 214))
      (print-range native spill-start (+ spill-start 196))
      (print (vector-get native (- n 2)))
      (print (vector-get native (- n 1)))
      0)))"#,
    );

    let lines: Vec<&str> = output.trim().lines().collect();
    let expected = [
        "3116", "72", "129", "236", "112", "0", "0", "0", "72", "137", "68", "36", "104", "72",
        "137", "76", "36", "96", "72", "139", "141", "248", "255", "255", "255", "72", "137", "76",
        "36", "88", "72", "139", "141", "240", "255", "255", "255", "72", "137", "76", "36", "80",
        "72", "139", "141", "232", "255", "255", "255", "72", "137", "76", "36", "72", "72", "139",
        "141", "224", "255", "255", "255", "72", "137", "76", "36", "64", "72", "139", "141",
        "216", "255", "255", "255", "72", "137", "76", "36", "56", "72", "139", "141", "208",
        "255", "255", "255", "72", "137", "76", "36", "48", "72", "139", "141", "200", "255",
        "255", "255", "72", "137", "76", "36", "40", "72", "139", "141", "192", "255", "255",
        "255", "72", "137", "76", "36", "32", "72", "139", "141", "184", "255", "255", "255", "72",
        "137", "76", "36", "24", "72", "139", "141", "176", "255", "255", "255", "72", "137", "76",
        "36", "16", "72", "139", "141", "168", "255", "255", "255", "72", "137", "76", "36", "8",
        "76", "139", "141", "160", "255", "255", "255", "76", "137", "12", "36", "76", "139",
        "141", "152", "255", "255", "255", "76", "139", "133", "144", "255", "255", "255", "72",
        "139", "141", "136", "255", "255", "255", "72", "139", "149", "128", "255", "255", "255",
        "72", "139", "181", "120", "255", "255", "255", "72", "139", "189", "112", "255", "255",
        "255", "232", "16", "0", "0", "0", "72", "129", "196", "112", "0", "0", "0", "72", "137",
        "189", "248", "255", "255", "255", "72", "137", "181", "240", "255", "255", "255", "72",
        "137", "149", "232", "255", "255", "255", "72", "137", "141", "224", "255", "255", "255",
        "76", "137", "133", "216", "255", "255", "255", "76", "137", "141", "208", "255", "255",
        "255", "72", "139", "69", "16", "72", "137", "133", "200", "255", "255", "255", "72",
        "139", "69", "24", "72", "137", "133", "192", "255", "255", "255", "72", "139", "69", "32",
        "72", "137", "133", "184", "255", "255", "255", "72", "139", "69", "40", "72", "137",
        "133", "176", "255", "255", "255", "72", "139", "69", "48", "72", "137", "133", "168",
        "255", "255", "255", "72", "139", "69", "56", "72", "137", "133", "160", "255", "255",
        "255", "72", "139", "69", "64", "72", "137", "133", "152", "255", "255", "255", "72",
        "139", "69", "72", "72", "137", "133", "144", "255", "255", "255", "72", "139", "69", "80",
        "72", "137", "133", "136", "255", "255", "255", "72", "139", "69", "88", "72", "137",
        "133", "128", "255", "255", "255", "72", "139", "69", "96", "72", "137", "133", "120",
        "255", "255", "255", "72", "139", "69", "104", "72", "137", "133", "112", "255", "255",
        "255", "72", "139", "69", "112", "72", "137", "133", "104", "255", "255", "255", "72",
        "139", "69", "120", "72", "137", "133", "96", "255", "255", "255", "93", "195",
    ];

    assert!(
        lines.len() >= expected.len(),
        "x86 direct call twenty-arg bundle bytes 出力が不足: {:?}",
        lines
    );
    assert_eq!(
        &lines[..expected.len()],
        expected,
        "x86_64 direct call twenty-arg bundle payload/call-layout exact bytes が一致しない"
    );
}

/// NATIVE-REAL-08za: x86_64 で 21 引数 direct call bundle が 15 stack arg を持つこと
#[test]
#[ignore]
fn test_native_codegen_emits_x86_direct_call_twenty_one_arg_bundle_bytes() {
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
  (let [caller-ir0 (vector-push (vector-new 22) (make-instr 3 31))
        caller-ir1 (vector-push caller-ir0 (make-instr 3 2))
        caller-ir2 (vector-push caller-ir1 (make-instr 3 3))
        caller-ir3 (vector-push caller-ir2 (make-instr 3 5))
        caller-ir4 (vector-push caller-ir3 (make-instr 3 7))
        caller-ir5 (vector-push caller-ir4 (make-instr 3 11))
        caller-ir6 (vector-push caller-ir5 (make-instr 3 13))
        caller-ir7 (vector-push caller-ir6 (make-instr 3 14))
        caller-ir8 (vector-push caller-ir7 (make-instr 3 17))
        caller-ir9 (vector-push caller-ir8 (make-instr 3 19))
        caller-ir10 (vector-push caller-ir9 (make-instr 3 23))
        caller-ir11 (vector-push caller-ir10 (make-instr 3 29))
        caller-ir12 (vector-push caller-ir11 (make-instr 3 31))
        caller-ir13 (vector-push caller-ir12 (make-instr 3 37))
        caller-ir14 (vector-push caller-ir13 (make-instr 3 1))
        caller-ir15 (vector-push caller-ir14 (make-instr 3 2))
        caller-ir16 (vector-push caller-ir15 (make-instr 3 4))
        caller-ir17 (vector-push caller-ir16 (make-instr 3 3))
        caller-ir18 (vector-push caller-ir17 (make-instr 3 1))
        caller-ir19 (vector-push caller-ir18 (make-instr 3 1))
        caller-ir20 (vector-push caller-ir19 (make-instr 3 1))
        caller-ir (vector-push caller-ir20 (make-call 1))
        callee-ir0 (vector-push (vector-new 41) (make-local-get 0))
        callee-ir1 (vector-push callee-ir0 (make-local-get 1))
        callee-ir2 (vector-push callee-ir1 (make-instr 24 0))
        callee-ir3 (vector-push callee-ir2 (make-local-get 2))
        callee-ir4 (vector-push callee-ir3 (make-instr 24 0))
        callee-ir5 (vector-push callee-ir4 (make-local-get 3))
        callee-ir6 (vector-push callee-ir5 (make-instr 24 0))
        callee-ir7 (vector-push callee-ir6 (make-local-get 4))
        callee-ir8 (vector-push callee-ir7 (make-instr 24 0))
        callee-ir9 (vector-push callee-ir8 (make-local-get 5))
        callee-ir10 (vector-push callee-ir9 (make-instr 24 0))
        callee-ir11 (vector-push callee-ir10 (make-local-get 6))
        callee-ir12 (vector-push callee-ir11 (make-instr 24 0))
        callee-ir13 (vector-push callee-ir12 (make-local-get 7))
        callee-ir14 (vector-push callee-ir13 (make-instr 24 0))
        callee-ir15 (vector-push callee-ir14 (make-local-get 8))
        callee-ir16 (vector-push callee-ir15 (make-instr 24 0))
        callee-ir17 (vector-push callee-ir16 (make-local-get 9))
        callee-ir18 (vector-push callee-ir17 (make-instr 24 0))
        callee-ir19 (vector-push callee-ir18 (make-local-get 10))
        callee-ir20 (vector-push callee-ir19 (make-instr 24 0))
        callee-ir21 (vector-push callee-ir20 (make-local-get 11))
        callee-ir22 (vector-push callee-ir21 (make-instr 24 0))
        callee-ir23 (vector-push callee-ir22 (make-local-get 12))
        callee-ir24 (vector-push callee-ir23 (make-instr 24 0))
        callee-ir25 (vector-push callee-ir24 (make-local-get 13))
        callee-ir26 (vector-push callee-ir25 (make-instr 24 0))
        callee-ir27 (vector-push callee-ir26 (make-local-get 14))
        callee-ir28 (vector-push callee-ir27 (make-instr 24 0))
        callee-ir29 (vector-push callee-ir28 (make-local-get 15))
        callee-ir30 (vector-push callee-ir29 (make-instr 24 0))
        callee-ir31 (vector-push callee-ir30 (make-local-get 16))
        callee-ir32 (vector-push callee-ir31 (make-instr 24 0))
        callee-ir33 (vector-push callee-ir32 (make-local-get 17))
        callee-ir34 (vector-push callee-ir33 (make-instr 24 0))
        callee-ir35 (vector-push callee-ir34 (make-local-get 18))
        callee-ir36 (vector-push callee-ir35 (make-instr 24 0))
        callee-ir37 (vector-push callee-ir36 (make-local-get 19))
        callee-ir38 (vector-push callee-ir37 (make-instr 24 0))
        callee-ir39 (vector-push callee-ir38 (make-local-get 20))
        callee-ir (vector-push callee-ir39 (make-instr 24 0))
        caller (make-function-meta 0 0 caller-ir)
        callee (make-function-meta 21 0 callee-ir)
        functions (vector-push (vector-push (vector-new 2) caller) callee)
        starts (collect-function-starts-x86 functions)
        caller-end (vector-get starts 1)
        call-start (- caller-end 235)
        spill-start (+ caller-end 11)
        target (make-target 1)
        native (emit-native-function-meta-bundle functions target)
        n (vector-length native)]
    (do
      (print n)
      (print-range native call-start (+ call-start 226))
      (print-range native spill-start (+ spill-start 210))
      (print (vector-get native (- n 2)))
      (print (vector-get native (- n 1)))
      0)))"#,
    );

    let lines: Vec<&str> = output.trim().lines().collect();
    let expected = [
        "3421", "72", "129", "236", "128", "0", "0", "0", "72", "137", "68", "36", "112", "72",
        "137", "76", "36", "104", "72", "139", "141", "248", "255", "255", "255", "72", "137",
        "76", "36", "96", "72", "139", "141", "240", "255", "255", "255", "72", "137", "76", "36",
        "88", "72", "139", "141", "232", "255", "255", "255", "72", "137", "76", "36", "80", "72",
        "139", "141", "224", "255", "255", "255", "72", "137", "76", "36", "72", "72", "139",
        "141", "216", "255", "255", "255", "72", "137", "76", "36", "64", "72", "139", "141",
        "208", "255", "255", "255", "72", "137", "76", "36", "56", "72", "139", "141", "200",
        "255", "255", "255", "72", "137", "76", "36", "48", "72", "139", "141", "192", "255",
        "255", "255", "72", "137", "76", "36", "40", "72", "139", "141", "184", "255", "255",
        "255", "72", "137", "76", "36", "32", "72", "139", "141", "176", "255", "255", "255", "72",
        "137", "76", "36", "24", "72", "139", "141", "168", "255", "255", "255", "72", "137", "76",
        "36", "16", "72", "139", "141", "160", "255", "255", "255", "72", "137", "76", "36", "8",
        "76", "139", "141", "152", "255", "255", "255", "76", "137", "12", "36", "76", "139",
        "141", "144", "255", "255", "255", "76", "139", "133", "136", "255", "255", "255", "72",
        "139", "141", "128", "255", "255", "255", "72", "139", "149", "120", "255", "255", "255",
        "72", "139", "181", "112", "255", "255", "255", "72", "139", "189", "104", "255", "255",
        "255", "232", "16", "0", "0", "0", "72", "129", "196", "128", "0", "0", "0", "72", "137",
        "189", "248", "255", "255", "255", "72", "137", "181", "240", "255", "255", "255", "72",
        "137", "149", "232", "255", "255", "255", "72", "137", "141", "224", "255", "255", "255",
        "76", "137", "133", "216", "255", "255", "255", "76", "137", "141", "208", "255", "255",
        "255", "72", "139", "69", "16", "72", "137", "133", "200", "255", "255", "255", "72",
        "139", "69", "24", "72", "137", "133", "192", "255", "255", "255", "72", "139", "69", "32",
        "72", "137", "133", "184", "255", "255", "255", "72", "139", "69", "40", "72", "137",
        "133", "176", "255", "255", "255", "72", "139", "69", "48", "72", "137", "133", "168",
        "255", "255", "255", "72", "139", "69", "56", "72", "137", "133", "160", "255", "255",
        "255", "72", "139", "69", "64", "72", "137", "133", "152", "255", "255", "255", "72",
        "139", "69", "72", "72", "137", "133", "144", "255", "255", "255", "72", "139", "69", "80",
        "72", "137", "133", "136", "255", "255", "255", "72", "139", "69", "88", "72", "137",
        "133", "128", "255", "255", "255", "72", "139", "69", "96", "72", "137", "133", "120",
        "255", "255", "255", "72", "139", "69", "104", "72", "137", "133", "112", "255", "255",
        "255", "72", "139", "69", "112", "72", "137", "133", "104", "255", "255", "255", "72",
        "139", "69", "120", "72", "137", "133", "96", "255", "255", "255", "72", "139", "133",
        "128", "0", "0", "0", "72", "137", "133", "88", "255", "255", "255", "93", "195",
    ];

    assert!(
        lines.len() >= expected.len(),
        "x86 direct call twenty-one-arg bundle bytes 出力が不足: {:?}",
        lines
    );
    assert_eq!(
        &lines[..expected.len()],
        expected,
        "x86_64 direct call twenty-one-arg bundle payload/call-layout exact bytes が一致しない"
    );
}
