(module Parser)
(import Token)
(import AST)
(import Lexer)

;; Parser.ls - L# セルフホスティング: 再帰降下パーサー
;;
;; Lexer が出力したトークン列 (3つ組 Vector) を受け取り、AST を構築する。
;; S 式構文なので、パーサーは比較的シンプル。
;;
;; === AST ノード表現 (vector ベース) ===
;; [tag, ...data]
;; tag=1: int [1, value]
;; tag=2: bool [2, 0/1]
;; tag=3: string [3, start, end]  (ソース位置参照)
;; tag=4: var [4, name-hash]  (名前ハッシュで識別)
;; tag=5: apply [5, func-node, arg-count, arg1, arg2, ...]
;; tag=6: if [6, cond, then, else]
;; tag=7: let [7, name-hash, init, body]
;; tag=8: lambda [8, param-count, param-hash1, ..., body]
;; tag=9: do [9, expr-count, expr1, expr2, ...]
;; tag=10: match [10, scrutinee, arm-count, pat1, body1, ...]
;; tag=20: defn [20, name-hash, param-count, param-hash1, ..., body]
;; tag=21: type-decl [21, name-hash]
;; tag=25: module-decl [25, name-hash]
;; tag=26: import-decl [26, name-hash]

;; トークン種別定数 (Token.ls より)
;; 0=LParen, 1=RParen, 2=LBracket, 3=RBracket, 4=LBrace, 5=RBrace
;; 10=Int, 11=Float, 12=String, 13=BoolTrue, 14=BoolFalse, 20=Symbol
;; 30=Defn, 31=Let, 32=If, 33=Match, 34=Type, 35=Fn, 36=Do
;; 37=Module, 38=Import, 39=Record, 40=Trait, 41=Impl, 42=Where
;; 50=Colon, 51=Arrow, 52=Pipe, 53=Dot, 99=Eof

;; === 3つ組トークンアクセス ===

;; N 番目のトークンの kind
(defn span-kind [spans n]
  (vector-get spans (* n 3)))

;; N 番目のトークンの start
(defn span-start [spans n]
  (vector-get spans (+ (* n 3) 1)))

;; N 番目のトークンの end
(defn span-end [spans n]
  (vector-get spans (+ (* n 3) 2)))

;; === パーサー状態 ===

;; 現在のトークン kind を取得
(defn p-current [spans pos-ref]
  (span-kind spans (ref-get pos-ref)))

;; パーサー位置を1つ進める
(defn p-advance [pos-ref]
  (ref-set pos-ref (+ (ref-get pos-ref) 1)))

;; 現在のトークンの start を取得
(defn p-start [spans pos-ref]
  (span-start spans (ref-get pos-ref)))

;; 現在のトークンの end を取得
(defn p-end [spans pos-ref]
  (span-end spans (ref-get pos-ref)))

;; 期待するトークンを消費 (種別が一致しなければ 0 を返す)
(defn p-expect [spans pos-ref expected]
  (if (== (p-current spans pos-ref) expected)
    (do (p-advance pos-ref) 1)
    0))

;; === 名前ハッシュ ===
;; 同じ名前は異なる位置に出現しても同一キーになる
(defn name-hash-loop [src pos end acc]
  (if (>= pos end) acc
    (name-hash-loop src (+ pos 1) end
      (+ (string-char-at src pos) (* acc 31)))))

(defn name-hash [src start end]
  (name-hash-loop src start end 0))

;; === 数値パース ===

(defn parse-int-from-str [src pos end acc]
  (if (>= pos end) acc
    (let [digit (- (string-char-at src pos) 48)]
      (parse-int-from-str src (+ pos 1) end (+ (* acc 10) digit)))))

(defn current-symbol-text-v3 [spans pos-ref src]
  (substring src (p-start spans pos-ref) (p-end spans pos-ref)))

(defn current-symbol-hash-v3 [spans pos-ref src]
  (name-hash src (p-start spans pos-ref) (p-end spans pos-ref)))

;; === AST ノード構築ヘルパー ===

;; 整数リテラルノード: [1, value]
(defn make-int-node [value]
  (vector-push (vector-push (vector-new 2) 1) value))

;; 真偽値ノード: [2, 0/1]
(defn make-bool-node [b]
  (vector-push (vector-push (vector-new 2) 2) b))

;; 変数参照ノード: [4, name-hash]
(defn make-var-node [h]
  (vector-push (vector-push (vector-new 2) 4) h))

;; 文字列ノード: [3, start, end]
(defn make-string-node [start end]
  (vector-push (vector-push (vector-push (vector-new 3) 3) start) end))

;; vector の特定インデックスを置換する
;; 注: vector は不変なので新しい vector を組み立てる
(defn vector-set-at [vec idx new-val]
  (let [len (vector-length vec)
        result (vector-new len)]
    (vector-set-at-loop vec result idx new-val 0 len)))

