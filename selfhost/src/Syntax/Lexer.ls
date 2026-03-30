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
  (let [head (string-char-at name 0)]
    (if (== head 100) ;; d
      (if (string-eq name "defn") 30
        (if (string-eq name "do") 36
          (if (string-eq name "defmacro") 44
            20)))
      (if (== head 108) ;; l
        (if (string-eq name "let") 31 20)
        (if (== head 105) ;; i
          (if (string-eq name "if") 32
            (if (string-eq name "import") 38
              (if (string-eq name "impl") 41
                20)))
          (if (== head 109) ;; m
            (if (string-eq name "match") 33
              (if (string-eq name "module") 37
                20))
            (if (== head 116) ;; t
              (if (string-eq name "type") 34
                (if (string-eq name "trait") 40
                  (if (string-eq name "true") 13
                    20)))
              (if (== head 102) ;; f
                (if (string-eq name "fn") 35
                  (if (string-eq name "false") 14
                    20))
                (if (== head 119) ;; w
                  (if (string-eq name "where") 42 20)
                  (if (== head 112) ;; p
                    (if (string-eq name "private") 43 20)
                    (if (== head 111) ;; o
                      (if (string-eq name "open") 49 20)
                      (if (== head 99) ;; c
                        (if (string-eq name "constrained") 46
                          (if (string-eq name "computation") 47
                            20))
                        (if (== head 98) ;; b
                          (if (string-eq name "builder") 48 20)
                          (if (== head 114) ;; r
                            (if (string-eq name "record") 39 20)
                            20))))))))))))))

;; === 数値読み取り ===

;; 数字を 1 文字だけ前進させる
(defn scan-digit-step [src pos len]
  (if (>= pos len)
    pos
    (if (is-digit-char (string-char-at src pos))
      (+ pos 1)
      pos)))

;; 数字を最大 8 文字まとめて前進させる
(defn scan-digits-step-8 [src pos len]
  (let [pos1 (scan-digit-step src pos len)]
    (if (= pos1 pos)
      pos
      (let [pos2 (scan-digit-step src pos1 len)]
        (if (= pos2 pos1)
          pos1
          (let [pos3 (scan-digit-step src pos2 len)]
            (if (= pos3 pos2)
              pos2
              (let [pos4 (scan-digit-step src pos3 len)]
                (if (= pos4 pos3)
                  pos3
                  (let [pos5 (scan-digit-step src pos4 len)]
                    (if (= pos5 pos4)
                      pos4
                      (let [pos6 (scan-digit-step src pos5 len)]
                        (if (= pos6 pos5)
                          pos5
                          (let [pos7 (scan-digit-step src pos6 len)]
                            (if (= pos7 pos6)
                              pos6
                              (scan-digit-step src pos7 len))))))))))))))))

;; 数字の終端位置を返す
(defn scan-digits [src pos len]
  (let [next (scan-digits-step-8 src pos len)]
    (if (= next pos)
      pos
      (scan-digits src next len))))

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
(defn scan-symbol-end-step-8 [src pos len]
  (let [pos1 (scan-symbol-step src pos len)]
    (if (= pos1 pos)
      pos
      (let [pos2 (scan-symbol-step src pos1 len)]
        (if (= pos2 pos1)
          pos1
          (let [pos3 (scan-symbol-step src pos2 len)]
            (if (= pos3 pos2)
              pos2
              (let [pos4 (scan-symbol-step src pos3 len)]
                (if (= pos4 pos3)
                  pos3
                  (let [pos5 (scan-symbol-step src pos4 len)]
                    (if (= pos5 pos4)
                      pos4
                      (let [pos6 (scan-symbol-step src pos5 len)]
                        (if (= pos6 pos5)
                          pos5
                          (let [pos7 (scan-symbol-step src pos6 len)]
                            (if (= pos7 pos6)
                              pos6
                              (scan-symbol-step src pos7 len))))))))))))))))

;; シンボルの終端位置を返す
(defn scan-symbol-end [src pos len]
  (let [next (scan-symbol-end-step-8 src pos len)]
    (if (= next pos)
      pos
      (scan-symbol-end src next len))))

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

;; selfhost stage2 の stack でも大入力を扱えるよう、
;; tokenization は複数段の helper でまとめて処理して再帰深さを抑える。
(defn make-tokenize-state [done next-pos next-tokens]
  (vector-push
    (vector-push
      (vector-push (vector-new 4) done)
      next-pos)
    next-tokens))

(defn append-kind-token [tokens kind]
  (vector-push tokens kind))

(defn append-span-token [tokens kind start end]
  (vector-push (vector-push (vector-push tokens kind) start) end))

(defn tokenize-step [src pos len tokens]
  (let [ws-pos (skip-ws-loop src pos len)]
    (if (>= ws-pos len)
      (make-tokenize-state 1 ws-pos (append-kind-token tokens 99))
      (let [result (lex-one src ws-pos len)
        kind (/ result 1000000)
        end-pos (- result (* kind 1000000))]
        (if (== kind 99)
          (make-tokenize-state 1 ws-pos (append-kind-token tokens 99))
          (make-tokenize-state 0 end-pos (append-kind-token tokens kind)))))))

;; 全トークンを Vector に収集 (kind のみ、後方互換)
(defn tokenize-loop-1 [src pos len tokens]
  (let [step (tokenize-step src pos len tokens)
    done (vector-get step 0)]
    (if (= done 1)
      (vector-get step 2)
      (tokenize-loop-2 src (vector-get step 1) len (vector-get step 2)))))

(defn tokenize-loop-2 [src pos len tokens]
  (let [step (tokenize-step src pos len tokens)
    done (vector-get step 0)]
    (if (= done 1)
      (vector-get step 2)
      (tokenize-loop-3 src (vector-get step 1) len (vector-get step 2)))))

(defn tokenize-loop-3 [src pos len tokens]
  (let [step (tokenize-step src pos len tokens)
    done (vector-get step 0)]
    (if (= done 1)
      (vector-get step 2)
      (tokenize-loop-4 src (vector-get step 1) len (vector-get step 2)))))

(defn tokenize-loop-4 [src pos len tokens]
  (let [step (tokenize-step src pos len tokens)
    done (vector-get step 0)]
    (if (= done 1)
      (vector-get step 2)
      (tokenize-loop-5 src (vector-get step 1) len (vector-get step 2)))))

(defn tokenize-loop-5 [src pos len tokens]
  (let [step (tokenize-step src pos len tokens)
    done (vector-get step 0)]
    (if (= done 1)
      (vector-get step 2)
      (tokenize-loop-6 src (vector-get step 1) len (vector-get step 2)))))

(defn tokenize-loop-6 [src pos len tokens]
  (let [step (tokenize-step src pos len tokens)
    done (vector-get step 0)]
    (if (= done 1)
      (vector-get step 2)
      (tokenize-loop-7 src (vector-get step 1) len (vector-get step 2)))))

(defn tokenize-loop-7 [src pos len tokens]
  (let [step (tokenize-step src pos len tokens)
    done (vector-get step 0)]
    (if (= done 1)
      (vector-get step 2)
      (tokenize-loop-8 src (vector-get step 1) len (vector-get step 2)))))

(defn tokenize-loop-8 [src pos len tokens]
  (let [step (tokenize-step src pos len tokens)
    done (vector-get step 0)]
    (if (= done 1)
      (vector-get step 2)
      (tokenize-loop src (vector-get step 1) len (vector-get step 2)))))

(defn tokenize-loop [src pos len tokens]
  (tokenize-loop-1 src pos len tokens))

;; ソース文字列をトークン化して種別の Vector を返す (後方互換)
(defn tokenize [src]
  (tokenize-loop src 0 (string-length src) (vector-new 16)))

;; === T2-1: 値つきトークン (kind, start, end) 3つ組 ===

;; 全トークンを (kind, start, end) 3つ組の Vector に収集
;; 結果の Vector は [kind0, start0, end0, kind1, start1, end1, ...] のフラット構造
(defn tokenize-spans-step [src pos len tokens]
  (let [ws-pos (skip-ws-loop src pos len)]
    (if (>= ws-pos len)
      ;; EOF トークン: (99, pos, pos)
      (make-tokenize-state 1 ws-pos (append-span-token tokens 99 ws-pos ws-pos))
      (let [result (lex-one src ws-pos len)
        kind (/ result 1000000)
        end-pos (- result (* kind 1000000))]
        (if (== kind 99)
          (make-tokenize-state 1 ws-pos (append-span-token tokens 99 ws-pos ws-pos))
          (make-tokenize-state 0 end-pos (append-span-token tokens kind ws-pos end-pos)))))))

;; 1 回の helper 呼び出しで複数トークンを進め、selfhost 実行時の再帰フレーム数をさらに抑える。
(defn tokenize-spans-step-8 [src pos len tokens]
  (let [step1 (tokenize-spans-step src pos len tokens)
    done1 (vector-get step1 0)]
    (if (= done1 1)
      step1
      (let [step2 (tokenize-spans-step src (vector-get step1 1) len (vector-get step1 2))
        done2 (vector-get step2 0)]
        (if (= done2 1)
          step2
          (let [step3 (tokenize-spans-step src (vector-get step2 1) len (vector-get step2 2))
            done3 (vector-get step3 0)]
            (if (= done3 1)
              step3
              (let [step4 (tokenize-spans-step src (vector-get step3 1) len (vector-get step3 2))
                done4 (vector-get step4 0)]
                (if (= done4 1)
                  step4
                  (let [step5 (tokenize-spans-step src (vector-get step4 1) len (vector-get step4 2))
                    done5 (vector-get step5 0)]
                    (if (= done5 1)
                      step5
                      (let [step6 (tokenize-spans-step src (vector-get step5 1) len (vector-get step5 2))
                        done6 (vector-get step6 0)]
                        (if (= done6 1)
                          step6
                          (let [step7 (tokenize-spans-step src (vector-get step6 1) len (vector-get step6 2))
                            done7 (vector-get step7 0)]
                            (if (= done7 1)
                              step7
                              (let [step8 (tokenize-spans-step src (vector-get step7 1) len (vector-get step7 2))
                                done8 (vector-get step8 0)]
                                 step8))))))))))))))))

(defn make-advance-state [done next-pos]
  (vector-push
    (vector-push (vector-new 2) done)
    next-pos))

;; chunk 境界探索用: token を蓄積せずに「どこまで進めるか」だけ返す。
(defn advance-spans-step [src pos len]
  (let [ws-pos (skip-ws-loop src pos len)]
    (if (>= ws-pos len)
      (make-advance-state 1 ws-pos)
      (let [result (lex-one src ws-pos len)
        kind (/ result 1000000)
        end-pos (- result (* kind 1000000))]
        (if (== kind 99)
          (make-advance-state 1 ws-pos)
          (make-advance-state 0 end-pos))))))

(defn advance-spans-step-8 [src pos len]
  (let [step1 (advance-spans-step src pos len)
    done1 (vector-get step1 0)]
    (if (= done1 1)
      step1
      (let [step2 (advance-spans-step src (vector-get step1 1) len)
        done2 (vector-get step2 0)]
        (if (= done2 1)
          step2
          (let [step3 (advance-spans-step src (vector-get step2 1) len)
            done3 (vector-get step3 0)]
            (if (= done3 1)
              step3
              (let [step4 (advance-spans-step src (vector-get step3 1) len)
                done4 (vector-get step4 0)]
                (if (= done4 1)
                  step4
                  (let [step5 (advance-spans-step src (vector-get step4 1) len)
                    done5 (vector-get step5 0)]
                    (if (= done5 1)
                      step5
                      (let [step6 (advance-spans-step src (vector-get step5 1) len)
                        done6 (vector-get step6 0)]
                        (if (= done6 1)
                          step6
                          (let [step7 (advance-spans-step src (vector-get step6 1) len)
                            done7 (vector-get step7 0)]
                            (if (= done7 1)
                              step7
                              (advance-spans-step src (vector-get step7 1) len))))))))))))))))

(defn advance-spans-step-16 [src pos len]
  (let [step1 (advance-spans-step-8 src pos len)
    done1 (vector-get step1 0)]
    (if (= done1 1)
      step1
      (advance-spans-step-8 src (vector-get step1 1) len))))

(defn advance-spans-step-32 [src pos len]
  (let [step1 (advance-spans-step-16 src pos len)
    done1 (vector-get step1 0)]
    (if (= done1 1)
      step1
      (advance-spans-step-16 src (vector-get step1 1) len))))

(defn advance-spans-step-64 [src pos len]
  (let [step1 (advance-spans-step-32 src pos len)
    done1 (vector-get step1 0)]
    (if (= done1 1)
      step1
      (advance-spans-step-32 src (vector-get step1 1) len))))

