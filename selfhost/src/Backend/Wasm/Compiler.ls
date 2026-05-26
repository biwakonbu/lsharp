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
    local-func-hash (if (= (vector-get func-node 0) 4) (vector-get func-node 1) func-hash)
    func-idx-ref (ref-new (ftable-lookup ftable local-func-hash))
    func-idx-ref-slot (root_push func-idx-ref)
    arg-instrs-list (compile-user-call-arg-instrs-with-source node source env ftable 0 arg-count (vector-new 8) data-ref)]
    (do
      (root_push arg-instrs-list)
      (let [temp-base (max-root-temp-base-list env arg-instrs-list arg-count)
        instrs1 (emit-user-call-args node arg-instrs-list 0 arg-count temp-base instrs)
        instrs2 (emit-user-call-arg-gets 0 arg-count temp-base instrs1)
        func-idx (ref-get func-idx-ref)
        instrs3 (emit-to instrs2 40 func-idx)]
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
              result)))))))

(defn compile-user-call-with-ftable [node env ftable instrs func-hash arg-count]
  (let [node-slot (root_push node)
    env-slot (root_push env)
    instrs-slot (root_push instrs)
    func-node (vector-get node 1)
    local-func-hash (if (= (vector-get func-node 0) 4) (vector-get func-node 1) func-hash)
    func-idx-ref (ref-new (ftable-lookup ftable local-func-hash))
    func-idx-ref-slot (root_push func-idx-ref)
    arg-instrs-list (compile-user-call-arg-instrs-with-ftable node env ftable 0 arg-count (vector-new 8))]
    (do
      (root_push arg-instrs-list)
      (let [temp-base (max-root-temp-base-list env arg-instrs-list arg-count)
        instrs1 (emit-user-call-args node arg-instrs-list 0 arg-count temp-base instrs)
        instrs2 (emit-user-call-arg-gets 0 arg-count temp-base instrs1)
        func-idx (ref-get func-idx-ref)
        instrs3 (emit-to instrs2 40 func-idx)]
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
              result)))))))

(defn source-builtin-map-op [bop] (if (= bop 62) true (if (= bop 63) true (if (= bop 65) true (= bop 66)))))
(defn map-insert-op [bop] (= bop 62))

(defn unary-builtin-op [bop] (if (= bop 51) true (if (= bop 52) true (if (= bop 57) true (if (= bop 61) true (if (= bop 59) true (if (= bop 64) true (if (= bop 67) true (if (= bop 73) true (if (= bop 74) true (if (= bop 54) true (= bop 56))))))))))))
(defn alloc-builtin-op [bop] (if (= bop 54) true (= bop 56)))

(defn env-slot-builtin-op [bop] (if (= bop 50) true (if (= bop 53) true (if (= bop 55) true (if (= bop 58) true (if (= bop 63) true (if (= bop 65) true (= bop 66))))))))
(defn nullary-builtin-op [bop] (= bop 75))
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
(defn compile-map-key-with-ftable [key-expr env ftable] (compile-expr-with-ftable key-expr env ftable (vector-new 8)))

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
    key-instrs (compile-map-key-with-ftable key-expr env ftable)
    map-root (alloc-root-needed map-expr)
    key-root (map-key-root-needed-with-source key-expr)
    simple-path (if (simple-map-operand map-expr) (simple-map-operand key-expr) false)]
    (do
      (root_push map-instrs)
      (root_push key-instrs)
      (if (= bop 62)
        (let [result (compile-map-insert-builtin-with-ftable node env ftable instrs bop map-instrs key-instrs map-root key-root)]
          (do
            (root_pop)
            (root_pop)
            result))
        (let [result (compile-map-lookup-builtin-with-ftable env instrs map-instrs key-instrs map-root key-root bop simple-path)]
          (do
            (root_pop)
            (root_pop)
            result))))))

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
  (let [value-local (max-root-temp-base1 env value-instrs)
    instrs1 (append-instr-vector instrs value-instrs)
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
            (compile-builtin-apply-fallback-with-source node source env ftable instrs data-ref bop safe-ftable-path))))))))

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
    key-instrs (compile-map-key-with-source key-expr source env ftable data-ref)
    map-root (alloc-root-needed map-expr)
    key-root (map-key-root-needed-with-source key-expr)
    simple-path (if (simple-map-operand map-expr) (simple-map-operand key-expr) false)]
    (do
      (root_push map-instrs)
      (root_push key-instrs)
      (if (= bop 62)
        (let [value-expr (vector-get node 5)
          value-instrs (compile-expr-with-source value-expr source env ftable (vector-new 8) data-ref)
          value-root (alloc-root-needed value-expr)]
          (do
            (root_push value-instrs)
            (let [result (compile-map-insert-builtin-instrs env instrs map-instrs key-instrs value-instrs map-root key-root value-root bop)]
              (do
                (root_pop)
                (root_pop)
                (root_pop)
                result))))
        (let [result (compile-map-lookup-builtin-with-ftable env instrs map-instrs key-instrs map-root key-root bop simple-path)]
          (do
            (root_pop)
            (root_pop)
            result))))))
