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

(defn typeinfer-unify-defn-param-annotations-loop [signature param-types idx count subst]
  (if (>= idx count)
    subst
    (let [type-expr (typeinfer-defn-signature-param-expr signature idx)]
      (if (= type-expr 0)
        (typeinfer-unify-defn-param-annotations-loop signature param-types (+ idx 1) count subst)
        (let [next-subst (unify (vector-get param-types idx) (typeinfer-resolve-type-expr type-expr) subst)]
          (if (= (unify-failed next-subst) 1)
            next-subst
            (typeinfer-unify-defn-param-annotations-loop signature param-types (+ idx 1) count next-subst)))))))

(defn typeinfer-defn-param-annotation-subst [node param-count param-types subst]
  (let [signature (typeinfer-defn-signature node param-count)]
    (if (= signature 0)
      subst
      (typeinfer-unify-defn-param-annotations-loop signature param-types 0 param-count subst))))

(defn typeinfer-defn-return-annotation-subst [node param-count body-ty subst]
  (let [signature (typeinfer-defn-signature node param-count)]
    (if (= signature 0)
      subst
      (let [return-expr (typeinfer-defn-signature-return-expr signature)]
        (if (= return-expr 0)
          subst
          (unify body-ty (typeinfer-resolve-type-expr return-expr) subst))))))

(defn typeinfer-build-curried-fun-loop [param-types subst idx count body-ty]
  (if (>= idx count)
    body-ty
    (let [rest-fun (typeinfer-build-curried-fun-loop param-types subst (+ idx 1) count body-ty)
      param-ty (vector-get param-types idx)]
      (mk-fun (apply-subst subst param-ty) rest-fun))))

(defn typeinfer-build-curried-fun [param-types subst body-ty]
  (typeinfer-build-curried-fun-loop param-types subst 0 (vector-length param-types) body-ty))

(defn typeinfer-finalize-defn-result-with-env-vars [env name-hash subst value-ty env-vars]
  (let [resolved-ty (apply-subst subst value-ty)
    scheme (generalize resolved-ty env-vars)
    new-env (type-env-insert env name-hash scheme)]
    (vector-push (make-result subst resolved-ty) new-env)))

(defn typeinfer-finalize-defn-result [env name-hash subst value-ty]
  (typeinfer-finalize-defn-result-with-env-vars env name-hash subst value-ty (map-new)))
