;; Main.ls - L# セルフホスティング: 統合パイプライン
;;
;; Token/AST/IR/Compiler/WasmEmit モジュールを結合し、
;; AST 構築 → IR 変換 → Wasm バイナリ生成の統合パイプラインを検証する。
;;
;; 注意: Lexer.ls / Parser.ls / TypeScheme.ls はコンパイラの既知の制限
;; (深いネスト if / 前方参照の相互再帰) によりコンパイル不可のため除外。
;; トークナイズ・パースは手動で行い、AST 以降のパイプラインを統合する。

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

;; 環境 (変数名ハッシュ → ローカルインデックス)
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
;; 統合パイプライン: AST → IR → Wasm
;; ============================================================

(defn main []
  (let [;; Step 1: AST 構築 (手動: Parser の代わり)
        ;; "(defn main [] 42)" の本体 = 整数リテラル 42
        ast-node (make-lit-int 42)

        ;; Step 2: IR 変換 (Compiler)
        env (env-new)
        ir-instrs (compile-expr ast-node env (vector-new 8))

        ;; Step 3: Wasm バイナリ生成 (WasmEmit)
        header (emit-header)
        type-sec (emit-type-section-main)]
    (do
      ;; === 検証出力 ===

      ;; AST ノード検証
      (print (ast-tag ast-node))         ;; 1 (lit-int)
      (print (vector-get ast-node 1))    ;; 42

      ;; IR 命令検証
      (print (vector-length ir-instrs))  ;; 1 (命令 1 個)
      (let [instr0 (vector-get ir-instrs 0)]
        (do
          (print (vector-get instr0 0))  ;; 1 (op: i64.const)
          (print (vector-get instr0 1))));; 42 (operand)

      ;; Wasm ヘッダー検証
      (print (vector-length header))     ;; 8
      (print (vector-get header 0))      ;; 0 (\0)
      (print (vector-get header 1))      ;; 97 (a)
      (print (vector-get header 2))      ;; 115 (s)
      (print (vector-get header 3))      ;; 109 (m)

      ;; Type セクション検証
      (print (vector-length type-sec))   ;; 7
      (print (vector-get type-sec 0))    ;; 1 (section-id: Type)

      0)))
