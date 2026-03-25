(module Compiler)
(import AST)
(import IR)

;; Compiler.ls - L# セルフホスティング: AST → IR 変換
;;
;; AST ノード (整数タグ + Vector) を IR 命令列 (Vector of Vector) に変換する。
;; サポート: 整数リテラル、変数参照、関数呼出、if 式、let 束縛、do ブロック、
;;           lambda 式、defn 宣言、match 式、ビルトイン関数

;; === AST タグ定数 (AST.ls から再定義) ===
(defn tag-lit-int [] 1)
(defn tag-lit-bool [] 2)
(defn tag-var [] 4)
(defn tag-apply [] 5)
(defn tag-if [] 6)
(defn tag-let [] 7)
(defn tag-lambda [] 8)
(defn tag-do [] 9)
(defn tag-match [] 10)
(defn tag-defn [] 20)

;; === IR opcode 定数 (IR.ls から再定義) ===
(defn op-i64-const [] 1)
(defn op-local-get [] 10)
(defn op-local-set [] 11)
(defn op-i64-add [] 20)
(defn op-i64-sub [] 21)
(defn op-i64-mul [] 22)
(defn op-i64-div [] 23)
(defn op-i64-eq [] 30)
(defn op-i64-gt [] 31)
(defn op-i64-lt [] 32)
(defn op-i64-ge [] 33)
(defn op-i64-le [] 34)
(defn op-call [] 40)
(defn op-if [] 41)
(defn op-end [] 43)

;; === T3-3: ビルトイン関数のハッシュ定数 ===
;; name-hash は文字列の先頭文字の ASCII コード (簡易ハッシュ)
;; selfhost コンパイラでは 1文字演算子はこの値で判定
(defn builtin-add [] 43)    ;; '+' の ASCII
(defn builtin-sub [] 45)    ;; '-' の ASCII
(defn builtin-mul [] 42)    ;; '*' の ASCII
(defn builtin-div [] 47)    ;; '/' の ASCII
(defn builtin-eq [] 61)     ;; '=' の ASCII
(defn builtin-gt [] 62)     ;; '>' の ASCII
(defn builtin-lt [] 60)     ;; '<' の ASCII
(defn builtin-mod [] 37)    ;; '%' の ASCII

;; ビルトイン演算子か判定し、対応する IR opcode を返す
;; 非ビルトインの場合は 0 を返す
(defn builtin-opcode [name-hash]
  (if (= name-hash 43) 20     ;; + -> i64.add
    (if (= name-hash 45) 21   ;; - -> i64.sub
      (if (= name-hash 42) 22 ;; * -> i64.mul
        (if (= name-hash 47) 23 ;; / -> i64.div
          (if (= name-hash 61) 30 ;; = -> i64.eq
            (if (= name-hash 62) 31 ;; > -> i64.gt
              (if (= name-hash 60) 32 ;; < -> i64.lt
                (if (= name-hash 37) 22 ;; % -> i64.mul (簡略化: rem 未実装)
                  0)))))))))

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

;; === 関数テーブル (defn 名ハッシュ → 関数インデックス) ===

;; 関数テーブルは HashMap<name-hash, func-index>
(defn ftable-new []
  (map-new))

(defn ftable-register [ftable name-hash func-idx]
  (map-insert ftable name-hash func-idx))

