(module Types.TypeInferCore)
(import Syntax.AST)
(import Types.Type)
(import Types.TypeScheme)

;; ============================================================
;; AST タグ定数 (AST.ls から参照)
;; ============================================================

(defn tag-lit-int [] 1)
(defn tag-lit-bool [] 2)
(defn tag-lit-string [] 3)
(defn tag-lit-float [] 19)
(defn tag-lit-unit [] 32)
(defn tag-var [] 4)
(defn tag-apply [] 5)
(defn tag-if [] 6)
(defn tag-let [] 7)
(defn tag-ann [] 11)
(defn tag-recordlit [] 12)
(defn tag-fieldaccess [] 13)
(defn tag-recordupdate [] 14)
(defn tag-computation [] 15)
(defn tag-quote [] 16)
(defn tag-unquote [] 17)
(defn tag-unquote-splice [] 18)
(defn tag-lambda [] 8)
(defn tag-do [] 9)
(defn tag-match [] 10)
(defn tag-type-named [] 60)
(defn tag-type-app [] 61)
(defn tag-type-fun [] 62)
(defn tag-type-var [] 63)

;; computation step kind
(defn comp-step-expr [] 0)
(defn comp-step-let-bang [] 1)
(defn comp-step-do-bang [] 2)
(defn comp-step-return [] 3)

;; ============================================================
;; 型タグ定数 (Type.ls から参照)
;; ============================================================

(defn ty-con [] 1)
(defn ty-var [] 2)
(defn ty-fun [] 3)
(defn ty-record [] 4)
(defn ty-app [] 5)

;; 型コンストラクタの名前ハッシュ
(defn hash-int [] 100)
(defn hash-bool [] 200)
(defn hash-string [] 300)
(defn hash-float [] 400)
(defn hash-unit [] 500)
(defn hash-vector [] 600)
(defn hash-map [] 700)
(defn hash-ref [] 800)

;; ============================================================
;; 型構築ヘルパー (Type.ls の関数を直接使用)
;; 連結コンパイル時に Type.ls の make-type-* が利用可能
;; ============================================================

(defn mk-int [] (make-type-int))
(defn mk-bool [] (make-type-bool))
(defn mk-string [] (make-type-string))
(defn mk-float [] (make-type-float))
(defn mk-unit [] (make-type-unit))
(defn mk-vector [] (mk-con (hash-vector)))
(defn mk-map [] (mk-con (hash-map)))
(defn mk-con [name-hash]
  (vector-push (vector-push (vector-new 2) (ty-con)) name-hash))
(defn mk-var [id] (make-type-var id))
(defn mk-fun [p r] (make-type-fun p r))
(defn mk-app [name-hash args] (make-type-app name-hash args))
(defn mk-app1 [name-hash arg] (make-type-app1 name-hash arg))
(defn mk-ref [inner] (mk-app1 (hash-ref) inner))

;; source TypeExpr の primitive 名は internal Type の識別子とは別空間である。
(defn source-type-int-hash [] 73679)
(defn source-type-bool-hash [] 2076426)
(defn source-type-string-hash [] 2486848561)
(defn source-type-float-hash [] 67973692)
(defn source-type-unit-hash [] 2641316)
(defn source-type-vector-hash [] 2558446947)
(defn source-type-map-hash [] 77116)
(defn source-type-ref-hash [] 82035)

;; raw TypeExpr を現在利用可能な internal Type へ解決する。
;; 未登録名は nominal constructor として保持し、ADT/alias registry の導入後に解決を拡張する。
(defn typeinfer-resolve-named-type [name-hash]
  (if (= name-hash (source-type-int-hash))
    (mk-int)
    (if (= name-hash (source-type-bool-hash))
      (mk-bool)
      (if (= name-hash (source-type-string-hash))
        (mk-string)
        (if (= name-hash (source-type-float-hash))
          (mk-float)
          (if (= name-hash (source-type-unit-hash))
            (mk-unit)
            (mk-con name-hash)))))))