(defn vector-set-at-loop [vec result idx new-val i len]
  (if (>= i len) result
    (if (= i idx)
      (vector-set-at-loop vec (vector-push result new-val)
        idx new-val (+ i 1) len)
      (vector-set-at-loop vec (vector-push result (vector-get vec i))
        idx new-val (+ i 1) len))))

;; === メインパーサー (v3: span ベース) ===

;; prefix quote 系: 'expr / ~expr / ~@expr
(defn parse-quote-v3 [spans pos-ref src]
  (do
    (p-advance pos-ref)
    (make-quote (parse-expr-v3 spans pos-ref src))))

(defn parse-unquote-v3 [spans pos-ref src]
  (do
    (p-advance pos-ref)
    (make-unquote (parse-expr-v3 spans pos-ref src))))

(defn parse-unquote-splice-v3 [spans pos-ref src]
  (do
    (p-advance pos-ref)
    (make-unquote-splice (parse-expr-v3 spans pos-ref src))))

;; record literal: {Point field1 expr1 field2 expr2 ...}
(defn brace-starts-recordlit-v3 [spans pos-ref src]
  (let [next-idx (+ (ref-get pos-ref) 1)
        next-kind (span-kind spans next-idx)]
    (if (== next-kind 20)
      (let [start (span-start spans next-idx)
            c (string-char-at src start)]
        (if (>= c 65)
          (if (<= c 90) 1 0)
          0))
      0)))

(defn parse-recordlit-v3 [spans pos-ref src]
  (do
    (p-advance pos-ref)  ;; { を消費
    (if (== (p-current spans pos-ref) 20)
      (let [type-start (p-start spans pos-ref)
            type-end (p-end spans pos-ref)
            type-h (name-hash src type-start type-end)
            result (make-recordlit type-h)]
        (do
          (p-advance pos-ref)  ;; type 名を消費
          (let [with-fields (parse-recordlit-fields-v3 spans pos-ref src result 0)
                field-count (/ (- (vector-length with-fields) 3) 2)]
            (vector-set-at with-fields 2 field-count))))
      (let [result (make-recordlit 0)
            with-fields (parse-recordlit-fields-v3 spans pos-ref src result 0)
            field-count (/ (- (vector-length with-fields) 3) 2)]
        (vector-set-at with-fields 2 field-count)))))

(defn parse-recordlit-fields-v3 [spans pos-ref src result count]
  (if (== (p-current spans pos-ref) 5)  ;; } で終了
    (do (p-advance pos-ref) result)
    (if (== (p-current spans pos-ref) 20)
      (let [field-start (p-start spans pos-ref)
            field-end (p-end spans pos-ref)
            field-h (name-hash src field-start field-end)]
        (do
          (p-advance pos-ref)  ;; field 名を消費
          (let [value (parse-expr-v3 spans pos-ref src)]
            (parse-recordlit-fields-v3 spans pos-ref src
              (vector-push (vector-push result field-h) value)
              (+ count 1)))))
      (do
        (p-advance pos-ref)
        (parse-recordlit-fields-v3 spans pos-ref src result count)))))

(defn parse-recordupdate-v3 [spans pos-ref src]
  (do
    (p-advance pos-ref)  ;; { を消費
    (let [base (parse-expr-v3 spans pos-ref src)
          result (make-recordupdate base)]
      (do
        (if (== (p-current spans pos-ref) 52)  ;; | を消費
          (do
            (p-advance pos-ref)
            0)
          0)
        (let [with-fields (parse-recordupdate-fields-v3 spans pos-ref src result 0)
              field-count (/ (- (vector-length with-fields) 3) 2)]
          (vector-set-at with-fields 2 field-count))))))

(defn parse-recordupdate-fields-v3 [spans pos-ref src result count]
  (if (== (p-current spans pos-ref) 5)  ;; } で終了
    (do (p-advance pos-ref) result)
    (if (== (p-current spans pos-ref) 20)
      (let [field-start (p-start spans pos-ref)
            field-end (p-end spans pos-ref)
            field-h (name-hash src field-start field-end)]
        (do
          (p-advance pos-ref)  ;; field 名を消費
          (let [value (parse-expr-v3 spans pos-ref src)]
            (parse-recordupdate-fields-v3 spans pos-ref src
              (vector-push (vector-push result field-h) value)
              (+ count 1)))))
      (do
        (p-advance pos-ref)
        (parse-recordupdate-fields-v3 spans pos-ref src result count)))))

(defn skip-type-expr-v3 [spans pos-ref]
  (if (== (p-current spans pos-ref) 0)
    (do
      (parse-skip-to-close-v3 spans pos-ref 1)
      0)
    (do
      (p-advance pos-ref)
      0)))

