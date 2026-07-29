(module Syntax.Parser)
(import Syntax.Token)
(import Syntax.AST)
(import Syntax.Lexer)

;; Parser.ls - L# セルフホスティング: 再帰降下パーサー
;;
;; Lexer が出力したトークン列 (3つ組 Vector) を受け取り、AST を構築する。
;; S 式構文なので、パーサーは比較的シンプル。
;;
;; === AST ノード表現 (vector ベース) ===
;; [tag, ...data]
;; tag=1: int [1, value]
;; tag=2: bool [2, 0/1]
;; tag=3: string [3, start, end, map-key-hash]  (ソース位置参照)
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
;; tag=26: import-decl [26, name-hash, name-start, name-end]
;; :as を含む場合は [26, name-hash, name-start, name-end, alias-hash]
;; :only を含む場合は [26, name-hash, name-start, name-end, alias-hash, only-hashes]

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
  (let [pos (ref-get pos-ref)]
    (if (>= (* pos 3) (vector-length spans))
      99 ;; EOF ガード: spans 境界外は EOF として扱う
      (span-kind spans pos))))

;; パーサー位置を1つ進める
(defn p-advance [pos-ref]
  (ref-set pos-ref (+ (ref-get pos-ref) 1)))

;; 現在のトークンの start を取得
(defn p-start [spans pos-ref]
  (span-start spans (ref-get pos-ref)))

;; 現在のトークンの end を取得
(defn p-end [spans pos-ref]
  (span-end spans (ref-get pos-ref)))

(defn previous-token-end-v3 [spans pos-ref]
  (let [idx (- (ref-get pos-ref) 1)]
    (if (>= idx 0) (span-end spans idx) 0)))

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

(defn symbol-dot-position-loop [src pos end]
  (if (>= pos end)
    -1
    (if (= (string-char-at src pos) 46)
      pos
      (symbol-dot-position-loop src (+ pos 1) end))))

(defn symbol-dot-position [src start end]
  (symbol-dot-position-loop src start end))

;; === 数値パース ===

(defn parse-int-digits-from-str [src pos end acc]
  (if (>= pos end) acc
    (let [digit (- (string-char-at src pos) 48)]
      (parse-int-digits-from-str src (+ pos 1) end (+ (* acc 10) digit)))))

(defn parse-int-from-str [src pos end acc]
  (if (>= pos end) acc
    (if (== (string-char-at src pos) 45)
      (- 0 (parse-int-digits-from-str src (+ pos 1) end 0))
      (parse-int-digits-from-str src pos end acc))))

(defn current-symbol-text-v3 [spans pos-ref src]
  (substring src (p-start spans pos-ref) (p-end spans pos-ref)))

(defn current-symbol-starts-uppercase-v3 [spans pos-ref src]
  (let [first-char (string-char-at src (p-start spans pos-ref))]
    (if (>= first-char 65)
      (if (<= first-char 90) 1 0)
      0)))

(defn current-symbol-hash-v3 [spans pos-ref src]
  (name-hash src (p-start spans pos-ref) (p-end spans pos-ref)))

(defn current-type-name-hash-v3 [spans pos-ref src]
  (let [start (p-start spans pos-ref)
    end (p-end spans pos-ref)
    dot-pos (symbol-dot-position src start end)]
    (if (>= dot-pos 0)
      (ast-qualified-name-hash
        (name-hash src start dot-pos)
        (name-hash src (+ dot-pos 1) end))
      (name-hash src start end))))

;; qualified type name の record marker 用 raw suffix hashを返す。
(defn current-type-name-suffix-hash-v3 [spans pos-ref src]
  (let [start (p-start spans pos-ref)
    end (p-end spans pos-ref)
    dot-pos (symbol-dot-position src start end)]
    (if (>= dot-pos 0)
      (name-hash src (+ dot-pos 1) end)
      (name-hash src start end))))

(defn current-type-name-qualified-v3 [spans pos-ref src]
  (let [start (p-start spans pos-ref)
    end (p-end spans pos-ref)]
    (if (>= (symbol-dot-position src start end) 0) 1 0)))

;; ftable 経路は AST だけを受け取るため、Map の文字列キー用 hash を
;; パース時に保持する。source 経路と同じく、エスケープ後の文字列を hash する。
(defn string-literal-map-hash-escaped-char [escaped]
  (if (= escaped 110)
    10
    (if (= escaped 116)
      9
      (if (= escaped 114)
        13
        (if (= escaped 34)
          34
          (if (= escaped 92)
            92
            escaped))))))

(defn string-literal-map-hash-loop [src pos end acc]
  (if (>= pos end)
    acc
    (let [char (string-char-at src pos)]
      (if (= char 92)
        (if (< (+ pos 1) end)
          (let [escaped (string-char-at src (+ pos 1))]
            (string-literal-map-hash-loop
              src
              (+ pos 2)
              end
              (+ (string-literal-map-hash-escaped-char escaped) (* acc 31))))
          (string-literal-map-hash-loop src (+ pos 1) end (+ char (* acc 31))))
        (string-literal-map-hash-loop src (+ pos 1) end (+ char (* acc 31)))))))

(defn string-literal-map-hash [src start end]
  (let [hash (string-literal-map-hash-loop src start end 0)]
    (if (= hash 0) 2 (if (= hash -1) 1 hash))))

;; === AST ノード構築ヘルパー ===

;; 整数リテラルノード: [1, value]
(defn make-int-node [value]
  (vector-push-pair-rooted-v3 (vector-new 2) 1 value))

;; 真偽値ノード: [2, 0/1]
(defn make-bool-node [b]
  (vector-push-pair-rooted-v3 (vector-new 2) 2 b))

;; 変数参照ノード: [4, name-hash]
(defn make-var-node [h]
  (vector-push-pair-rooted-v3 (vector-new 2) 4 h))

;; parser 由来の変数参照ノード: [4, name-hash, start, end]
(defn make-var-node-with-span [h start end]
  (vector-push-quad-rooted-v3 (vector-new 4) 4 h start end))

;; qualified symbol の変数参照ノード: [4, name-hash, start, end, prefix-hash, suffix-hash]
(defn make-var-node-with-qualified-span [h start end prefix-hash suffix-hash]
  (vector-push-pair-rooted-v3
    (vector-push-quad-rooted-v3 (vector-new 6) 4 h start end)
    prefix-hash
    suffix-hash))

;; 文字列ノード: [3, start, end, map-key-hash]
(defn make-string-node [start end map-key-hash]
  (vector-push-quad-rooted-v3 (vector-new 4) 3 start end map-key-hash))

;; 浮動小数点リテラルノード: [19, start, end]
(defn make-float-node [start end]
  (vector-push-triple-rooted-v3 (vector-new 3) 19 start end))

;; unit リテラルノード: [32]
(defn make-unit-node []
  (vector-push-single-rooted-v3 (vector-new 1) (ast-lit-unit)))

;; 計算式ノード: [15, builder-hash, step-count, step-kind1, aux1, expr1, ...]
(defn make-computation-node [builder-hash]
  (vector-push-triple-rooted-v3 (vector-new 8) 15 builder-hash 0))

(defn computation-add-step [node step-kind aux expr]
  (let [count (vector-get node 2)
    updated (vector-set-at-rooted-v3 node 2 (+ count 1))]
    (do
      (root_push updated)
      (root_push expr)
      (let [result (vector-push-triple-rooted-v3 updated step-kind aux expr)]
        (do
          (root_pop)
          (root_pop)
          result)))))

(defn make-computation-step-node [step-kind aux expr]
  (do
    (root_push expr)
    (let [result (vector-push-triple-rooted-v3 (vector-new 3) step-kind aux expr)]
      (do
        (root_pop)
        result))))

;; vector の特定インデックスを置換する
;; 注: vector は不変なので新しい vector を組み立てる
(defn vector-set-at [vec idx new-val]
  (do
    (root_push vec)
    (root_push new-val)
    (let [len (vector-length vec)
      result (vector-new len)]
      (do
        (root_push result)
        (let [updated (vector-set-at-loop vec result idx new-val 0 len)]
          (do
            (root_pop)
            (root_pop)
            (root_pop)
            updated))))))

(defn vector-set-at-loop [vec result idx new-val i len]
  (if (>= i len) result
    (do
      (root_push vec)
      (root_push result)
      (root_push new-val)
      (let [next-result
        (if (= i idx)
          (vector-push result new-val)
          (vector-push result (vector-get vec i)))]
        (do
          (root_pop)
          (root_pop)
          (root_pop)
          (vector-set-at-loop vec next-result idx new-val (+ i 1) len))))))

(defn vector-set-at-rooted-v3 [vec idx new-val]
  (do
    (root_push vec)
    (root_push new-val)
    (let [updated (vector-set-at vec idx new-val)]
      (do
        (root_pop)
        (root_pop)
        updated))))

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
    (p-advance pos-ref) ;; { を消費
    (if (== (p-current spans pos-ref) 20)
      (let [type-qualified (current-type-name-qualified-v3 spans pos-ref src)
        type-h (current-type-name-hash-v3 spans pos-ref src)
        raw-type-h (current-type-name-suffix-hash-v3 spans pos-ref src)
        result (make-recordlit type-h)
        result-slot (root_push result)]
        (do
          (p-advance pos-ref) ;; type 名を消費
          (let [with-fields (parse-recordlit-fields-v3 spans pos-ref src result 0)
            field-count (/ (- (vector-length with-fields) 3) 2)
            normalized (do
              (root_set result-slot with-fields)
              (vector-set-at-rooted-v3 with-fields 2 field-count))
            parsed-with-flag (do
              (root_set result-slot normalized)
              (vector-push-single-rooted-v3 normalized type-qualified))]
            (do
              (root_pop)
              (vector-push-single-rooted-v3 parsed-with-flag raw-type-h)))))
      (let [result (make-recordlit 0)
        result-slot (root_push result)]
        (do
          (let [with-fields (parse-recordlit-fields-v3 spans pos-ref src result 0)
            field-count (/ (- (vector-length with-fields) 3) 2)
            parsed (do
              (root_set result-slot with-fields)
              (vector-set-at-rooted-v3 with-fields 2 field-count))]
            (do
              (root_pop)
              parsed)))))))

(defn vector-push-single-rooted-v3 [base value]
  (do
    (root_push value)
    (let [base-slot (root_push base)
      result (vector-push base value)]
      (do
        (root_set base-slot result)
        (root_pop)
        (root_pop)
        result))))

