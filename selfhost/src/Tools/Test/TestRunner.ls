(module Tools.Test.TestRunner)
(import Syntax.AST)
(import Syntax.Lexer)
(import Syntax.Parser)
(import Syntax.Token)

;; TestRunner.ls - L# セルフホスティング: メタデータテストランナー
;;
;; :example は互換性のため source payload から抽出するが、:invariant は
;; parser が保持した defn metadata AST を test case へ投影する。
;; 算術・比較・if/let/do・トップレベル defn 呼び出しの subset を実行する。

(defn token-count [tokens]
  (/ (vector-length tokens) 3))

(defn token-kind [tokens n]
  (vector-get tokens (* n 3)))

(defn token-start [tokens n]
  (vector-get tokens (+ (* n 3) 1)))

(defn token-end [tokens n]
  (vector-get tokens (+ (* n 3) 2)))

(defn token-text [src tokens n]
  (substring src (token-start tokens n) (token-end tokens n)))

;; === テストケース構造 ===

;; テストケース: [name-id, function-name-hash, expr]
(defn make-test-case [name input expected]
  (vector-push
    (vector-push
      (vector-push (vector-new 3) name)
      input)
    expected))

;; テスト結果: [name-id, passed, actual, diagnostic-code]
(defn make-test-result [name passed actual]
  (vector-push
    (vector-push
      (vector-push
        (vector-push (vector-new 4) name)
        passed)
      actual)
    0))

(defn make-test-result-with-diagnostic [name passed actual diagnostic-code]
  (vector-push
    (vector-push
      (vector-push
        (vector-push (vector-new 4) name)
        passed)
      actual)
    diagnostic-code))

(defn make-suite [examples invariants]
  (vector-push
    (vector-push (vector-new 2) examples)
    invariants))

;; defn ノード末尾の metadata vector [doc, example, params, returns, invariant]
;; から parser-owned invariant AST を取り出す。
(defn test-defn-metadata [decl]
  (let [param-count (vector-get decl 2)
    meta-idx (+ 4 param-count)]
    (if (< meta-idx (vector-length decl))
      (vector-get decl meta-idx)
      0)))

(defn test-defn-invariant [decl]
  (let [meta (test-defn-metadata decl)]
    (if (= meta 0)
      0
      (if (> (vector-length meta) 4)
        (vector-get meta 4)
        0))))

(defn append-parser-invariant [decl results]
  (let [predicate (test-defn-invariant decl)]
    (if (= predicate 0)
      results
      (vector-push
        results
        (make-test-case
          (vector-length results)
          (vector-get decl 1)
          predicate)))))

;; parser AST から top-level defn の invariant test case を抽出する。
;; module/private declaration は既存 raw source scanner と同じくこの slice の対象外。
(defn extract-invariants-from-program-loop [program idx count results]
  (if (>= idx count)
    results
    (let [decl (vector-get program idx)
      next-results
      (if (= (vector-get decl 0) (ast-defn))
        (append-parser-invariant decl results)
        results)]
      (extract-invariants-from-program-loop
        program
        (+ idx 1)
        count
        next-results))))

(defn extract-invariants-from-program [program]
  (extract-invariants-from-program-loop
    program
    0
    (vector-length program)
    (vector-new 8)))

(defn test-result-diagnostic [result]
  (if (> (vector-length result) 3)
    (vector-get result 3)
    0))

(defn contract-diagnostic-undefined [] 1) ;; LS1001: undefined-variable

(defn diagnostic-count-loop [results idx count acc]
  (if (>= idx count)
    acc
    (let [code (test-result-diagnostic (vector-get results idx))]
      (diagnostic-count-loop results (+ idx 1) count
        (if (> code 0) (+ acc 1) acc)))))

(defn test-diagnostics-count [examples invariants]
  (+ (diagnostic-count-loop examples 0 (vector-length examples) 0)
    (diagnostic-count-loop invariants 0 (vector-length invariants) 0)))

(defn first-diagnostic-code-loop [results idx count]
  (if (>= idx count)
    0
    (let [code (test-result-diagnostic (vector-get results idx))]
      (if (> code 0)
        code
        (first-diagnostic-code-loop results (+ idx 1) count)))))

