(module Syntax.Lexer)
(import Syntax.Token)

;; Lexer.ls - L# セルフホスティング: 字句解析器

;; === 文字列比較 (ビルトイン非対応のため自前実装) ===
(defn string-eq-loop [s1 s2 i n]
  (if (>= i n) true
    (if (= (string-char-at s1 i) (string-char-at s2 i))
      (string-eq-loop s1 s2 (+ i 1) n)
      false)))

(defn string-eq [s1 s2]
  (let [len1 (string-length s1)
    len2 (string-length s2)]
    (if (= len1 len2)
      (string-eq-loop s1 s2 0 len1)
      false)))

;; === 文字判定 ===

;; 空白文字か
(defn is-ws [c]
  (if (== c 32) true ;; space
    (if (== c 9) true ;; tab
      (if (== c 10) true ;; newline
        (== c 13))))) ;; return

;; 数字か (0-9: ASCII 48-57)
(defn is-digit-char [c]
  (if (>= c 48) (<= c 57) false))

;; アルファベットか
(defn is-alpha-char [c]
  (if (>= c 65)
    (if (<= c 90) true ;; A-Z
      (if (>= c 97) (<= c 122) false)) ;; a-z
    false))

;; シンボル開始文字か
(defn is-symbol-start [c]
  (if (is-alpha-char c) true
    (if (== c 95) true ;; _
      (if (== c 43) true ;; +
        (if (== c 45) true ;; -
          (if (== c 42) true ;; *
            (if (== c 47) true ;; /
              (if (== c 61) true ;; =
                (if (== c 60) true ;; <
                  (if (== c 62) true ;; >
                    (if (== c 33) true ;; !
                      (if (== c 63) true ;; ?
                        (if (== c 38) true ;; &
                          (== c 37)))))))))))))) ;; %

;; シンボル継続文字か
(defn is-symbol-char [c]
  (if (is-symbol-start c) true
    (if (is-digit-char c) true
      (if (== c 46) true ;; .
        (== c 45))))) ;; -

;; === 空白・コメントスキップ ===

;; コメント行の終端を探す (改行まで読み飛ばし)
(defn skip-comment [src pos len]
  (if (>= pos len)
    pos
    (if (== (string-char-at src pos) 10) ;; newline
      (+ pos 1)
      (skip-comment src (+ pos 1) len))))

;; 空白とコメントをスキップし、次のトークン開始位置を返す
(defn skip-ws-loop [src pos len]
  (if (>= pos len)
    pos
    (let [c (string-char-at src pos)]
      (if (is-ws c)
        (skip-ws-loop src (+ pos 1) len)
        (if (== c 59) ;; ;
          (let [end (skip-comment src (+ pos 1) len)]
            (skip-ws-loop src end len))
          pos)))))

;; === キーワード判定 ===

;; 追加キーワード群を Token.ls と同じ canonical number に寄せる
(defn classify-extended-keyword [name]
  (if (string-eq name "open") 49
    (if (string-eq name "constrained") 46
      (if (string-eq name "computation") 47
        (if (string-eq name "builder") 48
          (if (string-eq name "defmacro") 44
            (if (string-eq name "true") 13
              (if (string-eq name "false") 14
                20))))))))

;; シンボル名からトークン種別を返す
;; キーワードでなければ tok-symbol (20) を返す
(defn classify-symbol [name]
  (if (string-eq name "defn") 30
    (if (string-eq name "let") 31
      (if (string-eq name "if") 32
        (if (string-eq name "match") 33
          (if (string-eq name "type") 34
            (if (string-eq name "fn") 35
              (if (string-eq name "do") 36
                (if (string-eq name "module") 37
                  (if (string-eq name "import") 38
                    (if (string-eq name "record") 39
                      (if (string-eq name "trait") 40
                        (if (string-eq name "impl") 41
                          (if (string-eq name "where") 42
                            (if (string-eq name "private") 43
                              (classify-extended-keyword name))))))))))))))))

;; === 数値読み取り ===

;; 数字の終端位置を返す
(defn scan-digits [src pos len]
  (if (>= pos len)
    pos
    (if (is-digit-char (string-char-at src pos))
      (scan-digits src (+ pos 1) len)
      pos)))

;; 小数を含む数値の終端位置を返す
;; 先頭の整数部は既に scan-digits 済みである前提
(defn scan-number-end [src int-end len]
  (if (>= int-end len)
    int-end
    (if (== (string-char-at src int-end) 46) ;; .
      (if (< (+ int-end 1) len)
        (if (is-digit-char (string-char-at src (+ int-end 1)))
          (scan-digits src (+ int-end 2) len)
          int-end)
        int-end)
      int-end)))

