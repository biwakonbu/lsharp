(module Tools.Test.TestRunner)
(import Syntax.AST)
(import Syntax.Lexer)
(import Syntax.Parser)
(import Syntax.Token)
(import Tools.Test.PropertyRunner)

;; TestRunner.ls - L# セルフホスティング: メタデータテストランナー
;;
;; :example / :invariant / :case は parser が保持した defn metadata を test case へ投影する。
;; ordered forms がない旧 metadata には集約 payload の fallback を残す。
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

;; canonical :case: [name-id, actual-expr, expected-expr, diagnostic-code]
(defn make-case-test-case [name actual expected diagnostic-code]
  (vector-push-quad-rooted
    (vector-new 4)
    name
    actual
    expected
    diagnostic-code))

(defn append-case-test-case-rooted [results name actual expected diagnostic-code]
  (let [test-case (make-case-test-case name actual expected diagnostic-code)]
    (vector-push-single-rooted results test-case)))

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

(defn make-suite-with-assertions [examples invariants assertions]
  (vector-push
    (vector-push
      (vector-push (vector-new 3) examples)
      invariants)
    assertions))

(defn make-suite-with-cases [examples invariants assertions cases]
  (vector-push
    (vector-push
      (vector-push
        (vector-push (vector-new 4) examples)
        invariants)
      assertions)
    cases))

(defn make-suite-with-properties [examples invariants assertions cases properties]
  (vector-push-single-rooted
    (make-suite-with-cases examples invariants assertions cases)
    properties))

;; defn ノード末尾の metadata vector [doc, example, params, returns, invariant, ordered-forms]
;; から parser-owned invariant AST を取り出す。
(defn test-defn-signature-node? [candidate]
  (if (= candidate 0)
    0
    (if (= (vector-get candidate 0) (ast-defn-signature)) 1 0)))

