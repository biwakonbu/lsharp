(module Types.TypeInferFunctions)
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
