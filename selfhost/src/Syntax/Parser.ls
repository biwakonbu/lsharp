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
;; tag=26: import-decl [26, name-hash, name-start, name-end]

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

(defn current-symbol-hash-v3 [spans pos-ref src]
  (name-hash src (p-start spans pos-ref) (p-end spans pos-ref)))

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

;; 文字列ノード: [3, start, end]
(defn make-string-node [start end]
  (vector-push-triple-rooted-v3 (vector-new 3) 3 start end))

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
      (let [type-start (p-start spans pos-ref)
        type-end (p-end spans pos-ref)
        type-h (name-hash src type-start type-end)
        result (make-recordlit type-h)
        result-slot (root_push result)]
        (do
          (p-advance pos-ref) ;; type 名を消費
          (let [with-fields (parse-recordlit-fields-v3 spans pos-ref src result 0)
            field-count (/ (- (vector-length with-fields) 3) 2)
            parsed (do
              (root_set result-slot with-fields)
              (vector-set-at-rooted-v3 with-fields 2 field-count))]
            (do
              (root_pop)
              parsed))))
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

(defn parse-recordlit-fields-rooted-v3 [spans pos-ref src result count]
  (if (== (p-current spans pos-ref) 5) ;; } で終了
    (do (p-advance pos-ref) result)
    (if (== (p-current spans pos-ref) 99) ;; EOF ガード: 無限ループ防止
      result
      (if (== (p-current spans pos-ref) 20)
        (do
          (root_push result)
          (let [field-start (p-start spans pos-ref)
            field-end (p-end spans pos-ref)
            field-h (name-hash src field-start field-end)]
            (do
              (p-advance pos-ref) ;; field 名を消費
              (let [value (parse-expr-v3 spans pos-ref src)]
                (do
                  (root_push value)
                  (let [next-result (vector-push-pair-rooted-v3 result field-h value)]
                    (do
                      (root_push next-result)
                      (let [parsed (parse-recordlit-fields-rooted-v3 spans pos-ref src next-result (+ count 1))]
                        (do
                          (root_pop)
                          (root_pop)
                          (root_pop)
                          parsed)))))))))
        (do
          (p-advance pos-ref)
          (parse-recordlit-fields-rooted-v3 spans pos-ref src result count))))))

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
                    (if (string-eq name "transitions") 1
                      (if (string-eq name "constraints") 1
                        0))))))))))))

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
      (do
        (p-advance pos-ref)
        (if (== kind 2)
          (parse-skip-bracket-v3 spans pos-ref (+ depth 1))
          (if (== kind 3)
            (parse-skip-bracket-v3 spans pos-ref (- depth 1))
            (parse-skip-bracket-v3 spans pos-ref depth)))))))

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

