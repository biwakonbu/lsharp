use super::support::*;

/// CP-04: `docs/schemas/knowledge.schema.json` が richer function metadata を定義していること
#[test]
fn test_e2e_knowledge_schema_json_contract() {
    let schema_path = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../docs/schemas/knowledge.schema.json");
    let raw = std::fs::read_to_string(&schema_path)
        .expect("docs/schemas/knowledge.schema.json を読めない");
    let v: serde_json::Value =
        serde_json::from_str(&raw).expect("knowledge.schema.json の JSON 解析");

    let top_required = v["required"].as_array().expect("トップレベル required");
    let top: Vec<&str> = top_required.iter().filter_map(|x| x.as_str()).collect();
    assert!(top.contains(&"module"));
    assert!(top.contains(&"functions"));
    assert!(top.contains(&"types"));

    let function_items = v["properties"]["functions"]["items"]
        .as_object()
        .expect("functions.items が object であること");
    let function_required = function_items["required"]
        .as_array()
        .expect("functions.items.required が配列であること");
    let function_req: Vec<&str> = function_required
        .iter()
        .filter_map(|x| x.as_str())
        .collect();
    assert!(
        function_req.contains(&"name"),
        "knowledge schema: functions entry は name 必須"
    );
    assert!(
        function_req.contains(&"arity"),
        "knowledge schema: functions entry は arity 必須"
    );
    assert!(
        function_req.contains(&"params"),
        "knowledge schema: functions entry は params 必須"
    );
    assert!(
        function_req.contains(&"returns"),
        "knowledge schema: functions entry は returns 必須"
    );

    assert_eq!(
        function_items["properties"]["params"]["items"]["type"].as_str(),
        Some("string"),
        "knowledge schema: params items は string であるべき"
    );
    assert_eq!(
        function_items["properties"]["returns"]["type"].as_str(),
        Some("string"),
        "knowledge schema: returns は string であるべき"
    );
    assert_eq!(
        function_items["properties"]["doc"]["type"].as_str(),
        Some("string"),
        "knowledge schema: doc は string であるべき"
    );
    assert_eq!(
        function_items["properties"]["example"]["type"].as_str(),
        Some("string"),
        "knowledge schema: example は string であるべき"
    );
}

/// CP-04: `docs/schemas/review.schema.json` が diagnostics の必須フィールドを定義していること
#[test]
fn test_e2e_review_schema_json_contract() {
    let schema_path = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../docs/schemas/review.schema.json");
    let raw =
        std::fs::read_to_string(&schema_path).expect("docs/schemas/review.schema.json を読めない");
    let v: serde_json::Value = serde_json::from_str(&raw).expect("review.schema.json の JSON 解析");

    let diag_items = v["properties"]["diagnostics"]["items"]
        .as_object()
        .expect("diagnostics.items が object であること");
    let required = diag_items["required"]
        .as_array()
        .expect("diagnostics.items.required が配列であること");
    let req: Vec<&str> = required.iter().filter_map(|x| x.as_str()).collect();
    assert!(
        req.contains(&"severity"),
        "review schema: diagnostics は severity 必須"
    );
    assert!(
        req.contains(&"title"),
        "review schema: diagnostics は title 必須"
    );
    assert!(
        req.contains(&"message"),
        "review schema: diagnostics は message 必須"
    );
    assert!(
        req.contains(&"line"),
        "review schema: diagnostics は line 必須"
    );
    assert!(
        req.contains(&"column"),
        "review schema: diagnostics は column 必須"
    );
    assert!(
        req.contains(&"code"),
        "review schema: diagnostics は code 必須"
    );

    let top_required = v["required"].as_array().expect("トップレベル required");
    let top: Vec<&str> = top_required.iter().filter_map(|x| x.as_str()).collect();
    assert!(top.contains(&"source"));
    assert!(top.contains(&"diagnostics"));
}

/// CP-04: `docs/schemas/doc-output.schema.json` が entry list と HTML section metadata を定義していること
#[test]
fn test_e2e_doc_output_schema_json_contract() {
    let schema_path = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../docs/schemas/doc-output.schema.json");
    let raw = std::fs::read_to_string(&schema_path)
        .expect("docs/schemas/doc-output.schema.json を読めない");
    let v: serde_json::Value =
        serde_json::from_str(&raw).expect("doc-output.schema.json の JSON 解析");

    let top_required = v["required"].as_array().expect("トップレベル required");
    let top: Vec<&str> = top_required.iter().filter_map(|x| x.as_str()).collect();
    assert!(top.contains(&"module"));
    assert!(top.contains(&"functions"));
    assert!(top.contains(&"types"));
    assert!(top.contains(&"html"));

    let function_items = v["properties"]["functions"]["items"]
        .as_object()
        .expect("functions.items が object であること");
    let function_required = function_items["required"]
        .as_array()
        .expect("functions.items.required が配列であること");
    let function_req: Vec<&str> = function_required
        .iter()
        .filter_map(|x| x.as_str())
        .collect();
    assert!(
        function_req.contains(&"name"),
        "doc-output schema: functions entry は name 必須"
    );
    assert!(
        function_req.contains(&"arity"),
        "doc-output schema: functions entry は arity 必須"
    );
    assert!(
        function_req.contains(&"params"),
        "doc-output schema: functions entry は params 必須"
    );
    assert!(
        function_req.contains(&"returns"),
        "doc-output schema: functions entry は returns 必須"
    );
    let param_items = function_items["properties"]["params"]["items"]
        .as_object()
        .expect("doc-output schema: params.items が object であること");
    let param_required = param_items["required"]
        .as_array()
        .expect("doc-output schema: params.items.required が配列であること");
    let param_req: Vec<&str> = param_required.iter().filter_map(|x| x.as_str()).collect();
    assert_eq!(
        function_items["properties"]["params"]["type"].as_str(),
        Some("array"),
        "doc-output schema: functions entry は params:array を持つべき"
    );
    assert!(
        param_req.contains(&"name"),
        "doc-output schema: params item は name 必須"
    );
    assert!(
        param_req.contains(&"type"),
        "doc-output schema: params item は type 必須"
    );
    assert_eq!(
        param_items["properties"]["name"]["type"].as_str(),
        Some("string"),
        "doc-output schema: params item は name:string を持つべき"
    );
    assert_eq!(
        param_items["properties"]["type"]["type"].as_str(),
        Some("string"),
        "doc-output schema: params item は type:string を持つべき"
    );
    assert_eq!(
        param_items["properties"]["doc"]["type"].as_str(),
        Some("string"),
        "doc-output schema: params item は optional doc:string を持つべき"
    );
    let returns_item = function_items["properties"]["returns"]
        .as_object()
        .expect("doc-output schema: returns が object であること");
    let returns_required = returns_item["required"]
        .as_array()
        .expect("doc-output schema: returns.required が配列であること");
    let returns_req: Vec<&str> = returns_required.iter().filter_map(|x| x.as_str()).collect();
    assert!(
        returns_req.contains(&"type"),
        "doc-output schema: returns は type 必須"
    );
    assert_eq!(
        returns_item["type"].as_str(),
        Some("object"),
        "doc-output schema: functions entry は returns:object を持つべき"
    );
    assert_eq!(
        returns_item["properties"]["type"]["type"].as_str(),
        Some("string"),
        "doc-output schema: returns.type は string であるべき"
    );
    assert_eq!(
        returns_item["properties"]["doc"]["type"].as_str(),
        Some("string"),
        "doc-output schema: returns.doc は optional string であるべき"
    );
    assert_eq!(
        function_items["properties"]["doc"]["type"].as_str(),
        Some("string"),
        "doc-output schema: functions entry は optional doc:string を持つべき"
    );
    assert_eq!(
        function_items["properties"]["example"]["type"].as_str(),
        Some("string"),
        "doc-output schema: functions entry は optional example:string を持つべき"
    );

    let type_items = v["properties"]["types"]["items"]
        .as_object()
        .expect("types.items が object であること");
    let type_required = type_items["required"]
        .as_array()
        .expect("types.items.required が配列であること");
    let type_req: Vec<&str> = type_required.iter().filter_map(|x| x.as_str()).collect();
    assert!(
        type_req.contains(&"name"),
        "doc-output schema: type entry は name 必須"
    );
    assert!(
        type_req.contains(&"kind"),
        "doc-output schema: type entry は kind 必須"
    );

    let section_items = v["properties"]["html"]["properties"]["sections"]["items"]
        .as_object()
        .expect("html.sections.items が object であること");
    let section_required = section_items["required"]
        .as_array()
        .expect("html.sections.items.required が配列であること");
    let section_req: Vec<&str> = section_required.iter().filter_map(|x| x.as_str()).collect();
    assert!(
        section_req.contains(&"id"),
        "doc-output schema: html.sections entry は id 必須"
    );
    assert!(
        section_req.contains(&"count"),
        "doc-output schema: html.sections entry は count 必須"
    );
}

