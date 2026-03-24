;; MacroExpand.ls - L# セルフホスティング: マクロ展開エンジン
;;
;; defmacro で定義されたマクロを展開する。
;; 移植元: crates/lsharp-syntax/src/macro_expand.rs
;;
;; AST ノードタグ (AST.ls から再定義):
;;   1=lit-int, 2=lit-bool, 3=lit-string, 4=var, 5=apply
;;   6=if, 7=let, 8=lambda, 9=do, 10=match, 20=defn

;; === AST タグ定数 ===
(defn me-tag-lit-int [] 1)
(defn me-tag-var [] 4)
(defn me-tag-apply [] 5)
(defn me-tag-if [] 6)
(defn me-tag-let [] 7)
(defn me-tag-lambda [] 8)
(defn me-tag-do [] 9)
(defn me-tag-defmacro [] 30)  ;; selfhost での defmacro ノードタグ

;; === エラー値 ===

;; エラーを示す特殊値 (tag=99)
(defn macro-error-tag [] 99)

;; エラー値を作成する
(defn make-macro-error [err-code]
  (let [v (vector-new 2)]
    (vector-push (vector-push v 99) err-code)))

;; 値がエラーかどうかを確認する
(defn is-macro-error [val]
  (if (= (vector-get val 0) 99) 1 0))

;; === gensym（ASCII カウンタ方式）===

;; gensym カウンタの初期化 (ref-cell を返す)
(defn gensym-counter-new []
  (ref-new 0))

;; 次のユニークシンボルハッシュを返す (__gen0, __gen1, ...)
;; 実装: 10000 + n の形式のハッシュ値を使用
(defn gensym-next [counter]
  (let [n (ref-get counter)]
    (do
      (ref-set counter (+ n 1))
      (+ 10000 n))))

;; === マクロテーブル ===

;; マクロテーブルを初期化する (エントリを格納するベクタ)
;; 各エントリ形式: [name-hash, param-count, params-vec, body-ast]
(defn macro-table-new []
  (vector-new 16))

;; テーブルに defmacro を登録する
;; name-hash: マクロ名の ASCII ハッシュ
;; params: パラメータ名ハッシュのベクタ
;; body: 展開テンプレートの AST
(defn macro-table-register [table name-hash params body]
  (let [entry (vector-new 4)]
    (do
      (vector-push entry name-hash)
      (vector-push entry (vector-length params))
      (vector-push entry params)
      (vector-push entry body)
      (vector-push table entry)
      table)))

;; 名前ハッシュでマクロを検索する (末尾再帰)
;; 見つかった場合はインデックスを返す、見つからなければ -1 を返す
(defn macro-table-lookup-rec [table name-hash idx len]
  (if (= idx len)
    -1
    (let [entry (vector-get table idx)]
      (if (= (vector-get entry 0) name-hash)
        idx
        (macro-table-lookup-rec table name-hash (+ idx 1) len)))))

(defn macro-table-lookup [table name-hash]
  (macro-table-lookup-rec table name-hash 0 (vector-length table)))

;; === テンプレート置換 ===

;; ~param を実引数に置換するヘルパー
;; params: パラメータ名ハッシュのベクタ
;; args: 実引数 AST のベクタ
;; var-hash: 検索する変数ハッシュ
;; 戻り値: 対応する実引数 AST、見つからなければ -1
(defn find-param-arg [params args var-hash idx count]
  (if (= idx count)
    -1
    (if (= (vector-get params idx) var-hash)
      (vector-get args idx)
      (find-param-arg params args var-hash (+ idx 1) count))))