(defn vector-push-pair-rooted-v3 [base first second]
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

(defn vector-push-triple-rooted-v3 [base first second third]
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

(defn vector-push-quad-rooted-v3 [base first second third fourth]
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

(defn parse-recordlit-fields-step-v3 [spans pos-ref src result]
  (if (== (p-current spans pos-ref) 5) ;; } で終了
    (do
      (p-advance pos-ref)
      (make-parse-loop-state 1 result))
    (if (== (p-current spans pos-ref) 99) ;; EOF ガード: 無限ループ防止
      (make-parse-loop-state 1 result)
      (if (== (p-current spans pos-ref) 20)
        (do
          (let [result-slot (root_push result)
            field-start (p-start spans pos-ref)
            field-end (p-end spans pos-ref)
            field-h (name-hash src field-start field-end)]
            (do
              (p-advance pos-ref) ;; field 名を消費
              (let [value (parse-expr-v3 spans pos-ref src)]
                (do
                  (root_push value)
                  (let [next-result (vector-push-pair-rooted-v3 result field-h value)
                    state (do
                      (root_set result-slot next-result)
                      (make-parse-loop-state 0 next-result))]
                    (do
                      (root_pop)
                      (root_pop)
                      state)))))))
        (do
          (p-advance pos-ref)
          (make-parse-loop-state 0 result))))))

(defn parse-recordlit-fields-step-64-loop-bounded [spans pos-ref src result remaining]
  (do
    (root_push result)
    (let [step (parse-recordlit-fields-step-v3 spans pos-ref src result)
      done (vector-get step 0)
      next-result (vector-get step 1)]
      (do
        (root_push step)
        (root_push next-result)
        (let [parsed
          (if (= done 1)
            step
            (if (<= remaining 1)
              step
              (parse-recordlit-fields-step-64-loop-bounded spans pos-ref src next-result (- remaining 1))))]
          (do
            (root_pop)
            (root_pop)
            (root_pop)
            parsed))))))

(defn parse-recordlit-fields-step-64 [spans pos-ref src result]
  (parse-recordlit-fields-step-64-loop-bounded spans pos-ref src result 64))

(defn parse-recordlit-fields-rooted-v3 [spans pos-ref src result count]
  (let [step (parse-recordlit-fields-step-64 spans pos-ref src result)]
    (if (= (vector-get step 0) 1)
      (vector-get step 1)
      (do
        (root_push step)
        (let [next-result (vector-get step 1)]
          (do
            (root_push next-result)
            (let [parsed (parse-recordlit-fields-rooted-v3 spans pos-ref src next-result count)]
              (do
                (root_pop)
                (root_pop)
                parsed))))))))

(defn parse-recordlit-fields-v3 [spans pos-ref src result count]
  (do
    (root_push spans)
    (root_push pos-ref)
    (root_push src)
    (let [parsed (parse-recordlit-fields-rooted-v3 spans pos-ref src result count)]
      (do
        (root_pop)
        (root_pop)
        (root_pop)
        parsed))))

(defn parse-recordupdate-v3 [spans pos-ref src]
  (do
    (p-advance pos-ref) ;; { を消費
    (let [base (parse-expr-v3 spans pos-ref src)]
      (do
        (root_push base)
        (let [result (make-recordupdate base)
          result-slot (root_push result)]
          (do
            (if (== (p-current spans pos-ref) 52) ;; | を消費
              (do
                (p-advance pos-ref)
                0)
              0)
            (let [with-fields (parse-recordupdate-fields-v3 spans pos-ref src result 0)
              field-count (/ (- (vector-length with-fields) 3) 2)
              parsed (do
                (root_set result-slot with-fields)
                (vector-set-at-rooted-v3 with-fields 2 field-count))]
              (do
                (root_pop)
                (root_pop)
                parsed))))))))

(defn parse-recordupdate-fields-step-v3 [spans pos-ref src result]
  (if (== (p-current spans pos-ref) 5) ;; } で終了
    (do
      (p-advance pos-ref)
      (make-parse-loop-state 1 result))
    (if (== (p-current spans pos-ref) 99) ;; EOF ガード: 無限ループ防止
      (make-parse-loop-state 1 result)
      (if (== (p-current spans pos-ref) 20)
        (do
          (let [result-slot (root_push result)
            field-start (p-start spans pos-ref)
            field-end (p-end spans pos-ref)
            field-h (name-hash src field-start field-end)]
            (do
              (p-advance pos-ref) ;; field 名を消費
              (let [value (parse-expr-v3 spans pos-ref src)]
                (do
                  (root_push value)
                  (let [next-result (vector-push-pair-rooted-v3 result field-h value)
                    state (do
                      (root_set result-slot next-result)
                      (make-parse-loop-state 0 next-result))]
                    (do
                      (root_pop)
                      (root_pop)
                      state)))))))
        (do
          (p-advance pos-ref)
          (make-parse-loop-state 0 result))))))

(defn parse-recordupdate-fields-step-64-loop-bounded [spans pos-ref src result remaining]
  (do
    (root_push result)
    (let [step (parse-recordupdate-fields-step-v3 spans pos-ref src result)
      done (vector-get step 0)
      next-result (vector-get step 1)]
      (do
        (root_push step)
        (root_push next-result)
        (let [parsed
          (if (= done 1)
            step
            (if (<= remaining 1)
              step
              (parse-recordupdate-fields-step-64-loop-bounded spans pos-ref src next-result (- remaining 1))))]
          (do
            (root_pop)
            (root_pop)
            (root_pop)
            parsed))))))

(defn parse-recordupdate-fields-step-64 [spans pos-ref src result]
  (parse-recordupdate-fields-step-64-loop-bounded spans pos-ref src result 64))

(defn parse-recordupdate-fields-rooted-v3 [spans pos-ref src result count]
  (let [step (parse-recordupdate-fields-step-64 spans pos-ref src result)]
    (if (= (vector-get step 0) 1)
      (vector-get step 1)
      (do
        (root_push step)
        (let [next-result (vector-get step 1)]
          (do
            (root_push next-result)
            (let [parsed (parse-recordupdate-fields-rooted-v3 spans pos-ref src next-result count)]
              (do
                (root_pop)
                (root_pop)
                parsed))))))))

(defn parse-recordupdate-fields-v3 [spans pos-ref src result count]
  (do
    (root_push spans)
    (root_push pos-ref)
    (root_push src)
    (let [parsed (parse-recordupdate-fields-rooted-v3 spans pos-ref src result count)]
      (do
        (root_pop)
        (root_pop)
        (root_pop)
        parsed))))

(defn skip-type-expr-v3 [spans pos-ref]
  (if (== (p-current spans pos-ref) 0)
    (do
      (parse-skip-to-close-v3 spans pos-ref 1)
      0)
    (do
      (p-advance pos-ref)
      0)))

;; 型リストを読み、対応する ) までの raw TypeExpr を収集する。
(defn parse-type-expr-list-step-v3 [spans pos-ref src result]
  (if (== (p-current spans pos-ref) 1)
    (do
      (p-advance pos-ref)
      (make-parse-loop-state 1 result))
    (if (== (p-current spans pos-ref) 99)
      (make-parse-loop-state 1 result)
      (do
        (let [result-slot (root_push result)
          item (parse-type-expr-v3 spans pos-ref src)]
          (do
            (root_push item)
            (let [next-result (vector-push-single-rooted-v3 result item)]
              (do
                (root_set result-slot next-result)
                (let [state (make-parse-loop-state 0 next-result)]
                  (do
                    (root_pop)
                    (root_pop)
                    state))))))))))

(defn parse-type-expr-list-step-64-loop-bounded [spans pos-ref src result remaining]
  (do
    (root_push result)
    (let [step (parse-type-expr-list-step-v3 spans pos-ref src result)
      done (vector-get step 0)
      next-result (vector-get step 1)]
      (do
        (root_push step)
        (root_push next-result)
        (let [parsed
          (if (= done 1)
            step
            (if (<= remaining 1)
              step
              (parse-type-expr-list-step-64-loop-bounded spans pos-ref src next-result (- remaining 1))))]
          (do
            (root_pop)
            (root_pop)
            (root_pop)
            parsed))))))

(defn parse-type-expr-list-step-64 [spans pos-ref src result]
  (parse-type-expr-list-step-64-loop-bounded spans pos-ref src result 64))

(defn parse-type-expr-list-rooted-v3 [spans pos-ref src result]
  (let [step (parse-type-expr-list-step-64 spans pos-ref src result)]
    (if (= (vector-get step 0) 1)
      (vector-get step 1)
      (do
        (root_push step)
        (let [next-result (vector-get step 1)]
          (do
            (root_push next-result)
            (let [parsed (parse-type-expr-list-rooted-v3 spans pos-ref src next-result)]
              (do
                (root_pop)
                (root_pop)
                parsed))))))))

(defn parse-type-expr-list-v3 [spans pos-ref src result]
  (do
    (root_push spans)
    (root_push pos-ref)
    (root_push src)
    (let [parsed (parse-type-expr-list-rooted-v3 spans pos-ref src result)]
      (do
        (root_pop)
        (root_pop)
        (root_pop)
        parsed))))

(defn parse-type-app-expr-v3 [spans pos-ref src name-hash]
  (let [args (vector-new 0)
    args-slot (root_push args)]
    (let [parsed-args (parse-type-expr-list-v3 spans pos-ref src args)]
      (do
        (root_set args-slot parsed-args)
        (let [parsed (make-type-app-expr name-hash parsed-args)]
          (do
            (root_pop)
            parsed))))))

(defn parse-type-fun-expr-v3 [spans pos-ref src]
  (do
    (p-advance pos-ref) ;; -> を消費
    (let [all-types (vector-new 0)
      all-types-slot (root_push all-types)]
      (let [parsed-types (parse-type-expr-list-v3 spans pos-ref src all-types)]
        (do
          (root_set all-types-slot parsed-types)
          (let [count (vector-length parsed-types)]
            (if (<= count 0)
              (do
                (root_pop)
                (make-type-named 0))
              (let [return-type (vector-get parsed-types (- count 1))
                params (type-expr-prefix parsed-types (- count 1))
                parsed (make-type-fun-expr params return-type)]
                (do
                  (root_pop)
                  parsed)))))))))

(defn parse-type-list-expr-v3 [spans pos-ref src]
  (do
    (p-advance pos-ref) ;; ( を消費
    (if (== (p-current spans pos-ref) 51)
      (parse-type-fun-expr-v3 spans pos-ref src)
      (if (== (p-current spans pos-ref) 20)
        (let [name-hash (current-symbol-hash-v3 spans pos-ref src)]
          (do
            (p-advance pos-ref)
            (parse-type-app-expr-v3 spans pos-ref src name-hash)))
        (do
          (parse-skip-to-close-v3 spans pos-ref 1)
          (make-type-named 0))))))

;; raw TypeExpr parser。Named、applied type、function type を保持する。
(defn parse-type-expr-v3 [spans pos-ref src]
  (if (== (p-current spans pos-ref) 20)
    (let [name-hash (current-type-name-hash-v3 spans pos-ref src)
      uppercase (current-symbol-starts-uppercase-v3 spans pos-ref src)]
      (do
        (p-advance pos-ref)
        (if (= uppercase 1)
          (make-type-named name-hash)
          (make-type-var-expr name-hash))))
    (if (== (p-current spans pos-ref) 0)
      (parse-type-list-expr-v3 spans pos-ref src)
      (do
        (skip-type-expr-v3 spans pos-ref)
        (make-type-named 0)))))

(defn source-directive-symbol-v3 [name]
  (if (string-eq name "intent") 1
    (if (string-eq name "claim") 1
      (if (string-eq name "assumption") 1
        (if (string-eq name "open-question") 1
          (if (string-eq name "motivates") 1
            (if (string-eq name "constrained-by") 1
              (if (string-eq name "tested-by") 1
                (if (string-eq name "supports") 1
                (if (string-eq name "contradicts") 1
                  (if (string-eq name "evidence") 1 0)))))))))))

(defn directive-symbol-v3 [name]
  (if (string-eq name "where") 1
    (if (string-eq name "doc") 1
      (if (string-eq name "params") 1
        (if (string-eq name "returns") 1
          (if (string-eq name "rationale") 1
            (if (string-eq name "since") 1
              (if (string-eq name "see-also") 1
                  (if (string-eq name "example") 1
                    (if (string-eq name "invariant") 1
                      (if (string-eq name "case") 1
                        (if (string-eq name "assert") 1
                        (if (string-eq name "property") 1
                          (if (string-eq name "transitions") 1
                            (if (string-eq name "constraints") 1
                              (source-directive-symbol-v3 name))))))))))))))))

(defn colon-directive-v3 [spans pos-ref src]
  (if (== (p-current spans pos-ref) 50)
    (let [next-idx (+ (ref-get pos-ref) 1)
      next-kind (span-kind spans next-idx)]
      (if (== next-kind 42) 1
        (if (== next-kind 20)
          (directive-symbol-v3
            (substring src (span-start spans next-idx) (span-end spans next-idx)))
          0)))
    0))

(defn parse-skip-bracket-v3 [spans pos-ref depth]
  (if (<= depth 0) 0
    (let [kind (p-current spans pos-ref)]
      (if (== kind 99)
        0
        (do
          (p-advance pos-ref)
          (if (== kind 2)
            (parse-skip-bracket-v3 spans pos-ref (+ depth 1))
            (if (== kind 3)
              (parse-skip-bracket-v3 spans pos-ref (- depth 1))
              (parse-skip-bracket-v3 spans pos-ref depth))))))))

(defn parse-skip-brace-v3 [spans pos-ref depth]
  (if (<= depth 0) 0
    (let [kind (p-current spans pos-ref)]
      (do
        (p-advance pos-ref)
        (if (== kind 4)
          (parse-skip-brace-v3 spans pos-ref (+ depth 1))
          (if (== kind 5)
            (parse-skip-brace-v3 spans pos-ref (- depth 1))
            (parse-skip-brace-v3 spans pos-ref depth)))))))

(defn skip-directive-payload-v3 [spans pos-ref]
  (let [kind (p-current spans pos-ref)]
    (if (== kind 0)
      (parse-skip-to-close-v3 spans pos-ref 1)
      (if (== kind 2)
        (do
          (p-advance pos-ref)
          (parse-skip-bracket-v3 spans pos-ref 1))
        (if (== kind 4)
          (do
            (p-advance pos-ref)
            (parse-skip-brace-v3 spans pos-ref 1))
          (do
            (p-advance pos-ref)
            0))))))

(defn skip-optional-metadata-rooted-v3 [spans pos-ref src]
  (if (== (colon-directive-v3 spans pos-ref src) 1)
    (do
      (p-advance pos-ref)
      (p-advance pos-ref)
      (skip-directive-payload-v3 spans pos-ref)
      (skip-optional-metadata-rooted-v3 spans pos-ref src))
    0))

(defn skip-optional-metadata-v3 [spans pos-ref src]
  (do
    (root_push spans)
    (root_push pos-ref)
    (root_push src)
    (let [result (skip-optional-metadata-rooted-v3 spans pos-ref src)]
      (do
        (root_pop)
        (root_pop)
        (root_pop)
        result))))

;; defn 用メタデータパーサー: :doc / :example / :params / :returns / :invariant / :case を記録する
;; 返却: [doc-string, example-text, params-vector, returns-string, invariant-expr, ordered-forms]
;; ordered form: [kind, payload, directive-start, directive-end]
(defn make-empty-defn-metadata-v3 []
  (let [params0 (vector-new 0)
    forms0 (vector-new 0)]
    (do
      (root_push params0)
      (root_push forms0)
      (let [meta4 (vector-push-quad-rooted-v3 (vector-new 4) "" "" params0 "")]
        (do
          (root_pop)
          (let [meta5 (vector-push-single-rooted-v3 meta4 0)]
            (do
              (root_push meta5)
              (let [result (vector-push-single-rooted-v3 meta5 forms0)]
                (do
                  (root_pop)
                  (root_pop)
                  result)))))))))

(defn metadata-directive-start-v3 [spans pos-ref]
  (let [idx (- (ref-get pos-ref) 2)]
    (if (>= idx 0) (span-start spans idx) 0)))

(defn metadata-directive-end-v3 [spans pos-ref]
  (let [idx (- (ref-get pos-ref) 1)]
    (if (>= idx 0) (span-end spans idx) 0)))

(defn append-defn-metadata-form-v3 [meta kind payload start end]
  (do
    ;; forms は form の確保より前に root 化する。native moving GC が meta の子を移動しても参照を保つ。
    (root_push meta)
    (root_push payload)
    (let [forms (vector-get meta 5)]
      (do
        (root_push forms)
        (let [form (vector-push-quad-rooted-v3 (vector-new 4) kind payload start end)]
          (do
            (root_push form)
            (let [updated-forms (vector-push-single-rooted-v3 forms form)]
              (do
                (root_push updated-forms)
                (let [updated-meta (vector-set-at-rooted-v3 meta 5 updated-forms)]
                  (do
                    (root_pop)
                    (root_pop)
                    (root_pop)
                    (root_pop)
                    (root_pop)
                    updated-meta))))))))))

(defn append-defn-metadata-form-with-extra-v3 [meta kind payload start end extra]
  (do
    ;; forms と extra は base-form の確保前から保持し、移動後の値を次の vector 操作へ渡す。
    (root_push meta)
    (root_push payload)
    (let [forms (vector-get meta 5)]
      (do
        (root_push forms)
        (root_push extra)
        (let [base-form (vector-push-quad-rooted-v3 (vector-new 4) kind payload start end)]
          (do
            (root_push base-form)
            (let [form (vector-push-single-rooted-v3 base-form extra)]
              (do
                (root_push form)
                (let [updated-forms (vector-push-single-rooted-v3 forms form)]
                  (do
                    (root_push updated-forms)
                    (let [updated-meta (vector-set-at-rooted-v3 meta 5 updated-forms)]
                      (do
                        (root_pop)
                        (root_pop)
                        (root_pop)
                        (root_pop)
                        (root_pop)
                        (root_pop)
                        (root_pop)
                        updated-meta))))))))))))

(defn parse-defn-metadata-v3 [spans pos-ref src]
  (parse-defn-metadata-loop-v3 spans pos-ref src (make-empty-defn-metadata-v3)))

(defn parse-defn-metadata-loop-rooted-v3 [spans pos-ref src meta]
  (if (== (colon-directive-v3 spans pos-ref src) 1)
    (do
      (root_push meta)
      (let [dir-idx (+ (ref-get pos-ref) 1)
        dir-name (substring src (span-start spans dir-idx) (span-end spans dir-idx))]
        (do
          (root_push dir-name)
          (p-advance pos-ref)
          (p-advance pos-ref)
          (let [result
            (if (string-eq dir-name "doc")
              (parse-defn-meta-doc-v3 spans pos-ref src meta)
              (if (string-eq dir-name "example")
                (parse-defn-meta-example-v3 spans pos-ref src meta)
                (if (string-eq dir-name "params")
                  (parse-defn-meta-params-v3 spans pos-ref src meta)
                  (if (string-eq dir-name "returns")
                    (parse-defn-meta-returns-v3 spans pos-ref src meta)
                    (if (string-eq dir-name "invariant")
                      (parse-defn-meta-invariant-v3 spans pos-ref src meta)
                      (if (string-eq dir-name "case")
                        (parse-defn-meta-case-v3 spans pos-ref src meta)
                        (if (string-eq dir-name "assert")
                          (parse-defn-meta-assert-v3 spans pos-ref src meta)
                          (if (string-eq dir-name "property")
                            (parse-defn-meta-property-v3 spans pos-ref src meta)
                            (let [source-kind (source-metadata-form-kind-v3 dir-name)]
                              (if (> source-kind 0)
                                (if (= source-kind 15)
                                  (parse-defn-meta-evidence-v3 spans pos-ref src meta)
                                  (parse-defn-meta-source-pair-v3 spans pos-ref src meta source-kind))
                                (do
                                  (skip-directive-payload-v3 spans pos-ref)
                                  (parse-defn-metadata-loop-rooted-v3 spans pos-ref src meta))))))))))))]
            (do
              (root_pop)
              (root_pop)
              result)))))
    meta))

(defn parse-defn-metadata-loop-v3 [spans pos-ref src meta]
  (do
    (root_push spans)
    (root_push pos-ref)
    (root_push src)
    (let [result (parse-defn-metadata-loop-rooted-v3 spans pos-ref src meta)]
      (do
        (root_pop)
        (root_pop)
        (root_pop)
        result))))

;; :doc "string" — 文字列リテラルの内容を抽出
(defn parse-defn-meta-doc-v3 [spans pos-ref src meta]
  (if (== (p-current spans pos-ref) 12)
    (let [s (p-start spans pos-ref)
      e (p-end spans pos-ref)
      doc-text (substring src (+ s 1) (- e 1))
      updated (vector-set-at-rooted-v3 meta 0 doc-text)]
      (do
        (p-advance pos-ref)
        (parse-defn-metadata-loop-v3 spans pos-ref src updated)))
    (do
      (skip-directive-payload-v3 spans pos-ref)
      (parse-defn-metadata-loop-v3 spans pos-ref src meta))))

;; :example [...] — ブラケット内の式テキストを抽出
(defn append-defn-example-text-v3 [meta example-text]
  (let [existing (vector-get meta 1)]
    (if (> (string-length existing) 0)
      (string-concat existing (string-concat " " example-text))
      example-text)))

(defn parse-defn-meta-example-v3 [spans pos-ref src meta]
  (if (== (p-current spans pos-ref) 2)
    (let [directive-start (metadata-directive-start-v3 spans pos-ref)]
      (do
        (p-advance pos-ref)
        (if (== (p-current spans pos-ref) 3)
          (do (p-advance pos-ref) (parse-defn-metadata-loop-v3 spans pos-ref src meta))
          (let [content-start (p-start spans pos-ref)
            expression-spans (collect-example-expression-spans-v3
              spans
              (ref-get pos-ref)
              (/ (vector-length spans) 3))]
            (do
              (root_push expression-spans)
              (parse-skip-bracket-v3 spans pos-ref 1)
              (let [last-idx (- (ref-get pos-ref) 2)
                content-end (span-end spans last-idx)
                example-text (substring src content-start content-end)
                combined (append-defn-example-text-v3 meta example-text)
                updated (vector-set-at-rooted-v3 meta 1 combined)]
                (do
                  (root_push updated)
                  (let [with-form (append-defn-metadata-form-with-extra-v3
                      updated
                      1
                      example-text
                      directive-start
                      (metadata-directive-end-v3 spans pos-ref)
                      expression-spans)]
                    (do
                      (root_pop)
                      (root_pop)
                      (parse-defn-metadata-loop-v3 spans pos-ref src with-form))))))))))
    (do
      (skip-directive-payload-v3 spans pos-ref)
      (parse-defn-metadata-loop-v3 spans pos-ref src meta))))

;; :params [(x "left") ...] — [name-hash, doc-string] の vector を抽出
(defn make-defn-param-metadata-entry [name-hash doc-text]
  (vector-push-pair-rooted-v3 (vector-new 2) name-hash doc-text))

(defn parse-defn-meta-param-doc-v3 [spans pos-ref src]
  (if (== (p-current spans pos-ref) 12)
    (let [s (p-start spans pos-ref)
      e (p-end spans pos-ref)
      doc-text (substring src (+ s 1) (- e 1))]
      (do
        (p-advance pos-ref)
        doc-text))
    ""))

(defn parse-defn-meta-params-entry-v3 [spans pos-ref src params]
  (if (== (p-current spans pos-ref) 0)
    (do
      (p-advance pos-ref)
      (if (== (p-current spans pos-ref) 20)
        (let [s (p-start spans pos-ref)
          e (p-end spans pos-ref)
          param-hash (name-hash src s e)]
          (do
            (p-advance pos-ref)
            (let [doc-text (parse-defn-meta-param-doc-v3 spans pos-ref src)]
              (do
                (p-expect spans pos-ref 1)
                (vector-push params (make-defn-param-metadata-entry param-hash doc-text))))))
        (do
          (parse-skip-to-close-v3 spans pos-ref 1)
          params)))
    params))

(defn parse-defn-meta-params-step-v3 [spans pos-ref src params]
  (if (== (p-current spans pos-ref) 3)
    (do
      (p-advance pos-ref)
      (make-parse-loop-state 1 params))
    (if (== (p-current spans pos-ref) 0)
      (do
        (root_push params)
        (let [updated (parse-defn-meta-params-entry-v3 spans pos-ref src params)]
          (do
            (root_push updated)
            (let [state (make-parse-loop-state 0 updated)]
              (do
                (root_pop)
                (root_pop)
                state)))))
      (do
        (parse-skip-bracket-v3 spans pos-ref 1)
        (make-parse-loop-state 1 params)))))

(defn parse-defn-meta-params-step-64-loop-bounded [spans pos-ref src params remaining]
  (do
    (root_push params)
    (let [step (parse-defn-meta-params-step-v3 spans pos-ref src params)
      done (vector-get step 0)
      next-params (vector-get step 1)]
      (do
        (root_push step)
        (root_push next-params)
        (let [parsed
          (if (= done 1)
            step
            (if (<= remaining 1)
              step
              (parse-defn-meta-params-step-64-loop-bounded spans pos-ref src next-params (- remaining 1))))]
          (do
            (root_pop)
            (root_pop)
            (root_pop)
            parsed))))))

(defn parse-defn-meta-params-step-64 [spans pos-ref src params]
  (parse-defn-meta-params-step-64-loop-bounded spans pos-ref src params 64))

(defn parse-defn-meta-params-loop-rooted-v3 [spans pos-ref src params]
  (let [step (parse-defn-meta-params-step-64 spans pos-ref src params)]
    (if (= (vector-get step 0) 1)
      (vector-get step 1)
      (do
        (root_push step)
        (let [next-params (vector-get step 1)]
          (do
            (root_push next-params)
            (let [parsed (parse-defn-meta-params-loop-rooted-v3 spans pos-ref src next-params)]
              (do
                (root_pop)
                (root_pop)
                parsed))))))))

(defn parse-defn-meta-params-loop-v3 [spans pos-ref src params]
  (do
    (root_push spans)
    (root_push pos-ref)
    (root_push src)
    (let [result (parse-defn-meta-params-loop-rooted-v3 spans pos-ref src params)]
      (do
        (root_pop)
        (root_pop)
        (root_pop)
        result))))

(defn parse-defn-meta-params-v3 [spans pos-ref src meta]
  (if (== (p-current spans pos-ref) 2)
    (do
      (p-advance pos-ref)
      (let [params0 (vector-new 0)]
        (do
          (root_push params0)
          (let [params (parse-defn-meta-params-loop-v3 spans pos-ref src params0)
            updated (vector-set-at-rooted-v3 meta 2 params)]
            (do
              (root_pop)
              (parse-defn-metadata-loop-v3 spans pos-ref src updated))))))
    (do
      (skip-directive-payload-v3 spans pos-ref)
      (parse-defn-metadata-loop-v3 spans pos-ref src meta))))

;; :returns "sum" — 戻り値説明文字列を抽出
(defn parse-defn-meta-returns-v3 [spans pos-ref src meta]
  (if (== (p-current spans pos-ref) 12)
    (let [s (p-start spans pos-ref)
      e (p-end spans pos-ref)
      returns-text (substring src (+ s 1) (- e 1))
      updated (vector-set-at-rooted-v3 meta 3 returns-text)]
      (do
        (p-advance pos-ref)
        (parse-defn-metadata-loop-v3 spans pos-ref src updated)))
    (do
      (skip-directive-payload-v3 spans pos-ref)
      (parse-defn-metadata-loop-v3 spans pos-ref src meta))))

;; :invariant expr — 事後条件 AST を保持する
(defn parse-defn-meta-invariant-v3 [spans pos-ref src meta]
  (let [directive-start (metadata-directive-start-v3 spans pos-ref)
    expression-start (p-start spans pos-ref)
    predicate (parse-expr-v3 spans pos-ref src)
    directive-end (metadata-directive-end-v3 spans pos-ref)]
    (do
      (root_push predicate)
      (let [expression-span (vector-push-pair-rooted-v3
          (vector-new 0)
          expression-start
          directive-end)]
        (do
          (root_push expression-span)
          (let [updated (vector-set-at-rooted-v3 meta 4 predicate)]
            (do
              (root_push updated)
              (let [with-form (append-defn-metadata-form-with-extra-v3
                  updated
                  2
                  predicate
                  directive-start
                  directive-end
                  expression-span)]
                (do
                  (root_pop)
                  (root_pop)
                  (root_pop)
                  (parse-defn-metadata-loop-v3 spans pos-ref src with-form))))))))))

;; :case [(expect actual expected) ...] — actual / expected と個別 span を保持する。
(defn parse-defn-meta-case-expectation-v3 [spans pos-ref src]
  (if (== (p-current spans pos-ref) 0)
    (let [entry-start (p-start spans pos-ref)]
      (do
        (p-advance pos-ref)
        (if (== (p-current spans pos-ref) 20)
          (let [name (current-symbol-text-v3 spans pos-ref src)]
            (if (string-eq name "expect")
              (do
                (p-advance pos-ref)
                (let [actual-start (p-start spans pos-ref)
                  actual (parse-expr-v3 spans pos-ref src)
                  actual-end (previous-token-end-v3 spans pos-ref)]
                  (do
                    (root_push actual)
                    (let [expected-start (p-start spans pos-ref)
                      expected (parse-expr-v3 spans pos-ref src)
                      expected-end (previous-token-end-v3 spans pos-ref)]
                      (do
                        (root_push expected)
                        (let [entry-end (p-end spans pos-ref)]
                          (do
                            (p-expect spans pos-ref 1)
                            (let [pair0 (vector-push-quad-rooted-v3
                                (vector-new 4)
                                actual
                                expected
                                entry-start
                                entry-end)]
                              (do
                                (root_push pair0)
                                (let [pair (vector-push-quad-rooted-v3
                                    pair0
                                    actual-start
                                    actual-end
                                    expected-start
                                    expected-end)]
                                  (do
                                    (root_pop)
                                    (root_pop)
                                    (root_pop)
                                    pair)))))))))))
              (do
                (parse-skip-to-close-v3 spans pos-ref 1)
                (vector-new 0))))
          (do
            (parse-skip-to-close-v3 spans pos-ref 1)
            (vector-new 0)))))
    (vector-new 0)))

(defn parse-defn-meta-case-step-v3 [spans pos-ref src expectations]
  (if (== (p-current spans pos-ref) 3)
    (do
      (p-advance pos-ref)
      (make-parse-loop-state 1 expectations))
    (if (== (p-current spans pos-ref) 99)
      (make-parse-loop-state 1 expectations)
      (do
        (root_push expectations)
        (let [expectation (parse-defn-meta-case-expectation-v3 spans pos-ref src)]
          (do
            (root_push expectation)
            (let [next-expectations
              (vector-push-single-rooted-v3 expectations expectation)]
              (do
                (root_push next-expectations)
                (let [state (make-parse-loop-state 0 next-expectations)]
                  (do
                    (root_pop)
                    (root_pop)
                    (root_pop)
                    state))))))))))

(defn parse-defn-meta-case-step-64-loop-bounded
  [spans pos-ref src expectations remaining]
  (do
    (root_push expectations)
    (let [step (parse-defn-meta-case-step-v3 spans pos-ref src expectations)
      done (vector-get step 0)
      next-expectations (vector-get step 1)]
      (do
        (root_push step)
        (root_push next-expectations)
        (let [parsed
          (if (= done 1)
            step
            (if (<= remaining 1)
              step
              (parse-defn-meta-case-step-64-loop-bounded
                spans
                pos-ref
                src
                next-expectations
                (- remaining 1))))]
          (do
            (root_pop)
            (root_pop)
            (root_pop)
            parsed))))))

(defn parse-defn-meta-case-step-64 [spans pos-ref src expectations]
  (parse-defn-meta-case-step-64-loop-bounded spans pos-ref src expectations 64))

(defn parse-defn-meta-case-loop-rooted-v3 [spans pos-ref src expectations]
  (let [step (parse-defn-meta-case-step-64 spans pos-ref src expectations)]
    (if (= (vector-get step 0) 1)
      (vector-get step 1)
      (do
        (root_push step)
        (let [next-expectations (vector-get step 1)]
          (do
            (root_push next-expectations)
            (let [parsed (parse-defn-meta-case-loop-rooted-v3
              spans pos-ref src next-expectations)]
              (do
                (root_pop)
                (root_pop)
                parsed))))))))

(defn parse-defn-meta-case-loop-v3 [spans pos-ref src expectations]
  (do
    (root_push spans)
    (root_push pos-ref)
    (root_push src)
    (let [parsed (parse-defn-meta-case-loop-rooted-v3 spans pos-ref src expectations)]
      (do
        (root_pop)
        (root_pop)
        (root_pop)
        parsed))))

(defn parse-defn-meta-case-v3 [spans pos-ref src meta]
  (if (== (p-current spans pos-ref) 2)
    (let [directive-start (metadata-directive-start-v3 spans pos-ref)]
      (do
        (p-advance pos-ref)
        (let [expectations0 (vector-new 0)]
          (do
            (root_push expectations0)
            (let [expectations (parse-defn-meta-case-loop-v3
              spans pos-ref src expectations0)]
              (do
                (root_push expectations)
                (let [updated (append-defn-metadata-form-v3
                  meta
                  (contract-form-case)
                  expectations
                  directive-start
                  (metadata-directive-end-v3 spans pos-ref))]
                  (do
                    (root_pop)
                    (root_pop)
                    (parse-defn-metadata-loop-v3 spans pos-ref src updated)))))))))
    (do
      (skip-directive-payload-v3 spans pos-ref)
      (parse-defn-metadata-loop-v3 spans pos-ref src meta))))

;; :assert [predicate ...] — canonical Bool predicate vector を保持する。
(defn parse-defn-meta-assert-step-v3 [spans pos-ref src predicates]
  (if (== (p-current spans pos-ref) 3)
    (do
      (p-advance pos-ref)
      (make-parse-loop-state 1 predicates))
    (if (== (p-current spans pos-ref) 99)
      (make-parse-loop-state 1 predicates)
      (do
        (root_push predicates)
        (let [predicate (parse-expr-v3 spans pos-ref src)]
          (do
            (root_push predicate)
            (let [next-predicates (vector-push-single-rooted-v3 predicates predicate)]
              (do
                (root_push next-predicates)
                (let [state (make-parse-loop-state 0 next-predicates)]
                  (do
                    (root_pop)
                    (root_pop)
                    (root_pop)
                    state))))))))))

(defn parse-defn-meta-assert-step-64-loop-bounded [spans pos-ref src predicates remaining]
  (do
    (root_push predicates)
    (let [step (parse-defn-meta-assert-step-v3 spans pos-ref src predicates)
      done (vector-get step 0)
      next-predicates (vector-get step 1)]
      (do
        (root_push step)
        (root_push next-predicates)
        (let [parsed
          (if (= done 1)
            step
            (if (<= remaining 1)
              step
              (parse-defn-meta-assert-step-64-loop-bounded
                spans
                pos-ref
                src
                next-predicates
                (- remaining 1))))]
          (do
            (root_pop)
            (root_pop)
            (root_pop)
            parsed))))))

(defn parse-defn-meta-assert-step-64 [spans pos-ref src predicates]
  (parse-defn-meta-assert-step-64-loop-bounded spans pos-ref src predicates 64))

(defn parse-defn-meta-assert-loop-rooted-v3 [spans pos-ref src predicates]
  (let [step (parse-defn-meta-assert-step-64 spans pos-ref src predicates)]
    (if (= (vector-get step 0) 1)
      (vector-get step 1)
      (do
        (root_push step)
        (let [next-predicates (vector-get step 1)]
          (do
            (root_push next-predicates)
            (let [parsed (parse-defn-meta-assert-loop-rooted-v3
              spans pos-ref src next-predicates)]
              (do
                (root_pop)
                (root_pop)
                parsed))))))))

(defn parse-defn-meta-assert-loop-v3 [spans pos-ref src predicates]
  (do
    (root_push spans)
    (root_push pos-ref)
    (root_push src)
    (let [parsed (parse-defn-meta-assert-loop-rooted-v3 spans pos-ref src predicates)]
      (do
        (root_pop)
        (root_pop)
        (root_pop)
        parsed))))

(defn parse-defn-meta-assert-v3 [spans pos-ref src meta]
  (if (== (p-current spans pos-ref) 2)
    (let [directive-start (metadata-directive-start-v3 spans pos-ref)]
      (do
        (p-advance pos-ref)
        (let [predicates0 (vector-new 0)]
          (do
            (root_push predicates0)
            (let [expression-spans (collect-example-expression-spans-v3
                spans
                (ref-get pos-ref)
                (/ (vector-length spans) 3))]
              (do
                (root_push expression-spans)
                (let [predicates (parse-defn-meta-assert-loop-v3 spans pos-ref src predicates0)]
                  (do
                    (root_push predicates)
                    (let [updated (append-defn-metadata-form-with-extra-v3
                      meta
                      (contract-form-assert)
                      predicates
                      directive-start
                      (metadata-directive-end-v3 spans pos-ref)
                      expression-spans)]
                      (do
                        (root_pop)
                        (root_pop)
                        (root_pop)
                        (parse-defn-metadata-loop-v3 spans pos-ref src updated)))))))))))
    (do
      (skip-directive-payload-v3 spans pos-ref)
      (parse-defn-metadata-loop-v3 spans pos-ref src meta))))

;; canonical :property payload を bracket-aware に lossless 保存する。
;; typed binder / sampling への projection は後続の selfhost contract slice で行う。
(defn parse-defn-meta-property-v3 [spans pos-ref src meta]
  (if (== (p-current spans pos-ref) 2)
    (let [directive-start (metadata-directive-start-v3 spans pos-ref)]
      (do
        (p-advance pos-ref)
        (if (== (p-current spans pos-ref) 3)
          (do
            (p-advance pos-ref)
            (let [updated (append-defn-metadata-form-v3
              meta
              (contract-form-property)
              ""
              directive-start
              (metadata-directive-end-v3 spans pos-ref))]
              (parse-defn-metadata-loop-v3 spans pos-ref src updated)))
          (let [content-start (p-start spans pos-ref)]
            (do
              (parse-skip-bracket-v3 spans pos-ref 1)
              (let [last-idx (- (ref-get pos-ref) 2)
                content-end (span-end spans last-idx)
                property-text (substring src content-start content-end)
                updated (append-defn-metadata-form-v3
                  meta
                  (contract-form-property)
                  property-text
                  directive-start
                  (metadata-directive-end-v3 spans pos-ref))]
                (do
                  (root_push updated)
                  (let [result
                    (parse-defn-metadata-loop-v3 spans pos-ref src updated)]
                    (do
                      (root_pop)
                      result)))))))))
    (do
      (skip-directive-payload-v3 spans pos-ref)
      (parse-defn-metadata-loop-v3 spans pos-ref src meta))))

;; M2 source node/edge form: valid な2つの文字列を
;; [wire-id, text-or-endpoint] として保持する。typed graph 投影は後段の境界で行う。
(defn source-metadata-form-kind-v3 [name]
  (if (string-eq name "intent") 6
    (if (string-eq name "claim") 7
      (if (string-eq name "assumption") 8
        (if (string-eq name "open-question") 9
          (if (string-eq name "motivates") 10
            (if (string-eq name "constrained-by") 11
              (if (string-eq name "tested-by") 12
                (if (string-eq name "supports") 13
                (if (string-eq name "contradicts") 14
                  (if (string-eq name "evidence") 15 0)))))))))))

(defn parse-source-metadata-string-v3 [spans pos-ref src]
  (if (== (p-current spans pos-ref) 12)
    (let [start (p-start spans pos-ref)
      end (p-end spans pos-ref)
      value (substring src (+ start 1) (- end 1))]
      (do
        (p-advance pos-ref)
        value))
    ""))

(defn parse-source-metadata-pair-v3 [spans pos-ref src]
  (let [first (parse-source-metadata-string-v3 spans pos-ref src)
    first-root (root_push first)
    second (parse-source-metadata-string-v3 spans pos-ref src)
    second-root (root_push second)
    result (vector-push-pair-rooted-v3 (vector-new 2) first second)]
    (do
      (root_pop)
      (root_pop)
      result)))

(defn parse-source-evidence-int-v3 [spans pos-ref src]
  (if (== (p-current spans pos-ref) 10)
    (let [start (p-start spans pos-ref)
      end (p-end spans pos-ref)
      value (parse-int-from-str src start end 0)
      advanced (p-advance pos-ref)]
      value)
    -1))

(defn advance-if-token-v3 [spans pos-ref token]
  (if (== (p-current spans pos-ref) token)
    (do
      (p-advance pos-ref)
      0)
    0))

(defn parse-source-evidence-shrinks-step-v3 [spans pos-ref src values]
  (if (or (== (p-current spans pos-ref) 3) (== (p-current spans pos-ref) 99))
    (do
      (advance-if-token-v3 spans pos-ref 3)
      (make-parse-loop-state 1 values))
    (if (== (p-current spans pos-ref) 10)
      (do
        (root_push values)
        (let [value (parse-source-evidence-int-v3 spans pos-ref src)]
          (do
            (root_push value)
            (let [next-values (vector-push-single-rooted-v3 values value)]
              (do
                (root_push next-values)
                (let [state (make-parse-loop-state 0 next-values)]
                  (do
                    (root_pop)
                    (root_pop)
                    (root_pop)
                    state)))))))
      (make-parse-loop-state 1 values))))

(defn parse-source-evidence-shrinks-step-64-loop-bounded [spans pos-ref src values remaining]
  (do
    (root_push values)
    (let [step (parse-source-evidence-shrinks-step-v3 spans pos-ref src values)
      done (vector-get step 0)
      next-values (vector-get step 1)]
      (do
        (root_push step)
        (root_push next-values)
        (let [parsed
          (if (= done 1)
            step
            (if (<= remaining 1)
              step
              (parse-source-evidence-shrinks-step-64-loop-bounded
                spans
                pos-ref
                src
                next-values
                (- remaining 1))))]
          (do
            (root_pop)
            (root_pop)
            (root_pop)
            parsed))))))

(defn parse-source-evidence-shrinks-step-64 [spans pos-ref src values]
  (parse-source-evidence-shrinks-step-64-loop-bounded spans pos-ref src values 64))

(defn parse-source-evidence-shrinks-rooted-v3 [spans pos-ref src values]
  (let [step (parse-source-evidence-shrinks-step-64 spans pos-ref src values)]
    (if (= (vector-get step 0) 1)
      (vector-get step 1)
      (do
        (root_push step)
        (let [next-values (vector-get step 1)]
          (do
            (root_push next-values)
            (let [parsed (parse-source-evidence-shrinks-rooted-v3
              spans pos-ref src next-values)]
              (do
                (root_pop)
                (root_pop)
                parsed))))))))

(defn parse-source-evidence-shrinks-loop-v3 [spans pos-ref src values]
  (do
    (root_push spans)
    (root_push pos-ref)
    (root_push src)
    (let [parsed (parse-source-evidence-shrinks-rooted-v3 spans pos-ref src values)]
      (do
        (root_pop)
        (root_pop)
        (root_pop)
        parsed))))

(defn parse-source-evidence-shrinks-v3 [spans pos-ref src]
  (if (== (p-current spans pos-ref) 2)
    (do
      (p-advance pos-ref)
      (parse-source-evidence-shrinks-loop-v3 spans pos-ref src (vector-new 0)))
    (vector-new 0)))

(defn parse-source-evidence-coverage-step-v3 [spans pos-ref src values]
  (if (or (== (p-current spans pos-ref) 3) (== (p-current spans pos-ref) 99))
    (do
      (advance-if-token-v3 spans pos-ref 3)
      (make-parse-loop-state 1 values))
    (if (== (p-current spans pos-ref) 0)
      (do
        (p-advance pos-ref)
        (root_push values)
        (let [bucket (parse-source-metadata-string-v3 spans pos-ref src)]
          (do
            (root_push bucket)
            (let [count (parse-source-evidence-int-v3 spans pos-ref src)]
              (do
                (root_push count)
                (advance-if-token-v3 spans pos-ref 1)
                (let [entry (vector-push-pair-rooted-v3 (vector-new 0) bucket count)]
                  (do
                    (root_push entry)
                    (let [next-values (vector-push-single-rooted-v3 values entry)]
                      (do
                        (root_push next-values)
                        (let [state (make-parse-loop-state 0 next-values)]
                          (do
                            (root_pop)
                            (root_pop)
                            (root_pop)
                            (root_pop)
                            (root_pop)
                            state)))))))))))
      (make-parse-loop-state 1 values))))

(defn parse-source-evidence-coverage-step-64-loop-bounded
  [spans pos-ref src values remaining]
  (do
    (root_push values)
    (let [step (parse-source-evidence-coverage-step-v3 spans pos-ref src values)
      done (vector-get step 0)
      next-values (vector-get step 1)]
      (do
        (root_push step)
        (root_push next-values)
        (let [parsed
          (if (= done 1)
            step
            (if (<= remaining 1)
              step
              (parse-source-evidence-coverage-step-64-loop-bounded
                spans
                pos-ref
                src
                next-values
                (- remaining 1))))]
          (do
            (root_pop)
            (root_pop)
            (root_pop)
            parsed))))))

(defn parse-source-evidence-coverage-step-64 [spans pos-ref src values]
  (parse-source-evidence-coverage-step-64-loop-bounded spans pos-ref src values 64))

(defn parse-source-evidence-coverage-rooted-v3 [spans pos-ref src values]
  (let [step (parse-source-evidence-coverage-step-64 spans pos-ref src values)]
    (if (= (vector-get step 0) 1)
      (vector-get step 1)
      (do
        (root_push step)
        (let [next-values (vector-get step 1)]
          (do
            (root_push next-values)
            (let [parsed (parse-source-evidence-coverage-rooted-v3
              spans pos-ref src next-values)]
              (do
                (root_pop)
                (root_pop)
                parsed))))))))

(defn parse-source-evidence-coverage-loop-v3 [spans pos-ref src values]
  (do
    (root_push spans)
    (root_push pos-ref)
    (root_push src)
    (let [parsed (parse-source-evidence-coverage-rooted-v3 spans pos-ref src values)]
      (do
        (root_pop)
        (root_pop)
        (root_pop)
        parsed))))

(defn parse-source-evidence-coverage-v3 [spans pos-ref src]
  (if (== (p-current spans pos-ref) 2)
    (do
      (p-advance pos-ref)
      (parse-source-evidence-coverage-loop-v3 spans pos-ref src (vector-new 0)))
    (vector-new 0)))

(defn source-evidence-field-kind-v3 [name]
  (if (string-eq name "subject") 1
    (if (string-eq name "method") 2
      (if (string-eq name "outcome") 3
        (if (string-eq name "runner") 4
          (if (string-eq name "target") 5
            (if (string-eq name "source-commit") 6
              (if (string-eq name "artifact-digest") 7
                (if (string-eq name "cases") 8
                  (if (string-eq name "seed") 9
                    (if (string-eq name "generator") 10
                      (if (string-eq name "shrinks") 11
                        (if (string-eq name "coverage") 12
                          (if (string-eq name "producer") 13
                            (if (string-eq name "tool-version") 14
                              (if (string-eq name "timestamp") 15
                                (if (string-eq name "independence") 16 0)))))))))))))))))

(defn parse-source-evidence-int-field-v3 [spans pos-ref src payload field-kind]
  (do
    (root_push payload)
    (let [value (parse-source-evidence-int-v3 spans pos-ref src)
      updated (vector-set-at-rooted-v3 payload field-kind value)]
      (do
        (root_pop)
        (parse-source-evidence-fields-loop-v3 spans pos-ref src updated)))))

(defn parse-source-evidence-string-field-v3 [spans pos-ref src payload field-kind]
  (do
    (root_push payload)
    (let [value (parse-source-metadata-string-v3 spans pos-ref src)]
      (do
        (root_push value)
        (let [updated (vector-set-at-rooted-v3 payload field-kind value)]
          (do
            (root_pop)
            (root_pop)
            (parse-source-evidence-fields-loop-v3 spans pos-ref src updated)))))))

(defn parse-source-evidence-vector-field-v3 [spans pos-ref src payload field-kind]
  (do
    (root_push payload)
    (let [value (if (= field-kind 11)
      (parse-source-evidence-shrinks-v3 spans pos-ref src)
      (parse-source-evidence-coverage-v3 spans pos-ref src))]
      (do
        (root_push value)
        (let [updated (vector-set-at-rooted-v3 payload field-kind value)]
          (do
            (root_pop)
            (root_pop)
            (parse-source-evidence-fields-loop-v3 spans pos-ref src updated)))))))

(defn parse-source-evidence-fields-loop-v3 [spans pos-ref src payload]
  (if (== (p-current spans pos-ref) 50)
    (do
      (root_push payload)
      (let [field-name-idx (+ (ref-get pos-ref) 1)
        field-name (substring src (span-start spans field-name-idx) (span-end spans field-name-idx))
        field-kind (source-evidence-field-kind-v3 field-name)]
        (do
          (root_push field-name)
          (if (> field-kind 0)
            (do
              (p-advance pos-ref)
              (p-advance pos-ref)
              (root_pop)
              (root_pop)
              (if (or (= field-kind 8) (= field-kind 9))
                (parse-source-evidence-int-field-v3 spans pos-ref src payload field-kind)
                (if (or (= field-kind 11) (= field-kind 12))
                  (parse-source-evidence-vector-field-v3 spans pos-ref src payload field-kind)
                  (parse-source-evidence-string-field-v3 spans pos-ref src payload field-kind))))
            (do
              (root_pop)
              (root_pop)
              payload)))))
    payload))

(defn make-empty-source-evidence-payload-v3 [id]
  (do
    (root_push id)
    (let [shrinks (vector-new 0)
      coverage (vector-new 0)
      first (vector-push-quad-rooted-v3 (vector-new 0) id "" "" "")
      second (vector-push-quad-rooted-v3 first "" "" "" "")
      third (vector-push-quad-rooted-v3 second -1 -1 "" shrinks)
      fourth (vector-push-quad-rooted-v3 third coverage "" "" "")
      result (vector-push-single-rooted-v3 fourth "")]
      (do
        (root_pop)
        result))))

(defn parse-defn-meta-evidence-v3 [spans pos-ref src meta]
  (let [directive-start (metadata-directive-start-v3 spans pos-ref)
    id (parse-source-metadata-string-v3 spans pos-ref src)
    payload0 (make-empty-source-evidence-payload-v3 id)
    payload (parse-source-evidence-fields-loop-v3 spans pos-ref src
      (vector-set-at-rooted-v3 payload0 0 id))
    directive-end (metadata-directive-end-v3 spans pos-ref)]
    (do
      (root_push payload)
      (let [updated (append-defn-metadata-form-v3
          meta
          15
          payload
          directive-start
          directive-end)]
        (do
          (root_pop)
          (parse-defn-metadata-loop-v3 spans pos-ref src updated))))))

(defn parse-defn-meta-source-pair-v3 [spans pos-ref src meta form-kind]
  (let [directive-start (metadata-directive-start-v3 spans pos-ref)
    payload (parse-source-metadata-pair-v3 spans pos-ref src)
    directive-end (metadata-directive-end-v3 spans pos-ref)]
    (do
      (root_push payload)
      (let [updated (append-defn-metadata-form-v3
          meta
          form-kind
          payload
          directive-start
          directive-end)]
        (do
          (root_pop)
          (parse-defn-metadata-loop-v3 spans pos-ref src updated))))))

(defn defn-metadata-present-v3 [meta]
  (if (> (string-length (vector-get meta 0)) 0)
    1
    (if (> (string-length (vector-get meta 1)) 0)
      1
      (if (> (vector-length (vector-get meta 2)) 0)
        1
        (if (> (string-length (vector-get meta 3)) 0)
          1
          (if (= (vector-get meta 4) 0)
            (if (> (vector-length meta) 5)
              (if (> (vector-length (vector-get meta 5)) 0) 1 0)
              0)
            1))))))

(defn finalize-defn-body-v3 [body defn-node]
  (do
    (root_push body)
    (root_push defn-node)
    (let [parsed (vector-push-single-rooted-v3 defn-node body)]
      (do
        (root_pop)
        (root_pop)
        parsed))))

(defn defn-signature-param-present-v3 [signature idx count]
  (if (>= idx count)
    0
    (if (= (vector-get signature (+ idx 2)) 0)
      (defn-signature-param-present-v3 signature (+ idx 1) count)
      1)))

(defn defn-signature-present-v3 [signature]
  (let [param-count (vector-get signature 1)
    return-type (vector-get signature (+ param-count 2))]
    (if (= return-type 0)
      (defn-signature-param-present-v3 signature 0 param-count)
      1)))

(defn maybe-append-defn-signature-v3 [node signature]
  (if (= signature 0)
    node
    (if (= (defn-signature-present-v3 signature) 1)
      (vector-push-single-rooted-v3 node signature)
      node)))

(defn maybe-append-defn-meta-v3 [node meta]
  (if (= (defn-metadata-present-v3 meta) 1)
    (vector-push-single-rooted-v3 node meta)
    node))

(defn finalize-defn-parsed-body-v3 [spans pos-ref defn-node param-count body]
  (do
    (root_push body)
    (root_push defn-node)
    (p-expect spans pos-ref 1) ;; ) を消費
    (let [parsed (finalize-defn-body-v3 body defn-node)]
      (do
        (root_pop)
        (root_pop)
        parsed))))

(defn parse-defn-bodyless-or-body-v3 [spans pos-ref src defn-node param-count]
  (if (== (p-current spans pos-ref) 1)
    (do
      (p-advance pos-ref) ;; bodyless defn の ) を消費
      (let [bodyless-defn-body (make-int-node 0)]
        (finalize-defn-body-v3 bodyless-defn-body defn-node)))
    (do
      (root_push spans)
      (root_push pos-ref)
      (root_push src)
      (let [parsed-defn-body (parse-expr-v3 spans pos-ref src)]
        (do
          (root_push parsed-defn-body)
          (let [parsed (finalize-defn-body-v3 parsed-defn-body defn-node)]
            (do
              (root_push parsed)
              (p-expect spans pos-ref 1) ;; ) を消費
              (root_pop)
              (root_pop)
              (root_pop)
              (root_pop)
              (root_pop)
              parsed)))))))

