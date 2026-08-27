use super::*;
use lsharp_ir::lower::Lower;
use lsharp_types::infer::Infer;

fn compile_preview1(source: &str) -> Vec<u8> {
    let program = lsharp_syntax::parse(source).unwrap();
    let mut infer = Infer::new();
    let type_results = infer.infer_program(&program).unwrap();
    let mut lower = Lower::new();
    let module = lower.lower_program(&program, &type_results).unwrap();
    crate::wasi::emit_wasm_wasi(&module).unwrap()
}

fn compile_preview2(source: &str) -> Vec<u8> {
    let program = lsharp_syntax::parse(source).unwrap();
    let mut infer = Infer::new();
    let type_results = infer.infer_program(&program).unwrap();
    let mut lower = Lower::new();
    let module = lower.lower_program(&program, &type_results).unwrap();
    crate::wasi::emit_wasm_wasi_p2(&module).unwrap()
}

#[test]
fn test_run_wasm_wasi_invalid_bytes() {
    // 不正な Wasm バイナリでエラーが返ること
    let result = run_wasm_wasi(&[0, 1, 2, 3]);
    assert!(result.is_err());
    assert!(
        result
            .unwrap_err()
            .contains("Wasm モジュールの読み込みに失敗")
    );
}

#[test]
fn test_classify_wasi_runtime_failure_maps_allocator_capacity_trap() {
    let error = "実行に失敗: error while executing at wasm backtrace:\n    0: <unknown>!<wasm function 10>: wasm trap: wasm `unreachable` instruction executed";

    let classified = classify_wasi_runtime_failure(error);

    assert!(classified.starts_with("LS4002:"), "分類結果: {classified}");
    assert!(classified.contains(error));
}

#[test]
fn test_classify_wasi_runtime_failure_maps_root_capacity_trap() {
    let error = "実行に失敗: error while executing at wasm backtrace:\n    0: <unknown>!<wasm function 22>: wasm trap: wasm `unreachable` instruction executed";

    let classified = classify_wasi_runtime_failure(error);

    assert!(classified.starts_with("LS4002:"), "分類結果: {classified}");
}

#[test]
fn test_classify_wasi_runtime_failure_maps_root_slot_invariant_trap() {
    let error = "実行に失敗: error while executing at wasm backtrace:\n    0: <unknown>!<wasm function 24>: wasm trap: wasm `unreachable` instruction executed";

    let classified = classify_wasi_runtime_failure(error);

    assert!(classified.starts_with("LS4003:"), "分類結果: {classified}");
    assert!(classified.contains(error));
}

#[test]
fn test_classify_wasi_runtime_failure_maps_root_slot_backtrace_without_trap_text() {
    let error = "_start 実行に失敗: error while executing at wasm backtrace:\n    0: <unknown>!<wasm function 24>";

    let classified = classify_wasi_runtime_failure(error);

    assert!(classified.starts_with("LS4003:"), "分類結果: {classified}");
}

#[test]
fn test_classify_wasi_runtime_failure_preserves_other_traps() {
    let error = "実行に失敗: error while executing at wasm backtrace:\n    0: <unknown>!<wasm function 27>: wasm trap: wasm `unreachable` instruction executed";

    assert_eq!(classify_wasi_runtime_failure(error), error);
}

