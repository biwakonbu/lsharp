
/// NATIVE-REAL-0913: x86_64 で 60 引数 direct call bundle helper が 54 stack arg / spill 60 を生成できること
#[test]
#[ignore]
fn test_native_codegen_emits_x86_direct_call_sixty_arg_bundle_bytes() {
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
  (let [call-bytes (emit-sixty-arg-call-x86 16 0)
        spill-ref (ref-new (vector-new 8))]
    (do
      (spill-native-function-params-x86-twenty-to-sixty 60 spill-ref)
      (print (vector-length call-bytes))
      (print-range call-bytes 0 (vector-length call-bytes))
      (print (vector-length (ref-get spill-ref)))
      (print-range (ref-get spill-ref) 0 (vector-length (ref-get spill-ref)))
      0)))"#,
    );
    let lines: Vec<&str> = output.trim().lines().collect();
    let expected: Vec<&str> = r#"
853 72 129 236 176 1 0 0 72 137 132 36 168 1 0 0
72 137 140 36 160 1 0 0 72 139 141 248 255 255 255 72
137 140 36 152 1 0 0 72 139 141 240 255 255 255 72 137
140 36 144 1 0 0 72 139 141 232 255 255 255 72 137 140
36 136 1 0 0 72 139 141 224 255 255 255 72 137 140 36
128 1 0 0 72 139 141 216 255 255 255 72 137 140 36 120
1 0 0 72 139 141 208 255 255 255 72 137 140 36 112 1
0 0 72 139 141 200 255 255 255 72 137 140 36 104 1 0
0 72 139 141 192 255 255 255 72 137 140 36 96 1 0 0
72 139 141 184 255 255 255 72 137 140 36 88 1 0 0 72
139 141 176 255 255 255 72 137 140 36 80 1 0 0 72 139
141 168 255 255 255 72 137 140 36 72 1 0 0 72 139 141
160 255 255 255 72 137 140 36 64 1 0 0 72 139 141 152
255 255 255 72 137 140 36 56 1 0 0 72 139 141 144 255
255 255 72 137 140 36 48 1 0 0 72 139 141 136 255 255
255 72 137 140 36 40 1 0 0 72 139 141 128 255 255 255
72 137 140 36 32 1 0 0 72 139 141 120 255 255 255 72
137 140 36 24 1 0 0 72 139 141 112 255 255 255 72 137
140 36 16 1 0 0 72 139 141 104 255 255 255 72 137 140
36 8 1 0 0 72 139 141 96 255 255 255 72 137 140 36
0 1 0 0 72 139 141 88 255 255 255 72 137 140 36 248
0 0 0 72 139 141 80 255 255 255 72 137 140 36 240 0
0 0 72 139 141 72 255 255 255 72 137 140 36 232 0 0
0 72 139 141 64 255 255 255 72 137 140 36 224 0 0 0
72 139 141 56 255 255 255 72 137 140 36 216 0 0 0 72
139 141 48 255 255 255 72 137 140 36 208 0 0 0 72 139
141 40 255 255 255 72 137 140 36 200 0 0 0 72 139 141
32 255 255 255 72 137 140 36 192 0 0 0 72 139 141 24
255 255 255 72 137 140 36 184 0 0 0 72 139 141 16 255
255 255 72 137 140 36 176 0 0 0 72 139 141 8 255 255
255 72 137 140 36 168 0 0 0 72 139 141 0 255 255 255
72 137 140 36 160 0 0 0 72 139 141 248 254 255 255 72
137 140 36 152 0 0 0 72 139 141 240 254 255 255 72 137
140 36 144 0 0 0 72 139 141 232 254 255 255 72 137 140
36 136 0 0 0 72 139 141 224 254 255 255 72 137 140 36
128 0 0 0 72 139 141 216 254 255 255 72 137 140 36 120
0 0 0 72 139 141 208 254 255 255 72 137 140 36 112 0
0 0 72 139 141 200 254 255 255 72 137 140 36 104 0 0
0 72 139 141 192 254 255 255 72 137 140 36 96 0 0 0
72 139 141 184 254 255 255 72 137 140 36 88 0 0 0 72
139 141 176 254 255 255 72 137 140 36 80 0 0 0 72 139
141 168 254 255 255 72 137 140 36 72 0 0 0 72 139 141
160 254 255 255 72 137 140 36 64 0 0 0 72 139 141 152
254 255 255 72 137 140 36 56 0 0 0 72 139 141 144 254
255 255 72 137 140 36 48 0 0 0 72 139 141 136 254 255
255 72 137 140 36 40 0 0 0 72 139 141 128 254 255 255
72 137 140 36 32 0 0 0 72 139 141 120 254 255 255 72
137 140 36 24 0 0 0 72 139 141 112 254 255 255 72 137
140 36 16 0 0 0 72 139 141 104 254 255 255 72 137 140
36 8 0 0 0 76 139 141 96 254 255 255 76 137 12 36
76 139 141 88 254 255 255 76 139 133 80 254 255 255 72 139
141 72 254 255 255 72 139 149 64 254 255 255 72 139 181 56
254 255 255 72 139 189 48 254 255 255 232 16 0 0 0 72
129 196 176 1 0 0 756 72 137 189 248 255 255 255 72 137
181 240 255 255 255 72 137 149 232 255 255 255 72 137 141 224
255 255 255 76 137 133 216 255 255 255 76 137 141 208 255 255
255 72 139 69 16 72 137 133 200 255 255 255 72 139 69 24
72 137 133 192 255 255 255 72 139 69 32 72 137 133 184 255
255 255 72 139 69 40 72 137 133 176 255 255 255 72 139 69
48 72 137 133 168 255 255 255 72 139 69 56 72 137 133 160
255 255 255 72 139 69 64 72 137 133 152 255 255 255 72 139
69 72 72 137 133 144 255 255 255 72 139 69 80 72 137 133
136 255 255 255 72 139 69 88 72 137 133 128 255 255 255 72
139 69 96 72 137 133 120 255 255 255 72 139 69 104 72 137
133 112 255 255 255 72 139 69 112 72 137 133 104 255 255 255
72 139 69 120 72 137 133 96 255 255 255 72 139 133 128 0
0 0 72 137 133 88 255 255 255 72 139 133 136 0 0 0
72 137 133 80 255 255 255 72 139 133 144 0 0 0 72 137
133 72 255 255 255 72 139 133 152 0 0 0 72 137 133 64
255 255 255 72 139 133 160 0 0 0 72 137 133 56 255 255
255 72 139 133 168 0 0 0 72 137 133 48 255 255 255 72
139 133 176 0 0 0 72 137 133 40 255 255 255 72 139 133
184 0 0 0 72 137 133 32 255 255 255 72 139 133 192 0
0 0 72 137 133 24 255 255 255 72 139 133 200 0 0 0
72 137 133 16 255 255 255 72 139 133 208 0 0 0 72 137
133 8 255 255 255 72 139 133 216 0 0 0 72 137 133 0
255 255 255 72 139 133 224 0 0 0 72 137 133 248 254 255
255 72 139 133 232 0 0 0 72 137 133 240 254 255 255 72
139 133 240 0 0 0 72 137 133 232 254 255 255 72 139 133
248 0 0 0 72 137 133 224 254 255 255 72 139 133 0 1
0 0 72 137 133 216 254 255 255 72 139 133 8 1 0 0
72 137 133 208 254 255 255 72 139 133 16 1 0 0 72 137
133 200 254 255 255 72 139 133 24 1 0 0 72 137 133 192
254 255 255 72 139 133 32 1 0 0 72 137 133 184 254 255
255 72 139 133 40 1 0 0 72 137 133 176 254 255 255 72
139 133 48 1 0 0 72 137 133 168 254 255 255 72 139 133
56 1 0 0 72 137 133 160 254 255 255 72 139 133 64 1
0 0 72 137 133 152 254 255 255 72 139 133 72 1 0 0
72 137 133 144 254 255 255 72 139 133 80 1 0 0 72 137
133 136 254 255 255 72 139 133 88 1 0 0 72 137 133 128
254 255 255 72 139 133 96 1 0 0 72 137 133 120 254 255
255 72 139 133 104 1 0 0 72 137 133 112 254 255 255 72
139 133 112 1 0 0 72 137 133 104 254 255 255 72 139 133
120 1 0 0 72 137 133 96 254 255 255 72 139 133 128 1
0 0 72 137 133 88 254 255 255 72 139 133 136 1 0 0
72 137 133 80 254 255 255 72 139 133 144 1 0 0 72 137
133 72 254 255 255 72 139 133 152 1 0 0 72 137 133 64
254 255 255 72 139 133 160 1 0 0 72 137 133 56 254 255
255 72 139 133 168 1 0 0 72 137 133 48 254 255 255 72
139 133 176 1 0 0 72 137 133 40 254 255 255 72 139 133
184 1 0 0 72 137 133 32 254 255 255
"#
    .split_whitespace()
    .collect();
    assert!(
        lines.len() >= expected.len(),
        "x86 direct call sixty-arg helper bytes 出力が不足: {:?}",
        lines
    );
    assert_eq!(
        &lines[..expected.len()],
        expected.as_slice(),
        "x86_64 direct call sixty-arg helper payload/call-layout exact bytes が一致しない"
    );
}

