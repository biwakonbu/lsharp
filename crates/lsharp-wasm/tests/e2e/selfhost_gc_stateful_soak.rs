use super::support::*;

fn run_stateful_lsp_harness(harness: &str) -> Vec<String> {
    let source = selfhost_lsp_runtime_bundle();
    let output = compile_and_run(&format!("{}\n{}", source, harness));
    output
        .trim()
        .lines()
        .map(std::string::ToString::to_string)
        .collect()
}

/// GC-05 honest slice: 現状は stdio 付き REPL ではなく、Cli.ls 内のセッション helper を
/// 同一 Wasm プロセスで繰り返し呼び出して状態保持を検証する。
#[test]
fn test_e2e_gc_repl_stateful_single_session_metrics() {
    let repl_src_a = "(defn main [] 42)";
    let repl_src_b = "(defn main [] (if true 1 2))";
    let iterations = 50usize;
    let expected_bytes: usize = (1..=iterations)
        .map(|n| {
            if n % 2 == 0 {
                repl_src_a.len()
            } else {
                repl_src_b.len()
            }
        })
        .sum();

    let harness = format!(
        r#"
(defn repl-loop [session n]
  (if (<= n 0)
    0
    (let [src (if (= (% n 2) 0) "{repl_src_a}" "{repl_src_b}")]
      (do
        (repl-session-eval session src)
        (repl-loop session (- n 1))))))

(defn main []
  (let [session (repl-session-new)]
    (do
      (repl-loop session {iterations})
      (print (repl-session-eval-count session))
      (print (repl-session-total-input-bytes session))
      (print (repl-session-last-type-name session))
      0)))
"#
    );

    let output = compile_and_run(&format!("{}\n{}", selfhost_cli_runtime_bundle(), harness));
    let lines: Vec<&str> = output.trim().lines().collect();

    assert_eq!(
        lines[0],
        iterations.to_string(),
        "単一 REPL session の eval 回数が保持されるべき"
    );
    assert_eq!(
        lines[1],
        expected_bytes.to_string(),
        "単一 REPL session の累積入力バイト数が保持されるべき"
    );
    assert_eq!(lines[2], "100", "最後の推論型は Int=100 であるべき");
}

/// GC-05 honest slice: REPL session helper が source 群をまとめて処理し、集計を返せること
#[test]
fn test_e2e_gc_repl_session_batch_metrics() {
    let src_a = "(defn main [] 42)";
    let src_b = "(defn main [] true)";
    let expected_bytes = src_a.len() + src_b.len();

    let harness = format!(
        r#"
(defn main []
  (let [inputs (vector-push
                 (vector-push (vector-new 2) "{src_a}")
                 "{src_b}")
        summary (repl-session-run inputs)]
    (do
      (print (vector-get summary 0))
      (print (vector-get summary 1))
      (print (vector-get summary 2))
      0)))
"#
    );

    let output = compile_and_run(&format!("{}\n{}", selfhost_cli_runtime_bundle(), harness));
    let lines: Vec<&str> = output.trim().lines().collect();

    assert_eq!(lines[0], "2", "batch helper は 2 eval を処理するべき");
    assert_eq!(
        lines[1],
        expected_bytes.to_string(),
        "batch helper は累積入力バイト数を返すべき"
    );
    assert_eq!(lines[2], "200", "最後の推論型は Bool=200 であるべき");
}

/// GC-05 honest slice: 単一 REPL session を長めに回しても集計が壊れないこと
#[test]
fn test_e2e_gc_repl_stateful_long_session_metrics() {
    let repl_src_a = "(defn main [] 42)";
    let repl_src_b = "(defn main [] (if true 1 2))";
    let iterations = 200usize;
    let expected_bytes: usize = (1..=iterations)
        .map(|n| {
            if n % 2 == 0 {
                repl_src_a.len()
            } else {
                repl_src_b.len()
            }
        })
        .sum();

    let harness = format!(
        r#"
(defn repl-loop [session n]
  (if (<= n 0)
    0
    (let [src (if (= (% n 2) 0) "{repl_src_a}" "{repl_src_b}")]
      (do
        (repl-session-eval session src)
        (repl-loop session (- n 1))))))

(defn main []
  (let [session (repl-session-new)]
    (do
      (repl-loop session {iterations})
      (print (repl-session-eval-count session))
      (print (repl-session-total-input-bytes session))
      (print (repl-session-last-type-name session))
      0)))
"#
    );

    let output = compile_and_run(&format!("{}\n{}", selfhost_cli_runtime_bundle(), harness));
    let lines: Vec<&str> = output.trim().lines().collect();

    assert_eq!(
        lines[0],
        iterations.to_string(),
        "long REPL session の eval 回数が保持されるべき"
    );
    assert_eq!(
        lines[1],
        expected_bytes.to_string(),
        "long REPL session の累積入力バイト数が保持されるべき"
    );
    assert_eq!(
        lines[2], "100",
        "long REPL session の最後の推論型は Int=100 であるべき"
    );
}

