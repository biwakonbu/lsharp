use super::support::*;


#[test]
fn test_e2e_stdlib_set() {
    // Set.ls: HashMap ベースの集合
    let result = compile_and_run(r#"
        (defn set-new [] (map-new))
        (defn set-add [s x] (map-insert s x 1))
        (defn set-contains? [s x] (map-contains? s x))
        (defn set-remove [s x] (map-remove s x))
        (defn set-size [s] (map-size s))
        (defn main []
          (let [s (set-new)
                s1 (set-add s 10)
                s2 (set-add s1 20)
                s3 (set-add s2 30)]
            (do
              (print (set-size s3))
              (print (set-contains? s3 20))
              (print (set-contains? s3 99))
              0)))
    "#);
    assert_eq!(result.trim(), "3\n1\n0");
}

// === ファイル I/O & WASI 拡張テスト ===

#[test]
fn test_e2e_command_line_args() {
    // command-line-args: コマンドライン引数の数を返す
    // wasmtime で実行した場合、引数が 0 以上の整数が返る
    let result = compile_and_run(r#"
        (defn main []
          (let [argc (command-line-args)]
            (do
              (print (>= argc 0))
              0)))
    "#);
    // argc >= 0 は常に true (1)
    assert_eq!(result.trim(), "1");
}

#[test]
fn test_e2e_write_and_read_file() {
    // write-file + read-file: ファイルに書き込んで読み出し
    let tmpdir = std::env::temp_dir().join("lsharp_test_file_io");
    std::fs::create_dir_all(&tmpdir).unwrap();
    let result = compile_and_run_with_dir(r#"
        (defn main []
          (let [written (write-file "test_output.txt" "hello")
                content (read-file "test_output.txt")]
            (do
              (print written)
              (print (string-length content))
              0)))
    "#, &tmpdir);
    // written = 5 (bytes), content length = 5
    assert_eq!(result.trim(), "5\n5");
    // クリーンアップ
    let _ = std::fs::remove_dir_all(&tmpdir);
}

#[test]
fn test_e2e_file_exists() {
    // file-exists?: ファイル存在チェック (preopened dir 付き)
    let tmpdir = std::env::temp_dir().join("lsharp_test_file_exists");
    std::fs::create_dir_all(&tmpdir).unwrap();
    let result = compile_and_run_with_dir(r#"
        (defn main []
          (do
            (print (file-exists? "nonexistent_file_xyz.txt"))
            0))
    "#, &tmpdir);
    assert_eq!(result.trim(), "0");
    let _ = std::fs::remove_dir_all(&tmpdir);
}

// === セルフホスティング: Lexer テスト ===

#[test]
fn test_e2e_selfhost_lexer_basic() {
    // セルフホスティング Lexer: 基本トークナイズ
    let result = compile_and_run(r#"
        (defn is-ws [c]
          (if (== c 32) true (if (== c 9) true (if (== c 10) true (== c 13)))))
        (defn is-digit-char [c]
          (if (>= c 48) (<= c 57) false))
        (defn is-alpha-char [c]
          (if (>= c 65) (if (<= c 90) true (if (>= c 97) (<= c 122) false)) false))
        (defn is-symbol-start [c]
          (if (is-alpha-char c) true
            (if (== c 95) true (if (== c 43) true (if (== c 45) true
              (if (== c 42) true (if (== c 47) true (if (== c 61) true
                (if (== c 60) true (if (== c 62) true (if (== c 33) true
                  (if (== c 63) true false))))))))))))
        (defn is-symbol-char [c]
          (if (is-symbol-start c) true (if (is-digit-char c) true (if (== c 46) true (== c 45)))))
        (defn skip-comment [src pos len]
          (if (>= pos len) pos
            (if (== (string-char-at src pos) 10) (+ pos 1)
              (skip-comment src (+ pos 1) len))))
        (defn skip-ws-loop [src pos len]
          (if (>= pos len) pos
            (let [c (string-char-at src pos)]
              (if (is-ws c) (skip-ws-loop src (+ pos 1) len)
                (if (== c 59) (let [end (skip-comment src (+ pos 1) len)]
                  (skip-ws-loop src end len)) pos)))))
        (defn classify-symbol [name]
          (if (string-eq name "defn") 30
            (if (string-eq name "let") 31
              (if (string-eq name "if") 32
                (if (string-eq name "true") 13
                  (if (string-eq name "false") 14 20))))))
        (defn scan-digits [src pos len]
          (if (>= pos len) pos
            (if (is-digit-char (string-char-at src pos)) (scan-digits src (+ pos 1) len) pos)))
        (defn scan-symbol-end [src pos len]
          (if (>= pos len) pos
            (if (is-symbol-char (string-char-at src pos)) (scan-symbol-end src (+ pos 1) len) pos)))
        (defn lex-one [src pos len]
          (if (>= pos len) (+ (* 99 1000000) pos)
            (let [c (string-char-at src pos)]
              (if (== c 40) (+ (* 0 1000000) (+ pos 1))
                (if (== c 41) (+ (* 1 1000000) (+ pos 1))
                  (if (== c 91) (+ (* 2 1000000) (+ pos 1))
                    (if (== c 93) (+ (* 3 1000000) (+ pos 1))
                      (if (is-digit-char c)
                        (let [end (scan-digits src (+ pos 1) len)]
                          (+ (* 10 1000000) end))
                        (if (is-symbol-start c)
                          (let [end (scan-symbol-end src (+ pos 1) len)
                                name (substring src pos end)
                                kind (classify-symbol name)]
                            (+ (* kind 1000000) end))
                          (+ (* 99 1000000) (+ pos 1)))))))))))
        (defn tokenize-loop [src pos len tokens]
          (let [ws-pos (skip-ws-loop src pos len)]
            (if (>= ws-pos len)
              (vector-push tokens 99)
              (let [result (lex-one src ws-pos len)
                    kind (/ result 1000000)
                    end-pos (- result (* kind 1000000))]
                (if (== kind 99)
                  (vector-push tokens 99)
                  (tokenize-loop src end-pos len (vector-push tokens kind)))))))
        (defn tokenize [src]
          (tokenize-loop src 0 (string-length src) (vector-new 16)))
        (defn main []
          (let [tokens (tokenize "(defn main [] 42)")
                len (vector-length tokens)]
            (do
              (print len)
              (print (vector-get tokens 0))
              (print (vector-get tokens 1))
              (print (vector-get tokens 2))
              (print (vector-get tokens 3))
              (print (vector-get tokens 4))
              (print (vector-get tokens 5))
              (print (vector-get tokens 6))
              (print (vector-get tokens 7))
              0)))
    "#);
    // 8 tokens: ( defn main [ ] 42 ) EOF
    // kinds:    0  30   20  2 3 10  1  99
    assert_eq!(result.trim(), "8\n0\n30\n20\n2\n3\n10\n1\n99");
}

#[test]
fn test_e2e_selfhost_parser_basic() {
    // セルフホスティング Parser: 基本的な S 式パース
    let result = compile_and_run(r#"
        (defn parse-expr [tokens pos]
          (let [tok (vector-get tokens (ref-get pos))]
            (if (== tok 0)
              (do (ref-set pos (+ (ref-get pos) 1))
                (let [inner-tok (vector-get tokens (ref-get pos))
                      result (if (== inner-tok 30) (do (ref-set pos (+ (ref-get pos) 1)) 20)
                               (if (== inner-tok 32) (do (ref-set pos (+ (ref-get pos) 1)) 6)
                                 5))]
                  (do
                    ;; skip until )
                    result)))
              (if (== tok 10) (do (ref-set pos (+ (ref-get pos) 1)) 1)
                (if (== tok 20) (do (ref-set pos (+ (ref-get pos) 1)) 4)
                  (if (== tok 13) (do (ref-set pos (+ (ref-get pos) 1)) 2)
                    0))))))
        (defn main []
          (let [tokens (vector-push (vector-push (vector-push (vector-push
                        (vector-push (vector-push (vector-push (vector-push
                          (vector-new 8) 0) 30) 20) 2) 3) 10) 1) 99)
                pos (ref-new 0)
                result (parse-expr tokens pos)]
            (do
              (print result)
              (print (ref-get pos))
              0)))
    "#);
    // defn ノード (20) を検出、位置は 2 進んだ
    assert_eq!(result.trim(), "20\n2");
}

#[test]
fn test_e2e_selfhost_type_system() {
    // セルフホスティング型システム: 型 ADT + Substitution
    let result = compile_and_run(r#"
        (defn make-type-con [hash]
          (vector-push (vector-push (vector-new 2) 1) hash))
        (defn make-type-var [id]
          (vector-push (vector-push (vector-new 2) 2) id))
        (defn type-tag [ty] (vector-get ty 0))
        (defn type-val [ty] (vector-get ty 1))
        (defn subst-new [] (map-new))
        (defn subst-bind [s var-id ty-tag] (map-insert s var-id ty-tag))
        (defn subst-lookup [s var-id] (map-get s var-id))
        (defn main []
          (let [int-ty (make-type-con 0)
                var-ty (make-type-var 42)
                s (subst-bind (subst-new) 42 0)]
            (do
              (print (type-tag int-ty))
              (print (type-tag var-ty))
              (print (type-val var-ty))
              (print (subst-lookup s 42))
              0)))
    "#);
    assert_eq!(result.trim(), "1\n2\n42\n0");
}

#[test]
fn test_e2e_selfhost_unification() {
    // セルフホスティング Unification: 型構築 + Substitution + occurs-check + unify
    // map-contains? (Bool) を避け、map-get + = (Int比較) で統一
    let result = compile_and_run(r#"
        ;; 型構築
        (defn make-type-con [hash]
          (vector-push (vector-push (vector-new 2) 1) hash))
        (defn make-type-int [] (make-type-con 100))
        (defn make-type-bool [] (make-type-con 200))
        (defn make-type-var [id]
          (vector-push (vector-push (vector-new 2) 2) id))

        ;; 型アクセス
        (defn type-tag [ty] (vector-get ty 0))
        (defn type-name [ty] (vector-get ty 1))

        ;; Substitution (map-get のみ使用、map-contains? を避ける)
        (defn subst-new [] (map-new))
        (defn subst-bind [s var-id ty] (map-insert s var-id ty))

        ;; 型の等価判定 (1=等しい, 0=異なる)
        (defn types-eq [ty1 ty2]
          (if (= (type-tag ty1) (type-tag ty2))
            (if (= (type-name ty1) (type-name ty2)) 1 0)
            0))

        ;; occurs-check (1=出現, 0=非出現)
        (defn occurs-check [var-id ty]
          (if (= (type-tag ty) 2)
            (if (= var-id (type-name ty)) 1 0)
            0))

        ;; エラーマーカー: 特殊キー -1 に値 1 を入れた Map
        (defn unify-error [] (map-insert (map-new) -1 1))
        ;; エラー判定: map-get で -1 キーを取得 (0 = エラーなし)
        (defn is-error [s] (map-get s -1))

        ;; 単純 unify (Con/Var のみ)
        (defn unify-simple [t1 t2 subst]
          (if (= (types-eq t1 t2) 1)
            subst
            (if (= (type-tag t1) 2)
              (if (= (occurs-check (type-name t1) t2) 1)
                (unify-error)
                (subst-bind subst (type-name t1) t2))
              (if (= (type-tag t2) 2)
                (if (= (occurs-check (type-name t2) t1) 1)
                  (unify-error)
                  (subst-bind subst (type-name t2) t1))
                (unify-error)))))

        ;; apply-subst: Con/Var 型のみ
        (defn apply-subst-simple [subst ty]
          (if (= (type-tag ty) 2)
            (let [looked (map-get subst (type-name ty))]
              (if (= looked 0)
                ty
                looked))
            ty))

        (defn main []
          (let [int1 (make-type-int)
                int2 (make-type-int)
                bool1 (make-type-bool)
                var1 (make-type-var 10)
                s0 (subst-new)]
            (do
              ;; テスト1: Int == Int → 成功 (is-error=0)
              (let [r1 (unify-simple int1 int2 s0)]
                (print (if (= (is-error r1) 0) 1 0)))

              ;; テスト2: Int != Bool → 失敗 (is-error=1)
              (let [r2 (unify-simple int1 bool1 s0)]
                (print (if (= (is-error r2) 0) 1 0)))

              ;; テスト3: Var(10) と Int → 成功 + 置換
              (let [r3 (unify-simple var1 int1 s0)]
                (do
                  (print (if (= (is-error r3) 0) 1 0))
                  ;; 置換に var-id=10 が含まれる (map-get で確認)
                  (let [v10 (map-get r3 10)]
                    (print (if (= v10 0) 0 1)))
                  ;; apply-subst で Var(10) → Int
                  (let [resolved (apply-subst-simple r3 var1)]
                    (do
                      (print (type-tag resolved))
                      (print (type-name resolved))))))

              ;; テスト4: occurs-check
              (print (occurs-check 10 var1))
              (print (occurs-check 99 var1))
              (print (occurs-check 10 int1))

              0)))
    "#);
    assert_eq!(result.trim(), "1\n0\n1\n1\n1\n100\n1\n0\n0");
}

#[test]
fn test_e2e_selfhost_ir() {
    // セルフホスティング IR: 命令構築
    let result = compile_and_run(r#"
        (defn make-instr [opcode operand]
          (vector-push (vector-push (vector-new 2) opcode) operand))
        (defn main []
          (let [c (make-instr 1 42)
                g (make-instr 10 0)]
            (do
              (print (vector-get c 0))
              (print (vector-get c 1))
              (print (vector-get g 0))
              (print (vector-get g 1))
              0)))
    "#);
    assert_eq!(result.trim(), "1\n42\n10\n0");
}

#[test]
fn test_e2e_selfhost_compiler() {
    // セルフホスティング Compiler: AST→IR 変換 + LEB128 エンコード
    let result = compile_and_run(r#"
        ;; IR 命令構築
        (defn emit-instr [opcode operand]
          (vector-push (vector-push (vector-new 2) opcode) operand))

        (defn emit-to [instrs opcode operand]
          (vector-push instrs (emit-instr opcode operand)))

        ;; 環境 (変数名ハッシュ → ローカルインデックス)
        (defn env-new [] (map-new))
        (defn env-bind [env name-hash idx] (map-insert env name-hash idx))
        (defn env-lookup [env name-hash] (map-get env name-hash))

        ;; AST → IR コンパイル (整数リテラル, 真偽値, 変数参照)
        (defn compile-expr [node env instrs]
          (let [tag (vector-get node 0)]
            (if (= tag 1)
              (emit-to instrs 1 (vector-get node 1))
              (if (= tag 2)
                (emit-to instrs 1 (vector-get node 1))
                (if (= tag 4)
                  (let [name-hash (vector-get node 1)
                        idx (env-lookup env name-hash)]
                    (if (= idx 0)
                      (emit-to instrs 1 0)
                      (emit-to instrs 10 idx)))
                  (emit-to instrs 1 0))))))

        ;; LEB128 符号なしエンコード
        (defn leb128-unsigned [value]
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

        (defn main []
          (let [;; 整数リテラル [1, 42] をコンパイル
                lit-node (vector-push (vector-push (vector-new 2) 1) 42)
                env (env-new)
                instrs (compile-expr lit-node env (vector-new 8))

                ;; 変数参照 [4, 99] を環境ありでコンパイル
                var-node (vector-push (vector-push (vector-new 2) 4) 99)
                env2 (env-bind env 99 3)
                instrs2 (compile-expr var-node env2 (vector-new 8))

                ;; LEB128 テスト
                leb5 (leb128-unsigned 5)
                leb300 (leb128-unsigned 300)]
            (do
              ;; 整数リテラルのコンパイル結果
              (print (vector-length instrs))
              (let [i0 (vector-get instrs 0)]
                (do
                  (print (vector-get i0 0))
                  (print (vector-get i0 1))))

              ;; 変数参照のコンパイル結果
              (print (vector-length instrs2))
              (let [i1 (vector-get instrs2 0)]
                (do
                  (print (vector-get i1 0))
                  (print (vector-get i1 1))))

              ;; LEB128
              (print (vector-length leb5))
              (print (vector-get leb5 0))
              (print (vector-length leb300))
              (print (vector-get leb300 0))
              (print (vector-get leb300 1))
              0)))
    "#);
    assert_eq!(result.trim(), "1\n1\n42\n1\n10\n3\n1\n5\n2\n172\n2");
}

#[test]
fn test_e2e_selfhost_type_scheme() {
    // セルフホスティング: TypeScheme (let 多相の instantiate/free-vars)
    let result = compile_and_run(r#"
        ;; TypeScheme = [type, bound-vars-vector]
        (defn mono [ty]
          (vector-push (vector-push (vector-new 2) ty) (vector-new 0)))

        (defn poly [ty bound-vars]
          (vector-push (vector-push (vector-new 2) ty) bound-vars))

        (defn scheme-type [scheme] (vector-get scheme 0))
        (defn scheme-vars [scheme] (vector-get scheme 1))

        ;; 型変数カウンタ
        (defn make-var-counter [] (ref-new 1000))
        (defn next-var [counter]
          (let [id (ref-get counter)]
            (do (ref-set counter (+ id 1)) id)))

        ;; instantiate-apply: 置換を型に適用
        (defn inst-apply [subst ty]
          (let [tag (vector-get ty 0)]
            (if (= tag 2)
              (let [looked (map-get subst (vector-get ty 1))]
                (if (= looked 0) ty looked))
              (if (= tag 3)
                (vector-push
                  (vector-push
                    (vector-push (vector-new 3) 3)
                    (inst-apply subst (vector-get ty 1)))
                  (inst-apply subst (vector-get ty 2)))
                ty))))

        ;; instantiate: 型スキームを具体化
        (defn instantiate [scheme counter]
          (let [ty (scheme-type scheme)
                vars (scheme-vars scheme)
                n (vector-length vars)]
            (if (= n 0)
              ty
              (let [subst (ref-new (map-new))
                    i (ref-new 0)]
                (do
                  (if (< (ref-get i) n)
                    (do
                      (let [old-v (vector-get vars (ref-get i))
                            new-id (next-var counter)
                            new-ty (vector-push (vector-push (vector-new 2) 2) new-id)]
                        (ref-set subst (map-insert (ref-get subst) old-v new-ty)))
                      (ref-set i (+ (ref-get i) 1))
                      0)
                    0)
                  (inst-apply (ref-get subst) ty))))))

        ;; free-vars: 型の自由変数を収集
        (defn free-vars [ty]
          (let [tag (vector-get ty 0)]
            (if (= tag 2)
              (vector-push (vector-new 1) (vector-get ty 1))
              (if (= tag 3)
                (let [pv (free-vars (vector-get ty 1))
                      rv (free-vars (vector-get ty 2))
                      result (ref-new pv)
                      j (ref-new 0)
                      m (vector-length rv)]
                  (do
                    (if (< (ref-get j) m)
                      (do
                        (ref-set result (vector-push (ref-get result) (vector-get rv (ref-get j))))
                        (ref-set j (+ (ref-get j) 1))
                        0)
                      0)
                    (ref-get result)))
                (vector-new 0)))))

        (defn main []
          (let [;; 型準備
                int-ty (vector-push (vector-push (vector-new 2) 1) 100)
                var-a (vector-push (vector-push (vector-new 2) 2) 1)
                fun-ty (vector-push (vector-push (vector-push (vector-new 3) 3) var-a) var-a)

                ;; 型スキーム
                int-scheme (mono int-ty)
                bound (vector-push (vector-new 1) 1)
                id-scheme (poly fun-ty bound)

                ;; instantiate
                counter (make-var-counter)
                inst1 (instantiate int-scheme counter)
                inst2 (instantiate id-scheme counter)]
            (do
              ;; 単相の instantiate
              (print (vector-get inst1 0))  ;; 1 (Con)
              (print (vector-get inst1 1))  ;; 100

              ;; 多相の instantiate (Fun型 + 新型変数)
              (print (vector-get inst2 0))  ;; 3 (Fun)
              (let [param (vector-get inst2 1)]
                (do
                  (print (vector-get param 0))  ;; 2 (Var)
                  (print (vector-get param 1)))) ;; 1000

              ;; free-vars
              (print (vector-length (free-vars int-ty)))  ;; 0
              (print (vector-length (free-vars var-a)))   ;; 1
              (print (vector-get (free-vars var-a) 0))    ;; 1

              0)))
    "#);
    assert_eq!(result.trim(), "1\n100\n3\n2\n1000\n0\n1\n1");
}

#[test]
fn test_e2e_selfhost_wasm_emit() {
    let result = compile_and_run(r#"
        ;; LEB128 unsigned エンコーディング
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

        ;; Wasm ヘッダー (8 バイト)
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

        (defn main []
          (let [header (emit-header)
                type-sec (emit-type-section-main)
                leb5 (leb128-u 5)
                leb300 (leb128-u 300)]
            (do
              ;; ヘッダー検証
              (print (vector-length header))
              (print (vector-get header 0))
              (print (vector-get header 1))
              (print (vector-get header 2))
              (print (vector-get header 3))
              (print (vector-get header 4))

              ;; Type セクション検証
              (print (vector-length type-sec))
              (print (vector-get type-sec 0))
              (print (vector-get type-sec 1))
              (print (vector-get type-sec 2))
              (print (vector-get type-sec 3))

              ;; LEB128 検証
              (print (vector-get leb5 0))
              (print (vector-get leb300 0))
              (print (vector-get leb300 1))

              0)))
    "#);
    // header: length=8, bytes: 0('\\0'), 97('a'), 115('s'), 109('m'), 1(version)
    // type-sec: length=7, bytes: 1(section-id), 5(size), 1(count), 96(0x60=func)
    // leb128(5)=[5], leb128(300)=[172, 2]
    assert_eq!(result.trim(), "8\n0\n97\n115\n109\n1\n7\n1\n5\n1\n96\n5\n172\n2");
}

#[test]
fn test_e2e_selfhost_type_inference_comparison() {
    // セルフホスト型推論 vs Rust 型推論の比較テスト
    // L# の Type.ls パターンで型を構築し、Rust の Type 列挙型と同等の表現を検証
    //
    // 対応関係:
    //   L# make-type-con(100) = [1, 100]  ↔  Rust Type::Con("Int")
    //   L# make-type-var(42)  = [2, 42]   ↔  Rust Type::Var(42)
    //   L# make-type-fun(p,r) = [3, p, r] ↔  Rust Type::Fun(vec![p], Box::new(r))
    //   L# subst-bind/apply-subst          ↔  Rust Substitution::apply
    let result = compile_and_run(r#"
        ;; 型構築 (Type.ls パターン)
        (defn make-type-con [hash]
          (vector-push (vector-push (vector-new 2) 1) hash))
        (defn make-type-var [id]
          (vector-push (vector-push (vector-new 2) 2) id))
        (defn make-type-fun [param-ty ret-ty]
          (vector-push (vector-push (vector-push (vector-new 3) 3) param-ty) ret-ty))

        ;; 型アクセス
        (defn type-tag [ty] (vector-get ty 0))
        (defn type-name [ty] (vector-get ty 1))
        (defn type-fun-param [ty] (vector-get ty 1))
        (defn type-fun-ret [ty] (vector-get ty 2))

        ;; Substitution
        (defn subst-new [] (map-new))
        (defn subst-bind [s var-id ty] (map-insert s var-id ty))
        (defn subst-lookup [s var-id] (map-get s var-id))

        ;; apply-subst: 置換を型に適用
        (defn apply-subst [subst ty]
          (if (= (type-tag ty) 2)
            (let [looked (subst-lookup subst (type-name ty))]
              (if (= looked 0)
                ty
                (apply-subst subst looked)))
            (if (= (type-tag ty) 3)
              (make-type-fun
                (apply-subst subst (type-fun-param ty))
                (apply-subst subst (type-fun-ret ty)))
              ty)))

        ;; 型等価判定 (1=等しい, 0=異なる)
        (defn types-eq [ty1 ty2]
          (if (= (type-tag ty1) (type-tag ty2))
            (if (= (type-tag ty1) 1)
              (if (= (type-name ty1) (type-name ty2)) 1 0)
              (if (= (type-tag ty1) 2)
                (if (= (type-name ty1) (type-name ty2)) 1 0)
                0))
            0))

        (defn main []
          (let [int-ty (make-type-con 100)
                var-ty (make-type-var 42)
                var1 (make-type-var 1)
                var2 (make-type-var 2)
                fun-ty (make-type-fun var1 var2)]
            (do
              ;; テスト1: Con 型構築 (Rust: Type::Con("Int") → tag=1, hash=100)
              (print (type-tag int-ty))
              (print (type-name int-ty))

              ;; テスト2: Var 型構築 (Rust: Type::Var(42) → tag=2, id=42)
              (print (type-tag var-ty))
              (print (type-name var-ty))

              ;; テスト3: Fun 型構築 (Rust: Type::Fun → tag=3, param/ret)
              (print (type-tag fun-ty))
              (print (type-tag (type-fun-param fun-ty)))
              (print (type-name (type-fun-param fun-ty)))
              (print (type-tag (type-fun-ret fun-ty)))
              (print (type-name (type-fun-ret fun-ty)))

              ;; テスト4: Substitution 比較 (Rust: Substitution::apply)
              ;; {42 -> Con(100)} を適用: Var(42) → Con(100)
              (let [s (subst-bind (subst-new) 42 int-ty)
                    resolved (apply-subst s var-ty)]
                (do
                  (print (type-tag resolved))
                  (print (type-name resolved))))

              ;; テスト5: types-eq 比較
              ;; Con(100) == Con(100) → 1
              (print (types-eq int-ty (make-type-con 100)))
              ;; Con(100) != Con(200) → 0
              (print (types-eq int-ty (make-type-con 200)))
              ;; Var(42) == Var(42) → 1
              (print (types-eq var-ty (make-type-var 42)))

              0)))
    "#);
    // Con: tag=1, hash=100
    // Var: tag=2, id=42
    // Fun: tag=3, param(tag=2,id=1), ret(tag=2,id=2)
    // Subst: resolved → Con(tag=1, hash=100)
    // types-eq: 1, 0, 1
    assert_eq!(
        result.trim(),
        "1\n100\n2\n42\n3\n2\n1\n2\n2\n1\n100\n1\n0\n1"
    );
}
