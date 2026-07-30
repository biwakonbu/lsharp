(module Types.TypeInferFunctions)
(import Syntax.AST)
(import Types.Type)
(import Types.TypeScheme)
(import Types.TypeInferCore)

;; lambda / defn の arity 依存処理を共有 helper へ集約する

(defn typeinfer-fresh-param-types-state [done next-idx result]
  (vector-push-triple-rooted (vector-new 3) done next-idx result))

(defn typeinfer-fresh-param-types-step-v3 [count counter idx acc]
  (if (>= idx count)
    (typeinfer-fresh-param-types-state 1 idx acc)
    (do
      (root_push counter)
      (root_push acc)
      (let [next-acc (vector-push-single-rooted acc (fresh-type-var counter))]
        (do
          (root_push next-acc)
          (let [state (typeinfer-fresh-param-types-state 0 (+ idx 1) next-acc)]
            (do
              (root_pop)
              (root_pop)
              (root_pop)
              state)))))))

(defn typeinfer-fresh-param-types-step-64-loop-bounded
  [count counter idx acc remaining]
  (do
    (root_push counter)
    (root_push acc)
    (let [step (typeinfer-fresh-param-types-step-v3 count counter idx acc)
      done (vector-get step 0)
      next-idx (vector-get step 1)
      next-acc (vector-get step 2)]
      (do
        (root_push step)
        (root_push next-acc)
        (let [parsed
          (if (= done 1)
            step
            (if (<= remaining 1)
              step
              (typeinfer-fresh-param-types-step-64-loop-bounded
                count counter next-idx next-acc (- remaining 1))))]
          (do
            (root_pop)
            (root_pop)
            (root_pop)
            (root_pop)
            parsed))))))

(defn typeinfer-fresh-param-types-step-64 [count counter idx acc]
  (typeinfer-fresh-param-types-step-64-loop-bounded
    count counter idx acc 64))

(defn typeinfer-fresh-param-types-rooted-v3 [count counter idx acc]
  (let [step (typeinfer-fresh-param-types-step-64 count counter idx acc)]
    (if (= (vector-get step 0) 1)
      (vector-get step 2)
      (do
        (root_push step)
        (let [next-idx (vector-get step 1)
          next-acc (vector-get step 2)]
          (do
            (root_push next-acc)
            (let [resolved
              (typeinfer-fresh-param-types-rooted-v3
                count counter next-idx next-acc)]
              (do
                (root_pop)
                (root_pop)
                resolved))))))))

(defn typeinfer-fresh-param-types [count counter]
  (do
    (root_push counter)
    (let [result
      (typeinfer-fresh-param-types-rooted-v3
        count counter 0 (vector-new count))]
      (do
        (root_pop)
        result))))

(defn typeinfer-extend-env-with-node-params-step-v3
  [env node count node-offset idx-ref param-types]
  (if (>= (ref-get idx-ref) count)
    env
    (do
      (root_push env)
      (root_push node)
      (root_push idx-ref)
      (root_push param-types)
      (let [idx (ref-get idx-ref)
        param-hash (vector-get node (+ node-offset idx))
        param-ty (vector-get param-types idx)
        scheme (mono param-ty)]
        (do
          (root_push scheme)
          (let [next-env (type-env-insert env param-hash scheme)]
            (do
              (root_push next-env)
              (ref-set idx-ref (+ idx 1))
              (do
                (root_pop)
                (root_pop)
                (root_pop)
                (root_pop)
                (root_pop)
                (root_pop)
                next-env))))))))

