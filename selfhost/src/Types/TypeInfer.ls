(module Types.TypeInfer)
(import Syntax.AST)
(import Types.Type)
(import Types.TypeScheme)
(import Types.TypeInferCore)
(import Types.TypeInferFunctions)
(import Types.TypeInferBuiltins)
(import Types.TypeInferAdt)
(import Types.TypeInferRecordDecl)

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
;;   TypeInferAdt.ls     - ADT constructor の宣言登録
;;   TypeInferRecordDecl.ls - record schema / constructor の宣言登録
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
;;   counter - 型変数カウンタと alias 環境を持つ推論 context
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
(defn typeinfer-var-scheme [node env]
  (let [name-hash (vector-get node 1)
    direct-scheme (type-env-lookup env name-hash)]
    (if (= direct-scheme 0)
      (if (> (vector-length node) 5)
        (type-env-lookup
          env
          (ast-qualified-name-hash
            (vector-get node 4)
            (vector-get node 5)))
        0)
      direct-scheme)))

(defn infer-var [node env subst counter]
  (do
    ;; 型スキームの具体化と結果構築は複数の allocation を跨ぐため、
    ;; native GC が scheme / instantiated type / result type を回収しないよう保持する。
    (root_push env)
    (root_push subst)
    (root_push counter)
    (let [name-hash (vector-get node 1)
      scheme (typeinfer-var-scheme node env)]
      (do
        (let [result
                (if (= scheme 0)
                  ;; 未定義変数: エラー
                  (if (> (vector-length node) 3)
                    (make-error-result-code-with-span-and-name
                      (error-code-undefined)
                      (vector-get node 2)
                      (vector-get node 3)
                      name-hash)
                    (make-error-result-code-and-name (error-code-undefined) name-hash))
                  ;; 型スキームを具体化
                  (let [instantiated (instantiate scheme counter)]
                    (do
                      (root_push instantiated)
                      (let [ty (apply-subst subst instantiated)]
                        (do
                          (root_push ty)
                          (let [result (make-result subst ty)]
                            (do
                              (root_pop)
                              (root_pop)
                              result)))))))]
          (do
            (root_pop)
            (root_pop)
            (root_pop)
            result))))))

;; if 式の型推論
;; [6, cond, then, else]
(defn infer-if [node env subst counter]
  (let [cond-node (vector-get node 1)
    then-node (vector-get node 2)
    else-node (vector-get node 3)
    ;; 条件式を推論
    cond-result (infer-expr cond-node env subst counter)]
    (if (= (result-failed cond-result) 1)
      (propagate-error-result-with-span-and-name cond-result)
      (let [s1 (result-subst cond-result)
        cond-ty (result-type cond-result)
        ;; 条件式は Bool であること
        s2 (unify cond-ty (mk-bool) s1)]
        (if (= (unify-failed s2) 1)
          (make-error-result-code (error-code-if-cond))
          ;; then 枝を推論
          (let [then-result (infer-expr then-node env s2 counter)]
            (if (= (result-failed then-result) 1)
              (propagate-error-result-with-span-and-name then-result)
              (let [s3 (result-subst then-result)
                then-ty (result-type then-result)
                ;; else 枝を推論
                else-result (infer-expr else-node env s3 counter)]
                (if (= (result-failed else-result) 1)
                  (propagate-error-result-with-span-and-name else-result)
                  ;; then と else の型を統一
                  (let [s4 (result-subst else-result)
                    else-ty (result-type else-result)
                    s5 (unify (apply-subst s4 then-ty) else-ty s4)]
                    (if (= (unify-failed s5) 1)
                      (make-error-result-code (error-code-if-branch))
                      (make-result s5 (apply-subst s5 else-ty)))))))))))))

;; ann 式の型推論
;; [11, expr, raw-type-expr]。旧 AST の payload 不在は内側の式をそのまま返す。
(defn typeinfer-record-constructor-result-type [ty]
  (if (= (ty-tag ty) (ty-fun))
    (typeinfer-record-constructor-result-type (ty-fr ty))
    (if (= (ty-tag ty) (ty-record)) ty 0)))

(defn typeinfer-visible-record-annotation-type [type-expr env counter]
  (if (= (vector-get type-expr 0) (tag-type-named))
    (let [scheme (type-env-lookup env (vector-get type-expr 1))]
      (if (= scheme 0)
        0
        (do
          (root_push scheme)
          (let [instantiated (instantiate scheme counter)]
            (do
              (root_push instantiated)
              (let [result (typeinfer-record-constructor-result-type instantiated)]
                (do
                  (root_pop)
                  (root_pop)
                  result)))))))
    0))

(defn infer-ann [node env subst counter]
  (let [expr-result (infer-expr (vector-get node 1) env subst counter)]
    (if (= (result-failed expr-result) 1)
      (propagate-error-result-with-span-and-name expr-result)
      (if (<= (vector-length node) 2)
        expr-result
        (let [type-expr (vector-get node 2)]
          (if (= type-expr 0)
            expr-result
            (let [s1 (result-subst expr-result)
              expr-ty (result-type expr-result)
              alias-env (var-counter-alias-env counter)]
              (do
                (root_push alias-env)
                (let [visible-record-ty
                        (typeinfer-visible-record-annotation-type type-expr env counter)
                  ann-ty
                    (if (= visible-record-ty 0)
                      (typeinfer-resolve-type-expr-with-aliases type-expr alias-env)
                      visible-record-ty)
                  s2 (unify expr-ty ann-ty s1)]
                  (do
                    (root_pop)
                    (if (= (unify-failed s2) 1)
                      (make-error-result-code (error-code-general))
                      (make-result s2 (apply-subst s2 ann-ty)))))))))))))

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
;; [20, name-hash, param-count, param-hash1, ..., body, signature?]
;; signature は [65, param-count, param-type-expr..., return-type-expr]。
;; compile-safe な covered slice として 0/1/2/3/4 引数を扱う

