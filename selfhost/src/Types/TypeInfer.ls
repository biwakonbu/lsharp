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

;; let 式の型推論
;; [7, name-hash, init-expr, body-expr]
(defn infer-let [node env subst counter]
  (let [name-hash (vector-get node 1)
    init-node (vector-get node 2)
    body-node (vector-get node 3)
    ;; init を推論
    init-result (infer-expr init-node env subst counter)]
    (if (= (result-failed init-result) 1)
      (propagate-error-result init-result)
      (let [s1 (result-subst init-result)
        init-ty (result-type init-result)
        ;; 汎化して環境に追加
        scheme (generalize (apply-subst s1 init-ty) (map-new))
        new-env (type-env-insert env name-hash scheme)]
        ;; body を推論
        (infer-expr body-node new-env s1 counter)))))

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

;; record field value 群を順に推論する
;; node は [tag, ..., field-count, field1-hash, expr1, ...]
(defn infer-record-fields [node idx count env subst counter]
  (if (= idx count)
    (make-result subst (mk-int))
    (let [value-node (vector-get node (+ 4 (* idx 2)))
      value-result (infer-expr value-node env subst counter)]
      (if (= (result-failed value-result) 1)
        (propagate-error-result value-result)
        (infer-record-fields
          node
          (+ idx 1)
          count
          env
          (result-subst value-result)
          counter)))))

;; record literal 用に field 型を保持しながら順に推論する
(defn infer-recordlit-fields [node idx count env subst counter record-ty]
  (make-result subst record-ty))

;; record literal から特定 field の value node を取り出す
;; 見つからない場合は 0 を返す
(defn recordlit-field-node-loop [record-node field-name-hash idx field-count]
  (if (>= idx field-count)
    0
    (let [field-offset (+ 3 (* idx 2))
      current-field-hash (vector-get record-node field-offset)]
      (if (= current-field-hash field-name-hash)
        (vector-get record-node (+ field-offset 1))
        (recordlit-field-node-loop
          record-node
          field-name-hash
          (+ idx 1)
          field-count)))))

(defn recordlit-field-node [record-node field-name-hash]
  (recordlit-field-node-loop
    record-node
    field-name-hash
    0
    (vector-get record-node 2)))

;; record literal の型推論
;; [12, type-name-hash, field-count, field1-hash, expr1, ...]
(defn infer-recordlit [node env subst counter]
  (let [type-name-hash (vector-get node 1)
    field-count (vector-get node 2)
    fields-result (infer-record-fields node 0 field-count env subst counter)]
    (if (= (result-failed fields-result) 1)
      (propagate-error-result fields-result)
      (make-result (result-subst fields-result) (mk-con type-name-hash)))))

;; field access の型推論
;; [13, expr, field-name-hash]
;; record 型なら対応フィールド型を返し、不明なら fresh var へ fallback する
(defn infer-fieldaccess [node env subst counter]
  (let [field-name-hash (vector-get node 2)
    base-node (vector-get node 1)]
    (if (= (vector-get base-node 0) (tag-recordlit))
      (let [field-node (recordlit-field-node base-node field-name-hash)]
        (if (= field-node 0)
          (make-result subst (fresh-type-var counter))
          (infer-expr field-node env subst counter)))
      (let [base-result (infer-expr base-node env subst counter)]
        (if (= (result-failed base-result) 1)
          (propagate-error-result base-result)
          (let [s1 (result-subst base-result)
            base-ty (apply-subst s1 (result-type base-result))]
            (if (= (ty-tag base-ty) (ty-record))
              (make-result s1 (mk-int))
              (make-result s1 (fresh-type-var counter)))))))))

;; record update の型推論
;; [14, base-expr, field-count, field1-hash, expr1, ...]
(defn infer-recordupdate-node [node env subst counter]
  (let [base-result (infer-expr (vector-get node 1) env subst counter)]
    (if (= (result-failed base-result) 1)
      (propagate-error-result base-result)
      (make-result (result-subst base-result) (result-type base-result)))))

;; computation expression の型推論
;; [15, builder-hash, step-count, step-kind1, aux1, expr1, ...]
;; 最小版: step を順に推論し、let! だけ束縛を環境へ追加して最後の式の型を返す
(defn infer-computation-steps [node idx step-count env subst counter last-ty]
  (make-result subst last-ty))

