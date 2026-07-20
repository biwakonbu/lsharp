(module Tools.Test.PropertyRunner)
(import Syntax.AST)
(import Syntax.Lexer)
(import Syntax.Parser)

;; 移行期 property profile の raw payload projection。
;; 対応範囲は 1..2 個の Int binder、1..2 個の Bool binder、または Int/Bool mixed 2 binder、
;; 単一の String binder、`:cases 1..5`、
;; precondition の conjunction、postcondition とする。二 binder は deterministic
;; pair prefix、Bool binder は false/true prefix へ投影する。
;; seed / shrink / 未知 option は TestRunner へ渡す前に拒否する。

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

(defn property-runner-balanced-bracket-end [src idx len depth]
  (if (>= idx len)
    -1
    (let [ch (string-char-at src idx)]
      (if (= ch 91)
        (property-runner-balanced-bracket-end src (+ idx 1) len (+ depth 1))
        (if (= ch 93)
          (if (= depth 1)
            (+ idx 1)
            (property-runner-balanced-bracket-end src (+ idx 1) len (- depth 1)))
          (property-runner-balanced-bracket-end src (+ idx 1) len depth))))))

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
        (if (or
            (string-eq (substring payload type-start type-end) "Int")
            (string-eq (substring payload type-start type-end) "String")) 1 0)
        close
        (name-hash payload type-start type-end))
      (property-runner-push-four 0 0 -1 0))))

(defn property-runner-type-supported? [payload type-start type-end]
  (let [type-hash (name-hash payload type-start type-end)]
    (if (or (= type-hash (property-runner-type-int-hash))
        (or (= type-hash (property-runner-type-bool-hash))
          (= type-hash (property-runner-type-string-hash)))) 1 0)))

(defn property-runner-collect-typed-binders-loop [payload idx close len result]
  (let [name-start (property-runner-skip-space payload idx len)]
    (if (>= name-start close)
      (vector-push-pair-rooted (vector-new 2) result 1)
      (let [name-end (property-runner-atom-end payload name-start len)
        type-start (property-runner-skip-space payload name-end len)
        type-end (property-runner-atom-end payload type-start len)
        valid (if (and
            (> name-end name-start)
            (and
              (> type-end type-start)
              (and
                (<= type-end close)
                (= (property-runner-type-supported? payload type-start type-end) 1)))) 1 0)]
        (if (= valid 0)
          (vector-push-pair-rooted (vector-new 2) result 0)
          (let [binder (vector-push-triple-rooted
              (vector-new 3)
              (name-hash payload name-start name-end)
              (name-hash payload type-start type-end)
              1)]
            (do
              (root_push result)
              (root_push binder)
              (let [next-result (vector-push-single-rooted result binder)]
                (do
                  (root_push next-result)
                  (let [parsed (property-runner-collect-typed-binders-loop
                      payload
                      type-end
                      close
                      len
                      next-result)]
                    (do
                      (root_pop)
                      (root_pop)
                      (root_pop)
                      parsed)))))))))))

(defn property-runner-typed-binders-info [payload]
  (let [close (property-runner-find-from
      payload
      "]"
      (+ (property-runner-find-from payload "[" 0) 1))]
    (if (and
        (>= (property-runner-find-from payload "[" 0) 0)
        (and (>= close 0)
          (> close (property-runner-find-from payload "[" 0))))
      (property-runner-collect-typed-binders-loop
        payload
        (+ (property-runner-find-from payload "[" 0) 1)
        close
        (string-length payload)
        (vector-new 0))
      (vector-push-pair-rooted (vector-new 2) (vector-new 0) 0))))

;; [case-count, case-count-valid, case-value-end]
(defn property-runner-cases-info [payload marker]
  (let [len (string-length payload)
    start (property-runner-skip-space payload (+ marker 6) len)
    end (property-runner-atom-end payload start len)
    digits (if (<= end start) 0 (property-runner-digits? payload start end))
    count (if (= digits 1) (parse-int-from-str payload start end 0) 0)
    valid (if (= digits 1) (if (and (> count 0) (<= count 5)) 1 0) 0)]
    (vector-push-triple-rooted (vector-new 3) count valid end)))

