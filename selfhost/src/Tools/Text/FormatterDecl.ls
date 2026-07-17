(module Tools.Text.FormatterDecl)
(import Syntax.AST)
(import Tools.Text.FormatterExpr)

;; FormatterDecl.ls - 宣言フォーマット・プログラム整形
;;
;; Formatter.ls から分割 (STR-02)
;; 宣言 (defn, type, trait, module, impl) および
;; レコード・computation 式のフォーマットを担当する。
;;
;; バンドルモードでは FormatterExpr.ls → FormatterDecl.ls → Formatter.ls の順に
;; 連結され、Formatter.ls のディスパッチャが本モジュールの関数を呼び出す。

;; 宣言リストのフォーマット
(defn format-decl-list [node idx count indent-level]
  (if (<= count 0) ""
    (let [decl-text (format-decl (vector-get node idx) indent-level)]
      (if (= count 1)
        decl-text
        (str3 decl-text " " (format-decl-list node (+ idx 1) (- count 1) indent-level))))))

(defn format-decl-list-with-source [node idx count indent-level source]
  (if (<= count 0) ""
    (let [decl-text (format-decl-with-source (vector-get node idx) indent-level source)]
      (if (= count 1)
        decl-text
        (str3 decl-text " " (format-decl-list-with-source node (+ idx 1) (- count 1) indent-level source))))))

;; レコードリテラルのフォーマット

(defn format-recordlit-fields [node idx count indent-level]
  (if (<= count 0) ""
    (let [field-text (symbol-from-hash (vector-get node idx))
      expr-text (format-expr (vector-get node (+ idx 1)) indent-level)
      pair-text (str3 field-text " " expr-text)]
      (if (= count 1)
        pair-text
        (str3 pair-text " " (format-recordlit-fields node (+ idx 2) (- count 1) indent-level))))))

(defn format-recordlit-fields-with-source [node idx count indent-level source]
  (if (<= count 0) ""
    (let [field-text (symbol-from-hash (vector-get node idx))
      expr-text (format-expr-with-source (vector-get node (+ idx 1)) indent-level source)
      pair-text (str3 field-text " " expr-text)]
      (if (= count 1)
        pair-text
        (str3 pair-text " " (format-recordlit-fields-with-source node (+ idx 2) (- count 1) indent-level source))))))

(defn format-recordlit [node indent-level]
  (let [type-text (symbol-from-hash (vector-get node 1))
    field-count (vector-get node 2)]
    (if (= field-count 0)
      (if (> (string-length type-text) 0)
        (str3 "{" type-text "}")
        "{}")
      (let [fields-text (format-recordlit-fields node 3 field-count indent-level)]
        (if (> (string-length type-text) 0)
          (str5 "{" type-text " " fields-text "}")
          (str3 "{" fields-text "}"))))))

(defn format-recordlit-with-source [node indent-level source]
  (let [type-text (symbol-from-hash (vector-get node 1))
    field-count (vector-get node 2)]
    (if (= field-count 0)
      (if (> (string-length type-text) 0)
        (str3 "{" type-text "}")
        "{}")
      (let [fields-text (format-recordlit-fields-with-source node 3 field-count indent-level source)]
        (if (> (string-length type-text) 0)
          (str5 "{" type-text " " fields-text "}")
          (str3 "{" fields-text "}"))))))

;; フィールドアクセスのフォーマット

(defn format-fieldaccess [node indent-level]
  (let [base-text (format-expr (vector-get node 1) indent-level)
    field-text (symbol-from-hash (vector-get node 2))]
    (str5 "(. " base-text " " field-text ")")))

(defn format-fieldaccess-with-source [node indent-level source]
  (let [base-text (format-expr-with-source (vector-get node 1) indent-level source)
    field-text (symbol-from-hash (vector-get node 2))]
    (str5 "(. " base-text " " field-text ")")))

;; レコード更新のフォーマット

(defn format-recordupdate-fields [node idx count indent-level]
  (if (<= count 0) ""
    (let [field-text (symbol-from-hash (vector-get node idx))
      expr-text (format-expr (vector-get node (+ idx 1)) indent-level)
      pair-text (str3 field-text " " expr-text)]
      (if (= count 1)
        pair-text
        (str3 pair-text " " (format-recordupdate-fields node (+ idx 2) (- count 1) indent-level))))))

