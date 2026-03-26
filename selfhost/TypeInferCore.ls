(module TypeInferCore)
(import AST)
(import Type)
(import TypeScheme)

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

;; 型コンストラクタの名前ハッシュ
(defn hash-int [] 100)
(defn hash-bool [] 200)
(defn hash-string [] 300)
(defn hash-float [] 400)
(defn hash-unit [] 500)

;; ============================================================
;; 型構築ヘルパー (Type.ls の関数を直接使用)
;; 連結コンパイル時に Type.ls の make-type-* が利用可能
;; ============================================================

(defn mk-int [] (make-type-int))
(defn mk-bool [] (make-type-bool))
(defn mk-string [] (make-type-string))
(defn mk-float [] (make-type-float))
(defn mk-unit [] (make-type-unit))
(defn mk-con [name-hash]
  (vector-push (vector-push (vector-new 2) (ty-con)) name-hash))
(defn mk-var [id] (make-type-var id))
(defn mk-fun [p r] (make-type-fun p r))

;; 型アクセサ (Type.ls を利用)
(defn ty-tag [ty] (type-tag ty))
(defn ty-name [ty] (type-name ty))
(defn ty-fp [ty] (type-fun-param ty))
(defn ty-fr [ty] (type-fun-ret ty))

;; ============================================================
;; 型環境 (TypeEnv)
;; ============================================================
;; HashMap<name-hash, TypeScheme>

(defn type-env-new []
  (map-new))

;; 型環境に型スキームを追加
(defn type-env-insert [env name-hash scheme]
  (map-insert env name-hash scheme))

;; 型環境から型スキームを取得 (0 = 未定義)
(defn type-env-lookup [env name-hash]
  (map-get env name-hash))

;; ============================================================
;; 推論結果 = [subst, type, error-code]
;; ============================================================

(defn make-result [subst ty]
  (vector-push (vector-push (vector-push (vector-new 3) subst) ty) 0))

(defn result-subst [r] (vector-get r 0))
(defn result-type [r] (vector-get r 1))
(defn result-error-code [r] (vector-get r 2))

;; エラー結果 (subst にエラーマーカー付き)
(defn make-error-result-code [code]
  (vector-push
    (vector-push
      (vector-push (vector-new 3) (map-insert (map-new) -1 1))
      (mk-int))
    code))

(defn make-error-result []
  (make-error-result-code 6))

(defn propagate-error-result [r]
  (make-error-result-code (result-error-code r)))

;; 結果がエラーか判定
(defn result-failed [r]
  (map-get (result-subst r) -1))

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
;; alias-env: HashMap<name-hash, target-type>
;; ty: 解決対象の型
;; 展開はシャロー (1段階のみ)
(defn resolve-alias [alias-env ty]
  (let [tag (ty-tag ty)]
    (if (= tag (ty-con))
      ;; Con 型: エイリアス環境を参照
      (let [name (ty-name ty)
            target (map-get alias-env name)]
        (if (= target 0)
          ;; エイリアスなし: そのまま返す
          ty
          ;; エイリアスあり: 展開
          target))
      ;; Con 以外: そのまま返す
      ty)))

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
