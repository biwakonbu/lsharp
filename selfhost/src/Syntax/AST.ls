(module Syntax.AST)
(import Syntax.Token)
(defn ast-lit-int [] 1)
(defn ast-lit-bool [] 2)
(defn ast-lit-string [] 3)
(defn ast-var [] 4)
(defn ast-apply [] 5)
(defn ast-if [] 6)
(defn ast-let [] 7)
(defn ast-lambda [] 8)
(defn ast-do [] 9)
(defn ast-match [] 10)
(defn ast-ann [] 11)
(defn ast-recordlit [] 12)
(defn ast-fieldaccess [] 13)
(defn ast-recordupdate [] 14)
(defn ast-computation [] 15)
(defn ast-quote [] 16)
(defn ast-unquote [] 17)
(defn ast-unquote-splice [] 18)
(defn ast-lit-float [] 19)
(defn ast-lit-unit [] 32)
(defn computation-step-expr [] 0)
(defn computation-step-let-bang [] 1)
(defn computation-step-do-bang [] 2)
(defn computation-step-return [] 3)
(defn ast-defn [] 20)
(defn ast-typedef [] 21)
(defn ast-type-decl [] 21)
(defn ast-recorddef [] 22)
(defn ast-typealias [] 23)
(defn ast-typeconstrained [] 24)
(defn ast-module-decl [] 25)
(defn ast-import-decl [] 26)
(defn ast-traitdef [] 27)
(defn ast-impldef [] 28)
(defn ast-private [] 29)
(defn ast-computationbuilder [] 30)
(defn ast-defmacro [] 31)
(defn ast-type-named [] 60)
(defn ast-type-app [] 61)
(defn ast-type-fun [] 62)
(defn ast-type-var [] 63)
(defn ast-defn-signature [] 65)
(defn contract-form-example [] 1)
(defn contract-form-invariant [] 2)
(defn contract-form-assert [] 3)
(defn contract-form-case [] 4)
(defn contract-form-property [] 5)
(defn ast-pat-wildcard [] 40)
(defn ast-pat-var [] 41)
(defn ast-pat-lit [] 42)
(defn ast-pat-constructor [] 43)
(defn ast-pat-recordpat [] 44)
(defn ast-match-guard [] 45)
(defn vector-push-single-rooted [base value]
  (do
    (root_push value)
    (let [base-slot (root_push base)
      result (vector-push base value)]
      (do
        (root_set base-slot result)
        (root_pop)
        (root_pop)
        result))))

(defn vector-push-pair-rooted [base first second]
  (do
    (root_push first)
    (root_push second)
    (let [base-slot (root_push base)
      with-first (vector-push base first)]
      (do
        (root_set base-slot with-first)
        (let [result (vector-push with-first second)]
          (do
            (root_pop)
            (root_pop)
            (root_pop)
            result))))))

(defn vector-push-triple-rooted [base first second third]
  (do
    (root_push first)
    (root_push second)
    (root_push third)
    (let [base-slot (root_push base)
      with-first (vector-push base first)]
      (do
        (root_set base-slot with-first)
        (let [with-second (vector-push with-first second)]
          (do
            (root_set base-slot with-second)
            (let [result (vector-push with-second third)]
              (do
                (root_pop)
                (root_pop)
                (root_pop)
                (root_pop)
                result))))))))

(defn vector-push-quad-rooted [base first second third fourth]
  (do
    (root_push first)
    (root_push second)
    (root_push third)
    (root_push fourth)
    (let [base-slot (root_push base)
      with-first (vector-push base first)]
      (do
        (root_set base-slot with-first)
        (let [with-second (vector-push with-first second)]
          (do
            (root_set base-slot with-second)
            (let [with-third (vector-push with-second third)]
              (do
                (root_set base-slot with-third)
                (let [result (vector-push with-third fourth)]
                  (do
                    (root_pop)
                    (root_pop)
                    (root_pop)
                    (root_pop)
                    (root_pop)
                    result))))))))))