(defn format-recordupdate-fields-with-source [node idx count indent-level source]
  (if (<= count 0) ""
    (let [field-text (symbol-from-hash (vector-get node idx))
      expr-text (format-expr-with-source (vector-get node (+ idx 1)) indent-level source)
      pair-text (str3 field-text " " expr-text)]
      (if (= count 1)
        pair-text
        (str3 pair-text " " (format-recordupdate-fields-with-source node (+ idx 2) (- count 1) indent-level source))))))

(defn format-recordupdate [node indent-level]
  (let [base-text (format-expr (vector-get node 1) indent-level)
    field-count (vector-get node 2)]
    (if (= field-count 0)
      (str3 "{" (string-concat base-text " |}") "")
      (let [fields-text (format-recordupdate-fields node 3 field-count indent-level)]
        (str5 "{" base-text " | " fields-text "}")))))

(defn format-recordupdate-with-source [node indent-level source]
  (let [base-text (format-expr-with-source (vector-get node 1) indent-level source)
    field-count (vector-get node 2)]
    (if (= field-count 0)
      (str3 "{" (string-concat base-text " |}") "")
      (let [fields-text (format-recordupdate-fields-with-source node 3 field-count indent-level source)]
        (str5 "{" base-text " | " fields-text "}")))))

;; computation 式のフォーマット

(defn format-computation-step [step-kind aux expr indent-level]
  (let [expr-text (format-expr expr indent-level)]
    (if (= step-kind 0)
      expr-text
      (if (= step-kind 1)
        (str5 "(let! " (symbol-from-hash aux) " " expr-text ")")
        (if (= step-kind 2)
          (str3 "(do! " expr-text ")")
          (if (= step-kind 3)
            (str3 "(return " expr-text ")")
            expr-text))))))

(defn format-computation-step-with-source [step-kind aux expr indent-level source]
  (let [expr-text (format-expr-with-source expr indent-level source)]
    (if (= step-kind 0)
      expr-text
      (if (= step-kind 1)
        (str5 "(let! " (symbol-from-hash aux) " " expr-text ")")
        (if (= step-kind 2)
          (str3 "(do! " expr-text ")")
          (if (= step-kind 3)
            (str3 "(return " expr-text ")")
            expr-text))))))

(defn format-computation-steps [node idx count indent-level]
  (if (<= count 0) ""
    (let [step-text
      (format-computation-step
        (vector-get node idx)
        (vector-get node (+ idx 1))
        (vector-get node (+ idx 2))
        indent-level)]
      (if (= count 1)
        step-text
        (str3 step-text " " (format-computation-steps node (+ idx 3) (- count 1) indent-level))))))

(defn format-computation-steps-with-source [node idx count indent-level source]
  (if (<= count 0) ""
    (let [step-text
      (format-computation-step-with-source
        (vector-get node idx)
        (vector-get node (+ idx 1))
        (vector-get node (+ idx 2))
        indent-level
        source)]
      (if (= count 1)
        step-text
        (str3 step-text " " (format-computation-steps-with-source node (+ idx 3) (- count 1) indent-level source))))))

(defn format-computation [node indent-level]
  (let [builder-text (symbol-from-hash (vector-get node 1))
    step-count (vector-get node 2)]
    (if (= step-count 0)
      (if (> (string-length builder-text) 0)
        (str3 "(computation " builder-text ")")
        "(computation)")
      (let [steps-text (format-computation-steps node 3 step-count indent-level)]
        (if (> (string-length builder-text) 0)
          (str5 "(computation " builder-text " " steps-text ")")
          (str3 "(computation " steps-text ")"))))))

(defn format-computation-with-source [node indent-level source]
  (let [builder-text (symbol-from-hash (vector-get node 1))
    step-count (vector-get node 2)]
    (if (= step-count 0)
      (if (> (string-length builder-text) 0)
        (str3 "(computation " builder-text ")")
        "(computation)")
      (let [steps-text (format-computation-steps-with-source node 3 step-count indent-level source)]
        (if (> (string-length builder-text) 0)
          (str5 "(computation " builder-text " " steps-text ")")
          (str3 "(computation " steps-text ")"))))))