(defn infer-defn-parameterized-predeclared
  [node body-env final-env counter subst placeholder env-vars alias-env type-param-env]
  (do
    ;; parameterized defn の各中間値を、後続 allocation の前に root へ積む。
    (root_push node)
    (root_push body-env)
    (root_push final-env)
    (root_push counter)
    (root_push subst)
    (root_push placeholder)
    (root_push env-vars)
    (root_push alias-env)
    (root_push type-param-env)
    (let [name-hash (vector-get node 1)
      param-count (vector-get node 2)
      param-types (typeinfer-fresh-param-types param-count counter)]
      (do
        (root_push param-types)
        (let [body-node (vector-get node (+ param-count 3))]
          (do
            (root_push body-node)
            (let [next-env
                    (typeinfer-extend-env-with-node-params
                      body-env
                      node
                      param-count
                      3
                      param-types)]
              (do
                (root_push next-env)
                (let [annotated-param-subst
                        (typeinfer-defn-param-annotation-subst
                          node
                          param-count
                          param-types
                          subst
                          alias-env
                          type-param-env
                          next-env
                          counter)]
                  (do
                    (root_push annotated-param-subst)
                    (let [result
                            (if (= (unify-failed annotated-param-subst) 1)
                              (make-error-result-code (error-code-general))
                              (let [body-result
                                      (infer-expr
                                        body-node
                                        next-env
                                        annotated-param-subst
                                        counter)]
                                (if (= (result-failed body-result) 1)
                                  (propagate-error-result-with-span-and-name body-result)
                                  (let [s (result-subst body-result)
                                    body-ty (result-type body-result)
                                    annotated-subst
                                      (typeinfer-defn-return-annotation-subst
                                        node
                                        param-count
                                        body-ty
                                        s
                                        alias-env
                                        type-param-env
                                        next-env
                                        counter)]
                                    (if (= (unify-failed annotated-subst) 1)
                                      (make-error-result-code (error-code-general))
                                      (let [fun-ty
                                              (typeinfer-build-curried-fun
                                                param-types
                                                annotated-subst
                                                body-ty)]
                                        (do
                                          (root_push fun-ty)
                                          (let [next-subst
                                                  (unify
                                                    placeholder
                                                    fun-ty
                                                    annotated-subst)]
                                            (do
                                              (root_push next-subst)
                                              (let [final-result
                                                      (if (= (unify-failed next-subst) 1)
                                                        (make-error-result-code (error-code-general))
                                                        (typeinfer-finalize-defn-result-with-env-vars
                                                          final-env
                                                          name-hash
                                                          next-subst
                                                          fun-ty
                                                          env-vars))]
                                                (do
                                                  (root_pop)
                                                  (root_pop)
                                                  final-result)))))))))))]
                      (do
                        (root_pop)
                        (root_pop)
                        (root_pop)
                        (root_pop)
                        (root_pop)
                        (root_pop)
                        (root_pop)
                        (root_pop)
                        (root_pop)
                        (root_pop)
                        (root_pop)
                        (root_pop)
                        (root_pop)
                        result))))))))))))

(defn infer-defn-predeclared [node body-env final-env counter subst placeholder env-vars alias-env]
  (do
    ;; parameterized defn の推論中も、AST と共有環境を native GC から保持する。
    (root_push node)
    (root_push body-env)
    (root_push final-env)
    (root_push counter)
    (root_push subst)
    (root_push placeholder)
    (root_push env-vars)
    (root_push alias-env)
    (let [result
            (let [name-hash (vector-get node 1)
              param-count (vector-get node 2)
              type-param-env (typeinfer-defn-type-param-env node param-count counter)]
              ;; signature 内の各 scoped variable は独立した型変数として扱う。
              (if (= param-count 0)
                (let [body-node (vector-get node 3)
                  result (infer-expr body-node body-env subst counter)]
                  (if (= (result-failed result) 1)
                    (propagate-error-result-with-span-and-name result)
                    (let [s (result-subst result)
                      body-ty (result-type result)
                      annotated-subst
                        (typeinfer-defn-return-annotation-subst
                          node
                          param-count
                          body-ty
                          s
                          alias-env
                          type-param-env
                          body-env
                          counter)]
                      (if (= (unify-failed annotated-subst) 1)
                        (make-error-result-code (error-code-general))
                        (let [next-subst (unify placeholder body-ty annotated-subst)]
                          (if (= (unify-failed next-subst) 1)
                            (make-error-result-code (error-code-general))
                            (typeinfer-finalize-defn-result-with-env-vars final-env name-hash next-subst body-ty env-vars)))))))
                (infer-defn-parameterized-predeclared
                  node
                  body-env
                  final-env
                  counter
                  subst
                  placeholder
                  env-vars
                  alias-env
                  type-param-env)))]
      (do
        (root_pop)
        (root_pop)
        (root_pop)
        (root_pop)
        (root_pop)
        (root_pop)
        (root_pop)
        (root_pop)
        result))))

;; 単独で呼ばれる infer-defn も自己再帰を許可する。
(defn infer-defn [node env counter]
  (let [name-hash (vector-get node 1)
    placeholder (fresh-type-var counter)
    body-env (type-env-insert env name-hash (mono placeholder))
    alias-env (var-counter-alias-env counter)]
    (infer-defn-predeclared node body-env env counter (subst-new) placeholder (map-new) alias-env)))

;; 型変数ベクタを一般化除外用の Set へ移す。
(defn typeinfer-free-vars-to-set [vars idx len env-vars]
  (if (>= idx len)
    env-vars
    (typeinfer-free-vars-to-set
      vars
      (+ idx 1)
      len
      (map-insert-int-safe env-vars (vector-get vars idx) 1))))

;; private wrapper は同一 module 内の型推論では内側の宣言を使う。
(defn typeinfer-unprivate-defn [decl]
  (if (= (vector-get decl 0) (ast-private))
    (typeinfer-unprivate-defn (vector-get decl 1))
    decl))

;; 次の module に進む前に、先行 module の unqualified defn を型環境から隠す。
(defn typeinfer-remove-defns-before-module-loop [program env idx limit]
  (if (>= idx limit)
    env
    (let [decl (typeinfer-unprivate-defn (vector-get program idx))
      tag (vector-get decl 0)]
      (if (= tag (ast-defn))
        (typeinfer-remove-defns-before-module-loop
          program
          (type-env-remove env (vector-get decl 1))
          (+ idx 1)
          limit)
        (typeinfer-remove-defns-before-module-loop program env (+ idx 1) limit)))))

(defn typeinfer-remove-defns-before-module [program env limit]
  (typeinfer-remove-record-defs-before-module
    program
    (typeinfer-remove-defns-before-module-loop program env 0 limit)
    limit))