fn selfhost_doctools_runtime_bundle() -> String {
    [
        selfhost_module("Token.ls"),
        selfhost_module("AST.ls"),
        selfhost_module("Lexer.ls"),
        selfhost_module("Parser.ls"),
        selfhost_module("Type.ls"),
        selfhost_module("TypeScheme.ls"),
        selfhost_module("TypeInferCore.ls"),
        selfhost_module("TypeInferFunctions.ls"),
        selfhost_module("TypeInferBuiltins.ls"),
        selfhost_module("TypeInfer.ls"),
        selfhost_module("TypeInferApply.ls"),
        selfhost_module("TypeInferBlock.ls"),
        selfhost_module("TypeInferPattern.ls"),
        selfhost_module("TypeInferRecord.ls"),
        selfhost_module("TypeInferAdt.ls"),
        selfhost_module("DocTools.ls"),
        selfhost_module("JsonRpc.ls"),
        selfhost_module("DocJson.ls"),
    ]
    .join("\n")
}

fn selfhost_cli_html_runtime_bundle() -> String {
    [
        &selfhost_doctools_runtime_bundle(),
        selfhost_module("HtmlTemplate.ls"),
        selfhost_module("HtmlLayout.ls"),
        selfhost_module("HtmlDoc.ls"),
    ]
    .join("\n")
}

/// D-3: DocTools.ls の generate-html が title/body と entry list を返すこと
#[test]
#[ignore]
fn test_e2e_selfhost_doctools_generate_html_basic() {
    let harness = r#"
(defn main []
  (let [program (parse-program "(defn add [x y] (+ x y)) (type Num Int)")
        html (generate-html program 0)
        functions (vector-get html 3)
        types (vector-get html 4)
        fn0 (vector-get functions 0)
        type0 (vector-get types 0)]
    (do
      (print (vector-length html))
      (print (vector-get html 0))
      (print (if (string-eq (vector-get html 1) "module-global") 1 0))
      (print (if (> (string-length (vector-get html 2)) 0) 1 0))
      (print (vector-length functions))
      (print-string (vector-get fn0 1))
      (print-string "\n")
      (print (vector-get fn0 2))
      (print (vector-length types))
      (print-string (vector-get type0 1))
      (print-string "\n")
      (print (if (string-eq (vector-get type0 2) "type") 1 0))
      0)))
"#;

    let combined = format!("{}\n{}", selfhost_doctools_runtime_bundle(), harness);
    let output = compile_and_run(&combined);
    let lines: Vec<&str> = output.trim().lines().collect();

    assert_eq!(
        lines,
        vec!["5", "1", "1", "1", "1", "add", "2", "1", "Num", "1"]
    );
}

/// D-3: DocTools.ls の generate-html が 2 回実行しても同一 payload を返すこと
#[test]
#[ignore]
fn test_e2e_selfhost_doctools_generate_html_idempotent() {
    let harness = r#"
(defn main []
  (let [program (parse-program "(defn add [x y] (+ x y)) (type Num Int)")
        html1 (generate-html program 0)
        html2 (generate-html program 0)
        functions1 (vector-get html1 3)
        functions2 (vector-get html2 3)
        types1 (vector-get html1 4)
        types2 (vector-get html2 4)
        fn1 (vector-get functions1 0)
        fn2 (vector-get functions2 0)
        type1 (vector-get types1 0)
        type2 (vector-get types2 0)]
    (do
      (print (if (string-eq (vector-get html1 1) (vector-get html2 1)) 1 0))
      (print (if (string-eq (vector-get html1 2) (vector-get html2 2)) 1 0))
      (print (if (= (vector-length functions1) (vector-length functions2)) 1 0))
      (print (if (= (vector-get fn1 0) (vector-get fn2 0)) 1 0))
      (print (if (string-eq (vector-get fn1 1) (vector-get fn2 1)) 1 0))
      (print (if (= (vector-get fn1 2) (vector-get fn2 2)) 1 0))
      (print (if (string-eq (vector-get type1 1) (vector-get type2 1)) 1 0))
      (print (if (string-eq (vector-get type1 2) (vector-get type2 2)) 1 0))
      0)))
"#;

    let combined = format!("{}\n{}", selfhost_doctools_runtime_bundle(), harness);
    let output = compile_and_run(&combined);
    let lines: Vec<&str> = output.trim().lines().collect();

    assert_eq!(lines, vec!["1", "1", "1", "1", "1", "1", "1", "1"]);
}

/// DOC-01: generate が title/body/function/type payload を返すこと
#[test]
#[ignore]
fn test_e2e_selfhost_doctools_generate_structured_doc_payload() {
    let harness = r#"
(defn main []
  (let [program (parse-program "(defn main [] 42) (type Doc Int)")
        doc (generate program 0)
        functions (vector-get doc 2)
        types (vector-get doc 3)]
    (do
      (print (vector-length doc))
      (print (if (string-eq (vector-get doc 0) "module-global") 1 0))
      (print (if (string-eq (vector-get doc 1) "functions:1,types:1,first-fn:main,first-type:Doc") 1 0))
      (print (vector-length functions))
      (print (vector-length types))
      0)))
"#;

    let combined = format!("{}\n{}", selfhost_doctools_runtime_bundle(), harness);
    let output = compile_and_run(&combined);
    let lines: Vec<&str> = output.trim().lines().collect();

    assert_eq!(lines, vec!["4", "1", "1", "1", "1"]);
}

/// DOC-01: module decl がある場合は title に module 名を反映すること
#[test]
#[ignore]
fn test_e2e_selfhost_doctools_module_title_uses_name() {
    let harness = r#"
(defn main []
  (let [program (parse-program "(module Demo (defn main [] 42))")
        doc (generate program 0)]
    (do
      (print (if (string-eq (vector-get doc 0) "module-Demo") 1 0))
      0)))
"#;

    let combined = format!("{}\n{}", selfhost_doctools_runtime_bundle(), harness);
    let output = compile_and_run(&combined);
    let lines: Vec<&str> = output.trim().lines().collect();

    assert_eq!(lines, vec!["1"]);
}