(defn typeinfer-resolve-app-name [name-hash]
  (if (= name-hash (source-type-vector-hash))
    (hash-vector)
    (if (= name-hash (source-type-map-hash))
      (hash-map)
      (if (= name-hash (source-type-ref-hash))
        (hash-ref)
        name-hash))))

(defn typeinfer-resolve-type-expr-args-state [done next-idx args]
  (vector-push-triple-rooted (vector-new 3) done next-idx args))

(defn typeinfer-resolve-type-expr-args-step-v3 [type-expr idx count args]
  (if (>= idx count)
    (typeinfer-resolve-type-expr-args-state 1 idx args)
    (do
      (root_push args)
      (let [arg-type (typeinfer-resolve-type-expr (vector-get type-expr (+ idx 3)))]
        (do
          (root_push arg-type)
          (let [next-args (push-object-vector-local args arg-type)]
            (do
              (root_pop)
              (root_pop)
              (typeinfer-resolve-type-expr-args-state 0 (+ idx 1) next-args))))))))

(defn typeinfer-resolve-type-expr-args-step-64-loop-bounded
  [type-expr idx count args remaining]
  (do
    (root_push args)
    (let [step (typeinfer-resolve-type-expr-args-step-v3 type-expr idx count args)
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
              (typeinfer-resolve-type-expr-args-step-64-loop-bounded
                type-expr next-idx count next-args (- remaining 1))))]
          (do
            (root_pop)
            (root_pop)
            (root_pop)
            parsed))))))

(defn typeinfer-resolve-type-expr-args-step-64
  [type-expr idx count args]
  (typeinfer-resolve-type-expr-args-step-64-loop-bounded
    type-expr idx count args 64))

(defn typeinfer-resolve-type-expr-args-rooted-v3
  [type-expr idx count args]
  (let [step (typeinfer-resolve-type-expr-args-step-64
    type-expr idx count args)]
    (if (= (vector-get step 0) 1)
      (vector-get step 2)
      (do
        (root_push step)
        (let [next-idx (vector-get step 1)
          next-args (vector-get step 2)]
          (do
            (root_push next-args)
            (let [resolved
              (typeinfer-resolve-type-expr-args-rooted-v3
                type-expr next-idx count next-args)]
              (do
                (root_pop)
                (root_pop)
                resolved))))))))

(defn typeinfer-resolve-type-expr-args-loop [type-expr idx count args]
  (typeinfer-resolve-type-expr-args-rooted-v3 type-expr idx count args))

(defn typeinfer-resolve-app-type [type-expr]
  (do
    (root_push type-expr)
    (let [name-hash (vector-get type-expr 1)
      arg-count (vector-get type-expr 2)
      args (typeinfer-resolve-type-expr-args-loop type-expr 0 arg-count (vector-new arg-count))]
      (do
        (root_push args)
        (let [result (mk-app (typeinfer-resolve-app-name name-hash) args)]
          (do
            (root_pop)
            (root_pop)
            result))))))

(defn typeinfer-resolve-fun-params-state [done next-idx result]
  (vector-push-triple-rooted (vector-new 3) done next-idx result))

(defn typeinfer-resolve-fun-params-step-v3 [type-expr idx return-type]
  (if (<= idx 0)
    (typeinfer-resolve-fun-params-state 1 idx return-type)
    (do
      (root_push return-type)
      (let [param-type (typeinfer-resolve-type-expr (vector-get type-expr (+ idx 1)))]
        (do
          (root_push param-type)
          (let [next-result (mk-fun param-type return-type)]
            (do
              (root_pop)
              (root_pop)
              (typeinfer-resolve-fun-params-state 0 (- idx 1) next-result))))))))

(defn typeinfer-resolve-fun-params-step-64-loop-bounded
  [type-expr idx return-type remaining]
  (do
    (root_push return-type)
    (let [step (typeinfer-resolve-fun-params-step-v3 type-expr idx return-type)
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
              (typeinfer-resolve-fun-params-step-64-loop-bounded
                type-expr next-idx next-result (- remaining 1)) ))]
          (do
            (root_pop)
            (root_pop)
            (root_pop)
            parsed))))))

