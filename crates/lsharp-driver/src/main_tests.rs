/// Knowledge JSON を構築
#[allow(dead_code)]
#[cfg(test)]
fn build_knowledge(
    program: &lsharp_syntax::ast::Program,
    type_results: &[(String, lsharp_types::types::TypeScheme)],
    infer: &lsharp_types::infer::Infer,
) -> lsharp_docs::knowledge::Knowledge {
    use lsharp_docs::knowledge::*;
    use lsharp_syntax::ast::Decl;

    let module_name = infer.module_env.name.clone();

    let mut functions = Vec::new();
    let mut types = Vec::new();
    let is_private_set: std::collections::HashSet<&str> = infer
        .module_env
        .privates
        .iter()
        .map(|s| s.as_str())
        .collect();

    // Private 名の収集のために再利用
    let _ = &is_private_set;

    for decl in &program.decls {
        // Private を展開
        let (actual_decl, is_priv) = match decl {
            Decl::Private { inner, .. } => (inner.as_ref(), true),
            other => (other, false),
        };

        match actual_decl {
            Decl::Defn {
                name,
                params,
                metadata,
                ..
            } => {
                let type_str = type_results
                    .iter()
                    .find(|(n, _)| n == name)
                    .map(|(_, s)| format!("{}", s.ty))
                    .unwrap_or_else(|| "?".to_string());

                let param_infos: Vec<ParamInfo> = params
                    .iter()
                    .map(|p| {
                        let desc = metadata.as_ref().and_then(|m| {
                            m.params
                                .iter()
                                .find(|(n, _)| n == &p.name)
                                .map(|(_, d)| d.clone())
                        });
                        ParamInfo {
                            name: p.name.clone(),
                            ty: p
                                .ty
                                .as_ref()
                                .map(|t| format!("{t}"))
                                .unwrap_or_else(|| "?".to_string()),
                            description: desc,
                        }
                    })
                    .collect();

                functions.push(FunctionInfo {
                    name: name.clone(),
                    params: param_infos,
                    return_type: type_str,
                    doc: metadata.as_ref().and_then(|m| m.doc.clone()),
                    module: module_name.clone(),
                    is_private: is_priv || is_private_set.contains(name.as_str()),
                });
            }
            Decl::RecordDef { name, fields, .. } => {
                let field_infos: Vec<FieldInfo> = fields
                    .iter()
                    .map(|(fname, ftype)| FieldInfo {
                        name: fname.clone(),
                        ty: format!("{ftype}"),
                    })
                    .collect();
                types.push(TypeInfo {
                    name: name.clone(),
                    kind: TypeKind::Record {
                        fields: field_infos,
                    },
                    type_params: vec![],
                    doc: None,
                });
            }
            Decl::TypeDef { name, variants, .. } => {
                let variant_infos: Vec<VariantInfo> = variants
                    .iter()
                    .map(|v| VariantInfo {
                        name: v.name.clone(),
                        fields: v.fields.iter().map(|f| format!("{f}")).collect(),
                    })
                    .collect();
                types.push(TypeInfo {
                    name: name.clone(),
                    kind: TypeKind::Adt {
                        variants: variant_infos,
                    },
                    type_params: vec![],
                    doc: None,
                });
            }
            Decl::TypeAlias { name, target, .. } => {
                types.push(TypeInfo {
                    name: name.clone(),
                    kind: TypeKind::Alias {
                        target: format!("{target}"),
                    },
                    type_params: vec![],
                    doc: None,
                });
            }
            Decl::TraitDef { name, methods, .. } => {
                let method_names: Vec<String> = methods.iter().map(|m| m.name.clone()).collect();
                types.push(TypeInfo {
                    name: name.clone(),
                    kind: TypeKind::Trait {
                        methods: method_names,
                    },
                    type_params: vec![],
                    doc: None,
                });
            }
            _ => {}
        }
    }

    // 依存関係
    let dependencies: Vec<DependencyInfo> = infer
        .module_env
        .imports
        .iter()
        .map(|imp| {
            let kind = if imp.open {
                DependencyKind::OpenImport
            } else if let Some(ref only) = imp.only {
                DependencyKind::SelectiveImport {
                    symbols: only.clone(),
                }
            } else {
                DependencyKind::Import
            };
            DependencyInfo {
                from: module_name.clone().unwrap_or_else(|| "main".to_string()),
                to: imp.module.clone(),
                kind,
            }
        })
        .collect();

    Knowledge {
        project: ProjectInfo {
            name: module_name.unwrap_or_else(|| "unnamed".to_string()),
            version: "0.1.0".to_string(),
        },
        functions,
        types,
        dependencies,
    }
}

#[cfg(test)]
#[derive(Debug, Clone, Default)]
struct ImportVisibilitySpec {
    only: Option<Vec<String>>,
}

#[cfg(test)]
#[derive(Debug, Clone, Default)]
struct ResolvedImportModule {
    results: Vec<(String, lsharp_types::types::TypeScheme)>,
    hidden: std::collections::HashSet<String>,
}

#[cfg(test)]
fn collect_import_visibility(
    program: &lsharp_syntax::ast::Program,
) -> std::collections::HashMap<String, ImportVisibilitySpec> {
    let mut imports = std::collections::HashMap::new();
    for decl in &program.decls {
        if let lsharp_syntax::ast::Decl::ImportDecl { module, only, .. } = decl {
            let entry = imports
                .entry(module.clone())
                .or_insert_with(ImportVisibilitySpec::default);
            match (&mut entry.only, only.as_ref()) {
                (None, None) => {}
                (slot @ None, Some(next)) => {
                    *slot = Some(next.clone());
                }
                (Some(existing), Some(next)) => {
                    for symbol in next {
                        if !existing.contains(symbol) {
                            existing.push(symbol.clone());
                        }
                    }
                }
                (Some(_), None) => {
                    entry.only = None;
                }
            }
        }
    }
    imports
}

#[cfg(test)]
fn declared_module_name(
    program: &lsharp_syntax::ast::Program,
    fallback_file: &std::path::Path,
) -> String {
    program
        .decls
        .iter()
        .find_map(|decl| {
            if let lsharp_syntax::ast::Decl::ModuleDecl { name, .. } = decl {
                Some(name.clone())
            } else {
                None
            }
        })
        .or_else(|| {
            fallback_file
                .file_stem()
                .and_then(|stem| stem.to_str())
                .map(ToString::to_string)
        })
        .unwrap_or_else(|| "Main".to_string())
}

#[cfg(test)]
fn resolve_import_module_recursive(
    module: &str,
    from_module: &str,
    search_paths: &lsharp_ir::module_graph::ModuleSearchPaths,
    cache: &mut std::collections::HashMap<String, ResolvedImportModule>,
    resolving: &mut std::collections::HashSet<String>,
) -> miette::Result<ResolvedImportModule> {
    if let Some(cached) = cache.get(module) {
        return Ok(cached.clone());
    }
    if !resolving.insert(module.to_string()) {
        return Ok(cache.get(module).cloned().unwrap_or_default());
    }

    let result = (|| -> miette::Result<ResolvedImportModule> {
        let import_path = lsharp_ir::module_graph::ModuleGraph::resolve_module_import_path(
            module,
            from_module,
            search_paths,
        )
        .map_err(|e| miette::miette!("{e}"))?
        .ok_or_else(|| {
            miette::miette!(
                "モジュール '{}' が見つかりません ('{}' からインポート)",
                module,
                from_module
            )
        })?;

        let import_source = std::fs::read_to_string(&import_path)
            .map_err(|e| driver_io_error(format!("{}: {}", import_path.display(), e)))?;
        let import_program = lsharp_syntax::parse(&import_source)
            .map_err(|e| miette::miette!("{}: {e}", import_path.display()))?;
        let import_module_name = declared_module_name(&import_program, &import_path);

        let mut import_infer = lsharp_types::infer::Infer::new();
        for (dependency, spec) in collect_import_visibility(&import_program) {
            let dependency_surface = resolve_import_module_recursive(
                &dependency,
                &import_module_name,
                search_paths,
                cache,
                resolving,
            )?;
            import_infer.inject_external_types_for_import(
                &dependency,
                spec.only.as_deref(),
                &dependency_surface.hidden,
                &dependency_surface.results,
            );
        }

        let import_results = import_infer
            .infer_program(&import_program)
            .map_err(|e| miette::miette!("{}: {e}", import_path.display()))?;
        Ok(ResolvedImportModule {
            results: import_results,
            hidden: import_infer.module_env.privates.iter().cloned().collect(),
        })
    })();

    resolving.remove(module);
    if let Ok(surface) = &result {
        cache.insert(module.to_string(), surface.clone());
    }
    result
}

/// 型チェック内部用: import 宣言を再帰的に解決する
///
/// package root / src / .lsharp/packages / stdlib の探索順に従って import を解決し、
/// 各 import ごとに `:only` / `private` / package exports を反映した型環境だけを注入する。
#[cfg(test)]
fn resolve_imports_recursive(
    program: &lsharp_syntax::ast::Program,
    entry_file: &std::path::Path,
    infer: &mut lsharp_types::infer::Infer,
    resolved: &mut std::collections::HashSet<String>,
) -> miette::Result<()> {
    let search_paths = lsharp_ir::module_graph::ModuleSearchPaths::discover(entry_file);
    let current_module = declared_module_name(program, entry_file);
    let mut cache = std::collections::HashMap::new();

    for (module, spec) in collect_import_visibility(program) {
        let imported = resolve_import_module_recursive(
            &module,
            &current_module,
            &search_paths,
            &mut cache,
            resolved,
        )?;
        infer.inject_external_types_for_import(
            &module,
            spec.only.as_deref(),
            &imported.hidden,
            &imported.results,
        );
    }

    Ok(())
}

/// git clone コマンドの引数を構築する (テスト用)
#[cfg(test)]
fn build_git_clone_args<'a>(
    url: &'a str,
    branch: Option<&'a str>,
    tag: Option<&'a str>,
    dest: &'a str,
) -> Vec<&'a str> {
    let mut args = vec!["clone", "--depth", "1"];
    let ref_spec = branch.or(tag);
    if let Some(r) = ref_spec {
        args.push("--branch");
        args.push(r);
    }
    args.push(url);
    args.push(dest);
    args
}

use super::*;
use clap::CommandFactory;

#[test]
fn test_guest_compile_success_does_not_request_host_fallback() {
    assert!(!should_fallback_to_host_compile(Some(0)));
    assert!(should_fallback_to_host_compile(Some(1)));
    assert!(should_fallback_to_host_compile(None));
}

#[test]
fn test_test_command_is_rust_native_metadata_command() {
    assert!(!is_selfhost_shadow_command("test"));
    assert!(!is_selfhost_shadow_command("compile"));
}

fn command_names_from_help(help: &str) -> Vec<&str> {
    help.lines()
        .filter_map(|line| line.strip_prefix("  "))
        .filter_map(|line| {
            let name = line.split_whitespace().next()?;
            (!name.starts_with('-')).then_some(name)
        })
        .collect()
}

fn os_args(args: &[&str]) -> Vec<std::ffi::OsString> {
    args.iter().map(std::ffi::OsString::from).collect()
}

fn dot_prefixed(path: &str) -> String {
    Path::new(".").join(path).to_string_lossy().into_owned()
}

#[test]
fn test_cli_help_excludes_removed_parse_check_fmt_subcommands() {
    let help = Cli::command().render_long_help().to_string();
    let commands = command_names_from_help(&help);

    assert!(commands.contains(&"compile"));
    assert!(commands.contains(&"language-guide"));
    assert!(!commands.contains(&"parse"));
    assert!(!commands.contains(&"check"));
    assert!(!commands.contains(&"fmt"));
}

#[test]
fn test_cli_try_parse_from_rejects_removed_parse_check_fmt_subcommands() {
    for subcommand in ["parse", "check", "fmt"] {
        let err = match Cli::try_parse_from(["lsharp", subcommand, "examples/fib.ls"]) {
            Ok(_) => panic!("旧 CLI サブコマンドは拒否されるべき: {subcommand}"),
            Err(err) => err,
        };

        assert_eq!(err.kind(), clap::error::ErrorKind::InvalidSubcommand);
        assert!(err.to_string().contains(subcommand));
    }
}

