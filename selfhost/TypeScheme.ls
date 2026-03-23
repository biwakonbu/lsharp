;; TypeScheme.ls - L# セルフホスティング: 型スキーム (let 多相)
;;
;; let 多相で必要な型スキーム (∀α.τ) の表現。
;; 型変数の一般化 (generalize) と具体化 (instantiate) を提供。

;; === 型スキーム ===

;; TypeScheme = [type, bound-vars-vector]
;; bound-vars-vector: 束縛された型変数 ID のベクタ (空なら単相型)

;; 単相型スキーム: 束縛変数なし
(defn mono [ty]
  (vector-push (vector-push (vector-new 2) ty) (vector-new 0)))

;; 多相型スキーム: 束縛変数あり
(defn poly [ty bound-vars]
  (vector-push (vector-push (vector-new 2) ty) bound-vars))

;; 型スキームの型を取得
(defn scheme-type [scheme]
  (vector-get scheme 0))

;; 型スキームの束縛変数を取得
(defn scheme-vars [scheme]
  (vector-get scheme 1))

;; === 型変数カウンタ ===
;; 新しい型変数を生成するためのグローバルカウンタ

;; カウンタ (ref-cell)
(defn make-var-counter []
  (ref-new 1000))

;; 次の型変数 ID を生成
(defn next-var [counter]
  (let [id (ref-get counter)]
    (do
      (ref-set counter (+ id 1))
      id)))

;; === instantiate ===

;; 型スキームを具体化: 束縛変数を新しい型変数で置換
;; counter: 変数カウンタ (ref-cell)
;; 戻り値: 具体化された型
(defn instantiate [scheme counter]
  (let [ty (scheme-type scheme)
        vars (scheme-vars scheme)
        n (vector-length vars)]
    (if (= n 0)
      ;; 単相型: そのまま返す
      ty
      ;; 多相型: 各束縛変数を新しい型変数に置き換え
      (let [subst (ref-new (map-new))
            i (ref-new 0)]
        (do
          ;; 置換マッピングを構築 (最大 4 変数)
          (if (< (ref-get i) n)
            (do
              (let [old-var (vector-get vars (ref-get i))
                    new-var (next-var counter)
                    new-ty (vector-push (vector-push (vector-new 2) 2) new-var)]
                (ref-set subst (map-insert (ref-get subst) old-var new-ty)))
              (ref-set i (+ (ref-get i) 1))
              (if (< (ref-get i) n)
                (do
                  (let [old-var (vector-get vars (ref-get i))
                        new-var (next-var counter)
                        new-ty (vector-push (vector-push (vector-new 2) 2) new-var)]
                    (ref-set subst (map-insert (ref-get subst) old-var new-ty)))
                  (ref-set i (+ (ref-get i) 1))
                  0)
                0))
            0)
          ;; apply-subst 相当: 置換を型に適用
          (instantiate-apply (ref-get subst) ty))))))

;; 置換を型に適用 (instantiate 用)
(defn instantiate-apply [subst ty]
  (let [tag (vector-get ty 0)]
    (if (= tag 2)
      ;; Var: 置換に存在すれば置き換え
      (let [looked (map-get subst (vector-get ty 1))]
        (if (= looked 0)
          ty
          looked))
      (if (= tag 3)
        ;; Fun: パラメータと戻り値に適用
        (vector-push
          (vector-push
            (vector-push (vector-new 3) 3)
            (instantiate-apply subst (vector-get ty 1)))
          (instantiate-apply subst (vector-get ty 2)))
        ;; Con: そのまま
        ty))))

;; === generalize ===

;; 型を一般化: 環境に出現しない自由変数を束縛
;; env-vars: 環境内の自由変数 ID の Set (map で代用)
;; 戻り値: TypeScheme
(defn generalize [ty env-vars]
  (let [free (free-vars ty)
        bound (ref-new (vector-new 4))
        i (ref-new 0)
        n (vector-length free)]
    (do
      ;; 環境にない自由変数を束縛変数として収集
      (if (< (ref-get i) n)
        (do
          (let [v (vector-get free (ref-get i))]
            (if (= (map-get env-vars v) 0)
              (ref-set bound (vector-push (ref-get bound) v))
              0))
          (ref-set i (+ (ref-get i) 1))
          (if (< (ref-get i) n)
            (do
              (let [v (vector-get free (ref-get i))]
                (if (= (map-get env-vars v) 0)
                  (ref-set bound (vector-push (ref-get bound) v))
                  0))
              (ref-set i (+ (ref-get i) 1))
              0)
            0))
        0)
      (poly ty (ref-get bound)))))

;; 型の自由変数を収集
(defn free-vars [ty]
  (let [tag (vector-get ty 0)]
    (if (= tag 2)
      ;; Var: その ID を返す
      (vector-push (vector-new 1) (vector-get ty 1))
      (if (= tag 3)
        ;; Fun: パラメータと戻り値の自由変数を結合
        (let [pv (free-vars (vector-get ty 1))
              rv (free-vars (vector-get ty 2))
              result (ref-new pv)
              j (ref-new 0)
              m (vector-length rv)]
          (do
            (if (< (ref-get j) m)
              (do
                (ref-set result (vector-push (ref-get result) (vector-get rv (ref-get j))))
                (ref-set j (+ (ref-get j) 1))
                (if (< (ref-get j) m)
                  (do
                    (ref-set result (vector-push (ref-get result) (vector-get rv (ref-get j))))
                    (ref-set j (+ (ref-get j) 1))
                    0)
                  0))
              0)
            (ref-get result)))
        ;; Con: 自由変数なし
        (vector-new 0)))))

;; === エントリポイント (テスト用) ===

(defn main []
  (let [;; 単相型スキーム
        int-ty (vector-push (vector-push (vector-new 2) 1) 100)
        int-scheme (mono int-ty)

        ;; 多相型スキーム: ∀a. a -> a
        var-a (vector-push (vector-push (vector-new 2) 2) 1)
        fun-ty (vector-push (vector-push (vector-push (vector-new 3) 3) var-a) var-a)
        bound (vector-push (vector-new 1) 1)
        id-scheme (poly fun-ty bound)

        ;; instantiate テスト
        counter (make-var-counter)
        inst1 (instantiate int-scheme counter)
        inst2 (instantiate id-scheme counter)]
    (do
      ;; 単相の instantiate: そのまま返る
      (print (vector-get inst1 0))  ;; 1 (Con)
      (print (vector-get inst1 1))  ;; 100 (Int hash)

      ;; 多相の instantiate: 型変数が新しい ID に
      (print (vector-get inst2 0))  ;; 3 (Fun)
      ;; パラメータと戻り値は新しい型変数 (ID=1000)
      (let [param (vector-get inst2 1)]
        (do
          (print (vector-get param 0))  ;; 2 (Var)
          (print (vector-get param 1))))  ;; 1000

      ;; free-vars テスト
      (print (vector-length (free-vars int-ty)))    ;; 0
      (print (vector-length (free-vars var-a)))     ;; 1
      (print (vector-get (free-vars var-a) 0))      ;; 1

      0)))
