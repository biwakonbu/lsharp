(module Tools.Text.FormatterExpr)
(import Syntax.AST)

;; FormatterExpr.ls - 式フォーマット・ユーティリティ関数
;;
;; Formatter.ls から分割 (STR-02)
;; 式 (リテラル・演算子・制御フロー・パターン) のフォーマットを担当する。
;;
;; バンドルモードでは FormatterExpr.ls → FormatterDecl.ls → Formatter.ls の順に
;; 連結され、Formatter.ls のディスパッチャが本モジュールの関数を呼び出す。

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

(defn format-lit-string-fallback []
  "\"\"")

(defn format-lit-float-fallback []
  "0.0")

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

(defn format-unsupported-pat [tag]
  (str3 "(unsupported-pat " (int-to-string tag) ")"))

(defn format-lit-string-from-source [expr source]
  (let [start (vector-get expr 1)
    end (vector-get expr 2)
    inner (substring source start end)]
    (str3 "\"" inner "\"")))

(defn format-lit-float-from-source [expr source]
  (substring source (vector-get expr 1) (vector-get expr 2)))

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

(defn format-expr-list-with-source [node idx count indent-level source]
  (if (<= count 0) ""
    (let [expr-text (format-expr-with-source (vector-get node idx) indent-level source)]
      (if (= count 1)
        expr-text
        (str3 expr-text " " (format-expr-list-with-source node (+ idx 1) (- count 1) indent-level source))))))

;; S 式のフォーマット (簡易)
;; 開き括弧 + 要素 + 閉じ括弧
;; 短ければ 1 行、長ければ改行インデント
(defn format-sexp-oneline [elem-count]
  (if (<= elem-count 3)
    1 ;; 1 行に収まる
    0));; 改行が必要

;; let 束縛のフォーマットルール
;; (let [x 1 y 2] body) の束縛部分を縦揃え
(defn format-let-bindings [binding-count indent-level]
  (if (<= binding-count 1)
    1 ;; 束縛 1 つなら 1 行
    (+ indent-level 1)));; 複数束縛はインデント +1

;; defn のフォーマットルール
;; (defn name [params] body) のレイアウト
(defn format-defn-layout [param-count body-lines]
  (if (= body-lines 1)
    1 ;; 本体が 1 行なら全体も 1 行候補
    (+ 1 body-lines)));; 本体が複数行なら行数 +1

;; フォーマット結果の統計
(defn format-stats-new []
  (let [v (vector-new 3)]
    (vector-push
      (vector-push
        (vector-push v 0) ;; 総行数
        0) ;; インデントレベル最大値
      0))) ;; 処理ノード数

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

(defn format-do-with-source [node indent-level source]
  (let [expr-count (vector-get node 1)]
    (if (= expr-count 0)
      "(do)"
      (let [expr-text (format-expr-list-with-source node 2 expr-count indent-level source)]
        (str3 "(do " expr-text ")")))))

(defn format-pattern-list [node idx count indent-level]
  (if (<= count 0) ""
    (let [pat-text (format-pattern (vector-get node idx) indent-level)]
      (if (= count 1)
        pat-text
        (str3 pat-text " " (format-pattern-list node (+ idx 1) (- count 1) indent-level))))))

(defn format-pattern-list-with-source [node idx count indent-level source]
  (if (<= count 0) ""
    (let [pat-text (format-pattern-with-source (vector-get node idx) indent-level source)]
      (if (= count 1)
        pat-text
        (str3 pat-text " " (format-pattern-list-with-source node (+ idx 1) (- count 1) indent-level source))))))

(defn format-recordpat-fields [node idx count indent-level]
  (if (<= count 0) ""
    (let [field-text (symbol-from-hash (vector-get node idx))
      pat-text (format-pattern (vector-get node (+ idx 1)) indent-level)
      pair-text (str3 field-text " " pat-text)]
      (if (= count 1)
        pair-text
        (str3 pair-text " " (format-recordpat-fields node (+ idx 2) (- count 1) indent-level))))))

(defn format-recordpat-fields-with-source [node idx count indent-level source]
  (if (<= count 0) ""
    (let [field-text (symbol-from-hash (vector-get node idx))
      pat-text (format-pattern-with-source (vector-get node (+ idx 1)) indent-level source)
      pair-text (str3 field-text " " pat-text)]
      (if (= count 1)
        pair-text
        (str3 pair-text " " (format-recordpat-fields-with-source node (+ idx 2) (- count 1) indent-level source))))))

