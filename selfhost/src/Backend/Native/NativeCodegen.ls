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

(defn vector-push-single-rooted [base value]
  (do
    (root_push value)
    (let [base-slot (root_push base)
      result (vector-push base value)]
      (do
        (root_set base-slot result)
        (root_pop)
        (root_pop)
        result))))

(defn vector-push-pair-rooted [base first second]
  (do
    (root_push first)
    (root_push second)
    (let [base-slot (root_push base)
      with-first (vector-push base first)]
      (do
        (root_set base-slot with-first)
        (let [result (vector-push with-first second)]
          (do
            (root_pop)
            (root_pop)
            (root_pop)
            result))))))

(defn vector-push-triple-rooted [base first second third]
  (do
    (root_push first)
    (root_push second)
    (root_push third)
    (let [base-slot (root_push base)
      with-first (vector-push base first)]
      (do
        (root_set base-slot with-first)
        (let [with-second (vector-push with-first second)]
          (do
            (root_set base-slot with-second)
            (let [result (vector-push with-second third)]
              (do
                (root_pop)
                (root_pop)
                (root_pop)
                (root_pop)
                result))))))))

(defn vector-push-quad-rooted [base first second third fourth]
  (do
    (root_push first)
    (root_push second)
    (root_push third)
    (root_push fourth)
    (let [base-slot (root_push base)
      with-first (vector-push base first)]
      (do
        (root_set base-slot with-first)
        (let [with-second (vector-push with-first second)]
          (do
            (root_set base-slot with-second)
            (let [with-third (vector-push with-second third)]
              (do
                (root_set base-slot with-third)
                (let [result (vector-push with-third fourth)]
                  (do
                    (root_pop)
                    (root_pop)
                    (root_pop)
                    (root_pop)
                    (root_pop)
                    result))))))))))

(defn vector-push-five-rooted [base first second third fourth fifth]
  (do
    (root_push first)
    (root_push second)
    (root_push third)
    (root_push fourth)
    (root_push fifth)
    (let [base-slot (root_push base)
      with-first (vector-push base first)]
      (do
        (root_set base-slot with-first)
        (let [with-second (vector-push with-first second)]
          (do
            (root_set base-slot with-second)
            (let [with-third (vector-push with-second third)]
              (do
                (root_set base-slot with-third)
                (let [with-fourth (vector-push with-third fourth)]
                  (do
                    (root_set base-slot with-fourth)
                    (let [result (vector-push with-fourth fifth)]
                      (do
                        (root_pop)
                        (root_pop)
                        (root_pop)
                        (root_pop)
                        (root_pop)
                        (root_pop)
                        result))))))))))))

(defn byte-vector-1 [first]
  (vector-push-single-rooted (vector-new 1) first))

(defn byte-vector-2 [first second]
  (vector-push-pair-rooted (vector-new 2) first second))

(defn byte-vector-3 [first second third]
  (vector-push-triple-rooted (vector-new 3) first second third))

(defn byte-vector-4 [first second third fourth]
  (vector-push-quad-rooted (vector-new 4) first second third fourth))

(defn byte-vector-5 [first second third fourth fifth]
  (vector-push-five-rooted (vector-new 5) first second third fourth fifth))

(defn emit-x86-rbp-disp32 [rex opcode modrm offset]
  (let [disp (encode-u32-le (- 4294967296 offset))]
    (concat-byte-vectors-rooted
      (byte-vector-3 rex opcode modrm)
      disp)))

;; x86_64 の MOV imm64 命令を生成
;; REX.W + MOV r64, imm64 (0x48 0xB8+rd imm64)
;; 戻り値: バイト列 Vector
(defn emit-mov-imm64 [reg value]
  (do
    (let [head (byte-vector-2 72 (+ 184 reg))
      low-value (if (< value 0) (+ 4294967296 value) value)
      low (encode-u32-le low-value)
      high (if (< value 0)
             (byte-vector-4 255 255 255 255)
             (encode-u32-le (/ value 4294967296)))]
      (do
        (root_push head)
        (root_push low)
        (root_push high)
        (let [head-low (concat-byte-vectors-rooted head low)]
          (do
            (root_push head-low)
            (let [result (concat-byte-vectors-rooted head-low high)]
              (do
                (root_pop)
                (root_pop)
                (root_pop)
                (root_pop)
                result))))))))

;; x86_64 の RET 命令
(defn emit-ret []
  (byte-vector-1 195)) ;; 0xC3

;; x86_64 の PUSH rbp
(defn emit-push-rbp []
  (byte-vector-1 85)) ;; 0x55

;; x86_64 の POP rbp
(defn emit-pop-rbp []
  (byte-vector-1 93)) ;; 0x5D

;; x86_64 の PUSH rcx
(defn emit-push-rcx []
  (byte-vector-1 81)) ;; 0x51

;; x86_64 の POP rcx
(defn emit-pop-rcx []
  (byte-vector-1 89)) ;; 0x59

;; x86_64 の MOV rbp, rsp
(defn emit-mov-rbp-rsp []
  (byte-vector-3
    72   ;; 0x48 REX.W
    137  ;; 0x89
    229)) ;; 0xE5 (rsp -> rbp)

;; x86_64 の MOV rcx, rax
(defn emit-mov-rcx-rax []
  (byte-vector-3 72 137 193))

;; x86_64 の MOV rax, rcx
(defn emit-mov-rax-rcx []
  (byte-vector-3 72 137 200))

;; x86_64 の MOV rdi, rax
(defn emit-mov-rdi-rax []
  (byte-vector-3 72 137 199))

;; x86_64 の MOV rsi, rax
(defn emit-mov-rsi-rax []
  (byte-vector-3 72 137 198))

;; x86_64 の MOV rdi, rcx
(defn emit-mov-rdi-rcx []
  (byte-vector-3 72 137 207))

;; x86_64 の MOV eax, imm32
(defn emit-mov-eax-imm32 [value]
  (let [imm (encode-u32-le value)]
    (concat-byte-vectors-rooted
      (byte-vector-1 184)
      imm)))

;; x86_64 の ADD eax, ecx
(defn emit-add-eax-ecx []
  (byte-vector-2 1 200))

;; x86_64 の IMUL eax, ecx
(defn emit-imul-eax-ecx []
  (byte-vector-3 15 175 193))

;; x86_64 の IMUL rax, rcx
(defn emit-imul-rax-rcx []
  (byte-vector-4 72 15 175 193))

;; x86_64 の CQO
(defn emit-cqo []
  (byte-vector-2 72 153))

;; x86_64 の IDIV rsi
(defn emit-idiv-rsi []
  (byte-vector-3 72 247 254))

;; x86_64 の i64.div_s (rcx / rax)
(defn emit-i64-div-rax-rcx []
  (concat-byte-vectors
    (concat-byte-vectors
      (concat-byte-vectors
        (emit-mov-rsi-rax)
        (emit-mov-rax-rcx))
      (emit-cqo))
    (emit-idiv-rsi)))

;; x86_64 の MOV rax, rdx
(defn emit-mov-rax-rdx []
  (byte-vector-3 72 137 208))

;; x86_64 の i64.rem_s (rcx % rax)
(defn emit-i64-rem-rax-rcx []
  (concat-byte-vectors
    (emit-i64-div-rax-rcx)
    (emit-mov-rax-rdx)))

;; x86_64 の AND eax, ecx
(defn emit-and-eax-ecx []
  (byte-vector-2 33 200))

;; x86_64 の OR eax, ecx
(defn emit-or-eax-ecx []
  (byte-vector-2 9 200))

;; x86_64 の MOV eax, eax
(defn emit-mov-eax-eax []
  (byte-vector-2 137 192))

;; x86_64 の MOVSXD rax, eax
(defn emit-movsxd-rax-eax []
  (byte-vector-3 72 99 192))

;; 32bit 値を little-endian 4 bytes に分解する
(defn encode-u32-le [value]
  (let [byte0 (% value 256)
    byte1 (% (/ value 256) 256)
    byte2 (% (/ value 65536) 256)
    byte3 (% (/ value 16777216) 256)]
    (byte-vector-4 byte0 byte1 byte2 byte3)))

;; 符号付き 32bit 値を two's complement little-endian 4 bytes に分解する
(defn encode-s32-le [value]
  (if (< value 0)
    (encode-u32-le (+ 4294967296 value))
    (encode-u32-le value)))

;; x86_64 の CALL rel32
(defn emit-call-rel32 [disp]
  (concat-byte-vectors-rooted
    (byte-vector-1 232)
    (encode-s32-le disp)))

;; x86_64 の JMP rel32
(defn emit-jmp-rel32 [disp]
  (let [imm (encode-s32-le disp)]
    (byte-vector-5
      233
      (vector-get imm 0)
      (vector-get imm 1)
      (vector-get imm 2)
      (vector-get imm 3))))

;; x86_64 の TEST eax, eax
(defn emit-test-eax-eax []
  (vector-push (vector-push (vector-new 2) 133) 192))

;; x86_64 の JZ rel32
(defn emit-jz-rel32 [disp]
  (let [imm (encode-s32-le disp)
    bytes (vector-new 6)
    b1 (vector-push bytes 15)
    b2 (vector-push b1 132)
    b3 (vector-push b2 (vector-get imm 0))
    b4 (vector-push b3 (vector-get imm 1))
    b5 (vector-push b4 (vector-get imm 2))
    b6 (vector-push b5 (vector-get imm 3))]
    b6))

;; x86_64 の JNZ rel32
(defn emit-jnz-rel32 [disp]
  (let [imm (encode-s32-le disp)
    bytes (vector-new 6)
    b1 (vector-push bytes 15)
    b2 (vector-push b1 133)
    b3 (vector-push b2 (vector-get imm 0))
    b4 (vector-push b3 (vector-get imm 1))
    b5 (vector-push b4 (vector-get imm 2))
    b6 (vector-push b5 (vector-get imm 3))]
    b6))

;; ローカル変数の stack slot offset (rbp/sp からの byte 数)
(defn local-slot-offset [idx]
  (* (+ idx 1) 8))

;; selfhost IR は slot 0 を scratch として予約するため、x86 引数は slot 1 から退避する
(defn native-param-slot-offset-x86 [param-index]
  (local-slot-offset (+ param-index 1)))


;; 16 byte alignment を満たす stack size に丸める
(defn align-16 [value]
  (let [remainder (% value 16)]
    (if (= remainder 0)
      value
      (+ value (- 16 remainder)))))

(defn concat-byte-vectors-range [result extra start end]
  (if (>= start end)
    result
    (let [width (- end start)]
      (if (= width 1)
        (vector-push result (vector-get extra start))
        (do
          (root_push extra)
          (root_push result)
          (let [mid (+ start (/ width 2))
            left (concat-byte-vectors-range result extra start mid)]
            (do
              (root_push left)
              (let [right (concat-byte-vectors-range left extra mid end)]
                (do
                  (root_pop)
                  (root_pop)
                  (root_pop)
                  right)))))))))

(defn concat-byte-vectors-loop [result extra idx len]
  (concat-byte-vectors-range result extra idx len))

(defn concat-byte-vectors [first second]
  (concat-byte-vectors-loop first second 0 (vector-length second)))

(defn concat-byte-vectors-rooted [first second]
  (do
    (root_push first)
    (root_push second)
    (let [result (concat-byte-vectors-loop first second 0 (vector-length second))]
      (do
        (root_pop)
        (root_pop)
        result))))

(defn concat-three-byte-vectors-rooted [first second third]
  (do
    (root_push first)
    (root_push second)
    (root_push third)
    (let [first-two (concat-byte-vectors-loop first second 0 (vector-length second))]
      (do
        (root_push first-two)
        (let [result (concat-byte-vectors-loop first-two third 0 (vector-length third))]
          (do
            (root_pop)
            (root_pop)
            (root_pop)
            (root_pop)
            result))))))

(defn concat-four-byte-vectors-rooted [first second third fourth]
  (do
    (root_push first)
    (root_push second)
    (root_push third)
    (root_push fourth)
    (let [first-two (concat-byte-vectors-loop first second 0 (vector-length second))]
      (do
        (root_push first-two)
        (let [first-three (concat-byte-vectors-loop first-two third 0 (vector-length third))]
          (do
            (root_push first-three)
            (let [result (concat-byte-vectors-loop first-three fourth 0 (vector-length fourth))]
              (do
                (root_pop)
                (root_pop)
                (root_pop)
                (root_pop)
                (root_pop)
                (root_pop)
                result))))))))

(defn concat-five-byte-vectors-rooted [first second third fourth fifth]
  (do
    (root_push first)
    (root_push second)
    (root_push third)
    (root_push fourth)
    (root_push fifth)
    (let [first-two (concat-byte-vectors-loop first second 0 (vector-length second))]
      (do
        (root_push first-two)
        (let [first-three (concat-byte-vectors-loop first-two third 0 (vector-length third))]
          (do
            (root_push first-three)
            (let [first-four (concat-byte-vectors-loop first-three fourth 0 (vector-length fourth))]
              (do
                (root_push first-four)
                (let [result (concat-byte-vectors-loop first-four fifth 0 (vector-length fifth))]
                  (do
                    (root_pop)
                    (root_pop)
                    (root_pop)
                    (root_pop)
                    (root_pop)
                    (root_pop)
                    (root_pop)
                    (root_pop)
                    result))))))))))

(defn vector-set-at-loop [vec result idx new-val i len]
  (if (>= i len)
    result
    (vector-set-at-loop
      vec
      (vector-push result
        (if (= i idx)
          new-val
          (vector-get vec i)))
      idx
      new-val
      (+ i 1)
      len)))

(defn vector-set-at [vec idx new-val]
  (vector-set-at-loop vec (vector-new (vector-length vec)) idx new-val 0 (vector-length vec)))

(defn map-insert-index [m key value]
  (map-insert m key (+ value 1)))

(defn map-get-index [m key]
  (let [value (map-get m key)]
    (if (= value 0)
      -1
      (- value 1))))

(defn make-control-stack-entry [start-idx opcode]
  (vector-push (vector-push (vector-new 2) start-idx) opcode))

(defn control-stack-entry-start [entry]
  (vector-get entry 0))

(defn control-stack-entry-opcode [entry]
  (vector-get entry 1))

(defn control-stack-push [stack depth entry]
  (if (< depth (vector-length stack))
    (vector-set-at stack depth entry)
    (vector-push stack entry)))

(defn is-if-opcode [opcode]
  (if (= opcode 41)
    1
    (if (= opcode 83)
      1
      0)))

(defn is-loop-opcode [opcode]
  (if (= opcode 82)
    1
    (if (= opcode 85)
      1
      0)))

(defn is-block-opcode [opcode]
  (if (= opcode 42)
    1
    (if (= opcode 84)
      1
      0)))

(defn is-control-start-opcode [opcode]
  (if (= (is-if-opcode opcode) 1)
    1
    (if (= (is-loop-opcode opcode) 1)
      1
      (is-block-opcode opcode))))

(defn is-control-opcode [opcode]
  (if (= (is-control-start-opcode opcode) 1)
    1
    (if (= opcode 43)
      1
      (if (= opcode 79)
        1
        (if (= opcode 80)
          1
          (if (= opcode 81)
            1
            0))))))

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

(defn find-max-local-index-step [ir-func idx len state]
  (if (>= idx len)
    (make-callable-object-state 1 idx state)
    (let [instr (vector-get ir-func idx)
      opcode (vector-get instr 0)
      operand (vector-get instr 1)
      next-state (update-max-local-index opcode operand state)]
      (make-callable-object-state 0 (+ idx 1) next-state))))

(defn find-max-local-index-step-64-loop-bounded [ir-func idx len state remaining]
  (do
    (root_push ir-func)
    (root_push state)
    (let [step-state (find-max-local-index-step ir-func idx len state)
      done (vector-get step-state 0)
      next-idx (vector-get step-state 1)
      next-state (vector-get step-state 2)]
      (do
        (root_push step-state)
        (root_push next-state)
        (let [final
          (if (= done 1)
            step-state
            (if (<= remaining 1)
              step-state
              (find-max-local-index-step-64-loop-bounded ir-func next-idx len next-state (- remaining 1))))]
          (do
            (root_pop)
            (root_pop)
            (root_pop)
            (root_pop)
            final))))))

(defn find-max-local-index-step-64 [ir-func idx len state]
  (find-max-local-index-step-64-loop-bounded ir-func idx len state 64))

(defn find-max-local-index-step-1024-loop-bounded [ir-func idx len state remaining]
  (do
    (root_push ir-func)
    (root_push state)
    (let [step-state (find-max-local-index-step-64 ir-func idx len state)
      done (vector-get step-state 0)
      next-idx (vector-get step-state 1)
      next-state (vector-get step-state 2)]
      (do
        (root_push step-state)
        (root_push next-state)
        (let [final
          (if (= done 1)
            step-state
            (if (<= remaining 1)
              step-state
              (find-max-local-index-step-1024-loop-bounded ir-func next-idx len next-state (- remaining 1))))]
          (do
            (root_pop)
            (root_pop)
            (root_pop)
            (root_pop)
            final))))))

(defn find-max-local-index-step-1024 [ir-func idx len state]
  (find-max-local-index-step-1024-loop-bounded ir-func idx len state 16))

(defn continue-find-max-local-index-step-1024 [ir-func len state]
  (if (= (vector-get state 0) 1)
    state
    (do
      (root_push ir-func)
      (root_push state)
      (let [next-state (find-max-local-index-step-1024 ir-func (vector-get state 1) len (vector-get state 2))]
        (do
          (root_push next-state)
          (let [final (continue-find-max-local-index-step-1024 ir-func len next-state)]
            (do
              (root_pop)
              (root_pop)
              (root_pop)
              final)))))))

(defn continue-find-max-local-index-step-64 [ir-func len state]
  (if (= (vector-get state 0) 1)
    state
    (do
      (root_push ir-func)
      (root_push state)
      (let [next-state (find-max-local-index-step-64 ir-func (vector-get state 1) len (vector-get state 2))]
        (do
          (root_push next-state)
          (let [final (continue-find-max-local-index-step-64 ir-func len next-state)]
            (do
              (root_pop)
              (root_pop)
              (root_pop)
              final)))))))

(defn native-slot-count-from-ir [ir-func]
  (let [state (vector-get
                (continue-find-max-local-index-step-1024
                  ir-func
                  (vector-length ir-func)
                  (find-max-local-index-step-64 ir-func 0 (vector-length ir-func) (make-local-scan-state 0 0)))
                2)
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

(defn aarch64-plain-stack-padding-needed [ir-func]
  (if (> (native-slot-count-from-ir ir-func) 0)
    1
    0))

(defn find-call-loop [ir-func idx len]
  (if (>= idx len)
      0
      (let [instr (vector-get ir-func idx)
        opcode (vector-get instr 0)]
      (if (= opcode 40)
        1
        (find-call-loop ir-func (+ idx 1) len)))))

(defn find-call-step [ir-func idx len]
  (if (>= idx len)
    (make-callable-sum-state 1 idx 0)
    (let [instr (vector-get ir-func idx)
      opcode (vector-get instr 0)]
      (if (= opcode 40)
        (make-callable-sum-state 1 (+ idx 1) 1)
        (make-callable-sum-state 0 (+ idx 1) 0)))))

(defn find-call-step-64-loop-bounded [ir-func idx len remaining]
  (do
    (root_push ir-func)
    (let [state (find-call-step ir-func idx len)
      done (vector-get state 0)
      next-idx (vector-get state 1)
      found (vector-get state 2)]
      (do
        (root_push state)
        (let [final
          (if (= done 1)
            state
            (if (= found 1)
              state
              (if (<= remaining 1)
                state
                (find-call-step-64-loop-bounded ir-func next-idx len (- remaining 1)))))]
          (do
            (root_pop)
            (root_pop)
            final))))))

(defn find-call-step-64 [ir-func idx len]
  (find-call-step-64-loop-bounded ir-func idx len 64))

(defn continue-find-call-step-64 [ir-func len state]
  (if (= (vector-get state 0) 1)
    (vector-get state 2)
    (if (= (vector-get state 2) 1)
      1
      (do
        (root_push ir-func)
        (root_push state)
        (let [next-state (find-call-step-64 ir-func (vector-get state 1) len)]
          (do
            (root_push next-state)
            (let [final (continue-find-call-step-64 ir-func len next-state)]
              (do
                (root_pop)
                (root_pop)
                (root_pop)
                final))))))))

(defn native-has-call [ir-func]
  (continue-find-call-step-64
    ir-func
    (vector-length ir-func)
    (find-call-step-64 ir-func 0 (vector-length ir-func))))

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
(defn opcode-pushes-stack [opcode]
  (if (= opcode 1)
    1
    (if (= opcode 3)
      1
      (if (= opcode 10)
        1
        (if (= opcode 60)
          1
          0)))))

(defn is-one-pop-reducer-opcode [opcode]
  (if (= opcode 11)
    1
    (if (= opcode 20)
      1
      (if (= opcode 21)
        1
        (if (= opcode 22)
          1
          (if (= opcode 23)
            1
            (if (= opcode 24)
              1
              (if (= opcode 25)
                1
                (if (= opcode 26)
                  1
                  (if (= opcode 27)
                    1
                    (if (= opcode 28)
                      1
                      (if (= opcode 41)
                        1
                        (if (= opcode 44)
                          1
                          (if (= opcode 71)
                            1
                            (if (= opcode 72)
                              1
                              (if (= opcode 81)
                                1
                                (if (= opcode 83)
                                  1
                                  0)))))))))))))))))

(defn opcode-reduces-stack [opcode]
  (if (= (is-one-pop-reducer-opcode opcode) 1)
    1
    (is-i64-compare-opcode opcode)))

(defn opcode-stack-delta [opcode operand function-metas]
  (if (= opcode 60)
    1
  (if (= opcode 40)
    (- 1 (native-function-param-count (vector-get function-metas operand)))
    (if (= opcode 41)
      -1
      (if (= opcode 79)
        -1
      (if (= opcode 81)
        -1
        (if (= opcode 83)
          -1
    (if (= opcode 75)
      1
      (if (= opcode 76)
        -1
        (if (= opcode 46)
          -2
          (if (= opcode 49)
            -2
            (if (= opcode 62)
              -2
              (if (= opcode 63)
                -1
              (if (= opcode 50)
                -1
                (if (= opcode 53)
                  -1
                  (if (= opcode 55)
                    -1
                    (if (= opcode 58)
                      -1
                      (if (= opcode 69)
                        -2
                        (if (= opcode 70)
                          -1
                          (if (= opcode 77)
                            -3
                            (if (= opcode 78)
                              -3
                              (if (= (opcode-pushes-stack opcode) 1)
                                1
                                (if (= (opcode-reduces-stack opcode) 1)
                                  -1
                                  0)))))))))))))))))))))))

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

(defn native-max-stack-depth-step [ir-func function-metas idx len current-depth max-depth]
  (if (>= idx len)
    (make-callable-object-offset-state 1 idx current-depth max-depth)
    (let [instr (vector-get ir-func idx)
      opcode (vector-get instr 0)
      operand (vector-get instr 1)
      next-depth (apply-stack-delta current-depth (opcode-stack-delta opcode operand function-metas))
      next-max (if (> next-depth max-depth) next-depth max-depth)]
      (make-callable-object-offset-state 0 (+ idx 1) next-depth next-max))))

(defn native-max-stack-depth-step-64-loop-bounded [ir-func function-metas idx len current-depth max-depth remaining]
  (do
    (root_push ir-func)
    (root_push function-metas)
    (let [state (native-max-stack-depth-step ir-func function-metas idx len current-depth max-depth)
      done (vector-get state 0)
      next-idx (vector-get state 1)
      next-depth (vector-get state 2)
      next-max (vector-get state 3)]
      (do
        (root_push state)
        (let [final
          (if (= done 1)
            state
            (if (<= remaining 1)
              state
              (native-max-stack-depth-step-64-loop-bounded ir-func function-metas next-idx len next-depth next-max (- remaining 1))))]
          (do
            (root_pop)
            (root_pop)
            (root_pop)
            final))))))

(defn native-max-stack-depth-step-64 [ir-func function-metas idx len current-depth max-depth]
  (native-max-stack-depth-step-64-loop-bounded ir-func function-metas idx len current-depth max-depth 64))

(defn continue-native-max-stack-depth-step-64 [ir-func function-metas len state]
  (if (= (vector-get state 0) 1)
    state
    (do
      (root_push ir-func)
      (root_push function-metas)
      (root_push state)
      (let [next-state (native-max-stack-depth-step-64 ir-func function-metas (vector-get state 1) len (vector-get state 2) (vector-get state 3))]
        (do
          (root_push next-state)
          (let [final (continue-native-max-stack-depth-step-64 ir-func function-metas len next-state)]
            (do
              (root_pop)
              (root_pop)
              (root_pop)
              (root_pop)
              final)))))))

(defn native-max-stack-depth [ir-func function-metas]
  (vector-get
    (continue-native-max-stack-depth-step-64
      ir-func
      function-metas
      (vector-length ir-func)
      (native-max-stack-depth-step-64 ir-func function-metas 0 (vector-length ir-func) 0 0))
    3))

;; partial slice の current-depth に応じて必要な spill slot をそのまま確保する
(defn native-value-window-spill-slot-count [ir-func function-metas]
  (let [extra-depth (- (native-max-stack-depth ir-func function-metas) 2)]
    (if (< extra-depth 0)
      0
      extra-depth)))

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

(defn native-value-window-spill-slot-count-x86 [ir-func function-metas]
  (let [actual (native-value-window-spill-slot-count ir-func function-metas)]
    (if (< actual 128) 128 actual)))

(defn native-total-slot-count-with-window-x86 [ir-func min-slot-count function-metas]
  (+ (native-frame-base-slot-count ir-func min-slot-count)
     (native-value-window-spill-slot-count-x86 ir-func function-metas)))

(defn native-local-stack-bytes-with-window-x86 [ir-func min-slot-count function-metas]
  (let [slot-count (native-total-slot-count-with-window-x86 ir-func min-slot-count function-metas)]
    (if (= slot-count 0)
      0
      (align-16 (* slot-count 8)))))

(defn aarch64-bundle-stack-padding-needed [ir-func min-slot-count function-metas]
  (if (> (native-frame-base-slot-count ir-func min-slot-count) 0)
    1
    (if (> (native-value-window-spill-slot-count ir-func function-metas) 0)
      1
      0)))

;; AArch64 の leaf function は [sp, #8] 起点の slot が caller frame に食い込まないよう、
;; 偶数 slot 個のときだけ 1 alignment 分の padding を追加する。
(defn native-value-window-spill-offset [frame-base-slot-count spill-idx]
  (local-slot-offset (+ frame-base-slot-count spill-idx)))


;; x86_64 の SUB rsp, imm32
(defn emit-sub-rsp-imm32 [value]
  (let [imm (encode-u32-le value)]
    (concat-byte-vectors-rooted
      (byte-vector-3 72 129 236)
      imm)))

;; x86_64 の ADD rsp, imm32
(defn emit-add-rsp-imm32 [value]
  (let [imm (encode-u32-le value)]
    (concat-byte-vectors-rooted
      (byte-vector-3 72 129 196)
      imm)))

;; x86_64 の MOV [rbp-offset], rax
(defn emit-mov-local-from-rax [offset]
  (emit-x86-rbp-disp32 72 137 133 offset))

;; x86_64 の MOV [rbp-offset], rcx
(defn emit-mov-local-from-rcx [offset]
  (emit-x86-rbp-disp32 72 137 141 offset))

;; x86_64 の MOV [rbp-offset], rdi
(defn emit-mov-local-from-rdi [offset]
  (emit-x86-rbp-disp32 72 137 189 offset))

;; x86_64 の MOV [rbp-offset], rdx
(defn emit-mov-local-from-rdx [offset]
  (emit-x86-rbp-disp32 72 137 149 offset))

;; x86_64 の MOV [rbp-offset], rsi
(defn emit-mov-local-from-rsi [offset]
  (emit-x86-rbp-disp32 72 137 181 offset))

;; x86_64 の MOV [rbp-offset], r8
(defn emit-mov-local-from-r8 [offset]
  (emit-x86-rbp-disp32 76 137 133 offset))

;; x86_64 の MOV [rbp-offset], r9
(defn emit-mov-local-from-r9 [offset]
  (emit-x86-rbp-disp32 76 137 141 offset))

;; x86_64 の MOV [rsp], rax
(defn emit-mov-top-stack-from-rax []
  (let [bytes (vector-new 4)]
    (vector-push (vector-push (vector-push (vector-push bytes 72) 137) 4) 36)))

;; x86_64 の MOV [rsp], rcx
(defn emit-mov-top-stack-from-rcx []
  (let [bytes (vector-new 4)]
    (vector-push (vector-push (vector-push (vector-push bytes 72) 137) 12) 36)))

;; x86_64 の MOV [rsp+imm32], rax
(defn emit-mov-stack-slot-from-rax [offset]
  (let [disp (encode-u32-le offset)
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

;; x86_64 の MOV [rsp+imm32], rcx
(defn emit-mov-stack-slot-from-rcx [offset]
  (let [disp (encode-u32-le offset)
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

;; x86_64 の MOV [rsp+168], rcx
(defn emit-mov-twenty-second-stack-from-rcx []
  (let [disp (encode-u32-le 168)
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

;; x86_64 の MOV [rsp+176], rax
(defn emit-mov-twenty-third-stack-from-rax []
  (let [disp (encode-u32-le 176)
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

;; x86_64 の MOV [rsp+176], rcx
(defn emit-mov-twenty-third-stack-from-rcx []
  (let [disp (encode-u32-le 176)
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

;; x86_64 の MOV [rsp+184], rax
(defn emit-mov-twenty-fourth-stack-from-rax []
  (let [disp (encode-u32-le 184)
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

;; x86_64 の MOV [rsp+184], rcx
(defn emit-mov-twenty-fourth-stack-from-rcx []
  (let [disp (encode-u32-le 184)
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

;; x86_64 の MOV [rsp+192], rax
(defn emit-mov-twenty-fifth-stack-from-rax []
  (let [disp (encode-u32-le 192)
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

;; x86_64 の MOV [rsp+192], rcx
(defn emit-mov-twenty-fifth-stack-from-rcx []
  (let [disp (encode-u32-le 192)
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

;; x86_64 の MOV [rsp+200], rax
(defn emit-mov-twenty-sixth-stack-from-rax []
  (let [disp (encode-u32-le 200)
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
  (emit-x86-rbp-disp32 72 139 133 offset))

;; x86_64 の MOV rdi, [rbp-offset]
(defn emit-mov-rdi-from-local [offset]
  (emit-x86-rbp-disp32 72 139 189 offset))

;; x86_64 の MOV rsi, [rbp-offset]
(defn emit-mov-rsi-from-local [offset]
  (emit-x86-rbp-disp32 72 139 181 offset))

;; x86_64 の MOV rcx, [rbp-offset]
(defn emit-mov-rcx-from-local [offset]
  (emit-x86-rbp-disp32 72 139 141 offset))

;; x86_64 の MOV rdx, [rbp-offset]
(defn emit-mov-rdx-from-local [offset]
  (emit-x86-rbp-disp32 72 139 149 offset))

;; x86_64 の MOV rax, [rax+offset]
(defn emit-mov-rax-from-rax-plus-offset [offset]
  (if (< offset 128)
    (let [bytes (vector-new 4)]
      (vector-push
        (vector-push
          (vector-push
            (vector-push bytes 72)
            139)
          64)
        offset))
    (let [disp (encode-u32-le offset)
      bytes (vector-new 7)
      b1 (vector-push bytes 72)
      b2 (vector-push b1 139)
      b3 (vector-push b2 128)
      b4 (vector-push b3 (vector-get disp 0))
      b5 (vector-push b4 (vector-get disp 1))
      b6 (vector-push b5 (vector-get disp 2))
      b7 (vector-push b6 (vector-get disp 3))]
      b7)))

;; x86_64 の MOV eax, [rax+offset]
(defn emit-mov-eax-from-rax-plus-offset [offset]
  (if (< offset 128)
    (let [bytes (vector-new 3)]
      (vector-push
        (vector-push
          (vector-push bytes 139)
          64)
        offset))
    (let [disp (encode-u32-le offset)
      bytes (vector-new 6)
      b1 (vector-push bytes 139)
      b2 (vector-push b1 128)
      b3 (vector-push b2 (vector-get disp 0))
      b4 (vector-push b3 (vector-get disp 1))
      b5 (vector-push b4 (vector-get disp 2))
      b6 (vector-push b5 (vector-get disp 3))]
      b6)))

;; x86_64 の MOVZX eax, byte ptr [rax+offset]
(defn emit-movzx-eax-from-rax-plus-offset [offset]
  (if (< offset 128)
    (let [bytes (vector-new 4)]
      (vector-push
        (vector-push
          (vector-push
            (vector-push bytes 15)
            182)
          64)
        offset))
    (let [disp (encode-u32-le offset)
      bytes (vector-new 7)
      b1 (vector-push bytes 15)
      b2 (vector-push b1 182)
      b3 (vector-push b2 128)
      b4 (vector-push b3 (vector-get disp 0))
      b5 (vector-push b4 (vector-get disp 1))
      b6 (vector-push b5 (vector-get disp 2))
      b7 (vector-push b6 (vector-get disp 3))]
      b7)))

;; x86_64 の MOV [rcx+offset], rax
(defn emit-mov-rcx-plus-offset-from-rax [offset]
  (if (< offset 128)
    (let [bytes (vector-new 4)]
      (vector-push
        (vector-push
          (vector-push
            (vector-push bytes 72)
            137)
          65)
        offset))
    (let [disp (encode-u32-le offset)
      bytes (vector-new 7)
      b1 (vector-push bytes 72)
      b2 (vector-push b1 137)
      b3 (vector-push b2 129)
      b4 (vector-push b3 (vector-get disp 0))
      b5 (vector-push b4 (vector-get disp 1))
      b6 (vector-push b5 (vector-get disp 2))
      b7 (vector-push b6 (vector-get disp 3))]
      b7)))

;; x86_64 の MOV [rcx+offset], eax
(defn emit-mov-rcx-plus-offset-from-eax [offset]
  (if (< offset 128)
    (let [bytes (vector-new 3)]
      (vector-push
        (vector-push
          (vector-push bytes 137)
          65)
        offset))
    (let [disp (encode-u32-le offset)
      bytes (vector-new 6)
      b1 (vector-push bytes 137)
      b2 (vector-push b1 129)
      b3 (vector-push b2 (vector-get disp 0))
      b4 (vector-push b3 (vector-get disp 1))
      b5 (vector-push b4 (vector-get disp 2))
      b6 (vector-push b5 (vector-get disp 3))]
      b6)))

;; x86_64 の MOV r8, [rbp-offset]
(defn emit-mov-r8-from-local [offset]
  (emit-x86-rbp-disp32 76 139 133 offset))

;; x86_64 の MOV r9, [rbp-offset]
(defn emit-mov-r9-from-local [offset]
  (emit-x86-rbp-disp32 76 139 141 offset))

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

;; x86_64 の REP MOVSB
(defn emit-rep-movsb []
  (let [bytes (vector-new 2)]
    (vector-push (vector-push bytes 243) 164)))

;; x86_64 の REP STOSB
(defn emit-rep-stosb []
  (let [bytes (vector-new 2)]
    (vector-push (vector-push bytes 243) 170)))

;; x86_64 の local.get: 直前値を rcx へ逃がしてから rax へ load
(defn emit-local-get-x86 [offset]
  (concat-byte-vectors-rooted
    (byte-vector-3 72 137 193)
    (emit-mov-rax-from-local offset)))

;; x86_64 の i32.const: 直前値を rcx へ逃がしてから eax へ即値をロード
(defn emit-i32-const-x86 [value]
  (let [mov-imm (emit-mov-eax-imm32 value)]
    (concat-byte-vectors-rooted
      (byte-vector-3 72 137 193)
      mov-imm)))

;; x86_64 CMP rcx, rax
(defn emit-cmp-rcx-rax []
  (let [bytes (vector-new 3)]
    (vector-push (vector-push (vector-push bytes 72) 57) 193)))

;; x86_64 SETE al
(defn emit-sete-al []
  (let [bytes (vector-new 3)]
    (vector-push (vector-push (vector-push bytes 15) 148) 192)))

;; x86_64 SETNE al
(defn emit-setne-al []
  (let [bytes (vector-new 3)]
    (vector-push (vector-push (vector-push bytes 15) 149) 192)))

;; x86_64 SETL al
(defn emit-setl-al []
  (let [bytes (vector-new 3)]
    (vector-push (vector-push (vector-push bytes 15) 156) 192)))

;; x86_64 SETG al
(defn emit-setg-al []
  (let [bytes (vector-new 3)]
    (vector-push (vector-push (vector-push bytes 15) 159) 192)))

;; x86_64 SETLE al
(defn emit-setle-al []
  (let [bytes (vector-new 3)]
    (vector-push (vector-push (vector-push bytes 15) 158) 192)))

;; x86_64 SETGE al
(defn emit-setge-al []
  (let [bytes (vector-new 3)]
    (vector-push (vector-push (vector-push bytes 15) 157) 192)))

;; x86_64 MOVZX eax, al
(defn emit-movzx-eax-al []
  (let [bytes (vector-new 3)]
    (vector-push (vector-push (vector-push bytes 15) 182) 192)))

(defn emit-compare-x86 [setcc]
  (concat-byte-vectors
    (concat-byte-vectors
      (emit-cmp-rcx-rax)
      setcc)
    (emit-movzx-eax-al)))

(defn emit-i64-eq-x86 []
  (emit-compare-x86 (emit-sete-al)))

(defn emit-i64-ne-x86 []
  (emit-compare-x86 (emit-setne-al)))

(defn emit-i64-lt-x86 []
  (emit-compare-x86 (emit-setl-al)))

(defn emit-i64-gt-x86 []
  (emit-compare-x86 (emit-setg-al)))

(defn emit-i64-le-x86 []
  (emit-compare-x86 (emit-setle-al)))

(defn emit-i64-ge-x86 []
  (emit-compare-x86 (emit-setge-al)))

(defn is-i64-compare-opcode [opcode]
  (if (= opcode 30)
    1
    (if (= opcode 31)
      1
      (if (= opcode 32)
        1
        (if (= opcode 33)
          1
          (if (= opcode 34)
            1
            (if (= opcode 35)
              1
            0)))))))

(defn emit-i64-compare-x86 [opcode]
  (if (= opcode 30)
    (emit-i64-eq-x86)
    (if (= opcode 31)
      (emit-i64-ne-x86)
      (if (= opcode 32)
        (emit-i64-lt-x86)
        (if (= opcode 33)
          (emit-i64-gt-x86)
          (if (= opcode 34)
            (emit-i64-le-x86)
            (emit-i64-ge-x86)))))))

;; x86_64 の MOV [rsp+200], rcx
(defn emit-mov-twenty-sixth-stack-from-rcx []
  (let [disp (encode-u32-le 200)
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

;; x86_64 の MOV [rsp+208], rax
(defn emit-mov-twenty-seventh-stack-from-rax []
  (let [disp (encode-u32-le 208)
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

;; x86_64 の MOV [rsp+208], rcx
(defn emit-mov-twenty-seventh-stack-from-rcx []
  (let [disp (encode-u32-le 208)
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

;; x86_64 の MOV [rsp+216], rax
(defn emit-mov-twenty-eighth-stack-from-rax []
  (let [disp (encode-u32-le 216)
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

;; x86_64 の MOV [rsp+216], rcx
(defn emit-mov-twenty-eighth-stack-from-rcx []
  (let [disp (encode-u32-le 216)
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

;; x86_64 の MOV [rsp+224], rax
(defn emit-mov-twenty-ninth-stack-from-rax []
  (let [disp (encode-u32-le 224)
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

;; x86_64 の MOV [rsp+224], rcx
(defn emit-mov-twenty-ninth-stack-from-rcx []
  (let [disp (encode-u32-le 224)
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

;; x86_64 の MOV [rsp+232], rax
(defn emit-mov-thirtieth-stack-from-rax []
  (let [disp (encode-u32-le 232)
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

;; x86_64 の MOV [rsp+232], rcx
(defn emit-mov-thirtieth-stack-from-rcx []
  (let [disp (encode-u32-le 232)
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

;; x86_64 の MOV [rsp+240], rax
(defn emit-mov-thirty-first-stack-from-rax []
  (let [disp (encode-u32-le 240)
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

;; x86_64 の MOV [rsp+240], rcx
(defn emit-mov-thirty-first-stack-from-rcx []
  (let [disp (encode-u32-le 240)
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

;; x86_64 の MOV [rsp+248], rax
(defn emit-mov-thirty-second-stack-from-rax []
  (let [disp (encode-u32-le 248)
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

;; x86_64 の MOV [rsp+248], rcx
(defn emit-mov-thirty-second-stack-from-rcx []
  (let [disp (encode-u32-le 248)
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

;; x86_64 の MOV [rsp+256], rcx
(defn emit-mov-thirty-third-stack-from-rcx []
  (let [disp (encode-u32-le 256)
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

;; x86_64 の MOV [rsp+256], rax
(defn emit-mov-thirty-third-stack-from-rax []
  (let [disp (encode-u32-le 256)
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

;; x86_64 の MOV [rsp+264], rax
(defn emit-mov-thirty-fourth-stack-from-rax []
  (let [disp (encode-u32-le 264)
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

;; x86_64 の MOV [rsp+264], rcx
(defn emit-mov-thirty-fourth-stack-from-rcx []
  (let [disp (encode-u32-le 264)
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

;; x86_64 の MOV [rsp+272], rax
(defn emit-mov-thirty-fifth-stack-from-rax []
  (let [disp (encode-u32-le 272)
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

;; x86_64 の MOV [rsp+272], rcx
(defn emit-mov-thirty-fifth-stack-from-rcx []
  (let [disp (encode-u32-le 272)
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

;; x86_64 の MOV [rsp+280], rax
(defn emit-mov-thirty-sixth-stack-from-rax []
  (let [disp (encode-u32-le 280)
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

;; x86_64 の MOV [rsp+280], rcx
(defn emit-mov-thirty-sixth-stack-from-rcx []
  (let [disp (encode-u32-le 280)
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

;; x86_64 の MOV [rsp+288], rax
(defn emit-mov-thirty-seventh-stack-from-rax []
  (let [disp (encode-u32-le 288)
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

;; x86_64 の MOV [rsp+288], rcx
(defn emit-mov-thirty-seventh-stack-from-rcx []
  (let [disp (encode-u32-le 288)
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

;; x86_64 の MOV [rsp+296], rax
(defn emit-mov-thirty-eighth-stack-from-rax []
  (let [disp (encode-u32-le 296)
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

;; x86_64 の MOV [rsp+296], rcx
(defn emit-mov-thirty-eighth-stack-from-rcx []
  (let [disp (encode-u32-le 296)
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

;; x86_64 の MOV [rsp+304], rax
(defn emit-mov-thirty-ninth-stack-from-rax []
  (let [disp (encode-u32-le 304)
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

;; x86_64 の MOV [rsp+304], rcx
(defn emit-mov-thirty-ninth-stack-from-rcx []
  (let [disp (encode-u32-le 304)
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

;; x86_64 の MOV [rsp+312], rax
(defn emit-mov-fortieth-stack-from-rax []
  (let [disp (encode-u32-le 312)
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

;; x86_64 の MOV [rsp+312], rcx
(defn emit-mov-fortieth-stack-from-rcx []
  (let [disp (encode-u32-le 312)
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

;; x86_64 の MOV [rsp+320], rax
(defn emit-mov-forty-first-stack-from-rax []
  (let [disp (encode-u32-le 320)
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

;; x86_64 の MOV [rsp+320], rcx
(defn emit-mov-forty-first-stack-from-rcx []
  (let [disp (encode-u32-le 320)
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

;; x86_64 の MOV [rsp+328], rax
(defn emit-mov-forty-second-stack-from-rax []
  (let [disp (encode-u32-le 328)
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

;; x86_64 の MOV [rsp+328], rcx
(defn emit-mov-forty-second-stack-from-rcx []
  (let [disp (encode-u32-le 328)
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

;; x86_64 の MOV [rsp+336], rax
(defn emit-mov-forty-third-stack-from-rax []
  (let [disp (encode-u32-le 336)
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

;; x86_64 の MOV [rsp+336], rcx
(defn emit-mov-forty-third-stack-from-rcx []
  (let [disp (encode-u32-le 336)
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

;; x86_64 の MOV [rsp+344], rax
(defn emit-mov-forty-fourth-stack-from-rax []
  (let [disp (encode-u32-le 344)
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

;; x86_64 の MOV [rsp+344], rcx
(defn emit-mov-forty-fourth-stack-from-rcx []
  (let [disp (encode-u32-le 344)
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

;; x86_64 の MOV [rsp+352], rax
(defn emit-mov-forty-fifth-stack-from-rax []
  (let [disp (encode-u32-le 352)
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

;; x86_64 bundle の i32.const: spill window が必要なら old previous を spill する
(defn spill-native-value-window-one-step-x86 [frame-base-slot-count current-depth]
  (concat-byte-vectors
    (emit-mov-rdx-from-local (native-value-window-spill-offset frame-base-slot-count (- current-depth 3)))
    (emit-mov-local-from-rdx (native-value-window-spill-offset frame-base-slot-count (- current-depth 2)))))

(defn emit-i32-const-bundle-x86 [value frame-base-slot-count current-depth]
  (if (>= current-depth 55)
    (concat-byte-vectors
      (spill-native-value-window-one-step-x86 frame-base-slot-count current-depth)
      (emit-i32-const-bundle-x86 value frame-base-slot-count (- current-depth 1)))
    (emit-i32-const-bundle-x86-core value frame-base-slot-count current-depth)))

(defn shift-native-value-window-x86-loop [frame-base-slot-count idx]
  (if (< idx 0)
    (vector-new 0)
    (concat-byte-vectors-rooted
      (concat-byte-vectors-rooted
        (emit-mov-rdx-from-local (native-value-window-spill-offset frame-base-slot-count idx))
        (emit-mov-local-from-rdx (native-value-window-spill-offset frame-base-slot-count (+ idx 1))))
      (shift-native-value-window-x86-loop frame-base-slot-count (- idx 1)))))

(defn emit-i32-const-bundle-x86-core [value frame-base-slot-count current-depth]
  (if (>= current-depth 2)
    (concat-byte-vectors-rooted
      (concat-byte-vectors-rooted
        (shift-native-value-window-x86-loop frame-base-slot-count (- current-depth 3))
        (emit-mov-local-from-rcx (native-value-window-spill-offset frame-base-slot-count 0)))
      (emit-i32-const-x86 value))
    (emit-i32-const-x86 value)))

;; x86_64 bundle の i64.const: 直前値がある場合は rcx/window へ保存する
(defn emit-i64-const-preserve-x86 [value]
  (concat-byte-vectors-rooted
    (byte-vector-3 72 137 193)
    (emit-mov-imm64 (reg-rax) value)))

(defn emit-i64-const-bundle-x86-core [value frame-base-slot-count current-depth]
  (if (>= current-depth 2)
    (concat-byte-vectors-rooted
      (concat-byte-vectors-rooted
        (shift-native-value-window-x86-loop frame-base-slot-count (- current-depth 3))
        (emit-mov-local-from-rcx (native-value-window-spill-offset frame-base-slot-count 0)))
      (emit-i64-const-preserve-x86 value))
    (if (= current-depth 1)
      (emit-i64-const-preserve-x86 value)
      (emit-mov-imm64 (reg-rax) value))))

(defn emit-i64-const-bundle-x86 [value frame-base-slot-count current-depth]
  (if (>= current-depth 55)
    (concat-byte-vectors
      (spill-native-value-window-one-step-x86 frame-base-slot-count current-depth)
      (emit-i64-const-bundle-x86 value frame-base-slot-count (- current-depth 1)))
    (emit-i64-const-bundle-x86-core value frame-base-slot-count current-depth)))

;; x86_64 bundle の local.get: spill window が必要なら old previous を spill する
(defn emit-local-get-bundle-x86 [offset frame-base-slot-count current-depth]
  (if (>= current-depth 55)
    (concat-byte-vectors
      (spill-native-value-window-one-step-x86 frame-base-slot-count current-depth)
      (emit-local-get-bundle-x86 offset frame-base-slot-count (- current-depth 1)))
    (emit-local-get-bundle-x86-core offset frame-base-slot-count current-depth)))

(defn emit-local-get-bundle-x86-core [offset frame-base-slot-count current-depth]
  (if (>= current-depth 2)
    (concat-three-byte-vectors-rooted
      (concat-byte-vectors-rooted
        (shift-native-value-window-x86-loop frame-base-slot-count (- current-depth 3))
        (emit-mov-local-from-rcx (native-value-window-spill-offset frame-base-slot-count 0)))
      (emit-mov-rcx-rax)
      (emit-local-get-x86 offset))
    (emit-local-get-x86 offset)))

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

(defn emit-twenty-nine-arg-call-x86 [rel frame-base-slot-count]
  (let [stack0 (concat-byte-vectors
                 (emit-sub-rsp-imm32 192)
                 (emit-mov-twenty-third-stack-from-rax))
        stack1 (concat-byte-vectors stack0 (emit-mov-twenty-second-stack-from-rcx))
        stack2 (concat-byte-vectors
                 stack1
                 (emit-mov-rcx-from-local (native-value-window-spill-offset frame-base-slot-count 0)))
        stack3 (concat-byte-vectors stack2 (emit-mov-twenty-first-stack-from-rcx))
        stack4 (concat-byte-vectors
                 stack3
                 (emit-mov-rcx-from-local (native-value-window-spill-offset frame-base-slot-count 1)))
        stack5 (concat-byte-vectors stack4 (emit-mov-twentieth-stack-from-rcx))
        stack6 (concat-byte-vectors
                 stack5
                 (emit-mov-rcx-from-local (native-value-window-spill-offset frame-base-slot-count 2)))
        stack7 (concat-byte-vectors stack6 (emit-mov-nineteenth-stack-from-rcx))
        stack8 (concat-byte-vectors
                 stack7
                 (emit-mov-rcx-from-local (native-value-window-spill-offset frame-base-slot-count 3)))
        stack9 (concat-byte-vectors stack8 (emit-mov-eighteenth-stack-from-rcx))
        stack10 (concat-byte-vectors
                  stack9
                  (emit-mov-rcx-from-local (native-value-window-spill-offset frame-base-slot-count 4)))
        stack11 (concat-byte-vectors stack10 (emit-mov-seventeenth-stack-from-rcx))
        stack12 (concat-byte-vectors
                  stack11
                  (emit-mov-rcx-from-local (native-value-window-spill-offset frame-base-slot-count 5)))
        stack13 (concat-byte-vectors stack12 (emit-mov-sixteenth-stack-from-rcx))
        stack14 (concat-byte-vectors
                  stack13
                  (emit-mov-rcx-from-local (native-value-window-spill-offset frame-base-slot-count 6)))
        stack15 (concat-byte-vectors stack14 (emit-mov-fifteenth-stack-from-rcx))
        stack16 (concat-byte-vectors
                  stack15
                  (emit-mov-rcx-from-local (native-value-window-spill-offset frame-base-slot-count 7)))
        stack17 (concat-byte-vectors stack16 (emit-mov-fourteenth-stack-from-rcx))
        stack18 (concat-byte-vectors
                  stack17
                  (emit-mov-rcx-from-local (native-value-window-spill-offset frame-base-slot-count 8)))
        stack19 (concat-byte-vectors stack18 (emit-mov-thirteenth-stack-from-rcx))
        stack20 (concat-byte-vectors
                  stack19
                  (emit-mov-rcx-from-local (native-value-window-spill-offset frame-base-slot-count 9)))
        stack21 (concat-byte-vectors stack20 (emit-mov-twelfth-stack-from-rcx))
        stack22 (concat-byte-vectors
                  stack21
                  (emit-mov-rcx-from-local (native-value-window-spill-offset frame-base-slot-count 10)))
        stack23 (concat-byte-vectors stack22 (emit-mov-eleventh-stack-from-rcx))
        stack24 (concat-byte-vectors
                  stack23
                  (emit-mov-rcx-from-local (native-value-window-spill-offset frame-base-slot-count 11)))
        stack25 (concat-byte-vectors stack24 (emit-mov-tenth-stack-from-rcx))
        stack26 (concat-byte-vectors
                  stack25
                  (emit-mov-rcx-from-local (native-value-window-spill-offset frame-base-slot-count 12)))
        stack27 (concat-byte-vectors stack26 (emit-mov-ninth-stack-from-rcx))
        stack28 (concat-byte-vectors
                  stack27
                  (emit-mov-rcx-from-local (native-value-window-spill-offset frame-base-slot-count 13)))
        stack29 (concat-byte-vectors stack28 (emit-mov-eighth-stack-from-rcx))
        stack30 (concat-byte-vectors
                  stack29
                  (emit-mov-rcx-from-local (native-value-window-spill-offset frame-base-slot-count 14)))
        stack31 (concat-byte-vectors stack30 (emit-mov-seventh-stack-from-rcx))
        stack32 (concat-byte-vectors
                  stack31
                  (emit-mov-rcx-from-local (native-value-window-spill-offset frame-base-slot-count 15)))
        stack33 (concat-byte-vectors stack32 (emit-mov-sixth-stack-from-rcx))
        stack34 (concat-byte-vectors
                  stack33
                  (emit-mov-rcx-from-local (native-value-window-spill-offset frame-base-slot-count 16)))
        stack35 (concat-byte-vectors stack34 (emit-mov-fifth-stack-from-rcx))
        stack36 (concat-byte-vectors
                  stack35
                  (emit-mov-rcx-from-local (native-value-window-spill-offset frame-base-slot-count 17)))
        stack37 (concat-byte-vectors stack36 (emit-mov-fourth-stack-from-rcx))
        stack38 (concat-byte-vectors
                  stack37
                  (emit-mov-rcx-from-local (native-value-window-spill-offset frame-base-slot-count 18)))
        stack39 (concat-byte-vectors stack38 (emit-mov-third-stack-from-rcx))
        stack40 (concat-byte-vectors
                  stack39
                  (emit-mov-rcx-from-local (native-value-window-spill-offset frame-base-slot-count 19)))
        stack41 (concat-byte-vectors stack40 (emit-mov-second-stack-from-rcx))
        stack-setup (concat-byte-vectors
                      stack41
                      (emit-mov-r9-from-local (native-value-window-spill-offset frame-base-slot-count 20)))
        reg0 (concat-byte-vectors
               (emit-mov-top-stack-from-r9)
               (emit-mov-r9-from-local (native-value-window-spill-offset frame-base-slot-count 21)))
        reg1 (concat-byte-vectors reg0 (emit-mov-r8-from-local (native-value-window-spill-offset frame-base-slot-count 22)))
        reg2 (concat-byte-vectors reg1 (emit-mov-rcx-from-local (native-value-window-spill-offset frame-base-slot-count 23)))
        reg3 (concat-byte-vectors reg2 (emit-mov-rdx-from-local (native-value-window-spill-offset frame-base-slot-count 24)))
        reg4 (concat-byte-vectors reg3 (emit-mov-rsi-from-local (native-value-window-spill-offset frame-base-slot-count 25)))
        reg-setup (concat-byte-vectors reg4 (emit-mov-rdi-from-local (native-value-window-spill-offset frame-base-slot-count 26)))
        call-seq (concat-byte-vectors
                   (emit-call-rel32 rel)
                   (emit-add-rsp-imm32 192))]
    (concat-byte-vectors
      (concat-byte-vectors stack-setup reg-setup)
      call-seq)))

(defn emit-thirty-arg-call-x86 [rel frame-base-slot-count]
  (let [stack0 (concat-byte-vectors
                 (emit-sub-rsp-imm32 192)
                 (emit-mov-twenty-fourth-stack-from-rax))
        stack1 (concat-byte-vectors stack0 (emit-mov-twenty-third-stack-from-rcx))
        stack2 (concat-byte-vectors
                 stack1
                 (emit-mov-rcx-from-local (native-value-window-spill-offset frame-base-slot-count 0)))
        stack3 (concat-byte-vectors stack2 (emit-mov-twenty-second-stack-from-rcx))
        stack4 (concat-byte-vectors
                 stack3
                 (emit-mov-rcx-from-local (native-value-window-spill-offset frame-base-slot-count 1)))
        stack5 (concat-byte-vectors stack4 (emit-mov-twenty-first-stack-from-rcx))
        stack6 (concat-byte-vectors
                 stack5
                 (emit-mov-rcx-from-local (native-value-window-spill-offset frame-base-slot-count 2)))
        stack7 (concat-byte-vectors stack6 (emit-mov-twentieth-stack-from-rcx))
        stack8 (concat-byte-vectors
                 stack7
                 (emit-mov-rcx-from-local (native-value-window-spill-offset frame-base-slot-count 3)))
        stack9 (concat-byte-vectors stack8 (emit-mov-nineteenth-stack-from-rcx))
        stack10 (concat-byte-vectors
                  stack9
                  (emit-mov-rcx-from-local (native-value-window-spill-offset frame-base-slot-count 4)))
        stack11 (concat-byte-vectors stack10 (emit-mov-eighteenth-stack-from-rcx))
        stack12 (concat-byte-vectors
                  stack11
                  (emit-mov-rcx-from-local (native-value-window-spill-offset frame-base-slot-count 5)))
        stack13 (concat-byte-vectors stack12 (emit-mov-seventeenth-stack-from-rcx))
        stack14 (concat-byte-vectors
                  stack13
                  (emit-mov-rcx-from-local (native-value-window-spill-offset frame-base-slot-count 6)))
        stack15 (concat-byte-vectors stack14 (emit-mov-sixteenth-stack-from-rcx))
        stack16 (concat-byte-vectors
                  stack15
                  (emit-mov-rcx-from-local (native-value-window-spill-offset frame-base-slot-count 7)))
        stack17 (concat-byte-vectors stack16 (emit-mov-fifteenth-stack-from-rcx))
        stack18 (concat-byte-vectors
                  stack17
                  (emit-mov-rcx-from-local (native-value-window-spill-offset frame-base-slot-count 8)))
        stack19 (concat-byte-vectors stack18 (emit-mov-fourteenth-stack-from-rcx))
        stack20 (concat-byte-vectors
                  stack19
                  (emit-mov-rcx-from-local (native-value-window-spill-offset frame-base-slot-count 9)))
        stack21 (concat-byte-vectors stack20 (emit-mov-thirteenth-stack-from-rcx))
        stack22 (concat-byte-vectors
                  stack21
                  (emit-mov-rcx-from-local (native-value-window-spill-offset frame-base-slot-count 10)))
        stack23 (concat-byte-vectors stack22 (emit-mov-twelfth-stack-from-rcx))
        stack24 (concat-byte-vectors
                  stack23
                  (emit-mov-rcx-from-local (native-value-window-spill-offset frame-base-slot-count 11)))
        stack25 (concat-byte-vectors stack24 (emit-mov-eleventh-stack-from-rcx))
        stack26 (concat-byte-vectors
                  stack25
                  (emit-mov-rcx-from-local (native-value-window-spill-offset frame-base-slot-count 12)))
        stack27 (concat-byte-vectors stack26 (emit-mov-tenth-stack-from-rcx))
        stack28 (concat-byte-vectors
                  stack27
                  (emit-mov-rcx-from-local (native-value-window-spill-offset frame-base-slot-count 13)))
        stack29 (concat-byte-vectors stack28 (emit-mov-ninth-stack-from-rcx))
        stack30 (concat-byte-vectors
                  stack29
                  (emit-mov-rcx-from-local (native-value-window-spill-offset frame-base-slot-count 14)))
        stack31 (concat-byte-vectors stack30 (emit-mov-eighth-stack-from-rcx))
        stack32 (concat-byte-vectors
                  stack31
                  (emit-mov-rcx-from-local (native-value-window-spill-offset frame-base-slot-count 15)))
        stack33 (concat-byte-vectors stack32 (emit-mov-seventh-stack-from-rcx))
        stack34 (concat-byte-vectors
                  stack33
                  (emit-mov-rcx-from-local (native-value-window-spill-offset frame-base-slot-count 16)))
        stack35 (concat-byte-vectors stack34 (emit-mov-sixth-stack-from-rcx))
        stack36 (concat-byte-vectors
                  stack35
                  (emit-mov-rcx-from-local (native-value-window-spill-offset frame-base-slot-count 17)))
        stack37 (concat-byte-vectors stack36 (emit-mov-fifth-stack-from-rcx))
        stack38 (concat-byte-vectors
                  stack37
                  (emit-mov-rcx-from-local (native-value-window-spill-offset frame-base-slot-count 18)))
        stack39 (concat-byte-vectors stack38 (emit-mov-fourth-stack-from-rcx))
        stack40 (concat-byte-vectors
                  stack39
                  (emit-mov-rcx-from-local (native-value-window-spill-offset frame-base-slot-count 19)))
        stack41 (concat-byte-vectors stack40 (emit-mov-third-stack-from-rcx))
        stack42 (concat-byte-vectors
                  stack41
                  (emit-mov-rcx-from-local (native-value-window-spill-offset frame-base-slot-count 20)))
        stack43 (concat-byte-vectors stack42 (emit-mov-second-stack-from-rcx))
        stack-setup (concat-byte-vectors
                      stack43
                      (emit-mov-r9-from-local (native-value-window-spill-offset frame-base-slot-count 21)))
        reg0 (concat-byte-vectors
               (emit-mov-top-stack-from-r9)
               (emit-mov-r9-from-local (native-value-window-spill-offset frame-base-slot-count 22)))
        reg1 (concat-byte-vectors reg0 (emit-mov-r8-from-local (native-value-window-spill-offset frame-base-slot-count 23)))
        reg2 (concat-byte-vectors reg1 (emit-mov-rcx-from-local (native-value-window-spill-offset frame-base-slot-count 24)))
        reg3 (concat-byte-vectors reg2 (emit-mov-rdx-from-local (native-value-window-spill-offset frame-base-slot-count 25)))
        reg4 (concat-byte-vectors reg3 (emit-mov-rsi-from-local (native-value-window-spill-offset frame-base-slot-count 26)))
        reg-setup (concat-byte-vectors reg4 (emit-mov-rdi-from-local (native-value-window-spill-offset frame-base-slot-count 27)))
        call-seq (concat-byte-vectors
                   (emit-call-rel32 rel)
                   (emit-add-rsp-imm32 192))]
    (concat-byte-vectors
      (concat-byte-vectors stack-setup reg-setup)
      call-seq)))

(defn emit-thirty-one-arg-call-x86 [rel frame-base-slot-count]
  (let [stack0 (concat-byte-vectors
                 (emit-sub-rsp-imm32 208)
                 (emit-mov-twenty-fifth-stack-from-rax))
        stack1 (concat-byte-vectors stack0 (emit-mov-twenty-fourth-stack-from-rcx))
        stack2 (concat-byte-vectors
                 stack1
                 (emit-mov-rcx-from-local (native-value-window-spill-offset frame-base-slot-count 0)))
        stack3 (concat-byte-vectors stack2 (emit-mov-twenty-third-stack-from-rcx))
        stack4 (concat-byte-vectors
                 stack3
                 (emit-mov-rcx-from-local (native-value-window-spill-offset frame-base-slot-count 1)))
        stack5 (concat-byte-vectors stack4 (emit-mov-twenty-second-stack-from-rcx))
        stack6 (concat-byte-vectors
                 stack5
                 (emit-mov-rcx-from-local (native-value-window-spill-offset frame-base-slot-count 2)))
        stack7 (concat-byte-vectors stack6 (emit-mov-twenty-first-stack-from-rcx))
        stack8 (concat-byte-vectors
                 stack7
                 (emit-mov-rcx-from-local (native-value-window-spill-offset frame-base-slot-count 3)))
        stack9 (concat-byte-vectors stack8 (emit-mov-twentieth-stack-from-rcx))
        stack10 (concat-byte-vectors
                  stack9
                  (emit-mov-rcx-from-local (native-value-window-spill-offset frame-base-slot-count 4)))
        stack11 (concat-byte-vectors stack10 (emit-mov-nineteenth-stack-from-rcx))
        stack12 (concat-byte-vectors
                  stack11
                  (emit-mov-rcx-from-local (native-value-window-spill-offset frame-base-slot-count 5)))
        stack13 (concat-byte-vectors stack12 (emit-mov-eighteenth-stack-from-rcx))
        stack14 (concat-byte-vectors
                  stack13
                  (emit-mov-rcx-from-local (native-value-window-spill-offset frame-base-slot-count 6)))
        stack15 (concat-byte-vectors stack14 (emit-mov-seventeenth-stack-from-rcx))
        stack16 (concat-byte-vectors
                  stack15
                  (emit-mov-rcx-from-local (native-value-window-spill-offset frame-base-slot-count 7)))
        stack17 (concat-byte-vectors stack16 (emit-mov-sixteenth-stack-from-rcx))
        stack18 (concat-byte-vectors
                  stack17
                  (emit-mov-rcx-from-local (native-value-window-spill-offset frame-base-slot-count 8)))
        stack19 (concat-byte-vectors stack18 (emit-mov-fifteenth-stack-from-rcx))
        stack20 (concat-byte-vectors
                  stack19
                  (emit-mov-rcx-from-local (native-value-window-spill-offset frame-base-slot-count 9)))
        stack21 (concat-byte-vectors stack20 (emit-mov-fourteenth-stack-from-rcx))
        stack22 (concat-byte-vectors
                  stack21
                  (emit-mov-rcx-from-local (native-value-window-spill-offset frame-base-slot-count 10)))
        stack23 (concat-byte-vectors stack22 (emit-mov-thirteenth-stack-from-rcx))
        stack24 (concat-byte-vectors
                  stack23
                  (emit-mov-rcx-from-local (native-value-window-spill-offset frame-base-slot-count 11)))
        stack25 (concat-byte-vectors stack24 (emit-mov-twelfth-stack-from-rcx))
        stack26 (concat-byte-vectors
                  stack25
                  (emit-mov-rcx-from-local (native-value-window-spill-offset frame-base-slot-count 12)))
        stack27 (concat-byte-vectors stack26 (emit-mov-eleventh-stack-from-rcx))
        stack28 (concat-byte-vectors
                  stack27
                  (emit-mov-rcx-from-local (native-value-window-spill-offset frame-base-slot-count 13)))
        stack29 (concat-byte-vectors stack28 (emit-mov-tenth-stack-from-rcx))
        stack30 (concat-byte-vectors
                  stack29
                  (emit-mov-rcx-from-local (native-value-window-spill-offset frame-base-slot-count 14)))
        stack31 (concat-byte-vectors stack30 (emit-mov-ninth-stack-from-rcx))
        stack32 (concat-byte-vectors
                  stack31
                  (emit-mov-rcx-from-local (native-value-window-spill-offset frame-base-slot-count 15)))
        stack33 (concat-byte-vectors stack32 (emit-mov-eighth-stack-from-rcx))
        stack34 (concat-byte-vectors
                  stack33
                  (emit-mov-rcx-from-local (native-value-window-spill-offset frame-base-slot-count 16)))
        stack35 (concat-byte-vectors stack34 (emit-mov-seventh-stack-from-rcx))
        stack36 (concat-byte-vectors
                  stack35
                  (emit-mov-rcx-from-local (native-value-window-spill-offset frame-base-slot-count 17)))
        stack37 (concat-byte-vectors stack36 (emit-mov-sixth-stack-from-rcx))
        stack38 (concat-byte-vectors
                  stack37
                  (emit-mov-rcx-from-local (native-value-window-spill-offset frame-base-slot-count 18)))
        stack39 (concat-byte-vectors stack38 (emit-mov-fifth-stack-from-rcx))
        stack40 (concat-byte-vectors
                  stack39
                  (emit-mov-rcx-from-local (native-value-window-spill-offset frame-base-slot-count 19)))
        stack41 (concat-byte-vectors stack40 (emit-mov-fourth-stack-from-rcx))
        stack42 (concat-byte-vectors
                  stack41
                  (emit-mov-rcx-from-local (native-value-window-spill-offset frame-base-slot-count 20)))
        stack43 (concat-byte-vectors stack42 (emit-mov-third-stack-from-rcx))
        stack44 (concat-byte-vectors
                  stack43
                  (emit-mov-rcx-from-local (native-value-window-spill-offset frame-base-slot-count 21)))
        stack45 (concat-byte-vectors stack44 (emit-mov-second-stack-from-rcx))
        stack-setup (concat-byte-vectors
                      stack45
                      (emit-mov-r9-from-local (native-value-window-spill-offset frame-base-slot-count 22)))
        reg0 (concat-byte-vectors
               (emit-mov-top-stack-from-r9)
               (emit-mov-r9-from-local (native-value-window-spill-offset frame-base-slot-count 23)))
        reg1 (concat-byte-vectors reg0 (emit-mov-r8-from-local (native-value-window-spill-offset frame-base-slot-count 24)))
        reg2 (concat-byte-vectors reg1 (emit-mov-rcx-from-local (native-value-window-spill-offset frame-base-slot-count 25)))
        reg3 (concat-byte-vectors reg2 (emit-mov-rdx-from-local (native-value-window-spill-offset frame-base-slot-count 26)))
        reg4 (concat-byte-vectors reg3 (emit-mov-rsi-from-local (native-value-window-spill-offset frame-base-slot-count 27)))
        reg-setup (concat-byte-vectors reg4 (emit-mov-rdi-from-local (native-value-window-spill-offset frame-base-slot-count 28)))
        call-seq (concat-byte-vectors
                   (emit-call-rel32 rel)
                   (emit-add-rsp-imm32 208))]
    (concat-byte-vectors
      (concat-byte-vectors stack-setup reg-setup)
      call-seq)))

(defn emit-thirty-two-arg-call-x86 [rel frame-base-slot-count]
  (let [stack0 (concat-byte-vectors
                 (emit-sub-rsp-imm32 208)
                 (emit-mov-twenty-sixth-stack-from-rax))
        stack1 (concat-byte-vectors stack0 (emit-mov-twenty-fifth-stack-from-rcx))
        stack2 (concat-byte-vectors
                 stack1
                 (emit-mov-rcx-from-local (native-value-window-spill-offset frame-base-slot-count 0)))
        stack3 (concat-byte-vectors stack2 (emit-mov-twenty-fourth-stack-from-rcx))
        stack4 (concat-byte-vectors
                 stack3
                 (emit-mov-rcx-from-local (native-value-window-spill-offset frame-base-slot-count 1)))
        stack5 (concat-byte-vectors stack4 (emit-mov-twenty-third-stack-from-rcx))
        stack6 (concat-byte-vectors
                 stack5
                 (emit-mov-rcx-from-local (native-value-window-spill-offset frame-base-slot-count 2)))
        stack7 (concat-byte-vectors stack6 (emit-mov-twenty-second-stack-from-rcx))
        stack8 (concat-byte-vectors
                 stack7
                 (emit-mov-rcx-from-local (native-value-window-spill-offset frame-base-slot-count 3)))
        stack9 (concat-byte-vectors stack8 (emit-mov-twenty-first-stack-from-rcx))
        stack10 (concat-byte-vectors
                  stack9
                  (emit-mov-rcx-from-local (native-value-window-spill-offset frame-base-slot-count 4)))
        stack11 (concat-byte-vectors stack10 (emit-mov-twentieth-stack-from-rcx))
        stack12 (concat-byte-vectors
                  stack11
                  (emit-mov-rcx-from-local (native-value-window-spill-offset frame-base-slot-count 5)))
        stack13 (concat-byte-vectors stack12 (emit-mov-nineteenth-stack-from-rcx))
        stack14 (concat-byte-vectors
                  stack13
                  (emit-mov-rcx-from-local (native-value-window-spill-offset frame-base-slot-count 6)))
        stack15 (concat-byte-vectors stack14 (emit-mov-eighteenth-stack-from-rcx))
        stack16 (concat-byte-vectors
                  stack15
                  (emit-mov-rcx-from-local (native-value-window-spill-offset frame-base-slot-count 7)))
        stack17 (concat-byte-vectors stack16 (emit-mov-seventeenth-stack-from-rcx))
        stack18 (concat-byte-vectors
                  stack17
                  (emit-mov-rcx-from-local (native-value-window-spill-offset frame-base-slot-count 8)))
        stack19 (concat-byte-vectors stack18 (emit-mov-sixteenth-stack-from-rcx))
        stack20 (concat-byte-vectors
                  stack19
                  (emit-mov-rcx-from-local (native-value-window-spill-offset frame-base-slot-count 9)))
        stack21 (concat-byte-vectors stack20 (emit-mov-fifteenth-stack-from-rcx))
        stack22 (concat-byte-vectors
                  stack21
                  (emit-mov-rcx-from-local (native-value-window-spill-offset frame-base-slot-count 10)))
        stack23 (concat-byte-vectors stack22 (emit-mov-fourteenth-stack-from-rcx))
        stack24 (concat-byte-vectors
                  stack23
                  (emit-mov-rcx-from-local (native-value-window-spill-offset frame-base-slot-count 11)))
        stack25 (concat-byte-vectors stack24 (emit-mov-thirteenth-stack-from-rcx))
        stack26 (concat-byte-vectors
                  stack25
                  (emit-mov-rcx-from-local (native-value-window-spill-offset frame-base-slot-count 12)))
        stack27 (concat-byte-vectors stack26 (emit-mov-twelfth-stack-from-rcx))
        stack28 (concat-byte-vectors
                  stack27
                  (emit-mov-rcx-from-local (native-value-window-spill-offset frame-base-slot-count 13)))
        stack29 (concat-byte-vectors stack28 (emit-mov-eleventh-stack-from-rcx))
        stack30 (concat-byte-vectors
                  stack29
                  (emit-mov-rcx-from-local (native-value-window-spill-offset frame-base-slot-count 14)))
        stack31 (concat-byte-vectors stack30 (emit-mov-tenth-stack-from-rcx))
        stack32 (concat-byte-vectors
                  stack31
                  (emit-mov-rcx-from-local (native-value-window-spill-offset frame-base-slot-count 15)))
        stack33 (concat-byte-vectors stack32 (emit-mov-ninth-stack-from-rcx))
        stack34 (concat-byte-vectors
                  stack33
                  (emit-mov-rcx-from-local (native-value-window-spill-offset frame-base-slot-count 16)))
        stack35 (concat-byte-vectors stack34 (emit-mov-eighth-stack-from-rcx))
        stack36 (concat-byte-vectors
                  stack35
                  (emit-mov-rcx-from-local (native-value-window-spill-offset frame-base-slot-count 17)))
        stack37 (concat-byte-vectors stack36 (emit-mov-seventh-stack-from-rcx))
        stack38 (concat-byte-vectors
                  stack37
                  (emit-mov-rcx-from-local (native-value-window-spill-offset frame-base-slot-count 18)))
        stack39 (concat-byte-vectors stack38 (emit-mov-sixth-stack-from-rcx))
        stack40 (concat-byte-vectors
                  stack39
                  (emit-mov-rcx-from-local (native-value-window-spill-offset frame-base-slot-count 19)))
        stack41 (concat-byte-vectors stack40 (emit-mov-fifth-stack-from-rcx))
        stack42 (concat-byte-vectors
                  stack41
                  (emit-mov-rcx-from-local (native-value-window-spill-offset frame-base-slot-count 20)))
        stack43 (concat-byte-vectors stack42 (emit-mov-fourth-stack-from-rcx))
        stack44 (concat-byte-vectors
                  stack43
                  (emit-mov-rcx-from-local (native-value-window-spill-offset frame-base-slot-count 21)))
        stack45 (concat-byte-vectors stack44 (emit-mov-third-stack-from-rcx))
        stack46 (concat-byte-vectors
                  stack45
                  (emit-mov-rcx-from-local (native-value-window-spill-offset frame-base-slot-count 22)))
        stack47 (concat-byte-vectors stack46 (emit-mov-second-stack-from-rcx))
        stack-setup (concat-byte-vectors
                      stack47
                      (emit-mov-r9-from-local (native-value-window-spill-offset frame-base-slot-count 23)))
        reg0 (concat-byte-vectors
               (emit-mov-top-stack-from-r9)
               (emit-mov-r9-from-local (native-value-window-spill-offset frame-base-slot-count 24)))
        reg1 (concat-byte-vectors reg0 (emit-mov-r8-from-local (native-value-window-spill-offset frame-base-slot-count 25)))
        reg2 (concat-byte-vectors reg1 (emit-mov-rcx-from-local (native-value-window-spill-offset frame-base-slot-count 26)))
        reg3 (concat-byte-vectors reg2 (emit-mov-rdx-from-local (native-value-window-spill-offset frame-base-slot-count 27)))
        reg4 (concat-byte-vectors reg3 (emit-mov-rsi-from-local (native-value-window-spill-offset frame-base-slot-count 28)))
        reg-setup (concat-byte-vectors reg4 (emit-mov-rdi-from-local (native-value-window-spill-offset frame-base-slot-count 29)))
        call-seq (concat-byte-vectors
                   (emit-call-rel32 rel)
                   (emit-add-rsp-imm32 208))]
    (concat-byte-vectors
      (concat-byte-vectors stack-setup reg-setup)
      call-seq)))

(defn emit-thirty-three-arg-call-x86 [rel frame-base-slot-count]
  (let [stack0 (concat-byte-vectors
                 (emit-sub-rsp-imm32 224)
                 (emit-mov-twenty-seventh-stack-from-rax))
        stack1 (concat-byte-vectors stack0 (emit-mov-twenty-sixth-stack-from-rcx))
        stack2 (concat-byte-vectors
                 stack1
                 (emit-mov-rcx-from-local (native-value-window-spill-offset frame-base-slot-count 0)))
        stack3 (concat-byte-vectors stack2 (emit-mov-twenty-fifth-stack-from-rcx))
        stack4 (concat-byte-vectors
                 stack3
                 (emit-mov-rcx-from-local (native-value-window-spill-offset frame-base-slot-count 1)))
        stack5 (concat-byte-vectors stack4 (emit-mov-twenty-fourth-stack-from-rcx))
        stack6 (concat-byte-vectors
                 stack5
                 (emit-mov-rcx-from-local (native-value-window-spill-offset frame-base-slot-count 2)))
        stack7 (concat-byte-vectors stack6 (emit-mov-twenty-third-stack-from-rcx))
        stack8 (concat-byte-vectors
                 stack7
                 (emit-mov-rcx-from-local (native-value-window-spill-offset frame-base-slot-count 3)))
        stack9 (concat-byte-vectors stack8 (emit-mov-twenty-second-stack-from-rcx))
        stack10 (concat-byte-vectors
                  stack9
                  (emit-mov-rcx-from-local (native-value-window-spill-offset frame-base-slot-count 4)))
        stack11 (concat-byte-vectors stack10 (emit-mov-twenty-first-stack-from-rcx))
        stack12 (concat-byte-vectors
                  stack11
                  (emit-mov-rcx-from-local (native-value-window-spill-offset frame-base-slot-count 5)))
        stack13 (concat-byte-vectors stack12 (emit-mov-twentieth-stack-from-rcx))
        stack14 (concat-byte-vectors
                  stack13
                  (emit-mov-rcx-from-local (native-value-window-spill-offset frame-base-slot-count 6)))
        stack15 (concat-byte-vectors stack14 (emit-mov-nineteenth-stack-from-rcx))
        stack16 (concat-byte-vectors
                  stack15
                  (emit-mov-rcx-from-local (native-value-window-spill-offset frame-base-slot-count 7)))
        stack17 (concat-byte-vectors stack16 (emit-mov-eighteenth-stack-from-rcx))
        stack18 (concat-byte-vectors
                  stack17
                  (emit-mov-rcx-from-local (native-value-window-spill-offset frame-base-slot-count 8)))
        stack19 (concat-byte-vectors stack18 (emit-mov-seventeenth-stack-from-rcx))
        stack20 (concat-byte-vectors
                  stack19
                  (emit-mov-rcx-from-local (native-value-window-spill-offset frame-base-slot-count 9)))
        stack21 (concat-byte-vectors stack20 (emit-mov-sixteenth-stack-from-rcx))
        stack22 (concat-byte-vectors
                  stack21
                  (emit-mov-rcx-from-local (native-value-window-spill-offset frame-base-slot-count 10)))
        stack23 (concat-byte-vectors stack22 (emit-mov-fifteenth-stack-from-rcx))
        stack24 (concat-byte-vectors
                  stack23
                  (emit-mov-rcx-from-local (native-value-window-spill-offset frame-base-slot-count 11)))
        stack25 (concat-byte-vectors stack24 (emit-mov-fourteenth-stack-from-rcx))
        stack26 (concat-byte-vectors
                  stack25
                  (emit-mov-rcx-from-local (native-value-window-spill-offset frame-base-slot-count 12)))
        stack27 (concat-byte-vectors stack26 (emit-mov-thirteenth-stack-from-rcx))
        stack28 (concat-byte-vectors
                  stack27
                  (emit-mov-rcx-from-local (native-value-window-spill-offset frame-base-slot-count 13)))
        stack29 (concat-byte-vectors stack28 (emit-mov-twelfth-stack-from-rcx))
        stack30 (concat-byte-vectors
                  stack29
                  (emit-mov-rcx-from-local (native-value-window-spill-offset frame-base-slot-count 14)))
        stack31 (concat-byte-vectors stack30 (emit-mov-eleventh-stack-from-rcx))
        stack32 (concat-byte-vectors
                  stack31
                  (emit-mov-rcx-from-local (native-value-window-spill-offset frame-base-slot-count 15)))
        stack33 (concat-byte-vectors stack32 (emit-mov-tenth-stack-from-rcx))
        stack34 (concat-byte-vectors
                  stack33
                  (emit-mov-rcx-from-local (native-value-window-spill-offset frame-base-slot-count 16)))
        stack35 (concat-byte-vectors stack34 (emit-mov-ninth-stack-from-rcx))
        stack36 (concat-byte-vectors
                  stack35
                  (emit-mov-rcx-from-local (native-value-window-spill-offset frame-base-slot-count 17)))
        stack37 (concat-byte-vectors stack36 (emit-mov-eighth-stack-from-rcx))
        stack38 (concat-byte-vectors
                  stack37
                  (emit-mov-rcx-from-local (native-value-window-spill-offset frame-base-slot-count 18)))
        stack39 (concat-byte-vectors stack38 (emit-mov-seventh-stack-from-rcx))
        stack40 (concat-byte-vectors
                  stack39
                  (emit-mov-rcx-from-local (native-value-window-spill-offset frame-base-slot-count 19)))
        stack41 (concat-byte-vectors stack40 (emit-mov-sixth-stack-from-rcx))
        stack42 (concat-byte-vectors
                  stack41
                  (emit-mov-rcx-from-local (native-value-window-spill-offset frame-base-slot-count 20)))
        stack43 (concat-byte-vectors stack42 (emit-mov-fifth-stack-from-rcx))
        stack44 (concat-byte-vectors
                  stack43
                  (emit-mov-rcx-from-local (native-value-window-spill-offset frame-base-slot-count 21)))
        stack45 (concat-byte-vectors stack44 (emit-mov-fourth-stack-from-rcx))
        stack46 (concat-byte-vectors
                  stack45
                  (emit-mov-rcx-from-local (native-value-window-spill-offset frame-base-slot-count 22)))
        stack47 (concat-byte-vectors stack46 (emit-mov-third-stack-from-rcx))
        stack48 (concat-byte-vectors
                  stack47
                  (emit-mov-rcx-from-local (native-value-window-spill-offset frame-base-slot-count 23)))
        stack49 (concat-byte-vectors stack48 (emit-mov-second-stack-from-rcx))
        stack-setup (concat-byte-vectors
                      stack49
                      (emit-mov-r9-from-local (native-value-window-spill-offset frame-base-slot-count 24)))
        reg0 (concat-byte-vectors
               (emit-mov-top-stack-from-r9)
               (emit-mov-r9-from-local (native-value-window-spill-offset frame-base-slot-count 25)))
        reg1 (concat-byte-vectors reg0 (emit-mov-r8-from-local (native-value-window-spill-offset frame-base-slot-count 26)))
        reg2 (concat-byte-vectors reg1 (emit-mov-rcx-from-local (native-value-window-spill-offset frame-base-slot-count 27)))
        reg3 (concat-byte-vectors reg2 (emit-mov-rdx-from-local (native-value-window-spill-offset frame-base-slot-count 28)))
        reg4 (concat-byte-vectors reg3 (emit-mov-rsi-from-local (native-value-window-spill-offset frame-base-slot-count 29)))
        reg-setup (concat-byte-vectors reg4 (emit-mov-rdi-from-local (native-value-window-spill-offset frame-base-slot-count 30)))
        call-seq (concat-byte-vectors
                   (emit-call-rel32 rel)
                   (emit-add-rsp-imm32 224))]
    (concat-byte-vectors
      (concat-byte-vectors stack-setup reg-setup)
      call-seq)))

(defn emit-thirty-four-arg-call-x86 [rel frame-base-slot-count]
  (let [stack0 (concat-byte-vectors
                 (emit-sub-rsp-imm32 224)
                 (emit-mov-twenty-eighth-stack-from-rax))
        stack1 (concat-byte-vectors stack0 (emit-mov-twenty-seventh-stack-from-rcx))
        stack2 (concat-byte-vectors
                 stack1
                 (emit-mov-rcx-from-local (native-value-window-spill-offset frame-base-slot-count 0)))
        stack3 (concat-byte-vectors stack2 (emit-mov-twenty-sixth-stack-from-rcx))
        stack4 (concat-byte-vectors
                 stack3
                 (emit-mov-rcx-from-local (native-value-window-spill-offset frame-base-slot-count 1)))
        stack5 (concat-byte-vectors stack4 (emit-mov-twenty-fifth-stack-from-rcx))
        stack6 (concat-byte-vectors
                 stack5
                 (emit-mov-rcx-from-local (native-value-window-spill-offset frame-base-slot-count 2)))
        stack7 (concat-byte-vectors stack6 (emit-mov-twenty-fourth-stack-from-rcx))
        stack8 (concat-byte-vectors
                 stack7
                 (emit-mov-rcx-from-local (native-value-window-spill-offset frame-base-slot-count 3)))
        stack9 (concat-byte-vectors stack8 (emit-mov-twenty-third-stack-from-rcx))
        stack10 (concat-byte-vectors
                  stack9
                  (emit-mov-rcx-from-local (native-value-window-spill-offset frame-base-slot-count 4)))
        stack11 (concat-byte-vectors stack10 (emit-mov-twenty-second-stack-from-rcx))
        stack12 (concat-byte-vectors
                  stack11
                  (emit-mov-rcx-from-local (native-value-window-spill-offset frame-base-slot-count 5)))
        stack13 (concat-byte-vectors stack12 (emit-mov-twenty-first-stack-from-rcx))
        stack14 (concat-byte-vectors
                  stack13
                  (emit-mov-rcx-from-local (native-value-window-spill-offset frame-base-slot-count 6)))
        stack15 (concat-byte-vectors stack14 (emit-mov-twentieth-stack-from-rcx))
        stack16 (concat-byte-vectors
                  stack15
                  (emit-mov-rcx-from-local (native-value-window-spill-offset frame-base-slot-count 7)))
        stack17 (concat-byte-vectors stack16 (emit-mov-nineteenth-stack-from-rcx))
        stack18 (concat-byte-vectors
                  stack17
                  (emit-mov-rcx-from-local (native-value-window-spill-offset frame-base-slot-count 8)))
        stack19 (concat-byte-vectors stack18 (emit-mov-eighteenth-stack-from-rcx))
        stack20 (concat-byte-vectors
                  stack19
                  (emit-mov-rcx-from-local (native-value-window-spill-offset frame-base-slot-count 9)))
        stack21 (concat-byte-vectors stack20 (emit-mov-seventeenth-stack-from-rcx))
        stack22 (concat-byte-vectors
                  stack21
                  (emit-mov-rcx-from-local (native-value-window-spill-offset frame-base-slot-count 10)))
        stack23 (concat-byte-vectors stack22 (emit-mov-sixteenth-stack-from-rcx))
        stack24 (concat-byte-vectors
                  stack23
                  (emit-mov-rcx-from-local (native-value-window-spill-offset frame-base-slot-count 11)))
        stack25 (concat-byte-vectors stack24 (emit-mov-fifteenth-stack-from-rcx))
        stack26 (concat-byte-vectors
                  stack25
                  (emit-mov-rcx-from-local (native-value-window-spill-offset frame-base-slot-count 12)))
        stack27 (concat-byte-vectors stack26 (emit-mov-fourteenth-stack-from-rcx))
        stack28 (concat-byte-vectors
                  stack27
                  (emit-mov-rcx-from-local (native-value-window-spill-offset frame-base-slot-count 13)))
        stack29 (concat-byte-vectors stack28 (emit-mov-thirteenth-stack-from-rcx))
        stack30 (concat-byte-vectors
                  stack29
                  (emit-mov-rcx-from-local (native-value-window-spill-offset frame-base-slot-count 14)))
        stack31 (concat-byte-vectors stack30 (emit-mov-twelfth-stack-from-rcx))
        stack32 (concat-byte-vectors
                  stack31
                  (emit-mov-rcx-from-local (native-value-window-spill-offset frame-base-slot-count 15)))
        stack33 (concat-byte-vectors stack32 (emit-mov-eleventh-stack-from-rcx))
        stack34 (concat-byte-vectors
                  stack33
                  (emit-mov-rcx-from-local (native-value-window-spill-offset frame-base-slot-count 16)))
        stack35 (concat-byte-vectors stack34 (emit-mov-tenth-stack-from-rcx))
        stack36 (concat-byte-vectors
                  stack35
                  (emit-mov-rcx-from-local (native-value-window-spill-offset frame-base-slot-count 17)))
        stack37 (concat-byte-vectors stack36 (emit-mov-ninth-stack-from-rcx))
        stack38 (concat-byte-vectors
                  stack37
                  (emit-mov-rcx-from-local (native-value-window-spill-offset frame-base-slot-count 18)))
        stack39 (concat-byte-vectors stack38 (emit-mov-eighth-stack-from-rcx))
        stack40 (concat-byte-vectors
                  stack39
                  (emit-mov-rcx-from-local (native-value-window-spill-offset frame-base-slot-count 19)))
        stack41 (concat-byte-vectors stack40 (emit-mov-seventh-stack-from-rcx))
        stack42 (concat-byte-vectors
                  stack41
                  (emit-mov-rcx-from-local (native-value-window-spill-offset frame-base-slot-count 20)))
        stack43 (concat-byte-vectors stack42 (emit-mov-sixth-stack-from-rcx))
        stack44 (concat-byte-vectors
                  stack43
                  (emit-mov-rcx-from-local (native-value-window-spill-offset frame-base-slot-count 21)))
        stack45 (concat-byte-vectors stack44 (emit-mov-fifth-stack-from-rcx))
        stack46 (concat-byte-vectors
                  stack45
                  (emit-mov-rcx-from-local (native-value-window-spill-offset frame-base-slot-count 22)))
        stack47 (concat-byte-vectors stack46 (emit-mov-fourth-stack-from-rcx))
        stack48 (concat-byte-vectors
                  stack47
                  (emit-mov-rcx-from-local (native-value-window-spill-offset frame-base-slot-count 23)))
        stack49 (concat-byte-vectors stack48 (emit-mov-third-stack-from-rcx))
        stack50 (concat-byte-vectors
                  stack49
                  (emit-mov-rcx-from-local (native-value-window-spill-offset frame-base-slot-count 24)))
        stack51 (concat-byte-vectors stack50 (emit-mov-second-stack-from-rcx))
        stack-setup (concat-byte-vectors
                      stack51
                      (emit-mov-r9-from-local (native-value-window-spill-offset frame-base-slot-count 25)))
        reg0 (concat-byte-vectors
               (emit-mov-top-stack-from-r9)
               (emit-mov-r9-from-local (native-value-window-spill-offset frame-base-slot-count 26)))
        reg1 (concat-byte-vectors reg0 (emit-mov-r8-from-local (native-value-window-spill-offset frame-base-slot-count 27)))
        reg2 (concat-byte-vectors reg1 (emit-mov-rcx-from-local (native-value-window-spill-offset frame-base-slot-count 28)))
        reg3 (concat-byte-vectors reg2 (emit-mov-rdx-from-local (native-value-window-spill-offset frame-base-slot-count 29)))
        reg4 (concat-byte-vectors reg3 (emit-mov-rsi-from-local (native-value-window-spill-offset frame-base-slot-count 30)))
        reg-setup (concat-byte-vectors reg4 (emit-mov-rdi-from-local (native-value-window-spill-offset frame-base-slot-count 31)))
        call-seq (concat-byte-vectors
                   (emit-call-rel32 rel)
                   (emit-add-rsp-imm32 224))]
    (concat-byte-vectors
      (concat-byte-vectors stack-setup reg-setup)
      call-seq)))

(defn emit-thirty-five-arg-call-x86 [rel frame-base-slot-count]
  (let [stack0 (concat-byte-vectors
                 (emit-sub-rsp-imm32 240)
                 (emit-mov-twenty-ninth-stack-from-rax))
        stack1 (concat-byte-vectors stack0 (emit-mov-twenty-eighth-stack-from-rcx))
        stack2 (concat-byte-vectors
                 stack1
                 (emit-mov-rcx-from-local (native-value-window-spill-offset frame-base-slot-count 0)))
        stack3 (concat-byte-vectors stack2 (emit-mov-twenty-seventh-stack-from-rcx))
        stack4 (concat-byte-vectors
                 stack3
                 (emit-mov-rcx-from-local (native-value-window-spill-offset frame-base-slot-count 1)))
        stack5 (concat-byte-vectors stack4 (emit-mov-twenty-sixth-stack-from-rcx))
        stack6 (concat-byte-vectors
                 stack5
                 (emit-mov-rcx-from-local (native-value-window-spill-offset frame-base-slot-count 2)))
        stack7 (concat-byte-vectors stack6 (emit-mov-twenty-fifth-stack-from-rcx))
        stack8 (concat-byte-vectors
                 stack7
                 (emit-mov-rcx-from-local (native-value-window-spill-offset frame-base-slot-count 3)))
        stack9 (concat-byte-vectors stack8 (emit-mov-twenty-fourth-stack-from-rcx))
        stack10 (concat-byte-vectors
                  stack9
                  (emit-mov-rcx-from-local (native-value-window-spill-offset frame-base-slot-count 4)))
        stack11 (concat-byte-vectors stack10 (emit-mov-twenty-third-stack-from-rcx))
        stack12 (concat-byte-vectors
                  stack11
                  (emit-mov-rcx-from-local (native-value-window-spill-offset frame-base-slot-count 5)))
        stack13 (concat-byte-vectors stack12 (emit-mov-twenty-second-stack-from-rcx))
        stack14 (concat-byte-vectors
                  stack13
                  (emit-mov-rcx-from-local (native-value-window-spill-offset frame-base-slot-count 6)))
        stack15 (concat-byte-vectors stack14 (emit-mov-twenty-first-stack-from-rcx))
        stack16 (concat-byte-vectors
                  stack15
                  (emit-mov-rcx-from-local (native-value-window-spill-offset frame-base-slot-count 7)))
        stack17 (concat-byte-vectors stack16 (emit-mov-twentieth-stack-from-rcx))
        stack18 (concat-byte-vectors
                  stack17
                  (emit-mov-rcx-from-local (native-value-window-spill-offset frame-base-slot-count 8)))
        stack19 (concat-byte-vectors stack18 (emit-mov-nineteenth-stack-from-rcx))
        stack20 (concat-byte-vectors
                  stack19
                  (emit-mov-rcx-from-local (native-value-window-spill-offset frame-base-slot-count 9)))
        stack21 (concat-byte-vectors stack20 (emit-mov-eighteenth-stack-from-rcx))
        stack22 (concat-byte-vectors
                  stack21
                  (emit-mov-rcx-from-local (native-value-window-spill-offset frame-base-slot-count 10)))
        stack23 (concat-byte-vectors stack22 (emit-mov-seventeenth-stack-from-rcx))
        stack24 (concat-byte-vectors
                  stack23
                  (emit-mov-rcx-from-local (native-value-window-spill-offset frame-base-slot-count 11)))
        stack25 (concat-byte-vectors stack24 (emit-mov-sixteenth-stack-from-rcx))
        stack26 (concat-byte-vectors
                  stack25
                  (emit-mov-rcx-from-local (native-value-window-spill-offset frame-base-slot-count 12)))
        stack27 (concat-byte-vectors stack26 (emit-mov-fifteenth-stack-from-rcx))
        stack28 (concat-byte-vectors
                  stack27
                  (emit-mov-rcx-from-local (native-value-window-spill-offset frame-base-slot-count 13)))
        stack29 (concat-byte-vectors stack28 (emit-mov-fourteenth-stack-from-rcx))
        stack30 (concat-byte-vectors
                  stack29
                  (emit-mov-rcx-from-local (native-value-window-spill-offset frame-base-slot-count 14)))
        stack31 (concat-byte-vectors stack30 (emit-mov-thirteenth-stack-from-rcx))
        stack32 (concat-byte-vectors
                  stack31
                  (emit-mov-rcx-from-local (native-value-window-spill-offset frame-base-slot-count 15)))
        stack33 (concat-byte-vectors stack32 (emit-mov-twelfth-stack-from-rcx))
        stack34 (concat-byte-vectors
                  stack33
                  (emit-mov-rcx-from-local (native-value-window-spill-offset frame-base-slot-count 16)))
        stack35 (concat-byte-vectors stack34 (emit-mov-eleventh-stack-from-rcx))
        stack36 (concat-byte-vectors
                  stack35
                  (emit-mov-rcx-from-local (native-value-window-spill-offset frame-base-slot-count 17)))
        stack37 (concat-byte-vectors stack36 (emit-mov-tenth-stack-from-rcx))
        stack38 (concat-byte-vectors
                  stack37
                  (emit-mov-rcx-from-local (native-value-window-spill-offset frame-base-slot-count 18)))
        stack39 (concat-byte-vectors stack38 (emit-mov-ninth-stack-from-rcx))
        stack40 (concat-byte-vectors
                  stack39
                  (emit-mov-rcx-from-local (native-value-window-spill-offset frame-base-slot-count 19)))
        stack41 (concat-byte-vectors stack40 (emit-mov-eighth-stack-from-rcx))
        stack42 (concat-byte-vectors
                  stack41
                  (emit-mov-rcx-from-local (native-value-window-spill-offset frame-base-slot-count 20)))
        stack43 (concat-byte-vectors stack42 (emit-mov-seventh-stack-from-rcx))
        stack44 (concat-byte-vectors
                  stack43
                  (emit-mov-rcx-from-local (native-value-window-spill-offset frame-base-slot-count 21)))
        stack45 (concat-byte-vectors stack44 (emit-mov-sixth-stack-from-rcx))
        stack46 (concat-byte-vectors
                  stack45
                  (emit-mov-rcx-from-local (native-value-window-spill-offset frame-base-slot-count 22)))
        stack47 (concat-byte-vectors stack46 (emit-mov-fifth-stack-from-rcx))
        stack48 (concat-byte-vectors
                  stack47
                  (emit-mov-rcx-from-local (native-value-window-spill-offset frame-base-slot-count 23)))
        stack49 (concat-byte-vectors stack48 (emit-mov-fourth-stack-from-rcx))
        stack50 (concat-byte-vectors
                  stack49
                  (emit-mov-rcx-from-local (native-value-window-spill-offset frame-base-slot-count 24)))
        stack51 (concat-byte-vectors stack50 (emit-mov-third-stack-from-rcx))
        stack52 (concat-byte-vectors
                  stack51
                  (emit-mov-rcx-from-local (native-value-window-spill-offset frame-base-slot-count 25)))
        stack53 (concat-byte-vectors stack52 (emit-mov-second-stack-from-rcx))
        stack-setup (concat-byte-vectors
                      stack53
                      (emit-mov-r9-from-local (native-value-window-spill-offset frame-base-slot-count 26)))
        reg0 (concat-byte-vectors
               (emit-mov-top-stack-from-r9)
               (emit-mov-r9-from-local (native-value-window-spill-offset frame-base-slot-count 27)))
        reg1 (concat-byte-vectors reg0 (emit-mov-r8-from-local (native-value-window-spill-offset frame-base-slot-count 28)))
        reg2 (concat-byte-vectors reg1 (emit-mov-rcx-from-local (native-value-window-spill-offset frame-base-slot-count 29)))
        reg3 (concat-byte-vectors reg2 (emit-mov-rdx-from-local (native-value-window-spill-offset frame-base-slot-count 30)))
        reg4 (concat-byte-vectors reg3 (emit-mov-rsi-from-local (native-value-window-spill-offset frame-base-slot-count 31)))
        reg-setup (concat-byte-vectors reg4 (emit-mov-rdi-from-local (native-value-window-spill-offset frame-base-slot-count 32)))
        call-seq (concat-byte-vectors
                   (emit-call-rel32 rel)
                   (emit-add-rsp-imm32 240))]
    (concat-byte-vectors
      (concat-byte-vectors stack-setup reg-setup)
      call-seq)))

(defn emit-thirty-six-arg-call-x86 [rel frame-base-slot-count]
  (let [stack0 (concat-byte-vectors
                 (emit-sub-rsp-imm32 240)
                 (emit-mov-thirtieth-stack-from-rax))
        stack1 (concat-byte-vectors stack0 (emit-mov-twenty-ninth-stack-from-rcx))
        stack2 (concat-byte-vectors
                 stack1
                 (emit-mov-rcx-from-local (native-value-window-spill-offset frame-base-slot-count 0)))
        stack3 (concat-byte-vectors stack2 (emit-mov-twenty-eighth-stack-from-rcx))
        stack4 (concat-byte-vectors
                 stack3
                 (emit-mov-rcx-from-local (native-value-window-spill-offset frame-base-slot-count 1)))
        stack5 (concat-byte-vectors stack4 (emit-mov-twenty-seventh-stack-from-rcx))
        stack6 (concat-byte-vectors
                 stack5
                 (emit-mov-rcx-from-local (native-value-window-spill-offset frame-base-slot-count 2)))
        stack7 (concat-byte-vectors stack6 (emit-mov-twenty-sixth-stack-from-rcx))
        stack8 (concat-byte-vectors
                 stack7
                 (emit-mov-rcx-from-local (native-value-window-spill-offset frame-base-slot-count 3)))
        stack9 (concat-byte-vectors stack8 (emit-mov-twenty-fifth-stack-from-rcx))
        stack10 (concat-byte-vectors
                  stack9
                  (emit-mov-rcx-from-local (native-value-window-spill-offset frame-base-slot-count 4)))
        stack11 (concat-byte-vectors stack10 (emit-mov-twenty-fourth-stack-from-rcx))
        stack12 (concat-byte-vectors
                  stack11
                  (emit-mov-rcx-from-local (native-value-window-spill-offset frame-base-slot-count 5)))
        stack13 (concat-byte-vectors stack12 (emit-mov-twenty-third-stack-from-rcx))
        stack14 (concat-byte-vectors
                  stack13
                  (emit-mov-rcx-from-local (native-value-window-spill-offset frame-base-slot-count 6)))
        stack15 (concat-byte-vectors stack14 (emit-mov-twenty-second-stack-from-rcx))
        stack16 (concat-byte-vectors
                  stack15
                  (emit-mov-rcx-from-local (native-value-window-spill-offset frame-base-slot-count 7)))
        stack17 (concat-byte-vectors stack16 (emit-mov-twenty-first-stack-from-rcx))
        stack18 (concat-byte-vectors
                  stack17
                  (emit-mov-rcx-from-local (native-value-window-spill-offset frame-base-slot-count 8)))
        stack19 (concat-byte-vectors stack18 (emit-mov-twentieth-stack-from-rcx))
        stack20 (concat-byte-vectors
                  stack19
                  (emit-mov-rcx-from-local (native-value-window-spill-offset frame-base-slot-count 9)))
        stack21 (concat-byte-vectors stack20 (emit-mov-nineteenth-stack-from-rcx))
        stack22 (concat-byte-vectors
                  stack21
                  (emit-mov-rcx-from-local (native-value-window-spill-offset frame-base-slot-count 10)))
        stack23 (concat-byte-vectors stack22 (emit-mov-eighteenth-stack-from-rcx))
        stack24 (concat-byte-vectors
                  stack23
                  (emit-mov-rcx-from-local (native-value-window-spill-offset frame-base-slot-count 11)))
        stack25 (concat-byte-vectors stack24 (emit-mov-seventeenth-stack-from-rcx))
        stack26 (concat-byte-vectors
                  stack25
                  (emit-mov-rcx-from-local (native-value-window-spill-offset frame-base-slot-count 12)))
        stack27 (concat-byte-vectors stack26 (emit-mov-sixteenth-stack-from-rcx))
        stack28 (concat-byte-vectors
                  stack27
                  (emit-mov-rcx-from-local (native-value-window-spill-offset frame-base-slot-count 13)))
        stack29 (concat-byte-vectors stack28 (emit-mov-fifteenth-stack-from-rcx))
        stack30 (concat-byte-vectors
                  stack29
                  (emit-mov-rcx-from-local (native-value-window-spill-offset frame-base-slot-count 14)))
        stack31 (concat-byte-vectors stack30 (emit-mov-fourteenth-stack-from-rcx))
        stack32 (concat-byte-vectors
                  stack31
                  (emit-mov-rcx-from-local (native-value-window-spill-offset frame-base-slot-count 15)))
        stack33 (concat-byte-vectors stack32 (emit-mov-thirteenth-stack-from-rcx))
        stack34 (concat-byte-vectors
                  stack33
                  (emit-mov-rcx-from-local (native-value-window-spill-offset frame-base-slot-count 16)))
        stack35 (concat-byte-vectors stack34 (emit-mov-twelfth-stack-from-rcx))
        stack36 (concat-byte-vectors
                  stack35
                  (emit-mov-rcx-from-local (native-value-window-spill-offset frame-base-slot-count 17)))
        stack37 (concat-byte-vectors stack36 (emit-mov-eleventh-stack-from-rcx))
        stack38 (concat-byte-vectors
                  stack37
                  (emit-mov-rcx-from-local (native-value-window-spill-offset frame-base-slot-count 18)))
        stack39 (concat-byte-vectors stack38 (emit-mov-tenth-stack-from-rcx))
        stack40 (concat-byte-vectors
                  stack39
                  (emit-mov-rcx-from-local (native-value-window-spill-offset frame-base-slot-count 19)))
        stack41 (concat-byte-vectors stack40 (emit-mov-ninth-stack-from-rcx))
        stack42 (concat-byte-vectors
                  stack41
                  (emit-mov-rcx-from-local (native-value-window-spill-offset frame-base-slot-count 20)))
        stack43 (concat-byte-vectors stack42 (emit-mov-eighth-stack-from-rcx))
        stack44 (concat-byte-vectors
                  stack43
                  (emit-mov-rcx-from-local (native-value-window-spill-offset frame-base-slot-count 21)))
        stack45 (concat-byte-vectors stack44 (emit-mov-seventh-stack-from-rcx))
        stack46 (concat-byte-vectors
                  stack45
                  (emit-mov-rcx-from-local (native-value-window-spill-offset frame-base-slot-count 22)))
        stack47 (concat-byte-vectors stack46 (emit-mov-sixth-stack-from-rcx))
        stack48 (concat-byte-vectors
                  stack47
                  (emit-mov-rcx-from-local (native-value-window-spill-offset frame-base-slot-count 23)))
        stack49 (concat-byte-vectors stack48 (emit-mov-fifth-stack-from-rcx))
        stack50 (concat-byte-vectors
                  stack49
                  (emit-mov-rcx-from-local (native-value-window-spill-offset frame-base-slot-count 24)))
        stack51 (concat-byte-vectors stack50 (emit-mov-fourth-stack-from-rcx))
        stack52 (concat-byte-vectors
                  stack51
                  (emit-mov-rcx-from-local (native-value-window-spill-offset frame-base-slot-count 25)))
        stack53 (concat-byte-vectors stack52 (emit-mov-third-stack-from-rcx))
        stack54 (concat-byte-vectors
                  stack53
                  (emit-mov-rcx-from-local (native-value-window-spill-offset frame-base-slot-count 26)))
        stack55 (concat-byte-vectors stack54 (emit-mov-second-stack-from-rcx))
        stack-setup (concat-byte-vectors
                      stack55
                      (emit-mov-r9-from-local (native-value-window-spill-offset frame-base-slot-count 27)))
        reg0 (concat-byte-vectors
               (emit-mov-top-stack-from-r9)
               (emit-mov-r9-from-local (native-value-window-spill-offset frame-base-slot-count 28)))
        reg1 (concat-byte-vectors reg0 (emit-mov-r8-from-local (native-value-window-spill-offset frame-base-slot-count 29)))
        reg2 (concat-byte-vectors reg1 (emit-mov-rcx-from-local (native-value-window-spill-offset frame-base-slot-count 30)))
        reg3 (concat-byte-vectors reg2 (emit-mov-rdx-from-local (native-value-window-spill-offset frame-base-slot-count 31)))
        reg4 (concat-byte-vectors reg3 (emit-mov-rsi-from-local (native-value-window-spill-offset frame-base-slot-count 32)))
        reg-setup (concat-byte-vectors reg4 (emit-mov-rdi-from-local (native-value-window-spill-offset frame-base-slot-count 33)))
        call-seq (concat-byte-vectors
                   (emit-call-rel32 rel)
                   (emit-add-rsp-imm32 240))]
    (concat-byte-vectors
      (concat-byte-vectors stack-setup reg-setup)
      call-seq)))

(defn emit-thirty-seven-arg-call-x86 [rel frame-base-slot-count]
  (let [stack0 (concat-byte-vectors
                 (emit-sub-rsp-imm32 248)
                 (emit-mov-thirty-first-stack-from-rax))
        stack1 (concat-byte-vectors stack0 (emit-mov-thirtieth-stack-from-rcx))
        stack2 (concat-byte-vectors
                 stack1
                 (emit-mov-rcx-from-local (native-value-window-spill-offset frame-base-slot-count 0)))
        stack3 (concat-byte-vectors stack2 (emit-mov-twenty-ninth-stack-from-rcx))
        stack4 (concat-byte-vectors
                 stack3
                 (emit-mov-rcx-from-local (native-value-window-spill-offset frame-base-slot-count 1)))
        stack5 (concat-byte-vectors stack4 (emit-mov-twenty-eighth-stack-from-rcx))
        stack6 (concat-byte-vectors
                 stack5
                 (emit-mov-rcx-from-local (native-value-window-spill-offset frame-base-slot-count 2)))
        stack7 (concat-byte-vectors stack6 (emit-mov-twenty-seventh-stack-from-rcx))
        stack8 (concat-byte-vectors
                 stack7
                 (emit-mov-rcx-from-local (native-value-window-spill-offset frame-base-slot-count 3)))
        stack9 (concat-byte-vectors stack8 (emit-mov-twenty-sixth-stack-from-rcx))
        stack10 (concat-byte-vectors
                  stack9
                  (emit-mov-rcx-from-local (native-value-window-spill-offset frame-base-slot-count 4)))
        stack11 (concat-byte-vectors stack10 (emit-mov-twenty-fifth-stack-from-rcx))
        stack12 (concat-byte-vectors
                  stack11
                  (emit-mov-rcx-from-local (native-value-window-spill-offset frame-base-slot-count 5)))
        stack13 (concat-byte-vectors stack12 (emit-mov-twenty-fourth-stack-from-rcx))
        stack14 (concat-byte-vectors
                  stack13
                  (emit-mov-rcx-from-local (native-value-window-spill-offset frame-base-slot-count 6)))
        stack15 (concat-byte-vectors stack14 (emit-mov-twenty-third-stack-from-rcx))
        stack16 (concat-byte-vectors
                  stack15
                  (emit-mov-rcx-from-local (native-value-window-spill-offset frame-base-slot-count 7)))
        stack17 (concat-byte-vectors stack16 (emit-mov-twenty-second-stack-from-rcx))
        stack18 (concat-byte-vectors
                  stack17
                  (emit-mov-rcx-from-local (native-value-window-spill-offset frame-base-slot-count 8)))
        stack19 (concat-byte-vectors stack18 (emit-mov-twenty-first-stack-from-rcx))
        stack20 (concat-byte-vectors
                  stack19
                  (emit-mov-rcx-from-local (native-value-window-spill-offset frame-base-slot-count 9)))
        stack21 (concat-byte-vectors stack20 (emit-mov-twentieth-stack-from-rcx))
        stack22 (concat-byte-vectors
                  stack21
                  (emit-mov-rcx-from-local (native-value-window-spill-offset frame-base-slot-count 10)))
        stack23 (concat-byte-vectors stack22 (emit-mov-nineteenth-stack-from-rcx))
        stack24 (concat-byte-vectors
                  stack23
                  (emit-mov-rcx-from-local (native-value-window-spill-offset frame-base-slot-count 11)))
        stack25 (concat-byte-vectors stack24 (emit-mov-eighteenth-stack-from-rcx))
        stack26 (concat-byte-vectors
                  stack25
                  (emit-mov-rcx-from-local (native-value-window-spill-offset frame-base-slot-count 12)))
        stack27 (concat-byte-vectors stack26 (emit-mov-seventeenth-stack-from-rcx))
        stack28 (concat-byte-vectors
                  stack27
                  (emit-mov-rcx-from-local (native-value-window-spill-offset frame-base-slot-count 13)))
        stack29 (concat-byte-vectors stack28 (emit-mov-sixteenth-stack-from-rcx))
        stack30 (concat-byte-vectors
                  stack29
                  (emit-mov-rcx-from-local (native-value-window-spill-offset frame-base-slot-count 14)))
        stack31 (concat-byte-vectors stack30 (emit-mov-fifteenth-stack-from-rcx))
        stack32 (concat-byte-vectors
                  stack31
                  (emit-mov-rcx-from-local (native-value-window-spill-offset frame-base-slot-count 15)))
        stack33 (concat-byte-vectors stack32 (emit-mov-fourteenth-stack-from-rcx))
        stack34 (concat-byte-vectors
                  stack33
                  (emit-mov-rcx-from-local (native-value-window-spill-offset frame-base-slot-count 16)))
        stack35 (concat-byte-vectors stack34 (emit-mov-thirteenth-stack-from-rcx))
        stack36 (concat-byte-vectors
                  stack35
                  (emit-mov-rcx-from-local (native-value-window-spill-offset frame-base-slot-count 17)))
        stack37 (concat-byte-vectors stack36 (emit-mov-twelfth-stack-from-rcx))
        stack38 (concat-byte-vectors
                  stack37
                  (emit-mov-rcx-from-local (native-value-window-spill-offset frame-base-slot-count 18)))
        stack39 (concat-byte-vectors stack38 (emit-mov-eleventh-stack-from-rcx))
        stack40 (concat-byte-vectors
                  stack39
                  (emit-mov-rcx-from-local (native-value-window-spill-offset frame-base-slot-count 19)))
        stack41 (concat-byte-vectors stack40 (emit-mov-tenth-stack-from-rcx))
        stack42 (concat-byte-vectors
                  stack41
                  (emit-mov-rcx-from-local (native-value-window-spill-offset frame-base-slot-count 20)))
        stack43 (concat-byte-vectors stack42 (emit-mov-ninth-stack-from-rcx))
        stack44 (concat-byte-vectors
                  stack43
                  (emit-mov-rcx-from-local (native-value-window-spill-offset frame-base-slot-count 21)))
        stack45 (concat-byte-vectors stack44 (emit-mov-eighth-stack-from-rcx))
        stack46 (concat-byte-vectors
                  stack45
                  (emit-mov-rcx-from-local (native-value-window-spill-offset frame-base-slot-count 22)))
        stack47 (concat-byte-vectors stack46 (emit-mov-seventh-stack-from-rcx))
        stack48 (concat-byte-vectors
                  stack47
                  (emit-mov-rcx-from-local (native-value-window-spill-offset frame-base-slot-count 23)))
        stack49 (concat-byte-vectors stack48 (emit-mov-sixth-stack-from-rcx))
        stack50 (concat-byte-vectors
                  stack49
                  (emit-mov-rcx-from-local (native-value-window-spill-offset frame-base-slot-count 24)))
        stack51 (concat-byte-vectors stack50 (emit-mov-fifth-stack-from-rcx))
        stack52 (concat-byte-vectors
                  stack51
                  (emit-mov-rcx-from-local (native-value-window-spill-offset frame-base-slot-count 25)))
        stack53 (concat-byte-vectors stack52 (emit-mov-fourth-stack-from-rcx))
        stack54 (concat-byte-vectors
                  stack53
                  (emit-mov-rcx-from-local (native-value-window-spill-offset frame-base-slot-count 26)))
        stack55 (concat-byte-vectors stack54 (emit-mov-third-stack-from-rcx))
        stack56 (concat-byte-vectors
                  stack55
                  (emit-mov-rcx-from-local (native-value-window-spill-offset frame-base-slot-count 27)))
        stack57 (concat-byte-vectors stack56 (emit-mov-second-stack-from-rcx))
        stack-setup (concat-byte-vectors
                      stack57
                      (emit-mov-r9-from-local (native-value-window-spill-offset frame-base-slot-count 28)))
        reg0 (concat-byte-vectors
               (emit-mov-top-stack-from-r9)
               (emit-mov-r9-from-local (native-value-window-spill-offset frame-base-slot-count 29)))
        reg1 (concat-byte-vectors reg0 (emit-mov-r8-from-local (native-value-window-spill-offset frame-base-slot-count 30)))
        reg2 (concat-byte-vectors reg1 (emit-mov-rcx-from-local (native-value-window-spill-offset frame-base-slot-count 31)))
        reg3 (concat-byte-vectors reg2 (emit-mov-rdx-from-local (native-value-window-spill-offset frame-base-slot-count 32)))
        reg4 (concat-byte-vectors reg3 (emit-mov-rsi-from-local (native-value-window-spill-offset frame-base-slot-count 33)))
        reg-setup (concat-byte-vectors reg4 (emit-mov-rdi-from-local (native-value-window-spill-offset frame-base-slot-count 34)))
        call-seq (concat-byte-vectors
                   (emit-call-rel32 rel)
                   (emit-add-rsp-imm32 248))]
    (concat-byte-vectors
      (concat-byte-vectors stack-setup reg-setup)
      call-seq)))

(defn emit-thirty-eight-arg-call-x86 [rel frame-base-slot-count]
  (let [stack0 (concat-byte-vectors
                 (emit-sub-rsp-imm32 256)
                 (emit-mov-thirty-second-stack-from-rax))
        stack1 (concat-byte-vectors stack0 (emit-mov-thirty-first-stack-from-rcx))
        stack2 (concat-byte-vectors
                 stack1
                 (emit-mov-rcx-from-local (native-value-window-spill-offset frame-base-slot-count 0)))
        stack3 (concat-byte-vectors stack2 (emit-mov-thirtieth-stack-from-rcx))
        stack4 (concat-byte-vectors
                 stack3
                 (emit-mov-rcx-from-local (native-value-window-spill-offset frame-base-slot-count 1)))
        stack5 (concat-byte-vectors stack4 (emit-mov-twenty-ninth-stack-from-rcx))
        stack6 (concat-byte-vectors
                 stack5
                 (emit-mov-rcx-from-local (native-value-window-spill-offset frame-base-slot-count 2)))
        stack7 (concat-byte-vectors stack6 (emit-mov-twenty-eighth-stack-from-rcx))
        stack8 (concat-byte-vectors
                 stack7
                 (emit-mov-rcx-from-local (native-value-window-spill-offset frame-base-slot-count 3)))
        stack9 (concat-byte-vectors stack8 (emit-mov-twenty-seventh-stack-from-rcx))
        stack10 (concat-byte-vectors
                  stack9
                  (emit-mov-rcx-from-local (native-value-window-spill-offset frame-base-slot-count 4)))
        stack11 (concat-byte-vectors stack10 (emit-mov-twenty-sixth-stack-from-rcx))
        stack12 (concat-byte-vectors
                  stack11
                  (emit-mov-rcx-from-local (native-value-window-spill-offset frame-base-slot-count 5)))
        stack13 (concat-byte-vectors stack12 (emit-mov-twenty-fifth-stack-from-rcx))
        stack14 (concat-byte-vectors
                  stack13
                  (emit-mov-rcx-from-local (native-value-window-spill-offset frame-base-slot-count 6)))
        stack15 (concat-byte-vectors stack14 (emit-mov-twenty-fourth-stack-from-rcx))
        stack16 (concat-byte-vectors
                  stack15
                  (emit-mov-rcx-from-local (native-value-window-spill-offset frame-base-slot-count 7)))
        stack17 (concat-byte-vectors stack16 (emit-mov-twenty-third-stack-from-rcx))
        stack18 (concat-byte-vectors
                  stack17
                  (emit-mov-rcx-from-local (native-value-window-spill-offset frame-base-slot-count 8)))
        stack19 (concat-byte-vectors stack18 (emit-mov-twenty-second-stack-from-rcx))
        stack20 (concat-byte-vectors
                  stack19
                  (emit-mov-rcx-from-local (native-value-window-spill-offset frame-base-slot-count 9)))
        stack21 (concat-byte-vectors stack20 (emit-mov-twenty-first-stack-from-rcx))
        stack22 (concat-byte-vectors
                  stack21
                  (emit-mov-rcx-from-local (native-value-window-spill-offset frame-base-slot-count 10)))
        stack23 (concat-byte-vectors stack22 (emit-mov-twentieth-stack-from-rcx))
        stack24 (concat-byte-vectors
                  stack23
                  (emit-mov-rcx-from-local (native-value-window-spill-offset frame-base-slot-count 11)))
        stack25 (concat-byte-vectors stack24 (emit-mov-nineteenth-stack-from-rcx))
        stack26 (concat-byte-vectors
                  stack25
                  (emit-mov-rcx-from-local (native-value-window-spill-offset frame-base-slot-count 12)))
        stack27 (concat-byte-vectors stack26 (emit-mov-eighteenth-stack-from-rcx))
        stack28 (concat-byte-vectors
                  stack27
                  (emit-mov-rcx-from-local (native-value-window-spill-offset frame-base-slot-count 13)))
        stack29 (concat-byte-vectors stack28 (emit-mov-seventeenth-stack-from-rcx))
        stack30 (concat-byte-vectors
                  stack29
                  (emit-mov-rcx-from-local (native-value-window-spill-offset frame-base-slot-count 14)))
        stack31 (concat-byte-vectors stack30 (emit-mov-sixteenth-stack-from-rcx))
        stack32 (concat-byte-vectors
                  stack31
                  (emit-mov-rcx-from-local (native-value-window-spill-offset frame-base-slot-count 15)))
        stack33 (concat-byte-vectors stack32 (emit-mov-fifteenth-stack-from-rcx))
        stack34 (concat-byte-vectors
                  stack33
                  (emit-mov-rcx-from-local (native-value-window-spill-offset frame-base-slot-count 16)))
        stack35 (concat-byte-vectors stack34 (emit-mov-fourteenth-stack-from-rcx))
        stack36 (concat-byte-vectors
                  stack35
                  (emit-mov-rcx-from-local (native-value-window-spill-offset frame-base-slot-count 17)))
        stack37 (concat-byte-vectors stack36 (emit-mov-thirteenth-stack-from-rcx))
        stack38 (concat-byte-vectors
                  stack37
                  (emit-mov-rcx-from-local (native-value-window-spill-offset frame-base-slot-count 18)))
        stack39 (concat-byte-vectors stack38 (emit-mov-twelfth-stack-from-rcx))
        stack40 (concat-byte-vectors
                  stack39
                  (emit-mov-rcx-from-local (native-value-window-spill-offset frame-base-slot-count 19)))
        stack41 (concat-byte-vectors stack40 (emit-mov-eleventh-stack-from-rcx))
        stack42 (concat-byte-vectors
                  stack41
                  (emit-mov-rcx-from-local (native-value-window-spill-offset frame-base-slot-count 20)))
        stack43 (concat-byte-vectors stack42 (emit-mov-tenth-stack-from-rcx))
        stack44 (concat-byte-vectors
                  stack43
                  (emit-mov-rcx-from-local (native-value-window-spill-offset frame-base-slot-count 21)))
        stack45 (concat-byte-vectors stack44 (emit-mov-ninth-stack-from-rcx))
        stack46 (concat-byte-vectors
                  stack45
                  (emit-mov-rcx-from-local (native-value-window-spill-offset frame-base-slot-count 22)))
        stack47 (concat-byte-vectors stack46 (emit-mov-eighth-stack-from-rcx))
        stack48 (concat-byte-vectors
                  stack47
                  (emit-mov-rcx-from-local (native-value-window-spill-offset frame-base-slot-count 23)))
        stack49 (concat-byte-vectors stack48 (emit-mov-seventh-stack-from-rcx))
        stack50 (concat-byte-vectors
                  stack49
                  (emit-mov-rcx-from-local (native-value-window-spill-offset frame-base-slot-count 24)))
        stack51 (concat-byte-vectors stack50 (emit-mov-sixth-stack-from-rcx))
        stack52 (concat-byte-vectors
                  stack51
                  (emit-mov-rcx-from-local (native-value-window-spill-offset frame-base-slot-count 25)))
        stack53 (concat-byte-vectors stack52 (emit-mov-fifth-stack-from-rcx))
        stack54 (concat-byte-vectors
                  stack53
                  (emit-mov-rcx-from-local (native-value-window-spill-offset frame-base-slot-count 26)))
        stack55 (concat-byte-vectors stack54 (emit-mov-fourth-stack-from-rcx))
        stack56 (concat-byte-vectors
                  stack55
                  (emit-mov-rcx-from-local (native-value-window-spill-offset frame-base-slot-count 27)))
        stack57 (concat-byte-vectors stack56 (emit-mov-third-stack-from-rcx))
        stack58 (concat-byte-vectors
                  stack57
                  (emit-mov-rcx-from-local (native-value-window-spill-offset frame-base-slot-count 28)))
        stack59 (concat-byte-vectors stack58 (emit-mov-second-stack-from-rcx))
        stack-setup (concat-byte-vectors
                      stack59
                      (emit-mov-r9-from-local (native-value-window-spill-offset frame-base-slot-count 29)))
        reg0 (concat-byte-vectors
               (emit-mov-top-stack-from-r9)
               (emit-mov-r9-from-local (native-value-window-spill-offset frame-base-slot-count 30)))
        reg1 (concat-byte-vectors reg0 (emit-mov-r8-from-local (native-value-window-spill-offset frame-base-slot-count 31)))
        reg2 (concat-byte-vectors reg1 (emit-mov-rcx-from-local (native-value-window-spill-offset frame-base-slot-count 32)))
        reg3 (concat-byte-vectors reg2 (emit-mov-rdx-from-local (native-value-window-spill-offset frame-base-slot-count 33)))
        reg4 (concat-byte-vectors reg3 (emit-mov-rsi-from-local (native-value-window-spill-offset frame-base-slot-count 34)))
        reg-setup (concat-byte-vectors reg4 (emit-mov-rdi-from-local (native-value-window-spill-offset frame-base-slot-count 35)))
        call-seq (concat-byte-vectors
                   (emit-call-rel32 rel)
                   (emit-add-rsp-imm32 256))]
    (concat-byte-vectors
      (concat-byte-vectors stack-setup reg-setup)
      call-seq)))

(defn emit-thirty-nine-arg-call-x86 [rel frame-base-slot-count]
  (let [stack0 (concat-byte-vectors
                 (emit-sub-rsp-imm32 264)
                 (emit-mov-thirty-third-stack-from-rax))
        stack1 (concat-byte-vectors stack0 (emit-mov-thirty-second-stack-from-rcx))
        stack2 (concat-byte-vectors
                  stack1
                  (emit-mov-rcx-from-local (native-value-window-spill-offset frame-base-slot-count 0)))
        stack3 (concat-byte-vectors stack2 (emit-mov-thirty-first-stack-from-rcx))
        stack4 (concat-byte-vectors
                  stack3
                  (emit-mov-rcx-from-local (native-value-window-spill-offset frame-base-slot-count 1)))
        stack5 (concat-byte-vectors stack4 (emit-mov-thirtieth-stack-from-rcx))
        stack6 (concat-byte-vectors
                  stack5
                  (emit-mov-rcx-from-local (native-value-window-spill-offset frame-base-slot-count 2)))
        stack7 (concat-byte-vectors stack6 (emit-mov-twenty-ninth-stack-from-rcx))
        stack8 (concat-byte-vectors
                  stack7
                  (emit-mov-rcx-from-local (native-value-window-spill-offset frame-base-slot-count 3)))
        stack9 (concat-byte-vectors stack8 (emit-mov-twenty-eighth-stack-from-rcx))
        stack10 (concat-byte-vectors
                  stack9
                  (emit-mov-rcx-from-local (native-value-window-spill-offset frame-base-slot-count 4)))
        stack11 (concat-byte-vectors stack10 (emit-mov-twenty-seventh-stack-from-rcx))
        stack12 (concat-byte-vectors
                  stack11
                  (emit-mov-rcx-from-local (native-value-window-spill-offset frame-base-slot-count 5)))
        stack13 (concat-byte-vectors stack12 (emit-mov-twenty-sixth-stack-from-rcx))
        stack14 (concat-byte-vectors
                  stack13
                  (emit-mov-rcx-from-local (native-value-window-spill-offset frame-base-slot-count 6)))
        stack15 (concat-byte-vectors stack14 (emit-mov-twenty-fifth-stack-from-rcx))
        stack16 (concat-byte-vectors
                  stack15
                  (emit-mov-rcx-from-local (native-value-window-spill-offset frame-base-slot-count 7)))
        stack17 (concat-byte-vectors stack16 (emit-mov-twenty-fourth-stack-from-rcx))
        stack18 (concat-byte-vectors
                  stack17
                  (emit-mov-rcx-from-local (native-value-window-spill-offset frame-base-slot-count 8)))
        stack19 (concat-byte-vectors stack18 (emit-mov-twenty-third-stack-from-rcx))
        stack20 (concat-byte-vectors
                  stack19
                  (emit-mov-rcx-from-local (native-value-window-spill-offset frame-base-slot-count 9)))
        stack21 (concat-byte-vectors stack20 (emit-mov-twenty-second-stack-from-rcx))
        stack22 (concat-byte-vectors
                  stack21
                  (emit-mov-rcx-from-local (native-value-window-spill-offset frame-base-slot-count 10)))
        stack23 (concat-byte-vectors stack22 (emit-mov-twenty-first-stack-from-rcx))
        stack24 (concat-byte-vectors
                  stack23
                  (emit-mov-rcx-from-local (native-value-window-spill-offset frame-base-slot-count 11)))
        stack25 (concat-byte-vectors stack24 (emit-mov-twentieth-stack-from-rcx))
        stack26 (concat-byte-vectors
                  stack25
                  (emit-mov-rcx-from-local (native-value-window-spill-offset frame-base-slot-count 12)))
        stack27 (concat-byte-vectors stack26 (emit-mov-nineteenth-stack-from-rcx))
        stack28 (concat-byte-vectors
                  stack27
                  (emit-mov-rcx-from-local (native-value-window-spill-offset frame-base-slot-count 13)))
        stack29 (concat-byte-vectors stack28 (emit-mov-eighteenth-stack-from-rcx))
        stack30 (concat-byte-vectors
                  stack29
                  (emit-mov-rcx-from-local (native-value-window-spill-offset frame-base-slot-count 14)))
        stack31 (concat-byte-vectors stack30 (emit-mov-seventeenth-stack-from-rcx))
        stack32 (concat-byte-vectors
                  stack31
                  (emit-mov-rcx-from-local (native-value-window-spill-offset frame-base-slot-count 15)))
        stack33 (concat-byte-vectors stack32 (emit-mov-sixteenth-stack-from-rcx))
        stack34 (concat-byte-vectors
                  stack33
                  (emit-mov-rcx-from-local (native-value-window-spill-offset frame-base-slot-count 16)))
        stack35 (concat-byte-vectors stack34 (emit-mov-fifteenth-stack-from-rcx))
        stack36 (concat-byte-vectors
                  stack35
                  (emit-mov-rcx-from-local (native-value-window-spill-offset frame-base-slot-count 17)))
        stack37 (concat-byte-vectors stack36 (emit-mov-fourteenth-stack-from-rcx))
        stack38 (concat-byte-vectors
                  stack37
                  (emit-mov-rcx-from-local (native-value-window-spill-offset frame-base-slot-count 18)))
        stack39 (concat-byte-vectors stack38 (emit-mov-thirteenth-stack-from-rcx))
        stack40 (concat-byte-vectors
                  stack39
                  (emit-mov-rcx-from-local (native-value-window-spill-offset frame-base-slot-count 19)))
        stack41 (concat-byte-vectors stack40 (emit-mov-twelfth-stack-from-rcx))
        stack42 (concat-byte-vectors
                  stack41
                  (emit-mov-rcx-from-local (native-value-window-spill-offset frame-base-slot-count 20)))
        stack43 (concat-byte-vectors stack42 (emit-mov-eleventh-stack-from-rcx))
        stack44 (concat-byte-vectors
                  stack43
                  (emit-mov-rcx-from-local (native-value-window-spill-offset frame-base-slot-count 21)))
        stack45 (concat-byte-vectors stack44 (emit-mov-tenth-stack-from-rcx))
        stack46 (concat-byte-vectors
                  stack45
                  (emit-mov-rcx-from-local (native-value-window-spill-offset frame-base-slot-count 22)))
        stack47 (concat-byte-vectors stack46 (emit-mov-ninth-stack-from-rcx))
        stack48 (concat-byte-vectors
                  stack47
                  (emit-mov-rcx-from-local (native-value-window-spill-offset frame-base-slot-count 23)))
        stack49 (concat-byte-vectors stack48 (emit-mov-eighth-stack-from-rcx))
        stack50 (concat-byte-vectors
                  stack49
                  (emit-mov-rcx-from-local (native-value-window-spill-offset frame-base-slot-count 24)))
        stack51 (concat-byte-vectors stack50 (emit-mov-seventh-stack-from-rcx))
        stack52 (concat-byte-vectors
                  stack51
                  (emit-mov-rcx-from-local (native-value-window-spill-offset frame-base-slot-count 25)))
        stack53 (concat-byte-vectors stack52 (emit-mov-sixth-stack-from-rcx))
        stack54 (concat-byte-vectors
                  stack53
                  (emit-mov-rcx-from-local (native-value-window-spill-offset frame-base-slot-count 26)))
        stack55 (concat-byte-vectors stack54 (emit-mov-fifth-stack-from-rcx))
        stack56 (concat-byte-vectors
                  stack55
                  (emit-mov-rcx-from-local (native-value-window-spill-offset frame-base-slot-count 27)))
        stack57 (concat-byte-vectors stack56 (emit-mov-fourth-stack-from-rcx))
        stack58 (concat-byte-vectors
                  stack57
                  (emit-mov-rcx-from-local (native-value-window-spill-offset frame-base-slot-count 28)))
        stack59 (concat-byte-vectors stack58 (emit-mov-third-stack-from-rcx))
        stack60 (concat-byte-vectors
                  stack59
                  (emit-mov-rcx-from-local (native-value-window-spill-offset frame-base-slot-count 29)))
        stack61 (concat-byte-vectors stack60 (emit-mov-second-stack-from-rcx))
        stack-setup (concat-byte-vectors
                      stack61
                      (emit-mov-r9-from-local (native-value-window-spill-offset frame-base-slot-count 30)))
        reg0 (concat-byte-vectors
               (emit-mov-top-stack-from-r9)
               (emit-mov-r9-from-local (native-value-window-spill-offset frame-base-slot-count 31)))
        reg1 (concat-byte-vectors reg0 (emit-mov-r8-from-local (native-value-window-spill-offset frame-base-slot-count 32)))
        reg2 (concat-byte-vectors reg1 (emit-mov-rcx-from-local (native-value-window-spill-offset frame-base-slot-count 33)))
        reg3 (concat-byte-vectors reg2 (emit-mov-rdx-from-local (native-value-window-spill-offset frame-base-slot-count 34)))
        reg4 (concat-byte-vectors reg3 (emit-mov-rsi-from-local (native-value-window-spill-offset frame-base-slot-count 35)))
        reg-setup (concat-byte-vectors reg4 (emit-mov-rdi-from-local (native-value-window-spill-offset frame-base-slot-count 36)))
        call-seq (concat-byte-vectors
                   (emit-call-rel32 rel)
                   (emit-add-rsp-imm32 264))]
    (concat-byte-vectors
      (concat-byte-vectors stack-setup reg-setup)
      call-seq)))

(defn emit-forty-arg-call-x86 [rel frame-base-slot-count]
  (let [stack0 (concat-byte-vectors
                 (emit-sub-rsp-imm32 272)
                 (emit-mov-thirty-fourth-stack-from-rax))
        stack1 (concat-byte-vectors stack0 (emit-mov-thirty-third-stack-from-rcx))
        stack2 (concat-byte-vectors
                  stack1
                  (emit-mov-rcx-from-local (native-value-window-spill-offset frame-base-slot-count 0)))
        stack3 (concat-byte-vectors stack2 (emit-mov-thirty-second-stack-from-rcx))
        stack4 (concat-byte-vectors
                  stack3
                  (emit-mov-rcx-from-local (native-value-window-spill-offset frame-base-slot-count 1)))
        stack5 (concat-byte-vectors stack4 (emit-mov-thirty-first-stack-from-rcx))
        stack6 (concat-byte-vectors
                  stack5
                  (emit-mov-rcx-from-local (native-value-window-spill-offset frame-base-slot-count 2)))
        stack7 (concat-byte-vectors stack6 (emit-mov-thirtieth-stack-from-rcx))
        stack8 (concat-byte-vectors
                  stack7
                  (emit-mov-rcx-from-local (native-value-window-spill-offset frame-base-slot-count 3)))
        stack9 (concat-byte-vectors stack8 (emit-mov-twenty-ninth-stack-from-rcx))
        stack10 (concat-byte-vectors
                   stack9
                   (emit-mov-rcx-from-local (native-value-window-spill-offset frame-base-slot-count 4)))
        stack11 (concat-byte-vectors stack10 (emit-mov-twenty-eighth-stack-from-rcx))
        stack12 (concat-byte-vectors
                   stack11
                   (emit-mov-rcx-from-local (native-value-window-spill-offset frame-base-slot-count 5)))
        stack13 (concat-byte-vectors stack12 (emit-mov-twenty-seventh-stack-from-rcx))
        stack14 (concat-byte-vectors
                   stack13
                   (emit-mov-rcx-from-local (native-value-window-spill-offset frame-base-slot-count 6)))
        stack15 (concat-byte-vectors stack14 (emit-mov-twenty-sixth-stack-from-rcx))
        stack16 (concat-byte-vectors
                   stack15
                   (emit-mov-rcx-from-local (native-value-window-spill-offset frame-base-slot-count 7)))
        stack17 (concat-byte-vectors stack16 (emit-mov-twenty-fifth-stack-from-rcx))
        stack18 (concat-byte-vectors
                   stack17
                   (emit-mov-rcx-from-local (native-value-window-spill-offset frame-base-slot-count 8)))
        stack19 (concat-byte-vectors stack18 (emit-mov-twenty-fourth-stack-from-rcx))
        stack20 (concat-byte-vectors
                   stack19
                   (emit-mov-rcx-from-local (native-value-window-spill-offset frame-base-slot-count 9)))
        stack21 (concat-byte-vectors stack20 (emit-mov-twenty-third-stack-from-rcx))
        stack22 (concat-byte-vectors
                   stack21
                   (emit-mov-rcx-from-local (native-value-window-spill-offset frame-base-slot-count 10)))
        stack23 (concat-byte-vectors stack22 (emit-mov-twenty-second-stack-from-rcx))
        stack24 (concat-byte-vectors
                   stack23
                   (emit-mov-rcx-from-local (native-value-window-spill-offset frame-base-slot-count 11)))
        stack25 (concat-byte-vectors stack24 (emit-mov-twenty-first-stack-from-rcx))
        stack26 (concat-byte-vectors
                   stack25
                   (emit-mov-rcx-from-local (native-value-window-spill-offset frame-base-slot-count 12)))
        stack27 (concat-byte-vectors stack26 (emit-mov-twentieth-stack-from-rcx))
        stack28 (concat-byte-vectors
                   stack27
                   (emit-mov-rcx-from-local (native-value-window-spill-offset frame-base-slot-count 13)))
        stack29 (concat-byte-vectors stack28 (emit-mov-nineteenth-stack-from-rcx))
        stack30 (concat-byte-vectors
                   stack29
                   (emit-mov-rcx-from-local (native-value-window-spill-offset frame-base-slot-count 14)))
        stack31 (concat-byte-vectors stack30 (emit-mov-eighteenth-stack-from-rcx))
        stack32 (concat-byte-vectors
                   stack31
                   (emit-mov-rcx-from-local (native-value-window-spill-offset frame-base-slot-count 15)))
        stack33 (concat-byte-vectors stack32 (emit-mov-seventeenth-stack-from-rcx))
        stack34 (concat-byte-vectors
                   stack33
                   (emit-mov-rcx-from-local (native-value-window-spill-offset frame-base-slot-count 16)))
        stack35 (concat-byte-vectors stack34 (emit-mov-sixteenth-stack-from-rcx))
        stack36 (concat-byte-vectors
                   stack35
                   (emit-mov-rcx-from-local (native-value-window-spill-offset frame-base-slot-count 17)))
        stack37 (concat-byte-vectors stack36 (emit-mov-fifteenth-stack-from-rcx))
        stack38 (concat-byte-vectors
                   stack37
                   (emit-mov-rcx-from-local (native-value-window-spill-offset frame-base-slot-count 18)))
        stack39 (concat-byte-vectors stack38 (emit-mov-fourteenth-stack-from-rcx))
        stack40 (concat-byte-vectors
                   stack39
                   (emit-mov-rcx-from-local (native-value-window-spill-offset frame-base-slot-count 19)))
        stack41 (concat-byte-vectors stack40 (emit-mov-thirteenth-stack-from-rcx))
        stack42 (concat-byte-vectors
                   stack41
                   (emit-mov-rcx-from-local (native-value-window-spill-offset frame-base-slot-count 20)))
        stack43 (concat-byte-vectors stack42 (emit-mov-twelfth-stack-from-rcx))
        stack44 (concat-byte-vectors
                   stack43
                   (emit-mov-rcx-from-local (native-value-window-spill-offset frame-base-slot-count 21)))
        stack45 (concat-byte-vectors stack44 (emit-mov-eleventh-stack-from-rcx))
        stack46 (concat-byte-vectors
                   stack45
                   (emit-mov-rcx-from-local (native-value-window-spill-offset frame-base-slot-count 22)))
        stack47 (concat-byte-vectors stack46 (emit-mov-tenth-stack-from-rcx))
        stack48 (concat-byte-vectors
                   stack47
                   (emit-mov-rcx-from-local (native-value-window-spill-offset frame-base-slot-count 23)))
        stack49 (concat-byte-vectors stack48 (emit-mov-ninth-stack-from-rcx))
        stack50 (concat-byte-vectors
                   stack49
                   (emit-mov-rcx-from-local (native-value-window-spill-offset frame-base-slot-count 24)))
        stack51 (concat-byte-vectors stack50 (emit-mov-eighth-stack-from-rcx))
        stack52 (concat-byte-vectors
                   stack51
                   (emit-mov-rcx-from-local (native-value-window-spill-offset frame-base-slot-count 25)))
        stack53 (concat-byte-vectors stack52 (emit-mov-seventh-stack-from-rcx))
        stack54 (concat-byte-vectors
                   stack53
                   (emit-mov-rcx-from-local (native-value-window-spill-offset frame-base-slot-count 26)))
        stack55 (concat-byte-vectors stack54 (emit-mov-sixth-stack-from-rcx))
        stack56 (concat-byte-vectors
                   stack55
                   (emit-mov-rcx-from-local (native-value-window-spill-offset frame-base-slot-count 27)))
        stack57 (concat-byte-vectors stack56 (emit-mov-fifth-stack-from-rcx))
        stack58 (concat-byte-vectors
                   stack57
                   (emit-mov-rcx-from-local (native-value-window-spill-offset frame-base-slot-count 28)))
        stack59 (concat-byte-vectors stack58 (emit-mov-fourth-stack-from-rcx))
        stack60 (concat-byte-vectors
                   stack59
                   (emit-mov-rcx-from-local (native-value-window-spill-offset frame-base-slot-count 29)))
        stack61 (concat-byte-vectors stack60 (emit-mov-third-stack-from-rcx))
        stack62 (concat-byte-vectors
                   stack61
                   (emit-mov-rcx-from-local (native-value-window-spill-offset frame-base-slot-count 30)))
        stack63 (concat-byte-vectors stack62 (emit-mov-second-stack-from-rcx))
        stack-setup (concat-byte-vectors
                      stack63
                      (emit-mov-r9-from-local (native-value-window-spill-offset frame-base-slot-count 31)))
        reg0 (concat-byte-vectors
               (emit-mov-top-stack-from-r9)
               (emit-mov-r9-from-local (native-value-window-spill-offset frame-base-slot-count 32)))
        reg1 (concat-byte-vectors reg0 (emit-mov-r8-from-local (native-value-window-spill-offset frame-base-slot-count 33)))
        reg2 (concat-byte-vectors reg1 (emit-mov-rcx-from-local (native-value-window-spill-offset frame-base-slot-count 34)))
        reg3 (concat-byte-vectors reg2 (emit-mov-rdx-from-local (native-value-window-spill-offset frame-base-slot-count 35)))
        reg4 (concat-byte-vectors reg3 (emit-mov-rsi-from-local (native-value-window-spill-offset frame-base-slot-count 36)))
        reg-setup (concat-byte-vectors reg4 (emit-mov-rdi-from-local (native-value-window-spill-offset frame-base-slot-count 37)))
        call-seq (concat-byte-vectors
                   (emit-call-rel32 rel)
                   (emit-add-rsp-imm32 272))]
    (concat-byte-vectors
      (concat-byte-vectors stack-setup reg-setup)
      call-seq)))

(defn emit-forty-one-arg-call-x86 [rel frame-base-slot-count]
  (let [stack0 (concat-byte-vectors
                 (emit-sub-rsp-imm32 280)
                 (emit-mov-thirty-fifth-stack-from-rax))
        stack1 (concat-byte-vectors stack0 (emit-mov-thirty-fourth-stack-from-rcx))
        stack2 (concat-byte-vectors
                  stack1
                  (emit-mov-rcx-from-local (native-value-window-spill-offset frame-base-slot-count 0)))
        stack3 (concat-byte-vectors stack2 (emit-mov-thirty-third-stack-from-rcx))
        stack4 (concat-byte-vectors
                  stack3
                  (emit-mov-rcx-from-local (native-value-window-spill-offset frame-base-slot-count 1)))
        stack5 (concat-byte-vectors stack4 (emit-mov-thirty-second-stack-from-rcx))
        stack6 (concat-byte-vectors
                  stack5
                  (emit-mov-rcx-from-local (native-value-window-spill-offset frame-base-slot-count 2)))
        stack7 (concat-byte-vectors stack6 (emit-mov-thirty-first-stack-from-rcx))
        stack8 (concat-byte-vectors
                  stack7
                  (emit-mov-rcx-from-local (native-value-window-spill-offset frame-base-slot-count 3)))
        stack9 (concat-byte-vectors stack8 (emit-mov-thirtieth-stack-from-rcx))
        stack10 (concat-byte-vectors
                   stack9
                   (emit-mov-rcx-from-local (native-value-window-spill-offset frame-base-slot-count 4)))
        stack11 (concat-byte-vectors stack10 (emit-mov-twenty-ninth-stack-from-rcx))
        stack12 (concat-byte-vectors
                   stack11
                   (emit-mov-rcx-from-local (native-value-window-spill-offset frame-base-slot-count 5)))
        stack13 (concat-byte-vectors stack12 (emit-mov-twenty-eighth-stack-from-rcx))
        stack14 (concat-byte-vectors
                   stack13
                   (emit-mov-rcx-from-local (native-value-window-spill-offset frame-base-slot-count 6)))
        stack15 (concat-byte-vectors stack14 (emit-mov-twenty-seventh-stack-from-rcx))
        stack16 (concat-byte-vectors
                   stack15
                   (emit-mov-rcx-from-local (native-value-window-spill-offset frame-base-slot-count 7)))
        stack17 (concat-byte-vectors stack16 (emit-mov-twenty-sixth-stack-from-rcx))
        stack18 (concat-byte-vectors
                   stack17
                   (emit-mov-rcx-from-local (native-value-window-spill-offset frame-base-slot-count 8)))
        stack19 (concat-byte-vectors stack18 (emit-mov-twenty-fifth-stack-from-rcx))
        stack20 (concat-byte-vectors
                   stack19
                   (emit-mov-rcx-from-local (native-value-window-spill-offset frame-base-slot-count 9)))
        stack21 (concat-byte-vectors stack20 (emit-mov-twenty-fourth-stack-from-rcx))
        stack22 (concat-byte-vectors
                   stack21
                   (emit-mov-rcx-from-local (native-value-window-spill-offset frame-base-slot-count 10)))
        stack23 (concat-byte-vectors stack22 (emit-mov-twenty-third-stack-from-rcx))
        stack24 (concat-byte-vectors
                   stack23
                   (emit-mov-rcx-from-local (native-value-window-spill-offset frame-base-slot-count 11)))
        stack25 (concat-byte-vectors stack24 (emit-mov-twenty-second-stack-from-rcx))
        stack26 (concat-byte-vectors
                   stack25
                   (emit-mov-rcx-from-local (native-value-window-spill-offset frame-base-slot-count 12)))
        stack27 (concat-byte-vectors stack26 (emit-mov-twenty-first-stack-from-rcx))
        stack28 (concat-byte-vectors
                   stack27
                   (emit-mov-rcx-from-local (native-value-window-spill-offset frame-base-slot-count 13)))
        stack29 (concat-byte-vectors stack28 (emit-mov-twentieth-stack-from-rcx))
        stack30 (concat-byte-vectors
                   stack29
                   (emit-mov-rcx-from-local (native-value-window-spill-offset frame-base-slot-count 14)))
        stack31 (concat-byte-vectors stack30 (emit-mov-nineteenth-stack-from-rcx))
        stack32 (concat-byte-vectors
                   stack31
                   (emit-mov-rcx-from-local (native-value-window-spill-offset frame-base-slot-count 15)))
        stack33 (concat-byte-vectors stack32 (emit-mov-eighteenth-stack-from-rcx))
        stack34 (concat-byte-vectors
                   stack33
                   (emit-mov-rcx-from-local (native-value-window-spill-offset frame-base-slot-count 16)))
        stack35 (concat-byte-vectors stack34 (emit-mov-seventeenth-stack-from-rcx))
        stack36 (concat-byte-vectors
                   stack35
                   (emit-mov-rcx-from-local (native-value-window-spill-offset frame-base-slot-count 17)))
        stack37 (concat-byte-vectors stack36 (emit-mov-sixteenth-stack-from-rcx))
        stack38 (concat-byte-vectors
                   stack37
                   (emit-mov-rcx-from-local (native-value-window-spill-offset frame-base-slot-count 18)))
        stack39 (concat-byte-vectors stack38 (emit-mov-fifteenth-stack-from-rcx))
        stack40 (concat-byte-vectors
                   stack39
                   (emit-mov-rcx-from-local (native-value-window-spill-offset frame-base-slot-count 19)))
        stack41 (concat-byte-vectors stack40 (emit-mov-fourteenth-stack-from-rcx))
        stack42 (concat-byte-vectors
                   stack41
                   (emit-mov-rcx-from-local (native-value-window-spill-offset frame-base-slot-count 20)))
        stack43 (concat-byte-vectors stack42 (emit-mov-thirteenth-stack-from-rcx))
        stack44 (concat-byte-vectors
                   stack43
                   (emit-mov-rcx-from-local (native-value-window-spill-offset frame-base-slot-count 21)))
        stack45 (concat-byte-vectors stack44 (emit-mov-twelfth-stack-from-rcx))
        stack46 (concat-byte-vectors
                   stack45
                   (emit-mov-rcx-from-local (native-value-window-spill-offset frame-base-slot-count 22)))
        stack47 (concat-byte-vectors stack46 (emit-mov-eleventh-stack-from-rcx))
        stack48 (concat-byte-vectors
                   stack47
                   (emit-mov-rcx-from-local (native-value-window-spill-offset frame-base-slot-count 23)))
        stack49 (concat-byte-vectors stack48 (emit-mov-tenth-stack-from-rcx))
        stack50 (concat-byte-vectors
                   stack49
                   (emit-mov-rcx-from-local (native-value-window-spill-offset frame-base-slot-count 24)))
        stack51 (concat-byte-vectors stack50 (emit-mov-ninth-stack-from-rcx))
        stack52 (concat-byte-vectors
                   stack51
                   (emit-mov-rcx-from-local (native-value-window-spill-offset frame-base-slot-count 25)))
        stack53 (concat-byte-vectors stack52 (emit-mov-eighth-stack-from-rcx))
        stack54 (concat-byte-vectors
                   stack53
                   (emit-mov-rcx-from-local (native-value-window-spill-offset frame-base-slot-count 26)))
        stack55 (concat-byte-vectors stack54 (emit-mov-seventh-stack-from-rcx))
        stack56 (concat-byte-vectors
                   stack55
                   (emit-mov-rcx-from-local (native-value-window-spill-offset frame-base-slot-count 27)))
        stack57 (concat-byte-vectors stack56 (emit-mov-sixth-stack-from-rcx))
        stack58 (concat-byte-vectors
                   stack57
                   (emit-mov-rcx-from-local (native-value-window-spill-offset frame-base-slot-count 28)))
        stack59 (concat-byte-vectors stack58 (emit-mov-fifth-stack-from-rcx))
        stack60 (concat-byte-vectors
                   stack59
                   (emit-mov-rcx-from-local (native-value-window-spill-offset frame-base-slot-count 29)))
        stack61 (concat-byte-vectors stack60 (emit-mov-fourth-stack-from-rcx))
        stack62 (concat-byte-vectors
                   stack61
                   (emit-mov-rcx-from-local (native-value-window-spill-offset frame-base-slot-count 30)))
        stack63 (concat-byte-vectors stack62 (emit-mov-third-stack-from-rcx))
        stack64 (concat-byte-vectors
                   stack63
                   (emit-mov-rcx-from-local (native-value-window-spill-offset frame-base-slot-count 31)))
        stack65 (concat-byte-vectors stack64 (emit-mov-second-stack-from-rcx))
        stack-setup (concat-byte-vectors
                      stack65
                      (emit-mov-r9-from-local (native-value-window-spill-offset frame-base-slot-count 32)))
        reg0 (concat-byte-vectors
               (emit-mov-top-stack-from-r9)
               (emit-mov-r9-from-local (native-value-window-spill-offset frame-base-slot-count 33)))
        reg1 (concat-byte-vectors reg0 (emit-mov-r8-from-local (native-value-window-spill-offset frame-base-slot-count 34)))
        reg2 (concat-byte-vectors reg1 (emit-mov-rcx-from-local (native-value-window-spill-offset frame-base-slot-count 35)))
        reg3 (concat-byte-vectors reg2 (emit-mov-rdx-from-local (native-value-window-spill-offset frame-base-slot-count 36)))
        reg4 (concat-byte-vectors reg3 (emit-mov-rsi-from-local (native-value-window-spill-offset frame-base-slot-count 37)))
        reg-setup (concat-byte-vectors reg4 (emit-mov-rdi-from-local (native-value-window-spill-offset frame-base-slot-count 38)))
        call-seq (concat-byte-vectors
                   (emit-call-rel32 rel)
                   (emit-add-rsp-imm32 280))]
    (concat-byte-vectors
      (concat-byte-vectors stack-setup reg-setup)
      call-seq)))

(defn emit-forty-two-arg-call-x86 [rel frame-base-slot-count]
  (let [stack0 (concat-byte-vectors
                 (emit-sub-rsp-imm32 288)
                 (emit-mov-thirty-sixth-stack-from-rax))
        stack1 (concat-byte-vectors stack0 (emit-mov-thirty-fifth-stack-from-rcx))
        stack2 (concat-byte-vectors
                  stack1
                  (emit-mov-rcx-from-local (native-value-window-spill-offset frame-base-slot-count 0)))
        stack3 (concat-byte-vectors stack2 (emit-mov-thirty-fourth-stack-from-rcx))
        stack4 (concat-byte-vectors
                  stack3
                  (emit-mov-rcx-from-local (native-value-window-spill-offset frame-base-slot-count 1)))
        stack5 (concat-byte-vectors stack4 (emit-mov-thirty-third-stack-from-rcx))
        stack6 (concat-byte-vectors
                  stack5
                  (emit-mov-rcx-from-local (native-value-window-spill-offset frame-base-slot-count 2)))
        stack7 (concat-byte-vectors stack6 (emit-mov-thirty-second-stack-from-rcx))
        stack8 (concat-byte-vectors
                  stack7
                  (emit-mov-rcx-from-local (native-value-window-spill-offset frame-base-slot-count 3)))
        stack9 (concat-byte-vectors stack8 (emit-mov-thirty-first-stack-from-rcx))
        stack10 (concat-byte-vectors
                   stack9
                   (emit-mov-rcx-from-local (native-value-window-spill-offset frame-base-slot-count 4)))
        stack11 (concat-byte-vectors stack10 (emit-mov-thirtieth-stack-from-rcx))
        stack12 (concat-byte-vectors
                   stack11
                   (emit-mov-rcx-from-local (native-value-window-spill-offset frame-base-slot-count 5)))
        stack13 (concat-byte-vectors stack12 (emit-mov-twenty-ninth-stack-from-rcx))
        stack14 (concat-byte-vectors
                   stack13
                   (emit-mov-rcx-from-local (native-value-window-spill-offset frame-base-slot-count 6)))
        stack15 (concat-byte-vectors stack14 (emit-mov-twenty-eighth-stack-from-rcx))
        stack16 (concat-byte-vectors
                   stack15
                   (emit-mov-rcx-from-local (native-value-window-spill-offset frame-base-slot-count 7)))
        stack17 (concat-byte-vectors stack16 (emit-mov-twenty-seventh-stack-from-rcx))
        stack18 (concat-byte-vectors
                   stack17
                   (emit-mov-rcx-from-local (native-value-window-spill-offset frame-base-slot-count 8)))
        stack19 (concat-byte-vectors stack18 (emit-mov-twenty-sixth-stack-from-rcx))
        stack20 (concat-byte-vectors
                   stack19
                   (emit-mov-rcx-from-local (native-value-window-spill-offset frame-base-slot-count 9)))
        stack21 (concat-byte-vectors stack20 (emit-mov-twenty-fifth-stack-from-rcx))
        stack22 (concat-byte-vectors
                   stack21
                   (emit-mov-rcx-from-local (native-value-window-spill-offset frame-base-slot-count 10)))
        stack23 (concat-byte-vectors stack22 (emit-mov-twenty-fourth-stack-from-rcx))
        stack24 (concat-byte-vectors
                   stack23
                   (emit-mov-rcx-from-local (native-value-window-spill-offset frame-base-slot-count 11)))
        stack25 (concat-byte-vectors stack24 (emit-mov-twenty-third-stack-from-rcx))
        stack26 (concat-byte-vectors
                   stack25
                   (emit-mov-rcx-from-local (native-value-window-spill-offset frame-base-slot-count 12)))
        stack27 (concat-byte-vectors stack26 (emit-mov-twenty-second-stack-from-rcx))
        stack28 (concat-byte-vectors
                   stack27
                   (emit-mov-rcx-from-local (native-value-window-spill-offset frame-base-slot-count 13)))
        stack29 (concat-byte-vectors stack28 (emit-mov-twenty-first-stack-from-rcx))
        stack30 (concat-byte-vectors
                   stack29
                   (emit-mov-rcx-from-local (native-value-window-spill-offset frame-base-slot-count 14)))
        stack31 (concat-byte-vectors stack30 (emit-mov-twentieth-stack-from-rcx))
        stack32 (concat-byte-vectors
                   stack31
                   (emit-mov-rcx-from-local (native-value-window-spill-offset frame-base-slot-count 15)))
        stack33 (concat-byte-vectors stack32 (emit-mov-nineteenth-stack-from-rcx))
        stack34 (concat-byte-vectors
                   stack33
                   (emit-mov-rcx-from-local (native-value-window-spill-offset frame-base-slot-count 16)))
        stack35 (concat-byte-vectors stack34 (emit-mov-eighteenth-stack-from-rcx))
        stack36 (concat-byte-vectors
                   stack35
                   (emit-mov-rcx-from-local (native-value-window-spill-offset frame-base-slot-count 17)))
        stack37 (concat-byte-vectors stack36 (emit-mov-seventeenth-stack-from-rcx))
        stack38 (concat-byte-vectors
                   stack37
                   (emit-mov-rcx-from-local (native-value-window-spill-offset frame-base-slot-count 18)))
        stack39 (concat-byte-vectors stack38 (emit-mov-sixteenth-stack-from-rcx))
        stack40 (concat-byte-vectors
                   stack39
                   (emit-mov-rcx-from-local (native-value-window-spill-offset frame-base-slot-count 19)))
        stack41 (concat-byte-vectors stack40 (emit-mov-fifteenth-stack-from-rcx))
        stack42 (concat-byte-vectors
                   stack41
                   (emit-mov-rcx-from-local (native-value-window-spill-offset frame-base-slot-count 20)))
        stack43 (concat-byte-vectors stack42 (emit-mov-fourteenth-stack-from-rcx))
        stack44 (concat-byte-vectors
                   stack43
                   (emit-mov-rcx-from-local (native-value-window-spill-offset frame-base-slot-count 21)))
        stack45 (concat-byte-vectors stack44 (emit-mov-thirteenth-stack-from-rcx))
        stack46 (concat-byte-vectors
                   stack45
                   (emit-mov-rcx-from-local (native-value-window-spill-offset frame-base-slot-count 22)))
        stack47 (concat-byte-vectors stack46 (emit-mov-twelfth-stack-from-rcx))
        stack48 (concat-byte-vectors
                   stack47
                   (emit-mov-rcx-from-local (native-value-window-spill-offset frame-base-slot-count 23)))
        stack49 (concat-byte-vectors stack48 (emit-mov-eleventh-stack-from-rcx))
        stack50 (concat-byte-vectors
                   stack49
                   (emit-mov-rcx-from-local (native-value-window-spill-offset frame-base-slot-count 24)))
        stack51 (concat-byte-vectors stack50 (emit-mov-tenth-stack-from-rcx))
        stack52 (concat-byte-vectors
                   stack51
                   (emit-mov-rcx-from-local (native-value-window-spill-offset frame-base-slot-count 25)))
        stack53 (concat-byte-vectors stack52 (emit-mov-ninth-stack-from-rcx))
        stack54 (concat-byte-vectors
                   stack53
                   (emit-mov-rcx-from-local (native-value-window-spill-offset frame-base-slot-count 26)))
        stack55 (concat-byte-vectors stack54 (emit-mov-eighth-stack-from-rcx))
        stack56 (concat-byte-vectors
                   stack55
                   (emit-mov-rcx-from-local (native-value-window-spill-offset frame-base-slot-count 27)))
        stack57 (concat-byte-vectors stack56 (emit-mov-seventh-stack-from-rcx))
        stack58 (concat-byte-vectors
                   stack57
                   (emit-mov-rcx-from-local (native-value-window-spill-offset frame-base-slot-count 28)))
        stack59 (concat-byte-vectors stack58 (emit-mov-sixth-stack-from-rcx))
        stack60 (concat-byte-vectors
                   stack59
                   (emit-mov-rcx-from-local (native-value-window-spill-offset frame-base-slot-count 29)))
        stack61 (concat-byte-vectors stack60 (emit-mov-fifth-stack-from-rcx))
        stack62 (concat-byte-vectors
                   stack61
                   (emit-mov-rcx-from-local (native-value-window-spill-offset frame-base-slot-count 30)))
        stack63 (concat-byte-vectors stack62 (emit-mov-fourth-stack-from-rcx))
        stack64 (concat-byte-vectors
                   stack63
                   (emit-mov-rcx-from-local (native-value-window-spill-offset frame-base-slot-count 31)))
        stack65 (concat-byte-vectors stack64 (emit-mov-third-stack-from-rcx))
        stack66 (concat-byte-vectors
                   stack65
                   (emit-mov-rcx-from-local (native-value-window-spill-offset frame-base-slot-count 32)))
        stack67 (concat-byte-vectors stack66 (emit-mov-second-stack-from-rcx))
        stack-setup (concat-byte-vectors
                      stack67
                      (emit-mov-r9-from-local (native-value-window-spill-offset frame-base-slot-count 33)))
        reg0 (concat-byte-vectors
               (emit-mov-top-stack-from-r9)
               (emit-mov-r9-from-local (native-value-window-spill-offset frame-base-slot-count 34)))
        reg1 (concat-byte-vectors reg0 (emit-mov-r8-from-local (native-value-window-spill-offset frame-base-slot-count 35)))
        reg2 (concat-byte-vectors reg1 (emit-mov-rcx-from-local (native-value-window-spill-offset frame-base-slot-count 36)))
        reg3 (concat-byte-vectors reg2 (emit-mov-rdx-from-local (native-value-window-spill-offset frame-base-slot-count 37)))
        reg4 (concat-byte-vectors reg3 (emit-mov-rsi-from-local (native-value-window-spill-offset frame-base-slot-count 38)))
        reg-setup (concat-byte-vectors reg4 (emit-mov-rdi-from-local (native-value-window-spill-offset frame-base-slot-count 39)))
        call-seq (concat-byte-vectors
                   (emit-call-rel32 rel)
                   (emit-add-rsp-imm32 288))]
    (concat-byte-vectors
      (concat-byte-vectors stack-setup reg-setup)
      call-seq)))

(defn emit-forty-three-arg-call-x86 [rel frame-base-slot-count]
  (let [stack0 (concat-byte-vectors
                 (emit-sub-rsp-imm32 296)
                 (emit-mov-thirty-seventh-stack-from-rax))
        stack1 (concat-byte-vectors stack0 (emit-mov-thirty-sixth-stack-from-rcx))
        stack2 (concat-byte-vectors
                  stack1
                  (emit-mov-rcx-from-local (native-value-window-spill-offset frame-base-slot-count 0)))
        stack3 (concat-byte-vectors stack2 (emit-mov-thirty-fifth-stack-from-rcx))
        stack4 (concat-byte-vectors
                  stack3
                  (emit-mov-rcx-from-local (native-value-window-spill-offset frame-base-slot-count 1)))
        stack5 (concat-byte-vectors stack4 (emit-mov-thirty-fourth-stack-from-rcx))
        stack6 (concat-byte-vectors
                  stack5
                  (emit-mov-rcx-from-local (native-value-window-spill-offset frame-base-slot-count 2)))
        stack7 (concat-byte-vectors stack6 (emit-mov-thirty-third-stack-from-rcx))
        stack8 (concat-byte-vectors
                  stack7
                  (emit-mov-rcx-from-local (native-value-window-spill-offset frame-base-slot-count 3)))
        stack9 (concat-byte-vectors stack8 (emit-mov-thirty-second-stack-from-rcx))
        stack10 (concat-byte-vectors
                   stack9
                   (emit-mov-rcx-from-local (native-value-window-spill-offset frame-base-slot-count 4)))
        stack11 (concat-byte-vectors stack10 (emit-mov-thirty-first-stack-from-rcx))
        stack12 (concat-byte-vectors
                   stack11
                   (emit-mov-rcx-from-local (native-value-window-spill-offset frame-base-slot-count 5)))
        stack13 (concat-byte-vectors stack12 (emit-mov-thirtieth-stack-from-rcx))
        stack14 (concat-byte-vectors
                   stack13
                   (emit-mov-rcx-from-local (native-value-window-spill-offset frame-base-slot-count 6)))
        stack15 (concat-byte-vectors stack14 (emit-mov-twenty-ninth-stack-from-rcx))
        stack16 (concat-byte-vectors
                   stack15
                   (emit-mov-rcx-from-local (native-value-window-spill-offset frame-base-slot-count 7)))
        stack17 (concat-byte-vectors stack16 (emit-mov-twenty-eighth-stack-from-rcx))
        stack18 (concat-byte-vectors
                   stack17
                   (emit-mov-rcx-from-local (native-value-window-spill-offset frame-base-slot-count 8)))
        stack19 (concat-byte-vectors stack18 (emit-mov-twenty-seventh-stack-from-rcx))
        stack20 (concat-byte-vectors
                   stack19
                   (emit-mov-rcx-from-local (native-value-window-spill-offset frame-base-slot-count 9)))
        stack21 (concat-byte-vectors stack20 (emit-mov-twenty-sixth-stack-from-rcx))
        stack22 (concat-byte-vectors
                   stack21
                   (emit-mov-rcx-from-local (native-value-window-spill-offset frame-base-slot-count 10)))
        stack23 (concat-byte-vectors stack22 (emit-mov-twenty-fifth-stack-from-rcx))
        stack24 (concat-byte-vectors
                   stack23
                   (emit-mov-rcx-from-local (native-value-window-spill-offset frame-base-slot-count 11)))
        stack25 (concat-byte-vectors stack24 (emit-mov-twenty-fourth-stack-from-rcx))
        stack26 (concat-byte-vectors
                   stack25
                   (emit-mov-rcx-from-local (native-value-window-spill-offset frame-base-slot-count 12)))
        stack27 (concat-byte-vectors stack26 (emit-mov-twenty-third-stack-from-rcx))
        stack28 (concat-byte-vectors
                   stack27
                   (emit-mov-rcx-from-local (native-value-window-spill-offset frame-base-slot-count 13)))
        stack29 (concat-byte-vectors stack28 (emit-mov-twenty-second-stack-from-rcx))
        stack30 (concat-byte-vectors
                   stack29
                   (emit-mov-rcx-from-local (native-value-window-spill-offset frame-base-slot-count 14)))
        stack31 (concat-byte-vectors stack30 (emit-mov-twenty-first-stack-from-rcx))
        stack32 (concat-byte-vectors
                   stack31
                   (emit-mov-rcx-from-local (native-value-window-spill-offset frame-base-slot-count 15)))
        stack33 (concat-byte-vectors stack32 (emit-mov-twentieth-stack-from-rcx))
        stack34 (concat-byte-vectors
                   stack33
                   (emit-mov-rcx-from-local (native-value-window-spill-offset frame-base-slot-count 16)))
        stack35 (concat-byte-vectors stack34 (emit-mov-nineteenth-stack-from-rcx))
        stack36 (concat-byte-vectors
                   stack35
                   (emit-mov-rcx-from-local (native-value-window-spill-offset frame-base-slot-count 17)))
        stack37 (concat-byte-vectors stack36 (emit-mov-eighteenth-stack-from-rcx))
        stack38 (concat-byte-vectors
                   stack37
                   (emit-mov-rcx-from-local (native-value-window-spill-offset frame-base-slot-count 18)))
        stack39 (concat-byte-vectors stack38 (emit-mov-seventeenth-stack-from-rcx))
        stack40 (concat-byte-vectors
                   stack39
                   (emit-mov-rcx-from-local (native-value-window-spill-offset frame-base-slot-count 19)))
        stack41 (concat-byte-vectors stack40 (emit-mov-sixteenth-stack-from-rcx))
        stack42 (concat-byte-vectors
                   stack41
                   (emit-mov-rcx-from-local (native-value-window-spill-offset frame-base-slot-count 20)))
        stack43 (concat-byte-vectors stack42 (emit-mov-fifteenth-stack-from-rcx))
        stack44 (concat-byte-vectors
                   stack43
                   (emit-mov-rcx-from-local (native-value-window-spill-offset frame-base-slot-count 21)))
        stack45 (concat-byte-vectors stack44 (emit-mov-fourteenth-stack-from-rcx))
        stack46 (concat-byte-vectors
                   stack45
                   (emit-mov-rcx-from-local (native-value-window-spill-offset frame-base-slot-count 22)))
        stack47 (concat-byte-vectors stack46 (emit-mov-thirteenth-stack-from-rcx))
        stack48 (concat-byte-vectors
                   stack47
                   (emit-mov-rcx-from-local (native-value-window-spill-offset frame-base-slot-count 23)))
        stack49 (concat-byte-vectors stack48 (emit-mov-twelfth-stack-from-rcx))
        stack50 (concat-byte-vectors
                   stack49
                   (emit-mov-rcx-from-local (native-value-window-spill-offset frame-base-slot-count 24)))
        stack51 (concat-byte-vectors stack50 (emit-mov-eleventh-stack-from-rcx))
        stack52 (concat-byte-vectors
                   stack51
                   (emit-mov-rcx-from-local (native-value-window-spill-offset frame-base-slot-count 25)))
        stack53 (concat-byte-vectors stack52 (emit-mov-tenth-stack-from-rcx))
        stack54 (concat-byte-vectors
                   stack53
                   (emit-mov-rcx-from-local (native-value-window-spill-offset frame-base-slot-count 26)))
        stack55 (concat-byte-vectors stack54 (emit-mov-ninth-stack-from-rcx))
        stack56 (concat-byte-vectors
                   stack55
                   (emit-mov-rcx-from-local (native-value-window-spill-offset frame-base-slot-count 27)))
        stack57 (concat-byte-vectors stack56 (emit-mov-eighth-stack-from-rcx))
        stack58 (concat-byte-vectors
                   stack57
                   (emit-mov-rcx-from-local (native-value-window-spill-offset frame-base-slot-count 28)))
        stack59 (concat-byte-vectors stack58 (emit-mov-seventh-stack-from-rcx))
        stack60 (concat-byte-vectors
                   stack59
                   (emit-mov-rcx-from-local (native-value-window-spill-offset frame-base-slot-count 29)))
        stack61 (concat-byte-vectors stack60 (emit-mov-sixth-stack-from-rcx))
        stack62 (concat-byte-vectors
                   stack61
                   (emit-mov-rcx-from-local (native-value-window-spill-offset frame-base-slot-count 30)))
        stack63 (concat-byte-vectors stack62 (emit-mov-fifth-stack-from-rcx))
        stack64 (concat-byte-vectors
                   stack63
                   (emit-mov-rcx-from-local (native-value-window-spill-offset frame-base-slot-count 31)))
        stack65 (concat-byte-vectors stack64 (emit-mov-fourth-stack-from-rcx))
        stack66 (concat-byte-vectors
                   stack65
                   (emit-mov-rcx-from-local (native-value-window-spill-offset frame-base-slot-count 32)))
        stack67 (concat-byte-vectors stack66 (emit-mov-third-stack-from-rcx))
        stack68 (concat-byte-vectors
                   stack67
                   (emit-mov-rcx-from-local (native-value-window-spill-offset frame-base-slot-count 33)))
        stack69 (concat-byte-vectors stack68 (emit-mov-second-stack-from-rcx))
        stack-setup (concat-byte-vectors
                      stack69
                      (emit-mov-r9-from-local (native-value-window-spill-offset frame-base-slot-count 34)))
        reg0 (concat-byte-vectors
               (emit-mov-top-stack-from-r9)
               (emit-mov-r9-from-local (native-value-window-spill-offset frame-base-slot-count 35)))
        reg1 (concat-byte-vectors reg0 (emit-mov-r8-from-local (native-value-window-spill-offset frame-base-slot-count 36)))
        reg2 (concat-byte-vectors reg1 (emit-mov-rcx-from-local (native-value-window-spill-offset frame-base-slot-count 37)))
        reg3 (concat-byte-vectors reg2 (emit-mov-rdx-from-local (native-value-window-spill-offset frame-base-slot-count 38)))
        reg4 (concat-byte-vectors reg3 (emit-mov-rsi-from-local (native-value-window-spill-offset frame-base-slot-count 39)))
        reg-setup (concat-byte-vectors reg4 (emit-mov-rdi-from-local (native-value-window-spill-offset frame-base-slot-count 40)))
        call-seq (concat-byte-vectors
                   (emit-call-rel32 rel)
                   (emit-add-rsp-imm32 296))]
    (concat-byte-vectors
      (concat-byte-vectors stack-setup reg-setup)
      call-seq)))

(defn emit-forty-four-arg-call-x86 [rel frame-base-slot-count]
  (let [stack0 (concat-byte-vectors
                 (emit-sub-rsp-imm32 304)
                 (emit-mov-thirty-eighth-stack-from-rax))
        stack1 (concat-byte-vectors stack0 (emit-mov-thirty-seventh-stack-from-rcx))
        stack2 (concat-byte-vectors
                  stack1
                  (emit-mov-rcx-from-local (native-value-window-spill-offset frame-base-slot-count 0)))
        stack3 (concat-byte-vectors stack2 (emit-mov-thirty-sixth-stack-from-rcx))
        stack4 (concat-byte-vectors
                  stack3
                  (emit-mov-rcx-from-local (native-value-window-spill-offset frame-base-slot-count 1)))
        stack5 (concat-byte-vectors stack4 (emit-mov-thirty-fifth-stack-from-rcx))
        stack6 (concat-byte-vectors
                  stack5
                  (emit-mov-rcx-from-local (native-value-window-spill-offset frame-base-slot-count 2)))
        stack7 (concat-byte-vectors stack6 (emit-mov-thirty-fourth-stack-from-rcx))
        stack8 (concat-byte-vectors
                  stack7
                  (emit-mov-rcx-from-local (native-value-window-spill-offset frame-base-slot-count 3)))
        stack9 (concat-byte-vectors stack8 (emit-mov-thirty-third-stack-from-rcx))
        stack10 (concat-byte-vectors
                   stack9
                   (emit-mov-rcx-from-local (native-value-window-spill-offset frame-base-slot-count 4)))
        stack11 (concat-byte-vectors stack10 (emit-mov-thirty-second-stack-from-rcx))
        stack12 (concat-byte-vectors
                   stack11
                   (emit-mov-rcx-from-local (native-value-window-spill-offset frame-base-slot-count 5)))
        stack13 (concat-byte-vectors stack12 (emit-mov-thirty-first-stack-from-rcx))
        stack14 (concat-byte-vectors
                   stack13
                   (emit-mov-rcx-from-local (native-value-window-spill-offset frame-base-slot-count 6)))
        stack15 (concat-byte-vectors stack14 (emit-mov-thirtieth-stack-from-rcx))
        stack16 (concat-byte-vectors
                   stack15
                   (emit-mov-rcx-from-local (native-value-window-spill-offset frame-base-slot-count 7)))
        stack17 (concat-byte-vectors stack16 (emit-mov-twenty-ninth-stack-from-rcx))
        stack18 (concat-byte-vectors
                   stack17
                   (emit-mov-rcx-from-local (native-value-window-spill-offset frame-base-slot-count 8)))
        stack19 (concat-byte-vectors stack18 (emit-mov-twenty-eighth-stack-from-rcx))
        stack20 (concat-byte-vectors
                   stack19
                   (emit-mov-rcx-from-local (native-value-window-spill-offset frame-base-slot-count 9)))
        stack21 (concat-byte-vectors stack20 (emit-mov-twenty-seventh-stack-from-rcx))
        stack22 (concat-byte-vectors
                   stack21
                   (emit-mov-rcx-from-local (native-value-window-spill-offset frame-base-slot-count 10)))
        stack23 (concat-byte-vectors stack22 (emit-mov-twenty-sixth-stack-from-rcx))
        stack24 (concat-byte-vectors
                   stack23
                   (emit-mov-rcx-from-local (native-value-window-spill-offset frame-base-slot-count 11)))
        stack25 (concat-byte-vectors stack24 (emit-mov-twenty-fifth-stack-from-rcx))
        stack26 (concat-byte-vectors
                   stack25
                   (emit-mov-rcx-from-local (native-value-window-spill-offset frame-base-slot-count 12)))
        stack27 (concat-byte-vectors stack26 (emit-mov-twenty-fourth-stack-from-rcx))
        stack28 (concat-byte-vectors
                   stack27
                   (emit-mov-rcx-from-local (native-value-window-spill-offset frame-base-slot-count 13)))
        stack29 (concat-byte-vectors stack28 (emit-mov-twenty-third-stack-from-rcx))
        stack30 (concat-byte-vectors
                   stack29
                   (emit-mov-rcx-from-local (native-value-window-spill-offset frame-base-slot-count 14)))
        stack31 (concat-byte-vectors stack30 (emit-mov-twenty-second-stack-from-rcx))
        stack32 (concat-byte-vectors
                   stack31
                   (emit-mov-rcx-from-local (native-value-window-spill-offset frame-base-slot-count 15)))
        stack33 (concat-byte-vectors stack32 (emit-mov-twenty-first-stack-from-rcx))
        stack34 (concat-byte-vectors
                   stack33
                   (emit-mov-rcx-from-local (native-value-window-spill-offset frame-base-slot-count 16)))
        stack35 (concat-byte-vectors stack34 (emit-mov-twentieth-stack-from-rcx))
        stack36 (concat-byte-vectors
                   stack35
                   (emit-mov-rcx-from-local (native-value-window-spill-offset frame-base-slot-count 17)))
        stack37 (concat-byte-vectors stack36 (emit-mov-nineteenth-stack-from-rcx))
        stack38 (concat-byte-vectors
                   stack37
                   (emit-mov-rcx-from-local (native-value-window-spill-offset frame-base-slot-count 18)))
        stack39 (concat-byte-vectors stack38 (emit-mov-eighteenth-stack-from-rcx))
        stack40 (concat-byte-vectors
                   stack39
                   (emit-mov-rcx-from-local (native-value-window-spill-offset frame-base-slot-count 19)))
        stack41 (concat-byte-vectors stack40 (emit-mov-seventeenth-stack-from-rcx))
        stack42 (concat-byte-vectors
                   stack41
                   (emit-mov-rcx-from-local (native-value-window-spill-offset frame-base-slot-count 20)))
        stack43 (concat-byte-vectors stack42 (emit-mov-sixteenth-stack-from-rcx))
        stack44 (concat-byte-vectors
                   stack43
                   (emit-mov-rcx-from-local (native-value-window-spill-offset frame-base-slot-count 21)))
        stack45 (concat-byte-vectors stack44 (emit-mov-fifteenth-stack-from-rcx))
        stack46 (concat-byte-vectors
                   stack45
                   (emit-mov-rcx-from-local (native-value-window-spill-offset frame-base-slot-count 22)))
        stack47 (concat-byte-vectors stack46 (emit-mov-fourteenth-stack-from-rcx))
        stack48 (concat-byte-vectors
                   stack47
                   (emit-mov-rcx-from-local (native-value-window-spill-offset frame-base-slot-count 23)))
        stack49 (concat-byte-vectors stack48 (emit-mov-thirteenth-stack-from-rcx))
        stack50 (concat-byte-vectors
                   stack49
                   (emit-mov-rcx-from-local (native-value-window-spill-offset frame-base-slot-count 24)))
        stack51 (concat-byte-vectors stack50 (emit-mov-twelfth-stack-from-rcx))
        stack52 (concat-byte-vectors
                   stack51
                   (emit-mov-rcx-from-local (native-value-window-spill-offset frame-base-slot-count 25)))
        stack53 (concat-byte-vectors stack52 (emit-mov-eleventh-stack-from-rcx))
        stack54 (concat-byte-vectors
                   stack53
                   (emit-mov-rcx-from-local (native-value-window-spill-offset frame-base-slot-count 26)))
        stack55 (concat-byte-vectors stack54 (emit-mov-tenth-stack-from-rcx))
        stack56 (concat-byte-vectors
                   stack55
                   (emit-mov-rcx-from-local (native-value-window-spill-offset frame-base-slot-count 27)))
        stack57 (concat-byte-vectors stack56 (emit-mov-ninth-stack-from-rcx))
        stack58 (concat-byte-vectors
                   stack57
                   (emit-mov-rcx-from-local (native-value-window-spill-offset frame-base-slot-count 28)))
        stack59 (concat-byte-vectors stack58 (emit-mov-eighth-stack-from-rcx))
        stack60 (concat-byte-vectors
                   stack59
                   (emit-mov-rcx-from-local (native-value-window-spill-offset frame-base-slot-count 29)))
        stack61 (concat-byte-vectors stack60 (emit-mov-seventh-stack-from-rcx))
        stack62 (concat-byte-vectors
                   stack61
                   (emit-mov-rcx-from-local (native-value-window-spill-offset frame-base-slot-count 30)))
        stack63 (concat-byte-vectors stack62 (emit-mov-sixth-stack-from-rcx))
        stack64 (concat-byte-vectors
                   stack63
                   (emit-mov-rcx-from-local (native-value-window-spill-offset frame-base-slot-count 31)))
        stack65 (concat-byte-vectors stack64 (emit-mov-fifth-stack-from-rcx))
        stack66 (concat-byte-vectors
                   stack65
                   (emit-mov-rcx-from-local (native-value-window-spill-offset frame-base-slot-count 32)))
        stack67 (concat-byte-vectors stack66 (emit-mov-fourth-stack-from-rcx))
        stack68 (concat-byte-vectors
                   stack67
                   (emit-mov-rcx-from-local (native-value-window-spill-offset frame-base-slot-count 33)))
        stack69 (concat-byte-vectors stack68 (emit-mov-third-stack-from-rcx))
        stack70 (concat-byte-vectors
                   stack69
                   (emit-mov-rcx-from-local (native-value-window-spill-offset frame-base-slot-count 34)))
        stack71 (concat-byte-vectors stack70 (emit-mov-second-stack-from-rcx))
        stack-setup (concat-byte-vectors
                      stack71
                      (emit-mov-r9-from-local (native-value-window-spill-offset frame-base-slot-count 35)))
        reg0 (concat-byte-vectors
               (emit-mov-top-stack-from-r9)
               (emit-mov-r9-from-local (native-value-window-spill-offset frame-base-slot-count 36)))
        reg1 (concat-byte-vectors reg0 (emit-mov-r8-from-local (native-value-window-spill-offset frame-base-slot-count 37)))
        reg2 (concat-byte-vectors reg1 (emit-mov-rcx-from-local (native-value-window-spill-offset frame-base-slot-count 38)))
        reg3 (concat-byte-vectors reg2 (emit-mov-rdx-from-local (native-value-window-spill-offset frame-base-slot-count 39)))
        reg4 (concat-byte-vectors reg3 (emit-mov-rsi-from-local (native-value-window-spill-offset frame-base-slot-count 40)))
        reg-setup (concat-byte-vectors reg4 (emit-mov-rdi-from-local (native-value-window-spill-offset frame-base-slot-count 41)))
        call-seq (concat-byte-vectors
                   (emit-call-rel32 rel)
                   (emit-add-rsp-imm32 304))]
    (concat-byte-vectors
      (concat-byte-vectors stack-setup reg-setup)
      call-seq)))

(defn emit-forty-five-arg-call-x86 [rel frame-base-slot-count]
  (let [stack0 (concat-byte-vectors
                 (emit-sub-rsp-imm32 312)
                 (emit-mov-thirty-ninth-stack-from-rax))
        stack1 (concat-byte-vectors stack0 (emit-mov-thirty-eighth-stack-from-rcx))
        stack2 (concat-byte-vectors
                  stack1
                  (emit-mov-rcx-from-local (native-value-window-spill-offset frame-base-slot-count 0)))
        stack3 (concat-byte-vectors stack2 (emit-mov-thirty-seventh-stack-from-rcx))
        stack4 (concat-byte-vectors
                  stack3
                  (emit-mov-rcx-from-local (native-value-window-spill-offset frame-base-slot-count 1)))
        stack5 (concat-byte-vectors stack4 (emit-mov-thirty-sixth-stack-from-rcx))
        stack6 (concat-byte-vectors
                  stack5
                  (emit-mov-rcx-from-local (native-value-window-spill-offset frame-base-slot-count 2)))
        stack7 (concat-byte-vectors stack6 (emit-mov-thirty-fifth-stack-from-rcx))
        stack8 (concat-byte-vectors
                  stack7
                  (emit-mov-rcx-from-local (native-value-window-spill-offset frame-base-slot-count 3)))
        stack9 (concat-byte-vectors stack8 (emit-mov-thirty-fourth-stack-from-rcx))
        stack10 (concat-byte-vectors
                   stack9
                   (emit-mov-rcx-from-local (native-value-window-spill-offset frame-base-slot-count 4)))
        stack11 (concat-byte-vectors stack10 (emit-mov-thirty-third-stack-from-rcx))
        stack12 (concat-byte-vectors
                   stack11
                   (emit-mov-rcx-from-local (native-value-window-spill-offset frame-base-slot-count 5)))
        stack13 (concat-byte-vectors stack12 (emit-mov-thirty-second-stack-from-rcx))
        stack14 (concat-byte-vectors
                   stack13
                   (emit-mov-rcx-from-local (native-value-window-spill-offset frame-base-slot-count 6)))
        stack15 (concat-byte-vectors stack14 (emit-mov-thirty-first-stack-from-rcx))
        stack16 (concat-byte-vectors
                   stack15
                   (emit-mov-rcx-from-local (native-value-window-spill-offset frame-base-slot-count 7)))
        stack17 (concat-byte-vectors stack16 (emit-mov-thirtieth-stack-from-rcx))
        stack18 (concat-byte-vectors
                   stack17
                   (emit-mov-rcx-from-local (native-value-window-spill-offset frame-base-slot-count 8)))
        stack19 (concat-byte-vectors stack18 (emit-mov-twenty-ninth-stack-from-rcx))
        stack20 (concat-byte-vectors
                   stack19
                   (emit-mov-rcx-from-local (native-value-window-spill-offset frame-base-slot-count 9)))
        stack21 (concat-byte-vectors stack20 (emit-mov-twenty-eighth-stack-from-rcx))
        stack22 (concat-byte-vectors
                   stack21
                   (emit-mov-rcx-from-local (native-value-window-spill-offset frame-base-slot-count 10)))
        stack23 (concat-byte-vectors stack22 (emit-mov-twenty-seventh-stack-from-rcx))
        stack24 (concat-byte-vectors
                   stack23
                   (emit-mov-rcx-from-local (native-value-window-spill-offset frame-base-slot-count 11)))
        stack25 (concat-byte-vectors stack24 (emit-mov-twenty-sixth-stack-from-rcx))
        stack26 (concat-byte-vectors
                   stack25
                   (emit-mov-rcx-from-local (native-value-window-spill-offset frame-base-slot-count 12)))
        stack27 (concat-byte-vectors stack26 (emit-mov-twenty-fifth-stack-from-rcx))
        stack28 (concat-byte-vectors
                   stack27
                   (emit-mov-rcx-from-local (native-value-window-spill-offset frame-base-slot-count 13)))
        stack29 (concat-byte-vectors stack28 (emit-mov-twenty-fourth-stack-from-rcx))
        stack30 (concat-byte-vectors
                   stack29
                   (emit-mov-rcx-from-local (native-value-window-spill-offset frame-base-slot-count 14)))
        stack31 (concat-byte-vectors stack30 (emit-mov-twenty-third-stack-from-rcx))
        stack32 (concat-byte-vectors
                   stack31
                   (emit-mov-rcx-from-local (native-value-window-spill-offset frame-base-slot-count 15)))
        stack33 (concat-byte-vectors stack32 (emit-mov-twenty-second-stack-from-rcx))
        stack34 (concat-byte-vectors
                   stack33
                   (emit-mov-rcx-from-local (native-value-window-spill-offset frame-base-slot-count 16)))
        stack35 (concat-byte-vectors stack34 (emit-mov-twenty-first-stack-from-rcx))
        stack36 (concat-byte-vectors
                   stack35
                   (emit-mov-rcx-from-local (native-value-window-spill-offset frame-base-slot-count 17)))
        stack37 (concat-byte-vectors stack36 (emit-mov-twentieth-stack-from-rcx))
        stack38 (concat-byte-vectors
                   stack37
                   (emit-mov-rcx-from-local (native-value-window-spill-offset frame-base-slot-count 18)))
        stack39 (concat-byte-vectors stack38 (emit-mov-nineteenth-stack-from-rcx))
        stack40 (concat-byte-vectors
                   stack39
                   (emit-mov-rcx-from-local (native-value-window-spill-offset frame-base-slot-count 19)))
        stack41 (concat-byte-vectors stack40 (emit-mov-eighteenth-stack-from-rcx))
        stack42 (concat-byte-vectors
                   stack41
                   (emit-mov-rcx-from-local (native-value-window-spill-offset frame-base-slot-count 20)))
        stack43 (concat-byte-vectors stack42 (emit-mov-seventeenth-stack-from-rcx))
        stack44 (concat-byte-vectors
                   stack43
                   (emit-mov-rcx-from-local (native-value-window-spill-offset frame-base-slot-count 21)))
        stack45 (concat-byte-vectors stack44 (emit-mov-sixteenth-stack-from-rcx))
        stack46 (concat-byte-vectors
                   stack45
                   (emit-mov-rcx-from-local (native-value-window-spill-offset frame-base-slot-count 22)))
        stack47 (concat-byte-vectors stack46 (emit-mov-fifteenth-stack-from-rcx))
        stack48 (concat-byte-vectors
                   stack47
                   (emit-mov-rcx-from-local (native-value-window-spill-offset frame-base-slot-count 23)))
        stack49 (concat-byte-vectors stack48 (emit-mov-fourteenth-stack-from-rcx))
        stack50 (concat-byte-vectors
                   stack49
                   (emit-mov-rcx-from-local (native-value-window-spill-offset frame-base-slot-count 24)))
        stack51 (concat-byte-vectors stack50 (emit-mov-thirteenth-stack-from-rcx))
        stack52 (concat-byte-vectors
                   stack51
                   (emit-mov-rcx-from-local (native-value-window-spill-offset frame-base-slot-count 25)))
        stack53 (concat-byte-vectors stack52 (emit-mov-twelfth-stack-from-rcx))
        stack54 (concat-byte-vectors
                   stack53
                   (emit-mov-rcx-from-local (native-value-window-spill-offset frame-base-slot-count 26)))
        stack55 (concat-byte-vectors stack54 (emit-mov-eleventh-stack-from-rcx))
        stack56 (concat-byte-vectors
                   stack55
                   (emit-mov-rcx-from-local (native-value-window-spill-offset frame-base-slot-count 27)))
        stack57 (concat-byte-vectors stack56 (emit-mov-tenth-stack-from-rcx))
        stack58 (concat-byte-vectors
                   stack57
                   (emit-mov-rcx-from-local (native-value-window-spill-offset frame-base-slot-count 28)))
        stack59 (concat-byte-vectors stack58 (emit-mov-ninth-stack-from-rcx))
        stack60 (concat-byte-vectors
                   stack59
                   (emit-mov-rcx-from-local (native-value-window-spill-offset frame-base-slot-count 29)))
        stack61 (concat-byte-vectors stack60 (emit-mov-eighth-stack-from-rcx))
        stack62 (concat-byte-vectors
                   stack61
                   (emit-mov-rcx-from-local (native-value-window-spill-offset frame-base-slot-count 30)))
        stack63 (concat-byte-vectors stack62 (emit-mov-seventh-stack-from-rcx))
        stack64 (concat-byte-vectors
                   stack63
                   (emit-mov-rcx-from-local (native-value-window-spill-offset frame-base-slot-count 31)))
        stack65 (concat-byte-vectors stack64 (emit-mov-sixth-stack-from-rcx))
        stack66 (concat-byte-vectors
                   stack65
                   (emit-mov-rcx-from-local (native-value-window-spill-offset frame-base-slot-count 32)))
        stack67 (concat-byte-vectors stack66 (emit-mov-fifth-stack-from-rcx))
        stack68 (concat-byte-vectors
                   stack67
                   (emit-mov-rcx-from-local (native-value-window-spill-offset frame-base-slot-count 33)))
        stack69 (concat-byte-vectors stack68 (emit-mov-fourth-stack-from-rcx))
        stack70 (concat-byte-vectors
                   stack69
                   (emit-mov-rcx-from-local (native-value-window-spill-offset frame-base-slot-count 34)))
        stack71 (concat-byte-vectors stack70 (emit-mov-third-stack-from-rcx))
        stack72 (concat-byte-vectors
                   stack71
                   (emit-mov-rcx-from-local (native-value-window-spill-offset frame-base-slot-count 35)))
        stack73 (concat-byte-vectors stack72 (emit-mov-second-stack-from-rcx))
        stack-setup (concat-byte-vectors
                      stack73
                      (emit-mov-r9-from-local (native-value-window-spill-offset frame-base-slot-count 36)))
        reg0 (concat-byte-vectors
               (emit-mov-top-stack-from-r9)
               (emit-mov-r9-from-local (native-value-window-spill-offset frame-base-slot-count 37)))
        reg1 (concat-byte-vectors reg0 (emit-mov-r8-from-local (native-value-window-spill-offset frame-base-slot-count 38)))
        reg2 (concat-byte-vectors reg1 (emit-mov-rcx-from-local (native-value-window-spill-offset frame-base-slot-count 39)))
        reg3 (concat-byte-vectors reg2 (emit-mov-rdx-from-local (native-value-window-spill-offset frame-base-slot-count 40)))
        reg4 (concat-byte-vectors reg3 (emit-mov-rsi-from-local (native-value-window-spill-offset frame-base-slot-count 41)))
        reg-setup (concat-byte-vectors reg4 (emit-mov-rdi-from-local (native-value-window-spill-offset frame-base-slot-count 42)))
        call-seq (concat-byte-vectors
                   (emit-call-rel32 rel)
                   (emit-add-rsp-imm32 312))]
    (concat-byte-vectors
      (concat-byte-vectors stack-setup reg-setup)
      call-seq)))

(defn emit-forty-six-arg-call-x86 [rel frame-base-slot-count]
  (let [stack0 (concat-byte-vectors
                 (emit-sub-rsp-imm32 320)
                 (emit-mov-fortieth-stack-from-rax))
        stack1 (concat-byte-vectors stack0 (emit-mov-thirty-ninth-stack-from-rcx))
        stack2 (concat-byte-vectors
                  stack1
                  (emit-mov-rcx-from-local (native-value-window-spill-offset frame-base-slot-count 0)))
        stack3 (concat-byte-vectors stack2 (emit-mov-thirty-eighth-stack-from-rcx))
        stack4 (concat-byte-vectors
                  stack3
                  (emit-mov-rcx-from-local (native-value-window-spill-offset frame-base-slot-count 1)))
        stack5 (concat-byte-vectors stack4 (emit-mov-thirty-seventh-stack-from-rcx))
        stack6 (concat-byte-vectors
                  stack5
                  (emit-mov-rcx-from-local (native-value-window-spill-offset frame-base-slot-count 2)))
        stack7 (concat-byte-vectors stack6 (emit-mov-thirty-sixth-stack-from-rcx))
        stack8 (concat-byte-vectors
                  stack7
                  (emit-mov-rcx-from-local (native-value-window-spill-offset frame-base-slot-count 3)))
        stack9 (concat-byte-vectors stack8 (emit-mov-thirty-fifth-stack-from-rcx))
        stack10 (concat-byte-vectors
                   stack9
                   (emit-mov-rcx-from-local (native-value-window-spill-offset frame-base-slot-count 4)))
        stack11 (concat-byte-vectors stack10 (emit-mov-thirty-fourth-stack-from-rcx))
        stack12 (concat-byte-vectors
                   stack11
                   (emit-mov-rcx-from-local (native-value-window-spill-offset frame-base-slot-count 5)))
        stack13 (concat-byte-vectors stack12 (emit-mov-thirty-third-stack-from-rcx))
        stack14 (concat-byte-vectors
                   stack13
                   (emit-mov-rcx-from-local (native-value-window-spill-offset frame-base-slot-count 6)))
        stack15 (concat-byte-vectors stack14 (emit-mov-thirty-second-stack-from-rcx))
        stack16 (concat-byte-vectors
                   stack15
                   (emit-mov-rcx-from-local (native-value-window-spill-offset frame-base-slot-count 7)))
        stack17 (concat-byte-vectors stack16 (emit-mov-thirty-first-stack-from-rcx))
        stack18 (concat-byte-vectors
                   stack17
                   (emit-mov-rcx-from-local (native-value-window-spill-offset frame-base-slot-count 8)))
        stack19 (concat-byte-vectors stack18 (emit-mov-thirtieth-stack-from-rcx))
        stack20 (concat-byte-vectors
                   stack19
                   (emit-mov-rcx-from-local (native-value-window-spill-offset frame-base-slot-count 9)))
        stack21 (concat-byte-vectors stack20 (emit-mov-twenty-ninth-stack-from-rcx))
        stack22 (concat-byte-vectors
                   stack21
                   (emit-mov-rcx-from-local (native-value-window-spill-offset frame-base-slot-count 10)))
        stack23 (concat-byte-vectors stack22 (emit-mov-twenty-eighth-stack-from-rcx))
        stack24 (concat-byte-vectors
                   stack23
                   (emit-mov-rcx-from-local (native-value-window-spill-offset frame-base-slot-count 11)))
        stack25 (concat-byte-vectors stack24 (emit-mov-twenty-seventh-stack-from-rcx))
        stack26 (concat-byte-vectors
                   stack25
                   (emit-mov-rcx-from-local (native-value-window-spill-offset frame-base-slot-count 12)))
        stack27 (concat-byte-vectors stack26 (emit-mov-twenty-sixth-stack-from-rcx))
        stack28 (concat-byte-vectors
                   stack27
                   (emit-mov-rcx-from-local (native-value-window-spill-offset frame-base-slot-count 13)))
        stack29 (concat-byte-vectors stack28 (emit-mov-twenty-fifth-stack-from-rcx))
        stack30 (concat-byte-vectors
                   stack29
                   (emit-mov-rcx-from-local (native-value-window-spill-offset frame-base-slot-count 14)))
        stack31 (concat-byte-vectors stack30 (emit-mov-twenty-fourth-stack-from-rcx))
        stack32 (concat-byte-vectors
                   stack31
                   (emit-mov-rcx-from-local (native-value-window-spill-offset frame-base-slot-count 15)))
        stack33 (concat-byte-vectors stack32 (emit-mov-twenty-third-stack-from-rcx))
        stack34 (concat-byte-vectors
                   stack33
                   (emit-mov-rcx-from-local (native-value-window-spill-offset frame-base-slot-count 16)))
        stack35 (concat-byte-vectors stack34 (emit-mov-twenty-second-stack-from-rcx))
        stack36 (concat-byte-vectors
                   stack35
                   (emit-mov-rcx-from-local (native-value-window-spill-offset frame-base-slot-count 17)))
        stack37 (concat-byte-vectors stack36 (emit-mov-twenty-first-stack-from-rcx))
        stack38 (concat-byte-vectors
                   stack37
                   (emit-mov-rcx-from-local (native-value-window-spill-offset frame-base-slot-count 18)))
        stack39 (concat-byte-vectors stack38 (emit-mov-twentieth-stack-from-rcx))
        stack40 (concat-byte-vectors
                   stack39
                   (emit-mov-rcx-from-local (native-value-window-spill-offset frame-base-slot-count 19)))
        stack41 (concat-byte-vectors stack40 (emit-mov-nineteenth-stack-from-rcx))
        stack42 (concat-byte-vectors
                   stack41
                   (emit-mov-rcx-from-local (native-value-window-spill-offset frame-base-slot-count 20)))
        stack43 (concat-byte-vectors stack42 (emit-mov-eighteenth-stack-from-rcx))
        stack44 (concat-byte-vectors
                   stack43
                   (emit-mov-rcx-from-local (native-value-window-spill-offset frame-base-slot-count 21)))
        stack45 (concat-byte-vectors stack44 (emit-mov-seventeenth-stack-from-rcx))
        stack46 (concat-byte-vectors
                   stack45
                   (emit-mov-rcx-from-local (native-value-window-spill-offset frame-base-slot-count 22)))
        stack47 (concat-byte-vectors stack46 (emit-mov-sixteenth-stack-from-rcx))
        stack48 (concat-byte-vectors
                   stack47
                   (emit-mov-rcx-from-local (native-value-window-spill-offset frame-base-slot-count 23)))
        stack49 (concat-byte-vectors stack48 (emit-mov-fifteenth-stack-from-rcx))
        stack50 (concat-byte-vectors
                   stack49
                   (emit-mov-rcx-from-local (native-value-window-spill-offset frame-base-slot-count 24)))
        stack51 (concat-byte-vectors stack50 (emit-mov-fourteenth-stack-from-rcx))
        stack52 (concat-byte-vectors
                   stack51
                   (emit-mov-rcx-from-local (native-value-window-spill-offset frame-base-slot-count 25)))
        stack53 (concat-byte-vectors stack52 (emit-mov-thirteenth-stack-from-rcx))
        stack54 (concat-byte-vectors
                   stack53
                   (emit-mov-rcx-from-local (native-value-window-spill-offset frame-base-slot-count 26)))
        stack55 (concat-byte-vectors stack54 (emit-mov-twelfth-stack-from-rcx))
        stack56 (concat-byte-vectors
                   stack55
                   (emit-mov-rcx-from-local (native-value-window-spill-offset frame-base-slot-count 27)))
        stack57 (concat-byte-vectors stack56 (emit-mov-eleventh-stack-from-rcx))
        stack58 (concat-byte-vectors
                   stack57
                   (emit-mov-rcx-from-local (native-value-window-spill-offset frame-base-slot-count 28)))
        stack59 (concat-byte-vectors stack58 (emit-mov-tenth-stack-from-rcx))
        stack60 (concat-byte-vectors
                   stack59
                   (emit-mov-rcx-from-local (native-value-window-spill-offset frame-base-slot-count 29)))
        stack61 (concat-byte-vectors stack60 (emit-mov-ninth-stack-from-rcx))
        stack62 (concat-byte-vectors
                   stack61
                   (emit-mov-rcx-from-local (native-value-window-spill-offset frame-base-slot-count 30)))
        stack63 (concat-byte-vectors stack62 (emit-mov-eighth-stack-from-rcx))
        stack64 (concat-byte-vectors
                   stack63
                   (emit-mov-rcx-from-local (native-value-window-spill-offset frame-base-slot-count 31)))
        stack65 (concat-byte-vectors stack64 (emit-mov-seventh-stack-from-rcx))
        stack66 (concat-byte-vectors
                   stack65
                   (emit-mov-rcx-from-local (native-value-window-spill-offset frame-base-slot-count 32)))
        stack67 (concat-byte-vectors stack66 (emit-mov-sixth-stack-from-rcx))
        stack68 (concat-byte-vectors
                   stack67
                   (emit-mov-rcx-from-local (native-value-window-spill-offset frame-base-slot-count 33)))
        stack69 (concat-byte-vectors stack68 (emit-mov-fifth-stack-from-rcx))
        stack70 (concat-byte-vectors
                   stack69
                   (emit-mov-rcx-from-local (native-value-window-spill-offset frame-base-slot-count 34)))
        stack71 (concat-byte-vectors stack70 (emit-mov-fourth-stack-from-rcx))
        stack72 (concat-byte-vectors
                   stack71
                   (emit-mov-rcx-from-local (native-value-window-spill-offset frame-base-slot-count 35)))
        stack73 (concat-byte-vectors stack72 (emit-mov-third-stack-from-rcx))
        stack74 (concat-byte-vectors
                   stack73
                   (emit-mov-rcx-from-local (native-value-window-spill-offset frame-base-slot-count 36)))
        stack75 (concat-byte-vectors stack74 (emit-mov-second-stack-from-rcx))
        stack-setup (concat-byte-vectors
                      stack75
                      (emit-mov-r9-from-local (native-value-window-spill-offset frame-base-slot-count 37)))
        reg0 (concat-byte-vectors
               (emit-mov-top-stack-from-r9)
               (emit-mov-r9-from-local (native-value-window-spill-offset frame-base-slot-count 38)))
        reg1 (concat-byte-vectors reg0 (emit-mov-r8-from-local (native-value-window-spill-offset frame-base-slot-count 39)))
        reg2 (concat-byte-vectors reg1 (emit-mov-rcx-from-local (native-value-window-spill-offset frame-base-slot-count 40)))
        reg3 (concat-byte-vectors reg2 (emit-mov-rdx-from-local (native-value-window-spill-offset frame-base-slot-count 41)))
        reg4 (concat-byte-vectors reg3 (emit-mov-rsi-from-local (native-value-window-spill-offset frame-base-slot-count 42)))
        reg-setup (concat-byte-vectors reg4 (emit-mov-rdi-from-local (native-value-window-spill-offset frame-base-slot-count 43)))
        call-seq (concat-byte-vectors
                   (emit-call-rel32 rel)
                   (emit-add-rsp-imm32 320))]
    (concat-byte-vectors
      (concat-byte-vectors stack-setup reg-setup)
      call-seq)))

(defn emit-forty-seven-arg-call-x86 [rel frame-base-slot-count]
  (let [stack0 (concat-byte-vectors
                 (emit-sub-rsp-imm32 328)
                 (emit-mov-forty-first-stack-from-rax))
        stack1 (concat-byte-vectors stack0 (emit-mov-fortieth-stack-from-rcx))
        stack2 (concat-byte-vectors
                   stack1
                   (emit-mov-rcx-from-local (native-value-window-spill-offset frame-base-slot-count 0)))
        stack3 (concat-byte-vectors stack2 (emit-mov-thirty-ninth-stack-from-rcx))
        stack4 (concat-byte-vectors
                   stack3
                   (emit-mov-rcx-from-local (native-value-window-spill-offset frame-base-slot-count 1)))
        stack5 (concat-byte-vectors stack4 (emit-mov-thirty-eighth-stack-from-rcx))
        stack6 (concat-byte-vectors
                   stack5
                   (emit-mov-rcx-from-local (native-value-window-spill-offset frame-base-slot-count 2)))
        stack7 (concat-byte-vectors stack6 (emit-mov-thirty-seventh-stack-from-rcx))
        stack8 (concat-byte-vectors
                   stack7
                   (emit-mov-rcx-from-local (native-value-window-spill-offset frame-base-slot-count 3)))
        stack9 (concat-byte-vectors stack8 (emit-mov-thirty-sixth-stack-from-rcx))
        stack10 (concat-byte-vectors
                    stack9
                    (emit-mov-rcx-from-local (native-value-window-spill-offset frame-base-slot-count 4)))
        stack11 (concat-byte-vectors stack10 (emit-mov-thirty-fifth-stack-from-rcx))
        stack12 (concat-byte-vectors
                    stack11
                    (emit-mov-rcx-from-local (native-value-window-spill-offset frame-base-slot-count 5)))
        stack13 (concat-byte-vectors stack12 (emit-mov-thirty-fourth-stack-from-rcx))
        stack14 (concat-byte-vectors
                    stack13
                    (emit-mov-rcx-from-local (native-value-window-spill-offset frame-base-slot-count 6)))
        stack15 (concat-byte-vectors stack14 (emit-mov-thirty-third-stack-from-rcx))
        stack16 (concat-byte-vectors
                    stack15
                    (emit-mov-rcx-from-local (native-value-window-spill-offset frame-base-slot-count 7)))
        stack17 (concat-byte-vectors stack16 (emit-mov-thirty-second-stack-from-rcx))
        stack18 (concat-byte-vectors
                    stack17
                    (emit-mov-rcx-from-local (native-value-window-spill-offset frame-base-slot-count 8)))
        stack19 (concat-byte-vectors stack18 (emit-mov-thirty-first-stack-from-rcx))
        stack20 (concat-byte-vectors
                    stack19
                    (emit-mov-rcx-from-local (native-value-window-spill-offset frame-base-slot-count 9)))
        stack21 (concat-byte-vectors stack20 (emit-mov-thirtieth-stack-from-rcx))
        stack22 (concat-byte-vectors
                    stack21
                    (emit-mov-rcx-from-local (native-value-window-spill-offset frame-base-slot-count 10)))
        stack23 (concat-byte-vectors stack22 (emit-mov-twenty-ninth-stack-from-rcx))
        stack24 (concat-byte-vectors
                    stack23
                    (emit-mov-rcx-from-local (native-value-window-spill-offset frame-base-slot-count 11)))
        stack25 (concat-byte-vectors stack24 (emit-mov-twenty-eighth-stack-from-rcx))
        stack26 (concat-byte-vectors
                    stack25
                    (emit-mov-rcx-from-local (native-value-window-spill-offset frame-base-slot-count 12)))
        stack27 (concat-byte-vectors stack26 (emit-mov-twenty-seventh-stack-from-rcx))
        stack28 (concat-byte-vectors
                    stack27
                    (emit-mov-rcx-from-local (native-value-window-spill-offset frame-base-slot-count 13)))
        stack29 (concat-byte-vectors stack28 (emit-mov-twenty-sixth-stack-from-rcx))
        stack30 (concat-byte-vectors
                    stack29
                    (emit-mov-rcx-from-local (native-value-window-spill-offset frame-base-slot-count 14)))
        stack31 (concat-byte-vectors stack30 (emit-mov-twenty-fifth-stack-from-rcx))
        stack32 (concat-byte-vectors
                    stack31
                    (emit-mov-rcx-from-local (native-value-window-spill-offset frame-base-slot-count 15)))
        stack33 (concat-byte-vectors stack32 (emit-mov-twenty-fourth-stack-from-rcx))
        stack34 (concat-byte-vectors
                    stack33
                    (emit-mov-rcx-from-local (native-value-window-spill-offset frame-base-slot-count 16)))
        stack35 (concat-byte-vectors stack34 (emit-mov-twenty-third-stack-from-rcx))
        stack36 (concat-byte-vectors
                    stack35
                    (emit-mov-rcx-from-local (native-value-window-spill-offset frame-base-slot-count 17)))
        stack37 (concat-byte-vectors stack36 (emit-mov-twenty-second-stack-from-rcx))
        stack38 (concat-byte-vectors
                    stack37
                    (emit-mov-rcx-from-local (native-value-window-spill-offset frame-base-slot-count 18)))
        stack39 (concat-byte-vectors stack38 (emit-mov-twenty-first-stack-from-rcx))
        stack40 (concat-byte-vectors
                    stack39
                    (emit-mov-rcx-from-local (native-value-window-spill-offset frame-base-slot-count 19)))
        stack41 (concat-byte-vectors stack40 (emit-mov-twentieth-stack-from-rcx))
        stack42 (concat-byte-vectors
                    stack41
                    (emit-mov-rcx-from-local (native-value-window-spill-offset frame-base-slot-count 20)))
        stack43 (concat-byte-vectors stack42 (emit-mov-nineteenth-stack-from-rcx))
        stack44 (concat-byte-vectors
                    stack43
                    (emit-mov-rcx-from-local (native-value-window-spill-offset frame-base-slot-count 21)))
        stack45 (concat-byte-vectors stack44 (emit-mov-eighteenth-stack-from-rcx))
        stack46 (concat-byte-vectors
                    stack45
                    (emit-mov-rcx-from-local (native-value-window-spill-offset frame-base-slot-count 22)))
        stack47 (concat-byte-vectors stack46 (emit-mov-seventeenth-stack-from-rcx))
        stack48 (concat-byte-vectors
                    stack47
                    (emit-mov-rcx-from-local (native-value-window-spill-offset frame-base-slot-count 23)))
        stack49 (concat-byte-vectors stack48 (emit-mov-sixteenth-stack-from-rcx))
        stack50 (concat-byte-vectors
                    stack49
                    (emit-mov-rcx-from-local (native-value-window-spill-offset frame-base-slot-count 24)))
        stack51 (concat-byte-vectors stack50 (emit-mov-fifteenth-stack-from-rcx))
        stack52 (concat-byte-vectors
                    stack51
                    (emit-mov-rcx-from-local (native-value-window-spill-offset frame-base-slot-count 25)))
        stack53 (concat-byte-vectors stack52 (emit-mov-fourteenth-stack-from-rcx))
        stack54 (concat-byte-vectors
                    stack53
                    (emit-mov-rcx-from-local (native-value-window-spill-offset frame-base-slot-count 26)))
        stack55 (concat-byte-vectors stack54 (emit-mov-thirteenth-stack-from-rcx))
        stack56 (concat-byte-vectors
                    stack55
                    (emit-mov-rcx-from-local (native-value-window-spill-offset frame-base-slot-count 27)))
        stack57 (concat-byte-vectors stack56 (emit-mov-twelfth-stack-from-rcx))
        stack58 (concat-byte-vectors
                    stack57
                    (emit-mov-rcx-from-local (native-value-window-spill-offset frame-base-slot-count 28)))
        stack59 (concat-byte-vectors stack58 (emit-mov-eleventh-stack-from-rcx))
        stack60 (concat-byte-vectors
                    stack59
                    (emit-mov-rcx-from-local (native-value-window-spill-offset frame-base-slot-count 29)))
        stack61 (concat-byte-vectors stack60 (emit-mov-tenth-stack-from-rcx))
        stack62 (concat-byte-vectors
                    stack61
                    (emit-mov-rcx-from-local (native-value-window-spill-offset frame-base-slot-count 30)))
        stack63 (concat-byte-vectors stack62 (emit-mov-ninth-stack-from-rcx))
        stack64 (concat-byte-vectors
                    stack63
                    (emit-mov-rcx-from-local (native-value-window-spill-offset frame-base-slot-count 31)))
        stack65 (concat-byte-vectors stack64 (emit-mov-eighth-stack-from-rcx))
        stack66 (concat-byte-vectors
                    stack65
                    (emit-mov-rcx-from-local (native-value-window-spill-offset frame-base-slot-count 32)))
        stack67 (concat-byte-vectors stack66 (emit-mov-seventh-stack-from-rcx))
        stack68 (concat-byte-vectors
                    stack67
                    (emit-mov-rcx-from-local (native-value-window-spill-offset frame-base-slot-count 33)))
        stack69 (concat-byte-vectors stack68 (emit-mov-sixth-stack-from-rcx))
        stack70 (concat-byte-vectors
                    stack69
                    (emit-mov-rcx-from-local (native-value-window-spill-offset frame-base-slot-count 34)))
        stack71 (concat-byte-vectors stack70 (emit-mov-fifth-stack-from-rcx))
        stack72 (concat-byte-vectors
                    stack71
                    (emit-mov-rcx-from-local (native-value-window-spill-offset frame-base-slot-count 35)))
        stack73 (concat-byte-vectors stack72 (emit-mov-fourth-stack-from-rcx))
        stack74 (concat-byte-vectors
                    stack73
                    (emit-mov-rcx-from-local (native-value-window-spill-offset frame-base-slot-count 36)))
        stack75 (concat-byte-vectors stack74 (emit-mov-third-stack-from-rcx))
        stack76 (concat-byte-vectors
                    stack75
                    (emit-mov-rcx-from-local (native-value-window-spill-offset frame-base-slot-count 37)))
        stack77 (concat-byte-vectors stack76 (emit-mov-second-stack-from-rcx))
        stack-setup (concat-byte-vectors
                      stack77
                      (emit-mov-r9-from-local (native-value-window-spill-offset frame-base-slot-count 38)))
        reg0 (concat-byte-vectors
               (emit-mov-top-stack-from-r9)
               (emit-mov-r9-from-local (native-value-window-spill-offset frame-base-slot-count 39)))
        reg1 (concat-byte-vectors reg0 (emit-mov-r8-from-local (native-value-window-spill-offset frame-base-slot-count 40)))
        reg2 (concat-byte-vectors reg1 (emit-mov-rcx-from-local (native-value-window-spill-offset frame-base-slot-count 41)))
        reg3 (concat-byte-vectors reg2 (emit-mov-rdx-from-local (native-value-window-spill-offset frame-base-slot-count 42)))
        reg4 (concat-byte-vectors reg3 (emit-mov-rsi-from-local (native-value-window-spill-offset frame-base-slot-count 43)))
        reg-setup (concat-byte-vectors reg4 (emit-mov-rdi-from-local (native-value-window-spill-offset frame-base-slot-count 44)))
        call-seq (concat-byte-vectors
                   (emit-call-rel32 rel)
                   (emit-add-rsp-imm32 328))]
    (concat-byte-vectors
      (concat-byte-vectors stack-setup reg-setup)
      call-seq)))

(defn emit-forty-eight-arg-call-x86 [rel frame-base-slot-count]
  (let [stack0 (concat-byte-vectors
                 (emit-sub-rsp-imm32 336)
                 (emit-mov-forty-second-stack-from-rax))
        stack1 (concat-byte-vectors stack0 (emit-mov-forty-first-stack-from-rcx))
        stack2 (concat-byte-vectors
                   stack1
                   (emit-mov-rcx-from-local (native-value-window-spill-offset frame-base-slot-count 0)))
        stack3 (concat-byte-vectors stack2 (emit-mov-fortieth-stack-from-rcx))
        stack4 (concat-byte-vectors
                   stack3
                   (emit-mov-rcx-from-local (native-value-window-spill-offset frame-base-slot-count 1)))
        stack5 (concat-byte-vectors stack4 (emit-mov-thirty-ninth-stack-from-rcx))
        stack6 (concat-byte-vectors
                   stack5
                   (emit-mov-rcx-from-local (native-value-window-spill-offset frame-base-slot-count 2)))
        stack7 (concat-byte-vectors stack6 (emit-mov-thirty-eighth-stack-from-rcx))
        stack8 (concat-byte-vectors
                   stack7
                   (emit-mov-rcx-from-local (native-value-window-spill-offset frame-base-slot-count 3)))
        stack9 (concat-byte-vectors stack8 (emit-mov-thirty-seventh-stack-from-rcx))
        stack10 (concat-byte-vectors
                    stack9
                    (emit-mov-rcx-from-local (native-value-window-spill-offset frame-base-slot-count 4)))
        stack11 (concat-byte-vectors stack10 (emit-mov-thirty-sixth-stack-from-rcx))
        stack12 (concat-byte-vectors
                    stack11
                    (emit-mov-rcx-from-local (native-value-window-spill-offset frame-base-slot-count 5)))
        stack13 (concat-byte-vectors stack12 (emit-mov-thirty-fifth-stack-from-rcx))
        stack14 (concat-byte-vectors
                    stack13
                    (emit-mov-rcx-from-local (native-value-window-spill-offset frame-base-slot-count 6)))
        stack15 (concat-byte-vectors stack14 (emit-mov-thirty-fourth-stack-from-rcx))
        stack16 (concat-byte-vectors
                    stack15
                    (emit-mov-rcx-from-local (native-value-window-spill-offset frame-base-slot-count 7)))
        stack17 (concat-byte-vectors stack16 (emit-mov-thirty-third-stack-from-rcx))
        stack18 (concat-byte-vectors
                    stack17
                    (emit-mov-rcx-from-local (native-value-window-spill-offset frame-base-slot-count 8)))
        stack19 (concat-byte-vectors stack18 (emit-mov-thirty-second-stack-from-rcx))
        stack20 (concat-byte-vectors
                    stack19
                    (emit-mov-rcx-from-local (native-value-window-spill-offset frame-base-slot-count 9)))
        stack21 (concat-byte-vectors stack20 (emit-mov-thirty-first-stack-from-rcx))
        stack22 (concat-byte-vectors
                    stack21
                    (emit-mov-rcx-from-local (native-value-window-spill-offset frame-base-slot-count 10)))
        stack23 (concat-byte-vectors stack22 (emit-mov-thirtieth-stack-from-rcx))
        stack24 (concat-byte-vectors
                    stack23
                    (emit-mov-rcx-from-local (native-value-window-spill-offset frame-base-slot-count 11)))
        stack25 (concat-byte-vectors stack24 (emit-mov-twenty-ninth-stack-from-rcx))
        stack26 (concat-byte-vectors
                    stack25
                    (emit-mov-rcx-from-local (native-value-window-spill-offset frame-base-slot-count 12)))
        stack27 (concat-byte-vectors stack26 (emit-mov-twenty-eighth-stack-from-rcx))
        stack28 (concat-byte-vectors
                    stack27
                    (emit-mov-rcx-from-local (native-value-window-spill-offset frame-base-slot-count 13)))
        stack29 (concat-byte-vectors stack28 (emit-mov-twenty-seventh-stack-from-rcx))
        stack30 (concat-byte-vectors
                    stack29
                    (emit-mov-rcx-from-local (native-value-window-spill-offset frame-base-slot-count 14)))
        stack31 (concat-byte-vectors stack30 (emit-mov-twenty-sixth-stack-from-rcx))
        stack32 (concat-byte-vectors
                    stack31
                    (emit-mov-rcx-from-local (native-value-window-spill-offset frame-base-slot-count 15)))
        stack33 (concat-byte-vectors stack32 (emit-mov-twenty-fifth-stack-from-rcx))
        stack34 (concat-byte-vectors
                    stack33
                    (emit-mov-rcx-from-local (native-value-window-spill-offset frame-base-slot-count 16)))
        stack35 (concat-byte-vectors stack34 (emit-mov-twenty-fourth-stack-from-rcx))
        stack36 (concat-byte-vectors
                    stack35
                    (emit-mov-rcx-from-local (native-value-window-spill-offset frame-base-slot-count 17)))
        stack37 (concat-byte-vectors stack36 (emit-mov-twenty-third-stack-from-rcx))
        stack38 (concat-byte-vectors
                    stack37
                    (emit-mov-rcx-from-local (native-value-window-spill-offset frame-base-slot-count 18)))
        stack39 (concat-byte-vectors stack38 (emit-mov-twenty-second-stack-from-rcx))
        stack40 (concat-byte-vectors
                    stack39
                    (emit-mov-rcx-from-local (native-value-window-spill-offset frame-base-slot-count 19)))
        stack41 (concat-byte-vectors stack40 (emit-mov-twenty-first-stack-from-rcx))
        stack42 (concat-byte-vectors
                    stack41
                    (emit-mov-rcx-from-local (native-value-window-spill-offset frame-base-slot-count 20)))
        stack43 (concat-byte-vectors stack42 (emit-mov-twentieth-stack-from-rcx))
        stack44 (concat-byte-vectors
                    stack43
                    (emit-mov-rcx-from-local (native-value-window-spill-offset frame-base-slot-count 21)))
        stack45 (concat-byte-vectors stack44 (emit-mov-nineteenth-stack-from-rcx))
        stack46 (concat-byte-vectors
                    stack45
                    (emit-mov-rcx-from-local (native-value-window-spill-offset frame-base-slot-count 22)))
        stack47 (concat-byte-vectors stack46 (emit-mov-eighteenth-stack-from-rcx))
        stack48 (concat-byte-vectors
                    stack47
                    (emit-mov-rcx-from-local (native-value-window-spill-offset frame-base-slot-count 23)))
        stack49 (concat-byte-vectors stack48 (emit-mov-seventeenth-stack-from-rcx))
        stack50 (concat-byte-vectors
                    stack49
                    (emit-mov-rcx-from-local (native-value-window-spill-offset frame-base-slot-count 24)))
        stack51 (concat-byte-vectors stack50 (emit-mov-sixteenth-stack-from-rcx))
        stack52 (concat-byte-vectors
                    stack51
                    (emit-mov-rcx-from-local (native-value-window-spill-offset frame-base-slot-count 25)))
        stack53 (concat-byte-vectors stack52 (emit-mov-fifteenth-stack-from-rcx))
        stack54 (concat-byte-vectors
                    stack53
                    (emit-mov-rcx-from-local (native-value-window-spill-offset frame-base-slot-count 26)))
        stack55 (concat-byte-vectors stack54 (emit-mov-fourteenth-stack-from-rcx))
        stack56 (concat-byte-vectors
                    stack55
                    (emit-mov-rcx-from-local (native-value-window-spill-offset frame-base-slot-count 27)))
        stack57 (concat-byte-vectors stack56 (emit-mov-thirteenth-stack-from-rcx))
        stack58 (concat-byte-vectors
                    stack57
                    (emit-mov-rcx-from-local (native-value-window-spill-offset frame-base-slot-count 28)))
        stack59 (concat-byte-vectors stack58 (emit-mov-twelfth-stack-from-rcx))
        stack60 (concat-byte-vectors
                    stack59
                    (emit-mov-rcx-from-local (native-value-window-spill-offset frame-base-slot-count 29)))
        stack61 (concat-byte-vectors stack60 (emit-mov-eleventh-stack-from-rcx))
        stack62 (concat-byte-vectors
                    stack61
                    (emit-mov-rcx-from-local (native-value-window-spill-offset frame-base-slot-count 30)))
        stack63 (concat-byte-vectors stack62 (emit-mov-tenth-stack-from-rcx))
        stack64 (concat-byte-vectors
                    stack63
                    (emit-mov-rcx-from-local (native-value-window-spill-offset frame-base-slot-count 31)))
        stack65 (concat-byte-vectors stack64 (emit-mov-ninth-stack-from-rcx))
        stack66 (concat-byte-vectors
                    stack65
                    (emit-mov-rcx-from-local (native-value-window-spill-offset frame-base-slot-count 32)))
        stack67 (concat-byte-vectors stack66 (emit-mov-eighth-stack-from-rcx))
        stack68 (concat-byte-vectors
                    stack67
                    (emit-mov-rcx-from-local (native-value-window-spill-offset frame-base-slot-count 33)))
        stack69 (concat-byte-vectors stack68 (emit-mov-seventh-stack-from-rcx))
        stack70 (concat-byte-vectors
                    stack69
                    (emit-mov-rcx-from-local (native-value-window-spill-offset frame-base-slot-count 34)))
        stack71 (concat-byte-vectors stack70 (emit-mov-sixth-stack-from-rcx))
        stack72 (concat-byte-vectors
                    stack71
                    (emit-mov-rcx-from-local (native-value-window-spill-offset frame-base-slot-count 35)))
        stack73 (concat-byte-vectors stack72 (emit-mov-fifth-stack-from-rcx))
        stack74 (concat-byte-vectors
                    stack73
                    (emit-mov-rcx-from-local (native-value-window-spill-offset frame-base-slot-count 36)))
        stack75 (concat-byte-vectors stack74 (emit-mov-fourth-stack-from-rcx))
        stack76 (concat-byte-vectors
                    stack75
                    (emit-mov-rcx-from-local (native-value-window-spill-offset frame-base-slot-count 37)))
        stack77 (concat-byte-vectors stack76 (emit-mov-third-stack-from-rcx))
        stack78 (concat-byte-vectors
                    stack77
                    (emit-mov-rcx-from-local (native-value-window-spill-offset frame-base-slot-count 38)))
        stack79 (concat-byte-vectors stack78 (emit-mov-second-stack-from-rcx))
        stack-setup (concat-byte-vectors
                      stack79
                      (emit-mov-r9-from-local (native-value-window-spill-offset frame-base-slot-count 39)))
        reg0 (concat-byte-vectors
               (emit-mov-top-stack-from-r9)
               (emit-mov-r9-from-local (native-value-window-spill-offset frame-base-slot-count 40)))
        reg1 (concat-byte-vectors reg0 (emit-mov-r8-from-local (native-value-window-spill-offset frame-base-slot-count 41)))
        reg2 (concat-byte-vectors reg1 (emit-mov-rcx-from-local (native-value-window-spill-offset frame-base-slot-count 42)))
        reg3 (concat-byte-vectors reg2 (emit-mov-rdx-from-local (native-value-window-spill-offset frame-base-slot-count 43)))
        reg4 (concat-byte-vectors reg3 (emit-mov-rsi-from-local (native-value-window-spill-offset frame-base-slot-count 44)))
        reg-setup (concat-byte-vectors reg4 (emit-mov-rdi-from-local (native-value-window-spill-offset frame-base-slot-count 45)))
        call-seq (concat-byte-vectors
                   (emit-call-rel32 rel)
                   (emit-add-rsp-imm32 336))]
    (concat-byte-vectors
      (concat-byte-vectors stack-setup reg-setup)
      call-seq)))

(defn emit-forty-nine-arg-call-x86 [rel frame-base-slot-count]
  (let [stack0 (concat-byte-vectors
                 (emit-sub-rsp-imm32 344)
                 (emit-mov-forty-third-stack-from-rax))
        stack1 (concat-byte-vectors stack0 (emit-mov-forty-second-stack-from-rcx))
        stack2 (concat-byte-vectors
                    stack1
                    (emit-mov-rcx-from-local (native-value-window-spill-offset frame-base-slot-count 0)))
        stack3 (concat-byte-vectors stack2 (emit-mov-forty-first-stack-from-rcx))
        stack4 (concat-byte-vectors
                    stack3
                    (emit-mov-rcx-from-local (native-value-window-spill-offset frame-base-slot-count 1)))
        stack5 (concat-byte-vectors stack4 (emit-mov-fortieth-stack-from-rcx))
        stack6 (concat-byte-vectors
                    stack5
                    (emit-mov-rcx-from-local (native-value-window-spill-offset frame-base-slot-count 2)))
        stack7 (concat-byte-vectors stack6 (emit-mov-thirty-ninth-stack-from-rcx))
        stack8 (concat-byte-vectors
                    stack7
                    (emit-mov-rcx-from-local (native-value-window-spill-offset frame-base-slot-count 3)))
        stack9 (concat-byte-vectors stack8 (emit-mov-thirty-eighth-stack-from-rcx))
        stack10 (concat-byte-vectors
                    stack9
                    (emit-mov-rcx-from-local (native-value-window-spill-offset frame-base-slot-count 4)))
        stack11 (concat-byte-vectors stack10 (emit-mov-thirty-seventh-stack-from-rcx))
        stack12 (concat-byte-vectors
                    stack11
                    (emit-mov-rcx-from-local (native-value-window-spill-offset frame-base-slot-count 5)))
        stack13 (concat-byte-vectors stack12 (emit-mov-thirty-sixth-stack-from-rcx))
        stack14 (concat-byte-vectors
                    stack13
                    (emit-mov-rcx-from-local (native-value-window-spill-offset frame-base-slot-count 6)))
        stack15 (concat-byte-vectors stack14 (emit-mov-thirty-fifth-stack-from-rcx))
        stack16 (concat-byte-vectors
                    stack15
                    (emit-mov-rcx-from-local (native-value-window-spill-offset frame-base-slot-count 7)))
        stack17 (concat-byte-vectors stack16 (emit-mov-thirty-fourth-stack-from-rcx))
        stack18 (concat-byte-vectors
                    stack17
                    (emit-mov-rcx-from-local (native-value-window-spill-offset frame-base-slot-count 8)))
        stack19 (concat-byte-vectors stack18 (emit-mov-thirty-third-stack-from-rcx))
        stack20 (concat-byte-vectors
                    stack19
                    (emit-mov-rcx-from-local (native-value-window-spill-offset frame-base-slot-count 9)))
        stack21 (concat-byte-vectors stack20 (emit-mov-thirty-second-stack-from-rcx))
        stack22 (concat-byte-vectors
                    stack21
                    (emit-mov-rcx-from-local (native-value-window-spill-offset frame-base-slot-count 10)))
        stack23 (concat-byte-vectors stack22 (emit-mov-thirty-first-stack-from-rcx))
        stack24 (concat-byte-vectors
                    stack23
                    (emit-mov-rcx-from-local (native-value-window-spill-offset frame-base-slot-count 11)))
        stack25 (concat-byte-vectors stack24 (emit-mov-thirtieth-stack-from-rcx))
        stack26 (concat-byte-vectors
                    stack25
                    (emit-mov-rcx-from-local (native-value-window-spill-offset frame-base-slot-count 12)))
        stack27 (concat-byte-vectors stack26 (emit-mov-twenty-ninth-stack-from-rcx))
        stack28 (concat-byte-vectors
                    stack27
                    (emit-mov-rcx-from-local (native-value-window-spill-offset frame-base-slot-count 13)))
        stack29 (concat-byte-vectors stack28 (emit-mov-twenty-eighth-stack-from-rcx))
        stack30 (concat-byte-vectors
                    stack29
                    (emit-mov-rcx-from-local (native-value-window-spill-offset frame-base-slot-count 14)))
        stack31 (concat-byte-vectors stack30 (emit-mov-twenty-seventh-stack-from-rcx))
        stack32 (concat-byte-vectors
                    stack31
                    (emit-mov-rcx-from-local (native-value-window-spill-offset frame-base-slot-count 15)))
        stack33 (concat-byte-vectors stack32 (emit-mov-twenty-sixth-stack-from-rcx))
        stack34 (concat-byte-vectors
                    stack33
                    (emit-mov-rcx-from-local (native-value-window-spill-offset frame-base-slot-count 16)))
        stack35 (concat-byte-vectors stack34 (emit-mov-twenty-fifth-stack-from-rcx))
        stack36 (concat-byte-vectors
                    stack35
                    (emit-mov-rcx-from-local (native-value-window-spill-offset frame-base-slot-count 17)))
        stack37 (concat-byte-vectors stack36 (emit-mov-twenty-fourth-stack-from-rcx))
        stack38 (concat-byte-vectors
                    stack37
                    (emit-mov-rcx-from-local (native-value-window-spill-offset frame-base-slot-count 18)))
        stack39 (concat-byte-vectors stack38 (emit-mov-twenty-third-stack-from-rcx))
        stack40 (concat-byte-vectors
                    stack39
                    (emit-mov-rcx-from-local (native-value-window-spill-offset frame-base-slot-count 19)))
        stack41 (concat-byte-vectors stack40 (emit-mov-twenty-second-stack-from-rcx))
        stack42 (concat-byte-vectors
                    stack41
                    (emit-mov-rcx-from-local (native-value-window-spill-offset frame-base-slot-count 20)))
        stack43 (concat-byte-vectors stack42 (emit-mov-twenty-first-stack-from-rcx))
        stack44 (concat-byte-vectors
                    stack43
                    (emit-mov-rcx-from-local (native-value-window-spill-offset frame-base-slot-count 21)))
        stack45 (concat-byte-vectors stack44 (emit-mov-twentieth-stack-from-rcx))
        stack46 (concat-byte-vectors
                    stack45
                    (emit-mov-rcx-from-local (native-value-window-spill-offset frame-base-slot-count 22)))
        stack47 (concat-byte-vectors stack46 (emit-mov-nineteenth-stack-from-rcx))
        stack48 (concat-byte-vectors
                    stack47
                    (emit-mov-rcx-from-local (native-value-window-spill-offset frame-base-slot-count 23)))
        stack49 (concat-byte-vectors stack48 (emit-mov-eighteenth-stack-from-rcx))
        stack50 (concat-byte-vectors
                    stack49
                    (emit-mov-rcx-from-local (native-value-window-spill-offset frame-base-slot-count 24)))
        stack51 (concat-byte-vectors stack50 (emit-mov-seventeenth-stack-from-rcx))
        stack52 (concat-byte-vectors
                    stack51
                    (emit-mov-rcx-from-local (native-value-window-spill-offset frame-base-slot-count 25)))
        stack53 (concat-byte-vectors stack52 (emit-mov-sixteenth-stack-from-rcx))
        stack54 (concat-byte-vectors
                    stack53
                    (emit-mov-rcx-from-local (native-value-window-spill-offset frame-base-slot-count 26)))
        stack55 (concat-byte-vectors stack54 (emit-mov-fifteenth-stack-from-rcx))
        stack56 (concat-byte-vectors
                    stack55
                    (emit-mov-rcx-from-local (native-value-window-spill-offset frame-base-slot-count 27)))
        stack57 (concat-byte-vectors stack56 (emit-mov-fourteenth-stack-from-rcx))
        stack58 (concat-byte-vectors
                    stack57
                    (emit-mov-rcx-from-local (native-value-window-spill-offset frame-base-slot-count 28)))
        stack59 (concat-byte-vectors stack58 (emit-mov-thirteenth-stack-from-rcx))
        stack60 (concat-byte-vectors
                    stack59
                    (emit-mov-rcx-from-local (native-value-window-spill-offset frame-base-slot-count 29)))
        stack61 (concat-byte-vectors stack60 (emit-mov-twelfth-stack-from-rcx))
        stack62 (concat-byte-vectors
                    stack61
                    (emit-mov-rcx-from-local (native-value-window-spill-offset frame-base-slot-count 30)))
        stack63 (concat-byte-vectors stack62 (emit-mov-eleventh-stack-from-rcx))
        stack64 (concat-byte-vectors
                    stack63
                    (emit-mov-rcx-from-local (native-value-window-spill-offset frame-base-slot-count 31)))
        stack65 (concat-byte-vectors stack64 (emit-mov-tenth-stack-from-rcx))
        stack66 (concat-byte-vectors
                    stack65
                    (emit-mov-rcx-from-local (native-value-window-spill-offset frame-base-slot-count 32)))
        stack67 (concat-byte-vectors stack66 (emit-mov-ninth-stack-from-rcx))
        stack68 (concat-byte-vectors
                    stack67
                    (emit-mov-rcx-from-local (native-value-window-spill-offset frame-base-slot-count 33)))
        stack69 (concat-byte-vectors stack68 (emit-mov-eighth-stack-from-rcx))
        stack70 (concat-byte-vectors
                    stack69
                    (emit-mov-rcx-from-local (native-value-window-spill-offset frame-base-slot-count 34)))
        stack71 (concat-byte-vectors stack70 (emit-mov-seventh-stack-from-rcx))
        stack72 (concat-byte-vectors
                    stack71
                    (emit-mov-rcx-from-local (native-value-window-spill-offset frame-base-slot-count 35)))
        stack73 (concat-byte-vectors stack72 (emit-mov-sixth-stack-from-rcx))
        stack74 (concat-byte-vectors
                    stack73
                    (emit-mov-rcx-from-local (native-value-window-spill-offset frame-base-slot-count 36)))
        stack75 (concat-byte-vectors stack74 (emit-mov-fifth-stack-from-rcx))
        stack76 (concat-byte-vectors
                    stack75
                    (emit-mov-rcx-from-local (native-value-window-spill-offset frame-base-slot-count 37)))
        stack77 (concat-byte-vectors stack76 (emit-mov-fourth-stack-from-rcx))
        stack78 (concat-byte-vectors
                    stack77
                    (emit-mov-rcx-from-local (native-value-window-spill-offset frame-base-slot-count 38)))
        stack79 (concat-byte-vectors stack78 (emit-mov-third-stack-from-rcx))
        stack80 (concat-byte-vectors
                    stack79
                    (emit-mov-rcx-from-local (native-value-window-spill-offset frame-base-slot-count 39)))
        stack81 (concat-byte-vectors stack80 (emit-mov-second-stack-from-rcx))
        stack-setup (concat-byte-vectors
                      stack81
                      (emit-mov-r9-from-local (native-value-window-spill-offset frame-base-slot-count 40)))
        reg0 (concat-byte-vectors
               (emit-mov-top-stack-from-r9)
               (emit-mov-r9-from-local (native-value-window-spill-offset frame-base-slot-count 41)))
        reg1 (concat-byte-vectors reg0 (emit-mov-r8-from-local (native-value-window-spill-offset frame-base-slot-count 42)))
        reg2 (concat-byte-vectors reg1 (emit-mov-rcx-from-local (native-value-window-spill-offset frame-base-slot-count 43)))
        reg3 (concat-byte-vectors reg2 (emit-mov-rdx-from-local (native-value-window-spill-offset frame-base-slot-count 44)))
        reg4 (concat-byte-vectors reg3 (emit-mov-rsi-from-local (native-value-window-spill-offset frame-base-slot-count 45)))
        reg-setup (concat-byte-vectors reg4 (emit-mov-rdi-from-local (native-value-window-spill-offset frame-base-slot-count 46)))
        call-seq (concat-byte-vectors
                   (emit-call-rel32 rel)
                   (emit-add-rsp-imm32 344))]
    (concat-byte-vectors
      (concat-byte-vectors stack-setup reg-setup)
      call-seq)))

(defn emit-fifty-arg-call-x86 [rel frame-base-slot-count]
  (let [stack0 (concat-byte-vectors
                 (emit-sub-rsp-imm32 352)
                 (emit-mov-forty-fourth-stack-from-rax))
        stack1 (concat-byte-vectors stack0 (emit-mov-forty-third-stack-from-rcx))
        stack2 (concat-byte-vectors
                    stack1
                    (emit-mov-rcx-from-local (native-value-window-spill-offset frame-base-slot-count 0)))
        stack3 (concat-byte-vectors stack2 (emit-mov-forty-second-stack-from-rcx))
        stack4 (concat-byte-vectors
                    stack3
                    (emit-mov-rcx-from-local (native-value-window-spill-offset frame-base-slot-count 1)))
        stack5 (concat-byte-vectors stack4 (emit-mov-forty-first-stack-from-rcx))
        stack6 (concat-byte-vectors
                    stack5
                    (emit-mov-rcx-from-local (native-value-window-spill-offset frame-base-slot-count 2)))
        stack7 (concat-byte-vectors stack6 (emit-mov-fortieth-stack-from-rcx))
        stack8 (concat-byte-vectors
                    stack7
                    (emit-mov-rcx-from-local (native-value-window-spill-offset frame-base-slot-count 3)))
        stack9 (concat-byte-vectors stack8 (emit-mov-thirty-ninth-stack-from-rcx))
        stack10 (concat-byte-vectors
                    stack9
                    (emit-mov-rcx-from-local (native-value-window-spill-offset frame-base-slot-count 4)))
        stack11 (concat-byte-vectors stack10 (emit-mov-thirty-eighth-stack-from-rcx))
        stack12 (concat-byte-vectors
                    stack11
                    (emit-mov-rcx-from-local (native-value-window-spill-offset frame-base-slot-count 5)))
        stack13 (concat-byte-vectors stack12 (emit-mov-thirty-seventh-stack-from-rcx))
        stack14 (concat-byte-vectors
                    stack13
                    (emit-mov-rcx-from-local (native-value-window-spill-offset frame-base-slot-count 6)))
        stack15 (concat-byte-vectors stack14 (emit-mov-thirty-sixth-stack-from-rcx))
        stack16 (concat-byte-vectors
                    stack15
                    (emit-mov-rcx-from-local (native-value-window-spill-offset frame-base-slot-count 7)))
        stack17 (concat-byte-vectors stack16 (emit-mov-thirty-fifth-stack-from-rcx))
        stack18 (concat-byte-vectors
                    stack17
                    (emit-mov-rcx-from-local (native-value-window-spill-offset frame-base-slot-count 8)))
        stack19 (concat-byte-vectors stack18 (emit-mov-thirty-fourth-stack-from-rcx))
        stack20 (concat-byte-vectors
                    stack19
                    (emit-mov-rcx-from-local (native-value-window-spill-offset frame-base-slot-count 9)))
        stack21 (concat-byte-vectors stack20 (emit-mov-thirty-third-stack-from-rcx))
        stack22 (concat-byte-vectors
                    stack21
                    (emit-mov-rcx-from-local (native-value-window-spill-offset frame-base-slot-count 10)))
        stack23 (concat-byte-vectors stack22 (emit-mov-thirty-second-stack-from-rcx))
        stack24 (concat-byte-vectors
                    stack23
                    (emit-mov-rcx-from-local (native-value-window-spill-offset frame-base-slot-count 11)))
        stack25 (concat-byte-vectors stack24 (emit-mov-thirty-first-stack-from-rcx))
        stack26 (concat-byte-vectors
                    stack25
                    (emit-mov-rcx-from-local (native-value-window-spill-offset frame-base-slot-count 12)))
        stack27 (concat-byte-vectors stack26 (emit-mov-thirtieth-stack-from-rcx))
        stack28 (concat-byte-vectors
                    stack27
                    (emit-mov-rcx-from-local (native-value-window-spill-offset frame-base-slot-count 13)))
        stack29 (concat-byte-vectors stack28 (emit-mov-twenty-ninth-stack-from-rcx))
        stack30 (concat-byte-vectors
                    stack29
                    (emit-mov-rcx-from-local (native-value-window-spill-offset frame-base-slot-count 14)))
        stack31 (concat-byte-vectors stack30 (emit-mov-twenty-eighth-stack-from-rcx))
        stack32 (concat-byte-vectors
                    stack31
                    (emit-mov-rcx-from-local (native-value-window-spill-offset frame-base-slot-count 15)))
        stack33 (concat-byte-vectors stack32 (emit-mov-twenty-seventh-stack-from-rcx))
        stack34 (concat-byte-vectors
                    stack33
                    (emit-mov-rcx-from-local (native-value-window-spill-offset frame-base-slot-count 16)))
        stack35 (concat-byte-vectors stack34 (emit-mov-twenty-sixth-stack-from-rcx))
        stack36 (concat-byte-vectors
                    stack35
                    (emit-mov-rcx-from-local (native-value-window-spill-offset frame-base-slot-count 17)))
        stack37 (concat-byte-vectors stack36 (emit-mov-twenty-fifth-stack-from-rcx))
        stack38 (concat-byte-vectors
                    stack37
                    (emit-mov-rcx-from-local (native-value-window-spill-offset frame-base-slot-count 18)))
        stack39 (concat-byte-vectors stack38 (emit-mov-twenty-fourth-stack-from-rcx))
        stack40 (concat-byte-vectors
                    stack39
                    (emit-mov-rcx-from-local (native-value-window-spill-offset frame-base-slot-count 19)))
        stack41 (concat-byte-vectors stack40 (emit-mov-twenty-third-stack-from-rcx))
        stack42 (concat-byte-vectors
                    stack41
                    (emit-mov-rcx-from-local (native-value-window-spill-offset frame-base-slot-count 20)))
        stack43 (concat-byte-vectors stack42 (emit-mov-twenty-second-stack-from-rcx))
        stack44 (concat-byte-vectors
                    stack43
                    (emit-mov-rcx-from-local (native-value-window-spill-offset frame-base-slot-count 21)))
        stack45 (concat-byte-vectors stack44 (emit-mov-twenty-first-stack-from-rcx))
        stack46 (concat-byte-vectors
                    stack45
                    (emit-mov-rcx-from-local (native-value-window-spill-offset frame-base-slot-count 22)))
        stack47 (concat-byte-vectors stack46 (emit-mov-twentieth-stack-from-rcx))
        stack48 (concat-byte-vectors
                    stack47
                    (emit-mov-rcx-from-local (native-value-window-spill-offset frame-base-slot-count 23)))
        stack49 (concat-byte-vectors stack48 (emit-mov-nineteenth-stack-from-rcx))
        stack50 (concat-byte-vectors
                    stack49
                    (emit-mov-rcx-from-local (native-value-window-spill-offset frame-base-slot-count 24)))
        stack51 (concat-byte-vectors stack50 (emit-mov-eighteenth-stack-from-rcx))
        stack52 (concat-byte-vectors
                    stack51
                    (emit-mov-rcx-from-local (native-value-window-spill-offset frame-base-slot-count 25)))
        stack53 (concat-byte-vectors stack52 (emit-mov-seventeenth-stack-from-rcx))
        stack54 (concat-byte-vectors
                    stack53
                    (emit-mov-rcx-from-local (native-value-window-spill-offset frame-base-slot-count 26)))
        stack55 (concat-byte-vectors stack54 (emit-mov-sixteenth-stack-from-rcx))
        stack56 (concat-byte-vectors
                    stack55
                    (emit-mov-rcx-from-local (native-value-window-spill-offset frame-base-slot-count 27)))
        stack57 (concat-byte-vectors stack56 (emit-mov-fifteenth-stack-from-rcx))
        stack58 (concat-byte-vectors
                    stack57
                    (emit-mov-rcx-from-local (native-value-window-spill-offset frame-base-slot-count 28)))
        stack59 (concat-byte-vectors stack58 (emit-mov-fourteenth-stack-from-rcx))
        stack60 (concat-byte-vectors
                    stack59
                    (emit-mov-rcx-from-local (native-value-window-spill-offset frame-base-slot-count 29)))
        stack61 (concat-byte-vectors stack60 (emit-mov-thirteenth-stack-from-rcx))
        stack62 (concat-byte-vectors
                    stack61
                    (emit-mov-rcx-from-local (native-value-window-spill-offset frame-base-slot-count 30)))
        stack63 (concat-byte-vectors stack62 (emit-mov-twelfth-stack-from-rcx))
        stack64 (concat-byte-vectors
                    stack63
                    (emit-mov-rcx-from-local (native-value-window-spill-offset frame-base-slot-count 31)))
        stack65 (concat-byte-vectors stack64 (emit-mov-eleventh-stack-from-rcx))
        stack66 (concat-byte-vectors
                    stack65
                    (emit-mov-rcx-from-local (native-value-window-spill-offset frame-base-slot-count 32)))
        stack67 (concat-byte-vectors stack66 (emit-mov-tenth-stack-from-rcx))
        stack68 (concat-byte-vectors
                    stack67
                    (emit-mov-rcx-from-local (native-value-window-spill-offset frame-base-slot-count 33)))
        stack69 (concat-byte-vectors stack68 (emit-mov-ninth-stack-from-rcx))
        stack70 (concat-byte-vectors
                    stack69
                    (emit-mov-rcx-from-local (native-value-window-spill-offset frame-base-slot-count 34)))
        stack71 (concat-byte-vectors stack70 (emit-mov-eighth-stack-from-rcx))
        stack72 (concat-byte-vectors
                    stack71
                    (emit-mov-rcx-from-local (native-value-window-spill-offset frame-base-slot-count 35)))
        stack73 (concat-byte-vectors stack72 (emit-mov-seventh-stack-from-rcx))
        stack74 (concat-byte-vectors
                    stack73
                    (emit-mov-rcx-from-local (native-value-window-spill-offset frame-base-slot-count 36)))
        stack75 (concat-byte-vectors stack74 (emit-mov-sixth-stack-from-rcx))
        stack76 (concat-byte-vectors
                    stack75
                    (emit-mov-rcx-from-local (native-value-window-spill-offset frame-base-slot-count 37)))
        stack77 (concat-byte-vectors stack76 (emit-mov-fifth-stack-from-rcx))
        stack78 (concat-byte-vectors
                    stack77
                    (emit-mov-rcx-from-local (native-value-window-spill-offset frame-base-slot-count 38)))
        stack79 (concat-byte-vectors stack78 (emit-mov-fourth-stack-from-rcx))
        stack80 (concat-byte-vectors
                    stack79
                    (emit-mov-rcx-from-local (native-value-window-spill-offset frame-base-slot-count 39)))
        stack81 (concat-byte-vectors stack80 (emit-mov-third-stack-from-rcx))
        stack82 (concat-byte-vectors
                    stack81
                    (emit-mov-rcx-from-local (native-value-window-spill-offset frame-base-slot-count 40)))
        stack83 (concat-byte-vectors stack82 (emit-mov-second-stack-from-rcx))
        stack-setup (concat-byte-vectors
                      stack83
                      (emit-mov-r9-from-local (native-value-window-spill-offset frame-base-slot-count 41)))
        reg0 (concat-byte-vectors
               (emit-mov-top-stack-from-r9)
               (emit-mov-r9-from-local (native-value-window-spill-offset frame-base-slot-count 42)))
        reg1 (concat-byte-vectors reg0 (emit-mov-r8-from-local (native-value-window-spill-offset frame-base-slot-count 43)))
        reg2 (concat-byte-vectors reg1 (emit-mov-rcx-from-local (native-value-window-spill-offset frame-base-slot-count 44)))
        reg3 (concat-byte-vectors reg2 (emit-mov-rdx-from-local (native-value-window-spill-offset frame-base-slot-count 45)))
        reg4 (concat-byte-vectors reg3 (emit-mov-rsi-from-local (native-value-window-spill-offset frame-base-slot-count 46)))
        reg-setup (concat-byte-vectors reg4 (emit-mov-rdi-from-local (native-value-window-spill-offset frame-base-slot-count 47)))
        call-seq (concat-byte-vectors
                   (emit-call-rel32 rel)
                   (emit-add-rsp-imm32 352))]
    (concat-byte-vectors
      (concat-byte-vectors stack-setup reg-setup)
      call-seq)))

(defn emit-fifty-one-arg-call-x86 [rel frame-base-slot-count]
  (let [stack0 (concat-byte-vectors
                 (emit-sub-rsp-imm32 360)
                 (emit-mov-stack-slot-from-rax 352))
        stack1 (concat-byte-vectors stack0 (emit-mov-stack-slot-from-rcx 344))
        stack2 (concat-byte-vectors
                    stack1
                    (emit-mov-rcx-from-local (native-value-window-spill-offset frame-base-slot-count 0)))
        stack3 (concat-byte-vectors stack2 (emit-mov-stack-slot-from-rcx 336))
        stack4 (concat-byte-vectors
                    stack3
                    (emit-mov-rcx-from-local (native-value-window-spill-offset frame-base-slot-count 1)))
        stack5 (concat-byte-vectors stack4 (emit-mov-stack-slot-from-rcx 328))
        stack6 (concat-byte-vectors
                    stack5
                    (emit-mov-rcx-from-local (native-value-window-spill-offset frame-base-slot-count 2)))
        stack7 (concat-byte-vectors stack6 (emit-mov-stack-slot-from-rcx 320))
        stack8 (concat-byte-vectors
                    stack7
                    (emit-mov-rcx-from-local (native-value-window-spill-offset frame-base-slot-count 3)))
        stack9 (concat-byte-vectors stack8 (emit-mov-stack-slot-from-rcx 312))
        stack10 (concat-byte-vectors
                    stack9
                    (emit-mov-rcx-from-local (native-value-window-spill-offset frame-base-slot-count 4)))
        stack11 (concat-byte-vectors stack10 (emit-mov-stack-slot-from-rcx 304))
        stack12 (concat-byte-vectors
                    stack11
                    (emit-mov-rcx-from-local (native-value-window-spill-offset frame-base-slot-count 5)))
        stack13 (concat-byte-vectors stack12 (emit-mov-stack-slot-from-rcx 296))
        stack14 (concat-byte-vectors
                    stack13
                    (emit-mov-rcx-from-local (native-value-window-spill-offset frame-base-slot-count 6)))
        stack15 (concat-byte-vectors stack14 (emit-mov-stack-slot-from-rcx 288))
        stack16 (concat-byte-vectors
                    stack15
                    (emit-mov-rcx-from-local (native-value-window-spill-offset frame-base-slot-count 7)))
        stack17 (concat-byte-vectors stack16 (emit-mov-stack-slot-from-rcx 280))
        stack18 (concat-byte-vectors
                    stack17
                    (emit-mov-rcx-from-local (native-value-window-spill-offset frame-base-slot-count 8)))
        stack19 (concat-byte-vectors stack18 (emit-mov-stack-slot-from-rcx 272))
        stack20 (concat-byte-vectors
                    stack19
                    (emit-mov-rcx-from-local (native-value-window-spill-offset frame-base-slot-count 9)))
        stack21 (concat-byte-vectors stack20 (emit-mov-stack-slot-from-rcx 264))
        stack22 (concat-byte-vectors
                    stack21
                    (emit-mov-rcx-from-local (native-value-window-spill-offset frame-base-slot-count 10)))
        stack23 (concat-byte-vectors stack22 (emit-mov-stack-slot-from-rcx 256))
        stack24 (concat-byte-vectors
                    stack23
                    (emit-mov-rcx-from-local (native-value-window-spill-offset frame-base-slot-count 11)))
        stack25 (concat-byte-vectors stack24 (emit-mov-stack-slot-from-rcx 248))
        stack26 (concat-byte-vectors
                    stack25
                    (emit-mov-rcx-from-local (native-value-window-spill-offset frame-base-slot-count 12)))
        stack27 (concat-byte-vectors stack26 (emit-mov-stack-slot-from-rcx 240))
        stack28 (concat-byte-vectors
                    stack27
                    (emit-mov-rcx-from-local (native-value-window-spill-offset frame-base-slot-count 13)))
        stack29 (concat-byte-vectors stack28 (emit-mov-stack-slot-from-rcx 232))
        stack30 (concat-byte-vectors
                    stack29
                    (emit-mov-rcx-from-local (native-value-window-spill-offset frame-base-slot-count 14)))
        stack31 (concat-byte-vectors stack30 (emit-mov-stack-slot-from-rcx 224))
        stack32 (concat-byte-vectors
                    stack31
                    (emit-mov-rcx-from-local (native-value-window-spill-offset frame-base-slot-count 15)))
        stack33 (concat-byte-vectors stack32 (emit-mov-stack-slot-from-rcx 216))
        stack34 (concat-byte-vectors
                    stack33
                    (emit-mov-rcx-from-local (native-value-window-spill-offset frame-base-slot-count 16)))
        stack35 (concat-byte-vectors stack34 (emit-mov-stack-slot-from-rcx 208))
        stack36 (concat-byte-vectors
                    stack35
                    (emit-mov-rcx-from-local (native-value-window-spill-offset frame-base-slot-count 17)))
        stack37 (concat-byte-vectors stack36 (emit-mov-stack-slot-from-rcx 200))
        stack38 (concat-byte-vectors
                    stack37
                    (emit-mov-rcx-from-local (native-value-window-spill-offset frame-base-slot-count 18)))
        stack39 (concat-byte-vectors stack38 (emit-mov-stack-slot-from-rcx 192))
        stack40 (concat-byte-vectors
                    stack39
                    (emit-mov-rcx-from-local (native-value-window-spill-offset frame-base-slot-count 19)))
        stack41 (concat-byte-vectors stack40 (emit-mov-stack-slot-from-rcx 184))
        stack42 (concat-byte-vectors
                    stack41
                    (emit-mov-rcx-from-local (native-value-window-spill-offset frame-base-slot-count 20)))
        stack43 (concat-byte-vectors stack42 (emit-mov-stack-slot-from-rcx 176))
        stack44 (concat-byte-vectors
                    stack43
                    (emit-mov-rcx-from-local (native-value-window-spill-offset frame-base-slot-count 21)))
        stack45 (concat-byte-vectors stack44 (emit-mov-stack-slot-from-rcx 168))
        stack46 (concat-byte-vectors
                    stack45
                    (emit-mov-rcx-from-local (native-value-window-spill-offset frame-base-slot-count 22)))
        stack47 (concat-byte-vectors stack46 (emit-mov-stack-slot-from-rcx 160))
        stack48 (concat-byte-vectors
                    stack47
                    (emit-mov-rcx-from-local (native-value-window-spill-offset frame-base-slot-count 23)))
        stack49 (concat-byte-vectors stack48 (emit-mov-stack-slot-from-rcx 152))
        stack50 (concat-byte-vectors
                    stack49
                    (emit-mov-rcx-from-local (native-value-window-spill-offset frame-base-slot-count 24)))
        stack51 (concat-byte-vectors stack50 (emit-mov-stack-slot-from-rcx 144))
        stack52 (concat-byte-vectors
                    stack51
                    (emit-mov-rcx-from-local (native-value-window-spill-offset frame-base-slot-count 25)))
        stack53 (concat-byte-vectors stack52 (emit-mov-stack-slot-from-rcx 136))
        stack54 (concat-byte-vectors
                    stack53
                    (emit-mov-rcx-from-local (native-value-window-spill-offset frame-base-slot-count 26)))
        stack55 (concat-byte-vectors stack54 (emit-mov-stack-slot-from-rcx 128))
        stack56 (concat-byte-vectors
                    stack55
                    (emit-mov-rcx-from-local (native-value-window-spill-offset frame-base-slot-count 27)))
        stack57 (concat-byte-vectors stack56 (emit-mov-stack-slot-from-rcx 120))
        stack58 (concat-byte-vectors
                    stack57
                    (emit-mov-rcx-from-local (native-value-window-spill-offset frame-base-slot-count 28)))
        stack59 (concat-byte-vectors stack58 (emit-mov-stack-slot-from-rcx 112))
        stack60 (concat-byte-vectors
                    stack59
                    (emit-mov-rcx-from-local (native-value-window-spill-offset frame-base-slot-count 29)))
        stack61 (concat-byte-vectors stack60 (emit-mov-stack-slot-from-rcx 104))
        stack62 (concat-byte-vectors
                    stack61
                    (emit-mov-rcx-from-local (native-value-window-spill-offset frame-base-slot-count 30)))
        stack63 (concat-byte-vectors stack62 (emit-mov-stack-slot-from-rcx 96))
        stack64 (concat-byte-vectors
                    stack63
                    (emit-mov-rcx-from-local (native-value-window-spill-offset frame-base-slot-count 31)))
        stack65 (concat-byte-vectors stack64 (emit-mov-stack-slot-from-rcx 88))
        stack66 (concat-byte-vectors
                    stack65
                    (emit-mov-rcx-from-local (native-value-window-spill-offset frame-base-slot-count 32)))
        stack67 (concat-byte-vectors stack66 (emit-mov-stack-slot-from-rcx 80))
        stack68 (concat-byte-vectors
                    stack67
                    (emit-mov-rcx-from-local (native-value-window-spill-offset frame-base-slot-count 33)))
        stack69 (concat-byte-vectors stack68 (emit-mov-stack-slot-from-rcx 72))
        stack70 (concat-byte-vectors
                    stack69
                    (emit-mov-rcx-from-local (native-value-window-spill-offset frame-base-slot-count 34)))
        stack71 (concat-byte-vectors stack70 (emit-mov-stack-slot-from-rcx 64))
        stack72 (concat-byte-vectors
                    stack71
                    (emit-mov-rcx-from-local (native-value-window-spill-offset frame-base-slot-count 35)))
        stack73 (concat-byte-vectors stack72 (emit-mov-stack-slot-from-rcx 56))
        stack74 (concat-byte-vectors
                    stack73
                    (emit-mov-rcx-from-local (native-value-window-spill-offset frame-base-slot-count 36)))
        stack75 (concat-byte-vectors stack74 (emit-mov-stack-slot-from-rcx 48))
        stack76 (concat-byte-vectors
                    stack75
                    (emit-mov-rcx-from-local (native-value-window-spill-offset frame-base-slot-count 37)))
        stack77 (concat-byte-vectors stack76 (emit-mov-stack-slot-from-rcx 40))
        stack78 (concat-byte-vectors
                    stack77
                    (emit-mov-rcx-from-local (native-value-window-spill-offset frame-base-slot-count 38)))
        stack79 (concat-byte-vectors stack78 (emit-mov-stack-slot-from-rcx 32))
        stack80 (concat-byte-vectors
                    stack79
                    (emit-mov-rcx-from-local (native-value-window-spill-offset frame-base-slot-count 39)))
        stack81 (concat-byte-vectors stack80 (emit-mov-stack-slot-from-rcx 24))
        stack82 (concat-byte-vectors
                    stack81
                    (emit-mov-rcx-from-local (native-value-window-spill-offset frame-base-slot-count 40)))
        stack83 (concat-byte-vectors stack82 (emit-mov-stack-slot-from-rcx 16))
        stack84 (concat-byte-vectors
                    stack83
                    (emit-mov-rcx-from-local (native-value-window-spill-offset frame-base-slot-count 41)))
        stack85 (concat-byte-vectors stack84 (emit-mov-stack-slot-from-rcx 8))
        stack-setup (concat-byte-vectors
                      stack85
                      (emit-mov-r9-from-local (native-value-window-spill-offset frame-base-slot-count 42)))
        reg0 (concat-byte-vectors
               (emit-mov-top-stack-from-r9)
               (emit-mov-r9-from-local (native-value-window-spill-offset frame-base-slot-count 43)))
        reg1 (concat-byte-vectors reg0 (emit-mov-r8-from-local (native-value-window-spill-offset frame-base-slot-count 44)))
        reg2 (concat-byte-vectors reg1 (emit-mov-rcx-from-local (native-value-window-spill-offset frame-base-slot-count 45)))
        reg3 (concat-byte-vectors reg2 (emit-mov-rdx-from-local (native-value-window-spill-offset frame-base-slot-count 46)))
        reg4 (concat-byte-vectors reg3 (emit-mov-rsi-from-local (native-value-window-spill-offset frame-base-slot-count 47)))
        reg-setup (concat-byte-vectors reg4 (emit-mov-rdi-from-local (native-value-window-spill-offset frame-base-slot-count 48)))
        call-seq (concat-byte-vectors
                   (emit-call-rel32 rel)
                   (emit-add-rsp-imm32 360))]
    (concat-byte-vectors
      (concat-byte-vectors stack-setup reg-setup)
      call-seq)))

(defn emit-fifty-two-arg-call-x86 [rel frame-base-slot-count]
  (let [stack0 (concat-byte-vectors
                 (emit-sub-rsp-imm32 368)
                 (emit-mov-stack-slot-from-rax 360))
        stack1 (concat-byte-vectors stack0 (emit-mov-stack-slot-from-rcx 352))
        stack2 (concat-byte-vectors
                    stack1
                    (emit-mov-rcx-from-local (native-value-window-spill-offset frame-base-slot-count 0)))
        stack3 (concat-byte-vectors stack2 (emit-mov-stack-slot-from-rcx 344))
        stack4 (concat-byte-vectors
                    stack3
                    (emit-mov-rcx-from-local (native-value-window-spill-offset frame-base-slot-count 1)))
        stack5 (concat-byte-vectors stack4 (emit-mov-stack-slot-from-rcx 336))
        stack6 (concat-byte-vectors
                    stack5
                    (emit-mov-rcx-from-local (native-value-window-spill-offset frame-base-slot-count 2)))
        stack7 (concat-byte-vectors stack6 (emit-mov-stack-slot-from-rcx 328))
        stack8 (concat-byte-vectors
                    stack7
                    (emit-mov-rcx-from-local (native-value-window-spill-offset frame-base-slot-count 3)))
        stack9 (concat-byte-vectors stack8 (emit-mov-stack-slot-from-rcx 320))
        stack10 (concat-byte-vectors
                    stack9
                    (emit-mov-rcx-from-local (native-value-window-spill-offset frame-base-slot-count 4)))
        stack11 (concat-byte-vectors stack10 (emit-mov-stack-slot-from-rcx 312))
        stack12 (concat-byte-vectors
                    stack11
                    (emit-mov-rcx-from-local (native-value-window-spill-offset frame-base-slot-count 5)))
        stack13 (concat-byte-vectors stack12 (emit-mov-stack-slot-from-rcx 304))
        stack14 (concat-byte-vectors
                    stack13
                    (emit-mov-rcx-from-local (native-value-window-spill-offset frame-base-slot-count 6)))
        stack15 (concat-byte-vectors stack14 (emit-mov-stack-slot-from-rcx 296))
        stack16 (concat-byte-vectors
                    stack15
                    (emit-mov-rcx-from-local (native-value-window-spill-offset frame-base-slot-count 7)))
        stack17 (concat-byte-vectors stack16 (emit-mov-stack-slot-from-rcx 288))
        stack18 (concat-byte-vectors
                    stack17
                    (emit-mov-rcx-from-local (native-value-window-spill-offset frame-base-slot-count 8)))
        stack19 (concat-byte-vectors stack18 (emit-mov-stack-slot-from-rcx 280))
        stack20 (concat-byte-vectors
                    stack19
                    (emit-mov-rcx-from-local (native-value-window-spill-offset frame-base-slot-count 9)))
        stack21 (concat-byte-vectors stack20 (emit-mov-stack-slot-from-rcx 272))
        stack22 (concat-byte-vectors
                    stack21
                    (emit-mov-rcx-from-local (native-value-window-spill-offset frame-base-slot-count 10)))
        stack23 (concat-byte-vectors stack22 (emit-mov-stack-slot-from-rcx 264))
        stack24 (concat-byte-vectors
                    stack23
                    (emit-mov-rcx-from-local (native-value-window-spill-offset frame-base-slot-count 11)))
        stack25 (concat-byte-vectors stack24 (emit-mov-stack-slot-from-rcx 256))
        stack26 (concat-byte-vectors
                    stack25
                    (emit-mov-rcx-from-local (native-value-window-spill-offset frame-base-slot-count 12)))
        stack27 (concat-byte-vectors stack26 (emit-mov-stack-slot-from-rcx 248))
        stack28 (concat-byte-vectors
                    stack27
                    (emit-mov-rcx-from-local (native-value-window-spill-offset frame-base-slot-count 13)))
        stack29 (concat-byte-vectors stack28 (emit-mov-stack-slot-from-rcx 240))
        stack30 (concat-byte-vectors
                    stack29
                    (emit-mov-rcx-from-local (native-value-window-spill-offset frame-base-slot-count 14)))
        stack31 (concat-byte-vectors stack30 (emit-mov-stack-slot-from-rcx 232))
        stack32 (concat-byte-vectors
                    stack31
                    (emit-mov-rcx-from-local (native-value-window-spill-offset frame-base-slot-count 15)))
        stack33 (concat-byte-vectors stack32 (emit-mov-stack-slot-from-rcx 224))
        stack34 (concat-byte-vectors
                    stack33
                    (emit-mov-rcx-from-local (native-value-window-spill-offset frame-base-slot-count 16)))
        stack35 (concat-byte-vectors stack34 (emit-mov-stack-slot-from-rcx 216))
        stack36 (concat-byte-vectors
                    stack35
                    (emit-mov-rcx-from-local (native-value-window-spill-offset frame-base-slot-count 17)))
        stack37 (concat-byte-vectors stack36 (emit-mov-stack-slot-from-rcx 208))
        stack38 (concat-byte-vectors
                    stack37
                    (emit-mov-rcx-from-local (native-value-window-spill-offset frame-base-slot-count 18)))
        stack39 (concat-byte-vectors stack38 (emit-mov-stack-slot-from-rcx 200))
        stack40 (concat-byte-vectors
                    stack39
                    (emit-mov-rcx-from-local (native-value-window-spill-offset frame-base-slot-count 19)))
        stack41 (concat-byte-vectors stack40 (emit-mov-stack-slot-from-rcx 192))
        stack42 (concat-byte-vectors
                    stack41
                    (emit-mov-rcx-from-local (native-value-window-spill-offset frame-base-slot-count 20)))
        stack43 (concat-byte-vectors stack42 (emit-mov-stack-slot-from-rcx 184))
        stack44 (concat-byte-vectors
                    stack43
                    (emit-mov-rcx-from-local (native-value-window-spill-offset frame-base-slot-count 21)))
        stack45 (concat-byte-vectors stack44 (emit-mov-stack-slot-from-rcx 176))
        stack46 (concat-byte-vectors
                    stack45
                    (emit-mov-rcx-from-local (native-value-window-spill-offset frame-base-slot-count 22)))
        stack47 (concat-byte-vectors stack46 (emit-mov-stack-slot-from-rcx 168))
        stack48 (concat-byte-vectors
                    stack47
                    (emit-mov-rcx-from-local (native-value-window-spill-offset frame-base-slot-count 23)))
        stack49 (concat-byte-vectors stack48 (emit-mov-stack-slot-from-rcx 160))
        stack50 (concat-byte-vectors
                    stack49
                    (emit-mov-rcx-from-local (native-value-window-spill-offset frame-base-slot-count 24)))
        stack51 (concat-byte-vectors stack50 (emit-mov-stack-slot-from-rcx 152))
        stack52 (concat-byte-vectors
                    stack51
                    (emit-mov-rcx-from-local (native-value-window-spill-offset frame-base-slot-count 25)))
        stack53 (concat-byte-vectors stack52 (emit-mov-stack-slot-from-rcx 144))
        stack54 (concat-byte-vectors
                    stack53
                    (emit-mov-rcx-from-local (native-value-window-spill-offset frame-base-slot-count 26)))
        stack55 (concat-byte-vectors stack54 (emit-mov-stack-slot-from-rcx 136))
        stack56 (concat-byte-vectors
                    stack55
                    (emit-mov-rcx-from-local (native-value-window-spill-offset frame-base-slot-count 27)))
        stack57 (concat-byte-vectors stack56 (emit-mov-stack-slot-from-rcx 128))
        stack58 (concat-byte-vectors
                    stack57
                    (emit-mov-rcx-from-local (native-value-window-spill-offset frame-base-slot-count 28)))
        stack59 (concat-byte-vectors stack58 (emit-mov-stack-slot-from-rcx 120))
        stack60 (concat-byte-vectors
                    stack59
                    (emit-mov-rcx-from-local (native-value-window-spill-offset frame-base-slot-count 29)))
        stack61 (concat-byte-vectors stack60 (emit-mov-stack-slot-from-rcx 112))
        stack62 (concat-byte-vectors
                    stack61
                    (emit-mov-rcx-from-local (native-value-window-spill-offset frame-base-slot-count 30)))
        stack63 (concat-byte-vectors stack62 (emit-mov-stack-slot-from-rcx 104))
        stack64 (concat-byte-vectors
                    stack63
                    (emit-mov-rcx-from-local (native-value-window-spill-offset frame-base-slot-count 31)))
        stack65 (concat-byte-vectors stack64 (emit-mov-stack-slot-from-rcx 96))
        stack66 (concat-byte-vectors
                    stack65
                    (emit-mov-rcx-from-local (native-value-window-spill-offset frame-base-slot-count 32)))
        stack67 (concat-byte-vectors stack66 (emit-mov-stack-slot-from-rcx 88))
        stack68 (concat-byte-vectors
                    stack67
                    (emit-mov-rcx-from-local (native-value-window-spill-offset frame-base-slot-count 33)))
        stack69 (concat-byte-vectors stack68 (emit-mov-stack-slot-from-rcx 80))
        stack70 (concat-byte-vectors
                    stack69
                    (emit-mov-rcx-from-local (native-value-window-spill-offset frame-base-slot-count 34)))
        stack71 (concat-byte-vectors stack70 (emit-mov-stack-slot-from-rcx 72))
        stack72 (concat-byte-vectors
                    stack71
                    (emit-mov-rcx-from-local (native-value-window-spill-offset frame-base-slot-count 35)))
        stack73 (concat-byte-vectors stack72 (emit-mov-stack-slot-from-rcx 64))
        stack74 (concat-byte-vectors
                    stack73
                    (emit-mov-rcx-from-local (native-value-window-spill-offset frame-base-slot-count 36)))
        stack75 (concat-byte-vectors stack74 (emit-mov-stack-slot-from-rcx 56))
        stack76 (concat-byte-vectors
                    stack75
                    (emit-mov-rcx-from-local (native-value-window-spill-offset frame-base-slot-count 37)))
        stack77 (concat-byte-vectors stack76 (emit-mov-stack-slot-from-rcx 48))
        stack78 (concat-byte-vectors
                    stack77
                    (emit-mov-rcx-from-local (native-value-window-spill-offset frame-base-slot-count 38)))
        stack79 (concat-byte-vectors stack78 (emit-mov-stack-slot-from-rcx 40))
        stack80 (concat-byte-vectors
                    stack79
                    (emit-mov-rcx-from-local (native-value-window-spill-offset frame-base-slot-count 39)))
        stack81 (concat-byte-vectors stack80 (emit-mov-stack-slot-from-rcx 32))
        stack82 (concat-byte-vectors
                    stack81
                    (emit-mov-rcx-from-local (native-value-window-spill-offset frame-base-slot-count 40)))
        stack83 (concat-byte-vectors stack82 (emit-mov-stack-slot-from-rcx 24))
        stack84 (concat-byte-vectors
                    stack83
                    (emit-mov-rcx-from-local (native-value-window-spill-offset frame-base-slot-count 41)))
        stack85 (concat-byte-vectors stack84 (emit-mov-stack-slot-from-rcx 16))
        stack86 (concat-byte-vectors
                    stack85
                    (emit-mov-rcx-from-local (native-value-window-spill-offset frame-base-slot-count 42)))
        stack87 (concat-byte-vectors stack86 (emit-mov-stack-slot-from-rcx 8))
        stack-setup (concat-byte-vectors
                      stack87
                      (emit-mov-r9-from-local (native-value-window-spill-offset frame-base-slot-count 43)))
        reg0 (concat-byte-vectors
               (emit-mov-top-stack-from-r9)
               (emit-mov-r9-from-local (native-value-window-spill-offset frame-base-slot-count 44)))
        reg1 (concat-byte-vectors reg0 (emit-mov-r8-from-local (native-value-window-spill-offset frame-base-slot-count 45)))
        reg2 (concat-byte-vectors reg1 (emit-mov-rcx-from-local (native-value-window-spill-offset frame-base-slot-count 46)))
        reg3 (concat-byte-vectors reg2 (emit-mov-rdx-from-local (native-value-window-spill-offset frame-base-slot-count 47)))
        reg4 (concat-byte-vectors reg3 (emit-mov-rsi-from-local (native-value-window-spill-offset frame-base-slot-count 48)))
        reg-setup (concat-byte-vectors reg4 (emit-mov-rdi-from-local (native-value-window-spill-offset frame-base-slot-count 49)))
        call-seq (concat-byte-vectors
                   (emit-call-rel32 rel)
                   (emit-add-rsp-imm32 368))]
    (concat-byte-vectors
      (concat-byte-vectors stack-setup reg-setup)
      call-seq)))

(defn emit-fifty-three-arg-call-x86 [rel frame-base-slot-count]
  (let [stack0 (concat-byte-vectors
                 (emit-sub-rsp-imm32 376)
                 (emit-mov-stack-slot-from-rax 368))
        stack1 (concat-byte-vectors stack0 (emit-mov-stack-slot-from-rcx 360))
        stack2 (concat-byte-vectors
                    stack1
                    (emit-mov-rcx-from-local (native-value-window-spill-offset frame-base-slot-count 0)))
        stack3 (concat-byte-vectors stack2 (emit-mov-stack-slot-from-rcx 352))
        stack4 (concat-byte-vectors
                    stack3
                    (emit-mov-rcx-from-local (native-value-window-spill-offset frame-base-slot-count 1)))
        stack5 (concat-byte-vectors stack4 (emit-mov-stack-slot-from-rcx 344))
        stack6 (concat-byte-vectors
                    stack5
                    (emit-mov-rcx-from-local (native-value-window-spill-offset frame-base-slot-count 2)))
        stack7 (concat-byte-vectors stack6 (emit-mov-stack-slot-from-rcx 336))
        stack8 (concat-byte-vectors
                    stack7
                    (emit-mov-rcx-from-local (native-value-window-spill-offset frame-base-slot-count 3)))
        stack9 (concat-byte-vectors stack8 (emit-mov-stack-slot-from-rcx 328))
        stack10 (concat-byte-vectors
                     stack9
                     (emit-mov-rcx-from-local (native-value-window-spill-offset frame-base-slot-count 4)))
        stack11 (concat-byte-vectors stack10 (emit-mov-stack-slot-from-rcx 320))
        stack12 (concat-byte-vectors
                     stack11
                     (emit-mov-rcx-from-local (native-value-window-spill-offset frame-base-slot-count 5)))
        stack13 (concat-byte-vectors stack12 (emit-mov-stack-slot-from-rcx 312))
        stack14 (concat-byte-vectors
                     stack13
                     (emit-mov-rcx-from-local (native-value-window-spill-offset frame-base-slot-count 6)))
        stack15 (concat-byte-vectors stack14 (emit-mov-stack-slot-from-rcx 304))
        stack16 (concat-byte-vectors
                     stack15
                     (emit-mov-rcx-from-local (native-value-window-spill-offset frame-base-slot-count 7)))
        stack17 (concat-byte-vectors stack16 (emit-mov-stack-slot-from-rcx 296))
        stack18 (concat-byte-vectors
                     stack17
                     (emit-mov-rcx-from-local (native-value-window-spill-offset frame-base-slot-count 8)))
        stack19 (concat-byte-vectors stack18 (emit-mov-stack-slot-from-rcx 288))
        stack20 (concat-byte-vectors
                     stack19
                     (emit-mov-rcx-from-local (native-value-window-spill-offset frame-base-slot-count 9)))
        stack21 (concat-byte-vectors stack20 (emit-mov-stack-slot-from-rcx 280))
        stack22 (concat-byte-vectors
                     stack21
                     (emit-mov-rcx-from-local (native-value-window-spill-offset frame-base-slot-count 10)))
        stack23 (concat-byte-vectors stack22 (emit-mov-stack-slot-from-rcx 272))
        stack24 (concat-byte-vectors
                     stack23
                     (emit-mov-rcx-from-local (native-value-window-spill-offset frame-base-slot-count 11)))
        stack25 (concat-byte-vectors stack24 (emit-mov-stack-slot-from-rcx 264))
        stack26 (concat-byte-vectors
                     stack25
                     (emit-mov-rcx-from-local (native-value-window-spill-offset frame-base-slot-count 12)))
        stack27 (concat-byte-vectors stack26 (emit-mov-stack-slot-from-rcx 256))
        stack28 (concat-byte-vectors
                     stack27
                     (emit-mov-rcx-from-local (native-value-window-spill-offset frame-base-slot-count 13)))
        stack29 (concat-byte-vectors stack28 (emit-mov-stack-slot-from-rcx 248))
        stack30 (concat-byte-vectors
                     stack29
                     (emit-mov-rcx-from-local (native-value-window-spill-offset frame-base-slot-count 14)))
        stack31 (concat-byte-vectors stack30 (emit-mov-stack-slot-from-rcx 240))
        stack32 (concat-byte-vectors
                     stack31
                     (emit-mov-rcx-from-local (native-value-window-spill-offset frame-base-slot-count 15)))
        stack33 (concat-byte-vectors stack32 (emit-mov-stack-slot-from-rcx 232))
        stack34 (concat-byte-vectors
                     stack33
                     (emit-mov-rcx-from-local (native-value-window-spill-offset frame-base-slot-count 16)))
        stack35 (concat-byte-vectors stack34 (emit-mov-stack-slot-from-rcx 224))
        stack36 (concat-byte-vectors
                     stack35
                     (emit-mov-rcx-from-local (native-value-window-spill-offset frame-base-slot-count 17)))
        stack37 (concat-byte-vectors stack36 (emit-mov-stack-slot-from-rcx 216))
        stack38 (concat-byte-vectors
                     stack37
                     (emit-mov-rcx-from-local (native-value-window-spill-offset frame-base-slot-count 18)))
        stack39 (concat-byte-vectors stack38 (emit-mov-stack-slot-from-rcx 208))
        stack40 (concat-byte-vectors
                     stack39
                     (emit-mov-rcx-from-local (native-value-window-spill-offset frame-base-slot-count 19)))
        stack41 (concat-byte-vectors stack40 (emit-mov-stack-slot-from-rcx 200))
        stack42 (concat-byte-vectors
                     stack41
                     (emit-mov-rcx-from-local (native-value-window-spill-offset frame-base-slot-count 20)))
        stack43 (concat-byte-vectors stack42 (emit-mov-stack-slot-from-rcx 192))
        stack44 (concat-byte-vectors
                     stack43
                     (emit-mov-rcx-from-local (native-value-window-spill-offset frame-base-slot-count 21)))
        stack45 (concat-byte-vectors stack44 (emit-mov-stack-slot-from-rcx 184))
        stack46 (concat-byte-vectors
                     stack45
                     (emit-mov-rcx-from-local (native-value-window-spill-offset frame-base-slot-count 22)))
        stack47 (concat-byte-vectors stack46 (emit-mov-stack-slot-from-rcx 176))
        stack48 (concat-byte-vectors
                     stack47
                     (emit-mov-rcx-from-local (native-value-window-spill-offset frame-base-slot-count 23)))
        stack49 (concat-byte-vectors stack48 (emit-mov-stack-slot-from-rcx 168))
        stack50 (concat-byte-vectors
                     stack49
                     (emit-mov-rcx-from-local (native-value-window-spill-offset frame-base-slot-count 24)))
        stack51 (concat-byte-vectors stack50 (emit-mov-stack-slot-from-rcx 160))
        stack52 (concat-byte-vectors
                     stack51
                     (emit-mov-rcx-from-local (native-value-window-spill-offset frame-base-slot-count 25)))
        stack53 (concat-byte-vectors stack52 (emit-mov-stack-slot-from-rcx 152))
        stack54 (concat-byte-vectors
                     stack53
                     (emit-mov-rcx-from-local (native-value-window-spill-offset frame-base-slot-count 26)))
        stack55 (concat-byte-vectors stack54 (emit-mov-stack-slot-from-rcx 144))
        stack56 (concat-byte-vectors
                     stack55
                     (emit-mov-rcx-from-local (native-value-window-spill-offset frame-base-slot-count 27)))
        stack57 (concat-byte-vectors stack56 (emit-mov-stack-slot-from-rcx 136))
        stack58 (concat-byte-vectors
                     stack57
                     (emit-mov-rcx-from-local (native-value-window-spill-offset frame-base-slot-count 28)))
        stack59 (concat-byte-vectors stack58 (emit-mov-stack-slot-from-rcx 128))
        stack60 (concat-byte-vectors
                     stack59
                     (emit-mov-rcx-from-local (native-value-window-spill-offset frame-base-slot-count 29)))
        stack61 (concat-byte-vectors stack60 (emit-mov-stack-slot-from-rcx 120))
        stack62 (concat-byte-vectors
                     stack61
                     (emit-mov-rcx-from-local (native-value-window-spill-offset frame-base-slot-count 30)))
        stack63 (concat-byte-vectors stack62 (emit-mov-stack-slot-from-rcx 112))
        stack64 (concat-byte-vectors
                     stack63
                     (emit-mov-rcx-from-local (native-value-window-spill-offset frame-base-slot-count 31)))
        stack65 (concat-byte-vectors stack64 (emit-mov-stack-slot-from-rcx 104))
        stack66 (concat-byte-vectors
                     stack65
                     (emit-mov-rcx-from-local (native-value-window-spill-offset frame-base-slot-count 32)))
        stack67 (concat-byte-vectors stack66 (emit-mov-stack-slot-from-rcx 96))
        stack68 (concat-byte-vectors
                     stack67
                     (emit-mov-rcx-from-local (native-value-window-spill-offset frame-base-slot-count 33)))
        stack69 (concat-byte-vectors stack68 (emit-mov-stack-slot-from-rcx 88))
        stack70 (concat-byte-vectors
                     stack69
                     (emit-mov-rcx-from-local (native-value-window-spill-offset frame-base-slot-count 34)))
        stack71 (concat-byte-vectors stack70 (emit-mov-stack-slot-from-rcx 80))
        stack72 (concat-byte-vectors
                     stack71
                     (emit-mov-rcx-from-local (native-value-window-spill-offset frame-base-slot-count 35)))
        stack73 (concat-byte-vectors stack72 (emit-mov-stack-slot-from-rcx 72))
        stack74 (concat-byte-vectors
                     stack73
                     (emit-mov-rcx-from-local (native-value-window-spill-offset frame-base-slot-count 36)))
        stack75 (concat-byte-vectors stack74 (emit-mov-stack-slot-from-rcx 64))
        stack76 (concat-byte-vectors
                     stack75
                     (emit-mov-rcx-from-local (native-value-window-spill-offset frame-base-slot-count 37)))
        stack77 (concat-byte-vectors stack76 (emit-mov-stack-slot-from-rcx 56))
        stack78 (concat-byte-vectors
                     stack77
                     (emit-mov-rcx-from-local (native-value-window-spill-offset frame-base-slot-count 38)))
        stack79 (concat-byte-vectors stack78 (emit-mov-stack-slot-from-rcx 48))
        stack80 (concat-byte-vectors
                     stack79
                     (emit-mov-rcx-from-local (native-value-window-spill-offset frame-base-slot-count 39)))
        stack81 (concat-byte-vectors stack80 (emit-mov-stack-slot-from-rcx 40))
        stack82 (concat-byte-vectors
                     stack81
                     (emit-mov-rcx-from-local (native-value-window-spill-offset frame-base-slot-count 40)))
        stack83 (concat-byte-vectors stack82 (emit-mov-stack-slot-from-rcx 32))
        stack84 (concat-byte-vectors
                     stack83
                     (emit-mov-rcx-from-local (native-value-window-spill-offset frame-base-slot-count 41)))
        stack85 (concat-byte-vectors stack84 (emit-mov-stack-slot-from-rcx 24))
        stack86 (concat-byte-vectors
                     stack85
                     (emit-mov-rcx-from-local (native-value-window-spill-offset frame-base-slot-count 42)))
        stack87 (concat-byte-vectors stack86 (emit-mov-stack-slot-from-rcx 16))
        stack88 (concat-byte-vectors
                     stack87
                     (emit-mov-rcx-from-local (native-value-window-spill-offset frame-base-slot-count 43)))
        stack89 (concat-byte-vectors stack88 (emit-mov-stack-slot-from-rcx 8))
        stack-setup (concat-byte-vectors
                      stack89
                      (emit-mov-r9-from-local (native-value-window-spill-offset frame-base-slot-count 44)))
        reg0 (concat-byte-vectors
               (emit-mov-top-stack-from-r9)
               (emit-mov-r9-from-local (native-value-window-spill-offset frame-base-slot-count 45)))
        reg1 (concat-byte-vectors reg0 (emit-mov-r8-from-local (native-value-window-spill-offset frame-base-slot-count 46)))
        reg2 (concat-byte-vectors reg1 (emit-mov-rcx-from-local (native-value-window-spill-offset frame-base-slot-count 47)))
        reg3 (concat-byte-vectors reg2 (emit-mov-rdx-from-local (native-value-window-spill-offset frame-base-slot-count 48)))
        reg4 (concat-byte-vectors reg3 (emit-mov-rsi-from-local (native-value-window-spill-offset frame-base-slot-count 49)))
        reg-setup (concat-byte-vectors reg4 (emit-mov-rdi-from-local (native-value-window-spill-offset frame-base-slot-count 50)))
        call-seq (concat-byte-vectors
                   (emit-call-rel32 rel)
                   (emit-add-rsp-imm32 376))]
    (concat-byte-vectors
      (concat-byte-vectors stack-setup reg-setup)
      call-seq)))

(defn emit-fifty-four-arg-call-x86 [rel frame-base-slot-count]
  (let [stack0 (concat-byte-vectors
                 (emit-sub-rsp-imm32 384)
                 (emit-mov-stack-slot-from-rax 376))
        stack1 (concat-byte-vectors stack0 (emit-mov-stack-slot-from-rcx 368))
        stack2 (concat-byte-vectors
                    stack1
                    (emit-mov-rcx-from-local (native-value-window-spill-offset frame-base-slot-count 0)))
        stack3 (concat-byte-vectors stack2 (emit-mov-stack-slot-from-rcx 360))
        stack4 (concat-byte-vectors
                    stack3
                    (emit-mov-rcx-from-local (native-value-window-spill-offset frame-base-slot-count 1)))
        stack5 (concat-byte-vectors stack4 (emit-mov-stack-slot-from-rcx 352))
        stack6 (concat-byte-vectors
                    stack5
                    (emit-mov-rcx-from-local (native-value-window-spill-offset frame-base-slot-count 2)))
        stack7 (concat-byte-vectors stack6 (emit-mov-stack-slot-from-rcx 344))
        stack8 (concat-byte-vectors
                    stack7
                    (emit-mov-rcx-from-local (native-value-window-spill-offset frame-base-slot-count 3)))
        stack9 (concat-byte-vectors stack8 (emit-mov-stack-slot-from-rcx 336))
        stack10 (concat-byte-vectors
                     stack9
                     (emit-mov-rcx-from-local (native-value-window-spill-offset frame-base-slot-count 4)))
        stack11 (concat-byte-vectors stack10 (emit-mov-stack-slot-from-rcx 328))
        stack12 (concat-byte-vectors
                     stack11
                     (emit-mov-rcx-from-local (native-value-window-spill-offset frame-base-slot-count 5)))
        stack13 (concat-byte-vectors stack12 (emit-mov-stack-slot-from-rcx 320))
        stack14 (concat-byte-vectors
                     stack13
                     (emit-mov-rcx-from-local (native-value-window-spill-offset frame-base-slot-count 6)))
        stack15 (concat-byte-vectors stack14 (emit-mov-stack-slot-from-rcx 312))
        stack16 (concat-byte-vectors
                     stack15
                     (emit-mov-rcx-from-local (native-value-window-spill-offset frame-base-slot-count 7)))
        stack17 (concat-byte-vectors stack16 (emit-mov-stack-slot-from-rcx 304))
        stack18 (concat-byte-vectors
                     stack17
                     (emit-mov-rcx-from-local (native-value-window-spill-offset frame-base-slot-count 8)))
        stack19 (concat-byte-vectors stack18 (emit-mov-stack-slot-from-rcx 296))
        stack20 (concat-byte-vectors
                     stack19
                     (emit-mov-rcx-from-local (native-value-window-spill-offset frame-base-slot-count 9)))
        stack21 (concat-byte-vectors stack20 (emit-mov-stack-slot-from-rcx 288))
        stack22 (concat-byte-vectors
                     stack21
                     (emit-mov-rcx-from-local (native-value-window-spill-offset frame-base-slot-count 10)))
        stack23 (concat-byte-vectors stack22 (emit-mov-stack-slot-from-rcx 280))
        stack24 (concat-byte-vectors
                     stack23
                     (emit-mov-rcx-from-local (native-value-window-spill-offset frame-base-slot-count 11)))
        stack25 (concat-byte-vectors stack24 (emit-mov-stack-slot-from-rcx 272))
        stack26 (concat-byte-vectors
                     stack25
                     (emit-mov-rcx-from-local (native-value-window-spill-offset frame-base-slot-count 12)))
        stack27 (concat-byte-vectors stack26 (emit-mov-stack-slot-from-rcx 264))
        stack28 (concat-byte-vectors
                     stack27
                     (emit-mov-rcx-from-local (native-value-window-spill-offset frame-base-slot-count 13)))
        stack29 (concat-byte-vectors stack28 (emit-mov-stack-slot-from-rcx 256))
        stack30 (concat-byte-vectors
                     stack29
                     (emit-mov-rcx-from-local (native-value-window-spill-offset frame-base-slot-count 14)))
        stack31 (concat-byte-vectors stack30 (emit-mov-stack-slot-from-rcx 248))
        stack32 (concat-byte-vectors
                     stack31
                     (emit-mov-rcx-from-local (native-value-window-spill-offset frame-base-slot-count 15)))
        stack33 (concat-byte-vectors stack32 (emit-mov-stack-slot-from-rcx 240))
        stack34 (concat-byte-vectors
                     stack33
                     (emit-mov-rcx-from-local (native-value-window-spill-offset frame-base-slot-count 16)))
        stack35 (concat-byte-vectors stack34 (emit-mov-stack-slot-from-rcx 232))
        stack36 (concat-byte-vectors
                     stack35
                     (emit-mov-rcx-from-local (native-value-window-spill-offset frame-base-slot-count 17)))
        stack37 (concat-byte-vectors stack36 (emit-mov-stack-slot-from-rcx 224))
        stack38 (concat-byte-vectors
                     stack37
                     (emit-mov-rcx-from-local (native-value-window-spill-offset frame-base-slot-count 18)))
        stack39 (concat-byte-vectors stack38 (emit-mov-stack-slot-from-rcx 216))
        stack40 (concat-byte-vectors
                     stack39
                     (emit-mov-rcx-from-local (native-value-window-spill-offset frame-base-slot-count 19)))
        stack41 (concat-byte-vectors stack40 (emit-mov-stack-slot-from-rcx 208))
        stack42 (concat-byte-vectors
                     stack41
                     (emit-mov-rcx-from-local (native-value-window-spill-offset frame-base-slot-count 20)))
        stack43 (concat-byte-vectors stack42 (emit-mov-stack-slot-from-rcx 200))
        stack44 (concat-byte-vectors
                     stack43
                     (emit-mov-rcx-from-local (native-value-window-spill-offset frame-base-slot-count 21)))
        stack45 (concat-byte-vectors stack44 (emit-mov-stack-slot-from-rcx 192))
        stack46 (concat-byte-vectors
                     stack45
                     (emit-mov-rcx-from-local (native-value-window-spill-offset frame-base-slot-count 22)))
        stack47 (concat-byte-vectors stack46 (emit-mov-stack-slot-from-rcx 184))
        stack48 (concat-byte-vectors
                     stack47
                     (emit-mov-rcx-from-local (native-value-window-spill-offset frame-base-slot-count 23)))
        stack49 (concat-byte-vectors stack48 (emit-mov-stack-slot-from-rcx 176))
        stack50 (concat-byte-vectors
                     stack49
                     (emit-mov-rcx-from-local (native-value-window-spill-offset frame-base-slot-count 24)))
        stack51 (concat-byte-vectors stack50 (emit-mov-stack-slot-from-rcx 168))
        stack52 (concat-byte-vectors
                     stack51
                     (emit-mov-rcx-from-local (native-value-window-spill-offset frame-base-slot-count 25)))
        stack53 (concat-byte-vectors stack52 (emit-mov-stack-slot-from-rcx 160))
        stack54 (concat-byte-vectors
                     stack53
                     (emit-mov-rcx-from-local (native-value-window-spill-offset frame-base-slot-count 26)))
        stack55 (concat-byte-vectors stack54 (emit-mov-stack-slot-from-rcx 152))
        stack56 (concat-byte-vectors
                     stack55
                     (emit-mov-rcx-from-local (native-value-window-spill-offset frame-base-slot-count 27)))
        stack57 (concat-byte-vectors stack56 (emit-mov-stack-slot-from-rcx 144))
        stack58 (concat-byte-vectors
                     stack57
                     (emit-mov-rcx-from-local (native-value-window-spill-offset frame-base-slot-count 28)))
        stack59 (concat-byte-vectors stack58 (emit-mov-stack-slot-from-rcx 136))
        stack60 (concat-byte-vectors
                     stack59
                     (emit-mov-rcx-from-local (native-value-window-spill-offset frame-base-slot-count 29)))
        stack61 (concat-byte-vectors stack60 (emit-mov-stack-slot-from-rcx 128))
        stack62 (concat-byte-vectors
                     stack61
                     (emit-mov-rcx-from-local (native-value-window-spill-offset frame-base-slot-count 30)))
        stack63 (concat-byte-vectors stack62 (emit-mov-stack-slot-from-rcx 120))
        stack64 (concat-byte-vectors
                     stack63
                     (emit-mov-rcx-from-local (native-value-window-spill-offset frame-base-slot-count 31)))
        stack65 (concat-byte-vectors stack64 (emit-mov-stack-slot-from-rcx 112))
        stack66 (concat-byte-vectors
                     stack65
                     (emit-mov-rcx-from-local (native-value-window-spill-offset frame-base-slot-count 32)))
        stack67 (concat-byte-vectors stack66 (emit-mov-stack-slot-from-rcx 104))
        stack68 (concat-byte-vectors
                     stack67
                     (emit-mov-rcx-from-local (native-value-window-spill-offset frame-base-slot-count 33)))
        stack69 (concat-byte-vectors stack68 (emit-mov-stack-slot-from-rcx 96))
        stack70 (concat-byte-vectors
                     stack69
                     (emit-mov-rcx-from-local (native-value-window-spill-offset frame-base-slot-count 34)))
        stack71 (concat-byte-vectors stack70 (emit-mov-stack-slot-from-rcx 88))
        stack72 (concat-byte-vectors
                     stack71
                     (emit-mov-rcx-from-local (native-value-window-spill-offset frame-base-slot-count 35)))
        stack73 (concat-byte-vectors stack72 (emit-mov-stack-slot-from-rcx 80))
        stack74 (concat-byte-vectors
                     stack73
                     (emit-mov-rcx-from-local (native-value-window-spill-offset frame-base-slot-count 36)))
        stack75 (concat-byte-vectors stack74 (emit-mov-stack-slot-from-rcx 72))
        stack76 (concat-byte-vectors
                     stack75
                     (emit-mov-rcx-from-local (native-value-window-spill-offset frame-base-slot-count 37)))
        stack77 (concat-byte-vectors stack76 (emit-mov-stack-slot-from-rcx 64))
        stack78 (concat-byte-vectors
                     stack77
                     (emit-mov-rcx-from-local (native-value-window-spill-offset frame-base-slot-count 38)))
        stack79 (concat-byte-vectors stack78 (emit-mov-stack-slot-from-rcx 56))
        stack80 (concat-byte-vectors
                     stack79
                     (emit-mov-rcx-from-local (native-value-window-spill-offset frame-base-slot-count 39)))
        stack81 (concat-byte-vectors stack80 (emit-mov-stack-slot-from-rcx 48))
        stack82 (concat-byte-vectors
                     stack81
                     (emit-mov-rcx-from-local (native-value-window-spill-offset frame-base-slot-count 40)))
        stack83 (concat-byte-vectors stack82 (emit-mov-stack-slot-from-rcx 40))
        stack84 (concat-byte-vectors
                     stack83
                     (emit-mov-rcx-from-local (native-value-window-spill-offset frame-base-slot-count 41)))
        stack85 (concat-byte-vectors stack84 (emit-mov-stack-slot-from-rcx 32))
        stack86 (concat-byte-vectors
                     stack85
                     (emit-mov-rcx-from-local (native-value-window-spill-offset frame-base-slot-count 42)))
        stack87 (concat-byte-vectors stack86 (emit-mov-stack-slot-from-rcx 24))
        stack88 (concat-byte-vectors
                     stack87
                     (emit-mov-rcx-from-local (native-value-window-spill-offset frame-base-slot-count 43)))
        stack89 (concat-byte-vectors stack88 (emit-mov-stack-slot-from-rcx 16))
        stack90 (concat-byte-vectors
                     stack89
                     (emit-mov-rcx-from-local (native-value-window-spill-offset frame-base-slot-count 44)))
        stack91 (concat-byte-vectors stack90 (emit-mov-stack-slot-from-rcx 8))
        stack-setup (concat-byte-vectors
                      stack91
                      (emit-mov-r9-from-local (native-value-window-spill-offset frame-base-slot-count 45)))
        reg0 (concat-byte-vectors
               (emit-mov-top-stack-from-r9)
               (emit-mov-r9-from-local (native-value-window-spill-offset frame-base-slot-count 46)))
        reg1 (concat-byte-vectors reg0 (emit-mov-r8-from-local (native-value-window-spill-offset frame-base-slot-count 47)))
        reg2 (concat-byte-vectors reg1 (emit-mov-rcx-from-local (native-value-window-spill-offset frame-base-slot-count 48)))
        reg3 (concat-byte-vectors reg2 (emit-mov-rdx-from-local (native-value-window-spill-offset frame-base-slot-count 49)))
        reg4 (concat-byte-vectors reg3 (emit-mov-rsi-from-local (native-value-window-spill-offset frame-base-slot-count 50)))
        reg-setup (concat-byte-vectors reg4 (emit-mov-rdi-from-local (native-value-window-spill-offset frame-base-slot-count 51)))
        call-seq (concat-byte-vectors
                   (emit-call-rel32 rel)
                   (emit-add-rsp-imm32 384))]
    (concat-byte-vectors
      (concat-byte-vectors stack-setup reg-setup)
      call-seq)))

(defn emit-x86-window-stack-arg-spills [frame-base-slot-count spill-index last-spill-index slot-offset]
  (if (> spill-index last-spill-index)
    (vector-new 0)
    (concat-byte-vectors
      (concat-byte-vectors
        (emit-mov-rcx-from-local (native-value-window-spill-offset frame-base-slot-count spill-index))
        (emit-mov-stack-slot-from-rcx slot-offset))
      (emit-x86-window-stack-arg-spills frame-base-slot-count (+ spill-index 1) last-spill-index (- slot-offset 8)))))

(defn emit-fifty-five-arg-call-x86 [rel frame-base-slot-count]
  (let [stack-body (emit-x86-window-stack-arg-spills frame-base-slot-count 0 45 368)
        stack-setup (concat-byte-vectors
                      (concat-byte-vectors
                        (concat-byte-vectors
                          (concat-byte-vectors
                            (emit-sub-rsp-imm32 392)
                            (emit-mov-stack-slot-from-rax 384))
                          (emit-mov-stack-slot-from-rcx 376))
                        stack-body)
                      (emit-mov-r9-from-local (native-value-window-spill-offset frame-base-slot-count 46)))
        reg0 (concat-byte-vectors
               (emit-mov-top-stack-from-r9)
               (emit-mov-r9-from-local (native-value-window-spill-offset frame-base-slot-count 47)))
        reg1 (concat-byte-vectors reg0 (emit-mov-r8-from-local (native-value-window-spill-offset frame-base-slot-count 48)))
        reg2 (concat-byte-vectors reg1 (emit-mov-rcx-from-local (native-value-window-spill-offset frame-base-slot-count 49)))
        reg3 (concat-byte-vectors reg2 (emit-mov-rdx-from-local (native-value-window-spill-offset frame-base-slot-count 50)))
        reg4 (concat-byte-vectors reg3 (emit-mov-rsi-from-local (native-value-window-spill-offset frame-base-slot-count 51)))
        reg-setup (concat-byte-vectors reg4 (emit-mov-rdi-from-local (native-value-window-spill-offset frame-base-slot-count 52)))
        call-seq (concat-byte-vectors
                   (emit-call-rel32 rel)
                   (emit-add-rsp-imm32 392))]
    (concat-byte-vectors
      (concat-byte-vectors stack-setup reg-setup)
      call-seq)))

(defn emit-fifty-six-arg-call-x86 [rel frame-base-slot-count]
  (let [stack-body (emit-x86-window-stack-arg-spills frame-base-slot-count 0 46 376)
        stack-setup (concat-byte-vectors
                      (concat-byte-vectors
                        (concat-byte-vectors
                          (concat-byte-vectors
                            (emit-sub-rsp-imm32 400)
                            (emit-mov-stack-slot-from-rax 392))
                          (emit-mov-stack-slot-from-rcx 384))
                        stack-body)
                      (emit-mov-r9-from-local (native-value-window-spill-offset frame-base-slot-count 47)))
        reg0 (concat-byte-vectors
               (emit-mov-top-stack-from-r9)
               (emit-mov-r9-from-local (native-value-window-spill-offset frame-base-slot-count 48)))
        reg1 (concat-byte-vectors reg0 (emit-mov-r8-from-local (native-value-window-spill-offset frame-base-slot-count 49)))
        reg2 (concat-byte-vectors reg1 (emit-mov-rcx-from-local (native-value-window-spill-offset frame-base-slot-count 50)))
        reg3 (concat-byte-vectors reg2 (emit-mov-rdx-from-local (native-value-window-spill-offset frame-base-slot-count 51)))
        reg4 (concat-byte-vectors reg3 (emit-mov-rsi-from-local (native-value-window-spill-offset frame-base-slot-count 52)))
        reg-setup (concat-byte-vectors reg4 (emit-mov-rdi-from-local (native-value-window-spill-offset frame-base-slot-count 53)))
        call-seq (concat-byte-vectors
                   (emit-call-rel32 rel)
                   (emit-add-rsp-imm32 400))]
    (concat-byte-vectors
      (concat-byte-vectors stack-setup reg-setup)
      call-seq)))

(defn emit-fifty-seven-arg-call-x86 [rel frame-base-slot-count]
  (let [stack-body (emit-x86-window-stack-arg-spills frame-base-slot-count 0 47 384)
        stack-setup (concat-byte-vectors
                      (concat-byte-vectors
                        (concat-byte-vectors
                          (concat-byte-vectors
                            (emit-sub-rsp-imm32 408)
                            (emit-mov-stack-slot-from-rax 400))
                          (emit-mov-stack-slot-from-rcx 392))
                        stack-body)
                      (emit-mov-r9-from-local (native-value-window-spill-offset frame-base-slot-count 48)))
        reg0 (concat-byte-vectors
               (emit-mov-top-stack-from-r9)
               (emit-mov-r9-from-local (native-value-window-spill-offset frame-base-slot-count 49)))
        reg1 (concat-byte-vectors reg0 (emit-mov-r8-from-local (native-value-window-spill-offset frame-base-slot-count 50)))
        reg2 (concat-byte-vectors reg1 (emit-mov-rcx-from-local (native-value-window-spill-offset frame-base-slot-count 51)))
        reg3 (concat-byte-vectors reg2 (emit-mov-rdx-from-local (native-value-window-spill-offset frame-base-slot-count 52)))
        reg4 (concat-byte-vectors reg3 (emit-mov-rsi-from-local (native-value-window-spill-offset frame-base-slot-count 53)))
        reg-setup (concat-byte-vectors reg4 (emit-mov-rdi-from-local (native-value-window-spill-offset frame-base-slot-count 54)))
        call-seq (concat-byte-vectors
                   (emit-call-rel32 rel)
                   (emit-add-rsp-imm32 408))]
    (concat-byte-vectors
      (concat-byte-vectors stack-setup reg-setup)
      call-seq)))

(defn emit-fifty-eight-arg-call-x86 [rel frame-base-slot-count]
  (let [stack-body (emit-x86-window-stack-arg-spills frame-base-slot-count 0 48 392)
        stack-setup (concat-byte-vectors
                      (concat-byte-vectors
                        (concat-byte-vectors
                          (concat-byte-vectors
                            (emit-sub-rsp-imm32 416)
                            (emit-mov-stack-slot-from-rax 408))
                          (emit-mov-stack-slot-from-rcx 400))
                        stack-body)
                      (emit-mov-r9-from-local (native-value-window-spill-offset frame-base-slot-count 49)))
        reg0 (concat-byte-vectors
               (emit-mov-top-stack-from-r9)
               (emit-mov-r9-from-local (native-value-window-spill-offset frame-base-slot-count 50)))
        reg1 (concat-byte-vectors reg0 (emit-mov-r8-from-local (native-value-window-spill-offset frame-base-slot-count 51)))
        reg2 (concat-byte-vectors reg1 (emit-mov-rcx-from-local (native-value-window-spill-offset frame-base-slot-count 52)))
        reg3 (concat-byte-vectors reg2 (emit-mov-rdx-from-local (native-value-window-spill-offset frame-base-slot-count 53)))
        reg4 (concat-byte-vectors reg3 (emit-mov-rsi-from-local (native-value-window-spill-offset frame-base-slot-count 54)))
        reg-setup (concat-byte-vectors reg4 (emit-mov-rdi-from-local (native-value-window-spill-offset frame-base-slot-count 55)))
        call-seq (concat-byte-vectors
                   (emit-call-rel32 rel)
                   (emit-add-rsp-imm32 416))]
    (concat-byte-vectors
      (concat-byte-vectors stack-setup reg-setup)
      call-seq)))

(defn emit-fifty-nine-arg-call-x86 [rel frame-base-slot-count]
  (let [stack-body (emit-x86-window-stack-arg-spills frame-base-slot-count 0 49 400)
        stack-setup (concat-byte-vectors
                      (concat-byte-vectors
                        (concat-byte-vectors
                          (concat-byte-vectors
                            (emit-sub-rsp-imm32 424)
                            (emit-mov-stack-slot-from-rax 416))
                          (emit-mov-stack-slot-from-rcx 408))
                        stack-body)
                      (emit-mov-r9-from-local (native-value-window-spill-offset frame-base-slot-count 50)))
        reg0 (concat-byte-vectors
               (emit-mov-top-stack-from-r9)
               (emit-mov-r9-from-local (native-value-window-spill-offset frame-base-slot-count 51)))
        reg1 (concat-byte-vectors reg0 (emit-mov-r8-from-local (native-value-window-spill-offset frame-base-slot-count 52)))
        reg2 (concat-byte-vectors reg1 (emit-mov-rcx-from-local (native-value-window-spill-offset frame-base-slot-count 53)))
        reg3 (concat-byte-vectors reg2 (emit-mov-rdx-from-local (native-value-window-spill-offset frame-base-slot-count 54)))
        reg4 (concat-byte-vectors reg3 (emit-mov-rsi-from-local (native-value-window-spill-offset frame-base-slot-count 55)))
        reg-setup (concat-byte-vectors reg4 (emit-mov-rdi-from-local (native-value-window-spill-offset frame-base-slot-count 56)))
        call-seq (concat-byte-vectors
                   (emit-call-rel32 rel)
                   (emit-add-rsp-imm32 424))]
    (concat-byte-vectors
      (concat-byte-vectors stack-setup reg-setup)
      call-seq)))

(defn emit-sixty-arg-call-x86 [rel frame-base-slot-count]
  (let [stack-body (emit-x86-window-stack-arg-spills frame-base-slot-count 0 50 408)
        stack-setup (concat-byte-vectors
                      (concat-byte-vectors
                        (concat-byte-vectors
                          (concat-byte-vectors
                            (emit-sub-rsp-imm32 432)
                            (emit-mov-stack-slot-from-rax 424))
                          (emit-mov-stack-slot-from-rcx 416))
                        stack-body)
                      (emit-mov-r9-from-local (native-value-window-spill-offset frame-base-slot-count 51)))
        reg0 (concat-byte-vectors
               (emit-mov-top-stack-from-r9)
               (emit-mov-r9-from-local (native-value-window-spill-offset frame-base-slot-count 52)))
        reg1 (concat-byte-vectors reg0 (emit-mov-r8-from-local (native-value-window-spill-offset frame-base-slot-count 53)))
        reg2 (concat-byte-vectors reg1 (emit-mov-rcx-from-local (native-value-window-spill-offset frame-base-slot-count 54)))
        reg3 (concat-byte-vectors reg2 (emit-mov-rdx-from-local (native-value-window-spill-offset frame-base-slot-count 55)))
        reg4 (concat-byte-vectors reg3 (emit-mov-rsi-from-local (native-value-window-spill-offset frame-base-slot-count 56)))
        reg-setup (concat-byte-vectors reg4 (emit-mov-rdi-from-local (native-value-window-spill-offset frame-base-slot-count 57)))
        call-seq (concat-byte-vectors
                   (emit-call-rel32 rel)
                   (emit-add-rsp-imm32 432))]
    (concat-byte-vectors
      (concat-byte-vectors stack-setup reg-setup)
      call-seq)))

(defn emit-x86-twenty-plus-reg-setup [frame-base-slot-count top-stack-local-index]
  (let [reg0 (concat-byte-vectors
               (emit-mov-r9-from-local (native-value-window-spill-offset frame-base-slot-count top-stack-local-index))
               (emit-mov-top-stack-from-r9))
        reg1 (concat-byte-vectors reg0 (emit-mov-r9-from-local (native-value-window-spill-offset frame-base-slot-count (+ top-stack-local-index 1))))
        reg2 (concat-byte-vectors reg1 (emit-mov-r8-from-local (native-value-window-spill-offset frame-base-slot-count (+ top-stack-local-index 2))))
        reg3 (concat-byte-vectors reg2 (emit-mov-rcx-from-local (native-value-window-spill-offset frame-base-slot-count (+ top-stack-local-index 3))))
        reg4 (concat-byte-vectors reg3 (emit-mov-rdx-from-local (native-value-window-spill-offset frame-base-slot-count (+ top-stack-local-index 4))))
        reg5 (concat-byte-vectors reg4 (emit-mov-rsi-from-local (native-value-window-spill-offset frame-base-slot-count (+ top-stack-local-index 5))))]
    (concat-byte-vectors reg5 (emit-mov-rdi-from-local (native-value-window-spill-offset frame-base-slot-count (+ top-stack-local-index 6))))))

(defn emit-twenty-plus-arg-call-x86 [target-param-count rel frame-base-slot-count]
  (let [stack-bytes (* (- target-param-count 6) 8)
        stack-body-last (- target-param-count 10)
        stack-body-offset (* (- target-param-count 9) 8)
        top-stack-local-index (- target-param-count 9)
        stack-body (emit-x86-window-stack-arg-spills frame-base-slot-count 0 stack-body-last stack-body-offset)
        stack-setup (concat-byte-vectors
                      (concat-byte-vectors
                        (concat-byte-vectors
                          (concat-byte-vectors
                            (emit-sub-rsp-imm32 stack-bytes)
                            (emit-mov-stack-slot-from-rax (- stack-bytes 8)))
                          (emit-mov-stack-slot-from-rcx (- stack-bytes 16)))
                        stack-body)
                      (vector-new 0))
        reg-setup (emit-x86-twenty-plus-reg-setup frame-base-slot-count top-stack-local-index)
        call-seq (concat-byte-vectors
                   (emit-call-rel32 rel)
                   (emit-add-rsp-imm32 stack-bytes))]
    (concat-byte-vectors
      (concat-byte-vectors stack-setup reg-setup)
      call-seq)))

(defn emit-sixty-one-arg-call-x86 [rel frame-base-slot-count]
  (emit-twenty-plus-arg-call-x86 61 rel frame-base-slot-count))

(defn emit-one-arg-call-x86-core-with-call-bytes [call-rel-bytes]
  (let [call-rel-slot (root_push call-rel-bytes)]
    (let [mov-rdi (emit-mov-rdi-rax)]
      (do
        (root_push mov-rdi)
        (let [push-rcx (emit-push-rcx)]
          (do
            (root_push push-rcx)
            (let [pop-rcx (emit-pop-rcx)]
              (do
                (root_push pop-rcx)
                (let [result (concat-four-byte-vectors-rooted mov-rdi push-rcx call-rel-bytes pop-rcx)]
                  (do
                    (root_set call-rel-slot result)
                    (root_pop)
                    (root_pop)
                    (root_pop)
                    (root_pop)
                    result))))))))))

(defn emit-three-arg-call-x86-core [rel frame-base-slot-count]
  (concat-byte-vectors
    (concat-byte-vectors
      (concat-byte-vectors
        (emit-mov-rdx-rax)
        (emit-mov-rsi-rcx))
      (emit-mov-rdi-from-local (native-value-window-spill-offset frame-base-slot-count 0)))
    (emit-call-rel32 rel)))

(defn emit-three-arg-call-x86-core-with-call-bytes [call-rel-bytes frame-base-slot-count]
  (concat-byte-vectors
    (concat-byte-vectors
      (concat-byte-vectors
        (emit-mov-rdx-rax)
        (emit-mov-rsi-rcx))
      (emit-mov-rdi-from-local (native-value-window-spill-offset frame-base-slot-count 0)))
    call-rel-bytes))

(defn emit-three-arg-call-x86 [rel frame-base-slot-count current-depth]
  (emit-consume-three-produce-one-bundle-x86
    (emit-three-arg-call-x86-core rel frame-base-slot-count)
                           frame-base-slot-count
                          current-depth))

(defn emit-four-arg-call-x86-core [rel frame-base-slot-count]
  (let [setup (concat-four-byte-vectors-rooted
                 (emit-mov-rdx-rcx)
                 (emit-mov-rsi-from-local (native-value-window-spill-offset frame-base-slot-count 0))
                 (emit-mov-rdi-from-local (native-value-window-spill-offset frame-base-slot-count 1))
                 (emit-mov-rcx-rax))]
    (concat-byte-vectors-rooted setup (emit-call-rel32 rel))))

(defn emit-four-arg-call-x86-core-with-call-bytes [call-rel-bytes frame-base-slot-count]
  (let [setup (concat-four-byte-vectors-rooted
                 (emit-mov-rdx-rcx)
                 (emit-mov-rsi-from-local (native-value-window-spill-offset frame-base-slot-count 0))
                 (emit-mov-rdi-from-local (native-value-window-spill-offset frame-base-slot-count 1))
                 (emit-mov-rcx-rax))]
    (do
      (root_push setup)
      (let [result (concat-byte-vectors-rooted setup call-rel-bytes)]
        (do
          (root_pop)
          result)))))

(defn emit-four-arg-call-x86-core-with-rel-ref [call-rel-ref frame-base-slot-count]
  (do
    (root_push call-rel-ref)
    (let [setup (concat-four-byte-vectors-rooted
                   (emit-mov-rdx-rcx)
                   (emit-mov-rsi-from-local (native-value-window-spill-offset frame-base-slot-count 0))
                   (emit-mov-rdi-from-local (native-value-window-spill-offset frame-base-slot-count 1))
                   (emit-mov-rcx-rax))]
      (do
        (root_push setup)
        (let [call-rel-bytes (emit-call-rel32 (ref-get call-rel-ref))]
          (do
            (root_push call-rel-bytes)
            (let [result (concat-byte-vectors-rooted setup call-rel-bytes)]
              (do
                (root_pop)
                (root_pop)
                (root_pop)
                result))))))))

(defn emit-four-arg-call-x86 [rel frame-base-slot-count current-depth]
  (emit-consume-four-produce-one-bundle-x86
    (let [call-rel-ref (ref-new rel)]
      (do
        (root_push call-rel-ref)
        (let [result (emit-four-arg-call-x86-core-with-rel-ref call-rel-ref frame-base-slot-count)]
          (do
            (root_pop)
            result))))
                           frame-base-slot-count
                          current-depth))

(defn emit-five-arg-call-x86-core [rel frame-base-slot-count]
  (let [setup (concat-four-byte-vectors-rooted
                (emit-mov-r8-rax)
                (emit-mov-rdx-from-local (native-value-window-spill-offset frame-base-slot-count 0))
                (emit-mov-rsi-from-local (native-value-window-spill-offset frame-base-slot-count 1))
                (emit-mov-rdi-from-local (native-value-window-spill-offset frame-base-slot-count 2)))]
    (concat-byte-vectors-rooted setup (emit-call-rel32 rel))))

(defn emit-five-arg-call-x86-core-with-call-bytes [call-rel-bytes frame-base-slot-count]
  (let [setup (concat-four-byte-vectors-rooted
                (emit-mov-r8-rax)
                (emit-mov-rdx-from-local (native-value-window-spill-offset frame-base-slot-count 0))
                (emit-mov-rsi-from-local (native-value-window-spill-offset frame-base-slot-count 1))
                (emit-mov-rdi-from-local (native-value-window-spill-offset frame-base-slot-count 2)))]
    (do
      (root_push setup)
      (let [result (concat-byte-vectors-rooted setup call-rel-bytes)]
        (do
          (root_pop)
          result)))))

(defn emit-five-arg-call-x86 [rel frame-base-slot-count current-depth]
  (emit-consume-five-produce-one-bundle-x86
    (emit-five-arg-call-x86-core rel frame-base-slot-count)
    frame-base-slot-count
    current-depth))

(defn emit-six-arg-call-x86-core [rel frame-base-slot-count]
  (let [setup1 (concat-four-byte-vectors-rooted
                 (emit-mov-r9-rax)
                 (emit-mov-r8-rcx)
                 (emit-mov-rcx-from-local (native-value-window-spill-offset frame-base-slot-count 0))
                 (emit-mov-rdx-from-local (native-value-window-spill-offset frame-base-slot-count 1)))]
    (do
      (root_push setup1)
      (let [setup2 (concat-three-byte-vectors-rooted
                     setup1
                     (emit-mov-rsi-from-local (native-value-window-spill-offset frame-base-slot-count 2))
                     (emit-mov-rdi-from-local (native-value-window-spill-offset frame-base-slot-count 3)))]
        (do
          (root_push setup2)
          (let [result (concat-byte-vectors-rooted setup2 (emit-call-rel32 rel))]
            (do
              (root_pop)
              (root_pop)
              result)))))))

(defn emit-six-arg-call-x86-core-with-call-bytes [call-rel-bytes frame-base-slot-count]
  (let [setup1 (concat-four-byte-vectors-rooted
                 (emit-mov-r9-rax)
                 (emit-mov-r8-rcx)
                 (emit-mov-rcx-from-local (native-value-window-spill-offset frame-base-slot-count 0))
                 (emit-mov-rdx-from-local (native-value-window-spill-offset frame-base-slot-count 1)))]
    (do
      (root_push setup1)
      (let [setup2 (concat-three-byte-vectors-rooted
                     setup1
                     (emit-mov-rsi-from-local (native-value-window-spill-offset frame-base-slot-count 2))
                     (emit-mov-rdi-from-local (native-value-window-spill-offset frame-base-slot-count 3)))]
        (do
          (root_push setup2)
          (let [result (concat-byte-vectors-rooted setup2 call-rel-bytes)]
            (do
              (root_pop)
              (root_pop)
              result)))))))

(defn emit-six-arg-call-x86 [rel frame-base-slot-count current-depth]
  (emit-consume-six-produce-one-bundle-x86
    (emit-six-arg-call-x86-core rel frame-base-slot-count)
    frame-base-slot-count
    current-depth))

(defn emit-zero-arg-call-x86 [rel frame-base-slot-count current-depth]
  (let [call-bytes (emit-call-rel32 rel)]
    (if (>= current-depth 2)
      (concat-byte-vectors
        (concat-byte-vectors
          (concat-byte-vectors
            (shift-native-value-window-x86-loop frame-base-slot-count (- current-depth 3))
            (emit-mov-local-from-rcx (native-value-window-spill-offset frame-base-slot-count 0)))
          (emit-push-rax))
        (concat-byte-vectors call-bytes (emit-pop-rcx)))
      (if (= current-depth 1)
        (concat-byte-vectors
          (concat-byte-vectors (emit-push-rax) call-bytes)
          (emit-pop-rcx))
        call-bytes))))

(defn emit-seven-arg-call-x86-core [rel frame-base-slot-count]
  (let [setup1 (concat-five-byte-vectors-rooted
                 (emit-sub-rsp-imm32 16)
                 (emit-mov-top-stack-from-rax)
                 (emit-mov-r9-rcx)
                 (emit-mov-r8-from-local (native-value-window-spill-offset frame-base-slot-count 0))
                 (emit-mov-rcx-from-local (native-value-window-spill-offset frame-base-slot-count 1)))]
    (do
      (root_push setup1)
      (let [setup2 (concat-three-byte-vectors-rooted
                     setup1
                     (emit-mov-rdx-from-local (native-value-window-spill-offset frame-base-slot-count 2))
                     (emit-mov-rsi-from-local (native-value-window-spill-offset frame-base-slot-count 3)))]
        (do
          (root_push setup2)
          (let [result (concat-four-byte-vectors-rooted
                         setup2
                         (emit-mov-rdi-from-local (native-value-window-spill-offset frame-base-slot-count 4))
                         (emit-call-rel32 rel)
                         (emit-add-rsp-imm32 16))]
            (do
              (root_pop)
              (root_pop)
              result)))))))

(defn emit-seven-arg-call-x86-core-with-call-bytes [call-rel-bytes frame-base-slot-count]
  (let [setup1 (concat-five-byte-vectors-rooted
                 (emit-sub-rsp-imm32 16)
                 (emit-mov-top-stack-from-rax)
                 (emit-mov-r9-rcx)
                 (emit-mov-r8-from-local (native-value-window-spill-offset frame-base-slot-count 0))
                 (emit-mov-rcx-from-local (native-value-window-spill-offset frame-base-slot-count 1)))]
    (do
      (root_push setup1)
      (let [setup2 (concat-three-byte-vectors-rooted
                     setup1
                     (emit-mov-rdx-from-local (native-value-window-spill-offset frame-base-slot-count 2))
                     (emit-mov-rsi-from-local (native-value-window-spill-offset frame-base-slot-count 3)))]
        (do
          (root_push setup2)
          (let [result (concat-four-byte-vectors-rooted
                         setup2
                         (emit-mov-rdi-from-local (native-value-window-spill-offset frame-base-slot-count 4))
                         call-rel-bytes
                         (emit-add-rsp-imm32 16))]
            (do
              (root_pop)
              (root_pop)
              result)))))))

(defn emit-seven-arg-call-x86 [rel frame-base-slot-count current-depth]
  (emit-consume-seven-produce-one-bundle-x86
    (emit-seven-arg-call-x86-core rel frame-base-slot-count)
    frame-base-slot-count
    current-depth))

(defn emit-eight-arg-call-x86-core [rel frame-base-slot-count]
  (let [setup1 (concat-five-byte-vectors-rooted
                 (emit-sub-rsp-imm32 16)
                 (emit-mov-second-stack-from-rax)
                 (emit-mov-top-stack-from-rcx)
                 (emit-mov-r9-from-local (native-value-window-spill-offset frame-base-slot-count 0))
                 (emit-mov-r8-from-local (native-value-window-spill-offset frame-base-slot-count 1)))]
    (do
      (root_push setup1)
      (let [setup2 (concat-three-byte-vectors-rooted
                     setup1
                     (emit-mov-rcx-from-local (native-value-window-spill-offset frame-base-slot-count 2))
                     (emit-mov-rdx-from-local (native-value-window-spill-offset frame-base-slot-count 3)))]
        (do
          (root_push setup2)
          (let [result (concat-five-byte-vectors-rooted
                         setup2
                         (emit-mov-rsi-from-local (native-value-window-spill-offset frame-base-slot-count 4))
                         (emit-mov-rdi-from-local (native-value-window-spill-offset frame-base-slot-count 5))
                         (emit-call-rel32 rel)
                         (emit-add-rsp-imm32 16))]
            (do
              (root_pop)
              (root_pop)
              result)))))))

(defn emit-eight-arg-call-x86-core-with-call-bytes [call-rel-bytes frame-base-slot-count]
  (let [setup1 (concat-five-byte-vectors-rooted
                 (emit-sub-rsp-imm32 16)
                 (emit-mov-second-stack-from-rax)
                 (emit-mov-top-stack-from-rcx)
                 (emit-mov-r9-from-local (native-value-window-spill-offset frame-base-slot-count 0))
                 (emit-mov-r8-from-local (native-value-window-spill-offset frame-base-slot-count 1)))]
    (do
      (root_push setup1)
      (let [setup2 (concat-three-byte-vectors-rooted
                     setup1
                     (emit-mov-rcx-from-local (native-value-window-spill-offset frame-base-slot-count 2))
                     (emit-mov-rdx-from-local (native-value-window-spill-offset frame-base-slot-count 3)))]
        (do
          (root_push setup2)
          (let [result (concat-five-byte-vectors-rooted
                         setup2
                         (emit-mov-rsi-from-local (native-value-window-spill-offset frame-base-slot-count 4))
                         (emit-mov-rdi-from-local (native-value-window-spill-offset frame-base-slot-count 5))
                         call-rel-bytes
                         (emit-add-rsp-imm32 16))]
            (do
              (root_pop)
              (root_pop)
              result)))))))

(defn emit-eight-arg-call-x86 [rel frame-base-slot-count current-depth]
  (emit-consume-eight-produce-one-bundle-x86
    (emit-eight-arg-call-x86-core rel frame-base-slot-count)
    frame-base-slot-count
    current-depth))

(defn emit-nine-arg-call-x86-core [rel frame-base-slot-count]
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

(defn emit-nine-arg-call-x86-core-with-call-bytes [call-rel-bytes frame-base-slot-count]
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
       call-rel-bytes)
     (emit-add-rsp-imm32 32)))

(defn emit-nine-arg-call-x86 [rel frame-base-slot-count current-depth]
  (emit-consume-nine-produce-one-bundle-x86
    (emit-nine-arg-call-x86-core rel frame-base-slot-count)
    frame-base-slot-count
    current-depth))

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

(defn emit-two-arg-call-x86-with-call-bytes [call-rel-bytes frame-base-slot-count current-depth]
  (let [call-seq (concat-three-byte-vectors-rooted
                   (emit-mov-rsi-rax)
                   (emit-mov-rdi-rcx)
                   call-rel-bytes)]
    (if (>= current-depth 3)
      (concat-byte-vectors-rooted
        call-seq
        (emit-mov-rcx-from-local (native-value-window-spill-offset frame-base-slot-count 0)))
      call-seq)))

(defn emit-two-arg-call-x86 [rel frame-base-slot-count current-depth]
  (emit-two-arg-call-x86-with-call-bytes
    (emit-call-rel32 rel)
    frame-base-slot-count
    current-depth))

(defn emit-drop-window-spill-shifts-x86 [frame-base-slot-count shift-idx last-shift-idx]
  (if (> shift-idx last-shift-idx)
    (vector-new 0)
    (concat-byte-vectors
      (concat-byte-vectors
        (emit-mov-rsi-from-local (native-value-window-spill-offset frame-base-slot-count shift-idx))
        (emit-mov-local-from-rsi (native-value-window-spill-offset frame-base-slot-count (- shift-idx 1))))
      (emit-drop-window-spill-shifts-x86 frame-base-slot-count (+ shift-idx 1) last-shift-idx))))

(defn emit-drop-bundle-x86 [frame-base-slot-count current-depth]
  (if (>= current-depth 3)
    (concat-byte-vectors
      (concat-byte-vectors
        (emit-mov-rax-rcx)
        (emit-mov-rcx-from-local (native-value-window-spill-offset frame-base-slot-count 0)))
      (emit-drop-window-spill-shifts-x86 frame-base-slot-count 1 (- current-depth 3)))
    (emit-mov-rax-rcx)))

(defn emit-local-set-bundle-x86 [offset frame-base-slot-count current-depth]
  (if (>= current-depth 3)
    (concat-byte-vectors
      (concat-byte-vectors
        (concat-byte-vectors
          (emit-mov-local-from-rax offset)
          (emit-mov-rax-rcx))
        (emit-mov-rcx-from-local (native-value-window-spill-offset frame-base-slot-count 0)))
      (emit-drop-window-spill-shifts-x86 frame-base-slot-count 1 (- current-depth 3)))
    (if (= current-depth 2)
      (concat-byte-vectors
        (emit-mov-local-from-rax offset)
        (emit-mov-rax-rcx))
      (emit-mov-local-from-rax offset))))

(defn emit-root-set-bundle-x86 [frame-base-slot-count current-depth]
  (if (>= current-depth 3)
    (concat-byte-vectors
      (concat-byte-vectors
        (emit-mov-rax-rcx)
        (emit-mov-rcx-from-local (native-value-window-spill-offset frame-base-slot-count 0)))
      (emit-drop-window-spill-shifts-x86 frame-base-slot-count 1 (- current-depth 3)))
    (if (= current-depth 2)
      (emit-mov-rax-rcx)
      (vector-new 0))))

(defn emit-root-push-x86 []
  (byte-vector-2 49 192))

(defn emit-store-window-spill-shifts-x86 [frame-base-slot-count shift-idx last-shift-idx]
  (if (> shift-idx last-shift-idx)
    (vector-new 0)
    (concat-byte-vectors
      (concat-byte-vectors
        (emit-mov-rsi-from-local (native-value-window-spill-offset frame-base-slot-count shift-idx))
        (emit-mov-local-from-rsi (native-value-window-spill-offset frame-base-slot-count (- shift-idx 2))))
      (emit-store-window-spill-shifts-x86 frame-base-slot-count (+ shift-idx 1) last-shift-idx))))

(defn emit-store-window-spill-shifts-x86-consumed [frame-base-slot-count shift-idx last-shift-idx consumed-local-count]
  (if (> shift-idx last-shift-idx)
    (vector-new 0)
    (concat-byte-vectors
      (concat-byte-vectors
        (emit-mov-rsi-from-local (native-value-window-spill-offset frame-base-slot-count shift-idx))
        (emit-mov-local-from-rsi (native-value-window-spill-offset frame-base-slot-count (- shift-idx consumed-local-count))))
      (emit-store-window-spill-shifts-x86-consumed frame-base-slot-count (+ shift-idx 1) last-shift-idx consumed-local-count))))

(defn emit-store-bundle-x86 [store-bytes frame-base-slot-count current-depth]
  (if (>= current-depth 4)
    (concat-byte-vectors
      (concat-byte-vectors
        (concat-byte-vectors
          store-bytes
          (emit-mov-rax-from-local (native-value-window-spill-offset frame-base-slot-count 0)))
        (emit-mov-rcx-from-local (native-value-window-spill-offset frame-base-slot-count 1)))
      (emit-store-window-spill-shifts-x86 frame-base-slot-count 2 (- current-depth 3)))
    (if (= current-depth 3)
      (concat-byte-vectors
        store-bytes
        (emit-mov-rax-from-local (native-value-window-spill-offset frame-base-slot-count 0)))
      store-bytes)))

(defn emit-i32-store-bundle-x86 [offset frame-base-slot-count current-depth]
  (emit-store-bundle-x86 (emit-mov-rcx-plus-offset-from-eax offset) frame-base-slot-count current-depth))

(defn emit-i64-store-bundle-x86 [offset frame-base-slot-count current-depth]
  (emit-store-bundle-x86 (emit-mov-rcx-plus-offset-from-rax offset) frame-base-slot-count current-depth))

(defn emit-consume-three-window-spill-shifts-x86 [frame-base-slot-count shift-idx last-shift-idx]
  (if (> shift-idx last-shift-idx)
    (vector-new 0)
    (concat-byte-vectors
      (concat-byte-vectors
        (emit-mov-rsi-from-local (native-value-window-spill-offset frame-base-slot-count shift-idx))
        (emit-mov-local-from-rsi (native-value-window-spill-offset frame-base-slot-count (- shift-idx 3))))
      (emit-consume-three-window-spill-shifts-x86 frame-base-slot-count (+ shift-idx 1) last-shift-idx))))

(defn emit-consume-three-bundle-x86 [op-bytes frame-base-slot-count current-depth]
  (if (>= current-depth 5)
    (concat-byte-vectors
      (concat-byte-vectors
        (concat-byte-vectors
          op-bytes
          (emit-mov-rax-from-local (native-value-window-spill-offset frame-base-slot-count 1)))
        (emit-mov-rcx-from-local (native-value-window-spill-offset frame-base-slot-count 2)))
      (emit-consume-three-window-spill-shifts-x86 frame-base-slot-count 3 (- current-depth 3)))
    (if (= current-depth 4)
      (concat-byte-vectors
        op-bytes
        (emit-mov-rax-from-local (native-value-window-spill-offset frame-base-slot-count 1)))
      op-bytes)))

(defn emit-memory-copy-bundle-x86 [frame-base-slot-count current-depth]
  (let [copy-bytes (concat-byte-vectors
                     (concat-byte-vectors
                       (concat-byte-vectors
                         (emit-mov-rdi-from-local (native-value-window-spill-offset frame-base-slot-count 0))
                         (emit-mov-rsi-rcx))
                       (emit-mov-rcx-rax))
                     (emit-rep-movsb))]
    (emit-consume-three-bundle-x86 copy-bytes frame-base-slot-count current-depth)))

(defn emit-memory-fill-bundle-x86 [frame-base-slot-count current-depth]
  (let [fill-bytes (concat-byte-vectors
                     (concat-byte-vectors
                       (concat-byte-vectors
                         (concat-byte-vectors
                           (emit-mov-rdx-rcx)
                           (emit-mov-rdi-from-local (native-value-window-spill-offset frame-base-slot-count 0)))
                         (emit-mov-rcx-rax))
                       (emit-mov-rax-rdx))
                     (emit-rep-stosb))]
    (emit-consume-three-bundle-x86 fill-bytes frame-base-slot-count current-depth)))

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
      (if (= opcode 74)
        ;; root_push -> non-moving native heap では root slot 0 を返す
        (emit-root-push-x86)
      (if (= opcode 75)
        ;; root_pop -> push dummy unit value
        (emit-i32-const-x86 0)
      (if (= opcode 10)
        ;; local.get -> rcx へ退避してから mov rax, [rbp-offset]
        (emit-local-get-x86 (local-slot-offset operand))
        (if (= opcode 11)
          ;; local.set -> mov [rbp-offset], rax
          (emit-mov-local-from-rax (local-slot-offset operand))
            (if (= opcode 20)
              ;; i64.add -> add rax, rcx
              (byte-vector-3 72 1 200)
              (if (= opcode 21)
                ;; i64.sub -> rcx - rax
                (concat-byte-vectors-rooted
                  (byte-vector-4 72 41 193 72)
                  (byte-vector-2 137 200))
              (if (= opcode 22)
                ;; i64.mul -> imul rax, rcx
                (emit-imul-rax-rcx)
                (if (= opcode 23)
                  ;; i64.div_s -> rcx / rax
                  (emit-i64-div-rax-rcx)
                  (if (= opcode 24)
                    ;; i32.add -> add eax, ecx
                    (emit-add-eax-ecx)
                    (if (= opcode 25)
                      ;; i32.mul -> imul eax, ecx
                      (emit-imul-eax-ecx)
                      (if (= opcode 26)
                        ;; i32.and -> and eax, ecx
                        (emit-and-eax-ecx)
                        (if (= opcode 27)
                          ;; i32.or -> or eax, ecx
                          (emit-or-eax-ecx)
                          (if (= opcode 28)
                            ;; i64.rem_s -> rcx % rax
                            (emit-i64-rem-rax-rcx)
                            (if (= opcode 71)
                              ;; selfhost logical and -> and eax, ecx
                              (emit-and-eax-ecx)
                              (if (= opcode 72)
                                ;; selfhost logical or -> or eax, ecx
                                (emit-or-eax-ecx)
                                (if (= opcode 45)
                                  ;; i32.load -> mov eax, [rax+offset]
                                  (emit-mov-eax-from-rax-plus-offset operand)
                                  (if (= opcode 46)
                                    ;; i32.store -> mov dword ptr [rcx+offset], eax
                                    (emit-mov-rcx-plus-offset-from-eax operand)
                                    (if (= opcode 47)
                                      ;; i32.load8_u -> movzx eax, byte ptr [rax+offset]
                                      (emit-movzx-eax-from-rax-plus-offset operand)
                                      (if (= opcode 48)
                                        ;; i64.load -> mov rax, [rax+offset]
                                        (emit-mov-rax-from-rax-plus-offset operand)
                                        (if (= opcode 49)
                                          ;; i64.store -> mov qword ptr [rcx+offset], rax
                                          (emit-mov-rcx-plus-offset-from-rax operand)
                                          (if (= (is-i64-compare-opcode opcode) 1)
                                            (emit-i64-compare-x86 opcode)
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
                                                (byte-vector-1 144))))))))))))))))))))))))))))) ;; 0x90

(defn native-call-bundle-size-x86-twenty-plus-core [target-param-count]
  (if (= target-param-count 58)
    778
    (if (= target-param-count 57)
      763
    (if (= target-param-count 56)
      748
    (if (= target-param-count 55)
      733
      (if (= target-param-count 54)
        718
        (if (= target-param-count 53)
       703
         (if (= target-param-count 52)
           688
           (if (= target-param-count 51)
             673
             (if (= target-param-count 50)
               658
         (if (= target-param-count 49)
           643
           (if (= target-param-count 48)
             628
             (if (= target-param-count 47)
               613
               (if (= target-param-count 46)
          598
                 (if (= target-param-count 45)
                   583
                   (if (= target-param-count 44)
                     568
                     (if (= target-param-count 43)
                       553
                       (if (= target-param-count 42)
                         538
                         (if (= target-param-count 41)
                           523
                           (if (= target-param-count 40)
                             508
                          (if (= target-param-count 39)
                            493
                            (if (= target-param-count 38)
                              478
                              (if (= target-param-count 37)
                                462
                                (if (= target-param-count 36)
                                  447
                                  (if (= target-param-count 35)
                                    432
                                    (if (= target-param-count 34)
                                      417
                                      (if (= target-param-count 33)
                                        402
                                        (if (= target-param-count 32)
                                          387
                                          (if (= target-param-count 31)
                                            372
                                            (if (= target-param-count 30)
                                              357
                                              (if (= target-param-count 29)
                                                342
                                                (if (= target-param-count 28)
                                                  327
                                                  (if (= target-param-count 27)
                                                    312
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
                                                                214)))))))))))))))))))))))))))))))))))))))

(defn native-call-bundle-size-x86-twenty-plus [target-param-count]
  (if (> target-param-count 54)
    (- (* 15 target-param-count) 47)
    (native-call-bundle-size-x86-twenty-plus-core target-param-count)))

(defn native-call-bundle-size-x86 [target-param-count current-depth]
  (if (>= target-param-count 20)
    (native-call-bundle-size-x86-twenty-plus target-param-count)
    (if (= target-param-count 9)
      (if (>= current-depth 11) (+ 89 (* (- current-depth 10) 14)) (if (= current-depth 10) 89 82))
      (if (> target-param-count 9)
        (+ 82 (* (- target-param-count 9) 12))
        (if (= target-param-count 8)
        (if (>= current-depth 10) (+ 77 (* (- current-depth 9) 14)) (if (= current-depth 9) 77 70))
        (if (= target-param-count 7)
          (if (>= current-depth 9) (+ 68 (* (- current-depth 8) 14)) (if (= current-depth 8) 68 61))
          (if (= target-param-count 6)
            (if (>= current-depth 8) (+ 46 (* (- current-depth 7) 14)) (if (= current-depth 7) 46 39))
            (if (= target-param-count 5)
              (if (>= current-depth 7) (+ 36 (* (- current-depth 6) 14)) (if (= current-depth 6) 36 29))
               (if (= target-param-count 4)
                 (if (>= current-depth 6) (+ 32 (* (- current-depth 5) 14)) (if (= current-depth 5) 32 25))
                (if (= target-param-count 3)
                  (if (>= current-depth 5) (+ 25 (* (- current-depth 4) 14)) (if (= current-depth 4) 25 18))
                  (if (= target-param-count 2)
                    (if (>= current-depth 3) 18 11)
                    (if (= target-param-count 1)
                      10
                      (if (= current-depth 0)
                        5
                        (if (= current-depth 1)
                          7
                           (+ 14 (* (- current-depth 2) 14))))))))))))))))

(defn native-call-rel-next-offset-x86 [target-param-count current-depth]
  (if (>= target-param-count 20)
    (if (> target-param-count 54)
      (- (* 15 target-param-count) 54)
      (if (= target-param-count 54)
        710
        (if (= target-param-count 53)
          695
          (if (= target-param-count 52)
            680
            (if (= target-param-count 51)
              665
              (if (= target-param-count 50)
                650
                (if (= target-param-count 49)
                  635
                  (if (= target-param-count 48)
                    620
                    (if (= target-param-count 47)
                      605
                      (if (= target-param-count 46)
                        590
                        (if (= target-param-count 45)
                          575
                          (if (= target-param-count 44)
                            560
                            (if (= target-param-count 43)
                              545
                              (if (= target-param-count 42)
                                530
                                (if (= target-param-count 41)
                                  515
                                  (if (= target-param-count 40)
                                    500
                                    (if (= target-param-count 39)
                                      485
                                      (if (= target-param-count 38)
                                        470
                                        (if (= target-param-count 37)
                                          455
                                          (if (= target-param-count 36)
                                            440
                                            (if (= target-param-count 35)
                                              425
                                              (if (= target-param-count 34)
                                                410
                                                (if (= target-param-count 33)
                                                  395
                                                  (if (= target-param-count 32)
                                                    380
                                                    (if (= target-param-count 31)
                                                      365
                                                      (if (= target-param-count 30)
                                                        350
                                                        (if (= target-param-count 29)
                                                          335
                                                          (if (= target-param-count 28)
                                                            320
                                                            (if (= target-param-count 27)
                                                              305
                                                              (if (= target-param-count 26)
                                                                290
                                                                (if (= target-param-count 25)
                                                                  275
                                                                  (if (= target-param-count 24)
                                                                    260
                                                                    (if (= target-param-count 23)
                                                                      246
                                                                      (if (= target-param-count 22)
                                                                        231
                                                                        (if (= target-param-count 21)
                                                                          219
                                                                          207)))))))))))))))))))))))))))))))))))
    (if (> target-param-count 8)
      (+ 75 (* (- target-param-count 9) 12))
      (if (= target-param-count 8)
        63
        (if (= target-param-count 7)
          54
          (if (= target-param-count 6)
            39
            (if (= target-param-count 5)
              29
              (if (= target-param-count 4)
                25
                (if (= target-param-count 3)
                  18
                  (if (= target-param-count 2)
                    11
                    (if (= target-param-count 1)
                      9
                      (if (= current-depth 0)
                        5
                        (if (= current-depth 1)
                          6
                          (+ 13 (* (- current-depth 2) 14)))))))))))))))

(defn native-call-rel-x86 [target-offset target-param-count current-offset current-depth]
  (- target-offset (+ current-offset (native-call-rel-next-offset-x86 target-param-count current-depth))))

(defn x86-plain-two-to-one-needs-window-restore [opcode]
  (if (= opcode 20)
    1
    (if (= opcode 21)
      1
      (if (= opcode 22)
        1
        (if (= opcode 23)
          1
          (if (= opcode 24)
            1
            (if (= opcode 25)
              1
              (if (= opcode 26)
                1
                (if (= opcode 27)
                  1
                  (if (= opcode 28)
                    1
                    (if (= opcode 71)
                      1
                      (if (= opcode 72)
                        1
                        (is-i64-compare-opcode opcode)))))))))))))

(defn native-plain-two-to-one-bundle-size-x86 [opcode operand current-depth]
  (let [plain-size (native-plain-instr-size-x86 opcode operand)]
    (if (>= current-depth 3)
      (+ plain-size (+ 7 (* (- current-depth 3) 14)))
      plain-size)))

(defn native-instr-size-x86 [opcode operand function-metas current-depth]
  (if (= opcode 40)
    (native-call-bundle-size-x86
      (native-function-param-count (vector-get function-metas operand))
      current-depth)
    (if (= opcode 1)
      (if (>= current-depth 2) (+ 20 (* (- current-depth 2) 14)) (if (= current-depth 1) 13 10))
      (if (= opcode 3)
        (if (>= current-depth 2) (+ 15 (* (- current-depth 2) 14)) 8)
        (if (= opcode 74)
          2
        (if (= opcode 75)
          (if (>= current-depth 2) (+ 15 (* (- current-depth 2) 14)) 8)
          (if (= opcode 10)
          (if (>= current-depth 2) (+ 20 (* (- current-depth 2) 14)) 10)
          (if (= opcode 11)
            (if (>= current-depth 3) (+ 17 (* (- current-depth 3) 14)) (if (= current-depth 2) 10 7))
          (if (= opcode 44)
            (if (>= current-depth 3) (+ 10 (* (- current-depth 3) 14)) 3)
            (if (= opcode 76)
              (if (>= current-depth 3) (+ 10 (* (- current-depth 3) 14)) (if (= current-depth 2) 3 0))
              (if (= opcode 64)
                7
                (if (= opcode 67)
                  7
                  (if (= opcode 51)
                    7
                    (if (= opcode 50)
                      (if (>= current-depth 3) (+ 12 (* (- current-depth 3) 14)) 5)
                      (if (= opcode 59)
                        7
                        (if (= opcode 54)
                          7
                          (if (= opcode 52)
                            7
                            (if (= opcode 56)
                              7
                              (if (= opcode 57)
                                7
                                (if (= opcode 53)
                                  (if (>= current-depth 3) (+ 12 (* (- current-depth 3) 14)) 5)
                                 (if (= opcode 55)
                                   (if (>= current-depth 3) (+ 12 (* (- current-depth 3) 14)) 5)
                                   (if (= opcode 58)
                                     (if (>= current-depth 3) (+ 12 (* (- current-depth 3) 14)) 5)
                                      (if (= opcode 69)
                                        (if (>= current-depth 4) (+ 19 (* (- current-depth 4) 14)) 12)
                                        (if (= opcode 70)
                                          (if (>= current-depth 3) (+ 12 (* (- current-depth 3) 14)) 5)
                                          (if (= opcode 60)
                                            (if (>= current-depth 2) (+ 14 (* (- current-depth 2) 14)) (if (= current-depth 1) 7 5))
                                            (if (= opcode 61)
                                              7
                                              (if (= opcode 62)
                                                (if (>= current-depth 4) (+ 19 (* (- current-depth 4) 14)) 12)
                                                (if (= opcode 63)
                                                  (if (>= current-depth 3) (+ 12 (* (- current-depth 3) 14)) 5)
                                                  (if (= opcode 73)
                                                    7
                                                    (if (= (x86-plain-two-to-one-needs-window-restore opcode) 1)
                                                      (native-plain-two-to-one-bundle-size-x86 opcode operand current-depth)
                                                      (native-plain-instr-size-x86 opcode operand)))))))))))))))))))))))))))))))

(defn make-x86-body-size-context [ir-func function-metas len]
  (vector-push
    (vector-push
      (vector-push (vector-new 3) ir-func)
      function-metas)
    len))

(defn x86-body-size-loop-ctx [ctx idx total current-depth]
  (let [ir-func (vector-get ctx 0)
    function-metas (vector-get ctx 1)
    len (vector-get ctx 2)]
  (if (>= idx len)
    total
    (let [instr (vector-get ir-func idx)
      opcode (vector-get instr 0)
      operand (vector-get instr 1)
      next-total (+ total (native-instr-size-x86 opcode operand function-metas current-depth))
      next-depth (apply-stack-delta current-depth (opcode-stack-delta opcode operand function-metas))]
      (x86-body-size-loop-ctx ctx (+ idx 1) next-total next-depth)))))

(defn native-function-body-size-x86-loop [ir-func function-metas idx len total current-depth]
  (let [ctx (make-x86-body-size-context ir-func function-metas len)]
    (x86-body-size-loop-ctx ctx idx total current-depth)))

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

(defn native-param-spill-bytes-x86-twenty-to-twenty-nine [param-count]
  (if (= param-count 29)
    322
    (native-param-spill-bytes-x86-twenty-to-twenty-eight param-count)))

(defn native-param-spill-bytes-x86-twenty-to-thirty [param-count]
  (if (= param-count 30)
    336
    (native-param-spill-bytes-x86-twenty-to-twenty-nine param-count)))

(defn native-param-spill-bytes-x86-twenty-to-thirty-one [param-count]
  (if (= param-count 31)
    350
    (native-param-spill-bytes-x86-twenty-to-thirty param-count)))

(defn native-param-spill-bytes-x86-twenty-to-thirty-two [param-count]
  (if (= param-count 32)
    364
    (native-param-spill-bytes-x86-twenty-to-thirty-one param-count)))

(defn native-param-spill-bytes-x86-twenty-to-thirty-three [param-count]
  (if (= param-count 33)
    378
    (native-param-spill-bytes-x86-twenty-to-thirty-two param-count)))

(defn native-param-spill-bytes-x86-twenty-to-thirty-four [param-count]
  (if (= param-count 34)
    392
    (native-param-spill-bytes-x86-twenty-to-thirty-three param-count)))

(defn native-param-spill-bytes-x86-twenty-to-thirty-five [param-count]
  (if (= param-count 35)
    406
    (native-param-spill-bytes-x86-twenty-to-thirty-four param-count)))

(defn native-param-spill-bytes-x86-twenty-to-thirty-six [param-count]
  (if (= param-count 36)
    420
    (native-param-spill-bytes-x86-twenty-to-thirty-five param-count)))

(defn native-param-spill-bytes-x86-twenty-to-thirty-seven [param-count]
  (if (= param-count 37)
    434
    (native-param-spill-bytes-x86-twenty-to-thirty-six param-count)))

(defn native-param-spill-bytes-x86-twenty-to-thirty-eight [param-count]
  (if (= param-count 38)
    448
    (native-param-spill-bytes-x86-twenty-to-thirty-seven param-count)))

(defn native-param-spill-bytes-x86-twenty-to-thirty-nine [param-count]
  (if (= param-count 39)
    462
    (native-param-spill-bytes-x86-twenty-to-thirty-eight param-count)))

(defn native-param-spill-bytes-x86-twenty-to-forty [param-count]
  (if (= param-count 40)
    476
    (native-param-spill-bytes-x86-twenty-to-thirty-nine param-count)))

(defn native-param-spill-bytes-x86-twenty-to-forty-one [param-count]
  (if (= param-count 41)
    490
    (native-param-spill-bytes-x86-twenty-to-forty param-count)))

(defn native-param-spill-bytes-x86-twenty-to-forty-two [param-count]
  (if (= param-count 42)
    504
    (native-param-spill-bytes-x86-twenty-to-forty-one param-count)))

(defn native-param-spill-bytes-x86-twenty-to-forty-three [param-count]
  (if (= param-count 43)
    518
    (native-param-spill-bytes-x86-twenty-to-forty-two param-count)))

(defn native-param-spill-bytes-x86-twenty-to-forty-four [param-count]
  (if (= param-count 44)
    532
    (native-param-spill-bytes-x86-twenty-to-forty-three param-count)))

(defn native-param-spill-bytes-x86-twenty-to-forty-five [param-count]
  (if (= param-count 45)
    546
    (native-param-spill-bytes-x86-twenty-to-forty-four param-count)))

(defn native-param-spill-bytes-x86-twenty-to-forty-six [param-count]
  (if (= param-count 46)
    560
    (native-param-spill-bytes-x86-twenty-to-forty-five param-count)))

(defn native-param-spill-bytes-x86-twenty-to-forty-seven [param-count]
  (if (= param-count 47)
    574
    (native-param-spill-bytes-x86-twenty-to-forty-six param-count)))

(defn native-param-spill-bytes-x86-twenty-to-forty-eight [param-count]
  (if (= param-count 48)
    588
    (native-param-spill-bytes-x86-twenty-to-forty-seven param-count)))

(defn native-param-spill-bytes-x86-twenty-to-forty-nine [param-count]
  (if (= param-count 49)
    602
    (native-param-spill-bytes-x86-twenty-to-forty-eight param-count)))

(defn native-param-spill-bytes-x86-twenty-to-fifty [param-count]
  (if (= param-count 50)
    616
    (native-param-spill-bytes-x86-twenty-to-forty-nine param-count)))

(defn native-param-spill-bytes-x86-twenty-to-fifty-one [param-count]
  (if (= param-count 51)
    630
    (native-param-spill-bytes-x86-twenty-to-fifty param-count)))

(defn native-param-spill-bytes-x86-twenty-to-fifty-two [param-count]
  (if (= param-count 52)
    644
    (native-param-spill-bytes-x86-twenty-to-fifty-one param-count)))

(defn native-param-spill-bytes-x86-twenty-to-fifty-three [param-count]
  (if (= param-count 53)
    658
    (native-param-spill-bytes-x86-twenty-to-fifty-two param-count)))

(defn native-param-spill-bytes-x86-twenty-to-fifty-four [param-count]
  (if (= param-count 54)
    672
    (native-param-spill-bytes-x86-twenty-to-fifty-three param-count)))

(defn native-param-spill-bytes-x86-twenty-to-fifty-five [param-count]
  (if (= param-count 55)
    686
    (native-param-spill-bytes-x86-twenty-to-fifty-four param-count)))

(defn native-param-spill-bytes-x86-twenty-to-fifty-six [param-count]
  (if (= param-count 56)
    700
    (native-param-spill-bytes-x86-twenty-to-fifty-five param-count)))

(defn native-param-spill-bytes-x86-twenty-to-fifty-seven [param-count]
  (if (= param-count 57)
    714
    (native-param-spill-bytes-x86-twenty-to-fifty-six param-count)))

(defn native-param-spill-bytes-x86-twenty-to-fifty-eight [param-count]
  (if (= param-count 58)
    728
    (native-param-spill-bytes-x86-twenty-to-fifty-seven param-count)))

(defn native-param-spill-bytes-x86-twenty-to-sixty [param-count]
  (if (= param-count 60)
    756
    (if (= param-count 59)
      742
      (native-param-spill-bytes-x86-twenty-to-fifty-eight param-count))))

(defn native-param-spill-bytes-x86-twenty-plus [param-count]
  (if (> param-count 60)
    (let [stack-param-count (- param-count 6)
      imm8-count (if (> stack-param-count 14) 14 stack-param-count)]
      (+ 42 (+ (* imm8-count 11) (* (- stack-param-count imm8-count) 14))))
    (native-param-spill-bytes-x86-twenty-to-sixty param-count)))

(defn native-function-size-x86 [func-meta function-metas]
  (let [param-count (native-function-param-count func-meta)
    local-count (native-function-local-count func-meta)
    ir-func (native-function-ir func-meta)
    stack-bytes (native-local-stack-bytes-with-window-x86 ir-func (+ (+ param-count local-count) 1) function-metas)
    frame-bytes (if (> stack-bytes 0) 14 0)
    param-spill-bytes (if (>= param-count 20)
                        (native-param-spill-bytes-x86-twenty-plus param-count)
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
    body-size-ctx (make-x86-body-size-context ir-func function-metas (vector-length ir-func))
    body-bytes (if (> (vector-length ir-func) 4096)
                 (+ 131072 (* (vector-length ir-func) 16))
                 (x86-body-size-loop-ctx body-size-ctx 0 0 0))]
    (+ (+ (+ 6 frame-bytes) param-spill-bytes) body-bytes)))

(defn collect-callable-function-starts-x86-loop [functions idx len starts offset]
  (if (>= idx len)
    starts
    (let [func-meta (vector-get functions idx)
      next-starts (vector-push starts offset)
      next-offset (+ offset (native-function-size-x86 func-meta functions))]
      (collect-callable-function-starts-x86-loop functions (+ idx 1) len next-starts next-offset))))

(defn collect-callable-function-starts-x86 [functions import-count]
  (collect-callable-function-starts-x86-loop functions import-count (vector-length functions) (vector-new 8) 0))

(defn callable-user-total-size-x86-loop [functions idx len total]
  (if (>= idx len)
    total
    (let [func-meta (vector-get functions idx)
      next-total (+ total (native-function-size-x86 func-meta functions))]
      (callable-user-total-size-x86-loop functions (+ idx 1) len next-total))))

(defn callable-user-total-size-x86 [functions import-count]
  (callable-user-total-size-x86-loop functions import-count (vector-length functions) 0))

(defn collect-function-starts-x86 [functions]
  (collect-callable-function-starts-x86 functions 0))

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

(defn emit-call-bundle-x86-twenty-to-twenty-nine [target-param-count rel frame-base-slot-count]
  (if (= target-param-count 29)
    (emit-twenty-nine-arg-call-x86 rel frame-base-slot-count)
    (emit-call-bundle-x86-twenty-to-twenty-eight target-param-count rel frame-base-slot-count)))

(defn emit-call-bundle-x86-twenty-to-thirty [target-param-count rel frame-base-slot-count]
  (if (= target-param-count 30)
    (emit-thirty-arg-call-x86 rel frame-base-slot-count)
    (emit-call-bundle-x86-twenty-to-twenty-nine target-param-count rel frame-base-slot-count)))

(defn emit-call-bundle-x86-twenty-to-thirty-one [target-param-count rel frame-base-slot-count]
  (if (= target-param-count 31)
    (emit-thirty-one-arg-call-x86 rel frame-base-slot-count)
    (emit-call-bundle-x86-twenty-to-thirty target-param-count rel frame-base-slot-count)))

(defn emit-call-bundle-x86-twenty-to-thirty-two [target-param-count rel frame-base-slot-count]
  (if (= target-param-count 32)
    (emit-thirty-two-arg-call-x86 rel frame-base-slot-count)
    (emit-call-bundle-x86-twenty-to-thirty-one target-param-count rel frame-base-slot-count)))

(defn emit-call-bundle-x86-twenty-to-thirty-three [target-param-count rel frame-base-slot-count]
  (if (= target-param-count 33)
    (emit-thirty-three-arg-call-x86 rel frame-base-slot-count)
    (emit-call-bundle-x86-twenty-to-thirty-two target-param-count rel frame-base-slot-count)))

(defn emit-call-bundle-x86-twenty-to-thirty-four [target-param-count rel frame-base-slot-count]
  (if (= target-param-count 34)
    (emit-thirty-four-arg-call-x86 rel frame-base-slot-count)
    (emit-call-bundle-x86-twenty-to-thirty-three target-param-count rel frame-base-slot-count)))

(defn emit-call-bundle-x86-twenty-to-thirty-five [target-param-count rel frame-base-slot-count]
  (if (= target-param-count 35)
    (emit-thirty-five-arg-call-x86 rel frame-base-slot-count)
    (emit-call-bundle-x86-twenty-to-thirty-four target-param-count rel frame-base-slot-count)))

(defn emit-call-bundle-x86-twenty-to-thirty-six [target-param-count rel frame-base-slot-count]
  (if (= target-param-count 36)
    (emit-thirty-six-arg-call-x86 rel frame-base-slot-count)
    (emit-call-bundle-x86-twenty-to-thirty-five target-param-count rel frame-base-slot-count)))

(defn emit-call-bundle-x86-twenty-to-thirty-seven [target-param-count rel frame-base-slot-count]
  (if (= target-param-count 37)
    (emit-thirty-seven-arg-call-x86 rel frame-base-slot-count)
    (emit-call-bundle-x86-twenty-to-thirty-six target-param-count rel frame-base-slot-count)))

(defn emit-call-bundle-x86-twenty-to-thirty-eight [target-param-count rel frame-base-slot-count]
  (if (= target-param-count 38)
    (emit-thirty-eight-arg-call-x86 rel frame-base-slot-count)
    (emit-call-bundle-x86-twenty-to-thirty-seven target-param-count rel frame-base-slot-count)))

(defn emit-call-bundle-x86-twenty-to-thirty-nine [target-param-count rel frame-base-slot-count]
  (if (= target-param-count 39)
    (emit-thirty-nine-arg-call-x86 rel frame-base-slot-count)
    (emit-call-bundle-x86-twenty-to-thirty-eight target-param-count rel frame-base-slot-count)))

(defn emit-call-bundle-x86-twenty-to-forty [target-param-count rel frame-base-slot-count]
  (if (= target-param-count 40)
    (emit-forty-arg-call-x86 rel frame-base-slot-count)
    (emit-call-bundle-x86-twenty-to-thirty-nine target-param-count rel frame-base-slot-count)))

(defn emit-call-bundle-x86-twenty-to-forty-one [target-param-count rel frame-base-slot-count]
  (if (= target-param-count 41)
    (emit-forty-one-arg-call-x86 rel frame-base-slot-count)
    (emit-call-bundle-x86-twenty-to-forty target-param-count rel frame-base-slot-count)))

(defn emit-call-bundle-x86-twenty-to-forty-two [target-param-count rel frame-base-slot-count]
  (if (= target-param-count 42)
    (emit-forty-two-arg-call-x86 rel frame-base-slot-count)
    (emit-call-bundle-x86-twenty-to-forty-one target-param-count rel frame-base-slot-count)))

(defn emit-call-bundle-x86-twenty-to-forty-three [target-param-count rel frame-base-slot-count]
  (if (= target-param-count 43)
    (emit-forty-three-arg-call-x86 rel frame-base-slot-count)
    (emit-call-bundle-x86-twenty-to-forty-two target-param-count rel frame-base-slot-count)))

(defn emit-call-bundle-x86-twenty-to-forty-four [target-param-count rel frame-base-slot-count]
  (if (= target-param-count 44)
    (emit-forty-four-arg-call-x86 rel frame-base-slot-count)
    (emit-call-bundle-x86-twenty-to-forty-three target-param-count rel frame-base-slot-count)))

(defn emit-call-bundle-x86-twenty-to-forty-five [target-param-count rel frame-base-slot-count]
  (if (= target-param-count 45)
    (emit-forty-five-arg-call-x86 rel frame-base-slot-count)
    (emit-call-bundle-x86-twenty-to-forty-four target-param-count rel frame-base-slot-count)))

(defn emit-call-bundle-x86-twenty-to-forty-six [target-param-count rel frame-base-slot-count]
  (if (= target-param-count 46)
    (emit-forty-six-arg-call-x86 rel frame-base-slot-count)
    (emit-call-bundle-x86-twenty-to-forty-five target-param-count rel frame-base-slot-count)))

(defn emit-call-bundle-x86-twenty-to-forty-seven [target-param-count rel frame-base-slot-count]
  (if (= target-param-count 47)
    (emit-forty-seven-arg-call-x86 rel frame-base-slot-count)
    (emit-call-bundle-x86-twenty-to-forty-six target-param-count rel frame-base-slot-count)))

(defn emit-call-bundle-x86-twenty-to-forty-eight [target-param-count rel frame-base-slot-count]
  (if (= target-param-count 48)
    (emit-forty-eight-arg-call-x86 rel frame-base-slot-count)
    (emit-call-bundle-x86-twenty-to-forty-seven target-param-count rel frame-base-slot-count)))

(defn emit-call-bundle-x86-twenty-to-forty-nine [target-param-count rel frame-base-slot-count]
  (if (= target-param-count 49)
    (emit-forty-nine-arg-call-x86 rel frame-base-slot-count)
    (emit-call-bundle-x86-twenty-to-forty-eight target-param-count rel frame-base-slot-count)))

(defn emit-call-bundle-x86-twenty-to-fifty [target-param-count rel frame-base-slot-count]
  (if (= target-param-count 50)
    (emit-fifty-arg-call-x86 rel frame-base-slot-count)
    (emit-call-bundle-x86-twenty-to-forty-nine target-param-count rel frame-base-slot-count)))

(defn emit-call-bundle-x86-twenty-to-fifty-one [target-param-count rel frame-base-slot-count]
  (if (= target-param-count 51)
    (emit-fifty-one-arg-call-x86 rel frame-base-slot-count)
    (emit-call-bundle-x86-twenty-to-fifty target-param-count rel frame-base-slot-count)))

(defn emit-call-bundle-x86-twenty-to-fifty-two [target-param-count rel frame-base-slot-count]
  (if (= target-param-count 52)
    (emit-fifty-two-arg-call-x86 rel frame-base-slot-count)
    (emit-call-bundle-x86-twenty-to-fifty-one target-param-count rel frame-base-slot-count)))

(defn emit-call-bundle-x86-twenty-to-fifty-three [target-param-count rel frame-base-slot-count]
  (if (= target-param-count 53)
    (emit-fifty-three-arg-call-x86 rel frame-base-slot-count)
    (emit-call-bundle-x86-twenty-to-fifty-two target-param-count rel frame-base-slot-count)))

(defn emit-call-bundle-x86-twenty-to-fifty-four [target-param-count rel frame-base-slot-count]
  (if (= target-param-count 54)
    (emit-fifty-four-arg-call-x86 rel frame-base-slot-count)
    (emit-call-bundle-x86-twenty-to-fifty-three target-param-count rel frame-base-slot-count)))

(defn emit-call-bundle-x86-twenty-to-fifty-five [target-param-count rel frame-base-slot-count]
  (if (= target-param-count 55)
    (emit-fifty-five-arg-call-x86 rel frame-base-slot-count)
    (emit-call-bundle-x86-twenty-to-fifty-four target-param-count rel frame-base-slot-count)))

(defn emit-call-bundle-x86-twenty-to-fifty-six [target-param-count rel frame-base-slot-count]
  (if (= target-param-count 56)
    (emit-fifty-six-arg-call-x86 rel frame-base-slot-count)
    (emit-call-bundle-x86-twenty-to-fifty-five target-param-count rel frame-base-slot-count)))

(defn emit-call-bundle-x86-twenty-to-fifty-seven [target-param-count rel frame-base-slot-count]
  (if (= target-param-count 57)
    (emit-fifty-seven-arg-call-x86 rel frame-base-slot-count)
    (emit-call-bundle-x86-twenty-to-fifty-six target-param-count rel frame-base-slot-count)))

(defn emit-call-bundle-x86-twenty-to-fifty-eight [target-param-count rel frame-base-slot-count]
  (if (= target-param-count 58)
    (emit-fifty-eight-arg-call-x86 rel frame-base-slot-count)
    (emit-call-bundle-x86-twenty-to-fifty-seven target-param-count rel frame-base-slot-count)))

(defn emit-call-bundle-x86-twenty-to-sixty [target-param-count rel frame-base-slot-count]
  (if (> target-param-count 60)
    (emit-twenty-plus-arg-call-x86 target-param-count rel frame-base-slot-count)
    (if (= target-param-count 60)
      (emit-sixty-arg-call-x86 rel frame-base-slot-count)
      (if (= target-param-count 59)
        (emit-fifty-nine-arg-call-x86 rel frame-base-slot-count)
        (emit-call-bundle-x86-twenty-to-fifty-eight target-param-count rel frame-base-slot-count)))))

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
    (emit-nine-arg-call-x86 rel frame-base-slot-count current-depth)
    (if (= target-param-count 8)
      (emit-eight-arg-call-x86 rel frame-base-slot-count current-depth)
      (if (= target-param-count 7)
        (emit-seven-arg-call-x86 rel frame-base-slot-count current-depth)
        (if (= target-param-count 6)
          (emit-six-arg-call-x86 rel frame-base-slot-count current-depth)
          (if (= target-param-count 5)
            (emit-five-arg-call-x86 rel frame-base-slot-count current-depth)
              (if (= target-param-count 4)
                (emit-four-arg-call-x86 rel frame-base-slot-count current-depth)
                (if (= target-param-count 3)
                  (emit-three-arg-call-x86 rel frame-base-slot-count current-depth)
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
                    (emit-zero-arg-call-x86 rel frame-base-slot-count current-depth)))))))))))

(defn emit-x86-selfhost-command-line-arg-helper []
  (let [part1 (concat-byte-vectors-rooted
                (byte-vector-4 72 133 192 124)
                (byte-vector-4 10 76 57 224))
    part2 (concat-byte-vectors-rooted
            (byte-vector-4 125 5 73 139)
            (concat-byte-vectors-rooted
              (byte-vector-4 4 199 195 49)
              (byte-vector-2 192 195)))]
    (concat-byte-vectors-rooted part1 part2)))

(defn emit-x86-selfhost-string-length-helper []
  (let [part1 (byte-vector-4 72 133 192 116)
    part2 (byte-vector-4 44 121 9 72)
    part3 (byte-vector-4 15 186 240 63)
    part4 (byte-vector-4 139 64 4 195)
    part5 (byte-vector-4 72 61 0 0)
    part6 (byte-vector-4 0 64 115 7)
    part7 (byte-vector-4 76 1 240 139)
    part8 (byte-vector-4 64 4 195 72)
    part9 (byte-vector-4 49 201 128 60)
    part10 (byte-vector-4 8 0 116 5)
    part11 (byte-vector-4 72 255 193 235)
    part12 (byte-vector-4 245 72 137 200)
    part13 (byte-vector-4 195 49 192 195)]
    (concat-five-byte-vectors-rooted
      (concat-five-byte-vectors-rooted part1 part2 part3 part4 part5)
      part6
      part7
      part8
      (concat-five-byte-vectors-rooted part9 part10 part11 part12 part13))))

(defn emit-x86-selfhost-string-char-at-helper []
  (let [part1 (byte-vector-4 72 133 192 120)
    part2 (byte-vector-4 63 72 133 201)
    part3 (byte-vector-4 116 58 72 133)
    part4 (byte-vector-4 201 121 16 72)
    part5 (byte-vector-4 15 186 241 63)
    part6 (byte-vector-4 59 65 4 115)
    part7 (byte-vector-4 43 15 182 68)
    part8 (byte-vector-4 1 8 195 72)
    part9 (byte-vector-4 129 249 0 0)
    part10 (byte-vector-4 0 64 115 14)
    part11 (byte-vector-4 76 1 241 59)
    part12 (byte-vector-4 65 4 115 20)
    part13 (byte-vector-4 15 182 68 1)
    part14 (byte-vector-4 8 195 72 129)
    part15 (byte-vector-4 249 0 16 0)
    part16 (byte-vector-4 0 114 5 15)
    part17 (byte-vector-4 182 4 1 195)
    part18 (byte-vector-3 49 192 195)]
    (concat-five-byte-vectors-rooted
      (concat-five-byte-vectors-rooted part1 part2 part3 part4 part5)
      part6
      part7
      part8
      (concat-five-byte-vectors-rooted
        (concat-five-byte-vectors-rooted part9 part10 part11 part12 part13)
        part14
        part15
        part16
        (concat-byte-vectors-rooted part17 part18)))))

(defn emit-x86-selfhost-print-helper []
  (let [part1 (byte-vector-4 83 72 131 236)
    part2 (byte-vector-4 32 72 137 195)
    part3 (byte-vector-4 72 141 116 36)
    part4 (byte-vector-4 31 198 6 10)
    part5 (byte-vector-4 72 199 193 1)
    part6 (byte-vector-4 0 0 0 72)
    part7 (byte-vector-4 131 251 0 117)
    part8 (byte-vector-4 11 72 255 206)
    part9 (byte-vector-4 198 6 48 72)
    part10 (byte-vector-4 255 193 235 35)
    part11 (byte-vector-4 72 137 216 72)
    part12 (byte-vector-4 49 210 73 199)
    part13 (byte-vector-4 192 10 0 0)
    part14 (byte-vector-4 0 73 247 240)
    part15 (byte-vector-4 128 194 48 72)
    part16 (byte-vector-4 255 206 136 22)
    part17 (byte-vector-4 72 255 193 72)
    part18 (byte-vector-4 137 195 72 133)
    part19 (byte-vector-4 219 117 221 72)
    part20 (byte-vector-4 199 192 1 0)
    part21 (byte-vector-4 0 0 72 199)
    part22 (byte-vector-4 199 1 0 0)
    part23 (byte-vector-4 0 72 137 202)
    part24 (byte-vector-4 15 5 49 192)
    part25 (byte-vector-4 72 131 196 32)
    part26 (byte-vector-2 91 195)]
    (concat-five-byte-vectors-rooted
      (concat-five-byte-vectors-rooted
        (concat-five-byte-vectors-rooted part1 part2 part3 part4 part5)
        (concat-five-byte-vectors-rooted part6 part7 part8 part9 part10)
        (concat-five-byte-vectors-rooted part11 part12 part13 part14 part15)
        (concat-five-byte-vectors-rooted part16 part17 part18 part19 part20)
        part21)
      part22
      part23
      part24
      (concat-three-byte-vectors-rooted part25 part26 (vector-new 0)))))

(defn emit-x86-selfhost-vector-new-helper []
  (let [part1 (byte-vector-4 81 80 72 141)
    part2 (byte-vector-4 52 197 16 0)
    part3 (byte-vector-4 0 0 72 49)
    part4 (byte-vector-4 255 72 199 194)
    part5 (byte-vector-4 3 0 0 0)
    part6 (byte-vector-4 73 199 194 34)
    part7 (byte-vector-4 0 0 0 73)
    part8 (byte-vector-4 199 192 255 255)
    part9 (byte-vector-4 255 255 77 49)
    part10 (byte-vector-4 201 72 199 192)
    part11 (byte-vector-4 9 0 0 0)
    part12 (byte-vector-4 15 5 72 133)
    part13 (byte-vector-4 192 120 63 65)
    part14 (byte-vector-4 91 72 137 194)
    part15 (byte-vector-4 72 131 194 16)
    part16 (byte-vector-4 76 137 217 72)
    part17 (byte-vector-4 191 144 144 144)
    part18 (byte-vector-4 144 144 144 144)
    part19 (byte-vector-4 144 72 133 201)
    part20 (byte-vector-4 116 12 72 137)
    part21 (byte-vector-4 58 72 131 194)
    part22 (byte-vector-4 8 72 255 201)
    part23 (byte-vector-4 117 244 199 0)
    part24 (byte-vector-4 2 0 0 0)
    part25 (byte-vector-4 68 137 88 4)
    part26 (byte-vector-4 199 64 8 0)
    part27 (byte-vector-4 0 0 0 72)
    part28 (byte-vector-4 15 186 232 63)
    part29 (byte-vector-4 89 195 88 49)
    part30 (byte-vector-3 192 89 195)]
    (concat-five-byte-vectors-rooted
      (concat-five-byte-vectors-rooted part1 part2 part3 part4 part5)
      (concat-five-byte-vectors-rooted part6 part7 part8 part9 part10)
      (concat-five-byte-vectors-rooted part11 part12 part13 part14 part15)
      (concat-five-byte-vectors-rooted part16 part17 part18 part19 part20)
      (concat-three-byte-vectors-rooted
        (concat-five-byte-vectors-rooted part21 part22 part23 part24 part25)
        (concat-five-byte-vectors-rooted part26 part27 part28 part29 part30)
        (vector-new 0)))))

(defn emit-x86-selfhost-vector-length-helper []
  (let [part1 (byte-vector-4 72 133 192 121)
    part2 (byte-vector-4 9 72 15 186)
    part3 (byte-vector-4 240 63 139 64)
    part4 (byte-vector-4 8 195 49 192)
    part5 (byte-vector-1 195)]
    (concat-five-byte-vectors-rooted part1 part2 part3 part4 part5)))

(defn emit-x86-selfhost-vector-get-helper []
  (let [part1 (byte-vector-5 72 133 201 121 39)
    part2 (byte-vector-5 72 129 249 0 240)
    part3 (byte-vector-5 255 255 127 30 72)
    part4 (byte-vector-5 15 186 241 63 72)
    part5 (byte-vector-5 137 202 72 193 234)
    part6 (byte-vector-5 47 117 16 131 57)
    part7 (byte-vector-5 2 117 11 59 65)
    part8 (byte-vector-5 8 115 6 72 139)
    part9 (byte-vector-5 68 193 16 195 49)
    part10 (byte-vector-2 192 195)]
    (concat-byte-vectors-rooted
      (concat-five-byte-vectors-rooted part1 part2 part3 part4 part5)
      (concat-five-byte-vectors-rooted part6 part7 part8 part9 part10))))

(defn emit-x86-selfhost-vector-push-helper []
  (let [part1 (byte-vector-4 65 84 65 85)
    part2 (byte-vector-4 72 133 201 15)
    part3 (byte-vector-4 137 185 0 0)
    part4 (byte-vector-4 0 72 15 186)
    part5 (byte-vector-4 241 63 139 81)
    part6 (byte-vector-4 8 68 139 65)
    part7 (byte-vector-4 4 68 57 194)
    part8 (byte-vector-4 15 130 141 0)
    part9 (byte-vector-4 0 0 80 81)
    part10 (byte-vector-4 65 137 212 69)
    part11 (byte-vector-4 137 197 69 133)
    part12 (byte-vector-4 237 117 8 65)
    part13 (byte-vector-4 189 1 0 0)
    part14 (byte-vector-4 0 235 3 69)
    part15 (byte-vector-4 1 237 49 255)
    part16 (byte-vector-4 74 141 52 237)
    part17 (byte-vector-4 16 0 0 0)
    part18 (byte-vector-4 186 3 0 0)
    part19 (byte-vector-4 0 65 186 34)
    part20 (byte-vector-4 0 0 0 73)
    part21 (byte-vector-4 199 192 255 255)
    part22 (byte-vector-4 255 255 69 49)
    part23 (byte-vector-4 201 184 9 0)
    part24 (byte-vector-4 0 0 15 5)
    part25 (byte-vector-4 72 133 192 120)
    part26 (byte-vector-4 63 199 0 2)
    part27 (byte-vector-4 0 0 0 68)
    part28 (byte-vector-4 137 104 4 68)
    part29 (byte-vector-4 137 96 8 72)
    part30 (byte-vector-4 137 194 72 139)
    part31 (byte-vector-4 52 36 72 141)
    part32 (byte-vector-4 118 16 72 141)
    part33 (byte-vector-4 120 16 68 137)
    part34 (byte-vector-4 225 243 72 165)
    part35 (byte-vector-4 94 88 68 137)
    part36 (byte-vector-4 225 72 137 68)
    part37 (byte-vector-4 202 16 255 193)
    part38 (byte-vector-4 137 74 8 72)
    part39 (byte-vector-4 15 186 234 63)
    part40 (byte-vector-4 72 137 208 65)
    part41 (byte-vector-4 93 65 92 195)
    part42 (byte-vector-4 72 131 196 16)
    part43 (byte-vector-4 65 93 65 92)
    part44 (byte-vector-4 49 192 195 72)
    part45 (byte-vector-4 137 68 209 16)
    part46 (byte-vector-4 255 194 137 81)
    part47 (byte-vector-4 8 72 15 186)
    part48 (byte-vector-4 233 63 72 137)
    part49 (byte-vector-4 200 65 93 65)
    part50 (byte-vector-4 92 195 65 93)
    part51 (byte-vector-4 65 92 49 192)
    part52 (byte-vector-1 195)
    group1 (concat-five-byte-vectors-rooted part1 part2 part3 part4 part5)
    group2 (concat-five-byte-vectors-rooted part6 part7 part8 part9 part10)
    group3 (concat-five-byte-vectors-rooted part11 part12 part13 part14 part15)
    group4 (concat-five-byte-vectors-rooted part16 part17 part18 part19 part20)
    group5 (concat-five-byte-vectors-rooted part21 part22 part23 part24 part25)
    group6 (concat-five-byte-vectors-rooted part26 part27 part28 part29 part30)
    group7 (concat-five-byte-vectors-rooted part31 part32 part33 part34 part35)
    group8 (concat-five-byte-vectors-rooted part36 part37 part38 part39 part40)
    group9 (concat-five-byte-vectors-rooted part41 part42 part43 part44 part45)
    group10 (concat-five-byte-vectors-rooted part46 part47 part48 part49 part50)
    group11 (concat-three-byte-vectors-rooted part51 part52 (vector-new 0))
    head (concat-five-byte-vectors-rooted group1 group2 group3 group4 group5)
    tail (concat-five-byte-vectors-rooted group6 group7 group8 group9 group10)]
    (concat-three-byte-vectors-rooted head tail group11)))

(defn emit-x86-selfhost-ref-new-helper []
  (let [part1 (byte-vector-4 81 80 72 49)
    part2 (byte-vector-4 255 72 199 198)
    part3 (byte-vector-4 16 0 0 0)
    part4 (byte-vector-4 72 199 194 3)
    part5 (byte-vector-4 0 0 0 73)
    part6 (byte-vector-4 199 194 34 0)
    part7 (byte-vector-4 0 0 73 199)
    part8 (byte-vector-4 192 255 255 255)
    part9 (byte-vector-4 255 77 49 201)
    part10 (byte-vector-4 72 199 192 9)
    part11 (byte-vector-4 0 0 0 15)
    part12 (byte-vector-4 5 72 133 192)
    part13 (byte-vector-4 120 18 199 0)
    part14 (byte-vector-4 3 0 0 0)
    part15 (byte-vector-4 90 72 137 80)
    part16 (byte-vector-4 8 72 15 186)
    part17 (byte-vector-4 232 63 89 195)
    part18 (byte-vector-4 88 49 192 89)
    part19 (byte-vector-1 195)]
    (concat-five-byte-vectors-rooted
      (concat-five-byte-vectors-rooted part1 part2 part3 part4 part5)
      (concat-five-byte-vectors-rooted part6 part7 part8 part9 part10)
      (concat-five-byte-vectors-rooted part11 part12 part13 part14 part15)
      (concat-three-byte-vectors-rooted part16 part17 part18)
      part19)))

(defn emit-x86-selfhost-ref-get-helper []
  (let [part1 (byte-vector-4 72 133 192 121)
    part2 (byte-vector-4 10 72 15 186)
    part3 (byte-vector-4 240 63 72 139)
    part4 (byte-vector-4 64 8 195 49)
    part5 (byte-vector-2 192 195)]
    (concat-five-byte-vectors-rooted part1 part2 part3 part4 part5)))

(defn emit-x86-selfhost-ref-set-helper []
  (let [part1 (byte-vector-4 72 133 201 121)
    part2 (byte-vector-4 12 72 15 186)
    part3 (byte-vector-4 241 63 72 137)
    part4 (byte-vector-4 65 8 49 192)
    part5 (byte-vector-4 195 49 192 195)]
    (concat-five-byte-vectors-rooted part1 part2 part3 part4 part5)))

(defn emit-x86-selfhost-substring-helper []
  (let [
    part1 (byte-vector-4 83 65 84 65)
    part2 (byte-vector-4 85 65 86 65)
    part3 (byte-vector-4 87 73 137 215)
    part4 (byte-vector-4 77 133 255 120)
    part5 (byte-vector-4 14 73 129 255)
    part6 (byte-vector-4 0 0 0 64)
    part7 (byte-vector-4 115 107 77 1)
    part8 (byte-vector-4 247 235 5 73)
    part9 (byte-vector-4 15 186 247 63)
    part10 (byte-vector-4 65 137 204 41)
    part11 (byte-vector-4 200 65 137 197)
    part12 (byte-vector-4 68 137 238 72)
    part13 (byte-vector-4 131 198 8 72)
    part14 (byte-vector-4 131 198 15 72)
    part15 (byte-vector-4 131 230 240 49)
    part16 (byte-vector-4 255 186 3 0)
    part17 (byte-vector-4 0 0 65 186)
    part18 (byte-vector-4 34 0 0 0)
    part19 (byte-vector-4 73 199 192 255)
    part20 (byte-vector-4 255 255 255 69)
    part21 (byte-vector-4 49 201 184 9)
    part22 (byte-vector-4 0 0 0 15)
    part23 (byte-vector-4 5 72 133 192)
    part24 (byte-vector-4 120 39 199 0)
    part25 (byte-vector-4 1 0 0 0)
    part26 (byte-vector-4 68 137 104 4)
    part27 (byte-vector-4 72 141 120 8)
    part28 (byte-vector-4 75 141 116 39)
    part29 (byte-vector-4 8 68 137 233)
    part30 (byte-vector-4 243 164 72 15)
    part31 (byte-vector-4 186 232 63 65)
    part32 (byte-vector-4 95 65 94 65)
    part33 (byte-vector-4 93 65 92 91)
    part34 (byte-vector-4 195 49 192 65)
    part35 (byte-vector-4 95 65 94 65)
    part36 (byte-vector-4 93 65 92 91)
    part37 (byte-vector-1 195)
    group1 (concat-five-byte-vectors-rooted part1 part2 part3 part4 part5)
    group2 (concat-five-byte-vectors-rooted part6 part7 part8 part9 part10)
    group3 (concat-five-byte-vectors-rooted part11 part12 part13 part14 part15)
    group4 (concat-five-byte-vectors-rooted part16 part17 part18 part19 part20)
    group5 (concat-five-byte-vectors-rooted part21 part22 part23 part24 part25)
    group6 (concat-five-byte-vectors-rooted part26 part27 part28 part29 part30)
    group7 (concat-five-byte-vectors-rooted part31 part32 part33 part34 part35)
    group8 (concat-byte-vectors-rooted part36 part37)
    head (concat-five-byte-vectors-rooted group1 group2 group3 group4 group5)
    tail (concat-three-byte-vectors-rooted group6 group7 group8)]
    (concat-byte-vectors-rooted head tail)))

(defn emit-x86-selfhost-string-concat-helper []
  (let [
    part1 (byte-vector-4 72 133 201 120)
    part2 (byte-vector-4 18 72 129 249)
    part3 (byte-vector-4 0 0 0 64)
    part4 (byte-vector-4 15 131 174 0)
    part5 (byte-vector-4 0 0 76 1)
    part6 (byte-vector-4 241 235 5 72)
    part7 (byte-vector-4 15 186 241 63)
    part8 (byte-vector-4 72 133 192 120)
    part9 (byte-vector-4 17 72 61 0)
    part10 (byte-vector-4 0 0 64 15)
    part11 (byte-vector-4 131 147 0 0)
    part12 (byte-vector-4 0 76 1 240)
    part13 (byte-vector-4 235 5 72 15)
    part14 (byte-vector-4 186 240 63 83)
    part15 (byte-vector-4 65 84 65 85)
    part16 (byte-vector-4 65 86 65 87)
    part17 (byte-vector-4 73 137 207 73)
    part18 (byte-vector-4 137 196 69 139)
    part19 (byte-vector-4 111 4 69 139)
    part20 (byte-vector-4 116 36 4 67)
    part21 (byte-vector-4 141 92 53 0)
    part22 (byte-vector-4 137 222 72 131)
    part23 (byte-vector-4 198 8 72 131)
    part24 (byte-vector-4 198 15 72 131)
    part25 (byte-vector-4 230 240 49 255)
    part26 (byte-vector-4 186 3 0 0)
    part27 (byte-vector-4 0 65 186 34)
    part28 (byte-vector-4 0 0 0 73)
    part29 (byte-vector-4 199 192 255 255)
    part30 (byte-vector-4 255 255 69 49)
    part31 (byte-vector-4 201 184 9 0)
    part32 (byte-vector-4 0 0 15 5)
    part33 (byte-vector-4 72 133 192 120)
    part34 (byte-vector-4 47 199 0 1)
    part35 (byte-vector-4 0 0 0 137)
    part36 (byte-vector-4 88 4 72 141)
    part37 (byte-vector-4 120 8 73 141)
    part38 (byte-vector-4 119 8 68 137)
    part39 (byte-vector-4 233 243 164 73)
    part40 (byte-vector-4 141 116 36 8)
    part41 (byte-vector-4 68 137 241 243)
    part42 (byte-vector-4 164 72 15 186)
    part43 (byte-vector-4 232 63 65 95)
    part44 (byte-vector-4 65 94 65 93)
    part45 (byte-vector-4 65 92 91 195)
    part46 (byte-vector-4 49 192 65 95)
    part47 (byte-vector-4 65 94 65 93)
    part48 (byte-vector-4 65 92 91 195)
    part49 (byte-vector-3 49 192 195)
    group1 (concat-five-byte-vectors-rooted part1 part2 part3 part4 part5)
    group2 (concat-five-byte-vectors-rooted part6 part7 part8 part9 part10)
    group3 (concat-five-byte-vectors-rooted part11 part12 part13 part14 part15)
    group4 (concat-five-byte-vectors-rooted part16 part17 part18 part19 part20)
    group5 (concat-five-byte-vectors-rooted part21 part22 part23 part24 part25)
    group6 (concat-five-byte-vectors-rooted part26 part27 part28 part29 part30)
    group7 (concat-five-byte-vectors-rooted part31 part32 part33 part34 part35)
    group8 (concat-five-byte-vectors-rooted part36 part37 part38 part39 part40)
    group9 (concat-five-byte-vectors-rooted part41 part42 part43 part44 part45)
    group10 (concat-four-byte-vectors-rooted part46 part47 part48 part49)
    group11 (concat-five-byte-vectors-rooted group1 group2 group3 group4 group5)
    group12 (concat-five-byte-vectors-rooted group6 group7 group8 group9 group10)]
    (concat-byte-vectors-rooted group11 group12)))

(defn emit-x86-selfhost-map-new-helper []
  (let [
    part1 (byte-vector-4 81 49 255 190)
    part2 (byte-vector-4 16 255 0 0)
    part3 (byte-vector-4 186 3 0 0)
    part4 (byte-vector-4 0 65 186 34)
    part5 (byte-vector-4 0 0 0 73)
    part6 (byte-vector-4 199 192 255 255)
    part7 (byte-vector-4 255 255 69 49)
    part8 (byte-vector-4 201 184 9 0)
    part9 (byte-vector-4 0 0 15 5)
    part10 (byte-vector-4 72 133 192 120)
    part11 (byte-vector-4 27 199 0 4)
    part12 (byte-vector-4 0 0 0 199)
    part13 (byte-vector-4 64 4 240 15)
    part14 (byte-vector-4 0 0 199 64)
    part15 (byte-vector-4 8 0 0 0)
    part16 (byte-vector-4 0 72 15 186)
    part17 (byte-vector-4 232 63 89 195)
    part18 (byte-vector-4 49 192 89 195)
    group1 (concat-five-byte-vectors-rooted part1 part2 part3 part4 part5)
    group2 (concat-five-byte-vectors-rooted part6 part7 part8 part9 part10)
    group3 (concat-five-byte-vectors-rooted part11 part12 part13 part14 part15)
    group4 (concat-three-byte-vectors-rooted part16 part17 part18)]
    (concat-four-byte-vectors-rooted group1 group2 group3 group4)))

(defn emit-x86-selfhost-map-size-helper []
  (let [part1 (byte-vector-4 72 133 192 121)
    part2 (byte-vector-4 9 72 15 186)
    part3 (byte-vector-4 240 63 139 64)
    part4 (byte-vector-4 8 195 49 192)
    part5 (byte-vector-1 195)]
    (concat-five-byte-vectors-rooted part1 part2 part3 part4 part5)))

(defn emit-x86-selfhost-map-insert-helper []
  (let [
    part1 (byte-vector-4 83 72 137 211)
    part2 (byte-vector-4 72 133 219 121)
    part3 (byte-vector-4 48 72 15 186)
    part4 (byte-vector-4 243 63 68 139)
    part5 (byte-vector-4 83 8 76 141)
    part6 (byte-vector-4 91 16 73 193)
    part7 (byte-vector-4 226 4 77 1)
    part8 (byte-vector-4 211 73 137 11)
    part9 (byte-vector-4 73 137 67 8)
    part10 (byte-vector-4 73 193 234 4)
    part11 (byte-vector-4 65 255 194 68)
    part12 (byte-vector-4 137 83 8 72)
    part13 (byte-vector-4 137 216 72 15)
    part14 (byte-vector-4 186 232 63 91)
    part15 (byte-vector-4 195 49 192 91)
    part16 (byte-vector-1 195)
    group1 (concat-five-byte-vectors-rooted part1 part2 part3 part4 part5)
    group2 (concat-five-byte-vectors-rooted part6 part7 part8 part9 part10)
    group3 (concat-five-byte-vectors-rooted part11 part12 part13 part14 part15)]
    (concat-four-byte-vectors-rooted group1 group2 group3 part16)))

(defn emit-x86-selfhost-map-get-helper []
  (let [
    part1 (byte-vector-4 83 65 84 72)
    part2 (byte-vector-4 133 201 121 48)
    part3 (byte-vector-4 72 137 203 72)
    part4 (byte-vector-4 15 186 243 63)
    part5 (byte-vector-4 68 139 99 8)
    part6 (byte-vector-4 49 201 68 57)
    part7 (byte-vector-4 225 115 29 72)
    part8 (byte-vector-4 137 202 72 193)
    part9 (byte-vector-4 226 4 72 141)
    part10 (byte-vector-4 84 19 16 72)
    part11 (byte-vector-4 57 2 116 4)
    part12 (byte-vector-4 255 193 235 230)
    part13 (byte-vector-4 72 139 66 8)
    part14 (byte-vector-4 65 92 91 195)
    part15 (byte-vector-4 49 192 65 92)
    part16 (byte-vector-2 91 195)
    group1 (concat-five-byte-vectors-rooted part1 part2 part3 part4 part5)
    group2 (concat-five-byte-vectors-rooted part6 part7 part8 part9 part10)
    group3 (concat-five-byte-vectors-rooted part11 part12 part13 part14 part15)]
    (concat-four-byte-vectors-rooted group1 group2 group3 part16)))

(defn emit-x86-selfhost-file-exists-helper []
  (let [
    part1 (byte-vector-4 83 65 84 72)
    part2 (byte-vector-4 137 227 72 133)
    part3 (byte-vector-4 192 121 45 72)
    part4 (byte-vector-4 15 186 240 63)
    part5 (byte-vector-4 68 139 96 4)
    part6 (byte-vector-4 72 141 112 8)
    part7 (byte-vector-4 76 137 225 72)
    part8 (byte-vector-4 137 202 72 131)
    part9 (byte-vector-4 194 16 72 131)
    part10 (byte-vector-4 226 240 72 41)
    part11 (byte-vector-4 212 72 137 231)
    part12 (byte-vector-4 243 164 66 198)
    part13 (byte-vector-4 4 36 0 72)
    part14 (byte-vector-4 137 231 235 3)
    part15 (byte-vector-4 72 137 199 49)
    part16 (byte-vector-4 246 184 21 0)
    part17 (byte-vector-4 0 0 15 5)
    part18 (byte-vector-4 131 248 0 15)
    part19 (byte-vector-4 148 192 15 182)
    part20 (byte-vector-4 192 72 137 220)
    part21 (byte-vector-4 65 92 91 195)
    group1 (concat-five-byte-vectors-rooted part1 part2 part3 part4 part5)
    group2 (concat-five-byte-vectors-rooted part6 part7 part8 part9 part10)
    group3 (concat-five-byte-vectors-rooted part11 part12 part13 part14 part15)
    group4 (concat-five-byte-vectors-rooted part16 part17 part18 part19 part20)]
    (concat-five-byte-vectors-rooted group1 group2 group3 group4 part21)))

;; x86_64 ReadFile helper: raw C string / tagged L# string path を受け取り、
;; syscall で最大 2MiB を mmap した tagged string として返す。
(defn emit-x86-selfhost-read-file-helper []
  (let [
    part1 (byte-vector-4 83 65 84 65)
    part2 (byte-vector-4 85 69 49 237)
    part3 (byte-vector-4 72 137 199 72)
    part4 (byte-vector-4 133 255 121 40)
    part5 (byte-vector-4 72 15 186 240)
    part6 (byte-vector-4 63 73 137 229)
    part7 (byte-vector-4 139 72 4 72)
    part8 (byte-vector-4 141 112 8 72)
    part9 (byte-vector-4 137 202 72 131)
    part10 (byte-vector-4 194 16 72 131)
    part11 (byte-vector-4 226 240 72 41)
    part12 (byte-vector-4 212 72 137 231)
    part13 (byte-vector-4 243 164 198 7)
    part14 (byte-vector-4 0 72 137 231)
    part15 (byte-vector-4 184 2 0 0)
    part16 (byte-vector-4 0 49 246 49)
    part17 (byte-vector-4 210 15 5 77)
    part18 (byte-vector-4 133 237 116 3)
    part19 (byte-vector-4 76 137 236 114)
    part20 (byte-vector-4 122 72 133 192)
    part21 (byte-vector-4 120 117 137 195)
    part22 (byte-vector-4 49 255 190 16)
    part23 (byte-vector-4 0 32 0 186)
    part24 (byte-vector-4 3 0 0 0)
    part25 (byte-vector-4 65 186 34 0)
    part26 (byte-vector-4 0 0 65 184)
    part27 (byte-vector-4 255 255 255 255)
    part28 (byte-vector-4 69 49 201 184)
    part29 (byte-vector-4 9 0 0 0)
    part30 (byte-vector-4 15 5 114 70)
    part31 (byte-vector-4 72 133 192 120)
    part32 (byte-vector-4 65 73 137 196)
    part33 (byte-vector-4 137 223 73 141)
    part34 (byte-vector-4 116 36 8 186)
    part35 (byte-vector-4 0 0 32 0)
    part36 (byte-vector-4 184 0 0 0)
    part37 (byte-vector-4 0 15 5 114)
    part38 (byte-vector-4 41 72 133 192)
    part39 (byte-vector-4 120 36 65 199)
    part40 (byte-vector-4 4 36 5 0)
    part41 (byte-vector-4 0 0 65 137)
    part42 (byte-vector-4 68 36 4 137)
    part43 (byte-vector-4 223 184 3 0)
    part44 (byte-vector-4 0 0 15 5)
    part45 (byte-vector-4 76 137 224 72)
    part46 (byte-vector-4 15 186 232 63)
    part47 (byte-vector-4 65 93 65 92)
    part48 (byte-vector-4 91 195 137 223)
    part49 (byte-vector-4 184 3 0 0)
    part50 (byte-vector-4 0 15 5 49)
    part51 (byte-vector-4 192 65 93 65)
    part52 (byte-vector-3 92 91 195)
    group1 (concat-five-byte-vectors-rooted part1 part2 part3 part4 part5)
    group2 (concat-five-byte-vectors-rooted part6 part7 part8 part9 part10)
    group3 (concat-five-byte-vectors-rooted part11 part12 part13 part14 part15)
    group4 (concat-five-byte-vectors-rooted part16 part17 part18 part19 part20)
    group5 (concat-five-byte-vectors-rooted part21 part22 part23 part24 part25)
    group6 (concat-five-byte-vectors-rooted part26 part27 part28 part29 part30)
    group7 (concat-five-byte-vectors-rooted part31 part32 part33 part34 part35)
    group8 (concat-five-byte-vectors-rooted part36 part37 part38 part39 part40)
    group9 (concat-five-byte-vectors-rooted part41 part42 part43 part44 part45)
    group10 (concat-five-byte-vectors-rooted part46 part47 part48 part49 part50)
    group11 (concat-three-byte-vectors-rooted part51 part52 (vector-new 0))
    head (concat-five-byte-vectors-rooted group1 group2 group3 group4 group5)
    tail (concat-five-byte-vectors-rooted group6 group7 group8 group9 group10)]
    (concat-three-byte-vectors-rooted head tail group11)))

(defn append-x86-selfhost-read-file-helper-rooted [result]
  (do
    (root_push result)
    (let [native (emit-x86-selfhost-read-file-helper)]
      (do
        (root_push native)
        (append-native-bytes-rooted result native 207)
        (root_pop)
        (root_pop)
        0))))

(defn emit-x86-helper-call-preserving-rcx [rel]
  (concat-three-byte-vectors-rooted
    (emit-push-rcx)
    (emit-call-rel32 rel)
    (emit-pop-rcx)))

(defn emit-push-rax []
  (byte-vector-1 80))

(defn emit-map-new-bundle-x86 [rel frame-base-slot-count current-depth]
  (let [call-bytes (emit-call-rel32 rel)]
    (if (>= current-depth 2)
      (concat-byte-vectors
        (concat-byte-vectors
          (concat-byte-vectors
            (shift-native-value-window-x86-loop frame-base-slot-count (- current-depth 3))
            (emit-mov-local-from-rcx (native-value-window-spill-offset frame-base-slot-count 0)))
          (emit-push-rax))
        (concat-byte-vectors call-bytes (emit-pop-rcx)))
      (if (= current-depth 1)
        (concat-byte-vectors
          (concat-byte-vectors (emit-push-rax) call-bytes)
          (emit-pop-rcx))
        call-bytes))))

(defn emit-consume-two-bundle-x86 [op-bytes frame-base-slot-count current-depth]
  (if (>= current-depth 3)
    (concat-byte-vectors
      (concat-byte-vectors
        op-bytes
        (emit-mov-rcx-from-local (native-value-window-spill-offset frame-base-slot-count 0)))
      (emit-drop-window-spill-shifts-x86 frame-base-slot-count 1 (- current-depth 3)))
    op-bytes))

(defn emit-consume-three-produce-one-bundle-x86 [op-bytes frame-base-slot-count current-depth]
  (if (>= current-depth 5)
    (concat-byte-vectors
      (concat-byte-vectors
        op-bytes
        (emit-mov-rcx-from-local (native-value-window-spill-offset frame-base-slot-count 1)))
      (emit-store-window-spill-shifts-x86 frame-base-slot-count 2 (- current-depth 3)))
    (if (= current-depth 4)
      (concat-byte-vectors
        op-bytes
        (emit-mov-rcx-from-local (native-value-window-spill-offset frame-base-slot-count 1)))
      op-bytes)))

(defn emit-consume-four-produce-one-bundle-x86 [op-bytes frame-base-slot-count current-depth]
  (if (>= current-depth 6)
    (concat-byte-vectors
      (concat-byte-vectors
        op-bytes
        (emit-mov-rcx-from-local (native-value-window-spill-offset frame-base-slot-count 2)))
      (emit-store-window-spill-shifts-x86-consumed frame-base-slot-count 3 (- current-depth 3) 3))
    (if (= current-depth 5)
      (concat-byte-vectors
        op-bytes
        (emit-mov-rcx-from-local (native-value-window-spill-offset frame-base-slot-count 2)))
      op-bytes)))

(defn emit-consume-five-produce-one-bundle-x86 [op-bytes frame-base-slot-count current-depth]
  (if (>= current-depth 7)
    (concat-byte-vectors
      (concat-byte-vectors
        op-bytes
        (emit-mov-rcx-from-local (native-value-window-spill-offset frame-base-slot-count 3)))
      (emit-store-window-spill-shifts-x86-consumed frame-base-slot-count 4 (- current-depth 3) 4))
    (if (= current-depth 6)
      (concat-byte-vectors
        op-bytes
        (emit-mov-rcx-from-local (native-value-window-spill-offset frame-base-slot-count 3)))
      op-bytes)))

(defn emit-consume-six-produce-one-bundle-x86 [op-bytes frame-base-slot-count current-depth]
  (if (>= current-depth 8)
    (concat-byte-vectors
      (concat-byte-vectors
        op-bytes
        (emit-mov-rcx-from-local (native-value-window-spill-offset frame-base-slot-count 4)))
      (emit-store-window-spill-shifts-x86-consumed frame-base-slot-count 5 (- current-depth 3) 5))
    (if (= current-depth 7)
      (concat-byte-vectors
        op-bytes
        (emit-mov-rcx-from-local (native-value-window-spill-offset frame-base-slot-count 4)))
      op-bytes)))

(defn emit-consume-seven-produce-one-bundle-x86 [op-bytes frame-base-slot-count current-depth]
  (if (>= current-depth 9)
    (concat-byte-vectors
      (concat-byte-vectors
        op-bytes
        (emit-mov-rcx-from-local (native-value-window-spill-offset frame-base-slot-count 5)))
      (emit-store-window-spill-shifts-x86-consumed frame-base-slot-count 6 (- current-depth 3) 6))
    (if (= current-depth 8)
      (concat-byte-vectors
        op-bytes
        (emit-mov-rcx-from-local (native-value-window-spill-offset frame-base-slot-count 5)))
      op-bytes)))

(defn emit-consume-eight-produce-one-bundle-x86 [op-bytes frame-base-slot-count current-depth]
  (if (>= current-depth 10)
    (concat-byte-vectors
      (concat-byte-vectors
        op-bytes
        (emit-mov-rcx-from-local (native-value-window-spill-offset frame-base-slot-count 6)))
      (emit-store-window-spill-shifts-x86-consumed frame-base-slot-count 7 (- current-depth 3) 7))
    (if (= current-depth 9)
      (concat-byte-vectors
        op-bytes
        (emit-mov-rcx-from-local (native-value-window-spill-offset frame-base-slot-count 6)))
      op-bytes)))

(defn emit-consume-nine-produce-one-bundle-x86 [op-bytes frame-base-slot-count current-depth]
  (if (>= current-depth 11)
    (concat-byte-vectors
      (concat-byte-vectors
        op-bytes
        (emit-mov-rcx-from-local (native-value-window-spill-offset frame-base-slot-count 7)))
      (emit-store-window-spill-shifts-x86-consumed frame-base-slot-count 8 (- current-depth 3) 8))
    (if (= current-depth 10)
      (concat-byte-vectors
        op-bytes
        (emit-mov-rcx-from-local (native-value-window-spill-offset frame-base-slot-count 7)))
      op-bytes)))

(defn emit-map-insert-bundle-x86 [rel frame-base-slot-count current-depth]
  (emit-consume-three-produce-one-bundle-x86
    (concat-byte-vectors
      (emit-mov-rdx-from-local (native-value-window-spill-offset frame-base-slot-count 0))
      (emit-call-rel32 rel))
    frame-base-slot-count
    current-depth))

(defn emit-substring-bundle-x86 [rel frame-base-slot-count current-depth]
  (emit-consume-three-produce-one-bundle-x86
    (concat-byte-vectors
      (emit-mov-rdx-from-local (native-value-window-spill-offset frame-base-slot-count 0))
      (emit-call-rel32 rel))
    frame-base-slot-count
    current-depth))

(defn x86-import-stub-size [import-count]
  (if (> import-count 0) 1 0))

(defn x86-import-ret-stub-offset [import-stub-offset import-count import-idx]
  import-stub-offset)

(defn x86-bundle-initial-capacity [import-stub-offset import-count]
  (+ import-stub-offset (+ (x86-import-stub-size import-count) 2048)))

(defn x86-selfhost-command-line-arg-helper-size []
  18)

(defn x86-selfhost-read-file-helper-size []
  207)

(defn x86-selfhost-string-length-helper-size []
  52)

(defn x86-selfhost-string-char-at-helper-size []
  71)

(defn x86-selfhost-print-helper-size []
  102)

(defn x86-selfhost-vector-new-helper-size []
  119)

(defn x86-selfhost-vector-length-helper-size []
  17)

(defn x86-selfhost-vector-get-helper-size []
  47)

(defn x86-selfhost-vector-push-helper-size []
  205)

(defn x86-selfhost-ref-new-helper-size []
  73)

(defn x86-selfhost-ref-get-helper-size []
  18)

(defn x86-selfhost-ref-set-helper-size []
  20)

(defn x86-selfhost-substring-helper-size []
  145)

(defn x86-selfhost-string-concat-helper-size []
  195)

(defn x86-selfhost-map-new-helper-size []
  72)

(defn x86-selfhost-map-size-helper-size []
  17)

(defn x86-selfhost-map-insert-helper-size []
  61)

(defn x86-selfhost-map-get-helper-size []
  62)

(defn x86-selfhost-file-exists-helper-size []
  84)

(defn x86-selfhost-helper-trailer-size [import-count]
  (+ (x86-import-stub-size import-count)
     (+ (x86-selfhost-command-line-arg-helper-size)
        (+ (x86-selfhost-read-file-helper-size)
           (+ (x86-selfhost-string-length-helper-size)
              (+ (x86-selfhost-string-char-at-helper-size)
                 (+ (x86-selfhost-print-helper-size)
                    (+ (x86-selfhost-vector-new-helper-size)
                        (+ (x86-selfhost-vector-length-helper-size)
                           (+ (x86-selfhost-vector-get-helper-size)
                              (+ (x86-selfhost-vector-push-helper-size)
                                 (+ (x86-selfhost-ref-new-helper-size)
                                    (+ (x86-selfhost-ref-get-helper-size)
                                       (+ (x86-selfhost-ref-set-helper-size)
                                          (+ (x86-selfhost-substring-helper-size)
                                             (+ (x86-selfhost-string-concat-helper-size)
                                                (+ (x86-selfhost-map-new-helper-size)
                                                   (+ (x86-selfhost-map-size-helper-size)
                                                      (+ (x86-selfhost-map-insert-helper-size)
                                                         (+ (x86-selfhost-map-get-helper-size)
                                                             (x86-selfhost-file-exists-helper-size)))))))))))))))))))))

(defn x86-helper-base-offset [import-stub-offset import-count]
  (+ import-stub-offset (x86-import-stub-size import-count)))

(defn x86-selfhost-command-line-arg-helper-offset [import-stub-offset import-count]
  (+ (x86-helper-base-offset import-stub-offset import-count) 0))

(defn x86-selfhost-read-file-helper-offset [import-stub-offset import-count]
  (+ (x86-helper-base-offset import-stub-offset import-count) 18))

(defn x86-selfhost-string-length-helper-offset [import-stub-offset import-count]
  (+ (x86-helper-base-offset import-stub-offset import-count) 225))

(defn x86-selfhost-string-char-at-helper-offset [import-stub-offset import-count]
  (+ (x86-helper-base-offset import-stub-offset import-count) 277))

(defn x86-selfhost-print-helper-offset [import-stub-offset import-count]
  (+ (x86-helper-base-offset import-stub-offset import-count) 348))

(defn x86-selfhost-vector-new-helper-offset [import-stub-offset import-count]
  (+ (x86-helper-base-offset import-stub-offset import-count) 450))

(defn x86-selfhost-vector-length-helper-offset [import-stub-offset import-count]
  (+ (x86-helper-base-offset import-stub-offset import-count) 569))

(defn x86-selfhost-vector-get-helper-offset [import-stub-offset import-count]
  (+ (x86-helper-base-offset import-stub-offset import-count) 586))

(defn x86-selfhost-vector-push-helper-offset [import-stub-offset import-count]
  (+ (x86-helper-base-offset import-stub-offset import-count) 633))

(defn x86-selfhost-ref-new-helper-offset [import-stub-offset import-count]
  (+ (x86-helper-base-offset import-stub-offset import-count) 838))

(defn x86-selfhost-ref-get-helper-offset [import-stub-offset import-count]
  (+ (x86-helper-base-offset import-stub-offset import-count) 911))

(defn x86-selfhost-ref-set-helper-offset [import-stub-offset import-count]
  (+ (x86-helper-base-offset import-stub-offset import-count) 929))

(defn x86-selfhost-substring-helper-offset [import-stub-offset import-count]
  (+ (x86-helper-base-offset import-stub-offset import-count) 949))

(defn x86-selfhost-string-concat-helper-offset [import-stub-offset import-count]
  (+ (x86-helper-base-offset import-stub-offset import-count) 1094))

(defn x86-selfhost-map-new-helper-offset [import-stub-offset import-count]
  (+ (x86-helper-base-offset import-stub-offset import-count) 1289))

(defn x86-selfhost-map-size-helper-offset [import-stub-offset import-count]
  (+ (x86-helper-base-offset import-stub-offset import-count) 1361))

(defn x86-selfhost-map-insert-helper-offset [import-stub-offset import-count]
  (+ (x86-helper-base-offset import-stub-offset import-count) 1378))

(defn x86-selfhost-map-get-helper-offset [import-stub-offset import-count]
  (+ (x86-helper-base-offset import-stub-offset import-count) 1439))

(defn x86-selfhost-file-exists-helper-offset [import-stub-offset import-count]
  (+ (x86-helper-base-offset import-stub-offset import-count) 1501))

(defn is-selfhost-runtime-opcode-x86 [opcode]
  (if (= opcode 64)
    1
    (if (= opcode 67)
      1
      (if (= opcode 51)
        1
        (if (= opcode 50)
          1
          (if (= opcode 59)
            1
            (if (= opcode 54)
              1
              (if (= opcode 52)
                1
                (if (= opcode 53)
                  1
                  (if (= opcode 55)
                    1
                    (if (= opcode 56)
                      1
                      (if (= opcode 57)
                        1
                        (if (= opcode 58)
                          1
                          (if (= opcode 69)
                            1
                            (if (= opcode 70)
                              1
                              (if (= opcode 60)
                                1
                                (if (= opcode 61)
                                  1
                                  (if (= opcode 62)
                                    1
                                    (if (= opcode 63)
                                      1
                                      (if (= opcode 73)
                                        1
                                        0))))))))))))))))))))

(defn codegen-selfhost-runtime-bundle-x86 [opcode current-offset import-stub-offset import-count frame-base-slot-count current-depth]
  (if (if (= opcode 64) true (if (= opcode 67) true (= opcode 59)))
    (emit-x86-helper-call-preserving-rcx
      (- (if (= opcode 64)
           (x86-selfhost-read-file-helper-offset import-stub-offset import-count)
           (if (= opcode 67)
             (x86-selfhost-command-line-arg-helper-offset import-stub-offset import-count)
             (x86-selfhost-print-helper-offset import-stub-offset import-count)))
         (+ current-offset 6)))
      (if (= opcode 51)
        (emit-x86-helper-call-preserving-rcx
          (- (x86-selfhost-string-length-helper-offset import-stub-offset import-count)
             (+ current-offset 6)))
        (if (if (= opcode 50) true (= opcode 63))
          (emit-consume-two-bundle-x86
            (emit-call-rel32
              (- (if (= opcode 50)
                   (x86-selfhost-string-char-at-helper-offset import-stub-offset import-count)
                   (x86-selfhost-map-get-helper-offset import-stub-offset import-count))
                 (+ current-offset 5)))
	                          frame-base-slot-count
	                          current-depth)
          (if (= opcode 54)
              (emit-x86-helper-call-preserving-rcx
                (- (x86-selfhost-vector-new-helper-offset import-stub-offset import-count)
                   (+ current-offset 6)))
              (if (= opcode 52)
                (emit-x86-helper-call-preserving-rcx
                  (- (x86-selfhost-vector-length-helper-offset import-stub-offset import-count)
                     (+ current-offset 6)))
                (if (= opcode 53)
                  (emit-consume-two-bundle-x86
                    (emit-call-rel32
                      (- (x86-selfhost-vector-get-helper-offset import-stub-offset import-count)
                         (+ current-offset 5)))
                    frame-base-slot-count
                    current-depth)
                  (if (= opcode 55)
                    (emit-consume-two-bundle-x86
                      (emit-call-rel32
                        (- (x86-selfhost-vector-push-helper-offset import-stub-offset import-count)
                           (+ current-offset 5)))
                      frame-base-slot-count
                      current-depth)
                    (if (= opcode 56)
                      (emit-x86-helper-call-preserving-rcx
                        (- (x86-selfhost-ref-new-helper-offset import-stub-offset import-count)
                           (+ current-offset 6)))
                      (if (= opcode 57)
                        (emit-x86-helper-call-preserving-rcx
                          (- (x86-selfhost-ref-get-helper-offset import-stub-offset import-count)
                             (+ current-offset 6)))
                        (if (= opcode 58)
                          (emit-consume-two-bundle-x86
                            (emit-call-rel32
                              (- (x86-selfhost-ref-set-helper-offset import-stub-offset import-count)
                                 (+ current-offset 5)))
                            frame-base-slot-count
                            current-depth)
                          (if (= opcode 69)
                            (emit-substring-bundle-x86
                              (- (x86-selfhost-substring-helper-offset import-stub-offset import-count)
                                 (+ current-offset 12))
                              frame-base-slot-count
                              current-depth)
                            (if (= opcode 70)
                              (emit-consume-two-bundle-x86
                                (emit-call-rel32
                                  (- (x86-selfhost-string-concat-helper-offset import-stub-offset import-count)
                                     (+ current-offset 5)))
                                frame-base-slot-count
                                current-depth)
                              (if (= opcode 60)
                                (if (= current-depth 0)
                                  (emit-call-rel32
                                    (- (x86-selfhost-map-new-helper-offset import-stub-offset import-count)
                                       (+ current-offset 5)))
                                  (if (= current-depth 1)
                                    (concat-byte-vectors
                                      (concat-byte-vectors
                                        (emit-push-rax)
                                        (emit-call-rel32
                                          (- (x86-selfhost-map-new-helper-offset import-stub-offset import-count)
                                             (+ current-offset 6))))
                                      (emit-pop-rcx))
                                    (concat-byte-vectors
                                      (concat-byte-vectors
                                        (concat-byte-vectors
                                          (shift-native-value-window-x86-loop frame-base-slot-count (- current-depth 3))
                                          (emit-mov-local-from-rcx (native-value-window-spill-offset frame-base-slot-count 0)))
                                        (emit-push-rax))
                                      (concat-byte-vectors
                                        (emit-call-rel32
                                          (- (x86-selfhost-map-new-helper-offset import-stub-offset import-count)
                                             (+ current-offset (+ 13 (* (- current-depth 2) 14)))))
                                        (emit-pop-rcx)))))
                                (if (= opcode 61)
                                  (emit-x86-helper-call-preserving-rcx
                                    (- (x86-selfhost-map-size-helper-offset import-stub-offset import-count)
                                       (+ current-offset 6)))
                                  (if (= opcode 62)
	                                    (emit-map-insert-bundle-x86
	                                      (- (x86-selfhost-map-insert-helper-offset import-stub-offset import-count)
	                                         (+ current-offset 12))
	                                      frame-base-slot-count
	                                      current-depth)
	                                    (if (= opcode 73)
	                                      (emit-x86-helper-call-preserving-rcx
	                                        (- (x86-selfhost-file-exists-helper-offset import-stub-offset import-count)
	                                           (+ current-offset 6)))
                                      (vector-new 0))))))))))))))))))

(defn codegen-x86-opcode-call-bundle [operand current-offset function-starts context]
  (let [function-metas (vector-get context 0)
    import-count (vector-get context 1)
    import-stub-offset (vector-get context 2)
    function-start-base (vector-get context 3)
	    frame-base-slot-count (vector-get context 4)
	    current-depth (vector-get context 5)]
    (let [operand-ref (ref-new operand)]
	    (do
	      (root_push operand-ref)
        (let [current-offset-ref (ref-new current-offset)]
	        (do
	          (root_push current-offset-ref)
            (let [function-starts-ref (ref-new function-starts)]
	            (do
	              (root_push function-starts-ref)
	              (let [target-meta (vector-get function-metas (ref-get operand-ref))
	        target-param-count (native-function-param-count target-meta)
	        call-next-offset (native-call-rel-next-offset-x86 target-param-count current-depth)
	        target-offset (if (< (ref-get operand-ref) import-count)
	                        (x86-import-ret-stub-offset import-stub-offset import-count (ref-get operand-ref))
	                        (- (vector-get (ref-get function-starts-ref) (- (ref-get operand-ref) import-count)) function-start-base))
	        call-rel (- target-offset (+ (ref-get current-offset-ref) call-next-offset))]
        (let [call-rel-bytes (emit-call-rel32 call-rel)]
          (do
            (root_push call-rel-bytes)
          (let [result (if (= target-param-count 0)
                     (if (= current-depth 0)
                       (concat-byte-vectors-rooted call-rel-bytes (vector-new 0))
                       (if (= current-depth 1)
                         (concat-byte-vectors-rooted
                           (concat-byte-vectors-rooted
                             (emit-push-rax)
                             call-rel-bytes)
                           (emit-pop-rcx))
                         (concat-byte-vectors
                           (concat-byte-vectors
                             (concat-byte-vectors
                               (shift-native-value-window-x86-loop frame-base-slot-count (- current-depth 3))
                               (emit-mov-local-from-rcx (native-value-window-spill-offset frame-base-slot-count 0)))
                             (emit-push-rax))
                           (concat-byte-vectors
                             call-rel-bytes
                             (emit-pop-rcx)))))
                     (if (= target-param-count 1)
                       (emit-call-bundle-x86-one-to-nine 1 call-rel frame-base-slot-count current-depth)
                       (if (= target-param-count 2)
                         (emit-two-arg-call-x86-with-call-bytes call-rel-bytes frame-base-slot-count current-depth)
                         (if (= target-param-count 3)
                           (emit-consume-three-produce-one-bundle-x86
                             (emit-three-arg-call-x86-core-with-call-bytes call-rel-bytes frame-base-slot-count)
                             frame-base-slot-count
                             current-depth)
                           (if (= target-param-count 4)
                             (emit-consume-four-produce-one-bundle-x86
                               (let [call-rel-ref (ref-new call-rel)]
                                 (do
                                  (root_push call-rel-ref)
                                  (let [result (emit-four-arg-call-x86-core-with-rel-ref call-rel-ref frame-base-slot-count)]
                                   (do
                                     (root_pop)
                                     result))))
                               frame-base-slot-count
                               current-depth)
                             (if (= target-param-count 5)
                               (emit-consume-five-produce-one-bundle-x86
                                 (emit-five-arg-call-x86-core-with-call-bytes call-rel-bytes frame-base-slot-count)
                                 frame-base-slot-count
                                 current-depth)
                               (if (= target-param-count 6)
                                 (emit-consume-six-produce-one-bundle-x86
                                   (emit-six-arg-call-x86-core-with-call-bytes call-rel-bytes frame-base-slot-count)
                                   frame-base-slot-count
                                   current-depth)
                                 (if (= target-param-count 7)
                                   (emit-consume-seven-produce-one-bundle-x86
                                     (emit-seven-arg-call-x86-core-with-call-bytes call-rel-bytes frame-base-slot-count)
                                     frame-base-slot-count
                                     current-depth)
                                   (if (= target-param-count 8)
                                     (emit-consume-eight-produce-one-bundle-x86
                                       (emit-eight-arg-call-x86-core-with-call-bytes call-rel-bytes frame-base-slot-count)
                                       frame-base-slot-count
                                       current-depth)
                                     (if (= target-param-count 9)
                                       (emit-consume-nine-produce-one-bundle-x86
                                         (emit-nine-arg-call-x86-core-with-call-bytes call-rel-bytes frame-base-slot-count)
                                         frame-base-slot-count
                                         current-depth)
                                         (if (>= target-param-count 20)
                                           (emit-call-bundle-x86-twenty-to-sixty target-param-count call-rel frame-base-slot-count)
                                           (if (>= target-param-count 10)
                                             (emit-call-bundle-x86-ten-to-nineteen target-param-count call-rel frame-base-slot-count)
                                             (vector-new 0)))))))))))))]
	            (do
                  (ref-set operand-ref result)
			              (root_pop)
			              (root_pop)
			              (root_pop)
                  (let [final-result (ref-get operand-ref)]
                    (do
			                  (root_pop)
				                  final-result)))))))))))))))

(defn codegen-ir-instr-bundle-x86-with-import-count-and-base [opcode operand current-offset function-starts function-metas import-count import-stub-offset function-start-base frame-base-slot-count current-depth]
  (if (= opcode 40)
    (do
      (root_push function-starts)
      (let [call-context (vector-push
                           (vector-push
                             (vector-push
                               (vector-push
                                 (vector-push
                                   (vector-push (vector-new 6) function-metas)
                                   import-count)
                                 import-stub-offset)
                               function-start-base)
                           frame-base-slot-count)
                         current-depth)]
        (do
          (root_push call-context)
          (let [result (codegen-x86-opcode-call-bundle operand current-offset function-starts call-context)]
            (do
              (root_pop)
              (root_pop)
              result)))))
    (if (= opcode 1)
      (emit-i64-const-bundle-x86 operand frame-base-slot-count current-depth)
      (if (= opcode 3)
        (emit-i32-const-bundle-x86 operand frame-base-slot-count current-depth)
        (if (= opcode 74)
          (emit-root-push-x86)
        (if (= opcode 75)
          (emit-i32-const-bundle-x86 0 frame-base-slot-count current-depth)
          (if (= (is-selfhost-runtime-opcode-x86 opcode) 1)
            (codegen-selfhost-runtime-bundle-x86 opcode current-offset import-stub-offset import-count frame-base-slot-count current-depth)
                (if (= opcode 10)
                  (emit-local-get-bundle-x86 (local-slot-offset operand) frame-base-slot-count current-depth)
                  (if (= opcode 11)
                    (emit-local-set-bundle-x86 (local-slot-offset operand) frame-base-slot-count current-depth)
                  (if (= opcode 44)
                    (emit-drop-bundle-x86 frame-base-slot-count current-depth)
                    (if (= opcode 76)
                      (emit-root-set-bundle-x86 frame-base-slot-count current-depth)
                      (if (= opcode 46)
                        (emit-i32-store-bundle-x86 operand frame-base-slot-count current-depth)
                        (if (= opcode 49)
                          (emit-i64-store-bundle-x86 operand frame-base-slot-count current-depth)
                          (if (= opcode 77)
                            (emit-memory-copy-bundle-x86 frame-base-slot-count current-depth)
                                               (if (= opcode 78)
                                                   (emit-memory-fill-bundle-x86 frame-base-slot-count current-depth)
                                                   (if (= (x86-plain-two-to-one-needs-window-restore opcode) 1)
                                                     (emit-consume-two-bundle-x86
                                                       (codegen-ir-instr opcode operand)
                                                       frame-base-slot-count
                                                       current-depth)
                                   (codegen-ir-instr opcode operand)))))))))))))))))

(defn codegen-ir-instr-bundle-x86-with-import-count [opcode operand current-offset function-starts function-metas import-count import-stub-offset frame-base-slot-count current-depth]
  (codegen-ir-instr-bundle-x86-with-import-count-and-base opcode operand current-offset function-starts function-metas import-count import-stub-offset 0 frame-base-slot-count current-depth))

(defn codegen-ir-instr-bundle-x86 [opcode operand current-offset function-starts function-metas frame-base-slot-count current-depth]
  (codegen-ir-instr-bundle-x86-with-import-count opcode operand current-offset function-starts function-metas 0 0 frame-base-slot-count current-depth))

(defn generate-native-instr-bundle-loop-x86-with-import-count [ir-func result function-starts function-metas import-count import-stub-offset frame-base-slot-count current-offset current-depth idx len]
  (if (>= idx len)
    current-offset
    (let [instr (vector-get ir-func idx)
      opcode (vector-get instr 0)
      operand (vector-get instr 1)
      native (codegen-ir-instr-bundle-x86-with-import-count opcode operand current-offset function-starts function-metas import-count import-stub-offset frame-base-slot-count current-depth)
      native-len (vector-length native)
      next-depth (apply-stack-delta current-depth (opcode-stack-delta opcode operand function-metas))]
      (do
        (append-native-bytes-loop result native 0 native-len)
        (generate-native-instr-bundle-loop-x86-with-import-count ir-func result function-starts function-metas import-count import-stub-offset frame-base-slot-count (+ current-offset native-len) next-depth (+ idx 1) len)))))

(defn generate-native-instr-bundle-loop-x86 [ir-func result function-starts function-metas frame-base-slot-count current-offset current-depth idx len]
  (generate-native-instr-bundle-loop-x86-with-import-count ir-func result function-starts function-metas 0 0 frame-base-slot-count current-offset current-depth idx len))

(defn x86-current-emitted-offset [result emit-start-base]
  (let [actual-len (vector-length (ref-get result))]
    (if (< actual-len emit-start-base)
      actual-len
      (- actual-len emit-start-base))))

(defn make-x86-control-bundle-context [ir-func result meta offsets depths function-starts function-metas layout frame-base-slot-count]
  (vector-push
    (vector-push
      (vector-push
        (vector-push
          (vector-push
            (vector-push
              (vector-push
                (vector-push
                  (vector-push (vector-new 9) ir-func)
                  result)
                meta)
              offsets)
            depths)
          function-starts)
        function-metas)
      layout)
    frame-base-slot-count))

(defn make-x86-control-loop-state [idx remaining]
  (vector-push
    (vector-push (vector-new 2) idx)
    remaining))

(defn continue-native-control-instr-bundle-loop-x86 [ctx idx remaining]
  (if (<= remaining 1)
    0
    (do
      (root_push ctx)
      (let [next-state (make-x86-control-loop-state (+ idx 1) (- remaining 1))]
        (do
          (root_push next-state)
          (let [final (generate-native-control-instr-bundle-loop-x86-with-context ctx next-state)]
            (do
              (root_pop)
              (root_pop)
              final)))))))

(defn codegen-x86-control-loop-fallback-native [ctx idx opcode operand actual-current-offset]
  (let [ir-func (vector-get ctx 0)
    meta (vector-get ctx 2)
    offsets (vector-get ctx 3)
    depths (vector-get ctx 4)
    layout (vector-get ctx 7)
    function-metas (vector-get ctx 6)
    import-count (vector-get layout 0)
    import-stub-offset (vector-get layout 1)
    frame-base-slot-count (vector-get ctx 8)
    current-depth (vector-get depths idx)]
    (if (= (is-control-opcode opcode) 1)
      (emit-control-instr-x86 ir-func meta offsets idx)
      (if (= (is-selfhost-runtime-opcode-x86 opcode) 1)
        (do
          (let [actual-current-offset-ref (ref-new actual-current-offset)]
            (do
              (root_push actual-current-offset-ref)
              (let [import-stub-offset-ref (ref-new import-stub-offset)]
                (do
	                  (root_push import-stub-offset-ref)
	                  (let [runtime-result (codegen-selfhost-runtime-bundle-x86 opcode (ref-get actual-current-offset-ref) (ref-get import-stub-offset-ref) import-count frame-base-slot-count current-depth)]
	                    (do
	                      (root_pop)
	                      (root_pop)
	                      runtime-result)))))))
        (if (= opcode 1)
          (emit-i64-const-bundle-x86 operand frame-base-slot-count current-depth)
          (if (= opcode 3)
            (emit-i32-const-bundle-x86 operand frame-base-slot-count current-depth)
            (if (= opcode 74)
              (emit-root-push-x86)
            (if (= opcode 75)
              (emit-i32-const-bundle-x86 0 frame-base-slot-count current-depth)
              (if (= opcode 10)
                (emit-local-get-bundle-x86 (local-slot-offset operand) frame-base-slot-count current-depth)
                (if (= opcode 11)
                  (emit-local-set-bundle-x86 (local-slot-offset operand) frame-base-slot-count current-depth)
                  (if (= opcode 44)
                    (emit-drop-bundle-x86 frame-base-slot-count current-depth)
                    (if (= opcode 76)
                      (emit-root-set-bundle-x86 frame-base-slot-count current-depth)
                      (if (= opcode 46)
                        (emit-i32-store-bundle-x86 operand frame-base-slot-count current-depth)
                        (if (= opcode 49)
                          (emit-i64-store-bundle-x86 operand frame-base-slot-count current-depth)
                          (if (= opcode 77)
                            (emit-memory-copy-bundle-x86 frame-base-slot-count current-depth)
                            (if (= opcode 78)
                              (emit-memory-fill-bundle-x86 frame-base-slot-count current-depth)
                              (codegen-ir-instr opcode operand)))))))))))))))))

(defn generate-native-control-instr-bundle-loop-x86-with-context [ctx state]
  (let [idx (vector-get state 0)
    remaining (vector-get state 1)]
  (if (<= remaining 0)
    0
    (let [ir-func (vector-get ctx 0)
      result (vector-get ctx 1)
      meta (vector-get ctx 2)
      offsets (vector-get ctx 3)
      depths (vector-get ctx 4)
      function-starts (vector-get ctx 5)
      function-metas (vector-get ctx 6)
      layout (vector-get ctx 7)
      import-count (vector-get layout 0)
      import-stub-offset (vector-get layout 1)
      function-start-base (vector-get layout 2)
      emit-start-base (vector-get layout 3)
      frame-base-slot-count (vector-get ctx 8)
      current-depth (vector-get depths idx)
      chunk-size (if (> remaining 64) 64 remaining)]
      (if (> remaining 64)
        (do
          (root_push ctx)
          (root_push ir-func)
          (root_push result)
          (root_push meta)
          (root_push offsets)
          (root_push depths)
          (root_push function-starts)
          (root_push function-metas)
          (let [chunk-state (make-x86-control-loop-state idx chunk-size)]
            (do
              (root_push chunk-state)
              (generate-native-control-instr-bundle-loop-x86-with-context ctx chunk-state)
              (root_pop)
              (let [tail-state (make-x86-control-loop-state (+ idx chunk-size) (- remaining chunk-size))]
                (do
                  (root_push tail-state)
                  (let [final (generate-native-control-instr-bundle-loop-x86-with-context ctx tail-state)]
                  (do
                    (root_pop)
                    (root_pop)
                    (root_pop)
                    (root_pop)
                    (root_pop)
                    (root_pop)
                    (root_pop)
                    (root_pop)
                    (root_pop)
                    final)))))))
        (let [instr (vector-get ir-func idx)
        opcode (vector-get instr 0)
	        operand (vector-get instr 1)
        current-offset (vector-get offsets idx)]
        (if (= opcode 40)
          (let [operand-ref (ref-new operand)]
            (do
              (root_push operand-ref)
              (let [actual-current-offset (x86-current-emitted-offset result emit-start-base)
                actual-current-offset-ref (ref-new actual-current-offset)]
                (do
                  (root_push actual-current-offset-ref)
                  (let [function-start-base-ref (ref-new function-start-base)]
                    (do
                      (root_push function-start-base-ref)
                      (let [import-stub-offset-ref (ref-new import-stub-offset)]
                        (do
                          (root_push import-stub-offset-ref)
                          (root_push function-starts)
                          (let [call-context (vector-push
                                               (vector-push
                                                 (vector-push
                                                   (vector-push
                                                     (vector-push
                                                       (vector-push (vector-new 6) function-metas)
                                                       import-count)
                                                     (ref-get import-stub-offset-ref))
                                                   (ref-get function-start-base-ref))
                                                 frame-base-slot-count)
                                               current-depth)]
                            (do
                              (root_push call-context)
	                              (let [native (codegen-x86-opcode-call-bundle (ref-get operand-ref) (ref-get actual-current-offset-ref) function-starts call-context)]
	                                (do
	                                  (root_push native)
	                                  (let [native-len (vector-length native)]
	                                    (do
	                                      (append-native-bytes-loop result native 0 native-len)
	                                      (root_pop)
	                                      (root_pop)
	                                      (root_pop)
	                                      (root_pop)
	                                      (root_pop)
	                                      (root_pop)
	                                      (root_pop)
	                                      (continue-native-control-instr-bundle-loop-x86 ctx idx remaining)))))))))))))))
        (if (= opcode 44)
          (do
            (append-drop-bundle-x86 result frame-base-slot-count current-depth)
            (continue-native-control-instr-bundle-loop-x86 ctx idx remaining))
        (if (= opcode 41)
          (do
            (append-control-if-instr-x86 result meta offsets idx)
            (continue-native-control-instr-bundle-loop-x86 ctx idx remaining))
        (if (= opcode 79)
          (do
            (append-control-else-instr-x86 result meta offsets idx)
            (continue-native-control-instr-bundle-loop-x86 ctx idx remaining))
        (if (= opcode 43)
          (continue-native-control-instr-bundle-loop-x86 ctx idx remaining)
        (if (= (direct-append-x86-opcode opcode) 6)
          (do
            (append-i64-const-bundle-x86 result operand frame-base-slot-count current-depth)
            (continue-native-control-instr-bundle-loop-x86 ctx idx remaining))
        (if (= (direct-append-x86-opcode opcode) 9)
          (do
            (append-root-push-bundle-x86 result)
            (continue-native-control-instr-bundle-loop-x86 ctx idx remaining))
        (if (= (direct-append-x86-opcode opcode) 8)
          (do
            (append-i32-const-bundle-x86 result (if (= opcode 75) 0 operand) frame-base-slot-count current-depth)
            (continue-native-control-instr-bundle-loop-x86 ctx idx remaining))
        (if (= (direct-append-x86-opcode opcode) 11)
          (do
            (append-root-set-bundle-x86 result frame-base-slot-count current-depth)
            (continue-native-control-instr-bundle-loop-x86 ctx idx remaining))
        (if (= opcode 60)
          (do
            (if (= current-depth 0)
              (append-call-rel32-x86
                result
                (- (x86-selfhost-map-new-helper-offset import-stub-offset import-count)
                   (+ (x86-current-emitted-offset result emit-start-base) 5)))
              (append-zero-arg-call-bundle-x86
                result
                (- (x86-selfhost-map-new-helper-offset import-stub-offset import-count)
                   (+ (x86-current-emitted-offset result emit-start-base)
                      (if (= current-depth 1)
                        6
                        (+ 13 (* (- current-depth 2) 14)))))
                frame-base-slot-count
                current-depth))
            (continue-native-control-instr-bundle-loop-x86 ctx idx remaining))
        (if (if (= opcode 64) true (if (= opcode 67) true (= opcode 59)))
          (do
            (append-x86-helper-call-preserving-rcx
              result
              (- (if (= opcode 64)
                   (x86-selfhost-read-file-helper-offset import-stub-offset import-count)
                   (if (= opcode 67)
                     (x86-selfhost-command-line-arg-helper-offset import-stub-offset import-count)
                     (x86-selfhost-print-helper-offset import-stub-offset import-count)))
                 (+ (x86-current-emitted-offset result emit-start-base) 6)))
            (continue-native-control-instr-bundle-loop-x86 ctx idx remaining))
        (if (if (= opcode 51) true (if (= opcode 52) true (= opcode 61)))
          (do
            (append-x86-helper-call-preserving-rcx
              result
              (- (if (= opcode 51)
                   (x86-selfhost-string-length-helper-offset import-stub-offset import-count)
                   (if (= opcode 52)
                     (x86-selfhost-vector-length-helper-offset import-stub-offset import-count)
                     (x86-selfhost-map-size-helper-offset import-stub-offset import-count)))
                 (+ (x86-current-emitted-offset result emit-start-base) 6)))
            (continue-native-control-instr-bundle-loop-x86 ctx idx remaining))
        (if (if (= opcode 50) true (if (= opcode 53) true (if (= opcode 55) true (if (= opcode 63) true (= opcode 70)))))
          (do
            (append-consume-two-helper-call-x86
              result
              (- (if (= opcode 50)
                   (x86-selfhost-string-char-at-helper-offset import-stub-offset import-count)
                   (if (= opcode 53)
                     (x86-selfhost-vector-get-helper-offset import-stub-offset import-count)
                     (if (= opcode 55)
                       (x86-selfhost-vector-push-helper-offset import-stub-offset import-count)
                       (if (= opcode 63)
                         (x86-selfhost-map-get-helper-offset import-stub-offset import-count)
                         (x86-selfhost-string-concat-helper-offset import-stub-offset import-count)))))
                 (+ (x86-current-emitted-offset result emit-start-base) 5))
              frame-base-slot-count
              current-depth)
            (continue-native-control-instr-bundle-loop-x86 ctx idx remaining))
        (if (= opcode 54)
          (do
            (append-x86-helper-call-preserving-rcx
              result
              (- (x86-selfhost-vector-new-helper-offset import-stub-offset import-count)
                 (+ (x86-current-emitted-offset result emit-start-base) 6)))
            (continue-native-control-instr-bundle-loop-x86 ctx idx remaining))
        (if (= opcode 56)
          (do
            (append-x86-helper-call-preserving-rcx
              result
              (- (x86-selfhost-ref-new-helper-offset import-stub-offset import-count)
                 (+ (x86-current-emitted-offset result emit-start-base) 6)))
            (continue-native-control-instr-bundle-loop-x86 ctx idx remaining))
        (if (if (= opcode 57) true (= opcode 73))
          (do
            (append-x86-helper-call-preserving-rcx
              result
              (- (if (= opcode 57)
                   (x86-selfhost-ref-get-helper-offset import-stub-offset import-count)
                   (x86-selfhost-file-exists-helper-offset import-stub-offset import-count))
                 (+ (x86-current-emitted-offset result emit-start-base) 6)))
            (continue-native-control-instr-bundle-loop-x86 ctx idx remaining))
        (if (= opcode 58)
          (do
            (append-consume-two-helper-call-x86
              result
              (- (x86-selfhost-ref-set-helper-offset import-stub-offset import-count)
                 (+ (x86-current-emitted-offset result emit-start-base) 5))
              frame-base-slot-count
              current-depth)
            (continue-native-control-instr-bundle-loop-x86 ctx idx remaining))
        (if (= opcode 69)
          (do
            (append-substring-helper-call-x86
              result
              (- (x86-selfhost-substring-helper-offset import-stub-offset import-count)
                 (+ (x86-current-emitted-offset result emit-start-base) 12))
              frame-base-slot-count
              current-depth)
            (continue-native-control-instr-bundle-loop-x86 ctx idx remaining))
        (if (= opcode 62)
          (do
            (append-map-insert-helper-call-x86
              result
              (- (x86-selfhost-map-insert-helper-offset import-stub-offset import-count)
                 (+ (x86-current-emitted-offset result emit-start-base) 12))
              frame-base-slot-count
              current-depth)
            (continue-native-control-instr-bundle-loop-x86 ctx idx remaining))
        (if (= (direct-append-x86-opcode opcode) 1)
          (do
            (append-produce-one-bundle-x86 result (direct-append-produce-one-bytes-x86 opcode operand) frame-base-slot-count current-depth)
            (continue-native-control-instr-bundle-loop-x86 ctx idx remaining))
          (if (= (direct-append-x86-opcode opcode) 2)
            (do
              (append-consume-two-bundle-x86 result (codegen-ir-instr opcode operand) frame-base-slot-count current-depth)
              (continue-native-control-instr-bundle-loop-x86 ctx idx remaining))
            (if (= (direct-append-x86-opcode opcode) 3)
              (do
                (append-local-set-bundle-x86 result (local-slot-offset operand) frame-base-slot-count current-depth)
                (continue-native-control-instr-bundle-loop-x86 ctx idx remaining))
              (if (= (direct-append-x86-opcode opcode) 4)
                (do
                  (append-local-get-bundle-x86 result (local-slot-offset operand) frame-base-slot-count current-depth)
                  (continue-native-control-instr-bundle-loop-x86 ctx idx remaining))
                (if (= (direct-append-x86-opcode opcode) 5)
                  (do
                    (append-drop-bundle-x86 result frame-base-slot-count current-depth)
                    (continue-native-control-instr-bundle-loop-x86 ctx idx remaining))
                  (if (= (x86-plain-two-to-one-needs-window-restore opcode) 1)
                    (do
                      (append-plain-two-to-one-codegen-bundle-x86 result opcode operand frame-base-slot-count current-depth)
                      (continue-native-control-instr-bundle-loop-x86 ctx idx remaining))
                    (let [native (codegen-x86-control-loop-fallback-native ctx idx opcode operand (x86-current-emitted-offset result emit-start-base))
                      native-len (vector-length native)]
                      (do
                        (append-native-bytes-loop result native 0 native-len)
                        (continue-native-control-instr-bundle-loop-x86 ctx idx remaining))))))))))))))))))))))))))))))))))
(defn generate-native-control-instr-bundle-loop-x86-with-import-count-and-base [ir-func result meta offsets function-starts function-metas import-count import-stub-offset function-start-base frame-base-slot-count current-depth idx len]
  (let [emit-start-base (vector-length (ref-get result))
    layout (make-x86-function-emit-layout import-count import-stub-offset function-start-base emit-start-base)]
    (do
      (root_push layout)
      (let [depths (collect-native-bundle-depths-x86 ir-func function-metas)]
        (do
          (root_push depths)
          (let [ctx (make-x86-control-bundle-context ir-func result meta offsets depths function-starts function-metas layout frame-base-slot-count)]
            (do
              (root_push ctx)
              (let [state (make-x86-control-loop-state idx len)]
                (let [final (generate-native-control-instr-bundle-loop-x86-with-context ctx state)]
                (do
                  (root_pop)
                  (root_pop)
                  (root_pop)
                  final))))))))))

(defn generate-native-control-instr-bundle-loop-x86-with-import-count [ir-func result meta offsets function-starts function-metas import-count import-stub-offset frame-base-slot-count current-depth idx len]
  (generate-native-control-instr-bundle-loop-x86-with-import-count-and-base ir-func result meta offsets function-starts function-metas import-count import-stub-offset 0 frame-base-slot-count current-depth idx len))

(defn generate-native-control-instr-bundle-loop-x86 [ir-func result meta offsets function-starts function-metas frame-base-slot-count current-depth idx len]
  (generate-native-control-instr-bundle-loop-x86-with-import-count ir-func result meta offsets function-starts function-metas 0 0 frame-base-slot-count current-depth idx len))

;; === コード生成メイン関数 ===

(defn append-native-bytes-loop-bounded [result native idx len remaining]
  (if (if (>= idx len) true (<= remaining 0))
    idx
    (let [current (ref-get result)]
      (do
        (root_push native)
        (root_push current)
        (let [next (vector-push current (vector-get native idx))]
          (do
            (root_push next)
            (ref-set result next)
            (let [final (append-native-bytes-loop-bounded result native (+ idx 1) len (- remaining 1))]
              (do
                (root_pop)
                (root_pop)
                (root_pop)
                final))))))))

(defn continue-append-native-bytes-loop-step-64 [result native len idx]
  (if (>= idx len)
    0
    (do
      (root_push result)
      (root_push native)
      (let [next-idx (append-native-bytes-loop-bounded result native idx len 64)]
        (let [final (continue-append-native-bytes-loop-step-64 result native len next-idx)]
          (do
            (root_pop)
            (root_pop)
            final))))))

(defn append-native-bytes-loop [result native idx len]
  (continue-append-native-bytes-loop-step-64 result native len idx))

(defn append-native-bytes-rooted [result native len]
  (do
    (root_push native)
    (let [final (append-native-bytes-loop result native 0 len)]
      (do
        (root_pop)
        final))))

(defn append-native-value-window-spill-one-step-x86 [result frame-base-slot-count current-depth]
  (do
    (append-native-bytes-rooted result (emit-mov-rdx-from-local (native-value-window-spill-offset frame-base-slot-count (- current-depth 3))) 7)
    (append-native-bytes-rooted result (emit-mov-local-from-rdx (native-value-window-spill-offset frame-base-slot-count (- current-depth 2))) 7)
    0))

(defn append-native-value-window-overflow-spills-x86 [result frame-base-slot-count current-depth]
  (if (>= current-depth 55)
    (do
      (append-native-value-window-spill-one-step-x86 result frame-base-slot-count current-depth)
      (append-native-value-window-overflow-spills-x86 result frame-base-slot-count (- current-depth 1)))
    current-depth))

(defn append-shift-native-value-window-x86-loop [result frame-base-slot-count idx]
  (if (< idx 0)
    0
    (do
      (append-native-bytes-rooted result (emit-mov-rdx-from-local (native-value-window-spill-offset frame-base-slot-count idx)) 7)
      (append-native-bytes-rooted result (emit-mov-local-from-rdx (native-value-window-spill-offset frame-base-slot-count (+ idx 1))) 7)
      (append-shift-native-value-window-x86-loop result frame-base-slot-count (- idx 1)))))

(defn append-produce-one-bundle-x86-core [result op-bytes frame-base-slot-count current-depth]
  (do
    (root_push op-bytes)
    (if (>= current-depth 2)
      (do
        (append-shift-native-value-window-x86-loop result frame-base-slot-count (- current-depth 3))
        (append-native-bytes-rooted result (emit-mov-local-from-rcx (native-value-window-spill-offset frame-base-slot-count 0)) 7))
      0)
    (append-native-bytes-loop result op-bytes 0 (vector-length op-bytes))
    (root_pop)
    0))

(defn append-produce-one-bundle-x86 [result op-bytes frame-base-slot-count current-depth]
  (let [core-depth (append-native-value-window-overflow-spills-x86 result frame-base-slot-count current-depth)]
    (append-produce-one-bundle-x86-core result op-bytes frame-base-slot-count core-depth)))

(defn append-i64-const-bundle-x86 [result value frame-base-slot-count current-depth]
  (let [core-depth (append-native-value-window-overflow-spills-x86 result frame-base-slot-count current-depth)]
    (if (>= core-depth 2)
        (do
          (append-shift-native-value-window-x86-loop result frame-base-slot-count (- core-depth 3))
        (append-x86-rbp-disp32 result 72 137 141 (native-value-window-spill-offset frame-base-slot-count 0))
        (append-mov-rcx-rax-x86 result)
        (append-mov-rax-imm64-x86 result value))
      (if (= core-depth 1)
        (do
          (append-mov-rcx-rax-x86 result)
          (append-mov-rax-imm64-x86 result value))
        (append-mov-rax-imm64-x86 result value)))))

(defn append-mov-rcx-rax-x86 [result]
  (do
    (append-x86-byte result 72)
    (append-x86-byte result 137)
    (append-x86-byte result 193)))

(defn append-mov-rax-rcx-x86 [result]
  (do
    (append-x86-byte result 72)
    (append-x86-byte result 137)
    (append-x86-byte result 200)))

(defn append-mov-rax-imm64-x86 [result value]
  (let [low-value (if (< value 0) (+ 4294967296 value) value)
    high-value (if (< value 0) 4294967295 (/ value 4294967296))]
    (do
      (append-x86-byte result 72)
      (append-x86-byte result 184)
      (append-u32-immediate-x86 result low-value)
      (append-u32-immediate-x86 result high-value))))

(defn append-u32-immediate-x86 [result value]
  (let [u32 (if (< value 0) (+ 4294967296 value) value)]
    (do
      (append-x86-byte result (% u32 256))
      (append-x86-byte result (% (/ u32 256) 256))
      (append-x86-byte result (% (/ u32 65536) 256))
      (append-x86-byte result (% (/ u32 16777216) 256)))))

(defn append-mov-eax-imm32-x86 [result value]
  (do
    (append-x86-byte result 184)
    (append-u32-immediate-x86 result value)))

(defn append-call-rel32-x86 [result disp]
  (do
    (append-x86-byte result 232)
    (append-u32-immediate-x86 result disp)))

(defn append-x86-helper-call-preserving-rcx [result rel]
  (do
    (append-x86-byte result 81)
    (append-call-rel32-x86 result rel)
    (append-x86-byte result 89)))

(defn append-zero-arg-call-bundle-x86 [result call-rel frame-base-slot-count current-depth]
  (if (= current-depth 0)
    (append-call-rel32-x86 result call-rel)
    (if (= current-depth 1)
      (do
        (append-x86-byte result 80)
        (append-call-rel32-x86 result call-rel)
        (append-x86-byte result 89))
      (do
        (append-shift-native-value-window-x86-loop result frame-base-slot-count (- current-depth 3))
        (append-x86-rbp-disp32 result 72 137 141 (native-value-window-spill-offset frame-base-slot-count 0))
        (append-x86-byte result 80)
        (append-call-rel32-x86 result call-rel)
        (append-x86-byte result 89)))))

(defn append-i32-const-bundle-x86 [result value frame-base-slot-count current-depth]
  (let [core-depth (append-native-value-window-overflow-spills-x86 result frame-base-slot-count current-depth)]
    (do
      (if (>= core-depth 2)
        (do
          (append-shift-native-value-window-x86-loop result frame-base-slot-count (- core-depth 3))
          (append-x86-rbp-disp32 result 72 137 141 (native-value-window-spill-offset frame-base-slot-count 0)))
        0)
      (append-mov-rcx-rax-x86 result)
      (append-mov-eax-imm32-x86 result value))))

(defn append-root-push-bundle-x86 [result]
  (do
    (append-x86-byte result 49)
    (append-x86-byte result 192)))

(defn append-root-set-bundle-x86 [result frame-base-slot-count current-depth]
  (do
    (if (>= current-depth 3)
      (do
        (append-mov-rcx-rax-x86 result)
        (append-x86-rbp-disp32 result 72 139 141 (native-value-window-spill-offset frame-base-slot-count 0))
        (append-drop-window-spill-shifts-x86 result frame-base-slot-count 1 (- current-depth 3)))
      (if (= current-depth 2)
        (append-mov-rcx-rax-x86 result)
        0))
    0))

(defn append-local-get-bundle-x86-core [result offset frame-base-slot-count current-depth]
  (do
    (if (>= current-depth 2)
      (do
        (append-shift-native-value-window-x86-loop result frame-base-slot-count (- current-depth 3))
        (append-x86-rbp-disp32 result 72 137 141 (native-value-window-spill-offset frame-base-slot-count 0))
        (append-mov-rcx-rax-x86 result))
      0)
    (append-mov-rcx-rax-x86 result)
    (append-x86-rbp-disp32 result 72 139 133 offset)
    0))

(defn append-local-get-bundle-x86 [result offset frame-base-slot-count current-depth]
  (let [core-depth (append-native-value-window-overflow-spills-x86 result frame-base-slot-count current-depth)]
    (append-local-get-bundle-x86-core result offset frame-base-slot-count core-depth)))

(defn append-drop-window-spill-shifts-x86 [result frame-base-slot-count shift-idx last-shift-idx]
  (if (> shift-idx last-shift-idx)
    0
    (do
      (append-native-bytes-rooted result (emit-mov-rsi-from-local (native-value-window-spill-offset frame-base-slot-count shift-idx)) 7)
      (append-native-bytes-rooted result (emit-mov-local-from-rsi (native-value-window-spill-offset frame-base-slot-count (- shift-idx 1))) 7)
      (append-drop-window-spill-shifts-x86 result frame-base-slot-count (+ shift-idx 1) last-shift-idx))))

(defn append-store-window-spill-shifts-x86 [result frame-base-slot-count shift-idx last-shift-idx]
  (if (> shift-idx last-shift-idx)
    0
    (do
      (append-native-bytes-rooted result (emit-mov-rsi-from-local (native-value-window-spill-offset frame-base-slot-count shift-idx)) 7)
      (append-native-bytes-rooted result (emit-mov-local-from-rsi (native-value-window-spill-offset frame-base-slot-count (- shift-idx 2))) 7)
      (append-store-window-spill-shifts-x86 result frame-base-slot-count (+ shift-idx 1) last-shift-idx))))

(defn append-drop-bundle-x86 [result frame-base-slot-count current-depth]
  (do
    (append-mov-rax-rcx-x86 result)
    (if (>= current-depth 3)
      (do
        (append-x86-rbp-disp32 result 72 139 141 (native-value-window-spill-offset frame-base-slot-count 0))
        (append-drop-window-spill-shifts-x86 result frame-base-slot-count 1 (- current-depth 3)))
      0)))

(defn append-local-set-bundle-x86 [result offset frame-base-slot-count current-depth]
  (do
    (append-x86-rbp-disp32 result 72 137 133 offset)
    (if (>= current-depth 2)
      (do
        (append-x86-byte result 72)
        (append-x86-byte result 137)
        (append-x86-byte result 200))
      0)
    (if (>= current-depth 3)
      (do
        (append-x86-rbp-disp32 result 72 139 141 (native-value-window-spill-offset frame-base-slot-count 0))
        (append-drop-window-spill-shifts-x86 result frame-base-slot-count 1 (- current-depth 3)))
      0)))

(defn append-consume-two-bundle-x86 [result op-bytes frame-base-slot-count current-depth]
  (do
    (root_push op-bytes)
    (append-native-bytes-loop result op-bytes 0 (vector-length op-bytes))
    (if (>= current-depth 3)
      (do
        (append-native-bytes-rooted result (emit-mov-rcx-from-local (native-value-window-spill-offset frame-base-slot-count 0)) 7)
        (append-drop-window-spill-shifts-x86 result frame-base-slot-count 1 (- current-depth 3)))
      0)
    (root_pop)
    0))

(defn append-consume-two-helper-call-x86 [result rel frame-base-slot-count current-depth]
  (do
    (append-call-rel32-x86 result rel)
    (if (>= current-depth 3)
      (do
        (append-native-bytes-rooted result (emit-mov-rcx-from-local (native-value-window-spill-offset frame-base-slot-count 0)) 7)
        (append-drop-window-spill-shifts-x86 result frame-base-slot-count 1 (- current-depth 3)))
      0)
    0))

(defn append-map-insert-helper-call-x86 [result rel frame-base-slot-count current-depth]
  (do
    (append-native-bytes-rooted result (emit-mov-rdx-from-local (native-value-window-spill-offset frame-base-slot-count 0)) 7)
    (append-call-rel32-x86 result rel)
    (if (>= current-depth 5)
      (do
        (append-native-bytes-rooted result (emit-mov-rcx-from-local (native-value-window-spill-offset frame-base-slot-count 1)) 7)
        (append-store-window-spill-shifts-x86 result frame-base-slot-count 2 (- current-depth 3)))
      (if (= current-depth 4)
        (append-native-bytes-rooted result (emit-mov-rcx-from-local (native-value-window-spill-offset frame-base-slot-count 1)) 7)
        0))
    0))

(defn append-substring-helper-call-x86 [result rel frame-base-slot-count current-depth]
  (append-map-insert-helper-call-x86 result rel frame-base-slot-count current-depth))

(defn append-plain-two-to-one-codegen-bundle-x86 [result opcode operand frame-base-slot-count current-depth]
  (let [op-bytes (codegen-ir-instr opcode operand)]
    (append-consume-two-bundle-x86 result op-bytes frame-base-slot-count current-depth)))

(defn append-control-if-instr-x86 [result meta offsets idx]
  (let [native (emit-control-if-x86 meta offsets idx)
    native-len (vector-length native)]
    (append-native-bytes-rooted result native native-len)))

(defn append-control-else-instr-x86 [result meta offsets idx]
  (let [native (emit-control-else-x86 meta offsets idx)
    native-len (vector-length native)]
    (append-native-bytes-rooted result native native-len)))

(defn direct-append-x86-opcode [opcode]
    (if (= opcode 1)
      6
      (if (= opcode 74)
        9
        (if (= opcode 3)
          8
        (if (= opcode 75)
          8
          (if (= opcode 10)
            4
          (if (= opcode 76)
            11
            (if (= opcode 11)
              3
              0))))))))

(defn direct-append-produce-one-bytes-x86 [opcode operand]
  (if (= opcode 1)
    (emit-mov-imm64 (reg-rax) operand)
    (emit-i32-const-x86 (if (= opcode 75) 0 operand))))

(defn append-encoded-u32-rooted [result value]
  (do
    (root_push result)
    (let [bytes (encode-u32-le value)]
      (do
        (root_push bytes)
        (append-native-bytes-rooted result bytes 4)
        (root_pop)
        (root_pop)))))

(defn append-x86-byte [result value]
  (ref-set result (vector-push (ref-get result) value)))

(defn append-x86-rbp-disp32 [result rex opcode modrm offset]
  (let [disp (encode-u32-le (- 4294967296 offset))]
    (do
      (root_push disp)
      (append-x86-byte result rex)
      (append-x86-byte result opcode)
      (append-x86-byte result modrm)
      (append-x86-byte result (vector-get disp 0))
      (append-x86-byte result (vector-get disp 1))
      (append-x86-byte result (vector-get disp 2))
      (append-x86-byte result (vector-get disp 3))
      (root_pop))))

(defn make-control-flow-meta [end-map else-map branch-map]
  (vector-push
    (vector-push
      (vector-push (vector-new 3) end-map)
      else-map)
    branch-map))

(defn control-flow-end-map [meta]
  (vector-get meta 0))

(defn control-flow-else-map [meta]
  (vector-get meta 1))

(defn control-flow-branch-map [meta]
  (vector-get meta 2))

(defn scan-control-flow-meta-handle-start [ir-func idx len stack depth end-map else-map branch-map opcode]
  (let [entry (make-control-stack-entry idx opcode)
    next-stack (control-stack-push stack depth entry)]
    (scan-control-flow-meta-loop ir-func (+ idx 1) len next-stack (+ depth 1) end-map else-map branch-map)))

(defn scan-control-flow-meta-handle-else [ir-func idx len stack depth end-map else-map branch-map]
  (let [top-entry (vector-get stack (- depth 1))
    start-idx (control-stack-entry-start top-entry)
    next-else-map (map-insert-index else-map start-idx idx)]
    (scan-control-flow-meta-loop ir-func (+ idx 1) len stack depth end-map next-else-map branch-map)))

(defn scan-control-flow-meta-handle-end [ir-func idx len stack depth end-map else-map branch-map]
  (let [top-entry (vector-get stack (- depth 1))
    start-idx (control-stack-entry-start top-entry)
    else-idx (map-get-index else-map start-idx)
    end-map1 (map-insert-index end-map start-idx idx)]
    (do
      (root_push end-map1)
      (let [end-map2 (if (< else-idx 0)
                   end-map1
                   (map-insert-index end-map1 else-idx idx))]
        (do
          (root_push end-map2)
          (let [final (scan-control-flow-meta-loop ir-func (+ idx 1) len stack (- depth 1) end-map2 else-map branch-map)]
            (do
              (root_pop)
              (root_pop)
              final)))))))

(defn scan-control-flow-meta-handle-branch [ir-func idx len stack depth end-map else-map branch-map operand]
  (let [target-entry (vector-get stack (- depth (+ operand 1)))
    target-start (control-stack-entry-start target-entry)
    next-branch-map (map-insert-index branch-map idx target-start)]
    (scan-control-flow-meta-loop ir-func (+ idx 1) len stack depth end-map else-map next-branch-map)))

(defn scan-control-flow-meta-loop [ir-func idx len stack depth end-map else-map branch-map]
  (if (>= idx len)
    (make-control-flow-meta end-map else-map branch-map)
    (let [instr (vector-get ir-func idx)
      opcode (vector-get instr 0)
      operand (vector-get instr 1)]
      (if (= (is-control-start-opcode opcode) 1)
        (scan-control-flow-meta-handle-start ir-func idx len stack depth end-map else-map branch-map opcode)
        (if (= opcode 79)
          (scan-control-flow-meta-handle-else ir-func idx len stack depth end-map else-map branch-map)
          (if (= opcode 43)
            (scan-control-flow-meta-handle-end ir-func idx len stack depth end-map else-map branch-map)
            (if (= opcode 80)
              (scan-control-flow-meta-handle-branch ir-func idx len stack depth end-map else-map branch-map operand)
              (if (= opcode 81)
                (scan-control-flow-meta-handle-branch ir-func idx len stack depth end-map else-map branch-map operand)
                (scan-control-flow-meta-loop ir-func (+ idx 1) len stack depth end-map else-map branch-map)))))))))

(defn make-control-flow-scan-state [done next-idx next-stack next-depth next-end-map next-else-map next-branch-map]
  (do
    (root_push next-stack)
    (root_push next-end-map)
    (root_push next-else-map)
    (root_push next-branch-map)
    (let [base0 (vector-push (vector-new 7) done)]
      (do
        (root_push base0)
        (let [base1 (vector-push base0 next-idx)]
          (do
            (root_push base1)
            (let [base2 (vector-push base1 next-stack)]
              (do
                (root_push base2)
                (let [base3 (vector-push base2 next-depth)]
                  (do
                    (root_push base3)
                    (let [base4 (vector-push base3 next-end-map)]
                      (do
                        (root_push base4)
                        (let [base5 (vector-push base4 next-else-map)]
                          (do
                            (root_push base5)
                            (let [state (vector-push base5 next-branch-map)]
                              (do
                                (root_pop)
                                (root_pop)
                                (root_pop)
                                (root_pop)
                                (root_pop)
                                (root_pop)
                                (root_pop)
                                (root_pop)
                                (root_pop)
                                (root_pop)
                                state))))))))))))))))

(defn scan-control-flow-meta-step [ir-func idx len stack depth end-map else-map branch-map]
  (if (>= idx len)
    (make-control-flow-scan-state 1 idx stack depth end-map else-map branch-map)
    (let [instr (vector-get ir-func idx)
      opcode (vector-get instr 0)
      operand (vector-get instr 1)]
      (if (= (is-control-start-opcode opcode) 1)
        (let [entry (make-control-stack-entry idx opcode)
          next-stack (control-stack-push stack depth entry)]
          (make-control-flow-scan-state 0 (+ idx 1) next-stack (+ depth 1) end-map else-map branch-map))
        (if (= opcode 79)
          (let [top-entry (vector-get stack (- depth 1))
            start-idx (control-stack-entry-start top-entry)
            next-else-map (map-insert-index else-map start-idx idx)]
            (make-control-flow-scan-state 0 (+ idx 1) stack depth end-map next-else-map branch-map))
          (if (= opcode 43)
            (let [top-entry (vector-get stack (- depth 1))
              start-idx (control-stack-entry-start top-entry)
              else-idx (map-get-index else-map start-idx)
              end-map1 (map-insert-index end-map start-idx idx)]
              (do
                (root_push end-map1)
                (let [end-map2 (if (< else-idx 0)
                             end-map1
                             (map-insert-index end-map1 else-idx idx))]
                  (do
                    (root_push end-map2)
                    (let [state (make-control-flow-scan-state 0 (+ idx 1) stack (- depth 1) end-map2 else-map branch-map)]
                      (do
                        (root_pop)
                        (root_pop)
                        state))))))
            (if (= opcode 80)
              (let [target-entry (vector-get stack (- depth (+ operand 1)))
                target-start (control-stack-entry-start target-entry)
                next-branch-map (map-insert-index branch-map idx target-start)]
                (make-control-flow-scan-state 0 (+ idx 1) stack depth end-map else-map next-branch-map))
              (if (= opcode 81)
                (let [target-entry (vector-get stack (- depth (+ operand 1)))
                  target-start (control-stack-entry-start target-entry)
                  next-branch-map (map-insert-index branch-map idx target-start)]
                  (make-control-flow-scan-state 0 (+ idx 1) stack depth end-map else-map next-branch-map))
                (make-control-flow-scan-state 0 (+ idx 1) stack depth end-map else-map branch-map)))))))))

(defn scan-control-flow-meta-step-64-loop-bounded [ir-func idx len stack depth end-map else-map branch-map remaining]
  (do
    (root_push ir-func)
    (root_push stack)
    (root_push end-map)
    (root_push else-map)
    (root_push branch-map)
    (let [state (scan-control-flow-meta-step ir-func idx len stack depth end-map else-map branch-map)
      done (vector-get state 0)
      next-idx (vector-get state 1)
      next-stack (vector-get state 2)
      next-depth (vector-get state 3)
      next-end-map (vector-get state 4)
      next-else-map (vector-get state 5)
      next-branch-map (vector-get state 6)]
      (do
        (root_push state)
        (root_push next-stack)
        (root_push next-end-map)
        (root_push next-else-map)
        (root_push next-branch-map)
        (let [final
          (if (= done 1)
            state
            (if (<= remaining 1)
              state
              (scan-control-flow-meta-step-64-loop-bounded ir-func next-idx len next-stack next-depth next-end-map next-else-map next-branch-map (- remaining 1))))]
          (do
            (root_pop)
            (root_pop)
            (root_pop)
            (root_pop)
            (root_pop)
            (root_pop)
            (root_pop)
            (root_pop)
            (root_pop)
            (root_pop)
            final))))))

(defn scan-control-flow-meta-step-64 [ir-func idx len stack depth end-map else-map branch-map]
  (scan-control-flow-meta-step-64-loop-bounded ir-func idx len stack depth end-map else-map branch-map 64))

(defn continue-scan-control-flow-meta-step-64 [ir-func len state]
  (if (= (vector-get state 0) 1)
    state
    (do
      (root_push ir-func)
      (root_push state)
      (let [next-state
        (scan-control-flow-meta-step-64
          ir-func
          (vector-get state 1)
          len
          (vector-get state 2)
          (vector-get state 3)
          (vector-get state 4)
          (vector-get state 5)
          (vector-get state 6))]
        (do
          (root_push next-state)
          (let [final (continue-scan-control-flow-meta-step-64 ir-func len next-state)]
            (do
              (root_pop)
              (root_pop)
              (root_pop)
              final)))))))

(defn scan-control-flow-meta [ir-func]
  (do
    (root_push ir-func)
    (let [len (vector-length ir-func)
      state (continue-scan-control-flow-meta-step-64
              ir-func
              (vector-length ir-func)
              (scan-control-flow-meta-step-64 ir-func 0 len (vector-new 8) 0 (map-new) (map-new) (map-new)))]
      (do
        (root_push state)
        (let [end-map (vector-get state 4)
          else-map (vector-get state 5)
          branch-map (vector-get state 6)]
          (do
            (root_push end-map)
            (root_push else-map)
            (root_push branch-map)
            (let [meta (make-control-flow-meta end-map else-map branch-map)]
              (do
                (root_pop)
                (root_pop)
                (root_pop)
                (root_pop)
                (root_pop)
                meta))))))))

(defn native-control-instr-size-x86 [opcode]
  (if (= opcode 41)
    8
    (if (= opcode 79)
      5
      (if (= opcode 80)
        5
        (if (= opcode 81)
          8
          (if (= opcode 83)
            8
            0))))))

(defn native-control-instr-size-aarch64 [opcode]
  (if (= opcode 41)
    4
    (if (= opcode 79)
      4
      (if (= opcode 80)
        4
        (if (= opcode 81)
          4
          (if (= opcode 83)
            4
            0))))))

(defn native-drop-bundle-size-aarch64 [current-depth]
  (if (>= current-depth 3)
    (+ 8 (* (- current-depth 3) 8))
    4))

(defn native-conditional-control-instr-size-aarch64 [current-depth]
  (+ (+ 4 (native-drop-bundle-size-aarch64 current-depth)) 4))

(defn native-plain-instr-size-x86 [opcode operand]
  (if (= (is-control-opcode opcode) 1)
    (native-control-instr-size-x86 opcode)
    (if (= opcode 1)
      10
      (if (= opcode 3)
        8
        (if (= opcode 10)
          10
          (if (= opcode 11)
            7
            (if (= opcode 20)
              3
              (if (= opcode 21)
                6
                (if (= opcode 22)
                  4
                  (if (= opcode 23)
                    11
                    (if (= opcode 24)
                      2
                      (if (= opcode 25)
                        3
                        (if (= opcode 26)
                          2
                          (if (= opcode 27)
                            2
                            (if (= opcode 28)
                              14
                              (if (= opcode 71)
                                2
                                (if (= opcode 72)
                                  2
                                  (if (= opcode 75)
                                    8
                                    (if (= (is-i64-compare-opcode opcode) 1)
                                      9
                                      (if (= opcode 36)
                                        3
                                        (if (= opcode 37)
                                          2
                                          (if (= opcode 38)
                                            2
                                              (if (= opcode 44)
                                                3
                                                (if (= opcode 45)
                                                 (if (< operand 128) 7 10)
                                                 (if (= opcode 46)
                                                   (if (< operand 128) 7 10)
                                                   (if (= opcode 47)
                                                     (if (< operand 128) 8 11)
                                                     (if (= opcode 48)
                                                       (if (< operand 128) 8 11)
                                                       (if (= opcode 49)
                                                         (if (< operand 128) 8 11)
                                                         1))))))))))))))))))))))))))))

(defn native-plain-instr-size-aarch64 [opcode operand]
  (if (= (is-control-opcode opcode) 1)
    (native-control-instr-size-aarch64 opcode)
    (if (= opcode 1)
      (aarch64-load-i64-x0-size operand)
      (if (= opcode 3)
        (+ 4 (aarch64-load-u32-w0-size operand))
        (if (= opcode 10)
          8
          (if (= opcode 11)
            4
            (if (= opcode 20)
              4
              (if (= opcode 21)
                4
                (if (= opcode 22)
                  4
                  (if (= opcode 23)
                    4
                    (if (= opcode 24)
                      4
                      (if (= opcode 25)
                        4
                        (if (= opcode 26)
                          4
                          (if (= opcode 27)
                            4
                            (if (= opcode 28)
                              12
                              (if (= opcode 71)
                                4
                                (if (= opcode 72)
                                  4
                                  (if (= opcode 74)
                                    16
                                    (if (= opcode 75)
                                      12
                                      (if (= opcode 76)
                                        12
                                      (if (= (is-i64-compare-opcode opcode) 1)
                                        8
                                        (if (= opcode 36)
                                        4
                                        (if (= opcode 37)
                                          4
                                          (if (= opcode 38)
                                            4
                                             (if (= opcode 44)
                                               4
                                               (if (= opcode 45)
                                                 8
                                                 (if (= opcode 46)
                                                   8
                                                   (if (= opcode 47)
                                                     8
                                                     (if (= opcode 48)
                                                       8
                                                       (if (= opcode 49)
                                                         8
                                                          4))))))))))))))))))))))))))))))

(defn collect-native-offsets-x86-loop [ir-func result current-offset idx len]
  (if (>= idx len)
    (vector-push result current-offset)
    (let [instr (vector-get ir-func idx)
      opcode (vector-get instr 0)
      operand (vector-get instr 1)
      next-offset (+ current-offset (native-plain-instr-size-x86 opcode operand))]
      (collect-native-offsets-x86-loop ir-func (vector-push result current-offset) next-offset (+ idx 1) len))))

(defn collect-native-offsets-x86 [ir-func]
  (collect-native-offsets-x86-loop ir-func (vector-new (+ (vector-length ir-func) 1)) 0 0 (vector-length ir-func)))

(defn collect-native-bundle-offsets-x86-loop [ir-func function-metas result current-offset current-depth idx len]
  (if (>= idx len)
    (vector-push result current-offset)
    (let [instr (vector-get ir-func idx)
      opcode (vector-get instr 0)
      operand (vector-get instr 1)
      next-offset (+ current-offset (native-instr-size-x86 opcode operand function-metas current-depth))
      next-depth (apply-stack-delta current-depth (opcode-stack-delta opcode operand function-metas))]
      (collect-native-bundle-offsets-x86-loop ir-func function-metas (vector-push result current-offset) next-offset next-depth (+ idx 1) len))))

(defn collect-native-bundle-offsets-x86 [ir-func function-metas start-offset]
  (vector-get (collect-native-bundle-offset-depths-x86 ir-func function-metas start-offset) 2))

(defn make-native-bundle-offset-depth-collection-state [done next-idx offsets depths next-offset next-depth]
  (do
    (root_push offsets)
    (root_push depths)
    (let [base0 (vector-push (vector-new 6) done)]
      (do
        (root_push base0)
        (let [base1 (vector-push base0 next-idx)]
          (do
            (root_push base1)
            (let [base2 (vector-push base1 offsets)]
              (do
                (root_push base2)
                (let [base3 (vector-push base2 depths)]
                  (do
                    (root_push base3)
                    (let [base4 (vector-push base3 next-offset)]
                      (do
                        (root_push base4)
                        (let [state (vector-push base4 next-depth)]
                          (do
                            (root_pop)
                            (root_pop)
                            (root_pop)
                            (root_pop)
                            (root_pop)
                            (root_pop)
                            (root_pop)
                            state))))))))))))))

(defn collect-native-bundle-offset-depths-x86-step [ir-func function-metas offsets depths current-offset current-depth idx len]
  (if (>= idx len)
    (make-native-bundle-offset-depth-collection-state
      1
      idx
      (vector-push offsets current-offset)
      (vector-push depths current-depth)
      current-offset
      current-depth)
    (let [instr (vector-get ir-func idx)
      opcode (vector-get instr 0)
      operand (vector-get instr 1)
      next-offset (+ current-offset (native-instr-size-x86 opcode operand function-metas current-depth))
      next-depth (apply-stack-delta current-depth (opcode-stack-delta opcode operand function-metas))
      next-offsets (vector-push offsets current-offset)
      next-depths (vector-push depths current-depth)]
      (make-native-bundle-offset-depth-collection-state
        0
        (+ idx 1)
        next-offsets
        next-depths
        next-offset
        next-depth))))

(defn collect-native-bundle-offset-depths-x86-step-64-loop-bounded [ir-func function-metas offsets depths current-offset current-depth idx len remaining]
  (do
    (root_push ir-func)
    (root_push function-metas)
    (root_push offsets)
    (root_push depths)
    (let [state (collect-native-bundle-offset-depths-x86-step ir-func function-metas offsets depths current-offset current-depth idx len)
      done (vector-get state 0)
      next-idx (vector-get state 1)
      next-offsets (vector-get state 2)
      next-depths (vector-get state 3)
      next-offset (vector-get state 4)
      next-depth (vector-get state 5)]
      (do
        (root_push state)
        (root_push next-offsets)
        (root_push next-depths)
        (let [final
          (if (= done 1)
            state
            (if (<= remaining 1)
              state
              (collect-native-bundle-offset-depths-x86-step-64-loop-bounded
                ir-func
                function-metas
                next-offsets
                next-depths
                next-offset
                next-depth
                next-idx
                len
                (- remaining 1))))]
          (do
            (root_pop)
            (root_pop)
            (root_pop)
            (root_pop)
            (root_pop)
            (root_pop)
            (root_pop)
            final))))))

(defn collect-native-bundle-offset-depths-x86-step-64 [ir-func function-metas offsets depths current-offset current-depth idx len]
  (collect-native-bundle-offset-depths-x86-step-64-loop-bounded ir-func function-metas offsets depths current-offset current-depth idx len 64))

(defn continue-collect-native-bundle-offset-depths-x86-step-64 [ir-func function-metas len state]
  (if (= (vector-get state 0) 1)
    state
    (do
      (root_push ir-func)
      (root_push function-metas)
      (root_push state)
      (let [next-state
        (collect-native-bundle-offset-depths-x86-step-64
          ir-func
          function-metas
          (vector-get state 2)
          (vector-get state 3)
          (vector-get state 4)
          (vector-get state 5)
          (vector-get state 1)
          len)]
        (do
          (root_push next-state)
          (let [final (continue-collect-native-bundle-offset-depths-x86-step-64 ir-func function-metas len next-state)]
            (do
              (root_pop)
              (root_pop)
              (root_pop)
              (root_pop)
              final)))))))

(defn collect-native-bundle-offset-depths-x86 [ir-func function-metas start-offset]
  (let [len (vector-length ir-func)]
    (continue-collect-native-bundle-offset-depths-x86-step-64
      ir-func
      function-metas
      len
      (collect-native-bundle-offset-depths-x86-step-64
        ir-func
        function-metas
        (vector-new (+ len 1))
        (vector-new (+ len 1))
        start-offset
        0
        0
        len))))

(defn collect-native-bundle-depths-x86 [ir-func function-metas]
  (vector-get (collect-native-bundle-offset-depths-x86 ir-func function-metas 0) 3))

(defn collect-native-offsets-aarch64-loop [ir-func result current-offset idx len]
  (if (>= idx len)
    (vector-push result current-offset)
    (let [instr (vector-get ir-func idx)
      opcode (vector-get instr 0)
      operand (vector-get instr 1)
      next-offset (+ current-offset (native-plain-instr-size-aarch64 opcode operand))]
      (collect-native-offsets-aarch64-loop ir-func (vector-push result current-offset) next-offset (+ idx 1) len))))

(defn make-native-offset-collection-state [done next-idx result next-offset]
  (do
    (root_push result)
    (let [base0 (vector-push (vector-new 4) done)]
      (do
        (root_push base0)
        (let [base1 (vector-push base0 next-idx)]
          (do
            (root_push base1)
            (let [base2 (vector-push base1 result)]
              (do
                (root_push base2)
                (let [state (vector-push base2 next-offset)]
                  (do
                    (root_pop)
                    (root_pop)
                    (root_pop)
                    (root_pop)
                    state))))))))))

(defn collect-native-offsets-aarch64-step [ir-func result current-offset idx len]
  (if (>= idx len)
    (make-native-offset-collection-state 1 idx (vector-push result current-offset) current-offset)
    (let [instr (vector-get ir-func idx)
      opcode (vector-get instr 0)
      operand (vector-get instr 1)
      next-result (vector-push result current-offset)
      next-offset (+ current-offset (native-plain-instr-size-aarch64 opcode operand))]
      (make-native-offset-collection-state 0 (+ idx 1) next-result next-offset))))

(defn collect-native-offsets-aarch64-step-64-loop-bounded [ir-func result current-offset idx len remaining]
  (do
    (root_push ir-func)
    (root_push result)
    (let [state (collect-native-offsets-aarch64-step ir-func result current-offset idx len)
      done (vector-get state 0)
      next-idx (vector-get state 1)
      next-result (vector-get state 2)
      next-offset (vector-get state 3)]
      (do
        (root_push state)
        (root_push next-result)
        (let [final
          (if (= done 1)
            state
            (if (<= remaining 1)
              state
              (collect-native-offsets-aarch64-step-64-loop-bounded ir-func next-result next-offset next-idx len (- remaining 1))))]
          (do
            (root_pop)
            (root_pop)
            (root_pop)
            (root_pop)
            final))))))

(defn collect-native-offsets-aarch64-step-64 [ir-func result current-offset idx len]
  (collect-native-offsets-aarch64-step-64-loop-bounded ir-func result current-offset idx len 64))

(defn continue-collect-native-offsets-aarch64-step-64 [ir-func len state]
  (if (= (vector-get state 0) 1)
    state
    (do
      (root_push ir-func)
      (root_push state)
      (let [next-state (collect-native-offsets-aarch64-step-64 ir-func (vector-get state 2) (vector-get state 3) (vector-get state 1) len)]
        (do
          (root_push next-state)
          (let [final (continue-collect-native-offsets-aarch64-step-64 ir-func len next-state)]
            (do
              (root_pop)
              (root_pop)
              (root_pop)
              final)))))))

(defn collect-native-offsets-aarch64 [ir-func]
  (vector-get
    (continue-collect-native-offsets-aarch64-step-64
      ir-func
      (vector-length ir-func)
      (collect-native-offsets-aarch64-step-64 ir-func (vector-new (+ (vector-length ir-func) 1)) 0 0 (vector-length ir-func)))
    2))

(defn collect-native-bundle-offsets-aarch64-loop [ir-func function-metas result current-offset current-depth idx len]
  (if (>= idx len)
    (vector-push result current-offset)
    (let [instr (vector-get ir-func idx)
      opcode (vector-get instr 0)
      operand (vector-get instr 1)
      next-offset (+ current-offset (native-instr-size-aarch64 opcode operand function-metas current-depth))
      next-depth (apply-stack-delta current-depth (opcode-stack-delta opcode operand function-metas))]
      (collect-native-bundle-offsets-aarch64-loop ir-func function-metas (vector-push result current-offset) next-offset next-depth (+ idx 1) len))))

(defn make-native-bundle-offset-collection-state [done next-idx result next-offset next-depth]
  (do
    (root_push result)
    (let [base0 (vector-push (vector-new 5) done)]
      (do
        (root_push base0)
        (let [base1 (vector-push base0 next-idx)]
          (do
            (root_push base1)
            (let [base2 (vector-push base1 result)]
              (do
                (root_push base2)
                (let [base3 (vector-push base2 next-offset)]
                  (do
                    (root_push base3)
                    (let [state (vector-push base3 next-depth)]
                      (do
                        (root_pop)
                        (root_pop)
                        (root_pop)
                        (root_pop)
                        (root_pop)
                        state))))))))))))

(defn collect-native-bundle-offsets-aarch64-step [ir-func function-metas result current-offset current-depth idx len]
  (if (>= idx len)
    (make-native-bundle-offset-collection-state 1 idx (vector-push result current-offset) current-offset current-depth)
    (let [instr (vector-get ir-func idx)
      opcode (vector-get instr 0)
      operand (vector-get instr 1)
      next-result (vector-push result current-offset)
      next-offset (+ current-offset (native-instr-size-aarch64 opcode operand function-metas current-depth))
      next-depth (apply-stack-delta current-depth (opcode-stack-delta opcode operand function-metas))]
      (make-native-bundle-offset-collection-state 0 (+ idx 1) next-result next-offset next-depth))))

(defn collect-native-bundle-offsets-aarch64-step-64-loop-bounded [ir-func function-metas result current-offset current-depth idx len remaining]
  (do
    (root_push ir-func)
    (root_push function-metas)
    (root_push result)
    (let [state (collect-native-bundle-offsets-aarch64-step ir-func function-metas result current-offset current-depth idx len)
      done (vector-get state 0)
      next-idx (vector-get state 1)
      next-result (vector-get state 2)
      next-offset (vector-get state 3)
      next-depth (vector-get state 4)]
      (do
        (root_push state)
        (root_push next-result)
        (let [final
          (if (= done 1)
            state
            (if (<= remaining 1)
              state
              (collect-native-bundle-offsets-aarch64-step-64-loop-bounded ir-func function-metas next-result next-offset next-depth next-idx len (- remaining 1))))]
          (do
            (root_pop)
            (root_pop)
            (root_pop)
            (root_pop)
            (root_pop)
            final))))))

(defn collect-native-bundle-offsets-aarch64-step-64 [ir-func function-metas result current-offset current-depth idx len]
  (collect-native-bundle-offsets-aarch64-step-64-loop-bounded ir-func function-metas result current-offset current-depth idx len 64))

(defn continue-collect-native-bundle-offsets-aarch64-step-64 [ir-func function-metas len state]
  (if (= (vector-get state 0) 1)
    state
    (do
      (root_push ir-func)
      (root_push function-metas)
      (root_push state)
      (let [next-state (collect-native-bundle-offsets-aarch64-step-64
                         ir-func
                         function-metas
                         (vector-get state 2)
                         (vector-get state 3)
                         (vector-get state 4)
                         (vector-get state 1)
                         len)]
        (do
          (root_push next-state)
          (let [final (continue-collect-native-bundle-offsets-aarch64-step-64 ir-func function-metas len next-state)]
            (do
              (root_pop)
              (root_pop)
              (root_pop)
              (root_pop)
              final)))))))

(defn collect-native-bundle-offsets-aarch64 [ir-func function-metas start-offset]
  (vector-get
    (continue-collect-native-bundle-offsets-aarch64-step-64
      ir-func
      function-metas
      (vector-length ir-func)
      (collect-native-bundle-offsets-aarch64-step-64 ir-func function-metas (vector-new (+ (vector-length ir-func) 1)) start-offset 0 0 (vector-length ir-func)))
    2))

(defn control-end-target-offset [meta offsets start-idx]
  (let [end-idx (map-get-index (control-flow-end-map meta) start-idx)]
    (vector-get offsets (+ end-idx 1))))

(defn control-if-false-target-offset [meta offsets start-idx]
  (let [else-idx (map-get-index (control-flow-else-map meta) start-idx)]
    (if (< else-idx 0)
      (control-end-target-offset meta offsets start-idx)
      (vector-get offsets (+ else-idx 1)))))

(defn control-branch-target-offset [ir-func meta offsets idx]
  (let [target-start (map-get-index (control-flow-branch-map meta) idx)
    target-instr (vector-get ir-func target-start)
    target-opcode (vector-get target-instr 0)]
    (if (= (is-loop-opcode target-opcode) 1)
      (vector-get offsets (+ target-start 1))
      (control-end-target-offset meta offsets target-start))))

(defn emit-control-if-x86 [meta offsets idx]
  (let [current-offset (vector-get offsets idx)
    target-offset (control-if-false-target-offset meta offsets idx)
    disp (- target-offset (+ current-offset 8))]
    (concat-byte-vectors (emit-test-eax-eax) (emit-jz-rel32 disp))))

(defn emit-control-else-x86 [meta offsets idx]
  (let [current-offset (vector-get offsets idx)
    target-offset (control-end-target-offset meta offsets idx)
    disp (- target-offset (+ current-offset 5))]
    (emit-jmp-rel32 disp)))

(defn emit-control-branch-x86 [ir-func meta offsets idx]
  (let [current-offset (vector-get offsets idx)
    target-offset (control-branch-target-offset ir-func meta offsets idx)
    disp (- target-offset (+ current-offset 5))]
    (emit-jmp-rel32 disp)))

(defn emit-control-branch-if-x86 [ir-func meta offsets idx]
  (let [current-offset (vector-get offsets idx)
    target-offset (control-branch-target-offset ir-func meta offsets idx)
    disp (- target-offset (+ current-offset 8))]
    (concat-byte-vectors (emit-test-eax-eax) (emit-jnz-rel32 disp))))

(defn emit-control-instr-x86 [ir-func meta offsets idx]
  (let [instr (vector-get ir-func idx)
    opcode (vector-get instr 0)]
    (if (= opcode 41)
      (emit-control-if-x86 meta offsets idx)
      (if (= opcode 79)
        (emit-control-else-x86 meta offsets idx)
        (if (= opcode 80)
          (emit-control-branch-x86 ir-func meta offsets idx)
          (if (= opcode 81)
            (emit-control-branch-if-x86 ir-func meta offsets idx)
            (if (= opcode 83)
              (emit-control-if-x86 meta offsets idx)
              (vector-new 0))))))))

(defn emit-control-instr-aarch64 [ir-func meta offsets idx]
  (let [instr (vector-get ir-func idx)
    opcode (vector-get instr 0)
    current-offset (vector-get offsets idx)]
    (if (= opcode 41)
      (emit-aarch64-cbz-x0 (- (control-if-false-target-offset meta offsets idx) current-offset))
      (if (= opcode 79)
        (emit-aarch64-b (- (control-end-target-offset meta offsets idx) current-offset))
        (if (= opcode 80)
          (emit-aarch64-b (- (control-branch-target-offset ir-func meta offsets idx) current-offset))
          (if (= opcode 81)
            (emit-aarch64-cbnz-x0 (- (control-branch-target-offset ir-func meta offsets idx) current-offset))
            (if (= opcode 83)
              (emit-aarch64-cbz-x0 (- (control-if-false-target-offset meta offsets idx) current-offset))
              (vector-new 0))))))))

(defn emit-aarch64-conditional-pop-branch [target-offset current-offset frame-base-slot-count current-depth branch-if-nonzero]
  (let [drop-bytes (emit-drop-bundle-aarch64 frame-base-slot-count current-depth)
    branch-offset (+ (+ current-offset 4) (vector-length drop-bytes))
    branch-bytes (if (= branch-if-nonzero 1)
                   (emit-aarch64-b-ne (- target-offset branch-offset))
                   (emit-aarch64-b-eq (- target-offset branch-offset)))]
    (concat-byte-vectors
      (concat-byte-vectors (emit-aarch64-cmp-x0-zero) drop-bytes)
      branch-bytes)))

(defn emit-control-instr-bundle-aarch64 [ir-func meta offsets idx frame-base-slot-count current-depth]
  (let [instr (vector-get ir-func idx)
    opcode (vector-get instr 0)
    current-offset (vector-get offsets idx)]
    (if (= opcode 41)
      (emit-aarch64-conditional-pop-branch
        (control-if-false-target-offset meta offsets idx)
        current-offset
        frame-base-slot-count
        current-depth
        0)
      (if (= opcode 81)
        (emit-aarch64-conditional-pop-branch
          (control-branch-target-offset ir-func meta offsets idx)
          current-offset
          frame-base-slot-count
          current-depth
          1)
        (if (= opcode 83)
          (emit-aarch64-conditional-pop-branch
            (control-if-false-target-offset meta offsets idx)
            current-offset
            frame-base-slot-count
            current-depth
            0)
          (emit-control-instr-aarch64 ir-func meta offsets idx))))))

(defn generate-native-instr-loop-x86 [ir-func result meta offsets idx len]
  (if (>= idx len)
    0
    (do
      (root_push ir-func)
      (root_push result)
      (root_push meta)
      (root_push offsets)
      (let [instr (vector-get ir-func idx)
        opcode (vector-get instr 0)
        operand (vector-get instr 1)
        native (if (= (is-control-opcode opcode) 1)
                 (emit-control-instr-x86 ir-func meta offsets idx)
                 (codegen-ir-instr opcode operand))
        native-len (vector-length native)]
        (do
          (root_push native)
          (append-native-bytes-loop result native 0 native-len)
          (let [final (generate-native-instr-loop-x86 ir-func result meta offsets (+ idx 1) len)]
            (do
              (root_pop)
              (root_pop)
              (root_pop)
              (root_pop)
              (root_pop)
              final)))))))

(defn generate-native-control-instr-loop-aarch64 [ir-func result meta offsets idx len]
  (if (>= idx len)
    0
    (do
      (root_push ir-func)
      (root_push result)
      (root_push meta)
      (root_push offsets)
      (let [instr (vector-get ir-func idx)
        opcode (vector-get instr 0)
        operand (vector-get instr 1)
        native (if (= (is-control-opcode opcode) 1)
                 (emit-control-instr-aarch64 ir-func meta offsets idx)
                 (codegen-ir-instr-aarch64 opcode operand))
        native-len (vector-length native)]
        (do
          (root_push native)
          (append-native-bytes-loop result native 0 native-len)
          (let [final (generate-native-control-instr-loop-aarch64 ir-func result meta offsets (+ idx 1) len)]
            (do
              (root_pop)
              (root_pop)
              (root_pop)
              (root_pop)
              (root_pop)
              final)))))))

(defn make-native-control-bundle-loop-state [done next-idx next-depth]
  (let [base0 (vector-push (vector-new 3) done)]
    (do
      (root_push base0)
      (let [base1 (vector-push base0 next-idx)]
        (do
          (root_push base1)
          (let [state (vector-push base1 next-depth)]
            (do
              (root_pop)
              (root_pop)
              state)))))))

(defn direct-append-aarch64-opcode [opcode current-depth]
  (if (if (= opcode 11) (>= current-depth 1) false)
    3
    (if (= opcode 10)
      4
      (if (>= current-depth 8)
        (if (= opcode 1)
          1
          (if (= opcode 3)
            1
            (if (= opcode 75)
              1
              (if (= opcode 20) 2 0))))
        0))))

(defn direct-append-produce-one-bytes-aarch64 [opcode operand]
  (if (= opcode 1)
    (emit-aarch64-load-i64-x0 operand)
    (if (= opcode 3)
      (emit-aarch64-load-u32-w0 operand)
      (if (= opcode 10)
        (emit-aarch64-ldr-x0-sp (local-slot-offset operand))
        (emit-root-pop-aarch64)))))

(defn generate-native-control-instr-bundle-loop-aarch64-with-import-count-step [ir-func result meta offsets function-starts function-metas import-count import-stub-offset frame-base-slot-count current-depth idx len]
  (if (>= idx len)
    (make-native-control-bundle-loop-state 1 idx current-depth)
    (let [instr (vector-get ir-func idx)
      opcode (vector-get instr 0)
      operand (vector-get instr 1)
      current-offset (vector-get offsets idx)
      next-depth (apply-stack-delta current-depth (opcode-stack-delta opcode operand function-metas))]
      (if (= (direct-append-aarch64-opcode opcode current-depth) 1)
        (do
          (append-produce-one-bundle-aarch64
            result
            (direct-append-produce-one-bytes-aarch64 opcode operand)
            frame-base-slot-count
            current-depth)
          (make-native-control-bundle-loop-state 0 (+ idx 1) next-depth))
        (if (= (direct-append-aarch64-opcode opcode current-depth) 2)
          (do
            (append-consume-two-bundle-aarch64 result (codegen-ir-instr-aarch64 opcode operand) frame-base-slot-count current-depth)
            (make-native-control-bundle-loop-state 0 (+ idx 1) next-depth))
          (if (= (direct-append-aarch64-opcode opcode current-depth) 3)
            (do
              (append-local-set-bundle-aarch64 result (local-slot-offset operand) frame-base-slot-count current-depth)
              (make-native-control-bundle-loop-state 0 (+ idx 1) next-depth))
            (if (= (direct-append-aarch64-opcode opcode current-depth) 4)
              (do
                (append-local-get-bundle-aarch64 result (local-slot-offset operand) frame-base-slot-count current-depth)
                (make-native-control-bundle-loop-state 0 (+ idx 1) next-depth))
               (let [native (if (= (is-control-opcode opcode) 1)
                              (emit-control-instr-bundle-aarch64 ir-func meta offsets idx frame-base-slot-count current-depth)
                              (codegen-ir-instr-bundle-aarch64-with-import-count opcode operand current-offset function-starts function-metas import-count import-stub-offset frame-base-slot-count current-depth))
                 native-len (vector-length native)]
                (do
                  (root_push native)
                  (append-native-bytes-loop result native 0 native-len)
                  (root_pop)
                  (make-native-control-bundle-loop-state 0 (+ idx 1) next-depth))))))))))

(defn generate-native-control-instr-bundle-loop-aarch64-with-import-count-step-64-loop-bounded [ir-func result meta offsets function-starts function-metas import-count import-stub-offset frame-base-slot-count idx len current-depth remaining]
  (do
    (root_push ir-func)
    (root_push result)
    (root_push meta)
    (root_push offsets)
    (root_push function-starts)
    (root_push function-metas)
    (let [state (generate-native-control-instr-bundle-loop-aarch64-with-import-count-step ir-func result meta offsets function-starts function-metas import-count import-stub-offset frame-base-slot-count current-depth idx len)
      done (vector-get state 0)
      next-idx (vector-get state 1)
      next-depth (vector-get state 2)]
      (do
        (root_push state)
        (let [final
          (if (= done 1)
            state
            (if (<= remaining 1)
              state
              (generate-native-control-instr-bundle-loop-aarch64-with-import-count-step-64-loop-bounded ir-func result meta offsets function-starts function-metas import-count import-stub-offset frame-base-slot-count next-idx len next-depth (- remaining 1))))]
          (do
            (root_pop)
            (root_pop)
            (root_pop)
            (root_pop)
            (root_pop)
            (root_pop)
            (root_pop)
            final))))))

(defn generate-native-control-instr-bundle-loop-aarch64-with-import-count-step-64 [ir-func result meta offsets function-starts function-metas import-count import-stub-offset frame-base-slot-count idx len current-depth]
  (generate-native-control-instr-bundle-loop-aarch64-with-import-count-step-64-loop-bounded ir-func result meta offsets function-starts function-metas import-count import-stub-offset frame-base-slot-count idx len current-depth 64))

(defn continue-generate-native-control-instr-bundle-loop-aarch64-with-import-count-step-64 [ir-func result meta offsets function-starts function-metas import-count import-stub-offset frame-base-slot-count len state]
  (if (= (vector-get state 0) 1)
    state
    (do
      (root_push ir-func)
      (root_push result)
      (root_push meta)
      (root_push offsets)
      (root_push function-starts)
      (root_push function-metas)
      (root_push state)
      (let [next-state
        (generate-native-control-instr-bundle-loop-aarch64-with-import-count-step-64
          ir-func
          result
          meta
          offsets
          function-starts
          function-metas
          import-count
          import-stub-offset
          frame-base-slot-count
          (vector-get state 1)
          len
          (vector-get state 2))]
        (do
          (root_push next-state)
          (let [final
            (continue-generate-native-control-instr-bundle-loop-aarch64-with-import-count-step-64 ir-func result meta offsets function-starts function-metas import-count import-stub-offset frame-base-slot-count len next-state)]
            (do
              (root_pop)
              (root_pop)
              (root_pop)
              (root_pop)
              (root_pop)
              (root_pop)
              (root_pop)
              (root_pop)
              final)))))))

(defn generate-native-control-instr-bundle-loop-aarch64-with-import-count [ir-func result meta offsets function-starts function-metas import-count import-stub-offset frame-base-slot-count current-depth idx len]
  (do
    (continue-generate-native-control-instr-bundle-loop-aarch64-with-import-count-step-64
      ir-func
      result
      meta
      offsets
      function-starts
      function-metas
      import-count
      import-stub-offset
      frame-base-slot-count
      len
      (generate-native-control-instr-bundle-loop-aarch64-with-import-count-step-64
        ir-func
        result
        meta
        offsets
        function-starts
        function-metas
        import-count
        import-stub-offset
        frame-base-slot-count
        idx
        len
        current-depth))
    0))

(defn generate-native-control-instr-bundle-loop-aarch64 [ir-func result meta offsets function-starts function-metas frame-base-slot-count current-depth idx len]
  (generate-native-control-instr-bundle-loop-aarch64-with-import-count ir-func result meta offsets function-starts function-metas 0 0 frame-base-slot-count current-depth idx len))

;; === x86_64 コード生成 ===

;; x86_64 IR 関数をネイティブコードに変換 (プロローグ・エピローグ付き)
;; ir-func: IR 命令列の Vector [[opcode, operand], ...]
;; 戻り値: ネイティブ機械語バイト列
(defn generate-native-x86-64 [ir-func]
  (do
    (root_push ir-func)
    (let [result (ref-new (vector-new 64))]
      (do
        (root_push result)
        (let [stack-bytes (native-local-stack-bytes ir-func)
          control-meta (scan-control-flow-meta ir-func)
          offsets (collect-native-offsets-x86 ir-func)
          n (vector-length ir-func)]
          (do
            (root_push control-meta)
            (root_push offsets)
            (let [prologue-push (emit-push-rbp)
              prologue-mov (emit-mov-rbp-rsp)]
              (do
                (root_push prologue-push)
                (root_push prologue-mov)
                (ref-set result (vector-push (ref-get result) (vector-get prologue-push 0)))
                (ref-set result (vector-push (ref-get result) (vector-get prologue-mov 0)))
                (ref-set result (vector-push (ref-get result) (vector-get prologue-mov 1)))
                (ref-set result (vector-push (ref-get result) (vector-get prologue-mov 2)))
                (if (> stack-bytes 0)
                  (append-native-bytes-loop result (emit-sub-rsp-imm32 stack-bytes) 0 7)
                  0)
                (generate-native-instr-loop-x86 ir-func result control-meta offsets 0 n)
                ;; 関数エピローグ
                (let [epilogue-pop (emit-pop-rbp)
                  epilogue-ret (emit-ret)]
                  (do
                    (root_push epilogue-pop)
                    (root_push epilogue-ret)
                    (if (> stack-bytes 0)
                      (append-native-bytes-loop result (emit-add-rsp-imm32 stack-bytes) 0 7)
                      0)
                    (ref-set result (vector-push (ref-get result) (vector-get epilogue-pop 0)))
                    (ref-set result (vector-push (ref-get result) (vector-get epilogue-ret 0)))
                    (let [final (ref-get result)]
                      (do
                        (root_pop)
                        (root_pop)
                        (root_pop)
                        (root_pop)
                        (root_pop)
                        (root_pop)
                        (root_pop)
                        (root_pop)
                        final))))))))))))

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

(defn spill-native-function-params-x86-twenty-to-twenty-nine [param-count result]
  (if (= param-count 29)
    (do
      (spill-native-function-params-x86-twenty-to-twenty-eight 28 result)
      (append-native-bytes-loop result (emit-mov-rax-from-rbp-plus-imm32 192) 0 7)
      (append-native-bytes-loop result (emit-mov-local-from-rax (local-slot-offset 28)) 0 7))
    (spill-native-function-params-x86-twenty-to-twenty-eight param-count result)))

(defn spill-native-function-params-x86-twenty-to-thirty [param-count result]
  (if (= param-count 30)
    (do
      (spill-native-function-params-x86-twenty-to-twenty-nine 29 result)
      (append-native-bytes-loop result (emit-mov-rax-from-rbp-plus-imm32 200) 0 7)
      (append-native-bytes-loop result (emit-mov-local-from-rax (local-slot-offset 29)) 0 7))
    (spill-native-function-params-x86-twenty-to-twenty-nine param-count result)))

(defn spill-native-function-params-x86-twenty-to-thirty-one [param-count result]
  (if (= param-count 31)
    (do
      (spill-native-function-params-x86-twenty-to-thirty 30 result)
      (append-native-bytes-loop result (emit-mov-rax-from-rbp-plus-imm32 208) 0 7)
      (append-native-bytes-loop result (emit-mov-local-from-rax (local-slot-offset 30)) 0 7))
    (spill-native-function-params-x86-twenty-to-thirty param-count result)))

(defn spill-native-function-params-x86-twenty-to-thirty-two [param-count result]
  (if (= param-count 32)
    (do
      (spill-native-function-params-x86-twenty-to-thirty-one 31 result)
      (append-native-bytes-loop result (emit-mov-rax-from-rbp-plus-imm32 216) 0 7)
      (append-native-bytes-loop result (emit-mov-local-from-rax (local-slot-offset 31)) 0 7))
    (spill-native-function-params-x86-twenty-to-thirty-one param-count result)))

(defn spill-native-function-params-x86-twenty-to-thirty-three [param-count result]
  (if (= param-count 33)
    (do
      (spill-native-function-params-x86-twenty-to-thirty-two 32 result)
      (append-native-bytes-loop result (emit-mov-rax-from-rbp-plus-imm32 224) 0 7)
      (append-native-bytes-loop result (emit-mov-local-from-rax (local-slot-offset 32)) 0 7))
    (spill-native-function-params-x86-twenty-to-thirty-two param-count result)))

(defn spill-native-function-params-x86-twenty-to-thirty-four [param-count result]
  (if (= param-count 34)
    (do
      (spill-native-function-params-x86-twenty-to-thirty-three 33 result)
      (append-native-bytes-loop result (emit-mov-rax-from-rbp-plus-imm32 232) 0 7)
      (append-native-bytes-loop result (emit-mov-local-from-rax (local-slot-offset 33)) 0 7))
    (spill-native-function-params-x86-twenty-to-thirty-three param-count result)))

(defn spill-native-function-params-x86-twenty-to-thirty-five [param-count result]
  (if (= param-count 35)
    (do
      (spill-native-function-params-x86-twenty-to-thirty-four 34 result)
      (append-native-bytes-loop result (emit-mov-rax-from-rbp-plus-imm32 240) 0 7)
      (append-native-bytes-loop result (emit-mov-local-from-rax (local-slot-offset 34)) 0 7))
    (spill-native-function-params-x86-twenty-to-thirty-four param-count result)))

(defn spill-native-function-params-x86-twenty-to-thirty-six [param-count result]
  (if (= param-count 36)
    (do
      (spill-native-function-params-x86-twenty-to-thirty-five 35 result)
      (append-native-bytes-loop result (emit-mov-rax-from-rbp-plus-imm32 248) 0 7)
      (append-native-bytes-loop result (emit-mov-local-from-rax (local-slot-offset 35)) 0 7))
    (spill-native-function-params-x86-twenty-to-thirty-five param-count result)))

(defn spill-native-function-params-x86-twenty-to-thirty-seven [param-count result]
  (if (= param-count 37)
    (do
      (spill-native-function-params-x86-twenty-to-thirty-six 36 result)
      (append-native-bytes-loop result (emit-mov-rax-from-rbp-plus-imm32 256) 0 7)
      (append-native-bytes-loop result (emit-mov-local-from-rax (local-slot-offset 36)) 0 7))
    (spill-native-function-params-x86-twenty-to-thirty-six param-count result)))

(defn spill-native-function-params-x86-twenty-to-thirty-eight [param-count result]
  (if (= param-count 38)
    (do
      (spill-native-function-params-x86-twenty-to-thirty-seven 37 result)
      (append-native-bytes-loop result (emit-mov-rax-from-rbp-plus-imm32 264) 0 7)
      (append-native-bytes-loop result (emit-mov-local-from-rax (local-slot-offset 37)) 0 7))
    (spill-native-function-params-x86-twenty-to-thirty-seven param-count result)))

(defn spill-native-function-params-x86-twenty-to-thirty-nine [param-count result]
  (if (= param-count 39)
    (do
      (spill-native-function-params-x86-twenty-to-thirty-eight 38 result)
      (append-native-bytes-loop result (emit-mov-rax-from-rbp-plus-imm32 272) 0 7)
      (append-native-bytes-loop result (emit-mov-local-from-rax (local-slot-offset 38)) 0 7))
    (spill-native-function-params-x86-twenty-to-thirty-eight param-count result)))

(defn spill-native-function-params-x86-twenty-to-forty [param-count result]
  (if (= param-count 40)
    (do
      (spill-native-function-params-x86-twenty-to-thirty-nine 39 result)
      (append-native-bytes-loop result (emit-mov-rax-from-rbp-plus-imm32 280) 0 7)
      (append-native-bytes-loop result (emit-mov-local-from-rax (local-slot-offset 39)) 0 7))
    (spill-native-function-params-x86-twenty-to-thirty-nine param-count result)))

(defn spill-native-function-params-x86-twenty-to-forty-one [param-count result]
  (if (= param-count 41)
    (do
      (spill-native-function-params-x86-twenty-to-forty 40 result)
      (append-native-bytes-loop result (emit-mov-rax-from-rbp-plus-imm32 288) 0 7)
      (append-native-bytes-loop result (emit-mov-local-from-rax (local-slot-offset 40)) 0 7))
    (spill-native-function-params-x86-twenty-to-forty param-count result)))

(defn spill-native-function-params-x86-twenty-to-forty-two [param-count result]
  (if (= param-count 42)
    (do
      (spill-native-function-params-x86-twenty-to-forty-one 41 result)
      (append-native-bytes-loop result (emit-mov-rax-from-rbp-plus-imm32 296) 0 7)
      (append-native-bytes-loop result (emit-mov-local-from-rax (local-slot-offset 41)) 0 7))
    (spill-native-function-params-x86-twenty-to-forty-one param-count result)))

(defn spill-native-function-params-x86-twenty-to-forty-three [param-count result]
  (if (= param-count 43)
    (do
      (spill-native-function-params-x86-twenty-to-forty-two 42 result)
      (append-native-bytes-loop result (emit-mov-rax-from-rbp-plus-imm32 304) 0 7)
      (append-native-bytes-loop result (emit-mov-local-from-rax (local-slot-offset 42)) 0 7))
    (spill-native-function-params-x86-twenty-to-forty-two param-count result)))

(defn spill-native-function-params-x86-twenty-to-forty-four [param-count result]
  (if (= param-count 44)
    (do
      (spill-native-function-params-x86-twenty-to-forty-three 43 result)
      (append-native-bytes-loop result (emit-mov-rax-from-rbp-plus-imm32 312) 0 7)
      (append-native-bytes-loop result (emit-mov-local-from-rax (local-slot-offset 43)) 0 7))
    (spill-native-function-params-x86-twenty-to-forty-three param-count result)))

(defn spill-native-function-params-x86-twenty-to-forty-five [param-count result]
  (if (= param-count 45)
    (do
      (spill-native-function-params-x86-twenty-to-forty-four 44 result)
      (append-native-bytes-loop result (emit-mov-rax-from-rbp-plus-imm32 320) 0 7)
      (append-native-bytes-loop result (emit-mov-local-from-rax (local-slot-offset 44)) 0 7))
    (spill-native-function-params-x86-twenty-to-forty-four param-count result)))

(defn spill-native-function-params-x86-twenty-to-forty-six [param-count result]
  (if (= param-count 46)
    (do
      (spill-native-function-params-x86-twenty-to-forty-five 45 result)
      (append-native-bytes-loop result (emit-mov-rax-from-rbp-plus-imm32 328) 0 7)
      (append-native-bytes-loop result (emit-mov-local-from-rax (local-slot-offset 45)) 0 7))
    (spill-native-function-params-x86-twenty-to-forty-five param-count result)))

(defn spill-native-function-params-x86-twenty-to-forty-seven [param-count result]
  (if (= param-count 47)
    (do
      (spill-native-function-params-x86-twenty-to-forty-six 46 result)
      (append-native-bytes-loop result (emit-mov-rax-from-rbp-plus-imm32 336) 0 7)
      (append-native-bytes-loop result (emit-mov-local-from-rax (local-slot-offset 46)) 0 7))
    (spill-native-function-params-x86-twenty-to-forty-six param-count result)))

(defn spill-native-function-params-x86-twenty-to-forty-eight [param-count result]
  (if (= param-count 48)
    (do
      (spill-native-function-params-x86-twenty-to-forty-seven 47 result)
      (append-native-bytes-loop result (emit-mov-rax-from-rbp-plus-imm32 344) 0 7)
      (append-native-bytes-loop result (emit-mov-local-from-rax (local-slot-offset 47)) 0 7))
    (spill-native-function-params-x86-twenty-to-forty-seven param-count result)))

(defn spill-native-function-params-x86-twenty-to-forty-nine [param-count result]
  (if (= param-count 49)
    (do
      (spill-native-function-params-x86-twenty-to-forty-eight 48 result)
      (append-native-bytes-loop result (emit-mov-rax-from-rbp-plus-imm32 352) 0 7)
      (append-native-bytes-loop result (emit-mov-local-from-rax (local-slot-offset 48)) 0 7))
    (spill-native-function-params-x86-twenty-to-forty-eight param-count result)))

(defn spill-native-function-params-x86-twenty-to-fifty [param-count result]
  (if (= param-count 50)
    (do
      (spill-native-function-params-x86-twenty-to-forty-nine 49 result)
      (append-native-bytes-loop result (emit-mov-rax-from-rbp-plus-imm32 360) 0 7)
      (append-native-bytes-loop result (emit-mov-local-from-rax (local-slot-offset 49)) 0 7))
    (spill-native-function-params-x86-twenty-to-forty-nine param-count result)))

(defn spill-native-function-params-x86-twenty-to-fifty-one [param-count result]
  (if (= param-count 51)
    (do
      (spill-native-function-params-x86-twenty-to-fifty 50 result)
      (append-native-bytes-loop result (emit-mov-rax-from-rbp-plus-imm32 368) 0 7)
      (append-native-bytes-loop result (emit-mov-local-from-rax (local-slot-offset 50)) 0 7))
    (spill-native-function-params-x86-twenty-to-fifty param-count result)))

(defn spill-native-function-params-x86-twenty-to-fifty-two [param-count result]
  (if (= param-count 52)
    (do
      (spill-native-function-params-x86-twenty-to-fifty-one 51 result)
      (append-native-bytes-loop result (emit-mov-rax-from-rbp-plus-imm32 376) 0 7)
      (append-native-bytes-loop result (emit-mov-local-from-rax (local-slot-offset 51)) 0 7))
    (spill-native-function-params-x86-twenty-to-fifty-one param-count result)))

(defn spill-native-function-params-x86-twenty-to-fifty-three [param-count result]
  (if (= param-count 53)
    (do
      (spill-native-function-params-x86-twenty-to-fifty-two 52 result)
      (append-native-bytes-loop result (emit-mov-rax-from-rbp-plus-imm32 384) 0 7)
      (append-native-bytes-loop result (emit-mov-local-from-rax (local-slot-offset 52)) 0 7))
    (spill-native-function-params-x86-twenty-to-fifty-two param-count result)))

(defn spill-native-function-params-x86-twenty-to-fifty-four [param-count result]
  (if (= param-count 54)
    (do
      (spill-native-function-params-x86-twenty-to-fifty-three 53 result)
      (append-native-bytes-loop result (emit-mov-rax-from-rbp-plus-imm32 392) 0 7)
      (append-native-bytes-loop result (emit-mov-local-from-rax (local-slot-offset 53)) 0 7))
    (spill-native-function-params-x86-twenty-to-fifty-three param-count result)))

(defn spill-native-function-params-x86-twenty-to-fifty-five [param-count result]
  (if (= param-count 55)
    (do
      (spill-native-function-params-x86-twenty-to-fifty-four 54 result)
      (append-native-bytes-loop result (emit-mov-rax-from-rbp-plus-imm32 400) 0 7)
      (append-native-bytes-loop result (emit-mov-local-from-rax (local-slot-offset 54)) 0 7))
    (spill-native-function-params-x86-twenty-to-fifty-four param-count result)))

(defn spill-native-function-params-x86-twenty-to-fifty-six [param-count result]
  (if (= param-count 56)
    (do
      (spill-native-function-params-x86-twenty-to-fifty-five 55 result)
      (append-native-bytes-loop result (emit-mov-rax-from-rbp-plus-imm32 408) 0 7)
      (append-native-bytes-loop result (emit-mov-local-from-rax (local-slot-offset 55)) 0 7))
    (spill-native-function-params-x86-twenty-to-fifty-five param-count result)))

(defn spill-native-function-params-x86-twenty-to-fifty-seven [param-count result]
  (if (= param-count 57)
    (do
      (spill-native-function-params-x86-twenty-to-fifty-six 56 result)
      (append-native-bytes-loop result (emit-mov-rax-from-rbp-plus-imm32 416) 0 7)
      (append-native-bytes-loop result (emit-mov-local-from-rax (local-slot-offset 56)) 0 7))
    (spill-native-function-params-x86-twenty-to-fifty-six param-count result)))

(defn spill-native-function-params-x86-twenty-to-fifty-eight [param-count result]
  (if (= param-count 58)
    (do
      (spill-native-function-params-x86-twenty-to-fifty-seven 57 result)
      (append-native-bytes-loop result (emit-mov-rax-from-rbp-plus-imm32 424) 0 7)
      (append-native-bytes-loop result (emit-mov-local-from-rax (local-slot-offset 57)) 0 7))
    (spill-native-function-params-x86-twenty-to-fifty-seven param-count result)))

(defn spill-native-function-params-x86-twenty-to-sixty [param-count result]
  (if (= param-count 60)
    (do
      (spill-native-function-params-x86-twenty-to-sixty 59 result)
      (append-native-bytes-loop result (emit-mov-rax-from-rbp-plus-imm32 440) 0 7)
      (append-native-bytes-loop result (emit-mov-local-from-rax (local-slot-offset 59)) 0 7))
    (if (= param-count 59)
      (do
        (spill-native-function-params-x86-twenty-to-fifty-eight 58 result)
        (append-native-bytes-loop result (emit-mov-rax-from-rbp-plus-imm32 432) 0 7)
        (append-native-bytes-loop result (emit-mov-local-from-rax (local-slot-offset 58)) 0 7))
      (spill-native-function-params-x86-twenty-to-fifty-eight param-count result))))

(defn spill-native-function-stack-params-x86-loop [param-index param-count rbp-offset result]
  (if (>= param-index param-count)
    0
    (let [load-bytes (if (< rbp-offset 128)
                       (emit-mov-rax-from-rbp-plus-imm8 rbp-offset)
                       (emit-mov-rax-from-rbp-plus-imm32 rbp-offset))
      load-len (if (< rbp-offset 128) 4 7)]
      (do
        (append-native-bytes-loop result load-bytes 0 load-len)
        (append-native-bytes-loop result (emit-mov-local-from-rax (native-param-slot-offset-x86 param-index)) 0 7)
        (spill-native-function-stack-params-x86-loop (+ param-index 1) param-count (+ rbp-offset 8) result)))))

(defn spill-native-function-param-x86 [param-index result]
  (if (= param-index 0)
    (append-native-bytes-loop result (emit-mov-local-from-rdi (native-param-slot-offset-x86 0)) 0 7)
    (if (= param-index 1)
      (append-native-bytes-loop result (emit-mov-local-from-rsi (native-param-slot-offset-x86 1)) 0 7)
      (if (= param-index 2)
        (append-native-bytes-loop result (emit-mov-local-from-rdx (native-param-slot-offset-x86 2)) 0 7)
        (if (= param-index 3)
          (append-native-bytes-loop result (emit-mov-local-from-rcx (native-param-slot-offset-x86 3)) 0 7)
          (if (= param-index 4)
            (append-native-bytes-loop result (emit-mov-local-from-r8 (native-param-slot-offset-x86 4)) 0 7)
            (if (= param-index 5)
              (append-native-bytes-loop result (emit-mov-local-from-r9 (native-param-slot-offset-x86 5)) 0 7)
              (let [rbp-offset (+ 16 (* (- param-index 6) 8))
                load-bytes (if (< rbp-offset 128)
                            (emit-mov-rax-from-rbp-plus-imm8 rbp-offset)
                            (emit-mov-rax-from-rbp-plus-imm32 rbp-offset))
                load-len (if (< rbp-offset 128) 4 7)]
                (do
                  (append-native-bytes-loop result load-bytes 0 load-len)
                  (append-native-bytes-loop result (emit-mov-local-from-rax (native-param-slot-offset-x86 param-index)) 0 7))))))))))

(defn spill-native-function-params-x86-loop [param-index param-count result]
  (if (>= param-index param-count)
    0
    (do
      (spill-native-function-param-x86 param-index result)
      (spill-native-function-params-x86-loop (+ param-index 1) param-count result))))

(defn spill-native-function-params-x86-twenty-plus [param-count result]
  (do
    (append-native-bytes-loop result (emit-mov-local-from-rdi (local-slot-offset 0)) 0 7)
    (append-native-bytes-loop result (emit-mov-local-from-rsi (local-slot-offset 1)) 0 7)
    (append-native-bytes-loop result (emit-mov-local-from-rdx (local-slot-offset 2)) 0 7)
    (append-native-bytes-loop result (emit-mov-local-from-rcx (local-slot-offset 3)) 0 7)
    (append-native-bytes-loop result (emit-mov-local-from-r8 (local-slot-offset 4)) 0 7)
    (append-native-bytes-loop result (emit-mov-local-from-r9 (local-slot-offset 5)) 0 7)
    (spill-native-function-stack-params-x86-loop 6 param-count 16 result)))

(defn spill-native-function-params-x86-twenty-to-sixty-one [param-count result]
  (if (> param-count 60)
    (spill-native-function-params-x86-twenty-plus param-count result)
    (spill-native-function-params-x86-twenty-to-sixty param-count result)))

(defn make-x86-function-emit-layout [import-count import-stub-offset function-start-base emit-start-base]
  (vector-push
    (vector-push
      (vector-push
        (vector-push (vector-new 4) import-count)
        import-stub-offset)
      function-start-base)
    emit-start-base))

(defn x86-function-emit-layout-import-count [layout]
  (vector-get layout 0))

(defn x86-function-emit-layout-import-stub-offset [layout]
  (vector-get layout 1))

(defn x86-function-emit-layout-start-base [layout]
  (vector-get layout 2))

(defn x86-function-emit-layout-emit-start-base [layout]
  (vector-get layout 3))

(defn generate-native-control-instr-bundle-row-step-x86 [ctx idx]
  (do
    (root_push ctx)
    (let [row-state (make-x86-control-loop-state idx 1)]
      (do
        (root_push row-state)
        (generate-native-control-instr-bundle-loop-x86-with-context ctx row-state)
        (root_pop)
        (root_pop)
        0))))

(defn generate-native-control-instr-bundle-row-chunk-x86 [ctx idx end]
  (if (>= idx end)
    0
    (let [_row (generate-native-control-instr-bundle-row-step-x86 ctx idx)]
      (generate-native-control-instr-bundle-row-chunk-x86 ctx (+ idx 1) end))))

(defn generate-native-control-instr-bundle-row-loop-x86 [ctx idx len]
  (if (>= idx len)
    0
    (let [chunk-end (if (> (+ idx 64) len) len (+ idx 64))
      _chunk (generate-native-control-instr-bundle-row-chunk-x86 ctx idx chunk-end)]
      (generate-native-control-instr-bundle-row-loop-x86 ctx chunk-end len))))

(defn generate-native-function-x86-64-bundle-with-layout [func-meta result function-starts function-metas layout]
  (let [import-count (vector-get layout 0)
    import-stub-offset (vector-get layout 1)
    function-start-base (vector-get layout 2)
    emit-start-base (vector-get layout 3)
    param-count (native-function-param-count func-meta)
    local-count (native-function-local-count func-meta)
    ir-func (native-function-ir func-meta)
    frame-base-slot-count (native-frame-base-slot-count ir-func (+ (+ param-count local-count) 1))
    stack-bytes (native-local-stack-bytes-with-window-x86 ir-func (+ (+ param-count local-count) 1) function-metas)
    function-start 0
    encoded-import-stub-offset (- import-stub-offset function-start-base)
    encoded-function-start-base function-start-base
    prologue-push (emit-push-rbp)
    prologue-mov (emit-mov-rbp-rsp)
    base-offset (+ function-start 4)
    after-stack-offset (if (> stack-bytes 0) (+ base-offset 7) base-offset)
    param-spill-bytes (if (>= param-count 20)
                        (native-param-spill-bytes-x86-twenty-plus param-count)
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
      (root_push func-meta)
      (root_push result)
      (root_push function-starts)
      (root_push function-metas)
      (root_push layout)
      (root_push ir-func)
      (ref-set result (vector-push (ref-get result) (vector-get prologue-push 0)))
      (ref-set result (vector-push (ref-get result) (vector-get prologue-mov 0)))
      (ref-set result (vector-push (ref-get result) (vector-get prologue-mov 1)))
      (ref-set result (vector-push (ref-get result) (vector-get prologue-mov 2)))
      (if (> stack-bytes 0)
        (append-native-bytes-loop result (emit-sub-rsp-imm32 stack-bytes) 0 7)
        0)
      (spill-native-function-params-x86-loop 0 param-count result)
      (let [control-meta (scan-control-flow-meta ir-func)]
        (do
          (root_push control-meta)
          (let [offsets (collect-native-bundle-offsets-x86 ir-func function-metas body-offset)]
            (do
              (root_push offsets)
              (let [depths (collect-native-bundle-depths-x86 ir-func function-metas)]
                (do
                  (root_push depths)
                  (let [control-layout (make-x86-function-emit-layout import-count encoded-import-stub-offset encoded-function-start-base emit-start-base)]
                    (do
                      (root_push control-layout)
                      (let [control-ctx (make-x86-control-bundle-context ir-func result control-meta offsets depths function-starts function-metas control-layout frame-base-slot-count)]
                        (do
                          (root_push control-ctx)
                          (generate-native-control-instr-bundle-row-loop-x86 control-ctx 0 n)
                          (root_pop)
                          (root_pop)
                          (root_pop)
                          (root_pop)
                          (root_pop)))))))))
      (if (> stack-bytes 0)
        (append-native-bytes-loop result (emit-add-rsp-imm32 stack-bytes) 0 7)
        0)
      (let [epilogue-pop (emit-pop-rbp)
        epilogue-ret (emit-ret)]
        (do
          (ref-set result (vector-push (ref-get result) (vector-get epilogue-pop 0)))
          (ref-set result (vector-push (ref-get result) (vector-get epilogue-ret 0)))
          (root_pop)
          (root_pop)
          (root_pop)
          (root_pop)
          (root_pop)
          (root_pop)
            0)))))))

(defn generate-native-function-x86-64-bundle-with-import-count-and-base [func-meta result function-starts function-metas import-count import-stub-offset function-start-base]
  (generate-native-function-x86-64-bundle-with-layout
    func-meta
    result
    function-starts
    function-metas
    (make-x86-function-emit-layout import-count import-stub-offset function-start-base (vector-length (ref-get result)))))

(defn generate-native-function-x86-64-bundle-with-import-count [func-meta result function-starts function-metas import-count import-stub-offset]
  (generate-native-function-x86-64-bundle-with-import-count-and-base
    func-meta
    result
    function-starts
    function-metas
    import-count
    import-stub-offset
    (vector-length (ref-get result))))

(defn generate-native-function-x86-64-bundle [func-meta result function-starts function-metas function-start]
  (generate-native-function-x86-64-bundle-with-import-count func-meta result function-starts function-metas 0 0))

(defn make-native-x86-bundle-loop-state [done next-idx]
  (vector-push (vector-push (vector-new 2) done) next-idx))

(defn generate-native-x86-64-bundle-loop-with-import-count-step [functions result function-starts import-count import-stub-offset idx len]
  (if (>= idx len)
    (make-native-x86-bundle-loop-state 1 idx)
    (do
      (root_push functions)
      (root_push result)
      (root_push function-starts)
      (let [actual-idx (+ idx import-count)
        func-meta (vector-get functions actual-idx)
        layout (make-x86-function-emit-layout import-count import-stub-offset (vector-get function-starts idx) (vector-length (ref-get result)))]
        (do
          (root_push func-meta)
          (root_push layout)
          (generate-native-function-x86-64-bundle-with-layout
            func-meta
            result
            function-starts
            functions
            layout)
          (root_pop)
          (root_pop)
          (root_pop)
          (root_pop)
          (root_pop)
          (make-native-x86-bundle-loop-state 0 (+ idx 1)))))))

(defn generate-native-x86-64-bundle-loop-with-import-count-step-64-loop-bounded [functions result function-starts import-count import-stub-offset idx len remaining]
  (do
    (root_push functions)
    (root_push result)
    (root_push function-starts)
    (let [state (generate-native-x86-64-bundle-loop-with-import-count-step functions result function-starts import-count import-stub-offset idx len)
      done (vector-get state 0)
      next-idx (vector-get state 1)]
      (do
        (root_push state)
        (let [final
          (if (= done 1)
            state
            (if (<= remaining 1)
              state
              (generate-native-x86-64-bundle-loop-with-import-count-step-64-loop-bounded functions result function-starts import-count import-stub-offset next-idx len (- remaining 1))))]
          (do
            (root_pop)
            (root_pop)
            (root_pop)
            (root_pop)
            final))))))

(defn generate-native-x86-64-bundle-loop-with-import-count-step-64 [functions result function-starts import-count import-stub-offset idx len]
  (generate-native-x86-64-bundle-loop-with-import-count-step-64-loop-bounded functions result function-starts import-count import-stub-offset idx len 64))

(defn continue-generate-native-x86-64-bundle-loop-with-import-count-step-64 [functions result function-starts import-count import-stub-offset len state]
  (if (= (vector-get state 0) 1)
    state
    (do
      (root_push functions)
      (root_push result)
      (root_push function-starts)
      (root_push state)
      (let [next-state (generate-native-x86-64-bundle-loop-with-import-count-step-64 functions result function-starts import-count import-stub-offset (vector-get state 1) len)]
        (do
          (root_push next-state)
          (let [final (continue-generate-native-x86-64-bundle-loop-with-import-count-step-64 functions result function-starts import-count import-stub-offset len next-state)]
            (do
              (root_pop)
              (root_pop)
              (root_pop)
              (root_pop)
              (root_pop)
              final)))))))

(defn generate-native-x86-64-bundle-loop-with-import-count [functions result function-starts import-count import-stub-offset idx len]
  (continue-generate-native-x86-64-bundle-loop-with-import-count-step-64
    functions
    result
    function-starts
    import-count
    import-stub-offset
    len
    (generate-native-x86-64-bundle-loop-with-import-count-step-64 functions result function-starts import-count import-stub-offset idx len)))

(defn generate-native-x86-64-bundle-with-import-count [functions import-count]
  (let [function-starts (collect-callable-function-starts-x86 functions import-count)
    import-stub-offset (callable-user-total-size-x86 functions import-count)
    result (ref-new (vector-new (x86-bundle-initial-capacity import-stub-offset import-count)))
    n (- (vector-length functions) import-count)]
    (do
      (root_push functions)
      (root_push function-starts)
      (root_push result)
      (let [final
        (do
          (generate-native-x86-64-bundle-loop-with-import-count functions result function-starts import-count import-stub-offset 0 n)
          (if (> import-count 0)
            (append-native-bytes-loop result (emit-ret) 0 1)
            0)
          (append-native-bytes-rooted result (emit-x86-selfhost-command-line-arg-helper) 18)
          (append-x86-selfhost-read-file-helper-rooted result)
          (append-native-bytes-rooted result (emit-x86-selfhost-string-length-helper) 52)
          (append-native-bytes-rooted result (emit-x86-selfhost-string-char-at-helper) 71)
          (append-native-bytes-rooted result (emit-x86-selfhost-print-helper) 102)
          (append-native-bytes-rooted result (emit-x86-selfhost-vector-new-helper) 119)
          (append-native-bytes-rooted result (emit-x86-selfhost-vector-length-helper) 17)
          (append-native-bytes-rooted result (emit-x86-selfhost-vector-get-helper) 47)
          (append-native-bytes-rooted result (emit-x86-selfhost-vector-push-helper) 205)
          (append-native-bytes-rooted result (emit-x86-selfhost-ref-new-helper) 73)
          (append-native-bytes-rooted result (emit-x86-selfhost-ref-get-helper) 18)
          (append-native-bytes-rooted result (emit-x86-selfhost-ref-set-helper) 20)
          (append-native-bytes-rooted result (emit-x86-selfhost-substring-helper) 145)
          (append-native-bytes-rooted result (emit-x86-selfhost-string-concat-helper) 195)
          (append-native-bytes-rooted result (emit-x86-selfhost-map-new-helper) 72)
          (append-native-bytes-rooted result (emit-x86-selfhost-map-size-helper) 17)
          (append-native-bytes-rooted result (emit-x86-selfhost-map-insert-helper) 61)
          (append-native-bytes-rooted result (emit-x86-selfhost-map-get-helper) 62)
          (append-native-bytes-rooted result (emit-x86-selfhost-file-exists-helper) 84)
          (ref-get result))]
        (do
          (root_pop)
          (root_pop)
          (root_pop)
          final)))))

(defn generate-native-x86-64-bundle [functions]
  (generate-native-x86-64-bundle-with-import-count functions 0))

;; === AArch64 命令エンコーダ ===

;; AArch64 MOVZ W0, #imm 命令を生成 (imm は 0-65535)
;; エンコーディング: 0x52800000 | (imm << 5) → LE バイト列 4 bytes
;; 例: MOVZ W0, #42 = 0x52800540 → [0x40, 0x05, 0x80, 0x52]
(defn emit-aarch64-move-wide [base imm hw]
  (encode-u32-le (+ (+ base (* hw 2097152)) (* imm 32))))

(defn emit-aarch64-movz-w0-shift [imm hw]
  (emit-aarch64-move-wide 1384120320 imm hw))

(defn emit-aarch64-movk-w0-shift [imm hw]
  (emit-aarch64-move-wide 1920991232 imm hw))

(defn emit-aarch64-movz-x0-shift [imm hw]
  (emit-aarch64-move-wide 3531603968 imm hw))

(defn emit-aarch64-movk-x0-shift [imm hw]
  (emit-aarch64-move-wide 4068474880 imm hw))

(defn emit-aarch64-movz-w0 [imm]
  (emit-aarch64-movz-w0-shift imm 0))

(defn normalize-u32-immediate [value]
  (if (< value 0)
    (+ 4294967296 value)
    value))

(defn aarch64-immediate-chunk-0 [value]
  (if (< value 4294967296)
    (let [bytes (encode-u32-le (normalize-u32-immediate value))]
      (+ (vector-get bytes 0) (* (vector-get bytes 1) 256)))
    (% value 65536)))

(defn aarch64-immediate-chunk-1 [value]
  (if (< value 4294967296)
    (let [bytes (encode-u32-le (normalize-u32-immediate value))]
      (+ (vector-get bytes 2) (* (vector-get bytes 3) 256)))
    (% (/ value 65536) 65536)))

(defn aarch64-immediate-chunk-2 [value]
  (if (< value 0)
    65535
    (if (< value 4294967296)
      0
      (% (/ value 4294967296) 65536))))

(defn aarch64-immediate-chunk-3 [value]
  (if (< value 0)
    65535
    (if (< value 4294967296)
      0
      (% (/ value 281474976710656) 65536))))

(defn aarch64-load-u32-w0-size [value]
  (let [uvalue (normalize-u32-immediate value)]
    (if (> (aarch64-immediate-chunk-1 uvalue) 0) 8 4)))

(defn aarch64-load-i64-x0-size [value]
  (if (> (aarch64-immediate-chunk-3 value) 0)
    16
    (if (> (aarch64-immediate-chunk-2 value) 0)
      12
      (if (> (aarch64-immediate-chunk-1 value) 0) 8 4))))

(defn native-produce-one-size-aarch64 [op-size current-depth]
  (if (>= current-depth 2)
    (+ op-size (* 8 (- current-depth 1)))
    (if (= current-depth 1) (+ op-size 4) op-size)))

(defn native-produce-one-prefix-size-aarch64 [op-size current-depth]
  (- (native-produce-one-size-aarch64 op-size current-depth) op-size))

(defn emit-aarch64-load-u32-w0 [value]
  (let [uvalue (normalize-u32-immediate value)
    part0 (emit-aarch64-movz-w0-shift (aarch64-immediate-chunk-0 uvalue) 0)
    chunk1 (aarch64-immediate-chunk-1 uvalue)]
    (if (> chunk1 0)
      (concat-byte-vectors-rooted part0 (emit-aarch64-movk-w0-shift chunk1 1))
      part0)))

(defn emit-aarch64-load-i64-x0 [value]
  (let [chunk0 (aarch64-immediate-chunk-0 value)
    chunk1 (aarch64-immediate-chunk-1 value)
    chunk2 (aarch64-immediate-chunk-2 value)
    chunk3 (aarch64-immediate-chunk-3 value)
    part0 (emit-aarch64-movz-x0-shift chunk0 0)]
    (if (> chunk3 0)
      (let [part1 (concat-byte-vectors-rooted part0 (emit-aarch64-movk-x0-shift chunk1 1))
        part2 (concat-byte-vectors-rooted part1 (emit-aarch64-movk-x0-shift chunk2 2))]
        (concat-byte-vectors-rooted part2 (emit-aarch64-movk-x0-shift chunk3 3)))
      (if (> chunk2 0)
        (let [part1 (concat-byte-vectors-rooted part0 (emit-aarch64-movk-x0-shift chunk1 1))]
          (concat-byte-vectors-rooted part1 (emit-aarch64-movk-x0-shift chunk2 2)))
        (if (> chunk1 0)
          (concat-byte-vectors-rooted part0 (emit-aarch64-movk-x0-shift chunk1 1))
          part0)))))

;; AArch64 RET 命令 (X30 経由リターン)
;; エンコーディング: 0xD65F03C0 → [0xC0, 0x03, 0x5F, 0xD6]
(defn emit-aarch64-ret []
  (byte-vector-4 192 3 95 214))

(defn emit-aarch64-selfhost-command-line-arg-helper []
  (concat-byte-vectors
    (concat-byte-vectors
      (concat-byte-vectors
        (concat-byte-vectors
          (encode-u32-le 4043309087)
          (encode-u32-le 1409286315))
        (concat-byte-vectors
          (encode-u32-le 3943890975)
          (encode-u32-le 1409286250)))
      (concat-byte-vectors
        (encode-u32-le 4167072384)
        (encode-u32-le 3596551104)))
    (concat-byte-vectors
      (encode-u32-le 2854159328)
      (encode-u32-le 3596551104))))

(defn emit-aarch64-selfhost-string-length-helper []
  (let [part1 (concat-four-byte-vectors-rooted
                (encode-u32-le 2852127723)
                (encode-u32-le 2854159328)
                (encode-u32-le 3019899275)
                (encode-u32-le 3086483723))
    part2 (concat-four-byte-vectors-rooted
            (encode-u32-le 3944087935)
            (encode-u32-le 1409286371)
            (encode-u32-le 960495980)
            (encode-u32-le 872415468))
    part3 (concat-four-byte-vectors-rooted
            (encode-u32-le 2432697707)
            (encode-u32-le 2432697344)
            (encode-u32-le 402653180)
            (encode-u32-le 2453731691))
    tail (concat-three-byte-vectors-rooted
           (encode-u32-le 2332754603)
           (encode-u32-le 3107980640)
           (encode-u32-le 3596551104))]
    (concat-four-byte-vectors-rooted part1 part2 part3 tail)))

(defn emit-aarch64-selfhost-print-helper []
  (let [part1 (concat-four-byte-vectors-rooted
                (encode-u32-le 3506471935)
                (encode-u32-le 1384120649)
                (encode-u32-le 956334057)
                (encode-u32-le 2432729067))
    part2 (concat-four-byte-vectors-rooted
            (encode-u32-le 2852127724)
            (encode-u32-le 2854159341)
            (encode-u32-le 4043309471)
            (encode-u32-le 1409286250))
    part3 (concat-four-byte-vectors-rooted
            (encode-u32-le 3531604013)
            (encode-u32-le 3406562284)
            (encode-u32-le 3036676268)
            (encode-u32-le 3506439531))
    part4 (concat-four-byte-vectors-rooted
            (encode-u32-le 1384121870)
            (encode-u32-le 956301678)
            (encode-u32-le 335544329)
            (encode-u32-le 3531604302))
    part5 (concat-four-byte-vectors-rooted
            (encode-u32-le 2597194127)
            (encode-u32-le 2601431536)
            (encode-u32-le 3506439531)
            (encode-u32-le 285262352))
    part6 (concat-four-byte-vectors-rooted
            (encode-u32-le 956301680)
            (encode-u32-le 2853110764)
            (encode-u32-le 3053453132)
            (encode-u32-le 3019899021))
    part7 (concat-four-byte-vectors-rooted
            (encode-u32-le 3506439531)
            (encode-u32-le 1384121774)
            (encode-u32-le 956301678)
            (encode-u32-le 3531604000))
    part8 (concat-four-byte-vectors-rooted
            (encode-u32-le 2852848609)
            (encode-u32-le 2432730082)
            (encode-u32-le 3406495810)
            (encode-u32-le 3531604112))
    part9 (concat-four-byte-vectors-rooted
            (encode-u32-le 3556773889)
            (encode-u32-le 2432730111)
            (encode-u32-le 2854159328)
            (encode-u32-le 3596551104))
    head (concat-five-byte-vectors-rooted part1 part2 part3 part4 part5)
    tail (concat-four-byte-vectors-rooted part6 part7 part8 part9)]
    (concat-byte-vectors-rooted head tail)))

(defn emit-aarch64-selfhost-vector-new-helper []
  (concat-byte-vectors
    (concat-byte-vectors
      (concat-byte-vectors
        (concat-byte-vectors
          (concat-byte-vectors
            (encode-u32-le 2852127719)
            (encode-u32-le 3036676469))
          (concat-byte-vectors
            (encode-u32-le 3531603968)
            (encode-u32-le 3533709313)))
        (concat-byte-vectors
          (concat-byte-vectors
            (encode-u32-le 3531604066)
            (encode-u32-le 3531735107))
          (concat-byte-vectors
            (encode-u32-le 2457862148)
            (encode-u32-le 3531603973))))
      (concat-byte-vectors
        (concat-byte-vectors
          (concat-byte-vectors
            (encode-u32-le 3531610288)
            (encode-u32-le 3556773889))
          (concat-byte-vectors
            (encode-u32-le 2852127733)
            (encode-u32-le 3533701174)))
        (concat-byte-vectors
          (concat-byte-vectors
            (encode-u32-le 2852586465)
            (encode-u32-le 3548246050))
          (concat-byte-vectors
            (encode-u32-le 2432712770)
            (encode-u32-le 2853569504)))))
    (concat-byte-vectors
      (concat-byte-vectors
        (concat-byte-vectors
          (concat-byte-vectors
            (encode-u32-le 2332164822)
            (encode-u32-le 2332033699))
          (concat-byte-vectors
            (encode-u32-le 1384120484)
            (encode-u32-le 3103785060)))
        (concat-byte-vectors
          (concat-byte-vectors
            (encode-u32-le 3103786081)
            (encode-u32-le 1384120324))
          (concat-byte-vectors
            (encode-u32-le 3103787108)
            (encode-u32-le 3103788132))))
      (concat-byte-vectors
        (concat-byte-vectors
          (encode-u32-le 3538944004)
          (encode-u32-le 2852388864))
        (encode-u32-le 3596551104)))))

(defn emit-aarch64-selfhost-vector-length-helper []
  (concat-byte-vectors
    (concat-byte-vectors
      (concat-byte-vectors
        (encode-u32-le 3538944001)
        (encode-u32-le 2332098560))
      (concat-byte-vectors
        (encode-u32-le 2332033696)
        (encode-u32-le 3107981312)))
    (encode-u32-le 3596551104)))

(defn emit-aarch64-selfhost-alloc-helper []
  (let [part1 (concat-byte-vectors
                (concat-byte-vectors
                  (encode-u32-le 2852127719)
                  (encode-u32-le 3036676469))
                (concat-byte-vectors
                  (encode-u32-le 3531603968)
                  (encode-u32-le 3533709313)))
    part2 (concat-byte-vectors
            (concat-byte-vectors
              (encode-u32-le 3531604066)
              (encode-u32-le 3531735107))
            (concat-byte-vectors
              (encode-u32-le 2457862148)
              (encode-u32-le 3531603973)))
    part3 (concat-byte-vectors
            (concat-byte-vectors
              (encode-u32-le 3531610288)
              (encode-u32-le 3556773889))
            (concat-byte-vectors
              (encode-u32-le 2852127733)
              (encode-u32-le 3533701174)))
    part4 (concat-byte-vectors
            (concat-byte-vectors
              (encode-u32-le 2432703713)
              (encode-u32-le 3544448033))
            (concat-byte-vectors
              (concat-byte-vectors
                (encode-u32-le 3548246049)
                (encode-u32-le 2853569504))
              (concat-byte-vectors
                (encode-u32-le 2332099286)
                (encode-u32-le 3596551104))))]
    (concat-byte-vectors
      (concat-byte-vectors part1 part2)
      (concat-byte-vectors part3 part4))))

(defn emit-aarch64-selfhost-string-char-at-helper []
  (let [part1 (concat-byte-vectors
                (encode-u32-le 3019899241)
                (encode-u32-le 3086483625))
    part2 (concat-byte-vectors
            (encode-u32-le 3070230761)
            (encode-u32-le 2332623529))
    part3 (concat-byte-vectors
            (encode-u32-le 2432704809)
            (concat-byte-vectors
              (encode-u32-le 335544324)
              (encode-u32-le 2453731625)))
    part4 (concat-byte-vectors
            (encode-u32-le 2332623529)
            (encode-u32-le 2432704809))
    part5 (concat-byte-vectors
            (encode-u32-le 945842464)
            (encode-u32-le 3596551104))
    part6 (concat-byte-vectors
            (encode-u32-le 2854159328)
            (encode-u32-le 3596551104))]
    (concat-byte-vectors
      (concat-byte-vectors
        (concat-byte-vectors part1 part2)
        (concat-byte-vectors part3 part4))
      (concat-byte-vectors part5 part6))))

(defn emit-aarch64-selfhost-vector-get-helper []
  (let [part1 (concat-byte-vectors
                (encode-u32-le 3019899241)
                (encode-u32-le 2852127723))
    part2 (concat-byte-vectors
            (encode-u32-le 3538944001)
            (concat-byte-vectors
              (encode-u32-le 2332098849)
              (encode-u32-le 2332099233)))
    part3 (concat-byte-vectors
            (encode-u32-le 3107981346)
            (encode-u32-le 3942777215))
    part4 (concat-byte-vectors
            (encode-u32-le 1409286282)
            (concat-byte-vectors
              (encode-u32-le 2332757024)
              (encode-u32-le 4181723136)))
    part5 (concat-byte-vectors
            (encode-u32-le 3596551104)
            (concat-byte-vectors
              (encode-u32-le 2854159328)
              (encode-u32-le 3596551104)))]
    (concat-byte-vectors
      (concat-byte-vectors
        (concat-byte-vectors part1 part2)
        (concat-byte-vectors part3 part4))
      part5)))

(defn emit-aarch64-selfhost-vector-push-helper []
  (let [part1 (concat-byte-vectors
                (concat-byte-vectors
                  (encode-u32-le 3019900873)
                  (encode-u32-le 2852127723))
                (concat-byte-vectors
                  (encode-u32-le 3538944001)
                  (encode-u32-le 2332098860)))
    part2 (concat-byte-vectors
            (concat-byte-vectors
              (encode-u32-le 2332820140)
              (encode-u32-le 3107981698))
            (concat-byte-vectors
              (encode-u32-le 3107980675)
              (encode-u32-le 1795358815)))
    part3 (concat-byte-vectors
            (concat-byte-vectors
              (encode-u32-le 1409287691)
              (encode-u32-le 184746084))
            (concat-byte-vectors
              (encode-u32-le 1895829663)
              (encode-u32-le 1409286218)))
    part4 (concat-byte-vectors
            (concat-byte-vectors
              (encode-u32-le 1384120452)
              (encode-u32-le 2852914159))
            (concat-byte-vectors
              (encode-u32-le 704775150)
              (encode-u32-le 2852389856)))
    part5 (concat-byte-vectors
            (concat-byte-vectors
              (encode-u32-le 3548246016)
              (encode-u32-le 2432712704))
            (concat-byte-vectors
              (encode-u32-le 2852127719)
              (encode-u32-le 3036676469)))
    part6 (concat-byte-vectors
            (concat-byte-vectors
              (encode-u32-le 3531603968)
              (encode-u32-le 3533709313))
            (concat-byte-vectors
              (encode-u32-le 3531604066)
              (encode-u32-le 3531735107)))
    part7 (concat-byte-vectors
            (concat-byte-vectors
              (encode-u32-le 2457862148)
              (encode-u32-le 3531603973))
            (concat-byte-vectors
              (encode-u32-le 3531610288)
              (encode-u32-le 3556773889)))
    part8 (concat-byte-vectors
            (concat-byte-vectors
              (encode-u32-le 2852127733)
              (encode-u32-le 3533701174))
            (concat-byte-vectors
              (encode-u32-le 2432703713)
              (encode-u32-le 3544448033)))
    part9 (concat-byte-vectors
            (concat-byte-vectors
              (encode-u32-le 3548246049)
              (encode-u32-le 2853569504))
            (concat-byte-vectors
              (encode-u32-le 2332099286)
              (encode-u32-le 2332033697)))
    part10 (concat-byte-vectors
             (concat-byte-vectors
               (encode-u32-le 1384120485)
               (encode-u32-le 3103784997))
             (concat-byte-vectors
               (encode-u32-le 3103786020)
               (encode-u32-le 285214150)))
    part11 (concat-byte-vectors
             (concat-byte-vectors
               (encode-u32-le 3103787046)
               (encode-u32-le 1384120325))
             (concat-byte-vectors
               (encode-u32-le 3103788069)
               (encode-u32-le 2432713186)))
    part12 (concat-byte-vectors
             (concat-byte-vectors
               (encode-u32-le 2432712739)
               (encode-u32-le 705561573))
             (concat-byte-vectors
               (encode-u32-le 3548246181)
               (encode-u32-le 3019899045)))
    part13 (concat-byte-vectors
             (concat-byte-vectors
               (encode-u32-le 943723590)
               (encode-u32-le 939529318))
             (concat-byte-vectors
               (encode-u32-le 4043310245)
               (encode-u32-le 1426063233)))
    part14 (concat-byte-vectors
             (concat-byte-vectors
               (encode-u32-le 4177526891)
               (encode-u32-le 3538944001))
             (concat-byte-vectors
               (encode-u32-le 2852192256)
               (encode-u32-le 3596551104)))
    part15 (concat-byte-vectors
             (concat-byte-vectors
               (encode-u32-le 2332167556)
               (encode-u32-le 4177528971))
             (concat-byte-vectors
               (encode-u32-le 285213762)
               (encode-u32-le 3103787394)))
    part16 (concat-byte-vectors
             (concat-byte-vectors
               (encode-u32-le 2852717536)
               (encode-u32-le 3596551104))
             (concat-byte-vectors
               (encode-u32-le 2854159328)
               (encode-u32-le 3596551104)))]
    (concat-byte-vectors
      (concat-byte-vectors
        (concat-byte-vectors
          (concat-byte-vectors part1 part2)
          (concat-byte-vectors part3 part4))
        (concat-byte-vectors
          (concat-byte-vectors part5 part6)
          (concat-byte-vectors part7 part8)))
      (concat-byte-vectors
        (concat-byte-vectors
          (concat-byte-vectors part9 part10)
          (concat-byte-vectors part11 part12))
        (concat-byte-vectors
          (concat-byte-vectors part13 part14)
          (concat-byte-vectors part15 part16))))))

(defn emit-aarch64-selfhost-ref-new-helper []
  (let [part1 (concat-byte-vectors
                (concat-byte-vectors
                  (encode-u32-le 2852127719)
                  (encode-u32-le 3036676469))
                (concat-byte-vectors
                  (encode-u32-le 3531603968)
                  (encode-u32-le 3533709313)))
    part2 (concat-byte-vectors
            (concat-byte-vectors
              (encode-u32-le 3531604066)
              (encode-u32-le 3531735107))
            (concat-byte-vectors
              (encode-u32-le 2457862148)
              (encode-u32-le 3531603973)))
    part3 (concat-byte-vectors
            (concat-byte-vectors
              (encode-u32-le 3531610288)
              (encode-u32-le 3556773889))
            (concat-byte-vectors
              (encode-u32-le 2852127733)
              (encode-u32-le 3533701174)))
    part4 (concat-byte-vectors
            (concat-byte-vectors
              (encode-u32-le 3531604481)
              (encode-u32-le 2853569504))
            (concat-byte-vectors
              (encode-u32-le 2332099286)
              (encode-u32-le 2332033699)))
    part5 (concat-byte-vectors
            (concat-byte-vectors
              (encode-u32-le 1384120548)
              (encode-u32-le 3103785060))
            (concat-byte-vectors
              (encode-u32-le 1384120836)
              (encode-u32-le 3103786084)))
    part6 (concat-byte-vectors
            (concat-byte-vectors
              (encode-u32-le 4177527911)
              (encode-u32-le 3538944004))
            (concat-byte-vectors
              (encode-u32-le 2852388864)
              (encode-u32-le 3596551104)))]
    (concat-byte-vectors
      (concat-byte-vectors
        (concat-byte-vectors part1 part2)
        (concat-byte-vectors part3 part4))
      (concat-byte-vectors part5 part6))))

(defn emit-aarch64-selfhost-ref-get-helper []
  (concat-byte-vectors
    (concat-byte-vectors
      (concat-byte-vectors
        (encode-u32-le 3538944001)
        (encode-u32-le 2332098560))
      (concat-byte-vectors
        (encode-u32-le 2332033696)
        (encode-u32-le 4181722112)))
    (encode-u32-le 3596551104)))

(defn emit-aarch64-selfhost-ref-set-helper []
  (concat-byte-vectors
    (concat-byte-vectors
      (concat-byte-vectors
        (encode-u32-le 3538944001)
        (encode-u32-le 2332098849))
      (concat-byte-vectors
        (encode-u32-le 2332099233)
        (encode-u32-le 4177527840)))
    (concat-byte-vectors
      (encode-u32-le 3531603968)
      (encode-u32-le 3596551104))))

(defn emit-aarch64-selfhost-substring-helper []
  (let [part1 (concat-four-byte-vectors-rooted
                (encode-u32-le 2852127735)
                (encode-u32-le 2852193272)
                (encode-u32-le 2852717561)
                (encode-u32-le 3036676469))
    part2 (concat-four-byte-vectors-rooted
            (encode-u32-le 3531603968)
            (encode-u32-le 3533709313)
            (encode-u32-le 3531604066)
            (encode-u32-le 3531735107))
    part3 (concat-four-byte-vectors-rooted
            (encode-u32-le 2457862148)
            (encode-u32-le 3531603973)
            (encode-u32-le 3531610288)
            (encode-u32-le 3556773889))
    part4 (concat-four-byte-vectors-rooted
            (encode-u32-le 2852127733)
            (encode-u32-le 3533701174)
            (encode-u32-le 2853635040)
            (encode-u32-le 2853700577))
    part5 (concat-four-byte-vectors-rooted
            (encode-u32-le 2853766121)
            (encode-u32-le 3406364679)
            (encode-u32-le 2432704738)
            (encode-u32-le 2432703554))
    part6 (concat-four-byte-vectors-rooted
            (encode-u32-le 3544448066)
            (encode-u32-le 3548246082)
            (encode-u32-le 2853569504)
            (encode-u32-le 2332164822))
    part7 (concat-four-byte-vectors-rooted
            (encode-u32-le 2332033699)
            (encode-u32-le 1384120356)
            (encode-u32-le 3103785060)
            (encode-u32-le 705102820))
    part8 (concat-four-byte-vectors-rooted
             (encode-u32-le 3103786084)
             (encode-u32-le 3086483745)
             (encode-u32-le 3944087615)
             (encode-u32-le 1409286243))
    part9 (concat-four-byte-vectors-rooted
             (encode-u32-le 2332622881)
             (encode-u32-le 335544329)
             (encode-u32-le 2332099233)
             (encode-u32-le 2432704545))
    part10 (concat-four-byte-vectors-rooted
              (encode-u32-le 2332622881)
              (encode-u32-le 335544325)
              (encode-u32-le 2453731361)
              (encode-u32-le 2332099233))
    part11 (concat-four-byte-vectors-rooted
              (encode-u32-le 2432704545)
              (encode-u32-le 2332622881)
              (encode-u32-le 2432704612)
              (encode-u32-le 2852586469))
    part12 (concat-four-byte-vectors-rooted
             (encode-u32-le 3019899045)
             (encode-u32-le 943723558)
             (encode-u32-le 939529350)
             (encode-u32-le 3506439333))
    part13 (concat-four-byte-vectors-rooted
             (encode-u32-le 402653180)
             (encode-u32-le 3538944004)
             (encode-u32-le 2852388864)
             (encode-u32-le 3596551104))
    head (concat-five-byte-vectors-rooted part1 part2 part3 part4 part5)
    mid (concat-four-byte-vectors-rooted part6 part7 part8 part9)
    tail (concat-four-byte-vectors-rooted part10 part11 part12 part13)]
    (concat-three-byte-vectors-rooted head mid tail)))

(defn emit-aarch64-selfhost-string-concat-helper-chunk1 []
  (let [result (ref-new (vector-new 80))]
    (do
      (root_push result)
      (append-encoded-u32-rooted result 2852717559)
      (append-encoded-u32-rooted result 2852127736)
      (append-encoded-u32-rooted result 3036676469)
      (append-encoded-u32-rooted result 3531603968)
      (append-encoded-u32-rooted result 3533709313)
      (append-encoded-u32-rooted result 3531604066)
      (append-encoded-u32-rooted result 3531735107)
      (append-encoded-u32-rooted result 2457862148)
      (append-encoded-u32-rooted result 3531603973)
      (append-encoded-u32-rooted result 3531610288)
      (append-encoded-u32-rooted result 3556773889)
      (append-encoded-u32-rooted result 2852127733)
      (append-encoded-u32-rooted result 3533701174)
      (append-encoded-u32-rooted result 3019899415)
      (append-encoded-u32-rooted result 3086483767)
      (append-encoded-u32-rooted result 2853635052)
      (append-encoded-u32-rooted result 2853635054)
      (append-encoded-u32-rooted result 2854159338)
      (append-encoded-u32-rooted result 960496079)
      (append-encoded-u32-rooted result 872415631)
      (let [final (ref-get result)]
        (do
          (root_pop)
          final)))))

(defn emit-aarch64-selfhost-string-concat-helper-chunk2 []
  (let [result (ref-new (vector-new 80))]
    (do
      (root_push result)
      (append-encoded-u32-rooted result 2432697806)
      (append-encoded-u32-rooted result 2432697674)
      (append-encoded-u32-rooted result 402653180)
      (append-encoded-u32-rooted result 3538944002)
      (append-encoded-u32-rooted result 2332164846)
      (append-encoded-u32-rooted result 2332951214)
      (append-encoded-u32-rooted result 3107980746)
      (append-encoded-u32-rooted result 2432704972)
      (append-encoded-u32-rooted result 335544323)
      (append-encoded-u32-rooted result 2854159338)
      (append-encoded-u32-rooted result 2854159340)
      (append-encoded-u32-rooted result 3019899416)
      (append-encoded-u32-rooted result 3086483768)
      (append-encoded-u32-rooted result 2853700589)
      (append-encoded-u32-rooted result 2853700590)
      (append-encoded-u32-rooted result 2854159339)
      (append-encoded-u32-rooted result 960496079)
      (append-encoded-u32-rooted result 872415631)
      (append-encoded-u32-rooted result 2432697806)
      (append-encoded-u32-rooted result 2432697707)
      (let [final (ref-get result)]
        (do
          (root_pop)
          final)))))

(defn emit-aarch64-selfhost-string-concat-helper-chunk3 []
  (let [result (ref-new (vector-new 80))]
    (do
      (root_push result)
      (append-encoded-u32-rooted result 402653180)
      (append-encoded-u32-rooted result 3538944002)
      (append-encoded-u32-rooted result 2332164878)
      (append-encoded-u32-rooted result 2332951214)
      (append-encoded-u32-rooted result 3107980747)
      (append-encoded-u32-rooted result 2432704973)
      (append-encoded-u32-rooted result 335544323)
      (append-encoded-u32-rooted result 2854159339)
      (append-encoded-u32-rooted result 2854159341)
      (append-encoded-u32-rooted result 2332754247)
      (append-encoded-u32-rooted result 2432704738)
      (append-encoded-u32-rooted result 2432703554)
      (append-encoded-u32-rooted result 3544448066)
      (append-encoded-u32-rooted result 3548246082)
      (append-encoded-u32-rooted result 2853569504)
      (append-encoded-u32-rooted result 2332164822)
      (append-encoded-u32-rooted result 2332033699)
      (append-encoded-u32-rooted result 1384120356)
      (append-encoded-u32-rooted result 3103785060)
      (append-encoded-u32-rooted result 705102820)
      (let [final (ref-get result)]
        (do
          (root_pop)
          final)))))

(defn emit-aarch64-selfhost-string-concat-helper-chunk4 []
  (let [result (ref-new (vector-new 68))]
    (do
      (root_push result)
      (append-encoded-u32-rooted result 3103786084)
      (append-encoded-u32-rooted result 2432704622)
      (append-encoded-u32-rooted result 2852783087)
      (append-encoded-u32-rooted result 3019899055)
      (append-encoded-u32-rooted result 943723920)
      (append-encoded-u32-rooted result 939529680)
      (append-encoded-u32-rooted result 3506439663)
      (append-encoded-u32-rooted result 3053453231)
      (append-encoded-u32-rooted result 2852848623)
      (append-encoded-u32-rooted result 3019899055)
      (append-encoded-u32-rooted result 943723952)
      (append-encoded-u32-rooted result 939529680)
      (append-encoded-u32-rooted result 3506439663)
      (append-encoded-u32-rooted result 3053453231)
      (append-encoded-u32-rooted result 3538944004)
      (append-encoded-u32-rooted result 2852388864)
      (append-encoded-u32-rooted result 3596551104)
      (let [final (ref-get result)]
        (do
          (root_pop)
          final)))))

(defn emit-aarch64-selfhost-string-concat-helper []
  (let [part1 (concat-four-byte-vectors-rooted
                (encode-u32-le 2852717559)
                (encode-u32-le 2852127736)
                (encode-u32-le 3036676469)
                (encode-u32-le 3531603968))
    part2 (concat-four-byte-vectors-rooted
            (encode-u32-le 3533709313)
            (encode-u32-le 3531604066)
            (encode-u32-le 3531735107)
            (encode-u32-le 2457862148))
    part3 (concat-four-byte-vectors-rooted
            (encode-u32-le 3531603973)
            (encode-u32-le 3531610288)
            (encode-u32-le 3556773889)
            (encode-u32-le 2852127733))
    part4 (concat-four-byte-vectors-rooted
            (encode-u32-le 3533701174)
            (encode-u32-le 2854159338)
            (encode-u32-le 3019899383)
            (encode-u32-le 3086483767))
    part5 (concat-four-byte-vectors-rooted
            (encode-u32-le 2853635054)
            (encode-u32-le 3053453623)
            (encode-u32-le 2853635052)
            (encode-u32-le 960496079))
    part6 (concat-four-byte-vectors-rooted
            (encode-u32-le 872415535)
            (encode-u32-le 2432697806)
            (encode-u32-le 2432697674)
            (encode-u32-le 402653180))
    part7 (concat-four-byte-vectors-rooted
            (encode-u32-le 3538944002)
            (encode-u32-le 2332164846)
            (encode-u32-le 2332951214)
            (encode-u32-le 3107980746))
    part8 (concat-four-byte-vectors-rooted
            (encode-u32-le 2432704972)
            (encode-u32-le 2854159339)
            (encode-u32-le 3019899384)
            (encode-u32-le 3086483768))
    part9 (concat-four-byte-vectors-rooted
            (encode-u32-le 2853700590)
            (encode-u32-le 3053453624)
            (encode-u32-le 2853700589)
            (encode-u32-le 960496079))
    part10 (concat-four-byte-vectors-rooted
             (encode-u32-le 872415535)
             (encode-u32-le 2432697806)
             (encode-u32-le 2432697707)
             (encode-u32-le 402653180))
    part11 (concat-four-byte-vectors-rooted
             (encode-u32-le 3538944002)
             (encode-u32-le 2332164878)
             (encode-u32-le 2332951214)
             (encode-u32-le 3107980747))
    part12 (concat-four-byte-vectors-rooted
             (encode-u32-le 2432704973)
             (encode-u32-le 2332754247)
             (encode-u32-le 2432704738)
             (encode-u32-le 2432703554))
    part13 (concat-four-byte-vectors-rooted
             (encode-u32-le 3544448066)
             (encode-u32-le 3548246082)
             (encode-u32-le 2853569504)
             (encode-u32-le 2332164822))
    part14 (concat-four-byte-vectors-rooted
             (encode-u32-le 2332033699)
             (encode-u32-le 1384120356)
             (encode-u32-le 3103785060)
             (encode-u32-le 705102820))
    part15 (concat-four-byte-vectors-rooted
             (encode-u32-le 3103786084)
             (encode-u32-le 2432704622)
             (encode-u32-le 2852783087)
             (encode-u32-le 3019899055))
    part16 (concat-four-byte-vectors-rooted
             (encode-u32-le 943723920)
             (encode-u32-le 939529680)
             (encode-u32-le 3506439663)
             (encode-u32-le 3053453231))
    part17 (concat-four-byte-vectors-rooted
             (encode-u32-le 2852848623)
             (encode-u32-le 3019899055)
             (encode-u32-le 943723952)
             (encode-u32-le 939529680))
    part18 (concat-four-byte-vectors-rooted
             (encode-u32-le 3506439663)
             (encode-u32-le 3053453231)
             (encode-u32-le 3538944004)
             (encode-u32-le 2852388864))
    tail (concat-five-byte-vectors-rooted
           (encode-u32-le 3596551104)
           (encode-u32-le 3573751839)
           (encode-u32-le 3573751839)
           (encode-u32-le 3573751839)
           (encode-u32-le 3573751839))
    head1 (concat-five-byte-vectors-rooted part1 part2 part3 part4 part5)
    head2 (concat-five-byte-vectors-rooted part6 part7 part8 part9 part10)
    head3 (concat-five-byte-vectors-rooted part11 part12 part13 part14 part15)
    head4 (concat-three-byte-vectors-rooted part16 part17 part18)]
    (concat-five-byte-vectors-rooted head1 head2 head3 head4 tail)))

(defn emit-aarch64-selfhost-map-size-helper []
  (concat-byte-vectors
    (concat-byte-vectors
      (concat-byte-vectors
        (encode-u32-le 3538944001)
        (encode-u32-le 2332098560))
      (concat-byte-vectors
        (encode-u32-le 2332033696)
        (encode-u32-le 3107981312)))
    (encode-u32-le 3596551104)))

(defn emit-aarch64-selfhost-map-new-helper []
  (let [part1 (concat-byte-vectors
                (concat-byte-vectors
                  (encode-u32-le 3036676469)
                  (encode-u32-le 3531603968))
                (concat-byte-vectors
                  (encode-u32-le 3533709313)
                  (encode-u32-le 3531604066)))
    part2 (concat-byte-vectors
            (concat-byte-vectors
              (encode-u32-le 3531735107)
              (encode-u32-le 2457862148))
            (concat-byte-vectors
              (encode-u32-le 3531603973)
              (encode-u32-le 3531610288)))
    part3 (concat-byte-vectors
            (concat-byte-vectors
              (encode-u32-le 3556773889)
              (encode-u32-le 2852127733))
            (concat-byte-vectors
              (encode-u32-le 3533701174)
              (encode-u32-le 2853569504)))
    part4 (concat-byte-vectors
            (concat-byte-vectors
              (encode-u32-le 3533701154)
              (encode-u32-le 2332164822))
            (concat-byte-vectors
              (encode-u32-le 2332033699)
              (encode-u32-le 1384120516)))
    part5 (concat-byte-vectors
            (concat-byte-vectors
              (encode-u32-le 3103785060)
              (encode-u32-le 1384250884))
            (concat-byte-vectors
              (encode-u32-le 3103786084)
              (encode-u32-le 1384120324)))
    part6 (concat-byte-vectors
            (concat-byte-vectors
              (encode-u32-le 3103787108)
              (encode-u32-le 3103788132))
            (concat-byte-vectors
              (encode-u32-le 3538944004)
              (encode-u32-le 2852388864)))
    part7 (encode-u32-le 3596551104)]
    (concat-byte-vectors
      (concat-byte-vectors
        (concat-byte-vectors part1 part2)
        (concat-byte-vectors part3 part4))
      (concat-byte-vectors
        (concat-byte-vectors part5 part6)
        part7))))

(defn emit-aarch64-selfhost-map-insert-helper []
  (let [part1 (concat-byte-vectors
                (concat-byte-vectors
                  (encode-u32-le 704709611)
                  (encode-u32-le 2332754603))
                (concat-byte-vectors
                  (encode-u32-le 3107980652)
                  (encode-u32-le 2432713069)))
    part2 (concat-byte-vectors
            (concat-byte-vectors
              (encode-u32-le 872415500)
              (encode-u32-le 4181721518))
            (concat-byte-vectors
              (encode-u32-le 3019899150)
              (encode-u32-le 3943236063)))
    part3 (concat-byte-vectors
            (concat-byte-vectors
              (encode-u32-le 1409286560)
              (encode-u32-le 1358955916))
            (concat-byte-vectors
              (encode-u32-le 2432713133)
              (encode-u32-le 905969484)))
    part4 (concat-byte-vectors
            (concat-byte-vectors
              (encode-u32-le 2852193248)
              (encode-u32-le 3596551104))
            (concat-byte-vectors
              (encode-u32-le 4177527209)
              (encode-u32-le 4177528224)))
    part5 (concat-byte-vectors
            (concat-byte-vectors
              (encode-u32-le 3107981679)
              (encode-u32-le 285214191))
            (concat-byte-vectors
              (encode-u32-le 3103787375)
              (encode-u32-le 2852193248)))
    part6 (concat-byte-vectors
            (concat-byte-vectors
              (encode-u32-le 3596551104)
              (encode-u32-le 4177528224))
            (concat-byte-vectors
              (encode-u32-le 2852193248)
              (encode-u32-le 3596551104)))]
    (concat-byte-vectors
      (concat-byte-vectors
        (concat-byte-vectors part1 part2)
        (concat-byte-vectors part3 part4))
      (concat-byte-vectors part5 part6))))

(defn emit-aarch64-selfhost-map-get-helper []
  (let [part1 (concat-byte-vectors
                (concat-byte-vectors
                  (encode-u32-le 705233899)
                  (encode-u32-le 2332754603))
                (concat-byte-vectors
                  (encode-u32-le 3107980652)
                  (encode-u32-le 2432713067)))
    part2 (concat-byte-vectors
            (concat-byte-vectors
              (encode-u32-le 872415468)
              (encode-u32-le 4181721453))
            (concat-byte-vectors
              (encode-u32-le 3942646207)
              (encode-u32-le 1409286336)))
    part3 (concat-byte-vectors
            (concat-byte-vectors
              (encode-u32-le 1358955916)
              (encode-u32-le 2432713067))
            (concat-byte-vectors
              (encode-u32-le 905969516)
              (encode-u32-le 2854159328)))
    part4 (concat-byte-vectors
            (concat-byte-vectors
              (encode-u32-le 3596551104)
              (encode-u32-le 4181722464))
            (encode-u32-le 3596551104))]
    (concat-byte-vectors
      (concat-byte-vectors part1 part2)
      (concat-byte-vectors part3 part4))))

(defn emit-aarch64-selfhost-map-new-fixed-helper []
  (let [part1 (concat-byte-vectors
                (concat-byte-vectors
                  (encode-u32-le 3533701152)
                  (encode-u32-le 2854093806))
                (concat-byte-vectors
                  (emit-aarch64-bl -2004)
                  (encode-u32-le 2332033699)))
    part2 (concat-byte-vectors
            (concat-byte-vectors
              (encode-u32-le 1384120516)
              (encode-u32-le 3103785060))
            (concat-byte-vectors
              (encode-u32-le 1384250884)
              (encode-u32-le 3103786084)))
    part3 (concat-byte-vectors
            (concat-byte-vectors
              (encode-u32-le 1384120324)
              (encode-u32-le 3103787108))
            (concat-byte-vectors
              (encode-u32-le 3103788132)
              (encode-u32-le 2432712805)))
    part4 (concat-byte-vectors
            (concat-byte-vectors
              (encode-u32-le 1384250886)
              (encode-u32-le 872415430))
            (concat-byte-vectors
              (encode-u32-le 4177526975)
              (encode-u32-le 4177527999)))
    part5 (concat-byte-vectors
            (concat-byte-vectors
              (encode-u32-le 2432712869)
              (encode-u32-le 1358955718))
            (concat-byte-vectors
              (encode-u32-le 905969510)
              (encode-u32-le 3538944004)))
    part6 (concat-byte-vectors
            (encode-u32-le 2852388864)
            (concat-byte-vectors
              (encode-u32-le 2853045246)
              (encode-u32-le 3596551104)))]
    (concat-byte-vectors
      (concat-byte-vectors
        (concat-byte-vectors part1 part2)
        (concat-byte-vectors part3 part4))
      (concat-byte-vectors part5 part6))))

(defn emit-aarch64-selfhost-file-exists-helper []
  (let [part1 (concat-byte-vectors
                (concat-byte-vectors
                  (encode-u32-le 2852127721)
                  (encode-u32-le 3086483936))
                (concat-byte-vectors
                  (encode-u32-le 2852717536)
                  (encode-u32-le 3531603969)))
    part2 (concat-byte-vectors
            (concat-byte-vectors
              (encode-u32-le 3531603970)
              (encode-u32-le 3531604144))
            (concat-byte-vectors
              (encode-u32-le 3556773889)
              (encode-u32-le 1409286370)))
    part3 (concat-byte-vectors
            (concat-byte-vectors
              (encode-u32-le 2852127721)
              (encode-u32-le 2852717536))
            (concat-byte-vectors
              (encode-u32-le 3531604176)
              (encode-u32-le 3556773889)))
    part4 (concat-byte-vectors
            (concat-byte-vectors
              (encode-u32-le 3531604000)
              (encode-u32-le 3596551104))
            (concat-byte-vectors
              (encode-u32-le 3531603968)
              (encode-u32-le 3596551104)))
    part5 (concat-byte-vectors
            (concat-byte-vectors
              (encode-u32-le 3538944010)
              (encode-u32-le 3406430505))
            (concat-byte-vectors
              (encode-u32-le 2332623530)
              (encode-u32-le 3107980619)))
    part6 (concat-byte-vectors
            (concat-byte-vectors
              (encode-u32-le 2432704844)
              (encode-u32-le 2432713070))
            (concat-byte-vectors
              (encode-u32-le 2457660878)
              (encode-u32-le 3408815103)))
    part7 (concat-byte-vectors
            (concat-byte-vectors
              (encode-u32-le 2432697325)
              (encode-u32-le 2852848623))
            (concat-byte-vectors
              (encode-u32-le 3019899055)
              (encode-u32-le 943723920)))
    part8 (concat-byte-vectors
            (concat-byte-vectors
              (encode-u32-le 939529648)
              (encode-u32-le 3506439663))
            (concat-byte-vectors
              (encode-u32-le 3053453231)
              (encode-u32-le 706675696)))
    part9 (concat-byte-vectors
            (concat-byte-vectors
              (encode-u32-le 956301744)
              (encode-u32-le 2432697312))
            (concat-byte-vectors
              (encode-u32-le 3531603969)
              (encode-u32-le 3531603970)))
    part10 (concat-byte-vectors
             (concat-byte-vectors
               (encode-u32-le 3531604144)
               (encode-u32-le 3556773889))
             (concat-byte-vectors
               (encode-u32-le 1409286402)
               (encode-u32-le 2852127721)))
    part11 (concat-byte-vectors
             (concat-byte-vectors
               (encode-u32-le 2852717536)
               (encode-u32-le 3531604176))
             (concat-byte-vectors
               (encode-u32-le 3556773889)
               (encode-u32-le 2335073279)))
    part12 (concat-byte-vectors
             (concat-byte-vectors
               (encode-u32-le 3531604000)
               (encode-u32-le 3596551104))
             (concat-byte-vectors
               (encode-u32-le 2335073279)
               (encode-u32-le 3531603968)))
    part13 (encode-u32-le 3596551104)]
    (concat-byte-vectors
      (concat-byte-vectors
        (concat-byte-vectors
          (concat-byte-vectors part1 part2)
          (concat-byte-vectors part3 part4))
        (concat-byte-vectors
          (concat-byte-vectors part5 part6)
          (concat-byte-vectors part7 part8)))
      (concat-byte-vectors
        (concat-byte-vectors
          (concat-byte-vectors part9 part10)
          (concat-byte-vectors part11 part12))
        part13))))

(defn emit-aarch64-selfhost-read-file-helper []
  (let [part1 (concat-byte-vectors-rooted
                (concat-four-byte-vectors-rooted
                  (encode-u32-le 2852127721)
                  (encode-u32-le 3086483744)
                  (encode-u32-le 2852717536)
                  (encode-u32-le 3531603969))
                (concat-four-byte-vectors-rooted
                  (encode-u32-le 3531603970)
                  (encode-u32-le 3531604144)
                  (encode-u32-le 3556773889)
                  (encode-u32-le 1409288930)))
    part2 (concat-byte-vectors-rooted
            (concat-four-byte-vectors-rooted
              (encode-u32-le 2852127735)
              (encode-u32-le 335544349)
              (encode-u32-le 3538944010)
              (encode-u32-le 3406430505))
            (concat-four-byte-vectors-rooted
              (encode-u32-le 2332623530)
              (encode-u32-le 3107980619)
              (encode-u32-le 2432704844)
              (encode-u32-le 2432713070)))
    part3 (concat-byte-vectors-rooted
            (concat-four-byte-vectors-rooted
              (encode-u32-le 2457660878)
              (encode-u32-le 3408815103)
              (encode-u32-le 2432697325)
              (encode-u32-le 2852848623))
            (concat-four-byte-vectors-rooted
              (encode-u32-le 3019899055)
              (encode-u32-le 943723920)
              (encode-u32-le 939529648)
              (encode-u32-le 3506439663)))
    part4 (concat-byte-vectors-rooted
            (concat-four-byte-vectors-rooted
              (encode-u32-le 3053453199)
              (encode-u32-le 706675696)
              (encode-u32-le 956301744)
              (encode-u32-le 2432697312))
            (concat-four-byte-vectors-rooted
              (encode-u32-le 3531603969)
              (encode-u32-le 3531603970)
              (encode-u32-le 3531604144)
              (encode-u32-le 3556773889)))
    part5 (concat-byte-vectors-rooted
            (concat-four-byte-vectors-rooted
              (encode-u32-le 1409286274)
              (encode-u32-le 2852127735)
              (encode-u32-le 2335073279)
              (encode-u32-le 335544323))
            (concat-four-byte-vectors-rooted
              (encode-u32-le 2335073279)
              (encode-u32-le 335544377)
              (encode-u32-le 2853635040)
              (encode-u32-le 3531603969)))
    part6 (concat-byte-vectors-rooted
            (concat-four-byte-vectors-rooted
              (encode-u32-le 3531604034)
              (encode-u32-le 3531610352)
              (encode-u32-le 3556773889)
              (encode-u32-le 1409287682))
            (concat-four-byte-vectors-rooted
              (encode-u32-le 2852127736)
              (encode-u32-le 2853635040)
              (encode-u32-le 3531603969)
              (encode-u32-le 3531603970)))
    part7 (concat-byte-vectors-rooted
            (concat-four-byte-vectors-rooted
              (encode-u32-le 3531610352)
              (encode-u32-le 3556773889)
              (encode-u32-le 1409287458)
              (encode-u32-le 3036676469))
            (concat-four-byte-vectors-rooted
              (encode-u32-le 3531603968)
              (encode-u32-le 3533709313)
              (encode-u32-le 3531604066)
              (encode-u32-le 3531735107)))
    part8 (concat-byte-vectors-rooted
            (concat-four-byte-vectors-rooted
              (encode-u32-le 2457862148)
              (encode-u32-le 3531603973)
              (encode-u32-le 3531610288)
              (encode-u32-le 3556773889))
            (concat-four-byte-vectors-rooted
              (encode-u32-le 2852127733)
              (encode-u32-le 3533701174)
              (encode-u32-le 2432705282)
              (encode-u32-le 2432703554)))
    part9 (concat-byte-vectors-rooted
            (concat-four-byte-vectors-rooted
              (encode-u32-le 3544448066)
              (encode-u32-le 3548246082)
              (encode-u32-le 2853569504)
              (encode-u32-le 2332164822))
            (concat-four-byte-vectors-rooted
              (encode-u32-le 2332033699)
              (encode-u32-le 1384120356)
              (encode-u32-le 3103785060)
              (encode-u32-le 706216932)))
    part10 (concat-byte-vectors-rooted
             (concat-four-byte-vectors-rooted
               (encode-u32-le 3103786084)
               (encode-u32-le 2852127737)
               (encode-u32-le 2852324346)
               (encode-u32-le 2853635040))
             (concat-four-byte-vectors-rooted
               (encode-u32-le 2432704609)
               (encode-u32-le 2853700578)
               (encode-u32-le 3531604080)
               (encode-u32-le 3556773889)))
    part11 (concat-byte-vectors-rooted
             (concat-four-byte-vectors-rooted
               (encode-u32-le 1409286434)
               (encode-u32-le 2852127736)
               (encode-u32-le 2853635040)
               (encode-u32-le 3531604176))
             (concat-four-byte-vectors-rooted
               (encode-u32-le 3556773889)
               (encode-u32-le 3103786840)
               (encode-u32-le 3538944004)
               (encode-u32-le 2852389664)))
    part12 (concat-byte-vectors-rooted
             (concat-four-byte-vectors-rooted
               (encode-u32-le 3596551104)
               (encode-u32-le 2854159352)
               (encode-u32-le 402653176)
               (encode-u32-le 2853635040))
             (concat-four-byte-vectors-rooted
               (encode-u32-le 3531604176)
               (encode-u32-le 3556773889)
               (encode-u32-le 3036676469)
               (encode-u32-le 3531603968)))
    part13 (concat-byte-vectors-rooted
             (concat-four-byte-vectors-rooted
               (encode-u32-le 3533709313)
               (encode-u32-le 3531604066)
               (encode-u32-le 3531735107)
               (encode-u32-le 2457862148))
             (concat-four-byte-vectors-rooted
               (encode-u32-le 3531603973)
               (encode-u32-le 3531610288)
               (encode-u32-le 3556773889)
               (encode-u32-le 2852127733)))
    part14 (concat-byte-vectors-rooted
             (concat-four-byte-vectors-rooted
               (encode-u32-le 3533701174)
               (encode-u32-le 2853569504)
               (encode-u32-le 3531604226)
               (encode-u32-le 2332164822))
             (concat-four-byte-vectors-rooted
               (encode-u32-le 2332033699)
               (encode-u32-le 1384120356)
               (encode-u32-le 3103785060)
               (encode-u32-le 706675684)))
    part15 (concat-four-byte-vectors-rooted
             (encode-u32-le 3103786084)
             (encode-u32-le 3538944004)
             (encode-u32-le 2852388864)
             (encode-u32-le 3596551104))
    head1 (concat-five-byte-vectors-rooted part1 part2 part3 part4 part5)
    head2 (concat-five-byte-vectors-rooted part6 part7 part8 part9 part10)
    head3 (concat-five-byte-vectors-rooted part11 part12 part13 part14 part15)]
    (concat-three-byte-vectors-rooted head1 head2 head3)))

(defn append-aarch64-selfhost-string-concat-helper-rooted [result]
  (do
    (append-native-bytes-rooted result (emit-aarch64-selfhost-string-concat-helper) 308)
    0))

(defn append-aarch64-selfhost-read-file-helper-rooted [result]
  (do
    (root_push result)
    (let [native (emit-aarch64-selfhost-read-file-helper)]
      (do
        (root_push native)
        (append-native-bytes-rooted result native 464)
        (root_pop)
        (root_pop)
        0))))

(defn aarch64-import-stub-count [import-count]
  (if (> import-count 0) import-count 1))

(defn aarch64-import-ret-stub-offset [import-stub-offset import-count import-idx]
  (+ import-stub-offset (* import-idx 4)))

(defn aarch64-helper-base-offset [import-stub-offset import-count]
  (+ import-stub-offset (* (aarch64-import-stub-count import-count) 4)))

(defn aarch64-selfhost-command-line-arg-helper-offset [import-stub-offset import-count]
  (aarch64-helper-base-offset import-stub-offset import-count))

(defn aarch64-selfhost-string-length-helper-offset [import-stub-offset import-count]
  (+ (aarch64-helper-base-offset import-stub-offset import-count) 32))

(defn aarch64-selfhost-print-helper-offset [import-stub-offset import-count]
  (+ (aarch64-helper-base-offset import-stub-offset import-count) 92))

(defn aarch64-selfhost-vector-new-helper-offset [import-stub-offset import-count]
  (+ (aarch64-helper-base-offset import-stub-offset import-count) 236))

(defn aarch64-selfhost-vector-length-helper-offset [import-stub-offset import-count]
  (+ (aarch64-helper-base-offset import-stub-offset import-count) 344))

(defn aarch64-selfhost-alloc-helper-offset [import-stub-offset import-count]
  (+ (aarch64-helper-base-offset import-stub-offset import-count) 364))

(defn aarch64-selfhost-string-char-at-helper-offset [import-stub-offset import-count]
  (+ (aarch64-helper-base-offset import-stub-offset import-count) 436))

(defn aarch64-selfhost-vector-get-helper-offset [import-stub-offset import-count]
  (+ (aarch64-helper-base-offset import-stub-offset import-count) 488))

(defn aarch64-selfhost-vector-push-helper-offset [import-stub-offset import-count]
  (+ (aarch64-helper-base-offset import-stub-offset import-count) 540))

(defn aarch64-selfhost-ref-new-helper-offset [import-stub-offset import-count]
  (+ (aarch64-helper-base-offset import-stub-offset import-count) 796))

(defn aarch64-selfhost-ref-get-helper-offset [import-stub-offset import-count]
  (+ (aarch64-helper-base-offset import-stub-offset import-count) 892))

(defn aarch64-selfhost-ref-set-helper-offset [import-stub-offset import-count]
  (+ (aarch64-helper-base-offset import-stub-offset import-count) 912))

(defn aarch64-selfhost-substring-helper-offset [import-stub-offset import-count]
  (+ (aarch64-helper-base-offset import-stub-offset import-count) 936))

(defn aarch64-selfhost-string-concat-helper-offset [import-stub-offset import-count]
  (+ (aarch64-helper-base-offset import-stub-offset import-count) 1144))

(defn aarch64-selfhost-map-size-helper-offset [import-stub-offset import-count]
  (+ (aarch64-helper-base-offset import-stub-offset import-count) 1452))

(defn aarch64-selfhost-map-new-helper-offset [import-stub-offset import-count]
  (+ (aarch64-helper-base-offset import-stub-offset import-count) 1472))

(defn aarch64-selfhost-file-exists-helper-offset [import-stub-offset import-count]
  (+ (aarch64-helper-base-offset import-stub-offset import-count) 1572))

(defn aarch64-selfhost-read-file-helper-offset [import-stub-offset import-count]
  (+ (aarch64-helper-base-offset import-stub-offset import-count) 1768))

(defn aarch64-selfhost-map-insert-helper-offset [import-stub-offset import-count]
  (+ (aarch64-helper-base-offset import-stub-offset import-count) 2232))

(defn aarch64-selfhost-map-get-helper-offset [import-stub-offset import-count]
  (+ (aarch64-helper-base-offset import-stub-offset import-count) 2328))

(defn aarch64-selfhost-map-new-fixed-helper-offset [import-stub-offset import-count]
  (+ (aarch64-helper-base-offset import-stub-offset import-count) 2388))

(defn aarch64-selfhost-helper-trailer-size [import-count]
  (+ (aarch64-selfhost-map-new-fixed-helper-offset 0 import-count) 92))

(defn aarch64-bundle-initial-capacity [import-stub-offset import-count]
  (+ import-stub-offset (aarch64-selfhost-helper-trailer-size import-count)))

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

;; AArch64 B imm26
(defn emit-aarch64-b [byte-disp]
  (let [word-disp (/ byte-disp 4)
    imm26 (if (< word-disp 0)
            (+ 67108864 word-disp)
            word-disp)]
    (encode-u32-le (+ 335544320 imm26))))

;; AArch64 CBZ x0, imm19
(defn emit-aarch64-cbz-x0 [byte-disp]
  (let [word-disp (/ byte-disp 4)
    imm19 (if (< word-disp 0)
            (+ 524288 word-disp)
            word-disp)]
    (encode-u32-le (+ 3019898880 (* imm19 32)))))

;; AArch64 CBNZ x0, imm19
(defn emit-aarch64-cbnz-x0 [byte-disp]
  (let [word-disp (/ byte-disp 4)
    imm19 (if (< word-disp 0)
            (+ 524288 word-disp)
            word-disp)]
    (encode-u32-le (+ 3036676096 (* imm19 32)))))

;; AArch64 CMP x0, #0
(defn emit-aarch64-cmp-x0-zero []
  (let [bytes (vector-new 4)]
    (vector-push (vector-push (vector-push (vector-push bytes 31) 0) 0) 241)))

;; AArch64 B.cond imm19
(defn emit-aarch64-b-cond [byte-disp cond]
  (let [word-disp (/ byte-disp 4)
    imm19 (if (< word-disp 0)
            (+ 524288 word-disp)
            word-disp)]
    (encode-u32-le (+ (+ 1409286144 (* imm19 32)) cond))))

(defn emit-aarch64-b-eq [byte-disp]
  (emit-aarch64-b-cond byte-disp 0))

(defn emit-aarch64-b-ne [byte-disp]
  (emit-aarch64-b-cond byte-disp 1))

;; AArch64 NOP 命令
;; エンコーディング: 0xD503201F → [0x1F, 0x20, 0x03, 0xD5]
(defn emit-aarch64-nop []
  (let [bytes (vector-new 4)]
    (vector-push (vector-push (vector-push (vector-push bytes 31) 32) 3) 213)))

;; AArch64 MOV x1, x0
(defn emit-aarch64-mov-x1-x0 []
  (let [bytes (vector-new 4)]
    (vector-push (vector-push (vector-push (vector-push bytes 225) 3) 0) 170)))

;; AArch64 MOV x1, x30
(defn emit-aarch64-mov-x1-x30 []
  (let [bytes (vector-new 4)]
    (vector-push (vector-push (vector-push (vector-push bytes 225) 3) 30) 170)))

;; AArch64 MOV x17, x30
(defn emit-aarch64-mov-x17-x30 []
  (let [bytes (vector-new 4)]
    (vector-push (vector-push (vector-push (vector-push bytes 241) 3) 30) 170)))

;; AArch64 STP x9, x30, [sp, #-16]!
(defn emit-aarch64-save-x9-x30 []
  (let [bytes (vector-new 4)]
    (vector-push (vector-push (vector-push (vector-push bytes 233) 123) 191) 169)))

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

;; AArch64 MOV x30, x1
(defn emit-aarch64-mov-x30-x1 []
  (let [bytes (vector-new 4)]
    (vector-push (vector-push (vector-push (vector-push bytes 254) 3) 1) 170)))

;; AArch64 MOV x30, x17
(defn emit-aarch64-mov-x30-x17 []
  (let [bytes (vector-new 4)]
    (vector-push (vector-push (vector-push (vector-push bytes 254) 3) 17) 170)))

;; AArch64 LDP x9, x30, [sp], #16
(defn emit-aarch64-restore-x9-x30 []
  (let [bytes (vector-new 4)]
    (vector-push (vector-push (vector-push (vector-push bytes 233) 123) 193) 168)))

;; AArch64 MOV x0, x9
(defn emit-aarch64-mov-x0-x9 []
  (let [bytes (vector-new 4)]
    (vector-push (vector-push (vector-push (vector-push bytes 224) 3) 9) 170)))

;; AArch64 MOV x0, x27
(defn emit-aarch64-mov-x0-x27 []
  (encode-u32-le 2853897184))

;; AArch64 MOV x28, x27
(defn emit-aarch64-mov-x28-x27 []
  (encode-u32-le 2853897212))

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

;; AArch64 ADD x0, x9, x0
(defn emit-aarch64-add-x0-x9-x0 []
  (let [bytes (vector-new 4)]
    (vector-push (vector-push (vector-push (vector-push bytes 32) 1) 0) 139)))

;; AArch64 ADD x0, x21, x0
(defn emit-aarch64-add-x0-x21-x0 []
  (encode-u32-le 2332033696))

;; AArch64 ADD x1, x21, x1
(defn emit-aarch64-add-x1-x21-x1 []
  (encode-u32-le 2332099233))

;; AArch64 ADD x2, x21, x2
(defn emit-aarch64-add-x2-x21-x2 []
  (encode-u32-le 2332164770))

;; AArch64 ADD x9, x21, x9
(defn emit-aarch64-add-x9-x21-x9 []
  (encode-u32-le 2332623529))

;; AArch64 MUL w0, w1, w0
(defn emit-aarch64-mul-w0-w1-w0 []
  (let [bytes (vector-new 4)]
    (vector-push (vector-push (vector-push (vector-push bytes 32) 124) 0) 27)))

;; AArch64 MUL w0, w9, w0
(defn emit-aarch64-mul-w0-w9-w0 []
  (let [bytes (vector-new 4)]
    (vector-push (vector-push (vector-push (vector-push bytes 32) 125) 0) 27)))

;; AArch64 MUL x0, x9, x0
(defn emit-aarch64-mul-x0-x9-x0 []
  (let [bytes (vector-new 4)]
    (vector-push (vector-push (vector-push (vector-push bytes 32) 125) 0) 155)))

;; AArch64 SDIV x0, x9, x0
(defn emit-aarch64-sdiv-x0-x9-x0 []
  (let [bytes (vector-new 4)]
    (vector-push (vector-push (vector-push (vector-push bytes 32) 13) 192) 154)))

;; AArch64 SDIV x10, x9, x0
(defn emit-aarch64-sdiv-x10-x9-x0 []
  (let [bytes (vector-new 4)]
    (vector-push (vector-push (vector-push (vector-push bytes 42) 13) 192) 154)))

;; AArch64 MSUB x0, x10, x1, x9
(defn emit-aarch64-msub-x0-x10-x1-x9 []
  (let [bytes (vector-new 4)]
    (vector-push (vector-push (vector-push (vector-push bytes 64) 165) 1) 155)))

;; AArch64 の i64.rem_s (x9 % x0)
(defn emit-aarch64-rem-x0-x9-x0 []
  (concat-byte-vectors
    (concat-byte-vectors
      (emit-aarch64-mov-x1-x0)
      (emit-aarch64-sdiv-x10-x9-x0))
    (emit-aarch64-msub-x0-x10-x1-x9)))

;; AArch64 AND w0, w9, w0
(defn emit-aarch64-and-w0-w9-w0 []
  (let [bytes (vector-new 4)]
    (vector-push (vector-push (vector-push (vector-push bytes 32) 1) 0) 10)))

;; AArch64 ORR w0, w9, w0
(defn emit-aarch64-orr-w0-w9-w0 []
  (let [bytes (vector-new 4)]
    (vector-push (vector-push (vector-push (vector-push bytes 32) 1) 0) 42)))

;; AArch64 SUB x0, x9, x0
(defn emit-aarch64-sub-x0-x9-x0 []
  (let [bytes (vector-new 4)]
    (vector-push (vector-push (vector-push (vector-push bytes 32) 1) 0) 203)))

;; AArch64 CMP x9, x0
(defn emit-aarch64-cmp-x9-x0 []
  (let [bytes (vector-new 4)]
    (vector-push (vector-push (vector-push (vector-push bytes 63) 1) 0) 235)))

;; AArch64 CSET w0, eq
(defn emit-aarch64-cset-w0-eq []
  (let [bytes (vector-new 4)]
    (vector-push (vector-push (vector-push (vector-push bytes 224) 23) 159) 26)))

;; AArch64 CSET w0, ne
(defn emit-aarch64-cset-w0-ne []
  (let [bytes (vector-new 4)]
    (vector-push (vector-push (vector-push (vector-push bytes 224) 7) 159) 26)))

;; AArch64 CSET w0, lt
(defn emit-aarch64-cset-w0-lt []
  (let [bytes (vector-new 4)]
    (vector-push (vector-push (vector-push (vector-push bytes 224) 167) 159) 26)))

;; AArch64 CSET w0, gt
(defn emit-aarch64-cset-w0-gt []
  (let [bytes (vector-new 4)]
    (vector-push (vector-push (vector-push (vector-push bytes 224) 215) 159) 26)))

;; AArch64 CSET w0, le
(defn emit-aarch64-cset-w0-le []
  (let [bytes (vector-new 4)]
    (vector-push (vector-push (vector-push (vector-push bytes 224) 199) 159) 26)))

;; AArch64 CSET w0, ge
(defn emit-aarch64-cset-w0-ge []
  (let [bytes (vector-new 4)]
    (vector-push (vector-push (vector-push (vector-push bytes 224) 183) 159) 26)))

(defn emit-aarch64-compare [cset]
  (concat-byte-vectors
    (emit-aarch64-cmp-x9-x0)
    cset))

(defn emit-aarch64-i64-eq []
  (emit-aarch64-compare (emit-aarch64-cset-w0-eq)))

(defn emit-aarch64-i64-ne []
  (emit-aarch64-compare (emit-aarch64-cset-w0-ne)))

(defn emit-aarch64-i64-lt []
  (emit-aarch64-compare (emit-aarch64-cset-w0-lt)))

(defn emit-aarch64-i64-gt []
  (emit-aarch64-compare (emit-aarch64-cset-w0-gt)))

(defn emit-aarch64-i64-le []
  (emit-aarch64-compare (emit-aarch64-cset-w0-le)))

(defn emit-aarch64-i64-ge []
  (emit-aarch64-compare (emit-aarch64-cset-w0-ge)))

(defn emit-i64-compare-aarch64 [opcode]
  (if (= opcode 30)
    (emit-aarch64-i64-eq)
    (if (= opcode 31)
      (emit-aarch64-i64-ne)
      (if (= opcode 32)
        (emit-aarch64-i64-lt)
        (if (= opcode 33)
          (emit-aarch64-i64-gt)
          (if (= opcode 34)
            (emit-aarch64-i64-le)
            (emit-aarch64-i64-ge)))))))

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

;; AArch64 ADD x27, x27, #8
(defn emit-aarch64-add-x27-x27-8 []
  (encode-u32-le 2432705403))

;; AArch64 ADD x10, x28, x9, LSL #3
(defn emit-aarch64-add-x10-x28-x9-lsl3 []
  (encode-u32-le 2332626826))

;; AArch64 SUB x27, x27, #8
(defn emit-aarch64-sub-x27-x27-8 []
  (encode-u32-le 3506447227))

;; AArch64 SUB x0, x27, x28
(defn emit-aarch64-sub-x0-x27-x28 []
  (encode-u32-le 3407610720))

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

;; AArch64 LDR x0, [x0, #offset]
(defn emit-aarch64-ldr-x0-x0 [offset]
  (let [scaled (/ offset 8)]
    (encode-u32-le (+ 4181721088 (* scaled 1024)))))

;; AArch64 LDR x0, [x27]
(defn emit-aarch64-ldr-x0-x27 []
  (encode-u32-le 4181721952))

;; AArch64 LSR x0, x0, #3
(defn emit-aarch64-lsr-x0-x0-3 []
  (encode-u32-le 3544448000))

;; AArch64 LDR w0, [x0, #offset]
(defn emit-aarch64-ldr-w0-x0 [offset]
  (let [scaled (/ offset 4)]
    (encode-u32-le (+ 3107979264 (* scaled 1024)))))

;; AArch64 LDRB w0, [x0, #offset]
(defn emit-aarch64-ldrb-w0-x0 [offset]
  (encode-u32-le (+ 960495616 (* offset 1024))))

;; AArch64 STR x0, [x9, #offset]
(defn emit-aarch64-str-x0-x9 [offset]
  (let [scaled (/ offset 8)]
    (encode-u32-le (+ 4177527072 (* scaled 1024)))))

;; AArch64 STR x0, [x27]
(defn emit-aarch64-str-x0-x27 []
  (encode-u32-le 4177527648))

;; AArch64 STR x0, [x10]
(defn emit-aarch64-str-x0-x10 []
  (encode-u32-le 4177527104))

;; AArch64 STR w0, [x9, #offset]
(defn emit-aarch64-str-w0-x9 [offset]
  (let [scaled (/ offset 4)]
    (encode-u32-le (+ 3103785248 (* scaled 1024)))))

(defn emit-aarch64-cbz-x3-fill-end []
  (vector-push (vector-push (vector-push (vector-push (vector-new 4) 131) 0) 0) 180))

(defn emit-aarch64-cbnz-x3-fill-loop []
  (vector-push (vector-push (vector-push (vector-push (vector-new 4) 195) 255) 255) 181))

(defn emit-aarch64-cbz-x3-copy-end []
  (vector-push (vector-push (vector-push (vector-push (vector-new 4) 163) 0) 0) 180))

(defn emit-aarch64-cbnz-x3-copy-loop []
  (vector-push (vector-push (vector-push (vector-push (vector-new 4) 163) 255) 255) 181))

(defn emit-aarch64-strb-w2-x1-post1 []
  (vector-push (vector-push (vector-push (vector-push (vector-new 4) 34) 20) 0) 56))

(defn emit-aarch64-ldrb-w4-x2-post1 []
  (vector-push (vector-push (vector-push (vector-push (vector-new 4) 68) 20) 64) 56))

(defn emit-aarch64-strb-w4-x1-post1 []
  (vector-push (vector-push (vector-push (vector-push (vector-new 4) 36) 20) 0) 56))

(defn emit-aarch64-sub-x3-x3-1 []
  (vector-push (vector-push (vector-push (vector-push (vector-new 4) 99) 4) 0) 209))

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
  (concat-byte-vectors-rooted
    (emit-aarch64-mov-x9-x0)
    (emit-aarch64-load-u32-w0 value)))

;; AArch64 bundle の i32.const: spill window が必要なら old previous を spill する
(defn spill-native-value-window-one-step-aarch64 [frame-base-slot-count current-depth]
  (concat-byte-vectors
    (emit-aarch64-ldr-x10-sp (native-value-window-spill-offset frame-base-slot-count (- current-depth 3)))
    (emit-aarch64-str-x10-sp (native-value-window-spill-offset frame-base-slot-count (- current-depth 2)))))

(defn emit-produce-one-bundle-aarch64 [op-bytes frame-base-slot-count current-depth]
  (if (>= current-depth 3)
    (concat-byte-vectors
      (spill-native-value-window-one-step-aarch64 frame-base-slot-count current-depth)
      (emit-produce-one-bundle-aarch64 op-bytes frame-base-slot-count (- current-depth 1)))
    (if (= current-depth 2)
      (concat-byte-vectors
        (emit-aarch64-str-x9-sp (native-value-window-spill-offset frame-base-slot-count 0))
        (concat-byte-vectors
          (emit-aarch64-mov-x9-x0)
          op-bytes))
      (if (= current-depth 1)
        (concat-byte-vectors
          (emit-aarch64-mov-x9-x0)
          op-bytes)
        op-bytes))))

(defn append-produce-one-spills-aarch64 [result frame-base-slot-count depth]
  (if (< depth 3)
    0
    (do
      (append-native-bytes-rooted result (emit-aarch64-ldr-x10-sp (native-value-window-spill-offset frame-base-slot-count (- depth 3))) 4)
      (append-native-bytes-rooted result (emit-aarch64-str-x10-sp (native-value-window-spill-offset frame-base-slot-count (- depth 2))) 4)
      (append-produce-one-spills-aarch64 result frame-base-slot-count (- depth 1)))))

(defn append-produce-one-bundle-aarch64 [result op-bytes frame-base-slot-count current-depth]
  (do
    (root_push op-bytes)
    (append-produce-one-spills-aarch64 result frame-base-slot-count current-depth)
    (if (>= current-depth 2)
      (do
        (append-native-bytes-rooted result (emit-aarch64-str-x9-sp (native-value-window-spill-offset frame-base-slot-count 0)) 4)
        (append-native-bytes-rooted result (emit-aarch64-mov-x9-x0) 4))
      (if (= current-depth 1)
        (append-native-bytes-rooted result (emit-aarch64-mov-x9-x0) 4)
        0))
    (append-native-bytes-loop result op-bytes 0 (vector-length op-bytes))
    (root_pop)
    0))

(defn append-local-get-bundle-aarch64 [result offset frame-base-slot-count current-depth]
  (do
    (append-produce-one-spills-aarch64 result frame-base-slot-count current-depth)
    (if (>= current-depth 2)
      (do
        (append-native-bytes-rooted result (emit-aarch64-str-x9-sp (native-value-window-spill-offset frame-base-slot-count 0)) 4)
        (append-native-bytes-rooted result (emit-aarch64-mov-x9-x0) 4))
      (if (= current-depth 1)
        (append-native-bytes-rooted result (emit-aarch64-mov-x9-x0) 4)
        0))
    (append-native-bytes-rooted result (emit-aarch64-ldr-x0-sp offset) 4)
    0))

(defn emit-i64-const-bundle-aarch64 [value frame-base-slot-count current-depth]
  (emit-produce-one-bundle-aarch64
    (emit-aarch64-load-i64-x0 value)
    frame-base-slot-count
    current-depth))

(defn emit-i32-const-bundle-aarch64 [value frame-base-slot-count current-depth]
  (emit-i32-const-bundle-aarch64-core value frame-base-slot-count current-depth))

(defn emit-i32-const-bundle-aarch64-core [value frame-base-slot-count current-depth]
  (emit-produce-one-bundle-aarch64
    (emit-aarch64-load-u32-w0 value)
    frame-base-slot-count
    current-depth))

;; AArch64 bundle の local.get: spill window が必要なら old previous を spill する
(defn emit-local-get-bundle-aarch64 [offset frame-base-slot-count current-depth]
  (emit-local-get-bundle-aarch64-core offset frame-base-slot-count current-depth))

(defn emit-local-get-bundle-aarch64-core [offset frame-base-slot-count current-depth]
  (emit-produce-one-bundle-aarch64
    (emit-aarch64-ldr-x0-sp offset)
    frame-base-slot-count
    current-depth))

(defn emit-twenty-six-arg-call-aarch64 [disp frame-base-slot-count]
  (emit-high-arg-call-aarch64 26 disp frame-base-slot-count))
(defn emit-twenty-seven-arg-call-aarch64 [disp frame-base-slot-count]
  (emit-high-arg-call-aarch64 27 disp frame-base-slot-count))
(defn emit-twenty-eight-arg-call-aarch64 [disp frame-base-slot-count]
  (emit-high-arg-call-aarch64 28 disp frame-base-slot-count))
(defn emit-twenty-nine-arg-call-aarch64 [disp frame-base-slot-count]
  (emit-high-arg-call-aarch64 29 disp frame-base-slot-count))
(defn emit-thirty-arg-call-aarch64 [disp frame-base-slot-count]
  (emit-high-arg-call-aarch64 30 disp frame-base-slot-count))
(defn emit-thirty-one-arg-call-aarch64 [disp frame-base-slot-count]
  (emit-high-arg-call-aarch64 31 disp frame-base-slot-count))
(defn emit-thirty-two-arg-call-aarch64 [disp frame-base-slot-count]
  (emit-high-arg-call-aarch64 32 disp frame-base-slot-count))
(defn emit-thirty-three-arg-call-aarch64 [disp frame-base-slot-count]
  (emit-high-arg-call-aarch64 33 disp frame-base-slot-count))
(defn emit-thirty-four-arg-call-aarch64 [disp frame-base-slot-count]
  (emit-high-arg-call-aarch64 34 disp frame-base-slot-count))
(defn emit-thirty-five-arg-call-aarch64 [disp frame-base-slot-count]
  (emit-high-arg-call-aarch64 35 disp frame-base-slot-count))
(defn emit-thirty-six-arg-call-aarch64 [disp frame-base-slot-count]
  (emit-high-arg-call-aarch64 36 disp frame-base-slot-count))
(defn emit-thirty-seven-arg-call-aarch64 [disp frame-base-slot-count]
  (emit-high-arg-call-aarch64 37 disp frame-base-slot-count))
(defn emit-thirty-eight-arg-call-aarch64 [disp frame-base-slot-count]
  (emit-high-arg-call-aarch64 38 disp frame-base-slot-count))
(defn emit-thirty-nine-arg-call-aarch64 [disp frame-base-slot-count]
  (emit-high-arg-call-aarch64 39 disp frame-base-slot-count))
(defn emit-forty-arg-call-aarch64 [disp frame-base-slot-count]
  (emit-high-arg-call-aarch64 40 disp frame-base-slot-count))
(defn emit-forty-one-arg-call-aarch64 [disp frame-base-slot-count]
  (emit-high-arg-call-aarch64 41 disp frame-base-slot-count))
(defn emit-forty-two-arg-call-aarch64 [disp frame-base-slot-count]
  (emit-high-arg-call-aarch64 42 disp frame-base-slot-count))
(defn emit-forty-three-arg-call-aarch64 [disp frame-base-slot-count]
  (emit-high-arg-call-aarch64 43 disp frame-base-slot-count))
(defn emit-forty-four-arg-call-aarch64 [disp frame-base-slot-count]
  (emit-high-arg-call-aarch64 44 disp frame-base-slot-count))
(defn emit-forty-five-arg-call-aarch64 [disp frame-base-slot-count]
  (emit-high-arg-call-aarch64 45 disp frame-base-slot-count))
(defn emit-forty-six-arg-call-aarch64 [disp frame-base-slot-count]
  (emit-high-arg-call-aarch64 46 disp frame-base-slot-count))
(defn emit-forty-seven-arg-call-aarch64 [disp frame-base-slot-count]
  (emit-high-arg-call-aarch64 47 disp frame-base-slot-count))
(defn emit-forty-eight-arg-call-aarch64 [disp frame-base-slot-count]
  (emit-high-arg-call-aarch64 48 disp frame-base-slot-count))
(defn emit-forty-nine-arg-call-aarch64 [disp frame-base-slot-count]
  (emit-high-arg-call-aarch64 49 disp frame-base-slot-count))
(defn emit-fifty-arg-call-aarch64 [disp frame-base-slot-count]
  (emit-high-arg-call-aarch64 50 disp frame-base-slot-count))
(defn emit-fifty-one-arg-call-aarch64 [disp frame-base-slot-count]
  (emit-high-arg-call-aarch64 51 disp frame-base-slot-count))
(defn emit-fifty-two-arg-call-aarch64 [disp frame-base-slot-count]
  (emit-high-arg-call-aarch64 52 disp frame-base-slot-count))
(defn emit-fifty-three-arg-call-aarch64 [disp frame-base-slot-count]
  (emit-high-arg-call-aarch64 53 disp frame-base-slot-count))
(defn emit-fifty-four-arg-call-aarch64 [disp frame-base-slot-count]
  (emit-high-arg-call-aarch64 54 disp frame-base-slot-count))
(defn emit-fifty-five-arg-call-aarch64 [disp frame-base-slot-count]
  (emit-high-arg-call-aarch64 55 disp frame-base-slot-count))
(defn emit-fifty-six-arg-call-aarch64 [disp frame-base-slot-count]
  (emit-high-arg-call-aarch64 56 disp frame-base-slot-count))
(defn emit-fifty-seven-arg-call-aarch64 [disp frame-base-slot-count]
  (emit-high-arg-call-aarch64 57 disp frame-base-slot-count))
(defn emit-fifty-eight-arg-call-aarch64 [disp frame-base-slot-count]
  (emit-high-arg-call-aarch64 58 disp frame-base-slot-count))
(defn emit-fifty-nine-arg-call-aarch64 [disp frame-base-slot-count]
  (emit-high-arg-call-aarch64 59 disp frame-base-slot-count))
(defn emit-sixty-arg-call-aarch64 [disp frame-base-slot-count]
  (emit-high-arg-call-aarch64 60 disp frame-base-slot-count))

(defn emit-aarch64-window-stack-arg-spills [frame-base-slot-count spill-index slot-offset base-offset]
  (if (< spill-index 0)
    (vector-new 0)
    (concat-byte-vectors
      (concat-byte-vectors
        (emit-aarch64-ldr-x10-sp (+ base-offset (native-value-window-spill-offset frame-base-slot-count spill-index)))
        (emit-aarch64-str-x10-sp slot-offset))
      (emit-aarch64-window-stack-arg-spills frame-base-slot-count (- spill-index 1) (+ slot-offset 8) base-offset))))

(defn emit-twenty-plus-arg-call-aarch64 [target-param-count disp frame-base-slot-count]
  (emit-high-arg-call-aarch64 target-param-count disp frame-base-slot-count))

(defn emit-sixty-one-arg-call-aarch64 [disp frame-base-slot-count]
  (emit-high-arg-call-aarch64 61 disp frame-base-slot-count))

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

(defn emit-aarch64-window-reg-load [reg-index offset]
  (if (= reg-index 7)
    (emit-aarch64-ldr-x7-sp offset)
    (if (= reg-index 6)
      (emit-aarch64-ldr-x6-sp offset)
      (if (= reg-index 5)
        (emit-aarch64-ldr-x5-sp offset)
        (if (= reg-index 4)
          (emit-aarch64-ldr-x4-sp offset)
          (if (= reg-index 3)
            (emit-aarch64-ldr-x3-sp offset)
            (if (= reg-index 2)
              (emit-aarch64-ldr-x2-sp offset)
              (if (= reg-index 1)
                (emit-aarch64-ldr-x1-sp offset)
                (emit-aarch64-ldr-x0-sp offset)))))))))

(defn emit-aarch64-window-reg-setup [frame-base-slot-count base-offset local-index reg-index]
  (if (< reg-index 0)
    (vector-new 0)
    (concat-byte-vectors
      (emit-aarch64-window-reg-load reg-index (+ base-offset (native-value-window-spill-offset frame-base-slot-count local-index)))
      (emit-aarch64-window-reg-setup frame-base-slot-count base-offset (+ local-index 1) (- reg-index 1)))))

(defn emit-aarch64-high-arg-stack-setup [frame-base-slot-count stack-arg-count stack-arg-bytes stack-bytes]
  (let [stack-base (emit-aarch64-sub-sp stack-bytes)]
    (if (= stack-arg-count 1)
      (concat-byte-vectors stack-base (emit-aarch64-str-x0-sp 0))
      (let [stack-head (if (> stack-arg-count 2)
                         (let [first-stack-local-index (- stack-arg-count 3)
                               stack-body-last (- stack-arg-count 4)
                               first-spill (concat-byte-vectors
                                             (concat-byte-vectors
                                               stack-base
                                               (emit-aarch64-ldr-x10-sp (+ stack-bytes (native-value-window-spill-offset frame-base-slot-count first-stack-local-index))))
                                             (emit-aarch64-str-x10-sp 0))]
                           (concat-byte-vectors
                             first-spill
                             (emit-aarch64-window-stack-arg-spills frame-base-slot-count stack-body-last 8 stack-bytes)))
                         stack-base)
            with-x9 (concat-byte-vectors stack-head (emit-aarch64-str-x9-sp (- stack-arg-bytes 16)))]
        (concat-byte-vectors with-x9 (emit-aarch64-str-x0-sp (- stack-arg-bytes 8)))))))

(defn emit-aarch64-high-arg-reg-setup [frame-base-slot-count stack-arg-count stack-bytes]
  (if (= stack-arg-count 1)
    (concat-byte-vectors
      (emit-aarch64-mov-x7-x9)
      (emit-aarch64-window-reg-setup frame-base-slot-count stack-bytes 0 6))
    (emit-aarch64-window-reg-setup frame-base-slot-count stack-bytes (- stack-arg-count 2) 7)))

(defn emit-high-arg-call-aarch64 [target-param-count disp frame-base-slot-count]
  (let [stack-arg-count (- target-param-count 8)
        stack-arg-bytes (* stack-arg-count 8)
        stack-bytes (align-16 stack-arg-bytes)
        stack-setup (emit-aarch64-high-arg-stack-setup frame-base-slot-count stack-arg-count stack-arg-bytes stack-bytes)
        reg-setup (emit-aarch64-high-arg-reg-setup frame-base-slot-count stack-arg-count stack-bytes)
        call-seq (concat-byte-vectors
                   (emit-aarch64-bl disp)
                   (emit-aarch64-add-sp stack-bytes))]
    (concat-byte-vectors
      (concat-byte-vectors stack-setup reg-setup)
      call-seq)))

(defn emit-nine-arg-call-aarch64 [disp frame-base-slot-count]
  (emit-high-arg-call-aarch64 9 disp frame-base-slot-count))

(defn emit-ten-arg-call-aarch64 [disp frame-base-slot-count]
  (emit-high-arg-call-aarch64 10 disp frame-base-slot-count))

(defn emit-eleven-arg-call-aarch64 [disp frame-base-slot-count]
  (emit-high-arg-call-aarch64 11 disp frame-base-slot-count))

(defn emit-twelve-arg-call-aarch64 [disp frame-base-slot-count]
  (emit-high-arg-call-aarch64 12 disp frame-base-slot-count))

(defn emit-thirteen-arg-call-aarch64 [disp frame-base-slot-count]
  (emit-high-arg-call-aarch64 13 disp frame-base-slot-count))

(defn emit-fourteen-arg-call-aarch64 [disp frame-base-slot-count]
  (emit-high-arg-call-aarch64 14 disp frame-base-slot-count))

(defn emit-fifteen-arg-call-aarch64 [disp frame-base-slot-count]
  (emit-high-arg-call-aarch64 15 disp frame-base-slot-count))

(defn emit-sixteen-arg-call-aarch64 [disp frame-base-slot-count]
  (emit-high-arg-call-aarch64 16 disp frame-base-slot-count))

(defn emit-seventeen-arg-call-aarch64 [disp frame-base-slot-count]
  (emit-high-arg-call-aarch64 17 disp frame-base-slot-count))

(defn emit-eighteen-arg-call-aarch64 [disp frame-base-slot-count]
  (emit-high-arg-call-aarch64 18 disp frame-base-slot-count))

(defn emit-nineteen-arg-call-aarch64 [disp frame-base-slot-count]
  (emit-high-arg-call-aarch64 19 disp frame-base-slot-count))

(defn emit-twenty-arg-call-aarch64 [disp frame-base-slot-count]
  (emit-high-arg-call-aarch64 20 disp frame-base-slot-count))

(defn emit-twenty-one-arg-call-aarch64 [disp frame-base-slot-count]
  (emit-high-arg-call-aarch64 21 disp frame-base-slot-count))

(defn emit-twenty-two-arg-call-aarch64 [disp frame-base-slot-count]
  (emit-high-arg-call-aarch64 22 disp frame-base-slot-count))

(defn emit-twenty-three-arg-call-aarch64 [disp frame-base-slot-count]
  (emit-high-arg-call-aarch64 23 disp frame-base-slot-count))

(defn emit-twenty-four-arg-call-aarch64 [disp frame-base-slot-count]
  (emit-high-arg-call-aarch64 24 disp frame-base-slot-count))

(defn emit-twenty-five-arg-call-aarch64 [disp frame-base-slot-count]
  (emit-high-arg-call-aarch64 25 disp frame-base-slot-count))

(defn emit-two-arg-call-aarch64 [disp frame-base-slot-count current-depth]
  (let [call-seq (concat-byte-vectors
                   (concat-byte-vectors
                     (emit-aarch64-mov-x1-x0)
                     (emit-aarch64-mov-x0-x9))
                   (emit-aarch64-bl disp))]
    (emit-consume-produce-one-bundle-aarch64 call-seq frame-base-slot-count current-depth 2)))

(defn emit-drop-window-spill-shifts-aarch64-step [frame-base-slot-count result shift-idx last-shift-idx]
  (if (> shift-idx last-shift-idx)
    (make-native-progress-state 1 shift-idx)
    (do
      (append-native-bytes-rooted result (emit-aarch64-ldr-x1-sp (native-value-window-spill-offset frame-base-slot-count shift-idx)) 4)
      (append-native-bytes-rooted result (emit-aarch64-str-x1-sp (native-value-window-spill-offset frame-base-slot-count (- shift-idx 1))) 4)
      (make-native-progress-state 0 (+ shift-idx 1)))))

(defn emit-drop-window-spill-shifts-aarch64-step-64-loop-bounded [frame-base-slot-count result shift-idx last-shift-idx remaining]
  (do
    (root_push result)
    (let [state (emit-drop-window-spill-shifts-aarch64-step frame-base-slot-count result shift-idx last-shift-idx)]
      (do
        (root_push state)
        (let [final
              (if (= (vector-get state 0) 1)
                state
                (if (<= remaining 1)
                  state
                  (emit-drop-window-spill-shifts-aarch64-step-64-loop-bounded frame-base-slot-count result (vector-get state 1) last-shift-idx (- remaining 1))))]
          (do
            (root_pop)
            (root_pop)
            final))))))

(defn emit-drop-window-spill-shifts-aarch64-step-64 [frame-base-slot-count result shift-idx last-shift-idx]
  (emit-drop-window-spill-shifts-aarch64-step-64-loop-bounded frame-base-slot-count result shift-idx last-shift-idx 64))

(defn continue-emit-drop-window-spill-shifts-aarch64-step-64 [frame-base-slot-count result last-shift-idx state]
  (if (= (vector-get state 0) 1)
    state
    (do
      (root_push result)
      (root_push state)
      (let [next-state (emit-drop-window-spill-shifts-aarch64-step-64 frame-base-slot-count result (vector-get state 1) last-shift-idx)]
        (do
          (root_push next-state)
          (let [final (continue-emit-drop-window-spill-shifts-aarch64-step-64 frame-base-slot-count result last-shift-idx next-state)]
            (do
              (root_pop)
              (root_pop)
              (root_pop)
              final)))))))

(defn emit-drop-window-spill-shifts-aarch64 [frame-base-slot-count shift-idx last-shift-idx]
  (let [result (ref-new (vector-new 64))]
    (do
      (root_push result)
      (continue-emit-drop-window-spill-shifts-aarch64-step-64
        frame-base-slot-count
        result
        last-shift-idx
        (emit-drop-window-spill-shifts-aarch64-step-64 frame-base-slot-count result shift-idx last-shift-idx))
      (let [final (ref-get result)]
        (do
          (root_pop)
          final)))))

(defn emit-drop-bundle-aarch64 [frame-base-slot-count current-depth]
  (if (>= current-depth 3)
    (concat-byte-vectors
      (concat-byte-vectors
        (emit-aarch64-mov-x0-x9)
        (emit-aarch64-ldr-x9-sp (native-value-window-spill-offset frame-base-slot-count 0)))
      (emit-drop-window-spill-shifts-aarch64 frame-base-slot-count 1 (- current-depth 3)))
    (emit-aarch64-mov-x0-x9)))

(defn append-drop-window-spill-shifts-aarch64 [result frame-base-slot-count shift-idx last-shift-idx]
  (do
    (root_push result)
    (continue-emit-drop-window-spill-shifts-aarch64-step-64
      frame-base-slot-count
      result
      last-shift-idx
      (emit-drop-window-spill-shifts-aarch64-step-64 frame-base-slot-count result shift-idx last-shift-idx))
    (root_pop)
    0))

(defn append-consume-two-bundle-aarch64 [result op-bytes frame-base-slot-count current-depth]
  (do
    (root_push op-bytes)
    (append-native-bytes-loop result op-bytes 0 (vector-length op-bytes))
    (root_pop)
    (if (>= current-depth 3)
      (do
        (append-native-bytes-rooted result (emit-aarch64-ldr-x9-sp (native-value-window-spill-offset frame-base-slot-count 0)) 4)
        (append-drop-window-spill-shifts-aarch64 result frame-base-slot-count 1 (- current-depth 3)))
      0)))

(defn emit-local-set-bundle-aarch64 [offset frame-base-slot-count current-depth]
  (if (>= current-depth 3)
    (concat-byte-vectors
      (concat-byte-vectors
        (concat-byte-vectors
          (emit-aarch64-str-x0-sp offset)
          (emit-aarch64-mov-x0-x9))
        (emit-aarch64-ldr-x9-sp (native-value-window-spill-offset frame-base-slot-count 0)))
      (emit-drop-window-spill-shifts-aarch64 frame-base-slot-count 1 (- current-depth 3)))
    (if (= current-depth 2)
      (concat-byte-vectors
        (emit-aarch64-str-x0-sp offset)
        (emit-aarch64-mov-x0-x9))
      (emit-aarch64-str-x0-sp offset))))

(defn append-local-set-bundle-aarch64 [result offset frame-base-slot-count current-depth]
  (do
    (append-native-bytes-rooted result (emit-aarch64-str-x0-sp offset) 4)
    (if (>= current-depth 3)
      (do
        (append-native-bytes-rooted result (emit-aarch64-mov-x0-x9) 4)
        (append-native-bytes-rooted result (emit-aarch64-ldr-x9-sp (native-value-window-spill-offset frame-base-slot-count 0)) 4)
        (append-drop-window-spill-shifts-aarch64 result frame-base-slot-count 1 (- current-depth 3)))
      (if (= current-depth 2)
        (append-native-bytes-rooted result (emit-aarch64-mov-x0-x9) 4)
        0))))

(defn emit-root-set-bundle-aarch64 [frame-base-slot-count current-depth]
  (emit-consume-two-bundle-aarch64
    (concat-byte-vectors
      (concat-byte-vectors
        (emit-aarch64-add-x10-x28-x9-lsl3)
        (emit-aarch64-str-x0-x10))
      (emit-aarch64-mov-x0-x9))
    frame-base-slot-count
    current-depth))

(defn emit-root-set-aarch64 []
  (concat-byte-vectors
    (concat-byte-vectors
      (emit-aarch64-add-x10-x28-x9-lsl3)
      (emit-aarch64-str-x0-x10))
    (emit-aarch64-mov-x0-x9)))

(defn emit-root-push-aarch64 []
  (concat-byte-vectors
    (concat-byte-vectors
      (emit-aarch64-str-x0-x27)
      (concat-byte-vectors
        (emit-aarch64-sub-x0-x27-x28)
        (emit-aarch64-lsr-x0-x0-3)))
    (emit-aarch64-add-x27-x27-8)))

(defn emit-root-pop-aarch64 []
  (concat-byte-vectors
    (concat-byte-vectors
      (emit-aarch64-mov-x9-x0)
      (emit-aarch64-sub-x27-x27-8))
    (emit-aarch64-ldr-x0-x27)))

(defn emit-store-window-spill-shifts-aarch64-step [frame-base-slot-count result shift-idx last-shift-idx]
  (if (> shift-idx last-shift-idx)
    (make-native-progress-state 1 shift-idx)
    (do
      (append-native-bytes-rooted result (emit-aarch64-ldr-x1-sp (native-value-window-spill-offset frame-base-slot-count shift-idx)) 4)
      (append-native-bytes-rooted result (emit-aarch64-str-x1-sp (native-value-window-spill-offset frame-base-slot-count (- shift-idx 2))) 4)
      (make-native-progress-state 0 (+ shift-idx 1)))))

(defn emit-store-window-spill-shifts-aarch64-step-64-loop-bounded [frame-base-slot-count result shift-idx last-shift-idx remaining]
  (do
    (root_push result)
    (let [state (emit-store-window-spill-shifts-aarch64-step frame-base-slot-count result shift-idx last-shift-idx)]
      (do
        (root_push state)
        (let [final
              (if (= (vector-get state 0) 1)
                state
                (if (<= remaining 1)
                  state
                  (emit-store-window-spill-shifts-aarch64-step-64-loop-bounded frame-base-slot-count result (vector-get state 1) last-shift-idx (- remaining 1))))]
          (do
            (root_pop)
            (root_pop)
            final))))))

(defn emit-store-window-spill-shifts-aarch64-step-64 [frame-base-slot-count result shift-idx last-shift-idx]
  (emit-store-window-spill-shifts-aarch64-step-64-loop-bounded frame-base-slot-count result shift-idx last-shift-idx 64))

(defn continue-emit-store-window-spill-shifts-aarch64-step-64 [frame-base-slot-count result last-shift-idx state]
  (if (= (vector-get state 0) 1)
    state
    (do
      (root_push result)
      (root_push state)
      (let [next-state (emit-store-window-spill-shifts-aarch64-step-64 frame-base-slot-count result (vector-get state 1) last-shift-idx)]
        (do
          (root_push next-state)
          (let [final (continue-emit-store-window-spill-shifts-aarch64-step-64 frame-base-slot-count result last-shift-idx next-state)]
            (do
              (root_pop)
              (root_pop)
              (root_pop)
              final)))))))

(defn emit-store-window-spill-shifts-aarch64 [frame-base-slot-count shift-idx last-shift-idx]
  (let [result (ref-new (vector-new 64))]
    (do
      (root_push result)
      (continue-emit-store-window-spill-shifts-aarch64-step-64
        frame-base-slot-count
        result
        last-shift-idx
        (emit-store-window-spill-shifts-aarch64-step-64 frame-base-slot-count result shift-idx last-shift-idx))
      (let [final (ref-get result)]
        (do
          (root_pop)
          final)))))

(defn emit-store-bundle-aarch64 [store-bytes frame-base-slot-count current-depth]
  (if (>= current-depth 4)
    (concat-byte-vectors
      (concat-byte-vectors
        (concat-byte-vectors
          store-bytes
          (emit-aarch64-ldr-x0-sp (native-value-window-spill-offset frame-base-slot-count 0)))
        (emit-aarch64-ldr-x9-sp (native-value-window-spill-offset frame-base-slot-count 1)))
      (emit-store-window-spill-shifts-aarch64 frame-base-slot-count 2 (- current-depth 3)))
    (if (= current-depth 3)
      (concat-byte-vectors
        store-bytes
        (emit-aarch64-ldr-x0-sp (native-value-window-spill-offset frame-base-slot-count 0)))
      store-bytes)))

(defn emit-consume-two-bundle-aarch64 [op-bytes frame-base-slot-count current-depth]
  (if (>= current-depth 3)
    (concat-byte-vectors
      (concat-byte-vectors
        op-bytes
        (emit-aarch64-ldr-x9-sp (native-value-window-spill-offset frame-base-slot-count 0)))
      (emit-drop-window-spill-shifts-aarch64 frame-base-slot-count 1 (- current-depth 3)))
    op-bytes))

(defn emit-consume-produce-window-spill-shifts-aarch64-step [frame-base-slot-count result shift-idx last-shift-idx shift-count]
  (if (> shift-idx last-shift-idx)
    (make-native-progress-state 1 shift-idx)
    (do
      (append-native-bytes-rooted result (emit-aarch64-ldr-x1-sp (native-value-window-spill-offset frame-base-slot-count shift-idx)) 4)
      (append-native-bytes-rooted result (emit-aarch64-str-x1-sp (native-value-window-spill-offset frame-base-slot-count (- shift-idx shift-count))) 4)
      (make-native-progress-state 0 (+ shift-idx 1)))))

(defn emit-consume-produce-window-spill-shifts-aarch64-step-64-loop-bounded [frame-base-slot-count result shift-idx last-shift-idx shift-count remaining]
  (do
    (root_push result)
    (let [state (emit-consume-produce-window-spill-shifts-aarch64-step frame-base-slot-count result shift-idx last-shift-idx shift-count)]
      (do
        (root_push state)
        (let [final
              (if (= (vector-get state 0) 1)
                state
                (if (<= remaining 1)
                  state
                  (emit-consume-produce-window-spill-shifts-aarch64-step-64-loop-bounded frame-base-slot-count result (vector-get state 1) last-shift-idx shift-count (- remaining 1))))]
          (do
            (root_pop)
            (root_pop)
            final))))))

(defn emit-consume-produce-window-spill-shifts-aarch64-step-64 [frame-base-slot-count result shift-idx last-shift-idx shift-count]
  (emit-consume-produce-window-spill-shifts-aarch64-step-64-loop-bounded frame-base-slot-count result shift-idx last-shift-idx shift-count 64))

(defn continue-emit-consume-produce-window-spill-shifts-aarch64-step-64 [frame-base-slot-count result last-shift-idx shift-count state]
  (if (= (vector-get state 0) 1)
    state
    (do
      (root_push result)
      (root_push state)
      (let [next-state (emit-consume-produce-window-spill-shifts-aarch64-step-64 frame-base-slot-count result (vector-get state 1) last-shift-idx shift-count)]
        (do
          (root_push next-state)
          (let [final (continue-emit-consume-produce-window-spill-shifts-aarch64-step-64 frame-base-slot-count result last-shift-idx shift-count next-state)]
            (do
              (root_pop)
              (root_pop)
              (root_pop)
              final)))))))

(defn emit-consume-produce-window-spill-shifts-aarch64 [frame-base-slot-count shift-idx last-shift-idx shift-count]
  (let [result (ref-new (vector-new 64))]
    (do
      (root_push result)
      (continue-emit-consume-produce-window-spill-shifts-aarch64-step-64
        frame-base-slot-count
        result
        last-shift-idx
        shift-count
        (emit-consume-produce-window-spill-shifts-aarch64-step-64 frame-base-slot-count result shift-idx last-shift-idx shift-count))
      (let [final (ref-get result)]
        (do
          (root_pop)
          final)))))

(defn emit-consume-produce-one-bundle-aarch64 [op-bytes frame-base-slot-count current-depth consume-count]
  (let [restore-spill-idx (- consume-count 2)
    shift-count (- consume-count 1)
    shift-start (- consume-count 1)
    last-shift-idx (- current-depth 3)]
    (if (>= current-depth (+ consume-count 2))
      (concat-byte-vectors
        (concat-byte-vectors
          op-bytes
          (emit-aarch64-ldr-x9-sp (native-value-window-spill-offset frame-base-slot-count restore-spill-idx)))
        (emit-consume-produce-window-spill-shifts-aarch64 frame-base-slot-count shift-start last-shift-idx shift-count))
      (if (= current-depth (+ consume-count 1))
        (concat-byte-vectors
          op-bytes
          (emit-aarch64-ldr-x9-sp (native-value-window-spill-offset frame-base-slot-count restore-spill-idx)))
        op-bytes))))

(defn emit-i32-store-bundle-aarch64 [offset frame-base-slot-count current-depth]
  (emit-store-bundle-aarch64
    (concat-byte-vectors
      (emit-aarch64-add-x9-x21-x9)
      (emit-aarch64-str-w0-x9 offset))
    frame-base-slot-count
    current-depth))

(defn emit-i64-store-bundle-aarch64 [offset frame-base-slot-count current-depth]
  (emit-store-bundle-aarch64
    (concat-byte-vectors
      (emit-aarch64-add-x9-x21-x9)
      (emit-aarch64-str-x0-x9 offset))
    frame-base-slot-count
    current-depth))

(defn emit-consume-three-window-spill-shifts-aarch64-step [frame-base-slot-count result shift-idx last-shift-idx]
  (if (> shift-idx last-shift-idx)
    (make-native-progress-state 1 shift-idx)
    (do
      (append-native-bytes-rooted result (emit-aarch64-ldr-x1-sp (native-value-window-spill-offset frame-base-slot-count shift-idx)) 4)
      (append-native-bytes-rooted result (emit-aarch64-str-x1-sp (native-value-window-spill-offset frame-base-slot-count (- shift-idx 3))) 4)
      (make-native-progress-state 0 (+ shift-idx 1)))))

(defn emit-consume-three-window-spill-shifts-aarch64-step-64-loop-bounded [frame-base-slot-count result shift-idx last-shift-idx remaining]
  (do
    (root_push result)
    (let [state (emit-consume-three-window-spill-shifts-aarch64-step frame-base-slot-count result shift-idx last-shift-idx)]
      (do
        (root_push state)
        (let [final
              (if (= (vector-get state 0) 1)
                state
                (if (<= remaining 1)
                  state
                  (emit-consume-three-window-spill-shifts-aarch64-step-64-loop-bounded frame-base-slot-count result (vector-get state 1) last-shift-idx (- remaining 1))))]
          (do
            (root_pop)
            (root_pop)
            final))))))

(defn emit-consume-three-window-spill-shifts-aarch64-step-64 [frame-base-slot-count result shift-idx last-shift-idx]
  (emit-consume-three-window-spill-shifts-aarch64-step-64-loop-bounded frame-base-slot-count result shift-idx last-shift-idx 64))

(defn continue-emit-consume-three-window-spill-shifts-aarch64-step-64 [frame-base-slot-count result last-shift-idx state]
  (if (= (vector-get state 0) 1)
    state
    (do
      (root_push result)
      (root_push state)
      (let [next-state (emit-consume-three-window-spill-shifts-aarch64-step-64 frame-base-slot-count result (vector-get state 1) last-shift-idx)]
        (do
          (root_push next-state)
          (let [final (continue-emit-consume-three-window-spill-shifts-aarch64-step-64 frame-base-slot-count result last-shift-idx next-state)]
            (do
              (root_pop)
              (root_pop)
              (root_pop)
              final)))))))

(defn emit-consume-three-window-spill-shifts-aarch64 [frame-base-slot-count shift-idx last-shift-idx]
  (let [result (ref-new (vector-new 64))]
    (do
      (root_push result)
      (continue-emit-consume-three-window-spill-shifts-aarch64-step-64
        frame-base-slot-count
        result
        last-shift-idx
        (emit-consume-three-window-spill-shifts-aarch64-step-64 frame-base-slot-count result shift-idx last-shift-idx))
      (let [final (ref-get result)]
        (do
          (root_pop)
          final)))))

(defn emit-consume-three-bundle-aarch64 [op-bytes frame-base-slot-count current-depth]
  (if (>= current-depth 5)
    (concat-byte-vectors
      (concat-byte-vectors
        (concat-byte-vectors
          op-bytes
          (emit-aarch64-ldr-x0-sp (native-value-window-spill-offset frame-base-slot-count 1)))
        (emit-aarch64-ldr-x9-sp (native-value-window-spill-offset frame-base-slot-count 2)))
      (emit-consume-three-window-spill-shifts-aarch64 frame-base-slot-count 3 (- current-depth 3)))
    (if (= current-depth 4)
      (concat-byte-vectors
        op-bytes
        (emit-aarch64-ldr-x0-sp (native-value-window-spill-offset frame-base-slot-count 1)))
      op-bytes)))

(defn emit-consume-three-produce-one-bundle-aarch64 [op-bytes frame-base-slot-count current-depth]
  (if (>= current-depth 5)
    (concat-byte-vectors
      (concat-byte-vectors
        op-bytes
        (emit-aarch64-ldr-x9-sp (native-value-window-spill-offset frame-base-slot-count 1)))
      (emit-store-window-spill-shifts-aarch64 frame-base-slot-count 2 (- current-depth 3)))
    (if (= current-depth 4)
      (concat-byte-vectors
        op-bytes
        (emit-aarch64-ldr-x9-sp (native-value-window-spill-offset frame-base-slot-count 1)))
      op-bytes)))

(defn emit-consume-four-produce-one-bundle-aarch64 [op-bytes frame-base-slot-count current-depth]
  (if (>= current-depth 6)
    (concat-byte-vectors
      (concat-byte-vectors
        op-bytes
        (emit-aarch64-ldr-x9-sp (native-value-window-spill-offset frame-base-slot-count 2)))
      (emit-consume-three-window-spill-shifts-aarch64 frame-base-slot-count 3 (- current-depth 3)))
    (if (= current-depth 5)
      (concat-byte-vectors
        op-bytes
        (emit-aarch64-ldr-x9-sp (native-value-window-spill-offset frame-base-slot-count 2)))
      op-bytes)))

(defn emit-substring-bundle-aarch64 [helper-disp frame-base-slot-count current-depth]
  (emit-consume-three-produce-one-bundle-aarch64
    (concat-byte-vectors
      (emit-aarch64-ldr-x1-sp (native-value-window-spill-offset frame-base-slot-count 0))
      (emit-aarch64-helper-call-preserving-prev-and-lr helper-disp))
    frame-base-slot-count
    current-depth))

(defn emit-map-insert-bundle-aarch64 [helper-disp frame-base-slot-count current-depth]
  (emit-consume-three-produce-one-bundle-aarch64
    (concat-byte-vectors
      (emit-aarch64-ldr-x1-sp (native-value-window-spill-offset frame-base-slot-count 0))
      (emit-aarch64-helper-call-preserving-prev-and-lr helper-disp))
    frame-base-slot-count
    current-depth))

(defn emit-memory-fill-bundle-aarch64 [frame-base-slot-count current-depth]
  (let [fill-bytes (concat-byte-vectors
                      (concat-byte-vectors
                        (concat-byte-vectors
                          (concat-byte-vectors
                            (concat-byte-vectors
                              (concat-byte-vectors
                                (concat-byte-vectors
                                  (emit-aarch64-ldr-x1-sp (native-value-window-spill-offset frame-base-slot-count 0))
                                  (emit-aarch64-add-x1-x21-x1))
                                (emit-aarch64-mov-x2-x9))
                              (emit-aarch64-mov-x3-x0))
                            (emit-aarch64-cbz-x3-fill-end))
                          (emit-aarch64-strb-w2-x1-post1))
                        (emit-aarch64-sub-x3-x3-1))
                      (emit-aarch64-cbnz-x3-fill-loop))]
    (emit-consume-three-bundle-aarch64 fill-bytes frame-base-slot-count current-depth)))

(defn emit-memory-copy-bundle-aarch64 [frame-base-slot-count current-depth]
  (let [copy-bytes (concat-byte-vectors
                      (concat-byte-vectors
                        (concat-byte-vectors
                          (concat-byte-vectors
                            (concat-byte-vectors
                              (concat-byte-vectors
                                (concat-byte-vectors
                                  (concat-byte-vectors
                                    (emit-aarch64-ldr-x1-sp (native-value-window-spill-offset frame-base-slot-count 0))
                                    (emit-aarch64-add-x1-x21-x1))
                                  (concat-byte-vectors
                                    (emit-aarch64-mov-x2-x9)
                                    (emit-aarch64-add-x2-x21-x2)))
                                (emit-aarch64-mov-x3-x0))
                              (emit-aarch64-cbz-x3-copy-end))
                            (emit-aarch64-ldrb-w4-x2-post1))
                          (emit-aarch64-strb-w4-x1-post1))
                       (emit-aarch64-sub-x3-x3-1))
                     (emit-aarch64-cbnz-x3-copy-loop))]
    (emit-consume-three-bundle-aarch64 copy-bytes frame-base-slot-count current-depth)))

;; IR opcode を AArch64 命令列に変換
(defn codegen-ir-instr-aarch64 [opcode operand]
  (if (= opcode 1)
    ;; i64.const -> MOVZ/MOVK X0, #operand
    (emit-aarch64-load-i64-x0 operand)
    (if (= opcode 3)
      ;; i32.const -> MOV x9, x0; MOVZ W0, #operand
      (emit-i32-const-aarch64 operand)
      (if (= opcode 74)
        (emit-root-push-aarch64)
        (if (= opcode 75)
          (emit-root-pop-aarch64)
          (if (= opcode 76)
            (emit-root-set-aarch64)
          (if (= opcode 10)
        ;; local.get -> MOV x9, x0; LDR x0, [sp, #offset]
        (emit-local-get-aarch64 (local-slot-offset operand))
        (if (= opcode 11)
          ;; local.set -> STR x0, [sp, #offset]
          (emit-aarch64-str-x0-sp (local-slot-offset operand))
          (if (= opcode 20)
            ;; i64.add -> add x0, x9, x0
            (emit-aarch64-add-x0-x9-x0)
            (if (= opcode 21)
              ;; i64.sub -> sub x0, x9, x0
              (emit-aarch64-sub-x0-x9-x0)
              (if (= opcode 22)
                ;; i64.mul -> mul x0, x9, x0
                (emit-aarch64-mul-x0-x9-x0)
                (if (= opcode 23)
                  ;; i64.div_s -> x9 / x0
                  (emit-aarch64-sdiv-x0-x9-x0)
                  (if (= opcode 24)
                    ;; i32.add -> add w0, w9, w0
                    (emit-aarch64-add-w0-w9-w0)
                    (if (= opcode 25)
                      ;; i32.mul -> mul w0, w9, w0
                      (emit-aarch64-mul-w0-w9-w0)
                      (if (= opcode 26)
                        ;; i32.and -> and w0, w9, w0
                        (emit-aarch64-and-w0-w9-w0)
                        (if (= opcode 27)
                          ;; i32.or -> orr w0, w9, w0
                          (emit-aarch64-orr-w0-w9-w0)
                          (if (= opcode 28)
                            ;; i64.rem_s -> x9 % x0
                            (emit-aarch64-rem-x0-x9-x0)
                            (if (= opcode 71)
                              ;; selfhost logical and -> and w0, w9, w0
                              (emit-aarch64-and-w0-w9-w0)
                              (if (= opcode 72)
                                ;; selfhost logical or -> orr w0, w9, w0
                                (emit-aarch64-orr-w0-w9-w0)
                                (if (= opcode 45)
                                   ;; i32.load -> add x0, x21, x0; ldr w0, [x0, #offset]
                                   (concat-byte-vectors
                                     (emit-aarch64-add-x0-x21-x0)
                                     (emit-aarch64-ldr-w0-x0 operand))
                                   (if (= opcode 46)
                                     ;; i32.store -> add x9, x21, x9; str w0, [x9, #offset]
                                     (concat-byte-vectors
                                       (emit-aarch64-add-x9-x21-x9)
                                       (emit-aarch64-str-w0-x9 operand))
                                     (if (= opcode 47)
                                       ;; i32.load8_u -> add x0, x21, x0; ldrb w0, [x0, #offset]
                                       (concat-byte-vectors
                                         (emit-aarch64-add-x0-x21-x0)
                                         (emit-aarch64-ldrb-w0-x0 operand))
                                       (if (= opcode 48)
                                         ;; i64.load -> add x0, x21, x0; ldr x0, [x0, #offset]
                                         (concat-byte-vectors
                                           (emit-aarch64-add-x0-x21-x0)
                                           (emit-aarch64-ldr-x0-x0 operand))
                                         (if (= opcode 49)
                                           ;; i64.store -> add x9, x21, x9; str x0, [x9, #offset]
                                           (concat-byte-vectors
                                             (emit-aarch64-add-x9-x21-x9)
                                             (emit-aarch64-str-x0-x9 operand))
                                           (if (= (is-i64-compare-opcode opcode) 1)
                                            (emit-i64-compare-aarch64 opcode)
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
                                                    (emit-aarch64-nop))))))))))))))))))))))))))))))

(defn native-call-bundle-size-aarch64-twenty-to-twenty-two [target-param-count]
  (if (= target-param-count 22)
    148
    (if (= target-param-count 21)
      140
      132)))

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

(defn native-call-bundle-size-aarch64-twenty-to-twenty-nine [target-param-count]
  (if (= target-param-count 29)
    204
    (native-call-bundle-size-aarch64-twenty-to-twenty-eight target-param-count)))

(defn native-call-bundle-size-aarch64-twenty-to-thirty [target-param-count]
  (if (= target-param-count 30)
    212
    (native-call-bundle-size-aarch64-twenty-to-twenty-nine target-param-count)))

(defn native-call-bundle-size-aarch64-twenty-to-thirty-one [target-param-count]
  (if (= target-param-count 31)
    220
    (native-call-bundle-size-aarch64-twenty-to-thirty target-param-count)))

(defn native-call-bundle-size-aarch64-twenty-to-thirty-two [target-param-count]
  (if (= target-param-count 32)
    228
    (native-call-bundle-size-aarch64-twenty-to-thirty-one target-param-count)))

(defn native-call-bundle-size-aarch64-twenty-to-thirty-three [target-param-count]
  (if (= target-param-count 33)
    236
    (native-call-bundle-size-aarch64-twenty-to-thirty-two target-param-count)))

(defn native-call-bundle-size-aarch64-twenty-to-thirty-four [target-param-count]
  (if (= target-param-count 34)
    244
    (native-call-bundle-size-aarch64-twenty-to-thirty-three target-param-count)))

(defn native-call-bundle-size-aarch64-twenty-to-thirty-five [target-param-count]
  (if (= target-param-count 35)
    252
    (native-call-bundle-size-aarch64-twenty-to-thirty-four target-param-count)))

(defn native-call-bundle-size-aarch64-twenty-to-thirty-six [target-param-count]
  (if (= target-param-count 36)
    260
    (native-call-bundle-size-aarch64-twenty-to-thirty-five target-param-count)))

(defn native-call-bundle-size-aarch64-twenty-to-thirty-seven [target-param-count]
  (if (= target-param-count 37)
    268
    (native-call-bundle-size-aarch64-twenty-to-thirty-six target-param-count)))

(defn native-call-bundle-size-aarch64-twenty-to-thirty-eight [target-param-count]
  (if (= target-param-count 38)
    276
    (native-call-bundle-size-aarch64-twenty-to-thirty-seven target-param-count)))

(defn native-call-bundle-size-aarch64-twenty-to-thirty-nine [target-param-count]
  (if (= target-param-count 39)
    284
    (native-call-bundle-size-aarch64-twenty-to-thirty-eight target-param-count)))

(defn native-call-bundle-size-aarch64-twenty-to-forty [target-param-count]
  (if (= target-param-count 40)
    292
    (native-call-bundle-size-aarch64-twenty-to-thirty-nine target-param-count)))

(defn native-call-bundle-size-aarch64-twenty-to-forty-one [target-param-count]
  (if (= target-param-count 41)
    300
    (native-call-bundle-size-aarch64-twenty-to-forty target-param-count)))

(defn native-call-bundle-size-aarch64-twenty-to-forty-two [target-param-count]
  (if (= target-param-count 42)
    308
    (native-call-bundle-size-aarch64-twenty-to-forty-one target-param-count)))

(defn native-call-bundle-size-aarch64-twenty-to-forty-three [target-param-count]
  (if (= target-param-count 43)
    316
    (native-call-bundle-size-aarch64-twenty-to-forty-two target-param-count)))

(defn native-call-bundle-size-aarch64-twenty-to-forty-four [target-param-count]
  (if (= target-param-count 44)
    324
    (native-call-bundle-size-aarch64-twenty-to-forty-three target-param-count)))

(defn native-call-bundle-size-aarch64-twenty-to-forty-five [target-param-count]
  (if (= target-param-count 45)
    332
    (native-call-bundle-size-aarch64-twenty-to-forty-four target-param-count)))

(defn native-call-bundle-size-aarch64-twenty-to-forty-six [target-param-count]
  (if (= target-param-count 46)
    340
    (native-call-bundle-size-aarch64-twenty-to-forty-five target-param-count)))

(defn native-call-bundle-size-aarch64-twenty-to-forty-seven [target-param-count]
  (if (= target-param-count 47)
    348
    (native-call-bundle-size-aarch64-twenty-to-forty-six target-param-count)))

(defn native-call-bundle-size-aarch64-twenty-to-forty-eight [target-param-count]
  (if (= target-param-count 48)
    356
    (native-call-bundle-size-aarch64-twenty-to-forty-seven target-param-count)))

(defn native-call-bundle-size-aarch64-twenty-to-forty-nine [target-param-count]
  (if (= target-param-count 49)
    364
    (native-call-bundle-size-aarch64-twenty-to-forty-eight target-param-count)))

(defn native-call-bundle-size-aarch64-twenty-to-fifty [target-param-count]
  (if (= target-param-count 50)
    372
    (native-call-bundle-size-aarch64-twenty-to-forty-nine target-param-count)))

(defn native-call-bundle-size-aarch64-twenty-to-fifty-one [target-param-count]
  (if (= target-param-count 51)
    380
    (native-call-bundle-size-aarch64-twenty-to-fifty target-param-count)))

(defn native-call-bundle-size-aarch64-twenty-to-fifty-two [target-param-count]
  (if (= target-param-count 52)
    388
    (native-call-bundle-size-aarch64-twenty-to-fifty-one target-param-count)))

(defn native-call-bundle-size-aarch64-twenty-to-fifty-three [target-param-count]
  (if (= target-param-count 53)
    396
    (native-call-bundle-size-aarch64-twenty-to-fifty-two target-param-count)))

(defn native-call-bundle-size-aarch64-twenty-to-fifty-four [target-param-count]
  (if (= target-param-count 54)
    404
    (native-call-bundle-size-aarch64-twenty-to-fifty-three target-param-count)))

(defn native-call-bundle-size-aarch64-twenty-to-fifty-five [target-param-count]
  (if (= target-param-count 55)
    412
    (native-call-bundle-size-aarch64-twenty-to-fifty-four target-param-count)))

(defn native-call-bundle-size-aarch64-twenty-to-fifty-six [target-param-count]
  (if (= target-param-count 56)
    420
    (native-call-bundle-size-aarch64-twenty-to-fifty-five target-param-count)))

(defn native-call-bundle-size-aarch64-twenty-to-fifty-seven [target-param-count]
  (if (= target-param-count 57)
    428
    (native-call-bundle-size-aarch64-twenty-to-fifty-six target-param-count)))

(defn native-call-bundle-size-aarch64-twenty-to-fifty-eight [target-param-count]
  (if (= target-param-count 58)
    436
    (native-call-bundle-size-aarch64-twenty-to-fifty-seven target-param-count)))

(defn native-call-bundle-size-aarch64-twenty-to-sixty [target-param-count]
  (if (> target-param-count 60)
    (- (* 8 target-param-count) 28)
    (if (= target-param-count 60)
      452
      (if (= target-param-count 59)
        444
        (native-call-bundle-size-aarch64-twenty-to-fifty-eight target-param-count)))))

(defn native-call-bundle-disp-aarch64-twenty-to-twenty-two [target-param-count target-offset current-offset]
  (if (= target-param-count 22)
    (- target-offset (+ current-offset 140))
    (if (= target-param-count 21)
      (- target-offset (+ current-offset 132))
      (- target-offset (+ current-offset 124)))))

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

(defn native-call-bundle-disp-aarch64-twenty-to-twenty-nine [target-param-count target-offset current-offset]
  (if (= target-param-count 29)
    (- target-offset (+ current-offset 196))
    (native-call-bundle-disp-aarch64-twenty-to-twenty-eight target-param-count target-offset current-offset)))

(defn native-call-bundle-disp-aarch64-twenty-to-thirty [target-param-count target-offset current-offset]
  (if (= target-param-count 30)
    (- target-offset (+ current-offset 204))
    (native-call-bundle-disp-aarch64-twenty-to-twenty-nine target-param-count target-offset current-offset)))

(defn native-call-bundle-disp-aarch64-twenty-to-thirty-one [target-param-count target-offset current-offset]
  (if (= target-param-count 31)
    (- target-offset (+ current-offset 212))
    (native-call-bundle-disp-aarch64-twenty-to-thirty target-param-count target-offset current-offset)))

(defn native-call-bundle-disp-aarch64-twenty-to-thirty-two [target-param-count target-offset current-offset]
  (if (= target-param-count 32)
    (- target-offset (+ current-offset 220))
    (native-call-bundle-disp-aarch64-twenty-to-thirty-one target-param-count target-offset current-offset)))

(defn native-call-bundle-disp-aarch64-twenty-to-thirty-three [target-param-count target-offset current-offset]
  (if (= target-param-count 33)
    (- target-offset (+ current-offset 228))
    (native-call-bundle-disp-aarch64-twenty-to-thirty-two target-param-count target-offset current-offset)))

(defn native-call-bundle-disp-aarch64-twenty-to-thirty-four [target-param-count target-offset current-offset]
  (if (= target-param-count 34)
    (- target-offset (+ current-offset 236))
    (native-call-bundle-disp-aarch64-twenty-to-thirty-three target-param-count target-offset current-offset)))

(defn native-call-bundle-disp-aarch64-twenty-to-thirty-five [target-param-count target-offset current-offset]
  (if (= target-param-count 35)
    (- target-offset (+ current-offset 244))
    (native-call-bundle-disp-aarch64-twenty-to-thirty-four target-param-count target-offset current-offset)))

(defn native-call-bundle-disp-aarch64-twenty-to-thirty-six [target-param-count target-offset current-offset]
  (if (= target-param-count 36)
    (- target-offset (+ current-offset 252))
    (native-call-bundle-disp-aarch64-twenty-to-thirty-five target-param-count target-offset current-offset)))

(defn native-call-bundle-disp-aarch64-twenty-to-thirty-seven [target-param-count target-offset current-offset]
  (if (= target-param-count 37)
    (- target-offset (+ current-offset 260))
    (native-call-bundle-disp-aarch64-twenty-to-thirty-six target-param-count target-offset current-offset)))

(defn native-call-bundle-disp-aarch64-twenty-to-thirty-eight [target-param-count target-offset current-offset]
  (if (= target-param-count 38)
    (- target-offset (+ current-offset 268))
    (native-call-bundle-disp-aarch64-twenty-to-thirty-seven target-param-count target-offset current-offset)))

(defn native-call-bundle-disp-aarch64-twenty-to-thirty-nine [target-param-count target-offset current-offset]
  (if (= target-param-count 39)
    (- target-offset (+ current-offset 276))
    (native-call-bundle-disp-aarch64-twenty-to-thirty-eight target-param-count target-offset current-offset)))

(defn native-call-bundle-disp-aarch64-twenty-to-forty [target-param-count target-offset current-offset]
  (if (= target-param-count 40)
    (- target-offset (+ current-offset 284))
    (native-call-bundle-disp-aarch64-twenty-to-thirty-nine target-param-count target-offset current-offset)))

(defn native-call-bundle-disp-aarch64-twenty-to-forty-one [target-param-count target-offset current-offset]
  (if (= target-param-count 41)
    (- target-offset (+ current-offset 292))
    (native-call-bundle-disp-aarch64-twenty-to-forty target-param-count target-offset current-offset)))

(defn native-call-bundle-disp-aarch64-twenty-to-forty-two [target-param-count target-offset current-offset]
  (if (= target-param-count 42)
    (- target-offset (+ current-offset 300))
    (native-call-bundle-disp-aarch64-twenty-to-forty-one target-param-count target-offset current-offset)))

(defn native-call-bundle-disp-aarch64-twenty-to-forty-three [target-param-count target-offset current-offset]
  (if (= target-param-count 43)
    (- target-offset (+ current-offset 308))
    (native-call-bundle-disp-aarch64-twenty-to-forty-two target-param-count target-offset current-offset)))

(defn native-call-bundle-disp-aarch64-twenty-to-forty-four [target-param-count target-offset current-offset]
  (if (= target-param-count 44)
    (- target-offset (+ current-offset 316))
    (native-call-bundle-disp-aarch64-twenty-to-forty-three target-param-count target-offset current-offset)))

(defn native-call-bundle-disp-aarch64-twenty-to-forty-five [target-param-count target-offset current-offset]
  (if (= target-param-count 45)
    (- target-offset (+ current-offset 324))
    (native-call-bundle-disp-aarch64-twenty-to-forty-four target-param-count target-offset current-offset)))

(defn native-call-bundle-disp-aarch64-twenty-to-forty-six [target-param-count target-offset current-offset]
  (if (= target-param-count 46)
    (- target-offset (+ current-offset 332))
    (native-call-bundle-disp-aarch64-twenty-to-forty-five target-param-count target-offset current-offset)))

(defn native-call-bundle-disp-aarch64-twenty-to-forty-seven [target-param-count target-offset current-offset]
  (if (= target-param-count 47)
    (- target-offset (+ current-offset 340))
    (native-call-bundle-disp-aarch64-twenty-to-forty-six target-param-count target-offset current-offset)))

(defn native-call-bundle-disp-aarch64-twenty-to-forty-eight [target-param-count target-offset current-offset]
  (if (= target-param-count 48)
    (- target-offset (+ current-offset 348))
    (native-call-bundle-disp-aarch64-twenty-to-forty-seven target-param-count target-offset current-offset)))

(defn native-call-bundle-disp-aarch64-twenty-to-forty-nine [target-param-count target-offset current-offset]
  (if (= target-param-count 49)
    (- target-offset (+ current-offset 356))
    (native-call-bundle-disp-aarch64-twenty-to-forty-eight target-param-count target-offset current-offset)))

(defn native-call-bundle-disp-aarch64-twenty-to-fifty [target-param-count target-offset current-offset]
  (if (= target-param-count 50)
    (- target-offset (+ current-offset 364))
    (native-call-bundle-disp-aarch64-twenty-to-forty-nine target-param-count target-offset current-offset)))

(defn native-call-bundle-disp-aarch64-twenty-to-fifty-one [target-param-count target-offset current-offset]
  (if (= target-param-count 51)
    (- target-offset (+ current-offset 372))
    (native-call-bundle-disp-aarch64-twenty-to-fifty target-param-count target-offset current-offset)))

(defn native-call-bundle-disp-aarch64-twenty-to-fifty-two [target-param-count target-offset current-offset]
  (if (= target-param-count 52)
    (- target-offset (+ current-offset 380))
    (native-call-bundle-disp-aarch64-twenty-to-fifty-one target-param-count target-offset current-offset)))

(defn native-call-bundle-disp-aarch64-twenty-to-fifty-three [target-param-count target-offset current-offset]
  (if (= target-param-count 53)
    (- target-offset (+ current-offset 388))
    (native-call-bundle-disp-aarch64-twenty-to-fifty-two target-param-count target-offset current-offset)))

(defn native-call-bundle-disp-aarch64-twenty-to-fifty-four [target-param-count target-offset current-offset]
  (if (= target-param-count 54)
    (- target-offset (+ current-offset 396))
    (native-call-bundle-disp-aarch64-twenty-to-fifty-three target-param-count target-offset current-offset)))

(defn native-call-bundle-disp-aarch64-twenty-to-fifty-five [target-param-count target-offset current-offset]
  (if (= target-param-count 55)
    (- target-offset (+ current-offset 404))
    (native-call-bundle-disp-aarch64-twenty-to-fifty-four target-param-count target-offset current-offset)))

(defn native-call-bundle-disp-aarch64-twenty-to-fifty-six [target-param-count target-offset current-offset]
  (if (= target-param-count 56)
    (- target-offset (+ current-offset 412))
    (native-call-bundle-disp-aarch64-twenty-to-fifty-five target-param-count target-offset current-offset)))

(defn native-call-bundle-disp-aarch64-twenty-to-fifty-seven [target-param-count target-offset current-offset]
  (if (= target-param-count 57)
    (- target-offset (+ current-offset 420))
    (native-call-bundle-disp-aarch64-twenty-to-fifty-six target-param-count target-offset current-offset)))

(defn native-call-bundle-disp-aarch64-twenty-to-fifty-eight [target-param-count target-offset current-offset]
  (if (= target-param-count 58)
    (- target-offset (+ current-offset 428))
    (native-call-bundle-disp-aarch64-twenty-to-fifty-seven target-param-count target-offset current-offset)))

(defn native-call-bundle-disp-aarch64-twenty-to-sixty [target-param-count target-offset current-offset]
  (if (> target-param-count 60)
    (- target-offset (+ current-offset (- (* 8 target-param-count) 36)))
    (if (= target-param-count 60)
      (- target-offset (+ current-offset 444))
      (if (= target-param-count 59)
        (- target-offset (+ current-offset 436))
        (native-call-bundle-disp-aarch64-twenty-to-fifty-eight target-param-count target-offset current-offset)))))

(defn native-consume-produce-one-tail-size-aarch64 [current-depth consume-count]
  (if (>= current-depth (+ consume-count 2))
    (+ 4 (* (- (- current-depth consume-count) 1) 8))
    (if (= current-depth (+ consume-count 1)) 4 0)))

(defn native-consume-produce-one-size-aarch64 [base-size current-depth consume-count]
  (+ base-size (native-consume-produce-one-tail-size-aarch64 current-depth consume-count)))

(defn native-selfhost-runtime-helper-tail-size-aarch64 [opcode current-depth]
  (if (= opcode 62)
    (if (>= current-depth 5)
      (+ 20 (* (- current-depth 4) 8))
      (if (= current-depth 4) 20 16))
    (if (= opcode 63)
      (if (>= current-depth 3) (+ 16 (* (- current-depth 3) 8)) 12)
      (if (= opcode 64)
        12
        (if (= opcode 73)
          12
          (if (= opcode 67)
            12
            (if (= opcode 59)
              12
              (if (= opcode 54)
                12
                (if (= opcode 52)
                  12
                  (if (= opcode 56)
                    12
                    (if (= opcode 57) 12 0)))))))))))

(defn native-selfhost-runtime-helper-size-aarch64 [opcode current-depth]
  (if (= opcode 50)
    (if (>= current-depth 3) (+ 16 (* (- current-depth 3) 8)) 12)
    (if (= opcode 51)
      12
      (if (= opcode 53)
        (if (>= current-depth 3) (+ 16 (* (- current-depth 3) 8)) 12)
        (if (= opcode 55)
          (if (>= current-depth 3) (+ 16 (* (- current-depth 3) 8)) 12)
            (if (= opcode 58)
              (if (>= current-depth 3) (+ 16 (* (- current-depth 3) 8)) 12)
              (if (= opcode 69)
                (if (>= current-depth 5)
                  (+ 20 (* (- current-depth 4) 8))
                  (if (= current-depth 4) 20 16))
                (if (= opcode 70)
                  (if (>= current-depth 3) (+ 16 (* (- current-depth 3) 8)) 12)
                (if (= opcode 60)
                  (if (>= current-depth 2)
                    (+ 20 (* (- current-depth 2) 8))
                    (if (= current-depth 1) 16 12))
                  (if (= opcode 61)
                    12
                    (native-selfhost-runtime-helper-tail-size-aarch64 opcode current-depth)))))))))))

(defn native-instr-size-aarch64 [opcode operand function-metas current-depth]
  (if (= (is-control-opcode opcode) 1)
    (if (= opcode 41)
      (native-conditional-control-instr-size-aarch64 current-depth)
      (if (= opcode 81)
        (native-conditional-control-instr-size-aarch64 current-depth)
        (if (= opcode 83)
          (native-conditional-control-instr-size-aarch64 current-depth)
          (native-control-instr-size-aarch64 opcode))))
    (if (= opcode 40)
    (let [target-meta (vector-get function-metas operand)
      target-param-count (native-function-param-count target-meta)]
      (if (>= target-param-count 20)
        (native-call-bundle-size-aarch64-twenty-to-sixty target-param-count)
        (if (> target-param-count 9)
          (+ 52 (* (- target-param-count 10) 8))
          (if (= target-param-count 9)
            (native-consume-produce-one-size-aarch64 48 current-depth 9)
            (if (= target-param-count 8)
              (native-consume-produce-one-size-aarch64 36 current-depth 8)
              (if (= target-param-count 7)
                (native-consume-produce-one-size-aarch64 32 current-depth 7)
                (if (= target-param-count 6)
                  (native-consume-produce-one-size-aarch64 28 current-depth 6)
                  (if (= target-param-count 5)
                    (native-consume-produce-one-size-aarch64 24 current-depth 5)
                    (if (= target-param-count 4)
                      (native-consume-produce-one-size-aarch64 20 current-depth 4)
                      (if (= target-param-count 3)
                        (native-consume-produce-one-size-aarch64 16 current-depth 3)
                        (if (= target-param-count 2)
                          (native-consume-produce-one-size-aarch64 12 current-depth 2)
                           (if (= target-param-count 1)
                             12
                             (native-produce-one-size-aarch64 4 current-depth)))))))))))))
    (if (= opcode 1)
      (native-produce-one-size-aarch64 (aarch64-load-i64-x0-size operand) current-depth)
      (if (= opcode 3)
        (native-produce-one-size-aarch64 (aarch64-load-u32-w0-size operand) current-depth)
        (if (= opcode 74)
          16
          (if (= opcode 75)
            (native-produce-one-size-aarch64 12 current-depth)
            (if (> (native-selfhost-runtime-helper-size-aarch64 opcode current-depth) 0)
            (native-selfhost-runtime-helper-size-aarch64 opcode current-depth)
            (if (= opcode 45)
              8
              (if (= opcode 46)
                (if (>= current-depth 4)
                  (+ 16 (* (- current-depth 4) 8))
                  (if (= current-depth 3) 12 8))
                (if (= opcode 47)
                  8
                  (if (= opcode 48)
                    8
                    (if (= opcode 49)
                      (if (>= current-depth 4)
                        (+ 16 (* (- current-depth 4) 8))
                        (if (= current-depth 3) 12 8))
                      (if (= opcode 77)
                        (if (>= current-depth 5)
                          (+ 48 (* (- current-depth 5) 8))
                          (if (= current-depth 4) 36 40))
                        (if (= opcode 78)
                          (if (>= current-depth 5)
                            (+ 40 (* (- current-depth 5) 8))
                            (if (= current-depth 4) 32 32))
                            (if (= opcode 11)
                              (if (>= current-depth 3)
                                (+ 12 (* (- current-depth 3) 8))
                                (if (= current-depth 2) 8 4))
                             (if (= opcode 10)
                               (if (>= current-depth 2)
                                 (+ 12 (* (- current-depth 2) 8))
                                 (if (= current-depth 1) 8 4))
                                 (if (= opcode 44)
                                   (if (>= current-depth 3) (+ 8 (* (- current-depth 3) 8)) 4)
                                   (if (= opcode 76)
                                   (if (>= current-depth 3) (+ 16 (* (- current-depth 3) 8)) 12)
                                     (let [plain-size (native-plain-instr-size-aarch64 opcode operand)]
                                       (if (= (opcode-stack-delta opcode operand function-metas) -1)
                                         (if (>= current-depth 3)
                                         (+ (+ plain-size 4) (* (- current-depth 3) 8))
                                         plain-size)
                                       plain-size)))))))))))))))))))))

(defn native-function-body-size-aarch64-loop [ir-func function-metas idx len total current-depth]
  (if (>= idx len)
    total
    (let [instr (vector-get ir-func idx)
      opcode (vector-get instr 0)
      operand (vector-get instr 1)
      next-total (+ total (native-instr-size-aarch64 opcode operand function-metas current-depth))
      next-depth (apply-stack-delta current-depth (opcode-stack-delta opcode operand function-metas))]
      (native-function-body-size-aarch64-loop ir-func function-metas (+ idx 1) len next-total next-depth))))

(defn native-function-body-size-aarch64-step [ir-func function-metas idx len total current-depth]
  (if (>= idx len)
    (make-callable-object-offset-state 1 idx total current-depth)
    (let [instr (vector-get ir-func idx)
      opcode (vector-get instr 0)
      operand (vector-get instr 1)
      next-total (+ total (native-instr-size-aarch64 opcode operand function-metas current-depth))
      next-depth (apply-stack-delta current-depth (opcode-stack-delta opcode operand function-metas))]
      (make-callable-object-offset-state 0 (+ idx 1) next-total next-depth))))

(defn native-function-body-size-aarch64-step-64-loop-bounded [ir-func function-metas idx len total current-depth remaining]
  (do
    (root_push ir-func)
    (root_push function-metas)
    (let [state (native-function-body-size-aarch64-step ir-func function-metas idx len total current-depth)
      done (vector-get state 0)
      next-idx (vector-get state 1)
      next-total (vector-get state 2)
      next-depth (vector-get state 3)]
      (do
        (root_push state)
        (let [final
          (if (= done 1)
            state
            (if (<= remaining 1)
              state
              (native-function-body-size-aarch64-step-64-loop-bounded ir-func function-metas next-idx len next-total next-depth (- remaining 1))))]
          (do
            (root_pop)
            (root_pop)
            (root_pop)
            final))))))

(defn native-function-body-size-aarch64-step-64 [ir-func function-metas idx len total current-depth]
  (native-function-body-size-aarch64-step-64-loop-bounded ir-func function-metas idx len total current-depth 64))

(defn continue-native-function-body-size-aarch64-step-64 [ir-func function-metas len state]
  (if (= (vector-get state 0) 1)
    state
    (do
      (root_push ir-func)
      (root_push function-metas)
      (root_push state)
      (let [next-state (native-function-body-size-aarch64-step-64
                         ir-func
                         function-metas
                         (vector-get state 1)
                         len
                         (vector-get state 2)
                         (vector-get state 3))]
        (do
          (root_push next-state)
          (let [final (continue-native-function-body-size-aarch64-step-64 ir-func function-metas len next-state)]
            (do
              (root_pop)
              (root_pop)
              (root_pop)
              (root_pop)
              final)))))))

(defn native-function-size-aarch64 [func-meta function-metas]
  (let [param-count (native-function-param-count func-meta)
    local-count (native-function-local-count func-meta)
    ir-func (native-function-ir func-meta)
    stack-bytes (native-local-stack-bytes-with-window ir-func (+ param-count local-count) function-metas)
    min-slot-count (+ param-count local-count)
    has-call (native-has-call ir-func)
    prologue-stack-bytes (if (if (= (aarch64-bundle-stack-padding-needed ir-func min-slot-count function-metas) 1) (> stack-bytes 0) false)
                           (align-16 (+ stack-bytes 8))
                           stack-bytes)
    stack-frame-bytes (if (> prologue-stack-bytes 0) 8 0)
    call-frame-bytes (if (= has-call 1) 8 0)
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
    body-bytes (vector-get
                 (continue-native-function-body-size-aarch64-step-64
                   ir-func
                   function-metas
                   (vector-length ir-func)
                   (native-function-body-size-aarch64-step-64 ir-func function-metas 0 (vector-length ir-func) 0 0))
                 2)
    local-zero-bytes (* local-count 8)]
    (+ (+ (+ (+ (+ 4 stack-frame-bytes) call-frame-bytes) param-spill-bytes) local-zero-bytes) body-bytes)))

(defn make-callable-object-state [done next-idx next-values]
  (do
    (root_push next-values)
    (let [base0 (vector-push (vector-new 3) done)]
      (do
        (root_push base0)
        (let [base1 (vector-push base0 next-idx)]
          (do
            (root_push base1)
            (let [state (vector-push base1 next-values)]
              (do
                (root_pop)
                (root_pop)
                (root_pop)
                state))))))))

(defn make-callable-object-offset-state [done next-idx next-values next-offset]
  (do
    (root_push next-values)
    (let [base0 (vector-push (vector-new 4) done)]
      (do
        (root_push base0)
        (let [base1 (vector-push base0 next-idx)]
          (do
            (root_push base1)
            (let [with-values (vector-push base1 next-values)]
              (do
                (root_push with-values)
                (let [state (vector-push with-values next-offset)]
                  (do
                    (root_pop)
                    (root_pop)
                    (root_pop)
                    (root_pop)
                    state))))))))))

(defn make-callable-sum-state [done next-idx total]
  (let [base0 (vector-push (vector-new 3) done)]
    (do
      (root_push base0)
      (let [base1 (vector-push base0 next-idx)]
        (do
          (root_push base1)
          (let [state (vector-push base1 total)]
            (do
              (root_pop)
              (root_pop)
              state)))))))

(defn collect-callable-function-starts-aarch64-step [functions idx len starts offset]
  (if (>= idx len)
    (make-callable-object-offset-state 1 idx starts offset)
    (do
      (root_push functions)
      (root_push starts)
      (let [func-meta (vector-get functions idx)]
        (do
          (root_push func-meta)
          (let [next-starts (vector-push starts offset)
            next-offset (+ offset (native-function-size-aarch64 func-meta functions))]
            (do
              (root_push next-starts)
              (let [state (make-callable-object-offset-state 0 (+ idx 1) next-starts next-offset)]
                (do
                  (root_pop)
                  (root_pop)
                  (root_pop)
                  (root_pop)
                  state)))))))))

(defn collect-callable-function-starts-aarch64-step-64-loop-bounded [functions idx len starts offset remaining]
  (do
    (root_push functions)
    (root_push starts)
    (let [state (collect-callable-function-starts-aarch64-step functions idx len starts offset)
      done (vector-get state 0)
      next-idx (vector-get state 1)
      next-starts (vector-get state 2)
      next-offset (vector-get state 3)]
      (do
        (root_push state)
        (root_push next-starts)
        (let [result
          (if (= done 1)
            state
            (if (<= remaining 1)
              state
              (collect-callable-function-starts-aarch64-step-64-loop-bounded functions next-idx len next-starts next-offset (- remaining 1))))]
          (do
            (root_pop)
            (root_pop)
            (root_pop)
            (root_pop)
            result))))))

(defn collect-callable-function-starts-aarch64-step-64 [functions idx len starts offset]
  (collect-callable-function-starts-aarch64-step-64-loop-bounded functions idx len starts offset 64))

(defn continue-collect-callable-function-starts-aarch64-step-64 [functions len state]
  (if (= (vector-get state 0) 1)
    state
    (do
      (root_push functions)
      (root_push state)
      (let [next-state (collect-callable-function-starts-aarch64-step-64 functions (vector-get state 1) len (vector-get state 2) (vector-get state 3))]
        (do
          (root_push next-state)
          (let [result (continue-collect-callable-function-starts-aarch64-step-64 functions len next-state)]
            (do
              (root_pop)
              (root_pop)
              (root_pop)
              result)))))))

(defn collect-callable-function-starts-aarch64 [functions import-count]
  (vector-get
    (continue-collect-callable-function-starts-aarch64-step-64
      functions
      (vector-length functions)
      (collect-callable-function-starts-aarch64-step-64 functions import-count (vector-length functions) (vector-new 8) 0))
    2))

(defn measure-native-function-aarch64-bundle-with-import-count [func-meta function-starts function-metas import-count import-stub-offset function-start]
  (do
    (root_push func-meta)
    (root_push function-starts)
    (root_push function-metas)
    (let [result (ref-new (vector-new 64))]
      (do
        (root_push result)
        (let [length
          (do
            (generate-native-function-aarch64-bundle-with-import-count func-meta result function-starts function-metas import-count import-stub-offset function-start)
            (vector-length (ref-get result)))]
          (do
            (root_pop)
            (root_pop)
            (root_pop)
            (root_pop)
            length))))))

(defn collect-callable-function-lengths-aarch64-step [functions idx len rough-starts import-count import-stub-offset lengths]
  (if (>= idx len)
    (make-callable-object-state 1 idx lengths)
    (do
      (root_push functions)
      (root_push rough-starts)
      (root_push lengths)
      (let [user-idx (- idx import-count)
        func-meta (vector-get functions idx)
        function-start (vector-get rough-starts user-idx)]
        (do
          (root_push func-meta)
          (let [function-length (native-function-size-aarch64 func-meta functions)
            next-lengths (vector-push lengths function-length)]
            (do
              (root_push next-lengths)
              (let [state (make-callable-object-state 0 (+ idx 1) next-lengths)]
                (do
                  (root_pop)
                  (root_pop)
                  (root_pop)
                  (root_pop)
                  (root_pop)
                  state)))))))))

(defn collect-callable-function-lengths-aarch64-step-64-loop-bounded [functions idx len rough-starts import-count import-stub-offset lengths remaining]
  (do
    (root_push functions)
    (root_push rough-starts)
    (root_push lengths)
    (let [state (collect-callable-function-lengths-aarch64-step functions idx len rough-starts import-count import-stub-offset lengths)
      done (vector-get state 0)
      next-idx (vector-get state 1)
      next-lengths (vector-get state 2)]
      (do
        (root_push state)
        (root_push next-lengths)
        (let [result
          (if (= done 1)
            state
            (if (<= remaining 1)
              state
              (collect-callable-function-lengths-aarch64-step-64-loop-bounded functions next-idx len rough-starts import-count import-stub-offset next-lengths (- remaining 1))))]
          (do
            (root_pop)
            (root_pop)
            (root_pop)
            (root_pop)
            (root_pop)
            result))))))

(defn collect-callable-function-lengths-aarch64-step-64 [functions idx len rough-starts import-count import-stub-offset lengths]
  (collect-callable-function-lengths-aarch64-step-64-loop-bounded functions idx len rough-starts import-count import-stub-offset lengths 64))

(defn continue-collect-callable-function-lengths-aarch64-step-64 [functions len rough-starts import-count import-stub-offset state]
  (if (= (vector-get state 0) 1)
    state
    (do
      (root_push functions)
      (root_push rough-starts)
      (root_push state)
      (let [next-state (collect-callable-function-lengths-aarch64-step-64 functions (vector-get state 1) len rough-starts import-count import-stub-offset (vector-get state 2))]
        (do
          (root_push next-state)
          (let [result (continue-collect-callable-function-lengths-aarch64-step-64 functions len rough-starts import-count import-stub-offset next-state)]
            (do
              (root_pop)
              (root_pop)
              (root_pop)
              (root_pop)
              result)))))))

(defn collect-callable-function-lengths-aarch64 [functions import-count rough-starts import-stub-offset]
  (vector-get
    (continue-collect-callable-function-lengths-aarch64-step-64
      functions
      (vector-length functions)
      rough-starts
      import-count
      import-stub-offset
      (collect-callable-function-lengths-aarch64-step-64 functions import-count (vector-length functions) rough-starts import-count import-stub-offset (vector-new 8)))
    2))

(defn collect-callable-function-starts-from-lengths-aarch64-step [lengths idx len starts offset]
  (if (>= idx len)
    (make-callable-object-offset-state 1 idx starts offset)
    (do
      (root_push lengths)
      (root_push starts)
      (let [next-starts (vector-push starts offset)
        next-offset (+ offset (vector-get lengths idx))]
        (do
          (root_push next-starts)
          (let [state (make-callable-object-offset-state 0 (+ idx 1) next-starts next-offset)]
            (do
              (root_pop)
              (root_pop)
              (root_pop)
              state)))))))

(defn collect-callable-function-starts-from-lengths-aarch64-step-64-loop-bounded [lengths idx len starts offset remaining]
  (do
    (root_push lengths)
    (root_push starts)
    (let [state (collect-callable-function-starts-from-lengths-aarch64-step lengths idx len starts offset)
      done (vector-get state 0)
      next-idx (vector-get state 1)
      next-starts (vector-get state 2)
      next-offset (vector-get state 3)]
      (do
        (root_push state)
        (root_push next-starts)
        (let [result
          (if (= done 1)
            state
            (if (<= remaining 1)
              state
              (collect-callable-function-starts-from-lengths-aarch64-step-64-loop-bounded lengths next-idx len next-starts next-offset (- remaining 1))))]
          (do
            (root_pop)
            (root_pop)
            (root_pop)
            (root_pop)
            result))))))

(defn collect-callable-function-starts-from-lengths-aarch64-step-64 [lengths idx len starts offset]
  (collect-callable-function-starts-from-lengths-aarch64-step-64-loop-bounded lengths idx len starts offset 64))

(defn continue-collect-callable-function-starts-from-lengths-aarch64-step-64 [lengths len state]
  (if (= (vector-get state 0) 1)
    state
    (do
      (root_push lengths)
      (root_push state)
      (let [next-state (collect-callable-function-starts-from-lengths-aarch64-step-64 lengths (vector-get state 1) len (vector-get state 2) (vector-get state 3))]
        (do
          (root_push next-state)
          (let [result (continue-collect-callable-function-starts-from-lengths-aarch64-step-64 lengths len next-state)]
            (do
              (root_pop)
              (root_pop)
              (root_pop)
              result)))))))

(defn collect-callable-function-starts-from-lengths-aarch64 [lengths]
  (vector-get
    (continue-collect-callable-function-starts-from-lengths-aarch64-step-64
      lengths
      (vector-length lengths)
      (collect-callable-function-starts-from-lengths-aarch64-step-64 lengths 0 (vector-length lengths) (vector-new 8) 0))
    2))

(defn sum-callable-function-lengths-aarch64-step [lengths idx len total]
  (if (>= idx len)
    (make-callable-sum-state 1 idx total)
    (do
      (root_push lengths)
      (let [state (make-callable-sum-state 0 (+ idx 1) (+ total (vector-get lengths idx)))]
        (do
          (root_pop)
          state)))))

(defn sum-callable-function-lengths-aarch64-step-64-loop-bounded [lengths idx len total remaining]
  (do
    (root_push lengths)
    (let [state (sum-callable-function-lengths-aarch64-step lengths idx len total)
      done (vector-get state 0)
      next-idx (vector-get state 1)
      next-total (vector-get state 2)]
      (do
        (root_push state)
        (let [result
          (if (= done 1)
            state
            (if (<= remaining 1)
              state
              (sum-callable-function-lengths-aarch64-step-64-loop-bounded lengths next-idx len next-total (- remaining 1))))]
          (do
            (root_pop)
            (root_pop)
            result))))))

(defn sum-callable-function-lengths-aarch64-step-64 [lengths idx len total]
  (sum-callable-function-lengths-aarch64-step-64-loop-bounded lengths idx len total 64))

(defn continue-sum-callable-function-lengths-aarch64-step-64 [lengths len state]
  (if (= (vector-get state 0) 1)
    state
    (do
      (root_push lengths)
      (root_push state)
      (let [next-state (sum-callable-function-lengths-aarch64-step-64 lengths (vector-get state 1) len (vector-get state 2))]
        (do
          (root_push next-state)
          (let [result (continue-sum-callable-function-lengths-aarch64-step-64 lengths len next-state)]
            (do
              (root_pop)
              (root_pop)
              (root_pop)
              result)))))))

(defn sum-callable-function-lengths-aarch64 [lengths]
  (vector-get
    (continue-sum-callable-function-lengths-aarch64-step-64
      lengths
      (vector-length lengths)
      (sum-callable-function-lengths-aarch64-step-64 lengths 0 (vector-length lengths) 0))
    2))

(defn make-callable-layout-aarch64 [function-starts import-stub-offset]
  (vector-push (vector-push (vector-new 2) function-starts) import-stub-offset))

(defn callable-layout-function-starts-aarch64 [layout]
  (vector-get layout 0))

(defn callable-layout-import-stub-offset-aarch64 [layout]
  (vector-get layout 1))

(defn collect-callable-actual-layout-aarch64 [functions import-count]
  (do
    (root_push functions)
    (let [rough-starts (collect-callable-function-starts-aarch64 functions import-count)
      rough-import-stub-offset (callable-user-total-size-aarch64 functions import-count)]
      (do
        (root_push rough-starts)
        (let [actual-lengths (collect-callable-function-lengths-aarch64 functions import-count rough-starts rough-import-stub-offset)]
          (do
            (root_push actual-lengths)
            (let [actual-starts (collect-callable-function-starts-from-lengths-aarch64 actual-lengths)
              actual-import-stub-offset (sum-callable-function-lengths-aarch64 actual-lengths)]
              (do
                (root_push actual-starts)
                (let [layout (make-callable-layout-aarch64 actual-starts actual-import-stub-offset)]
                  (do
                    (root_pop)
                    (root_pop)
                    (root_pop)
                    (root_pop)
                    layout))))))))))

(defn callable-user-total-size-aarch64-step [functions idx len total]
  (if (>= idx len)
    (make-callable-sum-state 1 idx total)
    (do
      (root_push functions)
      (let [func-meta (vector-get functions idx)
        next-total (+ total (native-function-size-aarch64 func-meta functions))]
        (do
          (root_pop)
          (make-callable-sum-state 0 (+ idx 1) next-total))))))

(defn callable-user-total-size-aarch64-step-64-loop-bounded [functions idx len total remaining]
  (do
    (root_push functions)
    (let [state (callable-user-total-size-aarch64-step functions idx len total)
      done (vector-get state 0)
      next-idx (vector-get state 1)
      next-total (vector-get state 2)]
      (do
        (root_push state)
        (let [result
          (if (= done 1)
            state
            (if (<= remaining 1)
              state
              (callable-user-total-size-aarch64-step-64-loop-bounded functions next-idx len next-total (- remaining 1))))]
          (do
            (root_pop)
            (root_pop)
            result))))))

(defn callable-user-total-size-aarch64-step-64 [functions idx len total]
  (callable-user-total-size-aarch64-step-64-loop-bounded functions idx len total 64))

(defn continue-callable-user-total-size-aarch64-step-64 [functions len state]
  (if (= (vector-get state 0) 1)
    state
    (do
      (root_push functions)
      (root_push state)
      (let [next-state (callable-user-total-size-aarch64-step-64 functions (vector-get state 1) len (vector-get state 2))]
        (do
          (root_push next-state)
          (let [result (continue-callable-user-total-size-aarch64-step-64 functions len next-state)]
            (do
              (root_pop)
              (root_pop)
              (root_pop)
              result)))))))

(defn callable-user-total-size-aarch64 [functions import-count]
  (vector-get
    (continue-callable-user-total-size-aarch64-step-64
      functions
      (vector-length functions)
      (callable-user-total-size-aarch64-step-64 functions import-count (vector-length functions) 0))
    2))

(defn collect-function-starts-aarch64 [functions]
  (collect-callable-function-starts-aarch64 functions 0))

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

(defn emit-call-bundle-aarch64-twenty-to-twenty-nine [target-param-count disp frame-base-slot-count]
  (if (= target-param-count 29)
    (emit-twenty-nine-arg-call-aarch64 disp frame-base-slot-count)
    (emit-call-bundle-aarch64-twenty-to-twenty-eight target-param-count disp frame-base-slot-count)))

(defn emit-call-bundle-aarch64-twenty-to-thirty [target-param-count disp frame-base-slot-count]
  (if (= target-param-count 30)
    (emit-thirty-arg-call-aarch64 disp frame-base-slot-count)
    (emit-call-bundle-aarch64-twenty-to-twenty-nine target-param-count disp frame-base-slot-count)))

(defn emit-call-bundle-aarch64-twenty-to-thirty-one [target-param-count disp frame-base-slot-count]
  (if (= target-param-count 31)
    (emit-thirty-one-arg-call-aarch64 disp frame-base-slot-count)
    (emit-call-bundle-aarch64-twenty-to-thirty target-param-count disp frame-base-slot-count)))

(defn emit-call-bundle-aarch64-twenty-to-thirty-two [target-param-count disp frame-base-slot-count]
  (if (= target-param-count 32)
    (emit-thirty-two-arg-call-aarch64 disp frame-base-slot-count)
    (emit-call-bundle-aarch64-twenty-to-thirty-one target-param-count disp frame-base-slot-count)))

(defn emit-call-bundle-aarch64-twenty-to-thirty-three [target-param-count disp frame-base-slot-count]
  (if (= target-param-count 33)
    (emit-thirty-three-arg-call-aarch64 disp frame-base-slot-count)
    (emit-call-bundle-aarch64-twenty-to-thirty-two target-param-count disp frame-base-slot-count)))

(defn emit-call-bundle-aarch64-twenty-to-thirty-four [target-param-count disp frame-base-slot-count]
  (if (= target-param-count 34)
    (emit-thirty-four-arg-call-aarch64 disp frame-base-slot-count)
    (emit-call-bundle-aarch64-twenty-to-thirty-three target-param-count disp frame-base-slot-count)))

(defn emit-call-bundle-aarch64-twenty-to-thirty-five [target-param-count disp frame-base-slot-count]
  (if (= target-param-count 35)
    (emit-thirty-five-arg-call-aarch64 disp frame-base-slot-count)
    (emit-call-bundle-aarch64-twenty-to-thirty-four target-param-count disp frame-base-slot-count)))

(defn emit-call-bundle-aarch64-twenty-to-thirty-six [target-param-count disp frame-base-slot-count]
  (if (= target-param-count 36)
    (emit-thirty-six-arg-call-aarch64 disp frame-base-slot-count)
    (emit-call-bundle-aarch64-twenty-to-thirty-five target-param-count disp frame-base-slot-count)))

(defn emit-call-bundle-aarch64-twenty-to-thirty-seven [target-param-count disp frame-base-slot-count]
  (if (= target-param-count 37)
    (emit-thirty-seven-arg-call-aarch64 disp frame-base-slot-count)
    (emit-call-bundle-aarch64-twenty-to-thirty-six target-param-count disp frame-base-slot-count)))

(defn emit-call-bundle-aarch64-twenty-to-thirty-eight [target-param-count disp frame-base-slot-count]
  (if (= target-param-count 38)
    (emit-thirty-eight-arg-call-aarch64 disp frame-base-slot-count)
    (emit-call-bundle-aarch64-twenty-to-thirty-seven target-param-count disp frame-base-slot-count)))

(defn emit-call-bundle-aarch64-twenty-to-thirty-nine [target-param-count disp frame-base-slot-count]
  (if (= target-param-count 39)
    (emit-thirty-nine-arg-call-aarch64 disp frame-base-slot-count)
    (emit-call-bundle-aarch64-twenty-to-thirty-eight target-param-count disp frame-base-slot-count)))

(defn emit-call-bundle-aarch64-twenty-to-forty [target-param-count disp frame-base-slot-count]
  (if (= target-param-count 40)
    (emit-forty-arg-call-aarch64 disp frame-base-slot-count)
    (emit-call-bundle-aarch64-twenty-to-thirty-nine target-param-count disp frame-base-slot-count)))

(defn emit-call-bundle-aarch64-twenty-to-forty-one [target-param-count disp frame-base-slot-count]
  (if (= target-param-count 41)
    (emit-forty-one-arg-call-aarch64 disp frame-base-slot-count)
    (emit-call-bundle-aarch64-twenty-to-forty target-param-count disp frame-base-slot-count)))

(defn emit-call-bundle-aarch64-twenty-to-forty-two [target-param-count disp frame-base-slot-count]
  (if (= target-param-count 42)
    (emit-forty-two-arg-call-aarch64 disp frame-base-slot-count)
    (emit-call-bundle-aarch64-twenty-to-forty-one target-param-count disp frame-base-slot-count)))

(defn emit-call-bundle-aarch64-twenty-to-forty-three [target-param-count disp frame-base-slot-count]
  (if (= target-param-count 43)
    (emit-forty-three-arg-call-aarch64 disp frame-base-slot-count)
    (emit-call-bundle-aarch64-twenty-to-forty-two target-param-count disp frame-base-slot-count)))

(defn emit-call-bundle-aarch64-twenty-to-forty-four [target-param-count disp frame-base-slot-count]
  (if (= target-param-count 44)
    (emit-forty-four-arg-call-aarch64 disp frame-base-slot-count)
    (emit-call-bundle-aarch64-twenty-to-forty-three target-param-count disp frame-base-slot-count)))

(defn emit-call-bundle-aarch64-twenty-to-forty-five [target-param-count disp frame-base-slot-count]
  (if (= target-param-count 45)
    (emit-forty-five-arg-call-aarch64 disp frame-base-slot-count)
    (emit-call-bundle-aarch64-twenty-to-forty-four target-param-count disp frame-base-slot-count)))

(defn emit-call-bundle-aarch64-twenty-to-forty-six [target-param-count disp frame-base-slot-count]
  (if (= target-param-count 46)
    (emit-forty-six-arg-call-aarch64 disp frame-base-slot-count)
    (emit-call-bundle-aarch64-twenty-to-forty-five target-param-count disp frame-base-slot-count)))

(defn emit-call-bundle-aarch64-twenty-to-forty-seven [target-param-count disp frame-base-slot-count]
  (if (= target-param-count 47)
    (emit-forty-seven-arg-call-aarch64 disp frame-base-slot-count)
    (emit-call-bundle-aarch64-twenty-to-forty-six target-param-count disp frame-base-slot-count)))

(defn emit-call-bundle-aarch64-twenty-to-forty-eight [target-param-count disp frame-base-slot-count]
  (if (= target-param-count 48)
    (emit-forty-eight-arg-call-aarch64 disp frame-base-slot-count)
    (emit-call-bundle-aarch64-twenty-to-forty-seven target-param-count disp frame-base-slot-count)))

(defn emit-call-bundle-aarch64-twenty-to-forty-nine [target-param-count disp frame-base-slot-count]
  (if (= target-param-count 49)
    (emit-forty-nine-arg-call-aarch64 disp frame-base-slot-count)
    (emit-call-bundle-aarch64-twenty-to-forty-eight target-param-count disp frame-base-slot-count)))

(defn emit-call-bundle-aarch64-twenty-to-fifty [target-param-count disp frame-base-slot-count]
  (if (= target-param-count 50)
    (emit-fifty-arg-call-aarch64 disp frame-base-slot-count)
    (emit-call-bundle-aarch64-twenty-to-forty-nine target-param-count disp frame-base-slot-count)))

(defn emit-call-bundle-aarch64-twenty-to-fifty-one [target-param-count disp frame-base-slot-count]
  (if (= target-param-count 51)
    (emit-fifty-one-arg-call-aarch64 disp frame-base-slot-count)
    (emit-call-bundle-aarch64-twenty-to-fifty target-param-count disp frame-base-slot-count)))

(defn emit-call-bundle-aarch64-twenty-to-fifty-two [target-param-count disp frame-base-slot-count]
  (if (= target-param-count 52)
    (emit-fifty-two-arg-call-aarch64 disp frame-base-slot-count)
    (emit-call-bundle-aarch64-twenty-to-fifty-one target-param-count disp frame-base-slot-count)))

(defn emit-call-bundle-aarch64-twenty-to-fifty-three [target-param-count disp frame-base-slot-count]
  (if (= target-param-count 53)
    (emit-fifty-three-arg-call-aarch64 disp frame-base-slot-count)
    (emit-call-bundle-aarch64-twenty-to-fifty-two target-param-count disp frame-base-slot-count)))

(defn emit-call-bundle-aarch64-twenty-to-fifty-four [target-param-count disp frame-base-slot-count]
  (if (= target-param-count 54)
    (emit-fifty-four-arg-call-aarch64 disp frame-base-slot-count)
    (emit-call-bundle-aarch64-twenty-to-fifty-three target-param-count disp frame-base-slot-count)))

(defn emit-call-bundle-aarch64-twenty-to-fifty-five [target-param-count disp frame-base-slot-count]
  (if (= target-param-count 55)
    (emit-fifty-five-arg-call-aarch64 disp frame-base-slot-count)
    (emit-call-bundle-aarch64-twenty-to-fifty-four target-param-count disp frame-base-slot-count)))

(defn emit-call-bundle-aarch64-twenty-to-fifty-six [target-param-count disp frame-base-slot-count]
  (if (= target-param-count 56)
    (emit-fifty-six-arg-call-aarch64 disp frame-base-slot-count)
    (emit-call-bundle-aarch64-twenty-to-fifty-five target-param-count disp frame-base-slot-count)))

(defn emit-call-bundle-aarch64-twenty-to-fifty-seven [target-param-count disp frame-base-slot-count]
  (if (= target-param-count 57)
    (emit-fifty-seven-arg-call-aarch64 disp frame-base-slot-count)
    (emit-call-bundle-aarch64-twenty-to-fifty-six target-param-count disp frame-base-slot-count)))

(defn emit-call-bundle-aarch64-twenty-to-fifty-eight [target-param-count disp frame-base-slot-count]
  (if (= target-param-count 58)
    (emit-fifty-eight-arg-call-aarch64 disp frame-base-slot-count)
    (emit-call-bundle-aarch64-twenty-to-fifty-seven target-param-count disp frame-base-slot-count)))

(defn emit-call-bundle-aarch64-twenty-to-sixty [target-param-count disp frame-base-slot-count]
  (if (> target-param-count 60)
    (emit-twenty-plus-arg-call-aarch64 target-param-count disp frame-base-slot-count)
    (if (= target-param-count 60)
      (emit-sixty-arg-call-aarch64 disp frame-base-slot-count)
      (if (= target-param-count 59)
        (emit-fifty-nine-arg-call-aarch64 disp frame-base-slot-count)
        (emit-call-bundle-aarch64-twenty-to-fifty-eight target-param-count disp frame-base-slot-count)))))

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
    (emit-consume-produce-one-bundle-aarch64
      (emit-nine-arg-call-aarch64 disp frame-base-slot-count)
      frame-base-slot-count
      current-depth
      9)
    (if (= target-param-count 8)
      (emit-consume-produce-one-bundle-aarch64
        (emit-eight-arg-call-aarch64 disp frame-base-slot-count)
        frame-base-slot-count
        current-depth
        8)
      (if (= target-param-count 7)
        (emit-consume-produce-one-bundle-aarch64
          (emit-seven-arg-call-aarch64 disp frame-base-slot-count)
          frame-base-slot-count
          current-depth
          7)
        (if (= target-param-count 6)
          (emit-consume-produce-one-bundle-aarch64
            (emit-six-arg-call-aarch64 disp frame-base-slot-count)
            frame-base-slot-count
            current-depth
            6)
          (if (= target-param-count 5)
            (emit-consume-produce-one-bundle-aarch64
              (emit-five-arg-call-aarch64 disp frame-base-slot-count)
              frame-base-slot-count
              current-depth
              5)
            (if (= target-param-count 4)
              (emit-consume-produce-one-bundle-aarch64
                (emit-four-arg-call-aarch64 disp frame-base-slot-count)
                frame-base-slot-count
                current-depth
                4)
              (if (= target-param-count 3)
                 (emit-consume-produce-one-bundle-aarch64
                   (emit-three-arg-call-aarch64 disp frame-base-slot-count)
                   frame-base-slot-count
                   current-depth
                   3)
                    (if (= target-param-count 2)
                     (emit-two-arg-call-aarch64 disp frame-base-slot-count current-depth)
                    (if (= target-param-count 1)
                      (let [save-prev (emit-aarch64-mov-x10-x9)
                        call-bl (emit-aarch64-bl disp)
                        restore-prev (emit-aarch64-mov-x9-x10)]
                        (concat-three-byte-vectors-rooted save-prev call-bl restore-prev))
                       (emit-produce-one-bundle-aarch64
                         (emit-aarch64-bl disp)
                         frame-base-slot-count
                         current-depth)))))))))))

(defn emit-aarch64-helper-call-preserving-prev-and-lr [disp]
  (let [save-frame (emit-aarch64-save-x9-x30)
    call-bl (emit-aarch64-bl disp)
    restore-frame (emit-aarch64-restore-x9-x30)]
     (concat-three-byte-vectors-rooted save-frame call-bl restore-frame)))

(defn emit-aarch64-import-stub [import-idx import-count import-stub-offset]
  (if (= import-idx 1)
    (emit-aarch64-b
      (- (aarch64-selfhost-alloc-helper-offset import-stub-offset import-count)
         (aarch64-import-ret-stub-offset import-stub-offset import-count import-idx)))
    (emit-aarch64-ret)))

(defn make-native-progress-state [done next-idx]
  (let [base0 (vector-push (vector-new 2) done)]
    (do
      (root_push base0)
      (let [state (vector-push base0 next-idx)]
        (do
          (root_pop)
          state)))))

(defn append-aarch64-import-stubs-step [result import-count import-stub-offset idx stub-count]
  (if (>= idx stub-count)
    (make-native-progress-state 1 idx)
    (do
      (append-native-bytes-rooted
        result
        (if (> import-count 0)
          (emit-aarch64-import-stub idx import-count import-stub-offset)
          (emit-aarch64-ret))
        4)
      (make-native-progress-state 0 (+ idx 1)))))

(defn append-aarch64-import-stubs-step-64-loop-bounded [result import-count import-stub-offset idx stub-count remaining]
  (do
    (root_push result)
    (let [state (append-aarch64-import-stubs-step result import-count import-stub-offset idx stub-count)]
      (do
        (root_push state)
        (let [final
              (if (= (vector-get state 0) 1)
                state
                (if (<= remaining 1)
                  state
                  (append-aarch64-import-stubs-step-64-loop-bounded result import-count import-stub-offset (vector-get state 1) stub-count (- remaining 1))))]
          (do
            (root_pop)
            (root_pop)
            final))))))

(defn append-aarch64-import-stubs-step-64 [result import-count import-stub-offset idx stub-count]
  (append-aarch64-import-stubs-step-64-loop-bounded result import-count import-stub-offset idx stub-count 64))

(defn continue-append-aarch64-import-stubs-step-64 [result import-count import-stub-offset stub-count state]
  (if (= (vector-get state 0) 1)
    state
    (do
      (root_push result)
      (root_push state)
      (let [next-state (append-aarch64-import-stubs-step-64 result import-count import-stub-offset (vector-get state 1) stub-count)]
        (do
          (root_push next-state)
          (let [final (continue-append-aarch64-import-stubs-step-64 result import-count import-stub-offset stub-count next-state)]
            (do
              (root_pop)
              (root_pop)
              (root_pop)
              final)))))))

(defn append-aarch64-import-stubs-loop [result import-count import-stub-offset idx stub-count]
  (continue-append-aarch64-import-stubs-step-64
    result
    import-count
    import-stub-offset
    stub-count
    (append-aarch64-import-stubs-step-64 result import-count import-stub-offset idx stub-count)))

(defn codegen-selfhost-runtime-bundle-aarch64-tail [opcode current-offset import-stub-offset import-count frame-base-slot-count current-depth]
  (if (= opcode 62)
    (emit-map-insert-bundle-aarch64
      (- (aarch64-selfhost-map-insert-helper-offset import-stub-offset import-count) (+ current-offset 8))
      frame-base-slot-count
      current-depth)
    (if (= opcode 63)
      (emit-consume-two-bundle-aarch64
        (emit-aarch64-helper-call-preserving-prev-and-lr
          (- (aarch64-selfhost-map-get-helper-offset import-stub-offset import-count) (+ current-offset 4)))
        frame-base-slot-count
        current-depth)
      (if (= opcode 64)
        (emit-aarch64-helper-call-preserving-prev-and-lr
          (- (aarch64-selfhost-read-file-helper-offset import-stub-offset import-count) (+ current-offset 4)))
        (if (= opcode 73)
          (emit-aarch64-helper-call-preserving-prev-and-lr
            (- (aarch64-selfhost-file-exists-helper-offset import-stub-offset import-count) (+ current-offset 4)))
          (if (= opcode 67)
            (emit-aarch64-helper-call-preserving-prev-and-lr
              (- (aarch64-selfhost-command-line-arg-helper-offset import-stub-offset import-count) (+ current-offset 4)))
            (if (= opcode 59)
              (emit-aarch64-helper-call-preserving-prev-and-lr
                (- (aarch64-selfhost-print-helper-offset import-stub-offset import-count) (+ current-offset 4)))
              (if (= opcode 54)
                (emit-aarch64-helper-call-preserving-prev-and-lr
                  (- (aarch64-selfhost-vector-new-helper-offset import-stub-offset import-count) (+ current-offset 4)))
                (if (= opcode 52)
                  (emit-aarch64-helper-call-preserving-prev-and-lr
                    (- (aarch64-selfhost-vector-length-helper-offset import-stub-offset import-count) (+ current-offset 4)))
                  (if (= opcode 56)
                    (emit-aarch64-helper-call-preserving-prev-and-lr
                      (- (aarch64-selfhost-ref-new-helper-offset import-stub-offset import-count) (+ current-offset 4)))
                    (if (= opcode 57)
                      (emit-aarch64-helper-call-preserving-prev-and-lr
                        (- (aarch64-selfhost-ref-get-helper-offset import-stub-offset import-count) (+ current-offset 4)))
                      (vector-new 0))))))))))))

(defn codegen-selfhost-runtime-bundle-aarch64 [opcode current-offset import-stub-offset import-count frame-base-slot-count current-depth]
  (if (= opcode 51)
    (emit-aarch64-helper-call-preserving-prev-and-lr
      (- (aarch64-selfhost-string-length-helper-offset import-stub-offset import-count) (+ current-offset 4)))
    (if (= opcode 50)
      (emit-consume-two-bundle-aarch64
        (emit-aarch64-helper-call-preserving-prev-and-lr
          (- (aarch64-selfhost-string-char-at-helper-offset import-stub-offset import-count) (+ current-offset 4)))
        frame-base-slot-count
        current-depth)
      (if (= opcode 53)
        (emit-consume-two-bundle-aarch64
          (emit-aarch64-helper-call-preserving-prev-and-lr
            (- (aarch64-selfhost-vector-get-helper-offset import-stub-offset import-count) (+ current-offset 4)))
          frame-base-slot-count
          current-depth)
        (if (= opcode 55)
          (emit-consume-two-bundle-aarch64
            (emit-aarch64-helper-call-preserving-prev-and-lr
              (- (aarch64-selfhost-vector-push-helper-offset import-stub-offset import-count) (+ current-offset 4)))
            frame-base-slot-count
            current-depth)
          (if (= opcode 58)
            (emit-consume-two-bundle-aarch64
              (emit-aarch64-helper-call-preserving-prev-and-lr
                (- (aarch64-selfhost-ref-set-helper-offset import-stub-offset import-count) (+ current-offset 4)))
              frame-base-slot-count
              current-depth)
            (if (= opcode 69)
              (emit-substring-bundle-aarch64
                (- (aarch64-selfhost-substring-helper-offset import-stub-offset import-count) (+ current-offset 8))
                frame-base-slot-count
                current-depth)
              (if (= opcode 70)
                (emit-consume-two-bundle-aarch64
                  (emit-aarch64-helper-call-preserving-prev-and-lr
                    (- (aarch64-selfhost-string-concat-helper-offset import-stub-offset import-count) (+ current-offset 4)))
                  frame-base-slot-count
                  current-depth)
                (if (= opcode 60)
                  (emit-produce-one-bundle-aarch64
                    (emit-aarch64-helper-call-preserving-prev-and-lr
                      (- (aarch64-selfhost-map-new-helper-offset import-stub-offset import-count) (+ current-offset 4)))
                    frame-base-slot-count
                    current-depth)
                  (if (= opcode 61)
                    (emit-aarch64-helper-call-preserving-prev-and-lr
                      (- (aarch64-selfhost-map-size-helper-offset import-stub-offset import-count) (+ current-offset 4)))
                    (codegen-selfhost-runtime-bundle-aarch64-tail opcode current-offset import-stub-offset import-count frame-base-slot-count current-depth)))))))))))

(defn codegen-ir-instr-bundle-aarch64-with-import-count [opcode operand current-offset function-starts function-metas import-count import-stub-offset frame-base-slot-count current-depth]
  (if (= opcode 40)
    (let [target-meta (vector-get function-metas operand)
      target-offset (if (< operand import-count)
                      (aarch64-import-ret-stub-offset import-stub-offset import-count operand)
                      (vector-get function-starts (- operand import-count)))
      target-param-count (native-function-param-count target-meta)
      disp (if (>= target-param-count 20)
             (native-call-bundle-disp-aarch64-twenty-to-sixty target-param-count target-offset current-offset)
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
                                  (- target-offset (+ current-offset (native-produce-one-prefix-size-aarch64 4 current-depth))))))))))))))
      call-bytes (if (>= target-param-count 20)
                 (emit-call-bundle-aarch64-twenty-to-sixty target-param-count disp frame-base-slot-count)
                  (if (>= target-param-count 10)
                    (emit-call-bundle-aarch64-ten-to-nineteen target-param-count disp frame-base-slot-count)
                    (emit-call-bundle-aarch64-one-to-nine target-param-count disp frame-base-slot-count current-depth)))]
      call-bytes)
      (if (= opcode 1)
        (emit-i64-const-bundle-aarch64 operand frame-base-slot-count current-depth)
        (if (= opcode 3)
          (emit-i32-const-bundle-aarch64 operand frame-base-slot-count current-depth)
          (if (= opcode 74)
            (emit-root-push-aarch64)
            (if (= opcode 75)
              (emit-produce-one-bundle-aarch64
                (emit-root-pop-aarch64)
                frame-base-slot-count
                current-depth)
              (let [selfhost-runtime-bundle (codegen-selfhost-runtime-bundle-aarch64
                                            opcode
                                            current-offset
                                            import-stub-offset
                                            import-count
                                            frame-base-slot-count
                                            current-depth)]
                (if (> (vector-length selfhost-runtime-bundle) 0)
                  selfhost-runtime-bundle
                  (if (= opcode 10)
                    (emit-local-get-bundle-aarch64 (local-slot-offset operand) frame-base-slot-count current-depth)
                      (if (= opcode 11)
                        (emit-local-set-bundle-aarch64 (local-slot-offset operand) frame-base-slot-count current-depth)
                      (if (= opcode 44)
                        (emit-drop-bundle-aarch64 frame-base-slot-count current-depth)
                        (if (= opcode 76)
                          (emit-root-set-bundle-aarch64 frame-base-slot-count current-depth)
                          (if (= opcode 46)
                            (emit-i32-store-bundle-aarch64 operand frame-base-slot-count current-depth)
                            (if (= opcode 49)
                              (emit-i64-store-bundle-aarch64 operand frame-base-slot-count current-depth)
                              (if (= opcode 77)
                                (emit-memory-copy-bundle-aarch64 frame-base-slot-count current-depth)
                                (if (= opcode 78)
                                  (emit-memory-fill-bundle-aarch64 frame-base-slot-count current-depth)
                                  (let [plain-native (codegen-ir-instr-aarch64 opcode operand)]
                                    (if (= (opcode-stack-delta opcode operand function-metas) -1)
                                      (emit-consume-two-bundle-aarch64 plain-native frame-base-slot-count current-depth)
                                      plain-native))))))))))))))))))

(defn codegen-ir-instr-bundle-aarch64 [opcode operand current-offset function-starts function-metas frame-base-slot-count current-depth]
  (codegen-ir-instr-bundle-aarch64-with-import-count opcode operand current-offset function-starts function-metas 0 0 frame-base-slot-count current-depth))

(defn generate-native-instr-bundle-loop-aarch64-with-import-count [ir-func result function-starts function-metas import-count import-stub-offset frame-base-slot-count current-offset current-depth idx len]
  (if (>= idx len)
    current-offset
    (let [instr (vector-get ir-func idx)
      opcode (vector-get instr 0)
      operand (vector-get instr 1)
      native (codegen-ir-instr-bundle-aarch64-with-import-count opcode operand current-offset function-starts function-metas import-count import-stub-offset frame-base-slot-count current-depth)
      native-len (vector-length native)
      next-depth (apply-stack-delta current-depth (opcode-stack-delta opcode operand function-metas))]
      (do
        (root_push native)
        (append-native-bytes-loop result native 0 native-len)
        (root_pop)
        (generate-native-instr-bundle-loop-aarch64-with-import-count ir-func result function-starts function-metas import-count import-stub-offset frame-base-slot-count (+ current-offset native-len) next-depth (+ idx 1) len)))))

(defn generate-native-instr-bundle-loop-aarch64 [ir-func result function-starts function-metas frame-base-slot-count current-offset current-depth idx len]
  (generate-native-instr-bundle-loop-aarch64-with-import-count ir-func result function-starts function-metas 0 0 frame-base-slot-count current-offset current-depth idx len))

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
  (do
    (root_push ir-func)
    (let [result (ref-new (vector-new 16))]
      (do
        (root_push result)
        (let [stack-bytes (native-local-stack-bytes ir-func)
          prologue-stack-bytes (if (= (aarch64-plain-stack-padding-needed ir-func) 1)
                                 (align-16 (+ stack-bytes 8))
                                 stack-bytes)
          has-call (native-has-call ir-func)
          control-meta (scan-control-flow-meta ir-func)
          offsets (collect-native-offsets-aarch64 ir-func)
          n (vector-length ir-func)]
          (do
            (root_push control-meta)
            (root_push offsets)
            (if (= has-call 1)
              (append-native-bytes-loop result (emit-aarch64-save-fp-lr) 0 4)
              0)
            (if (> prologue-stack-bytes 0)
              (append-native-bytes-loop result (emit-aarch64-sub-sp prologue-stack-bytes) 0 4)
              0)
            (generate-native-control-instr-loop-aarch64 ir-func result control-meta offsets 0 n)
            (let [ret-bytes (emit-aarch64-ret)]
              (do
                (root_push ret-bytes)
                (if (> prologue-stack-bytes 0)
                  (append-native-bytes-loop result (emit-aarch64-add-sp prologue-stack-bytes) 0 4)
                  0)
                (if (= has-call 1)
                  (append-native-bytes-loop result (emit-aarch64-restore-fp-lr) 0 4)
                  0)
                (append-native-bytes-loop result ret-bytes 0 4)
                (let [final (ref-get result)]
                  (do
                    (root_pop)
                    (root_pop)
                    (root_pop)
                    (root_pop)
                    (root_pop)
                    final))))))))))

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

(defn spill-native-function-params-aarch64-twenty-to-twenty-nine [param-count result stack-arg-base-offset]
  (if (= param-count 29)
    (do
      (spill-native-function-params-aarch64-twenty-to-twenty-eight 28 result stack-arg-base-offset)
      (append-native-bytes-loop result (emit-aarch64-ldr-x10-sp (+ stack-arg-base-offset 160)) 0 4)
      (append-native-bytes-loop result (emit-aarch64-str-x10-sp (local-slot-offset 28)) 0 4))
    (spill-native-function-params-aarch64-twenty-to-twenty-eight param-count result stack-arg-base-offset)))

(defn spill-native-function-params-aarch64-twenty-to-thirty [param-count result stack-arg-base-offset]
  (if (= param-count 30)
    (do
      (spill-native-function-params-aarch64-twenty-to-twenty-nine 29 result stack-arg-base-offset)
      (append-native-bytes-loop result (emit-aarch64-ldr-x10-sp (+ stack-arg-base-offset 168)) 0 4)
      (append-native-bytes-loop result (emit-aarch64-str-x10-sp (local-slot-offset 29)) 0 4))
    (spill-native-function-params-aarch64-twenty-to-twenty-nine param-count result stack-arg-base-offset)))

(defn spill-native-function-params-aarch64-twenty-to-thirty-one [param-count result stack-arg-base-offset]
  (if (= param-count 31)
    (do
      (spill-native-function-params-aarch64-twenty-to-thirty 30 result stack-arg-base-offset)
      (append-native-bytes-loop result (emit-aarch64-ldr-x10-sp (+ stack-arg-base-offset 176)) 0 4)
      (append-native-bytes-loop result (emit-aarch64-str-x10-sp (local-slot-offset 30)) 0 4))
    (spill-native-function-params-aarch64-twenty-to-thirty param-count result stack-arg-base-offset)))

(defn spill-native-function-params-aarch64-twenty-to-thirty-two [param-count result stack-arg-base-offset]
  (if (= param-count 32)
    (do
      (spill-native-function-params-aarch64-twenty-to-thirty-one 31 result stack-arg-base-offset)
      (append-native-bytes-loop result (emit-aarch64-ldr-x10-sp (+ stack-arg-base-offset 184)) 0 4)
      (append-native-bytes-loop result (emit-aarch64-str-x10-sp (local-slot-offset 31)) 0 4))
    (spill-native-function-params-aarch64-twenty-to-thirty-one param-count result stack-arg-base-offset)))

(defn spill-native-function-params-aarch64-twenty-to-thirty-three [param-count result stack-arg-base-offset]
  (if (= param-count 33)
    (do
      (spill-native-function-params-aarch64-twenty-to-thirty-two 32 result stack-arg-base-offset)
      (append-native-bytes-loop result (emit-aarch64-ldr-x10-sp (+ stack-arg-base-offset 192)) 0 4)
      (append-native-bytes-loop result (emit-aarch64-str-x10-sp (local-slot-offset 32)) 0 4))
    (spill-native-function-params-aarch64-twenty-to-thirty-two param-count result stack-arg-base-offset)))

(defn spill-native-function-params-aarch64-twenty-to-thirty-four [param-count result stack-arg-base-offset]
  (if (= param-count 34)
    (do
      (spill-native-function-params-aarch64-twenty-to-thirty-three 33 result stack-arg-base-offset)
      (append-native-bytes-loop result (emit-aarch64-ldr-x10-sp (+ stack-arg-base-offset 200)) 0 4)
      (append-native-bytes-loop result (emit-aarch64-str-x10-sp (local-slot-offset 33)) 0 4))
    (spill-native-function-params-aarch64-twenty-to-thirty-three param-count result stack-arg-base-offset)))

(defn spill-native-function-params-aarch64-twenty-to-thirty-five [param-count result stack-arg-base-offset]
  (if (= param-count 35)
    (do
      (spill-native-function-params-aarch64-twenty-to-thirty-four 34 result stack-arg-base-offset)
      (append-native-bytes-loop result (emit-aarch64-ldr-x10-sp (+ stack-arg-base-offset 208)) 0 4)
      (append-native-bytes-loop result (emit-aarch64-str-x10-sp (local-slot-offset 34)) 0 4))
    (spill-native-function-params-aarch64-twenty-to-thirty-four param-count result stack-arg-base-offset)))

(defn spill-native-function-params-aarch64-twenty-to-thirty-six [param-count result stack-arg-base-offset]
  (if (= param-count 36)
    (do
      (spill-native-function-params-aarch64-twenty-to-thirty-five 35 result stack-arg-base-offset)
      (append-native-bytes-loop result (emit-aarch64-ldr-x10-sp (+ stack-arg-base-offset 216)) 0 4)
      (append-native-bytes-loop result (emit-aarch64-str-x10-sp (local-slot-offset 35)) 0 4))
    (spill-native-function-params-aarch64-twenty-to-thirty-five param-count result stack-arg-base-offset)))

(defn spill-native-function-params-aarch64-twenty-to-thirty-seven [param-count result stack-arg-base-offset]
  (if (= param-count 37)
    (do
      (spill-native-function-params-aarch64-twenty-to-thirty-six 36 result stack-arg-base-offset)
      (append-native-bytes-loop result (emit-aarch64-ldr-x10-sp (+ stack-arg-base-offset 224)) 0 4)
      (append-native-bytes-loop result (emit-aarch64-str-x10-sp (local-slot-offset 36)) 0 4))
    (spill-native-function-params-aarch64-twenty-to-thirty-six param-count result stack-arg-base-offset)))

(defn spill-native-function-params-aarch64-twenty-to-thirty-eight [param-count result stack-arg-base-offset]
  (if (= param-count 38)
    (do
      (spill-native-function-params-aarch64-twenty-to-thirty-seven 37 result stack-arg-base-offset)
      (append-native-bytes-loop result (emit-aarch64-ldr-x10-sp (+ stack-arg-base-offset 232)) 0 4)
      (append-native-bytes-loop result (emit-aarch64-str-x10-sp (local-slot-offset 37)) 0 4))
    (spill-native-function-params-aarch64-twenty-to-thirty-seven param-count result stack-arg-base-offset)))

(defn spill-native-function-params-aarch64-twenty-to-thirty-nine [param-count result stack-arg-base-offset]
  (if (= param-count 39)
    (do
      (spill-native-function-params-aarch64-twenty-to-thirty-eight 38 result stack-arg-base-offset)
      (append-native-bytes-loop result (emit-aarch64-ldr-x10-sp (+ stack-arg-base-offset 240)) 0 4)
      (append-native-bytes-loop result (emit-aarch64-str-x10-sp (local-slot-offset 38)) 0 4))
    (spill-native-function-params-aarch64-twenty-to-thirty-eight param-count result stack-arg-base-offset)))

(defn spill-native-function-params-aarch64-twenty-to-forty [param-count result stack-arg-base-offset]
  (if (= param-count 40)
    (do
      (spill-native-function-params-aarch64-twenty-to-thirty-nine 39 result stack-arg-base-offset)
      (append-native-bytes-loop result (emit-aarch64-ldr-x10-sp (+ stack-arg-base-offset 248)) 0 4)
      (append-native-bytes-loop result (emit-aarch64-str-x10-sp (local-slot-offset 39)) 0 4))
    (spill-native-function-params-aarch64-twenty-to-thirty-nine param-count result stack-arg-base-offset)))

(defn spill-native-function-params-aarch64-twenty-to-forty-one [param-count result stack-arg-base-offset]
  (if (= param-count 41)
    (do
      (spill-native-function-params-aarch64-twenty-to-forty 40 result stack-arg-base-offset)
      (append-native-bytes-loop result (emit-aarch64-ldr-x10-sp (+ stack-arg-base-offset 256)) 0 4)
      (append-native-bytes-loop result (emit-aarch64-str-x10-sp (local-slot-offset 40)) 0 4))
    (spill-native-function-params-aarch64-twenty-to-forty param-count result stack-arg-base-offset)))

(defn spill-native-function-params-aarch64-twenty-to-forty-two [param-count result stack-arg-base-offset]
  (if (= param-count 42)
    (do
      (spill-native-function-params-aarch64-twenty-to-forty-one 41 result stack-arg-base-offset)
      (append-native-bytes-loop result (emit-aarch64-ldr-x10-sp (+ stack-arg-base-offset 264)) 0 4)
      (append-native-bytes-loop result (emit-aarch64-str-x10-sp (local-slot-offset 41)) 0 4))
    (spill-native-function-params-aarch64-twenty-to-forty-one param-count result stack-arg-base-offset)))

(defn spill-native-function-params-aarch64-twenty-to-forty-three [param-count result stack-arg-base-offset]
  (if (= param-count 43)
    (do
      (spill-native-function-params-aarch64-twenty-to-forty-two 42 result stack-arg-base-offset)
      (append-native-bytes-loop result (emit-aarch64-ldr-x10-sp (+ stack-arg-base-offset 272)) 0 4)
      (append-native-bytes-loop result (emit-aarch64-str-x10-sp (local-slot-offset 42)) 0 4))
    (spill-native-function-params-aarch64-twenty-to-forty-two param-count result stack-arg-base-offset)))

(defn spill-native-function-params-aarch64-twenty-to-forty-four [param-count result stack-arg-base-offset]
  (if (= param-count 44)
    (do
      (spill-native-function-params-aarch64-twenty-to-forty-three 43 result stack-arg-base-offset)
      (append-native-bytes-loop result (emit-aarch64-ldr-x10-sp (+ stack-arg-base-offset 280)) 0 4)
      (append-native-bytes-loop result (emit-aarch64-str-x10-sp (local-slot-offset 43)) 0 4))
    (spill-native-function-params-aarch64-twenty-to-forty-three param-count result stack-arg-base-offset)))

(defn spill-native-function-params-aarch64-twenty-to-forty-five [param-count result stack-arg-base-offset]
  (if (= param-count 45)
    (do
      (spill-native-function-params-aarch64-twenty-to-forty-four 44 result stack-arg-base-offset)
      (append-native-bytes-loop result (emit-aarch64-ldr-x10-sp (+ stack-arg-base-offset 288)) 0 4)
      (append-native-bytes-loop result (emit-aarch64-str-x10-sp (local-slot-offset 44)) 0 4))
    (spill-native-function-params-aarch64-twenty-to-forty-four param-count result stack-arg-base-offset)))

(defn spill-native-function-params-aarch64-twenty-to-forty-six [param-count result stack-arg-base-offset]
  (if (= param-count 46)
    (do
      (spill-native-function-params-aarch64-twenty-to-forty-five 45 result stack-arg-base-offset)
      (append-native-bytes-loop result (emit-aarch64-ldr-x10-sp (+ stack-arg-base-offset 296)) 0 4)
      (append-native-bytes-loop result (emit-aarch64-str-x10-sp (local-slot-offset 45)) 0 4))
    (spill-native-function-params-aarch64-twenty-to-forty-five param-count result stack-arg-base-offset)))

(defn spill-native-function-params-aarch64-twenty-to-forty-seven [param-count result stack-arg-base-offset]
  (if (= param-count 47)
    (do
      (spill-native-function-params-aarch64-twenty-to-forty-six 46 result stack-arg-base-offset)
      (append-native-bytes-loop result (emit-aarch64-ldr-x10-sp (+ stack-arg-base-offset 304)) 0 4)
      (append-native-bytes-loop result (emit-aarch64-str-x10-sp (local-slot-offset 46)) 0 4))
    (spill-native-function-params-aarch64-twenty-to-forty-six param-count result stack-arg-base-offset)))

(defn spill-native-function-params-aarch64-twenty-to-forty-eight [param-count result stack-arg-base-offset]
  (if (= param-count 48)
    (do
      (spill-native-function-params-aarch64-twenty-to-forty-seven 47 result stack-arg-base-offset)
      (append-native-bytes-loop result (emit-aarch64-ldr-x10-sp (+ stack-arg-base-offset 312)) 0 4)
      (append-native-bytes-loop result (emit-aarch64-str-x10-sp (local-slot-offset 47)) 0 4))
    (spill-native-function-params-aarch64-twenty-to-forty-seven param-count result stack-arg-base-offset)))

(defn spill-native-function-params-aarch64-twenty-to-forty-nine [param-count result stack-arg-base-offset]
  (if (= param-count 49)
    (do
      (spill-native-function-params-aarch64-twenty-to-forty-eight 48 result stack-arg-base-offset)
      (append-native-bytes-loop result (emit-aarch64-ldr-x10-sp (+ stack-arg-base-offset 320)) 0 4)
      (append-native-bytes-loop result (emit-aarch64-str-x10-sp (local-slot-offset 48)) 0 4))
    (spill-native-function-params-aarch64-twenty-to-forty-eight param-count result stack-arg-base-offset)))

(defn spill-native-function-params-aarch64-twenty-to-fifty [param-count result stack-arg-base-offset]
  (if (= param-count 50)
    (do
      (spill-native-function-params-aarch64-twenty-to-forty-nine 49 result stack-arg-base-offset)
      (append-native-bytes-loop result (emit-aarch64-ldr-x10-sp (+ stack-arg-base-offset 328)) 0 4)
      (append-native-bytes-loop result (emit-aarch64-str-x10-sp (local-slot-offset 49)) 0 4))
    (spill-native-function-params-aarch64-twenty-to-forty-nine param-count result stack-arg-base-offset)))

(defn spill-native-function-params-aarch64-twenty-to-fifty-one [param-count result stack-arg-base-offset]
  (if (= param-count 51)
    (do
      (spill-native-function-params-aarch64-twenty-to-fifty 50 result stack-arg-base-offset)
      (append-native-bytes-loop result (emit-aarch64-ldr-x10-sp (+ stack-arg-base-offset 336)) 0 4)
      (append-native-bytes-loop result (emit-aarch64-str-x10-sp (local-slot-offset 50)) 0 4))
    (spill-native-function-params-aarch64-twenty-to-fifty param-count result stack-arg-base-offset)))

(defn spill-native-function-params-aarch64-twenty-to-fifty-two [param-count result stack-arg-base-offset]
  (if (= param-count 52)
    (do
      (spill-native-function-params-aarch64-twenty-to-fifty-one 51 result stack-arg-base-offset)
      (append-native-bytes-loop result (emit-aarch64-ldr-x10-sp (+ stack-arg-base-offset 344)) 0 4)
      (append-native-bytes-loop result (emit-aarch64-str-x10-sp (local-slot-offset 51)) 0 4))
    (spill-native-function-params-aarch64-twenty-to-fifty-one param-count result stack-arg-base-offset)))

(defn spill-native-function-params-aarch64-twenty-to-fifty-three [param-count result stack-arg-base-offset]
  (if (= param-count 53)
    (do
      (spill-native-function-params-aarch64-twenty-to-fifty-two 52 result stack-arg-base-offset)
      (append-native-bytes-loop result (emit-aarch64-ldr-x10-sp (+ stack-arg-base-offset 352)) 0 4)
      (append-native-bytes-loop result (emit-aarch64-str-x10-sp (local-slot-offset 52)) 0 4))
    (spill-native-function-params-aarch64-twenty-to-fifty-two param-count result stack-arg-base-offset)))

(defn spill-native-function-params-aarch64-twenty-to-fifty-four [param-count result stack-arg-base-offset]
  (if (= param-count 54)
    (do
      (spill-native-function-params-aarch64-twenty-to-fifty-three 53 result stack-arg-base-offset)
      (append-native-bytes-loop result (emit-aarch64-ldr-x10-sp (+ stack-arg-base-offset 360)) 0 4)
      (append-native-bytes-loop result (emit-aarch64-str-x10-sp (local-slot-offset 53)) 0 4))
    (spill-native-function-params-aarch64-twenty-to-fifty-three param-count result stack-arg-base-offset)))

(defn spill-native-function-params-aarch64-twenty-to-fifty-five [param-count result stack-arg-base-offset]
  (if (= param-count 55)
    (do
      (spill-native-function-params-aarch64-twenty-to-fifty-four 54 result stack-arg-base-offset)
      (append-native-bytes-loop result (emit-aarch64-ldr-x10-sp (+ stack-arg-base-offset 368)) 0 4)
      (append-native-bytes-loop result (emit-aarch64-str-x10-sp (local-slot-offset 54)) 0 4))
    (spill-native-function-params-aarch64-twenty-to-fifty-four param-count result stack-arg-base-offset)))

(defn spill-native-function-params-aarch64-twenty-to-fifty-six [param-count result stack-arg-base-offset]
  (if (= param-count 56)
    (do
      (spill-native-function-params-aarch64-twenty-to-fifty-five 55 result stack-arg-base-offset)
      (append-native-bytes-loop result (emit-aarch64-ldr-x10-sp (+ stack-arg-base-offset 376)) 0 4)
      (append-native-bytes-loop result (emit-aarch64-str-x10-sp (local-slot-offset 55)) 0 4))
    (spill-native-function-params-aarch64-twenty-to-fifty-five param-count result stack-arg-base-offset)))

(defn spill-native-function-params-aarch64-twenty-to-fifty-seven [param-count result stack-arg-base-offset]
  (if (= param-count 57)
    (do
      (spill-native-function-params-aarch64-twenty-to-fifty-six 56 result stack-arg-base-offset)
      (append-native-bytes-loop result (emit-aarch64-ldr-x10-sp (+ stack-arg-base-offset 384)) 0 4)
      (append-native-bytes-loop result (emit-aarch64-str-x10-sp (local-slot-offset 56)) 0 4))
    (spill-native-function-params-aarch64-twenty-to-fifty-six param-count result stack-arg-base-offset)))

(defn spill-native-function-params-aarch64-twenty-to-fifty-eight [param-count result stack-arg-base-offset]
  (if (= param-count 58)
    (do
      (spill-native-function-params-aarch64-twenty-to-fifty-seven 57 result stack-arg-base-offset)
      (append-native-bytes-loop result (emit-aarch64-ldr-x10-sp (+ stack-arg-base-offset 392)) 0 4)
      (append-native-bytes-loop result (emit-aarch64-str-x10-sp (local-slot-offset 57)) 0 4))
    (spill-native-function-params-aarch64-twenty-to-fifty-seven param-count result stack-arg-base-offset)))

(defn spill-native-function-params-aarch64-twenty-to-sixty [param-count result stack-arg-base-offset]
  (if (= param-count 60)
    (do
      (spill-native-function-params-aarch64-twenty-to-sixty 59 result stack-arg-base-offset)
      (append-native-bytes-loop result (emit-aarch64-ldr-x10-sp (+ stack-arg-base-offset 408)) 0 4)
      (append-native-bytes-loop result (emit-aarch64-str-x10-sp (local-slot-offset 59)) 0 4))
    (if (= param-count 59)
      (do
        (spill-native-function-params-aarch64-twenty-to-fifty-eight 58 result stack-arg-base-offset)
        (append-native-bytes-loop result (emit-aarch64-ldr-x10-sp (+ stack-arg-base-offset 400)) 0 4)
        (append-native-bytes-loop result (emit-aarch64-str-x10-sp (local-slot-offset 58)) 0 4))
      (spill-native-function-params-aarch64-twenty-to-fifty-eight param-count result stack-arg-base-offset))))

(defn spill-native-function-stack-params-aarch64-loop [param-index param-count stack-offset result stack-arg-base-offset]
  (if (>= param-index param-count)
    0
    (do
      (append-native-bytes-loop result (emit-aarch64-ldr-x10-sp (+ stack-arg-base-offset stack-offset)) 0 4)
      (append-native-bytes-loop result (emit-aarch64-str-x10-sp (local-slot-offset param-index)) 0 4)
      (spill-native-function-stack-params-aarch64-loop (+ param-index 1) param-count (+ stack-offset 8) result stack-arg-base-offset))))

(defn spill-native-function-params-aarch64-twenty-plus [param-count result stack-arg-base-offset]
  (do
    (append-native-bytes-loop result (emit-aarch64-str-x0-sp (local-slot-offset 0)) 0 4)
    (append-native-bytes-loop result (emit-aarch64-str-x1-sp (local-slot-offset 1)) 0 4)
    (append-native-bytes-loop result (emit-aarch64-str-x2-sp (local-slot-offset 2)) 0 4)
    (append-native-bytes-loop result (emit-aarch64-str-x3-sp (local-slot-offset 3)) 0 4)
    (append-native-bytes-loop result (emit-aarch64-str-x4-sp (local-slot-offset 4)) 0 4)
    (append-native-bytes-loop result (emit-aarch64-str-x5-sp (local-slot-offset 5)) 0 4)
    (append-native-bytes-loop result (emit-aarch64-str-x6-sp (local-slot-offset 6)) 0 4)
    (append-native-bytes-loop result (emit-aarch64-str-x7-sp (local-slot-offset 7)) 0 4)
    (spill-native-function-stack-params-aarch64-loop 8 param-count 0 result stack-arg-base-offset)))

(defn spill-native-function-params-aarch64-twenty-to-sixty-one [param-count result stack-arg-base-offset]
  (if (> param-count 60)
    (spill-native-function-params-aarch64-twenty-plus param-count result stack-arg-base-offset)
    (spill-native-function-params-aarch64-twenty-to-sixty param-count result stack-arg-base-offset)))

(defn zero-native-local-only-slots-aarch64-step [param-count local-count result idx]
  (if (>= idx local-count)
    (make-native-progress-state 1 idx)
    (do
      (append-native-bytes-rooted result (emit-aarch64-movz-w0 0) 4)
      (append-native-bytes-rooted result (emit-aarch64-str-x0-sp (local-slot-offset (+ param-count idx))) 4)
      (make-native-progress-state 0 (+ idx 1)))))

(defn zero-native-local-only-slots-aarch64-step-64-loop-bounded [param-count local-count result idx remaining]
  (do
    (root_push result)
    (let [state (zero-native-local-only-slots-aarch64-step param-count local-count result idx)]
      (do
        (root_push state)
        (let [final
              (if (= (vector-get state 0) 1)
                state
                (if (<= remaining 1)
                  state
                  (zero-native-local-only-slots-aarch64-step-64-loop-bounded param-count local-count result (vector-get state 1) (- remaining 1))))]
          (do
            (root_pop)
            (root_pop)
            final))))))

(defn zero-native-local-only-slots-aarch64-step-64 [param-count local-count result idx]
  (zero-native-local-only-slots-aarch64-step-64-loop-bounded param-count local-count result idx 64))

(defn continue-zero-native-local-only-slots-aarch64-step-64 [param-count local-count result state]
  (if (= (vector-get state 0) 1)
    state
    (do
      (root_push result)
      (root_push state)
      (let [next-state (zero-native-local-only-slots-aarch64-step-64 param-count local-count result (vector-get state 1))]
        (do
          (root_push next-state)
          (let [final (continue-zero-native-local-only-slots-aarch64-step-64 param-count local-count result next-state)]
            (do
              (root_pop)
              (root_pop)
              (root_pop)
              final)))))))

(defn zero-native-local-only-slots-aarch64-loop [param-count local-count result idx]
  (continue-zero-native-local-only-slots-aarch64-step-64
    param-count
    local-count
    result
    (zero-native-local-only-slots-aarch64-step-64 param-count local-count result idx)))

(defn generate-native-function-aarch64-bundle-with-import-count [func-meta result function-starts function-metas import-count import-stub-offset function-start]
  (let [param-count (native-function-param-count func-meta)
    local-count (native-function-local-count func-meta)
    ir-func (native-function-ir func-meta)]
    (do
      (root_push func-meta)
      (root_push result)
      (root_push function-starts)
      (root_push function-metas)
      (root_push ir-func)
        (let [frame-base-slot-count (native-frame-base-slot-count ir-func (+ param-count local-count))
          stack-bytes (native-local-stack-bytes-with-window ir-func (+ param-count local-count) function-metas)
          has-call (native-has-call ir-func)
          min-slot-count (+ param-count local-count)
          prologue-stack-bytes (if (if (= (aarch64-bundle-stack-padding-needed ir-func min-slot-count function-metas) 1) (> stack-bytes 0) false)
                                 (align-16 (+ stack-bytes 8))
                                 stack-bytes)
          stack-arg-base-offset (+ prologue-stack-bytes (if (= has-call 1) 16 0))
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
        body-offset (+ (+ after-stack-offset param-spill-bytes) (* local-count 8))
        n (vector-length ir-func)]
        (let [final
          (do
            (if (= has-call 1)
              (append-native-bytes-loop result (emit-aarch64-save-fp-lr) 0 4)
              0)
            (if (> prologue-stack-bytes 0)
              (append-native-bytes-loop result (emit-aarch64-sub-sp prologue-stack-bytes) 0 4)
              0)
           (if (>= param-count 20)
             (spill-native-function-params-aarch64-twenty-to-sixty-one param-count result stack-arg-base-offset)
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
            (if (> local-count 0)
             (zero-native-local-only-slots-aarch64-loop param-count local-count result 0)
              0)
            (let [control-meta (scan-control-flow-meta ir-func)
              offsets (collect-native-bundle-offsets-aarch64 ir-func function-metas body-offset)]
              (do
                (root_push control-meta)
                (root_push offsets)
                (let [generated
                  (generate-native-control-instr-bundle-loop-aarch64-with-import-count
                    ir-func
                    result
                    control-meta
                    offsets
                    function-starts
                    function-metas
                    import-count
                    import-stub-offset
                    frame-base-slot-count
                     0
                     0
                     n)]
                   (do
                     (root_pop)
                     (root_pop)
                     generated))))
            (if (> prologue-stack-bytes 0)
              (append-native-bytes-loop result (emit-aarch64-add-sp prologue-stack-bytes) 0 4)
              0)
           (if (= has-call 1)
             (append-native-bytes-loop result (emit-aarch64-restore-fp-lr) 0 4)
             0)
           (append-native-bytes-loop result (emit-aarch64-ret) 0 4))]
          (do
            (root_pop)
            (root_pop)
            (root_pop)
            (root_pop)
            (root_pop)
            final))))))

(defn generate-native-function-aarch64-bundle [func-meta result function-starts function-metas function-start]
  (generate-native-function-aarch64-bundle-with-import-count func-meta result function-starts function-metas 0 0 function-start))

(defn make-native-bundle-loop-state [done next-idx]
  (do
    (root_push done)
    (root_push next-idx)
    (let [base (vector-new 2)]
      (do
        (let [base-slot (root_push base)
          with-done (vector-push base done)]
          (do
            (root_set base-slot with-done)
            (let [result (vector-push with-done next-idx)]
              (do
                (root_pop)
                (root_pop)
                (root_pop)
                result))))))))

(defn generate-native-aarch64-bundle-loop-with-import-count-step [functions result function-starts import-count import-stub-offset idx len]
  (if (>= idx len)
    (make-native-bundle-loop-state 1 idx)
    (do
      (root_push functions)
      (root_push result)
      (root_push function-starts)
      (let [actual-idx (+ idx import-count)
        func-meta (vector-get functions actual-idx)
        function-start (vector-get function-starts idx)]
        (do
          (root_push func-meta)
          (generate-native-function-aarch64-bundle-with-import-count func-meta result function-starts functions import-count import-stub-offset function-start)
          (root_pop)
          (root_pop)
          (root_pop)
          (root_pop)
          (make-native-bundle-loop-state 0 (+ idx 1)))))))

(defn generate-native-aarch64-bundle-loop-with-import-count-step-64-loop-bounded [functions result function-starts import-count import-stub-offset idx len remaining]
  (do
    (root_push functions)
    (root_push result)
    (root_push function-starts)
    (let [state (generate-native-aarch64-bundle-loop-with-import-count-step functions result function-starts import-count import-stub-offset idx len)
      done (vector-get state 0)
      next-idx (vector-get state 1)]
      (do
        (root_push state)
        (let [final
          (if (= done 1)
            state
            (if (<= remaining 1)
              state
              (generate-native-aarch64-bundle-loop-with-import-count-step-64-loop-bounded functions result function-starts import-count import-stub-offset next-idx len (- remaining 1))))]
          (do
            (root_pop)
            (root_pop)
            (root_pop)
            (root_pop)
            final))))))

(defn generate-native-aarch64-bundle-loop-with-import-count-step-64 [functions result function-starts import-count import-stub-offset idx len]
  (generate-native-aarch64-bundle-loop-with-import-count-step-64-loop-bounded functions result function-starts import-count import-stub-offset idx len 64))

(defn continue-generate-native-aarch64-bundle-loop-with-import-count-step-64 [functions result function-starts import-count import-stub-offset len state]
  (if (= (vector-get state 0) 1)
    state
    (do
      (root_push functions)
      (root_push result)
      (root_push function-starts)
      (root_push state)
      (let [next-state (generate-native-aarch64-bundle-loop-with-import-count-step-64 functions result function-starts import-count import-stub-offset (vector-get state 1) len)]
        (do
          (root_push next-state)
          (let [final (continue-generate-native-aarch64-bundle-loop-with-import-count-step-64 functions result function-starts import-count import-stub-offset len next-state)]
            (do
              (root_pop)
              (root_pop)
              (root_pop)
              (root_pop)
              (root_pop)
              final)))))))

(defn generate-native-aarch64-bundle-loop-with-import-count [functions result function-starts import-count import-stub-offset idx len]
  (continue-generate-native-aarch64-bundle-loop-with-import-count-step-64
    functions
    result
    function-starts
    import-count
    import-stub-offset
    len
    (generate-native-aarch64-bundle-loop-with-import-count-step-64 functions result function-starts import-count import-stub-offset idx len)))

(defn generate-native-aarch64-bundle-with-layout [functions import-count function-starts import-stub-offset]
  (do
    (root_push functions)
    (root_push function-starts)
    (let [result (ref-new (vector-new (aarch64-bundle-initial-capacity import-stub-offset import-count)))
      import-stub-count (aarch64-import-stub-count import-count)
      n (- (vector-length functions) import-count)]
      (do
        (root_push result)
        (let [final
          (do
            (generate-native-aarch64-bundle-loop-with-import-count functions result function-starts import-count import-stub-offset 0 n)
            (append-aarch64-import-stubs-loop result import-count import-stub-offset 0 import-stub-count)
            (append-native-bytes-rooted result (emit-aarch64-selfhost-command-line-arg-helper) 32)
            (append-native-bytes-rooted result (emit-aarch64-selfhost-string-length-helper) 60)
            (append-native-bytes-rooted result (emit-aarch64-selfhost-print-helper) 144)
            (append-native-bytes-rooted result (emit-aarch64-selfhost-vector-new-helper) 108)
            (append-native-bytes-rooted result (emit-aarch64-selfhost-vector-length-helper) 20)
            (append-native-bytes-rooted result (emit-aarch64-selfhost-alloc-helper) 72)
            (append-native-bytes-rooted result (emit-aarch64-selfhost-string-char-at-helper) 52)
            (append-native-bytes-rooted result (emit-aarch64-selfhost-vector-get-helper) 52)
            (append-native-bytes-rooted result (emit-aarch64-selfhost-vector-push-helper) 256)
            (append-native-bytes-rooted result (emit-aarch64-selfhost-ref-new-helper) 96)
            (append-native-bytes-rooted result (emit-aarch64-selfhost-ref-get-helper) 20)
            (append-native-bytes-rooted result (emit-aarch64-selfhost-ref-set-helper) 24)
            (append-native-bytes-rooted result (emit-aarch64-selfhost-substring-helper) 208)
            (append-aarch64-selfhost-string-concat-helper-rooted result)
            (append-native-bytes-rooted result (emit-aarch64-selfhost-map-size-helper) 20)
            (append-native-bytes-rooted result (emit-aarch64-selfhost-map-new-helper) 100)
            (append-native-bytes-rooted result (emit-aarch64-selfhost-file-exists-helper) 196)
            (append-aarch64-selfhost-read-file-helper-rooted result)
            (append-native-bytes-rooted result (emit-aarch64-selfhost-map-insert-helper) 96)
            (append-native-bytes-rooted result (emit-aarch64-selfhost-map-get-helper) 60)
            (append-native-bytes-rooted result (emit-aarch64-selfhost-map-new-fixed-helper) 92)
            (ref-get result))]
          (do
            (root_pop)
            (root_pop)
            (root_pop)
            final))))))

(defn generate-native-aarch64-bundle-with-import-count [functions import-count]
  (do
    (root_push functions)
    (let [layout (collect-callable-actual-layout-aarch64 functions import-count)]
      (do
        (root_push layout)
        (let [function-starts (callable-layout-function-starts-aarch64 layout)]
          (do
            (root_push function-starts)
            (let [final (generate-native-aarch64-bundle-with-layout
                          functions
                          import-count
                          function-starts
                          (callable-layout-import-stub-offset-aarch64 layout))]
              (do
                (root_pop)
                (root_pop)
                (root_pop)
                final))))))))

(defn generate-native-aarch64-bundle [functions]
  (generate-native-aarch64-bundle-with-import-count functions 0))

;; IR 関数をネイティブコードに変換
;; ir-func: IR 命令列の Vector [[opcode, operand], ...]
;; target: ターゲット記述子
;; 戻り値: ネイティブ機械語バイト列
(defn generate-native [ir-func target]
  (do
    (root_push ir-func)
    (root_push target)
    (let [arch (target-arch target)
      result (if (= arch 2)
               ;; aarch64 → AArch64 命令列
               (generate-native-aarch64 ir-func)
               ;; x86_64 (arch=1) またはデフォルト
               (generate-native-x86-64 ir-func))]
      (do
        (root_pop)
        (root_pop)
        result))))

(defn wrap-ir-functions-as-meta-loop [functions idx len result]
  (if (>= idx len)
    result
    (let [ir-func (vector-get functions idx)
      next-result (vector-push result (make-native-function-meta 0 0 ir-func))]
      (wrap-ir-functions-as-meta-loop functions (+ idx 1) len next-result))))

(defn wrap-ir-functions-as-meta [functions]
  (wrap-ir-functions-as-meta-loop functions 0 (vector-length functions) (vector-new 8)))

(defn normalize-selfhost-native-local-instr [instr]
  (let [opcode (vector-get instr 0)
    operand (vector-get instr 1)]
    (if (= opcode 10)
      (vector-push (vector-push (vector-new 2) opcode) (- operand 1))
      (if (= opcode 11)
        (vector-push (vector-push (vector-new 2) opcode) (- operand 1))
        instr))))

(defn normalize-selfhost-native-ir-step [ir idx len result]
  (if (>= idx len)
    (vector-push (vector-push (vector-push (vector-new 3) 1) idx) result)
    (do
      (root_push ir)
      (root_push result)
      (let [next-instr (normalize-selfhost-native-local-instr (vector-get ir idx))]
        (do
          (root_push next-instr)
          (let [next-result (vector-push result next-instr)]
            (do
              (root_push next-result)
              (let [state (vector-push (vector-push (vector-push (vector-new 3) 0) (+ idx 1)) next-result)]
                (do
                  (root_pop)
                  (root_pop)
                  (root_pop)
                  (root_pop)
                  state)))))))))

(defn normalize-selfhost-native-ir-step-64-loop-bounded [ir idx len result remaining]
  (do
    (root_push ir)
    (root_push result)
    (let [state (normalize-selfhost-native-ir-step ir idx len result)
      done (vector-get state 0)
      next-idx (vector-get state 1)
      next-result (vector-get state 2)]
      (do
        (root_push state)
        (root_push next-result)
        (let [final
          (if (= done 1)
            state
            (if (<= remaining 1)
              state
              (normalize-selfhost-native-ir-step-64-loop-bounded ir next-idx len next-result (- remaining 1))))]
          (do
            (root_pop)
            (root_pop)
            (root_pop)
            (root_pop)
            final))))))

(defn normalize-selfhost-native-ir-step-64 [ir idx len result]
  (normalize-selfhost-native-ir-step-64-loop-bounded ir idx len result 64))

(defn continue-normalize-selfhost-native-ir-step-64 [ir len state]
  (if (= (vector-get state 0) 1)
    state
    (do
      (root_push ir)
      (root_push state)
      (let [next-state (normalize-selfhost-native-ir-step-64 ir (vector-get state 1) len (vector-get state 2))]
        (do
          (root_push next-state)
          (let [result (continue-normalize-selfhost-native-ir-step-64 ir len next-state)]
            (do
              (root_pop)
              (root_pop)
              (root_pop)
              result)))))))

(defn normalize-selfhost-native-ir [ir]
  (vector-get
    (continue-normalize-selfhost-native-ir-step-64
      ir
      (vector-length ir)
      (normalize-selfhost-native-ir-step-64 ir 0 (vector-length ir) (vector-new 8)))
    2))

(defn normalize-selfhost-native-function-meta [func-meta]
  (do
    (root_push func-meta)
    (let [normalized-ir (normalize-selfhost-native-ir (native-function-ir func-meta))]
      (do
        (root_push normalized-ir)
        (let [result (make-native-function-meta
                       (native-function-param-count func-meta)
                       (native-function-local-count func-meta)
                       normalized-ir)]
          (do
            (root_pop)
            (root_pop)
            result))))))

(defn normalize-selfhost-native-function-metas-step [functions idx len result]
  (if (>= idx len)
    (vector-push (vector-push (vector-push (vector-new 3) 1) idx) result)
    (do
      (root_push functions)
      (root_push result)
      (let [next-func-meta (normalize-selfhost-native-function-meta (vector-get functions idx))]
        (do
          (root_push next-func-meta)
          (let [next-result (vector-push result next-func-meta)]
            (do
              (root_push next-result)
              (let [state (vector-push (vector-push (vector-push (vector-new 3) 0) (+ idx 1)) next-result)]
                (do
                  (root_pop)
                  (root_pop)
                  (root_pop)
                  (root_pop)
                  state)))))))))

(defn normalize-selfhost-native-function-metas-step-64-loop-bounded [functions idx len result remaining]
  (do
    (root_push functions)
    (root_push result)
    (let [state (normalize-selfhost-native-function-metas-step functions idx len result)
      done (vector-get state 0)
      next-idx (vector-get state 1)
      next-result (vector-get state 2)]
      (do
        (root_push state)
        (root_push next-result)
        (let [final
          (if (= done 1)
            state
            (if (<= remaining 1)
              state
              (normalize-selfhost-native-function-metas-step-64-loop-bounded functions next-idx len next-result (- remaining 1))))]
          (do
            (root_pop)
            (root_pop)
            (root_pop)
            (root_pop)
            final))))))

(defn normalize-selfhost-native-function-metas-step-64 [functions idx len result]
  (normalize-selfhost-native-function-metas-step-64-loop-bounded functions idx len result 64))

(defn continue-normalize-selfhost-native-function-metas-step-64 [functions len state]
  (if (= (vector-get state 0) 1)
    state
    (do
      (root_push functions)
      (root_push state)
      (let [next-state (normalize-selfhost-native-function-metas-step-64 functions (vector-get state 1) len (vector-get state 2))]
        (do
          (root_push next-state)
          (let [result (continue-normalize-selfhost-native-function-metas-step-64 functions len next-state)]
            (do
              (root_pop)
              (root_pop)
              (root_pop)
              result)))))))

(defn normalize-selfhost-native-function-metas [functions]
  (vector-get
    (continue-normalize-selfhost-native-function-metas-step-64
      functions
      (vector-length functions)
      (normalize-selfhost-native-function-metas-step-64 functions 0 (vector-length functions) (vector-new 8)))
    2))

(defn normalize-selfhost-native-function-metas-for-target [functions target]
  (if (= (target-arch target) 1)
    functions
    (normalize-selfhost-native-function-metas functions)))

(defn generate-native-function-meta-bundle [functions target]
  (let [arch (target-arch target)]
    (if (= arch 2)
      (generate-native-aarch64-bundle functions)
      (generate-native-x86-64-bundle functions))))

(defn make-native-bundle-entrypoint-payload [bundle entrypoint-offset]
  (do
    (root_push bundle)
    (let [payload1 (vector-push (vector-new 2) bundle)]
      (do
        (root_push payload1)
        (let [payload2 (vector-push payload1 entrypoint-offset)]
          (do
            (root_pop)
            (root_pop)
            payload2))))))

(defn native-bundle-entrypoint-offset-for-function-with-import-count [function-starts import-count entrypoint-func-idx]
  (let [callable-idx (- entrypoint-func-idx import-count)
    len (vector-length function-starts)]
    (if (>= callable-idx 0)
      (if (< callable-idx len)
        (vector-get function-starts callable-idx)
        0)
      0)))

(defn native-last-callable-function-idx-with-import-count [functions]
  (let [len (vector-length functions)]
    (if (> len 0)
      (- len 1)
      0)))

(defn generate-native-x86-64-bundle-entrypoint-payload-for-function-with-import-count [functions import-count entrypoint-func-idx]
  (let [function-starts (collect-callable-function-starts-x86 functions import-count)
    bundle (generate-native-x86-64-bundle-with-import-count functions import-count)
    entrypoint-offset (native-bundle-entrypoint-offset-for-function-with-import-count function-starts import-count entrypoint-func-idx)]
    (make-native-bundle-entrypoint-payload bundle entrypoint-offset)))

(defn generate-native-x86-64-bundle-entrypoint-payload-with-import-count [functions import-count]
  (generate-native-x86-64-bundle-entrypoint-payload-for-function-with-import-count
    functions
    import-count
    (native-last-callable-function-idx-with-import-count functions)))

(defn generate-native-aarch64-bundle-entrypoint-payload-for-function-with-import-count [functions import-count entrypoint-func-idx]
  (do
    (root_push functions)
    (let [stable-functions (concat-byte-vectors-loop (vector-new (vector-length functions)) functions 0 (vector-length functions))]
      (do
        (root_push stable-functions)
        (let [layout (collect-callable-actual-layout-aarch64 stable-functions import-count)
          function-starts (callable-layout-function-starts-aarch64 layout)
          import-stub-offset (callable-layout-import-stub-offset-aarch64 layout)
          bundle (generate-native-aarch64-bundle-with-layout
                   stable-functions
                   import-count
                   function-starts
                   import-stub-offset)
          static-entrypoint-offset (native-bundle-entrypoint-offset-for-function-with-import-count function-starts import-count entrypoint-func-idx)
          last-func-idx (native-last-callable-function-idx-with-import-count stable-functions)
          entrypoint-length (if (= entrypoint-func-idx last-func-idx)
                              (measure-native-function-aarch64-bundle-with-import-count
                                (vector-get stable-functions entrypoint-func-idx)
                                function-starts
                                stable-functions
                                import-count
                                import-stub-offset
                                static-entrypoint-offset)
                              0)
          trailer-length (aarch64-selfhost-helper-trailer-size import-count)
          actual-user-total (- (vector-length bundle) trailer-length)
          entrypoint-offset (if (= entrypoint-func-idx last-func-idx)
                              (- actual-user-total entrypoint-length)
                              static-entrypoint-offset)]
          (do
            (root_push bundle)
            (let [payload (make-native-bundle-entrypoint-payload bundle entrypoint-offset)]
              (do
                (root_pop)
                (root_pop)
                (root_pop)
                payload))))))))

(defn generate-native-aarch64-bundle-entrypoint-payload-with-import-count [functions import-count]
  (generate-native-aarch64-bundle-entrypoint-payload-for-function-with-import-count
    functions
    import-count
    (native-last-callable-function-idx-with-import-count functions)))

(defn generate-native-function-meta-bundle-with-import-count [functions import-count target]
  (let [arch (target-arch target)]
    (if (= arch 2)
      (do
        (root_push functions)
        (let [stable-functions (concat-byte-vectors-loop (vector-new (vector-length functions)) functions 0 (vector-length functions))]
          (do
            (root_push stable-functions)
            (let [result (generate-native-aarch64-bundle-with-import-count stable-functions import-count)]
              (do
                (root_pop)
                (root_pop)
                result)))))
      (generate-native-x86-64-bundle-with-import-count functions import-count))))

(defn generate-native-function-meta-bundle-entrypoint-payload-with-import-count [functions import-count target]
  (let [arch (target-arch target)]
    (if (= arch 2)
      (generate-native-aarch64-bundle-entrypoint-payload-with-import-count functions import-count)
      (generate-native-x86-64-bundle-entrypoint-payload-with-import-count functions import-count))))

(defn generate-native-function-meta-bundle-entrypoint-payload-for-function-with-import-count [functions import-count entrypoint-func-idx target]
  (let [arch (target-arch target)]
    (if (= arch 2)
      (generate-native-aarch64-bundle-entrypoint-payload-for-function-with-import-count functions import-count entrypoint-func-idx)
      (generate-native-x86-64-bundle-entrypoint-payload-for-function-with-import-count functions import-count entrypoint-func-idx))))

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

(defn emit-native-function-meta-bundle-with-import-count [functions import-count target]
  (generate-native-function-meta-bundle-with-import-count functions import-count target))

(defn emit-native-function-meta-bundle-entrypoint-payload-with-import-count [functions import-count target]
  (generate-native-function-meta-bundle-entrypoint-payload-with-import-count functions import-count target))

(defn emit-native-function-meta-bundle-entrypoint-payload-for-function-with-import-count [functions import-count entrypoint-func-idx target]
  (generate-native-function-meta-bundle-entrypoint-payload-for-function-with-import-count functions import-count entrypoint-func-idx target))

(defn emit-native-selfhost-function-meta-bundle-with-import-count [functions import-count target]
  (emit-native-function-meta-bundle-with-import-count
    (normalize-selfhost-native-function-metas-for-target functions target)
    import-count
    target))

(defn emit-native-selfhost-function-meta-bundle-entrypoint-payload-with-import-count [functions import-count target]
  (emit-native-function-meta-bundle-entrypoint-payload-with-import-count
    (normalize-selfhost-native-function-metas-for-target functions target)
    import-count
    target))

(defn emit-native-selfhost-function-meta-bundle-entrypoint-payload-for-function-with-import-count [functions import-count entrypoint-func-idx target]
  (emit-native-function-meta-bundle-entrypoint-payload-for-function-with-import-count
    (normalize-selfhost-native-function-metas-for-target functions target)
    import-count
    entrypoint-func-idx
    target))

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
