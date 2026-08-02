fn project_context_tool(arguments: &Value) -> Result<Value, String> {
    let object = arguments
        .as_object()
        .ok_or_else(|| "lsharp_project_context の arguments は object が必要です".to_string())?;
    if let Some(unknown) = object.keys().find(|key| key.as_str() != "project_dir") {
        return Err(format!("lsharp_project_context の未知の引数: {unknown}"));
    }
    let project_dir = match arguments.get("project_dir") {
        None => project_dir_argument(arguments),
        Some(Value::String(project_dir)) if !project_dir.trim().is_empty() => {
            PathBuf::from(project_dir)
        }
        Some(_) => {
            return Err("lsharp_project_context の project_dir は文字列が必要です".to_string());
        }
    };
    validate_project_context_dependency_sources(&project_dir)?;
    let cfg = config::load_config(&project_dir);
    let mut dependencies = cfg
        .dependencies
        .iter()
        .map(|(name, spec)| dependency_summary(name, spec, &project_dir))
        .collect::<Vec<_>>();
    dependencies.sort_by(|left, right| left["name"].as_str().cmp(&right["name"].as_str()));

    Ok(json!({
        "project": {
            "name": cfg.project.name,
            "version": cfg.project.version,
            "description": cfg.project.description,
            "exports": cfg.project.exports.modules,
        },
        "dependencies": dependencies,
        "installedPackages": installed_packages(&project_dir),
    }))
}

fn validate_project_context_dependency_sources(project_dir: &Path) -> Result<(), String> {
    let config_path = project_dir.join("lsharp.toml");
    let Ok(content) = std::fs::read_to_string(&config_path) else {
        return Ok(());
    };
    let value: toml::Value = toml::from_str(&content).map_err(|error| {
        format!(
            "lsharp_project_context の lsharp.toml が不正です: {}: {error}",
            config_path.display()
        )
    })?;
    let Some(dependencies) = value.get("dependencies") else {
        return Ok(());
    };
    let dependencies = dependencies
        .as_table()
        .ok_or_else(|| "lsharp_project_context の dependencies は table が必要です".to_string())?;
    for (name, spec) in dependencies {
        let Some(spec) = spec.as_table() else {
            let version = spec.as_str().ok_or_else(|| {
                format!("dependencies.{name} は version 文字列または source table が必要です")
            })?;
            if version.trim().is_empty() {
                return Err(format!("dependencies.{name} の version は空にできません"));
            }
            continue;
        };
        let unknown = spec
            .keys()
            .filter(|key| !matches!(key.as_str(), "path" | "git" | "branch" | "tag"))
            .cloned()
            .collect::<Vec<_>>();
        if !unknown.is_empty() {
            return Err(format!(
                "dependencies.{name} に未知の依存元属性があります: {}",
                unknown.join(", ")
            ));
        }
        let has_path = spec.contains_key("path");
        let has_git = spec.contains_key("git");
        if has_path == has_git {
            return Err(format!(
                "dependencies.{name} は path または git の一つだけを指定してください"
            ));
        }
        if has_path {
            let path = spec
                .get("path")
                .and_then(toml::Value::as_str)
                .ok_or_else(|| format!("dependencies.{name}.path は空でない文字列が必要です"))?;
            if path.trim().is_empty() {
                return Err(format!(
                    "dependencies.{name}.path は空でない文字列が必要です"
                ));
            }
            if spec.contains_key("branch") || spec.contains_key("tag") {
                return Err(format!(
                    "dependencies.{name} の path には branch/tag を指定できません"
                ));
            }
        } else {
            let git = spec
                .get("git")
                .and_then(toml::Value::as_str)
                .ok_or_else(|| format!("dependencies.{name}.git は空でない文字列が必要です"))?;
            if git.trim().is_empty() {
                return Err(format!(
                    "dependencies.{name}.git は空でない文字列が必要です"
                ));
            }
            for key in ["branch", "tag"] {
                if let Some(value) = spec.get(key) {
                    let value = value
                        .as_str()
                        .ok_or_else(|| format!("dependencies.{name}.{key} は文字列が必要です"))?;
                    if value.trim().is_empty() {
                        return Err(format!("dependencies.{name}.{key} は空にできません"));
                    }
                }
            }
        }
    }
    Ok(())
}