(defn compile-expr-with-ftable-dispatch-complex-2-rest [tag node env ftable instrs]
  (if (= tag 8)
    (compile-lambda-with-ftable node env ftable instrs)
    (if (= tag 9)
      (compile-do-exprs node env ftable 0 (vector-get node 1) instrs)
      (if (= tag 10)
        (compile-match-with-ftable node env ftable instrs)
        (emit-to instrs 1 0)))))

(defn compile-apply-with-source [node source env ftable instrs data-ref]
  (let [func-node (vector-get node 1)
    arg-count (vector-get node 2)]
    (let [func-tag (vector-get func-node 0)
      func-hash (if (= func-tag 4) (vector-get func-node 1) 0)]
      (let [bop (builtin-opcode func-hash)]
        (if (> bop 0)
          (compile-builtin-apply-with-source node source env ftable instrs data-ref bop)
          (compile-user-call-with-source node source env ftable instrs data-ref func-hash arg-count))))))
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
(defn compile-match-with-source [node source env ftable instrs data-ref] (let [scrutinee (vector-get node 1) arm-count (vector-get node 2) scr-idx (+ 1 (map-size env)) instrs1 (compile-expr-with-source scrutinee source env ftable instrs data-ref) instrs2 (emit-to instrs1 11 scr-idx)] (if (> arm-count 0) (let [pat1 (vector-get node 3) body1 (vector-get node 4) i5 (compile-match-pattern-check pat1 scr-idx instrs2) i6 (emit-to i5 41 0) i7 (compile-expr-with-source body1 source env ftable i6 data-ref) i8 (emit-to i7 43 0)] (if (> arm-count 1) (let [pat2 (vector-get node 5) body2 (vector-get node 6) i11 (compile-match-pattern-check pat2 scr-idx i8) i12 (emit-to i11 41 0) i13 (compile-expr-with-source body2 source env ftable i12 data-ref) i14 (emit-to i13 43 0)] (if (> arm-count 2) (let [pat3 (vector-get node 7) body3 (vector-get node 8) i17 (compile-match-pattern-check pat3 scr-idx i14) i18 (emit-to i17 41 0) i19 (compile-expr-with-source body3 source env ftable i18 data-ref) i20 (emit-to i19 43 0) i21 (emit-to i20 1 0) i22 (emit-to i21 43 0) i23 (emit-to i22 43 0) i24 (emit-to i23 43 0)] i24) (let [i15 (emit-to i14 1 0) i16 (emit-to i15 43 0) i17 (emit-to i16 43 0)] i17))) (let [i9 (emit-to i8 1 0) i10 (emit-to i9 43 0)] i10))) (emit-to instrs2 1 0))))
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
                  (compile-expr-with-ftable node env ftable instrs))))))))))
(defn compile-expr-with-source [node source env ftable instrs data-ref]
  (let [node-slot (root_push node)
    source-slot (root_push source)
    env-slot (root_push env)
    ftable-slot (root_push ftable)
    instrs-slot (root_push instrs)
    data-slot (root_push data-ref)
    result (compile-expr-with-source-dispatch node source env ftable instrs data-ref)]
    (do
      (root_pop)
      (root_pop)
      (root_pop)
      (root_pop)
      (root_pop)
      (root_pop)
      result)))
