(module TypeInfer)
(import AST)
(import Type)
(import TypeScheme)

;; TypeInfer.ls - L# セルフホスティング: Hindley-Milner 型推論
;;
;; Type.ls (型定義・単一化・代入) と TypeScheme.ls (汎化・具体化) を使い、
;; AST ノードに対して型推論を行う。
;;
;; 依存: Type.ls, TypeScheme.ls, AST.ls
;;
;; 型環境 (TypeEnv) = HashMap<name-hash, TypeScheme>
;; 推論結果 = [subst, type] (Vector of 2 要素)

;; ============================================================
;; AST タグ定数 (AST.ls から参照)
;; ============================================================

(defn tag-lit-int [] 1)
(defn tag-lit-bool [] 2)
(defn tag-lit-string [] 3)
(defn tag-var [] 4)
(defn tag-apply [] 5)
(defn tag-if [] 6)
(defn tag-let [] 7)
(defn tag-lambda [] 8)
(defn tag-do [] 9)
(defn tag-match [] 10)

;; ============================================================
;; 型タグ定数 (Type.ls から参照)
;; ============================================================

(defn ty-con [] 1)
(defn ty-var [] 2)
(defn ty-fun [] 3)

;; 型コンストラクタの名前ハッシュ
(defn hash-int [] 100)
(defn hash-bool [] 200)
(defn hash-string [] 300)

;; ============================================================
;; 型構築ヘルパー (Type.ls の関数を直接使用)
;; 連結コンパイル時に Type.ls の make-type-* が利用可能
;; ============================================================

(defn mk-int [] (make-type-int))
(defn mk-bool [] (make-type-bool))
(defn mk-string [] (make-type-string))
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
;; 推論結果 = [subst, type]
;; ============================================================

(defn make-result [subst ty]
  (vector-push (vector-push (vector-new 2) subst) ty))

(defn result-subst [r] (vector-get r 0))
(defn result-type [r] (vector-get r 1))

;; エラー結果 (subst にエラーマーカー付き)
(defn make-error-result []
  (make-result (map-insert (map-new) -1 1) (mk-int)))

;; 結果がエラーか判定
(defn result-failed [r]
  (map-get (result-subst r) -1))

;; ============================================================
;; 新しい型変数の生成
;; ============================================================

(defn fresh-type-var [counter]
  (mk-var (next-var counter)))

;; ============================================================
;; infer-expr: AST ノードの型推論
;; ============================================================
;; 引数:
;;   node    - AST ノード (Vector)
;;   env     - 型環境 (HashMap<name-hash, TypeScheme>)
;;   subst   - 現在の置換 (HashMap<var-id, Type>)
;;   counter - 型変数カウンタ (ref-cell)
;; 戻り値:
;;   [subst, type] - 更新された置換と推論された型

;; リテラルの型推論
(defn infer-lit [node]
  (let [tag (vector-get node 0)]
    (if (= tag 1)
      (mk-int)
      (if (= tag 2)
        (mk-bool)
        (if (= tag 3)
          (mk-string)
          ;; 不明なリテラル -> Int にフォールバック
          (mk-int))))))

;; 変数参照の型推論
(defn infer-var [node env subst counter]
  (let [name-hash (vector-get node 1)
        scheme (type-env-lookup env name-hash)]
    (if (= scheme 0)
      ;; 未定義変数: エラー
      (make-error-result)
      ;; 型スキームを具体化
      (let [ty (instantiate scheme counter)]
        (make-result subst ty)))))

;; if 式の型推論
;; [6, cond, then, else]
(defn infer-if [node env subst counter]
  (let [cond-node (vector-get node 1)
        then-node (vector-get node 2)
        else-node (vector-get node 3)
        ;; 条件式を推論
        cond-result (infer-expr cond-node env subst counter)]
    (if (= (result-failed cond-result) 1)
      (make-error-result)
      (let [s1 (result-subst cond-result)
            cond-ty (result-type cond-result)
            ;; 条件式は Bool であること
            s2 (unify cond-ty (mk-bool) s1)]
        (if (= (unify-failed s2) 1)
          (make-error-result)
          ;; then 枝を推論
          (let [then-result (infer-expr then-node env s2 counter)]
            (if (= (result-failed then-result) 1)
              (make-error-result)
              (let [s3 (result-subst then-result)
                    then-ty (result-type then-result)
                    ;; else 枝を推論
                    else-result (infer-expr else-node env s3 counter)]
                (if (= (result-failed else-result) 1)
                  (make-error-result)
                  ;; then と else の型を統一
                  (let [s4 (result-subst else-result)
                        else-ty (result-type else-result)
                        s5 (unify (apply-subst s4 then-ty) else-ty s4)]
                    (if (= (unify-failed s5) 1)
                      (make-error-result)
                      (make-result s5 (apply-subst s5 else-ty)))))))))))))

