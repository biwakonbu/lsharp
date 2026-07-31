(module Types.TypeInferRecord)
(import Syntax.AST)
(import Types.Type)
(import Types.TypeScheme)
(import Types.TypeInferCore)
(import Types.TypeInfer)

;; TypeInferRecord.ls - レコード型の型推論
;;
;; infer-record-fields: record field 値群の順次推論
;; infer-recordlit-fields: record literal 用に field 型を保持して推論
;; recordlit-field-node-scan: record literal から特定 field の value node 取得 (bounded scan)
;; recordlit-field-node: record literal から特定 field の value node 取得
;; infer-recordlit: record literal の型推論
;; infer-fieldaccess: field access の型推論
;; infer-recordupdate-node: record update の型推論

;; record field value 群を順に推論する
;; node は [tag, ..., field-count, field1-hash, expr1, ...]
(defn infer-record-fields-state [done next-idx result]
  (vector-push-triple-rooted (vector-new 3) done next-idx result))

(defn infer-record-fields-step-v3
  [node idx count env current-result counter]
  (if (>= idx count)
    (infer-record-fields-state 1 idx current-result)
    (do
      (root_push node)
      (root_push env)
      (root_push current-result)
      (root_push counter)
      (let [value-node (vector-get node (+ 4 (* idx 2)))]
        (do
          (root_push value-node)
          (let [value-result
                  (infer-expr
                    value-node
                    env
                    (result-subst current-result)
                    counter)]
            (do
              (root_push value-result)
              (let [next-result
                      (if (= (result-failed value-result) 1)
                        (propagate-error-result value-result)
                        (make-result (result-subst value-result) (mk-int)))]
                (do
                  (root_push next-result)
                  (let [state
                          (if (= (result-failed next-result) 1)
                            (infer-record-fields-state 1 idx next-result)
                            (infer-record-fields-state 0 (+ idx 1) next-result))]
                    (do
                      (root_pop)
                      (root_pop)
                      (root_pop)
                      (root_pop)
                      (root_pop)
                      (root_pop)
                      (root_pop)
                      state)))))))))))

(defn infer-record-fields-step-64-loop-bounded
  [node idx count env current-result counter remaining]
  (do
    (root_push node)
    (root_push env)
    (root_push current-result)
    (root_push counter)
    (let [step
            (infer-record-fields-step-v3
              node idx count env current-result counter)
          done (vector-get step 0)
          next-idx (vector-get step 1)
          next-result (vector-get step 2)]
      (do
        (root_push step)
        (root_push next-result)
        (let [parsed
                (if (= done 1)
                  step
                  (if (<= remaining 1)
                    step
                    (infer-record-fields-step-64-loop-bounded
                      node
                      next-idx
                      count
                      env
                      next-result
                      counter
                      (- remaining 1))))]
          (do
            (root_pop)
            (root_pop)
            (root_pop)
            (root_pop)
            (root_pop)
            (root_pop)
            parsed))))))

(defn infer-record-fields-step-64
  [node idx count env current-result counter]
  (infer-record-fields-step-64-loop-bounded
    node idx count env current-result counter 64))

(defn infer-record-fields-rooted-v3
  [node idx count env current-result counter]
  (let [step
          (infer-record-fields-step-64
            node idx count env current-result counter)]
    (if (= (vector-get step 0) 1)
      (vector-get step 2)
      (do
        (root_push step)
        (let [next-idx (vector-get step 1)
              next-result (vector-get step 2)]
          (do
            (root_push next-result)
            (let [resolved
                    (infer-record-fields-rooted-v3
                      node
                      next-idx
                      count
                      env
                      next-result
                      counter)]
              (do
                (root_pop)
                (root_pop)
                resolved))))))))

(defn infer-record-fields [node idx count env subst counter]
  (do
    (root_push subst)
    (let [initial (make-result subst (mk-int))]
      (do
        (root_push initial)
        (let [result
                (infer-record-fields-rooted-v3
                  node idx count env initial counter)]
          (do
            (root_pop)
            (root_pop)
            result))))))

;; record literal 用に field 型を保持しながら順に推論する
(defn infer-recordlit-fields [node idx count env subst counter record-ty]
  (make-result subst record-ty))

;; record literal から特定 field の value node を取り出す
;; 見つからない場合は 0 を返す
(defn recordlit-field-node-state [done next-idx result]
  (vector-push-triple-rooted (vector-new 3) done next-idx result))

