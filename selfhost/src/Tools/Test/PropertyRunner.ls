(module Tools.Test.PropertyRunner)
(import Syntax.AST)
(import Syntax.Lexer)
(import Syntax.Parser)

;; 移行期 property smoke profile の raw payload projection。
;; 対応範囲は `for-all [x Int] :cases 1..5 :postcondition expr` のみとし、
;; seed / shrink / precondition / 未知 option は TestRunner へ渡す前に拒否する。

(defn property-runner-space? [ch]
  (if (or (= ch 32) (= ch 9))
    1
    (if (or (= ch 10) (= ch 13)) 1 0)))

(defn property-runner-skip-space [src idx len]
  (if (>= idx len)
    idx
    (if (= (property-runner-space? (string-char-at src idx)) 1)
      (property-runner-skip-space src (+ idx 1) len)
      idx)))

(defn property-runner-prefix? [src idx needle]
  (let [len (string-length src)
    needle-len (string-length needle)]
    (if (> (+ idx needle-len) len)
      0
      (if (string-eq (substring src idx (+ idx needle-len)) needle) 1 0))))

(defn property-runner-find-loop [src needle idx len needle-len]
  (if (> (+ idx needle-len) len)
    -1
    (if (= (property-runner-prefix? src idx needle) 1)
      idx
      (property-runner-find-loop src needle (+ idx 1) len needle-len))))

(defn property-runner-find-from [src needle idx]
  (property-runner-find-loop
    src
    needle
    idx
    (string-length src)
    (string-length needle)))

(defn property-runner-balanced-end [src idx len depth]
  (if (>= idx len)
    -1
    (let [ch (string-char-at src idx)]
      (if (= ch 40)
        (property-runner-balanced-end src (+ idx 1) len (+ depth 1))
        (if (= ch 41)
          (if (= depth 1)
            (+ idx 1)
            (property-runner-balanced-end src (+ idx 1) len (- depth 1)))
          (property-runner-balanced-end src (+ idx 1) len depth))))))

(defn property-runner-atom-end [src idx len]
  (if (>= idx len)
    idx
    (let [ch (string-char-at src idx)]
      (if (or (= (property-runner-space? ch) 1) (or (= ch 41) (= ch 93)))
        idx
        (property-runner-atom-end src (+ idx 1) len)))))

(defn property-runner-digits? [src idx end]
  (if (>= idx end)
    1
    (let [ch (string-char-at src idx)]
      (if (or (< ch 48) (> ch 57))
        0
        (property-runner-digits? src (+ idx 1) end)))))

;; 4 要素を root 付きで追加する local helper。PropertyRunner の既存 import 境界を保つ。
(defn property-runner-push-four [first second third fourth]
  (let [with-three (vector-push-triple-rooted
      (vector-new 4)
      first
      second
      third)]
    (vector-push-single-rooted with-three fourth)))

;; [binder-name-hash, binder-is-int, binder-close, binder-type-hash]
(defn property-runner-binder-info [payload open close]
  (let [len (string-length payload)
    name-start (property-runner-skip-space payload (+ open 1) len)
    name-end (property-runner-atom-end payload name-start len)
    type-start (property-runner-skip-space payload name-end len)
    type-end (property-runner-atom-end payload type-start len)
    after-type (property-runner-skip-space payload type-end len)]
    (if (= after-type close)
      (property-runner-push-four
        (name-hash payload name-start name-end)
        (if (string-eq (substring payload type-start type-end) "Int") 1 0)
        close
        (name-hash payload type-start type-end))
      (property-runner-push-four 0 0 -1 0))))

;; [case-count, case-count-valid, case-value-end]
(defn property-runner-cases-info [payload marker]
  (let [len (string-length payload)
    start (property-runner-skip-space payload (+ marker 6) len)
    end (property-runner-atom-end payload start len)
    digits (if (<= end start) 0 (property-runner-digits? payload start end))
    count (if (= digits 1) (parse-int-from-str payload start end 0) 0)
    valid (if (= digits 1) (if (and (> count 0) (<= count 5)) 1 0) 0)]
    (vector-push-triple-rooted (vector-new 3) count valid end)))

