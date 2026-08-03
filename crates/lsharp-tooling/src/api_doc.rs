use lsharp_syntax::ast::Program;
use lsharp_types::{infer::Infer, types::TypeScheme};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

use crate::diagnostics::driver_io_error;

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
    let source = std::fs::read_to_string(file)
        .map_err(|e| driver_io_error(format!("{}: {}", file.display(), e)))?;
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
    if std::fs::symlink_metadata(&source_root)
        .map(|metadata| metadata.file_type().is_symlink())
        .unwrap_or(false)
    {
        return Err(driver_io_error(format!(
            "{}: package src must be a regular non-symlink directory",
            source_root.display()
        )));
    }
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
        std::fs::read_dir(dir).map_err(|e| driver_io_error(format!("{}: {}", dir.display(), e)))?;
    for entry in entries {
        let entry = entry.map_err(|e| driver_io_error(format!("{}: {}", dir.display(), e)))?;
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
#[path = "api_doc_tests.rs"]
mod tests;