(defn test-defn-metadata [decl]
  (let [param-count (vector-get decl 2)
    body-end (+ 4 param-count)
    meta-idx
      (if (< body-end (vector-length decl))
        (if (= (test-defn-signature-node? (vector-get decl body-end)) 1)
          (+ body-end 1)
          body-end)
        (vector-length decl))]
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

(defn test-defn-example-text [decl]
  (let [meta (test-defn-metadata decl)]
    (if (= meta 0)
      ""
      (if (> (vector-length meta) 1)
        (vector-get meta 1)
        ""))))

(defn test-defn-ordered-forms [decl]
  (let [meta (test-defn-metadata decl)]
    (if (= meta 0)
      0
      (if (> (vector-length meta) 5)
        (vector-get meta 5)
        0))))

;; parser-owned contract suite: [owner-hash, ordered-forms, executable-forms, pending-forms]
(defn make-parser-contract-suite [owner forms executable pending]
  (vector-push-quad-rooted
    (vector-new 4)
    owner
    forms
    executable
    pending))

(defn parser-contract-form-executable? [kind]
  (if (>= kind (contract-form-assert)) 1 0))

(defn parser-contract-form-pending? [kind]
  (if (or (= kind (contract-form-example)) (== kind (contract-form-invariant)))
    1
    0))

(defn partition-parser-contract-forms-loop
  [forms idx count executable pending]
  (if (>= idx count)
    (vector-push-pair-rooted (vector-new 2) executable pending)
    (let [form (vector-get forms idx)
      kind (vector-get form 0)
      next-executable (if (= (parser-contract-form-executable? kind) 1)
        (vector-push-single-rooted executable form)
        executable)
      next-pending (if (= (parser-contract-form-pending? kind) 1)
        (vector-push-single-rooted pending form)
        pending)]
      (do
        (root_push next-executable)
        (root_push next-pending)
        (let [result (partition-parser-contract-forms-loop
            forms
            (+ idx 1)
            count
            next-executable
            next-pending)]
          (do
            (root_pop)
            (root_pop)
            result))))))

(defn partition-parser-contract-forms [forms]
  (partition-parser-contract-forms-loop
    forms
    0
    (vector-length forms)
    (vector-new 0)
    (vector-new 0)))

(defn append-parser-contract-suite-from-decl [decl results]
  (let [tag (vector-get decl 0)]
    (if (= tag (ast-defn))
      (let [forms (test-defn-ordered-forms decl)]
        (if (= forms 0)
          results
          (do
            (root_push forms)
            (let [partitioned (partition-parser-contract-forms forms)]
              (do
                (root_push partitioned)
                (let [suite (make-parser-contract-suite
                    (vector-get decl 1)
                    forms
                    (vector-get partitioned 0)
                    (vector-get partitioned 1))
                  result (vector-push-single-rooted results suite)]
                  (do
                    (root_pop)
                    (root_pop)
                    result)))))))
      (if (= tag (ast-private))
        (append-parser-contract-suite-from-decl (vector-get decl 1) results)
        (if (= tag (ast-module-decl))
          (append-parser-contract-suites-from-module-loop
            decl
            0
            (vector-get decl 2)
            results)
          results)))))

(defn append-parser-contract-suites-from-module-loop [module-node idx count results]
  (if (>= idx count)
    results
    (let [next-results (append-parser-contract-suite-from-decl
        (vector-get module-node (+ idx 3))
        results)]
      (do
        (root_push next-results)
        (let [parsed (append-parser-contract-suites-from-module-loop
            module-node
            (+ idx 1)
            count
            next-results)]
          (do
            (root_pop)
            parsed))))))

(defn extract-parser-contract-suites-loop [program idx count results]
  (if (>= idx count)
    results
    (let [next-results (append-parser-contract-suite-from-decl
        (vector-get program idx)
        results)]
      (do
        (root_push next-results)
        (let [parsed (extract-parser-contract-suites-loop
            program
            (+ idx 1)
            count
            next-results)]
          (do
            (root_pop)
            parsed))))))

(defn extract-parser-contract-suites [src]
  (let [program (parse-program src)]
    (extract-parser-contract-suites-loop
      program
      0
      (vector-length program)
      (vector-new 0))))

;; 未対応 property profile は実行件数 0 の成功へ流さず、明示的な境界コードを返す。
(defn has-unsupported-property-form-loop [forms idx count]
  (if (>= idx count)
    0
    (let [form (vector-get forms idx)]
      (if (= (vector-get form 0) (contract-form-property))
        1
        (has-unsupported-property-form-loop forms (+ idx 1) count)))))

(defn has-unsupported-property-in-module-loop [module-node idx count]
  (if (>= idx count)
    0
    (if (= (has-unsupported-property-in-decl (vector-get module-node (+ idx 3))) 1)
      1
      (has-unsupported-property-in-module-loop module-node (+ idx 1) count))))

(defn has-unsupported-property-in-decl [decl]
  (let [tag (vector-get decl 0)]
    (if (= tag (ast-defn))
      (let [forms (test-defn-ordered-forms decl)]
        (if (= forms 0)
          0
          (has-unsupported-property-form-loop forms 0 (vector-length forms))))
      (if (= tag (ast-private))
        (has-unsupported-property-in-decl (vector-get decl 1))
        (if (= tag (ast-module-decl))
          (has-unsupported-property-in-module-loop
            decl
            0
            (vector-get decl 2))
          0)))))

(defn has-unsupported-property-in-program-loop [program idx count]
  (if (>= idx count)
    0
    (if (= (has-unsupported-property-in-decl (vector-get program idx)) 1)
      1
      (has-unsupported-property-in-program-loop program (+ idx 1) count))))

(defn has-unsupported-property-in-program? [program]
  (has-unsupported-property-in-program-loop
    program
    0
    (vector-length program)))

(defn contract-diagnostic-unsupported-property [] 3002) ;; LS3002: unimplemented property runner

(defn metadata-test-runner-boundary-code [program]
  (property-runner-boundary-code program))

(defn append-parser-ordered-invariant-form [form decl results]
  (if (= (vector-get form 0) 2)
    (vector-push
      results
      (make-test-case
        (vector-length results)
        (vector-get decl 1)
        (vector-get form 1)))
    results))

(defn append-parser-ordered-invariants-loop [forms idx count decl results]
  (if (>= idx count)
    results
    (let [form (vector-get forms idx)
      next-results (append-parser-ordered-invariant-form form decl results)]
      (append-parser-ordered-invariants-loop
        forms
        (+ idx 1)
        count
        decl
        next-results))))

(defn append-parser-invariant [decl results]
  (let [forms (test-defn-ordered-forms decl)]
    (if (= forms 0)
      (let [predicate (test-defn-invariant decl)]
        (if (= predicate 0)
          results
          (vector-push
            results
            (make-test-case
              (vector-length results)
              (vector-get decl 1)
              predicate))))
      (append-parser-ordered-invariants-loop
        forms
        0
        (vector-length forms)
        decl
        results))))

;; parser AST から declaration tree 内の defn invariant test case を抽出する。
(defn append-parser-invariants-from-module-loop [module-node idx count results]
  (if (>= idx count)
    results
    (append-parser-invariants-from-module-loop
      module-node
      (+ idx 1)
      count
      (append-parser-invariants-from-decl
        (vector-get module-node (+ idx 3))
        results))))

(defn append-parser-invariants-from-decl [decl results]
  (let [tag (vector-get decl 0)]
    (if (= tag (ast-defn))
      (append-parser-invariant decl results)
      (if (= tag (ast-private))
        (append-parser-invariants-from-decl (vector-get decl 1) results)
        (if (= tag (ast-module-decl))
          (append-parser-invariants-from-module-loop
            decl
            0
            (vector-get decl 2)
            results)
          results)))))

;; parser AST から defn の invariant test case を declaration tree 順に抽出する。
(defn extract-invariants-from-program-loop [program idx count results]
  (if (>= idx count)
    results
    (let [next-results (append-parser-invariants-from-decl
        (vector-get program idx)
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

;; parser-owned canonical :assert form [3, predicate-vector] を assertion case へ投影する。
(defn append-parser-assertion-predicates-loop [predicates idx count decl results]
  (if (>= idx count)
    results
    (append-parser-assertion-predicates-loop
      predicates
      (+ idx 1)
      count
      decl
      (vector-push
        results
        (make-test-case
          (vector-length results)
          (vector-get decl 1)
          (vector-get predicates idx))))))

(defn append-parser-ordered-assertion-form [form decl results]
  (if (= (vector-get form 0) (contract-form-assert))
    (let [predicates (vector-get form 1)]
      (append-parser-assertion-predicates-loop
        predicates
        0
        (vector-length predicates)
        decl
        results))
    results))

(defn append-parser-ordered-assertions-loop [forms idx count decl results]
  (if (>= idx count)
    results
    (let [form (vector-get forms idx)
      next-results (append-parser-ordered-assertion-form form decl results)]
      (append-parser-ordered-assertions-loop
        forms
        (+ idx 1)
        count
        decl
        next-results))))

(defn append-parser-assertions [decl results]
  (let [forms (test-defn-ordered-forms decl)]
    (if (= forms 0)
      results
      (append-parser-ordered-assertions-loop
        forms
        0
        (vector-length forms)
        decl
        results))))

(defn append-parser-assertions-from-module-loop [module-node idx count results]
  (if (>= idx count)
    results
    (append-parser-assertions-from-module-loop
      module-node
      (+ idx 1)
      count
      (append-parser-assertions-from-decl
        (vector-get module-node (+ idx 3))
        results))))

(defn append-parser-assertions-from-decl [decl results]
  (let [tag (vector-get decl 0)]
    (if (= tag (ast-defn))
      (append-parser-assertions decl results)
      (if (= tag (ast-private))
        (append-parser-assertions-from-decl (vector-get decl 1) results)
        (if (= tag (ast-module-decl))
          (append-parser-assertions-from-module-loop
            decl
            0
            (vector-get decl 2)
            results)
          results)))))

(defn extract-assertions-from-program-loop [program idx count results]
  (if (>= idx count)
    results
    (let [next-results (append-parser-assertions-from-decl
        (vector-get program idx)
        results)]
      (extract-assertions-from-program-loop
        program
        (+ idx 1)
        count
        next-results))))

(defn extract-assertions-from-program [program]
  (extract-assertions-from-program-loop
    program
    0
    (vector-length program)
    (vector-new 8)))

;; parser-owned canonical :case form [4, [[actual, expected] ...]] を case へ投影する。
(defn append-parser-case-expectations-loop [expectations idx count results]
  (if (>= idx count)
    results
    (let [pair (vector-get expectations idx)
      actual (vector-get pair 0)
      expected (vector-get pair 1)]
      (append-parser-case-expectations-loop
        expectations
        (+ idx 1)
        count
        (append-case-test-case-rooted
          results
          (vector-length results)
          actual
          expected
          0)))))

(defn append-parser-ordered-case-form [form results]
  (if (= (vector-get form 0) (contract-form-case))
    (let [expectations (vector-get form 1)]
      (if (= (vector-length expectations) 0)
        (append-case-test-case-rooted
          results
          (vector-length results)
          (value-unit)
          (value-unit)
          (contract-diagnostic-empty-case))
        (append-parser-case-expectations-loop
          expectations
          0
          (vector-length expectations)
          results)))
    results))

(defn append-parser-ordered-cases-loop [forms idx count results]
  (if (>= idx count)
    results
    (append-parser-ordered-cases-loop
      forms
      (+ idx 1)
      count
      (append-parser-ordered-case-form (vector-get forms idx) results))))

(defn append-parser-cases [decl results]
  (let [forms (test-defn-ordered-forms decl)]
    (if (= forms 0)
      results
      (append-parser-ordered-cases-loop
        forms
        0
        (vector-length forms)
        results))))

(defn append-parser-cases-from-module-loop [module-node idx count results]
  (if (>= idx count)
    results
    (append-parser-cases-from-module-loop
      module-node
      (+ idx 1)
      count
      (append-parser-cases-from-decl
        (vector-get module-node (+ idx 3))
        results))))

(defn append-parser-cases-from-decl [decl results]
  (let [tag (vector-get decl 0)]
    (if (= tag (ast-defn))
      (append-parser-cases decl results)
      (if (= tag (ast-private))
        (append-parser-cases-from-decl (vector-get decl 1) results)
        (if (= tag (ast-module-decl))
          (append-parser-cases-from-module-loop
            decl
            0
            (vector-get decl 2)
            results)
          results)))))

(defn extract-cases-from-program-loop [program idx count results]
  (if (>= idx count)
    results
    (let [next-results (append-parser-cases-from-decl
        (vector-get program idx)
        results)]
      (extract-cases-from-program-loop
        program
        (+ idx 1)
        count
        next-results))))

(defn extract-cases-from-program [program]
  (extract-cases-from-program-loop
    program
    0
    (vector-length program)
    (vector-new 8)))

(defn test-result-diagnostic [result]
  (if (> (vector-length result) 3)
    (vector-get result 3)
    0))

(defn contract-diagnostic-undefined [] 1) ;; LS1001: undefined-variable
(defn contract-diagnostic-non-bool [] 2) ;; LS1002: invariant-predicate-must-be-bool
(defn contract-diagnostic-empty-case [] 2006) ;; LS2006: empty-case-contract

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
    (if (= code (contract-diagnostic-non-bool))
      "LS1002"
      (if (= code (contract-diagnostic-empty-case))
        "LS2006"
        (if (= code (contract-diagnostic-unsupported-property))
          "LS3002"
          "LS0000")))))

(defn test-diagnostics-summary [examples invariants]
  (let [count (test-diagnostics-count examples invariants)
    code (first-test-diagnostic-code examples invariants)]
    (if (= count 0)
      "diagnostics:0"
      (string-concat "diagnostics:"
        (string-concat (int-to-string count)
          (string-concat "," (test-diagnostic-code-text code)))))))

(defn test-diagnostics-count-with-assertions [examples invariants assertions]
  (+ (test-diagnostics-count examples invariants)
    (diagnostic-count-loop assertions 0 (vector-length assertions) 0)))

(defn first-test-diagnostic-code-with-assertions [examples invariants assertions]
  (let [code (first-test-diagnostic-code examples invariants)]
    (if (> code 0)
      code
      (first-diagnostic-code-loop assertions 0 (vector-length assertions)))))

(defn test-diagnostics-summary-with-assertions [examples invariants assertions]
  (let [count (test-diagnostics-count-with-assertions examples invariants assertions)
    code (first-test-diagnostic-code-with-assertions examples invariants assertions)]
    (if (= count 0)
      "diagnostics:0"
      (string-concat "diagnostics:"
        (string-concat (int-to-string count)
          (string-concat "," (test-diagnostic-code-text code)))))))

(defn test-diagnostics-count-with-cases [examples invariants assertions cases]
  (+ (test-diagnostics-count-with-assertions examples invariants assertions)
    (diagnostic-count-loop cases 0 (vector-length cases) 0)))

(defn first-test-diagnostic-code-with-cases [examples invariants assertions cases]
  (let [code (first-test-diagnostic-code-with-assertions examples invariants assertions)]
    (if (> code 0)
      code
      (first-diagnostic-code-loop cases 0 (vector-length cases)))))

(defn test-diagnostics-summary-with-cases [examples invariants assertions cases]
  (let [count (test-diagnostics-count-with-cases examples invariants assertions cases)
    code (first-test-diagnostic-code-with-cases examples invariants assertions cases)]
    (if (= count 0)
      "diagnostics:0"
      (string-concat "diagnostics:"
        (string-concat (int-to-string count)
          (string-concat "," (test-diagnostic-code-text code)))))))

(defn test-diagnostics-count-with-properties [examples invariants assertions cases properties]
  (+ (test-diagnostics-count-with-cases examples invariants assertions cases)
    (diagnostic-count-loop properties 0 (vector-length properties) 0)))

(defn first-test-diagnostic-code-with-properties [examples invariants assertions cases properties]
  (let [code (first-test-diagnostic-code-with-cases examples invariants assertions cases)]
    (if (> code 0)
      code
      (first-diagnostic-code-loop properties 0 (vector-length properties)))))

(defn test-diagnostics-summary-with-properties [examples invariants assertions cases properties]
  (let [count (test-diagnostics-count-with-properties
      examples
      invariants
      assertions
      cases
      properties)
    code (first-test-diagnostic-code-with-properties
      examples
      invariants
      assertions
      cases
      properties)]
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

(defn find-defn-in-module-body-loop [module-node target-hash idx count]
  (if (>= idx count)
    (vector-new 0)
    (let [found (find-defn-in-decl-by-hash
        (vector-get module-node (+ idx 3))
        target-hash)]
      (if (> (vector-length found) 0)
        found
        (find-defn-in-module-body-loop
          module-node
          target-hash
          (+ idx 1)
          count)))))

(defn find-defn-in-decl-by-hash [decl target-hash]
  (let [tag (vector-get decl 0)]
    (if (= tag (ast-defn))
      (if (= (vector-get decl 1) target-hash)
        decl
        (vector-new 0))
      (if (= tag (ast-private))
        (find-defn-in-decl-by-hash (vector-get decl 1) target-hash)
        (if (= tag (ast-module-decl))
          (find-defn-in-module-body-loop
            decl
            target-hash
            0
            (vector-get decl 2))
          (vector-new 0))))))

(defn find-defn-by-hash [program target-hash idx count]
  (if (>= idx count)
    (vector-new 0)
    (let [found (find-defn-in-decl-by-hash
        (vector-get program idx)
        target-hash)]
      (if (> (vector-length found) 0)
        found
        (find-defn-by-hash program target-hash (+ idx 1) count)))))

(defn known-contract-name? [program env name-hash allow-result]
  (if (= (env-has? env name-hash) 1)
    1
    (if (= (builtin-hash? name-hash) 1)
      1
      (if (= name-hash (hash-result))
        allow-result
        (if (> (vector-length (find-defn-by-hash program name-hash 0 (vector-length program))) 0)
          1
          0)))))

(defn first-unknown-hash [left right]
  (if (>= left 0) left right))

(defn contract-node-unknown-hash-args-loop [program node env allow-result idx count]
  (if (>= idx count)
    -1
    (let [found (contract-node-unknown-hash program (vector-get node (+ 3 idx)) env allow-result)]
      (if (>= found 0)
        found
        (contract-node-unknown-hash-args-loop program node env allow-result (+ idx 1) count)))))

(defn contract-node-unknown-hash-do-loop [program node env allow-result idx count]
  (if (>= idx count)
    -1
    (let [found (contract-node-unknown-hash program (vector-get node (+ 2 idx)) env allow-result)]
      (if (>= found 0)
        found
        (contract-node-unknown-hash-do-loop program node env allow-result (+ idx 1) count)))))

(defn contract-node-unknown-hash-computation-loop [program node env allow-result idx count]
  (if (>= idx count)
    -1
    (let [step-base (+ 3 (* idx 3))
      step-kind (vector-get node step-base)
      step-found (contract-node-unknown-hash
        program
        (vector-get node (+ step-base 2))
        env
        allow-result)]
      (if (>= step-found 0)
        step-found
        (let [next-env (if (= step-kind (computation-step-let-bang))
          (env-bind env (vector-get node (+ step-base 1)) (value-unit))
          env)]
          (contract-node-unknown-hash-computation-loop
            program
            node
            next-env
            allow-result
            (+ idx 1)
            count))))))

(defn contract-bind-pattern-vars-constructor-loop [env pattern idx count]
  (if (>= idx count)
    env
    (contract-bind-pattern-vars-constructor-loop
      (contract-bind-pattern-vars env (vector-get pattern (+ 3 idx)))
      pattern
      (+ idx 1)
      count)))

(defn contract-bind-pattern-vars-record-loop [env pattern idx count]
  (if (>= idx count)
    env
    (contract-bind-pattern-vars-record-loop
      (contract-bind-pattern-vars env (vector-get pattern (+ 3 (* idx 2))))
      pattern
      (+ idx 1)
      count)))

(defn contract-bind-pattern-vars [env pattern]
  (let [tag (vector-get pattern 0)]
    (if (= tag (ast-pat-var))
      (env-bind env (vector-get pattern 1) (value-unit))
      (if (= tag (ast-pat-constructor))
        (contract-bind-pattern-vars-constructor-loop
          env
          pattern
          0
          (vector-get pattern 2))
        (if (= tag (ast-pat-recordpat))
          (contract-bind-pattern-vars-record-loop
            env
            pattern
            0
            (vector-get pattern 1))
          env)))))

(defn contract-node-unknown-hash-match-loop [program node env allow-result idx count]
  (if (>= idx count)
    -1
    (let [arm-base (+ 3 (* idx 2))
      arm-env (contract-bind-pattern-vars env (vector-get node arm-base))
      found (contract-node-unknown-hash
        program
        (vector-get node (+ arm-base 1))
        arm-env
        allow-result)]
      (if (>= found 0)
        found
        (contract-node-unknown-hash-match-loop
          program
          node
          env
          allow-result
          (+ idx 1)
          count)))))

(defn contract-bind-lambda-params-loop [env node idx count]
  (if (>= idx count)
    env
    (contract-bind-lambda-params-loop
      (env-bind env (vector-get node (+ 2 idx)) (value-unit))
      node
      (+ idx 1)
      count)))

(defn contract-node-unknown-hash [program node env allow-result]
  (let [tag (vector-get node 0)]
    (if (= tag (ast-var))
      (if (= (known-contract-name? program env (vector-get node 1) allow-result) 1)
        -1
        (vector-get node 1))
      (if (= tag (ast-apply))
        (let [callee-found (contract-node-unknown-hash program (vector-get node 1) env allow-result)]
          (if (>= callee-found 0)
            callee-found
            (contract-node-unknown-hash-args-loop program node env allow-result 0 (vector-get node 2))))
        (if (= tag (ast-if))
          (let [cond-found (contract-node-unknown-hash program (vector-get node 1) env allow-result)
            then-found (contract-node-unknown-hash program (vector-get node 2) env allow-result)]
            (if (>= cond-found 0)
              cond-found
              (if (>= then-found 0)
                then-found
                (contract-node-unknown-hash program (vector-get node 3) env allow-result))))
          (if (= tag (ast-let))
            (let [init-found (contract-node-unknown-hash program (vector-get node 2) env allow-result)]
              (if (>= init-found 0)
                init-found
                (contract-node-unknown-hash program (vector-get node 3)
                  (env-bind env (vector-get node 1) (value-unit)) allow-result)))
              (if (= tag (ast-lambda))
              (let [param-count (vector-get node 1)
                lambda-env (contract-bind-lambda-params-loop env node 0 param-count)]
                (contract-node-unknown-hash
                  program
                  (vector-get node (+ 2 param-count))
                  lambda-env
                  allow-result))
              (if (= tag (ast-match))
                (let [scrutinee-found (contract-node-unknown-hash program (vector-get node 1) env allow-result)]
                  (if (>= scrutinee-found 0)
                    scrutinee-found
                    (contract-node-unknown-hash-match-loop
                      program
                      node
                      env
                      allow-result
                      0
                      (vector-get node 2))))
                (if (= tag (ast-computation))
                  (contract-node-unknown-hash-computation-loop
                    program
                    node
                    env
                    allow-result
                    0
                    (vector-get node 2))
                  (if (= tag (ast-do))
                    (contract-node-unknown-hash-do-loop program node env allow-result 0 (vector-get node 1))
                    (if (= tag (ast-ann))
                      (contract-node-unknown-hash program (vector-get node 1) env allow-result)
                      -1)))))))))))

(defn invariant-unknown-variable [program expr decl param-count]
  (let [scope (bind-params-loop
                (env-bind (env-new) (hash-result) (value-unit))
                decl
                (vector-new 0)
                0
                param-count)]
    (contract-node-unknown-hash program expr scope 1)))

;; canonical :case は owner の引数や result を暗黙に束縛しない。
(defn case-unknown-variable [program actual expected]
  (let [scope (env-new)
    actual-found (contract-node-unknown-hash program actual scope 0)]
    (if (>= actual-found 0)
      actual-found
      (contract-node-unknown-hash program expected scope 0))))

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

;; 移行期 contract evaluator の match subset。
;; literal / wildcard / variable pattern だけを扱い、constructor/record は未対応境界に残す。
(defn match-pattern? [pattern value]
  (let [tag (vector-get pattern 0)]
    (if (= tag (ast-pat-wildcard))
      1
      (if (= tag (ast-pat-var))
        1
        (if (= tag (ast-pat-lit))
          (values-equal value (vector-get pattern 1))
          0)))))

(defn match-bind-pattern [env pattern value]
  (if (= (vector-get pattern 0) (ast-pat-var))
    (env-bind env (vector-get pattern 1) value)
    env))

(defn eval-match-loop [program node env value idx count]
  (if (>= idx count)
    (value-unit)
    (let [arm-base (+ 3 (* idx 2))
      pattern (vector-get node arm-base)
      body (vector-get node (+ arm-base 1))]
      (if (= (match-pattern? pattern value) 1)
        (eval-node program body (match-bind-pattern env pattern value))
        (eval-match-loop program node env value (+ idx 1) count)))))

(defn eval-match [program node env]
  (let [value (eval-node program (vector-get node 1) env)]
    (eval-match-loop program node env value 0 (vector-get node 2))))

;; 移行期 contract evaluator の computation subset。
;; identity 相当の builder では、各 step の値を順に評価して let! だけ環境へ束縛する。
(defn eval-computation-loop [program node env idx count last]
  (if (>= idx count)
    last
    (let [step-base (+ 3 (* idx 3))
      step-kind (vector-get node step-base)
      aux (vector-get node (+ step-base 1))
      expr (vector-get node (+ step-base 2))
      value (eval-node program expr env)
      next-env (if (= step-kind (computation-step-let-bang))
        (env-bind env aux value)
        env)]
      (eval-computation-loop
        program
        node
        next-env
        (+ idx 1)
        count
        value))))

(defn eval-computation [program node env]
  (eval-computation-loop
    program
    node
    env
    0
    (vector-get node 2)
    (value-unit)))

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
                  (if (= tag (ast-match))
                    (eval-match program node env)
                    (if (= tag (ast-do))
                  (eval-do-loop program node env 0 (vector-get node 1) (value-unit))
                  (if (= tag (ast-computation))
                    (eval-computation program node env)
                    (if (= tag (ast-ann))
                      (eval-node program (vector-get node 1) env)
                      (if (= tag (ast-apply))
                        (eval-apply program node env)
                        (value-unit))))))))))))))

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
      (if (string-eq name "case")
        1
        (if (string-eq name "assert")
          1
          (if (string-eq name "property") 1 0))))))

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

