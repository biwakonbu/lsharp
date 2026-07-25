(module Backend.Wasm.Compiler)
(import Syntax.AST)
(import IR.IR)
(import Backend.Wasm.CompilerBase)
(import Backend.Wasm.CompilerSplit)
(defn compile-call-args-step-with-source [node source env ftable arg-idx arg-count instrs data-ref]
  (if (>= arg-idx arg-count)
    (make-compile-step-state 1 arg-idx instrs)
    (make-compile-step-state
      0
      (+ arg-idx 1)
      (compile-expr-with-source (vector-get node (+ 3 arg-idx)) source env ftable instrs data-ref))))

(defn continue-compile-call-args-step-with-source [node source env ftable arg-count state data-ref]
  (if (= (vector-get state 0) 1)
    state
    (compile-call-args-step-with-source node source env ftable (vector-get state 1) arg-count (vector-get state 2) data-ref)))

(defn compile-call-args-step-8-with-source [node source env ftable arg-idx arg-count instrs data-ref]
  (let [step1 (compile-call-args-step-with-source node source env ftable arg-idx arg-count instrs data-ref)
    step2 (continue-compile-call-args-step-with-source node source env ftable arg-count step1 data-ref)
    step3 (continue-compile-call-args-step-with-source node source env ftable arg-count step2 data-ref)
    step4 (continue-compile-call-args-step-with-source node source env ftable arg-count step3 data-ref)
    step5 (continue-compile-call-args-step-with-source node source env ftable arg-count step4 data-ref)
    step6 (continue-compile-call-args-step-with-source node source env ftable arg-count step5 data-ref)
    step7 (continue-compile-call-args-step-with-source node source env ftable arg-count step6 data-ref)
    step8 (continue-compile-call-args-step-with-source node source env ftable arg-count step7 data-ref)]
    step8))

(defn continue-compile-call-args-step-8-with-source [node source env ftable arg-count state data-ref]
  (if (= (vector-get state 0) 1)
    state
    (compile-call-args-step-8-with-source node source env ftable (vector-get state 1) arg-count (vector-get state 2) data-ref)))

(defn compile-call-args-step-64-with-source [node source env ftable arg-idx arg-count instrs data-ref]
  (let [step1 (compile-call-args-step-8-with-source node source env ftable arg-idx arg-count instrs data-ref)
    step2 (continue-compile-call-args-step-8-with-source node source env ftable arg-count step1 data-ref)
    step3 (continue-compile-call-args-step-8-with-source node source env ftable arg-count step2 data-ref)
    step4 (continue-compile-call-args-step-8-with-source node source env ftable arg-count step3 data-ref)
    step5 (continue-compile-call-args-step-8-with-source node source env ftable arg-count step4 data-ref)
    step6 (continue-compile-call-args-step-8-with-source node source env ftable arg-count step5 data-ref)
    step7 (continue-compile-call-args-step-8-with-source node source env ftable arg-count step6 data-ref)
    step8 (continue-compile-call-args-step-8-with-source node source env ftable arg-count step7 data-ref)]
    step8))

(defn compile-call-args-with-source [node source env ftable arg-idx arg-count instrs data-ref]
  (let [step (compile-call-args-step-64-with-source node source env ftable arg-idx arg-count instrs data-ref)]
    (if (= (vector-get step 0) 1)
      (vector-get step 2)
      (compile-call-args-with-source node source env ftable (vector-get step 1) arg-count (vector-get step 2) data-ref))))
(defn compile-call-args-step-with-ftable [node env ftable arg-idx arg-count instrs]
  (if (>= arg-idx arg-count)
    (make-compile-step-state 1 arg-idx instrs)
    (make-compile-step-state
      0
      (+ arg-idx 1)
      (compile-expr-with-ftable (vector-get node (+ 3 arg-idx)) env ftable instrs))))

(defn continue-compile-call-args-step-with-ftable [node env ftable arg-count state]
  (if (= (vector-get state 0) 1)
    state
    (compile-call-args-step-with-ftable node env ftable (vector-get state 1) arg-count (vector-get state 2))))

(defn compile-call-args-step-8-with-ftable [node env ftable arg-idx arg-count instrs]
  (let [step1 (compile-call-args-step-with-ftable node env ftable arg-idx arg-count instrs)
    step2 (continue-compile-call-args-step-with-ftable node env ftable arg-count step1)
    step3 (continue-compile-call-args-step-with-ftable node env ftable arg-count step2)
    step4 (continue-compile-call-args-step-with-ftable node env ftable arg-count step3)
    step5 (continue-compile-call-args-step-with-ftable node env ftable arg-count step4)
    step6 (continue-compile-call-args-step-with-ftable node env ftable arg-count step5)
    step7 (continue-compile-call-args-step-with-ftable node env ftable arg-count step6)
    step8 (continue-compile-call-args-step-with-ftable node env ftable arg-count step7)]
    step8))

(defn continue-compile-call-args-step-8-with-ftable [node env ftable arg-count state]
  (if (= (vector-get state 0) 1)
    state
    (compile-call-args-step-8-with-ftable node env ftable (vector-get state 1) arg-count (vector-get state 2))))

(defn compile-call-args-step-64-with-ftable [node env ftable arg-idx arg-count instrs]
  (let [step1 (compile-call-args-step-8-with-ftable node env ftable arg-idx arg-count instrs)
    step2 (continue-compile-call-args-step-8-with-ftable node env ftable arg-count step1)
    step3 (continue-compile-call-args-step-8-with-ftable node env ftable arg-count step2)
    step4 (continue-compile-call-args-step-8-with-ftable node env ftable arg-count step3)
    step5 (continue-compile-call-args-step-8-with-ftable node env ftable arg-count step4)
    step6 (continue-compile-call-args-step-8-with-ftable node env ftable arg-count step5)
    step7 (continue-compile-call-args-step-8-with-ftable node env ftable arg-count step6)
    step8 (continue-compile-call-args-step-8-with-ftable node env ftable arg-count step7)]
    step8))

(defn compile-call-args-with-ftable [node env ftable arg-idx arg-count instrs]
  (let [step (compile-call-args-step-64-with-ftable node env ftable arg-idx arg-count instrs)]
    (if (= (vector-get step 0) 1)
      (vector-get step 2)
      (compile-call-args-with-ftable node env ftable (vector-get step 1) arg-count (vector-get step 2)))))

(defn compile-user-call-arg-instrs-step-with-source [node source env ftable arg-idx arg-count arg-instrs data-ref]
  (if (>= arg-idx arg-count)
    (make-compile-step-state 1 arg-idx arg-instrs)
    (do
      (root_push node)
      (root_push source)
      (root_push env)
      (root_push ftable)
      (root_push arg-instrs)
      (root_push data-ref)
      (let [next-arg-instr (compile-expr-with-source (vector-get node (+ 3 arg-idx)) source env ftable (vector-new 8) data-ref)]
        (do
          (root_push next-arg-instr)
          (let [next-arg-instrs (push-object-vector arg-instrs next-arg-instr)]
            (do
              (root_push next-arg-instrs)
              (let [result (make-compile-step-state 0 (+ arg-idx 1) next-arg-instrs)]
                (do
                  (root_pop)
                  (root_pop)
                  (root_pop)
                  (root_pop)
                  (root_pop)
                  (root_pop)
                  (root_pop)
                  (root_pop)
                  result)))))))))

(defn continue-compile-user-call-arg-instrs-step-with-source [node source env ftable arg-count state data-ref]
  (if (= (vector-get state 0) 1)
    state
    (compile-user-call-arg-instrs-step-with-source node source env ftable (vector-get state 1) arg-count (vector-get state 2) data-ref)))

(defn compile-user-call-arg-instrs-step-8-with-source [node source env ftable arg-idx arg-count arg-instrs data-ref]
  (let [step1 (compile-user-call-arg-instrs-step-with-source node source env ftable arg-idx arg-count arg-instrs data-ref)
    step2 (continue-compile-user-call-arg-instrs-step-with-source node source env ftable arg-count step1 data-ref)
    step3 (continue-compile-user-call-arg-instrs-step-with-source node source env ftable arg-count step2 data-ref)
    step4 (continue-compile-user-call-arg-instrs-step-with-source node source env ftable arg-count step3 data-ref)
    step5 (continue-compile-user-call-arg-instrs-step-with-source node source env ftable arg-count step4 data-ref)
    step6 (continue-compile-user-call-arg-instrs-step-with-source node source env ftable arg-count step5 data-ref)
    step7 (continue-compile-user-call-arg-instrs-step-with-source node source env ftable arg-count step6 data-ref)
    step8 (continue-compile-user-call-arg-instrs-step-with-source node source env ftable arg-count step7 data-ref)]
    step8))

(defn continue-compile-user-call-arg-instrs-step-8-with-source [node source env ftable arg-count state data-ref]
  (if (= (vector-get state 0) 1)
    state
    (compile-user-call-arg-instrs-step-8-with-source node source env ftable (vector-get state 1) arg-count (vector-get state 2) data-ref)))

(defn compile-user-call-arg-instrs-step-64-with-source [node source env ftable arg-idx arg-count arg-instrs data-ref]
  (let [step1 (compile-user-call-arg-instrs-step-8-with-source node source env ftable arg-idx arg-count arg-instrs data-ref)
    step2 (continue-compile-user-call-arg-instrs-step-8-with-source node source env ftable arg-count step1 data-ref)
    step3 (continue-compile-user-call-arg-instrs-step-8-with-source node source env ftable arg-count step2 data-ref)
    step4 (continue-compile-user-call-arg-instrs-step-8-with-source node source env ftable arg-count step3 data-ref)
    step5 (continue-compile-user-call-arg-instrs-step-8-with-source node source env ftable arg-count step4 data-ref)
    step6 (continue-compile-user-call-arg-instrs-step-8-with-source node source env ftable arg-count step5 data-ref)
    step7 (continue-compile-user-call-arg-instrs-step-8-with-source node source env ftable arg-count step6 data-ref)
    step8 (continue-compile-user-call-arg-instrs-step-8-with-source node source env ftable arg-count step7 data-ref)]
    step8))

(defn compile-user-call-arg-instrs-with-source [node source env ftable arg-idx arg-count arg-instrs data-ref]
  (if (>= arg-idx arg-count)
    arg-instrs
    (let [node-slot (root_push node)
      source-slot (root_push source)
      env-slot (root_push env)
      ftable-slot (root_push ftable)
      data-slot (root_push data-ref)
      arg-instrs-slot (root_push arg-instrs)
      arg-node (vector-get node (+ 3 arg-idx))
      arg-node-slot (root_push arg-node)
      next-arg-base (vector-new 8)
      next-base-slot (root_push next-arg-base)
      next-arg-instr (compile-expr-with-source arg-node source env ftable next-arg-base data-ref)
      next-arg-slot (root_push next-arg-instr)
      next-arg-instrs (push-object-vector arg-instrs next-arg-instr)]
      (do
        (root_set arg-instrs-slot next-arg-instrs)
        (root_pop)
        (root_pop)
        (root_pop)
        (let [result (compile-user-call-arg-instrs-with-source node source env ftable (+ arg-idx 1) arg-count next-arg-instrs data-ref)]
          (do
            (root_pop)
            (root_pop)
            (root_pop)
            (root_pop)
            (root_pop)
            (root_pop)
            result))))))

(defn compile-user-call-arg-instrs-step-with-ftable [node env ftable arg-idx arg-count arg-instrs]
  (if (>= arg-idx arg-count)
    (make-compile-step-state 1 arg-idx arg-instrs)
    (do
      (root_push node)
      (root_push env)
      (root_push ftable)
      (root_push arg-instrs)
      (let [next-arg-instr (compile-expr-with-ftable (vector-get node (+ 3 arg-idx)) env ftable (vector-new 8))]
        (do
          (root_push next-arg-instr)
          (let [next-arg-instrs (push-object-vector arg-instrs next-arg-instr)]
            (do
              (root_push next-arg-instrs)
              (let [result (make-compile-step-state 0 (+ arg-idx 1) next-arg-instrs)]
                (do
                  (root_pop)
                  (root_pop)
                  (root_pop)
                  (root_pop)
                  (root_pop)
                  (root_pop)
                  result)))))))))

(defn continue-compile-user-call-arg-instrs-step-with-ftable [node env ftable arg-count state]
  (if (= (vector-get state 0) 1)
    state
    (compile-user-call-arg-instrs-step-with-ftable node env ftable (vector-get state 1) arg-count (vector-get state 2))))

(defn compile-user-call-arg-instrs-step-8-with-ftable [node env ftable arg-idx arg-count arg-instrs]
  (let [step1 (compile-user-call-arg-instrs-step-with-ftable node env ftable arg-idx arg-count arg-instrs)
    step2 (continue-compile-user-call-arg-instrs-step-with-ftable node env ftable arg-count step1)
    step3 (continue-compile-user-call-arg-instrs-step-with-ftable node env ftable arg-count step2)
    step4 (continue-compile-user-call-arg-instrs-step-with-ftable node env ftable arg-count step3)
    step5 (continue-compile-user-call-arg-instrs-step-with-ftable node env ftable arg-count step4)
    step6 (continue-compile-user-call-arg-instrs-step-with-ftable node env ftable arg-count step5)
    step7 (continue-compile-user-call-arg-instrs-step-with-ftable node env ftable arg-count step6)
    step8 (continue-compile-user-call-arg-instrs-step-with-ftable node env ftable arg-count step7)]
    step8))

(defn continue-compile-user-call-arg-instrs-step-8-with-ftable [node env ftable arg-count state]
  (if (= (vector-get state 0) 1)
    state
    (compile-user-call-arg-instrs-step-8-with-ftable node env ftable (vector-get state 1) arg-count (vector-get state 2))))

(defn compile-user-call-arg-instrs-step-64-with-ftable [node env ftable arg-idx arg-count arg-instrs]
  (let [step1 (compile-user-call-arg-instrs-step-8-with-ftable node env ftable arg-idx arg-count arg-instrs)
    step2 (continue-compile-user-call-arg-instrs-step-8-with-ftable node env ftable arg-count step1)
    step3 (continue-compile-user-call-arg-instrs-step-8-with-ftable node env ftable arg-count step2)
    step4 (continue-compile-user-call-arg-instrs-step-8-with-ftable node env ftable arg-count step3)
    step5 (continue-compile-user-call-arg-instrs-step-8-with-ftable node env ftable arg-count step4)
    step6 (continue-compile-user-call-arg-instrs-step-8-with-ftable node env ftable arg-count step5)
    step7 (continue-compile-user-call-arg-instrs-step-8-with-ftable node env ftable arg-count step6)
    step8 (continue-compile-user-call-arg-instrs-step-8-with-ftable node env ftable arg-count step7)]
    step8))

(defn compile-user-call-arg-instrs-with-ftable [node env ftable arg-idx arg-count arg-instrs]
  (if (>= arg-idx arg-count)
    arg-instrs
    (let [node-slot (root_push node)
      env-slot (root_push env)
      ftable-slot (root_push ftable)
      arg-instrs-slot (root_push arg-instrs)
      next-arg-instr (compile-expr-with-ftable (vector-get node (+ 3 arg-idx)) env ftable (vector-new 8))
      next-arg-slot (root_push next-arg-instr)
      next-arg-instrs (push-object-vector arg-instrs next-arg-instr)]
      (do
        (root_set arg-instrs-slot next-arg-instrs)
        (root_pop)
        (let [result (compile-user-call-arg-instrs-with-ftable node env ftable (+ arg-idx 1) arg-count next-arg-instrs)]
          (do
            (root_pop)
            (root_pop)
            (root_pop)
            (root_pop)
            result))))))

(defn emit-user-call-args-step [node arg-instrs-list arg-idx arg-count temp-base instrs]
  (if (>= arg-idx arg-count)
    (make-compile-step-state 1 arg-idx instrs)
    (let [arg-expr (vector-get node (+ 3 arg-idx))
      arg-instrs (vector-get arg-instrs-list arg-idx)
      arg-local (+ temp-base arg-idx)
      should-root (alloc-root-needed arg-expr)
      instrs1 (append-instr-vector instrs arg-instrs)
      instrs2 (emit-to instrs1 11 arg-local)
      instrs3 (maybe-root-push-drop instrs2 should-root arg-local)]
      (make-compile-step-state 0 (+ arg-idx 1) instrs3))))

(defn continue-emit-user-call-args-step [node arg-instrs-list arg-count temp-base state]
  (if (= (vector-get state 0) 1)
    state
    (emit-user-call-args-step node arg-instrs-list (vector-get state 1) arg-count temp-base (vector-get state 2))))

(defn emit-user-call-args-step-8 [node arg-instrs-list arg-idx arg-count temp-base instrs]
  (let [step1 (emit-user-call-args-step node arg-instrs-list arg-idx arg-count temp-base instrs)
    step2 (continue-emit-user-call-args-step node arg-instrs-list arg-count temp-base step1)
    step3 (continue-emit-user-call-args-step node arg-instrs-list arg-count temp-base step2)
    step4 (continue-emit-user-call-args-step node arg-instrs-list arg-count temp-base step3)
    step5 (continue-emit-user-call-args-step node arg-instrs-list arg-count temp-base step4)
    step6 (continue-emit-user-call-args-step node arg-instrs-list arg-count temp-base step5)
    step7 (continue-emit-user-call-args-step node arg-instrs-list arg-count temp-base step6)
    step8 (continue-emit-user-call-args-step node arg-instrs-list arg-count temp-base step7)]
    step8))

(defn continue-emit-user-call-args-step-8 [node arg-instrs-list arg-count temp-base state]
  (if (= (vector-get state 0) 1)
    state
    (emit-user-call-args-step-8 node arg-instrs-list (vector-get state 1) arg-count temp-base (vector-get state 2))))

(defn emit-user-call-args-step-64 [node arg-instrs-list arg-idx arg-count temp-base instrs]
  (let [step1 (emit-user-call-args-step-8 node arg-instrs-list arg-idx arg-count temp-base instrs)
    step2 (continue-emit-user-call-args-step-8 node arg-instrs-list arg-count temp-base step1)
    step3 (continue-emit-user-call-args-step-8 node arg-instrs-list arg-count temp-base step2)
    step4 (continue-emit-user-call-args-step-8 node arg-instrs-list arg-count temp-base step3)
    step5 (continue-emit-user-call-args-step-8 node arg-instrs-list arg-count temp-base step4)
    step6 (continue-emit-user-call-args-step-8 node arg-instrs-list arg-count temp-base step5)
    step7 (continue-emit-user-call-args-step-8 node arg-instrs-list arg-count temp-base step6)
    step8 (continue-emit-user-call-args-step-8 node arg-instrs-list arg-count temp-base step7)]
    step8))

(defn emit-user-call-args [node arg-instrs-list arg-idx arg-count temp-base instrs]
  (if (>= arg-idx arg-count)
    instrs
    (let [node-slot (root_push node)
      arg-instrs-list-slot (root_push arg-instrs-list)
      arg-expr (vector-get node (+ 3 arg-idx))
      arg-instrs (vector-get arg-instrs-list arg-idx)
      arg-instrs-slot (root_push arg-instrs)
      arg-local (+ temp-base arg-idx)
      should-root (alloc-root-needed arg-expr)
      instrs1 (append-instr-vector instrs arg-instrs)]
      (do
        (root_push instrs1)
        (let [instrs2 (emit-to instrs1 11 arg-local)]
          (do
            (root_push instrs2)
            (let [instrs3 (maybe-root-push-drop instrs2 should-root arg-local)]
              (do
                (root_push instrs3)
                (let [result (emit-user-call-args node arg-instrs-list (+ arg-idx 1) arg-count temp-base instrs3)]
                  (do
                    (root_pop)
                    (root_pop)
                    (root_pop)
                    (root_pop)
                    (root_pop)
                    (root_pop)
                    result))))))))))

(defn emit-user-call-arg-gets-step [arg-idx arg-count temp-base instrs]
  (if (>= arg-idx arg-count)
    (make-compile-step-state 1 arg-idx instrs)
    (make-compile-step-state 0 (+ arg-idx 1) (emit-to instrs 10 (+ temp-base arg-idx)))))

(defn continue-emit-user-call-arg-gets-step [arg-count temp-base state]
  (if (= (vector-get state 0) 1)
    state
    (emit-user-call-arg-gets-step (vector-get state 1) arg-count temp-base (vector-get state 2))))

(defn emit-user-call-arg-gets-step-8 [arg-idx arg-count temp-base instrs]
  (let [step1 (emit-user-call-arg-gets-step arg-idx arg-count temp-base instrs)
    step2 (continue-emit-user-call-arg-gets-step arg-count temp-base step1)
    step3 (continue-emit-user-call-arg-gets-step arg-count temp-base step2)
    step4 (continue-emit-user-call-arg-gets-step arg-count temp-base step3)
    step5 (continue-emit-user-call-arg-gets-step arg-count temp-base step4)
    step6 (continue-emit-user-call-arg-gets-step arg-count temp-base step5)
    step7 (continue-emit-user-call-arg-gets-step arg-count temp-base step6)
    step8 (continue-emit-user-call-arg-gets-step arg-count temp-base step7)]
    step8))

(defn continue-emit-user-call-arg-gets-step-8 [arg-count temp-base state]
  (if (= (vector-get state 0) 1)
    state
    (emit-user-call-arg-gets-step-8 (vector-get state 1) arg-count temp-base (vector-get state 2))))

(defn emit-user-call-arg-gets-step-64 [arg-idx arg-count temp-base instrs]
  (let [step1 (emit-user-call-arg-gets-step-8 arg-idx arg-count temp-base instrs)
    step2 (continue-emit-user-call-arg-gets-step-8 arg-count temp-base step1)
    step3 (continue-emit-user-call-arg-gets-step-8 arg-count temp-base step2)
    step4 (continue-emit-user-call-arg-gets-step-8 arg-count temp-base step3)
    step5 (continue-emit-user-call-arg-gets-step-8 arg-count temp-base step4)
    step6 (continue-emit-user-call-arg-gets-step-8 arg-count temp-base step5)
    step7 (continue-emit-user-call-arg-gets-step-8 arg-count temp-base step6)
    step8 (continue-emit-user-call-arg-gets-step-8 arg-count temp-base step7)]
    step8))

(defn emit-user-call-arg-gets [arg-idx arg-count temp-base instrs]
  (if (>= arg-idx arg-count)
    instrs
    (let [next-instrs (emit-to instrs 10 (+ temp-base arg-idx))]
      (do
        (root_push next-instrs)
        (let [result (emit-user-call-arg-gets (+ arg-idx 1) arg-count temp-base next-instrs)]
          (do
            (root_pop)
            result))))))

(defn emit-user-call-root-pops-step [node arg-idx instrs]
  (if (< arg-idx 0)
    (make-compile-step-state 1 arg-idx instrs)
    (let [arg-expr (vector-get node (+ 3 arg-idx))
      instrs1 (maybe-root-pop-drop instrs (alloc-root-needed arg-expr))]
      (make-compile-step-state 0 (- arg-idx 1) instrs1))))

(defn continue-emit-user-call-root-pops-step [node state]
  (if (= (vector-get state 0) 1)
    state
    (emit-user-call-root-pops-step node (vector-get state 1) (vector-get state 2))))

(defn emit-user-call-root-pops-step-8 [node arg-idx instrs]
  (let [step1 (emit-user-call-root-pops-step node arg-idx instrs)
    step2 (continue-emit-user-call-root-pops-step node step1)
    step3 (continue-emit-user-call-root-pops-step node step2)
    step4 (continue-emit-user-call-root-pops-step node step3)
    step5 (continue-emit-user-call-root-pops-step node step4)
    step6 (continue-emit-user-call-root-pops-step node step5)
    step7 (continue-emit-user-call-root-pops-step node step6)
    step8 (continue-emit-user-call-root-pops-step node step7)]
    step8))

