(module Types.TypeInferSignature)
(import Syntax.AST)
(import Types.Type)
(import Types.TypeScheme)
(import Types.TypeInferCore)
(import Types.TypeInferFunctions)

;; defn signature の型変数収集と parameter annotation unify を bounded に処理する。

(defn typeinfer-bind-signature-type-var [type-param-env name-hash counter]
  (let [bound (map-get-safe type-param-env name-hash)]
    (if (= bound 0)
      (map-insert-object-safe type-param-env name-hash (fresh-type-var counter))
      type-param-env)))

(defn typeinfer-signature-loop-state [done next-idx result]
  (vector-push-triple-rooted (vector-new 3) done next-idx result))

(defn typeinfer-collect-signature-type-expr-list-step-v3
  [type-expr idx count counter type-param-env]
  (if (>= idx count)
    (typeinfer-signature-loop-state 1 idx type-param-env)
    (do
      (root_push type-expr)
      (root_push counter)
      (root_push type-param-env)
      (let [next-env
              (typeinfer-collect-signature-type-expr
                (vector-get type-expr (+ idx 2))
                counter
                type-param-env)]
        (do
          (root_push next-env)
          (let [state
                  (typeinfer-signature-loop-state 0 (+ idx 1) next-env)]
            (do
              (root_pop)
              (root_pop)
              (root_pop)
              (root_pop)
              state)))))))

(defn typeinfer-collect-signature-type-expr-list-step-64-loop-bounded
  [type-expr idx count counter type-param-env remaining]
  (do
    (root_push type-expr)
    (root_push counter)
    (root_push type-param-env)
    (let [step
            (typeinfer-collect-signature-type-expr-list-step-v3
              type-expr idx count counter type-param-env)
      done (vector-get step 0)
      next-idx (vector-get step 1)
      next-env (vector-get step 2)]
      (do
        (root_push step)
        (root_push next-env)
        (let [parsed
          (if (= done 1)
            step
            (if (<= remaining 1)
              step
              (typeinfer-collect-signature-type-expr-list-step-64-loop-bounded
                type-expr next-idx count counter next-env (- remaining 1))))]
          (do
            (root_pop)
            (root_pop)
            (root_pop)
            (root_pop)
            (root_pop)
            parsed))))))

(defn typeinfer-collect-signature-type-expr-list-step-64
  [type-expr idx count counter type-param-env]
  (typeinfer-collect-signature-type-expr-list-step-64-loop-bounded
    type-expr idx count counter type-param-env 64))

(defn typeinfer-collect-signature-type-expr-list-rooted-v3
  [type-expr idx count counter type-param-env]
  (let [step
          (typeinfer-collect-signature-type-expr-list-step-64
            type-expr idx count counter type-param-env)]
    (if (= (vector-get step 0) 1)
      (vector-get step 2)
      (do
        (root_push step)
        (let [next-idx (vector-get step 1)
          next-env (vector-get step 2)]
          (do
            (root_push next-env)
            (let [resolved
              (typeinfer-collect-signature-type-expr-list-rooted-v3
                type-expr next-idx count counter next-env)]
              (do
                (root_pop)
                (root_pop)
                resolved))))))))

(defn typeinfer-collect-signature-type-expr-list
  [type-expr idx count counter type-param-env]
  (typeinfer-collect-signature-type-expr-list-rooted-v3
    type-expr idx count counter type-param-env))

(defn typeinfer-collect-signature-type-expr [type-expr counter type-param-env]
  (if (= type-expr 0)
    type-param-env
    (let [tag (vector-get type-expr 0)]
      (if (= tag (ast-type-var))
        (typeinfer-bind-signature-type-var
          type-param-env
          (vector-get type-expr 1)
          counter)
        (if (= tag (ast-type-app))
          (typeinfer-collect-signature-type-expr-list
            type-expr
            1
            (vector-get type-expr 2)
            counter
            type-param-env)
          (if (= tag (ast-type-fun))
            (let [param-count (vector-get type-expr 1)
              after-params
                (typeinfer-collect-signature-type-expr-list
                  type-expr
                  0
                  param-count
                  counter
                  type-param-env)]
              (typeinfer-collect-signature-type-expr
                (vector-get type-expr (+ param-count 2))
                counter
                after-params))
            type-param-env))))))

(defn typeinfer-defn-type-param-env [node param-count counter]
  (let [signature (typeinfer-defn-signature node param-count)]
    (if (= signature 0)
      (map-new)
      (let [initial (map-new)
        after-params
          (typeinfer-collect-signature-type-expr-list
            signature
            0
            (vector-get signature 1)
            counter
            initial)]
        (typeinfer-collect-signature-type-expr
          (typeinfer-defn-signature-return-expr signature)
          counter
          after-params)))))

