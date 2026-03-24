;; TypeInfer.ls - L# セルフホスティング: Hindley-Milner 型推論
;;
;; HM 型推論コアの実装。
;; Type.ls の unify/apply-subst/occurs-check を利用。
;; TypeScheme.ls の instantiate/generalize/next-var を利用。
;;
;; 移植元: crates/lsharp-types/src/infer.rs
;;
;; 型タグ定数 (Type.ls から再定義):
;;   1=Con, 2=Var, 3=Fun
;; AST タグ定数:
;;   1=lit-int, 2=lit-bool, 3=lit-string, 4=var, 5=apply
;;   6=if, 7=let, 8=lambda, 9=do, 10=match, 20=defn

;; === 型タグ定数 ===
(defn ti-type-con [] 1)
(defn ti-type-var [] 2)
(defn ti-type-fun [] 3)

;; === 基本型のハッシュ定数 ===
(defn ti-hash-int [] 100)   ;; Int 型ハッシュ (Type.ls と一致)
(defn ti-hash-bool [] 200)  ;; Bool 型ハッシュ
(defn ti-hash-unit [] 400)  ;; Unit 型ハッシュ

;; === AST タグ定数 ===
(defn ti-ast-lit-int [] 1)
(defn ti-ast-lit-bool [] 2)
(defn ti-ast-lit-string [] 3)
(defn ti-ast-var [] 4)
(defn ti-ast-apply [] 5)
(defn ti-ast-if [] 6)
(defn ti-ast-let [] 7)
(defn ti-ast-lambda [] 8)
(defn ti-ast-do [] 9)

;; === 型コンストラクタ ===

;; Int 型
(defn ti-make-int []
  (let [v (vector-new 2)]
    (do
      (vector-push v 1)
      (vector-push v 100)
      v)))

;; Bool 型
(defn ti-make-bool []
  (let [v (vector-new 2)]
    (do
      (vector-push v 1)
      (vector-push v 200)
      v)))

;; Unit 型
(defn ti-make-unit []
  (let [v (vector-new 2)]
    (do
      (vector-push v 1)
      (vector-push v 400)
      v)))

;; 型変数
(defn ti-make-var [id]
  (let [v (vector-new 2)]
    (do
      (vector-push v 2)
      (vector-push v id)
      v)))

;; 関数型: param -> ret
(defn ti-make-fun [param-ty ret-ty]
  (let [v (vector-new 3)]
    (do
      (vector-push v 3)
      (vector-push v param-ty)
      (vector-push v ret-ty)
      v)))

;; === 型のアクセサ ===
(defn ti-type-tag [ty]
  (vector-get ty 0))

(defn ti-type-name [ty]
  (vector-get ty 1))

(defn ti-type-fun-param [ty]
  (vector-get ty 1))

(defn ti-type-fun-ret [ty]
  (vector-get ty 2))

;; === 型変数カウンタ ===

;; 型変数カウンタの初期化
(defn ti-var-counter-new []
  (ref-new 1000))

;; 新鮮な型変数 ID を生成
(defn ti-fresh-var-id [counter]
  (let [id (ref-get counter)]
    (do
      (ref-set counter (+ id 1))
      id)))

;; 新鮮な型変数を生成
(defn ti-fresh-var [counter]
  (ti-make-var (ti-fresh-var-id counter)))

;; === 型環境 (型スキームを格納する連想リスト) ===

;; 型環境の初期化 (map: name-hash -> type-scheme)
;; type-scheme: [ty, bound-vars-count, bound-var1, ...]
(defn ti-env-new []
  (map-new))

;; 型環境に単相型をバインド
(defn ti-env-bind-mono [env name-hash ty]
  (let [scheme (vector-new 4)]
    (do
      (vector-push scheme ty)
      (vector-push scheme 0)
      (map-insert env name-hash scheme))))

;; 型環境に型スキームをバインド (generalize 後)
(defn ti-env-bind-scheme [env name-hash scheme]
  (map-insert env name-hash scheme))

;; 型環境から型スキームを検索 (0 = 未定義)
(defn ti-env-lookup [env name-hash]
  (map-get env name-hash))

;; === 代入環境 (型変数 ID -> 型) ===

