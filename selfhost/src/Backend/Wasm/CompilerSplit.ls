(module Backend.Wasm.CompilerSplit)
(import Syntax.AST)
(import Backend.Wasm.CompilerBase)

(defn append-instr-vector-loop [dst src idx count]
  (if (>= idx count)
    dst
    (let [instr (vector-get src idx)]
      (do
        (root_push instr)
        (let [next-dst (push-object-vector dst instr)]
          (do
            (root_pop)
            (root_push next-dst)
            (let [result (append-instr-vector-loop next-dst src (+ idx 1) count)]
              (do
                (root_pop)
                result))))))))
(defn append-instr-vector [dst src]
  (do
    (root_push src)
    (let [result (append-instr-vector-loop dst src 0 (vector-length src))]
      (do
        (root_pop)
        result))))

(defn max-local-slot-list-step [instrs-list idx count current-max]
  (if (>= idx count)
    (make-loop-step-state 1 idx current-max)
    (let [instrs (vector-get instrs-list idx)]
      (do
        (root_push instrs)
        (let [instrs-max (max-local-slot instrs 0 (vector-length instrs) 0)
          next-max (if (> instrs-max current-max) instrs-max current-max)]
          (do
            (root_pop)
            (make-loop-step-state 0 (+ idx 1) next-max)))))))

(defn continue-max-local-slot-list-step [instrs-list count state]
  (if (= (vector-get state 0) 1)
    state
    (max-local-slot-list-step instrs-list (vector-get state 1) count (vector-get state 2))))

(defn continue-max-local-slot-list-step-times [instrs-list count remaining state]
  (if (= remaining 0)
    state
    (if (= (vector-get state 0) 1)
      state
      (do
        (root_push instrs-list)
        (root_push state)
        (let [next-state (continue-max-local-slot-list-step instrs-list count state)]
          (do
            (root_push next-state)
            (let [result (continue-max-local-slot-list-step-times instrs-list count (- remaining 1) next-state)]
              (do
                (root_pop)
                (root_pop)
                (root_pop)
                result))))))))

(defn max-local-slot-list-step-8 [instrs-list idx count current-max]
  (do
    (root_push instrs-list)
    (let [state (max-local-slot-list-step instrs-list idx count current-max)]
      (do
        (root_push state)
        (let [result (continue-max-local-slot-list-step-times instrs-list count 7 state)]
          (do
            (root_pop)
            (root_pop)
            result))))))

(defn continue-max-local-slot-list-step-8 [instrs-list count state]
  (if (= (vector-get state 0) 1)
    state
    (max-local-slot-list-step-8 instrs-list (vector-get state 1) count (vector-get state 2))))

(defn max-local-slot-list-step-64 [instrs-list idx count current-max]
  (do
    (root_push instrs-list)
    (let [state (max-local-slot-list-step instrs-list idx count current-max)]
      (do
        (root_push state)
        (let [result (continue-max-local-slot-list-step-times instrs-list count 63 state)]
          (do
            (root_pop)
            (root_pop)
            result))))))

(defn max-local-slot-list [instrs-list idx count current-max]
  (let [step (max-local-slot-list-step-64 instrs-list idx count current-max)]
    (if (= (vector-get step 0) 1)
      (vector-get step 2)
      (max-local-slot-list instrs-list (vector-get step 1) count (vector-get step 2)))))

(defn max-local-slot-direct [instrs idx count current-max]
  (if (>= idx count)
    current-max
    (let [instr (vector-get instrs idx)
      opcode (vector-get instr 0)
      operand (vector-get instr 1)
      next-max (max-local-slot-op opcode operand current-max)]
      (max-local-slot-direct instrs (+ idx 1) count next-max))))

(defn max-local-slot-list-direct [instrs-list idx count current-max]
  (if (>= idx count)
    current-max
    (let [instrs (vector-get instrs-list idx)
      instrs-max (max-local-slot-direct instrs 0 (vector-length instrs) 0)
      next-max (if (> instrs-max current-max) instrs-max current-max)]
      (max-local-slot-list-direct instrs-list (+ idx 1) count next-max))))

