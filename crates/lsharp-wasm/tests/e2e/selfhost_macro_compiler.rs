use super::support::*;

// =================================================// selfhost Compiler.ls 拡張テスト (Step 5)
// =================================================
#[test]
fn test_e2e_selfhost_compiler_if_let_pipeline() {
    // Parser v3 → Compiler パイプライン: if と let をコンパイルして IR を生成
    let source = r#"
;; === AST タグ + IR opcode 定数 ===
(defn tag-lit-int [] 1)
(defn tag-var [] 4)
(defn tag-if [] 6)
(defn tag-let [] 7)
(defn tag-apply [] 5)

(defn op-i64-const [] 1)
(defn op-local-get [] 10)
(defn op-local-set [] 11)
(defn op-i64-add [] 20)
(defn op-i64-eq [] 30)
(defn op-i64-gt [] 31)
(defn op-if [] 41)
(defn op-end [] 43)

;; IR 命令構築
(defn emit-instr [opcode operand]
  (vector-push (vector-push (vector-new 2) opcode) operand))

(defn emit-to [instrs opcode operand]
  (vector-push instrs (emit-instr opcode operand)))

;; 環境
(defn env-new [] (map-new))
(defn env-bind [env key val] (map-insert env key val))
(defn env-lookup [env key] (map-get env key))

;; ビルトイン演算子
(defn builtin-opcode [name-hash]
  (if (= name-hash 43) 20
    (if (= name-hash 62) 31
      (if (= name-hash 61) 30
        0))))

;; compile-expr (再帰: int/var/if/let/apply 対応)
(defn compile-expr [node env instrs]
  (let [tag (vector-get node 0)]
    (if (= tag 1)
      (emit-to instrs 1 (vector-get node 1))
      (if (= tag 4)
        (let [name-key (vector-get node 1)
              idx (env-lookup env name-key)]
          (if (= idx 0) (emit-to instrs 1 0)
            (emit-to instrs 10 idx)))
        (if (= tag 6)
          (let [cond-expr (vector-get node 1)
                then-expr (vector-get node 2)
                else-expr (vector-get node 3)
                i1 (compile-expr cond-expr env instrs)
                i2 (emit-to i1 41 0)
                i3 (compile-expr then-expr env i2)
                i4 (emit-to i3 43 0)
                i5 (compile-expr else-expr env i4)]
            (emit-to i5 43 0))
          (if (= tag 7)
            (let [name-key (vector-get node 1)
                  init-expr (vector-get node 2)
                  body-expr (vector-get node 3)
                  i1 (compile-expr init-expr env instrs)
                  new-idx (+ 1 (map-size env))
                  i2 (emit-to i1 11 new-idx)
                  new-env (env-bind env name-key new-idx)]
              (compile-expr body-expr new-env i2))
            (if (= tag 5)
              ;; apply: [5, func-node, arg-count, arg1, arg2, ...]
              (let [func-node (vector-get node 1)
                    bop (if (= (vector-get func-node 0) 4) (builtin-opcode (vector-get func-node 1)) 0)]
                (if (> bop 0)
                  (let [i1 (compile-expr (vector-get node 3) env instrs)
                        i2 (compile-expr (vector-get node 4) env i1)]
                    (emit-to i2 bop 0))
                  (emit-to instrs 1 0)))
              (emit-to instrs 1 0))))))))

;; === Lexer (インライン) ===
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
          (if (== c 63) true (if (== c 38) true
            (if (== c 37) true (== c 126)))))))))))))))
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
        (if (== c 59) (let [end (skip-comment src (+ pos 1) len)] (skip-ws-loop src end len))
          pos)))))
(defn classify-symbol [name]
  (if (string-eq name "defn") 30
    (if (string-eq name "let") 31
      (if (string-eq name "if") 32
        (if (string-eq name "match") 33
          (if (string-eq name "fn") 35
            (if (string-eq name "do") 36
              (if (string-eq name "true") 13
                (if (string-eq name "false") 14
                  20)))))))))
(defn scan-digits [src pos len]
  (if (>= pos len) pos
    (if (is-digit-char (string-char-at src pos)) (scan-digits src (+ pos 1) len) pos)))
