(module App.CompilerMode)
(import App.ModuleResolver)
(import Syntax.Parser)
(import Syntax.Lexer)
(import Backend.Wasm.Compiler)
(import Backend.Wasm.WasmEmit)
(defn decl-tag-or-minus-one [decls idx] (if (< idx (vector-length decls)) (vector-get (vector-get decls idx) 0) -1))
(defn text-char-or-minus-one [text idx] (if (< idx (string-length text)) (string-char-at text idx) -1))
(defn span-kind-or-minus-one [spans idx] (if (< (* idx 3) (vector-length spans)) (span-kind spans idx) -1))
(defn span-start-or-minus-one [spans idx] (if (< (+ (* idx 3) 1) (vector-length spans)) (span-start spans idx) -1))
(defn span-end-or-minus-one [spans idx] (if (< (+ (* idx 3) 2) (vector-length spans)) (span-end spans idx) -1))
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
(defn source-fingerprint-step-8 [src pos end acc] (let [step1 (source-fingerprint-step src pos end acc) step2 (continue-source-fingerprint-step src end step1) step3 (continue-source-fingerprint-step src end step2) step4 (continue-source-fingerprint-step src end step3) step5 (continue-source-fingerprint-step src end step4) step6 (continue-source-fingerprint-step src end step5) step7 (continue-source-fingerprint-step src end step6) step8 (continue-source-fingerprint-step src end step7)] step8))
(defn continue-source-fingerprint-step-8 [src end state] (if (= (vector-get state 0) 1) state (source-fingerprint-step-8 src (vector-get state 1) end (vector-get state 2))))
(defn source-fingerprint-step-64 [src pos end acc] (let [step1 (source-fingerprint-step-8 src pos end acc) step2 (continue-source-fingerprint-step-8 src end step1) step3 (continue-source-fingerprint-step-8 src end step2) step4 (continue-source-fingerprint-step-8 src end step3) step5 (continue-source-fingerprint-step-8 src end step4) step6 (continue-source-fingerprint-step-8 src end step5) step7 (continue-source-fingerprint-step-8 src end step6) step8 (continue-source-fingerprint-step-8 src end step7)] step8))
(defn source-fingerprint-loop [src pos end acc]
  (do
    (root_push src)
    (let [step (source-fingerprint-step-64 src pos end acc)]
      (do
        (root_push step)
        (let [result
            (if (= (vector-get step 0) 1)
              (vector-get step 2)
              (source-fingerprint-loop src (vector-get step 1) end (vector-get step 2)))]
          (do
            (root_pop)
            (root_pop)
            result))))))
(defn source-fingerprint [src] (source-fingerprint-loop src 0 (string-length src) 0))
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
(defn parse-src-decl-pair [src]
  (let [src-slot (root_push src)
    decls (parse-program src)
    decls-slot (root_push decls)]
    (do
      (root_set src-slot (make-src-decl-pair src decls))
      (root_pop)
      (root_pop))))
(defn load-src-decl-pair-with-cache [path cache-ref parse-count-ref]
  (let [path-slot (root_push path)
    cache-slot (root_push cache-ref)
    parse-slot (root_push parse-count-ref)
    src (read-file path)
    src-slot (root_push src)
    fingerprint (source-fingerprint src)
    cache-key (src-decl-cache-key path)
    cached-entry (ref-map-get-safe cache-ref cache-key)]
    (if (= 0 cached-entry)
      (let [pair (parse-src-decl-pair src)
        pair-slot (root_push pair)
        entry (make-src-decl-cache-entry fingerprint pair)
        entry-slot (root_push entry)]
        (do
          (ref-set parse-count-ref (+ (ref-get parse-count-ref) 1))
          (ref-set cache-ref (ref-map-insert-object-safe cache-ref cache-key entry))
          (root_pop)
          (root_set path-slot (clone-src-decl-pair pair))
          (root_pop)
          (root_pop)
          (root_pop)
          (root_pop)
          (root_pop)))
      (if (= (src-decl-cache-entry-fingerprint cached-entry) fingerprint)
        (do
          (root_set path-slot (clone-src-decl-pair (src-decl-cache-entry-pair cached-entry)))
          (root_pop)
          (root_pop)
          (root_pop)
          (root_pop))
        (let [pair (parse-src-decl-pair src)
          pair-slot (root_push pair)
          entry (make-src-decl-cache-entry fingerprint pair)
          entry-slot (root_push entry)]
          (do
            (ref-set parse-count-ref (+ (ref-get parse-count-ref) 1))
            (ref-set cache-ref (ref-map-insert-object-safe cache-ref cache-key entry))
            (root_pop)
            (root_set path-slot (clone-src-decl-pair pair))
            (root_pop)
            (root_pop)
            (root_pop)
            (root_pop)
            (root_pop)))))))
