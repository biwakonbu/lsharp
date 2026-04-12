(module Backend.Native.NativeCodegen)
(import Backend.Native.NativeTarget)
(import IR.IR)

;; NativeCodegen.ls - L# セルフホスティング: ネイティブコード生成
;;
;; IR 命令列からネイティブ (x86_64 / AArch64) の機械語命令列を生成する。
;; 決定的コード生成 (deterministic codegen) を保証:
;;   同一入力 IR に対して常に同一のバイト列を出力する。
;;
;; 設計方針:
;;   - ソート済みシンボルテーブル (挿入順序に依存しない)
;;   - タイムスタンプや乱数を含まない
;;   - reproducible builds 対応

;; === レジスタ定数 (x86_64) ===
(defn reg-rax [] 0)
(defn reg-rcx [] 1)
(defn reg-rdx [] 2)
(defn reg-rbx [] 3)
(defn reg-rsp [] 4)
(defn reg-rbp [] 5)
(defn reg-rsi [] 6)
(defn reg-rdi [] 7)

;; === レジスタ定数 (AArch64) ===
(defn reg-x0 [] 0)
(defn reg-x1 [] 1)
(defn reg-x29 [] 29)
(defn reg-x30 [] 30)
(defn reg-sp [] 31)

;; === ネイティブ命令エンコーダ ===

;; x86_64 の MOV imm64 命令を生成
;; REX.W + MOV r64, imm64 (0x48 0xB8+rd imm64)
;; 戻り値: バイト列 Vector
(defn emit-mov-imm64 [reg value]
  (let [bytes (vector-new 10)
    ;; REX.W プリフィックス
    b1 (vector-push bytes 72) ;; 0x48 (REX.W)
    ;; MOV opcode + レジスタ
    b2 (vector-push b1 (+ 184 reg)) ;; 0xB8 + rd
    ;; 64bit 即値 (リトルエンディアン、最大8バイト)
    ;; 簡易版: 下位4バイトのみ (上位4バイトは 0)
    byte0 (% value 256)
    byte1 (% (/ value 256) 256)
    byte2 (% (/ value 65536) 256)
    byte3 (% (/ value 16777216) 256)
    b3 (vector-push b2 byte0)
    b4 (vector-push b3 byte1)
    b5 (vector-push b4 byte2)
    b6 (vector-push b5 byte3)
    b7 (vector-push b6 0)
    b8 (vector-push b7 0)
    b9 (vector-push b8 0)
    b10 (vector-push b9 0)]
    b10))

;; x86_64 の RET 命令
(defn emit-ret []
  (vector-push (vector-new 1) 195)) ;; 0xC3

;; x86_64 の PUSH rbp
(defn emit-push-rbp []
  (vector-push (vector-new 1) 85)) ;; 0x55

;; x86_64 の POP rbp
(defn emit-pop-rbp []
  (vector-push (vector-new 1) 93)) ;; 0x5D

;; x86_64 の PUSH rcx
(defn emit-push-rcx []
  (vector-push (vector-new 1) 81)) ;; 0x51

;; x86_64 の POP rcx
(defn emit-pop-rcx []
  (vector-push (vector-new 1) 89)) ;; 0x59

;; x86_64 の MOV rbp, rsp
(defn emit-mov-rbp-rsp []
  (let [bytes (vector-new 3)]
    (vector-push (vector-push (vector-push bytes
          72) ;; 0x48 REX.W
        137) ;; 0x89
      229))) ;; 0xE5 (rsp -> rbp)

;; x86_64 の MOV rcx, rax
(defn emit-mov-rcx-rax []
  (let [bytes (vector-new 3)]
    (vector-push (vector-push (vector-push bytes 72) 137) 193)))

;; x86_64 の MOV rax, rcx
(defn emit-mov-rax-rcx []
  (let [bytes (vector-new 3)]
    (vector-push (vector-push (vector-push bytes 72) 137) 200)))

;; x86_64 の MOV rdi, rax
(defn emit-mov-rdi-rax []
  (let [bytes (vector-new 3)]
    (vector-push (vector-push (vector-push bytes 72) 137) 199)))

;; x86_64 の MOV rsi, rax
(defn emit-mov-rsi-rax []
  (let [bytes (vector-new 3)]
    (vector-push (vector-push (vector-push bytes 72) 137) 198)))

;; x86_64 の MOV rdi, rcx
(defn emit-mov-rdi-rcx []
  (let [bytes (vector-new 3)]
    (vector-push (vector-push (vector-push bytes 72) 137) 207)))

;; x86_64 の MOV eax, imm32
(defn emit-mov-eax-imm32 [value]
  (let [imm (encode-u32-le value)
    bytes (vector-new 5)
    b1 (vector-push bytes 184)
    b2 (vector-push b1 (vector-get imm 0))
    b3 (vector-push b2 (vector-get imm 1))
    b4 (vector-push b3 (vector-get imm 2))
    b5 (vector-push b4 (vector-get imm 3))]
    b5))

;; x86_64 の ADD eax, ecx
(defn emit-add-eax-ecx []
  (let [bytes (vector-new 2)]
    (vector-push (vector-push bytes 1) 200)))

;; x86_64 の IMUL eax, ecx
(defn emit-imul-eax-ecx []
  (let [bytes (vector-new 3)]
    (vector-push (vector-push (vector-push bytes 15) 175) 193)))

;; x86_64 の MOV eax, eax
(defn emit-mov-eax-eax []
  (let [bytes (vector-new 2)]
    (vector-push (vector-push bytes 137) 192)))

;; x86_64 の MOVSXD rax, eax
(defn emit-movsxd-rax-eax []
  (let [bytes (vector-new 3)]
    (vector-push (vector-push (vector-push bytes 72) 99) 192)))

;; 32bit 値を little-endian 4 bytes に分解する
(defn encode-u32-le [value]
  (let [byte0 (% value 256)
    byte1 (% (/ value 256) 256)
    byte2 (% (/ value 65536) 256)
    byte3 (% (/ value 16777216) 256)
    bytes (vector-new 4)]
    (vector-push (vector-push (vector-push (vector-push bytes byte0) byte1) byte2) byte3)))

;; 符号付き 32bit 値を two's complement little-endian 4 bytes に分解する
(defn encode-s32-le [value]
  (if (< value 0)
    (encode-u32-le (+ 4294967296 value))
    (encode-u32-le value)))

;; x86_64 の CALL rel32
(defn emit-call-rel32 [disp]
  (let [imm (encode-s32-le disp)
    bytes (vector-new 5)
    b1 (vector-push bytes 232)
    b2 (vector-push b1 (vector-get imm 0))
    b3 (vector-push b2 (vector-get imm 1))
    b4 (vector-push b3 (vector-get imm 2))
    b5 (vector-push b4 (vector-get imm 3))]
    b5))

;; ローカル変数の stack slot offset (rbp/sp からの byte 数)
(defn local-slot-offset [idx]
  (* (+ idx 1) 8))

;; 16 byte alignment を満たす stack size に丸める
(defn align-16 [value]
  (let [remainder (% value 16)]
    (if (= remainder 0)
      value
      (+ value (- 16 remainder)))))

;; 2 つの byte vector を連結する
(defn concat-byte-vectors-loop [result extra idx len]
  (if (>= idx len)
    result
    (concat-byte-vectors-loop
      (vector-push result (vector-get extra idx))
      extra
      (+ idx 1)
      len)))

(defn concat-byte-vectors [first second]
  (concat-byte-vectors-loop first second 0 (vector-length second)))

