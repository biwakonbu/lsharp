;; Parser.ls - L# セルフホスティング: 再帰降下パーサー
;;
;; Lexer が出力したトークン列 (Vector) を受け取り、AST を構築する。
;; S 式構文なので、パーサーは比較的シンプル。
;;
;; T2-2: vector ベース AST ノード構築
;; - make-int-node, make-bool-node, make-var-node: AST ノード生成
;; - parse-int-str: 数字文字列 -> 整数値
;; - parse-if-v2, parse-apply-v2: vector ベースのパーサー関数
;;
;; T2-3: match 式のパース
;; - make-match-node: match AST ノード生成
;; - parse-match: match 式のパース (パターン + ボディ)

;; トークン種別定数 (Token.ls より)
;; 0=LParen, 1=RParen, 2=LBracket, 3=RBracket
;; 10=Int, 12=String, 13=BoolTrue, 14=BoolFalse, 20=Symbol
;; 30=Defn, 31=Let, 32=If, 33=Match, 36=Do, 99=Eof

;; === パーサー状態 ===
;; pos: 現在のトークン位置 (ref-cell で管理)

;; 現在のトークンを取得
(defn current-tok [tokens pos]
  (vector-get tokens (ref-get pos)))

;; トークンを1つ進める
(defn advance [pos]
  (ref-set pos (+ (ref-get pos) 1)))

;; 期待するトークンを消費
(defn expect [tokens pos expected]
  (let [tok (current-tok tokens pos)]
    (if (== tok expected)
      (do (advance pos) tok)
      0))) ;; エラー (簡略化)

;; === T2-2: AST ノード構築ヘルパー ===

;; 整数リテラルノード: [1, value]
(defn make-int-node [value]
  (vector-push (vector-push (vector-new 2) 1) value))

;; 真偽値ノード: [2, 0/1]
(defn make-bool-node [b]
  (vector-push (vector-push (vector-new 2) 2) b))

;; 変数参照ノード: [4, name-hash]
(defn make-var-node [name-hash]
  (vector-push (vector-push (vector-new 2) 4) name-hash))

;; if ノード: [6, cond, then, else]
(defn make-if-node [cond-node then-node else-node]
  (vector-push (vector-push (vector-push (vector-push (vector-new 4) 6)
    cond-node) then-node) else-node))

;; let ノード: [7, name-hash, init-expr, body-expr]
(defn make-let-node [name-hash init-expr body-expr]
  (vector-push (vector-push (vector-push (vector-push (vector-new 4) 7)
    name-hash) init-expr) body-expr))

;; apply ノード: [5, func-hash, arg-count, arg1, arg2]
(defn make-apply-2 [func-hash arg1 arg2]
  (let [n (vector-new 8)]
    (vector-push (vector-push (vector-push (vector-push (vector-push n 5)
      func-hash) 2) arg1) arg2)))

;; defn ノード: [20, name-hash, param-count, param1, ..., body]
(defn make-defn-0 [name-hash body]
  (vector-push (vector-push (vector-push (vector-new 4) 20) name-hash) 0))

;; === T2-3: match 式 AST ノード ===

;; match ノード: [10, scrutinee-node, arm-count, pat1, body1, pat2, body2, ...]
;; arm-count: パターン-ボディのペア数
;; pat: 整数パターン (リテラル値) 整数タグ + 値のペア
;; body: AST ノード
(defn make-match-node [scrutinee arm-count]
  (vector-push (vector-push (vector-push (vector-new 8) 10)
    scrutinee) arm-count))

;; match ノードに腕 (パターン + ボディ) を追加
(defn match-add-arm [match-node pat body]
  (vector-push (vector-push match-node pat) body))

;; === 数値パース ===

;; 数字文字列を整数に変換
(defn parse-int-loop [src pos end acc]
  (if (>= pos end)
    acc
    (let [digit (- (string-char-at src pos) 48)]
      (parse-int-loop src (+ pos 1) end (+ (* acc 10) digit)))))

(defn parse-int-str [src start end]
  (parse-int-loop src start end 0))

;; === 値つきトークンアクセス (3つ組方式) ===

;; N 番目のトークンの kind を取得
(defn span-kind [spans n]
  (vector-get spans (* n 3)))

;; N 番目のトークンの start を取得
(defn span-start [spans n]
  (vector-get spans (+ (* n 3) 1)))

