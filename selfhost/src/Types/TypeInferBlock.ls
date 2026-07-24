(module Types.TypeInferBlock)
(import Syntax.AST)
(import Types.Type)
(import Types.TypeScheme)
(import Types.TypeInferCore)
(import Types.TypeInfer)

;; TypeInferBlock.ls - let 式、do ブロック、computation 式の型推論
;;
;; infer-let: let 式の型推論
;; infer-do: do ブロックの型推論 (1-14 式の covered slice)
;; infer-computation: computation 式の型推論
;; infer-computation-steps: computation ステップの型推論

;; let 式の型推論
;; [7, name-hash, init-expr, body-expr]
(defn infer-let [node env subst counter]
  (let [name-hash (vector-get node 1)
    init-node (vector-get node 2)
    body-node (vector-get node 3)
    ;; init を推論
    init-result (infer-expr init-node env subst counter)]
    (if (= (result-failed init-result) 1)
      (propagate-error-result-with-span-and-name init-result)
      (let [s1 (result-subst init-result)
        init-ty (result-type init-result)
        ;; 汎化して環境に追加
        scheme (generalize (apply-subst s1 init-ty) (map-new))
        new-env (type-env-insert env name-hash scheme)]
        ;; body を推論
        (infer-expr body-node new-env s1 counter)))))

;; computation expression の型推論
;; [15, builder-hash, step-count, step-kind1, aux1, expr1, ...]
;; 最小版: step を順に推論し、let! だけ束縛を環境へ追加して最後の式の型を返す
(defn infer-computation-steps [node idx step-count env subst counter last-ty]
  (make-result subst last-ty))

(defn infer-computation [node env subst counter]
  (let [step-count (vector-get node 2)]
    (if (= step-count 0)
      (make-result subst (mk-int))
      (if (= step-count 1)
        (infer-expr (vector-get node 5) env subst counter)
        (if (= step-count 2)
          (let [step1-kind (vector-get node 3)
            step1-aux (vector-get node 4)
            step1-expr (vector-get node 5)
            step2-kind (vector-get node 6)
            final-expr (vector-get node 8)]
            (if (= step2-kind (comp-step-return))
              (if (= step1-kind (comp-step-let-bang))
                (let [step1-result (infer-expr step1-expr env subst counter)]
                  (if (= (result-failed step1-result) 1)
                    (propagate-error-result-with-span-and-name step1-result)
                    (let [s1 (result-subst step1-result)
                      bound-ty (result-type step1-result)
                      env2 (type-env-insert env step1-aux (mono bound-ty))]
                      (infer-expr final-expr env2 s1 counter))))
                (if (= step1-kind (comp-step-do-bang))
                  (let [step1-result (infer-expr step1-expr env subst counter)]
                    (if (= (result-failed step1-result) 1)
                      (propagate-error-result step1-result)
                      (infer-expr final-expr env (result-subst step1-result) counter)))
                  (make-result subst (mk-int))))
              (make-result subst (mk-int))))
          (if (= step-count 3)
            (let [step1-kind (vector-get node 3)
              step1-aux (vector-get node 4)
              step1-expr (vector-get node 5)
              step2-kind (vector-get node 6)
              step2-aux (vector-get node 7)
              step2-expr (vector-get node 8)
              step3-kind (vector-get node 9)
              final-expr (vector-get node 11)]
              (if (= step3-kind (comp-step-return))
                (if (= step1-kind (comp-step-let-bang))
                  (let [step1-result (infer-expr step1-expr env subst counter)]
                    (if (= (result-failed step1-result) 1)
                      (propagate-error-result step1-result)
                      (let [s1 (result-subst step1-result)
                        bound1-ty (result-type step1-result)
                        env2 (type-env-insert env step1-aux (mono bound1-ty))]
                        (if (= step2-kind (comp-step-do-bang))
                          (let [step2-result (infer-expr step2-expr env2 s1 counter)]
                            (if (= (result-failed step2-result) 1)
                              (propagate-error-result step2-result)
                              (infer-expr final-expr env2 (result-subst step2-result) counter)))
                          (if (= step2-kind (comp-step-let-bang))
                            (let [step2-result (infer-expr step2-expr env2 s1 counter)]
                              (if (= (result-failed step2-result) 1)
                                (propagate-error-result step2-result)
                                (let [s2 (result-subst step2-result)
                                  bound2-ty (result-type step2-result)
                                  env3 (type-env-insert env2 step2-aux (mono bound2-ty))]
                                  (infer-expr final-expr env3 s2 counter))))
                            (make-result subst (mk-int)))))))
                  (if (= step1-kind (comp-step-do-bang))
                    (let [step1-result (infer-expr step1-expr env subst counter)]
                      (if (= (result-failed step1-result) 1)
                        (propagate-error-result step1-result)
                        (let [s1 (result-subst step1-result)]
                          (if (= step2-kind (comp-step-let-bang))
                            (let [step2-result (infer-expr step2-expr env s1 counter)]
                              (if (= (result-failed step2-result) 1)
                                (propagate-error-result step2-result)
                                (let [s2 (result-subst step2-result)
                                  bound2-ty (result-type step2-result)
                                  env2 (type-env-insert env step2-aux (mono bound2-ty))]
                                  (infer-expr final-expr env2 s2 counter))))
                            (if (= step2-kind (comp-step-do-bang))
                              (let [step2-result (infer-expr step2-expr env s1 counter)]
                                (if (= (result-failed step2-result) 1)
                                  (propagate-error-result step2-result)
                                  (infer-expr final-expr env (result-subst step2-result) counter)))
                              (make-result subst (mk-int)))))))
                    (make-result subst (mk-int))))
                (make-result subst (mk-int))))
            (make-result subst (mk-int))))))))

