(module Type)

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
  (vector-push (vector-push (vector-push (vector-new 3) 3) param-ty) ret-ty))

;; レコード型
(defn make-type-record [name-hash]
  (vector-push (vector-push (vector-new 8) 4) name-hash))

;; レコード型にフィールドを追加
(defn type-record-add-field [ty field-name-hash field-ty]
  (vector-push (vector-push ty field-name-hash) field-ty))

;; レコード型からフィールド型を取得
(defn type-record-field-type [ty field-name-hash]
  (type-record-field-type-loop ty field-name-hash 2 (vector-length ty)))

(defn type-record-field-type-loop [ty field-name-hash idx len]
  (if (>= idx len)
    0
    (if (= (vector-get ty idx) field-name-hash)
      (vector-get ty (+ idx 1))
      (type-record-field-type-loop ty field-name-hash (+ idx 2) len))))

;; 関数型のパラメータ型を取得
(defn type-fun-param [ty]
  (vector-get ty 1))

;; 関数型の戻り値型を取得
(defn type-fun-ret [ty]
  (vector-get ty 2))

;; === 型アクセス ===

;; 型のタグを取得
(defn type-tag [ty]
  (vector-get ty 0))

;; 型コンストラクタの場合、名前ハッシュを取得
(defn type-name [ty]
  (vector-get ty 1))

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
          0)))
    0))

;; === apply-subst ===

;; 置換を型に適用
(defn apply-subst [subst ty]
  (if (= (type-tag ty) 2)
    ;; Var: 置換に存在すれば再帰的に適用
    (let [looked (subst-lookup subst (type-name ty))]
      (if (= looked 0)
        ty
        (apply-subst subst looked)))
    (if (= (type-tag ty) 3)
      ;; Fun: パラメータと戻り値に適用
      (make-type-fun
        (apply-subst subst (type-fun-param ty))
        (apply-subst subst (type-fun-ret ty)))
      ;; Con: そのまま返す
      ty)))

;; === occurs-check ===

;; var-id が ty 内に出現するかチェック (1=出現, 0=非出現)
(defn occurs-check [var-id ty]
  (if (= (type-tag ty) 2)
    ;; Var: ID が一致すれば出現
    (if (= var-id (type-name ty)) 1 0)
    (if (= (type-tag ty) 3)
      ;; Fun: パラメータまたは戻り値に出現
      (if (= (occurs-check var-id (type-fun-param ty)) 1)
        1
        (occurs-check var-id (type-fun-ret ty)))
      ;; Con: 出現しない
      0)))

;; === Unification ===

;; エラーを示す置換 (key=-1 にマーカーを設定)
(defn unify-error []
  (map-insert (map-new) -1 1))

;; 置換がエラーかチェック (0=正常, 1=エラー)
(defn unify-failed [result]
  (map-get result -1))

;; 二つの型を単一化し、更新された置換を返す
;; 成功時: 置換 (map), 失敗時: エラーマーカー付き map
(defn unify [t1 t2 subst]
  (let [ty1 (apply-subst subst t1)
        ty2 (apply-subst subst t2)]
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
              (unify-error))))))))

;; エントリポイント (テスト用)
(defn main []
  (let [int-ty (make-type-int)
        var-ty (make-type-var 42)
        s (subst-new)
        s1 (subst-bind s 42 int-ty)]
    (do
      (print (type-tag int-ty))      ;; 1 (Con)
      (print (type-tag var-ty))      ;; 2 (Var)
      (print (type-name var-ty))     ;; 42
      (print (type-tag (subst-lookup s1 42)))  ;; 1 (Con)
      0)))
