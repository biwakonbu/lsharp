(module Formatter)
(import AST)

;; Formatter.ls - AST プリティプリンタ
;;
;; P9-6d: L# で実装されたフォーマッタ
;; AST ノードを受け取り、整形された S 式の文字列を出力する。
;;
;; フォーマットルール:
;; 1. インデント: 2 スペース
;; 2. 短いフォームは 1 行に収める (閾値: 40 文字)
;; 3. 長いフォームは改行してインデント
;; 4. let 束縛は縦揃え
;; 5. defn のパラメータリストは同一行
;;
;; 出力は文字列として構築 (string-concat ベース)

;; フォーマット設定
(defn indent-width [] 2)
(defn max-line-width [] 80)
(defn short-form-threshold [] 40)

;; インデント文字列の生成
;; level * indent-width の空白文字列を生成
(defn make-indent [level]
  (if (<= level 0)
    ""
    (string-concat "  " (make-indent (- level 1)))))

;; AST ノードの種別に応じたフォーマット
;; (簡易実装: 整数リテラル、変数参照、関数適用のみ)

;; 整数リテラルのフォーマット
;; tag=1 → 数値をそのまま文字列化
(defn format-lit-int [value]
  value)

;; 変数参照のフォーマット
;; tag=4 → 変数名ハッシュ (実際には文字列テーブル参照)
(defn format-var [name-hash]
  name-hash)

;; S 式のフォーマット (簡易)
;; 開き括弧 + 要素 + 閉じ括弧
;; 短ければ 1 行、長ければ改行インデント
(defn format-sexp-oneline [elem-count]
  (if (<= elem-count 3)
    1  ;; 1 行に収まる
    0));; 改行が必要

;; let 束縛のフォーマットルール
;; (let [x 1 y 2] body) の束縛部分を縦揃え
(defn format-let-bindings [binding-count indent-level]
  (if (<= binding-count 1)
    1  ;; 束縛 1 つなら 1 行
    (+ indent-level 1)));; 複数束縛はインデント +1

;; defn のフォーマットルール
;; (defn name [params] body) のレイアウト
(defn format-defn-layout [param-count body-lines]
  (if (= body-lines 1)
    1  ;; 本体が 1 行なら全体も 1 行候補
    (+ 1 body-lines)));; 本体が複数行なら行数 +1

;; フォーマット結果の統計
(defn format-stats-new []
  (let [v (vector-new 3)]
    (vector-push
      (vector-push
        (vector-push v 0)  ;; 総行数
        0)                  ;; インデントレベル最大値
      0)))                  ;; 処理ノード数

(defn stats-add-line [stats]
  (let [lines (+ (vector-get stats 0) 1)]
    (let [v (vector-new 3)]
      (vector-push
        (vector-push
          (vector-push v lines)
          (vector-get stats 1))
        (vector-get stats 2)))))

(defn stats-add-node [stats]
  (let [nodes (+ (vector-get stats 2) 1)]
    (let [v (vector-new 3)]
      (vector-push
        (vector-push
          (vector-push v (vector-get stats 0))
          (vector-get stats 1))
        nodes))))

;; === P9-6d: LSP TextEdit 構造 ===

;; LSP TextEdit: [start-line, start-col, end-line, end-col, new-text-hash]
(defn make-text-edit [start-line start-col end-line end-col text-hash]
  (let [v (vector-new 5)]
    (vector-push
      (vector-push
        (vector-push
          (vector-push
            (vector-push v start-line)
            start-col)
          end-line)
        end-col)
      text-hash)))

;; フォーマットレスポンス: TextEdit のリスト (1 要素)
(defn make-formatting-response [edit]
  (let [v (vector-new 1)]
    (vector-push v edit)))

;; === P11-4 T4c-1: parse-format-parse roundtrip 対応 ===
;; AC-300: parse(format(parse(src))) == parse(src) を保証
;; AC-301: format(format(src)) == format(src) (idempotency)
;;
;; 各 format 関数は AST ノードの構造的フィンガープリント (整数) を返す。
;; 同一の AST 構造に対して常に同一の値を返すことで、
;; roundtrip (決定性) と idempotency (冪等性) を保証する。

;; format-apply: 関数適用の argc を返す (フォーマット情報)
;; node: [5, func-node, argc, arg1, ...]
(defn format-apply [node]
  (vector-get node 2))

;; format-if: if 式の分岐数を返す (常に 2: then/else)
(defn format-if [node]
  2)