(defn compile-defn-with-source [node source ftable data-ref]
  (do
    (root_push node)
    (root_push source)
    (root_push ftable)
    (root_push data-ref)
    (let [param-count (vector-get node 2)
      body-idx (+ 3 param-count)
      body-expr (vector-get node body-idx)]
      (do
        (root_push body-expr)
        (let [env (bind-node-params node 3 0 param-count (env-new) 1)]
        (do
        (root_push env)
        (let [instrs0 (vector-new 8)
          result (compile-expr-with-source body-expr source env ftable instrs0 data-ref)]
          (do
            (root_pop)
            (root_pop)
            (root_pop)
            (root_pop)
            (root_pop)
            (root_pop)
            result))))))))
(defn compile-defn-function-with-source [node source ftable data-ref]
  (do
    (root_push node)
    (root_push source)
    (root_push ftable)
    (root_push data-ref)
    (let [param-count (vector-get node 2)
      source-ir (compile-defn-with-source node source ftable data-ref)]
      (do
        (root_push source-ir)
        (let [ir (if (> (vector-length source-ir) 0) source-ir (compile-defn-with-ftable node ftable))]
          (do
            (root_push ir)
            (let [local-max (max-local-slot ir 0 (vector-length ir) 0)
              local-count (if (> local-max param-count) (- local-max param-count) 0)
              result (make-function-meta param-count local-count ir)]
              (do
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
  (let [n (vector-length decls)
    pass1 (register-defns-chunked decls 0 n (ftable-new) base-idx)
    ftable (vector-get pass1 2)
    data-ref (ref-new (vector-new 8))
    functions (compile-source-defn-functions-chunked decls 0 n src ftable data-ref (vector-new 8))
    data (ref-get data-ref)]
    (let [payload1 (push-object-vector (vector-new 3) ftable)]
      (do
        (root_push payload1)
        (let [payload2 (push-object-vector payload1 functions)]
          (do
            (root_push payload2)
            (let [payload3 (push-object-vector payload2 data)]
              (do
                (root_pop)
                (root_pop)
                payload3))))))))
(defn compile-program-functions-with-source [src decls]
  (let [n (vector-length decls)
    pass1 (register-defns-chunked decls 0 n (ftable-new) 10)
    ftable (vector-get pass1 2)
    data-ref (ref-new (vector-new 8))
    functions (compile-source-defn-functions-chunked decls 0 n src ftable data-ref (vector-new 8))
    data (ref-get data-ref)]
    (let [payload1 (push-object-vector (vector-new 3) ftable)]
      (do
        (root_push payload1)
        (let [payload2 (push-object-vector payload1 functions)]
          (do
            (root_push payload2)
            (let [payload3 (push-object-vector payload2 data)]
              (do
                (root_pop)
                (root_pop)
                payload3))))))))
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
    (do
      (let [state-slot (root_push state)]
        (do
          (let [next-idx (vector-get state 1)
            next-functions (vector-get state 2)]
            (do
              (root_push next-functions)
              (let [result (compile-defn-functions-step-with-source decls next-idx n source ftable data-ref next-functions)]
                (do
                  (root_set state-slot result)
                  (root_pop)
                  (root_pop)
                  result)))))))))

(defn continue-compile-defn-functions-step-times-with-source [decls n source ftable data-ref remaining state]
  (if (= remaining 0)
    state
    (if (= (vector-get state 0) 1)
      state
      (do
        (let [decls-slot (root_push decls)]
          (do
            (root_push source)
            (root_push ftable)
            (root_push data-ref)
            (root_push state)
            (let [next-state (continue-compile-defn-functions-step-with-source decls n source ftable data-ref state)]
              (do
                (root_push next-state)
                (let [result (continue-compile-defn-functions-step-times-with-source decls n source ftable data-ref (- remaining 1) next-state)]
                  (do
                    (root_set decls-slot result)
                    (root_pop)
                    (root_pop)
                    (root_pop)
                    (root_pop)
                    (root_pop)
                    (root_pop)
                    result))))))))))

(defn compile-defn-functions-step-8-with-source [decls idx n source ftable data-ref functions]
  (do
    (let [decls-slot (root_push decls)]
      (do
        (root_push source)
        (root_push ftable)
        (root_push data-ref)
        (root_push functions)
        (let [state (compile-defn-functions-step-with-source decls idx n source ftable data-ref functions)]
          (do
            (root_push state)
            (let [result (continue-compile-defn-functions-step-times-with-source decls n source ftable data-ref 7 state)]
              (do
                (root_set decls-slot result)
                (root_pop)
                (root_pop)
                (root_pop)
                (root_pop)
                (root_pop)
                (root_pop)
                result))))))))

(defn continue-compile-defn-functions-step-8-with-source [decls n source ftable data-ref state]
  (if (= (vector-get state 0) 1)
    state
    (do
      (let [decls-slot (root_push decls)]
        (do
          (root_push source)
          (root_push ftable)
          (root_push data-ref)
          (root_push state)
          (let [next-idx (vector-get state 1)
            next-functions (vector-get state 2)]
            (do
              (root_push next-functions)
              (let [result (compile-defn-functions-step-8-with-source decls next-idx n source ftable data-ref next-functions)]
                (do
                  (root_set decls-slot result)
                  (root_pop)
                  (root_pop)
                  (root_pop)
                  (root_pop)
                  (root_pop)
                  (root_pop)
                  result)))))))))

(defn compile-defn-functions-step-64-with-source [decls idx n source ftable data-ref functions]
  (do
    (let [decls-slot (root_push decls)]
      (do
        (root_push source)
        (root_push ftable)
        (root_push data-ref)
        (root_push functions)
        (let [state (compile-defn-functions-step-with-source decls idx n source ftable data-ref functions)]
          (do
            (root_push state)
            (let [result (continue-compile-defn-functions-step-times-with-source decls n source ftable data-ref 63 state)]
              (do
                (root_set decls-slot result)
                (root_pop)
                (root_pop)
                (root_pop)
                (root_pop)
                (root_pop)
                (root_pop)
                result))))))))

(defn continue-compile-defn-functions-step-64-with-source [decls n source ftable data-ref state]
  (if (= (vector-get state 0) 1)
    state
    (do
      (let [decls-slot (root_push decls)]
        (do
          (root_push source)
          (root_push ftable)
          (root_push data-ref)
          (root_push state)
          (let [next-idx (vector-get state 1)
            next-functions (vector-get state 2)]
            (do
              (root_push next-functions)
              (let [next-state (compile-defn-functions-step-64-with-source decls next-idx n source ftable data-ref next-functions)]
                (do
                  (root_push next-state)
                  (let [result (continue-compile-defn-functions-step-64-with-source decls n source ftable data-ref next-state)]
                    (do
                      (root_set decls-slot result)
                      (root_pop)
                      (root_pop)
                      (root_pop)
                      (root_pop)
                      (root_pop)
                      (root_pop)
                      (root_pop)
                      result)))))))))))

(defn compile-source-defn-functions-chunked [decls idx n source ftable data-ref functions]
  (let [source-chunk-progress-mode (if (> (string-length (command-line-arg 8)) 0) 1 0)]
    (do
      (root_push decls)
      (root_push source)
      (root_push ftable)
      (root_push data-ref)
      (let [functions-root (root_push functions)]
        (let [state0 (compile-defn-functions-step-64-with-source decls idx n source ftable data-ref functions)]
          (do
            (root_push state0)
            (if (= source-chunk-progress-mode 1)
              (do
                (print 213)
                (print (vector-length functions))
                (print (vector-get state0 0))
                (print (vector-get state0 1))
                (print (vector-length (vector-get state0 2))))
              (do))
            (let [state1 (continue-compile-defn-functions-step-64-with-source decls n source ftable data-ref state0)]
              (do
                (root_push state1)
                (if (= source-chunk-progress-mode 1)
                  (do
                    (print 214)
                    (print (vector-get state1 0))
                    (print (vector-get state1 1))
                    (print (vector-length (vector-get state1 2))))
                  (do))
                (let [result (vector-get state1 2)]
                  (do
                    (root_set functions-root result)
                    (root_pop)
                    (root_pop)
                    (root_pop)
                    (root_pop)
                    (root_pop)
                    (root_pop)
                    (root_pop)
                    result))))))))))
(defn continue-compile-let-chain-step-with-source [source ftable state data-ref]
  (if (= (vector-get state 0) 1)
    state
    (do
      (root_push source)
      (root_push ftable)
      (root_push state)
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
              (root_pop)
              (root_pop)
              (root_pop)
              (root_pop)
              (root_pop)
              result)))))))
(defn compile-let-chain-step-8-with-source [node source env ftable instrs data-ref rooted-count]
  (let [step1 (compile-let-chain-step-with-source node source env ftable instrs data-ref rooted-count)
    step2 (continue-compile-let-chain-step-with-source source ftable step1 data-ref)
    step3 (continue-compile-let-chain-step-with-source source ftable step2 data-ref)
    step4 (continue-compile-let-chain-step-with-source source ftable step3 data-ref)
    step5 (continue-compile-let-chain-step-with-source source ftable step4 data-ref)
    step6 (continue-compile-let-chain-step-with-source source ftable step5 data-ref)
    step7 (continue-compile-let-chain-step-with-source source ftable step6 data-ref)
    step8 (continue-compile-let-chain-step-with-source source ftable step7 data-ref)]
    step8))
(defn continue-compile-let-chain-step-8-with-source [source ftable state data-ref]
  (if (= (vector-get state 0) 1)
    state
    (do
      (root_push source)
      (root_push ftable)
      (root_push state)
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
              (root_pop)
              (root_pop)
              (root_pop)
              (root_pop)
              (root_pop)
              result)))))))