(defn scan-symbol-end [src pos len]
  (if (>= pos len) pos
    (if (is-symbol-char (string-char-at src pos)) (scan-symbol-end src (+ pos 1) len) pos)))
(defn scan-string-end [src pos len]
  (if (>= pos len) pos
    (let [c (string-char-at src pos)]
      (if (== c 34) (+ pos 1) (if (== c 92) (scan-string-end src (+ pos 2) len)
        (scan-string-end src (+ pos 1) len))))))
(defn lex-one [src pos len]
  (if (>= pos len) (+ (* 99 1000000) pos)
    (let [c (string-char-at src pos)]
      (if (== c 40) (+ (* 0 1000000) (+ pos 1))
        (if (== c 41) (+ (* 1 1000000) (+ pos 1))
          (if (== c 91) (+ (* 2 1000000) (+ pos 1))
            (if (== c 93) (+ (* 3 1000000) (+ pos 1))
              (if (== c 34)
                (let [end (scan-string-end src (+ pos 1) len)] (+ (* 12 1000000) end))
                (if (is-digit-char c)
                  (let [end (scan-digits src (+ pos 1) len)] (+ (* 10 1000000) end))
                  (if (is-symbol-start c)
                    (let [end (scan-symbol-end src (+ pos 1) len)
                          name (substring src pos end)
                          kind (classify-symbol name)]
                      (+ (* kind 1000000) end))
                    (+ (* 99 1000000) (+ pos 1))))))))))))
(defn tokenize-spans-loop [src pos len tokens]
  (let [ws-pos (skip-ws-loop src pos len)]
    (if (>= ws-pos len)
      (vector-push (vector-push (vector-push tokens 99) ws-pos) ws-pos)
      (let [result (lex-one src ws-pos len)
            kind (/ result 1000000)
            end-pos (- result (* kind 1000000))]
        (if (== kind 99)
          (vector-push (vector-push (vector-push tokens 99) ws-pos) ws-pos)
          (tokenize-spans-loop src end-pos len
            (vector-push (vector-push (vector-push tokens kind) ws-pos) end-pos)))))))
(defn tokenize-with-spans [src]
  (tokenize-spans-loop src 0 (string-length src) (vector-new 32)))

;; === Parser v3 (インライン: if/let/apply) ===
(defn span-kind [spans n] (vector-get spans (* n 3)))
(defn p-current [spans pos-ref] (span-kind spans (ref-get pos-ref)))
(defn p-advance [pos-ref] (ref-set pos-ref (+ (ref-get pos-ref) 1)))
(defn p-start [spans pos-ref] (vector-get spans (+ (* (ref-get pos-ref) 3) 1)))
(defn p-end [spans pos-ref] (vector-get spans (+ (* (ref-get pos-ref) 3) 2)))
(defn p-expect [spans pos-ref expected]
  (if (== (p-current spans pos-ref) expected) (do (p-advance pos-ref) 1) 0))
(defn parse-int-from-str [src pos end acc]
  (if (>= pos end) acc
    (let [digit (- (string-char-at src pos) 48)]
      (parse-int-from-str src (+ pos 1) end (+ (* acc 10) digit)))))

(defn parse-expr-v3 [spans pos-ref src]
  (let [kind (p-current spans pos-ref)]
    (if (== kind 10)
      (let [start (p-start spans pos-ref) end-pos (p-end spans pos-ref)
            value (parse-int-from-str src start end-pos 0)]
        (do (p-advance pos-ref)
            (vector-push (vector-push (vector-new 2) 1) value)))
      (if (== kind 13) (do (p-advance pos-ref) (vector-push (vector-push (vector-new 2) 2) 1))
        (if (== kind 14) (do (p-advance pos-ref) (vector-push (vector-push (vector-new 2) 2) 0))
          (if (== kind 20)
            (let [start (p-start spans pos-ref)]
              (do (p-advance pos-ref) (vector-push (vector-push (vector-new 2) 4) start)))
            (if (== kind 0) (parse-sexp-v3 spans pos-ref src)
              (do (p-advance pos-ref) (vector-push (vector-push (vector-new 2) 1) 0)))))))))