;; defn 用メタデータパーサー: :doc / :example / :params / :returns を記録する
;; 返却: [doc-string, example-text, params-vector, returns-string]
(defn parse-defn-metadata-v3 [spans pos-ref src]
  (let [params0 (vector-new 0)]
    (do
      (root_push params0)
      (let [meta (vector-push-quad-rooted-v3 (vector-new 4) "" "" params0 "")]
        (do
          (root_pop)
          (parse-defn-metadata-loop-v3 spans pos-ref src meta))))))

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
                  (do
                    (skip-directive-payload-v3 spans pos-ref)
                    (parse-defn-metadata-loop-rooted-v3 spans pos-ref src meta))))))]
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
(defn parse-defn-meta-example-v3 [spans pos-ref src meta]
  (if (== (p-current spans pos-ref) 2)
    (do
      (p-advance pos-ref)
      (if (== (p-current spans pos-ref) 3)
        (do (p-advance pos-ref) (parse-defn-metadata-loop-v3 spans pos-ref src meta))
        (let [content-start (p-start spans pos-ref)]
          (do
            (parse-skip-bracket-v3 spans pos-ref 1)
            (let [last-idx (- (ref-get pos-ref) 2)
              content-end (span-end spans last-idx)
              example-text (substring src content-start content-end)
              updated (vector-set-at-rooted-v3 meta 1 example-text)]
              (parse-defn-metadata-loop-v3 spans pos-ref src updated))))))
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

(defn defn-metadata-present-v3 [meta]
  (if (> (string-length (vector-get meta 0)) 0)
    1
    (if (> (string-length (vector-get meta 1)) 0)
      1
      (if (> (vector-length (vector-get meta 2)) 0)
        1
        (if (> (string-length (vector-get meta 3)) 0)
          1
          0)))))

(defn finalize-defn-body-v3 [defn-node param-count body]
  (let [body-idx (+ 3 param-count)
    placeholder (make-int-node 0)]
    (do
      (root_push placeholder)
      (let [node-with-placeholder (vector-push defn-node placeholder)]
        (do
          (root_push node-with-placeholder)
          (let [parsed (vector-set-at-rooted-v3 node-with-placeholder body-idx body)]
            (do
              (if (> (string-length (command-line-arg 8)) 0)
                (do
                  (print 225)
                  (print body-idx)
                  (print (vector-get node-with-placeholder 0))
                  (print (vector-length node-with-placeholder))
                  (print (vector-get parsed 0))
                  (print (vector-length parsed)))
                (do))
              (root_pop)
              (root_pop)
              parsed)))))))

(defn maybe-append-defn-meta-v3 [node meta]
  (if (= (defn-metadata-present-v3 meta) 1)
    (vector-push node meta)
    node))

(defn finalize-defn-parsed-body-v3 [spans pos-ref defn-node param-count body]
  (let [parsed-ref (ref-new (make-int-node 0))]
    (do
      (root_push parsed-ref)
      (root_push defn-node)
      (root_push body)
      (p-expect spans pos-ref 1) ;; ) を消費
      (let [parsed (finalize-defn-body-v3 defn-node param-count body)]
        (do
          (ref-set parsed-ref parsed)
          (if (> (string-length (command-line-arg 8)) 0)
            (do
              (print 226)
              (print 0)
              (print (vector-get body 0))
              (print (vector-length body))
              (print (vector-get parsed 0))
              (print (vector-length parsed))
              (print (vector-get (ref-get parsed-ref) 0))
              (print (vector-length (ref-get parsed-ref))))
            (do))
          (root_pop)
          (root_pop)
          (root_pop)
          (if (> (string-length (command-line-arg 8)) 0)
            (do
              (print 226)
              (print 1)
              (print (vector-get body 0))
              (print (vector-length body))
              (print (vector-get parsed 0))
              (print (vector-length parsed))
              (print (vector-get (ref-get parsed-ref) 0))
              (print (vector-length (ref-get parsed-ref))))
            (do))
          (ref-get parsed-ref))))))

(defn parse-defn-bodyless-or-body-v3 [spans pos-ref src defn-node param-count]
  (if (== (p-current spans pos-ref) 1)
    (do
      (p-advance pos-ref) ;; bodyless defn の ) を消費
      (let [bodyless-body (make-int-node 0)]
        (do
          (root_push bodyless-body)
          (let [bodyless-parsed (finalize-defn-body-v3 defn-node param-count bodyless-body)]
            (do
              (root_pop)
              bodyless-parsed)))))
    (let [helper-body (parse-expr-v3 spans pos-ref src)]
      (do
        (root_push helper-body)
        (let [helper-parsed (finalize-defn-parsed-body-v3 spans pos-ref defn-node param-count helper-body)]
          (do
            (root_pop)
            helper-parsed))))))

(defn parse-defn-bodyless-or-body-with-meta-v3 [spans pos-ref src defn-node param-count meta]
  (maybe-append-defn-meta-v3
    (parse-defn-bodyless-or-body-v3 spans pos-ref src defn-node param-count)
    meta))

(defn skip-optional-type-sig-v3 [spans pos-ref src]
  (if (== (colon-directive-v3 spans pos-ref src) 1)
    0
    (if (== (p-current spans pos-ref) 50) ;; :
      (do
        (p-advance pos-ref)
        (skip-type-expr-v3 spans pos-ref)
        0)
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
        (skip-type-expr-v3 spans pos-ref)
        (p-expect spans pos-ref 1)
        (let [parsed (make-ann expr)]
          (do
            (root_pop)
            parsed))))))

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
              (parse-skip-to-close-v3 spans pos-ref 1)
              (skip-type-expr-v3 spans pos-ref)
              (p-expect spans pos-ref 1) ;; ) を消費
              (make-type-alias name-h)))
          (do
            (parse-skip-to-close-v3 spans pos-ref 1)
            (parse-skip-to-close-v3 spans pos-ref 1)
            (make-type-alias 0))))
      (if (== (p-current spans pos-ref) 20)
        (let [name-h (current-symbol-hash-v3 spans pos-ref src)]
          (do
            (p-advance pos-ref) ;; alias 名を消費
            (skip-type-expr-v3 spans pos-ref)
            (p-expect spans pos-ref 1) ;; ) を消費
            (make-type-alias name-h)))
        (do
          (parse-skip-to-close-v3 spans pos-ref 1)
          (make-type-alias 0))))))

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

