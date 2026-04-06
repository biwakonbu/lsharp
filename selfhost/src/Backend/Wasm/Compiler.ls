(module Backend.Wasm.Compiler)
(import Syntax.AST)
(import IR.IR)
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
(defn op-substring [] 69)
(defn op-string-concat [] 70)
(defn op-file-exists [] 73)
(defn op-root-push [] 74)
(defn op-root-pop [] 75)
(defn op-root-set [] 76)
(defn builtin-add [] 43)
(defn builtin-sub [] 45)
(defn builtin-mul [] 42)
(defn builtin-div [] 47)
(defn builtin-eq [] 61)
(defn builtin-gt [] 62)
(defn builtin-lt [] 60)
(defn builtin-mod [] 37)
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
(defn builtin-map-contains [] (- 0 3820778934353407281))
(defn builtin-map-remove [] 2967773956947477)
(defn builtin-command-line-arg [] 4333701572691766591)
(defn builtin-file-exists [] 2680668565995926546)
(defn builtin-root-push [] 100385403511895)
(defn builtin-root-pop [] 3238238822772)
(defn builtin-root-set [] 3238238825349)
(defn builtin-basic-opcode [name-hash]
  (if (= name-hash (builtin-add))
    (op-i64-add)
    (if (= name-hash (builtin-sub))
      (op-i64-sub)
      (if (= name-hash (builtin-mul))
        (op-i64-mul)
        (if (= name-hash (builtin-div))
          (op-i64-div)
          (if (= name-hash (builtin-mod))
            24
            (if (= name-hash (builtin-eq))
              (op-i64-eq)
              (if (= name-hash (builtin-gt))
                (op-i64-gt)
                (if (= name-hash (builtin-lt))
                  (op-i64-lt)
                  (if (= name-hash 1983)
                    (op-i64-ge)
                    (if (= name-hash 1921)
                      (op-i64-le)
                      (if (= name-hash 1952)
                        (op-i64-eq)
                        0))))))))))))

(defn builtin-string-opcode [name-hash]
  (if (= name-hash (builtin-string-char-at))
    (op-string-char-at)
    (if (= name-hash (builtin-string-length))
      (op-string-length)
      (if (= name-hash (builtin-print))
        (op-print)
        (if (= name-hash 1391193566852316240)
          (op-string-concat)
          (if (= name-hash 101391823498833)
            (op-substring)
            0))))))

(defn builtin-vector-ref-opcode [name-hash]
  (if (= name-hash (builtin-vector-length))
    (op-vector-length)
    (if (= name-hash (builtin-vector-get))
      (op-vector-get)
      (if (= name-hash (builtin-vector-new))
        (op-vector-new)
        (if (= name-hash (builtin-vector-push))
          (op-vector-push)
          (if (= name-hash (builtin-ref-new))
            (op-ref-new)
            (if (= name-hash (builtin-ref-get))
              (op-ref-get)
              (if (= name-hash (builtin-ref-set))
                (op-ref-set)
                0))))))))

(defn builtin-map-core-opcode [name-hash]
  (if (= name-hash (builtin-map-new))
    (op-map-new)
    (if (= name-hash (builtin-map-size))
      (op-map-size)
      (if (= name-hash (builtin-map-get))
        (op-map-get)
        (if (= name-hash (builtin-map-insert))
          (op-map-insert)
          0)))))

(defn builtin-io-opcode [name-hash]
  (if (= name-hash (builtin-read-file))
    (op-read-file)
    (if (= name-hash (builtin-command-line-arg))
      (op-command-line-arg)
      (if (= name-hash (builtin-file-exists))
        (op-file-exists)
        0))))

(defn builtin-map-extra-opcode [name-hash]
  (if (= name-hash (builtin-map-contains))
    (op-map-contains)
    (if (= name-hash (builtin-map-remove))
      (op-map-remove)
      0)))

(defn builtin-map-runtime-opcode [name-hash]
  (let [core-op (builtin-map-core-opcode name-hash)]
    (if (> core-op 0)
      core-op
      (let [io-op (builtin-io-opcode name-hash)]
        (if (> io-op 0)
          io-op
          (builtin-map-extra-opcode name-hash))))))

(defn builtin-logic-opcode [name-hash]
  (if (= name-hash 96727)
    71
    (if (= name-hash 3555)
      72
      0)))

(defn builtin-root-opcode [name-hash]
  (if (= name-hash (builtin-root-push))
    (op-root-push)
    (if (= name-hash (builtin-root-pop))
      (op-root-pop)
      (if (= name-hash (builtin-root-set))
        (op-root-set)
        0))))

(defn builtin-runtime-opcode [name-hash]
  (let [string-op (builtin-string-opcode name-hash)]
    (if (> string-op 0)
      string-op
      (let [vector-op (builtin-vector-ref-opcode name-hash)]
        (if (> vector-op 0)
          vector-op
          (let [map-op (builtin-map-runtime-opcode name-hash)]
            (if (> map-op 0)
              map-op
              (let [logic-op (builtin-logic-opcode name-hash)]
                (if (> logic-op 0)
                  logic-op
                  (builtin-root-opcode name-hash))))))))))

(defn builtin-opcode [name-hash]
  (let [basic (builtin-basic-opcode name-hash)]
    (if (> basic 0)
      basic
      (builtin-runtime-opcode name-hash))))

(defn emit-instr [opcode operand] (vector-push (vector-push (vector-new 2) opcode) operand))
(defn emit-to [instrs opcode operand] (vector-push instrs (emit-instr opcode operand)))
(defn env-new [] (map-new))
(defn env-bind [env name-hash idx] (map-insert env name-hash idx))
(defn env-lookup [env name-hash] (map-get env name-hash))
(defn ftable-new [] (map-new))
(defn ftable-register [ftable name-hash func-idx] (map-insert ftable name-hash func-idx))
(defn ftable-lookup [ftable name-hash] (map-get ftable name-hash))
(defn make-loop-step-state [done next-idx next-value]
  (vector-push
    (vector-push
      (vector-push (vector-new 3) done)
      next-idx)
    next-value))

(defn make-bind-node-params-state [done next-param-idx next-env next-local-idx]
  (vector-push
    (vector-push
      (vector-push
        (vector-push (vector-new 4) done)
        next-param-idx)
      next-env)
    next-local-idx))

(defn bind-node-params-step [node param-base idx param-count env next-idx]
  (if (>= idx param-count)
    (make-bind-node-params-state 1 idx env next-idx)
    (make-bind-node-params-state
      0
      (+ idx 1)
      (env-bind env (vector-get node (+ param-base idx)) next-idx)
      (+ next-idx 1))))

(defn continue-bind-node-params-step [node param-base param-count state]
  (if (= (vector-get state 0) 1)
    state
    (bind-node-params-step
      node
      param-base
      (vector-get state 1)
      param-count
      (vector-get state 2)
      (vector-get state 3))))

(defn bind-node-params-step-8 [node param-base idx param-count env next-idx]
  (let [step1 (bind-node-params-step node param-base idx param-count env next-idx)
    step2 (continue-bind-node-params-step node param-base param-count step1)
    step3 (continue-bind-node-params-step node param-base param-count step2)
    step4 (continue-bind-node-params-step node param-base param-count step3)
    step5 (continue-bind-node-params-step node param-base param-count step4)
    step6 (continue-bind-node-params-step node param-base param-count step5)
    step7 (continue-bind-node-params-step node param-base param-count step6)
    step8 (continue-bind-node-params-step node param-base param-count step7)]
    step8))

(defn continue-bind-node-params-step-8 [node param-base param-count state]
  (if (= (vector-get state 0) 1)
    state
    (bind-node-params-step-8
      node
      param-base
      (vector-get state 1)
      param-count
      (vector-get state 2)
      (vector-get state 3))))

(defn bind-node-params-step-64 [node param-base idx param-count env next-idx]
  (let [step1 (bind-node-params-step-8 node param-base idx param-count env next-idx)
    step2 (continue-bind-node-params-step-8 node param-base param-count step1)
    step3 (continue-bind-node-params-step-8 node param-base param-count step2)
    step4 (continue-bind-node-params-step-8 node param-base param-count step3)
    step5 (continue-bind-node-params-step-8 node param-base param-count step4)
    step6 (continue-bind-node-params-step-8 node param-base param-count step5)
    step7 (continue-bind-node-params-step-8 node param-base param-count step6)
    step8 (continue-bind-node-params-step-8 node param-base param-count step7)]
    step8))

(defn bind-node-params [node param-base idx param-count env next-idx]
  (let [step (bind-node-params-step-64 node param-base idx param-count env next-idx)]
    (if (= (vector-get step 0) 1)
      (vector-get step 2)
      (bind-node-params
        node
        param-base
        (vector-get step 1)
        param-count
        (vector-get step 2)
        (vector-get step 3)))))
(defn make-compile-step-state [done next-idx next-value]
  (make-loop-step-state done next-idx next-value))

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
    (make-compile-step-state
      0
      (+ arg-idx 1)
      (vector-push
        arg-instrs
        (compile-expr-with-source (vector-get node (+ 3 arg-idx)) source env ftable (vector-new 8) data-ref)))))

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
  (let [step (compile-user-call-arg-instrs-step-64-with-source node source env ftable arg-idx arg-count arg-instrs data-ref)]
    (if (= (vector-get step 0) 1)
      (vector-get step 2)
      (compile-user-call-arg-instrs-with-source node source env ftable (vector-get step 1) arg-count (vector-get step 2) data-ref))))

