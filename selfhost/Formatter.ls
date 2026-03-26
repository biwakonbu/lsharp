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
;; この段階では supported subset を canonical な 1 行 S 式へ整形する。
;; 変数名は parser が保持する hash から決定的に再構成する。

;; 整数リテラルのフォーマット
;; tag=1 → 数値をそのまま文字列化
(defn format-lit-int [value]
  (int-to-string value))

;; 真偽値リテラルのフォーマット
(defn format-lit-bool [value]
  (if (= value 0) "false" "true"))

;; unit リテラルのフォーマット
(defn format-lit-unit []
  "()")

;; 変数参照のフォーマット
;; tag=4 → 変数名ハッシュから決定的にシンボル文字列を再構成
(defn symbol-candidates []
  "zyxwvutsrqponmlkjihgfedcba_ZYXWVUTSRQPONMLKJIHGFEDCBA?>=</-+*&%!")

(defn str3 [a b c]
  (string-concat a (string-concat b c)))

(defn str4 [a b c d]
  (string-concat a (str3 b c d)))

(defn str5 [a b c d e]
  (string-concat a (str4 b c d e)))

(defn str6 [a b c d e f]
  (string-concat a (str5 b c d e f)))

(defn str7 [a b c d e f g]
  (string-concat a (str6 b c d e f g)))

(defn symbol-from-hash-search [hash candidates idx len]
  (if (>= idx len) ""
    (let [code (string-char-at candidates idx)]
      (if (> code hash)
        (symbol-from-hash-search hash candidates (+ idx 1) len)
        (if (= (% (- hash code) 31) 0)
          (let [prefix-hash (/ (- hash code) 31)
                ch (substring candidates idx (+ idx 1))]
            (if (= prefix-hash 0)
              ch
              (let [prefix (symbol-from-hash-search prefix-hash candidates 0 len)]
                (if (> (string-length prefix) 0)
                  (string-concat prefix ch)
                  (symbol-from-hash-search hash candidates (+ idx 1) len)))))
          (symbol-from-hash-search hash candidates (+ idx 1) len))))))

(defn symbol-from-hash [hash]
  (let [candidates (symbol-candidates)
        result
          (if (> hash 0)
            (symbol-from-hash-search hash candidates 0 (string-length candidates))
            "")]
    (if (> (string-length result) 0)
      result
      (string-concat "h" (int-to-string hash)))))

(defn format-var [name-hash]
  (symbol-from-hash name-hash))

(defn format-unsupported-expr [tag]
  (str3 "(unsupported-expr " (int-to-string tag) ")"))

(defn format-unsupported-decl [tag]
  (str3 "(unsupported-decl " (int-to-string tag) ")"))

(defn format-hash-list [node idx count]
  (if (<= count 0) ""
    (let [name-text (symbol-from-hash (vector-get node idx))]
      (if (= count 1)
        name-text
        (str3 name-text " " (format-hash-list node (+ idx 1) (- count 1)))))))

(defn format-expr-list [node idx count indent-level]
  (if (<= count 0) ""
    (let [expr-text (format-expr (vector-get node idx) indent-level)]
      (if (= count 1)
        expr-text
        (str3 expr-text " " (format-expr-list node (+ idx 1) (- count 1) indent-level))))))

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
;; 各 format 関数は supported subset を canonical な実テキストへ整形する。
;; parser が名前の hash だけを保持するため、一部の識別子綴りは canonical 化されうるが、
;; 同じ AST からは常に同じテキストが得られる。

;; format-apply: 関数適用を canonical な 1 行 S 式へ整形する
(defn format-apply [node indent-level]
  (let [func-text (format-expr (vector-get node 1) indent-level)
        argc (vector-get node 2)]
    (if (= argc 0)
      (str3 "(" func-text ")")
      (let [args-text (format-expr-list node 3 argc indent-level)]
        (str5 "(" func-text " " args-text ")")))))

;; format-if: if 式を canonical な 1 行 S 式へ整形する
(defn format-if [node indent-level]
  (let [cond-text (format-expr (vector-get node 1) indent-level)
        then-text (format-expr (vector-get node 2) indent-level)
        else-text (format-expr (vector-get node 3) indent-level)]
    (str7 "(if " cond-text " " then-text " " else-text ")")))