/// NATIVE-REAL-0914: x86_64 で 61 引数 direct call bundle helper が 55 stack arg / spill 61 を生成できること
#[test]
#[ignore]
fn test_native_codegen_emits_x86_direct_call_sixty_one_arg_bundle_bytes() {
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
  (let [call-bytes (emit-sixty-one-arg-call-x86 16 0)
        spill-ref (ref-new (vector-new 8))]
    (do
      (spill-native-function-params-x86-twenty-to-sixty-one 61 spill-ref)
      (print (vector-length call-bytes))
      (print-range call-bytes 0 (vector-length call-bytes))
      (print (vector-length (ref-get spill-ref)))
      (print-range (ref-get spill-ref) 0 (vector-length (ref-get spill-ref)))
      0)))"#,
    );
    let lines: Vec<&str> = output.trim().lines().collect();
    let expected: Vec<&str> = r#"
868 72 129 236 184 1 0 0 72 137 132 36 176 1 0 0
72 137 140 36 168 1 0 0 72 139 141 248 255 255 255 72
137 140 36 160 1 0 0 72 139 141 240 255 255 255 72 137
140 36 152 1 0 0 72 139 141 232 255 255 255 72 137 140
36 144 1 0 0 72 139 141 224 255 255 255 72 137 140 36
136 1 0 0 72 139 141 216 255 255 255 72 137 140 36 128
1 0 0 72 139 141 208 255 255 255 72 137 140 36 120 1
0 0 72 139 141 200 255 255 255 72 137 140 36 112 1 0
0 72 139 141 192 255 255 255 72 137 140 36 104 1 0 0
72 139 141 184 255 255 255 72 137 140 36 96 1 0 0 72
139 141 176 255 255 255 72 137 140 36 88 1 0 0 72 139
141 168 255 255 255 72 137 140 36 80 1 0 0 72 139 141
160 255 255 255 72 137 140 36 72 1 0 0 72 139 141 152
255 255 255 72 137 140 36 64 1 0 0 72 139 141 144 255
255 255 72 137 140 36 56 1 0 0 72 139 141 136 255 255
255 72 137 140 36 48 1 0 0 72 139 141 128 255 255 255
72 137 140 36 40 1 0 0 72 139 141 120 255 255 255 72
137 140 36 32 1 0 0 72 139 141 112 255 255 255 72 137
140 36 24 1 0 0 72 139 141 104 255 255 255 72 137 140
36 16 1 0 0 72 139 141 96 255 255 255 72 137 140 36
8 1 0 0 72 139 141 88 255 255 255 72 137 140 36 0
1 0 0 72 139 141 80 255 255 255 72 137 140 36 248 0
0 0 72 139 141 72 255 255 255 72 137 140 36 240 0 0
0 72 139 141 64 255 255 255 72 137 140 36 232 0 0 0
72 139 141 56 255 255 255 72 137 140 36 224 0 0 0 72
139 141 48 255 255 255 72 137 140 36 216 0 0 0 72 139
141 40 255 255 255 72 137 140 36 208 0 0 0 72 139 141
32 255 255 255 72 137 140 36 200 0 0 0 72 139 141 24
255 255 255 72 137 140 36 192 0 0 0 72 139 141 16 255
255 255 72 137 140 36 184 0 0 0 72 139 141 8 255 255
255 72 137 140 36 176 0 0 0 72 139 141 0 255 255 255
72 137 140 36 168 0 0 0 72 139 141 248 254 255 255 72
137 140 36 160 0 0 0 72 139 141 240 254 255 255 72 137
140 36 152 0 0 0 72 139 141 232 254 255 255 72 137 140
36 144 0 0 0 72 139 141 224 254 255 255 72 137 140 36
136 0 0 0 72 139 141 216 254 255 255 72 137 140 36 128
0 0 0 72 139 141 208 254 255 255 72 137 140 36 120 0
0 0 72 139 141 200 254 255 255 72 137 140 36 112 0 0
0 72 139 141 192 254 255 255 72 137 140 36 104 0 0 0
72 139 141 184 254 255 255 72 137 140 36 96 0 0 0 72
139 141 176 254 255 255 72 137 140 36 88 0 0 0 72 139
141 168 254 255 255 72 137 140 36 80 0 0 0 72 139 141
160 254 255 255 72 137 140 36 72 0 0 0 72 139 141 152
254 255 255 72 137 140 36 64 0 0 0 72 139 141 144 254
255 255 72 137 140 36 56 0 0 0 72 139 141 136 254 255
255 72 137 140 36 48 0 0 0 72 139 141 128 254 255 255
72 137 140 36 40 0 0 0 72 139 141 120 254 255 255 72
137 140 36 32 0 0 0 72 139 141 112 254 255 255 72 137
140 36 24 0 0 0 72 139 141 104 254 255 255 72 137 140
36 16 0 0 0 72 139 141 96 254 255 255 72 137 140 36
8 0 0 0 76 139 141 88 254 255 255 76 137 12 36 76
139 141 80 254 255 255 76 139 133 72 254 255 255 72 139 141
64 254 255 255 72 139 149 56 254 255 255 72 139 181 48 254
255 255 72 139 189 40 254 255 255 232 16 0 0 0 72 129
196 184 1 0 0 770 72 137 189 248 255 255 255 72 137 181
240 255 255 255 72 137 149 232 255 255 255 72 137 141 224 255
255 255 76 137 133 216 255 255 255 76 137 141 208 255 255 255
72 139 69 16 72 137 133 200 255 255 255 72 139 69 24 72
137 133 192 255 255 255 72 139 69 32 72 137 133 184 255 255
255 72 139 69 40 72 137 133 176 255 255 255 72 139 69 48
72 137 133 168 255 255 255 72 139 69 56 72 137 133 160 255
255 255 72 139 69 64 72 137 133 152 255 255 255 72 139 69
72 72 137 133 144 255 255 255 72 139 69 80 72 137 133 136
255 255 255 72 139 69 88 72 137 133 128 255 255 255 72 139
69 96 72 137 133 120 255 255 255 72 139 69 104 72 137 133
112 255 255 255 72 139 69 112 72 137 133 104 255 255 255 72
139 69 120 72 137 133 96 255 255 255 72 139 133 128 0 0
0 72 137 133 88 255 255 255 72 139 133 136 0 0 0 72
137 133 80 255 255 255 72 139 133 144 0 0 0 72 137 133
72 255 255 255 72 139 133 152 0 0 0 72 137 133 64 255
255 255 72 139 133 160 0 0 0 72 137 133 56 255 255 255
72 139 133 168 0 0 0 72 137 133 48 255 255 255 72 139
133 176 0 0 0 72 137 133 40 255 255 255 72 139 133 184
0 0 0 72 137 133 32 255 255 255 72 139 133 192 0 0
0 72 137 133 24 255 255 255 72 139 133 200 0 0 0 72
137 133 16 255 255 255 72 139 133 208 0 0 0 72 137 133
8 255 255 255 72 139 133 216 0 0 0 72 137 133 0 255
255 255 72 139 133 224 0 0 0 72 137 133 248 254 255 255
72 139 133 232 0 0 0 72 137 133 240 254 255 255 72 139
133 240 0 0 0 72 137 133 232 254 255 255 72 139 133 248
0 0 0 72 137 133 224 254 255 255 72 139 133 0 1 0
0 72 137 133 216 254 255 255 72 139 133 8 1 0 0 72
137 133 208 254 255 255 72 139 133 16 1 0 0 72 137 133
200 254 255 255 72 139 133 24 1 0 0 72 137 133 192 254
255 255 72 139 133 32 1 0 0 72 137 133 184 254 255 255
72 139 133 40 1 0 0 72 137 133 176 254 255 255 72 139
133 48 1 0 0 72 137 133 168 254 255 255 72 139 133 56
1 0 0 72 137 133 160 254 255 255 72 139 133 64 1 0
0 72 137 133 152 254 255 255 72 139 133 72 1 0 0 72
137 133 144 254 255 255 72 139 133 80 1 0 0 72 137 133
136 254 255 255 72 139 133 88 1 0 0 72 137 133 128 254
255 255 72 139 133 96 1 0 0 72 137 133 120 254 255 255
72 139 133 104 1 0 0 72 137 133 112 254 255 255 72 139
133 112 1 0 0 72 137 133 104 254 255 255 72 139 133 120
1 0 0 72 137 133 96 254 255 255 72 139 133 128 1 0
0 72 137 133 88 254 255 255 72 139 133 136 1 0 0 72
137 133 80 254 255 255 72 139 133 144 1 0 0 72 137 133
72 254 255 255 72 139 133 152 1 0 0 72 137 133 64 254
255 255 72 139 133 160 1 0 0 72 137 133 56 254 255 255
72 139 133 168 1 0 0 72 137 133 48 254 255 255 72 139
133 176 1 0 0 72 137 133 40 254 255 255 72 139 133 184
1 0 0 72 137 133 32 254 255 255 72 139 133 192 1 0
0 72 137 133 24 254 255 255
"#
    .split_whitespace()
    .collect();
    assert!(
        lines.len() >= expected.len(),
        "x86 direct call sixty-one-arg helper bytes 出力が不足: {:?}",
        lines
    );
    assert_eq!(
        &lines[..expected.len()],
        expected.as_slice(),
        "x86_64 direct call sixty-one-arg helper payload/call-layout exact bytes が一致しない"
    );
}