(defn make-pairs-step-state [done next-idx next-pairs]
  (do
    (root_push next-pairs)
    (let [state (push-object-vector (push-int-vector-local (push-int-vector-local (vector-new 3) done) next-idx) next-pairs)]
      (do
        (root_pop)
        state))))
(defn load-imports-from-decls-step [decls src idx n seen-ref pairs source-root package-root] (if (>= idx n) (make-pairs-step-state 1 idx pairs) (let [decl (vector-get decls idx)] (if (= (vector-get decl 0) 26) (let [name-start (vector-get decl 2) name-end (vector-get decl 3) module-name (substring src name-start name-end) updated-pairs (load-module-if-new module-name source-root package-root seen-ref pairs)] (make-pairs-step-state 0 (+ idx 1) updated-pairs)) (make-pairs-step-state 0 (+ idx 1) pairs)))))
(defn continue-load-imports-from-decls-step [decls src n seen-ref source-root package-root state] (if (= (vector-get state 0) 1) state (load-imports-from-decls-step decls src (vector-get state 1) n seen-ref (vector-get state 2) source-root package-root)))
(defn load-imports-from-decls-step-8 [decls src idx n seen-ref pairs source-root package-root] (let [step1 (load-imports-from-decls-step decls src idx n seen-ref pairs source-root package-root) step2 (continue-load-imports-from-decls-step decls src n seen-ref source-root package-root step1) step3 (continue-load-imports-from-decls-step decls src n seen-ref source-root package-root step2) step4 (continue-load-imports-from-decls-step decls src n seen-ref source-root package-root step3) step5 (continue-load-imports-from-decls-step decls src n seen-ref source-root package-root step4) step6 (continue-load-imports-from-decls-step decls src n seen-ref source-root package-root step5) step7 (continue-load-imports-from-decls-step decls src n seen-ref source-root package-root step6) step8 (continue-load-imports-from-decls-step decls src n seen-ref source-root package-root step7)] step8))
(defn continue-load-imports-from-decls-step-8 [decls src n seen-ref source-root package-root state] (if (= (vector-get state 0) 1) state (load-imports-from-decls-step-8 decls src (vector-get state 1) n seen-ref (vector-get state 2) source-root package-root)))
(defn load-imports-from-decls-step-64 [decls src idx n seen-ref pairs source-root package-root] (let [step1 (load-imports-from-decls-step-8 decls src idx n seen-ref pairs source-root package-root) step2 (continue-load-imports-from-decls-step-8 decls src n seen-ref source-root package-root step1) step3 (continue-load-imports-from-decls-step-8 decls src n seen-ref source-root package-root step2) step4 (continue-load-imports-from-decls-step-8 decls src n seen-ref source-root package-root step3) step5 (continue-load-imports-from-decls-step-8 decls src n seen-ref source-root package-root step4) step6 (continue-load-imports-from-decls-step-8 decls src n seen-ref source-root package-root step5) step7 (continue-load-imports-from-decls-step-8 decls src n seen-ref source-root package-root step6) step8 (continue-load-imports-from-decls-step-8 decls src n seen-ref source-root package-root step7)] step8))
(defn load-imports-from-decls [decls src idx n seen-ref pairs source-root package-root] (let [step (load-imports-from-decls-step-64 decls src idx n seen-ref pairs source-root package-root)] (if (= (vector-get step 0) 1) (vector-get step 2) (load-imports-from-decls decls src (vector-get step 1) n seen-ref (vector-get step 2) source-root package-root))))
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
                      next-pairs)))))))
        (do
          (root_pop)
          (root_pop)
          (root_pop)
          (root_pop)
          (root_pop)
          pairs)))))