(defn parse-sexp-v3 [spans pos-ref src]
  (do (p-advance pos-ref)
    (let [kind (p-current spans pos-ref)]
      (if (== kind 32) (parse-if-v3 spans pos-ref src)
        (if (== kind 31) (parse-let-v3 spans pos-ref src)
          (if (== kind 36) (parse-do-v3 spans pos-ref src)
            (parse-apply-v3 spans pos-ref src)))))))

(defn parse-if-v3 [spans pos-ref src]
  (do (p-advance pos-ref)
    (let [c (parse-expr-v3 spans pos-ref src)
          t (parse-expr-v3 spans pos-ref src)
          e (parse-expr-v3 spans pos-ref src)]
      (do (p-expect spans pos-ref 1)
          (vector-push (vector-push (vector-push (vector-push (vector-new 8) 6) c) t) e)))))

(defn parse-let-v3 [spans pos-ref src]
  (do (p-advance pos-ref) (p-expect spans pos-ref 2)
    (let [name-start (p-start spans pos-ref)]
      (do (p-advance pos-ref)
        (let [init (parse-expr-v3 spans pos-ref src)]
          (do (p-expect spans pos-ref 3)
            (let [body (parse-expr-v3 spans pos-ref src)]
              (do (p-expect spans pos-ref 1)
                (vector-push (vector-push (vector-push (vector-push (vector-new 8) 7)
                  name-start) init) body)))))))))

(defn parse-do-v3 [spans pos-ref src]
  (do (p-advance pos-ref)
    (let [first (parse-expr-v3 spans pos-ref src)
          second (if (== (p-current spans pos-ref) 1) first
                   (parse-expr-v3 spans pos-ref src))]
      (do (p-advance pos-ref)
        (vector-push (vector-push (vector-push (vector-new 8) 9) first) second)))))

(defn parse-apply-v3 [spans pos-ref src]
  (let [func (parse-expr-v3 spans pos-ref src)
        arg1 (if (== (p-current spans pos-ref) 1) 0
               (parse-expr-v3 spans pos-ref src))
        arg2 (if (== (p-current spans pos-ref) 1) 0
               (parse-expr-v3 spans pos-ref src))]
    (do (p-advance pos-ref)
      (vector-push (vector-push (vector-push (vector-push (vector-new 8) 5)
        func) 2) arg1))))

;; === テスト: Lexer → Parser → Compiler パイプライン ===
(defn main []
  (let [;; テスト1: (if (> 10 5) 42 0) → if コンパイル
        src1 "(if (> 10 5) 42 0)"
        spans1 (tokenize-with-spans src1)
        pos1 (ref-new 0)
        ast1 (parse-expr-v3 spans1 pos1 src1)
        ir1 (compile-expr ast1 (env-new) (vector-new 16))
        ir1-len (vector-length ir1)]
    (do
      (print (vector-get ast1 0))  ;; 6 (if tag)
      (print ir1-len)              ;; IR 命令数 > 0

      ;; テスト2: (let [x 5] (+ x 1)) → let コンパイル
      (let [src2 "(let [x 5] (+ x 1))"
            spans2 (tokenize-with-spans src2)
            pos2 (ref-new 0)
            ast2 (parse-expr-v3 spans2 pos2 src2)
            ir2 (compile-expr ast2 (env-new) (vector-new 16))
            ir2-len (vector-length ir2)]
        (do
          (print (vector-get ast2 0))  ;; 7 (let tag)
          (print ir2-len)              ;; IR 命令数 > 0
          0)))))
"#;
    let result = compile_and_run(source);
    let lines: Vec<&str> = result.trim().lines().collect();
    assert!(lines.len() >= 4, "4行以上の出力が期待される");
    assert_eq!(lines[0], "6", "if 式の AST tag");
    assert!(lines[1].parse::<i32>().unwrap() > 0, "if の IR 命令数 > 0");
    assert_eq!(lines[2], "7", "let 式の AST tag");
    assert!(lines[3].parse::<i32>().unwrap() > 0, "let の IR 命令数 > 0");
}

