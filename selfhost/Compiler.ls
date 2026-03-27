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
(defn tag-lit-string [] 3)
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
(defn op-drop [] 44)
(defn op-string-char-at [] 50)
(defn op-string-length [] 51)
(defn op-vector-length [] 52)
(defn op-vector-get [] 53)
(defn op-vector-new [] 54)
(defn op-vector-push [] 55)
(defn op-ref-new [] 56)
(defn op-ref-get [] 57)
(defn op-ref-set [] 58)
(defn op-print [] 59)
(defn op-map-new [] 60)
(defn op-map-size [] 61)
(defn op-map-insert [] 62)
(defn op-map-get [] 63)
(defn op-read-file [] 64)
(defn op-map-contains [] 65)
(defn op-map-remove [] 66)
(defn op-command-line-arg [] 67)
(defn op-runtime-hash-string [] 68)

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
(defn builtin-string-char-at [] 6233512424790686798)
(defn builtin-string-length [] 1391193567100747810)
(defn builtin-vector-length [] 3361052332089172656)
(defn builtin-vector-get [] 3208847393524684)
(defn builtin-vector-new [] 3208847393531414)
(defn builtin-vector-push [] 99474269199548772)
(defn builtin-ref-new [] 104162612582)
(defn builtin-ref-get [] 104162605852)
(defn builtin-ref-set [] 104162617384)
(defn builtin-print [] 106934957)
(defn builtin-map-new [] 99619812783)
(defn builtin-map-size [] 3088214349266)
(defn builtin-map-get [] 99619806053)
(defn builtin-map-insert [] 2967773707765834)
(defn builtin-read-file [] 100097347767123)
(defn builtin-map-contains [] -3820778934353407281)
(defn builtin-map-remove [] 2967773956947477)
(defn builtin-command-line-arg [] 4333701572691766591)

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
                  (if (= name-hash 6233512424790686798) 50 ;; string-char-at
                    (if (= name-hash 1391193567100747810) 51 ;; string-length
                      (if (= name-hash 3361052332089172656) 52 ;; vector-length
                        (if (= name-hash 3208847393524684) 53 ;; vector-get
                          (if (= name-hash 3208847393531414) 54 ;; vector-new
                            (if (= name-hash 99474269199548772) 55 ;; vector-push
                               (if (= name-hash 104162612582) 56 ;; ref-new
                                 (if (= name-hash 104162605852) 57 ;; ref-get
                                   (if (= name-hash 104162617384) 58 ;; ref-set
                                      (if (= name-hash 106934957) 59 ;; print
                                        (if (= name-hash 99619812783) 60 ;; map-new
                                          (if (= name-hash 3088214349266) 61 ;; map-size
                                              (if (= name-hash 99619806053) 63 ;; map-get
                                                (if (= name-hash 2967773707765834) 62 ;; map-insert
                                                   (if (= name-hash 100097347767123) 64 ;; read-file
                                                     (if (= name-hash -3820778934353407281) 65 ;; map-contains?
                                                       (if (= name-hash 2967773956947477) 66 ;; map-remove
                                                         (if (= name-hash 4333701572691766591) 67 ;; command-line-arg
                                                           0)))))))))))))))))))))))))))

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

;; do ブロックの式列を先頭から順にコンパイルする
;; 中間式の値は最終式を残すために drop する
(defn compile-do-exprs [node env ftable idx expr-count instrs]
  (if (>= idx expr-count)
    instrs
    (let [value-instrs (compile-expr-with-ftable (vector-get node (+ 2 idx)) env ftable instrs)
          next-instrs (if (< (+ idx 1) expr-count)
                        (emit-to value-instrs (op-drop) 0)
                        value-instrs)]
      (compile-do-exprs node env ftable (+ idx 1) expr-count next-instrs))))