(defn advance-spans-step-128 [src pos len]
  (let [step1 (advance-spans-step-64 src pos len)
    done1 (vector-get step1 0)]
    (if (= done1 1)
      step1
      (advance-spans-step-64 src (vector-get step1 1) len))))

(defn advance-spans-step-256 [src pos len]
  (let [step1 (advance-spans-step-128 src pos len)
    done1 (vector-get step1 0)]
    (if (= done1 1)
      step1
      (advance-spans-step-128 src (vector-get step1 1) len))))

;; stage2 compiler 自身でも大きい入力を裁けるよう、8 トークン束をさらに 8 回まとめる。
(defn tokenize-spans-step-64 [src pos len tokens]
  (let [step1 (tokenize-spans-step-8 src pos len tokens)
    done1 (vector-get step1 0)]
    (if (= done1 1)
      step1
      (let [step2 (tokenize-spans-step-8 src (vector-get step1 1) len (vector-get step1 2))
        done2 (vector-get step2 0)]
        (if (= done2 1)
          step2
          (let [step3 (tokenize-spans-step-8 src (vector-get step2 1) len (vector-get step2 2))
            done3 (vector-get step3 0)]
            (if (= done3 1)
              step3
              (let [step4 (tokenize-spans-step-8 src (vector-get step3 1) len (vector-get step3 2))
                done4 (vector-get step4 0)]
                (if (= done4 1)
                  step4
                  (let [step5 (tokenize-spans-step-8 src (vector-get step4 1) len (vector-get step4 2))
                    done5 (vector-get step5 0)]
                    (if (= done5 1)
                      step5
                      (let [step6 (tokenize-spans-step-8 src (vector-get step5 1) len (vector-get step5 2))
                        done6 (vector-get step6 0)]
                        (if (= done6 1)
                          step6
                          (let [step7 (tokenize-spans-step-8 src (vector-get step6 1) len (vector-get step6 2))
                            done7 (vector-get step7 0)]
                            (if (= done7 1)
                              step7
                              (let [step8 (tokenize-spans-step-8 src (vector-get step7 1) len (vector-get step7 2))
                                done8 (vector-get step8 0)]
                                step8))))))))))))))))

;; 大きい単一ファイル入力向けに、64 トークン束をさらに 8 回まとめる。
(defn tokenize-spans-step-512 [src pos len tokens]
  (let [step1 (tokenize-spans-step-64 src pos len tokens)
    done1 (vector-get step1 0)]
    (if (= done1 1)
      step1
      (let [step2 (tokenize-spans-step-64 src (vector-get step1 1) len (vector-get step1 2))
        done2 (vector-get step2 0)]
        (if (= done2 1)
          step2
          (let [step3 (tokenize-spans-step-64 src (vector-get step2 1) len (vector-get step2 2))
            done3 (vector-get step3 0)]
            (if (= done3 1)
              step3
              (let [step4 (tokenize-spans-step-64 src (vector-get step3 1) len (vector-get step3 2))
                done4 (vector-get step4 0)]
                (if (= done4 1)
                  step4
                  (let [step5 (tokenize-spans-step-64 src (vector-get step4 1) len (vector-get step4 2))
                    done5 (vector-get step5 0)]
                    (if (= done5 1)
                      step5
                      (let [step6 (tokenize-spans-step-64 src (vector-get step5 1) len (vector-get step5 2))
                        done6 (vector-get step6 0)]
                        (if (= done6 1)
                          step6
                          (let [step7 (tokenize-spans-step-64 src (vector-get step6 1) len (vector-get step6 2))
                            done7 (vector-get step7 0)]
                            (if (= done7 1)
                              step7
                              (let [step8 (tokenize-spans-step-64 src (vector-get step7 1) len (vector-get step7 2))
                                done8 (vector-get step8 0)]
                                step8))))))))))))))))