(defn ti-subst-new []
  (map-new))

;; 型変数を解決 (0 = 未束縛)
(defn ti-subst-lookup [subst var-id]
  (map-get subst var-id))

;; 代入を型に適用 (末尾再帰ガード付き)
(defn ti-apply-subst [subst ty]
  (let [tag (vector-get ty 0)]
    (if (= tag 2)
      ;; Var: 置換に存在すれば再帰的に適用
      (let [looked (ti-subst-lookup subst (ti-type-name ty))]
        (if (= looked 0)
          ty
          (ti-apply-subst subst looked)))
      (if (= tag 3)
        ;; Fun: パラメータと戻り値に適用
        (ti-make-fun
          (ti-apply-subst subst (ti-type-fun-param ty))
          (ti-apply-subst subst (ti-type-fun-ret ty)))
        ;; Con: そのまま返す
        ty))))

;; 代入を拡張する
(defn ti-subst-extend [subst var-id ty]
  (map-insert subst var-id ty))

;; === occurs check ===

;; var-id が ty に出現するか確認
(defn ti-occurs-check [var-id ty]
  (let [tag (vector-get ty 0)]
    (if (= tag 2)
      (if (= var-id (ti-type-name ty)) 1 0)
      (if (= tag 3)
        (let [r1 (ti-occurs-check var-id (ti-type-fun-param ty))]
          (if (= r1 1) 1
            (ti-occurs-check var-id (ti-type-fun-ret ty))))
        0))))

;; === 型の等価判定 ===
(defn ti-types-eq [ty1 ty2]
  (let [tag1 (ti-type-tag ty1)
        tag2 (ti-type-tag ty2)]
    (if (= tag1 tag2)
      (if (= tag1 1)
        (if (= (ti-type-name ty1) (ti-type-name ty2)) 1 0)
        (if (= tag1 2)
          (if (= (ti-type-name ty1) (ti-type-name ty2)) 1 0)
          0))
      0)))

;; === エラー置換 ===
;; エラーを示すマーカー付き代入 (key=-1 に 1 を設定)
(defn ti-unify-error []
  (map-insert (map-new) -1 1))

(defn ti-unify-failed [subst]
  (map-get subst -1))

;; === 単一化 ===

;; 2 つの型を単一化する
;; 成功時: 拡張された代入環境
;; 失敗時: エラーマーカー付き代入
(defn ti-unify [ty1 ty2 subst]
  (let [t1 (ti-apply-subst subst ty1)
        t2 (ti-apply-subst subst ty2)]
    (if (= (ti-types-eq t1 t2) 1)
      subst
      (let [tag1 (ti-type-tag t1)
            tag2 (ti-type-tag t2)]
        (if (= tag1 2)
          ;; ty1 が型変数
          (let [var-id (ti-type-name t1)]
            (if (= (ti-occurs-check var-id t2) 1)
              (ti-unify-error)
              (ti-subst-extend subst var-id t2)))
          (if (= tag2 2)
            ;; ty2 が型変数
            (let [var-id (ti-type-name t2)]
              (if (= (ti-occurs-check var-id t1) 1)
                (ti-unify-error)
                (ti-subst-extend subst var-id t1)))
            ;; 両方が型コンストラクタまたは関数型
            (if (= tag1 3)
              (if (= tag2 3)
                ;; 両方 Fun: パラメータ・戻り値を順に単一化
                (let [s1 (ti-unify (ti-type-fun-param t1) (ti-type-fun-param t2) subst)]
                  (if (= (ti-unify-failed s1) 0)
                    (ti-unify (ti-type-fun-ret t1) (ti-type-fun-ret t2) s1)
                    (ti-unify-error)))
                (ti-unify-error))
              (ti-unify-error))))))))

;; === instantiate (TypeScheme.ls から転用) ===

