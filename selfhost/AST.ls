(module AST)
(import Token)

;; AST.ls - L# セルフホスティング: AST 定義
;;
;; Rust 版 ast.rs に対応する AST を整数タグで表現する。
;; 各ノードは Vector に格納: [tag, ...fields]

;; === AST ノード種別 ===

;; 式 (Expr)
(defn ast-lit-int [] 1)     ;; 整数リテラル
(defn ast-lit-bool [] 2)    ;; 真偽値リテラル
(defn ast-lit-string [] 3)  ;; 文字列リテラル
(defn ast-var [] 4)         ;; 変数参照
(defn ast-apply [] 5)       ;; 関数適用
(defn ast-if [] 6)          ;; 条件分岐
(defn ast-let [] 7)         ;; let 束縛
(defn ast-lambda [] 8)      ;; ラムダ式
(defn ast-do [] 9)          ;; do ブロック
(defn ast-match [] 10)      ;; match 式

;; Expr 追加ノード (Rust AST 全ノード型対応)
(defn ast-ann [] 11)            ;; 型注釈 Ann
(defn ast-recordlit [] 12)      ;; レコードリテラル RecordLit
(defn ast-fieldaccess [] 13)    ;; フィールドアクセス FieldAccess
(defn ast-recordupdate [] 14)   ;; レコード更新 RecordUpdate
(defn ast-computation [] 15)    ;; 計算式 Computation
(defn ast-quote [] 16)          ;; クオート Quote
(defn ast-unquote [] 17)        ;; アンクオート Unquote
(defn ast-unquote-splice [] 18) ;; アンクオートスプライス UnquoteSplice

;; 宣言 (Decl)
(defn ast-defn [] 20)           ;; 関数定義 Defn
(defn ast-typedef [] 21)        ;; 型定義 TypeDef
(defn ast-type-decl [] 21)      ;; 型定義 TypeDef (別名)
(defn ast-recorddef [] 22)      ;; レコード定義 RecordDef
(defn ast-typealias [] 23)      ;; 型エイリアス TypeAlias
(defn ast-typeconstrained [] 24) ;; 制約付き型 TypeConstrained
(defn ast-module-decl [] 25)    ;; モジュール宣言 ModuleDecl
(defn ast-import-decl [] 26)    ;; インポート宣言 ImportDecl
(defn ast-traitdef [] 27)       ;; トレイト定義 TraitDef
(defn ast-impldef [] 28)        ;; 実装定義 ImplDef
(defn ast-private [] 29)        ;; プライベート宣言 Private
(defn ast-computationbuilder [] 30) ;; 計算ビルダー ComputationBuilder
(defn ast-defmacro [] 31)       ;; マクロ定義 DefMacro

;; パターン (Pattern)
(defn ast-pat-wildcard [] 40)   ;; ワイルドカードパターン Wildcard
(defn ast-pat-var [] 41)        ;; 変数パターン Var (パターン用)
(defn ast-pat-lit [] 42)        ;; リテラルパターン Lit
(defn ast-pat-constructor [] 43) ;; コンストラクタパターン Constructor
(defn ast-pat-recordpat [] 44)  ;; レコードパターン RecordPat

;; === AST ノード構築 ===

;; 整数リテラル: [1, value]
(defn make-lit-int [value]
  (let [v (vector-new 2)]
    (vector-push (vector-push v 1) value)))

;; 真偽値リテラル: [2, 0/1]
(defn make-lit-bool [b]
  (let [v (vector-new 2)]
    (vector-push (vector-push v 2) b)))

;; 変数参照: [4, name-hash]
;; name-hash は文字列のハッシュ (簡易的に先頭数文字のコードを使用)
(defn make-var [name-hash]
  (let [v (vector-new 2)]
    (vector-push (vector-push v 4) name-hash)))

;; 型宣言: [21, name-hash]
(defn make-type-decl [name-hash]
  (let [v (vector-new 2)]
    (vector-push (vector-push v (ast-type-decl)) name-hash)))

;; レコード定義: [22, name-hash]
(defn make-record-def [name-hash]
  (let [v (vector-new 2)]
    (vector-push (vector-push v (ast-recorddef)) name-hash)))

;; モジュール宣言: [25, name-hash]
(defn make-module-decl [name-hash]
  (let [v (vector-new 2)]
    (vector-push (vector-push v (ast-module-decl)) name-hash)))

;; import 宣言: [26, name-hash]
(defn make-import-decl [name-hash]
  (let [v (vector-new 2)]
    (vector-push (vector-push v (ast-import-decl)) name-hash)))

;; trait 宣言: [27, name-hash]
(defn make-trait-def [name-hash]
  (let [v (vector-new 2)]
    (vector-push (vector-push v (ast-traitdef)) name-hash)))