(defn skip-optional-type-sig-v3 [spans pos-ref]
  (if (== (p-current spans pos-ref) 50)  ;; :
    (do
      (p-advance pos-ref)
      (skip-type-expr-v3 spans pos-ref)
      0)
    0))

(defn parse-type-alias-v3 [spans pos-ref src]
  (do
    (p-advance pos-ref)  ;; type-alias を消費
    (if (== (p-current spans pos-ref) 0)
      (do
        (p-advance pos-ref)  ;; alias head の ( を消費
        (if (== (p-current spans pos-ref) 20)
          (let [name-h (current-symbol-hash-v3 spans pos-ref src)]
            (do
              (p-advance pos-ref)  ;; alias 名を消費
              (parse-skip-to-close-v3 spans pos-ref 1)
              (skip-type-expr-v3 spans pos-ref)
              (p-expect spans pos-ref 1)  ;; ) を消費
              (make-type-alias name-h)))
          (do
            (parse-skip-to-close-v3 spans pos-ref 1)
            (parse-skip-to-close-v3 spans pos-ref 1)
            (make-type-alias 0))))
      (if (== (p-current spans pos-ref) 20)
        (let [name-h (current-symbol-hash-v3 spans pos-ref src)]
          (do
            (p-advance pos-ref)  ;; alias 名を消費
            (skip-type-expr-v3 spans pos-ref)
            (p-expect spans pos-ref 1)  ;; ) を消費
            (make-type-alias name-h)))
        (do
          (parse-skip-to-close-v3 spans pos-ref 1)
          (make-type-alias 0))))))

(defn parse-type-constrained-v3 [spans pos-ref src]
  (do
    (p-advance pos-ref)  ;; type-constrained を消費
    (if (== (p-current spans pos-ref) 20)
      (let [name-h (current-symbol-hash-v3 spans pos-ref src)]
        (do
          (p-advance pos-ref)  ;; name を消費
          (parse-skip-to-close-v3 spans pos-ref 1)
          (make-type-constrained name-h)))
      (do
        (parse-skip-to-close-v3 spans pos-ref 1)
        (make-type-constrained 0)))))

(defn parse-computation-builder-v3 [spans pos-ref src]
  (do
    (p-advance pos-ref)  ;; computation-builder を消費
    (if (== (p-current spans pos-ref) 20)
      (let [name-h (current-symbol-hash-v3 spans pos-ref src)]
        (do
          (p-advance pos-ref)
          (if (== (p-current spans pos-ref) 20)
            (let [bind-h (current-symbol-hash-v3 spans pos-ref src)]
              (do
                (p-advance pos-ref)
                (if (== (p-current spans pos-ref) 20)
                  (let [return-h (current-symbol-hash-v3 spans pos-ref src)]
                    (do
                      (p-advance pos-ref)
                      (p-expect spans pos-ref 1)  ;; ) を消費
                      (make-computation-builder name-h bind-h return-h)))
                  (do
                    (parse-skip-to-close-v3 spans pos-ref 1)
                    (make-computation-builder name-h bind-h 0)))))
            (do
              (parse-skip-to-close-v3 spans pos-ref 1)
              (make-computation-builder name-h 0 0)))))
      (do
        (parse-skip-to-close-v3 spans pos-ref 1)
        (make-computation-builder 0 0 0)))))

