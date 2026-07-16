(module Backend.Wasm.WasmEmit)
(import IR.IR)
(import Backend.Wasm.WasiBackend)
(defn wasm-magic-0 [] 0)
(defn wasm-magic-1 [] 97)
(defn wasm-magic-2 [] 115)
(defn wasm-magic-3 [] 109)
(defn wasm-version-0 [] 1)
(defn wasm-version-1 [] 0)
(defn wasm-version-2 [] 0)
(defn wasm-version-3 [] 0)
(defn function-meta-param-count [func-meta] (vector-get func-meta 0))
(defn function-meta-local-count [func-meta] (vector-get func-meta 1))
(defn function-meta-ir [func-meta] (vector-get func-meta 2))
(defn section-type [] 1)
(defn section-import [] 2)
(defn section-function [] 3)
(defn section-memory [] 5)
(defn section-export [] 7)
(defn section-code [] 10)
(defn wasm-i32 [] 127)
(defn wasm-i64 [] 126)
(defn wasm-funcref [] 112)
(defn wasm-end [] 11)
(defn wasm-i64-const [] 66)
(defn wasm-local-get [] 32)
(defn wasm-local-set [] 33)
(defn wasm-i64-add [] 124)
(defn wasm-i64-sub [] 125)
(defn wasm-i64-mul [] 126)
(defn wasm-call [] 16)
(defn wasm-return [] 15)
(defn wasm-i64-eq [] 81)
(defn wasm-i64-div-s [] 127)
(defn wasm-if [] 4)
(defn wasm-else [] 5)
(defn wasm-drop [] 26)
(defn push-int-vector [dst value]
  (do
    (root_push dst)
    (let [next-dst (vector-push dst value)]
      (do
        (root_pop)
        next-dst))))
(defn push-object-vector [dst value]
  (do
    (root_push dst)
    (root_push value)
    (let [next-dst (vector-push dst value)]
      (do
        (root_pop)
        (root_pop)
        next-dst))))
(defn leb128-u-loop [v acc] (let [low7 (% v 128) rest (/ v 128)] (if (= rest 0) (push-int-vector acc low7) (leb128-u-loop rest (push-int-vector acc (+ low7 128))))))
(defn leb128-u [value] (leb128-u-loop value (vector-new 4)))
(defn leb128-s-pos [v acc]
  (let [low7 (% v 128)
    rest (/ v 128)]
    (if (= rest 0)
      (if (< low7 64)
        (push-int-vector acc low7)
        (do
          (root_push acc)
          (let [with-low7 (push-int-vector acc (+ low7 128))]
            (do
              (root_push with-low7)
              (let [result (push-int-vector with-low7 0)]
                (do
                  (root_pop)
                  (root_pop)
                  result))))))
      (do
        (root_push acc)
        (let [next-acc (push-int-vector acc (+ low7 128))]
          (do
            (root_push next-acc)
            (let [result (leb128-s-pos rest next-acc)]
              (do
                (root_pop)
                (root_pop)
                result))))))))
(defn leb128-s [value] (if (< value 0) (let [result (ref-new (vector-new 4)) v (ref-new value) done (ref-new 0)] (do (let [byte1 (% (+ (% (ref-get v) 128) 128) 128) rest1 (if (< (ref-get v) -64) 1 0)] (if (= rest1 0) (do (ref-set result (push-int-vector (ref-get result) byte1)) 0) (do (ref-set result (push-int-vector (ref-get result) (+ byte1 128))) (let [shifted (/ (- (ref-get v) byte1) 128) byte2 (% (+ (% shifted 128) 128) 128) rest2 (if (< shifted -64) 1 0)] (if (= rest2 0) (do (ref-set result (push-int-vector (ref-get result) byte2)) 0) (do (ref-set result (push-int-vector (ref-get result) (+ byte2 128))) (let [shifted2 (/ (- shifted byte2) 128) byte3 (% (+ (% shifted2 128) 128) 128)] (do (ref-set result (push-int-vector (ref-get result) byte3)) 0)))))))) (ref-get result))) (leb128-s-pos value (vector-new 4))))
(defn make-loop-step-state [done next-idx next-value]
  (push-object-vector
    (push-int-vector
      (push-int-vector (vector-new 3) done)
      next-idx)
    next-value))

(defn make-emit-if-state [done next-idx next-body next-if-depth next-if-flags]
  (push-int-vector
    (push-int-vector
      (push-object-vector
        (push-int-vector
          (push-int-vector (vector-new 5) done)
          next-idx)
        next-body)
      next-if-depth)
    next-if-flags))

(defn emit-leb128 [bytes value]
  (let [leb (leb128-u value)]
    (do
      (root_push leb)
      (let [result (append-byte-vector bytes leb 0 (vector-length leb))]
        (do
          (root_pop)
          result)))))
(defn emit-leb128-s [bytes value]
  (let [leb (leb128-s value)]
    (do
      (root_push leb)
      (let [result (append-byte-vector bytes leb 0 (vector-length leb))]
        (do
          (root_pop)
          result)))))
(defn emit-byte [bytes b] (push-int-vector bytes b))
(defn emit-standalone-byte-seq-1 [dst a]
  (let [result (emit-byte dst a)]
    (do
      (vector-length result)
      result)))
(defn emit-standalone-byte-seq-2 [dst a b]
  (let [b1 (emit-byte dst a)
    result (emit-byte b1 b)]
    (do
      (vector-length result)
      result)))
(defn emit-standalone-byte-seq-4 [dst a b c d]
  (let [b1 (emit-byte dst a)
    b2 (emit-byte b1 b)
    b3 (emit-byte b2 c)
    result (emit-byte b3 d)]
    (do
      (vector-length result)
      result)))
(defn emit-standalone-byte-seq-6 [dst a b c d e f]
  (let [b1 (emit-byte dst a)
    b2 (emit-byte b1 b)
    b3 (emit-byte b2 c)
    b4 (emit-byte b3 d)
    b5 (emit-byte b4 e)
    result (emit-byte b5 f)]
    (do
      (vector-length result)
      result)))
(defn emit-standalone-byte-seq-8 [dst a b c d e f g h]
  (let [b1 (emit-byte dst a)
    b2 (emit-byte b1 b)
    b3 (emit-byte b2 c)
    b4 (emit-byte b3 d)
    b5 (emit-byte b4 e)
    b6 (emit-byte b5 f)
    b7 (emit-byte b6 g)
    result (emit-byte b7 h)]
    (do
      (vector-length result)
      result)))
(defn emit-header [] (let [h (vector-new 8)] (vector-push (vector-push (vector-push (vector-push (vector-push (vector-push (vector-push (vector-push h 0) 97) 115) 109) 1) 0) 0) 0)))
(defn emit-type-section-main [] (let [bytes (vector-new 16)] (let [b1 (emit-byte bytes 1) b2 (emit-byte b1 5) b3 (emit-byte b2 1) b4 (emit-byte b3 96) b5 (emit-byte b4 0) b6 (emit-byte b5 1) b7 (emit-byte b6 126)] b7)))
(defn append-i64-param-types-step [dst idx param-count]
  (if (>= idx param-count)
    (make-loop-step-state 1 idx dst)
    (make-loop-step-state 0 (+ idx 1) (emit-byte dst 126))))

(defn continue-append-i64-param-types-step [param-count state]
  (if (= (vector-get state 0) 1)
    state
    (append-i64-param-types-step (vector-get state 2) (vector-get state 1) param-count)))

(defn append-i64-param-types-step-8 [dst idx param-count]
  (let [step1 (append-i64-param-types-step dst idx param-count)
    step2 (continue-append-i64-param-types-step param-count step1)
    step3 (continue-append-i64-param-types-step param-count step2)
    step4 (continue-append-i64-param-types-step param-count step3)
    step5 (continue-append-i64-param-types-step param-count step4)
    step6 (continue-append-i64-param-types-step param-count step5)
    step7 (continue-append-i64-param-types-step param-count step6)
    step8 (continue-append-i64-param-types-step param-count step7)]
    step8))

(defn continue-append-i64-param-types-step-8 [param-count state]
  (if (= (vector-get state 0) 1)
    state
    (append-i64-param-types-step-8 (vector-get state 2) (vector-get state 1) param-count)))

(defn append-i64-param-types-step-64 [dst idx param-count]
  (let [step1 (append-i64-param-types-step-8 dst idx param-count)
    step2 (continue-append-i64-param-types-step-8 param-count step1)
    step3 (continue-append-i64-param-types-step-8 param-count step2)
    step4 (continue-append-i64-param-types-step-8 param-count step3)
    step5 (continue-append-i64-param-types-step-8 param-count step4)
    step6 (continue-append-i64-param-types-step-8 param-count step5)
    step7 (continue-append-i64-param-types-step-8 param-count step6)
    step8 (continue-append-i64-param-types-step-8 param-count step7)]
    step8))

(defn append-i64-param-types [dst idx param-count]
  (let [step (append-i64-param-types-step-64 dst idx param-count)]
    (if (= (vector-get step 0) 1)
      (vector-get step 2)
      (append-i64-param-types (vector-get step 2) (vector-get step 1) param-count))))

(defn append-function-types-step [dst functions idx func-count]
  (if (>= idx func-count)
    (make-loop-step-state 1 idx dst)
    (let [func-meta (vector-get functions idx)
      param-count (function-meta-param-count func-meta)
      body0 (emit-byte dst 96)
      body1 (emit-leb128 body0 param-count)
      body2 (append-i64-param-types body1 0 param-count)
      body3 (emit-byte body2 1)
      body4 (emit-byte body3 126)]
      (make-loop-step-state 0 (+ idx 1) body4))))

(defn continue-append-function-types-step [functions func-count state]
  (if (= (vector-get state 0) 1)
    state
    (append-function-types-step (vector-get state 2) functions (vector-get state 1) func-count)))

(defn append-function-types-step-8 [dst functions idx func-count]
  (let [step1 (append-function-types-step dst functions idx func-count)
    step2 (continue-append-function-types-step functions func-count step1)
    step3 (continue-append-function-types-step functions func-count step2)
    step4 (continue-append-function-types-step functions func-count step3)
    step5 (continue-append-function-types-step functions func-count step4)
    step6 (continue-append-function-types-step functions func-count step5)
    step7 (continue-append-function-types-step functions func-count step6)
    step8 (continue-append-function-types-step functions func-count step7)]
    step8))

(defn continue-append-function-types-step-8 [functions func-count state]
  (if (= (vector-get state 0) 1)
    state
    (append-function-types-step-8 (vector-get state 2) functions (vector-get state 1) func-count)))

(defn append-function-types-step-64 [dst functions idx func-count]
  (let [step1 (append-function-types-step-8 dst functions idx func-count)
    step2 (continue-append-function-types-step-8 functions func-count step1)
    step3 (continue-append-function-types-step-8 functions func-count step2)
    step4 (continue-append-function-types-step-8 functions func-count step3)
    step5 (continue-append-function-types-step-8 functions func-count step4)
    step6 (continue-append-function-types-step-8 functions func-count step5)
    step7 (continue-append-function-types-step-8 functions func-count step6)
    step8 (continue-append-function-types-step-8 functions func-count step7)]
    step8))

(defn continue-append-function-types-step-64 [functions func-count state]
  (if (= (vector-get state 0) 1)
    state
    (append-function-types-step-64 (vector-get state 2) functions (vector-get state 1) func-count)))

(defn append-function-types-step-512 [dst functions idx func-count]
  (let [step1 (append-function-types-step-64 dst functions idx func-count)
    step2 (continue-append-function-types-step-64 functions func-count step1)
    step3 (continue-append-function-types-step-64 functions func-count step2)
    step4 (continue-append-function-types-step-64 functions func-count step3)
    step5 (continue-append-function-types-step-64 functions func-count step4)
    step6 (continue-append-function-types-step-64 functions func-count step5)
    step7 (continue-append-function-types-step-64 functions func-count step6)
    step8 (continue-append-function-types-step-64 functions func-count step7)]
    step8))

(defn append-function-types [dst functions idx func-count]
  (do
    (root_push dst)
    (root_push functions)
    (let [step (append-function-types-step-512 dst functions idx func-count)]
      (do
        (root_push step)
        (let [result
            (if (= (vector-get step 0) 1)
              (vector-get step 2)
              (append-function-types (vector-get step 2) functions (vector-get step 1) func-count))]
          (do
            (root_pop)
            (root_pop)
            (root_pop)
            result))))))
(defn emit-type-section-functions [functions] (let [func-count (vector-length functions) body0 (emit-leb128 (vector-new 32) func-count) body1 (append-function-types body0 functions 0 func-count) body-size (vector-length body1) result0 (emit-byte (vector-new 32) 1) result1 (emit-leb128 result0 body-size)] (append-byte-vector result1 body1 0 body-size)))
(defn emit-type-section-functions-wasi [functions] (let [func-count (vector-length functions) body0 (emit-leb128 (vector-new 32) (+ func-count 1)) body1 (append-function-types body0 functions 0 func-count) body2 (emit-byte body1 96) body3 (emit-leb128 body2 0) body4 (emit-leb128 body3 0) body-size (vector-length body4) result0 (emit-byte (vector-new 32) 1) result1 (emit-leb128 result0 body-size)] (append-byte-vector result1 body4 0 body-size)))
(defn emit-type-section-alloc-main [] (let [body0 (emit-leb128 (vector-new 24) 2) body1 (emit-byte body0 96) body2 (emit-leb128 body1 1) body3 (emit-byte body2 126) body4 (emit-byte body3 1) body5 (emit-byte body4 126) body6 (emit-byte body5 96) body7 (emit-leb128 body6 0) body8 (emit-byte body7 1) body9 (emit-byte body8 126) body-size (vector-length body9) result0 (emit-byte (vector-new 24) 1) result1 (emit-leb128 result0 body-size)] (append-byte-vector result1 body9 0 body-size)))
(defn emit-type-section-alloc-print-main [] (let [body0 (emit-leb128 (vector-new 32) 3) body1 (emit-byte body0 96) body2 (emit-leb128 body1 1) body3 (emit-byte body2 126) body4 (emit-byte body3 1) body5 (emit-byte body4 126) body6 (emit-byte body5 96) body7 (emit-leb128 body6 1) body8 (emit-byte body7 126) body9 (emit-leb128 body8 0) body10 (emit-byte body9 96) body11 (emit-leb128 body10 0) body12 (emit-byte body11 1) body13 (emit-byte body12 126) body-size (vector-length body13) result0 (emit-byte (vector-new 32) 1) result1 (emit-leb128 result0 body-size)] (append-byte-vector result1 body13 0 body-size)))
(defn helper-id-alloc [] 0)
(defn helper-id-print [] 1)
(defn helper-id-read-file [] 2)
(defn helper-id-runtime-hash [] 3)
(defn helper-id-command-line-arg [] 4)
(defn emit-type-section-helper-pair-main [helper-a helper-b] (if (= helper-a (helper-id-alloc)) (if (= helper-b (helper-id-print)) (emit-type-section-alloc-print-main) (emit-type-section-main)) (emit-type-section-main)))
(defn emit-type-section-helper-triple-main [helper-a helper-b helper-c] (if (= helper-a (helper-id-alloc)) (if (= helper-b (helper-id-print)) (if (= helper-c (helper-id-read-file)) (emit-type-section-alloc-print-main) (emit-type-section-main)) (emit-type-section-main)) (emit-type-section-main)))
(defn emit-type-section-helper-quad-main [helper-a helper-b helper-c helper-d] (if (= helper-a (helper-id-alloc)) (if (= helper-b (helper-id-print)) (if (= helper-c (helper-id-read-file)) (if (= helper-d (helper-id-command-line-arg)) (emit-type-section-alloc-print-main) (if (= helper-d (helper-id-runtime-hash)) (emit-type-section-alloc-print-main) (emit-type-section-main))) (emit-type-section-main)) (emit-type-section-main)) (emit-type-section-main)))
(defn append-type-index-zeros-step [dst idx func-count]
  (if (>= idx func-count)
    (make-loop-step-state 1 idx dst)
    (make-loop-step-state 0 (+ idx 1) (emit-byte dst 0))))

(defn continue-append-type-index-zeros-step [func-count state]
  (if (= (vector-get state 0) 1)
    state
    (append-type-index-zeros-step (vector-get state 2) (vector-get state 1) func-count)))

(defn append-type-index-zeros-step-8 [dst idx func-count]
  (let [step1 (append-type-index-zeros-step dst idx func-count)
    step2 (continue-append-type-index-zeros-step func-count step1)
    step3 (continue-append-type-index-zeros-step func-count step2)
    step4 (continue-append-type-index-zeros-step func-count step3)
    step5 (continue-append-type-index-zeros-step func-count step4)
    step6 (continue-append-type-index-zeros-step func-count step5)
    step7 (continue-append-type-index-zeros-step func-count step6)
    step8 (continue-append-type-index-zeros-step func-count step7)]
    step8))

(defn continue-append-type-index-zeros-step-8 [func-count state]
  (if (= (vector-get state 0) 1)
    state
    (append-type-index-zeros-step-8 (vector-get state 2) (vector-get state 1) func-count)))

(defn append-type-index-zeros-step-64 [dst idx func-count]
  (let [step1 (append-type-index-zeros-step-8 dst idx func-count)
    step2 (continue-append-type-index-zeros-step-8 func-count step1)
    step3 (continue-append-type-index-zeros-step-8 func-count step2)
    step4 (continue-append-type-index-zeros-step-8 func-count step3)
    step5 (continue-append-type-index-zeros-step-8 func-count step4)
    step6 (continue-append-type-index-zeros-step-8 func-count step5)
    step7 (continue-append-type-index-zeros-step-8 func-count step6)
    step8 (continue-append-type-index-zeros-step-8 func-count step7)]
    step8))

(defn append-type-index-zeros [dst idx func-count]
  (let [step (append-type-index-zeros-step-64 dst idx func-count)]
    (if (= (vector-get step 0) 1)
      (vector-get step 2)
      (append-type-index-zeros (vector-get step 2) (vector-get step 1) func-count))))

(defn append-type-index-sequence-step [dst idx func-count]
  (if (>= idx func-count)
    (make-loop-step-state 1 idx dst)
    (make-loop-step-state 0 (+ idx 1) (emit-leb128 dst idx))))

(defn continue-append-type-index-sequence-step [func-count state]
  (if (= (vector-get state 0) 1)
    state
    (append-type-index-sequence-step (vector-get state 2) (vector-get state 1) func-count)))

(defn append-type-index-sequence-step-8 [dst idx func-count]
  (let [step1 (append-type-index-sequence-step dst idx func-count)
    step2 (continue-append-type-index-sequence-step func-count step1)
    step3 (continue-append-type-index-sequence-step func-count step2)
    step4 (continue-append-type-index-sequence-step func-count step3)
    step5 (continue-append-type-index-sequence-step func-count step4)
    step6 (continue-append-type-index-sequence-step func-count step5)
    step7 (continue-append-type-index-sequence-step func-count step6)
    step8 (continue-append-type-index-sequence-step func-count step7)]
    step8))

(defn continue-append-type-index-sequence-step-8 [func-count state]
  (if (= (vector-get state 0) 1)
    state
    (append-type-index-sequence-step-8 (vector-get state 2) (vector-get state 1) func-count)))

(defn append-type-index-sequence-step-64 [dst idx func-count]
  (let [step1 (append-type-index-sequence-step-8 dst idx func-count)
    step2 (continue-append-type-index-sequence-step-8 func-count step1)
    step3 (continue-append-type-index-sequence-step-8 func-count step2)
    step4 (continue-append-type-index-sequence-step-8 func-count step3)
    step5 (continue-append-type-index-sequence-step-8 func-count step4)
    step6 (continue-append-type-index-sequence-step-8 func-count step5)
    step7 (continue-append-type-index-sequence-step-8 func-count step6)
    step8 (continue-append-type-index-sequence-step-8 func-count step7)]
    step8))

(defn append-type-index-sequence [dst idx func-count]
  (let [step (append-type-index-sequence-step-64 dst idx func-count)]
    (if (= (vector-get step 0) 1)
      (vector-get step 2)
      (append-type-index-sequence (vector-get step 2) (vector-get step 1) func-count))))
