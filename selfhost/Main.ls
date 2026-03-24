;; Main.ls - L# セルフホスティング: 統合パイプライン
;;
;; Token/AST/IR/Compiler/WasmEmit モジュールを結合し、
;; AST 構築 -> IR 変換 -> Wasm バイナリ生成の統合パイプラインを検証する。
;;
;; P8-9 T4-1: WASI ファイル I/O 統合
;; - read-file でソースコード読み込み
;; - write-file で .wasm バイナリ出力
;;
;; P8-9 T4-2: モジュール結合
;; - 全 selfhost ファイルのコア機能をこのファイルに統合
;; - (将来的には import/module で分離予定)

;; ============================================================
;; Token 定数 (Token.ls より)
;; ============================================================

(defn tok-lparen [] 0)
(defn tok-rparen [] 1)
(defn tok-lbracket [] 2)
(defn tok-rbracket [] 3)
(defn tok-lbrace [] 4)
(defn tok-rbrace [] 5)
(defn tok-int [] 10)
(defn tok-float [] 11)
(defn tok-string [] 12)
(defn tok-bool-true [] 13)
(defn tok-bool-false [] 14)
(defn tok-symbol [] 20)
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
(defn tok-colon [] 50)
(defn tok-arrow [] 51)
(defn tok-pipe [] 52)
(defn tok-dot [] 53)
(defn tok-eof [] 99)

;; ============================================================
;; AST 定義 (AST.ls より)
;; ============================================================

;; AST ノード種別
(defn ast-lit-int [] 1)
(defn ast-lit-bool [] 2)
(defn ast-lit-string [] 3)
(defn ast-var [] 4)
(defn ast-apply [] 5)
(defn ast-if [] 6)
(defn ast-let [] 7)
(defn ast-lambda [] 8)
(defn ast-do [] 9)
(defn ast-defn [] 20)
(defn ast-type-decl [] 21)

;; AST ノード構築
(defn make-lit-int [value]
  (let [v (vector-new 2)]
    (vector-push (vector-push v 1) value)))

(defn make-lit-bool [b]
  (let [v (vector-new 2)]
    (vector-push (vector-push v 2) b)))

(defn make-var [name-hash]
  (let [v (vector-new 2)]
    (vector-push (vector-push v 4) name-hash)))

;; AST ノードアクセス
(defn ast-tag [node]
  (vector-get node 0))

;; ============================================================
;; IR 定義 (IR.ls より)
;; ============================================================

(defn ir-i64-const [] 1)
(defn ir-f64-const [] 2)
(defn ir-local-get [] 10)
(defn ir-local-set [] 11)
(defn ir-i64-add [] 20)
(defn ir-i64-sub [] 21)
(defn ir-i64-mul [] 22)
(defn ir-i64-div [] 23)
(defn ir-i64-eq [] 30)
(defn ir-i64-ne [] 31)
(defn ir-i64-lt [] 32)
(defn ir-i64-gt [] 33)
(defn ir-i64-le [] 34)
(defn ir-i64-ge [] 35)
(defn ir-call [] 40)
(defn ir-if [] 41)
(defn ir-block [] 42)
(defn ir-end [] 43)

;; IR 命令構築
(defn make-instr [opcode operand]
  (vector-push (vector-push (vector-new 2) opcode) operand))

(defn make-i64-const [value]
  (make-instr 1 value))

(defn make-local-get [idx]
  (make-instr 10 idx))

(defn make-call [func-idx]
  (make-instr 40 func-idx))

;; ============================================================
;; Compiler (Compiler.ls より、tag/op 定数は上で定義済み)
;; ============================================================

;; IR 命令: [opcode, operand]
(defn emit-instr [opcode operand]
  (vector-push (vector-push (vector-new 2) opcode) operand))

;; IR 命令列に命令を追加
(defn emit-to [instrs opcode operand]
  (vector-push instrs (emit-instr opcode operand)))

