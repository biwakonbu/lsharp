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
(defn is-symbol-start-punct-low-a [c]
  (if (== c 33) true ;; !
    (if (== c 37) true ;; %
      (if (== c 38) true ;; &
        (== c 42))))) ;; *

(defn is-symbol-start-punct-low-b [c]
  (if (== c 43) true ;; +
    (if (== c 45) true ;; -
      (== c 47)))) ;; /

(defn is-symbol-start-punct-high [c]
  (if (== c 60) true ;; <
    (if (== c 61) true ;; =
      (if (== c 62) true ;; >
        (== c 63))))) ;; ?

(defn is-symbol-start [c]
  (if (is-alpha-char c) true
    (if (== c 95) true ;; _
      (if (<= c 42)
        (is-symbol-start-punct-low-a c)
        (if (<= c 47)
          (is-symbol-start-punct-low-b c)
          (is-symbol-start-punct-high c))))))

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
    (do
      (root_push src)
      (let [result
        (if (== (string-char-at src pos) 10) ;; newline
          (+ pos 1)
          (skip-comment src (+ pos 1) len))]
        (do
          (root_pop)
          result)))))

;; 空白とコメントをスキップし、次のトークン開始位置を返す
(defn skip-ws-loop [src pos len]
  (if (>= pos len)
    pos
    (do
      (root_push src)
      (let [c (string-char-at src pos)]
        (let [result
          (if (is-ws c)
            (skip-ws-loop src (+ pos 1) len)
            (if (== c 59) ;; ;
              (let [end (skip-comment src (+ pos 1) len)]
                (skip-ws-loop src end len))
              pos))]
        (do
          (root_pop)
          result))))))

;; === キーワード判定 ===

;; シンボル名からトークン種別を返す
;; キーワードでなければ tok-symbol (20) を返す
(defn classify-symbol-d-rest [name]
  (if (string-eq name "do") 36
    (if (string-eq name "defmacro") 44
      20)))

(defn classify-symbol-d [name]
  (if (string-eq name "defn") 30
    (classify-symbol-d-rest name)))

(defn classify-symbol-l [name]
  (if (string-eq name "let") 31 20))

(defn classify-symbol-i-rest [name]
  (if (string-eq name "import") 38
    (if (string-eq name "impl") 41
      20)))

(defn classify-symbol-i [name]
  (if (string-eq name "if") 32
    (classify-symbol-i-rest name)))

(defn classify-symbol-m [name]
  (if (string-eq name "match") 33
    (if (string-eq name "module") 37
      20)))

(defn classify-symbol-t-rest [name]
  (if (string-eq name "trait") 40
    (if (string-eq name "true") 13
      20)))

(defn classify-symbol-t [name]
  (if (string-eq name "type") 34
    (classify-symbol-t-rest name)))

(defn classify-symbol-f [name]
  (if (string-eq name "fn") 35
    (if (string-eq name "false") 14
      20)))

(defn classify-symbol-w [name]
  (if (string-eq name "where") 42 20))

(defn classify-symbol-p [name]
  (if (string-eq name "private") 43 20))

(defn classify-symbol-o [name]
  (if (string-eq name "open") 49 20))

(defn classify-symbol-c [name]
  (if (string-eq name "constrained") 46
    (if (string-eq name "computation") 47
      20)))

(defn classify-symbol-b [name]
  (if (string-eq name "builder") 48 20))

(defn classify-symbol-r [name]
  (if (string-eq name "record") 39 20))

(defn classify-symbol-low-head [head name]
  (if (== head 98) ;; b
    (classify-symbol-b name)
    (if (== head 99) ;; c
      (classify-symbol-c name)
      (if (== head 100) ;; d
        (classify-symbol-d name)
        20))))

(defn classify-symbol-mid-head [head name]
  (if (== head 102) ;; f
    (classify-symbol-f name)
    (if (== head 105) ;; i
      (classify-symbol-i name)
      (if (== head 108) ;; l
        (classify-symbol-l name)
        (if (== head 109) ;; m
          (classify-symbol-m name)
          20)))))

(defn classify-symbol-high-head [head name]
  (if (== head 111) ;; o
    (classify-symbol-o name)
    (if (== head 112) ;; p
      (classify-symbol-p name)
      20)))

