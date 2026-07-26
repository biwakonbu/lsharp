use std::fs;
use std::path::PathBuf;

fn selfhost_cli_source() -> String {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../selfhost/src/App/Cli.ls");
    fs::read_to_string(&path)
        .unwrap_or_else(|error| panic!("selfhost Cli.ls の読み込みに失敗 {}: {error}", path.display()))
}

fn selfhost_evidence_source() -> String {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../selfhost/src/Tools/Validation/Evidence.ls");
    fs::read_to_string(&path)
        .unwrap_or_else(|error| panic!("selfhost Evidence.ls の読み込みに失敗 {}: {error}", path.display()))
}

fn selfhost_json_rpc_source() -> String {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../selfhost/src/Tools/Lsp/JsonRpc.ls");
    fs::read_to_string(&path)
        .unwrap_or_else(|error| panic!("selfhost JsonRpc.ls の読み込みに失敗 {}: {error}", path.display()))
}

#[test]
fn selfhost_cli_validation_surface_is_registered() {
    let source = selfhost_cli_source();

    assert!(
        source.contains("(defn cmd-validate []"),
        "App.Cli は validate command id を公開するべき"
    );
    assert!(
        source.contains("validate --source"),
        "App.Cli help は validate --source surface を説明するべき"
    );
    assert!(
        source.contains("(defn run-validate-source"),
        "App.Cli は source validation の実装入口を持つべき"
    );
    assert!(
        source.contains("trace-gap.claim-without-test"),
        "selfhost source validation は claim の未接続を明示するべき"
    );
    assert!(
        source.contains("(defn parse-validate-cli-option"),
        "App.Cli は validate の source/json option 契約を検査するべき"
    );
    assert!(
        source.contains("--emit-manifest"),
        "App.Cli help/options は source validation manifest 出力を公開するべき"
    );
    assert!(
        source.contains("(defn parse-validate-cli-options"),
        "App.Cli は validate の manifest output option を解析するべき"
    );
    assert!(
        source.contains("format-seen"),
        "App.Cli は validate の format option を必須として追跡するべき"
    );
    assert!(
        source.contains("validate-options-source-path"),
        "App.Cli は option の並び順に依存せず source path を保持するべき"
    );
    assert!(
        source.contains("validation-source-manifest-json"),
        "App.Cli は source graph の version 1 manifest JSON projection を利用するべき"
    );
    let evidence = selfhost_evidence_source();
    assert!(
        evidence.contains("validation-source-manifest-json-state"),
        "source manifest serializer は native x86 の多引数再帰を避ける state-loop を持つべき"
    );
    assert!(
        !evidence.contains(
            "(vector-push-quad-rooted-v3 (vector-new 4) items idx len out)"
        ),
        "source manifest serializer の state constructor は native x86 の 4 引数 rooted helper を避けるべき"
    );
    assert!(
        evidence.contains("(defn validation-source-manifest-json-state [state]"),
        "source manifest serializer の state boundary は native x86 の 1 引数に限定するべき"
    );
    let node_loop_start = evidence
        .find("(defn validation-source-nodes-json-state-loop")
        .expect("source manifest serializer は node state loop を持つべき");
    let node_loop_end = evidence[node_loop_start..]
        .find("(defn validation-source-int-array-json-state-loop")
        .map(|offset| node_loop_start + offset)
        .expect("node state loop の終端を特定できるべき");
    let node_loop = &evidence[node_loop_start..node_loop_end];
    assert!(
        node_loop.contains("state0 (vector-new 4)")
            && node_loop.contains("(vector-push-single-rooted-v3 state0")
            && node_loop.contains("(root_push state)")
            && node_loop.contains("(root_pop)")
            && !node_loop.contains("(vector-set-at-rooted-v3 state 1 (+ idx 1))"),
        "native x86 の node manifest loop は state を root し、object/string を含む state を vector-set で更新せず fresh rooted state として進めるべき"
    );
    assert!(
        node_loop.contains("(root_pop)\n                    (root_push next-state)\n                    (let [result (validation-source-nodes-json-state-loop next-state)]")
            && !node_loop.contains(
                "(root_push next-state)\n                    (root_pop)\n                    (root_pop)\n                    (let [result (validation-source-nodes-json-state-loop next-state)]"
            ),
        "native x86 の node manifest loop は next-state を recursive call の直前まで root として保持するべき"
    );
    assert!(
        node_loop.contains("(let [node-json (validation-source-node-json")
            && node_loop.contains("(root_push node-json)")
            && node_loop.contains("(validation-json-append out node-json)")
            && !node_loop.contains(
                "(validation-json-append out (validation-source-node-json"
            ),
        "native x86 の node manifest loop は string return を root してから append call へ渡すべき"
    );
    let node_json_start = evidence
        .find("(defn validation-source-node-json")
        .expect("source manifest serializer は node JSON helper を持つべき");
    let node_json_end = evidence[node_json_start..]
        .find("(defn validation-source-nodes-json-state-loop")
        .map(|offset| node_json_start + offset)
        .expect("node JSON helper の終端を特定できるべき");
    let node_json = &evidence[node_json_start..node_json_end];
    assert!(
        node_json.contains("(root_push node)")
            && node_json.contains("(validation-source-node-json-state-loop state)")
            && node_json.contains("(vector-new 3)"),
        "native x86 の node JSON helper は入力 node と rooted state を 1 引数 state-loop へ渡すべき"
    );
    let manifest_start = evidence
        .find("(defn validation-source-manifest-json [graph]")
        .expect("source manifest serializer の manifest helper を特定できるべき");
    let manifest_end = evidence[manifest_start..]
        .find("(defn source-evidence-edge-form-result")
        .map(|offset| manifest_start + offset)
        .expect("manifest helper の終端を特定できるべき");
    let manifest = &evidence[manifest_start..manifest_end];
    assert!(
        manifest.contains("(root_push graph)")
            && manifest.contains("(root_push nodes)")
            && manifest.contains("(root_push edges)")
            && manifest.contains("(root_push registry)")
            && manifest.contains("(root_push nodes-state)")
            && manifest.contains("(root_push evidence-state)")
            && manifest.contains("(root_push edges-state)"),
        "native x86 の manifest helper は複数 serializer state を作る間 graph/vector を GC root として保持するべき"
    );
    let check_start = source
        .find("(defn run-check-program")
        .expect("App.Cli は run-check-program を持つべき");
    let check_end = source[check_start..]
        .find("(defn run-check-source")
        .map(|offset| check_start + offset)
        .expect("run-check-program の終端を特定できるべき");
    let check_body = &source[check_start..check_end];
    assert_eq!(
        check_body.matches("(root_pop)").count(),
        6,
        "run-check-program は JSON/text 各分岐で context/program/analysis の root lease を全て release するべき"
    );
    let json_test_start = source
        .find("(defn run-test-source-json")
        .expect("App.Cli は run-test-source-json を持つべき");
    let json_test_end = source[json_test_start..]
        .find("(defn case-preflight-diagnostics-summary")
        .map(|offset| json_test_start + offset)
        .expect("run-test-source-json の終端を特定できるべき");
    assert_eq!(
        source[json_test_start..json_test_end]
            .matches("(root_pop)")
            .count(),
        12,
        "run-test-source-json は preflight/suite 各経路で4つの root leaseを解放するべき"
    );
    let text_test_start = source
        .find("(defn run-test-source-text")
        .expect("App.Cli は run-test-source-text を持つべき");
    let text_test_end = source[text_test_start..]
        .find("(defn run-test-source [")
        .map(|offset| text_test_start + offset)
        .expect("run-test-source-text の終端を特定できるべき");
    assert_eq!(
        source[text_test_start..text_test_end]
            .matches("(root_pop)")
            .count(),
        12,
        "run-test-source-text は preflight/suite 各経路で4つの root leaseを解放するべき"
    );
}

