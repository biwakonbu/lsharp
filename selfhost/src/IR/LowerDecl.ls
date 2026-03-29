(module IR.LowerDecl)
(import IR.IR)
(import IR.LowerExpr)

;; LowerDecl.ls - L# セルフホスティング: 宣言の lowering
;;
;; AST の宣言ノードを IR に変換する。
;; トレイトの辞書引数付き call 変換も含む。

;; === 宣言の lowering ===

;; defn 宣言を IR 関数に変換
;; decl: [20, name-hash, param-count, param1-hash, ..., body-expr]
;; 戻り値: IR 関数データ
(defn lower-decl [decl]
  (let [tag (vector-get decl 0)]
    (if (= tag 20)
      ;; defn: パラメータを環境に登録して body を lowering
      (let [name-hash (vector-get decl 1)
            param-count (vector-get decl 2)
            env (ref-new (map-new))
            idx (ref-new 1)]
        (do
          ;; パラメータを環境に登録 (最大4つ)
          (if (> param-count 0)
            (do
              (ref-set env (map-insert (ref-get env) (vector-get decl 3) (ref-get idx)))
              (ref-set idx (+ (ref-get idx) 1))
              (if (> param-count 1)
                (do
                  (ref-set env (map-insert (ref-get env) (vector-get decl 4) (ref-get idx)))
                  (ref-set idx (+ (ref-get idx) 1))
                  0)
                0))
            0)
          ;; body を lowering
          (let [body-idx (+ 3 param-count)
                body (vector-get decl body-idx)]
            (lower-expr body (ref-get env) (vector-new 8)))))
      ;; その他の宣言: そのまま返す
      decl)))

;; === trait dispatch lowering ===

;; トレイトメソッド呼び出しを辞書引数付き関数呼び出しに変換
;; trait メソッド call を [call, dict-arg, method-idx, args...] 形式に変換
;;
;; 入力: trait 呼び出しノード [5, method-hash, arg-count, args...]
;; dict: トレイト辞書 (HashMap<method-hash, impl-func-idx>)
;; 戻り値: 辞書引数を先頭に追加した関数呼び出し IR
(defn lower-trait-call [call-node dict]
  (let [method-hash (vector-get call-node 1)
        arg-count (vector-get call-node 2)
        ;; 辞書からメソッドの実装関数インデックスを取得
        impl-idx (map-get dict method-hash)
        ;; 辞書引数を先頭に追加した新しい呼び出しを構築
        result (ref-new (vector-new 8))]
    (do
      ;; 辞書引数をまず push
      (ref-set result (vector-push (ref-get result) (make-instr 1 impl-idx)))
      ;; 元の引数を lowering
      (if (> arg-count 0)
        (do
          (ref-set result (vector-push (ref-get result)
            (make-instr 10 1)))  ;; 仮: 最初の引数を local.get
          0)
        0)
      ;; 関数呼び出し命令
      (ref-set result (vector-push (ref-get result) (make-instr 40 impl-idx)))
      (ref-get result))))

;; === エントリポイント (テスト用) ===

(defn main []
  (let [;; defn 宣言の lowering テスト
        ;; (defn f [x] x) -> [20, hash-f, 1, hash-x, var(hash-x)]
        var-node (vector-push (vector-push (vector-new 2) 4) 99)
        decl (vector-push (vector-push (vector-push (vector-push (vector-push
          (vector-new 5) 20) 100) 1) 99) var-node)
        result (lower-decl decl)]
    (do
      (print (vector-length result))  ;; IR 命令数
      0)))