;; quote: [16, expr]
(defn make-quote [expr]
  (let [v (vector-new 2)]
    (vector-push (vector-push v (ast-quote)) expr)))

;; unquote: [17, expr]
(defn make-unquote [expr]
  (let [v (vector-new 2)]
    (vector-push (vector-push v (ast-unquote)) expr)))

;; splice-unquote: [18, expr]
(defn make-unquote-splice [expr]
  (let [v (vector-new 2)]
    (vector-push (vector-push v (ast-unquote-splice)) expr)))

;; match 式: [10, scrutinee-node, arm-count, pat1, body1, pat2, body2, ...]
;; pat は整数 (リテラルパターン) またはノード
;; body は AST ノード

;; 関数適用: [5, func-node-idx, arg-count, arg1, arg2, ...]
;; ノードはインデックスで参照

;; === AST ノードアクセス ===

;; ノードの種別を取得
(defn ast-tag [node]
  (vector-get node 0))

;; === AST 走査基盤 ===

;; ノードが子を持つか判定 (0 = 子なしリーフ)
;; tag: 1(lit-int), 2(lit-bool), 4(var) → リーフ
;; tag: 6(if) → 3子, 7(let) → 2子(init,body), 8(lambda) → 1子
;; tag: 5(apply), 9(do), 10(match) → 可変長
(defn ast-is-leaf [tag]
  (if (= tag 1) 1
    (if (= tag 2) 1
      (if (= tag 4) 1
        (if (= tag 3) 1
          0)))))

;; ノード内で特定の name-hash を持つ var 参照が存在するか検索
;; 見つかれば 1、なければ 0
;; node: AST ノード (Vector)、target-hash: 検索対象の変数ハッシュ
(defn ast-contains-var [node target-hash]
  (let [tag (vector-get node 0)]
    (if (= tag 4)
      ;; var ノード: name-hash が一致するか
      (if (= (vector-get node 1) target-hash) 1 0)
      (if (= tag 1) 0      ;; lit-int: 子なし
      (if (= tag 2) 0      ;; lit-bool: 子なし
      (if (= tag 3) 0      ;; lit-string: 子なし
      (if (= tag 16)
        (ast-contains-var (vector-get node 1) target-hash)
      (if (= tag 17)
        (ast-contains-var (vector-get node 1) target-hash)
      (if (= tag 18)
        (ast-contains-var (vector-get node 1) target-hash)
      (if (= tag 6)
        ;; if ノード: [6, cond, then, else]
        (let [r1 (ast-contains-var (vector-get node 1) target-hash)]
          (if (= r1 1) 1
            (let [r2 (ast-contains-var (vector-get node 2) target-hash)]
              (if (= r2 1) 1
                (ast-contains-var (vector-get node 3) target-hash)))))
      (if (= tag 7)
        ;; let ノード: [7, name-hash, init-expr, body-expr]
        (let [r1 (ast-contains-var (vector-get node 2) target-hash)]
          (if (= r1 1) 1
            (ast-contains-var (vector-get node 3) target-hash)))
      (if (= tag 5)
        ;; apply ノード: [5, func-hash, arg-count, arg1, arg2, ...]
        ;; arg を走査 (最大 2 引数)
        (let [argc (vector-get node 2)]
          (if (> argc 0)
            (let [r1 (ast-contains-var (vector-get node 3) target-hash)]
              (if (= r1 1) 1
                (if (> argc 1)
                  (ast-contains-var (vector-get node 4) target-hash)
                  0)))
            0))
      0))))))))))))

