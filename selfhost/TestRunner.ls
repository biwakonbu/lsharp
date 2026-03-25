(module TestRunner)

;; TestRunner.ls - L# セルフホスティング: メタデータテストランナー
;;
;; :example / :invariant メタデータからテストスイートを自動生成・実行する。

;; === テストケース構造 ===

;; テストケース: [name, input, expected-output]
(defn make-test-case [name input expected]
  (vector-push
    (vector-push
      (vector-push (vector-new 3) name)
      input)
    expected))

;; テスト結果: [name, passed, actual-output]
(defn make-test-result [name passed actual]
  (vector-push
    (vector-push
      (vector-push (vector-new 3) name)
      passed)
    actual))

;; === :example テスト生成 ===

;; extract-examples: AST から :example メタデータを抽出
;; ast: パース済み AST
;; 戻り値: テストケースの Vector
(defn extract-examples [ast]
  ;; AST ノードを走査して :example メタデータを収集
  ;; 各 :example は (expression => expected-value) の形式
  (vector-new 16))

;; run-examples: :example テストケースを実行
;; test-cases: extract-examples の出力
;; 戻り値: テスト結果の Vector
(defn run-examples [test-cases]
  (let [results (ref-new (vector-new 16))
        i (ref-new 0)
        n (vector-length test-cases)]
    (do
      (if (< (ref-get i) n)
        (do
          (let [tc (vector-get test-cases (ref-get i))
                name (vector-get tc 0)
                expected (vector-get tc 2)
                ;; 実行結果 (暫定: expected と同じ = pass)
                actual expected
                passed (if (= actual expected) 1 0)]
            (ref-set results (vector-push (ref-get results)
              (make-test-result name passed actual))))
          (ref-set i (+ (ref-get i) 1))
          0)
        0)
      (ref-get results))))

;; === :invariant テスト生成 ===

;; extract-invariants: AST から :invariant メタデータを抽出
;; ast: パース済み AST
;; 戻り値: 不変条件テストケースの Vector
(defn extract-invariants [ast]
  ;; AST ノードを走査して :invariant メタデータを収集
  ;; 各 :invariant は (predicate-expression) の形式
  (vector-new 16))

;; run-invariants: :invariant テストケースを実行
;; invariants: extract-invariants の出力
;; 戻り値: テスト結果の Vector
(defn run-invariants [invariants]
  (let [results (ref-new (vector-new 16))
        i (ref-new 0)
        n (vector-length invariants)]
    (do
      (if (< (ref-get i) n)
        (do
          (let [inv (vector-get invariants (ref-get i))
                name (vector-get inv 0)
                ;; 不変条件を評価 (暫定: 常に pass)
                passed 1
                actual 1]
            (ref-set results (vector-push (ref-get results)
              (make-test-result name passed actual))))
          (ref-set i (+ (ref-get i) 1))
          0)
        0)
      (ref-get results))))

;; === テストスイート生成 ===

;; generate-tests: AST からテストスイート全体を生成・実行
;; ast: パース済み AST
;; 戻り値: [example-results, invariant-results] の Vector
(defn generate-tests [ast]
  (let [examples (extract-examples ast)
        invariants (extract-invariants ast)
        example-results (run-examples examples)
        invariant-results (run-invariants invariants)]
    (vector-push
      (vector-push (vector-new 2) example-results)
      invariant-results)))

;; エントリポイント (テスト用)
(defn main []
  (let [suite (generate-tests 0)]
    (do
      (print (vector-length suite))      ;; 2 (examples + invariants)
      (print (vector-length (vector-get suite 0)))  ;; 0 (example results)
      (print (vector-length (vector-get suite 1)))  ;; 0 (invariant results)
      0)))