(defn parse-impl-v3 [spans pos-ref src]
  (do
    (p-advance pos-ref) ;; impl を消費
    (if (== (p-current spans pos-ref) 0)
      (do
        (p-advance pos-ref) ;; impl head の ( を消費
        (if (== (p-current spans pos-ref) 20)
          (let [trait-h (current-symbol-hash-v3 spans pos-ref src)]
            (do
              (p-advance pos-ref) ;; trait 名を消費
              (if (== (p-current spans pos-ref) 20)
                (let [type-h (current-symbol-hash-v3 spans pos-ref src)]
                  (do
                    (p-advance pos-ref) ;; type 名を消費
                    (parse-skip-to-close-v3 spans pos-ref 1)
                    (let [with-body (parse-decl-body-v3 spans pos-ref src
                        (make-impl-def trait-h type-h))]
                      (vector-set-at-rooted-v3 with-body 3 (- (vector-length with-body) 4)))))
                (do
                  (parse-skip-to-close-v3 spans pos-ref 1)
                  (let [with-body (parse-decl-body-v3 spans pos-ref src
                      (make-impl-def trait-h 0))]
                    (vector-set-at-rooted-v3 with-body 3 (- (vector-length with-body) 4)))))))
          (do
            (parse-skip-to-close-v3 spans pos-ref 1)
            (let [with-body (parse-decl-body-v3 spans pos-ref src
                (make-impl-def 0 0))]
              (vector-set-at-rooted-v3 with-body 3 (- (vector-length with-body) 4))))))
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

(defn parse-computation-steps-rooted-v3 [spans pos-ref src result]
  (if (== (p-current spans pos-ref) 1)
    (do
      (p-advance pos-ref)
      result)
    (do
      (root_push result)
      (let [step (parse-computation-step-v3 spans pos-ref src)]
        (do
          (root_push step)
          (let [next-result (computation-add-step result (vector-get step 0) (vector-get step 1) (vector-get step 2))]
            (do
              (root_push next-result)
              (let [parsed (parse-computation-steps-rooted-v3 spans pos-ref src next-result)]
                (do
                  (root_pop)
                  (root_pop)
                  (root_pop)
                  parsed)))))))))

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
    h (name-hash src start end)]
    (do
      (p-advance pos-ref)
      (make-var-node h))))

(defn parse-string-node-v3 [spans pos-ref]
  (let [start (p-start spans pos-ref)
    end (p-end spans pos-ref)]
    (do
      (p-advance pos-ref)
      (make-string-node (+ start 1) (- end 1)))))

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
              (parse-string-node-v3 spans pos-ref) ;; 引用符を除く
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
                (p-expect spans pos-ref 1) ;; ) を消費
                (let [parsed (vector-push-quad-rooted-v3 (vector-new 8) 6 cond-node then-node else-node)]
                  (do
                    (root_pop)
                    (root_pop)
                    (root_pop)
                    parsed))))))))))

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

(defn parse-let-after-first-binding-v3 [spans pos-ref src nh init]
  (do
    (root_push init)
    ;; 追加バインディングがあるかチェック
    (if (== (p-current spans pos-ref) 3) ;; ] で終了
      (do
        (p-advance pos-ref) ;; ] を消費
        (let [body (parse-let-body-v3 spans pos-ref src)]
          (do
            (root_push body)
            (p-expect spans pos-ref 1) ;; ) を消費
            (let [result (vector-push-quad-rooted-v3 (vector-new 8) 7 nh init body)]
              (do
                (root_pop)
                (root_pop)
                result)))))
      ;; 複数バインディング: 次のバインディングを body として再帰
      (let [ns2 (p-start spans pos-ref)
        ne2 (p-end spans pos-ref)
        nh2 (name-hash src ns2 ne2)]
        (do
          (p-advance pos-ref) ;; name2 を消費
          (let [init2 (parse-expr-v3 spans pos-ref src)]
            (do
              (root_push init2)
              (let [rest-body (parse-let-rest-v3 spans pos-ref src)]
                (do
                  (root_push rest-body)
                  (let [inner (vector-push-quad-rooted-v3 (vector-new 8) 7 nh2 init2 rest-body)]
                    (do
                      (root_push inner)
                      (p-expect spans pos-ref 1) ;; ) を消費
                      (let [result (vector-push-quad-rooted-v3 (vector-new 8) 7 nh init inner)]
                        (do
                          (root_pop)
                          (root_pop)
                          (root_pop)
                          (root_pop)
                          result)))))))))))))

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
            (root_push init)
            (let [parsed (parse-let-after-first-binding-v3 spans pos-ref src nh init)]
              (do
                (root_pop)
                parsed))))))))