;; === 宣言フォーマット (ソース文字列付き) ===

(defn format-raw-string-literal [text]
  (str3 "\"" text "\""))

(defn format-defn-param-metadata-entry [entry]
  (let [name-text (symbol-from-hash (vector-get entry 0))
    doc-text (format-raw-string-literal (vector-get entry 1))]
    (str5 "(" name-text " " doc-text ")")))

(defn format-defn-param-metadata-list [params idx count]
  (if (>= idx count)
    ""
    (let [entry-text (format-defn-param-metadata-entry (vector-get params idx))]
      (if (= (+ idx 1) count)
        entry-text
        (str3 entry-text " " (format-defn-param-metadata-list params (+ idx 1) count))))))

(defn append-metadata-piece [acc piece]
  (if (> (string-length piece) 0)
    (if (> (string-length acc) 0)
      (str3 acc " " piece)
      piece)
    acc))

(defn formatter-defn-signature-node? [candidate]
  (if (= candidate 0)
    0
    (if (= (vector-get candidate 0) (ast-defn-signature)) 1 0)))

(defn extract-defn-metadata [decl]
  (let [body-end (+ 4 (vector-get decl 2))
    meta-idx
      (if (< body-end (vector-length decl))
        (if (= (formatter-defn-signature-node? (vector-get decl body-end)) 1)
          (+ body-end 1)
          body-end)
        (vector-length decl))]
    (if (< meta-idx (vector-length decl))
      (vector-get decl meta-idx)
      0)))

(defn format-defn-metadata-params [meta]
  (let [params (vector-get meta 2)
    count (vector-length params)]
    (if (= count 0)
      ""
      (string-concat ":params [" (string-concat (format-defn-param-metadata-list params 0 count) "]")))))

(defn format-defn-metadata-returns [meta]
  (let [returns-text (vector-get meta 3)]
    (if (> (string-length returns-text) 0)
      (string-concat ":returns " (format-raw-string-literal returns-text))
      "")))

(defn format-defn-metadata-doc [meta]
  (let [doc-text (vector-get meta 0)]
    (if (> (string-length doc-text) 0)
      (string-concat ":doc " (format-raw-string-literal doc-text))
      "")))

(defn format-defn-metadata-example [meta]
  (let [example-text (vector-get meta 1)]
    (if (> (string-length example-text) 0)
      (string-concat ":example [" (string-concat example-text "]"))
      "")))

(defn format-defn-metadata-invariant [meta]
  (let [predicate (vector-get meta 4)]
    (if (= predicate 0)
      ""
      (string-concat ":invariant " (format-expr predicate 0)))))

(defn format-defn-metadata-assert-form [form indent-level]
  (let [predicates (vector-get form 1)
    count (vector-length predicates)]
    (string-concat ":assert ["
      (string-concat (format-expr-list predicates 0 count indent-level) "]"))))

(defn format-defn-metadata-assert-loop [forms idx count indent-level]
  (if (>= idx count)
    ""
    (let [form (vector-get forms idx)]
      (if (= (vector-get form 0) (contract-form-assert))
        (format-defn-metadata-assert-form form indent-level)
        (format-defn-metadata-assert-loop forms (+ idx 1) count indent-level)))))

(defn format-defn-metadata-assert [meta indent-level]
  (if (> (vector-length meta) 5)
    (let [forms (vector-get meta 5)
      text (format-defn-metadata-assert-loop forms 0 (vector-length forms) indent-level)]
      text)
    ""))

(defn format-defn-metadata-case-expectation [expectation indent-level]
  (let [actual-text (format-expr (vector-get expectation 0) indent-level)
    expected-text (format-expr (vector-get expectation 1) indent-level)]
    (str5 "(expect " actual-text " " expected-text ")")))

(defn format-defn-metadata-case-expectations [expectations idx count indent-level]
  (if (<= count 0)
    ""
    (let [expectation-text (format-defn-metadata-case-expectation
        (vector-get expectations idx) indent-level)]
      (if (= count 1)
        expectation-text
        (str3 expectation-text " "
          (format-defn-metadata-case-expectations
            expectations (+ idx 1) (- count 1) indent-level))))))