(defn compile-do-exprs-with-source [node source env ftable idx expr-count instrs data-ref]
  (if (>= idx expr-count)
    instrs
    (let [value-instrs (compile-expr-with-source (vector-get node (+ 2 idx)) source env ftable instrs data-ref)
          next-instrs (if (< (+ idx 1) expr-count)
                        (emit-to value-instrs (op-drop) 0)
                        value-instrs)]
      (compile-do-exprs-with-source node source env ftable (+ idx 1) expr-count next-instrs data-ref))))

;; source 付き string literal lowering 用ヘルパー
(defn string-literal-data-base [] 1024)

(defn append-byte-vector [dst src idx count]
  (if (>= idx count)
    dst
    (append-byte-vector
      (vector-push dst (vector-get src idx))
      src
      (+ idx 1)
      count)))

(defn string-to-byte-vector [text idx count bytes]
  (if (>= idx count)
    bytes
    (string-to-byte-vector
      text
      (+ idx 1)
      count
      (vector-push bytes (string-char-at text idx)))))

(defn compile-string-literal-with-source [node source instrs data-ref]
  (let [start (vector-get node 1)
        end (vector-get node 2)
        text (substring source start end)
        text-len (string-length text)
        bytes (string-to-byte-vector text 0 text-len (vector-new 8))
        offset (+ (string-literal-data-base) (vector-length (ref-get data-ref)))
        updated-data (append-byte-vector (ref-get data-ref) bytes 0 (vector-length bytes))
        instrs1 (emit-to instrs 1 offset)]
    (do
      (ref-set data-ref updated-data)
      instrs1)))

(defn string-key-hash-loop [source pos end acc]
  (if (>= pos end)
    acc
    (string-key-hash-loop
      source
      (+ pos 1)
      end
      (+ (string-char-at source pos) (* acc 31)))))

(defn normalize-map-key-hash [hash]
  (if (= hash 0)
    2
    (if (= hash -1)
      1
      hash)))

(defn compile-string-key-hash-with-source [node source instrs]
  (let [start (vector-get node 1)
        end (vector-get node 2)
        hash (normalize-map-key-hash (string-key-hash-loop source start end 0))]
    (emit-to instrs (op-i64-const) hash)))

(defn compile-map-builtin-with-source [node source env ftable instrs data-ref bop]
  (let [map-expr (vector-get node 3)
        key-expr (vector-get node 4)
        map-instrs (compile-expr-with-source map-expr source env ftable instrs data-ref)
        key-instrs (if (= (vector-get key-expr 0) (tag-lit-string))
                     (compile-string-key-hash-with-source key-expr source map-instrs)
                     (let [value-instrs (compile-expr-with-source key-expr source env ftable map-instrs data-ref)]
                       (emit-to value-instrs (op-runtime-hash-string) 0)))]
    (if (= bop (op-map-insert))
      (let [value-expr (vector-get node 5)
            value-instrs (compile-expr-with-source value-expr source env ftable key-instrs data-ref)]
        (emit-to value-instrs bop (+ 1 (map-size env))))
      (emit-to key-instrs bop (+ 1 (map-size env))))))

(defn compile-match-pattern-check [pat scr-idx instrs]
  (let [pat-tag (vector-get pat 0)]
    (if (= pat-tag (ast-pat-lit))
      (let [lit (vector-get pat 1)
            lit-tag (vector-get lit 0)]
        (if (= lit-tag (ast-lit-int))
          (let [i1 (emit-to instrs (op-local-get) scr-idx)
                i2 (emit-to i1 (op-i64-const) (vector-get lit 1))]
            (emit-to i2 (op-i64-eq) 0))
          (if (= lit-tag (ast-lit-bool))
            (let [i1 (emit-to instrs (op-local-get) scr-idx)
                  i2 (emit-to i1 (op-i64-const) (vector-get lit 1))]
              (emit-to i2 (op-i64-eq) 0))
            (if (= lit-tag (ast-lit-unit))
              (let [i1 (emit-to instrs (op-local-get) scr-idx)
                    i2 (emit-to i1 (op-i64-const) 0)]
                (emit-to i2 (op-i64-eq) 0))
              (emit-to instrs (op-i64-const) 0)))))
      (if (or (= pat-tag (ast-pat-wildcard)) (= pat-tag (ast-pat-var)))
        (emit-to instrs (op-i64-const) 1)
        (emit-to instrs (op-i64-const) 0)))))