(defn make-lit-int [value] (vector-push-pair-rooted (vector-new 2) 1 value))
(defn make-lit-float [start end] (vector-push-triple-rooted (vector-new 3) (ast-lit-float) start end))
(defn make-lit-bool [b] (vector-push-pair-rooted (vector-new 2) 2 b))
(defn make-var [name-hash] (vector-push-pair-rooted (vector-new 2) 4 name-hash))
(defn make-lit-unit [] (vector-push-single-rooted (vector-new 1) (ast-lit-unit)))
(defn make-if [cond-expr then-expr else-expr] (vector-push-triple-rooted (vector-push-single-rooted (vector-new 4) (ast-if)) cond-expr then-expr else-expr))
(defn make-let [name-hash init-expr body-expr] (vector-push-pair-rooted (vector-push-pair-rooted (vector-new 4) (ast-let) name-hash) init-expr body-expr))
(defn ast-qualified-name-hash [prefix-hash suffix-hash]
  (+ (* prefix-hash 131) suffix-hash))
;; 互換用の annotation constructor。型 payload 不在は 0 として扱う。
(defn make-ann [expr]
  (vector-push-triple-rooted (vector-new 3) (ast-ann) expr 0))

;; raw TypeExpr の named form: [60, source-name-hash]
(defn make-type-named [name-hash]
  (vector-push-pair-rooted (vector-new 2) (ast-type-named) name-hash))

;; raw TypeExpr の型変数: [63, source-name-hash]
(defn make-type-var-expr [name-hash]
  (vector-push-pair-rooted (vector-new 2) (ast-type-var) name-hash))

;; raw TypeExpr の object payload を左から複写する。
(defn type-expr-append-items [items idx len out]
  (if (>= idx len)
    out
    (type-expr-append-items
      items
      (+ idx 1)
      len
      (vector-push-single-rooted out (vector-get items idx)))))

(defn type-expr-prefix [items count]
  (do
    (root_push items)
    (let [result (type-expr-append-items items 0 count (vector-new count))]
      (do
        (root_pop)
        result))))

;; raw TypeExpr の applied form: [61, source-name-hash, arg-count, arg...]
(defn make-type-app-expr [name-hash args]
  (do
    (root_push args)
    (let [arg-count (vector-length args)
      prefix
        (vector-push
          (vector-push
            (vector-push (vector-new (+ arg-count 3)) (ast-type-app))
            name-hash)
          arg-count)
      result (type-expr-append-items args 0 arg-count prefix)]
      (do
        (root_pop)
        result))))

;; raw TypeExpr の function form: [62, param-count, param..., return-type]
(defn make-type-fun-expr [params return-type]
  (do
    (root_push params)
    (root_push return-type)
    (let [param-count (vector-length params)
      prefix
        (vector-push
          (vector-push (vector-new (+ param-count 3)) (ast-type-fun))
          param-count)
      with-params (type-expr-append-items params 0 param-count prefix)
      result (vector-push-single-rooted with-params return-type)]
      (do
        (root_pop)
        (root_pop)
        result))))

;; annotation: [11, expr, raw-type-expr]
(defn make-ann-typed [expr type-expr]
  (vector-push-triple-rooted (vector-new 3) (ast-ann) expr type-expr))
(defn make-defn-signature [param-count]
  (vector-push-pair-rooted (vector-new 2) (ast-defn-signature) param-count))
(defn make-recordlit [type-name-hash] (vector-push-triple-rooted (vector-new 8) (ast-recordlit) type-name-hash 0))
(defn make-fieldaccess [expr field-name-hash] (vector-push-triple-rooted (vector-new 3) (ast-fieldaccess) expr field-name-hash))
(defn make-recordupdate [base-expr] (vector-push-triple-rooted (vector-new 8) (ast-recordupdate) base-expr 0))
(defn make-computation [builder-hash] (vector-push-triple-rooted (vector-new 8) (ast-computation) builder-hash 0))
(defn make-type-decl [name-hash] (vector-push-pair-rooted (vector-new 2) (ast-type-decl) name-hash))
(defn make-record-def [name-hash] (vector-push-pair-rooted (vector-new 2) (ast-recorddef) name-hash))
;; record 定義: [22, name-hash, field 名/accessor 名/型式を並べた vector]
(defn make-record-def-with-fields [name-hash fields]
  (vector-push-triple-rooted (vector-new 3) (ast-recorddef) name-hash fields))
;; parametric record 定義: [22, name-hash, parameter 名 vector, field 名/accessor 名/型式 vector]
(defn make-record-def-with-params [name-hash params fields]
  (vector-push-quad-rooted (vector-new 4) (ast-recorddef) name-hash params fields))