(defn compile-let-chain-step-64-with-source [node source env ftable instrs data-ref rooted-count]
  (let [step1 (compile-let-chain-step-8-with-source node source env ftable instrs data-ref rooted-count)
    step2 (continue-compile-let-chain-step-8-with-source source ftable step1 data-ref)
    step3 (continue-compile-let-chain-step-8-with-source source ftable step2 data-ref)
    step4 (continue-compile-let-chain-step-8-with-source source ftable step3 data-ref)
    step5 (continue-compile-let-chain-step-8-with-source source ftable step4 data-ref)
    step6 (continue-compile-let-chain-step-8-with-source source ftable step5 data-ref)
    step7 (continue-compile-let-chain-step-8-with-source source ftable step6 data-ref)
    step8 (continue-compile-let-chain-step-8-with-source source ftable step7 data-ref)]
    step8))
(defn compile-let-with-source [node source env ftable instrs data-ref]
  (compile-let-chain-with-source node source env ftable instrs data-ref 0))
(defn compile-defn-function [node ftable]
  (do
    (root_push node)
    (root_push ftable)
    (let [param-count (vector-get node 2)
      ir (compile-defn-with-ftable node ftable)]
      (do
        (root_push ir)
        (let [local-max (max-local-slot ir 0 (vector-length ir) 0)
          local-count (if (> local-max param-count) (- local-max param-count) 0)
          result (make-function-meta param-count local-count ir)]
          (do
            (root_pop)
            (root_pop)
            (root_pop)
            result))))))

