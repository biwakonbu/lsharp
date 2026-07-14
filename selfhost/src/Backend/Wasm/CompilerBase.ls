(module Backend.Wasm.CompilerBase)
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
(defn op-i64-mod [] 28)
(defn op-i64-eq [] 30)
(defn op-i64-ne [] 31)
(defn op-i64-lt [] 32)
(defn op-i64-gt [] 33)
(defn op-i64-le [] 34)
(defn op-i64-ge [] 35)
(defn op-i64-and [] 71)
(defn op-call [] 40)
(defn op-block [] 42)
(defn op-loop [] 82)
(defn op-br [] 80)
(defn op-br-if [] 81)
(defn op-if-empty [] 83)
(defn op-if [] 41)
(defn op-else [] 79)
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
(defn op-command-line-args [] 86)
(defn op-print-string [] 87)
(defn op-proc-exit [] 88)
(defn op-write-file [] 89)
(defn op-write-file-bytes [] 90)
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
(defn builtin-write-file [] 3246539326542506)
(defn builtin-write-file-bytes [] 7965480599336288136)
(defn builtin-map-contains [] (- 0 3820778934353407281))
(defn builtin-map-remove [] 2967773956947477)
(defn builtin-command-line-arg [] 4333701572691766591)
(defn builtin-file-exists [] 2680668565995926546)
(defn builtin-command-line-args [] 5217540237477903124)
(defn builtin-print-string [] 2942060250258025265)
(defn builtin-proc-exit [] 98761626082613)
(defn builtin-root-push [] 100385403511895)
(defn builtin-root-pop [] 3238238822772)
(defn builtin-root-set [] 3238238825349)
(defn builtin-int-to-string [] (- 0 6637826915257342139))
(defn builtin-basic-opcode [name-hash]
  (if (= name-hash 43)
    20
    (if (= name-hash 45)
      21
      (if (= name-hash 42)
        22
        (if (= name-hash 47)
          23
          (if (= name-hash 37)
            28
            (if (= name-hash 61)
              30
              (if (= name-hash 62)
                33
                (if (= name-hash 60)
                  32
                  (if (= name-hash 1983)
                    35
                    (if (= name-hash 1921)
                      34
                      (if (= name-hash 1952)
                        30
                        0))))))))))))

(defn builtin-string-opcode [name-hash]
  (if (= name-hash 6233512424790686798)
    50
    (if (= name-hash 1391193567100747810)
      51
      (if (= name-hash 106934957)
        59
        (if (= name-hash 1391193566852316240)
          70
          (if (= name-hash 101391823498833)
            69
            0))))))

(defn builtin-vector-ref-opcode [name-hash]
  (if (= name-hash 3361052332089172656)
    52
    (if (= name-hash 3208847393524684)
      53
      (if (= name-hash 3208847393531414)
        54
        (if (= name-hash 99474269199548772)
          55
          (if (= name-hash 104162612582)
            56
            (if (= name-hash 104162605852)
              57
              (if (= name-hash 104162617384)
                58
                0))))))))

(defn builtin-map-core-opcode [name-hash]
  (if (= name-hash 99619812783)
    60
    (if (= name-hash 3088214349266)
      61
      (if (= name-hash 99619806053)
        63
        (if (= name-hash 2967773707765834)
          62
          0)))))

(defn builtin-io-opcode [name-hash]
  (if (= name-hash 100097347767123)
    64
    (if (= name-hash 3246539326542506)
      89
      (if (= name-hash 7965480599336288136)
        90
        (if (= name-hash 4333701572691766591)
          67
          (if (= name-hash 2680668565995926546)
            73
            (if (= name-hash 5217540237477903124)
              86
              (if (= name-hash 2942060250258025265)
                87
                (if (= name-hash 98761626082613)
                  88
                  0)))))))))

