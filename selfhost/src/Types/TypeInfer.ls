(module Types.TypeInfer)
(import Syntax.AST)
(import Types.Type)
(import Types.TypeScheme)
(import Types.TypeInferCore)
(import Types.TypeInferFunctions)
(import Types.TypeInferBuiltins)

;; TypeInfer.ls - L# セルフホスティング: Hindley-Milner 型推論
;;
;; Type.ls (型定義・単一化・代入) と TypeScheme.ls (汎化・具体化) を使い、
;; AST ノードに対して型推論を行う。
;;
;; バンドルモードでは以下のサブモジュールが実装を上書きする:
;;   TypeInferApply.ls   - lambda 式・関数適用の型推論
;;   TypeInferBlock.ls   - let 式・do ブロック・computation 式の型推論
;;   TypeInferPattern.ls - パターンマッチの型推論
;;   TypeInferRecord.ls  - レコード型の型推論
;;
;; 依存: Type.ls, TypeScheme.ls, AST.ls
;;
;; 型環境 (TypeEnv) = HashMap<name-hash, TypeScheme>
;; 推論結果 = [subst, type] (Vector of 2 要素)

;; ============================================================
;; infer-expr: AST ノードの型推論
;; ============================================================
;; 引数:
;;   node    - AST ノード (Vector)
;;   env     - 型環境 (HashMap<name-hash, TypeScheme>)
;;   subst   - 現在の置換 (HashMap<var-id, Type>)
;;   counter - 型変数カウンタ (ref-cell)
;; 戻り値:
;;   [subst, type, error-code] - 更新された置換と推論された型

;; リテラルの型推論
(defn infer-lit [node]
  (let [tag (vector-get node 0)]
    (if (= tag 1)
      (mk-int)
      (if (= tag 2)
        (mk-bool)
        (if (= tag 3)
          (mk-string)
          (if (= tag 19)
            (mk-float)
            (if (= tag 32)
              (mk-unit)
              ;; 不明なリテラル -> Int にフォールバック
              (mk-int))))))))

;; 変数参照の型推論
(defn infer-var [node env subst counter]
  (let [name-hash (vector-get node 1)
    scheme (type-env-lookup env name-hash)]
    (if (= scheme 0)
      ;; 未定義変数: エラー
      (make-error-result-code (error-code-undefined))
      ;; 型スキームを具体化
      (let [ty (apply-subst subst (instantiate scheme counter))]
        (make-result subst ty)))))

;; if 式の型推論
;; [6, cond, then, else]
(defn infer-if [node env subst counter]
  (let [cond-node (vector-get node 1)
    then-node (vector-get node 2)
    else-node (vector-get node 3)
    ;; 条件式を推論
    cond-result (infer-expr cond-node env subst counter)]
    (if (= (result-failed cond-result) 1)
      (make-error-result-code (result-error-code cond-result))
      (let [s1 (result-subst cond-result)
        cond-ty (result-type cond-result)
        ;; 条件式は Bool であること
        s2 (unify cond-ty (mk-bool) s1)]
        (if (= (unify-failed s2) 1)
          (make-error-result-code (error-code-if-cond))
          ;; then 枝を推論
          (let [then-result (infer-expr then-node env s2 counter)]
            (if (= (result-failed then-result) 1)
              (make-error-result-code (result-error-code then-result))
              (let [s3 (result-subst then-result)
                then-ty (result-type then-result)
                ;; else 枝を推論
                else-result (infer-expr else-node env s3 counter)]
                (if (= (result-failed else-result) 1)
                  (make-error-result-code (result-error-code else-result))
                  ;; then と else の型を統一
                  (let [s4 (result-subst else-result)
                    else-ty (result-type else-result)
                    s5 (unify (apply-subst s4 then-ty) else-ty s4)]
                    (if (= (unify-failed s5) 1)
                      (make-error-result-code (error-code-if-branch))
                      (make-result s5 (apply-subst s5 else-ty)))))))))))))

;; ann 式の型推論
;; [11, expr, raw-type-expr]。旧 AST の payload 不在は内側の式をそのまま返す。
(defn infer-ann [node env subst counter]
  (let [expr-result (infer-expr (vector-get node 1) env subst counter)]
    (if (= (result-failed expr-result) 1)
      (propagate-error-result expr-result)
      (if (<= (vector-length node) 2)
        expr-result
        (let [type-expr (vector-get node 2)]
          (if (= type-expr 0)
            expr-result
            (let [s1 (result-subst expr-result)
              expr-ty (result-type expr-result)
              ann-ty (typeinfer-resolve-type-expr type-expr)
              s2 (unify expr-ty ann-ty s1)]
              (if (= (unify-failed s2) 1)
                (make-error-result-code (error-code-general))
                (make-result s2 (apply-subst s2 ann-ty))))))))))