(defn infer-computation [node env subst counter]
  (let [step-count (vector-get node 2)]
    (if (= step-count 0)
      (make-result subst (mk-int))
      (if (= step-count 1)
        (infer-expr (vector-get node 5) env subst counter)
        (if (= step-count 2)
          (let [step1-kind (vector-get node 3)
            step1-aux (vector-get node 4)
            step1-expr (vector-get node 5)
            step2-kind (vector-get node 6)
            final-expr (vector-get node 8)]
            (if (= step2-kind (comp-step-return))
              (if (= step1-kind (comp-step-let-bang))
                (let [step1-result (infer-expr step1-expr env subst counter)]
                  (if (= (result-failed step1-result) 1)
                    (propagate-error-result step1-result)
                    (let [s1 (result-subst step1-result)
                      bound-ty (result-type step1-result)
                      env2 (type-env-insert env step1-aux (mono bound-ty))]
                      (infer-expr final-expr env2 s1 counter))))
                (if (= step1-kind (comp-step-do-bang))
                  (let [step1-result (infer-expr step1-expr env subst counter)]
                    (if (= (result-failed step1-result) 1)
                      (propagate-error-result step1-result)
                      (infer-expr final-expr env (result-subst step1-result) counter)))
                  (make-result subst (mk-int))))
              (make-result subst (mk-int))))
          (if (= step-count 3)
            (let [step1-kind (vector-get node 3)
              step1-aux (vector-get node 4)
              step1-expr (vector-get node 5)
              step2-kind (vector-get node 6)
              step2-aux (vector-get node 7)
              step2-expr (vector-get node 8)
              step3-kind (vector-get node 9)
              final-expr (vector-get node 11)]
              (if (= step3-kind (comp-step-return))
                (if (= step1-kind (comp-step-let-bang))
                  (let [step1-result (infer-expr step1-expr env subst counter)]
                    (if (= (result-failed step1-result) 1)
                      (propagate-error-result step1-result)
                      (let [s1 (result-subst step1-result)
                        bound1-ty (result-type step1-result)
                        env2 (type-env-insert env step1-aux (mono bound1-ty))]
                        (if (= step2-kind (comp-step-do-bang))
                          (let [step2-result (infer-expr step2-expr env2 s1 counter)]
                            (if (= (result-failed step2-result) 1)
                              (propagate-error-result step2-result)
                              (infer-expr final-expr env2 (result-subst step2-result) counter)))
                          (if (= step2-kind (comp-step-let-bang))
                            (let [step2-result (infer-expr step2-expr env2 s1 counter)]
                              (if (= (result-failed step2-result) 1)
                                (propagate-error-result step2-result)
                                (let [s2 (result-subst step2-result)
                                  bound2-ty (result-type step2-result)
                                  env3 (type-env-insert env2 step2-aux (mono bound2-ty))]
                                  (infer-expr final-expr env3 s2 counter))))
                            (make-result subst (mk-int)))))))
                  (if (= step1-kind (comp-step-do-bang))
                    (let [step1-result (infer-expr step1-expr env subst counter)]
                      (if (= (result-failed step1-result) 1)
                        (propagate-error-result step1-result)
                        (let [s1 (result-subst step1-result)]
                          (if (= step2-kind (comp-step-let-bang))
                            (let [step2-result (infer-expr step2-expr env s1 counter)]
                              (if (= (result-failed step2-result) 1)
                                (propagate-error-result step2-result)
                                (let [s2 (result-subst step2-result)
                                  bound2-ty (result-type step2-result)
                                  env2 (type-env-insert env step2-aux (mono bound2-ty))]
                                  (infer-expr final-expr env2 s2 counter))))
                            (if (= step2-kind (comp-step-do-bang))
                              (let [step2-result (infer-expr step2-expr env s1 counter)]
                                (if (= (result-failed step2-result) 1)
                                  (propagate-error-result step2-result)
                                  (infer-expr final-expr env (result-subst step2-result) counter)))
                              (make-result subst (mk-int)))))))
                    (make-result subst (mk-int))))
                (make-result subst (mk-int))))
            (make-result subst (mk-int))))))))

;; lambda 式の型推論
;; [8, param-count, param-hash1, ..., body]
;; compile-safe な covered slice として 0/1/2/3/4 引数を扱う
(defn infer-lambda [node env subst counter]
  (let [param-count (vector-get node 1)]
    (if (= param-count 0)
      (let [body-node (vector-get node 2)
        body-result (infer-expr body-node env subst counter)]
        (if (= (result-failed body-result) 1)
          (propagate-error-result body-result)
          (let [s1 (result-subst body-result)
            body-ty (result-type body-result)
            fun-ty (mk-fun (mk-unit) body-ty)]
            (make-result s1 fun-ty))))
      (let [param-types (typeinfer-fresh-param-types param-count counter)
        body-node (vector-get node (+ param-count 2))
        next-env (typeinfer-extend-env-with-node-params env node param-count 2 param-types)
        body-result (infer-expr body-node next-env subst counter)]
        (if (= (result-failed body-result) 1)
          (propagate-error-result body-result)
          (let [s1 (result-subst body-result)
            body-ty (result-type body-result)
            fun-ty (typeinfer-build-curried-fun param-types s1 body-ty)]
            (make-result s1 fun-ty)))))))