;; let 式の型推論
;; [7, name-hash, init-expr, body-expr]
(defn infer-let [node env subst counter]
  (let [name-hash (vector-get node 1)
        init-node (vector-get node 2)
        body-node (vector-get node 3)
        ;; init を推論
        init-result (infer-expr init-node env subst counter)]
    (if (= (result-failed init-result) 1)
      (make-error-result)
      (let [s1 (result-subst init-result)
            init-ty (result-type init-result)
            ;; 汎化して環境に追加
            scheme (generalize (apply-subst s1 init-ty) (map-new))
            new-env (type-env-insert env name-hash scheme)]
        ;; body を推論
        (infer-expr body-node new-env s1 counter)))))

;; lambda 式の型推論
;; 簡易版: 1 引数のみ対応 [8, param-hash, body]
(defn infer-lambda [node env subst counter]
  (let [param-hash (vector-get node 1)
        body-node (vector-get node 2)
        ;; パラメータに新しい型変数を割り当て
        param-ty (fresh-type-var counter)
        ;; パラメータを単相型として環境に追加
        param-scheme (mono param-ty)
        new-env (type-env-insert env param-hash param-scheme)
        ;; body を推論
        body-result (infer-expr body-node new-env subst counter)]
    (if (= (result-failed body-result) 1)
      (make-error-result)
      (let [s1 (result-subst body-result)
            body-ty (result-type body-result)
            ;; 関数型を構築: param-ty -> body-ty
            fun-ty (mk-fun (apply-subst s1 param-ty) body-ty)]
        (make-result s1 fun-ty)))))

;; 関数適用の型推論
;; [5, func-node, arg-count, arg1, arg2, ...]
;; 簡易版: func は変数ノード、引数は 1-2 個対応
(defn infer-apply [node env subst counter]
  (let [func-node (vector-get node 1)
        argc (vector-get node 2)]
    (if (= argc 0)
      ;; 引数なし: func を推論してそのまま返す
      (infer-expr func-node env subst counter)
      (if (= argc 1)
        ;; 1 引数の適用
        (let [arg1 (vector-get node 3)
              ;; func を推論
              func-result (infer-expr func-node env subst counter)]
          (if (= (result-failed func-result) 1)
            (make-error-result)
            (let [s1 (result-subst func-result)
                  func-ty (result-type func-result)
                  ;; arg1 を推論
                  arg1-result (infer-expr arg1 env s1 counter)]
              (if (= (result-failed arg1-result) 1)
                (make-error-result)
                (let [s2 (result-subst arg1-result)
                      arg1-ty (result-type arg1-result)
                      ;; 戻り値の型変数
                      ret-ty (fresh-type-var counter)
                      ;; func-ty = arg1-ty -> ret-ty と統一
                      expected (mk-fun arg1-ty ret-ty)
                      s3 (unify (apply-subst s2 func-ty) expected s2)]
                  (if (= (unify-failed s3) 1)
                    (make-error-result)
                    (make-result s3 (apply-subst s3 ret-ty))))))))
        ;; 2 引数の適用 (ビルトイン演算子等)
        (let [arg1 (vector-get node 3)
              arg2 (vector-get node 4)
              ;; func を推論
              func-result (infer-expr func-node env subst counter)]
          (if (= (result-failed func-result) 1)
            (make-error-result)
            (let [s1 (result-subst func-result)
                  func-ty (result-type func-result)
                  ;; arg1 を推論
                  arg1-result (infer-expr arg1 env s1 counter)]
              (if (= (result-failed arg1-result) 1)
                (make-error-result)
                (let [s2 (result-subst arg1-result)
                      arg1-ty (result-type arg1-result)
                      ;; arg2 を推論
                      arg2-result (infer-expr arg2 env s2 counter)]
                  (if (= (result-failed arg2-result) 1)
                    (make-error-result)
                    (let [s3 (result-subst arg2-result)
                          arg2-ty (result-type arg2-result)
                          ;; 戻り値の型変数
                          ret-ty (fresh-type-var counter)
                          ;; func-ty = arg1-ty -> arg2-ty -> ret-ty と統一
                          ;; カリー化: func-ty = arg1-ty -> (arg2-ty -> ret-ty)
                          inner-fun (mk-fun arg2-ty ret-ty)
                          expected (mk-fun arg1-ty inner-fun)
                          s4 (unify (apply-subst s3 func-ty) expected s3)]
                      (if (= (unify-failed s4) 1)
                        (make-error-result)
                        (make-result s4 (apply-subst s4 ret-ty))))))))))))))

