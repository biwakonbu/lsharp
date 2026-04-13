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

;; 現状の partial slice では 28-value window ぶんまで spill slot を確保する
(defn native-value-window-spill-slot-count [ir-func function-metas]
  (let [extra-depth (- (native-max-stack-depth ir-func function-metas) 2)]
    (if (< extra-depth 0)
      0
      (if (> extra-depth 26)
        26
        extra-depth))))

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

;; x86_64 の MOV [rbp-offset], r8
(defn emit-mov-local-from-r8 [offset]
  (let [disp (encode-u32-le (- 4294967296 offset))
    bytes (vector-new 7)
    b1 (vector-push bytes 76)
    b2 (vector-push b1 137)
    b3 (vector-push b2 133)
    b4 (vector-push b3 (vector-get disp 0))
    b5 (vector-push b4 (vector-get disp 1))
    b6 (vector-push b5 (vector-get disp 2))
    b7 (vector-push b6 (vector-get disp 3))]
     b7))

;; x86_64 の MOV [rbp-offset], r9
(defn emit-mov-local-from-r9 [offset]
  (let [disp (encode-u32-le (- 4294967296 offset))
    bytes (vector-new 7)
    b1 (vector-push bytes 76)
    b2 (vector-push b1 137)
    b3 (vector-push b2 141)
    b4 (vector-push b3 (vector-get disp 0))
    b5 (vector-push b4 (vector-get disp 1))
    b6 (vector-push b5 (vector-get disp 2))
    b7 (vector-push b6 (vector-get disp 3))]
     b7))

;; x86_64 の MOV [rsp], rax
(defn emit-mov-top-stack-from-rax []
  (let [bytes (vector-new 4)]
    (vector-push (vector-push (vector-push (vector-push bytes 72) 137) 4) 36)))

;; x86_64 の MOV [rsp], rcx
(defn emit-mov-top-stack-from-rcx []
  (let [bytes (vector-new 4)]
    (vector-push (vector-push (vector-push (vector-push bytes 72) 137) 12) 36)))

;; x86_64 の MOV [rsp+8], rax
(defn emit-mov-second-stack-from-rax []
  (let [bytes (vector-new 5)]
    (vector-push
      (vector-push
        (vector-push
          (vector-push
            (vector-push bytes 72)
            137)
          68)
        36)
      8)))

;; x86_64 の MOV [rsp+8], rcx
(defn emit-mov-second-stack-from-rcx []
  (let [bytes (vector-new 5)]
    (vector-push
      (vector-push
        (vector-push
          (vector-push
            (vector-push bytes 72)
            137)
          76)
        36)
      8)))

;; x86_64 の MOV [rsp+16], rax
(defn emit-mov-third-stack-from-rax []
  (let [bytes (vector-new 5)]
    (vector-push
      (vector-push
        (vector-push
          (vector-push
            (vector-push bytes 72)
            137)
          68)
         36)
       16)))

;; x86_64 の MOV [rsp+16], rcx
(defn emit-mov-third-stack-from-rcx []
  (let [bytes (vector-new 5)]
    (vector-push
      (vector-push
        (vector-push
          (vector-push
            (vector-push bytes 72)
            137)
          76)
        36)
      16)))

;; x86_64 の MOV [rsp+24], rax
(defn emit-mov-fourth-stack-from-rax []
  (let [bytes (vector-new 5)]
    (vector-push
      (vector-push
        (vector-push
          (vector-push
            (vector-push bytes 72)
            137)
           68)
         36)
       24)))

;; x86_64 の MOV [rsp+24], rcx
(defn emit-mov-fourth-stack-from-rcx []
  (let [bytes (vector-new 5)]
    (vector-push
      (vector-push
        (vector-push
          (vector-push
            (vector-push bytes 72)
            137)
          76)
        36)
      24)))

;; x86_64 の MOV [rsp+32], rax
(defn emit-mov-fifth-stack-from-rax []
  (let [bytes (vector-new 5)]
    (vector-push
      (vector-push
        (vector-push
          (vector-push
            (vector-push bytes 72)
            137)
          68)
         36)
       32)))

;; x86_64 の MOV [rsp+32], rcx
(defn emit-mov-fifth-stack-from-rcx []
  (let [bytes (vector-new 5)]
    (vector-push
      (vector-push
        (vector-push
          (vector-push
            (vector-push bytes 72)
            137)
          76)
        36)
      32)))

;; x86_64 の MOV [rsp+40], rax
(defn emit-mov-sixth-stack-from-rax []
  (let [bytes (vector-new 5)]
    (vector-push
      (vector-push
        (vector-push
          (vector-push
            (vector-push bytes 72)
            137)
          68)
        36)
      40)))

;; x86_64 の MOV [rsp+40], rcx
(defn emit-mov-sixth-stack-from-rcx []
  (let [bytes (vector-new 5)]
    (vector-push
      (vector-push
        (vector-push
          (vector-push
            (vector-push bytes 72)
            137)
          76)
        36)
      40)))

;; x86_64 の MOV [rsp+48], rax
(defn emit-mov-seventh-stack-from-rax []
  (let [bytes (vector-new 5)]
    (vector-push
      (vector-push
        (vector-push
          (vector-push
            (vector-push bytes 72)
            137)
          68)
        36)
      48)))

;; x86_64 の MOV [rsp+48], rcx
(defn emit-mov-seventh-stack-from-rcx []
  (let [bytes (vector-new 5)]
    (vector-push
      (vector-push
        (vector-push
          (vector-push
            (vector-push bytes 72)
            137)
          76)
        36)
      48)))

;; x86_64 の MOV [rsp+56], rax
(defn emit-mov-eighth-stack-from-rax []
  (let [bytes (vector-new 5)]
    (vector-push
      (vector-push
        (vector-push
          (vector-push
            (vector-push bytes 72)
            137)
          68)
        36)
      56)))

;; x86_64 の MOV [rsp+56], rcx
(defn emit-mov-eighth-stack-from-rcx []
  (let [bytes (vector-new 5)]
    (vector-push
      (vector-push
        (vector-push
          (vector-push
            (vector-push bytes 72)
            137)
          76)
        36)
      56)))

;; x86_64 の MOV [rsp+64], rax
(defn emit-mov-ninth-stack-from-rax []
  (let [bytes (vector-new 5)]
    (vector-push
      (vector-push
        (vector-push
          (vector-push
            (vector-push bytes 72)
            137)
           68)
         36)
       64)))

;; x86_64 の MOV [rsp+64], rcx
(defn emit-mov-ninth-stack-from-rcx []
  (let [bytes (vector-new 5)]
    (vector-push
      (vector-push
        (vector-push
          (vector-push
            (vector-push bytes 72)
            137)
          76)
        36)
      64)))

;; x86_64 の MOV [rsp+72], rax
(defn emit-mov-tenth-stack-from-rax []
  (let [bytes (vector-new 5)]
    (vector-push
      (vector-push
        (vector-push
          (vector-push
            (vector-push bytes 72)
            137)
           68)
         36)
       72)))

;; x86_64 の MOV [rsp+72], rcx
(defn emit-mov-tenth-stack-from-rcx []
  (let [bytes (vector-new 5)]
    (vector-push
      (vector-push
        (vector-push
          (vector-push
            (vector-push bytes 72)
            137)
          76)
        36)
      72)))

;; x86_64 の MOV [rsp+80], rax
(defn emit-mov-eleventh-stack-from-rax []
  (let [bytes (vector-new 5)]
    (vector-push
      (vector-push
        (vector-push
          (vector-push
            (vector-push bytes 72)
            137)
          68)
        36)
      80)))

;; x86_64 の MOV [rsp+80], rcx
(defn emit-mov-eleventh-stack-from-rcx []
  (let [bytes (vector-new 5)]
    (vector-push
      (vector-push
        (vector-push
          (vector-push
            (vector-push bytes 72)
            137)
          76)
        36)
      80)))

;; x86_64 の MOV [rsp+88], rax
(defn emit-mov-twelfth-stack-from-rax []
  (let [bytes (vector-new 5)]
    (vector-push
      (vector-push
        (vector-push
          (vector-push
            (vector-push bytes 72)
            137)
          68)
        36)
      88)))

;; x86_64 の MOV [rsp+88], rcx
(defn emit-mov-twelfth-stack-from-rcx []
  (let [bytes (vector-new 5)]
    (vector-push
      (vector-push
        (vector-push
          (vector-push
            (vector-push bytes 72)
            137)
          76)
        36)
      88)))

;; x86_64 の MOV [rsp+96], rax
(defn emit-mov-thirteenth-stack-from-rax []
  (let [bytes (vector-new 5)]
    (vector-push
      (vector-push
        (vector-push
          (vector-push
            (vector-push bytes 72)
            137)
           68)
         36)
      96)))

;; x86_64 の MOV [rsp+96], rcx
(defn emit-mov-thirteenth-stack-from-rcx []
  (let [bytes (vector-new 5)]
    (vector-push
      (vector-push
        (vector-push
          (vector-push
            (vector-push bytes 72)
            137)
          76)
        36)
      96)))

;; x86_64 の MOV [rsp+104], rax
(defn emit-mov-fourteenth-stack-from-rax []
  (let [bytes (vector-new 5)]
    (vector-push
      (vector-push
        (vector-push
          (vector-push
            (vector-push bytes 72)
            137)
          68)
         36)
       104)))

;; x86_64 の MOV [rsp+104], rcx
(defn emit-mov-fourteenth-stack-from-rcx []
  (let [bytes (vector-new 5)]
    (vector-push
      (vector-push
        (vector-push
          (vector-push
            (vector-push bytes 72)
            137)
          76)
        36)
      104)))

;; x86_64 の MOV [rsp+112], rax
(defn emit-mov-fifteenth-stack-from-rax []
  (let [bytes (vector-new 5)]
    (vector-push
      (vector-push
        (vector-push
          (vector-push
            (vector-push bytes 72)
            137)
          68)
        36)
      112)))

;; x86_64 の MOV [rsp+112], rcx
(defn emit-mov-fifteenth-stack-from-rcx []
  (let [bytes (vector-new 5)]
    (vector-push
      (vector-push
        (vector-push
          (vector-push
            (vector-push bytes 72)
            137)
          76)
        36)
      112)))

;; x86_64 の MOV [rsp+120], rax
(defn emit-mov-sixteenth-stack-from-rax []
  (let [bytes (vector-new 5)]
    (vector-push
      (vector-push
        (vector-push
          (vector-push
            (vector-push bytes 72)
            137)
          68)
        36)
      120)))

;; x86_64 の MOV [rsp+120], rcx
(defn emit-mov-sixteenth-stack-from-rcx []
  (let [bytes (vector-new 5)]
    (vector-push
      (vector-push
        (vector-push
          (vector-push
            (vector-push bytes 72)
            137)
          76)
        36)
      120)))

;; x86_64 の MOV [rsp+128], rax
(defn emit-mov-seventeenth-stack-from-rax []
  (let [disp (encode-u32-le 128)
    bytes (vector-new 8)
    b1 (vector-push bytes 72)
    b2 (vector-push b1 137)
    b3 (vector-push b2 132)
    b4 (vector-push b3 36)
    b5 (vector-push b4 (vector-get disp 0))
    b6 (vector-push b5 (vector-get disp 1))
    b7 (vector-push b6 (vector-get disp 2))
    b8 (vector-push b7 (vector-get disp 3))]
    b8))

;; x86_64 の MOV [rsp+128], rcx
(defn emit-mov-seventeenth-stack-from-rcx []
  (let [disp (encode-u32-le 128)
    bytes (vector-new 8)
    b1 (vector-push bytes 72)
    b2 (vector-push b1 137)
    b3 (vector-push b2 140)
    b4 (vector-push b3 36)
    b5 (vector-push b4 (vector-get disp 0))
    b6 (vector-push b5 (vector-get disp 1))
    b7 (vector-push b6 (vector-get disp 2))
    b8 (vector-push b7 (vector-get disp 3))]
    b8))

;; x86_64 の MOV [rsp+136], rax
(defn emit-mov-eighteenth-stack-from-rax []
  (let [disp (encode-u32-le 136)
    bytes (vector-new 8)
    b1 (vector-push bytes 72)
    b2 (vector-push b1 137)
    b3 (vector-push b2 132)
    b4 (vector-push b3 36)
    b5 (vector-push b4 (vector-get disp 0))
    b6 (vector-push b5 (vector-get disp 1))
    b7 (vector-push b6 (vector-get disp 2))
    b8 (vector-push b7 (vector-get disp 3))]
    b8))

;; x86_64 の MOV [rsp+136], rcx
(defn emit-mov-eighteenth-stack-from-rcx []
  (let [disp (encode-u32-le 136)
    bytes (vector-new 8)
    b1 (vector-push bytes 72)
    b2 (vector-push b1 137)
    b3 (vector-push b2 140)
    b4 (vector-push b3 36)
    b5 (vector-push b4 (vector-get disp 0))
    b6 (vector-push b5 (vector-get disp 1))
    b7 (vector-push b6 (vector-get disp 2))
    b8 (vector-push b7 (vector-get disp 3))]
    b8))

;; x86_64 の MOV [rsp+144], rax
(defn emit-mov-nineteenth-stack-from-rax []
  (let [disp (encode-u32-le 144)
    bytes (vector-new 8)
    b1 (vector-push bytes 72)
    b2 (vector-push b1 137)
    b3 (vector-push b2 132)
    b4 (vector-push b3 36)
    b5 (vector-push b4 (vector-get disp 0))
    b6 (vector-push b5 (vector-get disp 1))
    b7 (vector-push b6 (vector-get disp 2))
     b8 (vector-push b7 (vector-get disp 3))]
     b8))

;; x86_64 の MOV [rsp+144], rcx
(defn emit-mov-nineteenth-stack-from-rcx []
  (let [disp (encode-u32-le 144)
    bytes (vector-new 8)
    b1 (vector-push bytes 72)
    b2 (vector-push b1 137)
    b3 (vector-push b2 140)
    b4 (vector-push b3 36)
    b5 (vector-push b4 (vector-get disp 0))
    b6 (vector-push b5 (vector-get disp 1))
    b7 (vector-push b6 (vector-get disp 2))
    b8 (vector-push b7 (vector-get disp 3))]
    b8))

;; x86_64 の MOV [rsp+152], rax
(defn emit-mov-twentieth-stack-from-rax []
  (let [disp (encode-u32-le 152)
    bytes (vector-new 8)
    b1 (vector-push bytes 72)
    b2 (vector-push b1 137)
    b3 (vector-push b2 132)
    b4 (vector-push b3 36)
    b5 (vector-push b4 (vector-get disp 0))
    b6 (vector-push b5 (vector-get disp 1))
     b7 (vector-push b6 (vector-get disp 2))
     b8 (vector-push b7 (vector-get disp 3))]
     b8))

;; x86_64 の MOV [rsp+152], rcx
(defn emit-mov-twentieth-stack-from-rcx []
  (let [disp (encode-u32-le 152)
    bytes (vector-new 8)
    b1 (vector-push bytes 72)
    b2 (vector-push b1 137)
    b3 (vector-push b2 140)
    b4 (vector-push b3 36)
    b5 (vector-push b4 (vector-get disp 0))
    b6 (vector-push b5 (vector-get disp 1))
    b7 (vector-push b6 (vector-get disp 2))
    b8 (vector-push b7 (vector-get disp 3))]
    b8))

;; x86_64 の MOV [rsp+160], rax
(defn emit-mov-twenty-first-stack-from-rax []
  (let [disp (encode-u32-le 160)
    bytes (vector-new 8)
    b1 (vector-push bytes 72)
    b2 (vector-push b1 137)
    b3 (vector-push b2 132)
    b4 (vector-push b3 36)
    b5 (vector-push b4 (vector-get disp 0))
    b6 (vector-push b5 (vector-get disp 1))
    b7 (vector-push b6 (vector-get disp 2))
    b8 (vector-push b7 (vector-get disp 3))]
    b8))

;; x86_64 の MOV [rsp+160], rcx
(defn emit-mov-twenty-first-stack-from-rcx []
  (let [disp (encode-u32-le 160)
    bytes (vector-new 8)
    b1 (vector-push bytes 72)
    b2 (vector-push b1 137)
    b3 (vector-push b2 140)
    b4 (vector-push b3 36)
    b5 (vector-push b4 (vector-get disp 0))
    b6 (vector-push b5 (vector-get disp 1))
    b7 (vector-push b6 (vector-get disp 2))
    b8 (vector-push b7 (vector-get disp 3))]
    b8))

;; x86_64 の MOV [rsp+168], rax
(defn emit-mov-twenty-second-stack-from-rax []
  (let [disp (encode-u32-le 168)
    bytes (vector-new 8)
    b1 (vector-push bytes 72)
    b2 (vector-push b1 137)
    b3 (vector-push b2 132)
    b4 (vector-push b3 36)
    b5 (vector-push b4 (vector-get disp 0))
    b6 (vector-push b5 (vector-get disp 1))
    b7 (vector-push b6 (vector-get disp 2))
    b8 (vector-push b7 (vector-get disp 3))]
    b8))

;; x86_64 の MOV [rsp], r9
(defn emit-mov-top-stack-from-r9 []
  (let [bytes (vector-new 4)]
    (vector-push (vector-push (vector-push (vector-push bytes 76) 137) 12) 36)))

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

;; x86_64 の MOV rsi, [rbp-offset]
(defn emit-mov-rsi-from-local [offset]
  (let [disp (encode-u32-le (- 4294967296 offset))
    bytes (vector-new 7)
    b1 (vector-push bytes 72)
    b2 (vector-push b1 139)
    b3 (vector-push b2 181)
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

;; x86_64 の MOV rdx, [rbp-offset]
(defn emit-mov-rdx-from-local [offset]
  (let [disp (encode-u32-le (- 4294967296 offset))
    bytes (vector-new 7)
    b1 (vector-push bytes 72)
    b2 (vector-push b1 139)
    b3 (vector-push b2 149)
    b4 (vector-push b3 (vector-get disp 0))
    b5 (vector-push b4 (vector-get disp 1))
    b6 (vector-push b5 (vector-get disp 2))
    b7 (vector-push b6 (vector-get disp 3))]
    b7))

;; x86_64 の MOV r8, [rbp-offset]
(defn emit-mov-r8-from-local [offset]
  (let [disp (encode-u32-le (- 4294967296 offset))
    bytes (vector-new 7)
    b1 (vector-push bytes 76)
    b2 (vector-push b1 139)
    b3 (vector-push b2 133)
    b4 (vector-push b3 (vector-get disp 0))
    b5 (vector-push b4 (vector-get disp 1))
    b6 (vector-push b5 (vector-get disp 2))
    b7 (vector-push b6 (vector-get disp 3))]
     b7))

;; x86_64 の MOV r9, [rbp-offset]
(defn emit-mov-r9-from-local [offset]
  (let [disp (encode-u32-le (- 4294967296 offset))
    bytes (vector-new 7)
    b1 (vector-push bytes 76)
    b2 (vector-push b1 139)
    b3 (vector-push b2 141)
    b4 (vector-push b3 (vector-get disp 0))
    b5 (vector-push b4 (vector-get disp 1))
    b6 (vector-push b5 (vector-get disp 2))
    b7 (vector-push b6 (vector-get disp 3))]
    b7))

;; x86_64 の MOV rax, [rbp+disp8]
(defn emit-mov-rax-from-rbp-plus-imm8 [offset]
  (let [bytes (vector-new 4)]
    (vector-push
      (vector-push
        (vector-push
          (vector-push bytes 72)
          139)
         69)
       offset)))

;; x86_64 の MOV rax, [rbp+disp32]
(defn emit-mov-rax-from-rbp-plus-imm32 [offset]
  (let [disp (encode-u32-le offset)
    bytes (vector-new 7)
    b1 (vector-push bytes 72)
    b2 (vector-push b1 139)
    b3 (vector-push b2 133)
    b4 (vector-push b3 (vector-get disp 0))
    b5 (vector-push b4 (vector-get disp 1))
    b6 (vector-push b5 (vector-get disp 2))
    b7 (vector-push b6 (vector-get disp 3))]
    b7))

;; x86_64 の MOV rdx, rax
(defn emit-mov-rdx-rax []
  (let [bytes (vector-new 3)]
    (vector-push (vector-push (vector-push bytes 72) 137) 194)))

;; x86_64 の MOV rdx, rcx
(defn emit-mov-rdx-rcx []
  (let [bytes (vector-new 3)]
    (vector-push (vector-push (vector-push bytes 72) 137) 202)))

;; x86_64 の MOV r8, rax
(defn emit-mov-r8-rax []
  (let [bytes (vector-new 3)]
    (vector-push (vector-push (vector-push bytes 73) 137) 192)))

;; x86_64 の MOV r8, rcx
(defn emit-mov-r8-rcx []
  (let [bytes (vector-new 3)]
    (vector-push (vector-push (vector-push bytes 73) 137) 200)))

;; x86_64 の MOV r9, rax
(defn emit-mov-r9-rax []
  (let [bytes (vector-new 3)]
    (vector-push (vector-push (vector-push bytes 73) 137) 193)))

;; x86_64 の MOV r9, rcx
(defn emit-mov-r9-rcx []
  (let [bytes (vector-new 3)]
    (vector-push (vector-push (vector-push bytes 73) 137) 201)))

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

;; x86_64 bundle の i32.const: spill window が必要なら old previous を spill する
(defn emit-i32-const-bundle-x86 [value frame-base-slot-count current-depth]
  (if (>= current-depth 27)
    (concat-byte-vectors
      (concat-byte-vectors
        (emit-mov-rdx-from-local (native-value-window-spill-offset frame-base-slot-count 24))
        (emit-mov-local-from-rdx (native-value-window-spill-offset frame-base-slot-count 25)))
      (emit-i32-const-bundle-x86 value frame-base-slot-count 26))
    (if (>= current-depth 26)
    (concat-byte-vectors
      (concat-byte-vectors
        (emit-mov-rdx-from-local (native-value-window-spill-offset frame-base-slot-count 23))
        (emit-mov-local-from-rdx (native-value-window-spill-offset frame-base-slot-count 24)))
      (emit-i32-const-bundle-x86 value frame-base-slot-count 25))
    (if (>= current-depth 25)
    (concat-byte-vectors
      (concat-byte-vectors
        (emit-mov-rdx-from-local (native-value-window-spill-offset frame-base-slot-count 22))
        (emit-mov-local-from-rdx (native-value-window-spill-offset frame-base-slot-count 23)))
      (emit-i32-const-bundle-x86 value frame-base-slot-count 24))
    (if (>= current-depth 24)
    (concat-byte-vectors
      (concat-byte-vectors
        (emit-mov-rdx-from-local (native-value-window-spill-offset frame-base-slot-count 21))
        (emit-mov-local-from-rdx (native-value-window-spill-offset frame-base-slot-count 22)))
      (emit-i32-const-bundle-x86 value frame-base-slot-count 23))
    (if (>= current-depth 23)
    (concat-byte-vectors
      (concat-byte-vectors
        (emit-mov-rdx-from-local (native-value-window-spill-offset frame-base-slot-count 20))
        (emit-mov-local-from-rdx (native-value-window-spill-offset frame-base-slot-count 21)))
      (emit-i32-const-bundle-x86 value frame-base-slot-count 22))
    (if (>= current-depth 22)
    (concat-byte-vectors
      (concat-byte-vectors
        (emit-mov-rdx-from-local (native-value-window-spill-offset frame-base-slot-count 19))
        (emit-mov-local-from-rdx (native-value-window-spill-offset frame-base-slot-count 20)))
      (emit-i32-const-bundle-x86 value frame-base-slot-count 21))
    (if (>= current-depth 21)
    (concat-byte-vectors
      (concat-byte-vectors
        (emit-mov-rdx-from-local (native-value-window-spill-offset frame-base-slot-count 18))
        (emit-mov-local-from-rdx (native-value-window-spill-offset frame-base-slot-count 19)))
      (emit-i32-const-bundle-x86 value frame-base-slot-count 20))
    (if (>= current-depth 20)
    (concat-byte-vectors
      (concat-byte-vectors
        (emit-mov-rdx-from-local (native-value-window-spill-offset frame-base-slot-count 17))
        (emit-mov-local-from-rdx (native-value-window-spill-offset frame-base-slot-count 18)))
      (emit-i32-const-bundle-x86 value frame-base-slot-count 19))
    (if (>= current-depth 19)
    (concat-byte-vectors
      (concat-byte-vectors
        (emit-mov-rdx-from-local (native-value-window-spill-offset frame-base-slot-count 16))
        (emit-mov-local-from-rdx (native-value-window-spill-offset frame-base-slot-count 17)))
      (emit-i32-const-bundle-x86 value frame-base-slot-count 18))
    (if (>= current-depth 18)
    (concat-byte-vectors
      (concat-byte-vectors
        (emit-mov-rdx-from-local (native-value-window-spill-offset frame-base-slot-count 15))
        (emit-mov-local-from-rdx (native-value-window-spill-offset frame-base-slot-count 16)))
      (emit-i32-const-bundle-x86 value frame-base-slot-count 17))
    (if (>= current-depth 17)
    (concat-byte-vectors
      (concat-byte-vectors
        (emit-mov-rdx-from-local (native-value-window-spill-offset frame-base-slot-count 14))
        (emit-mov-local-from-rdx (native-value-window-spill-offset frame-base-slot-count 15)))
      (emit-i32-const-bundle-x86 value frame-base-slot-count 16))
    (if (>= current-depth 16)
    (concat-byte-vectors
      (concat-byte-vectors
        (emit-mov-rdx-from-local (native-value-window-spill-offset frame-base-slot-count 13))
        (emit-mov-local-from-rdx (native-value-window-spill-offset frame-base-slot-count 14)))
      (emit-i32-const-bundle-x86 value frame-base-slot-count 15))
    (if (>= current-depth 15)
    (concat-byte-vectors
      (concat-byte-vectors
        (emit-mov-rdx-from-local (native-value-window-spill-offset frame-base-slot-count 12))
        (emit-mov-local-from-rdx (native-value-window-spill-offset frame-base-slot-count 13)))
      (emit-i32-const-bundle-x86 value frame-base-slot-count 14))
    (if (>= current-depth 14)
    (concat-byte-vectors
      (concat-byte-vectors
        (emit-mov-rdx-from-local (native-value-window-spill-offset frame-base-slot-count 11))
        (emit-mov-local-from-rdx (native-value-window-spill-offset frame-base-slot-count 12)))
      (emit-i32-const-bundle-x86 value frame-base-slot-count 13))
    (if (>= current-depth 13)
    (concat-byte-vectors
      (concat-byte-vectors
        (emit-mov-rdx-from-local (native-value-window-spill-offset frame-base-slot-count 10))
        (emit-mov-local-from-rdx (native-value-window-spill-offset frame-base-slot-count 11)))
      (emit-i32-const-bundle-x86 value frame-base-slot-count 12))
    (if (>= current-depth 12)
    (concat-byte-vectors
      (concat-byte-vectors
        (emit-mov-rdx-from-local (native-value-window-spill-offset frame-base-slot-count 9))
        (emit-mov-local-from-rdx (native-value-window-spill-offset frame-base-slot-count 10)))
      (emit-i32-const-bundle-x86 value frame-base-slot-count 11))
    (if (>= current-depth 11)
    (concat-byte-vectors
      (concat-byte-vectors
        (emit-mov-rdx-from-local (native-value-window-spill-offset frame-base-slot-count 8))
        (emit-mov-local-from-rdx (native-value-window-spill-offset frame-base-slot-count 9)))
      (if (>= current-depth 10)
        (concat-byte-vectors
          (concat-byte-vectors
            (concat-byte-vectors
              (concat-byte-vectors
                (concat-byte-vectors
                  (concat-byte-vectors
                    (concat-byte-vectors
                      (concat-byte-vectors
                        (concat-byte-vectors
                          (concat-byte-vectors
                            (concat-byte-vectors
                              (concat-byte-vectors
                                (concat-byte-vectors
                                  (concat-byte-vectors
                                    (emit-mov-rdx-from-local (native-value-window-spill-offset frame-base-slot-count 7))
                                    (emit-mov-local-from-rdx (native-value-window-spill-offset frame-base-slot-count 8)))
                                  (emit-mov-rdx-from-local (native-value-window-spill-offset frame-base-slot-count 6)))
                                (emit-mov-local-from-rdx (native-value-window-spill-offset frame-base-slot-count 7)))
                              (emit-mov-rdx-from-local (native-value-window-spill-offset frame-base-slot-count 5)))
                            (emit-mov-local-from-rdx (native-value-window-spill-offset frame-base-slot-count 6)))
                          (emit-mov-rdx-from-local (native-value-window-spill-offset frame-base-slot-count 4)))
                        (emit-mov-local-from-rdx (native-value-window-spill-offset frame-base-slot-count 5)))
                      (emit-mov-rdx-from-local (native-value-window-spill-offset frame-base-slot-count 3)))
                    (emit-mov-local-from-rdx (native-value-window-spill-offset frame-base-slot-count 4)))
                  (emit-mov-rdx-from-local (native-value-window-spill-offset frame-base-slot-count 2)))
                (emit-mov-local-from-rdx (native-value-window-spill-offset frame-base-slot-count 3)))
              (emit-mov-rdx-from-local (native-value-window-spill-offset frame-base-slot-count 1)))
            (emit-mov-local-from-rdx (native-value-window-spill-offset frame-base-slot-count 2)))
          (concat-byte-vectors
            (emit-mov-rdx-from-local (native-value-window-spill-offset frame-base-slot-count 0))
            (concat-byte-vectors
              (emit-mov-local-from-rdx (native-value-window-spill-offset frame-base-slot-count 1))
              (concat-byte-vectors
                (emit-mov-local-from-rcx (native-value-window-spill-offset frame-base-slot-count 0))
                (emit-i32-const-x86 value)))))
        (emit-i32-const-x86 value)))
    (if (>= current-depth 10)
    (concat-byte-vectors
      (concat-byte-vectors
        (concat-byte-vectors
          (concat-byte-vectors
            (concat-byte-vectors
              (concat-byte-vectors
                (concat-byte-vectors
                  (concat-byte-vectors
                    (concat-byte-vectors
                      (concat-byte-vectors
                        (concat-byte-vectors
                          (concat-byte-vectors
                            (concat-byte-vectors
                              (concat-byte-vectors
                                (concat-byte-vectors
                                  (emit-mov-rdx-from-local (native-value-window-spill-offset frame-base-slot-count 7))
                                  (emit-mov-local-from-rdx (native-value-window-spill-offset frame-base-slot-count 8)))
                                (emit-mov-rdx-from-local (native-value-window-spill-offset frame-base-slot-count 6)))
                              (emit-mov-local-from-rdx (native-value-window-spill-offset frame-base-slot-count 7)))
                            (emit-mov-rdx-from-local (native-value-window-spill-offset frame-base-slot-count 5)))
                          (emit-mov-local-from-rdx (native-value-window-spill-offset frame-base-slot-count 6)))
                        (emit-mov-rdx-from-local (native-value-window-spill-offset frame-base-slot-count 4)))
                      (emit-mov-local-from-rdx (native-value-window-spill-offset frame-base-slot-count 5)))
                    (emit-mov-rdx-from-local (native-value-window-spill-offset frame-base-slot-count 3)))
                  (emit-mov-local-from-rdx (native-value-window-spill-offset frame-base-slot-count 4)))
                (emit-mov-rdx-from-local (native-value-window-spill-offset frame-base-slot-count 2)))
              (emit-mov-local-from-rdx (native-value-window-spill-offset frame-base-slot-count 3)))
            (emit-mov-rdx-from-local (native-value-window-spill-offset frame-base-slot-count 1)))
          (emit-mov-local-from-rdx (native-value-window-spill-offset frame-base-slot-count 2)))
        (emit-mov-rdx-from-local (native-value-window-spill-offset frame-base-slot-count 0)))
      (concat-byte-vectors
        (emit-mov-local-from-rdx (native-value-window-spill-offset frame-base-slot-count 1))
        (concat-byte-vectors
          (emit-mov-local-from-rcx (native-value-window-spill-offset frame-base-slot-count 0))
           (emit-i32-const-x86 value))))
    (if (>= current-depth 9)
    (concat-byte-vectors
      (concat-byte-vectors
        (concat-byte-vectors
          (concat-byte-vectors
            (concat-byte-vectors
              (concat-byte-vectors
                (concat-byte-vectors
                  (concat-byte-vectors
                    (concat-byte-vectors
                      (concat-byte-vectors
                        (concat-byte-vectors
                          (concat-byte-vectors
                            (concat-byte-vectors
                              (emit-mov-rdx-from-local (native-value-window-spill-offset frame-base-slot-count 6))
                              (emit-mov-local-from-rdx (native-value-window-spill-offset frame-base-slot-count 7)))
                            (emit-mov-rdx-from-local (native-value-window-spill-offset frame-base-slot-count 5)))
                          (emit-mov-local-from-rdx (native-value-window-spill-offset frame-base-slot-count 6)))
                        (emit-mov-rdx-from-local (native-value-window-spill-offset frame-base-slot-count 4)))
                      (emit-mov-local-from-rdx (native-value-window-spill-offset frame-base-slot-count 5)))
                    (emit-mov-rdx-from-local (native-value-window-spill-offset frame-base-slot-count 3)))
                  (emit-mov-local-from-rdx (native-value-window-spill-offset frame-base-slot-count 4)))
                (emit-mov-rdx-from-local (native-value-window-spill-offset frame-base-slot-count 2)))
              (emit-mov-local-from-rdx (native-value-window-spill-offset frame-base-slot-count 3)))
            (emit-mov-rdx-from-local (native-value-window-spill-offset frame-base-slot-count 1)))
          (emit-mov-local-from-rdx (native-value-window-spill-offset frame-base-slot-count 2)))
        (emit-mov-rdx-from-local (native-value-window-spill-offset frame-base-slot-count 0)))
      (concat-byte-vectors
        (emit-mov-local-from-rdx (native-value-window-spill-offset frame-base-slot-count 1))
        (concat-byte-vectors
          (emit-mov-local-from-rcx (native-value-window-spill-offset frame-base-slot-count 0))
          (emit-i32-const-x86 value))))
    (if (>= current-depth 8)
    (concat-byte-vectors
      (concat-byte-vectors
        (concat-byte-vectors
          (concat-byte-vectors
            (concat-byte-vectors
              (concat-byte-vectors
                (concat-byte-vectors
                  (concat-byte-vectors
                    (concat-byte-vectors
                      (concat-byte-vectors
                        (concat-byte-vectors
                          (emit-mov-rdx-from-local (native-value-window-spill-offset frame-base-slot-count 5))
                          (emit-mov-local-from-rdx (native-value-window-spill-offset frame-base-slot-count 6)))
                        (emit-mov-rdx-from-local (native-value-window-spill-offset frame-base-slot-count 4)))
                      (emit-mov-local-from-rdx (native-value-window-spill-offset frame-base-slot-count 5)))
                    (emit-mov-rdx-from-local (native-value-window-spill-offset frame-base-slot-count 3)))
                  (emit-mov-local-from-rdx (native-value-window-spill-offset frame-base-slot-count 4)))
                (emit-mov-rdx-from-local (native-value-window-spill-offset frame-base-slot-count 2)))
              (emit-mov-local-from-rdx (native-value-window-spill-offset frame-base-slot-count 3)))
            (emit-mov-rdx-from-local (native-value-window-spill-offset frame-base-slot-count 1)))
          (emit-mov-local-from-rdx (native-value-window-spill-offset frame-base-slot-count 2)))
        (emit-mov-rdx-from-local (native-value-window-spill-offset frame-base-slot-count 0)))
      (concat-byte-vectors
        (emit-mov-local-from-rdx (native-value-window-spill-offset frame-base-slot-count 1))
        (concat-byte-vectors
          (emit-mov-local-from-rcx (native-value-window-spill-offset frame-base-slot-count 0))
          (emit-i32-const-x86 value))))
    (if (>= current-depth 7)
    (concat-byte-vectors
      (concat-byte-vectors
        (concat-byte-vectors
          (concat-byte-vectors
            (concat-byte-vectors
              (concat-byte-vectors
                (concat-byte-vectors
                  (concat-byte-vectors
                    (concat-byte-vectors
                      (concat-byte-vectors
                        (emit-mov-rdx-from-local (native-value-window-spill-offset frame-base-slot-count 4))
                        (emit-mov-local-from-rdx (native-value-window-spill-offset frame-base-slot-count 5)))
                      (emit-mov-rdx-from-local (native-value-window-spill-offset frame-base-slot-count 3)))
                    (emit-mov-local-from-rdx (native-value-window-spill-offset frame-base-slot-count 4)))
                  (emit-mov-rdx-from-local (native-value-window-spill-offset frame-base-slot-count 2)))
                (emit-mov-local-from-rdx (native-value-window-spill-offset frame-base-slot-count 3)))
              (emit-mov-rdx-from-local (native-value-window-spill-offset frame-base-slot-count 1)))
            (emit-mov-local-from-rdx (native-value-window-spill-offset frame-base-slot-count 2)))
          (emit-mov-rdx-from-local (native-value-window-spill-offset frame-base-slot-count 0)))
        (emit-mov-local-from-rdx (native-value-window-spill-offset frame-base-slot-count 1)))
      (concat-byte-vectors
        (emit-mov-local-from-rcx (native-value-window-spill-offset frame-base-slot-count 0))
        (emit-i32-const-x86 value)))
    (if (>= current-depth 6)
    (concat-byte-vectors
      (concat-byte-vectors
        (concat-byte-vectors
          (concat-byte-vectors
            (concat-byte-vectors
              (concat-byte-vectors
                (concat-byte-vectors
                  (concat-byte-vectors
                    (concat-byte-vectors
                      (emit-mov-rdx-from-local (native-value-window-spill-offset frame-base-slot-count 3))
                      (emit-mov-local-from-rdx (native-value-window-spill-offset frame-base-slot-count 4)))
                    (emit-mov-rdx-from-local (native-value-window-spill-offset frame-base-slot-count 2)))
                  (emit-mov-local-from-rdx (native-value-window-spill-offset frame-base-slot-count 3)))
                (emit-mov-rdx-from-local (native-value-window-spill-offset frame-base-slot-count 1)))
              (emit-mov-local-from-rdx (native-value-window-spill-offset frame-base-slot-count 2)))
            (emit-mov-rdx-from-local (native-value-window-spill-offset frame-base-slot-count 0)))
          (emit-mov-local-from-rdx (native-value-window-spill-offset frame-base-slot-count 1)))
        (emit-mov-local-from-rcx (native-value-window-spill-offset frame-base-slot-count 0)))
      (emit-i32-const-x86 value))
    (if (>= current-depth 5)
    (concat-byte-vectors
      (concat-byte-vectors
        (concat-byte-vectors
          (concat-byte-vectors
            (concat-byte-vectors
              (concat-byte-vectors
                (concat-byte-vectors
                  (emit-mov-rdx-from-local (native-value-window-spill-offset frame-base-slot-count 2))
                  (emit-mov-local-from-rdx (native-value-window-spill-offset frame-base-slot-count 3)))
                (emit-mov-rdx-from-local (native-value-window-spill-offset frame-base-slot-count 1)))
              (emit-mov-local-from-rdx (native-value-window-spill-offset frame-base-slot-count 2)))
            (emit-mov-rdx-from-local (native-value-window-spill-offset frame-base-slot-count 0)))
          (emit-mov-local-from-rdx (native-value-window-spill-offset frame-base-slot-count 1)))
        (emit-mov-local-from-rcx (native-value-window-spill-offset frame-base-slot-count 0)))
      (emit-i32-const-x86 value))
    (if (>= current-depth 4)
    (concat-byte-vectors
      (concat-byte-vectors
        (concat-byte-vectors
          (concat-byte-vectors
            (concat-byte-vectors
              (emit-mov-rdx-from-local (native-value-window-spill-offset frame-base-slot-count 1))
              (emit-mov-local-from-rdx (native-value-window-spill-offset frame-base-slot-count 2)))
            (emit-mov-rdx-from-local (native-value-window-spill-offset frame-base-slot-count 0)))
          (emit-mov-local-from-rdx (native-value-window-spill-offset frame-base-slot-count 1)))
        (emit-mov-local-from-rcx (native-value-window-spill-offset frame-base-slot-count 0)))
      (emit-i32-const-x86 value))
    (if (>= current-depth 3)
      (concat-byte-vectors
        (concat-byte-vectors
          (concat-byte-vectors
            (emit-mov-rdx-from-local (native-value-window-spill-offset frame-base-slot-count 0))
            (emit-mov-local-from-rdx (native-value-window-spill-offset frame-base-slot-count 1)))
          (emit-mov-local-from-rcx (native-value-window-spill-offset frame-base-slot-count 0)))
        (emit-i32-const-x86 value))
      (if (>= current-depth 2)
        (concat-byte-vectors
          (emit-mov-local-from-rcx (native-value-window-spill-offset frame-base-slot-count 0))
         (emit-i32-const-x86 value))
          (emit-i32-const-x86 value))))))))))))))))))))))))))))

;; x86_64 bundle の local.get: spill window が必要なら old previous を spill する
(defn emit-local-get-bundle-x86 [offset frame-base-slot-count current-depth]
  (if (>= current-depth 27)
    (concat-byte-vectors
      (concat-byte-vectors
        (emit-mov-rdx-from-local (native-value-window-spill-offset frame-base-slot-count 24))
        (emit-mov-local-from-rdx (native-value-window-spill-offset frame-base-slot-count 25)))
      (emit-local-get-bundle-x86 offset frame-base-slot-count 26))
    (if (>= current-depth 26)
    (concat-byte-vectors
      (concat-byte-vectors
        (emit-mov-rdx-from-local (native-value-window-spill-offset frame-base-slot-count 23))
        (emit-mov-local-from-rdx (native-value-window-spill-offset frame-base-slot-count 24)))
      (emit-local-get-bundle-x86 offset frame-base-slot-count 25))
    (if (>= current-depth 25)
    (concat-byte-vectors
      (concat-byte-vectors
        (emit-mov-rdx-from-local (native-value-window-spill-offset frame-base-slot-count 22))
        (emit-mov-local-from-rdx (native-value-window-spill-offset frame-base-slot-count 23)))
      (emit-local-get-bundle-x86 offset frame-base-slot-count 24))
    (if (>= current-depth 24)
    (concat-byte-vectors
      (concat-byte-vectors
        (emit-mov-rdx-from-local (native-value-window-spill-offset frame-base-slot-count 21))
        (emit-mov-local-from-rdx (native-value-window-spill-offset frame-base-slot-count 22)))
      (emit-local-get-bundle-x86 offset frame-base-slot-count 23))
    (if (>= current-depth 23)
    (concat-byte-vectors
      (concat-byte-vectors
        (emit-mov-rdx-from-local (native-value-window-spill-offset frame-base-slot-count 20))
        (emit-mov-local-from-rdx (native-value-window-spill-offset frame-base-slot-count 21)))
      (emit-local-get-bundle-x86 offset frame-base-slot-count 22))
    (if (>= current-depth 22)
    (concat-byte-vectors
      (concat-byte-vectors
        (emit-mov-rdx-from-local (native-value-window-spill-offset frame-base-slot-count 19))
        (emit-mov-local-from-rdx (native-value-window-spill-offset frame-base-slot-count 20)))
      (emit-local-get-bundle-x86 offset frame-base-slot-count 21))
    (if (>= current-depth 21)
    (concat-byte-vectors
      (concat-byte-vectors
        (emit-mov-rdx-from-local (native-value-window-spill-offset frame-base-slot-count 18))
        (emit-mov-local-from-rdx (native-value-window-spill-offset frame-base-slot-count 19)))
      (emit-local-get-bundle-x86 offset frame-base-slot-count 20))
    (if (>= current-depth 20)
    (concat-byte-vectors
      (concat-byte-vectors
        (emit-mov-rdx-from-local (native-value-window-spill-offset frame-base-slot-count 17))
        (emit-mov-local-from-rdx (native-value-window-spill-offset frame-base-slot-count 18)))
      (emit-local-get-bundle-x86 offset frame-base-slot-count 19))
    (if (>= current-depth 19)
    (concat-byte-vectors
      (concat-byte-vectors
        (emit-mov-rdx-from-local (native-value-window-spill-offset frame-base-slot-count 16))
        (emit-mov-local-from-rdx (native-value-window-spill-offset frame-base-slot-count 17)))
      (emit-local-get-bundle-x86 offset frame-base-slot-count 18))
    (if (>= current-depth 18)
    (concat-byte-vectors
      (concat-byte-vectors
        (emit-mov-rdx-from-local (native-value-window-spill-offset frame-base-slot-count 15))
        (emit-mov-local-from-rdx (native-value-window-spill-offset frame-base-slot-count 16)))
      (emit-local-get-bundle-x86 offset frame-base-slot-count 17))
    (if (>= current-depth 17)
    (concat-byte-vectors
      (concat-byte-vectors
        (emit-mov-rdx-from-local (native-value-window-spill-offset frame-base-slot-count 14))
        (emit-mov-local-from-rdx (native-value-window-spill-offset frame-base-slot-count 15)))
      (emit-local-get-bundle-x86 offset frame-base-slot-count 16))
    (if (>= current-depth 16)
    (concat-byte-vectors
      (concat-byte-vectors
        (emit-mov-rdx-from-local (native-value-window-spill-offset frame-base-slot-count 13))
        (emit-mov-local-from-rdx (native-value-window-spill-offset frame-base-slot-count 14)))
      (emit-local-get-bundle-x86 offset frame-base-slot-count 15))
    (if (>= current-depth 15)
    (concat-byte-vectors
      (concat-byte-vectors
        (emit-mov-rdx-from-local (native-value-window-spill-offset frame-base-slot-count 12))
        (emit-mov-local-from-rdx (native-value-window-spill-offset frame-base-slot-count 13)))
      (emit-local-get-bundle-x86 offset frame-base-slot-count 14))
    (if (>= current-depth 14)
    (concat-byte-vectors
      (concat-byte-vectors
        (emit-mov-rdx-from-local (native-value-window-spill-offset frame-base-slot-count 11))
        (emit-mov-local-from-rdx (native-value-window-spill-offset frame-base-slot-count 12)))
      (emit-local-get-bundle-x86 offset frame-base-slot-count 13))
    (if (>= current-depth 13)
    (concat-byte-vectors
      (concat-byte-vectors
        (emit-mov-rdx-from-local (native-value-window-spill-offset frame-base-slot-count 10))
        (emit-mov-local-from-rdx (native-value-window-spill-offset frame-base-slot-count 11)))
      (emit-local-get-bundle-x86 offset frame-base-slot-count 12))
    (if (>= current-depth 12)
    (concat-byte-vectors
      (concat-byte-vectors
        (emit-mov-rdx-from-local (native-value-window-spill-offset frame-base-slot-count 9))
        (emit-mov-local-from-rdx (native-value-window-spill-offset frame-base-slot-count 10)))
      (emit-local-get-bundle-x86 offset frame-base-slot-count 11))
    (if (>= current-depth 11)
    (concat-byte-vectors
      (concat-byte-vectors
        (emit-mov-rdx-from-local (native-value-window-spill-offset frame-base-slot-count 8))
        (emit-mov-local-from-rdx (native-value-window-spill-offset frame-base-slot-count 9)))
      (if (>= current-depth 10)
        (concat-byte-vectors
          (concat-byte-vectors
            (concat-byte-vectors
              (concat-byte-vectors
                (concat-byte-vectors
                  (concat-byte-vectors
                    (concat-byte-vectors
                      (concat-byte-vectors
                        (concat-byte-vectors
                          (concat-byte-vectors
                            (concat-byte-vectors
                              (concat-byte-vectors
                                (concat-byte-vectors
                                  (concat-byte-vectors
                                    (emit-mov-rdx-from-local (native-value-window-spill-offset frame-base-slot-count 7))
                                    (emit-mov-local-from-rdx (native-value-window-spill-offset frame-base-slot-count 8)))
                                  (emit-mov-rdx-from-local (native-value-window-spill-offset frame-base-slot-count 6)))
                                (emit-mov-local-from-rdx (native-value-window-spill-offset frame-base-slot-count 7)))
                              (emit-mov-rdx-from-local (native-value-window-spill-offset frame-base-slot-count 5)))
                            (emit-mov-local-from-rdx (native-value-window-spill-offset frame-base-slot-count 6)))
                          (emit-mov-rdx-from-local (native-value-window-spill-offset frame-base-slot-count 4)))
                        (emit-mov-local-from-rdx (native-value-window-spill-offset frame-base-slot-count 5)))
                      (emit-mov-rdx-from-local (native-value-window-spill-offset frame-base-slot-count 3)))
                    (emit-mov-local-from-rdx (native-value-window-spill-offset frame-base-slot-count 4)))
                  (emit-mov-rdx-from-local (native-value-window-spill-offset frame-base-slot-count 2)))
                (emit-mov-local-from-rdx (native-value-window-spill-offset frame-base-slot-count 3)))
              (emit-mov-rdx-from-local (native-value-window-spill-offset frame-base-slot-count 1)))
            (emit-mov-local-from-rdx (native-value-window-spill-offset frame-base-slot-count 2)))
          (concat-byte-vectors
            (emit-mov-rdx-from-local (native-value-window-spill-offset frame-base-slot-count 0))
            (concat-byte-vectors
              (emit-mov-local-from-rdx (native-value-window-spill-offset frame-base-slot-count 1))
              (concat-byte-vectors
                (emit-mov-local-from-rcx (native-value-window-spill-offset frame-base-slot-count 0))
                (emit-local-get-x86 offset)))))
        (emit-local-get-x86 offset)))
    (if (>= current-depth 10)
    (concat-byte-vectors
      (concat-byte-vectors
        (concat-byte-vectors
          (concat-byte-vectors
            (concat-byte-vectors
              (concat-byte-vectors
                (concat-byte-vectors
                  (concat-byte-vectors
                    (concat-byte-vectors
                      (concat-byte-vectors
                        (concat-byte-vectors
                          (concat-byte-vectors
                            (concat-byte-vectors
                              (concat-byte-vectors
                                (concat-byte-vectors
                                  (emit-mov-rdx-from-local (native-value-window-spill-offset frame-base-slot-count 7))
                                  (emit-mov-local-from-rdx (native-value-window-spill-offset frame-base-slot-count 8)))
                                (emit-mov-rdx-from-local (native-value-window-spill-offset frame-base-slot-count 6)))
                              (emit-mov-local-from-rdx (native-value-window-spill-offset frame-base-slot-count 7)))
                            (emit-mov-rdx-from-local (native-value-window-spill-offset frame-base-slot-count 5)))
                          (emit-mov-local-from-rdx (native-value-window-spill-offset frame-base-slot-count 6)))
                        (emit-mov-rdx-from-local (native-value-window-spill-offset frame-base-slot-count 4)))
                      (emit-mov-local-from-rdx (native-value-window-spill-offset frame-base-slot-count 5)))
                    (emit-mov-rdx-from-local (native-value-window-spill-offset frame-base-slot-count 3)))
                  (emit-mov-local-from-rdx (native-value-window-spill-offset frame-base-slot-count 4)))
                (emit-mov-rdx-from-local (native-value-window-spill-offset frame-base-slot-count 2)))
              (emit-mov-local-from-rdx (native-value-window-spill-offset frame-base-slot-count 3)))
            (emit-mov-rdx-from-local (native-value-window-spill-offset frame-base-slot-count 1)))
          (emit-mov-local-from-rdx (native-value-window-spill-offset frame-base-slot-count 2)))
        (emit-mov-rdx-from-local (native-value-window-spill-offset frame-base-slot-count 0)))
      (concat-byte-vectors
        (emit-mov-local-from-rdx (native-value-window-spill-offset frame-base-slot-count 1))
        (concat-byte-vectors
          (emit-mov-local-from-rcx (native-value-window-spill-offset frame-base-slot-count 0))
          (emit-local-get-x86 offset))))
    (if (>= current-depth 9)
    (concat-byte-vectors
      (concat-byte-vectors
        (concat-byte-vectors
          (concat-byte-vectors
            (concat-byte-vectors
              (concat-byte-vectors
                (concat-byte-vectors
                  (concat-byte-vectors
                    (concat-byte-vectors
                      (concat-byte-vectors
                        (concat-byte-vectors
                          (concat-byte-vectors
                            (concat-byte-vectors
                              (emit-mov-rdx-from-local (native-value-window-spill-offset frame-base-slot-count 6))
                              (emit-mov-local-from-rdx (native-value-window-spill-offset frame-base-slot-count 7)))
                            (emit-mov-rdx-from-local (native-value-window-spill-offset frame-base-slot-count 5)))
                          (emit-mov-local-from-rdx (native-value-window-spill-offset frame-base-slot-count 6)))
                        (emit-mov-rdx-from-local (native-value-window-spill-offset frame-base-slot-count 4)))
                      (emit-mov-local-from-rdx (native-value-window-spill-offset frame-base-slot-count 5)))
                    (emit-mov-rdx-from-local (native-value-window-spill-offset frame-base-slot-count 3)))
                  (emit-mov-local-from-rdx (native-value-window-spill-offset frame-base-slot-count 4)))
                (emit-mov-rdx-from-local (native-value-window-spill-offset frame-base-slot-count 2)))
              (emit-mov-local-from-rdx (native-value-window-spill-offset frame-base-slot-count 3)))
            (emit-mov-rdx-from-local (native-value-window-spill-offset frame-base-slot-count 1)))
          (emit-mov-local-from-rdx (native-value-window-spill-offset frame-base-slot-count 2)))
        (emit-mov-rdx-from-local (native-value-window-spill-offset frame-base-slot-count 0)))
      (concat-byte-vectors
        (emit-mov-local-from-rdx (native-value-window-spill-offset frame-base-slot-count 1))
        (concat-byte-vectors
          (emit-mov-local-from-rcx (native-value-window-spill-offset frame-base-slot-count 0))
          (emit-local-get-x86 offset))))
    (if (>= current-depth 8)
    (concat-byte-vectors
      (concat-byte-vectors
        (concat-byte-vectors
          (concat-byte-vectors
            (concat-byte-vectors
              (concat-byte-vectors
                (concat-byte-vectors
                  (concat-byte-vectors
                    (concat-byte-vectors
                      (concat-byte-vectors
                        (concat-byte-vectors
                          (emit-mov-rdx-from-local (native-value-window-spill-offset frame-base-slot-count 5))
                          (emit-mov-local-from-rdx (native-value-window-spill-offset frame-base-slot-count 6)))
                        (emit-mov-rdx-from-local (native-value-window-spill-offset frame-base-slot-count 4)))
                      (emit-mov-local-from-rdx (native-value-window-spill-offset frame-base-slot-count 5)))
                    (emit-mov-rdx-from-local (native-value-window-spill-offset frame-base-slot-count 3)))
                  (emit-mov-local-from-rdx (native-value-window-spill-offset frame-base-slot-count 4)))
                (emit-mov-rdx-from-local (native-value-window-spill-offset frame-base-slot-count 2)))
              (emit-mov-local-from-rdx (native-value-window-spill-offset frame-base-slot-count 3)))
            (emit-mov-rdx-from-local (native-value-window-spill-offset frame-base-slot-count 1)))
          (emit-mov-local-from-rdx (native-value-window-spill-offset frame-base-slot-count 2)))
        (emit-mov-rdx-from-local (native-value-window-spill-offset frame-base-slot-count 0)))
      (concat-byte-vectors
        (emit-mov-local-from-rdx (native-value-window-spill-offset frame-base-slot-count 1))
        (concat-byte-vectors
          (emit-mov-local-from-rcx (native-value-window-spill-offset frame-base-slot-count 0))
          (emit-local-get-x86 offset))))
    (if (>= current-depth 7)
    (concat-byte-vectors
      (concat-byte-vectors
        (concat-byte-vectors
          (concat-byte-vectors
            (concat-byte-vectors
              (concat-byte-vectors
                (concat-byte-vectors
                  (concat-byte-vectors
                    (concat-byte-vectors
                      (concat-byte-vectors
                        (emit-mov-rdx-from-local (native-value-window-spill-offset frame-base-slot-count 4))
                        (emit-mov-local-from-rdx (native-value-window-spill-offset frame-base-slot-count 5)))
                      (emit-mov-rdx-from-local (native-value-window-spill-offset frame-base-slot-count 3)))
                    (emit-mov-local-from-rdx (native-value-window-spill-offset frame-base-slot-count 4)))
                  (emit-mov-rdx-from-local (native-value-window-spill-offset frame-base-slot-count 2)))
                (emit-mov-local-from-rdx (native-value-window-spill-offset frame-base-slot-count 3)))
              (emit-mov-rdx-from-local (native-value-window-spill-offset frame-base-slot-count 1)))
            (emit-mov-local-from-rdx (native-value-window-spill-offset frame-base-slot-count 2)))
          (emit-mov-rdx-from-local (native-value-window-spill-offset frame-base-slot-count 0)))
        (emit-mov-local-from-rdx (native-value-window-spill-offset frame-base-slot-count 1)))
      (concat-byte-vectors
        (emit-mov-local-from-rcx (native-value-window-spill-offset frame-base-slot-count 0))
        (emit-local-get-x86 offset)))
    (if (>= current-depth 6)
    (concat-byte-vectors
      (concat-byte-vectors
        (concat-byte-vectors
          (concat-byte-vectors
            (concat-byte-vectors
              (concat-byte-vectors
                (concat-byte-vectors
                  (concat-byte-vectors
                    (concat-byte-vectors
                      (emit-mov-rdx-from-local (native-value-window-spill-offset frame-base-slot-count 3))
                      (emit-mov-local-from-rdx (native-value-window-spill-offset frame-base-slot-count 4)))
                    (emit-mov-rdx-from-local (native-value-window-spill-offset frame-base-slot-count 2)))
                  (emit-mov-local-from-rdx (native-value-window-spill-offset frame-base-slot-count 3)))
                (emit-mov-rdx-from-local (native-value-window-spill-offset frame-base-slot-count 1)))
              (emit-mov-local-from-rdx (native-value-window-spill-offset frame-base-slot-count 2)))
            (emit-mov-rdx-from-local (native-value-window-spill-offset frame-base-slot-count 0)))
          (emit-mov-local-from-rdx (native-value-window-spill-offset frame-base-slot-count 1)))
        (emit-mov-local-from-rcx (native-value-window-spill-offset frame-base-slot-count 0)))
      (emit-local-get-x86 offset))
    (if (>= current-depth 5)
    (concat-byte-vectors
      (concat-byte-vectors
        (concat-byte-vectors
          (concat-byte-vectors
            (concat-byte-vectors
              (concat-byte-vectors
                (concat-byte-vectors
                  (emit-mov-rdx-from-local (native-value-window-spill-offset frame-base-slot-count 2))
                  (emit-mov-local-from-rdx (native-value-window-spill-offset frame-base-slot-count 3)))
                (emit-mov-rdx-from-local (native-value-window-spill-offset frame-base-slot-count 1)))
              (emit-mov-local-from-rdx (native-value-window-spill-offset frame-base-slot-count 2)))
            (emit-mov-rdx-from-local (native-value-window-spill-offset frame-base-slot-count 0)))
          (emit-mov-local-from-rdx (native-value-window-spill-offset frame-base-slot-count 1)))
        (emit-mov-local-from-rcx (native-value-window-spill-offset frame-base-slot-count 0)))
      (emit-local-get-x86 offset))
    (if (>= current-depth 4)
    (concat-byte-vectors
      (concat-byte-vectors
        (concat-byte-vectors
          (concat-byte-vectors
            (concat-byte-vectors
              (emit-mov-rdx-from-local (native-value-window-spill-offset frame-base-slot-count 1))
              (emit-mov-local-from-rdx (native-value-window-spill-offset frame-base-slot-count 2)))
            (emit-mov-rdx-from-local (native-value-window-spill-offset frame-base-slot-count 0)))
          (emit-mov-local-from-rdx (native-value-window-spill-offset frame-base-slot-count 1)))
        (emit-mov-local-from-rcx (native-value-window-spill-offset frame-base-slot-count 0)))
      (emit-local-get-x86 offset))
    (if (>= current-depth 3)
      (concat-byte-vectors
        (concat-byte-vectors
          (concat-byte-vectors
            (emit-mov-rdx-from-local (native-value-window-spill-offset frame-base-slot-count 0))
            (emit-mov-local-from-rdx (native-value-window-spill-offset frame-base-slot-count 1)))
          (emit-mov-local-from-rcx (native-value-window-spill-offset frame-base-slot-count 0)))
        (emit-local-get-x86 offset))
      (if (>= current-depth 2)
        (concat-byte-vectors
          (emit-mov-local-from-rcx (native-value-window-spill-offset frame-base-slot-count 0))
          (emit-local-get-x86 offset))
         (emit-local-get-x86 offset))))))))))))))))))))))))))))

(defn emit-twenty-six-arg-call-x86 [rel frame-base-slot-count]
  (let [stack0 (concat-byte-vectors
                 (emit-sub-rsp-imm32 160)
                 (emit-mov-twentieth-stack-from-rax))
        stack1 (concat-byte-vectors stack0 (emit-mov-nineteenth-stack-from-rcx))
        stack2 (concat-byte-vectors
                 stack1
                 (emit-mov-rcx-from-local (native-value-window-spill-offset frame-base-slot-count 0)))
        stack3 (concat-byte-vectors stack2 (emit-mov-eighteenth-stack-from-rcx))
        stack4 (concat-byte-vectors
                 stack3
                 (emit-mov-rcx-from-local (native-value-window-spill-offset frame-base-slot-count 1)))
        stack5 (concat-byte-vectors stack4 (emit-mov-seventeenth-stack-from-rcx))
        stack6 (concat-byte-vectors
                 stack5
                 (emit-mov-rcx-from-local (native-value-window-spill-offset frame-base-slot-count 2)))
        stack7 (concat-byte-vectors stack6 (emit-mov-sixteenth-stack-from-rcx))
        stack8 (concat-byte-vectors
                 stack7
                 (emit-mov-rcx-from-local (native-value-window-spill-offset frame-base-slot-count 3)))
        stack9 (concat-byte-vectors stack8 (emit-mov-fifteenth-stack-from-rcx))
        stack10 (concat-byte-vectors
                  stack9
                  (emit-mov-rcx-from-local (native-value-window-spill-offset frame-base-slot-count 4)))
        stack11 (concat-byte-vectors stack10 (emit-mov-fourteenth-stack-from-rcx))
        stack12 (concat-byte-vectors
                  stack11
                  (emit-mov-rcx-from-local (native-value-window-spill-offset frame-base-slot-count 5)))
        stack13 (concat-byte-vectors stack12 (emit-mov-thirteenth-stack-from-rcx))
        stack14 (concat-byte-vectors
                  stack13
                  (emit-mov-rcx-from-local (native-value-window-spill-offset frame-base-slot-count 6)))
        stack15 (concat-byte-vectors stack14 (emit-mov-twelfth-stack-from-rcx))
        stack16 (concat-byte-vectors
                  stack15
                  (emit-mov-rcx-from-local (native-value-window-spill-offset frame-base-slot-count 7)))
        stack17 (concat-byte-vectors stack16 (emit-mov-eleventh-stack-from-rcx))
        stack18 (concat-byte-vectors
                  stack17
                  (emit-mov-rcx-from-local (native-value-window-spill-offset frame-base-slot-count 8)))
        stack19 (concat-byte-vectors stack18 (emit-mov-tenth-stack-from-rcx))
        stack20 (concat-byte-vectors
                  stack19
                  (emit-mov-rcx-from-local (native-value-window-spill-offset frame-base-slot-count 9)))
        stack21 (concat-byte-vectors stack20 (emit-mov-ninth-stack-from-rcx))
        stack22 (concat-byte-vectors
                  stack21
                  (emit-mov-rcx-from-local (native-value-window-spill-offset frame-base-slot-count 10)))
        stack23 (concat-byte-vectors stack22 (emit-mov-eighth-stack-from-rcx))
        stack24 (concat-byte-vectors
                  stack23
                  (emit-mov-rcx-from-local (native-value-window-spill-offset frame-base-slot-count 11)))
        stack25 (concat-byte-vectors stack24 (emit-mov-seventh-stack-from-rcx))
        stack26 (concat-byte-vectors
                  stack25
                  (emit-mov-rcx-from-local (native-value-window-spill-offset frame-base-slot-count 12)))
        stack27 (concat-byte-vectors stack26 (emit-mov-sixth-stack-from-rcx))
        stack28 (concat-byte-vectors
                  stack27
                  (emit-mov-rcx-from-local (native-value-window-spill-offset frame-base-slot-count 13)))
        stack29 (concat-byte-vectors stack28 (emit-mov-fifth-stack-from-rcx))
        stack30 (concat-byte-vectors
                  stack29
                  (emit-mov-rcx-from-local (native-value-window-spill-offset frame-base-slot-count 14)))
        stack31 (concat-byte-vectors stack30 (emit-mov-fourth-stack-from-rcx))
        stack32 (concat-byte-vectors
                  stack31
                  (emit-mov-rcx-from-local (native-value-window-spill-offset frame-base-slot-count 15)))
        stack33 (concat-byte-vectors stack32 (emit-mov-third-stack-from-rcx))
        stack34 (concat-byte-vectors
                  stack33
                  (emit-mov-rcx-from-local (native-value-window-spill-offset frame-base-slot-count 16)))
        stack35 (concat-byte-vectors stack34 (emit-mov-second-stack-from-rcx))
        stack-setup (concat-byte-vectors
                      stack35
                      (emit-mov-r9-from-local (native-value-window-spill-offset frame-base-slot-count 17)))
        reg0 (concat-byte-vectors
               (emit-mov-top-stack-from-r9)
               (emit-mov-r9-from-local (native-value-window-spill-offset frame-base-slot-count 18)))
        reg1 (concat-byte-vectors reg0 (emit-mov-r8-from-local (native-value-window-spill-offset frame-base-slot-count 19)))
        reg2 (concat-byte-vectors reg1 (emit-mov-rcx-from-local (native-value-window-spill-offset frame-base-slot-count 20)))
        reg3 (concat-byte-vectors reg2 (emit-mov-rdx-from-local (native-value-window-spill-offset frame-base-slot-count 21)))
        reg4 (concat-byte-vectors reg3 (emit-mov-rsi-from-local (native-value-window-spill-offset frame-base-slot-count 22)))
        reg-setup (concat-byte-vectors reg4 (emit-mov-rdi-from-local (native-value-window-spill-offset frame-base-slot-count 23)))
        call-seq (concat-byte-vectors
                   (emit-call-rel32 rel)
                   (emit-add-rsp-imm32 160))]
    (concat-byte-vectors
      (concat-byte-vectors stack-setup reg-setup)
      call-seq)))

(defn emit-twenty-seven-arg-call-x86 [rel frame-base-slot-count]
  (let [stack0 (concat-byte-vectors
                 (emit-sub-rsp-imm32 176)
                 (emit-mov-twenty-first-stack-from-rax))
        stack1 (concat-byte-vectors stack0 (emit-mov-twentieth-stack-from-rcx))
        stack2 (concat-byte-vectors
                 stack1
                 (emit-mov-rcx-from-local (native-value-window-spill-offset frame-base-slot-count 0)))
        stack3 (concat-byte-vectors stack2 (emit-mov-nineteenth-stack-from-rcx))
        stack4 (concat-byte-vectors
                 stack3
                 (emit-mov-rcx-from-local (native-value-window-spill-offset frame-base-slot-count 1)))
        stack5 (concat-byte-vectors stack4 (emit-mov-eighteenth-stack-from-rcx))
        stack6 (concat-byte-vectors
                 stack5
                 (emit-mov-rcx-from-local (native-value-window-spill-offset frame-base-slot-count 2)))
        stack7 (concat-byte-vectors stack6 (emit-mov-seventeenth-stack-from-rcx))
        stack8 (concat-byte-vectors
                 stack7
                 (emit-mov-rcx-from-local (native-value-window-spill-offset frame-base-slot-count 3)))
        stack9 (concat-byte-vectors stack8 (emit-mov-sixteenth-stack-from-rcx))
        stack10 (concat-byte-vectors
                  stack9
                  (emit-mov-rcx-from-local (native-value-window-spill-offset frame-base-slot-count 4)))
        stack11 (concat-byte-vectors stack10 (emit-mov-fifteenth-stack-from-rcx))
        stack12 (concat-byte-vectors
                  stack11
                  (emit-mov-rcx-from-local (native-value-window-spill-offset frame-base-slot-count 5)))
        stack13 (concat-byte-vectors stack12 (emit-mov-fourteenth-stack-from-rcx))
        stack14 (concat-byte-vectors
                  stack13
                  (emit-mov-rcx-from-local (native-value-window-spill-offset frame-base-slot-count 6)))
        stack15 (concat-byte-vectors stack14 (emit-mov-thirteenth-stack-from-rcx))
        stack16 (concat-byte-vectors
                  stack15
                  (emit-mov-rcx-from-local (native-value-window-spill-offset frame-base-slot-count 7)))
        stack17 (concat-byte-vectors stack16 (emit-mov-twelfth-stack-from-rcx))
        stack18 (concat-byte-vectors
                  stack17
                  (emit-mov-rcx-from-local (native-value-window-spill-offset frame-base-slot-count 8)))
        stack19 (concat-byte-vectors stack18 (emit-mov-eleventh-stack-from-rcx))
        stack20 (concat-byte-vectors
                  stack19
                  (emit-mov-rcx-from-local (native-value-window-spill-offset frame-base-slot-count 9)))
        stack21 (concat-byte-vectors stack20 (emit-mov-tenth-stack-from-rcx))
        stack22 (concat-byte-vectors
                  stack21
                  (emit-mov-rcx-from-local (native-value-window-spill-offset frame-base-slot-count 10)))
        stack23 (concat-byte-vectors stack22 (emit-mov-ninth-stack-from-rcx))
        stack24 (concat-byte-vectors
                  stack23
                  (emit-mov-rcx-from-local (native-value-window-spill-offset frame-base-slot-count 11)))
        stack25 (concat-byte-vectors stack24 (emit-mov-eighth-stack-from-rcx))
        stack26 (concat-byte-vectors
                  stack25
                  (emit-mov-rcx-from-local (native-value-window-spill-offset frame-base-slot-count 12)))
        stack27 (concat-byte-vectors stack26 (emit-mov-seventh-stack-from-rcx))
        stack28 (concat-byte-vectors
                  stack27
                  (emit-mov-rcx-from-local (native-value-window-spill-offset frame-base-slot-count 13)))
        stack29 (concat-byte-vectors stack28 (emit-mov-sixth-stack-from-rcx))
        stack30 (concat-byte-vectors
                  stack29
                  (emit-mov-rcx-from-local (native-value-window-spill-offset frame-base-slot-count 14)))
        stack31 (concat-byte-vectors stack30 (emit-mov-fifth-stack-from-rcx))
        stack32 (concat-byte-vectors
                  stack31
                  (emit-mov-rcx-from-local (native-value-window-spill-offset frame-base-slot-count 15)))
        stack33 (concat-byte-vectors stack32 (emit-mov-fourth-stack-from-rcx))
        stack34 (concat-byte-vectors
                  stack33
                  (emit-mov-rcx-from-local (native-value-window-spill-offset frame-base-slot-count 16)))
        stack35 (concat-byte-vectors stack34 (emit-mov-third-stack-from-rcx))
        stack36 (concat-byte-vectors
                  stack35
                  (emit-mov-rcx-from-local (native-value-window-spill-offset frame-base-slot-count 17)))
        stack37 (concat-byte-vectors stack36 (emit-mov-second-stack-from-rcx))
        stack-setup (concat-byte-vectors
                      stack37
                      (emit-mov-r9-from-local (native-value-window-spill-offset frame-base-slot-count 18)))
        reg0 (concat-byte-vectors
               (emit-mov-top-stack-from-r9)
               (emit-mov-r9-from-local (native-value-window-spill-offset frame-base-slot-count 19)))
        reg1 (concat-byte-vectors reg0 (emit-mov-r8-from-local (native-value-window-spill-offset frame-base-slot-count 20)))
        reg2 (concat-byte-vectors reg1 (emit-mov-rcx-from-local (native-value-window-spill-offset frame-base-slot-count 21)))
        reg3 (concat-byte-vectors reg2 (emit-mov-rdx-from-local (native-value-window-spill-offset frame-base-slot-count 22)))
        reg4 (concat-byte-vectors reg3 (emit-mov-rsi-from-local (native-value-window-spill-offset frame-base-slot-count 23)))
        reg-setup (concat-byte-vectors reg4 (emit-mov-rdi-from-local (native-value-window-spill-offset frame-base-slot-count 24)))
        call-seq (concat-byte-vectors
                   (emit-call-rel32 rel)
                   (emit-add-rsp-imm32 176))]
    (concat-byte-vectors
      (concat-byte-vectors stack-setup reg-setup)
      call-seq)))

(defn emit-twenty-eight-arg-call-x86 [rel frame-base-slot-count]
  (let [stack0 (concat-byte-vectors
                 (emit-sub-rsp-imm32 176)
                 (emit-mov-twenty-second-stack-from-rax))
        stack1 (concat-byte-vectors stack0 (emit-mov-twenty-first-stack-from-rcx))
        stack2 (concat-byte-vectors
                 stack1
                 (emit-mov-rcx-from-local (native-value-window-spill-offset frame-base-slot-count 0)))
        stack3 (concat-byte-vectors stack2 (emit-mov-twentieth-stack-from-rcx))
        stack4 (concat-byte-vectors
                 stack3
                 (emit-mov-rcx-from-local (native-value-window-spill-offset frame-base-slot-count 1)))
        stack5 (concat-byte-vectors stack4 (emit-mov-nineteenth-stack-from-rcx))
        stack6 (concat-byte-vectors
                 stack5
                 (emit-mov-rcx-from-local (native-value-window-spill-offset frame-base-slot-count 2)))
        stack7 (concat-byte-vectors stack6 (emit-mov-eighteenth-stack-from-rcx))
        stack8 (concat-byte-vectors
                 stack7
                 (emit-mov-rcx-from-local (native-value-window-spill-offset frame-base-slot-count 3)))
        stack9 (concat-byte-vectors stack8 (emit-mov-seventeenth-stack-from-rcx))
        stack10 (concat-byte-vectors
                  stack9
                  (emit-mov-rcx-from-local (native-value-window-spill-offset frame-base-slot-count 4)))
        stack11 (concat-byte-vectors stack10 (emit-mov-sixteenth-stack-from-rcx))
        stack12 (concat-byte-vectors
                  stack11
                  (emit-mov-rcx-from-local (native-value-window-spill-offset frame-base-slot-count 5)))
        stack13 (concat-byte-vectors stack12 (emit-mov-fifteenth-stack-from-rcx))
        stack14 (concat-byte-vectors
                  stack13
                  (emit-mov-rcx-from-local (native-value-window-spill-offset frame-base-slot-count 6)))
        stack15 (concat-byte-vectors stack14 (emit-mov-fourteenth-stack-from-rcx))
        stack16 (concat-byte-vectors
                  stack15
                  (emit-mov-rcx-from-local (native-value-window-spill-offset frame-base-slot-count 7)))
        stack17 (concat-byte-vectors stack16 (emit-mov-thirteenth-stack-from-rcx))
        stack18 (concat-byte-vectors
                  stack17
                  (emit-mov-rcx-from-local (native-value-window-spill-offset frame-base-slot-count 8)))
        stack19 (concat-byte-vectors stack18 (emit-mov-twelfth-stack-from-rcx))
        stack20 (concat-byte-vectors
                  stack19
                  (emit-mov-rcx-from-local (native-value-window-spill-offset frame-base-slot-count 9)))
        stack21 (concat-byte-vectors stack20 (emit-mov-eleventh-stack-from-rcx))
        stack22 (concat-byte-vectors
                  stack21
                  (emit-mov-rcx-from-local (native-value-window-spill-offset frame-base-slot-count 10)))
        stack23 (concat-byte-vectors stack22 (emit-mov-tenth-stack-from-rcx))
        stack24 (concat-byte-vectors
                  stack23
                  (emit-mov-rcx-from-local (native-value-window-spill-offset frame-base-slot-count 11)))
        stack25 (concat-byte-vectors stack24 (emit-mov-ninth-stack-from-rcx))
        stack26 (concat-byte-vectors
                  stack25
                  (emit-mov-rcx-from-local (native-value-window-spill-offset frame-base-slot-count 12)))
        stack27 (concat-byte-vectors stack26 (emit-mov-eighth-stack-from-rcx))
        stack28 (concat-byte-vectors
                  stack27
                  (emit-mov-rcx-from-local (native-value-window-spill-offset frame-base-slot-count 13)))
        stack29 (concat-byte-vectors stack28 (emit-mov-seventh-stack-from-rcx))
        stack30 (concat-byte-vectors
                  stack29
                  (emit-mov-rcx-from-local (native-value-window-spill-offset frame-base-slot-count 14)))
        stack31 (concat-byte-vectors stack30 (emit-mov-sixth-stack-from-rcx))
        stack32 (concat-byte-vectors
                  stack31
                  (emit-mov-rcx-from-local (native-value-window-spill-offset frame-base-slot-count 15)))
        stack33 (concat-byte-vectors stack32 (emit-mov-fifth-stack-from-rcx))
        stack34 (concat-byte-vectors
                  stack33
                  (emit-mov-rcx-from-local (native-value-window-spill-offset frame-base-slot-count 16)))
        stack35 (concat-byte-vectors stack34 (emit-mov-fourth-stack-from-rcx))
        stack36 (concat-byte-vectors
                  stack35
                  (emit-mov-rcx-from-local (native-value-window-spill-offset frame-base-slot-count 17)))
        stack37 (concat-byte-vectors stack36 (emit-mov-third-stack-from-rcx))
        stack38 (concat-byte-vectors
                  stack37
                  (emit-mov-rcx-from-local (native-value-window-spill-offset frame-base-slot-count 18)))
        stack39 (concat-byte-vectors stack38 (emit-mov-second-stack-from-rcx))
        stack-setup (concat-byte-vectors
                      stack39
                      (emit-mov-r9-from-local (native-value-window-spill-offset frame-base-slot-count 19)))
        reg0 (concat-byte-vectors
               (emit-mov-top-stack-from-r9)
               (emit-mov-r9-from-local (native-value-window-spill-offset frame-base-slot-count 20)))
        reg1 (concat-byte-vectors reg0 (emit-mov-r8-from-local (native-value-window-spill-offset frame-base-slot-count 21)))
        reg2 (concat-byte-vectors reg1 (emit-mov-rcx-from-local (native-value-window-spill-offset frame-base-slot-count 22)))
        reg3 (concat-byte-vectors reg2 (emit-mov-rdx-from-local (native-value-window-spill-offset frame-base-slot-count 23)))
        reg4 (concat-byte-vectors reg3 (emit-mov-rsi-from-local (native-value-window-spill-offset frame-base-slot-count 24)))
        reg-setup (concat-byte-vectors reg4 (emit-mov-rdi-from-local (native-value-window-spill-offset frame-base-slot-count 25)))
        call-seq (concat-byte-vectors
                   (emit-call-rel32 rel)
                   (emit-add-rsp-imm32 176))]
    (concat-byte-vectors
      (concat-byte-vectors stack-setup reg-setup)
      call-seq)))

(defn emit-three-arg-call-x86 [rel frame-base-slot-count]
  (concat-byte-vectors
    (concat-byte-vectors
      (concat-byte-vectors
        (emit-mov-rdx-rax)
        (emit-mov-rsi-rcx))
      (emit-mov-rdi-from-local (native-value-window-spill-offset frame-base-slot-count 0)))
    (emit-call-rel32 rel)))

(defn emit-four-arg-call-x86 [rel frame-base-slot-count]
  (concat-byte-vectors
    (concat-byte-vectors
      (concat-byte-vectors
        (concat-byte-vectors
          (emit-mov-rdx-rcx)
          (emit-mov-rsi-from-local (native-value-window-spill-offset frame-base-slot-count 0)))
        (emit-mov-rdi-from-local (native-value-window-spill-offset frame-base-slot-count 1)))
      (emit-mov-rcx-rax))
    (emit-call-rel32 rel)))

(defn emit-five-arg-call-x86 [rel frame-base-slot-count]
  (concat-byte-vectors
    (concat-byte-vectors
      (concat-byte-vectors
        (concat-byte-vectors
          (emit-mov-r8-rax)
          (emit-mov-rdx-from-local (native-value-window-spill-offset frame-base-slot-count 0)))
        (emit-mov-rsi-from-local (native-value-window-spill-offset frame-base-slot-count 1)))
      (emit-mov-rdi-from-local (native-value-window-spill-offset frame-base-slot-count 2)))
    (emit-call-rel32 rel)))

(defn emit-six-arg-call-x86 [rel frame-base-slot-count]
  (concat-byte-vectors
    (concat-byte-vectors
      (concat-byte-vectors
        (concat-byte-vectors
          (concat-byte-vectors
            (concat-byte-vectors
              (emit-mov-r9-rax)
              (emit-mov-r8-rcx))
            (emit-mov-rcx-from-local (native-value-window-spill-offset frame-base-slot-count 0)))
          (emit-mov-rdx-from-local (native-value-window-spill-offset frame-base-slot-count 1)))
        (emit-mov-rsi-from-local (native-value-window-spill-offset frame-base-slot-count 2)))
      (emit-mov-rdi-from-local (native-value-window-spill-offset frame-base-slot-count 3)))
    (emit-call-rel32 rel)))

(defn emit-seven-arg-call-x86 [rel frame-base-slot-count]
  (concat-byte-vectors
    (concat-byte-vectors
      (concat-byte-vectors
        (concat-byte-vectors
          (concat-byte-vectors
            (concat-byte-vectors
              (concat-byte-vectors
                (concat-byte-vectors
                  (concat-byte-vectors
                    (emit-sub-rsp-imm32 16)
                    (emit-mov-top-stack-from-rax))
                  (emit-mov-r9-rcx))
                (emit-mov-r8-from-local (native-value-window-spill-offset frame-base-slot-count 0)))
              (emit-mov-rcx-from-local (native-value-window-spill-offset frame-base-slot-count 1)))
            (emit-mov-rdx-from-local (native-value-window-spill-offset frame-base-slot-count 2)))
          (emit-mov-rsi-from-local (native-value-window-spill-offset frame-base-slot-count 3)))
        (emit-mov-rdi-from-local (native-value-window-spill-offset frame-base-slot-count 4)))
      (emit-call-rel32 rel))
    (emit-add-rsp-imm32 16)))

(defn emit-eight-arg-call-x86 [rel frame-base-slot-count]
  (concat-byte-vectors
    (concat-byte-vectors
      (concat-byte-vectors
        (concat-byte-vectors
          (concat-byte-vectors
            (concat-byte-vectors
              (concat-byte-vectors
                (concat-byte-vectors
                  (concat-byte-vectors
                    (concat-byte-vectors
                      (emit-sub-rsp-imm32 16)
                      (emit-mov-second-stack-from-rax))
                    (emit-mov-top-stack-from-rcx))
                  (emit-mov-r9-from-local (native-value-window-spill-offset frame-base-slot-count 0)))
                (emit-mov-r8-from-local (native-value-window-spill-offset frame-base-slot-count 1)))
              (emit-mov-rcx-from-local (native-value-window-spill-offset frame-base-slot-count 2)))
            (emit-mov-rdx-from-local (native-value-window-spill-offset frame-base-slot-count 3)))
          (emit-mov-rsi-from-local (native-value-window-spill-offset frame-base-slot-count 4)))
        (emit-mov-rdi-from-local (native-value-window-spill-offset frame-base-slot-count 5)))
      (emit-call-rel32 rel))
    (emit-add-rsp-imm32 16)))

(defn emit-nine-arg-call-x86 [rel frame-base-slot-count]
  (concat-byte-vectors
    (concat-byte-vectors
      (concat-byte-vectors
        (concat-byte-vectors
          (concat-byte-vectors
            (concat-byte-vectors
              (concat-byte-vectors
                (concat-byte-vectors
                  (concat-byte-vectors
                    (concat-byte-vectors
                      (concat-byte-vectors
                        (concat-byte-vectors
                          (emit-sub-rsp-imm32 32)
                          (emit-mov-third-stack-from-rax))
                        (emit-mov-second-stack-from-rcx))
                      (emit-mov-r9-from-local (native-value-window-spill-offset frame-base-slot-count 0)))
                    (emit-mov-top-stack-from-r9))
                  (emit-mov-r9-from-local (native-value-window-spill-offset frame-base-slot-count 1)))
                (emit-mov-r8-from-local (native-value-window-spill-offset frame-base-slot-count 2)))
              (emit-mov-rcx-from-local (native-value-window-spill-offset frame-base-slot-count 3)))
            (emit-mov-rdx-from-local (native-value-window-spill-offset frame-base-slot-count 4)))
          (emit-mov-rsi-from-local (native-value-window-spill-offset frame-base-slot-count 5)))
        (emit-mov-rdi-from-local (native-value-window-spill-offset frame-base-slot-count 6)))
       (emit-call-rel32 rel))
     (emit-add-rsp-imm32 32)))

(defn emit-ten-arg-call-x86 [rel frame-base-slot-count]
  (let [stack0 (concat-byte-vectors
                 (emit-sub-rsp-imm32 32)
                 (emit-mov-fourth-stack-from-rax))
        stack1 (concat-byte-vectors stack0 (emit-mov-third-stack-from-rcx))
        stack2 (concat-byte-vectors
                 stack1
                 (emit-mov-rcx-from-local (native-value-window-spill-offset frame-base-slot-count 0)))
        stack3 (concat-byte-vectors stack2 (emit-mov-second-stack-from-rcx))
        stack-setup (concat-byte-vectors
                      stack3
                      (emit-mov-r9-from-local (native-value-window-spill-offset frame-base-slot-count 1)))
        reg0 (concat-byte-vectors
               (emit-mov-top-stack-from-r9)
               (emit-mov-r9-from-local (native-value-window-spill-offset frame-base-slot-count 2)))
        reg1 (concat-byte-vectors reg0 (emit-mov-r8-from-local (native-value-window-spill-offset frame-base-slot-count 3)))
        reg2 (concat-byte-vectors reg1 (emit-mov-rcx-from-local (native-value-window-spill-offset frame-base-slot-count 4)))
        reg3 (concat-byte-vectors reg2 (emit-mov-rdx-from-local (native-value-window-spill-offset frame-base-slot-count 5)))
        reg4 (concat-byte-vectors reg3 (emit-mov-rsi-from-local (native-value-window-spill-offset frame-base-slot-count 6)))
        reg-setup (concat-byte-vectors reg4 (emit-mov-rdi-from-local (native-value-window-spill-offset frame-base-slot-count 7)))
        call-seq (concat-byte-vectors
                   (emit-call-rel32 rel)
                   (emit-add-rsp-imm32 32))]
    (concat-byte-vectors
      (concat-byte-vectors stack-setup reg-setup)
      call-seq)))

(defn emit-eleven-arg-call-x86 [rel frame-base-slot-count]
  (let [stack0 (concat-byte-vectors
                 (emit-sub-rsp-imm32 48)
                 (emit-mov-fifth-stack-from-rax))
        stack1 (concat-byte-vectors stack0 (emit-mov-fourth-stack-from-rcx))
        stack2 (concat-byte-vectors
                 stack1
                 (emit-mov-rcx-from-local (native-value-window-spill-offset frame-base-slot-count 0)))
        stack3 (concat-byte-vectors stack2 (emit-mov-third-stack-from-rcx))
        stack4 (concat-byte-vectors
                 stack3
                 (emit-mov-rcx-from-local (native-value-window-spill-offset frame-base-slot-count 1)))
        stack5 (concat-byte-vectors stack4 (emit-mov-second-stack-from-rcx))
        stack-setup (concat-byte-vectors
                      stack5
                      (emit-mov-r9-from-local (native-value-window-spill-offset frame-base-slot-count 2)))
        reg0 (concat-byte-vectors
               (emit-mov-top-stack-from-r9)
               (emit-mov-r9-from-local (native-value-window-spill-offset frame-base-slot-count 3)))
        reg1 (concat-byte-vectors reg0 (emit-mov-r8-from-local (native-value-window-spill-offset frame-base-slot-count 4)))
        reg2 (concat-byte-vectors reg1 (emit-mov-rcx-from-local (native-value-window-spill-offset frame-base-slot-count 5)))
        reg3 (concat-byte-vectors reg2 (emit-mov-rdx-from-local (native-value-window-spill-offset frame-base-slot-count 6)))
        reg4 (concat-byte-vectors reg3 (emit-mov-rsi-from-local (native-value-window-spill-offset frame-base-slot-count 7)))
        reg-setup (concat-byte-vectors reg4 (emit-mov-rdi-from-local (native-value-window-spill-offset frame-base-slot-count 8)))
        call-seq (concat-byte-vectors
                   (emit-call-rel32 rel)
                   (emit-add-rsp-imm32 48))]
    (concat-byte-vectors
      (concat-byte-vectors stack-setup reg-setup)
      call-seq)))

(defn emit-twelve-arg-call-x86 [rel frame-base-slot-count]
  (let [stack0 (concat-byte-vectors
                 (emit-sub-rsp-imm32 48)
                 (emit-mov-sixth-stack-from-rax))
        stack1 (concat-byte-vectors stack0 (emit-mov-fifth-stack-from-rcx))
        stack2 (concat-byte-vectors
                 stack1
                 (emit-mov-rcx-from-local (native-value-window-spill-offset frame-base-slot-count 0)))
        stack3 (concat-byte-vectors stack2 (emit-mov-fourth-stack-from-rcx))
        stack4 (concat-byte-vectors
                 stack3
                 (emit-mov-rcx-from-local (native-value-window-spill-offset frame-base-slot-count 1)))
        stack5 (concat-byte-vectors stack4 (emit-mov-third-stack-from-rcx))
        stack6 (concat-byte-vectors
                 stack5
                 (emit-mov-rcx-from-local (native-value-window-spill-offset frame-base-slot-count 2)))
        stack7 (concat-byte-vectors stack6 (emit-mov-second-stack-from-rcx))
        stack-setup (concat-byte-vectors
                      stack7
                      (emit-mov-r9-from-local (native-value-window-spill-offset frame-base-slot-count 3)))
        reg0 (concat-byte-vectors
               (emit-mov-top-stack-from-r9)
               (emit-mov-r9-from-local (native-value-window-spill-offset frame-base-slot-count 4)))
        reg1 (concat-byte-vectors reg0 (emit-mov-r8-from-local (native-value-window-spill-offset frame-base-slot-count 5)))
        reg2 (concat-byte-vectors reg1 (emit-mov-rcx-from-local (native-value-window-spill-offset frame-base-slot-count 6)))
        reg3 (concat-byte-vectors reg2 (emit-mov-rdx-from-local (native-value-window-spill-offset frame-base-slot-count 7)))
        reg4 (concat-byte-vectors reg3 (emit-mov-rsi-from-local (native-value-window-spill-offset frame-base-slot-count 8)))
        reg-setup (concat-byte-vectors reg4 (emit-mov-rdi-from-local (native-value-window-spill-offset frame-base-slot-count 9)))
        call-seq (concat-byte-vectors
                   (emit-call-rel32 rel)
                   (emit-add-rsp-imm32 48))]
    (concat-byte-vectors
      (concat-byte-vectors stack-setup reg-setup)
      call-seq)))

(defn emit-thirteen-arg-call-x86 [rel frame-base-slot-count]
  (let [stack0 (concat-byte-vectors
                 (emit-sub-rsp-imm32 64)
                 (emit-mov-seventh-stack-from-rax))
        stack1 (concat-byte-vectors stack0 (emit-mov-sixth-stack-from-rcx))
        stack2 (concat-byte-vectors
                 stack1
                 (emit-mov-rcx-from-local (native-value-window-spill-offset frame-base-slot-count 0)))
        stack3 (concat-byte-vectors stack2 (emit-mov-fifth-stack-from-rcx))
        stack4 (concat-byte-vectors
                 stack3
                 (emit-mov-rcx-from-local (native-value-window-spill-offset frame-base-slot-count 1)))
        stack5 (concat-byte-vectors stack4 (emit-mov-fourth-stack-from-rcx))
        stack6 (concat-byte-vectors
                 stack5
                 (emit-mov-rcx-from-local (native-value-window-spill-offset frame-base-slot-count 2)))
        stack7 (concat-byte-vectors stack6 (emit-mov-third-stack-from-rcx))
        stack8 (concat-byte-vectors
                 stack7
                 (emit-mov-rcx-from-local (native-value-window-spill-offset frame-base-slot-count 3)))
        stack9 (concat-byte-vectors stack8 (emit-mov-second-stack-from-rcx))
        stack-setup (concat-byte-vectors
                      stack9
                      (emit-mov-r9-from-local (native-value-window-spill-offset frame-base-slot-count 4)))
        reg0 (concat-byte-vectors
               (emit-mov-top-stack-from-r9)
               (emit-mov-r9-from-local (native-value-window-spill-offset frame-base-slot-count 5)))
        reg1 (concat-byte-vectors reg0 (emit-mov-r8-from-local (native-value-window-spill-offset frame-base-slot-count 6)))
        reg2 (concat-byte-vectors reg1 (emit-mov-rcx-from-local (native-value-window-spill-offset frame-base-slot-count 7)))
        reg3 (concat-byte-vectors reg2 (emit-mov-rdx-from-local (native-value-window-spill-offset frame-base-slot-count 8)))
        reg4 (concat-byte-vectors reg3 (emit-mov-rsi-from-local (native-value-window-spill-offset frame-base-slot-count 9)))
        reg-setup (concat-byte-vectors reg4 (emit-mov-rdi-from-local (native-value-window-spill-offset frame-base-slot-count 10)))
        call-seq (concat-byte-vectors
                   (emit-call-rel32 rel)
                   (emit-add-rsp-imm32 64))]
    (concat-byte-vectors
      (concat-byte-vectors stack-setup reg-setup)
      call-seq)))

(defn emit-fourteen-arg-call-x86 [rel frame-base-slot-count]
  (let [stack0 (concat-byte-vectors
                 (emit-sub-rsp-imm32 64)
                 (emit-mov-eighth-stack-from-rax))
        stack1 (concat-byte-vectors stack0 (emit-mov-seventh-stack-from-rcx))
        stack2 (concat-byte-vectors
                 stack1
                 (emit-mov-rcx-from-local (native-value-window-spill-offset frame-base-slot-count 0)))
        stack3 (concat-byte-vectors stack2 (emit-mov-sixth-stack-from-rcx))
        stack4 (concat-byte-vectors
                 stack3
                 (emit-mov-rcx-from-local (native-value-window-spill-offset frame-base-slot-count 1)))
        stack5 (concat-byte-vectors stack4 (emit-mov-fifth-stack-from-rcx))
        stack6 (concat-byte-vectors
                 stack5
                 (emit-mov-rcx-from-local (native-value-window-spill-offset frame-base-slot-count 2)))
        stack7 (concat-byte-vectors stack6 (emit-mov-fourth-stack-from-rcx))
        stack8 (concat-byte-vectors
                 stack7
                 (emit-mov-rcx-from-local (native-value-window-spill-offset frame-base-slot-count 3)))
        stack9 (concat-byte-vectors stack8 (emit-mov-third-stack-from-rcx))
        stack10 (concat-byte-vectors
                  stack9
                  (emit-mov-rcx-from-local (native-value-window-spill-offset frame-base-slot-count 4)))
        stack11 (concat-byte-vectors stack10 (emit-mov-second-stack-from-rcx))
        stack-setup (concat-byte-vectors
                      stack11
                      (emit-mov-r9-from-local (native-value-window-spill-offset frame-base-slot-count 5)))
        reg0 (concat-byte-vectors
               (emit-mov-top-stack-from-r9)
               (emit-mov-r9-from-local (native-value-window-spill-offset frame-base-slot-count 6)))
        reg1 (concat-byte-vectors reg0 (emit-mov-r8-from-local (native-value-window-spill-offset frame-base-slot-count 7)))
        reg2 (concat-byte-vectors reg1 (emit-mov-rcx-from-local (native-value-window-spill-offset frame-base-slot-count 8)))
        reg3 (concat-byte-vectors reg2 (emit-mov-rdx-from-local (native-value-window-spill-offset frame-base-slot-count 9)))
        reg4 (concat-byte-vectors reg3 (emit-mov-rsi-from-local (native-value-window-spill-offset frame-base-slot-count 10)))
        reg-setup (concat-byte-vectors reg4 (emit-mov-rdi-from-local (native-value-window-spill-offset frame-base-slot-count 11)))
        call-seq (concat-byte-vectors
                   (emit-call-rel32 rel)
                   (emit-add-rsp-imm32 64))]
    (concat-byte-vectors
      (concat-byte-vectors stack-setup reg-setup)
      call-seq)))

(defn emit-fifteen-arg-call-x86 [rel frame-base-slot-count]
  (let [stack0 (concat-byte-vectors
                 (emit-sub-rsp-imm32 80)
                 (emit-mov-ninth-stack-from-rax))
        stack1 (concat-byte-vectors stack0 (emit-mov-eighth-stack-from-rcx))
        stack2 (concat-byte-vectors
                 stack1
                 (emit-mov-rcx-from-local (native-value-window-spill-offset frame-base-slot-count 0)))
        stack3 (concat-byte-vectors stack2 (emit-mov-seventh-stack-from-rcx))
        stack4 (concat-byte-vectors
                 stack3
                 (emit-mov-rcx-from-local (native-value-window-spill-offset frame-base-slot-count 1)))
        stack5 (concat-byte-vectors stack4 (emit-mov-sixth-stack-from-rcx))
        stack6 (concat-byte-vectors
                 stack5
                 (emit-mov-rcx-from-local (native-value-window-spill-offset frame-base-slot-count 2)))
        stack7 (concat-byte-vectors stack6 (emit-mov-fifth-stack-from-rcx))
        stack8 (concat-byte-vectors
                 stack7
                 (emit-mov-rcx-from-local (native-value-window-spill-offset frame-base-slot-count 3)))
        stack9 (concat-byte-vectors stack8 (emit-mov-fourth-stack-from-rcx))
        stack10 (concat-byte-vectors
                  stack9
                  (emit-mov-rcx-from-local (native-value-window-spill-offset frame-base-slot-count 4)))
        stack11 (concat-byte-vectors stack10 (emit-mov-third-stack-from-rcx))
        stack12 (concat-byte-vectors
                  stack11
                  (emit-mov-rcx-from-local (native-value-window-spill-offset frame-base-slot-count 5)))
        stack13 (concat-byte-vectors stack12 (emit-mov-second-stack-from-rcx))
        stack-setup (concat-byte-vectors
                      stack13
                      (emit-mov-r9-from-local (native-value-window-spill-offset frame-base-slot-count 6)))
        reg0 (concat-byte-vectors
               (emit-mov-top-stack-from-r9)
               (emit-mov-r9-from-local (native-value-window-spill-offset frame-base-slot-count 7)))
        reg1 (concat-byte-vectors reg0 (emit-mov-r8-from-local (native-value-window-spill-offset frame-base-slot-count 8)))
        reg2 (concat-byte-vectors reg1 (emit-mov-rcx-from-local (native-value-window-spill-offset frame-base-slot-count 9)))
        reg3 (concat-byte-vectors reg2 (emit-mov-rdx-from-local (native-value-window-spill-offset frame-base-slot-count 10)))
        reg4 (concat-byte-vectors reg3 (emit-mov-rsi-from-local (native-value-window-spill-offset frame-base-slot-count 11)))
        reg-setup (concat-byte-vectors reg4 (emit-mov-rdi-from-local (native-value-window-spill-offset frame-base-slot-count 12)))
        call-seq (concat-byte-vectors
                   (emit-call-rel32 rel)
                   (emit-add-rsp-imm32 80))]
    (concat-byte-vectors
      (concat-byte-vectors stack-setup reg-setup)
      call-seq)))

(defn emit-sixteen-arg-call-x86 [rel frame-base-slot-count]
  (let [stack0 (concat-byte-vectors
                 (emit-sub-rsp-imm32 80)
                 (emit-mov-tenth-stack-from-rax))
        stack1 (concat-byte-vectors stack0 (emit-mov-ninth-stack-from-rcx))
        stack2 (concat-byte-vectors
                 stack1
                 (emit-mov-rcx-from-local (native-value-window-spill-offset frame-base-slot-count 0)))
        stack3 (concat-byte-vectors stack2 (emit-mov-eighth-stack-from-rcx))
        stack4 (concat-byte-vectors
                 stack3
                 (emit-mov-rcx-from-local (native-value-window-spill-offset frame-base-slot-count 1)))
        stack5 (concat-byte-vectors stack4 (emit-mov-seventh-stack-from-rcx))
        stack6 (concat-byte-vectors
                 stack5
                 (emit-mov-rcx-from-local (native-value-window-spill-offset frame-base-slot-count 2)))
        stack7 (concat-byte-vectors stack6 (emit-mov-sixth-stack-from-rcx))
        stack8 (concat-byte-vectors
                 stack7
                 (emit-mov-rcx-from-local (native-value-window-spill-offset frame-base-slot-count 3)))
        stack9 (concat-byte-vectors stack8 (emit-mov-fifth-stack-from-rcx))
        stack10 (concat-byte-vectors
                  stack9
                  (emit-mov-rcx-from-local (native-value-window-spill-offset frame-base-slot-count 4)))
        stack11 (concat-byte-vectors stack10 (emit-mov-fourth-stack-from-rcx))
        stack12 (concat-byte-vectors
                  stack11
                  (emit-mov-rcx-from-local (native-value-window-spill-offset frame-base-slot-count 5)))
        stack13 (concat-byte-vectors stack12 (emit-mov-third-stack-from-rcx))
        stack14 (concat-byte-vectors
                  stack13
                  (emit-mov-rcx-from-local (native-value-window-spill-offset frame-base-slot-count 6)))
        stack15 (concat-byte-vectors stack14 (emit-mov-second-stack-from-rcx))
        stack-setup (concat-byte-vectors
                      stack15
                      (emit-mov-r9-from-local (native-value-window-spill-offset frame-base-slot-count 7)))
        reg0 (concat-byte-vectors
               (emit-mov-top-stack-from-r9)
               (emit-mov-r9-from-local (native-value-window-spill-offset frame-base-slot-count 8)))
        reg1 (concat-byte-vectors reg0 (emit-mov-r8-from-local (native-value-window-spill-offset frame-base-slot-count 9)))
        reg2 (concat-byte-vectors reg1 (emit-mov-rcx-from-local (native-value-window-spill-offset frame-base-slot-count 10)))
        reg3 (concat-byte-vectors reg2 (emit-mov-rdx-from-local (native-value-window-spill-offset frame-base-slot-count 11)))
        reg4 (concat-byte-vectors reg3 (emit-mov-rsi-from-local (native-value-window-spill-offset frame-base-slot-count 12)))
        reg-setup (concat-byte-vectors reg4 (emit-mov-rdi-from-local (native-value-window-spill-offset frame-base-slot-count 13)))
        call-seq (concat-byte-vectors
                   (emit-call-rel32 rel)
                   (emit-add-rsp-imm32 80))]
    (concat-byte-vectors
      (concat-byte-vectors stack-setup reg-setup)
      call-seq)))

(defn emit-seventeen-arg-call-x86 [rel frame-base-slot-count]
  (let [stack0 (concat-byte-vectors
                 (emit-sub-rsp-imm32 96)
                 (emit-mov-eleventh-stack-from-rax))
        stack1 (concat-byte-vectors stack0 (emit-mov-tenth-stack-from-rcx))
        stack2 (concat-byte-vectors
                 stack1
                 (emit-mov-rcx-from-local (native-value-window-spill-offset frame-base-slot-count 0)))
        stack3 (concat-byte-vectors stack2 (emit-mov-ninth-stack-from-rcx))
        stack4 (concat-byte-vectors
                 stack3
                 (emit-mov-rcx-from-local (native-value-window-spill-offset frame-base-slot-count 1)))
        stack5 (concat-byte-vectors stack4 (emit-mov-eighth-stack-from-rcx))
        stack6 (concat-byte-vectors
                 stack5
                 (emit-mov-rcx-from-local (native-value-window-spill-offset frame-base-slot-count 2)))
        stack7 (concat-byte-vectors stack6 (emit-mov-seventh-stack-from-rcx))
        stack8 (concat-byte-vectors
                 stack7
                 (emit-mov-rcx-from-local (native-value-window-spill-offset frame-base-slot-count 3)))
        stack9 (concat-byte-vectors stack8 (emit-mov-sixth-stack-from-rcx))
        stack10 (concat-byte-vectors
                  stack9
                  (emit-mov-rcx-from-local (native-value-window-spill-offset frame-base-slot-count 4)))
        stack11 (concat-byte-vectors stack10 (emit-mov-fifth-stack-from-rcx))
        stack12 (concat-byte-vectors
                  stack11
                  (emit-mov-rcx-from-local (native-value-window-spill-offset frame-base-slot-count 5)))
        stack13 (concat-byte-vectors stack12 (emit-mov-fourth-stack-from-rcx))
        stack14 (concat-byte-vectors
                  stack13
                  (emit-mov-rcx-from-local (native-value-window-spill-offset frame-base-slot-count 6)))
        stack15 (concat-byte-vectors stack14 (emit-mov-third-stack-from-rcx))
        stack16 (concat-byte-vectors
                  stack15
                  (emit-mov-rcx-from-local (native-value-window-spill-offset frame-base-slot-count 7)))
        stack17 (concat-byte-vectors stack16 (emit-mov-second-stack-from-rcx))
        stack-setup (concat-byte-vectors
                      stack17
                      (emit-mov-r9-from-local (native-value-window-spill-offset frame-base-slot-count 8)))
        reg0 (concat-byte-vectors
               (emit-mov-top-stack-from-r9)
               (emit-mov-r9-from-local (native-value-window-spill-offset frame-base-slot-count 9)))
        reg1 (concat-byte-vectors reg0 (emit-mov-r8-from-local (native-value-window-spill-offset frame-base-slot-count 10)))
        reg2 (concat-byte-vectors reg1 (emit-mov-rcx-from-local (native-value-window-spill-offset frame-base-slot-count 11)))
        reg3 (concat-byte-vectors reg2 (emit-mov-rdx-from-local (native-value-window-spill-offset frame-base-slot-count 12)))
        reg4 (concat-byte-vectors reg3 (emit-mov-rsi-from-local (native-value-window-spill-offset frame-base-slot-count 13)))
        reg-setup (concat-byte-vectors reg4 (emit-mov-rdi-from-local (native-value-window-spill-offset frame-base-slot-count 14)))
        call-seq (concat-byte-vectors
                   (emit-call-rel32 rel)
                   (emit-add-rsp-imm32 96))]
    (concat-byte-vectors
      (concat-byte-vectors stack-setup reg-setup)
      call-seq)))

(defn emit-eighteen-arg-call-x86 [rel frame-base-slot-count]
  (let [stack0 (concat-byte-vectors
                 (emit-sub-rsp-imm32 96)
                 (emit-mov-twelfth-stack-from-rax))
        stack1 (concat-byte-vectors stack0 (emit-mov-eleventh-stack-from-rcx))
        stack2 (concat-byte-vectors
                 stack1
                 (emit-mov-rcx-from-local (native-value-window-spill-offset frame-base-slot-count 0)))
        stack3 (concat-byte-vectors stack2 (emit-mov-tenth-stack-from-rcx))
        stack4 (concat-byte-vectors
                 stack3
                 (emit-mov-rcx-from-local (native-value-window-spill-offset frame-base-slot-count 1)))
        stack5 (concat-byte-vectors stack4 (emit-mov-ninth-stack-from-rcx))
        stack6 (concat-byte-vectors
                 stack5
                 (emit-mov-rcx-from-local (native-value-window-spill-offset frame-base-slot-count 2)))
        stack7 (concat-byte-vectors stack6 (emit-mov-eighth-stack-from-rcx))
        stack8 (concat-byte-vectors
                 stack7
                 (emit-mov-rcx-from-local (native-value-window-spill-offset frame-base-slot-count 3)))
        stack9 (concat-byte-vectors stack8 (emit-mov-seventh-stack-from-rcx))
        stack10 (concat-byte-vectors
                  stack9
                  (emit-mov-rcx-from-local (native-value-window-spill-offset frame-base-slot-count 4)))
        stack11 (concat-byte-vectors stack10 (emit-mov-sixth-stack-from-rcx))
        stack12 (concat-byte-vectors
                  stack11
                  (emit-mov-rcx-from-local (native-value-window-spill-offset frame-base-slot-count 5)))
        stack13 (concat-byte-vectors stack12 (emit-mov-fifth-stack-from-rcx))
        stack14 (concat-byte-vectors
                  stack13
                  (emit-mov-rcx-from-local (native-value-window-spill-offset frame-base-slot-count 6)))
        stack15 (concat-byte-vectors stack14 (emit-mov-fourth-stack-from-rcx))
        stack16 (concat-byte-vectors
                  stack15
                  (emit-mov-rcx-from-local (native-value-window-spill-offset frame-base-slot-count 7)))
        stack17 (concat-byte-vectors stack16 (emit-mov-third-stack-from-rcx))
        stack18 (concat-byte-vectors
                  stack17
                  (emit-mov-rcx-from-local (native-value-window-spill-offset frame-base-slot-count 8)))
        stack19 (concat-byte-vectors stack18 (emit-mov-second-stack-from-rcx))
        stack-setup (concat-byte-vectors
                      stack19
                      (emit-mov-r9-from-local (native-value-window-spill-offset frame-base-slot-count 9)))
        reg0 (concat-byte-vectors
               (emit-mov-top-stack-from-r9)
               (emit-mov-r9-from-local (native-value-window-spill-offset frame-base-slot-count 10)))
        reg1 (concat-byte-vectors reg0 (emit-mov-r8-from-local (native-value-window-spill-offset frame-base-slot-count 11)))
        reg2 (concat-byte-vectors reg1 (emit-mov-rcx-from-local (native-value-window-spill-offset frame-base-slot-count 12)))
        reg3 (concat-byte-vectors reg2 (emit-mov-rdx-from-local (native-value-window-spill-offset frame-base-slot-count 13)))
        reg4 (concat-byte-vectors reg3 (emit-mov-rsi-from-local (native-value-window-spill-offset frame-base-slot-count 14)))
        reg-setup (concat-byte-vectors reg4 (emit-mov-rdi-from-local (native-value-window-spill-offset frame-base-slot-count 15)))
        call-seq (concat-byte-vectors
                   (emit-call-rel32 rel)
                   (emit-add-rsp-imm32 96))]
    (concat-byte-vectors
      (concat-byte-vectors stack-setup reg-setup)
      call-seq)))

(defn emit-twenty-five-arg-call-x86 [rel frame-base-slot-count]
  (let [stack0 (concat-byte-vectors
                 (emit-sub-rsp-imm32 160)
                 (emit-mov-nineteenth-stack-from-rax))
        stack1 (concat-byte-vectors stack0 (emit-mov-eighteenth-stack-from-rcx))
        stack2 (concat-byte-vectors
                 stack1
                 (emit-mov-rcx-from-local (native-value-window-spill-offset frame-base-slot-count 0)))
        stack3 (concat-byte-vectors stack2 (emit-mov-seventeenth-stack-from-rcx))
        stack4 (concat-byte-vectors
                 stack3
                 (emit-mov-rcx-from-local (native-value-window-spill-offset frame-base-slot-count 1)))
        stack5 (concat-byte-vectors stack4 (emit-mov-sixteenth-stack-from-rcx))
        stack6 (concat-byte-vectors
                 stack5
                 (emit-mov-rcx-from-local (native-value-window-spill-offset frame-base-slot-count 2)))
        stack7 (concat-byte-vectors stack6 (emit-mov-fifteenth-stack-from-rcx))
        stack8 (concat-byte-vectors
                 stack7
                 (emit-mov-rcx-from-local (native-value-window-spill-offset frame-base-slot-count 3)))
        stack9 (concat-byte-vectors stack8 (emit-mov-fourteenth-stack-from-rcx))
        stack10 (concat-byte-vectors
                  stack9
                  (emit-mov-rcx-from-local (native-value-window-spill-offset frame-base-slot-count 4)))
        stack11 (concat-byte-vectors stack10 (emit-mov-thirteenth-stack-from-rcx))
        stack12 (concat-byte-vectors
                  stack11
                  (emit-mov-rcx-from-local (native-value-window-spill-offset frame-base-slot-count 5)))
        stack13 (concat-byte-vectors stack12 (emit-mov-twelfth-stack-from-rcx))
        stack14 (concat-byte-vectors
                  stack13
                  (emit-mov-rcx-from-local (native-value-window-spill-offset frame-base-slot-count 6)))
        stack15 (concat-byte-vectors stack14 (emit-mov-eleventh-stack-from-rcx))
        stack16 (concat-byte-vectors
                  stack15
                  (emit-mov-rcx-from-local (native-value-window-spill-offset frame-base-slot-count 7)))
        stack17 (concat-byte-vectors stack16 (emit-mov-tenth-stack-from-rcx))
        stack18 (concat-byte-vectors
                  stack17
                  (emit-mov-rcx-from-local (native-value-window-spill-offset frame-base-slot-count 8)))
        stack19 (concat-byte-vectors stack18 (emit-mov-ninth-stack-from-rcx))
        stack20 (concat-byte-vectors
                  stack19
                  (emit-mov-rcx-from-local (native-value-window-spill-offset frame-base-slot-count 9)))
        stack21 (concat-byte-vectors stack20 (emit-mov-eighth-stack-from-rcx))
        stack22 (concat-byte-vectors
                  stack21
                  (emit-mov-rcx-from-local (native-value-window-spill-offset frame-base-slot-count 10)))
        stack23 (concat-byte-vectors stack22 (emit-mov-seventh-stack-from-rcx))
        stack24 (concat-byte-vectors
                  stack23
                  (emit-mov-rcx-from-local (native-value-window-spill-offset frame-base-slot-count 11)))
        stack25 (concat-byte-vectors stack24 (emit-mov-sixth-stack-from-rcx))
        stack26 (concat-byte-vectors
                  stack25
                  (emit-mov-rcx-from-local (native-value-window-spill-offset frame-base-slot-count 12)))
        stack27 (concat-byte-vectors stack26 (emit-mov-fifth-stack-from-rcx))
        stack28 (concat-byte-vectors
                  stack27
                  (emit-mov-rcx-from-local (native-value-window-spill-offset frame-base-slot-count 13)))
        stack29 (concat-byte-vectors stack28 (emit-mov-fourth-stack-from-rcx))
        stack30 (concat-byte-vectors
                  stack29
                  (emit-mov-rcx-from-local (native-value-window-spill-offset frame-base-slot-count 14)))
        stack31 (concat-byte-vectors stack30 (emit-mov-third-stack-from-rcx))
        stack32 (concat-byte-vectors
                  stack31
                  (emit-mov-rcx-from-local (native-value-window-spill-offset frame-base-slot-count 15)))
        stack33 (concat-byte-vectors stack32 (emit-mov-second-stack-from-rcx))
        stack-setup (concat-byte-vectors
                      stack33
                      (emit-mov-r9-from-local (native-value-window-spill-offset frame-base-slot-count 16)))
        reg0 (concat-byte-vectors
               (emit-mov-top-stack-from-r9)
               (emit-mov-r9-from-local (native-value-window-spill-offset frame-base-slot-count 17)))
        reg1 (concat-byte-vectors reg0 (emit-mov-r8-from-local (native-value-window-spill-offset frame-base-slot-count 18)))
        reg2 (concat-byte-vectors reg1 (emit-mov-rcx-from-local (native-value-window-spill-offset frame-base-slot-count 19)))
        reg3 (concat-byte-vectors reg2 (emit-mov-rdx-from-local (native-value-window-spill-offset frame-base-slot-count 20)))
        reg4 (concat-byte-vectors reg3 (emit-mov-rsi-from-local (native-value-window-spill-offset frame-base-slot-count 21)))
        reg-setup (concat-byte-vectors reg4 (emit-mov-rdi-from-local (native-value-window-spill-offset frame-base-slot-count 22)))
        call-seq (concat-byte-vectors
                   (emit-call-rel32 rel)
                   (emit-add-rsp-imm32 160))]
    (concat-byte-vectors
      (concat-byte-vectors stack-setup reg-setup)
      call-seq)))

(defn emit-twenty-four-arg-call-x86 [rel frame-base-slot-count]
  (let [stack0 (concat-byte-vectors
                 (emit-sub-rsp-imm32 144)
                 (emit-mov-eighteenth-stack-from-rax))
        stack1 (concat-byte-vectors stack0 (emit-mov-seventeenth-stack-from-rcx))
        stack2 (concat-byte-vectors
                 stack1
                 (emit-mov-rcx-from-local (native-value-window-spill-offset frame-base-slot-count 0)))
        stack3 (concat-byte-vectors stack2 (emit-mov-sixteenth-stack-from-rcx))
        stack4 (concat-byte-vectors
                 stack3
                 (emit-mov-rcx-from-local (native-value-window-spill-offset frame-base-slot-count 1)))
        stack5 (concat-byte-vectors stack4 (emit-mov-fifteenth-stack-from-rcx))
        stack6 (concat-byte-vectors
                 stack5
                 (emit-mov-rcx-from-local (native-value-window-spill-offset frame-base-slot-count 2)))
        stack7 (concat-byte-vectors stack6 (emit-mov-fourteenth-stack-from-rcx))
        stack8 (concat-byte-vectors
                 stack7
                 (emit-mov-rcx-from-local (native-value-window-spill-offset frame-base-slot-count 3)))
        stack9 (concat-byte-vectors stack8 (emit-mov-thirteenth-stack-from-rcx))
        stack10 (concat-byte-vectors
                  stack9
                  (emit-mov-rcx-from-local (native-value-window-spill-offset frame-base-slot-count 4)))
        stack11 (concat-byte-vectors stack10 (emit-mov-twelfth-stack-from-rcx))
        stack12 (concat-byte-vectors
                  stack11
                  (emit-mov-rcx-from-local (native-value-window-spill-offset frame-base-slot-count 5)))
        stack13 (concat-byte-vectors stack12 (emit-mov-eleventh-stack-from-rcx))
        stack14 (concat-byte-vectors
                  stack13
                  (emit-mov-rcx-from-local (native-value-window-spill-offset frame-base-slot-count 6)))
        stack15 (concat-byte-vectors stack14 (emit-mov-tenth-stack-from-rcx))
        stack16 (concat-byte-vectors
                  stack15
                  (emit-mov-rcx-from-local (native-value-window-spill-offset frame-base-slot-count 7)))
        stack17 (concat-byte-vectors stack16 (emit-mov-ninth-stack-from-rcx))
        stack18 (concat-byte-vectors
                  stack17
                  (emit-mov-rcx-from-local (native-value-window-spill-offset frame-base-slot-count 8)))
        stack19 (concat-byte-vectors stack18 (emit-mov-eighth-stack-from-rcx))
        stack20 (concat-byte-vectors
                  stack19
                  (emit-mov-rcx-from-local (native-value-window-spill-offset frame-base-slot-count 9)))
        stack21 (concat-byte-vectors stack20 (emit-mov-seventh-stack-from-rcx))
        stack22 (concat-byte-vectors
                  stack21
                  (emit-mov-rcx-from-local (native-value-window-spill-offset frame-base-slot-count 10)))
        stack23 (concat-byte-vectors stack22 (emit-mov-sixth-stack-from-rcx))
        stack24 (concat-byte-vectors
                  stack23
                  (emit-mov-rcx-from-local (native-value-window-spill-offset frame-base-slot-count 11)))
        stack25 (concat-byte-vectors stack24 (emit-mov-fifth-stack-from-rcx))
        stack26 (concat-byte-vectors
                  stack25
                  (emit-mov-rcx-from-local (native-value-window-spill-offset frame-base-slot-count 12)))
        stack27 (concat-byte-vectors stack26 (emit-mov-fourth-stack-from-rcx))
        stack28 (concat-byte-vectors
                  stack27
                  (emit-mov-rcx-from-local (native-value-window-spill-offset frame-base-slot-count 13)))
        stack29 (concat-byte-vectors stack28 (emit-mov-third-stack-from-rcx))
        stack30 (concat-byte-vectors
                  stack29
                  (emit-mov-rcx-from-local (native-value-window-spill-offset frame-base-slot-count 14)))
        stack31 (concat-byte-vectors stack30 (emit-mov-second-stack-from-rcx))
        stack-setup (concat-byte-vectors
                      stack31
                      (emit-mov-r9-from-local (native-value-window-spill-offset frame-base-slot-count 15)))
        reg0 (concat-byte-vectors
               (emit-mov-top-stack-from-r9)
               (emit-mov-r9-from-local (native-value-window-spill-offset frame-base-slot-count 16)))
        reg1 (concat-byte-vectors reg0 (emit-mov-r8-from-local (native-value-window-spill-offset frame-base-slot-count 17)))
        reg2 (concat-byte-vectors reg1 (emit-mov-rcx-from-local (native-value-window-spill-offset frame-base-slot-count 18)))
        reg3 (concat-byte-vectors reg2 (emit-mov-rdx-from-local (native-value-window-spill-offset frame-base-slot-count 19)))
        reg4 (concat-byte-vectors reg3 (emit-mov-rsi-from-local (native-value-window-spill-offset frame-base-slot-count 20)))
        reg-setup (concat-byte-vectors reg4 (emit-mov-rdi-from-local (native-value-window-spill-offset frame-base-slot-count 21)))
        call-seq (concat-byte-vectors
                   (emit-call-rel32 rel)
                   (emit-add-rsp-imm32 144))]
    (concat-byte-vectors
      (concat-byte-vectors stack-setup reg-setup)
      call-seq)))

(defn emit-twenty-three-arg-call-x86 [rel frame-base-slot-count]
  (let [stack0 (concat-byte-vectors
                 (emit-sub-rsp-imm32 144)
                 (emit-mov-seventeenth-stack-from-rax))
        stack1 (concat-byte-vectors stack0 (emit-mov-sixteenth-stack-from-rcx))
        stack2 (concat-byte-vectors
                 stack1
                 (emit-mov-rcx-from-local (native-value-window-spill-offset frame-base-slot-count 0)))
        stack3 (concat-byte-vectors stack2 (emit-mov-fifteenth-stack-from-rcx))
        stack4 (concat-byte-vectors
                 stack3
                 (emit-mov-rcx-from-local (native-value-window-spill-offset frame-base-slot-count 1)))
        stack5 (concat-byte-vectors stack4 (emit-mov-fourteenth-stack-from-rcx))
        stack6 (concat-byte-vectors
                 stack5
                 (emit-mov-rcx-from-local (native-value-window-spill-offset frame-base-slot-count 2)))
        stack7 (concat-byte-vectors stack6 (emit-mov-thirteenth-stack-from-rcx))
        stack8 (concat-byte-vectors
                 stack7
                 (emit-mov-rcx-from-local (native-value-window-spill-offset frame-base-slot-count 3)))
        stack9 (concat-byte-vectors stack8 (emit-mov-twelfth-stack-from-rcx))
        stack10 (concat-byte-vectors
                   stack9
                   (emit-mov-rcx-from-local (native-value-window-spill-offset frame-base-slot-count 4)))
        stack11 (concat-byte-vectors stack10 (emit-mov-eleventh-stack-from-rcx))
        stack12 (concat-byte-vectors
                   stack11
                   (emit-mov-rcx-from-local (native-value-window-spill-offset frame-base-slot-count 5)))
        stack13 (concat-byte-vectors stack12 (emit-mov-tenth-stack-from-rcx))
        stack14 (concat-byte-vectors
                   stack13
                   (emit-mov-rcx-from-local (native-value-window-spill-offset frame-base-slot-count 6)))
        stack15 (concat-byte-vectors stack14 (emit-mov-ninth-stack-from-rcx))
        stack16 (concat-byte-vectors
                   stack15
                   (emit-mov-rcx-from-local (native-value-window-spill-offset frame-base-slot-count 7)))
        stack17 (concat-byte-vectors stack16 (emit-mov-eighth-stack-from-rcx))
        stack18 (concat-byte-vectors
                   stack17
                   (emit-mov-rcx-from-local (native-value-window-spill-offset frame-base-slot-count 8)))
        stack19 (concat-byte-vectors stack18 (emit-mov-seventh-stack-from-rcx))
        stack20 (concat-byte-vectors
                   stack19
                   (emit-mov-rcx-from-local (native-value-window-spill-offset frame-base-slot-count 9)))
        stack21 (concat-byte-vectors stack20 (emit-mov-sixth-stack-from-rcx))
        stack22 (concat-byte-vectors
                   stack21
                   (emit-mov-rcx-from-local (native-value-window-spill-offset frame-base-slot-count 10)))
        stack23 (concat-byte-vectors stack22 (emit-mov-fifth-stack-from-rcx))
        stack24 (concat-byte-vectors
                   stack23
                   (emit-mov-rcx-from-local (native-value-window-spill-offset frame-base-slot-count 11)))
        stack25 (concat-byte-vectors stack24 (emit-mov-fourth-stack-from-rcx))
        stack26 (concat-byte-vectors
                   stack25
                   (emit-mov-rcx-from-local (native-value-window-spill-offset frame-base-slot-count 12)))
        stack27 (concat-byte-vectors stack26 (emit-mov-third-stack-from-rcx))
        stack28 (concat-byte-vectors
                   stack27
                   (emit-mov-rcx-from-local (native-value-window-spill-offset frame-base-slot-count 13)))
        stack29 (concat-byte-vectors stack28 (emit-mov-second-stack-from-rcx))
        stack-setup (concat-byte-vectors
                      stack29
                      (emit-mov-r9-from-local (native-value-window-spill-offset frame-base-slot-count 14)))
        reg0 (concat-byte-vectors
               (emit-mov-top-stack-from-r9)
               (emit-mov-r9-from-local (native-value-window-spill-offset frame-base-slot-count 15)))
        reg1 (concat-byte-vectors reg0 (emit-mov-r8-from-local (native-value-window-spill-offset frame-base-slot-count 16)))
        reg2 (concat-byte-vectors reg1 (emit-mov-rcx-from-local (native-value-window-spill-offset frame-base-slot-count 17)))
        reg3 (concat-byte-vectors reg2 (emit-mov-rdx-from-local (native-value-window-spill-offset frame-base-slot-count 18)))
        reg4 (concat-byte-vectors reg3 (emit-mov-rsi-from-local (native-value-window-spill-offset frame-base-slot-count 19)))
        reg-setup (concat-byte-vectors reg4 (emit-mov-rdi-from-local (native-value-window-spill-offset frame-base-slot-count 20)))
        call-seq (concat-byte-vectors
                   (emit-call-rel32 rel)
                   (emit-add-rsp-imm32 144))]
    (concat-byte-vectors
      (concat-byte-vectors stack-setup reg-setup)
      call-seq)))

(defn emit-twenty-two-arg-call-x86 [rel frame-base-slot-count]
  (let [stack0 (concat-byte-vectors
                 (emit-sub-rsp-imm32 128)
                 (emit-mov-sixteenth-stack-from-rax))
        stack1 (concat-byte-vectors stack0 (emit-mov-fifteenth-stack-from-rcx))
        stack2 (concat-byte-vectors
                 stack1
                 (emit-mov-rcx-from-local (native-value-window-spill-offset frame-base-slot-count 0)))
        stack3 (concat-byte-vectors stack2 (emit-mov-fourteenth-stack-from-rcx))
        stack4 (concat-byte-vectors
                 stack3
                 (emit-mov-rcx-from-local (native-value-window-spill-offset frame-base-slot-count 1)))
        stack5 (concat-byte-vectors stack4 (emit-mov-thirteenth-stack-from-rcx))
        stack6 (concat-byte-vectors
                 stack5
                 (emit-mov-rcx-from-local (native-value-window-spill-offset frame-base-slot-count 2)))
        stack7 (concat-byte-vectors stack6 (emit-mov-twelfth-stack-from-rcx))
        stack8 (concat-byte-vectors
                 stack7
                 (emit-mov-rcx-from-local (native-value-window-spill-offset frame-base-slot-count 3)))
        stack9 (concat-byte-vectors stack8 (emit-mov-eleventh-stack-from-rcx))
        stack10 (concat-byte-vectors
                  stack9
                  (emit-mov-rcx-from-local (native-value-window-spill-offset frame-base-slot-count 4)))
        stack11 (concat-byte-vectors stack10 (emit-mov-tenth-stack-from-rcx))
        stack12 (concat-byte-vectors
                  stack11
                  (emit-mov-rcx-from-local (native-value-window-spill-offset frame-base-slot-count 5)))
        stack13 (concat-byte-vectors stack12 (emit-mov-ninth-stack-from-rcx))
        stack14 (concat-byte-vectors
                  stack13
                  (emit-mov-rcx-from-local (native-value-window-spill-offset frame-base-slot-count 6)))
        stack15 (concat-byte-vectors stack14 (emit-mov-eighth-stack-from-rcx))
        stack16 (concat-byte-vectors
                  stack15
                  (emit-mov-rcx-from-local (native-value-window-spill-offset frame-base-slot-count 7)))
        stack17 (concat-byte-vectors stack16 (emit-mov-seventh-stack-from-rcx))
        stack18 (concat-byte-vectors
                  stack17
                  (emit-mov-rcx-from-local (native-value-window-spill-offset frame-base-slot-count 8)))
        stack19 (concat-byte-vectors stack18 (emit-mov-sixth-stack-from-rcx))
        stack20 (concat-byte-vectors
                  stack19
                  (emit-mov-rcx-from-local (native-value-window-spill-offset frame-base-slot-count 9)))
        stack21 (concat-byte-vectors stack20 (emit-mov-fifth-stack-from-rcx))
        stack22 (concat-byte-vectors
                  stack21
                  (emit-mov-rcx-from-local (native-value-window-spill-offset frame-base-slot-count 10)))
        stack23 (concat-byte-vectors stack22 (emit-mov-fourth-stack-from-rcx))
        stack24 (concat-byte-vectors
                  stack23
                  (emit-mov-rcx-from-local (native-value-window-spill-offset frame-base-slot-count 11)))
        stack25 (concat-byte-vectors stack24 (emit-mov-third-stack-from-rcx))
        stack26 (concat-byte-vectors
                  stack25
                  (emit-mov-rcx-from-local (native-value-window-spill-offset frame-base-slot-count 12)))
        stack27 (concat-byte-vectors stack26 (emit-mov-second-stack-from-rcx))
        stack-setup (concat-byte-vectors
                      stack27
                      (emit-mov-r9-from-local (native-value-window-spill-offset frame-base-slot-count 13)))
        reg0 (concat-byte-vectors
               (emit-mov-top-stack-from-r9)
               (emit-mov-r9-from-local (native-value-window-spill-offset frame-base-slot-count 14)))
        reg1 (concat-byte-vectors reg0 (emit-mov-r8-from-local (native-value-window-spill-offset frame-base-slot-count 15)))
        reg2 (concat-byte-vectors reg1 (emit-mov-rcx-from-local (native-value-window-spill-offset frame-base-slot-count 16)))
        reg3 (concat-byte-vectors reg2 (emit-mov-rdx-from-local (native-value-window-spill-offset frame-base-slot-count 17)))
        reg4 (concat-byte-vectors reg3 (emit-mov-rsi-from-local (native-value-window-spill-offset frame-base-slot-count 18)))
        reg-setup (concat-byte-vectors reg4 (emit-mov-rdi-from-local (native-value-window-spill-offset frame-base-slot-count 19)))
        call-seq (concat-byte-vectors
                   (emit-call-rel32 rel)
                   (emit-add-rsp-imm32 128))]
    (concat-byte-vectors
      (concat-byte-vectors stack-setup reg-setup)
      call-seq)))

(defn emit-twenty-one-arg-call-x86 [rel frame-base-slot-count]
  (let [stack0 (concat-byte-vectors
                 (emit-sub-rsp-imm32 128)
                 (emit-mov-fifteenth-stack-from-rax))
        stack1 (concat-byte-vectors stack0 (emit-mov-fourteenth-stack-from-rcx))
        stack2 (concat-byte-vectors
                 stack1
                 (emit-mov-rcx-from-local (native-value-window-spill-offset frame-base-slot-count 0)))
        stack3 (concat-byte-vectors stack2 (emit-mov-thirteenth-stack-from-rcx))
        stack4 (concat-byte-vectors
                 stack3
                 (emit-mov-rcx-from-local (native-value-window-spill-offset frame-base-slot-count 1)))
        stack5 (concat-byte-vectors stack4 (emit-mov-twelfth-stack-from-rcx))
        stack6 (concat-byte-vectors
                 stack5
                 (emit-mov-rcx-from-local (native-value-window-spill-offset frame-base-slot-count 2)))
        stack7 (concat-byte-vectors stack6 (emit-mov-eleventh-stack-from-rcx))
        stack8 (concat-byte-vectors
                 stack7
                 (emit-mov-rcx-from-local (native-value-window-spill-offset frame-base-slot-count 3)))
        stack9 (concat-byte-vectors stack8 (emit-mov-tenth-stack-from-rcx))
        stack10 (concat-byte-vectors
                  stack9
                  (emit-mov-rcx-from-local (native-value-window-spill-offset frame-base-slot-count 4)))
        stack11 (concat-byte-vectors stack10 (emit-mov-ninth-stack-from-rcx))
        stack12 (concat-byte-vectors
                  stack11
                  (emit-mov-rcx-from-local (native-value-window-spill-offset frame-base-slot-count 5)))
        stack13 (concat-byte-vectors stack12 (emit-mov-eighth-stack-from-rcx))
        stack14 (concat-byte-vectors
                  stack13
                  (emit-mov-rcx-from-local (native-value-window-spill-offset frame-base-slot-count 6)))
        stack15 (concat-byte-vectors stack14 (emit-mov-seventh-stack-from-rcx))
        stack16 (concat-byte-vectors
                  stack15
                  (emit-mov-rcx-from-local (native-value-window-spill-offset frame-base-slot-count 7)))
        stack17 (concat-byte-vectors stack16 (emit-mov-sixth-stack-from-rcx))
        stack18 (concat-byte-vectors
                  stack17
                  (emit-mov-rcx-from-local (native-value-window-spill-offset frame-base-slot-count 8)))
        stack19 (concat-byte-vectors stack18 (emit-mov-fifth-stack-from-rcx))
        stack20 (concat-byte-vectors
                  stack19
                  (emit-mov-rcx-from-local (native-value-window-spill-offset frame-base-slot-count 9)))
        stack21 (concat-byte-vectors stack20 (emit-mov-fourth-stack-from-rcx))
        stack22 (concat-byte-vectors
                  stack21
                  (emit-mov-rcx-from-local (native-value-window-spill-offset frame-base-slot-count 10)))
        stack23 (concat-byte-vectors stack22 (emit-mov-third-stack-from-rcx))
        stack24 (concat-byte-vectors
                  stack23
                  (emit-mov-rcx-from-local (native-value-window-spill-offset frame-base-slot-count 11)))
        stack25 (concat-byte-vectors stack24 (emit-mov-second-stack-from-rcx))
        stack-setup (concat-byte-vectors
                      stack25
                      (emit-mov-r9-from-local (native-value-window-spill-offset frame-base-slot-count 12)))
        reg0 (concat-byte-vectors
               (emit-mov-top-stack-from-r9)
               (emit-mov-r9-from-local (native-value-window-spill-offset frame-base-slot-count 13)))
        reg1 (concat-byte-vectors reg0 (emit-mov-r8-from-local (native-value-window-spill-offset frame-base-slot-count 14)))
        reg2 (concat-byte-vectors reg1 (emit-mov-rcx-from-local (native-value-window-spill-offset frame-base-slot-count 15)))
        reg3 (concat-byte-vectors reg2 (emit-mov-rdx-from-local (native-value-window-spill-offset frame-base-slot-count 16)))
        reg4 (concat-byte-vectors reg3 (emit-mov-rsi-from-local (native-value-window-spill-offset frame-base-slot-count 17)))
        reg-setup (concat-byte-vectors reg4 (emit-mov-rdi-from-local (native-value-window-spill-offset frame-base-slot-count 18)))
        call-seq (concat-byte-vectors
                   (emit-call-rel32 rel)
                   (emit-add-rsp-imm32 128))]
    (concat-byte-vectors
      (concat-byte-vectors stack-setup reg-setup)
      call-seq)))

(defn emit-twenty-arg-call-x86 [rel frame-base-slot-count]
  (let [stack0 (concat-byte-vectors
                 (emit-sub-rsp-imm32 112)
                 (emit-mov-fourteenth-stack-from-rax))
        stack1 (concat-byte-vectors stack0 (emit-mov-thirteenth-stack-from-rcx))
        stack2 (concat-byte-vectors
                 stack1
                 (emit-mov-rcx-from-local (native-value-window-spill-offset frame-base-slot-count 0)))
        stack3 (concat-byte-vectors stack2 (emit-mov-twelfth-stack-from-rcx))
        stack4 (concat-byte-vectors
                 stack3
                 (emit-mov-rcx-from-local (native-value-window-spill-offset frame-base-slot-count 1)))
        stack5 (concat-byte-vectors stack4 (emit-mov-eleventh-stack-from-rcx))
        stack6 (concat-byte-vectors
                 stack5
                 (emit-mov-rcx-from-local (native-value-window-spill-offset frame-base-slot-count 2)))
        stack7 (concat-byte-vectors stack6 (emit-mov-tenth-stack-from-rcx))
        stack8 (concat-byte-vectors
                 stack7
                 (emit-mov-rcx-from-local (native-value-window-spill-offset frame-base-slot-count 3)))
        stack9 (concat-byte-vectors stack8 (emit-mov-ninth-stack-from-rcx))
        stack10 (concat-byte-vectors
                  stack9
                  (emit-mov-rcx-from-local (native-value-window-spill-offset frame-base-slot-count 4)))
        stack11 (concat-byte-vectors stack10 (emit-mov-eighth-stack-from-rcx))
        stack12 (concat-byte-vectors
                  stack11
                  (emit-mov-rcx-from-local (native-value-window-spill-offset frame-base-slot-count 5)))
        stack13 (concat-byte-vectors stack12 (emit-mov-seventh-stack-from-rcx))
        stack14 (concat-byte-vectors
                  stack13
                  (emit-mov-rcx-from-local (native-value-window-spill-offset frame-base-slot-count 6)))
        stack15 (concat-byte-vectors stack14 (emit-mov-sixth-stack-from-rcx))
        stack16 (concat-byte-vectors
                  stack15
                  (emit-mov-rcx-from-local (native-value-window-spill-offset frame-base-slot-count 7)))
        stack17 (concat-byte-vectors stack16 (emit-mov-fifth-stack-from-rcx))
        stack18 (concat-byte-vectors
                  stack17
                  (emit-mov-rcx-from-local (native-value-window-spill-offset frame-base-slot-count 8)))
        stack19 (concat-byte-vectors stack18 (emit-mov-fourth-stack-from-rcx))
        stack20 (concat-byte-vectors
                  stack19
                  (emit-mov-rcx-from-local (native-value-window-spill-offset frame-base-slot-count 9)))
        stack21 (concat-byte-vectors stack20 (emit-mov-third-stack-from-rcx))
        stack22 (concat-byte-vectors
                  stack21
                  (emit-mov-rcx-from-local (native-value-window-spill-offset frame-base-slot-count 10)))
        stack23 (concat-byte-vectors stack22 (emit-mov-second-stack-from-rcx))
        stack-setup (concat-byte-vectors
                      stack23
                      (emit-mov-r9-from-local (native-value-window-spill-offset frame-base-slot-count 11)))
        reg0 (concat-byte-vectors
               (emit-mov-top-stack-from-r9)
               (emit-mov-r9-from-local (native-value-window-spill-offset frame-base-slot-count 12)))
        reg1 (concat-byte-vectors reg0 (emit-mov-r8-from-local (native-value-window-spill-offset frame-base-slot-count 13)))
        reg2 (concat-byte-vectors reg1 (emit-mov-rcx-from-local (native-value-window-spill-offset frame-base-slot-count 14)))
        reg3 (concat-byte-vectors reg2 (emit-mov-rdx-from-local (native-value-window-spill-offset frame-base-slot-count 15)))
        reg4 (concat-byte-vectors reg3 (emit-mov-rsi-from-local (native-value-window-spill-offset frame-base-slot-count 16)))
        reg-setup (concat-byte-vectors reg4 (emit-mov-rdi-from-local (native-value-window-spill-offset frame-base-slot-count 17)))
        call-seq (concat-byte-vectors
                   (emit-call-rel32 rel)
                   (emit-add-rsp-imm32 112))]
    (concat-byte-vectors
      (concat-byte-vectors stack-setup reg-setup)
      call-seq)))

(defn emit-nineteen-arg-call-x86 [rel frame-base-slot-count]
  (let [stack0 (concat-byte-vectors
                 (emit-sub-rsp-imm32 112)
                 (emit-mov-thirteenth-stack-from-rax))
        stack1 (concat-byte-vectors stack0 (emit-mov-twelfth-stack-from-rcx))
        stack2 (concat-byte-vectors
                 stack1
                 (emit-mov-rcx-from-local (native-value-window-spill-offset frame-base-slot-count 0)))
        stack3 (concat-byte-vectors stack2 (emit-mov-eleventh-stack-from-rcx))
        stack4 (concat-byte-vectors
                 stack3
                 (emit-mov-rcx-from-local (native-value-window-spill-offset frame-base-slot-count 1)))
        stack5 (concat-byte-vectors stack4 (emit-mov-tenth-stack-from-rcx))
        stack6 (concat-byte-vectors
                 stack5
                 (emit-mov-rcx-from-local (native-value-window-spill-offset frame-base-slot-count 2)))
        stack7 (concat-byte-vectors stack6 (emit-mov-ninth-stack-from-rcx))
        stack8 (concat-byte-vectors
                 stack7
                 (emit-mov-rcx-from-local (native-value-window-spill-offset frame-base-slot-count 3)))
        stack9 (concat-byte-vectors stack8 (emit-mov-eighth-stack-from-rcx))
        stack10 (concat-byte-vectors
                  stack9
                  (emit-mov-rcx-from-local (native-value-window-spill-offset frame-base-slot-count 4)))
        stack11 (concat-byte-vectors stack10 (emit-mov-seventh-stack-from-rcx))
        stack12 (concat-byte-vectors
                  stack11
                  (emit-mov-rcx-from-local (native-value-window-spill-offset frame-base-slot-count 5)))
        stack13 (concat-byte-vectors stack12 (emit-mov-sixth-stack-from-rcx))
        stack14 (concat-byte-vectors
                  stack13
                  (emit-mov-rcx-from-local (native-value-window-spill-offset frame-base-slot-count 6)))
        stack15 (concat-byte-vectors stack14 (emit-mov-fifth-stack-from-rcx))
        stack16 (concat-byte-vectors
                  stack15
                  (emit-mov-rcx-from-local (native-value-window-spill-offset frame-base-slot-count 7)))
        stack17 (concat-byte-vectors stack16 (emit-mov-fourth-stack-from-rcx))
        stack18 (concat-byte-vectors
                  stack17
                  (emit-mov-rcx-from-local (native-value-window-spill-offset frame-base-slot-count 8)))
        stack19 (concat-byte-vectors stack18 (emit-mov-third-stack-from-rcx))
        stack20 (concat-byte-vectors
                  stack19
                  (emit-mov-rcx-from-local (native-value-window-spill-offset frame-base-slot-count 9)))
        stack21 (concat-byte-vectors stack20 (emit-mov-second-stack-from-rcx))
        stack-setup (concat-byte-vectors
                      stack21
                      (emit-mov-r9-from-local (native-value-window-spill-offset frame-base-slot-count 10)))
        reg0 (concat-byte-vectors
               (emit-mov-top-stack-from-r9)
               (emit-mov-r9-from-local (native-value-window-spill-offset frame-base-slot-count 11)))
        reg1 (concat-byte-vectors reg0 (emit-mov-r8-from-local (native-value-window-spill-offset frame-base-slot-count 12)))
        reg2 (concat-byte-vectors reg1 (emit-mov-rcx-from-local (native-value-window-spill-offset frame-base-slot-count 13)))
        reg3 (concat-byte-vectors reg2 (emit-mov-rdx-from-local (native-value-window-spill-offset frame-base-slot-count 14)))
        reg4 (concat-byte-vectors reg3 (emit-mov-rsi-from-local (native-value-window-spill-offset frame-base-slot-count 15)))
        reg-setup (concat-byte-vectors reg4 (emit-mov-rdi-from-local (native-value-window-spill-offset frame-base-slot-count 16)))
        call-seq (concat-byte-vectors
                   (emit-call-rel32 rel)
                   (emit-add-rsp-imm32 112))]
    (concat-byte-vectors
      (concat-byte-vectors stack-setup reg-setup)
      call-seq)))

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

(defn emit-drop-bundle-x86 [frame-base-slot-count current-depth]
  (if (>= current-depth 22)
    (concat-byte-vectors
      (emit-drop-bundle-x86 frame-base-slot-count 21)
      (concat-byte-vectors
        (emit-mov-rsi-from-local (native-value-window-spill-offset frame-base-slot-count 19))
        (emit-mov-local-from-rsi (native-value-window-spill-offset frame-base-slot-count 18))))
    (if (>= current-depth 21)
    (concat-byte-vectors
      (emit-drop-bundle-x86 frame-base-slot-count 20)
      (concat-byte-vectors
        (emit-mov-rsi-from-local (native-value-window-spill-offset frame-base-slot-count 18))
        (emit-mov-local-from-rsi (native-value-window-spill-offset frame-base-slot-count 17))))
    (if (>= current-depth 20)
    (concat-byte-vectors
      (emit-drop-bundle-x86 frame-base-slot-count 19)
      (concat-byte-vectors
        (emit-mov-rsi-from-local (native-value-window-spill-offset frame-base-slot-count 17))
        (emit-mov-local-from-rsi (native-value-window-spill-offset frame-base-slot-count 16))))
    (if (>= current-depth 19)
    (concat-byte-vectors
      (emit-drop-bundle-x86 frame-base-slot-count 18)
      (concat-byte-vectors
        (emit-mov-rsi-from-local (native-value-window-spill-offset frame-base-slot-count 16))
        (emit-mov-local-from-rsi (native-value-window-spill-offset frame-base-slot-count 15))))
    (if (>= current-depth 18)
    (concat-byte-vectors
      (emit-drop-bundle-x86 frame-base-slot-count 17)
      (concat-byte-vectors
        (emit-mov-rsi-from-local (native-value-window-spill-offset frame-base-slot-count 15))
        (emit-mov-local-from-rsi (native-value-window-spill-offset frame-base-slot-count 14))))
    (if (>= current-depth 17)
    (concat-byte-vectors
      (emit-drop-bundle-x86 frame-base-slot-count 16)
      (concat-byte-vectors
        (emit-mov-rsi-from-local (native-value-window-spill-offset frame-base-slot-count 14))
        (emit-mov-local-from-rsi (native-value-window-spill-offset frame-base-slot-count 13))))
    (if (>= current-depth 16)
    (concat-byte-vectors
      (emit-drop-bundle-x86 frame-base-slot-count 15)
      (concat-byte-vectors
        (emit-mov-rsi-from-local (native-value-window-spill-offset frame-base-slot-count 13))
        (emit-mov-local-from-rsi (native-value-window-spill-offset frame-base-slot-count 12))))
    (if (>= current-depth 15)
    (concat-byte-vectors
      (emit-drop-bundle-x86 frame-base-slot-count 14)
      (concat-byte-vectors
        (emit-mov-rsi-from-local (native-value-window-spill-offset frame-base-slot-count 12))
        (emit-mov-local-from-rsi (native-value-window-spill-offset frame-base-slot-count 11))))
    (if (>= current-depth 14)
    (concat-byte-vectors
      (emit-drop-bundle-x86 frame-base-slot-count 13)
      (concat-byte-vectors
        (emit-mov-rsi-from-local (native-value-window-spill-offset frame-base-slot-count 11))
        (emit-mov-local-from-rsi (native-value-window-spill-offset frame-base-slot-count 10))))
    (if (>= current-depth 13)
    (concat-byte-vectors
      (emit-drop-bundle-x86 frame-base-slot-count 12)
      (concat-byte-vectors
        (emit-mov-rsi-from-local (native-value-window-spill-offset frame-base-slot-count 10))
        (emit-mov-local-from-rsi (native-value-window-spill-offset frame-base-slot-count 9))))
    (if (>= current-depth 12)
    (concat-byte-vectors
      (concat-byte-vectors
        (concat-byte-vectors
          (concat-byte-vectors
            (concat-byte-vectors
              (concat-byte-vectors
                (concat-byte-vectors
                  (concat-byte-vectors
                    (concat-byte-vectors
                      (concat-byte-vectors
                        (concat-byte-vectors
                          (concat-byte-vectors
                            (concat-byte-vectors
                              (concat-byte-vectors
                                (concat-byte-vectors
                                  (emit-mov-rax-rcx)
                                  (emit-mov-rsi-from-local (native-value-window-spill-offset frame-base-slot-count 1)))
                                (emit-mov-rcx-from-local (native-value-window-spill-offset frame-base-slot-count 0)))
                              (emit-mov-local-from-rsi (native-value-window-spill-offset frame-base-slot-count 0)))
                            (emit-mov-rsi-from-local (native-value-window-spill-offset frame-base-slot-count 2)))
                          (emit-mov-local-from-rsi (native-value-window-spill-offset frame-base-slot-count 1)))
                        (emit-mov-rsi-from-local (native-value-window-spill-offset frame-base-slot-count 3)))
                      (emit-mov-local-from-rsi (native-value-window-spill-offset frame-base-slot-count 2)))
                    (emit-mov-rsi-from-local (native-value-window-spill-offset frame-base-slot-count 4)))
                  (emit-mov-local-from-rsi (native-value-window-spill-offset frame-base-slot-count 3)))
                (emit-mov-rsi-from-local (native-value-window-spill-offset frame-base-slot-count 5)))
              (emit-mov-local-from-rsi (native-value-window-spill-offset frame-base-slot-count 4)))
            (concat-byte-vectors
              (emit-mov-rsi-from-local (native-value-window-spill-offset frame-base-slot-count 6))
              (emit-mov-local-from-rsi (native-value-window-spill-offset frame-base-slot-count 5))))
          (concat-byte-vectors
            (emit-mov-rsi-from-local (native-value-window-spill-offset frame-base-slot-count 7))
            (emit-mov-local-from-rsi (native-value-window-spill-offset frame-base-slot-count 6))))
        (concat-byte-vectors
          (emit-mov-rsi-from-local (native-value-window-spill-offset frame-base-slot-count 8))
          (emit-mov-local-from-rsi (native-value-window-spill-offset frame-base-slot-count 7))))
      (concat-byte-vectors
        (emit-mov-rsi-from-local (native-value-window-spill-offset frame-base-slot-count 9))
        (emit-mov-local-from-rsi (native-value-window-spill-offset frame-base-slot-count 8))))
    (if (>= current-depth 11)
    (concat-byte-vectors
      (concat-byte-vectors
        (concat-byte-vectors
          (concat-byte-vectors
            (concat-byte-vectors
              (concat-byte-vectors
                (concat-byte-vectors
                  (concat-byte-vectors
                    (concat-byte-vectors
                      (concat-byte-vectors
                        (concat-byte-vectors
                          (concat-byte-vectors
                            (concat-byte-vectors
                              (concat-byte-vectors
                                (emit-mov-rax-rcx)
                                (emit-mov-rsi-from-local (native-value-window-spill-offset frame-base-slot-count 1)))
                              (emit-mov-rcx-from-local (native-value-window-spill-offset frame-base-slot-count 0)))
                            (emit-mov-local-from-rsi (native-value-window-spill-offset frame-base-slot-count 0)))
                          (emit-mov-rsi-from-local (native-value-window-spill-offset frame-base-slot-count 2)))
                        (emit-mov-local-from-rsi (native-value-window-spill-offset frame-base-slot-count 1)))
                      (emit-mov-rsi-from-local (native-value-window-spill-offset frame-base-slot-count 3)))
                    (emit-mov-local-from-rsi (native-value-window-spill-offset frame-base-slot-count 2)))
                  (emit-mov-rsi-from-local (native-value-window-spill-offset frame-base-slot-count 4)))
                (emit-mov-local-from-rsi (native-value-window-spill-offset frame-base-slot-count 3)))
              (emit-mov-rsi-from-local (native-value-window-spill-offset frame-base-slot-count 5)))
            (emit-mov-local-from-rsi (native-value-window-spill-offset frame-base-slot-count 4)))
          (concat-byte-vectors
            (emit-mov-rsi-from-local (native-value-window-spill-offset frame-base-slot-count 6))
            (emit-mov-local-from-rsi (native-value-window-spill-offset frame-base-slot-count 5))))
        (concat-byte-vectors
          (emit-mov-rsi-from-local (native-value-window-spill-offset frame-base-slot-count 7))
          (emit-mov-local-from-rsi (native-value-window-spill-offset frame-base-slot-count 6))))
      (concat-byte-vectors
        (emit-mov-rsi-from-local (native-value-window-spill-offset frame-base-slot-count 8))
        (emit-mov-local-from-rsi (native-value-window-spill-offset frame-base-slot-count 7))))
    (if (>= current-depth 10)
    (concat-byte-vectors
      (concat-byte-vectors
        (concat-byte-vectors
          (concat-byte-vectors
            (concat-byte-vectors
              (concat-byte-vectors
                (concat-byte-vectors
                  (concat-byte-vectors
                    (concat-byte-vectors
                      (concat-byte-vectors
                        (concat-byte-vectors
                          (concat-byte-vectors
                            (concat-byte-vectors
                              (emit-mov-rax-rcx)
                              (emit-mov-rsi-from-local (native-value-window-spill-offset frame-base-slot-count 1)))
                            (emit-mov-rcx-from-local (native-value-window-spill-offset frame-base-slot-count 0)))
                          (emit-mov-local-from-rsi (native-value-window-spill-offset frame-base-slot-count 0)))
                        (emit-mov-rsi-from-local (native-value-window-spill-offset frame-base-slot-count 2)))
                      (emit-mov-local-from-rsi (native-value-window-spill-offset frame-base-slot-count 1)))
                    (emit-mov-rsi-from-local (native-value-window-spill-offset frame-base-slot-count 3)))
                  (emit-mov-local-from-rsi (native-value-window-spill-offset frame-base-slot-count 2)))
                (emit-mov-rsi-from-local (native-value-window-spill-offset frame-base-slot-count 4)))
              (emit-mov-local-from-rsi (native-value-window-spill-offset frame-base-slot-count 3)))
            (emit-mov-rsi-from-local (native-value-window-spill-offset frame-base-slot-count 5)))
          (emit-mov-local-from-rsi (native-value-window-spill-offset frame-base-slot-count 4)))
        (concat-byte-vectors
          (emit-mov-rsi-from-local (native-value-window-spill-offset frame-base-slot-count 6))
          (emit-mov-local-from-rsi (native-value-window-spill-offset frame-base-slot-count 5))))
      (concat-byte-vectors
        (emit-mov-rsi-from-local (native-value-window-spill-offset frame-base-slot-count 7))
        (emit-mov-local-from-rsi (native-value-window-spill-offset frame-base-slot-count 6))))
    (if (>= current-depth 9)
    (concat-byte-vectors
      (concat-byte-vectors
        (concat-byte-vectors
          (concat-byte-vectors
            (concat-byte-vectors
              (concat-byte-vectors
                (concat-byte-vectors
                  (concat-byte-vectors
                    (concat-byte-vectors
                      (concat-byte-vectors
                        (concat-byte-vectors
                          (concat-byte-vectors
                            (emit-mov-rax-rcx)
                            (emit-mov-rsi-from-local (native-value-window-spill-offset frame-base-slot-count 1)))
                          (emit-mov-rcx-from-local (native-value-window-spill-offset frame-base-slot-count 0)))
                        (emit-mov-local-from-rsi (native-value-window-spill-offset frame-base-slot-count 0)))
                      (emit-mov-rsi-from-local (native-value-window-spill-offset frame-base-slot-count 2)))
                    (emit-mov-local-from-rsi (native-value-window-spill-offset frame-base-slot-count 1)))
                  (emit-mov-rsi-from-local (native-value-window-spill-offset frame-base-slot-count 3)))
                (emit-mov-local-from-rsi (native-value-window-spill-offset frame-base-slot-count 2)))
              (emit-mov-rsi-from-local (native-value-window-spill-offset frame-base-slot-count 4)))
            (emit-mov-local-from-rsi (native-value-window-spill-offset frame-base-slot-count 3)))
          (emit-mov-rsi-from-local (native-value-window-spill-offset frame-base-slot-count 5)))
        (emit-mov-local-from-rsi (native-value-window-spill-offset frame-base-slot-count 4)))
      (concat-byte-vectors
        (emit-mov-rsi-from-local (native-value-window-spill-offset frame-base-slot-count 6))
        (emit-mov-local-from-rsi (native-value-window-spill-offset frame-base-slot-count 5))))
    (if (>= current-depth 8)
    (concat-byte-vectors
      (concat-byte-vectors
        (concat-byte-vectors
          (concat-byte-vectors
            (concat-byte-vectors
              (concat-byte-vectors
                (concat-byte-vectors
                  (concat-byte-vectors
                    (concat-byte-vectors
                      (concat-byte-vectors
                        (concat-byte-vectors
                          (emit-mov-rax-rcx)
                          (emit-mov-rsi-from-local (native-value-window-spill-offset frame-base-slot-count 1)))
                        (emit-mov-rcx-from-local (native-value-window-spill-offset frame-base-slot-count 0)))
                      (emit-mov-local-from-rsi (native-value-window-spill-offset frame-base-slot-count 0)))
                    (emit-mov-rsi-from-local (native-value-window-spill-offset frame-base-slot-count 2)))
                  (emit-mov-local-from-rsi (native-value-window-spill-offset frame-base-slot-count 1)))
                (emit-mov-rsi-from-local (native-value-window-spill-offset frame-base-slot-count 3)))
              (emit-mov-local-from-rsi (native-value-window-spill-offset frame-base-slot-count 2)))
            (emit-mov-rsi-from-local (native-value-window-spill-offset frame-base-slot-count 4)))
          (emit-mov-local-from-rsi (native-value-window-spill-offset frame-base-slot-count 3)))
        (emit-mov-rsi-from-local (native-value-window-spill-offset frame-base-slot-count 5)))
      (emit-mov-local-from-rsi (native-value-window-spill-offset frame-base-slot-count 4)))
    (if (>= current-depth 7)
    (concat-byte-vectors
      (concat-byte-vectors
        (concat-byte-vectors
          (concat-byte-vectors
            (concat-byte-vectors
              (concat-byte-vectors
                (concat-byte-vectors
                  (concat-byte-vectors
                    (concat-byte-vectors
                      (emit-mov-rax-rcx)
                      (emit-mov-rsi-from-local (native-value-window-spill-offset frame-base-slot-count 1)))
                    (emit-mov-rcx-from-local (native-value-window-spill-offset frame-base-slot-count 0)))
                  (emit-mov-local-from-rsi (native-value-window-spill-offset frame-base-slot-count 0)))
                (emit-mov-rsi-from-local (native-value-window-spill-offset frame-base-slot-count 2)))
              (emit-mov-local-from-rsi (native-value-window-spill-offset frame-base-slot-count 1)))
            (emit-mov-rsi-from-local (native-value-window-spill-offset frame-base-slot-count 3)))
          (emit-mov-local-from-rsi (native-value-window-spill-offset frame-base-slot-count 2)))
        (emit-mov-rsi-from-local (native-value-window-spill-offset frame-base-slot-count 4)))
      (emit-mov-local-from-rsi (native-value-window-spill-offset frame-base-slot-count 3)))
    (if (>= current-depth 6)
    (concat-byte-vectors
      (concat-byte-vectors
        (concat-byte-vectors
          (concat-byte-vectors
            (concat-byte-vectors
              (concat-byte-vectors
                (concat-byte-vectors
                  (emit-mov-rax-rcx)
                  (emit-mov-rsi-from-local (native-value-window-spill-offset frame-base-slot-count 1)))
                (emit-mov-rcx-from-local (native-value-window-spill-offset frame-base-slot-count 0)))
              (emit-mov-local-from-rsi (native-value-window-spill-offset frame-base-slot-count 0)))
            (emit-mov-rsi-from-local (native-value-window-spill-offset frame-base-slot-count 2)))
          (emit-mov-local-from-rsi (native-value-window-spill-offset frame-base-slot-count 1)))
        (emit-mov-rsi-from-local (native-value-window-spill-offset frame-base-slot-count 3)))
      (emit-mov-local-from-rsi (native-value-window-spill-offset frame-base-slot-count 2)))
    (if (>= current-depth 5)
    (concat-byte-vectors
      (concat-byte-vectors
        (concat-byte-vectors
          (concat-byte-vectors
            (concat-byte-vectors
              (emit-mov-rax-rcx)
              (emit-mov-rdx-from-local (native-value-window-spill-offset frame-base-slot-count 2)))
            (emit-mov-rcx-from-local (native-value-window-spill-offset frame-base-slot-count 0)))
          (emit-mov-rsi-from-local (native-value-window-spill-offset frame-base-slot-count 1)))
        (emit-mov-local-from-rsi (native-value-window-spill-offset frame-base-slot-count 0)))
      (emit-mov-local-from-rdx (native-value-window-spill-offset frame-base-slot-count 1)))
      (if (>= current-depth 4)
        (concat-byte-vectors
          (concat-byte-vectors
            (concat-byte-vectors
            (emit-mov-rax-rcx)
            (emit-mov-rdx-from-local (native-value-window-spill-offset frame-base-slot-count 1)))
          (emit-mov-rcx-from-local (native-value-window-spill-offset frame-base-slot-count 0)))
        (emit-mov-local-from-rdx (native-value-window-spill-offset frame-base-slot-count 0)))
        (if (>= current-depth 3)
          (concat-byte-vectors
            (emit-mov-rax-rcx)
            (emit-mov-rcx-from-local (native-value-window-spill-offset frame-base-slot-count 0)))
        (emit-mov-rax-rcx))))))))))))))))))))))

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
      target-param-count (native-function-param-count target-meta)
      size (if (>= target-param-count 20)
             (if (= target-param-count 26)
               297
               (if (= target-param-count 25)
                 282
                 (if (= target-param-count 24)
                   267
                   (if (= target-param-count 23)
                     253
                     (if (= target-param-count 22)
                       238
                       (if (= target-param-count 21)
                         226
                         214))))))
             (if (> target-param-count 8)
               (+ 82 (* (- target-param-count 9) 12))
               (if (= target-param-count 8)
                70
                (if (= target-param-count 7)
                  61
                 (if (= target-param-count 6)
                   39
                   (if (= target-param-count 5)
                     29
                     (if (= target-param-count 4)
                       25
                       (if (= target-param-count 3)
                         18
                         (if (= target-param-count 2)
                           (if (>= current-depth 3) 18 11)
                            (if (= target-param-count 1)
                              10
                              5))))))))))]
      size)
    (if (= opcode 3)
      (if (>= current-depth 2) (+ 15 (* (- current-depth 2) 14)) 8)
      (if (= opcode 10)
        (if (>= current-depth 2) (+ 17 (* (- current-depth 2) 14)) 10)
        (if (= opcode 44)
          (if (>= current-depth 3) (+ 10 (* (- current-depth 3) 14)) 3)
          (vector-length (codegen-ir-instr opcode operand)))))))

(defn native-function-body-size-x86-loop [ir-func function-metas idx len total current-depth]
  (if (>= idx len)
    total
    (let [instr (vector-get ir-func idx)
      opcode (vector-get instr 0)
      operand (vector-get instr 1)
      next-total (+ total (native-instr-size-x86 opcode operand function-metas current-depth))
      next-depth (apply-stack-delta current-depth (opcode-stack-delta opcode operand function-metas))]
      (native-function-body-size-x86-loop ir-func function-metas (+ idx 1) len next-total next-depth))))

(defn native-param-spill-bytes-x86-twenty-to-twenty-two [param-count]
  (if (= param-count 22)
    224
    (if (= param-count 21)
      210
      (+ 53 (* (- param-count 7) 11)))))

(defn native-param-spill-bytes-x86-twenty-to-twenty-three [param-count]
  (if (= param-count 23)
    238
    (native-param-spill-bytes-x86-twenty-to-twenty-two param-count)))

(defn native-param-spill-bytes-x86-twenty-to-twenty-four [param-count]
  (if (= param-count 24)
    252
    (native-param-spill-bytes-x86-twenty-to-twenty-three param-count)))

(defn native-param-spill-bytes-x86-twenty-to-twenty-five [param-count]
  (if (= param-count 25)
    266
    (native-param-spill-bytes-x86-twenty-to-twenty-four param-count)))

(defn native-param-spill-bytes-x86-twenty-to-twenty-six [param-count]
  (if (= param-count 26)
    280
    (native-param-spill-bytes-x86-twenty-to-twenty-five param-count)))

(defn native-param-spill-bytes-x86-twenty-to-twenty-seven [param-count]
  (if (= param-count 27)
    294
    (native-param-spill-bytes-x86-twenty-to-twenty-six param-count)))

(defn native-param-spill-bytes-x86-twenty-to-twenty-eight [param-count]
  (if (= param-count 28)
    308
    (native-param-spill-bytes-x86-twenty-to-twenty-seven param-count)))

(defn native-function-size-x86 [func-meta function-metas]
  (let [param-count (native-function-param-count func-meta)
    local-count (native-function-local-count func-meta)
    ir-func (native-function-ir func-meta)
    stack-bytes (native-local-stack-bytes-with-window ir-func (+ param-count local-count) function-metas)
    frame-bytes (if (> stack-bytes 0) 14 0)
    param-spill-bytes (if (>= param-count 20)
                        (native-param-spill-bytes-x86-twenty-to-twenty-eight param-count)
                        (if (> param-count 6)
                          (+ 53 (* (- param-count 7) 11))
                          (if (= param-count 6)
                            42
                            (if (= param-count 5)
                              35
                              (if (= param-count 4)
                                28
                                (if (= param-count 3)
                                  21
                                   (if (= param-count 2)
                                      14
                                       (if (= param-count 1) 7 0))))))))
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

(defn emit-call-bundle-x86-twenty-to-twenty-two [target-param-count rel frame-base-slot-count]
  (if (= target-param-count 22)
    (emit-twenty-two-arg-call-x86 rel frame-base-slot-count)
    (if (= target-param-count 21)
      (emit-twenty-one-arg-call-x86 rel frame-base-slot-count)
      (emit-twenty-arg-call-x86 rel frame-base-slot-count))))

(defn emit-call-bundle-x86-twenty-to-twenty-three [target-param-count rel frame-base-slot-count]
  (if (= target-param-count 23)
    (emit-twenty-three-arg-call-x86 rel frame-base-slot-count)
    (emit-call-bundle-x86-twenty-to-twenty-two target-param-count rel frame-base-slot-count)))

(defn emit-call-bundle-x86-twenty-to-twenty-four [target-param-count rel frame-base-slot-count]
  (if (= target-param-count 24)
    (emit-twenty-four-arg-call-x86 rel frame-base-slot-count)
    (emit-call-bundle-x86-twenty-to-twenty-three target-param-count rel frame-base-slot-count)))

(defn emit-call-bundle-x86-twenty-to-twenty-five [target-param-count rel frame-base-slot-count]
  (if (= target-param-count 25)
    (emit-twenty-five-arg-call-x86 rel frame-base-slot-count)
    (emit-call-bundle-x86-twenty-to-twenty-four target-param-count rel frame-base-slot-count)))

(defn emit-call-bundle-x86-twenty-to-twenty-six [target-param-count rel frame-base-slot-count]
  (if (= target-param-count 26)
    (emit-twenty-six-arg-call-x86 rel frame-base-slot-count)
    (emit-call-bundle-x86-twenty-to-twenty-five target-param-count rel frame-base-slot-count)))

(defn emit-call-bundle-x86-twenty-to-twenty-seven [target-param-count rel frame-base-slot-count]
  (if (= target-param-count 27)
    (emit-twenty-seven-arg-call-x86 rel frame-base-slot-count)
    (emit-call-bundle-x86-twenty-to-twenty-six target-param-count rel frame-base-slot-count)))

(defn emit-call-bundle-x86-twenty-to-twenty-eight [target-param-count rel frame-base-slot-count]
  (if (= target-param-count 28)
    (emit-twenty-eight-arg-call-x86 rel frame-base-slot-count)
    (emit-call-bundle-x86-twenty-to-twenty-seven target-param-count rel frame-base-slot-count)))

(defn emit-call-bundle-x86-ten-to-nineteen [target-param-count rel frame-base-slot-count]
  (if (= target-param-count 19)
    (emit-nineteen-arg-call-x86 rel frame-base-slot-count)
    (if (= target-param-count 18)
      (emit-eighteen-arg-call-x86 rel frame-base-slot-count)
      (if (= target-param-count 17)
        (emit-seventeen-arg-call-x86 rel frame-base-slot-count)
        (if (= target-param-count 16)
          (emit-sixteen-arg-call-x86 rel frame-base-slot-count)
          (if (= target-param-count 15)
            (emit-fifteen-arg-call-x86 rel frame-base-slot-count)
            (if (= target-param-count 14)
              (emit-fourteen-arg-call-x86 rel frame-base-slot-count)
              (if (= target-param-count 13)
                (emit-thirteen-arg-call-x86 rel frame-base-slot-count)
                (if (= target-param-count 12)
                  (emit-twelve-arg-call-x86 rel frame-base-slot-count)
                  (if (= target-param-count 11)
                    (emit-eleven-arg-call-x86 rel frame-base-slot-count)
                    (emit-ten-arg-call-x86 rel frame-base-slot-count)))))))))))

(defn emit-call-bundle-x86-one-to-nine [target-param-count rel frame-base-slot-count current-depth]
  (if (= target-param-count 9)
    (emit-nine-arg-call-x86 rel frame-base-slot-count)
    (if (= target-param-count 8)
      (emit-eight-arg-call-x86 rel frame-base-slot-count)
      (if (= target-param-count 7)
        (emit-seven-arg-call-x86 rel frame-base-slot-count)
        (if (= target-param-count 6)
          (emit-six-arg-call-x86 rel frame-base-slot-count)
          (if (= target-param-count 5)
            (emit-five-arg-call-x86 rel frame-base-slot-count)
            (if (= target-param-count 4)
              (emit-four-arg-call-x86 rel frame-base-slot-count)
              (if (= target-param-count 3)
                (emit-three-arg-call-x86 rel frame-base-slot-count)
                (if (= target-param-count 2)
                  (emit-two-arg-call-x86 rel frame-base-slot-count current-depth)
                  (if (= target-param-count 1)
                    (let [call-rel (emit-call-rel32 rel)
                      push-rcx (emit-push-rcx)
                      pop-rcx (emit-pop-rcx)
                      bytes (vector-new 10)
                      b1 (vector-push bytes 72)
                      b2 (vector-push b1 137)
                      b3 (vector-push b2 199)
                      b4 (vector-push b3 (vector-get push-rcx 0))
                      b5 (vector-push b4 (vector-get call-rel 0))
                      b6 (vector-push b5 (vector-get call-rel 1))
                      b7 (vector-push b6 (vector-get call-rel 2))
                      b8 (vector-push b7 (vector-get call-rel 3))
                      b9 (vector-push b8 (vector-get call-rel 4))
                      b10 (vector-push b9 (vector-get pop-rcx 0))]
                      b10)
                    (emit-call-rel32 rel)))))))))))

(defn codegen-ir-instr-bundle-x86 [opcode operand current-offset function-starts function-metas frame-base-slot-count current-depth]
  (if (= opcode 40)
    (let [target-offset (vector-get function-starts operand)
      target-meta (vector-get function-metas operand)
      target-param-count (native-function-param-count target-meta)
      rel (if (>= target-param-count 20)
             (- target-offset
               (if (= target-param-count 28)
                  (+ current-offset 320)
                  (if (= target-param-count 27)
                  (+ current-offset 305)
                  (if (= target-param-count 26)
                    (+ current-offset 290)
                   (if (= target-param-count 25)
                     (+ current-offset 275)
                    (if (= target-param-count 24)
                      (+ current-offset 260)
                      (if (= target-param-count 23)
                        (+ current-offset 246)
                        (if (= target-param-count 22)
                          (+ current-offset 231)
                          (if (= target-param-count 21)
                            (+ current-offset 219)
                            (+ current-offset 207))))))))))
               (if (> target-param-count 8)
                 (- target-offset (+ current-offset (+ 75 (* (- target-param-count 9) 12))))
                (if (= target-param-count 8)
                (- target-offset (+ current-offset 63))
                (if (= target-param-count 7)
                  (- target-offset (+ current-offset 54))
                  (if (= target-param-count 6)
                    (- target-offset (+ current-offset 39))
                    (if (= target-param-count 5)
                      (- target-offset (+ current-offset 29))
                      (if (= target-param-count 4)
                        (- target-offset (+ current-offset 25))
                        (if (= target-param-count 3)
                          (- target-offset (+ current-offset 18))
                          (if (= target-param-count 2)
                            (- target-offset (+ current-offset 11))
                            (if (= target-param-count 1)
                              (- target-offset (+ current-offset 9))
                              (- target-offset (+ current-offset 5))))))))))))
      call-bytes (if (>= target-param-count 20)
                    (emit-call-bundle-x86-twenty-to-twenty-eight target-param-count rel frame-base-slot-count)
                     (if (>= target-param-count 10)
                       (emit-call-bundle-x86-ten-to-nineteen target-param-count rel frame-base-slot-count)
                       (emit-call-bundle-x86-one-to-nine target-param-count rel frame-base-slot-count current-depth)))]
      call-bytes)
    (if (= opcode 3)
      (emit-i32-const-bundle-x86 operand frame-base-slot-count current-depth)
      (if (= opcode 10)
        (emit-local-get-bundle-x86 (local-slot-offset operand) frame-base-slot-count current-depth)
        (if (= opcode 44)
          (emit-drop-bundle-x86 frame-base-slot-count current-depth)
          (codegen-ir-instr opcode operand))))))

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

(defn spill-native-function-params-x86-twenty-to-twenty-two [param-count result]
  (if (= param-count 22)
    (do
      (append-native-bytes-loop result (emit-mov-local-from-rdi (local-slot-offset 0)) 0 7)
      (append-native-bytes-loop result (emit-mov-local-from-rsi (local-slot-offset 1)) 0 7)
      (append-native-bytes-loop result (emit-mov-local-from-rdx (local-slot-offset 2)) 0 7)
      (append-native-bytes-loop result (emit-mov-local-from-rcx (local-slot-offset 3)) 0 7)
      (append-native-bytes-loop result (emit-mov-local-from-r8 (local-slot-offset 4)) 0 7)
      (append-native-bytes-loop result (emit-mov-local-from-r9 (local-slot-offset 5)) 0 7)
      (append-native-bytes-loop result (emit-mov-rax-from-rbp-plus-imm8 16) 0 4)
      (append-native-bytes-loop result (emit-mov-local-from-rax (local-slot-offset 6)) 0 7)
      (append-native-bytes-loop result (emit-mov-rax-from-rbp-plus-imm8 24) 0 4)
      (append-native-bytes-loop result (emit-mov-local-from-rax (local-slot-offset 7)) 0 7)
      (append-native-bytes-loop result (emit-mov-rax-from-rbp-plus-imm8 32) 0 4)
      (append-native-bytes-loop result (emit-mov-local-from-rax (local-slot-offset 8)) 0 7)
      (append-native-bytes-loop result (emit-mov-rax-from-rbp-plus-imm8 40) 0 4)
      (append-native-bytes-loop result (emit-mov-local-from-rax (local-slot-offset 9)) 0 7)
      (append-native-bytes-loop result (emit-mov-rax-from-rbp-plus-imm8 48) 0 4)
      (append-native-bytes-loop result (emit-mov-local-from-rax (local-slot-offset 10)) 0 7)
      (append-native-bytes-loop result (emit-mov-rax-from-rbp-plus-imm8 56) 0 4)
      (append-native-bytes-loop result (emit-mov-local-from-rax (local-slot-offset 11)) 0 7)
      (append-native-bytes-loop result (emit-mov-rax-from-rbp-plus-imm8 64) 0 4)
      (append-native-bytes-loop result (emit-mov-local-from-rax (local-slot-offset 12)) 0 7)
      (append-native-bytes-loop result (emit-mov-rax-from-rbp-plus-imm8 72) 0 4)
      (append-native-bytes-loop result (emit-mov-local-from-rax (local-slot-offset 13)) 0 7)
      (append-native-bytes-loop result (emit-mov-rax-from-rbp-plus-imm8 80) 0 4)
      (append-native-bytes-loop result (emit-mov-local-from-rax (local-slot-offset 14)) 0 7)
      (append-native-bytes-loop result (emit-mov-rax-from-rbp-plus-imm8 88) 0 4)
      (append-native-bytes-loop result (emit-mov-local-from-rax (local-slot-offset 15)) 0 7)
      (append-native-bytes-loop result (emit-mov-rax-from-rbp-plus-imm8 96) 0 4)
      (append-native-bytes-loop result (emit-mov-local-from-rax (local-slot-offset 16)) 0 7)
      (append-native-bytes-loop result (emit-mov-rax-from-rbp-plus-imm8 104) 0 4)
      (append-native-bytes-loop result (emit-mov-local-from-rax (local-slot-offset 17)) 0 7)
      (append-native-bytes-loop result (emit-mov-rax-from-rbp-plus-imm8 112) 0 4)
      (append-native-bytes-loop result (emit-mov-local-from-rax (local-slot-offset 18)) 0 7)
      (append-native-bytes-loop result (emit-mov-rax-from-rbp-plus-imm8 120) 0 4)
      (append-native-bytes-loop result (emit-mov-local-from-rax (local-slot-offset 19)) 0 7)
      (append-native-bytes-loop result (emit-mov-rax-from-rbp-plus-imm32 128) 0 7)
      (append-native-bytes-loop result (emit-mov-local-from-rax (local-slot-offset 20)) 0 7)
      (append-native-bytes-loop result (emit-mov-rax-from-rbp-plus-imm32 136) 0 7)
      (append-native-bytes-loop result (emit-mov-local-from-rax (local-slot-offset 21)) 0 7))
    (if (= param-count 21)
      (do
        (append-native-bytes-loop result (emit-mov-local-from-rdi (local-slot-offset 0)) 0 7)
        (append-native-bytes-loop result (emit-mov-local-from-rsi (local-slot-offset 1)) 0 7)
        (append-native-bytes-loop result (emit-mov-local-from-rdx (local-slot-offset 2)) 0 7)
        (append-native-bytes-loop result (emit-mov-local-from-rcx (local-slot-offset 3)) 0 7)
        (append-native-bytes-loop result (emit-mov-local-from-r8 (local-slot-offset 4)) 0 7)
        (append-native-bytes-loop result (emit-mov-local-from-r9 (local-slot-offset 5)) 0 7)
        (append-native-bytes-loop result (emit-mov-rax-from-rbp-plus-imm8 16) 0 4)
        (append-native-bytes-loop result (emit-mov-local-from-rax (local-slot-offset 6)) 0 7)
        (append-native-bytes-loop result (emit-mov-rax-from-rbp-plus-imm8 24) 0 4)
        (append-native-bytes-loop result (emit-mov-local-from-rax (local-slot-offset 7)) 0 7)
        (append-native-bytes-loop result (emit-mov-rax-from-rbp-plus-imm8 32) 0 4)
        (append-native-bytes-loop result (emit-mov-local-from-rax (local-slot-offset 8)) 0 7)
        (append-native-bytes-loop result (emit-mov-rax-from-rbp-plus-imm8 40) 0 4)
        (append-native-bytes-loop result (emit-mov-local-from-rax (local-slot-offset 9)) 0 7)
        (append-native-bytes-loop result (emit-mov-rax-from-rbp-plus-imm8 48) 0 4)
        (append-native-bytes-loop result (emit-mov-local-from-rax (local-slot-offset 10)) 0 7)
        (append-native-bytes-loop result (emit-mov-rax-from-rbp-plus-imm8 56) 0 4)
        (append-native-bytes-loop result (emit-mov-local-from-rax (local-slot-offset 11)) 0 7)
        (append-native-bytes-loop result (emit-mov-rax-from-rbp-plus-imm8 64) 0 4)
        (append-native-bytes-loop result (emit-mov-local-from-rax (local-slot-offset 12)) 0 7)
        (append-native-bytes-loop result (emit-mov-rax-from-rbp-plus-imm8 72) 0 4)
        (append-native-bytes-loop result (emit-mov-local-from-rax (local-slot-offset 13)) 0 7)
        (append-native-bytes-loop result (emit-mov-rax-from-rbp-plus-imm8 80) 0 4)
        (append-native-bytes-loop result (emit-mov-local-from-rax (local-slot-offset 14)) 0 7)
        (append-native-bytes-loop result (emit-mov-rax-from-rbp-plus-imm8 88) 0 4)
        (append-native-bytes-loop result (emit-mov-local-from-rax (local-slot-offset 15)) 0 7)
        (append-native-bytes-loop result (emit-mov-rax-from-rbp-plus-imm8 96) 0 4)
        (append-native-bytes-loop result (emit-mov-local-from-rax (local-slot-offset 16)) 0 7)
        (append-native-bytes-loop result (emit-mov-rax-from-rbp-plus-imm8 104) 0 4)
        (append-native-bytes-loop result (emit-mov-local-from-rax (local-slot-offset 17)) 0 7)
        (append-native-bytes-loop result (emit-mov-rax-from-rbp-plus-imm8 112) 0 4)
        (append-native-bytes-loop result (emit-mov-local-from-rax (local-slot-offset 18)) 0 7)
        (append-native-bytes-loop result (emit-mov-rax-from-rbp-plus-imm8 120) 0 4)
        (append-native-bytes-loop result (emit-mov-local-from-rax (local-slot-offset 19)) 0 7)
        (append-native-bytes-loop result (emit-mov-rax-from-rbp-plus-imm32 128) 0 7)
        (append-native-bytes-loop result (emit-mov-local-from-rax (local-slot-offset 20)) 0 7))
      (do
        (append-native-bytes-loop result (emit-mov-local-from-rdi (local-slot-offset 0)) 0 7)
        (append-native-bytes-loop result (emit-mov-local-from-rsi (local-slot-offset 1)) 0 7)
        (append-native-bytes-loop result (emit-mov-local-from-rdx (local-slot-offset 2)) 0 7)
        (append-native-bytes-loop result (emit-mov-local-from-rcx (local-slot-offset 3)) 0 7)
        (append-native-bytes-loop result (emit-mov-local-from-r8 (local-slot-offset 4)) 0 7)
        (append-native-bytes-loop result (emit-mov-local-from-r9 (local-slot-offset 5)) 0 7)
        (append-native-bytes-loop result (emit-mov-rax-from-rbp-plus-imm8 16) 0 4)
        (append-native-bytes-loop result (emit-mov-local-from-rax (local-slot-offset 6)) 0 7)
        (append-native-bytes-loop result (emit-mov-rax-from-rbp-plus-imm8 24) 0 4)
        (append-native-bytes-loop result (emit-mov-local-from-rax (local-slot-offset 7)) 0 7)
        (append-native-bytes-loop result (emit-mov-rax-from-rbp-plus-imm8 32) 0 4)
        (append-native-bytes-loop result (emit-mov-local-from-rax (local-slot-offset 8)) 0 7)
        (append-native-bytes-loop result (emit-mov-rax-from-rbp-plus-imm8 40) 0 4)
        (append-native-bytes-loop result (emit-mov-local-from-rax (local-slot-offset 9)) 0 7)
        (append-native-bytes-loop result (emit-mov-rax-from-rbp-plus-imm8 48) 0 4)
        (append-native-bytes-loop result (emit-mov-local-from-rax (local-slot-offset 10)) 0 7)
        (append-native-bytes-loop result (emit-mov-rax-from-rbp-plus-imm8 56) 0 4)
        (append-native-bytes-loop result (emit-mov-local-from-rax (local-slot-offset 11)) 0 7)
        (append-native-bytes-loop result (emit-mov-rax-from-rbp-plus-imm8 64) 0 4)
        (append-native-bytes-loop result (emit-mov-local-from-rax (local-slot-offset 12)) 0 7)
        (append-native-bytes-loop result (emit-mov-rax-from-rbp-plus-imm8 72) 0 4)
        (append-native-bytes-loop result (emit-mov-local-from-rax (local-slot-offset 13)) 0 7)
        (append-native-bytes-loop result (emit-mov-rax-from-rbp-plus-imm8 80) 0 4)
        (append-native-bytes-loop result (emit-mov-local-from-rax (local-slot-offset 14)) 0 7)
        (append-native-bytes-loop result (emit-mov-rax-from-rbp-plus-imm8 88) 0 4)
        (append-native-bytes-loop result (emit-mov-local-from-rax (local-slot-offset 15)) 0 7)
        (append-native-bytes-loop result (emit-mov-rax-from-rbp-plus-imm8 96) 0 4)
        (append-native-bytes-loop result (emit-mov-local-from-rax (local-slot-offset 16)) 0 7)
        (append-native-bytes-loop result (emit-mov-rax-from-rbp-plus-imm8 104) 0 4)
        (append-native-bytes-loop result (emit-mov-local-from-rax (local-slot-offset 17)) 0 7)
        (append-native-bytes-loop result (emit-mov-rax-from-rbp-plus-imm8 112) 0 4)
        (append-native-bytes-loop result (emit-mov-local-from-rax (local-slot-offset 18)) 0 7)
        (append-native-bytes-loop result (emit-mov-rax-from-rbp-plus-imm8 120) 0 4)
        (append-native-bytes-loop result (emit-mov-local-from-rax (local-slot-offset 19)) 0 7)))))

(defn spill-native-function-params-x86-twenty-to-twenty-three [param-count result]
  (if (= param-count 23)
    (do
      (append-native-bytes-loop result (emit-mov-local-from-rdi (local-slot-offset 0)) 0 7)
      (append-native-bytes-loop result (emit-mov-local-from-rsi (local-slot-offset 1)) 0 7)
      (append-native-bytes-loop result (emit-mov-local-from-rdx (local-slot-offset 2)) 0 7)
      (append-native-bytes-loop result (emit-mov-local-from-rcx (local-slot-offset 3)) 0 7)
      (append-native-bytes-loop result (emit-mov-local-from-r8 (local-slot-offset 4)) 0 7)
      (append-native-bytes-loop result (emit-mov-local-from-r9 (local-slot-offset 5)) 0 7)
      (append-native-bytes-loop result (emit-mov-rax-from-rbp-plus-imm8 16) 0 4)
      (append-native-bytes-loop result (emit-mov-local-from-rax (local-slot-offset 6)) 0 7)
      (append-native-bytes-loop result (emit-mov-rax-from-rbp-plus-imm8 24) 0 4)
      (append-native-bytes-loop result (emit-mov-local-from-rax (local-slot-offset 7)) 0 7)
      (append-native-bytes-loop result (emit-mov-rax-from-rbp-plus-imm8 32) 0 4)
      (append-native-bytes-loop result (emit-mov-local-from-rax (local-slot-offset 8)) 0 7)
      (append-native-bytes-loop result (emit-mov-rax-from-rbp-plus-imm8 40) 0 4)
      (append-native-bytes-loop result (emit-mov-local-from-rax (local-slot-offset 9)) 0 7)
      (append-native-bytes-loop result (emit-mov-rax-from-rbp-plus-imm8 48) 0 4)
      (append-native-bytes-loop result (emit-mov-local-from-rax (local-slot-offset 10)) 0 7)
      (append-native-bytes-loop result (emit-mov-rax-from-rbp-plus-imm8 56) 0 4)
      (append-native-bytes-loop result (emit-mov-local-from-rax (local-slot-offset 11)) 0 7)
      (append-native-bytes-loop result (emit-mov-rax-from-rbp-plus-imm8 64) 0 4)
      (append-native-bytes-loop result (emit-mov-local-from-rax (local-slot-offset 12)) 0 7)
      (append-native-bytes-loop result (emit-mov-rax-from-rbp-plus-imm8 72) 0 4)
      (append-native-bytes-loop result (emit-mov-local-from-rax (local-slot-offset 13)) 0 7)
      (append-native-bytes-loop result (emit-mov-rax-from-rbp-plus-imm8 80) 0 4)
      (append-native-bytes-loop result (emit-mov-local-from-rax (local-slot-offset 14)) 0 7)
      (append-native-bytes-loop result (emit-mov-rax-from-rbp-plus-imm8 88) 0 4)
      (append-native-bytes-loop result (emit-mov-local-from-rax (local-slot-offset 15)) 0 7)
      (append-native-bytes-loop result (emit-mov-rax-from-rbp-plus-imm8 96) 0 4)
      (append-native-bytes-loop result (emit-mov-local-from-rax (local-slot-offset 16)) 0 7)
      (append-native-bytes-loop result (emit-mov-rax-from-rbp-plus-imm8 104) 0 4)
      (append-native-bytes-loop result (emit-mov-local-from-rax (local-slot-offset 17)) 0 7)
      (append-native-bytes-loop result (emit-mov-rax-from-rbp-plus-imm8 112) 0 4)
      (append-native-bytes-loop result (emit-mov-local-from-rax (local-slot-offset 18)) 0 7)
      (append-native-bytes-loop result (emit-mov-rax-from-rbp-plus-imm8 120) 0 4)
      (append-native-bytes-loop result (emit-mov-local-from-rax (local-slot-offset 19)) 0 7)
      (append-native-bytes-loop result (emit-mov-rax-from-rbp-plus-imm32 128) 0 7)
      (append-native-bytes-loop result (emit-mov-local-from-rax (local-slot-offset 20)) 0 7)
      (append-native-bytes-loop result (emit-mov-rax-from-rbp-plus-imm32 136) 0 7)
      (append-native-bytes-loop result (emit-mov-local-from-rax (local-slot-offset 21)) 0 7)
      (append-native-bytes-loop result (emit-mov-rax-from-rbp-plus-imm32 144) 0 7)
       (append-native-bytes-loop result (emit-mov-local-from-rax (local-slot-offset 22)) 0 7))
    (spill-native-function-params-x86-twenty-to-twenty-two param-count result)))

(defn spill-native-function-params-x86-twenty-to-twenty-four [param-count result]
  (if (= param-count 24)
    (do
      (append-native-bytes-loop result (emit-mov-local-from-rdi (local-slot-offset 0)) 0 7)
      (append-native-bytes-loop result (emit-mov-local-from-rsi (local-slot-offset 1)) 0 7)
      (append-native-bytes-loop result (emit-mov-local-from-rdx (local-slot-offset 2)) 0 7)
      (append-native-bytes-loop result (emit-mov-local-from-rcx (local-slot-offset 3)) 0 7)
      (append-native-bytes-loop result (emit-mov-local-from-r8 (local-slot-offset 4)) 0 7)
      (append-native-bytes-loop result (emit-mov-local-from-r9 (local-slot-offset 5)) 0 7)
      (append-native-bytes-loop result (emit-mov-rax-from-rbp-plus-imm8 16) 0 4)
      (append-native-bytes-loop result (emit-mov-local-from-rax (local-slot-offset 6)) 0 7)
      (append-native-bytes-loop result (emit-mov-rax-from-rbp-plus-imm8 24) 0 4)
      (append-native-bytes-loop result (emit-mov-local-from-rax (local-slot-offset 7)) 0 7)
      (append-native-bytes-loop result (emit-mov-rax-from-rbp-plus-imm8 32) 0 4)
      (append-native-bytes-loop result (emit-mov-local-from-rax (local-slot-offset 8)) 0 7)
      (append-native-bytes-loop result (emit-mov-rax-from-rbp-plus-imm8 40) 0 4)
      (append-native-bytes-loop result (emit-mov-local-from-rax (local-slot-offset 9)) 0 7)
      (append-native-bytes-loop result (emit-mov-rax-from-rbp-plus-imm8 48) 0 4)
      (append-native-bytes-loop result (emit-mov-local-from-rax (local-slot-offset 10)) 0 7)
      (append-native-bytes-loop result (emit-mov-rax-from-rbp-plus-imm8 56) 0 4)
      (append-native-bytes-loop result (emit-mov-local-from-rax (local-slot-offset 11)) 0 7)
      (append-native-bytes-loop result (emit-mov-rax-from-rbp-plus-imm8 64) 0 4)
      (append-native-bytes-loop result (emit-mov-local-from-rax (local-slot-offset 12)) 0 7)
      (append-native-bytes-loop result (emit-mov-rax-from-rbp-plus-imm8 72) 0 4)
      (append-native-bytes-loop result (emit-mov-local-from-rax (local-slot-offset 13)) 0 7)
      (append-native-bytes-loop result (emit-mov-rax-from-rbp-plus-imm8 80) 0 4)
      (append-native-bytes-loop result (emit-mov-local-from-rax (local-slot-offset 14)) 0 7)
      (append-native-bytes-loop result (emit-mov-rax-from-rbp-plus-imm8 88) 0 4)
      (append-native-bytes-loop result (emit-mov-local-from-rax (local-slot-offset 15)) 0 7)
      (append-native-bytes-loop result (emit-mov-rax-from-rbp-plus-imm8 96) 0 4)
      (append-native-bytes-loop result (emit-mov-local-from-rax (local-slot-offset 16)) 0 7)
      (append-native-bytes-loop result (emit-mov-rax-from-rbp-plus-imm8 104) 0 4)
      (append-native-bytes-loop result (emit-mov-local-from-rax (local-slot-offset 17)) 0 7)
      (append-native-bytes-loop result (emit-mov-rax-from-rbp-plus-imm8 112) 0 4)
      (append-native-bytes-loop result (emit-mov-local-from-rax (local-slot-offset 18)) 0 7)
      (append-native-bytes-loop result (emit-mov-rax-from-rbp-plus-imm8 120) 0 4)
      (append-native-bytes-loop result (emit-mov-local-from-rax (local-slot-offset 19)) 0 7)
      (append-native-bytes-loop result (emit-mov-rax-from-rbp-plus-imm32 128) 0 7)
      (append-native-bytes-loop result (emit-mov-local-from-rax (local-slot-offset 20)) 0 7)
      (append-native-bytes-loop result (emit-mov-rax-from-rbp-plus-imm32 136) 0 7)
      (append-native-bytes-loop result (emit-mov-local-from-rax (local-slot-offset 21)) 0 7)
      (append-native-bytes-loop result (emit-mov-rax-from-rbp-plus-imm32 144) 0 7)
      (append-native-bytes-loop result (emit-mov-local-from-rax (local-slot-offset 22)) 0 7)
      (append-native-bytes-loop result (emit-mov-rax-from-rbp-plus-imm32 152) 0 7)
       (append-native-bytes-loop result (emit-mov-local-from-rax (local-slot-offset 23)) 0 7))
    (spill-native-function-params-x86-twenty-to-twenty-three param-count result)))

(defn spill-native-function-params-x86-twenty-to-twenty-five [param-count result]
  (if (= param-count 25)
    (do
      (append-native-bytes-loop result (emit-mov-local-from-rdi (local-slot-offset 0)) 0 7)
      (append-native-bytes-loop result (emit-mov-local-from-rsi (local-slot-offset 1)) 0 7)
      (append-native-bytes-loop result (emit-mov-local-from-rdx (local-slot-offset 2)) 0 7)
      (append-native-bytes-loop result (emit-mov-local-from-rcx (local-slot-offset 3)) 0 7)
      (append-native-bytes-loop result (emit-mov-local-from-r8 (local-slot-offset 4)) 0 7)
      (append-native-bytes-loop result (emit-mov-local-from-r9 (local-slot-offset 5)) 0 7)
      (append-native-bytes-loop result (emit-mov-rax-from-rbp-plus-imm8 16) 0 4)
      (append-native-bytes-loop result (emit-mov-local-from-rax (local-slot-offset 6)) 0 7)
      (append-native-bytes-loop result (emit-mov-rax-from-rbp-plus-imm8 24) 0 4)
      (append-native-bytes-loop result (emit-mov-local-from-rax (local-slot-offset 7)) 0 7)
      (append-native-bytes-loop result (emit-mov-rax-from-rbp-plus-imm8 32) 0 4)
      (append-native-bytes-loop result (emit-mov-local-from-rax (local-slot-offset 8)) 0 7)
      (append-native-bytes-loop result (emit-mov-rax-from-rbp-plus-imm8 40) 0 4)
      (append-native-bytes-loop result (emit-mov-local-from-rax (local-slot-offset 9)) 0 7)
      (append-native-bytes-loop result (emit-mov-rax-from-rbp-plus-imm8 48) 0 4)
      (append-native-bytes-loop result (emit-mov-local-from-rax (local-slot-offset 10)) 0 7)
      (append-native-bytes-loop result (emit-mov-rax-from-rbp-plus-imm8 56) 0 4)
      (append-native-bytes-loop result (emit-mov-local-from-rax (local-slot-offset 11)) 0 7)
      (append-native-bytes-loop result (emit-mov-rax-from-rbp-plus-imm8 64) 0 4)
      (append-native-bytes-loop result (emit-mov-local-from-rax (local-slot-offset 12)) 0 7)
      (append-native-bytes-loop result (emit-mov-rax-from-rbp-plus-imm8 72) 0 4)
      (append-native-bytes-loop result (emit-mov-local-from-rax (local-slot-offset 13)) 0 7)
      (append-native-bytes-loop result (emit-mov-rax-from-rbp-plus-imm8 80) 0 4)
      (append-native-bytes-loop result (emit-mov-local-from-rax (local-slot-offset 14)) 0 7)
      (append-native-bytes-loop result (emit-mov-rax-from-rbp-plus-imm8 88) 0 4)
      (append-native-bytes-loop result (emit-mov-local-from-rax (local-slot-offset 15)) 0 7)
      (append-native-bytes-loop result (emit-mov-rax-from-rbp-plus-imm8 96) 0 4)
      (append-native-bytes-loop result (emit-mov-local-from-rax (local-slot-offset 16)) 0 7)
      (append-native-bytes-loop result (emit-mov-rax-from-rbp-plus-imm8 104) 0 4)
      (append-native-bytes-loop result (emit-mov-local-from-rax (local-slot-offset 17)) 0 7)
      (append-native-bytes-loop result (emit-mov-rax-from-rbp-plus-imm8 112) 0 4)
      (append-native-bytes-loop result (emit-mov-local-from-rax (local-slot-offset 18)) 0 7)
      (append-native-bytes-loop result (emit-mov-rax-from-rbp-plus-imm8 120) 0 4)
      (append-native-bytes-loop result (emit-mov-local-from-rax (local-slot-offset 19)) 0 7)
      (append-native-bytes-loop result (emit-mov-rax-from-rbp-plus-imm32 128) 0 7)
      (append-native-bytes-loop result (emit-mov-local-from-rax (local-slot-offset 20)) 0 7)
      (append-native-bytes-loop result (emit-mov-rax-from-rbp-plus-imm32 136) 0 7)
      (append-native-bytes-loop result (emit-mov-local-from-rax (local-slot-offset 21)) 0 7)
      (append-native-bytes-loop result (emit-mov-rax-from-rbp-plus-imm32 144) 0 7)
      (append-native-bytes-loop result (emit-mov-local-from-rax (local-slot-offset 22)) 0 7)
      (append-native-bytes-loop result (emit-mov-rax-from-rbp-plus-imm32 152) 0 7)
      (append-native-bytes-loop result (emit-mov-local-from-rax (local-slot-offset 23)) 0 7)
      (append-native-bytes-loop result (emit-mov-rax-from-rbp-plus-imm32 160) 0 7)
      (append-native-bytes-loop result (emit-mov-local-from-rax (local-slot-offset 24)) 0 7))
    (spill-native-function-params-x86-twenty-to-twenty-four param-count result)))

(defn spill-native-function-params-x86-twenty-to-twenty-six [param-count result]
  (if (= param-count 26)
    (do
      (append-native-bytes-loop result (emit-mov-local-from-rdi (local-slot-offset 0)) 0 7)
      (append-native-bytes-loop result (emit-mov-local-from-rsi (local-slot-offset 1)) 0 7)
      (append-native-bytes-loop result (emit-mov-local-from-rdx (local-slot-offset 2)) 0 7)
      (append-native-bytes-loop result (emit-mov-local-from-rcx (local-slot-offset 3)) 0 7)
      (append-native-bytes-loop result (emit-mov-local-from-r8 (local-slot-offset 4)) 0 7)
      (append-native-bytes-loop result (emit-mov-local-from-r9 (local-slot-offset 5)) 0 7)
      (append-native-bytes-loop result (emit-mov-rax-from-rbp-plus-imm8 16) 0 4)
      (append-native-bytes-loop result (emit-mov-local-from-rax (local-slot-offset 6)) 0 7)
      (append-native-bytes-loop result (emit-mov-rax-from-rbp-plus-imm8 24) 0 4)
      (append-native-bytes-loop result (emit-mov-local-from-rax (local-slot-offset 7)) 0 7)
      (append-native-bytes-loop result (emit-mov-rax-from-rbp-plus-imm8 32) 0 4)
      (append-native-bytes-loop result (emit-mov-local-from-rax (local-slot-offset 8)) 0 7)
      (append-native-bytes-loop result (emit-mov-rax-from-rbp-plus-imm8 40) 0 4)
      (append-native-bytes-loop result (emit-mov-local-from-rax (local-slot-offset 9)) 0 7)
      (append-native-bytes-loop result (emit-mov-rax-from-rbp-plus-imm8 48) 0 4)
      (append-native-bytes-loop result (emit-mov-local-from-rax (local-slot-offset 10)) 0 7)
      (append-native-bytes-loop result (emit-mov-rax-from-rbp-plus-imm8 56) 0 4)
      (append-native-bytes-loop result (emit-mov-local-from-rax (local-slot-offset 11)) 0 7)
      (append-native-bytes-loop result (emit-mov-rax-from-rbp-plus-imm8 64) 0 4)
      (append-native-bytes-loop result (emit-mov-local-from-rax (local-slot-offset 12)) 0 7)
      (append-native-bytes-loop result (emit-mov-rax-from-rbp-plus-imm8 72) 0 4)
      (append-native-bytes-loop result (emit-mov-local-from-rax (local-slot-offset 13)) 0 7)
      (append-native-bytes-loop result (emit-mov-rax-from-rbp-plus-imm8 80) 0 4)
      (append-native-bytes-loop result (emit-mov-local-from-rax (local-slot-offset 14)) 0 7)
      (append-native-bytes-loop result (emit-mov-rax-from-rbp-plus-imm8 88) 0 4)
      (append-native-bytes-loop result (emit-mov-local-from-rax (local-slot-offset 15)) 0 7)
      (append-native-bytes-loop result (emit-mov-rax-from-rbp-plus-imm8 96) 0 4)
      (append-native-bytes-loop result (emit-mov-local-from-rax (local-slot-offset 16)) 0 7)
      (append-native-bytes-loop result (emit-mov-rax-from-rbp-plus-imm8 104) 0 4)
      (append-native-bytes-loop result (emit-mov-local-from-rax (local-slot-offset 17)) 0 7)
      (append-native-bytes-loop result (emit-mov-rax-from-rbp-plus-imm8 112) 0 4)
      (append-native-bytes-loop result (emit-mov-local-from-rax (local-slot-offset 18)) 0 7)
      (append-native-bytes-loop result (emit-mov-rax-from-rbp-plus-imm8 120) 0 4)
      (append-native-bytes-loop result (emit-mov-local-from-rax (local-slot-offset 19)) 0 7)
      (append-native-bytes-loop result (emit-mov-rax-from-rbp-plus-imm32 128) 0 7)
      (append-native-bytes-loop result (emit-mov-local-from-rax (local-slot-offset 20)) 0 7)
      (append-native-bytes-loop result (emit-mov-rax-from-rbp-plus-imm32 136) 0 7)
      (append-native-bytes-loop result (emit-mov-local-from-rax (local-slot-offset 21)) 0 7)
      (append-native-bytes-loop result (emit-mov-rax-from-rbp-plus-imm32 144) 0 7)
      (append-native-bytes-loop result (emit-mov-local-from-rax (local-slot-offset 22)) 0 7)
      (append-native-bytes-loop result (emit-mov-rax-from-rbp-plus-imm32 152) 0 7)
      (append-native-bytes-loop result (emit-mov-local-from-rax (local-slot-offset 23)) 0 7)
      (append-native-bytes-loop result (emit-mov-rax-from-rbp-plus-imm32 160) 0 7)
      (append-native-bytes-loop result (emit-mov-local-from-rax (local-slot-offset 24)) 0 7)
       (append-native-bytes-loop result (emit-mov-rax-from-rbp-plus-imm32 168) 0 7)
       (append-native-bytes-loop result (emit-mov-local-from-rax (local-slot-offset 25)) 0 7))
    (spill-native-function-params-x86-twenty-to-twenty-five param-count result)))

(defn spill-native-function-params-x86-twenty-to-twenty-seven [param-count result]
  (if (= param-count 27)
    (do
      (append-native-bytes-loop result (emit-mov-local-from-rdi (local-slot-offset 0)) 0 7)
      (append-native-bytes-loop result (emit-mov-local-from-rsi (local-slot-offset 1)) 0 7)
      (append-native-bytes-loop result (emit-mov-local-from-rdx (local-slot-offset 2)) 0 7)
      (append-native-bytes-loop result (emit-mov-local-from-rcx (local-slot-offset 3)) 0 7)
      (append-native-bytes-loop result (emit-mov-local-from-r8 (local-slot-offset 4)) 0 7)
      (append-native-bytes-loop result (emit-mov-local-from-r9 (local-slot-offset 5)) 0 7)
      (append-native-bytes-loop result (emit-mov-rax-from-rbp-plus-imm8 16) 0 4)
      (append-native-bytes-loop result (emit-mov-local-from-rax (local-slot-offset 6)) 0 7)
      (append-native-bytes-loop result (emit-mov-rax-from-rbp-plus-imm8 24) 0 4)
      (append-native-bytes-loop result (emit-mov-local-from-rax (local-slot-offset 7)) 0 7)
      (append-native-bytes-loop result (emit-mov-rax-from-rbp-plus-imm8 32) 0 4)
      (append-native-bytes-loop result (emit-mov-local-from-rax (local-slot-offset 8)) 0 7)
      (append-native-bytes-loop result (emit-mov-rax-from-rbp-plus-imm8 40) 0 4)
      (append-native-bytes-loop result (emit-mov-local-from-rax (local-slot-offset 9)) 0 7)
      (append-native-bytes-loop result (emit-mov-rax-from-rbp-plus-imm8 48) 0 4)
      (append-native-bytes-loop result (emit-mov-local-from-rax (local-slot-offset 10)) 0 7)
      (append-native-bytes-loop result (emit-mov-rax-from-rbp-plus-imm8 56) 0 4)
      (append-native-bytes-loop result (emit-mov-local-from-rax (local-slot-offset 11)) 0 7)
      (append-native-bytes-loop result (emit-mov-rax-from-rbp-plus-imm8 64) 0 4)
      (append-native-bytes-loop result (emit-mov-local-from-rax (local-slot-offset 12)) 0 7)
      (append-native-bytes-loop result (emit-mov-rax-from-rbp-plus-imm8 72) 0 4)
      (append-native-bytes-loop result (emit-mov-local-from-rax (local-slot-offset 13)) 0 7)
      (append-native-bytes-loop result (emit-mov-rax-from-rbp-plus-imm8 80) 0 4)
      (append-native-bytes-loop result (emit-mov-local-from-rax (local-slot-offset 14)) 0 7)
      (append-native-bytes-loop result (emit-mov-rax-from-rbp-plus-imm8 88) 0 4)
      (append-native-bytes-loop result (emit-mov-local-from-rax (local-slot-offset 15)) 0 7)
      (append-native-bytes-loop result (emit-mov-rax-from-rbp-plus-imm8 96) 0 4)
      (append-native-bytes-loop result (emit-mov-local-from-rax (local-slot-offset 16)) 0 7)
      (append-native-bytes-loop result (emit-mov-rax-from-rbp-plus-imm8 104) 0 4)
      (append-native-bytes-loop result (emit-mov-local-from-rax (local-slot-offset 17)) 0 7)
      (append-native-bytes-loop result (emit-mov-rax-from-rbp-plus-imm8 112) 0 4)
      (append-native-bytes-loop result (emit-mov-local-from-rax (local-slot-offset 18)) 0 7)
      (append-native-bytes-loop result (emit-mov-rax-from-rbp-plus-imm8 120) 0 4)
      (append-native-bytes-loop result (emit-mov-local-from-rax (local-slot-offset 19)) 0 7)
      (append-native-bytes-loop result (emit-mov-rax-from-rbp-plus-imm32 128) 0 7)
      (append-native-bytes-loop result (emit-mov-local-from-rax (local-slot-offset 20)) 0 7)
      (append-native-bytes-loop result (emit-mov-rax-from-rbp-plus-imm32 136) 0 7)
      (append-native-bytes-loop result (emit-mov-local-from-rax (local-slot-offset 21)) 0 7)
      (append-native-bytes-loop result (emit-mov-rax-from-rbp-plus-imm32 144) 0 7)
      (append-native-bytes-loop result (emit-mov-local-from-rax (local-slot-offset 22)) 0 7)
      (append-native-bytes-loop result (emit-mov-rax-from-rbp-plus-imm32 152) 0 7)
      (append-native-bytes-loop result (emit-mov-local-from-rax (local-slot-offset 23)) 0 7)
      (append-native-bytes-loop result (emit-mov-rax-from-rbp-plus-imm32 160) 0 7)
      (append-native-bytes-loop result (emit-mov-local-from-rax (local-slot-offset 24)) 0 7)
      (append-native-bytes-loop result (emit-mov-rax-from-rbp-plus-imm32 168) 0 7)
      (append-native-bytes-loop result (emit-mov-local-from-rax (local-slot-offset 25)) 0 7)
       (append-native-bytes-loop result (emit-mov-rax-from-rbp-plus-imm32 176) 0 7)
       (append-native-bytes-loop result (emit-mov-local-from-rax (local-slot-offset 26)) 0 7))
    (spill-native-function-params-x86-twenty-to-twenty-six param-count result)))

(defn spill-native-function-params-x86-twenty-to-twenty-eight [param-count result]
  (if (= param-count 28)
    (do
      (append-native-bytes-loop result (emit-mov-local-from-rdi (local-slot-offset 0)) 0 7)
      (append-native-bytes-loop result (emit-mov-local-from-rsi (local-slot-offset 1)) 0 7)
      (append-native-bytes-loop result (emit-mov-local-from-rdx (local-slot-offset 2)) 0 7)
      (append-native-bytes-loop result (emit-mov-local-from-rcx (local-slot-offset 3)) 0 7)
      (append-native-bytes-loop result (emit-mov-local-from-r8 (local-slot-offset 4)) 0 7)
      (append-native-bytes-loop result (emit-mov-local-from-r9 (local-slot-offset 5)) 0 7)
      (append-native-bytes-loop result (emit-mov-rax-from-rbp-plus-imm8 16) 0 4)
      (append-native-bytes-loop result (emit-mov-local-from-rax (local-slot-offset 6)) 0 7)
      (append-native-bytes-loop result (emit-mov-rax-from-rbp-plus-imm8 24) 0 4)
      (append-native-bytes-loop result (emit-mov-local-from-rax (local-slot-offset 7)) 0 7)
      (append-native-bytes-loop result (emit-mov-rax-from-rbp-plus-imm8 32) 0 4)
      (append-native-bytes-loop result (emit-mov-local-from-rax (local-slot-offset 8)) 0 7)
      (append-native-bytes-loop result (emit-mov-rax-from-rbp-plus-imm8 40) 0 4)
      (append-native-bytes-loop result (emit-mov-local-from-rax (local-slot-offset 9)) 0 7)
      (append-native-bytes-loop result (emit-mov-rax-from-rbp-plus-imm8 48) 0 4)
      (append-native-bytes-loop result (emit-mov-local-from-rax (local-slot-offset 10)) 0 7)
      (append-native-bytes-loop result (emit-mov-rax-from-rbp-plus-imm8 56) 0 4)
      (append-native-bytes-loop result (emit-mov-local-from-rax (local-slot-offset 11)) 0 7)
      (append-native-bytes-loop result (emit-mov-rax-from-rbp-plus-imm8 64) 0 4)
      (append-native-bytes-loop result (emit-mov-local-from-rax (local-slot-offset 12)) 0 7)
      (append-native-bytes-loop result (emit-mov-rax-from-rbp-plus-imm8 72) 0 4)
      (append-native-bytes-loop result (emit-mov-local-from-rax (local-slot-offset 13)) 0 7)
      (append-native-bytes-loop result (emit-mov-rax-from-rbp-plus-imm8 80) 0 4)
      (append-native-bytes-loop result (emit-mov-local-from-rax (local-slot-offset 14)) 0 7)
      (append-native-bytes-loop result (emit-mov-rax-from-rbp-plus-imm8 88) 0 4)
      (append-native-bytes-loop result (emit-mov-local-from-rax (local-slot-offset 15)) 0 7)
      (append-native-bytes-loop result (emit-mov-rax-from-rbp-plus-imm8 96) 0 4)
      (append-native-bytes-loop result (emit-mov-local-from-rax (local-slot-offset 16)) 0 7)
      (append-native-bytes-loop result (emit-mov-rax-from-rbp-plus-imm8 104) 0 4)
      (append-native-bytes-loop result (emit-mov-local-from-rax (local-slot-offset 17)) 0 7)
      (append-native-bytes-loop result (emit-mov-rax-from-rbp-plus-imm8 112) 0 4)
      (append-native-bytes-loop result (emit-mov-local-from-rax (local-slot-offset 18)) 0 7)
      (append-native-bytes-loop result (emit-mov-rax-from-rbp-plus-imm8 120) 0 4)
      (append-native-bytes-loop result (emit-mov-local-from-rax (local-slot-offset 19)) 0 7)
      (append-native-bytes-loop result (emit-mov-rax-from-rbp-plus-imm32 128) 0 7)
      (append-native-bytes-loop result (emit-mov-local-from-rax (local-slot-offset 20)) 0 7)
      (append-native-bytes-loop result (emit-mov-rax-from-rbp-plus-imm32 136) 0 7)
      (append-native-bytes-loop result (emit-mov-local-from-rax (local-slot-offset 21)) 0 7)
      (append-native-bytes-loop result (emit-mov-rax-from-rbp-plus-imm32 144) 0 7)
      (append-native-bytes-loop result (emit-mov-local-from-rax (local-slot-offset 22)) 0 7)
      (append-native-bytes-loop result (emit-mov-rax-from-rbp-plus-imm32 152) 0 7)
      (append-native-bytes-loop result (emit-mov-local-from-rax (local-slot-offset 23)) 0 7)
      (append-native-bytes-loop result (emit-mov-rax-from-rbp-plus-imm32 160) 0 7)
      (append-native-bytes-loop result (emit-mov-local-from-rax (local-slot-offset 24)) 0 7)
      (append-native-bytes-loop result (emit-mov-rax-from-rbp-plus-imm32 168) 0 7)
      (append-native-bytes-loop result (emit-mov-local-from-rax (local-slot-offset 25)) 0 7)
      (append-native-bytes-loop result (emit-mov-rax-from-rbp-plus-imm32 176) 0 7)
      (append-native-bytes-loop result (emit-mov-local-from-rax (local-slot-offset 26)) 0 7)
      (append-native-bytes-loop result (emit-mov-rax-from-rbp-plus-imm32 184) 0 7)
      (append-native-bytes-loop result (emit-mov-local-from-rax (local-slot-offset 27)) 0 7))
    (spill-native-function-params-x86-twenty-to-twenty-seven param-count result)))

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
    param-spill-bytes (if (>= param-count 20)
                        (native-param-spill-bytes-x86-twenty-to-twenty-eight param-count)
                        (if (> param-count 6)
                          (+ 53 (* (- param-count 7) 11))
                          (if (= param-count 6)
                            42
                            (if (= param-count 5)
                              35
                              (if (= param-count 4)
                                28
                                (if (= param-count 3)
                                  21
                                  (if (= param-count 2)
                                    14
                                    (if (= param-count 1) 7 0))))))))
    body-offset (+ after-stack-offset param-spill-bytes)
    n (vector-length ir-func)]
    (do
      (ref-set result (vector-push (ref-get result) (vector-get prologue-push 0)))
      (ref-set result (vector-push (ref-get result) (vector-get prologue-mov 0)))
      (ref-set result (vector-push (ref-get result) (vector-get prologue-mov 1)))
      (ref-set result (vector-push (ref-get result) (vector-get prologue-mov 2)))
      (if (> stack-bytes 0)
        (append-native-bytes-loop result (emit-sub-rsp-imm32 stack-bytes) 0 7)
        0)
      (if (>= param-count 20)
        (spill-native-function-params-x86-twenty-to-twenty-eight param-count result)
        (if (= param-count 19)
        (do
          (append-native-bytes-loop result (emit-mov-local-from-rdi (local-slot-offset 0)) 0 7)
          (append-native-bytes-loop result (emit-mov-local-from-rsi (local-slot-offset 1)) 0 7)
          (append-native-bytes-loop result (emit-mov-local-from-rdx (local-slot-offset 2)) 0 7)
          (append-native-bytes-loop result (emit-mov-local-from-rcx (local-slot-offset 3)) 0 7)
          (append-native-bytes-loop result (emit-mov-local-from-r8 (local-slot-offset 4)) 0 7)
          (append-native-bytes-loop result (emit-mov-local-from-r9 (local-slot-offset 5)) 0 7)
          (append-native-bytes-loop result (emit-mov-rax-from-rbp-plus-imm8 16) 0 4)
          (append-native-bytes-loop result (emit-mov-local-from-rax (local-slot-offset 6)) 0 7)
          (append-native-bytes-loop result (emit-mov-rax-from-rbp-plus-imm8 24) 0 4)
          (append-native-bytes-loop result (emit-mov-local-from-rax (local-slot-offset 7)) 0 7)
          (append-native-bytes-loop result (emit-mov-rax-from-rbp-plus-imm8 32) 0 4)
          (append-native-bytes-loop result (emit-mov-local-from-rax (local-slot-offset 8)) 0 7)
          (append-native-bytes-loop result (emit-mov-rax-from-rbp-plus-imm8 40) 0 4)
          (append-native-bytes-loop result (emit-mov-local-from-rax (local-slot-offset 9)) 0 7)
          (append-native-bytes-loop result (emit-mov-rax-from-rbp-plus-imm8 48) 0 4)
          (append-native-bytes-loop result (emit-mov-local-from-rax (local-slot-offset 10)) 0 7)
          (append-native-bytes-loop result (emit-mov-rax-from-rbp-plus-imm8 56) 0 4)
          (append-native-bytes-loop result (emit-mov-local-from-rax (local-slot-offset 11)) 0 7)
          (append-native-bytes-loop result (emit-mov-rax-from-rbp-plus-imm8 64) 0 4)
          (append-native-bytes-loop result (emit-mov-local-from-rax (local-slot-offset 12)) 0 7)
          (append-native-bytes-loop result (emit-mov-rax-from-rbp-plus-imm8 72) 0 4)
          (append-native-bytes-loop result (emit-mov-local-from-rax (local-slot-offset 13)) 0 7)
          (append-native-bytes-loop result (emit-mov-rax-from-rbp-plus-imm8 80) 0 4)
          (append-native-bytes-loop result (emit-mov-local-from-rax (local-slot-offset 14)) 0 7)
          (append-native-bytes-loop result (emit-mov-rax-from-rbp-plus-imm8 88) 0 4)
          (append-native-bytes-loop result (emit-mov-local-from-rax (local-slot-offset 15)) 0 7)
          (append-native-bytes-loop result (emit-mov-rax-from-rbp-plus-imm8 96) 0 4)
          (append-native-bytes-loop result (emit-mov-local-from-rax (local-slot-offset 16)) 0 7)
          (append-native-bytes-loop result (emit-mov-rax-from-rbp-plus-imm8 104) 0 4)
          (append-native-bytes-loop result (emit-mov-local-from-rax (local-slot-offset 17)) 0 7)
          (append-native-bytes-loop result (emit-mov-rax-from-rbp-plus-imm8 112) 0 4)
          (append-native-bytes-loop result (emit-mov-local-from-rax (local-slot-offset 18)) 0 7))
        (if (= param-count 18)
        (do
          (append-native-bytes-loop result (emit-mov-local-from-rdi (local-slot-offset 0)) 0 7)
          (append-native-bytes-loop result (emit-mov-local-from-rsi (local-slot-offset 1)) 0 7)
          (append-native-bytes-loop result (emit-mov-local-from-rdx (local-slot-offset 2)) 0 7)
          (append-native-bytes-loop result (emit-mov-local-from-rcx (local-slot-offset 3)) 0 7)
          (append-native-bytes-loop result (emit-mov-local-from-r8 (local-slot-offset 4)) 0 7)
          (append-native-bytes-loop result (emit-mov-local-from-r9 (local-slot-offset 5)) 0 7)
          (append-native-bytes-loop result (emit-mov-rax-from-rbp-plus-imm8 16) 0 4)
          (append-native-bytes-loop result (emit-mov-local-from-rax (local-slot-offset 6)) 0 7)
          (append-native-bytes-loop result (emit-mov-rax-from-rbp-plus-imm8 24) 0 4)
          (append-native-bytes-loop result (emit-mov-local-from-rax (local-slot-offset 7)) 0 7)
          (append-native-bytes-loop result (emit-mov-rax-from-rbp-plus-imm8 32) 0 4)
          (append-native-bytes-loop result (emit-mov-local-from-rax (local-slot-offset 8)) 0 7)
          (append-native-bytes-loop result (emit-mov-rax-from-rbp-plus-imm8 40) 0 4)
          (append-native-bytes-loop result (emit-mov-local-from-rax (local-slot-offset 9)) 0 7)
          (append-native-bytes-loop result (emit-mov-rax-from-rbp-plus-imm8 48) 0 4)
          (append-native-bytes-loop result (emit-mov-local-from-rax (local-slot-offset 10)) 0 7)
          (append-native-bytes-loop result (emit-mov-rax-from-rbp-plus-imm8 56) 0 4)
          (append-native-bytes-loop result (emit-mov-local-from-rax (local-slot-offset 11)) 0 7)
          (append-native-bytes-loop result (emit-mov-rax-from-rbp-plus-imm8 64) 0 4)
          (append-native-bytes-loop result (emit-mov-local-from-rax (local-slot-offset 12)) 0 7)
          (append-native-bytes-loop result (emit-mov-rax-from-rbp-plus-imm8 72) 0 4)
          (append-native-bytes-loop result (emit-mov-local-from-rax (local-slot-offset 13)) 0 7)
          (append-native-bytes-loop result (emit-mov-rax-from-rbp-plus-imm8 80) 0 4)
          (append-native-bytes-loop result (emit-mov-local-from-rax (local-slot-offset 14)) 0 7)
          (append-native-bytes-loop result (emit-mov-rax-from-rbp-plus-imm8 88) 0 4)
          (append-native-bytes-loop result (emit-mov-local-from-rax (local-slot-offset 15)) 0 7)
          (append-native-bytes-loop result (emit-mov-rax-from-rbp-plus-imm8 96) 0 4)
          (append-native-bytes-loop result (emit-mov-local-from-rax (local-slot-offset 16)) 0 7)
          (append-native-bytes-loop result (emit-mov-rax-from-rbp-plus-imm8 104) 0 4)
          (append-native-bytes-loop result (emit-mov-local-from-rax (local-slot-offset 17)) 0 7))
        (if (= param-count 17)
        (do
          (append-native-bytes-loop result (emit-mov-local-from-rdi (local-slot-offset 0)) 0 7)
          (append-native-bytes-loop result (emit-mov-local-from-rsi (local-slot-offset 1)) 0 7)
          (append-native-bytes-loop result (emit-mov-local-from-rdx (local-slot-offset 2)) 0 7)
          (append-native-bytes-loop result (emit-mov-local-from-rcx (local-slot-offset 3)) 0 7)
          (append-native-bytes-loop result (emit-mov-local-from-r8 (local-slot-offset 4)) 0 7)
          (append-native-bytes-loop result (emit-mov-local-from-r9 (local-slot-offset 5)) 0 7)
          (append-native-bytes-loop result (emit-mov-rax-from-rbp-plus-imm8 16) 0 4)
          (append-native-bytes-loop result (emit-mov-local-from-rax (local-slot-offset 6)) 0 7)
          (append-native-bytes-loop result (emit-mov-rax-from-rbp-plus-imm8 24) 0 4)
          (append-native-bytes-loop result (emit-mov-local-from-rax (local-slot-offset 7)) 0 7)
          (append-native-bytes-loop result (emit-mov-rax-from-rbp-plus-imm8 32) 0 4)
          (append-native-bytes-loop result (emit-mov-local-from-rax (local-slot-offset 8)) 0 7)
          (append-native-bytes-loop result (emit-mov-rax-from-rbp-plus-imm8 40) 0 4)
          (append-native-bytes-loop result (emit-mov-local-from-rax (local-slot-offset 9)) 0 7)
          (append-native-bytes-loop result (emit-mov-rax-from-rbp-plus-imm8 48) 0 4)
          (append-native-bytes-loop result (emit-mov-local-from-rax (local-slot-offset 10)) 0 7)
          (append-native-bytes-loop result (emit-mov-rax-from-rbp-plus-imm8 56) 0 4)
          (append-native-bytes-loop result (emit-mov-local-from-rax (local-slot-offset 11)) 0 7)
          (append-native-bytes-loop result (emit-mov-rax-from-rbp-plus-imm8 64) 0 4)
          (append-native-bytes-loop result (emit-mov-local-from-rax (local-slot-offset 12)) 0 7)
          (append-native-bytes-loop result (emit-mov-rax-from-rbp-plus-imm8 72) 0 4)
          (append-native-bytes-loop result (emit-mov-local-from-rax (local-slot-offset 13)) 0 7)
          (append-native-bytes-loop result (emit-mov-rax-from-rbp-plus-imm8 80) 0 4)
          (append-native-bytes-loop result (emit-mov-local-from-rax (local-slot-offset 14)) 0 7)
          (append-native-bytes-loop result (emit-mov-rax-from-rbp-plus-imm8 88) 0 4)
          (append-native-bytes-loop result (emit-mov-local-from-rax (local-slot-offset 15)) 0 7)
          (append-native-bytes-loop result (emit-mov-rax-from-rbp-plus-imm8 96) 0 4)
          (append-native-bytes-loop result (emit-mov-local-from-rax (local-slot-offset 16)) 0 7))
        (if (= param-count 16)
        (do
          (append-native-bytes-loop result (emit-mov-local-from-rdi (local-slot-offset 0)) 0 7)
          (append-native-bytes-loop result (emit-mov-local-from-rsi (local-slot-offset 1)) 0 7)
          (append-native-bytes-loop result (emit-mov-local-from-rdx (local-slot-offset 2)) 0 7)
          (append-native-bytes-loop result (emit-mov-local-from-rcx (local-slot-offset 3)) 0 7)
          (append-native-bytes-loop result (emit-mov-local-from-r8 (local-slot-offset 4)) 0 7)
          (append-native-bytes-loop result (emit-mov-local-from-r9 (local-slot-offset 5)) 0 7)
          (append-native-bytes-loop result (emit-mov-rax-from-rbp-plus-imm8 16) 0 4)
          (append-native-bytes-loop result (emit-mov-local-from-rax (local-slot-offset 6)) 0 7)
          (append-native-bytes-loop result (emit-mov-rax-from-rbp-plus-imm8 24) 0 4)
          (append-native-bytes-loop result (emit-mov-local-from-rax (local-slot-offset 7)) 0 7)
          (append-native-bytes-loop result (emit-mov-rax-from-rbp-plus-imm8 32) 0 4)
          (append-native-bytes-loop result (emit-mov-local-from-rax (local-slot-offset 8)) 0 7)
          (append-native-bytes-loop result (emit-mov-rax-from-rbp-plus-imm8 40) 0 4)
          (append-native-bytes-loop result (emit-mov-local-from-rax (local-slot-offset 9)) 0 7)
          (append-native-bytes-loop result (emit-mov-rax-from-rbp-plus-imm8 48) 0 4)
          (append-native-bytes-loop result (emit-mov-local-from-rax (local-slot-offset 10)) 0 7)
          (append-native-bytes-loop result (emit-mov-rax-from-rbp-plus-imm8 56) 0 4)
          (append-native-bytes-loop result (emit-mov-local-from-rax (local-slot-offset 11)) 0 7)
          (append-native-bytes-loop result (emit-mov-rax-from-rbp-plus-imm8 64) 0 4)
          (append-native-bytes-loop result (emit-mov-local-from-rax (local-slot-offset 12)) 0 7)
          (append-native-bytes-loop result (emit-mov-rax-from-rbp-plus-imm8 72) 0 4)
          (append-native-bytes-loop result (emit-mov-local-from-rax (local-slot-offset 13)) 0 7)
          (append-native-bytes-loop result (emit-mov-rax-from-rbp-plus-imm8 80) 0 4)
          (append-native-bytes-loop result (emit-mov-local-from-rax (local-slot-offset 14)) 0 7)
          (append-native-bytes-loop result (emit-mov-rax-from-rbp-plus-imm8 88) 0 4)
          (append-native-bytes-loop result (emit-mov-local-from-rax (local-slot-offset 15)) 0 7))
        (if (= param-count 15)
        (do
          (append-native-bytes-loop result (emit-mov-local-from-rdi (local-slot-offset 0)) 0 7)
          (append-native-bytes-loop result (emit-mov-local-from-rsi (local-slot-offset 1)) 0 7)
          (append-native-bytes-loop result (emit-mov-local-from-rdx (local-slot-offset 2)) 0 7)
          (append-native-bytes-loop result (emit-mov-local-from-rcx (local-slot-offset 3)) 0 7)
          (append-native-bytes-loop result (emit-mov-local-from-r8 (local-slot-offset 4)) 0 7)
          (append-native-bytes-loop result (emit-mov-local-from-r9 (local-slot-offset 5)) 0 7)
          (append-native-bytes-loop result (emit-mov-rax-from-rbp-plus-imm8 16) 0 4)
          (append-native-bytes-loop result (emit-mov-local-from-rax (local-slot-offset 6)) 0 7)
          (append-native-bytes-loop result (emit-mov-rax-from-rbp-plus-imm8 24) 0 4)
          (append-native-bytes-loop result (emit-mov-local-from-rax (local-slot-offset 7)) 0 7)
          (append-native-bytes-loop result (emit-mov-rax-from-rbp-plus-imm8 32) 0 4)
          (append-native-bytes-loop result (emit-mov-local-from-rax (local-slot-offset 8)) 0 7)
          (append-native-bytes-loop result (emit-mov-rax-from-rbp-plus-imm8 40) 0 4)
          (append-native-bytes-loop result (emit-mov-local-from-rax (local-slot-offset 9)) 0 7)
          (append-native-bytes-loop result (emit-mov-rax-from-rbp-plus-imm8 48) 0 4)
          (append-native-bytes-loop result (emit-mov-local-from-rax (local-slot-offset 10)) 0 7)
          (append-native-bytes-loop result (emit-mov-rax-from-rbp-plus-imm8 56) 0 4)
          (append-native-bytes-loop result (emit-mov-local-from-rax (local-slot-offset 11)) 0 7)
          (append-native-bytes-loop result (emit-mov-rax-from-rbp-plus-imm8 64) 0 4)
          (append-native-bytes-loop result (emit-mov-local-from-rax (local-slot-offset 12)) 0 7)
          (append-native-bytes-loop result (emit-mov-rax-from-rbp-plus-imm8 72) 0 4)
          (append-native-bytes-loop result (emit-mov-local-from-rax (local-slot-offset 13)) 0 7)
          (append-native-bytes-loop result (emit-mov-rax-from-rbp-plus-imm8 80) 0 4)
          (append-native-bytes-loop result (emit-mov-local-from-rax (local-slot-offset 14)) 0 7))
        (if (= param-count 14)
        (do
          (append-native-bytes-loop result (emit-mov-local-from-rdi (local-slot-offset 0)) 0 7)
          (append-native-bytes-loop result (emit-mov-local-from-rsi (local-slot-offset 1)) 0 7)
          (append-native-bytes-loop result (emit-mov-local-from-rdx (local-slot-offset 2)) 0 7)
          (append-native-bytes-loop result (emit-mov-local-from-rcx (local-slot-offset 3)) 0 7)
          (append-native-bytes-loop result (emit-mov-local-from-r8 (local-slot-offset 4)) 0 7)
          (append-native-bytes-loop result (emit-mov-local-from-r9 (local-slot-offset 5)) 0 7)
          (append-native-bytes-loop result (emit-mov-rax-from-rbp-plus-imm8 16) 0 4)
          (append-native-bytes-loop result (emit-mov-local-from-rax (local-slot-offset 6)) 0 7)
          (append-native-bytes-loop result (emit-mov-rax-from-rbp-plus-imm8 24) 0 4)
          (append-native-bytes-loop result (emit-mov-local-from-rax (local-slot-offset 7)) 0 7)
          (append-native-bytes-loop result (emit-mov-rax-from-rbp-plus-imm8 32) 0 4)
          (append-native-bytes-loop result (emit-mov-local-from-rax (local-slot-offset 8)) 0 7)
          (append-native-bytes-loop result (emit-mov-rax-from-rbp-plus-imm8 40) 0 4)
          (append-native-bytes-loop result (emit-mov-local-from-rax (local-slot-offset 9)) 0 7)
          (append-native-bytes-loop result (emit-mov-rax-from-rbp-plus-imm8 48) 0 4)
          (append-native-bytes-loop result (emit-mov-local-from-rax (local-slot-offset 10)) 0 7)
          (append-native-bytes-loop result (emit-mov-rax-from-rbp-plus-imm8 56) 0 4)
          (append-native-bytes-loop result (emit-mov-local-from-rax (local-slot-offset 11)) 0 7)
          (append-native-bytes-loop result (emit-mov-rax-from-rbp-plus-imm8 64) 0 4)
          (append-native-bytes-loop result (emit-mov-local-from-rax (local-slot-offset 12)) 0 7)
          (append-native-bytes-loop result (emit-mov-rax-from-rbp-plus-imm8 72) 0 4)
          (append-native-bytes-loop result (emit-mov-local-from-rax (local-slot-offset 13)) 0 7))
        (if (= param-count 13)
        (do
          (append-native-bytes-loop result (emit-mov-local-from-rdi (local-slot-offset 0)) 0 7)
          (append-native-bytes-loop result (emit-mov-local-from-rsi (local-slot-offset 1)) 0 7)
          (append-native-bytes-loop result (emit-mov-local-from-rdx (local-slot-offset 2)) 0 7)
          (append-native-bytes-loop result (emit-mov-local-from-rcx (local-slot-offset 3)) 0 7)
          (append-native-bytes-loop result (emit-mov-local-from-r8 (local-slot-offset 4)) 0 7)
          (append-native-bytes-loop result (emit-mov-local-from-r9 (local-slot-offset 5)) 0 7)
          (append-native-bytes-loop result (emit-mov-rax-from-rbp-plus-imm8 16) 0 4)
          (append-native-bytes-loop result (emit-mov-local-from-rax (local-slot-offset 6)) 0 7)
          (append-native-bytes-loop result (emit-mov-rax-from-rbp-plus-imm8 24) 0 4)
          (append-native-bytes-loop result (emit-mov-local-from-rax (local-slot-offset 7)) 0 7)
          (append-native-bytes-loop result (emit-mov-rax-from-rbp-plus-imm8 32) 0 4)
          (append-native-bytes-loop result (emit-mov-local-from-rax (local-slot-offset 8)) 0 7)
          (append-native-bytes-loop result (emit-mov-rax-from-rbp-plus-imm8 40) 0 4)
          (append-native-bytes-loop result (emit-mov-local-from-rax (local-slot-offset 9)) 0 7)
          (append-native-bytes-loop result (emit-mov-rax-from-rbp-plus-imm8 48) 0 4)
          (append-native-bytes-loop result (emit-mov-local-from-rax (local-slot-offset 10)) 0 7)
          (append-native-bytes-loop result (emit-mov-rax-from-rbp-plus-imm8 56) 0 4)
          (append-native-bytes-loop result (emit-mov-local-from-rax (local-slot-offset 11)) 0 7)
          (append-native-bytes-loop result (emit-mov-rax-from-rbp-plus-imm8 64) 0 4)
          (append-native-bytes-loop result (emit-mov-local-from-rax (local-slot-offset 12)) 0 7))
        (if (= param-count 12)
        (do
          (append-native-bytes-loop result (emit-mov-local-from-rdi (local-slot-offset 0)) 0 7)
          (append-native-bytes-loop result (emit-mov-local-from-rsi (local-slot-offset 1)) 0 7)
          (append-native-bytes-loop result (emit-mov-local-from-rdx (local-slot-offset 2)) 0 7)
          (append-native-bytes-loop result (emit-mov-local-from-rcx (local-slot-offset 3)) 0 7)
          (append-native-bytes-loop result (emit-mov-local-from-r8 (local-slot-offset 4)) 0 7)
          (append-native-bytes-loop result (emit-mov-local-from-r9 (local-slot-offset 5)) 0 7)
          (append-native-bytes-loop result (emit-mov-rax-from-rbp-plus-imm8 16) 0 4)
          (append-native-bytes-loop result (emit-mov-local-from-rax (local-slot-offset 6)) 0 7)
          (append-native-bytes-loop result (emit-mov-rax-from-rbp-plus-imm8 24) 0 4)
          (append-native-bytes-loop result (emit-mov-local-from-rax (local-slot-offset 7)) 0 7)
          (append-native-bytes-loop result (emit-mov-rax-from-rbp-plus-imm8 32) 0 4)
          (append-native-bytes-loop result (emit-mov-local-from-rax (local-slot-offset 8)) 0 7)
          (append-native-bytes-loop result (emit-mov-rax-from-rbp-plus-imm8 40) 0 4)
          (append-native-bytes-loop result (emit-mov-local-from-rax (local-slot-offset 9)) 0 7)
          (append-native-bytes-loop result (emit-mov-rax-from-rbp-plus-imm8 48) 0 4)
          (append-native-bytes-loop result (emit-mov-local-from-rax (local-slot-offset 10)) 0 7)
          (append-native-bytes-loop result (emit-mov-rax-from-rbp-plus-imm8 56) 0 4)
          (append-native-bytes-loop result (emit-mov-local-from-rax (local-slot-offset 11)) 0 7))
        (if (= param-count 11)
        (do
          (append-native-bytes-loop result (emit-mov-local-from-rdi (local-slot-offset 0)) 0 7)
          (append-native-bytes-loop result (emit-mov-local-from-rsi (local-slot-offset 1)) 0 7)
          (append-native-bytes-loop result (emit-mov-local-from-rdx (local-slot-offset 2)) 0 7)
          (append-native-bytes-loop result (emit-mov-local-from-rcx (local-slot-offset 3)) 0 7)
          (append-native-bytes-loop result (emit-mov-local-from-r8 (local-slot-offset 4)) 0 7)
          (append-native-bytes-loop result (emit-mov-local-from-r9 (local-slot-offset 5)) 0 7)
          (append-native-bytes-loop result (emit-mov-rax-from-rbp-plus-imm8 16) 0 4)
          (append-native-bytes-loop result (emit-mov-local-from-rax (local-slot-offset 6)) 0 7)
          (append-native-bytes-loop result (emit-mov-rax-from-rbp-plus-imm8 24) 0 4)
          (append-native-bytes-loop result (emit-mov-local-from-rax (local-slot-offset 7)) 0 7)
          (append-native-bytes-loop result (emit-mov-rax-from-rbp-plus-imm8 32) 0 4)
          (append-native-bytes-loop result (emit-mov-local-from-rax (local-slot-offset 8)) 0 7)
          (append-native-bytes-loop result (emit-mov-rax-from-rbp-plus-imm8 40) 0 4)
          (append-native-bytes-loop result (emit-mov-local-from-rax (local-slot-offset 9)) 0 7)
          (append-native-bytes-loop result (emit-mov-rax-from-rbp-plus-imm8 48) 0 4)
          (append-native-bytes-loop result (emit-mov-local-from-rax (local-slot-offset 10)) 0 7))
        (if (= param-count 10)
        (do
          (append-native-bytes-loop result (emit-mov-local-from-rdi (local-slot-offset 0)) 0 7)
          (append-native-bytes-loop result (emit-mov-local-from-rsi (local-slot-offset 1)) 0 7)
          (append-native-bytes-loop result (emit-mov-local-from-rdx (local-slot-offset 2)) 0 7)
          (append-native-bytes-loop result (emit-mov-local-from-rcx (local-slot-offset 3)) 0 7)
          (append-native-bytes-loop result (emit-mov-local-from-r8 (local-slot-offset 4)) 0 7)
          (append-native-bytes-loop result (emit-mov-local-from-r9 (local-slot-offset 5)) 0 7)
          (append-native-bytes-loop result (emit-mov-rax-from-rbp-plus-imm8 16) 0 4)
          (append-native-bytes-loop result (emit-mov-local-from-rax (local-slot-offset 6)) 0 7)
          (append-native-bytes-loop result (emit-mov-rax-from-rbp-plus-imm8 24) 0 4)
          (append-native-bytes-loop result (emit-mov-local-from-rax (local-slot-offset 7)) 0 7)
          (append-native-bytes-loop result (emit-mov-rax-from-rbp-plus-imm8 32) 0 4)
          (append-native-bytes-loop result (emit-mov-local-from-rax (local-slot-offset 8)) 0 7)
          (append-native-bytes-loop result (emit-mov-rax-from-rbp-plus-imm8 40) 0 4)
          (append-native-bytes-loop result (emit-mov-local-from-rax (local-slot-offset 9)) 0 7))
        (if (= param-count 9)
        (do
          (append-native-bytes-loop result (emit-mov-local-from-rdi (local-slot-offset 0)) 0 7)
          (append-native-bytes-loop result (emit-mov-local-from-rsi (local-slot-offset 1)) 0 7)
          (append-native-bytes-loop result (emit-mov-local-from-rdx (local-slot-offset 2)) 0 7)
          (append-native-bytes-loop result (emit-mov-local-from-rcx (local-slot-offset 3)) 0 7)
          (append-native-bytes-loop result (emit-mov-local-from-r8 (local-slot-offset 4)) 0 7)
          (append-native-bytes-loop result (emit-mov-local-from-r9 (local-slot-offset 5)) 0 7)
          (append-native-bytes-loop result (emit-mov-rax-from-rbp-plus-imm8 16) 0 4)
          (append-native-bytes-loop result (emit-mov-local-from-rax (local-slot-offset 6)) 0 7)
          (append-native-bytes-loop result (emit-mov-rax-from-rbp-plus-imm8 24) 0 4)
          (append-native-bytes-loop result (emit-mov-local-from-rax (local-slot-offset 7)) 0 7)
          (append-native-bytes-loop result (emit-mov-rax-from-rbp-plus-imm8 32) 0 4)
          (append-native-bytes-loop result (emit-mov-local-from-rax (local-slot-offset 8)) 0 7))
        (if (= param-count 8)
        (do
          (append-native-bytes-loop result (emit-mov-local-from-rdi (local-slot-offset 0)) 0 7)
          (append-native-bytes-loop result (emit-mov-local-from-rsi (local-slot-offset 1)) 0 7)
          (append-native-bytes-loop result (emit-mov-local-from-rdx (local-slot-offset 2)) 0 7)
          (append-native-bytes-loop result (emit-mov-local-from-rcx (local-slot-offset 3)) 0 7)
          (append-native-bytes-loop result (emit-mov-local-from-r8 (local-slot-offset 4)) 0 7)
          (append-native-bytes-loop result (emit-mov-local-from-r9 (local-slot-offset 5)) 0 7)
          (append-native-bytes-loop result (emit-mov-rax-from-rbp-plus-imm8 16) 0 4)
          (append-native-bytes-loop result (emit-mov-local-from-rax (local-slot-offset 6)) 0 7)
          (append-native-bytes-loop result (emit-mov-rax-from-rbp-plus-imm8 24) 0 4)
          (append-native-bytes-loop result (emit-mov-local-from-rax (local-slot-offset 7)) 0 7))
        (if (= param-count 7)
          (do
            (append-native-bytes-loop result (emit-mov-local-from-rdi (local-slot-offset 0)) 0 7)
            (append-native-bytes-loop result (emit-mov-local-from-rsi (local-slot-offset 1)) 0 7)
            (append-native-bytes-loop result (emit-mov-local-from-rdx (local-slot-offset 2)) 0 7)
            (append-native-bytes-loop result (emit-mov-local-from-rcx (local-slot-offset 3)) 0 7)
            (append-native-bytes-loop result (emit-mov-local-from-r8 (local-slot-offset 4)) 0 7)
            (append-native-bytes-loop result (emit-mov-local-from-r9 (local-slot-offset 5)) 0 7)
            (append-native-bytes-loop result (emit-mov-rax-from-rbp-plus-imm8 16) 0 4)
            (append-native-bytes-loop result (emit-mov-local-from-rax (local-slot-offset 6)) 0 7))
          (if (= param-count 6)
            (do
              (append-native-bytes-loop result (emit-mov-local-from-rdi (local-slot-offset 0)) 0 7)
              (append-native-bytes-loop result (emit-mov-local-from-rsi (local-slot-offset 1)) 0 7)
              (append-native-bytes-loop result (emit-mov-local-from-rdx (local-slot-offset 2)) 0 7)
              (append-native-bytes-loop result (emit-mov-local-from-rcx (local-slot-offset 3)) 0 7)
              (append-native-bytes-loop result (emit-mov-local-from-r8 (local-slot-offset 4)) 0 7)
              (append-native-bytes-loop result (emit-mov-local-from-r9 (local-slot-offset 5)) 0 7))
            (if (= param-count 5)
              (do
                (append-native-bytes-loop result (emit-mov-local-from-rdi (local-slot-offset 0)) 0 7)
                (append-native-bytes-loop result (emit-mov-local-from-rsi (local-slot-offset 1)) 0 7)
                (append-native-bytes-loop result (emit-mov-local-from-rdx (local-slot-offset 2)) 0 7)
                (append-native-bytes-loop result (emit-mov-local-from-rcx (local-slot-offset 3)) 0 7)
                (append-native-bytes-loop result (emit-mov-local-from-r8 (local-slot-offset 4)) 0 7))
              (if (= param-count 4)
                (do
                  (append-native-bytes-loop result (emit-mov-local-from-rdi (local-slot-offset 0)) 0 7)
                  (append-native-bytes-loop result (emit-mov-local-from-rsi (local-slot-offset 1)) 0 7)
                  (append-native-bytes-loop result (emit-mov-local-from-rdx (local-slot-offset 2)) 0 7)
                  (append-native-bytes-loop result (emit-mov-local-from-rcx (local-slot-offset 3)) 0 7))
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
                      0))))))))))))))))))))
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

;; AArch64 MOV x2, x9
(defn emit-aarch64-mov-x2-x9 []
  (let [bytes (vector-new 4)]
    (vector-push (vector-push (vector-push (vector-push bytes 226) 3) 9) 170)))

;; AArch64 MOV x3, x0
(defn emit-aarch64-mov-x3-x0 []
  (let [bytes (vector-new 4)]
    (vector-push (vector-push (vector-push (vector-push bytes 227) 3) 0) 170)))

;; AArch64 MOV x3, x9
(defn emit-aarch64-mov-x3-x9 []
  (let [bytes (vector-new 4)]
    (vector-push (vector-push (vector-push (vector-push bytes 227) 3) 9) 170)))

;; AArch64 MOV x4, x0
(defn emit-aarch64-mov-x4-x0 []
  (let [bytes (vector-new 4)]
    (vector-push (vector-push (vector-push (vector-push bytes 228) 3) 0) 170)))

;; AArch64 MOV x4, x9
(defn emit-aarch64-mov-x4-x9 []
  (let [bytes (vector-new 4)]
    (vector-push (vector-push (vector-push (vector-push bytes 228) 3) 9) 170)))

;; AArch64 MOV x5, x0
(defn emit-aarch64-mov-x5-x0 []
  (let [bytes (vector-new 4)]
    (vector-push (vector-push (vector-push (vector-push bytes 229) 3) 0) 170)))

;; AArch64 MOV x5, x9
(defn emit-aarch64-mov-x5-x9 []
  (let [bytes (vector-new 4)]
    (vector-push (vector-push (vector-push (vector-push bytes 229) 3) 9) 170)))

;; AArch64 MOV x6, x0
(defn emit-aarch64-mov-x6-x0 []
  (let [bytes (vector-new 4)]
    (vector-push (vector-push (vector-push (vector-push bytes 230) 3) 0) 170)))

;; AArch64 MOV x6, x9
(defn emit-aarch64-mov-x6-x9 []
  (let [bytes (vector-new 4)]
    (vector-push (vector-push (vector-push (vector-push bytes 230) 3) 9) 170)))

;; AArch64 MOV x7, x0
(defn emit-aarch64-mov-x7-x0 []
  (let [bytes (vector-new 4)]
    (vector-push (vector-push (vector-push (vector-push bytes 231) 3) 0) 170)))

;; AArch64 MOV x7, x9
(defn emit-aarch64-mov-x7-x9 []
  (let [bytes (vector-new 4)]
    (vector-push (vector-push (vector-push (vector-push bytes 231) 3) 9) 170)))

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

;; AArch64 STR x3, [sp, #offset]
(defn emit-aarch64-str-x3-sp [offset]
  (let [scaled (/ offset 8)]
    (encode-u32-le (+ (+ 4177526787 (* scaled 1024)) 992))))

;; AArch64 STR x4, [sp, #offset]
(defn emit-aarch64-str-x4-sp [offset]
  (let [scaled (/ offset 8)]
    (encode-u32-le (+ (+ 4177526788 (* scaled 1024)) 992))))

;; AArch64 STR x5, [sp, #offset]
(defn emit-aarch64-str-x5-sp [offset]
  (let [scaled (/ offset 8)]
    (encode-u32-le (+ (+ 4177526789 (* scaled 1024)) 992))))

;; AArch64 STR x6, [sp, #offset]
(defn emit-aarch64-str-x6-sp [offset]
  (let [scaled (/ offset 8)]
    (encode-u32-le (+ (+ 4177526790 (* scaled 1024)) 992))))

;; AArch64 STR x7, [sp, #offset]
(defn emit-aarch64-str-x7-sp [offset]
  (let [scaled (/ offset 8)]
    (encode-u32-le (+ (+ 4177526791 (* scaled 1024)) 992))))

;; AArch64 STR x9, [sp, #offset]
(defn emit-aarch64-str-x9-sp [offset]
  (let [scaled (/ offset 8)]
    (encode-u32-le (+ (+ 4177526793 (* scaled 1024)) 992))))

;; AArch64 STR x10, [sp, #offset]
(defn emit-aarch64-str-x10-sp [offset]
  (let [scaled (/ offset 8)]
    (encode-u32-le (+ (+ 4177526794 (* scaled 1024)) 992))))

;; AArch64 LDR x0, [sp, #offset]
(defn emit-aarch64-ldr-x0-sp [offset]
  (let [scaled (/ offset 8)]
    (encode-u32-le (+ (+ 4181721088 (* scaled 1024)) 992))))

;; AArch64 LDR x1, [sp, #offset]
(defn emit-aarch64-ldr-x1-sp [offset]
  (let [scaled (/ offset 8)]
    (encode-u32-le (+ (+ 4181721089 (* scaled 1024)) 992))))

;; AArch64 LDR x2, [sp, #offset]
(defn emit-aarch64-ldr-x2-sp [offset]
  (let [scaled (/ offset 8)]
    (encode-u32-le (+ (+ 4181721090 (* scaled 1024)) 992))))

;; AArch64 LDR x3, [sp, #offset]
(defn emit-aarch64-ldr-x3-sp [offset]
  (let [scaled (/ offset 8)]
    (encode-u32-le (+ (+ 4181721091 (* scaled 1024)) 992))))

;; AArch64 LDR x4, [sp, #offset]
(defn emit-aarch64-ldr-x4-sp [offset]
  (let [scaled (/ offset 8)]
    (encode-u32-le (+ (+ 4181721092 (* scaled 1024)) 992))))

;; AArch64 LDR x5, [sp, #offset]
(defn emit-aarch64-ldr-x5-sp [offset]
  (let [scaled (/ offset 8)]
    (encode-u32-le (+ (+ 4181721093 (* scaled 1024)) 992))))

;; AArch64 LDR x6, [sp, #offset]
(defn emit-aarch64-ldr-x6-sp [offset]
  (let [scaled (/ offset 8)]
    (encode-u32-le (+ (+ 4181721094 (* scaled 1024)) 992))))

;; AArch64 LDR x7, [sp, #offset]
(defn emit-aarch64-ldr-x7-sp [offset]
  (let [scaled (/ offset 8)]
    (encode-u32-le (+ (+ 4181721095 (* scaled 1024)) 992))))

;; AArch64 LDR x9, [sp, #offset]
(defn emit-aarch64-ldr-x9-sp [offset]
  (let [scaled (/ offset 8)]
    (encode-u32-le (+ (+ 4181721097 (* scaled 1024)) 992))))

;; AArch64 LDR x10, [sp, #offset]
(defn emit-aarch64-ldr-x10-sp [offset]
  (let [scaled (/ offset 8)]
    (encode-u32-le (+ (+ 4181721098 (* scaled 1024)) 992))))

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

;; AArch64 bundle の i32.const: spill window が必要なら old previous を spill する
(defn emit-i32-const-bundle-aarch64 [value frame-base-slot-count current-depth]
  (if (>= current-depth 27)
    (concat-byte-vectors
      (concat-byte-vectors
        (emit-aarch64-ldr-x10-sp (native-value-window-spill-offset frame-base-slot-count 24))
        (emit-aarch64-str-x10-sp (native-value-window-spill-offset frame-base-slot-count 25)))
      (emit-i32-const-bundle-aarch64 value frame-base-slot-count 26))
    (if (>= current-depth 26)
    (concat-byte-vectors
      (concat-byte-vectors
        (emit-aarch64-ldr-x10-sp (native-value-window-spill-offset frame-base-slot-count 23))
        (emit-aarch64-str-x10-sp (native-value-window-spill-offset frame-base-slot-count 24)))
      (emit-i32-const-bundle-aarch64 value frame-base-slot-count 25))
    (if (>= current-depth 25)
    (concat-byte-vectors
      (concat-byte-vectors
        (emit-aarch64-ldr-x10-sp (native-value-window-spill-offset frame-base-slot-count 22))
        (emit-aarch64-str-x10-sp (native-value-window-spill-offset frame-base-slot-count 23)))
      (emit-i32-const-bundle-aarch64 value frame-base-slot-count 24))
    (if (>= current-depth 24)
    (concat-byte-vectors
      (concat-byte-vectors
        (emit-aarch64-ldr-x10-sp (native-value-window-spill-offset frame-base-slot-count 21))
        (emit-aarch64-str-x10-sp (native-value-window-spill-offset frame-base-slot-count 22)))
      (emit-i32-const-bundle-aarch64 value frame-base-slot-count 23))
    (if (>= current-depth 23)
    (concat-byte-vectors
      (concat-byte-vectors
        (emit-aarch64-ldr-x10-sp (native-value-window-spill-offset frame-base-slot-count 20))
        (emit-aarch64-str-x10-sp (native-value-window-spill-offset frame-base-slot-count 21)))
      (emit-i32-const-bundle-aarch64 value frame-base-slot-count 22))
    (if (>= current-depth 22)
    (concat-byte-vectors
      (concat-byte-vectors
        (emit-aarch64-ldr-x10-sp (native-value-window-spill-offset frame-base-slot-count 19))
        (emit-aarch64-str-x10-sp (native-value-window-spill-offset frame-base-slot-count 20)))
      (emit-i32-const-bundle-aarch64 value frame-base-slot-count 21))
    (if (>= current-depth 21)
    (concat-byte-vectors
      (concat-byte-vectors
        (emit-aarch64-ldr-x10-sp (native-value-window-spill-offset frame-base-slot-count 18))
        (emit-aarch64-str-x10-sp (native-value-window-spill-offset frame-base-slot-count 19)))
      (emit-i32-const-bundle-aarch64 value frame-base-slot-count 20))
    (if (>= current-depth 20)
    (concat-byte-vectors
      (concat-byte-vectors
        (emit-aarch64-ldr-x10-sp (native-value-window-spill-offset frame-base-slot-count 17))
        (emit-aarch64-str-x10-sp (native-value-window-spill-offset frame-base-slot-count 18)))
      (emit-i32-const-bundle-aarch64 value frame-base-slot-count 19))
    (if (>= current-depth 19)
    (concat-byte-vectors
      (concat-byte-vectors
        (emit-aarch64-ldr-x10-sp (native-value-window-spill-offset frame-base-slot-count 16))
        (emit-aarch64-str-x10-sp (native-value-window-spill-offset frame-base-slot-count 17)))
      (emit-i32-const-bundle-aarch64 value frame-base-slot-count 18))
    (if (>= current-depth 18)
    (concat-byte-vectors
      (concat-byte-vectors
        (emit-aarch64-ldr-x10-sp (native-value-window-spill-offset frame-base-slot-count 15))
        (emit-aarch64-str-x10-sp (native-value-window-spill-offset frame-base-slot-count 16)))
      (emit-i32-const-bundle-aarch64 value frame-base-slot-count 17))
    (if (>= current-depth 17)
    (concat-byte-vectors
      (concat-byte-vectors
        (emit-aarch64-ldr-x10-sp (native-value-window-spill-offset frame-base-slot-count 14))
        (emit-aarch64-str-x10-sp (native-value-window-spill-offset frame-base-slot-count 15)))
      (emit-i32-const-bundle-aarch64 value frame-base-slot-count 16))
    (if (>= current-depth 16)
    (concat-byte-vectors
      (concat-byte-vectors
        (emit-aarch64-ldr-x10-sp (native-value-window-spill-offset frame-base-slot-count 13))
        (emit-aarch64-str-x10-sp (native-value-window-spill-offset frame-base-slot-count 14)))
      (emit-i32-const-bundle-aarch64 value frame-base-slot-count 15))
    (if (>= current-depth 15)
    (concat-byte-vectors
      (concat-byte-vectors
        (emit-aarch64-ldr-x10-sp (native-value-window-spill-offset frame-base-slot-count 12))
        (emit-aarch64-str-x10-sp (native-value-window-spill-offset frame-base-slot-count 13)))
      (emit-i32-const-bundle-aarch64 value frame-base-slot-count 14))
    (if (>= current-depth 14)
    (concat-byte-vectors
      (concat-byte-vectors
        (emit-aarch64-ldr-x10-sp (native-value-window-spill-offset frame-base-slot-count 11))
        (emit-aarch64-str-x10-sp (native-value-window-spill-offset frame-base-slot-count 12)))
      (emit-i32-const-bundle-aarch64 value frame-base-slot-count 13))
    (if (>= current-depth 13)
    (concat-byte-vectors
      (concat-byte-vectors
        (emit-aarch64-ldr-x10-sp (native-value-window-spill-offset frame-base-slot-count 10))
        (emit-aarch64-str-x10-sp (native-value-window-spill-offset frame-base-slot-count 11)))
      (emit-i32-const-bundle-aarch64 value frame-base-slot-count 12))
    (if (>= current-depth 12)
    (concat-byte-vectors
      (concat-byte-vectors
        (emit-aarch64-ldr-x10-sp (native-value-window-spill-offset frame-base-slot-count 9))
        (emit-aarch64-str-x10-sp (native-value-window-spill-offset frame-base-slot-count 10)))
      (emit-i32-const-bundle-aarch64 value frame-base-slot-count 11))
    (if (>= current-depth 11)
    (concat-byte-vectors
      (concat-byte-vectors
        (emit-aarch64-ldr-x10-sp (native-value-window-spill-offset frame-base-slot-count 8))
        (emit-aarch64-str-x10-sp (native-value-window-spill-offset frame-base-slot-count 9)))
      (if (>= current-depth 10)
        (concat-byte-vectors
          (concat-byte-vectors
            (concat-byte-vectors
              (concat-byte-vectors
                (concat-byte-vectors
                  (concat-byte-vectors
                    (concat-byte-vectors
                      (concat-byte-vectors
                        (concat-byte-vectors
                          (concat-byte-vectors
                            (concat-byte-vectors
                              (concat-byte-vectors
                                (concat-byte-vectors
                                  (concat-byte-vectors
                                    (emit-aarch64-ldr-x10-sp (native-value-window-spill-offset frame-base-slot-count 7))
                                    (emit-aarch64-str-x10-sp (native-value-window-spill-offset frame-base-slot-count 8)))
                                  (emit-aarch64-ldr-x10-sp (native-value-window-spill-offset frame-base-slot-count 6)))
                                (emit-aarch64-str-x10-sp (native-value-window-spill-offset frame-base-slot-count 7)))
                              (emit-aarch64-ldr-x10-sp (native-value-window-spill-offset frame-base-slot-count 5)))
                            (emit-aarch64-str-x10-sp (native-value-window-spill-offset frame-base-slot-count 6)))
                          (emit-aarch64-ldr-x10-sp (native-value-window-spill-offset frame-base-slot-count 4)))
                        (emit-aarch64-str-x10-sp (native-value-window-spill-offset frame-base-slot-count 5)))
                      (emit-aarch64-ldr-x10-sp (native-value-window-spill-offset frame-base-slot-count 3)))
                    (emit-aarch64-str-x10-sp (native-value-window-spill-offset frame-base-slot-count 4)))
                  (emit-aarch64-ldr-x10-sp (native-value-window-spill-offset frame-base-slot-count 2)))
                (emit-aarch64-str-x10-sp (native-value-window-spill-offset frame-base-slot-count 3)))
              (emit-aarch64-ldr-x10-sp (native-value-window-spill-offset frame-base-slot-count 1)))
            (emit-aarch64-str-x10-sp (native-value-window-spill-offset frame-base-slot-count 2)))
          (concat-byte-vectors
            (emit-aarch64-ldr-x10-sp (native-value-window-spill-offset frame-base-slot-count 0))
            (concat-byte-vectors
              (emit-aarch64-str-x10-sp (native-value-window-spill-offset frame-base-slot-count 1))
              (concat-byte-vectors
                (emit-aarch64-str-x9-sp (native-value-window-spill-offset frame-base-slot-count 0))
                (emit-i32-const-aarch64 value)))))
        (emit-i32-const-aarch64 value)))
    (if (>= current-depth 10)
    (concat-byte-vectors
      (concat-byte-vectors
        (concat-byte-vectors
          (concat-byte-vectors
            (concat-byte-vectors
              (concat-byte-vectors
                (concat-byte-vectors
                  (concat-byte-vectors
                    (concat-byte-vectors
                      (concat-byte-vectors
                        (concat-byte-vectors
                          (concat-byte-vectors
                            (concat-byte-vectors
                              (concat-byte-vectors
                                (concat-byte-vectors
                                  (emit-aarch64-ldr-x10-sp (native-value-window-spill-offset frame-base-slot-count 7))
                                  (emit-aarch64-str-x10-sp (native-value-window-spill-offset frame-base-slot-count 8)))
                                (emit-aarch64-ldr-x10-sp (native-value-window-spill-offset frame-base-slot-count 6)))
                              (emit-aarch64-str-x10-sp (native-value-window-spill-offset frame-base-slot-count 7)))
                            (emit-aarch64-ldr-x10-sp (native-value-window-spill-offset frame-base-slot-count 5)))
                          (emit-aarch64-str-x10-sp (native-value-window-spill-offset frame-base-slot-count 6)))
                        (emit-aarch64-ldr-x10-sp (native-value-window-spill-offset frame-base-slot-count 4)))
                      (emit-aarch64-str-x10-sp (native-value-window-spill-offset frame-base-slot-count 5)))
                    (emit-aarch64-ldr-x10-sp (native-value-window-spill-offset frame-base-slot-count 3)))
                  (emit-aarch64-str-x10-sp (native-value-window-spill-offset frame-base-slot-count 4)))
                (emit-aarch64-ldr-x10-sp (native-value-window-spill-offset frame-base-slot-count 2)))
              (emit-aarch64-str-x10-sp (native-value-window-spill-offset frame-base-slot-count 3)))
            (emit-aarch64-ldr-x10-sp (native-value-window-spill-offset frame-base-slot-count 1)))
          (emit-aarch64-str-x10-sp (native-value-window-spill-offset frame-base-slot-count 2)))
        (emit-aarch64-ldr-x10-sp (native-value-window-spill-offset frame-base-slot-count 0)))
      (concat-byte-vectors
        (emit-aarch64-str-x10-sp (native-value-window-spill-offset frame-base-slot-count 1))
        (concat-byte-vectors
          (emit-aarch64-str-x9-sp (native-value-window-spill-offset frame-base-slot-count 0))
          (emit-i32-const-aarch64 value))))
    (if (>= current-depth 9)
    (concat-byte-vectors
      (concat-byte-vectors
        (concat-byte-vectors
          (concat-byte-vectors
            (concat-byte-vectors
              (concat-byte-vectors
                (concat-byte-vectors
                  (concat-byte-vectors
                    (concat-byte-vectors
                      (concat-byte-vectors
                        (concat-byte-vectors
                          (concat-byte-vectors
                            (concat-byte-vectors
                              (emit-aarch64-ldr-x10-sp (native-value-window-spill-offset frame-base-slot-count 6))
                              (emit-aarch64-str-x10-sp (native-value-window-spill-offset frame-base-slot-count 7)))
                            (emit-aarch64-ldr-x10-sp (native-value-window-spill-offset frame-base-slot-count 5)))
                          (emit-aarch64-str-x10-sp (native-value-window-spill-offset frame-base-slot-count 6)))
                        (emit-aarch64-ldr-x10-sp (native-value-window-spill-offset frame-base-slot-count 4)))
                      (emit-aarch64-str-x10-sp (native-value-window-spill-offset frame-base-slot-count 5)))
                    (emit-aarch64-ldr-x10-sp (native-value-window-spill-offset frame-base-slot-count 3)))
                  (emit-aarch64-str-x10-sp (native-value-window-spill-offset frame-base-slot-count 4)))
                (emit-aarch64-ldr-x10-sp (native-value-window-spill-offset frame-base-slot-count 2)))
              (emit-aarch64-str-x10-sp (native-value-window-spill-offset frame-base-slot-count 3)))
            (emit-aarch64-ldr-x10-sp (native-value-window-spill-offset frame-base-slot-count 1)))
          (emit-aarch64-str-x10-sp (native-value-window-spill-offset frame-base-slot-count 2)))
        (emit-aarch64-ldr-x10-sp (native-value-window-spill-offset frame-base-slot-count 0)))
      (concat-byte-vectors
        (emit-aarch64-str-x10-sp (native-value-window-spill-offset frame-base-slot-count 1))
        (concat-byte-vectors
          (emit-aarch64-str-x9-sp (native-value-window-spill-offset frame-base-slot-count 0))
          (emit-i32-const-aarch64 value))))
    (if (>= current-depth 8)
    (concat-byte-vectors
      (concat-byte-vectors
        (concat-byte-vectors
          (concat-byte-vectors
            (concat-byte-vectors
              (concat-byte-vectors
                (concat-byte-vectors
                  (concat-byte-vectors
                    (concat-byte-vectors
                      (concat-byte-vectors
                        (concat-byte-vectors
                          (emit-aarch64-ldr-x10-sp (native-value-window-spill-offset frame-base-slot-count 5))
                          (emit-aarch64-str-x10-sp (native-value-window-spill-offset frame-base-slot-count 6)))
                        (emit-aarch64-ldr-x10-sp (native-value-window-spill-offset frame-base-slot-count 4)))
                      (emit-aarch64-str-x10-sp (native-value-window-spill-offset frame-base-slot-count 5)))
                    (emit-aarch64-ldr-x10-sp (native-value-window-spill-offset frame-base-slot-count 3)))
                  (emit-aarch64-str-x10-sp (native-value-window-spill-offset frame-base-slot-count 4)))
                (emit-aarch64-ldr-x10-sp (native-value-window-spill-offset frame-base-slot-count 2)))
              (emit-aarch64-str-x10-sp (native-value-window-spill-offset frame-base-slot-count 3)))
            (emit-aarch64-ldr-x10-sp (native-value-window-spill-offset frame-base-slot-count 1)))
          (emit-aarch64-str-x10-sp (native-value-window-spill-offset frame-base-slot-count 2)))
        (emit-aarch64-ldr-x10-sp (native-value-window-spill-offset frame-base-slot-count 0)))
      (concat-byte-vectors
        (emit-aarch64-str-x10-sp (native-value-window-spill-offset frame-base-slot-count 1))
        (concat-byte-vectors
          (emit-aarch64-str-x9-sp (native-value-window-spill-offset frame-base-slot-count 0))
          (emit-i32-const-aarch64 value))))
    (if (>= current-depth 7)
    (concat-byte-vectors
      (concat-byte-vectors
        (concat-byte-vectors
          (concat-byte-vectors
            (concat-byte-vectors
              (concat-byte-vectors
                (concat-byte-vectors
                  (concat-byte-vectors
                    (concat-byte-vectors
                      (concat-byte-vectors
                        (emit-aarch64-ldr-x10-sp (native-value-window-spill-offset frame-base-slot-count 4))
                        (emit-aarch64-str-x10-sp (native-value-window-spill-offset frame-base-slot-count 5)))
                      (emit-aarch64-ldr-x10-sp (native-value-window-spill-offset frame-base-slot-count 3)))
                    (emit-aarch64-str-x10-sp (native-value-window-spill-offset frame-base-slot-count 4)))
                  (emit-aarch64-ldr-x10-sp (native-value-window-spill-offset frame-base-slot-count 2)))
                (emit-aarch64-str-x10-sp (native-value-window-spill-offset frame-base-slot-count 3)))
              (emit-aarch64-ldr-x10-sp (native-value-window-spill-offset frame-base-slot-count 1)))
            (emit-aarch64-str-x10-sp (native-value-window-spill-offset frame-base-slot-count 2)))
          (emit-aarch64-ldr-x10-sp (native-value-window-spill-offset frame-base-slot-count 0)))
        (emit-aarch64-str-x10-sp (native-value-window-spill-offset frame-base-slot-count 1)))
      (concat-byte-vectors
        (emit-aarch64-str-x9-sp (native-value-window-spill-offset frame-base-slot-count 0))
        (emit-i32-const-aarch64 value)))
    (if (>= current-depth 6)
    (concat-byte-vectors
      (concat-byte-vectors
        (concat-byte-vectors
          (concat-byte-vectors
            (concat-byte-vectors
              (concat-byte-vectors
                (concat-byte-vectors
                  (concat-byte-vectors
                    (concat-byte-vectors
                      (emit-aarch64-ldr-x10-sp (native-value-window-spill-offset frame-base-slot-count 3))
                      (emit-aarch64-str-x10-sp (native-value-window-spill-offset frame-base-slot-count 4)))
                    (emit-aarch64-ldr-x10-sp (native-value-window-spill-offset frame-base-slot-count 2)))
                  (emit-aarch64-str-x10-sp (native-value-window-spill-offset frame-base-slot-count 3)))
                (emit-aarch64-ldr-x10-sp (native-value-window-spill-offset frame-base-slot-count 1)))
              (emit-aarch64-str-x10-sp (native-value-window-spill-offset frame-base-slot-count 2)))
            (emit-aarch64-ldr-x10-sp (native-value-window-spill-offset frame-base-slot-count 0)))
          (emit-aarch64-str-x10-sp (native-value-window-spill-offset frame-base-slot-count 1)))
        (emit-aarch64-str-x9-sp (native-value-window-spill-offset frame-base-slot-count 0)))
      (emit-i32-const-aarch64 value))
    (if (>= current-depth 5)
    (concat-byte-vectors
      (concat-byte-vectors
        (concat-byte-vectors
          (concat-byte-vectors
            (concat-byte-vectors
              (concat-byte-vectors
                (concat-byte-vectors
                  (emit-aarch64-ldr-x10-sp (native-value-window-spill-offset frame-base-slot-count 2))
                  (emit-aarch64-str-x10-sp (native-value-window-spill-offset frame-base-slot-count 3)))
                (emit-aarch64-ldr-x10-sp (native-value-window-spill-offset frame-base-slot-count 1)))
              (emit-aarch64-str-x10-sp (native-value-window-spill-offset frame-base-slot-count 2)))
            (emit-aarch64-ldr-x10-sp (native-value-window-spill-offset frame-base-slot-count 0)))
          (emit-aarch64-str-x10-sp (native-value-window-spill-offset frame-base-slot-count 1)))
        (emit-aarch64-str-x9-sp (native-value-window-spill-offset frame-base-slot-count 0)))
      (emit-i32-const-aarch64 value))
    (if (>= current-depth 4)
    (concat-byte-vectors
      (concat-byte-vectors
        (concat-byte-vectors
          (concat-byte-vectors
            (concat-byte-vectors
              (emit-aarch64-ldr-x10-sp (native-value-window-spill-offset frame-base-slot-count 1))
              (emit-aarch64-str-x10-sp (native-value-window-spill-offset frame-base-slot-count 2)))
            (emit-aarch64-ldr-x10-sp (native-value-window-spill-offset frame-base-slot-count 0)))
          (emit-aarch64-str-x10-sp (native-value-window-spill-offset frame-base-slot-count 1)))
        (emit-aarch64-str-x9-sp (native-value-window-spill-offset frame-base-slot-count 0)))
      (emit-i32-const-aarch64 value))
    (if (>= current-depth 3)
      (concat-byte-vectors
        (concat-byte-vectors
          (concat-byte-vectors
            (emit-aarch64-ldr-x10-sp (native-value-window-spill-offset frame-base-slot-count 0))
            (emit-aarch64-str-x10-sp (native-value-window-spill-offset frame-base-slot-count 1)))
          (emit-aarch64-str-x9-sp (native-value-window-spill-offset frame-base-slot-count 0)))
        (emit-i32-const-aarch64 value))
      (if (>= current-depth 2)
        (concat-byte-vectors
          (emit-aarch64-str-x9-sp (native-value-window-spill-offset frame-base-slot-count 0))
          (emit-i32-const-aarch64 value))
           (emit-i32-const-aarch64 value))))))))))))))))))))))))))))

;; AArch64 bundle の local.get: spill window が必要なら old previous を spill する
(defn emit-local-get-bundle-aarch64 [offset frame-base-slot-count current-depth]
  (if (>= current-depth 27)
    (concat-byte-vectors
      (concat-byte-vectors
        (emit-aarch64-ldr-x10-sp (native-value-window-spill-offset frame-base-slot-count 24))
        (emit-aarch64-str-x10-sp (native-value-window-spill-offset frame-base-slot-count 25)))
      (emit-local-get-bundle-aarch64 offset frame-base-slot-count 26))
    (if (>= current-depth 26)
    (concat-byte-vectors
      (concat-byte-vectors
        (emit-aarch64-ldr-x10-sp (native-value-window-spill-offset frame-base-slot-count 23))
        (emit-aarch64-str-x10-sp (native-value-window-spill-offset frame-base-slot-count 24)))
      (emit-local-get-bundle-aarch64 offset frame-base-slot-count 25))
    (if (>= current-depth 25)
    (concat-byte-vectors
      (concat-byte-vectors
        (emit-aarch64-ldr-x10-sp (native-value-window-spill-offset frame-base-slot-count 22))
        (emit-aarch64-str-x10-sp (native-value-window-spill-offset frame-base-slot-count 23)))
      (emit-local-get-bundle-aarch64 offset frame-base-slot-count 24))
    (if (>= current-depth 24)
    (concat-byte-vectors
      (concat-byte-vectors
        (emit-aarch64-ldr-x10-sp (native-value-window-spill-offset frame-base-slot-count 21))
        (emit-aarch64-str-x10-sp (native-value-window-spill-offset frame-base-slot-count 22)))
      (emit-local-get-bundle-aarch64 offset frame-base-slot-count 23))
    (if (>= current-depth 23)
    (concat-byte-vectors
      (concat-byte-vectors
        (emit-aarch64-ldr-x10-sp (native-value-window-spill-offset frame-base-slot-count 20))
        (emit-aarch64-str-x10-sp (native-value-window-spill-offset frame-base-slot-count 21)))
      (emit-local-get-bundle-aarch64 offset frame-base-slot-count 22))
    (if (>= current-depth 22)
    (concat-byte-vectors
      (concat-byte-vectors
        (emit-aarch64-ldr-x10-sp (native-value-window-spill-offset frame-base-slot-count 19))
        (emit-aarch64-str-x10-sp (native-value-window-spill-offset frame-base-slot-count 20)))
      (emit-local-get-bundle-aarch64 offset frame-base-slot-count 21))
    (if (>= current-depth 21)
    (concat-byte-vectors
      (concat-byte-vectors
        (emit-aarch64-ldr-x10-sp (native-value-window-spill-offset frame-base-slot-count 18))
        (emit-aarch64-str-x10-sp (native-value-window-spill-offset frame-base-slot-count 19)))
      (emit-local-get-bundle-aarch64 offset frame-base-slot-count 20))
    (if (>= current-depth 20)
    (concat-byte-vectors
      (concat-byte-vectors
        (emit-aarch64-ldr-x10-sp (native-value-window-spill-offset frame-base-slot-count 17))
        (emit-aarch64-str-x10-sp (native-value-window-spill-offset frame-base-slot-count 18)))
      (emit-local-get-bundle-aarch64 offset frame-base-slot-count 19))
    (if (>= current-depth 19)
    (concat-byte-vectors
      (concat-byte-vectors
        (emit-aarch64-ldr-x10-sp (native-value-window-spill-offset frame-base-slot-count 16))
        (emit-aarch64-str-x10-sp (native-value-window-spill-offset frame-base-slot-count 17)))
      (emit-local-get-bundle-aarch64 offset frame-base-slot-count 18))
    (if (>= current-depth 18)
    (concat-byte-vectors
      (concat-byte-vectors
        (emit-aarch64-ldr-x10-sp (native-value-window-spill-offset frame-base-slot-count 15))
        (emit-aarch64-str-x10-sp (native-value-window-spill-offset frame-base-slot-count 16)))
      (emit-local-get-bundle-aarch64 offset frame-base-slot-count 17))
    (if (>= current-depth 17)
    (concat-byte-vectors
      (concat-byte-vectors
        (emit-aarch64-ldr-x10-sp (native-value-window-spill-offset frame-base-slot-count 14))
        (emit-aarch64-str-x10-sp (native-value-window-spill-offset frame-base-slot-count 15)))
      (emit-local-get-bundle-aarch64 offset frame-base-slot-count 16))
    (if (>= current-depth 16)
    (concat-byte-vectors
      (concat-byte-vectors
        (emit-aarch64-ldr-x10-sp (native-value-window-spill-offset frame-base-slot-count 13))
        (emit-aarch64-str-x10-sp (native-value-window-spill-offset frame-base-slot-count 14)))
      (emit-local-get-bundle-aarch64 offset frame-base-slot-count 15))
    (if (>= current-depth 15)
    (concat-byte-vectors
      (concat-byte-vectors
        (emit-aarch64-ldr-x10-sp (native-value-window-spill-offset frame-base-slot-count 12))
        (emit-aarch64-str-x10-sp (native-value-window-spill-offset frame-base-slot-count 13)))
      (emit-local-get-bundle-aarch64 offset frame-base-slot-count 14))
    (if (>= current-depth 14)
    (concat-byte-vectors
      (concat-byte-vectors
        (emit-aarch64-ldr-x10-sp (native-value-window-spill-offset frame-base-slot-count 11))
        (emit-aarch64-str-x10-sp (native-value-window-spill-offset frame-base-slot-count 12)))
      (emit-local-get-bundle-aarch64 offset frame-base-slot-count 13))
    (if (>= current-depth 13)
    (concat-byte-vectors
      (concat-byte-vectors
        (emit-aarch64-ldr-x10-sp (native-value-window-spill-offset frame-base-slot-count 10))
        (emit-aarch64-str-x10-sp (native-value-window-spill-offset frame-base-slot-count 11)))
      (emit-local-get-bundle-aarch64 offset frame-base-slot-count 12))
    (if (>= current-depth 12)
    (concat-byte-vectors
      (concat-byte-vectors
        (emit-aarch64-ldr-x10-sp (native-value-window-spill-offset frame-base-slot-count 9))
        (emit-aarch64-str-x10-sp (native-value-window-spill-offset frame-base-slot-count 10)))
      (emit-local-get-bundle-aarch64 offset frame-base-slot-count 11))
    (if (>= current-depth 11)
    (concat-byte-vectors
      (concat-byte-vectors
        (emit-aarch64-ldr-x10-sp (native-value-window-spill-offset frame-base-slot-count 8))
        (emit-aarch64-str-x10-sp (native-value-window-spill-offset frame-base-slot-count 9)))
      (if (>= current-depth 10)
        (concat-byte-vectors
          (concat-byte-vectors
            (concat-byte-vectors
              (concat-byte-vectors
                (concat-byte-vectors
                  (concat-byte-vectors
                    (concat-byte-vectors
                      (concat-byte-vectors
                        (concat-byte-vectors
                          (concat-byte-vectors
                            (concat-byte-vectors
                              (concat-byte-vectors
                                (concat-byte-vectors
                                  (concat-byte-vectors
                                    (emit-aarch64-ldr-x10-sp (native-value-window-spill-offset frame-base-slot-count 7))
                                    (emit-aarch64-str-x10-sp (native-value-window-spill-offset frame-base-slot-count 8)))
                                  (emit-aarch64-ldr-x10-sp (native-value-window-spill-offset frame-base-slot-count 6)))
                                (emit-aarch64-str-x10-sp (native-value-window-spill-offset frame-base-slot-count 7)))
                              (emit-aarch64-ldr-x10-sp (native-value-window-spill-offset frame-base-slot-count 5)))
                            (emit-aarch64-str-x10-sp (native-value-window-spill-offset frame-base-slot-count 6)))
                          (emit-aarch64-ldr-x10-sp (native-value-window-spill-offset frame-base-slot-count 4)))
                        (emit-aarch64-str-x10-sp (native-value-window-spill-offset frame-base-slot-count 5)))
                      (emit-aarch64-ldr-x10-sp (native-value-window-spill-offset frame-base-slot-count 3)))
                    (emit-aarch64-str-x10-sp (native-value-window-spill-offset frame-base-slot-count 4)))
                  (emit-aarch64-ldr-x10-sp (native-value-window-spill-offset frame-base-slot-count 2)))
                (emit-aarch64-str-x10-sp (native-value-window-spill-offset frame-base-slot-count 3)))
              (emit-aarch64-ldr-x10-sp (native-value-window-spill-offset frame-base-slot-count 1)))
            (emit-aarch64-str-x10-sp (native-value-window-spill-offset frame-base-slot-count 2)))
          (concat-byte-vectors
            (emit-aarch64-ldr-x10-sp (native-value-window-spill-offset frame-base-slot-count 0))
            (concat-byte-vectors
              (emit-aarch64-str-x10-sp (native-value-window-spill-offset frame-base-slot-count 1))
              (concat-byte-vectors
                (emit-aarch64-str-x9-sp (native-value-window-spill-offset frame-base-slot-count 0))
                (emit-local-get-aarch64 offset)))))
        (emit-local-get-aarch64 offset)))
    (if (>= current-depth 10)
    (concat-byte-vectors
      (concat-byte-vectors
        (concat-byte-vectors
          (concat-byte-vectors
            (concat-byte-vectors
              (concat-byte-vectors
                (concat-byte-vectors
                  (concat-byte-vectors
                    (concat-byte-vectors
                      (concat-byte-vectors
                        (concat-byte-vectors
                          (concat-byte-vectors
                            (concat-byte-vectors
                              (concat-byte-vectors
                                (concat-byte-vectors
                                  (emit-aarch64-ldr-x10-sp (native-value-window-spill-offset frame-base-slot-count 7))
                                  (emit-aarch64-str-x10-sp (native-value-window-spill-offset frame-base-slot-count 8)))
                                (emit-aarch64-ldr-x10-sp (native-value-window-spill-offset frame-base-slot-count 6)))
                              (emit-aarch64-str-x10-sp (native-value-window-spill-offset frame-base-slot-count 7)))
                            (emit-aarch64-ldr-x10-sp (native-value-window-spill-offset frame-base-slot-count 5)))
                          (emit-aarch64-str-x10-sp (native-value-window-spill-offset frame-base-slot-count 6)))
                        (emit-aarch64-ldr-x10-sp (native-value-window-spill-offset frame-base-slot-count 4)))
                      (emit-aarch64-str-x10-sp (native-value-window-spill-offset frame-base-slot-count 5)))
                    (emit-aarch64-ldr-x10-sp (native-value-window-spill-offset frame-base-slot-count 3)))
                  (emit-aarch64-str-x10-sp (native-value-window-spill-offset frame-base-slot-count 4)))
                (emit-aarch64-ldr-x10-sp (native-value-window-spill-offset frame-base-slot-count 2)))
              (emit-aarch64-str-x10-sp (native-value-window-spill-offset frame-base-slot-count 3)))
            (emit-aarch64-ldr-x10-sp (native-value-window-spill-offset frame-base-slot-count 1)))
          (emit-aarch64-str-x10-sp (native-value-window-spill-offset frame-base-slot-count 2)))
        (emit-aarch64-ldr-x10-sp (native-value-window-spill-offset frame-base-slot-count 0)))
      (concat-byte-vectors
        (emit-aarch64-str-x10-sp (native-value-window-spill-offset frame-base-slot-count 1))
        (concat-byte-vectors
          (emit-aarch64-str-x9-sp (native-value-window-spill-offset frame-base-slot-count 0))
          (emit-local-get-aarch64 offset))))
    (if (>= current-depth 9)
    (concat-byte-vectors
      (concat-byte-vectors
        (concat-byte-vectors
          (concat-byte-vectors
            (concat-byte-vectors
              (concat-byte-vectors
                (concat-byte-vectors
                  (concat-byte-vectors
                    (concat-byte-vectors
                      (concat-byte-vectors
                        (concat-byte-vectors
                          (concat-byte-vectors
                            (concat-byte-vectors
                              (emit-aarch64-ldr-x10-sp (native-value-window-spill-offset frame-base-slot-count 6))
                              (emit-aarch64-str-x10-sp (native-value-window-spill-offset frame-base-slot-count 7)))
                            (emit-aarch64-ldr-x10-sp (native-value-window-spill-offset frame-base-slot-count 5)))
                          (emit-aarch64-str-x10-sp (native-value-window-spill-offset frame-base-slot-count 6)))
                        (emit-aarch64-ldr-x10-sp (native-value-window-spill-offset frame-base-slot-count 4)))
                      (emit-aarch64-str-x10-sp (native-value-window-spill-offset frame-base-slot-count 5)))
                    (emit-aarch64-ldr-x10-sp (native-value-window-spill-offset frame-base-slot-count 3)))
                  (emit-aarch64-str-x10-sp (native-value-window-spill-offset frame-base-slot-count 4)))
                (emit-aarch64-ldr-x10-sp (native-value-window-spill-offset frame-base-slot-count 2)))
              (emit-aarch64-str-x10-sp (native-value-window-spill-offset frame-base-slot-count 3)))
            (emit-aarch64-ldr-x10-sp (native-value-window-spill-offset frame-base-slot-count 1)))
          (emit-aarch64-str-x10-sp (native-value-window-spill-offset frame-base-slot-count 2)))
        (emit-aarch64-ldr-x10-sp (native-value-window-spill-offset frame-base-slot-count 0)))
      (concat-byte-vectors
        (emit-aarch64-str-x10-sp (native-value-window-spill-offset frame-base-slot-count 1))
        (concat-byte-vectors
          (emit-aarch64-str-x9-sp (native-value-window-spill-offset frame-base-slot-count 0))
          (emit-local-get-aarch64 offset))))
    (if (>= current-depth 8)
    (concat-byte-vectors
      (concat-byte-vectors
        (concat-byte-vectors
          (concat-byte-vectors
            (concat-byte-vectors
              (concat-byte-vectors
                (concat-byte-vectors
                  (concat-byte-vectors
                    (concat-byte-vectors
                      (concat-byte-vectors
                        (concat-byte-vectors
                          (emit-aarch64-ldr-x10-sp (native-value-window-spill-offset frame-base-slot-count 5))
                          (emit-aarch64-str-x10-sp (native-value-window-spill-offset frame-base-slot-count 6)))
                        (emit-aarch64-ldr-x10-sp (native-value-window-spill-offset frame-base-slot-count 4)))
                      (emit-aarch64-str-x10-sp (native-value-window-spill-offset frame-base-slot-count 5)))
                    (emit-aarch64-ldr-x10-sp (native-value-window-spill-offset frame-base-slot-count 3)))
                  (emit-aarch64-str-x10-sp (native-value-window-spill-offset frame-base-slot-count 4)))
                (emit-aarch64-ldr-x10-sp (native-value-window-spill-offset frame-base-slot-count 2)))
              (emit-aarch64-str-x10-sp (native-value-window-spill-offset frame-base-slot-count 3)))
            (emit-aarch64-ldr-x10-sp (native-value-window-spill-offset frame-base-slot-count 1)))
          (emit-aarch64-str-x10-sp (native-value-window-spill-offset frame-base-slot-count 2)))
        (emit-aarch64-ldr-x10-sp (native-value-window-spill-offset frame-base-slot-count 0)))
      (concat-byte-vectors
        (emit-aarch64-str-x10-sp (native-value-window-spill-offset frame-base-slot-count 1))
        (concat-byte-vectors
          (emit-aarch64-str-x9-sp (native-value-window-spill-offset frame-base-slot-count 0))
          (emit-local-get-aarch64 offset))))
    (if (>= current-depth 7)
    (concat-byte-vectors
      (concat-byte-vectors
        (concat-byte-vectors
          (concat-byte-vectors
            (concat-byte-vectors
              (concat-byte-vectors
                (concat-byte-vectors
                  (concat-byte-vectors
                    (concat-byte-vectors
                      (concat-byte-vectors
                        (emit-aarch64-ldr-x10-sp (native-value-window-spill-offset frame-base-slot-count 4))
                        (emit-aarch64-str-x10-sp (native-value-window-spill-offset frame-base-slot-count 5)))
                      (emit-aarch64-ldr-x10-sp (native-value-window-spill-offset frame-base-slot-count 3)))
                    (emit-aarch64-str-x10-sp (native-value-window-spill-offset frame-base-slot-count 4)))
                  (emit-aarch64-ldr-x10-sp (native-value-window-spill-offset frame-base-slot-count 2)))
                (emit-aarch64-str-x10-sp (native-value-window-spill-offset frame-base-slot-count 3)))
              (emit-aarch64-ldr-x10-sp (native-value-window-spill-offset frame-base-slot-count 1)))
            (emit-aarch64-str-x10-sp (native-value-window-spill-offset frame-base-slot-count 2)))
          (emit-aarch64-ldr-x10-sp (native-value-window-spill-offset frame-base-slot-count 0)))
        (emit-aarch64-str-x10-sp (native-value-window-spill-offset frame-base-slot-count 1)))
      (concat-byte-vectors
        (emit-aarch64-str-x9-sp (native-value-window-spill-offset frame-base-slot-count 0))
        (emit-local-get-aarch64 offset)))
    (if (>= current-depth 6)
    (concat-byte-vectors
      (concat-byte-vectors
        (concat-byte-vectors
          (concat-byte-vectors
            (concat-byte-vectors
              (concat-byte-vectors
                (concat-byte-vectors
                  (concat-byte-vectors
                    (concat-byte-vectors
                      (emit-aarch64-ldr-x10-sp (native-value-window-spill-offset frame-base-slot-count 3))
                      (emit-aarch64-str-x10-sp (native-value-window-spill-offset frame-base-slot-count 4)))
                    (emit-aarch64-ldr-x10-sp (native-value-window-spill-offset frame-base-slot-count 2)))
                  (emit-aarch64-str-x10-sp (native-value-window-spill-offset frame-base-slot-count 3)))
                (emit-aarch64-ldr-x10-sp (native-value-window-spill-offset frame-base-slot-count 1)))
              (emit-aarch64-str-x10-sp (native-value-window-spill-offset frame-base-slot-count 2)))
            (emit-aarch64-ldr-x10-sp (native-value-window-spill-offset frame-base-slot-count 0)))
          (emit-aarch64-str-x10-sp (native-value-window-spill-offset frame-base-slot-count 1)))
        (emit-aarch64-str-x9-sp (native-value-window-spill-offset frame-base-slot-count 0)))
      (emit-local-get-aarch64 offset))
    (if (>= current-depth 5)
    (concat-byte-vectors
      (concat-byte-vectors
        (concat-byte-vectors
          (concat-byte-vectors
            (concat-byte-vectors
              (concat-byte-vectors
                (concat-byte-vectors
                  (emit-aarch64-ldr-x10-sp (native-value-window-spill-offset frame-base-slot-count 2))
                  (emit-aarch64-str-x10-sp (native-value-window-spill-offset frame-base-slot-count 3)))
                (emit-aarch64-ldr-x10-sp (native-value-window-spill-offset frame-base-slot-count 1)))
              (emit-aarch64-str-x10-sp (native-value-window-spill-offset frame-base-slot-count 2)))
            (emit-aarch64-ldr-x10-sp (native-value-window-spill-offset frame-base-slot-count 0)))
          (emit-aarch64-str-x10-sp (native-value-window-spill-offset frame-base-slot-count 1)))
        (emit-aarch64-str-x9-sp (native-value-window-spill-offset frame-base-slot-count 0)))
      (emit-local-get-aarch64 offset))
    (if (>= current-depth 4)
    (concat-byte-vectors
      (concat-byte-vectors
        (concat-byte-vectors
          (concat-byte-vectors
            (concat-byte-vectors
              (emit-aarch64-ldr-x10-sp (native-value-window-spill-offset frame-base-slot-count 1))
              (emit-aarch64-str-x10-sp (native-value-window-spill-offset frame-base-slot-count 2)))
            (emit-aarch64-ldr-x10-sp (native-value-window-spill-offset frame-base-slot-count 0)))
          (emit-aarch64-str-x10-sp (native-value-window-spill-offset frame-base-slot-count 1)))
        (emit-aarch64-str-x9-sp (native-value-window-spill-offset frame-base-slot-count 0)))
      (emit-local-get-aarch64 offset))
    (if (>= current-depth 3)
      (concat-byte-vectors
        (concat-byte-vectors
          (concat-byte-vectors
            (emit-aarch64-ldr-x10-sp (native-value-window-spill-offset frame-base-slot-count 0))
            (emit-aarch64-str-x10-sp (native-value-window-spill-offset frame-base-slot-count 1)))
          (emit-aarch64-str-x9-sp (native-value-window-spill-offset frame-base-slot-count 0)))
        (emit-local-get-aarch64 offset))
      (if (>= current-depth 2)
        (concat-byte-vectors
          (emit-aarch64-str-x9-sp (native-value-window-spill-offset frame-base-slot-count 0))
          (emit-local-get-aarch64 offset))
          (emit-local-get-aarch64 offset))))))))))))))))))))))))))))

(defn emit-twenty-six-arg-call-aarch64 [disp frame-base-slot-count]
  (let [stack0 (concat-byte-vectors
                 (emit-aarch64-sub-sp 144)
                 (emit-aarch64-ldr-x10-sp (+ 144 (native-value-window-spill-offset frame-base-slot-count 15))))
        stack1 (concat-byte-vectors stack0 (emit-aarch64-str-x10-sp 0))
        stack2 (concat-byte-vectors
                    stack1
                    (emit-aarch64-ldr-x10-sp (+ 144 (native-value-window-spill-offset frame-base-slot-count 14))))
        stack3 (concat-byte-vectors stack2 (emit-aarch64-str-x10-sp 8))
        stack4 (concat-byte-vectors
                    stack3
                    (emit-aarch64-ldr-x10-sp (+ 144 (native-value-window-spill-offset frame-base-slot-count 13))))
        stack5 (concat-byte-vectors stack4 (emit-aarch64-str-x10-sp 16))
        stack6 (concat-byte-vectors
                    stack5
                    (emit-aarch64-ldr-x10-sp (+ 144 (native-value-window-spill-offset frame-base-slot-count 12))))
        stack7 (concat-byte-vectors stack6 (emit-aarch64-str-x10-sp 24))
        stack8 (concat-byte-vectors
                    stack7
                    (emit-aarch64-ldr-x10-sp (+ 144 (native-value-window-spill-offset frame-base-slot-count 11))))
        stack9 (concat-byte-vectors stack8 (emit-aarch64-str-x10-sp 32))
        stack10 (concat-byte-vectors
                     stack9
                     (emit-aarch64-ldr-x10-sp (+ 144 (native-value-window-spill-offset frame-base-slot-count 10))))
        stack11 (concat-byte-vectors stack10 (emit-aarch64-str-x10-sp 40))
        stack12 (concat-byte-vectors
                     stack11
                     (emit-aarch64-ldr-x10-sp (+ 144 (native-value-window-spill-offset frame-base-slot-count 9))))
        stack13 (concat-byte-vectors stack12 (emit-aarch64-str-x10-sp 48))
        stack14 (concat-byte-vectors
                     stack13
                     (emit-aarch64-ldr-x10-sp (+ 144 (native-value-window-spill-offset frame-base-slot-count 8))))
        stack15 (concat-byte-vectors stack14 (emit-aarch64-str-x10-sp 56))
        stack16 (concat-byte-vectors
                     stack15
                     (emit-aarch64-ldr-x10-sp (+ 144 (native-value-window-spill-offset frame-base-slot-count 7))))
        stack17 (concat-byte-vectors stack16 (emit-aarch64-str-x10-sp 64))
        stack18 (concat-byte-vectors
                     stack17
                     (emit-aarch64-ldr-x10-sp (+ 144 (native-value-window-spill-offset frame-base-slot-count 6))))
        stack19 (concat-byte-vectors stack18 (emit-aarch64-str-x10-sp 72))
        stack20 (concat-byte-vectors
                     stack19
                     (emit-aarch64-ldr-x10-sp (+ 144 (native-value-window-spill-offset frame-base-slot-count 5))))
        stack21 (concat-byte-vectors stack20 (emit-aarch64-str-x10-sp 80))
        stack22 (concat-byte-vectors
                     stack21
                     (emit-aarch64-ldr-x10-sp (+ 144 (native-value-window-spill-offset frame-base-slot-count 4))))
        stack23 (concat-byte-vectors stack22 (emit-aarch64-str-x10-sp 88))
        stack24 (concat-byte-vectors
                     stack23
                     (emit-aarch64-ldr-x10-sp (+ 144 (native-value-window-spill-offset frame-base-slot-count 3))))
        stack25 (concat-byte-vectors stack24 (emit-aarch64-str-x10-sp 96))
        stack26 (concat-byte-vectors
                     stack25
                     (emit-aarch64-ldr-x10-sp (+ 144 (native-value-window-spill-offset frame-base-slot-count 2))))
        stack27 (concat-byte-vectors stack26 (emit-aarch64-str-x10-sp 104))
        stack28 (concat-byte-vectors
                     stack27
                     (emit-aarch64-ldr-x10-sp (+ 144 (native-value-window-spill-offset frame-base-slot-count 1))))
        stack29 (concat-byte-vectors stack28 (emit-aarch64-str-x10-sp 112))
        stack30 (concat-byte-vectors
                     stack29
                     (emit-aarch64-ldr-x10-sp (+ 144 (native-value-window-spill-offset frame-base-slot-count 0))))
        stack31 (concat-byte-vectors stack30 (emit-aarch64-str-x10-sp 120))
        stack32 (concat-byte-vectors stack31 (emit-aarch64-str-x9-sp 128))
        stack33 (concat-byte-vectors stack32 (emit-aarch64-str-x0-sp 136))
        reg0 (concat-byte-vectors stack33 (emit-aarch64-ldr-x7-sp (+ 144 (native-value-window-spill-offset frame-base-slot-count 16))))
        reg1 (concat-byte-vectors reg0 (emit-aarch64-ldr-x6-sp (+ 144 (native-value-window-spill-offset frame-base-slot-count 17))))
        reg2 (concat-byte-vectors reg1 (emit-aarch64-ldr-x5-sp (+ 144 (native-value-window-spill-offset frame-base-slot-count 18))))
        reg3 (concat-byte-vectors reg2 (emit-aarch64-ldr-x4-sp (+ 144 (native-value-window-spill-offset frame-base-slot-count 19))))
        reg4 (concat-byte-vectors reg3 (emit-aarch64-ldr-x3-sp (+ 144 (native-value-window-spill-offset frame-base-slot-count 20))))
        reg5 (concat-byte-vectors reg4 (emit-aarch64-ldr-x2-sp (+ 144 (native-value-window-spill-offset frame-base-slot-count 21))))
        reg6 (concat-byte-vectors reg5 (emit-aarch64-ldr-x1-sp (+ 144 (native-value-window-spill-offset frame-base-slot-count 22))))
        reg7 (concat-byte-vectors reg6 (emit-aarch64-ldr-x0-sp (+ 144 (native-value-window-spill-offset frame-base-slot-count 23))))
        call-seq (concat-byte-vectors
                   (emit-aarch64-bl disp)
                   (emit-aarch64-add-sp 144))]
    (concat-byte-vectors reg7 call-seq)))

(defn emit-twenty-seven-arg-call-aarch64 [disp frame-base-slot-count]
  (let [stack0 (concat-byte-vectors
                 (emit-aarch64-sub-sp 160)
                 (emit-aarch64-ldr-x10-sp (+ 160 (native-value-window-spill-offset frame-base-slot-count 16))))
        stack1 (concat-byte-vectors stack0 (emit-aarch64-str-x10-sp 0))
        stack2 (concat-byte-vectors
                    stack1
                    (emit-aarch64-ldr-x10-sp (+ 160 (native-value-window-spill-offset frame-base-slot-count 15))))
        stack3 (concat-byte-vectors stack2 (emit-aarch64-str-x10-sp 8))
        stack4 (concat-byte-vectors
                    stack3
                    (emit-aarch64-ldr-x10-sp (+ 160 (native-value-window-spill-offset frame-base-slot-count 14))))
        stack5 (concat-byte-vectors stack4 (emit-aarch64-str-x10-sp 16))
        stack6 (concat-byte-vectors
                    stack5
                    (emit-aarch64-ldr-x10-sp (+ 160 (native-value-window-spill-offset frame-base-slot-count 13))))
        stack7 (concat-byte-vectors stack6 (emit-aarch64-str-x10-sp 24))
        stack8 (concat-byte-vectors
                    stack7
                    (emit-aarch64-ldr-x10-sp (+ 160 (native-value-window-spill-offset frame-base-slot-count 12))))
        stack9 (concat-byte-vectors stack8 (emit-aarch64-str-x10-sp 32))
        stack10 (concat-byte-vectors
                     stack9
                     (emit-aarch64-ldr-x10-sp (+ 160 (native-value-window-spill-offset frame-base-slot-count 11))))
        stack11 (concat-byte-vectors stack10 (emit-aarch64-str-x10-sp 40))
        stack12 (concat-byte-vectors
                     stack11
                     (emit-aarch64-ldr-x10-sp (+ 160 (native-value-window-spill-offset frame-base-slot-count 10))))
        stack13 (concat-byte-vectors stack12 (emit-aarch64-str-x10-sp 48))
        stack14 (concat-byte-vectors
                     stack13
                     (emit-aarch64-ldr-x10-sp (+ 160 (native-value-window-spill-offset frame-base-slot-count 9))))
        stack15 (concat-byte-vectors stack14 (emit-aarch64-str-x10-sp 56))
        stack16 (concat-byte-vectors
                     stack15
                     (emit-aarch64-ldr-x10-sp (+ 160 (native-value-window-spill-offset frame-base-slot-count 8))))
        stack17 (concat-byte-vectors stack16 (emit-aarch64-str-x10-sp 64))
        stack18 (concat-byte-vectors
                     stack17
                     (emit-aarch64-ldr-x10-sp (+ 160 (native-value-window-spill-offset frame-base-slot-count 7))))
        stack19 (concat-byte-vectors stack18 (emit-aarch64-str-x10-sp 72))
        stack20 (concat-byte-vectors
                     stack19
                     (emit-aarch64-ldr-x10-sp (+ 160 (native-value-window-spill-offset frame-base-slot-count 6))))
        stack21 (concat-byte-vectors stack20 (emit-aarch64-str-x10-sp 80))
        stack22 (concat-byte-vectors
                     stack21
                     (emit-aarch64-ldr-x10-sp (+ 160 (native-value-window-spill-offset frame-base-slot-count 5))))
        stack23 (concat-byte-vectors stack22 (emit-aarch64-str-x10-sp 88))
        stack24 (concat-byte-vectors
                     stack23
                     (emit-aarch64-ldr-x10-sp (+ 160 (native-value-window-spill-offset frame-base-slot-count 4))))
        stack25 (concat-byte-vectors stack24 (emit-aarch64-str-x10-sp 96))
        stack26 (concat-byte-vectors
                     stack25
                     (emit-aarch64-ldr-x10-sp (+ 160 (native-value-window-spill-offset frame-base-slot-count 3))))
        stack27 (concat-byte-vectors stack26 (emit-aarch64-str-x10-sp 104))
        stack28 (concat-byte-vectors
                     stack27
                     (emit-aarch64-ldr-x10-sp (+ 160 (native-value-window-spill-offset frame-base-slot-count 2))))
        stack29 (concat-byte-vectors stack28 (emit-aarch64-str-x10-sp 112))
        stack30 (concat-byte-vectors
                     stack29
                     (emit-aarch64-ldr-x10-sp (+ 160 (native-value-window-spill-offset frame-base-slot-count 1))))
        stack31 (concat-byte-vectors stack30 (emit-aarch64-str-x10-sp 120))
        stack32 (concat-byte-vectors
                     stack31
                     (emit-aarch64-ldr-x10-sp (+ 160 (native-value-window-spill-offset frame-base-slot-count 0))))
        stack33 (concat-byte-vectors stack32 (emit-aarch64-str-x10-sp 128))
        stack34 (concat-byte-vectors stack33 (emit-aarch64-str-x9-sp 136))
        stack35 (concat-byte-vectors stack34 (emit-aarch64-str-x0-sp 144))
        reg0 (concat-byte-vectors stack35 (emit-aarch64-ldr-x7-sp (+ 160 (native-value-window-spill-offset frame-base-slot-count 17))))
        reg1 (concat-byte-vectors reg0 (emit-aarch64-ldr-x6-sp (+ 160 (native-value-window-spill-offset frame-base-slot-count 18))))
        reg2 (concat-byte-vectors reg1 (emit-aarch64-ldr-x5-sp (+ 160 (native-value-window-spill-offset frame-base-slot-count 19))))
        reg3 (concat-byte-vectors reg2 (emit-aarch64-ldr-x4-sp (+ 160 (native-value-window-spill-offset frame-base-slot-count 20))))
        reg4 (concat-byte-vectors reg3 (emit-aarch64-ldr-x3-sp (+ 160 (native-value-window-spill-offset frame-base-slot-count 21))))
        reg5 (concat-byte-vectors reg4 (emit-aarch64-ldr-x2-sp (+ 160 (native-value-window-spill-offset frame-base-slot-count 22))))
        reg6 (concat-byte-vectors reg5 (emit-aarch64-ldr-x1-sp (+ 160 (native-value-window-spill-offset frame-base-slot-count 23))))
        reg7 (concat-byte-vectors reg6 (emit-aarch64-ldr-x0-sp (+ 160 (native-value-window-spill-offset frame-base-slot-count 24))))
        call-seq (concat-byte-vectors
                   (emit-aarch64-bl disp)
                   (emit-aarch64-add-sp 160))]
    (concat-byte-vectors reg7 call-seq)))

(defn emit-twenty-eight-arg-call-aarch64 [disp frame-base-slot-count]
  (let [stack0 (concat-byte-vectors
                 (emit-aarch64-sub-sp 160)
                 (emit-aarch64-ldr-x10-sp (+ 160 (native-value-window-spill-offset frame-base-slot-count 17))))
        stack1 (concat-byte-vectors stack0 (emit-aarch64-str-x10-sp 0))
        stack2 (concat-byte-vectors
                    stack1
                    (emit-aarch64-ldr-x10-sp (+ 160 (native-value-window-spill-offset frame-base-slot-count 16))))
        stack3 (concat-byte-vectors stack2 (emit-aarch64-str-x10-sp 8))
        stack4 (concat-byte-vectors
                    stack3
                    (emit-aarch64-ldr-x10-sp (+ 160 (native-value-window-spill-offset frame-base-slot-count 15))))
        stack5 (concat-byte-vectors stack4 (emit-aarch64-str-x10-sp 16))
        stack6 (concat-byte-vectors
                    stack5
                    (emit-aarch64-ldr-x10-sp (+ 160 (native-value-window-spill-offset frame-base-slot-count 14))))
        stack7 (concat-byte-vectors stack6 (emit-aarch64-str-x10-sp 24))
        stack8 (concat-byte-vectors
                    stack7
                    (emit-aarch64-ldr-x10-sp (+ 160 (native-value-window-spill-offset frame-base-slot-count 13))))
        stack9 (concat-byte-vectors stack8 (emit-aarch64-str-x10-sp 32))
        stack10 (concat-byte-vectors
                     stack9
                     (emit-aarch64-ldr-x10-sp (+ 160 (native-value-window-spill-offset frame-base-slot-count 12))))
        stack11 (concat-byte-vectors stack10 (emit-aarch64-str-x10-sp 40))
        stack12 (concat-byte-vectors
                     stack11
                     (emit-aarch64-ldr-x10-sp (+ 160 (native-value-window-spill-offset frame-base-slot-count 11))))
        stack13 (concat-byte-vectors stack12 (emit-aarch64-str-x10-sp 48))
        stack14 (concat-byte-vectors
                     stack13
                     (emit-aarch64-ldr-x10-sp (+ 160 (native-value-window-spill-offset frame-base-slot-count 10))))
        stack15 (concat-byte-vectors stack14 (emit-aarch64-str-x10-sp 56))
        stack16 (concat-byte-vectors
                     stack15
                     (emit-aarch64-ldr-x10-sp (+ 160 (native-value-window-spill-offset frame-base-slot-count 9))))
        stack17 (concat-byte-vectors stack16 (emit-aarch64-str-x10-sp 64))
        stack18 (concat-byte-vectors
                     stack17
                     (emit-aarch64-ldr-x10-sp (+ 160 (native-value-window-spill-offset frame-base-slot-count 8))))
        stack19 (concat-byte-vectors stack18 (emit-aarch64-str-x10-sp 72))
        stack20 (concat-byte-vectors
                     stack19
                     (emit-aarch64-ldr-x10-sp (+ 160 (native-value-window-spill-offset frame-base-slot-count 7))))
        stack21 (concat-byte-vectors stack20 (emit-aarch64-str-x10-sp 80))
        stack22 (concat-byte-vectors
                     stack21
                     (emit-aarch64-ldr-x10-sp (+ 160 (native-value-window-spill-offset frame-base-slot-count 6))))
        stack23 (concat-byte-vectors stack22 (emit-aarch64-str-x10-sp 88))
        stack24 (concat-byte-vectors
                     stack23
                     (emit-aarch64-ldr-x10-sp (+ 160 (native-value-window-spill-offset frame-base-slot-count 5))))
        stack25 (concat-byte-vectors stack24 (emit-aarch64-str-x10-sp 96))
        stack26 (concat-byte-vectors
                     stack25
                     (emit-aarch64-ldr-x10-sp (+ 160 (native-value-window-spill-offset frame-base-slot-count 4))))
        stack27 (concat-byte-vectors stack26 (emit-aarch64-str-x10-sp 104))
        stack28 (concat-byte-vectors
                     stack27
                     (emit-aarch64-ldr-x10-sp (+ 160 (native-value-window-spill-offset frame-base-slot-count 3))))
        stack29 (concat-byte-vectors stack28 (emit-aarch64-str-x10-sp 112))
        stack30 (concat-byte-vectors
                     stack29
                     (emit-aarch64-ldr-x10-sp (+ 160 (native-value-window-spill-offset frame-base-slot-count 2))))
        stack31 (concat-byte-vectors stack30 (emit-aarch64-str-x10-sp 120))
        stack32 (concat-byte-vectors
                     stack31
                     (emit-aarch64-ldr-x10-sp (+ 160 (native-value-window-spill-offset frame-base-slot-count 1))))
        stack33 (concat-byte-vectors stack32 (emit-aarch64-str-x10-sp 128))
        stack34 (concat-byte-vectors
                     stack33
                     (emit-aarch64-ldr-x10-sp (+ 160 (native-value-window-spill-offset frame-base-slot-count 0))))
        stack35 (concat-byte-vectors stack34 (emit-aarch64-str-x10-sp 136))
        stack36 (concat-byte-vectors stack35 (emit-aarch64-str-x9-sp 144))
        stack37 (concat-byte-vectors stack36 (emit-aarch64-str-x0-sp 152))
        reg0 (concat-byte-vectors stack37 (emit-aarch64-ldr-x7-sp (+ 160 (native-value-window-spill-offset frame-base-slot-count 18))))
        reg1 (concat-byte-vectors reg0 (emit-aarch64-ldr-x6-sp (+ 160 (native-value-window-spill-offset frame-base-slot-count 19))))
        reg2 (concat-byte-vectors reg1 (emit-aarch64-ldr-x5-sp (+ 160 (native-value-window-spill-offset frame-base-slot-count 20))))
        reg3 (concat-byte-vectors reg2 (emit-aarch64-ldr-x4-sp (+ 160 (native-value-window-spill-offset frame-base-slot-count 21))))
        reg4 (concat-byte-vectors reg3 (emit-aarch64-ldr-x3-sp (+ 160 (native-value-window-spill-offset frame-base-slot-count 22))))
        reg5 (concat-byte-vectors reg4 (emit-aarch64-ldr-x2-sp (+ 160 (native-value-window-spill-offset frame-base-slot-count 23))))
        reg6 (concat-byte-vectors reg5 (emit-aarch64-ldr-x1-sp (+ 160 (native-value-window-spill-offset frame-base-slot-count 24))))
        reg7 (concat-byte-vectors reg6 (emit-aarch64-ldr-x0-sp (+ 160 (native-value-window-spill-offset frame-base-slot-count 25))))
        call-seq (concat-byte-vectors
                   (emit-aarch64-bl disp)
                   (emit-aarch64-add-sp 160))]
    (concat-byte-vectors reg7 call-seq)))

(defn emit-three-arg-call-aarch64 [disp frame-base-slot-count]
  (concat-byte-vectors
    (concat-byte-vectors
      (concat-byte-vectors
        (emit-aarch64-mov-x2-x0)
        (emit-aarch64-mov-x1-x9))
      (emit-aarch64-ldr-x0-sp (native-value-window-spill-offset frame-base-slot-count 0)))
    (emit-aarch64-bl disp)))

(defn emit-four-arg-call-aarch64 [disp frame-base-slot-count]
  (concat-byte-vectors
    (concat-byte-vectors
      (concat-byte-vectors
        (concat-byte-vectors
          (emit-aarch64-mov-x2-x9)
          (emit-aarch64-mov-x3-x0))
        (emit-aarch64-ldr-x1-sp (native-value-window-spill-offset frame-base-slot-count 0)))
      (emit-aarch64-ldr-x0-sp (native-value-window-spill-offset frame-base-slot-count 1)))
    (emit-aarch64-bl disp)))

(defn emit-five-arg-call-aarch64 [disp frame-base-slot-count]
  (concat-byte-vectors
    (concat-byte-vectors
      (concat-byte-vectors
        (concat-byte-vectors
          (emit-aarch64-mov-x4-x0)
          (emit-aarch64-mov-x3-x9))
        (emit-aarch64-ldr-x2-sp (native-value-window-spill-offset frame-base-slot-count 0)))
      (concat-byte-vectors
        (emit-aarch64-ldr-x1-sp (native-value-window-spill-offset frame-base-slot-count 1))
        (emit-aarch64-ldr-x0-sp (native-value-window-spill-offset frame-base-slot-count 2))))
    (emit-aarch64-bl disp)))

(defn emit-six-arg-call-aarch64 [disp frame-base-slot-count]
  (concat-byte-vectors
    (concat-byte-vectors
      (concat-byte-vectors
        (concat-byte-vectors
          (concat-byte-vectors
            (concat-byte-vectors
              (emit-aarch64-mov-x5-x0)
              (emit-aarch64-mov-x4-x9))
            (emit-aarch64-ldr-x3-sp (native-value-window-spill-offset frame-base-slot-count 0)))
          (emit-aarch64-ldr-x2-sp (native-value-window-spill-offset frame-base-slot-count 1)))
        (emit-aarch64-ldr-x1-sp (native-value-window-spill-offset frame-base-slot-count 2)))
      (emit-aarch64-ldr-x0-sp (native-value-window-spill-offset frame-base-slot-count 3)))
    (emit-aarch64-bl disp)))

(defn emit-seven-arg-call-aarch64 [disp frame-base-slot-count]
  (concat-byte-vectors
    (concat-byte-vectors
      (concat-byte-vectors
        (concat-byte-vectors
          (concat-byte-vectors
            (concat-byte-vectors
              (concat-byte-vectors
                (emit-aarch64-mov-x6-x0)
                (emit-aarch64-mov-x5-x9))
              (emit-aarch64-ldr-x4-sp (native-value-window-spill-offset frame-base-slot-count 0)))
            (emit-aarch64-ldr-x3-sp (native-value-window-spill-offset frame-base-slot-count 1)))
          (emit-aarch64-ldr-x2-sp (native-value-window-spill-offset frame-base-slot-count 2)))
        (emit-aarch64-ldr-x1-sp (native-value-window-spill-offset frame-base-slot-count 3)))
      (emit-aarch64-ldr-x0-sp (native-value-window-spill-offset frame-base-slot-count 4)))
    (emit-aarch64-bl disp)))

(defn emit-eight-arg-call-aarch64 [disp frame-base-slot-count]
  (concat-byte-vectors
    (concat-byte-vectors
      (concat-byte-vectors
        (concat-byte-vectors
          (concat-byte-vectors
            (concat-byte-vectors
              (concat-byte-vectors
                (concat-byte-vectors
                  (emit-aarch64-mov-x7-x0)
                  (emit-aarch64-mov-x6-x9))
                (emit-aarch64-ldr-x5-sp (native-value-window-spill-offset frame-base-slot-count 0)))
              (emit-aarch64-ldr-x4-sp (native-value-window-spill-offset frame-base-slot-count 1)))
            (emit-aarch64-ldr-x3-sp (native-value-window-spill-offset frame-base-slot-count 2)))
          (emit-aarch64-ldr-x2-sp (native-value-window-spill-offset frame-base-slot-count 3)))
        (emit-aarch64-ldr-x1-sp (native-value-window-spill-offset frame-base-slot-count 4)))
      (emit-aarch64-ldr-x0-sp (native-value-window-spill-offset frame-base-slot-count 5)))
    (emit-aarch64-bl disp)))

(defn emit-nine-arg-call-aarch64 [disp frame-base-slot-count]
  (concat-byte-vectors
    (concat-byte-vectors
      (concat-byte-vectors
        (concat-byte-vectors
          (concat-byte-vectors
            (concat-byte-vectors
              (concat-byte-vectors
                (concat-byte-vectors
                  (concat-byte-vectors
                    (concat-byte-vectors
                      (concat-byte-vectors
                        (emit-aarch64-sub-sp 16)
                        (emit-aarch64-str-x0-sp 0))
                      (emit-aarch64-mov-x7-x9))
                    (emit-aarch64-ldr-x6-sp (+ 16 (native-value-window-spill-offset frame-base-slot-count 0))))
                  (emit-aarch64-ldr-x5-sp (+ 16 (native-value-window-spill-offset frame-base-slot-count 1))))
                (emit-aarch64-ldr-x4-sp (+ 16 (native-value-window-spill-offset frame-base-slot-count 2))))
              (emit-aarch64-ldr-x3-sp (+ 16 (native-value-window-spill-offset frame-base-slot-count 3))))
            (emit-aarch64-ldr-x2-sp (+ 16 (native-value-window-spill-offset frame-base-slot-count 4))))
          (emit-aarch64-ldr-x1-sp (+ 16 (native-value-window-spill-offset frame-base-slot-count 5))))
        (emit-aarch64-ldr-x0-sp (+ 16 (native-value-window-spill-offset frame-base-slot-count 6))))
       (emit-aarch64-bl disp))
     (emit-aarch64-add-sp 16)))

(defn emit-ten-arg-call-aarch64 [disp frame-base-slot-count]
  (let [stack0 (concat-byte-vectors
                 (emit-aarch64-sub-sp 16)
                 (emit-aarch64-str-x9-sp 0))
        stack1 (concat-byte-vectors stack0 (emit-aarch64-str-x0-sp 8))
        reg0 (concat-byte-vectors stack1 (emit-aarch64-ldr-x7-sp (+ 16 (native-value-window-spill-offset frame-base-slot-count 0))))
        reg1 (concat-byte-vectors reg0 (emit-aarch64-ldr-x6-sp (+ 16 (native-value-window-spill-offset frame-base-slot-count 1))))
        reg2 (concat-byte-vectors reg1 (emit-aarch64-ldr-x5-sp (+ 16 (native-value-window-spill-offset frame-base-slot-count 2))))
        reg3 (concat-byte-vectors reg2 (emit-aarch64-ldr-x4-sp (+ 16 (native-value-window-spill-offset frame-base-slot-count 3))))
        reg4 (concat-byte-vectors reg3 (emit-aarch64-ldr-x3-sp (+ 16 (native-value-window-spill-offset frame-base-slot-count 4))))
        reg5 (concat-byte-vectors reg4 (emit-aarch64-ldr-x2-sp (+ 16 (native-value-window-spill-offset frame-base-slot-count 5))))
        reg6 (concat-byte-vectors reg5 (emit-aarch64-ldr-x1-sp (+ 16 (native-value-window-spill-offset frame-base-slot-count 6))))
        reg-setup (concat-byte-vectors reg6 (emit-aarch64-ldr-x0-sp (+ 16 (native-value-window-spill-offset frame-base-slot-count 7))))
        call-seq (concat-byte-vectors
                   (emit-aarch64-bl disp)
                   (emit-aarch64-add-sp 16))]
    (concat-byte-vectors reg-setup call-seq)))

(defn emit-eleven-arg-call-aarch64 [disp frame-base-slot-count]
  (let [stack0 (concat-byte-vectors
                 (emit-aarch64-sub-sp 32)
                 (emit-aarch64-ldr-x10-sp (+ 32 (native-value-window-spill-offset frame-base-slot-count 0))))
        stack1 (concat-byte-vectors stack0 (emit-aarch64-str-x10-sp 0))
        stack2 (concat-byte-vectors stack1 (emit-aarch64-str-x9-sp 8))
        stack3 (concat-byte-vectors stack2 (emit-aarch64-str-x0-sp 16))
        reg0 (concat-byte-vectors stack3 (emit-aarch64-ldr-x7-sp (+ 32 (native-value-window-spill-offset frame-base-slot-count 1))))
        reg1 (concat-byte-vectors reg0 (emit-aarch64-ldr-x6-sp (+ 32 (native-value-window-spill-offset frame-base-slot-count 2))))
        reg2 (concat-byte-vectors reg1 (emit-aarch64-ldr-x5-sp (+ 32 (native-value-window-spill-offset frame-base-slot-count 3))))
        reg3 (concat-byte-vectors reg2 (emit-aarch64-ldr-x4-sp (+ 32 (native-value-window-spill-offset frame-base-slot-count 4))))
        reg4 (concat-byte-vectors reg3 (emit-aarch64-ldr-x3-sp (+ 32 (native-value-window-spill-offset frame-base-slot-count 5))))
        reg5 (concat-byte-vectors reg4 (emit-aarch64-ldr-x2-sp (+ 32 (native-value-window-spill-offset frame-base-slot-count 6))))
        reg6 (concat-byte-vectors reg5 (emit-aarch64-ldr-x1-sp (+ 32 (native-value-window-spill-offset frame-base-slot-count 7))))
        reg-setup (concat-byte-vectors reg6 (emit-aarch64-ldr-x0-sp (+ 32 (native-value-window-spill-offset frame-base-slot-count 8))))
        call-seq (concat-byte-vectors
                   (emit-aarch64-bl disp)
                    (emit-aarch64-add-sp 32))]
    (concat-byte-vectors reg-setup call-seq)))

(defn emit-twelve-arg-call-aarch64 [disp frame-base-slot-count]
  (let [stack0 (concat-byte-vectors
                 (emit-aarch64-sub-sp 32)
                 (emit-aarch64-ldr-x10-sp (+ 32 (native-value-window-spill-offset frame-base-slot-count 1))))
        stack1 (concat-byte-vectors stack0 (emit-aarch64-str-x10-sp 0))
        stack2 (concat-byte-vectors
                 stack1
                 (emit-aarch64-ldr-x10-sp (+ 32 (native-value-window-spill-offset frame-base-slot-count 0))))
        stack3 (concat-byte-vectors stack2 (emit-aarch64-str-x10-sp 8))
        stack4 (concat-byte-vectors stack3 (emit-aarch64-str-x9-sp 16))
        stack5 (concat-byte-vectors stack4 (emit-aarch64-str-x0-sp 24))
        reg0 (concat-byte-vectors stack5 (emit-aarch64-ldr-x7-sp (+ 32 (native-value-window-spill-offset frame-base-slot-count 2))))
        reg1 (concat-byte-vectors reg0 (emit-aarch64-ldr-x6-sp (+ 32 (native-value-window-spill-offset frame-base-slot-count 3))))
        reg2 (concat-byte-vectors reg1 (emit-aarch64-ldr-x5-sp (+ 32 (native-value-window-spill-offset frame-base-slot-count 4))))
        reg3 (concat-byte-vectors reg2 (emit-aarch64-ldr-x4-sp (+ 32 (native-value-window-spill-offset frame-base-slot-count 5))))
        reg4 (concat-byte-vectors reg3 (emit-aarch64-ldr-x3-sp (+ 32 (native-value-window-spill-offset frame-base-slot-count 6))))
        reg5 (concat-byte-vectors reg4 (emit-aarch64-ldr-x2-sp (+ 32 (native-value-window-spill-offset frame-base-slot-count 7))))
        reg6 (concat-byte-vectors reg5 (emit-aarch64-ldr-x1-sp (+ 32 (native-value-window-spill-offset frame-base-slot-count 8))))
        reg-setup (concat-byte-vectors reg6 (emit-aarch64-ldr-x0-sp (+ 32 (native-value-window-spill-offset frame-base-slot-count 9))))
        call-seq (concat-byte-vectors
                    (emit-aarch64-bl disp)
                    (emit-aarch64-add-sp 32))]
    (concat-byte-vectors reg-setup call-seq)))

(defn emit-thirteen-arg-call-aarch64 [disp frame-base-slot-count]
  (let [stack0 (concat-byte-vectors
                 (emit-aarch64-sub-sp 48)
                 (emit-aarch64-ldr-x10-sp (+ 48 (native-value-window-spill-offset frame-base-slot-count 2))))
        stack1 (concat-byte-vectors stack0 (emit-aarch64-str-x10-sp 0))
        stack2 (concat-byte-vectors
                 stack1
                 (emit-aarch64-ldr-x10-sp (+ 48 (native-value-window-spill-offset frame-base-slot-count 1))))
        stack3 (concat-byte-vectors stack2 (emit-aarch64-str-x10-sp 8))
        stack4 (concat-byte-vectors
                 stack3
                 (emit-aarch64-ldr-x10-sp (+ 48 (native-value-window-spill-offset frame-base-slot-count 0))))
        stack5 (concat-byte-vectors stack4 (emit-aarch64-str-x10-sp 16))
        stack6 (concat-byte-vectors stack5 (emit-aarch64-str-x9-sp 24))
        stack7 (concat-byte-vectors stack6 (emit-aarch64-str-x0-sp 32))
        reg0 (concat-byte-vectors stack7 (emit-aarch64-ldr-x7-sp (+ 48 (native-value-window-spill-offset frame-base-slot-count 3))))
        reg1 (concat-byte-vectors reg0 (emit-aarch64-ldr-x6-sp (+ 48 (native-value-window-spill-offset frame-base-slot-count 4))))
        reg2 (concat-byte-vectors reg1 (emit-aarch64-ldr-x5-sp (+ 48 (native-value-window-spill-offset frame-base-slot-count 5))))
        reg3 (concat-byte-vectors reg2 (emit-aarch64-ldr-x4-sp (+ 48 (native-value-window-spill-offset frame-base-slot-count 6))))
        reg4 (concat-byte-vectors reg3 (emit-aarch64-ldr-x3-sp (+ 48 (native-value-window-spill-offset frame-base-slot-count 7))))
        reg5 (concat-byte-vectors reg4 (emit-aarch64-ldr-x2-sp (+ 48 (native-value-window-spill-offset frame-base-slot-count 8))))
        reg6 (concat-byte-vectors reg5 (emit-aarch64-ldr-x1-sp (+ 48 (native-value-window-spill-offset frame-base-slot-count 9))))
        reg-setup (concat-byte-vectors reg6 (emit-aarch64-ldr-x0-sp (+ 48 (native-value-window-spill-offset frame-base-slot-count 10))))
        call-seq (concat-byte-vectors
                    (emit-aarch64-bl disp)
                    (emit-aarch64-add-sp 48))]
    (concat-byte-vectors reg-setup call-seq)))

(defn emit-fourteen-arg-call-aarch64 [disp frame-base-slot-count]
  (let [stack0 (concat-byte-vectors
                 (emit-aarch64-sub-sp 48)
                 (emit-aarch64-ldr-x10-sp (+ 48 (native-value-window-spill-offset frame-base-slot-count 3))))
        stack1 (concat-byte-vectors stack0 (emit-aarch64-str-x10-sp 0))
        stack2 (concat-byte-vectors
                 stack1
                 (emit-aarch64-ldr-x10-sp (+ 48 (native-value-window-spill-offset frame-base-slot-count 2))))
        stack3 (concat-byte-vectors stack2 (emit-aarch64-str-x10-sp 8))
        stack4 (concat-byte-vectors
                 stack3
                 (emit-aarch64-ldr-x10-sp (+ 48 (native-value-window-spill-offset frame-base-slot-count 1))))
        stack5 (concat-byte-vectors stack4 (emit-aarch64-str-x10-sp 16))
        stack6 (concat-byte-vectors
                 stack5
                 (emit-aarch64-ldr-x10-sp (+ 48 (native-value-window-spill-offset frame-base-slot-count 0))))
        stack7 (concat-byte-vectors stack6 (emit-aarch64-str-x10-sp 24))
        stack8 (concat-byte-vectors stack7 (emit-aarch64-str-x9-sp 32))
        stack9 (concat-byte-vectors stack8 (emit-aarch64-str-x0-sp 40))
        reg0 (concat-byte-vectors stack9 (emit-aarch64-ldr-x7-sp (+ 48 (native-value-window-spill-offset frame-base-slot-count 4))))
        reg1 (concat-byte-vectors reg0 (emit-aarch64-ldr-x6-sp (+ 48 (native-value-window-spill-offset frame-base-slot-count 5))))
        reg2 (concat-byte-vectors reg1 (emit-aarch64-ldr-x5-sp (+ 48 (native-value-window-spill-offset frame-base-slot-count 6))))
        reg3 (concat-byte-vectors reg2 (emit-aarch64-ldr-x4-sp (+ 48 (native-value-window-spill-offset frame-base-slot-count 7))))
        reg4 (concat-byte-vectors reg3 (emit-aarch64-ldr-x3-sp (+ 48 (native-value-window-spill-offset frame-base-slot-count 8))))
        reg5 (concat-byte-vectors reg4 (emit-aarch64-ldr-x2-sp (+ 48 (native-value-window-spill-offset frame-base-slot-count 9))))
        reg6 (concat-byte-vectors reg5 (emit-aarch64-ldr-x1-sp (+ 48 (native-value-window-spill-offset frame-base-slot-count 10))))
        reg-setup (concat-byte-vectors reg6 (emit-aarch64-ldr-x0-sp (+ 48 (native-value-window-spill-offset frame-base-slot-count 11))))
        call-seq (concat-byte-vectors
                   (emit-aarch64-bl disp)
                   (emit-aarch64-add-sp 48))]
    (concat-byte-vectors reg-setup call-seq)))

(defn emit-fifteen-arg-call-aarch64 [disp frame-base-slot-count]
  (let [stack0 (concat-byte-vectors
                 (emit-aarch64-sub-sp 64)
                 (emit-aarch64-ldr-x10-sp (+ 64 (native-value-window-spill-offset frame-base-slot-count 4))))
        stack1 (concat-byte-vectors stack0 (emit-aarch64-str-x10-sp 0))
        stack2 (concat-byte-vectors
                 stack1
                 (emit-aarch64-ldr-x10-sp (+ 64 (native-value-window-spill-offset frame-base-slot-count 3))))
        stack3 (concat-byte-vectors stack2 (emit-aarch64-str-x10-sp 8))
        stack4 (concat-byte-vectors
                 stack3
                 (emit-aarch64-ldr-x10-sp (+ 64 (native-value-window-spill-offset frame-base-slot-count 2))))
        stack5 (concat-byte-vectors stack4 (emit-aarch64-str-x10-sp 16))
        stack6 (concat-byte-vectors
                 stack5
                 (emit-aarch64-ldr-x10-sp (+ 64 (native-value-window-spill-offset frame-base-slot-count 1))))
        stack7 (concat-byte-vectors stack6 (emit-aarch64-str-x10-sp 24))
        stack8 (concat-byte-vectors
                 stack7
                 (emit-aarch64-ldr-x10-sp (+ 64 (native-value-window-spill-offset frame-base-slot-count 0))))
        stack9 (concat-byte-vectors stack8 (emit-aarch64-str-x10-sp 32))
        stack10 (concat-byte-vectors stack9 (emit-aarch64-str-x9-sp 40))
        stack11 (concat-byte-vectors stack10 (emit-aarch64-str-x0-sp 48))
        reg0 (concat-byte-vectors stack11 (emit-aarch64-ldr-x7-sp (+ 64 (native-value-window-spill-offset frame-base-slot-count 5))))
        reg1 (concat-byte-vectors reg0 (emit-aarch64-ldr-x6-sp (+ 64 (native-value-window-spill-offset frame-base-slot-count 6))))
        reg2 (concat-byte-vectors reg1 (emit-aarch64-ldr-x5-sp (+ 64 (native-value-window-spill-offset frame-base-slot-count 7))))
        reg3 (concat-byte-vectors reg2 (emit-aarch64-ldr-x4-sp (+ 64 (native-value-window-spill-offset frame-base-slot-count 8))))
        reg4 (concat-byte-vectors reg3 (emit-aarch64-ldr-x3-sp (+ 64 (native-value-window-spill-offset frame-base-slot-count 9))))
        reg5 (concat-byte-vectors reg4 (emit-aarch64-ldr-x2-sp (+ 64 (native-value-window-spill-offset frame-base-slot-count 10))))
        reg6 (concat-byte-vectors reg5 (emit-aarch64-ldr-x1-sp (+ 64 (native-value-window-spill-offset frame-base-slot-count 11))))
        reg-setup (concat-byte-vectors reg6 (emit-aarch64-ldr-x0-sp (+ 64 (native-value-window-spill-offset frame-base-slot-count 12))))
        call-seq (concat-byte-vectors
                   (emit-aarch64-bl disp)
                   (emit-aarch64-add-sp 64))]
    (concat-byte-vectors reg-setup call-seq)))

(defn emit-sixteen-arg-call-aarch64 [disp frame-base-slot-count]
  (let [stack0 (concat-byte-vectors
                 (emit-aarch64-sub-sp 64)
                 (emit-aarch64-ldr-x10-sp (+ 64 (native-value-window-spill-offset frame-base-slot-count 5))))
        stack1 (concat-byte-vectors stack0 (emit-aarch64-str-x10-sp 0))
        stack2 (concat-byte-vectors
                 stack1
                 (emit-aarch64-ldr-x10-sp (+ 64 (native-value-window-spill-offset frame-base-slot-count 4))))
        stack3 (concat-byte-vectors stack2 (emit-aarch64-str-x10-sp 8))
        stack4 (concat-byte-vectors
                 stack3
                 (emit-aarch64-ldr-x10-sp (+ 64 (native-value-window-spill-offset frame-base-slot-count 3))))
        stack5 (concat-byte-vectors stack4 (emit-aarch64-str-x10-sp 16))
        stack6 (concat-byte-vectors
                 stack5
                 (emit-aarch64-ldr-x10-sp (+ 64 (native-value-window-spill-offset frame-base-slot-count 2))))
        stack7 (concat-byte-vectors stack6 (emit-aarch64-str-x10-sp 24))
        stack8 (concat-byte-vectors
                 stack7
                 (emit-aarch64-ldr-x10-sp (+ 64 (native-value-window-spill-offset frame-base-slot-count 1))))
        stack9 (concat-byte-vectors stack8 (emit-aarch64-str-x10-sp 32))
        stack10 (concat-byte-vectors
                  stack9
                  (emit-aarch64-ldr-x10-sp (+ 64 (native-value-window-spill-offset frame-base-slot-count 0))))
        stack11 (concat-byte-vectors stack10 (emit-aarch64-str-x10-sp 40))
        stack12 (concat-byte-vectors stack11 (emit-aarch64-str-x9-sp 48))
        stack13 (concat-byte-vectors stack12 (emit-aarch64-str-x0-sp 56))
        reg0 (concat-byte-vectors stack13 (emit-aarch64-ldr-x7-sp (+ 64 (native-value-window-spill-offset frame-base-slot-count 6))))
        reg1 (concat-byte-vectors reg0 (emit-aarch64-ldr-x6-sp (+ 64 (native-value-window-spill-offset frame-base-slot-count 7))))
        reg2 (concat-byte-vectors reg1 (emit-aarch64-ldr-x5-sp (+ 64 (native-value-window-spill-offset frame-base-slot-count 8))))
        reg3 (concat-byte-vectors reg2 (emit-aarch64-ldr-x4-sp (+ 64 (native-value-window-spill-offset frame-base-slot-count 9))))
        reg4 (concat-byte-vectors reg3 (emit-aarch64-ldr-x3-sp (+ 64 (native-value-window-spill-offset frame-base-slot-count 10))))
        reg5 (concat-byte-vectors reg4 (emit-aarch64-ldr-x2-sp (+ 64 (native-value-window-spill-offset frame-base-slot-count 11))))
        reg6 (concat-byte-vectors reg5 (emit-aarch64-ldr-x1-sp (+ 64 (native-value-window-spill-offset frame-base-slot-count 12))))
        reg-setup (concat-byte-vectors reg6 (emit-aarch64-ldr-x0-sp (+ 64 (native-value-window-spill-offset frame-base-slot-count 13))))
        call-seq (concat-byte-vectors
                   (emit-aarch64-bl disp)
                   (emit-aarch64-add-sp 64))]
    (concat-byte-vectors reg-setup call-seq)))

(defn emit-seventeen-arg-call-aarch64 [disp frame-base-slot-count]
  (let [stack0 (concat-byte-vectors
                 (emit-aarch64-sub-sp 80)
                 (emit-aarch64-ldr-x10-sp (+ 80 (native-value-window-spill-offset frame-base-slot-count 6))))
        stack1 (concat-byte-vectors stack0 (emit-aarch64-str-x10-sp 0))
        stack2 (concat-byte-vectors
                 stack1
                 (emit-aarch64-ldr-x10-sp (+ 80 (native-value-window-spill-offset frame-base-slot-count 5))))
        stack3 (concat-byte-vectors stack2 (emit-aarch64-str-x10-sp 8))
        stack4 (concat-byte-vectors
                 stack3
                 (emit-aarch64-ldr-x10-sp (+ 80 (native-value-window-spill-offset frame-base-slot-count 4))))
        stack5 (concat-byte-vectors stack4 (emit-aarch64-str-x10-sp 16))
        stack6 (concat-byte-vectors
                 stack5
                 (emit-aarch64-ldr-x10-sp (+ 80 (native-value-window-spill-offset frame-base-slot-count 3))))
        stack7 (concat-byte-vectors stack6 (emit-aarch64-str-x10-sp 24))
        stack8 (concat-byte-vectors
                 stack7
                 (emit-aarch64-ldr-x10-sp (+ 80 (native-value-window-spill-offset frame-base-slot-count 2))))
        stack9 (concat-byte-vectors stack8 (emit-aarch64-str-x10-sp 32))
        stack10 (concat-byte-vectors
                  stack9
                  (emit-aarch64-ldr-x10-sp (+ 80 (native-value-window-spill-offset frame-base-slot-count 1))))
        stack11 (concat-byte-vectors stack10 (emit-aarch64-str-x10-sp 40))
        stack12 (concat-byte-vectors
                  stack11
                  (emit-aarch64-ldr-x10-sp (+ 80 (native-value-window-spill-offset frame-base-slot-count 0))))
        stack13 (concat-byte-vectors stack12 (emit-aarch64-str-x10-sp 48))
        stack14 (concat-byte-vectors stack13 (emit-aarch64-str-x9-sp 56))
        stack15 (concat-byte-vectors stack14 (emit-aarch64-str-x0-sp 64))
        reg0 (concat-byte-vectors stack15 (emit-aarch64-ldr-x7-sp (+ 80 (native-value-window-spill-offset frame-base-slot-count 7))))
        reg1 (concat-byte-vectors reg0 (emit-aarch64-ldr-x6-sp (+ 80 (native-value-window-spill-offset frame-base-slot-count 8))))
        reg2 (concat-byte-vectors reg1 (emit-aarch64-ldr-x5-sp (+ 80 (native-value-window-spill-offset frame-base-slot-count 9))))
        reg3 (concat-byte-vectors reg2 (emit-aarch64-ldr-x4-sp (+ 80 (native-value-window-spill-offset frame-base-slot-count 10))))
        reg4 (concat-byte-vectors reg3 (emit-aarch64-ldr-x3-sp (+ 80 (native-value-window-spill-offset frame-base-slot-count 11))))
        reg5 (concat-byte-vectors reg4 (emit-aarch64-ldr-x2-sp (+ 80 (native-value-window-spill-offset frame-base-slot-count 12))))
        reg6 (concat-byte-vectors reg5 (emit-aarch64-ldr-x1-sp (+ 80 (native-value-window-spill-offset frame-base-slot-count 13))))
        reg-setup (concat-byte-vectors reg6 (emit-aarch64-ldr-x0-sp (+ 80 (native-value-window-spill-offset frame-base-slot-count 14))))
        call-seq (concat-byte-vectors
                   (emit-aarch64-bl disp)
                   (emit-aarch64-add-sp 80))]
    (concat-byte-vectors reg-setup call-seq)))

(defn emit-eighteen-arg-call-aarch64 [disp frame-base-slot-count]
  (let [stack0 (concat-byte-vectors
                 (emit-aarch64-sub-sp 80)
                 (emit-aarch64-ldr-x10-sp (+ 80 (native-value-window-spill-offset frame-base-slot-count 7))))
        stack1 (concat-byte-vectors stack0 (emit-aarch64-str-x10-sp 0))
        stack2 (concat-byte-vectors
                 stack1
                 (emit-aarch64-ldr-x10-sp (+ 80 (native-value-window-spill-offset frame-base-slot-count 6))))
        stack3 (concat-byte-vectors stack2 (emit-aarch64-str-x10-sp 8))
        stack4 (concat-byte-vectors
                 stack3
                 (emit-aarch64-ldr-x10-sp (+ 80 (native-value-window-spill-offset frame-base-slot-count 5))))
        stack5 (concat-byte-vectors stack4 (emit-aarch64-str-x10-sp 16))
        stack6 (concat-byte-vectors
                 stack5
                 (emit-aarch64-ldr-x10-sp (+ 80 (native-value-window-spill-offset frame-base-slot-count 4))))
        stack7 (concat-byte-vectors stack6 (emit-aarch64-str-x10-sp 24))
        stack8 (concat-byte-vectors
                 stack7
                 (emit-aarch64-ldr-x10-sp (+ 80 (native-value-window-spill-offset frame-base-slot-count 3))))
        stack9 (concat-byte-vectors stack8 (emit-aarch64-str-x10-sp 32))
        stack10 (concat-byte-vectors
                  stack9
                  (emit-aarch64-ldr-x10-sp (+ 80 (native-value-window-spill-offset frame-base-slot-count 2))))
        stack11 (concat-byte-vectors stack10 (emit-aarch64-str-x10-sp 40))
        stack12 (concat-byte-vectors
                  stack11
                  (emit-aarch64-ldr-x10-sp (+ 80 (native-value-window-spill-offset frame-base-slot-count 1))))
        stack13 (concat-byte-vectors stack12 (emit-aarch64-str-x10-sp 48))
        stack14 (concat-byte-vectors
                  stack13
                  (emit-aarch64-ldr-x10-sp (+ 80 (native-value-window-spill-offset frame-base-slot-count 0))))
        stack15 (concat-byte-vectors stack14 (emit-aarch64-str-x10-sp 56))
        stack16 (concat-byte-vectors stack15 (emit-aarch64-str-x9-sp 64))
        stack17 (concat-byte-vectors stack16 (emit-aarch64-str-x0-sp 72))
        reg0 (concat-byte-vectors stack17 (emit-aarch64-ldr-x7-sp (+ 80 (native-value-window-spill-offset frame-base-slot-count 8))))
        reg1 (concat-byte-vectors reg0 (emit-aarch64-ldr-x6-sp (+ 80 (native-value-window-spill-offset frame-base-slot-count 9))))
        reg2 (concat-byte-vectors reg1 (emit-aarch64-ldr-x5-sp (+ 80 (native-value-window-spill-offset frame-base-slot-count 10))))
        reg3 (concat-byte-vectors reg2 (emit-aarch64-ldr-x4-sp (+ 80 (native-value-window-spill-offset frame-base-slot-count 11))))
        reg4 (concat-byte-vectors reg3 (emit-aarch64-ldr-x3-sp (+ 80 (native-value-window-spill-offset frame-base-slot-count 12))))
        reg5 (concat-byte-vectors reg4 (emit-aarch64-ldr-x2-sp (+ 80 (native-value-window-spill-offset frame-base-slot-count 13))))
        reg6 (concat-byte-vectors reg5 (emit-aarch64-ldr-x1-sp (+ 80 (native-value-window-spill-offset frame-base-slot-count 14))))
        reg-setup (concat-byte-vectors reg6 (emit-aarch64-ldr-x0-sp (+ 80 (native-value-window-spill-offset frame-base-slot-count 15))))
        call-seq (concat-byte-vectors
                   (emit-aarch64-bl disp)
                   (emit-aarch64-add-sp 80))]
    (concat-byte-vectors reg-setup call-seq)))

(defn emit-twenty-five-arg-call-aarch64 [disp frame-base-slot-count]
  (let [stack0 (concat-byte-vectors
                 (emit-aarch64-sub-sp 144)
                 (emit-aarch64-ldr-x10-sp (+ 144 (native-value-window-spill-offset frame-base-slot-count 14))))
        stack1 (concat-byte-vectors stack0 (emit-aarch64-str-x10-sp 0))
        stack2 (concat-byte-vectors
                    stack1
                    (emit-aarch64-ldr-x10-sp (+ 144 (native-value-window-spill-offset frame-base-slot-count 13))))
        stack3 (concat-byte-vectors stack2 (emit-aarch64-str-x10-sp 8))
        stack4 (concat-byte-vectors
                    stack3
                    (emit-aarch64-ldr-x10-sp (+ 144 (native-value-window-spill-offset frame-base-slot-count 12))))
        stack5 (concat-byte-vectors stack4 (emit-aarch64-str-x10-sp 16))
        stack6 (concat-byte-vectors
                    stack5
                    (emit-aarch64-ldr-x10-sp (+ 144 (native-value-window-spill-offset frame-base-slot-count 11))))
        stack7 (concat-byte-vectors stack6 (emit-aarch64-str-x10-sp 24))
        stack8 (concat-byte-vectors
                    stack7
                    (emit-aarch64-ldr-x10-sp (+ 144 (native-value-window-spill-offset frame-base-slot-count 10))))
        stack9 (concat-byte-vectors stack8 (emit-aarch64-str-x10-sp 32))
        stack10 (concat-byte-vectors
                     stack9
                     (emit-aarch64-ldr-x10-sp (+ 144 (native-value-window-spill-offset frame-base-slot-count 9))))
        stack11 (concat-byte-vectors stack10 (emit-aarch64-str-x10-sp 40))
        stack12 (concat-byte-vectors
                     stack11
                     (emit-aarch64-ldr-x10-sp (+ 144 (native-value-window-spill-offset frame-base-slot-count 8))))
        stack13 (concat-byte-vectors stack12 (emit-aarch64-str-x10-sp 48))
        stack14 (concat-byte-vectors
                     stack13
                     (emit-aarch64-ldr-x10-sp (+ 144 (native-value-window-spill-offset frame-base-slot-count 7))))
        stack15 (concat-byte-vectors stack14 (emit-aarch64-str-x10-sp 56))
        stack16 (concat-byte-vectors
                     stack15
                     (emit-aarch64-ldr-x10-sp (+ 144 (native-value-window-spill-offset frame-base-slot-count 6))))
        stack17 (concat-byte-vectors stack16 (emit-aarch64-str-x10-sp 64))
        stack18 (concat-byte-vectors
                     stack17
                     (emit-aarch64-ldr-x10-sp (+ 144 (native-value-window-spill-offset frame-base-slot-count 5))))
        stack19 (concat-byte-vectors stack18 (emit-aarch64-str-x10-sp 72))
        stack20 (concat-byte-vectors
                     stack19
                     (emit-aarch64-ldr-x10-sp (+ 144 (native-value-window-spill-offset frame-base-slot-count 4))))
        stack21 (concat-byte-vectors stack20 (emit-aarch64-str-x10-sp 80))
        stack22 (concat-byte-vectors
                     stack21
                     (emit-aarch64-ldr-x10-sp (+ 144 (native-value-window-spill-offset frame-base-slot-count 3))))
        stack23 (concat-byte-vectors stack22 (emit-aarch64-str-x10-sp 88))
        stack24 (concat-byte-vectors
                     stack23
                     (emit-aarch64-ldr-x10-sp (+ 144 (native-value-window-spill-offset frame-base-slot-count 2))))
        stack25 (concat-byte-vectors stack24 (emit-aarch64-str-x10-sp 96))
        stack26 (concat-byte-vectors
                     stack25
                     (emit-aarch64-ldr-x10-sp (+ 144 (native-value-window-spill-offset frame-base-slot-count 1))))
        stack27 (concat-byte-vectors stack26 (emit-aarch64-str-x10-sp 104))
        stack28 (concat-byte-vectors
                     stack27
                     (emit-aarch64-ldr-x10-sp (+ 144 (native-value-window-spill-offset frame-base-slot-count 0))))
        stack29 (concat-byte-vectors stack28 (emit-aarch64-str-x10-sp 112))
        stack30 (concat-byte-vectors stack29 (emit-aarch64-str-x9-sp 120))
        stack31 (concat-byte-vectors stack30 (emit-aarch64-str-x0-sp 128))
        reg0 (concat-byte-vectors stack31 (emit-aarch64-ldr-x7-sp (+ 144 (native-value-window-spill-offset frame-base-slot-count 15))))
        reg1 (concat-byte-vectors reg0 (emit-aarch64-ldr-x6-sp (+ 144 (native-value-window-spill-offset frame-base-slot-count 16))))
        reg2 (concat-byte-vectors reg1 (emit-aarch64-ldr-x5-sp (+ 144 (native-value-window-spill-offset frame-base-slot-count 17))))
        reg3 (concat-byte-vectors reg2 (emit-aarch64-ldr-x4-sp (+ 144 (native-value-window-spill-offset frame-base-slot-count 18))))
        reg4 (concat-byte-vectors reg3 (emit-aarch64-ldr-x3-sp (+ 144 (native-value-window-spill-offset frame-base-slot-count 19))))
        reg5 (concat-byte-vectors reg4 (emit-aarch64-ldr-x2-sp (+ 144 (native-value-window-spill-offset frame-base-slot-count 20))))
        reg6 (concat-byte-vectors reg5 (emit-aarch64-ldr-x1-sp (+ 144 (native-value-window-spill-offset frame-base-slot-count 21))))
        reg7 (concat-byte-vectors reg6 (emit-aarch64-ldr-x0-sp (+ 144 (native-value-window-spill-offset frame-base-slot-count 22))))
        call-seq (concat-byte-vectors
                   (emit-aarch64-bl disp)
                   (emit-aarch64-add-sp 144))]
    (concat-byte-vectors reg7 call-seq)))

(defn emit-twenty-four-arg-call-aarch64 [disp frame-base-slot-count]
  (let [stack0 (concat-byte-vectors
                 (emit-aarch64-sub-sp 128)
                 (emit-aarch64-ldr-x10-sp (+ 128 (native-value-window-spill-offset frame-base-slot-count 13))))
        stack1 (concat-byte-vectors stack0 (emit-aarch64-str-x10-sp 0))
        stack2 (concat-byte-vectors
                    stack1
                    (emit-aarch64-ldr-x10-sp (+ 128 (native-value-window-spill-offset frame-base-slot-count 12))))
        stack3 (concat-byte-vectors stack2 (emit-aarch64-str-x10-sp 8))
        stack4 (concat-byte-vectors
                    stack3
                    (emit-aarch64-ldr-x10-sp (+ 128 (native-value-window-spill-offset frame-base-slot-count 11))))
        stack5 (concat-byte-vectors stack4 (emit-aarch64-str-x10-sp 16))
        stack6 (concat-byte-vectors
                    stack5
                    (emit-aarch64-ldr-x10-sp (+ 128 (native-value-window-spill-offset frame-base-slot-count 10))))
        stack7 (concat-byte-vectors stack6 (emit-aarch64-str-x10-sp 24))
        stack8 (concat-byte-vectors
                    stack7
                    (emit-aarch64-ldr-x10-sp (+ 128 (native-value-window-spill-offset frame-base-slot-count 9))))
        stack9 (concat-byte-vectors stack8 (emit-aarch64-str-x10-sp 32))
        stack10 (concat-byte-vectors
                     stack9
                     (emit-aarch64-ldr-x10-sp (+ 128 (native-value-window-spill-offset frame-base-slot-count 8))))
        stack11 (concat-byte-vectors stack10 (emit-aarch64-str-x10-sp 40))
        stack12 (concat-byte-vectors
                     stack11
                     (emit-aarch64-ldr-x10-sp (+ 128 (native-value-window-spill-offset frame-base-slot-count 7))))
        stack13 (concat-byte-vectors stack12 (emit-aarch64-str-x10-sp 48))
        stack14 (concat-byte-vectors
                     stack13
                     (emit-aarch64-ldr-x10-sp (+ 128 (native-value-window-spill-offset frame-base-slot-count 6))))
        stack15 (concat-byte-vectors stack14 (emit-aarch64-str-x10-sp 56))
        stack16 (concat-byte-vectors
                     stack15
                     (emit-aarch64-ldr-x10-sp (+ 128 (native-value-window-spill-offset frame-base-slot-count 5))))
        stack17 (concat-byte-vectors stack16 (emit-aarch64-str-x10-sp 64))
        stack18 (concat-byte-vectors
                     stack17
                     (emit-aarch64-ldr-x10-sp (+ 128 (native-value-window-spill-offset frame-base-slot-count 4))))
        stack19 (concat-byte-vectors stack18 (emit-aarch64-str-x10-sp 72))
        stack20 (concat-byte-vectors
                     stack19
                     (emit-aarch64-ldr-x10-sp (+ 128 (native-value-window-spill-offset frame-base-slot-count 3))))
        stack21 (concat-byte-vectors stack20 (emit-aarch64-str-x10-sp 80))
        stack22 (concat-byte-vectors
                     stack21
                     (emit-aarch64-ldr-x10-sp (+ 128 (native-value-window-spill-offset frame-base-slot-count 2))))
        stack23 (concat-byte-vectors stack22 (emit-aarch64-str-x10-sp 88))
        stack24 (concat-byte-vectors
                     stack23
                     (emit-aarch64-ldr-x10-sp (+ 128 (native-value-window-spill-offset frame-base-slot-count 1))))
        stack25 (concat-byte-vectors stack24 (emit-aarch64-str-x10-sp 96))
        stack26 (concat-byte-vectors
                     stack25
                     (emit-aarch64-ldr-x10-sp (+ 128 (native-value-window-spill-offset frame-base-slot-count 0))))
        stack27 (concat-byte-vectors stack26 (emit-aarch64-str-x10-sp 104))
        stack28 (concat-byte-vectors stack27 (emit-aarch64-str-x9-sp 112))
        stack29 (concat-byte-vectors stack28 (emit-aarch64-str-x0-sp 120))
        reg0 (concat-byte-vectors stack29 (emit-aarch64-ldr-x7-sp (+ 128 (native-value-window-spill-offset frame-base-slot-count 14))))
        reg1 (concat-byte-vectors reg0 (emit-aarch64-ldr-x6-sp (+ 128 (native-value-window-spill-offset frame-base-slot-count 15))))
        reg2 (concat-byte-vectors reg1 (emit-aarch64-ldr-x5-sp (+ 128 (native-value-window-spill-offset frame-base-slot-count 16))))
        reg3 (concat-byte-vectors reg2 (emit-aarch64-ldr-x4-sp (+ 128 (native-value-window-spill-offset frame-base-slot-count 17))))
        reg4 (concat-byte-vectors reg3 (emit-aarch64-ldr-x3-sp (+ 128 (native-value-window-spill-offset frame-base-slot-count 18))))
        reg5 (concat-byte-vectors reg4 (emit-aarch64-ldr-x2-sp (+ 128 (native-value-window-spill-offset frame-base-slot-count 19))))
        reg6 (concat-byte-vectors reg5 (emit-aarch64-ldr-x1-sp (+ 128 (native-value-window-spill-offset frame-base-slot-count 20))))
        reg7 (concat-byte-vectors reg6 (emit-aarch64-ldr-x0-sp (+ 128 (native-value-window-spill-offset frame-base-slot-count 21))))
        call-seq (concat-byte-vectors
                   (emit-aarch64-bl disp)
                   (emit-aarch64-add-sp 128))]
    (concat-byte-vectors reg7 call-seq)))

(defn emit-twenty-three-arg-call-aarch64 [disp frame-base-slot-count]
  (let [stack0 (concat-byte-vectors
                 (emit-aarch64-sub-sp 128)
                 (emit-aarch64-ldr-x10-sp (+ 128 (native-value-window-spill-offset frame-base-slot-count 12))))
        stack1 (concat-byte-vectors stack0 (emit-aarch64-str-x10-sp 0))
        stack2 (concat-byte-vectors
                   stack1
                   (emit-aarch64-ldr-x10-sp (+ 128 (native-value-window-spill-offset frame-base-slot-count 11))))
        stack3 (concat-byte-vectors stack2 (emit-aarch64-str-x10-sp 8))
        stack4 (concat-byte-vectors
                   stack3
                   (emit-aarch64-ldr-x10-sp (+ 128 (native-value-window-spill-offset frame-base-slot-count 10))))
        stack5 (concat-byte-vectors stack4 (emit-aarch64-str-x10-sp 16))
        stack6 (concat-byte-vectors
                   stack5
                   (emit-aarch64-ldr-x10-sp (+ 128 (native-value-window-spill-offset frame-base-slot-count 9))))
        stack7 (concat-byte-vectors stack6 (emit-aarch64-str-x10-sp 24))
        stack8 (concat-byte-vectors
                   stack7
                   (emit-aarch64-ldr-x10-sp (+ 128 (native-value-window-spill-offset frame-base-slot-count 8))))
        stack9 (concat-byte-vectors stack8 (emit-aarch64-str-x10-sp 32))
        stack10 (concat-byte-vectors
                    stack9
                    (emit-aarch64-ldr-x10-sp (+ 128 (native-value-window-spill-offset frame-base-slot-count 7))))
        stack11 (concat-byte-vectors stack10 (emit-aarch64-str-x10-sp 40))
        stack12 (concat-byte-vectors
                    stack11
                    (emit-aarch64-ldr-x10-sp (+ 128 (native-value-window-spill-offset frame-base-slot-count 6))))
        stack13 (concat-byte-vectors stack12 (emit-aarch64-str-x10-sp 48))
        stack14 (concat-byte-vectors
                    stack13
                    (emit-aarch64-ldr-x10-sp (+ 128 (native-value-window-spill-offset frame-base-slot-count 5))))
        stack15 (concat-byte-vectors stack14 (emit-aarch64-str-x10-sp 56))
        stack16 (concat-byte-vectors
                    stack15
                    (emit-aarch64-ldr-x10-sp (+ 128 (native-value-window-spill-offset frame-base-slot-count 4))))
        stack17 (concat-byte-vectors stack16 (emit-aarch64-str-x10-sp 64))
        stack18 (concat-byte-vectors
                    stack17
                    (emit-aarch64-ldr-x10-sp (+ 128 (native-value-window-spill-offset frame-base-slot-count 3))))
        stack19 (concat-byte-vectors stack18 (emit-aarch64-str-x10-sp 72))
        stack20 (concat-byte-vectors
                    stack19
                    (emit-aarch64-ldr-x10-sp (+ 128 (native-value-window-spill-offset frame-base-slot-count 2))))
        stack21 (concat-byte-vectors stack20 (emit-aarch64-str-x10-sp 80))
        stack22 (concat-byte-vectors
                    stack21
                    (emit-aarch64-ldr-x10-sp (+ 128 (native-value-window-spill-offset frame-base-slot-count 1))))
        stack23 (concat-byte-vectors stack22 (emit-aarch64-str-x10-sp 88))
        stack24 (concat-byte-vectors
                    stack23
                    (emit-aarch64-ldr-x10-sp (+ 128 (native-value-window-spill-offset frame-base-slot-count 0))))
        stack25 (concat-byte-vectors stack24 (emit-aarch64-str-x10-sp 96))
        stack26 (concat-byte-vectors stack25 (emit-aarch64-str-x9-sp 104))
        stack27 (concat-byte-vectors stack26 (emit-aarch64-str-x0-sp 112))
        reg0 (concat-byte-vectors stack27 (emit-aarch64-ldr-x7-sp (+ 128 (native-value-window-spill-offset frame-base-slot-count 13))))
        reg1 (concat-byte-vectors reg0 (emit-aarch64-ldr-x6-sp (+ 128 (native-value-window-spill-offset frame-base-slot-count 14))))
        reg2 (concat-byte-vectors reg1 (emit-aarch64-ldr-x5-sp (+ 128 (native-value-window-spill-offset frame-base-slot-count 15))))
        reg3 (concat-byte-vectors reg2 (emit-aarch64-ldr-x4-sp (+ 128 (native-value-window-spill-offset frame-base-slot-count 16))))
        reg4 (concat-byte-vectors reg3 (emit-aarch64-ldr-x3-sp (+ 128 (native-value-window-spill-offset frame-base-slot-count 17))))
        reg5 (concat-byte-vectors reg4 (emit-aarch64-ldr-x2-sp (+ 128 (native-value-window-spill-offset frame-base-slot-count 18))))
        reg6 (concat-byte-vectors reg5 (emit-aarch64-ldr-x1-sp (+ 128 (native-value-window-spill-offset frame-base-slot-count 19))))
        reg7 (concat-byte-vectors reg6 (emit-aarch64-ldr-x0-sp (+ 128 (native-value-window-spill-offset frame-base-slot-count 20))))
        call-seq (concat-byte-vectors
                   (emit-aarch64-bl disp)
                   (emit-aarch64-add-sp 128))]
    (concat-byte-vectors reg7 call-seq)))

(defn emit-twenty-two-arg-call-aarch64 [disp frame-base-slot-count]
  (let [stack0 (concat-byte-vectors
                 (emit-aarch64-sub-sp 112)
                 (emit-aarch64-ldr-x10-sp (+ 112 (native-value-window-spill-offset frame-base-slot-count 11))))
        stack1 (concat-byte-vectors stack0 (emit-aarch64-str-x10-sp 0))
        stack2 (concat-byte-vectors
                  stack1
                  (emit-aarch64-ldr-x10-sp (+ 112 (native-value-window-spill-offset frame-base-slot-count 10))))
        stack3 (concat-byte-vectors stack2 (emit-aarch64-str-x10-sp 8))
        stack4 (concat-byte-vectors
                  stack3
                  (emit-aarch64-ldr-x10-sp (+ 112 (native-value-window-spill-offset frame-base-slot-count 9))))
        stack5 (concat-byte-vectors stack4 (emit-aarch64-str-x10-sp 16))
        stack6 (concat-byte-vectors
                  stack5
                  (emit-aarch64-ldr-x10-sp (+ 112 (native-value-window-spill-offset frame-base-slot-count 8))))
        stack7 (concat-byte-vectors stack6 (emit-aarch64-str-x10-sp 24))
        stack8 (concat-byte-vectors
                  stack7
                  (emit-aarch64-ldr-x10-sp (+ 112 (native-value-window-spill-offset frame-base-slot-count 7))))
        stack9 (concat-byte-vectors stack8 (emit-aarch64-str-x10-sp 32))
        stack10 (concat-byte-vectors
                   stack9
                   (emit-aarch64-ldr-x10-sp (+ 112 (native-value-window-spill-offset frame-base-slot-count 6))))
        stack11 (concat-byte-vectors stack10 (emit-aarch64-str-x10-sp 40))
        stack12 (concat-byte-vectors
                   stack11
                   (emit-aarch64-ldr-x10-sp (+ 112 (native-value-window-spill-offset frame-base-slot-count 5))))
        stack13 (concat-byte-vectors stack12 (emit-aarch64-str-x10-sp 48))
        stack14 (concat-byte-vectors
                   stack13
                   (emit-aarch64-ldr-x10-sp (+ 112 (native-value-window-spill-offset frame-base-slot-count 4))))
        stack15 (concat-byte-vectors stack14 (emit-aarch64-str-x10-sp 56))
        stack16 (concat-byte-vectors
                   stack15
                   (emit-aarch64-ldr-x10-sp (+ 112 (native-value-window-spill-offset frame-base-slot-count 3))))
        stack17 (concat-byte-vectors stack16 (emit-aarch64-str-x10-sp 64))
        stack18 (concat-byte-vectors
                   stack17
                   (emit-aarch64-ldr-x10-sp (+ 112 (native-value-window-spill-offset frame-base-slot-count 2))))
        stack19 (concat-byte-vectors stack18 (emit-aarch64-str-x10-sp 72))
        stack20 (concat-byte-vectors
                   stack19
                   (emit-aarch64-ldr-x10-sp (+ 112 (native-value-window-spill-offset frame-base-slot-count 1))))
        stack21 (concat-byte-vectors stack20 (emit-aarch64-str-x10-sp 80))
        stack22 (concat-byte-vectors
                   stack21
                   (emit-aarch64-ldr-x10-sp (+ 112 (native-value-window-spill-offset frame-base-slot-count 0))))
        stack23 (concat-byte-vectors stack22 (emit-aarch64-str-x10-sp 88))
        stack24 (concat-byte-vectors stack23 (emit-aarch64-str-x9-sp 96))
        stack25 (concat-byte-vectors stack24 (emit-aarch64-str-x0-sp 104))
        reg0 (concat-byte-vectors stack25 (emit-aarch64-ldr-x7-sp (+ 112 (native-value-window-spill-offset frame-base-slot-count 12))))
        reg1 (concat-byte-vectors reg0 (emit-aarch64-ldr-x6-sp (+ 112 (native-value-window-spill-offset frame-base-slot-count 13))))
        reg2 (concat-byte-vectors reg1 (emit-aarch64-ldr-x5-sp (+ 112 (native-value-window-spill-offset frame-base-slot-count 14))))
        reg3 (concat-byte-vectors reg2 (emit-aarch64-ldr-x4-sp (+ 112 (native-value-window-spill-offset frame-base-slot-count 15))))
        reg4 (concat-byte-vectors reg3 (emit-aarch64-ldr-x3-sp (+ 112 (native-value-window-spill-offset frame-base-slot-count 16))))
        reg5 (concat-byte-vectors reg4 (emit-aarch64-ldr-x2-sp (+ 112 (native-value-window-spill-offset frame-base-slot-count 17))))
        reg6 (concat-byte-vectors reg5 (emit-aarch64-ldr-x1-sp (+ 112 (native-value-window-spill-offset frame-base-slot-count 18))))
        reg-setup (concat-byte-vectors reg6 (emit-aarch64-ldr-x0-sp (+ 112 (native-value-window-spill-offset frame-base-slot-count 19))))
        call-seq (concat-byte-vectors
                   (emit-aarch64-bl disp)
                   (emit-aarch64-add-sp 112))]
    (concat-byte-vectors reg-setup call-seq)))

(defn emit-twenty-one-arg-call-aarch64 [disp frame-base-slot-count]
  (let [stack0 (concat-byte-vectors
                 (emit-aarch64-sub-sp 112)
                 (emit-aarch64-ldr-x10-sp (+ 112 (native-value-window-spill-offset frame-base-slot-count 10))))
        stack1 (concat-byte-vectors stack0 (emit-aarch64-str-x10-sp 0))
        stack2 (concat-byte-vectors
                 stack1
                 (emit-aarch64-ldr-x10-sp (+ 112 (native-value-window-spill-offset frame-base-slot-count 9))))
        stack3 (concat-byte-vectors stack2 (emit-aarch64-str-x10-sp 8))
        stack4 (concat-byte-vectors
                 stack3
                 (emit-aarch64-ldr-x10-sp (+ 112 (native-value-window-spill-offset frame-base-slot-count 8))))
        stack5 (concat-byte-vectors stack4 (emit-aarch64-str-x10-sp 16))
        stack6 (concat-byte-vectors
                 stack5
                 (emit-aarch64-ldr-x10-sp (+ 112 (native-value-window-spill-offset frame-base-slot-count 7))))
        stack7 (concat-byte-vectors stack6 (emit-aarch64-str-x10-sp 24))
        stack8 (concat-byte-vectors
                 stack7
                 (emit-aarch64-ldr-x10-sp (+ 112 (native-value-window-spill-offset frame-base-slot-count 6))))
        stack9 (concat-byte-vectors stack8 (emit-aarch64-str-x10-sp 32))
        stack10 (concat-byte-vectors
                  stack9
                  (emit-aarch64-ldr-x10-sp (+ 112 (native-value-window-spill-offset frame-base-slot-count 5))))
        stack11 (concat-byte-vectors stack10 (emit-aarch64-str-x10-sp 40))
        stack12 (concat-byte-vectors
                  stack11
                  (emit-aarch64-ldr-x10-sp (+ 112 (native-value-window-spill-offset frame-base-slot-count 4))))
        stack13 (concat-byte-vectors stack12 (emit-aarch64-str-x10-sp 48))
        stack14 (concat-byte-vectors
                  stack13
                  (emit-aarch64-ldr-x10-sp (+ 112 (native-value-window-spill-offset frame-base-slot-count 3))))
        stack15 (concat-byte-vectors stack14 (emit-aarch64-str-x10-sp 56))
        stack16 (concat-byte-vectors
                  stack15
                  (emit-aarch64-ldr-x10-sp (+ 112 (native-value-window-spill-offset frame-base-slot-count 2))))
        stack17 (concat-byte-vectors stack16 (emit-aarch64-str-x10-sp 64))
        stack18 (concat-byte-vectors
                  stack17
                  (emit-aarch64-ldr-x10-sp (+ 112 (native-value-window-spill-offset frame-base-slot-count 1))))
        stack19 (concat-byte-vectors stack18 (emit-aarch64-str-x10-sp 72))
        stack20 (concat-byte-vectors
                  stack19
                  (emit-aarch64-ldr-x10-sp (+ 112 (native-value-window-spill-offset frame-base-slot-count 0))))
        stack21 (concat-byte-vectors stack20 (emit-aarch64-str-x10-sp 80))
        stack22 (concat-byte-vectors stack21 (emit-aarch64-str-x9-sp 88))
        stack23 (concat-byte-vectors stack22 (emit-aarch64-str-x0-sp 96))
        reg0 (concat-byte-vectors stack23 (emit-aarch64-ldr-x7-sp (+ 112 (native-value-window-spill-offset frame-base-slot-count 11))))
        reg1 (concat-byte-vectors reg0 (emit-aarch64-ldr-x6-sp (+ 112 (native-value-window-spill-offset frame-base-slot-count 12))))
        reg2 (concat-byte-vectors reg1 (emit-aarch64-ldr-x5-sp (+ 112 (native-value-window-spill-offset frame-base-slot-count 13))))
        reg3 (concat-byte-vectors reg2 (emit-aarch64-ldr-x4-sp (+ 112 (native-value-window-spill-offset frame-base-slot-count 14))))
        reg4 (concat-byte-vectors reg3 (emit-aarch64-ldr-x3-sp (+ 112 (native-value-window-spill-offset frame-base-slot-count 15))))
        reg5 (concat-byte-vectors reg4 (emit-aarch64-ldr-x2-sp (+ 112 (native-value-window-spill-offset frame-base-slot-count 16))))
        reg6 (concat-byte-vectors reg5 (emit-aarch64-ldr-x1-sp (+ 112 (native-value-window-spill-offset frame-base-slot-count 17))))
        reg-setup (concat-byte-vectors reg6 (emit-aarch64-ldr-x0-sp (+ 112 (native-value-window-spill-offset frame-base-slot-count 18))))
        call-seq (concat-byte-vectors
                   (emit-aarch64-bl disp)
                   (emit-aarch64-add-sp 112))]
    (concat-byte-vectors reg-setup call-seq)))

(defn emit-twenty-arg-call-aarch64 [disp frame-base-slot-count]
  (let [stack0 (concat-byte-vectors
                 (emit-aarch64-sub-sp 96)
                 (emit-aarch64-ldr-x10-sp (+ 96 (native-value-window-spill-offset frame-base-slot-count 9))))
        stack1 (concat-byte-vectors stack0 (emit-aarch64-str-x10-sp 0))
        stack2 (concat-byte-vectors
                 stack1
                 (emit-aarch64-ldr-x10-sp (+ 96 (native-value-window-spill-offset frame-base-slot-count 8))))
        stack3 (concat-byte-vectors stack2 (emit-aarch64-str-x10-sp 8))
        stack4 (concat-byte-vectors
                 stack3
                 (emit-aarch64-ldr-x10-sp (+ 96 (native-value-window-spill-offset frame-base-slot-count 7))))
        stack5 (concat-byte-vectors stack4 (emit-aarch64-str-x10-sp 16))
        stack6 (concat-byte-vectors
                 stack5
                 (emit-aarch64-ldr-x10-sp (+ 96 (native-value-window-spill-offset frame-base-slot-count 6))))
        stack7 (concat-byte-vectors stack6 (emit-aarch64-str-x10-sp 24))
        stack8 (concat-byte-vectors
                 stack7
                 (emit-aarch64-ldr-x10-sp (+ 96 (native-value-window-spill-offset frame-base-slot-count 5))))
        stack9 (concat-byte-vectors stack8 (emit-aarch64-str-x10-sp 32))
        stack10 (concat-byte-vectors
                  stack9
                  (emit-aarch64-ldr-x10-sp (+ 96 (native-value-window-spill-offset frame-base-slot-count 4))))
        stack11 (concat-byte-vectors stack10 (emit-aarch64-str-x10-sp 40))
        stack12 (concat-byte-vectors
                  stack11
                  (emit-aarch64-ldr-x10-sp (+ 96 (native-value-window-spill-offset frame-base-slot-count 3))))
        stack13 (concat-byte-vectors stack12 (emit-aarch64-str-x10-sp 48))
        stack14 (concat-byte-vectors
                  stack13
                  (emit-aarch64-ldr-x10-sp (+ 96 (native-value-window-spill-offset frame-base-slot-count 2))))
        stack15 (concat-byte-vectors stack14 (emit-aarch64-str-x10-sp 56))
        stack16 (concat-byte-vectors
                  stack15
                  (emit-aarch64-ldr-x10-sp (+ 96 (native-value-window-spill-offset frame-base-slot-count 1))))
        stack17 (concat-byte-vectors stack16 (emit-aarch64-str-x10-sp 64))
        stack18 (concat-byte-vectors
                  stack17
                  (emit-aarch64-ldr-x10-sp (+ 96 (native-value-window-spill-offset frame-base-slot-count 0))))
        stack19 (concat-byte-vectors stack18 (emit-aarch64-str-x10-sp 72))
        stack20 (concat-byte-vectors stack19 (emit-aarch64-str-x9-sp 80))
        stack21 (concat-byte-vectors stack20 (emit-aarch64-str-x0-sp 88))
        reg0 (concat-byte-vectors stack21 (emit-aarch64-ldr-x7-sp (+ 96 (native-value-window-spill-offset frame-base-slot-count 10))))
        reg1 (concat-byte-vectors reg0 (emit-aarch64-ldr-x6-sp (+ 96 (native-value-window-spill-offset frame-base-slot-count 11))))
        reg2 (concat-byte-vectors reg1 (emit-aarch64-ldr-x5-sp (+ 96 (native-value-window-spill-offset frame-base-slot-count 12))))
        reg3 (concat-byte-vectors reg2 (emit-aarch64-ldr-x4-sp (+ 96 (native-value-window-spill-offset frame-base-slot-count 13))))
        reg4 (concat-byte-vectors reg3 (emit-aarch64-ldr-x3-sp (+ 96 (native-value-window-spill-offset frame-base-slot-count 14))))
        reg5 (concat-byte-vectors reg4 (emit-aarch64-ldr-x2-sp (+ 96 (native-value-window-spill-offset frame-base-slot-count 15))))
        reg6 (concat-byte-vectors reg5 (emit-aarch64-ldr-x1-sp (+ 96 (native-value-window-spill-offset frame-base-slot-count 16))))
        reg-setup (concat-byte-vectors reg6 (emit-aarch64-ldr-x0-sp (+ 96 (native-value-window-spill-offset frame-base-slot-count 17))))
        call-seq (concat-byte-vectors
                   (emit-aarch64-bl disp)
                   (emit-aarch64-add-sp 96))]
    (concat-byte-vectors reg-setup call-seq)))

(defn emit-nineteen-arg-call-aarch64 [disp frame-base-slot-count]
  (let [stack0 (concat-byte-vectors
                 (emit-aarch64-sub-sp 96)
                 (emit-aarch64-ldr-x10-sp (+ 96 (native-value-window-spill-offset frame-base-slot-count 8))))
        stack1 (concat-byte-vectors stack0 (emit-aarch64-str-x10-sp 0))
        stack2 (concat-byte-vectors
                 stack1
                 (emit-aarch64-ldr-x10-sp (+ 96 (native-value-window-spill-offset frame-base-slot-count 7))))
        stack3 (concat-byte-vectors stack2 (emit-aarch64-str-x10-sp 8))
        stack4 (concat-byte-vectors
                 stack3
                 (emit-aarch64-ldr-x10-sp (+ 96 (native-value-window-spill-offset frame-base-slot-count 6))))
        stack5 (concat-byte-vectors stack4 (emit-aarch64-str-x10-sp 16))
        stack6 (concat-byte-vectors
                 stack5
                 (emit-aarch64-ldr-x10-sp (+ 96 (native-value-window-spill-offset frame-base-slot-count 5))))
        stack7 (concat-byte-vectors stack6 (emit-aarch64-str-x10-sp 24))
        stack8 (concat-byte-vectors
                 stack7
                 (emit-aarch64-ldr-x10-sp (+ 96 (native-value-window-spill-offset frame-base-slot-count 4))))
        stack9 (concat-byte-vectors stack8 (emit-aarch64-str-x10-sp 32))
        stack10 (concat-byte-vectors
                  stack9
                  (emit-aarch64-ldr-x10-sp (+ 96 (native-value-window-spill-offset frame-base-slot-count 3))))
        stack11 (concat-byte-vectors stack10 (emit-aarch64-str-x10-sp 40))
        stack12 (concat-byte-vectors
                  stack11
                  (emit-aarch64-ldr-x10-sp (+ 96 (native-value-window-spill-offset frame-base-slot-count 2))))
        stack13 (concat-byte-vectors stack12 (emit-aarch64-str-x10-sp 48))
        stack14 (concat-byte-vectors
                  stack13
                  (emit-aarch64-ldr-x10-sp (+ 96 (native-value-window-spill-offset frame-base-slot-count 1))))
        stack15 (concat-byte-vectors stack14 (emit-aarch64-str-x10-sp 56))
        stack16 (concat-byte-vectors
                  stack15
                  (emit-aarch64-ldr-x10-sp (+ 96 (native-value-window-spill-offset frame-base-slot-count 0))))
        stack17 (concat-byte-vectors stack16 (emit-aarch64-str-x10-sp 64))
        stack18 (concat-byte-vectors stack17 (emit-aarch64-str-x9-sp 72))
        stack19 (concat-byte-vectors stack18 (emit-aarch64-str-x0-sp 80))
        reg0 (concat-byte-vectors stack19 (emit-aarch64-ldr-x7-sp (+ 96 (native-value-window-spill-offset frame-base-slot-count 9))))
        reg1 (concat-byte-vectors reg0 (emit-aarch64-ldr-x6-sp (+ 96 (native-value-window-spill-offset frame-base-slot-count 10))))
        reg2 (concat-byte-vectors reg1 (emit-aarch64-ldr-x5-sp (+ 96 (native-value-window-spill-offset frame-base-slot-count 11))))
        reg3 (concat-byte-vectors reg2 (emit-aarch64-ldr-x4-sp (+ 96 (native-value-window-spill-offset frame-base-slot-count 12))))
        reg4 (concat-byte-vectors reg3 (emit-aarch64-ldr-x3-sp (+ 96 (native-value-window-spill-offset frame-base-slot-count 13))))
        reg5 (concat-byte-vectors reg4 (emit-aarch64-ldr-x2-sp (+ 96 (native-value-window-spill-offset frame-base-slot-count 14))))
        reg6 (concat-byte-vectors reg5 (emit-aarch64-ldr-x1-sp (+ 96 (native-value-window-spill-offset frame-base-slot-count 15))))
        reg-setup (concat-byte-vectors reg6 (emit-aarch64-ldr-x0-sp (+ 96 (native-value-window-spill-offset frame-base-slot-count 16))))
        call-seq (concat-byte-vectors
                   (emit-aarch64-bl disp)
                   (emit-aarch64-add-sp 96))]
    (concat-byte-vectors reg-setup call-seq)))

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

(defn emit-drop-bundle-aarch64 [frame-base-slot-count current-depth]
  (if (>= current-depth 22)
    (concat-byte-vectors
      (emit-drop-bundle-aarch64 frame-base-slot-count 21)
      (concat-byte-vectors
        (emit-aarch64-ldr-x1-sp (native-value-window-spill-offset frame-base-slot-count 19))
        (emit-aarch64-str-x1-sp (native-value-window-spill-offset frame-base-slot-count 18))))
    (if (>= current-depth 21)
    (concat-byte-vectors
      (emit-drop-bundle-aarch64 frame-base-slot-count 20)
      (concat-byte-vectors
        (emit-aarch64-ldr-x1-sp (native-value-window-spill-offset frame-base-slot-count 18))
        (emit-aarch64-str-x1-sp (native-value-window-spill-offset frame-base-slot-count 17))))
    (if (>= current-depth 20)
    (concat-byte-vectors
      (emit-drop-bundle-aarch64 frame-base-slot-count 19)
      (concat-byte-vectors
        (emit-aarch64-ldr-x1-sp (native-value-window-spill-offset frame-base-slot-count 17))
        (emit-aarch64-str-x1-sp (native-value-window-spill-offset frame-base-slot-count 16))))
    (if (>= current-depth 19)
    (concat-byte-vectors
      (emit-drop-bundle-aarch64 frame-base-slot-count 18)
      (concat-byte-vectors
        (emit-aarch64-ldr-x1-sp (native-value-window-spill-offset frame-base-slot-count 16))
        (emit-aarch64-str-x1-sp (native-value-window-spill-offset frame-base-slot-count 15))))
    (if (>= current-depth 18)
    (concat-byte-vectors
      (emit-drop-bundle-aarch64 frame-base-slot-count 17)
      (concat-byte-vectors
        (emit-aarch64-ldr-x1-sp (native-value-window-spill-offset frame-base-slot-count 15))
        (emit-aarch64-str-x1-sp (native-value-window-spill-offset frame-base-slot-count 14))))
    (if (>= current-depth 17)
    (concat-byte-vectors
      (emit-drop-bundle-aarch64 frame-base-slot-count 16)
      (concat-byte-vectors
        (emit-aarch64-ldr-x1-sp (native-value-window-spill-offset frame-base-slot-count 14))
        (emit-aarch64-str-x1-sp (native-value-window-spill-offset frame-base-slot-count 13))))
    (if (>= current-depth 16)
    (concat-byte-vectors
      (emit-drop-bundle-aarch64 frame-base-slot-count 15)
      (concat-byte-vectors
        (emit-aarch64-ldr-x1-sp (native-value-window-spill-offset frame-base-slot-count 13))
        (emit-aarch64-str-x1-sp (native-value-window-spill-offset frame-base-slot-count 12))))
    (if (>= current-depth 15)
    (concat-byte-vectors
      (emit-drop-bundle-aarch64 frame-base-slot-count 14)
      (concat-byte-vectors
        (emit-aarch64-ldr-x1-sp (native-value-window-spill-offset frame-base-slot-count 12))
        (emit-aarch64-str-x1-sp (native-value-window-spill-offset frame-base-slot-count 11))))
    (if (>= current-depth 14)
    (concat-byte-vectors
      (emit-drop-bundle-aarch64 frame-base-slot-count 13)
      (concat-byte-vectors
        (emit-aarch64-ldr-x1-sp (native-value-window-spill-offset frame-base-slot-count 11))
        (emit-aarch64-str-x1-sp (native-value-window-spill-offset frame-base-slot-count 10))))
    (if (>= current-depth 13)
    (concat-byte-vectors
      (emit-drop-bundle-aarch64 frame-base-slot-count 12)
      (concat-byte-vectors
        (emit-aarch64-ldr-x1-sp (native-value-window-spill-offset frame-base-slot-count 10))
        (emit-aarch64-str-x1-sp (native-value-window-spill-offset frame-base-slot-count 9))))
    (if (>= current-depth 12)
    (concat-byte-vectors
      (concat-byte-vectors
        (concat-byte-vectors
          (concat-byte-vectors
            (concat-byte-vectors
              (concat-byte-vectors
                (concat-byte-vectors
                  (concat-byte-vectors
                    (concat-byte-vectors
                      (concat-byte-vectors
                        (concat-byte-vectors
                          (concat-byte-vectors
                            (concat-byte-vectors
                              (concat-byte-vectors
                                (emit-aarch64-mov-x0-x9)
                                (emit-aarch64-ldr-x1-sp (native-value-window-spill-offset frame-base-slot-count 1)))
                              (emit-aarch64-ldr-x9-sp (native-value-window-spill-offset frame-base-slot-count 0)))
                            (emit-aarch64-str-x1-sp (native-value-window-spill-offset frame-base-slot-count 0)))
                          (emit-aarch64-ldr-x1-sp (native-value-window-spill-offset frame-base-slot-count 2)))
                        (emit-aarch64-str-x1-sp (native-value-window-spill-offset frame-base-slot-count 1)))
                      (emit-aarch64-ldr-x1-sp (native-value-window-spill-offset frame-base-slot-count 3)))
                    (emit-aarch64-str-x1-sp (native-value-window-spill-offset frame-base-slot-count 2)))
                  (emit-aarch64-ldr-x1-sp (native-value-window-spill-offset frame-base-slot-count 4)))
                (emit-aarch64-str-x1-sp (native-value-window-spill-offset frame-base-slot-count 3)))
              (emit-aarch64-ldr-x1-sp (native-value-window-spill-offset frame-base-slot-count 5)))
            (emit-aarch64-str-x1-sp (native-value-window-spill-offset frame-base-slot-count 4)))
          (concat-byte-vectors
            (emit-aarch64-ldr-x1-sp (native-value-window-spill-offset frame-base-slot-count 6))
            (emit-aarch64-str-x1-sp (native-value-window-spill-offset frame-base-slot-count 5))))
        (concat-byte-vectors
          (emit-aarch64-ldr-x1-sp (native-value-window-spill-offset frame-base-slot-count 7))
          (emit-aarch64-str-x1-sp (native-value-window-spill-offset frame-base-slot-count 6))))
      (concat-byte-vectors
        (concat-byte-vectors
          (emit-aarch64-ldr-x1-sp (native-value-window-spill-offset frame-base-slot-count 8))
          (emit-aarch64-str-x1-sp (native-value-window-spill-offset frame-base-slot-count 7)))
        (concat-byte-vectors
          (emit-aarch64-ldr-x1-sp (native-value-window-spill-offset frame-base-slot-count 9))
          (emit-aarch64-str-x1-sp (native-value-window-spill-offset frame-base-slot-count 8)))))
    (if (>= current-depth 11)
    (concat-byte-vectors
      (concat-byte-vectors
        (concat-byte-vectors
          (concat-byte-vectors
            (concat-byte-vectors
              (concat-byte-vectors
                (concat-byte-vectors
                  (concat-byte-vectors
                    (concat-byte-vectors
                      (concat-byte-vectors
                        (concat-byte-vectors
                          (concat-byte-vectors
                            (concat-byte-vectors
                              (concat-byte-vectors
                                (emit-aarch64-mov-x0-x9)
                                (emit-aarch64-ldr-x1-sp (native-value-window-spill-offset frame-base-slot-count 1)))
                              (emit-aarch64-ldr-x9-sp (native-value-window-spill-offset frame-base-slot-count 0)))
                            (emit-aarch64-str-x1-sp (native-value-window-spill-offset frame-base-slot-count 0)))
                          (emit-aarch64-ldr-x1-sp (native-value-window-spill-offset frame-base-slot-count 2)))
                        (emit-aarch64-str-x1-sp (native-value-window-spill-offset frame-base-slot-count 1)))
                      (emit-aarch64-ldr-x1-sp (native-value-window-spill-offset frame-base-slot-count 3)))
                    (emit-aarch64-str-x1-sp (native-value-window-spill-offset frame-base-slot-count 2)))
                  (emit-aarch64-ldr-x1-sp (native-value-window-spill-offset frame-base-slot-count 4)))
                (emit-aarch64-str-x1-sp (native-value-window-spill-offset frame-base-slot-count 3)))
              (emit-aarch64-ldr-x1-sp (native-value-window-spill-offset frame-base-slot-count 5)))
            (emit-aarch64-str-x1-sp (native-value-window-spill-offset frame-base-slot-count 4)))
          (concat-byte-vectors
            (emit-aarch64-ldr-x1-sp (native-value-window-spill-offset frame-base-slot-count 6))
            (emit-aarch64-str-x1-sp (native-value-window-spill-offset frame-base-slot-count 5))))
        (concat-byte-vectors
          (emit-aarch64-ldr-x1-sp (native-value-window-spill-offset frame-base-slot-count 7))
          (emit-aarch64-str-x1-sp (native-value-window-spill-offset frame-base-slot-count 6))))
      (concat-byte-vectors
        (emit-aarch64-ldr-x1-sp (native-value-window-spill-offset frame-base-slot-count 8))
        (emit-aarch64-str-x1-sp (native-value-window-spill-offset frame-base-slot-count 7))))
    (if (>= current-depth 10)
    (concat-byte-vectors
      (concat-byte-vectors
        (concat-byte-vectors
          (concat-byte-vectors
            (concat-byte-vectors
              (concat-byte-vectors
                (concat-byte-vectors
                  (concat-byte-vectors
                    (concat-byte-vectors
                      (concat-byte-vectors
                        (concat-byte-vectors
                          (concat-byte-vectors
                            (concat-byte-vectors
                              (emit-aarch64-mov-x0-x9)
                              (emit-aarch64-ldr-x1-sp (native-value-window-spill-offset frame-base-slot-count 1)))
                            (emit-aarch64-ldr-x9-sp (native-value-window-spill-offset frame-base-slot-count 0)))
                          (emit-aarch64-str-x1-sp (native-value-window-spill-offset frame-base-slot-count 0)))
                        (emit-aarch64-ldr-x1-sp (native-value-window-spill-offset frame-base-slot-count 2)))
                      (emit-aarch64-str-x1-sp (native-value-window-spill-offset frame-base-slot-count 1)))
                    (emit-aarch64-ldr-x1-sp (native-value-window-spill-offset frame-base-slot-count 3)))
                  (emit-aarch64-str-x1-sp (native-value-window-spill-offset frame-base-slot-count 2)))
                (emit-aarch64-ldr-x1-sp (native-value-window-spill-offset frame-base-slot-count 4)))
              (emit-aarch64-str-x1-sp (native-value-window-spill-offset frame-base-slot-count 3)))
            (emit-aarch64-ldr-x1-sp (native-value-window-spill-offset frame-base-slot-count 5)))
          (emit-aarch64-str-x1-sp (native-value-window-spill-offset frame-base-slot-count 4)))
        (concat-byte-vectors
          (emit-aarch64-ldr-x1-sp (native-value-window-spill-offset frame-base-slot-count 6))
          (emit-aarch64-str-x1-sp (native-value-window-spill-offset frame-base-slot-count 5))))
      (concat-byte-vectors
        (emit-aarch64-ldr-x1-sp (native-value-window-spill-offset frame-base-slot-count 7))
        (emit-aarch64-str-x1-sp (native-value-window-spill-offset frame-base-slot-count 6))))
    (if (>= current-depth 9)
    (concat-byte-vectors
      (concat-byte-vectors
        (concat-byte-vectors
          (concat-byte-vectors
            (concat-byte-vectors
              (concat-byte-vectors
                (concat-byte-vectors
                  (concat-byte-vectors
                    (concat-byte-vectors
                      (concat-byte-vectors
                        (concat-byte-vectors
                          (concat-byte-vectors
                            (emit-aarch64-mov-x0-x9)
                            (emit-aarch64-ldr-x1-sp (native-value-window-spill-offset frame-base-slot-count 1)))
                          (emit-aarch64-ldr-x9-sp (native-value-window-spill-offset frame-base-slot-count 0)))
                        (emit-aarch64-str-x1-sp (native-value-window-spill-offset frame-base-slot-count 0)))
                      (emit-aarch64-ldr-x1-sp (native-value-window-spill-offset frame-base-slot-count 2)))
                    (emit-aarch64-str-x1-sp (native-value-window-spill-offset frame-base-slot-count 1)))
                  (emit-aarch64-ldr-x1-sp (native-value-window-spill-offset frame-base-slot-count 3)))
                (emit-aarch64-str-x1-sp (native-value-window-spill-offset frame-base-slot-count 2)))
              (emit-aarch64-ldr-x1-sp (native-value-window-spill-offset frame-base-slot-count 4)))
            (emit-aarch64-str-x1-sp (native-value-window-spill-offset frame-base-slot-count 3)))
          (emit-aarch64-ldr-x1-sp (native-value-window-spill-offset frame-base-slot-count 5)))
        (emit-aarch64-str-x1-sp (native-value-window-spill-offset frame-base-slot-count 4)))
      (concat-byte-vectors
        (emit-aarch64-ldr-x1-sp (native-value-window-spill-offset frame-base-slot-count 6))
        (emit-aarch64-str-x1-sp (native-value-window-spill-offset frame-base-slot-count 5))))
    (if (>= current-depth 8)
    (concat-byte-vectors
      (concat-byte-vectors
        (concat-byte-vectors
          (concat-byte-vectors
            (concat-byte-vectors
              (concat-byte-vectors
                (concat-byte-vectors
                  (concat-byte-vectors
                    (concat-byte-vectors
                      (concat-byte-vectors
                        (concat-byte-vectors
                          (emit-aarch64-mov-x0-x9)
                          (emit-aarch64-ldr-x1-sp (native-value-window-spill-offset frame-base-slot-count 1)))
                        (emit-aarch64-ldr-x9-sp (native-value-window-spill-offset frame-base-slot-count 0)))
                      (emit-aarch64-str-x1-sp (native-value-window-spill-offset frame-base-slot-count 0)))
                    (emit-aarch64-ldr-x1-sp (native-value-window-spill-offset frame-base-slot-count 2)))
                  (emit-aarch64-str-x1-sp (native-value-window-spill-offset frame-base-slot-count 1)))
                (emit-aarch64-ldr-x1-sp (native-value-window-spill-offset frame-base-slot-count 3)))
              (emit-aarch64-str-x1-sp (native-value-window-spill-offset frame-base-slot-count 2)))
            (emit-aarch64-ldr-x1-sp (native-value-window-spill-offset frame-base-slot-count 4)))
          (emit-aarch64-str-x1-sp (native-value-window-spill-offset frame-base-slot-count 3)))
        (emit-aarch64-ldr-x1-sp (native-value-window-spill-offset frame-base-slot-count 5)))
      (emit-aarch64-str-x1-sp (native-value-window-spill-offset frame-base-slot-count 4)))
    (if (>= current-depth 7)
    (concat-byte-vectors
      (concat-byte-vectors
        (concat-byte-vectors
          (concat-byte-vectors
            (concat-byte-vectors
              (concat-byte-vectors
                (concat-byte-vectors
                  (concat-byte-vectors
                    (concat-byte-vectors
                      (emit-aarch64-mov-x0-x9)
                      (emit-aarch64-ldr-x1-sp (native-value-window-spill-offset frame-base-slot-count 1)))
                    (emit-aarch64-ldr-x9-sp (native-value-window-spill-offset frame-base-slot-count 0)))
                  (emit-aarch64-str-x1-sp (native-value-window-spill-offset frame-base-slot-count 0)))
                (emit-aarch64-ldr-x1-sp (native-value-window-spill-offset frame-base-slot-count 2)))
              (emit-aarch64-str-x1-sp (native-value-window-spill-offset frame-base-slot-count 1)))
            (emit-aarch64-ldr-x1-sp (native-value-window-spill-offset frame-base-slot-count 3)))
          (emit-aarch64-str-x1-sp (native-value-window-spill-offset frame-base-slot-count 2)))
        (emit-aarch64-ldr-x1-sp (native-value-window-spill-offset frame-base-slot-count 4)))
      (emit-aarch64-str-x1-sp (native-value-window-spill-offset frame-base-slot-count 3)))
    (if (>= current-depth 6)
    (concat-byte-vectors
      (concat-byte-vectors
        (concat-byte-vectors
          (concat-byte-vectors
            (concat-byte-vectors
              (concat-byte-vectors
                (concat-byte-vectors
                  (emit-aarch64-mov-x0-x9)
                  (emit-aarch64-ldr-x1-sp (native-value-window-spill-offset frame-base-slot-count 1)))
                (emit-aarch64-ldr-x9-sp (native-value-window-spill-offset frame-base-slot-count 0)))
              (emit-aarch64-str-x1-sp (native-value-window-spill-offset frame-base-slot-count 0)))
            (emit-aarch64-ldr-x1-sp (native-value-window-spill-offset frame-base-slot-count 2)))
          (emit-aarch64-str-x1-sp (native-value-window-spill-offset frame-base-slot-count 1)))
        (emit-aarch64-ldr-x1-sp (native-value-window-spill-offset frame-base-slot-count 3)))
      (emit-aarch64-str-x1-sp (native-value-window-spill-offset frame-base-slot-count 2)))
    (if (>= current-depth 5)
    (concat-byte-vectors
      (concat-byte-vectors
        (concat-byte-vectors
          (concat-byte-vectors
            (concat-byte-vectors
              (emit-aarch64-mov-x0-x9)
              (emit-aarch64-ldr-x10-sp (native-value-window-spill-offset frame-base-slot-count 2)))
            (emit-aarch64-ldr-x9-sp (native-value-window-spill-offset frame-base-slot-count 0)))
          (emit-aarch64-ldr-x1-sp (native-value-window-spill-offset frame-base-slot-count 1)))
        (emit-aarch64-str-x1-sp (native-value-window-spill-offset frame-base-slot-count 0)))
      (emit-aarch64-str-x10-sp (native-value-window-spill-offset frame-base-slot-count 1)))
    (if (>= current-depth 4)
      (concat-byte-vectors
        (concat-byte-vectors
          (concat-byte-vectors
            (emit-aarch64-mov-x0-x9)
            (emit-aarch64-ldr-x10-sp (native-value-window-spill-offset frame-base-slot-count 1)))
          (emit-aarch64-ldr-x9-sp (native-value-window-spill-offset frame-base-slot-count 0)))
        (emit-aarch64-str-x10-sp (native-value-window-spill-offset frame-base-slot-count 0)))
        (if (>= current-depth 3)
          (concat-byte-vectors
            (emit-aarch64-mov-x0-x9)
            (emit-aarch64-ldr-x9-sp (native-value-window-spill-offset frame-base-slot-count 0)))
        (emit-aarch64-mov-x0-x9))))))))))))))))))))))

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

(defn native-call-bundle-size-aarch64-twenty-to-twenty-two [target-param-count]
  (if (= target-param-count 22)
    148
    (if (= target-param-count 21)
      140
      128)))

(defn native-call-bundle-size-aarch64-twenty-to-twenty-three [target-param-count]
  (if (= target-param-count 23)
    156
    (native-call-bundle-size-aarch64-twenty-to-twenty-two target-param-count)))

(defn native-call-bundle-size-aarch64-twenty-to-twenty-four [target-param-count]
  (if (= target-param-count 24)
    164
    (native-call-bundle-size-aarch64-twenty-to-twenty-three target-param-count)))

(defn native-call-bundle-size-aarch64-twenty-to-twenty-five [target-param-count]
  (if (= target-param-count 25)
    172
    (native-call-bundle-size-aarch64-twenty-to-twenty-four target-param-count)))

(defn native-call-bundle-size-aarch64-twenty-to-twenty-six [target-param-count]
  (if (= target-param-count 26)
    180
    (native-call-bundle-size-aarch64-twenty-to-twenty-five target-param-count)))

(defn native-call-bundle-size-aarch64-twenty-to-twenty-seven [target-param-count]
  (if (= target-param-count 27)
    188
    (native-call-bundle-size-aarch64-twenty-to-twenty-six target-param-count)))

(defn native-call-bundle-size-aarch64-twenty-to-twenty-eight [target-param-count]
  (if (= target-param-count 28)
    196
    (native-call-bundle-size-aarch64-twenty-to-twenty-seven target-param-count)))

(defn native-call-bundle-disp-aarch64-twenty-to-twenty-two [target-param-count target-offset current-offset]
  (if (= target-param-count 22)
    (- target-offset (+ current-offset 140))
    (if (= target-param-count 21)
      (- target-offset (+ current-offset 132))
      (- target-offset (+ current-offset 120)))))

(defn native-call-bundle-disp-aarch64-twenty-to-twenty-three [target-param-count target-offset current-offset]
  (if (= target-param-count 23)
    (- target-offset (+ current-offset 148))
    (native-call-bundle-disp-aarch64-twenty-to-twenty-two target-param-count target-offset current-offset)))

(defn native-call-bundle-disp-aarch64-twenty-to-twenty-four [target-param-count target-offset current-offset]
  (if (= target-param-count 24)
    (- target-offset (+ current-offset 156))
    (native-call-bundle-disp-aarch64-twenty-to-twenty-three target-param-count target-offset current-offset)))

(defn native-call-bundle-disp-aarch64-twenty-to-twenty-five [target-param-count target-offset current-offset]
  (if (= target-param-count 25)
    (- target-offset (+ current-offset 164))
    (native-call-bundle-disp-aarch64-twenty-to-twenty-four target-param-count target-offset current-offset)))

(defn native-call-bundle-disp-aarch64-twenty-to-twenty-six [target-param-count target-offset current-offset]
  (if (= target-param-count 26)
    (- target-offset (+ current-offset 172))
    (native-call-bundle-disp-aarch64-twenty-to-twenty-five target-param-count target-offset current-offset)))

(defn native-call-bundle-disp-aarch64-twenty-to-twenty-seven [target-param-count target-offset current-offset]
  (if (= target-param-count 27)
    (- target-offset (+ current-offset 180))
    (native-call-bundle-disp-aarch64-twenty-to-twenty-six target-param-count target-offset current-offset)))

(defn native-call-bundle-disp-aarch64-twenty-to-twenty-eight [target-param-count target-offset current-offset]
  (if (= target-param-count 28)
    (- target-offset (+ current-offset 188))
    (native-call-bundle-disp-aarch64-twenty-to-twenty-seven target-param-count target-offset current-offset)))

(defn native-instr-size-aarch64 [opcode operand function-metas current-depth]
  (if (= opcode 40)
    (let [target-meta (vector-get function-metas operand)
      target-param-count (native-function-param-count target-meta)
      size (if (>= target-param-count 20)
             (native-call-bundle-size-aarch64-twenty-to-twenty-eight target-param-count)
              (if (> target-param-count 9)
                 (+ 52 (* (- target-param-count 10) 8))
                (if (= target-param-count 9)
                48
                (if (= target-param-count 8)
                  36
                  (if (= target-param-count 7)
                    32
                    (if (= target-param-count 6)
                      28
                      (if (= target-param-count 5)
                        24
                        (if (= target-param-count 4)
                          20
                          (if (= target-param-count 3)
                            16
                            (if (= target-param-count 2)
                              (if (>= current-depth 3) 16 12)
                               (if (= target-param-count 1)
                                 12
                                  4)))))))))))]
      size)
    (if (= opcode 3)
      (if (>= current-depth 2) (+ 12 (* (- current-depth 2) 8)) 8)
      (if (= opcode 10)
        (if (>= current-depth 2) (+ 12 (* (- current-depth 2) 8)) 8)
        (if (= opcode 44)
          (if (>= current-depth 3) (+ 8 (* (- current-depth 3) 8)) 4)
          (vector-length (codegen-ir-instr-aarch64 opcode operand)))))))

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
    param-spill-bytes (if (> param-count 8)
                        (+ 40 (* (- param-count 9) 8))
                        (if (= param-count 8)
                        32
                        (if (= param-count 7)
                          28
                          (if (= param-count 6)
                            24
                            (if (= param-count 5)
                              20
                              (if (= param-count 4)
                                16
                                (if (= param-count 3)
                                  12
                                    (if (= param-count 2)
                                      8
                                           (if (= param-count 1) 4 0)))))))))
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

(defn emit-call-bundle-aarch64-twenty-to-twenty-two [target-param-count disp frame-base-slot-count]
  (if (= target-param-count 22)
    (emit-twenty-two-arg-call-aarch64 disp frame-base-slot-count)
    (if (= target-param-count 21)
      (emit-twenty-one-arg-call-aarch64 disp frame-base-slot-count)
      (emit-twenty-arg-call-aarch64 disp frame-base-slot-count))))

(defn emit-call-bundle-aarch64-twenty-to-twenty-three [target-param-count disp frame-base-slot-count]
  (if (= target-param-count 23)
    (emit-twenty-three-arg-call-aarch64 disp frame-base-slot-count)
    (emit-call-bundle-aarch64-twenty-to-twenty-two target-param-count disp frame-base-slot-count)))

(defn emit-call-bundle-aarch64-twenty-to-twenty-four [target-param-count disp frame-base-slot-count]
  (if (= target-param-count 24)
    (emit-twenty-four-arg-call-aarch64 disp frame-base-slot-count)
    (emit-call-bundle-aarch64-twenty-to-twenty-three target-param-count disp frame-base-slot-count)))

(defn emit-call-bundle-aarch64-twenty-to-twenty-five [target-param-count disp frame-base-slot-count]
  (if (= target-param-count 25)
    (emit-twenty-five-arg-call-aarch64 disp frame-base-slot-count)
    (emit-call-bundle-aarch64-twenty-to-twenty-four target-param-count disp frame-base-slot-count)))

(defn emit-call-bundle-aarch64-twenty-to-twenty-six [target-param-count disp frame-base-slot-count]
  (if (= target-param-count 26)
    (emit-twenty-six-arg-call-aarch64 disp frame-base-slot-count)
    (emit-call-bundle-aarch64-twenty-to-twenty-five target-param-count disp frame-base-slot-count)))

(defn emit-call-bundle-aarch64-twenty-to-twenty-seven [target-param-count disp frame-base-slot-count]
  (if (= target-param-count 27)
    (emit-twenty-seven-arg-call-aarch64 disp frame-base-slot-count)
    (emit-call-bundle-aarch64-twenty-to-twenty-six target-param-count disp frame-base-slot-count)))

(defn emit-call-bundle-aarch64-twenty-to-twenty-eight [target-param-count disp frame-base-slot-count]
  (if (= target-param-count 28)
    (emit-twenty-eight-arg-call-aarch64 disp frame-base-slot-count)
    (emit-call-bundle-aarch64-twenty-to-twenty-seven target-param-count disp frame-base-slot-count)))

(defn emit-call-bundle-aarch64-ten-to-nineteen [target-param-count disp frame-base-slot-count]
  (if (= target-param-count 19)
    (emit-nineteen-arg-call-aarch64 disp frame-base-slot-count)
    (if (= target-param-count 18)
      (emit-eighteen-arg-call-aarch64 disp frame-base-slot-count)
      (if (= target-param-count 17)
        (emit-seventeen-arg-call-aarch64 disp frame-base-slot-count)
        (if (= target-param-count 16)
          (emit-sixteen-arg-call-aarch64 disp frame-base-slot-count)
          (if (= target-param-count 15)
            (emit-fifteen-arg-call-aarch64 disp frame-base-slot-count)
            (if (= target-param-count 14)
              (emit-fourteen-arg-call-aarch64 disp frame-base-slot-count)
              (if (= target-param-count 13)
                (emit-thirteen-arg-call-aarch64 disp frame-base-slot-count)
                (if (= target-param-count 12)
                  (emit-twelve-arg-call-aarch64 disp frame-base-slot-count)
                  (if (= target-param-count 11)
                    (emit-eleven-arg-call-aarch64 disp frame-base-slot-count)
                    (emit-ten-arg-call-aarch64 disp frame-base-slot-count)))))))))))

(defn emit-call-bundle-aarch64-one-to-nine [target-param-count disp frame-base-slot-count current-depth]
  (if (= target-param-count 9)
    (emit-nine-arg-call-aarch64 disp frame-base-slot-count)
    (if (= target-param-count 8)
      (emit-eight-arg-call-aarch64 disp frame-base-slot-count)
      (if (= target-param-count 7)
        (emit-seven-arg-call-aarch64 disp frame-base-slot-count)
        (if (= target-param-count 6)
          (emit-six-arg-call-aarch64 disp frame-base-slot-count)
          (if (= target-param-count 5)
            (emit-five-arg-call-aarch64 disp frame-base-slot-count)
            (if (= target-param-count 4)
              (emit-four-arg-call-aarch64 disp frame-base-slot-count)
              (if (= target-param-count 3)
                (emit-three-arg-call-aarch64 disp frame-base-slot-count)
                (if (= target-param-count 2)
                  (emit-two-arg-call-aarch64 disp frame-base-slot-count current-depth)
                  (if (= target-param-count 1)
                    (let [save-prev (emit-aarch64-mov-x10-x9)
                      call-bl (emit-aarch64-bl disp)
                      restore-prev (emit-aarch64-mov-x9-x10)
                      bytes (vector-new 12)
                      b1 (vector-push bytes (vector-get save-prev 0))
                      b2 (vector-push b1 (vector-get save-prev 1))
                      b3 (vector-push b2 (vector-get save-prev 2))
                      b4 (vector-push b3 (vector-get save-prev 3))
                      b5 (vector-push b4 (vector-get call-bl 0))
                      b6 (vector-push b5 (vector-get call-bl 1))
                      b7 (vector-push b6 (vector-get call-bl 2))
                      b8 (vector-push b7 (vector-get call-bl 3))
                      b9 (vector-push b8 (vector-get restore-prev 0))
                      b10 (vector-push b9 (vector-get restore-prev 1))
                      b11 (vector-push b10 (vector-get restore-prev 2))
                      b12 (vector-push b11 (vector-get restore-prev 3))]
                      b12)
                    (emit-aarch64-bl disp)))))))))))

(defn codegen-ir-instr-bundle-aarch64 [opcode operand current-offset function-starts function-metas frame-base-slot-count current-depth]
  (if (= opcode 40)
    (let [target-offset (vector-get function-starts operand)
      target-meta (vector-get function-metas operand)
      target-param-count (native-function-param-count target-meta)
      disp (if (>= target-param-count 20)
             (native-call-bundle-disp-aarch64-twenty-to-twenty-eight target-param-count target-offset current-offset)
                (if (> target-param-count 9)
                  (- target-offset (+ current-offset (+ 44 (* (- target-param-count 10) 8))))
              (if (= target-param-count 9)
                (- target-offset (+ current-offset 40))
                (if (= target-param-count 8)
                  (- target-offset (+ current-offset 32))
                  (if (= target-param-count 7)
                    (- target-offset (+ current-offset 28))
                    (if (= target-param-count 6)
                      (- target-offset (+ current-offset 24))
                      (if (= target-param-count 5)
                        (- target-offset (+ current-offset 20))
                        (if (= target-param-count 4)
                          (- target-offset (+ current-offset 16))
                          (if (= target-param-count 3)
                            (- target-offset (+ current-offset 12))
                            (if (= target-param-count 2)
                              (- target-offset (+ current-offset 8))
                               (if (= target-param-count 1)
                                 (- target-offset (+ current-offset 4))
                                 (- target-offset current-offset))))))))))))
      call-bytes (if (>= target-param-count 20)
                    (emit-call-bundle-aarch64-twenty-to-twenty-eight target-param-count disp frame-base-slot-count)
                     (if (>= target-param-count 10)
                       (emit-call-bundle-aarch64-ten-to-nineteen target-param-count disp frame-base-slot-count)
                       (emit-call-bundle-aarch64-one-to-nine target-param-count disp frame-base-slot-count current-depth)))]
      call-bytes)
    (if (= opcode 3)
      (emit-i32-const-bundle-aarch64 operand frame-base-slot-count current-depth)
      (if (= opcode 10)
        (emit-local-get-bundle-aarch64 (local-slot-offset operand) frame-base-slot-count current-depth)
        (if (= opcode 44)
          (emit-drop-bundle-aarch64 frame-base-slot-count current-depth)
          (codegen-ir-instr-aarch64 opcode operand))))))

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

(defn spill-native-function-params-aarch64-twenty-to-twenty-two [param-count result stack-arg-base-offset]
  (if (= param-count 22)
    (do
      (append-native-bytes-loop result (emit-aarch64-str-x0-sp (local-slot-offset 0)) 0 4)
      (append-native-bytes-loop result (emit-aarch64-str-x1-sp (local-slot-offset 1)) 0 4)
      (append-native-bytes-loop result (emit-aarch64-str-x2-sp (local-slot-offset 2)) 0 4)
      (append-native-bytes-loop result (emit-aarch64-str-x3-sp (local-slot-offset 3)) 0 4)
      (append-native-bytes-loop result (emit-aarch64-str-x4-sp (local-slot-offset 4)) 0 4)
      (append-native-bytes-loop result (emit-aarch64-str-x5-sp (local-slot-offset 5)) 0 4)
      (append-native-bytes-loop result (emit-aarch64-str-x6-sp (local-slot-offset 6)) 0 4)
      (append-native-bytes-loop result (emit-aarch64-str-x7-sp (local-slot-offset 7)) 0 4)
      (append-native-bytes-loop result (emit-aarch64-ldr-x10-sp stack-arg-base-offset) 0 4)
      (append-native-bytes-loop result (emit-aarch64-str-x10-sp (local-slot-offset 8)) 0 4)
      (append-native-bytes-loop result (emit-aarch64-ldr-x10-sp (+ stack-arg-base-offset 8)) 0 4)
      (append-native-bytes-loop result (emit-aarch64-str-x10-sp (local-slot-offset 9)) 0 4)
      (append-native-bytes-loop result (emit-aarch64-ldr-x10-sp (+ stack-arg-base-offset 16)) 0 4)
      (append-native-bytes-loop result (emit-aarch64-str-x10-sp (local-slot-offset 10)) 0 4)
      (append-native-bytes-loop result (emit-aarch64-ldr-x10-sp (+ stack-arg-base-offset 24)) 0 4)
      (append-native-bytes-loop result (emit-aarch64-str-x10-sp (local-slot-offset 11)) 0 4)
      (append-native-bytes-loop result (emit-aarch64-ldr-x10-sp (+ stack-arg-base-offset 32)) 0 4)
      (append-native-bytes-loop result (emit-aarch64-str-x10-sp (local-slot-offset 12)) 0 4)
      (append-native-bytes-loop result (emit-aarch64-ldr-x10-sp (+ stack-arg-base-offset 40)) 0 4)
      (append-native-bytes-loop result (emit-aarch64-str-x10-sp (local-slot-offset 13)) 0 4)
      (append-native-bytes-loop result (emit-aarch64-ldr-x10-sp (+ stack-arg-base-offset 48)) 0 4)
      (append-native-bytes-loop result (emit-aarch64-str-x10-sp (local-slot-offset 14)) 0 4)
      (append-native-bytes-loop result (emit-aarch64-ldr-x10-sp (+ stack-arg-base-offset 56)) 0 4)
      (append-native-bytes-loop result (emit-aarch64-str-x10-sp (local-slot-offset 15)) 0 4)
      (append-native-bytes-loop result (emit-aarch64-ldr-x10-sp (+ stack-arg-base-offset 64)) 0 4)
      (append-native-bytes-loop result (emit-aarch64-str-x10-sp (local-slot-offset 16)) 0 4)
      (append-native-bytes-loop result (emit-aarch64-ldr-x10-sp (+ stack-arg-base-offset 72)) 0 4)
      (append-native-bytes-loop result (emit-aarch64-str-x10-sp (local-slot-offset 17)) 0 4)
      (append-native-bytes-loop result (emit-aarch64-ldr-x10-sp (+ stack-arg-base-offset 80)) 0 4)
      (append-native-bytes-loop result (emit-aarch64-str-x10-sp (local-slot-offset 18)) 0 4)
      (append-native-bytes-loop result (emit-aarch64-ldr-x10-sp (+ stack-arg-base-offset 88)) 0 4)
      (append-native-bytes-loop result (emit-aarch64-str-x10-sp (local-slot-offset 19)) 0 4)
      (append-native-bytes-loop result (emit-aarch64-ldr-x10-sp (+ stack-arg-base-offset 96)) 0 4)
      (append-native-bytes-loop result (emit-aarch64-str-x10-sp (local-slot-offset 20)) 0 4)
      (append-native-bytes-loop result (emit-aarch64-ldr-x10-sp (+ stack-arg-base-offset 104)) 0 4)
      (append-native-bytes-loop result (emit-aarch64-str-x10-sp (local-slot-offset 21)) 0 4))
    (if (= param-count 21)
      (do
        (append-native-bytes-loop result (emit-aarch64-str-x0-sp (local-slot-offset 0)) 0 4)
        (append-native-bytes-loop result (emit-aarch64-str-x1-sp (local-slot-offset 1)) 0 4)
        (append-native-bytes-loop result (emit-aarch64-str-x2-sp (local-slot-offset 2)) 0 4)
        (append-native-bytes-loop result (emit-aarch64-str-x3-sp (local-slot-offset 3)) 0 4)
        (append-native-bytes-loop result (emit-aarch64-str-x4-sp (local-slot-offset 4)) 0 4)
        (append-native-bytes-loop result (emit-aarch64-str-x5-sp (local-slot-offset 5)) 0 4)
        (append-native-bytes-loop result (emit-aarch64-str-x6-sp (local-slot-offset 6)) 0 4)
        (append-native-bytes-loop result (emit-aarch64-str-x7-sp (local-slot-offset 7)) 0 4)
        (append-native-bytes-loop result (emit-aarch64-ldr-x10-sp stack-arg-base-offset) 0 4)
        (append-native-bytes-loop result (emit-aarch64-str-x10-sp (local-slot-offset 8)) 0 4)
        (append-native-bytes-loop result (emit-aarch64-ldr-x10-sp (+ stack-arg-base-offset 8)) 0 4)
        (append-native-bytes-loop result (emit-aarch64-str-x10-sp (local-slot-offset 9)) 0 4)
        (append-native-bytes-loop result (emit-aarch64-ldr-x10-sp (+ stack-arg-base-offset 16)) 0 4)
        (append-native-bytes-loop result (emit-aarch64-str-x10-sp (local-slot-offset 10)) 0 4)
        (append-native-bytes-loop result (emit-aarch64-ldr-x10-sp (+ stack-arg-base-offset 24)) 0 4)
        (append-native-bytes-loop result (emit-aarch64-str-x10-sp (local-slot-offset 11)) 0 4)
        (append-native-bytes-loop result (emit-aarch64-ldr-x10-sp (+ stack-arg-base-offset 32)) 0 4)
        (append-native-bytes-loop result (emit-aarch64-str-x10-sp (local-slot-offset 12)) 0 4)
        (append-native-bytes-loop result (emit-aarch64-ldr-x10-sp (+ stack-arg-base-offset 40)) 0 4)
        (append-native-bytes-loop result (emit-aarch64-str-x10-sp (local-slot-offset 13)) 0 4)
        (append-native-bytes-loop result (emit-aarch64-ldr-x10-sp (+ stack-arg-base-offset 48)) 0 4)
        (append-native-bytes-loop result (emit-aarch64-str-x10-sp (local-slot-offset 14)) 0 4)
        (append-native-bytes-loop result (emit-aarch64-ldr-x10-sp (+ stack-arg-base-offset 56)) 0 4)
        (append-native-bytes-loop result (emit-aarch64-str-x10-sp (local-slot-offset 15)) 0 4)
        (append-native-bytes-loop result (emit-aarch64-ldr-x10-sp (+ stack-arg-base-offset 64)) 0 4)
        (append-native-bytes-loop result (emit-aarch64-str-x10-sp (local-slot-offset 16)) 0 4)
        (append-native-bytes-loop result (emit-aarch64-ldr-x10-sp (+ stack-arg-base-offset 72)) 0 4)
        (append-native-bytes-loop result (emit-aarch64-str-x10-sp (local-slot-offset 17)) 0 4)
        (append-native-bytes-loop result (emit-aarch64-ldr-x10-sp (+ stack-arg-base-offset 80)) 0 4)
        (append-native-bytes-loop result (emit-aarch64-str-x10-sp (local-slot-offset 18)) 0 4)
        (append-native-bytes-loop result (emit-aarch64-ldr-x10-sp (+ stack-arg-base-offset 88)) 0 4)
        (append-native-bytes-loop result (emit-aarch64-str-x10-sp (local-slot-offset 19)) 0 4)
        (append-native-bytes-loop result (emit-aarch64-ldr-x10-sp (+ stack-arg-base-offset 96)) 0 4)
        (append-native-bytes-loop result (emit-aarch64-str-x10-sp (local-slot-offset 20)) 0 4))
      (do
        (append-native-bytes-loop result (emit-aarch64-str-x0-sp (local-slot-offset 0)) 0 4)
        (append-native-bytes-loop result (emit-aarch64-str-x1-sp (local-slot-offset 1)) 0 4)
        (append-native-bytes-loop result (emit-aarch64-str-x2-sp (local-slot-offset 2)) 0 4)
        (append-native-bytes-loop result (emit-aarch64-str-x3-sp (local-slot-offset 3)) 0 4)
        (append-native-bytes-loop result (emit-aarch64-str-x4-sp (local-slot-offset 4)) 0 4)
        (append-native-bytes-loop result (emit-aarch64-str-x5-sp (local-slot-offset 5)) 0 4)
        (append-native-bytes-loop result (emit-aarch64-str-x6-sp (local-slot-offset 6)) 0 4)
        (append-native-bytes-loop result (emit-aarch64-str-x7-sp (local-slot-offset 7)) 0 4)
        (append-native-bytes-loop result (emit-aarch64-ldr-x10-sp stack-arg-base-offset) 0 4)
        (append-native-bytes-loop result (emit-aarch64-str-x10-sp (local-slot-offset 8)) 0 4)
        (append-native-bytes-loop result (emit-aarch64-ldr-x10-sp (+ stack-arg-base-offset 8)) 0 4)
        (append-native-bytes-loop result (emit-aarch64-str-x10-sp (local-slot-offset 9)) 0 4)
        (append-native-bytes-loop result (emit-aarch64-ldr-x10-sp (+ stack-arg-base-offset 16)) 0 4)
        (append-native-bytes-loop result (emit-aarch64-str-x10-sp (local-slot-offset 10)) 0 4)
        (append-native-bytes-loop result (emit-aarch64-ldr-x10-sp (+ stack-arg-base-offset 24)) 0 4)
        (append-native-bytes-loop result (emit-aarch64-str-x10-sp (local-slot-offset 11)) 0 4)
        (append-native-bytes-loop result (emit-aarch64-ldr-x10-sp (+ stack-arg-base-offset 32)) 0 4)
        (append-native-bytes-loop result (emit-aarch64-str-x10-sp (local-slot-offset 12)) 0 4)
        (append-native-bytes-loop result (emit-aarch64-ldr-x10-sp (+ stack-arg-base-offset 40)) 0 4)
        (append-native-bytes-loop result (emit-aarch64-str-x10-sp (local-slot-offset 13)) 0 4)
        (append-native-bytes-loop result (emit-aarch64-ldr-x10-sp (+ stack-arg-base-offset 48)) 0 4)
        (append-native-bytes-loop result (emit-aarch64-str-x10-sp (local-slot-offset 14)) 0 4)
        (append-native-bytes-loop result (emit-aarch64-ldr-x10-sp (+ stack-arg-base-offset 56)) 0 4)
        (append-native-bytes-loop result (emit-aarch64-str-x10-sp (local-slot-offset 15)) 0 4)
        (append-native-bytes-loop result (emit-aarch64-ldr-x10-sp (+ stack-arg-base-offset 64)) 0 4)
        (append-native-bytes-loop result (emit-aarch64-str-x10-sp (local-slot-offset 16)) 0 4)
        (append-native-bytes-loop result (emit-aarch64-ldr-x10-sp (+ stack-arg-base-offset 72)) 0 4)
        (append-native-bytes-loop result (emit-aarch64-str-x10-sp (local-slot-offset 17)) 0 4)
        (append-native-bytes-loop result (emit-aarch64-ldr-x10-sp (+ stack-arg-base-offset 80)) 0 4)
        (append-native-bytes-loop result (emit-aarch64-str-x10-sp (local-slot-offset 18)) 0 4)
        (append-native-bytes-loop result (emit-aarch64-ldr-x10-sp (+ stack-arg-base-offset 88)) 0 4)
        (append-native-bytes-loop result (emit-aarch64-str-x10-sp (local-slot-offset 19)) 0 4)))))

(defn spill-native-function-params-aarch64-twenty-to-twenty-three [param-count result stack-arg-base-offset]
  (if (= param-count 23)
    (do
      (append-native-bytes-loop result (emit-aarch64-str-x0-sp (local-slot-offset 0)) 0 4)
      (append-native-bytes-loop result (emit-aarch64-str-x1-sp (local-slot-offset 1)) 0 4)
      (append-native-bytes-loop result (emit-aarch64-str-x2-sp (local-slot-offset 2)) 0 4)
      (append-native-bytes-loop result (emit-aarch64-str-x3-sp (local-slot-offset 3)) 0 4)
      (append-native-bytes-loop result (emit-aarch64-str-x4-sp (local-slot-offset 4)) 0 4)
      (append-native-bytes-loop result (emit-aarch64-str-x5-sp (local-slot-offset 5)) 0 4)
      (append-native-bytes-loop result (emit-aarch64-str-x6-sp (local-slot-offset 6)) 0 4)
      (append-native-bytes-loop result (emit-aarch64-str-x7-sp (local-slot-offset 7)) 0 4)
      (append-native-bytes-loop result (emit-aarch64-ldr-x10-sp (+ stack-arg-base-offset 0)) 0 4)
      (append-native-bytes-loop result (emit-aarch64-str-x10-sp (local-slot-offset 8)) 0 4)
      (append-native-bytes-loop result (emit-aarch64-ldr-x10-sp (+ stack-arg-base-offset 8)) 0 4)
      (append-native-bytes-loop result (emit-aarch64-str-x10-sp (local-slot-offset 9)) 0 4)
      (append-native-bytes-loop result (emit-aarch64-ldr-x10-sp (+ stack-arg-base-offset 16)) 0 4)
      (append-native-bytes-loop result (emit-aarch64-str-x10-sp (local-slot-offset 10)) 0 4)
      (append-native-bytes-loop result (emit-aarch64-ldr-x10-sp (+ stack-arg-base-offset 24)) 0 4)
      (append-native-bytes-loop result (emit-aarch64-str-x10-sp (local-slot-offset 11)) 0 4)
      (append-native-bytes-loop result (emit-aarch64-ldr-x10-sp (+ stack-arg-base-offset 32)) 0 4)
      (append-native-bytes-loop result (emit-aarch64-str-x10-sp (local-slot-offset 12)) 0 4)
      (append-native-bytes-loop result (emit-aarch64-ldr-x10-sp (+ stack-arg-base-offset 40)) 0 4)
      (append-native-bytes-loop result (emit-aarch64-str-x10-sp (local-slot-offset 13)) 0 4)
      (append-native-bytes-loop result (emit-aarch64-ldr-x10-sp (+ stack-arg-base-offset 48)) 0 4)
      (append-native-bytes-loop result (emit-aarch64-str-x10-sp (local-slot-offset 14)) 0 4)
      (append-native-bytes-loop result (emit-aarch64-ldr-x10-sp (+ stack-arg-base-offset 56)) 0 4)
      (append-native-bytes-loop result (emit-aarch64-str-x10-sp (local-slot-offset 15)) 0 4)
      (append-native-bytes-loop result (emit-aarch64-ldr-x10-sp (+ stack-arg-base-offset 64)) 0 4)
      (append-native-bytes-loop result (emit-aarch64-str-x10-sp (local-slot-offset 16)) 0 4)
      (append-native-bytes-loop result (emit-aarch64-ldr-x10-sp (+ stack-arg-base-offset 72)) 0 4)
      (append-native-bytes-loop result (emit-aarch64-str-x10-sp (local-slot-offset 17)) 0 4)
      (append-native-bytes-loop result (emit-aarch64-ldr-x10-sp (+ stack-arg-base-offset 80)) 0 4)
      (append-native-bytes-loop result (emit-aarch64-str-x10-sp (local-slot-offset 18)) 0 4)
      (append-native-bytes-loop result (emit-aarch64-ldr-x10-sp (+ stack-arg-base-offset 88)) 0 4)
      (append-native-bytes-loop result (emit-aarch64-str-x10-sp (local-slot-offset 19)) 0 4)
      (append-native-bytes-loop result (emit-aarch64-ldr-x10-sp (+ stack-arg-base-offset 96)) 0 4)
      (append-native-bytes-loop result (emit-aarch64-str-x10-sp (local-slot-offset 20)) 0 4)
      (append-native-bytes-loop result (emit-aarch64-ldr-x10-sp (+ stack-arg-base-offset 104)) 0 4)
      (append-native-bytes-loop result (emit-aarch64-str-x10-sp (local-slot-offset 21)) 0 4)
      (append-native-bytes-loop result (emit-aarch64-ldr-x10-sp (+ stack-arg-base-offset 112)) 0 4)
       (append-native-bytes-loop result (emit-aarch64-str-x10-sp (local-slot-offset 22)) 0 4))
    (spill-native-function-params-aarch64-twenty-to-twenty-two param-count result stack-arg-base-offset)))

(defn spill-native-function-params-aarch64-twenty-to-twenty-four [param-count result stack-arg-base-offset]
  (if (= param-count 24)
    (do
      (append-native-bytes-loop result (emit-aarch64-str-x0-sp (local-slot-offset 0)) 0 4)
      (append-native-bytes-loop result (emit-aarch64-str-x1-sp (local-slot-offset 1)) 0 4)
      (append-native-bytes-loop result (emit-aarch64-str-x2-sp (local-slot-offset 2)) 0 4)
      (append-native-bytes-loop result (emit-aarch64-str-x3-sp (local-slot-offset 3)) 0 4)
      (append-native-bytes-loop result (emit-aarch64-str-x4-sp (local-slot-offset 4)) 0 4)
      (append-native-bytes-loop result (emit-aarch64-str-x5-sp (local-slot-offset 5)) 0 4)
      (append-native-bytes-loop result (emit-aarch64-str-x6-sp (local-slot-offset 6)) 0 4)
      (append-native-bytes-loop result (emit-aarch64-str-x7-sp (local-slot-offset 7)) 0 4)
      (append-native-bytes-loop result (emit-aarch64-ldr-x10-sp (+ stack-arg-base-offset 0)) 0 4)
      (append-native-bytes-loop result (emit-aarch64-str-x10-sp (local-slot-offset 8)) 0 4)
      (append-native-bytes-loop result (emit-aarch64-ldr-x10-sp (+ stack-arg-base-offset 8)) 0 4)
      (append-native-bytes-loop result (emit-aarch64-str-x10-sp (local-slot-offset 9)) 0 4)
      (append-native-bytes-loop result (emit-aarch64-ldr-x10-sp (+ stack-arg-base-offset 16)) 0 4)
      (append-native-bytes-loop result (emit-aarch64-str-x10-sp (local-slot-offset 10)) 0 4)
      (append-native-bytes-loop result (emit-aarch64-ldr-x10-sp (+ stack-arg-base-offset 24)) 0 4)
      (append-native-bytes-loop result (emit-aarch64-str-x10-sp (local-slot-offset 11)) 0 4)
      (append-native-bytes-loop result (emit-aarch64-ldr-x10-sp (+ stack-arg-base-offset 32)) 0 4)
      (append-native-bytes-loop result (emit-aarch64-str-x10-sp (local-slot-offset 12)) 0 4)
      (append-native-bytes-loop result (emit-aarch64-ldr-x10-sp (+ stack-arg-base-offset 40)) 0 4)
      (append-native-bytes-loop result (emit-aarch64-str-x10-sp (local-slot-offset 13)) 0 4)
      (append-native-bytes-loop result (emit-aarch64-ldr-x10-sp (+ stack-arg-base-offset 48)) 0 4)
      (append-native-bytes-loop result (emit-aarch64-str-x10-sp (local-slot-offset 14)) 0 4)
      (append-native-bytes-loop result (emit-aarch64-ldr-x10-sp (+ stack-arg-base-offset 56)) 0 4)
      (append-native-bytes-loop result (emit-aarch64-str-x10-sp (local-slot-offset 15)) 0 4)
      (append-native-bytes-loop result (emit-aarch64-ldr-x10-sp (+ stack-arg-base-offset 64)) 0 4)
      (append-native-bytes-loop result (emit-aarch64-str-x10-sp (local-slot-offset 16)) 0 4)
      (append-native-bytes-loop result (emit-aarch64-ldr-x10-sp (+ stack-arg-base-offset 72)) 0 4)
      (append-native-bytes-loop result (emit-aarch64-str-x10-sp (local-slot-offset 17)) 0 4)
      (append-native-bytes-loop result (emit-aarch64-ldr-x10-sp (+ stack-arg-base-offset 80)) 0 4)
      (append-native-bytes-loop result (emit-aarch64-str-x10-sp (local-slot-offset 18)) 0 4)
      (append-native-bytes-loop result (emit-aarch64-ldr-x10-sp (+ stack-arg-base-offset 88)) 0 4)
      (append-native-bytes-loop result (emit-aarch64-str-x10-sp (local-slot-offset 19)) 0 4)
      (append-native-bytes-loop result (emit-aarch64-ldr-x10-sp (+ stack-arg-base-offset 96)) 0 4)
      (append-native-bytes-loop result (emit-aarch64-str-x10-sp (local-slot-offset 20)) 0 4)
      (append-native-bytes-loop result (emit-aarch64-ldr-x10-sp (+ stack-arg-base-offset 104)) 0 4)
      (append-native-bytes-loop result (emit-aarch64-str-x10-sp (local-slot-offset 21)) 0 4)
      (append-native-bytes-loop result (emit-aarch64-ldr-x10-sp (+ stack-arg-base-offset 112)) 0 4)
      (append-native-bytes-loop result (emit-aarch64-str-x10-sp (local-slot-offset 22)) 0 4)
      (append-native-bytes-loop result (emit-aarch64-ldr-x10-sp (+ stack-arg-base-offset 120)) 0 4)
       (append-native-bytes-loop result (emit-aarch64-str-x10-sp (local-slot-offset 23)) 0 4))
    (spill-native-function-params-aarch64-twenty-to-twenty-three param-count result stack-arg-base-offset)))

(defn spill-native-function-params-aarch64-twenty-to-twenty-five [param-count result stack-arg-base-offset]
  (if (= param-count 25)
    (do
      (append-native-bytes-loop result (emit-aarch64-str-x0-sp (local-slot-offset 0)) 0 4)
      (append-native-bytes-loop result (emit-aarch64-str-x1-sp (local-slot-offset 1)) 0 4)
      (append-native-bytes-loop result (emit-aarch64-str-x2-sp (local-slot-offset 2)) 0 4)
      (append-native-bytes-loop result (emit-aarch64-str-x3-sp (local-slot-offset 3)) 0 4)
      (append-native-bytes-loop result (emit-aarch64-str-x4-sp (local-slot-offset 4)) 0 4)
      (append-native-bytes-loop result (emit-aarch64-str-x5-sp (local-slot-offset 5)) 0 4)
      (append-native-bytes-loop result (emit-aarch64-str-x6-sp (local-slot-offset 6)) 0 4)
      (append-native-bytes-loop result (emit-aarch64-str-x7-sp (local-slot-offset 7)) 0 4)
      (append-native-bytes-loop result (emit-aarch64-ldr-x10-sp (+ stack-arg-base-offset 0)) 0 4)
      (append-native-bytes-loop result (emit-aarch64-str-x10-sp (local-slot-offset 8)) 0 4)
      (append-native-bytes-loop result (emit-aarch64-ldr-x10-sp (+ stack-arg-base-offset 8)) 0 4)
      (append-native-bytes-loop result (emit-aarch64-str-x10-sp (local-slot-offset 9)) 0 4)
      (append-native-bytes-loop result (emit-aarch64-ldr-x10-sp (+ stack-arg-base-offset 16)) 0 4)
      (append-native-bytes-loop result (emit-aarch64-str-x10-sp (local-slot-offset 10)) 0 4)
      (append-native-bytes-loop result (emit-aarch64-ldr-x10-sp (+ stack-arg-base-offset 24)) 0 4)
      (append-native-bytes-loop result (emit-aarch64-str-x10-sp (local-slot-offset 11)) 0 4)
      (append-native-bytes-loop result (emit-aarch64-ldr-x10-sp (+ stack-arg-base-offset 32)) 0 4)
      (append-native-bytes-loop result (emit-aarch64-str-x10-sp (local-slot-offset 12)) 0 4)
      (append-native-bytes-loop result (emit-aarch64-ldr-x10-sp (+ stack-arg-base-offset 40)) 0 4)
      (append-native-bytes-loop result (emit-aarch64-str-x10-sp (local-slot-offset 13)) 0 4)
      (append-native-bytes-loop result (emit-aarch64-ldr-x10-sp (+ stack-arg-base-offset 48)) 0 4)
      (append-native-bytes-loop result (emit-aarch64-str-x10-sp (local-slot-offset 14)) 0 4)
      (append-native-bytes-loop result (emit-aarch64-ldr-x10-sp (+ stack-arg-base-offset 56)) 0 4)
      (append-native-bytes-loop result (emit-aarch64-str-x10-sp (local-slot-offset 15)) 0 4)
      (append-native-bytes-loop result (emit-aarch64-ldr-x10-sp (+ stack-arg-base-offset 64)) 0 4)
      (append-native-bytes-loop result (emit-aarch64-str-x10-sp (local-slot-offset 16)) 0 4)
      (append-native-bytes-loop result (emit-aarch64-ldr-x10-sp (+ stack-arg-base-offset 72)) 0 4)
      (append-native-bytes-loop result (emit-aarch64-str-x10-sp (local-slot-offset 17)) 0 4)
      (append-native-bytes-loop result (emit-aarch64-ldr-x10-sp (+ stack-arg-base-offset 80)) 0 4)
      (append-native-bytes-loop result (emit-aarch64-str-x10-sp (local-slot-offset 18)) 0 4)
      (append-native-bytes-loop result (emit-aarch64-ldr-x10-sp (+ stack-arg-base-offset 88)) 0 4)
      (append-native-bytes-loop result (emit-aarch64-str-x10-sp (local-slot-offset 19)) 0 4)
      (append-native-bytes-loop result (emit-aarch64-ldr-x10-sp (+ stack-arg-base-offset 96)) 0 4)
      (append-native-bytes-loop result (emit-aarch64-str-x10-sp (local-slot-offset 20)) 0 4)
      (append-native-bytes-loop result (emit-aarch64-ldr-x10-sp (+ stack-arg-base-offset 104)) 0 4)
      (append-native-bytes-loop result (emit-aarch64-str-x10-sp (local-slot-offset 21)) 0 4)
      (append-native-bytes-loop result (emit-aarch64-ldr-x10-sp (+ stack-arg-base-offset 112)) 0 4)
      (append-native-bytes-loop result (emit-aarch64-str-x10-sp (local-slot-offset 22)) 0 4)
      (append-native-bytes-loop result (emit-aarch64-ldr-x10-sp (+ stack-arg-base-offset 120)) 0 4)
      (append-native-bytes-loop result (emit-aarch64-str-x10-sp (local-slot-offset 23)) 0 4)
      (append-native-bytes-loop result (emit-aarch64-ldr-x10-sp (+ stack-arg-base-offset 128)) 0 4)
      (append-native-bytes-loop result (emit-aarch64-str-x10-sp (local-slot-offset 24)) 0 4))
    (spill-native-function-params-aarch64-twenty-to-twenty-four param-count result stack-arg-base-offset)))

(defn spill-native-function-params-aarch64-twenty-to-twenty-six [param-count result stack-arg-base-offset]
  (if (= param-count 26)
    (do
      (append-native-bytes-loop result (emit-aarch64-str-x0-sp (local-slot-offset 0)) 0 4)
      (append-native-bytes-loop result (emit-aarch64-str-x1-sp (local-slot-offset 1)) 0 4)
      (append-native-bytes-loop result (emit-aarch64-str-x2-sp (local-slot-offset 2)) 0 4)
      (append-native-bytes-loop result (emit-aarch64-str-x3-sp (local-slot-offset 3)) 0 4)
      (append-native-bytes-loop result (emit-aarch64-str-x4-sp (local-slot-offset 4)) 0 4)
      (append-native-bytes-loop result (emit-aarch64-str-x5-sp (local-slot-offset 5)) 0 4)
      (append-native-bytes-loop result (emit-aarch64-str-x6-sp (local-slot-offset 6)) 0 4)
      (append-native-bytes-loop result (emit-aarch64-str-x7-sp (local-slot-offset 7)) 0 4)
      (append-native-bytes-loop result (emit-aarch64-ldr-x10-sp (+ stack-arg-base-offset 0)) 0 4)
      (append-native-bytes-loop result (emit-aarch64-str-x10-sp (local-slot-offset 8)) 0 4)
      (append-native-bytes-loop result (emit-aarch64-ldr-x10-sp (+ stack-arg-base-offset 8)) 0 4)
      (append-native-bytes-loop result (emit-aarch64-str-x10-sp (local-slot-offset 9)) 0 4)
      (append-native-bytes-loop result (emit-aarch64-ldr-x10-sp (+ stack-arg-base-offset 16)) 0 4)
      (append-native-bytes-loop result (emit-aarch64-str-x10-sp (local-slot-offset 10)) 0 4)
      (append-native-bytes-loop result (emit-aarch64-ldr-x10-sp (+ stack-arg-base-offset 24)) 0 4)
      (append-native-bytes-loop result (emit-aarch64-str-x10-sp (local-slot-offset 11)) 0 4)
      (append-native-bytes-loop result (emit-aarch64-ldr-x10-sp (+ stack-arg-base-offset 32)) 0 4)
      (append-native-bytes-loop result (emit-aarch64-str-x10-sp (local-slot-offset 12)) 0 4)
      (append-native-bytes-loop result (emit-aarch64-ldr-x10-sp (+ stack-arg-base-offset 40)) 0 4)
      (append-native-bytes-loop result (emit-aarch64-str-x10-sp (local-slot-offset 13)) 0 4)
      (append-native-bytes-loop result (emit-aarch64-ldr-x10-sp (+ stack-arg-base-offset 48)) 0 4)
      (append-native-bytes-loop result (emit-aarch64-str-x10-sp (local-slot-offset 14)) 0 4)
      (append-native-bytes-loop result (emit-aarch64-ldr-x10-sp (+ stack-arg-base-offset 56)) 0 4)
      (append-native-bytes-loop result (emit-aarch64-str-x10-sp (local-slot-offset 15)) 0 4)
      (append-native-bytes-loop result (emit-aarch64-ldr-x10-sp (+ stack-arg-base-offset 64)) 0 4)
      (append-native-bytes-loop result (emit-aarch64-str-x10-sp (local-slot-offset 16)) 0 4)
      (append-native-bytes-loop result (emit-aarch64-ldr-x10-sp (+ stack-arg-base-offset 72)) 0 4)
      (append-native-bytes-loop result (emit-aarch64-str-x10-sp (local-slot-offset 17)) 0 4)
      (append-native-bytes-loop result (emit-aarch64-ldr-x10-sp (+ stack-arg-base-offset 80)) 0 4)
      (append-native-bytes-loop result (emit-aarch64-str-x10-sp (local-slot-offset 18)) 0 4)
      (append-native-bytes-loop result (emit-aarch64-ldr-x10-sp (+ stack-arg-base-offset 88)) 0 4)
      (append-native-bytes-loop result (emit-aarch64-str-x10-sp (local-slot-offset 19)) 0 4)
      (append-native-bytes-loop result (emit-aarch64-ldr-x10-sp (+ stack-arg-base-offset 96)) 0 4)
      (append-native-bytes-loop result (emit-aarch64-str-x10-sp (local-slot-offset 20)) 0 4)
      (append-native-bytes-loop result (emit-aarch64-ldr-x10-sp (+ stack-arg-base-offset 104)) 0 4)
      (append-native-bytes-loop result (emit-aarch64-str-x10-sp (local-slot-offset 21)) 0 4)
      (append-native-bytes-loop result (emit-aarch64-ldr-x10-sp (+ stack-arg-base-offset 112)) 0 4)
      (append-native-bytes-loop result (emit-aarch64-str-x10-sp (local-slot-offset 22)) 0 4)
      (append-native-bytes-loop result (emit-aarch64-ldr-x10-sp (+ stack-arg-base-offset 120)) 0 4)
      (append-native-bytes-loop result (emit-aarch64-str-x10-sp (local-slot-offset 23)) 0 4)
      (append-native-bytes-loop result (emit-aarch64-ldr-x10-sp (+ stack-arg-base-offset 128)) 0 4)
      (append-native-bytes-loop result (emit-aarch64-str-x10-sp (local-slot-offset 24)) 0 4)
       (append-native-bytes-loop result (emit-aarch64-ldr-x10-sp (+ stack-arg-base-offset 136)) 0 4)
       (append-native-bytes-loop result (emit-aarch64-str-x10-sp (local-slot-offset 25)) 0 4))
    (spill-native-function-params-aarch64-twenty-to-twenty-five param-count result stack-arg-base-offset)))

(defn spill-native-function-params-aarch64-twenty-to-twenty-seven [param-count result stack-arg-base-offset]
  (if (= param-count 27)
    (do
      (append-native-bytes-loop result (emit-aarch64-str-x0-sp (local-slot-offset 0)) 0 4)
      (append-native-bytes-loop result (emit-aarch64-str-x1-sp (local-slot-offset 1)) 0 4)
      (append-native-bytes-loop result (emit-aarch64-str-x2-sp (local-slot-offset 2)) 0 4)
      (append-native-bytes-loop result (emit-aarch64-str-x3-sp (local-slot-offset 3)) 0 4)
      (append-native-bytes-loop result (emit-aarch64-str-x4-sp (local-slot-offset 4)) 0 4)
      (append-native-bytes-loop result (emit-aarch64-str-x5-sp (local-slot-offset 5)) 0 4)
      (append-native-bytes-loop result (emit-aarch64-str-x6-sp (local-slot-offset 6)) 0 4)
      (append-native-bytes-loop result (emit-aarch64-str-x7-sp (local-slot-offset 7)) 0 4)
      (append-native-bytes-loop result (emit-aarch64-ldr-x10-sp (+ stack-arg-base-offset 0)) 0 4)
      (append-native-bytes-loop result (emit-aarch64-str-x10-sp (local-slot-offset 8)) 0 4)
      (append-native-bytes-loop result (emit-aarch64-ldr-x10-sp (+ stack-arg-base-offset 8)) 0 4)
      (append-native-bytes-loop result (emit-aarch64-str-x10-sp (local-slot-offset 9)) 0 4)
      (append-native-bytes-loop result (emit-aarch64-ldr-x10-sp (+ stack-arg-base-offset 16)) 0 4)
      (append-native-bytes-loop result (emit-aarch64-str-x10-sp (local-slot-offset 10)) 0 4)
      (append-native-bytes-loop result (emit-aarch64-ldr-x10-sp (+ stack-arg-base-offset 24)) 0 4)
      (append-native-bytes-loop result (emit-aarch64-str-x10-sp (local-slot-offset 11)) 0 4)
      (append-native-bytes-loop result (emit-aarch64-ldr-x10-sp (+ stack-arg-base-offset 32)) 0 4)
      (append-native-bytes-loop result (emit-aarch64-str-x10-sp (local-slot-offset 12)) 0 4)
      (append-native-bytes-loop result (emit-aarch64-ldr-x10-sp (+ stack-arg-base-offset 40)) 0 4)
      (append-native-bytes-loop result (emit-aarch64-str-x10-sp (local-slot-offset 13)) 0 4)
      (append-native-bytes-loop result (emit-aarch64-ldr-x10-sp (+ stack-arg-base-offset 48)) 0 4)
      (append-native-bytes-loop result (emit-aarch64-str-x10-sp (local-slot-offset 14)) 0 4)
      (append-native-bytes-loop result (emit-aarch64-ldr-x10-sp (+ stack-arg-base-offset 56)) 0 4)
      (append-native-bytes-loop result (emit-aarch64-str-x10-sp (local-slot-offset 15)) 0 4)
      (append-native-bytes-loop result (emit-aarch64-ldr-x10-sp (+ stack-arg-base-offset 64)) 0 4)
      (append-native-bytes-loop result (emit-aarch64-str-x10-sp (local-slot-offset 16)) 0 4)
      (append-native-bytes-loop result (emit-aarch64-ldr-x10-sp (+ stack-arg-base-offset 72)) 0 4)
      (append-native-bytes-loop result (emit-aarch64-str-x10-sp (local-slot-offset 17)) 0 4)
      (append-native-bytes-loop result (emit-aarch64-ldr-x10-sp (+ stack-arg-base-offset 80)) 0 4)
      (append-native-bytes-loop result (emit-aarch64-str-x10-sp (local-slot-offset 18)) 0 4)
      (append-native-bytes-loop result (emit-aarch64-ldr-x10-sp (+ stack-arg-base-offset 88)) 0 4)
      (append-native-bytes-loop result (emit-aarch64-str-x10-sp (local-slot-offset 19)) 0 4)
      (append-native-bytes-loop result (emit-aarch64-ldr-x10-sp (+ stack-arg-base-offset 96)) 0 4)
      (append-native-bytes-loop result (emit-aarch64-str-x10-sp (local-slot-offset 20)) 0 4)
      (append-native-bytes-loop result (emit-aarch64-ldr-x10-sp (+ stack-arg-base-offset 104)) 0 4)
      (append-native-bytes-loop result (emit-aarch64-str-x10-sp (local-slot-offset 21)) 0 4)
      (append-native-bytes-loop result (emit-aarch64-ldr-x10-sp (+ stack-arg-base-offset 112)) 0 4)
      (append-native-bytes-loop result (emit-aarch64-str-x10-sp (local-slot-offset 22)) 0 4)
      (append-native-bytes-loop result (emit-aarch64-ldr-x10-sp (+ stack-arg-base-offset 120)) 0 4)
      (append-native-bytes-loop result (emit-aarch64-str-x10-sp (local-slot-offset 23)) 0 4)
      (append-native-bytes-loop result (emit-aarch64-ldr-x10-sp (+ stack-arg-base-offset 128)) 0 4)
      (append-native-bytes-loop result (emit-aarch64-str-x10-sp (local-slot-offset 24)) 0 4)
      (append-native-bytes-loop result (emit-aarch64-ldr-x10-sp (+ stack-arg-base-offset 136)) 0 4)
      (append-native-bytes-loop result (emit-aarch64-str-x10-sp (local-slot-offset 25)) 0 4)
       (append-native-bytes-loop result (emit-aarch64-ldr-x10-sp (+ stack-arg-base-offset 144)) 0 4)
       (append-native-bytes-loop result (emit-aarch64-str-x10-sp (local-slot-offset 26)) 0 4))
    (spill-native-function-params-aarch64-twenty-to-twenty-six param-count result stack-arg-base-offset)))

(defn spill-native-function-params-aarch64-twenty-to-twenty-eight [param-count result stack-arg-base-offset]
  (if (= param-count 28)
    (do
      (append-native-bytes-loop result (emit-aarch64-str-x0-sp (local-slot-offset 0)) 0 4)
      (append-native-bytes-loop result (emit-aarch64-str-x1-sp (local-slot-offset 1)) 0 4)
      (append-native-bytes-loop result (emit-aarch64-str-x2-sp (local-slot-offset 2)) 0 4)
      (append-native-bytes-loop result (emit-aarch64-str-x3-sp (local-slot-offset 3)) 0 4)
      (append-native-bytes-loop result (emit-aarch64-str-x4-sp (local-slot-offset 4)) 0 4)
      (append-native-bytes-loop result (emit-aarch64-str-x5-sp (local-slot-offset 5)) 0 4)
      (append-native-bytes-loop result (emit-aarch64-str-x6-sp (local-slot-offset 6)) 0 4)
      (append-native-bytes-loop result (emit-aarch64-str-x7-sp (local-slot-offset 7)) 0 4)
      (append-native-bytes-loop result (emit-aarch64-ldr-x10-sp (+ stack-arg-base-offset 0)) 0 4)
      (append-native-bytes-loop result (emit-aarch64-str-x10-sp (local-slot-offset 8)) 0 4)
      (append-native-bytes-loop result (emit-aarch64-ldr-x10-sp (+ stack-arg-base-offset 8)) 0 4)
      (append-native-bytes-loop result (emit-aarch64-str-x10-sp (local-slot-offset 9)) 0 4)
      (append-native-bytes-loop result (emit-aarch64-ldr-x10-sp (+ stack-arg-base-offset 16)) 0 4)
      (append-native-bytes-loop result (emit-aarch64-str-x10-sp (local-slot-offset 10)) 0 4)
      (append-native-bytes-loop result (emit-aarch64-ldr-x10-sp (+ stack-arg-base-offset 24)) 0 4)
      (append-native-bytes-loop result (emit-aarch64-str-x10-sp (local-slot-offset 11)) 0 4)
      (append-native-bytes-loop result (emit-aarch64-ldr-x10-sp (+ stack-arg-base-offset 32)) 0 4)
      (append-native-bytes-loop result (emit-aarch64-str-x10-sp (local-slot-offset 12)) 0 4)
      (append-native-bytes-loop result (emit-aarch64-ldr-x10-sp (+ stack-arg-base-offset 40)) 0 4)
      (append-native-bytes-loop result (emit-aarch64-str-x10-sp (local-slot-offset 13)) 0 4)
      (append-native-bytes-loop result (emit-aarch64-ldr-x10-sp (+ stack-arg-base-offset 48)) 0 4)
      (append-native-bytes-loop result (emit-aarch64-str-x10-sp (local-slot-offset 14)) 0 4)
      (append-native-bytes-loop result (emit-aarch64-ldr-x10-sp (+ stack-arg-base-offset 56)) 0 4)
      (append-native-bytes-loop result (emit-aarch64-str-x10-sp (local-slot-offset 15)) 0 4)
      (append-native-bytes-loop result (emit-aarch64-ldr-x10-sp (+ stack-arg-base-offset 64)) 0 4)
      (append-native-bytes-loop result (emit-aarch64-str-x10-sp (local-slot-offset 16)) 0 4)
      (append-native-bytes-loop result (emit-aarch64-ldr-x10-sp (+ stack-arg-base-offset 72)) 0 4)
      (append-native-bytes-loop result (emit-aarch64-str-x10-sp (local-slot-offset 17)) 0 4)
      (append-native-bytes-loop result (emit-aarch64-ldr-x10-sp (+ stack-arg-base-offset 80)) 0 4)
      (append-native-bytes-loop result (emit-aarch64-str-x10-sp (local-slot-offset 18)) 0 4)
      (append-native-bytes-loop result (emit-aarch64-ldr-x10-sp (+ stack-arg-base-offset 88)) 0 4)
      (append-native-bytes-loop result (emit-aarch64-str-x10-sp (local-slot-offset 19)) 0 4)
      (append-native-bytes-loop result (emit-aarch64-ldr-x10-sp (+ stack-arg-base-offset 96)) 0 4)
      (append-native-bytes-loop result (emit-aarch64-str-x10-sp (local-slot-offset 20)) 0 4)
      (append-native-bytes-loop result (emit-aarch64-ldr-x10-sp (+ stack-arg-base-offset 104)) 0 4)
      (append-native-bytes-loop result (emit-aarch64-str-x10-sp (local-slot-offset 21)) 0 4)
      (append-native-bytes-loop result (emit-aarch64-ldr-x10-sp (+ stack-arg-base-offset 112)) 0 4)
      (append-native-bytes-loop result (emit-aarch64-str-x10-sp (local-slot-offset 22)) 0 4)
      (append-native-bytes-loop result (emit-aarch64-ldr-x10-sp (+ stack-arg-base-offset 120)) 0 4)
      (append-native-bytes-loop result (emit-aarch64-str-x10-sp (local-slot-offset 23)) 0 4)
      (append-native-bytes-loop result (emit-aarch64-ldr-x10-sp (+ stack-arg-base-offset 128)) 0 4)
      (append-native-bytes-loop result (emit-aarch64-str-x10-sp (local-slot-offset 24)) 0 4)
      (append-native-bytes-loop result (emit-aarch64-ldr-x10-sp (+ stack-arg-base-offset 136)) 0 4)
      (append-native-bytes-loop result (emit-aarch64-str-x10-sp (local-slot-offset 25)) 0 4)
      (append-native-bytes-loop result (emit-aarch64-ldr-x10-sp (+ stack-arg-base-offset 144)) 0 4)
      (append-native-bytes-loop result (emit-aarch64-str-x10-sp (local-slot-offset 26)) 0 4)
      (append-native-bytes-loop result (emit-aarch64-ldr-x10-sp (+ stack-arg-base-offset 152)) 0 4)
      (append-native-bytes-loop result (emit-aarch64-str-x10-sp (local-slot-offset 27)) 0 4))
    (spill-native-function-params-aarch64-twenty-to-twenty-seven param-count result stack-arg-base-offset)))

(defn generate-native-function-aarch64-bundle [func-meta result function-starts function-metas function-start]
  (let [param-count (native-function-param-count func-meta)
    local-count (native-function-local-count func-meta)
    ir-func (native-function-ir func-meta)
    frame-base-slot-count (native-frame-base-slot-count ir-func (+ param-count local-count))
    stack-bytes (native-local-stack-bytes-with-window ir-func (+ param-count local-count) function-metas)
    has-call (native-has-call ir-func)
    stack-arg-base-offset (+ stack-bytes (if (= has-call 1) 16 0))
    after-call-save (if (= has-call 1) (+ function-start 4) function-start)
    after-stack-offset (if (> stack-bytes 0) (+ after-call-save 4) after-call-save)
    param-spill-bytes (if (> param-count 8)
                      (+ 40 (* (- param-count 9) 8))
                      (if (= param-count 8)
                        32
                        (if (= param-count 7)
                          28
                          (if (= param-count 6)
                            24
                            (if (= param-count 5)
                              20
                              (if (= param-count 4)
                                16
                                (if (= param-count 3)
                                  12
                                  (if (= param-count 2)
                                    8
                                    (if (= param-count 1) 4 0)))))))))
    body-offset (+ after-stack-offset param-spill-bytes)
    n (vector-length ir-func)]
    (do
      (if (= has-call 1)
        (append-native-bytes-loop result (emit-aarch64-save-fp-lr) 0 4)
        0)
      (if (> stack-bytes 0)
        (append-native-bytes-loop result (emit-aarch64-sub-sp stack-bytes) 0 4)
        0)
      (if (>= param-count 20)
        (spill-native-function-params-aarch64-twenty-to-twenty-eight param-count result stack-arg-base-offset)
        (if (= param-count 19)
        (do
          (append-native-bytes-loop result (emit-aarch64-str-x0-sp (local-slot-offset 0)) 0 4)
          (append-native-bytes-loop result (emit-aarch64-str-x1-sp (local-slot-offset 1)) 0 4)
          (append-native-bytes-loop result (emit-aarch64-str-x2-sp (local-slot-offset 2)) 0 4)
          (append-native-bytes-loop result (emit-aarch64-str-x3-sp (local-slot-offset 3)) 0 4)
          (append-native-bytes-loop result (emit-aarch64-str-x4-sp (local-slot-offset 4)) 0 4)
          (append-native-bytes-loop result (emit-aarch64-str-x5-sp (local-slot-offset 5)) 0 4)
          (append-native-bytes-loop result (emit-aarch64-str-x6-sp (local-slot-offset 6)) 0 4)
          (append-native-bytes-loop result (emit-aarch64-str-x7-sp (local-slot-offset 7)) 0 4)
          (append-native-bytes-loop result (emit-aarch64-ldr-x10-sp stack-arg-base-offset) 0 4)
          (append-native-bytes-loop result (emit-aarch64-str-x10-sp (local-slot-offset 8)) 0 4)
          (append-native-bytes-loop result (emit-aarch64-ldr-x10-sp (+ stack-arg-base-offset 8)) 0 4)
          (append-native-bytes-loop result (emit-aarch64-str-x10-sp (local-slot-offset 9)) 0 4)
          (append-native-bytes-loop result (emit-aarch64-ldr-x10-sp (+ stack-arg-base-offset 16)) 0 4)
          (append-native-bytes-loop result (emit-aarch64-str-x10-sp (local-slot-offset 10)) 0 4)
          (append-native-bytes-loop result (emit-aarch64-ldr-x10-sp (+ stack-arg-base-offset 24)) 0 4)
          (append-native-bytes-loop result (emit-aarch64-str-x10-sp (local-slot-offset 11)) 0 4)
          (append-native-bytes-loop result (emit-aarch64-ldr-x10-sp (+ stack-arg-base-offset 32)) 0 4)
          (append-native-bytes-loop result (emit-aarch64-str-x10-sp (local-slot-offset 12)) 0 4)
          (append-native-bytes-loop result (emit-aarch64-ldr-x10-sp (+ stack-arg-base-offset 40)) 0 4)
          (append-native-bytes-loop result (emit-aarch64-str-x10-sp (local-slot-offset 13)) 0 4)
          (append-native-bytes-loop result (emit-aarch64-ldr-x10-sp (+ stack-arg-base-offset 48)) 0 4)
          (append-native-bytes-loop result (emit-aarch64-str-x10-sp (local-slot-offset 14)) 0 4)
          (append-native-bytes-loop result (emit-aarch64-ldr-x10-sp (+ stack-arg-base-offset 56)) 0 4)
          (append-native-bytes-loop result (emit-aarch64-str-x10-sp (local-slot-offset 15)) 0 4)
          (append-native-bytes-loop result (emit-aarch64-ldr-x10-sp (+ stack-arg-base-offset 64)) 0 4)
          (append-native-bytes-loop result (emit-aarch64-str-x10-sp (local-slot-offset 16)) 0 4)
          (append-native-bytes-loop result (emit-aarch64-ldr-x10-sp (+ stack-arg-base-offset 72)) 0 4)
          (append-native-bytes-loop result (emit-aarch64-str-x10-sp (local-slot-offset 17)) 0 4)
          (append-native-bytes-loop result (emit-aarch64-ldr-x10-sp (+ stack-arg-base-offset 80)) 0 4)
          (append-native-bytes-loop result (emit-aarch64-str-x10-sp (local-slot-offset 18)) 0 4))
        (if (= param-count 18)
        (do
          (append-native-bytes-loop result (emit-aarch64-str-x0-sp (local-slot-offset 0)) 0 4)
          (append-native-bytes-loop result (emit-aarch64-str-x1-sp (local-slot-offset 1)) 0 4)
          (append-native-bytes-loop result (emit-aarch64-str-x2-sp (local-slot-offset 2)) 0 4)
          (append-native-bytes-loop result (emit-aarch64-str-x3-sp (local-slot-offset 3)) 0 4)
          (append-native-bytes-loop result (emit-aarch64-str-x4-sp (local-slot-offset 4)) 0 4)
          (append-native-bytes-loop result (emit-aarch64-str-x5-sp (local-slot-offset 5)) 0 4)
          (append-native-bytes-loop result (emit-aarch64-str-x6-sp (local-slot-offset 6)) 0 4)
          (append-native-bytes-loop result (emit-aarch64-str-x7-sp (local-slot-offset 7)) 0 4)
          (append-native-bytes-loop result (emit-aarch64-ldr-x10-sp stack-arg-base-offset) 0 4)
          (append-native-bytes-loop result (emit-aarch64-str-x10-sp (local-slot-offset 8)) 0 4)
          (append-native-bytes-loop result (emit-aarch64-ldr-x10-sp (+ stack-arg-base-offset 8)) 0 4)
          (append-native-bytes-loop result (emit-aarch64-str-x10-sp (local-slot-offset 9)) 0 4)
          (append-native-bytes-loop result (emit-aarch64-ldr-x10-sp (+ stack-arg-base-offset 16)) 0 4)
          (append-native-bytes-loop result (emit-aarch64-str-x10-sp (local-slot-offset 10)) 0 4)
          (append-native-bytes-loop result (emit-aarch64-ldr-x10-sp (+ stack-arg-base-offset 24)) 0 4)
          (append-native-bytes-loop result (emit-aarch64-str-x10-sp (local-slot-offset 11)) 0 4)
          (append-native-bytes-loop result (emit-aarch64-ldr-x10-sp (+ stack-arg-base-offset 32)) 0 4)
          (append-native-bytes-loop result (emit-aarch64-str-x10-sp (local-slot-offset 12)) 0 4)
          (append-native-bytes-loop result (emit-aarch64-ldr-x10-sp (+ stack-arg-base-offset 40)) 0 4)
          (append-native-bytes-loop result (emit-aarch64-str-x10-sp (local-slot-offset 13)) 0 4)
          (append-native-bytes-loop result (emit-aarch64-ldr-x10-sp (+ stack-arg-base-offset 48)) 0 4)
          (append-native-bytes-loop result (emit-aarch64-str-x10-sp (local-slot-offset 14)) 0 4)
          (append-native-bytes-loop result (emit-aarch64-ldr-x10-sp (+ stack-arg-base-offset 56)) 0 4)
          (append-native-bytes-loop result (emit-aarch64-str-x10-sp (local-slot-offset 15)) 0 4)
          (append-native-bytes-loop result (emit-aarch64-ldr-x10-sp (+ stack-arg-base-offset 64)) 0 4)
          (append-native-bytes-loop result (emit-aarch64-str-x10-sp (local-slot-offset 16)) 0 4)
          (append-native-bytes-loop result (emit-aarch64-ldr-x10-sp (+ stack-arg-base-offset 72)) 0 4)
          (append-native-bytes-loop result (emit-aarch64-str-x10-sp (local-slot-offset 17)) 0 4))
        (if (= param-count 17)
        (do
          (append-native-bytes-loop result (emit-aarch64-str-x0-sp (local-slot-offset 0)) 0 4)
          (append-native-bytes-loop result (emit-aarch64-str-x1-sp (local-slot-offset 1)) 0 4)
          (append-native-bytes-loop result (emit-aarch64-str-x2-sp (local-slot-offset 2)) 0 4)
          (append-native-bytes-loop result (emit-aarch64-str-x3-sp (local-slot-offset 3)) 0 4)
          (append-native-bytes-loop result (emit-aarch64-str-x4-sp (local-slot-offset 4)) 0 4)
          (append-native-bytes-loop result (emit-aarch64-str-x5-sp (local-slot-offset 5)) 0 4)
          (append-native-bytes-loop result (emit-aarch64-str-x6-sp (local-slot-offset 6)) 0 4)
          (append-native-bytes-loop result (emit-aarch64-str-x7-sp (local-slot-offset 7)) 0 4)
          (append-native-bytes-loop result (emit-aarch64-ldr-x10-sp stack-arg-base-offset) 0 4)
          (append-native-bytes-loop result (emit-aarch64-str-x10-sp (local-slot-offset 8)) 0 4)
          (append-native-bytes-loop result (emit-aarch64-ldr-x10-sp (+ stack-arg-base-offset 8)) 0 4)
          (append-native-bytes-loop result (emit-aarch64-str-x10-sp (local-slot-offset 9)) 0 4)
          (append-native-bytes-loop result (emit-aarch64-ldr-x10-sp (+ stack-arg-base-offset 16)) 0 4)
          (append-native-bytes-loop result (emit-aarch64-str-x10-sp (local-slot-offset 10)) 0 4)
          (append-native-bytes-loop result (emit-aarch64-ldr-x10-sp (+ stack-arg-base-offset 24)) 0 4)
          (append-native-bytes-loop result (emit-aarch64-str-x10-sp (local-slot-offset 11)) 0 4)
          (append-native-bytes-loop result (emit-aarch64-ldr-x10-sp (+ stack-arg-base-offset 32)) 0 4)
          (append-native-bytes-loop result (emit-aarch64-str-x10-sp (local-slot-offset 12)) 0 4)
          (append-native-bytes-loop result (emit-aarch64-ldr-x10-sp (+ stack-arg-base-offset 40)) 0 4)
          (append-native-bytes-loop result (emit-aarch64-str-x10-sp (local-slot-offset 13)) 0 4)
          (append-native-bytes-loop result (emit-aarch64-ldr-x10-sp (+ stack-arg-base-offset 48)) 0 4)
          (append-native-bytes-loop result (emit-aarch64-str-x10-sp (local-slot-offset 14)) 0 4)
          (append-native-bytes-loop result (emit-aarch64-ldr-x10-sp (+ stack-arg-base-offset 56)) 0 4)
          (append-native-bytes-loop result (emit-aarch64-str-x10-sp (local-slot-offset 15)) 0 4)
          (append-native-bytes-loop result (emit-aarch64-ldr-x10-sp (+ stack-arg-base-offset 64)) 0 4)
          (append-native-bytes-loop result (emit-aarch64-str-x10-sp (local-slot-offset 16)) 0 4))
        (if (= param-count 16)
        (do
          (append-native-bytes-loop result (emit-aarch64-str-x0-sp (local-slot-offset 0)) 0 4)
          (append-native-bytes-loop result (emit-aarch64-str-x1-sp (local-slot-offset 1)) 0 4)
          (append-native-bytes-loop result (emit-aarch64-str-x2-sp (local-slot-offset 2)) 0 4)
          (append-native-bytes-loop result (emit-aarch64-str-x3-sp (local-slot-offset 3)) 0 4)
          (append-native-bytes-loop result (emit-aarch64-str-x4-sp (local-slot-offset 4)) 0 4)
          (append-native-bytes-loop result (emit-aarch64-str-x5-sp (local-slot-offset 5)) 0 4)
          (append-native-bytes-loop result (emit-aarch64-str-x6-sp (local-slot-offset 6)) 0 4)
          (append-native-bytes-loop result (emit-aarch64-str-x7-sp (local-slot-offset 7)) 0 4)
          (append-native-bytes-loop result (emit-aarch64-ldr-x10-sp stack-arg-base-offset) 0 4)
          (append-native-bytes-loop result (emit-aarch64-str-x10-sp (local-slot-offset 8)) 0 4)
          (append-native-bytes-loop result (emit-aarch64-ldr-x10-sp (+ stack-arg-base-offset 8)) 0 4)
          (append-native-bytes-loop result (emit-aarch64-str-x10-sp (local-slot-offset 9)) 0 4)
          (append-native-bytes-loop result (emit-aarch64-ldr-x10-sp (+ stack-arg-base-offset 16)) 0 4)
          (append-native-bytes-loop result (emit-aarch64-str-x10-sp (local-slot-offset 10)) 0 4)
          (append-native-bytes-loop result (emit-aarch64-ldr-x10-sp (+ stack-arg-base-offset 24)) 0 4)
          (append-native-bytes-loop result (emit-aarch64-str-x10-sp (local-slot-offset 11)) 0 4)
          (append-native-bytes-loop result (emit-aarch64-ldr-x10-sp (+ stack-arg-base-offset 32)) 0 4)
          (append-native-bytes-loop result (emit-aarch64-str-x10-sp (local-slot-offset 12)) 0 4)
          (append-native-bytes-loop result (emit-aarch64-ldr-x10-sp (+ stack-arg-base-offset 40)) 0 4)
          (append-native-bytes-loop result (emit-aarch64-str-x10-sp (local-slot-offset 13)) 0 4)
          (append-native-bytes-loop result (emit-aarch64-ldr-x10-sp (+ stack-arg-base-offset 48)) 0 4)
          (append-native-bytes-loop result (emit-aarch64-str-x10-sp (local-slot-offset 14)) 0 4)
          (append-native-bytes-loop result (emit-aarch64-ldr-x10-sp (+ stack-arg-base-offset 56)) 0 4)
          (append-native-bytes-loop result (emit-aarch64-str-x10-sp (local-slot-offset 15)) 0 4))
        (if (= param-count 15)
        (do
          (append-native-bytes-loop result (emit-aarch64-str-x0-sp (local-slot-offset 0)) 0 4)
          (append-native-bytes-loop result (emit-aarch64-str-x1-sp (local-slot-offset 1)) 0 4)
          (append-native-bytes-loop result (emit-aarch64-str-x2-sp (local-slot-offset 2)) 0 4)
          (append-native-bytes-loop result (emit-aarch64-str-x3-sp (local-slot-offset 3)) 0 4)
          (append-native-bytes-loop result (emit-aarch64-str-x4-sp (local-slot-offset 4)) 0 4)
          (append-native-bytes-loop result (emit-aarch64-str-x5-sp (local-slot-offset 5)) 0 4)
          (append-native-bytes-loop result (emit-aarch64-str-x6-sp (local-slot-offset 6)) 0 4)
          (append-native-bytes-loop result (emit-aarch64-str-x7-sp (local-slot-offset 7)) 0 4)
          (append-native-bytes-loop result (emit-aarch64-ldr-x10-sp stack-arg-base-offset) 0 4)
          (append-native-bytes-loop result (emit-aarch64-str-x10-sp (local-slot-offset 8)) 0 4)
          (append-native-bytes-loop result (emit-aarch64-ldr-x10-sp (+ stack-arg-base-offset 8)) 0 4)
          (append-native-bytes-loop result (emit-aarch64-str-x10-sp (local-slot-offset 9)) 0 4)
          (append-native-bytes-loop result (emit-aarch64-ldr-x10-sp (+ stack-arg-base-offset 16)) 0 4)
          (append-native-bytes-loop result (emit-aarch64-str-x10-sp (local-slot-offset 10)) 0 4)
          (append-native-bytes-loop result (emit-aarch64-ldr-x10-sp (+ stack-arg-base-offset 24)) 0 4)
          (append-native-bytes-loop result (emit-aarch64-str-x10-sp (local-slot-offset 11)) 0 4)
          (append-native-bytes-loop result (emit-aarch64-ldr-x10-sp (+ stack-arg-base-offset 32)) 0 4)
          (append-native-bytes-loop result (emit-aarch64-str-x10-sp (local-slot-offset 12)) 0 4)
          (append-native-bytes-loop result (emit-aarch64-ldr-x10-sp (+ stack-arg-base-offset 40)) 0 4)
          (append-native-bytes-loop result (emit-aarch64-str-x10-sp (local-slot-offset 13)) 0 4)
          (append-native-bytes-loop result (emit-aarch64-ldr-x10-sp (+ stack-arg-base-offset 48)) 0 4)
          (append-native-bytes-loop result (emit-aarch64-str-x10-sp (local-slot-offset 14)) 0 4))
        (if (= param-count 14)
        (do
          (append-native-bytes-loop result (emit-aarch64-str-x0-sp (local-slot-offset 0)) 0 4)
          (append-native-bytes-loop result (emit-aarch64-str-x1-sp (local-slot-offset 1)) 0 4)
          (append-native-bytes-loop result (emit-aarch64-str-x2-sp (local-slot-offset 2)) 0 4)
          (append-native-bytes-loop result (emit-aarch64-str-x3-sp (local-slot-offset 3)) 0 4)
          (append-native-bytes-loop result (emit-aarch64-str-x4-sp (local-slot-offset 4)) 0 4)
          (append-native-bytes-loop result (emit-aarch64-str-x5-sp (local-slot-offset 5)) 0 4)
          (append-native-bytes-loop result (emit-aarch64-str-x6-sp (local-slot-offset 6)) 0 4)
          (append-native-bytes-loop result (emit-aarch64-str-x7-sp (local-slot-offset 7)) 0 4)
          (append-native-bytes-loop result (emit-aarch64-ldr-x10-sp stack-arg-base-offset) 0 4)
          (append-native-bytes-loop result (emit-aarch64-str-x10-sp (local-slot-offset 8)) 0 4)
          (append-native-bytes-loop result (emit-aarch64-ldr-x10-sp (+ stack-arg-base-offset 8)) 0 4)
          (append-native-bytes-loop result (emit-aarch64-str-x10-sp (local-slot-offset 9)) 0 4)
          (append-native-bytes-loop result (emit-aarch64-ldr-x10-sp (+ stack-arg-base-offset 16)) 0 4)
          (append-native-bytes-loop result (emit-aarch64-str-x10-sp (local-slot-offset 10)) 0 4)
          (append-native-bytes-loop result (emit-aarch64-ldr-x10-sp (+ stack-arg-base-offset 24)) 0 4)
          (append-native-bytes-loop result (emit-aarch64-str-x10-sp (local-slot-offset 11)) 0 4)
          (append-native-bytes-loop result (emit-aarch64-ldr-x10-sp (+ stack-arg-base-offset 32)) 0 4)
          (append-native-bytes-loop result (emit-aarch64-str-x10-sp (local-slot-offset 12)) 0 4)
          (append-native-bytes-loop result (emit-aarch64-ldr-x10-sp (+ stack-arg-base-offset 40)) 0 4)
          (append-native-bytes-loop result (emit-aarch64-str-x10-sp (local-slot-offset 13)) 0 4))
        (if (= param-count 13)
        (do
          (append-native-bytes-loop result (emit-aarch64-str-x0-sp (local-slot-offset 0)) 0 4)
          (append-native-bytes-loop result (emit-aarch64-str-x1-sp (local-slot-offset 1)) 0 4)
          (append-native-bytes-loop result (emit-aarch64-str-x2-sp (local-slot-offset 2)) 0 4)
          (append-native-bytes-loop result (emit-aarch64-str-x3-sp (local-slot-offset 3)) 0 4)
          (append-native-bytes-loop result (emit-aarch64-str-x4-sp (local-slot-offset 4)) 0 4)
          (append-native-bytes-loop result (emit-aarch64-str-x5-sp (local-slot-offset 5)) 0 4)
          (append-native-bytes-loop result (emit-aarch64-str-x6-sp (local-slot-offset 6)) 0 4)
          (append-native-bytes-loop result (emit-aarch64-str-x7-sp (local-slot-offset 7)) 0 4)
          (append-native-bytes-loop result (emit-aarch64-ldr-x10-sp stack-arg-base-offset) 0 4)
          (append-native-bytes-loop result (emit-aarch64-str-x10-sp (local-slot-offset 8)) 0 4)
          (append-native-bytes-loop result (emit-aarch64-ldr-x10-sp (+ stack-arg-base-offset 8)) 0 4)
          (append-native-bytes-loop result (emit-aarch64-str-x10-sp (local-slot-offset 9)) 0 4)
          (append-native-bytes-loop result (emit-aarch64-ldr-x10-sp (+ stack-arg-base-offset 16)) 0 4)
          (append-native-bytes-loop result (emit-aarch64-str-x10-sp (local-slot-offset 10)) 0 4)
          (append-native-bytes-loop result (emit-aarch64-ldr-x10-sp (+ stack-arg-base-offset 24)) 0 4)
          (append-native-bytes-loop result (emit-aarch64-str-x10-sp (local-slot-offset 11)) 0 4)
          (append-native-bytes-loop result (emit-aarch64-ldr-x10-sp (+ stack-arg-base-offset 32)) 0 4)
          (append-native-bytes-loop result (emit-aarch64-str-x10-sp (local-slot-offset 12)) 0 4))
        (if (= param-count 12)
        (do
          (append-native-bytes-loop result (emit-aarch64-str-x0-sp (local-slot-offset 0)) 0 4)
          (append-native-bytes-loop result (emit-aarch64-str-x1-sp (local-slot-offset 1)) 0 4)
          (append-native-bytes-loop result (emit-aarch64-str-x2-sp (local-slot-offset 2)) 0 4)
          (append-native-bytes-loop result (emit-aarch64-str-x3-sp (local-slot-offset 3)) 0 4)
          (append-native-bytes-loop result (emit-aarch64-str-x4-sp (local-slot-offset 4)) 0 4)
          (append-native-bytes-loop result (emit-aarch64-str-x5-sp (local-slot-offset 5)) 0 4)
          (append-native-bytes-loop result (emit-aarch64-str-x6-sp (local-slot-offset 6)) 0 4)
          (append-native-bytes-loop result (emit-aarch64-str-x7-sp (local-slot-offset 7)) 0 4)
          (append-native-bytes-loop result (emit-aarch64-ldr-x10-sp stack-arg-base-offset) 0 4)
          (append-native-bytes-loop result (emit-aarch64-str-x10-sp (local-slot-offset 8)) 0 4)
          (append-native-bytes-loop result (emit-aarch64-ldr-x10-sp (+ stack-arg-base-offset 8)) 0 4)
          (append-native-bytes-loop result (emit-aarch64-str-x10-sp (local-slot-offset 9)) 0 4)
          (append-native-bytes-loop result (emit-aarch64-ldr-x10-sp (+ stack-arg-base-offset 16)) 0 4)
          (append-native-bytes-loop result (emit-aarch64-str-x10-sp (local-slot-offset 10)) 0 4)
          (append-native-bytes-loop result (emit-aarch64-ldr-x10-sp (+ stack-arg-base-offset 24)) 0 4)
          (append-native-bytes-loop result (emit-aarch64-str-x10-sp (local-slot-offset 11)) 0 4))
        (if (= param-count 11)
        (do
          (append-native-bytes-loop result (emit-aarch64-str-x0-sp (local-slot-offset 0)) 0 4)
          (append-native-bytes-loop result (emit-aarch64-str-x1-sp (local-slot-offset 1)) 0 4)
          (append-native-bytes-loop result (emit-aarch64-str-x2-sp (local-slot-offset 2)) 0 4)
          (append-native-bytes-loop result (emit-aarch64-str-x3-sp (local-slot-offset 3)) 0 4)
          (append-native-bytes-loop result (emit-aarch64-str-x4-sp (local-slot-offset 4)) 0 4)
          (append-native-bytes-loop result (emit-aarch64-str-x5-sp (local-slot-offset 5)) 0 4)
          (append-native-bytes-loop result (emit-aarch64-str-x6-sp (local-slot-offset 6)) 0 4)
          (append-native-bytes-loop result (emit-aarch64-str-x7-sp (local-slot-offset 7)) 0 4)
          (append-native-bytes-loop result (emit-aarch64-ldr-x10-sp stack-arg-base-offset) 0 4)
          (append-native-bytes-loop result (emit-aarch64-str-x10-sp (local-slot-offset 8)) 0 4)
          (append-native-bytes-loop result (emit-aarch64-ldr-x10-sp (+ stack-arg-base-offset 8)) 0 4)
          (append-native-bytes-loop result (emit-aarch64-str-x10-sp (local-slot-offset 9)) 0 4)
          (append-native-bytes-loop result (emit-aarch64-ldr-x10-sp (+ stack-arg-base-offset 16)) 0 4)
          (append-native-bytes-loop result (emit-aarch64-str-x10-sp (local-slot-offset 10)) 0 4))
        (if (= param-count 10)
        (do
          (append-native-bytes-loop result (emit-aarch64-str-x0-sp (local-slot-offset 0)) 0 4)
          (append-native-bytes-loop result (emit-aarch64-str-x1-sp (local-slot-offset 1)) 0 4)
          (append-native-bytes-loop result (emit-aarch64-str-x2-sp (local-slot-offset 2)) 0 4)
          (append-native-bytes-loop result (emit-aarch64-str-x3-sp (local-slot-offset 3)) 0 4)
          (append-native-bytes-loop result (emit-aarch64-str-x4-sp (local-slot-offset 4)) 0 4)
          (append-native-bytes-loop result (emit-aarch64-str-x5-sp (local-slot-offset 5)) 0 4)
          (append-native-bytes-loop result (emit-aarch64-str-x6-sp (local-slot-offset 6)) 0 4)
          (append-native-bytes-loop result (emit-aarch64-str-x7-sp (local-slot-offset 7)) 0 4)
          (append-native-bytes-loop result (emit-aarch64-ldr-x10-sp stack-arg-base-offset) 0 4)
          (append-native-bytes-loop result (emit-aarch64-str-x10-sp (local-slot-offset 8)) 0 4)
          (append-native-bytes-loop result (emit-aarch64-ldr-x10-sp (+ stack-arg-base-offset 8)) 0 4)
          (append-native-bytes-loop result (emit-aarch64-str-x10-sp (local-slot-offset 9)) 0 4))
        (if (= param-count 9)
        (do
          (append-native-bytes-loop result (emit-aarch64-str-x0-sp (local-slot-offset 0)) 0 4)
          (append-native-bytes-loop result (emit-aarch64-str-x1-sp (local-slot-offset 1)) 0 4)
          (append-native-bytes-loop result (emit-aarch64-str-x2-sp (local-slot-offset 2)) 0 4)
          (append-native-bytes-loop result (emit-aarch64-str-x3-sp (local-slot-offset 3)) 0 4)
          (append-native-bytes-loop result (emit-aarch64-str-x4-sp (local-slot-offset 4)) 0 4)
          (append-native-bytes-loop result (emit-aarch64-str-x5-sp (local-slot-offset 5)) 0 4)
          (append-native-bytes-loop result (emit-aarch64-str-x6-sp (local-slot-offset 6)) 0 4)
          (append-native-bytes-loop result (emit-aarch64-str-x7-sp (local-slot-offset 7)) 0 4)
          (append-native-bytes-loop result (emit-aarch64-ldr-x10-sp stack-arg-base-offset) 0 4)
          (append-native-bytes-loop result (emit-aarch64-str-x10-sp (local-slot-offset 8)) 0 4))
        (if (= param-count 8)
        (do
          (append-native-bytes-loop result (emit-aarch64-str-x0-sp (local-slot-offset 0)) 0 4)
          (append-native-bytes-loop result (emit-aarch64-str-x1-sp (local-slot-offset 1)) 0 4)
          (append-native-bytes-loop result (emit-aarch64-str-x2-sp (local-slot-offset 2)) 0 4)
          (append-native-bytes-loop result (emit-aarch64-str-x3-sp (local-slot-offset 3)) 0 4)
          (append-native-bytes-loop result (emit-aarch64-str-x4-sp (local-slot-offset 4)) 0 4)
          (append-native-bytes-loop result (emit-aarch64-str-x5-sp (local-slot-offset 5)) 0 4)
          (append-native-bytes-loop result (emit-aarch64-str-x6-sp (local-slot-offset 6)) 0 4)
          (append-native-bytes-loop result (emit-aarch64-str-x7-sp (local-slot-offset 7)) 0 4))
        (if (= param-count 7)
          (do
            (append-native-bytes-loop result (emit-aarch64-str-x0-sp (local-slot-offset 0)) 0 4)
            (append-native-bytes-loop result (emit-aarch64-str-x1-sp (local-slot-offset 1)) 0 4)
            (append-native-bytes-loop result (emit-aarch64-str-x2-sp (local-slot-offset 2)) 0 4)
            (append-native-bytes-loop result (emit-aarch64-str-x3-sp (local-slot-offset 3)) 0 4)
            (append-native-bytes-loop result (emit-aarch64-str-x4-sp (local-slot-offset 4)) 0 4)
            (append-native-bytes-loop result (emit-aarch64-str-x5-sp (local-slot-offset 5)) 0 4)
            (append-native-bytes-loop result (emit-aarch64-str-x6-sp (local-slot-offset 6)) 0 4))
          (if (= param-count 6)
            (do
              (append-native-bytes-loop result (emit-aarch64-str-x0-sp (local-slot-offset 0)) 0 4)
              (append-native-bytes-loop result (emit-aarch64-str-x1-sp (local-slot-offset 1)) 0 4)
              (append-native-bytes-loop result (emit-aarch64-str-x2-sp (local-slot-offset 2)) 0 4)
              (append-native-bytes-loop result (emit-aarch64-str-x3-sp (local-slot-offset 3)) 0 4)
              (append-native-bytes-loop result (emit-aarch64-str-x4-sp (local-slot-offset 4)) 0 4)
              (append-native-bytes-loop result (emit-aarch64-str-x5-sp (local-slot-offset 5)) 0 4))
            (if (= param-count 5)
              (do
                (append-native-bytes-loop result (emit-aarch64-str-x0-sp (local-slot-offset 0)) 0 4)
                (append-native-bytes-loop result (emit-aarch64-str-x1-sp (local-slot-offset 1)) 0 4)
                (append-native-bytes-loop result (emit-aarch64-str-x2-sp (local-slot-offset 2)) 0 4)
                (append-native-bytes-loop result (emit-aarch64-str-x3-sp (local-slot-offset 3)) 0 4)
                (append-native-bytes-loop result (emit-aarch64-str-x4-sp (local-slot-offset 4)) 0 4))
              (if (= param-count 4)
                (do
                  (append-native-bytes-loop result (emit-aarch64-str-x0-sp (local-slot-offset 0)) 0 4)
                  (append-native-bytes-loop result (emit-aarch64-str-x1-sp (local-slot-offset 1)) 0 4)
                  (append-native-bytes-loop result (emit-aarch64-str-x2-sp (local-slot-offset 2)) 0 4)
                  (append-native-bytes-loop result (emit-aarch64-str-x3-sp (local-slot-offset 3)) 0 4))
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
                      0))))))))))))))))))))
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
