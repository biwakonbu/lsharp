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
  (vector-push-triple-rooted (vector-new 3) name input expected))

;; canonical :assert: [name-id, function-name-hash, expr, span-start, span-end]
(defn make-assertion-test-case [name input predicate span-start span-end]
  (let [base (vector-push-quad-rooted
      (vector-new 5)
      name
      input
      predicate
      span-start)]
    (vector-push-single-rooted base span-end)))

;; canonical :case: [name-id, actual-expr, expected-expr, diagnostic-code,
;;                   actual-start, actual-end, expected-start, expected-end]
(defn make-case-test-case
  [name actual expected diagnostic-code actual-start actual-end expected-start expected-end]
  (let [base (vector-push-quad-rooted
      (vector-new 8)
      name
      actual
      expected
      diagnostic-code)]
    (do
      (root_push base)
      (let [result (vector-push-quad-rooted
          base
          actual-start
          actual-end
          expected-start
          expected-end)]
        (do
          (root_pop)
          result)))))

(defn append-case-test-case-rooted
  [results name actual expected diagnostic-code actual-start actual-end expected-start expected-end]
  (let [test-case (make-case-test-case
      name
      actual
      expected
      diagnostic-code
      actual-start
      actual-end
      expected-start
      expected-end)]
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

;; 診断結果: [name-id, passed, actual, diagnostic-code, span-start, span-end]
(defn make-test-result-with-diagnostic-span
  [name passed actual diagnostic-code span-start span-end]
  (vector-push
    (vector-push
      (vector-push
        (vector-push
          (vector-push
            (vector-push (vector-new 6) name)
            passed)
          actual)
        diagnostic-code)
      span-start)
    span-end))

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

(defn make-parser-contract-form [form owner src]
  (let [kind (vector-get form 0)
    start (if (> (vector-length form) 2) (vector-get form 2) 0)
    end (if (> (vector-length form) 3) (vector-get form 3) 0)
    payload (if (= kind (contract-form-property))
      (property-runner-form-typed-payload-with-source form owner src)
      (if (> (vector-length form) 1) (vector-get form 1) 0))
    base-form (vector-push-quad-rooted (vector-new 4) kind payload start end)]
    (if (and
        (= kind (contract-form-assert))
        (> (vector-length form) 4))
      (do
        (root_push base-form)
        (let [with-extra (vector-push-single-rooted
            base-form
            (vector-get form 4))]
          (do
            (root_pop)
            with-extra)))
      base-form)))

(defn partition-parser-contract-forms-loop
  [forms idx count owner executable pending src]
  (if (>= idx count)
    (vector-push-pair-rooted (vector-new 2) executable pending)
    (let [form (vector-get forms idx)
      kind (vector-get form 0)
      next-executable (if (= (parser-contract-form-executable? kind) 1)
        (vector-push-single-rooted
          executable
          (make-parser-contract-form form owner src))
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
            owner
            next-executable
            next-pending
            src)]
          (do
            (root_pop)
            (root_pop)
            result))))))

(defn partition-parser-contract-forms [forms owner src]
  (partition-parser-contract-forms-loop
    forms
    0
    (vector-length forms)
    owner
    (vector-new 0)
    (vector-new 0)
    src))

