(module Tools.Text.Formatter)
(import Syntax.AST)
(import Tools.Text.FormatterExpr)
(import Tools.Text.FormatterDecl)

;; Formatter.ls - AST プリティプリンタ (ディスパッチャ)
;;
;; P9-6d: L# で実装されたフォーマッタ
;; AST ノードを受け取り、整形された S 式の文字列を出力する。
;;
;; STR-02: モジュール分割
;; 以下のサブモジュールに実装を分離:
;;   FormatterExpr.ls - 式フォーマット (リテラル・演算子・制御フロー・パターン)
;;   FormatterDecl.ls - 宣言フォーマット (defn, type, trait, module, impl) + プログラム整形
;;                      format-program, format-decl, main を含む
;;
;; バンドルモードでは FormatterExpr.ls → FormatterDecl.ls → Formatter.ls の順に
;; 連結され、本ファイルのディスパッチャがサブモジュールの関数を呼び出す。
;;
;; フォーマットルール:
;; 1. インデント: 2 スペース
;; 2. 短いフォームは 1 行に収める (閾値: 40 文字)
;; 3. 長いフォームは改行してインデント
;; 4. let 束縛は縦揃え
;; 5. defn のパラメータリストは同一行
;;
;; 出力は文字列として構築 (string-concat ベース)

;; ============================================================
;; format-expr: 全式ノードをフォーマットする
;; ============================================================
;; 入力: AST (Expr) + インデントレベル
;; 出力: canonical な実テキスト。未対応ノードは fallback フォームを返す。
(defn format-expr [expr indent-level]
  (let [tag (vector-get expr 0)]
    (if (= tag 1) (format-lit-int (vector-get expr 1))
      (if (= tag 2) (format-lit-bool (vector-get expr 1))
        (if (= tag 3) (format-lit-string-fallback)
          (if (= tag 4) (format-var (vector-get expr 1))
            (if (= tag 5) (format-apply expr indent-level)
              (if (= tag 6) (format-if expr indent-level)
                (if (= tag 7) (format-let-expr expr indent-level)
                  (if (= tag 8) (format-lambda expr indent-level)
                    (if (= tag 9) (format-do expr indent-level)
                      (if (= tag 10) (format-match expr indent-level)
                        (if (= tag 11) (format-expr (vector-get expr 1) indent-level)
                          (if (= tag 12) (format-recordlit expr indent-level)
                            (if (= tag 13) (format-fieldaccess expr indent-level)
                              (if (= tag 14) (format-recordupdate expr indent-level)
                                (if (= tag 15) (format-computation expr indent-level)
                                  (if (= tag 16) (string-concat "'" (format-expr (vector-get expr 1) indent-level))
                                    (if (= tag 17) (string-concat "~" (format-expr (vector-get expr 1) indent-level))
                                      (if (= tag 18) (string-concat "~@" (format-expr (vector-get expr 1) indent-level))
                                        (if (= tag 19) (format-lit-float-fallback)
                                          (if (= tag 32) (format-lit-unit)
                                            (format-unsupported-expr tag)))))))))))))))))))))))

;; ============================================================
;; format-expr-with-source: ソース文字列付き式フォーマット
;; ============================================================
;; string/float リテラルをソースから復元する source-aware バリアント
(defn format-expr-with-source [expr indent-level source]
  (let [tag (vector-get expr 0)]
    (if (= tag 1) (format-lit-int (vector-get expr 1))
      (if (= tag 2) (format-lit-bool (vector-get expr 1))
        (if (= tag 3) (format-lit-string-from-source expr source)
          (if (= tag 4) (format-var (vector-get expr 1))
            (if (= tag 5) (format-apply-with-source expr indent-level source)
              (if (= tag 6) (format-if-with-source expr indent-level source)
                (if (= tag 7) (format-let-expr-with-source expr indent-level source)
                  (if (= tag 8) (format-lambda-with-source expr indent-level source)
                    (if (= tag 9) (format-do-with-source expr indent-level source)
                      (if (= tag 10) (format-match-with-source expr indent-level source)
                        (if (= tag 11) (format-expr-with-source (vector-get expr 1) indent-level source)
                          (if (= tag 12) (format-recordlit-with-source expr indent-level source)
                            (if (= tag 13) (format-fieldaccess-with-source expr indent-level source)
                              (if (= tag 14) (format-recordupdate-with-source expr indent-level source)
                                (if (= tag 15) (format-computation-with-source expr indent-level source)
                                  (if (= tag 16) (string-concat "'" (format-expr-with-source (vector-get expr 1) indent-level source))
                                    (if (= tag 17) (string-concat "~" (format-expr-with-source (vector-get expr 1) indent-level source))
                                      (if (= tag 18) (string-concat "~@" (format-expr-with-source (vector-get expr 1) indent-level source))
                                        (if (= tag 19) (format-lit-float-from-source expr source)
                                          (if (= tag 32) (format-lit-unit)
                                            (format-unsupported-expr tag)))))))))))))))))))))))