(defn tokenize-spans-loop-1 [src pos len tokens] (let [step (tokenize-spans-step-512 src pos len tokens) done (vector-get step 0)] (if (= done 1) (vector-get step 2) (tokenize-spans-loop-2 src (vector-get step 1) len (vector-get step 2)))))
(defn tokenize-spans-loop-2 [src pos len tokens] (let [step (tokenize-spans-step-512 src pos len tokens) done (vector-get step 0)] (if (= done 1) (vector-get step 2) (tokenize-spans-loop-3 src (vector-get step 1) len (vector-get step 2)))))
(defn tokenize-spans-loop-3 [src pos len tokens] (let [step (tokenize-spans-step-512 src pos len tokens) done (vector-get step 0)] (if (= done 1) (vector-get step 2) (tokenize-spans-loop-4 src (vector-get step 1) len (vector-get step 2)))))
(defn tokenize-spans-loop-4 [src pos len tokens] (let [step (tokenize-spans-step-512 src pos len tokens) done (vector-get step 0)] (if (= done 1) (vector-get step 2) (tokenize-spans-loop-5 src (vector-get step 1) len (vector-get step 2)))))
(defn tokenize-spans-loop-5 [src pos len tokens] (let [step (tokenize-spans-step-512 src pos len tokens) done (vector-get step 0)] (if (= done 1) (vector-get step 2) (tokenize-spans-loop-6 src (vector-get step 1) len (vector-get step 2)))))
(defn tokenize-spans-loop-6 [src pos len tokens] (let [step (tokenize-spans-step-512 src pos len tokens) done (vector-get step 0)] (if (= done 1) (vector-get step 2) (tokenize-spans-loop-7 src (vector-get step 1) len (vector-get step 2)))))
(defn tokenize-spans-loop-7 [src pos len tokens] (let [step (tokenize-spans-step-512 src pos len tokens) done (vector-get step 0)] (if (= done 1) (vector-get step 2) (tokenize-spans-loop-8 src (vector-get step 1) len (vector-get step 2)))))
(defn tokenize-spans-loop-8 [src pos len tokens] (let [step (tokenize-spans-step-512 src pos len tokens) done (vector-get step 0)] (if (= done 1) (vector-get step 2) (tokenize-spans-loop-9 src (vector-get step 1) len (vector-get step 2)))))
(defn tokenize-spans-loop-9 [src pos len tokens] (let [step (tokenize-spans-step-512 src pos len tokens) done (vector-get step 0)] (if (= done 1) (vector-get step 2) (tokenize-spans-loop-10 src (vector-get step 1) len (vector-get step 2)))))
(defn tokenize-spans-loop-10 [src pos len tokens] (let [step (tokenize-spans-step-512 src pos len tokens) done (vector-get step 0)] (if (= done 1) (vector-get step 2) (tokenize-spans-loop-11 src (vector-get step 1) len (vector-get step 2)))))
(defn tokenize-spans-loop-11 [src pos len tokens] (let [step (tokenize-spans-step-512 src pos len tokens) done (vector-get step 0)] (if (= done 1) (vector-get step 2) (tokenize-spans-loop-12 src (vector-get step 1) len (vector-get step 2)))))
(defn tokenize-spans-loop-12 [src pos len tokens] (let [step (tokenize-spans-step-512 src pos len tokens) done (vector-get step 0)] (if (= done 1) (vector-get step 2) (tokenize-spans-loop-13 src (vector-get step 1) len (vector-get step 2)))))
(defn tokenize-spans-loop-13 [src pos len tokens] (let [step (tokenize-spans-step-512 src pos len tokens) done (vector-get step 0)] (if (= done 1) (vector-get step 2) (tokenize-spans-loop-14 src (vector-get step 1) len (vector-get step 2)))))
(defn tokenize-spans-loop-14 [src pos len tokens] (let [step (tokenize-spans-step-512 src pos len tokens) done (vector-get step 0)] (if (= done 1) (vector-get step 2) (tokenize-spans-loop-15 src (vector-get step 1) len (vector-get step 2)))))
(defn tokenize-spans-loop-15 [src pos len tokens] (let [step (tokenize-spans-step-512 src pos len tokens) done (vector-get step 0)] (if (= done 1) (vector-get step 2) (tokenize-spans-loop-16 src (vector-get step 1) len (vector-get step 2)))))
(defn tokenize-spans-loop-16 [src pos len tokens] (let [step (tokenize-spans-step-512 src pos len tokens) done (vector-get step 0)] (if (= done 1) (vector-get step 2) (tokenize-spans-loop-17 src (vector-get step 1) len (vector-get step 2)))))
(defn tokenize-spans-loop-17 [src pos len tokens] (let [step (tokenize-spans-step-512 src pos len tokens) done (vector-get step 0)] (if (= done 1) (vector-get step 2) (tokenize-spans-loop-18 src (vector-get step 1) len (vector-get step 2)))))
(defn tokenize-spans-loop-18 [src pos len tokens] (let [step (tokenize-spans-step-512 src pos len tokens) done (vector-get step 0)] (if (= done 1) (vector-get step 2) (tokenize-spans-loop-19 src (vector-get step 1) len (vector-get step 2)))))
(defn tokenize-spans-loop-19 [src pos len tokens] (let [step (tokenize-spans-step-512 src pos len tokens) done (vector-get step 0)] (if (= done 1) (vector-get step 2) (tokenize-spans-loop-20 src (vector-get step 1) len (vector-get step 2)))))
(defn tokenize-spans-loop-20 [src pos len tokens] (let [step (tokenize-spans-step-512 src pos len tokens) done (vector-get step 0)] (if (= done 1) (vector-get step 2) (tokenize-spans-loop-21 src (vector-get step 1) len (vector-get step 2)))))
(defn tokenize-spans-loop-21 [src pos len tokens] (let [step (tokenize-spans-step-512 src pos len tokens) done (vector-get step 0)] (if (= done 1) (vector-get step 2) (tokenize-spans-loop-22 src (vector-get step 1) len (vector-get step 2)))))
(defn tokenize-spans-loop-22 [src pos len tokens] (let [step (tokenize-spans-step-512 src pos len tokens) done (vector-get step 0)] (if (= done 1) (vector-get step 2) (tokenize-spans-loop-23 src (vector-get step 1) len (vector-get step 2)))))
(defn tokenize-spans-loop-23 [src pos len tokens] (let [step (tokenize-spans-step-512 src pos len tokens) done (vector-get step 0)] (if (= done 1) (vector-get step 2) (tokenize-spans-loop-24 src (vector-get step 1) len (vector-get step 2)))))
(defn tokenize-spans-loop-24 [src pos len tokens] (let [step (tokenize-spans-step-512 src pos len tokens) done (vector-get step 0)] (if (= done 1) (vector-get step 2) (tokenize-spans-loop-25 src (vector-get step 1) len (vector-get step 2)))))
(defn tokenize-spans-loop-25 [src pos len tokens] (let [step (tokenize-spans-step-512 src pos len tokens) done (vector-get step 0)] (if (= done 1) (vector-get step 2) (tokenize-spans-loop-26 src (vector-get step 1) len (vector-get step 2)))))
(defn tokenize-spans-loop-26 [src pos len tokens] (let [step (tokenize-spans-step-512 src pos len tokens) done (vector-get step 0)] (if (= done 1) (vector-get step 2) (tokenize-spans-loop-27 src (vector-get step 1) len (vector-get step 2)))))
(defn tokenize-spans-loop-27 [src pos len tokens] (let [step (tokenize-spans-step-512 src pos len tokens) done (vector-get step 0)] (if (= done 1) (vector-get step 2) (tokenize-spans-loop-28 src (vector-get step 1) len (vector-get step 2)))))
(defn tokenize-spans-loop-28 [src pos len tokens] (let [step (tokenize-spans-step-512 src pos len tokens) done (vector-get step 0)] (if (= done 1) (vector-get step 2) (tokenize-spans-loop-29 src (vector-get step 1) len (vector-get step 2)))))
(defn tokenize-spans-loop-29 [src pos len tokens] (let [step (tokenize-spans-step-512 src pos len tokens) done (vector-get step 0)] (if (= done 1) (vector-get step 2) (tokenize-spans-loop-30 src (vector-get step 1) len (vector-get step 2)))))
(defn tokenize-spans-loop-30 [src pos len tokens] (let [step (tokenize-spans-step-512 src pos len tokens) done (vector-get step 0)] (if (= done 1) (vector-get step 2) (tokenize-spans-loop-31 src (vector-get step 1) len (vector-get step 2)))))
(defn tokenize-spans-loop-31 [src pos len tokens] (let [step (tokenize-spans-step-512 src pos len tokens) done (vector-get step 0)] (if (= done 1) (vector-get step 2) (tokenize-spans-loop-32 src (vector-get step 1) len (vector-get step 2)))))
(defn tokenize-spans-loop-32 [src pos len tokens] (let [step (tokenize-spans-step-512 src pos len tokens) done (vector-get step 0)] (if (= done 1) (vector-get step 2) (tokenize-spans-loop-33 src (vector-get step 1) len (vector-get step 2)))))
(defn tokenize-spans-loop-33 [src pos len tokens] (let [step (tokenize-spans-step-512 src pos len tokens) done (vector-get step 0)] (if (= done 1) (vector-get step 2) (tokenize-spans-loop-34 src (vector-get step 1) len (vector-get step 2)))))
(defn tokenize-spans-loop-34 [src pos len tokens] (let [step (tokenize-spans-step-512 src pos len tokens) done (vector-get step 0)] (if (= done 1) (vector-get step 2) (tokenize-spans-loop-35 src (vector-get step 1) len (vector-get step 2)))))
(defn tokenize-spans-loop-35 [src pos len tokens] (let [step (tokenize-spans-step-512 src pos len tokens) done (vector-get step 0)] (if (= done 1) (vector-get step 2) (tokenize-spans-loop-36 src (vector-get step 1) len (vector-get step 2)))))
(defn tokenize-spans-loop-36 [src pos len tokens] (let [step (tokenize-spans-step-512 src pos len tokens) done (vector-get step 0)] (if (= done 1) (vector-get step 2) (tokenize-spans-loop-37 src (vector-get step 1) len (vector-get step 2)))))
(defn tokenize-spans-loop-37 [src pos len tokens] (let [step (tokenize-spans-step-512 src pos len tokens) done (vector-get step 0)] (if (= done 1) (vector-get step 2) (tokenize-spans-loop-38 src (vector-get step 1) len (vector-get step 2)))))
(defn tokenize-spans-loop-38 [src pos len tokens] (let [step (tokenize-spans-step-512 src pos len tokens) done (vector-get step 0)] (if (= done 1) (vector-get step 2) (tokenize-spans-loop-39 src (vector-get step 1) len (vector-get step 2)))))
(defn tokenize-spans-loop-39 [src pos len tokens] (let [step (tokenize-spans-step-512 src pos len tokens) done (vector-get step 0)] (if (= done 1) (vector-get step 2) (tokenize-spans-loop-40 src (vector-get step 1) len (vector-get step 2)))))
(defn tokenize-spans-loop-40 [src pos len tokens] (let [step (tokenize-spans-step-512 src pos len tokens) done (vector-get step 0)] (if (= done 1) (vector-get step 2) (tokenize-spans-loop-41 src (vector-get step 1) len (vector-get step 2)))))
(defn tokenize-spans-loop-41 [src pos len tokens] (let [step (tokenize-spans-step-512 src pos len tokens) done (vector-get step 0)] (if (= done 1) (vector-get step 2) (tokenize-spans-loop-42 src (vector-get step 1) len (vector-get step 2)))))
(defn tokenize-spans-loop-42 [src pos len tokens] (let [step (tokenize-spans-step-512 src pos len tokens) done (vector-get step 0)] (if (= done 1) (vector-get step 2) (tokenize-spans-loop-43 src (vector-get step 1) len (vector-get step 2)))))
(defn tokenize-spans-loop-43 [src pos len tokens] (let [step (tokenize-spans-step-512 src pos len tokens) done (vector-get step 0)] (if (= done 1) (vector-get step 2) (tokenize-spans-loop-44 src (vector-get step 1) len (vector-get step 2)))))
(defn tokenize-spans-loop-44 [src pos len tokens] (let [step (tokenize-spans-step-512 src pos len tokens) done (vector-get step 0)] (if (= done 1) (vector-get step 2) (tokenize-spans-loop-45 src (vector-get step 1) len (vector-get step 2)))))
(defn tokenize-spans-loop-45 [src pos len tokens] (let [step (tokenize-spans-step-512 src pos len tokens) done (vector-get step 0)] (if (= done 1) (vector-get step 2) (tokenize-spans-loop-46 src (vector-get step 1) len (vector-get step 2)))))
(defn tokenize-spans-loop-46 [src pos len tokens] (let [step (tokenize-spans-step-512 src pos len tokens) done (vector-get step 0)] (if (= done 1) (vector-get step 2) (tokenize-spans-loop-47 src (vector-get step 1) len (vector-get step 2)))))
(defn tokenize-spans-loop-47 [src pos len tokens] (let [step (tokenize-spans-step-512 src pos len tokens) done (vector-get step 0)] (if (= done 1) (vector-get step 2) (tokenize-spans-loop-48 src (vector-get step 1) len (vector-get step 2)))))
(defn tokenize-spans-loop-48 [src pos len tokens] (let [step (tokenize-spans-step-512 src pos len tokens) done (vector-get step 0)] (if (= done 1) (vector-get step 2) (tokenize-spans-loop-49 src (vector-get step 1) len (vector-get step 2)))))
(defn tokenize-spans-loop-49 [src pos len tokens] (let [step (tokenize-spans-step-512 src pos len tokens) done (vector-get step 0)] (if (= done 1) (vector-get step 2) (tokenize-spans-loop-50 src (vector-get step 1) len (vector-get step 2)))))
(defn tokenize-spans-loop-50 [src pos len tokens] (let [step (tokenize-spans-step-512 src pos len tokens) done (vector-get step 0)] (if (= done 1) (vector-get step 2) (tokenize-spans-loop-51 src (vector-get step 1) len (vector-get step 2)))))
(defn tokenize-spans-loop-51 [src pos len tokens] (let [step (tokenize-spans-step-512 src pos len tokens) done (vector-get step 0)] (if (= done 1) (vector-get step 2) (tokenize-spans-loop-52 src (vector-get step 1) len (vector-get step 2)))))
(defn tokenize-spans-loop-52 [src pos len tokens] (let [step (tokenize-spans-step-512 src pos len tokens) done (vector-get step 0)] (if (= done 1) (vector-get step 2) (tokenize-spans-loop-53 src (vector-get step 1) len (vector-get step 2)))))
(defn tokenize-spans-loop-53 [src pos len tokens] (let [step (tokenize-spans-step-512 src pos len tokens) done (vector-get step 0)] (if (= done 1) (vector-get step 2) (tokenize-spans-loop-54 src (vector-get step 1) len (vector-get step 2)))))
(defn tokenize-spans-loop-54 [src pos len tokens] (let [step (tokenize-spans-step-512 src pos len tokens) done (vector-get step 0)] (if (= done 1) (vector-get step 2) (tokenize-spans-loop-55 src (vector-get step 1) len (vector-get step 2)))))
(defn tokenize-spans-loop-55 [src pos len tokens] (let [step (tokenize-spans-step-512 src pos len tokens) done (vector-get step 0)] (if (= done 1) (vector-get step 2) (tokenize-spans-loop-56 src (vector-get step 1) len (vector-get step 2)))))
(defn tokenize-spans-loop-56 [src pos len tokens] (let [step (tokenize-spans-step-512 src pos len tokens) done (vector-get step 0)] (if (= done 1) (vector-get step 2) (tokenize-spans-loop-57 src (vector-get step 1) len (vector-get step 2)))))
(defn tokenize-spans-loop-57 [src pos len tokens] (let [step (tokenize-spans-step-512 src pos len tokens) done (vector-get step 0)] (if (= done 1) (vector-get step 2) (tokenize-spans-loop-58 src (vector-get step 1) len (vector-get step 2)))))
(defn tokenize-spans-loop-58 [src pos len tokens] (let [step (tokenize-spans-step-512 src pos len tokens) done (vector-get step 0)] (if (= done 1) (vector-get step 2) (tokenize-spans-loop-59 src (vector-get step 1) len (vector-get step 2)))))
(defn tokenize-spans-loop-59 [src pos len tokens] (let [step (tokenize-spans-step-512 src pos len tokens) done (vector-get step 0)] (if (= done 1) (vector-get step 2) (tokenize-spans-loop-60 src (vector-get step 1) len (vector-get step 2)))))
(defn tokenize-spans-loop-60 [src pos len tokens] (let [step (tokenize-spans-step-512 src pos len tokens) done (vector-get step 0)] (if (= done 1) (vector-get step 2) (tokenize-spans-loop-61 src (vector-get step 1) len (vector-get step 2)))))
(defn tokenize-spans-loop-61 [src pos len tokens] (let [step (tokenize-spans-step-512 src pos len tokens) done (vector-get step 0)] (if (= done 1) (vector-get step 2) (tokenize-spans-loop-62 src (vector-get step 1) len (vector-get step 2)))))
(defn tokenize-spans-loop-62 [src pos len tokens] (let [step (tokenize-spans-step-512 src pos len tokens) done (vector-get step 0)] (if (= done 1) (vector-get step 2) (tokenize-spans-loop-63 src (vector-get step 1) len (vector-get step 2)))))
(defn tokenize-spans-loop-63 [src pos len tokens] (let [step (tokenize-spans-step-512 src pos len tokens) done (vector-get step 0)] (if (= done 1) (vector-get step 2) (tokenize-spans-loop-64 src (vector-get step 1) len (vector-get step 2)))))
(defn tokenize-spans-loop-64 [src pos len tokens] (let [step (tokenize-spans-step-512 src pos len tokens) done (vector-get step 0)] (if (= done 1) (vector-get step 2) (tokenize-spans-loop src (vector-get step 1) len (vector-get step 2)))))

(defn make-append-state [done next-idx next-dst]
  (vector-push
    (vector-push
      (vector-push (vector-new 4) done)
      next-idx)
    next-dst))

(defn append-span-tokens-step [dst chunk-tokens idx emit-count offset]
  (if (>= idx emit-count)
    (make-append-state 1 idx dst)
    (make-append-state
      0
      (+ idx 1)
      (append-span-token
        dst
        (vector-get chunk-tokens (* idx 3))
        (+ offset (vector-get chunk-tokens (+ (* idx 3) 1)))
        (+ offset (vector-get chunk-tokens (+ (* idx 3) 2)))))))

(defn append-span-tokens-step-8 [dst chunk-tokens idx emit-count offset]
  (let [step1 (append-span-tokens-step dst chunk-tokens idx emit-count offset)
    done1 (vector-get step1 0)]
    (if (= done1 1)
      step1
      (let [step2 (append-span-tokens-step (vector-get step1 2) chunk-tokens (vector-get step1 1) emit-count offset)
        done2 (vector-get step2 0)]
        (if (= done2 1)
          step2
          (let [step3 (append-span-tokens-step (vector-get step2 2) chunk-tokens (vector-get step2 1) emit-count offset)
            done3 (vector-get step3 0)]
            (if (= done3 1)
              step3
              (let [step4 (append-span-tokens-step (vector-get step3 2) chunk-tokens (vector-get step3 1) emit-count offset)
                done4 (vector-get step4 0)]
                (if (= done4 1)
                  step4
                  (let [step5 (append-span-tokens-step (vector-get step4 2) chunk-tokens (vector-get step4 1) emit-count offset)
                    done5 (vector-get step5 0)]
                    (if (= done5 1)
                      step5
                      (let [step6 (append-span-tokens-step (vector-get step5 2) chunk-tokens (vector-get step5 1) emit-count offset)
                        done6 (vector-get step6 0)]
                        (if (= done6 1)
                          step6
                          (let [step7 (append-span-tokens-step (vector-get step6 2) chunk-tokens (vector-get step6 1) emit-count offset)
                            done7 (vector-get step7 0)]
                            (if (= done7 1)
                              step7
                              (append-span-tokens-step (vector-get step7 2) chunk-tokens (vector-get step7 1) emit-count offset))))))))))))))))