(defn compile-apply-with-source [node source env ftable instrs data-ref]
  (let [func-node (vector-get node 1)
        func-tag (vector-get func-node 0)
        func-hash (if (= func-tag (tag-var)) (vector-get func-node 1) 0)
        arg-count (vector-get node 2)
        bop (builtin-opcode func-hash)]
    (if (> bop 0)
      (if (= bop (op-map-new))
        (emit-to instrs bop (+ 1 (map-size env)))
        (if (or (= bop (op-map-insert))
                (or (= bop (op-map-get))
                    (or (= bop (op-map-contains)) (= bop (op-map-remove)))))
          (compile-map-builtin-with-source node source env ftable instrs data-ref bop)
          (let [instrs1 (compile-expr-with-source (vector-get node 3) source env ftable instrs data-ref)]
            (if (or (or (or (or (= bop (op-string-length)) (= bop (op-vector-length))) (= bop (op-ref-get)))
                        (or (or (= bop (op-map-size)) (= bop (op-print)))
                            (or (= bop (op-read-file)) (= bop (op-command-line-arg)))))
                    (or (= bop (op-vector-new)) (= bop (op-ref-new))))
              (if (or (= bop (op-vector-new)) (= bop (op-ref-new)))
                (emit-to instrs1 bop (+ 1 (map-size env)))
                (emit-to instrs1 bop 0))
              (let [instrs2 (compile-expr-with-source (vector-get node 4) source env ftable instrs1 data-ref)]
                (if (or (or (or (or (= bop (op-string-char-at)) (= bop (op-vector-get))) (= bop (op-vector-push)))
                            (= bop (op-ref-set)))
                        (or (= bop (op-map-get))
                            (or (= bop (op-map-contains)) (= bop (op-map-remove)))))
                  (emit-to instrs2 bop (+ 1 (map-size env)))
                  (if (= bop (op-map-insert))
                    (let [instrs3 (compile-expr-with-source (vector-get node 5) source env ftable instrs2 data-ref)]
                      (emit-to instrs3 bop (+ 1 (map-size env))))
                    (emit-to instrs2 bop 0))))))))
      (let [func-idx (ftable-lookup ftable func-hash)
            instrs1 (ref-new instrs)]
        (do
          (if (> arg-count 0)
            (do
              (ref-set instrs1 (compile-expr-with-source (vector-get node 3) source env ftable (ref-get instrs1) data-ref))
              (if (> arg-count 1)
                (do
                  (ref-set instrs1 (compile-expr-with-source (vector-get node 4) source env ftable (ref-get instrs1) data-ref))
                  0)
                0))
            0)
          (emit-to (ref-get instrs1) (op-call) func-idx))))))