(defn builtin-map-extra-opcode [name-hash]
  (if (= name-hash (- 0 3820778934353407281))
    65
    (if (= name-hash 2967773956947477)
      66
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
  (if (= name-hash 100385403511895)
    74
    (if (= name-hash 3238238822772)
      75
      (if (= name-hash 3238238825349)
        76
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

(defn push-int-vector [dst value]
  (do
    (root_push dst)
    (let [next-dst (vector-push dst value)]
      (do
        (root_pop)
        next-dst))))
(defn push-int-vector-local [dst value]
  (do
    (root_push dst)
    (let [next-dst (vector-push dst value)]
      (do
        (root_pop)
        next-dst))))
(defn emit-instr [opcode operand] (push-int-vector (push-int-vector (vector-new 2) opcode) operand))
(defn push-object-vector [dst value]
  (do
    (root_push dst)
    (root_push value)
    (let [next-dst (vector-push dst value)]
      (do
        (root_pop)
        (root_pop)
        next-dst))))
(defn emit-to [instrs opcode operand]
  (do
    (root_push instrs)
    (let [instr (emit-instr opcode operand)]
      (do
        (root_push instr)
        (let [result (push-object-vector instrs instr)]
          (do
            (root_pop)
            (root_pop)
            result))))))
(defn make-max-local-slot-state [done next-idx next-value]
  (push-int-vector
    (push-int-vector
      (push-int-vector (vector-new 3) done)
      next-idx)
    next-value))
(defn max-local-slot-op [opcode operand current-max] (if (if (= opcode 10) true (if (= opcode 11) true (if (= opcode 50) true (= opcode 53)))) (if (> operand current-max) operand current-max) (if (= opcode 54) (if (> (+ operand 1) current-max) (+ operand 1) current-max) (if (= opcode 55) (if (> (+ operand 5) current-max) (+ operand 5) current-max) (if (= opcode 56) (if (> (+ operand 1) current-max) (+ operand 1) current-max) (if (= opcode 58) (if (> operand current-max) operand current-max) (if (= opcode 60) (if (> operand current-max) operand current-max) (if (= opcode 62) (if (> (+ operand 5) current-max) (+ operand 5) current-max) (if (= opcode 63) (if (> (+ operand 5) current-max) (+ operand 5) current-max) (if (= opcode 65) (if (> (+ operand 5) current-max) (+ operand 5) current-max) (if (= opcode 66) (if (> (+ operand 5) current-max) (+ operand 5) current-max) current-max)))))))))))
(defn max-local-slot-step [instrs idx count current-max]
  (if (>= idx count)
    (make-max-local-slot-state 1 idx current-max)
    (let [instr (vector-get instrs idx)
      opcode (vector-get instr 0)
      operand (vector-get instr 1)
      next-max (max-local-slot-op opcode operand current-max)]
      (make-max-local-slot-state 0 (+ idx 1) next-max))))

(defn continue-max-local-slot-step [instrs count state]
  (if (= (vector-get state 0) 1)
    state
    (max-local-slot-step instrs (vector-get state 1) count (vector-get state 2))))

(defn max-local-slot-step-8 [instrs idx count current-max]
  (let [step1 (max-local-slot-step instrs idx count current-max)]
    (do
      (root_push step1)
      (let [step2 (continue-max-local-slot-step instrs count step1)]
        (do
          (root_push step2)
          (let [step3 (continue-max-local-slot-step instrs count step2)]
            (do
              (root_push step3)
              (let [step4 (continue-max-local-slot-step instrs count step3)]
                (do
                  (root_push step4)
                  (let [step5 (continue-max-local-slot-step instrs count step4)]
                    (do
                      (root_push step5)
                      (let [step6 (continue-max-local-slot-step instrs count step5)]
                        (do
                          (root_push step6)
                          (let [step7 (continue-max-local-slot-step instrs count step6)]
                            (do
                              (root_push step7)
                              (let [step8 (continue-max-local-slot-step instrs count step7)]
                                (do
                                  (root_pop)
                                  (root_pop)
                                  (root_pop)
                                  (root_pop)
                                  (root_pop)
                                  (root_pop)
                                  (root_pop)
                                  step8)))))))))))))))))

(defn continue-max-local-slot-step-8 [instrs count state]
  (if (= (vector-get state 0) 1)
    state
    (max-local-slot-step-8 instrs (vector-get state 1) count (vector-get state 2))))

(defn max-local-slot-step-64 [instrs idx count current-max]
  (let [step1 (max-local-slot-step-8 instrs idx count current-max)]
    (do
      (root_push step1)
      (let [step2 (continue-max-local-slot-step-8 instrs count step1)]
        (do
          (root_push step2)
          (let [step3 (continue-max-local-slot-step-8 instrs count step2)]
            (do
              (root_push step3)
              (let [step4 (continue-max-local-slot-step-8 instrs count step3)]
                (do
                  (root_push step4)
                  (let [step5 (continue-max-local-slot-step-8 instrs count step4)]
                    (do
                      (root_push step5)
                      (let [step6 (continue-max-local-slot-step-8 instrs count step5)]
                        (do
                          (root_push step6)
                          (let [step7 (continue-max-local-slot-step-8 instrs count step6)]
                            (do
                              (root_push step7)
                              (let [step8 (continue-max-local-slot-step-8 instrs count step7)]
                                (do
                                  (root_pop)
                                  (root_pop)
                                  (root_pop)
                                  (root_pop)
                                  (root_pop)
                                  (root_pop)
                                  (root_pop)
                                  step8)))))))))))))))))

(defn max-local-slot [instrs idx count current-max]
  (let [step (max-local-slot-step-64 instrs idx count current-max)]
    (if (= (vector-get step 0) 1)
      (vector-get step 2)
      (max-local-slot instrs (vector-get step 1) count (vector-get step 2)))))
(defn env-new [] (map-new))
(defn env-bind [env name-hash idx]
  (let [env-slot (root_push env)]
    (do
      (let [updated (map-insert env name-hash idx)]
        (do
          (root_set env-slot updated)
          (root_pop)
          updated)))))
(defn env-lookup [env name-hash]
  (do
    (root_push env)
    (let [value (map-get env name-hash)]
      (do
        (root_pop)
        value))))
(defn ftable-new [] (vector-new 8))
(defn ftable-register [ftable name-hash func-idx]
  (do
    (root_push ftable)
    (let [name-hash-ref (ref-new name-hash)
      func-idx-ref (ref-new func-idx)]
      (do
        (root_push name-hash-ref)
        (root_push func-idx-ref)
        (let [with-name (vector-push ftable (ref-get name-hash-ref))]
          (do
            (root_push with-name)
            (let [result (vector-push with-name (ref-get func-idx-ref))]
              (do
                (root_pop)
                (root_pop)
                (root_pop)
                (root_pop)
                result))))))))
(defn ftable-with-native-runtime-imports []
  (ftable-register (ftable-new) (builtin-int-to-string) 6))
(defn ftable-lookup-loop [ftable idx name-hash]
  (if (< idx 0)
    0
    (if (= (vector-get ftable idx) name-hash)
      (vector-get ftable (+ idx 1))
      (ftable-lookup-loop ftable (- idx 2) name-hash))))
(defn ftable-lookup [ftable name-hash]
  (do
    (root_push ftable)
    (let [value (ftable-lookup-loop ftable (- (vector-length ftable) 2) name-hash)]
      (do
        (root_pop)
        value))))
(defn ftable-size [ftable] (/ (vector-length ftable) 2))
(defn ftable-register-map-legacy [ftable name-hash func-idx]
  (let [ftable-slot (root_push ftable)]
    (do
      (let [updated (map-insert ftable name-hash func-idx)]
        (do
          (root_set ftable-slot updated)
          (root_pop)
          updated)))))
(defn ftable-lookup-map-legacy [ftable name-hash]
  (do
    (root_push ftable)
    (let [value (map-get ftable name-hash)]
      (do
        (root_pop)
        value))))
(defn make-loop-step-state [done next-idx next-value]
  (push-int-vector
    (push-int-vector
      (push-int-vector (vector-new 3) done)
      next-idx)
    next-value))

(defn make-bind-node-params-state [done next-param-idx next-env next-local-idx]
  (do
    (root_push next-env)
    (let [state
        (push-int-vector
          (push-object-vector
            (push-int-vector
              (push-int-vector (vector-new 4) done)
              next-param-idx)
            next-env)
          next-local-idx)]
      (do
        (root_pop)
        state))))

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
  (if (>= idx param-count)
    env
    (do
      (root_push node)
      (let [param-hash (vector-get node (+ param-base idx))
        next-env (env-bind env param-hash next-idx)]
        (do
          (root_push next-env)
          (let [result (bind-node-params node param-base (+ idx 1) param-count next-env (+ next-idx 1))]
            (do
              (root_pop)
              (root_pop)
              result)))))))
(defn write-compile-step-state-ref [state-ref done next-idx next-value]
  (do
    (root_push state-ref)
    (root_push next-value)
    (let [base (vector-new 3)]
      (do
        (let [base-slot (root_push base)]
          (do
            (let [with-done (push-int-vector-local base done)]
              (do
                (root_set base-slot with-done)
                (let [with-idx (push-int-vector-local with-done next-idx)]
                  (do
                    (root_set base-slot with-idx)
                    (let [state (push-object-vector with-idx next-value)]
                      (do
                        (root_set base-slot state)
                        (ref-set state-ref state)
                        (root_pop)
                        (root_pop)
                        (root_pop)
                        0))))))))))))
(defn write-compile-step-state-ref-normal-setup-diagnostic [state-ref done next-idx next-value]
  (do
    (root_push state-ref)
    (root_push next-value)
    (print 9000000217)
    (print done)
    (print next-idx)
    (print (vector-length next-value))
    (let [base (vector-new 3)]
      (do
        (let [base-slot (root_push base)]
          (do
            (let [with-done (push-int-vector-local base done)]
              (do
                (root_set base-slot with-done)
                (print 9000000218)
                (print (vector-length with-done))
                (print (vector-get with-done 0))
                (let [with-idx (push-int-vector-local with-done next-idx)]
                  (do
                    (root_set base-slot with-idx)
                    (print 9000000219)
                    (print (vector-length with-idx))
                    (print (vector-get with-idx 0))
                    (print (vector-get with-idx 1))
                    (let [state (push-object-vector with-idx next-value)]
                      (do
                        (root_set base-slot state)
                        (print 9000000220)
                        (print (vector-length state))
                        (print (vector-get state 0))
                        (print (vector-get state 1))
                        (print (vector-length (vector-get state 2)))
                        (ref-set state-ref state)
                        (print 9000000221)
                        (print (vector-length (ref-get state-ref)))
                        (print (vector-get (ref-get state-ref) 0))
                        (print (vector-get (ref-get state-ref) 1))
                        (print (vector-length (vector-get (ref-get state-ref) 2)))
                        (root_pop)
                        (root_pop)
                        (root_pop)
                        0))))))))))))
(defn make-compile-step-state [done next-idx next-value]
  (let [value-slot (root_push next-value)
    compile-step-state-progress-mode (if (> (string-length (command-line-arg 8)) 0) 1 0)]
    (do
      (if (= compile-step-state-progress-mode 1)
        (if (< (vector-length next-value) 128)
          (do
            (print 9000000076)
            (print 0)
            (print done)
            (print next-idx)
            (print (vector-length next-value)))
          (do))
        (do))
      (let [base0 (push-int-vector-local (vector-new 3) done)]
        (do
          (root_push base0)
          (let [base1 (push-int-vector-local base0 next-idx)]
            (do
              (root_push base1)
              (let [state (push-object-vector base1 next-value)]
                (do
                  (root_set value-slot state)
                  (if (= compile-step-state-progress-mode 1)
                    (if (< (vector-length next-value) 128)
                      (let [state-root (root_push state)]
                        (do
                          (print 9000000076)
                          (print 1)
                          (print (vector-get state 0))
                          (print (vector-get state 1))
                          (print (vector-length (vector-get state 2)))
                          (root_pop)))
                      (do))
                    (do))
                  (root_pop)
                  (root_pop)
                  (root_pop)
                  state)))))))))
(defn string-literal-data-base [] 1024)
(defn append-byte-vector-step [dst src idx count]
  (if (>= idx count)
    (make-compile-step-state 1 idx dst)
    (make-compile-step-state 0 (+ idx 1) (push-int-vector dst (vector-get src idx)))))

(defn continue-append-byte-vector-step [src count state]
  (if (= (vector-get state 0) 1)
    state
    (append-byte-vector-step (vector-get state 2) src (vector-get state 1) count)))

(defn continue-append-byte-vector-step-times [src count remaining state]
  (if (= remaining 0)
    state
    (if (= (vector-get state 0) 1)
      state
      (do
        (root_push src)
        (root_push state)
        (let [next-state (continue-append-byte-vector-step src count state)]
          (do
            (root_push next-state)
            (let [result (continue-append-byte-vector-step-times src count (- remaining 1) next-state)]
              (do
                (root_pop)
                (root_pop)
                (root_pop)
                result))))))))

(defn append-byte-vector-step-8 [dst src idx count]
  (do
    (root_push src)
    (let [state (append-byte-vector-step dst src idx count)]
      (do
        (root_push state)
        (let [result (continue-append-byte-vector-step-times src count 7 state)]
          (do
            (root_pop)
            (root_pop)
            result))))))

(defn continue-append-byte-vector-step-8 [src count state]
  (if (= (vector-get state 0) 1)
    state
    (append-byte-vector-step-8 (vector-get state 2) src (vector-get state 1) count)))

(defn append-byte-vector-step-64 [dst src idx count]
  (do
    (root_push src)
    (let [state (append-byte-vector-step dst src idx count)]
      (do
        (root_push state)
        (let [result (continue-append-byte-vector-step-times src count 63 state)]
          (do
            (root_pop)
            (root_pop)
            result))))))

(defn append-byte-vector [dst src idx count]
  (let [state (append-byte-vector-step-8 dst src idx count)]
    (if (= (vector-get state 0) 1)
      (vector-get state 2)
      (do
        (root_push src)
        (root_push state)
        (let [result (append-byte-vector (vector-get state 2) src (vector-get state 1) count)]
          (do
            (root_pop)
            (root_pop)
            result))))))

(defn string-to-byte-vector-step [text idx count bytes]
  (if (>= idx count)
    (make-compile-step-state 1 idx bytes)
    (make-compile-step-state 0 (+ idx 1) (push-int-vector bytes (string-char-at text idx)))))

(defn continue-string-to-byte-vector-step [text count state]
  (if (= (vector-get state 0) 1)
    state
    (string-to-byte-vector-step text (vector-get state 1) count (vector-get state 2))))

(defn continue-string-to-byte-vector-step-times [text count remaining state]
  (if (= remaining 0)
    state
    (if (= (vector-get state 0) 1)
      state
      (do
        (root_push text)
        (root_push state)
        (let [next-state (continue-string-to-byte-vector-step text count state)]
          (do
            (root_push next-state)
            (let [result (continue-string-to-byte-vector-step-times text count (- remaining 1) next-state)]
              (do
                (root_pop)
                (root_pop)
                (root_pop)
                result))))))))

(defn string-to-byte-vector-step-8 [text idx count bytes]
  (do
    (root_push text)
    (let [state (string-to-byte-vector-step text idx count bytes)]
      (do
        (root_push state)
        (let [result (continue-string-to-byte-vector-step-times text count 7 state)]
          (do
            (root_pop)
            (root_pop)
            result))))))

(defn continue-string-to-byte-vector-step-8 [text count state]
  (if (= (vector-get state 0) 1)
    state
    (string-to-byte-vector-step-8 text (vector-get state 1) count (vector-get state 2))))

(defn string-to-byte-vector-step-64 [text idx count bytes]
  (do
    (root_push text)
    (let [state (string-to-byte-vector-step text idx count bytes)]
      (do
        (root_push state)
        (let [result (continue-string-to-byte-vector-step-times text count 63 state)]
          (do
            (root_pop)
            (root_pop)
            result))))))

(defn string-to-byte-vector [text idx count bytes]
  (let [state (string-to-byte-vector-step-8 text idx count bytes)]
    (if (= (vector-get state 0) 1)
      (vector-get state 2)
      (do
        (root_push text)
        (root_push state)
        (let [result (string-to-byte-vector text (vector-get state 1) count (vector-get state 2))]
          (do
            (root_pop)
            (root_pop)
            result))))))
(defn write-i32-le [vec value] (push-int-vector (push-int-vector (push-int-vector (push-int-vector vec (% value 256)) (% (/ value 256) 256)) (% (/ value 65536) 256)) (% (/ value 16777216) 256)))

(defn string-literal-unescape-consumed [src idx len]
  (if (if (< (+ idx 1) len) (= (string-char-at src idx) 92) false)
    2
    1))

(defn string-literal-unescape-piece [src idx len]
  (if (if (< (+ idx 1) len) (= (string-char-at src idx) 92) false)
    (let [escaped (string-char-at src (+ idx 1))]
      (if (= escaped 110)
        "\n"
        (if (= escaped 116)
          "\t"
          (if (= escaped 114)
            "\r"
            (if (= escaped 34)
              "\""
              (if (= escaped 92)
                "\\"
                (substring src (+ idx 1) (+ idx 2))))))))
    (substring src idx (+ idx 1))))

(defn string-literal-unescape-loop [src idx len out]
  (if (>= idx len)
    out
    (do
      (root_push src)
      (root_push out)
      (let [piece (string-literal-unescape-piece src idx len)
        next-idx (+ idx (string-literal-unescape-consumed src idx len))]
        (do
          (root_push piece)
          (let [next-out (string-concat out piece)]
            (do
              (root_push next-out)
              (let [result (string-literal-unescape-loop src next-idx len next-out)]
                (do
                  (root_pop)
                  (root_pop)
                  (root_pop)
                  (root_pop)
                  result)))))))))

(defn string-literal-unescape [src]
  (do
    (root_push src)
    (let [result (string-literal-unescape-loop src 0 (string-length src) "")]
      (do
        (root_pop)
        result))))

(defn compile-string-literal-with-source [node source instrs data-ref]
  (do
    (root_push source)
    (root_push instrs)
    (root_push data-ref)
    (let [start (vector-get node 1)
      end (vector-get node 2)]
      (let [raw-text (substring source start end)]
        (do
          (root_push raw-text)
          (let [text (string-literal-unescape raw-text)]
            (do
              (root_push text)
              (let [text-len (string-length text)
                bytes (string-to-byte-vector text 0 text-len (vector-new 8))
                offset (+ (string-literal-data-base) (vector-length (ref-get data-ref)))]
                (do
                  (root_push bytes)
                  (let [header (write-i32-le (write-i32-le (vector-new 8) 1) text-len)]
                    (do
                      (root_push header)
                      (let [data-with-header (append-byte-vector (ref-get data-ref) header 0 8)]
                        (do
                          (root_push data-with-header)
                          (let [updated-data (append-byte-vector data-with-header bytes 0 (vector-length bytes))]
                            (do
                              (root_push updated-data)
                              (ref-set data-ref updated-data)
                              (let [result (emit-to instrs 1 offset)]
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
                                  result)))))))))))))))))
(defn string-key-hash-step [source pos end acc]
  (if (>= pos end)
    (make-loop-step-state 1 pos acc)
    (make-loop-step-state 0 (+ pos 1) (+ (string-char-at source pos) (* acc 31)))))

(defn continue-string-key-hash-step [source end state]
  (if (= (vector-get state 0) 1)
    state
    (string-key-hash-step source (vector-get state 1) end (vector-get state 2))))

(defn continue-string-key-hash-step-times [source end remaining state]
  (if (= remaining 0)
    state
    (if (= (vector-get state 0) 1)
      state
      (do
        (root_push source)
        (root_push state)
        (let [next-state (continue-string-key-hash-step source end state)]
          (do
            (root_push next-state)
            (let [result (continue-string-key-hash-step-times source end (- remaining 1) next-state)]
              (do
                (root_pop)
                (root_pop)
                (root_pop)
                result))))))))

(defn string-key-hash-step-8 [source pos end acc]
  (do
    (root_push source)
    (let [state (string-key-hash-step source pos end acc)]
      (do
        (root_push state)
        (let [result (continue-string-key-hash-step-times source end 7 state)]
          (do
            (root_pop)
            (root_pop)
            result))))))

(defn continue-string-key-hash-step-8 [source end state]
  (if (= (vector-get state 0) 1)
    state
    (string-key-hash-step-8 source (vector-get state 1) end (vector-get state 2))))

(defn string-key-hash-step-64 [source pos end acc]
  (do
    (root_push source)
    (let [state (string-key-hash-step source pos end acc)]
      (do
        (root_push state)
        (let [result (continue-string-key-hash-step-times source end 63 state)]
          (do
            (root_pop)
            (root_pop)
            result))))))

(defn string-key-hash-loop [source pos end acc]
  (let [step (string-key-hash-step-64 source pos end acc)]
    (if (= (vector-get step 0) 1)
      (vector-get step 2)
      (string-key-hash-loop source (vector-get step 1) end (vector-get step 2)))))
(defn normalize-map-key-hash [hash] (if (= hash 0) 2 (if (= hash -1) 1 hash)))
(defn compile-string-key-hash-with-source [node source instrs]
  (let [start (vector-get node 1)
    end (vector-get node 2)
    raw-text (substring source start end)]
    (do
      (root_push raw-text)
      (let [text (string-literal-unescape raw-text)]
        (do
          (root_push text)
          (let [hash (normalize-map-key-hash (string-key-hash-loop text 0 (string-length text) 0))
            result (emit-to instrs 1 hash)]
            (do
              (root_pop)
              (root_pop)
              result)))))))
(defn immediate-builtin-op [bop]
  (if (= bop 20)
    true
    (if (= bop 21)
      true
      (if (= bop 22)
        true
        (if (= bop 23)
          true
          (if (= bop 28)
            true
            (if (= bop 30)
              true
              (if (= bop 33)
                true
                (if (= bop 32)
                  true
                  (if (= bop 35)
                    true
                    (if (= bop 34)
                      true
                      (if (= bop 50)
                        true
                        (if (= bop 51)
                          true
                          (if (= bop 52)
                            true
                            (if (= bop 61)
                              true
                              (if (= bop 65)
                                true
                                (if (= bop 73)
                                  true
                                  (if (= bop 59)
                                    true
                                    (if (= bop 74)
                                      true
                                      (if (= bop 75)
                                        true
                                        (if (= bop 76)
                                          true
                                          (if (= bop 71)
                                            true
                                            (if (= bop 72)
                                              true
                                              (if (= bop 86)
                                                true
                                                (if (= bop 87)
                                                  true
                                                  (= bop 88))))))))))))))))))))))))))
(defn alloc-root-needed [expr]
  (let [tag (vector-get expr 0)]
    (if (= tag 1)
      0
      (if (= tag 2)
        0
        (if (= tag 5)
          (let [func-node (vector-get expr 1)
            func-tag (vector-get func-node 0)
            func-hash (if (= func-tag 4) (vector-get func-node 1) 0)
            bop (builtin-opcode func-hash)]
            (if (immediate-builtin-op bop) 0 1))
          1)))))
(defn simple-map-operand [expr] (let [tag (vector-get expr 0)] (if (= tag 4) true (if (= tag 1) true (if (= tag 2) true (= tag 3))))))
(defn emit-root-push-drop [instrs local-idx]
  (let [instrs1 (emit-to instrs 10 local-idx)]
    (do
      (root_push instrs1)
      (let [instrs2 (emit-to instrs1 74 0)]
        (do
          (root_push instrs2)
          (let [result (emit-to instrs2 44 0)]
            (do
              (root_pop)
              (root_pop)
              result)))))))
(defn emit-root-pop-drop [instrs]
  (let [instrs1 (emit-to instrs 75 0)]
    (do
      (root_push instrs1)
      (let [result (emit-to instrs1 44 0)]
        (do
          (root_pop)
          result)))))
(defn emit-root-pop-drops-step [remaining instrs]
  (if (<= remaining 0)
    (make-compile-step-state 1 remaining instrs)
    (make-compile-step-state 0 (- remaining 1) (emit-root-pop-drop instrs))))
(defn continue-emit-root-pop-drops-step [state]
  (if (= (vector-get state 0) 1)
    state
    (emit-root-pop-drops-step (vector-get state 1) (vector-get state 2))))
(defn emit-root-pop-drops-step-8 [remaining instrs]
  (let [step1 (emit-root-pop-drops-step remaining instrs)
    step2 (continue-emit-root-pop-drops-step step1)
    step3 (continue-emit-root-pop-drops-step step2)
    step4 (continue-emit-root-pop-drops-step step3)
    step5 (continue-emit-root-pop-drops-step step4)
    step6 (continue-emit-root-pop-drops-step step5)
    step7 (continue-emit-root-pop-drops-step step6)
    step8 (continue-emit-root-pop-drops-step step7)]
    step8))
(defn continue-emit-root-pop-drops-step-8 [state]
  (if (= (vector-get state 0) 1)
    state
    (emit-root-pop-drops-step-8 (vector-get state 1) (vector-get state 2))))
(defn emit-root-pop-drops-step-64 [remaining instrs]
  (let [step1 (emit-root-pop-drops-step-8 remaining instrs)
    step2 (continue-emit-root-pop-drops-step-8 step1)
    step3 (continue-emit-root-pop-drops-step-8 step2)
    step4 (continue-emit-root-pop-drops-step-8 step3)
    step5 (continue-emit-root-pop-drops-step-8 step4)
    step6 (continue-emit-root-pop-drops-step-8 step5)
    step7 (continue-emit-root-pop-drops-step-8 step6)
    step8 (continue-emit-root-pop-drops-step-8 step7)]
    step8))
(defn emit-root-pop-drops [instrs remaining]
  (let [step (emit-root-pop-drops-step-64 remaining instrs)]
    (if (= (vector-get step 0) 1)
      (vector-get step 2)
      (emit-root-pop-drops (vector-get step 2) (vector-get step 1)))))
(defn maybe-root-push-drop [instrs should-root local-idx] (if (= should-root 0) instrs (emit-root-push-drop instrs local-idx)))
(defn maybe-root-pop-drop [instrs should-root] (if (= should-root 0) instrs (emit-root-pop-drop instrs)))
(defn make-let-chain-next-value [node env instrs]
  (do
    (root_push node)
    (root_push env)
    (root_push instrs)
    (let [base0 (push-object-vector (vector-new 3) node)]
      (do
        (root_push base0)
        (let [base1 (push-object-vector base0 env)]
          (do
            (root_push base1)
            (let [result (push-object-vector base1 instrs)]
              (do
                (root_pop)
                (root_pop)
                (root_pop)
                (root_pop)
                (root_pop)
                result))))))))
(defn compile-let-chain-step-finish [name-hash body-expr init-instrs env rooted-count init-root]
  (do
    (root_push env)
    (root_push init-instrs)
    (let [new-idx (+ 1 (map-size env))
      next-instrs1 (emit-to init-instrs 11 new-idx)]
      (do
        (root_push next-instrs1)
        (let [next-instrs2 (maybe-root-push-drop next-instrs1 init-root new-idx)]
          (do
            (root_push next-instrs2)
            (let [next-env (env-bind env name-hash new-idx)]
              (do
                (root_push next-env)
                (let [next-value (make-let-chain-next-value body-expr next-env next-instrs2)
                  result (make-compile-step-state
                    (if (= (vector-get body-expr 0) 7) 0 1)
                    (+ rooted-count init-root)
                    next-value)]
                  (do
                    (root_pop)
                    (root_pop)
                    (root_pop)
                    (root_pop)
                    (root_pop)
                    result))))))))))
(defn compile-defn-functions-step-finish [functions compiled-fn idx]
  (let [updated-functions (push-object-vector functions compiled-fn)]
    (do
      (let [updated-slot (root_push updated-functions)]
      (let [next-state (make-compile-step-state 0 (+ idx 1) updated-functions)]
        (do
          (root_push next-state)
          (root_set updated-slot next-state)
          (root_pop)
          (root_pop)
          next-state))))))
(defn compile-let-with-ftable-prepare [name-hash init-root init-instrs env]
  (do
    (root_push env)
    (root_push init-instrs)
    (let [new-idx (+ 1 (map-size env))
      instrs1 (emit-to init-instrs 11 new-idx)]
      (do
        (root_push instrs1)
        (let [instrs2 (maybe-root-push-drop instrs1 init-root new-idx)]
          (do
            (root_push instrs2)
            (let [new-env (env-bind env name-hash new-idx)]
              (do
                (root_push new-env)
                (let [prep0 (push-object-vector (vector-new 2) new-env)]
                  (do
                    (root_push prep0)
                    (let [result (push-object-vector prep0 instrs2)]
                      (do
                        (root_pop)
                        (root_pop)
                        (root_pop)
                        (root_pop)
                        (root_pop)
                        (root_pop)
                        result))))))))))))
(defn function-meta-param-count [func-meta] (vector-get func-meta 0))
(defn function-meta-local-count [func-meta] (vector-get func-meta 1))
(defn function-meta-ir [func-meta] (vector-get func-meta 2))
(defn make-function-meta [param-count local-count ir]
  (do
    (root_push ir)
    (let [meta1 (push-int-vector (vector-new 3) param-count)]
      (do
        (root_push meta1)
        (let [meta2 (push-int-vector meta1 local-count)]
          (do
            (root_push meta2)
            (let [result (push-object-vector meta2 ir)]
              (do
                (root_pop)
                (root_pop)
                (root_pop)
                result))))))))
(defn write-register-state-ref [state-ref done next-idx next-ftable next-func-idx]
  (do
    (root_push state-ref)
    (let [base (vector-new 4)]
      (do
        (let [base-slot (root_push base)]
          (do
            (let [done-ref (ref-new done)
              next-idx-ref (ref-new next-idx)
              next-func-idx-ref (ref-new next-func-idx)]
              (do
                (root_push done-ref)
                (root_push next-idx-ref)
                (root_push next-func-idx-ref)
                (root_push next-ftable)
                (let [with-done (vector-push base (ref-get done-ref))]
                  (do
                    (root_set base-slot with-done)
                    (let [with-idx (vector-push with-done (ref-get next-idx-ref))]
                      (do
                        (root_set base-slot with-idx)
                        (let [with-ftable (vector-push with-idx next-ftable)]
                          (do
                            (root_set base-slot with-ftable)
                            (let [state (vector-push with-ftable (ref-get next-func-idx-ref))]
                              (do
                                (root_set base-slot state)
                                (ref-set state-ref state)
                                (root_pop)
                                (root_pop)
                                (root_pop)
                                (root_pop)
                                (root_pop)
                                (root_pop)
                                0))))))))))))))))
(defn make-register-state [done next-idx next-ftable next-func-idx]
  (let [state-ref (ref-new 0)]
    (do
      (root_push state-ref)
      (write-register-state-ref state-ref done next-idx next-ftable next-func-idx)
      (let [state (ref-get state-ref)]
        (do
          (root_pop)
          state)))))