(defn typeinfer-resolve-fun-params-step-64
  [type-expr idx return-type]
  (typeinfer-resolve-fun-params-step-64-loop-bounded
    type-expr idx return-type 64))

(defn typeinfer-resolve-fun-params-rooted-v3 [type-expr idx return-type]
  (let [step (typeinfer-resolve-fun-params-step-64
    type-expr idx return-type)]
    (if (= (vector-get step 0) 1)
      (vector-get step 2)
      (do
        (root_push step)
        (let [next-idx (vector-get step 1)
          next-result (vector-get step 2)]
          (do
            (root_push next-result)
            (let [resolved
              (typeinfer-resolve-fun-params-rooted-v3
                type-expr next-idx next-result)]
              (do
                (root_pop)
                (root_pop)
                resolved))))))))

(defn typeinfer-resolve-fun-params-loop [type-expr idx count return-type]
  (typeinfer-resolve-fun-params-rooted-v3 type-expr count return-type))

(defn typeinfer-resolve-fun-type [type-expr]
  (do
    (root_push type-expr)
    (let [param-count (vector-get type-expr 1)
      return-type-expr (vector-get type-expr (+ param-count 2))
      return-type (typeinfer-resolve-type-expr return-type-expr)]
      (do
        (root_push return-type)
        (let [result (typeinfer-resolve-fun-params-loop type-expr 0 param-count return-type)]
          (do
            (root_pop)
            (root_pop)
            result))))))

(defn typeinfer-resolve-type-expr [type-expr]
  (if (= type-expr 0)
    (mk-con 0)
    (let [tag (vector-get type-expr 0)]
      (if (= tag (tag-type-named))
        (typeinfer-resolve-named-type (vector-get type-expr 1))
        (if (= tag (tag-type-app))
          (typeinfer-resolve-app-type type-expr)
          (if (= tag (tag-type-fun))
            (typeinfer-resolve-fun-type type-expr)
            (if (= tag (tag-type-var))
              ;; defn annotation の free variable は Rust implementation と同じ nominal name として扱う。
              (mk-con (vector-get type-expr 1))
              (mk-con 0))))))))

;; 型アクセサ (Type.ls を利用)
(defn ty-tag [ty] (type-tag ty))
(defn ty-name [ty] (type-name ty))
(defn ty-fp [ty] (type-fun-param ty))
(defn ty-fr [ty] (type-fun-ret ty))

;; ============================================================
;; 型環境 (TypeEnv)
;; ============================================================
;; HashMap<name-hash, TypeScheme>

(defn push-int-vector-local [dst value]
  (do
    (root_push dst)
    (let [next-dst (vector-push dst value)]
      (do
        (root_pop)
        next-dst))))

(defn push-object-vector-local [dst value]
  (do
    (root_push dst)
    (root_push value)
    (let [next-dst (vector-push dst value)]
      (do
        (root_pop)
        (root_pop)
        next-dst))))

(defn map-get-safe [m key]
  (do
    (root_push m)
    (let [value (map-get m key)]
      (do
        (root_pop)
        value))))

(defn map-insert-object-safe [m key value]
  (do
    (root_push m)
    (root_push value)
    (let [next-map (map-insert m key value)]
      (do
        (root_pop)
        (root_pop)
        next-map))))

(defn map-insert-int-safe [m key value]
  (do
    (root_push m)
    (let [next-map (map-insert m key value)]
      (do
        (root_pop)
        next-map))))

(defn map-remove-object-safe [m key]
  (do
    (root_push m)
    (let [next-map (map-remove m key)]
      (do
        (root_pop)
        next-map))))

(defn type-env-new []
  (map-new))

;; 型環境に型スキームを追加
(defn type-env-insert [env name-hash scheme]
  (map-insert-object-safe env name-hash scheme))

(defn type-env-remove [env name-hash]
  (map-remove-object-safe env name-hash))

;; 型環境から型スキームを取得 (0 = 未定義)
(defn type-env-lookup [env name-hash]
  (map-get-safe env name-hash))

