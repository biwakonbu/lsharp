use crate::error_codes::driver_io_error;
use serde_json::{Map, Value, json};
use std::fs;
use std::path::{Path, PathBuf};

pub(crate) fn language_guide_markdown() -> &'static str {
    include_str!("../templates/lsharp-language-guide.md")
}

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
        .map_err(|e| driver_io_error(format!("{}: {}", claude_dir.display(), e)))?;
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
            .map_err(|e| driver_io_error(format!("{}: {}", settings_path.display(), e)))?;
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
        .map_err(|e| driver_io_error(format!("{}: {}", settings_path.display(), e)))?;
    Ok(())
}

fn install_skill(claude_dir: &Path, template_path: &Path) -> miette::Result<()> {
    let skill_dir = claude_dir.join("skills/lsharp-language-guide");
    fs::create_dir_all(&skill_dir)
        .map_err(|e| driver_io_error(format!("{}: {}", skill_dir.display(), e)))?;

    let template = fs::read_to_string(template_path)
        .map_err(|e| driver_io_error(format!("{}: {}", template_path.display(), e)))?;
    let skill_path = skill_dir.join("SKILL.md");
    fs::write(&skill_path, template)
        .map_err(|e| driver_io_error(format!("{}: {}", skill_path.display(), e)))?;
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
#[path = "claude_plugin_tests.rs"]
mod tests;