(defn format-defn-metadata-case-form [form indent-level]
  (let [expectations (vector-get form 1)
    count (vector-length expectations)]
    (str3 ":case ["
      (format-defn-metadata-case-expectations expectations 0 count indent-level)
      "]")))

(defn format-defn-metadata-case-loop [forms idx count indent-level]
  (if (>= idx count)
    ""
    (let [form (vector-get forms idx)]
      (if (= (vector-get form 0) (contract-form-case))
        (format-defn-metadata-case-form form indent-level)
        (format-defn-metadata-case-loop forms (+ idx 1) count indent-level)))))

(defn format-defn-metadata-case [meta indent-level]
  (if (> (vector-length meta) 5)
    (let [forms (vector-get meta 5)]
      (format-defn-metadata-case-loop forms 0 (vector-length forms) indent-level))
    ""))

(defn format-defn-metadata [decl]
  (let [meta (extract-defn-metadata decl)]
    (if (= meta 0)
      ""
      (let [pieces-1 (append-metadata-piece "" (format-defn-metadata-params meta))
        pieces-2 (append-metadata-piece pieces-1 (format-defn-metadata-returns meta))
        pieces-3 (append-metadata-piece pieces-2 (format-defn-metadata-doc meta))
        pieces-4 (append-metadata-piece pieces-3 (format-defn-metadata-example meta))
        pieces-5 (append-metadata-piece pieces-4 (format-defn-metadata-invariant meta))
        pieces-6 (append-metadata-piece pieces-5 (format-defn-metadata-case meta 0))
        pieces-7 (append-metadata-piece pieces-6 (format-defn-metadata-assert meta 0))]
        (if (> (string-length pieces-7) 0)
          (string-concat " " pieces-7)
          "")))))

(defn format-defn-metadata-invariant-with-source [meta source]
  (let [predicate (vector-get meta 4)]
    (if (= predicate 0)
      ""
      (string-concat ":invariant " (format-expr-with-source predicate 0 source)))))

(defn format-defn-metadata-assert-form-with-source [form indent-level source]
  (let [predicates (vector-get form 1)
    count (vector-length predicates)]
    (string-concat ":assert ["
      (string-concat
        (format-expr-list-with-source predicates 0 count indent-level source)
        "]"))))

(defn format-defn-metadata-assert-loop-with-source [forms idx count indent-level source]
  (if (>= idx count)
    ""
    (let [form (vector-get forms idx)]
      (if (= (vector-get form 0) (contract-form-assert))
        (format-defn-metadata-assert-form-with-source form indent-level source)
        (format-defn-metadata-assert-loop-with-source
          forms (+ idx 1) count indent-level source)))))

(defn format-defn-metadata-assert-with-source [meta indent-level source]
  (if (> (vector-length meta) 5)
    (let [forms (vector-get meta 5)]
      (format-defn-metadata-assert-loop-with-source
        forms 0 (vector-length forms) indent-level source))
    ""))

(defn format-defn-metadata-case-expectation-with-source [expectation indent-level source]
  (let [actual-text (format-expr-with-source
      (vector-get expectation 0) indent-level source)
    expected-text (format-expr-with-source
      (vector-get expectation 1) indent-level source)]
    (str5 "(expect " actual-text " " expected-text ")")))

(defn format-defn-metadata-case-expectations-with-source
  [expectations idx count indent-level source]
  (if (<= count 0)
    ""
    (let [expectation-text (format-defn-metadata-case-expectation-with-source
        (vector-get expectations idx) indent-level source)]
      (if (= count 1)
        expectation-text
        (str3 expectation-text " "
          (format-defn-metadata-case-expectations-with-source
            expectations (+ idx 1) (- count 1) indent-level source))))))

(defn format-defn-metadata-case-form-with-source [form indent-level source]
  (let [expectations (vector-get form 1)
    count (vector-length expectations)]
    (str3 ":case ["
      (format-defn-metadata-case-expectations-with-source
        expectations 0 count indent-level source)
      "]")))

