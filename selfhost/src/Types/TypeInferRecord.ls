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

;; 宣言済み record の [field-hash, resolved-type, ...] から field 型を取得する。
(defn record-decl-field-type-loop [record-fields field-name-hash idx len]
  (if (>= idx len)
    0
    (if (= (vector-get record-fields idx) field-name-hash)
      (vector-get record-fields (+ idx 1))
      (record-decl-field-type-loop
        record-fields
        field-name-hash
        (+ idx 2)
        len))))

(defn record-decl-field-type [record-fields field-name-hash]
  (record-decl-field-type-loop
    record-fields
    field-name-hash
    0
    (vector-length record-fields)))

;; declared record literal の各 field を宣言型と単一化する。
(defn infer-declared-recordlit-fields [node idx count env subst counter record-fields]
  (do
    (root_push record-fields)
    (let [result
            (if (>= idx count)
              (make-result subst (mk-int))
              (let [field-offset (+ 3 (* idx 2))
                field-name-hash (vector-get node field-offset)
                value-node (vector-get node (+ field-offset 1))
                expected-ty (record-decl-field-type record-fields field-name-hash)]
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
                            record-fields))))))))]
      (do
        (root_pop)
        result))))

;; record literal の型推論
;; [12, type-name-hash, field-count, field1-hash, expr1, ...]
(defn infer-recordlit [node env subst counter]
  (let [type-name-hash (vector-get node 1)
    field-count (vector-get node 2)
    record-fields (map-get-safe (var-counter-record-env counter) type-name-hash)
    fields-result
      (if (= record-fields 0)
        (infer-record-fields node 0 field-count env subst counter)
        (if (= field-count (/ (vector-length record-fields) 2))
          (infer-declared-recordlit-fields
            node
            0
            field-count
            env
            subst
            counter
            record-fields)
          (make-error-result-code (error-code-general))))]
    (if (= (result-failed fields-result) 1)
      (propagate-error-result fields-result)
      (make-result (result-subst fields-result) (mk-con type-name-hash)))))

;; field access の型推論
;; [13, expr, field-name-hash]
;; record 型なら対応フィールド型を返し、不明なら fresh var へ fallback する
(defn infer-fieldaccess [node env subst counter]
  (let [field-name-hash (vector-get node 2)
    base-node (vector-get node 1)]
    (if (= (vector-get base-node 0) (tag-recordlit))
      (let [field-node (recordlit-field-node base-node field-name-hash)]
        (if (= field-node 0)
          (make-result subst (fresh-type-var counter))
          (infer-expr field-node env subst counter)))
      (let [base-result (infer-expr base-node env subst counter)]
        (if (= (result-failed base-result) 1)
          (propagate-error-result base-result)
          (let [s1 (result-subst base-result)
            base-ty (apply-subst s1 (result-type base-result))]
            (if (= (ty-tag base-ty) (ty-record))
              (make-result s1 (mk-int))
              (make-result s1 (fresh-type-var counter)))))))))

;; record update の型推論
;; [14, base-expr, field-count, field1-hash, expr1, ...]
(defn infer-recordupdate-node [node env subst counter]
  (let [base-result (infer-expr (vector-get node 1) env subst counter)]
    (if (= (result-failed base-result) 1)
      (propagate-error-result base-result)
      (make-result (result-subst base-result) (result-type base-result)))))
