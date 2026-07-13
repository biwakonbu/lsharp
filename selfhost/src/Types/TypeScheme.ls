(module Types.TypeScheme)
(import Types.Type)

;; TypeScheme.ls - L# セルフホスティング: 型スキーム (let 多相)
;;
;; let 多相で必要な型スキーム (∀α.τ) の表現。
;; 型変数の一般化 (generalize) と具体化 (instantiate) を提供。

;; === 型スキーム ===

;; TypeScheme = [type, bound-vars-vector]
;; bound-vars-vector: 束縛された型変数 ID のベクタ (空なら単相型)

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

;; 単相型スキーム: 束縛変数なし
(defn mono [ty]
  (push-object-vector-local (push-object-vector-local (vector-new 2) ty) (vector-new 0)))

;; 多相型スキーム: 束縛変数あり
(defn poly [ty bound-vars]
  (push-object-vector-local (push-object-vector-local (vector-new 2) ty) bound-vars))

;; 型スキームの型を取得
(defn scheme-type [scheme]
  (vector-get scheme 0))

;; 型スキームの束縛変数を取得
(defn scheme-vars [scheme]
  (vector-get scheme 1))

;; === 型変数カウンタ ===
;; 新しい型変数を生成するためのグローバルカウンタ

;; 型別名環境 = [closed-aliases, parametric-aliases]
(defn make-type-alias-env [closed-aliases parametric-aliases]
  (do
    (root_push closed-aliases)
    (root_push parametric-aliases)
    (let [with-closed (push-object-vector-local (vector-new 2) closed-aliases)]
      (do
        (root_push with-closed)
        (let [result (push-object-vector-local with-closed parametric-aliases)]
          (do
            (root_pop)
            (root_pop)
            (root_pop)
            result))))))

(defn type-alias-env-closed [alias-env]
  (vector-get alias-env 0))

(defn type-alias-env-parametric [alias-env]
  (vector-get alias-env 1))

;; 型推論 context = [next-id-ref, alias-env]。型変数 ID の API を保ったまま宣言環境を共有する。
(defn make-var-counter-with-alias-env [alias-env]
  (do
    (root_push alias-env)
    (let [id-ref (ref-new 1000)]
      (do
        (root_push id-ref)
        (let [with-id-ref (push-object-vector-local (vector-new 2) id-ref)]
          (do
            (root_push with-id-ref)
            (let [result (push-object-vector-local with-id-ref alias-env)]
              (do
                (root_pop)
                (root_pop)
                (root_pop)
                result))))))))

(defn make-var-counter []
  (make-var-counter-with-alias-env (make-type-alias-env (map-new) (map-new))))

(defn var-counter-id-ref [counter]
  (vector-get counter 0))

(defn var-counter-alias-env [counter]
  (vector-get counter 1))

;; record 宣言環境を追加した推論 context を構築する。
;; 既存の [next-id-ref, alias-env] 読み取りは index 0/1 のまま互換に保つ。
(defn var-counter-with-alias-env-and-record-env [counter alias-env record-env]
  (let [id-ref (var-counter-id-ref counter)]
    (do
      (root_push id-ref)
      (root_push alias-env)
      (root_push record-env)
      (let [with-id-ref (push-object-vector-local (vector-new 3) id-ref)]
        (do
          (root_push with-id-ref)
          (let [with-alias-env (push-object-vector-local with-id-ref alias-env)]
            (do
              (root_push with-alias-env)
              (let [result (push-object-vector-local with-alias-env record-env)]
                (do
                  (root_pop)
                  (root_pop)
                  (root_pop)
                  (root_pop)
                  (root_pop)
                  result)))))))))

;; 旧形式の counter にも空の record 環境を返す。
(defn var-counter-record-env [counter]
  (if (> (vector-length counter) 2)
    (vector-get counter 2)
    (map-new)))

;; 既存の型変数 ID 供給を保ったまま、宣言 prepass 後の alias 環境へ差し替える。
(defn var-counter-with-alias-env [counter alias-env]
  (let [id-ref (var-counter-id-ref counter)]
    (do
      (root_push id-ref)
      (root_push alias-env)
      (let [with-id-ref (push-object-vector-local (vector-new 2) id-ref)]
        (do
          (root_push with-id-ref)
          (let [result (push-object-vector-local with-id-ref alias-env)]
            (do
              (root_pop)
              (root_pop)
              (root_pop)
              result)))))))

;; 次の型変数 ID を生成
(defn next-var [counter]
  (let [id-ref (var-counter-id-ref counter)
    id (ref-get id-ref)]
    (do
      (ref-set id-ref (+ id 1))
      id)))

;; 束縛変数ベクタを左から順に fresh な型変数へ写す
(defn instantiate-build-subst [vars idx len counter subst]
  (if (>= idx len)
    subst
    (let [old-var (vector-get vars idx)
      new-ty (make-type-var (next-var counter))]
      (instantiate-build-subst
        vars
        (+ idx 1)
        len
        counter
        (map-insert-object-safe subst old-var new-ty)))))

;; record 型の field type に置換を適用する
(defn instantiate-apply-record-fields [subst ty idx len out]
  (if (>= idx len)
    out
    (let [field-hash (vector-get ty idx)
      field-ty (vector-get ty (+ idx 1))]
      (instantiate-apply-record-fields
        subst
        ty
        (+ idx 2)
        len
        (type-record-add-field out field-hash (instantiate-apply subst field-ty))))))