;; 型スキームを具体化: 束縛変数を新鮮な型変数に置換
;; scheme = [ty, bound-count, bound-var1, bound-var2, ...]
(defn ti-instantiate [scheme counter]
  (let [ty (vector-get scheme 0)
        bound-count (vector-get scheme 1)]
    (if (= bound-count 0)
      ty
      ;; bound-count > 0 の場合: 束縛変数を新鮮な型変数に置換
      ;; v1: 最大 2 束縛変数まで対応
      (let [subst (ref-new (map-new))]
        (do
          (let [old-var (vector-get scheme 2)
                new-id (ti-fresh-var-id counter)]
            (ref-set subst (map-insert (ref-get subst) old-var (ti-make-var new-id))))
          (if (> bound-count 1)
            (let [old-var2 (vector-get scheme 3)
                  new-id2 (ti-fresh-var-id counter)]
              (ref-set subst (map-insert (ref-get subst) old-var2 (ti-make-var new-id2))))
            0)
          ;; 置換を型に適用
          (ti-apply-subst (ref-get subst) ty))))))

;; === 型の自由変数収集 ===
(defn ti-free-vars-rec [ty seen result]
  (let [tag (ti-type-tag ty)]
    (if (= tag 2)
      ;; Var: ID を追加
      (let [id (ti-type-name ty)
            already (map-get seen id)]
        (if (= already 0)
          (do
            (vector-push result id)
            (map-insert seen id 1))
          seen))
      (if (= tag 3)
        ;; Fun: パラメータと戻り値を走査
        (let [seen2 (ti-free-vars-rec (ti-type-fun-param ty) seen result)]
          (ti-free-vars-rec (ti-type-fun-ret ty) seen2 result))
        ;; Con: 自由変数なし
        seen))))

;; === generalize ===

;; 型を一般化: 環境に出現しない自由変数を束縛
;; env: 型環境 (name-hash -> scheme)
;; ty: 一般化する型
;; counter: 型変数カウンタ (自由変数 ID 収集に使用)
;; 戻り値: 型スキーム [ty, bound-count, bound-var1, ...]
(defn ti-generalize [ty env-free-set]
  (let [free-result (vector-new 4)
        seen-map (ref-new (map-new))
        dummy (ti-free-vars-rec ty (map-new) free-result)
        n (vector-length free-result)
        scheme (vector-new 4)
        bound (vector-new 4)
        i (ref-new 0)]
    (do
      ;; 環境に出現しない自由変数を束縛変数として収集
      (let [collect-bound (fn [dummy2]
              (if (< (ref-get i) n)
                (do
                  (let [v (vector-get free-result (ref-get i))]
                    (if (= (map-get env-free-set v) 0)
                      (vector-push bound v)
                      0))
                  (ref-set i (+ (ref-get i) 1))
                  0)
                0))]
        (collect-bound 0))
      (if (< (ref-get i) n)
        (do
          (let [v (vector-get free-result (ref-get i))]
            (if (= (map-get env-free-set v) 0)
              (vector-push bound v)
              0))
          (ref-set i (+ (ref-get i) 1))
          0)
        0)
      ;; 型スキームを構築
      (vector-push scheme ty)
      (vector-push scheme (vector-length bound))
      (if (> (vector-length bound) 0)
        (do
          (vector-push scheme (vector-get bound 0))
          (if (> (vector-length bound) 1)
            (vector-push scheme (vector-get bound 1))
            0))
        0)
      scheme)))

;; === 型推論エラー ===
;; 推論エラーを示す特殊値
(defn ti-infer-error-tag [] 88)

(defn ti-make-infer-error [code]
  (let [v (vector-new 2)]
    (do
      (vector-push v 88)
      (vector-push v code)
      v)))

(defn ti-is-infer-error [val]
  (if (= (vector-get val 0) 88) 1 0))

;; 推論結果: [ty, subst] ペア
(defn ti-make-result [ty subst]
  (let [v (vector-new 2)]
    (do
      (vector-push v ty)
      (vector-push v subst)
      v)))

(defn ti-result-ty [result]
  (vector-get result 0))

(defn ti-result-subst [result]
  (vector-get result 1))

;; === 型推論メイン ===

