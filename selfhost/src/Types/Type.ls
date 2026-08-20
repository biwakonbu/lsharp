(module Types.Type)

;; Type.ls - L# セルフホスティング: 型定義
;;
;; Hindley-Milner 型推論で使用する型の表現。
;; 整数タグ + Vector でエンコード。

;; === 型種別 ===

;; 型コンストラクタ: [1, name-hash]
(defn type-con [] 1)

;; 型変数: [2, id]
(defn type-var [] 2)

;; 関数型: [3, param-type, return-type]
(defn type-fun [] 3)

;; レコード型: [4, name-hash, field1-hash, field1-type, ...]
(defn type-record [] 4)

;; 型適用: [5, name-hash, arg-count, arg1, ...]
(defn type-app [] 5)

;; === 型構築 ===

;; 整数型 (hash=100, 0 は map-get のデフォルト値と衝突するため)
(defn make-type-int []
  (vector-push (vector-push (vector-new 2) 1) 100))

;; Bool 型
(defn make-type-bool []
  (vector-push (vector-push (vector-new 2) 1) 200))

;; 文字列型
(defn make-type-string []
  (vector-push (vector-push (vector-new 2) 1) 300))

;; Float 型
(defn make-type-float []
  (vector-push (vector-push (vector-new 2) 1) 400))

;; Unit 型
(defn make-type-unit []
  (vector-push (vector-push (vector-new 2) 1) 500))

;; 型変数
(defn make-type-var [id]
  (vector-push (vector-push (vector-new 2) 2) id))

;; 関数型: param -> return
(defn make-type-fun [param-ty ret-ty]
  (do
    (root_push param-ty)
    (root_push ret-ty)
    (let [base (vector-new 3)
      base-slot (root_push base)
      with-tag (vector-push base 3)]
      (do
        (root_set base-slot with-tag)
        (let [with-param (vector-push with-tag param-ty)]
          (do
            (root_set base-slot with-param)
            (let [result (vector-push with-param ret-ty)]
              (do
                (root_pop)
                (root_pop)
                (root_pop)
                result))))))))

;; レコード型
(defn make-type-record [name-hash]
  (vector-push (vector-push (vector-new 8) 4) name-hash))

;; object を保持したまま Vector へ追加する。
(defn type-push-object [dst value]
  (do
    (root_push dst)
    (root_push value)
    (let [next-dst (vector-push dst value)]
      (do
        (root_pop)
        (root_pop)
        next-dst))))

;; 型適用の引数を左から複写する。
(defn type-app-append-args [args idx len out]
  (if (>= idx len)
    out
    (type-app-append-args
      args
      (+ idx 1)
      len
      (type-push-object out (vector-get args idx)))))

;; 型適用。例: Ref String = [5, Ref, 1, String]
(defn make-type-app [name-hash args]
  (let [arg-count (vector-length args)
    prefix
      (vector-push
        (vector-push
          (vector-push (vector-new (+ arg-count 3)) 5)
          name-hash)
        arg-count)]
    (type-app-append-args args 0 arg-count prefix)))

;; 1 引数型適用の頻出経路。
(defn make-type-app1 [name-hash arg]
  (type-push-object
    (vector-push
      (vector-push
        (vector-push (vector-new 4) 5)
        name-hash)
      1)
    arg))

;; レコード型にフィールドを追加
(defn type-record-add-field [ty field-name-hash field-ty]
  (vector-push (vector-push ty field-name-hash) field-ty))

;; レコード型からフィールド型を取得
(defn type-record-field-type [ty field-name-hash]
  (do
    (root_push ty)
    (let [result
            (type-record-field-type-rooted-v3
              ty field-name-hash 2 (vector-length ty))]
      (do
        (root_pop)
        result))))

(defn type-record-operation-state [done next-idx result]
  (do
    (root_push result)
    (let [base (vector-new 3)
          base-slot (root_push base)
          with-done (vector-push base done)]
      (do
        (root_set base-slot with-done)
        (let [with-idx (vector-push with-done next-idx)]
          (do
            (root_set base-slot with-idx)
            (let [state (vector-push with-idx result)]
              (do
                (root_pop)
                (root_pop)
                state))))))))

(defn type-record-field-type-state [done next-idx result]
  (type-record-operation-state done next-idx result))