/// DOC-01: generate-knowledge の出力が module + function entries + type entries を返すこと
#[test]
#[ignore]
fn test_e2e_selfhost_doctools_schema_knowledge() {
    let harness = r#"
(defn main []
  (let [program (parse-program "(defn add [x y] :doc \"Add two ints\" :example [(add 1 2)] (+ x y)) (type Doc Int) (type-alias Alias Int)")
        kb (generate-knowledge program 100)
        functions (vector-get kb 1)
        types (vector-get kb 2)
        fn0 (vector-get functions 0)
        params (if (> (vector-length fn0) 3) (vector-get fn0 3) (vector-new 0))
        returns (if (> (vector-length fn0) 4) (vector-get fn0 4) "missing")
        doc (if (> (vector-length fn0) 5) (vector-get fn0 5) "missing")
        example (if (> (vector-length fn0) 6) (vector-get fn0 6) "missing")
        type0 (vector-get types 0)
        type1 (vector-get types 1)]
    (do
      (print (vector-length kb))
      (print (vector-get kb 0))
      (print (vector-length functions))
      (print-string (vector-get fn0 1))
      (print-string "\n")
      (print (vector-get fn0 2))
      (print (vector-length fn0))
      (print (vector-length params))
      (print-string (if (> (vector-length params) 0) (vector-get params 0) ""))
      (print-string "\n")
      (print-string (if (> (vector-length params) 1) (vector-get params 1) ""))
      (print-string "\n")
      (print-string returns)
      (print-string "\n")
      (print-string doc)
      (print-string "\n")
      (print-string example)
      (print-string "\n")
      (print (vector-length types))
      (print (if (string-eq (vector-get type0 1) "Doc") 1 0))
      (print (if (string-eq (vector-get type0 2) "type") 1 0))
      (print (if (string-eq (vector-get type1 1) "Alias") 1 0))
      (print (if (string-eq (vector-get type1 2) "typealias") 1 0))
      0)))
"#;

    let combined = format!("{}\n{}", selfhost_doctools_runtime_bundle(), harness);
    let output = compile_and_run(&combined);
    let lines: Vec<&str> = output.trim().lines().collect();

    assert_eq!(
        lines,
        vec![
            "3",
            "100",
            "1",
            "add",
            "2",
            "7",
            "2",
            "x:Int",
            "y:Int",
            "Int",
            "Add two ints",
            "(add 1 2)",
            "2",
            "1",
            "1",
            "1",
            "1"
        ]
    );
}

/// DOC-01: DocTools entry list が source 順ではなく deterministic な hash 昇順で並ぶこと
#[test]
#[ignore]
fn test_e2e_selfhost_doctools_sorts_entries_by_hash() {
    let harness = r#"
(defn main []
  (let [program (parse-program "(defn zebra [] 0) (defn add [] 1) (type Zebra Int) (type Alias Int)")
        kb (generate-knowledge program 100)
        fns (vector-get kb 1)
        tys (vector-get kb 2)
        fn0 (vector-get fns 0)
        fn1 (vector-get fns 1)
        ty0 (vector-get tys 0)
        ty1 (vector-get tys 1)
        zebra-hash (name-hash "zebra" 0 5)
        add-hash (name-hash "add" 0 3)
        zebra-type-hash (name-hash "Zebra" 0 5)
        alias-hash (name-hash "Alias" 0 5)]
    (do
      (print (vector-length fns))
      (print (if (< add-hash zebra-hash)
               (= (vector-get fn0 0) add-hash)
               (= (vector-get fn0 0) zebra-hash)))
      (print (if (< add-hash zebra-hash)
               (= (vector-get fn1 0) zebra-hash)
               (= (vector-get fn1 0) add-hash)))
      (print (vector-length tys))
      (print (if (< alias-hash zebra-type-hash)
               (= (vector-get ty0 0) alias-hash)
               (= (vector-get ty0 0) zebra-type-hash)))
      (print (if (< alias-hash zebra-type-hash)
               (= (vector-get ty1 0) zebra-type-hash)
               (= (vector-get ty1 0) alias-hash)))
      0)))
"#;

    let combined = format!("{}\n{}", selfhost_doctools_runtime_bundle(), harness);
    let output = compile_and_run(&combined);
    let lines: Vec<&str> = output.trim().lines().collect();

    assert_eq!(lines, vec!["2", "1", "1", "2", "1", "1"]);
}

/// DOC-01: generate-review が unused-let を deterministic diagnostic として返すこと
#[test]
#[ignore]
fn test_e2e_selfhost_doctools_schema_review() {
    let harness = r#"
(defn main []
  (let [program (parse-program "(defn main [] (let [x 42] 0))")
        rev (generate-review program 200)
        diags (vector-get rev 1)
        diag0 (vector-get diags 0)]
    (do
      (print (vector-length rev))
      (print (vector-get rev 0))
      (print (vector-length diags))
      (print (vector-length diag0))
      (print (vector-get diag0 0))
      (print-string (vector-get diag0 1))
      (print-string "\n")
      (print-string (vector-get diag0 2))
      (print-string "\n")
      (print-string (vector-get diag0 3))
      (print-string "\n")
      (print (vector-get diag0 4))
      (print (vector-get diag0 5))
      (print-string (vector-get diag0 6))
      (print-string "\n")
      0)))
"#;

    let combined = format!("{}\n{}", selfhost_doctools_runtime_bundle(), harness);
    let output = compile_and_run(&combined);
    let lines: Vec<&str> = output.trim().lines().collect();

    assert_eq!(
        lines,
        vec![
            "2",
            "200",
            "1",
            "7",
            "100",
            "unused-let",
            "let binding x is not used",
            "warning",
            "1",
            "1",
            "L0001",
        ]
    );
}

/// DOC-01: generate-review が empty-do を deterministic diagnostic として返すこと
#[test]
#[ignore]
fn test_e2e_selfhost_doctools_schema_review_empty_do() {
    let harness = r#"
(defn main []
  (let [program (parse-program "(defn main [] (do))")
        rev (generate-review program 300)
        diags (vector-get rev 1)
        diag0 (vector-get diags 0)]
    (do
      (print (vector-get rev 0))
      (print (vector-length diags))
      (print (vector-length diag0))
      (print (vector-get diag0 0))
      (print-string (vector-get diag0 1))
      (print-string "\n")
      (print-string (vector-get diag0 2))
      (print-string "\n")
      (print-string (vector-get diag0 3))
      (print-string "\n")
      (print (vector-get diag0 4))
      (print (vector-get diag0 5))
      (print-string (vector-get diag0 6))
      (print-string "\n")
      0)))
"#;

    let combined = format!("{}\n{}", selfhost_doctools_runtime_bundle(), harness);
    let output = compile_and_run(&combined);
    let lines: Vec<&str> = output.trim().lines().collect();

    assert_eq!(
        lines,
        vec![
            "300",
            "1",
            "7",
            "104",
            "empty-do",
            "do block has no expressions",
            "warning",
            "1",
            "1",
            "L0002",
        ]
    );
}

/// DOC-01: generate-doc-ack が status/title/body と trailer を返すこと
#[test]
#[ignore]
fn test_e2e_selfhost_doctools_doc_ack_trailer_payload() {
    let harness = r#"
(defn main []
  (let [program (parse-program "(module Demo (defn main [] 42))")
        ack (generate-doc-ack program "alice")
        trailers (vector-get ack 3)]
    (do
      (print (vector-length ack))
      (print-string (vector-get ack 0))
      (print-string "\n")
      (print-string (vector-get ack 1))
      (print-string "\n")
      (print-string (vector-get ack 2))
      (print-string "\n")
      (print (vector-length trailers))
      (print-string (vector-get trailers 0))
      (print-string "\n")
      0)))
"#;

    let combined = format!("{}\n{}", selfhost_doctools_runtime_bundle(), harness);
    let output = compile_and_run(&combined);
    let lines: Vec<&str> = output.trim().lines().collect();

    assert_eq!(
        lines,
        vec![
            "4",
            "ack:recorded",
            "module-Demo",
            "functions:1,types:0,first-fn:main",
            "1",
            "; Doc-Reviewed-By: alice",
        ]
    );
}

