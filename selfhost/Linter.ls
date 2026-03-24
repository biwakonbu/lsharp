;; Linter.ls - AST ベースのリントルール基盤
;;
;; P9-6c: L# で実装されたリンター
;; selfhost/AST.ls の AST ノードを走査して、
;; コーディング規約違反や潜在的バグを検出する。
;;
;; リントルール:
;; 1. 未使用変数検出 (let 束縛の変数が本体で参照されない)
;; 2. 未使用 import 検出
;; 3. 型注釈推奨 (公開関数に型注釈がない)
;;
;; 各ルールは整数タグ + Vector で診断情報を返す

;; === AST ヘルパー (単体コンパイル用) ===

;; 整数リテラル: [1, value]
(defn make-lit-int [value]
  (let [v (vector-new 2)]
    (vector-push (vector-push v 1) value)))

;; 変数参照: [4, name-hash]
(defn make-var [name-hash]
  (let [v (vector-new 2)]
    (vector-push (vector-push v 4) name-hash)))

;; ノード内で特定の name-hash を持つ var 参照が存在するか検索
;; 見つかれば 1、なければ 0
(defn ast-contains-var [node target-hash]
  (let [tag (vector-get node 0)]
    (if (= tag 4)
      (if (= (vector-get node 1) target-hash) 1 0)
      (if (= tag 1) 0
      (if (= tag 2) 0
      (if (= tag 3) 0
      (if (= tag 6)
        ;; if ノード: [6, cond, then, else]
        (let [r1 (ast-contains-var (vector-get node 1) target-hash)]
          (if (= r1 1) 1
            (let [r2 (ast-contains-var (vector-get node 2) target-hash)]
              (if (= r2 1) 1
                (ast-contains-var (vector-get node 3) target-hash)))))
      (if (= tag 7)
        ;; let ノード: [7, name-hash, init-expr, body-expr]
        (let [r1 (ast-contains-var (vector-get node 2) target-hash)]
          (if (= r1 1) 1
            (ast-contains-var (vector-get node 3) target-hash)))
      (if (= tag 5)
        ;; apply ノード: [5, func-hash, arg-count, arg1, arg2, ...]
        (let [argc (vector-get node 2)]
          (if (> argc 0)
            (let [r1 (ast-contains-var (vector-get node 3) target-hash)]
              (if (= r1 1) 1
                (if (> argc 1)
                  (ast-contains-var (vector-get node 4) target-hash)
                  0)))
            0))
      (if (= tag 9)
        ;; do ノード: [9, expr-count, expr1, expr2, ...]
        ;; 最大5式を展開して走査
        (let [ec (vector-get node 1)]
          (if (> ec 0)
            (let [r1 (ast-contains-var (vector-get node 2) target-hash)]
              (if (= r1 1) 1
                (if (> ec 1)
                  (let [r2 (ast-contains-var (vector-get node 3) target-hash)]
                    (if (= r2 1) 1
                      (if (> ec 2)
                        (let [r3 (ast-contains-var (vector-get node 4) target-hash)]
                          (if (= r3 1) 1
                            (if (> ec 3)
                              (let [r4 (ast-contains-var (vector-get node 5) target-hash)]
                                (if (= r4 1) 1
                                  (if (> ec 4)
                                    (ast-contains-var (vector-get node 6) target-hash)
                                    0)))
                              0)))
                        0)))
                  0)))
            0))
      (if (= tag 10)
        ;; match ノード: [10, scrutinee, arm-count, pat1, body1, ...]
        ;; scrutinee + 最大3腕の body を走査
        (let [r1 (ast-contains-var (vector-get node 1) target-hash)
              ac (vector-get node 2)]
          (if (= r1 1) 1
            (if (> ac 0)
              (let [rb1 (ast-contains-var (vector-get node 4) target-hash)]
                (if (= rb1 1) 1
                  (if (> ac 1)
                    (let [rb2 (ast-contains-var (vector-get node 6) target-hash)]
                      (if (= rb2 1) 1
                        (if (> ac 2)
                          (ast-contains-var (vector-get node 8) target-hash)
                          0)))
                    0)))
              0)))
      0)))))))))))