;; `profile` が 0 の場合だけ、layout 全体がこの移行期 slice に一致する。
(defn property-runner-profile-code [payload]
  (let [len (string-length payload)
    start (property-runner-skip-space payload 0 len)
    open (property-runner-find-from payload "[" start)
    close (property-runner-find-from payload "]" (+ open 1))
    binder (if (and (>= open 0) (>= close 0))
      (property-runner-binder-info payload open close)
      (vector-push-triple-rooted (vector-new 3) 0 0 -1))
    after-binder (property-runner-skip-space payload (+ close 1) len)
    cases-marker (property-runner-find-from payload ":cases" after-binder)
    cases (if (= cases-marker after-binder)
      (property-runner-cases-info payload cases-marker)
      (vector-push-triple-rooted (vector-new 3) 0 0 -1))
    after-cases (property-runner-skip-space payload (vector-get cases 2) len)
    post-marker (property-runner-find-from payload ":postcondition" after-cases)
    post-layout-ok (if (= post-marker after-cases) 1 0)
    post-start (property-runner-skip-space payload (+ post-marker 14) len)
    post-end (if (= (string-char-at payload post-start) 40)
      (property-runner-balanced-end payload post-start len 0)
      (property-runner-atom-end payload post-start len))
    after-post (property-runner-skip-space payload post-end len)
    payload-end-ok (if (= after-post len)
      1
      (if (and (= after-post (- len 1)) (= (string-char-at payload after-post) 41)) 1 0))]
    (if (= (property-runner-prefix? payload start "(for-all") 0)
      3002
      (if (or (< open 0) (< close 0))
        3002
        (if (= (vector-get binder 1) 0)
          3002
          (if (= (vector-get cases 1) 0)
            3002
              (if (or (< post-marker 0) (or (= post-layout-ok 0) (<= post-end post-start)))
                3002
              (if (= payload-end-ok 0) 3002 0))))))))

(defn property-runner-postcondition-text [payload]
  (let [marker (property-runner-find-from payload ":postcondition" 0)
    len (string-length payload)
    start (property-runner-skip-space payload (+ marker 14) len)
    end (if (= (string-char-at payload start) 40)
      (property-runner-balanced-end payload start len 0)
      (property-runner-atom-end payload start len))]
    (if (or (< marker 0) (<= end start)) "" (substring payload start end))))

(defn property-runner-postcondition [payload]
  (let [text (property-runner-postcondition-text payload)]
    (if (= (string-length text) 0)
      0
      (let [program (parse-program text)]
        (if (> (vector-length program) 0) (vector-get program 0) 0)))))

;; property test case:
;; [name-id, owner-function-hash, binder-hash, postcondition, cases, profile-code]
(defn make-property-test-case [name owner binder postcondition cases profile-code]
  (let [v1 (vector-push-single-rooted (vector-new 6) name)
    v2 (vector-push-single-rooted v1 owner)
    v3 (vector-push-single-rooted v2 binder)
    v4 (vector-push-single-rooted v3 postcondition)
    v5 (vector-push-single-rooted v4 cases)]
    (vector-push-single-rooted v5 profile-code)))

(defn property-runner-form-case [form owner name]
  (let [payload (if (> (vector-length form) 1) (vector-get form 1) "")
    profile-code (property-runner-profile-code payload)
    open (property-runner-find-from payload "[" 0)
    close (property-runner-find-from payload "]" (+ open 1))
    binder-info (if (and (>= open 0) (>= close 0))
      (property-runner-binder-info payload open close)
      (vector-push-triple-rooted (vector-new 3) 0 0 -1))
    cases-marker (property-runner-find-from payload ":cases" 0)
    cases-info (if (>= cases-marker 0)
      (property-runner-cases-info payload cases-marker)
      (vector-push-triple-rooted (vector-new 3) 0 0 -1))
    postcondition (if (= profile-code 0) (property-runner-postcondition payload) 0)]
    (make-property-test-case
      name
      owner
      (vector-get binder-info 0)
      postcondition
      (vector-get cases-info 0)
      profile-code)))