#[test]
fn test_e2e_selfhost_integrated_pipeline_v3() {
    // 統合パイプライン: ソース文字列 → Lexer → Parser v3 → Compiler → IR
    // Main.ls の compile-source の v3 版として検証
    let source = r#"
;; === 統合パイプライン v3 テスト ===
;; Lexer (tokenize-with-spans) → Parser v3 (parse-expr-v3) → Compiler (compile-expr)

;; --- Lexer ---
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
          (if (== c 63) true (if (== c 38) true
            (if (== c 37) true (== c 126)))))))))))))))
(defn is-symbol-char [c]
  (if (is-symbol-start c) true (if (is-digit-char c) true (if (== c 46) true (== c 45)))))
(defn skip-comment [src pos len]
  (if (>= pos len) pos
    (if (== (string-char-at src pos) 10) (+ pos 1) (skip-comment src (+ pos 1) len))))
(defn skip-ws-loop [src pos len]
  (if (>= pos len) pos
    (let [c (string-char-at src pos)]
      (if (is-ws c) (skip-ws-loop src (+ pos 1) len)
        (if (== c 59) (let [end (skip-comment src (+ pos 1) len)] (skip-ws-loop src end len))
          pos)))))
(defn classify-symbol [name]
  (if (string-eq name "defn") 30
    (if (string-eq name "let") 31
      (if (string-eq name "if") 32
        (if (string-eq name "match") 33
          (if (string-eq name "fn") 35
            (if (string-eq name "do") 36
              (if (string-eq name "true") 13
                (if (string-eq name "false") 14
                  20)))))))))
(defn scan-digits [src pos len]
  (if (>= pos len) pos
    (if (is-digit-char (string-char-at src pos)) (scan-digits src (+ pos 1) len) pos)))
(defn scan-symbol-end [src pos len]
  (if (>= pos len) pos
    (if (is-symbol-char (string-char-at src pos)) (scan-symbol-end src (+ pos 1) len) pos)))
(defn scan-string-end [src pos len]
  (if (>= pos len) pos
    (let [c (string-char-at src pos)]
      (if (== c 34) (+ pos 1) (if (== c 92) (scan-string-end src (+ pos 2) len)
        (scan-string-end src (+ pos 1) len))))))
(defn lex-one [src pos len]
  (if (>= pos len) (+ (* 99 1000000) pos)
    (let [c (string-char-at src pos)]
      (if (== c 40) (+ (* 0 1000000) (+ pos 1))
        (if (== c 41) (+ (* 1 1000000) (+ pos 1))
          (if (== c 91) (+ (* 2 1000000) (+ pos 1))
            (if (== c 93) (+ (* 3 1000000) (+ pos 1))
              (if (== c 34)
                (let [end (scan-string-end src (+ pos 1) len)] (+ (* 12 1000000) end))
                (if (is-digit-char c)
                  (let [end (scan-digits src (+ pos 1) len)] (+ (* 10 1000000) end))
                  (if (is-symbol-start c)
                    (let [end (scan-symbol-end src (+ pos 1) len)
                          name (substring src pos end)
                          kind (classify-symbol name)]
                      (+ (* kind 1000000) end))
                    (+ (* 99 1000000) (+ pos 1))))))))))))
(defn tokenize-spans-loop [src pos len tokens]
  (let [ws-pos (skip-ws-loop src pos len)]
    (if (>= ws-pos len)
      (vector-push (vector-push (vector-push tokens 99) ws-pos) ws-pos)
      (let [result (lex-one src ws-pos len)
            kind (/ result 1000000)
            end-pos (- result (* kind 1000000))]
        (if (== kind 99)
          (vector-push (vector-push (vector-push tokens 99) ws-pos) ws-pos)
          (tokenize-spans-loop src end-pos len
            (vector-push (vector-push (vector-push tokens kind) ws-pos) end-pos)))))))
(defn tokenize-with-spans [src]
  (tokenize-spans-loop src 0 (string-length src) (vector-new 32)))

;; --- 名前ハッシュ ---
(defn name-hash-loop [src pos end acc]
  (if (>= pos end) acc
    (name-hash-loop src (+ pos 1) end
      (+ (string-char-at src pos) (* acc 31)))))