(defn append-parser-ordered-example-form [form decl results]
  (if (= (vector-get form 0) 1)
    (let [payload (vector-get form 1)]
      (if (> (string-length payload) 0)
        (append-parsed-cases
          (parse-program payload)
          (vector-get decl 1)
          results)
        results))
    results))

(defn append-parser-ordered-examples-loop [forms idx count decl results]
  (if (>= idx count)
    results
    (let [form (vector-get forms idx)
      next-results (append-parser-ordered-example-form form decl results)]
      (append-parser-ordered-examples-loop
        forms
        (+ idx 1)
        count
        decl
        next-results))))

(defn append-parser-examples [decl results]
  (let [forms (test-defn-ordered-forms decl)]
    (if (= forms 0)
      (let [text (test-defn-example-text decl)]
        (if (> (string-length text) 0)
          (append-parsed-cases
            (parse-program text)
            (vector-get decl 1)
            results)
          results))
      (append-parser-ordered-examples-loop
        forms
        0
        (vector-length forms)
        decl
        results))))

;; parser が保持する defn metadata の example payload を test case へ投影する。
;; payload は互換のため文字列で保持し、ここで AST に再パースする。
(defn append-parser-examples-from-module-loop [module-node idx count results]
  (if (>= idx count)
    results
    (append-parser-examples-from-module-loop
      module-node
      (+ idx 1)
      count
      (append-parser-examples-from-decl
        (vector-get module-node (+ idx 3))
        results))))

