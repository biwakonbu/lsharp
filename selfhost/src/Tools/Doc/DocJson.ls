(module Tools.Doc.DocJson)
(import Tools.Doc.DocTools)
(import Tools.Lsp.JsonRpc)

;; DocJson.ls - DocTools payload を schema object JSON へ変換する

(defn docjson-object-wrap [body]
  (string-concat "{"
    (string-concat body "}")))

(defn docjson-array-wrap [body]
  (string-concat "["
    (string-concat body "]")))

(defn docjson-append [out piece]
  (if (= (string-length out) 0)
    piece
    (string-concat out
      (string-concat "," piece))))

(defn docjson-field [name value-json]
  (string-concat "\""
    (string-concat name
      (string-concat "\":" value-json))))

(defn docjson-string-literal [value]
  (string-concat "\""
    (string-concat (json-escape-string value) "\"")))

(defn docjson-string-field [name value]
  (docjson-field name (docjson-string-literal value)))

(defn docjson-int-field [name value]
  (docjson-field name (int-to-string value)))

(defn docjson-array-field [name value-json]
  (docjson-field name value-json))

(defn docjson-object-field [name value-json]
  (docjson-field name value-json))

(defn docjson-render-string-array-loop [items idx len out]
  (if (>= idx len)
    out
    (let [next-out
          (docjson-append out
            (docjson-string-literal (vector-get items idx)))]
      (docjson-render-string-array-loop items (+ idx 1) len next-out))))

(defn docjson-render-string-array [items]
  (docjson-array-wrap
    (docjson-render-string-array-loop items 0 (vector-length items) "")))

(defn docjson-module-text [ast module-id]
  (let [module-hash (find-module-hash ast)]
    (if (= module-hash 0)
      (title-from-module-id module-id)
      (title-from-hash module-hash))))

(defn docjson-source-text [source-id]
  (string-concat "source-" (int-to-string source-id)))

(defn docjson-render-knowledge-function [entry]
  (let [fields0 ""
    fields1 (docjson-append fields0
      (docjson-string-field "name" (vector-get entry 1)))
    fields2 (docjson-append fields1
      (docjson-int-field "arity" (vector-get entry 2)))
    fields3 (docjson-append fields2
      (docjson-string-field "doc" (vector-get entry 5)))
    fields4 (docjson-append fields3
      (docjson-string-field "example" (vector-get entry 6)))
    fields5 (docjson-append fields4
      (docjson-array-field "params"
        (docjson-render-string-array (vector-get entry 3))))
    fields6 (docjson-append fields5
      (docjson-string-field "returns" (vector-get entry 4)))]
    (docjson-object-wrap fields6)))

(defn docjson-render-knowledge-functions-loop [functions idx len out]
  (if (>= idx len)
    out
    (let [next-out
          (docjson-append out
            (docjson-render-knowledge-function (vector-get functions idx)))]
      (docjson-render-knowledge-functions-loop functions (+ idx 1) len next-out))))

(defn docjson-render-knowledge-functions [functions]
  (docjson-array-wrap
    (docjson-render-knowledge-functions-loop functions 0 (vector-length functions) "")))

(defn docjson-render-type-entry [entry]
  (let [fields0 ""
    fields1 (docjson-append fields0
      (docjson-string-field "name" (vector-get entry 1)))
    fields2 (docjson-append fields1
      (docjson-string-field "kind" (vector-get entry 2)))]
    (docjson-object-wrap fields2)))

(defn docjson-render-type-entries-loop [types idx len out]
  (if (>= idx len)
    out
    (let [next-out
          (docjson-append out
            (docjson-render-type-entry (vector-get types idx)))]
      (docjson-render-type-entries-loop types (+ idx 1) len next-out))))

(defn docjson-render-type-entries [types]
  (docjson-array-wrap
    (docjson-render-type-entries-loop types 0 (vector-length types) "")))

(defn docjson-render-review-diagnostic [diag]
  (let [fields0 ""
    fields1 (docjson-append fields0
      (docjson-string-field "title" (vector-get diag 1)))
    fields2 (docjson-append fields1
      (docjson-string-field "severity" (vector-get diag 3)))
    fields3 (docjson-append fields2
      (docjson-string-field "message" (vector-get diag 2)))
    fields4 (docjson-append fields3
      (docjson-int-field "line" (vector-get diag 4)))
    fields5 (docjson-append fields4
      (docjson-int-field "column" (vector-get diag 5)))
    fields6 (docjson-append fields5
      (docjson-string-field "code" (vector-get diag 6)))]
    (docjson-object-wrap fields6)))

(defn docjson-render-review-diagnostics-loop [diagnostics idx len out]
  (if (>= idx len)
    out
    (let [next-out
          (docjson-append out
            (docjson-render-review-diagnostic (vector-get diagnostics idx)))]
      (docjson-render-review-diagnostics-loop diagnostics (+ idx 1) len next-out))))

(defn docjson-render-review-diagnostics [diagnostics]
  (docjson-array-wrap
    (docjson-render-review-diagnostics-loop diagnostics 0 (vector-length diagnostics) "")))