(defn append-span-tokens-loop-1 [dst chunk-tokens idx emit-count offset] (let [step (append-span-tokens-step-8 dst chunk-tokens idx emit-count offset) done (vector-get step 0)] (if (= done 1) (vector-get step 2) (append-span-tokens-loop-2 (vector-get step 2) chunk-tokens (vector-get step 1) emit-count offset))))
(defn append-span-tokens-loop-2 [dst chunk-tokens idx emit-count offset] (let [step (append-span-tokens-step-8 dst chunk-tokens idx emit-count offset) done (vector-get step 0)] (if (= done 1) (vector-get step 2) (append-span-tokens-loop-3 (vector-get step 2) chunk-tokens (vector-get step 1) emit-count offset))))
(defn append-span-tokens-loop-3 [dst chunk-tokens idx emit-count offset] (let [step (append-span-tokens-step-8 dst chunk-tokens idx emit-count offset) done (vector-get step 0)] (if (= done 1) (vector-get step 2) (append-span-tokens-loop-4 (vector-get step 2) chunk-tokens (vector-get step 1) emit-count offset))))
(defn append-span-tokens-loop-4 [dst chunk-tokens idx emit-count offset] (let [step (append-span-tokens-step-8 dst chunk-tokens idx emit-count offset) done (vector-get step 0)] (if (= done 1) (vector-get step 2) (append-span-tokens-loop-5 (vector-get step 2) chunk-tokens (vector-get step 1) emit-count offset))))
(defn append-span-tokens-loop-5 [dst chunk-tokens idx emit-count offset] (let [step (append-span-tokens-step-8 dst chunk-tokens idx emit-count offset) done (vector-get step 0)] (if (= done 1) (vector-get step 2) (append-span-tokens-loop-6 (vector-get step 2) chunk-tokens (vector-get step 1) emit-count offset))))
(defn append-span-tokens-loop-6 [dst chunk-tokens idx emit-count offset] (let [step (append-span-tokens-step-8 dst chunk-tokens idx emit-count offset) done (vector-get step 0)] (if (= done 1) (vector-get step 2) (append-span-tokens-loop-7 (vector-get step 2) chunk-tokens (vector-get step 1) emit-count offset))))
(defn append-span-tokens-loop-7 [dst chunk-tokens idx emit-count offset] (let [step (append-span-tokens-step-8 dst chunk-tokens idx emit-count offset) done (vector-get step 0)] (if (= done 1) (vector-get step 2) (append-span-tokens-loop-8 (vector-get step 2) chunk-tokens (vector-get step 1) emit-count offset))))
(defn append-span-tokens-loop-8 [dst chunk-tokens idx emit-count offset] (let [step (append-span-tokens-step-8 dst chunk-tokens idx emit-count offset) done (vector-get step 0)] (if (= done 1) (vector-get step 2) (append-span-tokens-loop-9 (vector-get step 2) chunk-tokens (vector-get step 1) emit-count offset))))
(defn append-span-tokens-loop-9 [dst chunk-tokens idx emit-count offset] (let [step (append-span-tokens-step-8 dst chunk-tokens idx emit-count offset) done (vector-get step 0)] (if (= done 1) (vector-get step 2) (append-span-tokens-loop-10 (vector-get step 2) chunk-tokens (vector-get step 1) emit-count offset))))
(defn append-span-tokens-loop-10 [dst chunk-tokens idx emit-count offset] (let [step (append-span-tokens-step-8 dst chunk-tokens idx emit-count offset) done (vector-get step 0)] (if (= done 1) (vector-get step 2) (append-span-tokens-loop-11 (vector-get step 2) chunk-tokens (vector-get step 1) emit-count offset))))
(defn append-span-tokens-loop-11 [dst chunk-tokens idx emit-count offset] (let [step (append-span-tokens-step-8 dst chunk-tokens idx emit-count offset) done (vector-get step 0)] (if (= done 1) (vector-get step 2) (append-span-tokens-loop-12 (vector-get step 2) chunk-tokens (vector-get step 1) emit-count offset))))
(defn append-span-tokens-loop-12 [dst chunk-tokens idx emit-count offset] (let [step (append-span-tokens-step-8 dst chunk-tokens idx emit-count offset) done (vector-get step 0)] (if (= done 1) (vector-get step 2) (append-span-tokens-loop-13 (vector-get step 2) chunk-tokens (vector-get step 1) emit-count offset))))
(defn append-span-tokens-loop-13 [dst chunk-tokens idx emit-count offset] (let [step (append-span-tokens-step-8 dst chunk-tokens idx emit-count offset) done (vector-get step 0)] (if (= done 1) (vector-get step 2) (append-span-tokens-loop-14 (vector-get step 2) chunk-tokens (vector-get step 1) emit-count offset))))
(defn append-span-tokens-loop-14 [dst chunk-tokens idx emit-count offset] (let [step (append-span-tokens-step-8 dst chunk-tokens idx emit-count offset) done (vector-get step 0)] (if (= done 1) (vector-get step 2) (append-span-tokens-loop-15 (vector-get step 2) chunk-tokens (vector-get step 1) emit-count offset))))
(defn append-span-tokens-loop-15 [dst chunk-tokens idx emit-count offset] (let [step (append-span-tokens-step-8 dst chunk-tokens idx emit-count offset) done (vector-get step 0)] (if (= done 1) (vector-get step 2) (append-span-tokens-loop-16 (vector-get step 2) chunk-tokens (vector-get step 1) emit-count offset))))
(defn append-span-tokens-loop-16 [dst chunk-tokens idx emit-count offset] (let [step (append-span-tokens-step-8 dst chunk-tokens idx emit-count offset) done (vector-get step 0)] (if (= done 1) (vector-get step 2) (append-span-tokens-loop-17 (vector-get step 2) chunk-tokens (vector-get step 1) emit-count offset))))
(defn append-span-tokens-loop-17 [dst chunk-tokens idx emit-count offset] (let [step (append-span-tokens-step-8 dst chunk-tokens idx emit-count offset) done (vector-get step 0)] (if (= done 1) (vector-get step 2) (append-span-tokens-loop-18 (vector-get step 2) chunk-tokens (vector-get step 1) emit-count offset))))
(defn append-span-tokens-loop-18 [dst chunk-tokens idx emit-count offset] (let [step (append-span-tokens-step-8 dst chunk-tokens idx emit-count offset) done (vector-get step 0)] (if (= done 1) (vector-get step 2) (append-span-tokens-loop-19 (vector-get step 2) chunk-tokens (vector-get step 1) emit-count offset))))
(defn append-span-tokens-loop-19 [dst chunk-tokens idx emit-count offset] (let [step (append-span-tokens-step-8 dst chunk-tokens idx emit-count offset) done (vector-get step 0)] (if (= done 1) (vector-get step 2) (append-span-tokens-loop-20 (vector-get step 2) chunk-tokens (vector-get step 1) emit-count offset))))
(defn append-span-tokens-loop-20 [dst chunk-tokens idx emit-count offset] (let [step (append-span-tokens-step-8 dst chunk-tokens idx emit-count offset) done (vector-get step 0)] (if (= done 1) (vector-get step 2) (append-span-tokens-loop-21 (vector-get step 2) chunk-tokens (vector-get step 1) emit-count offset))))
(defn append-span-tokens-loop-21 [dst chunk-tokens idx emit-count offset] (let [step (append-span-tokens-step-8 dst chunk-tokens idx emit-count offset) done (vector-get step 0)] (if (= done 1) (vector-get step 2) (append-span-tokens-loop-22 (vector-get step 2) chunk-tokens (vector-get step 1) emit-count offset))))
(defn append-span-tokens-loop-22 [dst chunk-tokens idx emit-count offset] (let [step (append-span-tokens-step-8 dst chunk-tokens idx emit-count offset) done (vector-get step 0)] (if (= done 1) (vector-get step 2) (append-span-tokens-loop-23 (vector-get step 2) chunk-tokens (vector-get step 1) emit-count offset))))
(defn append-span-tokens-loop-23 [dst chunk-tokens idx emit-count offset] (let [step (append-span-tokens-step-8 dst chunk-tokens idx emit-count offset) done (vector-get step 0)] (if (= done 1) (vector-get step 2) (append-span-tokens-loop-24 (vector-get step 2) chunk-tokens (vector-get step 1) emit-count offset))))
(defn append-span-tokens-loop-24 [dst chunk-tokens idx emit-count offset] (let [step (append-span-tokens-step-8 dst chunk-tokens idx emit-count offset) done (vector-get step 0)] (if (= done 1) (vector-get step 2) (append-span-tokens-loop-25 (vector-get step 2) chunk-tokens (vector-get step 1) emit-count offset))))
(defn append-span-tokens-loop-25 [dst chunk-tokens idx emit-count offset] (let [step (append-span-tokens-step-8 dst chunk-tokens idx emit-count offset) done (vector-get step 0)] (if (= done 1) (vector-get step 2) (append-span-tokens-loop-26 (vector-get step 2) chunk-tokens (vector-get step 1) emit-count offset))))
(defn append-span-tokens-loop-26 [dst chunk-tokens idx emit-count offset] (let [step (append-span-tokens-step-8 dst chunk-tokens idx emit-count offset) done (vector-get step 0)] (if (= done 1) (vector-get step 2) (append-span-tokens-loop-27 (vector-get step 2) chunk-tokens (vector-get step 1) emit-count offset))))
(defn append-span-tokens-loop-27 [dst chunk-tokens idx emit-count offset] (let [step (append-span-tokens-step-8 dst chunk-tokens idx emit-count offset) done (vector-get step 0)] (if (= done 1) (vector-get step 2) (append-span-tokens-loop-28 (vector-get step 2) chunk-tokens (vector-get step 1) emit-count offset))))
(defn append-span-tokens-loop-28 [dst chunk-tokens idx emit-count offset] (let [step (append-span-tokens-step-8 dst chunk-tokens idx emit-count offset) done (vector-get step 0)] (if (= done 1) (vector-get step 2) (append-span-tokens-loop-29 (vector-get step 2) chunk-tokens (vector-get step 1) emit-count offset))))
(defn append-span-tokens-loop-29 [dst chunk-tokens idx emit-count offset] (let [step (append-span-tokens-step-8 dst chunk-tokens idx emit-count offset) done (vector-get step 0)] (if (= done 1) (vector-get step 2) (append-span-tokens-loop-30 (vector-get step 2) chunk-tokens (vector-get step 1) emit-count offset))))
(defn append-span-tokens-loop-30 [dst chunk-tokens idx emit-count offset] (let [step (append-span-tokens-step-8 dst chunk-tokens idx emit-count offset) done (vector-get step 0)] (if (= done 1) (vector-get step 2) (append-span-tokens-loop-31 (vector-get step 2) chunk-tokens (vector-get step 1) emit-count offset))))
(defn append-span-tokens-loop-31 [dst chunk-tokens idx emit-count offset] (let [step (append-span-tokens-step-8 dst chunk-tokens idx emit-count offset) done (vector-get step 0)] (if (= done 1) (vector-get step 2) (append-span-tokens-loop-32 (vector-get step 2) chunk-tokens (vector-get step 1) emit-count offset))))
(defn append-span-tokens-loop-32 [dst chunk-tokens idx emit-count offset] (let [step (append-span-tokens-step-8 dst chunk-tokens idx emit-count offset) done (vector-get step 0)] (if (= done 1) (vector-get step 2) (append-span-tokens-with-offset (vector-get step 2) chunk-tokens (vector-get step 1) emit-count offset))))

(defn append-span-tokens-with-offset [dst chunk-tokens idx emit-count offset]
  (append-span-tokens-loop-1 dst chunk-tokens idx emit-count offset))