(defn continue-emit-user-call-root-pops-step-8 [node state]
  (if (= (vector-get state 0) 1)
    state
    (emit-user-call-root-pops-step-8 node (vector-get state 1) (vector-get state 2))))

(defn emit-user-call-root-pops-step-64 [node arg-idx instrs]
  (let [step1 (emit-user-call-root-pops-step-8 node arg-idx instrs)
    step2 (continue-emit-user-call-root-pops-step-8 node step1)
    step3 (continue-emit-user-call-root-pops-step-8 node step2)
    step4 (continue-emit-user-call-root-pops-step-8 node step3)
    step5 (continue-emit-user-call-root-pops-step-8 node step4)
    step6 (continue-emit-user-call-root-pops-step-8 node step5)
    step7 (continue-emit-user-call-root-pops-step-8 node step6)
    step8 (continue-emit-user-call-root-pops-step-8 node step7)]
    step8))

(defn emit-user-call-root-pops [node arg-idx instrs]
  (let [step (emit-user-call-root-pops-step-64 node arg-idx instrs)]
    (if (= (vector-get step 0) 1)
      (vector-get step 2)
      (do
        (root_push node)
        (root_push step)
        (let [result (emit-user-call-root-pops node (vector-get step 1) (vector-get step 2))]
          (do
            (root_pop)
            (root_pop)
            result))))))

(defn compile-user-call-with-source [node source env ftable instrs data-ref func-hash arg-count]
  (let [node-slot (root_push node)
    source-slot (root_push source)
    env-slot (root_push env)
    ftable-slot (root_push ftable)
    instrs-slot (root_push instrs)
    data-slot (root_push data-ref)
    func-node (vector-get node 1)
    call-target (ftable-lookup-call-target ftable func-node func-hash)
    call-instrs (emit-to (vector-new 1) 40 call-target)]
    (do
      (root_push call-instrs)
      (let [arg-instrs-list (compile-user-call-arg-instrs-with-source node source env ftable 0 arg-count (vector-new 8) data-ref)]
        (do
          (root_push arg-instrs-list)
          (let [temp-base (max-root-temp-base-list env arg-instrs-list arg-count)
            instrs1 (emit-user-call-args node arg-instrs-list 0 arg-count temp-base instrs)
            instrs2 (emit-user-call-arg-gets 0 arg-count temp-base instrs1)
            instrs3 (append-instr-vector instrs2 call-instrs)]
            (do
              (root_push instrs3)
              (let [result (emit-user-call-root-pops node (- arg-count 1) instrs3)]
                (do
                  (root_pop)
                  (root_pop)
                  (root_pop)
                  (root_pop)
                  (root_pop)
                  (root_pop)
                  (root_pop)
                  (root_pop)
                  (root_pop)
                  result)))))))))

(defn compile-user-call-with-ftable [node env ftable instrs func-hash arg-count]
  (let [node-slot (root_push node)
    env-slot (root_push env)
    instrs-slot (root_push instrs)
    func-node (vector-get node 1)
    call-target (ftable-lookup-call-target ftable func-node func-hash)
    call-instrs (emit-to (vector-new 1) 40 call-target)]
    (do
      (root_push call-instrs)
      (let [arg-instrs-list (compile-user-call-arg-instrs-with-ftable node env ftable 0 arg-count (vector-new 8))]
        (do
          (root_push arg-instrs-list)
          (let [temp-base (max-root-temp-base-list env arg-instrs-list arg-count)
            instrs1 (emit-user-call-args node arg-instrs-list 0 arg-count temp-base instrs)
            instrs2 (emit-user-call-arg-gets 0 arg-count temp-base instrs1)
            instrs3 (append-instr-vector instrs2 call-instrs)]
            (do
              (root_push instrs3)
              (let [result (emit-user-call-root-pops node (- arg-count 1) instrs3)]
                (do
                  (root_pop)
                  (root_pop)
                  (root_pop)
                  (root_pop)
                  (root_pop)
                  (root_pop)
                  result)))))))))

(defn source-builtin-map-op [bop] (if (= bop 62) true (if (= bop 63) true (if (= bop 65) true (= bop 66)))))
(defn map-insert-op [bop] (= bop 62))

(defn unary-builtin-op [bop] (if (= bop 51) true (if (= bop 52) true (if (= bop 57) true (if (= bop 61) true (if (= bop 59) true (if (= bop 64) true (if (= bop 67) true (if (= bop 73) true (if (= bop 74) true (if (= bop 54) true (if (= bop 56) true (if (= bop 87) true (= bop 88))))))))))))))
(defn alloc-builtin-op [bop] (if (= bop 54) true (= bop 56)))

(defn env-slot-builtin-op [bop] (if (= bop 50) true (if (= bop 53) true (if (= bop 55) true (if (= bop 58) true (if (= bop 63) true (if (= bop 65) true (= bop 66))))))))
(defn nullary-builtin-op [bop] (if (= bop 75) true (= bop 86)))
(defn ternary-builtin-op [bop] (= bop 69))

(defn apply-args-safe-for-ftable [node arg-idx arg-count]
  (if (>= arg-idx arg-count)
    true
    (let [arg-tag (vector-get (vector-get node (+ 3 arg-idx)) 0)]
      (if (if (= arg-tag 1) true (if (= arg-tag 2) true (= arg-tag 4)))
        (apply-args-safe-for-ftable node (+ arg-idx 1) arg-count)
        false))))

(defn source-neutral-ftable-builtin-op [bop]
  (if (= bop 54)
    true
    (if (immediate-builtin-op bop)
      (if (= bop 65)
        false
        (if (= bop 74)
          false
          (if (= bop 76)
            false
            true)))
      false)))

(defn max-root-temp-base [env lhs-instrs rhs-instrs]
  (do
    (root_push lhs-instrs)
    (root_push rhs-instrs)
    (let [lhs-max (max-local-slot lhs-instrs 0 (vector-length lhs-instrs) 0)
      rhs-max (max-local-slot rhs-instrs 0 (vector-length rhs-instrs) 0)
      used-max1 (if (> lhs-max rhs-max) lhs-max rhs-max)
      used-max2 (if (> (map-size env) used-max1) (map-size env) used-max1)]
      (do
        (root_pop)
        (root_pop)
        (+ used-max2 1)))))

(defn max-root-temp-base3 [env instrs-a instrs-b instrs-c]
  (do
    (root_push instrs-a)
    (root_push instrs-b)
    (root_push instrs-c)
    (let [max-a (max-local-slot instrs-a 0 (vector-length instrs-a) 0)
      max-b (max-local-slot instrs-b 0 (vector-length instrs-b) 0)
      max-c (max-local-slot instrs-c 0 (vector-length instrs-c) 0)
      used-max1 (if (> max-a max-b) max-a max-b)
      used-max2 (if (> max-c used-max1) max-c used-max1)
      used-max3 (if (> (map-size env) used-max2) (map-size env) used-max2)]
      (do
        (root_pop)
        (root_pop)
        (root_pop)
        (+ used-max3 1)))))

(defn compile-map-key-with-source [key-expr source env ftable data-ref] (if (= (vector-get key-expr 0) 3) (compile-string-key-hash-with-source key-expr source (vector-new 8)) (compile-expr-with-source key-expr source env ftable (vector-new 8) data-ref)))
(defn compile-map-key-with-ftable [key-expr env ftable]
  (if (= (vector-get key-expr 0) 3)
    (if (> (vector-length key-expr) 3)
      (emit-to (vector-new 8) 1 (vector-get key-expr 3))
      (emit-to (vector-new 8) 1 0))
    (compile-expr-with-ftable key-expr env ftable (vector-new 8))))

(defn compile-ref-new-with-source [node source env ftable instrs data-ref]
  (let [node-slot (root_push node)
    source-slot (root_push source)
    env-slot (root_push env)
    ftable-slot (root_push ftable)
    instrs-slot (root_push instrs)
    data-slot (root_push data-ref)
    value-expr (vector-get node 3)]
    (if (= (alloc-root-needed value-expr) 0)
      (let [instrs1 (compile-expr-with-source value-expr source env ftable instrs data-ref)]
        (do
          (root_pop)
          (root_pop)
          (root_pop)
          (root_pop)
          (root_pop)
          (root_pop)
          (emit-to instrs1 56 (+ 1 (map-size env)))))
      (let [value-instrs (compile-expr-with-source value-expr source env ftable (vector-new 8) data-ref)]
        (do
          (root_push value-instrs)
          (let [temp-base (max-root-temp-base1 env value-instrs)
            value-local temp-base
            instrs1 (append-instr-vector instrs value-instrs)
            instrs2 (emit-to instrs1 11 value-local)
            instrs3 (emit-root-push-drop instrs2 value-local)
            instrs4 (emit-to instrs3 10 value-local)
            instrs5 (emit-to instrs4 56 (+ 1 (map-size env)))
            instrs6 (emit-root-pop-drop instrs5)]
            (do
              (root_pop)
              (root_pop)
              (root_pop)
              (root_pop)
              (root_pop)
              (root_pop)
              (root_pop)
              instrs6)))))))
(defn compile-ref-new-with-ftable [node env ftable instrs]
  (let [node-slot (root_push node)
    env-slot (root_push env)
    ftable-slot (root_push ftable)
    instrs-slot (root_push instrs)
    value-expr (vector-get node 3)]
    (if (= (alloc-root-needed value-expr) 0)
      (let [instrs1 (compile-expr-with-ftable value-expr env ftable instrs)]
        (do
          (root_pop)
          (root_pop)
          (root_pop)
          (root_pop)
          (emit-to instrs1 56 (+ 1 (map-size env)))))
      (let [value-instrs (compile-expr-with-ftable value-expr env ftable (vector-new 8))]
        (do
          (root_push value-instrs)
          (let [temp-base (max-root-temp-base1 env value-instrs)
            value-local temp-base
            instrs1 (append-instr-vector instrs value-instrs)
            instrs2 (emit-to instrs1 11 value-local)
            instrs3 (emit-root-push-drop instrs2 value-local)
            instrs4 (emit-to instrs3 10 value-local)
            instrs5 (emit-to instrs4 56 (+ 1 (map-size env)))
            instrs6 (emit-root-pop-drop instrs5)]
            (do
              (root_pop)
              (root_pop)
              (root_pop)
              (root_pop)
              (root_pop)
              instrs6)))))))

(defn compile-vector-push-with-source [node source env ftable instrs data-ref]
  (let [vector-expr (vector-get node 3)
    value-expr (vector-get node 4)
    vector-root (alloc-root-needed vector-expr)
    value-root (alloc-root-needed value-expr)
    vector-instrs (compile-expr-with-source vector-expr source env ftable (vector-new 8) data-ref)
    value-instrs (compile-expr-with-source value-expr source env ftable (vector-new 8) data-ref)]
    (do
      (root_push vector-instrs)
      (root_push value-instrs)
      (let [result (compile-vector-push-instrs env instrs vector-instrs value-instrs vector-root value-root)]
        (do
          (root_pop)
          (root_pop)
          result)))))
(defn compile-vector-push-instrs [env instrs vector-instrs value-instrs vector-root value-root]
  (let [temp-base (max-root-temp-base env vector-instrs value-instrs)
    vector-local temp-base
    value-local (+ temp-base 1)
    instrs1 (append-instr-vector instrs vector-instrs)
    instrs2 (emit-to instrs1 11 vector-local)
    instrs3 (maybe-root-push-drop instrs2 vector-root vector-local)
    instrs4 (append-instr-vector instrs3 value-instrs)
    instrs5 (emit-to instrs4 11 value-local)
    instrs6 (maybe-root-push-drop instrs5 value-root value-local)
    instrs7 (emit-to instrs6 10 vector-local)
    instrs8 (emit-to instrs7 10 value-local)
    instrs9 (emit-to instrs8 55 (+ 1 (map-size env)))
    instrs10 (maybe-root-pop-drop instrs9 value-root)]
    (maybe-root-pop-drop instrs10 vector-root)))
(defn compile-vector-push-with-ftable [node env ftable instrs]
  (let [vector-expr (vector-get node 3)
    value-expr (vector-get node 4)
    vector-root (alloc-root-needed vector-expr)
    value-root (alloc-root-needed value-expr)
    vector-instrs (compile-expr-with-ftable vector-expr env ftable (vector-new 8))
    value-instrs (compile-expr-with-ftable value-expr env ftable (vector-new 8))]
    (do
      (root_push vector-instrs)
      (root_push value-instrs)
      (let [result (compile-vector-push-instrs env instrs vector-instrs value-instrs vector-root value-root)]
        (do
          (root_pop)
          (root_pop)
          result)))))

(defn compile-map-insert-builtin-instrs [env instrs map-instrs key-instrs value-instrs map-root key-root value-root bop]
  (let [temp-base (max-root-temp-base3 env map-instrs key-instrs value-instrs)
    map-local temp-base
    key-local (+ temp-base 1)
    value-local (+ temp-base 2)
    instrs1 (append-instr-vector instrs map-instrs)
    instrs2 (emit-to instrs1 11 map-local)
    instrs3 (maybe-root-push-drop instrs2 map-root map-local)
    instrs4 (append-instr-vector instrs3 key-instrs)
    instrs5 (emit-to instrs4 11 key-local)
    instrs6 (maybe-root-push-drop instrs5 key-root key-local)
    instrs7 (append-instr-vector instrs6 value-instrs)
    instrs8 (emit-to instrs7 11 value-local)
    instrs9 (maybe-root-push-drop instrs8 value-root value-local)
    instrs10 (emit-to instrs9 10 map-local)
    instrs11 (emit-to instrs10 10 key-local)
    instrs12 (emit-to instrs11 10 value-local)
    instrs13 (emit-to instrs12 bop (+ 1 (map-size env)))
    instrs14 (maybe-root-pop-drop instrs13 value-root)
    instrs15 (maybe-root-pop-drop instrs14 key-root)
    instrs16 (maybe-root-pop-drop instrs15 map-root)]
    instrs16))

(defn compile-map-builtin-simple-instrs [env instrs map-instrs key-instrs bop]
  (let [instrs1 (append-instr-vector instrs map-instrs)
    instrs2 (append-instr-vector instrs1 key-instrs)
    instrs3 (emit-to instrs2 bop (+ 1 (map-size env)))]
    instrs3))

(defn compile-map-builtin-rooted-instrs [env instrs map-instrs key-instrs map-root key-root bop]
  (let [temp-base (max-root-temp-base env map-instrs key-instrs)
    map-local temp-base
    key-local (+ temp-base 1)
    instrs1 (append-instr-vector instrs map-instrs)
    instrs2 (emit-to instrs1 11 map-local)
    instrs3 (maybe-root-push-drop instrs2 map-root map-local)
    instrs4 (append-instr-vector instrs3 key-instrs)
    instrs5 (emit-to instrs4 11 key-local)
    instrs6 (maybe-root-push-drop instrs5 key-root key-local)
    instrs7 (emit-to instrs6 10 map-local)
    instrs8 (emit-to instrs7 10 key-local)
    instrs9 (emit-to instrs8 bop (+ 1 (map-size env)))
    instrs10 (maybe-root-pop-drop instrs9 key-root)
    instrs11 (maybe-root-pop-drop instrs10 map-root)]
    instrs11))

(defn compile-map-insert-builtin-with-ftable [node env ftable instrs bop map-instrs key-instrs map-root key-root]
  (let [value-expr (vector-get node 5)
    value-instrs (compile-expr-with-ftable value-expr env ftable (vector-new 8))
    value-root (alloc-root-needed value-expr)]
    (do
      (root_push value-instrs)
      (let [instrs16 (compile-map-insert-builtin-instrs env instrs map-instrs key-instrs value-instrs map-root key-root value-root bop)]
        (do
          (root_pop)
          instrs16)))))

(defn compile-map-lookup-builtin-with-ftable [env instrs map-instrs key-instrs map-root key-root bop simple-path]
  (if simple-path
    (compile-map-builtin-simple-instrs env instrs map-instrs key-instrs bop)
    (compile-map-builtin-rooted-instrs env instrs map-instrs key-instrs map-root key-root bop)))

(defn compile-map-builtin-with-ftable [node env ftable instrs bop]
  (let [map-expr (vector-get node 3)
    key-expr (vector-get node 4)
    map-instrs (compile-expr-with-ftable map-expr env ftable (vector-new 8))
    map-root (alloc-root-needed map-expr)]
    (let [map-slot (root_push map-instrs)]
      (let [key-instrs (compile-map-key-with-ftable key-expr env ftable)
        key-root (map-key-root-needed-with-source key-expr)
        simple-path (if (simple-map-operand map-expr) (simple-map-operand key-expr) false)]
        (let [key-slot (root_push key-instrs)]
          (if (= bop 62)
            (let [result (compile-map-insert-builtin-with-ftable node env ftable instrs bop map-instrs key-instrs map-root key-root)]
              (do
                (root_push result)
                (root_set map-slot result)
                (root_pop)
                (root_pop)
                (root_pop)
                result))
            (let [result (compile-map-lookup-builtin-with-ftable env instrs map-instrs key-instrs map-root key-root bop simple-path)]
              (do
                (root_push result)
                (root_set map-slot result)
                (root_pop)
                (root_pop)
                (root_pop)
                result))))))))

(defn compile-map-builtin-with-ftable-normal-setup-diagnostic [node env ftable instrs bop data-ref]
  (let [map-expr (vector-get node 3)
    key-expr (vector-get node 4)]
    (do
      (print 9000000250)
      (print bop)
      (print (vector-get map-expr 0))
      (print (vector-get key-expr 0))
      (print (vector-length instrs))
      (print (vector-length (ref-get data-ref)))
      (let [map-instrs (compile-expr-with-ftable map-expr env ftable (vector-new 8))
        map-root (alloc-root-needed map-expr)
        map-simple (if (simple-map-operand map-expr) 1 0)]
        (do
          (root_push map-instrs)
          (print 9000000251)
          (print (vector-length map-instrs))
          (print map-root)
          (print map-simple)
          (print (vector-length (ref-get data-ref)))
          (let [key-instrs (compile-map-key-with-ftable key-expr env ftable)
            key-root (map-key-root-needed-with-source key-expr)
            simple-path (if (simple-map-operand map-expr) (simple-map-operand key-expr) false)]
            (do
              (root_push key-instrs)
              (print 9000000252)
              (print (vector-length key-instrs))
              (print key-root)
              (print (if simple-path 1 0))
              (print (vector-length (ref-get data-ref)))
              (if (= bop 62)
                (let [result (compile-map-insert-builtin-with-ftable node env ftable instrs bop map-instrs key-instrs map-root key-root)]
                  (do
                    (root_push result)
                    (print 9000000253)
                    (print (vector-length result))
                    (print (vector-length (ref-get data-ref)))
                    (root_pop)
                    (root_pop)
                    (root_pop)
                    result))
                (let [result (compile-map-lookup-builtin-with-ftable env instrs map-instrs key-instrs map-root key-root bop simple-path)]
                  (do
                    (root_push result)
                    (print 9000000253)
                    (print (vector-length result))
                    (print (vector-length (ref-get data-ref)))
                    (root_pop)
                    (root_pop)
                    (root_pop)
                    result))))))))))

(defn compile-substring-instrs [env instrs src-instrs start-instrs end-instrs]
  (let [temp-base (max-root-temp-base3 env src-instrs start-instrs end-instrs)
    src-local temp-base
    instrs1 (append-instr-vector instrs src-instrs)
    instrs2 (emit-to instrs1 11 src-local)
    instrs3 (emit-root-push-drop instrs2 src-local)
    instrs4 (emit-to instrs3 10 src-local)
    instrs5 (append-instr-vector instrs4 start-instrs)
    instrs6 (append-instr-vector instrs5 end-instrs)
    instrs7 (emit-to instrs6 69 0)
    instrs8 (emit-root-pop-drop instrs7)]
    instrs8))

(defn compile-substring-with-source [node source env ftable instrs data-ref]
  (let [src-instrs (compile-expr-with-source (vector-get node 3) source env ftable (vector-new 8) data-ref)
    start-instrs (compile-expr-with-source (vector-get node 4) source env ftable (vector-new 8) data-ref)
    end-instrs (compile-expr-with-source (vector-get node 5) source env ftable (vector-new 8) data-ref)]
    (do
      (root_push src-instrs)
      (root_push start-instrs)
      (root_push end-instrs)
      (let [result (compile-substring-instrs env instrs src-instrs start-instrs end-instrs)]
        (do
          (root_pop)
          (root_pop)
          (root_pop)
          result)))))
(defn compile-substring-with-ftable [node env ftable instrs]
  (let [src-instrs (compile-expr-with-ftable (vector-get node 3) env ftable (vector-new 8))
    start-instrs (compile-expr-with-ftable (vector-get node 4) env ftable (vector-new 8))
    end-instrs (compile-expr-with-ftable (vector-get node 5) env ftable (vector-new 8))]
    (do
      (root_push src-instrs)
      (root_push start-instrs)
      (root_push end-instrs)
      (let [result (compile-substring-instrs env instrs src-instrs start-instrs end-instrs)]
        (do
          (root_pop)
          (root_pop)
          (root_pop)
          result)))))

(defn compile-string-concat-instrs [env instrs lhs-instrs rhs-instrs]
  (let [temp-base (max-root-temp-base env lhs-instrs rhs-instrs)
    lhs-local temp-base
    rhs-local (+ temp-base 1)
    instrs1 (append-instr-vector instrs lhs-instrs)
    instrs2 (emit-to instrs1 11 lhs-local)
    instrs3 (emit-root-push-drop instrs2 lhs-local)
    instrs4 (append-instr-vector instrs3 rhs-instrs)
    instrs5 (emit-to instrs4 11 rhs-local)
    instrs6 (emit-root-push-drop instrs5 rhs-local)
    instrs7 (emit-to instrs6 10 lhs-local)
    instrs8 (emit-to instrs7 10 rhs-local)
    instrs9 (emit-to instrs8 70 0)
    instrs10 (emit-root-pop-drop instrs9)
    instrs11 (emit-root-pop-drop instrs10)]
    instrs11))

(defn compile-string-concat-with-source [node source env ftable instrs data-ref]
  (let [lhs-instrs (compile-expr-with-source (vector-get node 3) source env ftable (vector-new 8) data-ref)
    rhs-instrs (compile-expr-with-source (vector-get node 4) source env ftable (vector-new 8) data-ref)]
    (do
      (root_push lhs-instrs)
      (root_push rhs-instrs)
      (let [result (compile-string-concat-instrs env instrs lhs-instrs rhs-instrs)]
        (do
          (root_pop)
          (root_pop)
          result)))))
(defn compile-string-concat-with-ftable [node env ftable instrs]
  (let [lhs-instrs (compile-expr-with-ftable (vector-get node 3) env ftable (vector-new 8))
    rhs-instrs (compile-expr-with-ftable (vector-get node 4) env ftable (vector-new 8))]
    (do
      (root_push lhs-instrs)
      (root_push rhs-instrs)
      (let [result (compile-string-concat-instrs env instrs lhs-instrs rhs-instrs)]
        (do
          (root_pop)
          (root_pop)
          result)))))

(defn emit-unary-builtin-with-ftable [instrs bop env] (if (alloc-builtin-op bop) (emit-to instrs bop (+ 1 (map-size env))) (emit-to instrs bop 0)))