#[test]
fn test_cli_compile_target_accepts_wasi_component_alias_and_web_wasm() {
    let cli = Cli::try_parse_from([
        "lsharp",
        "compile",
        "examples/fib.ls",
        "--target",
        "wasi-component",
    ])
    .expect("wasi-component target should parse");
    let Command::Compile { target, .. } = cli.command else {
        panic!("compile subcommand should parse");
    };
    assert_eq!(target, Some(CliCompileTarget::WasiComponent));

    let cli = Cli::try_parse_from([
        "lsharp",
        "compile",
        "examples/fib.ls",
        "--target",
        "wasi-preview1",
    ])
    .expect("wasi-preview1 target should parse");
    let Command::Compile { target, .. } = cli.command else {
        panic!("compile subcommand should parse");
    };
    assert_eq!(target, Some(CliCompileTarget::WasiPreview1));

    let cli = Cli::try_parse_from(["lsharp", "compile", "examples/fib.ls", "--target", "wasm"])
        .expect("wasm alias should parse");
    let Command::Compile { target, .. } = cli.command else {
        panic!("compile subcommand should parse");
    };
    assert_eq!(target, Some(CliCompileTarget::WasiComponent));

    let cli = Cli::try_parse_from([
        "lsharp",
        "compile",
        "examples/fib.ls",
        "--target",
        "web-wasm",
    ])
    .expect("web-wasm target should parse");
    let Command::Compile { target, .. } = cli.command else {
        panic!("compile subcommand should parse");
    };
    assert_eq!(target, Some(CliCompileTarget::WebWasm));
}

#[test]
fn test_cli_compile_backend_accepts_wasmgc_with_web_wasm_target() {
    let cli = Cli::try_parse_from([
        "lsharp",
        "compile",
        "examples/fib.ls",
        "--backend",
        "wasmgc",
        "--target",
        "web-wasm",
    ])
    .expect("wasmgc backend should parse");
    let Command::Compile {
        backend, target, ..
    } = cli.command
    else {
        panic!("compile subcommand should parse");
    };
    assert_eq!(backend, Some(CliCompileBackend::WasmGc));
    assert_eq!(target, Some(CliCompileTarget::WebWasm));
}

#[test]
fn test_cli_compile_artifact_cache_dir_is_explicit() {
    let cli = Cli::try_parse_from([
        "lsharp",
        "compile",
        "examples/fib.ls",
        "--artifact-cache-dir",
        "tmp/lsharp-cache",
    ])
    .expect("artifact cache directory は明示指定できるべき");
    let Command::Compile {
        artifact_cache_dir, ..
    } = cli.command
    else {
        panic!("compile subcommand should parse");
    };
    assert_eq!(
        artifact_cache_dir,
        Some(PathBuf::from("tmp/lsharp-cache")),
        "cache root は CLI の明示指定だけで有効になるべき"
    );
}

#[test]
fn test_resolve_artifact_cache_dir_prefers_cli_over_environment() {
    let resolved = resolve_artifact_cache_dir_from_values(
        Some(PathBuf::from("cli-cache")),
        Some(std::ffi::OsString::from("env-cache")),
    )
    .expect("CLI cache root は解決できるべき");

    assert_eq!(resolved, Some(PathBuf::from("cli-cache")));
}

#[test]
fn test_resolve_artifact_cache_dir_uses_environment_when_cli_is_absent() {
    let resolved =
        resolve_artifact_cache_dir_from_values(None, Some(std::ffi::OsString::from("env-cache")))
            .expect("環境変数の cache root は解決できるべき");

    assert_eq!(resolved, Some(PathBuf::from("env-cache")));
}

#[test]
fn test_resolve_artifact_cache_dir_keeps_cache_disabled_when_unset() {
    let resolved = resolve_artifact_cache_dir_from_values(None, None)
        .expect("未設定の cache root はエラーにならないべき");

    assert_eq!(resolved, None);
}

#[test]
fn test_resolve_artifact_cache_dir_rejects_empty_environment_value() {
    let error = resolve_artifact_cache_dir_from_values(None, Some(std::ffi::OsString::new()))
        .expect_err("空の cache root は暗黙の current directory になってはいけない");

    assert!(error.to_string().contains("LSHARP_ARTIFACT_CACHE_DIR"));
}

#[test]
fn test_resolve_artifact_cache_limits_prefers_cli_over_environment() {
    let resolved = resolve_artifact_cache_limits_from_values(
        Some(2),
        Some(4096),
        Some(std::ffi::OsString::from("3")),
        Some(std::ffi::OsString::from("8192")),
    )
    .expect("CLI maintenance limit は解決できるべき");

    assert_eq!(resolved, (Some(2), Some(4096)));
}

#[test]
fn test_resolve_artifact_cache_limits_uses_environment_when_cli_is_absent() {
    let resolved = resolve_artifact_cache_limits_from_values(
        None,
        None,
        Some(std::ffi::OsString::from("3")),
        Some(std::ffi::OsString::from("8192")),
    )
    .expect("環境変数の maintenance limit は解決できるべき");

    assert_eq!(resolved, (Some(3), Some(8192)));
}

#[test]
fn test_resolve_artifact_cache_limits_keeps_limits_disabled_when_unset() {
    let resolved = resolve_artifact_cache_limits_from_values(None, None, None, None)
        .expect("未設定の maintenance limit はエラーにならないべき");

    assert_eq!(resolved, (None, None));
}

#[test]
fn test_resolve_artifact_cache_limits_rejects_invalid_entry_value() {
    let error = resolve_artifact_cache_limits_from_values(
        None,
        None,
        Some(std::ffi::OsString::from("many")),
        None,
    )
    .expect_err("不正な entry limit は暗黙に無視してはいけない");

    assert!(
        error
            .to_string()
            .contains("LSHARP_ARTIFACT_CACHE_MAX_ENTRIES")
    );
}

#[test]
fn test_resolve_artifact_cache_limits_rejects_empty_byte_value() {
    let error = resolve_artifact_cache_limits_from_values(
        None,
        None,
        None,
        Some(std::ffi::OsString::new()),
    )
    .expect_err("空の byte limit は暗黙に無視してはいけない");

    assert!(
        error
            .to_string()
            .contains("LSHARP_ARTIFACT_CACHE_MAX_BYTES")
    );
}

#[test]
fn test_resolve_artifact_cache_limits_accepts_zero_values() {
    let resolved = resolve_artifact_cache_limits_from_values(
        None,
        None,
        Some(std::ffi::OsString::from("0")),
        Some(std::ffi::OsString::from("0")),
    )
    .expect("zero の maintenance limit は明示値として解釈できるべき");

    assert_eq!(resolved, (Some(0), Some(0)));
}

#[test]
fn test_cli_compile_artifact_cache_max_entries_is_explicit() {
    let cli = Cli::try_parse_from([
        "lsharp",
        "compile",
        "examples/fib.ls",
        "--artifact-cache-max-entries",
        "3",
    ])
    .expect("artifact cache entry limit は parse できるべき");
    let Command::Compile {
        artifact_cache_max_entries,
        ..
    } = cli.command
    else {
        panic!("compile subcommand should parse");
    };
    assert_eq!(
        artifact_cache_max_entries,
        Some(3),
        "entry limit は明示した値を保持するべき"
    );
}

#[test]
fn test_cli_compile_artifact_cache_max_entries_requires_cache_root() {
    let error = validate_artifact_cache_options(None, Some(3), None)
        .expect_err("entry limit 単独は cache root がないため拒否するべき");
    assert!(error.to_string().contains("--artifact-cache-dir"));
    assert!(validate_artifact_cache_options(None, None, None).is_ok());
}

#[test]
fn test_cli_compile_artifact_cache_max_bytes_is_explicit() {
    let cli = Cli::try_parse_from([
        "lsharp",
        "build",
        "examples/fib.ls",
        "--artifact-cache-max-bytes",
        "4096",
    ])
    .expect("artifact cache byte budget は parse できるべき");
    let Command::Build {
        artifact_cache_max_bytes,
        ..
    } = cli.command
    else {
        panic!("build subcommand should parse");
    };
    assert_eq!(artifact_cache_max_bytes, Some(4096));
}

#[test]
fn test_cli_compile_artifact_cache_max_bytes_requires_cache_root() {
    let error = validate_artifact_cache_options(None, None, Some(4096))
        .expect_err("byte budget 単独は cache root がないため拒否するべき");
    assert!(error.to_string().contains("--artifact-cache-dir"));
    assert!(validate_artifact_cache_options(None, None, None).is_ok());
}

#[test]
fn test_maintain_artifact_cache_trims_explicit_root() {
    let dir = std::env::temp_dir().join(format!(
        "lsharp_driver_cache_maintenance_{}_{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .expect("system clock は unix epoch より後であるべき")
            .as_nanos()
    ));
    let source = dir.join("Main.ls");
    std::fs::create_dir_all(&dir).unwrap();
    std::fs::write(&source, "(module Main)\n(defn main [] 7)\n").unwrap();
    let key = lsharp_tooling::compile::CompileCacheKey::from_entry(
        &source,
        lsharp_tooling::compile::CompileTarget::WasiPreview1,
        lsharp_tooling::compile::CompileBackend::Linear,
    )
    .unwrap();
    let cache_root = dir.join("cache");
    let cache = lsharp_tooling::artifact_cache::ArtifactCache::new(&cache_root);
    cache.store(&key, b"artifact").unwrap();
    let second_source = dir.join("Second.ls");
    std::fs::write(&second_source, "(module Second)\n(defn main [] 8)\n").unwrap();
    let second_key = lsharp_tooling::compile::CompileCacheKey::from_entry(
        &second_source,
        lsharp_tooling::compile::CompileTarget::WasiPreview1,
        lsharp_tooling::compile::CompileBackend::Linear,
    )
    .unwrap();
    cache.store(&second_key, b"second-artifact").unwrap();

    assert_eq!(
        maintain_artifact_cache(Some(&cache_root), Some(1), None).unwrap(),
        1,
        "CLI maintenance は明示 root の artifact を上限まで削除するべき"
    );
    assert_eq!(
        std::fs::read_dir(cache_root.join("lsharp-compile-artifact-v1"))
            .unwrap()
            .count(),
        1
    );
    let remaining = std::fs::read_dir(cache_root.join("lsharp-compile-artifact-v1"))
        .unwrap()
        .find_map(Result::ok)
        .expect("entry limit 後に artifact が一つ残るべき")
        .path();
    let remaining_bytes = std::fs::metadata(&remaining).unwrap().len();
    assert_eq!(
        maintain_artifact_cache(
            Some(&cache_root),
            None,
            Some(remaining_bytes.saturating_sub(1)),
        )
        .unwrap(),
        1,
        "CLI byte maintenance は残存 artifact も byte budget で削除できるべき"
    );
    assert_eq!(
        std::fs::read_dir(cache_root.join("lsharp-compile-artifact-v1"))
            .unwrap()
            .filter_map(Result::ok)
            .filter(|entry| entry
                .path()
                .extension()
                .is_some_and(|ext| ext == "artifact"))
            .count(),
        0
    );
    std::fs::remove_dir_all(&dir).unwrap();
}

#[test]
fn test_should_delegate_to_embedded_component_args_accepts_compile_build_component_subset() {
    assert!(should_delegate_to_embedded_component_args(&os_args(&[
        "compile",
        "examples/fib.ls",
        "-o",
        "fib.component.wasm",
    ])));
    assert!(should_delegate_to_embedded_component_args(&os_args(&[
        "build",
        "examples/fib.ls",
        "--target",
        "wasi-preview1",
        "--output",
        "fib.wasm",
    ])));
    assert!(should_delegate_to_embedded_component_args(&os_args(&[
        "review",
        "examples/fib.ls",
    ])));
    assert!(should_delegate_to_embedded_component_args(&os_args(&[
        "review",
        "examples/fib.ls",
        "--json",
    ])));
    assert!(should_delegate_to_embedded_component_args(&os_args(&[
        "review",
        "examples/fib.ls",
        "--format",
        "json",
    ])));
    assert!(should_delegate_to_embedded_component_args(&os_args(&[
        "doc-ack",
        "examples/fib.ls",
    ])));
    assert!(should_delegate_to_embedded_component_args(&os_args(&[
        "doc-ack",
        "examples/fib.ls",
        "--trailer",
    ])));
    assert!(should_delegate_to_embedded_component_args(&os_args(&[
        "doc-check",
        "examples/fib.ls",
    ])));
    assert!(should_delegate_to_embedded_component_args(&os_args(&[
        "doc-check",
        "examples/fib.ls",
        "--strict",
    ])));
}