/// GC-05 honest slice: hover を含む最小系列は shared state を明示的に渡して
/// open -> hover -> change -> completion -> formatting を 1 session で辿る。
/// `server-loop-step` の shared-state dispatch 自体は専用 targeted test で担保する。
#[test]
fn test_e2e_gc_lsp_stateful_session_sequence_metrics() {
    let open_src_literal = "(defn helper [] 1)\\n(defn main [] (helper 1))";
    let change_src_literal = "(defn helper [] 1)\\n(defn main []  (he))";
    let open_src = open_src_literal.replace("\\n", "\n");
    let change_src = change_src_literal.replace("\\n", "\n");

    let harness = format!(
        r#"
(defn main []
  (let [state (server-state-new)
        _ (handle-initialize 0 state)
        open-params (vector-push (vector-push (vector-new 2) 77) "{open_src_literal}")
        open-len (handle-didOpen open-params state)
        hover-params (vector-push (vector-push (vector-push (vector-new 3) 77) 2) 19)
        hover (handle-hover hover-params state)
        change-params (vector-push (vector-push (vector-new 2) 77) "{change_src_literal}")
        change-len (handle-didChange change-params state)
        completion-params (vector-push (vector-push (vector-push (vector-new 3) 77) 2) 19)
        completion (handle-completion completion-params state)
        formatting-params (vector-push (vector-new 1) 77)
        formatting (handle-formatting formatting-params state)
        edit (vector-get formatting 0)]
    (do
      (print open-len)
      (print (vector-length hover))
      (print-string (vector-get hover 1))
      (print-string "\n")
      (print change-len)
      (print (vector-length completion))
      (print-string (vector-get (vector-get completion 0) 0))
      (print-string "\n")
      (print (vector-length formatting))
      (print (if (string-eq (vector-get edit 4) "(defn helper [] 1)\n(defn main [] (he))\n") 1 0))
      (print (server-state-doc-count state))
      (print (server-state-request-count state))
      (print (server-state-source-length state))
      0)))
"#
    );

    let lines = run_stateful_lsp_harness(&harness);

    assert_eq!(
        lines[0],
        open_src.len().to_string(),
        "didOpen は session に開いた source を保持するべき"
    );
    assert_eq!(
        lines[1], "2",
        "hover response は [range, contents] の 2 要素であるべき"
    );
    assert_eq!(
        lines[2], "defn helper",
        "hover は didOpen 済み document の symbol 情報を返すべき"
    );
    assert_eq!(
        lines[3],
        change_src.len().to_string(),
        "didChange は session source を更新するべき"
    );
    assert_eq!(
        lines[4], "1",
        "completion は変更後 source の prefix から 1 件返すべき"
    );
    assert_eq!(lines[5], "helper", "completion は helper symbol を返すべき");
    assert_eq!(
        lines[6], "1",
        "formatting は変更後 document に対して 1 edit を返すべき"
    );
    assert_eq!(
        lines[7], "1",
        "formatting edit は session に保持した最新 source を使うべき"
    );
    assert_eq!(
        lines[8], "1",
        "session 中の open document 数は 1 件のままであるべき"
    );
    assert_eq!(
        lines[9], "6",
        "stateful sequence の request count が蓄積されるべき"
    );
    assert_eq!(
        lines[10],
        change_src.len().to_string(),
        "session が保持する最新 source length は didChange 後の値であるべき"
    );
}

/// GC-05 honest slice: shared-state server-loop-step を長めの単一 session で回しても
/// doc/request/source の集計が崩れないこと
#[test]
fn test_e2e_gc_lsp_stateful_repeated_sequence_metrics() {
    let open_src_literal = "(defn helper [] 1)\\n(defn main [] (helper 1))";
    let change_src_literal = "(defn helper [] 1)\\n(defn main []  (he))";
    let change_src = change_src_literal.replace("\\n", "\n");
    let iterations = 20usize;
    let expected_requests = 1 + (iterations * 5);

    let harness = format!(
        r#"
(defn make-loop-request [method-id params]
  (let [v (vector-new 2)]
    (vector-push (vector-push v method-id) params)))

(defn make-doc-params [uri src]
  (let [v (vector-new 2)]
    (vector-push (vector-push v uri) src)))

(defn run-loop [state n open-req hover-req change-req completion-req formatting-req]
  (if (<= n 0)
    0
    (do
      (server-loop-step state open-req)
      (server-loop-step state hover-req)
      (server-loop-step state change-req)
      (server-loop-step state completion-req)
      (server-loop-step state formatting-req)
      (run-loop state (- n 1) open-req hover-req change-req completion-req formatting-req))))

(defn main []
  (let [state (server-state-new)
        init-req (make-loop-request (lsp-method-initialize) 0)
        open-req (make-loop-request (lsp-method-did-open)
                   (make-doc-params 77 "{open_src_literal}"))
        hover-req (make-loop-request (lsp-method-hover)
                    (vector-push (vector-push (vector-push (vector-new 3) 77) 2) 19))
        change-req (make-loop-request (lsp-method-did-change)
                     (make-doc-params 77 "{change_src_literal}"))
        completion-req (make-loop-request (lsp-method-completion)
                         (vector-push (vector-push (vector-push (vector-new 3) 77) 2) 19))
        formatting-req (make-loop-request (lsp-method-formatting)
                        (vector-push (vector-new 1) 77))
        _ (server-loop-step state init-req)]
    (do
      (run-loop state {iterations} open-req hover-req change-req completion-req formatting-req)
      (print (server-state-doc-count state))
      (print (server-state-request-count state))
      (print (server-state-source-length state))
      0)))
"#
    );

    let lines = run_stateful_lsp_harness(&harness);

    assert_eq!(
        lines[0], "1",
        "repeated sequence でも open document 数は 1 件のままであるべき"
    );
    assert_eq!(
        lines[1],
        expected_requests.to_string(),
        "repeated sequence の request count が蓄積されるべき"
    );
    assert_eq!(
        lines[2],
        change_src.len().to_string(),
        "repeated sequence 後も最新 source length は didChange 後の値であるべき"
    );
}