/// NATIVE-REAL-09: emit-object が生成した native bytes 全体を object file へ保持すること
#[test]
fn test_native_emit_elf_peak_root_depth_does_not_grow_capacity_for_released_roots() {
    let mut chunk_expr = "bytes".to_string();
    for _ in 0..64 {
        chunk_expr = format!("(vector-push {chunk_expr} 0)");
    }
    let mut chunk_bindings = String::new();
    for idx in 0..128 {
        chunk_bindings.push_str(&format!(
            "b{} (append-native-code-chunk b{})\n        ",
            idx + 1,
            idx
        ));
    }
    let entry_source = format!(
        r#"(module Main)
(import Backend.Native.NativeTarget)
(import Backend.Native.NativeEmit)

(defn append-native-code-chunk [bytes]
  {chunk_expr})

(defn main []
  (let [b0 (vector-new 8193)
        {chunk_bindings}native-code (vector-push b128 0)
        object (emit-elf native-code)]
    (do
      (print (vector-length native-code))
      (print (vector-length object))
      0)))"#
    );
    let (output, telemetry) = compile_and_capture_selfhost_fixture_runtime_telemetry(
        "native-emit-elf-peak-root-depth",
        &["NativeTarget.ls", "NativeEmit.ls"],
        "src/Main.ls",
        &entry_source,
    );
    let lines: Vec<&str> = output.trim().lines().collect();

    assert_eq!(lines.first().copied(), Some("8193"));
    assert!(
        lines
            .get(1)
            .and_then(|line| line.parse::<usize>().ok())
            .is_some_and(|len| len > 8193),
        "ELF object は native payload を含むべき: {:?}",
        lines
    );
    assert_eq!(
        telemetry.root_stack_top, 0,
        "emit-elf 完了後に root stack が解放されるべき"
    );
    assert_eq!(
        telemetry.root_stack_capacity, 32768,
        "解放済みの byte append が root stack capacity を成長させないべき: {:?}",
        telemetry
    );
}

