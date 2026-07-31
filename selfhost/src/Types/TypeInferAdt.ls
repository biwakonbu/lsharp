(module Types.TypeInferAdt)
(import Syntax.AST)
(import Types.Type)
(import Types.TypeScheme)
(import Types.TypeInferCore)

;; TypeInferAdt.ls - ADT 宣言の constructor 型スキーム登録
;;
;; AST:
;;   nonparametric: [21, type-name, variants]
;;   parametric:    [21, type-name, params, variants]
;; variant:
;;   [constructor-name, raw-field-TypeExpr-vector]

(defn typeinfer-adt-decl-params [decl]
  (if (>= (vector-length decl) 4)
    (vector-get decl 2)
    (vector-new 0)))

(defn typeinfer-adt-decl-variants [decl]
  (if (>= (vector-length decl) 4)
    (vector-get decl 3)
    (if (> (vector-length decl) 2)
      (vector-get decl 2)
      0)))

(defn typeinfer-adt-make-param-state [param-env param-types bound-vars]
  (vector-push-triple-rooted (vector-new 3) param-env param-types bound-vars))

;; type parameter 名を fresh type variable と、scheme の bound variable ID へ対応付ける。
;; 1 chunk の深さを64に固定し、chunk間は param-state を continuation として渡す。
(defn typeinfer-adt-build-param-state-step-v3
  [params idx len counter param-env param-types bound-vars]
  (if (>= idx len)
    (vector-push-triple-rooted
      (vector-new 3)
      1
      idx
      (typeinfer-adt-make-param-state param-env param-types bound-vars))
    (let [param-hash (vector-get params idx)
      param-type (fresh-type-var counter)]
      (do
        (root_push param-env)
        (root_push param-types)
        (root_push bound-vars)
        (root_push param-type)
        (let [next-param-env (map-insert-object-safe param-env param-hash param-type)]
          (do
            (root_push next-param-env)
            (let [next-param-types (push-object-vector-local param-types param-type)]
              (do
                (root_push next-param-types)
                (let [next-bound-vars (push-int-vector-local bound-vars (ty-name param-type))]
                  (do
                    (root_push next-bound-vars)
                    (let [next-state
                            (typeinfer-adt-make-param-state
                              next-param-env
                              next-param-types
                              next-bound-vars)]
                      (do
                        (root_push next-state)
                        (let [state
                                (vector-push-triple-rooted
                                  (vector-new 3)
                                  0
                                  (+ idx 1)
                                  next-state)]
                          (do
                            (root_pop) (root_pop) (root_pop) (root_pop)
                            (root_pop) (root_pop) (root_pop) (root_pop)
                            state))))))))))))))
(defn typeinfer-adt-build-param-state-step-64-loop-bounded
  [params idx len counter param-env param-types bound-vars remaining]
  (do
    (root_push params)
    (root_push param-env)
    (root_push param-types)
    (root_push bound-vars)
    (let [step
            (typeinfer-adt-build-param-state-step-v3
              params
              idx
              len
              counter
              param-env
              param-types
              bound-vars)]
      (do
        (root_push step)
        (let [parsed
                (if (= (vector-get step 0) 1)
                  step
                  (if (<= remaining 1)
                    step
                    (let [next-state (vector-get step 2)]
                      (do
                        (root_push next-state)
                        (let [next
                                (typeinfer-adt-build-param-state-step-64-loop-bounded
                                  params
                                  (vector-get step 1)
                                  len
                                  counter
                                  (vector-get next-state 0)
                                  (vector-get next-state 1)
                                  (vector-get next-state 2)
                                  (- remaining 1))]
                          (do
                            (root_pop)
                            next))))))]
          (do
            (root_pop) (root_pop) (root_pop) (root_pop) (root_pop)
            parsed))))))

(defn typeinfer-adt-build-param-state-rooted-v3
  [params idx len counter param-env param-types bound-vars]
  (let [step
          (typeinfer-adt-build-param-state-step-64-loop-bounded
            params
            idx
            len
            counter
            param-env
            param-types
            bound-vars
            64)]
    (if (= (vector-get step 0) 1)
      (vector-get step 2)
      (do
        (root_push step)
        (let [next-state (vector-get step 2)]
          (do
            (root_push next-state)
            (let [parsed
                    (typeinfer-adt-build-param-state-rooted-v3
                      params
                      (vector-get step 1)
                      len
                      counter
                      (vector-get next-state 0)
                      (vector-get next-state 1)
                      (vector-get next-state 2))]
              (do
                (root_pop)
                (root_pop)
                parsed))))))))