;; LocalGet / LocalSet に現れる最大ローカル index を収集
(defn make-local-scan-state [found max-local]
  (vector-push (vector-push (vector-new 2) found) max-local))

(defn local-scan-found [state]
  (vector-get state 0))

(defn local-scan-max [state]
  (vector-get state 1))

(defn update-max-local-index [opcode operand state]
  (if (= opcode 10)
    (if (> operand (local-scan-max state))
      (make-local-scan-state 1 operand)
      (make-local-scan-state 1 (local-scan-max state)))
    (if (= opcode 11)
      (if (> operand (local-scan-max state))
        (make-local-scan-state 1 operand)
        (make-local-scan-state 1 (local-scan-max state)))
      state)))

(defn find-max-local-index-loop [ir-func idx len state]
  (if (>= idx len)
    state
    (let [instr (vector-get ir-func idx)
      opcode (vector-get instr 0)
      operand (vector-get instr 1)
      next-state (update-max-local-index opcode operand state)]
      (find-max-local-index-loop ir-func (+ idx 1) len next-state))))

(defn native-slot-count-from-ir [ir-func]
  (let [state (find-max-local-index-loop ir-func 0 (vector-length ir-func) (make-local-scan-state 0 0))
    found (local-scan-found state)
    max-local (local-scan-max state)]
    (if (= found 0)
      0
      (+ max-local 1))))

(defn native-local-stack-bytes-with-min-slots [ir-func min-slot-count]
  (let [slot-count-from-ir (native-slot-count-from-ir ir-func)
    slot-count (if (> min-slot-count slot-count-from-ir)
                 min-slot-count
                 slot-count-from-ir)]
    (if (= slot-count 0)
      0
      (align-16 (* slot-count 8)))))

(defn native-local-stack-bytes [ir-func]
  (native-local-stack-bytes-with-min-slots ir-func 0))

(defn find-call-loop [ir-func idx len]
  (if (>= idx len)
    0
    (let [instr (vector-get ir-func idx)
      opcode (vector-get instr 0)]
      (if (= opcode 40)
        1
        (find-call-loop ir-func (+ idx 1) len)))))

(defn native-has-call [ir-func]
  (find-call-loop ir-func 0 (vector-length ir-func)))

;; function meta: [param-count, local-count, ir]
(defn make-native-function-meta [param-count local-count ir]
  (vector-push
    (vector-push
      (vector-push (vector-new 3) param-count)
      local-count)
    ir))

(defn native-function-param-count [func-meta]
  (vector-get func-meta 0))

(defn native-function-local-count [func-meta]
  (vector-get func-meta 1))

(defn native-function-ir [func-meta]
  (vector-get func-meta 2))

;; 現在サポートしている IR opcode の stack effect を返す
(defn opcode-stack-delta [opcode operand function-metas]
  (if (= opcode 1)
    1
    (if (= opcode 3)
      1
      (if (= opcode 10)
        1
        (if (= opcode 11)
          -1
          (if (= opcode 20)
            -1
            (if (= opcode 21)
              -1
              (if (= opcode 24)
                -1
                (if (= opcode 25)
                  -1
                  (if (= opcode 36)
                    0
                    (if (= opcode 37)
                      0
                      (if (= opcode 38)
                        0
                        (if (= opcode 40)
                          (- 1 (native-function-param-count (vector-get function-metas operand)))
                          (if (= opcode 44)
                            -1
                            0))))))))))))))

(defn apply-stack-delta [current-depth delta]
  (let [next-depth (+ current-depth delta)]
    (if (< next-depth 0)
      0
      next-depth)))

(defn native-max-stack-depth-loop [ir-func function-metas idx len current-depth max-depth]
  (if (>= idx len)
    max-depth
    (let [instr (vector-get ir-func idx)
      opcode (vector-get instr 0)
      operand (vector-get instr 1)
      next-depth (apply-stack-delta current-depth (opcode-stack-delta opcode operand function-metas))
      next-max (if (> next-depth max-depth) next-depth max-depth)]
      (native-max-stack-depth-loop ir-func function-metas (+ idx 1) len next-depth next-max))))

(defn native-max-stack-depth [ir-func function-metas]
  (native-max-stack-depth-loop ir-func function-metas 0 (vector-length ir-func) 0 0))

;; 現状の partial slice では 3-value window ぶんだけ spill slot を確保する
(defn native-value-window-spill-slot-count [ir-func function-metas]
  (if (> (native-max-stack-depth ir-func function-metas) 2)
    1
    0))

(defn native-frame-base-slot-count [ir-func min-slot-count]
  (let [slot-count-from-ir (native-slot-count-from-ir ir-func)]
    (if (> min-slot-count slot-count-from-ir)
      min-slot-count
      slot-count-from-ir)))

(defn native-total-slot-count-with-window [ir-func min-slot-count function-metas]
  (+ (native-frame-base-slot-count ir-func min-slot-count)
     (native-value-window-spill-slot-count ir-func function-metas)))

(defn native-local-stack-bytes-with-window [ir-func min-slot-count function-metas]
  (let [slot-count (native-total-slot-count-with-window ir-func min-slot-count function-metas)]
    (if (= slot-count 0)
      0
      (align-16 (* slot-count 8)))))

(defn native-value-window-spill-offset [frame-base-slot-count spill-idx]
  (local-slot-offset (+ frame-base-slot-count spill-idx)))

;; x86_64 の SUB rsp, imm32
(defn emit-sub-rsp-imm32 [value]
  (let [imm (encode-u32-le value)
    bytes (vector-new 7)
    b1 (vector-push bytes 72)
    b2 (vector-push b1 129)
    b3 (vector-push b2 236)
    b4 (vector-push b3 (vector-get imm 0))
    b5 (vector-push b4 (vector-get imm 1))
    b6 (vector-push b5 (vector-get imm 2))
    b7 (vector-push b6 (vector-get imm 3))]
    b7))

;; x86_64 の ADD rsp, imm32
(defn emit-add-rsp-imm32 [value]
  (let [imm (encode-u32-le value)
    bytes (vector-new 7)
    b1 (vector-push bytes 72)
    b2 (vector-push b1 129)
    b3 (vector-push b2 196)
    b4 (vector-push b3 (vector-get imm 0))
    b5 (vector-push b4 (vector-get imm 1))
    b6 (vector-push b5 (vector-get imm 2))
    b7 (vector-push b6 (vector-get imm 3))]
    b7))

;; x86_64 の MOV [rbp-offset], rax
(defn emit-mov-local-from-rax [offset]
  (let [disp (encode-u32-le (- 4294967296 offset))
    bytes (vector-new 7)
    b1 (vector-push bytes 72)
    b2 (vector-push b1 137)
    b3 (vector-push b2 133)
    b4 (vector-push b3 (vector-get disp 0))
    b5 (vector-push b4 (vector-get disp 1))
    b6 (vector-push b5 (vector-get disp 2))
     b7 (vector-push b6 (vector-get disp 3))]
      b7))

;; x86_64 の MOV [rbp-offset], rcx
(defn emit-mov-local-from-rcx [offset]
  (let [disp (encode-u32-le (- 4294967296 offset))
    bytes (vector-new 7)
    b1 (vector-push bytes 72)
    b2 (vector-push b1 137)
    b3 (vector-push b2 141)
    b4 (vector-push b3 (vector-get disp 0))
    b5 (vector-push b4 (vector-get disp 1))
    b6 (vector-push b5 (vector-get disp 2))
    b7 (vector-push b6 (vector-get disp 3))]
    b7))

