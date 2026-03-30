(module Types.TypeInfer)
(import Syntax.AST)
(import Types.Type)
(import Types.TypeScheme)
(import Types.TypeInferCore)
(import Types.TypeInferFunctions)
(import Types.TypeInferBuiltins)

;; TypeInfer.ls - L# セルフホスティング: Hindley-Milner 型推論
;;
;; Type.ls (型定義・単一化・代入) と TypeScheme.ls (汎化・具体化) を使い、
;; AST ノードに対して型推論を行う。
;;
;; バンドルモードでは以下のサブモジュールが実装を上書きする:
;;   TypeInferApply.ls   - lambda 式・関数適用の型推論
;;   TypeInferBlock.ls   - let 式・do ブロック・computation 式の型推論
;;   TypeInferPattern.ls - パターンマッチの型推論
;;   TypeInferRecord.ls  - レコード型の型推論
;;
;; 依存: Type.ls, TypeScheme.ls, AST.ls
;;
;; 型環境 (TypeEnv) = HashMap<name-hash, TypeScheme>
;; 推論結果 = [subst, type] (Vector of 2 要素)

;; ============================================================
;; infer-expr: AST ノードの型推論
;; ============================================================
;; 引数:
;;   node    - AST ノード (Vector)
;;   env     - 型環境 (HashMap<name-hash, TypeScheme>)
;;   subst   - 現在の置換 (HashMap<var-id, Type>)
;;   counter - 型変数カウンタ (ref-cell)
;; 戻り値:
;;   [subst, type, error-code] - 更新された置換と推論された型

;; リテラルの型推論
(defn infer-lit [node]
  (let [tag (vector-get node 0)]
    (if (= tag 1)
      (mk-int)
      (if (= tag 2)
        (mk-bool)
        (if (= tag 3)
          (mk-string)
          (if (= tag 19)
            (mk-float)
            (if (= tag 32)
              (mk-unit)
              ;; 不明なリテラル -> Int にフォールバック
              (mk-int))))))))

;; 変数参照の型推論
(defn infer-var [node env subst counter]
  (let [name-hash (vector-get node 1)
    scheme (type-env-lookup env name-hash)]
    (if (= scheme 0)
      ;; 未定義変数: エラー
      (make-error-result-code (error-code-undefined))
      ;; 型スキームを具体化
      (let [ty (instantiate scheme counter)]
        (make-result subst ty)))))

;; if 式の型推論
;; [6, cond, then, else]
(defn infer-if [node env subst counter]
  (let [cond-node (vector-get node 1)
    then-node (vector-get node 2)
    else-node (vector-get node 3)
    ;; 条件式を推論
    cond-result (infer-expr cond-node env subst counter)]
    (if (= (result-failed cond-result) 1)
      (make-error-result-code (result-error-code cond-result))
      (let [s1 (result-subst cond-result)
        cond-ty (result-type cond-result)
        ;; 条件式は Bool であること
        s2 (unify cond-ty (mk-bool) s1)]
        (if (= (unify-failed s2) 1)
          (make-error-result-code (error-code-if-cond))
          ;; then 枝を推論
          (let [then-result (infer-expr then-node env s2 counter)]
            (if (= (result-failed then-result) 1)
              (make-error-result-code (result-error-code then-result))
              (let [s3 (result-subst then-result)
                then-ty (result-type then-result)
                ;; else 枝を推論
                else-result (infer-expr else-node env s3 counter)]
                (if (= (result-failed else-result) 1)
                  (make-error-result-code (result-error-code else-result))
                  ;; then と else の型を統一
                  (let [s4 (result-subst else-result)
                    else-ty (result-type else-result)
                    s5 (unify (apply-subst s4 then-ty) else-ty s4)]
                    (if (= (unify-failed s5) 1)
                      (make-error-result-code (error-code-if-branch))
                      (make-result s5 (apply-subst s5 else-ty)))))))))))))

;; ann 式の型推論
;; selfhost AST は型式 payload を保持していないため、現状は内側の式をそのまま推論する
;; [11, expr]
(defn infer-ann [node env subst counter]
  (infer-expr (vector-get node 1) env subst counter))

;; quote/unquote 系は現状すべて inner expr へ委譲する
(defn quote-like-tag? [tag]
  (if (= tag (tag-quote))
    1
    (if (= tag (tag-unquote))
      1
      (if (= tag (tag-unquote-splice))
        1
        0))))

;; ============================================================
;; スタブ定義 (バンドルモードではサブモジュールが上書き)
;; ============================================================
;; マルチファイルコンパイル時の型検査を通すためのフォールバック実装。
;; バンドル (連結) モードでは TypeInferApply/Block/Pattern/Record が
;; これらを完全な実装で上書きする。

;; --- Apply グループ (TypeInferApply.ls が上書き) ---
(defn infer-lambda [node env subst counter]
  (make-result subst (fresh-type-var counter)))
(defn infer-apply [node env subst counter]
  (make-result subst (fresh-type-var counter)))

;; --- Block グループ (TypeInferBlock.ls が上書き) ---
(defn infer-let [node env subst counter]
  (make-result subst (fresh-type-var counter)))
(defn infer-computation-steps [node idx step-count env subst counter last-ty]
  (make-result subst last-ty))
(defn infer-computation [node env subst counter]
  (make-result subst (mk-int)))
(defn infer-do [node env subst counter]
  (make-result subst (mk-int)))

;; --- Pattern グループ (TypeInferPattern.ls が上書き) ---
(defn pattern-children-subst [r]
  (vector-get r 0))
(defn pattern-children-env [r]
  (vector-get r 1))
