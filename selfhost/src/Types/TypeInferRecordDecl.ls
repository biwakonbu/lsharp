(module Types.TypeInferRecordDecl)
(import Syntax.AST)
(import Types.Type)
(import Types.TypeScheme)
(import Types.TypeInferCore)

;; TypeInferRecordDecl.ls - record 宣言の schema / constructor 登録
;;
;; record schema は TypeScheme として record-env に保持する。
;; parametric record では field 型 template と bound variable を同じ scheme に入れ、
;; literal ごとに instantiate して field 間の型変数共有を維持する。

;; nonparametric: [22, name, fields]
;; parametric:    [22, name, params, fields]
(defn typeinfer-record-decl-params [decl]
  (if (>= (vector-length decl) 4)
    (vector-get decl 2)
    (vector-new 0)))

(defn typeinfer-record-decl-field-exprs [decl]
  (if (>= (vector-length decl) 4)
    (vector-get decl 3)
    (if (> (vector-length decl) 2)
      (vector-get decl 2)
      0)))

(defn typeinfer-record-make-param-state [param-env bound-vars]
  (vector-push-pair-rooted (vector-new 2) param-env bound-vars))

;; declaration parameter 名を fresh type variable と scheme bound variable に対応付ける。
(defn typeinfer-record-build-param-state-loop [params idx len counter param-env bound-vars]
  (if (>= idx len)
    (typeinfer-record-make-param-state param-env bound-vars)
    (let [param-hash (vector-get params idx)
      param-type (fresh-type-var counter)
      next-param-env (map-insert-object-safe param-env param-hash param-type)
      next-bound-vars (push-int-vector-local bound-vars (ty-name param-type))]
      (typeinfer-record-build-param-state-loop
        params
        (+ idx 1)
        len
        counter
        next-param-env
        next-bound-vars))))

(defn typeinfer-record-build-param-state [params counter]
  (typeinfer-record-build-param-state-loop
    params
    0
    (vector-length params)
    counter
    (map-new)
    (vector-new (vector-length params))))

(defn typeinfer-record-fields-append [out field-hash field-ty]
  (let [with-name (push-int-vector-local out field-hash)]
    (push-object-vector-local with-name field-ty)))

;; raw field TypeExpr を alias と declaration parameter を解決した field 型列へ変換する。
(defn typeinfer-record-resolve-field-types-loop [raw-fields idx len alias-env param-env out]
  (if (>= idx len)
    out
    (do
      (root_push raw-fields)
      (root_push alias-env)
      (root_push param-env)
      (root_push out)
      (let [field-hash (vector-get raw-fields idx)
        raw-type-expr (vector-get raw-fields (+ idx 1))]
        (do
          (root_push raw-type-expr)
          (let [field-ty
                  (typeinfer-resolve-type-expr-with-aliases-and-params
                    raw-type-expr
                    alias-env
                    param-env)]
            (do
              (root_push field-ty)
              (let [next-out (typeinfer-record-fields-append out field-hash field-ty)]
                (do
                  (root_push next-out)
                  (let [parsed
                          (typeinfer-record-resolve-field-types-loop
                            raw-fields
                            (+ idx 2)
                            len
                            alias-env
                            param-env
                            next-out)]
                    (do
                      (root_pop)
                      (root_pop)
                      (root_pop)
                      (root_pop)
                      (root_pop)
                      (root_pop)
                      (root_pop)
                      parsed)))))))))))

(defn typeinfer-record-resolve-field-types [raw-fields alias-env param-env]
  (do
    (root_push raw-fields)
    (root_push alias-env)
    (root_push param-env)
    (let [out (vector-new 0)
      out-slot (root_push out)]
      (let [parsed
              (typeinfer-record-resolve-field-types-loop
                raw-fields
                0
                (vector-length raw-fields)
                alias-env
                param-env
                out)]
        (do
          (root_set out-slot parsed)
          (root_pop)
          (root_pop)
          (root_pop)
          (root_pop)
          parsed)))))

;; [field-hash, field-type, ...] を structural record type へ変換する。
(defn typeinfer-record-build-type-loop [record-ty field-types idx len]
  (if (>= idx len)
    record-ty
    (do
      (root_push record-ty)
      (root_push field-types)
      (let [field-hash (vector-get field-types idx)
        field-ty (vector-get field-types (+ idx 1))]
        (do
          (root_push field-ty)
          (let [next-record-ty (type-record-add-field record-ty field-hash field-ty)]
            (do
              (root_push next-record-ty)
              (let [result
                      (typeinfer-record-build-type-loop
                        next-record-ty
                        field-types
                        (+ idx 2)
                        len)]
                (do
                  (root_pop)
                  (root_pop)
                  (root_pop)
                  (root_pop)
                  result)))))))))

(defn typeinfer-record-build-type [record-name-hash field-types]
  (do
    (root_push field-types)
    (let [record-ty (make-type-record record-name-hash)]
      (do
        (root_push record-ty)
        (let [result
                (typeinfer-record-build-type-loop
                  record-ty
                  field-types
                  0
                  (vector-length field-types))]
          (do
            (root_pop)
            (root_pop)
            result))))))