#[test]
fn selfhost_validation_json_append_roots_nested_concat() {
    let evidence = selfhost_evidence_source();
    let append_start = evidence
        .find("(defn validation-json-append [out piece]")
        .expect("validation-json-append を特定できるべき");
    let append_end = evidence[append_start..]
        .find("(defn validation-json-field")
        .map(|offset| append_start + offset)
        .expect("validation-json-append の終端を特定できるべき");
    let append = &evidence[append_start..append_end];

    assert!(
        append.contains("(root_push out)")
            && append.contains("(root_push piece)")
            && append.contains("(let [comma-piece (string-concat \",\" piece)]")
            && append.contains("(root_push comma-piece)")
            && append.contains("(let [result (string-concat out comma-piece)]")
            && append.contains("(root_push result)"),
        "native x86 の JSON append は concat 前に out/piece を保持し、inner concat と outer concat の結果も root lease で保持するべき"
    );
    assert!(
        !append.contains("(string-concat out (string-concat \",\" piece))"),
        "native x86 の JSON append は nested string-concat を直接評価するべきではない"
    );
}

#[test]
fn selfhost_validation_json_string_literal_roots_nested_concat() {
    let evidence = selfhost_evidence_source();
    let literal_start = evidence
        .find("(defn validation-json-string-literal [value]")
        .expect("validation-json-string-literal を特定できるべき");
    let literal_end = evidence[literal_start..]
        .find("(defn validation-json-string-field")
        .map(|offset| literal_start + offset)
        .expect("validation-json-string-literal の終端を特定できるべき");
    let literal = &evidence[literal_start..literal_end];

    assert!(
        literal.contains("(root_push value)")
            && literal.contains("(let [escaped (json-escape-string value)]")
            && literal.contains("(root_push escaped)")
            && literal.contains(r#"(let [quoted (string-concat escaped "\"")]"#)
            && literal.contains("(root_push quoted)")
            && literal.contains(r#"(let [result (string-concat "\"" quoted)]"#)
            && literal.contains("(root_push result)"),
        "native x86 の JSON string literal は各 concat の入力と中間結果を root lease で保持するべき"
    );
    assert!(
        !literal.contains(r#"(string-concat "\"" (string-concat (json-escape-string value) "\""))"#),
        "native x86 の JSON string literal は nested string-concat を直接評価するべきではない"
    );
}

#[test]
fn selfhost_validation_json_field_roots_nested_concat() {
    let evidence = selfhost_evidence_source();
    let field_start = evidence
        .find("(defn validation-json-field [name value-json]")
        .expect("validation-json-field を特定できるべき");
    let field_end = evidence[field_start..]
        .find("(defn validation-json-string-literal")
        .map(|offset| field_start + offset)
        .expect("validation-json-field の終端を特定できるべき");
    let field = &evidence[field_start..field_end];

    assert!(
        field.contains("(root_push name)")
            && field.contains("(root_push value-json)")
            && field.contains("(let [colon-value (string-concat")
            && field.contains("(root_push colon-value)")
            && field.contains("(let [name-colon (string-concat name colon-value)]")
            && field.contains("(root_push name-colon)")
            && field.contains("(let [result (string-concat")
            && field.contains("(root_push result)"),
        "native x86 の JSON field は各 concat の入力と中間結果を root lease で保持するべき"
    );
    assert!(
        !field.contains("(string-concat name (string-concat"),
        "native x86 の JSON field は nested string-concat を直接評価するべきではない"
    );
}

#[test]
fn selfhost_validation_json_object_wrap_roots_nested_concat() {
    let evidence = selfhost_evidence_source();
    let object_start = evidence
        .find("(defn validation-json-object-wrap [body]")
        .expect("validation-json-object-wrap を特定できるべき");
    let object_end = evidence[object_start..]
        .find("(defn validation-json-array-wrap")
        .map(|offset| object_start + offset)
        .expect("validation-json-object-wrap の終端を特定できるべき");
    let object = &evidence[object_start..object_end];

    assert!(
        object.contains("(root_push body)")
            && object.contains("(let [closed-body (string-concat body")
            && object.contains("(root_push closed-body)")
            && object.contains("(let [result (string-concat")
            && object.contains("(root_push result)"),
        "native x86 の JSON object wrapper は body と各 concat 結果を root lease で保持するべき"
    );
    assert!(
        !object.contains("(string-concat \"{\" (string-concat body"),
        "native x86 の JSON object wrapper は nested string-concat を直接評価するべきではない"
    );
}

#[test]
fn selfhost_validation_json_array_wrap_roots_nested_concat() {
    let evidence = selfhost_evidence_source();
    let array_start = evidence
        .find("(defn validation-json-array-wrap [body]")
        .expect("validation-json-array-wrap を特定できるべき");
    let array_end = evidence[array_start..]
        .find("(defn validation-json-append")
        .map(|offset| array_start + offset)
        .expect("validation-json-array-wrap の終端を特定できるべき");
    let array = &evidence[array_start..array_end];

    assert!(
        array.contains("(root_push body)")
            && array.contains("(let [closed-body (string-concat body")
            && array.contains("(root_push closed-body)")
            && array.contains("(let [result (string-concat")
            && array.contains("(root_push result)"),
        "native x86 の JSON array wrapper は body と各 concat 結果を root lease で保持するべき"
    );
    assert!(
        !array.contains("(string-concat \"[\" (string-concat body"),
        "native x86 の JSON array wrapper は nested string-concat を直接評価するべきではない"
    );
}

#[test]
fn selfhost_validation_json_subject_roots_outer_concat() {
    let evidence = selfhost_evidence_source();
    let subject_start = evidence
        .find("(defn validation-source-subject-json [subject]")
        .expect("validation-source-subject-json を特定できるべき");
    let subject_end = evidence[subject_start..]
        .find("(defn validation-source-evidence-json")
        .map(|offset| subject_start + offset)
        .expect("validation-source-subject-json の終端を特定できるべき");
    let subject = &evidence[subject_start..subject_end];

    assert!(
        subject.contains("(root_push subject)")
            && subject.contains("(root_push fields0)")
            && subject.contains("(root_push id-object-fields)")
            && subject.contains("namespace-field (validation-json-string-field")
            && subject.contains("key-field (validation-json-string-field")
            && subject.contains("(root_push namespace-field)")
            && subject.contains("(root_push key-field)")
            && subject.contains("comma-key-fields (string-concat \",\" key-field)")
            && subject.contains("(root_push comma-key-fields)")
            && subject.contains("(let [comma-id-fields (string-concat \",\" id-object-fields)]")
            && subject.contains("(root_push comma-id-fields)")
            && subject.contains("(let [all-fields (string-concat fields0 comma-id-fields)]")
            && subject.contains("(root_push all-fields)"),
        "native x86 の JSON subject は outer concat の入力と中間結果を root lease で保持するべき"
    );
    assert!(
        !subject.contains(
            "(validation-json-object-wrap (string-concat fields0 (string-concat \",\" id-object-fields)))"
        )
            && !subject.contains(
                "(string-concat\n        (validation-json-string-field \"namespace\""
            )
            && !subject.contains(
                "(string-concat \",\"\n          (validation-json-string-field \"key\""
            ),
        "native x86 の JSON subject は nested string-concat を直接評価するべきではない"
    );
}

#[test]
fn selfhost_validation_manifest_write_roots_nested_payload() {
    let source = selfhost_cli_source();
    let writer_start = source
        .find("(defn validation-source-write-manifest [graph manifest-path]")
        .expect("App.Cli は source manifest writer を持つべき");
    let writer_end = source[writer_start..]
        .find("(defn run-validate-source")
        .map(|offset| writer_start + offset)
        .expect("source manifest writer の終端を特定できるべき");
    let writer = &source[writer_start..writer_end];

    assert!(
        writer.contains("(root_push graph)")
            && writer.contains("(root_push manifest-path)")
            && writer.contains(
                "(let [manifest-json (validation-source-manifest-json graph)]"
            )
            && writer.contains("(root_push manifest-json)")
            && writer.contains("(write-file manifest-path manifest-json)")
            && !writer.contains(
                "(write-file manifest-path (validation-source-manifest-json graph))"
            ),
        "native x86 の manifest writer は path を serializer 呼び出し中に保持し、戻り文字列を root してから write-file に渡すべき"
    );
}

#[test]
fn selfhost_validation_evidence_subject_json_roots_nested_payload() {
    let evidence = selfhost_evidence_source();
    let evidence_start = evidence
        .find("(defn validation-source-evidence-json [evidence-record]")
        .expect("validation-source-evidence-json を特定できるべき");
    let evidence_end = evidence[evidence_start..]
        .find("(defn validation-source-evidence-json-state-loop")
        .map(|offset| evidence_start + offset)
        .expect("validation-source-evidence-json の終端を特定できるべき");
    let body = &evidence[evidence_start..evidence_end];

    assert!(
        body.contains(
            "subject-json (validation-source-subject-json (source-evidence-record-subject evidence-record))"
        ) && body.contains("(root_push subject-json)")
            && body.contains(
                "(validation-json-object-field \"subject\" subject-json)"
            )
            && !body.contains(
                "(validation-json-object-field \"subject\" (validation-source-subject-json"
            ),
        "native x86 の evidence JSON は nested subject JSON を root lease で保持してから object field に渡すべき"
    );
}

#[test]
fn selfhost_json_escape_loop_uses_one_arg_state_boundary() {
    let source = selfhost_json_rpc_source();
    let state_start = source
        .find("(defn json-escape-string-state-loop [state]")
        .expect("JsonRpc は json escape state-loop を持つべき");
    let wrapper_start = source
        .find("(defn json-escape-string-loop [state]")
        .expect("JsonRpc は json escape wrapper を持つべき");
    let wrapper_end = source[wrapper_start..]
        .find("(defn json-escape-string [src]")
        .map(|offset| wrapper_start + offset)
        .expect("json escape wrapper の終端を特定できるべき");
    let state_body = &source[state_start..wrapper_start];
    let wrapper_body = &source[wrapper_start..wrapper_end];

    assert!(
        state_body.contains("(json-escape-string-state-loop next-state)")
            && state_body.contains("(root_push state)")
            && state_body.contains("(root_push next-state)")
            && wrapper_body.contains("(json-escape-string-state-loop state)")
            && !source.contains("(defn json-escape-string-loop [src idx len out]")
            && !state_body.contains(
                "(json-escape-string-loop src (+ idx 1) len (string-concat out piece))"
            ),
        "native x86 の JSON escape は string/object を含む4引数 call を公開経路に残さず1引数 state-loopへ分離するべき"
    );
}

#[test]
fn selfhost_node_manifest_uses_one_arg_state_boundary() {
    let source = selfhost_evidence_source();
    let node_start = source
        .find("(defn validation-source-node-json [node]")
        .expect("Evidence は node JSON wrapper を持つべき");
    let nodes_loop_start = source
        .find("(defn validation-source-nodes-json-state-loop")
        .expect("Evidence は node collection loop を持つべき");
    let node_wrapper = &source[node_start..nodes_loop_start];
    let state_start = source
        .find("(defn validation-source-node-json-state-loop [state]")
        .expect("Evidence は node JSON の 1 引数 state-loop を持つべき");
    let state_end = source[state_start..]
        .find("(defn validation-source-nodes-json-state-loop")
        .map(|offset| state_start + offset)
        .expect("node JSON state-loop の終端を特定できるべき");
    let state_loop = &source[state_start..state_end];

    assert!(
        state_loop.contains("(root_push state)")
            && state_loop.contains("(validation-source-node-json-state-loop next-state)"),
        "native x86 の node JSON state-loop は state を root し、1 引数 state だけで再帰するべき"
    );
    assert!(
        state_loop.contains("(vector-push-triple-rooted-v3 (vector-new 3)")
            && !state_loop.contains("state0 (vector-new 3)"),
        "node JSON state-loop は各段階の中間 state vector を残さず、3 要素の rooted state を一度に構築するべき"
    );
    assert!(
        node_wrapper.contains("(root_push node)")
            && node_wrapper.contains("(validation-source-node-json-state-loop state)")
            && node_wrapper.contains("(vector-push-triple-rooted-v3 (vector-new 3)"),
        "node JSON wrapper は入力 node を保持し、rooted state を 1 引数 loop へ渡すべき"
    );
    assert!(
        !node_wrapper.contains("fields1")
            && !node_wrapper.contains("fields2")
            && !node_wrapper.contains("fields3")
            && !node_wrapper.contains("fields4")
            && !node_wrapper.contains("fields5"),
        "node JSON wrapper は複数 field intermediate を同一 call frame に積まないべき"
    );
}
