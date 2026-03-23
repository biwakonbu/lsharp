;; Type.ls - L# セルフホスティング: 型定義
;;
;; Hindley-Milner 型推論で使用する型の表現。
;; 整数タグ + Vector でエンコード。

;; === 型種別 ===

;; 型コンストラクタ: [1, name-hash]
(defn type-con [] 1)

;; 型変数: [2, id]
(defn type-var [] 2)

;; 関数型: [3, param-count, param1, param2, ..., return-type]
(defn type-fun [] 3)

;; === 型構築 ===

;; 整数型
(defn make-type-int []
  (vector-push (vector-push (vector-new 2) 1) 0))  ;; Con("Int") hash=0

;; Bool 型
(defn make-type-bool []
  (vector-push (vector-push (vector-new 2) 1) 1))  ;; Con("Bool") hash=1

;; 文字列型
(defn make-type-string []
  (vector-push (vector-push (vector-new 2) 1) 2))  ;; Con("String") hash=2

;; 型変数
(defn make-type-var [id]
  (vector-push (vector-push (vector-new 2) 2) id))

;; === 型アクセス ===

;; 型のタグを取得
(defn type-tag [ty]
  (vector-get ty 0))

;; 型コンストラクタの場合、名前ハッシュを取得
(defn type-name [ty]
  (vector-get ty 1))

;; === Substitution ===

;; Substitution は HashMap<Int, Type> で表現
;; key = 型変数 ID, value = 解決された型のタグ (簡略化)

;; 空の置換
(defn subst-new []
  (map-new))

;; 型変数に型を割り当て
(defn subst-bind [s var-id ty-tag]
  (map-insert s var-id ty-tag))

;; 型変数を解決
(defn subst-lookup [s var-id]
  (map-get s var-id))

;; === Unification (簡略版) ===

;; 二つの型が一致するか判定 (簡略版: タグのみ比較)
(defn types-eq [ty1 ty2]
  (== (type-tag ty1) (type-tag ty2)))

;; エントリポイント (テスト用)
(defn main []
  (let [int-ty (make-type-int)
        bool-ty (make-type-bool)
        var-ty (make-type-var 42)
        s (subst-new)
        s1 (subst-bind s 42 0)]
    (do
      (print (type-tag int-ty))      ;; 1 (Con)
      (print (type-tag var-ty))      ;; 2 (Var)
      (print (type-name var-ty))     ;; 42
      (print (subst-lookup s1 42))   ;; 0 (Int hash)
      0)))