(defn name-hash [src start end]
  (name-hash-loop src start end 0))

;; --- Parser v3 ---
(defn span-kind [spans n] (vector-get spans (* n 3)))
(defn p-current [spans pos-ref] (span-kind spans (ref-get pos-ref)))
(defn p-advance [pos-ref] (ref-set pos-ref (+ (ref-get pos-ref) 1)))
(defn p-start [spans pos-ref] (vector-get spans (+ (* (ref-get pos-ref) 3) 1)))
(defn p-end [spans pos-ref] (vector-get spans (+ (* (ref-get pos-ref) 3) 2)))
(defn p-expect [spans pos-ref expected]
  (if (== (p-current spans pos-ref) expected) (do (p-advance pos-ref) 1) 0))
(defn parse-int-from-str [src pos end acc]
  (if (>= pos end) acc
    (let [digit (- (string-char-at src pos) 48)]
      (parse-int-from-str src (+ pos 1) end (+ (* acc 10) digit)))))

(defn parse-expr-v3 [spans pos-ref src]
  (let [kind (p-current spans pos-ref)]
    (if (== kind 10)
      (let [start (p-start spans pos-ref) end-pos (p-end spans pos-ref)
            value (parse-int-from-str src start end-pos 0)]
        (do (p-advance pos-ref)
            (vector-push (vector-push (vector-new 2) 1) value)))
      (if (== kind 13) (do (p-advance pos-ref) (vector-push (vector-push (vector-new 2) 2) 1))
        (if (== kind 14) (do (p-advance pos-ref) (vector-push (vector-push (vector-new 2) 2) 0))
          (if (== kind 20)
            (let [start (p-start spans pos-ref) end-pos (p-end spans pos-ref)
                  h (name-hash src start end-pos)]
              (do (p-advance pos-ref) (vector-push (vector-push (vector-new 2) 4) h)))
            (if (== kind 0) (parse-sexp-v3 spans pos-ref src)
              (do (p-advance pos-ref) (vector-push (vector-push (vector-new 2) 1) 0)))))))))

(defn parse-sexp-v3 [spans pos-ref src]
  (do (p-advance pos-ref)
    (let [kind (p-current spans pos-ref)]
      (if (== kind 32) (parse-if-v3 spans pos-ref src)
        (if (== kind 31) (parse-let-v3 spans pos-ref src)
          (if (== kind 36) (parse-do-v3 spans pos-ref src)
            (if (== kind 30) (parse-defn-v3 spans pos-ref src)
              (parse-apply-v3 spans pos-ref src))))))))

(defn parse-if-v3 [spans pos-ref src]
  (do (p-advance pos-ref)
    (let [c (parse-expr-v3 spans pos-ref src)
          t (parse-expr-v3 spans pos-ref src)
          e (parse-expr-v3 spans pos-ref src)]
      (do (p-expect spans pos-ref 1)
          (vector-push (vector-push (vector-push (vector-push (vector-new 8) 6) c) t) e)))))

(defn parse-let-v3 [spans pos-ref src]
  (do (p-advance pos-ref) (p-expect spans pos-ref 2)
    (let [ns (p-start spans pos-ref) ne (p-end spans pos-ref)
          nh (name-hash src ns ne)]
      (do (p-advance pos-ref)
        (let [init (parse-expr-v3 spans pos-ref src)]
          (do (p-expect spans pos-ref 3)
            (let [body (parse-expr-v3 spans pos-ref src)]
              (do (p-expect spans pos-ref 1)
                (vector-push (vector-push (vector-push (vector-push (vector-new 8) 7)
                  nh) init) body)))))))))

(defn parse-do-v3 [spans pos-ref src]
  (do (p-advance pos-ref)
    (let [first (parse-expr-v3 spans pos-ref src)
          second (if (== (p-current spans pos-ref) 1) first
                   (parse-expr-v3 spans pos-ref src))]
      (do (p-advance pos-ref)
        (vector-push (vector-push (vector-push (vector-new 8) 9) first) second)))))