(defn parse-impl-v3 [spans pos-ref src]
  (do
    (p-advance pos-ref)  ;; impl を消費
    (if (== (p-current spans pos-ref) 0)
      (do
        (p-advance pos-ref)  ;; impl head の ( を消費
        (if (== (p-current spans pos-ref) 20)
          (let [trait-h (current-symbol-hash-v3 spans pos-ref src)]
            (do
              (p-advance pos-ref)  ;; trait 名を消費
              (if (== (p-current spans pos-ref) 20)
                (let [type-h (current-symbol-hash-v3 spans pos-ref src)]
                  (do
                    (p-advance pos-ref)  ;; type 名を消費
                    (parse-skip-to-close-v3 spans pos-ref 2)
                    (make-impl-def trait-h type-h)))
                (do
                  (parse-skip-to-close-v3 spans pos-ref 1)
                  (parse-skip-to-close-v3 spans pos-ref 1)
                  (make-impl-def trait-h 0)))))
          (do
            (parse-skip-to-close-v3 spans pos-ref 2)
            (make-impl-def 0 0))))
      (do
        (parse-skip-to-close-v3 spans pos-ref 1)
        (make-impl-def 0 0)))))

(defn parse-symbol-form-v3 [spans pos-ref src]
  (let [name (current-symbol-text-v3 spans pos-ref src)]
    (if (string-eq name "type-alias")
      (parse-type-alias-v3 spans pos-ref src)
      (if (string-eq name "type-constrained")
        (parse-type-constrained-v3 spans pos-ref src)
        (if (string-eq name "computation-builder")
          (parse-computation-builder-v3 spans pos-ref src)
          (parse-apply-v3 spans pos-ref src))))))

;; 式のパース (メインディスパッチ)
(defn parse-expr-v3 [spans pos-ref src]
  (let [kind (p-current spans pos-ref)]
    (if (== kind 10)  ;; Int
      (let [start (p-start spans pos-ref)
            end (p-end spans pos-ref)
            value (parse-int-from-str src start end 0)]
        (do (p-advance pos-ref)
            (make-int-node value)))
      (if (== kind 13)  ;; true
        (do (p-advance pos-ref) (make-bool-node 1))
        (if (== kind 14)  ;; false
          (do (p-advance pos-ref) (make-bool-node 0))
          (if (== kind 12)  ;; String
            (let [start (p-start spans pos-ref)
                  end (p-end spans pos-ref)]
              (do (p-advance pos-ref)
                  (make-string-node (+ start 1) (- end 1)))) ;; 引用符を除く
            (if (== kind 54)  ;; '
              (parse-quote-v3 spans pos-ref src)
              (if (== kind 55)  ;; ~
                (parse-unquote-v3 spans pos-ref src)
                (if (== kind 56)  ;; ~@
                  (parse-unquote-splice-v3 spans pos-ref src)
                  (if (== kind 4)  ;; LBrace -> record literal
                    (if (= (brace-starts-recordlit-v3 spans pos-ref src) 1)
                      (parse-recordlit-v3 spans pos-ref src)
                      (parse-recordupdate-v3 spans pos-ref src))
                    (if (== kind 20)  ;; Symbol (変数参照)
                      (let [start (p-start spans pos-ref)
                            end (p-end spans pos-ref)
                            h (name-hash src start end)]
                        (do (p-advance pos-ref)
                            (make-var-node h)))
                      (if (== kind 0)  ;; LParen -> S 式
                        (parse-sexp-v3 spans pos-ref src)
                        ;; unknown token
                        (do (p-advance pos-ref)
                            (make-int-node 0))))))))))))))

;; S 式のパース (( の後のキーワードディスパッチ)
(defn parse-sexp-v3 [spans pos-ref src]
  (do
    (p-advance pos-ref)  ;; ( を消費
    (let [kind (p-current spans pos-ref)]
      (if (== kind 32)  ;; if
        (parse-if-v3 spans pos-ref src)
        (if (== kind 31)  ;; let
          (parse-let-v3 spans pos-ref src)
          (if (== kind 36)  ;; do
            (parse-do-v3 spans pos-ref src)
            (if (== kind 33)  ;; match
              (parse-match-v3 spans pos-ref src)
              (if (== kind 35)  ;; fn (lambda)
                (parse-lambda-v3 spans pos-ref src)
                (if (== kind 30)  ;; defn
                  (parse-defn-v3 spans pos-ref src)
                  (if (== kind 44)  ;; defmacro
                    (parse-defmacro-v3 spans pos-ref src)
                    (if (== kind 43)  ;; private
                      (parse-private-v3 spans pos-ref src)
                      (if (== kind 34)  ;; type
                        (parse-type-v3 spans pos-ref src)
                        (if (== kind 41)  ;; impl
                          (parse-impl-v3 spans pos-ref src)
                          (if (== kind 40)  ;; trait
                            (parse-trait-v3 spans pos-ref src)
                            (if (== kind 37)  ;; module
                              (parse-module-v3 spans pos-ref src)
                              (if (== kind 38)  ;; import
                                (parse-import-v3 spans pos-ref src)
                                (if (== kind 20)  ;; symbol-form
                                  (parse-symbol-form-v3 spans pos-ref src)
                                  ;; 関数適用 (apply)
                                  (parse-apply-v3 spans pos-ref src))))))))))))))))))

;; === if 式 ===
(defn parse-if-v3 [spans pos-ref src]
  (do
    (p-advance pos-ref)  ;; if を消費
    (let [cond-node (parse-expr-v3 spans pos-ref src)
          then-node (parse-expr-v3 spans pos-ref src)
          else-node (parse-expr-v3 spans pos-ref src)]
      (do
        (p-expect spans pos-ref 1)  ;; ) を消費
        (let [n (vector-new 8)]
          (vector-push (vector-push (vector-push (vector-push n 6)
            cond-node) then-node) else-node))))))

;; === let 式 (複数バインディング対応) ===
(defn parse-let-v3 [spans pos-ref src]
  (do
    (p-advance pos-ref)  ;; let を消費
    (p-expect spans pos-ref 2)  ;; [ を消費
    ;; 最初のバインディング
    (let [ns (p-start spans pos-ref)
          ne (p-end spans pos-ref)
          nh (name-hash src ns ne)]
      (do
        (p-advance pos-ref)  ;; name を消費
        (let [init (parse-expr-v3 spans pos-ref src)]
          ;; 追加バインディングがあるかチェック
          (if (== (p-current spans pos-ref) 3)  ;; ] で終了
            (do
              (p-advance pos-ref)  ;; ] を消費
              (let [body (parse-expr-v3 spans pos-ref src)]
                (do
                  (p-expect spans pos-ref 1)  ;; ) を消費
                  (let [n (vector-new 8)]
                    (vector-push (vector-push (vector-push (vector-push n 7)
                      nh) init) body)))))
            ;; 複数バインディング: 次のバインディングを body として再帰
            (let [ns2 (p-start spans pos-ref)
                  ne2 (p-end spans pos-ref)
                  nh2 (name-hash src ns2 ne2)]
              (do
                (p-advance pos-ref)  ;; name2 を消費
                (let [init2 (parse-expr-v3 spans pos-ref src)
                      ;; 残りのバインディングを処理
                      rest-body (parse-let-rest-v3 spans pos-ref src)]
                  ;; 内側の let を構築
                  (let [inner (vector-push (vector-push (vector-push
                                (vector-push (vector-new 8) 7) nh2) init2) rest-body)]
                    (do
                      (p-expect spans pos-ref 1)  ;; ) を消費
                      (let [n (vector-new 8)]
                        (vector-push (vector-push (vector-push (vector-push n 7)
                          nh) init) inner)))))))))))))