(defn emit-binary-or-ternary-builtin-extra-with-ftable [node env ftable instrs2 bop]
  (if (map-insert-op bop)
    (let [instrs3 (compile-expr-with-ftable (vector-get node 5) env ftable instrs2)]
      (do
        (root_push instrs3)
        (let [result (emit-to instrs3 bop (+ 1 (map-size env)))]
          (do
            (root_pop)
            result))))
    (if (ternary-builtin-op bop)
      (let [instrs3 (compile-expr-with-ftable (vector-get node 5) env ftable instrs2)]
        (do
          (root_push instrs3)
          (let [result (emit-to instrs3 bop 0)]
            (do
              (root_pop)
              result))))
      (emit-to instrs2 bop 0))))

(defn emit-binary-or-ternary-builtin-instrs-with-ftable [node env ftable instrs2 bop]
  (if (env-slot-builtin-op bop)
    (emit-to instrs2 bop (+ 1 (map-size env)))
    (emit-binary-or-ternary-builtin-extra-with-ftable node env ftable instrs2 bop)))

(defn compile-binary-or-ternary-builtin-with-ftable [node env ftable instrs1 bop]
  (let [instrs2 (compile-expr-with-ftable (vector-get node 4) env ftable instrs1)]
    (do
      (root_push instrs2)
      (let [result (emit-binary-or-ternary-builtin-instrs-with-ftable node env ftable instrs2 bop)]
        (do
          (root_pop)
          result)))))

(defn emit-unary-builtin-with-source [instrs bop env] (if (alloc-builtin-op bop) (emit-to instrs bop (+ 1 (map-size env))) (emit-to instrs bop 0)))
(defn emit-binary-or-ternary-builtin-extra-with-source [node source env ftable instrs2 data-ref bop]
  (if (map-insert-op bop)
    (let [instrs3 (compile-expr-with-source (vector-get node 5) source env ftable instrs2 data-ref)]
      (do
        (root_push instrs3)
        (let [result (emit-to instrs3 bop (+ 1 (map-size env)))]
          (do
            (root_pop)
            result))))
    (if (ternary-builtin-op bop)
      (let [instrs3 (compile-expr-with-source (vector-get node 5) source env ftable instrs2 data-ref)]
        (do
          (root_push instrs3)
          (let [result (emit-to instrs3 bop 0)]
            (do
              (root_pop)
              result))))
      (emit-to instrs2 bop 0))))
(defn emit-binary-or-ternary-builtin-instrs-with-source [node source env ftable instrs2 data-ref bop]
  (if (env-slot-builtin-op bop)
    (emit-to instrs2 bop (+ 1 (map-size env)))
    (emit-binary-or-ternary-builtin-extra-with-source node source env ftable instrs2 data-ref bop)))
(defn compile-binary-or-ternary-builtin-with-source [node source env ftable instrs1 data-ref bop]
  (let [instrs2 (compile-expr-with-source (vector-get node 4) source env ftable instrs1 data-ref)]
    (do
      (root_push instrs2)
      (let [result (emit-binary-or-ternary-builtin-instrs-with-source node source env ftable instrs2 data-ref bop)]
        (do
          (root_pop)
          result)))))

(defn compile-not-instrs [instrs]
  (do
    (root_push instrs)
    (let [with-zero (emit-to instrs 1 0)]
      (do
        (root_push with-zero)
        (let [result (emit-to with-zero 30 0)]
          (do
            (root_pop)
            (root_pop)
            result))))))

(defn compile-not-builtin-with-source [node source env ftable instrs data-ref]
  (let [arg-instrs (compile-expr-with-source (vector-get node 3) source env ftable instrs data-ref)]
    (do
      (root_push arg-instrs)
      (let [result (compile-not-instrs arg-instrs)]
        (do
          (root_pop)
          result)))))

(defn compile-not-builtin-with-ftable [node env ftable instrs]
  (let [arg-instrs (compile-expr-with-ftable (vector-get node 3) env ftable instrs)]
    (do
      (root_push arg-instrs)
      (let [result (compile-not-instrs arg-instrs)]
        (do
          (root_pop)
          result)))))

(defn compile-simple-builtin-with-source [node source env ftable instrs data-ref bop]
  (let [node-slot (root_push node)
    source-slot (root_push source)
    env-slot (root_push env)
    ftable-slot (root_push ftable)
    data-slot (root_push data-ref)
    instrs1 (compile-expr-with-source (vector-get node 3) source env ftable instrs data-ref)]
    (do
      (root_push instrs1)
      (let [result
        (if (unary-builtin-op bop)
          (emit-unary-builtin-with-source instrs1 bop env)
          (compile-binary-or-ternary-builtin-with-source node source env ftable instrs1 data-ref bop))]
        (do
          (root_pop)
          (root_pop)
          (root_pop)
          (root_pop)
          (root_pop)
          (root_pop)
          result)))))
(defn root-set-slot-simple [slot-expr]
  (let [slot-tag (vector-get slot-expr 0)]
    (if (= slot-tag 1) true (if (= slot-tag 2) true (= slot-tag 4)))))
(defn compile-root-set-rooted-with-source [source env ftable instrs data-ref slot-expr slot-simple value-expr value-root]
  (let [value-instrs (compile-expr-with-source value-expr source env ftable (vector-new 8) data-ref)]
    (do
      (root_push value-instrs)
      (let [result (compile-root-set-instrs-with-source source env ftable instrs data-ref slot-expr slot-simple value-root value-instrs)]
        (do
          (root_pop)
          result)))))
(defn compile-root-set-with-source [node source env ftable instrs data-ref]
  (let [slot-expr (vector-get node 3)
    value-expr (vector-get node 4)
    value-root (alloc-root-needed value-expr)]
    (if (= value-root 0)
      (compile-simple-builtin-with-source node source env ftable instrs data-ref 76)
      (compile-root-set-rooted-with-source source env ftable instrs data-ref slot-expr (root-set-slot-simple slot-expr) value-expr value-root))))
(defn compile-root-set-instrs-with-source [source env ftable instrs data-ref slot-expr slot-simple value-root value-instrs]
  (let [instrs1 (append-instr-vector instrs value-instrs)
    value-local (max-root-temp-base1 env instrs1)
    instrs2 (emit-to instrs1 11 value-local)
    instrs3 (if slot-simple instrs2 (maybe-root-push-drop instrs2 value-root value-local))
    instrs4 (compile-expr-with-source slot-expr source env ftable instrs3 data-ref)
    instrs5 (emit-to instrs4 10 value-local)
    instrs6 (emit-to instrs5 76 0)
    result (if slot-simple instrs6 (maybe-root-pop-drop instrs6 value-root))]
    result))
(defn compile-builtin-apply-fallback-with-source [node source env ftable instrs data-ref bop safe-ftable-path]
  (if (= bop 60)
    (emit-to instrs bop (+ 1 (map-size env)))
    (if (nullary-builtin-op bop)
      (emit-to instrs bop 0)
      (if safe-ftable-path
        (compile-builtin-apply-with-ftable node env ftable instrs bop)
        (if (source-builtin-map-op bop)
          (compile-map-builtin-with-source node source env ftable instrs data-ref bop)
          (compile-simple-builtin-with-source node source env ftable instrs data-ref bop))))))
(defn compile-string-family-builtin-with-source [node source env ftable instrs data-ref bop]
  (if (= bop 70)
    (compile-string-concat-with-source node source env ftable instrs data-ref)
    (compile-substring-with-source node source env ftable instrs data-ref)))
(defn compile-stateful-builtin-with-source [node source env ftable instrs data-ref bop]
  (if (= bop 55)
    (compile-vector-push-with-source node source env ftable instrs data-ref)
    (if (= bop 56)
      (compile-ref-new-with-source node source env ftable instrs data-ref)
      (compile-root-set-with-source node source env ftable instrs data-ref))))
(defn compile-builtin-apply-with-source [node source env ftable instrs data-ref bop]
  (let [arg-count (vector-get node 2)
    safe-ftable-path (if (source-neutral-ftable-builtin-op bop) (apply-args-safe-for-ftable node 0 arg-count) false)]
    (let [result
      (if (= bop 70)
        (compile-string-family-builtin-with-source node source env ftable instrs data-ref bop)
        (if (= bop 69)
          (compile-string-family-builtin-with-source node source env ftable instrs data-ref bop)
        (if (= bop 55)
          (compile-stateful-builtin-with-source node source env ftable instrs data-ref bop)
          (if (= bop 56)
            (compile-stateful-builtin-with-source node source env ftable instrs data-ref bop)
            (if (= bop 76)
              (compile-stateful-builtin-with-source node source env ftable instrs data-ref bop)
              (compile-builtin-apply-fallback-with-source node source env ftable instrs data-ref bop safe-ftable-path))))))]
      (do
        (root_push result)
        (root_pop)
        result))))

(defn compile-builtin-apply-with-source-normal-setup-diagnostic [node source env ftable instrs data-ref bop safe-ftable-path]
  (let [result
    (if (if safe-ftable-path (source-builtin-map-op bop) false)
      (compile-map-builtin-with-ftable-normal-setup-diagnostic node env ftable instrs bop data-ref)
      (compile-builtin-apply-with-source node source env ftable instrs data-ref bop))]
    (do
      (root_push result)
      (print 9000000249)
      (print bop)
      (print (vector-length result))
      (print (vector-length (ref-get data-ref)))
      (root_pop)
      result)))

(defn compile-builtin-apply-simple-fallback-with-ftable [node env ftable instrs bop]
  (let [instrs1 (compile-expr-with-ftable (vector-get node 3) env ftable instrs)]
    (do
      (root_push instrs1)
      (let [result
        (if (unary-builtin-op bop)
          (emit-unary-builtin-with-ftable instrs1 bop env)
          (compile-binary-or-ternary-builtin-with-ftable node env ftable instrs1 bop))]
        (do
          (root_pop)
          result)))))

(defn compile-builtin-apply-fallback-with-ftable [node env ftable instrs bop]
  (if (= bop 60)
    (emit-to instrs bop (+ 1 (map-size env)))
    (if (nullary-builtin-op bop)
      (emit-to instrs bop 0)
      (if (source-builtin-map-op bop)
        (compile-map-builtin-with-ftable node env ftable instrs bop)
        (compile-builtin-apply-simple-fallback-with-ftable node env ftable instrs bop)))))

(defn compile-string-family-builtin-with-ftable [node env ftable instrs bop]
  (if (= bop 70)
    (compile-string-concat-with-ftable node env ftable instrs)
    (compile-substring-with-ftable node env ftable instrs)))

(defn compile-stateful-builtin-with-ftable [node env ftable instrs bop]
  (if (= bop 55)
    (compile-vector-push-with-ftable node env ftable instrs)
    (compile-ref-new-with-ftable node env ftable instrs)))

(defn compile-builtin-apply-with-ftable [node env ftable instrs bop]
  (if (= bop 70)
    (compile-string-family-builtin-with-ftable node env ftable instrs bop)
    (if (= bop 69)
      (compile-string-family-builtin-with-ftable node env ftable instrs bop)
    (if (= bop 55)
      (compile-stateful-builtin-with-ftable node env ftable instrs bop)
      (if (= bop 56)
        (compile-stateful-builtin-with-ftable node env ftable instrs bop)
        (compile-builtin-apply-fallback-with-ftable node env ftable instrs bop))))))

(defn compile-do-exprs-step [node env ftable idx expr-count instrs]
  (if (>= idx expr-count)
    (make-compile-step-state 1 idx instrs)
    (let [value-instrs (compile-expr-with-ftable (vector-get node (+ 2 idx)) env ftable instrs)
      next-instrs (if (< (+ idx 1) expr-count) (emit-to value-instrs 44 0) value-instrs)]
      (make-compile-step-state 0 (+ idx 1) next-instrs))))

(defn continue-compile-do-exprs-step [node env ftable expr-count state]
  (if (= (vector-get state 0) 1)
    state
    (compile-do-exprs-step node env ftable (vector-get state 1) expr-count (vector-get state 2))))

(defn compile-do-exprs-step-8 [node env ftable idx expr-count instrs]
  (let [step1 (compile-do-exprs-step node env ftable idx expr-count instrs)
    step2 (continue-compile-do-exprs-step node env ftable expr-count step1)
    step3 (continue-compile-do-exprs-step node env ftable expr-count step2)
    step4 (continue-compile-do-exprs-step node env ftable expr-count step3)
    step5 (continue-compile-do-exprs-step node env ftable expr-count step4)
    step6 (continue-compile-do-exprs-step node env ftable expr-count step5)
    step7 (continue-compile-do-exprs-step node env ftable expr-count step6)
    step8 (continue-compile-do-exprs-step node env ftable expr-count step7)]
    step8))

(defn continue-compile-do-exprs-step-8 [node env ftable expr-count state]
  (if (= (vector-get state 0) 1)
    state
    (compile-do-exprs-step-8 node env ftable (vector-get state 1) expr-count (vector-get state 2))))

(defn compile-do-exprs-step-64 [node env ftable idx expr-count instrs]
  (let [step1 (compile-do-exprs-step-8 node env ftable idx expr-count instrs)
    step2 (continue-compile-do-exprs-step-8 node env ftable expr-count step1)
    step3 (continue-compile-do-exprs-step-8 node env ftable expr-count step2)
    step4 (continue-compile-do-exprs-step-8 node env ftable expr-count step3)
    step5 (continue-compile-do-exprs-step-8 node env ftable expr-count step4)
    step6 (continue-compile-do-exprs-step-8 node env ftable expr-count step5)
    step7 (continue-compile-do-exprs-step-8 node env ftable expr-count step6)
    step8 (continue-compile-do-exprs-step-8 node env ftable expr-count step7)]
    step8))

(defn continue-compile-do-exprs [node env ftable expr-count step]
  (if (= (vector-get step 0) 1)
    (vector-get step 2)
    (compile-do-exprs node env ftable (vector-get step 1) expr-count (vector-get step 2))))

(defn compile-match-with-ftable-rest [node env ftable arm-count scr-idx instrs]
  (if (> arm-count 1)
    (let [i1 (compile-match-arm-prefix node scr-idx 5 instrs)
      i2 (compile-expr-with-ftable (vector-get node 6) env ftable i1)
      i3 (emit-to i2 43 0)]
      (if (> arm-count 2)
        (let [i4 (compile-match-arm-prefix node scr-idx 7 i3)
          i5 (compile-expr-with-ftable (vector-get node 8) env ftable i4)
          i6 (emit-to i5 43 0)]
          (compile-match-default-double-tail i6))
        (compile-match-default-double-tail i3)))
    (compile-match-default-tail instrs)))

(defn compile-map-builtin-with-source [node source env ftable instrs data-ref bop]
  (let [map-expr (vector-get node 3)
    key-expr (vector-get node 4)
    map-instrs (compile-expr-with-source map-expr source env ftable (vector-new 8) data-ref)
    map-root (alloc-root-needed map-expr)]
    (let [map-slot (root_push map-instrs)]
      (let [key-instrs (compile-map-key-with-source key-expr source env ftable data-ref)
        key-root (map-key-root-needed-with-source key-expr)
        simple-path (if (simple-map-operand map-expr) (simple-map-operand key-expr) false)]
        (let [key-slot (root_push key-instrs)]
          (if (= bop 62)
            (let [value-expr (vector-get node 5)
              value-instrs (compile-expr-with-source value-expr source env ftable (vector-new 8) data-ref)
              value-root (alloc-root-needed value-expr)]
              (do
                (root_push value-instrs)
                (let [result (compile-map-insert-builtin-instrs env instrs map-instrs key-instrs value-instrs map-root key-root value-root bop)]
                  (do
                    (root_push result)
                    (root_set map-slot result)
                    (root_pop)
                    (root_pop)
                    (root_pop)
                    (root_pop)
                    result))))
            (let [result (compile-map-lookup-builtin-with-ftable env instrs map-instrs key-instrs map-root key-root bop simple-path)]
              (do
                (root_push result)
                (root_set map-slot result)
                (root_pop)
                (root_pop)
                (root_pop)
                result))))))))
(defn compile-recordupdate-with-ftable-entry [node env ftable instrs]
  (compile-recordupdate-with-ftable node env ftable instrs))

(defn compile-expr-with-ftable-dispatch-complex-2-rest [tag node env ftable instrs]
  (if (= tag 8)
    (compile-lambda-with-ftable node env ftable instrs)
      (if (= tag 9)
        (compile-do-exprs node env ftable 0 (vector-get node 1) instrs)
      (if (= tag 10)
        (compile-match-with-ftable node env ftable instrs)
        (if (= tag 12)
          (compile-recordlit-with-ftable node env ftable instrs)
          (if (= tag 13)
            (compile-fieldaccess-with-ftable node env ftable instrs)
            (if (= tag 14)
              (compile-recordupdate-with-ftable-entry node env ftable instrs)
              (emit-to instrs 1 0))))))))

(defn compile-apply-with-source [node source env ftable instrs data-ref]
  (let [func-node (vector-get node 1)
    arg-count (vector-get node 2)]
    (let [func-tag (vector-get func-node 0)
      func-hash (if (= func-tag 4) (vector-get func-node 1) 0)]
      (let [bop (builtin-opcode func-hash)]
        (let [result
          (if (builtin-not-application? func-hash arg-count)
            (compile-not-builtin-with-source node source env ftable instrs data-ref)
            (if (> bop 0)
              (compile-builtin-apply-with-source node source env ftable instrs data-ref bop)
              (compile-user-call-with-source node source env ftable instrs data-ref func-hash arg-count)))]
          (do
            (root_push result)
            (root_pop)
            result))))))
(defn compile-apply-with-source-normal-setup-diagnostic [node source env ftable instrs data-ref]
  (let [func-node (vector-get node 1)
    arg-count (vector-get node 2)]
    (let [func-tag (vector-get func-node 0)
      func-hash (if (= func-tag 4) (vector-get func-node 1) 0)]
      (let [bop (builtin-opcode func-hash)
        safe-ftable-path (if (source-neutral-ftable-builtin-op bop) (apply-args-safe-for-ftable node 0 arg-count) false)]
        (do
          (print 9000000248)
          (print arg-count)
          (print func-tag)
          (print bop)
          (print (if safe-ftable-path 1 0))
          (print (vector-length instrs))
          (print (vector-length (ref-get data-ref)))
          (let [result
            (if (builtin-not-application? func-hash arg-count)
              (compile-not-builtin-with-source node source env ftable instrs data-ref)
              (if (> bop 0)
                (compile-builtin-apply-with-source-normal-setup-diagnostic node source env ftable instrs data-ref bop safe-ftable-path)
                (compile-user-call-with-source node source env ftable instrs data-ref func-hash arg-count)))]
            (do
              (root_push result)
              (print 9000000249)
              (print bop)
              (print (vector-length result))
              (print (vector-length (ref-get data-ref)))
              (root_pop)
              result)))))))
(defn compile-do-exprs-step-with-source [node source env ftable idx expr-count instrs data-ref]
  (if (>= idx expr-count)
    (make-compile-step-state 1 idx instrs)
    (let [value-instrs (compile-do-expr-with-source node source env ftable idx instrs data-ref)]
      (finish-compile-do-exprs-step idx expr-count value-instrs))))

(defn continue-compile-do-exprs-step-with-source [node source env ftable expr-count state data-ref]
  (if (= (vector-get state 0) 1)
    state
    (compile-do-exprs-step-with-source node source env ftable (vector-get state 1) expr-count (vector-get state 2) data-ref)))

(defn compile-do-exprs-step-8-with-source [node source env ftable idx expr-count instrs data-ref]
  (let [step1 (compile-do-exprs-step-with-source node source env ftable idx expr-count instrs data-ref)
    step2 (continue-compile-do-exprs-step-with-source node source env ftable expr-count step1 data-ref)
    step3 (continue-compile-do-exprs-step-with-source node source env ftable expr-count step2 data-ref)
    step4 (continue-compile-do-exprs-step-with-source node source env ftable expr-count step3 data-ref)
    step5 (continue-compile-do-exprs-step-with-source node source env ftable expr-count step4 data-ref)
    step6 (continue-compile-do-exprs-step-with-source node source env ftable expr-count step5 data-ref)
    step7 (continue-compile-do-exprs-step-with-source node source env ftable expr-count step6 data-ref)
    step8 (continue-compile-do-exprs-step-with-source node source env ftable expr-count step7 data-ref)]
    step8))

(defn continue-compile-do-exprs-step-8-with-source [node source env ftable expr-count state data-ref]
  (if (= (vector-get state 0) 1)
    state
    (compile-do-exprs-step-8-with-source node source env ftable (vector-get state 1) expr-count (vector-get state 2) data-ref)))

(defn compile-do-exprs-step-64-with-source [node source env ftable idx expr-count instrs data-ref]
  (let [step1 (compile-do-exprs-step-8-with-source node source env ftable idx expr-count instrs data-ref)
    step2 (continue-compile-do-exprs-step-8-with-source node source env ftable expr-count step1 data-ref)
    step3 (continue-compile-do-exprs-step-8-with-source node source env ftable expr-count step2 data-ref)
    step4 (continue-compile-do-exprs-step-8-with-source node source env ftable expr-count step3 data-ref)
    step5 (continue-compile-do-exprs-step-8-with-source node source env ftable expr-count step4 data-ref)
    step6 (continue-compile-do-exprs-step-8-with-source node source env ftable expr-count step5 data-ref)
    step7 (continue-compile-do-exprs-step-8-with-source node source env ftable expr-count step6 data-ref)
    step8 (continue-compile-do-exprs-step-8-with-source node source env ftable expr-count step7 data-ref)]
    step8))

(defn continue-compile-do-exprs-with-source [node source env ftable expr-count step data-ref]
  (if (= (vector-get step 0) 1)
    (vector-get step 2)
    (compile-do-exprs-with-source node source env ftable (vector-get step 1) expr-count (vector-get step 2) data-ref)))

(defn compile-do-exprs-with-source [node source env ftable idx expr-count instrs data-ref]
  (continue-compile-do-exprs-with-source node source env ftable expr-count (compile-do-exprs-step-64-with-source node source env ftable idx expr-count instrs data-ref) data-ref))

(defn compile-do-with-source [node source env ftable instrs data-ref]
  (let [expr-count (vector-get node 1)]
    (if (= expr-count 0)
      instrs
      (compile-do-exprs-with-source node source env ftable 0 expr-count instrs data-ref))))
(defn compile-do-exprs-step-with-source-normal-setup-diagnostic [node source env ftable idx expr-count instrs data-ref]
  (if (>= idx expr-count)
    (make-compile-step-state 1 idx instrs)
    (let [expr (vector-get node (+ 2 idx))]
      (do
        (print 9000000244)
        (print idx)
        (print expr-count)
        (print (vector-get expr 0))
        (print (vector-length expr))
        (print (vector-length instrs))
        (print (vector-length (ref-get data-ref)))
        (let [value-instrs (compile-expr-with-source-normal-setup-diagnostic expr source env ftable instrs data-ref)]
          (do
            (root_push value-instrs)
            (print 9000000245)
            (print idx)
            (print (vector-length value-instrs))
            (print (vector-length (ref-get data-ref)))
            (let [state (finish-compile-do-exprs-step idx expr-count value-instrs)]
              (do
                (root_push state)
                (print 9000000246)
                (print idx)
                (print (vector-get state 0))
                (print (vector-get state 1))
                (print (vector-length (vector-get state 2)))
                (print (vector-length (ref-get data-ref)))
                (root_pop)
                (root_pop)
                state))))))))

(defn continue-compile-do-exprs-step-with-source-normal-setup-diagnostic [node source env ftable expr-count state data-ref]
  (if (= (vector-get state 0) 1)
    state
    (compile-do-exprs-step-with-source-normal-setup-diagnostic node source env ftable (vector-get state 1) expr-count (vector-get state 2) data-ref)))