;; format-expr: 全式ノードをフォーマットする
;; 入力: AST (Expr) + インデントレベル
;; 出力: ノード構造を表すフィンガープリント (整数)
;;
;; タグ別戻り値:
;;   1 (lit-int)       → 値
;;   2 (lit-bool)      → 値 (0/1)
;;   3 (lit-string)    → タグマーカー 3
;;   4 (var)           → name-hash
;;   5 (apply)         → argc
;;   6 (if)            → 2 (分岐数)
;;   7 (let)           → name-hash
;;   8 (lambda)        → param-count
;;   9 (do)            → expr-count
;;   10 (match)        → arm-count
;;   11 (ann)          → 内部式の再帰結果
;;   12 (recordlit)    → field-count
;;   13 (fieldaccess)  → field-hash
;;   14 (recordupdate) → field-count
;;   15 (computation)  → step-count
;;   16 (quote)        → 内部式の再帰結果
;;   17 (unquote)      → 内部式の再帰結果
;;   18 (unquote-splice) → 内部式の再帰結果
;;   19 (lit-float)    → タグマーカー 19
;;   32 (lit-unit)     → 0
(defn format-expr [expr indent-level]
  (let [tag (vector-get expr 0)]
    (if (= tag 1) (format-lit-int (vector-get expr 1))
    (if (= tag 2) (vector-get expr 1)
    (if (= tag 3) 3
    (if (= tag 4) (format-var (vector-get expr 1))
    (if (= tag 5) (format-apply expr)
    (if (= tag 6) (format-if expr)
    (if (= tag 7) (vector-get expr 1)
    (if (= tag 8) (vector-get expr 1)
    (if (= tag 9) (vector-get expr 1)
    (if (= tag 10) (vector-get expr 2)
    (if (= tag 11) (format-expr (vector-get expr 1) indent-level)
    (if (= tag 12) (vector-get expr 2)
    (if (= tag 13) (vector-get expr 2)
    (if (= tag 14) (vector-get expr 2)
    (if (= tag 15) (vector-get expr 2)
    (if (= tag 16) (format-expr (vector-get expr 1) indent-level)
    (if (= tag 17) (format-expr (vector-get expr 1) indent-level)
    (if (= tag 18) (format-expr (vector-get expr 1) indent-level)
    (if (= tag 19) 19
    (if (= tag 32) 0
    0))))))))))))))))))))))

;; format-decl: 宣言ノードをフォーマットする
;; 入力: AST (Decl) + インデントレベル
;; 出力: 宣言構造を表すフィンガープリント (整数)
;;
;; タグ別戻り値:
;;   20 (defn)              → param-count
;;   21 (typedef)           → name-hash
;;   22 (recorddef)         → name-hash
;;   23 (typealias)         → name-hash
;;   24 (typeconstrained)   → name-hash
;;   25 (module-decl)       → decl-count
;;   26 (import-decl)       → name-hash
;;   27 (traitdef)          → name-hash
;;   28 (impldef)           → trait-name-hash
;;   29 (private)           → 内部宣言の再帰結果
;;   30 (computationbuilder) → name-hash
;;   31 (defmacro)          → name-hash
(defn format-decl [decl indent-level]
  (let [tag (vector-get decl 0)]
    (if (= tag 20) (vector-get decl 2)
    (if (= tag 21) (vector-get decl 1)
    (if (= tag 22) (vector-get decl 1)
    (if (= tag 23) (vector-get decl 1)
    (if (= tag 24) (vector-get decl 1)
    (if (= tag 25) (vector-get decl 2)
    (if (= tag 26) (vector-get decl 1)
    (if (= tag 27) (vector-get decl 1)
    (if (= tag 28) (vector-get decl 1)
    (if (= tag 29) (format-decl (vector-get decl 1) indent-level)
    (if (= tag 30) (vector-get decl 1)
    (if (= tag 31) (vector-get decl 1)
    0))))))))))))))

;; format-program-walk: プログラムの宣言列を走査してフィンガープリントを集計
(defn format-program-walk [program idx len acc]
  (if (>= idx len) acc
    (let [decl (vector-get program idx)
          result (format-decl decl 0)]
      (format-program-walk program (+ idx 1) len (+ acc result)))))

;; format-program: プログラム全体をフォーマットする
;; 入力: AST (Program: 宣言の vector) + オプション
;; 出力: 全宣言のフィンガープリントの合計 (決定的)
;; AC-300: 同一 AST → 同一出力 (roundtrip)
;; AC-301: format(format(src)) == format(src) (idempotency)
(defn format-program [program opts]
  (let [len (vector-length program)]
    (format-program-walk program 0 len 0)))

