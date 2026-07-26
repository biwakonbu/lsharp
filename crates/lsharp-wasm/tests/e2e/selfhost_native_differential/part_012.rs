
/// NATIVE-REAL-08zb: x86_64 で 22 引数 direct call bundle が 16 stack arg を持つこと
#[test]
#[ignore]
fn test_native_codegen_emits_x86_direct_call_twenty_two_arg_bundle_bytes() {
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
  (let [caller-ir0 (vector-push (vector-new 23) (make-instr 3 31))
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
        caller-ir21 (vector-push caller-ir20 (make-instr 3 2))
        caller-ir (vector-push caller-ir21 (make-call 1))
        callee-ir0 (vector-push (vector-new 43) (make-local-get 0))
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
        callee-ir40 (vector-push callee-ir39 (make-instr 24 0))
        callee-ir41 (vector-push callee-ir40 (make-local-get 21))
        callee-ir (vector-push callee-ir41 (make-instr 24 0))
        caller (make-function-meta 0 0 caller-ir)
        callee (make-function-meta 22 0 callee-ir)
        functions (vector-push (vector-push (vector-new 2) caller) callee)
        starts (collect-function-starts-x86 functions)
        caller-end (vector-get starts 1)
        call-start (- caller-end 247)
        spill-start (+ caller-end 11)
        target (make-target 1)
        native (emit-native-function-meta-bundle functions target)
        n (vector-length native)]
    (do
      (print n)
      (print-range native call-start (+ call-start 238))
      (print-range native spill-start (+ spill-start 224))
      (print (vector-get native (- n 2)))
      (print (vector-get native (- n 1)))
      0)))"#,
    );

    let lines: Vec<&str> = output.trim().lines().collect();
    let expected = [
        "3740", "72", "129", "236", "128", "0", "0", "0", "72", "137", "68", "36", "120", "72",
        "137", "76", "36", "112", "72", "139", "141", "248", "255", "255", "255", "72", "137",
        "76", "36", "104", "72", "139", "141", "240", "255", "255", "255", "72", "137", "76", "36",
        "96", "72", "139", "141", "232", "255", "255", "255", "72", "137", "76", "36", "88", "72",
        "139", "141", "224", "255", "255", "255", "72", "137", "76", "36", "80", "72", "139",
        "141", "216", "255", "255", "255", "72", "137", "76", "36", "72", "72", "139", "141",
        "208", "255", "255", "255", "72", "137", "76", "36", "64", "72", "139", "141", "200",
        "255", "255", "255", "72", "137", "76", "36", "56", "72", "139", "141", "192", "255",
        "255", "255", "72", "137", "76", "36", "48", "72", "139", "141", "184", "255", "255",
        "255", "72", "137", "76", "36", "40", "72", "139", "141", "176", "255", "255", "255", "72",
        "137", "76", "36", "32", "72", "139", "141", "168", "255", "255", "255", "72", "137", "76",
        "36", "24", "72", "139", "141", "160", "255", "255", "255", "72", "137", "76", "36", "16",
        "72", "139", "141", "152", "255", "255", "255", "72", "137", "76", "36", "8", "76", "139",
        "141", "144", "255", "255", "255", "76", "137", "12", "36", "76", "139", "141", "136",
        "255", "255", "255", "76", "139", "133", "128", "255", "255", "255", "72", "139", "141",
        "120", "255", "255", "255", "72", "139", "149", "112", "255", "255", "255", "72", "139",
        "181", "104", "255", "255", "255", "72", "139", "189", "96", "255", "255", "255", "232",
        "16", "0", "0", "0", "72", "129", "196", "128", "0", "0", "0", "72", "137", "189", "248",
        "255", "255", "255", "72", "137", "181", "240", "255", "255", "255", "72", "137", "149",
        "232", "255", "255", "255", "72", "137", "141", "224", "255", "255", "255", "76", "137",
        "133", "216", "255", "255", "255", "76", "137", "141", "208", "255", "255", "255", "72",
        "139", "69", "16", "72", "137", "133", "200", "255", "255", "255", "72", "139", "69", "24",
        "72", "137", "133", "192", "255", "255", "255", "72", "139", "69", "32", "72", "137",
        "133", "184", "255", "255", "255", "72", "139", "69", "40", "72", "137", "133", "176",
        "255", "255", "255", "72", "139", "69", "48", "72", "137", "133", "168", "255", "255",
        "255", "72", "139", "69", "56", "72", "137", "133", "160", "255", "255", "255", "72",
        "139", "69", "64", "72", "137", "133", "152", "255", "255", "255", "72", "139", "69", "72",
        "72", "137", "133", "144", "255", "255", "255", "72", "139", "69", "80", "72", "137",
        "133", "136", "255", "255", "255", "72", "139", "69", "88", "72", "137", "133", "128",
        "255", "255", "255", "72", "139", "69", "96", "72", "137", "133", "120", "255", "255",
        "255", "72", "139", "69", "104", "72", "137", "133", "112", "255", "255", "255", "72",
        "139", "69", "112", "72", "137", "133", "104", "255", "255", "255", "72", "139", "69",
        "120", "72", "137", "133", "96", "255", "255", "255", "72", "139", "133", "128", "0", "0",
        "0", "72", "137", "133", "88", "255", "255", "255", "72", "139", "133", "136", "0", "0",
        "0", "72", "137", "133", "80", "255", "255", "255", "93", "195",
    ];

    assert!(
        lines.len() >= expected.len(),
        "x86 direct call twenty-two-arg bundle bytes 出力が不足: {:?}",
        lines
    );
    assert_eq!(
        &lines[..expected.len()],
        expected,
        "x86_64 direct call twenty-two-arg bundle payload/call-layout exact bytes が一致しない"
    );
}