(defn compile-do-exprs-step-8-with-source-normal-setup-diagnostic [node source env ftable idx expr-count instrs data-ref]
  (let [step1 (compile-do-exprs-step-with-source-normal-setup-diagnostic node source env ftable idx expr-count instrs data-ref)
    step2 (continue-compile-do-exprs-step-with-source-normal-setup-diagnostic node source env ftable expr-count step1 data-ref)
    step3 (continue-compile-do-exprs-step-with-source-normal-setup-diagnostic node source env ftable expr-count step2 data-ref)
    step4 (continue-compile-do-exprs-step-with-source-normal-setup-diagnostic node source env ftable expr-count step3 data-ref)
    step5 (continue-compile-do-exprs-step-with-source-normal-setup-diagnostic node source env ftable expr-count step4 data-ref)
    step6 (continue-compile-do-exprs-step-with-source-normal-setup-diagnostic node source env ftable expr-count step5 data-ref)
    step7 (continue-compile-do-exprs-step-with-source-normal-setup-diagnostic node source env ftable expr-count step6 data-ref)
    step8 (continue-compile-do-exprs-step-with-source-normal-setup-diagnostic node source env ftable expr-count step7 data-ref)]
    step8))

(defn continue-compile-do-exprs-step-8-with-source-normal-setup-diagnostic [node source env ftable expr-count state data-ref]
  (if (= (vector-get state 0) 1)
    state
    (compile-do-exprs-step-8-with-source-normal-setup-diagnostic node source env ftable (vector-get state 1) expr-count (vector-get state 2) data-ref)))

(defn compile-do-exprs-step-64-with-source-normal-setup-diagnostic [node source env ftable idx expr-count instrs data-ref]
  (let [step1 (compile-do-exprs-step-8-with-source-normal-setup-diagnostic node source env ftable idx expr-count instrs data-ref)
    step2 (continue-compile-do-exprs-step-8-with-source-normal-setup-diagnostic node source env ftable expr-count step1 data-ref)
    step3 (continue-compile-do-exprs-step-8-with-source-normal-setup-diagnostic node source env ftable expr-count step2 data-ref)
    step4 (continue-compile-do-exprs-step-8-with-source-normal-setup-diagnostic node source env ftable expr-count step3 data-ref)
    step5 (continue-compile-do-exprs-step-8-with-source-normal-setup-diagnostic node source env ftable expr-count step4 data-ref)
    step6 (continue-compile-do-exprs-step-8-with-source-normal-setup-diagnostic node source env ftable expr-count step5 data-ref)
    step7 (continue-compile-do-exprs-step-8-with-source-normal-setup-diagnostic node source env ftable expr-count step6 data-ref)
    step8 (continue-compile-do-exprs-step-8-with-source-normal-setup-diagnostic node source env ftable expr-count step7 data-ref)]
    step8))

(defn continue-compile-do-exprs-with-source-normal-setup-diagnostic [node source env ftable expr-count step data-ref]
  (if (= (vector-get step 0) 1)
    (vector-get step 2)
    (compile-do-exprs-with-source-normal-setup-diagnostic node source env ftable (vector-get step 1) expr-count (vector-get step 2) data-ref)))

(defn compile-do-exprs-with-source-normal-setup-diagnostic [node source env ftable idx expr-count instrs data-ref]
  (continue-compile-do-exprs-with-source-normal-setup-diagnostic node source env ftable expr-count (compile-do-exprs-step-64-with-source-normal-setup-diagnostic node source env ftable idx expr-count instrs data-ref) data-ref))

(defn compile-do-with-source-normal-setup-diagnostic [node source env ftable instrs data-ref]
  (let [expr-count (vector-get node 1)]
    (do
      (print 9000000243)
      (print expr-count)
      (print (vector-length node))
      (print (vector-length instrs))
      (print (vector-length (ref-get data-ref)))
      (if (= expr-count 0)
        instrs
        (let [result (compile-do-exprs-with-source-normal-setup-diagnostic node source env ftable 0 expr-count instrs data-ref)]
          (do
            (root_push result)
            (print 9000000247)
            (print expr-count)
            (print (vector-length result))
            (print (vector-length (ref-get data-ref)))
            (root_pop)
            result))))))
(defn compile-lambda-with-source [node source env ftable instrs data-ref]
  (do
    (root_push node)
    (root_push source)
    (root_push env)
    (root_push ftable)
    (root_push instrs)
    (root_push data-ref)
    (let [param-count (vector-get node 1)
      new-env (bind-node-params node 2 0 param-count env (+ 1 (map-size env)))]
      (do
        (root_push new-env)
        (let [result (compile-expr-with-source (vector-get node (+ 2 param-count)) source new-env ftable instrs data-ref)]
          (do
            (root_pop)
            (root_pop)
            (root_pop)
            (root_pop)
            (root_pop)
            (root_pop)
            (root_pop)
            result))))))
(defn compile-match-arms-with-source [node idx arm-count source env ftable data-ref scr-idx result-local scratch-base binder-base instrs]
  (if (>= idx arm-count)
    instrs
    (let [pattern-slot (+ 3 (* idx 2))
      body-slot (+ pattern-slot 1)
      pat (vector-get node pattern-slot)
      body (vector-get node body-slot)
      bind-state (bind-match-pattern pat env binder-base)]
      (do
        (root_push bind-state)
        (let [arm-env (vector-get bind-state 0)
          pattern-temp-base (vector-get bind-state 1)
          checked (compile-match-pattern-check-with-scratch pat scr-idx scratch-base pattern-temp-base ftable instrs)
          opened (emit-to checked (op-if-empty) 0)
          bound (compile-match-pattern-binders pat scr-idx arm-env scratch-base pattern-temp-base ftable opened)
          body-instrs (compile-expr-with-source body source arm-env ftable bound data-ref)
          stored (emit-to body-instrs (op-local-set) result-local)
          exited (emit-to stored (op-br) 1)
          else-opened (emit-to exited (op-else) 0)
          rest
            (compile-match-arms-with-source
              node
              (+ idx 1)
              arm-count
              source
              env
              ftable
              data-ref
              scr-idx
              result-local
              scratch-base
              binder-base
              else-opened)
          closed (emit-to rest (op-end) 0)]
          (do
            (root_pop)
            closed))))))

(defn compile-match-with-source [node source env ftable instrs data-ref]
  (let [scrutinee (vector-get node 1)
    arm-count (vector-get node 2)
    scr-idx (+ 1 (map-size env))
    instrs1 (compile-expr-with-source scrutinee source env ftable instrs data-ref)
    instrs2 (emit-to instrs1 (op-local-set) scr-idx)
    scratch-base (max-root-temp-base env instrs2 (vector-new 0))
    result-local (+ scratch-base 6)
    binder-base (+ result-local 1)
    instrs3 (emit-to instrs2 (op-i64-const) 0)
    instrs4 (emit-to instrs3 (op-local-set) result-local)
    instrs5 (emit-to instrs4 (op-block) 0)
    instrs6
      (compile-match-arms-with-source
        node
        0
        arm-count
        source
        env
        ftable
        data-ref
        scr-idx
        result-local
        scratch-base
        binder-base
        instrs5)
    instrs7 (emit-to instrs6 (op-end) 0)]
    (emit-to instrs7 (op-local-get) result-local)))
(defn compile-recordupdate-with-source-entry [node source env ftable instrs data-ref]
  (compile-recordupdate-with-source node source env ftable instrs data-ref))
(defn compile-expr-with-source-dispatch [node source env ftable instrs data-ref]
  (let [tag (vector-get node 0)]
    (if (= tag 3)
      (compile-string-literal-with-source node source instrs data-ref)
      (if (= tag 9)
        (compile-do-with-source node source env ftable instrs data-ref)
        (if (= tag 6)
          (compile-if-with-source node source env ftable instrs data-ref)
          (if (= tag 5)
            (compile-apply-with-source node source env ftable instrs data-ref)
            (if (= tag 7)
              (compile-let-with-source node source env ftable instrs data-ref)
              (if (= tag 8)
                (compile-lambda-with-source node source env ftable instrs data-ref)
                (if (= tag 10)
                  (compile-match-with-source node source env ftable instrs data-ref)
                  (if (= tag 12)
                    (compile-recordlit-with-source node source env ftable instrs data-ref)
                    (if (= tag 13)
                      (compile-fieldaccess-with-source node source env ftable instrs data-ref)
                      (if (= tag 14)
                        (compile-recordupdate-with-source-entry node source env ftable instrs data-ref)
                        (compile-expr-with-ftable node env ftable instrs)))))))))))))
(defn compile-expr-with-source [node source env ftable instrs data-ref]
  (let [node-slot (root_push node)
    source-slot (root_push source)
    env-slot (root_push env)
    ftable-slot (root_push ftable)
    instrs-slot (root_push instrs)
    data-slot (root_push data-ref)
    result (compile-expr-with-source-dispatch node source env ftable instrs data-ref)]
    (do
      (root_push result)
      (root_pop)
      (root_pop)
      (root_pop)
      (root_pop)
      (root_pop)
      (root_pop)
      (root_pop)
      result)))

;; record は既存の Map runtime に保持する。
;; field 値は順に map-insert し、record 自体は field 式の allocation をまたいで root に残す。
;; nominal discriminator は record pattern lowering と同時に追加する。
(defn record-update-base-key [] -1)

(defn record-literal-type-hash-for-compiler [node]
  (let [field-count (vector-get node 2)
    qualified-slot (+ 3 (* field-count 2))
    raw-type-slot (+ qualified-slot 1)]
    (if (> (vector-length node) raw-type-slot)
      (vector-get node raw-type-slot)
      (vector-get node 1))))

(defn record-literal-nominal-marker-for-compiler [node ftable]
  (let [type-hash (vector-get node 1)
    marker (ftable-lookup ftable (record-nominal-marker-lookup-key type-hash))]
    (if (> marker 0)
      marker
      (record-literal-type-hash-for-compiler node))))

(defn compile-record-nominal-marker [env instrs record-local type-hash]
  (if (= type-hash 0)
    instrs
    (let [marker-instrs (emit-to (vector-new 2) (op-i64-const) type-hash)]
      (do
        (root_push marker-instrs)
        (let [result
                (compile-record-map-field-instrs
                  env
                  instrs
                  record-local
                  (record-nominal-type-key)
                  marker-instrs)]
          (do
            (root_pop)
            result))))))

(defn compile-record-map-field-instrs [env instrs record-local field-hash value-instrs]
  (let [map-local (max-root-temp-base env instrs value-instrs)
    key-local (+ map-local 1)
    value-local (+ map-local 2)
    instrs1 (emit-to instrs (op-local-get) record-local)
    instrs2 (emit-to instrs1 (op-local-set) map-local)
    instrs3 (emit-to instrs2 (op-i64-const) field-hash)
    instrs4 (emit-to instrs3 (op-local-set) key-local)
    instrs5 (append-instr-vector instrs4 value-instrs)
    instrs6 (emit-to instrs5 (op-local-set) value-local)
    instrs7 (emit-to instrs6 (op-local-get) map-local)
    instrs8 (emit-to instrs7 (op-local-get) key-local)
    instrs9 (emit-to instrs8 (op-local-get) value-local)
    instrs10 (emit-to instrs9 (op-map-insert) map-local)]
    (emit-to instrs10 (op-local-set) record-local)))

;; record update は patch map と base map の二層で表現する。
;; field access は patch に無い field を base へ再帰的に委譲するため、
;; map runtime を変更せず元の record の値を保持できる。
(defn compile-record-get-with-fallback [env instrs record-instrs field-hash]
  (let [record-local (max-root-temp-base env instrs record-instrs)
    map-op (+ record-local 1)
    base-map-op (+ map-op 6)
    result-local (+ base-map-op 6)
    instrs1 (append-instr-vector instrs record-instrs)
    instrs2 (emit-to instrs1 (op-local-set) record-local)
    instrs3 (emit-root-push-drop instrs2 record-local)
    instrs4 (emit-to instrs3 (op-block) 0)
    instrs5 (emit-to instrs4 (op-loop) 0)
    instrs6 (emit-to instrs5 (op-local-get) record-local)
    instrs7 (emit-to instrs6 (op-i64-const) field-hash)
    instrs8 (emit-to instrs7 (op-map-contains) map-op)
    instrs9 (emit-to instrs8 (op-if-empty) 0)
    instrs10 (emit-to instrs9 (op-local-get) record-local)
    instrs11 (emit-to instrs10 (op-i64-const) field-hash)
    instrs12 (emit-to instrs11 (op-map-get) map-op)
    instrs13 (emit-to instrs12 (op-local-set) result-local)
    instrs14 (emit-to instrs13 (op-br) 2)
    instrs15 (emit-to instrs14 (op-else) 0)
    instrs16 (emit-to instrs15 (op-local-get) record-local)
    instrs17 (emit-to instrs16 (op-i64-const) (record-update-base-key))
    instrs18 (emit-to instrs17 (op-map-get) base-map-op)
    instrs19 (emit-to instrs18 (op-local-set) record-local)
    instrs20 (emit-to instrs19 (op-br) 1)
    instrs21 (emit-to instrs20 (op-end) 0)
    instrs22 (emit-to instrs21 (op-end) 0)
    instrs23 (emit-to instrs22 (op-end) 0)
    instrs24 (emit-to instrs23 (op-local-get) result-local)]
    (emit-root-pop-drop instrs24)))

(defn compile-recordupdate-with-source [node source env ftable instrs data-ref]
  (do
    (root_push node)
    (root_push source)
    (root_push env)
    (root_push ftable)
    (let [instrs-slot (root_push instrs)]
      (do
        (root_push data-ref)
        (let [base-expr (vector-get node 1)
          base-instrs (compile-expr-with-source base-expr source env ftable (vector-new 8) data-ref)]
          (do
            (root_push base-instrs)
            (let [base-local (max-root-temp-base env instrs base-instrs)
              record-local (+ base-local 1)
              instrs1 (append-instr-vector instrs base-instrs)
              instrs2 (emit-to instrs1 (op-local-set) base-local)
              instrs3 (emit-root-push-drop instrs2 base-local)
              instrs4 (emit-to instrs3 (op-map-new) record-local)
              instrs5 (emit-to instrs4 (op-local-set) record-local)
              instrs6 (emit-root-push-drop instrs5 record-local)
              base-value-instrs (emit-to (vector-new 2) (op-local-get) base-local)]
              (do
                (root_push base-value-instrs)
                (let [base-marker-instrs
                        (compile-record-get-with-fallback
                          env
                          (vector-new 2)
                          base-value-instrs
                          (record-nominal-type-key))]
                  (do
                    (root_push base-marker-instrs)
                    (let [instrs7 (compile-record-map-field-instrs
                                    env
                                    instrs6
                                    record-local
                                    (record-update-base-key)
                                    base-value-instrs)
                      instrs8 (compile-record-map-field-instrs
                                env
                                instrs7
                                record-local
                                (record-nominal-type-key)
                                base-marker-instrs)]
                      (do
                        (root_set instrs-slot instrs8)
                        (let [with-fields
                                (compile-recordlit-fields-with-source
                                  node
                                  source
                                  env
                                  ftable
                                  0
                                  (vector-get node 2)
                                  instrs8
                                  record-local
                                  data-ref)]
                          (do
                            (root_set instrs-slot with-fields)
                            (let [instrs9 (emit-to with-fields (op-local-get) record-local)
                              instrs10 (emit-root-pop-drop instrs9)
                              result (emit-root-pop-drop instrs10)]
                              (do
                                (root_set instrs-slot result)
                                (root_pop)
                                (root_pop)
                                (root_pop)
                                (root_pop)
                                (root_pop)
                                (root_pop)
                                (root_pop)
                                (root_pop)
                                (root_pop)
                                result))))))))))))))))

(defn compile-recordupdate-with-ftable [node env ftable instrs]
  (do
    (root_push node)
    (root_push env)
    (root_push ftable)
    (let [instrs-slot (root_push instrs)
      base-expr (vector-get node 1)
      base-instrs (compile-expr-with-ftable base-expr env ftable (vector-new 8))]
      (do
        (root_push base-instrs)
        (let [base-local (max-root-temp-base env instrs base-instrs)
          record-local (+ base-local 1)
          instrs1 (append-instr-vector instrs base-instrs)
          instrs2 (emit-to instrs1 (op-local-set) base-local)
          instrs3 (emit-root-push-drop instrs2 base-local)
          instrs4 (emit-to instrs3 (op-map-new) record-local)
          instrs5 (emit-to instrs4 (op-local-set) record-local)
          instrs6 (emit-root-push-drop instrs5 record-local)
          base-value-instrs (emit-to (vector-new 2) (op-local-get) base-local)]
          (do
            (root_push base-value-instrs)
            (let [base-marker-instrs
                    (compile-record-get-with-fallback
                      env
                      (vector-new 2)
                      base-value-instrs
                      (record-nominal-type-key))]
              (do
                (root_push base-marker-instrs)
                (let [instrs7 (compile-record-map-field-instrs
                                env
                                instrs6
                                record-local
                                (record-update-base-key)
                                base-value-instrs)
                  instrs8 (compile-record-map-field-instrs
                            env
                            instrs7
                            record-local
                            (record-nominal-type-key)
                            base-marker-instrs)]
                (do
                  (root_set instrs-slot instrs8)
                  (let [with-fields
                          (compile-recordlit-fields-with-ftable
                            node
                            env
                            ftable
                            0
                            (vector-get node 2)
                            instrs8
                            record-local)]
                    (do
                      (root_set instrs-slot with-fields)
                      (let [instrs9 (emit-to with-fields (op-local-get) record-local)
                        instrs10 (emit-root-pop-drop instrs9)
                        result (emit-root-pop-drop instrs10)]
                        (do
                          (root_set instrs-slot result)
                          (root_pop)
                          (root_pop)
                          (root_pop)
                          (root_pop)
                          (root_pop)
                          (root_pop)
                          (root_pop)
                          result))))))))))))))

(defn compile-recordlit-fields-with-source [node source env ftable idx count instrs record-local data-ref]
  (if (>= idx count)
    instrs
    (do
      (root_push node)
      (root_push source)
      (root_push env)
      (root_push ftable)
      (let [instrs-slot (root_push instrs)]
        (do
          (root_push data-ref)
          (let [field-hash (vector-get node (+ 3 (* idx 2)))
            value-node (vector-get node (+ 4 (* idx 2)))]
            (do
              (root_push value-node)
              (let [value-instrs (compile-expr-with-source value-node source env ftable (vector-new 8) data-ref)]
                (do
                  (root_push value-instrs)
                  (let [next-instrs (compile-record-map-field-instrs env instrs record-local field-hash value-instrs)]
                    (do
                      (root_set instrs-slot next-instrs)
                      (root_pop)
                      (root_pop)
                      (let [result (compile-recordlit-fields-with-source node source env ftable (+ idx 1) count next-instrs record-local data-ref)]
                        (do
                          (root_set instrs-slot result)
                          (root_pop)
                          (root_pop)
                          (root_pop)
                          (root_pop)
                          (root_pop)
                          (root_pop)
                          result)))))))))))))

(defn compile-recordlit-with-source [node source env ftable instrs data-ref]
  (do
    (root_push node)
    (root_push source)
    (root_push env)
    (root_push ftable)
    (let [instrs-slot (root_push instrs)
      record-local (max-root-temp-base1 env instrs)]
      (do
        (root_push data-ref)
        (let [instrs1 (emit-to instrs (op-map-new) record-local)
          instrs2 (emit-to instrs1 (op-local-set) record-local)
          instrs3 (emit-root-push-drop instrs2 record-local)]
          (do
            (root_set instrs-slot instrs3)
            (let [with-marker
                    (compile-record-nominal-marker
                      env
                      instrs3
                      record-local
                      (record-literal-nominal-marker-for-compiler node ftable))]
              (do
                (root_push with-marker)
                (let [with-fields (compile-recordlit-fields-with-source node source env ftable 0 (vector-get node 2) with-marker record-local data-ref)]
                  (do
                    (root_set instrs-slot with-fields)
                    (let [instrs4 (emit-to with-fields (op-local-get) record-local)
                      result (emit-root-pop-drop instrs4)]
                      (do
                        (root_set instrs-slot result)
                        (root_pop)
                        (root_pop)
                        (root_pop)
                        (root_pop)
                        (root_pop)
                        (root_pop)
                        (root_pop)
                        result))))))))))))

(defn compile-recordlit-fields-with-ftable [node env ftable idx count instrs record-local]
  (if (>= idx count)
    instrs
    (do
      (root_push node)
      (root_push env)
      (root_push ftable)
      (let [instrs-slot (root_push instrs)
        field-hash (vector-get node (+ 3 (* idx 2)))
        value-node (vector-get node (+ 4 (* idx 2)))]
        (do
          (root_push value-node)
          (let [value-instrs (compile-expr-with-ftable value-node env ftable (vector-new 8))]
            (do
              (root_push value-instrs)
              (let [next-instrs (compile-record-map-field-instrs env instrs record-local field-hash value-instrs)]
                (do
                  (root_set instrs-slot next-instrs)
                  (root_pop)
                  (root_pop)
                  (let [result (compile-recordlit-fields-with-ftable node env ftable (+ idx 1) count next-instrs record-local)]
                    (do
                      (root_set instrs-slot result)
                      (root_pop)
                      (root_pop)
                      (root_pop)
                      (root_pop)
                      result)))))))))))

(defn compile-recordlit-with-ftable [node env ftable instrs]
  (do
    (root_push node)
    (root_push env)
    (root_push ftable)
    (let [instrs-slot (root_push instrs)
      record-local (max-root-temp-base1 env instrs)]
      (do
        (let [instrs1 (emit-to instrs (op-map-new) record-local)
          instrs2 (emit-to instrs1 (op-local-set) record-local)
          instrs3 (emit-root-push-drop instrs2 record-local)]
          (do
            (root_set instrs-slot instrs3)
            (let [with-marker
                    (compile-record-nominal-marker
                      env
                      instrs3
                      record-local
                      (record-literal-nominal-marker-for-compiler node ftable))]
              (do
                (root_push with-marker)
                (let [with-fields (compile-recordlit-fields-with-ftable node env ftable 0 (vector-get node 2) with-marker record-local)]
                  (do
                    (root_set instrs-slot with-fields)
                    (let [instrs4 (emit-to with-fields (op-local-get) record-local)
                      result (emit-root-pop-drop instrs4)]
                      (do
                        (root_set instrs-slot result)
                        (root_pop)
                        (root_pop)
                        (root_pop)
                        (root_pop)
                        (root_pop)
                        result))))))))))))

(defn compile-fieldaccess-with-source [node source env ftable instrs data-ref]
  (let [record-expr (vector-get node 1)
    field-hash (vector-get node 2)
    record-instrs (compile-expr-with-source record-expr source env ftable (vector-new 8) data-ref)]
    (do
      (root_push record-instrs)
      (let [result (compile-record-get-with-fallback env instrs record-instrs field-hash)]
        (do
          (root_pop)
          result)))))

(defn compile-fieldaccess-with-ftable [node env ftable instrs]
  (let [record-expr (vector-get node 1)
    field-hash (vector-get node 2)
    record-instrs (compile-expr-with-ftable record-expr env ftable (vector-new 8))]
    (do
      (root_push record-instrs)
      (let [result (compile-record-get-with-fallback env instrs record-instrs field-hash)]
        (do
          (root_pop)
          result)))))

(defn compile-expr-with-source-normal-setup-diagnostic [node source env ftable instrs data-ref]
  (let [node-slot (root_push node)
    source-slot (root_push source)
    env-slot (root_push env)
    ftable-slot (root_push ftable)
    instrs-slot (root_push instrs)
    data-slot (root_push data-ref)
    tag (vector-get node 0)]
    (do
      (print 9000000236)
      (print tag)
      (print (vector-length node))
      (print (vector-length instrs))
      (print (vector-length (ref-get data-ref)))
      (let [result (compile-expr-with-source-dispatch-normal-setup-diagnostic node source env ftable instrs data-ref)]
        (do
          (root_push result)
          (print 9000000242)
          (print tag)
          (print (vector-length result))
          (print (vector-length (ref-get data-ref)))
          (root_pop)
          (root_pop)
          (root_pop)
          (root_pop)
          (root_pop)
          (root_pop)
          (root_pop)
          result)))))