(defn append-parser-examples-from-decl [decl results]
  (let [tag (vector-get decl 0)]
    (if (= tag (ast-defn))
      (append-parser-examples decl results)
      (if (= tag (ast-private))
        (append-parser-examples-from-decl (vector-get decl 1) results)
        (if (= tag (ast-module-decl))
          (append-parser-examples-from-module-loop
            decl
            0
            (vector-get decl 2)
            results)
          results)))))

(defn extract-examples-from-program-loop [program idx count results]
  (if (>= idx count)
    results
    (let [next-results (append-parser-examples-from-decl
        (vector-get program idx)
        results)]
      (extract-examples-from-program-loop
        program
        (+ idx 1)
        count
        next-results))))

(defn extract-examples-from-program [program]
  (extract-examples-from-program-loop
    program
    0
    (vector-length program)
    (vector-new 8)))

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

;; ordered contract form: [kind, function-name-hash, payload, start, end]
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

(defn contract-form-kind [name]
  (if (string-eq name "example")
    (contract-form-example)
    (if (string-eq name "invariant")
      (contract-form-invariant)
      (if (string-eq name "assert")
        (contract-form-assert)
        (if (string-eq name "case")
          (contract-form-case)
          (if (string-eq name "property")
            (contract-form-property)
            0))))))

