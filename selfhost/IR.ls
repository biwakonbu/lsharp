;; IR.ls - L# セルフホスティング: IR 定義
;;
;; AST から変換される中間表現。
;; スタックマシンの命令列で表現。

;; === IR 命令種別 ===

;; 定数
(defn ir-i64-const [] 1)
(defn ir-f64-const [] 2)

;; ローカル変数
(defn ir-local-get [] 10)
(defn ir-local-set [] 11)

;; 算術演算
(defn ir-i64-add [] 20)
(defn ir-i64-sub [] 21)
(defn ir-i64-mul [] 22)
(defn ir-i64-div [] 23)

;; 比較
(defn ir-i64-eq [] 30)
(defn ir-i64-ne [] 31)
(defn ir-i64-lt [] 32)
(defn ir-i64-gt [] 33)
(defn ir-i64-le [] 34)
(defn ir-i64-ge [] 35)

;; 制御フロー
(defn ir-call [] 40)
(defn ir-if [] 41)
(defn ir-block [] 42)
(defn ir-end [] 43)

;; === 命令構築 ===

;; 命令は [opcode, operand] の Vector
(defn make-instr [opcode operand]
  (vector-push (vector-push (vector-new 2) opcode) operand))

;; 定数ロード
(defn make-i64-const [value]
  (make-instr 1 value))

;; ローカル変数取得
(defn make-local-get [idx]
  (make-instr 10 idx))

;; 関数呼び出し
(defn make-call [func-idx]
  (make-instr 40 func-idx))

;; エントリポイント (テスト用)
(defn main []
  (let [c (make-i64-const 42)
        g (make-local-get 0)]
    (do
      (print (vector-get c 0))  ;; 1 (i64.const)
      (print (vector-get c 1))  ;; 42
      (print (vector-get g 0))  ;; 10 (local.get)
      (print (vector-get g 1))  ;; 0
      0)))