;; AST ノードの数を再帰的にカウント (走査テスト用)
(defn ast-count-nodes [node]
  (let [tag (vector-get node 0)]
    (if (= (ast-is-leaf tag) 1)
      1
      (if (= tag 16)
        (+ 1 (ast-count-nodes (vector-get node 1)))
      (if (= tag 17)
        (+ 1 (ast-count-nodes (vector-get node 1)))
      (if (= tag 18)
        (+ 1 (ast-count-nodes (vector-get node 1)))
      (if (= tag 6)
        ;; if: 1 + cond + then + else
        (+ 1 (+ (ast-count-nodes (vector-get node 1))
               (+ (ast-count-nodes (vector-get node 2))
                  (ast-count-nodes (vector-get node 3)))))
      (if (= tag 7)
        ;; let: 1 + init + body
        (+ 1 (+ (ast-count-nodes (vector-get node 2))
               (ast-count-nodes (vector-get node 3))))
      (if (= tag 5)
        ;; apply: 1 + args
        (let [argc (vector-get node 2)]
          (if (> argc 0)
            (if (> argc 1)
              (+ 1 (+ (ast-count-nodes (vector-get node 3))
                     (ast-count-nodes (vector-get node 4))))
              (+ 1 (ast-count-nodes (vector-get node 3))))
            1))
      (if (= tag 9)
        ;; do: 1 + 各子式 (最大5式展開)
        (let [ec (vector-get node 1)]
          (if (> ec 0)
            (if (> ec 1)
              (if (> ec 2)
                (if (> ec 3)
                  (if (> ec 4)
                    (+ 1 (+ (ast-count-nodes (vector-get node 2))
                           (+ (ast-count-nodes (vector-get node 3))
                              (+ (ast-count-nodes (vector-get node 4))
                                 (+ (ast-count-nodes (vector-get node 5))
                                    (ast-count-nodes (vector-get node 6)))))))
                    (+ 1 (+ (ast-count-nodes (vector-get node 2))
                           (+ (ast-count-nodes (vector-get node 3))
                              (+ (ast-count-nodes (vector-get node 4))
                                 (ast-count-nodes (vector-get node 5)))))))
                  (+ 1 (+ (ast-count-nodes (vector-get node 2))
                         (+ (ast-count-nodes (vector-get node 3))
                            (ast-count-nodes (vector-get node 4))))))
                (+ 1 (+ (ast-count-nodes (vector-get node 2))
                       (ast-count-nodes (vector-get node 3)))))
              (+ 1 (ast-count-nodes (vector-get node 2))))
            1))
      (if (= tag 10)
        ;; match: 1 + scrutinee + 腕の body (最大3腕展開)
        ;; [10, scrutinee, arm-count, pat1, body1, pat2, body2, ...]
        (let [ac (vector-get node 2)
              sc (ast-count-nodes (vector-get node 1))]
          (if (> ac 0)
            (if (> ac 1)
              (if (> ac 2)
                (+ 1 (+ sc (+ (+ (ast-count-nodes (vector-get node 3))
                                 (ast-count-nodes (vector-get node 4)))
                              (+ (+ (ast-count-nodes (vector-get node 5))
                                    (ast-count-nodes (vector-get node 6)))
                                 (+ (ast-count-nodes (vector-get node 7))
                                    (ast-count-nodes (vector-get node 8)))))))
                (+ 1 (+ sc (+ (+ (ast-count-nodes (vector-get node 3))
                                 (ast-count-nodes (vector-get node 4)))
                              (+ (ast-count-nodes (vector-get node 5))
                                 (ast-count-nodes (vector-get node 6)))))))
              (+ 1 (+ sc (+ (ast-count-nodes (vector-get node 3))
                           (ast-count-nodes (vector-get node 4))))))
            (+ 1 sc)))
      1)))))))))))

;; エントリポイント (テスト用)
(defn main []
  (let [lit (make-lit-int 42)
        var1 (make-var 99)
        ;; (if var1 42 0) → if ノード
        if-node (let [v (vector-new 4)]
                  (vector-push (vector-push (vector-push (vector-push v 6)
                    var1) lit) (make-lit-int 0)))
        ;; let x = 42 in (if x 42 0) → let ノード
        let-node (let [v (vector-new 4)]
                   (vector-push (vector-push (vector-push (vector-push v 7)
                     99) lit) if-node))]
    (do
      ;; 基本タグ検証
      (print (ast-tag lit))           ;; 1 (lit-int)
      (print (vector-get lit 1))      ;; 42
      (print (ast-match))             ;; 10

      ;; リーフ判定
      (print (ast-is-leaf 1))         ;; 1 (lit-int はリーフ)
      (print (ast-is-leaf 6))         ;; 0 (if はリーフでない)

      ;; ノードカウント
      (print (ast-count-nodes lit))   ;; 1 (リーフ)
      (print (ast-count-nodes if-node)) ;; 4 (if + var + lit + lit)

      ;; 変数検索
      (print (ast-contains-var if-node 99))  ;; 1 (var1 が含まれる)
      (print (ast-contains-var if-node 88))  ;; 0 (88 は含まれない)
      (print (ast-contains-var let-node 99)) ;; 1 (body 内に var 99 がある)

      ;; do ノード: [9, 2, var(99), lit(0)] → カウント 3
      (let [do-node (vector-push (vector-push (vector-push (vector-push (vector-new 4) 9) 2) var1) (make-lit-int 0))]
        (print (ast-count-nodes do-node)))   ;; 3

      ;; match ノード: [10, lit(0), 1, lit(1), var(99)] → カウント 4
      (let [match-node (vector-push (vector-push (vector-push (vector-push (vector-push (vector-new 6) 10) (make-lit-int 0)) 1) (make-lit-int 1)) var1)]
        (print (ast-count-nodes match-node))) ;; 4

      0)))