(defn type-record-field-type-step-v3 [ty field-name-hash idx len]
  (if (>= idx len)
    (type-record-field-type-state 1 idx 0)
    (do
      (root_push ty)
      (let [state
              (if (= (vector-get ty idx) field-name-hash)
                (let [field-ty (vector-get ty (+ idx 1))]
                  (do
                    (root_push field-ty)
                    (let [result
                            (type-record-field-type-state 1 idx field-ty)]
                      (do
                        (root_pop)
                        result))))
                (type-record-field-type-state 0 (+ idx 2) 0))]
        (do
          (root_pop)
          state)))))

(defn type-record-field-type-step-64-loop-bounded
  [ty field-name-hash idx len remaining]
  (do
    (root_push ty)
    (let [step
            (type-record-field-type-step-v3 ty field-name-hash idx len)
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
                    (type-record-field-type-step-64-loop-bounded
                      ty
                      field-name-hash
                      next-idx
                      len
                      (- remaining 1))))]
          (do
            (root_pop)
            (root_pop)
            (root_pop)
            parsed))))))

(defn type-record-field-type-step-64 [ty field-name-hash idx len]
  (type-record-field-type-step-64-loop-bounded
    ty field-name-hash idx len 64))

(defn type-record-field-type-rooted-v3 [ty field-name-hash idx len]
  (let [step
          (type-record-field-type-step-64 ty field-name-hash idx len)]
    (if (= (vector-get step 0) 1)
      (vector-get step 2)
      (do
        (root_push step)
        (let [next-idx (vector-get step 1)
              next-result (vector-get step 2)]
          (do
            (root_push next-result)
            (let [resolved
                    (type-record-field-type-rooted-v3
                      ty field-name-hash next-idx len)]
              (do
                (root_pop)
                (root_pop)
                resolved))))))))

;; 関数型のパラメータ型を取得
(defn type-fun-param [ty]
  (vector-get ty 1))

;; 関数型の戻り値型を取得
(defn type-fun-ret [ty]
  (vector-get ty 2))

;; 型適用の名前・引数を取得
(defn type-app-name [ty]
  (vector-get ty 1))

(defn type-app-arg-count [ty]
  (vector-get ty 2))

(defn type-app-arg [ty idx]
  (vector-get ty (+ idx 3)))

;; === 型アクセス ===

;; 型のタグを取得
(defn type-tag [ty]
  (vector-get ty 0))

;; 型コンストラクタの場合、名前ハッシュを取得
(defn type-name [ty]
  (vector-get ty 1))

;; 型適用の引数を構造的に比較する。
(defn type-app-args-eq [ty1 ty2 idx len]
  (if (>= idx len)
    1
    (if (= (types-eq (type-app-arg ty1 idx) (type-app-arg ty2 idx)) 1)
      (type-app-args-eq ty1 ty2 (+ idx 1) len)
      0)))

;; record の field 名と field 型を declaration order で構造比較する。
(defn type-record-fields-eq [ty1 ty2 idx len]
  (do
    (root_push ty1)
    (root_push ty2)
    (let [result
            (type-record-fields-eq-rooted-v3 ty1 ty2 idx len)]
      (do
        (root_pop)
        (root_pop)
        result))))

(defn type-record-fields-eq-state [done next-idx result]
  (type-record-operation-state done next-idx result))

(defn type-record-fields-eq-step-v3 [ty1 ty2 idx len]
  (if (>= idx len)
    (type-record-fields-eq-state 1 idx 1)
    (do
      (root_push ty1)
      (root_push ty2)
      (let [state
              (if (= (vector-get ty1 idx) (vector-get ty2 idx))
                (if (= (types-eq
                          (vector-get ty1 (+ idx 1))
                          (vector-get ty2 (+ idx 1)))
                       1)
                  (type-record-fields-eq-state 0 (+ idx 2) 1)
                  (type-record-fields-eq-state 1 idx 0))
                (type-record-fields-eq-state 1 idx 0))]
        (do
          (root_pop)
          (root_pop)
          state)))))

(defn type-record-fields-eq-step-64-loop-bounded
  [ty1 ty2 idx len remaining]
  (do
    (root_push ty1)
    (root_push ty2)
    (let [step
            (type-record-fields-eq-step-v3 ty1 ty2 idx len)
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
                    (type-record-fields-eq-step-64-loop-bounded
                      ty1 ty2 next-idx len (- remaining 1))))]
          (do
            (root_pop)
            (root_pop)
            (root_pop)
            (root_pop)
            parsed))))))

