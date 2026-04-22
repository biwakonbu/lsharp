(module Backend.Wasm.Codegen)
(import IR.IR)

;; Codegen.ls - L# セルフホスティング: IR -> Wasm 命令変換
;;
;; IR 命令列を Wasm オペコードに変換する。
;; 実際のバイナリ emit は Emit.ls が担当。

;; === IR -> Wasm 命令変換 ===

;; IR 命令を Wasm オペコードに変換
;; 戻り値: [wasm-opcode, operand] の Vector
(defn emit-instruction [ir-opcode operand]
  (if (= ir-opcode 1)
    ;; i64.const -> wasm i64.const (0x42)
    (make-instr 66 operand)
    (if (= ir-opcode 10)
      ;; local.get -> wasm local.get (0x20)
      (make-instr 32 operand)
      (if (= ir-opcode 11)
        ;; local.set -> wasm local.set (0x21)
        (make-instr 33 operand)
        (if (= ir-opcode 20)
          ;; i64.add -> wasm i64.add (0x7C)
          (make-instr 124 0)
          (if (= ir-opcode 21)
            ;; i64.sub -> wasm i64.sub (0x7D)
            (make-instr 125 0)
            (if (= ir-opcode 22)
              ;; i64.mul -> wasm i64.mul (0x7E)
              (make-instr 126 0)
              (if (= ir-opcode 23)
                ;; i64.div_s -> wasm i64.div_s (0x7F)
                (make-instr 127 0)
                (if (= ir-opcode 28)
                  ;; i64.rem_s -> wasm i64.rem_s (0x81)
                  (make-instr 129 0)
                  (if (= ir-opcode 45)
                    ;; i32.load -> wasm i32.load (0x28)
                    (make-instr 40 operand)
                    (if (= ir-opcode 46)
                      ;; i32.store -> wasm i32.store (0x36)
                      (make-instr 54 operand)
                      (if (= ir-opcode 47)
                        ;; i32.load8_u -> wasm i32.load8_u (0x2D)
                        (make-instr 45 operand)
                          (if (= ir-opcode 48)
                            ;; i64.load -> wasm i64.load (0x29)
                            (make-instr 41 operand)
                            (if (= ir-opcode 49)
                              ;; i64.store -> wasm i64.store (0x37)
                              (make-instr 55 operand)
                              (if (= ir-opcode 77)
                                ;; memory.copy -> WasmEmit 側の専用 opcode
                                (make-instr 77 0)
                                (if (= ir-opcode 78)
                                  ;; memory.fill -> WasmEmit 側の専用 opcode
                                  (make-instr 78 0)
                              (if (= ir-opcode 30)
                                ;; i64.eq -> wasm i64.eq (0x51)
                                (make-instr 81 0)
                                (if (= ir-opcode 40)
                                  ;; call -> wasm call (0x10)
                                  (make-instr 16 operand)
                                  ;; 未知の命令: nop (0x01)
                                  (make-instr 1 0)))))))))))))))

;; IR 関数を Wasm 関数に変換
;; ir-func: [name, params, body-instrs] の Vector
;; 戻り値: Wasm 命令列の Vector
(defn emit-function [ir-func]
  (let [body (vector-get ir-func 2)
    result (ref-new (vector-new 16))
    i (ref-new 0)
    n (vector-length body)]
    (do
      ;; 各 IR 命令を変換
      (if (< (ref-get i) n)
        (do
          (let [instr (vector-get body (ref-get i))
            opcode (vector-get instr 0)
            operand (vector-get instr 1)
            wasm-instr (emit-instruction opcode operand)]
            (ref-set result (vector-push (ref-get result) wasm-instr)))
          (ref-set i (+ (ref-get i) 1))
          (if (< (ref-get i) n)
            (do
              (let [instr (vector-get body (ref-get i))
                opcode (vector-get instr 0)
                operand (vector-get instr 1)
                wasm-instr (emit-instruction opcode operand)]
                (ref-set result (vector-push (ref-get result) wasm-instr)))
              (ref-set i (+ (ref-get i) 1))
              (if (< (ref-get i) n)
                (do
                  (let [instr (vector-get body (ref-get i))
                    opcode (vector-get instr 0)
                    operand (vector-get instr 1)
                    wasm-instr (emit-instruction opcode operand)]
                    (ref-set result (vector-push (ref-get result) wasm-instr)))
                  (ref-set i (+ (ref-get i) 1))
                  (if (< (ref-get i) n)
                    (do
                      (let [instr (vector-get body (ref-get i))
                        opcode (vector-get instr 0)
                        operand (vector-get instr 1)
                        wasm-instr (emit-instruction opcode operand)]
                        (ref-set result (vector-push (ref-get result) wasm-instr)))
                      0)
                    0))
                0))
            0))
        0)
      (ref-get result))))

;; エントリポイント (テスト用)
(defn main []
  (let [instr (emit-instruction 1 42)]
    (do
      (print (vector-get instr 0)) ;; 66 (wasm i64.const)
      (print (vector-get instr 1)) ;; 42
      0)))