(defn contract-form-payload [src tokens payload-start payload-end name]
  (let [text (payload-source src tokens payload-start payload-end)]
    (if (or (string-eq name "example") (string-eq name "invariant"))
      (if (> (string-length text) 0) (parse-program text) (vector-new 0))
      (vector-push-single-rooted (vector-new 1) text))))

(defn append-contract-form [src tokens idx payload-start payload-end name fn-hash forms]
  (let [kind (contract-form-kind name)]
    (if (> kind 0)
      (let [payload (contract-form-payload src tokens payload-start payload-end name)
            start (token-start tokens idx)
            end (token-end tokens (- payload-end 1))
            form (make-contract-form kind fn-hash payload start end)]
        (vector-push forms form))
      forms)))

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

(defn run-assertions-loop [program test-cases idx count results]
  (if (>= idx count)
    results
    (let [tc (vector-get test-cases idx)
      name (vector-get tc 0)
      expr (vector-get tc 2)
      actual (eval-node program expr (env-new))
      bool-valid (if (= (value-tag actual) (ast-lit-bool)) 1 0)
      passed (if (= bool-valid 1) (value-truthy actual) 0)
      diagnostic-code (if (= bool-valid 1) 0 (contract-diagnostic-non-bool))]
      (run-assertions-loop
        program
        test-cases
        (+ idx 1)
        count
        (vector-push
          results
          (make-test-result-with-diagnostic
            name
            passed
            passed
            diagnostic-code))))))