;; ADT variant: [constructor 名, raw field TypeExpr vector]
(defn make-type-variant [name-hash fields]
  (vector-push-pair-rooted (vector-new 2) name-hash fields))
;; GADT variant: [constructor 名, raw field TypeExpr vector, raw return TypeExpr]
(defn make-type-variant-with-return-type [name-hash fields return-type]
  (vector-push-triple-rooted (vector-new 3) name-hash fields return-type))
;; nonparametric ADT: [21, type 名, variant vector]
(defn make-type-decl-with-variants [name-hash variants]
  (vector-push-triple-rooted (vector-new 3) (ast-type-decl) name-hash variants))
;; parametric ADT: [21, type 名, parameter 名 vector, variant vector]
(defn make-type-decl-with-params-and-variants [name-hash params variants]
  (vector-push-quad-rooted (vector-new 4) (ast-type-decl) name-hash params variants))
;; closed type-alias は zero-parameter alias と同じ形で保持する。
;; [23, name-hash, empty-params, raw-target-type-expr]
;; parametric type-alias: [23, name-hash, param-name-hashes, raw-target-type-expr]
(defn make-type-alias [name-hash target-type-expr]
  ;; native の regular alias も、実績のある zero-parameter alias 経路へ揃える。
  (vector-push-quad-rooted (vector-new 4) (ast-typealias) name-hash (vector-new 0) target-type-expr))

(defn make-type-alias-with-params [name-hash params target-type-expr]
  (vector-push-quad-rooted (vector-new 4) (ast-typealias) name-hash params target-type-expr))
(defn make-type-constrained [name-hash] (vector-push-pair-rooted (vector-new 2) (ast-typeconstrained) name-hash))
(defn make-module-decl [name-hash] (vector-push-triple-rooted (vector-new 8) (ast-module-decl) name-hash 0))
(defn make-module-decl-with-span [name-hash name-start name-end]
  (vector-push-single-rooted
    (vector-push-quad-rooted (vector-new 8) (ast-module-decl) name-hash 0 name-start)
    name-end))
;; import 宣言: [26, module-name-hash, module-start, module-end]
;; :as を含む場合は [26, module-name-hash, module-start, module-end, alias-hash]
;; :only を含む場合は [26, module-name-hash, module-start, module-end, alias-hash, only-hashes]
;; :open を含む場合は [26, module-name-hash, module-start, module-end, 0, 0, 1]
;; :as + :only を含む場合は [26, module-name-hash, module-start, module-end, alias-hash, only-hashes]
;; :open + :only を含む場合は [26, module-name-hash, module-start, module-end, 0, only-hashes, 1]
;; :as + :open + :only を含む場合は [26, module-name-hash, module-start, module-end, alias-hash, only-hashes, 1]
(defn make-import-decl [name-hash name-start name-end]
  (vector-push-pair-rooted
    (vector-push-pair-rooted (vector-new 4) (ast-import-decl) name-hash)
    name-start
    name-end))
(defn make-import-decl-with-alias [name-hash name-start name-end alias-hash]
  (vector-push-single-rooted
    (vector-push-quad-rooted (vector-new 5) (ast-import-decl) name-hash name-start name-end)
    alias-hash))
(defn make-import-decl-with-only [name-hash name-start name-end only-hashes]
  (vector-push-pair-rooted
    (vector-push-quad-rooted (vector-new 6) (ast-import-decl) name-hash name-start name-end)
    0
    only-hashes))
(defn make-import-decl-with-open [name-hash name-start name-end]
  (vector-push-triple-rooted
    (vector-push-quad-rooted (vector-new 8) (ast-import-decl) name-hash name-start name-end)
    0
    0
    1))
(defn make-import-decl-with-only-and-open [name-hash name-start name-end only-hashes]
  (vector-push-single-rooted
    (vector-push-pair-rooted
      (vector-push-quad-rooted (vector-new 8) (ast-import-decl) name-hash name-start name-end)
      0
      only-hashes)
    1))
(defn make-import-decl-with-alias-and-only [name-hash name-start name-end alias-hash only-hashes]
  (vector-push-pair-rooted
    (vector-push-quad-rooted (vector-new 6) (ast-import-decl) name-hash name-start name-end)
    alias-hash
    only-hashes))