#[test]
fn test_json_metadata_test_stays_on_rust_driver_boundary() {
    assert!(should_delegate_to_embedded_component_args(&os_args(&[
        "test",
        "examples/fib.ls",
    ])));
    assert!(!should_delegate_to_embedded_component_args(&os_args(&[
        "test",
        "examples/fib.ls",
        "--format",
        "json",
    ])));
    assert!(!should_delegate_to_embedded_component_args(&os_args(&[
        "test",
        "examples/fib.ls",
        "--format=json",
    ])));
}

#[test]
fn test_embedded_component_delegation_rejects_environment_cache_root() {
    let args = os_args(&["compile", "examples/fib.ls"]);
    let cache_env = std::ffi::OsString::from("tmp/lsharp-cache");

    assert!(should_delegate_to_embedded_component_args_with_cache_env(
        &args, None, false
    ));
    assert!(!should_delegate_to_embedded_component_args_with_cache_env(
        &args,
        Some(cache_env.as_os_str()),
        false,
    ));
    assert!(!should_delegate_to_embedded_component_args_with_cache_env(
        &args, None, true,
    ));
}

#[test]
fn test_should_delegate_to_embedded_component_args_rejects_rust_only_compile_build_flags() {
    assert!(!should_delegate_to_embedded_component_args(&os_args(&[
        "compile", "--help",
    ])));
    assert!(!should_delegate_to_embedded_component_args(&os_args(&[
        "compile",
        "examples/fib.ls",
        "--emit-ir",
    ])));
    assert!(!should_delegate_to_embedded_component_args(&os_args(&[
        "compile",
        "examples/fib.ls",
        "--target",
        "web-wasm",
    ])));
    assert!(!should_delegate_to_embedded_component_args(&os_args(&[
        "compile",
        "examples/fib.ls",
        "--backend",
        "wasmgc",
        "--target",
        "web-wasm",
    ])));
    assert!(!should_delegate_to_embedded_component_args(&os_args(&[
        "compile",
        "examples/fib.ls",
        "--artifact-cache-dir",
        "tmp/lsharp-cache",
    ])));
    assert!(!should_delegate_to_embedded_component_args(&os_args(&[
        "compile",
        "examples/fib.ls",
        "--artifact-cache-max-entries",
        "3",
    ])));
    assert!(!should_delegate_to_embedded_component_args(&os_args(&[
        "build",
        "examples/fib.ls",
        "--artifact-cache-max-bytes",
        "4096",
    ])));
    assert!(!should_delegate_to_embedded_component_args(&os_args(&[
        "build",
        "examples/fib.ls",
        "--output",
        "--target",
    ])));
    assert!(!should_delegate_to_embedded_component_args(&os_args(&[
        "review", "--help",
    ])));
    assert!(!should_delegate_to_embedded_component_args(&os_args(&[
        "review",
        "examples/fib.ls",
        "--format",
        "yaml",
    ])));
    assert!(!should_delegate_to_embedded_component_args(&os_args(&[
        "review",
        "examples/fib.ls",
        "--json",
        "--format",
    ])));
    assert!(!should_delegate_to_embedded_component_args(&os_args(&[
        "doc-ack", "--help",
    ])));
    assert!(!should_delegate_to_embedded_component_args(&os_args(&[
        "doc-check",
        "--help",
    ])));
    assert!(!should_delegate_to_embedded_component_args(&os_args(&[
        "doc-ack",
        "examples/fib.ls",
        "--json",
    ])));
    assert!(!should_delegate_to_embedded_component_args(&os_args(&[
        "doc-check",
        "examples/fib.ls",
        "--format",
        "json",
    ])));
}

#[test]
fn test_normalize_guest_args_prefixes_relative_src_entry_paths() {
    let current_dir = std::env::temp_dir().join("lsharp_normalize_guest_args");
    let src_entry = dot_prefixed("src/Main.ls");

    let compile_args = normalize_guest_args_for_current_dir(
        &current_dir,
        vec!["compile".to_string(), "src/Main.ls".to_string()],
    );
    assert_eq!(compile_args, vec!["compile".to_string(), src_entry.clone()]);

    let review_args = normalize_guest_args_for_current_dir(
        &current_dir,
        vec!["review".to_string(), "src/Main.ls".to_string()],
    );
    assert_eq!(review_args, vec!["review".to_string(), src_entry]);
}

#[test]
fn test_normalize_guest_args_relativizes_absolute_src_entry_paths_with_dot_prefix() {
    let current_dir = std::env::temp_dir().join("lsharp_normalize_guest_args_abs");
    let src_entry = current_dir.join("src/Main.ls");
    let expected = dot_prefixed("src/Main.ls");

    let compile_args = normalize_guest_args_for_current_dir(
        &current_dir,
        vec![
            "compile".to_string(),
            src_entry.to_string_lossy().into_owned(),
            "--output".to_string(),
            current_dir
                .join("src/Main.component.wasm")
                .to_string_lossy()
                .into_owned(),
        ],
    );

    assert_eq!(compile_args[1], expected);
    assert_eq!(compile_args[3], dot_prefixed("src/Main.component.wasm"));
}

#[test]
fn test_normalize_guest_args_keeps_non_src_relative_paths_unchanged() {
    let current_dir = std::env::temp_dir().join("lsharp_normalize_guest_args_examples");
    let compile_args = normalize_guest_args_for_current_dir(
        &current_dir,
        vec!["compile".to_string(), "examples/fib.ls".to_string()],
    );

    assert_eq!(
        compile_args,
        vec!["compile".to_string(), "examples/fib.ls".to_string()]
    );
}

#[test]
fn test_cmd_install_no_dependencies() {
    // lsharp.toml がないディレクトリではデフォルト設定 (依存なし) で成功する
    let dir = std::env::temp_dir().join("lsharp_test_install_no_deps");
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();

    let result = cmd_install_in(&dir);
    assert!(result.is_ok(), "依存なしで cmd_install_in は成功するべき");

    std::fs::remove_dir_all(&dir).unwrap();
}

#[test]
fn test_cmd_install_missing_toml_uses_defaults() {
    // lsharp.toml が存在しないディレクトリでもデフォルト設定で動作する
    let dir = std::env::temp_dir().join("lsharp_test_install_missing_toml");
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();

    // lsharp.toml を作成しない
    let result = cmd_install_in(&dir);
    assert!(
        result.is_ok(),
        "lsharp.toml がなくてもデフォルトで成功するべき"
    );

    std::fs::remove_dir_all(&dir).unwrap();
}

#[test]
fn test_cmd_install_creation_failure_preserves_driver_io_error_code() {
    let project_file = std::env::temp_dir().join(format!(
        "lsharp_test_install_creation_error_{}",
        std::process::id()
    ));
    let _ = std::fs::remove_dir_all(&project_file);
    let _ = std::fs::remove_file(&project_file);
    std::fs::write(&project_file, "not a directory").unwrap();

    let error = cmd_install_in(&project_file)
        .expect_err("project path が file の install は package directory を作れず失敗するべき");

    assert!(
        error.to_string().starts_with("[LS5001]"),
        "install の package directory I/O 失敗は driver I/O code を保持するべき: {error}"
    );

    std::fs::remove_file(&project_file).unwrap();
}

#[test]
fn test_driver_path_canonicalize_failure_preserves_driver_io_error_code() {
    let base_file = std::env::temp_dir().join("lsharp_driver_canonicalize_failure");
    let _ = std::fs::remove_dir_all(&base_file);
    let _ = std::fs::remove_file(&base_file);
    std::fs::write(&base_file, "not a directory").unwrap();
    let blocked_path = base_file.join("child");

    let error = canonicalize_driver_path(&blocked_path)
        .expect_err("ファイル配下の canonicalize は失敗するべき");

    assert!(
        error.to_string().starts_with("[LS5001]"),
        "driver I/O 診断コードを保持するべき: {error:?}"
    );

    std::fs::remove_file(&base_file).unwrap();
}

#[test]
fn test_cmd_install_config_read_failure_preserves_driver_io_error_code() {
    let project_dir = std::env::temp_dir().join(format!(
        "lsharp_test_install_config_read_error_{}",
        std::process::id()
    ));
    let _ = std::fs::remove_dir_all(&project_dir);
    std::fs::create_dir_all(project_dir.join("lsharp.toml")).unwrap();

    let error = cmd_install_in(&project_dir)
        .expect_err("directory になっている lsharp.toml は install の設定読み込みに失敗するべき");

    assert!(
        error.to_string().starts_with("[LS5001]"),
        "config read failure は driver I/O code を保持するべき: {error}"
    );

    std::fs::remove_dir_all(&project_dir).unwrap();
}

#[test]
fn test_cmd_init_creation_failure_preserves_driver_io_error_code() {
    let base_file = std::env::temp_dir().join(format!(
        "lsharp_test_init_creation_error_{}",
        std::process::id()
    ));
    let _ = std::fs::remove_dir_all(&base_file);
    let _ = std::fs::remove_file(&base_file);
    std::fs::write(&base_file, "not a directory").unwrap();

    let error = cmd_init_in(&base_file, "demo")
        .expect_err("親 path が file の init は project layout を作れず失敗するべき");

    assert!(
        error.to_string().starts_with("[LS5001]"),
        "init の project layout I/O 失敗は driver I/O code を保持するべき: {error}"
    );

    std::fs::remove_file(&base_file).unwrap();
}

#[test]
fn test_cmd_doc_json_writes_docs_api_json() {
    let dir = std::env::temp_dir().join("lsharp_test_doc_json");
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(dir.join("src")).unwrap();
    std::fs::write(
        dir.join("lsharp.toml"),
        "[project]\nname = \"demo\"\nversion = \"0.2.0\"\n",
    )
    .unwrap();
    std::fs::write(
        dir.join("src/Geometry.ls"),
        "(module Geometry)\n(defn add [x y] (+ x y))",
    )
    .unwrap();

    let result = cmd_doc_json(&dir.join("src/Geometry.ls"), None);
    assert!(result.is_ok(), "doc --json は成功するべき: {result:?}");

    let api_path = dir.join("docs").join("api.json");
    let content = std::fs::read_to_string(&api_path).unwrap();
    assert!(content.contains("\"package\": \"demo\""));
    assert!(content.contains("\"version\": \"0.2.0\""));
    assert!(content.contains("\"name\": \"Geometry\""));

    std::fs::remove_dir_all(&dir).unwrap();
}

#[test]
fn test_cmd_doc_json_output_write_preserves_driver_io_error_code() {
    let dir = std::env::temp_dir().join(format!(
        "lsharp_test_doc_json_output_error_{}",
        std::process::id()
    ));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(dir.join("src")).unwrap();
    std::fs::write(
        dir.join("src/Main.ls"),
        "(module Main)\n(defn main [] 42)\n",
    )
    .unwrap();
    let blocked_parent = dir.join("blocked");
    std::fs::write(&blocked_parent, "not a directory").unwrap();

    let error = cmd_doc_json(
        &dir.join("src/Main.ls"),
        Some(&blocked_parent.join("api.json")),
    )
    .expect_err("書き込み先の親が file の doc --json は失敗するべき");

    assert!(
        error.to_string().starts_with("[LS5001]"),
        "doc --json の artifact I/O 失敗は driver I/O code を保持するべき: {error}"
    );

    std::fs::remove_dir_all(&dir).unwrap();
}

#[test]
fn test_cmd_test_succeeds_for_metadata_fixture() {
    let dir = std::env::temp_dir().join("lsharp_test_metadata_command");
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    let file = dir.join("metadata.ls");
    std::fs::write(
        &file,
        r#"(defn abs
  [x]
  :example [(= (abs 5) 5)]
  :invariant (>= result 0)
  (if (< x 0) (- 0 x) x))
"#,
    )
    .unwrap();

    let result = cmd_test_with_format(&file, CliTestFormat::Text);
    assert!(
        result.is_ok(),
        "metadata test command should succeed: {result:?}"
    );

    std::fs::remove_dir_all(&dir).unwrap();
}