(defn emit-function-section-count [func-count] (let [body0 (emit-leb128 (vector-new 16) func-count) body1 (append-type-index-zeros body0 0 func-count) body-size (vector-length body1) result0 (emit-byte (vector-new 16) 3) result1 (emit-leb128 result0 body-size)] (append-byte-vector result1 body1 0 body-size)))
(defn emit-function-section [] (emit-function-section-count 1))
(defn emit-function-section-main-type-index [type-idx] (let [body0 (emit-leb128 (vector-new 16) 1) body1 (emit-leb128 body0 type-idx) body-size (vector-length body1) result0 (emit-byte (vector-new 16) 3) result1 (emit-leb128 result0 body-size)] (append-byte-vector result1 body1 0 body-size)))
(defn emit-function-section-functions [functions] (let [func-count (vector-length functions) body0 (emit-leb128 (vector-new 32) func-count) body1 (append-type-index-sequence body0 0 func-count) body-size (vector-length body1) result0 (emit-byte (vector-new 32) 3) result1 (emit-leb128 result0 body-size)] (append-byte-vector result1 body1 0 body-size)))
(defn emit-function-section-functions-wasi [functions] (let [func-count (vector-length functions) body0 (emit-leb128 (vector-new 32) (+ func-count 1)) body1 (append-type-index-sequence body0 0 func-count) body2 (emit-leb128 body1 func-count) body-size (vector-length body2) result0 (emit-byte (vector-new 32) 3) result1 (emit-leb128 result0 body-size)] (append-byte-vector result1 body2 0 body-size)))
(defn emit-export-section-main-index [func-idx] (let [body0 (emit-leb128 (vector-new 16) 1) body1 (emit-byte body0 6) body2 (emit-byte body1 95) body3 (emit-byte body2 115) body4 (emit-byte body3 116) body5 (emit-byte body4 97) body6 (emit-byte body5 114) body7 (emit-byte body6 116) body8 (emit-byte body7 0) body9 (emit-leb128 body8 func-idx) body-size (vector-length body9) result0 (emit-byte (vector-new 16) 7) result1 (emit-leb128 result0 body-size)] (append-byte-vector result1 body9 0 body-size)))
(defn emit-export-section [] (emit-export-section-main-index 0))
(defn emit-export-section-main-memory-index [func-idx mem-idx] (let [body0 (emit-leb128 (vector-new 24) 2) body1 (emit-byte body0 6) body2 (emit-byte body1 95) body3 (emit-byte body2 115) body4 (emit-byte body3 116) body5 (emit-byte body4 97) body6 (emit-byte body5 114) body7 (emit-byte body6 116) body8 (emit-byte body7 0) body9 (emit-leb128 body8 func-idx) body10 (emit-byte body9 6) body11 (emit-byte body10 109) body12 (emit-byte body11 101) body13 (emit-byte body12 109) body14 (emit-byte body13 111) body15 (emit-byte body14 114) body16 (emit-byte body15 121) body17 (emit-byte body16 2) body18 (emit-leb128 body17 mem-idx) body-size (vector-length body18) result0 (emit-byte (vector-new 24) 7) result1 (emit-leb128 result0 body-size)] (append-byte-vector result1 body18 0 body-size)))
(defn emit-memory-section [] (let [bytes (vector-new 8)] (let [b1 (emit-byte bytes 5) b2 (emit-byte b1 4) b3 (emit-byte b2 1) b4 (emit-byte b3 0) b5 (emit-byte b4 128) b6 (emit-byte b5 2)] b6)))
(defn emit-import-section [] (let [bytes (vector-new 64)] (let [b1 (emit-byte bytes 2) b2 (emit-byte b1 36) b3 (emit-byte b2 1) b4 (emit-byte b3 21) b5 (emit-byte b4 119) b6 (emit-byte b5 97) b7 (emit-byte b6 115) b8 (emit-byte b7 105) b9 (emit-byte b8 95) b10 (emit-byte b9 115) b11 (emit-byte b10 110) b12 (emit-byte b11 97) b13 (emit-byte b12 112) b14 (emit-byte b13 115) b15 (emit-byte b14 104) b16 (emit-byte b15 111) b17 (emit-byte b16 116) b18 (emit-byte b17 95) b19 (emit-byte b18 112) b20 (emit-byte b19 114) b21 (emit-byte b20 101) b22 (emit-byte b21 118) b23 (emit-byte b22 105) b24 (emit-byte b23 101) b25 (emit-byte b24 119) b26 (emit-byte b25 49) b27 (emit-byte b26 8) b28 (emit-byte b27 102) b29 (emit-byte b28 100) b30 (emit-byte b29 95) b31 (emit-byte b30 119) b32 (emit-byte b31 114) b33 (emit-byte b32 105) b34 (emit-byte b33 116) b35 (emit-byte b34 101) b36 (emit-byte b35 0) b37 (emit-byte b36 0)] b37)))
(defn emit-import-section-wasi-standalone [func-count]
  (let [body0 (emit-byte (vector-new 192) 6)
    body1 (emit-standalone-byte-seq-8 body0 22 119 97 115 105 95 115 110)
    body2 (emit-standalone-byte-seq-8 body1 97 112 115 104 111 116 95 112)
    body3 (emit-standalone-byte-seq-8 body2 114 101 118 105 101 119 49 8)
    body4 (emit-standalone-byte-seq-8 body3 102 100 95 119 114 105 116 101)
    body5 (emit-standalone-byte-seq-2 body4 0 0)
    body6 (emit-standalone-byte-seq-8 body5 22 119 97 115 105 95 115 110)
    body7 (emit-standalone-byte-seq-8 body6 97 112 115 104 111 116 95 112)
    body8 (emit-standalone-byte-seq-8 body7 114 101 118 105 101 119 49 14)
    body9 (emit-standalone-byte-seq-8 body8 97 114 103 115 95 115 105 122)
    body10 (emit-standalone-byte-seq-8 body9 101 115 95 103 101 116 0 6)
    body11 (emit-standalone-byte-seq-8 body10 22 119 97 115 105 95 115 110)
    body12 (emit-standalone-byte-seq-8 body11 97 112 115 104 111 116 95 112)
    body13 (emit-standalone-byte-seq-8 body12 114 101 118 105 101 119 49 8)
    body14 (emit-standalone-byte-seq-8 body13 97 114 103 115 95 103 101 116)
    body15 (emit-standalone-byte-seq-2 body14 0 6)
    body16 (emit-standalone-byte-seq-8 body15 22 119 97 115 105 95 115 110)
    body17 (emit-standalone-byte-seq-8 body16 97 112 115 104 111 116 95 112)
    body18 (emit-standalone-byte-seq-8 body17 114 101 118 105 101 119 49 9)
    body19 (emit-standalone-byte-seq-8 body18 112 97 116 104 95 111 112 101)
    body20a (emit-standalone-byte-seq-2 body19 110 0)
    body20 (emit-leb128 body20a (+ func-count 8))
    body21 (emit-standalone-byte-seq-8 body20 22 119 97 115 105 95 115 110)
    body22 (emit-standalone-byte-seq-8 body21 97 112 115 104 111 116 95 112)
    body23 (emit-standalone-byte-seq-8 body22 114 101 118 105 101 119 49 8)
    body24 (emit-standalone-byte-seq-8 body23 102 100 95 99 108 111 115 101)
    body25a (emit-standalone-byte-seq-1 body24 0)
    body25 (emit-leb128 body25a (+ func-count 9))
    body26 (emit-standalone-byte-seq-8 body25 22 119 97 115 105 95 115 110)
    body27 (emit-standalone-byte-seq-8 body26 97 112 115 104 111 116 95 112)
    body28 (emit-standalone-byte-seq-8 body27 114 101 118 105 101 119 49 7)
    body29a (emit-standalone-byte-seq-6 body28 102 100 95 114 101 97)
    body29 (emit-standalone-byte-seq-1 body29a 100)
    body30a (emit-standalone-byte-seq-1 body29 0)
    body30 (emit-leb128 body30a (+ func-count 10))
    body-size (vector-length body30)
    result0 (emit-byte (vector-new 64) 2)
    result1 (emit-leb128 result0 body-size)]
    (append-byte-vector-chunked result1 body30 0 body-size)))
(defn append-component-module-name [body] (let [b1 (emit-leb128 body (wasi-module-name-length-for-target (wasi-target-component))) b2 (emit-byte b1 119) b3 (emit-byte b2 97) b4 (emit-byte b3 115) b5 (emit-byte b4 105)] b5))
(defn emit-import-section-component [] (let [body0 (emit-leb128 (vector-new 32) 1) body1 (append-component-module-name body0) body2 (emit-leb128 body1 8) body3 (emit-byte body2 102) body4 (emit-byte body3 100) body5 (emit-byte body4 95) body6 (emit-byte body5 119) body7 (emit-byte body6 114) body8 (emit-byte body7 105) body9 (emit-byte body8 116) body10 (emit-byte body9 101) body11 (emit-byte body10 0) body12 (emit-leb128 body11 0) body-size (vector-length body12) result0 (emit-byte (vector-new 32) 2) result1 (emit-leb128 result0 body-size)] (append-byte-vector result1 body12 0 body-size)))
(defn emit-import-section-for-target [target] (if (= target (wasi-target-component)) (emit-import-section-component) (emit-import-section)))
(defn emit-import-section-alloc [] (let [body0 (emit-leb128 (vector-new 24) 1) body1 (emit-leb128 body0 3) body2 (emit-byte body1 101) body3 (emit-byte body2 110) body4 (emit-byte body3 118) body5 (emit-leb128 body4 7) body6 (emit-byte body5 95) body7 (emit-byte body6 95) body8 (emit-byte body7 97) body9 (emit-byte body8 108) body10 (emit-byte body9 108) body11 (emit-byte body10 111) body12 (emit-byte body11 99) body13 (emit-byte body12 0) body14 (emit-leb128 body13 0) body-size (vector-length body14) result0 (emit-byte (vector-new 24) 2) result1 (emit-leb128 result0 body-size)] (append-byte-vector result1 body14 0 body-size)))
(defn emit-import-section-alloc-print [] (let [body0 (emit-leb128 (vector-new 32) 2) body1 (emit-leb128 body0 3) body2 (emit-byte body1 101) body3 (emit-byte body2 110) body4 (emit-byte body3 118) body5 (emit-leb128 body4 7) body6 (emit-byte body5 95) body7 (emit-byte body6 95) body8 (emit-byte body7 97) body9 (emit-byte body8 108) body10 (emit-byte body9 108) body11 (emit-byte body10 111) body12 (emit-byte body11 99) body13 (emit-byte body12 0) body14 (emit-leb128 body13 0) body15 (emit-leb128 body14 3) body16 (emit-byte body15 101) body17 (emit-byte body16 110) body18 (emit-byte body17 118) body19 (emit-leb128 body18 5) body20 (emit-byte body19 112) body21 (emit-byte body20 114) body22 (emit-byte body21 105) body23 (emit-byte body22 110) body24 (emit-byte body23 116) body25 (emit-byte body24 0) body26 (emit-leb128 body25 1) body-size (vector-length body26) result0 (emit-byte (vector-new 32) 2) result1 (emit-leb128 result0 body-size)] (append-byte-vector result1 body26 0 body-size)))
(defn emit-import-section-helper-pair [helper-a helper-b] (if (= helper-a (helper-id-alloc)) (if (= helper-b (helper-id-print)) (emit-import-section-alloc-print) (emit-import-section-alloc)) (emit-import-section-alloc)))
(defn emit-import-section-helper-triple [helper-a helper-b helper-c] (if (= helper-a (helper-id-alloc)) (if (= helper-b (helper-id-print)) (if (= helper-c (helper-id-read-file)) (emit-import-section-alloc-print-read) (emit-import-section-alloc-print)) (emit-import-section-alloc)) (emit-import-section-alloc)))
(defn emit-import-section-helper-quad [helper-a helper-b helper-c helper-d] (if (= helper-a (helper-id-alloc)) (if (= helper-b (helper-id-print)) (if (= helper-c (helper-id-read-file)) (if (= helper-d (helper-id-command-line-arg)) (emit-import-section-alloc-print-read-arg) (if (= helper-d (helper-id-runtime-hash)) (emit-import-section-alloc-print-read-hash) (emit-import-section-alloc-print-read))) (emit-import-section-alloc-print)) (emit-import-section-alloc)) (emit-import-section-alloc)))
(defn emit-import-section-alloc-print-read [] (let [body0 (emit-leb128 (vector-new 48) 3) body1 (emit-leb128 body0 3) body2 (emit-byte body1 101) body3 (emit-byte body2 110) body4 (emit-byte body3 118) body5 (emit-leb128 body4 7) body6 (emit-byte body5 95) body7 (emit-byte body6 95) body8 (emit-byte body7 97) body9 (emit-byte body8 108) body10 (emit-byte body9 108) body11 (emit-byte body10 111) body12 (emit-byte body11 99) body13 (emit-byte body12 0) body14 (emit-leb128 body13 0) body15 (emit-leb128 body14 3) body16 (emit-byte body15 101) body17 (emit-byte body16 110) body18 (emit-byte body17 118) body19 (emit-leb128 body18 5) body20 (emit-byte body19 112) body21 (emit-byte body20 114) body22 (emit-byte body21 105) body23 (emit-byte body22 110) body24 (emit-byte body23 116) body25 (emit-byte body24 0) body26 (emit-leb128 body25 1) body27 (emit-leb128 body26 3) body28 (emit-byte body27 101) body29 (emit-byte body28 110) body30 (emit-byte body29 118) body31 (emit-leb128 body30 9) body32 (emit-byte body31 114) body33 (emit-byte body32 101) body34 (emit-byte body33 97) body35 (emit-byte body34 100) body36 (emit-byte body35 45) body37 (emit-byte body36 102) body38 (emit-byte body37 105) body39 (emit-byte body38 108) body40 (emit-byte body39 101) body41 (emit-byte body40 0) body42 (emit-leb128 body41 0) body-size (vector-length body42) result0 (emit-byte (vector-new 48) 2) result1 (emit-leb128 result0 body-size)] (append-byte-vector result1 body42 0 body-size)))
(defn emit-import-section-alloc-print-read-hash [] (let [body0 (emit-leb128 (vector-new 64) 4) body1 (emit-leb128 body0 3) body2 (emit-byte body1 101) body3 (emit-byte body2 110) body4 (emit-byte body3 118) body5 (emit-leb128 body4 7) body6 (emit-byte body5 95) body7 (emit-byte body6 95) body8 (emit-byte body7 97) body9 (emit-byte body8 108) body10 (emit-byte body9 108) body11 (emit-byte body10 111) body12 (emit-byte body11 99) body13 (emit-byte body12 0) body14 (emit-leb128 body13 0) body15 (emit-leb128 body14 3) body16 (emit-byte body15 101) body17 (emit-byte body16 110) body18 (emit-byte body17 118) body19 (emit-leb128 body18 5) body20 (emit-byte body19 112) body21 (emit-byte body20 114) body22 (emit-byte body21 105) body23 (emit-byte body22 110) body24 (emit-byte body23 116) body25 (emit-byte body24 0) body26 (emit-leb128 body25 1) body27 (emit-leb128 body26 3) body28 (emit-byte body27 101) body29 (emit-byte body28 110) body30 (emit-byte body29 118) body31 (emit-leb128 body30 9) body32 (emit-byte body31 114) body33 (emit-byte body32 101) body34 (emit-byte body33 97) body35 (emit-byte body34 100) body36 (emit-byte body35 45) body37 (emit-byte body36 102) body38 (emit-byte body37 105) body39 (emit-byte body38 108) body40 (emit-byte body39 101) body41 (emit-byte body40 0) body42 (emit-leb128 body41 0) body43 (emit-leb128 body42 3) body44 (emit-byte body43 101) body45 (emit-byte body44 110) body46 (emit-byte body45 118) body47 (emit-leb128 body46 12) body48 (emit-byte body47 95) body49 (emit-byte body48 95) body50 (emit-byte body49 102) body51 (emit-byte body50 110) body52 (emit-byte body51 118) body53 (emit-byte body52 49) body54 (emit-byte body53 97) body55 (emit-byte body54 95) body56 (emit-byte body55 104) body57 (emit-byte body56 97) body58 (emit-byte body57 115) body59 (emit-byte body58 104) body60 (emit-byte body59 0) body61 (emit-leb128 body60 0) body-size (vector-length body61) result0 (emit-byte (vector-new 64) 2) result1 (emit-leb128 result0 body-size)] (append-byte-vector result1 body61 0 body-size)))
(defn emit-import-section-alloc-print-read-arg [] (let [body0 (emit-leb128 (vector-new 64) 4) body1 (emit-leb128 body0 3) body2 (emit-byte body1 101) body3 (emit-byte body2 110) body4 (emit-byte body3 118) body5 (emit-leb128 body4 7) body6 (emit-byte body5 95) body7 (emit-byte body6 95) body8 (emit-byte body7 97) body9 (emit-byte body8 108) body10 (emit-byte body9 108) body11 (emit-byte body10 111) body12 (emit-byte body11 99) body13 (emit-byte body12 0) body14 (emit-leb128 body13 0) body15 (emit-leb128 body14 3) body16 (emit-byte body15 101) body17 (emit-byte body16 110) body18 (emit-byte body17 118) body19 (emit-leb128 body18 5) body20 (emit-byte body19 112) body21 (emit-byte body20 114) body22 (emit-byte body21 105) body23 (emit-byte body22 110) body24 (emit-byte body23 116) body25 (emit-byte body24 0) body26 (emit-leb128 body25 1) body27 (emit-leb128 body26 3) body28 (emit-byte body27 101) body29 (emit-byte body28 110) body30 (emit-byte body29 118) body31 (emit-leb128 body30 9) body32 (emit-byte body31 114) body33 (emit-byte body32 101) body34 (emit-byte body33 97) body35 (emit-byte body34 100) body36 (emit-byte body35 45) body37 (emit-byte body36 102) body38 (emit-byte body37 105) body39 (emit-byte body38 108) body40 (emit-byte body39 101) body41 (emit-byte body40 0) body42 (emit-leb128 body41 0) body43 (emit-leb128 body42 3) body44 (emit-byte body43 101) body45 (emit-byte body44 110) body46 (emit-byte body45 118) body47 (emit-leb128 body46 16) body48 (emit-byte body47 99) body49 (emit-byte body48 111) body50 (emit-byte body49 109) body51 (emit-byte body50 109) body52 (emit-byte body51 97) body53 (emit-byte body52 110) body54 (emit-byte body53 100) body55 (emit-byte body54 45) body56 (emit-byte body55 108) body57 (emit-byte body56 105) body58 (emit-byte body57 110) body59 (emit-byte body58 101) body60 (emit-byte body59 45) body61 (emit-byte body60 97) body62 (emit-byte body61 114) body63 (emit-byte body62 103) body64 (emit-byte body63 0) body65 (emit-leb128 body64 0) body-size (vector-length body65) result0 (emit-byte (vector-new 64) 2) result1 (emit-leb128 result0 body-size)] (append-byte-vector result1 body65 0 body-size)))
(defn append-byte-vector-step [dst src idx count]
  (if (>= idx count)
    (make-loop-step-state 1 idx dst)
    (make-loop-step-state 0 (+ idx 1) (emit-byte dst (vector-get src idx)))))

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

(defn continue-append-byte-vector-step-64 [src count state]
  (if (= (vector-get state 0) 1)
    state
    (append-byte-vector-step-64 (vector-get state 2) src (vector-get state 1) count)))

(defn append-byte-vector-step-512 [dst src idx count]
  (let [step1 (append-byte-vector-step-64 dst src idx count)
    step2 (continue-append-byte-vector-step-64 src count step1)
    step3 (continue-append-byte-vector-step-64 src count step2)
    step4 (continue-append-byte-vector-step-64 src count step3)
    step5 (continue-append-byte-vector-step-64 src count step4)
    step6 (continue-append-byte-vector-step-64 src count step5)
    step7 (continue-append-byte-vector-step-64 src count step6)
    step8 (continue-append-byte-vector-step-64 src count step7)]
    step8))

(defn append-byte-vector [dst src idx count]
  (do
    (root_push dst)
    (root_push src)
    (let [step (append-byte-vector-step-512 dst src idx count)]
      (do
        (root_push step)
        (let [result
            (if (= (vector-get step 0) 1)
              (vector-get step 2)
              (append-byte-vector (vector-get step 2) src (vector-get step 1) count))]
          (do
            (root_pop)
            (root_pop)
            (root_pop)
            result))))))

(defn append-byte-vector-chunked [dst src idx count]
  (if (>= idx count)
    dst
    (do
      (root_push dst)
      (root_push src)
      (let [chunk-end (if (> (+ idx 4096) count) count (+ idx 4096))
        next-dst (append-byte-vector dst src idx (if (> (+ idx 4096) count) count (+ idx 4096)))]
        (do
          (root_push next-dst)
          (let [result (append-byte-vector-chunked next-dst src chunk-end count)]
            (do
              (root_pop)
              (root_pop)
              (root_pop)
              result)))))))

(defn append-byte-vector-progress-debug [dst src idx count]
  (do
    (print 525)
    (print idx)
    (if (>= idx count)
      dst
      (do
        (root_push dst)
        (root_push src)
        (let [chunk-end (if (> (+ idx 4096) count) count (+ idx 4096))
          next-dst (append-byte-vector dst src idx (if (> (+ idx 4096) count) count (+ idx 4096)))]
          (do
            (print 526)
            (print chunk-end)
            (root_push next-dst)
            (let [result (append-byte-vector-progress-debug next-dst src chunk-end count)]
              (do
                (root_pop)
                (root_pop)
                (root_pop)
                result))))))))

(defn append-ir-instrs-step-with-if-state [body ir-instrs idx count if-depth if-flags]
  (if (>= idx count)
    (make-emit-if-state 1 idx body if-depth if-flags)
    (let [instr (vector-get ir-instrs idx)
      opcode (vector-get instr 0)
      operand (vector-get instr 1)]
      (if (if (= opcode 41) true (= opcode 83))
        (make-emit-if-state 0 (+ idx 1) (emit-ir-instr body opcode operand) (+ if-depth 1) (* if-flags 2))
        (if (= opcode 79)
          (make-emit-if-state 0 (+ idx 1) (emit-byte body (wasm-else)) if-depth (+ if-flags 1))
          (if (= opcode 43)
            (if (= if-depth 0)
              (make-emit-if-state 0 (+ idx 1) (emit-ir-instr body opcode operand) if-depth if-flags)
              (if (= (% if-flags 2) 0)
                (make-emit-if-state 0 (+ idx 1) (emit-byte body (wasm-else)) if-depth (+ if-flags 1))
                (make-emit-if-state 0 (+ idx 1) (emit-ir-instr body opcode operand) (- if-depth 1) (/ if-flags 2))))
            (make-emit-if-state 0 (+ idx 1) (emit-ir-instr body opcode operand) if-depth if-flags)))))))