;; let の残りバインディングを処理
(defn parse-let-rest-v3 [spans pos-ref src]
  (if (== (p-current spans pos-ref) 3)  ;; ] に到達
    (do
      (p-advance pos-ref)  ;; ] を消費
      (parse-expr-v3 spans pos-ref src))  ;; body をパース
    ;; さらにバインディングがある
    (let [ns (p-start spans pos-ref)
          ne (p-end spans pos-ref)
          nh (name-hash src ns ne)]
      (do
        (p-advance pos-ref)  ;; name を消費
        (let [init (parse-expr-v3 spans pos-ref src)
              rest (parse-let-rest-v3 spans pos-ref src)]
          (let [n (vector-new 8)]
            (vector-push (vector-push (vector-push (vector-push n 7)
              nh) init) rest)))))))

;; === do 式 ===
(defn parse-do-v3 [spans pos-ref src]
  (do
    (p-advance pos-ref)  ;; do を消費
    (let [result (vector-push (vector-push (vector-new 16) 9) 0)]  ;; [9, count=0(後で更新)]
      (let [with-exprs (parse-do-exprs-v3 spans pos-ref src result 0)
            expr-count (- (vector-length with-exprs) 2)]
        (vector-set-at with-exprs 1 expr-count)))))

;; do 内の式を収集
(defn parse-do-exprs-v3 [spans pos-ref src result count]
  (if (== (p-current spans pos-ref) 1)  ;; ) で終了
    (do
      (p-advance pos-ref)  ;; ) を消費
      ;; count を更新 (index 1)
      result)
    (let [expr (parse-expr-v3 spans pos-ref src)]
      (parse-do-exprs-v3 spans pos-ref src
        (vector-push result expr) (+ count 1)))))

;; === match 式 ===
(defn parse-match-v3 [spans pos-ref src]
  (do
    (p-advance pos-ref)  ;; match を消費
    (let [scrutinee (parse-expr-v3 spans pos-ref src)
          result (vector-push (vector-push (vector-push (vector-new 16) 10)
                    scrutinee) 0)]  ;; [10, scrutinee, arm-count=0]
      (let [with-arms (parse-match-arms-v3 spans pos-ref src result 0)
            arm-count (/ (- (vector-length with-arms) 3) 2)]
        (vector-set-at with-arms 2 arm-count)))))

;; match の腕を収集
(defn parse-match-arms-v3 [spans pos-ref src result count]
  (if (== (p-current spans pos-ref) 1)  ;; ) で終了
    (do (p-advance pos-ref) result)
    (if (== (p-current spans pos-ref) 2)  ;; [ -> arm
      (do
        (p-advance pos-ref)  ;; [ を消費
        (let [pat (parse-expr-v3 spans pos-ref src)
              body (parse-expr-v3 spans pos-ref src)]
          (do
            (p-expect spans pos-ref 3)  ;; ] を消費
            (parse-match-arms-v3 spans pos-ref src
              (vector-push (vector-push result pat) body)
              (+ count 1)))))
      ;; 不正なトークン -> スキップ
      (do (p-advance pos-ref)
          (parse-match-arms-v3 spans pos-ref src result count)))))