(defn type-record-fields-eq-step-64 [ty1 ty2 idx len]
  (type-record-fields-eq-step-64-loop-bounded
    ty1 ty2 idx len 64))

(defn type-record-fields-eq-rooted-v3 [ty1 ty2 idx len]
  (let [step
          (type-record-fields-eq-step-64 ty1 ty2 idx len)]
    (if (= (vector-get step 0) 1)
      (vector-get step 2)
      (do
        (root_push step)
        (let [next-idx (vector-get step 1)
              next-result (vector-get step 2)]
          (do
            (root_push next-result)
            (let [resolved
                    (type-record-fields-eq-rooted-v3
                      ty1 ty2 next-idx len)]
              (do
                (root_pop)
                (root_pop)
                resolved))))))))

;; === Substitution ===

;; Substitution は HashMap<Int, Type> で表現
;; key = 型変数 ID, value = 型 (vector 参照)
;; map-contains? (Bool) は使わず、map-get (Int) で判定する

;; 空の置換
(defn subst-new []
  (map-new))

;; 型変数に型を割り当て
(defn subst-bind [s var-id ty]
  (map-insert s var-id ty))

;; 型変数を解決 (0 = 未束縛)
(defn subst-lookup [s var-id]
  (map-get s var-id))

;; === 型の等価判定 ===

;; 二つの型が構造的に等しいか判定 (1=等しい, 0=異なる)
(defn types-eq [ty1 ty2]
  (if (= (type-tag ty1) (type-tag ty2))
    (if (= (type-tag ty1) 1)
      ;; 両方 Con: 名前ハッシュを比較
      (if (= (type-name ty1) (type-name ty2)) 1 0)
      (if (= (type-tag ty1) 2)
        ;; 両方 Var: ID を比較
        (if (= (type-name ty1) (type-name ty2)) 1 0)
        (if (= (type-tag ty1) 3)
        ;; 両方 Fun: パラメータと戻り値をそれぞれ比較
          (if (= (types-eq (type-fun-param ty1) (type-fun-param ty2)) 1)
            (types-eq (type-fun-ret ty1) (type-fun-ret ty2))
            0)
          (if (= (type-tag ty1) 5)
            ;; 両方 App: constructor と全引数を比較
            (if (= (type-app-name ty1) (type-app-name ty2))
              (if (= (type-app-arg-count ty1) (type-app-arg-count ty2))
                (type-app-args-eq ty1 ty2 0 (type-app-arg-count ty1))
                0)
              0)
            (if (= (type-tag ty1) 4)
              ;; 両方 Record: name、field count、各 field を比較
              (if (= (type-name ty1) (type-name ty2))
                (if (= (vector-length ty1) (vector-length ty2))
                  (type-record-fields-eq ty1 ty2 2 (vector-length ty1))
                  0)
                0)
              0)))))
    0))

;; === apply-subst ===

;; 置換を型に適用
(defn apply-subst-app-args [subst ty idx len out]
  (if (>= idx len)
    out
    (apply-subst-app-args
      subst
      ty
      (+ idx 1)
      len
      (type-push-object out (apply-subst subst (type-app-arg ty idx))))))

;; Record の field type にも置換を再帰適用する。
(defn apply-subst-record-fields [subst ty idx len out]
  (do
    (root_push subst)
    (root_push ty)
    (root_push out)
    (let [result
            (apply-subst-record-fields-rooted-v3
              subst ty idx len out)]
      (do
        (root_pop)
        (root_pop)
        (root_pop)
        result))))

(defn apply-subst-record-fields-state [done next-idx out]
  (type-record-operation-state done next-idx out))

(defn apply-subst-record-fields-step-v3
  [subst ty idx len out]
  (if (>= idx len)
    (apply-subst-record-fields-state 1 idx out)
    (do
      (root_push subst)
      (root_push ty)
      (root_push out)
      (let [field-hash (vector-get ty idx)
            field-ty (vector-get ty (+ idx 1))]
        (do
          (root_push field-ty)
          (let [field-result (apply-subst subst field-ty)]
            (do
              (root_push field-result)
              (let [next-out
                      (type-record-add-field out field-hash field-result)]
                (do
                  (root_push next-out)
                  (let [state
                          (apply-subst-record-fields-state
                            0 (+ idx 2) next-out)]
                    (do
                      (root_pop)
                      (root_pop)
                      (root_pop)
                      (root_pop)
                      (root_pop)
                      (root_pop)
                      state)))))))))))