(defn format-defn-metadata-case-loop-with-source
  [forms idx count indent-level source]
  (if (>= idx count)
    ""
    (let [form (vector-get forms idx)]
      (if (= (vector-get form 0) (contract-form-case))
        (format-defn-metadata-case-form-with-source form indent-level source)
        (format-defn-metadata-case-loop-with-source
          forms (+ idx 1) count indent-level source)))))

(defn format-defn-metadata-case-with-source [meta indent-level source]
  (if (> (vector-length meta) 5)
    (let [forms (vector-get meta 5)]
      (format-defn-metadata-case-loop-with-source
        forms 0 (vector-length forms) indent-level source))
    ""))

(defn format-defn-metadata-with-source [decl source]
  (let [meta (extract-defn-metadata decl)]
    (if (= meta 0)
      ""
      (let [pieces-1 (append-metadata-piece "" (format-defn-metadata-params meta))
        pieces-2 (append-metadata-piece pieces-1 (format-defn-metadata-returns meta))
        pieces-3 (append-metadata-piece pieces-2 (format-defn-metadata-doc meta))
        pieces-4 (append-metadata-piece pieces-3 (format-defn-metadata-example meta))
        pieces-5 (append-metadata-piece pieces-4 (format-defn-metadata-invariant-with-source meta source))
        pieces-6 (append-metadata-piece pieces-5
          (format-defn-metadata-case-with-source meta 0 source))
        pieces-7 (append-metadata-piece pieces-6
          (format-defn-metadata-assert-with-source meta 0 source))]
        (if (> (string-length pieces-7) 0)
          (string-concat " " pieces-7)
          "")))))

(defn format-defn-with-source [decl indent-level source]
  (let [name-text (symbol-from-hash (vector-get decl 1))
    param-count (vector-get decl 2)
    params-text (format-hash-list decl 3 param-count)
    metadata-text (format-defn-metadata-with-source decl source)
    body-text (format-expr-with-source (vector-get decl (+ 3 param-count)) indent-level source)]
    (str3 (str7 "(defn " name-text " [" params-text "]" metadata-text " ") body-text ")")))

(defn format-type-decl [decl]
  (str3 "(type " (symbol-from-hash (vector-get decl 1)) ")"))

(defn format-record-def [decl]
  (str3 "(type " (symbol-from-hash (vector-get decl 1)) " (record))"))

(defn format-type-alias [decl]
  (let [name-text (symbol-from-hash (vector-get decl 1))]
    (str5 "(type-alias " name-text " " name-text ")")))

(defn format-type-constrained [decl]
  (let [name-text (symbol-from-hash (vector-get decl 1))]
    (str5 "(type-constrained " name-text " " name-text ")")))

(defn format-trait-decl-with-source [decl indent-level source]
  (let [name-text (symbol-from-hash (vector-get decl 1))
    body-count (vector-get decl 2)]
    (if (= body-count 0)
      (str3 "(trait (" name-text "))")
      (let [body-text (format-decl-list-with-source decl 3 body-count indent-level source)]
        (str5 "(trait (" name-text ") " body-text ")")))))

(defn format-defmacro-decl-with-source [decl indent-level source]
  (let [name-text (symbol-from-hash (vector-get decl 1))
    param-count (vector-get decl 2)
    params-text (format-hash-list decl 3 param-count)
    body-text (format-expr-with-source (vector-get decl (+ 3 param-count)) indent-level source)]
    (str7 "(defmacro " name-text " [" params-text "] " body-text ")")))

(defn format-module-decl-with-source [decl indent-level source]
  (let [name-text (symbol-from-hash (vector-get decl 1))
    body-count (vector-get decl 2)]
    (if (= body-count 0)
      (str3 "(module " name-text ")")
      (let [body-text (format-decl-list-with-source decl 3 body-count indent-level source)]
        (str5 "(module " name-text " " body-text ")")))))

(defn format-impl-decl-with-source [decl indent-level source]
  (let [trait-text (symbol-from-hash (vector-get decl 1))
    type-text (symbol-from-hash (vector-get decl 2))
    body-count (vector-get decl 3)]
    (if (= body-count 0)
      (str5 "(impl (" trait-text " " type-text "))")
      (let [body-text (format-decl-list-with-source decl 4 body-count indent-level source)]
        (str7 "(impl (" trait-text " " type-text ") " body-text ")")))))

