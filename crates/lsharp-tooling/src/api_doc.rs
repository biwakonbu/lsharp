use lsharp_syntax::ast::Program;
use lsharp_types::{infer::Infer, types::TypeScheme};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ApiDoc {
    pub package: String,
    pub version: String,
    pub modules: Vec<ApiModule>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ApiModule {
    pub name: String,
    pub doc: Option<String>,
    pub functions: Vec<ApiFunction>,
    pub types: Vec<ApiType>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ApiFunction {
    pub name: String,
    pub signature: String,
    pub params: Vec<ApiParam>,
    pub returns: ApiReturn,
    pub doc: Option<String>,
    pub example: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ApiParam {
    pub name: String,
    #[serde(rename = "type")]
    pub ty: String,
    pub doc: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ApiReturn {
    #[serde(rename = "type")]
    pub ty: String,
    pub doc: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ApiType {
    pub name: String,
    pub kind: String,
}

pub fn build_api_doc(
    package: &str,
    version: &str,
    program: &Program,
    type_results: &[(String, TypeScheme)],
    infer: &Infer,
) -> ApiDoc {
    use lsharp_syntax::ast::Decl;

    let module_name = infer
        .module_env
        .name
        .clone()
        .unwrap_or_else(|| "Main".to_string());
    let private_names: std::collections::HashSet<&str> = infer
        .module_env
        .privates
        .iter()
        .map(String::as_str)
        .collect();
    let mut functions = Vec::new();
    let mut types = Vec::new();

    for decl in &program.decls {
        let (decl, is_private) = match decl {
            Decl::Private { inner, .. } => (inner.as_ref(), true),
            other => (other, false),
        };

        match decl {
            Decl::Defn {
                name,
                params,
                metadata,
                ..
            } => {
                if is_private || private_names.contains(name.as_str()) {
                    continue;
                }
                let scheme = type_results.iter().find(|(candidate, _)| candidate == name);
                let (param_types, return_type) = scheme
                    .map(|(_, scheme)| split_signature(&scheme.ty))
                    .unwrap_or_else(|| (vec!["?".to_string(); params.len()], "?".to_string()));
                let signature = scheme
                    .map(|(_, scheme)| render_signature(&scheme.ty))
                    .unwrap_or_else(|| "?".to_string());
                let params = params
                    .iter()
                    .enumerate()
                    .map(|(index, param)| ApiParam {
                        name: param.name.clone(),
                        ty: param_types
                            .get(index)
                            .cloned()
                            .unwrap_or_else(|| "?".to_string()),
                        doc: metadata.as_ref().and_then(|meta| {
                            meta.params
                                .iter()
                                .find(|(name, _)| name == &param.name)
                                .map(|(_, doc)| doc.clone())
                        }),
                    })
                    .collect();
                let example = metadata
                    .as_ref()
                    .and_then(|meta| meta.example.first().map(|example| format!("{example}")));

                functions.push(ApiFunction {
                    name: name.clone(),
                    signature,
                    params,
                    returns: ApiReturn {
                        ty: return_type,
                        doc: metadata.as_ref().and_then(|meta| meta.returns.clone()),
                    },
                    doc: metadata.as_ref().and_then(|meta| meta.doc.clone()),
                    example,
                });
            }
            Decl::RecordDef { name, .. } => {
                if is_private || private_names.contains(name.as_str()) {
                    continue;
                }
                types.push(ApiType {
                    name: name.clone(),
                    kind: "record".to_string(),
                });
            }
            Decl::TypeDef { name, .. } => {
                if is_private || private_names.contains(name.as_str()) {
                    continue;
                }
                types.push(ApiType {
                    name: name.clone(),
                    kind: "adt".to_string(),
                });
            }
            Decl::TypeAlias { name, .. } => {
                if is_private || private_names.contains(name.as_str()) {
                    continue;
                }
                types.push(ApiType {
                    name: name.clone(),
                    kind: "alias".to_string(),
                });
            }
            Decl::TraitDef { name, .. } => {
                if is_private || private_names.contains(name.as_str()) {
                    continue;
                }
                types.push(ApiType {
                    name: name.clone(),
                    kind: "trait".to_string(),
                });
            }
            _ => {}
        }
    }

    ApiDoc {
        package: package.to_string(),
        version: version.to_string(),
        modules: vec![ApiModule {
            name: module_name,
            doc: None,
            functions,
            types,
        }],
    }
}

pub fn build_api_doc_for_file(package: &str, version: &str, file: &Path) -> miette::Result<ApiDoc> {
    let source =
        std::fs::read_to_string(file).map_err(|e| miette::miette!("{}: {}", file.display(), e))?;
    let program = lsharp_syntax::parse(&source)
        .map_err(|e| miette::miette!("[{}] API doc 用 parse に失敗しました: {e}", e.code()))?;
    let mut infer = Infer::new();
    let type_results = infer
        .infer_program(&program)
        .map_err(|e| miette::miette!("[{}] API doc 用 type check に失敗しました: {e}", e.code()))?;
    let mut api = build_api_doc(package, version, &program, &type_results, &infer);
    if let Some(module) = api.modules.first_mut() {
        module.name = infer
            .module_env
            .name
            .clone()
            .or_else(|| module_name_from_program(&program))
            .or_else(|| {
                file.file_stem()
                    .and_then(|stem| stem.to_str())
                    .map(|stem| stem.to_string())
            })
            .unwrap_or_else(|| "Main".to_string());
        module.doc = extract_module_doc(&source);
    }
    Ok(api)
}

pub fn build_api_doc_for_package(
    package_root: &Path,
    package: &str,
    version: &str,
) -> miette::Result<ApiDoc> {
    let source_root = package_root.join("src");
    let mut files = Vec::new();
    collect_lsharp_files(&source_root, &mut files)?;
    files.sort();

    let mut modules = Vec::new();
    for file in files {
        let mut doc = build_api_doc_for_file(package, version, &file)?;
        if let Some(module) = doc.modules.pop() {
            modules.push(module);
        }
    }
    modules.sort_by(|left, right| left.name.cmp(&right.name));

    Ok(ApiDoc {
        package: package.to_string(),
        version: version.to_string(),
        modules,
    })
}

fn collect_lsharp_files(dir: &Path, out: &mut Vec<PathBuf>) -> miette::Result<()> {
    if !dir.exists() {
        return Ok(());
    }
    let entries =
        std::fs::read_dir(dir).map_err(|e| miette::miette!("{}: {}", dir.display(), e))?;
    for entry in entries {
        let entry = entry.map_err(|e| miette::miette!("{}: {}", dir.display(), e))?;
        let path = entry.path();
        if path.is_dir() {
            collect_lsharp_files(&path, out)?;
        } else if path.extension().and_then(|ext| ext.to_str()) == Some("ls") {
            out.push(path);
        }
    }
    Ok(())
}

fn render_signature(ty: &lsharp_types::types::Type) -> String {
    use lsharp_types::types::Type;

    match ty {
        Type::Fun(params, ret) => {
            let mut parts: Vec<String> = params.iter().map(render_signature).collect();
            parts.push(render_signature(ret));
            parts.join(" -> ")
        }
        other => format!("{other}"),
    }
}

fn split_signature(ty: &lsharp_types::types::Type) -> (Vec<String>, String) {
    use lsharp_types::types::Type;

    match ty {
        Type::Fun(params, ret) => (
            params.iter().map(render_signature).collect(),
            render_signature(ret),
        ),
        other => (Vec::new(), render_signature(other)),
    }
}

fn module_name_from_program(program: &Program) -> Option<String> {
    use lsharp_syntax::ast::Decl;

    program.decls.iter().find_map(|decl| match decl {
        Decl::ModuleDecl { name, .. } => Some(name.clone()),
        _ => None,
    })
}

fn extract_module_doc(source: &str) -> Option<String> {
    let mut docs = Vec::new();

    for line in source.lines() {
        let trimmed = line.trim_start();
        if let Some(comment) = trimmed.strip_prefix(";;") {
            docs.push(comment.trim().to_string());
            continue;
        }
        if trimmed.is_empty() {
            continue;
        }
        break;
    }

    if docs.is_empty() {
        return None;
    }

    if docs
        .first()
        .is_some_and(|line| line.contains(".ls -") || line.contains(".ls:"))
    {
        docs.remove(0);
    }

    let text = docs
        .into_iter()
        .filter(|line| !line.is_empty())
        .collect::<Vec<_>>()
        .join("\n");
    if text.is_empty() { None } else { Some(text) }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_build_api_doc_includes_metadata_signature_and_return_docs() {
        let source = r#"
(module Geometry)
(defn add
  [x y]
  :doc "2 つの整数を加算する"
  :params [(x "左オペランド") (y "右オペランド")]
  :returns "加算結果"
  :example [(add 1 2)]
  (+ x y))
"#;

        let program = lsharp_syntax::parse(source).unwrap();
        let mut infer = Infer::new();
        let type_results = infer.infer_program(&program).unwrap();

        let api = build_api_doc("demo", "0.1.0", &program, &type_results, &infer);
        let module = api.modules.first().expect("module が必要");
        let function = module.functions.first().expect("function が必要");

        assert_eq!(api.package, "demo");
        assert_eq!(module.name, "Geometry");
        assert_eq!(function.name, "add");
        assert_eq!(function.signature, "Int -> Int -> Int");
        assert_eq!(function.params.len(), 2);
        assert_eq!(function.params[0].doc.as_deref(), Some("左オペランド"));
        assert_eq!(function.returns.doc.as_deref(), Some("加算結果"));
        assert_eq!(function.doc.as_deref(), Some("2 つの整数を加算する"));
        assert_eq!(function.example.as_deref(), Some("(add 1 2)"));
    }

    #[test]
    fn test_build_api_doc_serializes_modules_shape() {
        let source = "(module Sample) (defn main [] 42)";
        let program = lsharp_syntax::parse(source).unwrap();
        let mut infer = Infer::new();
        let type_results = infer.infer_program(&program).unwrap();

        let api = build_api_doc("sample", "0.1.0", &program, &type_results, &infer);
        let json = serde_json::to_string_pretty(&api).unwrap();

        assert!(json.contains("\"package\": \"sample\""));
        assert!(json.contains("\"modules\""));
        assert!(json.contains("\"functions\""));
        assert!(json.contains("\"signature\""));
    }

    #[test]
    fn test_build_api_doc_for_package_collects_modules_from_src_in_sorted_order() {
        let dir = std::env::temp_dir().join("lsharp_api_doc_package");
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(dir.join("src")).unwrap();
        std::fs::write(dir.join("src/Beta.ls"), "(module Beta)\n(defn beta [] 2)").unwrap();
        std::fs::write(
            dir.join("src/Alpha.ls"),
            "(module Alpha)\n(defn alpha [] 1)",
        )
        .unwrap();

        let api = build_api_doc_for_package(&dir, "demo", "0.1.0").unwrap();
        let names: Vec<&str> = api
            .modules
            .iter()
            .map(|module| module.name.as_str())
            .collect();

        assert_eq!(names, vec!["Alpha", "Beta"]);

        std::fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn test_build_api_doc_for_file_uses_file_stem_and_header_comment_for_module_metadata() {
        let dir = std::env::temp_dir().join("lsharp_api_doc_module_fallback");
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        let file = dir.join("Sample.ls");
        std::fs::write(
            &file,
            r#";; Sample.ls - 説明
;;
;; モジュール概要
(defn hello
  [name]
  :doc "挨拶を返す"
  :params [(name "対象名")]
  :returns "挨拶文字列"
  :example [(hello "L#")]
  name)
"#,
        )
        .unwrap();

        let api = build_api_doc_for_file("demo", "0.1.0", &file).unwrap();
        let module = api.modules.first().expect("module が必要");

        assert_eq!(module.name, "Sample");
        assert_eq!(module.doc.as_deref(), Some("モジュール概要"));

        std::fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn test_build_api_doc_for_stdlib_public_functions_have_metadata() {
        let stdlib_root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../stdlib");
        let mut public_functions = 0usize;

        for entry in std::fs::read_dir(&stdlib_root).unwrap() {
            let path = entry.unwrap().path();
            if path.extension().and_then(|ext| ext.to_str()) != Some("ls") {
                continue;
            }
            let api = build_api_doc_for_file("stdlib", "0.1.0", &path).unwrap();
            let module = api.modules.first().expect("module が必要");

            assert_ne!(module.name, "Main", "{} の module 名が不正", path.display());
            assert!(
                module.doc.is_some(),
                "{} の module doc が欠けている",
                path.display()
            );

            for function in &module.functions {
                assert_ne!(
                    function.name, "main",
                    "{} に main が公開 API として出ている",
                    module.name
                );
                assert!(
                    !function.name.ends_with("-impl"),
                    "{}::{} が内部 helper のまま公開されている",
                    module.name,
                    function.name
                );
                assert!(
                    function.doc.is_some(),
                    "{}::{} の :doc が欠けている",
                    module.name,
                    function.name
                );
                assert!(
                    function.params.iter().all(|param| param.doc.is_some()),
                    "{}::{} の :params が欠けている",
                    module.name,
                    function.name
                );
                assert!(
                    function.returns.doc.is_some(),
                    "{}::{} の :returns が欠けている",
                    module.name,
                    function.name
                );
                assert!(
                    function.example.is_some(),
                    "{}::{} の :example が欠けている",
                    module.name,
                    function.name
                );
                public_functions += 1;
            }
        }

        assert!(public_functions >= 40, "stdlib 公開関数数が少なすぎる");
    }

    #[test]
    fn test_build_api_doc_for_file_preserves_parse_error_code() {
        let dir =
            std::env::temp_dir().join(format!("lsharp_api_doc_diagnostic_{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).expect("api diagnostic directory を作成できる");
        let file = dir.join("Broken.ls");
        std::fs::write(&file, "(").expect("api diagnostic fixture を書き込める");

        let error = build_api_doc_for_file("demo", "0.1.0", &file)
            .expect_err("壊れた source は API doc 生成を失敗させるべき");
        assert!(
            error.to_string().contains("[LS0103]"),
            "API doc diagnostics は stable code を含むべき: {error}"
        );

        std::fs::remove_dir_all(&dir).expect("api diagnostic directory を削除できる");
    }
}
