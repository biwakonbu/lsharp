use crate::api_doc::{self, ApiDoc, ApiModule};
use std::fs;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, PartialEq, Eq)]
struct GuidePage {
    slug: String,
    title: String,
    markdown: String,
}

pub fn cmd_doc_site(output: &Path) -> miette::Result<()> {
    let repo_root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..");
    cmd_doc_site_in(&repo_root, output)
}

pub(crate) fn cmd_doc_site_in(repo_root: &Path, output: &Path) -> miette::Result<()> {
    let guides_src = repo_root.join("docs/guides");
    let stdlib_root = repo_root.join("stdlib");
    let guides_out = output.join("guides");
    let api_out = output.join("api");
    let guides = collect_guides(&guides_src)?;

    fs::create_dir_all(&guides_out)
        .map_err(|e| miette::miette!("{}: {}", guides_out.display(), e))?;
    fs::create_dir_all(&api_out).map_err(|e| miette::miette!("{}: {}", api_out.display(), e))?;

    let guide_nav = guide_links(&guides, "");
    let guide_nav_parent = guide_links(&guides, "../guides/");

    for guide in &guides {
        let html = render_page(
            &guide.title,
            &render_markdown(&guide.markdown),
            "../index.html",
            &guide_nav,
        );
        fs::write(guides_out.join(format!("{}.html", guide.slug)), html)
            .map_err(|e| miette::miette!("guide 出力失敗: {e}"))?;
    }

    let stdlib_api = build_stdlib_api(&stdlib_root)?;
    let stdlib_json = serde_json::to_string_pretty(&stdlib_api)
        .map_err(|e| miette::miette!("stdlib.json 直列化失敗: {e}"))?;
    fs::write(api_out.join("stdlib.json"), stdlib_json)
        .map_err(|e| miette::miette!("stdlib.json 出力失敗: {e}"))?;

    for module in &stdlib_api.modules {
        let html = render_page(
            &format!("API: {}", module.name),
            &render_api_module(module),
            "../index.html",
            &guide_nav_parent,
        );
        fs::write(api_out.join(format!("{}.html", module.name)), html)
            .map_err(|e| miette::miette!("API ページ出力失敗: {e}"))?;
    }

    let index_html = render_page(
        "L# Documentation",
        &render_index(&guides, &stdlib_api),
        "index.html",
        &guide_links(&guides, "guides/"),
    );
    fs::write(output.join("index.html"), index_html)
        .map_err(|e| miette::miette!("index.html 出力失敗: {e}"))?;

    println!("Language reference ... ok");
    println!("Stdlib API ({} modules) ... ok", stdlib_api.modules.len());
    println!("Guides ({} pages) ... ok", guides.len());
    println!("Site generated: {}", output.display());

    Ok(())
}

fn collect_guides(guides_src: &Path) -> miette::Result<Vec<GuidePage>> {
    let mut guides = Vec::new();
    for entry in
        fs::read_dir(guides_src).map_err(|e| miette::miette!("{}: {}", guides_src.display(), e))?
    {
        let path = entry
            .map_err(|e| miette::miette!("{}: {}", guides_src.display(), e))?
            .path();
        if path.extension().and_then(|ext| ext.to_str()) != Some("md") {
            continue;
        }
        if path.file_name().and_then(|name| name.to_str()) == Some("README.md") {
            continue;
        }

        let markdown =
            fs::read_to_string(&path).map_err(|e| miette::miette!("{}: {}", path.display(), e))?;
        let slug = path
            .file_stem()
            .and_then(|stem| stem.to_str())
            .ok_or_else(|| miette::miette!("guide slug を解決できません: {}", path.display()))?
            .to_string();
        let title = guide_title(&slug, &markdown);
        guides.push(GuidePage {
            slug,
            title,
            markdown,
        });
    }
    guides.sort_by_key(|guide| guide_sort_key(&guide.slug));
    Ok(guides)
}

fn guide_sort_key(slug: &str) -> (u8, String) {
    let priority = match slug {
        "quick-start" => 0,
        "language-reference" => 1,
        "package-layout" => 2,
        _ => 10,
    };
    (priority, slug.to_string())
}

fn guide_title(slug: &str, markdown: &str) -> String {
    match slug {
        "quick-start" => "Quick Start".to_string(),
        "language-reference" => "Language Reference".to_string(),
        "package-layout" => "Package Layout".to_string(),
        _ => markdown
            .lines()
            .find_map(|line| line.strip_prefix("# ").map(str::trim))
            .map(str::to_string)
            .unwrap_or_else(|| slug.replace('-', " ")),
    }
}

fn guide_links(guides: &[GuidePage], prefix: &str) -> Vec<(String, String)> {
    guides
        .iter()
        .map(|guide| (format!("{prefix}{}.html", guide.slug), guide.title.clone()))
        .collect()
}