(defn classify-symbol-tail-head [head name]
  (if (== head 114) ;; r
    (classify-symbol-r name)
    (if (== head 116) ;; t
      (classify-symbol-t name)
      (if (== head 119) ;; w
        (classify-symbol-w name)
        20))))

(defn classify-symbol [name]
  (let [head (string-char-at name 0)]
    (if (<= head 100)
      (classify-symbol-low-head head name)
      (if (<= head 109)
        (classify-symbol-mid-head head name)
        (if (<= head 112)
          (classify-symbol-high-head head name)
          (classify-symbol-tail-head head name))))))

(defn symbol-hash-loop [src pos end acc]
  (if (>= pos end)
    acc
    (symbol-hash-loop src (+ pos 1) end
      (+ (string-char-at src pos) (* acc 31)))))

(defn symbol-hash [src start end]
  (symbol-hash-loop src start end 0))

(defn classify-symbol-hash [h]
  (if (= h 3211) 36 ;; do
    (if (= h 2843923108583) 44 ;; defmacro
      (if (= h 3079433) 30 ;; defn
        (if (= h 107035) 31 ;; let
          (if (= h 3110171557) 38 ;; import
            (if (= h 3236384) 41 ;; impl
              (if (= h 3357) 32 ;; if
                (if (= h 103668165) 33 ;; match
                  (if (= h 3226183276) 37 ;; module
                    (if (= h 110621198) 40 ;; trait
                      (if (= h 3569038) 13 ;; true
                        (if (= h 3575610) 34 ;; type
                          (if (= h 3272) 35 ;; fn
                            (if (= h 97196323) 14 ;; false
                              (if (= h 113097959) 42 ;; where
                                (if (= h 102764717443) 43 ;; private
                                  (if (= h 3417674) 49 ;; open
                                    (if (= h 84175086742643798) 46 ;; constrained
                                      (if (= h 84174152258849223) 47 ;; computation
                                        (if (= h 90425257883) 48 ;; builder
                                          (if (= h 3360058449) 39 ;; record
                                            20))))))))))))))))))))))

(defn classify-symbol-span [src start end]
  (classify-symbol-hash (symbol-hash src start end)))

;; === 数値読み取り ===

;; 数字を 1 文字だけ前進させる
(defn scan-digit-step [src pos len]
  (if (>= pos len)
    pos
    (if (is-digit-char (string-char-at src pos))
      (+ pos 1)
      pos)))

;; 数字を最大 2 文字まとめて前進させる
(defn scan-digits-step-2 [src pos len]
  (let [pos1 (scan-digit-step src pos len)]
    (if (= pos1 pos)
      pos
      (scan-digit-step src pos1 len))))

(defn scan-digits-step-4 [src pos len]
  (let [pos1 (scan-digits-step-2 src pos len)]
    (if (= pos1 pos)
      pos
      (scan-digits-step-2 src pos1 len))))

;; 数字を最大 8 文字まとめて前進させる
(defn scan-digits-step-8 [src pos len]
  (let [pos1 (scan-digits-step-4 src pos len)]
    (if (= pos1 pos)
      pos
      (scan-digits-step-4 src pos1 len))))

;; 数字の終端位置を返す
(defn scan-digits [src pos len]
  (do
    (root_push src)
    (let [next (scan-digits-step-8 src pos len)
      result
        (if (= next pos)
          pos
          (scan-digits src next len))]
      (do
        (root_pop)
        result))))

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

;; シンボル文字を 1 文字だけ前進させる
(defn scan-symbol-step [src pos len]
  (if (>= pos len)
    pos
    (if (is-symbol-char (string-char-at src pos))
      (+ pos 1)
      pos)))

;; シンボル文字を最大 8 文字まとめて前進させる
(defn scan-symbol-end-step-2 [src pos len]
  (let [pos1 (scan-symbol-step src pos len)]
    (if (= pos1 pos)
      pos
      (scan-symbol-step src pos1 len))))

(defn scan-symbol-end-step-4 [src pos len]
  (let [pos1 (scan-symbol-end-step-2 src pos len)]
    (if (= pos1 pos)
      pos
      (scan-symbol-end-step-2 src pos1 len))))

(defn scan-symbol-end-step-8 [src pos len]
  (let [pos1 (scan-symbol-end-step-4 src pos len)]
    (if (= pos1 pos)
      pos
      (scan-symbol-end-step-4 src pos1 len))))