;; Rust canonical Property の移行期 deterministic profile を typed shape へ投影する。
;; contract: [owner, binders, preconditions, postcondition, sampling, profile-code]
;; binder: [name-hash, type-name-hash, generator-kind(1=type-directed)]
;; sampling: [cases, seed, generator-version, shrink(1=true), coverage-count]
(defn make-property-typed-binder [binder-info]
  (vector-push-triple-rooted
    (vector-new 3)
    (vector-get binder-info 0)
    (vector-get binder-info 3)
    1))

(defn make-property-sampling-plan [cases]
  (let [with-fields (property-runner-push-four
      cases
      0
      "type-directed-splitmix64-v1"
      1)]
    (vector-push-single-rooted with-fields 0)))

(defn make-property-typed-contract [owner payload]
  (let [profile-code (property-runner-profile-code payload)
    open (property-runner-find-from payload "[" 0)
    close (property-runner-find-from payload "]" (+ open 1))
    binder-info (if (and (>= open 0) (>= close 0))
      (property-runner-binder-info payload open close)
      (property-runner-push-four 0 0 -1 0))
    cases-marker (property-runner-find-from payload ":cases" 0)
    cases-info (if (>= cases-marker 0)
      (property-runner-cases-info payload cases-marker)
      (vector-push-triple-rooted (vector-new 3) 0 0 -1))]
    (do
      (root_push payload)
      (root_push binder-info)
      (root_push cases-info)
      (let [binders (if (= profile-code 0)
        (vector-push-single-rooted
          (vector-new 0)
          (make-property-typed-binder binder-info))
        (vector-new 0))]
        (do
          (root_push binders)
          (let [preconditions (vector-new 0)
            postcondition (if (= profile-code 0)
              (property-runner-postcondition payload)
              0)
            sampling (if (= profile-code 0)
              (make-property-sampling-plan (vector-get cases-info 0))
              (vector-new 0))]
            (do
              (root_push preconditions)
              (root_push postcondition)
              (root_push sampling)
              (let [row0 (property-runner-push-four
                  owner
                  binders
                  preconditions
                  postcondition)]
                (do
                  (root_push row0)
                  (let [row1 (vector-push-single-rooted row0 sampling)]
                    (do
                      (root_push row1)
                      (let [row2 (vector-push-single-rooted row1 profile-code)]
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
                          row2)))))))))))))

(defn property-runner-form-typed-contract [form owner]
  (let [payload (if (> (vector-length form) 1) (vector-get form 1) "")]
    (make-property-typed-contract owner payload)))

(defn property-runner-form-typed-payload [form owner]
  (let [contract (property-runner-form-typed-contract form owner)]
    (do
      (root_push contract)
      (let [payload0 (property-runner-push-four
          (vector-get contract 1)
          (vector-get contract 2)
          (vector-get contract 3)
          (vector-get contract 4))]
        (do
          (root_push payload0)
          (let [payload (vector-push-single-rooted
              payload0
              (vector-get contract 5))]
            (do
              (root_pop)
              (root_pop)
              payload)))))))

(defn property-runner-ordered-forms [decl]
  (let [param-count (vector-get decl 2)
    body-end (+ 4 param-count)
    signature (if (< body-end (vector-length decl)) (vector-get decl body-end) 0)
    offset (if (and (!= signature 0) (= (vector-get signature 0) (ast-defn-signature))) 1 0)
    meta-index (+ body-end offset)]
    (if (< meta-index (vector-length decl))
      (let [meta (vector-get decl meta-index)]
        (if (and (!= meta 0) (> (vector-length meta) 5)) (vector-get meta 5) 0))
      0)))

(defn property-runner-append-forms-loop [forms idx count owner results]
  (if (>= idx count)
    results
    (let [form (vector-get forms idx)
      next-results (if (= (vector-get form 0) (contract-form-property))
        (vector-push
          results
          (property-runner-form-case form owner (vector-length results)))
        results)]
      (property-runner-append-forms-loop forms (+ idx 1) count owner next-results))))