;; ============================================================
;; 推論結果 = [subst, type, error-code]。失敗時だけ [start, end] を後置できる。
;; ============================================================

(defn make-result [subst ty]
  (do
    ;; 推論結果の3要素を段階的に構築する間も、置換・型・中間 vector を
    ;; native GC から保持する。root slot は vector-push の再配置後に更新する。
    (root_push subst)
    (root_push ty)
    (let [base (vector-new 3)
      slot (root_push base)]
      (do
        (let [with-subst (vector-push base subst)]
          (do
            (root_set slot with-subst)
            (let [with-ty (vector-push with-subst ty)]
              (do
                (root_set slot with-ty)
                (let [result (vector-push with-ty 0)]
                  (do
                    (root_set slot result)
                    (root_pop)
                    (root_pop)
                    (root_pop)
                    result))))))))))

(defn result-subst [r] (vector-get r 0))
(defn result-type [r] (vector-get r 1))
(defn result-error-code [r] (vector-get r 2))
(defn result-error-start [r]
  (if (> (vector-length r) 3) (vector-get r 3) -1))
(defn result-error-end [r]
  (if (> (vector-length r) 4) (vector-get r 4) -1))
(defn result-error-name-hash [r]
  (if (> (vector-length r) 5) (vector-get r 5) -1))

;; エラー結果 (subst にエラーマーカー付き)
(defn make-error-result-code [code]
  (push-int-vector-local
    (push-object-vector-local
      (push-object-vector-local (vector-new 3) (map-insert (map-new) -1 1))
      (mk-int))
    code))

(defn make-error-result-code-with-span [code start end]
  (let [base (make-error-result-code code)]
    (do
      (let [base-slot (root_push base)
        with-start (vector-push base start)]
        (do
          (root_set base-slot with-start)
          (let [result (vector-push with-start end)]
            (do
              (root_pop)
              result)))))))

(defn make-error-result-code-and-name [code name-hash]
  (make-error-result-code-with-span-and-name code -1 -1 name-hash))

(defn make-error-result-code-with-span-and-name [code start end name-hash]
  (let [base (make-error-result-code-with-span code start end)]
    (do
      (root_push base)
      (let [result (vector-push base name-hash)]
        (do
          (root_pop)
          result)))))

(defn make-error-result []
  (make-error-result-code 6))

(defn propagate-error-result [r]
  (make-error-result-code (result-error-code r)))

(defn propagate-error-result-with-span [r]
  (let [start (result-error-start r)
    end (result-error-end r)]
    (if (and (>= start 0) (>= end start))
      (make-error-result-code-with-span (result-error-code r) start end)
      (make-error-result-code (result-error-code r)))))

(defn propagate-error-result-with-span-and-name [r]
  (let [start (result-error-start r)
    end (result-error-end r)
    name-hash (result-error-name-hash r)]
    (if (>= name-hash 0)
      (if (and (>= start 0) (>= end start))
        (make-error-result-code-with-span-and-name (result-error-code r) start end name-hash)
        (make-error-result-code-and-name (result-error-code r) name-hash))
      (propagate-error-result-with-span r))))

;; 結果がエラーか判定
(defn result-failed [r]
  (map-get-safe (result-subst r) -1))

;; ============================================================
;; 新しい型変数の生成
;; ============================================================

(defn fresh-type-var [counter]
  (mk-var (next-var counter)))

;; ============================================================
;; HKT (Higher-Kinded Types) 支援
;; ============================================================