fn package_api_tool(arguments: &Value) -> Result<Value, String> {
    let object = arguments
        .as_object()
        .ok_or_else(|| "lsharp_package_api の arguments は object が必要です".to_string())?;
    if let Some(unknown) = object
        .keys()
        .find(|key| !matches!(key.as_str(), "name" | "project_dir"))
    {
        return Err(format!("lsharp_package_api の未知の引数: {unknown}"));
    }
    let name = match arguments.get("name") {
        Some(Value::String(name)) if !name.trim().is_empty() => name,
        Some(Value::String(_)) => {
            return Err("lsharp_package_api の name は空でない文字列が必要です".to_string());
        }
        Some(_) => return Err("lsharp_package_api の name は文字列が必要です".to_string()),
        None => return Err("lsharp_package_api の name は必須です".to_string()),
    };
    let project_dir = match arguments.get("project_dir") {
        None => project_dir_argument(arguments),
        Some(Value::String(project_dir)) if !project_dir.trim().is_empty() => {
            PathBuf::from(project_dir)
        }
        Some(_) => {
            return Err("lsharp_package_api の project_dir は空でない文字列が必要です".to_string());
        }
    };
    let package_dir = find_installed_package_dir(&project_dir, name)
        .ok_or_else(|| format!("インストール済みパッケージ '{name}' が見つかりません"))?;
    read_or_generate_package_api(&package_dir)
}

fn stdlib_api_tool(arguments: &Value) -> Result<Value, String> {
    let object = arguments
        .as_object()
        .ok_or_else(|| "lsharp_stdlib_api の arguments は object が必要です".to_string())?;
    if let Some(unknown) = object.keys().find(|key| key.as_str() != "module") {
        return Err(format!("lsharp_stdlib_api の未知の引数: {unknown}"));
    }
    let target_module = match arguments.get("module") {
        None => None,
        Some(Value::String(module)) if !module.trim().is_empty() => Some(module.as_str()),
        Some(Value::String(_)) => {
            return Err("lsharp_stdlib_api の module は空でない文字列が必要です".to_string());
        }
        Some(_) => return Err("lsharp_stdlib_api の module は文字列が必要です".to_string()),
    };
    let stdlib_root = stdlib_root().ok_or_else(|| "stdlib が見つかりません".to_string())?;
    let package = "stdlib";
    let version = env!("CARGO_PKG_VERSION");
    let mut modules = Vec::new();
    let entries =
        std::fs::read_dir(&stdlib_root).map_err(|e| mcp_io_error(stdlib_root.display(), e))?;
    for entry in entries {
        let entry = entry.map_err(|e| mcp_io_error(stdlib_root.display(), e))?;
        let path = entry.path();
        if path.extension().and_then(|ext| ext.to_str()) != Some("ls") {
            continue;
        }
        let doc =
            api_doc::build_api_doc_for_file(package, version, &path).map_err(|e| e.to_string())?;
        let mut doc_modules = doc.modules;
        if let Some(module) = doc_modules.pop()
            && target_module.is_none_or(|target| target == module.name)
        {
            modules.push(module);
        }
    }
    modules.sort_by(|left, right| left.name.cmp(&right.name));

    Ok(json!({
        "package": package,
        "version": version,
        "modules": modules,
    }))
}

fn project_dir_argument(arguments: &Value) -> PathBuf {
    arguments
        .get("project_dir")
        .and_then(Value::as_str)
        .map(PathBuf::from)
        .unwrap_or_else(|| std::env::current_dir().unwrap_or_else(|_| PathBuf::from(".")))
}

fn dependency_summary(name: &str, spec: &config::DependencySpec, project_dir: &Path) -> Value {
    match spec {
        config::DependencySpec::Version(version) => json!({
            "name": name,
            "version": version,
            "source": "registry"
        }),
        config::DependencySpec::Path { path } => json!({
            "name": name,
            "source": "path",
            "path": project_dir.join(path).display().to_string()
        }),
        config::DependencySpec::Git { git, branch, tag } => json!({
            "name": name,
            "source": "git",
            "git": git,
            "branch": branch,
            "tag": tag
        }),
    }
}

fn installed_packages(project_dir: &Path) -> Vec<Value> {
    let packages_dir = project_dir.join(".lsharp").join("packages");
    let Ok(entries) = std::fs::read_dir(packages_dir) else {
        return Vec::new();
    };
    let mut packages = Vec::new();
    for entry in entries.flatten() {
        let path = entry.path();
        if !path.is_dir() {
            continue;
        }
        let cfg = config::load_config(&path);
        let name = if cfg.project.name.is_empty() {
            path.file_name()
                .and_then(|name| name.to_str())
                .unwrap_or("package")
                .to_string()
        } else {
            cfg.project.name
        };
        packages.push(json!({
            "name": name,
            "version": cfg.project.version,
            "path": path.display().to_string()
        }));
    }
    packages.sort_by(|left, right| {
        left["name"]
            .as_str()
            .cmp(&right["name"].as_str())
            .then_with(|| left["path"].as_str().cmp(&right["path"].as_str()))
    });
    packages
}

