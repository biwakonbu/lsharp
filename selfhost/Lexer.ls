;; Lexer.ls - L# セルフホスティング: 字句解析器
;;
;; ソース文字列を受け取り、トークン列 (Vector) を返す。
;; Rust 版 lexer.rs の L# 移植版。

;; === 文字判定 ===

;; 空白文字か
(defn is-ws [c]
  (if (== c 32) true    ;; space
    (if (== c 9) true   ;; tab
      (if (== c 10) true ;; newline
        (== c 13)))))    ;; return

;; 数字か (0-9: ASCII 48-57)
(defn is-digit-char [c]
  (if (>= c 48) (<= c 57) false))

;; アルファベットか
(defn is-alpha-char [c]
  (if (>= c 65)
    (if (<= c 90) true    ;; A-Z
      (if (>= c 97) (<= c 122) false))  ;; a-z
    false))

;; シンボル開始文字か
(defn is-symbol-start [c]
  (if (is-alpha-char c) true
    (if (== c 95) true     ;; _
      (if (== c 43) true   ;; +
        (if (== c 45) true ;; -
          (if (== c 42) true ;; *
            (if (== c 47) true ;; /
              (if (== c 61) true ;; =
                (if (== c 60) true ;; <
                  (if (== c 62) true ;; >
                    (if (== c 33) true ;; !
                      (if (== c 63) true ;; ?
                        (if (== c 38) true ;; &
                          (if (== c 37) true ;; %
                            (== c 126))))))))))))))) ;; ~

;; シンボル継続文字か
(defn is-symbol-char [c]
  (if (is-symbol-start c) true
    (if (is-digit-char c) true
      (if (== c 46) true ;; .
        (== c 45)))))    ;; -

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
                              (if (string-eq name "true") 13
                                (if (string-eq name "false") 14
                                  20)))))))))))))))))

;; === 数値読み取り ===

;; 数字の終端位置を返す
(defn scan-digits [src pos len]
  (if (>= pos len)
    pos
    (if (is-digit-char (string-char-at src pos))
      (scan-digits src (+ pos 1) len)
      pos)))

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
    (+ (* 99 1000000) pos)  ;; tok-eof
    (let [c (string-char-at src pos)]
      (if (== c 40) (+ (* 0 1000000) (+ pos 1))  ;; ( → LParen
        (if (== c 41) (+ (* 1 1000000) (+ pos 1))  ;; ) → RParen
          (if (== c 91) (+ (* 2 1000000) (+ pos 1))  ;; [ → LBracket
            (if (== c 93) (+ (* 3 1000000) (+ pos 1))  ;; ] → RBracket
              (if (== c 123) (+ (* 4 1000000) (+ pos 1))  ;; { → LBrace
                (if (== c 125) (+ (* 5 1000000) (+ pos 1))  ;; } → RBrace
                  (if (== c 58) (+ (* 50 1000000) (+ pos 1))  ;; : → Colon
                    (if (== c 124) (+ (* 52 1000000) (+ pos 1))  ;; | → Pipe
                      (if (== c 34) ;; " → String
                        (let [end (scan-string-end src (+ pos 1) len)]
                          (+ (* 12 1000000) end))
                        (if (is-digit-char c)
                          (let [end (scan-digits src (+ pos 1) len)]
                            (+ (* 10 1000000) end))  ;; Int
                          (if (is-symbol-start c)
                            (let [end (scan-symbol-end src (+ pos 1) len)
                                  name (substring src pos end)
                                  kind (classify-symbol name)]
                              (+ (* kind 1000000) end))
                            (+ (* 99 1000000) (+ pos 1)))))))))))))))) ;; unknown → skip

;; 全トークンを Vector に収集
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

;; ソース文字列をトークン化して種別の Vector を返す
(defn tokenize [src]
  (tokenize-loop src 0 (string-length src) (vector-new 16)))

;; エントリポイント (テスト用)
(defn main []
  (let [tokens (tokenize "(defn main [] 42)")
        len (vector-length tokens)]
    (do
      (print len)  ;; トークン数
      ;; 各トークンを出力
      (print (vector-get tokens 0))  ;; ( → 0 (LParen)
      (print (vector-get tokens 1))  ;; defn → 30 (Defn)
      (print (vector-get tokens 2))  ;; main → 20 (Symbol)
      (print (vector-get tokens 3))  ;; [ → 2 (LBracket)
      (print (vector-get tokens 4))  ;; ] → 3 (RBracket)
      (print (vector-get tokens 5))  ;; 42 → 10 (Int)
      (print (vector-get tokens 6))  ;; ) → 1 (RParen)
      (print (vector-get tokens 7))  ;; EOF → 99
      0)))