;; import の qualified key を、依存 module の public defn にだけ追加する。
;; :as があれば alias、無ければ module name を prefix に使う。
;; raw name hash は維持し、parser が保持する prefix/suffix hashから同じ keyを作る。
(defn typeinfer-import-alias-hash [decl]
  (if (> (vector-length decl) 4) (vector-get decl 4) 0))

(defn typeinfer-import-prefix-hash [decl]
  (let [alias-hash (typeinfer-import-alias-hash decl)]
    (if (= alias-hash 0) (vector-get decl 1) alias-hash)))

(defn typeinfer-import-only-hashes [decl]
  (if (> (vector-length decl) 5)
    (let [only-hashes (vector-get decl 5)]
      (if (= only-hashes 0) (vector-new 0) only-hashes))
    (vector-new 0)))

(defn typeinfer-import-open? [decl]
  (if (> (vector-length decl) 6)
    (if (= (vector-get decl 6) 1) 1 0)
    0))

(defn typeinfer-import-open-flag [decl]
  (if (and (> (vector-length decl) 6) (= (vector-get decl 6) 1)) 1 0))

(defn typeinfer-import-only-contains-loop [only-hashes idx len name-hash]
  (if (>= idx len)
    0
    (if (= (vector-get only-hashes idx) name-hash)
      1
      (typeinfer-import-only-contains-loop only-hashes (+ idx 1) len name-hash))))

(defn typeinfer-import-only-allows? [only-hashes name-hash]
  (if (= only-hashes 0)
    1
    (let [only-count (vector-length only-hashes)]
      (if (= only-count 0)
        1
        (typeinfer-import-only-contains-loop only-hashes 0 only-count name-hash)))))

(defn typeinfer-qualify-import-adt-variants-loop
  [variants idx len alias-hash only-hashes open-flag env]
  (if (>= idx len)
    env
    (let [variant (vector-get variants idx)
      name-hash (vector-get variant 0)]
      (if (= (typeinfer-import-only-allows? only-hashes name-hash) 1)
        (let [qualified-key (ast-qualified-name-hash alias-hash name-hash)
          scheme (type-env-lookup env name-hash)]
          (if (= scheme 0)
            (typeinfer-qualify-import-adt-variants-loop
              variants
              (+ idx 1)
              len
              alias-hash
              only-hashes
              open-flag
              env)
            (let [qualified-env (type-env-insert env qualified-key scheme)
              next-env
                (if (= open-flag 1)
                  (type-env-insert qualified-env name-hash scheme)
                  qualified-env)]
              (typeinfer-qualify-import-adt-variants-loop
                variants
                (+ idx 1)
                len
                alias-hash
                only-hashes
                open-flag
                next-env))))
        (typeinfer-qualify-import-adt-variants-loop
          variants
          (+ idx 1)
          len
          alias-hash
          only-hashes
          open-flag
          env)))))

(defn typeinfer-qualify-import-named-export
  [name-hash alias-hash only-hashes open-flag env]
  (if (= (typeinfer-import-only-allows? only-hashes name-hash) 1)
    (let [qualified-key (ast-qualified-name-hash alias-hash name-hash)
      scheme (type-env-lookup env name-hash)]
      (if (= scheme 0)
        env
        (let [qualified-env (type-env-insert env qualified-key scheme)]
          (if (= open-flag 1)
            (type-env-insert qualified-env name-hash scheme)
            qualified-env))))
    env))

(defn typeinfer-qualify-import-record-accessors-loop
  [raw-fields idx len alias-hash only-hashes open-flag env]
  (if (>= idx len)
    env
    (let [accessor-hash (vector-get raw-fields (+ idx 1))]
      (if (= (typeinfer-import-only-allows? only-hashes accessor-hash) 1)
        (let [qualified-key (ast-qualified-name-hash alias-hash accessor-hash)
          scheme (type-env-lookup env accessor-hash)]
          (if (= scheme 0)
            (typeinfer-qualify-import-record-accessors-loop
              raw-fields
              (+ idx 3)
              len
              alias-hash
              only-hashes
              open-flag
              env)
            (let [qualified-env (type-env-insert env qualified-key scheme)
              next-env
                (if (= open-flag 1)
                  (type-env-insert qualified-env accessor-hash scheme)
                  qualified-env)]
              (typeinfer-qualify-import-record-accessors-loop
                raw-fields
                (+ idx 3)
                len
                alias-hash
                only-hashes
                open-flag
                next-env))))
        (typeinfer-qualify-import-record-accessors-loop
          raw-fields
          (+ idx 3)
          len
          alias-hash
          only-hashes
          open-flag
          env)))))

(defn typeinfer-qualify-import-record-accessors
  [raw-fields alias-hash only-hashes open-flag env]
  (if (= raw-fields 0)
    env
    (typeinfer-qualify-import-record-accessors-loop
      raw-fields
      0
      (vector-length raw-fields)
      alias-hash
      only-hashes
      open-flag
      env)))

(defn typeinfer-qualify-import-source-step-state [done next-idx next-module env]
  (push-object-vector-local
    (push-int-vector-local
      (push-int-vector-local
        (push-int-vector-local (vector-new 4) done)
        next-idx)
      next-module)
    env))