/// NATIVE-REAL-08zc: x86_64 で 23 引数 direct call bundle helper が 17 stack arg / spill 23 を生成できること
#[test]
#[ignore]
fn test_native_codegen_emits_x86_direct_call_twenty_three_arg_bundle_bytes() {
    let output = run_native_codegen_harness(
        r#"(module Main)
(import NativeCodegen)

(defn print-range [bytes idx end]
  (if (>= idx end)
    0
    (do
      (print (vector-get bytes idx))
      (print-range bytes (+ idx 1) end))))

(defn main []
  (let [call-bytes (emit-twenty-three-arg-call-x86 16 0)
        spill-ref (ref-new (vector-new 8))]
    (do
      (spill-native-function-params-x86-twenty-to-twenty-three 23 spill-ref)
      (print (vector-length call-bytes))
      (print-range call-bytes 0 (vector-length call-bytes))
      (print (vector-length (ref-get spill-ref)))
      (print-range (ref-get spill-ref) 0 (vector-length (ref-get spill-ref)))
      0)))"#,
    );
    let lines: Vec<&str> = output.trim().lines().collect();
    let expected = [
        "253", "72", "129", "236", "144", "0", "0", "0", "72", "137", "132", "36", "128", "0", "0",
        "0", "72", "137", "76", "36", "120", "72", "139", "141", "248", "255", "255", "255", "72",
        "137", "76", "36", "112", "72", "139", "141", "240", "255", "255", "255", "72", "137",
        "76", "36", "104", "72", "139", "141", "232", "255", "255", "255", "72", "137", "76", "36",
        "96", "72", "139", "141", "224", "255", "255", "255", "72", "137", "76", "36", "88", "72",
        "139", "141", "216", "255", "255", "255", "72", "137", "76", "36", "80", "72", "139",
        "141", "208", "255", "255", "255", "72", "137", "76", "36", "72", "72", "139", "141",
        "200", "255", "255", "255", "72", "137", "76", "36", "64", "72", "139", "141", "192",
        "255", "255", "255", "72", "137", "76", "36", "56", "72", "139", "141", "184", "255",
        "255", "255", "72", "137", "76", "36", "48", "72", "139", "141", "176", "255", "255",
        "255", "72", "137", "76", "36", "40", "72", "139", "141", "168", "255", "255", "255", "72",
        "137", "76", "36", "32", "72", "139", "141", "160", "255", "255", "255", "72", "137", "76",
        "36", "24", "72", "139", "141", "152", "255", "255", "255", "72", "137", "76", "36", "16",
        "72", "139", "141", "144", "255", "255", "255", "72", "137", "76", "36", "8", "76", "139",
        "141", "136", "255", "255", "255", "76", "137", "12", "36", "76", "139", "141", "128",
        "255", "255", "255", "76", "139", "133", "120", "255", "255", "255", "72", "139", "141",
        "112", "255", "255", "255", "72", "139", "149", "104", "255", "255", "255", "72", "139",
        "181", "96", "255", "255", "255", "72", "139", "189", "88", "255", "255", "255", "232",
        "16", "0", "0", "0", "72", "129", "196", "144", "0", "0", "0", "238", "72", "137", "189",
        "248", "255", "255", "255", "72", "137", "181", "240", "255", "255", "255", "72", "137",
        "149", "232", "255", "255", "255", "72", "137", "141", "224", "255", "255", "255", "76",
        "137", "133", "216", "255", "255", "255", "76", "137", "141", "208", "255", "255", "255",
        "72", "139", "69", "16", "72", "137", "133", "200", "255", "255", "255", "72", "139", "69",
        "24", "72", "137", "133", "192", "255", "255", "255", "72", "139", "69", "32", "72", "137",
        "133", "184", "255", "255", "255", "72", "139", "69", "40", "72", "137", "133", "176",
        "255", "255", "255", "72", "139", "69", "48", "72", "137", "133", "168", "255", "255",
        "255", "72", "139", "69", "56", "72", "137", "133", "160", "255", "255", "255", "72",
        "139", "69", "64", "72", "137", "133", "152", "255", "255", "255", "72", "139", "69", "72",
        "72", "137", "133", "144", "255", "255", "255", "72", "139", "69", "80", "72", "137",
        "133", "136", "255", "255", "255", "72", "139", "69", "88", "72", "137", "133", "128",
        "255", "255", "255", "72", "139", "69", "96", "72", "137", "133", "120", "255", "255",
        "255", "72", "139", "69", "104", "72", "137", "133", "112", "255", "255", "255", "72",
        "139", "69", "112", "72", "137", "133", "104", "255", "255", "255", "72", "139", "69",
        "120", "72", "137", "133", "96", "255", "255", "255", "72", "139", "133", "128", "0", "0",
        "0", "72", "137", "133", "88", "255", "255", "255", "72", "139", "133", "136", "0", "0",
        "0", "72", "137", "133", "80", "255", "255", "255", "72", "139", "133", "144", "0", "0",
        "0", "72", "137", "133", "72", "255", "255", "255",
    ];

    assert!(
        lines.len() >= expected.len(),
        "x86 direct call twenty-three-arg helper bytes 出力が不足: {:?}",
        lines
    );
    assert_eq!(
        &lines[..expected.len()],
        expected,
        "x86_64 direct call twenty-three-arg helper payload/call-layout exact bytes が一致しない"
    );
}

/// NATIVE-REAL-08zd: x86_64 で 24 引数 direct call bundle helper が 18 stack arg / spill 24 を生成できること
#[test]
#[ignore]
fn test_native_codegen_emits_x86_direct_call_twenty_four_arg_bundle_bytes() {
    let output = run_native_codegen_harness(
        r#"(module Main)
(import NativeCodegen)

(defn print-range [bytes idx end]
  (if (>= idx end)
    0
    (do
      (print (vector-get bytes idx))
      (print-range bytes (+ idx 1) end))))

(defn main []
  (let [call-bytes (emit-twenty-four-arg-call-x86 16 0)
        spill-ref (ref-new (vector-new 8))]
    (do
      (spill-native-function-params-x86-twenty-to-twenty-four 24 spill-ref)
      (print (vector-length call-bytes))
      (print-range call-bytes 0 (vector-length call-bytes))
      (print (vector-length (ref-get spill-ref)))
      (print-range (ref-get spill-ref) 0 (vector-length (ref-get spill-ref)))
      0)))"#,
    );

    let lines: Vec<&str> = output.trim().lines().collect();
    let expected = [
        "268", "72", "129", "236", "144", "0", "0", "0", "72", "137", "132", "36", "136", "0", "0",
        "0", "72", "137", "140", "36", "128", "0", "0", "0", "72", "139", "141", "248", "255",
        "255", "255", "72", "137", "76", "36", "120", "72", "139", "141", "240", "255", "255",
        "255", "72", "137", "76", "36", "112", "72", "139", "141", "232", "255", "255", "255",
        "72", "137", "76", "36", "104", "72", "139", "141", "224", "255", "255", "255", "72",
        "137", "76", "36", "96", "72", "139", "141", "216", "255", "255", "255", "72", "137", "76",
        "36", "88", "72", "139", "141", "208", "255", "255", "255", "72", "137", "76", "36", "80",
        "72", "139", "141", "200", "255", "255", "255", "72", "137", "76", "36", "72", "72", "139",
        "141", "192", "255", "255", "255", "72", "137", "76", "36", "64", "72", "139", "141",
        "184", "255", "255", "255", "72", "137", "76", "36", "56", "72", "139", "141", "176",
        "255", "255", "255", "72", "137", "76", "36", "48", "72", "139", "141", "168", "255",
        "255", "255", "72", "137", "76", "36", "40", "72", "139", "141", "160", "255", "255",
        "255", "72", "137", "76", "36", "32", "72", "139", "141", "152", "255", "255", "255", "72",
        "137", "76", "36", "24", "72", "139", "141", "144", "255", "255", "255", "72", "137", "76",
        "36", "16", "72", "139", "141", "136", "255", "255", "255", "72", "137", "76", "36", "8",
        "76", "139", "141", "128", "255", "255", "255", "76", "137", "12", "36", "76", "139",
        "141", "120", "255", "255", "255", "76", "139", "133", "112", "255", "255", "255", "72",
        "139", "141", "104", "255", "255", "255", "72", "139", "149", "96", "255", "255", "255",
        "72", "139", "181", "88", "255", "255", "255", "72", "139", "189", "80", "255", "255",
        "255", "232", "16", "0", "0", "0", "72", "129", "196", "144", "0", "0", "0", "252", "72",
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
        "255", "72", "139", "69", "112", "72", "137", "133", "104", "255", "255", "255", "72",
        "139", "69", "120", "72", "137", "133", "96", "255", "255", "255", "72", "139", "133",
        "128", "0", "0", "0", "72", "137", "133", "88", "255", "255", "255", "72", "139", "133",
        "136", "0", "0", "0", "72", "137", "133", "80", "255", "255", "255", "72", "139", "133",
        "144", "0", "0", "0", "72", "137", "133", "72", "255", "255", "255", "72", "139", "133",
        "152", "0", "0", "0", "72", "137", "133", "64", "255", "255", "255",
    ];

    assert!(
        lines.len() >= expected.len(),
        "x86 direct call twenty-four-arg helper bytes 出力が不足: {:?}",
        lines
    );
    assert_eq!(
        &lines[..expected.len()],
        expected,
        "x86_64 direct call twenty-four-arg helper payload/call-layout exact bytes が一致しない"
    );
}

