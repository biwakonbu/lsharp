(module Parser)
(import Token)
(import AST)

;; Parser.ls - L# セルフホスティング: 再帰降下パーサー
;;
;; Lexer が出力したトークン列 (3つ組 Vector) を受け取り、AST を構築する。
;; S 式構文なので、パーサーは比較的シンプル。
;;
;; === AST ノード表現 (vector ベース) ===
;; [tag, ...data]
;; tag=1: int [1, value]
;; tag=2: bool [2, 0/1]
;; tag=3: string [3, start, end]  (ソース位置参照)
;; tag=4: var [4, name-hash]  (名前ハッシュで識別)
;; tag=5: apply [5, func-node, arg-count, arg1, arg2, ...]
;; tag=6: if [6, cond, then, else]
;; tag=7: let [7, name-hash, init, body]
;; tag=8: lambda [8, param-count, param-hash1, ..., body]
;; tag=9: do [9, expr-count, expr1, expr2, ...]
;; tag=10: match [10, scrutinee, arm-count, pat1, body1, ...]
;; tag=20: defn [20, name-hash, param-count, param-hash1, ..., body]

;; トークン種別定数 (Token.ls より)
;; 0=LParen, 1=RParen, 2=LBracket, 3=RBracket, 4=LBrace, 5=RBrace
;; 10=Int, 11=Float, 12=String, 13=BoolTrue, 14=BoolFalse, 20=Symbol
;; 30=Defn, 31=Let, 32=If, 33=Match, 34=Type, 35=Fn, 36=Do
;; 37=Module, 38=Import, 39=Record, 40=Trait, 41=Impl, 42=Where
;; 50=Colon, 51=Arrow, 52=Pipe, 53=Dot, 99=Eof

;; === 3つ組トークンアクセス ===

;; N 番目のトークンの kind
(defn span-kind [spans n]
  (vector-get spans (* n 3)))

;; N 番目のトークンの start
(defn span-start [spans n]
  (vector-get spans (+ (* n 3) 1)))

;; N 番目のトークンの end
(defn span-end [spans n]
  (vector-get spans (+ (* n 3) 2)))

;; === パーサー状態 ===

;; 現在のトークン kind を取得
(defn p-current [spans pos-ref]
  (span-kind spans (ref-get pos-ref)))

;; パーサー位置を1つ進める
(defn p-advance [pos-ref]
  (ref-set pos-ref (+ (ref-get pos-ref) 1)))

;; 現在のトークンの start を取得
(defn p-start [spans pos-ref]
  (span-start spans (ref-get pos-ref)))

;; 現在のトークンの end を取得
(defn p-end [spans pos-ref]
  (span-end spans (ref-get pos-ref)))

;; 期待するトークンを消費 (種別が一致しなければ 0 を返す)
(defn p-expect [spans pos-ref expected]
  (if (== (p-current spans pos-ref) expected)
    (do (p-advance pos-ref) 1)
    0))

;; === 名前ハッシュ ===
;; 同じ名前は異なる位置に出現しても同一キーになる
(defn name-hash-loop [src pos end acc]
  (if (>= pos end) acc
    (name-hash-loop src (+ pos 1) end
      (+ (string-char-at src pos) (* acc 31)))))

(defn name-hash [src start end]
  (name-hash-loop src start end 0))

;; === 数値パース ===

(defn parse-int-from-str [src pos end acc]
  (if (>= pos end) acc
    (let [digit (- (string-char-at src pos) 48)]
      (parse-int-from-str src (+ pos 1) end (+ (* acc 10) digit)))))

;; === AST ノード構築ヘルパー ===

;; 整数リテラルノード: [1, value]
(defn make-int-node [value]
  (vector-push (vector-push (vector-new 2) 1) value))

;; 真偽値ノード: [2, 0/1]
(defn make-bool-node [b]
  (vector-push (vector-push (vector-new 2) 2) b))

;; 変数参照ノード: [4, name-hash]
(defn make-var-node [h]
  (vector-push (vector-push (vector-new 2) 4) h))

;; 文字列ノード: [3, start, end]
(defn make-string-node [start end]
  (vector-push (vector-push (vector-push (vector-new 3) 3) start) end))

;; === メインパーサー (v3: span ベース) ===