(defn continue-append-ir-instrs-step-with-if-state [ir-instrs count state]
  (if (= (vector-get state 0) 1)
    state
    (append-ir-instrs-step-with-if-state
      (vector-get state 2)
      ir-instrs
      (vector-get state 1)
      count
      (vector-get state 3)
      (vector-get state 4))))

(defn append-ir-instrs-step-8-with-if-state [body ir-instrs idx count if-depth if-flags]
  (let [step1 (append-ir-instrs-step-with-if-state body ir-instrs idx count if-depth if-flags)
    step2 (continue-append-ir-instrs-step-with-if-state ir-instrs count step1)
    step3 (continue-append-ir-instrs-step-with-if-state ir-instrs count step2)
    step4 (continue-append-ir-instrs-step-with-if-state ir-instrs count step3)
    step5 (continue-append-ir-instrs-step-with-if-state ir-instrs count step4)
    step6 (continue-append-ir-instrs-step-with-if-state ir-instrs count step5)
    step7 (continue-append-ir-instrs-step-with-if-state ir-instrs count step6)
    step8 (continue-append-ir-instrs-step-with-if-state ir-instrs count step7)]
    step8))

(defn continue-append-ir-instrs-step-8-with-if-state [ir-instrs count state]
  (if (= (vector-get state 0) 1)
    state
    (append-ir-instrs-step-8-with-if-state
      (vector-get state 2)
      ir-instrs
      (vector-get state 1)
      count
      (vector-get state 3)
      (vector-get state 4))))

(defn append-ir-instrs-step-64-with-if-state [body ir-instrs idx count if-depth if-flags]
  (let [step1 (append-ir-instrs-step-8-with-if-state body ir-instrs idx count if-depth if-flags)
    step2 (continue-append-ir-instrs-step-8-with-if-state ir-instrs count step1)
    step3 (continue-append-ir-instrs-step-8-with-if-state ir-instrs count step2)
    step4 (continue-append-ir-instrs-step-8-with-if-state ir-instrs count step3)
    step5 (continue-append-ir-instrs-step-8-with-if-state ir-instrs count step4)
    step6 (continue-append-ir-instrs-step-8-with-if-state ir-instrs count step5)
    step7 (continue-append-ir-instrs-step-8-with-if-state ir-instrs count step6)
    step8 (continue-append-ir-instrs-step-8-with-if-state ir-instrs count step7)]
    step8))

(defn append-ir-instrs-with-if-state [body ir-instrs idx count if-depth if-flags]
  (do
    (root_push body)
    (root_push ir-instrs)
    (let [step (append-ir-instrs-step-64-with-if-state body ir-instrs idx count if-depth if-flags)]
      (do
        (root_push step)
        (let [result
            (if (= (vector-get step 0) 1)
              (vector-get step 2)
              (append-ir-instrs-with-if-state
                (vector-get step 2)
                ir-instrs
                (vector-get step 1)
                count
                (vector-get step 3)
                (vector-get step 4)))]
          (do
            (root_pop)
            (root_pop)
            (root_pop)
            result))))))
(defn append-ir-instrs [body ir-instrs idx count] (append-ir-instrs-with-if-state body ir-instrs idx count 0 0))
(defn append-ir-instrs-with-if-state-progress-debug [body ir-instrs idx count if-depth if-flags]
  (do
    (print 586)
    (print idx)
    (print 587)
    (print if-depth)
    (root_push body)
    (root_push ir-instrs)
    (let [step (append-ir-instrs-step-64-with-if-state body ir-instrs idx count if-depth if-flags)]
      (do
        (root_push step)
        (print 588)
        (print (vector-get step 1))
        (let [result
            (if (= (vector-get step 0) 1)
              (vector-get step 2)
              (append-ir-instrs-with-if-state-progress-debug
                (vector-get step 2)
                ir-instrs
                (vector-get step 1)
                count
                (vector-get step 3)
                (vector-get step 4)))]
          (do
            (root_pop)
            (root_pop)
            (root_pop)
            result))))))
(defn append-ir-instrs-progress-debug [body ir-instrs idx count] (append-ir-instrs-with-if-state-progress-debug body ir-instrs idx count 0 0))
(defn build-function-body [ir-instrs]
  (do
    (root_push ir-instrs)
    (let [body0 (emit-byte (vector-new 64) 0)
      body1 (append-ir-instrs body0 ir-instrs 0 (vector-length ir-instrs))]
      (do
        (root_push body1)
        (let [result (emit-byte body1 11)]
          (do
            (root_pop)
            (root_pop)
            result))))))
(defn build-function-body-function [func-meta]
  (do
    (root_push func-meta)
    (let [local-count (function-meta-local-count func-meta)
      ir-instrs (function-meta-ir func-meta)]
      (do
        (root_push ir-instrs)
        (let [body0 (if (= local-count 0) (emit-byte (vector-new 64) 0) (emit-byte (emit-leb128 (emit-leb128 (vector-new 64) 1) local-count) 126))
          body1 (append-ir-instrs body0 ir-instrs 0 (vector-length ir-instrs))]
          (do
            (root_push body1)
            (let [result (emit-byte body1 11)]
              (do
                (root_pop)
                (root_pop)
                (root_pop)
                result))))))))
(defn standalone-command-line-args-opcode [] 91)
(defn standalone-command-line-arg-opcode [] 92)
(defn standalone-file-exists-opcode [] 93)
(defn standalone-read-file-opcode [] 94)
(defn standalone-write-file-opcode [] 95)
(defn standalone-write-file-bytes-opcode [] 96)
(defn standalone-write-file-bytes-iovec-address [] 2176)
(defn standalone-write-file-bytes-nwritten-address [] 2184)
(defn standalone-ir-instr [instr]
  (let [opcode (vector-get instr 0)
    operand (vector-get instr 1)]
    (if (= opcode 86)
      (make-instr (standalone-command-line-args-opcode) 0)
      (if (= opcode 67)
        (make-instr (standalone-command-line-arg-opcode) 0)
        (if (= opcode 73)
          (make-instr (standalone-file-exists-opcode) 0)
          (if (= opcode 64)
            (make-instr (standalone-read-file-opcode) 0)
            (if (= opcode 89)
              (make-instr (standalone-write-file-opcode) 0)
              (if (= opcode 90)
                (make-instr (standalone-write-file-bytes-opcode) 0)
                (if (and (= opcode 40) (>= operand 12))
                  (make-instr 40 (+ operand 10))
                  instr)))))))))
(defn standalone-ir-instrs-step [ir idx count result]
  (if (>= idx count)
    (make-loop-step-state 1 idx result)
    (do
      (root_push result)
      (let [instr (vector-get ir idx)
        next-instr (standalone-ir-instr instr)]
        (do
          (root_push next-instr)
          (let [next-result (push-object-vector result next-instr)]
            (do
              (root_push next-result)
              (let [state (make-loop-step-state 0 (+ idx 1) next-result)]
                (do
                  (root_pop)
                  (root_pop)
                  (root_pop)
                  state)))))))))
(defn continue-standalone-ir-instrs-step [ir count state]
  (if (= (vector-get state 0) 1)
    state
    (standalone-ir-instrs-step ir (vector-get state 1) count (vector-get state 2))))
(defn standalone-ir-instrs-step-8 [ir idx count result]
  (let [s1 (standalone-ir-instrs-step ir idx count result)
    s2 (continue-standalone-ir-instrs-step ir count s1)
    s3 (continue-standalone-ir-instrs-step ir count s2)
    s4 (continue-standalone-ir-instrs-step ir count s3)
    s5 (continue-standalone-ir-instrs-step ir count s4)
    s6 (continue-standalone-ir-instrs-step ir count s5)
    s7 (continue-standalone-ir-instrs-step ir count s6)
    s8 (continue-standalone-ir-instrs-step ir count s7)]
    s8))
(defn continue-standalone-ir-instrs-step-8 [ir count state]
  (if (= (vector-get state 0) 1)
    state
    (standalone-ir-instrs-step-8 ir (vector-get state 1) count (vector-get state 2))))
(defn standalone-ir-instrs-step-64 [ir idx count result]
  (let [s1 (standalone-ir-instrs-step-8 ir idx count result)
    s2 (continue-standalone-ir-instrs-step-8 ir count s1)
    s3 (continue-standalone-ir-instrs-step-8 ir count s2)
    s4 (continue-standalone-ir-instrs-step-8 ir count s3)
    s5 (continue-standalone-ir-instrs-step-8 ir count s4)
    s6 (continue-standalone-ir-instrs-step-8 ir count s5)
    s7 (continue-standalone-ir-instrs-step-8 ir count s6)
    s8 (continue-standalone-ir-instrs-step-8 ir count s7)]
    s8))
(defn continue-standalone-ir-instrs-step-64 [ir count state]
  (if (= (vector-get state 0) 1)
    state
    (standalone-ir-instrs-step-64 ir (vector-get state 1) count (vector-get state 2))))
(defn standalone-ir-instrs-loop [ir count state]
  (if (= (vector-get state 0) 1)
    (vector-get state 2)
    (standalone-ir-instrs-loop
      ir
      count
      (continue-standalone-ir-instrs-step-64 ir count state))))
(defn standalone-ir-instrs [ir]
  (let [count (vector-length ir)
    state (standalone-ir-instrs-step-64 ir 0 count (vector-new 8))]
    (standalone-ir-instrs-loop ir count state)))
(defn shift-runtime-call-index-value [idx]
  (if (= idx 11)
    1
    (if (= idx 12)
      2
      (if (= idx 13)
        17
        (if (< idx 11)
          (+ idx 6)
          idx)))))
(defn shift-runtime-call-indices-step [bytes result idx count]
  (if (>= idx count)
    (make-loop-step-state 1 idx result)
    (let [byte (vector-get bytes idx)]
      (if (if (= byte 16) (< (+ idx 1) count) false)
        (let [next-byte (vector-get bytes (+ idx 1))]
          (make-loop-step-state
            0
            (+ idx 2)
            (emit-byte (emit-byte result 16) (shift-runtime-call-index-value next-byte))))
        (make-loop-step-state 0 (+ idx 1) (emit-byte result byte))))))
(defn shift-runtime-call-indices-step-8 [bytes state count]
  (let [s1 (if (= (vector-get state 0) 1) state (shift-runtime-call-indices-step bytes (vector-get state 2) (vector-get state 1) count))
    s2 (if (= (vector-get s1 0) 1) s1 (shift-runtime-call-indices-step bytes (vector-get s1 2) (vector-get s1 1) count))
    s3 (if (= (vector-get s2 0) 1) s2 (shift-runtime-call-indices-step bytes (vector-get s2 2) (vector-get s2 1) count))
    s4 (if (= (vector-get s3 0) 1) s3 (shift-runtime-call-indices-step bytes (vector-get s3 2) (vector-get s3 1) count))
    s5 (if (= (vector-get s4 0) 1) s4 (shift-runtime-call-indices-step bytes (vector-get s4 2) (vector-get s4 1) count))
    s6 (if (= (vector-get s5 0) 1) s5 (shift-runtime-call-indices-step bytes (vector-get s5 2) (vector-get s5 1) count))
    s7 (if (= (vector-get s6 0) 1) s6 (shift-runtime-call-indices-step bytes (vector-get s6 2) (vector-get s6 1) count))
    s8 (if (= (vector-get s7 0) 1) s7 (shift-runtime-call-indices-step bytes (vector-get s7 2) (vector-get s7 1) count))]
    s8))
(defn shift-runtime-call-indices-loop [bytes state count]
  (if (= (vector-get state 0) 1)
    (vector-get state 2)
    (shift-runtime-call-indices-loop bytes (shift-runtime-call-indices-step-8 bytes state count) count)))
(defn shift-runtime-call-indices [bytes]
  (shift-runtime-call-indices-loop bytes (make-loop-step-state 0 0 (vector-new 64)) (vector-length bytes)))
(defn shift-standalone-runtime-call-index-value [idx]
  (if (= idx 11)
    1
    (if (= idx 12)
      2
      (if (= idx 13)
        3
      (if (= idx 14)
        4
        (if (= idx 15)
          5
          (if (and (> idx 0) (< idx 11))
            (+ idx 5)
            idx)))))))
(defn shift-standalone-runtime-call-indices-step [bytes result idx count]
  (if (>= idx count)
    (make-loop-step-state 1 idx result)
    (let [byte (vector-get bytes idx)]
      (if (if (= byte 16) (< (+ idx 1) count) false)
        (let [next-byte (vector-get bytes (+ idx 1))]
          (make-loop-step-state
            0
            (+ idx 2)
            (emit-byte (emit-byte result 16) (shift-standalone-runtime-call-index-value next-byte))))
        (make-loop-step-state 0 (+ idx 1) (emit-byte result byte))))))
(defn shift-standalone-runtime-call-indices-step-8 [bytes state count]
  (let [s1 (if (= (vector-get state 0) 1) state (shift-standalone-runtime-call-indices-step bytes (vector-get state 2) (vector-get state 1) count))
    s2 (if (= (vector-get s1 0) 1) s1 (shift-standalone-runtime-call-indices-step bytes (vector-get s1 2) (vector-get s1 1) count))
    s3 (if (= (vector-get s2 0) 1) s2 (shift-standalone-runtime-call-indices-step bytes (vector-get s2 2) (vector-get s2 1) count))
    s4 (if (= (vector-get s3 0) 1) s3 (shift-standalone-runtime-call-indices-step bytes (vector-get s3 2) (vector-get s3 1) count))
    s5 (if (= (vector-get s4 0) 1) s4 (shift-standalone-runtime-call-indices-step bytes (vector-get s4 2) (vector-get s4 1) count))
    s6 (if (= (vector-get s5 0) 1) s5 (shift-standalone-runtime-call-indices-step bytes (vector-get s5 2) (vector-get s5 1) count))
    s7 (if (= (vector-get s6 0) 1) s6 (shift-standalone-runtime-call-indices-step bytes (vector-get s6 2) (vector-get s6 1) count))
    s8 (if (= (vector-get s7 0) 1) s7 (shift-standalone-runtime-call-indices-step bytes (vector-get s7 2) (vector-get s7 1) count))]
    s8))
(defn shift-standalone-runtime-call-indices-loop [bytes state count]
  (if (= (vector-get state 0) 1)
    (vector-get state 2)
    (shift-standalone-runtime-call-indices-loop bytes (shift-standalone-runtime-call-indices-step-8 bytes state count) count)))
(defn shift-standalone-runtime-call-indices [bytes]
  (shift-standalone-runtime-call-indices-loop bytes (make-loop-step-state 0 0 (vector-new 64)) (vector-length bytes)))
(defn build-function-body-function-standalone [func-meta]
  (do
    (root_push func-meta)
    (let [local-count (function-meta-local-count func-meta)
      ir-instrs (standalone-ir-instrs (function-meta-ir func-meta))]
    (do
      (root_push ir-instrs)
      (let [body0 (if (= local-count 0) (emit-byte (vector-new 64) 0) (emit-byte (emit-leb128 (emit-leb128 (vector-new 64) 1) local-count) 126))
        body (append-ir-instrs body0 ir-instrs 0 (vector-length ir-instrs))]
        (do
          (root_push body)
          (let [body1 (emit-byte body 11)]
            (do
              (root_push body1)
              (let [shifted-body (shift-runtime-call-indices body1)]
                (do
                  (root_pop)
                  (root_pop)
                  (root_pop)
                  (root_pop)
                  shifted-body))))))))))
(defn emit-standalone-allocator-body []
  (let [b0 (vector-new 64)
    b1 (emit-standalone-byte-seq-8 b0 1 2 127 32 0 167 65 7)
    b2 (emit-standalone-byte-seq-8 b1 106 65 120 113 33 1 65 32)
    b3 (emit-standalone-byte-seq-8 b2 40 2 0 33 2 32 2 69)
    b4a (emit-standalone-byte-seq-8 b3 4 64 65 128 192 0 33 2)
    b4 (emit-standalone-byte-seq-1 b4a 11)
    b5 (emit-standalone-byte-seq-8 b4 65 32 32 2 32 1 106 54)
    b6 (emit-standalone-byte-seq-6 b5 2 0 32 2 173 11)]
    b6))
(defn emit-standalone-print-body []
  (let [b0 (emit-byte (vector-new 256) 2)
    b1 (emit-standalone-byte-seq-8 b0 1 126 5 127 32 0 33 1)
    b2 (emit-standalone-byte-seq-8 b1 65 172 2 33 3 65 0 33)
    b3 (emit-standalone-byte-seq-8 b2 2 2 64 3 64 32 1 80)
    b4 (emit-standalone-byte-seq-8 b3 13 1 32 3 32 2 106 32)
    b5 (emit-standalone-byte-seq-8 b4 1 66 10 130 167 65 48 106)
    b6 (emit-standalone-byte-seq-8 b5 58 0 0 32 1 66 10 128)
    b7 (emit-standalone-byte-seq-8 b6 33 1 32 2 65 1 106 33)
    b8 (emit-standalone-byte-seq-8 b7 2 12 0 11 11 32 2 69)
    b9 (emit-standalone-byte-seq-8 b8 4 64 32 3 65 48 58 0)
    b10 (emit-standalone-byte-seq-8 b9 0 65 1 33 2 11 65 0)
    b11 (emit-standalone-byte-seq-8 b10 33 4 32 2 65 1 107 33)
    b12 (emit-standalone-byte-seq-8 b11 5 2 64 3 64 32 4 32)
    b13 (emit-standalone-byte-seq-8 b12 5 79 13 1 32 3 32 4)
    b14 (emit-standalone-byte-seq-8 b13 106 45 0 0 33 6 32 3)
    b15 (emit-standalone-byte-seq-8 b14 32 4 106 32 3 32 5 106)
    b16 (emit-standalone-byte-seq-8 b15 45 0 0 58 0 0 32 3)
    b17 (emit-standalone-byte-seq-8 b16 32 5 106 32 6 58 0 0)
    b18 (emit-standalone-byte-seq-8 b17 32 4 65 1 106 33 4 32)
    b19 (emit-standalone-byte-seq-8 b18 5 65 1 107 33 5 12 0)
    b20 (emit-standalone-byte-seq-8 b19 11 11 32 3 32 2 106 65)
    b21 (emit-standalone-byte-seq-8 b20 10 58 0 0 65 16 32 3)
    b22 (emit-standalone-byte-seq-8 b21 54 2 0 65 20 32 2 65)
    b23 (emit-standalone-byte-seq-8 b22 1 106 54 2 0 65 1 65)
    b24 (emit-standalone-byte-seq-8 b23 16 65 1 65 24 16 0 26)
    b25 (emit-standalone-byte-seq-1 b24 11)]
    b25))
(defn emit-standalone-zero-i64-body []
  (emit-standalone-byte-seq-4 (vector-new 8) 0 66 0 11))
(defn emit-standalone-identity-i64-body []
  (emit-standalone-byte-seq-4 (vector-new 8) 0 32 0 11))
(defn emit-standalone-drop-i64-body []
  (emit-byte (emit-standalone-byte-seq-4 (vector-new 8) 0 32 0 26) 11))
(defn emit-standalone-second-i64-body []
  (emit-standalone-byte-seq-4 (vector-new 8) 0 32 1 11))
(defn emit-standalone-file-exists-body []
  (let [b0 (emit-standalone-byte-seq-4 (vector-new 96) 1 3 127 32)
    b1 (emit-standalone-byte-seq-8 b0 0 167 65 8 106 33 1 32)
    b2 (emit-standalone-byte-seq-4 b1 0 167 40 2)
    b3 (emit-standalone-byte-seq-4 b2 4 33 2 65)
    b4 (emit-standalone-byte-seq-8 b3 3 65 0 32 1 32 2 65)
    b5 (emit-standalone-byte-seq-8 b4 0 66 2 66 0 65 0 65)
    b6 (emit-standalone-byte-seq-4 b5 192 17 16 13)
    b7 (emit-standalone-byte-seq-2 b6 33 3)
    b8 (emit-standalone-byte-seq-4 b7 32 3 69 4)
    b9 (emit-standalone-byte-seq-1 b8 64)
    b10 (emit-standalone-byte-seq-8 b9 65 192 17 40 2 0 16 14)
    b11 (emit-standalone-byte-seq-2 b10 26 11)
    b12 (emit-standalone-byte-seq-4 b11 32 3 69 173)
    b13 (emit-standalone-byte-seq-1 b12 11)]
    b13))
