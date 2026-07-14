(module Types.TypeInferPattern)
(import Syntax.AST)
(import Types.Type)
(import Types.TypeScheme)
(import Types.TypeInferCore)

;; TypeInferPattern.ls - パターンマッチの型推論
;;
;; パターン種別:
;;   1 = リテラル整数パターン
;;   2 = リテラル真偽値パターン
;;   3 = リテラル文字列パターン
;;   4 = 変数パターン (ワイルドカード含む)
;;   11 = コンストラクタパターン (tag-pattern)
;;   12 = レコードパターン
;;
;; 引数:
;;   pat     - パターンノード [tag, ...]
;;   env     - 型環境
;;   subst   - 現在の置換
;;   counter - 型変数カウンタ
;; 戻り値:
;;   [subst, type, updated-env] - 更新された置換、パターンの型、束縛追加後の環境

(defn pattern-children-subst [r]
  (vector-get r 0))

(defn pattern-children-env [r]
  (vector-get r 1))

;; subpattern 群を左から処理して binder env を積み上げる
;; base-index + idx * stride が subpattern の位置
(defn infer-pattern-children [node idx count base-index stride env subst counter]
  (if (>= idx count)
    (vector-push (vector-push (vector-new 2) subst) env)
    (let [child (vector-get node (+ base-index (* idx stride)))
      child-info (infer-pattern child env subst counter)
      child-subst (pat-result-subst child-info)
      child-env (pat-result-env child-info)]
      (if (= (map-get child-subst -1) 1)
        (vector-push
          (vector-push
            (vector-new 2)
            (map-insert child-subst -2 (result-error-code child-info)))
          child-env)
        (infer-pattern-children
          node
          (+ idx 1)
          count
          base-index
          stride
          child-env
          child-subst
          counter)))))

;; constructor pattern の subpattern を左から処理し、
;; コンストラクタ引数型との unify を行って最終戻り型を返す
(defn infer-constructor-pattern-children [node idx count env subst counter ctor-ty]
  (let [current-ctor (apply-subst subst ctor-ty)]
    (if (>= idx count)
      (if (= (ty-tag current-ctor) (ty-fun))
        (vector-push (make-error-result-code (error-code-general)) env)
        (vector-push (make-result subst current-ctor) env))
      (if (= (ty-tag current-ctor) (ty-fun))
        (let [child (vector-get node (+ 3 idx))
          child-info (infer-pattern child env subst counter)
          child-subst (result-subst child-info)
          child-ty (result-type child-info)
          child-env (vector-get child-info 3)]
          (if (= (result-failed child-info) 1)
            child-info
            (let [next-ctor (apply-subst child-subst current-ctor)
              param-ty (ty-fp next-ctor)
              ret-ty (ty-fr next-ctor)
              s2 (unify child-ty param-ty child-subst)]
              (if (= (unify-failed s2) 1)
                (vector-push (make-error-result-code (error-code-general)) child-env)
                (infer-constructor-pattern-children
                  node
                  (+ idx 1)
                  count
                  child-env
                  s2
                  counter
                  ret-ty)))))
        (vector-push (make-error-result-code (error-code-general)) env))))
)

;; canonical record pattern は既存の field 配置を保ったまま末尾に type 名 hash を持つ。
;; 旧手組み AST（type 名なし）は 0 を返し、従来の shallow fallback を維持する。
(defn record-pattern-type-hash [pat]
  (let [field-count (vector-get pat 1)
    type-slot (+ 2 (* field-count 2))]
    (if (> (vector-length pat) type-slot)
      (vector-get pat type-slot)
      0)))

;; schema がある record pattern の field を左から推論し、schema field 型と unify する。
(defn infer-record-pattern-schema-children [node idx count env subst counter record-ty]
  (if (>= idx count)
    (vector-push (make-result subst record-ty) env)
    (let [field-offset (+ 2 (* idx 2))
      field-hash (vector-get node field-offset)
      child (vector-get node (+ field-offset 1))
      expected-ty (type-record-field-type record-ty field-hash)]
      (if (= expected-ty 0)
        (vector-push (make-error-result-code (error-code-general)) env)
        (let [child-info (infer-pattern child env subst counter)]
          (if (= (result-failed child-info) 1)
            child-info
            (let [child-subst (result-subst child-info)
              child-ty (result-type child-info)
              child-env (vector-get child-info 3)
              next-subst (unify (apply-subst child-subst expected-ty) child-ty child-subst)]
              (if (= (unify-failed next-subst) 1)
                (vector-push (make-error-result-code (error-code-general)) child-env)
                (infer-record-pattern-schema-children
                  node
                  (+ idx 1)
                  count
                  child-env
                  next-subst
                  counter
                  record-ty)))))))))