(defn ftable-lookup [ftable name-hash]
  (map-get ftable name-hash))

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
          (if (= tag 5)
            ;; 関数適用 (tag=5): [5, func-node, arg-count, arg1, arg2, ...]
            ;; T3-3: ビルトイン演算子の場合はインライン命令を生成
            (let [func-node (vector-get node 1)
                  arg-count (vector-get node 2)
                  bop (builtin-opcode func-node)]
              (if (> bop 0)
                ;; ビルトイン二項演算子: 引数をコンパイルしてインライン命令
                (let [instrs1 (compile-expr (vector-get node 3) env instrs)
                      instrs2 (compile-expr (vector-get node 4) env instrs1)]
                  (emit-to instrs2 bop 0))
                ;; 通常の関数呼出し
                (let [instrs1 (ref-new instrs)]
                  (do
                    ;; 引数を順にコンパイル
                    (if (> arg-count 0)
                      (do
                        (ref-set instrs1 (compile-expr (vector-get node 3) env (ref-get instrs1)))
                        (if (> arg-count 1)
                          (do
                            (ref-set instrs1 (compile-expr (vector-get node 4) env (ref-get instrs1)))
                            0)
                          0))
                      0)
                    ;; 関数を呼出し
                    (emit-to (ref-get instrs1) 40 func-node)))))
            (if (= tag 6)
              ;; if 式 (tag=6): [6, cond-expr, then-expr, else-expr]
              (let [cond-expr (vector-get node 1)
                    then-expr (vector-get node 2)
                    else-expr (vector-get node 3)
                    ;; 条件式をコンパイル
                    instrs1 (compile-expr cond-expr env instrs)
                    ;; if 命令
                    instrs2 (emit-to instrs1 41 0)
                    ;; then ブランチ
                    instrs3 (compile-expr then-expr env instrs2)
                    ;; else マーカー (op-end の代用)
                    instrs4 (emit-to instrs3 43 0)
                    ;; else ブランチ
                    instrs5 (compile-expr else-expr env instrs4)]
                ;; end 命令
                (emit-to instrs5 43 0))
              (if (= tag 7)
                ;; let 束縛 (tag=7): [7, name-hash, init-expr, body-expr]
                (let [name-hash (vector-get node 1)
                      init-expr (vector-get node 2)
                      body-expr (vector-get node 3)
                      ;; init 式をコンパイル
                      instrs1 (compile-expr init-expr env instrs)
                      ;; 新しいローカル変数のインデックスを割当
                      new-idx (+ 1 (map-size env))
                      ;; local.set で変数に格納
                      instrs2 (emit-to instrs1 11 new-idx)
                      ;; 環境を拡張
                      new-env (env-bind env name-hash new-idx)]
                  ;; body をコンパイル
                  (compile-expr body-expr new-env instrs2))
                (if (= tag 8)
                  ;; lambda 式 (tag=8): [8, param-count, param1-hash, ..., body-expr]
                  ;; 直接呼出しのみ対応 (lambda lifting は後回し)
                  ;; パラメータを環境に登録して body をコンパイル
                  (let [param-count (vector-get node 1)
                        new-env (ref-new env)
                        new-idx (ref-new (+ 1 (map-size env)))]
                    (do
                      (if (> param-count 0)
                        (do
                          (ref-set new-env (env-bind (ref-get new-env) (vector-get node 2) (ref-get new-idx)))
                          (ref-set new-idx (+ (ref-get new-idx) 1))
                          (if (> param-count 1)
                            (do
                              (ref-set new-env (env-bind (ref-get new-env) (vector-get node 3) (ref-get new-idx)))
                              (ref-set new-idx (+ (ref-get new-idx) 1))
                              0)
                            0))
                        0)
                      ;; body は params の後の要素
                      (compile-expr (vector-get node (+ 2 param-count)) (ref-get new-env) instrs)))
                  (if (= tag 9)
                    ;; do ブロック (tag=9): [9, expr-count, expr1, expr2, ...]
                    ;; 全式をコンパイル、最後の値がブロックの値
                    (let [expr-count (vector-get node 1)
                          cur-instrs (ref-new instrs)]
                      (do
                        (if (> expr-count 0)
                          (do
                            (ref-set cur-instrs (compile-expr (vector-get node 2) env (ref-get cur-instrs)))
                            (if (> expr-count 1)
                              (do
                                (ref-set cur-instrs (compile-expr (vector-get node 3) env (ref-get cur-instrs)))
                                (if (> expr-count 2)
                                  (do
                                    (ref-set cur-instrs (compile-expr (vector-get node 4) env (ref-get cur-instrs)))
                                    (if (> expr-count 3)
                                      (do
                                        (ref-set cur-instrs (compile-expr (vector-get node 5) env (ref-get cur-instrs)))
                                        (if (> expr-count 4)
                                          (do
                                            (ref-set cur-instrs (compile-expr (vector-get node 6) env (ref-get cur-instrs)))
                                            0)
                                          0))
                                      0))
                                  0))
                              0))
                          0)
                        (ref-get cur-instrs)))
                    (if (= tag 10)
                      ;; T2-3/T3-5: match 式 (tag=10): [10, scrutinee, arm-count, pat1, body1, pat2, body2, ...]
                      ;; scrutinee をコンパイルし、各パターンをif-elseチェーンに変換
                      (let [scrutinee (vector-get node 1)
                            arm-count (vector-get node 2)
                            ;; scrutinee をコンパイルしてローカルに保存
                            scr-idx (+ 1 (map-size env))
                            instrs1 (compile-expr scrutinee env instrs)
                            instrs2 (emit-to instrs1 11 scr-idx)]
                        ;; 各腕を if-else チェーンに変換 (最大4腕)
                        (if (> arm-count 0)
                          (let [;; 腕1: if (scr == pat1) then body1
                                pat1 (vector-get node 3)
                                body1 (vector-get node 4)
                                ;; scrutinee をロード
                                i3 (emit-to instrs2 10 scr-idx)
                                ;; パターン値をプッシュ
                                i4 (emit-to i3 1 pat1)
                                ;; 比較
                                i5 (emit-to i4 30 0)
                                ;; if
                                i6 (emit-to i5 41 0)
                                ;; then: body1
                                i7 (compile-expr body1 env i6)
                                ;; else
                                i8 (emit-to i7 43 0)]
                            (if (> arm-count 1)
                              (let [pat2 (vector-get node 5)
                                    body2 (vector-get node 6)
                                    i9 (emit-to i8 10 scr-idx)
                                    i10 (emit-to i9 1 pat2)
                                    i11 (emit-to i10 30 0)
                                    i12 (emit-to i11 41 0)
                                    i13 (compile-expr body2 env i12)
                                    i14 (emit-to i13 43 0)]
                                (if (> arm-count 2)
                                  (let [pat3 (vector-get node 7)
                                        body3 (vector-get node 8)
                                        i15 (emit-to i14 10 scr-idx)
                                        i16 (emit-to i15 1 pat3)
                                        i17 (emit-to i16 30 0)
                                        i18 (emit-to i17 41 0)
                                        i19 (compile-expr body3 env i18)
                                        i20 (emit-to i19 43 0)
                                        ;; デフォルト: 0
                                        i21 (emit-to i20 1 0)
                                        i22 (emit-to i21 43 0)]
                                    (emit-to i22 43 0))
                                  ;; 2腕のみ: デフォルト 0
                                  (let [i15 (emit-to i14 1 0)
                                        i16 (emit-to i15 43 0)]
                                    (emit-to i16 43 0))))
                              ;; 1腕のみ: デフォルト 0
                              (let [i9 (emit-to i8 1 0)
                                    i10 (emit-to i9 43 0)]
                                i10)))
                          ;; 0腕: 0 を返す
                          (emit-to instrs2 1 0)))
                      ;; その他の式: 未実装 → 0 をプッシュ
                      (emit-to instrs 1 0))))))))))))