;; binder の検証方式だけを caller ごとに変え、option/layout の境界は共有する。
(defn property-runner-profile-layout-code [payload close binder-valid binder-count]
  (let [len (string-length payload)
    after-binder (property-runner-skip-space payload (+ close 1) len)
    cases-marker (property-runner-find-from payload ":cases" after-binder)
    cases (if (= cases-marker after-binder)
      (property-runner-cases-info payload cases-marker)
      (vector-push-triple-rooted (vector-new 3) 0 0 -1))
    after-cases (property-runner-skip-space payload (vector-get cases 2) len)
    pre-marker (property-runner-find-from payload ":precondition" after-cases)
    pre-open (if (= pre-marker after-cases)
      (property-runner-find-from payload "[" (+ pre-marker 13))
      -1)
    pre-end (if (>= pre-open 0)
      (property-runner-balanced-bracket-end payload pre-open len 0)
      -1)
    pre-layout-ok (if (= pre-marker after-cases)
      (if (and (>= pre-open 0) (> pre-end (+ pre-open 1))) 1 0)
      1)
    after-precondition (if (= pre-marker after-cases)
      (property-runner-skip-space payload pre-end len)
      after-cases)
    post-marker (property-runner-find-from payload ":postcondition" after-precondition)
    post-layout-ok (if (= post-marker after-precondition) 1 0)
    post-start (property-runner-skip-space payload (+ post-marker 14) len)
    post-end (if (= (string-char-at payload post-start) 40)
      (property-runner-balanced-end payload post-start len 0)
      (property-runner-atom-end payload post-start len))
    after-post (property-runner-skip-space payload post-end len)
    payload-end-ok (if (= after-post len)
      1
      (if (and (= after-post (- len 1)) (= (string-char-at payload after-post) 41)) 1 0))]
    (if (or (= binder-valid 0) (= binder-count 0))
      3002
      (if (= (vector-get cases 1) 0)
        3002
        (if (or (= pre-layout-ok 0) (or (< post-marker 0) (or (= post-layout-ok 0) (<= post-end post-start))))
          3002
          (if (= payload-end-ok 0) 3002 0))))))

;; raw fallback `profile` は single-binder layout を検査し、typed contract 側は
;; binder vector を使って 1..2 binder の境界を別途検査する。
(defn property-runner-profile-code [payload]
  (let [len (string-length payload)
    start (property-runner-skip-space payload 0 len)
    open (property-runner-find-from payload "[" start)
    close (property-runner-find-from payload "]" (+ open 1))
    binder (if (and (>= open 0) (>= close 0))
      (property-runner-binder-info payload open close)
      (vector-push-triple-rooted (vector-new 3) 0 0 -1))]
    (if (= (property-runner-prefix? payload start "(for-all") 0)
      3002
      (if (or (< open 0) (< close 0))
        3002
        (do
          (root_push binder)
          (let [code (property-runner-profile-layout-code
              payload
              close
              (vector-get binder 1)
              1)]
            (do
              (root_pop)
              code)))))))

(defn property-runner-typed-profile-code [payload]
  (let [len (string-length payload)
    start (property-runner-skip-space payload 0 len)
    open (property-runner-find-from payload "[" start)
    close (property-runner-find-from payload "]" (+ open 1))
    binder-info (property-runner-typed-binders-info payload)]
    (if (= (property-runner-prefix? payload start "(for-all") 0)
      3002
      (if (or (< open 0) (< close 0))
        3002
        (do
          (root_push binder-info)
          (let [code (property-runner-profile-layout-code
              payload
              close
              (vector-get binder-info 1)
              (vector-length (vector-get binder-info 0)))]
            (do
              (root_pop)
              code)))))))

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

(defn property-runner-precondition-text [payload]
  (let [marker (property-runner-find-from payload ":precondition" 0)
    len (string-length payload)]
    (if (< marker 0)
      ""
      (let [precondition-open (property-runner-find-from payload "[" (+ marker 13))]
        (if (< precondition-open 0)
          ""
          (let [bracket-end (property-runner-balanced-bracket-end payload precondition-open len 0)]
            (if (> bracket-end (+ precondition-open 1))
              (substring payload (+ precondition-open 1) (- bracket-end 1))
              "")))))))

(defn property-runner-preconditions [payload]
  (let [text (property-runner-precondition-text payload)]
    (if (= (string-length text) 0)
      (vector-new 0)
      (parse-program text))))