;; リント診断の重要度
(defn lint-error [] 0)
(defn lint-warning [] 1)
(defn lint-info [] 2)
(defn lint-hint [] 3)

;; リントルール ID
(defn rule-unused-var [] 100)
(defn rule-unused-import [] 101)
(defn rule-missing-type-ann [] 102)
(defn rule-shadowed-var [] 103)
(defn rule-empty-body [] 104)

;; 診断情報の構築
;; [severity, rule-id, line, column, message-hash]
(defn make-diagnostic [severity rule-id line col msg-hash]
  (let [v (vector-new 5)]
    (vector-push
      (vector-push
        (vector-push
          (vector-push
            (vector-push v severity)
            rule-id)
          line)
        col)
      msg-hash)))

;; 診断情報アクセサ
(defn diag-severity [d]
  (vector-get d 0))

(defn diag-rule [d]
  (vector-get d 1))

(defn diag-line [d]
  (vector-get d 2))

;; リントルール: 空の do ブロック検出
;; (do) → 警告
(defn check-empty-body [ast-tag child-count]
  (if (= ast-tag 9)
    (if (= child-count 0)
      (make-diagnostic (lint-warning) (rule-empty-body) 0 0 0)
      0)
    0))

;; リント結果の集約
;; diagnostics を Vector に集める
(defn lint-results-new []
  (vector-new 16))

(defn lint-add [results diagnostic]
  (if (= diagnostic 0)
    results
    (vector-push results diagnostic)))

;; カスタムルール定義の基盤
;; ルールは (ast-node -> diagnostic | 0) 関数
;; クロージャ制約のため、各ルールを直接呼び出す方式

(defn rule-count [] 5)

;; === ルール実装 ===

;; リントルール: 未使用変数検出
;; let ノード [7, name-hash, init-expr, body-expr] で、
;; body-expr 内に name-hash を参照する var ノードがなければ警告
(defn check-unused-var [node]
  (let [tag (vector-get node 0)]
    (if (= tag 7)
      (let [name-hash (vector-get node 1)
            body (vector-get node 3)
            ;; body 内に変数参照があるか検索
            found (ast-contains-var body name-hash)]
        (if (= found 0)
          (make-diagnostic (lint-warning) (rule-unused-var) 0 0 name-hash)
          0))
      0)))

;; === P9-6c: LSP Diagnostic 変換 ===

;; リント診断を LSP Diagnostic 形式に変換
;; 入力: [severity, rule-id, line, col, msg-hash]
;; 出力: [start-line, start-col, severity, rule-id]
(defn make-lsp-diagnostic [diagnostic]
  (let [v (vector-new 4)]
    (vector-push
      (vector-push
        (vector-push
          (vector-push v (diag-line diagnostic))
          (vector-get diagnostic 3))
        (diag-severity diagnostic))
      (diag-rule diagnostic))))

;; リント結果から LSP publishDiagnostics 用の診断数を返す
(defn diagnostics-to-lsp-count [results]
  (vector-length results))

;; === ルール一括実行 ===

;; 単一ノードに全ルールを適用し、結果を集約
(defn run-all-rules-on-node [node results]
  (let [r1 (lint-add results (check-empty-body (vector-get node 0) 0))
        r2 (lint-add r1 (check-unused-var node))]
    r2))