;; === lambda (fn) 式 ===
(defn parse-lambda-v3 [spans pos-ref src]
  (do
    (p-advance pos-ref)  ;; fn を消費
    (p-expect spans pos-ref 2)  ;; [ を消費
    (let [result (vector-push (vector-push (vector-new 8) 8) 0)]  ;; [8, param-count=0]
      (let [with-params (parse-params-v3 spans pos-ref src result 0)
            param-count (- (vector-length with-params) 2)
            lambda-node (vector-set-at with-params 1 param-count)
            body (parse-expr-v3 spans pos-ref src)]
        (do
          (p-expect spans pos-ref 1)  ;; ) を消費
          (vector-push lambda-node body))))))

;; パラメータリストを収集 (名前ハッシュ)
(defn parse-params-v3 [spans pos-ref src result count]
  (if (== (p-current spans pos-ref) 3)  ;; ] で終了
    (do (p-advance pos-ref) result)
    (let [s (p-start spans pos-ref)
          e (p-end spans pos-ref)
          h (name-hash src s e)]
      (do
        (p-advance pos-ref)  ;; param を消費
        (parse-params-v3 spans pos-ref src
          (vector-push result h) (+ count 1))))))

;; === defn 式 ===
(defn parse-defn-v3 [spans pos-ref src]
  (do
    (p-advance pos-ref)  ;; defn を消費
    (let [ns (p-start spans pos-ref)
          ne (p-end spans pos-ref)
          nh (name-hash src ns ne)]
      (do
        (p-advance pos-ref)  ;; name を消費
        (p-expect spans pos-ref 2)  ;; [ を消費
        (let [result (vector-push (vector-push (vector-push (vector-new 8) 20) nh) 0)]
          (let [with-params (parse-params-v3 spans pos-ref src result 0)
                param-count (- (vector-length with-params) 3)
                defn-node (vector-set-at with-params 2 param-count)
                body (parse-expr-v3 spans pos-ref src)]
            (do
              (p-expect spans pos-ref 1)  ;; ) を消費
              (vector-push defn-node body))))))))

;; === defmacro 宣言 ===
(defn parse-defmacro-v3 [spans pos-ref src]
  (do
    (p-advance pos-ref)  ;; defmacro を消費
    (let [ns (p-start spans pos-ref)
          ne (p-end spans pos-ref)
          nh (name-hash src ns ne)]
      (do
        (p-advance pos-ref)  ;; name を消費
        (p-expect spans pos-ref 2)  ;; [ を消費
        (let [result (make-defmacro nh)]
          (let [with-params (parse-params-v3 spans pos-ref src result 0)
                param-count (- (vector-length with-params) 3)
                macro-node (vector-set-at with-params 2 param-count)]
            (do
              (skip-optional-type-sig-v3 spans pos-ref)
              (let [body (parse-expr-v3 spans pos-ref src)]
                (do
                  (p-expect spans pos-ref 1)  ;; ) を消費
                  (vector-push macro-node body))))))))))

;; === private 宣言 ===
(defn parse-private-v3 [spans pos-ref src]
  (do
    (p-advance pos-ref)  ;; private を消費
    (let [inner (parse-expr-v3 spans pos-ref src)]
      (do
        (p-expect spans pos-ref 1)  ;; ) を消費
        (make-private inner)))))

;; === type 宣言 (簡易) ===
(defn parse-type-v3 [spans pos-ref src]
  (do
    (p-advance pos-ref)  ;; type を消費
    (let [kind (p-current spans pos-ref)]
      (if (== kind 20)  ;; 型名シンボル
        (let [start (p-start spans pos-ref)
              end (p-end spans pos-ref)
              h (name-hash src start end)]
          (do
            (p-advance pos-ref)
            ;; いったん残りの variant / metadata は読み飛ばすが、
            ;; record 本体だけは RecordDef として識別する
            (let [head-kind (if (== (p-current spans pos-ref) 0)
                              (span-kind spans (+ (ref-get pos-ref) 1))
                              0)
                  skipped (parse-skip-to-close-v3 spans pos-ref 1)]
              (if (== head-kind 39)
                (make-record-def h)
                (make-type-decl h)))))
        (do
          (parse-skip-to-close-v3 spans pos-ref 1)
          (make-type-decl 0))))))

;; === trait 宣言 (簡易) ===
(defn parse-trait-v3 [spans pos-ref src]
  (do
    (p-advance pos-ref)  ;; trait を消費
    (if (== (p-current spans pos-ref) 0)
      (do
        (p-advance pos-ref)  ;; trait head の ( を消費
        (if (== (p-current spans pos-ref) 20)
          (let [name-start (p-start spans pos-ref)
                name-end (p-end spans pos-ref)
                name-h (name-hash src name-start name-end)]
            (do
              (p-advance pos-ref)  ;; trait 名を消費
              (parse-skip-to-close-v3 spans pos-ref 2)
              (make-trait-def name-h)))
          (do
            (parse-skip-to-close-v3 spans pos-ref 1)
            (make-trait-def 0))))
      (do
        (parse-skip-to-close-v3 spans pos-ref 1)
        (make-trait-def 0)))))

;; === module 宣言 ===
(defn parse-module-v3 [spans pos-ref src]
  (do
    (p-advance pos-ref)  ;; module を消費
    (let [name-start (p-start spans pos-ref)
          name-end (p-end spans pos-ref)
          name-h (name-hash src name-start name-end)]
      (do
        (p-advance pos-ref)  ;; name を消費
        (p-expect spans pos-ref 1)  ;; ) を消費
        (make-module-decl name-h)))))

