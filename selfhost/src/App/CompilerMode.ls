(module App.CompilerMode)
(import App.ModuleResolver)
(import Syntax.Parser)
(import Syntax.Lexer)
(import Backend.Wasm.WasmEmit)
(import Backend.Wasm.CompilerBase)
(import Backend.Wasm.CompilerSplit)
(import Backend.Wasm.Compiler)
(defn find-first-defn-index [decls idx n]
  (if (>= idx n)
    -1
    (if (= (vector-get (vector-get decls idx) 0) 20)
      idx
      (find-first-defn-index decls (+ idx 1) n))))
(defn find-defn-index-by-hash [decls idx len target-hash]
  (if (>= idx len)
    -1
    (let [decl (vector-get decls idx)]
      (if (= (vector-get decl 0) 20)
        (if (= (vector-get decl 1) target-hash)
          idx
          (find-defn-index-by-hash decls (+ idx 1) len target-hash))
        (find-defn-index-by-hash decls (+ idx 1) len target-hash)))))
(defn decl-tag-or-minus-one [decls idx] (if (< idx (vector-length decls)) (vector-get (vector-get decls idx) 0) -1))
(defn text-char-or-minus-one [text idx] (if (< idx (string-length text)) (string-char-at text idx) -1))
(defn span-kind-or-minus-one [spans idx] (if (< (* idx 3) (vector-length spans)) (span-kind spans idx) -1))
(defn span-start-or-minus-one [spans idx] (if (< (+ (* idx 3) 1) (vector-length spans)) (span-start spans idx) -1))
(defn span-end-or-minus-one [spans idx] (if (< (+ (* idx 3) 2) (vector-length spans)) (span-end spans idx) -1))
(defn find-span-kind-index [spans idx n target-kind]
  (if (>= idx n)
    -1
    (if (= (span-kind spans idx) target-kind)
      idx
      (find-span-kind-index spans (+ idx 1) n target-kind))))
(defn push-int-vector-local [dst value] (do (root_push dst) (let [next-dst (vector-push dst value)] (do (root_pop) next-dst))))
(defn ref-map-get-safe [map-ref key]
  (let [map-value (ref-get map-ref)]
    (do
      (root_push map-value)
      (let [value (map-get map-value key)]
        (do
          (root_pop)
          value)))))
(defn ref-map-insert-int-safe [map-ref key value]
  (let [map-value (ref-get map-ref)]
    (do
      (root_push map-value)
      (let [next-map (map-insert map-value key value)]
        (do
          (root_pop)
          next-map)))))
(defn ref-map-insert-object-safe [map-ref key value]
  (let [map-value (ref-get map-ref)]
    (do
      (root_push map-value)
      (root_push value)
      (let [next-map (map-insert map-value key value)]
        (do
          (root_pop)
          (root_pop)
          next-map)))))
(defn make-src-decl-pair [src decls]
  (do
    (root_push src)
    (root_push decls)
    (let [pair1 (vector-push (vector-new 2) src)]
      (do
        (root_push pair1)
        (let [pair2 (vector-push pair1 decls)]
          (do
            (root_pop)
            (root_pop)
            (root_pop)
            pair2))))))
(defn clone-src-decl-pair [pair]
  (let [pair-slot (root_push pair)
    src (vector-get pair 0)
    decls (vector-get pair 1)]
    (do
      (root_set pair-slot (make-src-decl-pair src decls))
      (root_pop))))
(defn push-object-vector [dst value] (do (root_push dst) (root_push value) (let [next-dst (vector-push dst value)] (do (root_pop) (root_pop) next-dst))))
(defn make-source-fingerprint-state [done next-pos next-acc] (push-int-vector-local (push-int-vector-local (push-int-vector-local (vector-new 3) done) next-pos) next-acc))
(defn append-src-decl-pair [pairs src decls]
  (let [pair (make-src-decl-pair src decls)]
    (do
      (root_push pair)
      (let [result (push-object-vector pairs pair)]
        (do
          (root_pop)
          result)))))
(defn source-fingerprint-step [src pos end acc] (if (>= pos end) (make-source-fingerprint-state 1 pos acc) (make-source-fingerprint-state 0 (+ pos 1) (+ (* acc 31) (string-char-at src pos)))))
(defn continue-source-fingerprint-step [src end state] (if (= (vector-get state 0) 1) state (source-fingerprint-step src (vector-get state 1) end (vector-get state 2))))
(defn source-fingerprint-step-8-loop-bounded [src pos end acc remaining]
  (if (>= pos end)
    (make-source-fingerprint-state 1 pos acc)
    (let [next-pos (+ pos 1)
      next-acc (+ (* acc 31) (string-char-at src pos))]
      (if (>= next-pos end)
        (make-source-fingerprint-state 1 next-pos next-acc)
        (if (<= remaining 1)
          (make-source-fingerprint-state 0 next-pos next-acc)
          (source-fingerprint-step-8-loop-bounded src next-pos end next-acc (- remaining 1)))))))
(defn source-fingerprint-step-8 [src pos end acc] (source-fingerprint-step-8-loop-bounded src pos end acc 8))
(defn continue-source-fingerprint-step-8 [src end state] (if (= (vector-get state 0) 1) state (source-fingerprint-step-8 src (vector-get state 1) end (vector-get state 2))))
(defn source-fingerprint-step-64-loop-bounded [src pos end acc remaining]
  (let [step (source-fingerprint-step-8 src pos end acc)]
    (if (= (vector-get step 0) 1)
      step
      (if (<= remaining 1)
        step
        (source-fingerprint-step-64-loop-bounded src (vector-get step 1) end (vector-get step 2) (- remaining 1))))))
(defn source-fingerprint-step-64 [src pos end acc] (source-fingerprint-step-64-loop-bounded src pos end acc 8))
(defn continue-source-fingerprint-step-64 [src end state] (if (= (vector-get state 0) 1) state (source-fingerprint-step-64 src (vector-get state 1) end (vector-get state 2))))
(defn source-fingerprint-step-512-loop-bounded [src pos end acc remaining]
  (let [step (source-fingerprint-step-64 src pos end acc)]
    (if (= (vector-get step 0) 1)
      step
      (if (<= remaining 1)
        step
        (source-fingerprint-step-512-loop-bounded src (vector-get step 1) end (vector-get step 2) (- remaining 1))))))
(defn source-fingerprint-step-512 [src pos end acc] (source-fingerprint-step-512-loop-bounded src pos end acc 8))
(defn continue-source-fingerprint-step-512 [src end state] (if (= (vector-get state 0) 1) state (source-fingerprint-step-512 src (vector-get state 1) end (vector-get state 2))))
(defn source-fingerprint-step-4096-loop-bounded [src pos end acc remaining]
  (let [step (source-fingerprint-step-512 src pos end acc)]
    (if (= (vector-get step 0) 1)
      step
      (if (<= remaining 1)
        step
        (source-fingerprint-step-4096-loop-bounded src (vector-get step 1) end (vector-get step 2) (- remaining 1))))))
(defn source-fingerprint-step-4096 [src pos end acc] (source-fingerprint-step-4096-loop-bounded src pos end acc 8))
(defn continue-source-fingerprint-step-4096 [src end state] (if (= (vector-get state 0) 1) state (source-fingerprint-step-4096 src (vector-get state 1) end (vector-get state 2))))
(defn source-fingerprint-step-32768-loop-bounded [src pos end acc remaining]
  (let [step (source-fingerprint-step-4096 src pos end acc)]
    (if (= (vector-get step 0) 1)
      step
      (if (<= remaining 1)
        step
        (source-fingerprint-step-32768-loop-bounded src (vector-get step 1) end (vector-get step 2) (- remaining 1))))))
(defn source-fingerprint-step-32768 [src pos end acc] (source-fingerprint-step-32768-loop-bounded src pos end acc 8))
(defn continue-source-fingerprint-step-32768 [src end state] (if (= (vector-get state 0) 1) state (source-fingerprint-step-32768 src (vector-get state 1) end (vector-get state 2))))
(defn source-fingerprint-step-262144-loop-bounded [src pos end acc remaining]
  (let [step (source-fingerprint-step-32768 src pos end acc)]
    (if (= (vector-get step 0) 1)
      step
      (if (<= remaining 1)
        step
        (source-fingerprint-step-262144-loop-bounded src (vector-get step 1) end (vector-get step 2) (- remaining 1))))))
(defn source-fingerprint-step-262144 [src pos end acc] (source-fingerprint-step-262144-loop-bounded src pos end acc 8))
(defn source-fingerprint-min [a b] (if (< a b) a b))
(defn source-fingerprint-range-8-loop-bounded [src pos end acc remaining]
  (if (>= pos end)
    acc
    (if (<= remaining 0)
      acc
      (source-fingerprint-range-8-loop-bounded src (+ pos 1) end (+ (* acc 31) (string-char-at src pos)) (- remaining 1)))))
(defn source-fingerprint-range-8 [src pos end acc] (source-fingerprint-range-8-loop-bounded src pos end acc 8))
(defn source-fingerprint-range-64-loop-bounded [src pos end acc remaining]
  (if (>= pos end)
    acc
    (if (<= remaining 0)
      acc
      (let [next-pos (source-fingerprint-min (+ pos 8) end)
        next-acc (source-fingerprint-range-8 src pos next-pos acc)]
        (source-fingerprint-range-64-loop-bounded src next-pos end next-acc (- remaining 1))))))
(defn source-fingerprint-range-64 [src pos end acc] (source-fingerprint-range-64-loop-bounded src pos end acc 8))
(defn source-fingerprint-range-512-loop-bounded [src pos end acc remaining]
  (if (>= pos end)
    acc
    (if (<= remaining 0)
      acc
      (let [next-pos (source-fingerprint-min (+ pos 64) end)
        next-acc (source-fingerprint-range-64 src pos next-pos acc)]
        (source-fingerprint-range-512-loop-bounded src next-pos end next-acc (- remaining 1))))))
(defn source-fingerprint-range-512 [src pos end acc] (source-fingerprint-range-512-loop-bounded src pos end acc 8))
(defn source-fingerprint-range-4096-loop-bounded [src pos end acc remaining]
  (if (>= pos end)
    acc
    (if (<= remaining 0)
      acc
      (let [next-pos (source-fingerprint-min (+ pos 512) end)
        next-acc (source-fingerprint-range-512 src pos next-pos acc)]
        (source-fingerprint-range-4096-loop-bounded src next-pos end next-acc (- remaining 1))))))
(defn source-fingerprint-range-4096 [src pos end acc] (source-fingerprint-range-4096-loop-bounded src pos end acc 8))
(defn source-fingerprint-range-32768-loop-bounded [src pos end acc remaining]
  (if (>= pos end)
    acc
    (if (<= remaining 0)
      acc
      (let [next-pos (source-fingerprint-min (+ pos 4096) end)
        next-acc (source-fingerprint-range-4096 src pos next-pos acc)]
        (source-fingerprint-range-32768-loop-bounded src next-pos end next-acc (- remaining 1))))))
(defn source-fingerprint-range-32768 [src pos end acc] (source-fingerprint-range-32768-loop-bounded src pos end acc 8))
(defn source-fingerprint-range-262144-loop-bounded [src pos end acc remaining]
  (if (>= pos end)
    acc
    (if (<= remaining 0)
      acc
      (let [next-pos (source-fingerprint-min (+ pos 32768) end)
        next-acc (source-fingerprint-range-32768 src pos next-pos acc)]
        (source-fingerprint-range-262144-loop-bounded src next-pos end next-acc (- remaining 1))))))
(defn source-fingerprint-range-262144 [src pos end acc] (source-fingerprint-range-262144-loop-bounded src pos end acc 8))
(defn source-fingerprint-loop [src pos end acc]
  (if (>= pos end)
    acc
    (let [next-pos (source-fingerprint-min (+ pos 262144) end)
      next-acc (source-fingerprint-range-262144 src pos next-pos acc)]
      (source-fingerprint-loop src next-pos end next-acc))))
(defn source-fingerprint [src] (do (root_push src) (let [result (source-fingerprint-loop src 0 (string-length src) 0)] (do (root_pop) result))))
(defn src-decl-cache-key [path] (* (name-hash path 0 (string-length path)) 2))
(defn make-src-decl-cache-entry [fingerprint pair]
  (do
    (root_push pair)
    (let [entry (push-object-vector (push-int-vector-local (vector-new 2) fingerprint) pair)]
      (do
        (root_pop)
        entry))))
(defn src-decl-cache-entry-fingerprint [entry] (vector-get entry 0))
(defn src-decl-cache-entry-pair [entry] (vector-get entry 1))
(defn make-cache-compile-context [source-root package-root cache-ref parse-count-ref]
  (do
    (root_push source-root)
    (root_push package-root)
    (root_push cache-ref)
    (root_push parse-count-ref)
    (let [ctx1 (push-object-vector (vector-new 4) source-root)]
      (do
        (root_push ctx1)
        (let [ctx2 (push-object-vector ctx1 package-root)]
          (do
            (root_push ctx2)
            (let [ctx3 (push-object-vector ctx2 cache-ref)]
              (do
                (root_push ctx3)
                (let [ctx4 (push-object-vector ctx3 parse-count-ref)]
                  (do
                    (root_pop)
                    (root_pop)
                    (root_pop)
                    (root_pop)
                    (root_pop)
                    (root_pop)
                    (root_pop)
                    ctx4))))))))))
(defn cache-compile-context-source-root [ctx] (vector-get ctx 0))
(defn cache-compile-context-package-root [ctx] (vector-get ctx 1))
(defn cache-compile-context-cache-ref [ctx] (vector-get ctx 2))
(defn cache-compile-context-parse-count-ref [ctx] (vector-get ctx 3))
(defn parse-src-decl-pair [src]
  (do
    (root_push src)
    (let [decls (parse-program src)]
      (do
        (root_push decls)
        (let [pair (make-src-decl-pair src decls)]
          (do
            (root_pop)
            (root_pop)
            pair))))))
(defn load-src-decl-pair-with-cache [path cache-ref parse-count-ref]
  (let [src (read-file path)]
    (do
      (root_push src)
      (let [fingerprint (source-fingerprint src)
        cache-key (src-decl-cache-key path)
        cached-entry (ref-map-get-safe cache-ref cache-key)
        result
        (if (= 0 cached-entry)
          (let [pair (parse-src-decl-pair src)
            entry (make-src-decl-cache-entry fingerprint pair)]
            (do
              (root_push pair)
              (root_push entry)
              (ref-set parse-count-ref (+ (ref-get parse-count-ref) 1))
              (ref-set cache-ref (ref-map-insert-object-safe cache-ref cache-key entry))
              (root_pop)
              (root_pop)
              pair))
          (if (= (src-decl-cache-entry-fingerprint cached-entry) fingerprint)
            (let [pair (src-decl-cache-entry-pair cached-entry)]
              (do
                (root_push pair)
                (root_pop)
                pair))
            (let [pair (parse-src-decl-pair src)
              entry (make-src-decl-cache-entry fingerprint pair)]
              (do
                (root_push pair)
                (root_push entry)
                (ref-set parse-count-ref (+ (ref-get parse-count-ref) 1))
                (ref-set cache-ref (ref-map-insert-object-safe cache-ref cache-key entry))
                (root_pop)
                (root_pop)
                pair))))]
        (do
          (root_pop)
          result)))))
(defn make-pairs-step-state [done next-idx next-pairs]
  (do
    (root_push next-pairs)
    (let [base0 (push-int-vector-local (vector-new 3) done)]
      (do
        (root_push base0)
        (let [base1 (push-int-vector-local base0 next-idx)]
          (do
            (root_push base1)
            (let [state (push-object-vector base1 next-pairs)]
              (do
                (root_pop)
                (root_pop)
                (root_pop)
                state))))))))
(defn load-imports-from-decls-step [decls src idx n seen-ref pairs source-root package-root]
  (if (>= idx n)
    (make-pairs-step-state 1 idx pairs)
    (do
      (root_push decls)
      (root_push src)
      (root_push seen-ref)
      (root_push pairs)
      (root_push source-root)
      (root_push package-root)
      (let [decl (vector-get decls idx)]
        (do
          (root_push decl)
          (let [result
            (if (= (vector-get decl 0) 26)
              (let [name-start (vector-get decl 2)
                name-end (vector-get decl 3)
                module-name (substring src name-start name-end)]
                (do
                  (root_push module-name)
                  (let [updated-pairs (load-module-if-new module-name source-root package-root seen-ref pairs)]
                    (do
                      (root_push updated-pairs)
                      (let [state (make-pairs-step-state 0 (+ idx 1) updated-pairs)]
                        (do
                          (root_pop)
                          (root_pop)
                          (root_pop)
                          state))))))
              (make-pairs-step-state 0 (+ idx 1) pairs))]
            (do
              (root_pop)
              (root_pop)
              (root_pop)
              (root_pop)
              (root_pop)
              (root_pop)
              (root_pop)
              result)))))))
(defn continue-load-imports-from-decls-step [decls src n seen-ref source-root package-root state] (if (= (vector-get state 0) 1) state (load-imports-from-decls-step decls src (vector-get state 1) n seen-ref (vector-get state 2) source-root package-root)))
(defn load-imports-from-decls-step-8 [decls src idx n seen-ref pairs source-root package-root] (load-imports-from-decls-step-64-loop-bounded decls src idx n seen-ref pairs source-root package-root 8))
(defn continue-load-imports-from-decls-step-8 [decls src n seen-ref source-root package-root state] (if (= (vector-get state 0) 1) state (load-imports-from-decls-step-8 decls src (vector-get state 1) n seen-ref (vector-get state 2) source-root package-root)))
(defn load-imports-from-decls-step-64-loop-bounded [decls src idx n seen-ref pairs source-root package-root remaining]
  (do
    (root_push decls)
    (root_push src)
    (root_push seen-ref)
    (root_push pairs)
    (root_push source-root)
    (root_push package-root)
    (let [step (load-imports-from-decls-step decls src idx n seen-ref pairs source-root package-root)
      done (vector-get step 0)
      next-idx (vector-get step 1)
      next-pairs (vector-get step 2)]
      (do
        (root_push step)
        (root_push next-pairs)
        (let [result
          (if (= done 1)
            step
            (if (<= remaining 1)
              step
              (load-imports-from-decls-step-64-loop-bounded decls src next-idx n seen-ref next-pairs source-root package-root (- remaining 1))))]
          (do
            (root_pop)
            (root_pop)
            (root_pop)
            (root_pop)
            (root_pop)
            (root_pop)
            (root_pop)
            (root_pop)
            result))))))
