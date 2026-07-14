(module Types.TypeInferFunctions)
(import Syntax.AST)
(import Types.Type)
(import Types.TypeScheme)
(import Types.TypeInferCore)

;; lambda / defn の arity 依存処理を共有 helper へ集約する

(defn typeinfer-fresh-param-types-loop [count counter idx acc]
  (if (>= idx count)
    acc
    (typeinfer-fresh-param-types-loop
      count
      counter
      (+ idx 1)
      (vector-push acc (fresh-type-var counter)))))

(defn typeinfer-fresh-param-types [count counter]
  (typeinfer-fresh-param-types-loop count counter 0 (vector-new count)))

(defn typeinfer-extend-env-with-node-params-loop [env node count node-offset idx param-types]
  (if (>= idx count)
    env
    (let [param-hash (vector-get node (+ node-offset idx))
      param-ty (vector-get param-types idx)
      next-env (type-env-insert env param-hash (mono param-ty))]
      (typeinfer-extend-env-with-node-params-loop
        next-env
        node
        count
        node-offset
        (+ idx 1)
        param-types))))

(defn typeinfer-extend-env-with-node-params [env node count node-offset param-types]
  (typeinfer-extend-env-with-node-params-loop env node count node-offset 0 param-types))

;; defn signature は [65, param-count, param-type-expr..., return-type-expr]。
;; body の直後だけを参照し、後続の metadata とは区別する。
(defn typeinfer-defn-signature [node param-count]
  (let [signature-index (+ param-count 4)]
    (if (>= signature-index (vector-length node))
      0
      (let [candidate (vector-get node signature-index)]
        (if (= candidate 0)
          0
          (if (= (vector-get candidate 0) (ast-defn-signature))
            candidate
            0))))))

(defn typeinfer-defn-signature-param-expr [signature idx]
  (if (= signature 0)
    0
    (if (>= idx (vector-get signature 1))
      0
      (vector-get signature (+ idx 2)))))

(defn typeinfer-defn-signature-return-expr [signature]
  (if (= signature 0)
    0
    (vector-get signature (+ (vector-get signature 1) 2))))

(defn typeinfer-bind-signature-type-var [type-param-env name-hash counter]
  (let [bound (map-get-safe type-param-env name-hash)]
    (if (= bound 0)
      (map-insert-object-safe type-param-env name-hash (fresh-type-var counter))
      type-param-env)))

(defn typeinfer-collect-signature-type-expr-list
  [type-expr idx count counter type-param-env]
  (if (>= idx count)
    type-param-env
    (let [next-env
            (typeinfer-collect-signature-type-expr
              (vector-get type-expr (+ idx 2))
              counter
              type-param-env)]
      (typeinfer-collect-signature-type-expr-list
        type-expr
        (+ idx 1)
        count
        counter
        next-env))))

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

(defn typeinfer-unify-defn-param-annotations-loop
  [signature param-types idx count subst alias-env type-param-env]
  (if (>= idx count)
    subst
    (let [type-expr (typeinfer-defn-signature-param-expr signature idx)]
      (if (= type-expr 0)
        (typeinfer-unify-defn-param-annotations-loop
          signature
          param-types
          (+ idx 1)
          count
          subst
          alias-env
          type-param-env)
        (let [next-subst
                (unify
                  (vector-get param-types idx)
                  (typeinfer-resolve-type-expr-with-aliases-and-params
                    type-expr
                    alias-env
                    type-param-env)
                  subst)]
          (if (= (unify-failed next-subst) 1)
            next-subst
            (typeinfer-unify-defn-param-annotations-loop
              signature
              param-types
              (+ idx 1)
              count
              next-subst
              alias-env
              type-param-env)))))))

(defn typeinfer-defn-param-annotation-subst
  [node param-count param-types subst alias-env type-param-env]
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
        type-param-env))))

(defn typeinfer-defn-return-annotation-subst
  [node param-count body-ty subst alias-env type-param-env]
  (let [signature (typeinfer-defn-signature node param-count)]
    (if (= signature 0)
      subst
      (let [return-expr (typeinfer-defn-signature-return-expr signature)]
        (if (= return-expr 0)
          subst
          (unify
            body-ty
            (typeinfer-resolve-type-expr-with-aliases-and-params
              return-expr
              alias-env
              type-param-env)
            subst))))))

(defn typeinfer-build-curried-fun-loop [param-types subst idx count body-ty]
  (if (>= idx count)
    body-ty
    (let [rest-fun (typeinfer-build-curried-fun-loop param-types subst (+ idx 1) count body-ty)
      param-ty (vector-get param-types idx)]
      (mk-fun (apply-subst subst param-ty) rest-fun))))

(defn typeinfer-build-curried-fun [param-types subst body-ty]
  (typeinfer-build-curried-fun-loop param-types subst 0 (vector-length param-types) body-ty))

(defn typeinfer-finalize-defn-result-with-env-vars [env name-hash subst value-ty env-vars]
  (do
    ;; apply-subst / generalize / env insert の allocation 中も、型と環境を
    ;; native GC が回収しないように live object を明示的に保持する。
    (root_push env)
    (root_push subst)
    (root_push value-ty)
    (root_push env-vars)
    (let [resolved-ty (apply-subst subst value-ty)]
      (do
        (root_push resolved-ty)
        (let [scheme (generalize resolved-ty env-vars)]
          (do
            (root_push scheme)
            (let [new-env (type-env-insert env name-hash scheme)]
              (do
                (root_push new-env)
                ;; native backend でも result/env の戻り値 shape を壊さないよう、
                ;; make-result へ後付けせず rooted helper で4要素を構築する。
                (let [result
                        (push-object-vector-local
                          (push-int-vector-local
                            (push-object-vector-local
                              (push-object-vector-local (vector-new 4) subst)
                              resolved-ty)
                            0)
                          new-env)]
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

(defn typeinfer-finalize-defn-result [env name-hash subst value-ty]
  (typeinfer-finalize-defn-result-with-env-vars env name-hash subst value-ty (map-new)))