;; hkt-apply: 高カインド型コンストラクタ F に型引数 A を適用
;; 例: (hkt-apply List Int) => List<Int>
;; F = [ty-con, F-hash], A = 任意の型
;; 結果: [ty-con, applied-hash] (簡易版: ハッシュを合成)
(defn hkt-apply [f-ty arg-ty]
  (let [f-tag (ty-tag f-ty)]
    (if (= f-tag (ty-con))
      ;; 型コンストラクタに引数を適用
      ;; 簡易版: F の名前ハッシュと A のハッシュを組み合わせた新しい Con を作る
      (let [f-name (ty-name f-ty)
            ;; 適用結果のハッシュ = F * 1000 + A のタグ値
            result-hash (+ (* f-name 1000) (ty-tag arg-ty))]
        ;; Con 型を構築: [1, result-hash]
        (vector-push (vector-push (vector-new 2) 1) result-hash))
      ;; 型変数や関数型には適用不可: そのまま返す
      f-ty)))

;; ============================================================
;; GADT (Generalized Algebraic Data Types) 支援
;; ============================================================

;; gadt-check: GADT コンストラクタのパターンマッチで
;; 返り型の等式制約を環境に注入
;; ctor-ty: コンストラクタの型、scrut-ty: scrutinee の型、
;; env: 現在の型環境、subst: 現在の置換
(defn gadt-check [ctor-ty scrut-ty env subst]
  (let [s (unify ctor-ty scrut-ty subst)]
    (if (= (unify-failed s) 1)
      ;; 単一化失敗: エラー結果
      (vector-push (make-error-result) env)
      ;; 成功: 更新された置換と環境を返す
      (vector-push (make-result s scrut-ty) env))))

;; ============================================================
;; Type Alias 解決
;; ============================================================

;; resolve-alias: 型エイリアスを展開する
;; alias-env: [closed-aliases, parametric-aliases]
;; ty: 解決対象の型
;; 展開はシャロー (1段階のみ)
(defn resolve-alias [alias-env ty]
  (let [tag (ty-tag ty)]
    (if (= tag (ty-con))
      ;; Con 型: エイリアス環境を参照
      (let [name (ty-name ty)
            closed-aliases (type-alias-env-closed alias-env)
            target (map-get-safe closed-aliases name)]
        (if (= target 0)
          ;; エイリアスなし: そのまま返す
          ty
          ;; エイリアスあり: 展開
          target))
      ;; Con 以外: そのまま返す
      ty)))

;; alias-env は [closed-aliases, parametric-aliases]。
;; closed alias は named type を 1 段透過展開する。
(defn typeinfer-resolve-named-type-with-aliases [name-hash alias-env]
  (let [base-type (typeinfer-resolve-named-type name-hash)]
    (do
      (root_push base-type)
      (let [resolved (resolve-alias alias-env base-type)]
        (do
          (root_pop)
          resolved)))))

;; parametric alias target の raw TypeExpr だけは source parameter を型変数へ解決する。
(defn typeinfer-resolve-type-var-with-aliases-and-params [name-hash alias-env type-param-env]
  (let [bound (map-get-safe type-param-env name-hash)]
    (if (= bound 0)
      (typeinfer-resolve-named-type-with-aliases name-hash alias-env)
      bound)))

(defn typeinfer-resolve-type-expr-args-with-aliases-state [done next-idx args]
  (vector-push-triple-rooted (vector-new 3) done next-idx args))

(defn typeinfer-resolve-type-expr-args-with-aliases-step-v3
  [type-expr idx count args alias-env type-param-env]
  (if (>= idx count)
    (typeinfer-resolve-type-expr-args-with-aliases-state 1 idx args)
    (do
      (root_push args)
      (root_push alias-env)
      (root_push type-param-env)
      (let [arg-type
              (typeinfer-resolve-type-expr-with-aliases-and-params
                (vector-get type-expr (+ idx 3))
                alias-env
                type-param-env)]
        (do
          (root_push arg-type)
          (let [next-args (push-object-vector-local args arg-type)]
            (do
              (root_pop)
              (root_pop)
              (root_pop)
              (root_pop)
              (typeinfer-resolve-type-expr-args-with-aliases-state
                0 (+ idx 1) next-args))))))))