#[test]
fn test_repo_doc_status_dogfooding_is_wired_for_metadata_fixture() {
    let repo_root = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..");
    let status_path = repo_root.join(".lsharp-doc-status");
    assert!(
        status_path.exists(),
        ".lsharp-doc-status を repo で運用するべき"
    );

    let status = lsharp_docs::tracker::load_doc_status(&status_path);
    let abs = status
        .entries
        .get("abs")
        .expect("examples/metadata.ls の abs は doc-status で追跡するべき");
    assert_eq!(abs.freshness, lsharp_docs::tracker::Freshness::Fresh);
    assert_eq!(abs.reviewed_by.as_deref(), Some("docs-maintainers"));
    assert!(abs.last_reviewed.is_some(), "初回 ack の日時を保持するべき");

    let ci = std::fs::read_to_string(repo_root.join(".github/workflows/ci.yml")).unwrap();
    assert!(
        ci.contains("scripts/ci/doc-status-check.sh"),
        "CI は doc-status check script を実行するべき"
    );

    let operation_doc = repo_root.join("docs/development/operations/documentation-freshness.md");
    assert!(operation_doc.exists(), "doc-status 運用手順が必要");

    let site_manifest = std::fs::read_to_string(repo_root.join("docs/site.toml")).unwrap();
    assert!(
        site_manifest.contains("docs/development/operations/documentation-freshness.md"),
        "doc-status 運用手順は docs site に公開するべき"
    );
}

#[test]
fn test_has_metadata_errors_detects_lowercase_error_diagnostics() {
    let diagnostics = vec![
        "[warning] add: doc note".to_string(),
        "[error] abs: unknown-fn in :invariant".to_string(),
    ];

    assert!(
        has_metadata_errors(&diagnostics),
        "metadata diagnostics は lowercase display でも error を検出するべき"
    );
}

#[test]
fn test_cmd_check_package_generates_api_json_and_checksum() {
    let dir = std::env::temp_dir().join("lsharp_test_check_package");
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(dir.join("src")).unwrap();
    std::fs::write(
        dir.join("lsharp.toml"),
        "[project]\nname = \"demo\"\nversion = \"0.3.0\"\nentry = \"src/Geometry.ls\"\n",
    )
    .unwrap();
    std::fs::write(
        dir.join("src/Geometry.ls"),
        "(module Geometry)\n(defn add [x y] :doc \"加算\" (+ x y))",
    )
    .unwrap();

    let summary = cmd_check_package_in(&dir, None, None).unwrap();

    assert!(summary.contains("Validating lsharp.toml ... ok"));
    assert!(summary.contains("Generating api.json ... ok"));
    assert!(summary.contains("checksum: sha256:"));
    assert!(dir.join("docs/api.json").exists());

    std::fs::remove_dir_all(&dir).unwrap();
}

#[test]
fn test_cmd_info_reads_installed_package_api() {
    let dir = std::env::temp_dir().join("lsharp_test_info_package");
    let _ = std::fs::remove_dir_all(&dir);
    let package_dir = dir.join(".lsharp/packages/mylib-12345678");
    std::fs::create_dir_all(package_dir.join("docs")).unwrap();
    std::fs::create_dir_all(dir.join(".lsharp")).unwrap();
    std::fs::write(
        dir.join(".lsharp/lock.toml"),
        r#"
[[package]]
name = "mylib"
version = "0.2.0"
source = "git:https://github.com/user/mylib.git?tag=v0.2.0"
"#,
    )
    .unwrap();
    std::fs::write(
        package_dir.join("docs/api.json"),
        r#"{
  "package": "mylib",
  "version": "0.2.0",
  "modules": [
{
  "name": "Geometry",
  "doc": null,
  "functions": [
    {
      "name": "distance",
      "signature": "Point -> Point -> Float",
      "params": [],
      "returns": { "type": "Float", "doc": null },
      "doc": "2 点間の距離",
      "example": null
    }
  ],
  "types": []
}
  ]
}"#,
    )
    .unwrap();

    let summary = cmd_info_in(&dir, "mylib").unwrap();

    assert!(summary.contains("Package: mylib@0.2.0"));
    assert!(summary.contains("Source: git:https://github.com/user/mylib.git?tag=v0.2.0"));
    assert!(summary.contains("Geometry.distance : Point -> Point -> Float - 2 点間の距離"));
    assert!(summary.contains("Types:\n  (none)\n"));

    std::fs::remove_dir_all(&dir).unwrap();
}

#[test]
fn test_cmd_api_diff_reports_added_changed_removed() {
    let dir = std::env::temp_dir().join("lsharp_test_api_diff");
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();

    let old = dir.join("old.json");
    let new = dir.join("new.json");
    std::fs::write(
        &old,
        r#"{
  "package": "demo",
  "version": "0.1.0",
  "modules": [
{
  "name": "Geometry",
  "doc": null,
  "functions": [
    {
      "name": "distance",
      "signature": "Point -> Point -> Int",
      "params": [],
      "returns": { "type": "Int", "doc": null },
      "doc": null,
      "example": null
    },
    {
      "name": "obsolete",
      "signature": "Int -> Int",
      "params": [],
      "returns": { "type": "Int", "doc": null },
      "doc": null,
      "example": null
    }
  ],
  "types": []
}
  ]
}"#,
    )
    .unwrap();
    std::fs::write(
        &new,
        r#"{
  "package": "demo",
  "version": "0.2.0",
  "modules": [
{
  "name": "Geometry",
  "doc": null,
  "functions": [
    {
      "name": "distance",
      "signature": "Point -> Point -> Float",
      "params": [],
      "returns": { "type": "Float", "doc": null },
      "doc": null,
      "example": null
    },
    {
      "name": "rotate",
      "signature": "Vec2 -> Float -> Vec2",
      "params": [],
      "returns": { "type": "Vec2", "doc": null },
      "doc": null,
      "example": null
    }
  ],
  "types": []
}
  ]
}"#,
    )
    .unwrap();

    let summary =
        cmd_api_diff_specs(&dir, &old.display().to_string(), &new.display().to_string()).unwrap();

    assert!(summary.contains("Added:    + Geometry.rotate : Vec2 -> Float -> Vec2"));
    assert!(summary.contains(
        "Changed:  ~ Geometry.distance : Point -> Point -> Int -> Point -> Point -> Float  BREAKING"
    ));
    assert!(summary.contains("Removed:  - Geometry.obsolete"));

    std::fs::remove_dir_all(&dir).unwrap();
}

#[test]
fn test_cmd_api_diff_specs_supports_git_tags() {
    let dir = std::env::temp_dir().join("lsharp_test_api_diff_git_tags");
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(dir.join("docs")).unwrap();

    init_test_git_repo(&dir);
    std::fs::write(
        dir.join("docs/api.json"),
        r#"{
  "package": "demo",
  "version": "0.1.0",
  "modules": [
{
  "name": "Geometry",
  "doc": null,
  "functions": [],
  "types": []
}
  ]
}"#,
    )
    .unwrap();
    git_commit_all(&dir, "v0.1.0");
    git_tag(&dir, "v0.1.0");

    std::fs::write(
        dir.join("docs/api.json"),
        r#"{
  "package": "demo",
  "version": "0.2.0",
  "modules": [
{
  "name": "Geometry",
  "doc": null,
  "functions": [
    {
      "name": "rotate",
      "signature": "Vec2 -> Float -> Vec2",
      "params": [],
      "returns": { "type": "Vec2", "doc": null },
      "doc": null,
      "example": null
    }
  ],
  "types": []
}
  ]
}"#,
    )
    .unwrap();
    git_commit_all(&dir, "v0.2.0");
    git_tag(&dir, "v0.2.0");

    let summary = cmd_api_diff_specs(&dir, "v0.1.0", "v0.2.0").unwrap();

    assert!(summary.contains("Added:    + Geometry.rotate : Vec2 -> Float -> Vec2"));

    std::fs::remove_dir_all(&dir).unwrap();
}

#[test]
fn test_cmd_check_package_previous_tag_compares_against_git_tag() {
    let dir = std::env::temp_dir().join("lsharp_test_check_package_previous_tag");
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(dir.join("src")).unwrap();
    std::fs::create_dir_all(dir.join("docs")).unwrap();

    init_test_git_repo(&dir);
    std::fs::write(
        dir.join("lsharp.toml"),
        "[project]\nname = \"demo\"\nversion = \"0.1.0\"\nentry = \"src/Geometry.ls\"\n",
    )
    .unwrap();
    std::fs::write(
        dir.join("src/Geometry.ls"),
        "(module Geometry)\n(defn distance [p1 p2] 1)",
    )
    .unwrap();
    std::fs::write(
        dir.join("docs/api.json"),
        r#"{
  "package": "demo",
  "version": "0.1.0",
  "modules": [
{
  "name": "Geometry",
  "doc": null,
  "functions": [
    {
      "name": "distance",
      "signature": "Point -> Point -> Int",
      "params": [],
      "returns": { "type": "Int", "doc": null },
      "doc": null,
      "example": null
    }
  ],
  "types": []
}
  ]
}"#,
    )
    .unwrap();
    git_commit_all(&dir, "baseline");
    git_tag(&dir, "v0.1.0");

    std::fs::write(
        dir.join("src/Geometry.ls"),
        "(module Geometry)\n(defn distance [p1 p2] 1.0)\n(defn rotate [v angle] v)",
    )
    .unwrap();

    let summary = cmd_check_package_in(&dir, None, Some("v0.1.0")).unwrap();

    assert!(summary.contains("Comparing with v0.1.0 ..."));
    assert!(summary.contains("+ Geometry.rotate"));

    std::fs::remove_dir_all(&dir).unwrap();
}

fn init_test_git_repo(dir: &Path) {
    let output = std::process::Command::new("git")
        .args(["init"])
        .current_dir(dir)
        .output()
        .unwrap();
    assert!(output.status.success(), "git init failed: {:?}", output);
}

fn git_commit_all(dir: &Path, message: &str) {
    let add = std::process::Command::new("git")
        .args(["add", "."])
        .current_dir(dir)
        .output()
        .unwrap();
    assert!(add.status.success(), "git add failed: {:?}", add);

    let commit = std::process::Command::new("git")
        .args([
            "-c",
            "user.name=Codex",
            "-c",
            "user.email=codex@example.com",
            "commit",
            "-m",
            message,
        ])
        .current_dir(dir)
        .output()
        .unwrap();
    assert!(commit.status.success(), "git commit failed: {:?}", commit);
}

fn git_tag(dir: &Path, tag: &str) {
    let output = std::process::Command::new("git")
        .args(["tag", tag])
        .current_dir(dir)
        .output()
        .unwrap();
    assert!(output.status.success(), "git tag failed: {:?}", output);
}

#[test]
fn test_cmd_install_path_dependency() {
    // Path 依存のインストールをテスト
    let base_dir = std::env::temp_dir().join("lsharp_test_install_path_dep");
    let _ = std::fs::remove_dir_all(&base_dir);
    std::fs::create_dir_all(&base_dir).unwrap();

    // 依存先ディレクトリを作成 (lsharp.toml を含む)
    let dep_dir = base_dir.join("mylib");
    std::fs::create_dir_all(dep_dir.join("src")).unwrap();
    std::fs::write(dep_dir.join("lsharp.toml"), "[project]\nname = \"mylib\"\n").unwrap();
    std::fs::write(
        dep_dir.join("src/Lib.ls"),
        "(module Lib)\n(defn helper [] 1)",
    )
    .unwrap();

    // プロジェクトの lsharp.toml を作成
    std::fs::write(
        base_dir.join("lsharp.toml"),
        "[dependencies.mylib]\npath = \"mylib\"\n",
    )
    .unwrap();

    let result = cmd_install_in(&base_dir);
    assert!(
        result.is_ok(),
        "Path 依存のインストールは成功するべき: {:?}",
        result
    );

    let link_path = find_installed_package_dir(&base_dir, "mylib")
        .expect(".lsharp/packages/<name>-<hash> が必要");
    assert!(
        link_path.exists(),
        "インストール済み package dir が存在するべき"
    );
    assert!(
        dep_dir.join("docs/api.json").exists(),
        "install 時に docs/api.json を生成するべき"
    );
    assert!(
        base_dir.join(".lsharp").join("lock.toml").exists(),
        "install 時に .lsharp/lock.toml を生成するべき"
    );

    std::fs::remove_dir_all(&base_dir).unwrap();
}