/// DOC-01: generate-doc-check が status/title/body と trailer を返すこと
#[test]
#[ignore]
fn test_e2e_selfhost_doctools_doc_check_trailer_payload() {
    let harness = r#"
(defn main []
  (let [program (parse-program "(module Demo (defn main [] 42))")
        check (generate-doc-check program "alice")
        trailers (vector-get check 3)]
    (do
      (print (vector-length check))
      (print-string (vector-get check 0))
      (print-string "\n")
      (print-string (vector-get check 1))
      (print-string "\n")
      (print-string (vector-get check 2))
      (print-string "\n")
      (print (vector-length trailers))
      (print-string (vector-get trailers 0))
      (print-string "\n")
      (print-string (vector-get trailers 1))
      (print-string "\n")
      0)))
"#;

    let combined = format!("{}\n{}", selfhost_doctools_runtime_bundle(), harness);
    let output = compile_and_run(&combined);
    let lines: Vec<&str> = output.trim().lines().collect();

    assert_eq!(
        lines,
        vec![
            "4",
            "status:ok",
            "module-Demo",
            "functions:1,types:0,first-fn:main",
            "2",
            "; Doc-Review-Status: Passed",
            "; Doc-Reviewed-By: alice",
        ]
    );
}

/// DOC-02: doc-check trailer validation は末尾 comment trailer を受理すること
#[test]
#[ignore]
fn test_e2e_selfhost_doctools_doc_check_trailer_validation_accepts_comment_form() {
    let harness = r#"
(defn main []
  (let [source "(defn main [] 42)\n; Doc-Review-Status: Passed\n; Doc-Reviewed-By: alice\n"]
    (do
      (print (doc-check-trailer-valid? source))
      0)))
"#;

    let combined = format!("{}\n{}", selfhost_doctools_runtime_bundle(), harness);
    let output = compile_and_run(&combined);
    let lines: Vec<&str> = output.trim().lines().collect();

    assert_eq!(lines, vec!["1"]);
}

/// DOC-02: doc-check trailer validation は status trailer 欠落を拒否すること
#[test]
#[ignore]
fn test_e2e_selfhost_doctools_doc_check_trailer_validation_rejects_missing_status() {
    let harness = r#"
(defn main []
  (let [source "(defn main [] 42)\n; Doc-Reviewed-By: alice\n"]
    (do
      (print (doc-check-trailer-valid? source))
      0)))
"#;

    let combined = format!("{}\n{}", selfhost_doctools_runtime_bundle(), harness);
    let output = compile_and_run(&combined);
    let lines: Vec<&str> = output.trim().lines().collect();

    assert_eq!(lines, vec!["0"]);
}

/// DOC-02: doc-check trailer validation は reviewer 値欠落を拒否すること
#[test]
#[ignore]
fn test_e2e_selfhost_doctools_doc_check_trailer_validation_rejects_empty_reviewer() {
    let harness = r#"
(defn main []
  (let [source "(defn main [] 42)\n; Doc-Review-Status: Passed\n; Doc-Reviewed-By: \n"]
    (do
      (print (doc-check-trailer-valid? source))
      0)))
"#;

    let combined = format!("{}\n{}", selfhost_doctools_runtime_bundle(), harness);
    let output = compile_and_run(&combined);
    let lines: Vec<&str> = output.trim().lines().collect();

    assert_eq!(lines, vec!["0"]);
}

/// DOC-01: generate-doc-output の出力が function/type entries と title を含むこと
#[test]
#[ignore]
fn test_e2e_selfhost_doctools_schema_doc_output() {
    let harness = r#"
(defn main []
  (let [program (parse-program "(defn main [] 42) (type Doc Int)")
        doc-out (generate-doc-output program 300)
        sections (vector-get doc-out 4)
        section0 (vector-get sections 0)
        section1 (vector-get sections 1)]
    (do
      (print (vector-length doc-out))
      (print (vector-get doc-out 0))
      (print (vector-length (vector-get doc-out 1)))
      (print (vector-length (vector-get doc-out 2)))
      (print (if (string-eq (vector-get doc-out 3) "module-300") 1 0))
      (print (vector-length sections))
      (print (if (string-eq (vector-get section0 0) "functions") 1 0))
      (print (vector-get section0 1))
      (print (if (string-eq (vector-get section1 0) "types") 1 0))
      (print (vector-get section1 1))
      0)))
"#;

    let combined = format!("{}\n{}", selfhost_doctools_runtime_bundle(), harness);
    let output = compile_and_run(&combined);
    let lines: Vec<&str> = output.trim().lines().collect();

    assert_eq!(
        lines,
        vec!["5", "300", "1", "1", "1", "2", "1", "1", "1", "1"]
    );
}

/// DOC-01: generate-doc-output も module decl がある場合は module 名 title を使うこと
#[test]
#[ignore]
fn test_e2e_selfhost_doctools_schema_doc_output_module_title_name() {
    let harness = r#"
(defn main []
  (let [program (parse-program "(module Demo (defn main [] 42))")
        doc-out (generate-doc-output program 300)
        sections (vector-get doc-out 4)
        section0 (vector-get sections 0)]
    (do
      (print (if (string-eq (vector-get doc-out 3) "module-Demo") 1 0))
      (print (vector-length sections))
      (print (if (string-eq (vector-get section0 0) "functions") 1 0))
      (print (vector-get section0 1))
      0)))
"#;

    let combined = format!("{}\n{}", selfhost_doctools_runtime_bundle(), harness);
    let output = compile_and_run(&combined);
    let lines: Vec<&str> = output.trim().lines().collect();

    assert_eq!(lines, vec!["1", "1", "1", "1"]);
}

/// DOC-01: generate-doc-output の function entry が doc/example metadata を含むこと
#[test]
#[ignore]
fn test_e2e_selfhost_doctools_schema_doc_output_function_metadata() {
    let harness = r#"
(defn main []
  (let [program (parse-program "(defn add [x y] :params [(x \"left\") (y \"right\")] :returns \"sum\" :doc \"Add two ints\" :example [(add 1 2)] (+ x y))")
        doc-out (generate-doc-output program 300)
        functions (vector-get doc-out 1)
        fn0 (vector-get functions 0)
        params (if (> (vector-length fn0) 3) (vector-get fn0 3) (vector-new 0))
        param0 (if (> (vector-length params) 0) (vector-get params 0) (vector-new 0))
        param1 (if (> (vector-length params) 1) (vector-get params 1) (vector-new 0))
        returns (if (> (vector-length fn0) 4) (vector-get fn0 4) (vector-new 0))
        doc (if (> (vector-length fn0) 5) (vector-get fn0 5) "missing")
        example (if (> (vector-length fn0) 6) (vector-get fn0 6) "missing")]
    (do
      (print (vector-length fn0))
      (print (vector-length params))
      (print-string (if (> (vector-length param0) 0) (vector-get param0 0) ""))
      (print-string "\n")
      (print-string (if (> (vector-length param0) 1) (vector-get param0 1) ""))
      (print-string "\n")
      (print-string (if (> (vector-length param0) 2) (vector-get param0 2) ""))
      (print-string "\n")
      (print-string (if (> (vector-length param1) 0) (vector-get param1 0) ""))
      (print-string "\n")
      (print-string (if (> (vector-length param1) 1) (vector-get param1 1) ""))
      (print-string "\n")
      (print-string (if (> (vector-length param1) 2) (vector-get param1 2) ""))
      (print-string "\n")
      (print-string (if (> (vector-length returns) 0) (vector-get returns 0) ""))
      (print-string "\n")
      (print-string (if (> (vector-length returns) 1) (vector-get returns 1) ""))
      (print-string "\n")
      (print-string doc)
      (print-string "\n")
      (print-string example)
      (print-string "\n")
      0)))
"#;

    let combined = format!("{}\n{}", selfhost_doctools_runtime_bundle(), harness);
    let output = compile_and_run(&combined);
    let lines: Vec<&str> = output.trim().lines().collect();

    assert_eq!(
        lines,
        vec![
            "7",
            "2",
            "x",
            "Int",
            "left",
            "y",
            "Int",
            "right",
            "Int",
            "sum",
            "Add two ints",
            "(add 1 2)"
        ]
    );
}