;; quote/unquote 系は現状すべて inner expr へ委譲する
(defn quote-like-tag? [tag]
  (if (= tag (tag-quote))
    1
    (if (= tag (tag-unquote))
      1
      (if (= tag (tag-unquote-splice))
        1
        0))))

;; ============================================================
;; スタブ定義 (バンドルモードではサブモジュールが上書き)
;; ============================================================
;; マルチファイルコンパイル時の型検査を通すためのフォールバック実装。
;; バンドル (連結) モードでは TypeInferApply/Block/Pattern/Record が
;; これらを完全な実装で上書きする。

;; --- Apply グループ (TypeInferApply.ls が上書き) ---
(defn infer-lambda [node env subst counter]
  (make-result subst (fresh-type-var counter)))
(defn infer-apply [node env subst counter]
  (make-result subst (fresh-type-var counter)))

;; --- Block グループ (TypeInferBlock.ls が上書き) ---
(defn infer-let [node env subst counter]
  (make-result subst (fresh-type-var counter)))
(defn infer-computation-steps [node idx step-count env subst counter last-ty]
  (make-result subst last-ty))
(defn infer-computation [node env subst counter]
  (make-result subst (mk-int)))
(defn infer-do [node env subst counter]
  (make-result subst (mk-int)))

;; --- Pattern グループ (TypeInferPattern.ls が上書き) ---
(defn pattern-children-subst [r]
  (vector-get r 0))
(defn pattern-children-env [r]
  (vector-get r 1))
(defn infer-pattern-children [node idx count base-index stride env subst counter]
  (vector-push (vector-push (vector-new 2) subst) env))
(defn infer-constructor-pattern-children [node idx count env subst counter ctor-ty]
  (vector-push (make-result subst (mk-int)) env))
(defn infer-pattern [pat env subst counter]
  (vector-push (make-result subst (fresh-type-var counter)) env))
(defn pat-result-subst [r]
  (vector-get r 0))
(defn pat-result-type [r]
  (vector-get r 1))
(defn pat-result-env [r]
  (vector-get r 3))
(defn infer-match-arms [node idx arm-count env scrut-ty result-ty subst counter]
  (make-result subst result-ty))
(defn infer-match [node env subst counter]
  (make-result subst (fresh-type-var counter)))

;; --- Record グループ (TypeInferRecord.ls が上書き) ---
(defn infer-record-fields [node idx count env subst counter]
  (make-result subst (mk-int)))
(defn infer-recordlit-fields [node idx count env subst counter record-ty]
  (make-result subst record-ty))
(defn recordlit-field-node-loop [record-node field-name-hash idx field-count]
  0)
(defn recordlit-field-node [record-node field-name-hash]
  0)
(defn infer-recordlit [node env subst counter]
  (make-result subst (mk-int)))
(defn infer-fieldaccess [node env subst counter]
  (make-result subst (fresh-type-var counter)))
(defn infer-recordupdate-node [node env subst counter]
  (make-result subst (fresh-type-var counter)))

;; ============================================================
;; infer-expr: メインディスパッチ
;; ============================================================

(defn infer-expr [node env subst counter]
  (let [tag (vector-get node 0)]
    (if (= tag 1)
      ;; 整数リテラル
      (make-result subst (mk-int))
      (if (= tag 2)
        ;; 真偽値リテラル
        (make-result subst (mk-bool))
        (if (= tag 3)
          ;; 文字列リテラル
          (make-result subst (mk-string))
          (if (= tag 19)
            ;; 浮動小数点リテラル
            (make-result subst (mk-float))
            (if (= tag 32)
              ;; unit リテラル
              (make-result subst (mk-unit))
              (if (= tag 4)
                ;; 変数参照
                (infer-var node env subst counter)
                (if (= tag 5)
                  ;; 関数適用
                  (infer-apply node env subst counter)
                  (if (= tag 6)
                    ;; if 式
                    (infer-if node env subst counter)
                    (if (= tag 7)
                      ;; let 式
                      (infer-let node env subst counter)
                      (if (= tag (tag-ann))
                        ;; ann 式
                        (infer-ann node env subst counter)
                        (if (= (quote-like-tag? tag) 1)
                          ;; quote / unquote / unquote-splice
                          (infer-ann node env subst counter)
                          (if (= tag (tag-recordlit))
                            ;; record literal
                            (infer-recordlit node env subst counter)
                            (if (= tag (tag-fieldaccess))
                              ;; field access
                              (infer-fieldaccess node env subst counter)
                              (if (= tag (tag-recordupdate))
                                ;; record update
                                (infer-recordupdate-node node env subst counter)
                                (if (= tag (tag-computation))
                                  ;; computation 式
                                  (infer-computation node env subst counter)
                                  (if (= tag 8)
                                    ;; lambda 式
                                    (infer-lambda node env subst counter)
                                    (if (= tag 9)
                                      ;; do ブロック
                                      (infer-do node env subst counter)
                                      (if (= tag 10)
                                        ;; match 式
                                        (infer-match node env subst counter)
                                        ;; 未知のノード: エラー
                                        (make-error-result)))))))))))))))))))))