(defn recordlit-field-node-step-v3
  [record-node field-name-hash idx field-count]
  (if (>= idx field-count)
    (recordlit-field-node-state 1 idx 0)
    (do
      (root_push record-node)
      (let [field-offset (+ 3 (* idx 2))
            current-field-hash (vector-get record-node field-offset)]
        (if (= current-field-hash field-name-hash)
          (let [field-node (vector-get record-node (+ field-offset 1))]
            (do
              (root_push field-node)
              (let [state (recordlit-field-node-state 1 idx field-node)]
                (do
                  (root_pop)
                  (root_pop)
                  state))))
          (let [state
                  (recordlit-field-node-state 0 (+ idx 1) 0)]
            (do
              (root_pop)
              state)))))))

(defn recordlit-field-node-step-64-loop-bounded
  [record-node field-name-hash idx field-count remaining]
  (do
    (root_push record-node)
    (let [step
            (recordlit-field-node-step-v3
              record-node field-name-hash idx field-count)
          done (vector-get step 0)
          next-idx (vector-get step 1)
          next-result (vector-get step 2)]
      (do
        (root_push step)
        (root_push next-result)
        (let [parsed
                (if (= done 1)
                  step
                  (if (<= remaining 1)
                    step
                    (recordlit-field-node-step-64-loop-bounded
                      record-node
                      field-name-hash
                      next-idx
                      field-count
                      (- remaining 1))))]
          (do
            (root_pop)
            (root_pop)
            (root_pop)
            parsed))))))

(defn recordlit-field-node-step-64
  [record-node field-name-hash idx field-count]
  (recordlit-field-node-step-64-loop-bounded
    record-node field-name-hash idx field-count 64))

(defn recordlit-field-node-rooted-v3
  [record-node field-name-hash idx field-count]
  (let [step
          (recordlit-field-node-step-64
            record-node field-name-hash idx field-count)]
    (if (= (vector-get step 0) 1)
      (vector-get step 2)
      (do
        (root_push step)
        (let [next-idx (vector-get step 1)
              next-result (vector-get step 2)]
          (do
            (root_push next-result)
            (let [resolved
                    (recordlit-field-node-rooted-v3
                      record-node field-name-hash next-idx field-count)]
              (do
                (root_pop)
                (root_pop)
                resolved))))))))

(defn recordlit-field-node [record-node field-name-hash]
  (recordlit-field-node-rooted-v3
    record-node
    field-name-hash
    0
    (vector-get record-node 2)))

(defn infer-declared-recordlit-fields-state [done next-idx result]
  (vector-push-triple-rooted (vector-new 3) done next-idx result))

(defn infer-declared-recordlit-fields-step-v3
  [node idx count env current-result counter record-ty]
  (if (>= idx count)
    (infer-declared-recordlit-fields-state 1 idx current-result)
    (do
      (root_push node)
      (root_push env)
      (root_push current-result)
      (root_push counter)
      (root_push record-ty)
      (let [field-offset (+ 3 (* idx 2))
            field-name-hash (vector-get node field-offset)
            value-node (vector-get node (+ field-offset 1))
            expected-ty (type-record-field-type record-ty field-name-hash)]
        (do
          (root_push expected-ty)
          (root_push value-node)
          (let [field-result
                  (if (= expected-ty 0)
                    (make-error-result-code (error-code-general))
                    (let [value-result
                              (infer-expr
                                value-node
                                env
                                (result-subst current-result)
                                counter)]
                        (do
                          (root_push value-result)
                          (let [next-result
                                  (if (= (result-failed value-result) 1)
                                    (propagate-error-result value-result)
                                    (let [next-subst
                                            (unify
                                              expected-ty
                                              (result-type value-result)
                                              (result-subst value-result))]
                                      (do
                                        (root_push next-subst)
                                        (let [unified-result
                                                (if (= (unify-failed next-subst) 1)
                                                  (make-error-result-code (error-code-general))
                                                  (make-result next-subst record-ty))]
                                          (do
                                            (root_pop)
                                            unified-result)))))]
                            (do
                              (root_push next-result)
                              (let [result next-result]
                                (do
                                  (root_pop)
                                  (root_pop)
                                                                    result)))))))]
            (do
              (root_push field-result)
              (let [state
                      (if (= (result-failed field-result) 1)
                        (infer-declared-recordlit-fields-state 1 idx field-result)
                        (infer-declared-recordlit-fields-state
                          0 (+ idx 1) field-result))]
                (do
                  (root_pop)
                  (root_pop)
                  (root_pop)
                  (root_pop)
                  (root_pop)
                  (root_pop)
                  (root_pop)
                  (root_pop)
                  state)))))))))