(defn compile-if-with-source-normal-setup-diagnostic [node source env ftable instrs data-ref]
  (let [cond-expr (vector-get node 1)
    then-expr (vector-get node 2)
    else-expr (vector-get node 3)]
    (do
      (root_push cond-expr)
      (root_push then-expr)
      (root_push else-expr)
      (print 9000000360)
      (print (vector-length node))
      (print (vector-get cond-expr 0))
      (print (vector-get then-expr 0))
      (print (vector-get else-expr 0))
      (print (vector-length instrs))
      (print (vector-length (ref-get data-ref)))
      (let [instrs1 (compile-expr-with-source-normal-setup-diagnostic cond-expr source env ftable instrs data-ref)]
        (do
          (root_push instrs1)
          (print 9000000361)
          (print (vector-length instrs1))
          (print (vector-length (ref-get data-ref)))
          (let [instrs2 (emit-to instrs1 41 0)]
            (do
              (root_push instrs2)
              (print 9000000362)
              (print (vector-length instrs2))
              (print (vector-length (ref-get data-ref)))
              (let [instrs3 (compile-expr-with-source-normal-setup-diagnostic then-expr source env ftable instrs2 data-ref)]
                (do
                  (root_push instrs3)
                  (print 9000000363)
                  (print (vector-length instrs3))
                  (print (vector-length (ref-get data-ref)))
                  (let [instrs4 (emit-to instrs3 79 0)]
                    (do
                      (root_push instrs4)
                      (print 9000000364)
                      (print (vector-length instrs4))
                      (print (vector-length (ref-get data-ref)))
                      (let [instrs5 (compile-expr-with-source-normal-setup-diagnostic else-expr source env ftable instrs4 data-ref)]
                        (do
                          (root_push instrs5)
                          (print 9000000365)
                          (print (vector-length instrs5))
                          (print (vector-length (ref-get data-ref)))
                          (let [result (emit-to instrs5 43 0)]
                            (do
                              (root_push result)
                              (print 9000000366)
                              (print (vector-length result))
                              (print (vector-length (ref-get data-ref)))
                              (root_pop)
                              (root_pop)
                              (root_pop)
                              (root_pop)
                              (root_pop)
                              (root_pop)
                              (root_pop)
                              (root_pop)
                              (root_pop)
                              result)))))))))))))))
(defn compile-expr-with-source-dispatch-normal-setup-diagnostic [node source env ftable instrs data-ref]
  (let [tag (vector-get node 0)]
    (if (= tag 7)
      (compile-let-with-source-normal-setup-diagnostic node source env ftable instrs data-ref)
      (if (= tag 9)
        (compile-do-with-source-normal-setup-diagnostic node source env ftable instrs data-ref)
        (if (= tag 5)
          (compile-apply-with-source-normal-setup-diagnostic node source env ftable instrs data-ref)
          (if (= tag 6)
            (compile-if-with-source-normal-setup-diagnostic node source env ftable instrs data-ref)
            (compile-expr-with-source-dispatch node source env ftable instrs data-ref)))))))
(defn compile-defn-with-source [node source ftable data-ref]
  (do
    (root_push node)
    (root_push source)
    (root_push ftable)
    (root_push data-ref)
    (let [param-count (vector-get node 2)
      body-idx (+ 3 param-count)
      env (bind-node-params node 3 0 param-count (env-new) 1)]
      (do
        (root_push env)
        (let [body-expr (vector-get node body-idx)]
          (do
            (root_push body-expr)
            (let [instrs0 (vector-new 8)]
              (do
                (root_push instrs0)
                (let [result (compile-expr-with-source body-expr source env ftable instrs0 data-ref)]
                  (do
                    (root_push result)
                    (root_pop)
                    (root_pop)
                    (root_pop)
                    (root_pop)
                    (root_pop)
                    (root_pop)
                    (root_pop)
                    (root_pop)
                    result))))))))))
(defn compile-defn-function-with-source [node source ftable data-ref]
  (do
    (root_push node)
    (root_push source)
    (root_push ftable)
    (root_push data-ref)
    (let [source-ir (compile-defn-with-source node source ftable data-ref)]
      (do
        (root_push source-ir)
        (let [ir (if (> (vector-length source-ir) 0) source-ir (compile-defn-with-ftable node ftable))]
          (do
            (root_push ir)
            (let [local-max (max-local-slot ir 0 (vector-length ir) 0)
              final-param-count (vector-get node 2)
              local-count (if (> local-max final-param-count) (- local-max final-param-count) 0)
              result (make-function-meta final-param-count local-count ir)]
              (do
                (root_push result)
                (root_pop)
                (root_pop)
                (root_pop)
                (root_pop)
                (root_pop)
                (root_pop)
                (root_pop)
                result))))))))
(defn compile-defn-with-source-normal-setup-diagnostic [node source ftable data-ref]
  (do
    (root_push node)
    (root_push source)
    (root_push ftable)
    (root_push data-ref)
    (let [param-count (vector-get node 2)
      body-idx (+ 3 param-count)
      env (bind-node-params node 3 0 param-count (env-new) 1)]
      (do
        (root_push env)
        (let [body-expr (vector-get node body-idx)]
          (do
            (root_push body-expr)
            (print 9000000233)
            (print param-count)
            (print body-idx)
            (print (vector-length body-expr))
            (print (vector-get body-expr 0))
            (print (vector-length (ref-get data-ref)))
            (print 9000000234)
            (print param-count)
            (print body-idx)
            (print (vector-length body-expr))
            (print (vector-length (ref-get data-ref)))
            (let [instrs0 (vector-new 8)]
              (do
                (root_push instrs0)
                (let [result (compile-expr-with-source-normal-setup-diagnostic body-expr source env ftable instrs0 data-ref)]
                  (do
                    (root_push result)
                    (print 9000000235)
                    (print param-count)
                    (print (vector-length result))
                    (print (vector-length (ref-get data-ref)))
                    (root_pop)
                    (root_pop)
                    (root_pop)
                    (root_pop)
                    (root_pop)
                    (root_pop)
                    (root_pop)
                    (root_pop)
                    result))))))))))
(defn compile-defn-function-with-source-normal-setup-diagnostic [node source ftable data-ref]
  (do
    (root_push node)
    (root_push source)
    (root_push ftable)
    (root_push data-ref)
    (print 9000000229)
    (print (vector-get node 2))
    (print (vector-length node))
    (print (vector-length (ref-get data-ref)))
    (let [source-ir (compile-defn-with-source-normal-setup-diagnostic node source ftable data-ref)]
      (do
        (root_push source-ir)
        (print 9000000230)
        (print (vector-get node 2))
        (print (vector-length source-ir))
        (print (vector-length (ref-get data-ref)))
        (let [ir (if (> (vector-length source-ir) 0) source-ir (compile-defn-with-ftable node ftable))]
          (do
            (root_push ir)
            (print 9000000231)
            (print (vector-get node 2))
            (print (vector-length ir))
            (print (vector-length (ref-get data-ref)))
            (let [local-max (max-local-slot ir 0 (vector-length ir) 0)
              final-param-count (vector-get node 2)
              local-count (if (> local-max final-param-count) (- local-max final-param-count) 0)
              result (make-function-meta final-param-count local-count ir)]
              (do
                (root_push result)
                (print 9000000232)
                (print final-param-count)
                (print local-count)
                (print (vector-length ir))
                (print (vector-length (ref-get data-ref)))
                (root_pop)
                (root_pop)
                (root_pop)
                (root_pop)
                (root_pop)
                (root_pop)
                (root_pop)
                result))))))))
(defn compile-defn-functions-with-source [decls idx n source ftable data-ref functions]
  (compile-source-defn-functions-chunked decls idx n source ftable data-ref functions))
(defn source-program-functions-base [src decls base-idx]
  (do
    (root_push src)
    (root_push decls)
    (let [n (vector-length decls)
      prelude (record-prelude-chunked decls 0 n (ftable-new) base-idx (vector-new 8))]
      (do
        (root_push prelude)
        (let [prelude-ftable (vector-get prelude 2)
          prelude-func-idx (vector-get prelude 3)
          prelude-functions (vector-get prelude 4)]
          (do
            (root_push prelude-ftable)
            (root_push prelude-functions)
            (let [pass1 (register-defns-chunked decls 0 n prelude-ftable prelude-func-idx)]
              (do
                (root_push pass1)
                (let [ftable (vector-get pass1 2)
                  data-ref (ref-new (standalone-data-layout-prefix))]
                  (do
                    (root_push ftable)
                    (root_push data-ref)
                    (let [functions (compile-source-defn-functions-chunked decls 0 n src ftable data-ref prelude-functions)]
                      (do
                        (root_push functions)
                        (let [data (ref-get data-ref)]
                          (do
                            (root_push data)
                            (let [payload1 (push-object-vector (vector-new 3) ftable)]
                              (do
                                (root_push payload1)
                                (let [payload2 (push-object-vector payload1 functions)]
                                  (do
                                    (root_push payload2)
                                    (let [payload3 (push-object-vector payload2 data)]
                                      (do
                                        (root_push payload3)
                                        (root_pop)
                                        (root_pop)
                                        (root_pop)
                                        (root_pop)
                                        (root_pop)
                                        (root_pop)
                                        (root_pop)
                                        (root_pop)
                                        (root_pop)
                                        (root_pop)
                                        (root_pop)
                                        (root_pop)
                                        (root_pop)
                                        payload3))))))))))))))))))))
(defn compile-program-functions-with-source [src decls]
  (do
    (root_push src)
    (root_push decls)
    (let [n (vector-length decls)
      prelude (record-prelude-chunked decls 0 n (ftable-new) 11 (vector-new 8))]
      (do
        (root_push prelude)
        (let [prelude-ftable (vector-get prelude 2)
          prelude-func-idx (vector-get prelude 3)
          prelude-functions (vector-get prelude 4)]
          (do
            (root_push prelude-ftable)
            (root_push prelude-functions)
            (let [pass1 (register-defns-chunked decls 0 n prelude-ftable prelude-func-idx)]
              (do
                (root_push pass1)
                (let [ftable (vector-get pass1 2)
                  data-ref (ref-new (vector-new 8))]
                  (do
                    (root_push ftable)
                    (root_push data-ref)
                    (let [functions (compile-source-defn-functions-chunked decls 0 n src ftable data-ref prelude-functions)]
                      (do
                        (root_push functions)
                        (let [data (ref-get data-ref)]
                          (do
                            (root_push data)
                            (let [payload1 (push-object-vector (vector-new 3) ftable)]
                              (do
                                (root_push payload1)
                                (let [payload2 (push-object-vector payload1 functions)]
                                  (do
                                    (root_push payload2)
                                    (let [payload3 (push-object-vector payload2 data)]
                                      (do
                                        (root_push payload3)
                                        (root_pop)
                                        (root_pop)
                                        (root_pop)
                                        (root_pop)
                                        (root_pop)
                                        (root_pop)
                                        (root_pop)
                                        (root_pop)
                                        (root_pop)
                                        (root_pop)
                                        (root_pop)
                                        (root_pop)
                                        (root_pop)
                                        payload3))))))))))))))))))))
(defn compile-program-functions-with-source-base [src decls base-idx]
  (source-program-functions-base src decls base-idx))
(defn standalone-preview1-opcode-supported? [opcode]
  (if (= opcode 1)
    true
    (if (= opcode 10)
      true
      (if (= opcode 11)
        true
        (if (and (>= opcode 20) (<= opcode 23))
          true
          (if (= opcode 28)
            true
            (if (and (>= opcode 30) (<= opcode 35))
              true
              (if (and (>= opcode 40) (<= opcode 44))
                true
                (if (and (>= opcode 50) (<= opcode 63))
                  true
                  (if (and (>= opcode 65) (<= opcode 67))
                    true
                    (if (and (>= opcode 69) (<= opcode 70))
                      true
                      (if (and (>= opcode 71) (<= opcode 72))
                      true
                      (if (and (>= opcode 74) (<= opcode 85))
                      true
                      (if (if (= opcode 64) true (= opcode 73))
                        true
                        (if (= opcode 86) true (if (= opcode 87) true (if (= opcode 89) true (if (= opcode 90) true false))))))))))))))))))
(defn standalone-preview1-first-unsupported-ir-opcode [ir idx count]
  (if (>= idx count)
    -1
    (let [instr (vector-get ir idx)
      opcode (vector-get instr 0)]
      (if (standalone-preview1-opcode-supported? opcode)
        (standalone-preview1-first-unsupported-ir-opcode ir (+ idx 1) count)
        opcode))))
(defn standalone-preview1-first-unsupported-function-opcode [functions idx count]
  (if (>= idx count)
    -1
    (let [func-meta (vector-get functions idx)
      opcode (standalone-preview1-first-unsupported-ir-opcode
        (function-meta-ir func-meta)
        0
        (vector-length (function-meta-ir func-meta)))]
      (if (>= opcode 0)
        opcode
        (standalone-preview1-first-unsupported-function-opcode functions (+ idx 1) count)))))
(defn standalone-preview1-first-unsupported-opcode [functions]
  (standalone-preview1-first-unsupported-function-opcode functions 0 (vector-length functions)))
(defn compile-program-with-source [src decls]
  (let [pair (compile-program-functions-with-source src decls)
    ftable (vector-get pair 0)
    functions (vector-get pair 1)
    data (vector-get pair 2)
    ir-list (collect-function-irs functions 0 (vector-length functions) (vector-new 8))]
    (let [payload1 (push-object-vector (vector-new 3) ftable)]
      (do
        (root_push payload1)
        (let [payload2 (push-object-vector payload1 ir-list)]
          (do
            (root_push payload2)
            (let [payload3 (push-object-vector payload2 data)]
              (do
                (root_pop)
                (root_pop)
                payload3))))))))
(defn compile-let-with-ftable [node env ftable instrs]
  (compile-let-with-ftable-impl node env ftable instrs))

(defn compile-expr-with-ftable-dispatch [node env ftable instrs]
  (compile-expr-with-ftable-dispatch-impl node env ftable instrs))

(defn compile-expr-with-ftable [node env ftable instrs]
  (do
    (root_push node)
    (root_push env)
    (root_push ftable)
    (root_push instrs)
    (let [result (compile-expr-with-ftable-dispatch node env ftable instrs)]
      (do
        (root_pop)
        (root_pop)
        (root_pop)
        (root_pop)
        result))))

(defn compile-expr [node env instrs] (compile-expr-with-ftable node env (ftable-new) instrs))
(defn compile-defn-with-ftable [node ftable]
  (do
    (root_push node)
    (let [param-count (vector-get node 2)
      env (bind-node-params node 3 0 param-count (env-new) 1)
      body-idx (+ 3 param-count)]
      (do
        (root_push env)
        (let [result (compile-expr-with-ftable (vector-get node body-idx) env ftable (vector-new 8))]
          (do
            (root_pop)
            (root_pop)
            result))))))

(defn compile-defn [node] (compile-defn-with-ftable node (ftable-new)))
(defn continue-compile-defn-functions-step-with-source [decls n source ftable data-ref state]
  (if (= (vector-get state 0) 1)
    state
    (let [decls-slot (root_push decls)
      source-slot (root_push source)
      ftable-slot (root_push ftable)
      data-slot (root_push data-ref)
      state-slot (root_push state)]
      (let [next-idx (vector-get state 1)
        next-functions (vector-get state 2)]
        (do
          (root_push next-functions)
          (let [result (compile-defn-functions-step-with-source decls next-idx n source ftable data-ref next-functions)]
            (do
              (root_push result)
              (root_set state-slot result)
              (root_pop)
              (root_pop)
              (root_pop)
              (root_pop)
              (root_pop)
              (root_pop)
              (root_pop)
              result)))))))

(defn continue-compile-defn-functions-step-times-with-source [decls n source ftable data-ref remaining state]
  (if (= remaining 0)
    state
    (if (= (vector-get state 0) 1)
      state
      (let [decls-slot (root_push decls)
        source-slot (root_push source)
        ftable-slot (root_push ftable)
        data-slot (root_push data-ref)
        state-slot (root_push state)]
        (let [next-state (continue-compile-defn-functions-step-with-source decls n source ftable data-ref state)]
          (do
            (root_push next-state)
            (let [result (continue-compile-defn-functions-step-times-with-source decls n source ftable data-ref (- remaining 1) next-state)]
              (do
                (root_push result)
                (root_set state-slot result)
                (root_pop)
                (root_pop)
                (root_pop)
                (root_pop)
                (root_pop)
                (root_pop)
                (root_pop)
                result))))))))

(defn compile-defn-functions-step-8-with-source [decls idx n source ftable data-ref functions]
  (let [decls-slot (root_push decls)
    source-slot (root_push source)
    ftable-slot (root_push ftable)
    data-slot (root_push data-ref)
    functions-slot (root_push functions)]
    (let [state (compile-defn-functions-step-with-source decls idx n source ftable data-ref functions)]
      (do
        (root_push state)
        (let [result (continue-compile-defn-functions-step-times-with-source decls n source ftable data-ref 7 state)]
          (do
            (root_push result)
            (root_set decls-slot result)
            (root_set functions-slot result)
            (root_pop)
            (root_pop)
            (root_pop)
            (root_pop)
            (root_pop)
            (root_pop)
            (root_pop)
            result))))))

(defn continue-compile-defn-functions-step-8-with-source [decls n source ftable data-ref state]
  (if (= (vector-get state 0) 1)
    state
    (do
      (root_push decls)
      (root_push source)
      (root_push ftable)
      (root_push data-ref)
      (root_push state)
      (let [result (compile-defn-functions-step-8-with-source decls (vector-get state 1) n source ftable data-ref (vector-get state 2))]
        (do
          (root_pop)
          (root_pop)
          (root_pop)
          (root_pop)
          (root_pop)
          result)))))

(defn compile-defn-functions-step-64-with-source [decls idx n source ftable data-ref functions]
  (let [decls-slot (root_push decls)
    source-slot (root_push source)
    ftable-slot (root_push ftable)
    data-slot (root_push data-ref)
    functions-slot (root_push functions)]
    (let [state (compile-defn-functions-step-with-source decls idx n source ftable data-ref functions)]
      (do
        (root_push state)
        (let [result (continue-compile-defn-functions-step-times-with-source decls n source ftable data-ref 63 state)]
          (do
            (root_push result)
            (root_set decls-slot result)
            (root_set functions-slot result)
            (root_pop)
            (root_pop)
            (root_pop)
            (root_pop)
            (root_pop)
            (root_pop)
            (root_pop)
            result))))))

(defn continue-compile-defn-functions-step-64-with-source [decls n source ftable data-ref state]
  (if (= (vector-get state 0) 1)
    state
    (do
      (root_push decls)
      (root_push source)
      (root_push ftable)
      (root_push data-ref)
      (let [state-slot (root_push state)]
        (let [next-idx (vector-get state 1)
          next-functions (vector-get state 2)]
          (do
            (root_push next-functions)
            (let [next-state (compile-defn-functions-step-64-with-source decls next-idx n source ftable data-ref next-functions)]
              (do
                (root_pop)
                (root_push next-state)
                (let [result (continue-compile-defn-functions-step-64-with-source decls n source ftable data-ref next-state)]
                  (do
                    (root_push result)
                    (root_set state-slot result)
                    (root_pop)
                    (root_pop)
                    (root_pop)
                    (root_pop)
                    (root_pop)
                    (root_pop)
                    (root_pop)
                    result))))))))))

(defn compile-source-defn-functions-chunked [decls idx n source ftable data-ref functions]
  (let [state (compile-defn-functions-step-64-with-source decls idx n source ftable data-ref functions)]
    (do
      (let [state-slot (root_push state)]
        (do
          (let [result (continue-compile-defn-functions-step-64-with-source decls n source ftable data-ref state)]
            (do
              (root_push result)
              (let [functions-result (vector-get result 2)]
                (do
                  (root_push functions-result)
                  (root_set state-slot functions-result)
                  (root_pop)
                  (root_pop)
                  (root_pop)
                  functions-result)))))))))
(defn print-source-defn-normal-setup-finish-shape [idx functions compiled-fn result data-ref]
  (do
    (root_push functions)
    (root_push compiled-fn)
    (root_push result)
    (root_push data-ref)
    (print 9000000195)
    (print idx)
    (print (vector-length functions))
    (print (vector-length compiled-fn))
    (print (vector-get result 0))
    (print (vector-get result 1))
    (print (vector-length (vector-get result 2)))
    (print (vector-length (ref-get data-ref)))
    (root_pop)
    (root_pop)
    (root_pop)
    (root_pop)
    0))
(defn print-source-defn-normal-setup-entry-shape [idx n functions data-ref]
  (do
    (root_push functions)
    (root_push data-ref)
    (print 9000000215)
    (print idx)
    (print n)
    (print (vector-length functions))
    (print (vector-length (ref-get data-ref)))
    (root_pop)
    (root_pop)
    0))
(defn print-source-defn-normal-setup-entry-body-shape [idx decls probe-idx functions data-ref]
  (do
    (root_push decls)
    (root_push functions)
    (root_push data-ref)
    (let [decls-len (vector-length decls)
      decl (if (> decls-len probe-idx) (vector-get decls probe-idx) (vector-new 0))]
      (do
        (root_push decl)
        (let [decl-len (vector-length decl)
          param-count (if (> decl-len 2) (vector-get decl 2) -1)
          body-idx (+ 3 param-count)
          body-expr (if (> decl-len body-idx) (vector-get decl body-idx) (vector-new 0))]
          (do
            (root_push body-expr)
            (print 9000000369)
            (print idx)
            (print probe-idx)
            (print decls-len)
            (print decl-len)
            (print param-count)
            (print body-idx)
            (print (if (> decl-len body-idx) (vector-get body-expr 0) -1))
            (print (if (> decl-len body-idx) (vector-length body-expr) -1))
            (print (vector-length functions))
            (print (vector-length (ref-get data-ref)))
            (root_pop)
            (root_pop)
            (root_pop)
            (root_pop)
            (root_pop)
            0))))))
(defn print-source-defn-normal-setup-ref-after-write-shape [idx functions state-ref data-ref]
  (do
    (root_push functions)
    (root_push state-ref)
    (root_push data-ref)
    (let [state (ref-get state-ref)]
      (do
        (root_push state)
        (print 9000000222)
        (print idx)
        (print (vector-length functions))
        (print state)
        (print (vector-length state))
        (print (vector-get state 0))
        (print (vector-get state 1))
        (print (vector-length (vector-get state 2)))
        (print (vector-length (ref-get data-ref)))
        (root_pop)
        (root_pop)
        (root_pop)
        (root_pop)
        0))))
(defn print-source-defn-normal-setup-result-after-root-shape [idx functions result data-ref]
  (do
    (root_push functions)
    (root_push result)
    (root_push data-ref)
    (print 9000000223)
    (print idx)
    (print (vector-length functions))
    (print result)
    (print (vector-length result))
    (print (vector-get result 0))
    (print (vector-get result 1))
    (print (vector-length (vector-get result 2)))
    (print (vector-length (ref-get data-ref)))
    (root_pop)
    (root_pop)
    (root_pop)
    0))
(defn print-source-defn-normal-setup-skip-shape [idx functions result data-ref]
  (do
    (root_push functions)
    (root_push result)
    (root_push data-ref)
    (print 9000000216)
    (print idx)
    (print (vector-length functions))
    (print (vector-get result 0))
    (print (vector-get result 1))
    (print (vector-length (vector-get result 2)))
    (print (vector-length (ref-get data-ref)))
    (root_pop)
    (root_pop)
    (root_pop)
    0))