(defn emit-standalone-read-file-body []
  ;; 最大 1024-byte read の bounded slice。root stack 後方の 2176/2184/2240 scratch を使う。
  (let [b0a (emit-standalone-byte-seq-2 (vector-new 512) 1 6)
    b0 (emit-byte b0a 127)
    b1 (emit-standalone-byte-seq-8 b0 32 0 167 65 8 106 33 1)
    b2 (emit-standalone-byte-seq-8 b1 32 0 167 40 2 4 33 2)
    b3 (emit-standalone-byte-seq-8 b2 65 128 8 173 16 1 167 33)
    b4 (emit-standalone-byte-seq-1 b3 5)
    b5 (emit-standalone-byte-seq-8 b4 32 5 65 1 54 2 0 32)
    b6 (emit-standalone-byte-seq-6 b5 5 65 0 54 2 4)
    b7 (emit-standalone-byte-seq-8 b6 65 3 65 0 32 1 32 2)
    b8 (emit-standalone-byte-seq-8 b7 65 0 66 2 66 0 65 0)
    b9 (emit-standalone-byte-seq-6 b8 65 192 17 16 13 33)
    b10 (emit-standalone-byte-seq-1 b9 3)
    b11 (emit-standalone-byte-seq-8 b10 32 3 69 4 64 65 192 17)
    b12 (emit-standalone-byte-seq-8 b11 40 2 0 33 4 65 128 17)
    b13 (emit-standalone-byte-seq-8 b12 32 5 65 8 106 54 2 0)
    b14 (emit-standalone-byte-seq-8 b13 65 128 17 65 128 8 54 2)
    b15 (emit-standalone-byte-seq-1 b14 4)
    b16 (emit-standalone-byte-seq-8 b15 65 136 17 65 0 54 2 0)
    b17 (emit-standalone-byte-seq-8 b16 32 4 65 128 17 65 1 65)
    b18 (emit-standalone-byte-seq-4 b17 136 17 16 15)
    b19 (emit-standalone-byte-seq-1 b18 26)
    b20 (emit-standalone-byte-seq-8 b19 65 136 17 40 2 0 33 6)
    b21 (emit-standalone-byte-seq-4 b20 32 4 16 14)
    b22 (emit-standalone-byte-seq-1 b21 26)
    b23 (emit-standalone-byte-seq-6 b22 32 5 32 6 54 2)
    b24 (emit-standalone-byte-seq-1 b23 4)
    b25 (emit-standalone-byte-seq-1 b24 11)
    b26 (emit-standalone-byte-seq-4 b25 32 5 173 11)]
    b26))
(defn emit-standalone-write-file-body []
  ;; create|truncate + partial fd_write retry の bounded slice。root stack 後方の 2176/2184/2240 scratch を使う。
  (let [b0a (emit-standalone-byte-seq-2 (vector-new 1024) 1 10)
    b0 (emit-byte b0a 127)
    b1 (emit-standalone-byte-seq-8 b0 32 0 167 65 8 106 33 2)
    b2 (emit-standalone-byte-seq-8 b1 32 0 167 40 2 4 33 3)
    b3 (emit-standalone-byte-seq-8 b2 32 1 167 65 8 106 33 4)
    b4 (emit-standalone-byte-seq-8 b3 32 1 167 40 2 4 33 5)
    b5 (emit-standalone-byte-seq-8 b4 65 3 65 0 32 2 32 3)
    b6a (emit-byte b5 65)
    b6 (emit-leb128 b6a 5)
    b7a (emit-byte b6 66)
    b7 (emit-leb128-s b7a 64)
    b8a (emit-byte b7 66)
    b8 (emit-leb128-s b8a 0)
    b9 (emit-standalone-byte-seq-8 b8 65 0 65 192 17 16 13 33)
    b10 (emit-standalone-byte-seq-1 b9 6)
    b11a (emit-byte b10 65)
    b11 (emit-leb128-s b11a -1)
    b12a (emit-byte b11 33)
    b12 (emit-leb128 b12a 8)
    b13 (emit-standalone-byte-seq-8 b12 32 6 69 4 64 65 192 17)
    b14 (emit-standalone-byte-seq-6 b13 40 2 0 33 7 32)
    b15 (emit-standalone-byte-seq-8 b14 5 33 11 65 0 33 9 65)
    b16 (emit-standalone-byte-seq-8 b15 0 33 10 2 64 3 64 32)
    b17 (emit-standalone-byte-seq-8 b16 11 69 13 1 65 128 17 32)
    b18 (emit-standalone-byte-seq-8 b17 4 32 10 106 54 2 0 65)
    b19 (emit-standalone-byte-seq-8 b18 128 17 32 11 54 2 4 65)
    b20 (emit-standalone-byte-seq-8 b19 136 17 65 0 54 2 0 32)
    b21 (emit-standalone-byte-seq-8 b20 7 65 128 17 65 1 65 136)
    b22 (emit-standalone-byte-seq-8 b21 17 16 0 33 6 32 6 4)
    b23a (emit-byte b22 64)
    b23b (emit-byte b23a 65)
    b23c (emit-leb128-s b23b -1)
    b23d (emit-byte b23c 33)
    b23e (emit-leb128 b23d 8)
    b23f (emit-standalone-byte-seq-2 b23e 12 2)
    b23 (emit-byte b23f 11)
    b24 (emit-standalone-byte-seq-8 b23 65 136 17 40 2 0 33 6)
    b25 (emit-standalone-byte-seq-4 b24 32 6 69 4)
    b26a (emit-byte b25 64)
    b26b (emit-byte b26a 65)
    b26c (emit-leb128-s b26b -1)
    b26d (emit-byte b26c 33)
    b26e (emit-leb128 b26d 8)
    b26f (emit-standalone-byte-seq-2 b26e 12 2)
    b26 (emit-byte b26f 11)
    b27 (emit-standalone-byte-seq-6 b26 32 6 32 11 75 4)
    b28a (emit-byte b27 64)
    b28b (emit-byte b28a 65)
    b28c (emit-leb128-s b28b -1)
    b28d (emit-byte b28c 33)
    b28e (emit-leb128 b28d 8)
    b28f (emit-standalone-byte-seq-2 b28e 12 2)
    b28 (emit-byte b28f 11)
    b29 (emit-standalone-byte-seq-8 b28 32 9 32 6 106 33 9 32)
    b30 (emit-standalone-byte-seq-8 b29 10 32 6 106 33 10 32 11)
    b31 (emit-standalone-byte-seq-8 b30 32 6 107 33 11 12 0 11)
    b32 (emit-standalone-byte-seq-6 b31 32 9 33 8 11 32)
    b33 (emit-standalone-byte-seq-8 b32 7 16 14 26 11 32 8 172)
    b34 (emit-standalone-byte-seq-1 b33 11)]
    b34))
(defn emit-standalone-write-file-bytes-body []
  ;; Vector の下位8 bitを packed buffer へ詰め、partial fd_write を再試行する bounded slice。
  ;; root stack 後方の 2176/2184/2240 scratch を使う。
  (let [b0a (emit-standalone-byte-seq-2 (vector-new 1024) 1 11)
    b0 (emit-byte b0a 127)
    b1 (emit-standalone-byte-seq-8 b0 32 0 167 65 8 106 33 2)
    b2 (emit-standalone-byte-seq-8 b1 32 0 167 40 2 4 33 3)
    b3 (emit-standalone-byte-seq-4 b2 32 1 167 33)
    b3a (emit-byte b3 4)
    b4 (emit-standalone-byte-seq-4 b3a 32 4 40 2)
    b4a (emit-byte b4 8)
    b4b (emit-standalone-byte-seq-2 b4a 33 5)
    b5 (emit-standalone-byte-seq-8 b4b 32 5 173 16 1 167 33 6)
    b6 (emit-standalone-byte-seq-4 b5 65 0 33 7)
    b7 (emit-standalone-byte-seq-4 b6 2 64 3 64)
    b8 (emit-standalone-byte-seq-6 b7 32 7 32 5 79 13)
    b8a (emit-byte b8 1)
    b9 (emit-standalone-byte-seq-4 b8a 32 6 32 7)
    b9a (emit-byte b9 106)
    b10 (emit-standalone-byte-seq-8 b9a 32 4 65 16 106 32 7 65)
    b11 (emit-standalone-byte-seq-8 b10 3 116 106 41 0 0 167 58)
    b12 (emit-standalone-byte-seq-2 b11 0 0)
    b13 (emit-standalone-byte-seq-8 b12 32 7 65 1 106 33 7 12)
    b14 (emit-byte b13 0)
    b15 (emit-standalone-byte-seq-2 b14 11 11)
    b16 (emit-standalone-byte-seq-8 b15 65 3 65 0 32 2 32 3)
    b16a (emit-byte b16 65)
    b17a (emit-leb128 b16a 5)
    b17b (emit-byte b17a 66)
    b17c (emit-leb128-s b17b 64)
    b17d (emit-byte b17c 66)
    b17e (emit-leb128-s b17d 0)
    b18 (emit-standalone-byte-seq-8 b17e 65 0 65 192 17 16 13 26)
    b19 (emit-standalone-byte-seq-8 b18 65 192 17 40 2 0 33 8)
    b20 (emit-standalone-byte-seq-8 b19 32 5 33 11 65 0 33 9)
    b21 (emit-standalone-byte-seq-8 b20 65 0 33 10 2 64 3 64)
    b22 (emit-standalone-byte-seq-2 b21 32 11)
    b23 (emit-standalone-byte-seq-8 b22 69 13 1 65 128 17 32 6)
    b24 (emit-standalone-byte-seq-8 b23 32 10 106 54 2 0 65 128)
    b25 (emit-standalone-byte-seq-8 b24 17 32 11 54 2 4 65 136)
    b26 (emit-standalone-byte-seq-8 b25 17 65 0 54 2 0 32 8)
    b27 (emit-standalone-byte-seq-8 b26 65 128 17 65 1 65 136 17)
    b28 (emit-standalone-byte-seq-8 b27 16 0 33 12 32 12 4 64)
    b29a (emit-byte b28 65)
    b29b (emit-leb128-s b29a -1)
    b29c (emit-byte b29b 33)
    b29d (emit-leb128 b29c 9)
    b29e (emit-standalone-byte-seq-2 b29d 12 2)
    b29 (emit-byte b29e 11)
    b30 (emit-standalone-byte-seq-8 b29 65 136 17 40 2 0 33 12)
    b31 (emit-standalone-byte-seq-4 b30 32 12 69 4)
    b32a (emit-byte b31 64)
    b32b (emit-byte b32a 65)
    b32c (emit-leb128-s b32b -1)
    b32d (emit-byte b32c 33)
    b32e (emit-leb128 b32d 9)
    b32f (emit-standalone-byte-seq-2 b32e 12 2)
    b32 (emit-byte b32f 11)
    b33 (emit-standalone-byte-seq-6 b32 32 12 32 11 75 4)
    b34a (emit-byte b33 64)
    b34b (emit-byte b34a 65)
    b34c (emit-leb128-s b34b -1)
    b34d (emit-byte b34c 33)
    b34e (emit-leb128 b34d 9)
    b34f (emit-standalone-byte-seq-2 b34e 12 2)
    b34 (emit-byte b34f 11)
    b35 (emit-standalone-byte-seq-8 b34 32 9 32 12 106 33 9 32)
    b36 (emit-standalone-byte-seq-8 b35 10 32 12 106 33 10 32 11)
    b37 (emit-standalone-byte-seq-8 b36 32 12 107 33 11 12 0 11)
    b37a (emit-standalone-byte-seq-2 b37 11 32)
    b38 (emit-standalone-byte-seq-1 b37a 8)
    b39 (emit-standalone-byte-seq-6 b38 16 14 26 32 9 172)
    b40 (emit-standalone-byte-seq-1 b39 11)]
    b40))
(defn emit-standalone-print-string-body []
  (let [b0 (emit-standalone-byte-seq-8 (vector-new 64) 0 65 0 32 0 167 65 8)
    b1 (emit-standalone-byte-seq-4 b0 106 54 2 0)
    b2 (emit-standalone-byte-seq-8 b1 65 4 32 0 167 40 2 4)
    b2a (emit-standalone-byte-seq-2 b2 54 2)
    b2b (emit-standalone-byte-seq-1 b2a 0)
    b3 (emit-standalone-byte-seq-8 b2b 65 1 65 0 65 1 65 8)
    b4 (emit-standalone-byte-seq-4 b3 16 0 26 11)]
    b4))
(defn emit-standalone-string-concat-body []
  (let [b0 (emit-standalone-byte-seq-8 (vector-new 256) 3 5 127 1 126 3 127 32)
    b1a (emit-standalone-byte-seq-6 b0 0 167 33 2 32 1)
    b1 (emit-standalone-byte-seq-1 b1a 167)
    b2 (emit-standalone-byte-seq-4 b1 33 3 32 2)
    b3a (emit-standalone-byte-seq-6 b2 40 2 4 33 4 32)
    b3 (emit-standalone-byte-seq-1 b3a 3)
    b4 (emit-standalone-byte-seq-8 b3 40 2 4 33 5 32 4 32)
    b5 (emit-standalone-byte-seq-6 b4 5 106 33 6 32 6)
    b6 (emit-standalone-byte-seq-8 b5 173 66 8 124 16 1 33 7)
    b7 (emit-standalone-byte-seq-8 b6 32 7 167 65 1 54 2 0)
    b8 (emit-standalone-byte-seq-8 b7 32 7 167 65 4 106 32 6)
    b9a (emit-standalone-byte-seq-2 b8 54 2)
    b9 (emit-standalone-byte-seq-1 b9a 0)
    b10 (emit-standalone-byte-seq-8 b9 65 0 33 8 2 64 3 64)
    b11 (emit-standalone-byte-seq-6 b10 32 8 32 4 79 13)
    b12 (emit-standalone-byte-seq-1 b11 1)
    b13 (emit-standalone-byte-seq-8 b12 32 2 65 8 106 32 8 106)
    b13a (emit-standalone-byte-seq-4 b13 45 0 0 33)
    b13b (emit-standalone-byte-seq-1 b13a 10)
    b14 (emit-standalone-byte-seq-8 b13b 32 7 167 65 8 106 32 8)
    b15 (emit-standalone-byte-seq-8 b14 106 32 10 58 0 0 32 8)
    b16 (emit-standalone-byte-seq-8 b15 65 1 106 33 8 12 0 11)
    b17 (emit-standalone-byte-seq-1 b16 11)
    b18 (emit-standalone-byte-seq-8 b17 65 0 33 8 2 64 3 64)
    b19 (emit-standalone-byte-seq-6 b18 32 8 32 5 79 13)
    b20 (emit-standalone-byte-seq-1 b19 1)
    b21 (emit-standalone-byte-seq-8 b20 32 3 65 8 106 32 8 106)
    b21a (emit-standalone-byte-seq-4 b21 45 0 0 33)
    b21b (emit-standalone-byte-seq-1 b21a 10)
    b22 (emit-standalone-byte-seq-8 b21b 32 7 167 65 8 106 32 4)
    b23 (emit-standalone-byte-seq-8 b22 106 32 8 106 32 10 58 0)
    b24 (emit-standalone-byte-seq-1 b23 0)
    b25 (emit-standalone-byte-seq-6 b24 32 8 65 1 106 33)
    b26 (emit-standalone-byte-seq-4 b25 8 12 0 11)
    b27 (emit-standalone-byte-seq-1 b26 11)
    b28 (emit-standalone-byte-seq-2 b27 32 7)
    b29 (emit-standalone-byte-seq-1 b28 11)]
    b29))
(defn emit-standalone-substring-body []
  (let [b0 (emit-standalone-byte-seq-8 (vector-new 256) 3 5 127 1 126 2 127 32)
    b1 (emit-standalone-byte-seq-8 b0 0 167 33 3 32 1 167 33)
    b2 (emit-standalone-byte-seq-8 b1 4 32 2 167 33 5 32 3)
    b3 (emit-standalone-byte-seq-8 b2 40 2 4 33 6 32 4 32)
    b4 (emit-standalone-byte-seq-8 b3 5 75 4 64 0 11 32 5)
    b5 (emit-standalone-byte-seq-8 b4 32 6 75 4 64 0 11 32)
    b6 (emit-standalone-byte-seq-8 b5 5 32 4 107 33 7 32 7)
    b7 (emit-standalone-byte-seq-8 b6 173 66 8 124 16 1 33 8)
    b8 (emit-standalone-byte-seq-8 b7 32 8 167 65 1 54 2 0)
    b9 (emit-standalone-byte-seq-8 b8 32 8 167 65 4 106 32 7)
    b10 (emit-standalone-byte-seq-8 b9 54 2 0 65 0 33 9 2)
    b11 (emit-standalone-byte-seq-8 b10 64 3 64 32 9 32 7 79)
    b12 (emit-standalone-byte-seq-8 b11 13 1 32 3 65 8 106 32)
    b13 (emit-standalone-byte-seq-8 b12 4 106 32 9 106 45 0 0)
    b14 (emit-standalone-byte-seq-8 b13 33 10 32 8 167 65 8 106)
    b15 (emit-standalone-byte-seq-8 b14 32 9 106 32 10 58 0 0)
    b16 (emit-standalone-byte-seq-8 b15 32 9 65 1 106 33 9 12)
    b17 (emit-standalone-byte-seq-6 b16 0 11 11 32 8 11)]
    b17))
(defn emit-standalone-root-push-body []
  (let [b0 (emit-standalone-byte-seq-8 (vector-new 32) 1 2 127 65 192 0 40 2)
    b1 (emit-standalone-byte-seq-8 b0 0 33 1 32 1 65 240 1)
    b1a (emit-standalone-byte-seq-1 b1 79)
    b1b (emit-standalone-byte-seq-4 b1a 4 64 0 11)
    b1c (emit-standalone-byte-seq-4 b1b 65 128 1 32)
    b1d (emit-standalone-byte-seq-1 b1c 1)
    b2 (emit-standalone-byte-seq-8 b1d 65 3 116 106 33 2 32 2)
    b2a (emit-standalone-byte-seq-4 b2 32 0 55 3)
    b2b (emit-standalone-byte-seq-1 b2a 0)
    b3 (emit-standalone-byte-seq-8 b2b 65 192 0 32 1 65 1 106)
    b3a (emit-byte b3 54)
    b3b (emit-byte b3a 2)
    b3c (emit-byte b3b 0)
    b4 (emit-standalone-byte-seq-4 b3c 32 1 173 11)]
    b4))
(defn emit-standalone-root-pop-body []
  (let [b0 (emit-standalone-byte-seq-8 (vector-new 32) 1 2 127 65 192 0 40 2)
    b1 (emit-standalone-byte-seq-8 b0 0 33 0 32 0 69 4 126)
    b2 (emit-standalone-byte-seq-8 b1 66 0 5 32 0 65 1 107)
    b3 (emit-standalone-byte-seq-8 b2 33 0 65 192 0 32 0 54)
    b4 (emit-standalone-byte-seq-8 b3 2 0 65 128 1 32 0 65)
    b5 (emit-standalone-byte-seq-8 b4 3 116 106 33 1 32 1 41)
    b6 (emit-standalone-byte-seq-4 b5 3 0 11 11)]
    b6))
(defn emit-standalone-root-set-body []
  (let [b0 (emit-standalone-byte-seq-8 (vector-new 24) 1 3 127 32 0 167 33 2)
    b1 (emit-standalone-byte-seq-8 b0 65 192 0 40 2 0 33 4)
    b2 (emit-standalone-byte-seq-8 b1 32 2 32 4 79 4 64 0)
    b2a (emit-standalone-byte-seq-1 b2 11)
    b3 (emit-standalone-byte-seq-8 b2a 65 128 1 32 2 65 3 116)
    b4 (emit-standalone-byte-seq-8 b3 106 33 3 32 3 32 1 55)
    b5 (emit-standalone-byte-seq-6 b4 3 0 32 2 173 11)]
    b5))
(defn emit-standalone-command-line-arg-body []
  ;; args_sizes_get の scratch は root stack 外の 2256/2260 に置き、
  ;; args_get の argv table/buffer は allocator 管理の連続領域へ動的に確保する。
  (let [b0 (emit-standalone-byte-seq-4 (vector-new 512) 1 10 127 65)
    b1 (emit-standalone-byte-seq-8 b0 208 17 65 212 17 16 11 26)
    b2 (emit-standalone-byte-seq-8 b1 65 208 17 40 2 0 33 2)
    b3 (emit-standalone-byte-seq-8 b2 65 212 17 40 2 0 33 3)
    b4 (emit-standalone-byte-seq-8 b3 32 2 65 128 8 75 4 64)
    b5 (emit-standalone-byte-seq-4 b4 0 11 32 3)
    b6 (emit-standalone-byte-seq-6 b5 65 128 32 75 4 64)
    b7 (emit-standalone-byte-seq-8 b6 0 11 32 0 167 33 1 32)
    b8 (emit-standalone-byte-seq-6 b7 1 32 2 79 4 64)
    b9 (emit-standalone-byte-seq-8 b8 66 8 16 1 167 33 9 32)
    b10 (emit-standalone-byte-seq-8 b9 9 65 0 54 2 4 32 9)
    b11 (emit-standalone-byte-seq-8 b10 173 15 11 32 2 65 4 108)
    b12 (emit-standalone-byte-seq-8 b11 32 3 106 173 16 1 167 33)
    b13 (emit-standalone-byte-seq-8 b12 4 32 4 32 2 65 4 108)
    b14 (emit-standalone-byte-seq-8 b13 106 33 5 32 4 32 5 16)
    b15 (emit-standalone-byte-seq-8 b14 12 26 32 4 32 1 65 4)
    b16 (emit-standalone-byte-seq-2 b15 108 106)
    b17 (emit-standalone-byte-seq-8 b16 40 2 0 33 6 32 6 33)
    b18 (emit-standalone-byte-seq-8 b17 7 65 0 33 8 2 64 3)
    b19 (emit-standalone-byte-seq-8 b18 64 32 7 45 0 0 69 13)
    b20 (emit-standalone-byte-seq-8 b19 1 32 7 65 1 106 33 7)
    b21 (emit-standalone-byte-seq-8 b20 32 8 65 1 106 33 8 12)
    b22 (emit-standalone-byte-seq-8 b21 0 11 11 65 8 32 8 106)
    b23 (emit-standalone-byte-seq-8 b22 173 16 1 167 33 9 32 9)
    b24 (emit-standalone-byte-seq-8 b23 32 8 54 2 4 65 0 33)
    b25 (emit-standalone-byte-seq-8 b24 10 2 64 3 64 32 10 32)
    b26 (emit-standalone-byte-seq-8 b25 8 79 13 1 32 9 65 8)
    b27 (emit-standalone-byte-seq-8 b26 106 32 10 106 32 6 32 10)
    b28 (emit-standalone-byte-seq-8 b27 106 45 0 0 58 0 0 32)
    b29 (emit-standalone-byte-seq-8 b28 10 65 1 106 33 10 12 0)
    b30 (emit-standalone-byte-seq-4 b29 11 11 32 9)
    b31 (emit-standalone-byte-seq-2 b30 173 11)]
    b31))