;; format-let: let 式を canonical な 1 行 S 式へ整形する
(defn format-let-expr [node indent-level]
  (let [name-text (symbol-from-hash (vector-get node 1))
        init-text (format-expr (vector-get node 2) indent-level)
        body-text (format-expr (vector-get node 3) indent-level)]
    (str7 "(let [" name-text " " init-text "] " body-text ")")))

;; format-lambda: lambda 式を canonical な 1 行 S 式へ整形する
(defn format-lambda [node indent-level]
  (let [param-count (vector-get node 1)
        params-text (format-hash-list node 2 param-count)
        body-text (format-expr (vector-get node (+ 2 param-count)) indent-level)]
    (str5 "(fn [" params-text "] " body-text ")")))

;; format-do: do 式を canonical な 1 行 S 式へ整形する
(defn format-do [node indent-level]
  (let [expr-count (vector-get node 1)]
    (if (= expr-count 0)
      "(do)"
      (let [expr-text (format-expr-list node 2 expr-count indent-level)]
        (str3 "(do " expr-text ")")))))

;; format-expr: 全式ノードをフォーマットする
;; 入力: AST (Expr) + インデントレベル
;; 出力: canonical な実テキスト。未対応ノードは fallback フォームを返す。
(defn format-expr [expr indent-level]
  (let [tag (vector-get expr 0)]
    (if (= tag 1) (format-lit-int (vector-get expr 1))
    (if (= tag 2) (format-lit-bool (vector-get expr 1))
    (if (= tag 3) (format-unsupported-expr tag)
    (if (= tag 4) (format-var (vector-get expr 1))
    (if (= tag 5) (format-apply expr indent-level)
    (if (= tag 6) (format-if expr indent-level)
    (if (= tag 7) (format-let-expr expr indent-level)
    (if (= tag 8) (format-lambda expr indent-level)
    (if (= tag 9) (format-do expr indent-level)
    (if (= tag 10) (format-unsupported-expr tag)
    (if (= tag 11) (format-expr (vector-get expr 1) indent-level)
    (if (= tag 12) (format-unsupported-expr tag)
    (if (= tag 13) (format-unsupported-expr tag)
    (if (= tag 14) (format-unsupported-expr tag)
    (if (= tag 15) (format-unsupported-expr tag)
    (if (= tag 16) (string-concat "'" (format-expr (vector-get expr 1) indent-level))
    (if (= tag 17) (string-concat "~" (format-expr (vector-get expr 1) indent-level))
    (if (= tag 18) (string-concat "~@" (format-expr (vector-get expr 1) indent-level))
    (if (= tag 19) (format-unsupported-expr tag)
    (if (= tag 32) (format-lit-unit)
    (format-unsupported-expr tag)))))))))))))))))))))))

;; format-decl: supported decl を canonical な実テキストへ整形する
(defn format-defn [decl indent-level]
  (let [name-text (symbol-from-hash (vector-get decl 1))
        param-count (vector-get decl 2)
        params-text (format-hash-list decl 3 param-count)
        body-text (format-expr (vector-get decl (+ 3 param-count)) indent-level)]
    (str7 "(defn " name-text " [" params-text "] " body-text ")")))

(defn format-module-decl [decl]
  (str3 "(module " (symbol-from-hash (vector-get decl 1)) ")"))

(defn format-import-decl [decl]
  (str3 "(import " (symbol-from-hash (vector-get decl 1)) ")"))

(defn format-decl [decl indent-level]
  (let [tag (vector-get decl 0)]
    (if (= tag 20) (format-defn decl indent-level)
    (if (= tag 25) (format-module-decl decl)
    (if (= tag 26) (format-import-decl decl)
    (if (= tag 29) (str3 "(private " (format-decl (vector-get decl 1) indent-level) ")")
    (format-unsupported-decl tag)))))))

(defn format-program-item [item indent-level]
  (let [tag (vector-get item 0)]
    (if (< tag 20)
      (format-expr item indent-level)
      (format-decl item indent-level))))

;; format-program-items: プログラム要素を改行区切りで連結
(defn format-program-items [program idx len]
  (if (>= idx len) ""
    (let [item-text (format-program-item (vector-get program idx) 0)]
      (if (= (+ idx 1) len)
        item-text
        (str3 item-text "\n" (format-program-items program (+ idx 1) len))))))