(defn compile-expr-with-source [node source env ftable instrs data-ref]
  (let [tag (vector-get node 0)]
    (if (= tag (tag-lit-string))
      (compile-string-literal-with-source node source instrs data-ref)
        (if (= tag (tag-do))
          (let [expr-count (vector-get node 1)]
          (if (= expr-count 0)
            instrs
            (compile-do-exprs-with-source node source env ftable 0 expr-count instrs data-ref)))
        (if (= tag (tag-if))
          (let [cond-expr (vector-get node 1)
                then-expr (vector-get node 2)
                else-expr (vector-get node 3)
                instrs1 (compile-expr-with-source cond-expr source env ftable instrs data-ref)
                instrs2 (emit-to instrs1 (op-if) 0)
                instrs3 (compile-expr-with-source then-expr source env ftable instrs2 data-ref)
                instrs4 (emit-to instrs3 (op-end) 0)
                instrs5 (compile-expr-with-source else-expr source env ftable instrs4 data-ref)]
            (emit-to instrs5 (op-end) 0))
        (if (= tag (tag-apply))
          (compile-apply-with-source node source env ftable instrs data-ref)
          (if (= tag (tag-let))
            (let [name-hash (vector-get node 1)
                  init-expr (vector-get node 2)
                  body-expr (vector-get node 3)
                  instrs1 (compile-expr-with-source init-expr source env ftable instrs data-ref)
                  new-idx (+ 1 (map-size env))
                  instrs2 (emit-to instrs1 (op-local-set) new-idx)
                  new-env (env-bind env name-hash new-idx)]
              (compile-expr-with-source body-expr source new-env ftable instrs2 data-ref))
            (if (= tag (tag-lambda))
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
                  (compile-expr-with-source (vector-get node (+ 2 param-count)) source (ref-get new-env) ftable instrs data-ref)))
            (if (= tag (tag-match))
              (let [scrutinee (vector-get node 1)
                    arm-count (vector-get node 2)
                    scr-idx (+ 1 (map-size env))
                    instrs1 (compile-expr-with-source scrutinee source env ftable instrs data-ref)
                    instrs2 (emit-to instrs1 (op-local-set) scr-idx)]
                (if (> arm-count 0)
                  (let [pat1 (vector-get node 3)
                        body1 (vector-get node 4)
                        i5 (compile-match-pattern-check pat1 scr-idx instrs2)
                        i6 (emit-to i5 (op-if) 0)
                        i7 (compile-expr-with-source body1 source env ftable i6 data-ref)
                        i8 (emit-to i7 (op-end) 0)]
                    (if (> arm-count 1)
                      (let [pat2 (vector-get node 5)
                            body2 (vector-get node 6)
                            i11 (compile-match-pattern-check pat2 scr-idx i8)
                            i12 (emit-to i11 (op-if) 0)
                            i13 (compile-expr-with-source body2 source env ftable i12 data-ref)
                            i14 (emit-to i13 (op-end) 0)]
                        (if (> arm-count 2)
                          (let [pat3 (vector-get node 7)
                                body3 (vector-get node 8)
                                i17 (compile-match-pattern-check pat3 scr-idx i14)
                                i18 (emit-to i17 (op-if) 0)
                                i19 (compile-expr-with-source body3 source env ftable i18 data-ref)
                                i20 (emit-to i19 (op-end) 0)
                                i21 (emit-to i20 (op-i64-const) 0)
                                i22 (emit-to i21 (op-end) 0)
                                i23 (emit-to i22 (op-end) 0)
                                i24 (emit-to i23 (op-end) 0)]
                            i24)
                          (let [i15 (emit-to i14 (op-i64-const) 0)
                                i16 (emit-to i15 (op-end) 0)
                                i17 (emit-to i16 (op-end) 0)]
                            i17)))
                      (let [i9 (emit-to i8 (op-i64-const) 0)
                            i10 (emit-to i9 (op-end) 0)]
                        i10)))
                  (emit-to instrs2 (op-i64-const) 0)))
              (compile-expr-with-ftable node env ftable instrs))))))))))

(defn compile-defn-with-source [node source ftable data-ref]
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
        (compile-expr-with-source body-expr source (ref-get env) ftable (vector-new 8) data-ref)))))

