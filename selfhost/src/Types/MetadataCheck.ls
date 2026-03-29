(module Types.MetadataCheck)
(import Syntax.AST)
(import Syntax.Span)

;; MetadataCheck.ls - L# セルフホスティング: メタデータ検証
;;
;; :doc, :params, :returns 等のメタデータアノテーションを検証する。
;; Rust 版 metadata_check.rs に対応する。
;;
;; 依存: AST.ls, Span.ls

;; ============================================================
;; メタデータ種別タグ
;; ============================================================

(defn meta-doc [] 1)       ;; :doc メタデータ
(defn meta-params [] 2)    ;; :params メタデータ
(defn meta-returns [] 3)   ;; :returns メタデータ
(defn meta-example [] 4)   ;; :example メタデータ
(defn meta-invariant [] 5) ;; :invariant メタデータ

;; ============================================================
;; 検証結果
;; ============================================================
;; 結果 = [status, error-count, error1, error2, ...]
;; status: 0 = OK, 1 = エラーあり

(defn make-check-result [status errors]
  (vector-push (vector-push (vector-new 2) status) errors))

(defn check-ok []
  (make-check-result 0 (vector-new 0)))

(defn check-error [errors]
  (make-check-result 1 errors))

(defn check-result-ok [r]
  (= (vector-get r 0) 0))

(defn check-result-errors [r]
  (vector-get r 1))

;; ============================================================
;; validate-doc: :doc メタデータの検証
;; ============================================================
;; :doc は文字列リテラルであること
;; 空文字列はエラー
;;
;; 引数:
;;   meta - メタデータノード [meta-tag, value-node]
;; 戻り値:
;;   検証結果

(defn validate-doc [meta]
  (let [tag (vector-get meta 0)
        value (vector-get meta 1)]
    (if (= tag (meta-doc))
      ;; :doc の値ノードを検証
      (let [value-tag (vector-get value 0)]
        (if (= value-tag 3)
          ;; 文字列リテラル: OK
          ;; 空文字列チェック (value[1] がペイロード長)
          (let [payload (vector-get value 1)]
            (if (= payload 0)
              ;; 空文字列: エラー
              (check-error (vector-push (vector-new 1) 1))
              ;; 非空文字列: OK
              (check-ok)))
          ;; 文字列以外: エラー
          (check-error (vector-push (vector-new 1) 2))))
      ;; :doc 以外のメタデータ: スキップ (OK)
      (check-ok))))

;; ============================================================
;; validate-params: :params メタデータの検証
;; ============================================================
;; :params はパラメータ名と説明のペアリスト
;; 関数のパラメータリストと一致することを検証
;;
;; 引数:
;;   meta        - メタデータノード [meta-tag, param-count, name1, desc1, ...]
;;   param-names - 関数パラメータ名ハッシュのリスト (Vector)
;; 戻り値:
;;   検証結果

(defn validate-params [meta param-names]
  (let [tag (vector-get meta 0)]
    (if (= tag (meta-params))
      (let [param-count (vector-get meta 1)
            expected-count (vector-length param-names)]
        ;; パラメータ数の一致を検証
        (if (= param-count expected-count)
          (check-ok)
          ;; 数が不一致: エラー
          (check-error (vector-push (vector-new 1) 3))))
      ;; :params 以外: スキップ
      (check-ok))))

;; ============================================================
;; validate-returns: :returns メタデータの検証
;; ============================================================
;; :returns は戻り値の説明文字列
;; 空でないことを検証
;;
;; 引数:
;;   meta - メタデータノード [meta-tag, value-node]
;; 戻り値:
;;   検証結果

(defn validate-returns [meta]
  (let [tag (vector-get meta 0)]
    (if (= tag (meta-returns))
      (let [value (vector-get meta 1)
            value-tag (vector-get value 0)]
        (if (= value-tag 3)
          ;; 文字列リテラル: OK
          (let [payload (vector-get value 1)]
            (if (= payload 0)
              ;; 空文字列: エラー
              (check-error (vector-push (vector-new 1) 4))
              ;; 非空: OK
              (check-ok)))
          ;; 文字列以外: エラー
          (check-error (vector-push (vector-new 1) 5))))
      ;; :returns 以外: スキップ
      (check-ok))))

;; ============================================================
;; validate-all: 全メタデータの一括検証
;; ============================================================
;; 引数:
;;   metadata-list - メタデータノードのリスト (Vector)
;;   param-names   - 関数パラメータ名のリスト (Vector)
;; 戻り値:
;;   検証結果

(defn validate-all [metadata-list param-names]
  (let [count (vector-length metadata-list)]
    (validate-all-loop metadata-list param-names 0 count (vector-new 4))))

(defn validate-all-loop [metadata-list param-names i count errors]
  (if (>= i count)
    ;; 全メタデータチェック済み
    (if (= (vector-length errors) 0)
      (check-ok)
      (check-error errors))
    (let [meta (vector-get metadata-list i)
          tag (vector-get meta 0)
          r (if (= tag (meta-doc))
              (validate-doc meta)
              (if (= tag (meta-params))
                (validate-params meta param-names)
                (if (= tag (meta-returns))
                  (validate-returns meta)
                  (check-ok))))]
      (if (check-result-ok r)
        (validate-all-loop metadata-list param-names (+ i 1) count errors)
        ;; エラー: 蓄積して続行
        (validate-all-loop metadata-list param-names (+ i 1) count
          (vector-push errors (check-result-errors r)))))))

;; ============================================================
;; エントリポイント (テスト用)
;; ============================================================

(defn main []
  (do
    ;; テスト: :doc 検証 (正常)
    (let [doc-meta (vector-push (vector-push (vector-new 2) (meta-doc))
                     (vector-push (vector-push (vector-new 2) 3) 5))
          r1 (validate-doc doc-meta)]
      (print (vector-get r1 0)))  ;; 0 (OK)

    ;; テスト: :returns 検証 (正常)
    (let [ret-meta (vector-push (vector-push (vector-new 2) (meta-returns))
                     (vector-push (vector-push (vector-new 2) 3) 10))
          r2 (validate-returns ret-meta)]
      (print (vector-get r2 0)))  ;; 0 (OK)

    0))