(defn parse-defn-v3 [spans pos-ref src]
  (do (p-advance pos-ref)
    (let [name-start (p-start spans pos-ref) name-end (p-end spans pos-ref)
          name-h (name-hash src name-start name-end)]
      (do (p-advance pos-ref) (p-expect spans pos-ref 2)
        ;; パラメータ収集
        (let [params (ref-new (vector-new 4))
              dummy (parse-params-loop spans pos-ref src params)
              body (parse-expr-v3 spans pos-ref src)]
          (do (p-expect spans pos-ref 1)
            (let [p (ref-get params)
                  n (vector-new 8)
                  n1 (vector-push (vector-push (vector-push n 20) name-h) (vector-length p))]
              ;; パラメータを追加
              (vector-push (append-params n1 p 0 (vector-length p)) body))))))))

(defn parse-params-loop [spans pos-ref src params]
  (if (== (p-current spans pos-ref) 3) ;; ]
    (do (p-advance pos-ref) 0)
    (let [s (p-start spans pos-ref) e (p-end spans pos-ref)
          h (name-hash src s e)]
      (do
        (ref-set params (vector-push (ref-get params) h))
        (p-advance pos-ref)
        (parse-params-loop spans pos-ref src params)))))

(defn append-params [node params idx len]
  (if (>= idx len) node
    (append-params (vector-push node (vector-get params idx)) params (+ idx 1) len)))

(defn parse-apply-v3 [spans pos-ref src]
  (let [func (parse-expr-v3 spans pos-ref src)
        args (ref-new (vector-new 4))
        dummy (parse-args-loop spans pos-ref src args)
        a (ref-get args)
        n (vector-push (vector-push (vector-push (vector-new 8) 5) func) (vector-length a))]
    (append-params n a 0 (vector-length a))))

(defn parse-args-loop [spans pos-ref src args]
  (if (== (p-current spans pos-ref) 1) ;; )
    (do (p-advance pos-ref) 0)
    (do
      (ref-set args (vector-push (ref-get args) (parse-expr-v3 spans pos-ref src)))
      (parse-args-loop spans pos-ref src args))))

;; --- Compiler ---
(defn emit-instr [opcode operand]
  (vector-push (vector-push (vector-new 2) opcode) operand))
(defn emit-to [instrs opcode operand]
  (vector-push instrs (emit-instr opcode operand)))
(defn env-new [] (map-new))
(defn env-bind [env key val] (map-insert env key val))
(defn env-lookup [env key] (map-get env key))
(defn builtin-opcode [name-hash]
  (if (= name-hash 43) 20
    (if (= name-hash 45) 21
      (if (= name-hash 42) 22
        (if (= name-hash 47) 23
          (if (= name-hash 61) 30
            (if (= name-hash 62) 31
              (if (= name-hash 60) 32
                0))))))))

(defn compile-expr [node env instrs]
  (let [tag (vector-get node 0)]
    (if (= tag 1) (emit-to instrs 1 (vector-get node 1))
      (if (= tag 2) (emit-to instrs 1 (vector-get node 1))
        (if (= tag 4)
          (let [key (vector-get node 1) idx (env-lookup env key)]
            (if (= idx 0) (emit-to instrs 1 0) (emit-to instrs 10 idx)))
          (if (= tag 6)
            (let [i1 (compile-expr (vector-get node 1) env instrs)
                  i2 (emit-to i1 41 0)
                  i3 (compile-expr (vector-get node 2) env i2)
                  i4 (emit-to i3 43 0)
                  i5 (compile-expr (vector-get node 3) env i4)]
              (emit-to i5 43 0))
            (if (= tag 7)
              (let [key (vector-get node 1) init (vector-get node 2) body (vector-get node 3)
                    i1 (compile-expr init env instrs)
                    new-idx (+ 1 (map-size env))
                    i2 (emit-to i1 11 new-idx)
                    new-env (env-bind env key new-idx)]
                (compile-expr body new-env i2))
              (if (= tag 5)
                (let [func (vector-get node 1)
                      argc (vector-get node 2)
                      bop (if (= (vector-get func 0) 4) (builtin-opcode (vector-get func 1)) 0)]
                  (if (> bop 0)
                    (let [i1 (compile-expr (vector-get node 3) env instrs)
                          i2 (compile-expr (vector-get node 4) env i1)]
                      (emit-to i2 bop 0))
                    ;; 非ビルトイン: print 等のランタイム関数呼出し (簡略化)
                    (emit-to instrs 1 0)))
                (emit-to instrs 1 0)))))))))