;; source qualification の一要素だけを処理し、次の cursor を state で返す。
(defn typeinfer-qualify-import-source-step
  [program idx limit current-module target-module alias-hash only-hashes open-flag env record-env]
  (if (>= idx limit)
    (typeinfer-qualify-import-source-step-state 1 idx current-module env)
    (let [decl (vector-get program idx)
      tag (vector-get decl 0)]
      (if (= tag (ast-module-decl))
        (typeinfer-qualify-import-source-step-state
          0
          (+ idx 1)
          (vector-get decl 1)
          env)
        (if (and
              (= tag (ast-type-decl))
              (= current-module target-module))
          (let [variants (typeinfer-adt-decl-variants decl)]
            (if (= variants 0)
              (typeinfer-qualify-import-source-step-state
                0
                (+ idx 1)
                current-module
                env)
              (let [qualified-env
                      (typeinfer-qualify-import-adt-variants-loop
                        variants
                        0
                        (vector-length variants)
                        alias-hash
                        only-hashes
                        open-flag
                        env)]
                (typeinfer-qualify-import-source-step-state
                  1
                  limit
                  current-module
                  qualified-env))))
          (if (and
                (= tag (ast-recorddef))
                (= current-module target-module))
            (let [registered-env
                    (typeinfer-register-record-def decl env record-env)
              constructor-env
                    (typeinfer-qualify-import-named-export
                      (vector-get decl 1)
                      alias-hash
                      only-hashes
                      open-flag
                      registered-env)]
              (let [next-env
                      (typeinfer-qualify-import-record-accessors
                        (typeinfer-record-decl-field-exprs decl)
                        alias-hash
                        only-hashes
                        open-flag
                        constructor-env)]
                (let [clean-env
                        (typeinfer-clean-record-import-export
                          decl
                          only-hashes
                          open-flag
                          next-env)]
                  (typeinfer-qualify-import-source-step-state
                    0
                    (+ idx 1)
                    current-module
                    clean-env))))
            (if (and
                  (= tag (ast-defn))
                  (and
                    (= current-module target-module)
                    (= (typeinfer-import-only-allows?
                        only-hashes
                        (vector-get decl 1))
                      1)))
              (let [name-hash (vector-get decl 1)
                qualified-key (ast-qualified-name-hash alias-hash name-hash)
                raw-scheme (type-env-lookup env name-hash)
                scheme
                  (if (= raw-scheme 0)
                    (type-env-lookup env qualified-key)
                    raw-scheme)]
                (if (= scheme 0)
                  (typeinfer-qualify-import-source-step-state
                    0
                    (+ idx 1)
                    current-module
                    env)
                  (let [qualified-env (type-env-insert env qualified-key scheme)
                    next-env
                      (if (= open-flag 1)
                        (type-env-insert qualified-env name-hash scheme)
                        qualified-env)]
                    (typeinfer-qualify-import-source-step-state
                      0
                      (+ idx 1)
                      current-module
                      next-env))))
              (typeinfer-qualify-import-source-step-state
                0
                (+ idx 1)
                current-module
                env))))))))

(defn typeinfer-qualify-import-source-loop-bounded
  [program idx limit current-module target-module alias-hash only-hashes open-flag env record-env remaining]
  (do
    (root_push program)
    (root_push only-hashes)
    (root_push env)
    (root_push record-env)
    (let [step
            (typeinfer-qualify-import-source-step
              program
              idx
              limit
              current-module
              target-module
              alias-hash
              only-hashes
              open-flag
              env
              record-env)
      done (vector-get step 0)
      next-idx (vector-get step 1)
      next-module (vector-get step 2)
      next-env (vector-get step 3)]
      (do
        (root_push step)
        (root_push next-env)
        (let [result
                (if (= done 1)
                  step
                  (if (<= remaining 1)
                    step
                    (typeinfer-qualify-import-source-loop-bounded
                      program
                      next-idx
                      limit
                      next-module
                      target-module
                      alias-hash
                      only-hashes
                      open-flag
                      next-env
                      record-env
                      (- remaining 1))))]
          (do
            (root_pop)
            (root_pop)
            (root_pop)
            (root_pop)
            (root_pop)
            (root_pop)
            result))))))

(defn typeinfer-qualify-import-source-loop-64
  [program idx limit current-module target-module alias-hash only-hashes open-flag env record-env]
  (typeinfer-qualify-import-source-loop-bounded
    program
    idx
    limit
    current-module
    target-module
    alias-hash
    only-hashes
    open-flag
    env
    record-env
    64))

(defn typeinfer-qualify-import-source-loop
  [program idx limit current-module target-module alias-hash only-hashes open-flag env record-env]
  (let [step
          (typeinfer-qualify-import-source-loop-64
            program
            idx
            limit
            current-module
            target-module
            alias-hash
            only-hashes
            open-flag
            env
            record-env)
    done (vector-get step 0)]
    (if (= done 1)
      (vector-get step 3)
      (do
        (root_push program)
        (root_push only-hashes)
        (root_push record-env)
        (root_push step)
        (let [result
                (typeinfer-qualify-import-source-loop
                  program
                  (vector-get step 1)
                  limit
                  (vector-get step 2)
                  target-module
                  alias-hash
                  only-hashes
                  open-flag
                  (vector-get step 3)
                  record-env)]
          (do
            (root_pop)
            (root_pop)
            (root_pop)
            (root_pop)
            result))))))

(defn typeinfer-qualify-imports-loop-with-open
  [program idx len env allow-open record-env]
  (if (>= idx len)
    env
    (let [decl (vector-get program idx)
      tag (vector-get decl 0)]
      (if (= tag (ast-import-decl))
        (let [prefix-hash (typeinfer-import-prefix-hash decl)
          only-hashes (typeinfer-import-only-hashes decl)
          open-flag (if (= allow-open 1) (typeinfer-import-open-flag decl) 0)
          qualified-env
            (typeinfer-qualify-import-source-loop
              program
              0
              idx
              0
              (vector-get decl 1)
              prefix-hash
              only-hashes
              open-flag
              env
              record-env)]
          (typeinfer-qualify-imports-loop-with-open
            program
            (+ idx 1)
            len
            qualified-env
            allow-open
            record-env))
        (typeinfer-qualify-imports-loop-with-open
          program
          (+ idx 1)
          len
          env
          allow-open
          record-env)))))

(defn typeinfer-predeclare-qualified-imports [program env record-env]
  (typeinfer-qualify-imports-loop-with-open
    program
    0
    (vector-length program)
    env
    0
    record-env))

(defn typeinfer-predeclare-qualified-imports-for-module
  [program start end env record-env]
  (typeinfer-qualify-imports-loop-with-open
    program
    start
    end
    env
    1
    record-env))

(defn typeinfer-next-module-index [program idx len]
  (if (>= idx len)
    len
    (if (= (vector-get (vector-get program idx) 0) (ast-module-decl))
      idx
      (typeinfer-next-module-index program (+ idx 1) len))))

(defn typeinfer-open-import-source [program import-idx decl env record-env]
  (if (= (typeinfer-import-open? decl) 1)
    (typeinfer-qualify-import-source-loop
      program
      0
      import-idx
      0
      (vector-get decl 1)
      (vector-get decl 1)
      (typeinfer-import-only-hashes decl)
      1
      env
      record-env)
    env))