(defn compile-user-call-arg-instrs-step-with-ftable [node env ftable arg-idx arg-count arg-instrs]
  (if (>= arg-idx arg-count)
    (make-compile-step-state 1 arg-idx arg-instrs)
    (make-compile-step-state
      0
      (+ arg-idx 1)
      (vector-push
        arg-instrs
        (compile-expr-with-ftable (vector-get node (+ 3 arg-idx)) env ftable (vector-new 8))))))

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
  (let [step (compile-user-call-arg-instrs-step-64-with-ftable node env ftable arg-idx arg-count arg-instrs)]
    (if (= (vector-get step 0) 1)
      (vector-get step 2)
      (compile-user-call-arg-instrs-with-ftable node env ftable (vector-get step 1) arg-count (vector-get step 2)))))

(defn max-local-slot-list-step [instrs-list idx count current-max]
  (if (>= idx count)
    (make-loop-step-state 1 idx current-max)
    (let [instrs (vector-get instrs-list idx)
      instrs-max (max-local-slot instrs 0 (vector-length instrs) 0)
      next-max (if (> instrs-max current-max) instrs-max current-max)]
      (make-loop-step-state 0 (+ idx 1) next-max))))

(defn continue-max-local-slot-list-step [instrs-list count state]
  (if (= (vector-get state 0) 1)
    state
    (max-local-slot-list-step instrs-list (vector-get state 1) count (vector-get state 2))))

(defn max-local-slot-list-step-8 [instrs-list idx count current-max]
  (let [step1 (max-local-slot-list-step instrs-list idx count current-max)
    step2 (continue-max-local-slot-list-step instrs-list count step1)
    step3 (continue-max-local-slot-list-step instrs-list count step2)
    step4 (continue-max-local-slot-list-step instrs-list count step3)
    step5 (continue-max-local-slot-list-step instrs-list count step4)
    step6 (continue-max-local-slot-list-step instrs-list count step5)
    step7 (continue-max-local-slot-list-step instrs-list count step6)
    step8 (continue-max-local-slot-list-step instrs-list count step7)]
    step8))

(defn continue-max-local-slot-list-step-8 [instrs-list count state]
  (if (= (vector-get state 0) 1)
    state
    (max-local-slot-list-step-8 instrs-list (vector-get state 1) count (vector-get state 2))))

(defn max-local-slot-list-step-64 [instrs-list idx count current-max]
  (let [step1 (max-local-slot-list-step-8 instrs-list idx count current-max)
    step2 (continue-max-local-slot-list-step-8 instrs-list count step1)
    step3 (continue-max-local-slot-list-step-8 instrs-list count step2)
    step4 (continue-max-local-slot-list-step-8 instrs-list count step3)
    step5 (continue-max-local-slot-list-step-8 instrs-list count step4)
    step6 (continue-max-local-slot-list-step-8 instrs-list count step5)
    step7 (continue-max-local-slot-list-step-8 instrs-list count step6)
    step8 (continue-max-local-slot-list-step-8 instrs-list count step7)]
    step8))

(defn max-local-slot-list [instrs-list idx count current-max]
  (let [step (max-local-slot-list-step-64 instrs-list idx count current-max)]
    (if (= (vector-get step 0) 1)
      (vector-get step 2)
      (max-local-slot-list instrs-list (vector-get step 1) count (vector-get step 2)))))

(defn max-root-temp-base-list [env instrs-list count]
  (let [instrs-max (max-local-slot-list instrs-list 0 count 0)
    used-max (if (> (map-size env) instrs-max) (map-size env) instrs-max)]
    (+ used-max 1)))

(defn emit-user-call-args-step [node arg-instrs-list arg-idx arg-count temp-base instrs]
  (if (>= arg-idx arg-count)
    (make-compile-step-state 1 arg-idx instrs)
    (let [arg-expr (vector-get node (+ 3 arg-idx))
      arg-instrs (vector-get arg-instrs-list arg-idx)
      arg-local (+ temp-base arg-idx)
      should-root (alloc-root-needed arg-expr)
      instrs1 (append-instr-vector instrs arg-instrs)
      instrs2 (emit-to instrs1 (op-local-set) arg-local)
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
  (let [step (emit-user-call-args-step-64 node arg-instrs-list arg-idx arg-count temp-base instrs)]
    (if (= (vector-get step 0) 1)
      (vector-get step 2)
      (emit-user-call-args node arg-instrs-list (vector-get step 1) arg-count temp-base (vector-get step 2)))))

(defn emit-user-call-arg-gets-step [arg-idx arg-count temp-base instrs]
  (if (>= arg-idx arg-count)
    (make-compile-step-state 1 arg-idx instrs)
    (make-compile-step-state 0 (+ arg-idx 1) (emit-to instrs (op-local-get) (+ temp-base arg-idx)))))

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
  (let [step (emit-user-call-arg-gets-step-64 arg-idx arg-count temp-base instrs)]
    (if (= (vector-get step 0) 1)
      (vector-get step 2)
      (emit-user-call-arg-gets (vector-get step 1) arg-count temp-base (vector-get step 2)))))

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
      (emit-user-call-root-pops node (vector-get step 1) (vector-get step 2)))))

(defn compile-user-call-with-source [node source env ftable instrs data-ref func-hash arg-count]
  (let [func-idx (ftable-lookup ftable func-hash)
    arg-instrs-list (compile-user-call-arg-instrs-with-source node source env ftable 0 arg-count (vector-new 8) data-ref)
    temp-base (max-root-temp-base-list env arg-instrs-list arg-count)
    instrs1 (emit-user-call-args node arg-instrs-list 0 arg-count temp-base instrs)
    instrs2 (emit-user-call-arg-gets 0 arg-count temp-base instrs1)
    instrs3 (emit-to instrs2 (op-call) func-idx)]
    (emit-user-call-root-pops node (- arg-count 1) instrs3)))

(defn compile-user-call-with-ftable [node env ftable instrs func-hash arg-count]
  (let [func-idx (ftable-lookup ftable func-hash)
    arg-instrs-list (compile-user-call-arg-instrs-with-ftable node env ftable 0 arg-count (vector-new 8))
    temp-base (max-root-temp-base-list env arg-instrs-list arg-count)
    instrs1 (emit-user-call-args node arg-instrs-list 0 arg-count temp-base instrs)
    instrs2 (emit-user-call-arg-gets 0 arg-count temp-base instrs1)
    instrs3 (emit-to instrs2 (op-call) func-idx)]
    (emit-user-call-root-pops node (- arg-count 1) instrs3)))
