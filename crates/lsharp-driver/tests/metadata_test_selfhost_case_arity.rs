//! `I-45` / `CASE-ZERO-ARITY-01` の contract。
//!
//! `lsharp test` は `--format json` を付けない限り embedded selfhost component へ委譲される
//! (`should_delegate_test_to_embedded_component_args`)。ここで見るのは **selfhost lane** の
//! 挙動であり、Rust lane (`--format json`) は既に緑なので収束先として control に使う。
//!
//! 受入条件は 2 つの観測を同時に要求する:
//! - `lsharp test` の exit code (一致すれば 0 / 外れれば 非 0)
//! - `coverage.executed` が 1 以上 (preflight で短絡すると 0 になる)
//!
//! arity 1 の control を同じ fixture 群に置き、arity だけが変数であることを示す。

use serde_json::Value;
use std::path::PathBuf;
use std::process::Command;

/// selfhost lane は cwd を preopen dir として受け取るため、fixture は cwd 直下に置く。
struct Fixture {
    root: PathBuf,
}

impl Fixture {
    fn new(name: &str, source_text: &str) -> Self {
        let root =
            std::env::temp_dir().join(format!("lsharp_case_arity_{}_{}", name, std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        std::fs::create_dir_all(&root).expect("fixture directory の作成に失敗");
        std::fs::write(root.join("input.ls"), source_text).expect("fixture の書き込みに失敗");
        Self { root }
    }
}

impl Drop for Fixture {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.root);
    }
}

struct Report {
    exit_code: Option<i32>,
    json: Value,
}

impl Report {
    fn status(&self) -> &str {
        self.json["implementation_conformance"]["status"]
            .as_str()
            .unwrap_or("<missing>")
    }

    fn executed(&self) -> i64 {
        self.json["implementation_conformance"]["coverage"]["executed"]
            .as_i64()
            .unwrap_or(-1)
    }

    fn runner(&self) -> &str {
        self.json["implementation_conformance"]["provenance"]["runner"]
            .as_str()
            .unwrap_or("<missing>")
    }
}

fn run_selfhost_test(name: &str, source_text: &str) -> Report {
    let fixture = Fixture::new(name, source_text);
    let output = Command::new(env!("CARGO_BIN_EXE_lsharp"))
        .current_dir(&fixture.root)
        .args(["test", "input.ls"])
        .output()
        .expect("lsharp test の実行に失敗");
    let stdout = String::from_utf8(output.stdout).expect("stdout は UTF-8 であるべき");
    let line = stdout
        .trim()
        .lines()
        .next_back()
        .unwrap_or_default()
        .to_string();
    let json: Value = serde_json::from_str(&line).unwrap_or_else(|e| {
        panic!(
            "selfhost lane の report が JSON ではない ({e}): stdout={stdout} stderr={}",
            String::from_utf8_lossy(&output.stderr)
        )
    });
    Report {
        exit_code: output.status.code(),
        json,
    }
}

const ZERO_ARITY_ACTUAL: &str = concat!(
    "(defn zero [] 1)\n",
    "(defn caller [] :case [(expect (zero) 1)] (zero))\n",
    "(defn main [] 0)\n"
);

const ZERO_ARITY_EXPECTED_SIDE: &str = concat!(
    "(defn zero [] 1)\n",
    "(defn caller [] :case [(expect 1 (zero))] (zero))\n",
    "(defn main [] 0)\n"
);

const ZERO_ARITY_MISMATCH: &str = concat!(
    "(defn zero [] 1)\n",
    "(defn caller [] :case [(expect (zero) 2)] (zero))\n",
    "(defn main [] 0)\n"
);

const ARITY_ONE_CONTROL: &str = concat!(
    "(defn incr [x] (+ x 1))\n",
    "(defn caller [x] :case [(expect (incr 1) 2)] (incr x))\n",
    "(defn main [] 0)\n"
);

const ARITY_ONE_MISMATCH_CONTROL: &str = concat!(
    "(defn incr [x] (+ x 1))\n",
    "(defn caller [x] :case [(expect (incr 1) 99)] (incr x))\n",
    "(defn main [] 0)\n"
);

#[test]
fn selfhost_case_zero_arity_actual_side_is_executed_and_passes() {
    let report = run_selfhost_test("zero_actual", ZERO_ARITY_ACTUAL);
    assert_eq!(
        report.runner(),
        "selfhost",
        "既定 lane は selfhost であるべき"
    );
    assert!(
        report.executed() >= 1,
        "0 引数 defn を actual 側に置いた case は実行されるべき: executed={} json={}",
        report.executed(),
        report.json
    );
    assert_eq!(report.status(), "pass", "json={}", report.json);
    assert_eq!(report.exit_code, Some(0), "json={}", report.json);
}

#[test]
fn selfhost_case_zero_arity_expected_side_is_executed_and_passes() {
    let report = run_selfhost_test("zero_expected", ZERO_ARITY_EXPECTED_SIDE);
    assert!(
        report.executed() >= 1,
        "0 引数 defn を expected 側に置いた case も実行されるべき: executed={} json={}",
        report.executed(),
        report.json
    );
    assert_eq!(report.status(), "pass", "json={}", report.json);
    assert_eq!(report.exit_code, Some(0), "json={}", report.json);
}

#[test]
fn selfhost_case_zero_arity_mismatch_is_executed_and_fails() {
    let report = run_selfhost_test("zero_mismatch", ZERO_ARITY_MISMATCH);
    assert!(
        report.executed() >= 1,
        "期待値が外れた場合も preflight 短絡ではなく実行された結果として落ちるべき: \
         executed={} json={}",
        report.executed(),
        report.json
    );
    assert_eq!(report.status(), "fail", "json={}", report.json);
    assert_eq!(report.exit_code, Some(1), "json={}", report.json);
}

#[test]
fn selfhost_case_arity_one_control_is_executed_and_passes() {
    let report = run_selfhost_test("one_control", ARITY_ONE_CONTROL);
    assert!(
        report.executed() >= 1,
        "arity 1 の control: executed={} json={}",
        report.executed(),
        report.json
    );
    assert_eq!(report.status(), "pass", "json={}", report.json);
    assert_eq!(report.exit_code, Some(0), "json={}", report.json);
}

#[test]
fn selfhost_case_arity_one_mismatch_control_is_executed_and_fails() {
    let report = run_selfhost_test("one_mismatch", ARITY_ONE_MISMATCH_CONTROL);
    assert!(
        report.executed() >= 1,
        "arity 1 の不一致 control: executed={} json={}",
        report.executed(),
        report.json
    );
    assert_eq!(report.status(), "fail", "json={}", report.json);
    assert_eq!(report.exit_code, Some(1), "json={}", report.json);
}
