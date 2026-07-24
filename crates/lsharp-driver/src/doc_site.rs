use crate::api_doc::{self, ApiDoc, ApiModule};
use crate::error_codes::driver_io_error;
use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use std::fs;
use std::path::{Component, Path, PathBuf};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
struct SiteManifest {
    title: String,
    description: String,
    source: String,
    base_url: String,
    sections: Vec<SiteSection>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
struct SiteSection {
    id: String,
    title: String,
    description: String,
    pages: Vec<SitePage>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
struct SitePage {
    title: String,
    source: String,
    output: String,
    summary: String,
    audience: String,
}

impl SiteManifest {
    fn all_pages(&self) -> Vec<&SitePage> {
        self.sections
            .iter()
            .flat_map(|section| section.pages.iter())
            .collect()
    }
}

pub fn cmd_doc_site(output: &Path) -> miette::Result<()> {
    let repo_root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..");
    cmd_doc_site_in(&repo_root, output)
}

pub(crate) fn cmd_doc_site_in(repo_root: &Path, output: &Path) -> miette::Result<()> {
    let stdlib_root = repo_root.join("stdlib");
    let api_out = output.join("api");
    let manifest = load_site_manifest(repo_root)?;

    fs::create_dir_all(output)
        .map_err(|e| driver_io_error(format!("{}: {}", output.display(), e)))?;
    fs::create_dir_all(&api_out)
        .map_err(|e| driver_io_error(format!("{}: {}", api_out.display(), e)))?;

    for page in manifest.all_pages() {
        let source_path = repo_root.join(&page.source);
        let markdown = fs::read_to_string(&source_path)
            .map_err(|e| driver_io_error(format!("{}: {}", source_path.display(), e)))?;
        let output_path = output.join(&page.output);
        if let Some(parent) = output_path.parent() {
            fs::create_dir_all(parent)
                .map_err(|e| driver_io_error(format!("{}: {}", parent.display(), e)))?;
        }

        let html = render_page(
            &manifest,
            &page.title,
            &render_markdown(&markdown),
            &home_href(&page.output),
            &site_nav_links(&manifest, &page.output),
        );
        fs::write(&output_path, html).map_err(|e| {
            driver_io_error(format!(
                "site page 出力失敗: {}: {e}",
                output_path.display()
            ))
        })?;
    }

    let stdlib_api = build_stdlib_api(&stdlib_root)?;
    let stdlib_json = serde_json::to_string_pretty(&stdlib_api)
        .map_err(|e| miette::miette!("stdlib.json 直列化失敗: {e}"))?;
    fs::write(api_out.join("stdlib.json"), stdlib_json)
        .map_err(|e| driver_io_error(format!("stdlib.json 出力失敗: {e}")))?;

    for module in &stdlib_api.modules {
        let output_name = format!("api/{}.html", module.name);
        let html = render_page(
            &manifest,
            &format!("API: {}", module.name),
            &render_api_module(module),
            &home_href(&output_name),
            &site_nav_links(&manifest, &output_name),
        );
        fs::write(api_out.join(format!("{}.html", module.name)), html)
            .map_err(|e| driver_io_error(format!("API ページ出力失敗: {e}")))?;
    }

    let index_html = render_page(
        &manifest,
        &manifest.title,
        &render_index(&manifest, &stdlib_api),
        "index.html",
        &site_nav_links_from_index(&manifest),
    );
    fs::write(output.join("index.html"), index_html)
        .map_err(|e| driver_io_error(format!("index.html 出力失敗: {e}")))?;
    fs::write(output.join(".nojekyll"), "")
        .map_err(|e| driver_io_error(format!(".nojekyll 出力失敗: {e}")))?;
    fs::write(
        output.join("sitemap.xml"),
        render_sitemap(&manifest, &stdlib_api),
    )
    .map_err(|e| driver_io_error(format!("sitemap.xml 出力失敗: {e}")))?;
    let manifest_json = serde_json::to_string_pretty(&manifest)
        .map_err(|e| miette::miette!("docs-site-manifest.json 直列化失敗: {e}"))?;
    fs::write(output.join("docs-site-manifest.json"), manifest_json)
        .map_err(|e| driver_io_error(format!("docs-site-manifest.json 出力失敗: {e}")))?;

    println!("Site manifest ({}) ... ok", manifest.source);
    println!("Pages ({} pages) ... ok", manifest.all_pages().len());
    println!("Stdlib API ({} modules) ... ok", stdlib_api.modules.len());
    println!("Site generated: {}", output.display());

    Ok(())
}

fn load_site_manifest(repo_root: &Path) -> miette::Result<SiteManifest> {
    let manifest_path = repo_root.join("docs/site.toml");
    let text = fs::read_to_string(&manifest_path)
        .map_err(|e| driver_io_error(format!("{}: {}", manifest_path.display(), e)))?;
    let manifest: SiteManifest =
        toml::from_str(&text).map_err(|e| miette::miette!("docs/site.toml の読み込み失敗: {e}"))?;
    validate_site_manifest(repo_root, &manifest)?;
    Ok(manifest)
}

fn validate_site_manifest(repo_root: &Path, manifest: &SiteManifest) -> miette::Result<()> {
    if manifest.source != "docs/site.toml" {
        return Err(miette::miette!(
            "docs/site.toml: source は docs/site.toml で固定してください"
        ));
    }

    let mut section_ids = HashSet::new();
    let mut sources = HashSet::new();
    let mut outputs = HashSet::new();

    for section in &manifest.sections {
        if section.id.trim().is_empty() || section.title.trim().is_empty() {
            return Err(miette::miette!(
                "docs/site.toml: section id/title は必須です"
            ));
        }
        if !section_ids.insert(section.id.as_str()) {
            return Err(miette::miette!(
                "docs/site.toml: section id が重複しています: {}",
                section.id
            ));
        }
        if section.pages.is_empty() {
            return Err(miette::miette!(
                "docs/site.toml: section に page がありません: {}",
                section.id
            ));
        }

        for page in &section.pages {
            let source = Path::new(&page.source);
            let output = Path::new(&page.output);
            if !is_safe_relative_path(source) || !is_safe_relative_path(output) {
                return Err(miette::miette!(
                    "docs/site.toml: source/output は repo 相対パスで指定してください: {} -> {}",
                    page.source,
                    page.output
                ));
            }
            if output.extension().and_then(|ext| ext.to_str()) != Some("html") {
                return Err(miette::miette!(
                    "docs/site.toml: output は .html で終わる必要があります: {}",
                    page.output
                ));
            }
            if !repo_root.join(source).exists() {
                return Err(miette::miette!(
                    "docs/site.toml: source が存在しません: {}",
                    page.source
                ));
            }
            if !sources.insert(page.source.as_str()) {
                return Err(miette::miette!(
                    "docs/site.toml: source が重複しています: {}",
                    page.source
                ));
            }
            if !outputs.insert(page.output.as_str()) {
                return Err(miette::miette!(
                    "docs/site.toml: output が重複しています: {}",
                    page.output
                ));
            }
        }
    }

    Ok(())
}

fn is_safe_relative_path(path: &Path) -> bool {
    !path.is_absolute()
        && path
            .components()
            .all(|component| matches!(component, Component::Normal(_) | Component::CurDir))
}

fn build_stdlib_api(stdlib_root: &Path) -> miette::Result<ApiDoc> {
    let mut files = Vec::new();
    for entry in fs::read_dir(stdlib_root)
        .map_err(|e| driver_io_error(format!("{}: {}", stdlib_root.display(), e)))?
    {
        let path = entry
            .map_err(|e| driver_io_error(format!("{}: {}", stdlib_root.display(), e)))?
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

fn render_index(manifest: &SiteManifest, stdlib_api: &ApiDoc) -> String {
    let mut body = String::new();
    body.push_str(&format!("<h1>{}</h1>\n", escape_html(&manifest.title)));
    body.push_str(&format!("<p>{}</p>\n", escape_html(&manifest.description)));
    body.push_str(&format!(
        "<p><strong>SSOT:</strong> <code>{}</code> がサイト構成の正本です。本文は各 Markdown と <code>stdlib/*.ls</code> の metadata を正本にします。</p>\n",
        escape_html(&manifest.source)
    ));

    for section in &manifest.sections {
        body.push_str(&format!("<h2>{}</h2>\n", escape_html(&section.title)));
        body.push_str(&format!(
            "<p>{}</p>\n<ul>\n",
            escape_html(&section.description)
        ));
        for page in &section.pages {
            body.push_str(&format!(
                "<li><a href=\"{}\">{}</a><br><span>{}</span></li>\n",
                escape_html(&page.output),
                escape_html(&page.title),
                escape_html(&page.summary)
            ));
        }
        body.push_str("</ul>\n");
    }

    body.push_str("<h2>Stdlib API</h2>\n<p><code>stdlib/*.ls</code> の <code>:doc</code> metadata から生成します。</p>\n<ul>\n");
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
    manifest: &SiteManifest,
    title: &str,
    body: &str,
    home_href: &str,
    nav_links: &[(String, String)],
) -> String {
    let mut nav = format!("<a href=\"{}\">Home</a>", escape_html(home_href));
    for (href, label) in nav_links {
        nav.push_str(&format!(
            "<a href=\"{}\">{}</a>",
            escape_html(href),
            escape_html(label)
        ));
    }
    format!(
        "<!DOCTYPE html>\n<html lang=\"ja\"><head><meta charset=\"utf-8\"><meta name=\"viewport\" content=\"width=device-width, initial-scale=1\"><title>{}</title><style>{}</style></head><body><header><div class=\"brand\">{}</div><nav>{}</nav></header><main data-source=\"{}\">{}</main></body></html>\n",
        escape_html(title),
        site_css(),
        escape_html(&manifest.title),
        nav,
        escape_html(&manifest.source),
        body
    )
}

fn home_href(current_output: &str) -> String {
    format!("{}index.html", parent_prefix(current_output))
}

fn site_nav_links_from_index(manifest: &SiteManifest) -> Vec<(String, String)> {
    manifest
        .sections
        .iter()
        .filter_map(|section| {
            section
                .pages
                .first()
                .map(|page| (page.output.clone(), section.title.clone()))
        })
        .collect()
}

fn site_nav_links(manifest: &SiteManifest, current_output: &str) -> Vec<(String, String)> {
    let prefix = parent_prefix(current_output);
    manifest
        .sections
        .iter()
        .filter_map(|section| {
            section
                .pages
                .first()
                .map(|page| (format!("{prefix}{}", page.output), section.title.clone()))
        })
        .collect()
}

fn parent_prefix(output: &str) -> String {
    let depth = Path::new(output)
        .parent()
        .map(|parent| {
            parent
                .components()
                .filter(|component| matches!(component, Component::Normal(_)))
                .count()
        })
        .unwrap_or(0);
    "../".repeat(depth)
}

fn render_sitemap(manifest: &SiteManifest, stdlib_api: &ApiDoc) -> String {
    let mut paths = vec!["index.html".to_string()];
    paths.extend(manifest.all_pages().iter().map(|page| page.output.clone()));
    paths.extend(
        stdlib_api
            .modules
            .iter()
            .map(|module| format!("api/{}.html", module.name)),
    );
    paths.sort();

    let base_url = manifest.base_url.trim_end_matches('/');
    let mut xml = String::from("<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n");
    xml.push_str("<urlset xmlns=\"http://www.sitemaps.org/schemas/sitemap/0.9\">\n");
    for path in paths {
        xml.push_str("  <url><loc>");
        xml.push_str(&escape_xml(&format!("{base_url}/{path}")));
        xml.push_str("</loc></url>\n");
    }
    xml.push_str("</urlset>\n");
    xml
}

fn site_css() -> &'static str {
    "body{font-family:-apple-system,BlinkMacSystemFont,\"Segoe UI\",sans-serif;margin:0;line-height:1.65;color:#202124;background:#fbfaf7}header{position:sticky;top:0;background:#fffdf8;border-bottom:1px solid #e6dfd2;padding:14px 32px;display:flex;align-items:center;gap:24px;z-index:1}.brand{font-weight:700;white-space:nowrap}nav{display:flex;gap:14px;flex-wrap:wrap}nav a{color:#3d4f7a;text-decoration:none}main{max-width:1040px;margin:0 auto;padding:40px 32px 72px}h1{font-size:2.2rem;line-height:1.2;margin:0 0 18px}h2{margin-top:36px;border-top:1px solid #e6dfd2;padding-top:28px}h3{margin-top:28px}a{color:#234f9d}pre{background:#f1ece2;padding:16px;border-radius:8px;overflow:auto}code{font-family:ui-monospace,SFMono-Regular,Menlo,monospace}section{padding-bottom:16px;border-bottom:1px solid #ddd;margin-bottom:16px}ul{padding-left:22px}li{margin:8px 0}span{color:#5f6368}@media(max-width:720px){header{position:static;align-items:flex-start;flex-direction:column;padding:16px 20px}main{padding:28px 20px 56px}h1{font-size:1.8rem}}"
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

fn escape_xml(text: &str) -> String {
    escape_html(text).replace('"', "&quot;")
}

#[cfg(test)]
mod tests;
