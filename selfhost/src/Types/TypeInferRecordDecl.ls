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
;; fields: [field-hash, accessor-hash, raw-TypeExpr, ...]
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
;; private wrapper 内の record も宣言元 module の schema 登録対象にする。
;; import traversal は wrapper を開かないため、公開 export には追加されない。
(defn typeinfer-record-decl-unprivate [decl]
  (if (= (vector-get decl 0) (ast-private))
    (typeinfer-record-decl-unprivate (vector-get decl 1))
    decl))
(defn typeinfer-record-remove-accessors-loop [raw-fields idx len env]
  (if (>= idx len)
    env
    (typeinfer-record-remove-accessors-loop
      raw-fields
      (+ idx 3)
      len
      (type-env-remove env (vector-get raw-fields (+ idx 1))))))
(defn typeinfer-remove-record-def [decl env]
  (let [raw-fields (typeinfer-record-decl-field-exprs decl)
    constructor-env (type-env-remove env (vector-get decl 1))]
    (if (= raw-fields 0)
      constructor-env
      (typeinfer-record-remove-accessors-loop
        raw-fields
        0
        (vector-length raw-fields)
        constructor-env))))
(defn typeinfer-remove-record-defs-before-module-loop [program env idx limit]
  (if (>= idx limit)
    env
    (let [decl (typeinfer-record-decl-unprivate (vector-get program idx))
      tag (vector-get decl 0)]
      (if (= tag (ast-recorddef))
        (typeinfer-remove-record-defs-before-module-loop
          program
          (typeinfer-remove-record-def decl env)
          (+ idx 1)
          limit)
        (typeinfer-remove-record-defs-before-module-loop
          program
          env
          (+ idx 1)
          limit)))))
(defn typeinfer-remove-record-defs-before-module [program env limit]
  (typeinfer-remove-record-defs-before-module-loop program env 0 limit))
(defn typeinfer-record-only-contains-loop [only-hashes idx len name-hash]
  (if (>= idx len)
    0
    (if (= (vector-get only-hashes idx) name-hash)
      1
      (typeinfer-record-only-contains-loop
        only-hashes
        (+ idx 1)
        len
        name-hash))))
(defn typeinfer-record-export-allowed? [only-hashes name-hash]
  (if (= only-hashes 0)
    1
    (let [only-count (vector-length only-hashes)]
      (if (= only-count 0)
        1
        (typeinfer-record-only-contains-loop only-hashes 0 only-count name-hash)))))
(defn typeinfer-record-remove-unallowed-accessors-loop
  [raw-fields idx len only-hashes env]
  (if (>= idx len)
    env
    (let [accessor-hash (vector-get raw-fields (+ idx 1))
      next-env
        (if (= (typeinfer-record-export-allowed? only-hashes accessor-hash) 1)
          env
          (type-env-remove env accessor-hash))]
      (typeinfer-record-remove-unallowed-accessors-loop
        raw-fields
        (+ idx 3)
        len
        only-hashes
        next-env))))
(defn typeinfer-clean-record-import-export [decl only-hashes open-flag env]
  (if (= open-flag 1)
    (let [raw-fields (typeinfer-record-decl-field-exprs decl)
      constructor-env
        (if (= (typeinfer-record-export-allowed? only-hashes (vector-get decl 1)) 1)
          env
          (type-env-remove env (vector-get decl 1)))]
      (if (= raw-fields 0)
        constructor-env
        (typeinfer-record-remove-unallowed-accessors-loop
          raw-fields
          0
          (vector-length raw-fields)
          only-hashes
          constructor-env)))
    (typeinfer-remove-record-def decl env)))
(defn typeinfer-record-make-param-state [param-env bound-vars]
  (vector-push-pair-rooted (vector-new 2) param-env bound-vars))
;; declaration parameter 名を fresh type variable と scheme bound variable に対応付ける。
(defn typeinfer-record-build-param-state-loop [params idx len counter param-env bound-vars]
  (typeinfer-record-build-param-state-rooted-v3
    params idx len counter param-env bound-vars))
(defn typeinfer-record-build-param-state-step-state [done next-idx result]
  (vector-push-triple-rooted (vector-new 3) done next-idx result))