(defn first-test-diagnostic-code [examples invariants]
  (let [example-code (first-diagnostic-code-loop examples 0 (vector-length examples))]
    (if (> example-code 0)
      example-code
      (first-diagnostic-code-loop invariants 0 (vector-length invariants)))))

(defn test-diagnostic-code-text [code]
  (if (= code (contract-diagnostic-undefined))
    "LS1001"
    "LS0000"))

(defn test-diagnostics-summary [examples invariants]
  (let [count (test-diagnostics-count examples invariants)
    code (first-test-diagnostic-code examples invariants)]
    (if (= count 0)
      "diagnostics:0"
      (string-concat "diagnostics:"
        (string-concat (int-to-string count)
          (string-concat "," (test-diagnostic-code-text code)))))))

(defn test-hash-string [s]
  (name-hash s 0 (string-length s)))

(defn hash-result [] (test-hash-string "result"))
(defn hash-plus [] (test-hash-string "+"))
(defn hash-minus [] (test-hash-string "-"))
(defn hash-mul [] (test-hash-string "*"))
(defn hash-div [] (test-hash-string "/"))
(defn hash-mod [] (test-hash-string "%"))
(defn hash-eq [] (test-hash-string "="))
(defn hash-ne [] (test-hash-string "!="))
(defn hash-lt [] (test-hash-string "<"))
(defn hash-gt [] (test-hash-string ">"))
(defn hash-le [] (test-hash-string "<="))
(defn hash-ge [] (test-hash-string ">="))
(defn hash-and [] (test-hash-string "and"))
(defn hash-or [] (test-hash-string "or"))
(defn hash-not [] (test-hash-string "not"))

(defn value-int [n]
  (make-lit-int n))

(defn value-bool [b]
  (make-lit-bool b))

(defn value-unit []
  (make-lit-unit))

(defn value-tag [value]
  (if (= value 0)
    0
    (vector-get value 0)))

(defn value-int-or-bool [value]
  (if (= (value-tag value) (ast-lit-int))
    (vector-get value 1)
    (if (= (value-tag value) (ast-lit-bool))
      (vector-get value 1)
      0)))

(defn value-truthy [value]
  (if (= (value-int-or-bool value) 0) 0 1))

(defn values-equal [left right]
  (let [ltag (value-tag left)
    rtag (value-tag right)]
    (if (= ltag rtag)
      (if (= ltag (ast-lit-unit))
        1
        (if (= (vector-get left 1) (vector-get right 1)) 1 0))
      0)))

(defn env-new []
  (map-new))

(defn env-bind [env name-hash value]
  (map-insert env name-hash value))

(defn env-lookup [env name-hash]
  (let [value (map-get env name-hash)]
    (if (= value 0)
      (value-unit)
      value)))

(defn env-has? [env name-hash]
  (if (= (map-get env name-hash) 0) 0 1))

(defn builtin-hash-arith? [name-hash]
  (if (= name-hash (hash-plus)) 1
    (if (= name-hash (hash-minus)) 1
      (if (= name-hash (hash-mul)) 1
        (if (= name-hash (hash-div)) 1
          (if (= name-hash (hash-mod)) 1
            0))))))

(defn builtin-hash-compare? [name-hash]
  (if (= name-hash (hash-eq)) 1
    (if (= name-hash (hash-ne)) 1
      (if (= name-hash (hash-lt)) 1
        (if (= name-hash (hash-gt)) 1
          (if (= name-hash (hash-le)) 1
            (if (= name-hash (hash-ge)) 1
              0)))))))

(defn builtin-hash-logic? [name-hash]
  (if (= name-hash (hash-and)) 1
    (if (= name-hash (hash-or)) 1
      (if (= name-hash (hash-not)) 1
        0))))

(defn builtin-hash? [name-hash]
  (if (= (builtin-hash-arith? name-hash) 1)
    1
    (if (= (builtin-hash-compare? name-hash) 1)
      1
      (builtin-hash-logic? name-hash))))

(defn arg-value [args idx]
  (if (< idx (vector-length args))
    (vector-get args idx)
    (value-unit)))