(defn typeinfer-extend-env-with-node-params-step-64-loop-bounded
  [env node count node-offset idx-ref param-types remaining]
  (if (>= (ref-get idx-ref) count)
    env
    (if (<= remaining 0)
      env
      (do
        (root_push env)
        (root_push node)
        (root_push idx-ref)
        (root_push param-types)
        (let [next-env
          (typeinfer-extend-env-with-node-params-step-v3
            env node count node-offset idx-ref param-types)]
          (do
            (root_push next-env)
            (let [parsed
              (if (>= (ref-get idx-ref) count)
                next-env
                (if (<= remaining 1)
                  next-env
                  (typeinfer-extend-env-with-node-params-step-64-loop-bounded
                    next-env
                    node
                    count
                    node-offset
                    idx-ref
                    param-types
                    (- remaining 1))))]
              (do
                (root_pop)
                (root_pop)
                (root_pop)
                (root_pop)
                (root_pop)
                parsed))))))))

(defn typeinfer-extend-env-with-node-params-step-64
  [env node count node-offset idx-ref param-types]
  (typeinfer-extend-env-with-node-params-step-64-loop-bounded
    env node count node-offset idx-ref param-types 64))

(defn typeinfer-extend-env-with-node-params-rooted-v3
  [env node count node-offset idx-ref param-types]
  (let [next-env
    (typeinfer-extend-env-with-node-params-step-64
      env node count node-offset idx-ref param-types)]
    (if (>= (ref-get idx-ref) count)
      next-env
      (do
        (root_push next-env)
        (root_push node)
        (root_push idx-ref)
        (root_push param-types)
        (let [resolved
          (typeinfer-extend-env-with-node-params-rooted-v3
            next-env node count node-offset idx-ref param-types)]
          (do
            (root_pop)
            (root_pop)
            (root_pop)
            (root_pop)
            resolved))))))

(defn typeinfer-extend-env-with-node-params [env node count node-offset param-types]
  (do
    (root_push env)
    (root_push node)
    (root_push param-types)
    (let [idx-ref (ref-new 0)]
      (do
        (root_push idx-ref)
        (let [result
          (typeinfer-extend-env-with-node-params-rooted-v3
            env node count node-offset idx-ref param-types)]
          (do
            (root_pop)
            (root_pop)
            (root_pop)
            (root_pop)
            result))))))

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

;; defn signature の named record は、値環境へ登録済みの constructor scheme
;; の戻り値を使って解決する。未登録名と非-record scheme は既存の nominal resolverへ戻す。
(defn typeinfer-signature-record-result-type [ty]
  (if (= (ty-tag ty) (ty-fun))
    (typeinfer-signature-record-result-type (ty-fr ty))
    (if (= (ty-tag ty) (ty-record)) ty 0)))

(defn typeinfer-resolve-signature-app-args-state [done next-idx args]
  (vector-push-triple-rooted (vector-new 3) done next-idx args))

(defn typeinfer-resolve-signature-app-args-step-v3
  [type-expr idx count args alias-env type-param-env env counter]
  (if (>= idx count)
    (typeinfer-resolve-signature-app-args-state 1 idx args)
    (do
      (root_push args)
      (root_push alias-env)
      (root_push type-param-env)
      (root_push env)
      (root_push counter)
      (let [arg-type
        (typeinfer-resolve-signature-type-expr
          (vector-get type-expr (+ idx 3))
          alias-env
          type-param-env
          env
          counter)]
        (do
          (root_push arg-type)
          (let [next-args (push-object-vector-local args arg-type)]
            (do
              (root_pop)
              (root_pop)
              (root_pop)
              (root_pop)
              (root_pop)
              (root_pop)
              (typeinfer-resolve-signature-app-args-state
                0 (+ idx 1) next-args))))))))

(defn typeinfer-resolve-signature-app-args-step-64-loop-bounded
  [type-expr idx count args alias-env type-param-env env counter remaining]
  (do
    (root_push args)
    (root_push alias-env)
    (root_push type-param-env)
    (root_push env)
    (root_push counter)
    (let [step
      (typeinfer-resolve-signature-app-args-step-v3
        type-expr idx count args alias-env type-param-env env counter)
      done (vector-get step 0)
      next-idx (vector-get step 1)
      next-args (vector-get step 2)]
      (do
        (root_push step)
        (root_push next-args)
        (let [parsed
          (if (= done 1)
            step
            (if (<= remaining 1)
              step
              (typeinfer-resolve-signature-app-args-step-64-loop-bounded
                type-expr
                next-idx
                count
                next-args
                alias-env
                type-param-env
                env
                counter
                (- remaining 1))))]
          (do
            (root_pop)
            (root_pop)
            (root_pop)
            (root_pop)
            (root_pop)
            (root_pop)
            (root_pop)
            parsed))))))