;; --- 統合パイプライン v3 ---
(defn compile-source-v3 [src]
  (let [spans (tokenize-with-spans src)
        pos-ref (ref-new 0)
        ast (parse-expr-v3 spans pos-ref src)
        ;; defn の場合: body は最後の要素
        tag (vector-get ast 0)]
    (if (= tag 20)
      ;; defn: [20, name, param-count, param1, ..., body]
      (let [param-count (vector-get ast 2)
            body-idx (+ 3 param-count)
            body (vector-get ast body-idx)
            ;; パラメータを環境に登録
            env (ref-new (env-new))
            idx (ref-new 1)
            dummy (register-params ast env idx 0 param-count)]
        (compile-expr body (ref-get env) (vector-new 16)))
      ;; 式: そのままコンパイル
      (compile-expr ast (env-new) (vector-new 16)))))

(defn register-params [ast env-ref idx-ref i count]
  (if (>= i count) 0
    (do
      (ref-set env-ref (env-bind (ref-get env-ref) (vector-get ast (+ 3 i)) (ref-get idx-ref)))
      (ref-set idx-ref (+ (ref-get idx-ref) 1))
      (register-params ast env-ref idx-ref (+ i 1) count))))

;; === テスト ===
(defn main []
  (do
    ;; テスト1: (defn main [] 42) → IR: [i64.const 42]
    (let [ir1 (compile-source-v3 "(defn main [] 42)")
          len1 (vector-length ir1)]
      (do
        (print len1)  ;; 1
        (let [instr (vector-get ir1 0)]
          (do
            (print (vector-get instr 0))   ;; 1 (i64.const)
            (print (vector-get instr 1))   ;; 42
            0))))

    ;; テスト2: (defn f [x] (+ x 1)) → IR: [local.get, i64.const, i64.add]
    (let [ir2 (compile-source-v3 "(defn f [x] (+ x 1))")
          len2 (vector-length ir2)]
      (do
        (print len2)  ;; 3
        0))

    ;; テスト3: (if (> 10 5) 42 0) → IR with if/end
    (let [ir3 (compile-source-v3 "(if (> 10 5) 42 0)")
          len3 (vector-length ir3)]
      (do
        (print len3)  ;; > 0
        0))

    0))
"#;
    let result = compile_and_run(source);
    let lines: Vec<&str> = result.trim().lines().collect();
    assert!(lines.len() >= 5, "最低5行の出力");
    assert_eq!(lines[0], "1", "defn main [] 42 → IR 命令数 1");
    assert_eq!(lines[1], "1", "i64.const opcode");
    assert_eq!(lines[2], "42", "i64.const operand = 42");
    assert_eq!(lines[3], "3", "defn f [x] (+ x 1) → IR 命令数 3");
    assert!(
        lines[4].parse::<i32>().unwrap() > 0,
        "if 式 → IR 命令数 > 0"
    );
}

// === MacroExpand Tests ===

/// selfhost MacroExpand.ls テスト: defmacro 基本登録
#[test]
fn test_e2e_selfhost_macro_defmacro_register() {
    // selfhost compiler で defmacro を含むソースをコンパイルし、
    // マクロが登録されることを検証する
    // 期待値: defmacro 認識後にマクロテーブルに登録
    let source = r#"
(module Main)
(defmacro my-const [] 42)
(defn main [] (print (my-const)))
"#;
    let result = compile_and_run_expanded(source);
    assert_eq!(result.trim(), "42");
}