#[test]
fn test_cmd_install_path_dependency_refuses_existing_non_symlink_destination() {
    let base_dir = std::env::temp_dir().join(format!(
        "lsharp_test_install_destination_collision_{}",
        std::process::id()
    ));
    let _ = std::fs::remove_dir_all(&base_dir);
    std::fs::create_dir_all(&base_dir).unwrap();

    let dep_dir = base_dir.join("mylib");
    std::fs::create_dir_all(dep_dir.join("src")).unwrap();
    std::fs::write(
        dep_dir.join("lsharp.toml"),
        "[project]\nname = \"mylib\"\nversion = \"1.0.0\"\n",
    )
    .unwrap();
    std::fs::write(dep_dir.join("src/Lib.ls"), "(module Lib)\n").unwrap();
    std::fs::write(
        base_dir.join("lsharp.toml"),
        "[dependencies.mylib]\npath = \"mylib\"\n",
    )
    .unwrap();

    let packages_dir = base_dir.join(".lsharp/packages");
    std::fs::create_dir_all(&packages_dir).unwrap();
    let source_id = format!("path:{}", dep_dir.canonicalize().unwrap().display());
    let destination = installed_package_dir(&packages_dir, "mylib", &source_id);
    std::fs::create_dir_all(&destination).unwrap();
    let sentinel = destination.join("sentinel");
    std::fs::write(&sentinel, "preserve\n").unwrap();

    let error = cmd_install_in(&base_dir)
        .expect_err("既存の非 symlink package destination は fail-closed であるべき");
    assert!(
        error
            .to_string()
            .contains("refusing to replace non-symlink path package"),
        "unexpected install error: {error}"
    );
    assert_eq!(
        std::fs::read_to_string(&sentinel).unwrap(),
        "preserve\n",
        "既存 destination は install failure 後も保持されるべき"
    );

    std::fs::remove_dir_all(&base_dir).unwrap();
}

#[test]
fn test_cmd_install_path_dependency_writes_module_index_for_exported_modules() {
    let base_dir = std::env::temp_dir().join("lsharp_test_install_module_index");
    let _ = std::fs::remove_dir_all(&base_dir);
    std::fs::create_dir_all(&base_dir).unwrap();

    let dep_dir = base_dir.join("mylib");
    std::fs::create_dir_all(dep_dir.join("src/Geometry")).unwrap();
    std::fs::write(
        dep_dir.join("lsharp.toml"),
        "[project]\nname = \"mylib\"\n[project.exports]\nmodules = [\"Geometry\", \"Geometry.Vec2\"]\n",
    )
    .unwrap();
    std::fs::write(
        dep_dir.join("src/Geometry.ls"),
        "(module Geometry)\n(defn distance [] 1)",
    )
    .unwrap();
    std::fs::write(
        dep_dir.join("src/Geometry/Vec2.ls"),
        "(module Geometry.Vec2)\n(defn zero [] 0)",
    )
    .unwrap();
    std::fs::write(
        dep_dir.join("src/Hidden.ls"),
        "(module Hidden)\n(defn secret [] 99)",
    )
    .unwrap();

    std::fs::write(
        base_dir.join("lsharp.toml"),
        "[dependencies.mylib]\npath = \"mylib\"\n",
    )
    .unwrap();

    let result = cmd_install_in(&base_dir);
    assert!(result.is_ok(), "install は成功するべき: {result:?}");

    let installed_dir = find_installed_package_dir(&base_dir, "mylib")
        .expect(".lsharp/packages/<name>-<hash> が必要");
    let index_dir = base_dir.join(".lsharp/module-index");
    let geometry_index = index_dir.join("Geometry.path");
    let vec2_index = index_dir.join("Geometry/Vec2.path");
    let hidden_index = index_dir.join("Hidden.path");

    assert!(
        geometry_index.exists(),
        "exported module の index が生成されるべき"
    );
    assert!(
        vec2_index.exists(),
        "nested exported module の index が生成されるべき"
    );
    assert!(
        !hidden_index.exists(),
        "非公開 module の index は生成しないべき"
    );

    let geometry_target = std::fs::read_to_string(&geometry_index).unwrap();
    let vec2_target = std::fs::read_to_string(&vec2_index).unwrap();
    let installed_relative = installed_dir.strip_prefix(&base_dir).unwrap();
    assert_eq!(
        geometry_target.trim(),
        installed_relative
            .join("src/Geometry.ls")
            .display()
            .to_string()
    );
    assert_eq!(
        vec2_target.trim(),
        installed_relative
            .join("src/Geometry/Vec2.ls")
            .display()
            .to_string()
    );

    std::fs::remove_dir_all(&base_dir).unwrap();
}

#[test]
fn test_cmd_init_creates_standard_package_layout() {
    let base_dir = std::env::temp_dir().join("lsharp_test_init_layout");
    let _ = std::fs::remove_dir_all(&base_dir);
    std::fs::create_dir_all(&base_dir).unwrap();

    let result = cmd_init_in(&base_dir, "demo-lib");
    assert!(result.is_ok(), "init は成功するべき: {result:?}");

    let project_dir = base_dir.join("demo-lib");
    assert!(project_dir.join("lsharp.toml").exists());
    assert!(project_dir.join("src/Main.ls").exists());
    assert!(project_dir.join("examples").is_dir());
    assert!(project_dir.join("tests").is_dir());
    assert!(project_dir.join("docs").is_dir());
    assert!(project_dir.join(".gitignore").exists());

    std::fs::remove_dir_all(&base_dir).unwrap();
}

#[test]
fn test_cmd_init_writes_main_entry_and_gitignore_defaults() {
    let base_dir = std::env::temp_dir().join("lsharp_test_init_contents");
    let _ = std::fs::remove_dir_all(&base_dir);
    std::fs::create_dir_all(&base_dir).unwrap();

    let result = cmd_init_in(&base_dir, "demo-app");
    assert!(result.is_ok(), "init は成功するべき: {result:?}");

    let project_dir = base_dir.join("demo-app");
    let toml = std::fs::read_to_string(project_dir.join("lsharp.toml")).unwrap();
    let main = std::fs::read_to_string(project_dir.join("src/Main.ls")).unwrap();
    let gitignore = std::fs::read_to_string(project_dir.join(".gitignore")).unwrap();

    assert!(toml.contains("entry = \"src/Main.ls\""));
    assert!(main.contains("(module Main)"));
    assert!(gitignore.contains("/.lsharp/"));

    std::fs::remove_dir_all(&base_dir).unwrap();
}

#[test]
fn test_cmd_add_writes_tagged_github_dependency_to_lsharp_toml() {
    let base_dir = std::env::temp_dir().join("lsharp_test_add_dependency");
    let _ = std::fs::remove_dir_all(&base_dir);
    std::fs::create_dir_all(&base_dir).unwrap();
    std::fs::write(
        base_dir.join("lsharp.toml"),
        "[project]\nname = \"demo\"\nversion = \"0.1.0\"\nentry = \"src/Main.ls\"\n",
    )
    .unwrap();

    let result = cmd_add_in(&base_dir, "github.com/user/geometry-utils", Some("v0.2.0"));
    assert!(result.is_ok(), "add は成功するべき: {result:?}");

    let content = std::fs::read_to_string(base_dir.join("lsharp.toml")).unwrap();
    assert!(content.contains("[dependencies.geometry-utils]"));
    assert!(content.contains("git = \"https://github.com/user/geometry-utils.git\""));
    assert!(content.contains("tag = \"v0.2.0\""));

    std::fs::remove_dir_all(&base_dir).unwrap();
}

#[test]
fn test_cmd_add_rejects_duplicate_dependency_name() {
    let base_dir = std::env::temp_dir().join("lsharp_test_add_dependency_duplicate");
    let _ = std::fs::remove_dir_all(&base_dir);
    std::fs::create_dir_all(&base_dir).unwrap();
    std::fs::write(
        base_dir.join("lsharp.toml"),
        r#"[project]
name = "demo"
version = "0.1.0"
entry = "src/Main.ls"

[dependencies.geometry-utils]
git = "https://github.com/user/geometry-utils.git"
tag = "v0.1.0"
"#,
    )
    .unwrap();

    let result = cmd_add_in(
        &base_dir,
        "https://github.com/user/geometry-utils",
        Some("v0.2.0"),
    );
    assert!(result.is_err(), "重複 dependency は失敗するべき");

    std::fs::remove_dir_all(&base_dir).unwrap();
}

#[test]
fn test_cmd_install_path_dependency_missing_path() {
    // 宣言済みの存在しない path 依存は native installer と同じく fail-closed にする
    let base_dir = std::env::temp_dir().join("lsharp_test_install_missing_path");
    let _ = std::fs::remove_dir_all(&base_dir);
    std::fs::create_dir_all(&base_dir).unwrap();

    std::fs::write(
        base_dir.join("lsharp.toml"),
        "[dependencies.missing]\npath = \"nonexistent\"\n",
    )
    .unwrap();

    let error =
        cmd_install_in(&base_dir).expect_err("存在しない path 依存は暗黙に skip せず失敗するべき");
    assert!(
        error.to_string().contains("path dependency does not exist"),
        "native と同じ path provider input 診断を返すべき: {error}"
    );
    assert!(
        !base_dir.join(".lsharp/lock.toml").exists(),
        "invalid path で lock を確定してはいけない"
    );
    assert!(
        !base_dir.join(".lsharp/module-index").exists(),
        "invalid path で module-index を確定してはいけない"
    );
    assert!(
        !base_dir.join(".lsharp").exists(),
        "invalid path で managed install directory を作成してはいけない"
    );

    std::fs::remove_dir_all(&base_dir).unwrap();
}

#[test]
fn test_cmd_install_path_dependency_input_validation_fails_closed() {
    for (case, dependency_path) in [
        ("file", "not-a-directory"),
        ("missing-manifest", "missing-manifest"),
    ] {
        let base_dir = std::env::temp_dir().join(format!(
            "lsharp_test_install_path_input_{case}_{}",
            std::process::id()
        ));
        let _ = std::fs::remove_dir_all(&base_dir);
        std::fs::create_dir_all(&base_dir).unwrap();
        if case == "file" {
            std::fs::write(base_dir.join(dependency_path), "not a directory").unwrap();
        } else {
            std::fs::create_dir(base_dir.join(dependency_path)).unwrap();
        }
        std::fs::write(
            base_dir.join("lsharp.toml"),
            format!("[dependencies.invalid]\npath = \"{dependency_path}\"\n"),
        )
        .unwrap();

        let error = cmd_install_in(&base_dir)
            .expect_err("invalid path dependency must fail before metadata commit");
        let diagnostic = error.to_string();
        assert!(
            diagnostic.contains("path dependency is not a directory")
                || diagnostic.contains("path dependency has no lsharp.toml"),
            "unexpected native parity diagnostic for {case}: {diagnostic}"
        );
        assert!(!base_dir.join(".lsharp/lock.toml").exists());
        assert!(!base_dir.join(".lsharp/module-index").exists());
        std::fs::remove_dir_all(&base_dir).unwrap();
    }
}

#[test]
fn test_check_import_open_polymorphic_helper_stays_generalized() {
    let dir = std::env::temp_dir().join(format!(
        "lsharp_test_check_import_poly_helper_{}",
        std::process::id()
    ));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();

    std::fs::write(
        dir.join("Utils.ls"),
        "(module Utils)\n(defn choose-first [x y] x)\n(defn helper [] 0)",
    )
    .unwrap();
    std::fs::write(
        dir.join("Main.ls"),
        "(module Main)\n(import Utils :open)\n(defn main [] (do (print (choose-first 1 true)) (if (choose-first true 1) (print 1) (print 0))))",
    )
    .unwrap();

    let source = std::fs::read_to_string(dir.join("Main.ls")).unwrap();
    let program = lsharp_syntax::parse(&source).unwrap();
    let mut infer = lsharp_types::infer::Infer::new();
    let mut resolved_modules = std::collections::HashSet::new();

    resolve_imports_recursive(
        &program,
        &dir.join("Main.ls"),
        &mut infer,
        &mut resolved_modules,
    )
    .unwrap();
    let results = infer.infer_program(&program);
    let _ = std::fs::remove_dir_all(&dir);

    assert!(
        results.is_ok(),
        "extra helper があっても open import の多相関数は一般化を保つべき: {:?}",
        results.err()
    );
}

#[test]
fn test_check_selfhost_typeinfer_standalone_import_path() {
    let project_root = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..");
    let file = project_root.join("selfhost/src/Types/TypeInfer.ls");
    let source = std::fs::read_to_string(&file).unwrap();
    let program = lsharp_syntax::parse(&source).unwrap();
    let mut infer = lsharp_types::infer::Infer::new();
    let mut resolved_modules = std::collections::HashSet::new();

    resolve_imports_recursive(&program, &file, &mut infer, &mut resolved_modules).unwrap();
    let results = infer.infer_program(&program);

    assert!(
        results.is_ok(),
        "selfhost/src/Types/TypeInfer.ls standalone check path は成功するべき: {:?}",
        results.err()
    );
}