(defn load-imports-from-decls-step-64 [decls src idx n seen-ref pairs source-root package-root] (load-imports-from-decls-step-64-loop-bounded decls src idx n seen-ref pairs source-root package-root 64))
(defn continue-load-imports-from-decls-step-64 [decls src n seen-ref source-root package-root state] (if (= (vector-get state 0) 1) state (load-imports-from-decls-step-64 decls src (vector-get state 1) n seen-ref (vector-get state 2) source-root package-root)))
(defn load-imports-from-decls-step-512-loop-bounded [decls src idx n seen-ref pairs source-root package-root remaining]
  (do
    (root_push decls)
    (root_push src)
    (root_push seen-ref)
    (root_push pairs)
    (root_push source-root)
    (root_push package-root)
    (let [step (load-imports-from-decls-step-64 decls src idx n seen-ref pairs source-root package-root)
      done (vector-get step 0)
      next-idx (vector-get step 1)
      next-pairs (vector-get step 2)]
      (do
        (root_push step)
        (root_push next-pairs)
        (let [result
          (if (= done 1)
            step
            (if (<= remaining 1)
              step
              (load-imports-from-decls-step-512-loop-bounded decls src next-idx n seen-ref next-pairs source-root package-root (- remaining 1))))]
          (do
            (root_pop)
            (root_pop)
            (root_pop)
            (root_pop)
            (root_pop)
            (root_pop)
            (root_pop)
            (root_pop)
            result))))))
(defn load-imports-from-decls-step-512 [decls src idx n seen-ref pairs source-root package-root] (load-imports-from-decls-step-512-loop-bounded decls src idx n seen-ref pairs source-root package-root 8))
(defn continue-load-imports-from-decls-step-512 [decls src n seen-ref source-root package-root state] (if (= (vector-get state 0) 1) state (load-imports-from-decls-step-512 decls src (vector-get state 1) n seen-ref (vector-get state 2) source-root package-root)))
(defn load-imports-from-decls-step-4096-loop-bounded [decls src idx n seen-ref pairs source-root package-root remaining]
  (do
    (root_push decls)
    (root_push src)
    (root_push seen-ref)
    (root_push pairs)
    (root_push source-root)
    (root_push package-root)
    (let [step (load-imports-from-decls-step-512 decls src idx n seen-ref pairs source-root package-root)
      done (vector-get step 0)
      next-idx (vector-get step 1)
      next-pairs (vector-get step 2)]
      (do
        (root_push step)
        (root_push next-pairs)
        (let [result
          (if (= done 1)
            step
            (if (<= remaining 1)
              step
              (load-imports-from-decls-step-4096-loop-bounded decls src next-idx n seen-ref next-pairs source-root package-root (- remaining 1))))]
          (do
            (root_pop)
            (root_pop)
            (root_pop)
            (root_pop)
            (root_pop)
            (root_pop)
            (root_pop)
            (root_pop)
            result))))))
(defn load-imports-from-decls-step-4096 [decls src idx n seen-ref pairs source-root package-root] (load-imports-from-decls-step-4096-loop-bounded decls src idx n seen-ref pairs source-root package-root 8))
(defn load-imports-from-decls [decls src idx n seen-ref pairs source-root package-root] (let [step (load-imports-from-decls-step-4096 decls src idx n seen-ref pairs source-root package-root)] (if (= (vector-get step 0) 1) (vector-get step 2) (load-imports-from-decls decls src (vector-get step 1) n seen-ref (vector-get step 2) source-root package-root))))
(defn load-module-if-new [module-name source-root package-root seen-ref pairs]
  (do
    (root_push module-name)
    (root_push source-root)
    (root_push package-root)
    (root_push seen-ref)
    (root_push pairs)
    (let [module-key (name-hash module-name 0 (string-length module-name))]
      (if (= 0 (ref-map-get-safe seen-ref module-key))
        (do
          (ref-set seen-ref (ref-map-insert-int-safe seen-ref module-key 1))
          (let [path (resolve-module-path module-name source-root package-root)
            path-slot (root_push path)
            src (read-file path)
            decls (parse-program src)]
            (do
              (root_push src)
              (root_push decls)
              (let [pairs-with-deps (load-imports-from-decls decls src 0 (vector-length decls) seen-ref pairs source-root package-root)]
                (do
                  (root_push pairs-with-deps)
                  (let [next-pairs (append-src-decl-pair pairs-with-deps src decls)]
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
                      next-pairs)))))))
        (do
          (root_pop)
          (root_pop)
          (root_pop)
          (root_pop)
          (root_pop)
          pairs)))))
(defn load-imports-from-decls-with-cache-step [decls src idx n seen-ref pairs cache-ctx]
  (if (>= idx n)
    (make-pairs-step-state 1 idx pairs)
    (do
      (root_push decls)
      (root_push src)
      (root_push seen-ref)
      (root_push pairs)
      (root_push cache-ctx)
      (let [decl (vector-get decls idx)]
        (do
          (root_push decl)
          (let [result
            (if (= (vector-get decl 0) 26)
              (let [name-start (vector-get decl 2)
                name-end (vector-get decl 3)
                module-name (substring src name-start name-end)]
                (do
                  (root_push module-name)
                  (let [updated-pairs (load-module-if-new-with-cache module-name seen-ref pairs cache-ctx)]
                    (do
                      (root_push updated-pairs)
                      (let [state (make-pairs-step-state 0 (+ idx 1) updated-pairs)]
                        (do
                          (root_pop)
                          (root_pop)
                          (root_pop)
                          state))))))
              (make-pairs-step-state 0 (+ idx 1) pairs))]
            (do
              (root_pop)
              (root_pop)
              (root_pop)
              (root_pop)
              (root_pop)
              (root_pop)
              result)))))))
(defn continue-load-imports-from-decls-with-cache-step [decls src n seen-ref cache-ctx state] (if (= (vector-get state 0) 1) state (load-imports-from-decls-with-cache-step decls src (vector-get state 1) n seen-ref (vector-get state 2) cache-ctx)))
(defn load-imports-from-decls-with-cache-step-8 [decls src idx n seen-ref pairs cache-ctx] (load-imports-from-decls-with-cache-step-64-loop-bounded decls src idx n seen-ref pairs cache-ctx 8))
(defn continue-load-imports-from-decls-with-cache-step-8 [decls src n seen-ref cache-ctx state] (if (= (vector-get state 0) 1) state (load-imports-from-decls-with-cache-step-8 decls src (vector-get state 1) n seen-ref (vector-get state 2) cache-ctx)))
(defn load-imports-from-decls-with-cache-step-64-loop-bounded [decls src idx n seen-ref pairs cache-ctx remaining]
  (do
    (root_push decls)
    (root_push src)
    (root_push seen-ref)
    (root_push pairs)
    (root_push cache-ctx)
    (let [step (load-imports-from-decls-with-cache-step decls src idx n seen-ref pairs cache-ctx)
      done (vector-get step 0)
      next-idx (vector-get step 1)
      next-pairs (vector-get step 2)]
      (do
        (root_push step)
        (root_push next-pairs)
        (let [result
          (if (= done 1)
            step
            (if (<= remaining 1)
              step
              (load-imports-from-decls-with-cache-step-64-loop-bounded decls src next-idx n seen-ref next-pairs cache-ctx (- remaining 1))))]
          (do
            (root_pop)
            (root_pop)
            (root_pop)
            (root_pop)
            (root_pop)
            (root_pop)
            (root_pop)
            result))))))
(defn load-imports-from-decls-with-cache-step-64 [decls src idx n seen-ref pairs cache-ctx] (load-imports-from-decls-with-cache-step-64-loop-bounded decls src idx n seen-ref pairs cache-ctx 64))
(defn continue-load-imports-from-decls-with-cache-step-64 [decls src n seen-ref cache-ctx state]
  (if (= (vector-get state 0) 1)
    state
    (do
      (root_push decls)
      (root_push src)
      (root_push seen-ref)
      (root_push cache-ctx)
      (root_push state)
      (let [result (load-imports-from-decls-with-cache-step-64 decls src (vector-get state 1) n seen-ref (vector-get state 2) cache-ctx)]
        (do
          (root_pop)
          (root_pop)
          (root_pop)
          (root_pop)
          (root_pop)
          result)))))
(defn load-imports-from-decls-with-cache-step-512-loop-bounded [decls src idx n seen-ref pairs cache-ctx remaining]
  (do
    (root_push decls)
    (root_push src)
    (root_push seen-ref)
    (root_push pairs)
    (root_push cache-ctx)
    (let [step (load-imports-from-decls-with-cache-step-64 decls src idx n seen-ref pairs cache-ctx)
      done (vector-get step 0)
      next-idx (vector-get step 1)
      next-pairs (vector-get step 2)]
      (do
        (root_push step)
        (root_push next-pairs)
        (let [result
          (if (= done 1)
            step
            (if (<= remaining 1)
              step
              (load-imports-from-decls-with-cache-step-512-loop-bounded decls src next-idx n seen-ref next-pairs cache-ctx (- remaining 1))))]
          (do
            (root_pop)
            (root_pop)
            (root_pop)
            (root_pop)
            (root_pop)
            (root_pop)
            (root_pop)
            result))))))
(defn load-imports-from-decls-with-cache-step-512 [decls src idx n seen-ref pairs cache-ctx] (load-imports-from-decls-with-cache-step-512-loop-bounded decls src idx n seen-ref pairs cache-ctx 8))
(defn continue-load-imports-from-decls-with-cache-step-512 [decls src n seen-ref cache-ctx state]
  (if (= (vector-get state 0) 1)
    state
    (do
      (root_push decls)
      (root_push src)
      (root_push seen-ref)
      (root_push cache-ctx)
      (root_push state)
      (let [result (load-imports-from-decls-with-cache-step-512 decls src (vector-get state 1) n seen-ref (vector-get state 2) cache-ctx)]
        (do
          (root_pop)
          (root_pop)
          (root_pop)
          (root_pop)
          (root_pop)
          result)))))
(defn load-imports-from-decls-with-cache-step-4096-loop-bounded [decls src idx n seen-ref pairs cache-ctx remaining]
  (do
    (root_push decls)
    (root_push src)
    (root_push seen-ref)
    (root_push pairs)
    (root_push cache-ctx)
    (let [step (load-imports-from-decls-with-cache-step-512 decls src idx n seen-ref pairs cache-ctx)
      done (vector-get step 0)
      next-idx (vector-get step 1)
      next-pairs (vector-get step 2)]
      (do
        (root_push step)
        (root_push next-pairs)
        (let [result
          (if (= done 1)
            step
            (if (<= remaining 1)
              step
              (load-imports-from-decls-with-cache-step-4096-loop-bounded decls src next-idx n seen-ref next-pairs cache-ctx (- remaining 1))))]
          (do
            (root_pop)
            (root_pop)
            (root_pop)
            (root_pop)
            (root_pop)
            (root_pop)
            (root_pop)
            result))))))
(defn load-imports-from-decls-with-cache-step-4096 [decls src idx n seen-ref pairs cache-ctx] (load-imports-from-decls-with-cache-step-4096-loop-bounded decls src idx n seen-ref pairs cache-ctx 8))
(defn load-imports-from-decls-with-cache [decls src idx n seen-ref pairs cache-ctx]
  (let [step (load-imports-from-decls-with-cache-step-4096 decls src idx n seen-ref pairs cache-ctx)]
    (if (= (vector-get step 0) 1)
      (vector-get step 2)
      (do
        (root_push decls)
        (root_push src)
        (root_push seen-ref)
        (root_push cache-ctx)
        (root_push step)
        (let [result (load-imports-from-decls-with-cache decls src (vector-get step 1) n seen-ref (vector-get step 2) cache-ctx)]
          (do
            (root_pop)
            (root_pop)
            (root_pop)
            (root_pop)
            (root_pop)
            result))))))
(defn load-module-if-new-with-cache [module-name seen-ref pairs cache-ctx]
  (do
    (root_push module-name)
    (root_push seen-ref)
    (root_push pairs)
    (root_push cache-ctx)
    (let [module-key (name-hash module-name 0 (string-length module-name))
      source-root (cache-compile-context-source-root cache-ctx)
      package-root (cache-compile-context-package-root cache-ctx)
      cache-ref (cache-compile-context-cache-ref cache-ctx)
      parse-count-ref (cache-compile-context-parse-count-ref cache-ctx)]
      (if (= 0 (ref-map-get-safe seen-ref module-key))
        (do
          (ref-set seen-ref (ref-map-insert-int-safe seen-ref module-key 1))
          (let [path (resolve-module-path-with-cache module-name source-root package-root cache-ref)
            path-slot (root_push path)
            pair (load-src-decl-pair-with-cache path cache-ref parse-count-ref)
            pair-slot (root_push pair)
            src (vector-get pair 0)
            decls (vector-get pair 1)]
            (do
              (root_push src)
              (root_push decls)
              (let [pairs-with-deps (load-imports-from-decls-with-cache decls src 0 (vector-length decls) seen-ref pairs cache-ctx)]
                (do
                  (root_push pairs-with-deps)
                  (let [next-pairs (append-src-decl-pair pairs-with-deps src decls)]
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
                      next-pairs)))))))
        (do
          (root_pop)
          (root_pop)
          (root_pop)
          (root_pop)
          pairs)))))
(defn load-imports-from-decls-with-cache-progress-step [decls src idx n seen-ref pairs cache-ctx]
  (if (>= idx n)
    (make-pairs-step-state 1 idx pairs)
    (do
      (root_push decls)
      (root_push src)
      (root_push seen-ref)
      (root_push pairs)
      (root_push cache-ctx)
      (let [decl (vector-get decls idx)]
        (do
          (root_push decl)
          (let [result
            (if (= (vector-get decl 0) 26)
              (let [name-start (vector-get decl 2)
                name-end (vector-get decl 3)
                module-name (substring src name-start name-end)]
                (do
                  (root_push module-name)
                  (let [updated-pairs (load-module-if-new-with-cache-progress module-name seen-ref pairs cache-ctx)]
                    (do
                      (root_push updated-pairs)
                      (let [state (make-pairs-step-state 0 (+ idx 1) updated-pairs)]
                        (do
                          (root_pop)
                          (root_pop)
                          (root_pop)
                          state))))))
              (make-pairs-step-state 0 (+ idx 1) pairs))]
            (do
              (root_pop)
              (root_pop)
              (root_pop)
              (root_pop)
              (root_pop)
              (root_pop)
              result)))))))
(defn continue-load-imports-from-decls-with-cache-progress-step [decls src n seen-ref cache-ctx state] (if (= (vector-get state 0) 1) state (load-imports-from-decls-with-cache-progress-step decls src (vector-get state 1) n seen-ref (vector-get state 2) cache-ctx)))
(defn load-imports-from-decls-with-cache-progress-step-8 [decls src idx n seen-ref pairs cache-ctx] (load-imports-from-decls-with-cache-progress-step-64-loop-bounded decls src idx n seen-ref pairs cache-ctx 8))
(defn continue-load-imports-from-decls-with-cache-progress-step-8 [decls src n seen-ref cache-ctx state]
  (if (= (vector-get state 0) 1)
    state
    (do
      (root_push decls)
      (root_push src)
      (root_push seen-ref)
      (root_push cache-ctx)
      (root_push state)
      (let [result (load-imports-from-decls-with-cache-progress-step-8 decls src (vector-get state 1) n seen-ref (vector-get state 2) cache-ctx)]
        (do
          (root_pop)
          (root_pop)
          (root_pop)
          (root_pop)
          (root_pop)
          result)))))
(defn load-imports-from-decls-with-cache-progress-step-64-loop-bounded [decls src idx n seen-ref pairs cache-ctx remaining]
  (do
    (root_push decls)
    (root_push src)
    (root_push seen-ref)
    (root_push pairs)
    (root_push cache-ctx)
    (let [step (load-imports-from-decls-with-cache-progress-step decls src idx n seen-ref pairs cache-ctx)
      done (vector-get step 0)
      next-idx (vector-get step 1)
      next-pairs (vector-get step 2)]
      (do
        (root_push step)
        (root_push next-pairs)
        (let [result
          (if (= done 1)
            step
            (if (<= remaining 1)
              step
              (load-imports-from-decls-with-cache-progress-step-64-loop-bounded decls src next-idx n seen-ref next-pairs cache-ctx (- remaining 1))))]
          (do
            (root_pop)
            (root_pop)
            (root_pop)
            (root_pop)
            (root_pop)
            (root_pop)
            (root_pop)
            result))))))
(defn load-imports-from-decls-with-cache-progress-step-64 [decls src idx n seen-ref pairs cache-ctx] (load-imports-from-decls-with-cache-progress-step-64-loop-bounded decls src idx n seen-ref pairs cache-ctx 64))
(defn continue-load-imports-from-decls-with-cache-progress-step-64 [decls src n seen-ref cache-ctx state]
  (if (= (vector-get state 0) 1)
    state
    (do
      (root_push decls)
      (root_push src)
      (root_push seen-ref)
      (root_push cache-ctx)
      (root_push state)
      (let [result (load-imports-from-decls-with-cache-progress-step-64 decls src (vector-get state 1) n seen-ref (vector-get state 2) cache-ctx)]
        (do
          (root_pop)
          (root_pop)
          (root_pop)
          (root_pop)
          (root_pop)
          result)))))
(defn load-imports-from-decls-with-cache-progress-step-512-loop-bounded [decls src idx n seen-ref pairs cache-ctx remaining]
  (do
    (root_push decls)
    (root_push src)
    (root_push seen-ref)
    (root_push pairs)
    (root_push cache-ctx)
    (let [step (load-imports-from-decls-with-cache-progress-step-64 decls src idx n seen-ref pairs cache-ctx)
      done (vector-get step 0)
      next-idx (vector-get step 1)
      next-pairs (vector-get step 2)]
      (do
        (root_push step)
        (root_push next-pairs)
        (let [result
          (if (= done 1)
            step
            (if (<= remaining 1)
              step
              (load-imports-from-decls-with-cache-progress-step-512-loop-bounded decls src next-idx n seen-ref next-pairs cache-ctx (- remaining 1))))]
          (do
            (root_pop)
            (root_pop)
            (root_pop)
            (root_pop)
            (root_pop)
            (root_pop)
            (root_pop)
            result))))))
