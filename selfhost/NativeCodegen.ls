(module NativeCodegen)
(import NativeTarget)
(import IR)

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
        b1 (vector-push bytes 72)       ;; 0x48 (REX.W)
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
  (vector-push (vector-new 1) 195))  ;; 0xC3

;; x86_64 の PUSH rbp
(defn emit-push-rbp []
  (vector-push (vector-new 1) 85))   ;; 0x55

;; x86_64 の POP rbp
(defn emit-pop-rbp []
  (vector-push (vector-new 1) 93))   ;; 0x5D

;; x86_64 の MOV rbp, rsp
(defn emit-mov-rbp-rsp []
  (let [bytes (vector-new 3)]
    (vector-push (vector-push (vector-push bytes
      72)    ;; 0x48 REX.W
      137)   ;; 0x89
      229))) ;; 0xE5 (rsp -> rbp)

;; === IR -> ネイティブ変換 ===

;; IR opcode をネイティブ命令列に変換 (x86_64)
;; 戻り値: バイト列 Vector
(defn codegen-ir-instr [opcode operand]
  (if (= opcode 1)
    ;; i64.const -> mov rax, imm64
    (emit-mov-imm64 (reg-rax) operand)
    (if (= opcode 20)
      ;; i64.add -> add rax, rcx (簡易版)
      ;; 0x48 0x01 0xC8
      (vector-push (vector-push (vector-push (vector-new 3) 72) 1) 200)
      (if (= opcode 21)
        ;; i64.sub -> sub rax, rcx
        ;; 0x48 0x29 0xC8
        (vector-push (vector-push (vector-push (vector-new 3) 72) 41) 200)
        ;; 未知の opcode: NOP
        (vector-push (vector-new 1) 144)))))  ;; 0x90

;; === コード生成メイン関数 ===

;; IR 関数をネイティブコードに変換
;; ir-func: IR 命令列の Vector [[opcode, operand], ...]
;; target: ターゲット記述子
;; 戻り値: ネイティブ機械語バイト列
(defn generate-native [ir-func target]
  (let [result (ref-new (vector-new 64))
        ;; 関数プロローグ
        prologue-push (emit-push-rbp)
        prologue-mov (emit-mov-rbp-rsp)
        _ (ref-set result (vector-push (ref-get result) (vector-get prologue-push 0)))
        _ (ref-set result (vector-push (ref-get result) (vector-get prologue-mov 0)))
        _ (ref-set result (vector-push (ref-get result) (vector-get prologue-mov 1)))
        _ (ref-set result (vector-push (ref-get result) (vector-get prologue-mov 2)))
        ;; IR 命令を変換 (最大4命令)
        n (vector-length ir-func)
        i (ref-new 0)]
    (do
      (if (< (ref-get i) n)
        (let [instr (vector-get ir-func (ref-get i))
              opcode (vector-get instr 0)
              operand (vector-get instr 1)
              native (codegen-ir-instr opcode operand)
              native-len (vector-length native)
              j (ref-new 0)]
          (do
            (if (< (ref-get j) native-len)
              (do
                (ref-set result (vector-push (ref-get result) (vector-get native (ref-get j))))
                (ref-set j (+ (ref-get j) 1))
                (if (< (ref-get j) native-len)
                  (do
                    (ref-set result (vector-push (ref-get result) (vector-get native (ref-get j))))
                    (ref-set j (+ (ref-get j) 1))
                    (if (< (ref-get j) native-len)
                      (do
                        (ref-set result (vector-push (ref-get result) (vector-get native (ref-get j))))
                        0)
                      0))
                  0))
              0)
            (ref-set i (+ (ref-get i) 1))
            0))
        0)
      ;; 関数エピローグ
      (let [epilogue-pop (emit-pop-rbp)
            epilogue-ret (emit-ret)]
        (do
          (ref-set result (vector-push (ref-get result) (vector-get epilogue-pop 0)))
          (ref-set result (vector-push (ref-get result) (vector-get epilogue-ret 0)))
          (ref-get result))))))

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
        target (make-target 2)  ;; aarch64-apple-darwin
        native-code (emit-native ir target)]
    (do
      (print (vector-length native-code))  ;; ネイティブコードのバイト数
      0)))