;; let の残りバインディングを処理
(defn parse-let-rest-rooted-v3 [spans pos-ref src]
  (if (== (p-current spans pos-ref) 3) ;; ] に到達
    (do
      (p-advance pos-ref) ;; ] を消費
      (parse-let-body-v3 spans pos-ref src)) ;; body をパース
    (if (== (p-current spans pos-ref) 99)
      (make-int-node 0)
      (if (== (p-current spans pos-ref) 1)
        (make-int-node 0)
        ;; さらにバインディングがある
        (do
          (let [ns (p-start spans pos-ref)
            ne (p-end spans pos-ref)
            nh (name-hash src ns ne)]
            (do
              (p-advance pos-ref) ;; name を消費
              (let [init (parse-expr-v3 spans pos-ref src)]
                (do
                  (root_push init)
                  (let [rest (parse-let-rest-rooted-v3 spans pos-ref src)]
                    (do
                      (root_push rest)
                      (let [result (vector-push-quad-rooted-v3 (vector-new 8) 7 nh init rest)]
                        (do
                          (root_pop)
                          (root_pop)
                          result)))))))))))))

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

;; do 内の式を収集
(defn parse-do-exprs-rooted-v3 [spans pos-ref src result count]
  (if (== (p-current spans pos-ref) 1) ;; ) で終了
    (do
      (p-advance pos-ref) ;; ) を消費
      ;; count を更新 (index 1)
      result)
    (if (== (p-current spans pos-ref) 99)
      result
      (do
        (root_push result)
        (let [expr (parse-expr-v3 spans pos-ref src)]
          (do
            (root_push expr)
            (let [next-result (vector-push result expr)]
              (do
                (root_push next-result)
                (let [parsed (parse-do-exprs-rooted-v3 spans pos-ref src next-result (+ count 1))]
                  (do
                    (root_pop)
                    (root_pop)
                    (root_pop)
                    parsed))))))))))

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

(defn parse-constructor-pattern-args-rooted-v3 [spans pos-ref src result count]
  (if (== (p-current spans pos-ref) 1) ;; ) で終了
    (do (p-advance pos-ref) result)
    (do
      (root_push result)
      (let [pat (parse-pattern-v3 spans pos-ref src)]
        (do
          (root_push pat)
          (let [next-result (vector-push result pat)]
            (do
              (root_push next-result)
              (let [parsed (parse-constructor-pattern-args-rooted-v3 spans pos-ref src next-result (+ count 1))]
                (do
                  (root_pop)
                  (root_pop)
                  (root_pop)
                  parsed)))))))))

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