;; 検証用 main
(defn main []
  (let [;; 診断情報の生成テスト
        d1 (make-diagnostic (lint-warning) (rule-unused-var) 10 5 0)
        d2 (make-diagnostic (lint-error) (rule-missing-type-ann) 20 1 0)
        d3 (check-empty-body 9 0)

        ;; リント結果の集約テスト
        results (lint-results-new)
        r1 (lint-add results d1)
        r2 (lint-add r1 d2)
        r3 (lint-add r2 d3)

        ;; === 新規テスト: 未使用変数検出 ===
        ;; let x = 42 in 0 → x は使われていない → 警告
        unused-let (let [v (vector-new 4)]
                     (vector-push (vector-push (vector-push (vector-push v 7)
                       99) (make-lit-int 42)) (make-lit-int 0)))
        d-unused (check-unused-var unused-let)

        ;; let x = 42 in x → x は使われている → 警告なし
        used-let (let [v (vector-new 4)]
                   (vector-push (vector-push (vector-push (vector-push v 7)
                     99) (make-lit-int 42)) (make-var 99)))
        d-used (check-unused-var used-let)

        ;; === 新規テスト: ルール一括実行 ===
        all-results (lint-results-new)
        all-r1 (run-all-rules-on-node unused-let all-results)

        ;; === 新規テスト: do ノード内の変数参照検出 ===
        ;; do ノード: [9, 2, var(99), lit(0)]
        do-node (vector-push (vector-push (vector-push (vector-push (vector-new 4) 9) 2) (make-var 99)) (make-lit-int 0))
        ;; ast-contains-var: do ノード内で検索
        do-found (ast-contains-var do-node 99)
        do-not (ast-contains-var do-node 77)
        ;; let x = 42 in (do x 0) → x は使用されている → 警告なし
        used-do-let (vector-push (vector-push (vector-push (vector-push (vector-new 4) 7) 99) (make-lit-int 42)) do-node)
        d-used-do (check-unused-var used-do-let)

        ;; === 新規テスト: match ノード内の変数参照検出 ===
        ;; match ノード: [10, lit(0), 1, lit(1), var(99)]
        match-node (vector-push (vector-push (vector-push (vector-push (vector-push (vector-new 6) 10) (make-lit-int 0)) 1) (make-lit-int 1)) (make-var 99))
        ;; ast-contains-var: match ノード内で検索
        match-found (ast-contains-var match-node 99)
        match-not (ast-contains-var match-node 77)
        ;; let x = 42 in (match 0 [1 x]) → x は使用されている → 警告なし
        used-match-let (vector-push (vector-push (vector-push (vector-push (vector-new 4) 7) 99) (make-lit-int 42)) match-node)
        d-used-match (check-unused-var used-match-let)

        ;; === P9-6c: LSP 統合テスト ===
        ;; 診断情報を LSP Diagnostic 形式に変換
        lsp-d1 (make-lsp-diagnostic d1)
        ;; publishDiagnostics 用の診断数カウント
        lsp-count (diagnostics-to-lsp-count r3)]
    (do
      ;; 診断情報の検証
      (print (diag-severity d1))  ;; 1 (warning)
      (print (diag-rule d1))      ;; 100 (unused-var)
      (print (diag-line d1))      ;; 10

      (print (diag-severity d2))  ;; 0 (error)
      (print (diag-rule d2))      ;; 102 (missing-type-ann)

      ;; 空ブロック検出の検証
      (print (diag-severity d3))  ;; 1 (warning)
      (print (diag-rule d3))      ;; 104 (empty-body)

      ;; 集約結果の検証
      (print (vector-length r3))  ;; 3

      ;; ルール数
      (print (rule-count))        ;; 5

      ;; 未使用変数: 検出される
      (print (diag-severity d-unused))  ;; 1 (warning)
      (print (diag-rule d-unused))      ;; 100 (unused-var)

      ;; 使用済み変数: 検出されない (0)
      (print d-used)                    ;; 0

      ;; ルール一括実行: unused-let に対して1件検出
      (print (vector-length all-r1))    ;; 1

      ;; do ノード: ast-contains-var 直接検索
      (print do-found)                  ;; 1 (var 99 found)
      (print do-not)                    ;; 0 (var 77 not found)
      ;; do ノード: let 経由の未使用変数検出 → 警告なし
      (print d-used-do)                 ;; 0

      ;; match ノード: ast-contains-var 直接検索
      (print match-found)              ;; 1 (var 99 found)
      (print match-not)                ;; 0 (var 77 not found)
      ;; match ノード: let 経由の未使用変数検出 → 警告なし
      (print d-used-match)             ;; 0

      ;; === P9-6c: LSP 統合検証 ===
      ;; LSP Diagnostic: [start-line, start-col, severity, rule-id]
      (print (vector-get lsp-d1 0))    ;; 10 (start-line)
      (print (vector-get lsp-d1 1))    ;; 5  (start-col)
      (print (vector-get lsp-d1 2))    ;; 1  (severity: warning)
      (print (vector-get lsp-d1 3))    ;; 100 (code: unused-var)
      ;; publishDiagnostics 診断数
      (print lsp-count)                ;; 3

      0)))