(defn typeinfer-record-build-param-state-step-v3
  [params idx len counter param-env bound-vars]
  (if (>= idx len)
    (typeinfer-record-build-param-state-step-state
      1 idx (typeinfer-record-make-param-state param-env bound-vars))
    (do
      (root_push params)
      (root_push counter)
      (root_push param-env)
      (root_push bound-vars)
      (let [param-hash (vector-get params idx)
        param-type (fresh-type-var counter)
        next-param-env (map-insert-object-safe param-env param-hash param-type)
        next-bound-vars (push-int-vector-local bound-vars (ty-name param-type))]
        (do
          (root_push param-type)
          (root_push next-param-env)
          (root_push next-bound-vars)
            (let [next-state
                  (typeinfer-record-make-param-state
                    next-param-env next-bound-vars)
            state
              (typeinfer-record-build-param-state-step-state
                0 (+ idx 1) next-state)]
            (do
              (root_pop) (root_pop) (root_pop) (root_pop)
              (root_pop) (root_pop) (root_pop)
              state)))))))
(defn typeinfer-record-build-param-state-step-64-loop-bounded
  [params idx len counter param-env bound-vars remaining]
  (do
    (root_push params)
    (root_push counter)
    (root_push param-env)
    (root_push bound-vars)
    (let [step
      (typeinfer-record-build-param-state-step-v3
        params idx len counter param-env bound-vars)
      done (vector-get step 0)
      next-idx (vector-get step 1)
      next-state (vector-get step 2)]
      (do
        (root_push step)
        (root_push next-state)
        (let [next-param-env (vector-get next-state 0)
          next-bound-vars (vector-get next-state 1)
          parsed
            (if (= done 1)
              step
              (if (<= remaining 1)
                step
                (typeinfer-record-build-param-state-step-64-loop-bounded
                  params
                  next-idx
                  len
                  counter
                  next-param-env
                  next-bound-vars
                  (- remaining 1))))]
          (do
            (root_pop) (root_pop) (root_pop)
            (root_pop) (root_pop) (root_pop)
            parsed))))))
(defn typeinfer-record-build-param-state-step-64
  [params idx len counter param-env bound-vars]
  (typeinfer-record-build-param-state-step-64-loop-bounded
    params idx len counter param-env bound-vars 64))
(defn typeinfer-record-build-param-state-rooted-v3
  [params idx len counter param-env bound-vars]
  (let [step
    (typeinfer-record-build-param-state-step-64
      params idx len counter param-env bound-vars)]
    (if (= (vector-get step 0) 1)
      (vector-get step 2)
      (do
        (root_push step)
        (let [next-state (vector-get step 2)
          next-param-env (vector-get next-state 0)
          next-bound-vars (vector-get next-state 1)]
          (do
            (root_push next-state)
            (let [resolved
              (typeinfer-record-build-param-state-rooted-v3
                params
                (vector-get step 1)
                len
                counter
                next-param-env
                next-bound-vars)]
              (do
                (root_pop) (root_pop)
                resolved))))))))
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
  (typeinfer-record-resolve-field-types-rooted-v3
    raw-fields idx len alias-env param-env out))
(defn typeinfer-record-resolve-field-types-step-state [done next-idx result]
  (vector-push-triple-rooted (vector-new 3) done next-idx result))
(defn typeinfer-record-resolve-field-types-step-v3
  [raw-fields idx len alias-env param-env out]
  (if (>= idx len)
    (typeinfer-record-resolve-field-types-step-state 1 idx out)
    (do
      (root_push raw-fields)
      (root_push alias-env)
      (root_push param-env)
      (root_push out)
      (let [field-hash (vector-get raw-fields idx)
        raw-type-expr (vector-get raw-fields (+ idx 2))]
        (do
          (root_push raw-type-expr)
          (let [field-ty
                  (typeinfer-resolve-type-expr-with-aliases-and-params
                    raw-type-expr alias-env param-env)]
            (do
              (root_push field-ty)
              (let [next-out
                      (typeinfer-record-fields-append out field-hash field-ty)]
                (do
                  (root_push next-out)
                  (let [state
                    (typeinfer-record-resolve-field-types-step-state
                      0 (+ idx 3) next-out)]
                    (do
                      (root_pop) (root_pop) (root_pop) (root_pop)
                      (root_pop) (root_pop) (root_pop)
                      state)))))))))))