(defn apply-subst-record-fields-step-64-loop-bounded
  [subst ty idx len out remaining]
  (do
    (root_push subst)
    (root_push ty)
    (root_push out)
    (let [step
            (apply-subst-record-fields-step-v3 subst ty idx len out)
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
                    (apply-subst-record-fields-step-64-loop-bounded
                      subst ty next-idx len next-out (- remaining 1))))]
          (do
            (root_pop)
            (root_pop)
            (root_pop)
            (root_pop)
            (root_pop)
            parsed))))))

(defn apply-subst-record-fields-step-64
  [subst ty idx len out]
  (apply-subst-record-fields-step-64-loop-bounded
    subst ty idx len out 64))

(defn apply-subst-record-fields-rooted-v3
  [subst ty idx len out]
  (let [step
          (apply-subst-record-fields-step-64 subst ty idx len out)]
    (if (= (vector-get step 0) 1)
      (vector-get step 2)
      (do
        (root_push step)
        (let [next-idx (vector-get step 1)
              next-out (vector-get step 2)]
          (do
            (root_push next-out)
            (let [resolved
                    (apply-subst-record-fields-rooted-v3
                      subst ty next-idx len next-out)]
              (do
                (root_pop)
                (root_pop)
                resolved))))))))

(defn apply-subst-fun-rooted [subst ty]
  (do
    (root_push subst)
    (root_push ty)
    (let [param-resolved (apply-subst subst (type-fun-param ty))]
      (do
        (root_push param-resolved)
        (let [ret-resolved (apply-subst subst (type-fun-ret ty))]
          (do
            (root_push ret-resolved)
            (let [result (make-type-fun param-resolved ret-resolved)]
              (do
                (root_push result)
                (root_pop)
                (root_pop)
                (root_pop)
                (root_pop)
                (root_pop)
                result))))))))

(defn apply-subst [subst ty]
  (if (= (type-tag ty) 2)
    ;; Var: 置換に存在すれば再帰的に適用
    (let [looked (subst-lookup subst (type-name ty))]
      (if (= looked 0)
        ty
        (apply-subst subst looked)))
    (if (= (type-tag ty) 3)
      ;; Fun: パラメータと戻り値に適用
      (apply-subst-fun-rooted subst ty)
      (if (= (type-tag ty) 5)
        ;; App: すべての型引数に置換を適用
        (make-type-app
          (type-app-name ty)
          (apply-subst-app-args
            subst ty 0 (type-app-arg-count ty) (vector-new (type-app-arg-count ty))))
        (if (= (type-tag ty) 4)
          ;; Record: すべての field type に置換を適用
          (apply-subst-record-fields
            subst ty 2 (vector-length ty) (make-type-record (type-name ty)))
          ;; Con: そのまま返す
          ty)))))

;; === occurs-check ===

;; var-id が ty 内に出現するかチェック (1=出現, 0=非出現)
(defn occurs-check-app-args [var-id ty idx len]
  (if (>= idx len)
    0
    (if (= (occurs-check var-id (type-app-arg ty idx)) 1)
      1
      (occurs-check-app-args var-id ty (+ idx 1) len))))

;; record field 型のいずれかに型変数が出現するかを調べる。
(defn occurs-check-record-fields [var-id ty idx len]
  (do
    (root_push ty)
    (let [result
            (occurs-check-record-fields-rooted-v3 var-id ty idx len)]
      (do
        (root_pop)
        result))))

(defn occurs-check-record-fields-state [done next-idx result]
  (type-record-operation-state done next-idx result))

(defn occurs-check-record-fields-step-v3
  [var-id ty idx len]
  (if (>= idx len)
    (occurs-check-record-fields-state 1 idx 0)
    (do
      (root_push ty)
      (let [result
              (occurs-check var-id (vector-get ty (+ idx 1)))]
        (let [state
                (if (= result 1)
                  (occurs-check-record-fields-state 1 idx 1)
                  (occurs-check-record-fields-state 0 (+ idx 2) 0))]
          (do
            (root_pop)
            state))))))