(defn compile-defn-functions-step-with-source-normal-setup-diagnostic [decls idx n source ftable data-ref functions]
  (if (>= idx n)
    (make-compile-step-state 1 idx functions)
    (let [decls-slot (root_push decls)
      source-slot (root_push source)
      ftable-slot (root_push ftable)
      data-slot (root_push data-ref)
      functions-slot (root_push functions)
      decl (vector-get decls idx)]
      (if (= (vector-get decl 0) 20)
        (do
          (root_push decl)
          (print 9000000226)
          (print idx)
          (print (vector-length functions))
          (print (vector-get decl 0))
          (print (vector-length (ref-get data-ref)))
          (let [decl-len (vector-length decl)
            param-count (vector-get decl 2)
            body-idx (+ 3 param-count)
            body-expr (if (> decl-len body-idx) (vector-get decl body-idx) (vector-new 0))]
            (do
              (root_push body-expr)
              (print 9000000367)
              (print idx)
              (print (vector-length functions))
              (print decl-len)
              (print param-count)
              (print body-idx)
              (print (if (> decl-len body-idx) (vector-get body-expr 0) -1))
              (print (if (> decl-len body-idx) (vector-length body-expr) -1))
              (print (vector-length (ref-get data-ref)))
              (root_pop)))
          (let [compiled-fn (compile-defn-function-with-source-normal-setup-diagnostic decl source ftable data-ref)]
            (do
              (root_push compiled-fn)
              (print 9000000227)
              (print idx)
              (print (vector-length functions))
              (print (vector-length compiled-fn))
              (print (vector-length (ref-get data-ref)))
              (let [next-functions (push-object-vector functions compiled-fn)]
                (do
                  (root_push next-functions)
                  (print 9000000228)
                  (print idx)
                  (print (vector-length functions))
                  (print (vector-length compiled-fn))
                  (print (vector-length next-functions))
                  (print (vector-length (ref-get data-ref)))
                  (let [defn-result (make-compile-step-state 0 (+ idx 1) next-functions)]
                    (do
                      (root_push defn-result)
                      (print-source-defn-normal-setup-finish-shape idx functions compiled-fn defn-result data-ref)
                      (root_set functions-slot defn-result)
                      (root_pop)
                      (root_pop)
                      (root_pop)
                      (root_pop)
                      (root_pop)
                      (root_pop)
                      (root_pop)
                      (root_pop)
                      (root_pop)
                      defn-result)))))))
        (do
          (let [skip-state-ref (ref-new 0)]
            (do
              (root_push skip-state-ref)
              (print 9000000225)
              (print 0)
              (print idx)
              (print skip-state-ref)
              (print (vector-length functions))
              (print (vector-length (ref-get data-ref)))
              (write-compile-step-state-ref-normal-setup-diagnostic skip-state-ref 0 (+ idx 1) functions)
              (print 9000000225)
              (print 1)
              (print idx)
              (print skip-state-ref)
              (print (vector-length functions))
              (print (vector-length (ref-get data-ref)))
              (print-source-defn-normal-setup-ref-after-write-shape idx functions skip-state-ref data-ref)
              (print 9000000225)
              (print 2)
              (print idx)
              (print skip-state-ref)
              (print (vector-length functions))
              (print (vector-length (ref-get data-ref)))
              (let [skip-result (ref-get skip-state-ref)]
                (do
                  (print 9000000224)
                  (print idx)
                  (print (vector-length functions))
                  (print skip-result)
                  (print (vector-length skip-result))
                  (print (vector-get skip-result 0))
                  (print (vector-get skip-result 1))
                  (print (vector-length (vector-get skip-result 2)))
                  (print (vector-length (ref-get data-ref)))
                  (root_push skip-result)
                  (print-source-defn-normal-setup-result-after-root-shape idx functions skip-result data-ref)
                  (print-source-defn-normal-setup-skip-shape idx functions skip-result data-ref)
                  (root_set functions-slot skip-result)
                  (root_pop)
                  (root_pop)
                  (root_pop)
                  (root_pop)
                  (root_pop)
                  (root_pop)
                  (root_pop)
                  skip-result)))))))))
(defn continue-compile-defn-functions-step-with-source-normal-setup-diagnostic [decls n source ftable data-ref state]
  (if (= (vector-get state 0) 1)
    state
    (let [decls-slot (root_push decls)
      source-slot (root_push source)
      ftable-slot (root_push ftable)
      data-slot (root_push data-ref)
      state-slot (root_push state)]
      (let [next-idx (vector-get state 1)
        next-functions (vector-get state 2)]
        (do
          (root_push next-functions)
          (let [result (compile-defn-functions-step-with-source-normal-setup-diagnostic decls next-idx n source ftable data-ref next-functions)]
            (do
              (root_push result)
              (root_set state-slot result)
              (root_pop)
              (root_pop)
              (root_pop)
              (root_pop)
              (root_pop)
              (root_pop)
              (root_pop)
              result)))))))

(defn continue-compile-defn-functions-step-times-with-source-normal-setup-diagnostic [decls n source ftable data-ref remaining state]
  (if (= remaining 0)
    state
    (if (= (vector-get state 0) 1)
      state
      (let [decls-slot (root_push decls)
        source-slot (root_push source)
        ftable-slot (root_push ftable)
        data-slot (root_push data-ref)
        state-slot (root_push state)]
        (let [next-state (continue-compile-defn-functions-step-with-source-normal-setup-diagnostic decls n source ftable data-ref state)]
          (do
            (root_push next-state)
            (let [result (continue-compile-defn-functions-step-times-with-source-normal-setup-diagnostic decls n source ftable data-ref (- remaining 1) next-state)]
              (do
                (root_push result)
                (root_set state-slot result)
                (root_pop)
                (root_pop)
                (root_pop)
                (root_pop)
                (root_pop)
                (root_pop)
                (root_pop)
                result))))))))
(defn compile-defn-functions-step-64-with-source-normal-setup-diagnostic [decls idx n source ftable data-ref functions]
  (let [decls-slot (root_push decls)
    source-slot (root_push source)
    ftable-slot (root_push ftable)
    data-slot (root_push data-ref)
    functions-slot (root_push functions)]
    (let [state (compile-defn-functions-step-with-source-normal-setup-diagnostic decls idx n source ftable data-ref functions)]
      (do
        (root_push state)
        (let [result (continue-compile-defn-functions-step-times-with-source-normal-setup-diagnostic decls n source ftable data-ref 63 state)]
          (do
            (root_push result)
            (root_set decls-slot result)
            (root_set functions-slot result)
            (root_pop)
            (root_pop)
            (root_pop)
            (root_pop)
            (root_pop)
            (root_pop)
            (root_pop)
            result))))))

(defn continue-compile-defn-functions-step-64-with-source-normal-setup-diagnostic [decls n source ftable data-ref state]
  (if (= (vector-get state 0) 1)
    state
    (do
      (root_push decls)
      (root_push source)
      (root_push ftable)
      (root_push data-ref)
      (let [state-slot (root_push state)]
        (let [next-idx (vector-get state 1)
          next-functions (vector-get state 2)]
          (do
            (root_push next-functions)
            (let [next-state (compile-defn-functions-step-64-with-source-normal-setup-diagnostic decls next-idx n source ftable data-ref next-functions)]
              (do
                (root_pop)
                (root_push next-state)
                (let [result (continue-compile-defn-functions-step-64-with-source-normal-setup-diagnostic decls n source ftable data-ref next-state)]
                  (do
                    (root_push result)
                    (root_set state-slot result)
                    (root_pop)
                    (root_pop)
                    (root_pop)
                    (root_pop)
                    (root_pop)
                    (root_pop)
                    (root_pop)
                    result))))))))))
(defn compile-source-defn-functions-chunked-normal-setup-diagnostic [decls idx n source ftable data-ref functions]
  (do
    (print-source-defn-normal-setup-entry-shape idx n functions data-ref)
    (print-source-defn-normal-setup-entry-body-shape idx decls 3 functions data-ref)
    (let [state (compile-defn-functions-step-64-with-source-normal-setup-diagnostic decls idx n source ftable data-ref functions)]
    (do
      (let [state-slot (root_push state)]
        (do
          (let [result (continue-compile-defn-functions-step-64-with-source-normal-setup-diagnostic decls n source ftable data-ref state)]
            (do
              (root_push result)
              (let [functions-result (vector-get result 2)]
                (do
                  (root_push functions-result)
                  (root_set state-slot functions-result)
                  (root_pop)
                  (root_pop)
                  (root_pop)
                  functions-result))))))))))
(defn continue-compile-let-chain-step-with-source [source ftable state data-ref]
  (do
    (root_push state)
    (if (= (vector-get state 0) 1)
      (do
        (root_pop)
        state)
      (do
        (root_push source)
        (root_push ftable)
        (root_push data-ref)
        (let [next-value (vector-get state 2)]
          (do
            (root_push next-value)
            (let [result
              (compile-let-chain-step-with-source
                (vector-get next-value 0)
                source
                (vector-get next-value 1)
                ftable
                (vector-get next-value 2)
                data-ref
                (vector-get state 1))]
              (do
                (root_push result)
                (root_pop)
                (root_pop)
                (root_pop)
                (root_pop)
                (root_pop)
                (root_pop)
                result))))))))
(defn compile-let-chain-step-8-with-source [node source env ftable instrs data-ref rooted-count]
  (let [step1 (compile-let-chain-step-with-source node source env ftable instrs data-ref rooted-count)
    step2 (continue-compile-let-chain-step-with-source source ftable step1 data-ref)
    step3 (continue-compile-let-chain-step-with-source source ftable step2 data-ref)
    step4 (continue-compile-let-chain-step-with-source source ftable step3 data-ref)
    step5 (continue-compile-let-chain-step-with-source source ftable step4 data-ref)
    step6 (continue-compile-let-chain-step-with-source source ftable step5 data-ref)
    step7 (continue-compile-let-chain-step-with-source source ftable step6 data-ref)
    step8 (continue-compile-let-chain-step-with-source source ftable step7 data-ref)]
    (do
      (root_push step8)
      (root_pop)
      step8)))
(defn continue-compile-let-chain-step-8-with-source [source ftable state data-ref]
  (do
    (root_push state)
    (if (= (vector-get state 0) 1)
      (do
        (root_pop)
        state)
      (do
        (root_push source)
        (root_push ftable)
        (root_push data-ref)
        (let [next-value (vector-get state 2)]
          (do
            (root_push next-value)
            (let [result
              (compile-let-chain-step-8-with-source
                (vector-get next-value 0)
                source
                (vector-get next-value 1)
                ftable
                (vector-get next-value 2)
                data-ref
                (vector-get state 1))]
              (do
                (root_push result)
                (root_pop)
                (root_pop)
                (root_pop)
                (root_pop)
                (root_pop)
                (root_pop)
                result))))))))
(defn compile-let-chain-step-64-with-source [node source env ftable instrs data-ref rooted-count]
  (let [step1 (compile-let-chain-step-8-with-source node source env ftable instrs data-ref rooted-count)
    step2 (continue-compile-let-chain-step-8-with-source source ftable step1 data-ref)
    step3 (continue-compile-let-chain-step-8-with-source source ftable step2 data-ref)
    step4 (continue-compile-let-chain-step-8-with-source source ftable step3 data-ref)
    step5 (continue-compile-let-chain-step-8-with-source source ftable step4 data-ref)
    step6 (continue-compile-let-chain-step-8-with-source source ftable step5 data-ref)
    step7 (continue-compile-let-chain-step-8-with-source source ftable step6 data-ref)
    step8 (continue-compile-let-chain-step-8-with-source source ftable step7 data-ref)]
    (do
      (root_push step8)
      (root_pop)
      step8)))
(defn compile-let-with-source [node source env ftable instrs data-ref]
  (compile-let-chain-with-source node source env ftable instrs data-ref 0))
(defn compile-let-chain-step-with-source-normal-setup-diagnostic [node source env ftable instrs data-ref rooted-count]
  (let [node-slot (root_push node)
    source-slot (root_push source)
    env-slot (root_push env)
    ftable-slot (root_push ftable)
    instrs-slot (root_push instrs)
    data-slot (root_push data-ref)
    name-hash (vector-get node 1)
    init-expr (vector-get node 2)
    init-root (alloc-root-needed init-expr)]
    (do
      (print 9000000238)
      (print (vector-length node))
      (print name-hash)
      (print (vector-get init-expr 0))
      (print (vector-length init-expr))
      (print (vector-length (ref-get data-ref)))
      (let [init-instrs (compile-expr-with-source-normal-setup-diagnostic init-expr source env ftable instrs data-ref)]
        (do
          (root_push init-instrs)
          (print 9000000239)
          (print (vector-length init-instrs))
          (print init-root)
          (print (vector-length (ref-get data-ref)))
          (let [body-expr-after-init (vector-get node 3)
            result (compile-let-chain-step-finish name-hash body-expr-after-init init-instrs env rooted-count init-root)]
            (do
              (root_push result)
              (print 9000000240)
              (print (vector-length result))
              (print (vector-get result 0))
              (print (vector-get result 1))
              (print (vector-length (ref-get data-ref)))
              (root_pop)
              (root_pop)
              (root_pop)
              (root_pop)
              (root_pop)
              (root_pop)
              (root_pop)
              (root_pop)
              result)))))))
(defn compile-let-chain-with-source-normal-setup-diagnostic [node source env ftable instrs data-ref rooted-count]
  (let [step (compile-let-chain-step-with-source-normal-setup-diagnostic node source env ftable instrs data-ref rooted-count)]
    (do
      (root_push step)
      (let [next-value (vector-get step 2)]
        (do
          (root_push next-value)
          (print 9000000241)
          (print (vector-get step 0))
          (print (vector-get step 1))
          (print (vector-length next-value))
          (print (vector-length (ref-get data-ref)))
          (if (= (vector-get step 0) 1)
            (let [body-expr (vector-get next-value 0)
              next-env (vector-get next-value 1)
              next-instrs (vector-get next-value 2)
              body-instrs (compile-expr-with-source-normal-setup-diagnostic body-expr source next-env ftable next-instrs data-ref)]
              (do
                (root_push body-instrs)
                (let [result (emit-root-pop-drops body-instrs (vector-get step 1))]
                  (do
                    (root_pop)
                    (root_pop)
                    (root_pop)
                    result))))
            (let [result
              (compile-let-chain-with-source
                (vector-get next-value 0)
                source
                (vector-get next-value 1)
                ftable
                (vector-get next-value 2)
                data-ref
                (vector-get step 1))]
              (do
                (root_pop)
                (root_pop)
                result))))))))
(defn compile-let-with-source-normal-setup-diagnostic [node source env ftable instrs data-ref]
  (do
    (print 9000000237)
    (print (vector-length node))
    (print (vector-length instrs))
    (print (vector-length (ref-get data-ref)))
    (compile-let-chain-with-source-normal-setup-diagnostic node source env ftable instrs data-ref 0)))
(defn compile-defn-function [node ftable]
  (do
    (root_push node)
    (root_push ftable)
    (let [ir (compile-defn-with-ftable node ftable)]
      (do
        (root_push ir)
        (let [local-max (max-local-slot ir 0 (vector-length ir) 0)
          final-param-count (vector-get node 2)
          local-count (if (> local-max final-param-count) (- local-max final-param-count) 0)
          result (make-function-meta final-param-count local-count ir)]
          (do
            (root_push result)
            (root_pop)
            (root_pop)
            (root_pop)
            (root_pop)
            result))))))

;; record constructor/static accessor は user defn より先に function table / Wasm body に置く。
;; WasmEmit は末尾関数を _start から呼ぶため、helper は source-order の defn 群へ混在させない。
(defn record-def-fields [decl]
  (if (= (vector-length decl) 4)
    (vector-get decl 3)
    (vector-get decl 2)))

(defn compile-record-constructor-fields [fields idx count record-local env instrs]
  (if (>= idx count)
    instrs
    (do
      (root_push fields)
      (root_push env)
      (let [instrs-slot (root_push instrs)
        field-hash (vector-get fields (* idx 3))
        value-instrs (emit-to (vector-new 2) (op-local-get) (+ idx 1))]
        (do
          (root_push value-instrs)
          (let [next-instrs (compile-record-map-field-instrs env instrs record-local field-hash value-instrs)]
            (do
              (root_set instrs-slot next-instrs)
              (root_pop)
              (let [result (compile-record-constructor-fields fields (+ idx 1) count record-local env next-instrs)]
                (do
                  (root_pop)
                  (root_pop)
                  (root_pop)
                  result)))))))))

(defn compile-record-constructor-fields-with-nominal [decl module-hash fields idx count record-local env instrs]
  (let [with-marker
          (compile-record-nominal-marker
            env
            instrs
            record-local
            (if (= module-hash 0)
              (vector-get decl 1)
              (ast-qualified-name-hash module-hash (vector-get decl 1))))]
    (do
      (root_push with-marker)
      (let [with-fields
              (compile-record-constructor-fields
                fields
                idx
                count
                record-local
                env
                with-marker)]
        (do
          (root_pop)
          with-fields)))))

(defn make-record-constructor-meta [decl module-hash]
  (do
    (root_push decl)
    (let [fields (record-def-fields decl)]
      (do
        (root_push fields)
        (let [field-count (/ (vector-length fields) 3)
          record-local (+ field-count 1)
          env (env-new)
          instrs0 (vector-new 8)]
          (do
            (root_push env)
            (root_push instrs0)
            (let [instrs1 (emit-to instrs0 (op-map-new) record-local)]
              (do
                (root_push instrs1)
                (let [instrs2 (emit-to instrs1 (op-local-set) record-local)]
                  (do
                    (root_push instrs2)
                    (let [with-fields
                            (compile-record-constructor-fields-with-nominal
                              decl
                              module-hash
                              fields
                              0
                              field-count
                              record-local
                              env
                              instrs2)]
                      (do
                        (root_push with-fields)
                        (let [ir (emit-to with-fields (op-local-get) record-local)]
                          (do
                            (root_push ir)
                            (let [local-max (max-local-slot ir 0 (vector-length ir) 0)
                              local-count (if (> local-max field-count) (- local-max field-count) 0)
                              result (make-function-meta field-count local-count ir)]
                              (do
                                (root_push result)
                                (root_pop)
                                (root_pop)
                                (root_pop)
                                (root_pop)
                                (root_pop)
                                (root_pop)
                                (root_pop)
                                (root_pop)
                                (root_pop)
                                result))))))))))))))))

(defn make-record-accessor-meta [field-hash]
  (let [map-op 2
    base-map-op 8
    result-local 13
    ir0 (emit-to (vector-new 4) (op-block) 0)
    ir1 (emit-to ir0 (op-loop) 0)
    ir2 (emit-to ir1 (op-local-get) 1)
    ir3 (emit-to ir2 (op-i64-const) field-hash)
    ir4 (emit-to ir3 (op-map-contains) map-op)]
    (do
      (root_push ir0)
      (root_push ir4)
      (let [ir5 (emit-to ir4 (op-if-empty) 0)
        ir6 (emit-to ir5 (op-local-get) 1)
        ir7 (emit-to ir6 (op-i64-const) field-hash)
        ir8 (emit-to ir7 (op-map-get) map-op)
        ir9 (emit-to ir8 (op-local-set) result-local)
        ir10 (emit-to ir9 (op-br) 2)
        ir11 (emit-to ir10 (op-else) 0)
        ir12 (emit-to ir11 (op-local-get) 1)
        ir13 (emit-to ir12 (op-i64-const) (record-update-base-key))
        ir14 (emit-to ir13 (op-map-get) base-map-op)
        ir15 (emit-to ir14 (op-local-set) 1)
        ir16 (emit-to ir15 (op-br) 1)
        ir17 (emit-to ir16 (op-end) 0)
        ir18 (emit-to ir17 (op-end) 0)
        ir19 (emit-to ir18 (op-end) 0)
        ir20 (emit-to ir19 (op-local-get) result-local)
        local-max (max-local-slot ir20 0 (vector-length ir20) 0)
        local-count (if (> local-max 1) (- local-max 1) 0)
        result (make-function-meta 1 local-count ir20)]
        (do
          (root_push result)
          (root_pop)
          (root_pop)
          (root_pop)
          result)))))

(defn make-record-pattern-field-presence-meta [field-hash]
  (let [map-op 2
    base-map-op 8
    result-local 13
    ir0 (emit-to (vector-new 4) (op-block) 0)
    ir1 (emit-to ir0 (op-loop) 0)
    ir2 (emit-to ir1 (op-local-get) 1)
    ir3 (emit-to ir2 (op-i64-const) field-hash)
    ir4 (emit-to ir3 (op-map-contains) map-op)]
    (do
      (root_push ir0)
      (root_push ir4)
      (let [ir5 (emit-to ir4 (op-if-empty) 0)
        ir6 (emit-to ir5 (op-i64-const) 1)
        ir7 (emit-to ir6 (op-local-set) result-local)
        ir8 (emit-to ir7 (op-br) 2)
        ir9 (emit-to ir8 (op-else) 0)
        ir10 (emit-to ir9 (op-local-get) 1)
        ir11 (emit-to ir10 (op-i64-const) (record-update-base-key))
        ir12 (emit-to ir11 (op-map-contains) base-map-op)
        ir13 (emit-to ir12 (op-if-empty) 0)
        ir14 (emit-to ir13 (op-local-get) 1)
        ir15 (emit-to ir14 (op-i64-const) (record-update-base-key))
        ir16 (emit-to ir15 (op-map-get) base-map-op)
        ir17 (emit-to ir16 (op-local-set) 1)
        ir18 (emit-to ir17 (op-br) 2)
        ir19 (emit-to ir18 (op-else) 0)
        ir20 (emit-to ir19 (op-i64-const) 0)
        ir21 (emit-to ir20 (op-local-set) result-local)
        ir22 (emit-to ir21 (op-br) 3)
        ir23 (emit-to ir22 (op-end) 0)
        ir24 (emit-to ir23 (op-end) 0)
        ir25 (emit-to ir24 (op-end) 0)
        ir26 (emit-to ir25 (op-end) 0)
        ir27 (emit-to ir26 (op-local-get) result-local)
        local-max (max-local-slot ir27 0 (vector-length ir27) 0)
        local-count (if (> local-max 1) (- local-max 1) 0)
        result (make-function-meta 1 local-count ir27)]
        (do
          (root_push result)
          (root_pop)
          (root_pop)
          (root_pop)
          result)))))

;; ordinary ADT は record と同じ Map runtime の narrow slice を使う。
;; -2 は variant hash、0.. は constructor field index を保持する予約 key。
(defn compile-adt-constructor-fields [fields idx count constructor-local env instrs]
  (if (>= idx count)
    instrs
    (do
      (root_push fields)
      (root_push env)
      (let [instrs-slot (root_push instrs)
        value-instrs (emit-to (vector-new 2) (op-local-get) (+ idx 1))]
        (do
          (root_push value-instrs)
          (let [next-instrs
                  (compile-record-map-field-instrs
                    env
                    instrs
                    constructor-local
                    (adt-constructor-field-key idx)
                    value-instrs)]
            (do
              (root_set instrs-slot next-instrs)
              (root_pop)
              (let [result
                      (compile-adt-constructor-fields
                        fields
                        (+ idx 1)
                        count
                        constructor-local
                        env
                        next-instrs)]
                (do
                  (root_pop)
                  (root_pop)
                  (root_pop)
                  result)))))))))