(defn parse-defn-bodyless-or-body-with-meta-v3 [spans pos-ref src defn-node param-count signature meta]
  (maybe-append-defn-meta-v3
    (maybe-append-defn-signature-v3
      (parse-defn-bodyless-or-body-v3 spans pos-ref src defn-node param-count)
      signature)
    meta))

(defn skip-optional-type-sig-v3 [spans pos-ref src]
  (if (== (colon-directive-v3 spans pos-ref src) 1)
    0
    (if (== (p-current spans pos-ref) 50) ;; :
      (do
        (p-advance pos-ref)
        (parse-type-expr-v3 spans pos-ref src))
      0)))

(defn where-directive-v3 [spans pos-ref src]
  (if (== (p-current spans pos-ref) 50)
    (let [next-idx (+ (ref-get pos-ref) 1)
      next-kind (span-kind spans next-idx)]
      (if (== next-kind 42) 1
        (if (== next-kind 20)
          (if (string-eq
              (substring src (span-start spans next-idx) (span-end spans next-idx))
              "where")
            1
            0)
          0)))
    (if (== (p-current spans pos-ref) 42) 1
      (if (== (p-current spans pos-ref) 20)
        (if (string-eq (current-symbol-text-v3 spans pos-ref src) "where")
          1
          0)
        0))))