;; === defn 宣言のコンパイル ===

;; defn 宣言 (tag=20): [20, name-hash, param-count, param1-hash, ..., body-expr]
;; パラメータを環境に登録して body をコンパイルし、IR 命令列を返す
(defn compile-defn [node]
  (let [name-hash (vector-get node 1)
        param-count (vector-get node 2)
        env (ref-new (env-new))
        idx (ref-new 1)
        i (ref-new 0)]
    (do
      ;; パラメータを環境に登録 (最大4パラメータ)
      (if (> param-count 0)
        (do
          (ref-set env (env-bind (ref-get env) (vector-get node 3) (ref-get idx)))
          (ref-set idx (+ (ref-get idx) 1))
          (if (> param-count 1)
            (do
              (ref-set env (env-bind (ref-get env) (vector-get node 4) (ref-get idx)))
              (ref-set idx (+ (ref-get idx) 1))
              (if (> param-count 2)
                (do
                  (ref-set env (env-bind (ref-get env) (vector-get node 5) (ref-get idx)))
                  (ref-set idx (+ (ref-get idx) 1))
                  (if (> param-count 3)
                    (do
                      (ref-set env (env-bind (ref-get env) (vector-get node 6) (ref-get idx)))
                      (ref-set idx (+ (ref-get idx) 1))
                      0)
                    0))
                0))
            0))
        0)
      ;; body をコンパイル (body は params の後の要素)
      (let [body-idx (+ 3 param-count)
            body-expr (vector-get node body-idx)]
        (compile-expr body-expr (ref-get env) (vector-new 8))))))