;; x86_64 の MOV [rbp-offset], rdi
(defn emit-mov-local-from-rdi [offset]
  (let [disp (encode-u32-le (- 4294967296 offset))
    bytes (vector-new 7)
    b1 (vector-push bytes 72)
    b2 (vector-push b1 137)
    b3 (vector-push b2 189)
    b4 (vector-push b3 (vector-get disp 0))
    b5 (vector-push b4 (vector-get disp 1))
    b6 (vector-push b5 (vector-get disp 2))
    b7 (vector-push b6 (vector-get disp 3))]
     b7))

;; x86_64 の MOV [rbp-offset], rdx
(defn emit-mov-local-from-rdx [offset]
  (let [disp (encode-u32-le (- 4294967296 offset))
    bytes (vector-new 7)
    b1 (vector-push bytes 72)
    b2 (vector-push b1 137)
    b3 (vector-push b2 149)
    b4 (vector-push b3 (vector-get disp 0))
    b5 (vector-push b4 (vector-get disp 1))
    b6 (vector-push b5 (vector-get disp 2))
    b7 (vector-push b6 (vector-get disp 3))]
    b7))

;; x86_64 の MOV [rbp-offset], rsi
(defn emit-mov-local-from-rsi [offset]
  (let [disp (encode-u32-le (- 4294967296 offset))
    bytes (vector-new 7)
    b1 (vector-push bytes 72)
    b2 (vector-push b1 137)
    b3 (vector-push b2 181)
    b4 (vector-push b3 (vector-get disp 0))
    b5 (vector-push b4 (vector-get disp 1))
    b6 (vector-push b5 (vector-get disp 2))
    b7 (vector-push b6 (vector-get disp 3))]
     b7))

;; x86_64 の MOV rax, [rbp-offset]
(defn emit-mov-rax-from-local [offset]
  (let [disp (encode-u32-le (- 4294967296 offset))
    bytes (vector-new 7)
    b1 (vector-push bytes 72)
    b2 (vector-push b1 139)
    b3 (vector-push b2 133)
    b4 (vector-push b3 (vector-get disp 0))
    b5 (vector-push b4 (vector-get disp 1))
    b6 (vector-push b5 (vector-get disp 2))
    b7 (vector-push b6 (vector-get disp 3))]
    b7))

;; x86_64 の MOV rdi, [rbp-offset]
(defn emit-mov-rdi-from-local [offset]
  (let [disp (encode-u32-le (- 4294967296 offset))
    bytes (vector-new 7)
    b1 (vector-push bytes 72)
    b2 (vector-push b1 139)
    b3 (vector-push b2 189)
    b4 (vector-push b3 (vector-get disp 0))
    b5 (vector-push b4 (vector-get disp 1))
    b6 (vector-push b5 (vector-get disp 2))
    b7 (vector-push b6 (vector-get disp 3))]
    b7))

;; x86_64 の MOV rcx, [rbp-offset]
(defn emit-mov-rcx-from-local [offset]
  (let [disp (encode-u32-le (- 4294967296 offset))
    bytes (vector-new 7)
    b1 (vector-push bytes 72)
    b2 (vector-push b1 139)
    b3 (vector-push b2 141)
    b4 (vector-push b3 (vector-get disp 0))
    b5 (vector-push b4 (vector-get disp 1))
    b6 (vector-push b5 (vector-get disp 2))
    b7 (vector-push b6 (vector-get disp 3))]
    b7))

;; x86_64 の MOV rdx, rax
(defn emit-mov-rdx-rax []
  (let [bytes (vector-new 3)]
    (vector-push (vector-push (vector-push bytes 72) 137) 194)))

;; x86_64 の MOV rsi, rcx
(defn emit-mov-rsi-rcx []
  (let [bytes (vector-new 3)]
    (vector-push (vector-push (vector-push bytes 72) 137) 206)))

;; x86_64 の local.get: 直前値を rcx へ逃がしてから rax へ load
(defn emit-local-get-x86 [offset]
  (let [load (emit-mov-rax-from-local offset)
    bytes (vector-new 10)
    b1 (vector-push bytes 72)
    b2 (vector-push b1 137)
    b3 (vector-push b2 193)
    b4 (vector-push b3 (vector-get load 0))
    b5 (vector-push b4 (vector-get load 1))
    b6 (vector-push b5 (vector-get load 2))
    b7 (vector-push b6 (vector-get load 3))
    b8 (vector-push b7 (vector-get load 4))
    b9 (vector-push b8 (vector-get load 5))
    b10 (vector-push b9 (vector-get load 6))]
    b10))

;; x86_64 の i32.const: 直前値を rcx へ逃がしてから eax へ即値をロード
(defn emit-i32-const-x86 [value]
  (let [mov-imm (emit-mov-eax-imm32 value)
    bytes (vector-new 8)
    b1 (vector-push bytes 72)
    b2 (vector-push b1 137)
    b3 (vector-push b2 193)
    b4 (vector-push b3 (vector-get mov-imm 0))
    b5 (vector-push b4 (vector-get mov-imm 1))
    b6 (vector-push b5 (vector-get mov-imm 2))
    b7 (vector-push b6 (vector-get mov-imm 3))
    b8 (vector-push b7 (vector-get mov-imm 4))]
     b8))

;; x86_64 bundle の i32.const: 3-value window が必要なら old previous を spill する
(defn emit-i32-const-bundle-x86 [value frame-base-slot-count current-depth]
  (if (>= current-depth 2)
    (concat-byte-vectors
      (emit-mov-local-from-rcx (native-value-window-spill-offset frame-base-slot-count 0))
      (emit-i32-const-x86 value))
    (emit-i32-const-x86 value)))

;; x86_64 bundle の local.get: 3-value window が必要なら old previous を spill する
(defn emit-local-get-bundle-x86 [offset frame-base-slot-count current-depth]
  (if (>= current-depth 2)
    (concat-byte-vectors
      (emit-mov-local-from-rcx (native-value-window-spill-offset frame-base-slot-count 0))
      (emit-local-get-x86 offset))
    (emit-local-get-x86 offset)))

(defn emit-three-arg-call-x86 [rel frame-base-slot-count]
  (concat-byte-vectors
    (concat-byte-vectors
      (concat-byte-vectors
        (emit-mov-rdx-rax)
        (emit-mov-rsi-rcx))
      (emit-mov-rdi-from-local (native-value-window-spill-offset frame-base-slot-count 0)))
    (emit-call-rel32 rel)))

(defn emit-two-arg-call-x86 [rel frame-base-slot-count current-depth]
  (let [call-seq (concat-byte-vectors
                   (concat-byte-vectors
                     (emit-mov-rsi-rax)
                     (emit-mov-rdi-rcx))
                   (emit-call-rel32 rel))]
    (if (>= current-depth 3)
      (concat-byte-vectors
        call-seq
        (emit-mov-rcx-from-local (native-value-window-spill-offset frame-base-slot-count 0)))
      call-seq)))

;; === IR -> ネイティブ変換 ===