(defn scan-symbol-end-step-16 [src pos len]
  (let [pos1 (scan-symbol-end-step-8 src pos len)]
    (if (= pos1 pos)
      pos
      (scan-symbol-end-step-8 src pos1 len))))

(defn scan-symbol-end-step-32 [src pos len]
  (let [pos1 (scan-symbol-end-step-16 src pos len)]
    (if (= pos1 pos)
      pos
      (scan-symbol-end-step-16 src pos1 len))))

;; シンボルの終端位置を返す
(defn scan-symbol-end [src pos len]
  (do
    (root_push src)
    (let [next (scan-symbol-end-step-32 src pos len)
      result
        (if (= next pos)
          pos
          (scan-symbol-end src next len))]
      (do
        (root_pop)
        result))))

;; === 文字列読み取り ===

;; 文字列の終端 (閉じ引用符の次の位置) を返す
(defn scan-string-end [src pos len]
  (if (>= pos len)
    pos ;; 未終端 (エラーは呼び出し側で)
    (do
      (root_push src)
      (let [c (string-char-at src pos)]
        (let [result
          (if (== c 34) ;; "
            (+ pos 1)
            (if (== c 92) ;; \  (エスケープ)
              (scan-string-end src (+ pos 2) len)
              (scan-string-end src (+ pos 1) len)))]
        (do
          (root_pop)
          result))))))

;; === メインのトークナイザー ===

;; トークンを1つ読み取り、(kind, end_pos) を packed scalar で返す。
;; 未終端文字列の末尾 escape は source-len + 1 を返せるため、それより大きい radix を使う。
(defn lex-result-base [source-len] (+ source-len 2))

(defn make-lex-result [kind end-pos source-len]
  (+ (* kind (lex-result-base source-len)) end-pos))

(defn lex-result-kind [result source-len]
  (/ result (lex-result-base source-len)))

(defn lex-result-end [result source-len]
  (- result (* (lex-result-kind result source-len) (lex-result-base source-len))))

(defn lex-minus-token [src pos len]
  (if (< (+ pos 1) len)
    (if (== (string-char-at src (+ pos 1)) 62) ;; >
      (make-lex-result 51 (+ pos 2) len) ;; -> -> Arrow
      (if (is-digit-char (string-char-at src (+ pos 1)))
        (lex-number-token src pos len)
        (let [end (scan-symbol-end src (+ pos 1) len)]
          (let [kind (classify-symbol-span src pos end)]
            (make-lex-result kind end len)))))
    (make-lex-result 20 (+ pos 1) len)))

(defn lex-number-token [src pos len]
  (let [int-end (scan-digits src (+ pos 1) len)]
    (let [end (scan-number-end src int-end len)]
      (if (> end int-end)
        (make-lex-result 11 end len) ;; Float
        (make-lex-result 10 end len))))) ;; Int

(defn lex-symbol-token [src pos len]
  (let [end (scan-symbol-end src (+ pos 1) len)]
    (let [kind (classify-symbol-span src pos end)]
      (make-lex-result kind end len))))

(defn lex-one-structured-rest [src pos len c]
  (if (is-digit-char c)
    (lex-number-token src pos len)
    (if (is-symbol-start c)
      (lex-symbol-token src pos len)
      (make-lex-result 99 (+ pos 1) len)))) ;; unknown -> skip

(defn lex-one-structured [src pos len c]
  (if (== c 34) ;; " -> String
    (let [end (scan-string-end src (+ pos 1) len)]
      (make-lex-result 12 end len))
    (if (== c 45) ;; - -> Arrow / Symbol
      (lex-minus-token src pos len)
      (lex-one-structured-rest src pos len c))))

(defn lex-tilde-token [src pos len]
  (if (< (+ pos 1) len)
    (if (== (string-char-at src (+ pos 1)) 64) ;; @
      (make-lex-result 56 (+ pos 2) len) ;; ~@ -> SpliceUnquote
      (make-lex-result 55 (+ pos 1) len)) ;; ~ -> Unquote
    (make-lex-result 55 (+ pos 1) len)))

(defn lex-one-meta-special-rest [src pos len c]
  (if (== c 35) (make-lex-result 57 (+ pos 1) len) ;; # -> Hash
    (if (== c 64) (make-lex-result 58 (+ pos 1) len) ;; @ -> At
      (lex-one-structured src pos len c))))

