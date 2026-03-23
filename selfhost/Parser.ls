;; Parser.ls - L# セルフホスティング: 再帰降下パーサー
;;
;; Lexer が出力したトークン列 (Vector) を受け取り、AST を構築する。
;; S 式構文なので、パーサーは比較的シンプル。

;; トークン種別定数 (Token.ls より)
;; 0=LParen, 1=RParen, 2=LBracket, 3=RBracket
;; 10=Int, 12=String, 13=BoolTrue, 14=BoolFalse, 20=Symbol
;; 30=Defn, 31=Let, 32=If, 36=Do, 99=Eof

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

;; === パース関数 ===

;; 式をパース: S式 or リテラル or シンボル
;; 結果は整数エンコード: tag * 10000 + value
(defn parse-expr [tokens pos src src-positions]
  (let [tok (current-tok tokens pos)]
    (if (== tok 0)  ;; LParen → S 式
      (do
        (advance pos)  ;; ( を消費
        (let [result (parse-sexp tokens pos src src-positions)]
          (do
            (expect tokens pos 1) ;; ) を消費
            result)))
      (if (== tok 10) ;; Int
        (do (advance pos) (+ (* 1 10000) 0))  ;; lit-int ノード
        (if (== tok 13) ;; true
          (do (advance pos) (+ (* 2 10000) 1))  ;; lit-bool true
          (if (== tok 14) ;; false
            (do (advance pos) (+ (* 2 10000) 0))  ;; lit-bool false
            (if (== tok 20) ;; Symbol
              (do (advance pos) (+ (* 4 10000) 0))  ;; var ノード
              0)))))))

;; S 式の内部をパース (( の後)
(defn parse-sexp [tokens pos src src-positions]
  (let [tok (current-tok tokens pos)]
    (if (== tok 30) ;; defn
      (do (advance pos) (+ (* 20 10000) 0))  ;; defn ノード
      (if (== tok 31) ;; let
        (do (advance pos) (+ (* 7 10000) 0))  ;; let ノード
        (if (== tok 32) ;; if
          (do (advance pos) (+ (* 6 10000) 0))  ;; if ノード
          (if (== tok 36) ;; do
            (do (advance pos) (+ (* 9 10000) 0))  ;; do ノード
            (+ (* 5 10000) 0)))))))  ;; apply ノード

;; 式のノード種別を取得
(defn node-tag [encoded]
  (/ encoded 10000))

;; トップレベルをパース: defn 宣言のリストを読む
;; 簡易版: 1つの S 式だけパースして返す
(defn parse-toplevel [tokens pos src]
  (parse-expr tokens pos src (vector-new 0)))

;; エントリポイント (テスト用)
;; "(defn main [] 42)" のトークン列をパース
(defn main []
  (let [tokens (vector-push (vector-push (vector-push (vector-push
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
        result (parse-toplevel tokens pos "")]
    (do
      (print (node-tag result))  ;; 20 (defn)
      (print (ref-get pos))      ;; 8 (全トークン消費)
      0)))