(defn emit-standalone-wrapper-body [main-func-idx]
  (let [b0 (vector-new 8)
    b1 (emit-byte b0 0)
    b2 (emit-byte b1 16)
    b3 (emit-leb128 b2 main-func-idx)
    b4 (emit-byte b3 26)
    b5 (emit-byte b4 11)]
    b5))
(defn append-standalone-code-body [body func-body]
  (do
    (root_push body)
    (root_push func-body)
    (let [with-size (emit-leb128 body (vector-length func-body))]
      (do
        (root_push with-size)
        (let [result (append-byte-vector-chunked with-size func-body 0 (vector-length func-body))]
          (do
            (root_pop)
            (root_pop)
            (root_pop)
            result))))))
(defn standalone-runtime-body [idx]
  (if (= idx 0)
    (emit-standalone-allocator-body)
    (if (= idx 1)
      (emit-standalone-print-body)
      (if (= idx 2)
        (emit-standalone-zero-i64-body)
        (if (= idx 3)
          (emit-standalone-zero-i64-body)
            (if (= idx 4)
            (emit-standalone-string-concat-body)
            (if (= idx 5)
              (emit-standalone-substring-body)
                (if (= idx 6)
                  (emit-standalone-zero-i64-body)
                  (if (= idx 7)
                  (emit-standalone-root-push-body)
                  (if (= idx 8)
                    (emit-standalone-root-pop-body)
                    (if (= idx 9)
                      (emit-standalone-root-set-body)
                      (if (= idx 10)
                        (emit-standalone-print-string-body)
      (if (= idx 11)
        (emit-standalone-command-line-arg-body)
        (if (= idx 12)
          (emit-standalone-file-exists-body)
          (if (= idx 13)
            (emit-standalone-read-file-body)
            (if (= idx 14)
              (emit-standalone-write-file-body)
              (if (= idx 15)
                (emit-standalone-write-file-bytes-body)
                (emit-standalone-drop-i64-body))))))))))))))))))
(defn append-standalone-runtime-bodies-step [body idx]
  (if (>= idx 16)
    (make-loop-step-state 1 idx body)
    (let [func-body (shift-standalone-runtime-call-indices (standalone-runtime-body idx))]
      (do
        (root_push func-body)
        (let [next-body (append-standalone-code-body body func-body)]
          (do
            (root_pop)
            (make-loop-step-state 0 (+ idx 1) next-body)))))))
(defn continue-append-standalone-runtime-bodies-step [state]
  (if (= (vector-get state 0) 1)
    state
    (append-standalone-runtime-bodies-step (vector-get state 2) (vector-get state 1))))
(defn append-standalone-runtime-bodies-step-8 [body idx]
  (let [s1 (append-standalone-runtime-bodies-step body idx)
    s2 (continue-append-standalone-runtime-bodies-step s1)
    s3 (continue-append-standalone-runtime-bodies-step s2)
    s4 (continue-append-standalone-runtime-bodies-step s3)
    s5 (continue-append-standalone-runtime-bodies-step s4)
    s6 (continue-append-standalone-runtime-bodies-step s5)
    s7 (continue-append-standalone-runtime-bodies-step s6)
    s8 (continue-append-standalone-runtime-bodies-step s7)]
    s8))
(defn append-standalone-runtime-bodies [body idx]
  (let [step (append-standalone-runtime-bodies-step-8 body idx)]
    (if (= (vector-get step 0) 1)
      (vector-get step 2)
      (append-standalone-runtime-bodies (vector-get step 2) (vector-get step 1)))))
(defn build-function-body-function-progress-debug [func-meta]
  (do
    (root_push func-meta)
    (let [local-count (function-meta-local-count func-meta)
      ir-instrs (function-meta-ir func-meta)]
      (do
        (print 582)
        (print local-count)
        (print 583)
        (print (vector-length ir-instrs))
        (root_push ir-instrs)
        (let [body0 (if (= local-count 0) (emit-byte (vector-new 64) 0) (emit-byte (emit-leb128 (emit-leb128 (vector-new 64) 1) local-count) 126))]
          (do
            (print 584)
            (print (vector-length body0))
            (let [body1 (append-ir-instrs-progress-debug body0 ir-instrs 0 (vector-length ir-instrs))]
              (do
                (print 585)
                (print (vector-length body1))
                (root_push body1)
                (let [result (emit-byte body1 11)]
                  (do
                    (print 589)
                    (print (vector-length result))
                    (root_pop)
                    (root_pop)
                    (root_pop)
                    result))))))))))
(defn append-code-bodies-step [body ir-list idx func-count]
  (if (>= idx func-count)
    (make-loop-step-state 1 idx body)
    (let [func-body (build-function-body (vector-get ir-list idx))
      with-size (emit-leb128 body (vector-length func-body))
      with-body (append-byte-vector-chunked with-size func-body 0 (vector-length func-body))]
      (make-loop-step-state 0 (+ idx 1) with-body))))

(defn continue-append-code-bodies-step [ir-list func-count state]
  (if (= (vector-get state 0) 1)
    state
    (append-code-bodies-step (vector-get state 2) ir-list (vector-get state 1) func-count)))

(defn append-code-bodies-step-8 [body ir-list idx func-count]
  (let [step1 (append-code-bodies-step body ir-list idx func-count)
    step2 (continue-append-code-bodies-step ir-list func-count step1)
    step3 (continue-append-code-bodies-step ir-list func-count step2)
    step4 (continue-append-code-bodies-step ir-list func-count step3)
    step5 (continue-append-code-bodies-step ir-list func-count step4)
    step6 (continue-append-code-bodies-step ir-list func-count step5)
    step7 (continue-append-code-bodies-step ir-list func-count step6)
    step8 (continue-append-code-bodies-step ir-list func-count step7)]
    step8))

(defn continue-append-code-bodies-step-8 [ir-list func-count state]
  (if (= (vector-get state 0) 1)
    state
    (append-code-bodies-step-8 (vector-get state 2) ir-list (vector-get state 1) func-count)))

(defn append-code-bodies-step-64 [body ir-list idx func-count]
  (let [step1 (append-code-bodies-step-8 body ir-list idx func-count)
    step2 (continue-append-code-bodies-step-8 ir-list func-count step1)
    step3 (continue-append-code-bodies-step-8 ir-list func-count step2)
    step4 (continue-append-code-bodies-step-8 ir-list func-count step3)
    step5 (continue-append-code-bodies-step-8 ir-list func-count step4)
    step6 (continue-append-code-bodies-step-8 ir-list func-count step5)
    step7 (continue-append-code-bodies-step-8 ir-list func-count step6)
    step8 (continue-append-code-bodies-step-8 ir-list func-count step7)]
    step8))

(defn continue-append-code-bodies-step-64 [ir-list func-count state]
  (if (= (vector-get state 0) 1)
    state
    (append-code-bodies-step-64 (vector-get state 2) ir-list (vector-get state 1) func-count)))

(defn append-code-bodies-step-512 [body ir-list idx func-count]
  (let [step1 (append-code-bodies-step-64 body ir-list idx func-count)
    step2 (continue-append-code-bodies-step-64 ir-list func-count step1)
    step3 (continue-append-code-bodies-step-64 ir-list func-count step2)
    step4 (continue-append-code-bodies-step-64 ir-list func-count step3)
    step5 (continue-append-code-bodies-step-64 ir-list func-count step4)
    step6 (continue-append-code-bodies-step-64 ir-list func-count step5)
    step7 (continue-append-code-bodies-step-64 ir-list func-count step6)
    step8 (continue-append-code-bodies-step-64 ir-list func-count step7)]
    step8))

(defn append-code-bodies [body ir-list idx func-count]
  (do
    (root_push body)
    (root_push ir-list)
    (let [step (append-code-bodies-step-512 body ir-list idx func-count)]
      (do
        (root_push step)
        (let [result
            (if (= (vector-get step 0) 1)
              (vector-get step 2)
              (append-code-bodies (vector-get step 2) ir-list (vector-get step 1) func-count))]
          (do
            (root_pop)
            (root_pop)
            (root_pop)
            result))))))
(defn emit-code-section-list [ir-list] (let [func-count (vector-length ir-list) body0 (emit-leb128 (vector-new 64) func-count) body1 (append-code-bodies body0 ir-list 0 func-count) body-size (vector-length body1) result0 (emit-byte (vector-new 64) 10) result1 (emit-leb128 result0 body-size)] (append-byte-vector result1 body1 0 body-size)))
(defn append-code-bodies-functions-step [body functions idx func-count]
  (if (>= idx func-count)
    (make-loop-step-state 1 idx body)
    (let [func-body (build-function-body-function (vector-get functions idx))]
      (do
        (root_push func-body)
        (let [with-size (emit-leb128 body (vector-length func-body))
          with-body (append-byte-vector-chunked with-size func-body 0 (vector-length func-body))]
          (do
            (root_pop)
            (make-loop-step-state 0 (+ idx 1) with-body)))))))

(defn append-code-bodies-functions-step-progress-debug [body functions idx func-count]
  (if (>= idx func-count)
    (make-loop-step-state 1 idx body)
    (let [func-meta (vector-get functions idx)
      local-count (function-meta-local-count func-meta)
      ir-instrs (function-meta-ir func-meta)]
      (do
        (print 578)
        (print idx)
        (print 579)
        (print local-count)
        (print 580)
        (print (vector-length ir-instrs))
        (let [func-body (build-function-body-function-progress-debug func-meta)]
          (do
            (root_push func-body)
            (let [with-size (emit-leb128 body (vector-length func-body))
              with-body (append-byte-vector-chunked with-size func-body 0 (vector-length func-body))]
              (do
                (print 581)
                (print (vector-length func-body))
                (root_pop)
                (make-loop-step-state 0 (+ idx 1) with-body)))))))))

(defn continue-append-code-bodies-functions-step [functions func-count state]
  (if (= (vector-get state 0) 1)
    state
    (append-code-bodies-functions-step (vector-get state 2) functions (vector-get state 1) func-count)))

(defn continue-append-code-bodies-functions-step-progress-debug [functions func-count state]
  (if (= (vector-get state 0) 1)
    state
    (append-code-bodies-functions-step-progress-debug (vector-get state 2) functions (vector-get state 1) func-count)))

(defn append-code-bodies-functions-step-8 [body functions idx func-count]
  (let [step1 (append-code-bodies-functions-step body functions idx func-count)
    step2 (continue-append-code-bodies-functions-step functions func-count step1)
    step3 (continue-append-code-bodies-functions-step functions func-count step2)
    step4 (continue-append-code-bodies-functions-step functions func-count step3)
    step5 (continue-append-code-bodies-functions-step functions func-count step4)
    step6 (continue-append-code-bodies-functions-step functions func-count step5)
    step7 (continue-append-code-bodies-functions-step functions func-count step6)
    step8 (continue-append-code-bodies-functions-step functions func-count step7)]
    step8))

(defn append-code-bodies-functions-step-8-progress-debug [body functions idx func-count]
  (let [step1 (append-code-bodies-functions-step-progress-debug body functions idx func-count)
    step2 (continue-append-code-bodies-functions-step-progress-debug functions func-count step1)
    step3 (continue-append-code-bodies-functions-step-progress-debug functions func-count step2)
    step4 (continue-append-code-bodies-functions-step-progress-debug functions func-count step3)
    step5 (continue-append-code-bodies-functions-step-progress-debug functions func-count step4)
    step6 (continue-append-code-bodies-functions-step-progress-debug functions func-count step5)
    step7 (continue-append-code-bodies-functions-step-progress-debug functions func-count step6)
    step8 (continue-append-code-bodies-functions-step-progress-debug functions func-count step7)]
    step8))

(defn continue-append-code-bodies-functions-step-8 [functions func-count state]
  (if (= (vector-get state 0) 1)
    state
    (append-code-bodies-functions-step-8 (vector-get state 2) functions (vector-get state 1) func-count)))

(defn continue-append-code-bodies-functions-step-8-progress-debug [functions func-count state]
  (if (= (vector-get state 0) 1)
    state
    (append-code-bodies-functions-step-8-progress-debug (vector-get state 2) functions (vector-get state 1) func-count)))

(defn append-code-bodies-functions-step-64 [body functions idx func-count]
  (let [step1 (append-code-bodies-functions-step-8 body functions idx func-count)
    step2 (continue-append-code-bodies-functions-step-8 functions func-count step1)
    step3 (continue-append-code-bodies-functions-step-8 functions func-count step2)
    step4 (continue-append-code-bodies-functions-step-8 functions func-count step3)
    step5 (continue-append-code-bodies-functions-step-8 functions func-count step4)
    step6 (continue-append-code-bodies-functions-step-8 functions func-count step5)
    step7 (continue-append-code-bodies-functions-step-8 functions func-count step6)
    step8 (continue-append-code-bodies-functions-step-8 functions func-count step7)]
    step8))

(defn append-code-bodies-functions-step-64-progress-debug [body functions idx func-count]
  (let [step1 (append-code-bodies-functions-step-8-progress-debug body functions idx func-count)
    step2 (continue-append-code-bodies-functions-step-8-progress-debug functions func-count step1)
    step3 (continue-append-code-bodies-functions-step-8-progress-debug functions func-count step2)
    step4 (continue-append-code-bodies-functions-step-8-progress-debug functions func-count step3)
    step5 (continue-append-code-bodies-functions-step-8-progress-debug functions func-count step4)
    step6 (continue-append-code-bodies-functions-step-8-progress-debug functions func-count step5)
    step7 (continue-append-code-bodies-functions-step-8-progress-debug functions func-count step6)
    step8 (continue-append-code-bodies-functions-step-8-progress-debug functions func-count step7)]
    step8))

(defn continue-append-code-bodies-functions-step-64 [functions func-count state]
  (if (= (vector-get state 0) 1)
    state
    (append-code-bodies-functions-step-64 (vector-get state 2) functions (vector-get state 1) func-count)))

(defn continue-append-code-bodies-functions-step-64-progress-debug [functions func-count state]
  (if (= (vector-get state 0) 1)
    state
    (append-code-bodies-functions-step-64-progress-debug (vector-get state 2) functions (vector-get state 1) func-count)))

(defn append-code-bodies-functions-step-512 [body functions idx func-count]
  (let [step1 (append-code-bodies-functions-step-64 body functions idx func-count)
    step2 (continue-append-code-bodies-functions-step-64 functions func-count step1)
    step3 (continue-append-code-bodies-functions-step-64 functions func-count step2)
    step4 (continue-append-code-bodies-functions-step-64 functions func-count step3)
    step5 (continue-append-code-bodies-functions-step-64 functions func-count step4)
    step6 (continue-append-code-bodies-functions-step-64 functions func-count step5)
    step7 (continue-append-code-bodies-functions-step-64 functions func-count step6)
    step8 (continue-append-code-bodies-functions-step-64 functions func-count step7)]
    step8))

(defn append-code-bodies-functions-step-512-progress-debug [body functions idx func-count]
  (let [step1 (append-code-bodies-functions-step-64-progress-debug body functions idx func-count)
    step2 (continue-append-code-bodies-functions-step-64-progress-debug functions func-count step1)
    step3 (continue-append-code-bodies-functions-step-64-progress-debug functions func-count step2)
    step4 (continue-append-code-bodies-functions-step-64-progress-debug functions func-count step3)
    step5 (continue-append-code-bodies-functions-step-64-progress-debug functions func-count step4)
    step6 (continue-append-code-bodies-functions-step-64-progress-debug functions func-count step5)
    step7 (continue-append-code-bodies-functions-step-64-progress-debug functions func-count step6)
    step8 (continue-append-code-bodies-functions-step-64-progress-debug functions func-count step7)]
    step8))

(defn append-code-bodies-functions [body functions idx func-count]
  (do
    (root_push body)
    (root_push functions)
    (let [step (append-code-bodies-functions-step-512 body functions idx func-count)]
      (do
        (root_push step)
        (let [result
            (if (= (vector-get step 0) 1)
              (vector-get step 2)
              (append-code-bodies-functions (vector-get step 2) functions (vector-get step 1) func-count))]
          (do
            (root_pop)
            (root_pop)
            (root_pop)
            result))))))

(defn append-code-bodies-functions-standalone-step [body functions idx func-count]
  (if (>= idx func-count)
    (make-loop-step-state 1 idx body)
    (let [func-body (build-function-body-function-standalone (vector-get functions idx))]
      (do
        (root_push func-body)
        (let [with-size (emit-leb128 body (vector-length func-body))
          with-body (append-byte-vector-chunked with-size func-body 0 (vector-length func-body))]
          (do
            (root_pop)
            (make-loop-step-state 0 (+ idx 1) with-body)))))))
(defn continue-append-code-bodies-functions-standalone-step [functions func-count state]
  (if (= (vector-get state 0) 1)
    state
    (append-code-bodies-functions-standalone-step (vector-get state 2) functions (vector-get state 1) func-count)))
(defn append-code-bodies-functions-standalone-step-8 [body functions idx func-count]
  (let [step1 (append-code-bodies-functions-standalone-step body functions idx func-count)
    step2 (continue-append-code-bodies-functions-standalone-step functions func-count step1)
    step3 (continue-append-code-bodies-functions-standalone-step functions func-count step2)
    step4 (continue-append-code-bodies-functions-standalone-step functions func-count step3)
    step5 (continue-append-code-bodies-functions-standalone-step functions func-count step4)
    step6 (continue-append-code-bodies-functions-standalone-step functions func-count step5)
    step7 (continue-append-code-bodies-functions-standalone-step functions func-count step6)
    step8 (continue-append-code-bodies-functions-standalone-step functions func-count step7)]
    step8))
(defn continue-append-code-bodies-functions-standalone-step-8 [functions func-count state]
  (if (= (vector-get state 0) 1)
    state
    (append-code-bodies-functions-standalone-step-8 (vector-get state 2) functions (vector-get state 1) func-count)))
(defn append-code-bodies-functions-standalone-step-64 [body functions idx func-count]
  (let [step1 (append-code-bodies-functions-standalone-step-8 body functions idx func-count)
    step2 (continue-append-code-bodies-functions-standalone-step-8 functions func-count step1)
    step3 (continue-append-code-bodies-functions-standalone-step-8 functions func-count step2)
    step4 (continue-append-code-bodies-functions-standalone-step-8 functions func-count step3)
    step5 (continue-append-code-bodies-functions-standalone-step-8 functions func-count step4)
    step6 (continue-append-code-bodies-functions-standalone-step-8 functions func-count step5)
    step7 (continue-append-code-bodies-functions-standalone-step-8 functions func-count step6)
    step8 (continue-append-code-bodies-functions-standalone-step-8 functions func-count step7)]
    step8))
(defn continue-append-code-bodies-functions-standalone-step-64 [functions func-count state]
  (if (= (vector-get state 0) 1)
    state
    (append-code-bodies-functions-standalone-step-64 (vector-get state 2) functions (vector-get state 1) func-count)))
(defn append-code-bodies-functions-standalone-step-512 [body functions idx func-count]
  (let [step1 (append-code-bodies-functions-standalone-step-64 body functions idx func-count)
    step2 (continue-append-code-bodies-functions-standalone-step-64 functions func-count step1)
    step3 (continue-append-code-bodies-functions-standalone-step-64 functions func-count step2)
    step4 (continue-append-code-bodies-functions-standalone-step-64 functions func-count step3)
    step5 (continue-append-code-bodies-functions-standalone-step-64 functions func-count step4)
    step6 (continue-append-code-bodies-functions-standalone-step-64 functions func-count step5)
    step7 (continue-append-code-bodies-functions-standalone-step-64 functions func-count step6)
    step8 (continue-append-code-bodies-functions-standalone-step-64 functions func-count step7)]
    step8))