(defn typeinfer-adt-build-param-state-loop [params idx len counter param-env param-types bound-vars]
  (do
    (root_push params)
    (root_push counter)
    (root_push param-env)
    (root_push param-types)
    (root_push bound-vars)
    (let [result
            (typeinfer-adt-build-param-state-rooted-v3
              params
              idx
              len
              counter
              param-env
              param-types
              bound-vars)]
      (do
        (root_pop) (root_pop) (root_pop) (root_pop) (root_pop)
        result))))

(defn typeinfer-adt-build-param-state [params counter]
  (typeinfer-adt-build-param-state-loop
    params
    0
    (vector-length params)
    counter
    (map-new)
    (vector-new (vector-length params))
    (vector-new (vector-length params))))

(defn typeinfer-adt-result-type [type-name-hash param-types]
  (if (= (vector-length param-types) 0)
    (mk-con type-name-hash)
    (mk-app type-name-hash param-types)))

;; raw field TypeExpr を左から収集し、後段で右から curried constructor type へ変換する。
;; 収集とfoldを分けることで、chunk境界で field の順序を反転させない。
(defn typeinfer-adt-constructor-field-step-v3
  [raw-fields idx len alias-env param-env fields]
  (if (>= idx len)
    (vector-push-triple-rooted (vector-new 3) 1 idx fields)
    (do
      (root_push raw-fields)
      (root_push alias-env)
      (root_push param-env)
      (root_push fields)
      (let [raw-field (vector-get raw-fields idx)]
        (do
          (root_push raw-field)
          (let [field-type
                  (typeinfer-resolve-type-expr-with-aliases-and-params
                    raw-field
                    alias-env
                    param-env)]
            (do
              (root_push field-type)
              (let [next-fields (push-object-vector-local fields field-type)]
                (do
                  (root_push next-fields)
                  (let [state
                          (vector-push-triple-rooted
                            (vector-new 3)
                            0
                            (+ idx 1)
                            next-fields)]
                    (do
                      (root_pop) (root_pop) (root_pop) (root_pop)
                      (root_pop) (root_pop) (root_pop)
                      state)))))))))))

(defn typeinfer-adt-constructor-type-step-64-loop-bounded
  [raw-fields idx len alias-env param-env fields remaining]
  (do
    (root_push raw-fields)
    (root_push alias-env)
    (root_push param-env)
    (root_push fields)
    (let [step
            (typeinfer-adt-constructor-field-step-v3
              raw-fields
              idx
              len
              alias-env
              param-env
              fields)]
      (do
        (root_push step)
        (let [parsed
                (if (= (vector-get step 0) 1)
                  step
                  (if (<= remaining 1)
                    step
                    (let [next-fields (vector-get step 2)]
                      (do
                        (root_push next-fields)
                        (let [next
                                (typeinfer-adt-constructor-type-step-64-loop-bounded
                                  raw-fields
                                  (vector-get step 1)
                                  len
                                  alias-env
                                  param-env
                                  next-fields
                                  (- remaining 1))]
                          (do
                            (root_pop)
                            next))))))]
          (do
            (root_pop) (root_pop) (root_pop) (root_pop) (root_pop)
            parsed))))))

(defn typeinfer-adt-constructor-fold-step-v3 [fields idx result-type]
  (if (<= idx 0)
    (vector-push-triple-rooted (vector-new 3) 1 idx result-type)
    (let [field-type (vector-get fields (- idx 1))]
      (do
        (root_push fields)
        (root_push result-type)
        (root_push field-type)
        (let [constructed (mk-fun field-type result-type)]
          (do
            (root_push constructed)
            (let [state
                    (vector-push-triple-rooted
                      (vector-new 3)
                      0
                      (- idx 1)
                      constructed)]
              (do
                (root_pop)
                (root_pop)
                (root_pop)
                (root_pop)
                state))))))))