;; 式の型推論
;; ast: AST ノード
;; env: 型環境
;; subst: 代入環境
;; counter: 型変数カウンタ
;; 戻り値: [ty, subst] または エラー値
(defn ti-infer-expr [ast env subst counter]
  (let [tag (vector-get ast 0)]
    (if (= tag 1)
      ;; lit-int: Int 型
      (ti-make-result (ti-make-int) subst)
      (if (= tag 2)
        ;; lit-bool: Bool 型
        (ti-make-result (ti-make-bool) subst)
        (if (= tag 3)
          ;; lit-string: Unit 型として扱う (v1)
          (ti-make-result (ti-make-unit) subst)
          (if (= tag 4)
            ;; var: 型環境から検索
            (let [name-hash (vector-get ast 1)
                  scheme (ti-env-lookup env name-hash)]
              (if (= scheme 0)
                (ti-make-infer-error 1)  ;; unbound-variable
                (let [ty (ti-instantiate scheme counter)]
                  (ti-make-result ty subst))))
            (if (= tag 6)
              ;; if: cond=Bool, then と else の型を単一化
              (let [r-cond (ti-infer-expr (vector-get ast 1) env subst counter)]
                (if (= (ti-is-infer-error r-cond) 1)
                  r-cond
                  (let [s1 (ti-unify (ti-result-ty r-cond) (ti-make-bool) (ti-result-subst r-cond))]
                    (if (= (ti-unify-failed s1) 1)
                      (ti-make-infer-error 2)  ;; type-mismatch
                      (let [r-then (ti-infer-expr (vector-get ast 2) env s1 counter)]
                        (if (= (ti-is-infer-error r-then) 1)
                          r-then
                          (let [r-else (ti-infer-expr (vector-get ast 3) env (ti-result-subst r-then) counter)]
                            (if (= (ti-is-infer-error r-else) 1)
                              r-else
                              (let [s2 (ti-unify
                                        (ti-result-ty r-then)
                                        (ti-result-ty r-else)
                                        (ti-result-subst r-else))]
                                (if (= (ti-unify-failed s2) 1)
                                  (ti-make-infer-error 3)  ;; branch-type-mismatch
                                  (ti-make-result
                                    (ti-apply-subst s2 (ti-result-ty r-then))
                                    s2)))))))))))
              (if (= tag 7)
                ;; let: init を推論して generalize し環境に追加、body を推論
                (let [name-hash (vector-get ast 1)
                      r-init (ti-infer-expr (vector-get ast 2) env subst counter)]
                  (if (= (ti-is-infer-error r-init) 1)
                    r-init
                    (let [init-ty (ti-apply-subst (ti-result-subst r-init) (ti-result-ty r-init))
                          scheme (ti-make-result init-ty (ti-result-subst r-init))
                          mono-scheme (let [v (vector-new 2)]
                                        (do
                                          (vector-push v init-ty)
                                          (vector-push v 0)
                                          v))
                          new-env (ti-env-bind-scheme env name-hash mono-scheme)
                          r-body (ti-infer-expr (vector-get ast 3) new-env (ti-result-subst r-init) counter)]
                      r-body)))
                (if (= tag 8)
                  ;; lambda: パラメータに新鮮な型変数を割り当てて body を推論
                  (let [param-hash (vector-get ast 1)
                        param-ty (ti-fresh-var counter)
                        param-scheme (let [v (vector-new 2)]
                                       (do
                                         (vector-push v param-ty)
                                         (vector-push v 0)
                                         v))
                        new-env (ti-env-bind-scheme env param-hash param-scheme)
                        r-body (ti-infer-expr (vector-get ast 2) new-env subst counter)]
                    (if (= (ti-is-infer-error r-body) 1)
                      r-body
                      (let [fun-ty (ti-make-fun
                                    (ti-apply-subst (ti-result-subst r-body) param-ty)
                                    (ti-result-ty r-body))]
                        (ti-make-result fun-ty (ti-result-subst r-body)))))
                  (if (= tag 5)
                    ;; apply: func の型を推論し、引数を適用
                    (let [func-hash (vector-get ast 1)
                          argc (vector-get ast 2)
                          ret-ty (ti-fresh-var counter)]
                      (if (= argc 0)
                        ;; 引数なし: func の型を () -> ret_ty に単一化
                        (let [func-scheme (ti-env-lookup env func-hash)]
                          (if (= func-scheme 0)
                            (ti-make-infer-error 4)  ;; unbound-function
                            (let [func-ty (ti-instantiate func-scheme counter)
                                  expected (ti-make-fun (ti-make-unit) ret-ty)
                                  s1 (ti-unify func-ty expected subst)]
                              (if (= (ti-unify-failed s1) 1)
                                (ti-make-infer-error 5)  ;; function-type-mismatch
                                (ti-make-result (ti-apply-subst s1 ret-ty) s1)))))
                        ;; 引数あり: 最初の引数に対して推論
                        (let [arg1 (vector-get ast 3)
                              r-arg1 (ti-infer-expr arg1 env subst counter)]
                          (if (= (ti-is-infer-error r-arg1) 1)
                            r-arg1
                            (let [func-scheme (ti-env-lookup env func-hash)
                                  func-ty (if (= func-scheme 0)
                                            (ti-fresh-var counter)
                                            (ti-instantiate func-scheme counter))
                                  expected (ti-make-fun (ti-result-ty r-arg1) ret-ty)
                                  s1 (ti-unify func-ty expected (ti-result-subst r-arg1))]
                              (if (= (ti-unify-failed s1) 1)
                                (ti-make-infer-error 6)  ;; apply-type-mismatch
                                (ti-make-result (ti-apply-subst s1 ret-ty) s1)))))))
                    ;; その他: Unit 型を返す
                    (ti-make-result (ti-make-unit) subst)))))))))))