#[test]
fn test_check_import_only_blocks_non_selected_symbol() {
    let dir = std::env::temp_dir().join(format!(
        "lsharp_test_check_import_only_blocks_{}",
        std::process::id()
    ));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();

    std::fs::write(
        dir.join("Utils.ls"),
        "(module Utils)\n(defn helper [] 1)\n(defn secret [] 2)",
    )
    .unwrap();
    std::fs::write(
        dir.join("Main.ls"),
        "(module Main)\n(import Utils :only [helper])\n(defn main [] (secret))",
    )
    .unwrap();

    let source = std::fs::read_to_string(dir.join("Main.ls")).unwrap();
    let program = lsharp_syntax::parse(&source).unwrap();
    let mut infer = lsharp_types::infer::Infer::new();
    let mut resolved_modules = std::collections::HashSet::new();

    resolve_imports_recursive(
        &program,
        &dir.join("Main.ls"),
        &mut infer,
        &mut resolved_modules,
    )
    .unwrap();
    let results = infer.infer_program(&program);
    let _ = std::fs::remove_dir_all(&dir);

    assert!(
        results.is_err(),
        ":only で除外されたシンボルは参照できないべき"
    );
}

#[test]
fn test_check_private_import_blocks_symbol() {
    let dir = std::env::temp_dir().join(format!(
        "lsharp_test_check_private_import_blocks_{}",
        std::process::id()
    ));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();

    std::fs::write(
        dir.join("Utils.ls"),
        "(module Utils)\n(private (defn secret [] 2))\n(defn helper [] 1)",
    )
    .unwrap();
    std::fs::write(
        dir.join("Main.ls"),
        "(module Main)\n(import Utils)\n(defn main [] (secret))",
    )
    .unwrap();

    let source = std::fs::read_to_string(dir.join("Main.ls")).unwrap();
    let program = lsharp_syntax::parse(&source).unwrap();
    let mut infer = lsharp_types::infer::Infer::new();
    let mut resolved_modules = std::collections::HashSet::new();

    resolve_imports_recursive(
        &program,
        &dir.join("Main.ls"),
        &mut infer,
        &mut resolved_modules,
    )
    .unwrap();
    let results = infer.infer_program(&program);
    let _ = std::fs::remove_dir_all(&dir);

    assert!(
        results.is_err(),
        "private なシンボルは他モジュールから参照できないべき"
    );
}

#[test]
fn test_check_resolves_packages_from_project_root() {
    let dir = std::env::temp_dir().join(format!(
        "lsharp_test_check_project_root_packages_{}",
        std::process::id()
    ));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(dir.join("examples/demo")).unwrap();
    std::fs::create_dir_all(dir.join(".lsharp/packages/pkg-123/src")).unwrap();
    std::fs::write(dir.join("lsharp.toml"), "[project]\nname=\"demo\"\n").unwrap();
    std::fs::write(
        dir.join(".lsharp/packages/pkg-123/src/Helpers.ls"),
        "(module Helpers)\n(defn helper [] 1)",
    )
    .unwrap();
    std::fs::write(
        dir.join("examples/demo/Main.ls"),
        "(module Main)\n(import Helpers)\n(defn main [] (helper))",
    )
    .unwrap();

    let main_file = dir.join("examples/demo/Main.ls");
    let source = std::fs::read_to_string(&main_file).unwrap();
    let program = lsharp_syntax::parse(&source).unwrap();
    let mut infer = lsharp_types::infer::Infer::new();
    let mut resolved_modules = std::collections::HashSet::new();

    resolve_imports_recursive(&program, &main_file, &mut infer, &mut resolved_modules).unwrap();
    let results = infer.infer_program(&program);
    let _ = std::fs::remove_dir_all(&dir);

    assert!(
        results.is_ok(),
        "project root の packages 配下を探索できるべき"
    );
}

#[test]
fn test_check_rejects_non_exported_package_module() {
    let dir = std::env::temp_dir().join(format!(
        "lsharp_test_check_package_exports_{}",
        std::process::id()
    ));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(dir.join("src")).unwrap();
    std::fs::create_dir_all(dir.join(".lsharp/packages/demo-123/src")).unwrap();
    std::fs::write(dir.join("lsharp.toml"), "[project]\nname=\"app\"\n").unwrap();
    std::fs::write(
        dir.join(".lsharp/packages/demo-123/lsharp.toml"),
        "[project]\nname=\"demo\"\n[project.exports]\nmodules=[\"Public\"]\n",
    )
    .unwrap();
    std::fs::write(
        dir.join(".lsharp/packages/demo-123/src/Hidden.ls"),
        "(module Hidden)\n(defn helper [] 1)",
    )
    .unwrap();
    std::fs::write(
        dir.join("src/Main.ls"),
        "(module Main)\n(import Hidden)\n(defn main [] 0)",
    )
    .unwrap();

    let main_file = dir.join("src/Main.ls");
    let source = std::fs::read_to_string(&main_file).unwrap();
    let program = lsharp_syntax::parse(&source).unwrap();
    let mut infer = lsharp_types::infer::Infer::new();
    let mut resolved_modules = std::collections::HashSet::new();

    let result = resolve_imports_recursive(&program, &main_file, &mut infer, &mut resolved_modules);
    let _ = std::fs::remove_dir_all(&dir);

    assert!(
        result.is_err(),
        "非公開 package module の import は失敗するべき"
    );
}

#[test]
fn test_build_git_clone_args_basic() {
    // branch/tag なしの場合
    let args = build_git_clone_args(
        "https://github.com/user/repo.git",
        None,
        None,
        ".lsharp/packages/repo-12345678",
    );
    assert_eq!(
        args,
        vec![
            "clone",
            "--depth",
            "1",
            "https://github.com/user/repo.git",
            ".lsharp/packages/repo-12345678",
        ]
    );
}

#[test]
fn test_build_git_clone_args_with_branch() {
    let args = build_git_clone_args(
        "https://github.com/user/repo.git",
        Some("develop"),
        None,
        ".lsharp/packages/repo-12345678",
    );
    assert_eq!(
        args,
        vec![
            "clone",
            "--depth",
            "1",
            "--branch",
            "develop",
            "https://github.com/user/repo.git",
            ".lsharp/packages/repo-12345678",
        ]
    );
}

#[test]
fn test_build_git_clone_args_with_tag() {
    let args = build_git_clone_args(
        "https://github.com/user/repo.git",
        None,
        Some("v1.0.0"),
        ".lsharp/packages/repo-12345678",
    );
    assert_eq!(
        args,
        vec![
            "clone",
            "--depth",
            "1",
            "--branch",
            "v1.0.0",
            "https://github.com/user/repo.git",
            ".lsharp/packages/repo-12345678",
        ]
    );
}

#[test]
fn test_build_git_clone_args_branch_takes_priority_over_tag() {
    // branch と tag の両方が指定された場合、branch が優先される
    let args = build_git_clone_args(
        "https://github.com/user/repo.git",
        Some("main"),
        Some("v1.0.0"),
        ".lsharp/packages/repo-12345678",
    );
    assert_eq!(
        args,
        vec![
            "clone",
            "--depth",
            "1",
            "--branch",
            "main",
            "https://github.com/user/repo.git",
            ".lsharp/packages/repo-12345678",
        ]
    );
}