(defn load-imports-from-decls-with-cache-step [decls src idx n seen-ref pairs source-root package-root cache-ref parse-count-ref] (if (>= idx n) (make-pairs-step-state 1 idx pairs) (let [decl (vector-get decls idx)] (if (= (vector-get decl 0) 26) (let [name-start (vector-get decl 2) name-end (vector-get decl 3) module-name (substring src name-start name-end) updated-pairs (load-module-if-new-with-cache module-name source-root package-root seen-ref pairs cache-ref parse-count-ref)] (make-pairs-step-state 0 (+ idx 1) updated-pairs)) (make-pairs-step-state 0 (+ idx 1) pairs)))))
(defn continue-load-imports-from-decls-with-cache-step [decls src n seen-ref source-root package-root cache-ref parse-count-ref state] (if (= (vector-get state 0) 1) state (load-imports-from-decls-with-cache-step decls src (vector-get state 1) n seen-ref (vector-get state 2) source-root package-root cache-ref parse-count-ref)))
(defn load-imports-from-decls-with-cache-step-8 [decls src idx n seen-ref pairs source-root package-root cache-ref parse-count-ref] (let [step1 (load-imports-from-decls-with-cache-step decls src idx n seen-ref pairs source-root package-root cache-ref parse-count-ref) step2 (continue-load-imports-from-decls-with-cache-step decls src n seen-ref source-root package-root cache-ref parse-count-ref step1) step3 (continue-load-imports-from-decls-with-cache-step decls src n seen-ref source-root package-root cache-ref parse-count-ref step2) step4 (continue-load-imports-from-decls-with-cache-step decls src n seen-ref source-root package-root cache-ref parse-count-ref step3) step5 (continue-load-imports-from-decls-with-cache-step decls src n seen-ref source-root package-root cache-ref parse-count-ref step4) step6 (continue-load-imports-from-decls-with-cache-step decls src n seen-ref source-root package-root cache-ref parse-count-ref step5) step7 (continue-load-imports-from-decls-with-cache-step decls src n seen-ref source-root package-root cache-ref parse-count-ref step6) step8 (continue-load-imports-from-decls-with-cache-step decls src n seen-ref source-root package-root cache-ref parse-count-ref step7)] step8))
(defn continue-load-imports-from-decls-with-cache-step-8 [decls src n seen-ref source-root package-root cache-ref parse-count-ref state] (if (= (vector-get state 0) 1) state (load-imports-from-decls-with-cache-step-8 decls src (vector-get state 1) n seen-ref (vector-get state 2) source-root package-root cache-ref parse-count-ref)))
(defn load-imports-from-decls-with-cache-step-64 [decls src idx n seen-ref pairs source-root package-root cache-ref parse-count-ref] (let [step1 (load-imports-from-decls-with-cache-step-8 decls src idx n seen-ref pairs source-root package-root cache-ref parse-count-ref) step2 (continue-load-imports-from-decls-with-cache-step-8 decls src n seen-ref source-root package-root cache-ref parse-count-ref step1) step3 (continue-load-imports-from-decls-with-cache-step-8 decls src n seen-ref source-root package-root cache-ref parse-count-ref step2) step4 (continue-load-imports-from-decls-with-cache-step-8 decls src n seen-ref source-root package-root cache-ref parse-count-ref step3) step5 (continue-load-imports-from-decls-with-cache-step-8 decls src n seen-ref source-root package-root cache-ref parse-count-ref step4) step6 (continue-load-imports-from-decls-with-cache-step-8 decls src n seen-ref source-root package-root cache-ref parse-count-ref step5) step7 (continue-load-imports-from-decls-with-cache-step-8 decls src n seen-ref source-root package-root cache-ref parse-count-ref step6) step8 (continue-load-imports-from-decls-with-cache-step-8 decls src n seen-ref source-root package-root cache-ref parse-count-ref step7)] step8))
(defn load-imports-from-decls-with-cache [decls src idx n seen-ref pairs source-root package-root cache-ref parse-count-ref]
  (if (>= idx n)
    pairs
    (let [decl (vector-get decls idx)]
      (if (= (vector-get decl 0) 26)
        (let [name-start (vector-get decl 2)
          name-end (vector-get decl 3)
          module-name (substring src name-start name-end)
          updated-pairs (load-module-if-new-with-cache module-name source-root package-root seen-ref pairs cache-ref parse-count-ref)]
          (do
            (root_push updated-pairs)
            (let [result (load-imports-from-decls-with-cache decls src (+ idx 1) n seen-ref updated-pairs source-root package-root cache-ref parse-count-ref)]
              (do
                (root_pop)
                result))))
        (load-imports-from-decls-with-cache decls src (+ idx 1) n seen-ref pairs source-root package-root cache-ref parse-count-ref)))))