;; === シンボル読み取り ===

;; シンボルの終端位置を返す
(defn scan-symbol-end [src pos len]
  (if (>= pos len)
    pos
    (if (is-symbol-char (string-char-at src pos))
      (scan-symbol-end src (+ pos 1) len)
      pos)))

;; === 文字列読み取り ===

;; 文字列の終端 (閉じ引用符の次の位置) を返す
(defn scan-string-end [src pos len]
  (if (>= pos len)
    pos ;; 未終端 (エラーは呼び出し側で)
    (let [c (string-char-at src pos)]
      (if (== c 34) ;; "
        (+ pos 1)
        (if (== c 92) ;; \  (エスケープ)
          (scan-string-end src (+ pos 2) len)
          (scan-string-end src (+ pos 1) len))))))

;; === メインのトークナイザー ===

;; トークンを1つ読み取り、(kind, end_pos) をペアで返す
;; ペア表現: kind * 1000000 + end_pos (簡易エンコード)
(defn lex-one [src pos len]
  (if (>= pos len)
    (+ (* 99 1000000) pos) ;; tok-eof
    (let [c (string-char-at src pos)]
      (if (== c 40) (+ (* 0 1000000) (+ pos 1)) ;; ( -> LParen
        (if (== c 41) (+ (* 1 1000000) (+ pos 1)) ;; ) -> RParen
          (if (== c 91) (+ (* 2 1000000) (+ pos 1)) ;; [ -> LBracket
            (if (== c 93) (+ (* 3 1000000) (+ pos 1)) ;; ] -> RBracket
              (if (== c 123) (+ (* 4 1000000) (+ pos 1)) ;; { -> LBrace
                (if (== c 125) (+ (* 5 1000000) (+ pos 1)) ;; } -> RBrace
                  (if (== c 58) (+ (* 50 1000000) (+ pos 1)) ;; : -> Colon
                    (if (== c 124) (+ (* 52 1000000) (+ pos 1)) ;; | -> Pipe
                      (if (== c 46) (+ (* 53 1000000) (+ pos 1)) ;; . -> Dot
                        (if (== c 39) (+ (* 54 1000000) (+ pos 1)) ;; ' -> Quote
                          (if (== c 126) ;; ~ -> Unquote / SpliceUnquote
                            (if (< (+ pos 1) len)
                              (if (== (string-char-at src (+ pos 1)) 64) ;; @
                                (+ (* 56 1000000) (+ pos 2)) ;; ~@ -> SpliceUnquote
                                (+ (* 55 1000000) (+ pos 1))) ;; ~ -> Unquote
                              (+ (* 55 1000000) (+ pos 1)))
                            (if (== c 35) (+ (* 57 1000000) (+ pos 1)) ;; # -> Hash
                              (if (== c 64) (+ (* 58 1000000) (+ pos 1)) ;; @ -> At
                                (if (== c 34) ;; " -> String
                                  (let [end (scan-string-end src (+ pos 1) len)]
                                    (+ (* 12 1000000) end))
                                  ;; -> (arrow) の特殊処理: - の後に > が続く場合
                                  (if (== c 45)
                                    (if (< (+ pos 1) len)
                                      (if (== (string-char-at src (+ pos 1)) 62) ;; >
                                        (+ (* 51 1000000) (+ pos 2)) ;; -> -> Arrow
                                        ;; - で始まるシンボル
                                        (let [end (scan-symbol-end src (+ pos 1) len)
                                          name (substring src pos end)
                                          kind (classify-symbol name)]
                                          (+ (* kind 1000000) end)))
                                      ;; ソース末尾の - (シンボル)
                                      (+ (* 20 1000000) (+ pos 1)))
                                    (if (is-digit-char c)
                                      (let [int-end (scan-digits src (+ pos 1) len)
                                        end (scan-number-end src int-end len)]
                                        (if (> end int-end)
                                          (+ (* 11 1000000) end) ;; Float
                                          (+ (* 10 1000000) end))) ;; Int
                                      (if (is-symbol-start c)
                                        (let [end (scan-symbol-end src (+ pos 1) len)
                                          name (substring src pos end)
                                          kind (classify-symbol name)]
                                          (+ (* kind 1000000) end))
                                        (+ (* 99 1000000) (+ pos 1)))))))))))))))))))))) ;; unknown -> skip

;; 全トークンを Vector に収集 (kind のみ、後方互換)
(defn tokenize-loop [src pos len tokens]
  (let [ws-pos (skip-ws-loop src pos len)]
    (if (>= ws-pos len)
      (vector-push tokens 99) ;; EOF トークン (kind=99)
      (let [result (lex-one src ws-pos len)
        kind (/ result 1000000)
        end-pos (- result (* kind 1000000))]
        (if (== kind 99)
          (vector-push tokens 99)
          (tokenize-loop src end-pos len
            (vector-push tokens kind)))))))