;; 関数適用の型推論
;; [5, func-node, arg-count, arg1, arg2, ...]
;; compile-safe な covered slice として 0-4 引数を扱う
(defn infer-apply [node env subst counter]
  (let [func-node (vector-get node 1)
    argc (vector-get node 2)]
    (if (= argc 0)
      ;; 引数なし: func を推論してそのまま返す
      (infer-expr func-node env subst counter)
      (let [func-result (infer-expr func-node env subst counter)]
        (if (= (result-failed func-result) 1)
          (propagate-error-result func-result)
          (let [s1 (result-subst func-result)
            func-ty (result-type func-result)]
            (if (= argc 1)
              ;; 1 引数の適用
              (let [arg1-result (infer-expr (vector-get node 3) env s1 counter)]
                (if (= (result-failed arg1-result) 1)
                  (propagate-error-result arg1-result)
                  (let [s2 (result-subst arg1-result)
                    arg1-ty (result-type arg1-result)
                    ret-ty (fresh-type-var counter)
                    expected (mk-fun arg1-ty ret-ty)
                    applied-func-ty (apply-subst s2 func-ty)
                    failure-code
                    (if (= (type-tag applied-func-ty) 2)
                      (if (= (occurs-check (type-name applied-func-ty) expected) 1)
                        (error-code-infinite)
                        (error-code-arg-mismatch))
                      (error-code-arg-mismatch))
                    s3 (unify applied-func-ty expected s2)]
                    (if (= (unify-failed s3) 1)
                      (make-error-result-code failure-code)
                      (make-result s3 (apply-subst s3 ret-ty))))))
              (if (= argc 2)
                ;; 2 引数の適用
                (let [arg1-result (infer-expr (vector-get node 3) env s1 counter)]
                  (if (= (result-failed arg1-result) 1)
                    (propagate-error-result arg1-result)
                    (let [s2 (result-subst arg1-result)
                      arg1-ty (result-type arg1-result)
                      arg2-result (infer-expr (vector-get node 4) env s2 counter)]
                      (if (= (result-failed arg2-result) 1)
                        (propagate-error-result arg2-result)
                        (let [s3 (result-subst arg2-result)
                          arg2-ty (result-type arg2-result)
                          ret-ty (fresh-type-var counter)
                          expected (mk-fun arg1-ty (mk-fun arg2-ty ret-ty))
                          applied-func-ty (apply-subst s3 func-ty)
                          failure-code
                          (if (= (type-tag applied-func-ty) 2)
                            (if (= (occurs-check (type-name applied-func-ty) expected) 1)
                              (error-code-infinite)
                              (error-code-arg-mismatch))
                            (error-code-arg-mismatch))
                          s4 (unify applied-func-ty expected s3)]
                          (if (= (unify-failed s4) 1)
                            (make-error-result-code failure-code)
                            (make-result s4 (apply-subst s4 ret-ty))))))))
                (if (= argc 3)
                  ;; 3 引数の適用
                  (let [arg1-result (infer-expr (vector-get node 3) env s1 counter)]
                    (if (= (result-failed arg1-result) 1)
                      (propagate-error-result arg1-result)
                      (let [s2 (result-subst arg1-result)
                        arg1-ty (result-type arg1-result)
                        arg2-result (infer-expr (vector-get node 4) env s2 counter)]
                        (if (= (result-failed arg2-result) 1)
                          (propagate-error-result arg2-result)
                          (let [s3 (result-subst arg2-result)
                            arg2-ty (result-type arg2-result)
                            arg3-result (infer-expr (vector-get node 5) env s3 counter)]
                            (if (= (result-failed arg3-result) 1)
                              (propagate-error-result arg3-result)
                              (let [s4 (result-subst arg3-result)
                                arg3-ty (result-type arg3-result)
                                ret-ty (fresh-type-var counter)
                                expected (mk-fun arg1-ty (mk-fun arg2-ty (mk-fun arg3-ty ret-ty)))
                                applied-func-ty (apply-subst s4 func-ty)
                                failure-code
                                (if (= (type-tag applied-func-ty) 2)
                                  (if (= (occurs-check (type-name applied-func-ty) expected) 1)
                                    (error-code-infinite)
                                    (error-code-arg-mismatch))
                                  (error-code-arg-mismatch))
                                s5 (unify applied-func-ty expected s4)]
                                (if (= (unify-failed s5) 1)
                                  (make-error-result-code failure-code)
                                  (make-result s5 (apply-subst s5 ret-ty))))))))))
                  (if (= argc 4)
                    ;; 4 引数の適用
                    (let [arg1-result (infer-expr (vector-get node 3) env s1 counter)]
                      (if (= (result-failed arg1-result) 1)
                        (propagate-error-result arg1-result)
                        (let [s2 (result-subst arg1-result)
                          arg1-ty (result-type arg1-result)
                          arg2-result (infer-expr (vector-get node 4) env s2 counter)]
                          (if (= (result-failed arg2-result) 1)
                            (propagate-error-result arg2-result)
                            (let [s3 (result-subst arg2-result)
                              arg2-ty (result-type arg2-result)
                              arg3-result (infer-expr (vector-get node 5) env s3 counter)]
                              (if (= (result-failed arg3-result) 1)
                                (propagate-error-result arg3-result)
                                (let [s4 (result-subst arg3-result)
                                  arg3-ty (result-type arg3-result)
                                  arg4-result (infer-expr (vector-get node 6) env s4 counter)]
                                  (if (= (result-failed arg4-result) 1)
                                    (propagate-error-result arg4-result)
                                    (let [s5 (result-subst arg4-result)
                                      arg4-ty (result-type arg4-result)
                                      ret-ty (fresh-type-var counter)
                                      expected (mk-fun arg1-ty (mk-fun arg2-ty (mk-fun arg3-ty (mk-fun arg4-ty ret-ty))))
                                      applied-func-ty (apply-subst s5 func-ty)
                                      failure-code
                                      (if (= (type-tag applied-func-ty) 2)
                                        (if (= (occurs-check (type-name applied-func-ty) expected) 1)
                                          (error-code-infinite)
                                          (error-code-arg-mismatch))
                                        (error-code-arg-mismatch))
                                      s6 (unify applied-func-ty expected s5)]
                                      (if (= (unify-failed s6) 1)
                                        (make-error-result-code failure-code)
                                        (make-result s6 (apply-subst s6 ret-ty))))))))))))
                    (if (= argc 5)
                      ;; 5 引数の適用
                      (let [arg1-result (infer-expr (vector-get node 3) env s1 counter)]
                        (if (= (result-failed arg1-result) 1)
                          (propagate-error-result arg1-result)
                          (let [s2 (result-subst arg1-result)
                            arg1-ty (result-type arg1-result)
                            arg2-result (infer-expr (vector-get node 4) env s2 counter)]
                            (if (= (result-failed arg2-result) 1)
                              (propagate-error-result arg2-result)
                              (let [s3 (result-subst arg2-result)
                                arg2-ty (result-type arg2-result)
                                arg3-result (infer-expr (vector-get node 5) env s3 counter)]
                                (if (= (result-failed arg3-result) 1)
                                  (propagate-error-result arg3-result)
                                  (let [s4 (result-subst arg3-result)
                                    arg3-ty (result-type arg3-result)
                                    arg4-result (infer-expr (vector-get node 6) env s4 counter)]
                                    (if (= (result-failed arg4-result) 1)
                                      (propagate-error-result arg4-result)
                                      (let [s5 (result-subst arg4-result)
                                        arg4-ty (result-type arg4-result)
                                        arg5-result (infer-expr (vector-get node 7) env s5 counter)]
                                        (if (= (result-failed arg5-result) 1)
                                          (propagate-error-result arg5-result)
                                          (let [s6 (result-subst arg5-result)
                                            arg5-ty (result-type arg5-result)
                                            ret-ty (fresh-type-var counter)
                                            expected (mk-fun arg1-ty (mk-fun arg2-ty (mk-fun arg3-ty (mk-fun arg4-ty (mk-fun arg5-ty ret-ty)))))
                                            applied-func-ty (apply-subst s6 func-ty)
                                            failure-code
                                            (if (= (type-tag applied-func-ty) 2)
                                              (if (= (occurs-check (type-name applied-func-ty) expected) 1)
                                                (error-code-infinite)
                                                (error-code-arg-mismatch))
                                              (error-code-arg-mismatch))
                                            s7 (unify applied-func-ty expected s6)]
                                            (if (= (unify-failed s7) 1)
                                              (make-error-result-code failure-code)
                                              (make-result s7 (apply-subst s7 ret-ty))))))))))))))
                      (if (= argc 6)
                        ;; 6 引数の適用
                        (let [arg1-result (infer-expr (vector-get node 3) env s1 counter)]
                          (if (= (result-failed arg1-result) 1)
                            (propagate-error-result arg1-result)
                            (let [s2 (result-subst arg1-result)
                              arg1-ty (result-type arg1-result)
                              arg2-result (infer-expr (vector-get node 4) env s2 counter)]
                              (if (= (result-failed arg2-result) 1)
                                (propagate-error-result arg2-result)
                                (let [s3 (result-subst arg2-result)
                                  arg2-ty (result-type arg2-result)
                                  arg3-result (infer-expr (vector-get node 5) env s3 counter)]
                                  (if (= (result-failed arg3-result) 1)
                                    (propagate-error-result arg3-result)
                                    (let [s4 (result-subst arg3-result)
                                      arg3-ty (result-type arg3-result)
                                      arg4-result (infer-expr (vector-get node 6) env s4 counter)]
                                      (if (= (result-failed arg4-result) 1)
                                        (propagate-error-result arg4-result)
                                        (let [s5 (result-subst arg4-result)
                                          arg4-ty (result-type arg4-result)
                                          arg5-result (infer-expr (vector-get node 7) env s5 counter)]
                                          (if (= (result-failed arg5-result) 1)
                                            (propagate-error-result arg5-result)
                                            (let [s6 (result-subst arg5-result)
                                              arg5-ty (result-type arg5-result)
                                              arg6-result (infer-expr (vector-get node 8) env s6 counter)]
                                              (if (= (result-failed arg6-result) 1)
                                                (propagate-error-result arg6-result)
                                                (let [s7 (result-subst arg6-result)
                                                  arg6-ty (result-type arg6-result)
                                                  ret-ty (fresh-type-var counter)
                                                  expected (mk-fun arg1-ty (mk-fun arg2-ty (mk-fun arg3-ty (mk-fun arg4-ty (mk-fun arg5-ty (mk-fun arg6-ty ret-ty))))))
                                                  applied-func-ty (apply-subst s7 func-ty)
                                                  failure-code
                                                  (if (= (type-tag applied-func-ty) 2)
                                                    (if (= (occurs-check (type-name applied-func-ty) expected) 1)
                                                      (error-code-infinite)
                                                      (error-code-arg-mismatch))
                                                    (error-code-arg-mismatch))
                                                  s8 (unify applied-func-ty expected s7)]
                                                  (if (= (unify-failed s8) 1)
                                                    (make-error-result-code failure-code)
                                                    (make-result s8 (apply-subst s8 ret-ty))))))))))))))))
                        (if (= argc 7)
                          ;; 7 引数の適用
                          (let [arg1-result (infer-expr (vector-get node 3) env s1 counter)]
                            (if (= (result-failed arg1-result) 1)
                              (propagate-error-result arg1-result)
                              (let [s2 (result-subst arg1-result)
                                arg1-ty (result-type arg1-result)
                                arg2-result (infer-expr (vector-get node 4) env s2 counter)]
                                (if (= (result-failed arg2-result) 1)
                                  (propagate-error-result arg2-result)
                                  (let [s3 (result-subst arg2-result)
                                    arg2-ty (result-type arg2-result)
                                    arg3-result (infer-expr (vector-get node 5) env s3 counter)]
                                    (if (= (result-failed arg3-result) 1)
                                      (propagate-error-result arg3-result)
                                      (let [s4 (result-subst arg3-result)
                                        arg3-ty (result-type arg3-result)
                                        arg4-result (infer-expr (vector-get node 6) env s4 counter)]
                                        (if (= (result-failed arg4-result) 1)
                                          (propagate-error-result arg4-result)
                                          (let [s5 (result-subst arg4-result)
                                            arg4-ty (result-type arg4-result)
                                            arg5-result (infer-expr (vector-get node 7) env s5 counter)]
                                            (if (= (result-failed arg5-result) 1)
                                              (propagate-error-result arg5-result)
                                              (let [s6 (result-subst arg5-result)
                                                arg5-ty (result-type arg5-result)
                                                arg6-result (infer-expr (vector-get node 8) env s6 counter)]
                                                (if (= (result-failed arg6-result) 1)
                                                  (propagate-error-result arg6-result)
                                                  (let [s7 (result-subst arg6-result)
                                                    arg6-ty (result-type arg6-result)
                                                    arg7-result (infer-expr (vector-get node 9) env s7 counter)]
                                                    (if (= (result-failed arg7-result) 1)
                                                      (propagate-error-result arg7-result)
                                                      (let [s8 (result-subst arg7-result)
                                                        arg7-ty (result-type arg7-result)
                                                        ret-ty (fresh-type-var counter)
                                                        expected (mk-fun arg1-ty (mk-fun arg2-ty (mk-fun arg3-ty (mk-fun arg4-ty (mk-fun arg5-ty (mk-fun arg6-ty (mk-fun arg7-ty ret-ty)))))))
                                                        applied-func-ty (apply-subst s8 func-ty)
                                                        failure-code
                                                        (if (= (type-tag applied-func-ty) 2)
                                                          (if (= (occurs-check (type-name applied-func-ty) expected) 1)
                                                            (error-code-infinite)
                                                            (error-code-arg-mismatch))
                                                          (error-code-arg-mismatch))
                                                        s9 (unify applied-func-ty expected s8)]
                                                        (if (= (unify-failed s9) 1)
                                                          (make-error-result-code failure-code)
                                                          (make-result s9 (apply-subst s9 ret-ty))))))))))))))))))
                          (make-error-result))))))))))))))