(defn make-import-decl-with-alias-and-open [name-hash name-start name-end alias-hash]
  (vector-push-single-rooted
    (vector-push-single-rooted
      (vector-push-quad-rooted (vector-new 8) (ast-import-decl) name-hash name-start name-end)
      alias-hash)
    1))
(defn make-import-decl-with-alias-only-and-open [name-hash name-start name-end alias-hash only-hashes]
  (vector-push-single-rooted
    (vector-push-pair-rooted
      (vector-push-quad-rooted (vector-new 8) (ast-import-decl) name-hash name-start name-end)
      alias-hash
      only-hashes)
    1))
(defn make-import-decl-from-options
  [name-hash name-start name-end alias-hash only-present only-hashes open-present]
  (if (= open-present 1)
    (if (= only-present 1)
      (if (= alias-hash 0)
        (make-import-decl-with-only-and-open name-hash name-start name-end only-hashes)
        (make-import-decl-with-alias-only-and-open
          name-hash
          name-start
          name-end
          alias-hash
          only-hashes))
      (if (= alias-hash 0)
        (make-import-decl-with-open name-hash name-start name-end)
        (make-import-decl-with-alias-and-open name-hash name-start name-end alias-hash)))
    (if (= only-present 1)
      (if (= alias-hash 0)
        (make-import-decl-with-only name-hash name-start name-end only-hashes)
        (make-import-decl-with-alias-and-only name-hash name-start name-end alias-hash only-hashes))
      (if (= alias-hash 0)
        (make-import-decl name-hash name-start name-end)
        (make-import-decl-with-alias name-hash name-start name-end alias-hash)))))
(defn import-decl-alias-hash [decl]
  (if (> (vector-length decl) 4) (vector-get decl 4) 0))
(defn make-trait-def [name-hash] (vector-push-triple-rooted (vector-new 8) (ast-traitdef) name-hash 0))
(defn make-impl-def [trait-name-hash type-name-hash] (vector-push-pair-rooted (vector-push-pair-rooted (vector-new 8) (ast-impldef) trait-name-hash) type-name-hash 0))
(defn make-private [inner-node] (vector-push-pair-rooted (vector-new 2) (ast-private) inner-node))
(defn make-computation-builder [name-hash bind-hash return-hash] (vector-push-pair-rooted (vector-push-pair-rooted (vector-new 4) (ast-computationbuilder) name-hash) bind-hash return-hash))
(defn make-defmacro [name-hash] (vector-push-triple-rooted (vector-new 3) (ast-defmacro) name-hash 0))
(defn make-quote [expr] (vector-push-pair-rooted (vector-new 2) (ast-quote) expr))
(defn make-unquote [expr] (vector-push-pair-rooted (vector-new 2) (ast-unquote) expr))
(defn make-unquote-splice [expr] (vector-push-pair-rooted (vector-new 2) (ast-unquote-splice) expr))
(defn make-match-guard [guard body]
  (vector-push-triple-rooted (vector-new 3) (ast-match-guard) guard body))