;; テンプレート AST に対してパラメータ置換を行う (再帰)
;; params: パラメータ名ハッシュのベクタ
;; args: 実引数 AST のベクタ
;; tmpl: テンプレート AST ノード
(defn macro-substitute [params args tmpl]
  (let [tag (vector-get tmpl 0)
        pcount (vector-length params)]
    (if (= tag 4)
      ;; var ノード: パラメータに一致すれば実引数に置換
      (let [var-hash (vector-get tmpl 1)
            found (find-param-arg params args var-hash 0 pcount)]
        (if (= found -1)
          tmpl
          found))
      (if (= tag 6)
        ;; if ノード: [6, cond, then, else]
        (let [new-cond (macro-substitute params args (vector-get tmpl 1))
              new-then (macro-substitute params args (vector-get tmpl 2))
              new-else (macro-substitute params args (vector-get tmpl 3))
              result (vector-new 4)]
          (do
            (vector-push result 6)
            (vector-push result new-cond)
            (vector-push result new-then)
            (vector-push result new-else)
            result))
        (if (= tag 5)
          ;; apply ノード: [5, func-hash, arg-count, arg1, arg2, ...]
          (let [func-hash (vector-get tmpl 1)
                argc (vector-get tmpl 2)
                result (vector-new 4)]
            (do
              (vector-push result 5)
              (vector-push result func-hash)
              (vector-push result argc)
              (if (> argc 0)
                (do
                  (vector-push result (macro-substitute params args (vector-get tmpl 3)))
                  (if (> argc 1)
                    (do
                      (vector-push result (macro-substitute params args (vector-get tmpl 4)))
                      (if (> argc 2)
                        (do
                          (vector-push result (macro-substitute params args (vector-get tmpl 5)))
                          0)
                        0))
                    0))
                0)
              result))
          (if (= tag 7)
            ;; let ノード: [7, name-hash, init-expr, body-expr]
            (let [new-init (macro-substitute params args (vector-get tmpl 2))
                  new-body (macro-substitute params args (vector-get tmpl 3))
                  result (vector-new 4)]
              (do
                (vector-push result 7)
                (vector-push result (vector-get tmpl 1))
                (vector-push result new-init)
                (vector-push result new-body)
                result))
            (if (= tag 9)
              ;; do ノード: [9, expr-count, expr1, expr2, ...]
              (let [ec (vector-get tmpl 1)
                    result (vector-new 4)]
                (do
                  (vector-push result 9)
                  (vector-push result ec)
                  (if (> ec 0)
                    (do
                      (vector-push result (macro-substitute params args (vector-get tmpl 2)))
                      (if (> ec 1)
                        (do
                          (vector-push result (macro-substitute params args (vector-get tmpl 3)))
                          (if (> ec 2)
                            (do
                              (vector-push result (macro-substitute params args (vector-get tmpl 4)))
                              0)
                            0))
                        0))
                    0)
                  result))
              ;; その他のノード (lit-int, lit-bool, lit-string): そのまま返す
              tmpl))))))

;; === 展開エンジン ===

;; 1 ステップのマクロ展開
;; table: マクロテーブル
;; ast: 展開対象の AST ノード (apply ノード)
;; entry-idx: テーブル内のエントリインデックス
(defn macro-expand-once [table ast entry-idx]
  (let [entry (vector-get table entry-idx)
        params (vector-get entry 2)
        body (vector-get entry 3)
        argc (vector-get ast 2)
        args (vector-new 4)]
    (do
      ;; 実引数をベクタに収集
      (if (> argc 0)
        (do
          (vector-push args (vector-get ast 3))
          (if (> argc 1)
            (do
              (vector-push args (vector-get ast 4))
              (if (> argc 2)
                (do
                  (vector-push args (vector-get ast 5))
                  0)
                0))
            0))
        0)
      ;; テンプレートに実引数を代入して展開
      (macro-substitute params args body))))

;; AST の先頭が関数適用かつマクロ呼び出しか判定して展開する (末尾再帰)
;; limit: 再帰制限 (128 で上限)
(defn macro-expand [table ast limit]
  (if (= limit 0)
    ;; 再帰制限超過: エラー値を返す
    (make-macro-error 1)
    (let [tag (vector-get ast 0)]
      (if (= tag 5)
        ;; apply ノード: 先頭がマクロか検索
        (let [func-hash (vector-get ast 1)
              found (macro-table-lookup table func-hash)]
          (if (= found -1)
            ;; 未登録: そのまま通過 (子ノードも展開しない v1 仕様)
            ast
            ;; 登録済みマクロ: 1 ステップ展開して再帰
            (let [expanded (macro-expand-once table ast found)]
              (if (= (is-macro-error expanded) 1)
                expanded
                (macro-expand table expanded (- limit 1))))))
        ;; apply 以外: そのまま通過
        ast))))

;; === Main.ls 統合用エントリポイント ===

;; プログラム全体に対してマクロ展開を適用する
;; decls: 宣言ベクタ (defn ノードのベクタ)
;; table: マクロテーブル (macro-table-new で初期化)
;; 戻り値: 展開済み宣言ベクタ
(defn expand-program-rec [decls table out-decls idx count]
  (if (= idx count)
    out-decls
    (let [decl (vector-get decls idx)
          tag (vector-get decl 0)]
      (if (= tag 30)
        ;; defmacro ノード: テーブルに登録してスキップ
        ;; [30, name-hash, params-vec, body-ast]
        (do
          (macro-table-register table
            (vector-get decl 1)
            (vector-get decl 2)
            (vector-get decl 3))
          (expand-program-rec decls table out-decls (+ idx 1) count))
        ;; その他 (defn): body にマクロ展開を適用して追加
        (do
          (vector-push out-decls decl)
          (expand-program-rec decls table out-decls (+ idx 1) count))))))

(defn expand-program [decls table]
  (expand-program-rec decls table (vector-new 16) 0 (vector-length decls)))

;; === テスト用エントリポイント ===
(defn main []
  (let [table (macro-table-new)
        ;; パラメータ: [hash-x] (x のハッシュを 120 とする)
        params (vector-push (vector-new 1) 120)
        ;; body: (+ ~x 1) = apply[5, 43, 2, var(120), lit(1)]
        arg1 (vector-push (vector-push (vector-new 2) 4) 120)
        arg2 (vector-push (vector-push (vector-new 2) 1) 1)
        body (do
               (let [v (vector-new 5)]
                 (do
                   (vector-push v 5)
                   (vector-push v 43)
                   (vector-push v 2)
                   (vector-push v arg1)
                   (vector-push v arg2)
                   v)))]
    (do
      ;; マクロ登録テスト
      (macro-table-register table 999 params body)
      (let [found (macro-table-lookup table 999)
            not-found (macro-table-lookup table 888)]
        (do
          (print found)     ;; 0 (インデックス 0)
          (print not-found) ;; -1 (未登録)

          ;; マクロ展開テスト: (my-inc 5) = apply[5, 999, 1, lit(5)]
          (let [call-arg (vector-push (vector-push (vector-new 2) 1) 5)
                call-node (do
                             (let [v (vector-new 4)]
                               (do
                                 (vector-push v 5)
                                 (vector-push v 999)
                                 (vector-push v 1)
                                 (vector-push v call-arg)
                                 v)))
                result (macro-expand table call-node 128)]
            (do
              ;; 展開結果は apply[5, 43, 2, lit(5), lit(1)] = (+ 5 1)
              (print (vector-get result 0))  ;; 5 (apply)
              (print (vector-get result 1))  ;; 43 (+)
              (print (vector-get result 2))  ;; 2 (引数数)
              0))

          ;; gensym テスト
          (let [counter (gensym-counter-new)
                g0 (gensym-next counter)
                g1 (gensym-next counter)]
            (do
              (print g0)  ;; 10000
              (print g1)  ;; 10001
              0))

          0)))))