;; 式のパース (メインディスパッチ)
(defn parse-expr-v3 [spans pos-ref src]
  (let [kind (p-current spans pos-ref)]
    (if (== kind 10)  ;; Int
      (let [start (p-start spans pos-ref)
            end (p-end spans pos-ref)
            value (parse-int-from-str src start end 0)]
        (do (p-advance pos-ref)
            (make-int-node value)))
      (if (== kind 13)  ;; true
        (do (p-advance pos-ref) (make-bool-node 1))
        (if (== kind 14)  ;; false
          (do (p-advance pos-ref) (make-bool-node 0))
          (if (== kind 12)  ;; String
            (let [start (p-start spans pos-ref)
                  end (p-end spans pos-ref)]
              (do (p-advance pos-ref)
                  (make-string-node (+ start 1) (- end 1)))) ;; 引用符を除く
            (if (== kind 20)  ;; Symbol (変数参照)
              (let [start (p-start spans pos-ref)
                    end (p-end spans pos-ref)
                    h (name-hash src start end)]
                (do (p-advance pos-ref)
                    (make-var-node h)))
              (if (== kind 0)  ;; LParen -> S 式
                (parse-sexp-v3 spans pos-ref src)
                ;; unknown token
                (do (p-advance pos-ref)
                    (make-int-node 0))))))))))

;; S 式のパース (( の後のキーワードディスパッチ)
(defn parse-sexp-v3 [spans pos-ref src]
  (do
    (p-advance pos-ref)  ;; ( を消費
    (let [kind (p-current spans pos-ref)]
      (if (== kind 32)  ;; if
        (parse-if-v3 spans pos-ref src)
        (if (== kind 31)  ;; let
          (parse-let-v3 spans pos-ref src)
          (if (== kind 36)  ;; do
            (parse-do-v3 spans pos-ref src)
            (if (== kind 33)  ;; match
              (parse-match-v3 spans pos-ref src)
              (if (== kind 35)  ;; fn (lambda)
                (parse-lambda-v3 spans pos-ref src)
                (if (== kind 30)  ;; defn
                  (parse-defn-v3 spans pos-ref src)
                  (if (== kind 34)  ;; type
                    (parse-type-v3 spans pos-ref src)
                    (if (== kind 37)  ;; module
                      (parse-module-v3 spans pos-ref src)
                      (if (== kind 38)  ;; import
                        (parse-import-v3 spans pos-ref src)
                        ;; 関数適用 (apply)
                        (parse-apply-v3 spans pos-ref src)))))))))))))

;; === if 式 ===
(defn parse-if-v3 [spans pos-ref src]
  (do
    (p-advance pos-ref)  ;; if を消費
    (let [cond-node (parse-expr-v3 spans pos-ref src)
          then-node (parse-expr-v3 spans pos-ref src)
          else-node (parse-expr-v3 spans pos-ref src)]
      (do
        (p-expect spans pos-ref 1)  ;; ) を消費
        (let [n (vector-new 8)]
          (vector-push (vector-push (vector-push (vector-push n 6)
            cond-node) then-node) else-node))))))

;; === let 式 (複数バインディング対応) ===
(defn parse-let-v3 [spans pos-ref src]
  (do
    (p-advance pos-ref)  ;; let を消費
    (p-expect spans pos-ref 2)  ;; [ を消費
    ;; 最初のバインディング
    (let [ns (p-start spans pos-ref)
          ne (p-end spans pos-ref)
          nh (name-hash src ns ne)]
      (do
        (p-advance pos-ref)  ;; name を消費
        (let [init (parse-expr-v3 spans pos-ref src)]
          ;; 追加バインディングがあるかチェック
          (if (== (p-current spans pos-ref) 3)  ;; ] で終了
            (do
              (p-advance pos-ref)  ;; ] を消費
              (let [body (parse-expr-v3 spans pos-ref src)]
                (do
                  (p-expect spans pos-ref 1)  ;; ) を消費
                  (let [n (vector-new 8)]
                    (vector-push (vector-push (vector-push (vector-push n 7)
                      nh) init) body)))))
            ;; 複数バインディング: 次のバインディングを body として再帰
            (let [ns2 (p-start spans pos-ref)
                  ne2 (p-end spans pos-ref)
                  nh2 (name-hash src ns2 ne2)]
              (do
                (p-advance pos-ref)  ;; name2 を消費
                (let [init2 (parse-expr-v3 spans pos-ref src)
                      ;; 残りのバインディングを処理
                      rest-body (parse-let-rest-v3 spans pos-ref src)]
                  ;; 内側の let を構築
                  (let [inner (vector-push (vector-push (vector-push
                                (vector-push (vector-new 8) 7) nh2) init2) rest-body)]
                    (do
                      (p-expect spans pos-ref 1)  ;; ) を消費
                      (let [n (vector-new 8)]
                        (vector-push (vector-push (vector-push (vector-push n 7)
                          nh) init) inner)))))))))))))