;; === プログラム全体のコンパイル ===

;; 複数の defn 宣言をコンパイルし、関数テーブルと IR 命令列のリストを返す
;; 結果: [ftable, ir-list] の Vector
;; ir-list: 各関数の IR 命令列の Vector
(defn compile-program [decls]
  (let [ftable (ref-new (ftable-new))
        ir-list (ref-new (vector-new 8))
        func-idx (ref-new 0)
        i (ref-new 0)
        n (vector-length decls)]
    (do
      ;; Pass 1: 全関数名を登録
      (if (< (ref-get i) n)
        (do
          (let [decl (vector-get decls (ref-get i))]
            (if (= (vector-get decl 0) 20)
              (do
                (ref-set ftable (ftable-register (ref-get ftable) (vector-get decl 1) (ref-get func-idx)))
                (ref-set func-idx (+ (ref-get func-idx) 1))
                0)
              0))
          (ref-set i (+ (ref-get i) 1))
          (if (< (ref-get i) n)
            (do
              (let [decl (vector-get decls (ref-get i))]
                (if (= (vector-get decl 0) 20)
                  (do
                    (ref-set ftable (ftable-register (ref-get ftable) (vector-get decl 1) (ref-get func-idx)))
                    (ref-set func-idx (+ (ref-get func-idx) 1))
                    0)
                  0))
              (ref-set i (+ (ref-get i) 1))
              (if (< (ref-get i) n)
                (do
                  (let [decl (vector-get decls (ref-get i))]
                    (if (= (vector-get decl 0) 20)
                      (do
                        (ref-set ftable (ftable-register (ref-get ftable) (vector-get decl 1) (ref-get func-idx)))
                        (ref-set func-idx (+ (ref-get func-idx) 1))
                        0)
                      0))
                  (ref-set i (+ (ref-get i) 1))
                  (if (< (ref-get i) n)
                    (do
                      (let [decl (vector-get decls (ref-get i))]
                        (if (= (vector-get decl 0) 20)
                          (do
                            (ref-set ftable (ftable-register (ref-get ftable) (vector-get decl 1) (ref-get func-idx)))
                            (ref-set func-idx (+ (ref-get func-idx) 1))
                            0)
                          0))
                      0)
                    0))
                0))
            0))
        0)
      ;; Pass 2: 各関数をコンパイル
      (ref-set i 0)
      (if (< (ref-get i) n)
        (do
          (let [decl (vector-get decls (ref-get i))]
            (if (= (vector-get decl 0) 20)
              (do (ref-set ir-list (vector-push (ref-get ir-list) (compile-defn decl))) 0)
              0))
          (ref-set i (+ (ref-get i) 1))
          (if (< (ref-get i) n)
            (do
              (let [decl (vector-get decls (ref-get i))]
                (if (= (vector-get decl 0) 20)
                  (do (ref-set ir-list (vector-push (ref-get ir-list) (compile-defn decl))) 0)
                  0))
              (ref-set i (+ (ref-get i) 1))
              (if (< (ref-get i) n)
                (do
                  (let [decl (vector-get decls (ref-get i))]
                    (if (= (vector-get decl 0) 20)
                      (do (ref-set ir-list (vector-push (ref-get ir-list) (compile-defn decl))) 0)
                      0))
                  (ref-set i (+ (ref-get i) 1))
                  (if (< (ref-get i) n)
                    (do
                      (let [decl (vector-get decls (ref-get i))]
                        (if (= (vector-get decl 0) 20)
                          (do (ref-set ir-list (vector-push (ref-get ir-list) (compile-defn decl))) 0)
                          0))
                      0)
                    0))
                0))
            0))
        0)
      ;; 結果を [ftable, ir-list] として返す
      (let [result (vector-new 2)]
        (vector-push (vector-push result (ref-get ftable)) (ref-get ir-list))))))