/// DOC-01: ドキュメント文字列がタイムスタンプ・ホスト名・絶対パスを含まないこと
#[test]
#[ignore]
fn test_e2e_selfhost_doctools_no_timestamp() {
    let harness = r#"
(defn string-contains-loop [haystack needle i hlen nlen]
  (if (> (+ i nlen) hlen)
    0
    (if (string-eq (substring haystack i (+ i nlen)) needle)
      1
      (string-contains-loop haystack needle (+ i 1) hlen nlen))))

(defn string-contains [haystack needle]
  (let [hlen (string-length haystack)
        nlen (string-length needle)]
    (if (= nlen 0)
      1
      (if (> nlen hlen)
        0
        (string-contains-loop haystack needle 0 hlen nlen)))))

(defn main []
  (let [program (parse-program "(defn main [] 42) (type Doc Int)")
        doc (generate program 0)
        html (generate-html program 0)
        doc-out (generate-doc-output program 0)]
    (do
      (print (if (string-eq (vector-get doc 0) "module-global") 1 0))
      (print (if (string-eq (vector-get doc-out 3) "module-0") 1 0))
      (print (if (= (string-contains (vector-get doc 1) "/Users/") 0) 1 0))
      (print (if (= (string-contains (vector-get html 2) "localhost") 0) 1 0))
      (print (if (= (string-contains (vector-get html 2) "2026-") 0) 1 0))
      0)))
"#;

    let combined = format!("{}\n{}", selfhost_doctools_runtime_bundle(), harness);
    let output = compile_and_run(&combined);
    let lines: Vec<&str> = output.trim().lines().collect();

    assert_eq!(lines, vec!["1", "1", "1", "1", "1"]);
}

/// DOC-01: 同一入力に対し doc/html/schema 出力が deterministic であること
#[test]
#[ignore]
fn test_e2e_selfhost_doctools_deterministic() {
    let harness = r#"
(defn main []
  (let [program (parse-program "(defn add [x y] (+ x y)) (type Num Int)")
        html1 (generate-html program 0)
        html2 (generate-html program 0)
        kb1 (generate-knowledge program 50)
        kb2 (generate-knowledge program 50)
        doc1 (generate-doc-output program 50)
        doc2 (generate-doc-output program 50)
        rev1 (generate-review program 50)
        rev2 (generate-review program 50)
        kb-fn1 (vector-get (vector-get kb1 1) 0)
        kb-fn2 (vector-get (vector-get kb2 1) 0)
        kb-type1 (vector-get (vector-get kb1 2) 0)
        kb-type2 (vector-get (vector-get kb2 2) 0)
        doc-sections1 (vector-get doc1 4)
        doc-sections2 (vector-get doc2 4)
        doc-section11 (vector-get doc-sections1 0)
        doc-section12 (vector-get doc-sections1 1)
        doc-section21 (vector-get doc-sections2 0)
        doc-section22 (vector-get doc-sections2 1)]
    (do
      (print (if (string-eq (vector-get html1 1) (vector-get html2 1)) 1 0))
      (print (if (string-eq (vector-get html1 2) (vector-get html2 2)) 1 0))
      (print (if (= (vector-get kb1 0) (vector-get kb2 0)) 1 0))
      (print (if (= (vector-get kb-fn1 0) (vector-get kb-fn2 0)) 1 0))
      (print (if (string-eq (vector-get kb-fn1 1) (vector-get kb-fn2 1)) 1 0))
      (print (if (string-eq (vector-get kb-type1 1) (vector-get kb-type2 1)) 1 0))
      (print (if (string-eq (vector-get doc1 3) (vector-get doc2 3)) 1 0))
      (print (if (= (vector-length doc-sections1) (vector-length doc-sections2)) 1 0))
      (print (if (string-eq (vector-get doc-section11 0) (vector-get doc-section21 0)) 1 0))
      (print (if (= (vector-get doc-section11 1) (vector-get doc-section21 1)) 1 0))
      (print (if (string-eq (vector-get doc-section12 0) (vector-get doc-section22 0)) 1 0))
      (print (if (= (vector-get doc-section12 1) (vector-get doc-section22 1)) 1 0))
      (print (if (= (vector-get rev1 0) (vector-get rev2 0)) 1 0))
      (print (if (= (vector-length (vector-get rev1 1)) (vector-length (vector-get rev2 1))) 1 0))
      0)))
"#;

    let combined = format!("{}\n{}", selfhost_doctools_runtime_bundle(), harness);
    let output = compile_and_run(&combined);
    let lines: Vec<&str> = output.trim().lines().collect();

    assert_eq!(
        lines,
        vec![
            "1", "1", "1", "1", "1", "1", "1", "1", "1", "1", "1", "1", "1", "1"
        ]
    );
}

/// D-3: HtmlDoc.ls が supported subset の実 HTML を決定的に描画できること
#[test]
#[ignore]
fn test_e2e_selfhost_doctools_html_doc_render_html_supported_subset() {
    let harness = r#"
(defn string-contains-loop [haystack needle i hlen nlen]
  (if (> (+ i nlen) hlen)
    0
    (if (string-eq (substring haystack i (+ i nlen)) needle)
      1
      (string-contains-loop haystack needle (+ i 1) hlen nlen))))

(defn string-contains [haystack needle]
  (let [hlen (string-length haystack)
        nlen (string-length needle)]
    (if (= nlen 0)
      1
      (if (> nlen hlen)
        0
        (string-contains-loop haystack needle 0 hlen nlen)))))

(defn main []
  (let [program (parse-program "(defn add [x y] (+ x y)) (type Num Int)")
        doc (generate-html program 0)
        html1 (render-html doc 0)
        html2 (render-html doc 0)]
    (do
      (print (if (string-eq html1 html2) 1 0))
      (print (if (= (string-contains html1 "<!doctype html>") 1) 1 0))
      (print (if (= (string-contains html1 "<title>module-global</title>") 1) 1 0))
      (print (if (= (string-contains html1 "<section id=\"functions\">") 1) 1 0))
      (print (if (= (string-contains html1 "<section id=\"types\">") 1) 1 0))
      0)))
"#;

    let combined = format!("{}\n{}", selfhost_cli_html_runtime_bundle(), harness);
    let output = compile_and_run(&combined);
    let lines: Vec<&str> = output.trim().lines().collect();

    assert_eq!(lines, vec!["1", "1", "1", "1", "1"]);
}

/// D-4: Cli.ls の parse-diagnostics-count が正常ソースで 0 を返すこと
#[test]
#[ignore]
fn test_e2e_selfhost_cli_parse_diagnostics() {
    let harness = r#"
(defn main []
  (let [diag-count (parse-diagnostics-count "(defn main [] 42)")]
    (do
      (print diag-count)
      0)))
"#;

    let combined = format!("{}\n{}", selfhost_cli_runtime_bundle(), harness);
    let output = compile_and_run(&combined);
    let lines: Vec<&str> = output.trim().lines().collect();

    assert_eq!(
        lines.last().unwrap(),
        &"0",
        "正常ソースの parse diagnostics は 0 件であるべき"
    );
}

/// D-4: Cli.ls の parse-diagnostics-count が recovery 対象入力で 1 を返すこと
#[test]
#[ignore]
fn test_e2e_selfhost_cli_parse_diagnostics_recovery_error() {
    let harness = r#"
(defn main []
  (let [diag-count (parse-diagnostics-count ")")]
    (do
      (print diag-count)
      0)))
"#;

    let combined = format!("{}\n{}", selfhost_cli_runtime_bundle(), harness);
    let output = compile_and_run(&combined);
    let lines: Vec<&str> = output.trim().lines().collect();

    assert_eq!(
        lines,
        vec!["1"],
        "unexpected ')' の parse diagnostics は 1 件であるべき"
    );
}