(defn load-imports-from-decls-with-cache-progress-step-512 [decls src idx n seen-ref pairs cache-ctx] (load-imports-from-decls-with-cache-progress-step-512-loop-bounded decls src idx n seen-ref pairs cache-ctx 8))
(defn continue-load-imports-from-decls-with-cache-progress-step-512 [decls src n seen-ref cache-ctx state]
  (if (= (vector-get state 0) 1)
    state
    (do
      (root_push decls)
      (root_push src)
      (root_push seen-ref)
      (root_push cache-ctx)
      (root_push state)
      (let [result (load-imports-from-decls-with-cache-progress-step-512 decls src (vector-get state 1) n seen-ref (vector-get state 2) cache-ctx)]
        (do
          (root_pop)
          (root_pop)
          (root_pop)
          (root_pop)
          (root_pop)
          result)))))
(defn load-imports-from-decls-with-cache-progress-step-4096-loop-bounded [decls src idx n seen-ref pairs cache-ctx remaining]
  (do
    (root_push decls)
    (root_push src)
    (root_push seen-ref)
    (root_push pairs)
    (root_push cache-ctx)
    (let [step (load-imports-from-decls-with-cache-progress-step-512 decls src idx n seen-ref pairs cache-ctx)
      done (vector-get step 0)
      next-idx (vector-get step 1)
      next-pairs (vector-get step 2)]
      (do
        (root_push step)
        (root_push next-pairs)
        (let [result
          (if (= done 1)
            step
            (if (<= remaining 1)
              step
              (load-imports-from-decls-with-cache-progress-step-4096-loop-bounded decls src next-idx n seen-ref next-pairs cache-ctx (- remaining 1))))]
          (do
            (root_pop)
            (root_pop)
            (root_pop)
            (root_pop)
            (root_pop)
            (root_pop)
            (root_pop)
            result))))))
(defn load-imports-from-decls-with-cache-progress-step-4096 [decls src idx n seen-ref pairs cache-ctx] (load-imports-from-decls-with-cache-progress-step-4096-loop-bounded decls src idx n seen-ref pairs cache-ctx 8))
(defn load-imports-from-decls-with-cache-progress [decls src idx n seen-ref pairs cache-ctx]
  (let [step (load-imports-from-decls-with-cache-progress-step-4096 decls src idx n seen-ref pairs cache-ctx)]
    (if (= (vector-get step 0) 1)
      (vector-get step 2)
      (do
        (root_push decls)
        (root_push src)
        (root_push seen-ref)
        (root_push cache-ctx)
        (root_push step)
        (let [result (load-imports-from-decls-with-cache-progress decls src (vector-get step 1) n seen-ref (vector-get step 2) cache-ctx)]
          (do
            (root_pop)
            (root_pop)
            (root_pop)
            (root_pop)
            (root_pop)
            result))))))
(defn load-module-if-new-with-cache-progress [module-name seen-ref pairs cache-ctx]
  (do
    (root_push module-name)
    (root_push seen-ref)
    (root_push pairs)
    (root_push cache-ctx)
    (let [module-key (name-hash module-name 0 (string-length module-name))
      source-root (cache-compile-context-source-root cache-ctx)
      package-root (cache-compile-context-package-root cache-ctx)
      cache-ref (cache-compile-context-cache-ref cache-ctx)
      parse-count-ref (cache-compile-context-parse-count-ref cache-ctx)]
      (do
        (if (= 0 (ref-map-get-safe seen-ref module-key))
          (do
            (ref-set seen-ref (ref-map-insert-int-safe seen-ref module-key 1))
            (let [path (resolve-module-path-with-cache module-name source-root package-root cache-ref)
              path-slot (root_push path)]
              (do
                (let [pair (load-src-decl-pair-with-cache path cache-ref parse-count-ref)
                  pair-slot (root_push pair)
                  src (vector-get pair 0)
                  decls (vector-get pair 1)]
                  (do
                    (root_push src)
                    (root_push decls)
                    (let [pairs-with-deps (load-imports-from-decls-with-cache-progress decls src 0 (vector-length decls) seen-ref pairs cache-ctx)]
                      (do
                        (root_push pairs-with-deps)
                        (let [next-pairs (append-src-decl-pair pairs-with-deps src decls)]
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
                            next-pairs)))))))))
          (do
            (root_pop)
            (root_pop)
            (root_pop)
            (root_pop)
            pairs))))))
(defn compile-file-pairs-with-cache [path cache-ref parse-count-ref]
  (let [path-slot (root_push path)
    pair (load-src-decl-pair-with-cache path cache-ref parse-count-ref)
    pair-slot (root_push pair)
    source-root (resolve-source-root path)
    package-root (resolve-package-root path)
    src (vector-get pair 0)
    program (vector-get pair 1)]
    (do
      (root_push source-root)
      (root_push package-root)
      (root_push src)
      (root_push program)
      (let [cache-ctx (make-cache-compile-context source-root package-root cache-ref parse-count-ref)]
        (do
          (root_set pair-slot cache-ctx)
          (let [seen-ref (ref-new (map-new))]
            (do
              (root_push seen-ref)
              (let [pairs0 (vector-new 8)]
                (do
                  (root_push pairs0)
                  (let [imported-pairs (load-imports-from-decls-with-cache program src 0 (vector-length program) seen-ref pairs0 cache-ctx)]
                    (do
                      (root_push imported-pairs)
                      (let [result (append-src-decl-pair imported-pairs src program)]
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
                          result)))))))))))))
(defn compile-file-functions-with-cache [path func-idx cache-ref parse-count-ref data-ref]
  (let [all-pairs (compile-file-pairs-with-cache path cache-ref parse-count-ref)]
    (do
      (root_push all-pairs)
      (root_push data-ref)
      (let [n (vector-length all-pairs)
        reg-result (register-all-pairs all-pairs 0 n (ftable-new) func-idx)
        ftable (vector-get reg-result 0)]
        (do
          (root_push reg-result)
          (let [functions0 (vector-new 8)]
            (do
              (root_push functions0)
              (let [functions (compile-all-src-decl-pairs-chunked all-pairs 0 n ftable data-ref functions0)]
                (do
                  (root_pop)
                  (root_pop)
                  (root_pop)
                  (root_pop)
                  functions)))))))))
(defn compile-file-functions-payload-with-cache [path func-idx cache-ref parse-count-ref]
  (let [path-slot (root_push path)
    cache-slot (root_push cache-ref)
    parse-count-slot (root_push parse-count-ref)
    data-ref (ref-new (vector-new 8))
    data-slot (root_push data-ref)
    functions (compile-file-functions-with-cache path func-idx cache-ref parse-count-ref data-ref)]
    (do
      (root_push functions)
      (let [data (ref-get data-ref)]
        (do
          (root_push data)
          (let [payload1 (vector-push (vector-new 2) functions)]
            (do
              (root_push payload1)
              (let [payload2 (vector-push payload1 data)]
                (do
                  (root_pop)
                  (root_pop)
                  (root_pop)
                  (root_pop)
                  (root_pop)
                  (root_pop)
                  (root_pop)
                  payload2)))))))))
(defn compile-file-functions-payload-with-cache-progress [path func-idx cache-ref parse-count-ref]
  (do
    (print 9000000041)
    (let [src (read-file path)]
      (do
        (root_push src)
        (print 9000000042)
        (print (string-length src))
        (print (if (> (string-length src) 0) (string-char-at src 0) -1))
        (print 9000000043)
        (let [fingerprint (source-fingerprint src)]
          (do
            (print fingerprint)
            (let [decls (parse-program src)]
              (do
                (root_push decls)
                (print 9000000044)
                (print (vector-length decls))
                (root_pop)
                (root_pop)
                (print 9000000069)
                (let [path-slot (root_push path)
                  cache-slot (root_push cache-ref)
                  parse-count-slot (root_push parse-count-ref)
                  data-ref (ref-new (vector-new 8))
                  data-slot (root_push data-ref)
                  all-pairs (compile-file-pairs-with-cache path cache-ref parse-count-ref)]
                  (do
                    (root_push all-pairs)
                    (root_push data-ref)
                    (print 9000000070)
                    (print (vector-length all-pairs))
                    (print (ref-get parse-count-ref))
                    (let [n (vector-length all-pairs)
                      start-ftable (ftable-new)]
                      (do
                        (root_push start-ftable)
                        (print 9000000075)
                        (print n)
                        (let [reg-result (register-all-pairs-progress all-pairs 0 n start-ftable func-idx)
                          ftable (vector-get reg-result 0)]
                          (do
                            (root_push reg-result)
                            (print 9000000071)
                            (print n)
                            (print (vector-length reg-result))
                            (let [functions0 (vector-new 8)]
                              (do
                                (root_push functions0)
                                (print 9000000072)
                                (print (vector-length functions0))
                                (let [functions (compile-all-src-decl-pairs-chunked-progress all-pairs 0 n ftable data-ref functions0)]
                                  (do
                                    (root_push functions)
                                    (print 9000000132)
                                    (print (vector-length functions))
                                    (print (vector-length (ref-get data-ref)))
                                    (print 9000000073)
                                    (print (vector-length functions))
                                    (let [data (ref-get data-ref)]
                                      (do
                                        (root_push data)
                                        (let [payload1 (vector-push (vector-new 2) functions)]
                                          (do
                                            (root_push payload1)
                                            (let [payload2 (vector-push payload1 data)]
                                              (do
                                                (print 9000000074)
                                                (print (vector-length data))
                                                (print (vector-length payload2))
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
                                                payload2))))))))))))))))))))))))
(defn compile-file-mode-cache-probe []
  (let [path (command-line-arg 1)
    cache-ref (ref-new (map-new))
    parse-count-ref (ref-new 0)
    src (read-file path)]
    (do
      (root_push src)
      (let [decls (parse-program src)]
        (do
          (root_push decls)
          (let [pair (make-src-decl-pair src decls)
            fingerprint (source-fingerprint src)
            cache-key (src-decl-cache-key path)
            entry (make-src-decl-cache-entry fingerprint pair)]
            (do
              (root_push pair)
              (root_push entry)
              (ref-set parse-count-ref (+ (ref-get parse-count-ref) 1))
              (ref-set cache-ref (ref-map-insert-object-safe cache-ref cache-key entry))
              (print 80)
              (print (ref-get parse-count-ref))
              (print (string-length src))
              (print (vector-length decls))
              (root_pop)
              (root_pop)
              (root_pop)
              (root_pop)
              0)))))))
(defn compile-file-mode-cache-pairs-probe []
  (let [path (command-line-arg 1)]
    (do
      (root_push path)
      (let [cache-ref (ref-new (map-new))]
        (do
          (root_push cache-ref)
          (let [parse-count-ref (ref-new 0)]
            (do
              (root_push parse-count-ref)
              (let [all-pairs (compile-file-pairs-with-cache path cache-ref parse-count-ref)]
                (do
                  (root_push all-pairs)
                  (let [n (vector-length all-pairs)
                    entry-pair (vector-get all-pairs (- n 1))]
                    (do
                      (root_push entry-pair)
                      (let [entry-decls (vector-get entry-pair 1)]
                        (do
                          (root_push entry-decls)
                          (print 81)
                          (print (ref-get parse-count-ref))
                          (print n)
                          (print (vector-length entry-decls))
                          (root_pop)
                          (root_pop)
                          (root_pop)
                          (root_pop)
                          (root_pop)
                          0)))))))))))))
(defn compile-file-mode-cache-functions-probe []
  (let [path (command-line-arg 1)]
    (do
      (root_push path)
      (let [cache-ref (ref-new (map-new))]
        (do
          (root_push cache-ref)
          (let [parse-count-ref (ref-new 0)]
            (do
              (root_push parse-count-ref)
              (let [data-ref (ref-new (vector-new 8))]
                (do
                  (root_push data-ref)
                  (let [functions (compile-file-functions-with-cache path 10 cache-ref parse-count-ref data-ref)]
                    (do
                      (root_push functions)
                      (let [data (ref-get data-ref)]
                        (do
                          (root_push data)
                          (print 82)
                          (print (ref-get parse-count-ref))
                          (print (vector-length functions))
                          (print (vector-length data))
                          (root_pop)
                          (root_pop)
                          (root_pop)
                          (root_pop)
                          (root_pop)
                          (root_pop)
                          0)))))))))))))
(defn compile-file-mode-cache-payload-probe []
  (let [path (command-line-arg 1)]
    (do
      (root_push path)
      (let [cache-ref (ref-new (map-new))]
        (do
          (root_push cache-ref)
          (let [parse-count-ref (ref-new 0)]
            (do
              (root_push parse-count-ref)
              (let [payload (compile-file-functions-payload-with-cache path 10 cache-ref parse-count-ref)]
                (do
                  (root_push payload)
                  (let [functions (vector-get payload 0)
                    data (vector-get payload 1)]
                    (do
                      (root_push functions)
                      (root_push data)
                      (print 84)
                      (print (ref-get parse-count-ref))
                      (print (vector-length functions))
                      (print (vector-length data))
                      (root_pop)
                      (root_pop)
                      (root_pop)
                      (root_pop)
                      (root_pop)
                      0)))))))))))
(defn compile-file-mode-cache-pre-normalize-probe []
  (let [path (command-line-arg 1)]
    (do
      (root_push path)
      (let [cache-ref (ref-new (map-new))]
        (do
          (root_push cache-ref)
          (let [parse-count-ref (ref-new 0)]
            (do
              (root_push parse-count-ref)
              (let [payload (compile-file-functions-payload-with-cache path 10 cache-ref parse-count-ref)]
                (do
                  (root_push payload)
                  (let [functions (vector-get payload 0)
                    data (vector-get payload 1)]
                    (do
                      (root_push functions)
                      (root_push data)
                      (print 83)
                      (print (vector-length functions))
                      (print (+ 10 (vector-length functions)))
                      (print (ref-get parse-count-ref))
                      (root_pop)
                      (root_pop)
                      (root_pop)
                      (root_pop)
                      (root_pop)
                      0)))))))))))
(defn print-let-shape-progress-probe [node depth limit]
  (if (>= depth limit)
    0
    (do
      (root_push node)
      (let [tag (vector-get node 0)]
        (do
          (print 9000000051)
          (print depth)
          (print tag)
          (if (= tag 7)
            (let [init-expr (vector-get node 2)
              body-expr (vector-get node 3)]
              (do
                (root_push init-expr)
                (root_push body-expr)
                (print (vector-get node 1))
                (print (vector-get init-expr 0))
                (print (vector-get body-expr 0))
                (let [result (print-let-shape-progress-probe body-expr (+ depth 1) limit)]
                  (do
                    (root_pop)
                    (root_pop)
                    (root_pop)
                    result))))
            (do
              (print -1)
              (print -1)
              (print -1)
              (root_pop)
              0)))))))
(defn print-tokenize-step-progress-probe [src]
  (let [src-len (string-length src)
    tokens0 (vector-new 32)]
    (do
      (root_push src)
      (root_push tokens0)
      (print 9000000059)
      (print (string-char-at src 0))
      (print (skip-ws-loop src 0 src-len))
      (print (lex-one src 0 src-len))
      (let [manual-base (vector-new 32)]
        (do
          (root_push manual-base)
          (let [manual-kind (vector-push manual-base 0)]
            (do
              (root_push manual-kind)
              (let [manual-start (vector-push manual-kind 0)]
                (do
                  (root_push manual-start)
                  (let [manual-token (vector-push manual-start 1)]
                    (do
                      (root_push manual-token)
                      (print 9000000062)
                      (print (vector-length manual-kind))
                      (print (vector-length manual-start))
                      (print (vector-length manual-token))
                      (root_pop)))
                  (root_pop)))
              (root_pop)))
          (root_pop)))
      (let [manual-append-base (vector-new 32)]
        (do
          (root_push manual-append-base)
          (let [manual-appended (append-span-token manual-append-base 0 0 1)]
            (do
              (root_push manual-appended)
              (print 9000000063)
              (print (vector-length manual-appended))
              (let [manual-state (make-tokenize-state 0 1 manual-appended)]
                (do
                  (root_push manual-state)
                  (let [manual-state-tokens (vector-get manual-state 2)]
                    (do
                      (root_push manual-state-tokens)
                      (print 9000000064)
                      (print (vector-get manual-state 0))
                      (print (vector-get manual-state 1))
                      (print (vector-length manual-state-tokens))
                      (root_pop)))
                  (root_pop)))
              (root_pop)))
          (root_pop)))
      (let [manual-ws (skip-ws-loop src 0 src-len)]
        (let [manual-result (lex-one src manual-ws src-len)]
          (let [manual-kind2 (/ manual-result 1000000)]
            (let [manual-end (- manual-result (* manual-kind2 1000000))]
              (let [manual-next (append-span-token tokens0 manual-kind2 manual-ws manual-end)]
                (do
                  (root_push manual-next)
                  (print 9000000065)
                  (print manual-ws)
                  (print manual-kind2)
                  (print manual-end)
                  (print (vector-length manual-next))
                  (let [manual-step-state (make-tokenize-state-from-appended-tokens 0 manual-end manual-next)]
                    (do
                      (root_push manual-step-state)
                      (let [manual-step-tokens (vector-get manual-step-state 2)]
                        (do
                          (root_push manual-step-tokens)
                          (print 9000000066)
                          (print (vector-get manual-step-state 0))
                          (print (vector-get manual-step-state 1))
                          (print (vector-length manual-step-tokens))
                          (root_pop)))
                      (root_pop)))
                  (root_pop)))))))
      (let [manual-helper-state (append-span-token-state tokens0 0 1 0 0 1)]
        (do
          (root_push manual-helper-state)
          (let [manual-helper-tokens (vector-get manual-helper-state 2)]
            (do
              (root_push manual-helper-tokens)
              (print 9000000067)
              (print (vector-get manual-helper-state 0))
              (print (vector-get manual-helper-state 1))
              (print (vector-length manual-helper-tokens))
              (root_pop)))
          (root_pop)))
      (let [manual-lex-state (append-lex-result-state tokens0 (lex-one src 0 src-len) 0)]
        (do
          (root_push manual-lex-state)
          (let [manual-lex-tokens (vector-get manual-lex-state 2)]
            (do
              (root_push manual-lex-tokens)
              (print 9000000068)
              (print (vector-get manual-lex-state 0))
              (print (vector-get manual-lex-state 1))
              (print (vector-length manual-lex-tokens))
              (root_pop)))
          (root_pop)))
      (let [step1 (tokenize-spans-step src 0 src-len tokens0)]
        (do
          (root_push step1)
          (let [tokens1 (vector-get step1 2)]
            (do
              (root_push tokens1)
              (print 9000000060)
              (print (vector-get step1 0))
              (print (vector-get step1 1))
              (print (vector-length tokens1))
              (root_pop)))
          (let [step512 (tokenize-spans-step-512 src 0 src-len tokens0)]
            (do
              (root_push step512)
              (let [tokens512 (vector-get step512 2)]
                (do
                  (root_push tokens512)
                  (print 9000000061)
                  (print (vector-get step512 0))
                  (print (vector-get step512 1))
                  (print (vector-length tokens512))
                  (root_pop)))
              (root_pop)))
          (root_pop)
          (root_pop)
          (root_pop)
          0)))))