fn list_module_candidates(project_dir: &Path) -> Vec<String> {
    let mut modules = Vec::new();
    for package_dir in installed_package_dirs(project_dir) {
        let api_path = package_dir.join("docs").join("api.json");
        if let Ok(content) = std::fs::read_to_string(&api_path)
            && let Ok(value) = serde_json::from_str::<Value>(&content)
            && let Some(items) = value.get("modules").and_then(Value::as_array)
        {
            for item in items {
                if let Some(name) = item.get("name").and_then(Value::as_str) {
                    modules.push(name.to_string());
                }
            }
        }
    }
    modules.sort();
    modules.dedup();
    modules
}

fn installed_package_dirs(project_dir: &Path) -> Vec<PathBuf> {
    let packages_dir = project_dir.join(".lsharp").join("packages");
    let Ok(entries) = std::fs::read_dir(packages_dir) else {
        return Vec::new();
    };
    let mut paths: Vec<PathBuf> = entries
        .flatten()
        .map(|entry| entry.path())
        .filter(|path| path.is_dir())
        .collect();
    paths.sort();
    paths
}

fn find_installed_package_dir(project_dir: &Path, name: &str) -> Option<PathBuf> {
    installed_package_dirs(project_dir)
        .into_iter()
        .find(|path| {
            path.file_name()
                .and_then(|entry| entry.to_str())
                .is_some_and(|entry| entry.starts_with(&format!("{name}-")))
        })
}

fn read_or_generate_package_api(package_dir: &Path) -> Result<Value, String> {
    let api_path = package_dir.join("docs").join("api.json");
    let cfg = config::load_config(package_dir);
    let package = if cfg.project.name.is_empty() {
        package_dir
            .file_name()
            .and_then(|name| name.to_str())
            .unwrap_or("package")
            .to_string()
    } else {
        cfg.project.name.clone()
    };
    let version = if cfg.project.version.is_empty() {
        "0.1.0".to_string()
    } else {
        cfg.project.version.clone()
    };
    let expected_identity =
        (!cfg.project.name.is_empty()).then_some((package.as_str(), version.as_str()));
    if api_path.exists() {
        let content =
            std::fs::read_to_string(&api_path).map_err(|e| mcp_io_error(api_path.display(), e))?;
        let value: Value =
            serde_json::from_str(&content).map_err(|e| format!("{}: {e}", api_path.display()))?;
        if !value.is_object() {
            return Err(format!(
                "{}: api.json の root は object が必要です",
                api_path.display()
            ));
        }
        validate_package_api_value(&value, &api_path)?;
        validate_package_api_identity(&value, &api_path, expected_identity)?;
        return Ok(value);
    }

    let api = api_doc::build_api_doc_for_package(package_dir, &package, &version)
        .map_err(|e| e.to_string())?;
    let value = serde_json::to_value(api).map_err(|e| e.to_string())?;
    validate_package_api_value(&value, &api_path)?;
    validate_package_api_identity(&value, &api_path, expected_identity)?;
    Ok(value)
}

fn validate_package_api_identity(
    value: &Value,
    api_path: &Path,
    expected_identity: Option<(&str, &str)>,
) -> Result<(), String> {
    let Some((expected_package, expected_version)) = expected_identity else {
        return Ok(());
    };
    let actual_package = value["package"].as_str().unwrap_or_default();
    let actual_version = value["version"].as_str().unwrap_or_default();
    if actual_package != expected_package || actual_version != expected_version {
        return Err(format!(
            "{}: api.json identity mismatch: expected package '{}' version '{}'",
            api_path.display(),
            expected_package,
            expected_version
        ));
    }
    Ok(())
}