/// D-4: Cli.ls の check-diagnostics-count が正常ソースで 0 を返すこと
#[test]
#[ignore]
fn test_e2e_selfhost_cli_check_diagnostics() {
    let harness = r#"
(defn main []
  (let [diag-count (check-diagnostics-count "(defn main [] 42)")]
    (do
      (print diag-count)
      0)))
"#;

    let combined = format!("{}\n{}", selfhost_cli_runtime_bundle(), harness);
    let output = compile_and_run(&combined);
    let lines: Vec<&str> = output.trim().lines().collect();

    assert_eq!(
        lines.last().unwrap(),
        &"0",
        "正常ソースの check diagnostics は 0 件であるべき"
    );
}

/// D-4: Cli.ls の check-diagnostics-count が型エラー入力で 1 を返すこと
#[test]
#[ignore]
fn test_e2e_selfhost_cli_check_diagnostics_type_error() {
    let harness = r#"
(defn main []
  (let [diag-count (check-diagnostics-count "(defn main [] (if 42 1 0))")]
    (do
      (print diag-count)
      0)))
"#;

    let combined = format!("{}\n{}", selfhost_cli_runtime_bundle(), harness);
    let output = compile_and_run(&combined);
    let lines: Vec<&str> = output.trim().lines().collect();

    assert_eq!(
        lines,
        vec!["1"],
        "if 条件の型エラーは check diagnostics 1 件であるべき"
    );
}

// === DOC-02 統合テスト ===

/// DOC-02: DocTools.generate-html → HtmlDoc.render-html パイプラインが実 HTML を返す
#[test]
#[ignore]
fn test_e2e_selfhost_doctools_html_template_pipeline() {
    let harness = r#"
(defn string-contains-loop [haystack needle i hlen nlen]
  (if (> (+ i nlen) hlen)
    0
    (if (string-eq (substring haystack i (+ i nlen)) needle)
      1
      (string-contains-loop haystack needle (+ i 1) hlen nlen))))

(defn string-contains [haystack needle]
  (let [hlen (string-length haystack)
        nlen (string-length needle)]
    (if (= nlen 0)
      1
      (if (> nlen hlen)
        0
        (string-contains-loop haystack needle 0 hlen nlen)))))

(defn main []
  (let [program (parse-program "(defn add [x y] (+ x y)) (type Num Int)")
        doc (generate-html program 0)
        html (render-html doc 0)]
    (do
      ;; 出力が非空
      (print (if (> (string-length html) 0) 1 0))
      ;; <!doctype html> で始まる
      (print (if (string-eq (substring html 0 15) "<!doctype html>") 1 0))
      ;; <section id="functions"> を含む
      (print (if (= (string-contains html "<section id=\"functions\">") 1) 1 0))
      ;; <section id="types"> を含む
      (print (if (= (string-contains html "<section id=\"types\">") 1) 1 0))
      ;; </body></html> で終わる (base-layout)
      (print (if (= (string-contains html "</body></html>") 1) 1 0))
      ;; CSS を含む (base-layout の css-inline)
      (print (if (= (string-contains html "<style>") 1) 1 0))
      0)))
"#;

    let combined = format!("{}\n{}", selfhost_cli_html_runtime_bundle(), harness);
    let output = compile_and_run(&combined);
    let lines: Vec<&str> = output.trim().lines().collect();

    assert_eq!(lines, vec!["1", "1", "1", "1", "1", "1"]);
}

/// DOC-02: render-html の出力が deterministic
#[test]
#[ignore]
fn test_e2e_selfhost_doctools_html_template_deterministic() {
    let harness = r#"
(defn main []
  (let [program (parse-program "(defn foo [x] x) (type Bar Int)")
        doc (generate-html program 0)
        html1 (render-html doc 0)
        html2 (render-html doc 0)]
    (do
      (print (if (string-eq html1 html2) 1 0))
      (print (if (> (string-length html1) 100) 1 0))
      0)))
"#;

    let combined = format!("{}\n{}", selfhost_cli_html_runtime_bundle(), harness);
    let output = compile_and_run(&combined);
    let lines: Vec<&str> = output.trim().lines().collect();

    assert_eq!(lines, vec!["1", "1"]);
}

/// DOC-02: render-html の出力にタイムスタンプ・ホスト名・絶対パスが含まれない
#[test]
#[ignore]
fn test_e2e_selfhost_doctools_html_template_no_timestamp() {
    let harness = r#"
(defn string-contains-loop [haystack needle i hlen nlen]
  (if (> (+ i nlen) hlen)
    0
    (if (string-eq (substring haystack i (+ i nlen)) needle)
      1
      (string-contains-loop haystack needle (+ i 1) hlen nlen))))

(defn string-contains [haystack needle]
  (let [hlen (string-length haystack)
        nlen (string-length needle)]
    (if (= nlen 0)
      1
      (if (> nlen hlen)
        0
        (string-contains-loop haystack needle 0 hlen nlen)))))

(defn main []
  (let [program (parse-program "(defn main [] 42)")
        doc (generate-html program 0)
        html (render-html doc 0)]
    (do
      ;; タイムスタンプが含まれない
      (print (if (= (string-contains html "2026") 0) 1 0))
      ;; ホスト名パターンが含まれない
      (print (if (= (string-contains html "hostname") 0) 1 0))
      ;; 絶対パスが含まれない
      (print (if (= (string-contains html "/Users/") 0) 1 0))
      (print (if (= (string-contains html "/home/") 0) 1 0))
      0)))
"#;

    let combined = format!("{}\n{}", selfhost_cli_html_runtime_bundle(), harness);
    let output = compile_and_run(&combined);
    let lines: Vec<&str> = output.trim().lines().collect();

    assert_eq!(lines, vec!["1", "1", "1", "1"]);
}

// === HtmlDoc 単体テスト ===

/// HtmlDoc.render-function-signature が "<li>fn-{id}/{arity}</li>" 形式を返す
#[test]
#[ignore]
fn test_e2e_selfhost_htmldoc_render_function_signature() {
    let harness = r#"
(defn main []
  (let [func-doc (vector-push (vector-push (vector-push (vector-new 3) 42) "add") 3)
        result (render-function-signature func-doc)]
    (do
      (print (if (string-eq result "<li>add/3</li>") 1 0))
      0)))
"#;
    let combined = format!("{}\n{}", selfhost_cli_html_runtime_bundle(), harness);
    let output = compile_and_run(&combined);
    assert_eq!(output.trim(), "1");
}

/// HtmlDoc.render-function-signature は関数名を HTML エスケープする
#[test]
#[ignore]
fn test_e2e_selfhost_htmldoc_render_function_signature_escapes_html() {
    let harness = r#"
(defn main []
  (let [func-doc (vector-push (vector-push (vector-push (vector-new 3) 42) "<danger>") 3)
        result (render-function-signature func-doc)]
    (do
      (print (if (string-eq result "<li>&lt;danger&gt;/3</li>") 1 0))
      0)))
"#;
    let combined = format!("{}\n{}", selfhost_cli_html_runtime_bundle(), harness);
    let output = compile_and_run(&combined);
    assert_eq!(output.trim(), "1");
}