(defn lex-one-meta-special [src pos len c]
  (if (== c 39) (make-lex-result 54 (+ pos 1) len) ;; ' -> Quote
    (if (== c 126) ;; ~ -> Unquote / SpliceUnquote
      (lex-tilde-token src pos len)
      (lex-one-meta-special-rest src pos len c))))

(defn lex-one-meta [src pos len c]
  (if (== c 58) (make-lex-result 50 (+ pos 1) len) ;; : -> Colon
    (if (== c 124) (make-lex-result 52 (+ pos 1) len) ;; | -> Pipe
      (if (== c 46) (make-lex-result 53 (+ pos 1) len) ;; . -> Dot
        (lex-one-meta-special src pos len c)))))

(defn lex-one-delim-rest [src pos len c]
  (if (== c 123) (make-lex-result 4 (+ pos 1) len) ;; { -> LBrace
    (if (== c 125) (make-lex-result 5 (+ pos 1) len) ;; } -> RBrace
      (lex-one-meta src pos len c))))

(defn lex-one-delim [src pos len c]
  (if (== c 40) (make-lex-result 0 (+ pos 1) len) ;; ( -> LParen
    (if (== c 41) (make-lex-result 1 (+ pos 1) len) ;; ) -> RParen
      (if (== c 91) (make-lex-result 2 (+ pos 1) len) ;; [ -> LBracket
        (if (== c 93) (make-lex-result 3 (+ pos 1) len) ;; ] -> RBracket
          (lex-one-delim-rest src pos len c))))))

(defn lex-one [src pos len]
  (if (>= pos len)
    (make-lex-result 99 pos len) ;; tok-eof
    (let [c (string-char-at src pos)]
      (lex-one-delim src pos len c))))

;; selfhost stage2 の stack でも大入力を扱えるよう、
;; tokenization は複数段の helper でまとめて処理して再帰深さを抑える。
(defn make-tokenize-state [done next-pos next-tokens]
  (do
    (root_push next-tokens)
    (let [state0 (vector-push (vector-new 4) done)]
      (do
        (root_push state0)
        (let [state1 (vector-push state0 next-pos)]
          (do
            (root_push state1)
            (let [state (vector-push state1 next-tokens)]
              (do
                (root_pop)
                (root_pop)
                (root_pop)
                state))))))))

(defn append-span-token [tokens kind start end]
  (do
    (root_push tokens)
    (let [with-kind (vector-push tokens kind)]
      (do
        (root_push with-kind)
        (let [with-start (vector-push with-kind start)]
          (do
            (root_push with-start)
            (let [updated (vector-push with-start end)]
              (do
                (root_pop)
                (root_pop)
                (root_pop)
                updated))))))))

(defn make-tokenize-state-from-appended-tokens [done next-pos next-tokens]
  (do
    (root_push next-tokens)
    (let [state (make-tokenize-state done next-pos next-tokens)]
      (do
        (root_pop)
        state))))

(defn append-span-token-state [tokens done next-pos kind start end]
  (let [next-tokens (append-span-token tokens kind start end)]
    (make-tokenize-state-from-appended-tokens done next-pos next-tokens)))

(defn append-span-token-state-end [tokens done kind start end]
  (let [next-tokens (append-span-token tokens kind start end)]
    (make-tokenize-state-from-appended-tokens done end next-tokens)))

(defn append-lex-result-state [tokens result start source-len]
  (let [kind (lex-result-kind result source-len)]
    (let [end-pos (lex-result-end result source-len)]
      (if (== kind 99)
        (let [next-tokens (append-span-token tokens 99 start start)]
          (make-tokenize-state-from-appended-tokens 1 start next-tokens))
        (let [next-tokens (append-span-token tokens kind start end-pos)]
          (make-tokenize-state-from-appended-tokens 0 end-pos next-tokens))))))

(defn append-lex-result-state-rst [result start tokens source-len]
  (append-lex-result-state tokens result start source-len))

;; === T2-1: 値つきトークン (kind, start, end) 3つ組 ===