(defn parse-recordpat-fields-rooted-v3 [spans pos-ref src result count]
  (if (== (p-current spans pos-ref) 5) ;; } で終了
    (do (p-advance pos-ref) result)
    (if (== (p-current spans pos-ref) 99) ;; EOF ガード: 無限ループ防止
      result
      (if (== (p-current spans pos-ref) 20)
        (do
          (root_push result)
          (let [field-hash (current-symbol-hash-v3 spans pos-ref src)]
            (do
              (p-advance pos-ref)
              (let [pat (parse-pattern-v3 spans pos-ref src)]
                (do
                  (root_push pat)
                  (let [next-result (vector-push-pair-rooted-v3 result field-hash pat)]
                    (do
                      (root_push next-result)
                      (let [parsed (parse-recordpat-fields-rooted-v3 spans pos-ref src next-result (+ count 1))]
                        (do
                          (root_pop)
                          (root_pop)
                          (root_pop)
                          parsed)))))))))
        (do
          (p-advance pos-ref)
          (parse-recordpat-fields-rooted-v3 spans pos-ref src result count))))))

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
    (if (== (p-current spans pos-ref) 20)
      (do
        (p-advance pos-ref) ;; type 名を最小 parity で消費
        0)
      0)
    (let [result (vector-push-pair-rooted-v3 (vector-new 8) (ast-pat-recordpat) 0)]
      (do
        (root_push result)
        (let [with-fields (parse-recordpat-fields-v3 spans pos-ref src result 0)
          field-count (/ (- (vector-length with-fields) 2) 2)]
          (do
            (root_push with-fields)
            (let [parsed (vector-set-at-rooted-v3 with-fields 1 field-count)]
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

;; match の腕を収集
(defn parse-match-arms-rooted-v3 [spans pos-ref src result count]
  (if (== (p-current spans pos-ref) 1) ;; ) で終了
    (do (p-advance pos-ref) result)
    (if (== (p-current spans pos-ref) 99) ;; EOF ガード: 無限ループ防止
      result
      (if (== (p-current spans pos-ref) 2) ;; [ -> arm
        (do
          (root_push result)
          (p-advance pos-ref) ;; [ を消費
          (let [pat (parse-pattern-v3 spans pos-ref src)
            body (parse-expr-v3 spans pos-ref src)]
            (do
              (root_push pat)
              (root_push body)
              (p-expect spans pos-ref 3) ;; ] を消費
              (let [next-result (vector-push-pair-rooted-v3 result pat body)]
                (do
                  (root_push next-result)
                  (let [parsed (parse-match-arms-rooted-v3 spans pos-ref src next-result (+ count 1))]
                    (do
                      (root_pop)
                      (root_pop)
                      (root_pop)
                      (root_pop)
                      parsed)))))))
        ;; 不正なトークン -> スキップ
        (do (p-advance pos-ref)
          (parse-match-arms-rooted-v3 spans pos-ref src result count))))))

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

(defn parse-params-rooted-v3 [spans pos-ref src result count]
  (if (== (p-current spans pos-ref) 3) ;; ] で終了
    (do (p-advance pos-ref) result)
    (if (== (p-current spans pos-ref) 99) ;; EOF ガード: 無限ループ防止
      result
      (do
        (root_push result)
        (let [h (parse-param-hash-v3 spans pos-ref src)]
          (do
            (root_push h)
            (let [next-result (vector-push result h)]
              (do
                (root_push next-result)
                (let [parsed (parse-params-rooted-v3 spans pos-ref src next-result (+ count 1))]
                  (do
                    (root_pop)
                    (root_pop)
                    (root_pop)
                    parsed))))))))))

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
            spans pos-ref src defn-node param-count meta))
        (parse-defn-bodyless-or-body-v3
          spans pos-ref src defn-node param-count))]
      (do
        (root_pop)
        parsed))))