(defn typeinfer-unify-defn-param-annotations-step-v3
  [signature param-types idx count subst alias-env type-param-env env counter]
  (if (>= idx count)
    (typeinfer-signature-loop-state 1 idx subst)
    (do
      (root_push signature)
      (root_push param-types)
      (root_push subst)
      (root_push alias-env)
      (root_push type-param-env)
      (root_push env)
      (root_push counter)
      (let [type-expr (typeinfer-defn-signature-param-expr signature idx)]
        (if (= type-expr 0)
          (let [state (typeinfer-signature-loop-state 0 (+ idx 1) subst)]
            (do
              (root_pop)
              (root_pop)
              (root_pop)
              (root_pop)
              (root_pop)
              (root_pop)
              (root_pop)
              state))
          (let [resolved-type
                  (typeinfer-resolve-signature-type-expr
                    type-expr alias-env type-param-env env counter)]
            (do
              (root_push resolved-type)
              (let [next-subst
                      (unify
                        (vector-get param-types idx)
                        resolved-type
                        subst)]
                (do
                  (root_push next-subst)
                  (let [state
                          (if (= (unify-failed next-subst) 1)
                            (typeinfer-signature-loop-state 1 idx next-subst)
                            (typeinfer-signature-loop-state 0 (+ idx 1) next-subst))]
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
                      state)))))))))))

(defn typeinfer-unify-defn-param-annotations-step-64-loop-bounded
  [signature param-types idx count subst alias-env type-param-env env counter remaining]
  (do
    (root_push signature)
    (root_push param-types)
    (root_push subst)
    (root_push alias-env)
    (root_push type-param-env)
    (root_push env)
    (root_push counter)
    (let [step
            (typeinfer-unify-defn-param-annotations-step-v3
              signature param-types idx count subst alias-env type-param-env env counter)
      done (vector-get step 0)
      next-idx (vector-get step 1)
      next-subst (vector-get step 2)]
      (do
        (root_push step)
        (root_push next-subst)
        (let [parsed
          (if (= done 1)
            step
            (if (<= remaining 1)
              step
              (typeinfer-unify-defn-param-annotations-step-64-loop-bounded
                signature param-types next-idx count next-subst alias-env type-param-env env counter (- remaining 1))))]
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

(defn typeinfer-unify-defn-param-annotations-step-64
  [signature param-types idx count subst alias-env type-param-env env counter]
  (typeinfer-unify-defn-param-annotations-step-64-loop-bounded
    signature param-types idx count subst alias-env type-param-env env counter 64))

(defn typeinfer-unify-defn-param-annotations-rooted-v3
  [signature param-types idx count subst alias-env type-param-env env counter]
  (let [step
          (typeinfer-unify-defn-param-annotations-step-64
            signature param-types idx count subst alias-env type-param-env env counter)]
    (if (= (vector-get step 0) 1)
      (vector-get step 2)
      (do
        (root_push step)
        (let [next-idx (vector-get step 1)
          next-subst (vector-get step 2)]
          (do
            (root_push next-subst)
            (let [resolved
              (typeinfer-unify-defn-param-annotations-rooted-v3
                signature param-types next-idx count next-subst alias-env type-param-env env counter)]
              (do
                (root_pop)
                (root_pop)
                resolved))))))))

(defn typeinfer-unify-defn-param-annotations-loop
  [signature param-types idx count subst alias-env type-param-env env counter]
  (typeinfer-unify-defn-param-annotations-rooted-v3
    signature param-types idx count subst alias-env type-param-env env counter))

(defn typeinfer-defn-param-annotation-subst
  [node param-count param-types subst alias-env type-param-env env counter]
  (let [signature (typeinfer-defn-signature node param-count)]
    (if (= signature 0)
      subst
      (typeinfer-unify-defn-param-annotations-loop
        signature
        param-types
        0
        param-count
        subst
        alias-env
        type-param-env
        env
        counter))))

(defn typeinfer-defn-return-annotation-subst
  [node param-count body-ty subst alias-env type-param-env env counter]
  (let [signature (typeinfer-defn-signature node param-count)]
    (if (= signature 0)
      subst
      (let [return-expr (typeinfer-defn-signature-return-expr signature)]
        (if (= return-expr 0)
          subst
          (unify
            body-ty
            (typeinfer-resolve-signature-type-expr
              return-expr
              alias-env
              type-param-env
              env
              counter)
            subst))))))
