
/// NATIVE-REAL-10c: 同一 IR からの object emission が 3 target で決定的であること
#[test]
#[ignore]
fn test_native_emit_object_is_deterministic_across_three_targets() {
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
        obj-a (emit-object (emit-native ir target) target)
        obj-b (emit-object (emit-native ir target) target)
        tail-idx (if (= triple-id 1) 246 (if (= triple-id 3) 78 22))
        last-idx (if (= triple-id 1) 247 (if (= triple-id 3) 79 23))]
    (do
      (print (vector-length obj-a))
      (print (vector-length obj-b))
      (print (vector-get obj-a 0))
      (print (vector-get obj-b 0))
      (print (vector-get obj-a 4))
      (print (vector-get obj-b 4))
      (print (vector-get obj-a tail-idx))
      (print (vector-get obj-b tail-idx))
      (print (vector-get obj-a last-idx))
      (print (vector-get obj-b last-idx)))))

(defn main []
  (do
    (emit-summary 1)
    (emit-summary 2)
    (emit-summary 3)
    0))"#,
    );
    let lines: Vec<&str> = output.trim().lines().collect();

    assert!(
        lines.len() >= 30,
        "deterministic object summary 出力が不足: {:?}",
        lines
    );
    for chunk in lines.chunks_exact(10) {
        assert_eq!(
            chunk[0], chunk[1],
            "object len が repeated emission で変化した"
        );
        assert_eq!(
            chunk[2], chunk[3],
            "object byte0 が repeated emission で変化した"
        );
        assert_eq!(
            chunk[4], chunk[5],
            "object byte4 が repeated emission で変化した"
        );
        assert_eq!(
            chunk[6], chunk[7],
            "object tail-1 が repeated emission で変化した"
        );
        assert_eq!(
            chunk[8], chunk[9],
            "object tail が repeated emission で変化した"
        );
    }
    assert_eq!(lines[0], "276", "target 1 object len は 276 bytes");
    assert_eq!(lines[10], "24", "target 2 object len は 24 bytes (AArch64)");
    assert_eq!(lines[20], "600", "target 3 object len は 600 bytes");
}

/// NATIVE-REAL-11d: 同一 object list からの linker response が 3 target で決定的であること
#[test]
fn test_native_linker_response_is_deterministic_across_three_targets() {
    let output = run_native_linker_harness(
        r#"(module Main)
(import NativeTarget)
(import Linker)

(defn emit-summary [triple-id object-size]
  (let [target (make-target triple-id)
        objects (vector-push (vector-push (vector-new 2) object-size) object-size)
        response-a (generate-response-file (build-linker-args objects 99 target))
        response-b (generate-response-file (build-linker-args objects 99 target))]
    (do
      (print (vector-length response-a))
      (print (vector-length response-b))
      (print (vector-get response-a 0))
      (print (vector-get response-b 0))
      (print (vector-get response-a 2))
      (print (vector-get response-b 2))
      (print (vector-get response-a 4))
      (print (vector-get response-b 4))
      (print (vector-get response-a 6))
      (print (vector-get response-b 6)))))

(defn main []
  (do
    (emit-summary 1 276)
    (emit-summary 2 24)
    (emit-summary 3 600)
    0))"#,
    );
    let lines: Vec<&str> = output.trim().lines().collect();

    assert!(
        lines.len() >= 30,
        "deterministic linker summary 出力が不足: {:?}",
        lines
    );
    for chunk in lines.chunks_exact(10) {
        assert_eq!(
            chunk[0], chunk[1],
            "response len が repeated generation で変化した"
        );
        assert_eq!(
            chunk[2], chunk[3],
            "response byte0 が repeated generation で変化した"
        );
        assert_eq!(
            chunk[4], chunk[5],
            "response byte2 が repeated generation で変化した"
        );
        assert_eq!(
            chunk[6], chunk[7],
            "response byte4 が repeated generation で変化した"
        );
        assert_eq!(
            chunk[8], chunk[9],
            "response byte6 が repeated generation で変化した"
        );
    }
    assert_eq!(lines[0], "8", "target 1 response len は 8 bytes");
    assert_eq!(lines[10], "8", "target 2 response len は 8 bytes");
    assert_eq!(lines[20], "8", "target 3 response len は 8 bytes");
}
