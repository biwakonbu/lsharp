//! doc site の回帰テスト

use super::*;

#[test]
fn test_language_guides_and_agent_skill_template_exist() {
    let repo_root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..");

    let quick_start = repo_root.join("docs/guides/quick-start.md");
    let language_reference = repo_root.join("docs/guides/language-reference.md");
    let package_layout = repo_root.join("docs/guides/package-layout.md");
    let skill_template = repo_root.join("crates/lsharp-driver/templates/lsharp-language-guide.md");

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
    assert!(
        output
            .join("guides/metadata-driven-development.html")
            .exists(),
        "metadata-driven-development.html が必要"
    );
    assert!(
        output.join("guides/ide-setup.html").exists(),
        "ide-setup.html が必要"
    );
    assert!(
        output.join("guides/deployment-targets.html").exists(),
        "deployment-targets.html が必要"
    );
    assert!(
        output.join("guides/stdlib-guide.html").exists(),
        "stdlib-guide.html が必要"
    );
    assert!(
        output.join("guides/error-reference.html").exists(),
        "error-reference.html が必要"
    );
    assert!(
        output.join("guides/examples.html").exists(),
        "examples.html が必要"
    );
    assert!(output.join("api/Core.html").exists(), "Core.html が必要");
    assert!(
        output.join("api/stdlib.json").exists(),
        "stdlib.json が必要"
    );

    let index = std::fs::read_to_string(output.join("index.html")).unwrap();
    assert!(index.contains("Quick Start"));
    assert!(index.contains("Package Layout"));
    assert!(index.contains("Examples Matrix"));
    assert!(index.contains("Core"));

    let api = std::fs::read_to_string(output.join("api/stdlib.json")).unwrap();
    assert!(api.contains("\"package\": \"stdlib\""));
    assert!(api.contains("\"name\": \"Core\""));

    std::fs::remove_dir_all(&output).unwrap();
}

#[test]
fn test_cmd_doc_site_missing_manifest_preserves_driver_io_error_code() {
    let repo_root = std::env::temp_dir().join(format!(
        "lsharp_doc_site_missing_manifest_{}",
        std::process::id()
    ));
    let output = repo_root.join("generated");
    let _ = std::fs::remove_dir_all(&repo_root);

    let error = cmd_doc_site_in(&repo_root, &output)
        .expect_err("存在しない docs/site.toml は doc site 生成を失敗させるべき");

    assert!(
        error.to_string().starts_with("[LS5001]"),
        "manifest の読み込み失敗は driver I/O code を保持するべき: {error}"
    );

    let _ = std::fs::remove_dir_all(&repo_root);
}

#[test]
fn test_doc_site_manifest_is_single_source_of_truth() {
    let repo_root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..");
    let manifest = load_site_manifest(&repo_root).expect("docs/site.toml should load");
    let pages = manifest.all_pages();

    assert_eq!(manifest.source, "docs/site.toml");
    assert!(
        pages
            .iter()
            .any(|page| page.source == "docs/guides/quick-start.md"
                && page.output == "guides/quick-start.html"),
        "quick-start は manifest から生成対象になるべき"
    );
    assert!(
        pages
            .iter()
            .any(|page| page.source == "book/ch01-introduction.md"
                && page.output == "book/introduction.html"),
        "book の入口も manifest から生成対象になるべき"
    );
    assert!(
        pages.iter().any(
            |page| page.source == "docs/development/operations/documentation-site.md"
                && page.output == "operations/documentation-site.html"
        ),
        "公開運用手順も manifest から生成対象になるべき"
    );
}

#[test]
fn test_doc_site_manifest_exposes_examples_matrix() {
    let repo_root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..");
    let manifest = load_site_manifest(&repo_root).expect("docs/site.toml should load");
    let pages = manifest.all_pages();

    assert!(
        pages.iter().any(|page| page.title == "Examples Matrix"
            && page.source == "docs/guides/examples.md"
            && page.output == "guides/examples.html"),
        "examples matrix は公開 guide として manifest から生成対象になるべき"
    );
}

#[test]
fn test_doc_site_manifest_exposes_user_guide_expansion() {
    let repo_root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..");
    let manifest = load_site_manifest(&repo_root).expect("docs/site.toml should load");
    let pages = manifest.all_pages();

    for (title, source, output) in [
        (
            "Metadata-Driven Development",
            "docs/guides/metadata-driven-development.md",
            "guides/metadata-driven-development.html",
        ),
        (
            "IDE and LSP Setup",
            "docs/guides/ide-setup.md",
            "guides/ide-setup.html",
        ),
        (
            "Deployment Targets",
            "docs/guides/deployment-targets.md",
            "guides/deployment-targets.html",
        ),
        (
            "Stdlib Guide",
            "docs/guides/stdlib-guide.md",
            "guides/stdlib-guide.html",
        ),
        (
            "Error Reference",
            "docs/guides/error-reference.md",
            "guides/error-reference.html",
        ),
    ] {
        assert!(
            pages
                .iter()
                .any(|page| page.title == title && page.source == source && page.output == output),
            "{title} は公開 guide として manifest から生成対象になるべき"
        );
    }
}

#[test]
fn test_doc_site_manifest_separates_user_guides_from_implementation_book() {
    let repo_root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..");
    let manifest = load_site_manifest(&repo_root).expect("docs/site.toml should load");

    let start_section = manifest
        .sections
        .iter()
        .find(|section| section.id == "start")
        .expect("start section が必要");
    assert!(
        start_section
            .pages
            .iter()
            .all(|page| page.audience.contains("L# を使う開発者")
                || page
                    .audience
                    .contains("L# でアプリやライブラリを書く開発者")
                || page.audience.contains("L# package を作る開発者")
                || page.audience.contains("既存サンプルから L# を学ぶ開発者")),
        "guides は利用者向け audience に揃えるべき"
    );

    let book_section = manifest
        .sections
        .iter()
        .find(|section| section.id == "book")
        .expect("book section が必要");
    assert!(
        book_section
            .pages
            .iter()
            .all(|page| page.audience.contains("コンパイラ実装を読む開発者")),
        "book はコンパイラ実装を読む開発者向け audience に揃えるべき"
    );
}

#[test]
fn test_cmd_doc_site_generates_manifest_pages_and_publish_assets() {
    let repo_root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..");
    let output = std::env::temp_dir().join("lsharp_doc_site_manifest");
    let _ = std::fs::remove_dir_all(&output);

    let result = cmd_doc_site_in(&repo_root, &output);
    assert!(result.is_ok(), "doc-site は成功するべき: {result:?}");

    assert!(output.join(".nojekyll").exists(), ".nojekyll が必要");
    assert!(output.join("sitemap.xml").exists(), "sitemap.xml が必要");
    assert!(
        output.join("docs-site-manifest.json").exists(),
        "docs-site-manifest.json が必要"
    );
    assert!(
        output.join("book/introduction.html").exists(),
        "book/introduction.html が必要"
    );
    assert!(
        output.join("operations/documentation-site.html").exists(),
        "documentation-site.html が必要"
    );
    assert!(
        output
            .join("operations/documentation-freshness.html")
            .exists(),
        "documentation-freshness.html が必要"
    );

    let index = std::fs::read_to_string(output.join("index.html")).unwrap();
    assert!(index.contains("data-source=\"docs/site.toml\""));
    assert!(index.contains("L# を使う"));
    assert!(index.contains("言語と実装を読む"));

    std::fs::remove_dir_all(&output).unwrap();
}