(defn typeinfer-resolve-type-expr-args-with-aliases-step-64-loop-bounded
  [type-expr idx count args alias-env type-param-env remaining]
  (do
    (root_push args)
    (root_push alias-env)
    (root_push type-param-env)
    (let [step
            (typeinfer-resolve-type-expr-args-with-aliases-step-v3
              type-expr idx count args alias-env type-param-env)
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
              (typeinfer-resolve-type-expr-args-with-aliases-step-64-loop-bounded
                type-expr
                next-idx
                count
                next-args
                alias-env
                type-param-env
                (- remaining 1))))]
          (do
            (root_pop)
            (root_pop)
            (root_pop)
            (root_pop)
            (root_pop)
            parsed))))))

(defn typeinfer-resolve-type-expr-args-with-aliases-step-64
  [type-expr idx count args alias-env type-param-env]
  (typeinfer-resolve-type-expr-args-with-aliases-step-64-loop-bounded
    type-expr idx count args alias-env type-param-env 64))

(defn typeinfer-resolve-type-expr-args-with-aliases-rooted-v3
  [type-expr idx count args alias-env type-param-env]
  (let [step
          (typeinfer-resolve-type-expr-args-with-aliases-step-64
            type-expr idx count args alias-env type-param-env)]
    (if (= (vector-get step 0) 1)
      (vector-get step 2)
      (do
        (root_push step)
        (let [next-idx (vector-get step 1)
          next-args (vector-get step 2)]
          (do
            (root_push next-args)
            (let [resolved
              (typeinfer-resolve-type-expr-args-with-aliases-rooted-v3
                type-expr next-idx count next-args alias-env type-param-env)]
              (do
                (root_pop)
                (root_pop)
                resolved))))))))

(defn typeinfer-resolve-type-expr-args-with-aliases-loop
  [type-expr idx count args alias-env type-param-env]
  (typeinfer-resolve-type-expr-args-with-aliases-rooted-v3
    type-expr idx count args alias-env type-param-env))

(defn typeinfer-build-parametric-alias-subst-state [done next-idx subst]
  (vector-push-triple-rooted (vector-new 3) done next-idx subst))

(defn typeinfer-build-parametric-alias-subst-step-v3
  [param-types args idx count subst]
  (if (>= idx count)
    (typeinfer-build-parametric-alias-subst-state 1 idx subst)
    (do
      (root_push param-types)
      (root_push args)
      (root_push subst)
      (let [param-type (vector-get param-types idx)
        arg-type (vector-get args idx)
        next-subst (map-insert-object-safe subst (type-name param-type) arg-type)]
        (do
          (root_push next-subst)
          (let [step
            (typeinfer-build-parametric-alias-subst-state
              0
              (+ idx 1)
              next-subst)]
            (do
              (root_pop)
              (root_pop)
              (root_pop)
              (root_pop)
              step)))))))

(defn typeinfer-build-parametric-alias-subst-step-64-loop-bounded
  [param-types args idx count subst remaining]
  (do
    (root_push param-types)
    (root_push args)
    (root_push subst)
    (let [step
            (typeinfer-build-parametric-alias-subst-step-v3
              param-types args idx count subst)
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
              (typeinfer-build-parametric-alias-subst-step-64-loop-bounded
                param-types
                args
                next-idx
                count
                next-subst
                (- remaining 1))))]
          (do
            (root_pop)
            (root_pop)
            (root_pop)
            (root_pop)
            (root_pop)
            parsed))))))

(defn typeinfer-build-parametric-alias-subst-step-64
  [param-types args idx count subst]
  (typeinfer-build-parametric-alias-subst-step-64-loop-bounded
    param-types args idx count subst 64))

(defn typeinfer-build-parametric-alias-subst-rooted-v3
  [param-types args idx count subst]
  (let [step
          (typeinfer-build-parametric-alias-subst-step-64
            param-types args idx count subst)]
    (if (= (vector-get step 0) 1)
      (vector-get step 2)
      (do
        (root_push step)
        (let [next-idx (vector-get step 1)
          next-subst (vector-get step 2)]
          (do
            (root_push next-subst)
            (let [resolved
              (typeinfer-build-parametric-alias-subst-rooted-v3
                param-types args next-idx count next-subst)]
              (do
                (root_pop)
                (root_pop)
                resolved))))))))