(defn typeinfer-resolve-signature-app-args-step-64
  [type-expr idx count args alias-env type-param-env env counter]
  (typeinfer-resolve-signature-app-args-step-64-loop-bounded
    type-expr idx count args alias-env type-param-env env counter 64))

(defn typeinfer-resolve-signature-app-args-rooted-v3
  [type-expr idx count args alias-env type-param-env env counter]
  (let [step
    (typeinfer-resolve-signature-app-args-step-64
      type-expr idx count args alias-env type-param-env env counter)]
    (if (= (vector-get step 0) 1)
      (vector-get step 2)
      (do
        (root_push step)
        (let [next-idx (vector-get step 1)
          next-args (vector-get step 2)]
          (do
            (root_push next-args)
            (let [resolved
              (typeinfer-resolve-signature-app-args-rooted-v3
                type-expr next-idx count next-args alias-env type-param-env env counter)]
              (do
                (root_pop)
                (root_pop)
                resolved))))))))

(defn typeinfer-resolve-signature-app-args-loop
  [type-expr idx count args alias-env type-param-env env counter]
  (typeinfer-resolve-signature-app-args-rooted-v3
    type-expr idx count args alias-env type-param-env env counter))

(defn typeinfer-resolve-signature-app-type
  [type-expr alias-env type-param-env env counter]
  (do
    (root_push type-expr)
    (root_push alias-env)
    (root_push type-param-env)
    (let [name-hash (vector-get type-expr 1)
      arg-count (vector-get type-expr 2)
      args
      (typeinfer-resolve-signature-app-args-loop
        type-expr
        0
        arg-count
        (vector-new arg-count)
        alias-env
        type-param-env
        env
        counter)]
      (do
        (root_push args)
        (let [parametric-aliases (type-alias-env-parametric alias-env)]
          (do
            (root_push parametric-aliases)
            (let [entry (map-get-safe parametric-aliases name-hash)
              result
              (if (= entry 0)
                (mk-app (typeinfer-resolve-app-name name-hash) args)
                (do
                  (root_push entry)
                  (let [expanded
                    (typeinfer-resolve-parametric-alias-application
                      entry
                      args)]
                    (do
                      (root_pop)
                      (if (= expanded 0)
                        (mk-app (typeinfer-resolve-app-name name-hash) args)
                        expanded)))))]
              (do
                (root_pop)
                (root_pop)
                (root_pop)
                (root_pop)
                (root_pop)
                result))))))))

(defn typeinfer-resolve-signature-fun-params-state [done next-idx result]
  (vector-push-triple-rooted (vector-new 3) done next-idx result))

(defn typeinfer-resolve-signature-fun-params-step-v3
  [type-expr idx return-type alias-env type-param-env env counter]
  (if (<= idx 0)
    (typeinfer-resolve-signature-fun-params-state 1 idx return-type)
    (do
      (root_push return-type)
      (root_push alias-env)
      (root_push type-param-env)
      (root_push env)
      (root_push counter)
      (let [param-type
        (typeinfer-resolve-signature-type-expr
          (vector-get type-expr (+ idx 1))
          alias-env
          type-param-env
          env
          counter)]
        (do
          (root_push param-type)
          (let [next-result (mk-fun param-type return-type)]
            (do
              (root_pop)
              (root_pop)
              (root_pop)
              (root_pop)
              (root_pop)
              (root_pop)
              (typeinfer-resolve-signature-fun-params-state
                0 (- idx 1) next-result))))))))