(defn typeinfer-adt-constructor-fold-step-64-loop-bounded
  [fields idx result-type remaining]
  (do
    (root_push fields)
    (root_push result-type)
    (let [step (typeinfer-adt-constructor-fold-step-v3 fields idx result-type)]
      (do
        (root_push step)
        (let [parsed
                (if (= (vector-get step 0) 1)
                  step
                  (if (<= remaining 1)
                    step
                    (let [next-result (vector-get step 2)]
                      (do
                        (root_push next-result)
                        (let [next
                                (typeinfer-adt-constructor-fold-step-64-loop-bounded
                                  fields
                                  (vector-get step 1)
                                  next-result
                                  (- remaining 1))]
                          (do
                            (root_pop)
                            next))))))]
          (do
            (root_pop)
            (root_pop)
            (root_pop)
            parsed))))))

(defn typeinfer-adt-constructor-fold-rooted-v3 [fields idx result-type]
  (let [step
          (typeinfer-adt-constructor-fold-step-64-loop-bounded
            fields
            idx
            result-type
            64)]
    (if (= (vector-get step 0) 1)
      (vector-get step 2)
      (do
        (root_push step)
        (let [next-result (vector-get step 2)]
          (do
            (root_push next-result)
            (let [parsed
                    (typeinfer-adt-constructor-fold-rooted-v3
                      fields
                      (vector-get step 1)
                      next-result)]
              (do
                (root_pop)
                (root_pop)
                parsed))))))))

(defn typeinfer-adt-constructor-type-rooted-v3
  [raw-fields idx len alias-env param-env result-type fields]
  (let [step
          (typeinfer-adt-constructor-type-step-64-loop-bounded
            raw-fields
            idx
            len
            alias-env
            param-env
            fields
            64)]
    (if (= (vector-get step 0) 1)
      (do
        (root_push step)
        (let [collected (vector-get step 2)]
          (do
            (root_push collected)
            (let [result
                    (typeinfer-adt-constructor-fold-rooted-v3
                      collected
                      (vector-length collected)
                      result-type)]
              (do
                (root_pop)
                (root_pop)
                result)))))
      (do
        (root_push step)
        (let [next-fields (vector-get step 2)]
          (do
            (root_push next-fields)
            (let [parsed
                    (typeinfer-adt-constructor-type-rooted-v3
                      raw-fields
                      (vector-get step 1)
                      len
                      alias-env
                      param-env
                      result-type
                      next-fields)]
              (do
                (root_pop)
                (root_pop)
                parsed))))))))

(defn typeinfer-adt-constructor-type-loop [raw-fields idx len alias-env param-env result-type]
  (do
    (root_push raw-fields)
    (root_push alias-env)
    (root_push param-env)
    (root_push result-type)
    (let [fields (vector-new 0)]
      (do
        (root_push fields)
        (let [result
                (typeinfer-adt-constructor-type-rooted-v3
                  raw-fields
                  idx
                  len
                  alias-env
                  param-env
                  result-type
                  fields)]
          (do
            (root_pop) (root_pop) (root_pop) (root_pop) (root_pop)
            result))))))

(defn typeinfer-adt-constructor-type [raw-fields alias-env param-env result-type]
  (typeinfer-adt-constructor-type-loop
    raw-fields
    0
    (vector-length raw-fields)
    alias-env
    param-env
    result-type))

;; GADT variant は AST 末尾の raw return TypeExpr を constructor の戻り型に使う。
;; 旧形式の 2 要素 variant は宣言全体の result-type をそのまま使う。
(defn typeinfer-adt-variant-result-type [variant result-type alias-env param-env]
  (if (> (vector-length variant) 2)
    (typeinfer-resolve-type-expr-with-aliases-and-params
      (vector-get variant 2)
      alias-env
      param-env)
    result-type))

(defn typeinfer-adt-variant-scheme [variant constructor-type bound-vars]
  (if (> (vector-length variant) 2)
    (poly-gadt constructor-type bound-vars)
    (poly constructor-type bound-vars)))

