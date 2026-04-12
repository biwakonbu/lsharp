(module IR.IR)

;; IR.ls - L# セルフホスティング: IR 定義
;;
;; AST から変換される中間表現。
;; スタックマシンの命令列で表現。

;; === IR 命令種別 ===

;; 定数
(defn ir-i64-const [] 1)
(defn ir-f64-const [] 2)
(defn ir-i32-const [] 3)

;; ローカル変数
(defn ir-local-get [] 10)
(defn ir-local-set [] 11)

;; 算術演算
(defn ir-i64-add [] 20)
(defn ir-i64-sub [] 21)
(defn ir-i64-mul [] 22)
(defn ir-i64-div [] 23)
(defn ir-i32-add [] 24)
(defn ir-i32-mul [] 25)

;; 比較
(defn ir-i64-eq [] 30)
(defn ir-i64-ne [] 31)
(defn ir-i64-lt [] 32)
(defn ir-i64-gt [] 33)
(defn ir-i64-le [] 34)
(defn ir-i64-ge [] 35)
(defn ir-i64-extend-i32-s [] 36)
(defn ir-i64-extend-i32-u [] 37)
(defn ir-i32-wrap-i64 [] 38)

;; 制御フロー
(defn ir-call [] 40)
(defn ir-if [] 41)
(defn ir-block [] 42)
(defn ir-end [] 43)

;; スタック操作
(defn ir-drop [] 44)

;; 文字列 builtin
(defn ir-string-char-at [] 50)
(defn ir-string-length [] 51)

;; ベクタ builtin
(defn ir-vector-length [] 52)
(defn ir-vector-get [] 53)
(defn ir-vector-new [] 54)
(defn ir-vector-push [] 55)
(defn ir-ref-new [] 56)
(defn ir-ref-get [] 57)
(defn ir-ref-set [] 58)
(defn ir-print [] 59)
(defn ir-map-new [] 60)
(defn ir-map-size [] 61)
(defn ir-map-insert [] 62)
(defn ir-map-get [] 63)
(defn ir-read-file [] 64)
(defn ir-map-contains [] 65)
(defn ir-map-remove [] 66)
(defn ir-command-line-arg [] 67)

;; === 命令構築 ===

;; 命令は [opcode, operand] の Vector
(defn make-instr [opcode operand]
  (vector-push (vector-push (vector-new 2) opcode) operand))

;; 定数ロード
(defn make-i64-const [value]
  (make-instr 1 value))

;; 32bit 定数ロード
(defn make-i32-const [value]
  (make-instr 3 value))

;; ローカル変数取得
(defn make-local-get [idx]
  (make-instr 10 idx))

;; 32bit 加算
(defn make-i32-add []
  (make-instr 24 0))

;; 32bit 乗算
(defn make-i32-mul []
  (make-instr 25 0))

;; 64bit -> 32bit truncation
(defn make-i32-wrap-i64 []
  (make-instr 38 0))

;; 32bit -> 64bit sign extension
(defn make-i64-extend-i32-s []
  (make-instr 36 0))

;; 32bit -> 64bit zero extension
(defn make-i64-extend-i32-u []
  (make-instr 37 0))

;; 関数呼び出し
(defn make-call [func-idx]
  (make-instr 40 func-idx))

;; === Backend 3層境界型 ===

;; FrontendResult: パーサ + 型推論の出力
;; [ast, type-env, errors] の3要素 Vector
(defn make-frontend-result [ast type-env errors]
  (vector-push (vector-push (vector-push (vector-new 3) ast) type-env) errors))

;; LoweredModule: IR lowering の出力
;; [functions, globals, imports] の3要素 Vector
(defn make-lowered-module [functions globals imports]
  (vector-push (vector-push (vector-push (vector-new 3) functions) globals) imports))

;; CodegenArtifact: Wasm コード生成の出力
;; [wasm-bytes, source-map, debug-info] の3要素 Vector
(defn make-codegen-artifact [wasm-bytes source-map debug-info]
  (vector-push (vector-push (vector-push (vector-new 3) wasm-bytes) source-map) debug-info))

;; === IR snapshot シリアライザ ===

;; IR 命令列を line-based テキスト形式で出力する
;; instructions: IR 命令列 (Vector of [opcode, operand])
;; 戻り値: 各命令を1行ずつ整形した newline 区切りの行リスト (Vector of line-format 文字列ハッシュ)
;;
;; 出力フォーマット (line-format):
;;   "opcode:operand\n" を行ごとに生成
;;   例: "i64.const:42\n", "local.get:0\n"
(defn ir-to-snapshot [instructions]
  (let [n (vector-length instructions)
    lines (ref-new (vector-new n))
    i (ref-new 0)]
    (do
      ;; 各命令を行に変換
      (if (> n 0)
        (do
          (ref-set lines (vector-push (ref-get lines)
              (instr-to-line (vector-get instructions 0))))
          (if (> n 1)
            (do
              (ref-set lines (vector-push (ref-get lines)
                  (instr-to-line (vector-get instructions 1))))
              (if (> n 2)
                (do
                  (ref-set lines (vector-push (ref-get lines)
                      (instr-to-line (vector-get instructions 2))))
                  (if (> n 3)
                    (do
                      (ref-set lines (vector-push (ref-get lines)
                          (instr-to-line (vector-get instructions 3))))
                      (if (> n 4)
                        (do
                          (ref-set lines (vector-push (ref-get lines)
                              (instr-to-line (vector-get instructions 4))))
                          0)
                        0))
                    0))
                0))
            0))
        0)
      (ref-get lines))))

;; 単一命令を行形式に変換
;; instr: [opcode, operand]
;; 戻り値: opcode と operand のペア (newline 区切り出力用)
(defn instr-to-line [instr]
  (let [opcode (vector-get instr 0)
    operand (vector-get instr 1)
    ;; opcode 名を数値エンコード + operand を組み合わせた行データ
    ;; line-format: [opcode-id, operand, newline-marker]
    line (vector-new 3)]
    (vector-push (vector-push (vector-push line opcode) operand) 10))) ;; 10 = newline ASCII

;; エントリポイント (テスト用)
(defn main []
  (let [c (make-i64-const 42)
    g (make-local-get 0)]
    (do
      (print (vector-get c 0)) ;; 1 (i64.const)
      (print (vector-get c 1)) ;; 42
      (print (vector-get g 0)) ;; 10 (local.get)
      (print (vector-get g 1)) ;; 0
      0)))