;; === 組み込み型環境の初期化 ===

;; 組み込み算術演算の型スキームを登録
;; +, -, *, /: Int -> Int -> Int
;; =, >, <: Int -> Int -> Bool
(defn ti-init-builtin-env []
  (let [env (ref-new (ti-env-new))
        int-ty (ti-make-int)
        bool-ty (ti-make-bool)
        arith-ty (ti-make-fun int-ty (ti-make-fun int-ty int-ty))
        cmp-ty (ti-make-fun int-ty (ti-make-fun int-ty bool-ty))
        arith-scheme (let [v (vector-new 2)]
                       (do (vector-push v arith-ty) (vector-push v 0) v))
        cmp-scheme (let [v (vector-new 2)]
                     (do (vector-push v cmp-ty) (vector-push v 0) v))]
    (do
      ;; + (ASCII 43), - (45), * (42), / (47), % (37)
      (ref-set env (map-insert (ref-get env) 43 arith-scheme))
      (ref-set env (map-insert (ref-get env) 45 arith-scheme))
      (ref-set env (map-insert (ref-get env) 42 arith-scheme))
      (ref-set env (map-insert (ref-get env) 47 arith-scheme))
      (ref-set env (map-insert (ref-get env) 37 arith-scheme))
      ;; = (61), > (62), < (60)
      (ref-set env (map-insert (ref-get env) 61 cmp-scheme))
      (ref-set env (map-insert (ref-get env) 62 cmp-scheme))
      (ref-set env (map-insert (ref-get env) 60 cmp-scheme))
      (ref-get env))))

;; === テスト用エントリポイント ===
(defn main []
  (let [counter (ti-var-counter-new)
        env (ti-init-builtin-env)
        subst (ti-subst-new)]
    (do
      ;; テスト 1: (lit 42) → Int
      (let [lit-node (let [v (vector-new 2)]
                       (do (vector-push v 1) (vector-push v 42) v))
            result (ti-infer-expr lit-node env subst counter)]
        (do
          (print (ti-type-tag (ti-result-ty result)))  ;; 1 (Con)
          (print (ti-type-name (ti-result-ty result))) ;; 100 (Int hash)
          0))

      ;; テスト 2: 型変数カウンタ
      (let [v1 (ti-fresh-var counter)
            v2 (ti-fresh-var counter)]
        (do
          (print (ti-type-tag v1))  ;; 2 (Var)
          (print (ti-type-name v1)) ;; 1000
          (print (ti-type-name v2)) ;; 1001
          0))

      ;; テスト 3: 単一化 (Int と Int)
      (let [int1 (ti-make-int)
            int2 (ti-make-int)
            s1 (ti-unify int1 int2 (ti-subst-new))]
        (do
          (print (ti-unify-failed s1))  ;; 0 (成功)
          0))

      0)))