(defn occurs-check-record-fields-step-64-loop-bounded
  [var-id ty idx len remaining]
  (do
    (root_push ty)
    (let [step
            (occurs-check-record-fields-step-v3 var-id ty idx len)
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
                    (occurs-check-record-fields-step-64-loop-bounded
                      var-id ty next-idx len (- remaining 1))))]
          (do
            (root_pop)
            (root_pop)
            (root_pop)
            parsed))))))

(defn occurs-check-record-fields-step-64 [var-id ty idx len]
  (occurs-check-record-fields-step-64-loop-bounded
    var-id ty idx len 64))

(defn occurs-check-record-fields-rooted-v3 [var-id ty idx len]
  (let [step
          (occurs-check-record-fields-step-64 var-id ty idx len)]
    (if (= (vector-get step 0) 1)
      (vector-get step 2)
      (do
        (root_push step)
        (let [next-idx (vector-get step 1)
              next-result (vector-get step 2)]
          (do
            (root_push next-result)
            (let [resolved
                    (occurs-check-record-fields-rooted-v3
                      var-id ty next-idx len)]
              (do
                (root_pop)
                (root_pop)
                resolved))))))))

(defn occurs-check [var-id ty]
  (if (= (type-tag ty) 2)
    ;; Var: ID が一致すれば出現
    (if (= var-id (type-name ty)) 1 0)
    (if (= (type-tag ty) 3)
      ;; Fun: パラメータまたは戻り値に出現
      (if (= (occurs-check var-id (type-fun-param ty)) 1)
        1
        (occurs-check var-id (type-fun-ret ty)))
      (if (= (type-tag ty) 5)
        ;; App: いずれかの型引数に出現するか
        (occurs-check-app-args var-id ty 0 (type-app-arg-count ty))
        (if (= (type-tag ty) 4)
          ;; Record: いずれかの field 型に出現するか
          (occurs-check-record-fields var-id ty 2 (vector-length ty))
          ;; Con: 出現しない
          0)))))

;; === Unification ===

;; エラーを示す置換 (key=-1 にマーカーを設定)
(defn unify-error []
  (map-insert (map-new) -1 1))

;; 置換がエラーかチェック (0=正常, 1=エラー)
(defn unify-failed [result]
  (map-get result -1))

;; 同じ型コンストラクタの型引数を左から単一化する。
(defn unify-app-args [ty1 ty2 idx len subst]
  (if (>= idx len)
    subst
    (let [next-subst (unify (type-app-arg ty1 idx) (type-app-arg ty2 idx) subst)]
      (if (= (unify-failed next-subst) 1)
        next-subst
        (unify-app-args ty1 ty2 (+ idx 1) len next-subst)))))

;; 同名 record の field 型を declaration order で単一化する。
(defn unify-record-fields [ty1 ty2 idx len subst]
  (do
    (root_push ty1)
    (root_push ty2)
    (root_push subst)
    (let [result
            (unify-record-fields-rooted-v3 ty1 ty2 idx len subst)]
      (do
        (root_pop)
        (root_pop)
        (root_pop)
        result))))

(defn unify-record-fields-state [done next-idx subst]
  (type-record-operation-state done next-idx subst))

(defn unify-record-fields-step-v3
  [ty1 ty2 idx len subst]
  (if (>= idx len)
    (unify-record-fields-state 1 idx subst)
    (do
      (root_push ty1)
      (root_push ty2)
      (root_push subst)
      (let [state
              (if (= (vector-get ty1 idx) (vector-get ty2 idx))
                (let [next-subst
                        (unify
                          (vector-get ty1 (+ idx 1))
                          (vector-get ty2 (+ idx 1))
                          subst)]
                  (do
                    (root_push next-subst)
                    (let [next-state
                            (if (= (unify-failed next-subst) 1)
                              (unify-record-fields-state 1 idx next-subst)
                              (unify-record-fields-state
                                0 (+ idx 2) next-subst))]
                      (do
                        (root_pop)
                        next-state))))
                (unify-record-fields-state 1 idx (unify-error)))]
        (do
          (root_pop)
          (root_pop)
          (root_pop)
          state)))))

(defn unify-record-fields-step-64-loop-bounded
  [ty1 ty2 idx len subst remaining]
  (do
    (root_push ty1)
    (root_push ty2)
    (root_push subst)
    (let [step
            (unify-record-fields-step-v3 ty1 ty2 idx len subst)
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
                    (unify-record-fields-step-64-loop-bounded
                      ty1 ty2 next-idx len next-subst (- remaining 1))))]
          (do
            (root_pop)
            (root_pop)
            (root_pop)
            (root_pop)
            (root_pop)
            parsed))))))

