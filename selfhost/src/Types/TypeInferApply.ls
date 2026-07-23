(module Types.TypeInferApply)
(import Syntax.AST)
(import Types.Type)
(import Types.TypeScheme)
(import Types.TypeInferCore)
(import Types.TypeInferFunctions)
(import Types.TypeInfer)

;; TypeInferApply.ls - lambda 式と関数適用の型推論
;;
;; infer-lambda: lambda 式の型推論
;; infer-apply: 関数適用の型推論 (0-7 引数のアリティ分岐)

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

;; 1 引数 apply の成功結果を、解決済み戻り値型まで root 保持して構築する。
(defn infer-apply-one-success [s2 ret-ty]
  (do
    (root_push s2)
    (root_push ret-ty)
    (let [resolved-ret-ty (apply-subst s2 ret-ty)]
      (do
        (root_push resolved-ret-ty)
        (let [result (make-result s2 resolved-ret-ty)]
          (do
            (root_pop)
            (root_pop)
            (root_pop)
            result))))))

;; 1 引数 apply の unify 中間値を native GC から保持する。
(defn infer-apply-one-final [func-ty s1 arg1-ty counter]
  (do
    (root_push func-ty)
    (root_push s1)
    (root_push arg1-ty)
    (root_push counter)
    (let [ret-ty (fresh-type-var counter)]
      (do
        (root_push ret-ty)
        (let [expected (typeinfer-make-fun-rooted arg1-ty ret-ty)]
          (do
            (root_push expected)
            (let [applied-func-ty (apply-subst s1 func-ty)]
              (do
                (root_push applied-func-ty)
                (let [failure-code
                        (if (= (type-tag applied-func-ty) 2)
                          (if (= (occurs-check (type-name applied-func-ty) expected) 1)
                            (error-code-infinite)
                            (error-code-arg-mismatch))
                          (error-code-arg-mismatch))
                  s2 (unify applied-func-ty expected s1)]
                  (do
                    (root_push s2)
                    (let [result
                            (if (= (unify-failed s2) 1)
                              (make-error-result-code failure-code)
                              (infer-apply-one-success s2 ret-ty))]
                      (do
                        (root_pop)
                        (root_pop)
                        (root_pop)
                        (root_pop)
                        (root_pop)
                        (root_pop)
                        (root_pop)
                        (root_pop)
                        result))))))))))))

;; 2 引数 apply の期待関数型を、中間の curried function も保持して作る。
(defn infer-apply-two-expected [arg1-ty arg2-ty ret-ty]
  (do
    (root_push arg1-ty)
    (root_push arg2-ty)
    (root_push ret-ty)
    (let [rest-ty (typeinfer-make-fun-rooted arg2-ty ret-ty)]
      (do
        (root_push rest-ty)
        (let [result (typeinfer-make-fun-rooted arg1-ty rest-ty)]
          (do
            (root_pop)
            (root_pop)
            (root_pop)
            (root_pop)
            result))))))

(defn infer-apply-two-success [s4 ret-ty]
  (do
    (root_push s4)
    (root_push ret-ty)
    (let [resolved-ret-ty (apply-subst s4 ret-ty)]
      (do
        (root_push resolved-ret-ty)
        (let [result (make-result s4 resolved-ret-ty)]
          (do
            (root_pop)
            (root_pop)
            (root_pop)
            result))))))

(defn infer-apply-two-final [func-ty s3 arg1-ty arg2-ty counter]
  (do
    (root_push func-ty)
    (root_push s3)
    (root_push arg1-ty)
    (root_push arg2-ty)
    (root_push counter)
    (let [ret-ty (fresh-type-var counter)]
      (do
        (root_push ret-ty)
        (let [expected (infer-apply-two-expected arg1-ty arg2-ty ret-ty)]
          (do
            (root_push expected)
            (let [applied-func-ty (apply-subst s3 func-ty)]
              (do
                (root_push applied-func-ty)
                (let [failure-code
                        (if (= (type-tag applied-func-ty) 2)
                          (if (= (occurs-check (type-name applied-func-ty) expected) 1)
                            (error-code-infinite)
                            (error-code-arg-mismatch))
                          (error-code-arg-mismatch))
                  s4 (unify applied-func-ty expected s3)]
                  (do
                    (root_push s4)
                    (let [result
                            (if (= (unify-failed s4) 1)
                              (make-error-result-code failure-code)
                              (infer-apply-two-success s4 ret-ty))]
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
                        result))))))))))))

(defn infer-apply-two-after-arg2 [func-ty arg1-ty counter arg2-result]
  (do
    (root_push func-ty)
    (root_push arg1-ty)
    (root_push counter)
    (root_push arg2-result)
    (let [result
            (if (= (result-failed arg2-result) 1)
              (propagate-error-result arg2-result)
              (let [s3 (result-subst arg2-result)
                arg2-ty (result-type arg2-result)]
                (do
                  (root_push s3)
                  (root_push arg2-ty)
                  (let [next-result
                          (infer-apply-two-final
                            func-ty
                            s3
                            arg1-ty
                            arg2-ty
                            counter)]
                    (do
                      (root_pop)
                      (root_pop)
                      next-result)))))]
      (do
        (root_pop)
        (root_pop)
        (root_pop)
        (root_pop)
        result))))