;; do ブロックの型推論
;; [9, expr-count, expr1, expr2, ...]
(defn infer-do [node env subst counter]
  (let [ec (vector-get node 1)]
    (if (= ec 0)
      ;; 空の do: Int(0) を返す
      (make-result subst (mk-int))
      (if (= ec 1)
        ;; 1 式
        (infer-expr (vector-get node 2) env subst counter)
        ;; 2 式以上: 各式を順次推論、最後の型を返す
        (let [r1 (infer-expr (vector-get node 2) env subst counter)]
          (if (= (result-failed r1) 1)
            (make-error-result)
            (let [s1 (result-subst r1)]
              (if (= ec 2)
                (infer-expr (vector-get node 3) env s1 counter)
                (let [r2 (infer-expr (vector-get node 3) env s1 counter)]
                  (if (= (result-failed r2) 1)
                    (make-error-result)
                    (let [s2 (result-subst r2)]
                      (if (= ec 3)
                        (infer-expr (vector-get node 4) env s2 counter)
                        (let [r3 (infer-expr (vector-get node 4) env s2 counter)]
                          (if (= (result-failed r3) 1)
                            (make-error-result)
                            (let [s3 (result-subst r3)]
                              (if (= ec 4)
                                (infer-expr (vector-get node 5) env s3 counter)
                                ;; 5 式以上: 5 番目まで対応
                                (let [r4 (infer-expr (vector-get node 5) env s3 counter)]
                                  (if (= (result-failed r4) 1)
                                    (make-error-result)
                                    (infer-expr (vector-get node 6) env (result-subst r4) counter)))))))))))))))))))

;; ============================================================
;; infer-pattern: パターンの型推論
;; ============================================================
;; パターン種別:
;;   1 = リテラル整数パターン
;;   2 = リテラル真偽値パターン
;;   3 = リテラル文字列パターン
;;   4 = 変数パターン (ワイルドカード含む)
;;   11 = コンストラクタパターン (tag-pattern)
;;   12 = レコードパターン
;;
;; 引数:
;;   pat     - パターンノード [tag, ...]
;;   env     - 型環境
;;   subst   - 現在の置換
;;   counter - 型変数カウンタ
;; 戻り値:
;;   [subst, type, updated-env] - 更新された置換、パターンの型、束縛追加後の環境

(defn infer-pattern [pat env subst counter]
  (let [tag (vector-get pat 0)]
    (if (= tag 1)
      ;; 整数リテラルパターン: 型は Int、環境変化なし
      (vector-push (make-result subst (mk-int)) env)
      (if (= tag 2)
        ;; 真偽値リテラルパターン: 型は Bool、環境変化なし
        (vector-push (make-result subst (mk-bool)) env)
        (if (= tag 3)
          ;; 文字列リテラルパターン: 型は String、環境変化なし
          (vector-push (make-result subst (mk-string)) env)
          (if (= tag 4)
            ;; 変数パターン / ワイルドカード: 新しい型変数を割り当て
            (let [name-hash (vector-get pat 1)
                  ty (fresh-type-var counter)
                  scheme (mono ty)
                  new-env (type-env-insert env name-hash scheme)]
              (vector-push (make-result subst ty) new-env))
            (if (= tag 11)
              ;; コンストラクタパターン (tag-pattern / constructor-pattern)
              ;; [11, ctor-name-hash, sub-pat-count, sub-pat1, ...]
              (let [ctor-hash (vector-get pat 1)
                    ctor-scheme (type-env-lookup env ctor-hash)]
                (if (= ctor-scheme 0)
                  ;; 未定義コンストラクタ: エラー
                  (vector-push (make-error-result) env)
                  ;; コンストラクタの型を具体化して返す
                  (let [ctor-ty (instantiate ctor-scheme counter)]
                    (vector-push (make-result subst ctor-ty) env))))
              (if (= tag 12)
                ;; レコードパターン
                ;; [12, field-count, field-hash1, sub-pat1, ...]
                ;; 簡易版: 各フィールドに新しい型変数を割り当て
                (let [fc (vector-get pat 1)
                      ty (fresh-type-var counter)]
                  (vector-push (make-result subst ty) env))
                ;; 未知のパターン: 新しい型変数 (ワイルドカード扱い)
                (let [ty (fresh-type-var counter)]
                  (vector-push (make-result subst ty) env))))))))))