(defn make-adt-constructor-meta [variant]
  (do
    (root_push variant)
    (let [constructor-hash (vector-get variant 0)
      fields (vector-get variant 1)]
      (do
        (root_push fields)
        (let [field-count (vector-length fields)
          constructor-local (+ field-count 1)
          env (env-new)
          instrs0 (vector-new 8)]
          (do
            (root_push env)
            (root_push instrs0)
            (let [instrs1 (emit-to instrs0 (op-map-new) constructor-local)]
              (do
                (root_push instrs1)
                (let [instrs2 (emit-to instrs1 (op-local-set) constructor-local)]
                  (do
                    (root_push instrs2)
                    (let [tag-value (emit-to (vector-new 2) (op-i64-const) constructor-hash)]
                      (do
                        (root_push tag-value)
                        (let [with-tag
                                (compile-record-map-field-instrs
                                  env
                                  instrs2
                                  constructor-local
                                  (adt-constructor-tag-key)
                                  tag-value)]
                          (do
                            (root_push with-tag)
                            (let [with-fields
                                    (compile-adt-constructor-fields
                                      fields
                                      0
                                      field-count
                                      constructor-local
                                      env
                                      with-tag)
                              ir (emit-to with-fields (op-local-get) constructor-local)
                              local-max (max-local-slot ir 0 (vector-length ir) 0)
                              local-count (if (> local-max field-count) (- local-max field-count) 0)
                              result (make-function-meta field-count local-count ir)]
                              (do
                                (root_push result)
                                (root_pop)
                                (root_pop)
                                (root_pop)
                                (root_pop)
                                (root_pop)
                                (root_pop)
                                (root_pop)
                                (root_pop)
                                (root_pop)
                                result))))))))))))))))

(defn make-record-register-result [ftable func-idx functions]
  (do
    (root_push ftable)
    (root_push functions)
    (let [result1 (push-object-vector (vector-new 3) ftable)]
      (do
        (root_push result1)
        (let [result2 (push-int-vector result1 func-idx)]
          (do
            (root_push result2)
            (let [result (push-object-vector result2 functions)]
              (do
                (root_push result)
                (root_pop)
                (root_pop)
                (root_pop)
                (root_pop)
                (root_pop)
                result))))))))

(defn register-record-accessors [module-hash fields idx count ftable func-idx functions]
  (if (>= idx count)
    (make-record-register-result ftable func-idx functions)
    (do
      (root_push fields)
      (root_push ftable)
      (root_push functions)
      (let [field-hash (vector-get fields (* idx 3))
        accessor-hash (vector-get fields (+ (* idx 3) 1))
        module-qualified-hash
          (if (= module-hash 0)
            0
            (ast-qualified-name-hash module-hash accessor-hash))
        accessor-meta (make-record-accessor-meta field-hash)
        presence-meta (make-record-pattern-field-presence-meta field-hash)]
        (do
          (root_push accessor-meta)
          (root_push presence-meta)
          (let [with-accessor (ftable-register ftable accessor-hash func-idx)]
            (do
              (root_push with-accessor)
              (let [with-module
                      (if (= module-qualified-hash 0)
                        with-accessor
                        (ftable-register with-accessor module-qualified-hash func-idx))]
                (do
                  (root_push with-module)
                  (let [with-get
                          (ftable-register
                            with-module
                            (record-pattern-field-get-key field-hash)
                            func-idx)]
                    (do
                      (root_push with-get)
                      (let [next-ftable
                              (ftable-register
                                with-get
                                (record-pattern-field-presence-key field-hash)
                                (+ func-idx 1))]
                        (do
                          (root_push next-ftable)
                          (let [with-accessor-function
                                  (push-object-vector functions accessor-meta)]
                            (do
                              (root_push with-accessor-function)
                              (let [next-functions
                                      (push-object-vector
                                        with-accessor-function
                                        presence-meta)]
                                (do
                                  (root_push next-functions)
                                  (let [result
                                          (register-record-accessors
                                            module-hash
                                            fields
                                            (+ idx 3)
                                            count
                                            next-ftable
                                            (+ func-idx 2)
                                            next-functions)]
                                    (do
                                      (root_pop)
                                      (root_pop)
                                      (root_pop)
                                      (root_pop)
                                      (root_pop)
                                      (root_pop)
                                      (root_pop)
                                      (root_pop)
                                      (root_pop)
                                      (root_pop)
                                      (root_pop)
                                      result)))))))))))))))))))

(defn record-prelude-module-hash-loop [decls idx]
  (if (< idx 0)
    0
    (let [decl (vector-get decls idx)]
      (if (= (vector-get decl 0) 25)
        (vector-get decl 1)
        (record-prelude-module-hash-loop decls (- idx 1))))))

(defn register-record-decl [decl module-hash ftable func-idx functions]
  (do
    (root_push decl)
    (root_push ftable)
    (root_push functions)
    (let [fields (record-def-fields decl)
      constructor-hash (vector-get decl 1)
      constructor-meta (make-record-constructor-meta decl module-hash)]
      (do
        (root_push fields)
        (root_push constructor-meta)
        (let [with-raw (ftable-register ftable constructor-hash func-idx)
          module-qualified-hash
            (if (= module-hash 0)
              0
              (ast-qualified-name-hash module-hash constructor-hash))]
          (do
            (root_push with-raw)
            (let [next-ftable
                    (if (= module-qualified-hash 0)
                      with-raw
                      (ftable-register with-raw module-qualified-hash func-idx))]
              (do
                (root_push next-ftable)
                (let [next-functions (push-object-vector functions constructor-meta)]
                  (do
                    (root_push next-functions)
                    (let [result
                            (register-record-accessors
                              module-hash
                              fields
                              0
                              (vector-length fields)
                              next-ftable
                              (+ func-idx 1)
                              next-functions)]
                      (do
                        (root_pop)
                        (root_pop)
                        (root_pop)
                        (root_pop)
                        (root_pop)
                        (root_pop)
                        (root_pop)
                        (root_pop)
                        result))))))))))))

(defn register-adt-variants [variants idx count ftable func-idx functions]
  (if (>= idx count)
    (make-record-register-result ftable func-idx functions)
    (do
      (root_push variants)
      (root_push ftable)
      (root_push functions)
      (let [variant (vector-get variants idx)]
        (do
          (root_push variant)
          (let [constructor-hash (vector-get variant 0)
            constructor-meta (make-adt-constructor-meta variant)]
            (do
              (root_push constructor-meta)
              (let [next-ftable (ftable-register ftable constructor-hash func-idx)]
                (do
                  (root_push next-ftable)
                  (let [next-functions (push-object-vector functions constructor-meta)]
                    (do
                      (root_push next-functions)
                      (let [result
                              (register-adt-variants
                                variants
                                (+ idx 1)
                                count
                                next-ftable
                                (+ func-idx 1)
                                next-functions)]
                        (do
                          (root_pop)
                          (root_pop)
                          (root_pop)
                          (root_pop)
                          (root_pop)
                          (root_pop)
                          (root_pop)
                          result)))))))))))))

(defn register-adt-decl [decl ftable func-idx functions]
  (let [variants
          (if (>= (vector-length decl) 4)
            (vector-get decl 3)
            (vector-get decl 2))]
    (register-adt-variants variants 0 (vector-length variants) ftable func-idx functions)))

(defn make-record-prelude-state [done next-idx ftable next-func-idx functions]
  (do
    (root_push ftable)
    (root_push functions)
    (let [state1 (push-int-vector (vector-new 5) done)]
      (do
        (root_push state1)
        (let [state2 (push-int-vector state1 next-idx)]
          (do
            (root_push state2)
            (let [state3 (push-object-vector state2 ftable)]
              (do
                (root_push state3)
                (let [state4 (push-int-vector state3 next-func-idx)]
                  (do
                    (root_push state4)
                    (let [result (push-object-vector state4 functions)]
                      (do
                        (root_push result)
                        (root_pop)
                        (root_pop)
                        (root_pop)
                        (root_pop)
                        (root_pop)
                        (root_pop)
                        (root_pop)
                        result))))))))))))

(defn record-prelude-step [decls idx n ftable func-idx functions]
  (if (>= idx n)
    (make-record-prelude-state 1 idx ftable func-idx functions)
    (do
      (root_push decls)
      (root_push ftable)
      (root_push functions)
      (let [decl (vector-get decls idx)
        module-hash (record-prelude-module-hash-loop decls (- idx 1))]
        (do
          (root_push decl)
          (if (= (vector-get decl 0) 22)
            (let [record-result (register-record-decl decl module-hash ftable func-idx functions)]
              (do
                (root_push record-result)
                (let [result (make-record-prelude-state 0 (+ idx 1) (vector-get record-result 0) (vector-get record-result 1) (vector-get record-result 2))]
                  (do
                    (root_pop)
                    (root_pop)
                    (root_pop)
                    (root_pop)
                    (root_pop)
                    result))))
            (if (= (vector-get decl 0) 21)
              (let [adt-result (register-adt-decl decl ftable func-idx functions)]
                (do
                  (root_push adt-result)
                  (let [result (make-record-prelude-state 0 (+ idx 1) (vector-get adt-result 0) (vector-get adt-result 1) (vector-get adt-result 2))]
                    (do
                      (root_pop)
                      (root_pop)
                      (root_pop)
                      (root_pop)
                      (root_pop)
                      result))))
              (let [result (make-record-prelude-state 0 (+ idx 1) ftable func-idx functions)]
                (do
                  (root_pop)
                  (root_pop)
                  (root_pop)
                  (root_pop)
                  result)))))))))

(defn continue-record-prelude-step [decls n state]
  (if (= (vector-get state 0) 1)
    state
    (do
      (root_push decls)
      (root_push state)
      (let [next-ftable (vector-get state 2)
        next-functions (vector-get state 4)]
        (do
          (root_push next-ftable)
          (root_push next-functions)
          (let [result (record-prelude-step decls (vector-get state 1) n next-ftable (vector-get state 3) next-functions)]
            (do
              (root_pop)
              (root_pop)
              (root_pop)
              (root_pop)
              result)))))))

(defn continue-record-prelude-step-times [decls n remaining state]
  (if (= remaining 0)
    state
    (if (= (vector-get state 0) 1)
      state
      (do
        (root_push decls)
        (root_push state)
        (let [next-state (continue-record-prelude-step decls n state)]
          (do
            (root_push next-state)
            (let [result (continue-record-prelude-step-times decls n (- remaining 1) next-state)]
              (do
                (root_pop)
                (root_pop)
                (root_pop)
                result))))))))

(defn record-prelude-step-64 [decls idx n ftable func-idx functions]
  (do
    (root_push decls)
    (root_push ftable)
    (root_push functions)
    (let [state (record-prelude-step decls idx n ftable func-idx functions)]
      (do
        (root_push state)
        (let [result (continue-record-prelude-step-times decls n 63 state)]
          (do
            (root_pop)
            (root_pop)
            (root_pop)
            (root_pop)
            result))))))

(defn continue-record-prelude-step-64 [decls n state]
  (if (= (vector-get state 0) 1)
    state
    (do
      (root_push decls)
      (root_push state)
      (let [next-ftable (vector-get state 2)
        next-functions (vector-get state 4)]
        (do
          (root_push next-ftable)
          (root_push next-functions)
          (let [next-state (record-prelude-step-64 decls (vector-get state 1) n next-ftable (vector-get state 3) next-functions)]
            (do
              (root_push next-state)
              (let [result (continue-record-prelude-step-64 decls n next-state)]
                (do
                  (root_pop)
                  (root_pop)
                  (root_pop)
                  (root_pop)
                  (root_pop)
                  result)))))))))

(defn record-prelude-chunked [decls idx n ftable func-idx functions]
  (do
    (root_push decls)
    (root_push ftable)
    (root_push functions)
    (let [state (record-prelude-step-64 decls idx n ftable func-idx functions)]
      (do
        (root_push state)
        (let [result (continue-record-prelude-step-64 decls n state)]
          (do
            (root_pop)
            (root_pop)
            (root_pop)
            (root_pop)
            result))))))

(defn register-defns-step [decls idx n ftable func-idx]
  (if (>= idx n)
    (let [done-state-ref (ref-new 0)]
      (do
        (root_push done-state-ref)
        (write-register-state-ref done-state-ref 1 idx ftable func-idx)
        (let [done-state (ref-get done-state-ref)]
          (do
            (root_pop)
            done-state))))
    (do
      (root_push decls)
      (root_push ftable)
      (let [decl (vector-get decls idx)]
        (do
          (root_push decl)
          (if (= (vector-get decl 0) 20)
            (let [name-hash (vector-get decl 1)
              next-ftable (ftable-register ftable name-hash func-idx)]
                (do
                  (root_push next-ftable)
                  (let [defn-state-ref (ref-new 0)]
                    (do
                      (root_push defn-state-ref)
                      (write-register-state-ref defn-state-ref 0 (+ idx 1) next-ftable (+ func-idx 1))
                      (let [defn-state (ref-get defn-state-ref)]
                        (do
                          (root_pop)
                          (root_pop)
                          (root_pop)
                          (root_pop)
                          (root_pop)
                          defn-state))))))
            (let [non-defn-state-ref (ref-new 0)]
              (do
                (root_push non-defn-state-ref)
                (write-register-state-ref non-defn-state-ref 0 (+ idx 1) ftable func-idx)
                (let [non-defn-state (ref-get non-defn-state-ref)]
                  (do
                    (root_pop)
                    (root_pop)
                    (root_pop)
                    (root_pop)
                    non-defn-state))))))))))

(defn continue-register-defns-step [decls n state]
  (if (= (vector-get state 0) 1)
    state
    (do
      (root_push decls)
      (root_push state)
      (let [next-idx (vector-get state 1)
        next-ftable (vector-get state 2)
        next-func-idx (vector-get state 3)]
        (do
          (root_push next-ftable)
          (let [result (register-defns-step decls next-idx n next-ftable next-func-idx)]
            (do
              (root_pop)
              (root_pop)
              (root_pop)
              result)))))))

(defn continue-register-defns-step-times [decls n remaining state]
  (if (= remaining 0)
    state
    (if (= (vector-get state 0) 1)
      state
      (do
        (root_push decls)
        (root_push state)
        (let [next-state (continue-register-defns-step decls n state)]
          (do
            (root_push next-state)
            (let [result (continue-register-defns-step-times decls n (- remaining 1) next-state)]
              (do
                (root_pop)
                (root_pop)
                (root_pop)
                result))))))))

(defn register-defns-step-8 [decls idx n ftable func-idx]
  (do
    (root_push decls)
    (let [state (register-defns-step decls idx n ftable func-idx)]
      (do
        (root_push state)
        (let [result (continue-register-defns-step-times decls n 7 state)]
          (do
            (root_pop)
            (root_pop)
            result))))))

(defn continue-register-defns-step-8 [decls n state]
  (if (= (vector-get state 0) 1)
    state
    (do
      (root_push decls)
      (root_push state)
      (let [next-idx (vector-get state 1)
        next-ftable (vector-get state 2)
        next-func-idx (vector-get state 3)]
        (do
          (root_push next-ftable)
          (let [result (register-defns-step-8 decls next-idx n next-ftable next-func-idx)]
            (do
              (root_pop)
              (root_pop)
              (root_pop)
              result)))))))

(defn register-defns-step-64 [decls idx n ftable func-idx]
  (do
    (root_push decls)
    (let [state (register-defns-step decls idx n ftable func-idx)]
      (do
        (root_push state)
        (let [result (continue-register-defns-step-times decls n 63 state)]
          (do
            (root_pop)
            (root_pop)
            result))))))

(defn continue-register-defns-step-64 [decls n state]
  (if (= (vector-get state 0) 1)
    state
    (do
      (root_push decls)
      (root_push state)
      (let [next-idx (vector-get state 1)
        next-ftable (vector-get state 2)
        next-func-idx (vector-get state 3)]
        (do
          (root_push next-ftable)
          (let [next-state (register-defns-step-64 decls next-idx n next-ftable next-func-idx)]
            (do
              (root_push next-state)
              (let [result (continue-register-defns-step-64 decls n next-state)]
                (do
                  (root_pop)
                  (root_pop)
                  (root_pop)
                  (root_pop)
                  result)))))))))

(defn register-defns-chunked [decls idx n ftable func-idx]
  (do
    (root_push decls)
    (root_push ftable)
    (let [state (register-defns-step-64 decls idx n ftable func-idx)]
      (do
        (let [state-slot (root_push state)]
          (do
            (let [result (continue-register-defns-step-64 decls n state)]
              (do
                (root_pop)
                (root_pop)
                (root_pop)
                result))))))))

(defn register-defns [decls idx n ftable func-idx]
  (if (>= idx n)
    (vector-push (push-object-vector (vector-new 2) ftable) func-idx)
    (let [decl (vector-get decls idx)]
      (if (= (vector-get decl 0) 20)
        (let [next-ftable (ftable-register ftable (vector-get decl 1) func-idx)]
          (do
            (root_push next-ftable)
            (let [result (register-defns decls (+ idx 1) n next-ftable (+ func-idx 1))]
              (do
                (root_pop)
                result))))
        (register-defns decls (+ idx 1) n ftable func-idx)))))
(defn compile-defn-functions-step [decls idx n ftable functions]
  (if (>= idx n)
    (make-compile-step-state 1 idx functions)
    (let [decls-slot (root_push decls)
      ftable-slot (root_push ftable)
      functions-slot (root_push functions)
      decl (vector-get decls idx)]
      (if (= (vector-get decl 0) 20)
        (do
          (root_push decl)
          (let [compiled-fn (compile-defn-function decl ftable)]
            (do
              (root_push compiled-fn)
              (let [defn-result (compile-defn-functions-step-finish functions compiled-fn idx)]
                (do
                  (root_set functions-slot defn-result)
                  (root_pop)
                  (root_pop)
                  (root_pop)
                  (root_pop)
                  (root_pop)
                  defn-result)))))
        (do
          (let [skip-state-ref (ref-new 0)]
            (do
              (root_push skip-state-ref)
              (write-compile-step-state-ref skip-state-ref 0 (+ idx 1) functions)
              (let [skip-result (ref-get skip-state-ref)]
                (do
                  (root_push skip-result)
                  (root_set functions-slot skip-result)
                  (root_pop)
                  (root_pop)
                  (root_pop)
                  (root_pop)
                  (root_pop)
                  skip-result)))))))))

(defn continue-compile-defn-functions-step [decls n ftable state]
  (if (= (vector-get state 0) 1)
    state
    (compile-defn-functions-step decls (vector-get state 1) n ftable (vector-get state 2))))

(defn continue-compile-defn-functions-step-times [decls n ftable remaining state]
  (if (= remaining 0)
    state
    (if (= (vector-get state 0) 1)
      state
      (do
        (root_push decls)
        (root_push ftable)
        (root_push state)
        (let [next-state (continue-compile-defn-functions-step decls n ftable state)]
          (do
            (root_push next-state)
            (let [result (continue-compile-defn-functions-step-times decls n ftable (- remaining 1) next-state)]
              (do
                (root_pop)
                (root_pop)
                (root_pop)
                (root_pop)
                result))))))))

(defn compile-defn-functions-step-8 [decls idx n ftable functions]
  (do
    (root_push decls)
    (root_push ftable)
    (let [state (compile-defn-functions-step decls idx n ftable functions)]
      (do
        (root_push state)
        (let [result (continue-compile-defn-functions-step-times decls n ftable 7 state)]
          (do
            (root_pop)
            (root_pop)
            (root_pop)
            result))))))

(defn continue-compile-defn-functions-step-8 [decls n ftable state]
  (if (= (vector-get state 0) 1)
    state
    (do
      (root_push decls)
      (root_push ftable)
      (root_push state)
      (let [result (compile-defn-functions-step-8 decls (vector-get state 1) n ftable (vector-get state 2))]
        (do
          (root_pop)
          (root_pop)
          (root_pop)
          result)))))

(defn compile-defn-functions-step-64 [decls idx n ftable functions]
  (do
    (root_push decls)
    (root_push ftable)
    (let [state (compile-defn-functions-step decls idx n ftable functions)]
      (do
        (root_push state)
        (let [result (continue-compile-defn-functions-step-times decls n ftable 63 state)]
          (do
            (root_pop)
            (root_pop)
            (root_pop)
            result))))))

(defn continue-compile-defn-functions-step-64 [decls n ftable state]
  (if (= (vector-get state 0) 1)
    state
    (do
      (root_push decls)
      (root_push ftable)
      (root_push state)
      (let [next-state (compile-defn-functions-step-64 decls (vector-get state 1) n ftable (vector-get state 2))]
        (do
          (root_push next-state)
          (let [result (continue-compile-defn-functions-step-64 decls n ftable next-state)]
            (do
              (root_pop)
              (root_pop)
              (root_pop)
              (root_pop)
              result)))))))

(defn compile-defn-functions-chunked [decls idx n ftable functions]
  (let [state (compile-defn-functions-step-64 decls idx n ftable functions)]
    (do
      (let [state-slot (root_push state)]
        (do
          (let [result (continue-compile-defn-functions-step-64 decls n ftable state)]
            (do
              (root_push result)
              (root_set state-slot result)
              (root_pop)
              (root_pop)
              result)))))))

(defn compile-defn-functions [decls idx n ftable functions]
  (if (>= idx n)
    functions
    (let [decl (vector-get decls idx)]
      (if (= (vector-get decl 0) 20)
        (let [next-functions (push-object-vector functions (compile-defn-function decl ftable))]
          (do
            (root_push next-functions)
            (let [result (compile-defn-functions decls (+ idx 1) n ftable next-functions)]
              (do
                (root_pop)
                result))))
        (compile-defn-functions decls (+ idx 1) n ftable functions)))))
(defn collect-function-irs-step [functions idx count ir-list]
  (if (>= idx count)
    (make-compile-step-state 1 idx ir-list)
    (do
      (root_push functions)
      (root_push ir-list)
      (let [func-ir (function-meta-ir (vector-get functions idx))]
        (do
          (root_push func-ir)
          (let [next-ir-list (push-object-vector ir-list func-ir)]
            (do
              (root_push next-ir-list)
              (let [result (make-compile-step-state 0 (+ idx 1) next-ir-list)]
                (do
                  (root_pop)
                  (root_pop)
                  (root_pop)
                  (root_pop)
                  result)))))))))

(defn continue-collect-function-irs-step [functions count state]
  (if (= (vector-get state 0) 1)
    state
    (collect-function-irs-step functions (vector-get state 1) count (vector-get state 2))))

(defn collect-function-irs-step-8 [functions idx count ir-list]
  (let [step1 (collect-function-irs-step functions idx count ir-list)
    step2 (continue-collect-function-irs-step functions count step1)
    step3 (continue-collect-function-irs-step functions count step2)
    step4 (continue-collect-function-irs-step functions count step3)
    step5 (continue-collect-function-irs-step functions count step4)
    step6 (continue-collect-function-irs-step functions count step5)
    step7 (continue-collect-function-irs-step functions count step6)
    step8 (continue-collect-function-irs-step functions count step7)]
    step8))

(defn continue-collect-function-irs-step-8 [functions count state]
  (if (= (vector-get state 0) 1)
    state
    (collect-function-irs-step-8 functions (vector-get state 1) count (vector-get state 2))))

(defn collect-function-irs-step-64 [functions idx count ir-list]
  (let [step1 (collect-function-irs-step-8 functions idx count ir-list)
    step2 (continue-collect-function-irs-step-8 functions count step1)
    step3 (continue-collect-function-irs-step-8 functions count step2)
    step4 (continue-collect-function-irs-step-8 functions count step3)
    step5 (continue-collect-function-irs-step-8 functions count step4)
    step6 (continue-collect-function-irs-step-8 functions count step5)
    step7 (continue-collect-function-irs-step-8 functions count step6)
    step8 (continue-collect-function-irs-step-8 functions count step7)]
    step8))