;; 後続 top-level defn の placeholder に残る自由型変数を集める。
(defn typeinfer-pending-env-vars-loop [program idx len placeholders subst env-vars]
  (if (>= idx len)
    env-vars
    (let [decl (typeinfer-unprivate-defn (vector-get program idx))
      tag (vector-get decl 0)]
      (if (= tag 20)
        (let [name-hash (vector-get decl 1)
          placeholder (map-get-safe placeholders name-hash)
          resolved-placeholder (apply-subst subst placeholder)
          free (free-vars resolved-placeholder)
          next-env-vars (typeinfer-free-vars-to-set free 0 (vector-length free) env-vars)]
          (typeinfer-pending-env-vars-loop program (+ idx 1) len placeholders subst next-env-vars))
        (typeinfer-pending-env-vars-loop program (+ idx 1) len placeholders subst env-vars)))))

(defn typeinfer-pending-env-vars [program next-idx len placeholders subst]
  (typeinfer-pending-env-vars-loop program next-idx len placeholders subst (map-new)))

;; program 内の top-level defn 名を単相 placeholder として先に登録する。
(defn typeinfer-predeclare-defns-loop [program idx len env placeholders counter]
  (if (>= idx len)
    (push-object-vector-local (push-object-vector-local (vector-new 2) env) placeholders)
    (let [decl (typeinfer-unprivate-defn (vector-get program idx))
      tag (vector-get decl 0)]
      (if (= tag 20)
        (let [name-hash (vector-get decl 1)
          placeholder (fresh-type-var counter)
          next-env (type-env-insert env name-hash (mono placeholder))
          next-placeholders (map-insert-object-safe placeholders name-hash placeholder)]
          (typeinfer-predeclare-defns-loop program (+ idx 1) len next-env next-placeholders counter))
        (typeinfer-predeclare-defns-loop program (+ idx 1) len env placeholders counter)))))

(defn typeinfer-predeclare-defns [program env counter]
  (typeinfer-predeclare-defns-loop program 0 (vector-length program) env (map-new) counter))

;; type-alias の raw target を取得する。parametric alias は [tag, name, params, target] を使う。
(defn typeinfer-type-alias-target [decl]
  (if (<= (vector-length decl) 2)
    0
    (if (>= (vector-length decl) 4)
      (vector-get decl 3)
      (vector-get decl 2))))

;; Rust implementation と同じく、alias 自身を target に含む宣言は拒否する。
;; raw TypeExpr を直接走査するため、closed / parametric の両形式と nested target に対応する。
(defn typeinfer-type-expr-contains-name-range [type-expr idx end name-hash]
  (if (>= idx end)
    0
    (if (= (typeinfer-type-expr-contains-name (vector-get type-expr idx) name-hash) 1)
      1
      (typeinfer-type-expr-contains-name-range type-expr (+ idx 1) end name-hash))))

(defn typeinfer-type-expr-contains-name [type-expr name-hash]
  (if (= type-expr 0)
    0
    (let [tag (vector-get type-expr 0)]
      (if (= tag (ast-type-named))
        (if (= (vector-get type-expr 1) name-hash) 1 0)
        (if (= tag (ast-type-var))
          (if (= (vector-get type-expr 1) name-hash) 1 0)
          (if (= tag (ast-type-app))
            (if (= (vector-get type-expr 1) name-hash)
              1
              (typeinfer-type-expr-contains-name-range
                type-expr
                3
                (+ (vector-get type-expr 2) 3)
                name-hash))
            (if (= tag (ast-type-fun))
              (typeinfer-type-expr-contains-name-range
                type-expr
                2
                (+ (vector-get type-expr 1) 3)
                name-hash)
              0)))))))

(defn typeinfer-recursive-alias-count-loop [program idx len count]
  (if (>= idx len)
    count
    (let [decl (vector-get program idx)
      tag (vector-get decl 0)]
      (if (= tag (ast-typealias))
        (let [name-hash (vector-get decl 1)
          target-expr (typeinfer-type-alias-target decl)]
          (if (= (typeinfer-type-expr-contains-name target-expr name-hash) 1)
            (typeinfer-recursive-alias-count-loop program (+ idx 1) len (+ count 1))
            (typeinfer-recursive-alias-count-loop program (+ idx 1) len count)))
        (typeinfer-recursive-alias-count-loop program (+ idx 1) len count)))))

(defn typeinfer-recursive-alias-count [program]
  (typeinfer-recursive-alias-count-loop program 0 (vector-length program) 0))

;; parametric alias entry = [parameter-type-vars, resolved-target-type]
(defn typeinfer-make-parametric-alias-entry [param-types target-type]
  (do
    (root_push param-types)
    (root_push target-type)
    (let [with-param-types (push-object-vector-local (vector-new 2) param-types)]
      (do
        (root_push with-param-types)
        (let [result (push-object-vector-local with-param-types target-type)]
          (do
            (root_pop)
            (root_pop)
            (root_pop)
            result))))))

;; parametric alias の source parameter を fresh 型変数へ対応付ける。
;; 戻り値 = [param-name-to-type, param-types-in-source-order]
(defn typeinfer-build-parametric-alias-param-state-loop [params idx len counter param-env param-types]
  (if (>= idx len)
    (push-object-vector-local (push-object-vector-local (vector-new 2) param-env) param-types)
    (let [param-hash (vector-get params idx)
      param-type (fresh-type-var counter)
      next-param-env (map-insert-object-safe param-env param-hash param-type)
      next-param-types (push-object-vector-local param-types param-type)]
      (typeinfer-build-parametric-alias-param-state-loop
        params
        (+ idx 1)
        len
        counter
        next-param-env
        next-param-types))))

(defn typeinfer-build-parametric-alias-param-state [params counter]
  (typeinfer-build-parametric-alias-param-state-loop
    params
    0
    (vector-length params)
    counter
    (map-new)
    (vector-new (vector-length params))))

