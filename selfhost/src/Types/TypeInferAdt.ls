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
(defn typeinfer-adt-build-param-state-loop [params idx len counter param-env param-types bound-vars]
  (if (>= idx len)
    (typeinfer-adt-make-param-state param-env param-types bound-vars)
    (let [param-hash (vector-get params idx)
      param-type (fresh-type-var counter)
      next-param-env (map-insert-object-safe param-env param-hash param-type)
      next-param-types (push-object-vector-local param-types param-type)
      next-bound-vars (push-int-vector-local bound-vars (ty-name param-type))]
      (typeinfer-adt-build-param-state-loop
        params
        (+ idx 1)
        len
        counter
        next-param-env
        next-param-types
        next-bound-vars))))

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

;; raw field TypeExpr を左から curried constructor type へ変換する。
(defn typeinfer-adt-constructor-type-loop [raw-fields idx len alias-env param-env result-type]
  (if (>= idx len)
    result-type
    (do
      (root_push raw-fields)
      (root_push alias-env)
      (root_push param-env)
      (root_push result-type)
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
              (let [rest-type
                      (typeinfer-adt-constructor-type-loop
                        raw-fields
                        (+ idx 1)
                        len
                        alias-env
                        param-env
                        result-type)]
                (do
                  (root_push rest-type)
                  (let [constructed (mk-fun field-type rest-type)]
                    (do
                      (root_pop)
                      (root_pop)
                      (root_pop)
                      (root_pop)
                      (root_pop)
                      (root_pop)
                      (root_pop)
                      constructed)))))))))))

(defn typeinfer-adt-constructor-type [raw-fields alias-env param-env result-type]
  (do
    (root_push raw-fields)
    (root_push alias-env)
    (root_push param-env)
    (root_push result-type)
    (let [result
            (typeinfer-adt-constructor-type-loop
              raw-fields
              0
              (vector-length raw-fields)
              alias-env
              param-env
              result-type)]
      (do
        (root_pop)
        (root_pop)
        (root_pop)
        (root_pop)
        result))))

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
(defn typeinfer-register-adt-variants-loop [variants idx len env alias-env param-env result-type bound-vars]
  (if (>= idx len)
    env
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
              (root_push variant-result-type)
              (root_push raw-fields)
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
                          (let [parsed
                                  (typeinfer-register-adt-variants-loop
                                    variants
                                    (+ idx 1)
                                    len
                                    next-env
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
                              (root_pop)
                              (root_pop)
                              (root_pop)
                              (root_pop)
                              (root_pop)
                              parsed)))))))))))))))

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
(defn typeinfer-register-adt-defs-loop [program idx len env counter alias-env]
  (if (>= idx len)
    env
    (let [decl (vector-get program idx)]
      (if (= (vector-get decl 0) (ast-type-decl))
        (let [next-env (typeinfer-register-adt-decl decl env counter alias-env)]
          (typeinfer-register-adt-defs-loop
            program
            (+ idx 1)
            len
            next-env
            counter
            alias-env))
        (typeinfer-register-adt-defs-loop
          program
          (+ idx 1)
          len
          env
          counter
          alias-env)))))

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