;; let の残りバインディングを処理
(defn parse-let-rest-v3 [spans pos-ref src]
  (if (== (p-current spans pos-ref) 3)  ;; ] に到達
    (do
      (p-advance pos-ref)  ;; ] を消費
      (parse-expr-v3 spans pos-ref src))  ;; body をパース
    ;; さらにバインディングがある
    (let [ns (p-start spans pos-ref)
          ne (p-end spans pos-ref)
          nh (name-hash src ns ne)]
      (do
        (p-advance pos-ref)  ;; name を消費
        (let [init (parse-expr-v3 spans pos-ref src)
              rest (parse-let-rest-v3 spans pos-ref src)]
          (let [n (vector-new 8)]
            (vector-push (vector-push (vector-push (vector-push n 7)
              nh) init) rest)))))))

;; === do 式 ===
(defn parse-do-v3 [spans pos-ref src]
  (do
    (p-advance pos-ref)  ;; do を消費
    (let [result (vector-push (vector-push (vector-new 16) 9) 0)]  ;; [9, count=0(後で更新)]
      (parse-do-exprs-v3 spans pos-ref src result 0))))

;; do 内の式を収集
(defn parse-do-exprs-v3 [spans pos-ref src result count]
  (if (== (p-current spans pos-ref) 1)  ;; ) で終了
    (do
      (p-advance pos-ref)  ;; ) を消費
      ;; count を更新 (index 1)
      result)
    (let [expr (parse-expr-v3 spans pos-ref src)]
      (parse-do-exprs-v3 spans pos-ref src
        (vector-push result expr) (+ count 1)))))

;; === match 式 ===
(defn parse-match-v3 [spans pos-ref src]
  (do
    (p-advance pos-ref)  ;; match を消費
    (let [scrutinee (parse-expr-v3 spans pos-ref src)
          result (vector-push (vector-push (vector-push (vector-new 16) 10)
                   scrutinee) 0)]  ;; [10, scrutinee, arm-count=0]
      (parse-match-arms-v3 spans pos-ref src result 0))))

;; match の腕を収集
(defn parse-match-arms-v3 [spans pos-ref src result count]
  (if (== (p-current spans pos-ref) 1)  ;; ) で終了
    (do (p-advance pos-ref) result)
    (if (== (p-current spans pos-ref) 2)  ;; [ -> arm
      (do
        (p-advance pos-ref)  ;; [ を消費
        (let [pat (parse-expr-v3 spans pos-ref src)
              body (parse-expr-v3 spans pos-ref src)]
          (do
            (p-expect spans pos-ref 3)  ;; ] を消費
            (parse-match-arms-v3 spans pos-ref src
              (vector-push (vector-push result pat) body)
              (+ count 1)))))
      ;; 不正なトークン -> スキップ
      (do (p-advance pos-ref)
          (parse-match-arms-v3 spans pos-ref src result count)))))

;; === lambda (fn) 式 ===
(defn parse-lambda-v3 [spans pos-ref src]
  (do
    (p-advance pos-ref)  ;; fn を消費
    (p-expect spans pos-ref 2)  ;; [ を消費
    (let [result (vector-push (vector-push (vector-new 8) 8) 0)]  ;; [8, param-count=0]
      (let [with-params (parse-params-v3 spans pos-ref src result 0)
            body (parse-expr-v3 spans pos-ref src)]
        (do
          (p-expect spans pos-ref 1)  ;; ) を消費
          (vector-push with-params body))))))

;; パラメータリストを収集 (名前ハッシュ)
(defn parse-params-v3 [spans pos-ref src result count]
  (if (== (p-current spans pos-ref) 3)  ;; ] で終了
    (do (p-advance pos-ref) result)
    (let [s (p-start spans pos-ref)
          e (p-end spans pos-ref)
          h (name-hash src s e)]
      (do
        (p-advance pos-ref)  ;; param を消費
        (parse-params-v3 spans pos-ref src
          (vector-push result h) (+ count 1))))))

;; === defn 式 ===
(defn parse-defn-v3 [spans pos-ref src]
  (do
    (p-advance pos-ref)  ;; defn を消費
    (let [ns (p-start spans pos-ref)
          ne (p-end spans pos-ref)
          nh (name-hash src ns ne)]
      (do
        (p-advance pos-ref)  ;; name を消費
        (p-expect spans pos-ref 2)  ;; [ を消費
        (let [result (vector-push (vector-push (vector-push (vector-new 8) 20) nh) 0)]
          (let [with-params (parse-params-v3 spans pos-ref src result 0)
                body (parse-expr-v3 spans pos-ref src)]
            (do
              (p-expect spans pos-ref 1)  ;; ) を消費
              (vector-push with-params body))))))))