(defn run-assertions [program test-cases]
  (run-assertions-loop
    program
    test-cases
    0
    (vector-length test-cases)
    (vector-new (vector-length test-cases))))

(defn case-test-diagnostic [test-case]
  (if (> (vector-length test-case) 3)
    (vector-get test-case 3)
    0))

(defn run-cases-loop [program test-cases idx count results]
  (if (>= idx count)
    results
    (let [test-case (vector-get test-cases idx)
      name (vector-get test-case 0)
      diagnostic-code (case-test-diagnostic test-case)]
      (if (> diagnostic-code 0)
        (run-cases-loop
          program
          test-cases
          (+ idx 1)
          count
          (vector-push
            results
            (make-test-result-with-diagnostic name 0 0 diagnostic-code)))
        (let [actual-expr (vector-get test-case 1)
          expected-expr (vector-get test-case 2)
          unknown-hash (case-unknown-variable program actual-expr expected-expr)]
          (if (>= unknown-hash 0)
            (run-cases-loop
              program
              test-cases
              (+ idx 1)
              count
              (vector-push
                results
                (make-test-result-with-diagnostic
                  name
                  0
                  0
                  (contract-diagnostic-undefined))))
            (let [actual (eval-node program actual-expr (env-new))
              expected (eval-node program expected-expr (env-new))
              passed (values-equal actual expected)]
              (run-cases-loop
                program
                test-cases
                (+ idx 1)
                count
                (vector-push
                  results
                  (make-test-result-with-diagnostic
                    name
                    passed
                    (value-int-or-bool actual)
                    0))))))))))