;; 同一 ADT の variant は同じ parameter variables / bound-vars を共有する。
;; variantごとの env 更新を64要素単位で切り、chunk間は env を渡す。
(defn typeinfer-register-adt-variants-step-v3
  [variants idx len env alias-env param-env result-type bound-vars]
  (if (>= idx len)
    (vector-push-triple-rooted (vector-new 3) 1 idx env)
    (do
      (root_push variants)
      (root_push env)
      (root_push alias-env)
      (root_push param-env)
      (root_push result-type)
      (root_push bound-vars)
      (let [variant (vector-get variants idx)]
        (do
          (root_push variant)
          (let [constructor-name-hash (vector-get variant 0)
            raw-fields (vector-get variant 1)
            variant-result-type
              (typeinfer-adt-variant-result-type
                variant
                result-type
                alias-env
                param-env)]
            (do
              (root_push raw-fields)
              (root_push variant-result-type)
              (let [constructor-type
                      (typeinfer-adt-constructor-type
                        raw-fields
                        alias-env
                        param-env
                        variant-result-type)]
                (do
                  (root_push constructor-type)
                  (let [scheme
                          (typeinfer-adt-variant-scheme
                            variant
                            constructor-type
                            bound-vars)]
                    (do
                      (root_push scheme)
                      (let [next-env
                              (type-env-insert env constructor-name-hash scheme)]
                        (do
                          (root_push next-env)
                          (let [state
                                  (vector-push-triple-rooted
                                    (vector-new 3)
                                    0
                                    (+ idx 1)
                                    next-env)]
                            (do
                              (root_pop) (root_pop) (root_pop) (root_pop)
                              (root_pop) (root_pop) (root_pop) (root_pop)
                              (root_pop) (root_pop) (root_pop) (root_pop)
                              state)))))))))))))))

(defn typeinfer-register-adt-variants-step-64-loop-bounded
  [variants idx len env alias-env param-env result-type bound-vars remaining]
  (do
    (root_push variants)
    (root_push env)
    (root_push alias-env)
    (root_push param-env)
    (root_push result-type)
    (root_push bound-vars)
    (let [step
            (typeinfer-register-adt-variants-step-v3
              variants
              idx
              len
              env
              alias-env
              param-env
              result-type
              bound-vars)]
      (do
        (root_push step)
        (let [parsed
                (if (= (vector-get step 0) 1)
                  step
                  (if (<= remaining 1)
                    step
                    (let [next-env (vector-get step 2)]
                      (do
                        (root_push next-env)
                        (let [next
                                (typeinfer-register-adt-variants-step-64-loop-bounded
                                  variants
                                  (vector-get step 1)
                                  len
                                  next-env
                                  alias-env
                                  param-env
                                  result-type
                                  bound-vars
                                  (- remaining 1))]
                          (do
                            (root_pop)
                            next))))))]
          (do
            (root_pop)
            (root_pop)
            (root_pop)
            (root_pop)
            (root_pop)
            (root_pop)
            (root_pop)
            parsed))))))

(defn typeinfer-register-adt-variants-rooted-v3
  [variants idx len env alias-env param-env result-type bound-vars]
  (let [step
          (typeinfer-register-adt-variants-step-64-loop-bounded
            variants
            idx
            len
            env
            alias-env
            param-env
            result-type
            bound-vars
            64)]
    (if (= (vector-get step 0) 1)
      (vector-get step 2)
      (do
        (root_push step)
        (let [next-env (vector-get step 2)]
          (do
            (root_push next-env)
            (let [parsed
                    (typeinfer-register-adt-variants-rooted-v3
                      variants
                      (vector-get step 1)
                      len
                      next-env
                      alias-env
                      param-env
                      result-type
                      bound-vars)]
              (do
                (root_pop)
                (root_pop)
                parsed))))))))

(defn typeinfer-register-adt-variants-loop [variants idx len env alias-env param-env result-type bound-vars]
  (do
    (root_push variants)
    (root_push env)
    (root_push alias-env)
    (root_push param-env)
    (root_push result-type)
    (root_push bound-vars)
    (let [result
            (typeinfer-register-adt-variants-rooted-v3
              variants
              idx
              len
              env
              alias-env
              param-env
              result-type
              bound-vars)]
      (do
        (root_pop) (root_pop) (root_pop) (root_pop) (root_pop) (root_pop)
        result))))

