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
;; ルールレジストリに登録して一括実行

(defn rule-count [] 5)

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
        r3 (lint-add r2 d3)]
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

      0)))
