fn project_context_tool(arguments: &Value) -> Result<Value, String> {
    let project_dir = project_dir_argument(arguments);
    let cfg = config::load_config(&project_dir);
    let dependencies = cfg
        .dependencies
        .iter()
        .map(|(name, spec)| dependency_summary(name, spec, &project_dir))
        .collect::<Vec<_>>();

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

fn package_api_tool(arguments: &Value) -> Result<Value, String> {
    let name = arguments
        .get("name")
        .and_then(Value::as_str)
        .ok_or_else(|| "name が必要です".to_string())?;
    let project_dir = project_dir_argument(arguments);
    let package_dir = find_installed_package_dir(&project_dir, name)
        .ok_or_else(|| format!("インストール済みパッケージ '{name}' が見つかりません"))?;
    read_or_generate_package_api(&package_dir)
}

fn stdlib_api_tool(arguments: &Value) -> Result<Value, String> {
    let stdlib_root = stdlib_root().ok_or_else(|| "stdlib が見つかりません".to_string())?;
    let package = "stdlib";
    let version = env!("CARGO_PKG_VERSION");
    let mut modules = Vec::new();
    let target_module = arguments.get("module").and_then(Value::as_str);

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
        if !path.is_dir() && path.symlink_metadata().is_err() {
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
    let mut paths: Vec<PathBuf> = entries.flatten().map(|entry| entry.path()).collect();
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
    if api_path.exists() {
        let content =
            std::fs::read_to_string(&api_path).map_err(|e| mcp_io_error(api_path.display(), e))?;
        return serde_json::from_str(&content).map_err(|e| format!("{}: {e}", api_path.display()));
    }

    let cfg = config::load_config(package_dir);
    let package = if cfg.project.name.is_empty() {
        package_dir
            .file_name()
            .and_then(|name| name.to_str())
            .unwrap_or("package")
            .to_string()
    } else {
        cfg.project.name
    };
    let version = if cfg.project.version.is_empty() {
        "0.1.0".to_string()
    } else {
        cfg.project.version
    };
    let api = api_doc::build_api_doc_for_package(package_dir, &package, &version)
        .map_err(|e| e.to_string())?;
    serde_json::to_value(api).map_err(|e| e.to_string())
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