(defn source-builtin-map-op [bop] (or (= bop (op-map-insert)) (or (= bop (op-map-get)) (or (= bop (op-map-contains)) (= bop (op-map-remove))))))
(defn map-insert-op [bop] (= bop (op-map-insert)))
(defn unary-builtin-op [bop] (or (or (or (or (= bop (op-string-length)) (= bop (op-vector-length))) (= bop (op-ref-get))) (or (or (= bop (op-map-size)) (= bop (op-print))) (or (= bop (op-read-file)) (or (= bop (op-command-line-arg)) (or (= bop (op-file-exists)) (= bop (op-root-push))))))) (or (= bop (op-vector-new)) (= bop (op-ref-new)))))
(defn alloc-builtin-op [bop] (or (= bop (op-vector-new)) (= bop (op-ref-new))))
(defn env-slot-builtin-op [bop] (or (or (or (or (= bop (op-string-char-at)) (= bop (op-vector-get))) (= bop (op-vector-push))) (= bop (op-ref-set))) (or (= bop (op-map-get)) (or (= bop (op-map-contains)) (= bop (op-map-remove))))))
(defn nullary-builtin-op [bop] (= bop (op-root-pop)))
(defn ternary-builtin-op [bop] (= bop (op-substring)))
(defn append-instr-vector [dst src] (append-byte-vector dst src 0 (vector-length src)))
(defn max-root-temp-base1 [env instrs] (let [instrs-max (max-local-slot instrs 0 (vector-length instrs) 0) used-max (if (> (map-size env) instrs-max) (map-size env) instrs-max)] (+ used-max 1)))
(defn max-root-temp-base [env lhs-instrs rhs-instrs] (let [lhs-max (max-local-slot lhs-instrs 0 (vector-length lhs-instrs) 0) rhs-max (max-local-slot rhs-instrs 0 (vector-length rhs-instrs) 0) used-max1 (if (> lhs-max rhs-max) lhs-max rhs-max) used-max2 (if (> (map-size env) used-max1) (map-size env) used-max1)] (+ used-max2 1)))
(defn max-root-temp-base3 [env instrs-a instrs-b instrs-c] (let [max-a (max-local-slot instrs-a 0 (vector-length instrs-a) 0) max-b (max-local-slot instrs-b 0 (vector-length instrs-b) 0) max-c (max-local-slot instrs-c 0 (vector-length instrs-c) 0) used-max1 (if (> max-a max-b) max-a max-b) used-max2 (if (> max-c used-max1) max-c used-max1) used-max3 (if (> (map-size env) used-max2) (map-size env) used-max2)] (+ used-max3 1)))
(defn alloc-root-needed [expr] (let [tag (vector-get expr 0)] (if (= tag (tag-lit-int)) 0 (if (= tag (tag-lit-bool)) 0 1))))
(defn emit-root-push-drop [instrs local-idx] (let [instrs1 (emit-to instrs (op-local-get) local-idx) instrs2 (emit-to instrs1 (op-root-push) 0)] (emit-to instrs2 (op-drop) 0)))
(defn emit-root-pop-drop [instrs] (let [instrs1 (emit-to instrs (op-root-pop) 0)] (emit-to instrs1 (op-drop) 0)))
(defn maybe-root-push-drop [instrs should-root local-idx] (if (= should-root 0) instrs (emit-root-push-drop instrs local-idx)))
(defn maybe-root-pop-drop [instrs should-root] (if (= should-root 0) instrs (emit-root-pop-drop instrs)))
(defn map-key-root-needed-with-source [key-expr] (if (= (vector-get key-expr 0) (tag-lit-string)) 0 (alloc-root-needed key-expr)))
(defn compile-map-key-with-source [key-expr source env ftable data-ref] (if (= (vector-get key-expr 0) (tag-lit-string)) (compile-string-key-hash-with-source key-expr source (vector-new 8)) (compile-expr-with-source key-expr source env ftable (vector-new 8) data-ref)))
(defn compile-map-key-with-ftable [key-expr env ftable] (compile-expr-with-ftable key-expr env ftable (vector-new 8)))
(defn compile-ref-new-with-source [node source env ftable instrs data-ref] (let [value-expr (vector-get node 3)] (if (= (alloc-root-needed value-expr) 0) (let [instrs1 (compile-expr-with-source value-expr source env ftable instrs data-ref)] (emit-to instrs1 (op-ref-new) (+ 1 (map-size env)))) (let [value-instrs (compile-expr-with-source value-expr source env ftable (vector-new 8) data-ref) temp-base (max-root-temp-base1 env value-instrs) value-local temp-base instrs1 (append-instr-vector instrs value-instrs) instrs2 (emit-to instrs1 (op-local-set) value-local) instrs3 (emit-root-push-drop instrs2 value-local) instrs4 (emit-to instrs3 (op-local-get) value-local) instrs5 (emit-to instrs4 (op-ref-new) (+ 1 (map-size env))) instrs6 (emit-root-pop-drop instrs5)] instrs6))))
(defn compile-ref-new-with-ftable [node env ftable instrs] (let [value-expr (vector-get node 3)] (if (= (alloc-root-needed value-expr) 0) (let [instrs1 (compile-expr-with-ftable value-expr env ftable instrs)] (emit-to instrs1 (op-ref-new) (+ 1 (map-size env)))) (let [value-instrs (compile-expr-with-ftable value-expr env ftable (vector-new 8)) temp-base (max-root-temp-base1 env value-instrs) value-local temp-base instrs1 (append-instr-vector instrs value-instrs) instrs2 (emit-to instrs1 (op-local-set) value-local) instrs3 (emit-root-push-drop instrs2 value-local) instrs4 (emit-to instrs3 (op-local-get) value-local) instrs5 (emit-to instrs4 (op-ref-new) (+ 1 (map-size env))) instrs6 (emit-root-pop-drop instrs5)] instrs6))))
(defn compile-vector-push-with-source [node source env ftable instrs data-ref] (let [vector-expr (vector-get node 3) value-expr (vector-get node 4) vector-instrs (compile-expr-with-source vector-expr source env ftable (vector-new 8) data-ref) value-instrs (compile-expr-with-source value-expr source env ftable (vector-new 8) data-ref) temp-base (max-root-temp-base env vector-instrs value-instrs) vector-local temp-base value-local (+ temp-base 1) vector-root (alloc-root-needed vector-expr) value-root (alloc-root-needed value-expr) instrs1 (append-instr-vector instrs vector-instrs) instrs2 (emit-to instrs1 (op-local-set) vector-local) instrs3 (maybe-root-push-drop instrs2 vector-root vector-local) instrs4 (append-instr-vector instrs3 value-instrs) instrs5 (emit-to instrs4 (op-local-set) value-local) instrs6 (maybe-root-push-drop instrs5 value-root value-local) instrs7 (emit-to instrs6 (op-local-get) vector-local) instrs8 (emit-to instrs7 (op-local-get) value-local) instrs9 (emit-to instrs8 (op-vector-push) (+ 1 (map-size env))) instrs10 (maybe-root-pop-drop instrs9 value-root) instrs11 (maybe-root-pop-drop instrs10 vector-root)] instrs11))
(defn compile-vector-push-with-ftable [node env ftable instrs] (let [vector-expr (vector-get node 3) value-expr (vector-get node 4) vector-instrs (compile-expr-with-ftable vector-expr env ftable (vector-new 8)) value-instrs (compile-expr-with-ftable value-expr env ftable (vector-new 8)) temp-base (max-root-temp-base env vector-instrs value-instrs) vector-local temp-base value-local (+ temp-base 1) vector-root (alloc-root-needed vector-expr) value-root (alloc-root-needed value-expr) instrs1 (append-instr-vector instrs vector-instrs) instrs2 (emit-to instrs1 (op-local-set) vector-local) instrs3 (maybe-root-push-drop instrs2 vector-root vector-local) instrs4 (append-instr-vector instrs3 value-instrs) instrs5 (emit-to instrs4 (op-local-set) value-local) instrs6 (maybe-root-push-drop instrs5 value-root value-local) instrs7 (emit-to instrs6 (op-local-get) vector-local) instrs8 (emit-to instrs7 (op-local-get) value-local) instrs9 (emit-to instrs8 (op-vector-push) (+ 1 (map-size env))) instrs10 (maybe-root-pop-drop instrs9 value-root) instrs11 (maybe-root-pop-drop instrs10 vector-root)] instrs11))
(defn compile-map-builtin-with-ftable [node env ftable instrs bop] (let [map-expr (vector-get node 3) key-expr (vector-get node 4) map-instrs (compile-expr-with-ftable map-expr env ftable (vector-new 8)) key-instrs (compile-map-key-with-ftable key-expr env ftable) map-root (alloc-root-needed map-expr) key-root (alloc-root-needed key-expr)] (if (= bop (op-map-insert)) (let [value-expr (vector-get node 5) value-instrs (compile-expr-with-ftable value-expr env ftable (vector-new 8)) temp-base (max-root-temp-base3 env map-instrs key-instrs value-instrs) map-local temp-base key-local (+ temp-base 1) value-local (+ temp-base 2) value-root (alloc-root-needed value-expr) instrs1 (append-instr-vector instrs map-instrs) instrs2 (emit-to instrs1 (op-local-set) map-local) instrs3 (maybe-root-push-drop instrs2 map-root map-local) instrs4 (append-instr-vector instrs3 key-instrs) instrs5 (emit-to instrs4 (op-local-set) key-local) instrs6 (maybe-root-push-drop instrs5 key-root key-local) instrs7 (append-instr-vector instrs6 value-instrs) instrs8 (emit-to instrs7 (op-local-set) value-local) instrs9 (maybe-root-push-drop instrs8 value-root value-local) instrs10 (emit-to instrs9 (op-local-get) map-local) instrs11 (emit-to instrs10 (op-local-get) key-local) instrs12 (emit-to instrs11 (op-local-get) value-local) instrs13 (emit-to instrs12 bop (+ 1 (map-size env))) instrs14 (maybe-root-pop-drop instrs13 value-root) instrs15 (maybe-root-pop-drop instrs14 key-root) instrs16 (maybe-root-pop-drop instrs15 map-root)] instrs16) (let [temp-base (max-root-temp-base env map-instrs key-instrs) map-local temp-base key-local (+ temp-base 1) instrs1 (append-instr-vector instrs map-instrs) instrs2 (emit-to instrs1 (op-local-set) map-local) instrs3 (maybe-root-push-drop instrs2 map-root map-local) instrs4 (append-instr-vector instrs3 key-instrs) instrs5 (emit-to instrs4 (op-local-set) key-local) instrs6 (maybe-root-push-drop instrs5 key-root key-local) instrs7 (emit-to instrs6 (op-local-get) map-local) instrs8 (emit-to instrs7 (op-local-get) key-local) instrs9 (emit-to instrs8 bop (+ 1 (map-size env))) instrs10 (maybe-root-pop-drop instrs9 key-root) instrs11 (maybe-root-pop-drop instrs10 map-root)] instrs11))))
(defn compile-substring-with-source [node source env ftable instrs data-ref] (let [src-expr (vector-get node 3) start-expr (vector-get node 4) end-expr (vector-get node 5) src-instrs (compile-expr-with-source src-expr source env ftable (vector-new 8) data-ref) start-instrs (compile-expr-with-source start-expr source env ftable (vector-new 8) data-ref) end-instrs (compile-expr-with-source end-expr source env ftable (vector-new 8) data-ref) temp-base (max-root-temp-base3 env src-instrs start-instrs end-instrs) src-local temp-base instrs1 (append-instr-vector instrs src-instrs) instrs2 (emit-to instrs1 (op-local-set) src-local) instrs3 (emit-root-push-drop instrs2 src-local) instrs4 (emit-to instrs3 (op-local-get) src-local) instrs5 (append-instr-vector instrs4 start-instrs) instrs6 (append-instr-vector instrs5 end-instrs) instrs7 (emit-to instrs6 (op-substring) 0) instrs8 (emit-root-pop-drop instrs7)] instrs8))
(defn compile-substring-with-ftable [node env ftable instrs] (let [src-expr (vector-get node 3) start-expr (vector-get node 4) end-expr (vector-get node 5) src-instrs (compile-expr-with-ftable src-expr env ftable (vector-new 8)) start-instrs (compile-expr-with-ftable start-expr env ftable (vector-new 8)) end-instrs (compile-expr-with-ftable end-expr env ftable (vector-new 8)) temp-base (max-root-temp-base3 env src-instrs start-instrs end-instrs) src-local temp-base instrs1 (append-instr-vector instrs src-instrs) instrs2 (emit-to instrs1 (op-local-set) src-local) instrs3 (emit-root-push-drop instrs2 src-local) instrs4 (emit-to instrs3 (op-local-get) src-local) instrs5 (append-instr-vector instrs4 start-instrs) instrs6 (append-instr-vector instrs5 end-instrs) instrs7 (emit-to instrs6 (op-substring) 0) instrs8 (emit-root-pop-drop instrs7)] instrs8))
(defn compile-string-concat-with-source [node source env ftable instrs data-ref] (let [lhs-expr (vector-get node 3) rhs-expr (vector-get node 4) lhs-instrs (compile-expr-with-source lhs-expr source env ftable (vector-new 8) data-ref) rhs-instrs (compile-expr-with-source rhs-expr source env ftable (vector-new 8) data-ref) temp-base (max-root-temp-base env lhs-instrs rhs-instrs) lhs-local temp-base rhs-local (+ temp-base 1) instrs1 (append-instr-vector instrs lhs-instrs) instrs2 (emit-to instrs1 (op-local-set) lhs-local) instrs3 (emit-root-push-drop instrs2 lhs-local) instrs4 (append-instr-vector instrs3 rhs-instrs) instrs5 (emit-to instrs4 (op-local-set) rhs-local) instrs6 (emit-root-push-drop instrs5 rhs-local) instrs7 (emit-to instrs6 (op-local-get) lhs-local) instrs8 (emit-to instrs7 (op-local-get) rhs-local) instrs9 (emit-to instrs8 (op-string-concat) 0) instrs10 (emit-root-pop-drop instrs9) instrs11 (emit-root-pop-drop instrs10)] instrs11))
(defn compile-string-concat-with-ftable [node env ftable instrs] (let [lhs-expr (vector-get node 3) rhs-expr (vector-get node 4) lhs-instrs (compile-expr-with-ftable lhs-expr env ftable (vector-new 8)) rhs-instrs (compile-expr-with-ftable rhs-expr env ftable (vector-new 8)) temp-base (max-root-temp-base env lhs-instrs rhs-instrs) lhs-local temp-base rhs-local (+ temp-base 1) instrs1 (append-instr-vector instrs lhs-instrs) instrs2 (emit-to instrs1 (op-local-set) lhs-local) instrs3 (emit-root-push-drop instrs2 lhs-local) instrs4 (append-instr-vector instrs3 rhs-instrs) instrs5 (emit-to instrs4 (op-local-set) rhs-local) instrs6 (emit-root-push-drop instrs5 rhs-local) instrs7 (emit-to instrs6 (op-local-get) lhs-local) instrs8 (emit-to instrs7 (op-local-get) rhs-local) instrs9 (emit-to instrs8 (op-string-concat) 0) instrs10 (emit-root-pop-drop instrs9) instrs11 (emit-root-pop-drop instrs10)] instrs11))
(defn emit-unary-builtin-with-source [instrs bop env] (if (alloc-builtin-op bop) (emit-to instrs bop (+ 1 (map-size env))) (emit-to instrs bop 0)))
(defn emit-unary-builtin-with-ftable [instrs bop env] (if (alloc-builtin-op bop) (emit-to instrs bop (+ 1 (map-size env))) (emit-to instrs bop 0)))
(defn compile-binary-or-ternary-builtin-with-source [node source env ftable instrs1 data-ref bop] (let [instrs2 (compile-expr-with-source (vector-get node 4) source env ftable instrs1 data-ref)] (if (env-slot-builtin-op bop) (emit-to instrs2 bop (+ 1 (map-size env))) (if (ternary-builtin-op bop) (let [instrs3 (compile-expr-with-source (vector-get node 5) source env ftable instrs2 data-ref)] (emit-to instrs3 bop 0)) (emit-to instrs2 bop 0)))))
(defn compile-binary-or-ternary-builtin-with-ftable [node env ftable instrs1 bop] (let [instrs2 (compile-expr-with-ftable (vector-get node 4) env ftable instrs1)] (if (env-slot-builtin-op bop) (emit-to instrs2 bop (+ 1 (map-size env))) (if (map-insert-op bop) (let [instrs3 (compile-expr-with-ftable (vector-get node 5) env ftable instrs2)] (emit-to instrs3 bop (+ 1 (map-size env)))) (if (ternary-builtin-op bop) (let [instrs3 (compile-expr-with-ftable (vector-get node 5) env ftable instrs2)] (emit-to instrs3 bop 0)) (emit-to instrs2 bop 0))))))
(defn compile-builtin-apply-with-source [node source env ftable instrs data-ref bop]
  (if (= bop (op-string-concat))
    (compile-string-concat-with-source node source env ftable instrs data-ref)
    (if (= bop (op-substring))
      (compile-substring-with-source node source env ftable instrs data-ref)
      (if (= bop (op-vector-push))
        (compile-vector-push-with-source node source env ftable instrs data-ref)
        (if (= bop (op-ref-new))
          (compile-ref-new-with-source node source env ftable instrs data-ref)
          (if (= bop (op-map-new))
            (emit-to instrs bop (+ 1 (map-size env)))
            (if (nullary-builtin-op bop)
              (emit-to instrs bop 0)
              (if (source-builtin-map-op bop)
                (compile-map-builtin-with-source node source env ftable instrs data-ref bop)
                (let [instrs1 (compile-expr-with-source (vector-get node 3) source env ftable instrs data-ref)]
                  (if (unary-builtin-op bop)
                    (emit-unary-builtin-with-source instrs1 bop env)
                    (compile-binary-or-ternary-builtin-with-source node source env ftable instrs1 data-ref bop)))))))))))