(defn typeinfer-register-adt-decl [decl env counter alias-env]
  (let [variants (typeinfer-adt-decl-variants decl)]
    (if (= variants 0)
      env
      (do
        (root_push variants)
        (let [params (typeinfer-adt-decl-params decl)]
          (do
            (root_push params)
            (let [param-state (typeinfer-adt-build-param-state params counter)]
              (do
                (root_push param-state)
                (let [param-env (vector-get param-state 0)
                  param-types (vector-get param-state 1)
                  bound-vars (vector-get param-state 2)]
                  (do
                    (root_push param-env)
                    (root_push param-types)
                    (root_push bound-vars)
                    (let [result-type
                            (typeinfer-adt-result-type (vector-get decl 1) param-types)]
                      (do
                        (root_push result-type)
                        (let [result
                                (typeinfer-register-adt-variants-loop
                                  variants
                                  0
                                  (vector-length variants)
                                  env
                                  alias-env
                                  param-env
                                  result-type
                                  bound-vars)]
                          (do
                            (root_pop)
                            (root_pop)
                            (root_pop)
                            (root_pop)
                            (root_pop)
                            (root_pop)
                            (root_pop)
                            result))))))))))))))

;; type declaration を source order で走査し、constructor を通常の値環境へ登録する。
(defn typeinfer-register-adt-defs-step-v3
  [program idx len env counter alias-env]
  (if (>= idx len)
    (vector-push-triple-rooted (vector-new 3) 1 idx env)
    (do
      (root_push program)
      (root_push env)
      (root_push counter)
      (root_push alias-env)
      (let [decl (vector-get program idx)]
        (do
          (root_push decl)
          (if (= (vector-get decl 0) (ast-type-decl))
            (let [next-env (typeinfer-register-adt-decl decl env counter alias-env)]
              (do
                (root_push next-env)
                (let [state
                        (vector-push-triple-rooted
                          (vector-new 3)
                          0
                          (+ idx 1)
                          next-env)]
                  (do
                    (root_pop)
                    (root_pop)
                    (root_pop)
                    (root_pop)
                    (root_pop)
                    (root_pop)
                    state))))
            (let [state
                    (vector-push-triple-rooted
                      (vector-new 3)
                      0
                      (+ idx 1)
                      env)]
              (do
                (root_pop)
                (root_pop)
                (root_pop)
                (root_pop)
                (root_pop)
                    state))))))))

(defn typeinfer-register-adt-defs-step-64-loop-bounded
  [program idx len env counter alias-env remaining]
  (do
    (root_push program)
    (root_push env)
    (root_push counter)
    (root_push alias-env)
    (let [step
            (typeinfer-register-adt-defs-step-v3
              program
              idx
              len
              env
              counter
              alias-env)]
      (do
        (root_push step)
        (let [parsed
                (if (= (vector-get step 0) 1)
                  step
                  (if (<= remaining 1)
                    step
                    (let [next-env (vector-get step 2)]
                      (do
                        (root_push next-env)
                        (let [next
                                (typeinfer-register-adt-defs-step-64-loop-bounded
                                  program
                                  (vector-get step 1)
                                  len
                                  next-env
                                  counter
                                  alias-env
                                  (- remaining 1))]
                          (do
                            (root_pop)
                            next))))))]
          (do
            (root_pop)
            (root_pop)
            (root_pop)
            (root_pop)
            (root_pop)
            parsed))))))

(defn typeinfer-register-adt-defs-rooted-v3
  [program idx len env counter alias-env]
  (let [step
          (typeinfer-register-adt-defs-step-64-loop-bounded
            program
            idx
            len
            env
            counter
            alias-env
            64)]
    (if (= (vector-get step 0) 1)
      (vector-get step 2)
      (do
        (root_push step)
        (let [next-env (vector-get step 2)]
          (do
            (root_push next-env)
            (let [parsed
                    (typeinfer-register-adt-defs-rooted-v3
                      program
                      (vector-get step 1)
                      len
                      next-env
                      counter
                      alias-env)]
              (do
                (root_pop)
                (root_pop)
                parsed))))))))

(defn typeinfer-register-adt-defs-loop [program idx len env counter alias-env]
  (do
    (root_push program)
    (root_push env)
    (root_push counter)
    (root_push alias-env)
    (let [result
            (typeinfer-register-adt-defs-rooted-v3
              program
              idx
              len
              env
              counter
              alias-env)]
      (do
        (root_pop)
        (root_pop)
        (root_pop)
        (root_pop)
        result))))

(defn typeinfer-register-adt-defs [program env counter]
  (let [alias-env (var-counter-alias-env counter)]
    (do
      (root_push program)
      (root_push env)
      (root_push alias-env)
      (let [result
              (typeinfer-register-adt-defs-loop
                program
                0
                (vector-length program)
                env
                counter
                alias-env)]
        (do
          (root_pop)
          (root_pop)
          (root_pop)
          result)))))