(defn infer-declared-recordlit-fields-step-64-loop-bounded
  [node idx count env current-result counter record-ty remaining]
  (do
    (root_push node)
    (root_push env)
    (root_push current-result)
    (root_push counter)
    (root_push record-ty)
    (let [step
            (infer-declared-recordlit-fields-step-v3
              node idx count env current-result counter record-ty)
          done (vector-get step 0)
          next-idx (vector-get step 1)
          next-result (vector-get step 2)]
      (do
        (root_push step)
        (root_push next-result)
        (let [parsed
                (if (= done 1)
                  step
                  (if (<= remaining 1)
                    step
                    (infer-declared-recordlit-fields-step-64-loop-bounded
                      node
                      next-idx
                      count
                      env
                      next-result
                      counter
                      record-ty
                      (- remaining 1))))]
          (do
            (root_pop)
            (root_pop)
            (root_pop)
            (root_pop)
            (root_pop)
            (root_pop)
            (root_pop)
            parsed))))))

(defn infer-declared-recordlit-fields-step-64
  [node idx count env current-result counter record-ty]
  (infer-declared-recordlit-fields-step-64-loop-bounded
    node idx count env current-result counter record-ty 64))

(defn infer-declared-recordlit-fields-rooted-v3
  [node idx count env current-result counter record-ty]
  (let [step
          (infer-declared-recordlit-fields-step-64
            node idx count env current-result counter record-ty)]
    (if (= (vector-get step 0) 1)
      (vector-get step 2)
      (do
        (root_push step)
        (let [next-idx (vector-get step 1)
              next-result (vector-get step 2)]
          (do
            (root_push next-result)
            (let [resolved
                    (infer-declared-recordlit-fields-rooted-v3
                      node
                      next-idx
                      count
                      env
                      next-result
                      counter
                      record-ty)]
              (do
                (root_pop)
                (root_pop)
                resolved))))))))

(defn infer-declared-recordlit-fields
  [node idx count env subst counter record-ty]
  (do
    (root_push subst)
    (root_push record-ty)
    (let [initial (make-result subst record-ty)]
      (do
        (root_push initial)
        (let [result
                (infer-declared-recordlit-fields-rooted-v3
                  node idx count env initial counter record-ty)]
          (do
            (root_pop)
            (root_pop)
            (root_pop)
            result))))))

;; qualified record literal は visible な constructor scheme の戻り型から
;; record schema を取得する。record-env に raw 名しかない import 境界でも、
;; qualified export key が値環境に存在する間だけ field 型検査を有効にする。
(defn infer-recordlit-constructor-result-type [ty]
  (if (= (ty-tag ty) (ty-fun))
    (infer-recordlit-constructor-result-type (ty-fr ty))
    (if (= (ty-tag ty) (ty-record)) ty 0)))

(defn infer-recordlit-visible-record-type [node env counter]
  (let [scheme (type-env-lookup env (vector-get node 1))]
    (if (= scheme 0)
      0
      (do
        (root_push scheme)
        (let [instantiated (instantiate scheme counter)]
          (do
            (root_push instantiated)
            (let [result (infer-recordlit-constructor-result-type instantiated)]
              (do
                (root_pop)
                (root_pop)
                result))))))))

(defn infer-recordlit-with-record-type [node field-count env subst counter record-ty]
  (do
    (root_push record-ty)
    (let [result
            (if (= field-count (/ (- (vector-length record-ty) 2) 2))
              (infer-declared-recordlit-fields
                node
                0
                field-count
                env
                subst
                counter
                record-ty)
              (make-error-result-code (error-code-general)))]
      (do
        (root_pop)
        result))))

(defn infer-recordlit-is-qualified [node]
  (let [marker-index (+ 3 (* (vector-get node 2) 2))]
    (if (> (vector-length node) marker-index)
      (vector-get node marker-index)
      0)))

