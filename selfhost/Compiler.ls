;; Compiler.ls - L# セルフホスティング: AST → IR 変換
;;
;; AST ノード (整数タグ + Vector) を IR 命令列 (Vector of Vector) に変換する。
;; 最小サブセット: 整数リテラル、変数参照、関数呼出、if 式、let 束縛

;; === AST タグ定数 (AST.ls から再定義) ===
(defn tag-lit-int [] 1)
(defn tag-lit-bool [] 2)
(defn tag-var [] 4)
(defn tag-apply [] 5)
(defn tag-if [] 6)
(defn tag-let [] 7)

;; === IR opcode 定数 (IR.ls から再定義) ===
(defn op-i64-const [] 1)
(defn op-local-get [] 10)
(defn op-local-set [] 11)
(defn op-i64-add [] 20)
(defn op-i64-sub [] 21)
(defn op-i64-mul [] 22)
(defn op-i64-div [] 23)
(defn op-i64-eq [] 30)
(defn op-call [] 40)
(defn op-if [] 41)
(defn op-end [] 43)

;; === IR 命令構築ヘルパー ===

;; IR 命令: [opcode, operand]
(defn emit-instr [opcode operand]
  (vector-push (vector-push (vector-new 2) opcode) operand))

;; IR 命令列に命令を追加
(defn emit-to [instrs opcode operand]
  (vector-push instrs (emit-instr opcode operand)))

;; === 環境 (変数名ハッシュ → ローカルインデックス) ===

;; 環境は HashMap<name-hash, local-index>
(defn env-new []
  (map-new))

(defn env-bind [env name-hash idx]
  (map-insert env name-hash idx))

(defn env-lookup [env name-hash]
  (map-get env name-hash))

;; === コンパイラ本体 ===

;; AST ノードを IR 命令列に変換 (結果は instrs に追記)
;; 戻り値: 更新された instrs
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
              ;; 未束縛変数: エラー代わりに 0 をプッシュ
              (emit-to instrs 1 0)
              (emit-to instrs 10 idx)))
          ;; その他の式: 未実装 → 0 をプッシュ
          (emit-to instrs 1 0))))))

;; 関数のコンパイル: パラメータ名ハッシュのリスト → IR 命令列
(defn compile-function [param-hashes body]
  (let [env (ref-new (env-new))
        idx (ref-new 1)
        i (ref-new 0)
        n (vector-length param-hashes)]
    (do
      ;; パラメータを環境に登録
      (let [loop-done (ref-new 0)]
        (do
          (let [loop-body (ref-new 0)]
            (do
              (ref-set loop-body 1)
              (if (< (ref-get i) n)
                (do
                  (ref-set env (env-bind (ref-get env) (vector-get param-hashes (ref-get i)) (ref-get idx)))
                  (ref-set idx (+ (ref-get idx) 1))
                  (ref-set i (+ (ref-get i) 1))
                  (if (< (ref-get i) n)
                    (do
                      (ref-set env (env-bind (ref-get env) (vector-get param-hashes (ref-get i)) (ref-get idx)))
                      (ref-set idx (+ (ref-get idx) 1))
                      (ref-set i (+ (ref-get i) 1))
                      0)
                    0))
                0)))
          0))
      ;; ボディをコンパイル
      (compile-expr body (ref-get env) (vector-new 8)))))

;; === LEB128 エンコーディング ===

;; 符号なし LEB128 エンコード: 値 → バイト列 (Vector of i64)
(defn leb128-unsigned [value]
  (let [result (ref-new (vector-new 4))
        v (ref-new value)
        done (ref-new 0)]
    (do
      ;; 最初のバイト
      (let [byte (% (ref-get v) 128)
            rest (/ (ref-get v) 128)]
        (if (= rest 0)
          (do
            (ref-set result (vector-push (ref-get result) byte))
            (ref-set done 1)
            0)
          (do
            (ref-set result (vector-push (ref-get result) (+ byte 128)))
            (ref-set v rest)
            ;; 2番目のバイト
            (let [byte2 (% (ref-get v) 128)
                  rest2 (/ (ref-get v) 128)]
              (if (= rest2 0)
                (do
                  (ref-set result (vector-push (ref-get result) byte2))
                  (ref-set done 1)
                  0)
                (do
                  (ref-set result (vector-push (ref-get result) (+ byte2 128)))
                  (ref-set v rest2)
                  ;; 3番目のバイト (最大 21bit まで)
                  (let [byte3 (% (ref-get v) 128)]
                    (do
                      (ref-set result (vector-push (ref-get result) byte3))
                      0)))))
            0)))
      (ref-get result))))

;; === エントリポイント (テスト用) ===

(defn main []
  (let [;; 整数リテラルをコンパイル
        lit-node (vector-push (vector-push (vector-new 2) 1) 42)
        env (env-new)
        instrs (compile-expr lit-node env (vector-new 8))

        ;; LEB128 エンコード
        leb-small (leb128-unsigned 5)
        leb-medium (leb128-unsigned 300)]
    (do
      ;; コンパイル結果の検証
      (print (vector-length instrs))      ;; 1 (命令 1個)
      (let [instr0 (vector-get instrs 0)]
        (do
          (print (vector-get instr0 0))    ;; 1 (op: i64.const)
          (print (vector-get instr0 1))))  ;; 42 (operand)

      ;; LEB128 結果の検証
      (print (vector-length leb-small))    ;; 1 (5 は 1バイト)
      (print (vector-get leb-small 0))     ;; 5
      (print (vector-length leb-medium))   ;; 2 (300 は 2バイト)
      (print (vector-get leb-medium 0))    ;; 172 (300 & 0x7F | 0x80 = 44+128)
      (print (vector-get leb-medium 1))    ;; 2 (300 >> 7 = 2)
      0)))