(defn find-defn-by-hash [program target-hash idx count]
  (if (>= idx count)
    (vector-new 0)
    (let [decl (vector-get program idx)]
      (if (= (vector-get decl 0) (ast-defn))
        (if (= (vector-get decl 1) target-hash)
          decl
          (find-defn-by-hash program target-hash (+ idx 1) count))
        (find-defn-by-hash program target-hash (+ idx 1) count)))))

(defn known-contract-name? [program env name-hash]
  (if (= (env-has? env name-hash) 1)
    1
    (if (= (builtin-hash? name-hash) 1)
      1
      (if (= name-hash (hash-result))
        1
        (if (> (vector-length (find-defn-by-hash program name-hash 0 (vector-length program))) 0)
          1
          0)))))

(defn first-unknown-hash [left right]
  (if (>= left 0) left right))

(defn invariant-node-unknown-hash-args-loop [program node env idx count]
  (if (>= idx count)
    -1
    (let [found (invariant-node-unknown-hash program (vector-get node (+ 3 idx)) env)]
      (if (>= found 0)
        found
        (invariant-node-unknown-hash-args-loop program node env (+ idx 1) count)))))

(defn invariant-node-unknown-hash-do-loop [program node env idx count]
  (if (>= idx count)
    -1
    (let [found (invariant-node-unknown-hash program (vector-get node (+ 2 idx)) env)]
      (if (>= found 0)
        found
        (invariant-node-unknown-hash-do-loop program node env (+ idx 1) count)))))

(defn invariant-node-unknown-hash [program node env]
  (let [tag (vector-get node 0)]
    (if (= tag (ast-var))
      (if (= (known-contract-name? program env (vector-get node 1)) 1)
        -1
        (vector-get node 1))
      (if (= tag (ast-apply))
        (let [callee-found (invariant-node-unknown-hash program (vector-get node 1) env)]
          (if (>= callee-found 0)
            callee-found
            (invariant-node-unknown-hash-args-loop program node env 0 (vector-get node 2))))
        (if (= tag (ast-if))
          (let [cond-found (invariant-node-unknown-hash program (vector-get node 1) env)
            then-found (invariant-node-unknown-hash program (vector-get node 2) env)]
            (if (>= cond-found 0)
              cond-found
              (if (>= then-found 0)
                then-found
                (invariant-node-unknown-hash program (vector-get node 3) env))))
          (if (= tag (ast-let))
            (let [init-found (invariant-node-unknown-hash program (vector-get node 2) env)]
              (if (>= init-found 0)
                init-found
                (invariant-node-unknown-hash program
                  (vector-get node 3)
                  (env-bind env (vector-get node 1) (value-unit)))))
            (if (= tag (ast-do))
              (invariant-node-unknown-hash-do-loop program node env 0 (vector-get node 1))
              (if (= tag (ast-ann))
                (invariant-node-unknown-hash program (vector-get node 1) env)
                -1))))))))

(defn invariant-unknown-variable [program expr decl param-count]
  (let [scope (bind-params-loop
                (env-bind (env-new) (hash-result) (value-unit))
                decl
                (vector-new 0)
                0
                param-count)]
    (invariant-node-unknown-hash program expr scope)))

(defn bind-params-loop [env decl args idx count]
  (if (>= idx count)
    env
    (let [param-hash (vector-get decl (+ 3 idx))
      arg (arg-value args idx)]
      (bind-params-loop
        (env-bind env param-hash arg)
        decl
        args
        (+ idx 1)
        count))))

(defn eval-do-loop [program node env idx count last]
  (if (>= idx count)
    last
    (let [value (eval-node program (vector-get node (+ 2 idx)) env)]
      (eval-do-loop program node env (+ idx 1) count value))))

(defn eval-args-loop [program node env idx count results]
  (if (>= idx count)
    results
    (let [value (eval-node program (vector-get node (+ 3 idx)) env)]
      (eval-args-loop program node env (+ idx 1) count (vector-push results value)))))

(defn apply-builtin-arith [callee-hash args left right]
  (if (= callee-hash (hash-plus)) (value-int (+ left right))
    (if (= callee-hash (hash-minus))
      (if (= (vector-length args) 1)
        (value-int (- 0 left))
        (value-int (- left right)))
      (if (= callee-hash (hash-mul)) (value-int (* left right))
        (if (= callee-hash (hash-div))
          (if (= right 0)
            (value-int 0)
            (value-int (/ left right)))
          (if (= callee-hash (hash-mod))
            (if (= right 0)
              (value-int 0)
              (value-int (% left right)))
            0))))))