(defn print-direct-defn-build-progress-probe [spans first-defn-span src]
  (if (< first-defn-span 0)
    0
    (let [pos-ref (ref-new first-defn-span)]
      (do
        (root_push pos-ref)
        (print 180)
        (print (ref-get pos-ref))
        (print (p-current spans pos-ref))
        (p-advance pos-ref)
        (let [ns (p-start spans pos-ref)
          ne (p-end spans pos-ref)
          nh (name-hash src ns ne)]
          (do
            (print 181)
            (print (ref-get pos-ref))
            (print (p-current spans pos-ref))
            (p-advance pos-ref)
            (print 182)
            (print (ref-get pos-ref))
            (print (p-current spans pos-ref))
            (p-expect spans pos-ref 2)
            (print 183)
            (print (ref-get pos-ref))
            (print (p-current spans pos-ref))
            (let [result (vector-push-triple-rooted-v3 (vector-new 8) 20 nh 0)]
              (do
                (root_push result)
                (print 184)
                (print (vector-get result 0))
                (print (vector-length result))
                (let [with-params (parse-params-v3 spans pos-ref src result 0)]
                  (do
                    (root_push with-params)
                    (let [param-count (- (vector-length with-params) 3)
                      defn-node (vector-set-at-rooted-v3 with-params 2 param-count)]
                      (do
                        (root_push defn-node)
                        (print 185)
                        (print (vector-get with-params 0))
                        (print (vector-length with-params))
                        (print param-count)
                        (print (ref-get pos-ref))
                        (print (p-current spans pos-ref))
                        (print 186)
                        (print (vector-get defn-node 0))
                        (print (vector-length defn-node))
                        (skip-optional-type-sig-v3 spans pos-ref src)
                        (skip-optional-where-v3 spans pos-ref src)
                        (print 187)
                        (print (ref-get pos-ref))
                        (print (p-current spans pos-ref))
                        (let [body (parse-expr-v3 spans pos-ref src)]
                          (do
                            (root_push body)
                            (print 188)
                            (print (vector-get body 0))
                            (print (vector-length body))
                            (print (ref-get pos-ref))
                            (let [parsed (finalize-defn-parsed-body-v3 spans pos-ref defn-node param-count body)]
                              (do
                                (root_push parsed)
                                (print 189)
                                (print (vector-get parsed 0))
                                (print (vector-length parsed))
                                (print (ref-get pos-ref))
                                (root_pop)
                                (root_pop)
                                (root_pop)
                                (root_pop)
                                (root_pop)
                                (root_pop)
                                0))))))))))))))))

(defn print-direct-defn-return-cleanup-progress-probe [spans first-defn-span src]
  (if (< first-defn-span 0)
    0
    (let [pos-ref (ref-new first-defn-span)]
      (do
        (root_push pos-ref)
        (p-advance pos-ref)
        (let [ns (p-start spans pos-ref)
          ne (p-end spans pos-ref)
          nh (name-hash src ns ne)]
          (do
            (p-advance pos-ref)
            (p-expect spans pos-ref 2)
            (let [result (vector-push-triple-rooted-v3 (vector-new 8) 20 nh 0)]
              (do
                (let [result-slot (root_push result)
                  with-params (parse-params-v3 spans pos-ref src result 0)]
                  (do
                    (root_push with-params)
                    (let [param-count (- (vector-length with-params) 3)
                      defn-node (vector-set-at-rooted-v3 with-params 2 param-count)]
                      (do
                        (root_push defn-node)
                        (skip-optional-type-sig-v3 spans pos-ref src)
                        (skip-optional-where-v3 spans pos-ref src)
                        (let [parsed
                          (if (== (colon-directive-v3 spans pos-ref src) 1)
                            (let [meta (parse-defn-metadata-v3 spans pos-ref src)]
                              (parse-defn-bodyless-or-body-with-meta-v3
                                spans pos-ref src defn-node param-count meta))
                            (parse-defn-bodyless-or-body-v3
                              spans pos-ref src defn-node param-count))]
                          (do
                            (print 190)
                            (print (vector-get parsed 0))
                            (print (vector-length parsed))
                            (print (ref-get pos-ref))
                            (root_set result-slot parsed)
                            (root_pop)
                            (root_pop)
                            (root_pop)
                            (print 191)
                            (print (vector-get parsed 0))
                            (print (vector-length parsed))
                            (print (ref-get pos-ref))
                            (root_pop)
                            0))))))))))))))
(defn compile-file-mode-entry-shape-progress-probe []
  (let [path (command-line-arg 1)]
    (do
      (print 9000000050)
      (print (string-length path))
      (root_push path)
      (let [src (read-file path)]
        (do
          (root_push src)
          (print 9000000054)
          (print (string-length src))
          (print-tokenize-step-progress-probe src)
          (let [spans (tokenize-with-spans src)]
            (do
              (root_push spans)
              (print 9000000058)
              (print (vector-length spans))
              (root_pop)
              (root_pop)
              (root_pop)
              0)))))))
(defn compile-file-mode-cache-pairs-progress-probe []
  (let [path (command-line-arg 1)
    cache-ref (ref-new (map-new))
    parse-count-ref (ref-new 0)
    src (read-file path)
    program (parse-program src)
    pair (make-src-decl-pair src program)
    fingerprint (source-fingerprint src)
    cache-key (src-decl-cache-key path)
    entry (make-src-decl-cache-entry fingerprint pair)
    source-root (resolve-source-root path)
    package-root (resolve-package-root path)
    seen-ref (ref-new (map-new))
    cache-ctx (make-cache-compile-context source-root package-root cache-ref parse-count-ref)]
    (do
      (root_push src)
      (root_push program)
      (root_push pair)
      (root_push entry)
      (root_push seen-ref)
      (root_push cache-ctx)
      (ref-set parse-count-ref (+ (ref-get parse-count-ref) 1))
      (ref-set cache-ref (ref-map-insert-object-safe cache-ref cache-key entry))
      (let [pairs0 (vector-new 8)]
        (do
          (root_push pairs0)
          (let [imported-pairs
            (load-imports-from-decls-with-cache-progress program src 0 (vector-length program) seen-ref pairs0 cache-ctx)]
            (do
              (print 85)
              (print (ref-get parse-count-ref))
              (print (vector-length imported-pairs))
              (root_pop)
              (root_pop)
              (root_pop)
              (root_pop)
              (root_pop)
              (root_pop)
              (root_pop)
              0)))))))
(defn compile-file-mode-cache-compile-progress-probe []
  (let [path (command-line-arg 1)
    cache-ref (ref-new (map-new))
    parse-count-ref (ref-new 0)
    all-pairs (compile-file-pairs-with-cache path cache-ref parse-count-ref)]
    (do
      (root_push all-pairs)
      (let [data-ref (ref-new (vector-new 8))]
        (do
          (root_push data-ref)
          (let [n (vector-length all-pairs)
            reg-result (register-all-pairs all-pairs 0 n (ftable-new) 10)
            ftable (vector-get reg-result 0)]
            (do
              (root_push reg-result)
              (let [functions0 (vector-new 8)]
                (do
                  (root_push functions0)
                  (let [functions (compile-all-src-decl-pairs all-pairs 0 n ftable data-ref functions0)]
                    (do
                      (root_push functions)
                      (print 86)
                      (print (ref-get parse-count-ref))
                      (print 87)
                      (print n)
                      (print 88)
                      (print (- (vector-get reg-result 1) 10))
                      (print 89)
                      (print (vector-length functions))
                      (root_pop)
                      (root_pop)
                      (root_pop)
                      (root_pop)
                      0)))))))))))
(defn compile-file-mode-cache-compile-phase-probe []
  (let [path (command-line-arg 1)
    cache-ref (ref-new (map-new))
    parse-count-ref (ref-new 0)
    all-pairs (compile-file-pairs-with-cache path cache-ref parse-count-ref)]
    (do
      (print 150)
      (print (ref-get parse-count-ref))
      (root_push all-pairs)
      (let [data-ref (ref-new (vector-new 8))]
        (do
          (root_push data-ref)
          (let [n (vector-length all-pairs)
            reg-result (register-all-pairs all-pairs 0 n (ftable-new) 10)
            ftable (vector-get reg-result 0)]
            (do
              (root_push reg-result)
              (print 151)
              (print n)
              (print 152)
              (print (- (vector-get reg-result 1) 10))
              (let [functions0 (vector-new 8)]
                (do
                  (root_push functions0)
                  (let [functions (compile-all-src-decl-pairs all-pairs 0 n ftable data-ref functions0)]
                    (do
                      (root_push functions)
                      (print 153)
                      (print (vector-length functions))
                      (print 154)
                      (print (vector-length (ref-get data-ref)))
                      (root_pop)
                      (root_pop)
                      (root_pop)
                      (root_pop)
                      0)))))))))))
(defn compile-file-mode-cache-compile-pair-progress-probe []
  (let [path (command-line-arg 1)
    cache-ref (ref-new (map-new))
    parse-count-ref (ref-new 0)
    all-pairs (compile-file-pairs-with-cache path cache-ref parse-count-ref)]
    (do
      (print 150)
      (print (ref-get parse-count-ref))
      (root_push all-pairs)
      (let [data-ref (ref-new (vector-new 8))]
        (do
          (root_push data-ref)
          (let [n (vector-length all-pairs)
            reg-result (register-all-pairs all-pairs 0 n (ftable-new) 10)
            ftable (vector-get reg-result 0)]
            (do
              (root_push reg-result)
              (print 151)
              (print n)
              (print 152)
              (print (- (vector-get reg-result 1) 10))
              (let [first-pair-debug
                (if (> n 0)
                  (let [pair0 (vector-get all-pairs 0)
                    src0 (vector-get pair0 0)
                    decls0 (vector-get pair0 1)
                    debug-data-ref (ref-new (vector-new 8))]
                    (do
                      (root_push pair0)
                      (root_push src0)
                      (root_push decls0)
                      (root_push debug-data-ref)
                      (print 155)
                      (print (vector-length decls0))
                      (print (decl-tag-or-minus-one decls0 0))
                      (print (decl-tag-or-minus-one decls0 1))
                      (print (decl-tag-or-minus-one decls0 2))
                      (print (decl-tag-or-minus-one decls0 3))
                      (let [reparsed (parse-program src0)
                        spans0 (tokenize-with-spans src0)
                        span-count (/ (vector-length spans0) 3)
                        first-defn-span (find-span-kind-index spans0 0 span-count 30)]
                        (do
                          (root_push reparsed)
                          (root_push spans0)
                          (print 157)
                          (print (vector-length reparsed))
                          (print (decl-tag-or-minus-one reparsed 0))
                          (print (decl-tag-or-minus-one reparsed 1))
                          (print (decl-tag-or-minus-one reparsed 2))
                          (print (decl-tag-or-minus-one reparsed 3))
                          (print 158)
                          (print first-defn-span)
                          (print (span-kind-or-minus-one spans0 (- first-defn-span 1)))
                          (print (span-kind-or-minus-one spans0 first-defn-span))
                          (print (span-kind-or-minus-one spans0 (+ first-defn-span 1)))
                          (print (span-kind-or-minus-one spans0 (+ first-defn-span 2)))
                          (print (span-kind-or-minus-one spans0 (+ first-defn-span 3)))
                          (print (span-kind-or-minus-one spans0 (+ first-defn-span 4)))
                          (print (span-kind-or-minus-one spans0 (+ first-defn-span 5)))
                          (print (span-kind-or-minus-one spans0 (+ first-defn-span 6)))
                          (print (span-kind-or-minus-one spans0 (+ first-defn-span 7)))
                          (if (>= first-defn-span 0)
                            (let [direct-pos (ref-new (- first-defn-span 1))]
                              (do
                                (root_push direct-pos)
                                (let [direct-node (parse-expr-v3 spans0 direct-pos src0)]
                                  (do
                                    (root_push direct-node)
                                    (print (vector-get direct-node 0))
                                    (print (vector-length direct-node))
                                    (print (ref-get direct-pos))
                                    (print (vector-get direct-node 1))
                                    (print (vector-get direct-node 2))
                                    (print (vector-get direct-node 3))
                                    (print (vector-get direct-node 4))
                                    (print (vector-get direct-node 5))
                                    (print (vector-get direct-node 6))
                                    (print (vector-get direct-node 7))
                                    (print (vector-get direct-node 8))
                                    (root_pop)
                                    (root_pop)))))
                            0)
                          (print 159)
                          (if (>= first-defn-span 0)
                            (let [direct-defn-pos (ref-new first-defn-span)]
                              (do
                                (root_push direct-defn-pos)
                                (let [direct-defn (parse-defn-v3 spans0 direct-defn-pos src0)]
                                  (do
                                    (root_push direct-defn)
                                    (print (vector-get direct-defn 0))
                                    (print (vector-length direct-defn))
                                    (print (ref-get direct-defn-pos))
                                    (print (vector-get direct-defn 1))
                                    (print (vector-get direct-defn 2))
                                    (print (vector-get direct-defn 3))
                                    (print (vector-get direct-defn 4))
                                    (root_pop)
                                    (root_pop)))))
                            0)
                          (print-direct-defn-build-progress-probe spans0 first-defn-span src0)
                          (print-direct-defn-return-cleanup-progress-probe spans0 first-defn-span src0)
                          (root_pop)
                          (root_pop)))
                      (let [debug-functions (compile-defn-functions-chunked-step-progress-debug decls0 0 (vector-length decls0) src0 ftable debug-data-ref (vector-new 8))]
                        (do
                          (root_push debug-functions)
                          (print 156)
                          (print (vector-length debug-functions))
                          (print (vector-length (ref-get debug-data-ref)))
                          (root_pop)
                          (root_pop)
                          (root_pop)
                          (root_pop)
                          (root_pop)
                          0))))
                  0)]
                (let [functions0 (vector-new 8)]
                  (do
                    (root_push functions0)
                    (let [functions (compile-all-src-decl-pairs-chunked-progress all-pairs 0 n ftable data-ref functions0)]
                      (do
                        (root_push functions)
                        (print 153)
                        (print (vector-length functions))
                        (print 154)
                        (print (vector-length (ref-get data-ref)))
                        (root_pop)
                        (root_pop)
                        (root_pop)
                        (root_pop)
                        first-pair-debug))))))))))))
(defn compile-file-mode-ast-chunked-step-progress-probe []
  (let [path (command-line-arg 1)
    cache-ref (ref-new (map-new))
    parse-count-ref (ref-new 0)
    all-pairs (compile-file-pairs-with-cache path cache-ref parse-count-ref)]
    (do
      (print 150)
      (print (ref-get parse-count-ref))
      (root_push all-pairs)
      (let [data-ref (ref-new (vector-new 8))]
        (do
          (root_push data-ref)
          (let [n (vector-length all-pairs)
            reg-result (register-all-pairs all-pairs 0 n (ftable-new) 10)
            ftable (vector-get reg-result 0)
            pair0 (vector-get all-pairs 0)
            src0 (vector-get pair0 0)
            decls0 (vector-get pair0 1)]
            (do
              (root_push reg-result)
              (root_push pair0)
              (root_push src0)
              (root_push decls0)
              (print 151)
              (print (vector-length decls0))
              (let [functions0 (compile-defn-functions-chunked-step-progress-debug decls0 0 (vector-length decls0) src0 ftable data-ref (vector-new 8))]
                (do
                  (root_push functions0)
                  (print 153)
                  (print (vector-length functions0))
                  (root_pop)
                  (root_pop)
                  (root_pop)
                  (root_pop)
                  (root_pop)
                  (root_pop)
                  0)))))))))
(defn make-register-pairs-state [done next-idx next-ftable next-func-idx]
  (do
    (let [done-ref (ref-new done)
      next-idx-ref (ref-new next-idx)
      next-func-idx-ref (ref-new next-func-idx)]
      (do
        (root_push done-ref)
        (root_push next-idx-ref)
        (root_push next-func-idx-ref)
        (root_push next-ftable)
        (let [base0 (push-int-vector-local (vector-new 4) (ref-get done-ref))]
          (do
            (root_push base0)
            (let [base1 (push-int-vector-local base0 (ref-get next-idx-ref))]
              (do
                (root_push base1)
                (let [with-ftable (push-object-vector base1 next-ftable)]
                  (do
                    (root_push with-ftable)
                    (let [state (vector-push with-ftable (ref-get next-func-idx-ref))]
                      (do
                        (root_pop)
                        (root_pop)
                        (root_pop)
                        (root_pop)
                        (root_pop)
                        (root_pop)
                        (root_pop)
                        state))))))))))))
(defn register-all-pairs-step [pairs idx n ftable func-idx]
  (if (>= idx n)
    (make-register-pairs-state 1 idx ftable func-idx)
    (do
      (root_push pairs)
      (root_push ftable)
      (let [pair (vector-get pairs idx)]
        (do
          (root_push pair)
          (let [decls (vector-get pair 1)]
            (do
              (root_push decls)
              (let [result (register-defns-chunked decls 0 (vector-length decls) ftable func-idx)]
                (do
                  (root_push result)
                  (let [next-ftable (vector-get result 2)
                    next-func-idx (vector-get result 3)
                    next-state (make-register-pairs-state 0 (+ idx 1) next-ftable next-func-idx)]
                    (do
                      (root_pop)
                      (root_pop)
                      (root_pop)
                      (root_pop)
                      (root_pop)
                      next-state)))))))))))