;; 全トークンを (kind, start, end) 3つ組の Vector に収集
;; 結果の Vector は [kind0, start0, end0, kind1, start1, end1, ...] のフラット構造
(defn tokenize-spans-step [src pos len tokens]
  (do
    (root_push src)
    (root_push tokens)
    (let [ws-pos (skip-ws-loop src pos len)]
      (let [state
        (if (>= ws-pos len)
          ;; EOF トークン: (99, pos, pos)
          (append-span-token-state tokens 1 ws-pos 99 ws-pos ws-pos)
          (let [result (lex-one src ws-pos len)]
            (let [kind (lex-result-kind result len)]
              (let [end-pos (lex-result-end result len)]
                (if (== kind 99)
                  (append-span-token-state tokens 1 ws-pos 99 ws-pos ws-pos)
                  (append-span-token-state tokens 0 end-pos kind ws-pos end-pos))))))]
        (do
          (root_push state)
          (root_pop)
          (root_pop)
          (root_pop)
          state)))))

;; 1 回の helper 呼び出しで複数トークンを進め、selfhost 実行時の再帰フレーム数をさらに抑える。
(defn tokenize-spans-step-2 [src pos len tokens]
  (do
    (root_push src)
    (root_push tokens)
    (let [step1 (tokenize-spans-step src pos len tokens)]
      (do
        (root_push step1)
        (let [done (vector-get step1 0)]
          (let [next-pos (vector-get step1 1)]
            (let [next-tokens (vector-get step1 2)]
              (do
                (root_push next-tokens)
                (let [result
                  (if (= done 1)
                    step1
                    (tokenize-spans-step src next-pos len next-tokens))]
                  (do
                    (root_pop)
                    (root_pop)
                    (root_pop)
                    (root_pop)
                    result))))))))))

(defn tokenize-spans-step-4 [src pos len tokens]
  (do
    (root_push src)
    (root_push tokens)
    (let [step1 (tokenize-spans-step-2 src pos len tokens)]
      (do
        (root_push step1)
        (let [done (vector-get step1 0)]
          (let [next-pos (vector-get step1 1)]
            (let [next-tokens (vector-get step1 2)]
              (do
                (root_push next-tokens)
                (let [result
                  (if (= done 1)
                    step1
                    (tokenize-spans-step-2 src next-pos len next-tokens))]
                  (do
                    (root_pop)
                    (root_pop)
                    (root_pop)
                    (root_pop)
                    result))))))))))

(defn tokenize-spans-step-8 [src pos len tokens]
  (do
    (root_push src)
    (root_push tokens)
    (let [step1 (tokenize-spans-step-4 src pos len tokens)]
      (do
        (root_push step1)
        (let [done (vector-get step1 0)]
          (let [next-pos (vector-get step1 1)]
            (let [next-tokens (vector-get step1 2)]
              (do
                (root_push next-tokens)
                (let [result
                  (if (= done 1)
                    step1
                    (tokenize-spans-step-4 src next-pos len next-tokens))]
                  (do
                    (root_pop)
                    (root_pop)
                    (root_pop)
                    (root_pop)
                    result))))))))))

(defn tokenize-spans-step-512-state-loop [src len state remaining]
  (do
    (root_push src)
    (root_push state)
    (let [done (vector-get state 0)]
      (if (= done 1)
        (do
          (root_pop)
          (root_pop)
          state)
        (if (<= remaining 1)
          (do
            (root_pop)
            (root_pop)
            state)
          (let [next-pos (vector-get state 1)]
            (let [next-tokens (vector-get state 2)]
              (let [step (tokenize-spans-step-2 src next-pos len next-tokens)]
                (do
                  (root_push step)
                  (root_pop)
                  (root_pop)
                  (root_pop)
                  (tokenize-spans-step-512-state-loop src len step (- remaining 1)))))))))))

(defn tokenize-spans-step-512-loop-bounded [src pos len tokens remaining]
  (tokenize-spans-step-512-state-loop
    src
    len
    (make-tokenize-state 0 pos tokens)
    remaining))

;; stage2 compiler 自身でも大きい入力を裁けるよう、
;; 8 トークン束を 64 回回す bounded loop でまとめて進める。
(defn tokenize-spans-step-512 [src pos len tokens]
  (tokenize-spans-step-512-loop-bounded src pos len tokens 256))

