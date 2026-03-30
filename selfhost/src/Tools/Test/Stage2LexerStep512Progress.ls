(module Tools.Test.Stage2LexerStep512Progress)
(import Syntax.Lexer)

;; プローブ入力: step-512 の動作を検証するためのリテラルソース片
;; 再帰ループを排除し、stage2 コンパイラの再帰深度を最小化する
(defn probe-source []
  "(defn main [] 42)")

;; プロダクション lexer パス: tokenize-spans-step-512 を直接呼び出す
;; 出力: source_len, done1, next1, count1 (step1 完了時は 4 行、未完了時は 7 行)
(defn main []
  (let [src (probe-source)
    step1 (tokenize-spans-step-512 src 0 (string-length src) (vector-new 32))]
    (do
      (print (string-length src))
      (print (vector-get step1 0))
      (print (vector-get step1 1))
      (print (token-count (vector-get step1 2)))
      0)))