;; match 式の型推論
;; [10, scrutinee, arm-count, pat1, body1, pat2, body2, ...]
;; 簡易版: パターンはリテラルのみ (整数/真偽値)
(defn infer-match [node env subst counter]
  (let [scrutinee (vector-get node 1)
        arm-count (vector-get node 2)
        ;; scrutinee を推論
        scrut-result (infer-expr scrutinee env subst counter)]
    (if (= (result-failed scrut-result) 1)
      (make-error-result)
      (let [s1 (result-subst scrut-result)
            scrut-ty (result-type scrut-result)
            ;; 結果型の型変数
            result-ty (fresh-type-var counter)]
        ;; 各腕を処理 (最大 3 腕)
        (if (= arm-count 0)
          (make-result s1 result-ty)
          ;; 腕 1: pat1=node[3], body1=node[4]
          (let [pat1 (vector-get node 3)
                body1 (vector-get node 4)
                ;; パターンの型を推論 (リテラルパターン)
                pat1-ty (infer-lit pat1)
                s2 (unify scrut-ty pat1-ty s1)]
            (if (= (unify-failed s2) 1)
              (make-error-result)
              (let [body1-result (infer-expr body1 env s2 counter)]
                (if (= (result-failed body1-result) 1)
                  (make-error-result)
                  (let [s3 (result-subst body1-result)
                        body1-ty (result-type body1-result)
                        s4 (unify result-ty body1-ty s3)]
                    (if (= (unify-failed s4) 1)
                      (make-error-result)
                      (if (<= arm-count 1)
                        (make-result s4 (apply-subst s4 result-ty))
                        ;; 腕 2: pat2=node[5], body2=node[6]
                        (let [pat2 (vector-get node 5)
                              body2 (vector-get node 6)
                              pat2-ty (infer-lit pat2)
                              s5 (unify (apply-subst s4 scrut-ty) pat2-ty s4)]
                          (if (= (unify-failed s5) 1)
                            (make-error-result)
                            (let [body2-result (infer-expr body2 env s5 counter)]
                              (if (= (result-failed body2-result) 1)
                                (make-error-result)
                                (let [s6 (result-subst body2-result)
                                      body2-ty (result-type body2-result)
                                      s7 (unify (apply-subst s6 result-ty) body2-ty s6)]
                                  (if (= (unify-failed s7) 1)
                                    (make-error-result)
                                    (if (<= arm-count 2)
                                      (make-result s7 (apply-subst s7 result-ty))
                                      ;; 腕 3: pat3=node[7], body3=node[8]
                                      (let [pat3 (vector-get node 7)
                                            body3 (vector-get node 8)
                                            pat3-ty (infer-lit pat3)
                                            s8 (unify (apply-subst s7 scrut-ty) pat3-ty s7)]
                                        (if (= (unify-failed s8) 1)
                                          (make-error-result)
                                          (let [body3-result (infer-expr body3 env s8 counter)]
                                            (if (= (result-failed body3-result) 1)
                                              (make-error-result)
                                              (let [s9 (result-subst body3-result)
                                                    body3-ty (result-type body3-result)
                                                    s10 (unify (apply-subst s9 result-ty) body3-ty s9)]
                                                (if (= (unify-failed s10) 1)
                                                  (make-error-result)
                                                  (make-result s10 (apply-subst s10 result-ty)))))))))))))))))))))))))))

;; ============================================================
;; infer-expr: メインディスパッチ
;; ============================================================