(defn continue-append-code-bodies-functions-standalone-step-512 [functions func-count state]
  (if (= (vector-get state 0) 1)
    state
    (append-code-bodies-functions-standalone-step-512 (vector-get state 2) functions (vector-get state 1) func-count)))
(defn append-code-bodies-functions-standalone [body functions idx func-count]
  (do
    (root_push body)
    (root_push functions)
    (let [step (append-code-bodies-functions-standalone-step-512 body functions idx func-count)]
      (do
        (root_push step)
        (let [result
            (if (= (vector-get step 0) 1)
              (vector-get step 2)
              (append-code-bodies-functions-standalone (vector-get step 2) functions (vector-get step 1) func-count))]
          (do
            (root_pop)
            (root_pop)
            (root_pop)
            result))))))

(defn append-code-bodies-functions-progress-debug [body functions idx func-count]
  (do
    (print 576)
    (print idx)
    (root_push body)
    (root_push functions)
    (let [step (append-code-bodies-functions-step-512-progress-debug body functions idx func-count)]
      (do
        (root_push step)
        (print 577)
        (print (vector-get step 1))
        (let [result
            (if (= (vector-get step 0) 1)
              (vector-get step 2)
              (append-code-bodies-functions-progress-debug (vector-get step 2) functions (vector-get step 1) func-count))]
          (do
            (root_pop)
            (root_pop)
            (root_pop)
            result))))))
(defn emit-code-section-functions [functions] (let [func-count (vector-length functions) body0 (emit-leb128 (vector-new 64) func-count) body1 (append-code-bodies-functions body0 functions 0 func-count) body-size (vector-length body1) result0 (emit-byte (vector-new 64) 10) result1 (emit-leb128 result0 body-size)] (append-byte-vector result1 body1 0 body-size)))
(defn emit-code-section-functions-wasi [functions] (let [func-count (vector-length functions) body0 (emit-leb128 (vector-new 64) (+ func-count 1)) body1 (append-code-bodies-functions body0 functions 0 func-count) wrapper0 (emit-byte (vector-new 8) 0) wrapper1 (emit-byte wrapper0 16) wrapper2 (emit-leb128 wrapper1 (- func-count 1)) wrapper3 (emit-byte wrapper2 26) wrapper4 (emit-byte wrapper3 11) wrapper-size (vector-length wrapper4) body2 (emit-leb128 body1 wrapper-size) body3 (append-byte-vector body2 wrapper4 0 wrapper-size) body-size (vector-length body3) result0 (emit-byte (vector-new 64) 10) result1 (emit-leb128 result0 body-size)] (append-byte-vector result1 body3 0 body-size)))
;; 10-import (alloc/print/read-file/command-line-arg/string-concat/substring/file-exists?/root_push/root_pop/root_set) + memory + data モデル用セクション生成関数群
;; タイプセクション: type0=(i64->i64), type1=(i64->void), type2=(i64×2->i64), type3=(i64×3->i64), type4=(()->i64),
;;   type5..(5+N-1)=ユーザ関数型, type5+N=_start型
(defn emit-type-section-wasi-quad-functions [functions] (let [func-count (vector-length functions) total-count (+ func-count 6) body0 (emit-leb128 (vector-new 64) total-count) body1 (emit-byte body0 96) body2 (emit-leb128 body1 1) body3 (emit-byte body2 126) body4 (emit-byte body3 1) body5 (emit-byte body4 126) body6 (emit-byte body5 96) body7 (emit-leb128 body6 1) body8 (emit-byte body7 126) body9 (emit-leb128 body8 0) body10 (emit-byte body9 96) body11 (emit-leb128 body10 2) body12 (emit-byte body11 126) body13 (emit-byte body12 126) body14 (emit-byte body13 1) body15 (emit-byte body14 126) body16 (emit-byte body15 96) body17 (emit-leb128 body16 3) body18 (emit-byte body17 126) body19 (emit-byte body18 126) body20 (emit-byte body19 126) body21 (emit-byte body20 1) body22 (emit-byte body21 126) body23 (emit-byte body22 96) body24 (emit-leb128 body23 0) body25 (emit-byte body24 1) body26 (emit-byte body25 126) body27 (append-function-types body26 functions 0 func-count) body28 (emit-byte body27 96) body29 (emit-leb128 body28 0) body30 (emit-leb128 body29 0) body-size (vector-length body30) result0 (emit-byte (vector-new 64) 1) result1 (emit-leb128 result0 body-size)] (append-byte-vector-chunked result1 body30 0 body-size)))
(defn emit-type-section-wasi-quad-functions-progress-debug [functions]
  (let [func-count (vector-length functions)
    total-count (+ func-count 6)
    body0 (emit-leb128 (vector-new 64) total-count)
    body1 (emit-byte body0 96)
    body2 (emit-leb128 body1 1)
    body3 (emit-byte body2 126)
    body4 (emit-byte body3 1)
    body5 (emit-byte body4 126)
    body6 (emit-byte body5 96)
    body7 (emit-leb128 body6 1)
    body8 (emit-byte body7 126)
    body9 (emit-leb128 body8 0)
    body10 (emit-byte body9 96)
    body11 (emit-leb128 body10 2)
    body12 (emit-byte body11 126)
    body13 (emit-byte body12 126)
    body14 (emit-byte body13 1)
    body15 (emit-byte body14 126)
    body16 (emit-byte body15 96)
    body17 (emit-leb128 body16 3)
    body18 (emit-byte body17 126)
    body19 (emit-byte body18 126)
    body20 (emit-byte body19 126)
    body21 (emit-byte body20 1)
    body22 (emit-byte body21 126)
    body23 (emit-byte body22 96)
    body24 (emit-leb128 body23 0)
    body25 (emit-byte body24 1)
    body26 (emit-byte body25 126)]
    (do
      (print 521)
      (print func-count)
      (let [body27 (append-function-types body26 functions 0 func-count)]
        (do
          (print 522)
          (print (vector-length body27))
          (let [body28 (emit-byte body27 96)
            body29 (emit-leb128 body28 0)
            body30 (emit-leb128 body29 0)
            body-size (vector-length body30)
            result0 (emit-byte (vector-new 64) 1)
            result1 (emit-leb128 result0 body-size)]
            (do
              (print 523)
              (print body-size)
              (let [result (append-byte-vector-progress-debug result1 body30 0 body-size)]
                (do
                  (print 524)
                  (print (vector-length result))
                  result)))))))))
;; 関数セクション: ユーザ関数 type5..(5+N-1) + _start type(5+N)
(defn emit-function-section-wasi-quad-functions [functions] (let [func-count (vector-length functions) body0 (emit-leb128 (vector-new 32) (+ func-count 1)) body1 (append-type-index-sequence body0 5 (+ 5 func-count)) body2 (emit-leb128 body1 (+ 5 func-count)) body-size (vector-length body2) result0 (emit-byte (vector-new 32) 3) result1 (emit-leb128 result0 body-size)] (append-byte-vector result1 body2 0 body-size)))
;; コードセクション: 旧10-import runtime のユーザ関数ボディ + _start
(defn emit-code-section-wasi-quad-functions [functions] (let [func-count (vector-length functions) main-func-idx (+ 9 func-count) body0 (emit-leb128 (vector-new 64) (+ func-count 1)) body1 (append-code-bodies-functions body0 functions 0 func-count) wrapper0 (emit-byte (vector-new 8) 0) wrapper1 (emit-byte wrapper0 16) wrapper2 (emit-leb128 wrapper1 main-func-idx) wrapper3 (emit-byte wrapper2 26) wrapper4 (emit-byte wrapper3 11) wrapper-size (vector-length wrapper4) body2 (emit-leb128 body1 wrapper-size) body3 (append-byte-vector body2 wrapper4 0 wrapper-size) body-size (vector-length body3) result0 (emit-byte (vector-new 64) 10) result1 (emit-leb128 result0 body-size)] (append-byte-vector-chunked result1 body3 0 body-size)))
;; コードセクション: 11-import runtime のユーザ関数ボディ + _start
(defn emit-code-section-wasi-quad-functions-print-string [functions] (let [func-count (vector-length functions) main-func-idx (+ 10 func-count) body0 (emit-leb128 (vector-new 64) (+ func-count 1)) body1 (append-code-bodies-functions body0 functions 0 func-count) wrapper0 (emit-byte (vector-new 8) 0) wrapper1 (emit-byte wrapper0 16) wrapper2 (emit-leb128 wrapper1 main-func-idx) wrapper3 (emit-byte wrapper2 26) wrapper4 (emit-byte wrapper3 11) wrapper-size (vector-length wrapper4) body2 (emit-leb128 body1 wrapper-size) body3 (append-byte-vector body2 wrapper4 0 wrapper-size) body-size (vector-length body3) result0 (emit-byte (vector-new 64) 10) result1 (emit-leb128 result0 body-size)] (append-byte-vector-chunked result1 body3 0 body-size)))
;; 標準 WASI Preview1 の fd_write / args_sizes_get / args_get / path_open / fd_close / fd_read だけを外部 import に残し、旧 env runtime は
;; 同じ関数番号を保った内部 Wasm 関数として配置する standalone ABI。
(defn emit-type-section-wasi-standalone [functions]
  (let [func-count (vector-length functions)
    total-count (+ func-count 11)
    body0 (emit-leb128 (vector-new 96) total-count)
    body1 (emit-standalone-byte-seq-8 body0 96 4 127 127 127 127 1 127)
    body2 (emit-byte (emit-standalone-byte-seq-4 body1 96 1 126 1) 126)
    body3 (emit-standalone-byte-seq-4 body2 96 1 126 0)
    body4 (emit-standalone-byte-seq-6 body3 96 2 126 126 1 126)
    body5 (emit-byte (emit-standalone-byte-seq-6 body4 96 3 126 126 126 1) 126)
    body6 (emit-standalone-byte-seq-4 body5 96 0 1 126)
    body7 (emit-standalone-byte-seq-6 body6 96 2 127 127 1 127)
    body8 (append-function-types body7 functions 0 func-count)
    body9 (emit-byte (emit-standalone-byte-seq-2 body8 96 0) 0)
    body10 (emit-standalone-byte-seq-4 body9 96 9 127 127)
    body11 (emit-standalone-byte-seq-8 body10 127 127 127 126 126 127 127 1)
    body12 (emit-byte body11 127)
    body13 (emit-standalone-byte-seq-4 body12 96 1 127 1)
    body14 (emit-byte body13 127)
    body15 (emit-standalone-byte-seq-4 body14 96 4 127 127)
    body16 (emit-standalone-byte-seq-4 body15 127 127 1 127)
    body-size (vector-length body16)
    result0 (emit-byte (vector-new 96) 1)
    result1 (emit-leb128 result0 body-size)]
    (append-byte-vector-chunked result1 body16 0 body-size)))
(defn emit-function-section-wasi-standalone [functions]
  (let [func-count (vector-length functions)
    body0 (emit-leb128 (vector-new 64) (+ func-count 17))
    body1 (emit-leb128 body0 1)
    body2 (emit-leb128 body1 2)
    body3 (emit-leb128 body2 1)
    body4 (emit-leb128 body3 1)
    body5 (emit-leb128 body4 3)
    body6 (emit-leb128 body5 4)
    body7 (emit-leb128 body6 1)
    body8 (emit-leb128 body7 1)
    body9 (emit-leb128 body8 5)
    body10 (emit-leb128 body9 3)
    body11 (emit-leb128 body10 2)
    body12 (emit-leb128 body11 1)
    body13 (emit-leb128 body12 1)
    body14 (emit-leb128 body13 1)
    body15 (emit-leb128 body14 3)
    body16 (emit-leb128 body15 3)
    body17 (append-type-index-sequence body16 7 (+ 7 func-count))
    body18 (emit-leb128 body17 (+ 7 func-count))
    body-size (vector-length body18)
    result0 (emit-byte (vector-new 64) 3)
    result1 (emit-leb128 result0 body-size)]
    (append-byte-vector-chunked result1 body18 0 body-size)))
(defn emit-code-section-wasi-standalone [functions]
  (let [func-count (vector-length functions)
    main-func-idx (+ 21 func-count)
    body0 (emit-leb128 (vector-new 64) (+ func-count 17))
    body1 (append-standalone-runtime-bodies body0 0)
    body2 (append-code-bodies-functions-standalone body1 functions 0 func-count)
    body3 (append-standalone-code-body body2 (emit-standalone-wrapper-body main-func-idx))
    body-size (vector-length body3)
    result0 (emit-byte (vector-new 64) 10)
    result1 (emit-leb128 result0 body-size)]
    (append-byte-vector-chunked result1 body3 0 body-size)))
(defn emit-code-section-wasi-quad-functions-progress-debug [functions]
  (let [func-count (vector-length functions)
    main-func-idx (+ 10 func-count)
    body0 (emit-leb128 (vector-new 64) (+ func-count 1))]
    (do
      (print 571)
      (print func-count)
      (let [body1 (append-code-bodies-functions-progress-debug body0 functions 0 func-count)]
        (do
          (print 572)
          (print (vector-length body1))
          (let [wrapper0 (emit-byte (vector-new 8) 0)
            wrapper1 (emit-byte wrapper0 16)
            wrapper2 (emit-leb128 wrapper1 main-func-idx)
            wrapper3 (emit-byte wrapper2 26)
            wrapper4 (emit-byte wrapper3 11)
            wrapper-size (vector-length wrapper4)]
            (do
              (print 573)
              (print wrapper-size)
              (let [body2 (emit-leb128 body1 wrapper-size)
                body3 (append-byte-vector body2 wrapper4 0 wrapper-size)
                body-size (vector-length body3)
                result0 (emit-byte (vector-new 64) 10)
                result1 (emit-leb128 result0 body-size)]
                (do
                  (print 574)
                  (print body-size)
                  (let [result (append-byte-vector-progress-debug result1 body3 0 body-size)]
                    (do
                      (print 575)
                      (print (vector-length result))
                      result)))))))))))