(defn property-runner-append-decl [decl results]
  (let [tag (vector-get decl 0)]
    (if (= tag (ast-defn))
      (let [forms (property-runner-ordered-forms decl)]
        (if (= forms 0)
          results
          (property-runner-append-forms-loop
            forms
            0
            (vector-length forms)
            (vector-get decl 1)
            results)))
      (if (= tag (ast-private))
        (property-runner-append-decl (vector-get decl 1) results)
        (if (= tag (ast-module-decl))
          (property-runner-append-module-loop
            decl
            0
            (vector-get decl 2)
            results)
          results)))))

(defn property-runner-append-module-loop [module-node idx count results]
  (if (>= idx count)
    results
    (property-runner-append-module-loop
      module-node
      (+ idx 1)
      count
      (property-runner-append-decl (vector-get module-node (+ idx 3)) results))))

(defn property-runner-append-program-loop [program idx count results]
  (if (>= idx count)
    results
    (property-runner-append-program-loop
      program
      (+ idx 1)
      count
      (property-runner-append-decl (vector-get program idx) results))))

(defn extract-property-test-cases [program]
  (property-runner-append-program-loop
    program
    0
    (vector-length program)
    (vector-new 4)))

(defn property-runner-append-typed-forms-loop [forms idx count owner results]
  (if (>= idx count)
    results
    (let [form (vector-get forms idx)
      next-results (if (= (vector-get form 0) (contract-form-property))
        (vector-push-single-rooted
          results
          (property-runner-form-typed-contract form owner))
        results)]
      (do
        (root_push next-results)
        (let [parsed (property-runner-append-typed-forms-loop
            forms
            (+ idx 1)
            count
            owner
            next-results)]
          (do
            (root_pop)
            parsed))))))

(defn property-runner-append-typed-decl [decl results]
  (let [tag (vector-get decl 0)]
    (if (= tag (ast-defn))
      (let [forms (property-runner-ordered-forms decl)]
        (if (= forms 0)
          results
          (property-runner-append-typed-forms-loop
            forms
            0
            (vector-length forms)
            (vector-get decl 1)
            results)))
      (if (= tag (ast-private))
        (property-runner-append-typed-decl (vector-get decl 1) results)
        (if (= tag (ast-module-decl))
          (property-runner-append-typed-module-loop
            decl
            0
            (vector-get decl 2)
            results)
          results)))))

(defn property-runner-append-typed-module-loop [module-node idx count results]
  (if (>= idx count)
    results
    (property-runner-append-typed-module-loop
      module-node
      (+ idx 1)
      count
      (property-runner-append-typed-decl
        (vector-get module-node (+ idx 3))
        results))))

(defn property-runner-append-typed-program-loop [program idx count results]
  (if (>= idx count)
    results
    (property-runner-append-typed-program-loop
      program
      (+ idx 1)
      count
      (property-runner-append-typed-decl (vector-get program idx) results))))

(defn extract-parser-typed-property-contracts [program]
  (property-runner-append-typed-program-loop
    program
    0
    (vector-length program)
    (vector-new 0)))

(defn property-test-case-profile-code [test-case]
  (vector-get test-case 5))

(defn property-test-case-owner [test-case]
  (vector-get test-case 1))

(defn property-test-case-binder [test-case]
  (vector-get test-case 2))

(defn property-test-case-postcondition [test-case]
  (vector-get test-case 3))

(defn property-test-case-count [test-case]
  (vector-get test-case 4))

(defn property-runner-boundary-loop [test-cases idx count]
  (if (>= idx count)
    0
    (let [code (property-test-case-profile-code (vector-get test-cases idx))]
      (if (> code 0)
        code
        (property-runner-boundary-loop test-cases (+ idx 1) count)))))

(defn property-runner-boundary-code [program]
  (let [test-cases (extract-property-test-cases program)]
    (property-runner-boundary-loop test-cases 0 (vector-length test-cases))))