;; record literal の型推論
;; [12, type-name-hash, field-count, field1-hash, expr1, ...]
(defn infer-recordlit [node env subst counter]
  (let [type-name-hash (vector-get node 1)
    field-count (vector-get node 2)
    record-schema (map-get-safe (var-counter-record-env counter) type-name-hash)
    fields-result
      (if (= record-schema 0)
        (let [visible-record-ty
                (infer-recordlit-visible-record-type node env counter)]
          (if (= visible-record-ty 0)
            (if (= (infer-recordlit-is-qualified node) 1)
              (make-error-result-code (error-code-general))
              (infer-record-fields node 0 field-count env subst counter))
            (do
              (root_push visible-record-ty)
              (let [result
                      (infer-recordlit-with-record-type
                        node
                        field-count
                        env
                        subst
                        counter
                        visible-record-ty)]
                (do
                  (root_pop)
                  result)))))
        (if (= (type-env-lookup env type-name-hash) 0)
          ;; record-env は全 program の schema を保持するため、module 境界を
          ;; 越えた raw/private name は現在の value env に公開されていなければ拒否する。
          (make-error-result-code (error-code-undefined))
          (do
            (root_push record-schema)
            (let [record-ty (instantiate record-schema counter)]
              (do
                (root_push record-ty)
                (let [result
                        (infer-recordlit-with-record-type
                          node
                          field-count
                          env
                          subst
                          counter
                          record-ty)]
                  (do
                    (root_pop)
                    (root_pop)
                    result)))))))]
    (if (= (result-failed fields-result) 1)
      (propagate-error-result fields-result)
      (if (= record-schema 0)
        (if (= (ty-tag (result-type fields-result)) (ty-record))
          fields-result
          (make-result (result-subst fields-result) (mk-con type-name-hash)))
        fields-result))))

;; field access の型推論
;; [13, expr, field-name-hash]
;; declared record 型なら具体化済み field 型を返し、未宣言 literal だけ既存 fallback を保つ
(defn infer-fieldaccess [node env subst counter]
  (let [field-name-hash (vector-get node 2)
    base-node (vector-get node 1)]
    (let [base-result (infer-expr base-node env subst counter)]
      (if (= (result-failed base-result) 1)
        (propagate-error-result base-result)
        (let [s1 (result-subst base-result)
          base-ty (apply-subst s1 (result-type base-result))]
          (if (= (ty-tag base-ty) (ty-record))
            (let [field-ty (type-record-field-type base-ty field-name-hash)]
              (if (= field-ty 0)
                (make-error-result-code (error-code-general))
                (make-result s1 (apply-subst s1 field-ty))))
            (if (= (vector-get base-node 0) (tag-recordlit))
              (let [field-node (recordlit-field-node base-node field-name-hash)]
                (if (= field-node 0)
                  (make-result s1 (fresh-type-var counter))
                  (infer-expr field-node env s1 counter)))
              (make-result s1 (fresh-type-var counter)))))))))

;; record update の型推論
;; [14, base-expr, field-count, field1-hash, expr1, ...]
(defn infer-declared-recordupdate-fields-state [done next-idx result]
  (vector-push-triple-rooted (vector-new 3) done next-idx result))