(defn format-decl-with-source [decl indent-level source]
  (let [tag (vector-get decl 0)]
    (if (= tag 20) (format-defn-with-source decl indent-level source)
      (if (= tag 21) (format-type-decl decl)
        (if (= tag 22) (format-record-def decl)
          (if (= tag 23) (format-type-alias decl)
            (if (= tag 24) (format-type-constrained decl)
              (if (= tag 25) (format-module-decl-with-source decl indent-level source)
                (if (= tag 26) (format-import-decl decl)
                  (if (= tag 27) (format-trait-decl-with-source decl indent-level source)
                    (if (= tag 28) (format-impl-decl-with-source decl indent-level source)
                      (if (= tag 29) (str3 "(private " (format-decl-with-source (vector-get decl 1) indent-level source) ")")
                        (if (= tag 30) (format-computation-builder-decl decl)
                          (if (= tag 31) (format-defmacro-decl-with-source decl indent-level source)
                            (format-unsupported-decl tag)))))))))))))))

(defn format-program-item-with-source [item indent-level source]
  (let [tag (vector-get item 0)]
    (if (< tag 20)
      (format-expr-with-source item indent-level source)
      (format-decl-with-source item indent-level source))))

(defn format-program-items-with-source [program idx len source]
  (if (>= idx len) ""
    (let [item-text (format-program-item-with-source (vector-get program idx) 0 source)]
      (if (= (+ idx 1) len)
        item-text
        (str3 item-text "\n" (format-program-items-with-source program (+ idx 1) len source))))))

(defn format-program-with-source [program source]
  (let [len (vector-length program)]
    (if (= len 0)
      "\n"
      (string-concat (format-program-items-with-source program 0 len source) "\n"))))

;; === 宣言フォーマット (基本) ===

;; format-decl: supported decl を canonical な実テキストへ整形する
(defn format-defn [decl indent-level]
  (let [name-text (symbol-from-hash (vector-get decl 1))
    param-count (vector-get decl 2)
    params-text (format-hash-list decl 3 param-count)
    metadata-text (format-defn-metadata decl)
    body-text (format-expr (vector-get decl (+ 3 param-count)) indent-level)]
    (str3 (str7 "(defn " name-text " [" params-text "]" metadata-text " ") body-text ")")))

(defn format-trait-decl [decl indent-level]
  (let [name-text (symbol-from-hash (vector-get decl 1))
    body-count (vector-get decl 2)]
    (if (= body-count 0)
      (str3 "(trait (" name-text "))")
      (let [body-text (format-decl-list decl 3 body-count indent-level)]
        (str5 "(trait (" name-text ") " body-text ")")))))

(defn format-defmacro-decl [decl indent-level]
  (let [name-text (symbol-from-hash (vector-get decl 1))
    param-count (vector-get decl 2)
    params-text (format-hash-list decl 3 param-count)
    body-text (format-expr (vector-get decl (+ 3 param-count)) indent-level)]
    (str7 "(defmacro " name-text " [" params-text "] " body-text ")")))

(defn format-module-decl [decl indent-level]
  (let [name-text (symbol-from-hash (vector-get decl 1))
    body-count (vector-get decl 2)]
    (if (= body-count 0)
      (str3 "(module " name-text ")")
      (let [body-text (format-decl-list decl 3 body-count indent-level)]
        (str5 "(module " name-text " " body-text ")")))))

(defn format-import-decl [decl]
  (str3 "(import " (symbol-from-hash (vector-get decl 1)) ")"))

(defn format-impl-decl [decl indent-level]
  (let [trait-text (symbol-from-hash (vector-get decl 1))
    type-text (symbol-from-hash (vector-get decl 2))
    body-count (vector-get decl 3)]
    (if (= body-count 0)
      (str5 "(impl (" trait-text " " type-text "))")
      (let [body-text (format-decl-list decl 4 body-count indent-level)]
        (str7 "(impl (" trait-text " " type-text ") " body-text ")")))))