#[test]
#[ignore]
fn test_native_emit_object_keeps_full_native_payload() {
    let output = run_native_codegen_harness(
        r#"(module Main)
(import NativeTarget)
(import NativeCodegen)
(import NativeEmit)

(defn make-instr [opcode operand]
  (vector-push (vector-push (vector-new 2) opcode) operand))

(defn main []
  (let [instr (make-instr 1 42)
        ir (vector-push (vector-new 1) instr)
        target (make-target 1)
        native (emit-native ir target)
        obj (emit-object native target)]
    (do
      (print (vector-length native))
      (print (vector-length obj))
      (print (vector-get obj 0))
      (print (vector-get obj 1))
      (print (vector-get obj 2))
      (print (vector-get obj 3))
      (print (vector-get obj 232))
      (print (vector-get obj 233))
      (print (vector-get obj 246))
      (print (vector-get obj 247))
      0)))"#,
    );
    let lines: Vec<&str> = output.trim().lines().collect();

    assert!(
        lines.len() >= 10,
        "native object bytes 出力が不足: {:?}",
        lines
    );
    assert_eq!(
        lines[0], "16",
        "const native payload は 16 bytes であるべき"
    );
    assert_eq!(
        lines[1], "276",
        "x86_64 Mach-O object は linkable header/load commands/symtab + native payload を持つべき"
    );
    assert_eq!(lines[2], "207", "object 先頭は Mach-O magic 0xCF");
    assert_eq!(lines[3], "250", "object 2 byte 目は Mach-O magic 0xFA");
    assert_eq!(lines[4], "237", "object 3 byte 目は Mach-O magic 0xED");
    assert_eq!(lines[5], "254", "object 4 byte 目は Mach-O magic 0xFE");
    assert_eq!(lines[6], "85", "payload 先頭は push rbp (0x55)");
    assert_eq!(lines[7], "72", "payload 2 byte 目は REX.W (0x48)");
    assert_eq!(lines[8], "93", "payload 末尾 2 byte 手前は pop rbp (0x5D)");
    assert_eq!(lines[9], "195", "payload 末尾は ret (0xC3)");
}