(defn infer-expr [node env subst counter]
  (let [tag (vector-get node 0)]
    (if (= tag 1)
      ;; 整数リテラル
      (make-result subst (mk-int))
      (if (= tag 2)
        ;; 真偽値リテラル
        (make-result subst (mk-bool))
        (if (= tag 3)
          ;; 文字列リテラル
          (make-result subst (mk-string))
          (if (= tag 4)
            ;; 変数参照
            (infer-var node env subst counter)
            (if (= tag 5)
              ;; 関数適用
              (infer-apply node env subst counter)
              (if (= tag 6)
                ;; if 式
                (infer-if node env subst counter)
                (if (= tag 7)
                  ;; let 式
                  (infer-let node env subst counter)
                  (if (= tag 8)
                    ;; lambda 式
                    (infer-lambda node env subst counter)
                    (if (= tag 9)
                      ;; do ブロック
                      (infer-do node env subst counter)
                      (if (= tag 10)
                        ;; match 式
                        (infer-match node env subst counter)
                        ;; 未知のノード: エラー
                        (make-error-result)))))))))))))

;; ============================================================
;; infer-defn: トップレベル関数定義の型推論
;; ============================================================
;; [20, name-hash, param-count, body]
;; 簡易版: 0 引数のみ

(defn infer-defn [node env counter]
  (let [name-hash (vector-get node 1)
        body-node (vector-get node 3)
        subst (subst-new)
        ;; body を推論
        result (infer-expr body-node env subst counter)]
    (if (= (result-failed result) 1)
      (make-error-result)
      (let [s (result-subst result)
            body-ty (result-type result)
            ;; 汎化して環境に追加
            scheme (generalize (apply-subst s body-ty) (map-new))
            new-env (type-env-insert env name-hash scheme)]
        ;; 戻り値: [subst, type, updated-env]
        (vector-push (make-result s (apply-subst s body-ty)) new-env)))))

;; ============================================================
;; infer: 公開 API (Main.ls から呼び出される)
;; ============================================================

(defn infer [program]
  (let [counter (make-var-counter)
        env (init-builtin-env counter)
        n (vector-length program)]
    (if (> n 0)
      (let [decl (vector-get program 0)]
        (if (= (vector-get decl 0) 20)
          (let [out (infer-defn decl env counter)]
            (if (= (vector-length out) 2)
              (vector-get out 1)
              (vector-get out 1)))
          (mk-int)))
      (mk-int))))

;; ============================================================
;; ビルトイン型環境の初期化
;; ============================================================

;; ビルトイン演算子の型を登録
;; + : Int -> Int -> Int (カリー化)
;; = : Int -> Int -> Bool
;; print : Int -> Int
(defn init-builtin-env [counter]
  (let [env (type-env-new)
        int-ty (mk-int)
        bool-ty (mk-bool)
        ;; + : Int -> (Int -> Int)
        add-ty (mk-fun int-ty (mk-fun int-ty int-ty))
        ;; - : Int -> (Int -> Int)
        sub-ty (mk-fun int-ty (mk-fun int-ty int-ty))
        ;; * : Int -> (Int -> Int)
        mul-ty (mk-fun int-ty (mk-fun int-ty int-ty))
        ;; / : Int -> (Int -> Int)
        div-ty (mk-fun int-ty (mk-fun int-ty int-ty))
        ;; = : Int -> (Int -> Bool)
        eq-ty (mk-fun int-ty (mk-fun int-ty bool-ty))
        ;; > : Int -> (Int -> Bool)
        gt-ty (mk-fun int-ty (mk-fun int-ty bool-ty))
        ;; < : Int -> (Int -> Bool)
        lt-ty (mk-fun int-ty (mk-fun int-ty bool-ty))
        ;; print : Int -> Int
        print-ty (mk-fun int-ty int-ty)
        ;; 名前ハッシュ (ASCII コード)
        env1 (type-env-insert env 43 (mono add-ty))
        env2 (type-env-insert env1 45 (mono sub-ty))
        env3 (type-env-insert env2 42 (mono mul-ty))
        env4 (type-env-insert env3 47 (mono div-ty))
        env5 (type-env-insert env4 61 (mono eq-ty))
        env6 (type-env-insert env5 62 (mono gt-ty))
        env7 (type-env-insert env6 60 (mono lt-ty))
        env8 (type-env-insert env7 112 (mono print-ty))]
    env8))

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