(defn max-root-temp-base-list [env instrs-list count]
  (let [env-size (map-size env)
    instrs-max (max-local-slot-list-direct instrs-list 0 count 0)
    used-max (if (> env-size instrs-max) env-size instrs-max)]
    (+ used-max 1)))

(defn max-root-temp-base1 [env instrs]
  (do
    (root_push instrs)
    (let [instrs-max (max-local-slot instrs 0 (vector-length instrs) 0)
      used-max (if (> (map-size env) instrs-max) (map-size env) instrs-max)]
      (do
        (root_pop)
        (+ used-max 1)))))

(defn map-key-root-needed-with-source [key-expr]
  (if (simple-map-operand key-expr)
    0
    (alloc-root-needed key-expr)))

(defn finish-compile-do-exprs-step [idx expr-count value-instrs]
  (let [next-instrs (if (< (+ idx 1) expr-count) (emit-to value-instrs (op-drop) 0) value-instrs)]
    (make-compile-step-state 0 (+ idx 1) next-instrs)))

(defn compile-expr-with-ftable-dispatch-var [node env instrs]
  (let [idx (env-lookup env (vector-get node 1))]
    (if (= idx 0)
      (emit-to instrs 1 0)
      (emit-to instrs 10 idx))))

(defn make-match-bind-state [env next-local]
  (push-int-vector (push-object-vector (vector-new 2) env) next-local))

(defn bind-match-pattern-children [pat idx count env next-local]
  (if (>= idx count)
    (make-match-bind-state env next-local)
    (let [child (vector-get pat (+ 3 (* idx 2)))]
      (do
        (root_push pat)
        (root_push child)
        (root_push env)
        (let [state (bind-match-pattern child env next-local)]
          (do
            (root_push state)
            (let [result
              (bind-match-pattern-children
                pat
                (+ idx 1)
                count
                (vector-get state 0)
                (vector-get state 1))]
              (do
                (root_pop)
                (root_pop)
                (root_pop)
                (root_pop)
                result))))))))

(defn bind-match-pattern [pat env next-local]
  (let [tag (vector-get pat 0)]
    (if (or (= tag (ast-pat-var)) (= tag 4))
      (make-match-bind-state
        (env-bind env (vector-get pat 1) next-local)
        (+ next-local 1))
      (if (or (= tag 12) (= tag (ast-pat-recordpat)))
        (bind-match-pattern-children pat 0 (vector-get pat 1) env next-local)
        (if (or (= tag 11) (= tag (ast-pat-constructor)))
          (bind-match-pattern-children pat 0 (vector-get pat 2) env next-local)
          (make-match-bind-state env next-local))))))

(defn compile-match-literal-check [lit value-local instrs]
  (let [lit-tag (vector-get lit 0)]
    (if (= lit-tag (ast-lit-int))
      (let [i1 (emit-to instrs (op-local-get) value-local)
        i2 (emit-to i1 (op-i64-const) (vector-get lit 1))]
        (emit-to i2 (op-i64-eq) 0))
      (if (= lit-tag (ast-lit-bool))
        (let [i1 (emit-to instrs (op-local-get) value-local)
          i2 (emit-to i1 (op-i64-const) (vector-get lit 1))]
          (emit-to i2 (op-i64-eq) 0))
        (if (= lit-tag (ast-lit-unit))
          (let [i1 (emit-to instrs (op-local-get) value-local)
            i2 (emit-to i1 (op-i64-const) 0)]
            (emit-to i2 (op-i64-eq) 0))
          (emit-to instrs (op-i64-const) 0))))))