;; IR opcode をネイティブ命令列に変換 (x86_64)
;; 戻り値: バイト列 Vector
(defn codegen-ir-instr [opcode operand]
  (if (= opcode 1)
    ;; i64.const -> mov rax, imm64
    (emit-mov-imm64 (reg-rax) operand)
    (if (= opcode 3)
      ;; i32.const -> mov eax, imm32
      (emit-i32-const-x86 operand)
      (if (= opcode 10)
        ;; local.get -> rcx へ退避してから mov rax, [rbp-offset]
        (emit-local-get-x86 (local-slot-offset operand))
        (if (= opcode 11)
          ;; local.set -> mov [rbp-offset], rax
          (emit-mov-local-from-rax (local-slot-offset operand))
          (if (= opcode 20)
            ;; i64.add -> add rax, rcx (簡易版)
            ;; 0x48 0x01 0xC8
            (vector-push (vector-push (vector-push (vector-new 3) 72) 1) 200)
            (if (= opcode 21)
              ;; i64.sub -> sub rax, rcx
              ;; 0x48 0x29 0xC8
              (vector-push (vector-push (vector-push (vector-new 3) 72) 41) 200)
              (if (= opcode 24)
                ;; i32.add -> add eax, ecx
                (emit-add-eax-ecx)
                (if (= opcode 25)
                  ;; i32.mul -> imul eax, ecx
                  (emit-imul-eax-ecx)
                  (if (= opcode 36)
                    ;; i64.extend_i32_s -> movsxd rax, eax
                    (emit-movsxd-rax-eax)
                    (if (= opcode 37)
                      ;; i64.extend_i32_u -> mov eax, eax
                      (emit-mov-eax-eax)
                      (if (= opcode 38)
                        ;; i32.wrap_i64 -> mov eax, eax
                        (emit-mov-eax-eax)
                        (if (= opcode 44)
                          ;; drop -> 1 段下の値へ戻す
                          (emit-mov-rax-rcx)
                          ;; 未知の opcode: NOP
                          (vector-push (vector-new 1) 144)))))))))))))) ;; 0x90

(defn native-instr-size-x86 [opcode operand function-metas current-depth]
  (if (= opcode 40)
    (let [target-meta (vector-get function-metas operand)
      target-param-count (native-function-param-count target-meta)]
      (if (= target-param-count 3)
        18
        (if (= target-param-count 2)
        (if (>= current-depth 3) 18 11)
        (if (= target-param-count 1)
          10
          5))))
    (if (= opcode 3)
      (if (>= current-depth 2) 15 8)
      (if (= opcode 10)
        (if (>= current-depth 2) 17 10)
        (vector-length (codegen-ir-instr opcode operand))))))

(defn native-function-body-size-x86-loop [ir-func function-metas idx len total current-depth]
  (if (>= idx len)
    total
    (let [instr (vector-get ir-func idx)
      opcode (vector-get instr 0)
      operand (vector-get instr 1)
      next-total (+ total (native-instr-size-x86 opcode operand function-metas current-depth))
      next-depth (apply-stack-delta current-depth (opcode-stack-delta opcode operand function-metas))]
      (native-function-body-size-x86-loop ir-func function-metas (+ idx 1) len next-total next-depth))))

(defn native-function-size-x86 [func-meta function-metas]
  (let [param-count (native-function-param-count func-meta)
    local-count (native-function-local-count func-meta)
    ir-func (native-function-ir func-meta)
    stack-bytes (native-local-stack-bytes-with-window ir-func (+ param-count local-count) function-metas)
    frame-bytes (if (> stack-bytes 0) 14 0)
    param-spill-bytes (if (= param-count 3)
                        21
                        (if (= param-count 2)
                        14
                        (if (= param-count 1) 7 0)))
    body-bytes (native-function-body-size-x86-loop ir-func function-metas 0 (vector-length ir-func) 0 0)]
    (+ (+ (+ 6 frame-bytes) param-spill-bytes) body-bytes)))

(defn collect-function-starts-x86-loop [functions idx len starts offset]
  (if (>= idx len)
    starts
    (let [func-meta (vector-get functions idx)
      next-starts (vector-push starts offset)
      next-offset (+ offset (native-function-size-x86 func-meta functions))]
      (collect-function-starts-x86-loop functions (+ idx 1) len next-starts next-offset))))

(defn collect-function-starts-x86 [functions]
  (collect-function-starts-x86-loop functions 0 (vector-length functions) (vector-new 8) 0))

(defn codegen-ir-instr-bundle-x86 [opcode operand current-offset function-starts function-metas frame-base-slot-count current-depth]
  (if (= opcode 40)
    (let [target-offset (vector-get function-starts operand)
      target-meta (vector-get function-metas operand)
      target-param-count (native-function-param-count target-meta)
      rel (if (= target-param-count 3)
             (- target-offset (+ current-offset 18))
             (if (= target-param-count 2)
               (- target-offset (+ current-offset 11))
              (if (= target-param-count 1)
                (- target-offset (+ current-offset 9))
                (- target-offset (+ current-offset 5)))))]
      (if (= target-param-count 3)
        (emit-three-arg-call-x86 rel frame-base-slot-count)
        (if (= target-param-count 2)
        (emit-two-arg-call-x86 rel frame-base-slot-count current-depth)
         (if (= target-param-count 1)
          (let [call-bytes (emit-call-rel32 rel)
            push-rcx (emit-push-rcx)
           pop-rcx (emit-pop-rcx)
           bytes (vector-new 10)
           b1 (vector-push bytes 72)
           b2 (vector-push b1 137)
           b3 (vector-push b2 199)
           b4 (vector-push b3 (vector-get push-rcx 0))
           b5 (vector-push b4 (vector-get call-bytes 0))
           b6 (vector-push b5 (vector-get call-bytes 1))
           b7 (vector-push b6 (vector-get call-bytes 2))
           b8 (vector-push b7 (vector-get call-bytes 3))
           b9 (vector-push b8 (vector-get call-bytes 4))
           b10 (vector-push b9 (vector-get pop-rcx 0))]
           b10)
          (emit-call-rel32 rel)))))
    (if (= opcode 3)
      (emit-i32-const-bundle-x86 operand frame-base-slot-count current-depth)
      (if (= opcode 10)
        (emit-local-get-bundle-x86 (local-slot-offset operand) frame-base-slot-count current-depth)
        (codegen-ir-instr opcode operand)))))

(defn generate-native-instr-bundle-loop-x86 [ir-func result function-starts function-metas frame-base-slot-count current-offset current-depth idx len]
  (if (>= idx len)
    current-offset
    (let [instr (vector-get ir-func idx)
      opcode (vector-get instr 0)
      operand (vector-get instr 1)
      native (codegen-ir-instr-bundle-x86 opcode operand current-offset function-starts function-metas frame-base-slot-count current-depth)
      native-len (vector-length native)
      next-depth (apply-stack-delta current-depth (opcode-stack-delta opcode operand function-metas))]
      (do
        (append-native-bytes-loop result native 0 native-len)
        (generate-native-instr-bundle-loop-x86 ir-func result function-starts function-metas frame-base-slot-count (+ current-offset native-len) next-depth (+ idx 1) len)))))

;; === コード生成メイン関数 ===

(defn append-native-bytes-loop [result native idx len]
  (if (>= idx len)
    0
    (do
      (ref-set result (vector-push (ref-get result) (vector-get native idx)))
      (append-native-bytes-loop result native (+ idx 1) len))))

(defn generate-native-instr-loop [ir-func result idx len]
  (if (>= idx len)
    0
    (let [instr (vector-get ir-func idx)
      opcode (vector-get instr 0)
      operand (vector-get instr 1)
      native (codegen-ir-instr opcode operand)
      native-len (vector-length native)]
      (do
        (append-native-bytes-loop result native 0 native-len)
        (generate-native-instr-loop ir-func result (+ idx 1) len)))))

;; === x86_64 コード生成 ===

