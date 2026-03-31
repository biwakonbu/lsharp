use std::path::Path;

/// `doc` コマンド向けの HTML ドキュメントを生成する。
pub fn render_doc_html(file: &Path) -> miette::Result<String> {
    let source =
        std::fs::read_to_string(file).map_err(|e| miette::miette!("{}: {}", file.display(), e))?;
    let program = lsharp_syntax::parse(&source).map_err(|e| miette::miette!("{e}"))?;
    let mut infer = lsharp_types::infer::Infer::new();
    let type_results = infer
        .infer_program(&program)
        .map_err(|e| miette::miette!("{e}"))?;

    let mut html = String::new();
    html.push_str("<!DOCTYPE html>\n<html><head><meta charset=\"utf-8\">\n");
    html.push_str(&format!("<title>L# API - {}</title>\n", file.display()));
    html.push_str("<style>body{font-family:sans-serif;max-width:800px;margin:0 auto;padding:20px}\n");
    html.push_str("h1{color:#333}h2{color:#555;border-bottom:1px solid #ddd;padding-bottom:5px}\n");
    html.push_str(".sig{background:#f5f5f5;padding:8px;border-radius:4px;font-family:monospace}\n");
    html.push_str(".doc{color:#666;margin:8px 0}.params{margin-left:20px}\n");
    html.push_str("</style></head><body>\n");
    html.push_str(&format!(
        "<h1>{}</h1>\n",
        file.file_stem().unwrap_or_default().to_string_lossy()
    ));

    for decl in &program.decls {
        match decl {
            lsharp_syntax::ast::Decl::Defn {
                name,
                params,
                return_ty,
                metadata,
                ..
            } => {
                html.push_str(&format!("<h2>{}</h2>\n", name));

                let param_strs: Vec<String> = params.iter().map(|param| param.name.clone()).collect();
                let ret = return_ty
                    .as_ref()
                    .map_or("?".to_string(), |ty| format!("{ty:?}"));
                let type_str = type_results
                    .iter()
                    .find(|(candidate, _)| candidate == name)
                    .map(|(_, scheme)| format!("{}", scheme))
                    .unwrap_or_else(|| format!("({}) -> {}", param_strs.join(", "), ret));
                html.push_str(&format!("<div class=\"sig\">{}: {}</div>\n", name, type_str));

                if let Some(meta) = metadata {
                    if let Some(doc) = &meta.doc {
                        html.push_str(&format!("<div class=\"doc\">{}</div>\n", doc));
                    }
                    if !meta.params.is_empty() {
                        html.push_str("<div class=\"params\"><strong>パラメータ:</strong><ul>\n");
                        for (param_name, param_doc) in &meta.params {
                            html.push_str(&format!(
                                "<li><code>{}</code> - {}</li>\n",
                                param_name, param_doc
                            ));
                        }
                        html.push_str("</ul></div>\n");
                    }
                    if let Some(ret_doc) = &meta.returns {
                        html.push_str(&format!(
                            "<div class=\"doc\"><strong>戻り値:</strong> {}</div>\n",
                            ret_doc
                        ));
                    }
                }
            }
            lsharp_syntax::ast::Decl::TypeDef {
                name,
                type_params,
                variants,
                metadata,
                ..
            } => {
                html.push_str(&format!("<h2>type {}</h2>\n", name));
                if !type_params.is_empty() {
                    html.push_str(&format!(
                        "<div class=\"sig\">type ({} {})</div>\n",
                        name,
                        type_params.join(" ")
                    ));
                }
                html.push_str("<ul>\n");
                for variant in variants {
                    html.push_str(&format!("<li><code>{}</code></li>\n", variant.name));
                }
                html.push_str("</ul>\n");
                if let Some(meta) = metadata
                    && let Some(doc) = &meta.doc
                {
                    html.push_str(&format!("<div class=\"doc\">{}</div>\n", doc));
                }
            }
            _ => {}
        }
    }

    html.push_str("</body></html>\n");
    Ok(html)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::{
        fs,
        path::PathBuf,
        time::{SystemTime, UNIX_EPOCH},
    };

    fn unique_temp_dir(name: &str) -> PathBuf {
        let nonce = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("system time before unix epoch")
            .as_nanos();
        let dir = std::env::temp_dir().join(format!(
            "lsharp_tooling_doc_html_{name}_{}_{}",
            std::process::id(),
            nonce
        ));
        fs::create_dir_all(&dir).expect("temp dir creation failed");
        dir
    }

    #[test]
    fn test_render_doc_html_includes_metadata_sections() {
        let dir = unique_temp_dir("metadata");
        let file = dir.join("Main.ls");
        fs::write(
            &file,
            r#"(defn greet
  [name]
  :doc "挨拶を返す"
  :params [(name "名前")]
  :returns "挨拶文字列"
  name)
"#,
        )
        .unwrap();

        let html = render_doc_html(&file).expect("HTML generation should succeed");
        assert!(html.contains("<h2>greet</h2>"));
        assert!(html.contains("挨拶を返す"));
        assert!(html.contains("<code>name</code> - 名前"));
        assert!(html.contains("<strong>戻り値:</strong> 挨拶文字列"));

        let _ = fs::remove_dir_all(&dir);
    }
}