/// NATIVE-REAL-10: ELF object でも native payload 全体を保持すること
#[test]
#[ignore]
fn test_native_emit_elf_object_keeps_full_native_payload() {
    let output = run_native_codegen_harness(
        r#"(module Main)
(import NativeTarget)
(import NativeCodegen)
(import NativeEmit)

(defn make-instr [opcode operand]
  (vector-push (vector-push (vector-new 2) opcode) operand))

(defn main []
  (let [instr (make-instr 1 42)
        ir (vector-push (vector-new 1) instr)
        target (make-target 3)
        native (emit-native ir target)
        obj (emit-object native target)]
    (do
      (print (vector-length native))
      (print (vector-length obj))
      (print (vector-get obj 0))
      (print (vector-get obj 1))
      (print (vector-get obj 2))
      (print (vector-get obj 3))
      (print (vector-get obj 64))
      (print (vector-get obj 65))
      (print (vector-get obj 78))
      (print (vector-get obj 79))
      0)))"#,
    );
    let lines: Vec<&str> = output.trim().lines().collect();

    assert!(
        lines.len() >= 10,
        "ELF object bytes 出力が不足: {:?}",
        lines
    );
    assert_eq!(
        lines[0], "16",
        "const native payload は 16 bytes であるべき"
    );
    assert_eq!(
        lines[1], "600",
        "ELF64 linkable object は 600 bytes であるべき"
    );
    assert_eq!(lines[2], "127", "ELF 先頭は 0x7F");
    assert_eq!(lines[3], "69", "ELF 2 byte 目は 'E'");
    assert_eq!(lines[4], "76", "ELF 3 byte 目は 'L'");
    assert_eq!(lines[5], "70", "ELF 4 byte 目は 'F'");
    assert_eq!(lines[6], "85", "payload 先頭は push rbp (0x55)");
    assert_eq!(lines[7], "72", "payload 2 byte 目は REX.W (0x48)");
    assert_eq!(lines[8], "93", "payload 末尾 2 byte 手前は pop rbp (0x5D)");
    assert_eq!(lines[9], "195", "payload 末尾は ret (0xC3)");
}