(defn continue-register-all-pairs-step [pairs n state]
  (if (= (vector-get state 0) 1)
    state
    (do
      (root_push pairs)
      (root_push state)
      (let [result (register-all-pairs-step pairs (vector-get state 1) n (vector-get state 2) (vector-get state 3))]
        (do
          (root_pop)
          (root_pop)
          result)))))

(defn continue-register-all-pairs-step-times [pairs n remaining state]
  (if (= remaining 0)
    state
    (if (= (vector-get state 0) 1)
      state
      (do
        (root_push pairs)
        (root_push state)
        (let [next-state (continue-register-all-pairs-step pairs n state)]
          (do
            (root_push next-state)
            (let [result (continue-register-all-pairs-step-times pairs n (- remaining 1) next-state)]
              (do
                (root_pop)
                (root_pop)
                (root_pop)
                result))))))))

(defn register-all-pairs-step-8 [pairs idx n ftable func-idx]
  (do
    (root_push pairs)
    (let [state (register-all-pairs-step pairs idx n ftable func-idx)]
      (do
        (root_push state)
        (let [result (continue-register-all-pairs-step-times pairs n 7 state)]
          (do
            (root_pop)
            (root_pop)
            result))))))

(defn continue-register-all-pairs-step-8 [pairs n state]
  (if (= (vector-get state 0) 1)
    state
    (do
      (root_push pairs)
      (root_push state)
      (let [result (register-all-pairs-step-8 pairs (vector-get state 1) n (vector-get state 2) (vector-get state 3))]
        (do
          (root_pop)
          (root_pop)
          result)))))

(defn register-all-pairs-step-64 [pairs idx n ftable func-idx]
  (do
    (root_push pairs)
    (let [state (register-all-pairs-step pairs idx n ftable func-idx)]
      (do
        (root_push state)
        (let [result (continue-register-all-pairs-step-times pairs n 63 state)]
          (do
            (root_pop)
            (root_pop)
            result))))))

(defn continue-register-all-pairs-step-64 [pairs n state]
  (if (= (vector-get state 0) 1)
    state
    (do
      (root_push pairs)
      (root_push state)
      (let [next-state (register-all-pairs-step-64 pairs (vector-get state 1) n (vector-get state 2) (vector-get state 3))]
        (do
          (root_push next-state)
          (let [result (continue-register-all-pairs-step-64 pairs n next-state)]
            (do
              (root_pop)
              (root_pop)
              (root_pop)
              result)))))))
(defn write-register-state-progress-ref [state-ref done next-idx next-ftable next-func-idx]
  (do
    (print 9000000110)
    (print done)
    (print next-idx)
    (print next-func-idx)
    (print (vector-length next-ftable))
    (print 9000000117)
    (print state-ref)
    (print (ref-get state-ref))
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
                    (print 9000000111)
                    (print (vector-length with-done))
                    (print (vector-get with-done 0))
                    (let [with-idx (vector-push with-done (ref-get next-idx-ref))]
                      (do
                        (root_set base-slot with-idx)
                        (print 9000000112)
                        (print (vector-length with-idx))
                        (print (vector-get with-idx 0))
                        (print (vector-get with-idx 1))
                        (let [with-ftable (vector-push with-idx next-ftable)]
                          (do
                            (root_set base-slot with-ftable)
                            (print 9000000113)
                            (print (vector-length with-ftable))
                            (print (vector-get with-ftable 0))
                            (print (vector-get with-ftable 1))
                            (print (vector-length (vector-get with-ftable 2)))
                            (let [state (vector-push with-ftable (ref-get next-func-idx-ref))]
                              (do
                                (print 9000000114)
                                (print (vector-length state))
                                (print (vector-get state 0))
                                (print (vector-get state 1))
                                (print (vector-get state 3))
                                (print (vector-length (vector-get state 2)))
                                (print 9000000115)
                                (print (ref-get done-ref))
                                (print (ref-get next-idx-ref))
                                (print (ref-get next-func-idx-ref))
                                (root_set base-slot state)
                                (ref-set state-ref state)
                                (print 9000000118)
                                (print state-ref)
                                (print (ref-get state-ref))
                                (print (vector-length (ref-get state-ref)))
                                (print (vector-get (ref-get state-ref) 0))
                                (print (vector-get (ref-get state-ref) 1))
                                (print (vector-get (ref-get state-ref) 3))
                                (print (vector-length (vector-get (ref-get state-ref) 2)))
                                (root_pop)
                                (root_pop)
                                (root_pop)
                                (root_pop)
                                (root_pop)
                                (print 9000000116)
                                (print (vector-length state))
                                (print (vector-get state 0))
                                (print (vector-get state 1))
                                (print (vector-get state 3))
                                (print (vector-length (vector-get state 2)))
                                (root_pop)
                                0))))))))))))))))

(defn make-register-state-progress [done next-idx next-ftable next-func-idx]
  (let [state-ref (ref-new 0)]
    (do
      (root_push state-ref)
      (write-register-state-progress-ref state-ref done next-idx next-ftable next-func-idx)
      (let [state (ref-get state-ref)]
        (do
          (root_pop)
          state)))))

(defn register-defns-step-progress [decls idx n ftable func-idx]
  (if (>= idx n)
    (do
      (print 9000000090)
      (print idx)
      (print n)
      (print func-idx)
      (let [done-state-ref (ref-new 0)]
        (do
          (root_push done-state-ref)
          (print 9000000102)
          (print done-state-ref)
          (print (ref-get done-state-ref))
          (write-register-state-progress-ref done-state-ref 1 idx ftable func-idx)
          (print 9000000103)
          (print done-state-ref)
          (print (ref-get done-state-ref))
          (print (vector-length (ref-get done-state-ref)))
          (print (vector-get (ref-get done-state-ref) 0))
          (print (vector-get (ref-get done-state-ref) 1))
          (print (vector-get (ref-get done-state-ref) 3))
          (print (vector-length (vector-get (ref-get done-state-ref) 2)))
          (let [done-state (ref-get done-state-ref)]
            (do
              (root_pop)
              done-state)))))
    (do
      (root_push decls)
      (root_push ftable)
      (print 9000000091)
      (print idx)
      (print n)
      (print func-idx)
      (let [decl (vector-get decls idx)]
        (do
          (root_push decl)
          (print 9000000092)
          (print (vector-get decl 0))
          (print (vector-length decl))
          (if (= (vector-get decl 0) 20)
            (let [name-hash (vector-get decl 1)]
              (do
                (print 9000000093)
                (print name-hash)
                (print func-idx)
                (let [next-ftable (ftable-register ftable name-hash func-idx)]
                  (do
                    (root_push next-ftable)
                    (print 9000000094)
                    (print name-hash)
                    (let [defn-state-ref (ref-new 0)]
                      (do
                        (root_push defn-state-ref)
                        (print 9000000104)
                        (print defn-state-ref)
                        (print (ref-get defn-state-ref))
                        (write-register-state-progress-ref defn-state-ref 0 (+ idx 1) next-ftable (+ func-idx 1))
                        (print 9000000105)
                        (print defn-state-ref)
                        (print (ref-get defn-state-ref))
                        (print (vector-length (ref-get defn-state-ref)))
                        (print (vector-get (ref-get defn-state-ref) 0))
                        (print (vector-get (ref-get defn-state-ref) 1))
                        (print (vector-get (ref-get defn-state-ref) 3))
                        (print (vector-length (vector-get (ref-get defn-state-ref) 2)))
                        (let [defn-state (ref-get defn-state-ref)]
                          (do
                            (root_push defn-state)
                            (print 9000000095)
                            (print (vector-get defn-state 0))
                            (print (vector-get defn-state 1))
                            (print (vector-get defn-state 3))
                            (root_pop)
                            (root_pop)
                            (root_pop)
                            (root_pop)
                            (root_pop)
                            (root_pop)
                            defn-state))))))))
            (let [non-defn-state-ref (ref-new 0)]
              (do
                (root_push non-defn-state-ref)
                (print 9000000106)
                (print non-defn-state-ref)
                (print (ref-get non-defn-state-ref))
                (write-register-state-progress-ref non-defn-state-ref 0 (+ idx 1) ftable func-idx)
                (print 9000000107)
                (print non-defn-state-ref)
                (print (ref-get non-defn-state-ref))
                (print (vector-length (ref-get non-defn-state-ref)))
                (print (vector-get (ref-get non-defn-state-ref) 0))
                (print (vector-get (ref-get non-defn-state-ref) 1))
                (print (vector-get (ref-get non-defn-state-ref) 3))
                (print (vector-length (vector-get (ref-get non-defn-state-ref) 2)))
                (let [non-defn-state (ref-get non-defn-state-ref)]
                  (do
                    (root_push non-defn-state)
                    (print 9000000096)
                    (print (vector-get non-defn-state 0))
                    (print (vector-get non-defn-state 1))
                    (print (vector-get non-defn-state 3))
                    (root_pop)
                    (root_pop)
                    (root_pop)
                    (root_pop)
                    (root_pop)
                    non-defn-state))))))))))

(defn register-defns-chunked-progress [decls idx n ftable func-idx]
  (do
    (root_push decls)
    (root_push ftable)
    (print 9000000097)
    (print idx)
    (print n)
    (print func-idx)
    (let [state0 (register-defns-step-progress decls idx n ftable func-idx)]
      (do
        (root_push state0)
        (print 9000000098)
        (print (vector-get state0 0))
        (print (vector-get state0 1))
        (print (vector-get state0 3))
        (let [state0-next-idx (vector-get state0 1)
          state0-ftable (vector-get state0 2)
          state0-next-func-idx (vector-get state0 3)]
          (do
            (root_push state0-ftable)
            (let [result (register-defns-chunked decls state0-next-idx n state0-ftable state0-next-func-idx)]
              (do
                (root_push result)
                (print 9000000099)
                (print (vector-length result))
                (print (vector-get result 1))
                (print (vector-get result 3))
                (root_pop)
                (root_pop)
                (root_pop)
                (root_pop)
                (root_pop)
                result))))))))

(defn register-all-pairs-step-progress [pairs idx n ftable func-idx]
  (if (>= idx n)
    (do
      (print 9000000080)
      (print idx)
      (print n)
      (make-register-pairs-state 1 idx ftable func-idx))
    (do
      (root_push pairs)
      (root_push ftable)
      (print 9000000081)
      (print idx)
      (print n)
      (let [pair (vector-get pairs idx)]
        (do
          (root_push pair)
          (print 9000000082)
          (print (vector-length pair))
          (let [decls (vector-get pair 1)]
            (do
              (root_push decls)
              (print 9000000083)
              (print (vector-length decls))
              (let [result (register-defns-chunked-progress decls 0 (vector-length decls) ftable func-idx)]
                (do
                  (root_push result)
                  (print 9000000084)
                  (print (vector-length result))
                  (print (vector-get result 1))
                  (print (vector-get result 3))
                  (let [next-ftable (vector-get result 2)
                    next-func-idx (vector-get result 3)
                    next-state (make-register-pairs-state 0 (+ idx 1) next-ftable next-func-idx)]
                    (do
                      (root_push next-state)
                      (print 9000000085)
                      (print (vector-get next-state 0))
                      (print (vector-get next-state 1))
                      (print (vector-get next-state 3))
                      (root_pop)
                      (root_pop)
                      (root_pop)
                      (root_pop)
                      (root_pop)
                      (root_pop)
                      next-state)))))))))))

(defn register-all-pairs-progress-loop [pairs n state]
  (if (= (vector-get state 0) 1)
    state
    (do
      (root_push pairs)
      (root_push state)
      (print 9000000100)
      (print (vector-get state 1))
      (print n)
      (print (vector-get state 3))
      (let [next-state (register-all-pairs-step-progress pairs (vector-get state 1) n (vector-get state 2) (vector-get state 3))]
        (do
          (root_push next-state)
          (print 9000000101)
          (print (vector-get next-state 0))
          (print (vector-get next-state 1))
          (print (vector-get next-state 3))
          (let [result (register-all-pairs-progress-loop pairs n next-state)]
            (do
              (root_pop)
              (root_pop)
              (root_pop)
              result)))))))

(defn register-all-pairs-progress [pairs idx n ftable func-idx]
  (do
    (root_push pairs)
    (root_push ftable)
    (print 9000000086)
    (print idx)
    (print n)
    (let [state0 (register-all-pairs-step-progress pairs idx n ftable func-idx)]
      (do
        (root_push state0)
        (print 9000000087)
        (print (vector-get state0 0))
        (print (vector-get state0 1))
        (print (vector-get state0 3))
        (let [final-state (register-all-pairs-progress-loop pairs n state0)]
          (do
            (root_push final-state)
            (let [next-ftable (vector-get final-state 2)
              next-func-idx (vector-get final-state 3)]
              (do
                (root_push next-ftable)
                (let [with-ftable (push-object-vector (vector-new 2) next-ftable)]
                  (do
                    (root_push with-ftable)
                    (let [result (vector-push with-ftable next-func-idx)]
                      (do
                        (root_pop)
                        (root_pop)
                        (root_pop)
                        (root_pop)
                        (root_pop)
                        (root_pop)
                        result))))))))))))
(defn register-all-pairs [pairs idx n ftable func-idx]
  (let [state (continue-register-all-pairs-step-64 pairs n (register-all-pairs-step-64 pairs idx n ftable func-idx))]
    (do
      (root_push state)
      (let [next-ftable (vector-get state 2)
        next-func-idx (vector-get state 3)]
        (do
          (root_push next-ftable)
          (let [with-ftable (push-object-vector (vector-new 2) next-ftable)]
            (do
              (root_push with-ftable)
              (let [result (vector-push with-ftable next-func-idx)]
                (do
                  (root_pop)
                  (root_pop)
                  (root_pop)
                  result)))))))))
(defn compile-src-decl-pairs-step [pairs idx n ftable data-ref functions]
  (if (>= idx n)
    (make-pairs-step-state 1 idx functions)
    (do
      (root_push pairs)
      (root_push ftable)
      (root_push data-ref)
      (root_push functions)
      (let [pair (vector-get pairs idx)]
        (do
          (root_push pair)
          (let [src (vector-get pair 0)
            decls (vector-get pair 1)
            updated-functions (compile-source-defn-functions-chunked decls 0 (vector-length decls) src ftable data-ref functions)]
            (do
              (root_push updated-functions)
              (let [next-state (make-pairs-step-state 0 (+ idx 1) updated-functions)]
                (do
                  (root_pop)
                  (root_pop)
                  (root_pop)
                  (root_pop)
                  (root_pop)
                  (root_pop)
                  next-state)))))))))
(defn continue-compile-src-decl-pairs-step [pairs n ftable data-ref state] (if (= (vector-get state 0) 1) state (compile-src-decl-pairs-step pairs (vector-get state 1) n ftable data-ref (vector-get state 2))))
(defn compile-src-decl-pairs-step-8 [pairs idx n ftable data-ref functions] (let [step1 (compile-src-decl-pairs-step pairs idx n ftable data-ref functions) step2 (continue-compile-src-decl-pairs-step pairs n ftable data-ref step1) step3 (continue-compile-src-decl-pairs-step pairs n ftable data-ref step2) step4 (continue-compile-src-decl-pairs-step pairs n ftable data-ref step3) step5 (continue-compile-src-decl-pairs-step pairs n ftable data-ref step4) step6 (continue-compile-src-decl-pairs-step pairs n ftable data-ref step5) step7 (continue-compile-src-decl-pairs-step pairs n ftable data-ref step6) step8 (continue-compile-src-decl-pairs-step pairs n ftable data-ref step7)] step8))
(defn continue-compile-src-decl-pairs-step-8 [pairs n ftable data-ref state]
  (if (= (vector-get state 0) 1)
    state
    (do
      (root_push pairs)
      (root_push ftable)
      (root_push data-ref)
      (root_push state)
      (let [result (compile-src-decl-pairs-step-8 pairs (vector-get state 1) n ftable data-ref (vector-get state 2))]
        (do
          (root_pop)
          (root_pop)
          (root_pop)
          (root_pop)
          result)))))
(defn compile-src-decl-pairs-step-64 [pairs idx n ftable data-ref functions] (let [step1 (compile-src-decl-pairs-step-8 pairs idx n ftable data-ref functions) step2 (continue-compile-src-decl-pairs-step-8 pairs n ftable data-ref step1) step3 (continue-compile-src-decl-pairs-step-8 pairs n ftable data-ref step2) step4 (continue-compile-src-decl-pairs-step-8 pairs n ftable data-ref step3) step5 (continue-compile-src-decl-pairs-step-8 pairs n ftable data-ref step4) step6 (continue-compile-src-decl-pairs-step-8 pairs n ftable data-ref step5) step7 (continue-compile-src-decl-pairs-step-8 pairs n ftable data-ref step6) step8 (continue-compile-src-decl-pairs-step-8 pairs n ftable data-ref step7)] step8))
(defn continue-compile-src-decl-pairs-step-64 [pairs n ftable data-ref state]
  (if (= (vector-get state 0) 1)
    state
    (do
      (root_push pairs)
      (root_push ftable)
      (root_push data-ref)
      (root_push state)
      (let [next-state (compile-src-decl-pairs-step-64 pairs (vector-get state 1) n ftable data-ref (vector-get state 2))]
        (do
          (root_push next-state)
          (let [result (continue-compile-src-decl-pairs-step-64 pairs n ftable data-ref next-state)]
            (do
              (root_pop)
              (root_pop)
              (root_pop)
              (root_pop)
              (root_pop)
              result)))))))
(defn compile-all-src-decl-pairs-chunked [pairs idx n ftable data-ref functions]
  (vector-get
    (continue-compile-src-decl-pairs-step-64 pairs n ftable data-ref (compile-src-decl-pairs-step-64 pairs idx n ftable data-ref functions))
    2))
(defn compile-all-src-decl-pairs [pairs idx n ftable data-ref functions]
  (if (>= idx n)
    functions
    (do
      (root_push pairs)
      (root_push ftable)
      (root_push data-ref)
      (root_push functions)
      (let [pair (vector-get pairs idx)]
        (do
          (root_push pair)
          (let [src (vector-get pair 0)
            decls (vector-get pair 1)
            updated-functions (compile-defn-functions-with-source decls 0 (vector-length decls) src ftable data-ref functions)]
            (do
              (root_push updated-functions)
              (let [result (compile-all-src-decl-pairs pairs (+ idx 1) n ftable data-ref updated-functions)]
                (do
                  (root_pop)
                  (root_pop)
                  (root_pop)
                  (root_pop)
                  (root_pop)
                  (root_pop)
                  result)))))))))