(defn skip-optional-where-v3 [spans pos-ref src]
  (if (== (where-directive-v3 spans pos-ref src) 1)
    (do
      (if (== (p-current spans pos-ref) 50)
        (do
          (p-advance pos-ref)
          0)
        0)
      (p-advance pos-ref)
      (if (== (p-current spans pos-ref) 2)
        (do
          (p-advance pos-ref)
          (parse-skip-bracket-v3 spans pos-ref 1))
        0))
    0))

(defn parse-ann-v3 [spans pos-ref src]
  (do
    (p-advance pos-ref)
    (let [expr (parse-expr-v3 spans pos-ref src)]
      (do
        (root_push expr)
        (let [type-expr (parse-type-expr-v3 spans pos-ref src)]
          (do
            (root_push type-expr)
            (p-expect spans pos-ref 1)
            (let [parsed (make-ann-typed expr type-expr)]
              (do
                (root_pop)
                (root_pop)
                parsed))))))))

(defn parse-fieldaccess-v3 [spans pos-ref src]
  (do
    (p-advance pos-ref) ;; . を消費
    (let [expr (parse-expr-v3 spans pos-ref src)]
      (do
        (root_push expr)
        (let [parsed
          (if (== (p-current spans pos-ref) 20)
            (let [field-h (current-symbol-hash-v3 spans pos-ref src)]
              (do
                (p-advance pos-ref)
                (p-expect spans pos-ref 1)
                (make-fieldaccess expr field-h)))
            (do
              (p-expect spans pos-ref 1)
              (make-fieldaccess expr 0)))]
          (do
            (root_pop)
            parsed))))))

(defn parse-type-head-hash-v3 [spans pos-ref src]
  (if (== (p-current spans pos-ref) 0)
    (do
      (p-advance pos-ref) ;; type head の ( を消費
      (if (== (p-current spans pos-ref) 20)
        (let [name-h (current-symbol-hash-v3 spans pos-ref src)]
          (do
            (p-advance pos-ref) ;; type 名を消費
            (parse-skip-to-close-v3 spans pos-ref 1)
            name-h))
        (do
          (parse-skip-to-close-v3 spans pos-ref 1)
          0)))
    (if (== (p-current spans pos-ref) 20)
      (let [name-h (current-symbol-hash-v3 spans pos-ref src)]
        (do
          (p-advance pos-ref) ;; type 名を消費
          name-h))
      0)))

;; parametric type-alias head の parameter 名を source order で保持する。
(defn parse-type-alias-param-hashes-step-v3 [spans pos-ref src params]
  (if (== (p-current spans pos-ref) 1)
    (do
      (p-advance pos-ref)
      (make-parse-loop-state 1 params))
    (if (== (p-current spans pos-ref) 20)
      (let [param-hash (current-symbol-hash-v3 spans pos-ref src)]
        (do
          (p-advance pos-ref)
          (root_push params)
          (let [next-params (vector-push-single-rooted-v3 params param-hash)]
            (do
              (root_push next-params)
              (let [state (make-parse-loop-state 0 next-params)]
                (do
                  (root_pop)
                  (root_pop)
                  state))))))
      (do
        (parse-skip-to-close-v3 spans pos-ref 1)
        (make-parse-loop-state 1 0)))))

(defn parse-type-alias-param-hashes-step-64-loop-bounded
  [spans pos-ref src params remaining]
  (do
    (root_push params)
    (let [step (parse-type-alias-param-hashes-step-v3 spans pos-ref src params)
      done (vector-get step 0)
      next-params (vector-get step 1)]
      (do
        (root_push step)
        (root_push next-params)
        (let [parsed
          (if (= done 1)
            step
            (if (<= remaining 1)
              step
              (parse-type-alias-param-hashes-step-64-loop-bounded
                spans
                pos-ref
                src
                next-params
                (- remaining 1))))]
          (do
            (root_pop)
            (root_pop)
            (root_pop)
            parsed))))))

(defn parse-type-alias-param-hashes-step-64 [spans pos-ref src params]
  (parse-type-alias-param-hashes-step-64-loop-bounded
    spans
    pos-ref
    src
    params
    64))

(defn parse-type-alias-param-hashes-rooted-v3 [spans pos-ref src params]
  (let [step (parse-type-alias-param-hashes-step-64 spans pos-ref src params)]
    (if (= (vector-get step 0) 1)
      (vector-get step 1)
      (do
        (root_push step)
        (let [next-params (vector-get step 1)]
          (do
            (root_push next-params)
            (let [parsed
                    (parse-type-alias-param-hashes-rooted-v3
                      spans
                      pos-ref
                      src
                      next-params)]
              (do
                (root_pop)
                (root_pop)
                parsed))))))))

(defn parse-type-alias-param-hashes-v3 [spans pos-ref src]
  (do
    (root_push spans)
    (root_push pos-ref)
    (root_push src)
    (let [params (parse-type-alias-param-hashes-rooted-v3 spans pos-ref src (vector-new 0))]
      (do
        (root_pop)
        (root_pop)
        (root_pop)
        params))))

;; type 宣言 head を [name-hash, parameter 名 vector] として保持する。
;; parameter がない head は空 vector を返し、既存の nonparametric AST shape を維持する。
(defn make-type-decl-head-v3 [name-hash params]
  (vector-push-pair-rooted-v3 (vector-new 2) name-hash params))

(defn parse-type-decl-head-v3 [spans pos-ref src]
  (if (== (p-current spans pos-ref) 0)
    (do
      (p-advance pos-ref) ;; type head の ( を消費
      (if (== (p-current spans pos-ref) 20)
        (let [name-hash (current-symbol-hash-v3 spans pos-ref src)]
          (do
            (p-advance pos-ref)
            (let [params (parse-type-alias-param-hashes-v3 spans pos-ref src)]
              (do
                (root_push params)
                (let [parsed (make-type-decl-head-v3 name-hash params)]
                  (do
                    (root_pop)
                    parsed))))))
        (do
          (parse-skip-to-close-v3 spans pos-ref 1)
          (make-type-decl-head-v3 0 (vector-new 0)))))
    (if (== (p-current spans pos-ref) 20)
      (let [name-hash (current-symbol-hash-v3 spans pos-ref src)]
        (do
          (p-advance pos-ref)
          (make-type-decl-head-v3 name-hash (vector-new 0))))
      (make-type-decl-head-v3 0 (vector-new 0)))))

(defn parse-type-alias-v3 [spans pos-ref src]
  (do
    (p-advance pos-ref) ;; type-alias を消費
    (if (== (p-current spans pos-ref) 0)
      (do
        (p-advance pos-ref) ;; alias head の ( を消費
        (if (== (p-current spans pos-ref) 20)
          (let [name-h (current-symbol-hash-v3 spans pos-ref src)]
            (do
              (p-advance pos-ref) ;; alias 名を消費
              (let [params (parse-type-alias-param-hashes-v3 spans pos-ref src)]
                (do
                  (root_push params)
                  (let [target-type-expr (parse-type-expr-v3 spans pos-ref src)]
                    (do
                      (root_push target-type-expr)
                      (p-expect spans pos-ref 1) ;; ) を消費
                      (let [parsed (make-type-alias-with-params name-h params target-type-expr)]
                        (do
                          (root_pop)
                          (root_pop)
                          parsed))))))))
          (do
            (parse-skip-to-close-v3 spans pos-ref 1)
            (parse-skip-to-close-v3 spans pos-ref 1)
            (make-type-alias 0 0))))
      (if (== (p-current spans pos-ref) 20)
        (let [name-h (current-symbol-hash-v3 spans pos-ref src)]
          (do
            (p-advance pos-ref) ;; alias 名を消費
            (let [target-type-expr (parse-type-expr-v3 spans pos-ref src)]
              (do
                (root_push target-type-expr)
                (p-expect spans pos-ref 1) ;; ) を消費
                (let [parsed (make-type-alias name-h target-type-expr)]
                  (do
                    (root_pop)
                    parsed))))))
        (do
          (parse-skip-to-close-v3 spans pos-ref 1)
          (make-type-alias 0 0))))))

(defn parse-type-constrained-v3 [spans pos-ref src]
  (do
    (p-advance pos-ref) ;; type-constrained を消費
    (if (== (p-current spans pos-ref) 20)
      (let [name-h (current-symbol-hash-v3 spans pos-ref src)]
        (do
          (p-advance pos-ref) ;; name を消費
          (parse-skip-to-close-v3 spans pos-ref 1)
          (make-type-constrained name-h)))
      (do
        (parse-skip-to-close-v3 spans pos-ref 1)
        (make-type-constrained 0)))))

(defn parse-computation-builder-return-v3 [spans pos-ref src name-h bind-h]
  (if (== (p-current spans pos-ref) 20)
    (let [return-h (current-symbol-hash-v3 spans pos-ref src)]
      (do
        (p-advance pos-ref)
        (p-expect spans pos-ref 1) ;; ) を消費
        (make-computation-builder name-h bind-h return-h)))
    (do
      (parse-skip-to-close-v3 spans pos-ref 1)
      (make-computation-builder name-h bind-h 0))))

(defn parse-computation-builder-bind-v3 [spans pos-ref src name-h]
  (if (== (p-current spans pos-ref) 20)
    (let [bind-h (current-symbol-hash-v3 spans pos-ref src)]
      (do
        (p-advance pos-ref)
        (parse-computation-builder-return-v3 spans pos-ref src name-h bind-h)))
    (do
      (parse-skip-to-close-v3 spans pos-ref 1)
      (make-computation-builder name-h 0 0))))

(defn parse-computation-builder-v3 [spans pos-ref src]
  (do
    (p-advance pos-ref) ;; computation-builder を消費
    (if (== (p-current spans pos-ref) 20)
      (let [name-h (current-symbol-hash-v3 spans pos-ref src)]
        (do
          (p-advance pos-ref)
          (parse-computation-builder-bind-v3 spans pos-ref src name-h)))
      (do
        (parse-skip-to-close-v3 spans pos-ref 1)
        (make-computation-builder 0 0 0)))))

(defn parse-impl-body-v3 [spans pos-ref src trait-h type-h]
  (let [with-body (parse-decl-body-v3 spans pos-ref src
      (make-impl-def trait-h type-h))]
    (vector-set-at-rooted-v3 with-body 3 (- (vector-length with-body) 4))))

(defn parse-impl-type-v3 [spans pos-ref src trait-h]
  (if (== (p-current spans pos-ref) 20)
    (let [type-h (current-symbol-hash-v3 spans pos-ref src)]
      (do
        (p-advance pos-ref) ;; type 名を消費
        (parse-skip-to-close-v3 spans pos-ref 1)
        (parse-impl-body-v3 spans pos-ref src trait-h type-h)))
    (do
      (parse-skip-to-close-v3 spans pos-ref 1)
      (parse-impl-body-v3 spans pos-ref src trait-h 0))))

(defn parse-impl-trait-v3 [spans pos-ref src]
  (if (== (p-current spans pos-ref) 20)
    (let [trait-h (current-symbol-hash-v3 spans pos-ref src)]
      (do
        (p-advance pos-ref) ;; trait 名を消費
        (parse-impl-type-v3 spans pos-ref src trait-h)))
    (do
      (parse-skip-to-close-v3 spans pos-ref 1)
      (parse-impl-body-v3 spans pos-ref src 0 0))))