/// NATIVE-REAL-08ze: x86_64 で 25 引数 direct call bundle helper が 19 stack arg / spill 25 を生成できること
#[test]
#[ignore]
fn test_native_codegen_emits_x86_direct_call_twenty_five_arg_bundle_bytes() {
    let output = run_native_codegen_harness(
        r#"(module Main)
(import NativeCodegen)

(defn print-range [bytes idx end]
  (if (>= idx end)
    0
    (do
      (print (vector-get bytes idx))
      (print-range bytes (+ idx 1) end))))

(defn main []
  (let [call-bytes (emit-twenty-five-arg-call-x86 16 0)
        spill-ref (ref-new (vector-new 8))]
    (do
      (spill-native-function-params-x86-twenty-to-twenty-five 25 spill-ref)
      (print (vector-length call-bytes))
      (print-range call-bytes 0 (vector-length call-bytes))
      (print (vector-length (ref-get spill-ref)))
      (print-range (ref-get spill-ref) 0 (vector-length (ref-get spill-ref)))
      0)))"#,
    );

    let lines: Vec<&str> = output.trim().lines().collect();
    let expected = [
        "283", "72", "129", "236", "160", "0", "0", "0", "72", "137", "132", "36", "144", "0", "0",
        "0", "72", "137", "140", "36", "136", "0", "0", "0", "72", "139", "141", "248", "255",
        "255", "255", "72", "137", "140", "36", "128", "0", "0", "0", "72", "139", "141", "240",
        "255", "255", "255", "72", "137", "76", "36", "120", "72", "139", "141", "232", "255",
        "255", "255", "72", "137", "76", "36", "112", "72", "139", "141", "224", "255", "255",
        "255", "72", "137", "76", "36", "104", "72", "139", "141", "216", "255", "255", "255",
        "72", "137", "76", "36", "96", "72", "139", "141", "208", "255", "255", "255", "72", "137",
        "76", "36", "88", "72", "139", "141", "200", "255", "255", "255", "72", "137", "76", "36",
        "80", "72", "139", "141", "192", "255", "255", "255", "72", "137", "76", "36", "72", "72",
        "139", "141", "184", "255", "255", "255", "72", "137", "76", "36", "64", "72", "139",
        "141", "176", "255", "255", "255", "72", "137", "76", "36", "56", "72", "139", "141",
        "168", "255", "255", "255", "72", "137", "76", "36", "48", "72", "139", "141", "160",
        "255", "255", "255", "72", "137", "76", "36", "40", "72", "139", "141", "152", "255",
        "255", "255", "72", "137", "76", "36", "32", "72", "139", "141", "144", "255", "255",
        "255", "72", "137", "76", "36", "24", "72", "139", "141", "136", "255", "255", "255", "72",
        "137", "76", "36", "16", "72", "139", "141", "128", "255", "255", "255", "72", "137", "76",
        "36", "8", "76", "139", "141", "120", "255", "255", "255", "76", "137", "12", "36", "76",
        "139", "141", "112", "255", "255", "255", "76", "139", "133", "104", "255", "255", "255",
        "72", "139", "141", "96", "255", "255", "255", "72", "139", "149", "88", "255", "255",
        "255", "72", "139", "181", "80", "255", "255", "255", "72", "139", "189", "72", "255",
        "255", "255", "232", "16", "0", "0", "0", "72", "129", "196", "160", "0", "0", "0", "266",
        "72", "137", "189", "248", "255", "255", "255", "72", "137", "181", "240", "255", "255",
        "255", "72", "137", "149", "232", "255", "255", "255", "72", "137", "141", "224", "255",
        "255", "255", "76", "137", "133", "216", "255", "255", "255", "76", "137", "141", "208",
        "255", "255", "255", "72", "139", "69", "16", "72", "137", "133", "200", "255", "255",
        "255", "72", "139", "69", "24", "72", "137", "133", "192", "255", "255", "255", "72",
        "139", "69", "32", "72", "137", "133", "184", "255", "255", "255", "72", "139", "69", "40",
        "72", "137", "133", "176", "255", "255", "255", "72", "139", "69", "48", "72", "137",
        "133", "168", "255", "255", "255", "72", "139", "69", "56", "72", "137", "133", "160",
        "255", "255", "255", "72", "139", "69", "64", "72", "137", "133", "152", "255", "255",
        "255", "72", "139", "69", "72", "72", "137", "133", "144", "255", "255", "255", "72",
        "139", "69", "80", "72", "137", "133", "136", "255", "255", "255", "72", "139", "69", "88",
        "72", "137", "133", "128", "255", "255", "255", "72", "139", "69", "96", "72", "137",
        "133", "120", "255", "255", "255", "72", "139", "69", "104", "72", "137", "133", "112",
        "255", "255", "255", "72", "139", "69", "112", "72", "137", "133", "104", "255", "255",
        "255", "72", "139", "69", "120", "72", "137", "133", "96", "255", "255", "255", "72",
        "139", "133", "128", "0", "0", "0", "72", "137", "133", "88", "255", "255", "255", "72",
        "139", "133", "136", "0", "0", "0", "72", "137", "133", "80", "255", "255", "255", "72",
        "139", "133", "144", "0", "0", "0", "72", "137", "133", "72", "255", "255", "255", "72",
        "139", "133", "152", "0", "0", "0", "72", "137", "133", "64", "255", "255", "255", "72",
        "139", "133", "160", "0", "0", "0", "72", "137", "133", "56", "255", "255", "255",
    ];

    assert!(
        lines.len() >= expected.len(),
        "x86 direct call twenty-five-arg helper bytes 出力が不足: {:?}",
        lines
    );
    assert_eq!(
        &lines[..expected.len()],
        expected,
        "x86_64 direct call twenty-five-arg helper payload/call-layout exact bytes が一致しない"
    );
}