(defn apply-builtin-compare [callee-hash arg0 arg1 left right]
  (if (= callee-hash (hash-eq)) (value-bool (values-equal arg0 arg1))
    (if (= callee-hash (hash-ne)) (value-bool (if (= (values-equal arg0 arg1) 1) 0 1))
      (if (= callee-hash (hash-lt)) (value-bool (if (< left right) 1 0))
        (if (= callee-hash (hash-gt)) (value-bool (if (> left right) 1 0))
          (if (= callee-hash (hash-le)) (value-bool (if (<= left right) 1 0))
            (if (= callee-hash (hash-ge)) (value-bool (if (>= left right) 1 0))
              0)))))))

(defn apply-builtin-logic [callee-hash arg0 arg1]
  (if (= callee-hash (hash-and))
    (value-bool
      (if (= (value-truthy arg0) 1)
        (if (= (value-truthy arg1) 1) 1 0)
        0))
    (if (= callee-hash (hash-or))
      (value-bool
        (if (= (value-truthy arg0) 1)
          1
          (if (= (value-truthy arg1) 1) 1 0)))
      (if (= callee-hash (hash-not))
        (value-bool (if (= (value-truthy arg0) 1) 0 1))
        0))))

(defn apply-builtin [callee-hash args]
  (let [arg0 (arg-value args 0)
    arg1 (arg-value args 1)
    left (value-int-or-bool arg0)
    right (value-int-or-bool arg1)
    arith (apply-builtin-arith callee-hash args left right)]
    (if (= arith 0)
      (let [compare (apply-builtin-compare callee-hash arg0 arg1 left right)]
        (if (= compare 0)
          (let [logic (apply-builtin-logic callee-hash arg0 arg1)]
            (if (= logic 0)
              (value-unit)
              logic))
          compare))
      arith)))

(defn eval-defn-call [program decl args]
  (let [param-count (vector-get decl 2)
    env (bind-params-loop (env-new) decl args 0 param-count)
    body (vector-get decl (+ 3 param-count))]
    (eval-node program body env)))

(defn eval-apply [program node env]
  (let [callee (vector-get node 1)
    argc (vector-get node 2)
    args (eval-args-loop program node env 0 argc (vector-new (+ argc 1)))]
    (if (= (vector-get callee 0) (ast-var))
      (let [callee-hash (vector-get callee 1)]
        (if (= (builtin-hash? callee-hash) 1)
          (apply-builtin callee-hash args)
          (let [decl (find-defn-by-hash program callee-hash 0 (vector-length program))]
            (if (> (vector-length decl) 0)
              (eval-defn-call program decl args)
              (value-unit)))))
      (value-unit))))

(defn eval-node [program node env]
  (let [tag (vector-get node 0)]
    (if (= tag (ast-lit-int))
      node
      (if (= tag (ast-lit-bool))
        node
        (if (= tag (ast-lit-unit))
          node
          (if (= tag (ast-var))
            (env-lookup env (vector-get node 1))
            (if (= tag (ast-if))
              (let [cond-value (eval-node program (vector-get node 1) env)]
                (if (= (value-truthy cond-value) 1)
                  (eval-node program (vector-get node 2) env)
                  (eval-node program (vector-get node 3) env)))
              (if (= tag (ast-let))
                (let [name-hash (vector-get node 1)
                  init-value (eval-node program (vector-get node 2) env)
                  body-env (env-bind env name-hash init-value)]
                  (eval-node program (vector-get node 3) body-env))
                (if (= tag (ast-do))
                  (eval-do-loop program node env 0 (vector-get node 1) (value-unit))
                  (if (= tag (ast-ann))
                    (eval-node program (vector-get node 1) env)
                    (if (= tag (ast-apply))
                      (eval-apply program node env)
                      (value-unit))))))))))))

(defn depth-total [paren-depth bracket-depth brace-depth]
  (+ (+ paren-depth bracket-depth) brace-depth))