fn build_stdlib_api(stdlib_root: &Path) -> miette::Result<ApiDoc> {
    let mut files = Vec::new();
    for entry in fs::read_dir(stdlib_root)
        .map_err(|e| miette::miette!("{}: {}", stdlib_root.display(), e))?
    {
        let path = entry
            .map_err(|e| miette::miette!("{}: {}", stdlib_root.display(), e))?
            .path();
        if path.extension().and_then(|ext| ext.to_str()) == Some("ls") {
            files.push(path);
        }
    }
    files.sort();

    let version = env!("CARGO_PKG_VERSION");
    let mut modules = Vec::new();
    for file in files {
        let mut doc = api_doc::build_api_doc_for_file("stdlib", version, &file)?;
        if let Some(module) = doc.modules.pop() {
            modules.push(module);
        }
    }
    modules.sort_by(|left, right| left.name.cmp(&right.name));

    Ok(ApiDoc {
        package: "stdlib".to_string(),
        version: version.to_string(),
        modules,
    })
}

fn render_index(guides: &[GuidePage], stdlib_api: &ApiDoc) -> String {
    let mut body = String::new();
    body.push_str("<h1>L# Documentation</h1>\n");
    body.push_str("<p>L# のガイドと標準ライブラリ API をまとめた静的サイトです。</p>\n");
    body.push_str("<h2>Guides</h2>\n<ul>\n");
    for guide in guides {
        body.push_str(&format!(
            "<li><a href=\"guides/{}.html\">{}</a></li>\n",
            escape_html(&guide.slug),
            escape_html(&guide.title)
        ));
    }
    body.push_str("</ul>\n<h2>Stdlib API</h2>\n<ul>\n");
    for module in &stdlib_api.modules {
        body.push_str(&format!(
            "<li><a href=\"api/{}.html\">{}</a></li>\n",
            escape_html(&module.name),
            escape_html(&module.name)
        ));
    }
    body.push_str("</ul>\n");
    body
}

fn render_api_module(module: &ApiModule) -> String {
    let mut body = String::new();
    body.push_str(&format!("<h1>{}</h1>\n", escape_html(&module.name)));
    if let Some(doc) = &module.doc {
        body.push_str(&format!("<p>{}</p>\n", escape_html(doc)));
    }

    body.push_str("<h2>Functions</h2>\n");
    for function in &module.functions {
        body.push_str(&format!(
            "<section><h3>{}</h3>\n",
            escape_html(&function.name)
        ));
        body.push_str(&format!(
            "<pre><code>{}</code></pre>\n",
            escape_html(&function.signature)
        ));
        if let Some(doc) = &function.doc {
            body.push_str(&format!("<p>{}</p>\n", escape_html(doc)));
        }
        if !function.params.is_empty() {
            body.push_str("<h4>Parameters</h4><ul>\n");
            for param in &function.params {
                body.push_str(&format!(
                    "<li><code>{}</code>: {} ({})</li>\n",
                    escape_html(&param.name),
                    escape_html(param.doc.as_deref().unwrap_or("")),
                    escape_html(&param.ty)
                ));
            }
            body.push_str("</ul>\n");
        }
        body.push_str(&format!(
            "<p><strong>Returns:</strong> {} ({})</p>\n",
            escape_html(function.returns.doc.as_deref().unwrap_or("")),
            escape_html(&function.returns.ty)
        ));
        if let Some(example) = &function.example {
            body.push_str(&format!(
                "<pre><code>{}</code></pre>\n",
                escape_html(example)
            ));
        }
        body.push_str("</section>\n");
    }

    body.push_str("<h2>Types</h2>\n<ul>\n");
    for ty in &module.types {
        body.push_str(&format!(
            "<li><code>{}</code> ({})</li>\n",
            escape_html(&ty.name),
            escape_html(&ty.kind)
        ));
    }
    body.push_str("</ul>\n");
    body
}

fn render_page(
    title: &str,
    body: &str,
    home_href: &str,
    guide_links: &[(String, String)],
) -> String {
    let mut nav = format!("<a href=\"{}\">Home</a>", escape_html(home_href));
    for (href, label) in guide_links {
        nav.push_str(&format!(
            "<a href=\"{}\">{}</a>",
            escape_html(href),
            escape_html(label)
        ));
    }
    format!(
        "<!DOCTYPE html>\n<html lang=\"ja\"><head><meta charset=\"utf-8\"><title>{}</title><style>body{{font-family:-apple-system,BlinkMacSystemFont,\"Segoe UI\",sans-serif;max-width:960px;margin:0 auto;padding:40px;line-height:1.6;color:#222;background:#faf8f3}}nav a{{margin-right:16px}}pre{{background:#f1ece2;padding:16px;border-radius:8px;overflow:auto}}code{{font-family:ui-monospace,SFMono-Regular,Menlo,monospace}}section{{padding-bottom:16px;border-bottom:1px solid #ddd;margin-bottom:16px}}ul{{padding-left:20px}}</style></head><body><nav>{}</nav>{}</body></html>\n",
        escape_html(title),
        nav,
        body
    )
}