/// NATIVE-REAL-08zf: x86_64 で 26 引数 direct call bundle helper が 20 stack arg / spill 26 を生成できること
#[test]
#[ignore]
fn test_native_codegen_emits_x86_direct_call_twenty_six_arg_bundle_bytes() {
    let output = run_native_codegen_harness(
        r#"(module Main)
(import NativeCodegen)

(defn print-range [bytes idx end]
  (if (>= idx end)
    0
    (do
      (print (vector-get bytes idx))
      (print-range bytes (+ idx 1) end))))

(defn main []
  (let [call-bytes (emit-twenty-six-arg-call-x86 16 0)
        spill-ref (ref-new (vector-new 8))]
    (do
      (spill-native-function-params-x86-twenty-to-twenty-six 26 spill-ref)
      (print (vector-length call-bytes))
      (print-range call-bytes 0 (vector-length call-bytes))
      (print (vector-length (ref-get spill-ref)))
      (print-range (ref-get spill-ref) 0 (vector-length (ref-get spill-ref)))
      0)))"#,
    );

    let lines: Vec<&str> = output.trim().lines().collect();
    let expected: Vec<&str> = r#"298 72 129 236 160 0 0 0 72 137 132 36 152 0 0 0 72 137 140 36 144 0 0 0 72 139 141 248 255 255 255 72 137 140 36 136 0 0 0 72 139 141 240 255 255 255 72 137 140 36 128 0 0 0 72 139 141 232 255 255 255 72 137 76 36 120 72 139 141 224 255 255 255 72 137 76 36 112 72 139 141 216 255 255 255 72 137 76 36 104 72 139 141 208 255 255 255 72 137 76 36 96 72 139 141 200 255 255 255 72 137 76 36 88 72 139 141 192 255 255 255 72 137 76 36 80 72 139 141 184 255 255 255 72 137 76 36 72 72 139 141 176 255 255 255 72 137 76 36 64 72 139 141 168 255 255 255 72 137 76 36 56 72 139 141 160 255 255 255 72 137 76 36 48 72 139 141 152 255 255 255 72 137 76 36 40 72 139 141 144 255 255 255 72 137 76 36 32 72 139 141 136 255 255 255 72 137 76 36 24 72 139 141 128 255 255 255 72 137 76 36 16 72 139 141 120 255 255 255 72 137 76 36 8 76 139 141 112 255 255 255 76 137 12 36 76 139 141 104 255 255 255 76 139 133 96 255 255 255 72 139 141 88 255 255 255 72 139 149 80 255 255 255 72 139 181 72 255 255 255 72 139 189 64 255 255 255 232 16 0 0 0 72 129 196 160 0 0 0 280 72 137 189 248 255 255 255 72 137 181 240 255 255 255 72 137 149 232 255 255 255 72 137 141 224 255 255 255 76 137 133 216 255 255 255 76 137 141 208 255 255 255 72 139 69 16 72 137 133 200 255 255 255 72 139 69 24 72 137 133 192 255 255 255 72 139 69 32 72 137 133 184 255 255 255 72 139 69 40 72 137 133 176 255 255 255 72 139 69 48 72 137 133 168 255 255 255 72 139 69 56 72 137 133 160 255 255 255 72 139 69 64 72 137 133 152 255 255 255 72 139 69 72 72 137 133 144 255 255 255 72 139 69 80 72 137 133 136 255 255 255 72 139 69 88 72 137 133 128 255 255 255 72 139 69 96 72 137 133 120 255 255 255 72 139 69 104 72 137 133 112 255 255 255 72 139 69 112 72 137 133 104 255 255 255 72 139 69 120 72 137 133 96 255 255 255 72 139 133 128 0 0 0 72 137 133 88 255 255 255 72 139 133 136 0 0 0 72 137 133 80 255 255 255 72 139 133 144 0 0 0 72 137 133 72 255 255 255 72 139 133 152 0 0 0 72 137 133 64 255 255 255 72 139 133 160 0 0 0 72 137 133 56 255 255 255 72 139 133 168 0 0 0 72 137 133 48 255 255 255"#.split_whitespace().collect();
    assert!(
        lines.len() >= expected.len(),
        "x86 direct call twenty-six-arg helper bytes 出力が不足: {:?}",
        lines
    );
    assert_eq!(
        &lines[..expected.len()],
        expected.as_slice(),
        "x86_64 direct call twenty-six-arg helper payload/call-layout exact bytes が一致しない"
    );
}

/// NATIVE-REAL-08zg: x86_64 で 27 引数 direct call bundle helper が 21 stack arg / spill 27 を生成できること
#[test]
#[ignore]
fn test_native_codegen_emits_x86_direct_call_twenty_seven_arg_bundle_bytes() {
    let output = run_native_codegen_harness(
        r#"(module Main)
(import NativeCodegen)

(defn print-range [bytes idx end]
  (if (>= idx end)
    0
    (do
      (print (vector-get bytes idx))
      (print-range bytes (+ idx 1) end))))

(defn main []
  (let [call-bytes (emit-twenty-seven-arg-call-x86 16 0)
        spill-ref (ref-new (vector-new 8))]
    (do
      (spill-native-function-params-x86-twenty-to-twenty-seven 27 spill-ref)
      (print (vector-length call-bytes))
      (print-range call-bytes 0 (vector-length call-bytes))
      (print (vector-length (ref-get spill-ref)))
      (print-range (ref-get spill-ref) 0 (vector-length (ref-get spill-ref)))
      0)))"#,
    );

    let lines: Vec<&str> = output.trim().lines().collect();
    let expected: Vec<&str> = r#"313 72 129 236 176 0 0 0 72 137 132 36 160 0 0 0 72 137 140 36 152 0 0 0 72 139 141 248 255 255 255 72 137 140 36 144 0 0 0 72 139 141 240 255 255 255 72 137 140 36 136 0 0 0 72 139 141 232 255 255 255 72 137 140 36 128 0 0 0 72 139 141 224 255 255 255 72 137 76 36 120 72 139 141 216 255 255 255 72 137 76 36 112 72 139 141 208 255 255 255 72 137 76 36 104 72 139 141 200 255 255 255 72 137 76 36 96 72 139 141 192 255 255 255 72 137 76 36 88 72 139 141 184 255 255 255 72 137 76 36 80 72 139 141 176 255 255 255 72 137 76 36 72 72 139 141 168 255 255 255 72 137 76 36 64 72 139 141 160 255 255 255 72 137 76 36 56 72 139 141 152 255 255 255 72 137 76 36 48 72 139 141 144 255 255 255 72 137 76 36 40 72 139 141 136 255 255 255 72 137 76 36 32 72 139 141 128 255 255 255 72 137 76 36 24 72 139 141 120 255 255 255 72 137 76 36 16 72 139 141 112 255 255 255 72 137 76 36 8 76 139 141 104 255 255 255 76 137 12 36 76 139 141 96 255 255 255 76 139 133 88 255 255 255 72 139 141 80 255 255 255 72 139 149 72 255 255 255 72 139 181 64 255 255 255 72 139 189 56 255 255 255 232 16 0 0 0 72 129 196 176 0 0 0 294 72 137 189 248 255 255 255 72 137 181 240 255 255 255 72 137 149 232 255 255 255 72 137 141 224 255 255 255 76 137 133 216 255 255 255 76 137 141 208 255 255 255 72 139 69 16 72 137 133 200 255 255 255 72 139 69 24 72 137 133 192 255 255 255 72 139 69 32 72 137 133 184 255 255 255 72 139 69 40 72 137 133 176 255 255 255 72 139 69 48 72 137 133 168 255 255 255 72 139 69 56 72 137 133 160 255 255 255 72 139 69 64 72 137 133 152 255 255 255 72 139 69 72 72 137 133 144 255 255 255 72 139 69 80 72 137 133 136 255 255 255 72 139 69 88 72 137 133 128 255 255 255 72 139 69 96 72 137 133 120 255 255 255 72 139 69 104 72 137 133 112 255 255 255 72 139 69 112 72 137 133 104 255 255 255 72 139 69 120 72 137 133 96 255 255 255 72 139 133 128 0 0 0 72 137 133 88 255 255 255 72 139 133 136 0 0 0 72 137 133 80 255 255 255 72 139 133 144 0 0 0 72 137 133 72 255 255 255 72 139 133 152 0 0 0 72 137 133 64 255 255 255 72 139 133 160 0 0 0 72 137 133 56 255 255 255 72 139 133 168 0 0 0 72 137 133 48 255 255 255 72 139 133 176 0 0 0 72 137 133 40 255 255 255"#.split_whitespace().collect();
    assert!(
        lines.len() >= expected.len(),
        "x86 direct call twenty-seven-arg helper bytes 出力が不足: {:?}",
        lines
    );
    assert_eq!(
        &lines[..expected.len()],
        expected.as_slice(),
        "x86_64 direct call twenty-seven-arg helper payload/call-layout exact bytes が一致しない"
    );
}