;; closed / parametric type-alias を source order で登録する。
(defn typeinfer-predeclare-closed-aliases-loop [program idx len closed-aliases parametric-aliases counter]
  (if (>= idx len)
    (make-type-alias-env closed-aliases parametric-aliases)
    (let [decl (vector-get program idx)
      tag (vector-get decl 0)]
      (if (= tag (ast-typealias))
        (let [name-hash (vector-get decl 1)
          target-expr (typeinfer-type-alias-target decl)]
            (if (= target-expr 0)
            (typeinfer-predeclare-closed-aliases-loop program (+ idx 1) len closed-aliases parametric-aliases counter)
            (if (>= (vector-length decl) 4)
              (let [params (vector-get decl 2)]
                (if (= (vector-length params) 0)
                  (let [alias-env (make-type-alias-env closed-aliases parametric-aliases)
                    target-type (typeinfer-resolve-type-expr-with-aliases target-expr alias-env)
                    next-closed-aliases (map-insert-object-safe closed-aliases name-hash target-type)]
                    (typeinfer-predeclare-closed-aliases-loop
                      program
                      (+ idx 1)
                      len
                      next-closed-aliases
                      parametric-aliases
                      counter))
                  (let [alias-env (make-type-alias-env closed-aliases parametric-aliases)
                    param-state (typeinfer-build-parametric-alias-param-state params counter)
                    param-env (vector-get param-state 0)
                    param-types (vector-get param-state 1)
                    target-type
                      (typeinfer-resolve-type-expr-with-aliases-and-params
                        target-expr
                        alias-env
                        param-env)
                    entry (typeinfer-make-parametric-alias-entry param-types target-type)
                    next-parametric-aliases (map-insert-object-safe parametric-aliases name-hash entry)]
                    (typeinfer-predeclare-closed-aliases-loop
                      program
                      (+ idx 1)
                      len
                      closed-aliases
                      next-parametric-aliases
                      counter))))
              (let [alias-env (make-type-alias-env closed-aliases parametric-aliases)
                target-type (typeinfer-resolve-type-expr-with-aliases target-expr alias-env)
                next-closed-aliases (map-insert-object-safe closed-aliases name-hash target-type)]
                (typeinfer-predeclare-closed-aliases-loop
                  program
                  (+ idx 1)
                  len
                  next-closed-aliases
                  parametric-aliases
                  counter)))))
        (typeinfer-predeclare-closed-aliases-loop program (+ idx 1) len closed-aliases parametric-aliases counter)))))

;; 初回 prepass 後に閉 alias だけを再評価する。parametric alias は fresh 型変数を
;; 持つため、ここでは再登録せず、forward な閉 alias chain だけを収束させる。
(defn typeinfer-refresh-closed-aliases-loop [program idx len closed-aliases parametric-aliases]
  (if (>= idx len)
    (make-type-alias-env closed-aliases parametric-aliases)
    (let [decl (vector-get program idx)
      tag (vector-get decl 0)]
      (if (= tag (ast-typealias))
        (let [target-expr (typeinfer-type-alias-target decl)
          is-closed
            (if (< (vector-length decl) 4)
              1
              (if (= (vector-length (vector-get decl 2)) 0) 1 0))]
          (if (= is-closed 1)
            (if (= target-expr 0)
              (typeinfer-refresh-closed-aliases-loop
                program (+ idx 1) len closed-aliases parametric-aliases)
              (let [alias-env (make-type-alias-env closed-aliases parametric-aliases)
                target-type (typeinfer-resolve-type-expr-with-aliases target-expr alias-env)
                next-closed-aliases
                  (map-insert-object-safe closed-aliases (vector-get decl 1) target-type)]
                (typeinfer-refresh-closed-aliases-loop
                  program
                  (+ idx 1)
                  len
                  next-closed-aliases
                  parametric-aliases)))
            (typeinfer-refresh-closed-aliases-loop
              program (+ idx 1) len closed-aliases parametric-aliases)))
        (typeinfer-refresh-closed-aliases-loop
          program (+ idx 1) len closed-aliases parametric-aliases)))))

(defn typeinfer-refresh-closed-aliases-rounds [program alias-env rounds]
  (if (>= rounds (vector-length program))
    alias-env
    (do
      (root_push alias-env)
      (let [closed-aliases (type-alias-env-closed alias-env)
        parametric-aliases (type-alias-env-parametric alias-env)]
        (do
          (root_push closed-aliases)
          (root_push parametric-aliases)
          (let [next-env
                  (typeinfer-refresh-closed-aliases-loop
                    program
                    0
                    (vector-length program)
                    closed-aliases
                    parametric-aliases)]
            (do
              (root_push next-env)
              (let [result
                      (typeinfer-refresh-closed-aliases-rounds
                        program
                        next-env
                        (+ rounds 1))]
                (do
                  (root_pop)
                  (root_pop)
                  (root_pop)
                  (root_pop)
                  result)))))))))

(defn typeinfer-predeclare-closed-aliases [program counter]
  (let [initial-env
          (typeinfer-predeclare-closed-aliases-loop
            program
            0
            (vector-length program)
            (map-new)
            (map-new)
            counter)]
    (do
      (root_push initial-env)
      (let [result (typeinfer-refresh-closed-aliases-rounds program initial-env 0)]
        (do
          (root_pop)
          result)))))

;; alias / record 宣言と本体推論で同じ型変数 ID 供給を共有する。
(defn typeinfer-make-alias-aware-counter [program]
  (let [bootstrap-counter (make-var-counter)]
    (do
      (root_push bootstrap-counter)
      (let [alias-env (typeinfer-predeclare-closed-aliases program bootstrap-counter)]
        (do
          (root_push alias-env)
          (let [record-env (typeinfer-predeclare-record-env program alias-env bootstrap-counter)]
            (do
              (root_push record-env)
              (let [result
                      (var-counter-with-alias-env-and-record-env
                        bootstrap-counter
                        alias-env
                        record-env)]
                (do
                  (root_pop)
                  (root_pop)
                  (root_pop)
                  result)))))))))

;; 先行する defn の failure kind を name hash から探す。
;; failure-kinds は defn の処理順に [0=success, 1=direct, 2=dependency] を保持する。
(defn typeinfer-prior-definition-failed-loop [program scan-idx defn-idx current-idx failure-kinds target-hash]
  (if (>= scan-idx current-idx)
    0
    (let [decl (typeinfer-unprivate-defn (vector-get program scan-idx))
      tag (vector-get decl 0)]
      (if (= tag 20)
        (if (= (vector-get decl 1) target-hash)
          (if (> (vector-get failure-kinds defn-idx) 0) 1 0)
          (typeinfer-prior-definition-failed-loop
            program
            (+ scan-idx 1)
            (+ defn-idx 1)
            current-idx
            failure-kinds
            target-hash))
        (typeinfer-prior-definition-failed-loop
          program
          (+ scan-idx 1)
          defn-idx
          current-idx
          failure-kinds
          target-hash)))))

