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

;; x86_64 の MOV rbp, rsp
(defn emit-mov-rbp-rsp []
  (let [bytes (vector-new 3)]
    (vector-push (vector-push (vector-push bytes
          72) ;; 0x48 REX.W
        137) ;; 0x89
      229))) ;; 0xE5 (rsp -> rbp)

;; 32bit 値を little-endian 4 bytes に分解する
(defn encode-u32-le [value]
  (let [byte0 (% value 256)
    byte1 (% (/ value 256) 256)
    byte2 (% (/ value 65536) 256)
    byte3 (% (/ value 16777216) 256)
    bytes (vector-new 4)]
    (vector-push (vector-push (vector-push (vector-push bytes byte0) byte1) byte2) byte3)))

;; ローカル変数の stack slot offset (rbp/sp からの byte 数)
(defn local-slot-offset [idx]
  (* (+ idx 1) 8))

;; 16 byte alignment を満たす stack size に丸める
(defn align-16 [value]
  (let [remainder (% value 16)]
    (if (= remainder 0)
      value
      (+ value (- 16 remainder)))))

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

(defn native-local-stack-bytes [ir-func]
  (let [state (find-max-local-index-loop ir-func 0 (vector-length ir-func) (make-local-scan-state 0 0))
    found (local-scan-found state)
    max-local (local-scan-max state)]
    (if (= found 0)
      0
      (align-16 (* (+ max-local 1) 8)))))

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

;; === IR -> ネイティブ変換 ===

;; IR opcode をネイティブ命令列に変換 (x86_64)
;; 戻り値: バイト列 Vector
(defn codegen-ir-instr [opcode operand]
  (if (= opcode 1)
    ;; i64.const -> mov rax, imm64
    (emit-mov-imm64 (reg-rax) operand)
    (if (= opcode 10)
      ;; local.get -> mov rax, [rbp-offset]
      (emit-mov-rax-from-local (local-slot-offset operand))
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
        ;; 未知の opcode: NOP
        (vector-push (vector-new 1) 144))))))) ;; 0x90

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

;; AArch64 NOP 命令
;; エンコーディング: 0xD503201F → [0x1F, 0x20, 0x03, 0xD5]
(defn emit-aarch64-nop []
  (let [bytes (vector-new 4)]
    (vector-push (vector-push (vector-push (vector-push bytes 31) 32) 3) 213)))

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

;; AArch64 LDR x0, [sp, #offset]
(defn emit-aarch64-ldr-x0-sp [offset]
  (let [scaled (/ offset 8)]
    (encode-u32-le (+ (+ 4181721088 (* scaled 1024)) 992))))

;; IR opcode を AArch64 命令列に変換
(defn codegen-ir-instr-aarch64 [opcode operand]
  (if (= opcode 1)
    ;; i64.const -> MOVZ W0, #operand
    (emit-aarch64-movz-w0 operand)
    (if (= opcode 10)
      ;; local.get -> LDR x0, [sp, #offset]
      (emit-aarch64-ldr-x0-sp (local-slot-offset operand))
      (if (= opcode 11)
        ;; local.set -> STR x0, [sp, #offset]
        (emit-aarch64-str-x0-sp (local-slot-offset operand))
    ;; 未知の opcode: NOP
    (emit-aarch64-nop)))))

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
    n (vector-length ir-func)]
    (do
      (if (> stack-bytes 0)
        (append-native-bytes-loop result (emit-aarch64-sub-sp stack-bytes) 0 4)
        0)
      (generate-native-instr-loop-aarch64 ir-func result 0 n)
      (let [ret-bytes (emit-aarch64-ret)]
        (do
          (if (> stack-bytes 0)
            (append-native-bytes-loop result (emit-aarch64-add-sp stack-bytes) 0 4)
            0)
          (append-native-bytes-loop result ret-bytes 0 4)
          (ref-get result))))))

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

;; ネイティブコード生成のトップレベル関数
;; source-ir: プログラム全体の IR
;; target: ターゲット記述子
;; 戻り値: ネイティブ機械語バイト列
(defn emit-native [source-ir target]
  (generate-native source-ir target))

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