;; property test case:
;; [name-id, owner-function-hash, binder-hashes, postcondition, cases, profile-code,
;;  preconditions, binder-type-hashes, postcondition-source, precondition-source]
(defn make-property-test-case-with-preconditions
  [name owner binders postcondition cases profile-code preconditions binder-types]
  (let [v1 (vector-push-single-rooted (vector-new 8) name)
    v2 (vector-push-single-rooted v1 owner)
    v3 (vector-push-single-rooted v2 binders)
    v4 (vector-push-single-rooted v3 postcondition)
    v5 (vector-push-single-rooted v4 cases)
    v6 (vector-push-single-rooted v5 profile-code)
    v7 (vector-push-single-rooted v6 preconditions)]
    (vector-push-single-rooted v7 binder-types)))

(defn make-property-test-case [name owner binder postcondition cases profile-code]
  (make-property-test-case-with-preconditions
    name
    owner
    (vector-push-single-rooted (vector-new 1) binder)
    postcondition
    cases
    profile-code
    (vector-new 0)
    (vector-new 0)))

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

(defn make-property-sampling-plan [cases]
  (let [with-fields (property-runner-push-four
      cases
      0
      "type-directed-splitmix64-v1"
      1)]
    (vector-push-single-rooted with-fields 0)))

(defn make-property-typed-contract [owner payload]
  (let [profile-code (property-runner-typed-profile-code payload)
    open (property-runner-find-from payload "[" 0)
    close (property-runner-find-from payload "]" (+ open 1))
    cases-marker (property-runner-find-from payload ":cases" 0)
    cases-info (if (>= cases-marker 0)
      (property-runner-cases-info payload cases-marker)
      (vector-push-triple-rooted (vector-new 3) 0 0 -1))
    binder-info (property-runner-typed-binders-info payload)]
    (do
      (root_push payload)
      (root_push binder-info)
      (root_push cases-info)
      (let [binders (if (= profile-code 0)
        (vector-get binder-info 0)
        (vector-new 0))]
        (do
          (root_push binders)
          (let [postcondition (if (= profile-code 0)
              (property-runner-postcondition payload)
              0)
            sampling (if (= profile-code 0)
              (make-property-sampling-plan (vector-get cases-info 0))
              (vector-new 0))
            preconditions (if (= profile-code 0)
              (property-runner-preconditions payload)
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
                          (root_push row2)
                          ;; postcondition AST の offset は切り出し text 基準なので source も保持する。
                          (let [row3 (vector-push-single-rooted
                              row2
                              (if (= profile-code 0)
                                (property-runner-postcondition-text payload)
                                ""))]
                            (do
                              (root_push row3)
                              (let [row4 (vector-push-single-rooted
                                  row3
                                  (if (= profile-code 0)
                                    (property-runner-precondition-text payload)
                                    ""))]
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
                                  row4)))))))))))))))))

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

(defn property-runner-signature-node? [candidate]
  (if (= candidate 0)
    0
    (if (= (vector-get candidate 0) (ast-defn-signature)) 1 0)))