/// NATIVE-REAL-10b: 3 target で object header / payload invariants が保たれること
#[test]
#[ignore]
fn test_native_emit_object_headers_cover_all_three_targets() {
    let output = run_native_codegen_harness(
        r#"(module Main)
(import NativeTarget)
(import NativeCodegen)
(import NativeEmit)

(defn make-instr [opcode operand]
  (vector-push (vector-push (vector-new 2) opcode) operand))

(defn emit-summary [triple-id]
  (let [instr (make-instr 1 42)
        ir (vector-push (vector-new 1) instr)
        target (make-target triple-id)
        native (emit-native ir target)
        obj (emit-object native target)
        tail-idx (if (= triple-id 1) 246 (if (= triple-id 3) 78 22))
        last-idx (if (= triple-id 1) 247 (if (= triple-id 3) 79 23))]
    (do
      (print (vector-length obj))
      (print (vector-get obj 0))
      (print (vector-get obj 4))
      (print (vector-get obj tail-idx))
      (print (vector-get obj last-idx)))))

(defn main []
  (do
    (emit-summary 1)
    (emit-summary 2)
    (emit-summary 3)
    0))"#,
    );
    let lines: Vec<&str> = output.trim().lines().collect();

    assert!(
        lines.len() >= 15,
        "3 target object summary 出力が不足: {:?}",
        lines
    );
    assert_eq!(
        lines[0], "276",
        "target 1 x86_64 Mach-O object は linkable object として 276 bytes"
    );
    assert_eq!(lines[1], "207", "target 1 先頭 byte は Mach-O magic 0xCF");
    assert_eq!(lines[2], "7", "target 1 cpu byte は x86_64=0x07");
    assert_eq!(
        lines[3], "93",
        "target 1 payload 末尾 2 byte 手前は pop rbp"
    );
    assert_eq!(lines[4], "195", "target 1 payload 末尾は ret");
    assert_eq!(
        lines[5], "24",
        "target 2 Mach-O object は 24 bytes (AArch64)"
    );
    assert_eq!(lines[6], "207", "target 2 先頭 byte も Mach-O magic 0xCF");
    assert_eq!(lines[7], "12", "target 2 cpu byte は arm64=0x0C");
    assert_eq!(
        lines[8], "95",
        "target 2 payload 末尾 2 byte 手前は RET byte 2 (0x5F)"
    );
    assert_eq!(lines[9], "214", "target 2 payload 末尾は RET byte 3 (0xD6)");
    assert_eq!(lines[10], "600", "target 3 ELF object は 600 bytes");
    assert_eq!(lines[11], "127", "target 3 先頭 byte は ELF magic 0x7F");
    assert_eq!(lines[12], "2", "target 3 header byte 4 は ELFCLASS64=2");
    assert_eq!(
        lines[13], "93",
        "target 3 payload 末尾 2 byte 手前は pop rbp"
    );
    assert_eq!(lines[14], "195", "target 3 payload 末尾は ret");
}