(defn compile-defn-functions-step-progress-probe [decls idx n src ftable data-ref functions]
  (if (>= idx n)
    (make-compile-step-state 1 idx functions)
    (let [decls-slot (root_push decls)
      src-slot (root_push src)
      ftable-slot (root_push ftable)
      data-slot (root_push data-ref)
      functions-slot (root_push functions)
      decl (vector-get decls idx)]
      (do
        (print 170)
        (print idx)
        (print (vector-get decl 0))
        (if (= (vector-get decl 0) 20)
          (do
            (root_push decl)
            (print 172)
            (print idx)
            (print 175)
            (print idx)
            (let [source-ir (compile-defn-with-source-probe decl src ftable data-ref)]
              (do
                (root_push source-ir)
                (print 176)
                (print idx)
                (print (vector-length source-ir))
                (let [ir (if (> (vector-length source-ir) 0) source-ir (compile-defn-with-ftable decl ftable))]
                  (do
                    (root_push ir)
                    (print 177)
                    (print idx)
                    (print (vector-length ir))
                    (let [local-max (max-local-slot ir 0 (vector-length ir) 0)
                      final-param-count (vector-get decl 2)
                      local-count (if (> local-max final-param-count) (- local-max final-param-count) 0)
                      compiled-fn (make-function-meta final-param-count local-count ir)]
                      (do
                        (root_push compiled-fn)
                        (print 173)
                        (print idx)
                        (let [next-functions (push-object-vector functions compiled-fn)]
                          (do
                            (root_push next-functions)
                            (print 174)
                            (print idx)
                            (print 171)
                            (print idx)
                            (print (vector-length next-functions))
                            (let [result (make-compile-step-state 0 (+ idx 1) next-functions)]
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
                                result)))))))))))
          (do
            (print 171)
            (print idx)
            (print (vector-length functions))
            (let [result (make-compile-step-state 0 (+ idx 1) functions)]
              (do
                (root_pop)
                (root_pop)
                (root_pop)
                (root_pop)
                (root_pop)
                result))))))))
(defn continue-compile-defn-functions-step-progress-probe [decls n src ftable data-ref state]
  (if (= (vector-get state 0) 1)
    state
    (compile-defn-functions-step-progress-probe decls (vector-get state 1) n src ftable data-ref (vector-get state 2))))
(defn continue-compile-defn-functions-step-times-progress-probe [decls n src ftable data-ref remaining state]
  (if (= remaining 0)
    state
    (if (= (vector-get state 0) 1)
      state
      (do
        (root_push decls)
        (root_push src)
        (root_push ftable)
        (root_push data-ref)
        (root_push state)
        (let [next-state (continue-compile-defn-functions-step-progress-probe decls n src ftable data-ref state)]
          (do
            (root_push next-state)
            (let [result (continue-compile-defn-functions-step-times-progress-probe decls n src ftable data-ref (- remaining 1) next-state)]
              (do
                (root_pop)
                (root_pop)
                (root_pop)
                (root_pop)
                (root_pop)
                (root_pop)
                result))))))))
(defn compile-defn-functions-step-64-progress-probe [decls idx n src ftable data-ref functions]
  (do
    (root_push decls)
    (root_push src)
    (root_push ftable)
    (root_push data-ref)
    (let [state (compile-defn-functions-step-progress-probe decls idx n src ftable data-ref functions)]
      (do
        (root_push state)
        (let [result (continue-compile-defn-functions-step-times-progress-probe decls n src ftable data-ref 63 state)]
          (do
            (root_pop)
            (root_pop)
            (root_pop)
            (root_pop)
            (root_pop)
            result))))))
(defn continue-compile-defn-functions-step-64-progress-probe [decls n src ftable data-ref state]
  (if (= (vector-get state 0) 1)
    state
    (do
      (root_push decls)
      (root_push src)
      (root_push ftable)
      (root_push data-ref)
      (root_push state)
      (let [next-state (compile-defn-functions-step-64-progress-probe decls (vector-get state 1) n src ftable data-ref (vector-get state 2))]
        (do
          (root_push next-state)
          (let [result (continue-compile-defn-functions-step-64-progress-probe decls n src ftable data-ref next-state)]
            (do
              (root_pop)
              (root_pop)
              (root_pop)
              (root_pop)
              (root_pop)
              (root_pop)
              result)))))))
(defn compile-source-defn-functions-chunked-progress-probe [decls idx n src ftable data-ref functions]
  (vector-get
    (continue-compile-defn-functions-step-64-progress-probe
      decls
      n
      src
      ftable
      data-ref
      (compile-defn-functions-step-64-progress-probe decls idx n src ftable data-ref functions))
    2))
(defn compile-all-src-decl-pairs-chunked-progress [pairs idx n ftable data-ref functions]
  (if (>= idx n)
    functions
    (do
      (root_push pairs)
      (root_push ftable)
      (root_push data-ref)
      (root_push functions)
      (let [pair (vector-get pairs idx)]
        (do
          (root_push pair)
          (let [src (vector-get pair 0)
            decls (vector-get pair 1)]
            (do
              (print 160)
              (print idx)
              (print (vector-length decls))
              (print 9000000130)
              (print idx)
              (print (vector-length decls))
              (print (vector-length (ref-get data-ref)))
              (let [updated-functions (compile-source-defn-functions-chunked-progress-probe decls 0 (vector-length decls) src ftable data-ref functions)]
                (do
                  (root_push updated-functions)
                  (print 161)
                  (print idx)
                  (print (vector-length updated-functions))
                  (print 9000000131)
                  (print idx)
                  (print (vector-length updated-functions))
                  (print (vector-length (ref-get data-ref)))
                  (let [result (compile-all-src-decl-pairs-chunked-progress pairs (+ idx 1) n ftable data-ref updated-functions)]
                    (do
                      (root_pop)
                      (root_pop)
                      (root_pop)
                      (root_pop)
                      (root_pop)
                      (root_pop)
                      result)))))))))))
(defn compile-string-literal-with-source-probe [node source instrs data-ref]
  (do
    (root_push node)
    (root_push source)
    (root_push instrs)
    (root_push data-ref)
    (print 9000000140)
    (print (vector-length (ref-get data-ref)))
    (let [result (compile-string-literal-with-source node source instrs data-ref)]
      (do
        (root_push result)
        (print 9000000141)
        (print (vector-length result))
        (print (vector-length (ref-get data-ref)))
        (root_pop)
        (root_pop)
        (root_pop)
        (root_pop)
        (root_pop)
        result))))
(defn compile-let-chain-with-source-probe [node source env ftable instrs data-ref rooted-count]
  (do
    (root_push node)
    (root_push source)
    (root_push env)
    (root_push ftable)
    (root_push instrs)
    (root_push data-ref)
    (let [name-hash (vector-get node 1)
      init-expr (vector-get node 2)
      body-expr (vector-get node 3)
      init-root (alloc-root-needed init-expr)]
      (do
        (print 180)
        (print rooted-count)
        (print (vector-get init-expr 0))
        (print (vector-get body-expr 0))
        (let [init-instrs (compile-expr-with-source-probe init-expr source env ftable instrs data-ref rooted-count)]
          (do
            (root_push init-instrs)
            (print 181)
            (print (vector-length init-instrs))
            (let [new-idx (+ 1 (map-size env))
              next-instrs1 (emit-to init-instrs (op-local-set) new-idx)
              next-instrs2 (maybe-root-push-drop next-instrs1 init-root new-idx)]
              (do
                (root_push next-instrs2)
                (let [next-env (env-bind env name-hash new-idx)]
                  (do
                    (root_push next-env)
                    (print 182)
                    (print (+ rooted-count init-root))
                    (let [result
                      (if (= (vector-get body-expr 0) (tag-let))
                        (compile-let-chain-with-source-probe body-expr source next-env ftable next-instrs2 data-ref (+ rooted-count init-root))
                        (let [body-instrs (compile-expr-with-source-probe body-expr source next-env ftable next-instrs2 data-ref (+ rooted-count init-root))]
                          (do
                            (root_push body-instrs)
                            (let [final-instrs (emit-root-pop-drops body-instrs (+ rooted-count init-root))]
                              (do
                                (root_pop)
                                final-instrs)))))]
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
                        result))))))))))))
(defn compile-if-with-source-probe [node source env ftable instrs data-ref rooted-count]
  (do
    (print 186)
    (print rooted-count)
    (print (vector-get node 0))
    (root_push node)
    (root_push source)
    (root_push env)
    (root_push ftable)
    (root_push data-ref)
    (print 190)
    (print (vector-get (vector-get node 1) 0))
    (let [instrs1 (compile-expr-with-source-probe (vector-get node 1) source env ftable instrs data-ref rooted-count)]
      (do
        (root_push instrs1)
        (print 191)
        (print (vector-length instrs1))
        (let [instrs2 (emit-to instrs1 (op-if) 0)]
          (do
            (root_push instrs2)
            (print 192)
            (print (vector-get (vector-get node 2) 0))
            (let [instrs3 (compile-expr-with-source-probe (vector-get node 2) source env ftable instrs2 data-ref rooted-count)]
              (do
                (root_push instrs3)
                (let [instrs4 (emit-to instrs3 (op-else) 0)]
                  (do
                    (root_push instrs4)
                    (print 193)
                    (print (vector-get (vector-get node 3) 0))
                    (let [instrs5 (compile-expr-with-source-probe (vector-get node 3) source env ftable instrs4 data-ref rooted-count)]
                      (do
                        (root_push instrs5)
                        (let [result (emit-to instrs5 (op-end) 0)]
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
                            result))))))))))))))
(defn compile-simple-builtin-with-source-probe [node source env ftable instrs data-ref bop rooted-count]
  (do
    (print 194)
    (print bop)
    (print (vector-get (vector-get node 3) 0))
    (root_push node)
    (root_push source)
    (root_push env)
    (root_push ftable)
    (root_push data-ref)
    (let [instrs1 (compile-expr-with-source-probe (vector-get node 3) source env ftable instrs data-ref rooted-count)]
      (do
        (root_push instrs1)
        (print 195)
        (print (vector-length instrs1))
        (let [result
          (if (unary-builtin-op bop)
            (emit-unary-builtin-with-source instrs1 bop env)
            (compile-binary-or-ternary-builtin-with-source node source env ftable instrs1 data-ref bop))]
          (do
            (root_push result)
            (print 196)
            (print (vector-length result))
            (root_pop)
            (root_pop)
            (root_pop)
            (root_pop)
            (root_pop)
            (root_pop)
            (root_pop)
            result))))))
(defn print-user-call-arg-instrs-lengths [arg-instrs-list idx count]
  (if (>= idx count)
    0
    (let [arg-instrs (vector-get arg-instrs-list idx)]
      (do
        (root_push arg-instrs)
        (print 203)
        (print idx)
        (print (vector-length arg-instrs))
        (if (= idx 2)
          (print-instr-vector-probe arg-instrs 0 (vector-length arg-instrs))
          0)
        (print 205)
        (print idx)
        (print (max-local-slot arg-instrs 0 (vector-length arg-instrs) 0))
        (root_pop)
        (print-user-call-arg-instrs-lengths arg-instrs-list (+ idx 1) count)))))
(defn print-instr-vector-probe [instrs idx count]
  (if (>= idx count)
    0
    (let [instr (vector-get instrs idx)]
      (do
        (print 206)
        (print idx)
        (root_push instr)
        (print 209)
        (print (vector-length instr))
        (print 207)
        (print (vector-get instr 0))
        (print 208)
        (print (vector-get instr 1))
        (root_pop)
        (print-instr-vector-probe instrs (+ idx 1) count)))))
(defn compile-user-call-with-source-probe [node source env ftable instrs data-ref func-hash arg-count]
  (do
    (print 210)
    (print arg-count)
    (let [node-slot (root_push node)
      source-slot (root_push source)
      env-slot (root_push env)
      ftable-slot (root_push ftable)
      instrs-slot (root_push instrs)
      data-slot (root_push data-ref)
      func-idx (ftable-lookup ftable func-hash)]
      (do
        (print 211)
        (print func-idx)
        (let [
          arg-instrs-list (compile-user-call-arg-instrs-with-source node source env ftable 0 arg-count (vector-new 8) data-ref)]
          (do
            (print 212)
            (print (vector-length arg-instrs-list))
            (root_push arg-instrs-list)
            0
            (let [temp-base (max-root-temp-base-list env arg-instrs-list arg-count)]
              (do
                (print 204)
                (print temp-base)
                (let [instrs1 (emit-user-call-args node arg-instrs-list 0 arg-count temp-base instrs)]
                  (do
                    (root_push instrs1)
                    (print 199)
                    (print (vector-length instrs1))
                    (let [instrs2 (emit-user-call-arg-gets 0 arg-count temp-base instrs1)]
                      (do
                        (root_push instrs2)
                        (print 200)
                        (print (vector-length instrs2))
                        (let [instrs3 (emit-to instrs2 (op-call) func-idx)]
                          (do
                            (root_push instrs3)
                            (print 201)
                            (print (vector-length instrs3))
                            (let [result (emit-user-call-root-pops node (- arg-count 1) instrs3)]
                              (do
                                (print 202)
                                (print (vector-length result))
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
                                result)))))))))))))
    )))
(defn compile-apply-with-source-probe [node source env ftable instrs data-ref rooted-count]
  (let [func-node (vector-get node 1)
    arg-count (vector-get node 2)]
    (let [func-tag (vector-get func-node 0)
      func-hash (if (= func-tag 4) (vector-get func-node 1) 0)]
      (let [bop (builtin-opcode func-hash)]
        (do
          (print 197)
          (print bop)
          (print arg-count)
          (if (> bop 0)
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
                        (if (and (source-neutral-ftable-builtin-op bop) (apply-args-safe-for-ftable node 0 arg-count))
                          (compile-builtin-apply-with-ftable node env ftable instrs bop)
                          (if (source-builtin-map-op bop)
                            (compile-map-builtin-with-source node source env ftable instrs data-ref bop)
                            (compile-simple-builtin-with-source-probe node source env ftable instrs data-ref bop rooted-count)))))))))
            (compile-user-call-with-source-probe node source env ftable instrs data-ref func-hash arg-count)))))))
(defn compile-expr-with-source-probe-dispatch [node source env ftable instrs data-ref rooted-count]
  (if (= (vector-get node 0) (tag-lit-string))
    (compile-string-literal-with-source-probe node source instrs data-ref)
    (if (= (vector-get node 0) (tag-let))
      (compile-let-chain-with-source-probe node source env ftable instrs data-ref rooted-count)
      (if (= (vector-get node 0) (tag-if))
        (compile-if-with-source-probe node source env ftable instrs data-ref rooted-count)
        (if (= (vector-get node 0) (tag-apply))
          (compile-apply-with-source-probe node source env ftable instrs data-ref rooted-count)
          (do
            (print 9000000142)
            (print (vector-get node 0))
            (print (vector-length (ref-get data-ref)))
            (let [result (compile-expr-with-source node source env ftable instrs data-ref)]
              (do
                (root_push result)
                (print 9000000143)
                (print (vector-get node 0))
                (print (vector-length result))
                (print (vector-length (ref-get data-ref)))
                (root_pop)
                result))))))))
(defn compile-expr-with-source-probe [node source env ftable instrs data-ref rooted-count]
  (let [node-slot (root_push node)
    source-slot (root_push source)
    env-slot (root_push env)
    ftable-slot (root_push ftable)
    instrs-slot (root_push instrs)
    data-slot (root_push data-ref)]
    (do
      (print 185)
      (print rooted-count)
      (print (vector-get node 0))
      (let [result (compile-expr-with-source-probe-dispatch node source env ftable instrs data-ref rooted-count)]
        (do
          (root_pop)
          (root_pop)
          (root_pop)
          (root_pop)
          (root_pop)
          (root_pop)
          result)))))
(defn compile-defn-with-source-probe [node source ftable data-ref]
  (do
    (print 180)
    (root_push node)
    (root_push source)
    (root_push ftable)
    (root_push data-ref)
    (print 181)
    (let [param-count (vector-get node 2)
      body-idx (+ 3 param-count)
      body-expr (vector-get node body-idx)]
      (do
        (print 182)
        (print param-count)
        (print body-idx)
        (root_push body-expr)
        (let [env (bind-node-params node 3 0 param-count (env-new) 1)]
          (do
            (root_push env)
            (print 183)
            (let [instrs0 (vector-new 8)
              result (compile-expr-with-source-probe body-expr source env ftable instrs0 data-ref 0)]
              (do
                (print 184)
                (print (vector-length result))
                (root_pop)
                (root_pop)
                (root_pop)
                (root_pop)
                (root_pop)
                (root_pop)
                result))))))))
(defn compile-defn-functions-chunked-step-progress-debug [decls idx n src ftable data-ref functions]
  (if (>= idx n)
    functions
    (let [decls-slot (root_push decls)
      src-slot (root_push src)
      ftable-slot (root_push ftable)
      data-slot (root_push data-ref)
      functions-slot (root_push functions)]
      (let [decl (vector-get decls idx)]
        (do
          (print 170)
          (print idx)
          (print (vector-get decl 0))
          (if (= (vector-get decl 0) 20)
            (do
              (root_push decl)
              (print 172)
              (print idx)
              (print 175)
              (print idx)
              (let [param-count (vector-get decl 2)
                source-ir (compile-defn-with-source-probe decl src ftable data-ref)]
                (do
                  (root_push source-ir)
                  (print 176)
                  (print idx)
                  (print (vector-length source-ir))
                  (let [ir (if (> (vector-length source-ir) 0) source-ir (compile-defn-with-ftable decl ftable))]
                    (do
                      (root_push ir)
                      (print 177)
                      (print idx)
                      (print (vector-length ir))
                      (let [local-max (max-local-slot ir 0 (vector-length ir) 0)
                        local-count (if (> local-max param-count) (- local-max param-count) 0)
                        compiled-fn (make-function-meta param-count local-count ir)]
                        (do
                          (root_push compiled-fn)
                          (print 173)
                          (print idx)
                          (let [next-functions (push-object-vector functions compiled-fn)]
                            (do
                              (root_set functions-slot next-functions)
                              (print 174)
                              (print idx)
                              (print 171)
                              (print idx)
                              (print (vector-length next-functions))
                              (root_pop)
                              (root_pop)
                              (root_pop)
                              (root_pop)
                              (let [result (compile-defn-functions-chunked-step-progress-debug decls (+ idx 1) n src ftable data-ref next-functions)]
                                (do
                                  (root_pop)
                                  (root_pop)
                                  (root_pop)
                                  (root_pop)
                                  (root_pop)
                                  result)))))))))))
            (do
              (print 171)
              (print idx)
              (print (vector-length functions))
              (let [result (compile-defn-functions-chunked-step-progress-debug decls (+ idx 1) n src ftable data-ref functions)]
                (do
                  (root_pop)
                  (root_pop)
                  (root_pop)
                  (root_pop)
                  (root_pop)
                  result)))))))))