(defn typeinfer-build-parametric-alias-subst-loop
  [param-types args idx count subst]
  (typeinfer-build-parametric-alias-subst-rooted-v3
    param-types args idx count subst))

;; parametric alias entry = [parameter-type-vars, resolved-target-type]
(defn typeinfer-resolve-parametric-alias-application [entry args]
  (do
    (root_push entry)
    (root_push args)
    (let [param-types (vector-get entry 0)
      target-type (vector-get entry 1)
      param-count (vector-length param-types)]
      (do
        (root_push param-types)
        (root_push target-type)
        (let [result
          (if (= param-count (vector-length args))
            (let [subst
              (typeinfer-build-parametric-alias-subst-loop
                param-types
                args
                0
                param-count
                (map-new))]
              (do
                (root_push subst)
                (let [expanded (instantiate-apply subst target-type)]
                  (do
                    (root_pop)
                    expanded))))
            0)]
          (do
            (root_pop)
            (root_pop)
            (root_pop)
            (root_pop)
            result))))))

(defn typeinfer-resolve-app-type-with-aliases-and-params [type-expr alias-env type-param-env]
  (do
    (root_push type-expr)
    (root_push alias-env)
    (root_push type-param-env)
    (let [name-hash (vector-get type-expr 1)
      arg-count (vector-get type-expr 2)
      args
        (typeinfer-resolve-type-expr-args-with-aliases-loop
          type-expr
          0
          arg-count
          (vector-new arg-count)
          alias-env
          type-param-env)]
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
                    (let [expanded (typeinfer-resolve-parametric-alias-application entry args)]
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

(defn typeinfer-resolve-fun-params-with-aliases-state [done next-idx result]
  (vector-push-triple-rooted (vector-new 3) done next-idx result))

(defn typeinfer-resolve-fun-params-with-aliases-step-v3
  [type-expr idx return-type alias-env type-param-env]
  (if (<= idx 0)
    (typeinfer-resolve-fun-params-with-aliases-state 1 idx return-type)
    (do
      (root_push return-type)
      (root_push alias-env)
      (root_push type-param-env)
      (let [param-type
              (typeinfer-resolve-type-expr-with-aliases-and-params
                (vector-get type-expr (+ idx 1))
                alias-env
                type-param-env)]
        (do
          (root_push param-type)
          (let [next-result (mk-fun param-type return-type)]
            (do
              (root_pop)
              (root_pop)
              (root_pop)
              (root_pop)
              (typeinfer-resolve-fun-params-with-aliases-state
                0 (- idx 1) next-result))))))))

(defn typeinfer-resolve-fun-params-with-aliases-step-64-loop-bounded
  [type-expr idx return-type alias-env type-param-env remaining]
  (do
    (root_push return-type)
    (root_push alias-env)
    (root_push type-param-env)
    (let [step
            (typeinfer-resolve-fun-params-with-aliases-step-v3
              type-expr idx return-type alias-env type-param-env)
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
              (typeinfer-resolve-fun-params-with-aliases-step-64-loop-bounded
                type-expr
                next-idx
                next-result
                alias-env
                type-param-env
                (- remaining 1))))]
          (do
            (root_pop)
            (root_pop)
            (root_pop)
            (root_pop)
            (root_pop)
            parsed))))))

(defn typeinfer-resolve-fun-params-with-aliases-step-64
  [type-expr idx return-type alias-env type-param-env]
  (typeinfer-resolve-fun-params-with-aliases-step-64-loop-bounded
    type-expr idx return-type alias-env type-param-env 64))

(defn typeinfer-resolve-fun-params-with-aliases-rooted-v3
  [type-expr idx return-type alias-env type-param-env]
  (let [step
          (typeinfer-resolve-fun-params-with-aliases-step-64
            type-expr idx return-type alias-env type-param-env)]
    (if (= (vector-get step 0) 1)
      (vector-get step 2)
      (do
        (root_push step)
        (let [next-idx (vector-get step 1)
          next-result (vector-get step 2)]
          (do
            (root_push next-result)
            (let [resolved
              (typeinfer-resolve-fun-params-with-aliases-rooted-v3
                type-expr next-idx next-result alias-env type-param-env)]
              (do
                (root_pop)
                (root_pop)
                resolved))))))))

