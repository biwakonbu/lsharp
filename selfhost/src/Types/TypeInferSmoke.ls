(module Types.TypeInferSmoke)
(import Types.Type)
(import Types.TypeScheme)
(import Types.TypeInferCore)
(import Types.TypeInferBuiltins)
(import Types.TypeInfer)
(import Types.TypeInferApply)
(import Types.TypeInferBlock)
(import Types.TypeInferPattern)
(import Types.TypeInferRecord)

;; TypeInfer の test-only smoke entrypoint
;; 推論本体とは分離し、連結実行では最後の main として使う。
;; 上書き実装 4 本を明示 import しているので module-graph 経路でも正しく link される
;; (I-102 / decisions-selfhost-typeinfer-stub-override.md の決定 3)。

(defn main []
  (let [counter (make-var-counter)
    env (init-builtin-env counter)]
    (do
      ;; テスト 1: 整数リテラル -> Int
      (let [lit (vector-push (vector-push (vector-new 2) 1) 42)
        r1 (infer-expr lit env (subst-new) counter)]
        (do
          (print (result-failed r1))
          (print (ty-tag (result-type r1)))
          (print (ty-name (result-type r1)))))

      ;; テスト 2: 真偽値リテラル -> Bool
      (let [bool-lit (vector-push (vector-push (vector-new 2) 2) 1)
        r2 (infer-expr bool-lit env (subst-new) counter)]
        (do
          (print (ty-tag (result-type r2)))
          (print (ty-name (result-type r2)))))

      ;; テスト 3: if 式 -> then/else の型が一致
      (let [cond-node (vector-push (vector-push (vector-new 2) 2) 1)
        then-node (vector-push (vector-push (vector-new 2) 1) 42)
        else-node (vector-push (vector-push (vector-new 2) 1) 0)
        if-node (vector-push (vector-push (vector-push (vector-push (vector-new 4) 6) cond-node) then-node) else-node)
        r3 (infer-expr if-node env (subst-new) counter)]
        (do
          (print (result-failed r3))
          (print (ty-tag (result-type r3)))
          (print (ty-name (result-type r3)))))

      ;; テスト 4: let 式
      (let [init-node (vector-push (vector-push (vector-new 2) 1) 42)
        var-node (vector-push (vector-push (vector-new 2) 4) 999)
        let-node (vector-push (vector-push (vector-push (vector-push (vector-new 4) 7) 999) init-node) var-node)
        r4 (infer-expr let-node env (subst-new) counter)]
        (do
          (print (result-failed r4))
          (print (ty-tag (result-type r4)))
          (print (ty-name (result-type r4)))))

      ;; テスト 5: 変数の型環境登録と参照
      (let [env2 (type-env-insert env 777 (mono (mk-bool)))
        var-node (vector-push (vector-push (vector-new 2) 4) 777)
        r5 (infer-expr var-node env2 (subst-new) counter)]
        (do
          (print (result-failed r5))
          (print (ty-name (result-type r5)))))

      ;; テスト 6: 未定義変数 -> エラー
      (let [undef-var (vector-push (vector-push (vector-new 2) 4) 12345)
        r6 (infer-expr undef-var env (subst-new) counter)]
        (print (result-failed r6)))

      ;; テスト 7: do ブロック -> 最後の式の型
      (let [expr1 (vector-push (vector-push (vector-new 2) 1) 42)
        expr2 (vector-push (vector-push (vector-new 2) 2) 1)
        do-node (vector-push (vector-push (vector-push (vector-push (vector-new 4) 9) 2) expr1) expr2)
        r7 (infer-expr do-node env (subst-new) counter)]
        (do
          (print (result-failed r7))
          (print (ty-name (result-type r7)))))

      ;; テスト 8: if 式で条件が Bool でない -> エラー
      (let [bad-cond (vector-push (vector-push (vector-new 2) 1) 42)
        then-n (vector-push (vector-push (vector-new 2) 1) 1)
        else-n (vector-push (vector-push (vector-new 2) 1) 0)
        bad-if (vector-push (vector-push (vector-push (vector-push (vector-new 4) 6) bad-cond) then-n) else-n)
        r8 (infer-expr bad-if env (subst-new) counter)]
        (print (result-failed r8)))

      0)))