(defn format-computation-builder-decl [decl]
  (let [name-text (symbol-from-hash (vector-get decl 1))
    bind-text (symbol-from-hash (vector-get decl 2))
    return-text (symbol-from-hash (vector-get decl 3))]
    (str7 "(computation-builder " name-text " " bind-text " " return-text ")")))

(defn format-decl [decl indent-level]
  (let [tag (vector-get decl 0)]
    (if (= tag 20) (format-defn decl indent-level)
      (if (= tag 21) (format-type-decl decl)
        (if (= tag 22) (format-record-def decl)
          (if (= tag 23) (format-type-alias decl)
            (if (= tag 24) (format-type-constrained decl)
              (if (= tag 25) (format-module-decl decl indent-level)
                (if (= tag 26) (format-import-decl decl)
                  (if (= tag 27) (format-trait-decl decl indent-level)
                    (if (= tag 28) (format-impl-decl decl indent-level)
                      (if (= tag 29) (str3 "(private " (format-decl (vector-get decl 1) indent-level) ")")
                        (if (= tag 30) (format-computation-builder-decl decl)
                          (if (= tag 31) (format-defmacro-decl decl indent-level)
                            (format-unsupported-decl tag)))))))))))))))

(defn format-program-item [item indent-level]
  (let [tag (vector-get item 0)]
    (if (< tag 20)
      (format-expr item indent-level)
      (format-decl item indent-level))))

;; format-program-items: プログラム要素を改行区切りで連結
(defn format-program-items [program idx len]
  (if (>= idx len) ""
    (let [item-text (format-program-item (vector-get program idx) 0)]
      (if (= (+ idx 1) len)
        item-text
        (str3 item-text "\n" (format-program-items program (+ idx 1) len))))))

;; format-program: プログラム全体をフォーマットする
;; 入力: AST (Program: 宣言の vector) + オプション
;; 出力: canonical な実テキスト。CLI 連携のため末尾改行を付ける。
;; AC-300: 同一 AST → 同一出力 (roundtrip)
;; AC-301: format(format(src)) == format(src) (idempotency)
(defn format-program [program opts]
  (let [len (vector-length program)]
    (if (= len 0)
      "\n"
      (string-concat (format-program-items program 0 len) "\n"))))