(defn register-defns-step [decls idx n ftable func-idx]
  (if (>= idx n)
    (make-register-state 1 idx ftable func-idx)
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
                  (let [state (make-register-state 0 (+ idx 1) next-ftable (+ func-idx 1))]
                    (do
                      (root_pop)
                      (root_pop)
                      (root_pop)
                      (root_pop)
                      state))))
            (let [state (make-register-state 0 (+ idx 1) ftable func-idx)]
              (do
                (root_pop)
                (root_pop)
                (root_pop)
                state))))))))

(defn continue-register-defns-step [decls n state]
  (if (= (vector-get state 0) 1)
    state
    (do
      (root_push decls)
      (root_push state)
      (let [result (register-defns-step decls (vector-get state 1) n (vector-get state 2) (vector-get state 3))]
        (do
          (root_pop)
          (root_pop)
          result)))))

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
      (let [result (register-defns-step-8 decls (vector-get state 1) n (vector-get state 2) (vector-get state 3))]
        (do
          (root_pop)
          (root_pop)
          result)))))

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
      (let [next-state (register-defns-step-64 decls (vector-get state 1) n (vector-get state 2) (vector-get state 3))]
        (do
          (root_push next-state)
          (let [result (continue-register-defns-step-64 decls n next-state)]
            (do
              (root_pop)
              (root_pop)
              (root_pop)
              result)))))))