;; ============================================================
;; infer-defn: トップレベル関数定義の型推論
;; ============================================================
;; [20, name-hash, param-count, param-hash1, ..., body, signature?]
;; signature は [65, param-count, param-type-expr..., return-type-expr]。
;; compile-safe な covered slice として 0/1/2/3/4 引数を扱う

(defn infer-defn-predeclared [node body-env final-env counter subst placeholder env-vars]
  (let [name-hash (vector-get node 1)
    param-count (vector-get node 2)]
    (if (= param-count 0)
      (let [body-node (vector-get node 3)
        result (infer-expr body-node body-env subst counter)]
        (if (= (result-failed result) 1)
          (propagate-error-result result)
          (let [s (result-subst result)
            body-ty (result-type result)
            annotated-subst (typeinfer-defn-return-annotation-subst node param-count body-ty s)]
            (if (= (unify-failed annotated-subst) 1)
              (make-error-result-code (error-code-general))
              (let [next-subst (unify placeholder body-ty annotated-subst)]
                (if (= (unify-failed next-subst) 1)
                  (make-error-result-code (error-code-general))
                  (typeinfer-finalize-defn-result-with-env-vars final-env name-hash next-subst body-ty env-vars)))))))
      (let [param-types (typeinfer-fresh-param-types param-count counter)
        body-node (vector-get node (+ param-count 3))
        next-env (typeinfer-extend-env-with-node-params body-env node param-count 3 param-types)
        annotated-param-subst (typeinfer-defn-param-annotation-subst node param-count param-types subst)]
        (if (= (unify-failed annotated-param-subst) 1)
          (make-error-result-code (error-code-general))
          (let [result (infer-expr body-node next-env annotated-param-subst counter)]
            (if (= (result-failed result) 1)
              (propagate-error-result result)
              (let [s (result-subst result)
                body-ty (result-type result)
                annotated-subst (typeinfer-defn-return-annotation-subst node param-count body-ty s)]
                (if (= (unify-failed annotated-subst) 1)
                  (make-error-result-code (error-code-general))
                  (let [fun-ty (typeinfer-build-curried-fun param-types annotated-subst body-ty)
                    next-subst (unify placeholder fun-ty annotated-subst)]
                    (if (= (unify-failed next-subst) 1)
                      (make-error-result-code (error-code-general))
                      (typeinfer-finalize-defn-result-with-env-vars final-env name-hash next-subst fun-ty env-vars))))))))))))

;; 単独で呼ばれる infer-defn も自己再帰を許可する。
(defn infer-defn [node env counter]
  (let [name-hash (vector-get node 1)
    placeholder (fresh-type-var counter)
    body-env (type-env-insert env name-hash (mono placeholder))]
    (infer-defn-predeclared node body-env env counter (subst-new) placeholder (map-new))))

;; 型変数ベクタを一般化除外用の Set へ移す。
(defn typeinfer-free-vars-to-set [vars idx len env-vars]
  (if (>= idx len)
    env-vars
    (typeinfer-free-vars-to-set
      vars
      (+ idx 1)
      len
      (map-insert-int-safe env-vars (vector-get vars idx) 1))))

;; 後続 top-level defn の placeholder に残る自由型変数を集める。
(defn typeinfer-pending-env-vars-loop [program idx len placeholders subst env-vars]
  (if (>= idx len)
    env-vars
    (let [decl (vector-get program idx)
      tag (vector-get decl 0)]
      (if (= tag 20)
        (let [name-hash (vector-get decl 1)
          placeholder (map-get-safe placeholders name-hash)
          resolved-placeholder (apply-subst subst placeholder)
          free (free-vars resolved-placeholder)
          next-env-vars (typeinfer-free-vars-to-set free 0 (vector-length free) env-vars)]
          (typeinfer-pending-env-vars-loop program (+ idx 1) len placeholders subst next-env-vars))
        (typeinfer-pending-env-vars-loop program (+ idx 1) len placeholders subst env-vars)))))

(defn typeinfer-pending-env-vars [program next-idx len placeholders subst]
  (typeinfer-pending-env-vars-loop program next-idx len placeholders subst (map-new)))