(defn typeinfer-record-resolve-field-types-step-64-loop-bounded
  [raw-fields idx len alias-env param-env out remaining]
  (do
    (root_push raw-fields)
    (root_push alias-env)
    (root_push param-env)
    (root_push out)
    (let [step
      (typeinfer-record-resolve-field-types-step-v3
        raw-fields idx len alias-env param-env out)
      done (vector-get step 0)
      next-idx (vector-get step 1)
      next-out (vector-get step 2)]
      (do
        (root_push step)
        (root_push next-out)
        (let [parsed
          (if (= done 1)
            step
            (if (<= remaining 1)
              step
              (typeinfer-record-resolve-field-types-step-64-loop-bounded
                raw-fields
                next-idx
                len
                alias-env
                param-env
                next-out
                (- remaining 1))))]
          (do
            (root_pop) (root_pop) (root_pop)
            (root_pop) (root_pop) (root_pop)
            parsed))))))
(defn typeinfer-record-resolve-field-types-step-64
  [raw-fields idx len alias-env param-env out]
  (typeinfer-record-resolve-field-types-step-64-loop-bounded
    raw-fields idx len alias-env param-env out 64))
(defn typeinfer-record-resolve-field-types-rooted-v3
  [raw-fields idx len alias-env param-env out]
  (let [step
    (typeinfer-record-resolve-field-types-step-64
      raw-fields idx len alias-env param-env out)]
    (if (= (vector-get step 0) 1)
      (vector-get step 2)
      (do
        (root_push step)
        (let [next-idx (vector-get step 1)
          next-out (vector-get step 2)]
          (do
            (root_push next-out)
            (let [resolved
              (typeinfer-record-resolve-field-types-rooted-v3
                raw-fields next-idx len alias-env param-env next-out)]
              (do
                (root_pop) (root_pop)
                resolved))))))))
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
  (typeinfer-record-build-type-rooted-v3 record-ty field-types idx len))
(defn typeinfer-record-build-type-step-state [done next-idx result]
  (vector-push-triple-rooted (vector-new 3) done next-idx result))
(defn typeinfer-record-build-type-step-v3
  [record-ty field-types idx len]
  (if (>= idx len)
    (typeinfer-record-build-type-step-state 1 idx record-ty)
    (do
      (root_push record-ty)
      (root_push field-types)
      (let [field-hash (vector-get field-types idx)
        field-ty (vector-get field-types (+ idx 1))]
        (do
          (root_push field-ty)
          (let [next-record-ty
                  (type-record-add-field record-ty field-hash field-ty)]
            (do
              (root_push next-record-ty)
              (let [state
                (typeinfer-record-build-type-step-state
                  0 (+ idx 2) next-record-ty)]
                (do
                  (root_pop) (root_pop) (root_pop) (root_pop)
                  state)))))))))
(defn typeinfer-record-build-type-step-64-loop-bounded
  [record-ty field-types idx len remaining]
  (do
    (root_push record-ty)
    (root_push field-types)
    (let [step
      (typeinfer-record-build-type-step-v3
        record-ty field-types idx len)
      done (vector-get step 0)
      next-idx (vector-get step 1)
      next-record-ty (vector-get step 2)]
      (do
        (root_push step)
        (root_push next-record-ty)
        (let [parsed
          (if (= done 1)
            step
            (if (<= remaining 1)
              step
              (typeinfer-record-build-type-step-64-loop-bounded
                next-record-ty
                field-types
                next-idx
                len
                (- remaining 1))))]
          (do
            (root_pop) (root_pop) (root_pop) (root_pop)
            parsed))))))
(defn typeinfer-record-build-type-step-64
  [record-ty field-types idx len]
  (typeinfer-record-build-type-step-64-loop-bounded
    record-ty field-types idx len 64))
(defn typeinfer-record-build-type-rooted-v3
  [record-ty field-types idx len]
  (let [step
    (typeinfer-record-build-type-step-64
      record-ty field-types idx len)]
    (if (= (vector-get step 0) 1)
      (vector-get step 2)
      (do
        (root_push step)
        (let [next-idx (vector-get step 1)
          next-record-ty (vector-get step 2)]
          (do
            (root_push next-record-ty)
            (let [resolved
              (typeinfer-record-build-type-rooted-v3
                next-record-ty field-types next-idx len)]
              (do
                (root_pop) (root_pop)
                resolved))))))))
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
    (let [decl (typeinfer-record-decl-unprivate (vector-get program idx))
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
  (typeinfer-record-constructor-type-rooted-v3 record-ty len idx result))
(defn typeinfer-record-constructor-type-step-state [done next-idx result]
  (vector-push-triple-rooted (vector-new 3) done next-idx result))
(defn typeinfer-record-constructor-type-step-v3
  [record-ty idx lower result]
  (if (<= idx lower)
    (typeinfer-record-constructor-type-step-state 1 idx result)
    (do
      (root_push record-ty)
      (root_push result)
      (let [field-ty (vector-get record-ty (- idx 1))]
        (do
          (root_push field-ty)
          (let [next-result (mk-fun field-ty result)]
            (do
              (root_push next-result)
              (let [state
                (typeinfer-record-constructor-type-step-state
                  0 (- idx 2) next-result)]
                (do
                  (root_pop) (root_pop) (root_pop) (root_pop)
                  state)))))))))
(defn typeinfer-record-constructor-type-step-64-loop-bounded
  [record-ty idx lower result remaining]
  (do
    (root_push record-ty)
    (root_push result)
    (let [step
      (typeinfer-record-constructor-type-step-v3
        record-ty idx lower result)
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
              (typeinfer-record-constructor-type-step-64-loop-bounded
                record-ty
                next-idx
                lower
                next-result
                (- remaining 1))))]
          (do
            (root_pop) (root_pop) (root_pop) (root_pop)
            parsed))))))