(defn compile-match-record-field-check [pat value-local scratch-base idx instrs]
  (let [field-hash (vector-get pat (+ 2 (* idx 2)))
    child (vector-get pat (+ 3 (* idx 2)))
    map-op (+ scratch-base 1)
    i1 (emit-to instrs (op-local-get) value-local)
    i2 (emit-to i1 (op-i64-const) field-hash)
    contains (emit-to i2 (op-map-contains) map-op)
    child-tag (vector-get child 0)]
    (if (or (= child-tag (ast-pat-wildcard)) (or (= child-tag (ast-pat-var)) (= child-tag 4)))
      contains
      (let [i3 (emit-to contains (op-local-get) value-local)
        i4 (emit-to i3 (op-i64-const) field-hash)
        i5 (emit-to i4 (op-map-get) map-op)
        i6 (emit-to i5 (op-local-set) scratch-base)
        i7 (compile-match-pattern-check-with-scratch child scratch-base scratch-base i6)]
        (emit-to i7 (op-i64-and) 0)))))

(defn compile-match-record-fields [pat idx count value-local scratch-base instrs]
  (if (>= idx count)
    (if (= count 0)
      (emit-to instrs (op-i64-const) 1)
      instrs)
    (let [field-check
      (compile-match-record-field-check pat value-local scratch-base idx instrs)]
      (if (= idx 0)
        (compile-match-record-fields pat (+ idx 1) count value-local scratch-base field-check)
        (let [combined (emit-to field-check (op-i64-and) 0)]
          (compile-match-record-fields pat (+ idx 1) count value-local scratch-base combined))))))

(defn compile-match-pattern-check-with-scratch [pat value-local scratch-base instrs]
  (let [pat-tag (vector-get pat 0)]
    (if (= pat-tag (ast-pat-lit))
      (compile-match-literal-check (vector-get pat 1) value-local instrs)
      (if (or (= pat-tag (ast-pat-wildcard)) (= pat-tag (ast-pat-var)))
        (emit-to instrs (op-i64-const) 1)
        (if (or (= pat-tag 12) (= pat-tag (ast-pat-recordpat)))
          (compile-match-record-fields
            pat
            0
            (vector-get pat 1)
            value-local
            scratch-base
            instrs)
          (emit-to instrs (op-i64-const) 0))))))

(defn compile-match-pattern-check [pat scr-idx instrs]
  (compile-match-pattern-check-with-scratch pat scr-idx (+ scr-idx 1) instrs))

(defn compile-match-pattern-binders-children [pat idx count value-local env scratch-base instrs]
  (if (>= idx count)
    instrs
    (let [field-hash (vector-get pat (+ 2 (* idx 2)))
      child (vector-get pat (+ 3 (* idx 2)))
      map-op (+ scratch-base 1)
      i1 (emit-to instrs (op-local-get) value-local)
      i2 (emit-to i1 (op-i64-const) field-hash)
      i3 (emit-to i2 (op-map-get) map-op)
      i4 (emit-to i3 (op-local-set) scratch-base)
      i5 (compile-match-pattern-binders child scratch-base env scratch-base i4)]
      (compile-match-pattern-binders-children
        pat (+ idx 1) count value-local env scratch-base i5))))

(defn compile-match-pattern-binders [pat value-local env scratch-base instrs]
  (let [pat-tag (vector-get pat 0)]
    (if (or (= pat-tag (ast-pat-var)) (= pat-tag 4))
      (let [local-idx (env-lookup env (vector-get pat 1))]
        (if (> local-idx 0)
          (let [i1 (emit-to instrs (op-local-get) value-local)]
            (emit-to i1 (op-local-set) local-idx))
          instrs))
      (if (or (= pat-tag 12) (= pat-tag (ast-pat-recordpat)))
        (compile-match-pattern-binders-children
          pat
          0
          (vector-get pat 1)
          value-local
          env
          scratch-base
          instrs)
        instrs))))

(defn compile-match-arm-prefix [node scr-idx pattern-slot instrs]
  (let [i1 (emit-to instrs 10 scr-idx)
    i2 (emit-to i1 1 (vector-get node pattern-slot))
    i3 (emit-to i2 30 0)]
    (emit-to i3 41 0)))

(defn compile-match-default-tail [instrs]
  (let [i1 (emit-to instrs 1 0)]
    (emit-to i1 43 0)))

(defn compile-match-default-double-tail [instrs]
  (emit-to (compile-match-default-tail instrs) 43 0))