fn render_markdown(markdown: &str) -> String {
    let mut html = String::new();
    let mut paragraph = Vec::new();
    let mut in_list = false;
    let mut in_code = false;

    for line in markdown.lines() {
        let trimmed = line.trim_end();
        if trimmed.starts_with("```") {
            flush_paragraph(&mut html, &mut paragraph);
            if in_list {
                html.push_str("</ul>\n");
                in_list = false;
            }
            if in_code {
                html.push_str("</code></pre>\n");
            } else {
                html.push_str("<pre><code>");
            }
            in_code = !in_code;
            continue;
        }

        if in_code {
            html.push_str(&escape_html(trimmed));
            html.push('\n');
            continue;
        }

        if let Some(text) = trimmed.strip_prefix("# ") {
            flush_paragraph(&mut html, &mut paragraph);
            if in_list {
                html.push_str("</ul>\n");
                in_list = false;
            }
            html.push_str(&format!("<h1>{}</h1>\n", render_inline(text)));
            continue;
        }

        if let Some(text) = trimmed.strip_prefix("## ") {
            flush_paragraph(&mut html, &mut paragraph);
            if in_list {
                html.push_str("</ul>\n");
                in_list = false;
            }
            html.push_str(&format!("<h2>{}</h2>\n", render_inline(text)));
            continue;
        }

        if let Some(text) = trimmed.strip_prefix("### ") {
            flush_paragraph(&mut html, &mut paragraph);
            if in_list {
                html.push_str("</ul>\n");
                in_list = false;
            }
            html.push_str(&format!("<h3>{}</h3>\n", render_inline(text)));
            continue;
        }

        if let Some(text) = trimmed.strip_prefix("- ") {
            flush_paragraph(&mut html, &mut paragraph);
            if !in_list {
                html.push_str("<ul>\n");
                in_list = true;
            }
            html.push_str(&format!("<li>{}</li>\n", render_inline(text)));
            continue;
        }

        if trimmed.is_empty() {
            flush_paragraph(&mut html, &mut paragraph);
            if in_list {
                html.push_str("</ul>\n");
                in_list = false;
            }
            continue;
        }

        paragraph.push(trimmed.trim().to_string());
    }

    flush_paragraph(&mut html, &mut paragraph);
    if in_list {
        html.push_str("</ul>\n");
    }
    if in_code {
        html.push_str("</code></pre>\n");
    }
    html
}

fn flush_paragraph(html: &mut String, paragraph: &mut Vec<String>) {
    if paragraph.is_empty() {
        return;
    }
    let text = paragraph.join(" ");
    html.push_str(&format!("<p>{}</p>\n", render_inline(&text)));
    paragraph.clear();
}

fn render_inline(text: &str) -> String {
    let mut result = String::new();
    let mut in_code = false;

    for segment in text.split('`') {
        if in_code {
            result.push_str("<code>");
            result.push_str(&escape_html(segment));
            result.push_str("</code>");
        } else {
            result.push_str(&escape_html(segment));
        }
        in_code = !in_code;
    }

    if text.matches('`').count() % 2 == 1 {
        result.push('`');
    }

    result
}

fn escape_html(text: &str) -> String {
    text.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_language_guides_and_agent_skill_template_exist() {
        let repo_root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..");

        let quick_start = repo_root.join("docs/guides/quick-start.md");
        let language_reference = repo_root.join("docs/guides/language-reference.md");
        let package_layout = repo_root.join("docs/guides/package-layout.md");
        let skill_template =
            repo_root.join("crates/lsharp-driver/templates/lsharp-language-guide.md");

        assert!(quick_start.exists(), "quick-start.md が必要");
        assert!(language_reference.exists(), "language-reference.md が必要");
        assert!(package_layout.exists(), "package-layout.md が必要");
        assert!(skill_template.exists(), "Agent Skills テンプレートが必要");
    }

    #[test]
    fn test_cmd_doc_site_generates_guides_and_api_site() {
        let repo_root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..");
        let output = std::env::temp_dir().join("lsharp_doc_site");
        let _ = std::fs::remove_dir_all(&output);

        let result = cmd_doc_site_in(&repo_root, &output);
        assert!(result.is_ok(), "doc-site は成功するべき: {result:?}");

        assert!(output.join("index.html").exists(), "index.html が必要");
        assert!(
            output.join("guides/quick-start.html").exists(),
            "quick-start.html が必要"
        );
        assert!(
            output.join("guides/language-reference.html").exists(),
            "language-reference.html が必要"
        );
        assert!(
            output.join("guides/package-layout.html").exists(),
            "package-layout.html が必要"
        );
        assert!(output.join("api/Core.html").exists(), "Core.html が必要");
        assert!(
            output.join("api/stdlib.json").exists(),
            "stdlib.json が必要"
        );

        let index = std::fs::read_to_string(output.join("index.html")).unwrap();
        assert!(index.contains("Quick Start"));
        assert!(index.contains("Package Layout"));
        assert!(index.contains("Core"));

        let api = std::fs::read_to_string(output.join("api/stdlib.json")).unwrap();
        assert!(api.contains("\"package\": \"stdlib\""));
        assert!(api.contains("\"name\": \"Core\""));

        std::fs::remove_dir_all(&output).unwrap();
    }
}