fn validate_package_api_value(value: &Value, api_path: &Path) -> Result<(), String> {
    let root = package_api_object(value, api_path, "root", &["package", "version", "modules"])?;
    package_api_non_empty_string(&root["package"], api_path, "package")?;
    package_api_non_empty_string(&root["version"], api_path, "version")?;
    let modules = package_api_array(&root["modules"], api_path, "modules")?;

    for (module_index, module_value) in modules.iter().enumerate() {
        let module_path = format!("modules[{module_index}]");
        let module = package_api_object(
            module_value,
            api_path,
            &module_path,
            &["name", "doc", "functions", "types"],
        )?;
        package_api_non_empty_string(&module["name"], api_path, &format!("{module_path}.name"))?;
        package_api_nullable_string(&module["doc"], api_path, &format!("{module_path}.doc"))?;
        let functions = package_api_array(
            &module["functions"],
            api_path,
            &format!("{module_path}.functions"),
        )?;
        let types = package_api_array(&module["types"], api_path, &format!("{module_path}.types"))?;

        for (function_index, function_value) in functions.iter().enumerate() {
            let function_path = format!("{module_path}.functions[{function_index}]");
            let function = package_api_object(
                function_value,
                api_path,
                &function_path,
                &["name", "signature", "params", "returns", "doc", "example"],
            )?;
            package_api_non_empty_string(
                &function["name"],
                api_path,
                &format!("{function_path}.name"),
            )?;
            package_api_non_empty_string(
                &function["signature"],
                api_path,
                &format!("{function_path}.signature"),
            )?;
            package_api_nullable_string(
                &function["doc"],
                api_path,
                &format!("{function_path}.doc"),
            )?;
            package_api_nullable_string(
                &function["example"],
                api_path,
                &format!("{function_path}.example"),
            )?;
            let params = package_api_array(
                &function["params"],
                api_path,
                &format!("{function_path}.params"),
            )?;
            let returns_path = format!("{function_path}.returns");
            let returns = package_api_object(
                &function["returns"],
                api_path,
                &returns_path,
                &["type", "doc"],
            )?;
            package_api_non_empty_string(
                &returns["type"],
                api_path,
                &format!("{returns_path}.type"),
            )?;
            package_api_nullable_string(&returns["doc"], api_path, &format!("{returns_path}.doc"))?;

            for (param_index, param_value) in params.iter().enumerate() {
                let param_path = format!("{function_path}.params[{param_index}]");
                let param = package_api_object(
                    param_value,
                    api_path,
                    &param_path,
                    &["name", "type", "doc"],
                )?;
                package_api_non_empty_string(
                    &param["name"],
                    api_path,
                    &format!("{param_path}.name"),
                )?;
                package_api_non_empty_string(
                    &param["type"],
                    api_path,
                    &format!("{param_path}.type"),
                )?;
                package_api_nullable_string(&param["doc"], api_path, &format!("{param_path}.doc"))?;
            }
        }

        for (type_index, type_value) in types.iter().enumerate() {
            let type_path = format!("{module_path}.types[{type_index}]");
            let type_info =
                package_api_object(type_value, api_path, &type_path, &["name", "kind"])?;
            package_api_non_empty_string(
                &type_info["name"],
                api_path,
                &format!("{type_path}.name"),
            )?;
            package_api_non_empty_string(
                &type_info["kind"],
                api_path,
                &format!("{type_path}.kind"),
            )?;
        }
    }
    Ok(())
}

fn package_api_object<'a>(
    value: &'a Value,
    api_path: &Path,
    path: &str,
    fields: &[&str],
) -> Result<&'a serde_json::Map<String, Value>, String> {
    let object = value
        .as_object()
        .ok_or_else(|| package_api_error(api_path, path, "は object が必要です"))?;
    if let Some(unknown) = object
        .keys()
        .filter(|key| !fields.contains(&key.as_str()))
        .min()
    {
        return Err(package_api_error(
            api_path,
            &format!("{path}.{unknown}"),
            "は未知のフィールドです",
        ));
    }
    if let Some(missing) = fields.iter().find(|field| !object.contains_key(**field)) {
        return Err(package_api_error(
            api_path,
            &format!("{path}.{missing}"),
            "は必須です",
        ));
    }
    Ok(object)
}

fn package_api_array<'a>(
    value: &'a Value,
    api_path: &Path,
    path: &str,
) -> Result<&'a Vec<Value>, String> {
    value
        .as_array()
        .ok_or_else(|| package_api_error(api_path, path, "は配列が必要です"))
}

fn package_api_non_empty_string(value: &Value, api_path: &Path, path: &str) -> Result<(), String> {
    if value.as_str().is_some_and(|value| !value.trim().is_empty()) {
        Ok(())
    } else {
        Err(package_api_error(
            api_path,
            path,
            "は空でない文字列が必要です",
        ))
    }
}

fn package_api_nullable_string(value: &Value, api_path: &Path, path: &str) -> Result<(), String> {
    if value.is_null() || value.is_string() {
        Ok(())
    } else {
        Err(package_api_error(
            api_path,
            path,
            "は文字列または null が必要です",
        ))
    }
}

fn package_api_error(api_path: &Path, path: &str, message: &str) -> String {
    format!("{}: api.json の {path} {message}", api_path.display())
}

fn stdlib_root() -> Option<PathBuf> {
    if let Some(path) = std::env::var_os("LSHARP_STDLIB_PATH") {
        let path = PathBuf::from(path);
        if path.exists() {
            return Some(path);
        }
    }
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../stdlib");
    if path.exists() { Some(path) } else { None }
}