;; do ブロックの型推論
;; [9, expr-count, expr1, expr2, ...]
(defn infer-do [node env subst counter]
  (let [ec (vector-get node 1)]
    (if (= ec 0)
      ;; 空の do: Int(0) を返す
      (make-result subst (mk-int))
      (if (= ec 1)
        ;; 1 式
        (infer-expr (vector-get node 2) env subst counter)
        ;; 2 式以上: 各式を順次推論、最後の型を返す
        (let [r1 (infer-expr (vector-get node 2) env subst counter)]
          (if (= (result-failed r1) 1)
            (propagate-error-result-with-span-and-name r1)
            (let [s1 (result-subst r1)]
              (if (= ec 2)
                (infer-expr (vector-get node 3) env s1 counter)
                (let [r2 (infer-expr (vector-get node 3) env s1 counter)]
                  (if (= (result-failed r2) 1)
                    (propagate-error-result r2)
                    (let [s2 (result-subst r2)]
                      (if (= ec 3)
                        (infer-expr (vector-get node 4) env s2 counter)
                        (let [r3 (infer-expr (vector-get node 4) env s2 counter)]
                          (if (= (result-failed r3) 1)
                            (propagate-error-result r3)
                            (let [s3 (result-subst r3)]
                              (if (= ec 4)
                                (infer-expr (vector-get node 5) env s3 counter)
                                ;; covered slice として 5/6 式を扱う
                                (let [r4 (infer-expr (vector-get node 5) env s3 counter)]
                                  (if (= (result-failed r4) 1)
                                    (propagate-error-result r4)
                                    (let [s4 (result-subst r4)]
                                      (if (= ec 5)
                                        (infer-expr (vector-get node 6) env s4 counter)
                                        (if (= ec 6)
                                          (let [r5 (infer-expr (vector-get node 6) env s4 counter)]
                                            (if (= (result-failed r5) 1)
                                              (propagate-error-result r5)
                                              (infer-expr (vector-get node 7) env (result-subst r5) counter)))
                                          (if (= ec 7)
                                            (let [r5 (infer-expr (vector-get node 6) env s4 counter)]
                                              (if (= (result-failed r5) 1)
                                                (propagate-error-result r5)
                                                (let [s5 (result-subst r5)]
                                                  (infer-expr (vector-get node 8) env s5 counter))))
                                            (if (= ec 8)
                                              (let [r5 (infer-expr (vector-get node 6) env s4 counter)]
                                                (if (= (result-failed r5) 1)
                                                  (propagate-error-result r5)
                                                  (let [s5 (result-subst r5)
                                                    r6 (infer-expr (vector-get node 7) env s5 counter)]
                                                    (if (= (result-failed r6) 1)
                                                      (propagate-error-result r6)
                                                      (infer-expr (vector-get node 9) env (result-subst r6) counter)))))
                                              (if (= ec 9)
                                                (let [r5 (infer-expr (vector-get node 6) env s4 counter)]
                                                  (if (= (result-failed r5) 1)
                                                    (propagate-error-result r5)
                                                    (let [s5 (result-subst r5)
                                                      r6 (infer-expr (vector-get node 7) env s5 counter)]
                                                      (if (= (result-failed r6) 1)
                                                        (propagate-error-result r6)
                                                        (let [s6 (result-subst r6)]
                                                          (infer-expr (vector-get node 10) env s6 counter))))))
                                                (if (= ec 10)
                                                  (let [r5 (infer-expr (vector-get node 6) env s4 counter)]
                                                    (if (= (result-failed r5) 1)
                                                      (propagate-error-result r5)
                                                      (let [s5 (result-subst r5)
                                                        r6 (infer-expr (vector-get node 7) env s5 counter)]
                                                        (if (= (result-failed r6) 1)
                                                          (propagate-error-result r6)
                                                          (let [s6 (result-subst r6)
                                                            r7 (infer-expr (vector-get node 8) env s6 counter)]
                                                            (if (= (result-failed r7) 1)
                                                              (propagate-error-result r7)
                                                              (infer-expr (vector-get node 11) env (result-subst r7) counter)))))))
                                                  (if (= ec 11)
                                                    (let [r5 (infer-expr (vector-get node 6) env s4 counter)]
                                                      (if (= (result-failed r5) 1)
                                                        (propagate-error-result r5)
                                                        (let [s5 (result-subst r5)
                                                          r6 (infer-expr (vector-get node 7) env s5 counter)]
                                                          (if (= (result-failed r6) 1)
                                                            (propagate-error-result r6)
                                                            (let [s6 (result-subst r6)
                                                              r7 (infer-expr (vector-get node 8) env s6 counter)]
                                                              (if (= (result-failed r7) 1)
                                                                (propagate-error-result r7)
                                                                (let [s7 (result-subst r7)
                                                                  r8 (infer-expr (vector-get node 9) env s7 counter)]
                                                                  (if (= (result-failed r8) 1)
                                                                    (propagate-error-result r8)
                                                                    (infer-expr (vector-get node 12) env (result-subst r8) counter)))))))))
                                                    (if (= ec 12)
                                                      (let [r5 (infer-expr (vector-get node 6) env s4 counter)]
                                                        (if (= (result-failed r5) 1)
                                                          (propagate-error-result r5)
                                                          (let [s5 (result-subst r5)
                                                            r6 (infer-expr (vector-get node 7) env s5 counter)]
                                                            (if (= (result-failed r6) 1)
                                                              (propagate-error-result r6)
                                                              (let [s6 (result-subst r6)
                                                                r7 (infer-expr (vector-get node 8) env s6 counter)]
                                                                (if (= (result-failed r7) 1)
                                                                  (propagate-error-result r7)
                                                                  (let [s7 (result-subst r7)
                                                                    r8 (infer-expr (vector-get node 9) env s7 counter)]
                                                                    (if (= (result-failed r8) 1)
                                                                      (propagate-error-result r8)
                                                                      (let [s8 (result-subst r8)
                                                                        r9 (infer-expr (vector-get node 10) env s8 counter)]
                                                                        (if (= (result-failed r9) 1)
                                                                          (propagate-error-result r9)
                                                                          (infer-expr (vector-get node 13) env (result-subst r9) counter)))))))))))
                                                      (if (= ec 13)
                                                        (let [r5 (infer-expr (vector-get node 6) env s4 counter)]
                                                          (if (= (result-failed r5) 1)
                                                            (propagate-error-result r5)
                                                            (let [s5 (result-subst r5)
                                                              r6 (infer-expr (vector-get node 7) env s5 counter)]
                                                              (if (= (result-failed r6) 1)
                                                                (propagate-error-result r6)
                                                                (let [s6 (result-subst r6)
                                                                  r7 (infer-expr (vector-get node 8) env s6 counter)]
                                                                  (if (= (result-failed r7) 1)
                                                                    (propagate-error-result r7)
                                                                    (let [s7 (result-subst r7)
                                                                      r8 (infer-expr (vector-get node 9) env s7 counter)]
                                                                      (if (= (result-failed r8) 1)
                                                                        (propagate-error-result r8)
                                                                        (let [s8 (result-subst r8)
                                                                          r9 (infer-expr (vector-get node 10) env s8 counter)]
                                                                          (if (= (result-failed r9) 1)
                                                                            (propagate-error-result r9)
                                                                            (let [s9 (result-subst r9)
                                                                              r10 (infer-expr (vector-get node 11) env s9 counter)]
                                                                              (if (= (result-failed r10) 1)
                                                                                (propagate-error-result r10)
                                                                                (infer-expr (vector-get node 14) env (result-subst r10) counter)))))))))))))
                                                        (if (= ec 14)
                                                          (let [r5 (infer-expr (vector-get node 6) env s4 counter)]
                                                            (if (= (result-failed r5) 1)
                                                              (propagate-error-result r5)
                                                              (let [s5 (result-subst r5)
                                                                r6 (infer-expr (vector-get node 7) env s5 counter)]
                                                                (if (= (result-failed r6) 1)
                                                                  (propagate-error-result r6)
                                                                  (let [s6 (result-subst r6)
                                                                    r7 (infer-expr (vector-get node 8) env s6 counter)]
                                                                    (if (= (result-failed r7) 1)
                                                                      (propagate-error-result r7)
                                                                      (let [s7 (result-subst r7)
                                                                        r8 (infer-expr (vector-get node 9) env s7 counter)]
                                                                        (if (= (result-failed r8) 1)
                                                                          (propagate-error-result r8)
                                                                          (let [s8 (result-subst r8)
                                                                            r9 (infer-expr (vector-get node 10) env s8 counter)]
                                                                            (if (= (result-failed r9) 1)
                                                                              (propagate-error-result r9)
                                                                              (let [s9 (result-subst r9)
                                                                                r10 (infer-expr (vector-get node 11) env s9 counter)]
                                                                                (if (= (result-failed r10) 1)
                                                                                  (propagate-error-result r10)
                                                                                  (let [s10 (result-subst r10)
                                                                                    r11 (infer-expr (vector-get node 12) env s10 counter)]
                                                                                    (if (= (result-failed r11) 1)
                                                                                      (propagate-error-result r11)
                                                                                      (infer-expr (vector-get node 15) env (result-subst r11) counter)))))))))))))))
                                                          ;; 15 式以上は既存 fallback を維持
                                                          (infer-expr (vector-get node 6) env s4 counter))))))))))))))))))))))))))))))
