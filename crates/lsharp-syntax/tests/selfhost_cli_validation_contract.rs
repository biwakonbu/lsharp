use std::fs;
use std::path::PathBuf;

fn selfhost_cli_source() -> String {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../selfhost/src/App/Cli.ls");
    fs::read_to_string(&path)
        .unwrap_or_else(|error| panic!("selfhost Cli.ls の読み込みに失敗 {}: {error}", path.display()))
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
}