#[test]
fn test_git_clone_invalid_url_returns_error() {
    // 存在しない URL でクローンするとエラーを返す (クラッシュしない)
    let dir = std::env::temp_dir().join("lsharp_test_git_clone_invalid");
    let _ = std::fs::remove_dir_all(&dir);

    let dest = dir.join("nonexistent-repo");
    let result = git_clone(
        "https://invalid.example.com/no-such-repo.git",
        None,
        None,
        &dest,
    );

    assert!(
        result.is_err(),
        "存在しない URL の git clone はエラーを返すべき"
    );
    let err_msg = result.unwrap_err();
    assert!(
        err_msg.contains("git clone 失敗") || err_msg.contains("git コマンドの実行に失敗"),
        "エラーメッセージに適切な情報が含まれるべき: {err_msg}"
    );

    // クローン先ディレクトリが残っていれば削除
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn test_cmd_install_git_dependency_already_exists() {
    // 既にクローン済みのディレクトリがある場合はスキップされる
    let base_dir = std::env::temp_dir().join("lsharp_test_install_git_exists");
    let _ = std::fs::remove_dir_all(&base_dir);
    std::fs::create_dir_all(&base_dir).unwrap();

    // 依存先ディレクトリを手動で作成 (クローン済みを模擬)
    let source_id = dependency_source_string(
        &config::DependencySpec::Git {
            git: "https://github.com/user/mylib.git".to_string(),
            branch: Some("main".to_string()),
            tag: None,
        },
        &base_dir,
    );
    let deps_dir = installed_package_dir(
        &base_dir.join(".lsharp").join("packages"),
        "mylib",
        &source_id,
    );
    std::fs::create_dir_all(&deps_dir).unwrap();
    std::fs::write(
        deps_dir.join("lsharp.toml"),
        "[project]\nname = \"mylib\"\nversion = \"1.0.0\"\n",
    )
    .unwrap();

    std::fs::write(
        base_dir.join("lsharp.toml"),
        r#"[dependencies.mylib]
git = "https://github.com/user/mylib.git"
branch = "main"
"#,
    )
    .unwrap();

    let result = cmd_install_in(&base_dir);
    assert!(
        result.is_ok(),
        "既存ディレクトリがあればスキップして成功するべき"
    );

    std::fs::remove_dir_all(&base_dir).unwrap();
}

#[test]
fn test_cmd_install_git_dependency_clone_failure() {
    // local repository の存在しない branch は、temporary clone と state を残さず失敗するべき
    let base_dir = std::env::temp_dir().join(format!(
        "lsharp_test_install_git_fail_{}",
        std::process::id()
    ));
    let _ = std::fs::remove_dir_all(&base_dir);
    std::fs::create_dir_all(&base_dir).unwrap();
    let repository = base_dir.join("repository");
    std::fs::create_dir_all(&repository).unwrap();
    init_test_git_repo(&repository);
    std::fs::write(
        repository.join("lsharp.toml"),
        "[project]\nname = \"badrepo\"\nversion = \"1.0.0\"\n",
    )
    .unwrap();
    git_commit_all(&repository, "initial package");

    std::fs::write(
        base_dir.join("lsharp.toml"),
        format!(
            "[dependencies.badrepo]\ngit = \"{}\"\nbranch = \"missing\"\n",
            repository.display()
        ),
    )
    .unwrap();

    let result = cmd_install_in(&base_dir);
    assert!(
        result.is_err(),
        "git clone 失敗は lock/index 更新前に fail-closed であるべき"
    );
    assert!(result.unwrap_err().to_string().contains("git clone"));
    assert!(!base_dir.join(".lsharp/lock.toml").exists());
    assert!(!base_dir.join(".lsharp/module-index").exists());
    let packages_dir = base_dir.join(".lsharp/packages");
    assert!(
        !packages_dir
            .read_dir()
            .unwrap()
            .flatten()
            .any(|entry| entry.file_name().to_string_lossy().starts_with("badrepo-"))
    );
    assert!(
        !packages_dir
            .read_dir()
            .unwrap()
            .flatten()
            .any(|entry| entry.file_name().to_string_lossy().contains("tmp-"))
    );

    std::fs::remove_dir_all(&base_dir).unwrap();
}

#[test]
fn test_cmd_install_git_dependency_rejects_non_directory_destinations() {
    for destination_kind in ["file", "directory"] {
        let base_dir = std::env::temp_dir().join(format!(
            "lsharp_test_install_git_destination_{}_{}",
            destination_kind,
            std::process::id()
        ));
        let _ = std::fs::remove_dir_all(&base_dir);
        std::fs::create_dir_all(&base_dir).unwrap();
        let source_id = "git:file:///local/repository";
        let packages_dir = base_dir.join(".lsharp/packages");
        std::fs::create_dir_all(&packages_dir).unwrap();
        let destination = installed_package_dir(&packages_dir, "badrepo", source_id);
        if destination_kind == "file" {
            std::fs::write(&destination, "sentinel\n").unwrap();
        } else {
            std::fs::create_dir(&destination).unwrap();
        }
        std::fs::write(
            base_dir.join("lsharp.toml"),
            "[dependencies.badrepo]\ngit = \"file:///local/repository\"\n",
        )
        .unwrap();

        let error =
            cmd_install_in(&base_dir).expect_err("non-directory git destination は拒否されるべき");
        let expected = if destination_kind == "directory" {
            "existing git package has no lsharp.toml"
        } else {
            "git package destination is not a directory"
        };
        assert!(
            error.to_string().contains(expected),
            "unexpected error for {destination_kind}: {error}"
        );
        assert!(destination.exists());
        assert!(!base_dir.join(".lsharp/lock.toml").exists());
        assert!(!base_dir.join(".lsharp/module-index").exists());

        std::fs::remove_dir_all(&base_dir).unwrap();
    }
}

#[cfg(unix)]
#[test]
fn test_cmd_install_git_dependency_rejects_symlink_destinations() {
    for dangling in [false, true] {
        let base_dir = std::env::temp_dir().join(format!(
            "lsharp_test_install_git_symlink_{}_{}",
            dangling,
            std::process::id()
        ));
        let _ = std::fs::remove_dir_all(&base_dir);
        std::fs::create_dir_all(&base_dir).unwrap();
        let packages_dir = base_dir.join(".lsharp/packages");
        std::fs::create_dir_all(&packages_dir).unwrap();
        let destination =
            installed_package_dir(&packages_dir, "badrepo", "git:file:///local/repository");
        let target = if dangling {
            base_dir.join("missing-target")
        } else {
            let target = base_dir.join("existing-target");
            std::fs::create_dir(&target).unwrap();
            target
        };
        std::os::unix::fs::symlink(&target, &destination).unwrap();
        std::fs::write(
            base_dir.join("lsharp.toml"),
            "[dependencies.badrepo]\ngit = \"file:///local/repository\"\n",
        )
        .unwrap();

        let error =
            cmd_install_in(&base_dir).expect_err("symlinked git destination は拒否されるべき");
        assert!(
            error
                .to_string()
                .contains("refusing symlinked git package destination"),
            "unexpected error for dangling={dangling}: {error}"
        );
        assert!(destination.is_symlink());
        assert!(!base_dir.join(".lsharp/lock.toml").exists());
        assert!(!base_dir.join(".lsharp/module-index").exists());

        std::fs::remove_dir_all(&base_dir).unwrap();
    }
}

#[test]
fn test_cmd_install_path_dependency_no_toml() {
    // lsharp.toml がない依存先は native installer と同じく fail-closed にする
    let base_dir = std::env::temp_dir().join("lsharp_test_install_no_dep_toml");
    let _ = std::fs::remove_dir_all(&base_dir);
    std::fs::create_dir_all(&base_dir).unwrap();

    // 依存先ディレクトリを作成するが lsharp.toml は配置しない
    let dep_dir = base_dir.join("noconfig");
    std::fs::create_dir_all(&dep_dir).unwrap();

    std::fs::write(
        base_dir.join("lsharp.toml"),
        "[dependencies.noconfig]\npath = \"noconfig\"\n",
    )
    .unwrap();

    let error = cmd_install_in(&base_dir)
        .expect_err("lsharp.toml がない path 依存は暗黙に skip せず失敗するべき");
    assert!(
        error
            .to_string()
            .contains("path dependency has no lsharp.toml"),
        "native と同じ path provider input 診断を返すべき: {error}"
    );
    assert!(
        !base_dir.join(".lsharp/lock.toml").exists(),
        "invalid path で lock を確定してはいけない"
    );
    assert!(!base_dir.join(".lsharp/module-index").exists());

    std::fs::remove_dir_all(&base_dir).unwrap();
}

#[test]
fn test_cmd_install_version_dependency_uses_highest_compatible_cached_package() {
    let base_dir = std::env::temp_dir().join("lsharp_test_install_version_cached");
    let _ = std::fs::remove_dir_all(&base_dir);
    std::fs::create_dir_all(base_dir.join(".lsharp/packages/math-core-a/src")).unwrap();
    std::fs::create_dir_all(base_dir.join(".lsharp/packages/math-core-b/src")).unwrap();
    std::fs::create_dir_all(base_dir.join(".lsharp/packages/math-core-c/src")).unwrap();

    std::fs::write(
        base_dir.join(".lsharp/packages/math-core-a/lsharp.toml"),
        "[project]\nname = \"math-core\"\nversion = \"1.0.1\"\n",
    )
    .unwrap();
    std::fs::write(
        base_dir.join(".lsharp/packages/math-core-b/lsharp.toml"),
        "[project]\nname = \"math-core\"\nversion = \"1.4.0\"\n",
    )
    .unwrap();
    std::fs::write(
        base_dir.join(".lsharp/packages/math-core-c/lsharp.toml"),
        "[project]\nname = \"math-core\"\nversion = \"2.0.0\"\n",
    )
    .unwrap();
    std::fs::write(
        base_dir.join("lsharp.toml"),
        "[dependencies]\nmath-core = \"1.0.0\"\n",
    )
    .unwrap();

    let result = cmd_install_in(&base_dir);
    assert!(
        result.is_ok(),
        "cache からの semver 解決は成功するべき: {result:?}"
    );

    let lock = crate::lockfile::read_lockfile(&base_dir.join(".lsharp/lock.toml")).unwrap();
    let entry = lock
        .entries
        .iter()
        .find(|entry| entry.name == "math-core")
        .unwrap();
    assert_eq!(entry.version, "1.4.0");

    std::fs::remove_dir_all(&base_dir).unwrap();
}

#[test]
fn test_cmd_install_version_dependency_errors_when_no_cached_match_exists() {
    let base_dir = std::env::temp_dir().join("lsharp_test_install_version_missing");
    let _ = std::fs::remove_dir_all(&base_dir);
    std::fs::create_dir_all(base_dir.join(".lsharp/packages/math-core-a/src")).unwrap();

    std::fs::write(
        base_dir.join(".lsharp/packages/math-core-a/lsharp.toml"),
        "[project]\nname = \"math-core\"\nversion = \"2.0.0\"\n",
    )
    .unwrap();
    std::fs::write(
        base_dir.join("lsharp.toml"),
        "[dependencies]\nmath-core = \"1.0.0\"\n",
    )
    .unwrap();

    let result = cmd_install_in(&base_dir);
    assert!(
        result.is_err(),
        "一致する cache がない version 依存は失敗するべき"
    );

    std::fs::remove_dir_all(&base_dir).unwrap();
}

#[test]
fn test_cmd_install_version_dependency_requires_offline_cache_before_install_state() {
    let base_dir = std::env::temp_dir().join("lsharp_test_install_version_no_cache");
    let _ = std::fs::remove_dir_all(&base_dir);
    std::fs::create_dir_all(&base_dir).unwrap();
    std::fs::write(
        base_dir.join("lsharp.toml"),
        "[dependencies]\nmath-core = \"1.0.0\"\n",
    )
    .unwrap();

    let error = cmd_install_in(&base_dir)
        .expect_err("registry dependency without offline cache must fail before install state");
    assert!(
        error
            .to_string()
            .contains("registry provider acquisition is an external boundary"),
        "live registry acquisition must be explicit: {error}"
    );
    assert!(!base_dir.join(".lsharp").exists());

    std::fs::remove_dir_all(&base_dir).unwrap();
}

#[test]
fn test_cmd_install_version_dependency_rejects_signed_semver_requirement() {
    let base_dir = std::env::temp_dir().join("lsharp_test_install_version_signed");
    let _ = std::fs::remove_dir_all(&base_dir);
    std::fs::create_dir_all(base_dir.join(".lsharp/packages/math-core-a/src")).unwrap();

    std::fs::write(
        base_dir.join(".lsharp/packages/math-core-a/lsharp.toml"),
        "[project]\nname = \"math-core\"\nversion = \"1.0.0\"\n",
    )
    .unwrap();
    std::fs::write(
        base_dir.join("lsharp.toml"),
        "[dependencies]\nmath-core = \"+1.0.0\"\n",
    )
    .unwrap();

    let result = cmd_install_in(&base_dir);
    assert!(
        result.is_err(),
        "符号付き semver requirement は Rust/native とも fail-closed であるべき"
    );

    std::fs::remove_dir_all(&base_dir).unwrap();
}

#[test]
fn test_cmd_install_invalid_cached_candidate_fails_closed_without_state_change() {
    for candidate_kind in [
        "root-symlink",
        "nested-symlink",
        "invalid-manifest",
        "missing-version",
    ] {
        let base_dir = std::env::temp_dir().join(format!(
            "lsharp_test_install_cached_candidate_{candidate_kind}"
        ));
        let _ = std::fs::remove_dir_all(&base_dir);
        let project_dir = base_dir.join("project");
        let packages_dir = project_dir.join(".lsharp/packages");
        let index_dir = project_dir.join(".lsharp/module-index");
        std::fs::create_dir_all(&packages_dir).unwrap();
        std::fs::create_dir_all(&index_dir).unwrap();
        std::fs::write(
            project_dir.join("lsharp.toml"),
            "[dependencies]\ndemo = \"1.0.0\"\n",
        )
        .unwrap();
        std::fs::write(project_dir.join(".lsharp/lock.toml"), "lock sentinel\n").unwrap();
        std::fs::write(index_dir.join("sentinel.path"), "index sentinel\n").unwrap();

        let external = base_dir.join("external");
        std::fs::create_dir_all(external.join("src")).unwrap();
        std::fs::write(
            external.join("lsharp.toml"),
            "[project]\nname = \"demo\"\nversion = \"9.0.0\"\n",
        )
        .unwrap();
        let candidate = packages_dir.join("demo-invalid");
        match candidate_kind {
            "root-symlink" => {
                #[cfg(unix)]
                std::os::unix::fs::symlink(&external, &candidate).unwrap();
            }
            "nested-symlink" => {
                std::fs::create_dir_all(candidate.join("src")).unwrap();
                std::fs::write(
                    candidate.join("lsharp.toml"),
                    "[project]\nname = \"demo\"\nversion = \"9.0.0\"\n",
                )
                .unwrap();
                #[cfg(unix)]
                std::os::unix::fs::symlink(&external, candidate.join("src/linked-source")).unwrap();
            }
            "invalid-manifest" => {
                std::fs::create_dir_all(&candidate).unwrap();
                std::fs::write(
                    candidate.join("lsharp.toml"),
                    "[project\nname = \"demo\"\nversion = \"9.0.0\"\n",
                )
                .unwrap();
            }
            "missing-version" => {
                std::fs::create_dir_all(&candidate).unwrap();
                std::fs::write(
                    candidate.join("lsharp.toml"),
                    "[project]\nname = \"demo\"\n",
                )
                .unwrap();
            }
            _ => unreachable!(),
        }

        let result = cmd_install_in(&project_dir);

        assert!(result.is_err(), "unsafe cached candidate must fail closed");
        let error = result.unwrap_err().to_string();
        assert!(
            error.contains("cached candidate"),
            "unexpected error: {error}"
        );
        assert_eq!(
            std::fs::read_to_string(project_dir.join(".lsharp/lock.toml")).unwrap(),
            "lock sentinel\n"
        );
        assert_eq!(
            std::fs::read_to_string(index_dir.join("sentinel.path")).unwrap(),
            "index sentinel\n"
        );
        assert!(std::fs::read_dir(&packages_dir).unwrap().all(|entry| {
            !entry
                .unwrap()
                .file_name()
                .to_string_lossy()
                .starts_with(".install-txn-")
        }));

        std::fs::remove_dir_all(&base_dir).unwrap();
    }
}

#[test]
fn test_cmd_install_mixed_path_and_cached_failure_keeps_state_unpromoted() {
    let base_dir = std::env::temp_dir().join("lsharp_test_install_mixed_transaction");
    let _ = std::fs::remove_dir_all(&base_dir);
    let project_dir = base_dir.join("project");
    let dependency_dir = base_dir.join("local-lib");
    std::fs::create_dir_all(dependency_dir.join("src")).unwrap();
    std::fs::create_dir_all(project_dir.join(".lsharp/packages")).unwrap();
    std::fs::create_dir_all(project_dir.join(".lsharp/module-index")).unwrap();
    std::fs::write(
        dependency_dir.join("lsharp.toml"),
        "[project]\nname = \"local-lib\"\nversion = \"1.0.0\"\n",
    )
    .unwrap();
    std::fs::write(
        project_dir.join("lsharp.toml"),
        "[dependencies]\n\"a-local-lib\" = { path = \"../local-lib\" }\n\"z-missing\" = \"1.0.0\"\n",
    )
    .unwrap();
    std::fs::write(project_dir.join(".lsharp/lock.toml"), "lock sentinel\n").unwrap();
    std::fs::write(
        project_dir.join(".lsharp/module-index/sentinel.path"),
        "index sentinel\n",
    )
    .unwrap();

    let source_id = format!("path:{}", dependency_dir.canonicalize().unwrap().display());
    let destination = installed_package_dir(
        &project_dir.join(".lsharp/packages"),
        "a-local-lib",
        &source_id,
    );

    let result = cmd_install_in(&project_dir);
    assert!(
        result.is_err(),
        "後続 cached miss は install 全体を失敗させるべき"
    );
    assert!(
        destination.symlink_metadata().is_err(),
        "失敗時に path package destination を promote してはならない"
    );
    assert_eq!(
        std::fs::read_to_string(project_dir.join(".lsharp/lock.toml")).unwrap(),
        "lock sentinel\n"
    );
    assert_eq!(
        std::fs::read_to_string(project_dir.join(".lsharp/module-index/sentinel.path")).unwrap(),
        "index sentinel\n"
    );
    assert!(
        std::fs::read_dir(project_dir.join(".lsharp/packages"))
            .unwrap()
            .all(|entry| !entry
                .unwrap()
                .file_name()
                .to_string_lossy()
                .starts_with(".install-txn-")),
        "失敗時に transaction staging を残してはならない"
    );

    std::fs::remove_dir_all(&base_dir).unwrap();
}

#[test]
fn test_cmd_install_promotion_failure_restores_previous_state() {
    let base_dir = std::env::temp_dir().join("lsharp_test_install_promotion_rollback");
    let _ = std::fs::remove_dir_all(&base_dir);
    let project_dir = base_dir.join("project");
    let dependency_dir = base_dir.join("local-lib");
    let git_repo = base_dir.join("git-lib");
    std::fs::create_dir_all(dependency_dir.join("src")).unwrap();
    std::fs::create_dir_all(git_repo.join("src")).unwrap();
    init_test_git_repo(&git_repo);
    std::fs::write(
        dependency_dir.join("lsharp.toml"),
        "[project]\nname = \"local-lib\"\nversion = \"1.0.0\"\n",
    )
    .unwrap();
    std::fs::write(
        git_repo.join("lsharp.toml"),
        "[project]\nname = \"git-lib\"\nversion = \"1.0.0\"\n",
    )
    .unwrap();
    git_commit_all(&git_repo, "initial package");
    std::fs::create_dir_all(project_dir.join(".lsharp/packages")).unwrap();
    std::fs::create_dir_all(project_dir.join(".lsharp/module-index")).unwrap();
    std::fs::write(
        project_dir.join("lsharp.toml"),
        format!(
            "[dependencies]\n\"a-local-lib\" = {{ path = \"../local-lib\" }}\n\"z-git\" = {{ git = \"{}\" }}\n",
            git_repo.display()
        ),
    )
    .unwrap();
    std::fs::write(project_dir.join(".lsharp/lock.toml"), "lock sentinel\n").unwrap();
    std::fs::write(
        project_dir.join(".lsharp/module-index/sentinel.path"),
        "index sentinel\n",
    )
    .unwrap();

    let packages_dir = project_dir.join(".lsharp/packages");
    let source_id = format!("path:{}", dependency_dir.canonicalize().unwrap().display());
    let destination = installed_package_dir(&packages_dir, "a-local-lib", &source_id);
    let old_target = base_dir.join("old-local-lib");
    std::fs::create_dir_all(&old_target).unwrap();
    #[cfg(unix)]
    std::os::unix::fs::symlink(&old_target, &destination).unwrap();

    INSTALL_TEST_PROMOTION_FAILPOINT.store(1, std::sync::atomic::Ordering::SeqCst);
    let result = cmd_install_in(&project_dir);
    INSTALL_TEST_PROMOTION_FAILPOINT.store(usize::MAX, std::sync::atomic::Ordering::SeqCst);

    assert!(
        result.is_err(),
        "test-only promotion failpoint は install を失敗させるべき"
    );
    assert!(
        destination.is_symlink(),
        "既存 path destination は rollback 後も symlink のまま保持されるべき"
    );
    assert_eq!(std::fs::read_link(&destination).unwrap(), old_target);
    let git_source = format!("git:{}", git_repo.display());
    let git_destination = installed_package_dir(&packages_dir, "z-git", &git_source);
    assert!(
        git_destination.symlink_metadata().is_err(),
        "失敗時に後続 Git destination を残してはならない"
    );
    assert_eq!(
        std::fs::read_to_string(project_dir.join(".lsharp/lock.toml")).unwrap(),
        "lock sentinel\n"
    );
    assert_eq!(
        std::fs::read_to_string(project_dir.join(".lsharp/module-index/sentinel.path")).unwrap(),
        "index sentinel\n"
    );
    assert!(
        std::fs::read_dir(&packages_dir).unwrap().all(|entry| !entry
            .unwrap()
            .file_name()
            .to_string_lossy()
            .starts_with(".install-txn-")),
        "promotion rollback後に transaction staging を残してはならない"
    );

    std::fs::remove_dir_all(&base_dir).unwrap();
}

#[test]
fn test_cmd_install_metadata_failure_restores_previous_state() {
    for (failpoint, label) in [(1u8, "lock"), (2u8, "index")] {
        let base_dir = std::env::temp_dir().join(format!("lsharp_test_install_metadata_{label}"));
        let _ = std::fs::remove_dir_all(&base_dir);
        let project_dir = base_dir.join("project");
        let dependency_dir = base_dir.join("local-lib");
        let git_repo = base_dir.join("git-lib");
        std::fs::create_dir_all(dependency_dir.join("src")).unwrap();
        std::fs::create_dir_all(git_repo.join("src")).unwrap();
        init_test_git_repo(&git_repo);
        std::fs::write(
            dependency_dir.join("lsharp.toml"),
            "[project]\nname = \"local-lib\"\nversion = \"1.0.0\"\n",
        )
        .unwrap();
        std::fs::write(
            git_repo.join("lsharp.toml"),
            "[project]\nname = \"git-lib\"\nversion = \"1.0.0\"\n",
        )
        .unwrap();
        git_commit_all(&git_repo, "initial package");
        std::fs::create_dir_all(project_dir.join(".lsharp/packages")).unwrap();
        std::fs::create_dir_all(project_dir.join(".lsharp/module-index")).unwrap();
        std::fs::write(
            project_dir.join("lsharp.toml"),
            format!(
                "[dependencies]\n\"a-local-lib\" = {{ path = \"../local-lib\" }}\n\"z-git\" = {{ git = \"{}\" }}\n",
                git_repo.display()
            ),
        )
        .unwrap();
        std::fs::write(project_dir.join(".lsharp/lock.toml"), "lock sentinel\n").unwrap();
        std::fs::write(
            project_dir.join(".lsharp/module-index/sentinel.path"),
            "index sentinel\n",
        )
        .unwrap();

        let packages_dir = project_dir.join(".lsharp/packages");
        let source_id = format!("path:{}", dependency_dir.canonicalize().unwrap().display());
        let destination = installed_package_dir(&packages_dir, "a-local-lib", &source_id);
        let old_target = base_dir.join("old-local-lib");
        std::fs::create_dir_all(&old_target).unwrap();
        #[cfg(unix)]
        std::os::unix::fs::symlink(&old_target, &destination).unwrap();

        INSTALL_TEST_METADATA_FAILPOINT.store(failpoint, std::sync::atomic::Ordering::SeqCst);
        let result = cmd_install_in(&project_dir);
        INSTALL_TEST_METADATA_FAILPOINT.store(0, std::sync::atomic::Ordering::SeqCst);

        assert!(
            result.is_err(),
            "test-only {label} failpoint は install を失敗させるべき"
        );
        assert!(destination.is_symlink());
        assert_eq!(std::fs::read_link(&destination).unwrap(), old_target);
        let git_source = format!("git:{}", git_repo.display());
        let git_destination = installed_package_dir(&packages_dir, "z-git", &git_source);
        assert!(
            git_destination.symlink_metadata().is_err(),
            "{label} failure後に fresh Git destination を残してはならない"
        );
        assert_eq!(
            std::fs::read_to_string(project_dir.join(".lsharp/lock.toml")).unwrap(),
            "lock sentinel\n"
        );
        assert_eq!(
            std::fs::read_to_string(project_dir.join(".lsharp/module-index/sentinel.path"))
                .unwrap(),
            "index sentinel\n"
        );
        assert!(
            std::fs::read_dir(&packages_dir).unwrap().all(|entry| !entry
                .unwrap()
                .file_name()
                .to_string_lossy()
                .starts_with(".install-txn-")),
            "{label} rollback後に transaction staging を残してはならない"
        );

        std::fs::remove_dir_all(&base_dir).unwrap();
    }
}

#[test]
fn test_cmd_install_sync_failure_restores_previous_state() {
    for (failpoint, label) in [
        (INSTALL_SYNC_PROMOTION_BEFORE, "promotion-before-sync"),
        (INSTALL_SYNC_PROMOTION_AFTER, "promotion-after-sync"),
        (INSTALL_SYNC_LOCK, "lock-sync"),
        (INSTALL_SYNC_INDEX, "index-sync"),
    ] {
        let base_dir = std::env::temp_dir().join(format!("lsharp_test_install_sync_{label}"));
        let _ = std::fs::remove_dir_all(&base_dir);
        let project_dir = base_dir.join("project");
        let dependency_dir = base_dir.join("local-lib");
        let git_repo = base_dir.join("git-lib");
        std::fs::create_dir_all(dependency_dir.join("src")).unwrap();
        std::fs::create_dir_all(git_repo.join("src")).unwrap();
        init_test_git_repo(&git_repo);
        std::fs::write(
            dependency_dir.join("lsharp.toml"),
            "[project]\nname = \"local-lib\"\nversion = \"1.0.0\"\n",
        )
        .unwrap();
        std::fs::write(
            git_repo.join("lsharp.toml"),
            "[project]\nname = \"git-lib\"\nversion = \"1.0.0\"\n",
        )
        .unwrap();
        git_commit_all(&git_repo, "initial package");
        std::fs::create_dir_all(project_dir.join(".lsharp/packages")).unwrap();
        std::fs::create_dir_all(project_dir.join(".lsharp/module-index")).unwrap();
        std::fs::write(
            project_dir.join("lsharp.toml"),
            format!(
                "[dependencies]\n\"a-local-lib\" = {{ path = \"../local-lib\" }}\n\"z-git\" = {{ git = \"{}\" }}\n",
                git_repo.display()
            ),
        )
        .unwrap();
        std::fs::write(project_dir.join(".lsharp/lock.toml"), "lock sentinel\n").unwrap();
        std::fs::write(
            project_dir.join(".lsharp/module-index/sentinel.path"),
            "index sentinel\n",
        )
        .unwrap();

        let packages_dir = project_dir.join(".lsharp/packages");
        let source_id = format!("path:{}", dependency_dir.canonicalize().unwrap().display());
        let destination = installed_package_dir(&packages_dir, "a-local-lib", &source_id);
        let old_target = base_dir.join("old-local-lib");
        std::fs::create_dir_all(&old_target).unwrap();
        #[cfg(unix)]
        std::os::unix::fs::symlink(&old_target, &destination).unwrap();

        INSTALL_TEST_SYNC_FAILPOINT.store(failpoint, std::sync::atomic::Ordering::SeqCst);
        let result = cmd_install_in(&project_dir);
        INSTALL_TEST_SYNC_FAILPOINT.store(0, std::sync::atomic::Ordering::SeqCst);

        assert!(
            result.is_err(),
            "test-only {label} failpoint は install を失敗させるべき"
        );
        assert!(destination.is_symlink());
        assert_eq!(std::fs::read_link(&destination).unwrap(), old_target);
        let git_source = format!("git:{}", git_repo.display());
        let git_destination = installed_package_dir(&packages_dir, "z-git", &git_source);
        assert!(git_destination.symlink_metadata().is_err());
        assert_eq!(
            std::fs::read_to_string(project_dir.join(".lsharp/lock.toml")).unwrap(),
            "lock sentinel\n"
        );
        assert_eq!(
            std::fs::read_to_string(project_dir.join(".lsharp/module-index/sentinel.path"))
                .unwrap(),
            "index sentinel\n"
        );
        assert!(std::fs::read_dir(&packages_dir).unwrap().all(|entry| {
            !entry
                .unwrap()
                .file_name()
                .to_string_lossy()
                .starts_with(".install-txn-")
        }));

        std::fs::remove_dir_all(&base_dir).unwrap();
    }
}