(defn compile-builtin-apply-with-ftable [node env ftable instrs bop]
  (if (= bop (op-string-concat))
    (compile-string-concat-with-ftable node env ftable instrs)
    (if (= bop (op-substring))
      (compile-substring-with-ftable node env ftable instrs)
      (if (= bop (op-vector-push))
        (compile-vector-push-with-ftable node env ftable instrs)
        (if (= bop (op-ref-new))
          (compile-ref-new-with-ftable node env ftable instrs)
          (if (= bop (op-map-new))
            (emit-to instrs bop (+ 1 (map-size env)))
            (if (nullary-builtin-op bop)
              (emit-to instrs bop 0)
              (if (source-builtin-map-op bop)
                (compile-map-builtin-with-ftable node env ftable instrs bop)
                (let [instrs1 (compile-expr-with-ftable (vector-get node 3) env ftable instrs)]
                  (if (unary-builtin-op bop)
                    (emit-unary-builtin-with-ftable instrs1 bop env)
                    (compile-binary-or-ternary-builtin-with-ftable node env ftable instrs1 bop)))))))))))
(defn compile-do-exprs-step [node env ftable idx expr-count instrs]
  (if (>= idx expr-count)
    (make-compile-step-state 1 idx instrs)
    (let [value-instrs (compile-expr-with-ftable (vector-get node (+ 2 idx)) env ftable instrs)
      next-instrs (if (< (+ idx 1) expr-count) (emit-to value-instrs (op-drop) 0) value-instrs)]
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

(defn compile-do-exprs [node env ftable idx expr-count instrs]
  (let [step (compile-do-exprs-step-64 node env ftable idx expr-count instrs)]
    (if (= (vector-get step 0) 1)
      (vector-get step 2)
      (compile-do-exprs node env ftable (vector-get step 1) expr-count (vector-get step 2)))))
(defn compile-do-exprs-step-with-source [node source env ftable idx expr-count instrs data-ref]
  (if (>= idx expr-count)
    (make-compile-step-state 1 idx instrs)
    (let [value-instrs (compile-expr-with-source (vector-get node (+ 2 idx)) source env ftable instrs data-ref)
      next-instrs (if (< (+ idx 1) expr-count) (emit-to value-instrs (op-drop) 0) value-instrs)]
      (make-compile-step-state 0 (+ idx 1) next-instrs))))

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

(defn compile-do-exprs-with-source [node source env ftable idx expr-count instrs data-ref]
  (let [step (compile-do-exprs-step-64-with-source node source env ftable idx expr-count instrs data-ref)]
    (if (= (vector-get step 0) 1)
      (vector-get step 2)
      (compile-do-exprs-with-source node source env ftable (vector-get step 1) expr-count (vector-get step 2) data-ref))))
(defn string-literal-data-base [] 1024)
(defn append-byte-vector-step [dst src idx count]
  (if (>= idx count)
    (make-loop-step-state 1 idx dst)
    (make-loop-step-state 0 (+ idx 1) (vector-push dst (vector-get src idx)))))

(defn continue-append-byte-vector-step [src count state]
  (if (= (vector-get state 0) 1)
    state
    (append-byte-vector-step (vector-get state 2) src (vector-get state 1) count)))

(defn append-byte-vector-step-8 [dst src idx count]
  (let [step1 (append-byte-vector-step dst src idx count)
    step2 (continue-append-byte-vector-step src count step1)
    step3 (continue-append-byte-vector-step src count step2)
    step4 (continue-append-byte-vector-step src count step3)
    step5 (continue-append-byte-vector-step src count step4)
    step6 (continue-append-byte-vector-step src count step5)
    step7 (continue-append-byte-vector-step src count step6)
    step8 (continue-append-byte-vector-step src count step7)]
    step8))

(defn continue-append-byte-vector-step-8 [src count state]
  (if (= (vector-get state 0) 1)
    state
    (append-byte-vector-step-8 (vector-get state 2) src (vector-get state 1) count)))

(defn append-byte-vector-step-64 [dst src idx count]
  (let [step1 (append-byte-vector-step-8 dst src idx count)
    step2 (continue-append-byte-vector-step-8 src count step1)
    step3 (continue-append-byte-vector-step-8 src count step2)
    step4 (continue-append-byte-vector-step-8 src count step3)
    step5 (continue-append-byte-vector-step-8 src count step4)
    step6 (continue-append-byte-vector-step-8 src count step5)
    step7 (continue-append-byte-vector-step-8 src count step6)
    step8 (continue-append-byte-vector-step-8 src count step7)]
    step8))

(defn append-byte-vector [dst src idx count]
  (let [step (append-byte-vector-step-64 dst src idx count)]
    (if (= (vector-get step 0) 1)
      (vector-get step 2)
      (append-byte-vector (vector-get step 2) src (vector-get step 1) count))))

(defn string-to-byte-vector-step [text idx count bytes]
  (if (>= idx count)
    (make-loop-step-state 1 idx bytes)
    (make-loop-step-state 0 (+ idx 1) (vector-push bytes (string-char-at text idx)))))

(defn continue-string-to-byte-vector-step [text count state]
  (if (= (vector-get state 0) 1)
    state
    (string-to-byte-vector-step text (vector-get state 1) count (vector-get state 2))))

(defn string-to-byte-vector-step-8 [text idx count bytes]
  (let [step1 (string-to-byte-vector-step text idx count bytes)
    step2 (continue-string-to-byte-vector-step text count step1)
    step3 (continue-string-to-byte-vector-step text count step2)
    step4 (continue-string-to-byte-vector-step text count step3)
    step5 (continue-string-to-byte-vector-step text count step4)
    step6 (continue-string-to-byte-vector-step text count step5)
    step7 (continue-string-to-byte-vector-step text count step6)
    step8 (continue-string-to-byte-vector-step text count step7)]
    step8))

(defn continue-string-to-byte-vector-step-8 [text count state]
  (if (= (vector-get state 0) 1)
    state
    (string-to-byte-vector-step-8 text (vector-get state 1) count (vector-get state 2))))

(defn string-to-byte-vector-step-64 [text idx count bytes]
  (let [step1 (string-to-byte-vector-step-8 text idx count bytes)
    step2 (continue-string-to-byte-vector-step-8 text count step1)
    step3 (continue-string-to-byte-vector-step-8 text count step2)
    step4 (continue-string-to-byte-vector-step-8 text count step3)
    step5 (continue-string-to-byte-vector-step-8 text count step4)
    step6 (continue-string-to-byte-vector-step-8 text count step5)
    step7 (continue-string-to-byte-vector-step-8 text count step6)
    step8 (continue-string-to-byte-vector-step-8 text count step7)]
    step8))

(defn string-to-byte-vector [text idx count bytes]
  (let [step (string-to-byte-vector-step-64 text idx count bytes)]
    (if (= (vector-get step 0) 1)
      (vector-get step 2)
      (string-to-byte-vector text (vector-get step 1) count (vector-get step 2)))))
(defn write-i32-le [vec value] (vector-push (vector-push (vector-push (vector-push vec (% value 256)) (% (/ value 256) 256)) (% (/ value 65536) 256)) (% (/ value 16777216) 256)))
(defn compile-string-literal-with-source [node source instrs data-ref] (let [start (vector-get node 1) end (vector-get node 2) text (substring source start end) text-len (string-length text) bytes (string-to-byte-vector text 0 text-len (vector-new 8)) offset (+ (string-literal-data-base) (vector-length (ref-get data-ref))) header (write-i32-le (write-i32-le (vector-new 8) 1) text-len) data-with-header (append-byte-vector (ref-get data-ref) header 0 8) updated-data (append-byte-vector data-with-header bytes 0 (vector-length bytes)) instrs1 (emit-to instrs 1 offset)] (do (ref-set data-ref updated-data) instrs1)))
(defn string-key-hash-step [source pos end acc]
  (if (>= pos end)
    (make-loop-step-state 1 pos acc)
    (make-loop-step-state 0 (+ pos 1) (+ (string-char-at source pos) (* acc 31)))))

(defn continue-string-key-hash-step [source end state]
  (if (= (vector-get state 0) 1)
    state
    (string-key-hash-step source (vector-get state 1) end (vector-get state 2))))

(defn string-key-hash-step-8 [source pos end acc]
  (let [step1 (string-key-hash-step source pos end acc)
    step2 (continue-string-key-hash-step source end step1)
    step3 (continue-string-key-hash-step source end step2)
    step4 (continue-string-key-hash-step source end step3)
    step5 (continue-string-key-hash-step source end step4)
    step6 (continue-string-key-hash-step source end step5)
    step7 (continue-string-key-hash-step source end step6)
    step8 (continue-string-key-hash-step source end step7)]
    step8))

(defn continue-string-key-hash-step-8 [source end state]
  (if (= (vector-get state 0) 1)
    state
    (string-key-hash-step-8 source (vector-get state 1) end (vector-get state 2))))

(defn string-key-hash-step-64 [source pos end acc]
  (let [step1 (string-key-hash-step-8 source pos end acc)
    step2 (continue-string-key-hash-step-8 source end step1)
    step3 (continue-string-key-hash-step-8 source end step2)
    step4 (continue-string-key-hash-step-8 source end step3)
    step5 (continue-string-key-hash-step-8 source end step4)
    step6 (continue-string-key-hash-step-8 source end step5)
    step7 (continue-string-key-hash-step-8 source end step6)
    step8 (continue-string-key-hash-step-8 source end step7)]
    step8))

(defn string-key-hash-loop [source pos end acc]
  (let [step (string-key-hash-step-64 source pos end acc)]
    (if (= (vector-get step 0) 1)
      (vector-get step 2)
      (string-key-hash-loop source (vector-get step 1) end (vector-get step 2)))))
(defn normalize-map-key-hash [hash] (if (= hash 0) 2 (if (= hash -1) 1 hash)))
(defn compile-string-key-hash-with-source [node source instrs] (let [start (vector-get node 1) end (vector-get node 2) hash (normalize-map-key-hash (string-key-hash-loop source start end 0))] (emit-to instrs (op-i64-const) hash)))
(defn compile-map-builtin-with-source [node source env ftable instrs data-ref bop] (let [map-expr (vector-get node 3) key-expr (vector-get node 4) map-instrs (compile-expr-with-source map-expr source env ftable (vector-new 8) data-ref) key-instrs (compile-map-key-with-source key-expr source env ftable data-ref) map-root (alloc-root-needed map-expr) key-root (map-key-root-needed-with-source key-expr)] (if (= bop (op-map-insert)) (let [value-expr (vector-get node 5) value-instrs (compile-expr-with-source value-expr source env ftable (vector-new 8) data-ref) temp-base (max-root-temp-base3 env map-instrs key-instrs value-instrs) map-local temp-base key-local (+ temp-base 1) value-local (+ temp-base 2) value-root (alloc-root-needed value-expr) instrs1 (append-instr-vector instrs map-instrs) instrs2 (emit-to instrs1 (op-local-set) map-local) instrs3 (maybe-root-push-drop instrs2 map-root map-local) instrs4 (append-instr-vector instrs3 key-instrs) instrs5 (emit-to instrs4 (op-local-set) key-local) instrs6 (maybe-root-push-drop instrs5 key-root key-local) instrs7 (append-instr-vector instrs6 value-instrs) instrs8 (emit-to instrs7 (op-local-set) value-local) instrs9 (maybe-root-push-drop instrs8 value-root value-local) instrs10 (emit-to instrs9 (op-local-get) map-local) instrs11 (emit-to instrs10 (op-local-get) key-local) instrs12 (emit-to instrs11 (op-local-get) value-local) instrs13 (emit-to instrs12 bop (+ 1 (map-size env))) instrs14 (maybe-root-pop-drop instrs13 value-root) instrs15 (maybe-root-pop-drop instrs14 key-root) instrs16 (maybe-root-pop-drop instrs15 map-root)] instrs16) (let [temp-base (max-root-temp-base env map-instrs key-instrs) map-local temp-base key-local (+ temp-base 1) instrs1 (append-instr-vector instrs map-instrs) instrs2 (emit-to instrs1 (op-local-set) map-local) instrs3 (maybe-root-push-drop instrs2 map-root map-local) instrs4 (append-instr-vector instrs3 key-instrs) instrs5 (emit-to instrs4 (op-local-set) key-local) instrs6 (maybe-root-push-drop instrs5 key-root key-local) instrs7 (emit-to instrs6 (op-local-get) map-local) instrs8 (emit-to instrs7 (op-local-get) key-local) instrs9 (emit-to instrs8 bop (+ 1 (map-size env))) instrs10 (maybe-root-pop-drop instrs9 key-root) instrs11 (maybe-root-pop-drop instrs10 map-root)] instrs11))))
(defn compile-match-pattern-check [pat scr-idx instrs] (let [pat-tag (vector-get pat 0)] (if (= pat-tag (ast-pat-lit)) (let [lit (vector-get pat 1) lit-tag (vector-get lit 0)] (if (= lit-tag (ast-lit-int)) (let [i1 (emit-to instrs (op-local-get) scr-idx) i2 (emit-to i1 (op-i64-const) (vector-get lit 1))] (emit-to i2 (op-i64-eq) 0)) (if (= lit-tag (ast-lit-bool)) (let [i1 (emit-to instrs (op-local-get) scr-idx) i2 (emit-to i1 (op-i64-const) (vector-get lit 1))] (emit-to i2 (op-i64-eq) 0)) (if (= lit-tag (ast-lit-unit)) (let [i1 (emit-to instrs (op-local-get) scr-idx) i2 (emit-to i1 (op-i64-const) 0)] (emit-to i2 (op-i64-eq) 0)) (emit-to instrs (op-i64-const) 0))))) (if (or (= pat-tag (ast-pat-wildcard)) (= pat-tag (ast-pat-var))) (emit-to instrs (op-i64-const) 1) (emit-to instrs (op-i64-const) 0)))))
(defn compile-apply-with-source [node source env ftable instrs data-ref] (let [func-node (vector-get node 1) func-tag (vector-get func-node 0) func-hash (if (= func-tag (tag-var)) (vector-get func-node 1) 0) arg-count (vector-get node 2) bop (builtin-opcode func-hash)] (if (> bop 0) (compile-builtin-apply-with-source node source env ftable instrs data-ref bop) (compile-user-call-with-source node source env ftable instrs data-ref func-hash arg-count))))
(defn compile-do-with-source [node source env ftable instrs data-ref] (let [expr-count (vector-get node 1)] (if (= expr-count 0) instrs (compile-do-exprs-with-source node source env ftable 0 expr-count instrs data-ref))))
(defn compile-if-with-source [node source env ftable instrs data-ref] (let [cond-expr (vector-get node 1) then-expr (vector-get node 2) else-expr (vector-get node 3) instrs1 (compile-expr-with-source cond-expr source env ftable instrs data-ref) instrs2 (emit-to instrs1 (op-if) 0) instrs3 (compile-expr-with-source then-expr source env ftable instrs2 data-ref) instrs4 (emit-to instrs3 (op-end) 0) instrs5 (compile-expr-with-source else-expr source env ftable instrs4 data-ref)] (emit-to instrs5 (op-end) 0)))
(defn compile-let-with-source [node source env ftable instrs data-ref] (let [name-hash (vector-get node 1) init-expr (vector-get node 2) body-expr (vector-get node 3) instrs1 (compile-expr-with-source init-expr source env ftable instrs data-ref) new-idx (+ 1 (map-size env)) instrs2 (emit-to instrs1 (op-local-set) new-idx) new-env (env-bind env name-hash new-idx)] (compile-expr-with-source body-expr source new-env ftable instrs2 data-ref)))
(defn compile-lambda-with-source [node source env ftable instrs data-ref] (let [param-count (vector-get node 1) new-env (bind-node-params node 2 0 param-count env (+ 1 (map-size env)))] (compile-expr-with-source (vector-get node (+ 2 param-count)) source new-env ftable instrs data-ref)))
(defn compile-match-with-source [node source env ftable instrs data-ref] (let [scrutinee (vector-get node 1) arm-count (vector-get node 2) scr-idx (+ 1 (map-size env)) instrs1 (compile-expr-with-source scrutinee source env ftable instrs data-ref) instrs2 (emit-to instrs1 (op-local-set) scr-idx)] (if (> arm-count 0) (let [pat1 (vector-get node 3) body1 (vector-get node 4) i5 (compile-match-pattern-check pat1 scr-idx instrs2) i6 (emit-to i5 (op-if) 0) i7 (compile-expr-with-source body1 source env ftable i6 data-ref) i8 (emit-to i7 (op-end) 0)] (if (> arm-count 1) (let [pat2 (vector-get node 5) body2 (vector-get node 6) i11 (compile-match-pattern-check pat2 scr-idx i8) i12 (emit-to i11 (op-if) 0) i13 (compile-expr-with-source body2 source env ftable i12 data-ref) i14 (emit-to i13 (op-end) 0)] (if (> arm-count 2) (let [pat3 (vector-get node 7) body3 (vector-get node 8) i17 (compile-match-pattern-check pat3 scr-idx i14) i18 (emit-to i17 (op-if) 0) i19 (compile-expr-with-source body3 source env ftable i18 data-ref) i20 (emit-to i19 (op-end) 0) i21 (emit-to i20 (op-i64-const) 0) i22 (emit-to i21 (op-end) 0) i23 (emit-to i22 (op-end) 0) i24 (emit-to i23 (op-end) 0)] i24) (let [i15 (emit-to i14 (op-i64-const) 0) i16 (emit-to i15 (op-end) 0) i17 (emit-to i16 (op-end) 0)] i17))) (let [i9 (emit-to i8 (op-i64-const) 0) i10 (emit-to i9 (op-end) 0)] i10))) (emit-to instrs2 (op-i64-const) 0))))
(defn compile-expr-with-source [node source env ftable instrs data-ref] (let [tag (vector-get node 0)] (if (= tag (tag-lit-string)) (compile-string-literal-with-source node source instrs data-ref) (if (= tag (tag-do)) (compile-do-with-source node source env ftable instrs data-ref) (if (= tag (tag-if)) (compile-if-with-source node source env ftable instrs data-ref) (if (= tag (tag-apply)) (compile-apply-with-source node source env ftable instrs data-ref) (if (= tag (tag-let)) (compile-let-with-source node source env ftable instrs data-ref) (if (= tag (tag-lambda)) (compile-lambda-with-source node source env ftable instrs data-ref) (if (= tag (tag-match)) (compile-match-with-source node source env ftable instrs data-ref) (compile-expr-with-ftable node env ftable instrs))))))))))
(defn compile-defn-with-source [node source ftable data-ref] (let [param-count (vector-get node 2) env (bind-node-params node 3 0 param-count (env-new) 1) body-idx (+ 3 param-count) body-expr (vector-get node body-idx)] (compile-expr-with-source body-expr source env ftable (vector-new 8) data-ref)))
(defn compile-expr-with-ftable [node env ftable instrs] (let [tag (vector-get node 0)] (if (= tag 1) (emit-to instrs 1 (vector-get node 1)) (if (= tag 2) (emit-to instrs 1 (vector-get node 1)) (if (= tag 4) (let [name-hash (vector-get node 1) idx (env-lookup env name-hash)] (if (= idx 0) (emit-to instrs 1 0) (emit-to instrs 10 idx))) (if (= tag 5) (let [func-node (vector-get node 1) func-tag (vector-get func-node 0) func-hash (if (= func-tag 4) (vector-get func-node 1) 0) arg-count (vector-get node 2) bop (builtin-opcode func-hash)] (if (> bop 0) (compile-builtin-apply-with-ftable node env ftable instrs bop) (compile-user-call-with-ftable node env ftable instrs func-hash arg-count))) (if (= tag 6) (let [cond-expr (vector-get node 1) then-expr (vector-get node 2) else-expr (vector-get node 3) instrs1 (compile-expr-with-ftable cond-expr env ftable instrs) instrs2 (emit-to instrs1 41 0) instrs3 (compile-expr-with-ftable then-expr env ftable instrs2) instrs4 (emit-to instrs3 43 0) instrs5 (compile-expr-with-ftable else-expr env ftable instrs4)] (emit-to instrs5 43 0)) (if (= tag 7) (let [name-hash (vector-get node 1) init-expr (vector-get node 2) body-expr (vector-get node 3) instrs1 (compile-expr-with-ftable init-expr env ftable instrs) new-idx (+ 1 (map-size env)) instrs2 (emit-to instrs1 11 new-idx) new-env (env-bind env name-hash new-idx)] (compile-expr-with-ftable body-expr new-env ftable instrs2)) (if (= tag 8) (let [param-count (vector-get node 1) new-env (bind-node-params node 2 0 param-count env (+ 1 (map-size env)))] (compile-expr-with-ftable (vector-get node (+ 2 param-count)) new-env ftable instrs)) (if (= tag 9) (let [expr-count (vector-get node 1)] (compile-do-exprs node env ftable 0 expr-count instrs)) (if (= tag 10) (let [scrutinee (vector-get node 1) arm-count (vector-get node 2) scr-idx (+ 1 (map-size env)) instrs1 (compile-expr-with-ftable scrutinee env ftable instrs) instrs2 (emit-to instrs1 11 scr-idx)] (if (> arm-count 0) (let [pat1 (vector-get node 3) body1 (vector-get node 4) i3 (emit-to instrs2 10 scr-idx) i4 (emit-to i3 1 pat1) i5 (emit-to i4 30 0) i6 (emit-to i5 41 0) i7 (compile-expr-with-ftable body1 env ftable i6) i8 (emit-to i7 43 0)] (if (> arm-count 1) (let [pat2 (vector-get node 5) body2 (vector-get node 6) i9 (emit-to i8 10 scr-idx) i10 (emit-to i9 1 pat2) i11 (emit-to i10 30 0) i12 (emit-to i11 41 0) i13 (compile-expr-with-ftable body2 env ftable i12) i14 (emit-to i13 43 0)] (if (> arm-count 2) (let [pat3 (vector-get node 7) body3 (vector-get node 8) i15 (emit-to i14 10 scr-idx) i16 (emit-to i15 1 pat3) i17 (emit-to i16 30 0) i18 (emit-to i17 41 0) i19 (compile-expr-with-ftable body3 env ftable i18) i20 (emit-to i19 43 0) i21 (emit-to i20 1 0) i22 (emit-to i21 43 0)] (emit-to i22 43 0)) (let [i15 (emit-to i14 1 0) i16 (emit-to i15 43 0)] (emit-to i16 43 0)))) (let [i9 (emit-to i8 1 0) i10 (emit-to i9 43 0)] i10))) (emit-to instrs2 1 0))) (emit-to instrs 1 0))))))))))))
(defn compile-expr [node env instrs] (compile-expr-with-ftable node env (ftable-new) instrs))
(defn compile-defn-with-ftable [node ftable] (let [param-count (vector-get node 2) env (bind-node-params node 3 0 param-count (env-new) 1) body-idx (+ 3 param-count) body-expr (vector-get node body-idx)] (compile-expr-with-ftable body-expr env ftable (vector-new 8))))
(defn compile-defn [node] (compile-defn-with-ftable node (ftable-new)))
(defn compile-defn-function-with-source [node source ftable data-ref] (let [param-count (vector-get node 2) ir (compile-defn-with-source node source ftable data-ref) local-max (max-local-slot ir 0 (vector-length ir) 0) local-count (if (> local-max param-count) (- local-max param-count) 0)] (make-function-meta param-count local-count ir)))
(defn compile-defn-functions-step-with-source [decls idx n source ftable data-ref functions]
  (if (>= idx n)
    (make-compile-step-state 1 idx functions)
    (let [decl (vector-get decls idx)
      next-functions (if (= (vector-get decl 0) 20) (vector-push functions (compile-defn-function-with-source decl source ftable data-ref)) functions)]
      (make-compile-step-state 0 (+ idx 1) next-functions))))

(defn continue-compile-defn-functions-step-with-source [decls n source ftable data-ref state]
  (if (= (vector-get state 0) 1)
    state
    (compile-defn-functions-step-with-source decls (vector-get state 1) n source ftable data-ref (vector-get state 2))))

(defn compile-defn-functions-step-8-with-source [decls idx n source ftable data-ref functions]
  (let [step1 (compile-defn-functions-step-with-source decls idx n source ftable data-ref functions)
    step2 (continue-compile-defn-functions-step-with-source decls n source ftable data-ref step1)
    step3 (continue-compile-defn-functions-step-with-source decls n source ftable data-ref step2)
    step4 (continue-compile-defn-functions-step-with-source decls n source ftable data-ref step3)
    step5 (continue-compile-defn-functions-step-with-source decls n source ftable data-ref step4)
    step6 (continue-compile-defn-functions-step-with-source decls n source ftable data-ref step5)
    step7 (continue-compile-defn-functions-step-with-source decls n source ftable data-ref step6)
    step8 (continue-compile-defn-functions-step-with-source decls n source ftable data-ref step7)]
    step8))

(defn continue-compile-defn-functions-step-8-with-source [decls n source ftable data-ref state]
  (if (= (vector-get state 0) 1)
    state
    (compile-defn-functions-step-8-with-source decls (vector-get state 1) n source ftable data-ref (vector-get state 2))))

(defn compile-defn-functions-step-64-with-source [decls idx n source ftable data-ref functions]
  (let [step1 (compile-defn-functions-step-8-with-source decls idx n source ftable data-ref functions)
    step2 (continue-compile-defn-functions-step-8-with-source decls n source ftable data-ref step1)
    step3 (continue-compile-defn-functions-step-8-with-source decls n source ftable data-ref step2)
    step4 (continue-compile-defn-functions-step-8-with-source decls n source ftable data-ref step3)
    step5 (continue-compile-defn-functions-step-8-with-source decls n source ftable data-ref step4)
    step6 (continue-compile-defn-functions-step-8-with-source decls n source ftable data-ref step5)
    step7 (continue-compile-defn-functions-step-8-with-source decls n source ftable data-ref step6)
    step8 (continue-compile-defn-functions-step-8-with-source decls n source ftable data-ref step7)]
    step8))

(defn compile-defn-functions-with-source [decls idx n source ftable data-ref functions]
  (let [step (compile-defn-functions-step-64-with-source decls idx n source ftable data-ref functions)]
    (if (= (vector-get step 0) 1)
      (vector-get step 2)
      (compile-defn-functions-with-source decls (vector-get step 1) n source ftable data-ref (vector-get step 2)))))
(defn compile-program-functions-with-source [src decls] (let [n (vector-length decls) pass1 (register-defns decls 0 n (ftable-new) 0) ftable (vector-get pass1 0) data-ref (ref-new (vector-new 8)) functions (compile-defn-functions-with-source decls 0 n src ftable data-ref (vector-new 8))] (vector-push (vector-push (vector-push (vector-new 3) ftable) functions) (ref-get data-ref))))
(defn compile-program-with-source [src decls] (let [pair (compile-program-functions-with-source src decls) ftable (vector-get pair 0) functions (vector-get pair 1) data (vector-get pair 2) ir-list (collect-function-irs functions 0 (vector-length functions) (vector-new 8))] (vector-push (vector-push (vector-push (vector-new 3) ftable) ir-list) data)))
(defn max-local-slot-op [opcode operand current-max] (if (or (or (= opcode 10) (= opcode 11)) (or (= opcode 50) (= opcode 53))) (if (> operand current-max) operand current-max) (if (= opcode 54) (if (> (+ operand 1) current-max) (+ operand 1) current-max) (if (= opcode 55) (if (> (+ operand 5) current-max) (+ operand 5) current-max) (if (= opcode 56) (if (> (+ operand 1) current-max) (+ operand 1) current-max) (if (= opcode 58) (if (> operand current-max) operand current-max) (if (= opcode 60) (if (> operand current-max) operand current-max) (if (= opcode 62) (if (> (+ operand 5) current-max) (+ operand 5) current-max) (if (= opcode 63) (if (> (+ operand 5) current-max) (+ operand 5) current-max) (if (= opcode 65) (if (> (+ operand 5) current-max) (+ operand 5) current-max) (if (= opcode 66) (if (> (+ operand 5) current-max) (+ operand 5) current-max) current-max)))))))))))
(defn max-local-slot-step [instrs idx count current-max]
  (if (>= idx count)
    (make-compile-step-state 1 idx current-max)
    (let [instr (vector-get instrs idx)
      opcode (vector-get instr 0)
      operand (vector-get instr 1)
      next-max (max-local-slot-op opcode operand current-max)]
      (make-compile-step-state 0 (+ idx 1) next-max))))

(defn continue-max-local-slot-step [instrs count state]
  (if (= (vector-get state 0) 1)
    state
    (max-local-slot-step instrs (vector-get state 1) count (vector-get state 2))))

(defn max-local-slot-step-8 [instrs idx count current-max]
  (let [step1 (max-local-slot-step instrs idx count current-max)
    step2 (continue-max-local-slot-step instrs count step1)
    step3 (continue-max-local-slot-step instrs count step2)
    step4 (continue-max-local-slot-step instrs count step3)
    step5 (continue-max-local-slot-step instrs count step4)
    step6 (continue-max-local-slot-step instrs count step5)
    step7 (continue-max-local-slot-step instrs count step6)
    step8 (continue-max-local-slot-step instrs count step7)]
    step8))

(defn continue-max-local-slot-step-8 [instrs count state]
  (if (= (vector-get state 0) 1)
    state
    (max-local-slot-step-8 instrs (vector-get state 1) count (vector-get state 2))))

(defn max-local-slot-step-64 [instrs idx count current-max]
  (let [step1 (max-local-slot-step-8 instrs idx count current-max)
    step2 (continue-max-local-slot-step-8 instrs count step1)
    step3 (continue-max-local-slot-step-8 instrs count step2)
    step4 (continue-max-local-slot-step-8 instrs count step3)
    step5 (continue-max-local-slot-step-8 instrs count step4)
    step6 (continue-max-local-slot-step-8 instrs count step5)
    step7 (continue-max-local-slot-step-8 instrs count step6)
    step8 (continue-max-local-slot-step-8 instrs count step7)]
    step8))

(defn max-local-slot [instrs idx count current-max]
  (let [step (max-local-slot-step-64 instrs idx count current-max)]
    (if (= (vector-get step 0) 1)
      (vector-get step 2)
      (max-local-slot instrs (vector-get step 1) count (vector-get step 2)))))
(defn make-function-meta [param-count local-count ir] (vector-push (vector-push (vector-push (vector-new 3) param-count) local-count) ir))
(defn compile-defn-function [node ftable] (let [param-count (vector-get node 2) ir (compile-defn-with-ftable node ftable) local-max (max-local-slot ir 0 (vector-length ir) 0) local-count (if (> local-max param-count) (- local-max param-count) 0)] (make-function-meta param-count local-count ir)))
(defn make-register-state [done next-idx next-ftable next-func-idx]
  (vector-push
    (vector-push
      (vector-push
        (vector-push (vector-new 4) done)
        next-idx)
      next-ftable)
    next-func-idx))

(defn register-defns-step [decls idx n ftable func-idx]
  (if (>= idx n)
    (make-register-state 1 idx ftable func-idx)
    (let [decl (vector-get decls idx)]
      (if (= (vector-get decl 0) 20)
        (make-register-state 0 (+ idx 1) (ftable-register ftable (vector-get decl 1) func-idx) (+ func-idx 1))
        (make-register-state 0 (+ idx 1) ftable func-idx)))))

(defn continue-register-defns-step [decls n state]
  (if (= (vector-get state 0) 1)
    state
    (register-defns-step decls (vector-get state 1) n (vector-get state 2) (vector-get state 3))))

(defn register-defns-step-8 [decls idx n ftable func-idx]
  (let [step1 (register-defns-step decls idx n ftable func-idx)
    step2 (continue-register-defns-step decls n step1)
    step3 (continue-register-defns-step decls n step2)
    step4 (continue-register-defns-step decls n step3)
    step5 (continue-register-defns-step decls n step4)
    step6 (continue-register-defns-step decls n step5)
    step7 (continue-register-defns-step decls n step6)
    step8 (continue-register-defns-step decls n step7)]
    step8))

(defn continue-register-defns-step-8 [decls n state]
  (if (= (vector-get state 0) 1)
    state
    (register-defns-step-8 decls (vector-get state 1) n (vector-get state 2) (vector-get state 3))))

(defn register-defns-step-64 [decls idx n ftable func-idx]
  (let [step1 (register-defns-step-8 decls idx n ftable func-idx)
    step2 (continue-register-defns-step-8 decls n step1)
    step3 (continue-register-defns-step-8 decls n step2)
    step4 (continue-register-defns-step-8 decls n step3)
    step5 (continue-register-defns-step-8 decls n step4)
    step6 (continue-register-defns-step-8 decls n step5)
    step7 (continue-register-defns-step-8 decls n step6)
    step8 (continue-register-defns-step-8 decls n step7)]
    step8))

(defn register-defns [decls idx n ftable func-idx]
  (let [step (register-defns-step-64 decls idx n ftable func-idx)]
    (if (= (vector-get step 0) 1)
      (vector-push (vector-push (vector-new 2) (vector-get step 2)) (vector-get step 3))
      (register-defns decls (vector-get step 1) n (vector-get step 2) (vector-get step 3)))))
(defn compile-defn-functions-step [decls idx n ftable functions]
  (if (>= idx n)
    (make-compile-step-state 1 idx functions)
    (let [decl (vector-get decls idx)
      next-functions (if (= (vector-get decl 0) 20) (vector-push functions (compile-defn-function decl ftable)) functions)]
      (make-compile-step-state 0 (+ idx 1) next-functions))))

(defn continue-compile-defn-functions-step [decls n ftable state]
  (if (= (vector-get state 0) 1)
    state
    (compile-defn-functions-step decls (vector-get state 1) n ftable (vector-get state 2))))

(defn compile-defn-functions-step-8 [decls idx n ftable functions]
  (let [step1 (compile-defn-functions-step decls idx n ftable functions)
    step2 (continue-compile-defn-functions-step decls n ftable step1)
    step3 (continue-compile-defn-functions-step decls n ftable step2)
    step4 (continue-compile-defn-functions-step decls n ftable step3)
    step5 (continue-compile-defn-functions-step decls n ftable step4)
    step6 (continue-compile-defn-functions-step decls n ftable step5)
    step7 (continue-compile-defn-functions-step decls n ftable step6)
    step8 (continue-compile-defn-functions-step decls n ftable step7)]
    step8))

(defn continue-compile-defn-functions-step-8 [decls n ftable state]
  (if (= (vector-get state 0) 1)
    state
    (compile-defn-functions-step-8 decls (vector-get state 1) n ftable (vector-get state 2))))

(defn compile-defn-functions-step-64 [decls idx n ftable functions]
  (let [step1 (compile-defn-functions-step-8 decls idx n ftable functions)
    step2 (continue-compile-defn-functions-step-8 decls n ftable step1)
    step3 (continue-compile-defn-functions-step-8 decls n ftable step2)
    step4 (continue-compile-defn-functions-step-8 decls n ftable step3)
    step5 (continue-compile-defn-functions-step-8 decls n ftable step4)
    step6 (continue-compile-defn-functions-step-8 decls n ftable step5)
    step7 (continue-compile-defn-functions-step-8 decls n ftable step6)
    step8 (continue-compile-defn-functions-step-8 decls n ftable step7)]
    step8))

(defn compile-defn-functions [decls idx n ftable functions]
  (let [step (compile-defn-functions-step-64 decls idx n ftable functions)]
    (if (= (vector-get step 0) 1)
      (vector-get step 2)
      (compile-defn-functions decls (vector-get step 1) n ftable (vector-get step 2)))))
(defn collect-function-irs-step [functions idx count ir-list]
  (if (>= idx count)
    (make-compile-step-state 1 idx ir-list)
    (make-compile-step-state 0 (+ idx 1) (vector-push ir-list (vector-get (vector-get functions idx) 2)))))

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

(defn collect-function-irs [functions idx count ir-list]
  (let [step (collect-function-irs-step-64 functions idx count ir-list)]
    (if (= (vector-get step 0) 1)
      (vector-get step 2)
      (collect-function-irs functions (vector-get step 1) count (vector-get step 2)))))
(defn compile-program-functions [decls] (let [n (vector-length decls) pass1 (register-defns decls 0 n (ftable-new) 0) ftable (vector-get pass1 0) functions (compile-defn-functions decls 0 n ftable (vector-new 8))] (vector-push (vector-push (vector-new 2) ftable) functions)))
(defn compile-program [decls] (let [pair (compile-program-functions decls) ftable (vector-get pair 0) functions (vector-get pair 1) ir-list (collect-function-irs functions 0 (vector-length functions) (vector-new 8))] (vector-push (vector-push (vector-new 2) ftable) ir-list)))
(defn lower [x] (let [n (vector-length x)] (if (= n 0) (vector-new 0) (if (and (= n 2) (or (= (vector-get x 0) 1) (= (vector-get x 0) 2))) (compile-expr x (env-new) (vector-new 8)) (let [pair (compile-program x) ir-list (vector-get pair 1)] (if (> (vector-length ir-list) 0) (vector-get ir-list 0) (vector-new 0)))))))
(defn compile-function [param-hashes body] (let [env (ref-new (env-new)) idx (ref-new 1) i (ref-new 0) n (vector-length param-hashes)] (do (let [loop-done (ref-new 0)] (do (let [loop-body (ref-new 0)] (do (ref-set loop-body 1) (if (< (ref-get i) n) (do (ref-set env (env-bind (ref-get env) (vector-get param-hashes (ref-get i)) (ref-get idx))) (ref-set idx (+ (ref-get idx) 1)) (ref-set i (+ (ref-get i) 1)) (if (< (ref-get i) n) (do (ref-set env (env-bind (ref-get env) (vector-get param-hashes (ref-get i)) (ref-get idx))) (ref-set idx (+ (ref-get idx) 1)) (ref-set i (+ (ref-get i) 1)) 0) 0)) 0))) 0)) (compile-expr body (ref-get env) (vector-new 8)))))
(defn leb128-unsigned [value] (let [result (ref-new (vector-new 4)) v (ref-new value) done (ref-new 0)] (do (let [byte (% (ref-get v) 128) rest (/ (ref-get v) 128)] (if (= rest 0) (do (ref-set result (vector-push (ref-get result) byte)) (ref-set done 1) 0) (do (ref-set result (vector-push (ref-get result) (+ byte 128))) (ref-set v rest) (let [byte2 (% (ref-get v) 128) rest2 (/ (ref-get v) 128)] (if (= rest2 0) (do (ref-set result (vector-push (ref-get result) byte2)) (ref-set done 1) 0) (do (ref-set result (vector-push (ref-get result) (+ byte2 128))) (ref-set v rest2) (let [byte3 (% (ref-get v) 128)] (do (ref-set result (vector-push (ref-get result) byte3)) 0))))) 0))) (ref-get result))))
(defn main [] (let [lit-node (vector-push (vector-push (vector-new 2) 1) 42) env (env-new) instrs (compile-expr lit-node env (vector-new 8)) do-node (let [n (vector-new 8)] (let [n1 (vector-push n 9) n2 (vector-push n1 2) e1 (vector-push (vector-push (vector-new 2) 1) 10) n3 (vector-push n2 e1) e2 (vector-push (vector-push (vector-new 2) 1) 20) n4 (vector-push n3 e2)] n4)) do-instrs (compile-expr do-node env (vector-new 8)) leb-small (leb128-unsigned 5) leb-medium (leb128-unsigned 300) add-node (let [n (vector-new 8)] (let [n1 (vector-push n 5) n2 (vector-push n1 43) n3 (vector-push n2 2) a1 (vector-push (vector-push (vector-new 2) 1) 3) n4 (vector-push n3 a1) a2 (vector-push (vector-push (vector-new 2) 1) 4) n5 (vector-push n4 a2)] n5)) add-instrs (compile-expr add-node env (vector-new 8))] (do (print (vector-length instrs)) (let [instr0 (vector-get instrs 0)] (do (print (vector-get instr0 0)) (print (vector-get instr0 1)))) (print (vector-length do-instrs)) (print (vector-length leb-small)) (print (vector-get leb-small 0)) (print (vector-length leb-medium)) (print (vector-get leb-medium 0)) (print (vector-get leb-medium 1)) (print (vector-length add-instrs)) (let [ai0 (vector-get add-instrs 0) ai1 (vector-get add-instrs 1) ai2 (vector-get add-instrs 2)] (do (print (vector-get ai0 0)) (print (vector-get ai0 1)) (print (vector-get ai1 0)) (print (vector-get ai1 1)) (print (vector-get ai2 0)) 0)) 0)))