(defn ast-tag [node] (vector-get node 0))
(defn ast-is-leaf [tag] (if (= tag 1) 1 (if (= tag 2) 1 (if (= tag 4) 1 (if (= tag 3) 1 (if (= tag 19) 1 (if (= tag 32) 1 0)))))))
(defn recordlit-contains-var-loop [node target-hash idx count] (if (>= idx count) 0 (if (= (ast-contains-var (vector-get node (+ 4 (* idx 2))) target-hash) 1) 1 (recordlit-contains-var-loop node target-hash (+ idx 1) count))))
(defn recordupdate-contains-var-loop [node target-hash idx count] (if (>= idx count) 0 (if (= (ast-contains-var (vector-get node (+ 4 (* idx 2))) target-hash) 1) 1 (recordupdate-contains-var-loop node target-hash (+ idx 1) count))))
(defn computation-contains-var-loop [node target-hash idx count] (if (>= idx count) 0 (if (= (ast-contains-var (vector-get node (+ 5 (* idx 3))) target-hash) 1) 1 (computation-contains-var-loop node target-hash (+ idx 1) count))))
(defn apply-contains-var-loop [node target-hash idx count] (if (>= idx count) 0 (if (= (ast-contains-var (vector-get node (+ 3 idx)) target-hash) 1) 1 (apply-contains-var-loop node target-hash (+ idx 1) count))))
(defn do-contains-var-loop [node target-hash idx count] (if (>= idx count) 0 (if (= (ast-contains-var (vector-get node (+ 2 idx)) target-hash) 1) 1 (do-contains-var-loop node target-hash (+ idx 1) count))))
(defn match-contains-var-loop [node target-hash idx count] (if (>= idx count) 0 (if (= (ast-contains-var (vector-get node (+ 4 (* idx 2))) target-hash) 1) 1 (match-contains-var-loop node target-hash (+ idx 1) count))))
(defn if-contains-var [node target-hash] (if (= (ast-contains-var (vector-get node 1) target-hash) 1) 1 (if (= (ast-contains-var (vector-get node 2) target-hash) 1) 1 (ast-contains-var (vector-get node 3) target-hash))))
(defn let-contains-var [node target-hash] (if (= (ast-contains-var (vector-get node 2) target-hash) 1) 1 (ast-contains-var (vector-get node 3) target-hash)))
(defn apply-contains-var [node target-hash] (apply-contains-var-loop node target-hash 0 (vector-get node 2)))
(defn do-contains-var [node target-hash] (do-contains-var-loop node target-hash 0 (vector-get node 1)))
(defn match-contains-var [node target-hash] (let [scrutinee-found (ast-contains-var (vector-get node 1) target-hash)] (if (= scrutinee-found 1) 1 (match-contains-var-loop node target-hash 0 (vector-get node 2)))))
(defn ast-contains-var [node target-hash]
  (let [tag (vector-get node 0)]
    (if (= tag 4)
      (if (= (vector-get node 1) target-hash) 1 0)
      (if (= tag 1) 0
        (if (= tag 2) 0
          (if (= tag 3) 0
            (if (= tag 11)
              (ast-contains-var (vector-get node 1) target-hash)
              (if (= tag 12)
                (recordlit-contains-var-loop node target-hash 0 (vector-get node 2))
                (if (= tag 13)
                  (ast-contains-var (vector-get node 1) target-hash)
                  (if (= tag 14)
                    (if (= (ast-contains-var (vector-get node 1) target-hash) 1) 1
                      (recordupdate-contains-var-loop node target-hash 0 (vector-get node 2)))
                    (if (= tag 15)
                      (computation-contains-var-loop node target-hash 0 (vector-get node 2))
                      (if (= tag 16)
                        (ast-contains-var (vector-get node 1) target-hash)
                        (if (= tag 17)
                          (ast-contains-var (vector-get node 1) target-hash)
                          (if (= tag 18)
                            (ast-contains-var (vector-get node 1) target-hash)
                            (if (= tag 6)
                              (if-contains-var node target-hash)
                              (if (= tag 7)
                                (let-contains-var node target-hash)
                                (if (= tag 5)
                                  (apply-contains-var node target-hash)
                                  (if (= tag 9)
                                    (do-contains-var node target-hash)
                                    (if (= tag 10)
                                      (match-contains-var node target-hash)
                                      0)))))))))))))))))))