(defn run-cases [program test-cases]
  (run-cases-loop
    program
    test-cases
    0
    (vector-length test-cases)
    (vector-new (vector-length test-cases))))

;; 移行期 property smoke profile は legacy invariant と同じ固定値を使う。
(defn property-sample-value [idx]
  (if (= idx 0)
    (value-int 0)
    (if (= idx 1)
      (value-int 1)
      (if (= idx 2)
        (value-int 5)
        (if (= idx 3)
          (value-int (- 0 1))
          (value-int 42))))))

(defn property-unknown-variable [program test-case]
  (let [env (env-bind
      (env-bind
        (env-new)
        (property-test-case-binder test-case)
        (value-unit))
      (hash-result)
      (value-unit))]
    (contract-node-unknown-hash
      program
      (property-test-case-postcondition test-case)
      env
      1)))

(defn eval-property-sample-value [program test-case decl sample-idx]
  (let [sample (property-sample-value sample-idx)
    args (vector-push-single-rooted (vector-new 1) sample)
    result (eval-defn-call program decl args)
    owner-env (bind-params-loop (env-new) decl args 0 (vector-get decl 2))
    property-env (env-bind
      (env-bind
        owner-env
        (property-test-case-binder test-case)
        sample)
      (hash-result)
      result)]
    (eval-node
      program
      (property-test-case-postcondition test-case)
      property-env)))

(defn property-sample-summary [passed bool-valid]
  (vector-push
    (vector-push (vector-new 2) passed)
    bool-valid))