/// NATIVE-REAL-08zh: x86_64 で 28 引数 direct call bundle helper が 22 stack arg / spill 28 を生成できること
#[test]
#[ignore]
fn test_native_codegen_emits_x86_direct_call_twenty_eight_arg_bundle_bytes() {
    let output = run_native_codegen_harness(
        r#"(module Main)
(import NativeCodegen)

(defn print-range [bytes idx end]
  (if (>= idx end)
    0
    (do
      (print (vector-get bytes idx))
      (print-range bytes (+ idx 1) end))))

(defn main []
  (let [call-bytes (emit-twenty-eight-arg-call-x86 16 0)
        spill-ref (ref-new (vector-new 8))]
    (do
      (spill-native-function-params-x86-twenty-to-twenty-eight 28 spill-ref)
      (print (vector-length call-bytes))
      (print-range call-bytes 0 (vector-length call-bytes))
      (print (vector-length (ref-get spill-ref)))
      (print-range (ref-get spill-ref) 0 (vector-length (ref-get spill-ref)))
      0)))"#,
    );

    let lines: Vec<&str> = output.trim().lines().collect();
    let expected: Vec<&str> = r#"328 72 129 236 176 0 0 0 72 137 132 36 168 0 0 0 72 137 140 36 160 0 0 0 72 139 141 248 255 255 255 72 137 140 36 152 0 0 0 72 139 141 240 255 255 255 72 137 140 36 144 0 0 0 72 139 141 232 255 255 255 72 137 140 36 136 0 0 0 72 139 141 224 255 255 255 72 137 140 36 128 0 0 0 72 139 141 216 255 255 255 72 137 76 36 120 72 139 141 208 255 255 255 72 137 76 36 112 72 139 141 200 255 255 255 72 137 76 36 104 72 139 141 192 255 255 255 72 137 76 36 96 72 139 141 184 255 255 255 72 137 76 36 88 72 139 141 176 255 255 255 72 137 76 36 80 72 139 141 168 255 255 255 72 137 76 36 72 72 139 141 160 255 255 255 72 137 76 36 64 72 139 141 152 255 255 255 72 137 76 36 56 72 139 141 144 255 255 255 72 137 76 36 48 72 139 141 136 255 255 255 72 137 76 36 40 72 139 141 128 255 255 255 72 137 76 36 32 72 139 141 120 255 255 255 72 137 76 36 24 72 139 141 112 255 255 255 72 137 76 36 16 72 139 141 104 255 255 255 72 137 76 36 8 76 139 141 96 255 255 255 76 137 12 36 76 139 141 88 255 255 255 76 139 133 80 255 255 255 72 139 141 72 255 255 255 72 139 149 64 255 255 255 72 139 181 56 255 255 255 72 139 189 48 255 255 255 232 16 0 0 0 72 129 196 176 0 0 0 308 72 137 189 248 255 255 255 72 137 181 240 255 255 255 72 137 149 232 255 255 255 72 137 141 224 255 255 255 76 137 133 216 255 255 255 76 137 141 208 255 255 255 72 139 69 16 72 137 133 200 255 255 255 72 139 69 24 72 137 133 192 255 255 255 72 139 69 32 72 137 133 184 255 255 255 72 139 69 40 72 137 133 176 255 255 255 72 139 69 48 72 137 133 168 255 255 255 72 139 69 56 72 137 133 160 255 255 255 72 139 69 64 72 137 133 152 255 255 255 72 139 69 72 72 137 133 144 255 255 255 72 139 69 80 72 137 133 136 255 255 255 72 139 69 88 72 137 133 128 255 255 255 72 139 69 96 72 137 133 120 255 255 255 72 139 69 104 72 137 133 112 255 255 255 72 139 69 112 72 137 133 104 255 255 255 72 139 69 120 72 137 133 96 255 255 255 72 139 133 128 0 0 0 72 137 133 88 255 255 255 72 139 133 136 0 0 0 72 137 133 80 255 255 255 72 139 133 144 0 0 0 72 137 133 72 255 255 255 72 139 133 152 0 0 0 72 137 133 64 255 255 255 72 139 133 160 0 0 0 72 137 133 56 255 255 255 72 139 133 168 0 0 0 72 137 133 48 255 255 255 72 139 133 176 0 0 0 72 137 133 40 255 255 255 72 139 133 184 0 0 0 72 137 133 32 255 255 255"#.split_whitespace().collect();
    assert!(
        lines.len() >= expected.len(),
        "x86 direct call twenty-eight-arg helper bytes 出力が不足: {:?}",
        lines
    );
    assert_eq!(
        &lines[..expected.len()],
        expected.as_slice(),
        "x86_64 direct call twenty-eight-arg helper payload/call-layout exact bytes が一致しない"
    );
}

/// NATIVE-REAL-08zi: x86_64 で 29 引数 direct call bundle helper が 23 stack arg / spill 29 を生成できること
#[test]
#[ignore]
fn test_native_codegen_emits_x86_direct_call_twenty_nine_arg_bundle_bytes() {
    let output = run_native_codegen_harness(
        r#"(module Main)
(import NativeCodegen)

(defn print-range [bytes idx end]
  (if (>= idx end)
    0
    (do
      (print (vector-get bytes idx))
      (print-range bytes (+ idx 1) end))))

    (defn main []
      (let [call-bytes (emit-twenty-nine-arg-call-x86 16 0)
            spill-ref (ref-new (vector-new 8))]
        (do
      (spill-native-function-params-x86-twenty-to-twenty-nine 29 spill-ref)
      (print (vector-length call-bytes))
      (print-range call-bytes 0 (vector-length call-bytes))
      (print (vector-length (ref-get spill-ref)))
          (print-range (ref-get spill-ref) 0 (vector-length (ref-get spill-ref)))
          0)))"#,
    );

    let lines: Vec<&str> = output.trim().lines().collect();
    let expected: Vec<&str> = r#"343 72 129 236 192 0 0 0 72 137 132 36 176 0 0 0 72 137 140 36 168 0 0 0 72 139 141 248 255 255 255 72 137 140 36 160 0 0 0 72 139 141 240 255 255 255 72 137 140 36 152 0 0 0 72 139 141 232 255 255 255 72 137 140 36 144 0 0 0 72 139 141 224 255 255 255 72 137 140 36 136 0 0 0 72 139 141 216 255 255 255 72 137 140 36 128 0 0 0 72 139 141 208 255 255 255 72 137 76 36 120 72 139 141 200 255 255 255 72 137 76 36 112 72 139 141 192 255 255 255 72 137 76 36 104 72 139 141 184 255 255 255 72 137 76 36 96 72 139 141 176 255 255 255 72 137 76 36 88 72 139 141 168 255 255 255 72 137 76 36 80 72 139 141 160 255 255 255 72 137 76 36 72 72 139 141 152 255 255 255 72 137 76 36 64 72 139 141 144 255 255 255 72 137 76 36 56 72 139 141 136 255 255 255 72 137 76 36 48 72 139 141 128 255 255 255 72 137 76 36 40 72 139 141 120 255 255 255 72 137 76 36 32 72 139 141 112 255 255 255 72 137 76 36 24 72 139 141 104 255 255 255 72 137 76 36 16 72 139 141 96 255 255 255 72 137 76 36 8 76 139 141 88 255 255 255 76 137 12 36 76 139 141 80 255 255 255 76 139 133 72 255 255 255 72 139 141 64 255 255 255 72 139 149 56 255 255 255 72 139 181 48 255 255 255 72 139 189 40 255 255 255 232 16 0 0 0 72 129 196 192 0 0 0 322 72 137 189 248 255 255 255 72 137 181 240 255 255 255 72 137 149 232 255 255 255 72 137 141 224 255 255 255 76 137 133 216 255 255 255 76 137 141 208 255 255 255 72 139 69 16 72 137 133 200 255 255 255 72 139 69 24 72 137 133 192 255 255 255 72 139 69 32 72 137 133 184 255 255 255 72 139 69 40 72 137 133 176 255 255 255 72 139 69 48 72 137 133 168 255 255 255 72 139 69 56 72 137 133 160 255 255 255 72 139 69 64 72 137 133 152 255 255 255 72 139 69 72 72 137 133 144 255 255 255 72 139 69 80 72 137 133 136 255 255 255 72 139 69 88 72 137 133 128 255 255 255 72 139 69 96 72 137 133 120 255 255 255 72 139 69 104 72 137 133 112 255 255 255 72 139 69 112 72 137 133 104 255 255 255 72 139 69 120 72 137 133 96 255 255 255 72 139 133 128 0 0 0 72 137 133 88 255 255 255 72 139 133 136 0 0 0 72 137 133 80 255 255 255 72 139 133 144 0 0 0 72 137 133 72 255 255 255 72 139 133 152 0 0 0 72 137 133 64 255 255 255 72 139 133 160 0 0 0 72 137 133 56 255 255 255 72 139 133 168 0 0 0 72 137 133 48 255 255 255 72 139 133 176 0 0 0 72 137 133 40 255 255 255 72 139 133 184 0 0 0 72 137 133 32 255 255 255 72 139 133 192 0 0 0 72 137 133 24 255 255 255"#.split_whitespace().collect();
    assert!(
        lines.len() >= expected.len(),
        "x86 direct call twenty-nine-arg helper bytes 出力が不足: {:?}",
        lines
    );
    assert_eq!(
        &lines[..expected.len()],
        expected.as_slice(),
        "x86_64 direct call twenty-nine-arg helper payload/call-layout exact bytes が一致しない"
    );
}