(defn emit-code-section [ir-instrs] (emit-code-section-list (vector-push (vector-new 2) ir-instrs)))
(defn emit-wasm [ir-instrs] (let [h (emit-header) t (emit-type-section-main) c (emit-code-section ir-instrs)] (+ (+ (vector-length h) (vector-length t)) (vector-length c))))
(defn emit-wasm-with-target [ir-instrs target] (let [h (emit-header) i (emit-import-section-for-target target) t (emit-type-section-main) c (emit-code-section ir-instrs)] (+ (+ (+ (vector-length h) (vector-length i)) (vector-length t)) (vector-length c))))
(defn emit-tagged-pointer-high-bit [bytes] (let [b1 (emit-byte bytes 66) b2 (emit-byte b1 128) b3 (emit-byte b2 128) b4 (emit-byte b3 128) b5 (emit-byte b4 128) b6 (emit-byte b5 128) b7 (emit-byte b6 128) b8 (emit-byte b7 128) b9 (emit-byte b8 128) b10 (emit-byte b9 128) b11 (emit-byte b10 127)] b11))
;; 10-import セクション: alloc(type0), print(type1), read-file(type0), command-line-arg(type0), string-concat(type2), substring(type3), file-exists?(type0), root_push(type0), root_pop(type4), root_set(type2)
(defn append-import-env-prefix [body] (let [b1 (emit-leb128 body 3) b2 (emit-byte b1 101) b3 (emit-byte b2 110) b4 (emit-byte b3 118)] b4))
(defn append-import-alloc-entry [body] (let [b0 (append-import-env-prefix body) b1 (emit-leb128 b0 7) b2 (emit-byte b1 95) b3 (emit-byte b2 95) b4 (emit-byte b3 97) b5 (emit-byte b4 108) b6 (emit-byte b5 108) b7 (emit-byte b6 111) b8 (emit-byte b7 99) b9 (emit-byte b8 0)] (emit-leb128 b9 0)))
(defn append-import-print-entry [body] (let [b0 (append-import-env-prefix body) b1 (emit-leb128 b0 5) b2 (emit-byte b1 112) b3 (emit-byte b2 114) b4 (emit-byte b3 105) b5 (emit-byte b4 110) b6 (emit-byte b5 116) b7 (emit-byte b6 0)] (emit-leb128 b7 1)))
(defn append-import-read-file-entry [body] (let [b0 (append-import-env-prefix body) b1 (emit-leb128 b0 9) b2 (emit-byte b1 114) b3 (emit-byte b2 101) b4 (emit-byte b3 97) b5 (emit-byte b4 100) b6 (emit-byte b5 45) b7 (emit-byte b6 102) b8 (emit-byte b7 105) b9 (emit-byte b8 108) b10 (emit-byte b9 101) b11 (emit-byte b10 0)] (emit-leb128 b11 0)))
(defn append-import-command-line-arg-entry [body] (let [b0 (append-import-env-prefix body) b1 (emit-leb128 b0 16) b2 (emit-byte b1 99) b3 (emit-byte b2 111) b4 (emit-byte b3 109) b5 (emit-byte b4 109) b6 (emit-byte b5 97) b7 (emit-byte b6 110) b8 (emit-byte b7 100) b9 (emit-byte b8 45) b10 (emit-byte b9 108) b11 (emit-byte b10 105) b12 (emit-byte b11 110) b13 (emit-byte b12 101) b14 (emit-byte b13 45) b15 (emit-byte b14 97) b16 (emit-byte b15 114) b17 (emit-byte b16 103) b18 (emit-byte b17 0)] (emit-leb128 b18 0)))
(defn append-import-string-concat-entry [body] (let [b0 (append-import-env-prefix body) b1 (emit-leb128 b0 13) b2 (emit-byte b1 115) b3 (emit-byte b2 116) b4 (emit-byte b3 114) b5 (emit-byte b4 105) b6 (emit-byte b5 110) b7 (emit-byte b6 103) b8 (emit-byte b7 45) b9 (emit-byte b8 99) b10 (emit-byte b9 111) b11 (emit-byte b10 110) b12 (emit-byte b11 99) b13 (emit-byte b12 97) b14 (emit-byte b13 116) b15 (emit-byte b14 0)] (emit-leb128 b15 2)))
(defn append-import-substring-entry [body] (let [b0 (append-import-env-prefix body) b1 (emit-leb128 b0 9) b2 (emit-byte b1 115) b3 (emit-byte b2 117) b4 (emit-byte b3 98) b5 (emit-byte b4 115) b6 (emit-byte b5 116) b7 (emit-byte b6 114) b8 (emit-byte b7 105) b9 (emit-byte b8 110) b10 (emit-byte b9 103) b11 (emit-byte b10 0)] (emit-leb128 b11 3)))
(defn append-import-file-exists-entry [body] (let [b0 (append-import-env-prefix body) b1 (emit-leb128 b0 12) b2 (emit-byte b1 102) b3 (emit-byte b2 105) b4 (emit-byte b3 108) b5 (emit-byte b4 101) b6 (emit-byte b5 45) b7 (emit-byte b6 101) b8 (emit-byte b7 120) b9 (emit-byte b8 105) b10 (emit-byte b9 115) b11 (emit-byte b10 116) b12 (emit-byte b11 115) b13 (emit-byte b12 63) b14 (emit-byte b13 0)] (emit-leb128 b14 0)))
(defn append-import-root-push-entry [body] (let [b0 (append-import-env-prefix body) b1 (emit-leb128 b0 9) b2 (emit-byte b1 114) b3 (emit-byte b2 111) b4 (emit-byte b3 111) b5 (emit-byte b4 116) b6 (emit-byte b5 95) b7 (emit-byte b6 112) b8 (emit-byte b7 117) b9 (emit-byte b8 115) b10 (emit-byte b9 104) b11 (emit-byte b10 0)] (emit-leb128 b11 0)))
(defn append-import-root-pop-entry [body] (let [b0 (append-import-env-prefix body) b1 (emit-leb128 b0 8) b2 (emit-byte b1 114) b3 (emit-byte b2 111) b4 (emit-byte b3 111) b5 (emit-byte b4 116) b6 (emit-byte b5 95) b7 (emit-byte b6 112) b8 (emit-byte b7 111) b9 (emit-byte b8 112) b10 (emit-byte b9 0)] (emit-leb128 b10 4)))
(defn append-import-root-set-entry [body] (let [b0 (append-import-env-prefix body) b1 (emit-leb128 b0 8) b2 (emit-byte b1 114) b3 (emit-byte b2 111) b4 (emit-byte b3 111) b5 (emit-byte b4 116) b6 (emit-byte b5 95) b7 (emit-byte b6 115) b8 (emit-byte b7 101) b9 (emit-byte b8 116) b10 (emit-byte b9 0)] (emit-leb128 b10 2)))
(defn append-import-print-string-entry [body] (let [b0 (append-import-env-prefix body) b1 (emit-leb128 b0 12) b2 (emit-byte b1 112) b3 (emit-byte b2 114) b4 (emit-byte b3 105) b5 (emit-byte b4 110) b6 (emit-byte b5 116) b7 (emit-byte b6 45) b8 (emit-byte b7 115) b9 (emit-byte b8 116) b10 (emit-byte b9 114) b11 (emit-byte b10 105) b12 (emit-byte b11 110) b13 (emit-byte b12 103) b14 (emit-byte b13 0)] (emit-leb128 b14 1)))
(defn emit-import-section-alloc-print-read-arg-concat-sub [] (let [body0 (emit-leb128 (vector-new 160) 10) body1 (append-import-alloc-entry body0) body2 (append-import-print-entry body1) body3 (append-import-read-file-entry body2) body4 (append-import-command-line-arg-entry body3) body5 (append-import-string-concat-entry body4) body6 (append-import-substring-entry body5) body7 (append-import-file-exists-entry body6) body8 (append-import-root-push-entry body7) body9 (append-import-root-pop-entry body8) body10 (append-import-root-set-entry body9) body-size (vector-length body10) result0 (emit-byte (vector-new 160) 2) result1 (emit-leb128 result0 body-size)] (append-byte-vector result1 body10 0 body-size)))
;; 通常の selfhost Wasm module 用11-import layout。旧10-import helperは bootstrap harness 互換用に残す。
(defn emit-import-section-alloc-print-read-arg-concat-sub-print-string [] (let [body0 (emit-leb128 (vector-new 192) 11) body1 (append-import-alloc-entry body0) body2 (append-import-print-entry body1) body3 (append-import-read-file-entry body2) body4 (append-import-command-line-arg-entry body3) body5 (append-import-string-concat-entry body4) body6 (append-import-substring-entry body5) body7 (append-import-file-exists-entry body6) body8 (append-import-root-push-entry body7) body9 (append-import-root-pop-entry body8) body10 (append-import-root-set-entry body9) body11 (append-import-print-string-entry body10) body-size (vector-length body11) result0 (emit-byte (vector-new 192) 2) result1 (emit-leb128 result0 body-size)] (append-byte-vector result1 body11 0 body-size)))
;; selfhost 10-import レイアウト用エイリアス (harness から呼びやすい短縮名)
(defn emit-import-section-runtime [] (emit-import-section-alloc-print-read-arg-concat-sub))
;; runtime 10-import 用の関数セクション (WASI _start エントリなし)
;; emit-type-section-wasi-quad-functions と対になって使用する (typeIdx は 5 から始まる)
(defn emit-function-section-runtime-functions [functions] (let [func-count (vector-length functions) body0 (emit-leb128 (vector-new 32) func-count) body1 (append-type-index-sequence body0 5 (+ 5 func-count)) body-size (vector-length body1) result0 (emit-byte (vector-new 32) 3) result1 (emit-leb128 result0 body-size)] (append-byte-vector result1 body1 0 body-size)))
(defn emit-string-concat-instr [bytes] (emit-leb128 (emit-byte bytes 16) 4))
(defn emit-substring-instr [bytes] (emit-leb128 (emit-byte bytes 16) 5))
(defn emit-file-exists-instr [bytes] (emit-leb128 (emit-byte bytes 16) 6))
(defn emit-root-push-instr [bytes] (emit-leb128 (emit-byte bytes 16) 7))
(defn emit-root-pop-instr [bytes] (emit-leb128 (emit-byte bytes 16) 8))
(defn emit-root-set-instr [bytes] (emit-leb128 (emit-byte bytes 16) 9))
(defn emit-print-string-instr [bytes] (let [b1 (emit-leb128 (emit-byte bytes 16) 10)] (emit-leb128-s (emit-byte b1 66) 0)))
(defn emit-and-instr [bytes] (emit-byte bytes 131))
(defn emit-or-instr [bytes] (emit-byte bytes 132))
(defn emit-print-instr [bytes] (let [b1 (emit-leb128 (emit-byte bytes 16) 1)] (emit-leb128-s (emit-byte b1 66) 0)))
(defn emit-read-file-instr [bytes] (emit-leb128 (emit-byte bytes 16) 2))
(defn emit-command-line-arg-instr [bytes] (emit-leb128 (emit-byte bytes 16) 3))
(defn emit-runtime-hash-string-instr [bytes] (emit-leb128 (emit-byte bytes 16) 3))
(defn emit-string-char-at-instr [bytes operand] (let [temp-idx (- operand 1) b1 (emit-leb128 (emit-byte bytes 33) temp-idx) b2 (emit-byte b1 167) b3 (emit-leb128 (emit-byte b2 65) 8) b4 (emit-byte b3 106) b5 (emit-leb128 (emit-byte b4 32) temp-idx) b6 (emit-byte b5 167) b7 (emit-byte b6 106) b8 (emit-byte b7 45) b9 (emit-byte b8 0) b10 (emit-byte b9 0)] (emit-byte b10 173)))
(defn emit-vector-push-instr [bytes operand] (let [tagged-idx (- operand 1) val-idx operand len-idx (+ operand 1) cap-idx (+ operand 2) newcap-idx (+ operand 3) newaddr-idx (+ operand 4) b1 (emit-leb128 (emit-byte bytes 33) val-idx) b2 (emit-leb128 (emit-byte b1 33) tagged-idx) b3 (emit-leb128 (emit-byte b2 32) tagged-idx) b4 (emit-byte b3 167) b5 (emit-byte b4 40) b6 (emit-byte b5 0) b7 (emit-byte b6 8) b8 (emit-byte b7 173) b9 (emit-leb128 (emit-byte b8 33) len-idx) b10 (emit-leb128 (emit-byte b9 32) tagged-idx) b11 (emit-byte b10 167) b12 (emit-byte b11 40) b13 (emit-byte b12 0) b14 (emit-byte b13 4) b15 (emit-byte b14 173) b16 (emit-leb128 (emit-byte b15 33) cap-idx) b17 (emit-leb128 (emit-byte b16 32) len-idx) b18 (emit-leb128 (emit-byte b17 32) cap-idx) b19 (emit-byte b18 89) b20 (emit-byte b19 4) b21 (emit-byte b20 126) b22 (emit-leb128 (emit-byte b21 32) cap-idx) b23 (emit-leb128-s (emit-byte b22 66) 2) b24 (emit-byte b23 126) b25 (emit-leb128 (emit-byte b24 33) newcap-idx) b26 (emit-leb128 (emit-byte b25 32) newcap-idx) b27 (emit-leb128-s (emit-byte b26 66) 4) b28 (emit-byte b27 85) b29 (emit-byte b28 4) b30 (emit-byte b29 126) b31 (emit-leb128 (emit-byte b30 32) newcap-idx) b32 (emit-byte b31 5) b33 (emit-leb128-s (emit-byte b32 66) 4) b34 (emit-byte b33 11) b35 (emit-leb128 (emit-byte b34 33) newcap-idx) b36 (emit-leb128-s (emit-byte b35 66) 16) b37 (emit-leb128 (emit-byte b36 32) newcap-idx) b38 (emit-leb128-s (emit-byte b37 66) 8) b39 (emit-byte b38 126) b40 (emit-byte b39 124) b41 (emit-leb128 (emit-byte b40 16) 0) b42 (emit-leb128 (emit-byte b41 33) newaddr-idx) b43 (emit-leb128 (emit-byte b42 32) newaddr-idx) b44 (emit-byte b43 167) b45 (emit-leb128 (emit-byte b44 65) 5) b46 (emit-byte b45 54) b47 (emit-byte b46 0) b48 (emit-byte b47 0) b49 (emit-leb128 (emit-byte b48 32) newaddr-idx) b50 (emit-byte b49 167) b51 (emit-leb128 (emit-byte b50 32) newcap-idx) b52 (emit-byte b51 167) b53 (emit-byte b52 54) b54 (emit-byte b53 0) b55 (emit-byte b54 4) b56 (emit-leb128 (emit-byte b55 32) newaddr-idx) b57 (emit-byte b56 167) b58 (emit-leb128 (emit-byte b57 32) len-idx) b59 (emit-byte b58 167) b60 (emit-leb128 (emit-byte b59 65) 1) b61 (emit-byte b60 106) b62 (emit-byte b61 54) b63 (emit-byte b62 0) b64 (emit-byte b63 8) b65 (emit-leb128 (emit-byte b64 32) newaddr-idx) b66 (emit-byte b65 167) b67 (emit-leb128 (emit-byte b66 65) 0) b68 (emit-byte b67 54) b69 (emit-byte b68 0) b70 (emit-byte b69 12) b71 (emit-leb128 (emit-byte b70 32) newaddr-idx) b72 (emit-byte b71 167) b73 (emit-leb128 (emit-byte b72 65) 16) b74 (emit-byte b73 106) b75 (emit-leb128 (emit-byte b74 32) tagged-idx) b76 (emit-byte b75 167) b77 (emit-leb128 (emit-byte b76 65) 16) b78 (emit-byte b77 106) b79 (emit-leb128 (emit-byte b78 32) len-idx) b80 (emit-byte b79 167) b81 (emit-leb128 (emit-byte b80 65) 8) b82 (emit-byte b81 108) b83 (emit-byte b82 252) b84 (emit-byte b83 10) b85 (emit-byte b84 0) b86 (emit-byte b85 0) b87 (emit-leb128 (emit-byte b86 32) newaddr-idx) b88 (emit-byte b87 167) b89 (emit-leb128 (emit-byte b88 32) len-idx) b90 (emit-byte b89 167) b91 (emit-leb128 (emit-byte b90 65) 8) b92 (emit-byte b91 108) b93 (emit-leb128 (emit-byte b92 65) 16) b94 (emit-byte b93 106) b95 (emit-byte b94 106) b96 (emit-leb128 (emit-byte b95 32) val-idx) b97 (emit-byte b96 55) b98 (emit-byte b97 0) b99 (emit-byte b98 0) b100 (emit-leb128 (emit-byte b99 32) newaddr-idx) b101 (emit-tagged-pointer-high-bit b100) b102 (emit-byte b101 124) b103 (emit-byte b102 5) b104 (emit-leb128 (emit-byte b103 32) tagged-idx) b105 (emit-byte b104 167) b106 (emit-leb128 (emit-byte b105 32) len-idx) b107 (emit-byte b106 167) b108 (emit-leb128 (emit-byte b107 65) 8) b109 (emit-byte b108 108) b110 (emit-leb128 (emit-byte b109 65) 16) b111 (emit-byte b110 106) b112 (emit-byte b111 106) b113 (emit-leb128 (emit-byte b112 32) val-idx) b114 (emit-byte b113 55) b115 (emit-byte b114 0) b116 (emit-byte b115 0) b117 (emit-leb128 (emit-byte b116 32) tagged-idx) b118 (emit-byte b117 167) b119 (emit-leb128 (emit-byte b118 32) len-idx) b120 (emit-byte b119 167) b121 (emit-leb128 (emit-byte b120 65) 1) b122 (emit-byte b121 106) b123 (emit-byte b122 54) b124 (emit-byte b123 0) b125 (emit-byte b124 8) b126 (emit-leb128 (emit-byte b125 32) tagged-idx) b127 (emit-byte b126 11)] b127))
(defn emit-vector-get-instr [bytes operand] (let [temp-idx (- operand 1) b1 (emit-leb128 (emit-byte bytes 33) temp-idx) b2 (emit-byte b1 167) b3 (emit-leb128 (emit-byte b2 32) temp-idx) b4 (emit-byte b3 167) b5 (emit-leb128 (emit-byte b4 65) 8) b6 (emit-byte b5 108) b7 (emit-leb128 (emit-byte b6 65) 16) b8 (emit-byte b7 106) b9 (emit-byte b8 106) b10 (emit-byte b9 41) b11 (emit-byte b10 0) b12 (emit-byte b11 0)] b12))
(defn emit-ref-new-instr [bytes operand] (let [val-idx (- operand 1) addr-idx operand b1 (emit-leb128 (emit-byte bytes 33) val-idx) b2 (emit-leb128-s (emit-byte b1 66) 16) b3 (emit-leb128 (emit-byte b2 16) 0) b4 (emit-leb128 (emit-byte b3 33) addr-idx) b5 (emit-leb128 (emit-byte b4 32) addr-idx) b6 (emit-byte b5 167) b7 (emit-leb128 (emit-byte b6 65) 7) b8 (emit-byte b7 54) b9 (emit-byte b8 0) b10 (emit-byte b9 0) b11 (emit-leb128 (emit-byte b10 32) addr-idx) b12 (emit-byte b11 167) b13 (emit-leb128 (emit-byte b12 65) 16) b14 (emit-byte b13 54) b15 (emit-byte b14 0) b16 (emit-byte b15 4) b17 (emit-leb128 (emit-byte b16 32) addr-idx) b18 (emit-byte b17 167) b19 (emit-leb128 (emit-byte b18 32) val-idx) b20 (emit-byte b19 55) b21 (emit-byte b20 0) b22 (emit-byte b21 8) b23 (emit-leb128 (emit-byte b22 32) addr-idx) b24 (emit-tagged-pointer-high-bit b23)] (emit-byte b24 124)))
(defn emit-ref-set-instr [bytes operand] (let [val-idx (- operand 1) b1 (emit-leb128 (emit-byte bytes 33) val-idx) b2 (emit-byte b1 167) b3 (emit-leb128 (emit-byte b2 32) val-idx) b4 (emit-byte b3 55) b5 (emit-byte b4 0) b6 (emit-byte b5 8)] (emit-leb128-s (emit-byte b6 66) 0)))
(defn emit-block-empty [bytes] (emit-byte (emit-byte bytes 2) 64))
(defn emit-loop-empty [bytes] (emit-byte (emit-byte bytes 3) 64))
(defn emit-br [bytes depth] (emit-leb128 (emit-byte bytes 12) depth))
(defn emit-br-if [bytes depth] (emit-leb128 (emit-byte bytes 13) depth))
(defn emit-memory-copy [bytes] (let [b1 (emit-byte bytes 252) b2 (emit-byte b1 10) b3 (emit-byte b2 0) b4 (emit-byte b3 0)] b4))
(defn emit-memory-fill [bytes] (let [b1 (emit-byte bytes 252) b2 (emit-byte b1 11) b3 (emit-byte b2 0)] b3))
(defn emit-map-new-instr [bytes operand] (let [addr-idx (- operand 1) b1 (emit-leb128-s (emit-byte bytes 66) 65552) b2 (emit-leb128 (emit-byte b1 16) 0) b3 (emit-leb128 (emit-byte b2 33) addr-idx) b4 (emit-leb128 (emit-byte b3 32) addr-idx) b5 (emit-byte b4 167) b6 (emit-leb128 (emit-byte b5 65) 6) b7 (emit-byte b6 54) b8 (emit-byte b7 0) b9 (emit-byte b8 0) b10 (emit-leb128 (emit-byte b9 32) addr-idx) b11 (emit-byte b10 167) b12 (emit-leb128 (emit-byte b11 65) 4096) b13 (emit-byte b12 54) b14 (emit-byte b13 0) b15 (emit-byte b14 4) b16 (emit-leb128 (emit-byte b15 32) addr-idx) b17 (emit-byte b16 167) b18 (emit-leb128 (emit-byte b17 65) 0) b19 (emit-byte b18 54) b20 (emit-byte b19 0) b21 (emit-byte b20 8) b22 (emit-leb128 (emit-byte b21 32) addr-idx) b23 (emit-byte b22 167) b24 (emit-leb128 (emit-byte b23 65) 16) b25 (emit-byte b24 106) b26 (emit-leb128 (emit-byte b25 65) 0) b27 (emit-leb128 (emit-byte b26 65) 65536) b28 (emit-memory-fill b27) b29 (emit-leb128 (emit-byte b28 32) addr-idx) b30 (emit-tagged-pointer-high-bit b29)] (emit-byte b30 124)))
(defn emit-map-insert-instr [bytes operand] (let [tagged-idx (- operand 1) key-idx operand val-idx (+ operand 1) cap-idx (+ operand 2) i-idx (+ operand 3) ea-idx (+ operand 4) b1 (emit-leb128 (emit-byte bytes 33) val-idx) b2 (emit-leb128 (emit-byte b1 33) key-idx) b3 (emit-leb128 (emit-byte b2 33) tagged-idx) b4 (emit-leb128 (emit-byte b3 32) tagged-idx) b5 (emit-byte b4 167) b6 (emit-byte b5 40) b7 (emit-byte b6 0) b8 (emit-byte b7 4) b9 (emit-byte b8 173) b10 (emit-leb128 (emit-byte b9 33) cap-idx) b11 (emit-leb128-s (emit-byte b10 66) 0) b12 (emit-leb128 (emit-byte b11 33) i-idx) b13 (emit-block-empty b12) b14 (emit-loop-empty b13) b15 (emit-leb128 (emit-byte b14 32) i-idx) b16 (emit-leb128 (emit-byte b15 32) cap-idx) b17 (emit-byte b16 89) b18 (emit-br-if b17 1) b19 (emit-leb128 (emit-byte b18 32) tagged-idx) b20 (emit-byte b19 167) b21 (emit-byte b20 173) b22 (emit-leb128-s (emit-byte b21 66) 16) b23 (emit-byte b22 124) b24 (emit-leb128 (emit-byte b23 32) i-idx) b25 (emit-leb128-s (emit-byte b24 66) 16) b26 (emit-byte b25 126) b27 (emit-byte b26 124) b28 (emit-leb128 (emit-byte b27 33) ea-idx) b29 (emit-leb128 (emit-byte b28 32) ea-idx) b30 (emit-byte b29 167) b31 (emit-byte b30 41) b32 (emit-byte b31 0) b33 (emit-byte b32 0) b34 (emit-leb128-s (emit-byte b33 66) 0) b35 (emit-byte b34 81) b36 (emit-byte (emit-byte b35 4) 64) b37 (emit-leb128 (emit-byte b36 32) ea-idx) b38 (emit-byte b37 167) b39 (emit-leb128 (emit-byte b38 32) key-idx) b40 (emit-byte b39 55) b41 (emit-byte b40 0) b42 (emit-byte b41 0) b43 (emit-leb128 (emit-byte b42 32) ea-idx) b44 (emit-byte b43 167) b45 (emit-leb128 (emit-byte b44 32) val-idx) b46 (emit-byte b45 55) b47 (emit-byte b46 0) b48 (emit-byte b47 8) b49 (emit-leb128 (emit-byte b48 32) tagged-idx) b50 (emit-byte b49 167) b51 (emit-leb128 (emit-byte b50 32) tagged-idx) b52 (emit-byte b51 167) b53 (emit-byte b52 40) b54 (emit-byte b53 0) b55 (emit-byte b54 8) b56 (emit-leb128 (emit-byte b55 65) 1) b57 (emit-byte b56 106) b58 (emit-byte b57 54) b59 (emit-byte b58 0) b60 (emit-byte b59 8) b61 (emit-br b60 2) b62 (emit-byte b61 11) b63 (emit-leb128 (emit-byte b62 32) ea-idx) b64 (emit-byte b63 167) b65 (emit-byte b64 41) b66 (emit-byte b65 0) b67 (emit-byte b66 0) b68 (emit-leb128 (emit-byte b67 32) key-idx) b69 (emit-byte b68 81) b70 (emit-byte (emit-byte b69 4) 64) b71 (emit-leb128 (emit-byte b70 32) ea-idx) b72 (emit-byte b71 167) b73 (emit-leb128 (emit-byte b72 32) val-idx) b74 (emit-byte b73 55) b75 (emit-byte b74 0) b76 (emit-byte b75 8) b77 (emit-br b76 2) b78 (emit-byte b77 11) b79 (emit-leb128 (emit-byte b78 32) i-idx) b80 (emit-leb128-s (emit-byte b79 66) 1) b81 (emit-byte b80 124) b82 (emit-leb128 (emit-byte b81 33) i-idx) b83 (emit-br b82 0) b84 (emit-byte b83 11) b85 (emit-byte b84 11) b86 (emit-leb128 (emit-byte b85 32) tagged-idx)] b86))
(defn emit-map-get-instr [bytes operand] (let [tagged-idx (- operand 1) key-idx operand cap-idx (+ operand 1) result-idx (+ operand 2) i-idx (+ operand 3) ea-idx (+ operand 4) b1 (emit-leb128 (emit-byte bytes 33) key-idx) b2 (emit-leb128 (emit-byte b1 33) tagged-idx) b3 (emit-leb128 (emit-byte b2 32) tagged-idx) b4 (emit-byte b3 167) b5 (emit-byte b4 40) b6 (emit-byte b5 0) b7 (emit-byte b6 4) b8 (emit-byte b7 173) b9 (emit-leb128 (emit-byte b8 33) cap-idx) b10 (emit-leb128-s (emit-byte b9 66) 0) b11 (emit-leb128 (emit-byte b10 33) result-idx) b12 (emit-leb128-s (emit-byte b11 66) 0) b13 (emit-leb128 (emit-byte b12 33) i-idx) b14 (emit-block-empty b13) b15 (emit-loop-empty b14) b16 (emit-leb128 (emit-byte b15 32) i-idx) b17 (emit-leb128 (emit-byte b16 32) cap-idx) b18 (emit-byte b17 89) b19 (emit-br-if b18 1) b20 (emit-leb128 (emit-byte b19 32) tagged-idx) b21 (emit-byte b20 167) b22 (emit-byte b21 173) b23 (emit-leb128-s (emit-byte b22 66) 16) b24 (emit-byte b23 124) b25 (emit-leb128 (emit-byte b24 32) i-idx) b26 (emit-leb128-s (emit-byte b25 66) 16) b27 (emit-byte b26 126) b28 (emit-byte b27 124) b29 (emit-leb128 (emit-byte b28 33) ea-idx) b30 (emit-leb128 (emit-byte b29 32) ea-idx) b31 (emit-byte b30 167) b32 (emit-byte b31 41) b33 (emit-byte b32 0) b34 (emit-byte b33 0) b35 (emit-leb128 (emit-byte b34 32) key-idx) b36 (emit-byte b35 81) b37 (emit-byte (emit-byte b36 4) 64) b38 (emit-leb128 (emit-byte b37 32) ea-idx) b39 (emit-byte b38 167) b40 (emit-byte b39 41) b41 (emit-byte b40 0) b42 (emit-byte b41 8) b43 (emit-leb128 (emit-byte b42 33) result-idx) b44 (emit-br b43 2) b45 (emit-byte b44 11) b46 (emit-leb128 (emit-byte b45 32) i-idx) b47 (emit-leb128-s (emit-byte b46 66) 1) b48 (emit-byte b47 124) b49 (emit-leb128 (emit-byte b48 33) i-idx) b50 (emit-br b49 0) b51 (emit-byte b50 11) b52 (emit-byte b51 11) b53 (emit-leb128 (emit-byte b52 32) result-idx)] b53))
(defn emit-map-contains-instr [bytes operand] (let [tagged-idx (- operand 1) key-idx operand cap-idx (+ operand 1) result-idx (+ operand 2) i-idx (+ operand 3) ea-idx (+ operand 4) b1 (emit-leb128 (emit-byte bytes 33) key-idx) b2 (emit-leb128 (emit-byte b1 33) tagged-idx) b3 (emit-leb128 (emit-byte b2 32) tagged-idx) b4 (emit-byte b3 167) b5 (emit-byte b4 40) b6 (emit-byte b5 0) b7 (emit-byte b6 4) b8 (emit-byte b7 173) b9 (emit-leb128 (emit-byte b8 33) cap-idx) b10 (emit-leb128-s (emit-byte b9 66) 0) b11 (emit-leb128 (emit-byte b10 33) result-idx) b12 (emit-leb128-s (emit-byte b11 66) 0) b13 (emit-leb128 (emit-byte b12 33) i-idx) b14 (emit-block-empty b13) b15 (emit-loop-empty b14) b16 (emit-leb128 (emit-byte b15 32) i-idx) b17 (emit-leb128 (emit-byte b16 32) cap-idx) b18 (emit-byte b17 89) b19 (emit-br-if b18 1) b20 (emit-leb128 (emit-byte b19 32) tagged-idx) b21 (emit-byte b20 167) b22 (emit-byte b21 173) b23 (emit-leb128-s (emit-byte b22 66) 16) b24 (emit-byte b23 124) b25 (emit-leb128 (emit-byte b24 32) i-idx) b26 (emit-leb128-s (emit-byte b25 66) 16) b27 (emit-byte b26 126) b28 (emit-byte b27 124) b29 (emit-leb128 (emit-byte b28 33) ea-idx) b30 (emit-leb128 (emit-byte b29 32) ea-idx) b31 (emit-byte b30 167) b32 (emit-byte b31 41) b33 (emit-byte b32 0) b34 (emit-byte b33 0) b35 (emit-leb128 (emit-byte b34 32) key-idx) b36 (emit-byte b35 81) b37 (emit-byte (emit-byte b36 4) 64) b38 (emit-leb128-s (emit-byte b37 66) 1) b39 (emit-leb128 (emit-byte b38 33) result-idx) b40 (emit-br b39 2) b41 (emit-byte b40 11) b42 (emit-leb128 (emit-byte b41 32) i-idx) b43 (emit-leb128-s (emit-byte b42 66) 1) b44 (emit-byte b43 124) b45 (emit-leb128 (emit-byte b44 33) i-idx) b46 (emit-br b45 0) b47 (emit-byte b46 11) b48 (emit-byte b47 11) b49 (emit-leb128 (emit-byte b48 32) result-idx)] b49))
(defn emit-map-remove-instr [bytes operand] (let [tagged-idx (- operand 1) key-idx operand cap-idx (+ operand 1) i-idx (+ operand 2) ea-idx (+ operand 3) ek-idx (+ operand 4) b1 (emit-leb128 (emit-byte bytes 33) key-idx) b2 (emit-leb128 (emit-byte b1 33) tagged-idx) b3 (emit-leb128 (emit-byte b2 32) tagged-idx) b4 (emit-byte b3 167) b5 (emit-byte b4 40) b6 (emit-byte b5 0) b7 (emit-byte b6 4) b8 (emit-byte b7 173) b9 (emit-leb128 (emit-byte b8 33) cap-idx) b10 (emit-leb128-s (emit-byte b9 66) 0) b11 (emit-leb128 (emit-byte b10 33) i-idx) b12 (emit-block-empty b11) b13 (emit-loop-empty b12) b14 (emit-leb128 (emit-byte b13 32) i-idx) b15 (emit-leb128 (emit-byte b14 32) cap-idx) b16 (emit-byte b15 89) b17 (emit-br-if b16 1) b18 (emit-leb128 (emit-byte b17 32) tagged-idx) b19 (emit-byte b18 167) b20 (emit-byte b19 173) b21 (emit-leb128-s (emit-byte b20 66) 16) b22 (emit-byte b21 124) b23 (emit-leb128 (emit-byte b22 32) i-idx) b24 (emit-leb128-s (emit-byte b23 66) 16) b25 (emit-byte b24 126) b26 (emit-byte b25 124) b27 (emit-leb128 (emit-byte b26 33) ea-idx) b28 (emit-leb128 (emit-byte b27 32) ea-idx) b29 (emit-byte b28 167) b30 (emit-byte b29 41) b31 (emit-byte b30 0) b32 (emit-byte b31 0) b33 (emit-leb128 (emit-byte b32 33) ek-idx) b34 (emit-leb128 (emit-byte b33 32) ek-idx) b35 (emit-leb128 (emit-byte b34 32) key-idx) b36 (emit-byte b35 81) b37 (emit-byte (emit-byte b36 4) 64) b38 (emit-leb128 (emit-byte b37 32) ea-idx) b39 (emit-byte b38 167) b40 (emit-leb128-s (emit-byte b39 66) 0) b41 (emit-byte b40 55) b42 (emit-byte b41 0) b43 (emit-byte b42 0) b44 (emit-leb128 (emit-byte b43 32) ea-idx) b45 (emit-byte b44 167) b46 (emit-leb128-s (emit-byte b45 66) 0) b47 (emit-byte b46 55) b48 (emit-byte b47 0) b49 (emit-byte b48 8) b50 (emit-leb128 (emit-byte b49 32) tagged-idx) b51 (emit-byte b50 167) b52 (emit-leb128 (emit-byte b51 32) tagged-idx) b53 (emit-byte b52 167) b54 (emit-byte b53 40) b55 (emit-byte b54 0) b56 (emit-byte b55 8) b57 (emit-leb128 (emit-byte b56 65) 1) b58 (emit-byte b57 107) b59 (emit-byte b58 54) b60 (emit-byte b59 0) b61 (emit-byte b60 8) b62 (emit-br b61 2) b63 (emit-byte b62 11) b64 (emit-leb128 (emit-byte b63 32) i-idx) b65 (emit-leb128-s (emit-byte b64 66) 1) b66 (emit-byte b65 124) b67 (emit-leb128 (emit-byte b66 33) i-idx) b68 (emit-br b67 0) b69 (emit-byte b68 11) b70 (emit-byte b69 11) b71 (emit-leb128 (emit-byte b70 32) tagged-idx)] b71))
(defn emit-runtime-ir-instr-tail [bytes opcode operand]
  (if (= opcode 67)
    (emit-runtime-ir-instr-tail-low bytes opcode operand)
    (if (= opcode 68)
      (emit-runtime-ir-instr-tail-low bytes opcode operand)
      (if (= opcode 69)
        (emit-runtime-ir-instr-tail-low bytes opcode operand)
        (if (= opcode 70)
          (emit-runtime-ir-instr-tail-low bytes opcode operand)
          (if (= opcode 71)
            (emit-runtime-ir-instr-tail-low bytes opcode operand)
            (emit-runtime-ir-instr-tail-high bytes opcode operand)))))))