(defn format-pattern [pat indent-level]
  (let [tag (vector-get pat 0)]
    (if (= tag 40) "_"
      (if (= tag 41) (symbol-from-hash (vector-get pat 1))
        (if (= tag 42) (format-expr (vector-get pat 1) indent-level)
          (if (= tag 43)
            (let [ctor-text (symbol-from-hash (vector-get pat 1))
              arg-count (vector-get pat 2)]
              (if (= arg-count 0)
                ctor-text
                (let [args-text (format-pattern-list pat 3 arg-count indent-level)]
                  (str5 "(" ctor-text " " args-text ")"))))
            (if (= tag 44)
              (let [field-count (vector-get pat 1)]
                (if (= field-count 0)
                  "{}"
                  (let [fields-text (format-recordpat-fields pat 2 field-count indent-level)]
                    (str3 "{" fields-text "}"))))
              (format-unsupported-pat tag))))))))

(defn format-pattern-with-source [pat indent-level source]
  (let [tag (vector-get pat 0)]
    (if (= tag 40) "_"
      (if (= tag 41) (symbol-from-hash (vector-get pat 1))
        (if (= tag 42) (format-expr-with-source (vector-get pat 1) indent-level source)
          (if (= tag 43)
            (let [ctor-text (symbol-from-hash (vector-get pat 1))
              arg-count (vector-get pat 2)]
              (if (= arg-count 0)
                ctor-text
                (let [args-text (format-pattern-list-with-source pat 3 arg-count indent-level source)]
                  (str5 "(" ctor-text " " args-text ")"))))
            (if (= tag 44)
              (let [field-count (vector-get pat 1)]
                (if (= field-count 0)
                  "{}"
                  (let [fields-text (format-recordpat-fields-with-source pat 2 field-count indent-level source)]
                    (str3 "{" fields-text "}"))))
              (format-unsupported-pat tag))))))))

(defn format-match-arms [node idx count indent-level]
  (if (<= count 0) ""
    (let [pat-text (format-pattern (vector-get node idx) indent-level)
      body-text (format-expr (vector-get node (+ idx 1)) indent-level)
      arm-text (str5 "[" pat-text " " body-text "]")]
      (if (= count 1)
        arm-text
        (str3 arm-text " " (format-match-arms node (+ idx 2) (- count 1) indent-level))))))

(defn format-match-arms-with-source [node idx count indent-level source]
  (if (<= count 0) ""
    (let [pat-text (format-pattern-with-source (vector-get node idx) indent-level source)
      body-text (format-expr-with-source (vector-get node (+ idx 1)) indent-level source)
      arm-text (str5 "[" pat-text " " body-text "]")]
      (if (= count 1)
        arm-text
        (str3 arm-text " " (format-match-arms-with-source node (+ idx 2) (- count 1) indent-level source))))))

(defn format-match [node indent-level]
  (let [scrutinee-text (format-expr (vector-get node 1) indent-level)
    arm-count (vector-get node 2)]
    (if (= arm-count 0)
      (str3 "(match " scrutinee-text ")")
      (let [arms-text (format-match-arms node 3 arm-count indent-level)]
        (str5 "(match " scrutinee-text " " arms-text ")")))))

(defn format-match-with-source [node indent-level source]
  (let [scrutinee-text (format-expr-with-source (vector-get node 1) indent-level source)
    arm-count (vector-get node 2)]
    (if (= arm-count 0)
      (str3 "(match " scrutinee-text ")")
      (let [arms-text (format-match-arms-with-source node 3 arm-count indent-level source)]
        (str5 "(match " scrutinee-text " " arms-text ")")))))

;; ソース文字列付き式フォーマット (string/float リテラル復元用)

(defn format-apply-with-source [node indent-level source]
  (let [func-text (format-expr-with-source (vector-get node 1) indent-level source)
    argc (vector-get node 2)]
    (if (= argc 0)
      (str3 "(" func-text ")")
      (let [args-text (format-expr-list-with-source node 3 argc indent-level source)]
        (str5 "(" func-text " " args-text ")")))))

(defn format-if-with-source [node indent-level source]
  (let [cond-text (format-expr-with-source (vector-get node 1) indent-level source)
    then-text (format-expr-with-source (vector-get node 2) indent-level source)
    else-text (format-expr-with-source (vector-get node 3) indent-level source)]
    (str7 "(if " cond-text " " then-text " " else-text ")")))

(defn format-let-expr-with-source [node indent-level source]
  (let [name-text (symbol-from-hash (vector-get node 1))
    init-text (format-expr-with-source (vector-get node 2) indent-level source)
    body-text (format-expr-with-source (vector-get node 3) indent-level source)]
    (str7 "(let [" name-text " " init-text "] " body-text ")")))

(defn format-lambda-with-source [node indent-level source]
  (let [param-count (vector-get node 1)
    params-text (format-hash-list node 2 param-count)
    body-text (format-expr-with-source (vector-get node (+ 2 param-count)) indent-level source)]
    (str5 "(fn [" params-text "] " body-text ")")))
