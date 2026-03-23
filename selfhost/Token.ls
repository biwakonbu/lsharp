;; Token.ls - L# セルフホスティング: トークン定義
;;
;; Rust 版 token.rs に対応する Token ADT を定義する。
;; 整数タグで識別: 0=LParen, 1=RParen, ..., 25=Eof

;; トークン種別の定数定義
;; デリミタ
(defn tok-lparen [] 0)
(defn tok-rparen [] 1)
(defn tok-lbracket [] 2)
(defn tok-rbracket [] 3)
(defn tok-lbrace [] 4)
(defn tok-rbrace [] 5)

;; リテラル (値はペイロードとして別途保持)
(defn tok-int [] 10)
(defn tok-float [] 11)
(defn tok-string [] 12)
(defn tok-bool-true [] 13)
(defn tok-bool-false [] 14)

;; 識別子
(defn tok-symbol [] 20)

;; キーワード
(defn tok-defn [] 30)
(defn tok-let [] 31)
(defn tok-if [] 32)
(defn tok-match [] 33)
(defn tok-type [] 34)
(defn tok-fn [] 35)
(defn tok-do [] 36)
(defn tok-module [] 37)
(defn tok-import [] 38)
(defn tok-record [] 39)
(defn tok-trait [] 40)
(defn tok-impl [] 41)
(defn tok-where [] 42)
(defn tok-private [] 43)

;; 特殊記号
(defn tok-colon [] 50)
(defn tok-arrow [] 51)
(defn tok-pipe [] 52)
(defn tok-dot [] 53)

;; 終端
(defn tok-eof [] 99)

;; === トークン表現 ===
;; トークンは (kind, start, end) の3つ組で表現
;; リテラルトークンの値はソース文字列から start..end で取得可能

;; エントリポイント (テスト用)
(defn main []
  (do
    (print (tok-lparen))
    (print (tok-rparen))
    (print (tok-eof))
    0))