(defn append-parser-contract-suite-from-decl [decl results src]
  (let [tag (vector-get decl 0)]
    (if (= tag (ast-defn))
      (let [forms (test-defn-ordered-forms decl)]
        (if (= forms 0)
          results
          (do
            (root_push forms)
            (let [partitioned
                (partition-parser-contract-forms forms (vector-get decl 1) src)]
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
        (append-parser-contract-suite-from-decl (vector-get decl 1) results src)
        (if (= tag (ast-module-decl))
          (append-parser-contract-suites-from-module-loop
            decl
            0
            (vector-get decl 2)
            results
            src)
          results)))))

(defn append-parser-contract-suites-from-module-loop
  [module-node idx count results src]
  (if (>= idx count)
    results
    (let [next-results (append-parser-contract-suite-from-decl
        (vector-get module-node (+ idx 3))
        results
        src)]
      (do
        (root_push next-results)
        (let [parsed (append-parser-contract-suites-from-module-loop
            module-node
            (+ idx 1)
            count
            next-results
            src)]
          (do
            (root_pop)
            parsed))))))

(defn extract-parser-contract-suites-loop [program idx count results src]
  (if (>= idx count)
    results
    (let [next-results (append-parser-contract-suite-from-decl
        (vector-get program idx)
        results
        src)]
      (do
        (root_push next-results)
        (let [parsed (extract-parser-contract-suites-loop
            program
            (+ idx 1)
            count
            next-results
            src)]
          (do
            (root_pop)
            parsed))))))

(defn extract-parser-contract-suites [src]
  (let [program (parse-program src)]
    (extract-parser-contract-suites-loop
      program
      0
      (vector-length program)
      (vector-new 0)
      src)))

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
(defn contract-diagnostic-vacuous-property [] 2005) ;; LS2005: vacuous property

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

;; parser-owned canonical :assert form [3, predicate-vector, ..., spans] を assertion case へ投影する。
(defn append-parser-assertion-predicates-loop
  [predicates spans idx count decl results]
  (if (>= idx count)
    results
    (let [span-count (if (= spans 0) 0 (vector-length spans))
      span-index (* idx 2)
      predicate-start
        (if (> span-count span-index) (vector-get spans span-index) 0)
      predicate-end
        (if (> span-count (+ span-index 1))
          (vector-get spans (+ span-index 1))
          0)]
      (append-parser-assertion-predicates-loop
        predicates
        spans
        (+ idx 1)
        count
        decl
        (vector-push
          results
          (make-assertion-test-case
            (vector-length results)
            (vector-get decl 1)
            (vector-get predicates idx)
            predicate-start
            predicate-end))))))

(defn append-parser-ordered-assertion-form [form decl results]
  (if (= (vector-get form 0) (contract-form-assert))
    (let [predicates (vector-get form 1)
      spans (if (> (vector-length form) 4) (vector-get form 4) 0)]
      (append-parser-assertion-predicates-loop
        predicates
        spans
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

;; parser-owned canonical :case form [4, [[actual, expected, entry-start, entry-end,
;; actual-start, actual-end, expected-start, expected-end] ...]] を case へ投影する。
(defn append-parser-case-expectations-loop [expectations idx count results]
  (if (>= idx count)
    results
    (let [pair (vector-get expectations idx)
      actual (vector-get pair 0)
      expected (vector-get pair 1)
      actual-start (if (> (vector-length pair) 4) (vector-get pair 4) 0)
      actual-end (if (> (vector-length pair) 5) (vector-get pair 5) 0)
      expected-start (if (> (vector-length pair) 6) (vector-get pair 6) 0)
      expected-end (if (> (vector-length pair) 7) (vector-get pair 7) 0)]
      (append-parser-case-expectations-loop
        expectations
        (+ idx 1)
        count
        (append-case-test-case-rooted
          results
          (vector-length results)
          actual
          expected
          0
          actual-start
          actual-end
          expected-start
          expected-end)))))

(defn append-parser-ordered-case-form [form results]
  (if (= (vector-get form 0) (contract-form-case))
    (let [expectations (vector-get form 1)]
      (if (= (vector-length expectations) 0)
        (append-case-test-case-rooted
          results
          (vector-length results)
          (value-unit)
          (value-unit)
          (contract-diagnostic-empty-case)
          0
          0
          0
          0)
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

(defn test-result-diagnostic-start [result]
  (if (> (vector-length result) 4)
    (vector-get result 4)
    0))

(defn test-result-diagnostic-end [result]
  (if (> (vector-length result) 5)
    (vector-get result 5)
    0))

;; 最初の診断を [found, start, end] で返す。JSON report の source span 用。
(defn make-diagnostic-span-state [found start end]
  (vector-push
    (vector-push
      (vector-push (vector-new 3) found)
      start)
    end))

(defn first-diagnostic-span-loop [results idx count]
  (if (>= idx count)
    (make-diagnostic-span-state 0 0 0)
    (let [result (vector-get results idx)
      code (test-result-diagnostic result)]
      (if (> code 0)
        (make-diagnostic-span-state
          1
          (test-result-diagnostic-start result)
          (test-result-diagnostic-end result))
        (first-diagnostic-span-loop results (+ idx 1) count)))))

(defn first-diagnostic-span [results]
  (first-diagnostic-span-loop results 0 (vector-length results)))

(defn first-test-diagnostic-span-with-properties
  [examples invariants assertions cases properties]
  (let [example-span (first-diagnostic-span examples)]
    (if (= (vector-get example-span 0) 1)
      example-span
      (let [invariant-span (first-diagnostic-span invariants)]
        (if (= (vector-get invariant-span 0) 1)
          invariant-span
          (let [assertion-span (first-diagnostic-span assertions)]
            (if (= (vector-get assertion-span 0) 1)
              assertion-span
              (let [case-span (first-diagnostic-span cases)]
                (if (= (vector-get case-span 0) 1)
                  case-span
                  (first-diagnostic-span properties))))))))))

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
      (if (= code (contract-diagnostic-vacuous-property))
        "LS2005"
        (if (= code (contract-diagnostic-empty-case))
          "LS2006"
          (if (= code (contract-diagnostic-unsupported-property))
            "LS3002"
            "LS0000"))))))

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
(defn hash-not [] 109267)
(defn hash-string-eq [] (test-hash-string "string-eq"))

(defn value-int [n]
  (make-lit-int n))

(defn value-bool [b]
  (make-lit-bool b))

(defn value-string [text]
  (vector-push-pair-rooted (vector-new 2) (ast-lit-string) text))

(defn decode-string-escape [src idx end]
  (if (>= (+ idx 1) end)
    "\\"
    (let [escaped (string-char-at src (+ idx 1))]
      (if (= escaped 110) "\n"
        (if (= escaped 116) "\t"
          (if (= escaped 114) "\r"
            (if (= escaped 34) "\""
              (if (= escaped 92) "\\"
                (substring src (+ idx 1) (+ idx 2))))))))))

(defn decode-string-literal-loop [src idx end out]
  (if (>= idx end)
    out
    (if (= (string-char-at src idx) 92)
      (decode-string-literal-loop
        src
        (+ idx 2)
        end
        (string-concat out (decode-string-escape src idx end)))
      (decode-string-literal-loop
        src
        (+ idx 1)
        end
        (string-concat out (substring src idx (+ idx 1)))))))

(defn decode-string-literal [src start end]
  (decode-string-literal-loop src start end ""))

(defn value-string-node-with-source [node src]
  (value-string
    (decode-string-literal
      src
      (vector-get node 1)
      (vector-get node 2))))

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

(defn logic-bool-operands? [arg0 arg1]
  (if (= (value-tag arg0) (ast-lit-bool))
    (if (= (value-tag arg1) (ast-lit-bool)) 1 0)
    0))

(defn values-equal [left right]
  (let [ltag (value-tag left)
    rtag (value-tag right)]
    (if (= ltag rtag)
      (if (= ltag (ast-lit-unit))
        1
        (if (= ltag (ast-lit-string))
          (if (string-eq (vector-get left 1) (vector-get right 1)) 1 0)
          (if (= (vector-get left 1) (vector-get right 1)) 1 0)))
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

;; ADT constructor は evaluator 内で [tag, constructor-hash, arity, payload...] として保持する。
;; 型宣言に登録された constructor だけを値化し、未定義関数を誤って constructor として扱わない。
(defn find-constructor-in-variants-loop [variants target-hash idx count]
  (if (>= idx count)
    0
    (let [variant (vector-get variants idx)]
      (if (= (vector-get variant 0) target-hash)
        1
        (find-constructor-in-variants-loop
          variants
          target-hash
          (+ idx 1)
          count)))))

(defn find-constructor-in-module-body-loop [module-node target-hash idx count]
  (if (>= idx count)
    0
    (if (= (find-constructor-in-decl
        (vector-get module-node (+ idx 3))
        target-hash) 1)
      1
      (find-constructor-in-module-body-loop
        module-node
        target-hash
        (+ idx 1)
        count))))

(defn find-constructor-in-decl [decl target-hash]
  (let [tag (vector-get decl 0)]
    (if (= tag (ast-type-decl))
      (let [variants-index (if (> (vector-length decl) 3) 3 2)
        variants (vector-get decl variants-index)]
        (find-constructor-in-variants-loop
          variants
          target-hash
          0
          (vector-length variants)))
      (if (= tag (ast-private))
        (find-constructor-in-decl (vector-get decl 1) target-hash)
        (if (= tag (ast-module-decl))
          (find-constructor-in-module-body-loop
            decl
            target-hash
            0
            (vector-get decl 2))
          0)))))

(defn constructor-defined-loop [program target-hash idx count]
  (if (>= idx count)
    0
    (if (= (find-constructor-in-decl (vector-get program idx) target-hash) 1)
      1
      (constructor-defined-loop
        program
        target-hash
        (+ idx 1)
        count))))

(defn constructor-defined? [program target-hash]
  (constructor-defined-loop
    program
    target-hash
    0
    (vector-length program)))

(defn make-constructor-value-loop [base args idx count]
  (if (>= idx count)
    base
    (let [arg (vector-get args idx)]
      (do
        (root_push arg)
        (root_push base)
        (let [next (vector-push base arg)]
          (do
            (root_pop)
            (root_pop)
            (make-constructor-value-loop next args (+ idx 1) count)))))))

(defn make-constructor-value [constructor-hash args]
  (do
    (root_push args)
    (let [base (vector-push-triple-rooted
        (vector-new 3)
        (ast-pat-constructor)
        constructor-hash
        (vector-length args))]
      (do
        (root_push base)
        (let [result (make-constructor-value-loop
            base
            args
            0
            (vector-length args))]
          (do
            (root_pop)
            (root_pop)
            result))))))

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
      (if (= name-hash (hash-string-eq))
        1
        (builtin-hash-logic? name-hash)))))

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

(defn contract-node-unknown-hash-match-arm [program body env allow-result]
  (if (= (vector-get body 0) (ast-match-guard))
    (let [guard-found (contract-node-unknown-hash program (vector-get body 1) env allow-result)]
      (if (>= guard-found 0)
        guard-found
        (contract-node-unknown-hash program (vector-get body 2) env allow-result)))
    (contract-node-unknown-hash program body env allow-result)))

(defn contract-node-unknown-hash-match-loop [program node env allow-result idx count]
  (if (>= idx count)
    -1
    (let [arm-base (+ 3 (* idx 2))
      arm-env (contract-bind-pattern-vars env (vector-get node arm-base))
      found (contract-node-unknown-hash-match-arm
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

;; legacy invariant の root Bool 契約を、選択された sample だけでなく
;; match の未選択 armにも適用する。0=unknown、1=Bool、2=non-Bool。
(defn invariant-static-branch-kind [left right]
  (if (= left 2)
    2
    (if (= right 2)
      2
      (if (and (= left 1) (= right 1)) 1 0))))

(defn invariant-static-match-arm-kind [body]
  (if (= (vector-get body 0) (ast-match-guard))
    (let [guard-kind (invariant-static-bool-kind (vector-get body 1))
      body-kind (invariant-static-bool-kind (vector-get body 2))]
      (if (= guard-kind 2) 2 body-kind))
    (invariant-static-bool-kind body)))

(defn invariant-static-match-loop [node idx count all-bool]
  (if (>= idx count)
    all-bool
    (let [arm-base (+ 3 (* idx 2))
      kind (invariant-static-match-arm-kind (vector-get node (+ arm-base 1)))]
      (if (= kind 2)
        2
        (invariant-static-match-loop
          node
          (+ idx 1)
          count
          (if (= kind 1) all-bool 0))))))

(defn invariant-static-match-kind [node]
  (if (= (vector-get node 2) 0)
    2
    (invariant-static-match-loop node 0 (vector-get node 2) 1)))

(defn invariant-static-do-kind [node idx count]
  (if (>= idx count)
    2
    (if (= (+ idx 1) count)
      (invariant-static-bool-kind (vector-get node (+ 2 idx)))
      (invariant-static-do-kind node (+ idx 1) count))))

(defn invariant-static-computation-kind [node idx count]
  (if (>= idx count)
    2
    (let [step-base (+ 3 (* idx 3))]
      (if (= (+ idx 1) count)
        (invariant-static-bool-kind (vector-get node (+ step-base 2)))
        (invariant-static-computation-kind node (+ idx 1) count)))))

(defn invariant-static-logic-kind [node operator argc]
  (if (= operator (hash-not))
    (if (= argc 1)
      (let [operand-kind (invariant-static-bool-kind (vector-get node 3))]
        (if (= operand-kind 2) 2 (if (= operand-kind 1) 1 0)))
      0)
    (if (and (= argc 2) (or (= operator (hash-and)) (= operator (hash-or))))
      (let [left-kind (invariant-static-bool-kind (vector-get node 3))
        right-kind (invariant-static-bool-kind (vector-get node 4))]
        (if (or (= left-kind 2) (= right-kind 2))
          2
          (if (and (= left-kind 1) (= right-kind 1)) 1 0)))
      0)))

(defn invariant-static-apply-kind [node]
  (let [callee (vector-get node 1)
    argc (vector-get node 2)]
    (if (= (vector-get callee 0) (ast-var))
      (let [operator (vector-get callee 1)]
        (if (= (builtin-hash-arith? operator) 1)
          2
          (if (or (= (builtin-hash-compare? operator) 1)
              (= operator (hash-string-eq)))
            1
            (invariant-static-logic-kind node operator argc))))
      0)))

(defn invariant-static-bool-kind [node]
  (let [tag (vector-get node 0)]
    (if (= tag (ast-lit-bool))
      1
      (if (or (= tag (ast-lit-int))
          (or (= tag (ast-lit-string))
            (or (= tag (ast-lit-float)) (= tag (ast-lit-unit)))))
        2
        (if (= tag (ast-var))
          0
          (if (= tag (ast-apply))
            (invariant-static-apply-kind node)
            (if (= tag (ast-if))
              (let [condition-kind (invariant-static-bool-kind (vector-get node 1))
                branch-kind (invariant-static-branch-kind
                  (invariant-static-bool-kind (vector-get node 2))
                  (invariant-static-bool-kind (vector-get node 3)))]
                (if (= condition-kind 2) 2 branch-kind))
              (if (= tag (ast-let))
                (invariant-static-bool-kind (vector-get node 3))
                (if (= tag (ast-lambda))
                  2
                  (if (= tag (ast-do))
                    (invariant-static-do-kind node 0 (vector-get node 1))
                    (if (= tag (ast-match))
                      (invariant-static-match-kind node)
                      (if (= tag (ast-ann))
                        (invariant-static-bool-kind (vector-get node 1))
                        (if (or (= tag (ast-recordlit)) (= tag (ast-recordupdate)))
                          2
                          (if (= tag (ast-fieldaccess))
                            0
                            (if (= tag (ast-computation))
                              (invariant-static-computation-kind
                                node
                                0
                                (vector-get node 2))
                              (if (or (= tag (ast-quote)) (= tag (ast-unquote)))
                                2
                                (if (= tag (ast-unquote-splice)) 2 0)))))))))))))))))

(defn invariant-unknown-variable [program expr decl param-count]
  (let [scope (bind-params-loop
                (env-bind (env-new) (hash-result) (value-unit))
                decl
                (vector-new 0)
                0
                param-count)]
    (contract-node-unknown-hash program expr scope 1)))

;; canonical :case は owner の引数や result を暗黙に束縛しない。
;; 1=actual、2=expected、0=未検出を返し、診断 span の側を保持する。
(defn case-unknown-variable-side [program actual expected]
  (let [scope (env-new)
    actual-found (contract-node-unknown-hash program actual scope 0)]
    (if (>= actual-found 0)
      1
      (if (>= (contract-node-unknown-hash program expected scope 0) 0)
        2
        0))))

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
    (if (= (logic-bool-operands? arg0 arg1) 1)
      (value-bool
        (if (= (value-truthy arg0) 1)
          (if (= (value-truthy arg1) 1) 1 0)
          0))
      (value-int 0))
    (if (= callee-hash (hash-or))
      (if (= (logic-bool-operands? arg0 arg1) 1)
        (value-bool
          (if (= (value-truthy arg0) 1)
            1
            (if (= (value-truthy arg1) 1) 1 0)))
        (value-int 0))
      (if (= callee-hash (hash-not))
        (if (= (value-tag arg0) (ast-lit-bool))
          (value-bool (if (= (value-truthy arg0) 1) 0 1))
          (value-int 0))
        0))))

(defn apply-builtin-string [callee-hash arg0 arg1]
  (if (= callee-hash (hash-string-eq))
    (if (= (value-tag arg0) (ast-lit-string))
      (if (= (value-tag arg1) (ast-lit-string))
        (value-bool (if (string-eq (vector-get arg0 1) (vector-get arg1 1)) 1 0))
        (value-bool 0))
      (value-bool 0))
    0))

(defn apply-builtin [callee-hash args]
  (let [arg0 (arg-value args 0)
    arg1 (arg-value args 1)
    left (value-int-or-bool arg0)
    right (value-int-or-bool arg1)
    string-result (apply-builtin-string callee-hash arg0 arg1)]
    (if (= string-result 0)
      (let [arith (apply-builtin-arith callee-hash args left right)]
        (if (= arith 0)
          (let [compare (apply-builtin-compare callee-hash arg0 arg1 left right)]
            (if (= compare 0)
              (let [logic (apply-builtin-logic callee-hash arg0 arg1)]
                (if (= logic 0)
                  (value-unit)
                  logic))
              compare))
          arith))
      string-result)))

(defn eval-defn-call [program decl args]
  (let [param-count (vector-get decl 2)
    env (bind-params-loop (env-new) decl args 0 param-count)
    body (vector-get decl (+ 3 param-count))]
    (eval-node program body env)))

(defn make-record-value [type-hash field-count]
  (vector-push-triple-rooted
    (vector-new 3)
    (ast-recordlit)
    type-hash
    field-count))

(defn eval-record-fields-loop [program node env result idx count]
  (if (>= idx count)
    result
    (let [field-hash (vector-get node (+ 3 (* idx 2)))
      value (eval-node program (vector-get node (+ 4 (* idx 2))) env)
      next-result (vector-push-pair-rooted result field-hash value)]
      (eval-record-fields-loop
        program
        node
        env
        next-result
        (+ idx 1)
        count))))

(defn eval-record-literal [program node env]
  (eval-record-fields-loop
    program
    node
    env
    (make-record-value (vector-get node 1) (vector-get node 2))
    0
    (vector-get node 2)))

;; 移行期 contract evaluator の match subset。
;; literal / wildcard / variable に加え、1段の ADT constructor pattern を扱う。
(defn record-value-field-index-loop [value field-hash idx count]
  (if (>= idx count)
    -1
    (if (= (vector-get value (+ 3 (* idx 2))) field-hash)
      idx
      (record-value-field-index-loop value field-hash (+ idx 1) count))))

(defn record-value-field [value field-hash]
  (let [idx (record-value-field-index-loop
      value
      field-hash
      0
      (vector-get value 2))]
    (if (< idx 0)
      0
      (vector-get value (+ 4 (* idx 2))))))

(defn eval-field-access [program node env]
  (let [record-value (eval-node program (vector-get node 1) env)]
    (if (= (value-tag record-value) (ast-recordlit))
      (record-value-field record-value (vector-get node 2))
      (value-unit))))

(defn record-value-update-field [record-value field-hash new-value]
  (do
    (root_push record-value)
    (root_push new-value)
    (let [field-index (record-value-field-index-loop
        record-value
        field-hash
        0
        (vector-get record-value 2))
      updated (if (< field-index 0)
        record-value
        (vector-set-at
          record-value
          (+ 4 (* field-index 2))
          new-value))]
      (do
        (root_pop)
        (root_pop)
        updated))))

(defn eval-record-update-fields-loop
  [program node env record-value idx count]
  (if (>= idx count)
    record-value
    (do
      (root_push record-value)
      (let [field-hash (vector-get node (+ 3 (* idx 2)))
        new-value (eval-node program (vector-get node (+ 4 (* idx 2))) env)]
        (do
          (root_push new-value)
          (let [updated (record-value-update-field record-value field-hash new-value)]
            (do
              (root_pop)
              (root_pop)
              (eval-record-update-fields-loop
                program
                node
                env
                updated
                (+ idx 1)
                count))))))))

(defn eval-record-update [program node env]
  (let [record-value (eval-node program (vector-get node 1) env)]
    (if (= (value-tag record-value) (ast-recordlit))
      (eval-record-update-fields-loop
        program
        node
        env
        record-value
        0
        (vector-get node 2))
      (value-unit))))

(defn match-pattern-record-loop [pattern value idx count]
  (if (>= idx count)
    1
    (let [pattern-base (+ 2 (* idx 2))
      field-value (record-value-field
        value
        (vector-get pattern pattern-base))]
      (if (= field-value 0)
        0
        (if (= (match-pattern?
            (vector-get pattern (+ pattern-base 1))
            field-value) 1)
          (match-pattern-record-loop pattern value (+ idx 1) count)
          0)))))

(defn match-pattern-record [pattern value]
  (if (= (value-tag value) (ast-recordlit))
    (let [field-count (vector-get pattern 1)
      pattern-type-hash (vector-get pattern (+ 2 (* field-count 2)))]
      (if (= pattern-type-hash 0)
        (match-pattern-record-loop pattern value 0 field-count)
        (if (= pattern-type-hash (vector-get value 1))
          (match-pattern-record-loop pattern value 0 field-count)
          0)))
    0))

(defn match-pattern-constructor-loop [pattern value idx count]
  (if (>= idx count)
    1
    (if (= (match-pattern?
        (vector-get pattern (+ 3 idx))
        (vector-get value (+ 3 idx))) 1)
      (match-pattern-constructor-loop
        pattern
        value
        (+ idx 1)
        count)
      0)))

(defn match-pattern? [pattern value]
  (let [tag (vector-get pattern 0)]
    (if (= tag (ast-pat-wildcard))
      1
      (if (= tag (ast-pat-var))
        1
        (if (= tag (ast-pat-lit))
          (values-equal value (vector-get pattern 1))
          (if (= tag (ast-pat-constructor))
            (if (= (value-tag value) (ast-pat-constructor))
              (if (= (vector-get pattern 1) (vector-get value 1))
                (if (= (vector-get pattern 2) (vector-get value 2))
                  (match-pattern-constructor-loop
                    pattern
                    value
                    0
                    (vector-get pattern 2))
                  0)
                0)
              0)
            (if (= tag (ast-pat-recordpat))
              (match-pattern-record pattern value)
              0)))))))

(defn match-bind-pattern-record-loop [env pattern value idx count]
  (if (>= idx count)
    env
    (let [pattern-base (+ 2 (* idx 2))
      field-value (record-value-field
        value
        (vector-get pattern pattern-base))]
      (match-bind-pattern-record-loop
        (match-bind-pattern
          env
          (vector-get pattern (+ pattern-base 1))
          field-value)
        pattern
        value
        (+ idx 1)
        count))))

(defn match-bind-pattern-constructor-loop [env pattern value idx count]
  (if (>= idx count)
    env
    (match-bind-pattern-constructor-loop
      (match-bind-pattern
        env
        (vector-get pattern (+ 3 idx))
        (vector-get value (+ 3 idx)))
      pattern
      value
      (+ idx 1)
      count)))

(defn match-bind-pattern [env pattern value]
  (let [tag (vector-get pattern 0)]
    (if (= tag (ast-pat-var))
      (env-bind env (vector-get pattern 1) value)
      (if (= tag (ast-pat-constructor))
        (match-bind-pattern-constructor-loop
          env
          pattern
          value
          0
          (vector-get pattern 2))
        (if (= tag (ast-pat-recordpat))
          (match-bind-pattern-record-loop
            env
            pattern
            value
            0
            (vector-get pattern 1))
          env)))))

(defn eval-match-arm-body [program node env value pattern body idx count]
  (let [arm-env (match-bind-pattern env pattern value)]
    (if (= (vector-get body 0) (ast-match-guard))
      (let [guard-value (eval-node program (vector-get body 1) arm-env)]
        (if (= (value-truthy guard-value) 1)
          (eval-node program (vector-get body 2) arm-env)
          (eval-match-loop program node env value (+ idx 1) count)))
      (eval-node program body arm-env))))

(defn eval-match-loop [program node env value idx count]
  (if (>= idx count)
    (value-unit)
    (let [arm-base (+ 3 (* idx 2))
      pattern (vector-get node arm-base)
      body (vector-get node (+ arm-base 1))]
      (if (= (match-pattern? pattern value) 1)
        (eval-match-arm-body program node env value pattern body idx count)
        (eval-match-loop program node env value (+ idx 1) count)))))

(defn eval-match [program node env]
  (let [value (eval-node program (vector-get node 1) env)]
    (eval-match-loop program node env value 0 (vector-get node 2))))

(defn eval-match-arm-body-with-source [program node env value pattern body idx count src]
  (let [arm-env (match-bind-pattern env pattern value)]
    (if (= (vector-get body 0) (ast-match-guard))
      (let [guard-value
        (eval-node-with-source program (vector-get body 1) arm-env src)]
        (if (= (value-truthy guard-value) 1)
          (eval-node-with-source program (vector-get body 2) arm-env src)
          (eval-match-loop-with-source
            program
            node
            env
            value
            (+ idx 1)
            count
            src)))
      (eval-node-with-source program body arm-env src))))

(defn eval-match-loop-with-source [program node env value idx count src]
  (if (>= idx count)
    (value-unit)
    (let [arm-base (+ 3 (* idx 2))
      pattern (vector-get node arm-base)
      body (vector-get node (+ arm-base 1))]
      (if (= (match-pattern? pattern value) 1)
        (eval-match-arm-body-with-source
          program
          node
          env
          value
          pattern
          body
          idx
          count
          src)
        (eval-match-loop-with-source program node env value (+ idx 1) count src)))))

(defn eval-match-with-source [program node env src]
  (let [value (eval-node-with-source program (vector-get node 1) env src)]
    (eval-match-loop-with-source program node env value 0 (vector-get node 2) src)))

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

(defn eval-computation-loop-with-source [program node env idx count last src]
  (if (>= idx count)
    last
    (let [step-base (+ 3 (* idx 3))
      step-kind (vector-get node step-base)
      aux (vector-get node (+ step-base 1))
      expr (vector-get node (+ step-base 2))
      value (eval-node-with-source program expr env src)
      next-env (if (= step-kind (computation-step-let-bang))
        (env-bind env aux value)
        env)]
      (eval-computation-loop-with-source
        program
        node
        next-env
        (+ idx 1)
        count
        value
        src))))

(defn eval-computation-with-source [program node env src]
  (eval-computation-loop-with-source
    program
    node
    env
    0
    (vector-get node 2)
    (value-unit)
    src))

(defn eval-apply [program node env]
  (do
    (root_push program)
    (root_push node)
    (root_push env)
    (let [callee (vector-get node 1)
      argc (vector-get node 2)]
      (do
        (root_push callee)
        (let [args (eval-args-loop program node env 0 argc (vector-new (+ argc 1)))]
          (do
            (root_push args)
            (let [result
              (if (= (vector-get callee 0) (ast-var))
                (let [callee-hash (vector-get callee 1)]
                  (if (= (builtin-hash? callee-hash) 1)
                    (apply-builtin callee-hash args)
                    (let [decl (find-defn-by-hash program callee-hash 0 (vector-length program))]
                      (if (> (vector-length decl) 0)
                        (eval-defn-call program decl args)
                        (if (= (constructor-defined? program callee-hash) 1)
                          (make-constructor-value callee-hash args)
                          (value-unit))))))
                (value-unit))]
              (do
                (root_pop)
                (root_pop)
                (root_pop)
                (root_pop)
                (root_pop)
                result))))))))

(defn eval-node [program node env]
  (let [tag (vector-get node 0)]
    (if (= tag (ast-lit-int))
      node
      (if (= tag (ast-lit-bool))
        node
      (if (= tag (ast-lit-unit))
        node
        (if (= tag (ast-recordlit))
          (eval-record-literal program node env)
          (if (= tag (ast-fieldaccess))
            (eval-field-access program node env)
          (if (= tag (ast-recordupdate))
            (eval-record-update program node env)
          (if (= tag (ast-var))
            (let [name-hash (vector-get node 1)]
              (if (= (env-has? env name-hash) 1)
                (env-lookup env name-hash)
                (if (= (constructor-defined? program name-hash) 1)
                  (make-constructor-value name-hash (vector-new 0))
                  (value-unit))))
            (if (= tag (ast-if))
              (let [cond-value (eval-node program (vector-get node 1) env)]
                (if (= (value-tag cond-value) (ast-lit-bool))
                  (if (= (value-truthy cond-value) 1)
                    (eval-node program (vector-get node 2) env)
                    (eval-node program (vector-get node 3) env))
                  (value-int 0)))
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
                        (value-unit)))))))))))))))))

;; legacy invariant の String literal は AST が source offset を保持するため、
;; source-aware evaluator でだけ実値へ materialize する。
(defn eval-do-loop-with-source [program node env idx count last src]
  (if (>= idx count)
    last
    (let [value (eval-node-with-source program (vector-get node (+ 2 idx)) env src)]
      (eval-do-loop-with-source program node env (+ idx 1) count value src))))

(defn eval-args-loop-with-source [program node env idx count results src]
  (if (>= idx count)
    results
    (let [value (eval-node-with-source program (vector-get node (+ 3 idx)) env src)]
      (eval-args-loop-with-source
        program
        node
        env
        (+ idx 1)
        count
        (vector-push results value)
        src))))

(defn eval-defn-call-with-source [program decl args src]
  (let [param-count (vector-get decl 2)
    env (bind-params-loop (env-new) decl args 0 param-count)
    body (vector-get decl (+ 3 param-count))]
    (eval-node-with-source program body env src)))

(defn eval-record-fields-loop-with-source
  [program node env result idx count src]
  (if (>= idx count)
    result
    (let [field-hash (vector-get node (+ 3 (* idx 2)))
      value (eval-node-with-source
        program
        (vector-get node (+ 4 (* idx 2)))
        env
        src)
      next-result (vector-push-pair-rooted result field-hash value)]
      (eval-record-fields-loop-with-source
        program
        node
        env
        next-result
        (+ idx 1)
        count
        src))))

(defn eval-record-literal-with-source [program node env src]
  (eval-record-fields-loop-with-source
    program
    node
    env
    (make-record-value (vector-get node 1) (vector-get node 2))
    0
    (vector-get node 2)
    src))

(defn eval-field-access-with-source [program node env src]
  (let [record-value
    (eval-node-with-source program (vector-get node 1) env src)]
    (if (= (value-tag record-value) (ast-recordlit))
      (record-value-field record-value (vector-get node 2))
      (value-unit))))

(defn eval-record-update-fields-loop-with-source
  [program node env record-value idx count src]
  (if (>= idx count)
    record-value
    (do
      (root_push record-value)
      (let [field-hash (vector-get node (+ 3 (* idx 2)))
        new-value (eval-node-with-source
          program
          (vector-get node (+ 4 (* idx 2)))
          env
          src)]
        (do
          (root_push new-value)
          (let [updated (record-value-update-field record-value field-hash new-value)]
            (do
              (root_pop)
              (root_pop)
              (eval-record-update-fields-loop-with-source
                program
                node
                env
                updated
                (+ idx 1)
                count
                src))))))))

(defn eval-record-update-with-source [program node env src]
  (let [record-value
    (eval-node-with-source program (vector-get node 1) env src)]
    (if (= (value-tag record-value) (ast-recordlit))
      (eval-record-update-fields-loop-with-source
        program
        node
        env
        record-value
        0
        (vector-get node 2)
        src)
      (value-unit))))

(defn eval-apply-with-source [program node env src]
  (do
    (root_push program)
    (root_push node)
    (root_push env)
    (root_push src)
    (let [callee (vector-get node 1)
      argc (vector-get node 2)]
      (do
        (root_push callee)
        (let [args
          (eval-args-loop-with-source
            program
            node
            env
            0
            argc
            (vector-new (+ argc 1))
            src)]
          (do
            (root_push args)
            (let [result
              (if (= (vector-get callee 0) (ast-var))
                (let [callee-hash (vector-get callee 1)]
                  (if (= (builtin-hash? callee-hash) 1)
                    (apply-builtin callee-hash args)
                    (let [decl
                      (find-defn-by-hash program callee-hash 0 (vector-length program))]
                      (if (> (vector-length decl) 0)
                        (eval-defn-call-with-source program decl args src)
                        (if (= (constructor-defined? program callee-hash) 1)
                          (make-constructor-value callee-hash args)
                          (value-unit))))))
                (value-unit))]
              (do
                (root_pop)
                (root_pop)
                (root_pop)
                (root_pop)
                (root_pop)
                (root_pop)
                result))))))))

(defn eval-node-with-source [program node env src]
  (let [tag (vector-get node 0)]
    (if (= tag (ast-lit-int))
      node
      (if (= tag (ast-lit-bool))
        node
        (if (= tag (ast-lit-string))
          (value-string-node-with-source node src)
          (if (= tag (ast-lit-unit))
            node
            (if (= tag (ast-recordlit))
              (eval-record-literal-with-source program node env src)
            (if (= tag (ast-fieldaccess))
              (eval-field-access-with-source program node env src)
            (if (= tag (ast-recordupdate))
              (eval-record-update-with-source program node env src)
            (if (= tag (ast-var))
              (let [name-hash (vector-get node 1)]
                (if (= (env-has? env name-hash) 1)
                  (env-lookup env name-hash)
                  (if (= (constructor-defined? program name-hash) 1)
                    (make-constructor-value name-hash (vector-new 0))
                    (value-unit))))
              (if (= tag (ast-if))
                (let [cond-value
                  (eval-node-with-source program (vector-get node 1) env src)]
                  (if (= (value-tag cond-value) (ast-lit-bool))
                    (if (= (value-truthy cond-value) 1)
                      (eval-node-with-source program (vector-get node 2) env src)
                      (eval-node-with-source program (vector-get node 3) env src))
                    (value-int 0)))
                (if (= tag (ast-let))
                  (let [name-hash (vector-get node 1)
                    init-value (eval-node-with-source program (vector-get node 2) env src)
                    body-env (env-bind env name-hash init-value)]
                    (eval-node-with-source program (vector-get node 3) body-env src))
                  (if (= tag (ast-match))
                    (eval-match-with-source program node env src)
                    (if (= tag (ast-do))
                      (eval-do-loop-with-source
                        program
                        node
                        env
                        0
                        (vector-get node 1)
                        (value-unit)
                        src)
                      (if (= tag (ast-computation))
                        (eval-computation-with-source program node env src)
                        (if (= tag (ast-ann))
                          (eval-node-with-source program (vector-get node 1) env src)
                          (if (= tag (ast-apply))
                            (eval-apply-with-source program node env src)
                            (value-unit))))))))))))))))))

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

;; AST は互換のため expression span を保持しないため、source token から invariant payload span を再取得する。
(defn find-invariant-source-span-loop [src tokens idx end]
  (if (>= idx end)
    (vector-push (vector-push (vector-new 2) 0) 0)
    (if (= (token-kind tokens idx) (tok-colon))
      (if (< (+ idx 2) end)
        (if (= (token-kind tokens (+ idx 1)) (tok-symbol))
          (if (string-eq (token-text src tokens (+ idx 1)) "invariant")
            (let [payload-start (+ idx 2)
              payload-end (consume-form tokens payload-start)]
              (if (< payload-start payload-end)
                (vector-push
                  (vector-push (vector-new 2) (token-start tokens payload-start))
                  (token-end tokens (- payload-end 1)))
                (find-invariant-source-span-loop src tokens (+ idx 1) end)))
            (find-invariant-source-span-loop src tokens (+ idx 1) end))
          (find-invariant-source-span-loop src tokens (+ idx 1) end))
        (find-invariant-source-span-loop src tokens (+ idx 1) end))
      (find-invariant-source-span-loop src tokens (+ idx 1) end))))

(defn find-symbol-hash-source-span-loop [src tokens idx end target-hash]
  (if (>= idx end)
    (vector-push (vector-push (vector-new 2) 0) 0)
    (if (= (token-kind tokens idx) (tok-symbol))
      (let [start (token-start tokens idx)
        token-end-pos (token-end tokens idx)]
        (if (= (name-hash src start token-end-pos) target-hash)
          (vector-push (vector-push (vector-new 2) start) token-end-pos)
          (find-symbol-hash-source-span-loop src tokens (+ idx 1) end target-hash)))
      (find-symbol-hash-source-span-loop src tokens (+ idx 1) end target-hash))))

(defn find-property-unknown-source-span-loop [src tokens idx end target-hash]
  (if (>= idx end)
    (vector-push (vector-push (vector-new 2) 0) 0)
    (if (= (token-kind tokens idx) (tok-colon))
      (if (< (+ idx 2) end)
        (if (= (token-kind tokens (+ idx 1)) (tok-symbol))
          (if (string-eq (token-text src tokens (+ idx 1)) "postcondition")
            (let [expression-start (+ idx 2)
              expression-end (consume-form tokens expression-start)]
              (if (< expression-start expression-end)
                (find-symbol-hash-source-span-loop
                  src
                  tokens
                  expression-start
                  expression-end
                  target-hash)
                (find-property-unknown-source-span-loop src tokens (+ idx 1) end target-hash)))
            (if (string-eq (token-text src tokens (+ idx 1)) "precondition")
              (let [precondition-start (+ idx 2)
                precondition-end (if (= (token-kind tokens precondition-start) (tok-lbracket))
                  (consume-form tokens precondition-start)
                  precondition-start)]
                (if (> precondition-end (+ precondition-start 1))
                  (find-symbol-hash-source-span-loop
                    src
                    tokens
                    (+ precondition-start 1)
                    (- precondition-end 1)
                    target-hash)
                  (find-property-unknown-source-span-loop src tokens (+ idx 1) end target-hash)))
              (find-property-unknown-source-span-loop src tokens (+ idx 1) end target-hash)))
          (find-property-unknown-source-span-loop src tokens (+ idx 1) end target-hash))
        (find-property-unknown-source-span-loop src tokens (+ idx 1) end target-hash))
      (find-property-unknown-source-span-loop src tokens (+ idx 1) end target-hash))))

(defn find-property-unknown-source-span-loop-by-defn [src tokens idx count fn-hash target-hash]
  (if (>= idx count)
    (vector-push (vector-push (vector-new 2) 0) 0)
    (if (and
        (and (= (token-kind tokens idx) (tok-lparen)) (< (+ idx 2) count))
        (= (token-kind tokens (+ idx 1)) (tok-defn)))
      (let [name-start (token-start tokens (+ idx 2))
        name-end (token-end tokens (+ idx 2))
        next-idx (consume-form tokens idx)]
        (if (= (name-hash src name-start name-end) fn-hash)
          (find-property-unknown-source-span-loop
            src
            tokens
            (+ idx 3)
            (- next-idx 1)
            target-hash)
          (find-property-unknown-source-span-loop-by-defn
            src
            tokens
            next-idx
            count
            fn-hash
            target-hash)))
      (find-property-unknown-source-span-loop-by-defn
        src
        tokens
        (+ idx 1)
        count
        fn-hash
        target-hash))))

(defn find-property-unknown-source-span [src fn-hash target-hash]
  (let [tokens (tokenize-with-spans src)]
    (find-property-unknown-source-span-loop-by-defn
      src
      tokens
      0
      (token-count tokens)
      fn-hash
      target-hash)))

(defn find-invariant-unknown-source-span-loop [src tokens idx end target-hash]
  (if (>= idx end)
    (vector-push (vector-push (vector-new 2) 0) 0)
    (if (= (token-kind tokens idx) (tok-colon))
      (if (< (+ idx 2) end)
        (if (= (token-kind tokens (+ idx 1)) (tok-symbol))
          (if (string-eq (token-text src tokens (+ idx 1)) "invariant")
            (let [payload-start (+ idx 2)
              payload-end (consume-form tokens payload-start)]
              (if (< payload-start payload-end)
                (find-symbol-hash-source-span-loop
                  src
                  tokens
                  payload-start
                  payload-end
                  target-hash)
                (find-invariant-unknown-source-span-loop src tokens (+ idx 1) end target-hash)))
            (find-invariant-unknown-source-span-loop src tokens (+ idx 1) end target-hash))
          (find-invariant-unknown-source-span-loop src tokens (+ idx 1) end target-hash))
        (find-invariant-unknown-source-span-loop src tokens (+ idx 1) end target-hash))
      (find-invariant-unknown-source-span-loop src tokens (+ idx 1) end target-hash))))

(defn find-invariant-source-span-loop-by-defn [src tokens idx count target-hash]
  (if (>= idx count)
    (vector-push (vector-push (vector-new 2) 0) 0)
    (if (and
        (and (= (token-kind tokens idx) (tok-lparen)) (< (+ idx 2) count))
        (= (token-kind tokens (+ idx 1)) (tok-defn)))
      (let [name-start (token-start tokens (+ idx 2))
        name-end (token-end tokens (+ idx 2))
        next-idx (consume-form tokens idx)]
        (if (= (name-hash src name-start name-end) target-hash)
          (find-invariant-source-span-loop src tokens (+ idx 3) (- next-idx 1))
          (find-invariant-source-span-loop-by-defn src tokens next-idx count target-hash)))
      (find-invariant-source-span-loop-by-defn src tokens (+ idx 1) count target-hash))))

(defn find-invariant-source-span [src target-hash]
  (let [tokens (tokenize-with-spans src)]
    (find-invariant-source-span-loop-by-defn
      src
      tokens
      0
      (token-count tokens)
      target-hash)))

(defn find-invariant-unknown-source-span-loop-by-defn [src tokens idx count fn-hash target-hash]
  (if (>= idx count)
    (vector-push (vector-push (vector-new 2) 0) 0)
    (if (and
        (and (= (token-kind tokens idx) (tok-lparen)) (< (+ idx 2) count))
        (= (token-kind tokens (+ idx 1)) (tok-defn)))
      (let [name-start (token-start tokens (+ idx 2))
        name-end (token-end tokens (+ idx 2))
        next-idx (consume-form tokens idx)]
        (if (= (name-hash src name-start name-end) fn-hash)
          (find-invariant-unknown-source-span-loop src tokens (+ idx 3) (- next-idx 1) target-hash)
          (find-invariant-unknown-source-span-loop-by-defn src tokens next-idx count fn-hash target-hash)))
      (find-invariant-unknown-source-span-loop-by-defn src tokens (+ idx 1) count fn-hash target-hash))))

(defn find-invariant-unknown-source-span [src fn-hash target-hash]
  (let [tokens (tokenize-with-spans src)]
    (find-invariant-unknown-source-span-loop-by-defn
      src
      tokens
      0
      (token-count tokens)
      fn-hash
      target-hash)))

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
      expr (vector-get tc 2)]
      (do
        (root_push program)
        (root_push test-cases)
        (root_push tc)
        (root_push expr)
        (root_push results)
        (let [actual (eval-node program expr (env-new))
          passed (value-truthy actual)
          next-results
            (vector-push-single-rooted
              results
              (make-test-result name passed passed))]
          (do
            (root_pop)
            (root_pop)
            (root_pop)
            (root_pop)
            (root_pop)
            (run-examples-loop program test-cases (+ idx 1) count next-results)))))))

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
      diagnostic-code (if (= bool-valid 1) 0 (contract-diagnostic-non-bool))
      diagnostic-start (if (> (vector-length tc) 3) (vector-get tc 3) 0)
      diagnostic-end (if (> (vector-length tc) 4) (vector-get tc 4) 0)
      result (if (> diagnostic-code 0)
        (make-test-result-with-diagnostic-span
          name
          passed
          1
          diagnostic-code
          diagnostic-start
          diagnostic-end)
        (make-test-result-with-diagnostic
          name
          passed
          1
          diagnostic-code))]
      (run-assertions-loop
        program
        test-cases
        (+ idx 1)
        count
        (vector-push results result)))))

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