(defn emit-runtime-ir-instr [bytes opcode operand]
  (if (= opcode 59)
    (emit-print-instr bytes)
    (if (= opcode 60)
      (emit-map-new-instr bytes operand)
      (if (= opcode 61)
        (let [b1 (emit-byte bytes 167) b2 (emit-byte b1 40) b3 (emit-byte b2 0) b4 (emit-byte b3 8)]
          (emit-byte b4 173))
        (if (= opcode 62)
          (emit-map-insert-instr bytes operand)
          (if (= opcode 63)
            (emit-map-get-instr bytes operand)
            (if (= opcode 64)
              (emit-read-file-instr bytes)
              (if (= opcode 65)
                (emit-map-contains-instr bytes operand)
                (if (= opcode 66)
                  (emit-map-remove-instr bytes operand)
                  (emit-runtime-ir-instr-tail bytes opcode operand))))))))))

(defn emit-ir-instr [bytes opcode operand]
  (if (= opcode 1)
    (emit-leb128-s (emit-byte bytes 66) operand)
    (if (= opcode 42)
      (emit-block-empty bytes)
      (if (= opcode 82)
        (emit-loop-empty bytes)
        (if (= opcode 10)
          (emit-leb128 (emit-byte bytes 32) (- operand 1))
          (if (= opcode 11)
            (emit-leb128 (emit-byte bytes 33) (- operand 1))
            (if (= opcode 80)
              (emit-br bytes operand)
              (if (= opcode 81)
                (emit-br-if bytes operand)
                (if (= opcode 83)
                  (emit-byte (emit-byte (emit-byte bytes 167) 4) 64)
                  (emit-ir-instr-basic bytes opcode operand))))))))))
(defn emit-data-section [data-bytes offset]
  (do
    (root_push data-bytes)
    (let [data-len (vector-length data-bytes)
      body0 (emit-byte (vector-new 64) 1)]
      (do
        (root_push body0)
        (let [body1 (emit-byte body0 0)]
          (do
            (root_push body1)
            (let [body2 (emit-byte body1 65)]
              (do
                (root_push body2)
                (let [body3 (emit-leb128 body2 offset)]
                  (do
                    (root_push body3)
                    (let [body4 (emit-byte body3 11)]
                      (do
                        (root_push body4)
                        (let [body5 (emit-leb128 body4 data-len)]
                          (do
                            (root_push body5)
                            (let [body-vec (append-byte-vector-chunked body5 data-bytes 0 data-len)
                              body-size (vector-length body-vec)
                              result0 (emit-byte (vector-new 64) 11)]
                              (do
                                (root_push body-vec)
                                (root_push result0)
                                (let [result1 (emit-leb128 result0 body-size)]
                                  (do
                                    (root_push result1)
                                    (let [result (append-byte-vector-chunked result1 body-vec 0 body-size)]
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
                                        result))))))))))))))))))))

(defn emit-ir-instr-basic [bytes opcode operand]
  (if (= opcode 20)
    (emit-ir-instr-basic-low bytes opcode operand)
    (if (= opcode 21)
      (emit-ir-instr-basic-low bytes opcode operand)
      (if (= opcode 22)
        (emit-ir-instr-basic-low bytes opcode operand)
        (if (= opcode 23)
          (emit-ir-instr-basic-low bytes opcode operand)
          (if (= opcode 28)
            (emit-ir-instr-basic-low bytes opcode operand)
            (emit-ir-instr-basic-high bytes opcode operand)))))))

(defn emit-ir-instr-tail [bytes opcode operand]
  (if (= opcode 40)
    (emit-leb128 (emit-byte bytes 16) operand)
    (if (= opcode 41)
      (emit-byte (emit-byte (emit-byte bytes 167) 4) 126)
      (if (= opcode 43)
        (emit-byte bytes 11)
        (if (= opcode 44)
          (emit-byte bytes 26)
          (emit-ir-instr-complex bytes opcode operand))))))

(defn emit-ir-instr-complex [bytes opcode operand]
  (if (= opcode 50)
    (emit-ir-instr-complex-low bytes opcode operand)
    (if (= opcode 51)
      (emit-ir-instr-complex-low bytes opcode operand)
      (if (= opcode 52)
        (emit-ir-instr-complex-low bytes opcode operand)
        (if (= opcode 53)
          (emit-ir-instr-complex-low bytes opcode operand)
          (emit-ir-instr-complex-high bytes opcode operand))))))

(defn emit-runtime-ir-instr-tail-low [bytes opcode operand]
  (if (= opcode 67)
    (emit-command-line-arg-instr bytes)
    (if (= opcode 68)
      (emit-runtime-hash-string-instr bytes)
      (if (= opcode 69)
        (emit-substring-instr bytes)
        (if (= opcode 70)
          (emit-string-concat-instr bytes)
          (if (= opcode 71)
            (emit-and-instr bytes)
            bytes))))))

;; args_sizes_get の import index は runtime call の再配置前に 11 を sentinel として保持する。
;; argc/argv_buf_size scratch は root stack 外の 2272/2276 に置く。
(defn emit-command-line-args-standalone-instr [bytes]
  (let [b1 (emit-leb128-s (emit-byte bytes 65) 2272)
    b2 (emit-leb128-s (emit-byte b1 65) 2276)
    b3 (emit-leb128 (emit-byte b2 16) 11)
    b4 (emit-byte b3 26)
    b5 (emit-leb128-s (emit-byte b4 65) 2272)
    b6 (emit-standalone-byte-seq-4 b5 40 2 0 173)]
    b6))
(defn emit-command-line-arg-standalone-instr [bytes]
  (emit-leb128 (emit-byte bytes 16) 13))
(defn emit-file-exists-standalone-instr [bytes]
  (emit-leb128 (emit-byte bytes 16) 18))
(defn emit-read-file-standalone-instr [bytes]
  (emit-leb128 (emit-byte bytes 16) 19))
(defn emit-write-file-standalone-instr [bytes]
  (emit-leb128 (emit-byte bytes 16) 20))
(defn emit-write-file-bytes-standalone-instr [bytes]
  (emit-leb128 (emit-byte bytes 16) 21))

(defn reject-native-only-wasm-opcode [bytes opcode]
  (if (= opcode 86)
    (do (/ opcode 0) bytes)
    (if (= opcode 88)
      (do (/ opcode 0) bytes)
      bytes)))

(defn emit-runtime-ir-instr-tail-high-final [bytes opcode operand]
  (if (= opcode 87)
    (emit-print-string-instr bytes)
    (if (= opcode (standalone-command-line-args-opcode))
      (emit-command-line-args-standalone-instr bytes)
      (if (= opcode (standalone-command-line-arg-opcode))
        (emit-command-line-arg-standalone-instr bytes)
        (if (= opcode (standalone-file-exists-opcode))
          (emit-file-exists-standalone-instr bytes)
          (if (= opcode (standalone-read-file-opcode))
            (emit-read-file-standalone-instr bytes)
            (if (= opcode (standalone-write-file-opcode))
              (emit-write-file-standalone-instr bytes)
              (if (= opcode (standalone-write-file-bytes-opcode))
                (emit-write-file-bytes-standalone-instr bytes)
                (reject-native-only-wasm-opcode bytes opcode)))))))))

(defn emit-runtime-ir-instr-tail-high [bytes opcode operand]
  (if (= opcode 72)
    (emit-or-instr bytes)
    (if (= opcode 73)
      (emit-file-exists-instr bytes)
      (if (= opcode 74)
        (emit-root-push-instr bytes)
        (if (= opcode 75)
          (emit-root-pop-instr bytes)
          (if (= opcode 76)
            (emit-root-set-instr bytes)
            (emit-runtime-ir-instr-tail-high-final bytes opcode operand)))))))

(defn emit-ir-instr-basic-low [bytes opcode operand]
  (if (= opcode 20)
    (emit-byte bytes 124)
    (if (= opcode 21)
      (emit-byte bytes 125)
      (if (= opcode 22)
        (emit-byte bytes 126)
        (if (= opcode 23)
          (emit-byte bytes 127)
          (if (= opcode 28)
            (emit-byte bytes 129)
            (emit-ir-instr-basic-high bytes opcode operand)))))))

(defn emit-ir-instr-basic-high [bytes opcode operand]
  (if (= opcode 30)
    (emit-byte (emit-byte bytes 81) 172)
    (if (= opcode 31)
      (emit-byte (emit-byte bytes 82) 172)
      (if (= opcode 32)
        (emit-byte (emit-byte bytes 83) 172)
        (if (= opcode 33)
          (emit-byte (emit-byte bytes 85) 172)
          (if (= opcode 34)
            (emit-byte (emit-byte bytes 87) 172)
            (if (= opcode 35)
              (emit-byte (emit-byte bytes 89) 172)
              (emit-ir-instr-tail bytes opcode operand))))))))

(defn emit-ir-instr-complex-low [bytes opcode operand]
  (if (= opcode 50)
    (emit-string-char-at-instr bytes operand)
    (if (= opcode 51)
      (let [b1 (emit-byte bytes 167)
        b2 (emit-byte b1 40)
        b3 (emit-byte b2 0)
        b4 (emit-byte b3 4)]
        (emit-byte b4 173))
      (if (= opcode 52)
        (let [b1 (emit-byte bytes 167)
          b2 (emit-byte b1 40)
          b3 (emit-byte b2 0)
          b4 (emit-byte b3 8)]
          (emit-byte b4 173))
        (if (= opcode 53)
          (emit-vector-get-instr bytes operand)
          (emit-ir-instr-complex-high bytes opcode operand))))))

(defn emit-ir-instr-complex-high [bytes opcode operand]
  (if (= opcode 54)
    (let [cap-idx (- operand 1)
      addr-idx operand
      b1 (emit-leb128 (emit-byte bytes 33) cap-idx)
      b2 (emit-leb128-s (emit-byte b1 66) 16)
      b3 (emit-leb128 (emit-byte b2 32) cap-idx)
      b4 (emit-leb128-s (emit-byte b3 66) 8)
      b5 (emit-byte b4 126)
      b6 (emit-byte b5 124)
      b7 (emit-leb128 (emit-byte b6 16) 0)
      b8 (emit-leb128 (emit-byte b7 33) addr-idx)
      b9 (emit-leb128 (emit-byte b8 32) addr-idx)
      b10 (emit-byte b9 167)
      b11 (emit-leb128 (emit-byte b10 65) 5)
      b12 (emit-byte b11 54)
      b13 (emit-byte b12 0)
      b14 (emit-byte b13 0)
      b15 (emit-leb128 (emit-byte b14 32) addr-idx)
      b16 (emit-byte b15 167)
      b17 (emit-leb128 (emit-byte b16 32) cap-idx)
      b18 (emit-byte b17 167)
      b19 (emit-byte b18 54)
      b20 (emit-byte b19 0)
      b21 (emit-byte b20 4)
      b22 (emit-leb128 (emit-byte b21 32) addr-idx)
      b23 (emit-byte b22 167)
      b24 (emit-leb128 (emit-byte b23 65) 0)
      b25 (emit-byte b24 54)
      b26 (emit-byte b25 0)
      b27 (emit-byte b26 8)
      b28 (emit-leb128 (emit-byte b27 32) addr-idx)
      b29 (emit-byte b28 167)
      b30 (emit-leb128 (emit-byte b29 65) 0)
      b31 (emit-byte b30 54)
      b32 (emit-byte b31 0)
      b33 (emit-byte b32 12)
      b34 (emit-leb128 (emit-byte b33 32) addr-idx)
      b35 (emit-byte b34 66)
      b36 (emit-byte b35 128)
      b37 (emit-byte b36 128)
      b38 (emit-byte b37 128)
      b39 (emit-byte b38 128)
      b40 (emit-byte b39 128)
      b41 (emit-byte b40 128)
      b42 (emit-byte b41 128)
      b43 (emit-byte b42 128)
      b44 (emit-byte b43 128)
      b45 (emit-byte b44 127)]
      (emit-byte b45 124))
    (if (= opcode 55)
      (emit-vector-push-instr bytes operand)
      (if (= opcode 56)
        (emit-ref-new-instr bytes operand)
          (if (= opcode 57)
            (let [b1 (emit-byte bytes 167)
              b2 (emit-byte b1 41)
              b3 (emit-byte b2 0)
              b4 (emit-byte b3 8)]
              b4)
            (if (= opcode 58)
              (emit-ref-set-instr bytes operand)
              (if (= opcode 77)
                (emit-memory-copy bytes)
                (if (= opcode 78)
                  (emit-memory-fill bytes)
                  (emit-runtime-ir-instr bytes opcode operand)))))))))

(defn main [] (let [header (emit-header) type-sec (emit-type-section-main) leb5 (leb128-u 5) leb300 (leb128-u 300) sleb-pos (leb128-s 5) sleb-neg1 (leb128-s -1) sleb-neg128 (leb128-s -128)] (do (print (vector-length header)) (print (vector-get header 0)) (print (vector-get header 1)) (print (vector-get header 2)) (print (vector-get header 3)) (print (vector-get header 4)) (print (vector-length type-sec)) (print (vector-get type-sec 0)) (print (vector-get type-sec 1)) (print (vector-get type-sec 2)) (print (vector-get type-sec 3)) (print (vector-get leb5 0)) (print (vector-get leb300 0)) (print (vector-get leb300 1)) (print (vector-get sleb-pos 0)) (print (vector-length sleb-neg1)) (print (vector-get sleb-neg1 0)) 0)))