(defn compile-defn-functions-linear-with-source [decls idx n src ftable data-ref functions]
  (if (>= idx n)
    functions
    (do
      (root_push decls)
      (root_push src)
      (root_push ftable)
      (root_push data-ref)
      (root_push functions)
      (let [decl (vector-get decls idx)]
        (do
          (root_push decl)
          (let [next-functions (if (= (vector-get decl 0) 20)
              (let [compiled-fn (compile-defn-function-with-source decl src ftable data-ref)]
                (do
                  (root_push compiled-fn)
                  (let [updated-functions (push-object-vector functions compiled-fn)]
                    (do
                      (root_pop)
                      updated-functions))))
              functions)]
            (do
              (root_push next-functions)
              (let [result (compile-defn-functions-linear-with-source decls (+ idx 1) n src ftable data-ref next-functions)]
                (do
                  (root_pop)
                  (root_pop)
                  (root_pop)
                  (root_pop)
                  (root_pop)
                  (root_pop)
                  (root_pop)
                  result)))))))))
(defn compile-all-src-decl-pairs-linear [pairs idx n ftable data-ref functions]
  (if (>= idx n)
    functions
    (do
      (root_push pairs)
      (root_push ftable)
      (root_push data-ref)
      (root_push functions)
      (let [pair (vector-get pairs idx)]
        (do
          (root_push pair)
          (let [src (vector-get pair 0)
            decls (vector-get pair 1)
            updated-functions (compile-defn-functions-linear-with-source decls 0 (vector-length decls) src ftable data-ref functions)]
            (do
              (root_push updated-functions)
              (let [result (compile-all-src-decl-pairs-linear pairs (+ idx 1) n ftable data-ref updated-functions)]
                (do
                  (root_pop)
                  (root_pop)
                  (root_pop)
                  (root_pop)
                  (root_pop)
                  (root_pop)
                  result)))))))))
(defn compile-defn-functions-progress-debug [decls idx n src ftable data-ref functions]
  (if (>= idx n)
    functions
    (do
      (root_push decls)
      (root_push src)
      (root_push ftable)
      (root_push data-ref)
      (root_push functions)
      (let [decl (vector-get decls idx)]
        (do
          (root_push decl)
          (print 40)
          (print idx)
          (print (vector-get decl 0))
          (let [next-functions (if (= (vector-get decl 0) 20)
              (let [compiled-fn (compile-defn-function-with-source decl src ftable data-ref)]
                (do
                  (print 41)
                  (print idx)
                  (root_push compiled-fn)
                  (let [updated-functions (push-object-vector functions compiled-fn)]
                    (do
                      (print 42)
                      (print idx)
                      (print (vector-length updated-functions))
                      (root_pop)
                      updated-functions))))
              functions)]
            (do
              (print 43)
              (print idx)
              (root_push next-functions)
              (let [result (compile-defn-functions-progress-debug decls (+ idx 1) n src ftable data-ref next-functions)]
                (do
                  (root_pop)
                  (root_pop)
                  (root_pop)
                  (root_pop)
                  (root_pop)
                  (root_pop)
                  (root_pop)
                  result)))))))))
(defn compile-all-src-decl-pairs-progress-debug [pairs idx n ftable data-ref functions]
  (if (>= idx n)
    functions
    (do
      (root_push pairs)
      (root_push ftable)
      (root_push data-ref)
      (root_push functions)
      (let [pair (vector-get pairs idx)]
        (do
          (root_push pair)
          (let [src (vector-get pair 0)
            decls (vector-get pair 1)]
            (do
              (root_push src)
              (root_push decls)
              (print 29)
              (print idx)
              (print (string-length src))
              (print (vector-length decls))
              (let [updated-functions (compile-defn-functions-progress-debug decls 0 (vector-length decls) src ftable data-ref functions)]
                (do
                  (print 30)
                  (print idx)
                  (print (string-length src))
                  (print (vector-length decls))
                  (root_push updated-functions)
                  (let [result (compile-all-src-decl-pairs-progress-debug pairs (+ idx 1) n ftable data-ref updated-functions)]
                    (do
                      (root_pop)
                      (root_pop)
                      (root_pop)
                      (root_pop)
                      (root_pop)
                      (root_pop)
                      (root_pop)
                      (root_pop)
                      result)))))))))))
(defn make-print-step-state [done next-idx] (push-int-vector-local (push-int-vector-local (vector-new 2) done) next-idx))
(defn print-module-bytes-step [bytes idx count] (if (>= idx count) (make-print-step-state 1 idx) (let [value (vector-get bytes idx)] (do (print (if (< value 0) (+ value 256) value)) (make-print-step-state 0 (+ idx 1))))))
(defn continue-print-module-bytes-step [bytes count state] (if (= (vector-get state 0) 1) state (print-module-bytes-step bytes (vector-get state 1) count)))
(defn print-module-bytes-step-8 [bytes idx count] (let [step1 (print-module-bytes-step bytes idx count) step2 (continue-print-module-bytes-step bytes count step1) step3 (continue-print-module-bytes-step bytes count step2) step4 (continue-print-module-bytes-step bytes count step3) step5 (continue-print-module-bytes-step bytes count step4) step6 (continue-print-module-bytes-step bytes count step5) step7 (continue-print-module-bytes-step bytes count step6) step8 (continue-print-module-bytes-step bytes count step7)] step8))
(defn continue-print-module-bytes-step-8 [bytes count state] (if (= (vector-get state 0) 1) state (print-module-bytes-step-8 bytes (vector-get state 1) count)))
(defn print-module-bytes-step-64 [bytes idx count] (let [step1 (print-module-bytes-step-8 bytes idx count) step2 (continue-print-module-bytes-step-8 bytes count step1) step3 (continue-print-module-bytes-step-8 bytes count step2) step4 (continue-print-module-bytes-step-8 bytes count step3) step5 (continue-print-module-bytes-step-8 bytes count step4) step6 (continue-print-module-bytes-step-8 bytes count step5) step7 (continue-print-module-bytes-step-8 bytes count step6) step8 (continue-print-module-bytes-step-8 bytes count step7)] step8))
(defn continue-print-module-bytes-step-64 [bytes count state] (if (= (vector-get state 0) 1) state (print-module-bytes-step-64 bytes (vector-get state 1) count)))
(defn print-module-bytes-step-512 [bytes idx count] (let [step1 (print-module-bytes-step-64 bytes idx count) step2 (continue-print-module-bytes-step-64 bytes count step1) step3 (continue-print-module-bytes-step-64 bytes count step2) step4 (continue-print-module-bytes-step-64 bytes count step3) step5 (continue-print-module-bytes-step-64 bytes count step4) step6 (continue-print-module-bytes-step-64 bytes count step5) step7 (continue-print-module-bytes-step-64 bytes count step6) step8 (continue-print-module-bytes-step-64 bytes count step7)] step8))
(defn continue-print-module-bytes-step-512 [bytes count state] (if (= (vector-get state 0) 1) state (print-module-bytes-step-512 bytes (vector-get state 1) count)))
(defn print-module-bytes-step-4096 [bytes idx count] (let [step1 (print-module-bytes-step-512 bytes idx count) step2 (continue-print-module-bytes-step-512 bytes count step1) step3 (continue-print-module-bytes-step-512 bytes count step2) step4 (continue-print-module-bytes-step-512 bytes count step3) step5 (continue-print-module-bytes-step-512 bytes count step4) step6 (continue-print-module-bytes-step-512 bytes count step5) step7 (continue-print-module-bytes-step-512 bytes count step6) step8 (continue-print-module-bytes-step-512 bytes count step7)] step8))
(defn continue-print-module-bytes-step-4096 [bytes count state] (if (= (vector-get state 0) 1) state (print-module-bytes-step-4096 bytes (vector-get state 1) count)))
(defn print-module-bytes-step-32768 [bytes idx count] (let [step1 (print-module-bytes-step-4096 bytes idx count) step2 (continue-print-module-bytes-step-4096 bytes count step1) step3 (continue-print-module-bytes-step-4096 bytes count step2) step4 (continue-print-module-bytes-step-4096 bytes count step3) step5 (continue-print-module-bytes-step-4096 bytes count step4) step6 (continue-print-module-bytes-step-4096 bytes count step5) step7 (continue-print-module-bytes-step-4096 bytes count step6) step8 (continue-print-module-bytes-step-4096 bytes count step7)] step8))
(defn print-module-bytes-loop [bytes idx count] (let [step (print-module-bytes-step-32768 bytes idx count)] (if (= (vector-get step 0) 1) 0 (print-module-bytes-loop bytes (vector-get step 1) count))))
(defn print-wasm-module [bytes] (let [count (vector-length bytes)] (do (print count) (print-module-bytes-loop bytes 0 count) 0)))
(defn append-section-bytes [dst section] (append-byte-vector-chunked dst section 0 (vector-length section)))
(defn print-ir-pairs [ir idx count] (if (>= idx count) 0 (let [instr (vector-get ir idx)] (do (print (vector-get instr 0)) (print (vector-get instr 1)) (print-ir-pairs ir (+ idx 1) count)))))
(defn print-token-triples [spans idx count] (if (>= idx count) 0 (do (print (span-kind spans idx)) (print (span-start spans idx)) (print (span-end spans idx)) (print-token-triples spans (+ idx 1) count))))
(defn build-wasm-bytes-wasi [functions data]
  (let [func-count (vector-length functions)
    header (emit-header)]
    (let [type-sec (emit-type-section-wasi-quad-functions functions)]
      (let [import-sec (emit-import-section-alloc-print-read-arg-concat-sub)]
        (let [func-sec (emit-function-section-wasi-quad-functions functions)]
          (let [memory-sec (emit-memory-section)]
            (let [export-sec (emit-export-section-main-memory-index (+ 10 func-count) 0)]
              (let [code-sec (emit-code-section-wasi-quad-functions functions)]
                (let [data-sec (emit-data-section data 1024)]
                  (let [b0 (append-section-bytes (vector-new 64) header)
                    b1 (append-section-bytes b0 type-sec)
                    b2 (append-section-bytes b1 import-sec)
                    b3 (append-section-bytes b2 func-sec)
                    b4 (append-section-bytes b3 memory-sec)
                    b5 (append-section-bytes b4 export-sec)
                    b6 (append-section-bytes b5 code-sec)]
                    (append-section-bytes b6 data-sec)))))))))))
(defn build-wasm-bytes-wasi-progress-debug [functions data]
  (let [func-count (vector-length functions)
    data-len (vector-length data)]
    (do
      (print 50)
      (print func-count)
      (print data-len)
      (let [header (emit-header)]
        (do
          (print 51)
          (print (vector-length header))
          (let [type-sec (emit-type-section-wasi-quad-functions-progress-debug functions)]
            (do
              (print 52)
              (print (vector-length type-sec))
              (let [import-sec (emit-import-section-alloc-print-read-arg-concat-sub)]
                (do
                  (print 53)
                  (print (vector-length import-sec))
                  (let [func-sec (emit-function-section-wasi-quad-functions functions)]
                    (do
                      (print 54)
                      (print (vector-length func-sec))
                      (let [memory-sec (emit-memory-section)]
                        (do
                          (print 55)
                          (print (vector-length memory-sec))
                          (let [export-sec (emit-export-section-main-memory-index (+ 10 func-count) 0)]
                            (do
                              (print 56)
                              (print (vector-length export-sec))
                              (let [code-sec (emit-code-section-wasi-quad-functions-progress-debug functions)]
                                (do
                                  (print 57)
                                  (print (vector-length code-sec))
                                  (let [data-sec (emit-data-section data 1024)]
                                    (do
                                      (print 58)
                                      (print (vector-length data-sec))
                                      (let [b0 (append-byte-vector (vector-new 64) header 0 (vector-length header))]
                                        (do
                                          (print 59)
                                          (print (vector-length b0))
                                          (let [b1 (append-byte-vector-chunked b0 type-sec 0 (vector-length type-sec))]
                                            (do
                                              (print 60)
                                              (print (vector-length b1))
                                              (let [b2 (append-byte-vector-chunked b1 import-sec 0 (vector-length import-sec))]
                                                (do
                                                  (print 61)
                                                  (print (vector-length b2))
                                                  (let [b3 (append-byte-vector-chunked b2 func-sec 0 (vector-length func-sec))]
                                                    (do
                                                      (print 62)
                                                      (print (vector-length b3))
                                                      (let [b4 (append-byte-vector-chunked b3 memory-sec 0 (vector-length memory-sec))]
                                                        (do
                                                          (print 63)
                                                          (print (vector-length b4))
                                                          (let [b5 (append-byte-vector-chunked b4 export-sec 0 (vector-length export-sec))]
                                                            (do
                                                              (print 64)
                                                              (print (vector-length b5))
                                                              (let [b6 (append-byte-vector-chunked b5 code-sec 0 (vector-length code-sec))]
                                                                (do
                                                                  (print 65)
                                                                  (print (vector-length b6))
                                                                  (let [b7 (append-byte-vector-chunked b6 data-sec 0 (vector-length data-sec))]
                                                                    (do
                                                                      (print 66)
                                                                      (print (vector-length b7))
                                                                      b7)))))))))))))))))))))))))))))))))))
(defn compile-file-mode [] (let [path (command-line-arg 1) cache-ref (ref-new (map-new)) parse-count-ref (ref-new 0) data-ref (ref-new (vector-new 8)) functions (compile-file-functions-with-cache path 10 cache-ref parse-count-ref data-ref) wasm-bytes (build-wasm-bytes-wasi functions (ref-get data-ref))] (print-wasm-module wasm-bytes)))
(defn compile-file-mode-build-progress-debug [] (let [path (command-line-arg 1) cache-ref (ref-new (map-new)) parse-count-ref (ref-new 0) data-ref (ref-new (vector-new 8)) functions (compile-file-functions-with-cache path 10 cache-ref parse-count-ref data-ref) wasm-bytes (build-wasm-bytes-wasi-progress-debug functions (ref-get data-ref))] (do (print 67) (print (vector-length wasm-bytes)) 0)))
(defn compile-file-mode-build-phase-probe []
  (let [path (command-line-arg 1)
    cache-ref (ref-new (map-new))
    parse-count-ref (ref-new 0)
    data-ref (ref-new (vector-new 8))]
    (do
      (print 101)
      (let [functions (compile-file-functions-with-cache path 10 cache-ref parse-count-ref data-ref)]
        (do
          (root_push functions)
          (print 102)
          (print (vector-length functions))
          (print 104)
          (print (ref-get parse-count-ref))
          (let [data (ref-get data-ref)]
            (do
              (root_push data)
              (let [wasm-bytes (build-wasm-bytes-wasi-progress-debug functions data)]
                (do
                  (print 103)
                  (print (vector-length wasm-bytes))
                  (root_pop)
                  (root_pop)
                  0)))))))))
(defn compile-file-mode-build-compile-progress-probe []
  (let [path (command-line-arg 1)
    cache-ref (ref-new (map-new))
    parse-count-ref (ref-new 0)
    all-pairs (compile-file-pairs-with-cache path cache-ref parse-count-ref)]
    (do
      (print 111)
      (root_push all-pairs)
      (let [data-ref (ref-new (vector-new 8))]
        (do
          (root_push data-ref)
          (let [n (vector-length all-pairs)
            reg-result (register-all-pairs all-pairs 0 n (ftable-new) 10)
            ftable (vector-get reg-result 0)]
            (do
              (root_push reg-result)
              (print 112)
              (print n)
              (let [functions (compile-all-src-decl-pairs-progress-debug all-pairs 0 n ftable data-ref (vector-new 8))]
                (do
                  (root_push functions)
                  (print 113)
                  (print (vector-length functions))
                  (root_pop)
                  (root_pop)
                  (root_pop)
                  0)))))))))