;; 検証用 main
(defn main []
  (let [;; インデント生成テスト
    indent0 (make-indent 0)
    indent1 (make-indent 1)
    indent2 (make-indent 2)

    ;; フォーマットルールテスト
    oneline-short (format-sexp-oneline 2)
    oneline-long (format-sexp-oneline 5)

    ;; let 束縛のフォーマット
    let-single (format-let-bindings 1 0)
    let-multi (format-let-bindings 3 1)

    ;; defn のフォーマット
    defn-short (format-defn-layout 2 1)
    defn-long (format-defn-layout 3 5)

    ;; 統計テスト
    stats (format-stats-new)
    s1 (stats-add-line stats)
    s2 (stats-add-node s1)

    ;; 空プログラム整形の安定性
    empty-program (format-program (vector-new 0) 0)

    ;; P9-6d: LSP TextEdit テスト
    edit (make-text-edit 0 0 10 0 42)
    fmt-resp (make-formatting-response edit)]
    (do
      ;; インデント幅の検証
      (print (indent-width)) ;; 2
      (print (max-line-width)) ;; 80

      ;; インデント文字列の検証
      (print (string-length indent0)) ;; 0
      (print (string-length indent1)) ;; 2
      (print (string-length indent2)) ;; 4

      ;; 1 行フォーマット判定
      (print oneline-short) ;; 1 (1 行に収まる)
      (print oneline-long) ;; 0 (改行必要)

      ;; let 束縛フォーマット
      (print let-single) ;; 1
      (print let-multi) ;; 2

      ;; defn フォーマット
      (print defn-short) ;; 1
      (print defn-long) ;; 6

      ;; 統計
      (print (vector-get s2 0)) ;; 1 (行数)
      (print (vector-get s2 2)) ;; 1 (ノード数)

      ;; format-program: 空プログラムは末尾改行 1 文字、同一入力で連続一致
      (print (string-length empty-program)) ;; 1
      (print (if (string-eq empty-program (format-program (vector-new 0) 0)) 1 0)) ;; 1

      ;; === P9-6d: LSP TextEdit 検証 ===
      (print (vector-get edit 0)) ;; 0 (start-line)
      (print (vector-get edit 1)) ;; 0 (start-col)
      (print (vector-get edit 2)) ;; 10 (end-line)
      (print (vector-get edit 3)) ;; 0 (end-col)
      (print (vector-get edit 4)) ;; 42 (new-text hash)
      (print (vector-length fmt-resp)) ;; 1 (edit count)

      ;; === FMT-01: 拡張 format-expr テスト ===
      ;; let: [7, name-hash=x(120), init=[1,10], body=[4,x]]
      (let [let-init (vector-push (vector-push (vector-new 2) 1) 10)
        let-body (vector-push (vector-push (vector-new 2) 4) 120)
        let-node (vector-push (vector-push (vector-push
              (vector-push (vector-new 4) 7) 120) let-init) let-body)]
        (print (format-expr let-node 0))) ;; (let [x 10] x)

      ;; lambda: [8, param-count=2, x, y, body=[1,0]]
      (let [lam-body (vector-push (vector-push (vector-new 2) 1) 0)
        lam-node (vector-push (vector-push (vector-push
              (vector-push (vector-push (vector-new 5) 8) 2) 120) 121)
          lam-body)]
        (print (format-expr lam-node 0))) ;; (fn [x y] 0)

      ;; do: [9, expr-count=3, e1, e2, e3]
      (let [de1 (vector-push (vector-push (vector-new 2) 1) 1)
        de2 (vector-push (vector-push (vector-new 2) 1) 2)
        de3 (vector-push (vector-push (vector-new 2) 1) 3)
        do-node (vector-push (vector-push (vector-push
              (vector-push (vector-push (vector-new 5) 9) 3)
              de1) de2) de3)]
        (print (format-expr do-node 0))) ;; 3 (expr-count)

      ;; match: [10, scrutinee, arm-count=2, pat1, body1, pat2, body2]
      (let [scr (vector-push (vector-push (vector-new 2) 4) 120)
        mp1 (vector-push (vector-push (vector-new 2) 42) 1)
        mb1 (vector-push (vector-push (vector-new 2) 1) 10)
        mp2 (vector-push (vector-push (vector-new 2) 42) 2)
        mb2 (vector-push (vector-push (vector-new 2) 1) 20)
        match-node (vector-push (vector-push (vector-push
              (vector-push (vector-push (vector-push
                    (vector-push (vector-new 7) 10) scr) 2)
                mp1) mb1) mp2) mb2)]
        (print (format-expr match-node 0))) ;; (unsupported-expr 10)

      ;; format-decl: defn [20, nh=a(97), pc=3, x, y, z, body]
      (let [db (vector-push (vector-push (vector-new 2) 1) 0)
        dn (vector-push (vector-push (vector-push (vector-push
                (vector-push (vector-push (vector-push
                      (vector-new 7) 20) 97) 3) 120) 121) 122) db)]
        (print (format-decl dn 0))) ;; (defn a [x y z] 0)

      ;; format-program: 1 宣言のプログラム
      (let [db2 (vector-push (vector-push (vector-new 2) 1) 0)
        dn2 (vector-push (vector-push (vector-push (vector-push
                (vector-push (vector-new 5) 20) 97) 1) 120) db2)
        prog (vector-push (vector-new 1) dn2)
        r1 (format-program prog 0)
        r2 (format-program prog 0)]
        (do
          (print r1) ;; (defn a [x] 0)\n
          (print (if (string-eq r1 r2) 1 0))) ) ;; 1 (idempotency)

      ;; source-aware formatting: string / float literal
      (let [string-src "\"abc\""
        string-node (vector-push (vector-push (vector-push (vector-new 3) 3) 1) 4)
        string-prog (vector-push (vector-new 1) string-node)
        float-src "1.25"
        float-node (vector-push (vector-push (vector-push (vector-new 3) 19) 0) 4)
        float-prog (vector-push (vector-new 1) float-node)]
        (do
          (print (format-program-with-source string-prog string-src))
          (print (format-program-with-source float-prog float-src))))

      0)))