;; do ブロックの型推論
;; [9, expr-count, expr1, expr2, ...]
(defn infer-do [node env subst counter]
  (let [ec (vector-get node 1)]
    (if (= ec 0)
      ;; 空の do: Int(0) を返す
      (make-result subst (mk-int))
      (if (= ec 1)
        ;; 1 式
        (infer-expr (vector-get node 2) env subst counter)
        ;; 2 式以上: 各式を順次推論、最後の型を返す
        (let [r1 (infer-expr (vector-get node 2) env subst counter)]
          (if (= (result-failed r1) 1)
            (propagate-error-result r1)
            (let [s1 (result-subst r1)]
              (if (= ec 2)
                (infer-expr (vector-get node 3) env s1 counter)
                (let [r2 (infer-expr (vector-get node 3) env s1 counter)]
                  (if (= (result-failed r2) 1)
                    (propagate-error-result r2)
                    (let [s2 (result-subst r2)]
                      (if (= ec 3)
                        (infer-expr (vector-get node 4) env s2 counter)
                        (let [r3 (infer-expr (vector-get node 4) env s2 counter)]
                          (if (= (result-failed r3) 1)
                            (propagate-error-result r3)
                            (let [s3 (result-subst r3)]
                              (if (= ec 4)
                                (infer-expr (vector-get node 5) env s3 counter)
                                ;; covered slice として 5/6 式を扱う
                                (let [r4 (infer-expr (vector-get node 5) env s3 counter)]
                                  (if (= (result-failed r4) 1)
                                    (propagate-error-result r4)
                                    (let [s4 (result-subst r4)]
                                      (if (= ec 5)
                                        (infer-expr (vector-get node 6) env s4 counter)
                                        (if (= ec 6)
                                          (let [r5 (infer-expr (vector-get node 6) env s4 counter)]
                                            (if (= (result-failed r5) 1)
                                              (propagate-error-result r5)
                                              (infer-expr (vector-get node 7) env (result-subst r5) counter)))
                                          (if (= ec 7)
                                            (let [r5 (infer-expr (vector-get node 6) env s4 counter)]
                                              (if (= (result-failed r5) 1)
                                                (propagate-error-result r5)
                                                (let [s5 (result-subst r5)]
                                                  (infer-expr (vector-get node 8) env s5 counter))))
                                            (if (= ec 8)
                                              (let [r5 (infer-expr (vector-get node 6) env s4 counter)]
                                                (if (= (result-failed r5) 1)
                                                  (propagate-error-result r5)
                                                  (let [s5 (result-subst r5)
                                                    r6 (infer-expr (vector-get node 7) env s5 counter)]
                                                    (if (= (result-failed r6) 1)
                                                      (propagate-error-result r6)
                                                      (infer-expr (vector-get node 9) env (result-subst r6) counter)))))
                                              (if (= ec 9)
                                                (let [r5 (infer-expr (vector-get node 6) env s4 counter)]
                                                  (if (= (result-failed r5) 1)
                                                    (propagate-error-result r5)
                                                    (let [s5 (result-subst r5)
                                                      r6 (infer-expr (vector-get node 7) env s5 counter)]
                                                      (if (= (result-failed r6) 1)
                                                        (propagate-error-result r6)
                                                        (let [s6 (result-subst r6)]
                                                          (infer-expr (vector-get node 10) env s6 counter))))))
                                                (if (= ec 10)
                                                  (let [r5 (infer-expr (vector-get node 6) env s4 counter)]
                                                    (if (= (result-failed r5) 1)
                                                      (propagate-error-result r5)
                                                      (let [s5 (result-subst r5)
                                                        r6 (infer-expr (vector-get node 7) env s5 counter)]
                                                        (if (= (result-failed r6) 1)
                                                          (propagate-error-result r6)
                                                          (let [s6 (result-subst r6)
                                                            r7 (infer-expr (vector-get node 8) env s6 counter)]
                                                            (if (= (result-failed r7) 1)
                                                              (propagate-error-result r7)
                                                              (infer-expr (vector-get node 11) env (result-subst r7) counter)))))))
                                                  (if (= ec 11)
                                                    (let [r5 (infer-expr (vector-get node 6) env s4 counter)]
                                                      (if (= (result-failed r5) 1)
                                                        (propagate-error-result r5)
                                                        (let [s5 (result-subst r5)
                                                          r6 (infer-expr (vector-get node 7) env s5 counter)]
                                                          (if (= (result-failed r6) 1)
                                                            (propagate-error-result r6)
                                                            (let [s6 (result-subst r6)
                                                              r7 (infer-expr (vector-get node 8) env s6 counter)]
                                                              (if (= (result-failed r7) 1)
                                                                (propagate-error-result r7)
                                                                (let [s7 (result-subst r7)
                                                                  r8 (infer-expr (vector-get node 9) env s7 counter)]
                                                                  (if (= (result-failed r8) 1)
                                                                    (propagate-error-result r8)
                                                                    (infer-expr (vector-get node 12) env (result-subst r8) counter)))))))))
                                                    (if (= ec 12)
                                                      (let [r5 (infer-expr (vector-get node 6) env s4 counter)]
                                                        (if (= (result-failed r5) 1)
                                                          (propagate-error-result r5)
                                                          (let [s5 (result-subst r5)
                                                            r6 (infer-expr (vector-get node 7) env s5 counter)]
                                                            (if (= (result-failed r6) 1)
                                                              (propagate-error-result r6)
                                                              (let [s6 (result-subst r6)
                                                                r7 (infer-expr (vector-get node 8) env s6 counter)]
                                                                (if (= (result-failed r7) 1)
                                                                  (propagate-error-result r7)
                                                                  (let [s7 (result-subst r7)
                                                                    r8 (infer-expr (vector-get node 9) env s7 counter)]
                                                                    (if (= (result-failed r8) 1)
                                                                      (propagate-error-result r8)
                                                                      (let [s8 (result-subst r8)
                                                                        r9 (infer-expr (vector-get node 10) env s8 counter)]
                                                                        (if (= (result-failed r9) 1)
                                                                          (propagate-error-result r9)
                                                                          (infer-expr (vector-get node 13) env (result-subst r9) counter)))))))))))
                                                      (if (= ec 13)
                                                        (let [r5 (infer-expr (vector-get node 6) env s4 counter)]
                                                          (if (= (result-failed r5) 1)
                                                            (propagate-error-result r5)
                                                            (let [s5 (result-subst r5)
                                                              r6 (infer-expr (vector-get node 7) env s5 counter)]
                                                              (if (= (result-failed r6) 1)
                                                                (propagate-error-result r6)
                                                                (let [s6 (result-subst r6)
                                                                  r7 (infer-expr (vector-get node 8) env s6 counter)]
                                                                  (if (= (result-failed r7) 1)
                                                                    (propagate-error-result r7)
                                                                    (let [s7 (result-subst r7)
                                                                      r8 (infer-expr (vector-get node 9) env s7 counter)]
                                                                      (if (= (result-failed r8) 1)
                                                                        (propagate-error-result r8)
                                                                        (let [s8 (result-subst r8)
                                                                          r9 (infer-expr (vector-get node 10) env s8 counter)]
                                                                          (if (= (result-failed r9) 1)
                                                                            (propagate-error-result r9)
                                                                            (let [s9 (result-subst r9)
                                                                              r10 (infer-expr (vector-get node 11) env s9 counter)]
                                                                              (if (= (result-failed r10) 1)
                                                                                (propagate-error-result r10)
                                                                                (infer-expr (vector-get node 14) env (result-subst r10) counter)))))))))))))
                                                        (if (= ec 14)
                                                          (let [r5 (infer-expr (vector-get node 6) env s4 counter)]
                                                            (if (= (result-failed r5) 1)
                                                              (propagate-error-result r5)
                                                              (let [s5 (result-subst r5)
                                                                r6 (infer-expr (vector-get node 7) env s5 counter)]
                                                                (if (= (result-failed r6) 1)
                                                                  (propagate-error-result r6)
                                                                  (let [s6 (result-subst r6)
                                                                    r7 (infer-expr (vector-get node 8) env s6 counter)]
                                                                    (if (= (result-failed r7) 1)
                                                                      (propagate-error-result r7)
                                                                      (let [s7 (result-subst r7)
                                                                        r8 (infer-expr (vector-get node 9) env s7 counter)]
                                                                        (if (= (result-failed r8) 1)
                                                                          (propagate-error-result r8)
                                                                          (let [s8 (result-subst r8)
                                                                            r9 (infer-expr (vector-get node 10) env s8 counter)]
                                                                            (if (= (result-failed r9) 1)
                                                                              (propagate-error-result r9)
                                                                              (let [s9 (result-subst r9)
                                                                                r10 (infer-expr (vector-get node 11) env s9 counter)]
                                                                                (if (= (result-failed r10) 1)
                                                                                  (propagate-error-result r10)
                                                                                  (let [s10 (result-subst r10)
                                                                                    r11 (infer-expr (vector-get node 12) env s10 counter)]
                                                                                    (if (= (result-failed r11) 1)
                                                                                      (propagate-error-result r11)
                                                                                      (infer-expr (vector-get node 15) env (result-subst r11) counter)))))))))))))))
                                                          ;; 15 式以上は既存 fallback を維持
                                                          (infer-expr (vector-get node 6) env s4 counter))))))))))))))))))))))))))))))