(defn compile-file-mode-target-defn-parity-probe []
  (let [path (command-line-arg 1)
    src (read-file path)
    decls (parse-program src)
    target-idx 59
    cache-ref (ref-new (map-new))
    parse-count-ref (ref-new 0)
    all-pairs (compile-file-pairs-with-cache path cache-ref parse-count-ref)
    local-reg-result (register-defns-chunked decls 0 (vector-length decls) (ftable-new) 10)
    recursive-reg-result (register-defns decls 0 (vector-length decls) (ftable-new) 10)
    early-ftable (ftable-register (ftable-new) (vector-get (vector-get decls 2) 1) 555)
    direct-ftable (ftable-register (ftable-new) (vector-get (vector-get decls 31) 1) 777)
    literal-pos-ftable (ftable-register (ftable-new) 12345 444)
    literal-neg-ftable (ftable-register (ftable-new) -12345 333)
    direct-env (env-bind (env-new) 12345 222)
    direct-step-state (register-defns-step decls 31 (vector-length decls) (ftable-new) 777)
    tail-recursive-result (register-defns decls 31 (vector-length decls) (ftable-new) 777)]
    (do
      (print 121)
      (print target-idx)
      (root_push src)
      (root_push decls)
      (root_push all-pairs)
      (root_push local-reg-result)
      (root_push recursive-reg-result)
      (root_push early-ftable)
      (root_push direct-ftable)
      (root_push literal-pos-ftable)
      (root_push literal-neg-ftable)
      (root_push direct-env)
      (root_push direct-step-state)
      (root_push tail-recursive-result)
      (let [data-ref (ref-new (vector-new 8))]
        (do
          (root_push data-ref)
          (let [n (vector-length all-pairs)
            reg-result (register-all-pairs all-pairs 0 n (ftable-new) 10)
            ftable (vector-get reg-result 0)
            local-ftable (vector-get local-reg-result 2)
            recursive-ftable (vector-get recursive-reg-result 0)
            direct-step-ftable (vector-get direct-step-state 2)
            tail-recursive-ftable (vector-get tail-recursive-result 0)
            decl (vector-get decls target-idx)
            body (vector-get decl (+ 3 (vector-get decl 2)))
            outer-expr (vector-get body 3)
            inner-call (vector-get (vector-get outer-expr 3) 4)
            inner-func (vector-get inner-call 1)]
            (do
              (root_push reg-result)
              (root_push decl)
              (print 124)
              (print (vector-get decl 0))
              (print 125)
              (print (vector-get decl 2))
              (print 126)
              (print (vector-get body 0))
              (print 127)
              (print (vector-get inner-call 0))
              (print 128)
              (print (vector-get inner-func 0))
              (print 129)
              (print (vector-get inner-func 1))
              (print 130)
              (print (ftable-lookup ftable (vector-get inner-func 1)))
              (print 131)
              (print (vector-get (vector-get decls 31) 1))
              (print 132)
              (print (ftable-lookup ftable (vector-get (vector-get decls 31) 1)))
              (print 133)
              (print (ftable-lookup local-ftable (vector-get inner-func 1)))
              (print 134)
              (print (ftable-lookup local-ftable (vector-get (vector-get decls 31) 1)))
              (print 135)
              (print (ftable-lookup recursive-ftable (vector-get inner-func 1)))
              (print 136)
              (print (ftable-lookup recursive-ftable (vector-get (vector-get decls 31) 1)))
              (print 137)
              (print (ftable-lookup direct-step-ftable (vector-get (vector-get decls 31) 1)))
              (print 138)
              (print (ftable-lookup tail-recursive-ftable (vector-get (vector-get decls 31) 1)))
              (print 139)
              (print (ftable-lookup direct-ftable (vector-get (vector-get decls 31) 1)))
              (print 140)
              (print (vector-get direct-step-state 1))
              (print 141)
              (print (vector-get direct-step-state 3))
              (print 142)
              (print (ftable-lookup early-ftable (vector-get (vector-get decls 2) 1)))
              (print 143)
              (print (ftable-lookup literal-pos-ftable 12345))
              (print 144)
              (print (ftable-lookup literal-neg-ftable -12345))
              (print 145)
              (print (env-lookup direct-env 12345))
              (print 146)
              (print (ftable-size direct-ftable))
              (print 147)
              (print (map-size direct-env))
              (let [ftable-ir (compile-defn-with-ftable decl ftable)]
                (do
                  (root_push ftable-ir)
                  (print 123)
                  (print (vector-length ftable-ir))
                  (let [source-ir (compile-defn-with-source decl src ftable data-ref)]
                    (do
                      (print 122)
                      (print (vector-length source-ir))
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
                      (root_pop)
                      (root_pop)
                      (root_pop)
                      0)))))))))))
(defn compile-file-mode-warm-target-defn-parity-probe []
  (let [path (command-line-arg 1)
    cache-ref (ref-new (map-new))
    parse-count-ref (ref-new 0)
    all-pairs (compile-file-pairs-with-cache path cache-ref parse-count-ref)
    target-idx 59]
    (do
      (root_push all-pairs)
      (let [data-ref (ref-new (vector-new 8))]
        (do
          (root_push data-ref)
          (let [n (vector-length all-pairs)
            reg-result (register-all-pairs all-pairs 0 n (ftable-new) 10)
            ftable (vector-get reg-result 0)
            pair0 (vector-get all-pairs 0)
            pair1 (vector-get all-pairs 1)
            src0 (vector-get pair0 0)
            decls0 (vector-get pair0 1)
            src1 (vector-get pair1 0)
            decls1 (vector-get pair1 1)]
            (do
              (root_push reg-result)
              (root_push pair0)
              (root_push pair1)
              (root_push src0)
              (root_push decls0)
              (let [functions0 (compile-defn-functions-linear-with-source decls0 0 (vector-length decls0) src0 ftable data-ref (vector-new 8))]
                (do
                  (root_push functions0)
                  (root_push src1)
                  (root_push decls1)
                  (let [functions1 (compile-defn-functions-linear-with-source decls1 0 target-idx src1 ftable data-ref functions0)
                    decl (vector-get decls1 target-idx)]
                    (do
                      (root_push functions1)
                      (root_push decl)
                      (print 141)
                      (print (vector-length functions1))
                      (print 142)
                      (print (vector-length (ref-get data-ref)))
                      (print 124)
                      (print (vector-get decl 0))
                      (let [ftable-ir (compile-defn-with-ftable decl ftable)]
                        (do
                          (root_push ftable-ir)
                          (print 123)
                          (print (vector-length ftable-ir))
                          (let [function-meta (compile-defn-function-with-source decl src1 ftable data-ref)]
                            (do
                              (root_push function-meta)
                              (print 144)
                              (print (vector-length (function-meta-ir function-meta)))
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
                              0)))))))))))))))
(defn compile-file-mode-first-defn-ir-parity-probe []
  (let [path (command-line-arg 1)
    src (read-file path)
    decls (parse-program src)
    n (vector-length decls)
    defn-idx (find-first-defn-index decls 0 n)]
    (if (< defn-idx 0)
      (do
        (print 95)
        (print -1)
        (print 96)
        (print -1)
        (print 97)
        (print -1)
        (print 98)
        (print -1)
        0)
      (do
        (root_push src)
        (root_push decls)
        (let [reg-result (register-defns-chunked decls 0 n (ftable-new) 10)]
          (do
            (root_push reg-result)
            (let [defn-node (vector-get decls defn-idx)
              ftable (vector-get reg-result 2)
              data-ref (ref-new (vector-new 8))]
              (do
                (root_push defn-node)
                (root_push ftable)
                (root_push data-ref)
                (print 91)
                (print defn-idx)
                (let [raw-source (compile-defn-with-source defn-node src ftable data-ref)]
                  (do
                    (root_push raw-source)
                    (print 92)
                    (print (vector-length raw-source))
                    (print 93)
                    (let [with-source (compile-defn-function-with-source defn-node src ftable data-ref)]
                      (do
                        (root_push with-source)
                        (print 94)
                        (print (vector-length (function-meta-ir with-source)))
                        (let [with-ftable (compile-defn-function defn-node ftable)]
                          (do
                            (root_push with-ftable)
                            (print 95)
                            (print defn-idx)
                            (print 96)
                            (print (vector-length (function-meta-ir with-source)))
                            (print 97)
                            (print (vector-length (function-meta-ir with-ftable)))
                            (print 98)
                            (print (vector-length (ref-get data-ref)))
                            (let [raw-ftable (compile-defn-with-ftable defn-node ftable)]
                              (do
                                (root_push raw-ftable)
                                (print 99)
                                (print (vector-length raw-ftable))
                                (root_pop)))
                            (root_pop)
                            (root_pop)
                            (root_pop)
                            (root_pop)
                            (root_pop)
                            (root_pop)
                            (root_pop)
                            (root_pop)
                            (root_pop)
                            0))))))))))))))
(defn compile-file-mode-first-defn-source-probe []
  (let [path (command-line-arg 1)
    src (read-file path)
    decls (parse-program src)
    n (vector-length decls)
    defn-idx (find-first-defn-index decls 0 n)]
    (do
      (print 301)
      (print defn-idx)
      (if (< defn-idx 0)
        0
        (do
          (root_push src)
          (root_push decls)
          (let [reg-result (register-defns-chunked decls 0 n (ftable-new) 10)]
            (do
              (root_push reg-result)
              (let [defn-node (vector-get decls defn-idx)
                ftable (vector-get reg-result 2)
                data-ref (ref-new (vector-new 8))
                param-count (vector-get defn-node 2)
                env (bind-node-params defn-node 3 0 param-count (env-new) 1)
                body-idx (+ 3 param-count)
                body (vector-get defn-node body-idx)]
                (do
                  (root_push defn-node)
                  (root_push ftable)
                  (root_push data-ref)
                  (root_push env)
                  (root_push body)
                  (print 302)
                  (print (vector-get body 0))
                  (let [probe-result
                    (if (= (vector-get body 0) (tag-if))
                      (let [cond1 (vector-get (vector-get (vector-get body 3) 2) 5)]
                        (do
                          (root_push cond1)
                          (print 303)
                          (print (vector-get cond1 0))
                          (let [probe-value
                            (if false
                              (let [arg-count (vector-get cond1 2)
                                arg-instrs-list (compile-user-call-arg-instrs-with-source cond1 src env ftable 0 arg-count (vector-new 8) data-ref)]
                                (do
                                  (root_push arg-instrs-list)
                                  (print 304)
                                  (print (vector-length arg-instrs-list))
                                  (print 305)
                                  (print-user-call-arg-instrs-lengths arg-instrs-list 0 arg-count)
                                  (print 306)
                                  (print (max-root-temp-base-list env arg-instrs-list arg-count))
                                  (root_pop)
                                  0))
                              (let [cond1-ir (compile-expr-with-source cond1 src env ftable (vector-new 8) data-ref)]
                                (do
                                  (root_push cond1-ir)
                                  (print 304)
                                  (print (vector-length cond1-ir))
                                  (print-instr-vector-probe cond1-ir 0 (vector-length cond1-ir))
                                  (root_pop)
                                  0)))]
                            (do
                              (root_pop)
                              probe-value))))
                      0)]
                    (do
                      (root_pop)
                      (root_pop)
                      (root_pop)
                      (root_pop)
                      (root_pop)
                      (root_pop)
                      (root_pop)
                      (root_pop)
                      probe-result)))))))))))
(defn compile-file-mode-first-defn-source-step-probe []
  (compile-file-mode-first-defn-source-probe))
(defn compile-file-mode-progress-debug []
  (let [path (command-line-arg 1)
    src (read-file path)]
    (do
      (root_push path)
      (root_push src)
      (let [program (parse-program src)]
        (do
          (root_push program)
          (print 1)
          (print (vector-length program))
          (let [source-root (resolve-source-root path)]
            (do
              (root_push source-root)
              (let [package-root (resolve-package-root path)]
                (do
                  (root_push package-root)
                  (let [seen-ref (ref-new (map-new))]
                    (do
                      (root_push seen-ref)
                      (let [imported-pairs (load-imports-from-decls program src 0 (vector-length program) seen-ref (vector-new 8) source-root package-root)]
                        (do
                          (root_push imported-pairs)
                          (print 2)
                          (print (vector-length imported-pairs))
                          (let [all-pairs (vector-push imported-pairs (make-src-decl-pair src program))]
                            (do
                              (root_push all-pairs)
                              (let [n (vector-length all-pairs)
                                reg-result (register-all-pairs all-pairs 0 n (ftable-new) 10)]
                                (do
                                  (root_push reg-result)
                                  (print 3)
                                  (print (- (vector-get reg-result 1) 10))
                                  (let [ftable (vector-get reg-result 0)
                                    data-ref (ref-new (vector-new 8))]
                                    (do
                                      (root_push ftable)
                                      (root_push data-ref)
                                      (let [functions (compile-all-src-decl-pairs-progress-debug all-pairs 0 n ftable data-ref (vector-new 8))]
                                        (do
                                          (root_push functions)
                                          (print 4)
                                          (print (vector-length functions))
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
                                          (root_pop)
                                          0)))))))))))))))))))))
(defn compile-file-mode-token-debug []
  (let [path (command-line-arg 1)
    src (read-file path)]
    (do
      (root_push src)
      (let [spans (tokenize-with-spans src)
        token-count (/ (vector-length spans) 3)
        sample-count (if (> token-count 14) 14 token-count)]
        (do
          (root_push spans)
          (print 72)
          (print token-count)
          (print 0)
          (print-token-triples spans 0 sample-count)
          (root_pop)
          (root_pop)
          0)))))
(defn compile-file-mode-ir-debug []
  (let [path (command-line-arg 1)
    src (read-file path)]
    (do
      (root_push src)
      (let [program (parse-program src)
        decl-count (vector-length program)]
        (do
          (root_push program)
          (print 71)
          (print decl-count)
          (print (if (> decl-count 0) (vector-get (vector-get program (- decl-count 1)) 0) -1))
          (root_pop)
          (root_pop)
          0)))))
(defn compile-file-mode-expr-tag-debug []
  (let [path (command-line-arg 1)
    src (read-file path)
    spans (tokenize-with-spans src)
    program (parse-program src)
    decl-count (vector-length program)]
    (if (> decl-count 1)
      (let [decl (vector-get program 1)
        decl-tag (vector-get decl 0)
        expr-pos-ref (ref-new 9)
        if-pos-ref (ref-new 10)
        direct-expr (parse-expr-v3 spans expr-pos-ref src)
        direct-if (parse-if-v3 spans if-pos-ref src)
        direct-expr-tag (vector-get direct-expr 0)
        direct-if-tag (vector-get direct-if 0)]
        (do
          (print 73)
          (print (span-kind-or-minus-one spans 9))
          (print (span-kind-or-minus-one spans 10))
          (print (span-kind-or-minus-one spans 16))
          (print (span-kind-or-minus-one spans 17))
          (print direct-expr-tag)
          (print direct-if-tag)
          (print decl-tag)
          (if (= decl-tag 20)
            (let [param-count (vector-get decl 2)
              body-idx (+ 3 param-count)
              body (vector-get decl body-idx)
              body-tag (vector-get body 0)]
              (do
                (print param-count)
                (print body-tag)
                (if (= body-tag 6)
                  (do
                    (print (vector-get (vector-get body 2) 0))
                    (print (vector-get (vector-get body 3) 0))
                    0)
                  (do
                    (print -1)
                    (print -1)
                    0))))
            (do
              (print -1)
              (print -1)
              (print -1)
              0))))
      (do
        (print 73)
        (print -1)
        (print -1)
        (print -1)
        (print -1)
        (print -1)
        (print -1)
        (print -1)
        (print -1)
        (print -1)
        (print -1)
        0))))
(defn compile-file-mode-debug [] (let [path (command-line-arg 1) src (read-file path) src-len (string-length src) lex8 (lex-one src 8 src-len) spans (tokenize-with-spans src) program (parse-program src) source-root (resolve-source-root path) package-root (resolve-package-root path) seen-ref (ref-new (map-new)) imported-pairs (load-imports-from-decls program src 0 (vector-length program) seen-ref (vector-new 8) source-root package-root) all-pairs (vector-push imported-pairs (make-src-decl-pair src program)) n (vector-length all-pairs) reg-result (register-all-pairs all-pairs 0 n (ftable-new) 10) ftable (vector-get reg-result 0) data-ref (ref-new (vector-new 8)) functions (compile-all-src-decl-pairs all-pairs 0 n ftable data-ref (vector-new 8)) wasm-bytes (build-wasm-bytes-wasi functions (ref-get data-ref))] (do (print (vector-length program)) (print (decl-tag-or-minus-one program 0)) (print (decl-tag-or-minus-one program 1)) (print (vector-length imported-pairs)) (print (vector-length all-pairs)) (print (- (vector-get reg-result 1) 10)) (print (vector-length functions)) (print (vector-length wasm-bytes)) (print (vector-length spans)) (print (span-kind-or-minus-one spans 0)) (print (span-kind-or-minus-one spans 1)) (print (span-kind-or-minus-one spans 2)) (print (span-kind-or-minus-one spans 3)) (print (span-kind-or-minus-one spans 4)) (print (span-kind-or-minus-one spans 5)) (print (span-kind-or-minus-one spans 6)) (print (span-kind-or-minus-one spans 7)) (print (span-start-or-minus-one spans 2)) (print (span-end-or-minus-one spans 2)) (print (span-start-or-minus-one spans 3)) (print (span-end-or-minus-one spans 3)) (print src-len) (print (string-char-at src 7)) (print (string-char-at src 8)) (print (string-char-at src 15)) (print (string-char-at src 16)) (print (string-char-at src 17)) (print (string-char-at src 18)) (print (string-char-at src 19)) (print (skip-ws-loop src 7 src-len)) (print (skip-ws-loop src 8 src-len)) (print (/ lex8 1000000)) (print (- lex8 (* (/ lex8 1000000) 1000000))) (print (is-symbol-char (string-char-at src 15))) (print (is-symbol-char (string-char-at src 16))) (print (is-symbol-char (string-char-at src 17))) (print (is-symbol-char (string-char-at src 18))) (print (is-symbol-char (string-char-at src 19))) (print (scan-symbol-step src 16 src-len)) (print (scan-symbol-step src 17 src-len)) (print (scan-symbol-end-step-8 src 9 src-len)) (print (scan-symbol-end src 9 src-len)) 0)))
(defn compile-file-mode-path-debug [] (let [path (command-line-arg 1) entry-dir (path-parent path) parent-dir (path-parent entry-dir) entry-base (path-basename entry-dir) parent-base (path-basename parent-dir) src-ancestor (find-src-ancestor entry-dir) source-root (resolve-source-root path) package-root (resolve-package-root path)] (do (print (string-length path)) (print (string-length entry-dir)) (print (string-length parent-dir)) (print (string-length entry-base)) (print (text-char-or-minus-one entry-base 0)) (print (text-char-or-minus-one entry-base 1)) (print (text-char-or-minus-one entry-base 2)) (print (string-length parent-base)) (print (text-char-or-minus-one parent-base 0)) (print (text-char-or-minus-one parent-base 1)) (print (text-char-or-minus-one parent-base 2)) (print (if (is-src-dir-name parent-base) 1 0)) (print (string-length src-ancestor)) (print (text-char-or-minus-one src-ancestor 0)) (print (text-char-or-minus-one src-ancestor 1)) (print (text-char-or-minus-one src-ancestor 2)) (print (string-length source-root)) (print (text-char-or-minus-one source-root 0)) (print (text-char-or-minus-one source-root 1)) (print (text-char-or-minus-one source-root 2)) (print (text-char-or-minus-one source-root 3)) (print (string-length package-root)) (print (text-char-or-minus-one package-root 0)) (print (text-char-or-minus-one package-root 1)) (print (text-char-or-minus-one package-root 2)) (print (text-char-or-minus-one package-root 3)) (print (text-char-or-minus-one package-root 4)) (print (text-char-or-minus-one package-root 5)) (print (text-char-or-minus-one package-root 6)) 0)))
