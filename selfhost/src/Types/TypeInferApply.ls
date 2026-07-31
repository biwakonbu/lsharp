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
;; compile-safe な covered slice として 0-7 引数を扱う
(defn infer-apply-args-state [done next-idx payload arg-types]
  (vector-push-quad-rooted (vector-new 4) done next-idx payload arg-types))

(defn infer-apply-args-step-v3
  [node env counter argc idx subst arg-types]
  (if (>= idx argc)
    (infer-apply-args-state 1 idx subst arg-types)
    (do
      (root_push node)
      (root_push env)
      (root_push counter)
      (root_push subst)
      (root_push arg-types)
      (let [arg-result
              (infer-expr
                (vector-get node (+ idx 3))
                env
                subst
                counter)]
        (do
          (root_push arg-result)
          (if (= (result-failed arg-result) 1)
            (let [state
                    (infer-apply-args-state
                      2
                      idx
                      arg-result
                      arg-types)]
              (do
                (root_pop)
                (root_pop)
                (root_pop)
                (root_pop)
                (root_pop)
                (root_pop)
                state))
            (let [arg-ty (result-type arg-result)]
              (do
                (root_push arg-ty)
                (let [next-subst (result-subst arg-result)]
                  (do
                    (root_push next-subst)
                    (let [next-args
                            (push-object-vector-local arg-types arg-ty)]
                      (do
                        (root_push next-args)
                        (let [state
                                (infer-apply-args-state
                                  0
                                  (+ idx 1)
                                  next-subst
                                  next-args)]
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
                            state))))))))))))))

(defn infer-apply-args-step-64-loop-bounded
  [node env counter argc idx subst arg-types remaining]
  (do
    (root_push node)
    (root_push env)
    (root_push counter)
    (root_push argc)
    (root_push subst)
    (root_push arg-types)
    (let [step
            (infer-apply-args-step-v3
              node
              env
              counter
              argc
              idx
              subst
              arg-types)
      done (vector-get step 0)
      next-idx (vector-get step 1)
      payload (vector-get step 2)
      next-args (vector-get step 3)]
      (do
        (root_push step)
        (root_push payload)
        (root_push next-args)
        (let [parsed
                (if (>= done 1)
                  step
                  (if (<= remaining 1)
                    step
                    (infer-apply-args-step-64-loop-bounded
                      node
                      env
                      counter
                      argc
                      next-idx
                      payload
                      next-args
                      (- remaining 1))))]
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
            parsed))))))

(defn infer-apply-args-step-64
  [node env counter argc idx subst arg-types]
  (infer-apply-args-step-64-loop-bounded
    node
    env
    counter
    argc
    idx
    subst
    arg-types
    64))

(defn infer-apply-args-rooted-v3
  [node env counter argc idx subst arg-types]
  (let [step
          (infer-apply-args-step-64
            node
            env
            counter
            argc
            idx
            subst
            arg-types)]
    (if (>= (vector-get step 0) 1)
      step
      (do
        (root_push step)
        (let [next-idx (vector-get step 1)
          next-subst (vector-get step 2)
          next-args (vector-get step 3)]
          (do
            (root_push next-subst)
            (root_push next-args)
            (let [resolved
                    (infer-apply-args-rooted-v3
                      node
                      env
                      counter
                      argc
                      next-idx
                      next-subst
                      next-args)]
              (do
                (root_pop)
                (root_pop)
                (root_pop)
                resolved))))))))

(defn infer-apply-many-expected [arg-types subst ret-ty]
  (typeinfer-build-curried-fun arg-types subst ret-ty))

(defn infer-apply-many-final
  [node env subst counter argc func-ty]
  (do
    (root_push node)
    (root_push env)
    (root_push subst)
    (root_push counter)
    (root_push func-ty)
    (let [arg-types (vector-new 0)]
      (do
        (root_push arg-types)
        (let [state
                (infer-apply-args-rooted-v3
                  node
                  env
                  counter
                  argc
                  0
                  subst
                  arg-types)]
          (do
            (root_push state)
            (let [done (vector-get state 0)
              payload (vector-get state 2)
              collected (vector-get state 3)]
              (do
                (root_push payload)
                (root_push collected)
                (let [result
                        (if (= done 2)
                          (propagate-error-result payload)
                          (let [ret-ty (fresh-type-var counter)]
                            (do
                              (root_push ret-ty)
                              (let [expected
                                      (infer-apply-many-expected
                                        collected
                                        subst
                                        ret-ty)]
                                (do
                                  (root_push expected)
                                  (let [applied-func-ty
                                          (apply-subst subst func-ty)]
                                    (do
                                      (root_push applied-func-ty)
                                      (let [failure-code
                                              (if (= (type-tag applied-func-ty) 2)
                                                (if (= (occurs-check (type-name applied-func-ty) expected) 1)
                                                  (error-code-infinite)
                                                  (error-code-arg-mismatch))
                                                (error-code-arg-mismatch))
                                        next-subst
                                          (unify
                                            applied-func-ty
                                            expected
                                            subst)]
                                        (do
                                          (root_push next-subst)
                                          (let [final-result
                                                  (if (= (unify-failed next-subst) 1)
                                                    (make-error-result-code failure-code)
                                                    (make-result
                                                      next-subst
                                                      (apply-subst next-subst ret-ty)))]
                                            (do
                                              (root_pop)
                                              (root_pop)
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
                    result))))))))))

(defn infer-apply-many-rooted [node env subst counter argc]
  (do
    (root_push node)
    (root_push env)
    (root_push subst)
    (root_push counter)
    (let [func-result
            (infer-expr (vector-get node 1) env subst counter)]
      (do
        (root_push func-result)
        (if (= (result-failed func-result) 1)
          (let [result
                  (propagate-error-result-with-span-and-name func-result)]
            (do
              (root_pop)
              (root_pop)
              (root_pop)
              (root_pop)
              (root_pop)
              result))
          (let [s1 (result-subst func-result)
            func-ty (result-type func-result)]
            (do
              (root_push s1)
              (root_push func-ty)
              (let [next-result
                      (infer-apply-many-final
                        node
                        env
                        s1
                        counter
                        argc
                        func-ty)]
                (do
                  (root_push next-result)
                  (let [result next-result]
                    (do
                      (root_pop)
                      (root_pop)
                      (root_pop)
                      (root_pop)
                      (root_pop)
                      (root_pop)
                      (root_pop)
                      (root_pop)
                      result)))))))))))

(defn infer-apply-legacy-raw [node env subst counter]
  (let [func-node (vector-get node 1)
    argc (vector-get node 2)]
    (if (= argc 0)
      ;; 引数なし apply は Unit を渡す呼び出しとして扱う。
      (let [func-result (infer-expr func-node env subst counter)]
        (if (= (result-failed func-result) 1)
          (propagate-error-result-with-span-and-name func-result)
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
          (propagate-error-result-with-span-and-name func-result)
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
                              (propagate-error-result-with-span-and-name arg1-result)
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
              (make-error-result))))))))

;; 関数適用の型推論は複数の一時型・置換を確保するため、native GC の
;; collection 中も入力 AST と共有環境を保持する。
(defn infer-apply-raw [node env subst counter]
  (let [argc (vector-get node 2)]
    (if (= argc 2)
      (infer-apply-two-rooted node env subst counter)
      (if (>= argc 3)
        (if (<= argc 7)
          (infer-apply-many-rooted node env subst counter argc)
          (make-error-result))
        (infer-apply-legacy-raw node env subst counter)))))

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