(defn run-property-samples-summary-loop
  [program test-case decl sample-idx sample-count all-passed all-bool]
  (if (>= sample-idx sample-count)
    (property-sample-summary all-passed all-bool)
    (let [actual (eval-property-sample-value program test-case decl sample-idx)
      bool-valid (if (= (value-tag actual) (ast-lit-bool)) 1 0)
      passed (if (= bool-valid 1) (value-truthy actual) 0)
      next-passed (if (= passed 1) all-passed 0)
      next-bool (if (= bool-valid 1) all-bool 0)]
      (run-property-samples-summary-loop
        program
        test-case
        decl
        (+ sample-idx 1)
        sample-count
        next-passed
        next-bool))))

(defn materialize-property [program test-case]
  (let [name (vector-get test-case 0)
    owner (property-test-case-owner test-case)
    decl (find-defn-by-hash program owner 0 (vector-length program))
    profile-code (property-test-case-profile-code test-case)
    owner-valid (if (and (> (vector-length decl) 0) (= (vector-get decl 2) 1)) 1 0)
    unknown-hash (if (and (= profile-code 0) (= owner-valid 1))
      (property-unknown-variable program test-case)
      -1)
    sample-count (property-test-case-count test-case)
    sample-summary (if (or (> profile-code 0) (or (= owner-valid 0) (>= unknown-hash 0)))
      (property-sample-summary 0 0)
      (run-property-samples-summary-loop
        program
        test-case
        decl
        0
        sample-count
        1
        1))
    bool-valid (vector-get sample-summary 1)
    diagnostic-code (if (> profile-code 0)
      profile-code
      (if (= owner-valid 0)
        (contract-diagnostic-unsupported-property)
        (if (>= unknown-hash 0)
          (contract-diagnostic-undefined)
          (if (= bool-valid 1) 0 (contract-diagnostic-non-bool)))))
    passed (if (= diagnostic-code 0) (vector-get sample-summary 0) 0)
    actual (if (= diagnostic-code 0) sample-count 0)]
    (make-test-result-with-diagnostic name passed actual diagnostic-code)))

(defn run-properties-loop [program test-cases idx count results]
  (if (>= idx count)
    results
    (run-properties-loop
      program
      test-cases
      (+ idx 1)
      count
      (vector-push
        results
        (materialize-property program (vector-get test-cases idx))))))

(defn run-properties [program test-cases]
  (run-properties-loop
    program
    test-cases
    0
    (vector-length test-cases)
    (vector-new (vector-length test-cases))))

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
  (value-truthy (eval-invariant-sample-value program tc decl param-count sample-idx)))

(defn eval-invariant-sample-value [program tc decl param-count sample-idx]
  (let [args (invariant-sample-args param-count sample-idx)
    result (eval-defn-call program decl args)
    param-env (bind-params-loop (env-new) decl args 0 param-count)
    invariant-env (env-bind param-env (hash-result) result)
    actual (eval-node program (vector-get tc 2) invariant-env)]
    actual))

(defn invariant-sample-summary [passed bool-valid]
  (vector-push
    (vector-push (vector-new 2) passed)
    bool-valid))

(defn run-invariant-samples-loop [program tc decl sample-idx sample-count all-passed]
  (if (>= sample-idx sample-count)
    all-passed
    (let [param-count (vector-get decl 2)
      passed (eval-invariant-sample program tc decl param-count sample-idx)
      next-passed (if (= passed 1) all-passed 0)]
      (run-invariant-samples-loop program tc decl (+ sample-idx 1) sample-count next-passed))))

(defn run-invariant-sample-summary-loop
  [program tc decl sample-idx sample-count all-passed all-bool]
  (if (>= sample-idx sample-count)
    (invariant-sample-summary all-passed all-bool)
    (let [param-count (vector-get decl 2)
      actual (eval-invariant-sample-value
        program
        tc
        decl
        param-count
        sample-idx)
      bool-valid (if (= (value-tag actual) (ast-lit-bool)) 1 0)
      passed (if (= bool-valid 1) (value-truthy actual) 0)
      next-passed (if (= passed 1) all-passed 0)
      next-bool (if (= bool-valid 1) all-bool 0)]
      (run-invariant-sample-summary-loop
        program
        tc
        decl
        (+ sample-idx 1)
        sample-count
        next-passed
        next-bool))))

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
    sample-summary (if (>= unknown-hash 0)
      (invariant-sample-summary 0 0)
      (if (> sample-count 0)
        (run-invariant-sample-summary-loop
          program
          tc
          decl
          0
          sample-count
          1
          1)
        (invariant-sample-summary 0 1)))
    type-valid (vector-get sample-summary 1)
    diagnostic-code (if (>= unknown-hash 0)
      (contract-diagnostic-undefined)
      (if (= type-valid 1)
        0
        (contract-diagnostic-non-bool)))
    passed (if (= diagnostic-code 0)
      (vector-get sample-summary 0)
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
    examples (extract-examples-from-program program)
    invariants (extract-invariants-from-program program)
    assertions (extract-assertions-from-program program)
    cases (extract-cases-from-program program)
    properties (extract-property-test-cases program)
    example-results (run-examples program examples)
    invariant-results (run-invariants program invariants)
    assertion-results (run-assertions program assertions)
    case-results (run-cases program cases)
    property-results (run-properties program properties)]
    (make-suite-with-properties
      example-results
      invariant-results
      assertion-results
      case-results
      property-results)))

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
