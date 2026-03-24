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
      0)))))))))

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
        all-r1 (run-all-rules-on-node unused-let all-results)]
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

      0)))
