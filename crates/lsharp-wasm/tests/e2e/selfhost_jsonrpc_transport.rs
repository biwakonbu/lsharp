use super::support::*;

fn selfhost_jsonrpc_transport_bundle() -> String {
    selfhost_module("JsonRpc.ls").to_string()
}

fn run_transport_harness(harness: &str) -> String {
    compile_and_run(&format!("{}\n{}", selfhost_jsonrpc_transport_bundle(), harness))
}

/// LSP transport: Content-Length の簡易パースが決定的に動くこと
#[test]
fn test_e2e_selfhost_jsonrpc_parse_content_length() {
    let output = run_transport_harness(
        r#"
(module Main)
(defn main []
  (print (parse-content-length "Content-Length: 42\r\n\r\n")))
"#,
    );

    assert_eq!(output, "42\n", "parse-content-length は 42 を返すべき");
}

/// LSP transport: JSON-RPC response が Content-Length 付き frame になること
#[test]
fn test_e2e_selfhost_jsonrpc_render_initialize_frame() {
    let body = r#"{"jsonrpc":"2.0","id":1,"result":[1,1,1,1,1,1,1]}"#;
    let expected_frame = format!("Content-Length: {}\r\n\r\n{}", body.len(), body);
    let output = run_transport_harness(
        r#"
(module Main)
(defn main []
  (print-string (render-initialize-frame 1)))
"#,
    );

    assert_eq!(
        output,
        expected_frame,
        "initialize response は Content-Length framed JSON であるべき"
    );
}

/// LSP transport: error response も frame 化されること
#[test]
fn test_e2e_selfhost_jsonrpc_render_error_frame() {
    let body = r#"{"jsonrpc":"2.0","id":7,"error":{"code":-32600,"message":"bad request"}}"#;
    let expected_frame = format!("Content-Length: {}\r\n\r\n{}", body.len(), body);
    let output = run_transport_harness(
        r#"
(module Main)
(defn main []
  (print-string (render-rpc-error-response-frame 7 -32600 "bad request")))
"#,
    );

    assert_eq!(
        output,
        expected_frame,
        "error response は Content-Length framed JSON であるべき"
    );
}
