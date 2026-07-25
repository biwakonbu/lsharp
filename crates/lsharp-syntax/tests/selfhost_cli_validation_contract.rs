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