(defn tokenize-spans-local-loop-1 [src pos len tokens] (let [step (tokenize-spans-step-8 src pos len tokens) done (vector-get step 0)] (if (= done 1) (vector-get step 2) (tokenize-spans-local-loop-2 src (vector-get step 1) len (vector-get step 2)))))
(defn tokenize-spans-local-loop-2 [src pos len tokens] (let [step (tokenize-spans-step-8 src pos len tokens) done (vector-get step 0)] (if (= done 1) (vector-get step 2) (tokenize-spans-local-loop-3 src (vector-get step 1) len (vector-get step 2)))))
(defn tokenize-spans-local-loop-3 [src pos len tokens] (let [step (tokenize-spans-step-8 src pos len tokens) done (vector-get step 0)] (if (= done 1) (vector-get step 2) (tokenize-spans-local-loop-4 src (vector-get step 1) len (vector-get step 2)))))
(defn tokenize-spans-local-loop-4 [src pos len tokens] (let [step (tokenize-spans-step-8 src pos len tokens) done (vector-get step 0)] (if (= done 1) (vector-get step 2) (tokenize-spans-local-loop-5 src (vector-get step 1) len (vector-get step 2)))))
(defn tokenize-spans-local-loop-5 [src pos len tokens] (let [step (tokenize-spans-step-8 src pos len tokens) done (vector-get step 0)] (if (= done 1) (vector-get step 2) (tokenize-spans-local-loop-6 src (vector-get step 1) len (vector-get step 2)))))
(defn tokenize-spans-local-loop-6 [src pos len tokens] (let [step (tokenize-spans-step-8 src pos len tokens) done (vector-get step 0)] (if (= done 1) (vector-get step 2) (tokenize-spans-local-loop-7 src (vector-get step 1) len (vector-get step 2)))))
(defn tokenize-spans-local-loop-7 [src pos len tokens] (let [step (tokenize-spans-step-8 src pos len tokens) done (vector-get step 0)] (if (= done 1) (vector-get step 2) (tokenize-spans-local-loop-8 src (vector-get step 1) len (vector-get step 2)))))
(defn tokenize-spans-local-loop-8 [src pos len tokens] (let [step (tokenize-spans-step-8 src pos len tokens) done (vector-get step 0)] (if (= done 1) (vector-get step 2) (tokenize-spans-local-loop-9 src (vector-get step 1) len (vector-get step 2)))))
(defn tokenize-spans-local-loop-9 [src pos len tokens] (let [step (tokenize-spans-step-8 src pos len tokens) done (vector-get step 0)] (if (= done 1) (vector-get step 2) (tokenize-spans-local-loop-10 src (vector-get step 1) len (vector-get step 2)))))
(defn tokenize-spans-local-loop-10 [src pos len tokens] (let [step (tokenize-spans-step-8 src pos len tokens) done (vector-get step 0)] (if (= done 1) (vector-get step 2) (tokenize-spans-local-loop-11 src (vector-get step 1) len (vector-get step 2)))))
(defn tokenize-spans-local-loop-11 [src pos len tokens] (let [step (tokenize-spans-step-8 src pos len tokens) done (vector-get step 0)] (if (= done 1) (vector-get step 2) (tokenize-spans-local-loop-12 src (vector-get step 1) len (vector-get step 2)))))
(defn tokenize-spans-local-loop-12 [src pos len tokens] (let [step (tokenize-spans-step-8 src pos len tokens) done (vector-get step 0)] (if (= done 1) (vector-get step 2) (tokenize-spans-local-loop-13 src (vector-get step 1) len (vector-get step 2)))))
(defn tokenize-spans-local-loop-13 [src pos len tokens] (let [step (tokenize-spans-step-8 src pos len tokens) done (vector-get step 0)] (if (= done 1) (vector-get step 2) (tokenize-spans-local-loop-14 src (vector-get step 1) len (vector-get step 2)))))
(defn tokenize-spans-local-loop-14 [src pos len tokens] (let [step (tokenize-spans-step-8 src pos len tokens) done (vector-get step 0)] (if (= done 1) (vector-get step 2) (tokenize-spans-local-loop-15 src (vector-get step 1) len (vector-get step 2)))))
(defn tokenize-spans-local-loop-15 [src pos len tokens] (let [step (tokenize-spans-step-8 src pos len tokens) done (vector-get step 0)] (if (= done 1) (vector-get step 2) (tokenize-spans-local-loop-16 src (vector-get step 1) len (vector-get step 2)))))
(defn tokenize-spans-local-loop-16 [src pos len tokens] (let [step (tokenize-spans-step-8 src pos len tokens) done (vector-get step 0)] (if (= done 1) (vector-get step 2) (tokenize-spans-local-loop-17 src (vector-get step 1) len (vector-get step 2)))))
(defn tokenize-spans-local-loop-17 [src pos len tokens] (let [step (tokenize-spans-step-8 src pos len tokens) done (vector-get step 0)] (if (= done 1) (vector-get step 2) (tokenize-spans-local-loop-18 src (vector-get step 1) len (vector-get step 2)))))
(defn tokenize-spans-local-loop-18 [src pos len tokens] (let [step (tokenize-spans-step-8 src pos len tokens) done (vector-get step 0)] (if (= done 1) (vector-get step 2) (tokenize-spans-local-loop-19 src (vector-get step 1) len (vector-get step 2)))))
(defn tokenize-spans-local-loop-19 [src pos len tokens] (let [step (tokenize-spans-step-8 src pos len tokens) done (vector-get step 0)] (if (= done 1) (vector-get step 2) (tokenize-spans-local-loop-20 src (vector-get step 1) len (vector-get step 2)))))
(defn tokenize-spans-local-loop-20 [src pos len tokens] (let [step (tokenize-spans-step-8 src pos len tokens) done (vector-get step 0)] (if (= done 1) (vector-get step 2) (tokenize-spans-local-loop-21 src (vector-get step 1) len (vector-get step 2)))))
(defn tokenize-spans-local-loop-21 [src pos len tokens] (let [step (tokenize-spans-step-8 src pos len tokens) done (vector-get step 0)] (if (= done 1) (vector-get step 2) (tokenize-spans-local-loop-22 src (vector-get step 1) len (vector-get step 2)))))
(defn tokenize-spans-local-loop-22 [src pos len tokens] (let [step (tokenize-spans-step-8 src pos len tokens) done (vector-get step 0)] (if (= done 1) (vector-get step 2) (tokenize-spans-local-loop-23 src (vector-get step 1) len (vector-get step 2)))))
(defn tokenize-spans-local-loop-23 [src pos len tokens] (let [step (tokenize-spans-step-8 src pos len tokens) done (vector-get step 0)] (if (= done 1) (vector-get step 2) (tokenize-spans-local-loop-24 src (vector-get step 1) len (vector-get step 2)))))
(defn tokenize-spans-local-loop-24 [src pos len tokens] (let [step (tokenize-spans-step-8 src pos len tokens) done (vector-get step 0)] (if (= done 1) (vector-get step 2) (tokenize-spans-local-loop-25 src (vector-get step 1) len (vector-get step 2)))))
(defn tokenize-spans-local-loop-25 [src pos len tokens] (let [step (tokenize-spans-step-8 src pos len tokens) done (vector-get step 0)] (if (= done 1) (vector-get step 2) (tokenize-spans-local-loop-26 src (vector-get step 1) len (vector-get step 2)))))
(defn tokenize-spans-local-loop-26 [src pos len tokens] (let [step (tokenize-spans-step-8 src pos len tokens) done (vector-get step 0)] (if (= done 1) (vector-get step 2) (tokenize-spans-local-loop-27 src (vector-get step 1) len (vector-get step 2)))))
(defn tokenize-spans-local-loop-27 [src pos len tokens] (let [step (tokenize-spans-step-8 src pos len tokens) done (vector-get step 0)] (if (= done 1) (vector-get step 2) (tokenize-spans-local-loop-28 src (vector-get step 1) len (vector-get step 2)))))
(defn tokenize-spans-local-loop-28 [src pos len tokens] (let [step (tokenize-spans-step-8 src pos len tokens) done (vector-get step 0)] (if (= done 1) (vector-get step 2) (tokenize-spans-local-loop-29 src (vector-get step 1) len (vector-get step 2)))))
(defn tokenize-spans-local-loop-29 [src pos len tokens] (let [step (tokenize-spans-step-8 src pos len tokens) done (vector-get step 0)] (if (= done 1) (vector-get step 2) (tokenize-spans-local-loop-30 src (vector-get step 1) len (vector-get step 2)))))
(defn tokenize-spans-local-loop-30 [src pos len tokens] (let [step (tokenize-spans-step-8 src pos len tokens) done (vector-get step 0)] (if (= done 1) (vector-get step 2) (tokenize-spans-local-loop-31 src (vector-get step 1) len (vector-get step 2)))))
(defn tokenize-spans-local-loop-31 [src pos len tokens] (let [step (tokenize-spans-step-8 src pos len tokens) done (vector-get step 0)] (if (= done 1) (vector-get step 2) (tokenize-spans-local-loop-32 src (vector-get step 1) len (vector-get step 2)))))
(defn tokenize-spans-local-loop-32 [src pos len tokens] (let [step (tokenize-spans-step-8 src pos len tokens) done (vector-get step 0)] (if (= done 1) (vector-get step 2) (tokenize-spans-local-loop src (vector-get step 1) len (vector-get step 2)))))
(defn tokenize-spans-local-loop [src pos len tokens]
  (tokenize-spans-local-loop-1 src pos len tokens))

(defn tokenize-spans-chunks-step [src chunk-start len tokens]
  (let [boundary (advance-spans-step-256 src chunk-start len)
    boundary-done (vector-get boundary 0)
    boundary-end (if (= boundary-done 1) len (vector-get boundary 1))
    fallback8 (advance-spans-step-8 src chunk-start len)
    fallback8-done (vector-get fallback8 0)
    fallback8-end (if (= fallback8-done 1) len (vector-get fallback8 1))
    fallback1 (advance-spans-step src chunk-start len)
    fallback1-done (vector-get fallback1 0)
    fallback1-end (if (= fallback1-done 1) len (vector-get fallback1 1))
    chunk-end
      (if (> boundary-end chunk-start)
        boundary-end
        (if (> fallback8-end chunk-start)
          fallback8-end
          fallback1-end))
    done (if (>= chunk-end len) 1 0)
    chunk-src (substring src chunk-start chunk-end)
    chunk-tokens (tokenize-spans-local-loop chunk-src 0 (string-length chunk-src) (vector-new 32))
    chunk-count (/ (vector-length chunk-tokens) 3)
    emit-count (if (= done 1) chunk-count (- chunk-count 1))]
    (if (>= chunk-start chunk-end)
      (tokenize-spans-step src chunk-start len tokens)
      (if (and (= done 0) (<= emit-count 0))
        (let [remainder-src (substring src chunk-start len)
          remainder-tokens (tokenize-spans-local-loop remainder-src 0 (string-length remainder-src) (vector-new 32))
          remainder-count (/ (vector-length remainder-tokens) 3)
          merged (append-span-tokens-with-offset tokens remainder-tokens 0 remainder-count chunk-start)]
          (make-tokenize-state 1 len merged))
        (let [merged (append-span-tokens-with-offset tokens chunk-tokens 0 emit-count chunk-start)]
          (make-tokenize-state done chunk-end merged))))))