(defn typeinfer-resolve-signature-fun-params-step-64-loop-bounded
  [type-expr idx return-type alias-env type-param-env env counter remaining]
  (do
    (root_push return-type)
    (root_push alias-env)
    (root_push type-param-env)
    (root_push env)
    (root_push counter)
    (let [step
      (typeinfer-resolve-signature-fun-params-step-v3
        type-expr idx return-type alias-env type-param-env env counter)
      done (vector-get step 0)
      next-idx (vector-get step 1)
      next-result (vector-get step 2)]
      (do
        (root_push step)
        (root_push next-result)
        (let [parsed
          (if (= done 1)
            step
            (if (<= remaining 1)
              step
              (typeinfer-resolve-signature-fun-params-step-64-loop-bounded
                type-expr
                next-idx
                next-result
                alias-env
                type-param-env
                env
                counter
                (- remaining 1))))]
          (do
            (root_pop)
            (root_pop)
            (root_pop)
            (root_pop)
            (root_pop)
            (root_pop)
            (root_pop)
            parsed))))))

(defn typeinfer-resolve-signature-fun-params-step-64
  [type-expr idx return-type alias-env type-param-env env counter]
  (typeinfer-resolve-signature-fun-params-step-64-loop-bounded
    type-expr idx return-type alias-env type-param-env env counter 64))

(defn typeinfer-resolve-signature-fun-params-rooted-v3
  [type-expr idx return-type alias-env type-param-env env counter]
  (let [step
    (typeinfer-resolve-signature-fun-params-step-64
      type-expr idx return-type alias-env type-param-env env counter)]
    (if (= (vector-get step 0) 1)
      (vector-get step 2)
      (do
        (root_push step)
        (let [next-idx (vector-get step 1)
          next-result (vector-get step 2)]
          (do
            (root_push next-result)
            (let [resolved
              (typeinfer-resolve-signature-fun-params-rooted-v3
                type-expr next-idx next-result alias-env type-param-env env counter)]
              (do
                (root_pop)
                (root_pop)
                resolved))))))))

(defn typeinfer-resolve-signature-fun-params-loop
  [type-expr idx count return-type alias-env type-param-env env counter]
  (typeinfer-resolve-signature-fun-params-rooted-v3
    type-expr count return-type alias-env type-param-env env counter))

(defn typeinfer-resolve-signature-fun-type
  [type-expr alias-env type-param-env env counter]
  (do
    (root_push type-expr)
    (root_push alias-env)
    (root_push type-param-env)
    (let [param-count (vector-get type-expr 1)
      return-type-expr (vector-get type-expr (+ param-count 2))
      return-type
      (typeinfer-resolve-signature-type-expr
        return-type-expr
        alias-env
        type-param-env
        env
        counter)]
      (do
        (root_push return-type)
        (let [result
          (typeinfer-resolve-signature-fun-params-loop
            type-expr
            0
            param-count
            return-type
            alias-env
            type-param-env
            env
            counter)]
          (do
            (root_pop)
            (root_pop)
            (root_pop)
            (root_pop)
            result))))))

(defn typeinfer-resolve-signature-type-expr
  [type-expr alias-env type-param-env env counter]
  (if (= (vector-get type-expr 0) (ast-type-named))
    (let [scheme (type-env-lookup env (vector-get type-expr 1))]
      (if (= scheme 0)
        (typeinfer-resolve-type-expr-with-aliases-and-params
          type-expr
          alias-env
          type-param-env)
        (do
          (root_push scheme)
          (let [instantiated (instantiate scheme counter)]
            (do
              (root_push instantiated)
              (let [record-ty (typeinfer-signature-record-result-type instantiated)]
                (do
                  (root_pop)
                  (root_pop)
                  (if (= record-ty 0)
                    (typeinfer-resolve-type-expr-with-aliases-and-params
                      type-expr
                      alias-env
                      type-param-env)
                    record-ty))))))))
    (if (= (vector-get type-expr 0) (ast-type-fun))
      (typeinfer-resolve-signature-fun-type
        type-expr
        alias-env
        type-param-env
        env
        counter)
      (if (= (vector-get type-expr 0) (ast-type-app))
        (typeinfer-resolve-signature-app-type
          type-expr
          alias-env
          type-param-env
          env
          counter)
        (typeinfer-resolve-type-expr-with-aliases-and-params
          type-expr
          alias-env
          type-param-env)))))