(defn property-runner-ordered-forms [decl]
  (let [param-count (vector-get decl 2)
    body-end (+ 4 param-count)
    signature (if (< body-end (vector-length decl)) (vector-get decl body-end) 0)
    offset (if (= (property-runner-signature-node? signature) 1) 1 0)
    meta-index (+ body-end offset)]
    (if (< meta-index (vector-length decl))
      (let [meta (vector-get decl meta-index)]
        (if (> (vector-length meta) 5) (vector-get meta 5) 0))
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
  (let [contracts (extract-parser-typed-property-contracts program)]
    (do
      (root_push contracts)
      (let [results (property-runner-append-typed-test-cases-loop
          contracts
          0
          (vector-length contracts)
          (vector-new 4))]
        (do
          (root_pop)
          results)))))

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

;; canonical contract を移行期 evaluator の test-case shape へ変換する。
;; 実行可能なのは 1..2 個の Int binder の legacy prefix、単一 String binder、
;; または 3..8 個の Int/Bool/String binder を cases 1..2 で評価する
;; deterministic typed prefix に限定する。
(defn property-runner-execution-profile-code [contract]
  (let [profile-code (vector-get contract 5)
    binders (vector-get contract 1)]
    (if (> profile-code 0)
      profile-code
      (if (or (< (vector-length binders) 1) (> (vector-length binders) 8))
        3002
        (if (= (property-runner-binder-name-collision? binders) 1)
          3002
          (property-runner-binder-type-profile-code contract))))))

(defn property-runner-binder-name-collision-rest? [binders idx count binder-hash]
  (if (>= idx count)
    0
    (if (= (vector-get (vector-get binders idx) 0) binder-hash)
      1
      (property-runner-binder-name-collision-rest?
        binders
        (+ idx 1)
        count
        binder-hash))))

(defn property-runner-binder-name-collision-loop [binders idx count]
  (if (>= idx count)
    0
    (let [binder-hash (vector-get (vector-get binders idx) 0)]
      (if (= binder-hash (name-hash "result" 0 6))
        1
        (if (= (property-runner-binder-name-collision-rest?
            binders
            (+ idx 1)
            count
            binder-hash) 1)
          1
          (property-runner-binder-name-collision-loop binders (+ idx 1) count))))))

(defn property-runner-binder-name-collision? [binders]
  (property-runner-binder-name-collision-loop binders 0 (vector-length binders)))

(defn property-runner-type-int-hash [] (name-hash "Int" 0 3))
(defn property-runner-type-bool-hash [] (name-hash "Bool" 0 4))
(defn property-runner-type-string-hash [] (name-hash "String" 0 6))

(defn property-runner-binder-types-supported-loop [binders idx count]
  (if (>= idx count)
    1
    (let [type-hash (vector-get (vector-get binders idx) 1)]
      (if (or (= type-hash (property-runner-type-int-hash))
          (or (= type-hash (property-runner-type-bool-hash))
            (= type-hash (property-runner-type-string-hash))))
        (property-runner-binder-types-supported-loop binders (+ idx 1) count)
        0))))

(defn property-runner-binder-types-supported? [binders]
  (property-runner-binder-types-supported-loop binders 0 (vector-length binders)))

(defn property-runner-bool-binder-count-loop [binders idx count]
  (if (>= idx count)
    0
    (+ (if (= (vector-get (vector-get binders idx) 1) (property-runner-type-bool-hash)) 1 0)
      (property-runner-bool-binder-count-loop binders (+ idx 1) count))))

(defn property-runner-bool-binder-count [binders]
  (property-runner-bool-binder-count-loop binders 0 (vector-length binders)))

(defn property-runner-int-binder-count-loop [binders idx count]
  (if (>= idx count)
    0
    (+ (if (= (vector-get (vector-get binders idx) 1) (property-runner-type-int-hash)) 1 0)
      (property-runner-int-binder-count-loop binders (+ idx 1) count))))

(defn property-runner-int-binder-count [binders]
  (property-runner-int-binder-count-loop binders 0 (vector-length binders)))

(defn property-runner-string-binder-count-loop [binders idx count]
  (if (>= idx count)
    0
    (+ (if (= (vector-get (vector-get binders idx) 1) (property-runner-type-string-hash)) 1 0)
      (property-runner-string-binder-count-loop binders (+ idx 1) count))))

(defn property-runner-string-binder-count [binders]
  (property-runner-string-binder-count-loop binders 0 (vector-length binders)))

(defn property-runner-mixed-int-bool? [binders]
  (let [bool-count (property-runner-bool-binder-count binders)
    int-count (property-runner-int-binder-count binders)]
    (if (and (= bool-count 1)
        (and (= int-count 1) (= (vector-length binders) 2))) 1 0)))

(defn property-runner-two-bool? [binders]
  (if (and (= (property-runner-bool-binder-count binders) 2)
      (= (vector-length binders) 2)) 1 0))

(defn property-runner-three-bool? [binders]
  (if (and (= (property-runner-bool-binder-count binders) 3)
      (= (vector-length binders) 3)) 1 0))

(defn property-runner-three-mixed-int-bool? [binders]
  (let [bool-count (property-runner-bool-binder-count binders)
    int-count (property-runner-int-binder-count binders)]
    (if (and (= (vector-length binders) 3)
        (and (> bool-count 0) (> int-count 0))) 1 0)))

(defn property-runner-bool-profile-supported? [binders cases]
  (if (> cases 2)
    0
    (if (and (> (vector-length binders) 0) (<= (vector-length binders) 8)) 1 0)))

(defn property-runner-binder-type-profile-code [contract]
  (let [binders (vector-get contract 1)
    sampling (vector-get contract 4)
    cases (if (> (vector-length sampling) 0) (vector-get sampling 0) 0)
    bool-count (property-runner-bool-binder-count binders)
    string-count (property-runner-string-binder-count binders)]
    (if (= (property-runner-binder-types-supported? binders) 0)
      3002
      (if (> string-count 0)
        (if (and (= (vector-length binders) 1)
            (and (= string-count 1) (<= cases 5)))
          0
          (if (and (>= (vector-length binders) 3) (<= cases 2)) 0 3002))
        (if (= bool-count 0)
        (if (or (= (vector-length binders) 1) (= (vector-length binders) 2))
          0
          (if (and (>= (vector-length binders) 3) (<= cases 2)) 0 3002))
        (if (= (property-runner-bool-profile-supported? binders cases) 1)
          0
          3002))))))

(defn property-runner-binder-hashes-loop [binders idx count result]
  (if (>= idx count)
    result
    (let [binder (vector-get binders idx)
      next-result (vector-push-single-rooted result (vector-get binder 0))]
      (do
        (root_push next-result)
        (let [parsed (property-runner-binder-hashes-loop
            binders
            (+ idx 1)
            count
            next-result)]
          (do
            (root_pop)
            parsed))))))

(defn property-runner-binder-hashes [binders]
  (property-runner-binder-hashes-loop
    binders
    0
    (vector-length binders)
    (vector-new (vector-length binders))))

(defn property-runner-binder-types-loop [binders idx count result]
  (if (>= idx count)
    result
    (let [binder (vector-get binders idx)
      next-result (vector-push-single-rooted result (vector-get binder 1))]
      (do
        (root_push next-result)
        (let [parsed (property-runner-binder-types-loop
            binders
            (+ idx 1)
            count
            next-result)]
          (do
            (root_pop)
            parsed))))))

(defn property-runner-binder-types [binders]
  (property-runner-binder-types-loop
    binders
    0
    (vector-length binders)
    (vector-new (vector-length binders))))

(defn property-runner-typed-contract-test-case [contract name]
  (let [owner (vector-get contract 0)
    binders (vector-get contract 1)
    preconditions (vector-get contract 2)
    sampling (vector-get contract 4)
    profile-code (property-runner-execution-profile-code contract)
    binder-hashes (if (= profile-code 0)
      (property-runner-binder-hashes binders)
      (vector-new 0))
    binder-types (if (= profile-code 0)
      (property-runner-binder-types binders)
      (vector-new 0))
    postcondition-source (if (> (vector-length contract) 6)
      (vector-get contract 6)
      "")
    precondition-source (if (> (vector-length contract) 7)
      (vector-get contract 7)
      "")
    postcondition (if (= profile-code 0) (vector-get contract 3) 0)
    cases (if (= profile-code 0) (vector-get sampling 0) 0)]
        (do
          (root_push binder-hashes)
          (root_push binder-types)
          (let [result (make-property-test-case-with-preconditions
              name
              owner
              binder-hashes
              postcondition
              cases
              profile-code
              preconditions
              binder-types)]
            (do
              (root_push result)
              (let [with-source
                  (vector-push-single-rooted result postcondition-source)]
                (do
                  (root_push with-source)
                  (let [with-precondition-source
                      (vector-push-single-rooted with-source precondition-source)]
                    (do
                      (root_pop)
                      (root_pop)
                      (root_pop)
                      (root_pop)
                      with-precondition-source)))))))))

(defn property-runner-append-typed-test-cases-loop
  [contracts idx count results]
  (if (>= idx count)
    results
    (let [contract (vector-get contracts idx)
      ;; vector-push は容量内では同じベクタを更新するため、追加前の index を保存する。
      append-index (vector-length results)
      test-case (property-runner-typed-contract-test-case
        contract
        append-index)
      next-results (vector-push-single-rooted results test-case)]
      (do
        (root_push next-results)
        (let [parsed (property-runner-append-typed-test-cases-loop
            contracts
            (+ idx 1)
            count
            next-results)]
          (do
            (root_pop)
            parsed))))))

(defn property-test-case-profile-code [test-case]
  (vector-get test-case 5))

(defn property-test-case-owner [test-case]
  (vector-get test-case 1))

(defn property-test-case-binders [test-case]
  (vector-get test-case 2))

(defn property-test-case-postcondition [test-case]
  (vector-get test-case 3))

(defn property-test-case-postcondition-source [test-case]
  (if (> (vector-length test-case) 8)
    (vector-get test-case 8)
    ""))

(defn property-test-case-precondition-source [test-case]
  (if (> (vector-length test-case) 9)
    (vector-get test-case 9)
    ""))

(defn property-test-case-count [test-case]
  (vector-get test-case 4))

(defn property-test-case-preconditions [test-case]
  (if (> (vector-length test-case) 6)
    (vector-get test-case 6)
    (vector-new 0)))

(defn property-test-case-binder-types [test-case]
  (if (> (vector-length test-case) 7)
    (vector-get test-case 7)
    (vector-new 0)))

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