(defn tokenize-spans-chunks-loop-1 [src chunk-start len tokens] (let [step (tokenize-spans-chunks-step src chunk-start len tokens) done (vector-get step 0)] (if (= done 1) (vector-get step 2) (tokenize-spans-chunks-loop-2 src (vector-get step 1) len (vector-get step 2)))))
(defn tokenize-spans-chunks-loop-2 [src chunk-start len tokens] (let [step (tokenize-spans-chunks-step src chunk-start len tokens) done (vector-get step 0)] (if (= done 1) (vector-get step 2) (tokenize-spans-chunks-loop-3 src (vector-get step 1) len (vector-get step 2)))))
(defn tokenize-spans-chunks-loop-3 [src chunk-start len tokens] (let [step (tokenize-spans-chunks-step src chunk-start len tokens) done (vector-get step 0)] (if (= done 1) (vector-get step 2) (tokenize-spans-chunks-loop-4 src (vector-get step 1) len (vector-get step 2)))))
(defn tokenize-spans-chunks-loop-4 [src chunk-start len tokens] (let [step (tokenize-spans-chunks-step src chunk-start len tokens) done (vector-get step 0)] (if (= done 1) (vector-get step 2) (tokenize-spans-chunks-loop-5 src (vector-get step 1) len (vector-get step 2)))))
(defn tokenize-spans-chunks-loop-5 [src chunk-start len tokens] (let [step (tokenize-spans-chunks-step src chunk-start len tokens) done (vector-get step 0)] (if (= done 1) (vector-get step 2) (tokenize-spans-chunks-loop-6 src (vector-get step 1) len (vector-get step 2)))))
(defn tokenize-spans-chunks-loop-6 [src chunk-start len tokens] (let [step (tokenize-spans-chunks-step src chunk-start len tokens) done (vector-get step 0)] (if (= done 1) (vector-get step 2) (tokenize-spans-chunks-loop-7 src (vector-get step 1) len (vector-get step 2)))))
(defn tokenize-spans-chunks-loop-7 [src chunk-start len tokens] (let [step (tokenize-spans-chunks-step src chunk-start len tokens) done (vector-get step 0)] (if (= done 1) (vector-get step 2) (tokenize-spans-chunks-loop-8 src (vector-get step 1) len (vector-get step 2)))))
(defn tokenize-spans-chunks-loop-8 [src chunk-start len tokens] (let [step (tokenize-spans-chunks-step src chunk-start len tokens) done (vector-get step 0)] (if (= done 1) (vector-get step 2) (tokenize-spans-chunks-loop-9 src (vector-get step 1) len (vector-get step 2)))))
(defn tokenize-spans-chunks-loop-9 [src chunk-start len tokens] (let [step (tokenize-spans-chunks-step src chunk-start len tokens) done (vector-get step 0)] (if (= done 1) (vector-get step 2) (tokenize-spans-chunks-loop-10 src (vector-get step 1) len (vector-get step 2)))))
(defn tokenize-spans-chunks-loop-10 [src chunk-start len tokens] (let [step (tokenize-spans-chunks-step src chunk-start len tokens) done (vector-get step 0)] (if (= done 1) (vector-get step 2) (tokenize-spans-chunks-loop-11 src (vector-get step 1) len (vector-get step 2)))))
(defn tokenize-spans-chunks-loop-11 [src chunk-start len tokens] (let [step (tokenize-spans-chunks-step src chunk-start len tokens) done (vector-get step 0)] (if (= done 1) (vector-get step 2) (tokenize-spans-chunks-loop-12 src (vector-get step 1) len (vector-get step 2)))))
(defn tokenize-spans-chunks-loop-12 [src chunk-start len tokens] (let [step (tokenize-spans-chunks-step src chunk-start len tokens) done (vector-get step 0)] (if (= done 1) (vector-get step 2) (tokenize-spans-chunks-loop-13 src (vector-get step 1) len (vector-get step 2)))))
(defn tokenize-spans-chunks-loop-13 [src chunk-start len tokens] (let [step (tokenize-spans-chunks-step src chunk-start len tokens) done (vector-get step 0)] (if (= done 1) (vector-get step 2) (tokenize-spans-chunks-loop-14 src (vector-get step 1) len (vector-get step 2)))))
(defn tokenize-spans-chunks-loop-14 [src chunk-start len tokens] (let [step (tokenize-spans-chunks-step src chunk-start len tokens) done (vector-get step 0)] (if (= done 1) (vector-get step 2) (tokenize-spans-chunks-loop-15 src (vector-get step 1) len (vector-get step 2)))))
(defn tokenize-spans-chunks-loop-15 [src chunk-start len tokens] (let [step (tokenize-spans-chunks-step src chunk-start len tokens) done (vector-get step 0)] (if (= done 1) (vector-get step 2) (tokenize-spans-chunks-loop-16 src (vector-get step 1) len (vector-get step 2)))))
(defn tokenize-spans-chunks-loop-16 [src chunk-start len tokens] (let [step (tokenize-spans-chunks-step src chunk-start len tokens) done (vector-get step 0)] (if (= done 1) (vector-get step 2) (tokenize-spans-chunks-loop-17 src (vector-get step 1) len (vector-get step 2)))))
(defn tokenize-spans-chunks-loop-17 [src chunk-start len tokens] (let [step (tokenize-spans-chunks-step src chunk-start len tokens) done (vector-get step 0)] (if (= done 1) (vector-get step 2) (tokenize-spans-chunks-loop-18 src (vector-get step 1) len (vector-get step 2)))))
(defn tokenize-spans-chunks-loop-18 [src chunk-start len tokens] (let [step (tokenize-spans-chunks-step src chunk-start len tokens) done (vector-get step 0)] (if (= done 1) (vector-get step 2) (tokenize-spans-chunks-loop-19 src (vector-get step 1) len (vector-get step 2)))))
(defn tokenize-spans-chunks-loop-19 [src chunk-start len tokens] (let [step (tokenize-spans-chunks-step src chunk-start len tokens) done (vector-get step 0)] (if (= done 1) (vector-get step 2) (tokenize-spans-chunks-loop-20 src (vector-get step 1) len (vector-get step 2)))))
(defn tokenize-spans-chunks-loop-20 [src chunk-start len tokens] (let [step (tokenize-spans-chunks-step src chunk-start len tokens) done (vector-get step 0)] (if (= done 1) (vector-get step 2) (tokenize-spans-chunks-loop-21 src (vector-get step 1) len (vector-get step 2)))))
(defn tokenize-spans-chunks-loop-21 [src chunk-start len tokens] (let [step (tokenize-spans-chunks-step src chunk-start len tokens) done (vector-get step 0)] (if (= done 1) (vector-get step 2) (tokenize-spans-chunks-loop-22 src (vector-get step 1) len (vector-get step 2)))))
(defn tokenize-spans-chunks-loop-22 [src chunk-start len tokens] (let [step (tokenize-spans-chunks-step src chunk-start len tokens) done (vector-get step 0)] (if (= done 1) (vector-get step 2) (tokenize-spans-chunks-loop-23 src (vector-get step 1) len (vector-get step 2)))))
(defn tokenize-spans-chunks-loop-23 [src chunk-start len tokens] (let [step (tokenize-spans-chunks-step src chunk-start len tokens) done (vector-get step 0)] (if (= done 1) (vector-get step 2) (tokenize-spans-chunks-loop-24 src (vector-get step 1) len (vector-get step 2)))))
(defn tokenize-spans-chunks-loop-24 [src chunk-start len tokens] (let [step (tokenize-spans-chunks-step src chunk-start len tokens) done (vector-get step 0)] (if (= done 1) (vector-get step 2) (tokenize-spans-chunks-loop-25 src (vector-get step 1) len (vector-get step 2)))))
(defn tokenize-spans-chunks-loop-25 [src chunk-start len tokens] (let [step (tokenize-spans-chunks-step src chunk-start len tokens) done (vector-get step 0)] (if (= done 1) (vector-get step 2) (tokenize-spans-chunks-loop-26 src (vector-get step 1) len (vector-get step 2)))))
(defn tokenize-spans-chunks-loop-26 [src chunk-start len tokens] (let [step (tokenize-spans-chunks-step src chunk-start len tokens) done (vector-get step 0)] (if (= done 1) (vector-get step 2) (tokenize-spans-chunks-loop-27 src (vector-get step 1) len (vector-get step 2)))))
(defn tokenize-spans-chunks-loop-27 [src chunk-start len tokens] (let [step (tokenize-spans-chunks-step src chunk-start len tokens) done (vector-get step 0)] (if (= done 1) (vector-get step 2) (tokenize-spans-chunks-loop-28 src (vector-get step 1) len (vector-get step 2)))))
(defn tokenize-spans-chunks-loop-28 [src chunk-start len tokens] (let [step (tokenize-spans-chunks-step src chunk-start len tokens) done (vector-get step 0)] (if (= done 1) (vector-get step 2) (tokenize-spans-chunks-loop-29 src (vector-get step 1) len (vector-get step 2)))))
(defn tokenize-spans-chunks-loop-29 [src chunk-start len tokens] (let [step (tokenize-spans-chunks-step src chunk-start len tokens) done (vector-get step 0)] (if (= done 1) (vector-get step 2) (tokenize-spans-chunks-loop-30 src (vector-get step 1) len (vector-get step 2)))))
(defn tokenize-spans-chunks-loop-30 [src chunk-start len tokens] (let [step (tokenize-spans-chunks-step src chunk-start len tokens) done (vector-get step 0)] (if (= done 1) (vector-get step 2) (tokenize-spans-chunks-loop-31 src (vector-get step 1) len (vector-get step 2)))))
(defn tokenize-spans-chunks-loop-31 [src chunk-start len tokens] (let [step (tokenize-spans-chunks-step src chunk-start len tokens) done (vector-get step 0)] (if (= done 1) (vector-get step 2) (tokenize-spans-chunks-loop-32 src (vector-get step 1) len (vector-get step 2)))))
(defn tokenize-spans-chunks-loop-32 [src chunk-start len tokens] (let [step (tokenize-spans-chunks-step src chunk-start len tokens) done (vector-get step 0)] (if (= done 1) (vector-get step 2) (tokenize-spans-chunks-loop-33 src (vector-get step 1) len (vector-get step 2)))))
(defn tokenize-spans-chunks-loop-33 [src chunk-start len tokens] (let [step (tokenize-spans-chunks-step src chunk-start len tokens) done (vector-get step 0)] (if (= done 1) (vector-get step 2) (tokenize-spans-chunks-loop-34 src (vector-get step 1) len (vector-get step 2)))))
(defn tokenize-spans-chunks-loop-34 [src chunk-start len tokens] (let [step (tokenize-spans-chunks-step src chunk-start len tokens) done (vector-get step 0)] (if (= done 1) (vector-get step 2) (tokenize-spans-chunks-loop-35 src (vector-get step 1) len (vector-get step 2)))))
(defn tokenize-spans-chunks-loop-35 [src chunk-start len tokens] (let [step (tokenize-spans-chunks-step src chunk-start len tokens) done (vector-get step 0)] (if (= done 1) (vector-get step 2) (tokenize-spans-chunks-loop-36 src (vector-get step 1) len (vector-get step 2)))))
(defn tokenize-spans-chunks-loop-36 [src chunk-start len tokens] (let [step (tokenize-spans-chunks-step src chunk-start len tokens) done (vector-get step 0)] (if (= done 1) (vector-get step 2) (tokenize-spans-chunks-loop-37 src (vector-get step 1) len (vector-get step 2)))))
(defn tokenize-spans-chunks-loop-37 [src chunk-start len tokens] (let [step (tokenize-spans-chunks-step src chunk-start len tokens) done (vector-get step 0)] (if (= done 1) (vector-get step 2) (tokenize-spans-chunks-loop-38 src (vector-get step 1) len (vector-get step 2)))))
(defn tokenize-spans-chunks-loop-38 [src chunk-start len tokens] (let [step (tokenize-spans-chunks-step src chunk-start len tokens) done (vector-get step 0)] (if (= done 1) (vector-get step 2) (tokenize-spans-chunks-loop-39 src (vector-get step 1) len (vector-get step 2)))))
(defn tokenize-spans-chunks-loop-39 [src chunk-start len tokens] (let [step (tokenize-spans-chunks-step src chunk-start len tokens) done (vector-get step 0)] (if (= done 1) (vector-get step 2) (tokenize-spans-chunks-loop-40 src (vector-get step 1) len (vector-get step 2)))))
(defn tokenize-spans-chunks-loop-40 [src chunk-start len tokens] (let [step (tokenize-spans-chunks-step src chunk-start len tokens) done (vector-get step 0)] (if (= done 1) (vector-get step 2) (tokenize-spans-chunks-loop-41 src (vector-get step 1) len (vector-get step 2)))))
(defn tokenize-spans-chunks-loop-41 [src chunk-start len tokens] (let [step (tokenize-spans-chunks-step src chunk-start len tokens) done (vector-get step 0)] (if (= done 1) (vector-get step 2) (tokenize-spans-chunks-loop-42 src (vector-get step 1) len (vector-get step 2)))))
(defn tokenize-spans-chunks-loop-42 [src chunk-start len tokens] (let [step (tokenize-spans-chunks-step src chunk-start len tokens) done (vector-get step 0)] (if (= done 1) (vector-get step 2) (tokenize-spans-chunks-loop-43 src (vector-get step 1) len (vector-get step 2)))))
(defn tokenize-spans-chunks-loop-43 [src chunk-start len tokens] (let [step (tokenize-spans-chunks-step src chunk-start len tokens) done (vector-get step 0)] (if (= done 1) (vector-get step 2) (tokenize-spans-chunks-loop-44 src (vector-get step 1) len (vector-get step 2)))))
(defn tokenize-spans-chunks-loop-44 [src chunk-start len tokens] (let [step (tokenize-spans-chunks-step src chunk-start len tokens) done (vector-get step 0)] (if (= done 1) (vector-get step 2) (tokenize-spans-chunks-loop-45 src (vector-get step 1) len (vector-get step 2)))))
(defn tokenize-spans-chunks-loop-45 [src chunk-start len tokens] (let [step (tokenize-spans-chunks-step src chunk-start len tokens) done (vector-get step 0)] (if (= done 1) (vector-get step 2) (tokenize-spans-chunks-loop-46 src (vector-get step 1) len (vector-get step 2)))))
(defn tokenize-spans-chunks-loop-46 [src chunk-start len tokens] (let [step (tokenize-spans-chunks-step src chunk-start len tokens) done (vector-get step 0)] (if (= done 1) (vector-get step 2) (tokenize-spans-chunks-loop-47 src (vector-get step 1) len (vector-get step 2)))))
(defn tokenize-spans-chunks-loop-47 [src chunk-start len tokens] (let [step (tokenize-spans-chunks-step src chunk-start len tokens) done (vector-get step 0)] (if (= done 1) (vector-get step 2) (tokenize-spans-chunks-loop-48 src (vector-get step 1) len (vector-get step 2)))))
(defn tokenize-spans-chunks-loop-48 [src chunk-start len tokens] (let [step (tokenize-spans-chunks-step src chunk-start len tokens) done (vector-get step 0)] (if (= done 1) (vector-get step 2) (tokenize-spans-chunks-loop-49 src (vector-get step 1) len (vector-get step 2)))))
(defn tokenize-spans-chunks-loop-49 [src chunk-start len tokens] (let [step (tokenize-spans-chunks-step src chunk-start len tokens) done (vector-get step 0)] (if (= done 1) (vector-get step 2) (tokenize-spans-chunks-loop-50 src (vector-get step 1) len (vector-get step 2)))))
(defn tokenize-spans-chunks-loop-50 [src chunk-start len tokens] (let [step (tokenize-spans-chunks-step src chunk-start len tokens) done (vector-get step 0)] (if (= done 1) (vector-get step 2) (tokenize-spans-chunks-loop-51 src (vector-get step 1) len (vector-get step 2)))))
(defn tokenize-spans-chunks-loop-51 [src chunk-start len tokens] (let [step (tokenize-spans-chunks-step src chunk-start len tokens) done (vector-get step 0)] (if (= done 1) (vector-get step 2) (tokenize-spans-chunks-loop-52 src (vector-get step 1) len (vector-get step 2)))))
(defn tokenize-spans-chunks-loop-52 [src chunk-start len tokens] (let [step (tokenize-spans-chunks-step src chunk-start len tokens) done (vector-get step 0)] (if (= done 1) (vector-get step 2) (tokenize-spans-chunks-loop-53 src (vector-get step 1) len (vector-get step 2)))))
(defn tokenize-spans-chunks-loop-53 [src chunk-start len tokens] (let [step (tokenize-spans-chunks-step src chunk-start len tokens) done (vector-get step 0)] (if (= done 1) (vector-get step 2) (tokenize-spans-chunks-loop-54 src (vector-get step 1) len (vector-get step 2)))))
(defn tokenize-spans-chunks-loop-54 [src chunk-start len tokens] (let [step (tokenize-spans-chunks-step src chunk-start len tokens) done (vector-get step 0)] (if (= done 1) (vector-get step 2) (tokenize-spans-chunks-loop-55 src (vector-get step 1) len (vector-get step 2)))))
(defn tokenize-spans-chunks-loop-55 [src chunk-start len tokens] (let [step (tokenize-spans-chunks-step src chunk-start len tokens) done (vector-get step 0)] (if (= done 1) (vector-get step 2) (tokenize-spans-chunks-loop-56 src (vector-get step 1) len (vector-get step 2)))))
(defn tokenize-spans-chunks-loop-56 [src chunk-start len tokens] (let [step (tokenize-spans-chunks-step src chunk-start len tokens) done (vector-get step 0)] (if (= done 1) (vector-get step 2) (tokenize-spans-chunks-loop-57 src (vector-get step 1) len (vector-get step 2)))))
(defn tokenize-spans-chunks-loop-57 [src chunk-start len tokens] (let [step (tokenize-spans-chunks-step src chunk-start len tokens) done (vector-get step 0)] (if (= done 1) (vector-get step 2) (tokenize-spans-chunks-loop-58 src (vector-get step 1) len (vector-get step 2)))))
(defn tokenize-spans-chunks-loop-58 [src chunk-start len tokens] (let [step (tokenize-spans-chunks-step src chunk-start len tokens) done (vector-get step 0)] (if (= done 1) (vector-get step 2) (tokenize-spans-chunks-loop-59 src (vector-get step 1) len (vector-get step 2)))))
(defn tokenize-spans-chunks-loop-59 [src chunk-start len tokens] (let [step (tokenize-spans-chunks-step src chunk-start len tokens) done (vector-get step 0)] (if (= done 1) (vector-get step 2) (tokenize-spans-chunks-loop-60 src (vector-get step 1) len (vector-get step 2)))))
(defn tokenize-spans-chunks-loop-60 [src chunk-start len tokens] (let [step (tokenize-spans-chunks-step src chunk-start len tokens) done (vector-get step 0)] (if (= done 1) (vector-get step 2) (tokenize-spans-chunks-loop-61 src (vector-get step 1) len (vector-get step 2)))))
(defn tokenize-spans-chunks-loop-61 [src chunk-start len tokens] (let [step (tokenize-spans-chunks-step src chunk-start len tokens) done (vector-get step 0)] (if (= done 1) (vector-get step 2) (tokenize-spans-chunks-loop-62 src (vector-get step 1) len (vector-get step 2)))))
(defn tokenize-spans-chunks-loop-62 [src chunk-start len tokens] (let [step (tokenize-spans-chunks-step src chunk-start len tokens) done (vector-get step 0)] (if (= done 1) (vector-get step 2) (tokenize-spans-chunks-loop-63 src (vector-get step 1) len (vector-get step 2)))))
(defn tokenize-spans-chunks-loop-63 [src chunk-start len tokens] (let [step (tokenize-spans-chunks-step src chunk-start len tokens) done (vector-get step 0)] (if (= done 1) (vector-get step 2) (tokenize-spans-chunks-loop-64 src (vector-get step 1) len (vector-get step 2)))))
(defn tokenize-spans-chunks-loop-64 [src chunk-start len tokens] (let [step (tokenize-spans-chunks-step src chunk-start len tokens) done (vector-get step 0)] (if (= done 1) (vector-get step 2) (tokenize-spans-chunks-loop-65 src (vector-get step 1) len (vector-get step 2)))))
(defn tokenize-spans-chunks-loop-65 [src chunk-start len tokens] (let [step (tokenize-spans-chunks-step src chunk-start len tokens) done (vector-get step 0)] (if (= done 1) (vector-get step 2) (tokenize-spans-chunks-loop-66 src (vector-get step 1) len (vector-get step 2)))))
(defn tokenize-spans-chunks-loop-66 [src chunk-start len tokens] (let [step (tokenize-spans-chunks-step src chunk-start len tokens) done (vector-get step 0)] (if (= done 1) (vector-get step 2) (tokenize-spans-chunks-loop-67 src (vector-get step 1) len (vector-get step 2)))))
(defn tokenize-spans-chunks-loop-67 [src chunk-start len tokens] (let [step (tokenize-spans-chunks-step src chunk-start len tokens) done (vector-get step 0)] (if (= done 1) (vector-get step 2) (tokenize-spans-chunks-loop-68 src (vector-get step 1) len (vector-get step 2)))))
(defn tokenize-spans-chunks-loop-68 [src chunk-start len tokens] (let [step (tokenize-spans-chunks-step src chunk-start len tokens) done (vector-get step 0)] (if (= done 1) (vector-get step 2) (tokenize-spans-chunks-loop-69 src (vector-get step 1) len (vector-get step 2)))))
(defn tokenize-spans-chunks-loop-69 [src chunk-start len tokens] (let [step (tokenize-spans-chunks-step src chunk-start len tokens) done (vector-get step 0)] (if (= done 1) (vector-get step 2) (tokenize-spans-chunks-loop-70 src (vector-get step 1) len (vector-get step 2)))))
(defn tokenize-spans-chunks-loop-70 [src chunk-start len tokens] (let [step (tokenize-spans-chunks-step src chunk-start len tokens) done (vector-get step 0)] (if (= done 1) (vector-get step 2) (tokenize-spans-chunks-loop-71 src (vector-get step 1) len (vector-get step 2)))))
(defn tokenize-spans-chunks-loop-71 [src chunk-start len tokens] (let [step (tokenize-spans-chunks-step src chunk-start len tokens) done (vector-get step 0)] (if (= done 1) (vector-get step 2) (tokenize-spans-chunks-loop-72 src (vector-get step 1) len (vector-get step 2)))))
(defn tokenize-spans-chunks-loop-72 [src chunk-start len tokens] (let [step (tokenize-spans-chunks-step src chunk-start len tokens) done (vector-get step 0)] (if (= done 1) (vector-get step 2) (tokenize-spans-chunks-loop-73 src (vector-get step 1) len (vector-get step 2)))))
(defn tokenize-spans-chunks-loop-73 [src chunk-start len tokens] (let [step (tokenize-spans-chunks-step src chunk-start len tokens) done (vector-get step 0)] (if (= done 1) (vector-get step 2) (tokenize-spans-chunks-loop-74 src (vector-get step 1) len (vector-get step 2)))))
(defn tokenize-spans-chunks-loop-74 [src chunk-start len tokens] (let [step (tokenize-spans-chunks-step src chunk-start len tokens) done (vector-get step 0)] (if (= done 1) (vector-get step 2) (tokenize-spans-chunks-loop-75 src (vector-get step 1) len (vector-get step 2)))))
(defn tokenize-spans-chunks-loop-75 [src chunk-start len tokens] (let [step (tokenize-spans-chunks-step src chunk-start len tokens) done (vector-get step 0)] (if (= done 1) (vector-get step 2) (tokenize-spans-chunks-loop-76 src (vector-get step 1) len (vector-get step 2)))))
(defn tokenize-spans-chunks-loop-76 [src chunk-start len tokens] (let [step (tokenize-spans-chunks-step src chunk-start len tokens) done (vector-get step 0)] (if (= done 1) (vector-get step 2) (tokenize-spans-chunks-loop-77 src (vector-get step 1) len (vector-get step 2)))))
(defn tokenize-spans-chunks-loop-77 [src chunk-start len tokens] (let [step (tokenize-spans-chunks-step src chunk-start len tokens) done (vector-get step 0)] (if (= done 1) (vector-get step 2) (tokenize-spans-chunks-loop-78 src (vector-get step 1) len (vector-get step 2)))))
(defn tokenize-spans-chunks-loop-78 [src chunk-start len tokens] (let [step (tokenize-spans-chunks-step src chunk-start len tokens) done (vector-get step 0)] (if (= done 1) (vector-get step 2) (tokenize-spans-chunks-loop-79 src (vector-get step 1) len (vector-get step 2)))))
(defn tokenize-spans-chunks-loop-79 [src chunk-start len tokens] (let [step (tokenize-spans-chunks-step src chunk-start len tokens) done (vector-get step 0)] (if (= done 1) (vector-get step 2) (tokenize-spans-chunks-loop-80 src (vector-get step 1) len (vector-get step 2)))))
(defn tokenize-spans-chunks-loop-80 [src chunk-start len tokens] (let [step (tokenize-spans-chunks-step src chunk-start len tokens) done (vector-get step 0)] (if (= done 1) (vector-get step 2) (tokenize-spans-chunks-loop-81 src (vector-get step 1) len (vector-get step 2)))))
(defn tokenize-spans-chunks-loop-81 [src chunk-start len tokens] (let [step (tokenize-spans-chunks-step src chunk-start len tokens) done (vector-get step 0)] (if (= done 1) (vector-get step 2) (tokenize-spans-chunks-loop-82 src (vector-get step 1) len (vector-get step 2)))))
(defn tokenize-spans-chunks-loop-82 [src chunk-start len tokens] (let [step (tokenize-spans-chunks-step src chunk-start len tokens) done (vector-get step 0)] (if (= done 1) (vector-get step 2) (tokenize-spans-chunks-loop-83 src (vector-get step 1) len (vector-get step 2)))))
(defn tokenize-spans-chunks-loop-83 [src chunk-start len tokens] (let [step (tokenize-spans-chunks-step src chunk-start len tokens) done (vector-get step 0)] (if (= done 1) (vector-get step 2) (tokenize-spans-chunks-loop-84 src (vector-get step 1) len (vector-get step 2)))))
(defn tokenize-spans-chunks-loop-84 [src chunk-start len tokens] (let [step (tokenize-spans-chunks-step src chunk-start len tokens) done (vector-get step 0)] (if (= done 1) (vector-get step 2) (tokenize-spans-chunks-loop-85 src (vector-get step 1) len (vector-get step 2)))))
(defn tokenize-spans-chunks-loop-85 [src chunk-start len tokens] (let [step (tokenize-spans-chunks-step src chunk-start len tokens) done (vector-get step 0)] (if (= done 1) (vector-get step 2) (tokenize-spans-chunks-loop-86 src (vector-get step 1) len (vector-get step 2)))))
(defn tokenize-spans-chunks-loop-86 [src chunk-start len tokens] (let [step (tokenize-spans-chunks-step src chunk-start len tokens) done (vector-get step 0)] (if (= done 1) (vector-get step 2) (tokenize-spans-chunks-loop-87 src (vector-get step 1) len (vector-get step 2)))))
(defn tokenize-spans-chunks-loop-87 [src chunk-start len tokens] (let [step (tokenize-spans-chunks-step src chunk-start len tokens) done (vector-get step 0)] (if (= done 1) (vector-get step 2) (tokenize-spans-chunks-loop-88 src (vector-get step 1) len (vector-get step 2)))))
(defn tokenize-spans-chunks-loop-88 [src chunk-start len tokens] (let [step (tokenize-spans-chunks-step src chunk-start len tokens) done (vector-get step 0)] (if (= done 1) (vector-get step 2) (tokenize-spans-chunks-loop-89 src (vector-get step 1) len (vector-get step 2)))))
(defn tokenize-spans-chunks-loop-89 [src chunk-start len tokens] (let [step (tokenize-spans-chunks-step src chunk-start len tokens) done (vector-get step 0)] (if (= done 1) (vector-get step 2) (tokenize-spans-chunks-loop-90 src (vector-get step 1) len (vector-get step 2)))))
(defn tokenize-spans-chunks-loop-90 [src chunk-start len tokens] (let [step (tokenize-spans-chunks-step src chunk-start len tokens) done (vector-get step 0)] (if (= done 1) (vector-get step 2) (tokenize-spans-chunks-loop-91 src (vector-get step 1) len (vector-get step 2)))))
(defn tokenize-spans-chunks-loop-91 [src chunk-start len tokens] (let [step (tokenize-spans-chunks-step src chunk-start len tokens) done (vector-get step 0)] (if (= done 1) (vector-get step 2) (tokenize-spans-chunks-loop-92 src (vector-get step 1) len (vector-get step 2)))))
(defn tokenize-spans-chunks-loop-92 [src chunk-start len tokens] (let [step (tokenize-spans-chunks-step src chunk-start len tokens) done (vector-get step 0)] (if (= done 1) (vector-get step 2) (tokenize-spans-chunks-loop-93 src (vector-get step 1) len (vector-get step 2)))))
(defn tokenize-spans-chunks-loop-93 [src chunk-start len tokens] (let [step (tokenize-spans-chunks-step src chunk-start len tokens) done (vector-get step 0)] (if (= done 1) (vector-get step 2) (tokenize-spans-chunks-loop-94 src (vector-get step 1) len (vector-get step 2)))))
(defn tokenize-spans-chunks-loop-94 [src chunk-start len tokens] (let [step (tokenize-spans-chunks-step src chunk-start len tokens) done (vector-get step 0)] (if (= done 1) (vector-get step 2) (tokenize-spans-chunks-loop-95 src (vector-get step 1) len (vector-get step 2)))))
(defn tokenize-spans-chunks-loop-95 [src chunk-start len tokens] (let [step (tokenize-spans-chunks-step src chunk-start len tokens) done (vector-get step 0)] (if (= done 1) (vector-get step 2) (tokenize-spans-chunks-loop-96 src (vector-get step 1) len (vector-get step 2)))))
(defn tokenize-spans-chunks-loop-96 [src chunk-start len tokens] (let [step (tokenize-spans-chunks-step src chunk-start len tokens) done (vector-get step 0)] (if (= done 1) (vector-get step 2) (tokenize-spans-chunks-loop-97 src (vector-get step 1) len (vector-get step 2)))))
(defn tokenize-spans-chunks-loop-97 [src chunk-start len tokens] (let [step (tokenize-spans-chunks-step src chunk-start len tokens) done (vector-get step 0)] (if (= done 1) (vector-get step 2) (tokenize-spans-chunks-loop-98 src (vector-get step 1) len (vector-get step 2)))))
(defn tokenize-spans-chunks-loop-98 [src chunk-start len tokens] (let [step (tokenize-spans-chunks-step src chunk-start len tokens) done (vector-get step 0)] (if (= done 1) (vector-get step 2) (tokenize-spans-chunks-loop-99 src (vector-get step 1) len (vector-get step 2)))))
(defn tokenize-spans-chunks-loop-99 [src chunk-start len tokens] (let [step (tokenize-spans-chunks-step src chunk-start len tokens) done (vector-get step 0)] (if (= done 1) (vector-get step 2) (tokenize-spans-chunks-loop-100 src (vector-get step 1) len (vector-get step 2)))))
(defn tokenize-spans-chunks-loop-100 [src chunk-start len tokens] (let [step (tokenize-spans-chunks-step src chunk-start len tokens) done (vector-get step 0)] (if (= done 1) (vector-get step 2) (tokenize-spans-chunks-loop-101 src (vector-get step 1) len (vector-get step 2)))))
(defn tokenize-spans-chunks-loop-101 [src chunk-start len tokens] (let [step (tokenize-spans-chunks-step src chunk-start len tokens) done (vector-get step 0)] (if (= done 1) (vector-get step 2) (tokenize-spans-chunks-loop-102 src (vector-get step 1) len (vector-get step 2)))))
(defn tokenize-spans-chunks-loop-102 [src chunk-start len tokens] (let [step (tokenize-spans-chunks-step src chunk-start len tokens) done (vector-get step 0)] (if (= done 1) (vector-get step 2) (tokenize-spans-chunks-loop-103 src (vector-get step 1) len (vector-get step 2)))))
(defn tokenize-spans-chunks-loop-103 [src chunk-start len tokens] (let [step (tokenize-spans-chunks-step src chunk-start len tokens) done (vector-get step 0)] (if (= done 1) (vector-get step 2) (tokenize-spans-chunks-loop-104 src (vector-get step 1) len (vector-get step 2)))))
(defn tokenize-spans-chunks-loop-104 [src chunk-start len tokens] (let [step (tokenize-spans-chunks-step src chunk-start len tokens) done (vector-get step 0)] (if (= done 1) (vector-get step 2) (tokenize-spans-chunks-loop-105 src (vector-get step 1) len (vector-get step 2)))))
(defn tokenize-spans-chunks-loop-105 [src chunk-start len tokens] (let [step (tokenize-spans-chunks-step src chunk-start len tokens) done (vector-get step 0)] (if (= done 1) (vector-get step 2) (tokenize-spans-chunks-loop-106 src (vector-get step 1) len (vector-get step 2)))))
(defn tokenize-spans-chunks-loop-106 [src chunk-start len tokens] (let [step (tokenize-spans-chunks-step src chunk-start len tokens) done (vector-get step 0)] (if (= done 1) (vector-get step 2) (tokenize-spans-chunks-loop-107 src (vector-get step 1) len (vector-get step 2)))))
(defn tokenize-spans-chunks-loop-107 [src chunk-start len tokens] (let [step (tokenize-spans-chunks-step src chunk-start len tokens) done (vector-get step 0)] (if (= done 1) (vector-get step 2) (tokenize-spans-chunks-loop-108 src (vector-get step 1) len (vector-get step 2)))))
(defn tokenize-spans-chunks-loop-108 [src chunk-start len tokens] (let [step (tokenize-spans-chunks-step src chunk-start len tokens) done (vector-get step 0)] (if (= done 1) (vector-get step 2) (tokenize-spans-chunks-loop-109 src (vector-get step 1) len (vector-get step 2)))))
(defn tokenize-spans-chunks-loop-109 [src chunk-start len tokens] (let [step (tokenize-spans-chunks-step src chunk-start len tokens) done (vector-get step 0)] (if (= done 1) (vector-get step 2) (tokenize-spans-chunks-loop-110 src (vector-get step 1) len (vector-get step 2)))))
(defn tokenize-spans-chunks-loop-110 [src chunk-start len tokens] (let [step (tokenize-spans-chunks-step src chunk-start len tokens) done (vector-get step 0)] (if (= done 1) (vector-get step 2) (tokenize-spans-chunks-loop-111 src (vector-get step 1) len (vector-get step 2)))))
(defn tokenize-spans-chunks-loop-111 [src chunk-start len tokens] (let [step (tokenize-spans-chunks-step src chunk-start len tokens) done (vector-get step 0)] (if (= done 1) (vector-get step 2) (tokenize-spans-chunks-loop-112 src (vector-get step 1) len (vector-get step 2)))))
(defn tokenize-spans-chunks-loop-112 [src chunk-start len tokens] (let [step (tokenize-spans-chunks-step src chunk-start len tokens) done (vector-get step 0)] (if (= done 1) (vector-get step 2) (tokenize-spans-chunks-loop-113 src (vector-get step 1) len (vector-get step 2)))))
(defn tokenize-spans-chunks-loop-113 [src chunk-start len tokens] (let [step (tokenize-spans-chunks-step src chunk-start len tokens) done (vector-get step 0)] (if (= done 1) (vector-get step 2) (tokenize-spans-chunks-loop-114 src (vector-get step 1) len (vector-get step 2)))))
(defn tokenize-spans-chunks-loop-114 [src chunk-start len tokens] (let [step (tokenize-spans-chunks-step src chunk-start len tokens) done (vector-get step 0)] (if (= done 1) (vector-get step 2) (tokenize-spans-chunks-loop-115 src (vector-get step 1) len (vector-get step 2)))))
(defn tokenize-spans-chunks-loop-115 [src chunk-start len tokens] (let [step (tokenize-spans-chunks-step src chunk-start len tokens) done (vector-get step 0)] (if (= done 1) (vector-get step 2) (tokenize-spans-chunks-loop-116 src (vector-get step 1) len (vector-get step 2)))))
(defn tokenize-spans-chunks-loop-116 [src chunk-start len tokens] (let [step (tokenize-spans-chunks-step src chunk-start len tokens) done (vector-get step 0)] (if (= done 1) (vector-get step 2) (tokenize-spans-chunks-loop-117 src (vector-get step 1) len (vector-get step 2)))))
(defn tokenize-spans-chunks-loop-117 [src chunk-start len tokens] (let [step (tokenize-spans-chunks-step src chunk-start len tokens) done (vector-get step 0)] (if (= done 1) (vector-get step 2) (tokenize-spans-chunks-loop-118 src (vector-get step 1) len (vector-get step 2)))))
(defn tokenize-spans-chunks-loop-118 [src chunk-start len tokens] (let [step (tokenize-spans-chunks-step src chunk-start len tokens) done (vector-get step 0)] (if (= done 1) (vector-get step 2) (tokenize-spans-chunks-loop-119 src (vector-get step 1) len (vector-get step 2)))))
(defn tokenize-spans-chunks-loop-119 [src chunk-start len tokens] (let [step (tokenize-spans-chunks-step src chunk-start len tokens) done (vector-get step 0)] (if (= done 1) (vector-get step 2) (tokenize-spans-chunks-loop-120 src (vector-get step 1) len (vector-get step 2)))))
(defn tokenize-spans-chunks-loop-120 [src chunk-start len tokens] (let [step (tokenize-spans-chunks-step src chunk-start len tokens) done (vector-get step 0)] (if (= done 1) (vector-get step 2) (tokenize-spans-chunks-loop-121 src (vector-get step 1) len (vector-get step 2)))))
(defn tokenize-spans-chunks-loop-121 [src chunk-start len tokens] (let [step (tokenize-spans-chunks-step src chunk-start len tokens) done (vector-get step 0)] (if (= done 1) (vector-get step 2) (tokenize-spans-chunks-loop-122 src (vector-get step 1) len (vector-get step 2)))))
(defn tokenize-spans-chunks-loop-122 [src chunk-start len tokens] (let [step (tokenize-spans-chunks-step src chunk-start len tokens) done (vector-get step 0)] (if (= done 1) (vector-get step 2) (tokenize-spans-chunks-loop-123 src (vector-get step 1) len (vector-get step 2)))))
(defn tokenize-spans-chunks-loop-123 [src chunk-start len tokens] (let [step (tokenize-spans-chunks-step src chunk-start len tokens) done (vector-get step 0)] (if (= done 1) (vector-get step 2) (tokenize-spans-chunks-loop-124 src (vector-get step 1) len (vector-get step 2)))))
(defn tokenize-spans-chunks-loop-124 [src chunk-start len tokens] (let [step (tokenize-spans-chunks-step src chunk-start len tokens) done (vector-get step 0)] (if (= done 1) (vector-get step 2) (tokenize-spans-chunks-loop-125 src (vector-get step 1) len (vector-get step 2)))))
(defn tokenize-spans-chunks-loop-125 [src chunk-start len tokens] (let [step (tokenize-spans-chunks-step src chunk-start len tokens) done (vector-get step 0)] (if (= done 1) (vector-get step 2) (tokenize-spans-chunks-loop-126 src (vector-get step 1) len (vector-get step 2)))))
(defn tokenize-spans-chunks-loop-126 [src chunk-start len tokens] (let [step (tokenize-spans-chunks-step src chunk-start len tokens) done (vector-get step 0)] (if (= done 1) (vector-get step 2) (tokenize-spans-chunks-loop-127 src (vector-get step 1) len (vector-get step 2)))))
(defn tokenize-spans-chunks-loop-127 [src chunk-start len tokens] (let [step (tokenize-spans-chunks-step src chunk-start len tokens) done (vector-get step 0)] (if (= done 1) (vector-get step 2) (tokenize-spans-chunks-loop-128 src (vector-get step 1) len (vector-get step 2)))))
(defn tokenize-spans-chunks-loop-128 [src chunk-start len tokens] (let [step (tokenize-spans-chunks-step src chunk-start len tokens) done (vector-get step 0)] (if (= done 1) (vector-get step 2) (tokenize-spans-chunks-loop src (vector-get step 1) len (vector-get step 2)))))

(defn tokenize-spans-chunks-loop [src chunk-start len tokens]
  (tokenize-spans-chunks-loop-1 src chunk-start len tokens))

(defn tokenize-spans-loop [src pos len tokens]
  (tokenize-spans-loop-1 src pos len tokens))

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