;; 型適用の型引数へ具体化用置換を適用する。
(defn instantiate-apply-app-args [subst ty idx len out]
  (if (>= idx len)
    out
    (instantiate-apply-app-args
      subst
      ty
      (+ idx 1)
      len
      (push-object-vector-local out (instantiate-apply subst (type-app-arg ty idx))))))

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
      ;; 多相型: 各束縛変数を source order のまま新しい型変数に置き換え
      (instantiate-apply
        (instantiate-build-subst vars 0 n counter (map-new))
        ty))))

;; 置換を型に適用 (instantiate 用)
(defn instantiate-apply [subst ty]
  (let [tag (vector-get ty 0)]
    (if (= tag 2)
      ;; Var: 置換に存在すれば置き換え
      (let [looked (map-get-safe subst (vector-get ty 1))]
        (if (= looked 0)
          ty
          looked))
      (if (= tag 3)
        ;; Fun: パラメータと戻り値に適用
        (make-type-fun
          (instantiate-apply subst (vector-get ty 1))
          (instantiate-apply subst (vector-get ty 2)))
        (if (= tag 4)
          ;; Record: field type ごとに置換を適用
          (instantiate-apply-record-fields
            subst
            ty
            2
            (vector-length ty)
            (make-type-record (vector-get ty 1)))
          (if (= tag 5)
            ;; App: 型引数ごとに置換を適用
            (make-type-app
              (type-app-name ty)
              (instantiate-apply-app-args
                subst ty 0 (type-app-arg-count ty) (vector-new (type-app-arg-count ty))))
            ;; Con: そのまま
            ty))))))

;; === generalize ===

;; vars に target が含まれるか
(defn free-vars-contains [vars idx len target]
  (if (>= idx len)
    0
    (if (= (vector-get vars idx) target)
      1
      (free-vars-contains vars (+ idx 1) len target))))

;; source order を維持しつつ、未出現の型変数だけを追加
(defn free-vars-push-unique [vars target]
  (if (= (free-vars-contains vars 0 (vector-length vars) target) 1)
    vars
    (push-int-vector-local vars target)))

;; src を左から順に dst へマージし、自由変数順を安定化する
(defn free-vars-append-unique [dst src idx len]
  (if (>= idx len)
    dst
    (free-vars-append-unique
      (free-vars-push-unique dst (vector-get src idx))
      src
      (+ idx 1)
      len)))

;; record 型の field type を左から走査し、自由変数順を安定化する
(defn free-vars-record-fields [ty idx len acc]
  (if (>= idx len)
    acc
    (let [field-vars (free-vars (vector-get ty (+ idx 1)))]
      (free-vars-record-fields
        ty
        (+ idx 2)
        len
        (free-vars-append-unique acc field-vars 0 (vector-length field-vars))))))

;; 型適用の型引数を左から走査し、自由変数順を安定化する。
(defn free-vars-app-args [ty idx len acc]
  (if (>= idx len)
    acc
    (let [arg-vars (free-vars (type-app-arg ty idx))]
      (free-vars-app-args
        ty
        (+ idx 1)
        len
        (free-vars-append-unique acc arg-vars 0 (vector-length arg-vars))))))

;; 環境にない自由変数だけを source order のまま束縛変数へ積む
(defn generalize-collect-bound [free idx len env-vars bound]
  (if (>= idx len)
    bound
    (let [v (vector-get free idx)
      next-bound
      (if (= (map-get-safe env-vars v) 0)
        (push-int-vector-local bound v)
        bound)]
      (generalize-collect-bound free (+ idx 1) len env-vars next-bound))))

;; 型を一般化: 環境に出現しない自由変数を束縛
;; env-vars: 環境内の自由変数 ID の Set (map で代用)
;; 戻り値: TypeScheme
(defn generalize [ty env-vars]
  (let [free (free-vars ty)
    bound
    (generalize-collect-bound
      free 0 (vector-length free) env-vars (vector-new (vector-length free)))]
    (poly ty bound)))

;; 型の自由変数を収集
(defn free-vars [ty]
  (let [tag (vector-get ty 0)]
    (if (= tag 2)
      ;; Var: その ID を返す
      (push-int-vector-local (vector-new 1) (vector-get ty 1))
      (if (= tag 3)
        ;; Fun: パラメータと戻り値の自由変数を結合
        (let [pv (free-vars (vector-get ty 1))
          rv (free-vars (vector-get ty 2))]
          (free-vars-append-unique pv rv 0 (vector-length rv)))
        (if (= tag 4)
          ;; Record: field type を左から走査
          (free-vars-record-fields ty 2 (vector-length ty) (vector-new 4))
          (if (= tag 5)
            ;; App: 型引数を左から走査
            (free-vars-app-args ty 0 (type-app-arg-count ty) (vector-new (type-app-arg-count ty)))
            ;; Con: 自由変数なし
            (vector-new 0)))))))

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
      (print (vector-get inst1 0)) ;; 1 (Con)
      (print (vector-get inst1 1)) ;; 100 (Int hash)

      ;; 多相の instantiate: 型変数が新しい ID に
      (print (vector-get inst2 0)) ;; 3 (Fun)
      ;; パラメータと戻り値は新しい型変数 (ID=1000)
      (let [param (vector-get inst2 1)]
        (do
          (print (vector-get param 0)) ;; 2 (Var)
          (print (vector-get param 1)))) ;; 1000

      ;; free-vars テスト
      (print (vector-length (free-vars int-ty))) ;; 0
      (print (vector-length (free-vars var-a))) ;; 1
      (print (vector-get (free-vars var-a) 0)) ;; 1

      0)))