/// selfhost MacroExpand.ls テスト: 引数付きマクロ展開
#[test]
fn test_e2e_selfhost_macro_defmacro_with_args() {
    // 引数付きマクロの展開が正しく動作することを検証
    // 期待値: (double 21) → (+ 21 21) → 42
    let source = r#"
(module Main)
(defmacro double [x] '(+ ~x ~x))
(defn main [] (print (double 21)))
"#;
    let result = compile_and_run_expanded(source);
    assert_eq!(result.trim(), "42");
}

/// selfhost MacroExpand.ls テスト: quasiquote 基本
#[test]
fn test_e2e_selfhost_macro_quasiquote_basic() {
    // quasiquote/unquote を使ったマクロ展開の検証
    // 期待値: マクロ展開後にリテラル値が正しく埋め込まれる
    let source = r#"
(module Main)
(defmacro make-add [a b] '(+ ~a ~b))
(defn main [] (print (make-add 20 22)))
"#;
    let result = compile_and_run_expanded(source);
    assert_eq!(result.trim(), "42");
}

/// selfhost MacroExpand.ls テスト: AST 再構成
#[test]
fn test_e2e_selfhost_macro_ast_reconstruction() {
    // マクロ展開結果が有効な AST として再構成され、
    // 後続の型推論・コンパイルが成功することを検証
    // 期待値: マクロ展開 → let 束縛 → 正しい計算結果
    let source = r#"
(module Main)
(defmacro with-temp [body] '(let [tmp 42] ~body))
(defn main [] (with-temp (print tmp)))
"#;
    let result = compile_and_run_expanded(source);
    assert_eq!(result.trim(), "42");
}

/// selfhost MacroExpand.ls テスト: ネストされたマクロ
#[test]
fn test_e2e_selfhost_macro_nested_expansion() {
    // マクロ内でマクロを使用した場合の再帰展開を検証
    // 期待値: 内側マクロ展開 → 外側マクロ展開 → 正しい結果
    let source = r#"
(module Main)
(defmacro add1 [x] '(+ ~x 1))
(defmacro add2 [x] '(add1 (add1 ~x)))
(defn main [] (print (add2 40)))
"#;
    let result = compile_and_run_expanded(source);
    assert_eq!(result.trim(), "42");
}

// === TypeInfer Tests ===

/// selfhost TypeInfer.ls テスト: リテラル型推論
#[test]
#[ignore]
fn test_e2e_selfhost_typeinfer_literal() {
    // selfhost compiler でリテラルの型推論が動作することを検証
    // 期待値: Int リテラルが正しく型付けされ実行可能
    let source = r#"
(module Main)
(defn main [] (print 42))
"#;
    // selfhost パイプラインで compile & run
    // TypeInfer.ls が型推論を行い、正しく型付けされた AST を返す
    let result = compile_and_run(source);
    assert_eq!(result.trim(), "42");
}

/// selfhost TypeInfer.ls テスト: float / unit リテラル型推論
#[test]
#[ignore]
fn test_e2e_selfhost_typeinfer_float_and_unit_literals() {
    let harness = r#"
(defn main []
  (let [counter (make-var-counter)
        env (init-builtin-env counter)
        float-node (make-lit-float 0 4)
        unit-node (make-lit-unit)
        float-result (infer-expr float-node env (subst-new) counter)
        unit-result (infer-expr unit-node env (subst-new) counter)]
    (do
      (print (result-failed float-result))
      (print (ty-tag (result-type float-result)))
      (print (ty-name (result-type float-result)))
      (print (result-failed unit-result))
      (print (ty-tag (result-type unit-result)))
      (print (ty-name (result-type unit-result)))
      0)))
"#;

    let combined = format!("{}\n{}", selfhost_typeinfer_runtime_bundle(), harness);
    let output = compile_and_run(&combined);
    let lines: Vec<&str> = output.trim().lines().collect();

    assert!(
        lines.len() >= 6,
        "float/unit typeinfer 出力が不足: {:?}",
        lines
    );
    assert_eq!(lines[0], "0", "float infer は失敗すべきでない");
    assert_eq!(lines[1], "1", "float infer の型タグは Con であるべき");
    assert_eq!(
        lines[2], "400",
        "float infer の型名は Float hash=400 であるべき"
    );
    assert_eq!(lines[3], "0", "unit infer は失敗すべきでない");
    assert_eq!(lines[4], "1", "unit infer の型タグは Con であるべき");
    assert_eq!(
        lines[5], "500",
        "unit infer の型名は Unit hash=500 であるべき"
    );
}