;; ============================================================
;; infer-pattern: パターンの型推論
;; ============================================================
;; パターン種別:
;;   1 = リテラル整数パターン
;;   2 = リテラル真偽値パターン
;;   3 = リテラル文字列パターン
;;   4 = 変数パターン (ワイルドカード含む)
;;   11 = コンストラクタパターン (tag-pattern)
;;   12 = レコードパターン
;;
;; 引数:
;;   pat     - パターンノード [tag, ...]
;;   env     - 型環境
;;   subst   - 現在の置換
;;   counter - 型変数カウンタ
;; 戻り値:
;;   [subst, type, updated-env] - 更新された置換、パターンの型、束縛追加後の環境

(defn pattern-children-subst [r]
  (vector-get r 0))

(defn pattern-children-env [r]
  (vector-get r 1))

;; subpattern 群を左から処理して binder env を積み上げる
;; base-index + idx * stride が subpattern の位置
(defn infer-pattern-children [node idx count base-index stride env subst counter]
  (if (>= idx count)
    (vector-push (vector-push (vector-new 2) subst) env)
    (let [child (vector-get node (+ base-index (* idx stride)))
      child-info (infer-pattern child env subst counter)
      child-subst (pat-result-subst child-info)
      child-env (pat-result-env child-info)]
      (if (= (map-get child-subst -1) 1)
        (vector-push
          (vector-push
            (vector-new 2)
            (map-insert child-subst -2 (result-error-code child-info)))
          child-env)
        (infer-pattern-children
          node
          (+ idx 1)
          count
          base-index
          stride
          child-env
          child-subst
          counter)))))