(defn parse-impl-v3 [spans pos-ref src]
  (do
    (p-advance pos-ref) ;; impl を消費
    (if (== (p-current spans pos-ref) 0)
      (do
        (p-advance pos-ref) ;; impl head の ( を消費
        (parse-impl-trait-v3 spans pos-ref src))
      (do
        (parse-skip-to-close-v3 spans pos-ref 1)
        (make-impl-def 0 0)))))

(defn parse-symbol-form-v3 [spans pos-ref src]
  (let [name (current-symbol-text-v3 spans pos-ref src)]
    (do
      (root_push name)
      (let [parsed
        (if (string-eq name "type-alias")
          (parse-type-alias-v3 spans pos-ref src)
          (if (string-eq name "type-constrained")
            (parse-type-constrained-v3 spans pos-ref src)
            (if (string-eq name "computation-builder")
              (parse-computation-builder-v3 spans pos-ref src)
              (parse-apply-v3 spans pos-ref src))))]
        (do
          (root_pop)
          parsed)))))

(defn parse-computation-let-bang-step-v3 [spans pos-ref src]
  (do
    (p-advance pos-ref)
    (p-advance pos-ref)
    (if (== (p-current spans pos-ref) 20)
      (let [pat-hash (current-symbol-hash-v3 spans pos-ref src)]
        (do
          (p-advance pos-ref)
          (let [expr (parse-expr-v3 spans pos-ref src)]
            (do
              (root_push expr)
              (p-expect spans pos-ref 1)
              (let [parsed (make-computation-step-node (computation-step-let-bang) pat-hash expr)]
                (do
                  (root_pop)
                  parsed))))))
      (do
        (parse-skip-to-close-v3 spans pos-ref 1)
        (make-computation-step-node (computation-step-let-bang) 0 (make-int-node 0))))))

(defn parse-computation-do-bang-step-v3 [spans pos-ref src]
  (do
    (p-advance pos-ref)
    (p-advance pos-ref)
    (let [expr (parse-expr-v3 spans pos-ref src)]
      (do
        (root_push expr)
        (p-expect spans pos-ref 1)
        (let [parsed (make-computation-step-node (computation-step-do-bang) 0 expr)]
          (do
            (root_pop)
            parsed))))))

(defn parse-computation-return-step-v3 [spans pos-ref src]
  (do
    (p-advance pos-ref)
    (p-advance pos-ref)
    (let [expr (parse-expr-v3 spans pos-ref src)]
      (do
        (root_push expr)
        (p-expect spans pos-ref 1)
        (let [parsed (make-computation-step-node (computation-step-return) 0 expr)]
          (do
            (root_pop)
            parsed))))))

(defn parse-computation-step-v3 [spans pos-ref src]
  (if (== (p-current spans pos-ref) 0)
    (let [next-idx (+ (ref-get pos-ref) 1)
      next-kind (span-kind spans next-idx)]
      (if (== next-kind 20)
        (let [name (substring src (span-start spans next-idx) (span-end spans next-idx))]
          (if (string-eq name "let!")
            (parse-computation-let-bang-step-v3 spans pos-ref src)
            (if (string-eq name "do!")
              (parse-computation-do-bang-step-v3 spans pos-ref src)
              (if (string-eq name "return")
                (parse-computation-return-step-v3 spans pos-ref src)
                (let [expr (parse-expr-v3 spans pos-ref src)]
                  (make-computation-step-node (computation-step-expr) 0 expr))))))
        (let [expr (parse-expr-v3 spans pos-ref src)]
          (make-computation-step-node (computation-step-expr) 0 expr))))
    (let [expr (parse-expr-v3 spans pos-ref src)]
      (make-computation-step-node (computation-step-expr) 0 expr))))

(defn parse-computation-step-64-loop-bounded [spans pos-ref src result remaining]
  (if (== (p-current spans pos-ref) 1)
    (do
      (p-advance pos-ref)
      (make-parse-loop-state 1 result))
    (do
      (root_push result)
      (let [step (parse-computation-step-v3 spans pos-ref src)]
        (do
          (root_push step)
          (let [next-result
                  (computation-add-step
                    result
                    (vector-get step 0)
                    (vector-get step 1)
                    (vector-get step 2))]
            (do
              (root_push next-result)
              (let [parsed
                      (if (<= remaining 1)
                        (make-parse-loop-state 0 next-result)
                        (parse-computation-step-64-loop-bounded
                          spans
                          pos-ref
                          src
                          next-result
                          (- remaining 1)))]
                (do
                  (root_pop)
                  (root_pop)
                  (root_pop)
                  parsed)))))))))

(defn parse-computation-step-64 [spans pos-ref src result]
  (parse-computation-step-64-loop-bounded spans pos-ref src result 64))

(defn parse-computation-steps-rooted-v3 [spans pos-ref src result]
  (let [step (parse-computation-step-64 spans pos-ref src result)]
    (if (= (vector-get step 0) 1)
      (vector-get step 1)
      (do
        (root_push step)
        (let [next-result (vector-get step 1)]
          (do
            (root_push next-result)
            (let [parsed
                    (parse-computation-steps-rooted-v3
                      spans
                      pos-ref
                      src
                      next-result)]
              (do
                (root_pop)
                (root_pop)
                parsed))))))))

(defn parse-computation-steps-v3 [spans pos-ref src result]
  (do
    (root_push spans)
    (root_push pos-ref)
    (root_push src)
    (let [parsed (parse-computation-steps-rooted-v3 spans pos-ref src result)]
      (do
        (root_pop)
        (root_pop)
        (root_pop)
        parsed))))

(defn parse-computation-v3 [spans pos-ref src]
  (do
    (p-advance pos-ref)
    (if (== (p-current spans pos-ref) 20)
      (let [builder-hash (current-symbol-hash-v3 spans pos-ref src)]
        (let [result (make-computation-node builder-hash)]
          (do
            (root_push result)
            (p-advance pos-ref)
            (let [parsed (parse-computation-steps-v3 spans pos-ref src result)]
              (do
                (root_pop)
                parsed)))))
      (let [result (make-computation-node 0)]
        (do
          (root_push result)
          (let [parsed (parse-computation-steps-v3 spans pos-ref src result)]
            (do
              (root_pop)
              parsed)))))))

(defn parse-symbol-var-v3 [spans pos-ref src]
  (let [start (p-start spans pos-ref)
    end (p-end spans pos-ref)
    h (name-hash src start end)
    dot-pos (symbol-dot-position src start end)]
    (do
      (p-advance pos-ref)
      (if (>= dot-pos 0)
        (make-var-node-with-qualified-span
          h
          start
          end
          (name-hash src start dot-pos)
          (name-hash src (+ dot-pos 1) end))
        (make-var-node-with-span h start end)))))

(defn parse-string-node-v3 [spans pos-ref src]
  (let [start (p-start spans pos-ref)
    end (p-end spans pos-ref)]
    (do
      (p-advance pos-ref)
      (make-string-node
        (+ start 1)
        (- end 1)
        (string-literal-map-hash src (+ start 1) (- end 1))))))

;; 式のパース (メインディスパッチ)
(defn parse-expr-v3 [spans pos-ref src]
  (let [kind (p-current spans pos-ref)]
    (if (== kind 10) ;; Int
      (let [start (p-start spans pos-ref)
        end (p-end spans pos-ref)
        value (parse-int-from-str src start end 0)]
        (do (p-advance pos-ref)
          (make-int-node value)))
      (if (== kind 11) ;; Float
        (let [start (p-start spans pos-ref)
          end (p-end spans pos-ref)]
          (do (p-advance pos-ref)
            (make-float-node start end)))
        (if (== kind 13) ;; true
          (do (p-advance pos-ref) (make-bool-node 1))
          (if (== kind 14) ;; false
            (do (p-advance pos-ref) (make-bool-node 0))
            (if (== kind 12) ;; String
              (parse-string-node-v3 spans pos-ref src) ;; 引用符を除く
              (if (== kind 54) ;; '
                (parse-quote-v3 spans pos-ref src)
                (if (== kind 55) ;; ~
                  (parse-unquote-v3 spans pos-ref src)
                  (if (== kind 56) ;; ~@
                    (parse-unquote-splice-v3 spans pos-ref src)
                    (if (== kind 4) ;; LBrace -> record literal
                      (if (= (brace-starts-recordlit-v3 spans pos-ref src) 1)
                        (parse-recordlit-v3 spans pos-ref src)
                        (parse-recordupdate-v3 spans pos-ref src))
                      (if (== kind 20) ;; Symbol (変数参照)
                        (parse-symbol-var-v3 spans pos-ref src)
                        (if (== kind 0) ;; LParen -> S 式
                          (parse-sexp-v3 spans pos-ref src)
                          ;; unknown token
                          (do (p-advance pos-ref)
                            (make-int-node 0)))))))))))))))

;; S 式のパース (( の後のキーワードディスパッチ)
(defn parse-sexp-v3 [spans pos-ref src]
  (do
    (p-advance pos-ref) ;; ( を消費
    (let [kind (p-current spans pos-ref)]
      (if (== kind 1) ;; ()
        (do
          (p-advance pos-ref)
          (make-unit-node))
        (if (== kind 32) ;; if
          (parse-if-v3 spans pos-ref src)
          (if (== kind 31) ;; let
            (parse-let-v3 spans pos-ref src)
            (if (== kind 36) ;; do
              (parse-do-v3 spans pos-ref src)
              (if (== kind 33) ;; match
                (parse-match-v3 spans pos-ref src)
                (if (== kind 35) ;; fn (lambda)
                  (parse-lambda-v3 spans pos-ref src)
                  (if (== kind 30) ;; defn
                    (parse-defn-v3 spans pos-ref src)
                    (if (== kind 44) ;; defmacro
                      (parse-defmacro-v3 spans pos-ref src)
                      (if (== kind 43) ;; private
                        (parse-private-v3 spans pos-ref src)
                        (if (== kind 53) ;; .
                          (parse-fieldaccess-v3 spans pos-ref src)
                          (if (== kind 50) ;; :
                            (parse-ann-v3 spans pos-ref src)
                            (if (== kind 47) ;; computation
                              (parse-computation-v3 spans pos-ref src)
                              (if (== kind 34) ;; type
                                (parse-type-v3 spans pos-ref src)
                                (if (== kind 41) ;; impl
                                  (parse-impl-v3 spans pos-ref src)
                                  (if (== kind 40) ;; trait
                                    (parse-trait-v3 spans pos-ref src)
                                    (if (== kind 37) ;; module
                                      (parse-module-v3 spans pos-ref src)
                                      (if (== kind 38) ;; import
                                        (parse-import-v3 spans pos-ref src)
                                        (if (== kind 20) ;; symbol-form
                                          (parse-symbol-form-v3 spans pos-ref src)
                                          ;; 関数適用 (apply)
                                          (parse-apply-v3 spans pos-ref src))))))))))))))))))))))

;; === if 式 ===
(defn parse-if-v3 [spans pos-ref src]
  (do
    (p-advance pos-ref) ;; if を消費
    (let [cond-node (parse-expr-v3 spans pos-ref src)]
      (do
        (root_push cond-node)
        (let [then-node (parse-expr-v3 spans pos-ref src)]
          (do
            (root_push then-node)
            (let [else-node (parse-expr-v3 spans pos-ref src)]
              (do
                (root_push else-node)
                (let [result (vector-push-quad-rooted-v3 (vector-new 8) 6 cond-node then-node else-node)]
                  (do
                    (let [final-result (finish-parse-if-result-after-expect-v3 spans pos-ref result)]
                      (do
                        (root_pop)
                        (root_pop)
                        (root_pop)
                        final-result))))))))))))

(defn finish-parse-if-result-after-expect-v3 [spans pos-ref result]
  (do
    (root_push result)
    (let [result-ref (ref-new result)]
      (do
        (root_push result-ref)
        (p-expect spans pos-ref 1) ;; ) を消費
        (let [final-result (ref-get result-ref)]
          (do
            (root_pop)
            (root_pop)
            final-result))))))

;; === let 式 (複数バインディング対応) ===
(defn parse-let-body-starts-let-v3 [spans pos-ref]
  (if (== (p-current spans pos-ref) 0)
    (let [next-idx (+ (ref-get pos-ref) 1)]
      (if (>= (* next-idx 3) (vector-length spans))
        0
        (if (== (span-kind spans next-idx) 31) 1 0)))
    0))

(defn parse-let-body-v3 [spans pos-ref src]
  (if (= (parse-let-body-starts-let-v3 spans pos-ref) 1)
    (do
      (p-advance pos-ref) ;; nested let の ( を消費
      (parse-let-v3 spans pos-ref src))
    (parse-expr-v3 spans pos-ref src)))

(defn finish-parse-let-result-after-expect-v3 [spans pos-ref result]
  (do
    (root_push result)
    (let [result-ref (ref-new result)]
      (do
        (root_push result-ref)
        (p-expect spans pos-ref 1) ;; ) を消費
        (let [final-result (ref-get result-ref)]
          (do
            (root_pop)
            (root_pop)
            final-result))))))

(defn parse-let-after-first-binding-v3 [spans pos-ref src nh init]
  (do
    (let [init-slot (root_push init)]
      (do
        (let [rest-body (parse-let-rest-v3 spans pos-ref src)]
          (do
            (root_push rest-body)
            (let [result (vector-push-quad-rooted-v3 (vector-new 8) 7 nh init rest-body)]
              (do
                (let [final-result (finish-parse-let-result-after-expect-v3 spans pos-ref result)]
                  (do
                    (root_set init-slot final-result)
                    (root_pop)
                    (root_pop)
                    final-result))))))))))

(defn parse-let-v3 [spans pos-ref src]
  (do
    (p-advance pos-ref) ;; let を消費
    (p-expect spans pos-ref 2) ;; [ を消費
    (parse-let-first-binding-v3 spans pos-ref src)))

(defn parse-let-first-binding-v3 [spans pos-ref src]
  (do
    ;; 最初のバインディング
    (let [ns (p-start spans pos-ref)
      ne (p-end spans pos-ref)
      nh (name-hash src ns ne)]
      (do
        (p-advance pos-ref) ;; name を消費
        (let [init (parse-expr-v3 spans pos-ref src)]
          (do
            (let [init-slot (root_push init)]
              (let [parsed (parse-let-after-first-binding-v3 spans pos-ref src nh init)]
                (do
                  (root_push parsed)
                  (root_set init-slot parsed)
                  (root_pop)
                  (root_pop)
                  parsed)))))))))

;; let の残りバインディングを一要素ずつ収集する。
(defn parse-let-binding-step-v3 [spans pos-ref src bindings]
  (if (== (p-current spans pos-ref) 3)
    (do
      (p-advance pos-ref)
      (make-parse-loop-state 1 bindings))
    (if (== (p-current spans pos-ref) 99)
      (make-parse-loop-state 1 bindings)
      (if (== (p-current spans pos-ref) 1)
        (make-parse-loop-state 1 bindings)
        (let [binding-start (p-start spans pos-ref)
          binding-end (p-end spans pos-ref)
          binding-h (name-hash src binding-start binding-end)
          bindings-slot (root_push bindings)]
          (do
            (p-advance pos-ref)
            (let [init (parse-expr-v3 spans pos-ref src)]
              (do
                (root_push init)
                (let [next-bindings (vector-push-pair-rooted-v3 bindings binding-h init)]
                  (do
                    (root_set bindings-slot next-bindings)
                    (let [state (make-parse-loop-state 0 next-bindings)]
                      (do
                        (root_pop)
                        (root_pop)
                        state))))))))))))

(defn parse-let-binding-step-64-loop-bounded [spans pos-ref src bindings remaining]
  (do
    (root_push bindings)
    (let [step (parse-let-binding-step-v3 spans pos-ref src bindings)
      done (vector-get step 0)
      next-bindings (vector-get step 1)]
      (do
        (root_push step)
        (root_push next-bindings)
        (let [parsed
          (if (= done 1)
            step
            (if (<= remaining 1)
              step
              (parse-let-binding-step-64-loop-bounded spans pos-ref src next-bindings (- remaining 1))))]
          (do
            (root_pop)
            (root_pop)
            (root_pop)
            parsed))))))

(defn parse-let-binding-step-64 [spans pos-ref src bindings]
  (parse-let-binding-step-64-loop-bounded spans pos-ref src bindings 64))

(defn parse-let-bindings-rooted-v3 [spans pos-ref src bindings]
  (let [step (parse-let-binding-step-64 spans pos-ref src bindings)]
    (if (= (vector-get step 0) 1)
      (vector-get step 1)
      (do
        (root_push step)
        (let [next-bindings (vector-get step 1)]
          (do
            (root_push next-bindings)
            (let [parsed (parse-let-bindings-rooted-v3 spans pos-ref src next-bindings)]
              (do
                (root_pop)
                (root_pop)
                parsed))))))))

(defn parse-let-bindings-v3 [spans pos-ref src bindings]
  (do
    (root_push spans)
    (root_push pos-ref)
    (root_push src)
    (let [parsed (parse-let-bindings-rooted-v3 spans pos-ref src bindings)]
      (do
        (root_pop)
        (root_pop)
        (root_pop)
        parsed))))

(defn make-let-fold-state [done next-index result]
  (vector-push-triple-rooted-v3 (vector-new 3) done next-index result))

(defn parse-let-fold-step-v3 [bindings index result]
  (if (<= index 0)
    (make-let-fold-state 1 index result)
    (do
      (root_push bindings)
      (root_push result)
      (let [binding-index (- index 2)
        binding-h (vector-get bindings binding-index)
        init (vector-get bindings (+ binding-index 1))
        next-result (vector-push-quad-rooted-v3 (vector-new 8) 7 binding-h init result)]
        (do
          (root_push next-result)
          (let [state (make-let-fold-state 0 binding-index next-result)]
            (do
              (root_pop)
              (root_pop)
              (root_pop)
              state)))))))

(defn parse-let-fold-step-64-loop-bounded [bindings index result remaining]
  (do
    (root_push bindings)
    (root_push result)
    (let [step (parse-let-fold-step-v3 bindings index result)
      done (vector-get step 0)
      next-index (vector-get step 1)
      next-result (vector-get step 2)]
      (do
        (root_push step)
        (root_push next-result)
        (let [parsed
          (if (= done 1)
            step
            (if (<= remaining 1)
              step
              (parse-let-fold-step-64-loop-bounded bindings next-index next-result (- remaining 1))))]
          (do
            (root_pop)
            (root_pop)
            (root_pop)
            (root_pop)
            parsed))))))

(defn parse-let-fold-step-64 [bindings index result]
  (parse-let-fold-step-64-loop-bounded bindings index result 64))

(defn parse-let-fold-bindings-rooted-v3 [bindings index result]
  (let [step (parse-let-fold-step-64 bindings index result)]
    (if (= (vector-get step 0) 1)
      (vector-get step 2)
      (do
        (root_push step)
        (let [next-index (vector-get step 1)
          next-result (vector-get step 2)]
          (do
            (root_push next-result)
            (let [parsed (parse-let-fold-bindings-rooted-v3 bindings next-index next-result)]
              (do
                (root_pop)
                (root_pop)
                parsed))))))))

(defn parse-let-fold-bindings-v3 [bindings index result]
  (do
    (root_push bindings)
    (root_push result)
    (let [parsed (parse-let-fold-bindings-rooted-v3 bindings index result)]
      (do
        (root_pop)
        (root_pop)
        parsed))))

(defn parse-let-rest-rooted-v3 [spans pos-ref src]
  (if (== (p-current spans pos-ref) 3)
    (do
      (p-advance pos-ref)
      (parse-let-body-v3 spans pos-ref src))
    (if (== (p-current spans pos-ref) 99)
      (make-int-node 0)
      (if (== (p-current spans pos-ref) 1)
        (make-int-node 0)
        (let [bindings (vector-new 8)]
          (do
            (root_push bindings)
            (let [collected (parse-let-bindings-v3 spans pos-ref src bindings)]
              (do
                (root_push collected)
                (let [body (parse-let-body-v3 spans pos-ref src)]
                  (do
                    (root_push body)
                    (let [result (parse-let-fold-bindings-v3 collected (vector-length collected) body)]
                      (do
                        (root_pop)
                        (root_pop)
                        (root_pop)
                        result))))))))))))

(defn parse-let-rest-v3 [spans pos-ref src]
  (do
    (root_push spans)
    (root_push pos-ref)
    (root_push src)
    (let [parsed (parse-let-rest-rooted-v3 spans pos-ref src)]
      (do
        (root_pop)
        (root_pop)
        (root_pop)
        parsed))))

;; === do 式 ===
(defn parse-do-v3 [spans pos-ref src]
  (do
    (p-advance pos-ref) ;; do を消費
    (let [result (vector-push-pair-rooted-v3 (vector-new 16) 9 0) ;; [9, count=0(後で更新)]
      result-slot (root_push result)
      with-exprs (parse-do-exprs-v3 spans pos-ref src result 0)
      expr-count (- (vector-length with-exprs) 2)
      parsed (do
        (root_set result-slot with-exprs)
        (vector-set-at-rooted-v3 with-exprs 1 expr-count))]
      (do
        (root_pop)
        parsed))))

;; do 内の式を一つ処理する。
(defn parse-do-expr-step-v3 [spans pos-ref src result]
  (if (== (p-current spans pos-ref) 1) ;; ) で終了
    (do
      (p-advance pos-ref) ;; ) を消費
      (make-parse-loop-state 1 result))
    (if (== (p-current spans pos-ref) 99)
      (make-parse-loop-state 1 result)
      (do
        (let [result-slot (root_push result)
          expr (parse-expr-v3 spans pos-ref src)]
          (do
            (root_push expr)
            (let [next-result (vector-push result expr)
              state (do
                (root_set result-slot next-result)
                (make-parse-loop-state 0 next-result))]
              (do
                (root_pop)
                (root_pop)
                state))))))))

(defn parse-do-expr-step-64-loop-bounded [spans pos-ref src result remaining]
  (do
    (root_push result)
    (let [step (parse-do-expr-step-v3 spans pos-ref src result)
      done (vector-get step 0)
      next-result (vector-get step 1)]
      (do
        (root_push step)
        (root_push next-result)
        (let [parsed
          (if (= done 1)
            step
            (if (<= remaining 1)
              step
              (parse-do-expr-step-64-loop-bounded spans pos-ref src next-result (- remaining 1))))]
          (do
            (root_pop)
            (root_pop)
            (root_pop)
            parsed))))))

(defn parse-do-expr-step-64 [spans pos-ref src result]
  (parse-do-expr-step-64-loop-bounded spans pos-ref src result 64))

;; do 内の式を 64 個ずつ収集し、chunk 境界で handoff する。
(defn parse-do-exprs-rooted-v3 [spans pos-ref src result count]
  (let [step (parse-do-expr-step-64 spans pos-ref src result)]
    (if (= (vector-get step 0) 1)
      (vector-get step 1)
      (do
        (root_push step)
        (let [next-result (vector-get step 1)]
          (do
            (root_push next-result)
            (let [parsed (parse-do-exprs-rooted-v3 spans pos-ref src next-result count)]
              (do
                (root_pop)
                (root_pop)
                parsed))))))))

(defn parse-do-exprs-v3 [spans pos-ref src result count]
  (do
    (root_push spans)
    (root_push pos-ref)
    (root_push src)
    (let [parsed (parse-do-exprs-rooted-v3 spans pos-ref src result count)]
      (do
        (root_pop)
        (root_pop)
        (root_pop)
        parsed))))

;; === match 式 ===
(defn symbol-starts-uppercase-v3 [spans pos-ref src]
  (if (== (p-current spans pos-ref) 20)
    (let [c (string-char-at src (p-start spans pos-ref))]
      (if (>= c 65)
        (if (<= c 90) 1 0)
        0))
    0))

(defn parse-constructor-pattern-args-step-v3 [spans pos-ref src result]
  (if (== (p-current spans pos-ref) 1) ;; ) で終了
    (do
      (p-advance pos-ref)
      (make-parse-loop-state 1 result))
    (if (== (p-current spans pos-ref) 99)
      (make-parse-loop-state 1 result)
      (do
        (let [result-slot (root_push result)
          pat (parse-pattern-v3 spans pos-ref src)]
          (do
            (root_push pat)
            (let [next-result (vector-push result pat)
              state (do
                (root_set result-slot next-result)
                (make-parse-loop-state 0 next-result))]
              (do
                (root_pop)
                (root_pop)
                state))))))))

(defn parse-constructor-pattern-args-step-64-loop-bounded [spans pos-ref src result remaining]
  (do
    (root_push result)
    (let [step (parse-constructor-pattern-args-step-v3 spans pos-ref src result)
      done (vector-get step 0)
      next-result (vector-get step 1)]
      (do
        (root_push step)
        (root_push next-result)
        (let [parsed
          (if (= done 1)
            step
            (if (<= remaining 1)
              step
              (parse-constructor-pattern-args-step-64-loop-bounded
                spans pos-ref src next-result (- remaining 1))))]
          (do
            (root_pop)
            (root_pop)
            (root_pop)
            parsed))))))

(defn parse-constructor-pattern-args-step-64 [spans pos-ref src result]
  (parse-constructor-pattern-args-step-64-loop-bounded spans pos-ref src result 64))

(defn parse-constructor-pattern-args-rooted-v3 [spans pos-ref src result count]
  (let [step (parse-constructor-pattern-args-step-64 spans pos-ref src result)]
    (if (= (vector-get step 0) 1)
      (vector-get step 1)
      (do
        (root_push step)
        (let [next-result (vector-get step 1)]
          (do
            (root_push next-result)
            (let [parsed
              (parse-constructor-pattern-args-rooted-v3
                spans pos-ref src next-result (+ count 1))]
              (do
                (root_pop)
                (root_pop)
                parsed))))))))

(defn parse-constructor-pattern-args-v3 [spans pos-ref src result count]
  (do
    (root_push spans)
    (root_push pos-ref)
    (root_push src)
    (let [parsed (parse-constructor-pattern-args-rooted-v3 spans pos-ref src result count)]
      (do
        (root_pop)
        (root_pop)
        (root_pop)
        parsed))))

(defn parse-constructor-pattern-v3 [spans pos-ref src]
  (do
    (p-advance pos-ref) ;; ( を消費
    (if (== (p-current spans pos-ref) 20)
      (let [ctor-hash (current-symbol-hash-v3 spans pos-ref src)
        result (vector-push-triple-rooted-v3 (vector-new 8) (ast-pat-constructor) ctor-hash 0)]
        (do
          (root_push result)
          (p-advance pos-ref)
          (let [with-args (parse-constructor-pattern-args-v3 spans pos-ref src result 0)
            arg-count (- (vector-length with-args) 3)]
            (do
              (root_push with-args)
              (let [parsed (vector-set-at-rooted-v3 with-args 2 arg-count)]
                (do
                  (root_pop)
                  (root_pop)
                  parsed))))))
      (do
        (parse-skip-to-close-v3 spans pos-ref 1)
        (vector-push-triple-rooted-v3 (vector-new 3) (ast-pat-constructor) 0 0)))))

(defn parse-recordpat-fields-step-v3 [spans pos-ref src result]
  (if (== (p-current spans pos-ref) 5) ;; } で終了
    (do
      (p-advance pos-ref)
      (make-parse-loop-state 1 result))
    (if (== (p-current spans pos-ref) 99) ;; EOF ガード: 無限ループ防止
      (make-parse-loop-state 1 result)
      (if (== (p-current spans pos-ref) 20)
        (do
          (let [result-slot (root_push result)
            field-hash (current-symbol-hash-v3 spans pos-ref src)]
            (do
              (p-advance pos-ref)
              (let [pat (parse-pattern-v3 spans pos-ref src)]
                (do
                  (root_push pat)
                  (let [next-result (vector-push-pair-rooted-v3 result field-hash pat)
                    state (do
                      (root_set result-slot next-result)
                      (make-parse-loop-state 0 next-result))]
                    (do
                      (root_pop)
                      (root_pop)
                      state)))))))
        (do
          (p-advance pos-ref)
          (make-parse-loop-state 0 result))))))

(defn parse-recordpat-fields-step-64-loop-bounded [spans pos-ref src result remaining]
  (do
    (root_push result)
    (let [step (parse-recordpat-fields-step-v3 spans pos-ref src result)
      done (vector-get step 0)
      next-result (vector-get step 1)]
      (do
        (root_push step)
        (root_push next-result)
        (let [parsed
          (if (= done 1)
            step
            (if (<= remaining 1)
              step
              (parse-recordpat-fields-step-64-loop-bounded spans pos-ref src next-result (- remaining 1))))]
          (do
            (root_pop)
            (root_pop)
            (root_pop)
            parsed))))))

(defn parse-recordpat-fields-step-64 [spans pos-ref src result]
  (parse-recordpat-fields-step-64-loop-bounded spans pos-ref src result 64))

(defn parse-recordpat-fields-rooted-v3 [spans pos-ref src result count]
  (let [step (parse-recordpat-fields-step-64 spans pos-ref src result)]
    (if (= (vector-get step 0) 1)
      (vector-get step 1)
      (do
        (root_push step)
        (let [next-result (vector-get step 1)]
          (do
            (root_push next-result)
            (let [parsed (parse-recordpat-fields-rooted-v3 spans pos-ref src next-result count)]
              (do
                (root_pop)
                (root_pop)
                parsed))))))))

(defn parse-recordpat-fields-v3 [spans pos-ref src result count]
  (do
    (root_push spans)
    (root_push pos-ref)
    (root_push src)
    (let [parsed (parse-recordpat-fields-rooted-v3 spans pos-ref src result count)]
      (do
        (root_pop)
        (root_pop)
        (root_pop)
        parsed))))