(defn case-test-actual-start [test-case]
  (if (> (vector-length test-case) 4) (vector-get test-case 4) 0))

(defn case-test-actual-end [test-case]
  (if (> (vector-length test-case) 5) (vector-get test-case 5) 0))

(defn case-test-expected-start [test-case]
  (if (> (vector-length test-case) 6) (vector-get test-case 6) 0))

(defn case-test-expected-end [test-case]
  (if (> (vector-length test-case) 7) (vector-get test-case 7) 0))

(defn run-cases-loop [program test-cases idx count results]
  (if (>= idx count)
    results
    (let [test-case (vector-get test-cases idx)
      name (vector-get test-case 0)
      diagnostic-code (case-test-diagnostic test-case)
      actual-start (case-test-actual-start test-case)
      actual-end (case-test-actual-end test-case)]
      (if (> diagnostic-code 0)
        (run-cases-loop
          program
          test-cases
          (+ idx 1)
          count
          (vector-push
            results
            (make-test-result-with-diagnostic-span
              name
              0
              0
              diagnostic-code
              actual-start
              actual-end)))
        (let [actual-expr (vector-get test-case 1)
          expected-expr (vector-get test-case 2)
          expected-start (case-test-expected-start test-case)
          expected-end (case-test-expected-end test-case)
          unknown-side (case-unknown-variable-side program actual-expr expected-expr)
          unknown-start (if (= unknown-side 2) expected-start actual-start)
          unknown-end (if (= unknown-side 2) expected-end actual-end)]
          (if (> unknown-side 0)
            (run-cases-loop
              program
              test-cases
              (+ idx 1)
              count
              (vector-push
                results
                (make-test-result-with-diagnostic-span
                  name
                  0
                  0
                  (contract-diagnostic-undefined)
                  unknown-start
                  unknown-end)))
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

(defn property-sample-bool [idx]
  (value-bool (if (= (% idx 2) 0) 0 1)))

(defn property-sample-string [idx]
  (if (= idx 0)
    (value-string "")
    (if (= idx 1)
      (value-string "a")
      (if (= idx 2)
        (value-string "hello")
        (if (= idx 3)
          (value-string "lsharp")
          (value-string "42"))))))

(defn property-sample-by-type [type-hash idx]
  (if (= type-hash (property-runner-type-bool-hash))
    (property-sample-bool idx)
    (if (= type-hash (property-runner-type-string-hash))
      (property-sample-string idx)
      (property-sample-value idx))))

(defn property-sample-mixed-by-type [type-hash idx]
  (if (= type-hash (property-runner-type-string-hash))
    (value-string "")
    (property-sample-by-type type-hash idx)))

(defn property-sample-for-binders [binder-types idx sample-idx]
  (if (> (vector-length binder-types) 1)
    (property-sample-mixed-by-type
      (vector-get binder-types idx)
      sample-idx)
    (property-sample-by-type
      (vector-get binder-types idx)
      sample-idx)))

(defn property-sample-binder-type-bool? [binder-types idx]
  (if (< idx (vector-length binder-types))
    (if (= (vector-get binder-types idx) (property-runner-type-bool-hash)) 1 0)
    0))

(defn property-sample-binder-type-int? [binder-types idx]
  (if (< idx (vector-length binder-types))
    (if (= (vector-get binder-types idx) (property-runner-type-int-hash)) 1 0)
    0))

(defn property-sample-two-int-binder? [binder-count binder-types]
  (if (and (= binder-count 2) (= (vector-length binder-types) 2))
    (if (and (= (property-sample-binder-type-int? binder-types 0) 1)
        (= (property-sample-binder-type-int? binder-types 1) 1)) 1 0)
    0))

(defn property-sample-arguments-loop [binder-types idx sample-idx result]
  (if (>= idx (vector-length binder-types))
    result
    (let [next-result (vector-push-single-rooted
        result
        (property-sample-for-binders
          binder-types
          idx
          sample-idx))]
      (do
        (root_push next-result)
        (let [completed (property-sample-arguments-loop
            binder-types
            (+ idx 1)
            sample-idx
            next-result)]
          (do
            (root_pop)
            completed))))))

(defn property-sample-arguments [test-case sample-idx]
  (let [binder-count (vector-length (property-test-case-binders test-case))
    binder-types (property-test-case-binder-types test-case)]
    (if (= (property-sample-two-int-binder? binder-count binder-types) 1)
      (vector-push-pair-rooted
        (vector-new 2)
        (property-sample-value (/ sample-idx 3))
        (property-sample-value (% sample-idx 3)))
      (property-sample-arguments-loop
        binder-types
        0
        sample-idx
        (vector-new binder-count)))))

(defn property-bind-unit-binders-loop [env binders idx]
  (if (>= idx (vector-length binders))
    env
    (property-bind-unit-binders-loop
      ;; map-get の未登録値 0 と区別するため、存在確認用の raw sentinel を束縛する。
      (env-bind env (vector-get binders idx) 1)
      binders
      (+ idx 1))))

(defn property-bind-binders-loop [env binders samples idx]
  (if (>= idx (vector-length binders))
    env
    (property-bind-binders-loop
      (env-bind env (vector-get binders idx) (vector-get samples idx))
      binders
      samples
      (+ idx 1))))

(defn property-unknown-preconditions-loop [program preconditions env idx]
  (if (>= idx (vector-length preconditions))
    -1
    (let [unknown (contract-node-unknown-hash
        program
        (vector-get preconditions idx)
        env
        1)]
      (if (>= unknown 0)
        unknown
        (property-unknown-preconditions-loop
          program
          preconditions
          env
          (+ idx 1))))))

(defn property-unknown-variable [program test-case]
  (let [base-env (property-bind-unit-binders-loop
      (env-new)
      (property-test-case-binders test-case)
      0)
    env (env-bind
      base-env
      (hash-result)
      (value-unit))
    preconditions (property-test-case-preconditions test-case)
    precondition-unknown (property-unknown-preconditions-loop
      program
      preconditions
      env
      0)]
    (if (>= precondition-unknown 0)
      precondition-unknown
      (contract-node-unknown-hash
        program
        (property-test-case-postcondition test-case)
        env
        1))))

(defn eval-property-preconditions-loop [program preconditions env idx src]
  (if (>= idx (vector-length preconditions))
    (value-bool 1)
    (let [precondition (eval-node-with-source
        program
        (vector-get preconditions idx)
        env
        src)
      precondition-bool (if (= (value-tag precondition) (ast-lit-bool)) 1 0)]
      (if (= precondition-bool 0)
        precondition
        (if (= (value-truthy precondition) 0)
          (value-bool 0)
          (eval-property-preconditions-loop
            program
            preconditions
            env
            (+ idx 1)
            src))))))

(defn eval-property-preconditions-with-index-loop [program preconditions env idx src]
  (if (>= idx (vector-length preconditions))
    (vector-push-pair-rooted (vector-new 2) (value-bool 1) -1)
    (let [precondition (eval-node-with-source
        program
        (vector-get preconditions idx)
        env
        src)
      precondition-bool (if (= (value-tag precondition) (ast-lit-bool)) 1 0)]
      (if (= precondition-bool 0)
        (vector-push-pair-rooted (vector-new 2) precondition idx)
        (if (= (value-truthy precondition) 0)
          (vector-push-pair-rooted (vector-new 2) precondition -1)
          (eval-property-preconditions-with-index-loop
            program
            preconditions
            env
            (+ idx 1)
            src))))))

(defn eval-property-precondition-with-index [program test-case sample]
  (let [preconditions (property-test-case-preconditions test-case)
    env (env-bind
      (env-new)
      (hash-result)
      (value-unit))
    env (property-bind-binders-loop
      env
      (property-test-case-binders test-case)
      sample
      0)
    precondition-source (property-test-case-precondition-source test-case)]
    (eval-property-preconditions-with-index-loop
      program
      preconditions
      env
      0
      precondition-source)))

(defn eval-property-precondition [program test-case sample]
  (let [preconditions (property-test-case-preconditions test-case)
    env (env-bind
      (env-new)
      (hash-result)
      (value-unit))
    env (property-bind-binders-loop
      env
      (property-test-case-binders test-case)
      sample
      0)
    precondition-source (property-test-case-precondition-source test-case)]
    (eval-property-preconditions-loop program preconditions env 0 precondition-source)))

(defn eval-property-sample-value [program test-case decl sample src]
  (let [args sample
    result (eval-defn-call-with-source program decl args src)
    owner-env (bind-params-loop (env-new) decl args 0 (vector-get decl 2))
    property-env0 (property-bind-binders-loop
      owner-env
      (property-test-case-binders test-case)
      sample
      0)
    property-env (env-bind property-env0 (hash-result) result)
    postcondition-source (property-test-case-postcondition-source test-case)]
    (eval-node-with-source
      program
      (property-test-case-postcondition test-case)
      property-env
      postcondition-source)))

(defn property-sample-summary [passed bool-valid actual precondition-error-index]
  (vector-push
    (vector-push
      (vector-push
        (vector-push (vector-new 4) passed)
        bool-valid)
      actual)
    precondition-error-index))

(defn run-property-samples-summary-loop
  [program test-case decl sample-idx sample-count all-passed all-bool actual-count precondition-error-index src]
  (if (>= sample-idx sample-count)
    (property-sample-summary all-passed all-bool actual-count precondition-error-index)
    (let [sample (property-sample-arguments test-case sample-idx)
      precondition-result (eval-property-precondition-with-index
        program
        test-case
        sample)
      precondition (vector-get precondition-result 0)
      precondition-index (vector-get precondition-result 1)
      precondition-bool (if (= (value-tag precondition) (ast-lit-bool)) 1 0)
      precondition-passed (if (= precondition-bool 1) (value-truthy precondition) 0)]
      (if (= precondition-bool 0)
        (run-property-samples-summary-loop
          program
          test-case
          decl
          (+ sample-idx 1)
          sample-count
          0
          0
          actual-count
          (if (>= precondition-error-index 0) precondition-error-index precondition-index)
          src)
        (if (= precondition-passed 0)
          (run-property-samples-summary-loop
            program
            test-case
            decl
            (+ sample-idx 1)
            sample-count
            all-passed
            all-bool
            actual-count
            precondition-error-index
            src)
          (let [actual (eval-property-sample-value
              program
              test-case
              decl
              sample
              src)
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
              next-bool
              (+ actual-count 1)
              precondition-error-index
              src)))))))

(defn property-runner-static-comparison-result [operator left right]
  (if (or (= operator 61) (= operator 1952))
    (if (= left right) 1 2)
    (if (= operator 1084)
      (if (!= left right) 1 2)
      (if (= operator 60)
        (if (< left right) 1 2)
        (if (= operator 62)
          (if (> left right) 1 2)
          (if (= operator 1921)
            (if (<= left right) 1 2)
            (if (= operator 1983)
              (if (>= left right) 1 2)
              0)))))))

(defn property-runner-statically-integer-comparison? [predicate expected]
  (let [tag (vector-get predicate 0)]
    (if (= tag (ast-ann))
      (property-runner-statically-integer-comparison? (vector-get predicate 1) expected)
      (if (= tag (ast-apply))
        (let [callee (vector-get predicate 1)
          arg-count (vector-get predicate 2)]
          (if (and (= arg-count 2) (= (vector-get callee 0) (ast-var)))
            (if (and (= (vector-get (vector-get predicate 3) 0) (ast-lit-int))
                (= (vector-get (vector-get predicate 4) 0) (ast-lit-int)))
              (if (= (property-runner-static-comparison-result
                  (vector-get callee 1)
                  (vector-get (vector-get predicate 3) 1)
                  (vector-get (vector-get predicate 4) 1)) expected) 1 0)
              0)
            0))
        0))))

(defn property-runner-static-boolean-and [left right]
  (if (= left 2)
    2
    (if (= right 2)
      2
      (if (= left 1)
        right
        (if (= right 1) left 0)))))

(defn property-runner-static-boolean-or [left right]
  (if (= left 1)
    1
    (if (= right 1)
      1
      (if (= left 2)
        right
        (if (= right 2) left 0)))))

(defn property-runner-expression-shape-equal-raw [left right]
  (let [left-node (if (= (vector-get left 0) (ast-ann)) (vector-get left 1) left)
    right-node (if (= (vector-get right 0) (ast-ann)) (vector-get right 1) right)
    left-tag (vector-get left-node 0)
    right-tag (vector-get right-node 0)]
    (if (!= left-tag right-tag)
      0
      (if (= left-tag (ast-var))
        (if (= (vector-get left-node 1) (vector-get right-node 1)) 1 0)
        (if (= left-tag (ast-lit-int))
          (if (= (vector-get left-node 1) (vector-get right-node 1)) 1 0)
          (if (= left-tag (ast-lit-bool))
            (if (= (vector-get left-node 1) (vector-get right-node 1)) 1 0)
            (if (= left-tag (ast-lit-string))
              (if (string-eq (vector-get left-node 1) (vector-get right-node 1)) 1 0)
              (if (= left-tag (ast-lit-unit))
                1
                (if (= left-tag (ast-apply))
                  (if (= (vector-get left-node 2) (vector-get right-node 2))
                    (if (= (property-runner-expression-shape-equal
                        (vector-get left-node 1)
                        (vector-get right-node 1)) 1)
                      (if (= (vector-get left-node 2) 1)
                        (property-runner-expression-shape-equal
                          (vector-get left-node 3)
                          (vector-get right-node 3))
                        (if (= (vector-get left-node 2) 2)
                          (if (= (property-runner-expression-shape-equal
                              (vector-get left-node 3)
                              (vector-get right-node 3)) 1)
                            (if (= (property-runner-expression-shape-equal
                                  (vector-get left-node 4)
                                  (vector-get right-node 4)) 1)
                              1
                              0)
                            0)
                          0))
                      0)
                    0)
                  0)))))))))

(defn property-runner-expression-shape-equal [left right]
  (do
    (root_push left)
    (root_push right)
    (let [result (property-runner-expression-shape-equal-raw left right)]
      (do
        (root_pop)
        (root_pop)
        result))))

;; native の call/GC 境界でも直接 application の形状比較を失わない narrow fast path。
(defn property-runner-atom-shape-equal [left right]
  (let [left-node (if (= (vector-get left 0) (ast-ann)) (vector-get left 1) left)
    right-node (if (= (vector-get right 0) (ast-ann)) (vector-get right 1) right)
    left-tag (vector-get left-node 0)
    right-tag (vector-get right-node 0)]
    (if (!= left-tag right-tag)
      0
      (if (= left-tag (ast-var))
        (if (= (vector-get left-node 1) (vector-get right-node 1)) 1 0)
        (if (= left-tag (ast-lit-int))
          (if (= (vector-get left-node 1) (vector-get right-node 1)) 1 0)
          (if (= left-tag (ast-lit-bool))
            (if (= (vector-get left-node 1) (vector-get right-node 1)) 1 0)
            (if (= left-tag (ast-lit-string))
              (if (string-eq (vector-get left-node 1) (vector-get right-node 1)) 1 0)
              (if (= left-tag (ast-lit-unit)) 1 0))))))))

(defn property-runner-direct-application-shape-equal [left right]
  (let [left-node (if (= (vector-get left 0) (ast-ann)) (vector-get left 1) left)
    right-node (if (= (vector-get right 0) (ast-ann)) (vector-get right 1) right)
    left-tag (vector-get left-node 0)
    right-tag (vector-get right-node 0)]
    (if (!= left-tag right-tag)
      0
      (if (= left-tag (ast-apply))
        (let [left-count (vector-get left-node 2)
          right-count (vector-get right-node 2)
          left-callee (vector-get left-node 1)
          right-callee (vector-get right-node 1)]
          (if (!= left-count right-count)
            0
            (if (or (< left-count 1) (> left-count 2))
              0
              (if (or (!= (vector-get left-callee 0) (ast-var)) (!= (vector-get right-callee 0) (ast-var)))
                0
                (if (!= (vector-get left-callee 1) (vector-get right-callee 1))
                  0
                  (if (= left-count 1)
                    (property-runner-atom-shape-equal
                      (vector-get left-node 3)
                      (vector-get right-node 3))
                    (if (= (property-runner-atom-shape-equal
                        (vector-get left-node 3)
                        (vector-get right-node 3)) 1)
                      (property-runner-atom-shape-equal
                        (vector-get left-node 4)
                        (vector-get right-node 4))
                      0)))))))
        0))))

(defn property-runner-negation-shape-equal [left right]
  (if (= (property-runner-direct-application-shape-equal left right) 1)
    1
    (property-runner-expression-shape-equal left right)))

(defn property-runner-is-not-expression-raw? [node]
  (if (= (vector-get node 0) (ast-apply))
    (if (= (vector-get node 2) 1)
      (let [callee (vector-get node 1)]
        (if (= (vector-get callee 0) (ast-var))
          (if (= (vector-get callee 1) (hash-not)) 1 0)
          0))
      0)
    0))

(defn property-runner-is-not-expression? [node]
  (do
    (root_push node)
    (let [result (property-runner-is-not-expression-raw? node)]
      (do
        (root_pop)
        result))))

(defn property-runner-is-boolean-negation-pair-raw [left right]
  (let [left-node (if (= (vector-get left 0) (ast-ann)) (vector-get left 1) left)
    right-node (if (= (vector-get right 0) (ast-ann)) (vector-get right 1) right)
    left-is-not (property-runner-is-not-expression? left-node)
    right-is-not (property-runner-is-not-expression? right-node)]
    (if (= left-is-not 1)
      (if (= (property-runner-negation-shape-equal
          (vector-get left-node 3)
          right-node) 1)
        1
        (if (= right-is-not 1)
          (if (= (property-runner-negation-shape-equal
              (vector-get right-node 3)
              left-node) 1)
            1
            0)
          0))
      (if (= right-is-not 1)
        (if (= (property-runner-negation-shape-equal
            (vector-get right-node 3)
            left-node) 1)
          1
          0)
        0))))

(defn property-runner-is-boolean-negation-pair [left right]
  (do
    (root_push left)
    (root_push right)
    (let [result (property-runner-is-boolean-negation-pair-raw left right)]
      (do
        (root_pop)
        (root_pop)
        result))))

(defn property-runner-statically-boolean-result-raw [predicate]
  (let [tag (vector-get predicate 0)]
    (if (= tag (ast-ann))
      (property-runner-statically-boolean-result (vector-get predicate 1))
      (if (= tag (ast-lit-bool))
        (if (= (vector-get predicate 1) 1) 1 2)
        (if (= tag (ast-apply))
          (let [callee (vector-get predicate 1)
            arg-count (vector-get predicate 2)]
            (if (= (vector-get callee 0) (ast-var))
              (let [operator (vector-get callee 1)]
                (if (= operator 109267)
                  (if (= arg-count 1)
                    (let [operand-result (property-runner-statically-boolean-result
                        (vector-get predicate 3))]
                      (if (= operand-result 1) 2 (if (= operand-result 2) 1 0)))
                    0)
                  (if (= arg-count 2)
                    (let [left (vector-get predicate 3)
                      right (vector-get predicate 4)]
                      (if (= operator 96727)
                        (if (= (property-runner-is-boolean-negation-pair left right) 1)
                          2
                          (property-runner-static-boolean-and
                            (property-runner-statically-boolean-result left)
                            (property-runner-statically-boolean-result right)))
                        (if (= operator 3555)
                          (if (= (property-runner-is-boolean-negation-pair left right) 1)
                            1
                            (property-runner-static-boolean-or
                              (property-runner-statically-boolean-result left)
                              (property-runner-statically-boolean-result right)))
                          (if (= (property-runner-statically-integer-comparison? predicate 1) 1)
                            1
                            (if (= (property-runner-statically-integer-comparison? predicate 2) 1)
                              2
                              0)))))
                    0)))
              0))
          0)))))

(defn property-runner-statically-boolean-result [predicate]
  (do
    (root_push predicate)
    (let [result (property-runner-statically-boolean-result-raw predicate)]
      (do
        (root_pop)
        result))))

(defn property-runner-static-precondition-code-loop [preconditions idx count]
  (if (>= idx count)
    0
    (if (= (property-runner-statically-boolean-result (vector-get preconditions idx)) 2)
      (contract-diagnostic-vacuous-property)
      (property-runner-static-precondition-code-loop preconditions (+ idx 1) count))))

(defn property-runner-static-vacuous-code [test-case]
  (let [postcondition-result (property-runner-statically-boolean-result
      (property-test-case-postcondition test-case))
    preconditions (property-test-case-preconditions test-case)]
    (if (= postcondition-result 1)
      (contract-diagnostic-vacuous-property)
      (property-runner-static-precondition-code-loop
        preconditions
        0
        (vector-length preconditions)))))

(defn materialize-property-with-span [program test-case src contract-span precondition-spans]
  (let [name (vector-get test-case 0)
    owner (property-test-case-owner test-case)
    decl (find-defn-by-hash program owner 0 (vector-length program))
    profile-code (property-test-case-profile-code test-case)
    binder-count (vector-length (property-test-case-binders test-case))
    owner-valid (if (and
        (> (vector-length decl) 0)
        (= (vector-get decl 2) binder-count)) 1 0)
    precondition-count (vector-length (property-test-case-preconditions test-case))
    unknown-hash (if (and (= profile-code 0) (= owner-valid 1))
      (property-unknown-variable program test-case)
      -1)
    static-vacuous-code (if (and (= profile-code 0) (and (= owner-valid 1) (< unknown-hash 0)))
      (property-runner-static-vacuous-code test-case)
      0)
    sample-count (property-test-case-count test-case)
    sample-summary (if (or (> profile-code 0) (or (= owner-valid 0) (or (>= unknown-hash 0) (> static-vacuous-code 0))))
      (property-sample-summary 0 0 0 -1)
      (run-property-samples-summary-loop
        program
        test-case
        decl
        0
        sample-count
        1
        1
        0
        -1
        src))
    bool-valid (vector-get sample-summary 1)
    actual-count (vector-get sample-summary 2)
    precondition-error-index (vector-get sample-summary 3)
    diagnostic-code (if (> profile-code 0)
      profile-code
      (if (= owner-valid 0)
        (contract-diagnostic-unsupported-property)
        (if (>= unknown-hash 0)
          (contract-diagnostic-undefined)
          (if (> static-vacuous-code 0)
            static-vacuous-code
            (if (= bool-valid 0)
              (contract-diagnostic-non-bool)
              (if (and (> precondition-count 0) (= actual-count 0))
                (contract-diagnostic-vacuous-property)
                0))))))
    passed (if (= diagnostic-code 0) (vector-get sample-summary 0) 0)
    actual (if (= diagnostic-code 0) actual-count 0)
    precondition-span (property-runner-precondition-span-from-flat
      precondition-spans
      precondition-error-index)
    fallback-source-span (if (> (vector-length contract-span) 1)
      contract-span
      (vector-push (vector-push (vector-new 2) 0) 0))
    precondition-span-valid (if (> (vector-length precondition-span) 1)
      (if (> (vector-get precondition-span 1) (vector-get precondition-span 0)) 1 0)
      0)
    source-span (if (and (> (string-length src) 0) (>= unknown-hash 0))
      (find-property-unknown-source-span src owner unknown-hash)
      (if (and (= profile-code 0) (> diagnostic-code 0))
        (if (and (= bool-valid 0) (and (= actual-count 0) (= precondition-span-valid 1)))
          precondition-span
          fallback-source-span)
        (vector-push (vector-push (vector-new 2) 0) 0)))]
    (do
      (root_push source-span)
      (let [result (if (and (> diagnostic-code 0) (> (vector-get source-span 1) (vector-get source-span 0)))
        (make-test-result-with-diagnostic-span
          name
          passed
          actual
          diagnostic-code
          (vector-get source-span 0)
          (vector-get source-span 1))
        (make-test-result-with-diagnostic name passed actual diagnostic-code))]
        (do
          (root_pop)
          result)))))

(defn materialize-property [program test-case src]
  (materialize-property-with-span
    program
    test-case
    src
    (vector-new 0)
    (vector-new 0)))

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
        (materialize-property program (vector-get test-cases idx) "")))))

(defn run-properties [program test-cases]
  (run-properties-loop
    program
    test-cases
    0
    (vector-length test-cases)
    (vector-new (vector-length test-cases))))

(defn run-properties-from-source-loop
  [program test-cases source-spans idx count results src]
  (if (>= idx count)
    results
    (run-properties-from-source-loop
      program
      test-cases
      source-spans
      (+ idx 1)
      count
      (vector-push
        results
        (materialize-property-with-span
          program
          (vector-get test-cases idx)
          src
          (property-runner-source-span-at source-spans idx)
          (property-runner-precondition-spans-at source-spans idx)))
      src)))

(defn run-properties-from-source [program test-cases src]
  (do
    (root_push program)
    (root_push test-cases)
    (root_push src)
    (let [source-spans (extract-property-test-case-source-spans program src)]
      (do
        (root_push source-spans)
        (let [results (run-properties-from-source-loop
            program
            test-cases
            source-spans
            0
            (vector-length test-cases)
            (vector-new (vector-length test-cases))
            src)]
          (do
            (root_pop)
            (root_pop)
            (root_pop)
            (root_pop)
            results))))))

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

(defn eval-invariant-sample [program tc decl param-count sample-idx src]
  (value-truthy
    (eval-invariant-sample-value
      program
      tc
      decl
      param-count
      sample-idx
      src)))

(defn eval-invariant-sample-value [program tc decl param-count sample-idx src]
  (let [args (invariant-sample-args param-count sample-idx)
    result (eval-defn-call-with-source program decl args src)
    param-env (bind-params-loop (env-new) decl args 0 param-count)
    invariant-env (env-bind param-env (hash-result) result)
    actual (eval-node-with-source program (vector-get tc 2) invariant-env src)]
    actual))

(defn invariant-sample-summary [passed bool-valid]
  (vector-push
    (vector-push (vector-new 2) passed)
    bool-valid))

(defn run-invariant-samples-loop [program tc decl sample-idx sample-count all-passed src]
  (if (>= sample-idx sample-count)
    all-passed
    (let [param-count (vector-get decl 2)
      passed (eval-invariant-sample program tc decl param-count sample-idx src)
      next-passed (if (= passed 1) all-passed 0)]
      (run-invariant-samples-loop
        program
        tc
        decl
        (+ sample-idx 1)
        sample-count
        next-passed
        src))))

(defn run-invariant-sample-summary-loop
  [program tc decl sample-idx sample-count all-passed all-bool src]
  (if (>= sample-idx sample-count)
    (invariant-sample-summary all-passed all-bool)
    (let [param-count (vector-get decl 2)
      actual (eval-invariant-sample-value
        program
        tc
        decl
        param-count
        sample-idx
        src)
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
        next-bool
        src))))

(defn materialize-invariant [program tc src]
  (let [name (vector-get tc 0)
    fn-hash (vector-get tc 1)
    decl (find-defn-by-hash program fn-hash 0 (vector-length program))
    sample-count (if (> (vector-length decl) 0)
      (invariant-sample-count (vector-get decl 2))
      0)
    unknown-hash (if (> (vector-length decl) 0)
      (invariant-unknown-variable program (vector-get tc 2) decl (vector-get decl 2))
      -1)
    static-kind (if (> (vector-length decl) 0)
      (invariant-static-bool-kind (vector-get tc 2))
      0)
    sample-summary (if (or (>= unknown-hash 0) (= static-kind 2))
      (invariant-sample-summary 0 0)
      (if (> sample-count 0)
        (run-invariant-sample-summary-loop
          program
          tc
          decl
          0
          sample-count
          1
          1
          src)
        (invariant-sample-summary 0 1)))
    type-valid (vector-get sample-summary 1)
    diagnostic-code (if (>= unknown-hash 0)
      (contract-diagnostic-undefined)
      (if (= static-kind 2)
        (contract-diagnostic-non-bool)
        (if (= type-valid 1)
          0
          (contract-diagnostic-non-bool))))
    passed (if (= diagnostic-code 0)
      (vector-get sample-summary 0)
      0)
    actual (if (= diagnostic-code 0) sample-count 0)
    source-span (if (> diagnostic-code 0)
      (if (>= unknown-hash 0)
        (find-invariant-unknown-source-span src fn-hash unknown-hash)
        (find-invariant-source-span src fn-hash))
      (vector-push (vector-push (vector-new 2) 0) 0))]
    (if (> diagnostic-code 0)
      (make-test-result-with-diagnostic-span
        name
        passed
        actual
        diagnostic-code
        (vector-get source-span 0)
        (vector-get source-span 1))
      (make-test-result-with-diagnostic name passed actual diagnostic-code))))

(defn run-invariants-loop [program invariants src idx count results]
  (if (>= idx count)
    results
    (run-invariants-loop
      program
      invariants
      src
      (+ idx 1)
      count
      (vector-push
        results
        (materialize-invariant program (vector-get invariants idx) src)))))

(defn run-invariants [program invariants]
  (run-invariants-loop
    program
    invariants
    ""
    0
    (vector-length invariants)
    (vector-new (vector-length invariants))))

(defn run-invariants-from-source [program invariants src]
  (run-invariants-loop
    program
    invariants
    src
    0
    (vector-length invariants)
    (vector-new (vector-length invariants))))

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
    invariant-results (run-invariants-from-source program invariants src)
    assertion-results (run-assertions program assertions)
    case-results (run-cases program cases)
    property-results (run-properties-from-source program properties src)]
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