/// NATIVE-REAL-11: Linker response file が全 object entry を保持すること
#[test]
fn test_native_linker_response_keeps_full_object_list() {
    let output = run_native_linker_harness(
        r#"(module Main)
(import NativeTarget)
(import Linker)

(defn main []
  (let [target (make-target 1)
        objects (vector-push
                  (vector-push
                    (vector-push
                      (vector-push
                        (vector-push (vector-new 5) 10)
                        20)
                      30)
                    40)
                  50)
        args (build-linker-args objects 99 target)
        response (generate-response-file args)]
    (do
      (print (vector-length args))
      (print (vector-length response))
      (print (vector-get args 0))
      (print (vector-get args 1))
      (print (vector-get args 5))
      (print (vector-get args 6))
      (print (vector-get response 10))
      (print (vector-get response 11))
      (print (vector-get response 12))
      (print (vector-get response 13))
      0)))"#,
    );
    let lines: Vec<&str> = output.trim().lines().collect();

    assert!(lines.len() >= 10, "linker response 出力が不足: {:?}", lines);
    assert_eq!(
        lines[0], "7",
        "-o, output, object 5 件で args は 7 要素であるべき"
    );
    assert_eq!(
        lines[1], "14",
        "7 要素の response file は 14 bytes であるべき"
    );
    assert_eq!(lines[2], "1", "先頭 arg は -o フラグ sentinel");
    assert_eq!(lines[3], "99", "2 番目 arg は output 値");
    assert_eq!(lines[4], "40", "6 番目 arg は 4 個目 object");
    assert_eq!(lines[5], "50", "7 番目 arg は 5 個目 object");
    assert_eq!(lines[6], "40", "response 後半にも 4 個目 object が残ること");
    assert_eq!(lines[7], "10", "response の各 arg は改行区切りされること");
    assert_eq!(
        lines[8], "50",
        "response 末尾直前にも 5 個目 object が残ること"
    );
    assert_eq!(lines[9], "10", "response 末尾は改行で終わること");
}