;; ソース文字列をトークン化して種別の Vector を返す (後方互換)
(defn tokenize [src]
  (tokenize-loop src 0 (string-length src) (vector-new 16)))

;; === T2-1: 値つきトークン (kind, start, end) 3つ組 ===

;; 全トークンを (kind, start, end) 3つ組の Vector に収集
;; 結果の Vector は [kind0, start0, end0, kind1, start1, end1, ...] のフラット構造
(defn tokenize-spans-loop [src pos len tokens]
  (let [ws-pos (skip-ws-loop src pos len)]
    (if (>= ws-pos len)
      ;; EOF トークン: (99, pos, pos)
      (vector-push (vector-push (vector-push tokens 99) ws-pos) ws-pos)
      (let [result (lex-one src ws-pos len)
        kind (/ result 1000000)
        end-pos (- result (* kind 1000000))]
        (if (== kind 99)
          (vector-push (vector-push (vector-push tokens 99) ws-pos) ws-pos)
          (tokenize-spans-loop src end-pos len
            (vector-push (vector-push (vector-push tokens kind) ws-pos) end-pos)))))))

;; ソース文字列をトークン化して (kind, start, end) 3つ組を返す
(defn tokenize-with-spans [src]
  (tokenize-spans-loop src 0 (string-length src) (vector-new 32)))

;; === トークン値の取得 ===

;; トークン列からトークン数を計算 (3つ組方式)
(defn token-count [tokens]
  (/ (vector-length tokens) 3))

;; N 番目のトークンの kind を取得
(defn token-kind [tokens n]
  (vector-get tokens (* n 3)))

;; N 番目のトークンの start を取得
(defn token-start [tokens n]
  (vector-get tokens (+ (* n 3) 1)))

;; N 番目のトークンの end を取得
(defn token-end [tokens n]
  (vector-get tokens (+ (* n 3) 2)))

;; 整数トークンの値を取得 (ソース文字列から数値をパース)
(defn token-int-value [src tokens n]
  (let [start (token-start tokens n)
    end (token-end tokens n)]
    (parse-int-from-string src start end 0)))

;; 数字文字列を整数に変換 (再帰ヘルパー)
(defn parse-int-from-string [src pos end acc]
  (if (>= pos end)
    acc
    (let [digit (- (string-char-at src pos) 48)]
      (parse-int-from-string src (+ pos 1) end (+ (* acc 10) digit)))))

;; シンボル/キーワードトークンのソース文字列を取得
(defn token-text [src tokens n]
  (substring src (token-start tokens n) (token-end tokens n)))

;; エントリポイント (テスト用)
(defn main []
  (let [;; 後方互換テスト
    tokens (tokenize "(defn main [] 42)")
    len (vector-length tokens)]
    (do
      (print len) ;; トークン数
      ;; 各トークンを出力
      (print (vector-get tokens 0)) ;; ( -> 0 (LParen)
      (print (vector-get tokens 1)) ;; defn -> 30 (Defn)
      (print (vector-get tokens 2)) ;; main -> 20 (Symbol)
      (print (vector-get tokens 3)) ;; [ -> 2 (LBracket)
      (print (vector-get tokens 4)) ;; ] -> 3 (RBracket)
      (print (vector-get tokens 5)) ;; 42 -> 10 (Int)
      (print (vector-get tokens 6)) ;; ) -> 1 (RParen)
      (print (vector-get tokens 7)) ;; EOF -> 99

      ;; T2-1: 値つきトークンのテスト
      (let [spans (tokenize-with-spans "(+ 42 x)")
        n (token-count spans)]
        (do
          (print n) ;; トークン数 = 6
          (print (token-kind spans 0)) ;; ( -> 0 (LParen)
          (print (token-kind spans 1)) ;; + -> 20 (Symbol)
          (print (token-kind spans 2)) ;; 42 -> 10 (Int)
          (print (token-kind spans 3)) ;; x -> 20 (Symbol)
          (print (token-kind spans 4)) ;; ) -> 1 (RParen)
          (print (token-kind spans 5)) ;; EOF -> 99
          ;; 整数値の取得
          (print (token-int-value "(+ 42 x)" spans 2)) ;; 42
          ;; スパン情報
          (print (token-start spans 1)) ;; 1 (+ の開始位置)
          (print (token-end spans 1)) ;; 2 (+ の終了位置)
          0))
      0)))