/// NATIVE-REAL-08zj: x86_64 で 30 引数 direct call bundle helper が 24 stack arg / spill 30 を生成できること
#[test]
#[ignore]
fn test_native_codegen_emits_x86_direct_call_thirty_arg_bundle_bytes() {
    let output = run_native_codegen_harness(
        r#"(module Main)
(import NativeCodegen)

(defn print-range [bytes idx end]
  (if (>= idx end)
    0
    (do
      (print (vector-get bytes idx))
      (print-range bytes (+ idx 1) end))))

(defn main []
  (let [call-bytes (emit-thirty-arg-call-x86 16 0)
        spill-ref (ref-new (vector-new 8))]
    (do
      (spill-native-function-params-x86-twenty-to-thirty 30 spill-ref)
      (print (vector-length call-bytes))
      (print-range call-bytes 0 (vector-length call-bytes))
      (print (vector-length (ref-get spill-ref)))
      (print-range (ref-get spill-ref) 0 (vector-length (ref-get spill-ref)))
      0)))"#,
    );

    let lines: Vec<&str> = output.trim().lines().collect();
    let expected: Vec<&str> = r#"358 72 129 236 192 0 0 0 72 137 132 36 184 0 0 0 72 137 140 36 176 0 0 0 72 139 141 248 255 255 255 72 137 140 36 168 0 0 0 72 139 141 240 255 255 255 72 137 140 36 160 0 0 0 72 139 141 232 255 255 255 72 137 140 36 152 0 0 0 72 139 141 224 255 255 255 72 137 140 36 144 0 0 0 72 139 141 216 255 255 255 72 137 140 36 136 0 0 0 72 139 141 208 255 255 255 72 137 140 36 128 0 0 0 72 139 141 200 255 255 255 72 137 76 36 120 72 139 141 192 255 255 255 72 137 76 36 112 72 139 141 184 255 255 255 72 137 76 36 104 72 139 141 176 255 255 255 72 137 76 36 96 72 139 141 168 255 255 255 72 137 76 36 88 72 139 141 160 255 255 255 72 137 76 36 80 72 139 141 152 255 255 255 72 137 76 36 72 72 139 141 144 255 255 255 72 137 76 36 64 72 139 141 136 255 255 255 72 137 76 36 56 72 139 141 128 255 255 255 72 137 76 36 48 72 139 141 120 255 255 255 72 137 76 36 40 72 139 141 112 255 255 255 72 137 76 36 32 72 139 141 104 255 255 255 72 137 76 36 24 72 139 141 96 255 255 255 72 137 76 36 16 72 139 141 88 255 255 255 72 137 76 36 8 76 139 141 80 255 255 255 76 137 12 36 76 139 141 72 255 255 255 76 139 133 64 255 255 255 72 139 141 56 255 255 255 72 139 149 48 255 255 255 72 139 181 40 255 255 255 72 139 189 32 255 255 255 232 16 0 0 0 72 129 196 192 0 0 0 336 72 137 189 248 255 255 255 72 137 181 240 255 255 255 72 137 149 232 255 255 255 72 137 141 224 255 255 255 76 137 133 216 255 255 255 76 137 141 208 255 255 255 72 139 69 16 72 137 133 200 255 255 255 72 139 69 24 72 137 133 192 255 255 255 72 139 69 32 72 137 133 184 255 255 255 72 139 69 40 72 137 133 176 255 255 255 72 139 69 48 72 137 133 168 255 255 255 72 139 69 56 72 137 133 160 255 255 255 72 139 69 64 72 137 133 152 255 255 255 72 139 69 72 72 137 133 144 255 255 255 72 139 69 80 72 137 133 136 255 255 255 72 139 69 88 72 137 133 128 255 255 255 72 139 69 96 72 137 133 120 255 255 255 72 139 69 104 72 137 133 112 255 255 255 72 139 69 112 72 137 133 104 255 255 255 72 139 69 120 72 137 133 96 255 255 255 72 139 133 128 0 0 0 72 137 133 88 255 255 255 72 139 133 136 0 0 0 72 137 133 80 255 255 255 72 139 133 144 0 0 0 72 137 133 72 255 255 255 72 139 133 152 0 0 0 72 137 133 64 255 255 255 72 139 133 160 0 0 0 72 137 133 56 255 255 255 72 139 133 168 0 0 0 72 137 133 48 255 255 255 72 139 133 176 0 0 0 72 137 133 40 255 255 255 72 139 133 184 0 0 0 72 137 133 32 255 255 255 72 139 133 192 0 0 0 72 137 133 24 255 255 255 72 139 133 200 0 0 0 72 137 133 16 255 255 255"#.split_whitespace().collect();
    assert!(
        lines.len() >= expected.len(),
        "x86 direct call thirty-arg helper bytes 出力が不足: {:?}",
        lines
    );
    assert_eq!(
        &lines[..expected.len()],
        expected.as_slice(),
        "x86_64 direct call thirty-arg helper payload/call-layout exact bytes が一致しない"
    );
}