#[test]
fn test_run_wasm_wasi_hello() {
    // 実際のコンパイラで hello world を実行
    let wasm_bytes = compile_preview1(r#"(defn main [] (print 42))"#);

    let result = run_wasm_wasi(&wasm_bytes);
    assert!(result.is_ok());
    assert_eq!(result.unwrap().trim(), "42");
}

#[test]
fn test_run_wasm_wasi_capture_preserves_exit_code() {
    let wasm_bytes = compile_preview1("(defn main [] (do (proc-exit 17) 0))");

    let result = run_wasm_wasi_with_dir_args_and_stdin_capture(&wasm_bytes, None, &[], "")
        .expect("capture helper should succeed");
    assert_eq!(result.exit_code, 17);
    assert_eq!(result.stdout, "");
}

#[test]
fn test_run_wasm_wasi_capture_uses_provided_stdin() {
    let wasm_bytes = compile_preview1("(defn main [] (do (print-string (read-stdin)) 0))");

    let result =
        run_wasm_wasi_with_dir_args_and_stdin_capture(&wasm_bytes, None, &[], "stdin-smoke")
            .expect("capture helper should succeed");
    assert_eq!(result.exit_code, 0);
    assert_eq!(result.stdout, "stdin-smoke");
}

#[test]
fn test_run_wasm_wasi_capture_uses_long_provided_stdin() {
    let wasm_bytes = compile_preview1("(defn main [] (do (print-string (read-stdin)) 0))");
    let stdin = "lsp-wire-".repeat(700);

    let result = run_wasm_wasi_with_dir_args_and_stdin_capture(&wasm_bytes, None, &[], &stdin)
        .expect("capture helper should succeed");
    assert_eq!(result.exit_code, 0);
    assert_eq!(result.stdout, stdin);
}

#[test]
fn test_run_wasm_wasi_capture_reads_large_stdin_fully() {
    let wasm_bytes = compile_preview1("(defn main [] (print (string-length (read-stdin))))");
    let stdin = "abcdefghij".repeat(500);

    let result = run_wasm_wasi_with_dir_args_and_stdin_capture(&wasm_bytes, None, &[], &stdin)
        .expect("capture helper should read large stdin");

    assert_eq!(result.exit_code, 0);
    assert_eq!(result.stdout.trim(), stdin.len().to_string());
}

#[test]
fn test_run_wasm_wasi_capture_reads_soak_sized_stdin_fully() {
    let wasm_bytes = compile_preview1("(defn main [] (print (string-length (read-stdin))))");
    let stdin = "lsp-wire-".repeat(850);

    let result = run_wasm_wasi_with_dir_args_and_stdin_capture(&wasm_bytes, None, &[], &stdin)
        .expect("capture helper should read soak-sized stdin");

    assert_eq!(result.exit_code, 0);
    assert_eq!(result.stdout.trim(), stdin.len().to_string());
}

#[test]
fn test_run_wasm_wasi_capture_preserves_lsp_soak_wire_stdin() {
    let wasm_bytes = compile_preview1("(defn main [] (do (print-string (read-stdin)) 0))");
    let open_source = "(defn helper [] 1)\n(defn main [] (helper 1))";
    let change_source = "(defn helper [] 1)\n(defn main []  (he))";
    let iterations = 12usize;

    let render_wire_frame = |body: &str| format!("Content-Length: {}\r\n\r\n{}", body.len(), body);
    let repeat_rendered_frames = |frames: &[String], iterations: usize| {
        let mut rendered = String::new();
        for _ in 0..iterations {
            for frame in frames {
                rendered.push_str(frame);
            }
        }
        rendered
    };

    let init_body = r#"{"jsonrpc":"2.0","id":80,"method":"initialize","params":0}"#;
    let open_body = format!(
        r#"{{"jsonrpc":"2.0","method":"textDocument/didOpen","params":{{"uri":42,"source":"{}"}}}}"#,
        open_source
    );
    let hover_body = r#"{"jsonrpc":"2.0","id":81,"method":"textDocument/hover","params":{"uri":42,"line":1,"col":21}}"#;
    let change_body = format!(
        r#"{{"jsonrpc":"2.0","method":"textDocument/didChange","params":{{"uri":42,"source":"{}"}}}}"#,
        change_source
    );
    let completion_body = r#"{"jsonrpc":"2.0","id":82,"method":"textDocument/completion","params":{"uri":42,"line":1,"col":23}}"#;
    let formatting_body =
        r#"{"jsonrpc":"2.0","id":83,"method":"textDocument/formatting","params":{"uri":42}}"#;

    let stdin = format!(
        "{}{}",
        render_wire_frame(init_body),
        repeat_rendered_frames(
            &[
                render_wire_frame(&open_body),
                render_wire_frame(hover_body),
                render_wire_frame(&change_body),
                render_wire_frame(completion_body),
                render_wire_frame(formatting_body),
            ],
            iterations
        )
    );

    let result = run_wasm_wasi_with_dir_args_and_stdin_capture(&wasm_bytes, None, &[], &stdin)
        .expect("capture helper should preserve lsp soak wire stdin");

    assert_eq!(result.exit_code, 0);
    assert_eq!(result.stdout, stdin);
}

#[test]
fn test_run_wasm_wasi_capture_preserves_lsp_soak_wire_after_reading_args() {
    let wasm_bytes = compile_preview1(
        r#"
            (defn main []
              (do
                (print-string (command-line-arg 0))
                (print-string "\n---\n")
                (print-string (command-line-arg 1))
                (print-string "\n---\n")
                (print-string (read-stdin))
                0))
            "#,
    );
    let open_source = "(defn helper [] 1)\n(defn main [] (helper 1))";
    let change_source = "(defn helper [] 1)\n(defn main []  (he))";
    let iterations = 12usize;

    let render_wire_frame = |body: &str| format!("Content-Length: {}\r\n\r\n{}", body.len(), body);
    let repeat_rendered_frames = |frames: &[String], iterations: usize| {
        let mut rendered = String::new();
        for _ in 0..iterations {
            for frame in frames {
                rendered.push_str(frame);
            }
        }
        rendered
    };

    let init_body = r#"{"jsonrpc":"2.0","id":80,"method":"initialize","params":0}"#;
    let open_body = format!(
        r#"{{"jsonrpc":"2.0","method":"textDocument/didOpen","params":{{"uri":42,"source":"{}"}}}}"#,
        open_source
    );
    let hover_body = r#"{"jsonrpc":"2.0","id":81,"method":"textDocument/hover","params":{"uri":42,"line":1,"col":21}}"#;
    let change_body = format!(
        r#"{{"jsonrpc":"2.0","method":"textDocument/didChange","params":{{"uri":42,"source":"{}"}}}}"#,
        change_source
    );
    let completion_body = r#"{"jsonrpc":"2.0","id":82,"method":"textDocument/completion","params":{"uri":42,"line":1,"col":23}}"#;
    let formatting_body =
        r#"{"jsonrpc":"2.0","id":83,"method":"textDocument/formatting","params":{"uri":42}}"#;

    let stdin = format!(
        "{}{}",
        render_wire_frame(init_body),
        repeat_rendered_frames(
            &[
                render_wire_frame(&open_body),
                render_wire_frame(hover_body),
                render_wire_frame(&change_body),
                render_wire_frame(completion_body),
                render_wire_frame(formatting_body),
            ],
            iterations
        )
    );
    let expected = format!("lsp\n---\n--stdio\n---\n{}", stdin);

    let result = run_wasm_wasi_with_dir_args_and_stdin_capture(
        &wasm_bytes,
        None,
        &["lsp", "--stdio"],
        &stdin,
    )
    .expect("capture helper should preserve stdin after reading args");

    assert_eq!(result.exit_code, 0);
    assert_eq!(result.stdout, expected);
}

#[test]
fn test_wasi_mode_enum_exists() {
    // WasiMode enum の各バリアントが存在し、区別できること
    let p1 = WasiMode::Preview1;
    let p2 = WasiMode::Preview2;
    assert_ne!(p1, p2);
    assert_eq!(p1, WasiMode::Preview1);
    assert_eq!(p2, WasiMode::Preview2);
}

#[test]
fn test_run_wasm_with_mode_keeps_mode_specific_invalid_artifact_diagnostics() {
    let preview1 = run_wasm_with_mode(&[0, 1, 2, 3], WasiMode::Preview1)
        .expect_err("Preview1 の不正 artifact は拒否されるべき");
    assert!(preview1.contains("Wasm モジュールの読み込みに失敗"));

    let preview2 = run_wasm_with_mode(&[0, 1, 2, 3], WasiMode::Preview2)
        .expect_err("Preview2 の不正 artifact は拒否されるべき");
    assert!(preview2.contains("Component の読み込みに失敗"));
}

#[test]
fn test_wasi_mode_debug_display() {
    // WasiMode の Debug 表示が正しいこと
    assert_eq!(format!("{:?}", WasiMode::Preview1), "Preview1");
    assert_eq!(format!("{:?}", WasiMode::Preview2), "Preview2");
}

#[test]
fn test_wasi_mode_copy_clone() {
    // WasiMode が Copy + Clone を実装していること
    let mode = WasiMode::Preview2;
    let copied = mode;
    let cloned = mode;
    assert_eq!(mode, copied);
    assert_eq!(mode, cloned);
}

#[test]
fn test_run_wasm_with_mode_dispatches_preview1_core_module() {
    let wasm_bytes = compile_preview1(r#"(defn main [] (print 42))"#);

    let result = run_wasm_with_mode(&wasm_bytes, WasiMode::Preview1);

    assert!(
        result.is_ok(),
        "preview1 dispatch should succeed: {result:?}"
    );
    assert_eq!(result.unwrap().trim(), "42");
}

#[test]
fn test_run_wasm_with_mode_dispatches_preview2_component() {
    let component_bytes = compile_preview2(r#"(defn main [] (print 42))"#);

    let result = run_wasm_with_mode(&component_bytes, WasiMode::Preview2);

    assert!(
        result.is_ok(),
        "preview2 dispatch should succeed: {result:?}"
    );
    assert_eq!(result.unwrap().trim(), "42");
}

#[test]
fn test_run_wasm_with_mode_capture_preserves_preview2_stdin_and_stdout() {
    let component_bytes = compile_preview2("(defn main [] (do (print-string (read-stdin)) 0))");

    let result = run_wasm_with_mode_capture(
        &component_bytes,
        WasiMode::Preview2,
        None,
        &[],
        "preview2-stdin",
    );

    assert!(
        result.is_ok(),
        "preview2 capture helper should preserve stdin/stdout: {result:?}"
    );
    let output = result.unwrap();
    assert_eq!(output.exit_code, 0);
    assert_eq!(output.stdout, "preview2-stdin");
}

#[test]
fn test_run_wasm_component_invalid_bytes() {
    // 不正なバイナリで適切なエラーが返ること
    let result = run_wasm_component(&[0, 1, 2, 3]);
    assert!(result.is_err());
    assert!(result.unwrap_err().contains("Component の読み込みに失敗"));
}

#[test]
fn test_run_wasm_component_minimal() {
    // 最小限の Component Wasm (run export なし) はエラーを返すこと
    let component_bytes = build_minimal_component_wasm();
    let result = run_wasm_component(&component_bytes);
    // P1 の _start 不在と同様、run 関数がない component はエラー
    assert!(result.is_err(), "run export なしの component は失敗すべき");
    let err = result.unwrap_err();
    assert!(
        err.contains("run 関数が見つかりません"),
        "エラーメッセージに run 関数不在が含まれること: {err}"
    );
}

#[test]
fn test_run_wasm_component_plain_run_export_without_result() {
    let component_bytes = wat::parse_str(
        r#"
(component
  (core module $main
    (func (export "run"))
  )
  (core instance $main (instantiate $main))
  (type (func))
  (alias core export $main "run" (core func $run))
  (func $run (type 0) (canon lift (core func $run)))
  (export "run" (func $run))
)
"#,
    )
    .expect("component wat should parse");

    let result = run_wasm_component(&component_bytes);
    assert!(
        result.is_ok(),
        "plain run export component should execute via fallback: {result:?}"
    );
    assert_eq!(
        result.unwrap(),
        "",
        "plain run export fallback should not invent stdout"
    );
}

#[test]
fn test_run_wasm_component_plain_run_export_with_dir_argument() {
    let component_bytes = wat::parse_str(
        r#"
(component
  (core module $main
    (func (export "run"))
  )
  (core instance $main (instantiate $main))
  (type (func))
  (alias core export $main "run" (core func $run))
  (func $run (type 0) (canon lift (core func $run)))
  (export "run" (func $run))
)
"#,
    )
    .expect("component wat should parse");

    let temp_dir = std::env::temp_dir();
    let result = run_wasm_component_with_dir_args_and_stdin(
        &component_bytes,
        Some(temp_dir.as_path()),
        &["--version"],
        "",
    );
    assert!(
        result.is_ok(),
        "plain run export component should execute with preopened dir: {result:?}"
    );
}

#[test]
fn test_component_trap_error_preserves_captured_stdout() {
    let error = format_component_trap_with_stdout(
        "Component 実行に失敗: unreachable".to_string(),
        b"diagnostics:1\n",
    );

    assert_eq!(
        error,
        "Component 実行に失敗: unreachable; stdout_lossy=\"diagnostics:1\\n\""
    );
}

/// テスト用: 最小限の WASI Component Wasm を構築する
/// wasm-encoder を使って空の command component を生成
fn build_minimal_component_wasm() -> Vec<u8> {
    use wasm_encoder::Component;

    // wasm-encoder で最小の component binary を構築
    // 空の component (imports/exports なし) として正常にインスタンス化できる
    let component = Component::new();
    component.finish()
}

// --- I-91 / WASI-RUNNER-EXIT-STDOUT-01 ---
//
// exit code 非 0 のとき、runner は捕捉済みの stdout を捨てていた。
// selfhost CLI の `cli-stderr` は fd 2 ではなく `print-string` 経由で fd 1 へ書くので
// (`selfhost/src/App/EmbeddedCli.ls:1306`)、診断に必要な `error: ...` 行は
// まさにその捨てられていた stdout の中にある。

#[test]
fn test_run_wasm_wasi_nonzero_exit_reports_captured_stdout() {
    let wasm_bytes = compile_preview1(r#"(defn main [] (do (print-string "error: boom") (proc-exit 1) 0))"#);

    let err = run_wasm_wasi_with_dir_args_and_stdin(&wasm_bytes, None, &[], "")
        .expect_err("exit code 1 は Err になるべき");

    assert!(
        err.contains("exit code 1"),
        "既存の前置きを逐語で保つこと: {err}"
    );
    assert!(
        err.contains("error: boom"),
        "捕捉済み stdout を失敗メッセージへ載せること: {err}"
    );
}

#[test]
fn test_run_wasm_wasi_nonzero_exit_marks_empty_stdout() {
    // stdout が空であること自体が所見になる (= 出力が fd 2 か、そもそも出ていない)。
    // 黙って何も足さないと「載せ忘れ」と区別が付かないので明示する。
    let wasm_bytes = compile_preview1("(defn main [] (do (proc-exit 3) 0))");

    let err = run_wasm_wasi_with_dir_args_and_stdin(&wasm_bytes, None, &[], "")
        .expect_err("exit code 3 は Err になるべき");

    assert!(err.contains("exit code 3"), "{err}");
    assert!(
        err.contains("<空>"),
        "stdout が空であることを明示すること: {err}"
    );
}

#[test]
fn test_run_wasm_with_mode_preview1_nonzero_exit_reports_captured_stdout() {
    let wasm_bytes = compile_preview1(r#"(defn main [] (do (print-string "error: mode-boom") (proc-exit 1) 0))"#);

    let err = run_wasm_with_mode_and_dir_args_and_stdin(
        &wasm_bytes,
        WasiMode::Preview1,
        None,
        &[],
        "",
    )
    .expect_err("exit code 1 は Err になるべき");

    assert!(err.contains("WASI Preview1 実行に失敗"), "{err}");
    assert!(err.contains("exit code 1"), "{err}");
    assert!(
        err.contains("error: mode-boom"),
        "mode dispatch 経路でも stdout を載せること: {err}"
    );
}

#[test]
fn test_format_nonzero_exit_error_keeps_prefix_verbatim_and_appends_stdout() {
    let message = format_nonzero_exit_error("実行に失敗", 1, "error: boom\n");

    assert_eq!(
        message,
        "実行に失敗: exit code 1; stdout=\"error: boom\\n\"",
        "既存の前置きは逐語で残し、後ろへ足すだけにすること"
    );
}

#[test]
fn test_format_nonzero_exit_error_marks_empty_stdout_explicitly() {
    let message = format_nonzero_exit_error("Component 実行に失敗", 2, "");

    assert_eq!(message, "Component 実行に失敗: exit code 2; stdout=<空>");
}

#[test]
fn test_format_nonzero_exit_error_keeps_head_and_tail_of_long_stdout() {
    // CLI の `error:` 行は最後に出るので、末尾を落とす切り方では診断にならない。
    let mut stdout = String::new();
    stdout.push_str("HEAD-MARKER");
    stdout.push_str(&"x".repeat(8192));
    stdout.push_str("error: TAIL-MARKER");

    let message = format_nonzero_exit_error("実行に失敗", 1, &stdout);

    assert!(
        message.contains("HEAD-MARKER"),
        "先頭を残すこと: {message}"
    );
    assert!(
        message.contains("error: TAIL-MARKER"),
        "末尾を残すこと: {message}"
    );
    assert!(
        message.contains("bytes 省略"),
        "省略した事実と量を明示すること: {message}"
    );
    assert!(
        message.len() < stdout.len(),
        "省略が効いていること: {} vs {}",
        message.len(),
        stdout.len()
    );
}

#[test]
fn test_render_captured_stdout_does_not_truncate_at_boundary_length() {
    // 上限ちょうどは省略しない (境界で挙動が飛ばないこと)。
    let stdout = "y".repeat(CAPTURED_STDOUT_HEAD_BYTES + CAPTURED_STDOUT_TAIL_BYTES);

    let rendered = render_captured_stdout(&stdout);

    assert!(!rendered.contains("bytes 省略"), "境界では省略しないこと");
}