;; AST ノードを IR 命令列に変換 (結果は instrs に追記)
;; 戻り値: 更新された instrs
(defn compile-expr-with-ftable [node env ftable instrs]
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
                  func-tag (vector-get func-node 0)
                  func-hash (if (= func-tag 4) (vector-get func-node 1) 0)
                  arg-count (vector-get node 2)
                  bop (builtin-opcode func-hash)]
              (if (> bop 0)
                ;; ビルトイン演算子 / 文字列 helper
                (if (= bop (op-map-new))
                  (emit-to instrs bop (+ 1 (map-size env)))
                  (let [instrs1 (compile-expr-with-ftable (vector-get node 3) env ftable instrs)]
                    (if (or (or (or (or (= bop (op-string-length)) (= bop (op-vector-length))) (= bop (op-ref-get)))
                                (or (or (= bop (op-map-size)) (= bop (op-print)))
                                    (or (= bop (op-read-file)) (= bop (op-command-line-arg)))))
                            (or (= bop (op-vector-new)) (= bop (op-ref-new))))
                      (if (or (= bop (op-vector-new)) (= bop (op-ref-new)))
                        (emit-to instrs1 bop (+ 1 (map-size env)))
                        (emit-to instrs1 bop 0))
                      (let [instrs2 (compile-expr-with-ftable (vector-get node 4) env ftable instrs1)]
                        (if (or (or (or (or (= bop (op-string-char-at)) (= bop (op-vector-get))) (= bop (op-vector-push)))
                                    (= bop (op-ref-set)))
                                (or (= bop (op-map-get))
                                    (or (= bop (op-map-contains)) (= bop (op-map-remove)))))
                          (emit-to instrs2 bop (+ 1 (map-size env)))
                          (if (= bop (op-map-insert))
                            (let [instrs3 (compile-expr-with-ftable (vector-get node 5) env ftable instrs2)]
                              (emit-to instrs3 bop (+ 1 (map-size env))))
                            (emit-to instrs2 bop 0)))))))
                ;; 通常の関数呼出し
                (let [func-idx (ftable-lookup ftable func-hash)
                      instrs1 (ref-new instrs)]
                  (do
                    ;; 引数を順にコンパイル
                    (if (> arg-count 0)
                      (do
                        (ref-set instrs1 (compile-expr-with-ftable (vector-get node 3) env ftable (ref-get instrs1)))
                        (if (> arg-count 1)
                          (do
                            (ref-set instrs1 (compile-expr-with-ftable (vector-get node 4) env ftable (ref-get instrs1)))
                            0)
                          0))
                      0)
                    ;; 関数を呼出し
                    (emit-to (ref-get instrs1) 40 func-idx)))))
            (if (= tag 6)
              ;; if 式 (tag=6): [6, cond-expr, then-expr, else-expr]
              (let [cond-expr (vector-get node 1)
                    then-expr (vector-get node 2)
                    else-expr (vector-get node 3)
                    ;; 条件式をコンパイル
                    instrs1 (compile-expr-with-ftable cond-expr env ftable instrs)
                    ;; if 命令
                    instrs2 (emit-to instrs1 41 0)
                    ;; then ブランチ
                    instrs3 (compile-expr-with-ftable then-expr env ftable instrs2)
                    ;; else マーカー (op-end の代用)
                    instrs4 (emit-to instrs3 43 0)
                    ;; else ブランチ
                    instrs5 (compile-expr-with-ftable else-expr env ftable instrs4)]
                ;; end 命令
                (emit-to instrs5 43 0))
              (if (= tag 7)
                ;; let 束縛 (tag=7): [7, name-hash, init-expr, body-expr]
                (let [name-hash (vector-get node 1)
                      init-expr (vector-get node 2)
                      body-expr (vector-get node 3)
                      ;; init 式をコンパイル
                      instrs1 (compile-expr-with-ftable init-expr env ftable instrs)
                      ;; 新しいローカル変数のインデックスを割当
                      new-idx (+ 1 (map-size env))
                      ;; local.set で変数に格納
                      instrs2 (emit-to instrs1 11 new-idx)
                      ;; 環境を拡張
                      new-env (env-bind env name-hash new-idx)]
                  ;; body をコンパイル
                  (compile-expr-with-ftable body-expr new-env ftable instrs2))
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
                      (compile-expr-with-ftable (vector-get node (+ 2 param-count)) (ref-get new-env) ftable instrs)))
                  (if (= tag 9)
                    ;; do ブロック (tag=9): [9, expr-count, expr1, expr2, ...]
                    ;; 全式をコンパイル、最後の値がブロックの値
                    (let [expr-count (vector-get node 1)]
                      (compile-do-exprs node env ftable 0 expr-count instrs))
                    (if (= tag 10)
                      ;; T2-3/T3-5: match 式 (tag=10): [10, scrutinee, arm-count, pat1, body1, pat2, body2, ...]
                      ;; scrutinee をコンパイルし、各パターンをif-elseチェーンに変換
                      (let [scrutinee (vector-get node 1)
                            arm-count (vector-get node 2)
                            ;; scrutinee をコンパイルしてローカルに保存
                            scr-idx (+ 1 (map-size env))
                            instrs1 (compile-expr-with-ftable scrutinee env ftable instrs)
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
                                i7 (compile-expr-with-ftable body1 env ftable i6)
                                ;; else
                                i8 (emit-to i7 43 0)]
                            (if (> arm-count 1)
                              (let [pat2 (vector-get node 5)
                                    body2 (vector-get node 6)
                                    i9 (emit-to i8 10 scr-idx)
                                    i10 (emit-to i9 1 pat2)
                                    i11 (emit-to i10 30 0)
                                    i12 (emit-to i11 41 0)
                                    i13 (compile-expr-with-ftable body2 env ftable i12)
                                    i14 (emit-to i13 43 0)]
                                (if (> arm-count 2)
                                  (let [pat3 (vector-get node 7)
                                        body3 (vector-get node 8)
                                        i15 (emit-to i14 10 scr-idx)
                                        i16 (emit-to i15 1 pat3)
                                        i17 (emit-to i16 30 0)
                                        i18 (emit-to i17 41 0)
                                        i19 (compile-expr-with-ftable body3 env ftable i18)
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

(defn compile-expr [node env instrs]
  (compile-expr-with-ftable node env (ftable-new) instrs))

;; === defn 宣言のコンパイル ===

;; defn 宣言 (tag=20): [20, name-hash, param-count, param1-hash, ..., body-expr]
;; パラメータを環境に登録して body をコンパイルし、IR 命令列を返す
(defn compile-defn-with-ftable [node ftable]
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
        (compile-expr-with-ftable body-expr (ref-get env) ftable (vector-new 8))))))

(defn compile-defn [node]
  (compile-defn-with-ftable node (ftable-new)))

(defn compile-defn-function-with-source [node source ftable data-ref]
  (let [param-count (vector-get node 2)
        ir (compile-defn-with-source node source ftable data-ref)
        local-max (max-local-slot ir 0 (vector-length ir) 0)
        local-count (if (> local-max param-count) (- local-max param-count) 0)]
    (make-function-meta param-count local-count ir)))

(defn compile-defn-functions-with-source [decls idx n source ftable data-ref functions]
  (if (>= idx n)
    functions
    (let [decl (vector-get decls idx)]
      (if (= (vector-get decl 0) 20)
        (compile-defn-functions-with-source
          decls
          (+ idx 1)
          n
          source
          ftable
          data-ref
          (vector-push functions (compile-defn-function-with-source decl source ftable data-ref)))
        (compile-defn-functions-with-source decls (+ idx 1) n source ftable data-ref functions)))))

(defn compile-program-functions-with-source [src decls]
  (let [n (vector-length decls)
        pass1 (register-defns decls 0 n (ftable-new) 0)
        ftable (vector-get pass1 0)
        data-ref (ref-new (vector-new 8))
        functions (compile-defn-functions-with-source decls 0 n src ftable data-ref (vector-new 8))]
    (vector-push
      (vector-push
        (vector-push (vector-new 3) ftable)
        functions)
      (ref-get data-ref))))

(defn compile-program-with-source [src decls]
  (let [pair (compile-program-functions-with-source src decls)
        ftable (vector-get pair 0)
        functions (vector-get pair 1)
        data (vector-get pair 2)
        ir-list (collect-function-irs functions 0 (vector-length functions) (vector-new 8))]
    (vector-push (vector-push (vector-push (vector-new 3) ftable) ir-list) data)))

;; IR 命令列に含まれる最大ローカルスロット番号を返す
(defn max-local-slot-op [opcode operand current-max]
  (if (or (or (= opcode 10) (= opcode 11)) (or (= opcode 50) (= opcode 53)))
    (if (> operand current-max) operand current-max)
    (if (= opcode 54)
      (if (> (+ operand 1) current-max) (+ operand 1) current-max)
      (if (= opcode 55)
        (if (> (+ operand 5) current-max) (+ operand 5) current-max)
        (if (= opcode 56)
          (if (> (+ operand 1) current-max) (+ operand 1) current-max)
          (if (= opcode 58)
            (if (> operand current-max) operand current-max)
            (if (= opcode 60)
              (if (> operand current-max) operand current-max)
              (if (= opcode 62)
                (if (> (+ operand 5) current-max) (+ operand 5) current-max)
                (if (= opcode 63)
                  (if (> (+ operand 5) current-max) (+ operand 5) current-max)
                  (if (= opcode 65)
                    (if (> (+ operand 5) current-max) (+ operand 5) current-max)
                    (if (= opcode 66)
                      (if (> (+ operand 5) current-max) (+ operand 5) current-max)
                      current-max)))))))))))

(defn max-local-slot [instrs idx count current-max]
  (if (>= idx count)
    current-max
    (let [instr (vector-get instrs idx)
          opcode (vector-get instr 0)
          operand (vector-get instr 1)
          next-max (max-local-slot-op opcode operand current-max)]
      (max-local-slot instrs (+ idx 1) count next-max))))

;; 関数 metadata: [param-count, local-count, ir]
(defn make-function-meta [param-count local-count ir]
  (vector-push
    (vector-push
      (vector-push (vector-new 3) param-count)
      local-count)
    ir))

;; defn から関数 metadata を生成する
(defn compile-defn-function [node ftable]
  (let [param-count (vector-get node 2)
        ir (compile-defn-with-ftable node ftable)
        local-max (max-local-slot ir 0 (vector-length ir) 0)
        local-count (if (> local-max param-count) (- local-max param-count) 0)]
    (make-function-meta param-count local-count ir)))

;; defn 名をすべて関数テーブルへ登録する
(defn register-defns [decls idx n ftable func-idx]
  (if (>= idx n)
    (vector-push (vector-push (vector-new 2) ftable) func-idx)
    (let [decl (vector-get decls idx)]
      (if (= (vector-get decl 0) 20)
        (register-defns decls (+ idx 1) n (ftable-register ftable (vector-get decl 1) func-idx) (+ func-idx 1))
        (register-defns decls (+ idx 1) n ftable func-idx)))))

;; defn を順にコンパイルし、metadata list を返す
(defn compile-defn-functions [decls idx n ftable functions]
  (if (>= idx n)
    functions
    (let [decl (vector-get decls idx)]
      (if (= (vector-get decl 0) 20)
        (compile-defn-functions
          decls
          (+ idx 1)
          n
          ftable
          (vector-push functions (compile-defn-function decl ftable)))
        (compile-defn-functions decls (+ idx 1) n ftable functions)))))

;; 関数 metadata list から IR list を取り出す
(defn collect-function-irs [functions idx count ir-list]
  (if (>= idx count)
    ir-list
    (collect-function-irs
      functions
      (+ idx 1)
      count
      (vector-push ir-list (vector-get (vector-get functions idx) 2)))))

;; 複数の defn 宣言をコンパイルし、関数テーブルと関数 metadata list を返す
;; 結果: [ftable, functions] の Vector
;; functions: 各関数の [param-count, local-count, ir]
(defn compile-program-functions [decls]
  (let [n (vector-length decls)
        pass1 (register-defns decls 0 n (ftable-new) 0)
        ftable (vector-get pass1 0)
        functions (compile-defn-functions decls 0 n ftable (vector-new 8))]
    (vector-push (vector-push (vector-new 2) ftable) functions)))

;; === プログラム全体のコンパイル ===

;; 複数の defn 宣言をコンパイルし、関数テーブルと IR 命令列のリストを返す
;; 結果: [ftable, ir-list] の Vector
;; ir-list: 各関数の IR 命令列の Vector
(defn compile-program [decls]
  (let [pair (compile-program-functions decls)
        ftable (vector-get pair 0)
        functions (vector-get pair 1)
        ir-list (collect-function-irs functions 0 (vector-length functions) (vector-new 8))]
    (vector-push (vector-push (vector-new 2) ftable) ir-list)))

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