/// HtmlDoc.render-type-definition が "<li>{kind} {name}</li>" 形式を返す
#[test]
#[ignore]
fn test_e2e_selfhost_htmldoc_render_type_definition() {
    let harness = r#"
(defn main []
  (let [type-doc (vector-push (vector-push (vector-push (vector-new 3) 99) "Pair") "recorddef")
        result (render-type-definition type-doc)]
    (do
      (print (if (string-eq result "<li>recorddef Pair</li>") 1 0))
      0)))
"#;
    let combined = format!("{}\n{}", selfhost_cli_html_runtime_bundle(), harness);
    let output = compile_and_run(&combined);
    assert_eq!(output.trim(), "1");
}

/// HtmlDoc.render-type-definition は型名を HTML エスケープする
#[test]
#[ignore]
fn test_e2e_selfhost_htmldoc_render_type_definition_escapes_html() {
    let harness = r#"
(defn main []
  (let [type-doc (vector-push (vector-push (vector-push (vector-new 3) 99) "\"Quoted\"") "recorddef")
        result (render-type-definition type-doc)]
    (do
      (print (if (string-eq result "<li>recorddef &quot;Quoted&quot;</li>") 1 0))
      0)))
"#;
    let combined = format!("{}\n{}", selfhost_cli_html_runtime_bundle(), harness);
    let output = compile_and_run(&combined);
    assert_eq!(output.trim(), "1");
}

/// HtmlDoc.render-module-page が <main><h1>...</h1>... 構造を持つ
#[test]
#[ignore]
fn test_e2e_selfhost_htmldoc_render_module_page_structure() {
    let harness = r#"
(defn string-contains-loop [haystack needle i hlen nlen]
  (if (> (+ i nlen) hlen)
    0
    (if (string-eq (substring haystack i (+ i nlen)) needle)
      1
      (string-contains-loop haystack needle (+ i 1) hlen nlen))))

(defn string-contains [haystack needle]
  (let [hlen (string-length haystack)
        nlen (string-length needle)]
    (if (= nlen 0)
      1
      (if (> nlen hlen)
        0
        (string-contains-loop haystack needle 0 hlen nlen)))))

(defn main []
  (let [program (parse-program "(defn add [x y] (+ x y)) (defn sub [a b] (- a b)) (type Pair Int)")
        doc (generate-html program 0)
        page (render-module-page doc)]
    (do
      ;; <main><h1> で始まる
      (print (if (string-eq (substring page 0 10) "<main><h1>") 1 0))
      ;; </main> で終わる
      (let [len (string-length page)]
        (print (if (string-eq (substring page (- len 7) len) "</main>") 1 0)))
      ;; 関数セクションが存在する
      (print (if (= (string-contains page "<section id=\"functions\">") 1) 1 0))
      ;; 型セクションが存在する
      (print (if (= (string-contains page "<section id=\"types\">") 1) 1 0))
      ;; 関数シグネチャが code block に含まれる
      (print (if (= (string-contains page "<pre><code>add/2</code></pre>") 1) 1 0))
      ;; 型エントリが kind + name で <li> に含まれる
      (print (if (= (string-contains page "<li>type Pair</li>") 1) 1 0))
      0)))
"#;
    let combined = format!("{}\n{}", selfhost_cli_html_runtime_bundle(), harness);
    let output = compile_and_run(&combined);
    let lines: Vec<&str> = output.trim().lines().collect();
    assert_eq!(lines, vec!["1", "1", "1", "1", "1", "1"]);
}

/// HtmlDoc.render-module-page が function doc/example metadata を描画し HTML escape する
#[test]
#[ignore]
fn test_e2e_selfhost_htmldoc_render_module_page_function_metadata() {
    let harness = r#"
(defn string-contains-loop [haystack needle i hlen nlen]
  (if (> (+ i nlen) hlen)
    0
    (if (string-eq (substring haystack i (+ i nlen)) needle)
      1
      (string-contains-loop haystack needle (+ i 1) hlen nlen))))

(defn string-contains [haystack needle]
  (let [hlen (string-length haystack)
        nlen (string-length needle)]
    (if (= nlen 0)
      1
      (if (> nlen hlen)
        0
        (string-contains-loop haystack needle 0 hlen nlen)))))

(defn main []
  (let [program (parse-program "(defn add [x y] :params [(x \"left\") (y \"right\")] :returns \"sum\" :doc \"<danger>\" :example [(add 1 2)] (+ x y))")
        doc (generate-html program 0)
        page (render-module-page doc)]
    (do
      (print (if (= (string-contains page "<section id=\"functions\">") 1) 1 0))
      (print (if (= (string-contains page "add/2") 1) 1 0))
      (print (if (= (string-contains page "<li><code>x</code>: left (Int)</li>") 1) 1 0))
      (print (if (= (string-contains page "<li><code>y</code>: right (Int)</li>") 1) 1 0))
      (print (if (= (string-contains page "<p><strong>Returns:</strong> sum (Int)</p>") 1) 1 0))
      (print (if (= (string-contains page "&lt;danger&gt;") 1) 1 0))
      (print (if (= (string-contains page "<danger>") 0) 1 0))
      (print (if (= (string-contains page "<pre><code>(add 1 2)</code></pre>") 1) 1 0))
      0)))
"#;
    let combined = format!("{}\n{}", selfhost_cli_html_runtime_bundle(), harness);
    let output = compile_and_run(&combined);
    let lines: Vec<&str> = output.trim().lines().collect();
    assert_eq!(lines, vec!["1", "1", "1", "1", "1", "1", "1", "1"]);
}

/// HtmlDoc.render-html が完全な HTML ドキュメントを返し、title がエスケープされる
#[test]
#[ignore]
fn test_e2e_selfhost_htmldoc_render_html_full_document() {
    let harness = r#"
(defn string-contains-loop [haystack needle i hlen nlen]
  (if (> (+ i nlen) hlen)
    0
    (if (string-eq (substring haystack i (+ i nlen)) needle)
      1
      (string-contains-loop haystack needle (+ i 1) hlen nlen))))

(defn string-contains [haystack needle]
  (let [hlen (string-length haystack)
        nlen (string-length needle)]
    (if (= nlen 0)
      1
      (if (> nlen hlen)
        0
        (string-contains-loop haystack needle 0 hlen nlen)))))

(defn main []
  (let [program (parse-program "(defn main [] 42)")
        doc (generate-html program 0)
        html (render-html doc 0)]
    (do
      ;; <!doctype html> で始まる
      (print (if (string-eq (substring html 0 15) "<!doctype html>") 1 0))
      ;; <html> を含む
      (print (if (= (string-contains html "<html>") 1) 1 0))
      ;; <head> を含む
      (print (if (= (string-contains html "<head>") 1) 1 0))
      ;; <body> を含む
      (print (if (= (string-contains html "<body>") 1) 1 0))
      ;; </body></html> で終わる
      (let [len (string-length html)]
        (print (if (string-eq (substring html (- len 14) len) "</body></html>") 1 0)))
      ;; <title> を含む
      (print (if (= (string-contains html "<title>") 1) 1 0))
      0)))
"#;
    let combined = format!("{}\n{}", selfhost_cli_html_runtime_bundle(), harness);
    let output = compile_and_run(&combined);
    let lines: Vec<&str> = output.trim().lines().collect();
    assert_eq!(lines, vec!["1", "1", "1", "1", "1", "1"]);
}