(defn infer-pattern [pat env subst counter]
  (let [tag (vector-get pat 0)]
    (if (= tag 1)
      ;; 整数リテラルパターン: 型は Int、環境変化なし
      (vector-push (make-result subst (mk-int)) env)
      (if (= tag 2)
        ;; 真偽値リテラルパターン: 型は Bool、環境変化なし
        (vector-push (make-result subst (mk-bool)) env)
        (if (= tag 3)
          ;; 文字列リテラルパターン: 型は String、環境変化なし
          (vector-push (make-result subst (mk-string)) env)
          (if (= tag 4)
            ;; legacy な変数パターン: 新しい型変数を割り当て
            (let [name-hash (vector-get pat 1)
              ty (fresh-type-var counter)
              scheme (mono ty)
              new-env (type-env-insert env name-hash scheme)]
              (vector-push (make-result subst ty) new-env))
            (if (= tag 40)
              ;; canonical なワイルドカードパターン: fresh var だけ返し、束縛は追加しない
              (let [ty (fresh-type-var counter)]
                (vector-push (make-result subst ty) env))
              (if (= tag 41)
                ;; canonical な変数パターン: 新しい型変数を割り当て
                (let [name-hash (vector-get pat 1)
                  ty (fresh-type-var counter)
                  scheme (mono ty)
                  new-env (type-env-insert env name-hash scheme)]
                  (vector-push (make-result subst ty) new-env))
                (if (= tag 42)
                  ;; canonical なリテラルパターン: [42, lit-node]
                  (let [lit-node (vector-get pat 1)
                    lit-tag (vector-get lit-node 0)]
                    (if (= lit-tag 1)
                      (vector-push (make-result subst (mk-int)) env)
                      (if (= lit-tag 2)
                        (vector-push (make-result subst (mk-bool)) env)
                        (if (= lit-tag 32)
                          (vector-push (make-result subst (mk-unit)) env)
                          (if (= lit-tag 3)
                            (vector-push (make-result subst (mk-string)) env)
                            (let [ty (fresh-type-var counter)]
                              (vector-push (make-result subst ty) env)))))))
                  (if (or (= tag 11) (= tag 43))
                    ;; コンストラクタパターン (tag-pattern / constructor-pattern)
                    ;; [11, ctor-name-hash, sub-pat-count, sub-pat1, ...]
                    (let [ctor-hash (vector-get pat 1)
                      ctor-scheme (type-env-lookup env ctor-hash)]
                      (if (= ctor-scheme 0)
                        ;; 未定義コンストラクタ: エラー
                        (vector-push
                          (make-error-result-code (error-code-undefined))
                          env)
                        (let [sub-count (vector-get pat 2)
                          ctor-ty (instantiate ctor-scheme counter)]
                          (infer-constructor-pattern-children
                            pat
                            0
                            sub-count
                            env
                            subst
                            counter
                            ctor-ty))))
                    (if (or (= tag 12) (= tag 44))
                      ;; レコードパターン
                      ;; [12/44, field-count, field-hash1, sub-pat1, ..., type-name-hash?]
                      (let [fc (vector-get pat 1)
                        type-hash (if (= tag 44) (record-pattern-type-hash pat) 0)
                        record-env (var-counter-record-env counter)
                        record-schema (if (= type-hash 0) 0 (map-get-safe record-env type-hash))]
                        (if (= type-hash 0)
                          (let [child-info
                                (infer-pattern-children
                                  pat 0 fc 3 2 env subst counter)
                            child-subst (pattern-children-subst child-info)
                            child-env (pattern-children-env child-info)]
                            (if (= (map-get child-subst -1) 1)
                              (vector-push
                                (make-error-result-code (map-get child-subst -2))
                                child-env)
                              ;; 旧 AST は record schema を持たないため fresh var のまま扱う。
                              (let [ty (fresh-type-var counter)]
                                (vector-push (make-result child-subst ty) child-env))))
                          (if (= record-schema 0)
                            ;; parser が保持した record 名が未登録なら未定義 record として拒否する。
                            (vector-push
                              (make-error-result-code (error-code-undefined))
                              env)
                            (let [record-ty (instantiate record-schema counter)]
                              (infer-record-pattern-schema-children
                                pat
                                0
                                fc
                                env
                                subst
                                counter
                                record-ty)))))
                      ;; 未知のパターン: 新しい型変数 (ワイルドカード扱い)
                      (let [ty (fresh-type-var counter)]
                        (vector-push (make-result subst ty) env)))))))))))))

;; infer-pattern の戻り値アクセサ
;; [subst, type, updated-env]
(defn pat-result-subst [r]
  (vector-get r 0))

(defn pat-result-type [r]
  (vector-get r 1))

(defn pat-result-env [r]
  (vector-get r 3))

;; match 式の型推論
;; [10, scrutinee, arm-count, pat1, body1, pat2, body2, ...]
;; binder は各 arm body にだけ見え、次の arm には漏らさない
(defn infer-match-arms [node idx arm-count env scrut-ty result-ty subst counter]
  (if (>= idx arm-count)
    (make-result subst (apply-subst subst result-ty))
    (let [pat (vector-get node (+ 3 (* idx 2)))
      body (vector-get node (+ 4 (* idx 2)))
      pat-info (infer-pattern pat env subst counter)
      pat-subst (pat-result-subst pat-info)
      pat-ty (pat-result-type pat-info)
      pat-env (pat-result-env pat-info)]
      (if (= (map-get pat-subst -1) 1)
        (propagate-error-result pat-info)
        (let [s2 (unify (apply-subst pat-subst scrut-ty) pat-ty pat-subst)]
          (if (= (unify-failed s2) 1)
            (make-error-result-code (error-code-general))
            (let [body-result (infer-expr body pat-env s2 counter)]
              (if (= (result-failed body-result) 1)
                (propagate-error-result body-result)
                (let [s3 (result-subst body-result)
                  body-ty (result-type body-result)
                  s4 (unify (apply-subst s3 result-ty) body-ty s3)]
                  (if (= (unify-failed s4) 1)
                    (make-error-result-code (error-code-general))
                    (infer-match-arms
                      node
                      (+ idx 1)
                      arm-count
                      env
                      scrut-ty
                      result-ty
                      s4
                      counter)))))))))))

(defn infer-match [node env subst counter]
  (let [scrutinee (vector-get node 1)
    arm-count (vector-get node 2)
    scrut-result (infer-expr scrutinee env subst counter)]
    (if (= (result-failed scrut-result) 1)
      (propagate-error-result scrut-result)
      (let [s1 (result-subst scrut-result)
        scrut-ty (result-type scrut-result)
        result-ty (fresh-type-var counter)]
        (infer-match-arms node 0 arm-count env scrut-ty result-ty s1 counter)))))