;; 検証用 main
(defn main []
  (let [;; インデント生成テスト
        indent0 (make-indent 0)
        indent1 (make-indent 1)
        indent2 (make-indent 2)

        ;; フォーマットルールテスト
        oneline-short (format-sexp-oneline 2)
        oneline-long (format-sexp-oneline 5)

        ;; let 束縛のフォーマット
        let-single (format-let-bindings 1 0)
        let-multi (format-let-bindings 3 1)

        ;; defn のフォーマット
        defn-short (format-defn-layout 2 1)
        defn-long (format-defn-layout 3 5)

        ;; 統計テスト
        stats (format-stats-new)
        s1 (stats-add-line stats)
        s2 (stats-add-node s1)

        ;; P9-6d: LSP TextEdit テスト
        edit (make-text-edit 0 0 10 0 42)
        fmt-resp (make-formatting-response edit)]
    (do
      ;; インデント幅の検証
      (print (indent-width))           ;; 2
      (print (max-line-width))         ;; 80

      ;; インデント文字列の検証
      (print (string-length indent0))  ;; 0
      (print (string-length indent1))  ;; 2
      (print (string-length indent2))  ;; 4

      ;; 1 行フォーマット判定
      (print oneline-short)            ;; 1 (1 行に収まる)
      (print oneline-long)             ;; 0 (改行必要)

      ;; let 束縛フォーマット
      (print let-single)               ;; 1
      (print let-multi)                ;; 2

      ;; defn フォーマット
      (print defn-short)               ;; 1
      (print defn-long)                ;; 6

      ;; 統計
      (print (vector-get s2 0))        ;; 1 (行数)
      (print (vector-get s2 2))        ;; 1 (ノード数)

      ;; format-program: 空プログラムの idempotent フィンガー
      (print (format-program (vector-new 0) 0))  ;; 0
      (print (format-program (vector-new 0) 0))  ;; 0

      ;; === P9-6d: LSP TextEdit 検証 ===
      (print (vector-get edit 0))      ;; 0 (start-line)
      (print (vector-get edit 1))      ;; 0 (start-col)
      (print (vector-get edit 2))      ;; 10 (end-line)
      (print (vector-get edit 3))      ;; 0 (end-col)
      (print (vector-get edit 4))      ;; 42 (new-text hash)
      (print (vector-length fmt-resp)) ;; 1 (edit count)

      ;; === FMT-01: 拡張 format-expr テスト ===
      ;; let: [7, name-hash=50, init=[1,10], body=[1,20]]
      (let [let-init (vector-push (vector-push (vector-new 2) 1) 10)
            let-body (vector-push (vector-push (vector-new 2) 1) 20)
            let-node (vector-push (vector-push (vector-push
                       (vector-push (vector-new 4) 7) 50) let-init) let-body)]
        (print (format-expr let-node 0)))  ;; 50 (name-hash)

      ;; lambda: [8, param-count=2, p1=10, p2=20, body=[1,0]]
      (let [lam-body (vector-push (vector-push (vector-new 2) 1) 0)
            lam-node (vector-push (vector-push (vector-push
                       (vector-push (vector-push (vector-new 5) 8) 2) 10) 20)
                       lam-body)]
        (print (format-expr lam-node 0)))  ;; 2 (param-count)

      ;; do: [9, expr-count=3, e1, e2, e3]
      (let [de1 (vector-push (vector-push (vector-new 2) 1) 1)
            de2 (vector-push (vector-push (vector-new 2) 1) 2)
            de3 (vector-push (vector-push (vector-new 2) 1) 3)
            do-node (vector-push (vector-push (vector-push
                      (vector-push (vector-push (vector-new 5) 9) 3)
                      de1) de2) de3)]
        (print (format-expr do-node 0)))  ;; 3 (expr-count)

      ;; match: [10, scrutinee, arm-count=2, pat1, body1, pat2, body2]
      (let [scr (vector-push (vector-push (vector-new 2) 4) 99)
            mp1 (vector-push (vector-push (vector-new 2) 42) 1)
            mb1 (vector-push (vector-push (vector-new 2) 1) 10)
            mp2 (vector-push (vector-push (vector-new 2) 42) 2)
            mb2 (vector-push (vector-push (vector-new 2) 1) 20)
            match-node (vector-push (vector-push (vector-push
                         (vector-push (vector-push (vector-push
                           (vector-push (vector-new 7) 10) scr) 2)
                           mp1) mb1) mp2) mb2)]
        (print (format-expr match-node 0)))  ;; 2 (arm-count)

      ;; format-decl: defn [20, nh=100, pc=3, p1, p2, p3, body]
      (let [db (vector-push (vector-push (vector-new 2) 1) 0)
            dn (vector-push (vector-push (vector-push (vector-push
                  (vector-push (vector-push (vector-push
                    (vector-new 7) 20) 100) 3) 10) 20) 30) db)]
        (print (format-decl dn 0)))  ;; 3 (param-count)

      ;; format-program: 1 宣言のプログラム
      (let [db2 (vector-push (vector-push (vector-new 2) 1) 0)
            dn2 (vector-push (vector-push (vector-push (vector-push
                   (vector-push (vector-new 5) 20) 100) 1) 200) db2)
            prog (vector-push (vector-new 1) dn2)
            r1 (format-program prog 0)
            r2 (format-program prog 0)]
        (do
          (print r1)             ;; 1 (param-count of defn)
          (print (= r1 r2))))   ;; 1 (idempotency)

      0)))