(defn continue-collect-function-irs-step-64 [functions count state]
  (if (= (vector-get state 0) 1)
    state
    (let [next-state (collect-function-irs-step-64 functions (vector-get state 1) count (vector-get state 2))]
      (do
        (root_push next-state)
        (let [result (continue-collect-function-irs-step-64 functions count next-state)]
          (do
            (root_pop)
            result))))))

(defn collect-function-irs-chunked [functions idx count ir-list]
  (continue-collect-function-irs-step-64 functions count (collect-function-irs-step-64 functions idx count ir-list)))

(defn collect-function-irs [functions idx count ir-list]
  (if (>= idx count)
    ir-list
    (let [next-ir-list (push-object-vector ir-list (function-meta-ir (vector-get functions idx)))]
      (do
        (root_push next-ir-list)
        (let [result (collect-function-irs functions (+ idx 1) count next-ir-list)]
          (do
            (root_pop)
            result))))))
(defn program-functions-base [decls base-idx]
  (do
    (root_push decls)
    (let [n (vector-length decls)
      prelude (record-prelude-chunked decls 0 n (ftable-new) base-idx (vector-new 8))]
      (do
        (root_push prelude)
        (let [prelude-ftable (vector-get prelude 2)
          prelude-func-idx (vector-get prelude 3)
          prelude-functions (vector-get prelude 4)]
          (do
            (root_push prelude-ftable)
            (root_push prelude-functions)
            (let [pass1 (register-defns-chunked decls 0 n prelude-ftable prelude-func-idx)]
              (do
                (root_push pass1)
                (let [ftable (vector-get pass1 2)]
                  (do
                    (root_push ftable)
                    (let [functions-state (compile-defn-functions-chunked decls 0 n ftable prelude-functions)]
                      (do
                        (root_push functions-state)
                        (let [functions (vector-get functions-state 2)]
                          (do
                            (root_push functions)
                            (let [result1 (push-object-vector (vector-new 2) ftable)]
                              (do
                                (root_push result1)
                                (let [result (push-object-vector result1 functions)]
                                  (do
                                    (root_push result)
                                    (root_pop)
                                    (root_pop)
                                    (root_pop)
                                    (root_pop)
                                    (root_pop)
                                    (root_pop)
                                    (root_pop)
                                    (root_pop)
                                    (root_pop)
                                    (root_pop)
                                    result))))))))))))))))))

(defn compile-program-functions [decls]
  (program-functions-base decls 0))
;; V2-11: import section を含む harness 向けに base offset を取る変種。
;; base-idx は user func を ftable に登録する際の起点で、selfhost ランタイム
;; (root_push 等) を import 0..base-idx-1 に置く構成と整合する。
(defn compile-program-functions-with-base [decls base-idx]
  (program-functions-base decls base-idx))
(defn compile-program [decls] (let [pair (compile-program-functions decls) ftable (vector-get pair 0) functions (vector-get pair 1) ir-state (collect-function-irs-chunked functions 0 (vector-length functions) (vector-new 8)) ir-list (vector-get ir-state 2)] (push-object-vector (push-object-vector (vector-new 2) ftable) ir-list)))
(defn lower [x] (let [n (vector-length x)] (if (= n 0) (vector-new 0) (if (= n 2) (if (if (= (vector-get x 0) 1) true (= (vector-get x 0) 2)) (compile-expr x (env-new) (vector-new 8)) (let [pair (compile-program x) ir-list (vector-get pair 1)] (if (> (vector-length ir-list) 0) (vector-get ir-list 0) (vector-new 0)))) (let [pair (compile-program x) ir-list (vector-get pair 1)] (if (> (vector-length ir-list) 0) (vector-get ir-list 0) (vector-new 0)))))))
(defn bind-param-hashes [param-hashes idx n env local-idx]
  (if (>= idx n)
    env
    (do
      (root_push param-hashes)
      (root_push env)
      (let [next-env (env-bind env (vector-get param-hashes idx) local-idx)]
        (do
          (root_push next-env)
          (let [result (bind-param-hashes param-hashes (+ idx 1) n next-env (+ local-idx 1))]
            (do
              (root_pop)
              (root_pop)
              (root_pop)
              result)))))))
(defn compile-function [param-hashes body]
  (do
    (root_push param-hashes)
    (root_push body)
    (let [env (bind-param-hashes param-hashes 0 (vector-length param-hashes) (env-new) 1)]
      (do
        (root_push env)
        (let [result (compile-expr body env (vector-new 8))]
          (do
            (root_pop)
            (root_pop)
            (root_pop)
            result))))))
(defn leb128-append [buf value]
  (if (< value 128)
    (vector-push buf value)
    (do
      (root_push buf)
      (let [next-buf (vector-push buf (+ (% value 128) 128))]
        (do
          (root_push next-buf)
          (let [result (leb128-append next-buf (/ value 128))]
            (do
              (root_pop)
              (root_pop)
              result)))))))
(defn leb128-unsigned [value]
  (leb128-append (vector-new 4) value))
(defn main [] (compiler-main-run))
(defn compiler-main-lit-node [] (vector-push (vector-push (vector-new 2) 1) 42))
(defn compiler-main-do-node [] (let [n (vector-new 8)] (let [n1 (vector-push n 9) n2 (vector-push n1 2) e1 (vector-push (vector-push (vector-new 2) 1) 10) n3 (vector-push n2 e1) e2 (vector-push (vector-push (vector-new 2) 1) 20) n4 (vector-push n3 e2)] n4)))
(defn compiler-main-add-node [] (let [callee (vector-push (vector-push (vector-new 2) 4) 999) n (vector-new 8)] (let [n1 (vector-push n 5) n2 (vector-push n1 callee) n3 (vector-push n2 2) a1 (vector-push (vector-push (vector-new 2) 1) 3) n4 (vector-push n3 a1) a2 (vector-push (vector-push (vector-new 2) 1) 4) n5 (vector-push n4 a2)] n5)))
(defn compiler-main-run [] (let [lit-node (compiler-main-lit-node) env (env-new) instrs (compile-expr lit-node env (vector-new 8)) do-node (compiler-main-do-node) do-instrs (compile-expr do-node env (vector-new 8)) leb-small (leb128-unsigned 5) leb-medium (leb128-unsigned 300) add-node (compiler-main-add-node) add-instrs (compile-expr add-node env (vector-new 8))] (compiler-main-report instrs do-instrs leb-small leb-medium add-instrs)))
(defn compiler-main-report [instrs do-instrs leb-small leb-medium add-instrs] (do (print (vector-length instrs)) (let [instr0 (vector-get instrs 0)] (do (print (vector-get instr0 0)) (print (vector-get instr0 1)))) (print (vector-length do-instrs)) (print (vector-length leb-small)) (print (vector-get leb-small 0)) (print (vector-length leb-medium)) (print (vector-get leb-medium 0)) (print (vector-get leb-medium 1)) (print (vector-length add-instrs)) (let [ai0 (vector-get add-instrs 0) ai2 (vector-get add-instrs 2) ai-last (vector-get add-instrs (- (vector-length add-instrs) 1))] (do (print (vector-get ai0 0)) (print (vector-get ai0 1)) (print (vector-get ai2 0)) (print (vector-get ai2 1)) (print (vector-get ai-last 0)) 0)) 0))
(defn compile-if-with-source [node source env ftable instrs data-ref]
  (compile-if-with-source-impl node source env ftable instrs data-ref))
(defn compile-let-with-ftable-impl [node env ftable instrs]
  (compile-let-with-ftable-impl-body node env ftable instrs))

(defn compile-expr-with-ftable-dispatch-impl [node env ftable instrs]
  (compile-expr-with-ftable-dispatch-impl-body node env ftable instrs))

(defn compile-defn-functions-step-with-source [decls idx n source ftable data-ref functions]
  (compile-defn-functions-step-with-source-body-impl-3 decls idx n source ftable data-ref functions))
(defn compile-let-chain-step-with-source [node source env ftable instrs data-ref rooted-count]
  (compile-let-chain-step-with-source-body node source env ftable instrs data-ref rooted-count))
(defn compile-defn-functions-step-with-source-body [decls idx n source ftable data-ref functions]
  (compile-defn-functions-step-with-source-body-impl decls idx n source ftable data-ref functions))
(defn compile-if-with-source-impl [node source env ftable instrs data-ref]
  (compile-if-with-source-impl-body node source env ftable instrs data-ref))
(defn compile-let-with-ftable-impl-body [node env ftable instrs]
  (compile-let-with-ftable-impl-body-impl node env ftable instrs))

(defn compile-expr-with-ftable-dispatch-impl-body [node env ftable instrs]
  (compile-expr-with-ftable-dispatch-impl-body-impl node env ftable instrs))

(defn compile-let-chain-step-with-source-body [node source env ftable instrs data-ref rooted-count]
  (compile-let-chain-step-with-source-body-impl node source env ftable instrs data-ref rooted-count))
(defn compile-defn-functions-step-with-source-body-impl [decls idx n source ftable data-ref functions]
  (compile-defn-functions-step-with-source-body-impl-2 decls idx n source ftable data-ref functions))
(defn compile-if-with-source-impl-body [node source env ftable instrs data-ref]
  (compile-if-with-source-impl-body-impl node source env ftable instrs data-ref))
(defn compile-let-with-ftable-impl-body-impl [node env ftable instrs]
  (compile-let-with-ftable-impl-body-impl-2 node env ftable instrs))

(defn compile-let-chain-step-with-source-body-impl [node source env ftable instrs data-ref rooted-count]
  (compile-let-chain-step-with-source-body-impl-2 node source env ftable instrs data-ref rooted-count))
(defn compile-defn-functions-step-with-source-body-impl-2 [decls idx n source ftable data-ref functions]
  (compile-defn-functions-step-with-source-body-impl-3 decls idx n source ftable data-ref functions))
(defn compile-let-with-ftable-impl-body-impl-2 [node env ftable instrs]
  (compile-let-with-ftable-impl-body-impl-3 node env ftable instrs))

(defn compile-let-chain-step-with-source-body-impl-2 [node source env ftable instrs data-ref rooted-count]
  (compile-let-chain-step-with-source-body-impl-3 node source env ftable instrs data-ref rooted-count))
(defn compile-let-chain-step-with-source-body-impl-3 [node source env ftable instrs data-ref rooted-count]
  (let [node-slot (root_push node)
    source-slot (root_push source)
    env-slot (root_push env)
    ftable-slot (root_push ftable)
    instrs-slot (root_push instrs)
	    data-slot (root_push data-ref)
	    name-hash (vector-get node 1)
	    init-expr (vector-get node 2)
	    init-root (alloc-root-needed init-expr)
	    init-instrs (compile-expr-with-source init-expr source env ftable instrs data-ref)
	    init-slot (root_push init-instrs)
	    body-expr-after-init (vector-get node 3)]
	    (let [result (compile-let-chain-step-finish name-hash body-expr-after-init init-instrs env rooted-count init-root)]
	      (do
	        (root_push result)
	        (root_pop)
	        (root_pop)
	        (root_pop)
	        (root_pop)
	        (root_pop)
	        (root_pop)
	        (root_pop)
	        (root_pop)
	        result))))
(defn compile-defn-functions-step-with-source-body-impl-3 [decls idx n source ftable data-ref functions]
  (if (>= idx n)
    (make-compile-step-state 1 idx functions)
    (let [source-step-progress-mode (if (> (string-length (command-line-arg 8)) 0) 1 0)
      decls-slot (root_push decls)
      source-slot (root_push source)
      ftable-slot (root_push ftable)
      data-slot (root_push data-ref)
      functions-slot (root_push functions)
      decl (vector-get decls idx)]
      (if (= (vector-get decl 0) 20)
        (do
          (root_push decl)
          (let [compiled-fn (compile-defn-function-with-source decl source ftable data-ref)]
            (do
              (root_push compiled-fn)
              (let [next-functions (push-object-vector functions compiled-fn)]
                (do
                  (root_push next-functions)
                  (let [defn-result (make-compile-step-state 0 (+ idx 1) next-functions)]
                    (do
                      (root_push defn-result)
                      (if (= source-step-progress-mode 1)
                        (if (< (vector-length functions) 128)
                          (do
                            (print 9000000077)
                            (print idx)
                            (print (vector-length functions))
                            (print (vector-length next-functions))
                            (print (vector-get defn-result 0))
                            (print (vector-get defn-result 1))
                            (print (vector-length (vector-get defn-result 2))))
                          (do))
                        (do))
                      (root_set functions-slot defn-result)
                      (root_pop)
                      (root_pop)
                      (root_pop)
                      (root_pop)
                      (root_pop)
                      (root_pop)
                      (root_pop)
                      (root_pop)
                      (root_pop)
                      defn-result)))))))
        (do
          (let [next-skip-idx (+ idx 1)
            skip-state-ref (ref-new 0)]
            (do
              (root_push skip-state-ref)
              (write-compile-step-state-ref skip-state-ref 0 next-skip-idx functions)
              (let [skip-result (ref-get skip-state-ref)]
                (do
                  (root_push skip-result)
                  (if (= source-step-progress-mode 1)
                    (if (< (vector-length functions) 128)
                      (do
                        (print 9000000078)
                        (print idx)
                        (print (vector-length functions))
                        (print (vector-get skip-result 0))
                        (print (vector-get skip-result 1))
                        (print (vector-length (vector-get skip-result 2))))
                      (do))
                    (do))
                  (root_set functions-slot skip-result)
                  (root_pop)
                  (root_pop)
                  (root_pop)
                  (root_pop)
                  (root_pop)
                  (root_pop)
                  (root_pop)
                  skip-result)))))))))
(defn compile-let-with-ftable-impl-body-impl-3 [node env ftable instrs]
  (let [node-slot (root_push node)
    env-slot (root_push env)
    ftable-slot (root_push ftable)
    instrs-slot (root_push instrs)
    name-hash (vector-get node 1)
    init-expr (vector-get node 2)
    init-root (alloc-root-needed init-expr)
    init-expr-slot (root_push init-expr)
    init-instrs (compile-expr-with-ftable init-expr env ftable instrs)
    init-instrs-slot (root_push init-instrs)
    body-expr-after-init (vector-get node 3)
    body-expr-slot (root_push body-expr-after-init)
    prep (compile-let-with-ftable-prepare name-hash init-root init-instrs env)
    prep-slot (root_push prep)
    new-env (vector-get prep 0)
    instrs2 (vector-get prep 1)
    new-env-slot (root_push new-env)
    instrs2-slot (root_push instrs2)
    body-instrs (compile-expr-with-ftable body-expr-after-init new-env ftable instrs2)]
    (do
      (root_push body-instrs)
      (let [result (maybe-root-pop-drop body-instrs init-root)]
        (do
          (root_push result)
          (root_pop)
          (root_pop)
          (root_pop)
          (root_pop)
          (root_pop)
          (root_pop)
          (root_pop)
          (root_pop)
          (root_pop)
          (root_pop)
          (root_pop)
          (root_pop)
          result)))))

(defn compile-if-with-source-impl-body-impl [node source env ftable instrs data-ref]
  (let [cond-expr (vector-get node 1)
    then-expr (vector-get node 2)
    else-expr (vector-get node 3)]
    (do
      (root_push cond-expr)
      (root_push then-expr)
      (root_push else-expr)
      (let [instrs1 (compile-expr-with-source cond-expr source env ftable instrs data-ref)]
        (do
          (root_push instrs1)
          (let [instrs2 (emit-to instrs1 41 0)]
            (do
              (root_push instrs2)
              (let [instrs3 (compile-expr-with-source then-expr source env ftable instrs2 data-ref)]
                (do
                  (root_push instrs3)
                  (let [instrs4 (emit-to instrs3 79 0)]
                    (do
                      (root_push instrs4)
                      (let [instrs5 (compile-expr-with-source else-expr source env ftable instrs4 data-ref)]
                        (do
                          (root_push instrs5)
                          (let [result (emit-to instrs5 43 0)]
                            (do
                              (root_push result)
                              (root_pop)
                              (root_pop)
                              (root_pop)
                              (root_pop)
                              (root_pop)
                              (root_pop)
                              (root_pop)
                              (root_pop)
                              (root_pop)
                              result)))))))))))))))
(defn compile-let-chain-with-source [node source env ftable instrs data-ref rooted-count]
  (let [step (compile-let-chain-step-with-source node source env ftable instrs data-ref rooted-count)]
    (do
      (root_push step)
      (let [next-value (vector-get step 2)]
        (do
          (root_push next-value)
          (if (= (vector-get step 0) 1)
            (let [body-expr (vector-get next-value 0)
              next-env (vector-get next-value 1)
              next-instrs (vector-get next-value 2)
              body-instrs (compile-expr-with-source body-expr source next-env ftable next-instrs data-ref)]
              (do
                (root_push body-instrs)
                (let [result (emit-root-pop-drops body-instrs (vector-get step 1))]
                  (do
                    (root_push result)
                    (root_pop)
                    (root_pop)
                    (root_pop)
                    (root_pop)
                    result))))
            (let [result
              (compile-let-chain-with-source
                (vector-get next-value 0)
                source
                (vector-get next-value 1)
                ftable
                (vector-get next-value 2)
                data-ref
                (vector-get step 1))]
              (do
                (root_push result)
                (root_pop)
                (root_pop)
                (root_pop)
                result))))))))
(defn compile-expr-with-ftable-dispatch-impl-body-impl [node env ftable instrs]
  (compile-expr-with-ftable-dispatch-simple node env ftable instrs))

(defn compile-expr-with-ftable-dispatch-simple [node env ftable instrs]
  (compile-expr-with-ftable-dispatch-simple-2 node env ftable instrs))

(defn compile-expr-with-ftable-dispatch-simple-2 [node env ftable instrs]
  (compile-expr-with-ftable-dispatch-simple-3 node env ftable instrs))

(defn compile-expr-with-ftable-dispatch-complex [tag node env ftable instrs]
  (if (= tag 5)
    (compile-apply-with-ftable node env ftable instrs)
    (if (= tag 6)
      (compile-if-with-ftable node env ftable instrs)
      (compile-expr-with-ftable-dispatch-complex-2 tag node env ftable instrs))))

(defn compile-lambda-with-ftable [node env ftable instrs]
  (let [param-count (vector-get node 1)
    new-env (bind-node-params node 2 0 param-count env (+ 1 (map-size env)))]
    (compile-expr-with-ftable (vector-get node (+ 2 param-count)) new-env ftable instrs)))

(defn compile-apply-with-ftable [node env ftable instrs]
  (let [func-node (vector-get node 1)
    arg-count (vector-get node 2)
    func-hash (if (= (vector-get func-node 0) 4) (vector-get func-node 1) 0)
    bop (builtin-opcode func-hash)]
    (if (builtin-not-application? func-hash arg-count)
      (compile-not-builtin-with-ftable node env ftable instrs)
      (if (> bop 0)
        (compile-builtin-apply-with-ftable node env ftable instrs bop)
        (compile-user-call-with-ftable node env ftable instrs func-hash arg-count)))))

(defn compile-if-with-ftable [node env ftable instrs]
  (let [cond-expr (vector-get node 1)
    then-expr (vector-get node 2)
    else-expr (vector-get node 3)]
    (do
      (root_push cond-expr)
      (root_push then-expr)
      (root_push else-expr)
      (let [instrs1 (compile-expr-with-ftable cond-expr env ftable instrs)
        instrs2 (emit-to instrs1 41 0)
        instrs3 (compile-expr-with-ftable then-expr env ftable instrs2)
        instrs4 (emit-to instrs3 79 0)
        instrs5 (compile-expr-with-ftable else-expr env ftable instrs4)
        result (emit-to instrs5 43 0)]
        (do
          (root_pop)
          (root_pop)
          (root_pop)
          result)))))

(defn compile-expr-with-ftable-dispatch-simple-3 [node env ftable instrs]
  (let [tag (vector-get node 0)]
    (if (= tag 1)
      (emit-to instrs 1 (vector-get node 1))
      (if (= tag 2)
        (emit-to instrs 1 (vector-get node 1))
        (if (= tag 4)
          (compile-expr-with-ftable-dispatch-var node env ftable instrs)
          (compile-expr-with-ftable-dispatch-complex tag node env ftable instrs))))))

(defn compile-expr-with-ftable-dispatch-complex-2 [tag node env ftable instrs]
  (if (= tag 7)
    (compile-let-with-ftable node env ftable instrs)
    (compile-expr-with-ftable-dispatch-complex-2-rest tag node env ftable instrs)))

(defn compile-do-exprs [node env ftable idx expr-count instrs]
  (continue-compile-do-exprs node env ftable expr-count (compile-do-exprs-step-64 node env ftable idx expr-count instrs)))

(defn compile-do-expr-with-source [node source env ftable idx instrs data-ref]
  (compile-expr-with-source (vector-get node (+ 2 idx)) source env ftable instrs data-ref))

(defn compile-match-arms-with-ftable [node idx arm-count env ftable scr-idx result-local scratch-base binder-base instrs]
  (if (>= idx arm-count)
    instrs
    (let [pattern-slot (+ 3 (* idx 2))
      body-slot (+ pattern-slot 1)
      pat (vector-get node pattern-slot)
      body (vector-get node body-slot)
      bind-state (bind-match-pattern pat env binder-base)]
      (do
        (root_push bind-state)
        (let [arm-env (vector-get bind-state 0)
          pattern-temp-base (vector-get bind-state 1)
          checked (compile-match-pattern-check-with-scratch pat scr-idx scratch-base pattern-temp-base ftable instrs)
          opened (emit-to checked (op-if-empty) 0)
          bound (compile-match-pattern-binders pat scr-idx arm-env scratch-base pattern-temp-base ftable opened)
          body-instrs (compile-expr-with-ftable body arm-env ftable bound)
          stored (emit-to body-instrs (op-local-set) result-local)
          exited (emit-to stored (op-br) 1)
          else-opened (emit-to exited (op-else) 0)
          rest
            (compile-match-arms-with-ftable
              node
              (+ idx 1)
              arm-count
              env
              ftable
              scr-idx
              result-local
              scratch-base
              binder-base
              else-opened)
          closed (emit-to rest (op-end) 0)]
          (do
            (root_pop)
            closed))))))

(defn compile-match-with-ftable [node env ftable instrs]
  (compile-match-with-ftable-core node env ftable instrs))

(defn compile-match-with-ftable-core [node env ftable instrs]
  (compile-match-with-ftable-core-2 node env ftable instrs))

(defn compile-match-with-ftable-core-2 [node env ftable instrs]
  (compile-match-with-ftable-core-3 node env ftable instrs))

(defn compile-match-with-ftable-core-3 [node env ftable instrs]
  (compile-match-with-ftable-core-4 node env ftable instrs))

(defn compile-match-with-ftable-core-4 [node env ftable instrs]
  (let [arm-count (vector-get node 2)
    scr-idx (+ 1 (map-size env))
    instrs1 (compile-expr-with-ftable (vector-get node 1) env ftable instrs)
    instrs2 (emit-to instrs1 (op-local-set) scr-idx)
    scratch-base (max-root-temp-base env instrs2 (vector-new 0))
    result-local (+ scratch-base 6)
    binder-base (+ result-local 1)
    instrs3 (emit-to instrs2 (op-i64-const) 0)
    instrs4 (emit-to instrs3 (op-local-set) result-local)
    instrs5 (emit-to instrs4 (op-block) 0)
    instrs6
      (compile-match-arms-with-ftable
        node
        0
        arm-count
        env
        ftable
        scr-idx
        result-local
        scratch-base
        binder-base
        instrs5)
    instrs7 (emit-to instrs6 (op-end) 0)]
    (emit-to instrs7 (op-local-get) result-local)))