;; x86_64 IR 関数をネイティブコードに変換 (プロローグ・エピローグ付き)
;; ir-func: IR 命令列の Vector [[opcode, operand], ...]
;; 戻り値: ネイティブ機械語バイト列
(defn generate-native-x86-64 [ir-func]
  (let [result (ref-new (vector-new 64))
    stack-bytes (native-local-stack-bytes ir-func)
    ;; 関数プロローグ
    prologue-push (emit-push-rbp)
    prologue-mov (emit-mov-rbp-rsp)
    _ (ref-set result (vector-push (ref-get result) (vector-get prologue-push 0)))
    _ (ref-set result (vector-push (ref-get result) (vector-get prologue-mov 0)))
    _ (ref-set result (vector-push (ref-get result) (vector-get prologue-mov 1)))
    _ (ref-set result (vector-push (ref-get result) (vector-get prologue-mov 2)))
    _ (if (> stack-bytes 0)
        (append-native-bytes-loop result (emit-sub-rsp-imm32 stack-bytes) 0 7)
        0)
    ;; IR 命令列を順にネイティブ bytes へ落とす
    n (vector-length ir-func)]
    (do
      (generate-native-instr-loop ir-func result 0 n)
      ;; 関数エピローグ
      (let [epilogue-pop (emit-pop-rbp)
        epilogue-ret (emit-ret)]
        (do
          (if (> stack-bytes 0)
            (append-native-bytes-loop result (emit-add-rsp-imm32 stack-bytes) 0 7)
            0)
          (ref-set result (vector-push (ref-get result) (vector-get epilogue-pop 0)))
           (ref-set result (vector-push (ref-get result) (vector-get epilogue-ret 0)))
           (ref-get result))))))

(defn generate-native-function-x86-64-bundle [func-meta result function-starts function-metas function-start]
  (let [param-count (native-function-param-count func-meta)
    local-count (native-function-local-count func-meta)
    ir-func (native-function-ir func-meta)
    frame-base-slot-count (native-frame-base-slot-count ir-func (+ param-count local-count))
    stack-bytes (native-local-stack-bytes-with-window ir-func (+ param-count local-count) function-metas)
    prologue-push (emit-push-rbp)
    prologue-mov (emit-mov-rbp-rsp)
    base-offset (+ function-start 4)
    after-stack-offset (if (> stack-bytes 0) (+ base-offset 7) base-offset)
    body-offset (if (= param-count 3)
                  (+ after-stack-offset 21)
                  (if (= param-count 2)
                  (+ after-stack-offset 14)
                  (if (= param-count 1) (+ after-stack-offset 7) after-stack-offset)))
    n (vector-length ir-func)]
    (do
      (ref-set result (vector-push (ref-get result) (vector-get prologue-push 0)))
      (ref-set result (vector-push (ref-get result) (vector-get prologue-mov 0)))
      (ref-set result (vector-push (ref-get result) (vector-get prologue-mov 1)))
      (ref-set result (vector-push (ref-get result) (vector-get prologue-mov 2)))
      (if (> stack-bytes 0)
        (append-native-bytes-loop result (emit-sub-rsp-imm32 stack-bytes) 0 7)
        0)
      (if (= param-count 3)
        (do
          (append-native-bytes-loop result (emit-mov-local-from-rdi (local-slot-offset 0)) 0 7)
          (append-native-bytes-loop result (emit-mov-local-from-rsi (local-slot-offset 1)) 0 7)
          (append-native-bytes-loop result (emit-mov-local-from-rdx (local-slot-offset 2)) 0 7))
        (if (= param-count 2)
        (do
          (append-native-bytes-loop result (emit-mov-local-from-rdi (local-slot-offset 0)) 0 7)
          (append-native-bytes-loop result (emit-mov-local-from-rsi (local-slot-offset 1)) 0 7))
        (if (= param-count 1)
          (append-native-bytes-loop result (emit-mov-local-from-rdi (local-slot-offset 0)) 0 7)
          0)))
      (generate-native-instr-bundle-loop-x86 ir-func result function-starts function-metas frame-base-slot-count body-offset 0 0 n)
      (if (> stack-bytes 0)
        (append-native-bytes-loop result (emit-add-rsp-imm32 stack-bytes) 0 7)
        0)
      (let [epilogue-pop (emit-pop-rbp)
        epilogue-ret (emit-ret)]
        (do
          (ref-set result (vector-push (ref-get result) (vector-get epilogue-pop 0)))
          (ref-set result (vector-push (ref-get result) (vector-get epilogue-ret 0)))
          0)))))

(defn generate-native-x86-64-bundle-loop [functions result function-starts idx len]
  (if (>= idx len)
    0
    (let [func-meta (vector-get functions idx)
      function-start (vector-get function-starts idx)]
      (do
        (generate-native-function-x86-64-bundle func-meta result function-starts functions function-start)
        (generate-native-x86-64-bundle-loop functions result function-starts (+ idx 1) len)))))

(defn generate-native-x86-64-bundle [functions]
  (let [result (ref-new (vector-new 128))
    function-starts (collect-function-starts-x86 functions)
    n (vector-length functions)]
    (do
      (generate-native-x86-64-bundle-loop functions result function-starts 0 n)
      (ref-get result))))

;; === AArch64 命令エンコーダ ===

;; AArch64 MOVZ W0, #imm 命令を生成 (imm は 0-65535)
;; エンコーディング: 0x52800000 | (imm << 5) → LE バイト列 4 bytes
;; 例: MOVZ W0, #42 = 0x52800540 → [0x40, 0x05, 0x80, 0x52]
(defn emit-aarch64-movz-w0 [imm]
  (let [encoded (+ 1384120320 (* imm 32))
    b0 (% encoded 256)
    b1 (% (/ encoded 256) 256)
    b2 (% (/ encoded 65536) 256)
    b3 (% (/ encoded 16777216) 256)
    bytes (vector-new 4)]
    (vector-push (vector-push (vector-push (vector-push bytes b0) b1) b2) b3)))

;; AArch64 RET 命令 (X30 経由リターン)
;; エンコーディング: 0xD65F03C0 → [0xC0, 0x03, 0x5F, 0xD6]
(defn emit-aarch64-ret []
  (let [bytes (vector-new 4)]
    (vector-push (vector-push (vector-push (vector-push bytes 192) 3) 95) 214)))