;; Main.ls 用: リーフ式 [tag, ...] (長さ2の lit 等) は compile-expr、宣言列は compile-program
(defn lower [x]
  (let [n (vector-length x)]
    (if (= n 0)
      (vector-new 0)
      (if (and (= n 2) (or (= (vector-get x 0) 1) (= (vector-get x 0) 2)))
        (compile-expr x (env-new) (vector-new 8))
        (let [pair (compile-program x)
              ir-list (vector-get pair 1)]
          (if (> (vector-length ir-list) 0)
            (vector-get ir-list 0)
            (vector-new 0)))))))

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

        ;; do ブロックのテスト: [9, 2, [1, 10], [1, 20]]
        do-node (let [n (vector-new 8)]
                  (let [n1 (vector-push n 9)
                        n2 (vector-push n1 2)
                        e1 (vector-push (vector-push (vector-new 2) 1) 10)
                        n3 (vector-push n2 e1)
                        e2 (vector-push (vector-push (vector-new 2) 1) 20)
                        n4 (vector-push n3 e2)]
                    n4))
        do-instrs (compile-expr do-node env (vector-new 8))

        ;; LEB128 エンコード
        leb-small (leb128-unsigned 5)
        leb-medium (leb128-unsigned 300)

        ;; T3-3: ビルトイン演算子テスト
        ;; (+ 3 4) -> [5, 43, 2, [1, 3], [1, 4]]
        add-node (let [n (vector-new 8)]
                   (let [n1 (vector-push n 5)
                         n2 (vector-push n1 43)  ;; '+' hash
                         n3 (vector-push n2 2)   ;; arg-count
                         a1 (vector-push (vector-push (vector-new 2) 1) 3)
                         n4 (vector-push n3 a1)
                         a2 (vector-push (vector-push (vector-new 2) 1) 4)
                         n5 (vector-push n4 a2)]
                     n5))
        add-instrs (compile-expr add-node env (vector-new 8))]
    (do
      ;; コンパイル結果の検証
      (print (vector-length instrs))      ;; 1 (命令 1個)
      (let [instr0 (vector-get instrs 0)]
        (do
          (print (vector-get instr0 0))    ;; 1 (op: i64.const)
          (print (vector-get instr0 1))))  ;; 42 (operand)

      ;; do ブロック結果の検証 (2命令: i64.const 10, i64.const 20)
      (print (vector-length do-instrs))    ;; 2

      ;; LEB128 結果の検証
      (print (vector-length leb-small))    ;; 1 (5 は 1バイト)
      (print (vector-get leb-small 0))     ;; 5
      (print (vector-length leb-medium))   ;; 2 (300 は 2バイト)
      (print (vector-get leb-medium 0))    ;; 172 (300 & 0x7F | 0x80 = 44+128)
      (print (vector-get leb-medium 1))    ;; 2 (300 >> 7 = 2)

      ;; T3-3: ビルトイン加算テスト
      ;; (+ 3 4) → [i64.const 3, i64.const 4, i64.add]
      (print (vector-length add-instrs))   ;; 3 (3命令)
      (let [ai0 (vector-get add-instrs 0)
            ai1 (vector-get add-instrs 1)
            ai2 (vector-get add-instrs 2)]
        (do
          (print (vector-get ai0 0))       ;; 1 (i64.const)
          (print (vector-get ai0 1))       ;; 3
          (print (vector-get ai1 0))       ;; 1 (i64.const)
          (print (vector-get ai1 1))       ;; 4
          (print (vector-get ai2 0))       ;; 20 (i64.add)
          0))
      0)))