;; 環境 (変数名ハッシュ -> ローカルインデックス)
(defn env-new []
  (map-new))

(defn env-bind [env name-hash idx]
  (map-insert env name-hash idx))

(defn env-lookup [env name-hash]
  (map-get env name-hash))

;; AST ノードを IR 命令列に変換
(defn compile-expr [node env instrs]
  (let [tag (vector-get node 0)]
    (if (= tag 1)
      ;; 整数リテラル: i64.const value
      (emit-to instrs 1 (vector-get node 1))
      (if (= tag 2)
        ;; 真偽値リテラル: i64.const 0/1
        (emit-to instrs 1 (vector-get node 1))
        (if (= tag 4)
          ;; 変数参照: local.get idx
          (let [name-hash (vector-get node 1)
                idx (env-lookup env name-hash)]
            (if (= idx 0)
              (emit-to instrs 1 0)
              (emit-to instrs 10 idx)))
          ;; その他: 未実装
          (emit-to instrs 1 0))))))

;; ============================================================
;; WasmEmit (WasmEmit.ls より)
;; ============================================================

;; Wasm 定数
(defn wasm-magic-0 [] 0)
(defn wasm-magic-1 [] 97)
(defn wasm-magic-2 [] 115)
(defn wasm-magic-3 [] 109)
(defn wasm-version-0 [] 1)
(defn wasm-version-1 [] 0)
(defn wasm-version-2 [] 0)
(defn wasm-version-3 [] 0)

(defn section-type [] 1)
(defn section-function [] 3)
(defn section-export [] 7)
(defn section-code [] 10)

(defn wasm-i32 [] 127)
(defn wasm-i64 [] 126)
(defn wasm-funcref [] 112)

(defn wasm-end [] 11)
(defn wasm-i64-const [] 66)
(defn wasm-local-get [] 32)
(defn wasm-local-set [] 33)
(defn wasm-i64-add [] 124)
(defn wasm-i64-sub [] 125)
(defn wasm-i64-mul [] 126)
(defn wasm-call-op [] 16)
(defn wasm-return [] 15)

;; LEB128 エンコーディング (符号なし)
(defn leb128-u [value]
  (let [result (ref-new (vector-new 4))
        v (ref-new value)]
    (do
      (let [byte (% (ref-get v) 128)
            rest (/ (ref-get v) 128)]
        (if (= rest 0)
          (ref-set result (vector-push (ref-get result) byte))
          (do
            (ref-set result (vector-push (ref-get result) (+ byte 128)))
            (ref-set v rest)
            (let [byte2 (% (ref-get v) 128)
                  rest2 (/ (ref-get v) 128)]
              (if (= rest2 0)
                (ref-set result (vector-push (ref-get result) byte2))
                (do
                  (ref-set result (vector-push (ref-get result) (+ byte2 128)))
                  (ref-set v rest2)
                  (ref-set result (vector-push (ref-get result) (% (ref-get v) 128)))))))))
      (ref-get result))))

;; バイト列にバイトを追加
(defn emit-byte [bytes b]
  (vector-push bytes b))

;; Wasm ヘッダー (8 バイト: \0asm + version 1.0)
(defn emit-header []
  (let [h (vector-new 8)]
    (vector-push
      (vector-push
        (vector-push
          (vector-push
            (vector-push
              (vector-push
                (vector-push
                  (vector-push h 0)
                  97)
                115)
              109)
            1)
          0)
        0)
      0)))

;; Type セクション: () -> i64
(defn emit-type-section-main []
  (let [bytes (vector-new 16)]
    (let [b1 (emit-byte bytes 1)
          b2 (emit-byte b1 5)
          b3 (emit-byte b2 1)
          b4 (emit-byte b3 96)
          b5 (emit-byte b4 0)
          b6 (emit-byte b5 1)
          b7 (emit-byte b6 126)]
      b7)))