;; schema は record type template と bound variable を併せた TypeScheme で表す。
(defn typeinfer-record-decl-schema [decl alias-env counter]
  (let [raw-fields (typeinfer-record-decl-field-exprs decl)]
    (if (= raw-fields 0)
      0
      (do
        (root_push raw-fields)
        (let [params (typeinfer-record-decl-params decl)]
          (do
            (root_push params)
            (let [param-state (typeinfer-record-build-param-state params counter)]
              (do
                (root_push param-state)
                (let [param-env (vector-get param-state 0)
                  bound-vars (vector-get param-state 1)]
                  (do
                    (root_push param-env)
                    (root_push bound-vars)
                    (let [field-types
                            (typeinfer-record-resolve-field-types
                              raw-fields
                              alias-env
                              param-env)]
                      (do
                        (root_push field-types)
                        (let [record-ty
                                (typeinfer-record-build-type
                                  (vector-get decl 1)
                                  field-types)]
                          (do
                            (root_push record-ty)
                            (let [schema (poly record-ty bound-vars)]
                              (do
                                (root_pop)
                                (root_pop)
                                (root_pop)
                                (root_pop)
                                (root_pop)
                                (root_pop)
                                (root_pop)
                                schema))))))))))))))))

;; source order で作成済み schema を registry に登録して次の宣言へ進む。
(defn typeinfer-predeclare-record-env-with-schema [program idx len alias-env record-env counter decl schema]
  (do
    (root_push schema)
    (root_push record-env)
    (let [next-record-env
            (map-insert-object-safe record-env (vector-get decl 1) schema)]
      (do
        (root_push next-record-env)
        (let [parsed
                (typeinfer-predeclare-record-env-loop
                  program
                  (+ idx 1)
                  len
                  alias-env
                  next-record-env
                  counter)]
          (do
            (root_pop)
            (root_pop)
            (root_pop)
            parsed))))))

;; source order で record schema を registry に登録する。
(defn typeinfer-predeclare-record-env-loop [program idx len alias-env record-env counter]
  (if (>= idx len)
    record-env
    (let [decl (vector-get program idx)
      tag (vector-get decl 0)]
      (if (= tag (ast-recorddef))
        (let [schema (typeinfer-record-decl-schema decl alias-env counter)]
          (if (= schema 0)
            (typeinfer-predeclare-record-env-loop
              program
              (+ idx 1)
              len
              alias-env
              record-env
              counter)
            (typeinfer-predeclare-record-env-with-schema
              program
              idx
              len
              alias-env
              record-env
              counter
              decl
              schema)))
        (typeinfer-predeclare-record-env-loop
          program
          (+ idx 1)
          len
          alias-env
          record-env
          counter)))))

(defn typeinfer-predeclare-record-env [program alias-env counter]
  (do
    (root_push program)
    (root_push alias-env)
    (let [record-env (map-new)
      record-env-slot (root_push record-env)]
      (let [parsed
              (typeinfer-predeclare-record-env-loop
                program
                0
                (vector-length program)
                alias-env
                record-env
                counter)]
        (do
          (root_set record-env-slot parsed)
          (root_pop)
          (root_pop)
          (root_pop)
          parsed)))))

;; record type template の field 型列から curried constructor type を組み立てる。
(defn typeinfer-record-constructor-type-loop [record-ty idx len result]
  (if (>= idx len)
    result
    (do
      (root_push record-ty)
      (root_push result)
      (let [field-ty (vector-get record-ty (+ idx 1))]
        (do
          (root_push field-ty)
          (let [rest
                  (typeinfer-record-constructor-type-loop
                    record-ty
                    (+ idx 2)
                    len
                    result)]
            (do
              (root_push rest)
              (let [constructed (mk-fun field-ty rest)]
                (do
                  (root_pop)
                  (root_pop)
                  (root_pop)
                  (root_pop)
                  constructed)))))))))

(defn typeinfer-record-constructor-type [record-ty]
  (do
    (root_push record-ty)
    (let [result
            (typeinfer-record-constructor-type-loop
              record-ty
              2
              (vector-length record-ty)
              record-ty)]
      (do
        (root_pop)
        result))))

;; record constructor を schema と同じ bound variable で通常の値環境へ登録する。
(defn typeinfer-register-record-def [decl env record-env]
  (let [record-name-hash (vector-get decl 1)
    schema (map-get-safe record-env (vector-get decl 1))]
    (if (= schema 0)
      env
      (do
        (root_push schema)
        (root_push env)
        (let [record-ty (scheme-type schema)
          bound-vars (scheme-vars schema)]
          (do
            (root_push record-ty)
            (root_push bound-vars)
            (let [constructor-ty (typeinfer-record-constructor-type record-ty)]
              (do
                (root_push constructor-ty)
                (let [constructor-scheme (poly constructor-ty bound-vars)]
                  (do
                    (root_push constructor-scheme)
                    (let [next-env
                            (type-env-insert
                              env
                              record-name-hash
                              constructor-scheme)]
                      (do
                        (root_pop)
                        (root_pop)
                        (root_pop)
                        (root_pop)
                        (root_pop)
                        (root_pop)
                        next-env))))))))))))

(defn typeinfer-register-record-defs-loop [program idx len env record-env]
  (if (>= idx len)
    env
    (let [decl (vector-get program idx)
      tag (vector-get decl 0)]
      (if (= tag (ast-recorddef))
        (let [next-env (typeinfer-register-record-def decl env record-env)]
          (do
            (root_push next-env)
            (let [parsed
                    (typeinfer-register-record-defs-loop
                      program
                      (+ idx 1)
                      len
                      next-env
                      record-env)]
              (do
                (root_pop)
                parsed))))
        (typeinfer-register-record-defs-loop program (+ idx 1) len env record-env)))))

(defn typeinfer-register-record-defs [program env counter]
  (let [record-env (var-counter-record-env counter)]
    (do
      (root_push program)
      (root_push env)
      (root_push record-env)
      (let [result
              (typeinfer-register-record-defs-loop
                program
                0
                (vector-length program)
                env
                record-env)]
        (do
          (root_pop)
          (root_pop)
          (root_pop)
          result)))))