/// NATIVE-REAL-08zk: x86_64 で 31 引数 direct call bundle helper が 25 stack arg / spill 31 を生成できること
#[test]
#[ignore]
fn test_native_codegen_emits_x86_direct_call_thirty_one_arg_bundle_bytes() {
    let output = run_native_codegen_harness(
        r#"(module Main)
(import NativeCodegen)

(defn print-range [bytes idx end]
  (if (>= idx end)
    0
    (do
      (print (vector-get bytes idx))
      (print-range bytes (+ idx 1) end))))

(defn main []
  (let [call-bytes (emit-thirty-one-arg-call-x86 16 0)
        spill-ref (ref-new (vector-new 8))]
    (do
      (spill-native-function-params-x86-twenty-to-thirty-one 31 spill-ref)
      (print (vector-length call-bytes))
      (print-range call-bytes 0 (vector-length call-bytes))
      (print (vector-length (ref-get spill-ref)))
      (print-range (ref-get spill-ref) 0 (vector-length (ref-get spill-ref)))
      0)))"#,
    );

    let lines: Vec<&str> = output.trim().lines().collect();
    let expected: Vec<&str> = r#"373 72 129 236 208 0 0 0 72 137 132 36 192 0 0 0 72 137 140 36 184 0 0 0 72 139 141 248 255 255 255 72 137 140 36 176 0 0 0 72 139 141 240 255 255 255 72 137 140 36 168 0 0 0 72 139 141 232 255 255 255 72 137 140 36 160 0 0 0 72 139 141 224 255 255 255 72 137 140 36 152 0 0 0 72 139 141 216 255 255 255 72 137 140 36 144 0 0 0 72 139 141 208 255 255 255 72 137 140 36 136 0 0 0 72 139 141 200 255 255 255 72 137 140 36 128 0 0 0 72 139 141 192 255 255 255 72 137 76 36 120 72 139 141 184 255 255 255 72 137 76 36 112 72 139 141 176 255 255 255 72 137 76 36 104 72 139 141 168 255 255 255 72 137 76 36 96 72 139 141 160 255 255 255 72 137 76 36 88 72 139 141 152 255 255 255 72 137 76 36 80 72 139 141 144 255 255 255 72 137 76 36 72 72 139 141 136 255 255 255 72 137 76 36 64 72 139 141 128 255 255 255 72 137 76 36 56 72 139 141 120 255 255 255 72 137 76 36 48 72 139 141 112 255 255 255 72 137 76 36 40 72 139 141 104 255 255 255 72 137 76 36 32 72 139 141 96 255 255 255 72 137 76 36 24 72 139 141 88 255 255 255 72 137 76 36 16 72 139 141 80 255 255 255 72 137 76 36 8 76 139 141 72 255 255 255 76 137 12 36 76 139 141 64 255 255 255 76 139 133 56 255 255 255 72 139 141 48 255 255 255 72 139 149 40 255 255 255 72 139 181 32 255 255 255 72 139 189 24 255 255 255 232 16 0 0 0 72 129 196 208 0 0 0 350 72 137 189 248 255 255 255 72 137 181 240 255 255 255 72 137 149 232 255 255 255 72 137 141 224 255 255 255 76 137 133 216 255 255 255 76 137 141 208 255 255 255 72 139 69 16 72 137 133 200 255 255 255 72 139 69 24 72 137 133 192 255 255 255 72 139 69 32 72 137 133 184 255 255 255 72 139 69 40 72 137 133 176 255 255 255 72 139 69 48 72 137 133 168 255 255 255 72 139 69 56 72 137 133 160 255 255 255 72 139 69 64 72 137 133 152 255 255 255 72 139 69 72 72 137 133 144 255 255 255 72 139 69 80 72 137 133 136 255 255 255 72 139 69 88 72 137 133 128 255 255 255 72 139 69 96 72 137 133 120 255 255 255 72 139 69 104 72 137 133 112 255 255 255 72 139 69 112 72 137 133 104 255 255 255 72 139 69 120 72 137 133 96 255 255 255 72 139 133 128 0 0 0 72 137 133 88 255 255 255 72 139 133 136 0 0 0 72 137 133 80 255 255 255 72 139 133 144 0 0 0 72 137 133 72 255 255 255 72 139 133 152 0 0 0 72 137 133 64 255 255 255 72 139 133 160 0 0 0 72 137 133 56 255 255 255 72 139 133 168 0 0 0 72 137 133 48 255 255 255 72 139 133 176 0 0 0 72 137 133 40 255 255 255 72 139 133 184 0 0 0 72 137 133 32 255 255 255 72 139 133 192 0 0 0 72 137 133 24 255 255 255 72 139 133 200 0 0 0 72 137 133 16 255 255 255 72 139 133 208 0 0 0 72 137 133 8 255 255 255"#.split_whitespace().collect();
    assert!(
        lines.len() >= expected.len(),
        "x86 direct call thirty-one-arg helper bytes 出力が不足: {:?}",
        lines
    );
    assert_eq!(
        &lines[..expected.len()],
        expected.as_slice(),
        "x86_64 direct call thirty-one-arg helper payload/call-layout exact bytes が一致しない"
    );
}

/// NATIVE-REAL-08zl: x86_64 で 32 引数 direct call bundle helper が 26 stack arg / spill 32 を生成できること
#[test]
#[ignore]
fn test_native_codegen_emits_x86_direct_call_thirty_two_arg_bundle_bytes() {
    let output = run_native_codegen_harness(
        r#"(module Main)
(import NativeCodegen)

(defn print-range [bytes idx end]
  (if (>= idx end)
    0
    (do
      (print (vector-get bytes idx))
      (print-range bytes (+ idx 1) end))))

(defn main []
  (let [call-bytes (emit-thirty-two-arg-call-x86 16 0)
        spill-ref (ref-new (vector-new 8))]
    (do
      (spill-native-function-params-x86-twenty-to-thirty-two 32 spill-ref)
      (print (vector-length call-bytes))
      (print-range call-bytes 0 (vector-length call-bytes))
      (print (vector-length (ref-get spill-ref)))
      (print-range (ref-get spill-ref) 0 (vector-length (ref-get spill-ref)))
      0)))"#,
    );

    let lines: Vec<&str> = output.trim().lines().collect();
    let expected: Vec<&str> = r#"388 72 129 236 208 0 0 0 72 137 132 36 200 0 0 0 72 137 140 36 192 0 0 0 72 139 141 248 255 255 255 72 137 140 36 184 0 0 0 72 139 141 240 255 255 255 72 137 140 36 176 0 0 0 72 139 141 232 255 255 255 72 137 140 36 168 0 0 0 72 139 141 224 255 255 255 72 137 140 36 160 0 0 0 72 139 141 216 255 255 255 72 137 140 36 152 0 0 0 72 139 141 208 255 255 255 72 137 140 36 144 0 0 0 72 139 141 200 255 255 255 72 137 140 36 136 0 0 0 72 139 141 192 255 255 255 72 137 140 36 128 0 0 0 72 139 141 184 255 255 255 72 137 76 36 120 72 139 141 176 255 255 255 72 137 76 36 112 72 139 141 168 255 255 255 72 137 76 36 104 72 139 141 160 255 255 255 72 137 76 36 96 72 139 141 152 255 255 255 72 137 76 36 88 72 139 141 144 255 255 255 72 137 76 36 80 72 139 141 136 255 255 255 72 137 76 36 72 72 139 141 128 255 255 255 72 137 76 36 64 72 139 141 120 255 255 255 72 137 76 36 56 72 139 141 112 255 255 255 72 137 76 36 48 72 139 141 104 255 255 255 72 137 76 36 40 72 139 141 96 255 255 255 72 137 76 36 32 72 139 141 88 255 255 255 72 137 76 36 24 72 139 141 80 255 255 255 72 137 76 36 16 72 139 141 72 255 255 255 72 137 76 36 8 76 139 141 64 255 255 255 76 137 12 36 76 139 141 56 255 255 255 76 139 133 48 255 255 255 72 139 141 40 255 255 255 72 139 149 32 255 255 255 72 139 181 24 255 255 255 72 139 189 16 255 255 255 232 16 0 0 0 72 129 196 208 0 0 0 364 72 137 189 248 255 255 255 72 137 181 240 255 255 255 72 137 149 232 255 255 255 72 137 141 224 255 255 255 76 137 133 216 255 255 255 76 137 141 208 255 255 255 72 139 69 16 72 137 133 200 255 255 255 72 139 69 24 72 137 133 192 255 255 255 72 139 69 32 72 137 133 184 255 255 255 72 139 69 40 72 137 133 176 255 255 255 72 139 69 48 72 137 133 168 255 255 255 72 139 69 56 72 137 133 160 255 255 255 72 139 69 64 72 137 133 152 255 255 255 72 139 69 72 72 137 133 144 255 255 255 72 139 69 80 72 137 133 136 255 255 255 72 139 69 88 72 137 133 128 255 255 255 72 139 69 96 72 137 133 120 255 255 255 72 139 69 104 72 137 133 112 255 255 255 72 139 69 112 72 137 133 104 255 255 255 72 139 69 120 72 137 133 96 255 255 255 72 139 133 128 0 0 0 72 137 133 88 255 255 255 72 139 133 136 0 0 0 72 137 133 80 255 255 255 72 139 133 144 0 0 0 72 137 133 72 255 255 255 72 139 133 152 0 0 0 72 137 133 64 255 255 255 72 139 133 160 0 0 0 72 137 133 56 255 255 255 72 139 133 168 0 0 0 72 137 133 48 255 255 255 72 139 133 176 0 0 0 72 137 133 40 255 255 255 72 139 133 184 0 0 0 72 137 133 32 255 255 255 72 139 133 192 0 0 0 72 137 133 24 255 255 255 72 139 133 200 0 0 0 72 137 133 16 255 255 255 72 139 133 208 0 0 0 72 137 133 8 255 255 255 72 139 133 216 0 0 0 72 137 133 0 255 255 255"#.split_whitespace().collect();
    assert!(
        lines.len() >= expected.len(),
        "x86 direct call thirty-two-arg helper bytes 出力が不足: {:?}",
        lines
    );
    assert_eq!(
        &lines[..expected.len()],
        expected.as_slice(),
        "x86_64 direct call thirty-two-arg helper payload/call-layout exact bytes が一致しない"
    );
}