(defn unify-record-fields-step-64
  [ty1 ty2 idx len subst]
  (unify-record-fields-step-64-loop-bounded
    ty1 ty2 idx len subst 64))

(defn unify-record-fields-rooted-v3
  [ty1 ty2 idx len subst]
  (let [step
          (unify-record-fields-step-64 ty1 ty2 idx len subst)]
    (if (= (vector-get step 0) 1)
      (vector-get step 2)
      (do
        (root_push step)
        (let [next-idx (vector-get step 1)
              next-subst (vector-get step 2)]
          (do
            (root_push next-subst)
            (let [resolved
                    (unify-record-fields-rooted-v3
                      ty1 ty2 next-idx len next-subst)]
              (do
                (root_pop)
                (root_pop)
                resolved))))))))

(defn unify-record-types [ty1 ty2 subst]
  (if (= (type-tag ty2) 4)
    (if (= (type-name ty1) (type-name ty2))
      (if (= (vector-length ty1) (vector-length ty2))
        (unify-record-fields ty1 ty2 2 (vector-length ty1) subst)
        (unify-error))
      (unify-error))
    (unify-error)))

;; apply-subst 済みの型を単一化する。呼び出し側が ty1/ty2/subst を root する。
(defn unify-substituted [ty1 ty2 subst]
  (if (= (types-eq ty1 ty2) 1)
    ;; 同じ型なら置換をそのまま返す
    subst
    (if (= (type-tag ty1) 2)
      ;; ty1 が Var
      (if (= (occurs-check (type-name ty1) ty2) 1)
        (unify-error)
        (subst-bind subst (type-name ty1) ty2))
      (if (= (type-tag ty2) 2)
        ;; ty2 が Var
        (if (= (occurs-check (type-name ty2) ty1) 1)
          (unify-error)
          (subst-bind subst (type-name ty2) ty1))
        (if (= (type-tag ty1) 1)
          ;; 両方 Con: 名前が一致しないなら失敗
          (unify-error)
          (if (= (type-tag ty1) 3)
            ;; 両方 Fun: パラメータを単一化してから戻り値を単一化
            (if (= (type-tag ty2) 3)
              (let [s1 (unify (type-fun-param ty1) (type-fun-param ty2) subst)]
                (if (= (unify-failed s1) 0)
                  (unify (type-fun-ret ty1) (type-fun-ret ty2) s1)
                  (unify-error)))
              (unify-error))
            (if (= (type-tag ty1) 5)
              ;; 両方 App: constructor と arity が同じときだけ型引数を単一化
              (if (= (type-tag ty2) 5)
                (if (= (type-app-name ty1) (type-app-name ty2))
                  (if (= (type-app-arg-count ty1) (type-app-arg-count ty2))
                    (unify-app-args ty1 ty2 0 (type-app-arg-count ty1) subst)
                    (unify-error))
                  (unify-error))
                (unify-error))
              (if (= (type-tag ty1) 4)
                ;; 両方 Record: name、field count、各 field 型を単一化
                (unify-record-types ty1 ty2 subst)
                (unify-error)))))))))

;; 二つの型を単一化し、更新された置換を返す
;; 成功時: 置換 (map), 失敗時: エラーマーカー付き map
(defn unify [t1 t2 subst]
  (do
    (root_push t1)
    (root_push t2)
    (root_push subst)
    (let [ty1 (apply-subst subst t1)]
      (do
        (root_push ty1)
        (let [ty2 (apply-subst subst t2)]
          (do
            (root_push ty2)
            (let [result (unify-substituted ty1 ty2 subst)]
              (do
                (root_push result)
                (root_pop)
                (root_pop)
                (root_pop)
                (root_pop)
                (root_pop)
                (root_pop)
                result))))))))

;; エントリポイント (テスト用)
(defn main []
  (let [int-ty (make-type-int)
    var-ty (make-type-var 42)
    s (subst-new)
    s1 (subst-bind s 42 int-ty)]
    (do
      (print (type-tag int-ty)) ;; 1 (Con)
      (print (type-tag var-ty)) ;; 2 (Var)
      (print (type-name var-ty)) ;; 42
      (print (type-tag (subst-lookup s1 42))) ;; 1 (Con)
      0)))