;; === import 宣言 ===
(defn parse-import-v3 [spans pos-ref src]
  (do
    (p-advance pos-ref)  ;; import を消費
    (let [name-start (p-start spans pos-ref)
          name-end (p-end spans pos-ref)
          name-h (name-hash src name-start name-end)]
      (do
        (p-advance pos-ref)  ;; name を消費
        (p-expect spans pos-ref 1)  ;; ) を消費
        (make-import-decl name-h)))))

;; === apply (関数呼び出し) ===
(defn parse-apply-v3 [spans pos-ref src]
  (let [func-node (parse-expr-v3 spans pos-ref src)
        result (vector-push (vector-push (vector-push (vector-new 8) 5) func-node) 0)]
    (let [with-args (parse-apply-args-v3 spans pos-ref src result 0)
          arg-count (- (vector-length with-args) 3)]
      (vector-set-at with-args 2 arg-count))))

;; 引数を収集
(defn parse-apply-args-v3 [spans pos-ref src result count]
  (if (== (p-current spans pos-ref) 1)  ;; ) で終了
    (do (p-advance pos-ref) result)
    (let [arg (parse-expr-v3 spans pos-ref src)]
      (parse-apply-args-v3 spans pos-ref src
        (vector-push result arg) (+ count 1)))))

;; === Recovery + 診断収集 ===

;; 診断レコード: [severity code span message-hash]
;; severity: 0=error, 1=warning, 2=info
;; code: 整数エラーコード
;; span: ソース位置 (start)
;; message-hash: メッセージの名前ハッシュ
(defn make-diagnostic [severity code span message-hash]
  (let [d (vector-new 4)]
    (vector-push (vector-push (vector-push (vector-push d severity) code) span) message-hash)))

;; 診断コレクタ: 診断のベクタを管理
(defn collect-diagnostics []
  (vector-new 8))

;; 診断を追加
(defn add-diagnostic [diagnostics diag]
  (vector-push diagnostics diag))

;; 次の同期ポイント (閉じ括弧 or トップレベル) まで回復
;; kind=1 (RParen), kind=99 (EOF) で停止
(defn recover-to-next [spans pos-ref]
  (let [kind (p-current spans pos-ref)]
    (if (== kind 99) 0   ;; EOF で停止
      (if (== kind 1) 0  ;; ) で停止
        (do (p-advance pos-ref)
            (recover-to-next spans pos-ref))))))

;; recovery 付きパース: パースに失敗したら回復して診断を記録
;; 戻り値: [ast-node, diagnostics-vector]
(defn parse-with-recovery [spans pos-ref src diagnostics]
  (let [start-pos (ref-get pos-ref)
        kind (p-current spans pos-ref)]
    (if (== kind 99) ;; EOF
      (let [result (vector-new 2)]
        (vector-push (vector-push result (make-int-node 0)) diagnostics))
      ;; 不正なトークン (閉じ括弧が先に来た等) の場合 recovery
      (if (== kind 1) ;; 予期しない )
        (let [span (p-start spans pos-ref)
              diag (make-diagnostic 0 1001 span 0)
              diags (add-diagnostic diagnostics diag)]
          (do (p-advance pos-ref)
              (let [result (vector-new 2)]
                (vector-push (vector-push result (make-int-node 0)) diags))))
        (if (== kind 3) ;; 予期しない ]
          (let [span (p-start spans pos-ref)
                diag (make-diagnostic 0 1002 span 0)
                diags (add-diagnostic diagnostics diag)]
            (do (p-advance pos-ref)
                (let [result (vector-new 2)]
                  (vector-push (vector-push result (make-int-node 0)) diags))))
          ;; 通常パース
          (let [node (parse-expr-v3 spans pos-ref src)
                result (vector-new 2)]
            (vector-push (vector-push result node) diagnostics)))))))

;; === ユーティリティ ===

;; 対応する閉じ括弧まで読み飛ばし (ネスト対応)
(defn parse-skip-to-close-v3 [spans pos-ref depth]
  (if (<= depth 0) 0
    (let [kind (p-current spans pos-ref)]
      (do
        (p-advance pos-ref)
        (if (== kind 0)  ;; ( でネスト深くなる
          (parse-skip-to-close-v3 spans pos-ref (+ depth 1))
          (if (== kind 1)  ;; ) でネスト浅くなる
            (parse-skip-to-close-v3 spans pos-ref (- depth 1))
            (parse-skip-to-close-v3 spans pos-ref depth)))))))