(defn step-paren-depth [kind depth]
  (if (= kind (tok-lparen))
    (+ depth 1)
    (if (= kind (tok-rparen))
      (- depth 1)
      depth)))

(defn step-bracket-depth [kind depth]
  (if (= kind (tok-lbracket))
    (+ depth 1)
    (if (= kind (tok-rbracket))
      (- depth 1)
      depth)))

(defn step-brace-depth [kind depth]
  (if (= kind (tok-lbrace))
    (+ depth 1)
    (if (= kind (tok-rbrace))
      (- depth 1)
      depth)))

(defn consume-form-loop [tokens idx n paren-depth bracket-depth brace-depth]
  (if (>= idx n)
    idx
    (if (<= (depth-total paren-depth bracket-depth brace-depth) 0)
      idx
      (let [kind (token-kind tokens idx)
        next-paren (step-paren-depth kind paren-depth)
        next-bracket (step-bracket-depth kind bracket-depth)
        next-brace (step-brace-depth kind brace-depth)]
        (consume-form-loop tokens (+ idx 1) n next-paren next-bracket next-brace)))))

(defn consume-form [tokens idx]
  (if (>= idx (token-count tokens))
    idx
    (let [kind (token-kind tokens idx)]
      (if (= kind (tok-lparen))
        (consume-form-loop tokens (+ idx 1) (token-count tokens) 1 0 0)
        (if (= kind (tok-lbracket))
          (consume-form-loop tokens (+ idx 1) (token-count tokens) 0 1 0)
          (if (= kind (tok-lbrace))
            (consume-form-loop tokens (+ idx 1) (token-count tokens) 0 0 1)
            (+ idx 1)))))))

(defn at-defn-top-level [paren-depth bracket-depth brace-depth]
  (if (= paren-depth 1)
    (if (= bracket-depth 0)
      (if (= brace-depth 0) 1 0)
      0)
    0))

(defn directive-name [src tokens idx end]
  (if (< (+ idx 1) end)
    (let [next-kind (token-kind tokens (+ idx 1))]
      (if (= next-kind (tok-symbol))
        (token-text src tokens (+ idx 1))
        (if (= next-kind (tok-where))
          (token-text src tokens (+ idx 1))
          "")))
    ""))

(defn payload-source [src tokens payload-start payload-end]
  (if (>= payload-start payload-end)
    ""
    (let [kind (token-kind tokens payload-start)]
      (if (= kind (tok-lbracket))
        (substring src
          (token-end tokens payload-start)
          (token-start tokens (- payload-end 1)))
        (substring src
          (token-start tokens payload-start)
          (token-end tokens (- payload-end 1)))))))

(defn supported-test-directive? [name]
  (if (string-eq name "example")
    1
    (if (string-eq name "invariant")
      1
      0)))

(defn append-skip-span [spans start end]
  (vector-push-pair-rooted spans start end))

(defn collect-defn-test-skip-spans-loop [src tokens idx end spans paren-depth bracket-depth brace-depth]
  (if (>= idx end)
    spans
    (let [kind (token-kind tokens idx)]
      (if (= kind (tok-eof))
        spans
        (if (= kind (tok-lbracket))
          (collect-defn-test-skip-spans-loop src tokens (consume-form tokens idx) end spans 1 0 0)
          (if (= kind (tok-colon))
            (let [payload-start (+ idx 2)]
              (if (< payload-start end)
                (let [name (directive-name src tokens idx end)
                  payload-end (consume-form tokens payload-start)
                  next-spans (if (= (supported-test-directive? name) 1)
                    (append-skip-span spans
                      (token-start tokens idx)
                      (token-end tokens (- payload-end 1)))
                    spans)]
                  (do
                    (root_push next-spans)
                    (let [result (collect-defn-test-skip-spans-loop src tokens payload-end end next-spans 1 0 0)]
                      (do
                        (root_pop)
                        result))))
                spans))
            spans))))))