(defn infer-pattern-children [node idx count base-index stride env subst counter]
  (vector-push (vector-push (vector-new 2) subst) env))
(defn infer-constructor-pattern-children [node idx count env subst counter ctor-ty]
  (vector-push (make-result subst (mk-int)) env))
(defn infer-pattern [pat env subst counter]
  (vector-push (make-result subst (fresh-type-var counter)) env))
(defn pat-result-subst [r]
  (vector-get r 0))
(defn pat-result-type [r]
  (vector-get r 1))
(defn pat-result-env [r]
  (vector-get r 3))
(defn infer-match-arms [node idx arm-count env scrut-ty result-ty subst counter]
  (make-result subst result-ty))
(defn infer-match [node env subst counter]
  (make-result subst (fresh-type-var counter)))

;; --- Record グループ (TypeInferRecord.ls が上書き) ---
(defn infer-record-fields [node idx count env subst counter]
  (make-result subst (mk-int)))
(defn infer-recordlit-fields [node idx count env subst counter record-ty]
  (make-result subst record-ty))
(defn recordlit-field-node-loop [record-node field-name-hash idx field-count]
  0)
(defn recordlit-field-node [record-node field-name-hash]
  0)
(defn infer-recordlit [node env subst counter]
  (make-result subst (mk-int)))
(defn infer-fieldaccess [node env subst counter]
  (make-result subst (fresh-type-var counter)))
(defn infer-recordupdate-node [node env subst counter]
  (make-result subst (fresh-type-var counter)))

;; ============================================================
;; infer-expr: メインディスパッチ
;; ============================================================

(defn infer-expr [node env subst counter]
  (let [tag (vector-get node 0)]
    (if (= tag 1)
      ;; 整数リテラル
      (make-result subst (mk-int))
      (if (= tag 2)
        ;; 真偽値リテラル
        (make-result subst (mk-bool))
        (if (= tag 3)
          ;; 文字列リテラル
          (make-result subst (mk-string))
          (if (= tag 19)
            ;; 浮動小数点リテラル
            (make-result subst (mk-float))
            (if (= tag 32)
              ;; unit リテラル
              (make-result subst (mk-unit))
              (if (= tag 4)
                ;; 変数参照
                (infer-var node env subst counter)
                (if (= tag 5)
                  ;; 関数適用
                  (infer-apply node env subst counter)
                  (if (= tag 6)
                    ;; if 式
                    (infer-if node env subst counter)
                    (if (= tag 7)
                      ;; let 式
                      (infer-let node env subst counter)
                      (if (= tag (tag-ann))
                        ;; ann 式
                        (infer-ann node env subst counter)
                        (if (= (quote-like-tag? tag) 1)
                          ;; quote / unquote / unquote-splice
                          (infer-ann node env subst counter)
                          (if (= tag (tag-recordlit))
                            ;; record literal
                            (infer-recordlit node env subst counter)
                            (if (= tag (tag-fieldaccess))
                              ;; field access
                              (infer-fieldaccess node env subst counter)
                              (if (= tag (tag-recordupdate))
                                ;; record update
                                (infer-recordupdate-node node env subst counter)
                                (if (= tag (tag-computation))
                                  ;; computation 式
                                  (infer-computation node env subst counter)
                                  (if (= tag 8)
                                    ;; lambda 式
                                    (infer-lambda node env subst counter)
                                    (if (= tag 9)
                                      ;; do ブロック
                                      (infer-do node env subst counter)
                                      (if (= tag 10)
                                        ;; match 式
                                        (infer-match node env subst counter)
                                        ;; 未知のノード: エラー
                                        (make-error-result)))))))))))))))))))))

;; ============================================================
;; infer-defn: トップレベル関数定義の型推論
;; ============================================================
;; [20, name-hash, param-count, param-hash1, ..., body]
;; compile-safe な covered slice として 0/1/2/3/4 引数を扱う

(defn infer-defn [node env counter]
  (let [name-hash (vector-get node 1)
    param-count (vector-get node 2)
    subst (subst-new)]
    (if (= param-count 0)
      (let [body-node (vector-get node 3)
        result (infer-expr body-node env subst counter)]
        (if (= (result-failed result) 1)
          (propagate-error-result result)
          (let [s (result-subst result)
            body-ty (result-type result)]
            (typeinfer-finalize-defn-result env name-hash s body-ty))))
      (let [param-types (typeinfer-fresh-param-types param-count counter)
        body-node (vector-get node (+ param-count 3))
        next-env (typeinfer-extend-env-with-node-params env node param-count 3 param-types)
        result (infer-expr body-node next-env subst counter)]
        (if (= (result-failed result) 1)
          (propagate-error-result result)
          (let [s (result-subst result)
            body-ty (result-type result)
            fun-ty (typeinfer-build-curried-fun param-types s body-ty)]
            (typeinfer-finalize-defn-result env name-hash s fun-ty)))))))

;; ============================================================
;; infer: 公開 API (Main.ls から呼び出される)
;; ============================================================

(defn infer [program]
  (let [counter (make-var-counter)
    env (init-builtin-env counter)
    n (vector-length program)]
    (if (> n 0)
      (let [decl (vector-get program 0)]
        (if (= (vector-get decl 0) 20)
          (let [out (infer-defn decl env counter)]
            (if (= (vector-length out) 2)
              (vector-get out 1)
              (vector-get out 1)))
          (mk-int)))
      (mk-int))))

;; ============================================================
;; ビルトイン型環境の初期化
;; ============================================================

;; builtin env 本体は Types.TypeInferBuiltins へ分離
(defn init-builtin-env [counter]
  (typeinfer-init-builtin-env counter))