;; AArch64: stp x29, x30, [sp, #-16]!
(defn emit-aarch64-save-fp-lr []
  (let [bytes (vector-new 4)]
    (vector-push (vector-push (vector-push (vector-push bytes 253) 123) 191) 169)))

;; AArch64: ldp x29, x30, [sp], #16
(defn emit-aarch64-restore-fp-lr []
  (let [bytes (vector-new 4)]
    (vector-push (vector-push (vector-push (vector-push bytes 253) 123) 193) 168)))

;; AArch64 BL imm26
(defn emit-aarch64-bl [byte-disp]
  (let [word-disp (/ byte-disp 4)
    imm26 (if (< word-disp 0)
            (+ 67108864 word-disp)
            word-disp)]
    (encode-u32-le (+ 2483027968 imm26))))

;; AArch64 NOP 命令
;; エンコーディング: 0xD503201F → [0x1F, 0x20, 0x03, 0xD5]
(defn emit-aarch64-nop []
  (let [bytes (vector-new 4)]
    (vector-push (vector-push (vector-push (vector-push bytes 31) 32) 3) 213)))

;; AArch64 MOV x1, x0
(defn emit-aarch64-mov-x1-x0 []
  (let [bytes (vector-new 4)]
    (vector-push (vector-push (vector-push (vector-push bytes 225) 3) 0) 170)))

;; AArch64 MOV x1, x9
(defn emit-aarch64-mov-x1-x9 []
  (let [bytes (vector-new 4)]
    (vector-push (vector-push (vector-push (vector-push bytes 225) 3) 9) 170)))

;; AArch64 MOV x9, x0
(defn emit-aarch64-mov-x9-x0 []
  (let [bytes (vector-new 4)]
    (vector-push (vector-push (vector-push (vector-push bytes 233) 3) 0) 170)))

;; AArch64 MOV x0, x1
(defn emit-aarch64-mov-x0-x1 []
  (let [bytes (vector-new 4)]
    (vector-push (vector-push (vector-push (vector-push bytes 224) 3) 1) 170)))

;; AArch64 MOV x0, x9
(defn emit-aarch64-mov-x0-x9 []
  (let [bytes (vector-new 4)]
    (vector-push (vector-push (vector-push (vector-push bytes 224) 3) 9) 170)))

;; AArch64 MOV x2, x0
(defn emit-aarch64-mov-x2-x0 []
  (let [bytes (vector-new 4)]
    (vector-push (vector-push (vector-push (vector-push bytes 226) 3) 0) 170)))

;; AArch64 MOV x10, x9
(defn emit-aarch64-mov-x10-x9 []
  (let [bytes (vector-new 4)]
    (vector-push (vector-push (vector-push (vector-push bytes 234) 3) 9) 170)))

;; AArch64 MOV x9, x10
(defn emit-aarch64-mov-x9-x10 []
  (let [bytes (vector-new 4)]
    (vector-push (vector-push (vector-push (vector-push bytes 233) 3) 10) 170)))

;; AArch64 ADD w0, w1, w0
(defn emit-aarch64-add-w0-w1-w0 []
  (let [bytes (vector-new 4)]
    (vector-push (vector-push (vector-push (vector-push bytes 32) 0) 0) 11)))

;; AArch64 ADD w0, w9, w0
(defn emit-aarch64-add-w0-w9-w0 []
  (let [bytes (vector-new 4)]
    (vector-push (vector-push (vector-push (vector-push bytes 32) 1) 0) 11)))

;; AArch64 MUL w0, w1, w0
(defn emit-aarch64-mul-w0-w1-w0 []
  (let [bytes (vector-new 4)]
    (vector-push (vector-push (vector-push (vector-push bytes 32) 124) 0) 27)))

;; AArch64 MUL w0, w9, w0
(defn emit-aarch64-mul-w0-w9-w0 []
  (let [bytes (vector-new 4)]
    (vector-push (vector-push (vector-push (vector-push bytes 32) 125) 0) 27)))

;; AArch64 MOV w0, w0
(defn emit-aarch64-mov-w0-w0 []
  (let [bytes (vector-new 4)]
    (vector-push (vector-push (vector-push (vector-push bytes 224) 3) 0) 42)))

;; AArch64 SXTW x0, w0
(defn emit-aarch64-sxtw-x0-w0 []
  (let [bytes (vector-new 4)]
    (vector-push (vector-push (vector-push (vector-push bytes 0) 124) 64) 147)))

;; AArch64 SUB sp, sp, #imm
(defn emit-aarch64-sub-sp [imm]
  (encode-u32-le (+ (+ 3506438144 (* imm 1024)) 1023)))

;; AArch64 ADD sp, sp, #imm
(defn emit-aarch64-add-sp [imm]
  (encode-u32-le (+ (+ 2432696320 (* imm 1024)) 1023)))

;; AArch64 STR x0, [sp, #offset]
(defn emit-aarch64-str-x0-sp [offset]
  (let [scaled (/ offset 8)]
    (encode-u32-le (+ (+ 4177526784 (* scaled 1024)) 992))))

;; AArch64 STR x1, [sp, #offset]
(defn emit-aarch64-str-x1-sp [offset]
  (let [scaled (/ offset 8)]
    (encode-u32-le (+ (+ 4177526785 (* scaled 1024)) 992))))

;; AArch64 STR x2, [sp, #offset]
(defn emit-aarch64-str-x2-sp [offset]
  (let [scaled (/ offset 8)]
    (encode-u32-le (+ (+ 4177526786 (* scaled 1024)) 992))))

;; AArch64 STR x9, [sp, #offset]
(defn emit-aarch64-str-x9-sp [offset]
  (let [scaled (/ offset 8)]
    (encode-u32-le (+ (+ 4177526793 (* scaled 1024)) 992))))

;; AArch64 LDR x0, [sp, #offset]
(defn emit-aarch64-ldr-x0-sp [offset]
  (let [scaled (/ offset 8)]
    (encode-u32-le (+ (+ 4181721088 (* scaled 1024)) 992))))

;; AArch64 LDR x9, [sp, #offset]
(defn emit-aarch64-ldr-x9-sp [offset]
  (let [scaled (/ offset 8)]
    (encode-u32-le (+ (+ 4181721097 (* scaled 1024)) 992))))

;; AArch64 の local.get: 直前値を x9 へ退避してから x0 へ load
(defn emit-local-get-aarch64 [offset]
  (let [load (emit-aarch64-ldr-x0-sp offset)
    bytes (vector-new 8)
    b1 (vector-push bytes 233)
    b2 (vector-push b1 3)
    b3 (vector-push b2 0)
    b4 (vector-push b3 170)
    b5 (vector-push b4 (vector-get load 0))
    b6 (vector-push b5 (vector-get load 1))
    b7 (vector-push b6 (vector-get load 2))
    b8 (vector-push b7 (vector-get load 3))]
    b8))

;; AArch64 の i32.const: 直前値を x9 へ退避してから w0 へ即値をロード
(defn emit-i32-const-aarch64 [value]
  (let [movz (emit-aarch64-movz-w0 value)
    bytes (vector-new 8)
    b1 (vector-push bytes 233)
    b2 (vector-push b1 3)
    b3 (vector-push b2 0)
    b4 (vector-push b3 170)
    b5 (vector-push b4 (vector-get movz 0))
    b6 (vector-push b5 (vector-get movz 1))
    b7 (vector-push b6 (vector-get movz 2))
    b8 (vector-push b7 (vector-get movz 3))]
     b8))

;; AArch64 bundle の i32.const: 3-value window が必要なら old previous を spill する
(defn emit-i32-const-bundle-aarch64 [value frame-base-slot-count current-depth]
  (if (>= current-depth 2)
    (concat-byte-vectors
      (emit-aarch64-str-x9-sp (native-value-window-spill-offset frame-base-slot-count 0))
      (emit-i32-const-aarch64 value))
    (emit-i32-const-aarch64 value)))

;; AArch64 bundle の local.get: 3-value window が必要なら old previous を spill する
(defn emit-local-get-bundle-aarch64 [offset frame-base-slot-count current-depth]
  (if (>= current-depth 2)
    (concat-byte-vectors
      (emit-aarch64-str-x9-sp (native-value-window-spill-offset frame-base-slot-count 0))
      (emit-local-get-aarch64 offset))
    (emit-local-get-aarch64 offset)))

(defn emit-three-arg-call-aarch64 [disp frame-base-slot-count]
  (concat-byte-vectors
    (concat-byte-vectors
      (concat-byte-vectors
        (emit-aarch64-mov-x2-x0)
        (emit-aarch64-mov-x1-x9))
      (emit-aarch64-ldr-x0-sp (native-value-window-spill-offset frame-base-slot-count 0)))
    (emit-aarch64-bl disp)))

(defn emit-two-arg-call-aarch64 [disp frame-base-slot-count current-depth]
  (let [call-seq (concat-byte-vectors
                   (concat-byte-vectors
                     (emit-aarch64-mov-x1-x0)
                     (emit-aarch64-mov-x0-x9))
                   (emit-aarch64-bl disp))]
    (if (>= current-depth 3)
      (concat-byte-vectors
        call-seq
        (emit-aarch64-ldr-x9-sp (native-value-window-spill-offset frame-base-slot-count 0)))
      call-seq)))

;; IR opcode を AArch64 命令列に変換
(defn codegen-ir-instr-aarch64 [opcode operand]
    (if (= opcode 1)
    ;; i64.const -> MOVZ W0, #operand
    (emit-aarch64-movz-w0 operand)
    (if (= opcode 3)
      ;; i32.const -> MOV x9, x0; MOVZ W0, #operand
      (emit-i32-const-aarch64 operand)
      (if (= opcode 10)
        ;; local.get -> MOV x9, x0; LDR x0, [sp, #offset]
        (emit-local-get-aarch64 (local-slot-offset operand))
        (if (= opcode 11)
          ;; local.set -> STR x0, [sp, #offset]
          (emit-aarch64-str-x0-sp (local-slot-offset operand))
          (if (= opcode 24)
            ;; i32.add -> add w0, w9, w0
            (emit-aarch64-add-w0-w9-w0)
            (if (= opcode 25)
              ;; i32.mul -> mul w0, w9, w0
              (emit-aarch64-mul-w0-w9-w0)
              (if (= opcode 36)
                ;; i64.extend_i32_s -> sxtw x0, w0
                (emit-aarch64-sxtw-x0-w0)
                (if (= opcode 37)
                  ;; i64.extend_i32_u -> mov w0, w0
                  (emit-aarch64-mov-w0-w0)
                  (if (= opcode 38)
                    ;; i32.wrap_i64 -> mov w0, w0
                    (emit-aarch64-mov-w0-w0)
                    (if (= opcode 44)
                      ;; drop -> 1 段下の値へ戻す
                      (emit-aarch64-mov-x0-x9)
                       ;; 未知の opcode: NOP
                       (emit-aarch64-nop))))))))))))

(defn native-instr-size-aarch64 [opcode operand function-metas current-depth]
  (if (= opcode 40)
    (let [target-meta (vector-get function-metas operand)
      target-param-count (native-function-param-count target-meta)]
      (if (= target-param-count 3)
        16
        (if (= target-param-count 2)
        (if (>= current-depth 3) 16 12)
        (if (= target-param-count 1)
          12
          4))))
    (if (= opcode 3)
      (if (>= current-depth 2) 12 8)
      (if (= opcode 10)
        (if (>= current-depth 2) 12 8)
        (vector-length (codegen-ir-instr-aarch64 opcode operand))))))

(defn native-function-body-size-aarch64-loop [ir-func function-metas idx len total current-depth]
  (if (>= idx len)
    total
    (let [instr (vector-get ir-func idx)
      opcode (vector-get instr 0)
      operand (vector-get instr 1)
      next-total (+ total (native-instr-size-aarch64 opcode operand function-metas current-depth))
      next-depth (apply-stack-delta current-depth (opcode-stack-delta opcode operand function-metas))]
      (native-function-body-size-aarch64-loop ir-func function-metas (+ idx 1) len next-total next-depth))))

(defn native-function-size-aarch64 [func-meta function-metas]
  (let [param-count (native-function-param-count func-meta)
    local-count (native-function-local-count func-meta)
    ir-func (native-function-ir func-meta)
    stack-bytes (native-local-stack-bytes-with-window ir-func (+ param-count local-count) function-metas)
    stack-frame-bytes (if (> stack-bytes 0) 8 0)
    call-frame-bytes (if (= (native-has-call ir-func) 1) 8 0)
    param-spill-bytes (if (= param-count 3)
                        12
                        (if (= param-count 2)
                        8
                        (if (= param-count 1) 4 0)))
    body-bytes (native-function-body-size-aarch64-loop ir-func function-metas 0 (vector-length ir-func) 0 0)]
    (+ (+ (+ (+ 4 stack-frame-bytes) call-frame-bytes) param-spill-bytes) body-bytes)))

(defn collect-function-starts-aarch64-loop [functions idx len starts offset]
  (if (>= idx len)
    starts
    (let [func-meta (vector-get functions idx)
      next-starts (vector-push starts offset)
      next-offset (+ offset (native-function-size-aarch64 func-meta functions))]
      (collect-function-starts-aarch64-loop functions (+ idx 1) len next-starts next-offset))))

(defn collect-function-starts-aarch64 [functions]
  (collect-function-starts-aarch64-loop functions 0 (vector-length functions) (vector-new 8) 0))

(defn codegen-ir-instr-bundle-aarch64 [opcode operand current-offset function-starts function-metas frame-base-slot-count current-depth]
  (if (= opcode 40)
    (let [target-offset (vector-get function-starts operand)
      target-meta (vector-get function-metas operand)
      target-param-count (native-function-param-count target-meta)
      disp (if (= target-param-count 3)
              (- target-offset (+ current-offset 12))
              (if (= target-param-count 2)
              (- target-offset (+ current-offset 8))
              (if (= target-param-count 1)
                (- target-offset (+ current-offset 4))
                (- target-offset current-offset))))]
      (if (= target-param-count 3)
        (emit-three-arg-call-aarch64 disp frame-base-slot-count)
        (if (= target-param-count 2)
        (emit-two-arg-call-aarch64 disp frame-base-slot-count current-depth)
         (if (= target-param-count 1)
            (let [save-prev (emit-aarch64-mov-x10-x9)
             call-bytes (emit-aarch64-bl disp)
             restore-prev (emit-aarch64-mov-x9-x10)
             bytes (vector-new 12)
             b1 (vector-push bytes (vector-get save-prev 0))
             b2 (vector-push b1 (vector-get save-prev 1))
             b3 (vector-push b2 (vector-get save-prev 2))
             b4 (vector-push b3 (vector-get save-prev 3))
             b5 (vector-push b4 (vector-get call-bytes 0))
             b6 (vector-push b5 (vector-get call-bytes 1))
             b7 (vector-push b6 (vector-get call-bytes 2))
             b8 (vector-push b7 (vector-get call-bytes 3))
             b9 (vector-push b8 (vector-get restore-prev 0))
             b10 (vector-push b9 (vector-get restore-prev 1))
             b11 (vector-push b10 (vector-get restore-prev 2))
             b12 (vector-push b11 (vector-get restore-prev 3))]
              b12)
            (emit-aarch64-bl disp)))))
    (if (= opcode 3)
      (emit-i32-const-bundle-aarch64 operand frame-base-slot-count current-depth)
      (if (= opcode 10)
        (emit-local-get-bundle-aarch64 (local-slot-offset operand) frame-base-slot-count current-depth)
        (codegen-ir-instr-aarch64 opcode operand)))))

(defn generate-native-instr-bundle-loop-aarch64 [ir-func result function-starts function-metas frame-base-slot-count current-offset current-depth idx len]
  (if (>= idx len)
    current-offset
    (let [instr (vector-get ir-func idx)
      opcode (vector-get instr 0)
      operand (vector-get instr 1)
      native (codegen-ir-instr-bundle-aarch64 opcode operand current-offset function-starts function-metas frame-base-slot-count current-depth)
      native-len (vector-length native)
      next-depth (apply-stack-delta current-depth (opcode-stack-delta opcode operand function-metas))]
      (do
        (append-native-bytes-loop result native 0 native-len)
        (generate-native-instr-bundle-loop-aarch64 ir-func result function-starts function-metas frame-base-slot-count (+ current-offset native-len) next-depth (+ idx 1) len)))))

;; === AArch64 コード生成 ===

(defn generate-native-instr-loop-aarch64 [ir-func result idx len]
  (if (>= idx len)
    0
    (let [instr (vector-get ir-func idx)
      opcode (vector-get instr 0)
      operand (vector-get instr 1)
      native (codegen-ir-instr-aarch64 opcode operand)
      native-len (vector-length native)]
      (do
        (append-native-bytes-loop result native 0 native-len)
        (generate-native-instr-loop-aarch64 ir-func result (+ idx 1) len)))))

;; AArch64 IR 関数をネイティブコードに変換 (プロローグなし、末尾 RET のみ)
;; ir-func: IR 命令列の Vector [[opcode, operand], ...]
;; 戻り値: AArch64 機械語バイト列
(defn generate-native-aarch64 [ir-func]
  (let [result (ref-new (vector-new 16))
    stack-bytes (native-local-stack-bytes ir-func)
    has-call (native-has-call ir-func)
    n (vector-length ir-func)]
    (do
      (if (= has-call 1)
        (append-native-bytes-loop result (emit-aarch64-save-fp-lr) 0 4)
        0)
      (if (> stack-bytes 0)
        (append-native-bytes-loop result (emit-aarch64-sub-sp stack-bytes) 0 4)
        0)
      (generate-native-instr-loop-aarch64 ir-func result 0 n)
      (let [ret-bytes (emit-aarch64-ret)]
        (do
          (if (> stack-bytes 0)
            (append-native-bytes-loop result (emit-aarch64-add-sp stack-bytes) 0 4)
            0)
          (if (= has-call 1)
            (append-native-bytes-loop result (emit-aarch64-restore-fp-lr) 0 4)
            0)
          (append-native-bytes-loop result ret-bytes 0 4)
          (ref-get result))))))

(defn generate-native-function-aarch64-bundle [func-meta result function-starts function-metas function-start]
  (let [param-count (native-function-param-count func-meta)
    local-count (native-function-local-count func-meta)
    ir-func (native-function-ir func-meta)
    frame-base-slot-count (native-frame-base-slot-count ir-func (+ param-count local-count))
    stack-bytes (native-local-stack-bytes-with-window ir-func (+ param-count local-count) function-metas)
    has-call (native-has-call ir-func)
    after-call-save (if (= has-call 1) (+ function-start 4) function-start)
    after-stack-offset (if (> stack-bytes 0) (+ after-call-save 4) after-call-save)
    body-offset (if (= param-count 3)
                  (+ after-stack-offset 12)
                  (if (= param-count 2)
                  (+ after-stack-offset 8)
                  (if (= param-count 1) (+ after-stack-offset 4) after-stack-offset)))
    n (vector-length ir-func)]
    (do
      (if (= has-call 1)
        (append-native-bytes-loop result (emit-aarch64-save-fp-lr) 0 4)
        0)
      (if (> stack-bytes 0)
        (append-native-bytes-loop result (emit-aarch64-sub-sp stack-bytes) 0 4)
        0)
      (if (= param-count 3)
        (do
          (append-native-bytes-loop result (emit-aarch64-str-x0-sp (local-slot-offset 0)) 0 4)
          (append-native-bytes-loop result (emit-aarch64-str-x1-sp (local-slot-offset 1)) 0 4)
          (append-native-bytes-loop result (emit-aarch64-str-x2-sp (local-slot-offset 2)) 0 4))
        (if (= param-count 2)
        (do
          (append-native-bytes-loop result (emit-aarch64-str-x0-sp (local-slot-offset 0)) 0 4)
          (append-native-bytes-loop result (emit-aarch64-str-x1-sp (local-slot-offset 1)) 0 4))
        (if (= param-count 1)
          (append-native-bytes-loop result (emit-aarch64-str-x0-sp (local-slot-offset 0)) 0 4)
          0)))
      (generate-native-instr-bundle-loop-aarch64 ir-func result function-starts function-metas frame-base-slot-count body-offset 0 0 n)
      (if (> stack-bytes 0)
        (append-native-bytes-loop result (emit-aarch64-add-sp stack-bytes) 0 4)
        0)
      (if (= has-call 1)
        (append-native-bytes-loop result (emit-aarch64-restore-fp-lr) 0 4)
        0)
      (append-native-bytes-loop result (emit-aarch64-ret) 0 4))))

(defn generate-native-aarch64-bundle-loop [functions result function-starts idx len]
  (if (>= idx len)
    0
    (let [func-meta (vector-get functions idx)
      function-start (vector-get function-starts idx)]
      (do
        (generate-native-function-aarch64-bundle func-meta result function-starts functions function-start)
        (generate-native-aarch64-bundle-loop functions result function-starts (+ idx 1) len)))))

(defn generate-native-aarch64-bundle [functions]
  (let [result (ref-new (vector-new 128))
    function-starts (collect-function-starts-aarch64 functions)
    n (vector-length functions)]
    (do
      (generate-native-aarch64-bundle-loop functions result function-starts 0 n)
      (ref-get result))))

;; IR 関数をネイティブコードに変換
;; ir-func: IR 命令列の Vector [[opcode, operand], ...]
;; target: ターゲット記述子
;; 戻り値: ネイティブ機械語バイト列
(defn generate-native [ir-func target]
  (let [arch (target-arch target)]
    (if (= arch 2)
      ;; aarch64 → AArch64 命令列
      (generate-native-aarch64 ir-func)
      ;; x86_64 (arch=1) またはデフォルト
      (generate-native-x86-64 ir-func))))

(defn wrap-ir-functions-as-meta-loop [functions idx len result]
  (if (>= idx len)
    result
    (let [ir-func (vector-get functions idx)
      next-result (vector-push result (make-native-function-meta 0 0 ir-func))]
      (wrap-ir-functions-as-meta-loop functions (+ idx 1) len next-result))))

(defn wrap-ir-functions-as-meta [functions]
  (wrap-ir-functions-as-meta-loop functions 0 (vector-length functions) (vector-new 8)))

(defn generate-native-function-meta-bundle [functions target]
  (let [arch (target-arch target)]
    (if (= arch 2)
      (generate-native-aarch64-bundle functions)
      (generate-native-x86-64-bundle functions))))

;; ネイティブコード生成のトップレベル関数
;; source-ir: プログラム全体の IR
;; target: ターゲット記述子
;; 戻り値: ネイティブ機械語バイト列
(defn emit-native [source-ir target]
  (generate-native source-ir target))

(defn emit-native-bundle [functions target]
  (generate-native-function-meta-bundle (wrap-ir-functions-as-meta functions) target))

(defn emit-native-function-meta-bundle [functions target]
  (generate-native-function-meta-bundle functions target))

;; ネイティブコンパイルパイプライン関数
;; IR -> ネイティブコード生成 -> バイト列
(defn compile-to-native [ir target]
  (emit-native ir target))

;; ネイティブコンパイル + 実行関数 (将来の差分比較用)
;; 現在は機械語バイト列を返すのみ
(defn compile-and-run-native [ir target]
  (compile-to-native ir target))

;; === エントリポイント (テスト用) ===

(defn main []
  (let [;; i64.const 42 の IR 命令
    instr (vector-push (vector-push (vector-new 2) 1) 42)
    ir (vector-push (vector-new 2) instr)
    target (make-target 2) ;; aarch64-apple-darwin
    native-code (emit-native ir target)]
    (do
      (print (vector-length native-code)) ;; ネイティブコードのバイト数
      0)))