(defn typeinfer-record-constructor-type-step-64
  [record-ty idx lower result]
  (typeinfer-record-constructor-type-step-64-loop-bounded
    record-ty idx lower result 64))
(defn typeinfer-record-constructor-type-rooted-v3
  [record-ty idx lower result]
  (let [step
    (typeinfer-record-constructor-type-step-64
      record-ty idx lower result)]
    (if (= (vector-get step 0) 1)
      (vector-get step 2)
      (do
        (root_push step)
        (let [next-idx (vector-get step 1)
          next-result (vector-get step 2)]
          (do
            (root_push next-result)
            (let [resolved
              (typeinfer-record-constructor-type-rooted-v3
                record-ty next-idx lower next-result)]
              (do
                (root_pop) (root_pop)
                resolved))))))))
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
;; Type.field accessor を record schema と同じ bound variable で値環境へ登録する。
;; raw field は [field-hash, accessor-hash, raw-TypeExpr] の triple で保持されるが、
;; schema の record-ty は [field-hash, field-ty] の pair だけを持つ。
(defn typeinfer-register-record-accessors-loop [raw-fields idx len env record-ty bound-vars]
  (if (>= idx len)
    env
    (do
      (root_push raw-fields)
      (root_push env)
      (root_push record-ty)
      (root_push bound-vars)
      (let [field-hash (vector-get raw-fields idx)
        accessor-hash (vector-get raw-fields (+ idx 1))
        field-ty (type-record-field-type record-ty field-hash)]
        (do
          (root_push field-ty)
          (let [accessor-ty (mk-fun record-ty field-ty)]
            (do
              (root_push accessor-ty)
              (let [accessor-scheme (poly accessor-ty bound-vars)]
                (do
                  (root_push accessor-scheme)
                  (let [next-env
                          (type-env-insert env accessor-hash accessor-scheme)]
                    (do
                      (root_push next-env)
                      (let [result
                              (typeinfer-register-record-accessors-loop
                                raw-fields
                                (+ idx 3)
                                len
                                next-env
                                record-ty
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
                          result)))))))))))))
(defn typeinfer-register-record-accessors [raw-fields env record-ty bound-vars]
  (if (= raw-fields 0)
    env
    (do
      (root_push raw-fields)
      (root_push env)
      (root_push record-ty)
      (root_push bound-vars)
      (let [result
              (typeinfer-register-record-accessors-loop
                raw-fields
                0
                (vector-length raw-fields)
                env
                record-ty
                bound-vars)]
        (do
          (root_pop)
          (root_pop)
          (root_pop)
          (root_pop)
          result)))))
;; record constructor と Type.field accessor を schema と同じ bound variable で値環境へ登録する。
(defn typeinfer-register-record-def [decl env record-env]
  (let [record-name-hash (vector-get decl 1)
    schema (map-get-safe record-env (vector-get decl 1))
    raw-fields (typeinfer-record-decl-field-exprs decl)]
    (if (= schema 0)
      env
      (do
        (root_push schema)
        (root_push env)
        (root_push raw-fields)
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
                        (root_push next-env)
                        (let [result
                                (typeinfer-register-record-accessors
                                  raw-fields
                                  next-env
                                  record-ty
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
                            result))))))))))))))
(defn typeinfer-register-record-defs-loop [program idx len env record-env]
  (if (>= idx len)
    env
    (let [decl (typeinfer-record-decl-unprivate (vector-get program idx))
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