(defn parse-recordpat-v3 [spans pos-ref src]
  (do
    (p-advance pos-ref) ;; { を消費
    (let [type-hash
          (if (== (p-current spans pos-ref) 20)
            (current-type-name-hash-v3 spans pos-ref src)
            0)
      raw-type-hash
        (if (== (p-current spans pos-ref) 20)
          (current-type-name-suffix-hash-v3 spans pos-ref src)
          0)
      result (vector-push-pair-rooted-v3 (vector-new 8) (ast-pat-recordpat) 0)]
      (do
        (if (== (p-current spans pos-ref) 20) (do (p-advance pos-ref) 0) 0)
        (root_push result)
        (let [with-fields (parse-recordpat-fields-v3 spans pos-ref src result 0)
          field-count (/ (- (vector-length with-fields) 2) 2)
          with-type-hash (vector-push-single-rooted-v3 with-fields type-hash)
          with-raw-type-hash (vector-push-single-rooted-v3 with-type-hash raw-type-hash)]
          (do
            (root_push with-raw-type-hash)
            (let [parsed (vector-set-at-rooted-v3 with-raw-type-hash 1 field-count)]
              (do
                (root_pop)
                (root_pop)
                parsed))))))))

(defn wrap-literal-pattern-v3 [expr]
  (do
    (root_push expr)
    (let [tag (vector-get expr 0)
      result
      (if (= tag (ast-lit-int))
        (vector-push-pair-rooted-v3 (vector-new 2) (ast-pat-lit) expr)
        (if (= tag (ast-lit-bool))
          (vector-push-pair-rooted-v3 (vector-new 2) (ast-pat-lit) expr)
          (if (= tag (ast-lit-unit))
            (vector-push-pair-rooted-v3 (vector-new 2) (ast-pat-lit) expr)
            expr)))]
      (do
        (root_pop)
        result))))

(defn sexp-starts-unit-pattern-v3 [spans pos-ref]
  (let [next-idx (+ (ref-get pos-ref) 1)]
    (if (== (span-kind spans next-idx) 1) 1 0)))

(defn parse-pattern-v3 [spans pos-ref src]
  (if (== (p-current spans pos-ref) 20)
    (let [name (current-symbol-text-v3 spans pos-ref src)
      name-hash (current-symbol-hash-v3 spans pos-ref src)]
      (if (string-eq name "_")
        (do
          (p-advance pos-ref)
          (vector-push-single-rooted-v3 (vector-new 1) (ast-pat-wildcard)))
        (if (= (symbol-starts-uppercase-v3 spans pos-ref src) 1)
          (do
            (p-advance pos-ref)
            (vector-push-triple-rooted-v3 (vector-new 3) (ast-pat-constructor) name-hash 0))
          (do
            (p-advance pos-ref)
            (vector-push-pair-rooted-v3 (vector-new 2) (ast-pat-var) name-hash)))))
    (if (== (p-current spans pos-ref) 0)
      (if (= (sexp-starts-unit-pattern-v3 spans pos-ref) 1)
        (wrap-literal-pattern-v3 (parse-expr-v3 spans pos-ref src))
        (parse-constructor-pattern-v3 spans pos-ref src))
      (if (== (p-current spans pos-ref) 4)
        (parse-recordpat-v3 spans pos-ref src)
        (wrap-literal-pattern-v3 (parse-expr-v3 spans pos-ref src))))))

(defn parse-match-v3 [spans pos-ref src]
  (do
    (p-advance pos-ref) ;; match を消費
    (let [scrutinee (parse-expr-v3 spans pos-ref src)]
      (do
        (root_push scrutinee)
        (let [result (vector-push-triple-rooted-v3 (vector-new 16) 10 scrutinee 0) ;; [10, scrutinee, arm-count=0]
          result-slot (root_push result)
          with-arms (parse-match-arms-v3 spans pos-ref src result 0)
          arm-count (/ (- (vector-length with-arms) 3) 2)
          parsed (do
            (root_set result-slot with-arms)
            (vector-set-at-rooted-v3 with-arms 2 arm-count))]
          (do
            (root_pop)
            (root_pop)
            parsed))))))

;; match arm の `when` guard は既存の [pattern, body] 配置を保ったまま body wrapper にする。
(defn token-is-when-v3 [spans pos-ref src]
  (if (= (p-current spans pos-ref) 20)
    (if (string-eq (current-symbol-text-v3 spans pos-ref src) "when") 1 0)
    0))

(defn parse-match-arm-body-v3 [spans pos-ref src]
  (if (= (token-is-when-v3 spans pos-ref src) 1)
    (do
      (p-advance pos-ref)
      (let [guard (parse-expr-v3 spans pos-ref src)
        body (parse-expr-v3 spans pos-ref src)]
        (make-match-guard guard body)))
    (parse-expr-v3 spans pos-ref src)))

;; match の腕を一つ処理する。
(defn parse-match-arm-step-v3 [spans pos-ref src result]
  (if (== (p-current spans pos-ref) 1) ;; ) で終了
    (do
      (p-advance pos-ref)
      (make-parse-loop-state 1 result))
    (if (== (p-current spans pos-ref) 99) ;; EOF ガード: 無限ループ防止
      (make-parse-loop-state 1 result)
      (if (== (p-current spans pos-ref) 2) ;; [ -> arm
        (do
          (let [result-slot (root_push result)]
            (do
              (p-advance pos-ref) ;; [ を消費
              (let [pat (parse-pattern-v3 spans pos-ref src)
                body (parse-match-arm-body-v3 spans pos-ref src)]
                (do
                  (root_push pat)
                  (root_push body)
                  (p-expect spans pos-ref 3) ;; ] を消費
                  (let [next-result (vector-push-pair-rooted-v3 result pat body)
                    state (do
                      (root_set result-slot next-result)
                      (make-parse-loop-state 0 next-result))]
                    (do
                      (root_pop)
                      (root_pop)
                      (root_pop)
                      state)))))))
        ;; 不正なトークン -> 一つ進めて継続
        (do
          (p-advance pos-ref)
          (make-parse-loop-state 0 result))))))

(defn parse-match-arm-step-64-loop-bounded [spans pos-ref src result remaining]
  (do
    (root_push result)
    (let [step (parse-match-arm-step-v3 spans pos-ref src result)
      done (vector-get step 0)
      next-result (vector-get step 1)]
      (do
        (root_push step)
        (root_push next-result)
        (let [parsed
          (if (= done 1)
            step
            (if (<= remaining 1)
              step
              (parse-match-arm-step-64-loop-bounded
                spans pos-ref src next-result (- remaining 1))))]
          (do
            (root_pop)
            (root_pop)
            (root_pop)
            parsed))))))

(defn parse-match-arm-step-64 [spans pos-ref src result]
  (parse-match-arm-step-64-loop-bounded spans pos-ref src result 64))

;; match の腕を 64 個ずつ収集し、chunk 境界で handoff する。
(defn parse-match-arms-rooted-v3 [spans pos-ref src result count]
  (let [step (parse-match-arm-step-64 spans pos-ref src result)]
    (if (= (vector-get step 0) 1)
      (vector-get step 1)
      (do
        (root_push step)
        (let [next-result (vector-get step 1)]
          (do
            (root_push next-result)
            (let [parsed
              (parse-match-arms-rooted-v3
                spans pos-ref src next-result (+ count 1))]
              (do
                (root_pop)
                (root_pop)
                parsed))))))))

(defn parse-match-arms-v3 [spans pos-ref src result count]
  (do
    (root_push spans)
    (root_push pos-ref)
    (root_push src)
    (let [parsed (parse-match-arms-rooted-v3 spans pos-ref src result count)]
      (do
        (root_pop)
        (root_pop)
        (root_pop)
        parsed))))

;; === lambda (fn) 式 ===
(defn parse-lambda-v3 [spans pos-ref src]
  (do
    (p-advance pos-ref) ;; fn を消費
    (p-expect spans pos-ref 2) ;; [ を消費
    (let [result (vector-push-pair-rooted-v3 (vector-new 8) 8 0)] ;; [8, param-count=0]
      (do
        (let [result-slot (root_push result)
          with-params (parse-params-v3 spans pos-ref src result 0)]
          (do
            (root_push with-params)
            (let [param-count (- (vector-length with-params) 2)
              lambda-node (vector-set-at-rooted-v3 with-params 1 param-count)
              body (do
                (root_set result-slot lambda-node)
                (parse-expr-v3 spans pos-ref src))]
              (do
                (root_push body)
                (p-expect spans pos-ref 1) ;; ) を消費
                (let [parsed (vector-push lambda-node body)]
                  (do
                    (root_pop)
                    (root_pop)
                    (root_pop)
                    parsed))))))))))

;; パラメータリストを収集 (名前ハッシュ)
(defn parse-param-hash-v3 [spans pos-ref src]
  (if (== (p-current spans pos-ref) 0)
    (do
      (p-advance pos-ref)
      (if (== (p-current spans pos-ref) 50)
        (do
          (p-advance pos-ref)
          (if (== (p-current spans pos-ref) 20)
            (let [param-h (current-symbol-hash-v3 spans pos-ref src)]
              (do
                (p-advance pos-ref)
                (skip-type-expr-v3 spans pos-ref)
                (p-expect spans pos-ref 1)
                param-h))
            (do
              (parse-skip-to-close-v3 spans pos-ref 1)
              0)))
        (do
          (parse-skip-to-close-v3 spans pos-ref 1)
          0)))
    (let [s (p-start spans pos-ref)
      e (p-end spans pos-ref)
      h (name-hash src s e)]
      (do
        (p-advance pos-ref)
        h))))

(defn parse-params-step-v3 [spans pos-ref src result]
  (if (== (p-current spans pos-ref) 3) ;; ] で終了
    (do
      (p-advance pos-ref)
      (make-parse-loop-state 1 result))
    (if (== (p-current spans pos-ref) 99) ;; EOF ガード: 無限ループ防止
      (make-parse-loop-state 1 result)
      (do
        (root_push result)
        (let [h (parse-param-hash-v3 spans pos-ref src)]
          (do
            (root_push h)
            (let [next-result (vector-push result h)
              state (do
                (root_push next-result)
                (make-parse-loop-state 0 next-result))]
              (do
                (root_pop)
                (root_pop)
                (root_pop)
                state))))))))

(defn parse-params-step-64-loop-bounded [spans pos-ref src result remaining]
  (do
    (root_push result)
    (let [step (parse-params-step-v3 spans pos-ref src result)
      done (vector-get step 0)
      next-result (vector-get step 1)]
      (do
        (root_push step)
        (root_push next-result)
        (let [parsed
          (if (= done 1)
            step
            (if (<= remaining 1)
              step
              (parse-params-step-64-loop-bounded
                spans pos-ref src next-result (- remaining 1))))]
          (do
            (root_pop)
            (root_pop)
            (root_pop)
            parsed))))))

(defn parse-params-step-64 [spans pos-ref src result]
  (parse-params-step-64-loop-bounded spans pos-ref src result 64))

(defn parse-params-rooted-v3 [spans pos-ref src result count]
  (let [step (parse-params-step-64 spans pos-ref src result)]
    (if (= (vector-get step 0) 1)
      (vector-get step 1)
      (do
        (root_push step)
        (let [next-result (vector-get step 1)]
          (do
            (root_push next-result)
            (let [parsed
              (parse-params-rooted-v3
                spans pos-ref src next-result (+ count 1))]
              (do
                (root_pop)
                (root_pop)
                parsed))))))))

(defn parse-params-v3 [spans pos-ref src result count]
  (do
    (root_push spans)
    (root_push pos-ref)
    (root_push src)
    (let [parsed (parse-params-rooted-v3 spans pos-ref src result count)]
      (do
        (root_pop)
        (root_pop)
        (root_pop)
        parsed))))

(defn make-scan-defn-param-form-end-state [done next-idx next-depth]
  (vector-push-triple-rooted-v3 (vector-new 3) done next-idx next-depth))

(defn scan-defn-param-form-end-step-v3 [spans idx end depth]
  (if (>= idx end)
    (make-scan-defn-param-form-end-state 1 idx depth)
    (let [kind (span-kind spans idx)]
      (if (== kind 0)
        (make-scan-defn-param-form-end-state 0 (+ idx 1) (+ depth 1))
        (if (== kind 1)
          (if (= depth 1)
            (make-scan-defn-param-form-end-state 1 (+ idx 1) depth)
            (make-scan-defn-param-form-end-state 0 (+ idx 1) (- depth 1)))
          (make-scan-defn-param-form-end-state 0 (+ idx 1) depth))))))

(defn scan-defn-param-form-end-step-64-loop-bounded
  [spans idx end depth remaining]
  (do
    (root_push spans)
    (let [step (scan-defn-param-form-end-step-v3 spans idx end depth)
      done (vector-get step 0)
      next-idx (vector-get step 1)
      next-depth (vector-get step 2)]
      (do
        (root_push step)
        (let [parsed
          (if (= done 1)
            step
            (if (<= remaining 1)
              step
              (scan-defn-param-form-end-step-64-loop-bounded
                spans
                next-idx
                end
                next-depth
                (- remaining 1))))]
          (do
            (root_pop)
            (root_pop)
            parsed))))))

(defn scan-defn-param-form-end-step-64 [spans idx end depth]
  (scan-defn-param-form-end-step-64-loop-bounded spans idx end depth 64))

(defn scan-defn-param-form-end-rooted-v3 [spans idx end depth]
  (let [step (scan-defn-param-form-end-step-64 spans idx end depth)]
    (if (= (vector-get step 0) 1)
      (vector-get step 1)
      (do
        (root_push step)
        (let [parsed
          (scan-defn-param-form-end-rooted-v3
            spans
            (vector-get step 1)
            end
            (vector-get step 2))]
          (do
            (root_pop)
            parsed))))))

(defn scan-defn-param-form-end-v3 [spans idx end depth]
  (do
    (root_push spans)
    (let [parsed (scan-defn-param-form-end-rooted-v3 spans idx end depth)]
      (do
        (root_pop)
        parsed))))

(defn collect-example-expression-spans-step-v3 [spans idx end result]
  (if (>= idx end)
    (vector-push-triple-rooted-v3 (vector-new 3) 1 idx result)
    (if (== (span-kind spans idx) 3)
      (vector-push-triple-rooted-v3 (vector-new 3) 1 idx result)
      (let [kind (span-kind spans idx)
        next-idx (if (== kind 0)
          (scan-defn-param-form-end-v3 spans (+ idx 1) end 1)
          (+ idx 1))
        last-idx (- next-idx 1)
        expression-start (span-start spans idx)
        expression-end (span-end spans last-idx)]
        (do
          (root_push result)
          (let [next-result (vector-push-pair-rooted-v3 result expression-start expression-end)]
            (do
              (root_push next-result)
              (let [state (vector-push-triple-rooted-v3
                  (vector-new 3)
                  0
                  next-idx
                  next-result)]
                (do
                  (root_pop)
                  (root_pop)
                  state)))))))))

(defn collect-example-expression-spans-step-64-loop-bounded
  [spans idx end result remaining]
  (do
    (root_push result)
    (let [step (collect-example-expression-spans-step-v3 spans idx end result)
      done (vector-get step 0)
      next-idx (vector-get step 1)
      next-result (vector-get step 2)]
      (do
        (root_push step)
        (root_push next-result)
        (let [parsed
          (if (= done 1)
            step
            (if (<= remaining 1)
              step
              (collect-example-expression-spans-step-64-loop-bounded
                spans
                next-idx
                end
                next-result
                (- remaining 1))))]
          (do
            (root_pop)
            (root_pop)
            (root_pop)
            parsed))))))

(defn collect-example-expression-spans-step-64 [spans idx end result]
  (collect-example-expression-spans-step-64-loop-bounded spans idx end result 64))

(defn collect-example-expression-spans-rooted-v3 [spans idx end result]
  (let [step (collect-example-expression-spans-step-64 spans idx end result)]
    (if (= (vector-get step 0) 1)
      (vector-get step 2)
      (do
        (root_push step)
        (let [next-idx (vector-get step 1)
          next-result (vector-get step 2)]
          (do
            (root_push next-result)
            (let [parsed (collect-example-expression-spans-rooted-v3
              spans
              next-idx
              end
              next-result)]
              (do
                (root_pop)
                (root_pop)
                parsed))))))))

(defn collect-example-expression-spans-v3 [spans idx end]
  (do
    (root_push spans)
    (let [result (vector-new 0)]
      (do
        (root_push result)
        (let [parsed (collect-example-expression-spans-rooted-v3 spans idx end result)]
          (do
            (root_pop)
            (root_pop)
            parsed))))))

(defn type-expr-invalid-v3 [type-expr]
  (if (= type-expr 0)
    1
    (if (= (vector-get type-expr 0) (ast-type-named))
      (if (= (vector-get type-expr 1) 0) 1 0)
      0)))

;; param span 内だけを読むための一時カーソル。型式の末尾が param の閉じ ) の直前であることも検証する。
(defn parse-type-expr-from-span-v3 [spans start end src]
  (let [type-pos (ref-new start)
    type-pos-slot (root_push type-pos)]
    (let [parsed (parse-type-expr-v3 spans type-pos src)]
      (if (= (ref-get type-pos) end)
        (if (= (type-expr-invalid-v3 parsed) 1)
          (do
            (root_pop)
            0)
          (do
            (root_pop)
            parsed))
        (do
          (root_pop)
          0)))))

(defn make-defn-param-signature-state-v3 [done next-idx signature]
  (do
    (root_push signature)
    (let [state0 (vector-push (vector-new 4) done)]
      (do
        (root_push state0)
        (let [state1 (vector-push state0 next-idx)]
          (do
            (root_push state1)
            (let [state (vector-push state1 signature)]
              (do
                (root_pop)
                (root_pop)
                (root_pop)
                state))))))))

(defn parse-defn-param-signature-step-state-v3 [signature type-expr next-idx]
  (do
    (root_push signature)
    (root_push type-expr)
    (let [next-signature (vector-push signature type-expr)]
      (do
        (root_push next-signature)
        (let [state (make-defn-param-signature-state-v3
          0 next-idx next-signature)]
          (do
            (root_pop)
            (root_pop)
            (root_pop)
            state))))))

;; parse-params-v3 のカーソル進行を維持したまま、typed parameter の raw type を span から再読する。
(defn parse-defn-param-signature-step-v3 [spans idx end src signature]
  (if (>= idx end)
    (do
      (let [state (make-defn-param-signature-state-v3 1 idx signature)]
        (do
          state)))
    (let [kind (span-kind spans idx)]
      (if (== kind 0)
        (if (< (+ idx 2) end)
          (if (== (span-kind spans (+ idx 1)) 50)
            (if (== (span-kind spans (+ idx 2)) 20)
              (let [next-idx (scan-defn-param-form-end-v3 spans (+ idx 1) end 1)]
                (if (< (+ idx 3) next-idx)
                  (let [type-expr (parse-type-expr-from-span-v3
                    spans (+ idx 3) (- next-idx 1) src)]
                    (parse-defn-param-signature-step-state-v3
                      signature type-expr next-idx))
                  (parse-defn-param-signature-step-state-v3
                    signature 0 next-idx)))
              (parse-defn-param-signature-step-state-v3
                signature 0 (scan-defn-param-form-end-v3 spans (+ idx 1) end 1)))
            (parse-defn-param-signature-step-state-v3
              signature 0 (scan-defn-param-form-end-v3 spans (+ idx 1) end 1)))
          (parse-defn-param-signature-step-state-v3
            signature 0 (scan-defn-param-form-end-v3 spans (+ idx 1) end 1)))
        (if (== kind 20)
          (parse-defn-param-signature-step-state-v3 signature 0 (+ idx 1))
          (make-defn-param-signature-state-v3 0 (+ idx 1) signature))))))

(defn parse-defn-param-signature-step-64-loop-bounded
  [spans idx end src signature remaining]
  (do
    (root_push signature)
    (let [step (parse-defn-param-signature-step-v3 spans idx end src signature)
      done (vector-get step 0)
      next-idx (vector-get step 1)
      next-signature (vector-get step 2)]
      (do
        (root_push step)
        (root_push next-signature)
        (let [parsed
          (if (= done 1)
            step
            (if (<= remaining 1)
              step
              (parse-defn-param-signature-step-64-loop-bounded
                spans next-idx end src next-signature (- remaining 1))))]
          (do
            (root_pop)
            (root_pop)
            (root_pop)
            parsed))))))

(defn parse-defn-param-signature-step-64 [spans idx end src signature]
  (parse-defn-param-signature-step-64-loop-bounded
    spans idx end src signature 64))

(defn parse-defn-param-signature-loop-v3 [spans idx end src signature]
  (let [step (parse-defn-param-signature-step-64 spans idx end src signature)]
    (if (= (vector-get step 0) 1)
      (vector-get step 2)
      (do
        (root_push step)
        (let [next-idx (vector-get step 1)
          next-signature (vector-get step 2)]
          (do
            (root_push next-signature)
            (let [parsed
              (parse-defn-param-signature-loop-v3
                spans next-idx end src next-signature)]
              (do
                (root_pop)
                (root_pop)
                parsed))))))))

(defn parse-defn-param-signature-v3 [spans start end src param-count]
  (let [signature (make-defn-signature param-count)]
    (do
      (root_push signature)
      (let [parsed (parse-defn-param-signature-loop-v3 spans start end src signature)]
        (do
          (root_pop)
          parsed)))))

;; === defn 式 ===
(defn parse-defn-tail-v3 [spans pos-ref src defn-node param-count]
  (do
    (root_push defn-node)
    (skip-optional-type-sig-v3 spans pos-ref src)
    (skip-optional-where-v3 spans pos-ref src)
    (let [parsed
      (if (== (colon-directive-v3 spans pos-ref src) 1)
        (let [meta (parse-defn-metadata-v3 spans pos-ref src)]
          (parse-defn-bodyless-or-body-with-meta-v3
            spans pos-ref src defn-node param-count 0 meta))
        (parse-defn-bodyless-or-body-v3
          spans pos-ref src defn-node param-count))]
      (do
        (root_pop)
        parsed))))