(defn infer-apply-two-after-arg1 [node env subst counter func-ty arg1-result]
  (do
    (root_push node)
    (root_push env)
    (root_push subst)
    (root_push counter)
    (root_push func-ty)
    (root_push arg1-result)
    (let [result
            (if (= (result-failed arg1-result) 1)
              (propagate-error-result arg1-result)
              (let [s2 (result-subst arg1-result)
                arg1-ty (result-type arg1-result)]
                (do
                  (root_push s2)
                  (root_push arg1-ty)
                  (let [arg2-result
                          (infer-expr (vector-get node 4) env s2 counter)]
                    (do
                      (root_push arg2-result)
                      (let [next-result
                              (infer-apply-two-after-arg2
                                func-ty
                                arg1-ty
                                counter
                                arg2-result)]
                        (do
                          (root_pop)
                          (root_pop)
                          (root_pop)
                          next-result)))))))]
      (do
        (root_pop)
        (root_pop)
        (root_pop)
        (root_pop)
        (root_pop)
        (root_pop)
        result))))

(defn infer-apply-two-after-function [node env subst counter func-result]
  (do
    (root_push node)
    (root_push env)
    (root_push subst)
    (root_push counter)
    (root_push func-result)
    (let [result
            (if (= (result-failed func-result) 1)
              (propagate-error-result func-result)
              (let [s1 (result-subst func-result)
                func-ty (result-type func-result)]
                (do
                  (root_push s1)
                  (root_push func-ty)
                  (let [arg1-result
                          (infer-expr (vector-get node 3) env s1 counter)]
                    (do
                      (root_push arg1-result)
                      (let [next-result
                              (infer-apply-two-after-arg1
                                node
                                env
                                subst
                                counter
                                func-ty
                                arg1-result)]
                        (do
                          (root_pop)
                          (root_pop)
                          (root_pop)
                          next-result)))))))]
      (do
        (root_pop)
        (root_pop)
        (root_pop)
        (root_pop)
        (root_pop)
        result))))

(defn infer-apply-two-rooted [node env subst counter]
  (do
    (root_push node)
    (root_push env)
    (root_push subst)
    (root_push counter)
    (let [func-result
            (infer-expr (vector-get node 1) env subst counter)]
      (do
        (root_push func-result)
        (let [result
                (infer-apply-two-after-function
                  node
                  env
                  subst
                  counter
                  func-result)]
          (do
            (root_pop)
            (root_pop)
            (root_pop)
            (root_pop)
            (root_pop)
            result))))))

;; 関数適用の型推論
;; [5, func-node, arg-count, arg1, arg2, ...]
;; compile-safe な covered slice として 0-4 引数を扱う
(defn infer-apply-legacy-raw [node env subst counter]
  (let [func-node (vector-get node 1)
    argc (vector-get node 2)]
    (if (= argc 0)
      ;; 引数なし apply は Unit を渡す呼び出しとして扱う。
      (let [func-result (infer-expr func-node env subst counter)]
        (if (= (result-failed func-result) 1)
          (propagate-error-result-with-span func-result)
          (let [s1 (result-subst func-result)
            func-ty (result-type func-result)
            ret-ty (fresh-type-var counter)
            expected (mk-fun (mk-unit) ret-ty)
            applied-func-ty (apply-subst s1 func-ty)
            failure-code
            (if (= (type-tag applied-func-ty) 2)
              (if (= (occurs-check (type-name applied-func-ty) expected) 1)
                (error-code-infinite)
                (error-code-arg-mismatch))
              (error-code-arg-mismatch))
            s2 (unify applied-func-ty expected s1)]
            (if (= (unify-failed s2) 1)
              (make-error-result-code failure-code)
              (make-result s2 (apply-subst s2 ret-ty))))))
      (let [func-result (infer-expr func-node env subst counter)]
        (if (= (result-failed func-result) 1)
          (propagate-error-result-with-span func-result)
          (let [s1 (result-subst func-result)
            func-ty (result-type func-result)]
            (if (= argc 1)
              ;; 1 引数の適用
              (do
                (root_push s1)
                (root_push func-ty)
                (let [arg1-result (infer-expr (vector-get node 3) env s1 counter)]
                  (do
                    (root_push arg1-result)
                    (let [result
                            (if (= (result-failed arg1-result) 1)
                              (propagate-error-result arg1-result)
                              (let [s2 (result-subst arg1-result)
                                arg1-ty (result-type arg1-result)]
                                (do
                                  (root_push s2)
                                  (root_push arg1-ty)
                                  (let [next-result
                                          (infer-apply-one-final
                                            func-ty
                                            s2
                                            arg1-ty
                                            counter)]
                                    (do
                                      (root_pop)
                                      (root_pop)
                                      next-result)))))]
                      (do
                        (root_pop)
                        (root_pop)
                        (root_pop)
                        result)))))
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

;; 2 引数 application は中間値を保持する専用経路へ送る。
(defn infer-apply-raw [node env subst counter]
  (if (= (vector-get node 2) 2)
    (infer-apply-two-rooted node env subst counter)
    (infer-apply-legacy-raw node env subst counter)))

;; 関数適用の型推論は複数の一時型・置換を確保するため、native GC の
;; collection 中も入力 AST と共有環境を保持する。
(defn infer-apply [node env subst counter]
  (do
    (root_push node)
    (root_push env)
    (root_push subst)
    (root_push counter)
    (let [result (infer-apply-raw node env subst counter)]
      (do
        (root_pop)
        (root_pop)
        (root_pop)
        (root_pop)
        result))))