(defn collect-test-skip-spans-loop [src tokens idx count spans]
  (if (>= idx count)
    spans
    (let [kind (token-kind tokens idx)]
      (if (= kind (tok-eof))
        spans
        (let [next-idx (consume-form tokens idx)]
          (if (= kind (tok-lparen))
            (if (< (+ idx 2) count)
              (if (= (token-kind tokens (+ idx 1)) (tok-defn))
                (collect-test-skip-spans-loop src tokens next-idx count
                  (collect-defn-test-skip-spans-loop src tokens (+ idx 3) next-idx spans 1 0 0))
                (collect-test-skip-spans-loop src tokens next-idx count spans))
              (collect-test-skip-spans-loop src tokens next-idx count spans))
            (collect-test-skip-spans-loop src tokens next-idx count spans)))))))

(defn rebuild-source-with-skips-loop [src spans idx count last-pos out]
  (if (>= idx count)
    (string-concat out (substring src last-pos (string-length src)))
    (let [skip-start (vector-get spans (* idx 2))
      skip-end (vector-get spans (+ (* idx 2) 1))
      next-out (string-concat out (substring src last-pos skip-start))]
      (rebuild-source-with-skips-loop src spans (+ idx 1) count skip-end next-out))))

(defn strip-test-metadata [src]
  (let [tokens (tokenize-with-spans src)
    spans (collect-test-skip-spans-loop src tokens 0 (token-count tokens) (vector-new 8))]
    (rebuild-source-with-skips-loop src spans 0 (/ (vector-length spans) 2) 0 "")))

(defn append-parsed-cases-loop [exprs fn-hash idx count results]
  (if (>= idx count)
    results
    (append-parsed-cases-loop exprs fn-hash (+ idx 1) count
      (vector-push results
        (make-test-case (vector-length results) fn-hash (vector-get exprs idx))))))

(defn append-parsed-cases [exprs fn-hash results]
  (append-parsed-cases-loop exprs fn-hash 0 (vector-length exprs) results))

(defn append-example-payload [src tokens payload-start payload-end fn-hash results]
  (let [text (payload-source src tokens payload-start payload-end)]
    (if (> (string-length text) 0)
      (append-parsed-cases (parse-program text) fn-hash results)
      results)))

(defn append-invariant-payload [src tokens payload-start payload-end fn-hash results]
  (let [text (payload-source src tokens payload-start payload-end)]
    (if (> (string-length text) 0)
      (let [exprs (parse-program text)]
        (if (> (vector-length exprs) 0)
          (vector-push results
            (make-test-case (vector-length results) fn-hash (vector-get exprs 0)))
          results))
      results)))

(defn contract-form-example [] 1)
(defn contract-form-invariant [] 2)

;; ordered legacy form: [kind, function-name-hash, payload-expressions, start, end]
(defn make-contract-form [kind fn-hash payload start end]
  (vector-push
    (vector-push
      (vector-push
        (vector-push
          (vector-push (vector-new 5) kind)
          fn-hash)
        payload)
      start)
    end))

(defn append-contract-form [src tokens idx payload-start payload-end name fn-hash forms]
  (if (or (string-eq name "example") (string-eq name "invariant"))
    (let [text (payload-source src tokens payload-start payload-end)
      payload (if (> (string-length text) 0) (parse-program text) (vector-new 0))
      kind (if (string-eq name "example") (contract-form-example) (contract-form-invariant))
      start (token-start tokens idx)
      end (token-end tokens (- payload-end 1))
      form (make-contract-form kind fn-hash payload start end)]
      (vector-push forms form))
    forms))

(defn collect-defn-contract-forms-loop [src tokens idx end fn-hash forms]
  (if (>= idx end)
    forms
    (let [kind (token-kind tokens idx)]
      (if (= kind (tok-eof))
        forms
        (if (= kind (tok-lbracket))
          (collect-defn-contract-forms-loop src tokens (consume-form tokens idx) end fn-hash forms)
          (if (= kind (tok-colon))
            (let [payload-start (+ idx 2)]
              (if (< payload-start end)
                (let [name (directive-name src tokens idx end)
                  payload-end (consume-form tokens payload-start)
                  next-forms (append-contract-form
                    src tokens idx payload-start payload-end name fn-hash forms)]
                  (collect-defn-contract-forms-loop src tokens payload-end end fn-hash next-forms))
                forms))
            forms))))))