(defn parse-defn-v3 [spans pos-ref src]
  (do
    (root_push spans)
    (root_push pos-ref)
    (root_push src)
    (p-advance pos-ref) ;; defn を消費
    (let [ns (p-start spans pos-ref)
      ne (p-end spans pos-ref)
      nh (name-hash src ns ne)]
      (do
        (p-advance pos-ref) ;; name を消費
        (p-expect spans pos-ref 2) ;; [ を消費
        (let [params-start (ref-get pos-ref)
          result (vector-push-triple-rooted-v3 (vector-new 8) 20 nh 0)]
          (do
            (root_push result)
            (let [with-params (parse-params-v3 spans pos-ref src result 0)]
              (do
                (root_push with-params)
                (let [params-end (ref-get pos-ref)
                  param-count (- (vector-length with-params) 3)
                  defn-node (vector-set-at-rooted-v3 with-params 2 param-count)]
                  (do
                    (root_push defn-node)
                    (let [param-signature (parse-defn-param-signature-v3
                      spans params-start params-end src param-count)]
                      (do
                        (root_push param-signature)
                        (let [return-type (skip-optional-type-sig-v3 spans pos-ref src)]
                          (do
                            (root_push return-type)
                            (let [signature (vector-push-single-rooted-v3 param-signature return-type)]
                              (do
                                (root_push signature)
                                (skip-optional-where-v3 spans pos-ref src)
                                (let [parsed
                                  (if (== (colon-directive-v3 spans pos-ref src) 1)
                                    (let [meta (parse-defn-metadata-v3 spans pos-ref src)]
                                      (parse-defn-bodyless-or-body-with-meta-v3
                                        spans pos-ref src defn-node param-count signature meta))
                                    (if (== (p-current spans pos-ref) 1)
                                      (do
                                        (p-advance pos-ref)
                                        (let [bodyless-defn-body (make-int-node 0)
                                          bodyless-finalized-defn (finalize-defn-body-v3 bodyless-defn-body defn-node)]
                                          (maybe-append-defn-signature-v3 bodyless-finalized-defn signature)))
                                      (let [parsed-defn-body (parse-expr-v3 spans pos-ref src)]
                                        (do
                                          (root_push parsed-defn-body)
                                          (let [expr-finalized-defn (finalize-defn-body-v3 parsed-defn-body defn-node)]
                                            (do
                                              (root_push expr-finalized-defn)
                                              (p-expect spans pos-ref 1)
                                              (let [parsed-with-signature
                                                (maybe-append-defn-signature-v3 expr-finalized-defn signature)]
                                                (do
                                                  (root_pop)
                                                  (root_pop)
                                                  parsed-with-signature))))))))]
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
                                    parsed))))))))))))))))))

;; === defmacro 宣言 ===
(defn parse-defmacro-v3 [spans pos-ref src]
  (do
    (p-advance pos-ref) ;; defmacro を消費
    (let [ns (p-start spans pos-ref)
      ne (p-end spans pos-ref)
      nh (name-hash src ns ne)]
      (do
        (p-advance pos-ref) ;; name を消費
        (p-expect spans pos-ref 2) ;; [ を消費
        (let [result (make-defmacro nh)]
          (do
            (let [result-slot (root_push result)
              with-params (parse-params-v3 spans pos-ref src result 0)]
              (do
                (root_push with-params)
                (let [param-count (- (vector-length with-params) 3)
                  macro-node (vector-set-at-rooted-v3 with-params 2 param-count)]
                  (do
                    (root_set result-slot macro-node)
                    (skip-optional-type-sig-v3 spans pos-ref src)
                    (skip-optional-where-v3 spans pos-ref src)
                    (skip-optional-metadata-v3 spans pos-ref src)
                    (let [body (parse-expr-v3 spans pos-ref src)]
                      (do
                        (root_push body)
                        (p-expect spans pos-ref 1) ;; ) を消費
                        (let [parsed (vector-push macro-node body)]
                          (do
                            (root_pop)
                            (root_pop)
                            (root_pop)
                            parsed))))))))))))))

;; === private 宣言 ===
(defn parse-private-v3 [spans pos-ref src]
  (do
    (p-advance pos-ref) ;; private を消費
    (let [inner (parse-expr-v3 spans pos-ref src)]
      (do
        (root_push inner)
        (p-expect spans pos-ref 1) ;; ) を消費
        (let [parsed (make-private inner)]
          (do
            (root_pop)
            parsed))))))

;; `(record (: field Type) ...)` の field 名、Type.field accessor 名、raw TypeExpr を保持する。
(defn parse-record-decl-field-symbol-step-v3
  [spans pos-ref src record-name-hash fields]
  (if (== (p-current spans pos-ref) 20)
    (do
      (let [fields-slot (root_push fields)
        field-start (p-start spans pos-ref)
        field-end (p-end spans pos-ref)
        field-hash (name-hash src field-start field-end)
        accessor-hash
          (name-hash-loop
            src
            field-start
            field-end
            (+ 46 (* record-name-hash 31)))]
        (do
          (p-advance pos-ref)
          (let [type-expr (parse-type-expr-v3 spans pos-ref src)]
            (do
              (root_push type-expr)
              (p-expect spans pos-ref 1)
                (let [next-fields
                        (vector-push-triple-rooted-v3
                          fields
                          field-hash
                          accessor-hash
                          type-expr)]
                  (do
                    (root_push next-fields)
                    (let [state (make-parse-loop-state 0 next-fields)]
                      (do
                        (root_pop)
                        (root_pop)
                        (root_pop)
                        state)))))))))
    (do
      (parse-skip-to-close-v3 spans pos-ref 1)
      (make-parse-loop-state 0 fields))))

(defn parse-record-decl-field-form-step-v3
  [spans pos-ref src record-name-hash fields]
  (if (== (p-current spans pos-ref) 50)
    (do
      (p-advance pos-ref)
      (parse-record-decl-field-symbol-step-v3
        spans
        pos-ref
        src
        record-name-hash
        fields))
    (do
      (parse-skip-to-close-v3 spans pos-ref 1)
      (make-parse-loop-state 0 fields))))

(defn parse-record-decl-fields-step-v3 [spans pos-ref src record-name-hash fields]
  (if (== (p-current spans pos-ref) 1)
    (do
      (p-advance pos-ref)
      (make-parse-loop-state 1 fields))
    (if (== (p-current spans pos-ref) 99)
      (make-parse-loop-state 1 fields)
      (if (== (p-current spans pos-ref) 0)
        (do
          (p-advance pos-ref)
          (parse-record-decl-field-form-step-v3
            spans
            pos-ref
            src
            record-name-hash
            fields))
        (do
          (p-advance pos-ref)
          (make-parse-loop-state 0 fields))))))

(defn parse-record-decl-fields-step-64-loop-bounded
  [spans pos-ref src record-name-hash fields remaining]
  (do
    (root_push fields)
    (let [step
            (parse-record-decl-fields-step-v3
              spans
              pos-ref
              src
              record-name-hash
              fields)
      done (vector-get step 0)
      next-fields (vector-get step 1)]
      (do
        (root_push step)
        (root_push next-fields)
        (let [parsed
          (if (= done 1)
            step
            (if (<= remaining 1)
              step
              (parse-record-decl-fields-step-64-loop-bounded
                spans
                pos-ref
                src
                record-name-hash
                next-fields
                (- remaining 1))))]
          (do
            (root_pop)
            (root_pop)
            (root_pop)
            parsed))))))

(defn parse-record-decl-fields-step-64 [spans pos-ref src record-name-hash fields]
  (parse-record-decl-fields-step-64-loop-bounded
    spans
    pos-ref
    src
    record-name-hash
    fields
    64))

(defn parse-record-decl-fields-rooted-v3 [spans pos-ref src record-name-hash fields]
  (let [step
          (parse-record-decl-fields-step-64
            spans
            pos-ref
            src
            record-name-hash
            fields)]
    (if (= (vector-get step 0) 1)
      (vector-get step 1)
      (do
        (root_push step)
        (let [next-fields (vector-get step 1)]
          (do
            (root_push next-fields)
            (let [parsed
                    (parse-record-decl-fields-rooted-v3
                      spans
                      pos-ref
                      src
                      record-name-hash
                      next-fields)]
              (do
                (root_pop)
                (root_pop)
                parsed))))))))

(defn parse-record-decl-fields-v3 [spans pos-ref src record-name-hash]
  (do
    (root_push spans)
    (root_push pos-ref)
    (root_push src)
    (let [fields (vector-new 0)
      fields-slot (root_push fields)]
      (let [parsed
              (parse-record-decl-fields-rooted-v3
                spans
                pos-ref
                src
                record-name-hash
                fields)]
        (do
          (root_set fields-slot parsed)
          (root_pop)
          (root_pop)
          (root_pop)
          (root_pop)
          parsed)))))

(defn parse-record-def-v3 [spans pos-ref src name-hash params]
  (do
    (p-expect spans pos-ref 0) ;; record form の ( を消費
    (p-expect spans pos-ref 39) ;; record を消費
    (let [fields (parse-record-decl-fields-v3 spans pos-ref src name-hash)]
      (do
        (root_push fields)
        (root_push params)
        (p-expect spans pos-ref 1) ;; type 宣言の ) を消費
        (let [parsed
                (if (= (vector-length params) 0)
                  (make-record-def-with-fields name-hash fields)
                  (make-record-def-with-params name-hash params fields))]
          (do
            (root_pop)
            (root_pop)
            parsed))))))

;; ADT variant の field 型を左から保持する。末尾の ) はここで消費する。
(defn parse-type-variant-fields-step-v3 [spans pos-ref src fields]
  (if (== (p-current spans pos-ref) 1)
    (do
      (p-advance pos-ref)
      (make-parse-loop-state 1 fields))
    (if (== (p-current spans pos-ref) 99)
      (make-parse-loop-state 1 fields)
      (do
        (root_push fields)
        (let [type-expr (parse-type-expr-v3 spans pos-ref src)]
          (do
            (root_push type-expr)
            (let [next-fields (vector-push-single-rooted-v3 fields type-expr)]
              (do
                (root_push next-fields)
                (let [state (make-parse-loop-state 0 next-fields)]
                  (do
                    (root_pop)
                    (root_pop)
                    (root_pop)
                    state))))))))))

(defn parse-type-variant-fields-step-64-loop-bounded
  [spans pos-ref src fields remaining]
  (do
    (root_push fields)
    (let [step
            (parse-type-variant-fields-step-v3 spans pos-ref src fields)
      done (vector-get step 0)
      next-fields (vector-get step 1)]
      (do
        (root_push step)
        (root_push next-fields)
        (let [parsed
          (if (= done 1)
            step
            (if (<= remaining 1)
              step
              (parse-type-variant-fields-step-64-loop-bounded
                spans
                pos-ref
                src
                next-fields
                (- remaining 1))))]
          (do
            (root_pop)
            (root_pop)
            (root_pop)
            parsed))))))

(defn parse-type-variant-fields-step-64 [spans pos-ref src fields]
  (parse-type-variant-fields-step-64-loop-bounded
    spans
    pos-ref
    src
    fields
    64))

(defn parse-type-variant-fields-rooted-v3 [spans pos-ref src fields]
  (let [step (parse-type-variant-fields-step-64 spans pos-ref src fields)]
    (if (= (vector-get step 0) 1)
      (vector-get step 1)
      (do
        (root_push step)
        (let [next-fields (vector-get step 1)]
          (do
            (root_push next-fields)
            (let [parsed
                    (parse-type-variant-fields-rooted-v3
                      spans
                      pos-ref
                      src
                      next-fields)]
              (do
                (root_pop)
                (root_pop)
                parsed))))))))

(defn parse-type-variant-fields-v3 [spans pos-ref src]
  (do
    (root_push spans)
    (root_push pos-ref)
    (root_push src)
    (let [fields (vector-new 0)
      fields-slot (root_push fields)]
      (let [parsed (parse-type-variant-fields-rooted-v3 spans pos-ref src fields)]
        (do
          (root_set fields-slot parsed)
          (root_pop)
          (root_pop)
          (root_pop)
          (root_pop)
          parsed)))))

;; GADT variant の (: <variant-form> <return-type>) を parse する。
(defn parse-type-variant-gadt-v3 [spans pos-ref src]
  (do
    (p-advance pos-ref) ;; : を消費
    (if (== (p-current spans pos-ref) 0)
      (do
        (p-advance pos-ref) ;; variant form の ( を消費
        (let [name-hash (current-symbol-hash-v3 spans pos-ref src)]
          (do
            (p-advance pos-ref)
            (let [fields (parse-type-variant-fields-v3 spans pos-ref src)]
              (do
                (root_push fields)
                (let [return-type (parse-type-expr-v3 spans pos-ref src)]
                  (do
                    (root_push return-type)
                    (p-expect spans pos-ref 1)
                    (let [parsed
                            (make-type-variant-with-return-type
                              name-hash
                              fields
                              return-type)]
                      (do
                        (root_pop)
                        (root_pop)
                        parsed)))))))))
      (let [name-hash (current-symbol-hash-v3 spans pos-ref src)
        fields (vector-new 0)]
        (do
          (p-advance pos-ref)
          (root_push fields)
          (let [return-type (parse-type-expr-v3 spans pos-ref src)]
            (do
              (root_push return-type)
              (p-expect spans pos-ref 1)
              (let [parsed
                      (make-type-variant-with-return-type
                        name-hash
                        fields
                        return-type)]
                (do
                  (root_pop)
                  (root_pop)
                  parsed)))))))))

;; parenthesized ADT variant の outer `(` を消費した後を処理する。
(defn parse-type-variant-parenthesized-v3 [spans pos-ref src]
  (do
    (p-advance pos-ref)
    (if (== (p-current spans pos-ref) 50)
      (parse-type-variant-gadt-v3 spans pos-ref src)
      (if (== (p-current spans pos-ref) 20)
        (let [name-hash (current-symbol-hash-v3 spans pos-ref src)]
          (do
            (p-advance pos-ref)
            (let [fields (parse-type-variant-fields-v3 spans pos-ref src)]
              (do
                (root_push fields)
                (let [parsed (make-type-variant name-hash fields)]
                  (do
                    (root_pop)
                    parsed))))))
        (do
          (parse-skip-to-close-v3 spans pos-ref 1)
          0)))))

;; ADT variant は bare constructor または (Constructor field-type ...) で表す。
;; constructor head を持たない form は読み飛ばす。
(defn parse-type-variant-v3 [spans pos-ref src]
  (if (== (p-current spans pos-ref) 20)
    (let [name-hash (current-symbol-hash-v3 spans pos-ref src)]
      (do
        (p-advance pos-ref)
        (make-type-variant name-hash (vector-new 0))))
    (if (== (p-current spans pos-ref) 0)
      (parse-type-variant-parenthesized-v3 spans pos-ref src)
      (do
        (p-advance pos-ref)
        0))))

;; type 宣言の outer ) まで ADT variant を収集する。
(defn parse-type-variants-step-v3 [spans pos-ref src variants]
  (if (== (p-current spans pos-ref) 1)
    (do
      (p-advance pos-ref)
      (make-parse-loop-state 1 variants))
    (if (== (p-current spans pos-ref) 99)
      (make-parse-loop-state 1 variants)
      (do
        (root_push variants)
        (let [variant (parse-type-variant-v3 spans pos-ref src)]
          (if (= variant 0)
            (do
              (root_pop)
              (make-parse-loop-state 0 variants))
            (do
              (root_push variant)
              (let [next-variants (vector-push-single-rooted-v3 variants variant)]
                (do
                  (root_push next-variants)
                  (let [state (make-parse-loop-state 0 next-variants)]
                    (do
                      (root_pop)
                      (root_pop)
                      (root_pop)
                      state)))))))))))

(defn parse-type-variants-step-64-loop-bounded
  [spans pos-ref src variants remaining]
  (do
    (root_push variants)
    (let [step (parse-type-variants-step-v3 spans pos-ref src variants)
      done (vector-get step 0)
      next-variants (vector-get step 1)]
      (do
        (root_push step)
        (root_push next-variants)
        (let [parsed
          (if (= done 1)
            step
            (if (<= remaining 1)
              step
              (parse-type-variants-step-64-loop-bounded
                spans
                pos-ref
                src
                next-variants
                (- remaining 1))))]
          (do
            (root_pop)
            (root_pop)
            (root_pop)
            parsed))))))

(defn parse-type-variants-step-64 [spans pos-ref src variants]
  (parse-type-variants-step-64-loop-bounded
    spans
    pos-ref
    src
    variants
    64))

(defn parse-type-variants-rooted-v3 [spans pos-ref src variants]
  (let [step (parse-type-variants-step-64 spans pos-ref src variants)]
    (if (= (vector-get step 0) 1)
      (vector-get step 1)
      (do
        (root_push step)
        (let [next-variants (vector-get step 1)]
          (do
            (root_push next-variants)
            (let [parsed
                    (parse-type-variants-rooted-v3
                      spans
                      pos-ref
                      src
                      next-variants)]
              (do
                (root_pop)
                (root_pop)
                parsed))))))))

(defn parse-type-variants-v3 [spans pos-ref src]
  (do
    (root_push spans)
    (root_push pos-ref)
    (root_push src)
    (let [variants (vector-new 0)
      variants-slot (root_push variants)]
      (let [parsed (parse-type-variants-rooted-v3 spans pos-ref src variants)]
        (do
          (root_set variants-slot parsed)
          (root_pop)
          (root_pop)
          (root_pop)
          (root_pop)
          parsed)))))

;; === type 宣言 ===
(defn parse-type-v3 [spans pos-ref src]
  (do
    (p-advance pos-ref) ;; type を消費
    (let [head (parse-type-decl-head-v3 spans pos-ref src)]
      (do
        (root_push head)
        (let [name-hash (vector-get head 0)
          params (vector-get head 1)
          ;; variant / metadata は読み飛ばすが、record field は後段の型推論へ渡す。
          head-kind (if (== (p-current spans pos-ref) 0)
            (span-kind spans (+ (ref-get pos-ref) 1))
            0)
          parsed
            (if (== head-kind 39)
              (parse-record-def-v3 spans pos-ref src name-hash params)
              (let [variants (parse-type-variants-v3 spans pos-ref src)]
                (do
                  (root_push variants)
                  (root_push params)
                  (let [result
                          (if (= (vector-length variants) 0)
                            (make-type-decl name-hash)
                            (if (= (vector-length params) 0)
                              (make-type-decl-with-variants name-hash variants)
                              (make-type-decl-with-params-and-variants
                                name-hash
                                params
                                variants)))]
                    (do
                      (root_pop)
                      (root_pop)
                      result)))))]
          (do
            (root_pop)
            parsed))))))

;; === trait 宣言 (簡易) ===
(defn parse-decl-body-step-v3 [spans pos-ref src result]
  (if (== (p-current spans pos-ref) 1)
    (do
      (p-advance pos-ref) ;; ) を消費
      (make-parse-loop-state 1 result))
    (do
      (let [result-slot (root_push result)
        decl (parse-expr-v3 spans pos-ref src)]
        (do
          (root_push decl)
          (let [next-result (vector-push result decl)
            state (do
              (root_set result-slot next-result)
              (make-parse-loop-state 0 next-result))]
            (do
              (root_pop)
              (root_pop)
              state)))))))

(defn parse-decl-body-step-64-loop-bounded [spans pos-ref src result remaining]
  (do
    (root_push result)
    (let [step (parse-decl-body-step-v3 spans pos-ref src result)
      done (vector-get step 0)
      next-result (vector-get step 1)]
      (do
        (root_push step)
        (root_push next-result)
        (let [parsed
          (if (= done 1)
            step
            (if (<= remaining 1)
              step
              (parse-decl-body-step-64-loop-bounded spans pos-ref src next-result (- remaining 1))))]
          (do
            (root_pop)
            (root_pop)
            (root_pop)
            parsed))))))

(defn parse-decl-body-step-64 [spans pos-ref src result]
  (parse-decl-body-step-64-loop-bounded spans pos-ref src result 64))

(defn parse-decl-body-rooted-v3 [spans pos-ref src result]
  (let [step (parse-decl-body-step-64 spans pos-ref src result)]
    (if (= (vector-get step 0) 1)
      (vector-get step 1)
      (do
        (root_push step)
        (let [next-result (vector-get step 1)]
          (do
            (root_push next-result)
            (let [parsed (parse-decl-body-rooted-v3 spans pos-ref src next-result)]
              (do
                (root_pop)
                (root_pop)
                parsed))))))))

(defn parse-decl-body-v3 [spans pos-ref src result]
  (do
    (root_push spans)
    (root_push pos-ref)
    (root_push src)
    (let [parsed (parse-decl-body-rooted-v3 spans pos-ref src result)]
      (do
        (root_pop)
        (root_pop)
        (root_pop)
        parsed))))