(defn tokenize-spans-outer-loop-bounded [src pos len tokens remaining]
  (do
    (root_push src)
    (root_push tokens)
    (let [step (tokenize-spans-step-512 src pos len tokens)]
      (do
        (root_push step)
        (let [done (vector-get step 0)]
          (let [next-pos (vector-get step 1)]
            (let [next-tokens (vector-get step 2)]
              (do
                (root_push next-tokens)
                (if (= done 1)
                  (let [result (make-tokenize-state 1 next-pos next-tokens)]
                    (do
                      (root_pop)
                      (root_pop)
                      (root_pop)
                      (root_pop)
                      result))
                  (if (<= remaining 1)
                    (let [result (make-tokenize-state 0 next-pos next-tokens)]
                      (do
                        (root_pop)
                        (root_pop)
                        (root_pop)
                        (root_pop)
                        result))
                    (do
                      (root_pop)
                      (root_pop)
                      (root_pop)
                      (root_pop)
                      (tokenize-spans-outer-loop-bounded src next-pos len next-tokens (- remaining 1)))))))))))))

(defn tokenize-spans-loop [src pos len tokens]
  (do
    (root_push src)
    (root_push tokens)
    (let [batch (tokenize-spans-outer-loop-bounded src pos len tokens 256)]
      (do
        (root_push batch)
        (let [done (vector-get batch 0)
          next-pos (vector-get batch 1)
          next-tokens (vector-get batch 2)]
          (do
            (root_push next-tokens)
            (if (= done 1)
              (do
                (root_pop)
                (root_pop)
                (root_pop)
                (root_pop)
                next-tokens)
              (do
                (root_pop)
                (root_pop)
                (root_pop)
                (root_pop)
                (tokenize-spans-loop src next-pos len next-tokens)))))))))

;; ソース文字列をトークン化して (kind, start, end) 3つ組を返す
(defn tokenize-with-spans [src]
  (do
    (root_push src)
    (let [tokens (vector-new 32)]
      (do
        (root_push tokens)
        (let [result (tokenize-spans-loop src 0 (string-length src) tokens)]
          (do
            (root_pop)
            (root_pop)
            result))))))

;; === トークン値の取得 ===

;; 数字文字列を整数に変換 (再帰ヘルパー)
(defn parse-int-from-string [src pos end acc]
  (if (>= pos end)
    acc
    (let [digit (- (string-char-at src pos) 48)]
      (parse-int-from-string src (+ pos 1) end (+ (* acc 10) digit)))))

;; デモ用エントリポイント (テスト用)
(defn demo-main []
  (let [legacy-spans (tokenize-with-spans "(defn main [] 42)")
    len (/ (vector-length legacy-spans) 3)]
    (do
      (print len) ;; トークン数
      ;; 各トークンを出力
      (print (vector-get legacy-spans 0)) ;; ( -> 0 (LParen)
      (print (vector-get legacy-spans 3)) ;; defn -> 30 (Defn)
      (print (vector-get legacy-spans 6)) ;; main -> 20 (Symbol)
      (print (vector-get legacy-spans 9)) ;; [ -> 2 (LBracket)
      (print (vector-get legacy-spans 12)) ;; ] -> 3 (RBracket)
      (print (vector-get legacy-spans 15)) ;; 42 -> 10 (Int)
      (print (vector-get legacy-spans 18)) ;; ) -> 1 (RParen)
      (print (vector-get legacy-spans 21)) ;; EOF -> 99

      ;; T2-1: 値つきトークンのテスト
      (let [spans (tokenize-with-spans "(+ 42 x)")
        n (/ (vector-length spans) 3)]
        (do
          (print n) ;; トークン数 = 6
          (print (vector-get spans 0)) ;; ( -> 0 (LParen)
          (print (vector-get spans 3)) ;; + -> 20 (Symbol)
          (print (vector-get spans 6)) ;; 42 -> 10 (Int)
          (print (vector-get spans 9)) ;; x -> 20 (Symbol)
          (print (vector-get spans 12)) ;; ) -> 1 (RParen)
          (print (vector-get spans 15)) ;; EOF -> 99
          ;; 整数値の取得
          (print (parse-int-from-string "(+ 42 x)" (vector-get spans 7) (vector-get spans 8) 0)) ;; 42
          ;; スパン情報
          (print (vector-get spans 4)) ;; 1 (+ の開始位置)
          (print (vector-get spans 5)) ;; 2 (+ の終了位置)
          0))
      0)))