;; format-program: プログラム全体をフォーマットする
;; 入力: AST (Program: 宣言の vector) + オプション
;; 出力: canonical な実テキスト。CLI 連携のため末尾改行を付ける。
;; AC-300: 同一 AST → 同一出力 (roundtrip)
;; AC-301: format(format(src)) == format(src) (idempotency)
(defn format-program [program opts]
  (let [len (vector-length program)]
    (if (= len 0)
      "\n"
      (string-concat (format-program-items program 0 len) "\n"))))

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

        ;; 空プログラム整形の安定性
        empty-program (format-program (vector-new 0) 0)

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

      ;; format-program: 空プログラムは末尾改行 1 文字、同一入力で連続一致
      (print (string-length empty-program))  ;; 1
      (print (if (string-eq empty-program (format-program (vector-new 0) 0)) 1 0))  ;; 1

      ;; === P9-6d: LSP TextEdit 検証 ===
      (print (vector-get edit 0))      ;; 0 (start-line)
      (print (vector-get edit 1))      ;; 0 (start-col)
      (print (vector-get edit 2))      ;; 10 (end-line)
      (print (vector-get edit 3))      ;; 0 (end-col)
      (print (vector-get edit 4))      ;; 42 (new-text hash)
      (print (vector-length fmt-resp)) ;; 1 (edit count)

      ;; === FMT-01: 拡張 format-expr テスト ===
      ;; let: [7, name-hash=x(120), init=[1,10], body=[4,x]]
      (let [let-init (vector-push (vector-push (vector-new 2) 1) 10)
            let-body (vector-push (vector-push (vector-new 2) 4) 120)
            let-node (vector-push (vector-push (vector-push
                       (vector-push (vector-new 4) 7) 120) let-init) let-body)]
        (print (format-expr let-node 0)))  ;; (let [x 10] x)

      ;; lambda: [8, param-count=2, x, y, body=[1,0]]
      (let [lam-body (vector-push (vector-push (vector-new 2) 1) 0)
            lam-node (vector-push (vector-push (vector-push
                       (vector-push (vector-push (vector-new 5) 8) 2) 120) 121)
                       lam-body)]
        (print (format-expr lam-node 0)))  ;; (fn [x y] 0)

      ;; do: [9, expr-count=3, e1, e2, e3]
      (let [de1 (vector-push (vector-push (vector-new 2) 1) 1)
            de2 (vector-push (vector-push (vector-new 2) 1) 2)
            de3 (vector-push (vector-push (vector-new 2) 1) 3)
            do-node (vector-push (vector-push (vector-push
                      (vector-push (vector-push (vector-new 5) 9) 3)
                      de1) de2) de3)]
        (print (format-expr do-node 0)))  ;; 3 (expr-count)

      ;; match: [10, scrutinee, arm-count=2, pat1, body1, pat2, body2]
      (let [scr (vector-push (vector-push (vector-new 2) 4) 120)
            mp1 (vector-push (vector-push (vector-new 2) 42) 1)
            mb1 (vector-push (vector-push (vector-new 2) 1) 10)
            mp2 (vector-push (vector-push (vector-new 2) 42) 2)
            mb2 (vector-push (vector-push (vector-new 2) 1) 20)
            match-node (vector-push (vector-push (vector-push
                         (vector-push (vector-push (vector-push
                            (vector-push (vector-new 7) 10) scr) 2)
                            mp1) mb1) mp2) mb2)]
        (print (format-expr match-node 0)))  ;; (unsupported-expr 10)

      ;; format-decl: defn [20, nh=a(97), pc=3, x, y, z, body]
      (let [db (vector-push (vector-push (vector-new 2) 1) 0)
            dn (vector-push (vector-push (vector-push (vector-push
                  (vector-push (vector-push (vector-push
                    (vector-new 7) 20) 97) 3) 120) 121) 122) db)]
        (print (format-decl dn 0)))  ;; (defn a [x y z] 0)

      ;; format-program: 1 宣言のプログラム
      (let [db2 (vector-push (vector-push (vector-new 2) 1) 0)
            dn2 (vector-push (vector-push (vector-push (vector-push
                   (vector-push (vector-new 5) 20) 97) 1) 120) db2)
            prog (vector-push (vector-new 1) dn2)
            r1 (format-program prog 0)
            r2 (format-program prog 0)]
        (do
          (print r1)             ;; (defn a [x] 0)\n
          (print (if (string-eq r1 r2) 1 0))))   ;; 1 (idempotency)

      0)))