(defn parse-trait-v3 [spans pos-ref src]
  (do
    (p-advance pos-ref) ;; trait を消費
    (if (== (p-current spans pos-ref) 0)
      (do
        (p-advance pos-ref) ;; trait head の ( を消費
        (if (== (p-current spans pos-ref) 20)
          (let [name-start (p-start spans pos-ref)
            name-end (p-end spans pos-ref)
            name-h (name-hash src name-start name-end)]
            (do
              (p-advance pos-ref) ;; trait 名を消費
              (parse-skip-to-close-v3 spans pos-ref 1)
              (let [with-body (parse-decl-body-v3 spans pos-ref src
                  (make-trait-def name-h))]
                (do
                  (root_push with-body)
                  (let [parsed (vector-set-at-rooted-v3 with-body 2 (- (vector-length with-body) 3))]
                    (do
                      (root_pop)
                      parsed))))))
          (do
            (parse-skip-to-close-v3 spans pos-ref 1)
            (let [with-body (parse-decl-body-v3 spans pos-ref src
                (make-trait-def 0))]
              (do
                (root_push with-body)
                (let [parsed (vector-set-at-rooted-v3 with-body 2 (- (vector-length with-body) 3))]
                  (do
                    (root_pop)
                    parsed)))))))
      (do
        (parse-skip-to-close-v3 spans pos-ref 1)
        (make-trait-def 0)))))

;; === module 宣言 ===
(defn parse-module-v3 [spans pos-ref src]
  (do
    (p-advance pos-ref) ;; module を消費
    (let [name-start (p-start spans pos-ref)
      name-end (p-end spans pos-ref)
      name-h (name-hash src name-start name-end)]
      (do
        (p-advance pos-ref) ;; name を消費
        (let [with-body (parse-decl-body-v3 spans pos-ref src
            (make-module-decl-with-span name-h name-start name-end))]
          (do
            (root_push with-body)
            (let [parsed (vector-set-at-rooted-v3 with-body 2 (- (vector-length with-body) 3))]
              (do
                (root_pop)
                parsed))))))))

;; === import 宣言 ===
(defn parse-import-only-symbols-step-v3 [spans pos-ref src result]
  (if (== (p-current spans pos-ref) 3)
    (do
      (p-advance pos-ref)
      (make-parse-loop-state 1 result))
    (if (== (p-current spans pos-ref) 99)
      (make-parse-loop-state 1 result)
      (if (== (p-current spans pos-ref) 20)
        (do
          (root_push result)
          (let [item (current-symbol-hash-v3 spans pos-ref src)]
            (do
              (p-advance pos-ref)
              (root_push item)
              (let [next-result (vector-push-single-rooted-v3 result item)]
                (do
                  (root_push next-result)
                  (let [state (make-parse-loop-state 0 next-result)]
                    (do
                      (root_pop)
                      (root_pop)
                      (root_pop)
                      state)))))))
        (do
          (p-advance pos-ref)
          (make-parse-loop-state 0 result))))))

(defn parse-import-only-symbols-step-64-loop-bounded
  [spans pos-ref src result remaining]
  (do
    (root_push result)
    (let [step (parse-import-only-symbols-step-v3 spans pos-ref src result)
      done (vector-get step 0)
      next-result (vector-get step 1)]
      (do
        (root_push step)
        (root_push next-result)
        (let [parsed
          (if (= done 1)
            step
            (if (<= remaining 1)
              step
              (parse-import-only-symbols-step-64-loop-bounded
                spans
                pos-ref
                src
                next-result
                (- remaining 1))))]
          (do
            (root_pop)
            (root_pop)
            (root_pop)
            parsed))))))

(defn parse-import-only-symbols-step-64 [spans pos-ref src result]
  (parse-import-only-symbols-step-64-loop-bounded spans pos-ref src result 64))

(defn parse-import-only-symbols-rooted-v3 [spans pos-ref src result]
  (let [step (parse-import-only-symbols-step-64 spans pos-ref src result)]
    (if (= (vector-get step 0) 1)
      (vector-get step 1)
      (do
        (root_push step)
        (let [next-result (vector-get step 1)]
          (do
            (root_push next-result)
            (let [parsed
                    (parse-import-only-symbols-rooted-v3
                      spans
                      pos-ref
                      src
                      next-result)]
              (do
                (root_pop)
                (root_pop)
                parsed))))))))

(defn parse-import-only-symbols-v3 [spans pos-ref src result]
  (do
    (root_push spans)
    (root_push pos-ref)
    (root_push src)
    (let [parsed (parse-import-only-symbols-rooted-v3 spans pos-ref src result)]
      (do
        (root_pop)
        (root_pop)
        (root_pop)
        parsed))))

(defn parse-import-options-v3
  [spans pos-ref src name-h name-start name-end alias-hash only-present only-hashes open-present]
  (if (== (p-current spans pos-ref) 1)
    (do
      (p-advance pos-ref) ;; ) を消費
      (make-import-decl-from-options
        name-h
        name-start
        name-end
        alias-hash
        only-present
        only-hashes
        open-present))
    (if (== (p-current spans pos-ref) 50)
      (do
        (p-advance pos-ref) ;; : を消費
        (if (or (= (p-current spans pos-ref) 20)
            (= (p-current spans pos-ref) 49))
          (let [option-name (current-symbol-text-v3 spans pos-ref src)]
            (if (string-eq option-name "as")
              (do
                (p-advance pos-ref) ;; as を消費
                (let [alias-start (p-start spans pos-ref)
                  alias-end (p-end spans pos-ref)
                  next-alias-hash (name-hash src alias-start alias-end)]
                  (do
                    (p-advance pos-ref) ;; alias を消費
                    (root_push only-hashes)
                    (let [parsed
                            (parse-import-options-v3
                              spans
                              pos-ref
                              src
                              name-h
                              name-start
                              name-end
                              next-alias-hash
                              only-present
                              only-hashes
                              open-present)]
                      (do
                        (root_pop)
                        parsed)))))
              (if (string-eq option-name "only")
                (do
                  (p-advance pos-ref) ;; only を消費
                  (p-expect spans pos-ref 2) ;; [ を消費
                  (let [parsed-only
                          (parse-import-only-symbols-v3
                            spans
                            pos-ref
                            src
                            (vector-new 0))]
                    (do
                      (root_push parsed-only)
                      (let [parsed
                              (parse-import-options-v3
                                spans
                                pos-ref
                                src
                                name-h
                                name-start
                                name-end
                                alias-hash
                                1
                                parsed-only
                                open-present)]
                        (do
                          (root_pop)
                          parsed)))))
                (if (string-eq option-name "open")
                  (do
                    (p-advance pos-ref) ;; open を消費
                    (parse-import-options-v3
                      spans
                      pos-ref
                      src
                      name-h
                      name-start
                      name-end
                      alias-hash
                      only-present
                      only-hashes
                      1))
                  (do
                    (p-advance pos-ref) ;; 未知 option を消費
                    (parse-import-options-v3
                      spans
                      pos-ref
                      src
                      name-h
                      name-start
                      name-end
                      alias-hash
                      only-present
                      only-hashes
                      open-present))))))
          (do
            (p-expect spans pos-ref 1)
            (make-import-decl-from-options
              name-h
              name-start
              name-end
              alias-hash
              only-present
              only-hashes
              open-present))))
      (do
        (p-expect spans pos-ref 1)
        (make-import-decl-from-options
          name-h
          name-start
          name-end
          alias-hash
          only-present
          only-hashes
          open-present)))))

(defn parse-import-v3 [spans pos-ref src]
  (do
    (p-advance pos-ref) ;; import を消費
    (let [name-start (p-start spans pos-ref)
      name-end (p-end spans pos-ref)
      name-h (name-hash src name-start name-end)]
      (do
        (p-advance pos-ref) ;; name を消費
        (if (== (p-current spans pos-ref) 50)
          (let [empty-only (vector-new 0)]
            (do
              (root_push empty-only)
              (let [parsed
                      (parse-import-options-v3
                        spans
                        pos-ref
                        src
                        name-h
                        name-start
                        name-end
                        0
                        0
                        empty-only
                        0)]
                (do
                  (root_pop)
                  parsed))))
          (do
            (p-expect spans pos-ref 1) ;; ) を消費
            (make-import-decl name-h name-start name-end)))))))

;; === apply (関数呼び出し) ===
(defn parse-apply-v3 [spans pos-ref src]
  (let [func-node (parse-expr-v3 spans pos-ref src)]
    (do
      (root_push func-node)
      (let [result (vector-push-triple-rooted-v3 (vector-new 8) 5 func-node 0)
        result-slot (root_push result)
        with-args (parse-apply-args-v3 spans pos-ref src result 0)
        arg-count (- (vector-length with-args) 3)
        parsed (do
          (root_set result-slot with-args)
          (vector-set-at-rooted-v3 with-args 2 arg-count))]
        (do
          (root_pop)
          (root_pop)
          parsed)))))

;; 引数を収集
(defn parse-apply-args-step-v3 [spans pos-ref src result]
  (if (== (p-current spans pos-ref) 1) ;; ) で終了
    (do
      (p-advance pos-ref)
      (make-parse-loop-state 1 result))
    (if (== (p-current spans pos-ref) 99) ;; EOF ガード: 無限ループ防止
      (make-parse-loop-state 1 result)
      (do
        (let [result-slot (root_push result)
          arg (parse-expr-v3 spans pos-ref src)]
          (do
            (root_push arg)
            (let [next-result (vector-push result arg)
              state (do
                (root_set result-slot next-result)
                (make-parse-loop-state 0 next-result))]
              (do
                (root_pop)
                (root_pop)
                state))))))))

(defn parse-apply-args-step-64-loop-bounded [spans pos-ref src result remaining]
  (do
    (root_push result)
    (let [step (parse-apply-args-step-v3 spans pos-ref src result)
      done (vector-get step 0)
      next-result (vector-get step 1)]
      (do
        (root_push step)
        (root_push next-result)
        (let [parsed
          (if (= done 1)
            step
            (if (<= remaining 1)
              step
              (parse-apply-args-step-64-loop-bounded spans pos-ref src next-result (- remaining 1))))]
          (do
            (root_pop)
            (root_pop)
            (root_pop)
            parsed))))))

(defn parse-apply-args-step-64 [spans pos-ref src result]
  (parse-apply-args-step-64-loop-bounded spans pos-ref src result 64))

(defn parse-apply-args-rooted-v3 [spans pos-ref src result count]
  (let [step (parse-apply-args-step-64 spans pos-ref src result)]
    (if (= (vector-get step 0) 1)
      (vector-get step 1)
      (do
        (root_push step)
        (let [next-result (vector-get step 1)]
          (do
            (root_push next-result)
            (let [parsed (parse-apply-args-rooted-v3 spans pos-ref src next-result count)]
              (do
                (root_pop)
                (root_pop)
                parsed))))))))

(defn parse-apply-args-v3 [spans pos-ref src result count]
  (do
    (root_push spans)
    (root_push pos-ref)
    (root_push src)
    (let [parsed (parse-apply-args-rooted-v3 spans pos-ref src result count)]
      (do
        (root_pop)
        (root_pop)
        (root_pop)
        parsed))))

;; === Recovery + 診断収集 ===

;; 診断レコード: [severity code span message-hash]
;; severity: 0=error, 1=warning, 2=info
;; code: 整数エラーコード
;; span: ソース位置 (start)
;; message-hash: メッセージの名前ハッシュ
(defn make-diagnostic [severity code span message-hash]
  (vector-push-quad-rooted-v3 (vector-new 4) severity code span message-hash))

;; 診断コレクタ: 診断のベクタを管理
(defn collect-diagnostics []
  (vector-new 8))

;; 診断を追加
(defn add-diagnostic [diagnostics diag]
  (vector-push diagnostics diag))

;; parse recovery より先に delimiter の未閉鎖を検出し、深い parser 再帰を EOF で止める。
(defn make-delimiter-balance-state
  [done code next-idx next-paren-depth next-bracket-depth next-first-code]
  (let [cursor
    (vector-push-quad-rooted-v3
      (vector-new 4)
      next-idx
      next-paren-depth
      next-bracket-depth
      next-first-code)]
    (do
      (root_push cursor)
      (let [state (vector-push-triple-rooted-v3 (vector-new 3) done code cursor)]
        (do
          (root_pop)
          state)))))

(defn parse-delimiter-balance-step-v3 [spans idx count paren-depth bracket-depth first-code]
  (if (>= idx count)
    (make-delimiter-balance-state
      1
      (if (> first-code 0)
        first-code
        (if (> bracket-depth 0) 1002 (if (> paren-depth 0) 1001 0)))
      idx
      paren-depth
      bracket-depth
      first-code)
    (let [kind (span-kind spans idx)]
      (if (== kind 0)
        (make-delimiter-balance-state
          0
          0
          (+ idx 1)
          (+ paren-depth 1)
          bracket-depth
          first-code)
        (if (== kind 1)
          (make-delimiter-balance-state
            0
            0
            (+ idx 1)
            (if (> paren-depth 0) (- paren-depth 1) paren-depth)
            bracket-depth
            (if (and (= paren-depth 0) (= first-code 0)) 1001 first-code))
          (if (== kind 2)
            (make-delimiter-balance-state
              0
              0
              (+ idx 1)
              paren-depth
              (+ bracket-depth 1)
              first-code)
            (if (== kind 3)
              (make-delimiter-balance-state
                0
                0
                (+ idx 1)
                paren-depth
                (if (> bracket-depth 0) (- bracket-depth 1) bracket-depth)
                (if (and (= bracket-depth 0) (= first-code 0)) 1002 first-code))
              (make-delimiter-balance-state
                0
                0
                (+ idx 1)
                paren-depth
                bracket-depth
                first-code))))))))

(defn parse-delimiter-balance-step-64-loop-bounded
  [spans idx count paren-depth bracket-depth first-code remaining]
  (do
    (root_push spans)
    (let [step (parse-delimiter-balance-step-v3 spans idx count paren-depth bracket-depth first-code)]
      (do
        (root_push step)
        (let [done (vector-get step 0)
          next-state (vector-get step 2)]
          (do
            (root_push next-state)
            (let [next-idx (vector-get next-state 0)
              next-paren-depth (vector-get next-state 1)
              next-bracket-depth (vector-get next-state 2)
              next-first-code (vector-get next-state 3)
              parsed
              (if (= done 1)
                step
                (if (<= remaining 1)
                  step
                  (parse-delimiter-balance-step-64-loop-bounded
                    spans
                    next-idx
                    count
                    next-paren-depth
                    next-bracket-depth
                    next-first-code
                    (- remaining 1))))]
              (do
                (root_pop)
                (root_pop)
                (root_pop)
                parsed))))))))

(defn parse-delimiter-balance-step-64 [spans idx count paren-depth bracket-depth first-code]
  (parse-delimiter-balance-step-64-loop-bounded
    spans
    idx
    count
    paren-depth
    bracket-depth
    first-code
    64))

(defn parse-delimiter-balance-rooted-v3
  [spans idx count paren-depth bracket-depth first-code]
  (let [step
    (parse-delimiter-balance-step-64 spans idx count paren-depth bracket-depth first-code)]
    (if (= (vector-get step 0) 1)
      (vector-get step 1)
      (do
        (root_push step)
        (let [next-state (vector-get step 2)]
          (do
            (root_push next-state)
            (let [parsed
              (parse-delimiter-balance-rooted-v3
                spans
                (vector-get next-state 0)
                count
                (vector-get next-state 1)
                (vector-get next-state 2)
                (vector-get next-state 3))]
              (do
                (root_pop)
                (root_pop)
                parsed))))))))

(defn parse-delimiter-diagnostic-code [spans]
  (do
    (root_push spans)
    (let [code
      (parse-delimiter-balance-rooted-v3 spans 0 (/ (vector-length spans) 3) 0 0 0)]
      (do
        (root_pop)
        code))))
(defn parse-delimiter-diagnostics [spans src]
  (let [code (parse-delimiter-diagnostic-code spans)]
    (if (= code 0)
      (vector-new 0)
      (vector-push-single-rooted-v3
        (vector-new 0)
        (make-diagnostic 0 code (string-length src) 0)))))

;; 次の同期ポイント (閉じ括弧 or トップレベル) まで回復
;; kind=1 (RParen), kind=99 (EOF) で停止
(defn recover-to-next [spans pos-ref]
  (let [kind (p-current spans pos-ref)]
    (if (== kind 99) 0 ;; EOF で停止
      (if (== kind 1) 0 ;; ) で停止
        (do (p-advance pos-ref)
          (recover-to-next spans pos-ref))))))

;; recovery 付きパース: パースに失敗したら回復して診断を記録
;; 戻り値: [ast-node, diagnostics-vector]
(defn parse-with-recovery [spans pos-ref src diagnostics]
  (let [start-pos (ref-get pos-ref)
    kind (p-current spans pos-ref)]
    (if (== kind 99) ;; EOF
      (vector-push-pair-rooted-v3 (vector-new 2) (make-int-node 0) diagnostics)
      ;; 不正なトークン (閉じ括弧が先に来た等) の場合 recovery
      (if (== kind 1) ;; 予期しない )
        (let [span (p-start spans pos-ref)
          diag (make-diagnostic 0 1001 span 0)
          diags (add-diagnostic diagnostics diag)]
          (do
            (p-advance pos-ref)
            (let [dummy (make-int-node 0)]
              (do
                (root_push dummy)
                (let [result (vector-push-pair-rooted-v3 (vector-new 2) dummy diags)]
                  (do
                    (root_pop)
                    result))))))
        (if (== kind 3) ;; 予期しない ]
          (let [span (p-start spans pos-ref)
            diag (make-diagnostic 0 1002 span 0)
            diags (add-diagnostic diagnostics diag)]
            (do
              (p-advance pos-ref)
              (let [dummy (make-int-node 0)]
                (do
                  (root_push dummy)
                  (let [result (vector-push-pair-rooted-v3 (vector-new 2) dummy diags)]
                    (do
                      (root_pop)
                      result))))))
          ;; 通常パース
          (let [node (parse-expr-v3 spans pos-ref src)]
            (do
              (root_push node)
              (let [result (vector-push-pair-rooted-v3 (vector-new 2) node diagnostics)]
                (do
                  (root_pop)
                  result)))))))))

;; === ユーティリティ ===

;; 対応する閉じ括弧まで読み飛ばし (ネスト対応)
(defn parse-skip-to-close-rooted-v3 [spans pos-ref depth]
  (if (<= depth 0) 0
    (do
      (let [kind (p-current spans pos-ref)
        result
        (if (== kind 99) 0 ;; EOF ガード: 無限ループ防止
          (do
            (p-advance pos-ref)
            (if (== kind 0) ;; ( でネスト深くなる
              (parse-skip-to-close-rooted-v3 spans pos-ref (+ depth 1))
              (if (== kind 1) ;; ) でネスト浅くなる
                (parse-skip-to-close-rooted-v3 spans pos-ref (- depth 1))
                (parse-skip-to-close-rooted-v3 spans pos-ref depth)))))]
        result))))

(defn parse-skip-to-close-v3 [spans pos-ref depth]
  (do
    (root_push spans)
    (root_push pos-ref)
    (let [result (parse-skip-to-close-rooted-v3 spans pos-ref depth)]
      (do
        (root_pop)
        (root_pop)
        result))))

;; === トップレベルパース ===

;; 複数のトップレベル式をパース
(defn make-parse-loop-state [done result]
  (vector-push-pair-rooted-v3 (vector-new 2) done result))

(defn parse-program-step-expr-v3 [spans pos-ref src result]
  (let [result-slot (root_push result)
    expr (parse-expr-v3 spans pos-ref src)]
    (do
      (root_push expr)
      (let [next-result (vector-push-single-rooted-v3 result expr)
        state (do
          (root_set result-slot next-result)
          (make-parse-loop-state 0 next-result))]
        (do
          (root_pop)
          (root_pop)
          state)))))

(defn parse-program-step-v3 [spans pos-ref src result]
  (if (== (p-current spans pos-ref) 99)
    (make-parse-loop-state 1 result)
    (if (== (p-current spans pos-ref) 0)
      (let [next-idx (+ (ref-get pos-ref) 1)
        next-kind (span-kind spans next-idx)]
        (if (== next-kind 30)
          (do
            (p-advance pos-ref)
            (let [result-slot (root_push result)
              expr (parse-defn-v3 spans pos-ref src)]
              (do
                (root_push expr)
                (let [next-result (vector-push-single-rooted-v3 result expr)
                  state (do
                    (root_set result-slot next-result)
                    (make-parse-loop-state 0 next-result))]
                  (do
                    (root_pop)
                    (root_pop)
                    state)))))
          (parse-program-step-expr-v3 spans pos-ref src result)))
      (parse-program-step-expr-v3 spans pos-ref src result))))

(defn parse-program-step-64-loop-bounded [spans pos-ref src result remaining]
  (do
    (root_push result)
    (let [step (parse-program-step-v3 spans pos-ref src result)
      done (vector-get step 0)
      next-result (vector-get step 1)]
      (do
        (root_push step)
        (root_push next-result)
        (let [parsed
          (if (= done 1)
            step
            (if (<= remaining 1)
              step
              (parse-program-step-64-loop-bounded spans pos-ref src next-result (- remaining 1))))]
          (do
            (root_pop)
            (root_pop)
            (root_pop)
            parsed))))))

(defn parse-program-step-64 [spans pos-ref src result]
  (parse-program-step-64-loop-bounded spans pos-ref src result 64))

(defn parse-program-v3 [spans pos-ref src]
  (let [result (vector-new 16)]
    (do
      (root_push spans)
      (root_push pos-ref)
      (root_push src)
      (root_push result)
      (let [parsed (parse-program-loop-rooted-v3 spans pos-ref src result)]
        (do
          (root_pop)
          (root_pop)
          (root_pop)
          (root_pop)
          parsed)))))

(defn parse-program-loop-rooted-v3 [spans pos-ref src result]
  (let [step (parse-program-step-64 spans pos-ref src result)]
    (if (= (vector-get step 0) 1)
      (vector-get step 1)
      (do
        (root_push step)
        (let [next-result (vector-get step 1)]
          (do
            (root_push next-result)
            (let [parsed (parse-program-loop-rooted-v3 spans pos-ref src next-result)]
              (do
                (root_pop)
                (root_pop)
                parsed))))))))

(defn parse-program-loop-v3 [spans pos-ref src result]
  (do
    (root_push spans)
    (root_push pos-ref)
    (root_push src)
    (let [parsed (parse-program-loop-rooted-v3 spans pos-ref src result)]
      (do
        (root_pop)
        (root_pop)
        (root_pop)
        parsed))))

;; ソース文字列をトークン化してから v3 パーサでプログラム (宣言の Vector) を返す
(defn parse-program [src]
  (do
    (root_push src)
    (let [spans (tokenize-with-spans src)]
      (do
        (root_push spans)
        (let [pos-ref (ref-new 0)]
          (do
            (root_push pos-ref)
            (let [program (parse-program-v3 spans pos-ref src)]
              (do
                (root_pop)
                (root_pop)
                (root_pop)
                program))))))))

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

;; デモ用エントリポイント (テスト用)
(defn demo-main []
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
      (print (node-tag result)) ;; 20 (defn)
      (print (ref-get pos)) ;; 2 (パース後位置)
      (print (node-tag match-result)) ;; 10 (match)
      ;; match ノードのタグ検証
      (print (vector-get mn2 0)) ;; 10 (match tag)
      (print (vector-get mn2 2)) ;; 2 (arm-count)
      ;; 腕のパターン値
      (print (vector-get mn2 3)) ;; 1 (pat1)
      (print (vector-get mn2 5)) ;; 2 (pat2)
      0)))