(defn load-module-if-new-with-cache [module-name source-root package-root seen-ref pairs cache-ref parse-count-ref]
  (do
    (root_push module-name)
    (root_push source-root)
    (root_push package-root)
    (root_push seen-ref)
    (root_push pairs)
    (root_push cache-ref)
    (root_push parse-count-ref)
    (let [module-key (name-hash module-name 0 (string-length module-name))]
      (if (= 0 (ref-map-get-safe seen-ref module-key))
        (do
          (ref-set seen-ref (ref-map-insert-int-safe seen-ref module-key 1))
          (let [path (resolve-module-path-with-cache module-name source-root package-root cache-ref)
            pair (load-src-decl-pair-with-cache path cache-ref parse-count-ref)]
            (do
              (root_push pair)
              (let [src (vector-get pair 0)
                decls (vector-get pair 1)
                pairs-with-deps (load-imports-from-decls-with-cache decls src 0 (vector-length decls) seen-ref pairs source-root package-root cache-ref parse-count-ref)]
                (do
                  (root_push pairs-with-deps)
                  (let [next-pairs (push-object-vector pairs-with-deps pair)]
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
          (root_pop)
          (root_pop)
          pairs)))))
(defn load-imports-from-decls-with-cache-progress-step [decls src idx n seen-ref pairs source-root package-root cache-ref parse-count-ref] (if (>= idx n) (make-pairs-step-state 1 idx pairs) (let [decl (vector-get decls idx)] (if (= (vector-get decl 0) 26) (let [name-start (vector-get decl 2) name-end (vector-get decl 3) module-name (substring src name-start name-end) updated-pairs (load-module-if-new-with-cache-progress module-name source-root package-root seen-ref pairs cache-ref parse-count-ref)] (make-pairs-step-state 0 (+ idx 1) updated-pairs)) (make-pairs-step-state 0 (+ idx 1) pairs)))))
(defn continue-load-imports-from-decls-with-cache-progress-step [decls src n seen-ref source-root package-root cache-ref parse-count-ref state] (if (= (vector-get state 0) 1) state (load-imports-from-decls-with-cache-progress-step decls src (vector-get state 1) n seen-ref (vector-get state 2) source-root package-root cache-ref parse-count-ref)))
(defn load-imports-from-decls-with-cache-progress-step-8 [decls src idx n seen-ref pairs source-root package-root cache-ref parse-count-ref] (let [step1 (load-imports-from-decls-with-cache-progress-step decls src idx n seen-ref pairs source-root package-root cache-ref parse-count-ref) step2 (continue-load-imports-from-decls-with-cache-progress-step decls src n seen-ref source-root package-root cache-ref parse-count-ref step1) step3 (continue-load-imports-from-decls-with-cache-progress-step decls src n seen-ref source-root package-root cache-ref parse-count-ref step2) step4 (continue-load-imports-from-decls-with-cache-progress-step decls src n seen-ref source-root package-root cache-ref parse-count-ref step3) step5 (continue-load-imports-from-decls-with-cache-progress-step decls src n seen-ref source-root package-root cache-ref parse-count-ref step4) step6 (continue-load-imports-from-decls-with-cache-progress-step decls src n seen-ref source-root package-root cache-ref parse-count-ref step5) step7 (continue-load-imports-from-decls-with-cache-progress-step decls src n seen-ref source-root package-root cache-ref parse-count-ref step6) step8 (continue-load-imports-from-decls-with-cache-progress-step decls src n seen-ref source-root package-root cache-ref parse-count-ref step7)] step8))
(defn continue-load-imports-from-decls-with-cache-progress-step-8 [decls src n seen-ref source-root package-root cache-ref parse-count-ref state] (if (= (vector-get state 0) 1) state (load-imports-from-decls-with-cache-progress-step-8 decls src (vector-get state 1) n seen-ref (vector-get state 2) source-root package-root cache-ref parse-count-ref)))
(defn load-imports-from-decls-with-cache-progress-step-64 [decls src idx n seen-ref pairs source-root package-root cache-ref parse-count-ref] (let [step1 (load-imports-from-decls-with-cache-progress-step-8 decls src idx n seen-ref pairs source-root package-root cache-ref parse-count-ref) step2 (continue-load-imports-from-decls-with-cache-progress-step-8 decls src n seen-ref source-root package-root cache-ref parse-count-ref step1) step3 (continue-load-imports-from-decls-with-cache-progress-step-8 decls src n seen-ref source-root package-root cache-ref parse-count-ref step2) step4 (continue-load-imports-from-decls-with-cache-progress-step-8 decls src n seen-ref source-root package-root cache-ref parse-count-ref step3) step5 (continue-load-imports-from-decls-with-cache-progress-step-8 decls src n seen-ref source-root package-root cache-ref parse-count-ref step4) step6 (continue-load-imports-from-decls-with-cache-progress-step-8 decls src n seen-ref source-root package-root cache-ref parse-count-ref step5) step7 (continue-load-imports-from-decls-with-cache-progress-step-8 decls src n seen-ref source-root package-root cache-ref parse-count-ref step6) step8 (continue-load-imports-from-decls-with-cache-progress-step-8 decls src n seen-ref source-root package-root cache-ref parse-count-ref step7)] step8))
(defn load-imports-from-decls-with-cache-progress [decls src idx n seen-ref pairs source-root package-root cache-ref parse-count-ref] (let [step (load-imports-from-decls-with-cache-progress-step-64 decls src idx n seen-ref pairs source-root package-root cache-ref parse-count-ref)] (if (= (vector-get step 0) 1) (vector-get step 2) (load-imports-from-decls-with-cache-progress decls src (vector-get step 1) n seen-ref (vector-get step 2) source-root package-root cache-ref parse-count-ref))))
(defn load-module-if-new-with-cache-progress [module-name source-root package-root seen-ref pairs cache-ref parse-count-ref]
  (do
    (root_push module-name)
    (root_push source-root)
    (root_push package-root)
    (root_push seen-ref)
    (root_push pairs)
    (root_push cache-ref)
    (root_push parse-count-ref)
    (let [module-key (name-hash module-name 0 (string-length module-name))]
      (do
        (print 82)
        (print module-key)
        (if (= 0 (ref-map-get-safe seen-ref module-key))
          (do
            (ref-set seen-ref (ref-map-insert-int-safe seen-ref module-key 1))
            (let [path (resolve-module-path-with-cache module-name source-root package-root cache-ref)]
              (do
                (print 83)
                (print (src-decl-cache-key path))
                (let [pair (load-src-decl-pair-with-cache path cache-ref parse-count-ref)]
                  (do
                    (root_push pair)
                    (let [src (vector-get pair 0)
                      decls (vector-get pair 1)
                      pairs-with-deps (load-imports-from-decls-with-cache-progress decls src 0 (vector-length decls) seen-ref pairs source-root package-root cache-ref parse-count-ref)]
                      (do
                        (print 84)
                        (print (ref-get parse-count-ref))
                        (root_push pairs-with-deps)
                        (let [next-pairs (push-object-vector pairs-with-deps pair)]
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
            (root_pop)
            (root_pop)
            (root_pop)
            pairs))))))
(defn compile-file-pairs-with-cache [path cache-ref parse-count-ref]
  (let [pair (load-src-decl-pair-with-cache path cache-ref parse-count-ref)
    source-root (resolve-source-root path)
    package-root (resolve-package-root path)
    seen-ref (ref-new (map-new))]
    (do
      (root_push pair)
      (root_push source-root)
      (root_push package-root)
      (root_push seen-ref)
      (root_push cache-ref)
      (root_push parse-count-ref)
      (let [src (vector-get pair 0)
        program (vector-get pair 1)
        imported-pairs (load-imports-from-decls-with-cache program src 0 (vector-length program) seen-ref (vector-new 8) source-root package-root cache-ref parse-count-ref)]
        (do
          (root_push imported-pairs)
          (let [result (push-object-vector imported-pairs pair)]
            (do
              (root_pop)
              (root_pop)
              (root_pop)
              (root_pop)
              (root_pop)
              (root_pop)
              (root_pop)
              result)))))))
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
          (let [functions (compile-all-src-decl-pairs all-pairs 0 n ftable data-ref (vector-new 8))]
            (do
              (root_pop)
              (root_pop)
              (root_pop)
              functions)))))))
(defn compile-file-functions-payload-with-cache [path func-idx cache-ref parse-count-ref]
  (let [data-ref (ref-new (vector-new 8))
    functions (compile-file-functions-with-cache path func-idx cache-ref parse-count-ref data-ref)
    data (ref-get data-ref)]
    (do
      (root_push functions)
      (root_push data)
      (let [payload1 (vector-push (vector-new 2) functions)]
        (do
          (root_push payload1)
          (let [payload2 (vector-push payload1 data)]
            (do
              (root_pop)
              (root_pop)
              (root_pop)
              payload2)))))))
(defn compile-file-mode-cache-probe [] (let [path (command-line-arg 1) cache-ref (ref-new (map-new)) parse-count-ref (ref-new 0) pair (load-src-decl-pair-with-cache path cache-ref parse-count-ref) src (vector-get pair 0) decls (vector-get pair 1)] (do (print 80) (print (ref-get parse-count-ref)) (print (string-length src)) (print (vector-length decls)) 0)))
(defn compile-file-mode-cache-pairs-probe [] (let [path (command-line-arg 1) cache-ref (ref-new (map-new)) parse-count-ref (ref-new 0) all-pairs (compile-file-pairs-with-cache path cache-ref parse-count-ref) n (vector-length all-pairs) entry-pair (vector-get all-pairs (- n 1)) entry-decls (vector-get entry-pair 1)] (do (print 81) (print (ref-get parse-count-ref)) (print n) (print (vector-length entry-decls)) 0)))
(defn compile-file-mode-cache-pairs-progress-probe [] (let [path (command-line-arg 1) cache-ref (ref-new (map-new)) parse-count-ref (ref-new 0) pair (load-src-decl-pair-with-cache path cache-ref parse-count-ref) src (vector-get pair 0) program (vector-get pair 1) source-root (resolve-source-root path) package-root (resolve-package-root path) seen-ref (ref-new (map-new)) imported-pairs (load-imports-from-decls-with-cache-progress program src 0 (vector-length program) seen-ref (vector-new 8) source-root package-root cache-ref parse-count-ref)] (do (print 85) (print (ref-get parse-count-ref)) (print (vector-length imported-pairs)) 0)))
(defn make-register-pairs-state [done next-idx next-ftable next-func-idx]
  (do
    (root_push next-ftable)
    (let [state (push-int-vector-local (push-object-vector (push-int-vector-local (push-int-vector-local (vector-new 4) done) next-idx) next-ftable) next-func-idx)]
      (do
        (root_pop)
        state))))
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
                    next-func-idx (vector-get result 3)]
                    (do
                      (root_pop)
                      (root_pop)
                      (root_pop)
                      (root_pop)
                      (make-register-pairs-state 0 (+ idx 1) next-ftable next-func-idx))))))))))))
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
(defn register-all-pairs-step-8 [pairs idx n ftable func-idx] (let [step1 (register-all-pairs-step pairs idx n ftable func-idx) step2 (continue-register-all-pairs-step pairs n step1) step3 (continue-register-all-pairs-step pairs n step2) step4 (continue-register-all-pairs-step pairs n step3) step5 (continue-register-all-pairs-step pairs n step4) step6 (continue-register-all-pairs-step pairs n step5) step7 (continue-register-all-pairs-step pairs n step6) step8 (continue-register-all-pairs-step pairs n step7)] step8))
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
(defn register-all-pairs-step-64 [pairs idx n ftable func-idx] (let [step1 (register-all-pairs-step-8 pairs idx n ftable func-idx) step2 (continue-register-all-pairs-step-8 pairs n step1) step3 (continue-register-all-pairs-step-8 pairs n step2) step4 (continue-register-all-pairs-step-8 pairs n step3) step5 (continue-register-all-pairs-step-8 pairs n step4) step6 (continue-register-all-pairs-step-8 pairs n step5) step7 (continue-register-all-pairs-step-8 pairs n step6) step8 (continue-register-all-pairs-step-8 pairs n step7)] step8))
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
(defn register-all-pairs [pairs idx n ftable func-idx]
  (let [state (continue-register-all-pairs-step-64 pairs n (register-all-pairs-step-64 pairs idx n ftable func-idx))]
    (vector-push (push-object-vector (vector-new 2) (vector-get state 2)) (vector-get state 3))))