;; ============================================================
;; P8-9 T4-1: WASI ファイル I/O 統合
;; ============================================================

;; ソースファイルを読み込み、内容の長さを返す (パイプライン検証用)
(defn read-source [path]
  (if (file-exists? path)
    (let [content (read-file path)]
      (string-length content))
    0))

;; Wasm バイナリの最初の部分 (ヘッダー + Type セクション) を文字列として出力
;; (将来的には write-file で .wasm ファイルに書き出す)
(defn emit-wasm-header-bytes []
  (let [header (emit-header)
        type-sec (emit-type-section-main)
        ;; ヘッダー長 + Type セクション長
        total (+ (vector-length header) (vector-length type-sec))]
    total))

;; ============================================================
;; P8-9 T4-2: モジュール結合情報
;; ============================================================

;; 全 selfhost モジュールのリスト (依存順)
;; 1. Token.ls   - トークン定義
;; 2. AST.ls     - AST ノード定義
;; 3. IR.ls      - IR 命令定義
;; 4. Type.ls    - 型 ADT
;; 5. Lexer.ls   - 字句解析
;; 6. Parser.ls  - 構文解析
;; 7. TypeScheme.ls - 型スキーム
;; 8. Compiler.ls - AST -> IR 変換
;; 9. WasmEmit.ls - IR -> Wasm 変換
;; 10. Main.ls   - 統合パイプライン
;;
;; 現在のモジュール結合方式:
;; 各モジュールのコア関数をこのファイルに直接コピー
;; (L# にはまだ import/module による自動結合がないため)

(defn module-count [] 10)

;; ============================================================
;; T4-4: ミニトークナイザー (ソース文字列 → トークン列)
;; ============================================================

;; 文字種別判定
(defn is-whitespace [ch]
  (if (= ch 32) 1
    (if (= ch 10) 1
      (if (= ch 13) 1
        (if (= ch 9) 1
          0)))))

(defn is-digit [ch]
  (if (>= ch 48)
    (if (<= ch 57) 1 0)
    0))

;; 1文字トークンを追加してポインタを進める
(defn emit-tok [tokens pos kind]
  (do
    (ref-set tokens (vector-push (vector-push (ref-get tokens) kind) 0))
    (ref-set pos (+ (ref-get pos) 1))
    0))

;; シンボルの残りをスキップ (空白/括弧まで)
(defn mini-skip-symbol [src len pos]
  (do
    (ref-set pos (+ (ref-get pos) 1))
    (if (< (ref-get pos) len)
      (let [ch (string-char-at src (ref-get pos))]
        (if (= (is-whitespace ch) 1) 0
          (if (= ch 40) 0
            (if (= ch 41) 0
              (if (= ch 91) 0
                (if (= ch 93) 0
                  (mini-skip-symbol src len pos)))))))
      0)))

;; 数値を 1〜3 桁パース: [value, end-pos]
(defn scan-int [src pos len]
  (let [d0 (- (string-char-at src pos) 48)
        p1 (+ pos 1)]
    (if (< p1 len)
      (if (= (is-digit (string-char-at src p1)) 1)
        (let [d1 (- (string-char-at src p1) 48)
              p2 (+ p1 1)]
          (if (< p2 len)
            (if (= (is-digit (string-char-at src p2)) 1)
              (let [d2 (- (string-char-at src p2) 48)
                    result (vector-new 2)]
                (vector-push (vector-push result (+ (* (+ (* d0 10) d1) 10) d2)) (+ p2 1)))
              (let [result (vector-new 2)]
                (vector-push (vector-push result (+ (* d0 10) d1)) p2)))
            (let [result (vector-new 2)]
              (vector-push (vector-push result (+ (* d0 10) d1)) p2))))
        (let [result (vector-new 2)]
          (vector-push (vector-push result d0) p1)))
      (let [result (vector-new 2)]
        (vector-push (vector-push result d0) p1)))))

;; 'defn' キーワードかチェック (4文字先読み)
(defn is-defn-keyword [src pos len]
  (if (< (+ pos 3) len)
    (if (= (string-char-at src pos) 100)
      (if (= (string-char-at src (+ pos 1)) 101)
        (if (= (string-char-at src (+ pos 2)) 102)
          (if (= (string-char-at src (+ pos 3)) 110) 1 0)
          0)
        0)
      0)
    0))

;; 1 トークンをスキャン
(defn mini-scan-one [src len pos tokens]
  (if (>= (ref-get pos) len)
    0
    (let [ch (string-char-at src (ref-get pos))]
      (if (= (is-whitespace ch) 1)
        (do (ref-set pos (+ (ref-get pos) 1)) 1)
        (if (= ch 40)
          (emit-tok tokens pos 0)
          (if (= ch 41)
            (emit-tok tokens pos 1)
            (if (= ch 91)
              (emit-tok tokens pos 2)
              (if (= ch 93)
                (emit-tok tokens pos 3)
                (if (= (is-digit ch) 1)
                  (let [result (scan-int src (ref-get pos) len)]
                    (do
                      (ref-set tokens (vector-push (vector-push (ref-get tokens) 10) (vector-get result 0)))
                      (ref-set pos (vector-get result 1))
                      0))
                  (if (= (is-defn-keyword src (ref-get pos) len) 1)
                    (do
                      (ref-set tokens (vector-push (vector-push (ref-get tokens) 30) 0))
                      (ref-set pos (+ (ref-get pos) 4))
                      0)
                    (do
                      (ref-set tokens (vector-push (vector-push (ref-get tokens) 20) ch))
                      (mini-skip-symbol src len pos)
                      0)))))))))))

;; スキャンループ (最大 20 トークン展開)
(defn mini-scan-loop [src len pos tokens]
  (do
    (mini-scan-one src len pos tokens)
    (if (< (ref-get pos) len)
      (do (mini-scan-one src len pos tokens)
          (if (< (ref-get pos) len)
            (do (mini-scan-one src len pos tokens)
                (if (< (ref-get pos) len)
                  (do (mini-scan-one src len pos tokens)
                      (if (< (ref-get pos) len)
                        (do (mini-scan-one src len pos tokens)
                            (if (< (ref-get pos) len)
                              (do (mini-scan-one src len pos tokens)
                                  (if (< (ref-get pos) len)
                                    (do (mini-scan-one src len pos tokens)
                                        (if (< (ref-get pos) len)
                                          (do (mini-scan-one src len pos tokens)
                                              (if (< (ref-get pos) len)
                                                (do (mini-scan-one src len pos tokens)
                                                    (if (< (ref-get pos) len)
                                                      (mini-scan-one src len pos tokens)
                                                      0))
                                                0))
                                          0))
                                    0))
                              0))
                        0))
                  0))
            0))
      0)))

;; ミニトークナイザーのエントリポイント
(defn mini-tokenize [src]
  (let [len (string-length src)
        pos (ref-new 0)
        tokens (ref-new (vector-new 32))]
    (do
      (mini-scan-loop src len pos tokens)
      (ref-set tokens (vector-push (vector-push (ref-get tokens) 99) 0))
      (ref-get tokens))))

;; ============================================================
;; T4-4: ミニパーサー (トークン列 → Vector ベース AST)
;; ============================================================

;; トークン列のアクセサ (flat [kind, value, kind, value, ...])
(defn tok-at-kind [tokens idx]
  (vector-get tokens (* idx 2)))

(defn tok-at-value [tokens idx]
  (vector-get tokens (+ (* idx 2) 1)))

;; ミニパーサー: (defn NAME [] BODY) を Vector ベース AST に変換
;; MVP: body は整数リテラルのみ
;; 戻り値: defn AST ノード [20, name-hash, 0, body-node]
(defn mini-parse-defn [tokens]
  (let [;; tokens[0] = ( , tokens[1] = defn, tokens[2] = NAME
        ;; tokens[3] = [ , tokens[4] = ] , tokens[5] = BODY, tokens[6] = )
        name-kind (tok-at-kind tokens 2)
        body-kind (tok-at-kind tokens 5)
        body-value (tok-at-value tokens 5)]
    ;; body が整数リテラルの場合
    (if (= body-kind 10)
      (let [body-ast (make-lit-int body-value)
            defn-node (vector-new 4)]
        (vector-push (vector-push (vector-push (vector-push defn-node 20)
          (tok-at-value tokens 2)) 0) body-ast))
      ;; 未対応の body 型
      (make-lit-int 0))))

;; ============================================================
;; T4-4: compile-source (ソース文字列 → IR)
;; ============================================================

;; ソース文字列をトークナイズ → パース → IR 変換
(defn compile-source [src]
  (let [;; Step 1: トークナイズ
        tokens (mini-tokenize src)
        ;; Step 2: パース (defn の body を取得)
        defn-ast (mini-parse-defn tokens)
        ;; body は defn ノードの index 3
        body-ast (vector-get defn-ast 3)
        ;; Step 3: IR 変換
        env (env-new)
        ir-instrs (compile-expr body-ast env (vector-new 8))]
    ;; [tokens, defn-ast, ir-instrs] を返す
    (let [result (vector-new 3)]
      (vector-push (vector-push (vector-push result tokens) defn-ast) ir-instrs))))

;; ============================================================
;; 統合パイプライン: Source → Token → AST → IR → Wasm
;; ============================================================

(defn main []
  (let [;; === 旧パイプライン (手動 AST) ===
        ast-node (make-lit-int 42)
        env (env-new)
        ir-instrs (compile-expr ast-node env (vector-new 8))
        header (emit-header)
        type-sec (emit-type-section-main)
        wasm-size (emit-wasm-header-bytes)

        ;; === T4-4: 新パイプライン (ソースから) ===
        source "(defn main [] 42)"
        pipeline-result (compile-source source)
        src-tokens (vector-get pipeline-result 0)
        src-defn (vector-get pipeline-result 1)
        src-ir (vector-get pipeline-result 2)]
    (do
      ;; === 旧パイプライン検証 (既存テスト互換) ===
      (print (ast-tag ast-node))         ;; 1 (lit-int)
      (print (vector-get ast-node 1))    ;; 42
      (print (vector-length ir-instrs))  ;; 1
      (let [instr0 (vector-get ir-instrs 0)]
        (do
          (print (vector-get instr0 0))  ;; 1 (op: i64.const)
          (print (vector-get instr0 1))));; 42
      (print (vector-length header))     ;; 8
      (print (vector-get header 0))      ;; 0
      (print (vector-get header 1))      ;; 97
      (print (vector-get header 2))      ;; 115
      (print (vector-get header 3))      ;; 109
      (print (vector-length type-sec))   ;; 7
      (print (vector-get type-sec 0))    ;; 1
      (print wasm-size)                  ;; 15
      (print (module-count))             ;; 10

      ;; === T4-4: 新パイプライン検証 ===
      ;; トークン数 (kind,value ペア): 7+1(EOF) = 16 エントリ
      (print (vector-length src-tokens))  ;; 16

      ;; defn AST: tag=20
      (print (vector-get src-defn 0))     ;; 20 (defn)

      ;; body AST: tag=1, value=42
      (let [body (vector-get src-defn 3)]
        (do
          (print (vector-get body 0))     ;; 1 (lit-int)
          (print (vector-get body 1))))   ;; 42

      ;; IR 命令: i64.const 42
      (print (vector-length src-ir))      ;; 1
      (let [src-instr0 (vector-get src-ir 0)]
        (do
          (print (vector-get src-instr0 0))  ;; 1 (i64.const)
          (print (vector-get src-instr0 1))));; 42

      0)))