(defn register-defns-chunked [decls idx n ftable func-idx]
  (continue-register-defns-step-64 decls n (register-defns-step-64 decls idx n ftable func-idx)))

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
              (let [result (compile-defn-functions-step-finish functions compiled-fn idx)]
                (do
                  (root_pop)
                  (root_pop)
                  (root_pop)
                  (root_pop)
                  (root_pop)
                  result)))))
        (do
          (root_pop)
          (root_pop)
          (root_pop)
          (make-compile-step-state 0 (+ idx 1) functions))))))

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
  (continue-compile-defn-functions-step-64 decls n ftable (compile-defn-functions-step-64 decls idx n ftable functions)))

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
(defn compile-program-functions [decls] (let [n (vector-length decls) pass1 (register-defns-chunked decls 0 n (ftable-new) 0) ftable (vector-get pass1 2) functions-state (compile-defn-functions-chunked decls 0 n ftable (vector-new 8)) functions (vector-get functions-state 2)] (push-object-vector (push-object-vector (vector-new 2) ftable) functions)))
;; V2-11: import section を含む harness 向けに base offset を取る変種。
;; base-idx は user func を ftable に登録する際の起点で、selfhost ランタイム
;; (root_push 等) を import 0..base-idx-1 に置く構成と整合する。
(defn compile-program-functions-with-base [decls base-idx] (let [n (vector-length decls) pass1 (register-defns-chunked decls 0 n (ftable-new) base-idx) ftable (vector-get pass1 2) functions-state (compile-defn-functions-chunked decls 0 n ftable (vector-new 8)) functions (vector-get functions-state 2)] (push-object-vector (push-object-vector (vector-new 2) ftable) functions)))
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
  (compile-defn-functions-step-with-source-body decls idx n source ftable data-ref functions))
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
    body-expr-after-init (vector-get node 3)]
    (let [result (compile-let-chain-step-finish name-hash body-expr-after-init init-instrs env rooted-count init-root)]
      (do
        (root_pop)
        (root_pop)
        (root_pop)
        (root_pop)
        (root_pop)
        (root_pop)
        result))))
(defn compile-defn-functions-step-with-source-body-impl-3 [decls idx n source ftable data-ref functions]
  (let [source-step-progress-mode (if (> (string-length (command-line-arg 8)) 0) 1 0)]
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
            (let [compiled-fn (compile-defn-function-with-source decl source ftable data-ref)]
              (do
                (root_push compiled-fn)
                (let [next-functions (push-object-vector functions compiled-fn)]
                  (do
                    (root_set functions-slot next-functions)
                    (let [result (make-compile-step-state 0 (+ idx 1) next-functions)]
                      (do
                        (if (= source-step-progress-mode 1)
                          (if (< (vector-length functions) 128)
                            (let [result-root (root_push result)]
                              (do
                                (do
                                  (print 215)
                                  (print idx)
                                  (print (vector-length functions))
                                  (print (vector-length next-functions))
                                  (print (vector-get result 0))
                                  (print (vector-get result 1))
                                  (print (vector-length (vector-get result 2))))
                                (root_pop)))
                            (do))
                          (do))
                        (root_set functions-slot result)
                        (root_pop)
                        (root_pop)
                        (root_pop)
                        (root_pop)
                        (root_pop)
                        (root_pop)
                        (root_pop)
                        result)))))))
          (do
            (let [result (make-compile-step-state 0 (+ idx 1) functions)]
              (do
                (if (= source-step-progress-mode 1)
                  (if (< (vector-length functions) 128)
                    (let [result-root (root_push result)]
                      (do
                        (do
                          (print 216)
                          (print idx)
                          (print (vector-length functions))
                          (print (vector-get result 0))
                          (print (vector-get result 1))
                          (print (vector-length (vector-get result 2))))
                        (root_pop)))
                    (do))
                  (do))
                (root_set functions-slot result)
                (root_pop)
                (root_pop)
                (root_pop)
                (root_pop)
                (root_pop)
                result))))))))