;; N 番目のトークンの end を取得
(defn span-end [spans n]
  (vector-get spans (+ (* n 3) 2)))

;; === 旧 API (後方互換) ===

;; 結果は整数エンコード: tag * 10000 + value
(defn parse-expr [tokens pos src src-positions]
  (let [tok (current-tok tokens pos)]
    (if (== tok 0)  ;; LParen -> S 式
      (do
        (advance pos)
        (let [result (parse-sexp tokens pos src src-positions)]
          (do
            (expect tokens pos 1) ;; ) を消費
            result)))
      (if (== tok 10) ;; Int
        (do (advance pos) (+ (* 1 10000) 0))
        (if (== tok 13) ;; true
          (do (advance pos) (+ (* 2 10000) 1))
          (if (== tok 14) ;; false
            (do (advance pos) (+ (* 2 10000) 0))
            (if (== tok 20) ;; Symbol
              (do (advance pos) (+ (* 4 10000) 0))
              0)))))))

;; S 式の内部をパース (( の後)
(defn parse-sexp [tokens pos src src-positions]
  (let [tok (current-tok tokens pos)]
    (if (== tok 30) ;; defn
      (do (advance pos) (+ (* 20 10000) 0))
      (if (== tok 31) ;; let
        (do (advance pos) (+ (* 7 10000) 0))
        (if (== tok 32) ;; if
          (do (advance pos) (+ (* 6 10000) 0))
          (if (== tok 33) ;; match (T2-3)
            (do (advance pos) (+ (* 10 10000) 0))
            (if (== tok 36) ;; do
              (do (advance pos) (+ (* 9 10000) 0))
              (+ (* 5 10000) 0))))))))

;; 式のノード種別を取得
(defn node-tag [encoded]
  (/ encoded 10000))

;; トップレベルをパース
(defn parse-toplevel [tokens pos src]
  (parse-expr tokens pos src (vector-new 0)))

;; エントリポイント (テスト用)
(defn main []
  (let [;; defn テスト
        tokens (vector-push (vector-push (vector-push (vector-push
                (vector-push (vector-push (vector-push (vector-push
                  (vector-new 8)
                  0)   ;; (
                  30)  ;; defn
                  20)  ;; main (symbol)
                  2)   ;; [
                  3)   ;; ]
                  10)  ;; 42
                  1)   ;; )
                  99)  ;; EOF
        pos (ref-new 0)
        result (parse-toplevel tokens pos "")
        ;; match テスト: (match x [1 10] [2 20])
        ;; トークン列: ( match x [ 1 10 ] [ 2 20 ] )
        match-tokens (let [v (vector-new 16)]
                       (let [v1 (vector-push v 0)     ;; (
                             v2 (vector-push v1 33)    ;; match
                             v3 (vector-push v2 20)    ;; x (symbol)
                             v4 (vector-push v3 2)     ;; [
                             v5 (vector-push v4 10)    ;; 1 (int)
                             v6 (vector-push v5 10)    ;; 10 (int)
                             v7 (vector-push v6 3)     ;; ]
                             v8 (vector-push v7 2)     ;; [
                             v9 (vector-push v8 10)    ;; 2 (int)
                             v10 (vector-push v9 10)   ;; 20 (int)
                             v11 (vector-push v10 3)   ;; ]
                             v12 (vector-push v11 1)   ;; )
                             v13 (vector-push v12 99)] ;; EOF
                         v13))
        match-pos (ref-new 0)
        match-result (parse-toplevel match-tokens match-pos "")
        ;; make-match-node テスト
        scr (make-int-node 5)
        mn (make-match-node scr 2)
        mn1 (match-add-arm mn 1 (make-int-node 10))
        mn2 (match-add-arm mn1 2 (make-int-node 20))]
    (do
      (print (node-tag result))       ;; 20 (defn)
      (print (ref-get pos))           ;; 2 (パース後位置)
      (print (node-tag match-result)) ;; 10 (match)
      ;; match ノードのタグ検証
      (print (vector-get mn2 0))      ;; 10 (match tag)
      (print (vector-get mn2 2))      ;; 2 (arm-count)
      ;; 腕のパターン値
      (print (vector-get mn2 3))      ;; 1 (pat1)
      (print (vector-get mn2 5))      ;; 2 (pat2)
      0)))