;; === type 宣言 (簡易) ===
(defn parse-type-v3 [spans pos-ref src]
  (do
    (p-advance pos-ref)  ;; type を消費
    ;; ) まで読み飛ばし
    (parse-skip-to-close-v3 spans pos-ref 1)
    (make-int-node 0)))  ;; ダミーノード

;; === module 宣言 ===
(defn parse-module-v3 [spans pos-ref src]
  (do
    (p-advance pos-ref)  ;; module を消費
    (let [name-start (p-start spans pos-ref)]
      (do
        (p-advance pos-ref)  ;; name を消費
        (p-expect spans pos-ref 1)  ;; ) を消費
        (let [n (vector-new 4)]
          (vector-push (vector-push n 11) name-start))))))  ;; tag=11 for module

;; === import 宣言 ===
(defn parse-import-v3 [spans pos-ref src]
  (do
    (p-advance pos-ref)  ;; import を消費
    (let [name-start (p-start spans pos-ref)]
      (do
        (p-advance pos-ref)  ;; name を消費
        (p-expect spans pos-ref 1)  ;; ) を消費
        (let [n (vector-new 4)]
          (vector-push (vector-push n 12) name-start))))))  ;; tag=12 for import

;; === apply (関数呼び出し) ===
(defn parse-apply-v3 [spans pos-ref src]
  (let [func-node (parse-expr-v3 spans pos-ref src)
        result (vector-push (vector-push (vector-push (vector-new 8) 5) func-node) 0)]
    (parse-apply-args-v3 spans pos-ref src result 0)))

;; 引数を収集
(defn parse-apply-args-v3 [spans pos-ref src result count]
  (if (== (p-current spans pos-ref) 1)  ;; ) で終了
    (do (p-advance pos-ref) result)
    (let [arg (parse-expr-v3 spans pos-ref src)]
      (parse-apply-args-v3 spans pos-ref src
        (vector-push result arg) (+ count 1)))))

;; === Recovery + 診断収集 ===

;; 診断レコード: [severity code span message-hash]
;; severity: 0=error, 1=warning, 2=info
;; code: 整数エラーコード
;; span: ソース位置 (start)
;; message-hash: メッセージの名前ハッシュ
(defn make-diagnostic [severity code span message-hash]
  (let [d (vector-new 4)]
    (vector-push (vector-push (vector-push (vector-push d severity) code) span) message-hash)))

;; 診断コレクタ: 診断のベクタを管理
(defn collect-diagnostics []
  (vector-new 8))

;; 診断を追加
(defn add-diagnostic [diagnostics diag]
  (vector-push diagnostics diag))

;; 次の同期ポイント (閉じ括弧 or トップレベル) まで回復
;; kind=1 (RParen), kind=99 (EOF) で停止
(defn recover-to-next [spans pos-ref]
  (let [kind (p-current spans pos-ref)]
    (if (== kind 99) 0   ;; EOF で停止
      (if (== kind 1) 0  ;; ) で停止
        (do (p-advance pos-ref)
            (recover-to-next spans pos-ref))))))

;; recovery 付きパース: パースに失敗したら回復して診断を記録
;; 戻り値: [ast-node, diagnostics-vector]
(defn parse-with-recovery [spans pos-ref src diagnostics]
  (let [start-pos (ref-get pos-ref)
        kind (p-current spans pos-ref)]
    (if (== kind 99) ;; EOF
      (let [result (vector-new 2)]
        (vector-push (vector-push result (make-int-node 0)) diagnostics))
      ;; 不正なトークン (閉じ括弧が先に来た等) の場合 recovery
      (if (== kind 1) ;; 予期しない )
        (let [span (p-start spans pos-ref)
              diag (make-diagnostic 0 1001 span 0)
              diags (add-diagnostic diagnostics diag)]
          (do (p-advance pos-ref)
              (let [result (vector-new 2)]
                (vector-push (vector-push result (make-int-node 0)) diags))))
        (if (== kind 3) ;; 予期しない ]
          (let [span (p-start spans pos-ref)
                diag (make-diagnostic 0 1002 span 0)
                diags (add-diagnostic diagnostics diag)]
            (do (p-advance pos-ref)
                (let [result (vector-new 2)]
                  (vector-push (vector-push result (make-int-node 0)) diags))))
          ;; 通常パース
          (let [node (parse-expr-v3 spans pos-ref src)
                result (vector-new 2)]
            (vector-push (vector-push result node) diagnostics)))))))

