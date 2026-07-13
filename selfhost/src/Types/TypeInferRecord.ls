(module Types.TypeInferRecord)
(import Syntax.AST)
(import Types.Type)
(import Types.TypeScheme)
(import Types.TypeInferCore)

;; TypeInferRecord.ls - レコード型の型推論
;;
;; infer-record-fields: record field 値群の順次推論
;; infer-recordlit-fields: record literal 用に field 型を保持して推論
;; recordlit-field-node-loop: record literal から特定 field の value node 取得 (ループ)
;; recordlit-field-node: record literal から特定 field の value node 取得
;; infer-recordlit: record literal の型推論
;; infer-fieldaccess: field access の型推論
;; infer-recordupdate-node: record update の型推論

;; record field value 群を順に推論する
;; node は [tag, ..., field-count, field1-hash, expr1, ...]
(defn infer-record-fields [node idx count env subst counter]
  (if (= idx count)
    (make-result subst (mk-int))
    (let [value-node (vector-get node (+ 4 (* idx 2)))
      value-result (infer-expr value-node env subst counter)]
      (if (= (result-failed value-result) 1)
        (propagate-error-result value-result)
        (infer-record-fields
          node
          (+ idx 1)
          count
          env
          (result-subst value-result)
          counter)))))

;; record literal 用に field 型を保持しながら順に推論する
(defn infer-recordlit-fields [node idx count env subst counter record-ty]
  (make-result subst record-ty))

;; record literal から特定 field の value node を取り出す
;; 見つからない場合は 0 を返す
(defn recordlit-field-node-loop [record-node field-name-hash idx field-count]
  (if (>= idx field-count)
    0
    (let [field-offset (+ 3 (* idx 2))
      current-field-hash (vector-get record-node field-offset)]
      (if (= current-field-hash field-name-hash)
        (vector-get record-node (+ field-offset 1))
        (recordlit-field-node-loop
          record-node
          field-name-hash
          (+ idx 1)
          field-count)))))

(defn recordlit-field-node [record-node field-name-hash]
  (recordlit-field-node-loop
    record-node
    field-name-hash
    0
    (vector-get record-node 2)))

;; declared record literal の各 field を具体化済み schema の field 型と単一化する。
(defn infer-declared-recordlit-fields [node idx count env subst counter record-ty]
  (do
    (root_push record-ty)
    (let [result
            (if (>= idx count)
              (make-result subst record-ty)
              (let [field-offset (+ 3 (* idx 2))
                field-name-hash (vector-get node field-offset)
                value-node (vector-get node (+ field-offset 1))
                expected-ty (type-record-field-type record-ty field-name-hash)]
                (if (= expected-ty 0)
                  (make-error-result-code (error-code-general))
                  (let [value-result (infer-expr value-node env subst counter)]
                    (if (= (result-failed value-result) 1)
                      (propagate-error-result value-result)
                      (let [next-subst
                              (unify expected-ty (result-type value-result) (result-subst value-result))]
                        (if (= (unify-failed next-subst) 1)
                          (make-error-result-code (error-code-general))
                          (infer-declared-recordlit-fields
                            node
                            (+ idx 1)
                            count
                            env
                            next-subst
                            counter
                            record-ty))))))))]
      (do
        (root_pop)
        result))))

;; record literal の型推論
;; [12, type-name-hash, field-count, field1-hash, expr1, ...]
(defn infer-recordlit [node env subst counter]
  (let [type-name-hash (vector-get node 1)
    field-count (vector-get node 2)
    record-schema (map-get-safe (var-counter-record-env counter) type-name-hash)
    fields-result
      (if (= record-schema 0)
        (infer-record-fields node 0 field-count env subst counter)
        (do
          (root_push record-schema)
          (let [record-ty (instantiate record-schema counter)]
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
                  (root_pop)
                  result))))))]
    (if (= (result-failed fields-result) 1)
      (propagate-error-result fields-result)
      (if (= record-schema 0)
        (make-result (result-subst fields-result) (mk-con type-name-hash))
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
(defn infer-declared-recordupdate-fields [node idx count env subst counter record-ty]
  (do
    (root_push record-ty)
    (let [result
            (if (>= idx count)
              (make-result subst (apply-subst subst record-ty))
              (let [field-offset (+ 3 (* idx 2))
                field-name-hash (vector-get node field-offset)
                value-node (vector-get node (+ field-offset 1))
                expected-ty (type-record-field-type record-ty field-name-hash)]
                (if (= expected-ty 0)
                  (make-error-result-code (error-code-general))
                  (let [value-result (infer-expr value-node env subst counter)]
                    (if (= (result-failed value-result) 1)
                      (propagate-error-result value-result)
                      (let [next-subst
                              (unify expected-ty (result-type value-result) (result-subst value-result))]
                        (if (= (unify-failed next-subst) 1)
                          (make-error-result-code (error-code-general))
                          (infer-declared-recordupdate-fields
                            node
                            (+ idx 1)
                            count
                            env
                            next-subst
                            counter
                            record-ty))))))))]
      (do
        (root_pop)
        result))))

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
