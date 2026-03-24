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

;; 宣言 (Decl)
(defn ast-defn [] 20)       ;; 関数定義
(defn ast-type-decl [] 21)  ;; 型定義

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

;; match 式: [10, scrutinee-node, arm-count, pat1, body1, pat2, body2, ...]
;; pat は整数 (リテラルパターン) またはノード
;; body は AST ノード

;; 関数適用: [5, func-node-idx, arg-count, arg1, arg2, ...]
;; ノードはインデックスで参照

;; === AST ノードアクセス ===

;; ノードの種別を取得
(defn ast-tag [node]
  (vector-get node 0))

;; エントリポイント (テスト用)
(defn main []
  (let [lit (make-lit-int 42)]
    (do
      (print (ast-tag lit))   ;; 1 (lit-int)
      (print (vector-get lit 1))  ;; 42
      ;; match タグ検証
      (print (ast-match))  ;; 10
      0)))