(defn typeinfer-definition-failure-kind [program failure-kinds idx out]
  (if (= (result-error-code out) (error-code-undefined))
    (if (= (typeinfer-prior-definition-failed-loop
             program
             0
             0
             idx
             failure-kinds
             (result-error-name-hash out))
           1)
      2
      1)
    1))

;; program 推論の状態: [env, subst, first-type, first-seen, diagnostic-count, first-error-code, first-error-index, first-error-name-hash, first-error-start, first-error-end, failure-kinds]
(defn typeinfer-program-analysis-state-base [env subst first-ty first-seen diagnostic-count first-error-code first-error-index first-error-name-hash first-error-start first-error-end]
  (do
    ;; state の後続フィールドを追加する allocation 中も、先頭の型と
    ;; 中間ベクタを native GC から到達可能に保つ。
    (root_push env)
    (root_push subst)
    (root_push first-ty)
    (let [base (vector-new 10)]
      (do
        (root_push base)
        (let [with-env (push-object-vector-local base env)]
          (do
            (root_push with-env)
            (let [with-subst (push-object-vector-local with-env subst)]
              (do
                (root_push with-subst)
                (let [with-first-ty (push-object-vector-local with-subst first-ty)]
                  (do
                    (root_push with-first-ty)
                    (let [with-first-seen (push-int-vector-local with-first-ty first-seen)]
                      (do
                        (root_push with-first-seen)
                        (let [with-diagnostic-count
                                (push-int-vector-local with-first-seen diagnostic-count)]
                          (do
                            (root_push with-diagnostic-count)
                            (let [with-first-error-code
                                    (push-int-vector-local
                                      with-diagnostic-count
                                      first-error-code)]
                              (do
                                (root_push with-first-error-code)
                                (let [with-first-error-index
                                        (push-int-vector-local
                                          with-first-error-code
                                          first-error-index)]
                                  (do
                                    (root_push with-first-error-index)
                                    (let [with-first-error-name-hash
                                            (push-int-vector-local
                                              with-first-error-index
                                              first-error-name-hash)]
                                      (do
                                        (root_push with-first-error-name-hash)
                                        (let [with-first-error-start
                                                (push-int-vector-local
                                                  with-first-error-name-hash
                                                  first-error-start)]
                                          (do
                                            (root_push with-first-error-start)
                                            (let [result
                                                    (push-int-vector-local
                                                      with-first-error-start
                                                      first-error-end)]
                                              (do
                                                (root_pop)
                                                (root_pop)
                                                (root_pop)
                                                (root_pop)
                                                (root_pop)
                                                (root_pop)
                                                (root_pop)
                                                (root_pop)
                                                (root_pop)
                                                (root_pop)
                                                (root_pop)
                                                (root_pop)
                                                (root_pop)
                                                result))))))))))))))))))))))))

(defn typeinfer-program-analysis-state [env subst first-ty first-seen diagnostic-count first-error-code first-error-index first-error-name-hash first-error-start first-error-end failure-kinds]
  (push-object-vector-local
    (typeinfer-program-analysis-state-base
      env
      subst
      first-ty
      first-seen
      diagnostic-count
      first-error-code
      first-error-index
      first-error-name-hash
      first-error-start
      first-error-end)
    failure-kinds))

(defn typeinfer-program-analysis-state-with-env [state env]
  (do
    (root_push state)
    (root_push env)
    (let [result
            (typeinfer-program-analysis-state
              env
              (infer-program-analysis-subst state)
              (infer-program-analysis-raw-type state)
              (infer-program-analysis-first-seen state)
              (infer-program-analysis-diagnostic-count state)
              (infer-program-analysis-first-error-code state)
              (infer-program-analysis-first-error-index state)
              (infer-program-analysis-first-error-name-hash state)
              (infer-program-analysis-first-error-start state)
              (infer-program-analysis-first-error-end state)
              (infer-program-analysis-failure-kinds state))]
      (do
        (root_pop)
        (root_pop)
        result))))

(defn infer-program-analysis-env [analysis] (vector-get analysis 0))
(defn infer-program-analysis-subst [analysis] (vector-get analysis 1))
(defn infer-program-analysis-raw-type [analysis] (vector-get analysis 2))
(defn infer-program-analysis-first-seen [analysis] (vector-get analysis 3))
(defn infer-program-analysis-diagnostic-count [analysis] (vector-get analysis 4))
(defn infer-program-analysis-first-error-code [analysis] (vector-get analysis 5))
(defn infer-program-analysis-first-error-index [analysis] (vector-get analysis 6))
(defn infer-program-analysis-first-error-name-hash [analysis] (vector-get analysis 7))
(defn infer-program-analysis-first-error-start [analysis] (vector-get analysis 8))
(defn infer-program-analysis-first-error-end [analysis] (vector-get analysis 9))
(defn infer-program-analysis-failure-kinds [analysis] (vector-get analysis 10))

(defn infer-program-analysis-type [analysis]
  (do
    (root_push analysis)
    (let [result
            (apply-subst
              (infer-program-analysis-subst analysis)
              (infer-program-analysis-raw-type analysis))]
      (do
        (root_pop)
        result))))