(defn typeinfer-resolve-fun-params-with-aliases-loop
  [type-expr idx count return-type alias-env type-param-env]
  (typeinfer-resolve-fun-params-with-aliases-rooted-v3
    type-expr count return-type alias-env type-param-env))

(defn typeinfer-resolve-fun-type-with-aliases-and-params [type-expr alias-env type-param-env]
  (do
    (root_push type-expr)
    (root_push alias-env)
    (root_push type-param-env)
    (let [param-count (vector-get type-expr 1)
      return-type-expr (vector-get type-expr (+ param-count 2))
      return-type
        (typeinfer-resolve-type-expr-with-aliases-and-params
          return-type-expr
          alias-env
          type-param-env)]
      (do
        (root_push return-type)
        (let [result
                (typeinfer-resolve-fun-params-with-aliases-loop
                  type-expr
                  0
                  param-count
                  return-type
                  alias-env
                  type-param-env)]
          (do
            (root_pop)
            (root_pop)
            (root_pop)
            (root_pop)
            result))))))

(defn typeinfer-resolve-type-expr-with-aliases-and-params [type-expr alias-env type-param-env]
  (if (= type-expr 0)
    (mk-con 0)
    (let [tag (vector-get type-expr 0)]
      (if (= tag (tag-type-named))
        (typeinfer-resolve-named-type-with-aliases (vector-get type-expr 1) alias-env)
        (if (= tag (tag-type-app))
          (typeinfer-resolve-app-type-with-aliases-and-params type-expr alias-env type-param-env)
          (if (= tag (tag-type-fun))
            (typeinfer-resolve-fun-type-with-aliases-and-params type-expr alias-env type-param-env)
            (if (= tag (tag-type-var))
              (typeinfer-resolve-type-var-with-aliases-and-params
                (vector-get type-expr 1)
                alias-env
                type-param-env)
              (mk-con 0))))))))

(defn typeinfer-resolve-type-expr-with-aliases [type-expr alias-env]
  (do
    (root_push type-expr)
    (root_push alias-env)
    (let [type-param-env (map-new)]
      (do
        (root_push type-param-env)
        (let [result
          (typeinfer-resolve-type-expr-with-aliases-and-params
            type-expr
            alias-env
            type-param-env)]
          (do
            (root_pop)
            (root_pop)
            (root_pop)
            result))))))

;; ============================================================
;; Record Update 式の型推論
;; ============================================================

;; infer-record-update: レコード更新式 { base | field1 = e1, ... }
;; base-result: base 式の推論結果 [subst, type]
;; field-hash: 更新するフィールドのハッシュ
;; field-result: フィールド値の推論結果 [subst, type]
;; 結果: [subst, type] (base と同じ型)
(defn infer-record-update [base-result field-hash field-result]
  (if (= (result-failed base-result) 1)
    (make-error-result)
    (if (= (result-failed field-result) 1)
      (make-error-result)
      ;; base の型をそのまま返す (フィールドの型チェックは省略)
      (let [s1 (result-subst base-result)
            base-ty (result-type base-result)
            s2 (result-subst field-result)]
        (make-result s2 base-ty)))))

;; ============================================================
;; Type Error コード定数
;; ============================================================

;; エラーコード (E0001 形式)
(defn error-code-undefined [] 1)     ;; E0001: 未定義変数
(defn error-code-if-cond [] 2)       ;; E0002: if 条件が Bool でない
(defn error-code-if-branch [] 3)     ;; E0003: if 分岐の型不一致
(defn error-code-arg-mismatch [] 4)  ;; E0004: 関数引数の型不一致
(defn error-code-infinite [] 5)      ;; E0005: 無限型 (occurs check)
(defn error-code-general [] 6)       ;; E0006: 一般的な型不一致