(defn typeinfer-make-fun-rooted [param-ty ret-ty]
  (do
    (root_push param-ty)
    (root_push ret-ty)
    (let [base (vector-new 3)
      result (vector-push-triple-rooted base (ty-fun) param-ty ret-ty)]
      (do
        (root_pop)
        (root_pop)
        result))))

(defn typeinfer-build-curried-fun-state [done next-idx result]
  (vector-push-triple-rooted (vector-new 3) done next-idx result))

(defn typeinfer-build-curried-fun-step-v3
  [param-types subst idx lower result]
  (if (<= idx lower)
    (typeinfer-build-curried-fun-state 1 idx result)
    (do
      (root_push param-types)
      (root_push subst)
      (root_push result)
      (let [param-ty (vector-get param-types (- idx 1))
        applied-param-ty (apply-subst subst param-ty)]
        (do
          (root_push applied-param-ty)
          (let [next-result
            (typeinfer-make-fun-rooted applied-param-ty result)]
            (do
              (root_pop)
              (root_pop)
              (root_pop)
              (root_pop)
              (typeinfer-build-curried-fun-state
                0
                (- idx 1)
                next-result))))))))

(defn typeinfer-build-curried-fun-step-64-loop-bounded
  [param-types subst idx lower result remaining]
  (do
    (root_push param-types)
    (root_push subst)
    (root_push result)
    (let [step
      (typeinfer-build-curried-fun-step-v3
        param-types subst idx lower result)
      done (vector-get step 0)
      next-idx (vector-get step 1)
      next-result (vector-get step 2)]
      (do
        (root_push step)
        (root_push next-result)
        (let [parsed
          (if (= done 1)
            step
            (if (<= remaining 1)
              step
              (typeinfer-build-curried-fun-step-64-loop-bounded
                param-types
                subst
                next-idx
                lower
                next-result
                (- remaining 1))))]
          (do
            (root_pop)
            (root_pop)
            (root_pop)
            (root_pop)
            (root_pop)
            parsed))))))

(defn typeinfer-build-curried-fun-step-64
  [param-types subst idx lower result]
  (typeinfer-build-curried-fun-step-64-loop-bounded
    param-types subst idx lower result 64))

(defn typeinfer-build-curried-fun-rooted-v3
  [param-types subst idx lower result]
  (let [step
    (typeinfer-build-curried-fun-step-64
      param-types subst idx lower result)]
    (if (= (vector-get step 0) 1)
      (vector-get step 2)
      (do
        (root_push step)
        (let [next-idx (vector-get step 1)
          next-result (vector-get step 2)]
          (do
            (root_push next-result)
            (let [resolved
              (typeinfer-build-curried-fun-rooted-v3
                param-types subst next-idx lower next-result)]
              (do
                (root_pop)
                (root_pop)
                resolved))))))))

(defn typeinfer-build-curried-fun-loop [param-types subst idx count body-ty]
  (typeinfer-build-curried-fun-rooted-v3
    param-types subst count idx body-ty))

(defn typeinfer-build-curried-fun [param-types subst body-ty]
  (do
    (root_push param-types)
    (root_push subst)
    (root_push body-ty)
    (let [result
      (typeinfer-build-curried-fun-loop
        param-types
        subst
        0
        (vector-length param-types)
        body-ty)]
      (do
        (root_pop)
        (root_pop)
        (root_pop)
        result))))

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
                    result))))))))))

(defn typeinfer-finalize-defn-result [env name-hash subst value-ty]
  (typeinfer-finalize-defn-result-with-env-vars env name-hash subst value-ty (map-new)))