(defn compile-let-with-ftable-impl-body-impl-3 [node env ftable instrs]
  (let [name-hash (vector-get node 1)
    init-expr (vector-get node 2)
    init-root (alloc-root-needed init-expr)
    init-instrs (compile-expr-with-ftable init-expr env ftable instrs)
    body-expr-after-init (vector-get node 3)]
    (do
      (root_push body-expr-after-init)
      (let [prep (compile-let-with-ftable-prepare name-hash init-root init-instrs env)
        new-env (vector-get prep 0)
        instrs2 (vector-get prep 1)
        body-instrs (compile-expr-with-ftable body-expr-after-init new-env ftable instrs2)]
        (let [result (maybe-root-pop-drop body-instrs init-root)]
          (do
            (root_pop)
            result))))))

(defn compile-if-with-source-impl-body-impl [node source env ftable instrs data-ref]
  (let [cond-expr (vector-get node 1)
    then-expr (vector-get node 2)
    else-expr (vector-get node 3)]
    (do
      (root_push cond-expr)
      (root_push then-expr)
      (root_push else-expr)
      (let [instrs1 (compile-expr-with-source cond-expr source env ftable instrs data-ref)
        instrs2 (emit-to instrs1 41 0)
        instrs3 (compile-expr-with-source then-expr source env ftable instrs2 data-ref)
        instrs4 (emit-to instrs3 79 0)
        instrs5 (compile-expr-with-source else-expr source env ftable instrs4 data-ref)
        result (emit-to instrs5 43 0)]
        (do
          (root_pop)
          (root_pop)
          (root_pop)
          result)))))
(defn compile-let-chain-with-source [node source env ftable instrs data-ref rooted-count]
  (let [step (compile-let-chain-step-64-with-source node source env ftable instrs data-ref rooted-count)
    next-value (vector-get step 2)]
    (if (= (vector-get step 0) 1)
      (let [body-expr (vector-get next-value 0)
        next-env (vector-get next-value 1)
        next-instrs (vector-get next-value 2)
        body-instrs (compile-expr-with-source body-expr source next-env ftable next-instrs data-ref)]
        (emit-root-pop-drops body-instrs (vector-get step 1)))
      (compile-let-chain-with-source
        (vector-get next-value 0)
        source
        (vector-get next-value 1)
        ftable
        (vector-get next-value 2)
        data-ref
        (vector-get step 1)))))
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
    func-hash (if (= (vector-get func-node 0) 4) (vector-get func-node 1) 0)
    bop (builtin-opcode func-hash)]
    (if (> bop 0)
      (compile-builtin-apply-with-ftable node env ftable instrs bop)
      (compile-user-call-with-ftable node env ftable instrs func-hash (vector-get node 2)))))

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
          (compile-expr-with-ftable-dispatch-var node env instrs)
          (compile-expr-with-ftable-dispatch-complex tag node env ftable instrs))))))

(defn compile-expr-with-ftable-dispatch-complex-2 [tag node env ftable instrs]
  (if (= tag 7)
    (compile-let-with-ftable node env ftable instrs)
    (compile-expr-with-ftable-dispatch-complex-2-rest tag node env ftable instrs)))

(defn compile-do-exprs [node env ftable idx expr-count instrs]
  (continue-compile-do-exprs node env ftable expr-count (compile-do-exprs-step-64 node env ftable idx expr-count instrs)))

(defn compile-do-expr-with-source [node source env ftable idx instrs data-ref]
  (compile-expr-with-source (vector-get node (+ 2 idx)) source env ftable instrs data-ref))

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
    instrs2 (emit-to (compile-expr-with-ftable (vector-get node 1) env ftable instrs) 11 scr-idx)]
    (if (> arm-count 0)
      (let [i5 (emit-to
        (compile-expr-with-ftable (vector-get node 4) env ftable (compile-match-arm-prefix node scr-idx 3 instrs2))
        43
        0)]
        (compile-match-with-ftable-rest node env ftable arm-count scr-idx i5))
      (emit-to instrs2 1 0))))