(defn infer-declared-recordupdate-fields-step-v3
  [node idx count env current-result counter record-ty]
  (if (>= idx count)
    (do
      (root_push current-result)
      (root_push record-ty)
      (let [final-subst (result-subst current-result)]
        (do
          (root_push final-subst)
          (let [final-type (apply-subst final-subst record-ty)]
            (do
              (root_push final-type)
              (let [final-result (make-result final-subst final-type)]
                (do
                  (root_push final-result)
                  (let [state
                          (infer-declared-recordupdate-fields-state
                            1 idx final-result)]
                    (do
                      (root_pop)
                      (root_pop)
                      (root_pop)
                      (root_pop)
                      (root_pop)
                      state)))))))))
    (do
      (root_push node)
      (root_push env)
      (root_push current-result)
      (root_push counter)
      (root_push record-ty)
      (let [field-offset (+ 3 (* idx 2))
            field-name-hash (vector-get node field-offset)
            value-node (vector-get node (+ field-offset 1))
            expected-ty (type-record-field-type record-ty field-name-hash)]
        (do
          (root_push expected-ty)
          (root_push value-node)
          (let [field-result
                  (if (= expected-ty 0)
                    (make-error-result-code (error-code-general))
                    (let [value-result
                              (infer-expr
                                value-node
                                env
                                (result-subst current-result)
                                counter)]
                        (do
                          (root_push value-result)
                          (let [next-result
                                  (if (= (result-failed value-result) 1)
                                    (propagate-error-result value-result)
                                    (let [next-subst
                                            (unify
                                              expected-ty
                                              (result-type value-result)
                                              (result-subst value-result))]
                                      (do
                                        (root_push next-subst)
                                        (let [unified-result
                                                (if (= (unify-failed next-subst) 1)
                                                  (make-error-result-code (error-code-general))
                                                  (make-result next-subst record-ty))]
                                          (do
                                            (root_pop)
                                            unified-result)))))]
                            (do
                              (root_push next-result)
                              (let [result next-result]
                                (do
                                  (root_pop)
                                  (root_pop)
                                                                    result)))))))]
            (do
              (root_push field-result)
              (let [state
                      (if (= (result-failed field-result) 1)
                        (infer-declared-recordupdate-fields-state 1 idx field-result)
                        (infer-declared-recordupdate-fields-state
                          0 (+ idx 1) field-result))]
                (do
                  (root_pop)
                  (root_pop)
                  (root_pop)
                  (root_pop)
                  (root_pop)
                  (root_pop)
                  (root_pop)
                  (root_pop)
                  state)))))))))

(defn infer-declared-recordupdate-fields-step-64-loop-bounded
  [node idx count env current-result counter record-ty remaining]
  (do
    (root_push node)
    (root_push env)
    (root_push current-result)
    (root_push counter)
    (root_push record-ty)
    (let [step
            (infer-declared-recordupdate-fields-step-v3
              node idx count env current-result counter record-ty)
          done (vector-get step 0)
          next-idx (vector-get step 1)
          next-result (vector-get step 2)]
      (do
        (root_push step)
        (root_push next-result)
        (let [parsed
                (if (= done 1)
                  step
                  (if (<= remaining 1)
                    step
                    (infer-declared-recordupdate-fields-step-64-loop-bounded
                      node
                      next-idx
                      count
                      env
                      next-result
                      counter
                      record-ty
                      (- remaining 1))))]
          (do
            (root_pop)
            (root_pop)
            (root_pop)
            (root_pop)
            (root_pop)
            (root_pop)
            (root_pop)
            parsed))))))

(defn infer-declared-recordupdate-fields-step-64
  [node idx count env current-result counter record-ty]
  (infer-declared-recordupdate-fields-step-64-loop-bounded
    node idx count env current-result counter record-ty 64))

(defn infer-declared-recordupdate-fields-rooted-v3
  [node idx count env current-result counter record-ty]
  (let [step
          (infer-declared-recordupdate-fields-step-64
            node idx count env current-result counter record-ty)]
    (if (= (vector-get step 0) 1)
      (vector-get step 2)
      (do
        (root_push step)
        (let [next-idx (vector-get step 1)
              next-result (vector-get step 2)]
          (do
            (root_push next-result)
            (let [resolved
                    (infer-declared-recordupdate-fields-rooted-v3
                      node
                      next-idx
                      count
                      env
                      next-result
                      counter
                      record-ty)]
              (do
                (root_pop)
                (root_pop)
                resolved))))))))

(defn infer-declared-recordupdate-fields
  [node idx count env subst counter record-ty]
  (do
    (root_push subst)
    (root_push record-ty)
    (let [initial (make-result subst record-ty)]
      (do
        (root_push initial)
        (let [result
                (infer-declared-recordupdate-fields-rooted-v3
                  node idx count env initial counter record-ty)]
          (do
            (root_pop)
            (root_pop)
            (root_pop)
            result))))))

(defn infer-recordupdate-node [node env subst counter]
  (let [base-result (infer-expr (vector-get node 1) env subst counter)]
    (if (= (result-failed base-result) 1)
      (propagate-error-result base-result)
      (let [s1 (result-subst base-result)
        base-ty (apply-subst s1 (result-type base-result))]
        (if (= (ty-tag base-ty) (ty-record))
          (infer-declared-recordupdate-fields
            node
            0
            (vector-get node 2)
            env
            s1
            counter
            base-ty)
          (make-error-result-code (error-code-general)))))))