(defn parse-defn-v3 [spans pos-ref src]
  (do
    (p-advance pos-ref) ;; defn を消費
    (let [ns (p-start spans pos-ref)
      ne (p-end spans pos-ref)
      nh (name-hash src ns ne)]
      (do
        (p-advance pos-ref) ;; name を消費
        (p-expect spans pos-ref 2) ;; [ を消費
        (let [result (vector-push-triple-rooted-v3 (vector-new 8) 20 nh 0)]
          (do
            (root_push result)
            (let [with-params (parse-params-v3 spans pos-ref src result 0)]
              (do
                (root_push with-params)
                (let [param-count (- (vector-length with-params) 3)
                  defn-node (vector-set-at-rooted-v3 with-params 2 param-count)]
                  (do
                    (root_push defn-node)
                    (skip-optional-type-sig-v3 spans pos-ref src)
                    (skip-optional-where-v3 spans pos-ref src)
                    (if (== (colon-directive-v3 spans pos-ref src) 1)
                      (let [meta (parse-defn-metadata-v3 spans pos-ref src)
                        meta-parsed (parse-defn-bodyless-or-body-with-meta-v3
                          spans pos-ref src defn-node param-count meta)]
                        (do
                          (root_pop)
                          (root_pop)
                          (root_pop)
                          meta-parsed))
                      (if (== (p-current spans pos-ref) 1)
                        (let [empty-body (make-int-node 0)]
                          (do
                            (root_push empty-body)
                            (let [empty-parsed (finalize-defn-body-v3 defn-node param-count empty-body)]
                              (do
                                (root_pop)
                                (root_pop)
                                (root_pop)
                                (root_pop)
                                empty-parsed))))
                        (let [parsed-body (parse-expr-v3 spans pos-ref src)]
                          (do
                            (root_push parsed-body)
                            (let [parsed-defn (finalize-defn-parsed-body-v3 spans pos-ref defn-node param-count parsed-body)]
                              (do
                                (root_push parsed-defn)
                                (if (> (string-length (command-line-arg 8)) 0)
                                  (do
                                    (print 224)
                                    (print param-count)
                                    (print (vector-get defn-node 0))
                                    (print (vector-length defn-node))
                                    (print (vector-get parsed-body 0))
                                    (print (vector-length parsed-body))
                                    (print (vector-get parsed-defn 0))
                                    (print (vector-length parsed-defn))
                                    (print (ref-get pos-ref)))
                                  (do))
                                (root_pop)
                                (root_pop)
                                (root_pop)
                                (root_pop)
                                (root_pop)
                                parsed-defn))))))))))))))))

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

;; === type 宣言 (簡易) ===
(defn parse-type-v3 [spans pos-ref src]
  (do
    (p-advance pos-ref) ;; type を消費
    (let [h (parse-type-head-hash-v3 spans pos-ref src)
      ;; いったん残りの variant / metadata は読み飛ばすが、
      ;; record 本体だけは RecordDef として識別する
      head-kind (if (== (p-current spans pos-ref) 0)
        (span-kind spans (+ (ref-get pos-ref) 1))
        0)]
      (do
        (parse-skip-to-close-v3 spans pos-ref 1)
        (if (== head-kind 39)
          (make-record-def h)
          (make-type-decl h))))))

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
            (make-module-decl name-h))]
          (do
            (root_push with-body)
            (let [parsed (vector-set-at-rooted-v3 with-body 2 (- (vector-length with-body) 3))]
              (do
                (root_pop)
                parsed))))))))

;; === import 宣言 ===
(defn parse-import-v3 [spans pos-ref src]
  (do
    (p-advance pos-ref) ;; import を消費
    (let [name-start (p-start spans pos-ref)
      name-end (p-end spans pos-ref)
      name-h (name-hash src name-start name-end)]
      (do
        (p-advance pos-ref) ;; name を消費
        (p-expect spans pos-ref 1) ;; ) を消費
        (make-import-decl name-h name-start name-end)))))

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

(defn parse-program-step-v3 [spans pos-ref src result]
  (if (== (p-current spans pos-ref) 99)
    (make-parse-loop-state 1 result)
    (do
      (let [parse-program-progress-mode (if (> (string-length (command-line-arg 8)) 0) 1 0)
        result-slot (root_push result)
        before-pos (ref-get pos-ref)
        before-kind (p-current spans pos-ref)
        head-kind (if (== before-kind 0) (span-kind spans (+ before-pos 1)) -1)
        result-len (vector-length result)
        expr (parse-expr-v3 spans pos-ref src)]
        (do
          (root_push expr)
          (if (= parse-program-progress-mode 1)
            (do
              (print 221)
              (print before-pos)
              (print before-kind)
              (print head-kind)
              (print (ref-get pos-ref))
              (print (vector-get expr 0))
              (print (vector-length expr)))
            (do))
          (let [next-result (vector-push-single-rooted-v3 result expr)]
            (do
              (if (= parse-program-progress-mode 1)
                (do
                  (print 222)
                  (print before-pos)
                  (print before-kind)
                  (print (ref-get pos-ref))
                  (print (vector-length next-result))
                  (print (vector-get (vector-get next-result result-len) 0)))
                (do))
              (let [state (do
                (root_set result-slot next-result)
                (make-parse-loop-state 0 next-result))]
                (do
                  (if (= parse-program-progress-mode 1)
                    (do
                      (root_push state)
                      (print 223)
                      (print before-pos)
                      (print before-kind)
                      (print (ref-get pos-ref))
                      (print (vector-length (vector-get state 1)))
                      (print (vector-get (vector-get (vector-get state 1) result-len) 0))
                      (root_pop))
                    (do))
                  (root_pop)
                  (root_pop)
                  state)))))))))

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
    (let [spans (tokenize-with-spans src)
      pos-ref (ref-new 0)]
      (do
        (root_push spans)
        (root_push pos-ref)
        (let [program (parse-program-v3 spans pos-ref src)]
          (do
            (root_pop)
            (root_pop)
            (root_pop)
            program))))))

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
