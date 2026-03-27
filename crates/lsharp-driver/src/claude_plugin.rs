use serde_json::{Map, Value, json};
use std::fs;
use std::path::{Path, PathBuf};

pub fn cmd_claude_plugin() -> miette::Result<()> {
    let claude_dir = claude_dir()?;
    let template_path =
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("templates/lsharp-language-guide.md");
    cmd_claude_plugin_in(&claude_dir, &template_path)
}

fn claude_dir() -> miette::Result<std::path::PathBuf> {
    if let Some(dir) = std::env::var_os("LSHARP_CLAUDE_DIR") {
        return Ok(std::path::PathBuf::from(dir));
    }
    let home =
        dirs::home_dir().ok_or_else(|| miette::miette!("home directory が見つかりません"))?;
    Ok(home.join(".claude"))
}

pub(crate) fn cmd_claude_plugin_in(claude_dir: &Path, template_path: &Path) -> miette::Result<()> {
    fs::create_dir_all(claude_dir)
        .map_err(|e| miette::miette!("{}: {}", claude_dir.display(), e))?;
    install_mcp_settings(claude_dir)?;
    install_skill(claude_dir, template_path)?;

    println!(
        "Claude settings updated: {}",
        claude_dir.join("settings.json").display()
    );
    println!(
        "Claude skill installed: {}",
        claude_dir
            .join("skills/lsharp-language-guide/SKILL.md")
            .display()
    );
    Ok(())
}

fn install_mcp_settings(claude_dir: &Path) -> miette::Result<()> {
    let settings_path = claude_dir.join("settings.json");
    let mut root = if settings_path.exists() {
        let content = fs::read_to_string(&settings_path)
            .map_err(|e| miette::miette!("{}: {}", settings_path.display(), e))?;
        serde_json::from_str::<Value>(&content)
            .map_err(|e| miette::miette!("{}: JSON パース失敗: {e}", settings_path.display()))?
    } else {
        json!({})
    };

    let object = ensure_object(&mut root, "settings.json")?;
    let mcp_servers = ensure_child_object(object, "mcpServers")?;
    mcp_servers.insert(
        "lsharp".to_string(),
        json!({
            "command": "lsharp",
            "args": ["mcp-server"],
            "env": {}
        }),
    );

    let content = serde_json::to_string_pretty(&root)
        .map_err(|e| miette::miette!("settings.json 直列化失敗: {e}"))?;
    fs::write(&settings_path, format!("{content}\n"))
        .map_err(|e| miette::miette!("{}: {}", settings_path.display(), e))?;
    Ok(())
}

fn install_skill(claude_dir: &Path, template_path: &Path) -> miette::Result<()> {
    let skill_dir = claude_dir.join("skills/lsharp-language-guide");
    fs::create_dir_all(&skill_dir)
        .map_err(|e| miette::miette!("{}: {}", skill_dir.display(), e))?;

    let template = fs::read_to_string(template_path)
        .map_err(|e| miette::miette!("{}: {}", template_path.display(), e))?;
    let skill_path = skill_dir.join("SKILL.md");
    fs::write(&skill_path, template)
        .map_err(|e| miette::miette!("{}: {}", skill_path.display(), e))?;
    Ok(())
}

fn ensure_object<'a>(
    value: &'a mut Value,
    context: &str,
) -> miette::Result<&'a mut Map<String, Value>> {
    value
        .as_object_mut()
        .ok_or_else(|| miette::miette!("{context} は JSON object である必要があります"))
}

fn ensure_child_object<'a>(
    parent: &'a mut Map<String, Value>,
    key: &str,
) -> miette::Result<&'a mut Map<String, Value>> {
    let entry = parent
        .entry(key.to_string())
        .or_insert_with(|| Value::Object(Map::new()));
    entry
        .as_object_mut()
        .ok_or_else(|| miette::miette!("{key} は JSON object である必要があります"))
}

#[cfg(test)]
mod tests {
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
            serde_json::from_str(&std::fs::read_to_string(dir.join("settings.json")).unwrap())
                .unwrap();
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
}