(defn compile-src-decl-pairs-step [pairs idx n ftable data-ref functions] (if (>= idx n) (make-pairs-step-state 1 idx functions) (let [pair (vector-get pairs idx) src (vector-get pair 0) decls (vector-get pair 1) updated-functions (compile-defn-functions-chunked-with-source decls 0 (vector-length decls) src ftable data-ref functions)] (make-pairs-step-state 0 (+ idx 1) updated-functions))))
(defn continue-compile-src-decl-pairs-step [pairs n ftable data-ref state] (if (= (vector-get state 0) 1) state (compile-src-decl-pairs-step pairs (vector-get state 1) n ftable data-ref (vector-get state 2))))
(defn compile-src-decl-pairs-step-8 [pairs idx n ftable data-ref functions] (let [step1 (compile-src-decl-pairs-step pairs idx n ftable data-ref functions) step2 (continue-compile-src-decl-pairs-step pairs n ftable data-ref step1) step3 (continue-compile-src-decl-pairs-step pairs n ftable data-ref step2) step4 (continue-compile-src-decl-pairs-step pairs n ftable data-ref step3) step5 (continue-compile-src-decl-pairs-step pairs n ftable data-ref step4) step6 (continue-compile-src-decl-pairs-step pairs n ftable data-ref step5) step7 (continue-compile-src-decl-pairs-step pairs n ftable data-ref step6) step8 (continue-compile-src-decl-pairs-step pairs n ftable data-ref step7)] step8))
(defn continue-compile-src-decl-pairs-step-8 [pairs n ftable data-ref state] (if (= (vector-get state 0) 1) state (compile-src-decl-pairs-step-8 pairs (vector-get state 1) n ftable data-ref (vector-get state 2))))
(defn compile-src-decl-pairs-step-64 [pairs idx n ftable data-ref functions] (let [step1 (compile-src-decl-pairs-step-8 pairs idx n ftable data-ref functions) step2 (continue-compile-src-decl-pairs-step-8 pairs n ftable data-ref step1) step3 (continue-compile-src-decl-pairs-step-8 pairs n ftable data-ref step2) step4 (continue-compile-src-decl-pairs-step-8 pairs n ftable data-ref step3) step5 (continue-compile-src-decl-pairs-step-8 pairs n ftable data-ref step4) step6 (continue-compile-src-decl-pairs-step-8 pairs n ftable data-ref step5) step7 (continue-compile-src-decl-pairs-step-8 pairs n ftable data-ref step6) step8 (continue-compile-src-decl-pairs-step-8 pairs n ftable data-ref step7)] step8))
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
              updated-functions (compile-defn-functions-chunked-with-source decls 0 (vector-length decls) src ftable data-ref functions)]
            (do
              (root_pop)
              (root_pop)
              (root_push updated-functions)
              (let [result (compile-all-src-decl-pairs pairs (+ idx 1) n ftable data-ref updated-functions)]
                (do
                  (root_pop)
                  (root_pop)
                  (root_pop)
                  (root_pop)
                  result)))))))))