/// HtmlDoc.render-index がモジュール一覧ページを生成する
#[test]
#[ignore]
fn test_e2e_selfhost_htmldoc_render_index() {
    let harness = r#"
(defn string-contains-loop [haystack needle i hlen nlen]
  (if (> (+ i nlen) hlen)
    0
    (if (string-eq (substring haystack i (+ i nlen)) needle)
      1
      (string-contains-loop haystack needle (+ i 1) hlen nlen))))

(defn string-contains [haystack needle]
  (let [hlen (string-length haystack)
        nlen (string-length needle)]
    (if (= nlen 0)
      1
      (if (> nlen hlen)
        0
        (string-contains-loop haystack needle 0 hlen nlen)))))

(defn main []
  (let [modules (vector-push (vector-push (vector-push (vector-new 3) "Parser") "Lexer") "DocTools")
        html (render-index modules)]
    (do
      ;; <!doctype html> で始まる
      (print (if (string-eq (substring html 0 15) "<!doctype html>") 1 0))
      ;; <h1>modules</h1> を含む
      (print (if (= (string-contains html "<h1>modules</h1>") 1) 1 0))
      ;; 各モジュール名が <li> に含まれる
      (print (if (= (string-contains html "<li>Parser</li>") 1) 1 0))
      (print (if (= (string-contains html "<li>Lexer</li>") 1) 1 0))
      (print (if (= (string-contains html "<li>DocTools</li>") 1) 1 0))
      0)))
"#;
    let combined = format!("{}\n{}", selfhost_cli_html_runtime_bundle(), harness);
    let output = compile_and_run(&combined);
    let lines: Vec<&str> = output.trim().lines().collect();
    assert_eq!(lines, vec!["1", "1", "1", "1", "1"]);
}

/// HtmlDoc: 関数も型もない場合の render-module-page
#[test]
#[ignore]
fn test_e2e_selfhost_htmldoc_render_module_page_empty() {
    let harness = r#"
(defn string-contains-loop [haystack needle i hlen nlen]
  (if (> (+ i nlen) hlen)
    0
    (if (string-eq (substring haystack i (+ i nlen)) needle)
      1
      (string-contains-loop haystack needle (+ i 1) hlen nlen))))

(defn string-contains [haystack needle]
  (let [hlen (string-length haystack)
        nlen (string-length needle)]
    (if (= nlen 0)
      1
      (if (> nlen hlen)
        0
        (string-contains-loop haystack needle 0 hlen nlen)))))

(defn main []
  (let [program (parse-program "(defn main [] 42)")
        doc (generate-html program 0)
        page (render-module-page doc)]
    (do
      ;; <main><h1> で始まる
      (print (if (string-eq (substring page 0 10) "<main><h1>") 1 0))
      ;; 関数セクションは存在する (main が抽出される)
      (print (if (= (string-contains page "<section id=\"functions\">") 1) 1 0))
      ;; 型セクションは存在しない
      (print (if (= (string-contains page "<section id=\"types\">") 0) 1 0))
      0)))
"#;
    let combined = format!("{}\n{}", selfhost_cli_html_runtime_bundle(), harness);
    let output = compile_and_run(&combined);
    let lines: Vec<&str> = output.trim().lines().collect();
    assert_eq!(lines, vec!["1", "1", "1"]);
}

/// HtmlDoc.render-module-page は title を HTML エスケープする
#[test]
#[ignore]
fn test_e2e_selfhost_htmldoc_render_module_page_escapes_title() {
    let harness = r#"
(defn string-contains-loop [haystack needle i hlen nlen]
  (if (> (+ i nlen) hlen)
    0
    (if (string-eq (substring haystack i (+ i nlen)) needle)
      1
      (string-contains-loop haystack needle (+ i 1) hlen nlen))))

(defn string-contains [haystack needle]
  (let [hlen (string-length haystack)
        nlen (string-length needle)]
    (if (= nlen 0)
      1
      (if (> nlen hlen)
        0
        (string-contains-loop haystack needle 0 hlen nlen)))))

(defn main []
  (let [doc (vector-push
              (vector-push
                (vector-push
                  (vector-push
                    (vector-push (vector-new 5) 1)
                    "<module>")
                  "functions:0,types:0")
                (vector-new 0))
              (vector-new 0))
        page (render-module-page doc)]
    (do
      (print (if (= (string-contains page "<h1>&lt;module&gt;</h1>") 1) 1 0))
      (print (if (= (string-contains page "<h1><module></h1>") 0) 1 0))
      0)))
"#;
    let combined = format!("{}\n{}", selfhost_cli_html_runtime_bundle(), harness);
    let output = compile_and_run(&combined);
    let lines: Vec<&str> = output.trim().lines().collect();
    assert_eq!(lines, vec!["1", "1"]);
}

/// HtmlDoc.render-guide-page が guide 本文を含む完全な HTML を返す
#[test]
#[ignore]
fn test_e2e_selfhost_htmldoc_render_guide_page() {
    let harness = r#"
(defn string-contains-loop [haystack needle i hlen nlen]
  (if (> (+ i nlen) hlen)
    0
    (if (string-eq (substring haystack i (+ i nlen)) needle)
      1
      (string-contains-loop haystack needle (+ i 1) hlen nlen))))

(defn string-contains [haystack needle]
  (let [hlen (string-length haystack)
        nlen (string-length needle)]
    (if (= nlen 0)
      1
      (if (> nlen hlen)
        0
        (string-contains-loop haystack needle 0 hlen nlen)))))

(defn main []
  (let [html (render-guide-page "Quick Start" "<h1>Quick Start</h1><p>hello</p>")]
    (do
      (print (if (string-eq (substring html 0 15) "<!doctype html>") 1 0))
      (print (if (= (string-contains html "<title>Quick Start</title>") 1) 1 0))
      (print (if (= (string-contains html "<main class=\"guide\">") 1) 1 0))
      (print (if (= (string-contains html "<h1>Quick Start</h1>") 1) 1 0))
      (print (if (= (string-contains html "<p>hello</p>") 1) 1 0))
      0)))
"#;
    let combined = format!("{}\n{}", selfhost_cli_html_runtime_bundle(), harness);
    let output = compile_and_run(&combined);
    let lines: Vec<&str> = output.trim().lines().collect();
    assert_eq!(lines, vec!["1", "1", "1", "1", "1"]);
}

/// HtmlDoc.render-doc-site-index が guides と modules を並べた index を返す
#[test]
#[ignore]
fn test_e2e_selfhost_htmldoc_render_doc_site_index() {
    let harness = r#"
(defn string-contains-loop [haystack needle i hlen nlen]
  (if (> (+ i nlen) hlen)
    0
    (if (string-eq (substring haystack i (+ i nlen)) needle)
      1
      (string-contains-loop haystack needle (+ i 1) hlen nlen))))

(defn string-contains [haystack needle]
  (let [hlen (string-length haystack)
        nlen (string-length needle)]
    (if (= nlen 0)
      1
      (if (> nlen hlen)
        0
        (string-contains-loop haystack needle 0 hlen nlen)))))

(defn guide-link [href label]
  (let [node (vector-new 2)]
    (vector-push
      (vector-push node href)
      label)))

(defn main []
  (let [guides (vector-push
                 (vector-push
                   (vector-push (vector-new 3)
                     (guide-link "guides/quick-start.html" "Quick Start"))
                   (guide-link "guides/language-reference.html" "Language Reference"))
                 (guide-link "guides/package-layout.html" "Package Layout"))
        modules (vector-push (vector-push (vector-new 2) "Core") "List")
        html (render-doc-site-index guides modules)]
    (do
      (print (if (string-eq (substring html 0 15) "<!doctype html>") 1 0))
      (print (if (= (string-contains html "<h1>L# Documentation</h1>") 1) 1 0))
      (print (if (= (string-contains html "<a href=\"guides/package-layout.html\">Package Layout</a>") 1) 1 0))
      (print (if (= (string-contains html "<a href=\"api/Core.html\">Core</a>") 1) 1 0))
      (print (if (= (string-contains html "<a href=\"api/List.html\">List</a>") 1) 1 0))
      0)))
"#;
    let combined = format!("{}\n{}", selfhost_cli_html_runtime_bundle(), harness);
    let output = compile_and_run(&combined);
    let lines: Vec<&str> = output.trim().lines().collect();
    assert_eq!(lines, vec!["1", "1", "1", "1", "1"]);
}