/// NATIVE-REAL-11b: 3 target で linker selection と response content が安定すること
#[test]
fn test_native_linker_response_consistency_across_three_targets() {
    let output = run_native_linker_harness(
        r#"(module Main)
(import NativeTarget)
(import Linker)

(defn emit-summary [triple-id]
  (let [target (make-target triple-id)
        objects (vector-push (vector-push (vector-new 2) 11) 22)
        linker (select-linker target)
        args (build-linker-args objects 99 target)
        response (generate-response-file args)]
    (do
      (print linker)
      (print (vector-length response))
      (print (vector-get response 0))
      (print (vector-get response 2))
      (print (vector-get response 4))
      (print (vector-get response 6)))))

(defn main []
  (do
    (emit-summary 1)
    (emit-summary 2)
    (emit-summary 3)
    0))"#,
    );
    let lines: Vec<&str> = output.trim().lines().collect();

    assert!(
        lines.len() >= 18,
        "3 target linker summary 出力が不足: {:?}",
        lines
    );
    assert_eq!(lines[0], "1", "target 1 linker は ld64");
    assert_eq!(lines[1], "8", "target 1 response len は 8 bytes");
    assert_eq!(lines[2], "1", "target 1 response 先頭は -o sentinel");
    assert_eq!(lines[3], "99", "target 1 response は output=99 を含む");
    assert_eq!(lines[4], "11", "target 1 response は object 1 を含む");
    assert_eq!(lines[5], "22", "target 1 response は object 2 を含む");
    assert_eq!(lines[6], "1", "target 2 linker も ld64");
    assert_eq!(lines[7], "8", "target 2 response len は 8 bytes");
    assert_eq!(lines[8], "1", "target 2 response 先頭は -o sentinel");
    assert_eq!(lines[9], "99", "target 2 response は output=99 を含む");
    assert_eq!(lines[10], "11", "target 2 response は object 1 を含む");
    assert_eq!(lines[11], "22", "target 2 response は object 2 を含む");
    assert_eq!(lines[12], "2", "target 3 linker は ld.lld");
    assert_eq!(lines[13], "8", "target 3 response len は 8 bytes");
    assert_eq!(lines[14], "1", "target 3 response 先頭は -o sentinel");
    assert_eq!(lines[15], "99", "target 3 response は output=99 を含む");
    assert_eq!(lines[16], "11", "target 3 response は object 1 を含む");
    assert_eq!(lines[17], "22", "target 3 response は object 2 を含む");
}

/// NATIVE-REAL-11c: 3 target で multi-object response content が安定すること
#[test]
fn test_native_linker_multi_object_response_consistency_across_three_targets() {
    let output = run_native_linker_harness(
        r#"(module Main)
(import NativeTarget)
(import Linker)

(defn emit-summary [triple-id object-size]
  (let [target (make-target triple-id)
        objects (vector-push (vector-push (vector-new 2) object-size) object-size)
        response (generate-response-file (build-linker-args objects 99 target))]
    (do
      (print (vector-length response))
      (print (vector-get response 4))
      (print (vector-get response 6)))))

(defn main []
  (do
    (emit-summary 1 276)
    (emit-summary 2 24)
    (emit-summary 3 600)
    0))"#,
    );
    let lines: Vec<&str> = output.trim().lines().collect();

    assert!(
        lines.len() >= 9,
        "3 target multi response 出力が不足: {:?}",
        lines
    );
    assert_eq!(lines[0], "8", "target 1 multi response len は 8 bytes");
    assert_eq!(
        lines[1], "276",
        "target 1 multi response は object 1 size=276 を含む"
    );
    assert_eq!(
        lines[2], "276",
        "target 1 multi response は object 2 size=276 を含む"
    );
    assert_eq!(lines[3], "8", "target 2 multi response len も 8 bytes");
    assert_eq!(
        lines[4], "24",
        "target 2 multi response は object 1 size=24 を含む"
    );
    assert_eq!(
        lines[5], "24",
        "target 2 multi response は object 2 size=24 を含む"
    );
    assert_eq!(lines[6], "8", "target 3 multi response len も 8 bytes");
    assert_eq!(
        lines[7], "600",
        "target 3 multi response は object 1 size=600 を含む"
    );
    assert_eq!(
        lines[8], "600",
        "target 3 multi response は object 2 size=600 を含む"
    );
}