(defn recordlit-count-fields-loop [node idx count] (if (>= idx count) 0 (+ (ast-count-nodes (vector-get node (+ 4 (* idx 2)))) (recordlit-count-fields-loop node (+ idx 1) count))))
(defn recordupdate-count-fields-loop [node idx count] (if (>= idx count) 0 (+ (ast-count-nodes (vector-get node (+ 4 (* idx 2)))) (recordupdate-count-fields-loop node (+ idx 1) count))))
(defn computation-count-steps-loop [node idx count] (if (>= idx count) 0 (+ (ast-count-nodes (vector-get node (+ 5 (* idx 3)))) (computation-count-steps-loop node (+ idx 1) count))))
(defn ast-count-nodes [node] (let [tag (vector-get node 0)] (if (= (ast-is-leaf tag) 1) 1 (if (= tag 11) (+ 1 (ast-count-nodes (vector-get node 1))) (if (= tag 12) (+ 1 (recordlit-count-fields-loop node 0 (vector-get node 2))) (if (= tag 13) (+ 1 (ast-count-nodes (vector-get node 1))) (if (= tag 14) (+ 1 (+ (ast-count-nodes (vector-get node 1)) (recordupdate-count-fields-loop node 0 (vector-get node 2)))) (if (= tag 15) (+ 1 (computation-count-steps-loop node 0 (vector-get node 2))) (if (= tag 16) (+ 1 (ast-count-nodes (vector-get node 1))) (if (= tag 17) (+ 1 (ast-count-nodes (vector-get node 1))) (if (= tag 18) (+ 1 (ast-count-nodes (vector-get node 1))) (if (= tag 6) (+ 1 (+ (ast-count-nodes (vector-get node 1)) (+ (ast-count-nodes (vector-get node 2)) (ast-count-nodes (vector-get node 3))))) (if (= tag 7) (+ 1 (+ (ast-count-nodes (vector-get node 2)) (ast-count-nodes (vector-get node 3)))) (if (= tag 5) (let [argc (vector-get node 2)] (if (> argc 0) (if (> argc 1) (+ 1 (+ (ast-count-nodes (vector-get node 3)) (ast-count-nodes (vector-get node 4)))) (+ 1 (ast-count-nodes (vector-get node 3)))) 1)) (if (= tag 9) (let [ec (vector-get node 1)] (if (> ec 0) (if (> ec 1) (if (> ec 2) (if (> ec 3) (if (> ec 4) (+ 1 (+ (ast-count-nodes (vector-get node 2)) (+ (ast-count-nodes (vector-get node 3)) (+ (ast-count-nodes (vector-get node 4)) (+ (ast-count-nodes (vector-get node 5)) (ast-count-nodes (vector-get node 6))))))) (+ 1 (+ (ast-count-nodes (vector-get node 2)) (+ (ast-count-nodes (vector-get node 3)) (+ (ast-count-nodes (vector-get node 4)) (ast-count-nodes (vector-get node 5))))))) (+ 1 (+ (ast-count-nodes (vector-get node 2)) (+ (ast-count-nodes (vector-get node 3)) (ast-count-nodes (vector-get node 4)))))) (+ 1 (+ (ast-count-nodes (vector-get node 2)) (ast-count-nodes (vector-get node 3))))) (+ 1 (ast-count-nodes (vector-get node 2)))) 1)) (if (= tag 10) (let [ac (vector-get node 2) sc (ast-count-nodes (vector-get node 1))] (if (> ac 0) (if (> ac 1) (if (> ac 2) (+ 1 (+ sc (+ (+ (ast-count-nodes (vector-get node 3)) (ast-count-nodes (vector-get node 4))) (+ (+ (ast-count-nodes (vector-get node 5)) (ast-count-nodes (vector-get node 6))) (+ (ast-count-nodes (vector-get node 7)) (ast-count-nodes (vector-get node 8))))))) (+ 1 (+ sc (+ (+ (ast-count-nodes (vector-get node 3)) (ast-count-nodes (vector-get node 4))) (+ (ast-count-nodes (vector-get node 5)) (ast-count-nodes (vector-get node 6))))))) (+ 1 (+ sc (+ (ast-count-nodes (vector-get node 3)) (ast-count-nodes (vector-get node 4)))))) (+ 1 sc))) 1))))))))))))))))
(defn demo-main [] (let [lit (make-lit-int 42) var1 (make-var 99) if-node (let [v (vector-new 4)] (vector-push (vector-push (vector-push (vector-push v 6) var1) lit) (make-lit-int 0))) let-node (let [v (vector-new 4)] (vector-push (vector-push (vector-push (vector-push v 7) 99) lit) if-node))] (do (print (ast-tag lit)) (print (vector-get lit 1)) (print (ast-match)) (print (ast-is-leaf 1)) (print (ast-is-leaf 6)) (print (ast-count-nodes lit)) (print (ast-count-nodes if-node)) (print (ast-contains-var if-node 99)) (print (ast-contains-var if-node 88)) (print (ast-contains-var let-node 99)) (let [do-node (vector-push (vector-push (vector-push (vector-push (vector-new 4) 9) 2) var1) (make-lit-int 0))] (print (ast-count-nodes do-node))) (let [match-node (vector-push (vector-push (vector-push (vector-push (vector-push (vector-new 6) 10) (make-lit-int 0)) 1) (make-lit-int 1)) var1)] (print (ast-count-nodes match-node))) 0)))