;; === ユーティリティ ===

;; 対応する閉じ括弧まで読み飛ばし (ネスト対応)
(defn parse-skip-to-close-v3 [spans pos-ref depth]
  (if (<= depth 0) 0
    (let [kind (p-current spans pos-ref)]
      (do
        (p-advance pos-ref)
        (if (== kind 0)  ;; ( でネスト深くなる
          (parse-skip-to-close-v3 spans pos-ref (+ depth 1))
          (if (== kind 1)  ;; ) でネスト浅くなる
            (parse-skip-to-close-v3 spans pos-ref (- depth 1))
            (parse-skip-to-close-v3 spans pos-ref depth)))))))

;; === トップレベルパース ===

;; 複数のトップレベル式をパース
(defn parse-program-v3 [spans pos-ref src]
  (let [result (vector-new 16)]
    (parse-program-loop-v3 spans pos-ref src result)))

(defn parse-program-loop-v3 [spans pos-ref src result]
  (if (== (p-current spans pos-ref) 99)  ;; EOF
    result
    (let [expr (parse-expr-v3 spans pos-ref src)]
      (parse-program-loop-v3 spans pos-ref src
        (vector-push result expr)))))

;; === 旧 API (後方互換) ===

;; 現在のトークンを取得 (旧 kind-only 方式)
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
      0)))

;; 結果は整数エンコード: tag * 10000 + value
(defn parse-expr [tokens pos src src-positions]
  (let [tok (current-tok tokens pos)]
    (if (== tok 0)
      (do (advance pos)
        (let [result (parse-sexp tokens pos src src-positions)]
          (do (expect tokens pos 1) result)))
      (if (== tok 10)
        (do (advance pos) (+ (* 1 10000) 0))
        (if (== tok 13)
          (do (advance pos) (+ (* 2 10000) 1))
          (if (== tok 14)
            (do (advance pos) (+ (* 2 10000) 0))
            (if (== tok 20)
              (do (advance pos) (+ (* 4 10000) 0))
              0)))))))

(defn parse-sexp [tokens pos src src-positions]
  (let [tok (current-tok tokens pos)]
    (if (== tok 30) (do (advance pos) (+ (* 20 10000) 0))
      (if (== tok 31) (do (advance pos) (+ (* 7 10000) 0))
        (if (== tok 32) (do (advance pos) (+ (* 6 10000) 0))
          (if (== tok 33) (do (advance pos) (+ (* 10 10000) 0))
            (if (== tok 36) (do (advance pos) (+ (* 9 10000) 0))
              (+ (* 5 10000) 0))))))))

(defn node-tag [encoded]
  (/ encoded 10000))

(defn parse-toplevel [tokens pos src]
  (parse-expr tokens pos src (vector-new 0)))

;; エントリポイント (テスト用)
(defn main []
  (let [;; defn テスト
        tokens (vector-push (vector-push (vector-push (vector-push
                (vector-push (vector-push (vector-push (vector-push
                  (vector-new 8) 0) 30) 20) 2) 3) 10) 1) 99)
        pos (ref-new 0)
        result (parse-toplevel tokens pos "")
        ;; match テスト: (match x [1 10] [2 20])
        match-tokens (let [v (vector-new 16)]
                       (let [v1 (vector-push v 0)
                             v2 (vector-push v1 33)
                             v3 (vector-push v2 20)
                             v4 (vector-push v3 2)
                             v5 (vector-push v4 10)
                             v6 (vector-push v5 10)
                             v7 (vector-push v6 3)
                             v8 (vector-push v7 2)
                             v9 (vector-push v8 10)
                             v10 (vector-push v9 10)
                             v11 (vector-push v10 3)
                             v12 (vector-push v11 1)
                             v13 (vector-push v12 99)]
                         v13))
        match-pos (ref-new 0)
        match-result (parse-toplevel match-tokens match-pos "")
        ;; make-match-node テスト (旧 API ヘルパー)
        scr (make-int-node 5)
        mn (vector-push (vector-push (vector-push (vector-new 8) 10) scr) 2)
        mn1 (vector-push (vector-push mn 1) (make-int-node 10))
        mn2 (vector-push (vector-push mn1 2) (make-int-node 20))]
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