(defn compile-defn-functions-progress-debug [decls idx n src ftable data-ref functions] (if (>= idx n) functions (let [decl (vector-get decls idx)] (do (print 40) (print idx) (print (vector-get decl 0)) (compile-defn-functions-progress-debug decls (+ idx 1) n src ftable data-ref (if (= (vector-get decl 0) 20) (vector-push functions (compile-defn-function-with-source decl src ftable data-ref)) functions))))))
(defn compile-all-src-decl-pairs-progress-debug [pairs idx n ftable data-ref functions] (if (>= idx n) functions (let [pair (vector-get pairs idx) src (vector-get pair 0) decls (vector-get pair 1)] (do (print 30) (print idx) (print (string-length src)) (print (vector-length decls)) (compile-all-src-decl-pairs-progress-debug pairs (+ idx 1) n ftable data-ref (compile-defn-functions-progress-debug decls 0 (vector-length decls) src ftable data-ref functions))))))
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
(defn print-module-bytes-loop [bytes idx count] (let [step (print-module-bytes-step-4096 bytes idx count)] (if (= (vector-get step 0) 1) 0 (print-module-bytes-loop bytes (vector-get step 1) count))))
(defn print-wasm-module [bytes] (let [count (vector-length bytes)] (do (print count) (print-module-bytes-loop bytes 0 count) 0)))
(defn append-section-bytes [dst section] (append-byte-vector dst section 0 (vector-length section)))
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
          (let [type-sec (emit-type-section-wasi-quad-functions functions)]
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
                              (let [code-sec (emit-code-section-wasi-quad-functions functions)]
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
                                          (let [b1 (append-byte-vector b0 type-sec 0 (vector-length type-sec))]
                                            (do
                                              (print 60)
                                              (print (vector-length b1))
                                              (let [b2 (append-byte-vector b1 import-sec 0 (vector-length import-sec))]
                                                (do
                                                  (print 61)
                                                  (print (vector-length b2))
                                                  (let [b3 (append-byte-vector b2 func-sec 0 (vector-length func-sec))]
                                                    (do
                                                      (print 62)
                                                      (print (vector-length b3))
                                                      (let [b4 (append-byte-vector b3 memory-sec 0 (vector-length memory-sec))]
                                                        (do
                                                          (print 63)
                                                          (print (vector-length b4))
                                                          (let [b5 (append-byte-vector b4 export-sec 0 (vector-length export-sec))]
                                                            (do
                                                              (print 64)
                                                              (print (vector-length b5))
                                                              (let [b6 (append-byte-vector b5 code-sec 0 (vector-length code-sec))]
                                                                (do
                                                                  (print 65)
                                                                  (print (vector-length b6))
                                                                  (let [b7 (append-byte-vector b6 data-sec 0 (vector-length data-sec))]
                                                                    (do
                                                                      (print 66)
                                                                      (print (vector-length b7))
                                                                      b7)))))))))))))))))))))))))))))))))))
