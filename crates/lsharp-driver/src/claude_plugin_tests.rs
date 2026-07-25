use super::*;

#[test]
fn test_cmd_claude_plugin_installs_mcp_server_and_skill() {
    let dir = std::env::temp_dir().join("lsharp_claude_plugin_install");
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    std::fs::write(
        dir.join("settings.json"),
        r#"{
  "language": "ja",
  "mcpServers": {
    "existing": {
      "command": "existing"
    }
  }
}"#,
    )
    .unwrap();

    let template_path = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("templates/lsharp-language-guide.md");
    let result = cmd_claude_plugin_in(&dir, &template_path);
    assert!(result.is_ok(), "claude-plugin は成功するべき: {result:?}");

    let settings: serde_json::Value =
        serde_json::from_str(&std::fs::read_to_string(dir.join("settings.json")).unwrap()).unwrap();
    assert_eq!(settings["language"], "ja");
    assert_eq!(settings["mcpServers"]["existing"]["command"], "existing");
    assert_eq!(settings["mcpServers"]["lsharp"]["command"], "lsharp");
    assert_eq!(settings["mcpServers"]["lsharp"]["args"][0], "mcp-server");

    let skill_path = dir.join("skills/lsharp-language-guide/SKILL.md");
    assert!(skill_path.exists(), "SKILL.md が必要");
    let skill = std::fs::read_to_string(skill_path).unwrap();
    assert!(skill.contains("name: lsharp-language-guide"));
    assert!(skill.contains("lsharp_stdlib_api"));

    std::fs::remove_dir_all(&dir).unwrap();
}

#[test]
fn test_cmd_claude_plugin_creates_settings_when_missing() {
    let dir = std::env::temp_dir().join("lsharp_claude_plugin_missing_settings");
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();

    let template_path = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("templates/lsharp-language-guide.md");
    let result = cmd_claude_plugin_in(&dir, &template_path);
    assert!(result.is_ok(), "claude-plugin は成功するべき: {result:?}");

    let settings_path = dir.join("settings.json");
    assert!(settings_path.exists(), "settings.json が必要");

    let settings: serde_json::Value =
        serde_json::from_str(&std::fs::read_to_string(settings_path).unwrap()).unwrap();
    assert_eq!(settings["mcpServers"]["lsharp"]["command"], "lsharp");
    assert_eq!(settings["mcpServers"]["lsharp"]["args"][0], "mcp-server");

    std::fs::remove_dir_all(&dir).unwrap();
}

#[test]
fn test_cmd_claude_plugin_creation_failure_preserves_driver_io_error_code() {
    let base = std::env::temp_dir().join("lsharp_claude_plugin_creation_failure");
    let _ = std::fs::remove_dir_all(&base);
    let _ = std::fs::remove_file(&base);
    std::fs::write(&base, "not a directory").unwrap();
    let claude_dir = base.join(".claude");
    let template_path = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("templates/lsharp-language-guide.md");

    let error = cmd_claude_plugin_in(&claude_dir, &template_path)
        .expect_err("ファイル配下の Claude ディレクトリ作成は失敗するべき");

    assert!(
        error.to_string().starts_with("[LS5001]"),
        "driver I/O 診断コードを保持するべき: {error:?}"
    );

    std::fs::remove_file(&base).unwrap();
}

#[test]
fn test_lsharp_language_guide_template_covers_user_development_workflows() {
    let template_path = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("templates/lsharp-language-guide.md");
    let skill = std::fs::read_to_string(template_path).unwrap();

    for expected in [
        "## Quick Start",
        "lsharp compile",
        "lsharp test",
        "lsharp doc",
        "## CLI Workflows",
        "## Metadata-Driven Development",
        "## Modules And Packages",
        "## Deployment Targets",
        "## Known Limits",
        "Linux x86_64",
        "Mac Apple Silicon",
    ] {
        assert!(skill.contains(expected), "skill に {expected} が必要");
    }
}

#[test]
fn test_lsharp_language_guide_template_points_to_docs_guides_as_ssot() {
    let template_path = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("templates/lsharp-language-guide.md");
    let skill = std::fs::read_to_string(template_path).unwrap();

    for expected in [
        "docs/guides/",
        "docs/guides/metadata-driven-development.md",
        "docs/guides/ide-setup.md",
        "docs/guides/deployment-targets.md",
        "docs/guides/stdlib-guide.md",
        "docs/guides/error-reference.md",
        "docs/site.toml",
        "正本",
    ] {
        assert!(skill.contains(expected), "skill に {expected} が必要");
    }
}