(defn extract-contract-forms-loop [src tokens idx count forms]
  (if (>= idx count)
    forms
    (let [kind (token-kind tokens idx)]
      (if (= kind (tok-eof))
        forms
        (let [next-idx (consume-form tokens idx)]
          (if (= kind (tok-lparen))
            (if (< (+ idx 2) count)
              (if (= (token-kind tokens (+ idx 1)) (tok-defn))
                (let [fn-hash (name-hash src (token-start tokens (+ idx 2)) (token-end tokens (+ idx 2)))
                  next-forms (collect-defn-contract-forms-loop src tokens (+ idx 3) next-idx fn-hash forms)]
                  (extract-contract-forms-loop src tokens next-idx count next-forms))
                (extract-contract-forms-loop src tokens next-idx count forms))
              (extract-contract-forms-loop src tokens next-idx count forms))
            (extract-contract-forms-loop src tokens next-idx count forms)))))))

(defn extract-contract-forms [src]
  (let [tokens (tokenize-with-spans src)]
    (extract-contract-forms-loop src tokens 0 (token-count tokens) (vector-new 8))))

(defn collect-defn-metadata-loop [src tokens idx end fn-hash examples invariants]
  (if (>= idx end)
    (make-suite examples invariants)
    (let [kind (token-kind tokens idx)]
      (if (= kind (tok-eof))
        (make-suite examples invariants)
        (if (= kind (tok-lbracket))
          (collect-defn-metadata-loop src tokens (consume-form tokens idx) end fn-hash examples invariants)
          (if (= kind (tok-colon))
            (let [payload-start (+ idx 2)]
              (if (< payload-start end)
                (let [name (directive-name src tokens idx end)
                  payload-end (consume-form tokens payload-start)
                  next-examples (if (string-eq name "example")
                    (append-example-payload src tokens payload-start payload-end fn-hash examples)
                    examples)
                  next-invariants (if (string-eq name "invariant")
                    (append-invariant-payload src tokens payload-start payload-end fn-hash invariants)
                    invariants)]
                  (collect-defn-metadata-loop src tokens payload-end end fn-hash next-examples next-invariants))
                (make-suite examples invariants)))
            (make-suite examples invariants)))))))

(defn extract-test-cases-loop [src tokens idx count examples invariants]
  (if (>= idx count)
    (make-suite examples invariants)
    (let [kind (token-kind tokens idx)]
      (if (= kind (tok-eof))
        (make-suite examples invariants)
        (let [next-idx (consume-form tokens idx)]
          (if (= kind (tok-lparen))
            (if (< (+ idx 2) count)
              (if (= (token-kind tokens (+ idx 1)) (tok-defn))
                (let [fn-hash (name-hash src (token-start tokens (+ idx 2)) (token-end tokens (+ idx 2)))
                  pair (collect-defn-metadata-loop src tokens (+ idx 3) next-idx fn-hash examples invariants)]
                  (extract-test-cases-loop src tokens next-idx count (vector-get pair 0) (vector-get pair 1)))
                (extract-test-cases-loop src tokens next-idx count examples invariants))
              (extract-test-cases-loop src tokens next-idx count examples invariants))
            (extract-test-cases-loop src tokens next-idx count examples invariants)))))))

(defn extract-test-cases [src]
  (let [tokens (tokenize-with-spans src)]
    (extract-test-cases-loop src tokens 0 (token-count tokens) (vector-new 8) (vector-new 8))))

;; === :example / :invariant テスト生成 ===

(defn extract-examples [src]
  (vector-get (extract-test-cases src) 0))

(defn extract-invariants [src]
  (vector-get (extract-test-cases src) 1))

(defn run-examples-loop [program test-cases idx count results]
  (if (>= idx count)
    results
    (let [tc (vector-get test-cases idx)
      name (vector-get tc 0)
      expr (vector-get tc 2)
      actual (eval-node program expr (env-new))
      passed (value-truthy actual)]
      (run-examples-loop program test-cases (+ idx 1) count
        (vector-push results (make-test-result name passed passed))))))

(defn run-examples [program test-cases]
  (run-examples-loop program test-cases 0 (vector-length test-cases) (vector-new 16)))

(defn invariant-sample-count [param-count]
  (if (= param-count 0)
    1
    (if (= param-count 1)
      5
      9)))