;; program 内の top-level defn 名を単相 placeholder として先に登録する。
(defn typeinfer-predeclare-defns-loop [program idx len env placeholders counter]
  (if (>= idx len)
    (push-object-vector-local (push-object-vector-local (vector-new 2) env) placeholders)
    (let [decl (vector-get program idx)
      tag (vector-get decl 0)]
      (if (= tag 20)
        (let [name-hash (vector-get decl 1)
          placeholder (fresh-type-var counter)
          next-env (type-env-insert env name-hash (mono placeholder))
          next-placeholders (map-insert-object-safe placeholders name-hash placeholder)]
          (typeinfer-predeclare-defns-loop program (+ idx 1) len next-env next-placeholders counter))
        (typeinfer-predeclare-defns-loop program (+ idx 1) len env placeholders counter)))))

(defn typeinfer-predeclare-defns [program env counter]
  (typeinfer-predeclare-defns-loop program 0 (vector-length program) env (map-new) counter))

;; program 推論の状態: [env, subst, first-type, first-seen, diagnostic-count, first-error-code]
(defn typeinfer-program-analysis-state [env subst first-ty first-seen diagnostic-count first-error-code]
  (push-int-vector-local
    (push-int-vector-local
      (push-int-vector-local
        (push-object-vector-local
          (push-object-vector-local
            (push-object-vector-local (vector-new 6) env)
            subst)
          first-ty)
        first-seen)
      diagnostic-count)
    first-error-code))

(defn infer-program-analysis-env [analysis] (vector-get analysis 0))
(defn infer-program-analysis-subst [analysis] (vector-get analysis 1))
(defn infer-program-analysis-raw-type [analysis] (vector-get analysis 2))
(defn infer-program-analysis-first-seen [analysis] (vector-get analysis 3))
(defn infer-program-analysis-diagnostic-count [analysis] (vector-get analysis 4))
(defn infer-program-analysis-first-error-code [analysis] (vector-get analysis 5))

(defn infer-program-analysis-type [analysis]
  (apply-subst (infer-program-analysis-subst analysis) (infer-program-analysis-raw-type analysis)))

(defn typeinfer-program-analysis-loop [program idx len placeholders counter state]
  (if (>= idx len)
    state
    (let [decl (vector-get program idx)
      tag (vector-get decl 0)]
      (if (= tag 20)
        (let [env (infer-program-analysis-env state)
          subst (infer-program-analysis-subst state)
          first-ty (infer-program-analysis-raw-type state)
          first-seen (infer-program-analysis-first-seen state)
          diagnostic-count (infer-program-analysis-diagnostic-count state)
          first-error-code (infer-program-analysis-first-error-code state)
          name-hash (vector-get decl 1)
          placeholder (map-get-safe placeholders name-hash)
          pending-env-vars (typeinfer-pending-env-vars program (+ idx 1) len placeholders subst)
          out (infer-defn-predeclared decl env env counter subst placeholder pending-env-vars)]
          (if (= (result-failed out) 1)
            (let [next-first-ty (if (= first-seen 0) (result-type out) first-ty)
              next-first-seen (if (= first-seen 0) 1 first-seen)
              next-first-error-code (if (= first-error-code 0) (result-error-code out) first-error-code)
              next-state (typeinfer-program-analysis-state (type-env-remove env name-hash) subst next-first-ty next-first-seen (+ diagnostic-count 1) next-first-error-code)]
              (typeinfer-program-analysis-loop program (+ idx 1) len placeholders counter next-state))
            (let [next-first-ty (if (= first-seen 0) (result-type out) first-ty)
              next-first-seen (if (= first-seen 0) 1 first-seen)
              next-env (vector-get out 3)
              next-subst (result-subst out)
              next-state (typeinfer-program-analysis-state next-env next-subst next-first-ty next-first-seen diagnostic-count first-error-code)]
              (typeinfer-program-analysis-loop program (+ idx 1) len placeholders counter next-state))))
        (typeinfer-program-analysis-loop program (+ idx 1) len placeholders counter state)))))

;; top-level defn を先行登録して一度だけ推論し、CLI/LSP が共有する結果を返す。
(defn infer-program-analysis [program]
  (let [counter (make-var-counter)
    initial-env (init-builtin-env counter)
    predeclared (typeinfer-predeclare-defns program initial-env counter)
    env (vector-get predeclared 0)
    placeholders (vector-get predeclared 1)
    state (typeinfer-program-analysis-state env (subst-new) (mk-int) 0 0 0)]
    (typeinfer-program-analysis-loop program 0 (vector-length program) placeholders counter state)))

;; ============================================================
;; infer: 公開 API (Main.ls から呼び出される)
;; ============================================================

(defn infer [program]
  (infer-program-analysis-type (infer-program-analysis program)))

;; ============================================================
;; ビルトイン型環境の初期化
;; ============================================================

;; builtin env 本体は Types.TypeInferBuiltins へ分離
(defn init-builtin-env [counter]
  (typeinfer-init-builtin-env counter))