;; constructor pattern の subpattern を左から処理し、
;; コンストラクタ引数型との unify を行って最終戻り型を返す
(defn infer-constructor-pattern-children [node idx count env subst counter ctor-ty]
  (let [current-ctor (apply-subst subst ctor-ty)]
    (if (>= idx count)
      (if (= (ty-tag current-ctor) (ty-fun))
        (vector-push (make-error-result-code (error-code-general)) env)
        (vector-push (make-result subst current-ctor) env))
      (if (= (ty-tag current-ctor) (ty-fun))
        (let [child (vector-get node (+ 3 idx))
          child-info (infer-pattern child env subst counter)
          child-subst (result-subst child-info)
          child-ty (result-type child-info)
          child-env (vector-get child-info 3)]
          (if (= (result-failed child-info) 1)
            child-info
            (let [next-ctor (apply-subst child-subst current-ctor)
              param-ty (ty-fp next-ctor)
              ret-ty (ty-fr next-ctor)
              s2 (unify child-ty param-ty child-subst)]
              (if (= (unify-failed s2) 1)
                (vector-push (make-error-result-code (error-code-general)) child-env)
                (infer-constructor-pattern-children
                  node
                  (+ idx 1)
                  count
                  child-env
                  s2
                  counter
                  ret-ty)))))
        (vector-push (make-error-result-code (error-code-general)) env))))
)

