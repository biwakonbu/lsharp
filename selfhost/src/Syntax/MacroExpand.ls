(module Syntax.MacroExpand)
(import Syntax.AST)
(import Syntax.Token)

;; MacroExpand.ls - L# セルフホスティング: マクロ展開フェーズ
;;
;; defmacro で定義されたマクロを名前→展開ルールのマップで保持し、
;; AST を走査してマクロ呼び出しを展開した AST を返す。
;;
;; 依存: AST.ls, Parser.ls
;;
;; === マクロ展開パイプライン ===
;; 1. collect-macros: AST からマクロ定義を収集
;; 2. expand-macros: AST 内のマクロ呼び出しを展開
;; 3. 再帰展開: マクロ内マクロの展開をサポート

;; ============================================================
;; AST タグ定数 (AST.ls から再定義)
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
(defn tag-module [] 11)
(defn tag-import [] 12)

;; 準引用・アンクオート用タグ
(defn tag-quasiquote [] 13) ;; ` (準引用)
(defn tag-unquote [] 14) ;; ~ (アンクオート)

;; 宣言タグ
(defn tag-defn [] 20)
(defn tag-type-decl [] 21)
(defn tag-defmacro [] 31) ;; マクロ定義 (AST.ast-defmacro と同値)

;; ============================================================
;; マクロテーブル
;; ============================================================
;; マクロテーブル = HashMap<name-hash, macro-entry>
;; macro-entry = [param-count, param-hash1, ..., body-node]
;; (vector の末尾が body)

;; 新しいマクロテーブルを作成
(defn macro-table-new []
  (map-new))

;; マクロを登録
;; name-hash: マクロ名のハッシュ
;; entry: [param-count, param-hash1, ..., body-node]
(defn macro-table-set [table name-hash entry]
  (map-insert table name-hash entry))

;; マクロを検索 (0 = 未登録)
(defn macro-table-get [table name-hash]
  (map-get table name-hash))

;; ============================================================
;; マクロエントリ構築
;; ============================================================

;; defmacro AST ノードからマクロエントリを構築
;; defmacro ノード: [31, name-hash, param-count, param-hash1, ..., body]
;; → entry: [param-count, param-hash1, ..., body]
(defn make-macro-entry [defmacro-node]
  (let [param-count (vector-get defmacro-node 2)
    entry (vector-new 8)]
    (let [e1 (vector-push entry param-count)]
      (if (> param-count 0)
        (let [e2 (vector-push e1 (vector-get defmacro-node 3))]
          (if (> param-count 1)
            (let [e3 (vector-push e2 (vector-get defmacro-node 4))]
              (if (> param-count 2)
                (let [e4 (vector-push e3 (vector-get defmacro-node 5))]
                  (if (> param-count 3)
                    (let [e5 (vector-push e4 (vector-get defmacro-node 6))]
                      ;; body は param-count + 3 の位置
                      (vector-push e5 (vector-get defmacro-node (+ param-count 3))))
                    (vector-push e4 (vector-get defmacro-node (+ param-count 3)))))
                (vector-push e3 (vector-get defmacro-node (+ param-count 3)))))
            (vector-push e2 (vector-get defmacro-node (+ param-count 3)))))
        (vector-push e1 (vector-get defmacro-node 3))))))

;; エントリからパラメータ数を取得
(defn entry-param-count [entry]
  (vector-get entry 0))

;; エントリから N 番目のパラメータハッシュを取得 (0-indexed)
(defn entry-param-hash [entry n]
  (vector-get entry (+ n 1)))

;; エントリから body を取得
(defn entry-body [entry]
  (let [pc (entry-param-count entry)]
    (vector-get entry (+ pc 1))))

;; ============================================================
;; Phase 1: マクロ定義の収集
;; ============================================================

;; プログラム (トップレベル式の vector) からマクロ定義を収集
;; 戻り値: マクロテーブル (hashmap)
(defn collect-macros [program]
  (let [table (macro-table-new)
    len (vector-length program)]
    (collect-macros-loop program table 0 len)))

(defn collect-macros-loop [program table idx len]
  (if (>= idx len) table
    (let [node (vector-get program idx)
      tag (vector-get node 0)]
      (if (= tag (tag-defmacro))
        ;; defmacro ノード → テーブルに登録
        (let [name-hash (vector-get node 1)
          entry (make-macro-entry node)
          new-table (macro-table-set table name-hash entry)]
          (collect-macros-loop program new-table (+ idx 1) len))
        ;; defmacro 以外 → スキップ
        (collect-macros-loop program table (+ idx 1) len)))))

;; ============================================================
;; Phase 2: 引数置換 (テンプレート内のパラメータをマクロ引数で置き換え)
;; ============================================================

;; 引数バインディング = HashMap<param-hash, arg-node>
(defn make-arg-bindings [entry args]
  (let [bindings (map-new)
    pc (entry-param-count entry)]
    (make-arg-bindings-loop entry args bindings 0 pc)))

(defn make-arg-bindings-loop [entry args bindings idx count]
  (if (>= idx count) bindings
    (let [ph (entry-param-hash entry idx)
      arg (vector-get args idx)
      new-bindings (map-insert bindings ph arg)]
      (make-arg-bindings-loop entry args new-bindings (+ idx 1) count))))

;; AST ノード内の変数参照をバインディングで置換
;; bindings: HashMap<param-hash, arg-node>
;; 戻り値: 置換後の AST ノード
(defn substitute-node [node bindings]
  (let [tag (vector-get node 0)]
    (if (= tag (tag-var))
      ;; 変数参照 → バインディングに存在すれば置換
      (let [nh (vector-get node 1)
        bound (map-get bindings nh)]
        (if (= bound 0)
          node ;; バインディングなし → そのまま
          bound)) ;; バインディングあり → 引数ノードで置換
      (if (= tag (tag-lit-int))
        node ;; リーフ → そのまま
        (if (= tag (tag-lit-bool))
          node ;; リーフ → そのまま
          (if (= tag (tag-lit-string))
            node ;; リーフ → そのまま
            (if (= tag (tag-if))
              ;; if: 各子を再帰置換
              (let [cond-node (substitute-node (vector-get node 1) bindings)
                then-node (substitute-node (vector-get node 2) bindings)
                else-node (substitute-node (vector-get node 3) bindings)
                result (vector-new 4)]
                (vector-push (vector-push (vector-push (vector-push result
                        (tag-if)) cond-node) then-node) else-node))
              (if (= tag (tag-let))
                ;; let: init と body を再帰置換 (name-hash はそのまま)
                (let [nh (vector-get node 1)
                  init-node (substitute-node (vector-get node 2) bindings)
                  body-node (substitute-node (vector-get node 3) bindings)
                  result (vector-new 4)]
                  (vector-push (vector-push (vector-push (vector-push result
                          (tag-let)) nh) init-node) body-node))
                (if (= tag (tag-apply))
                  ;; apply: func と各引数を再帰置換
                  (substitute-apply node bindings)
                  (if (= tag (tag-do))
                    ;; do: 各式を再帰置換
                    (substitute-do node bindings)
                    (if (= tag (tag-lambda))
                      ;; lambda: body を再帰置換 (params はそのまま)
                      (substitute-lambda node bindings)
                      (if (= tag (tag-unquote))
                        ;; unquote: 内部の式を評価 (置換)
                        (let [inner (vector-get node 1)]
                          (substitute-node inner bindings))
                        ;; その他 → そのまま
                        node))))))))))))

;; apply ノードの置換
;; [5, func-node, arg-count, arg1, arg2, ..., start, end]
(defn preserve-apply-span [node result]
  (let [idx (+ (vector-get node 2) 3)]
    (if (> (vector-length node) (+ idx 1))
      (vector-push
        (vector-push result (vector-get node idx))
        (vector-get node (+ idx 1)))
      result)))

(defn substitute-apply [node bindings]
  (let [func-node (substitute-node (vector-get node 1) bindings)
    argc (vector-get node 2)
    result (vector-push (vector-push (vector-push (vector-new 8)
          (tag-apply)) func-node) argc)]
    (substitute-apply-args node bindings result 0 argc)))

(defn substitute-apply-args [node bindings result idx count]
  (if (>= idx count) (preserve-apply-span node result)
    (let [arg (vector-get node (+ idx 3))
      new-arg (substitute-node arg bindings)]
      (substitute-apply-args node bindings
        (vector-push result new-arg) (+ idx 1) count))))

;; do ノードの置換
;; [9, expr-count, expr1, expr2, ...]
(defn substitute-do [node bindings]
  (let [ec (vector-get node 1)
    result (vector-push (vector-push (vector-new 8)
        (tag-do)) ec)]
    (substitute-do-exprs node bindings result 0 ec)))

(defn substitute-do-exprs [node bindings result idx count]
  (if (>= idx count) result
    (let [expr (vector-get node (+ idx 2))
      new-expr (substitute-node expr bindings)]
      (substitute-do-exprs node bindings
        (vector-push result new-expr) (+ idx 1) count))))

;; lambda ノードの置換
;; [8, param-count, param-hash1, ..., body]
(defn substitute-lambda [node bindings]
  (let [pc (vector-get node 1)
    ;; params をコピー
    result (vector-push (vector-push (vector-new 8) (tag-lambda)) pc)]
    (let [with-params (copy-lambda-params node result 0 pc)
      body-idx (+ pc 2)
      body (vector-get node body-idx)
      ;; lambda のパラメータと同名の変数はシャドウイングする
      ;; (簡易実装: 外側バインディングをそのまま使う)
      new-body (substitute-node body bindings)]
      (vector-push with-params new-body))))

(defn copy-lambda-params [node result idx count]
  (if (>= idx count) result
    (let [ph (vector-get node (+ idx 2))]
      (copy-lambda-params node (vector-push result ph) (+ idx 1) count))))

;; ============================================================
;; Phase 3: マクロ展開 (AST 全体の走査)
;; ============================================================

;; 最大再帰展開回数 (無限ループ防止)
(defn max-expand-depth [] 16)

;; AST ノードを展開
;; table: マクロテーブル
;; depth: 現在の再帰深度
;; 戻り値: 展開後の AST ノード
(defn expand-node [node table depth]
  (if (>= depth (max-expand-depth))
    node ;; 深度制限 → そのまま返す
    (let [tag (vector-get node 0)]
      (if (= tag (tag-lit-int))
        node ;; リーフ
        (if (= tag (tag-lit-bool))
          node ;; リーフ
          (if (= tag (tag-lit-string))
            node ;; リーフ
            (if (= tag (tag-var))
              node ;; リーフ
              (if (= tag (tag-if))
                ;; if: 各子を再帰展開
                (let [c (expand-node (vector-get node 1) table depth)
                  t (expand-node (vector-get node 2) table depth)
                  e (expand-node (vector-get node 3) table depth)
                  result (vector-new 4)]
                  (vector-push (vector-push (vector-push (vector-push result
                          (tag-if)) c) t) e))
                (if (= tag (tag-let))
                  ;; let: init と body を再帰展開
                  (let [nh (vector-get node 1)
                    init (expand-node (vector-get node 2) table depth)
                    body (expand-node (vector-get node 3) table depth)
                    result (vector-new 4)]
                    (vector-push (vector-push (vector-push (vector-push result
                            (tag-let)) nh) init) body))
                  (if (= tag (tag-apply))
                    ;; apply: マクロ呼び出しの可能性あり
                    (expand-apply node table depth)
                    (if (= tag (tag-do))
                      ;; do: 各式を再帰展開
                      (expand-do node table depth)
                      (if (= tag (tag-lambda))
                        ;; lambda: body を再帰展開
                        (expand-lambda node table depth)
                        (if (= tag (tag-match))
                          ;; match: scrutinee と各腕を再帰展開
                          (expand-match node table depth)
                          ;; その他 (module, import, defn, type-decl) → そのまま
                          node)))))))))))))

;; apply ノードの展開
;; func が var でマクロテーブルにある場合は展開、なければ通常の再帰展開
(defn expand-apply [node table depth]
  (let [func (vector-get node 1)
    func-tag (vector-get func 0)]
    (if (= func-tag (tag-var))
      ;; func が変数参照 → マクロ検索
      (let [nh (vector-get func 1)
        entry (macro-table-get table nh)]
        (if (= entry 0)
          ;; マクロではない → 通常の再帰展開
          (expand-apply-normal node table depth)
          ;; マクロ → 引数を収集して展開
          (let [argc (vector-get node 2)
            args (collect-apply-args node argc)
            bindings (make-arg-bindings entry args)
            body (entry-body entry)
            expanded (substitute-node body bindings)]
            ;; 再帰展開 (マクロ内マクロの展開)
            (expand-node expanded table (+ depth 1)))))
      ;; func が変数参照でない → 通常の再帰展開
      (expand-apply-normal node table depth))))

;; apply の引数を vector に収集
(defn collect-apply-args [node argc]
  (let [args (vector-new 4)]
    (collect-apply-args-loop node args 0 argc)))

(defn collect-apply-args-loop [node args idx count]
  (if (>= idx count) args
    (let [arg (vector-get node (+ idx 3))]
      (collect-apply-args-loop node (vector-push args arg)
        (+ idx 1) count))))

;; 通常の apply 展開 (マクロ呼び出しでない場合)
(defn expand-apply-normal [node table depth]
  (let [func (expand-node (vector-get node 1) table depth)
    argc (vector-get node 2)
    result (vector-push (vector-push (vector-push (vector-new 8)
          (tag-apply)) func) argc)]
    (expand-apply-args-normal node table depth result 0 argc)))

(defn expand-apply-args-normal [node table depth result idx count]
  (if (>= idx count) (preserve-apply-span node result)
    (let [arg (vector-get node (+ idx 3))
      new-arg (expand-node arg table depth)]
      (expand-apply-args-normal node table depth
        (vector-push result new-arg) (+ idx 1) count))))

;; do ノードの展開
(defn expand-do [node table depth]
  (let [ec (vector-get node 1)
    result (vector-push (vector-push (vector-new 8) (tag-do)) ec)]
    (expand-do-exprs node table depth result 0 ec)))

(defn expand-do-exprs [node table depth result idx count]
  (if (>= idx count) result
    (let [expr (vector-get node (+ idx 2))
      new-expr (expand-node expr table depth)]
      (expand-do-exprs node table depth
        (vector-push result new-expr) (+ idx 1) count))))

;; lambda ノードの展開
(defn expand-lambda [node table depth]
  (let [pc (vector-get node 1)
    result (vector-push (vector-push (vector-new 8) (tag-lambda)) pc)]
    (let [with-params (copy-lambda-params node result 0 pc)
      body-idx (+ pc 2)
      body (vector-get node body-idx)
      new-body (expand-node body table depth)]
      (vector-push with-params new-body))))

;; match ノードの展開
;; [10, scrutinee, arm-count, pat1, body1, pat2, body2, ...]
(defn expand-match [node table depth]
  (let [scrutinee (expand-node (vector-get node 1) table depth)
    ac (vector-get node 2)
    result (vector-push (vector-push (vector-push (vector-new 16)
          (tag-match)) scrutinee) ac)]
    (expand-match-arms node table depth result 0 ac)))

(defn expand-match-arms [node table depth result idx count]
  (if (>= idx count) result
    (let [pat-offset (+ (* idx 2) 3)
      body-offset (+ pat-offset 1)
      pat (vector-get node pat-offset)
      body (vector-get node body-offset)
      ;; パターンは展開しない (リテラルまたはコンストラクタ)
      new-body (expand-node body table depth)]
      (expand-match-arms node table depth
        (vector-push (vector-push result pat) new-body)
        (+ idx 1) count))))

;; ============================================================
;; Phase 4: quasiquote 展開
;; ============================================================

;; quasiquote ノードを展開
;; [13, inner-node] → inner 内の unquote を評価
;; quasiquote の中では通常の式はそのままリテラルとして扱い、
;; unquote (~) が現れた場所だけ評価する
(defn expand-quasiquote [node bindings]
  (let [tag (vector-get node 0)]
    (if (= tag (tag-unquote))
      ;; ~expr → expr を置換
      (let [inner (vector-get node 1)]
        (substitute-node inner bindings))
      (if (= tag (tag-lit-int))
        node
        (if (= tag (tag-lit-bool))
          node
          (if (= tag (tag-lit-string))
            node
            (if (= tag (tag-var))
              node ;; quasiquote 内の var はそのまま (シンボルとして保持)
              (if (= tag (tag-if))
                (let [c (expand-quasiquote (vector-get node 1) bindings)
                  t (expand-quasiquote (vector-get node 2) bindings)
                  e (expand-quasiquote (vector-get node 3) bindings)
                  result (vector-new 4)]
                  (vector-push (vector-push (vector-push (vector-push result
                          (tag-if)) c) t) e))
                (if (= tag (tag-apply))
                  (expand-quasiquote-apply node bindings)
                  (if (= tag (tag-let))
                    (let [nh (vector-get node 1)
                      init (expand-quasiquote (vector-get node 2) bindings)
                      body (expand-quasiquote (vector-get node 3) bindings)
                      result (vector-new 4)]
                      (vector-push (vector-push (vector-push (vector-push result
                              (tag-let)) nh) init) body))
                    node))))))))))

;; quasiquote 内の apply 展開
(defn expand-quasiquote-apply [node bindings]
  (let [func (expand-quasiquote (vector-get node 1) bindings)
    argc (vector-get node 2)
    result (vector-push (vector-push (vector-push (vector-new 8)
          (tag-apply)) func) argc)]
    (expand-qq-apply-args node bindings result 0 argc)))

(defn expand-qq-apply-args [node bindings result idx count]
  (if (>= idx count) result
    (let [arg (vector-get node (+ idx 3))
      new-arg (expand-quasiquote arg bindings)]
      (expand-qq-apply-args node bindings
        (vector-push result new-arg) (+ idx 1) count))))

;; ============================================================
;; Phase 5: defmacro のフィルタリング
;; ============================================================

;; プログラムから defmacro 宣言を除去
;; (マクロ展開後、defmacro はもう不要)
(defn filter-defmacros [program]
  (let [result (vector-new 16)
    len (vector-length program)]
    (filter-defmacros-loop program result 0 len)))

(defn filter-defmacros-loop [program result idx len]
  (if (>= idx len) result
    (let [node (vector-get program idx)
      tag (vector-get node 0)]
      (if (= tag (tag-defmacro))
        ;; defmacro → 除去
        (filter-defmacros-loop program result (+ idx 1) len)
        ;; それ以外 → 保持
        (filter-defmacros-loop program (vector-push result node)
          (+ idx 1) len)))))

;; ============================================================
;; メインエントリポイント: expand-macros
;; ============================================================

;; プログラム全体のマクロ展開
;; program: トップレベル式の vector
;; 戻り値: マクロ展開後のプログラム (defmacro 除去済み)
(defn expand-macros [program]
  (let [;; Phase 1: マクロ定義を収集
    table (collect-macros program)
    ;; Phase 2-3: 各トップレベル式を展開
    len (vector-length program)
    expanded (expand-program-nodes program table 0 len)]
    ;; Phase 5: defmacro を除去
    (filter-defmacros expanded)))

;; プログラム内の各ノードを展開
(defn expand-program-nodes [program table idx len]
  (if (>= idx len) program
    (let [node (vector-get program idx)
      tag (vector-get node 0)]
      (if (= tag (tag-defmacro))
        ;; defmacro 自体は展開しない
        (expand-program-nodes program table (+ idx 1) len)
        (if (= tag (tag-defn))
          ;; defn: body を展開
          (let [expanded-defn (expand-defn-body node table)]
            (expand-program-nodes
              (vector-set-at program idx expanded-defn)
              table (+ idx 1) len))
          ;; その他のトップレベル式を展開
          (let [expanded-node (expand-node node table 0)]
            (expand-program-nodes
              (vector-set-at program idx expanded-node)
              table (+ idx 1) len)))))))

;; defn の body を展開
;; defn: [20, name-hash, param-count, param-hash1, ..., body]
(defn expand-defn-body [node table]
  (let [pc (vector-get node 2)
    body-idx (+ pc 3)
    body (vector-get node body-idx)
    new-body (expand-node body table 0)
    ;; 新しい defn ノードを構築
    result (vector-new 8)]
    (let [r1 (vector-push result (tag-defn))
      r2 (vector-push r1 (vector-get node 1))
      r3 (vector-push r2 pc)]
      (let [with-params (copy-defn-params node r3 0 pc)]
        (vector-push with-params new-body)))))

(defn copy-defn-params [node result idx count]
  (if (>= idx count) result
    (let [ph (vector-get node (+ idx 3))]
      (copy-defn-params node (vector-push result ph) (+ idx 1) count))))

;; vector の特定インデックスを置換 (簡易版: 新しい vector を構築)
;; 注: L# の vector は不変なので新しい vector を返す
(defn vector-set-at [vec idx new-val]
  (let [len (vector-length vec)
    result (vector-new len)]
    (vector-set-at-loop vec result idx new-val 0 len)))

(defn vector-set-at-loop [vec result idx new-val i len]
  (if (>= i len) result
    (if (= i idx)
      (vector-set-at-loop vec (vector-push result new-val)
        idx new-val (+ i 1) len)
      (vector-set-at-loop vec (vector-push result (vector-get vec i))
        idx new-val (+ i 1) len))))

;; ============================================================
;; エントリポイント (テスト用)
;; ============================================================

(defn main []
  (let [;; === テスト 1: 空のプログラムでマクロ展開 ===
    empty-prog (vector-new 4)
    result1 (expand-macros empty-prog)

    ;; === テスト 2: マクロなしプログラム (defn のみ) ===
    ;; (defn foo [] 42) → [20, name-hash, 0, [1, 42]]
    lit42 (vector-push (vector-push (vector-new 2) 1) 42)
    defn-foo (vector-push (vector-push (vector-push (vector-push
            (vector-new 4) 20) 100) 0) lit42)
    prog2 (vector-push (vector-new 4) defn-foo)
    result2 (expand-macros prog2)

    ;; === テスト 3: マクロ定義 + 呼び出し ===
    ;; (defmacro double [x] (+ x x))
    ;; → defmacro ノード: [31, name-hash(double), 1, param-hash(x), body]
    ;; body = (+ x x) = [5, [4, hash(+)], 2, [4, hash(x)], [4, hash(x)]]
    var-x (vector-push (vector-push (vector-new 2) 4) 120) ;; x のハッシュ = 120
    var-plus (vector-push (vector-push (vector-new 2) 4) 43) ;; + のハッシュ = 43
    plus-body (vector-push (vector-push (vector-push (vector-push
            (vector-push (vector-new 8) 5) var-plus) 2) var-x) var-x)
    defmacro-double (vector-push (vector-push (vector-push
          (vector-push (vector-push (vector-new 8) (tag-defmacro)) 200) 1) 120) plus-body)
    ;; name-hash(double) = 200, param x = 120

    ;; (defn bar [] (double 5))
    ;; → [20, hash(bar), 0, [5, [4, 200], 1, [1, 5]]]
    var-double (vector-push (vector-push (vector-new 2) 4) 200) ;; double
    lit5 (vector-push (vector-push (vector-new 2) 1) 5)
    call-double (vector-push (vector-push (vector-push (vector-push
            (vector-new 8) 5) var-double) 1) lit5)
    defn-bar (vector-push (vector-push (vector-push (vector-push
            (vector-new 4) 20) 300) 0) call-double)

    prog3 (vector-push (vector-push (vector-new 4) defmacro-double) defn-bar)

    ;; マクロテーブル構築テスト
    table3 (collect-macros prog3)
    entry3 (macro-table-get table3 200)]

    (do
      ;; テスト 1: 空プログラム
      (print (vector-length result1)) ;; 0

      ;; テスト 2: マクロなし (defn がそのまま残る)
      (print (vector-length result2)) ;; 1
      (let [d (vector-get result2 0)]
        (do
          (print (vector-get d 0)) ;; 20 (defn)
          (print (vector-get d 1)))) ;; 100 (name-hash)

      ;; テスト 3: マクロ収集
      (print (entry-param-count entry3)) ;; 1 (パラメータ 1 つ)
      (print (entry-param-hash entry3 0)) ;; 120 (x のハッシュ)

      ;; テスト 3: マクロ展開
      (let [result3 (expand-macros prog3)]
        (do
          ;; defmacro が除去され defn-bar のみ残る
          (print (vector-length result3)) ;; 1
          (let [expanded-bar (vector-get result3 0)
            ;; bar の body は展開されて (+ 5 5) になるはず
            bar-body (vector-get expanded-bar 3)]
            (do
              (print (vector-get expanded-bar 0)) ;; 20 (defn)
              ;; body は [5, [4, 43], 2, [1, 5], [1, 5]] (+ 5 5)
              (print (vector-get bar-body 0)) ;; 5 (apply)
              ;; func は + (hash=43)
              (let [func (vector-get bar-body 1)]
                (print (vector-get func 1))) ;; 43 (+)
              ;; argc = 2
              (print (vector-get bar-body 2)) ;; 2
              ;; arg1 = 5 (lit-int)
              (let [arg1 (vector-get bar-body 3)]
                (do
                  (print (vector-get arg1 0)) ;; 1 (lit-int)
                  (print (vector-get arg1 1)))) ;; 5
              ;; arg2 = 5 (lit-int)
              (let [arg2 (vector-get bar-body 4)]
                (do
                  (print (vector-get arg2 0)) ;; 1 (lit-int)
                  (print (vector-get arg2 1)))))))) ;; 5

      ;; テスト 4: substitute-node 単体テスト
      ;; var(120) を lit(99) に置換
      (let [bindings (map-insert (map-new) 120
          (vector-push (vector-push (vector-new 2) 1) 99))
        result4 (substitute-node var-x bindings)]
        (do
          (print (vector-get result4 0)) ;; 1 (lit-int)
          (print (vector-get result4 1)))) ;; 99

      ;; テスト 5: filter-defmacros
      (let [filtered (filter-defmacros prog3)]
        (do
          (print (vector-length filtered)) ;; 1 (defmacro 除去)
          (print (vector-get (vector-get filtered 0) 0)))) ;; 20 (defn)

      0)))