(defn invariant-sample-value [idx]
  (if (= idx 0)
    (value-int 0)
    (if (= idx 1)
      (value-int 1)
      (if (= idx 2)
        (value-int 5)
        (if (= idx 3)
          (value-int (- 0 1))
          (value-int 42))))))

(defn append-zero-invariant-args [args idx count]
  (if (>= idx count)
    args
    (append-zero-invariant-args
      (vector-push args (value-int 0))
      (+ idx 1)
      count)))

(defn invariant-sample-args [param-count sample-idx]
  (if (= param-count 0)
    (vector-new 0)
    (if (= param-count 1)
      (vector-push (vector-new 1) (invariant-sample-value sample-idx))
      (let [first (invariant-sample-value (/ sample-idx 3))
        second (invariant-sample-value (% sample-idx 3))
        with-first (vector-push (vector-new param-count) first)
        with-second (vector-push with-first second)]
        (append-zero-invariant-args with-second 2 param-count)))))

(defn eval-invariant-sample [program tc decl param-count sample-idx]
  (let [args (invariant-sample-args param-count sample-idx)
    result (eval-defn-call program decl args)
    param-env (bind-params-loop (env-new) decl args 0 param-count)
    invariant-env (env-bind param-env (hash-result) result)
    actual (eval-node program (vector-get tc 2) invariant-env)]
    (value-truthy actual)))

(defn run-invariant-samples-loop [program tc decl sample-idx sample-count all-passed]
  (if (>= sample-idx sample-count)
    all-passed
    (let [param-count (vector-get decl 2)
      passed (eval-invariant-sample program tc decl param-count sample-idx)
      next-passed (if (= passed 1) all-passed 0)]
      (run-invariant-samples-loop program tc decl (+ sample-idx 1) sample-count next-passed))))

(defn materialize-invariant [program tc]
  (let [name (vector-get tc 0)
    fn-hash (vector-get tc 1)
    decl (find-defn-by-hash program fn-hash 0 (vector-length program))
    sample-count (if (> (vector-length decl) 0)
      (invariant-sample-count (vector-get decl 2))
      0)
    unknown-hash (if (> (vector-length decl) 0)
      (invariant-unknown-variable program (vector-get tc 2) decl (vector-get decl 2))
      -1)
    diagnostic-code (if (>= unknown-hash 0)
      (contract-diagnostic-undefined)
      0)
    passed (if (= diagnostic-code 0)
      (if (> sample-count 0)
        (run-invariant-samples-loop program tc decl 0 sample-count 1)
        0)
      0)
    actual (if (= diagnostic-code 0) sample-count 0)]
    (make-test-result-with-diagnostic name passed actual diagnostic-code)))

(defn run-invariants-loop [program invariants idx count results]
  (if (>= idx count)
    results
    (run-invariants-loop
      program
      invariants
      (+ idx 1)
      count
      (vector-push results (materialize-invariant program (vector-get invariants idx))))))

(defn run-invariants [program invariants]
  (run-invariants-loop program invariants 0 (vector-length invariants) (vector-new (vector-length invariants))))

(defn count-passed-results-loop [results idx count acc]
  (if (>= idx count)
    acc
    (count-passed-results-loop results (+ idx 1) count
      (+ acc (vector-get (vector-get results idx) 1)))))

(defn count-passed-results [results]
  (count-passed-results-loop results 0 (vector-length results) 0))

(defn count-failed-results [results]
  (- (vector-length results) (count-passed-results results)))

;; generate-tests: source からテストスイート全体を生成・実行
(defn generate-tests [src]
  (let [program (parse-program src)
    cases (extract-test-cases src)
    examples (vector-get cases 0)
    invariants (extract-invariants-from-program program)
    example-results (run-examples program examples)
    invariant-results (run-invariants program invariants)]
    (make-suite example-results invariant-results)))

(defn generate-tests-from-source [src]
  (generate-tests src))

;; デモ用エントリポイント (テスト用)
(defn demo-main []
  (let [src "(defn abs [x] :example [(= (abs 5) 5)] :invariant (>= result 0) (if (< x 0) (- 0 x) x))"
    suite (generate-tests src)]
    (do
      (print (vector-length suite))
      (print (vector-length (vector-get suite 0)))
      (print (vector-length (vector-get suite 1)))
      0)))