(defn infer-pattern [pat env subst counter]
  (let [tag (vector-get pat 0)]
    (if (= tag 1)
      ;; 整数リテラルパターン: 型は Int、環境変化なし
      (vector-push (make-result subst (mk-int)) env)
      (if (= tag 2)
        ;; 真偽値リテラルパターン: 型は Bool、環境変化なし
        (vector-push (make-result subst (mk-bool)) env)
        (if (= tag 3)
          ;; 文字列リテラルパターン: 型は String、環境変化なし
          (vector-push (make-result subst (mk-string)) env)
          (if (= tag 4)
            ;; legacy な変数パターン: 新しい型変数を割り当て
            (let [name-hash (vector-get pat 1)
              ty (fresh-type-var counter)
              scheme (mono ty)
              new-env (type-env-insert env name-hash scheme)]
              (vector-push (make-result subst ty) new-env))
            (if (= tag 40)
              ;; canonical なワイルドカードパターン: fresh var だけ返し、束縛は追加しない
              (let [ty (fresh-type-var counter)]
                (vector-push (make-result subst ty) env))
              (if (= tag 41)
                ;; canonical な変数パターン: 新しい型変数を割り当て
                (let [name-hash (vector-get pat 1)
                  ty (fresh-type-var counter)
                  scheme (mono ty)
                  new-env (type-env-insert env name-hash scheme)]
                  (vector-push (make-result subst ty) new-env))
                (if (= tag 42)
                  ;; canonical なリテラルパターン: [42, lit-node]
                  (let [lit-node (vector-get pat 1)
                    lit-tag (vector-get lit-node 0)]
                    (if (= lit-tag 1)
                      (vector-push (make-result subst (mk-int)) env)
                      (if (= lit-tag 2)
                        (vector-push (make-result subst (mk-bool)) env)
                        (if (= lit-tag 32)
                          (vector-push (make-result subst (mk-unit)) env)
                          (if (= lit-tag 3)
                            (vector-push (make-result subst (mk-string)) env)
                            (let [ty (fresh-type-var counter)]
                              (vector-push (make-result subst ty) env)))))))
                  (if (or (= tag 11) (= tag 43))
                    ;; コンストラクタパターン (tag-pattern / constructor-pattern)
                    ;; [11, ctor-name-hash, sub-pat-count, sub-pat1, ...]
                    (let [ctor-hash (vector-get pat 1)
                      ctor-scheme (type-env-lookup env ctor-hash)]
                      (if (= ctor-scheme 0)
                        ;; 未定義コンストラクタ: エラー
                        (vector-push
                          (make-error-result-code (error-code-undefined))
                          env)
                        (let [sub-count (vector-get pat 2)
                          ctor-ty (instantiate ctor-scheme counter)]
                          (infer-constructor-pattern-children
                            pat
                            0
                            sub-count
                            env
                            subst
                            counter
                            ctor-ty))))
                    (if (or (= tag 12) (= tag 44))
                      ;; レコードパターン
                      ;; [12, field-count, field-hash1, sub-pat1, ...]
                      (let [fc (vector-get pat 1)
                        child-info
                        (infer-pattern-children
                          pat 0 fc 3 2 env subst counter)
                        child-subst (pattern-children-subst child-info)
                        child-env (pattern-children-env child-info)]
                        (if (= (map-get child-subst -1) 1)
                          (vector-push
                            (make-error-result-code (map-get child-subst -2))
                            child-env)
                          ;; レコード全体の型はまだ最小版として fresh var
                          (let [ty (fresh-type-var counter)]
                            (vector-push (make-result child-subst ty) child-env))))
                      ;; 未知のパターン: 新しい型変数 (ワイルドカード扱い)
                      (let [ty (fresh-type-var counter)]
                        (vector-push (make-result subst ty) env)))))))))))))