(defn typeinfer-program-analysis-loop [program idx len placeholders counter alias-env state]
  (if (>= idx len)
    state
    (if (= (vector-get (vector-get program idx) 0) (ast-module-decl))
      (let [env (infer-program-analysis-env state)
        module-end (typeinfer-next-module-index program (+ idx 1) len)
        removed-env (typeinfer-remove-defns-before-module program env idx)
        visible-env
          (typeinfer-predeclare-qualified-imports-for-module
            program
            (+ idx 1)
            module-end
            removed-env
            (var-counter-record-env counter))]
        (do
          (root_push visible-env)
          (let [next-state
                  (typeinfer-program-analysis-state-with-env state visible-env)]
            (do
              (root_pop)
              (root_push next-state)
              (let [result
                      (typeinfer-program-analysis-loop
                        program
                        (+ idx 1)
                        len
                        placeholders
                        counter
                        alias-env
                        next-state)]
                (do
                  (root_pop)
                  result))))))
      (do
      ;; native backend では state / out と、その中から取り出した live object を
      ;; 次の allocation と再帰呼び出しの間も明示的に保持する。
      (root_push state)
      (let [decl (typeinfer-unprivate-defn (vector-get program idx))
        tag (vector-get decl 0)]
        (do
          (root_push decl)
          (let [next-state
                  (if (= tag (ast-import-decl))
                    (let [env (infer-program-analysis-env state)
                      open-env
                        (typeinfer-open-import-source
                          program
                          idx
                          decl
                          env
                          (var-counter-record-env counter))]
                      (do
                        (root_push open-env)
                        (let [next-state
                                (typeinfer-program-analysis-state-with-env state open-env)]
                          (do
                            (root_pop)
                            (root_push next-state)
                            (let [result
                                    (typeinfer-program-analysis-loop
                                      program
                                      (+ idx 1)
                                      len
                                      placeholders
                                      counter
                                      alias-env
                                      next-state)]
                              (do
                                (root_pop)
                                result))))))
                    (if (= tag 20)
                      (let [env (infer-program-analysis-env state)
                      subst (infer-program-analysis-subst state)
                      first-ty (infer-program-analysis-raw-type state)
                      first-seen (infer-program-analysis-first-seen state)
                      diagnostic-count (infer-program-analysis-diagnostic-count state)
                      first-error-code (infer-program-analysis-first-error-code state)
                      first-error-index (infer-program-analysis-first-error-index state)
                      first-error-name-hash (infer-program-analysis-first-error-name-hash state)
                      first-error-start (infer-program-analysis-first-error-start state)
                      first-error-end (infer-program-analysis-first-error-end state)
                      failure-kinds (infer-program-analysis-failure-kinds state)
                      name-hash (vector-get decl 1)
                      placeholder (map-get-safe placeholders name-hash)
                      pending-env-vars (typeinfer-pending-env-vars program (+ idx 1) len placeholders subst)
                      out (infer-defn-predeclared decl env env counter subst placeholder pending-env-vars alias-env)]
                      (do
                        (root_push out)
                        (let [next-first-ty (if (= first-seen 0) (result-type out) first-ty)]
                          (do
                            ;; result-type out は後続の環境/置換取得より先に保持する。
                            (root_push next-first-ty)
                            (let [next-first-seen (if (= first-seen 0) 1 first-seen)
                              next-first-error-code (if (= first-error-code 0) (result-error-code out) first-error-code)
                              next-env (if (= (result-failed out) 1)
                                (type-env-remove env name-hash)
                                (vector-get out 3))
                              next-subst (if (= (result-failed out) 1) subst (result-subst out))
                              next-first-error-index
                                (if (= (result-failed out) 1)
                                  (if (< first-error-index 0) idx first-error-index)
                                  first-error-index)
                              next-first-error-name-hash
                                (if (= (result-failed out) 1)
                                  (if (< first-error-index 0) name-hash first-error-name-hash)
                                  first-error-name-hash)
                              next-first-error-start
                                (if (= (result-failed out) 1)
                                  (if (< first-error-index 0) (result-error-start out) first-error-start)
                                  first-error-start)
                              next-first-error-end
                                (if (= (result-failed out) 1)
                                  (if (< first-error-index 0) (result-error-end out) first-error-end)
                                  first-error-end)
                              failure-kind
                                (if (= (result-failed out) 1)
                                  (typeinfer-definition-failure-kind program failure-kinds idx out)
                                  0)
                              next-failure-kinds (push-int-vector-local failure-kinds failure-kind)]
                              (do
                                (root_push next-env)
                                (root_push next-subst)
                                (let [result
                                        (typeinfer-program-analysis-state
                                          next-env
                                          next-subst
                                          next-first-ty
                                          next-first-seen
                                          (if (= (result-failed out) 1) (+ diagnostic-count 1) diagnostic-count)
                                          (if (= (result-failed out) 1) next-first-error-code first-error-code)
                                          next-first-error-index
                                          next-first-error-name-hash
                                          next-first-error-start
                                          next-first-error-end
                                          next-failure-kinds)]
                                  (do
                                    (root_push result)
                                    (let [recur-result
                                            (typeinfer-program-analysis-loop
                                              program
                                              (+ idx 1)
                                              len
                                              placeholders
                                              counter
                                              alias-env
                                              result)]
                                      (do
                                        (root_pop)
                                        (root_pop)
                                        (root_pop)
                                        (root_pop)
                                        (root_pop)
                                        recur-result))))))))))
                    (typeinfer-program-analysis-loop program (+ idx 1) len placeholders counter alias-env state)))]
            (do
              (root_pop)
              (root_pop)
              next-state))))))))

;; top-level defn を先行登録して一度だけ推論し、CLI/LSP が共有する結果を返す。
(defn infer-program-analysis [program]
  (do
    ;; program の子ノードを prepass / top-level inference の全 allocation 中も保持する。
    (root_push program)
    (let [recursive-count (typeinfer-recursive-alias-count program)]
      (if (> recursive-count 0)
        (let [result
                (typeinfer-program-analysis-state
                  (type-env-new)
                  (subst-new)
                  (mk-int)
                  1
                  recursive-count
                  (error-code-general)
                  -1
                  -1
                  -1
                  -1
                  (vector-new 0))]
          (do
            (root_pop)
            result))
        (let [counter (typeinfer-make-alias-aware-counter program)
          alias-env (var-counter-alias-env counter)]
          (do
            (root_push counter)
            (let [initial-env (init-builtin-env counter)
              record-env (typeinfer-register-record-defs program initial-env counter)
              adt-env (typeinfer-register-adt-defs program record-env counter)
              predeclared (typeinfer-predeclare-defns program adt-env counter)
              env0 (vector-get predeclared 0)
              env
                (typeinfer-predeclare-qualified-imports
                  program
                  env0
                  (var-counter-record-env counter))
              placeholders (vector-get predeclared 1)
              state (typeinfer-program-analysis-state env (subst-new) (mk-int) 0 0 0 -1 -1 -1 -1 (vector-new 0))
              result
                (typeinfer-program-analysis-loop
                  program
                  0
                  (vector-length program)
                  placeholders
                  counter
                  alias-env
                  state)]
              (do
                (root_pop)
                (root_pop)
                result))))))))

;; ============================================================
;; infer: 公開 API (Main.ls から呼び出される)
;; ============================================================

(defn infer [program]
  (infer-program-analysis-type (infer-program-analysis program)))

;; ============================================================
;; ビルトイン型環境の初期化
;; ============================================================

;; builtin env 本体は Types.TypeInferBuiltins へ分離
(defn init-builtin-env [counter]
  (typeinfer-init-builtin-env counter))