;; ============================================================
;; エントリポイント (テスト用)
;; ============================================================

(defn main []
  (let [counter (make-var-counter)
        env (init-builtin-env counter)]
    (do
      ;; テスト 1: 整数リテラル -> Int
      (let [lit (vector-push (vector-push (vector-new 2) 1) 42)
            r1 (infer-expr lit env (subst-new) counter)]
        (do
          (print (result-failed r1))            ;; 0 (成功)
          (print (ty-tag (result-type r1)))      ;; 1 (Con)
          (print (ty-name (result-type r1)))))   ;; 100 (Int hash)

      ;; テスト 2: 真偽値リテラル -> Bool
      (let [bool-lit (vector-push (vector-push (vector-new 2) 2) 1)
            r2 (infer-expr bool-lit env (subst-new) counter)]
        (do
          (print (ty-tag (result-type r2)))      ;; 1 (Con)
          (print (ty-name (result-type r2)))))   ;; 200 (Bool hash)

      ;; テスト 3: if 式 -> then/else の型が一致
      ;; (if true 42 0) -> Int
      (let [cond-node (vector-push (vector-push (vector-new 2) 2) 1)
            then-node (vector-push (vector-push (vector-new 2) 1) 42)
            else-node (vector-push (vector-push (vector-new 2) 1) 0)
            if-node (vector-push (vector-push (vector-push (vector-push (vector-new 4) 6) cond-node) then-node) else-node)
            r3 (infer-expr if-node env (subst-new) counter)]
        (do
          (print (result-failed r3))            ;; 0 (成功)
          (print (ty-tag (result-type r3)))      ;; 1 (Con)
          (print (ty-name (result-type r3)))))   ;; 100 (Int hash)

      ;; テスト 4: let 式
      ;; (let [x 42] x) -> Int
      (let [init-node (vector-push (vector-push (vector-new 2) 1) 42)
            var-node (vector-push (vector-push (vector-new 2) 4) 999)
            let-node (vector-push (vector-push (vector-push (vector-push (vector-new 4) 7) 999) init-node) var-node)
            r4 (infer-expr let-node env (subst-new) counter)]
        (do
          (print (result-failed r4))            ;; 0 (成功)
          (print (ty-tag (result-type r4)))      ;; 1 (Con)
          (print (ty-name (result-type r4)))))   ;; 100 (Int hash)

      ;; テスト 5: 変数の型環境登録と参照
      (let [env2 (type-env-insert env 777 (mono (mk-bool)))
            var-node (vector-push (vector-push (vector-new 2) 4) 777)
            r5 (infer-expr var-node env2 (subst-new) counter)]
        (do
          (print (result-failed r5))            ;; 0 (成功)
          (print (ty-name (result-type r5)))))   ;; 200 (Bool hash)

      ;; テスト 6: 未定義変数 -> エラー
      (let [undef-var (vector-push (vector-push (vector-new 2) 4) 12345)
            r6 (infer-expr undef-var env (subst-new) counter)]
        (print (result-failed r6)))              ;; 1 (エラー)

      ;; テスト 7: do ブロック -> 最後の式の型
      ;; (do 42 true) -> Bool
      (let [expr1 (vector-push (vector-push (vector-new 2) 1) 42)
            expr2 (vector-push (vector-push (vector-new 2) 2) 1)
            do-node (vector-push (vector-push (vector-push (vector-push (vector-new 4) 9) 2) expr1) expr2)
            r7 (infer-expr do-node env (subst-new) counter)]
        (do
          (print (result-failed r7))            ;; 0 (成功)
          (print (ty-name (result-type r7)))))   ;; 200 (Bool hash)

      ;; テスト 8: if 式で条件が Bool でない -> エラー
      ;; (if 42 1 0) -> エラー (条件が Int)
      (let [bad-cond (vector-push (vector-push (vector-new 2) 1) 42)
            then-n (vector-push (vector-push (vector-new 2) 1) 1)
            else-n (vector-push (vector-push (vector-new 2) 1) 0)
            bad-if (vector-push (vector-push (vector-push (vector-push (vector-new 4) 6) bad-cond) then-n) else-n)
            r8 (infer-expr bad-if env (subst-new) counter)]
        (print (result-failed r8)))              ;; 1 (エラー: Int != Bool)

      0)))