(defn compile-file-mode [] (let [path (command-line-arg 1) cache-ref (ref-new (map-new)) parse-count-ref (ref-new 0) data-ref (ref-new (vector-new 8)) functions (compile-file-functions-with-cache path 10 cache-ref parse-count-ref data-ref) wasm-bytes (build-wasm-bytes-wasi functions (ref-get data-ref))] (print-wasm-module wasm-bytes)))
(defn compile-file-mode-build-progress-debug [] (let [path (command-line-arg 1) cache-ref (ref-new (map-new)) parse-count-ref (ref-new 0) data-ref (ref-new (vector-new 8)) functions (compile-file-functions-with-cache path 10 cache-ref parse-count-ref data-ref) wasm-bytes (build-wasm-bytes-wasi-progress-debug functions (ref-get data-ref))] (do (print 67) (print (vector-length wasm-bytes)) 0)))
(defn compile-file-mode-progress-debug [] (let [path (command-line-arg 1) src (read-file path) program (parse-program src)] (do (print 1) (print (vector-length program)) (let [source-root (resolve-source-root path) package-root (resolve-package-root path) seen-ref (ref-new (map-new)) imported-pairs (load-imports-from-decls program src 0 (vector-length program) seen-ref (vector-new 8) source-root package-root)] (do (print 2) (print (vector-length imported-pairs)) (let [all-pairs (vector-push imported-pairs (make-src-decl-pair src program)) n (vector-length all-pairs) reg-result (register-all-pairs all-pairs 0 n (ftable-new) 10) ftable (vector-get reg-result 0)] (do (print 3) (print (- (vector-get reg-result 1) 10)) (let [data-ref (ref-new (vector-new 8)) functions (compile-all-src-decl-pairs-progress-debug all-pairs 0 n ftable data-ref (vector-new 8))] (do (print 4) (print (vector-length functions)) 0)))))))))
(defn compile-file-mode-token-debug [] (let [path (command-line-arg 1) src (read-file path) spans (tokenize-with-spans src) token-count (/ (vector-length spans) 3) sample-count (if (> token-count 14) 14 token-count) sample-hash (if (> token-count 10) (name-hash src (span-start spans 10) (span-end spans 10)) 0)] (do (print 72) (print token-count) (print sample-hash) (print-token-triples spans 0 sample-count) 0)))
(defn compile-file-mode-ir-debug [] (let [path (command-line-arg 1) src (read-file path) program (parse-program src) decl-count (vector-length program)] (do (print 71) (print decl-count) (print (if (> decl-count 0) (vector-get (vector-get program (- decl-count 1)) 0) -1)) 0)))
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