(defn docjson-render-doc-param [param]
  (let [fields0 ""
    fields1 (docjson-append fields0
      (docjson-string-field "name" (vector-get param 0)))
    fields2 (docjson-append fields1
      (docjson-string-field "type" (vector-get param 1)))
    fields3 (docjson-append fields2
      (docjson-string-field "doc" (vector-get param 2)))]
    (docjson-object-wrap fields3)))

(defn docjson-render-doc-params-loop [params idx len out]
  (if (>= idx len)
    out
    (let [next-out
          (docjson-append out
            (docjson-render-doc-param (vector-get params idx)))]
      (docjson-render-doc-params-loop params (+ idx 1) len next-out))))

(defn docjson-render-doc-params [params]
  (docjson-array-wrap
    (docjson-render-doc-params-loop params 0 (vector-length params) "")))

(defn docjson-render-doc-returns [returns]
  (let [fields0 ""
    fields1 (docjson-append fields0
      (docjson-string-field "type" (vector-get returns 0)))
    fields2 (docjson-append fields1
      (docjson-string-field "doc" (vector-get returns 1)))]
    (docjson-object-wrap fields2)))

(defn docjson-render-doc-function [entry]
  (let [fields0 ""
    fields1 (docjson-append fields0
      (docjson-string-field "name" (vector-get entry 1)))
    fields2 (docjson-append fields1
      (docjson-int-field "arity" (vector-get entry 2)))
    fields3 (docjson-append fields2
      (docjson-array-field "params"
        (docjson-render-doc-params (vector-get entry 3))))
    fields4 (docjson-append fields3
      (docjson-object-field "returns"
        (docjson-render-doc-returns (vector-get entry 4))))
    fields5 (docjson-append fields4
      (docjson-string-field "doc" (vector-get entry 5)))
    fields6 (docjson-append fields5
      (docjson-string-field "example" (vector-get entry 6)))]
    (docjson-object-wrap fields6)))

(defn docjson-render-doc-functions-loop [functions idx len out]
  (if (>= idx len)
    out
    (let [next-out
          (docjson-append out
            (docjson-render-doc-function (vector-get functions idx)))]
      (docjson-render-doc-functions-loop functions (+ idx 1) len next-out))))

(defn docjson-render-doc-functions [functions]
  (docjson-array-wrap
    (docjson-render-doc-functions-loop functions 0 (vector-length functions) "")))

(defn docjson-render-doc-section [section]
  (let [fields0 ""
    fields1 (docjson-append fields0
      (docjson-string-field "id" (vector-get section 0)))
    fields2 (docjson-append fields1
      (docjson-int-field "count" (vector-get section 1)))]
    (docjson-object-wrap fields2)))

(defn docjson-render-doc-sections-loop [sections idx len out]
  (if (>= idx len)
    out
    (let [next-out
          (docjson-append out
            (docjson-render-doc-section (vector-get sections idx)))]
      (docjson-render-doc-sections-loop sections (+ idx 1) len next-out))))

(defn docjson-render-doc-sections [sections]
  (docjson-array-wrap
    (docjson-render-doc-sections-loop sections 0 (vector-length sections) "")))

(defn docjson-render-doc-html [title sections]
  (let [fields0 ""
    fields1 (docjson-append fields0
      (docjson-string-field "title" title))
    fields2 (docjson-append fields1
      (docjson-array-field "sections"
        (docjson-render-doc-sections sections)))]
    (docjson-object-wrap fields2)))

(defn generate-knowledge-schema-json [ast module-id]
  (let [knowledge (generate-knowledge ast module-id)
    functions (vector-get knowledge 1)
    types (vector-get knowledge 2)
    fields0 ""
    fields1 (docjson-append fields0
      (docjson-string-field "module" (docjson-module-text ast module-id)))
    fields2 (docjson-append fields1
      (docjson-array-field "functions"
        (docjson-render-knowledge-functions functions)))
    fields3 (docjson-append fields2
      (docjson-array-field "types"
        (docjson-render-type-entries types)))]
    (docjson-object-wrap fields3)))

(defn generate-review-schema-json [ast source-id]
  (let [review (generate-review ast source-id)
    diagnostics (vector-get review 1)
    fields0 ""
    fields1 (docjson-append fields0
      (docjson-string-field "source" (docjson-source-text source-id)))
    fields2 (docjson-append fields1
      (docjson-array-field "diagnostics"
        (docjson-render-review-diagnostics diagnostics)))]
    (docjson-object-wrap fields2)))

(defn generate-doc-output-schema-json [ast module-id]
  (let [doc (generate-doc-output ast module-id)
    title (vector-get doc 3)
    sections (vector-get doc 4)
    fields0 ""
    fields1 (docjson-append fields0
      (docjson-string-field "module" title))
    fields2 (docjson-append fields1
      (docjson-array-field "functions"
        (docjson-render-doc-functions (vector-get doc 1))))
    fields3 (docjson-append fields2
      (docjson-array-field "types"
        (docjson-render-type-entries (vector-get doc 2))))
    fields4 (docjson-append fields3
      (docjson-object-field "html"
        (docjson-render-doc-html title sections)))]
    (docjson-object-wrap fields4)))