;; === トップレベルパース ===

;; 複数のトップレベル式をパース
(defn parse-program-v3 [spans pos-ref src]
  (let [result (vector-new 16)]
    (parse-program-loop-v3 spans pos-ref src result)))

(defn parse-program-loop-v3 [spans pos-ref src result]
  (if (== (p-current spans pos-ref) 99)  ;; EOF
    result
    (let [expr (parse-expr-v3 spans pos-ref src)]
      (parse-program-loop-v3 spans pos-ref src
        (vector-push result expr)))))

;; ソース文字列をトークン化してから v3 パーサでプログラム (宣言の Vector) を返す
(defn parse-program [src]
  (let [spans (tokenize-with-spans src)
        pos-ref (ref-new 0)]
    (parse-program-v3 spans pos-ref src)))

;; === 旧 API (後方互換) ===

;; 現在のトークンを取得 (旧 kind-only 方式)
(defn current-tok [tokens pos]
  (vector-get tokens (ref-get pos)))

;; トークンを1つ進める
(defn advance [pos]
  (ref-set pos (+ (ref-get pos) 1)))

;; 期待するトークンを消費
(defn expect [tokens pos expected]
  (let [tok (current-tok tokens pos)]
    (if (== tok expected)
      (do (advance pos) tok)
      0)))

;; 結果は整数エンコード: tag * 10000 + value
(defn parse-expr [tokens pos src src-positions]
  (let [tok (current-tok tokens pos)]
    (if (== tok 0)
      (do (advance pos)
        (let [result (parse-sexp tokens pos src src-positions)]
          (do (expect tokens pos 1) result)))
      (if (== tok 10)
        (do (advance pos) (+ (* 1 10000) 0))
        (if (== tok 13)
          (do (advance pos) (+ (* 2 10000) 1))
          (if (== tok 14)
            (do (advance pos) (+ (* 2 10000) 0))
            (if (== tok 20)
              (do (advance pos) (+ (* 4 10000) 0))
              0)))))))

(defn parse-sexp [tokens pos src src-positions]
  (let [tok (current-tok tokens pos)]
    (if (== tok 30) (do (advance pos) (+ (* 20 10000) 0))
      (if (== tok 31) (do (advance pos) (+ (* 7 10000) 0))
        (if (== tok 32) (do (advance pos) (+ (* 6 10000) 0))
          (if (== tok 33) (do (advance pos) (+ (* 10 10000) 0))
            (if (== tok 36) (do (advance pos) (+ (* 9 10000) 0))
              (+ (* 5 10000) 0))))))))

(defn node-tag [encoded]
  (/ encoded 10000))

(defn parse-toplevel [tokens pos src]
  (parse-expr tokens pos src (vector-new 0)))

;; エントリポイント (テスト用)
(defn main []
  (let [;; defn テスト
        tokens (vector-push (vector-push (vector-push (vector-push
                (vector-push (vector-push (vector-push (vector-push
                  (vector-new 8) 0) 30) 20) 2) 3) 10) 1) 99)
        pos (ref-new 0)
        result (parse-toplevel tokens pos "")
        ;; match テスト: (match x [1 10] [2 20])
        match-tokens (let [v (vector-new 16)]
                       (let [v1 (vector-push v 0)
                             v2 (vector-push v1 33)
                             v3 (vector-push v2 20)
                             v4 (vector-push v3 2)
                             v5 (vector-push v4 10)
                             v6 (vector-push v5 10)
                             v7 (vector-push v6 3)
                             v8 (vector-push v7 2)
                             v9 (vector-push v8 10)
                             v10 (vector-push v9 10)
                             v11 (vector-push v10 3)
                             v12 (vector-push v11 1)
                             v13 (vector-push v12 99)]
                         v13))
        match-pos (ref-new 0)
        match-result (parse-toplevel match-tokens match-pos "")
        ;; make-match-node テスト (旧 API ヘルパー)
        scr (make-int-node 5)
        mn (vector-push (vector-push (vector-push (vector-new 8) 10) scr) 2)
        mn1 (vector-push (vector-push mn 1) (make-int-node 10))
        mn2 (vector-push (vector-push mn1 2) (make-int-node 20))]
    (do
      (print (node-tag result))       ;; 20 (defn)
      (print (ref-get pos))           ;; 2 (パース後位置)
      (print (node-tag match-result)) ;; 10 (match)
      ;; match ノードのタグ検証
      (print (vector-get mn2 0))      ;; 10 (match tag)
      (print (vector-get mn2 2))      ;; 2 (arm-count)
      ;; 腕のパターン値
      (print (vector-get mn2 3))      ;; 1 (pat1)
      (print (vector-get mn2 5))      ;; 2 (pat2)
      0)))