;; infer-pattern の戻り値アクセサ
;; [subst, type, updated-env]
(defn pat-result-subst [r]
  (vector-get r 0))

(defn pat-result-type [r]
  (vector-get r 1))

(defn pat-result-env [r]
  (vector-get r 3))

;; match 式の型推論
;; [10, scrutinee, arm-count, pat1, body1, pat2, body2, ...]
;; binder は各 arm body にだけ見え、次の arm には漏らさない
(defn infer-match-arms [node idx arm-count env scrut-ty result-ty subst counter]
  (if (>= idx arm-count)
    (make-result subst (apply-subst subst result-ty))
    (let [pat (vector-get node (+ 3 (* idx 2)))
      body (vector-get node (+ 4 (* idx 2)))
      pat-info (infer-pattern pat env subst counter)
      pat-subst (pat-result-subst pat-info)
      pat-ty (pat-result-type pat-info)
      pat-env (pat-result-env pat-info)]
      (if (= (map-get pat-subst -1) 1)
        (propagate-error-result pat-info)
        (let [s2 (unify (apply-subst pat-subst scrut-ty) pat-ty pat-subst)]
          (if (= (unify-failed s2) 1)
            (make-error-result-code (error-code-general))
            (let [body-result (infer-expr body pat-env s2 counter)]
              (if (= (result-failed body-result) 1)
                (propagate-error-result body-result)
                (let [s3 (result-subst body-result)
                  body-ty (result-type body-result)
                  s4 (unify (apply-subst s3 result-ty) body-ty s3)]
                  (if (= (unify-failed s4) 1)
                    (make-error-result-code (error-code-general))
                    (infer-match-arms
                      node
                      (+ idx 1)
                      arm-count
                      env
                      scrut-ty
                      result-ty
                      s4
                      counter)))))))))))

(defn infer-match [node env subst counter]
  (let [scrutinee (vector-get node 1)
    arm-count (vector-get node 2)
    scrut-result (infer-expr scrutinee env subst counter)]
    (if (= (result-failed scrut-result) 1)
      (propagate-error-result scrut-result)
      (let [s1 (result-subst scrut-result)
        scrut-ty (result-type scrut-result)
        result-ty (fresh-type-var counter)]
        (infer-match-arms node 0 arm-count env scrut-ty result-ty s1 counter)))))

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