/// NATIVE-REAL-08zm: x86_64 で 33 引数 direct call bundle helper が 27 stack arg / spill 33 を生成できること
#[test]
#[ignore]
fn test_native_codegen_emits_x86_direct_call_thirty_three_arg_bundle_bytes() {
    let output = run_native_codegen_harness(
        r#"(module Main)
(import NativeCodegen)

(defn print-range [bytes idx end]
  (if (>= idx end)
    0
    (do
      (print (vector-get bytes idx))
      (print-range bytes (+ idx 1) end))))

(defn main []
  (let [call-bytes (emit-thirty-three-arg-call-x86 16 0)
        spill-ref (ref-new (vector-new 8))]
    (do
      (spill-native-function-params-x86-twenty-to-thirty-three 33 spill-ref)
      (print (vector-length call-bytes))
      (print-range call-bytes 0 (vector-length call-bytes))
      (print (vector-length (ref-get spill-ref)))
      (print-range (ref-get spill-ref) 0 (vector-length (ref-get spill-ref)))
      0)))"#,
    );

    let lines: Vec<&str> = output.trim().lines().collect();
    let expected: Vec<&str> = r#"403 72 129 236 224 0 0 0 72 137 132 36 208 0 0 0 72 137 140 36 200 0 0 0 72 139 141 248 255 255 255 72 137 140 36 192 0 0 0 72 139 141 240 255 255 255 72 137 140 36 184 0 0 0 72 139 141 232 255 255 255 72 137 140 36 176 0 0 0 72 139 141 224 255 255 255 72 137 140 36 168 0 0 0 72 139 141 216 255 255 255 72 137 140 36 160 0 0 0 72 139 141 208 255 255 255 72 137 140 36 152 0 0 0 72 139 141 200 255 255 255 72 137 140 36 144 0 0 0 72 139 141 192 255 255 255 72 137 140 36 136 0 0 0 72 139 141 184 255 255 255 72 137 140 36 128 0 0 0 72 139 141 176 255 255 255 72 137 76 36 120 72 139 141 168 255 255 255 72 137 76 36 112 72 139 141 160 255 255 255 72 137 76 36 104 72 139 141 152 255 255 255 72 137 76 36 96 72 139 141 144 255 255 255 72 137 76 36 88 72 139 141 136 255 255 255 72 137 76 36 80 72 139 141 128 255 255 255 72 137 76 36 72 72 139 141 120 255 255 255 72 137 76 36 64 72 139 141 112 255 255 255 72 137 76 36 56 72 139 141 104 255 255 255 72 137 76 36 48 72 139 141 96 255 255 255 72 137 76 36 40 72 139 141 88 255 255 255 72 137 76 36 32 72 139 141 80 255 255 255 72 137 76 36 24 72 139 141 72 255 255 255 72 137 76 36 16 72 139 141 64 255 255 255 72 137 76 36 8 76 139 141 56 255 255 255 76 137 12 36 76 139 141 48 255 255 255 76 139 133 40 255 255 255 72 139 141 32 255 255 255 72 139 149 24 255 255 255 72 139 181 16 255 255 255 72 139 189 8 255 255 255 232 16 0 0 0 72 129 196 224 0 0 0 378 72 137 189 248 255 255 255 72 137 181 240 255 255 255 72 137 149 232 255 255 255 72 137 141 224 255 255 255 76 137 133 216 255 255 255 76 137 141 208 255 255 255 72 139 69 16 72 137 133 200 255 255 255 72 139 69 24 72 137 133 192 255 255 255 72 139 69 32 72 137 133 184 255 255 255 72 139 69 40 72 137 133 176 255 255 255 72 139 69 48 72 137 133 168 255 255 255 72 139 69 56 72 137 133 160 255 255 255 72 139 69 64 72 137 133 152 255 255 255 72 139 69 72 72 137 133 144 255 255 255 72 139 69 80 72 137 133 136 255 255 255 72 139 69 88 72 137 133 128 255 255 255 72 139 69 96 72 137 133 120 255 255 255 72 139 69 104 72 137 133 112 255 255 255 72 139 69 112 72 137 133 104 255 255 255 72 139 69 120 72 137 133 96 255 255 255 72 139 133 128 0 0 0 72 137 133 88 255 255 255 72 139 133 136 0 0 0 72 137 133 80 255 255 255 72 139 133 144 0 0 0 72 137 133 72 255 255 255 72 139 133 152 0 0 0 72 137 133 64 255 255 255 72 139 133 160 0 0 0 72 137 133 56 255 255 255 72 139 133 168 0 0 0 72 137 133 48 255 255 255 72 139 133 176 0 0 0 72 137 133 40 255 255 255 72 139 133 184 0 0 0 72 137 133 32 255 255 255 72 139 133 192 0 0 0 72 137 133 24 255 255 255 72 139 133 200 0 0 0 72 137 133 16 255 255 255 72 139 133 208 0 0 0 72 137 133 8 255 255 255 72 139 133 216 0 0 0 72 137 133 0 255 255 255 72 139 133 224 0 0 0 72 137 133 248 254 255 255"#.split_whitespace().collect();
    assert!(
        lines.len() >= expected.len(),
        "x86 direct call thirty-three-arg helper bytes 出力が不足: {:?}",
        lines
    );
    assert_eq!(
        &lines[..expected.len()],
        expected.as_slice(),
        "x86_64 direct call thirty-three-arg helper payload/call-layout exact bytes が一致しない"
    );
}
