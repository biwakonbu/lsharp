//! EC-M1-02 の migration baseline として Rust frontend の現行挙動を固定する。
//! この snapshot は v0.2 の最終仕様ではなく、surface 間の差を失わず移行するための inventory。

use lsharp_syntax::{
    ast::{Decl, Metadata, Program},
    metadata::MetadataFormKind,
    parse,
};
use serde_json::{Value, json};
use std::{
    fs,
    path::{Path, PathBuf},
    time::{SystemTime, UNIX_EPOCH},
};

const LEGACY_CONTRACTS: &str = include_str!("fixtures/metadata/frontend_legacy_contracts.ls");

struct TempSourceFile {
    path: PathBuf,
}

impl TempSourceFile {
    fn new(source: &str) -> Self {
        let nonce = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("system time は UNIX epoch 以降であるべき")
            .as_nanos();
        let path = std::env::temp_dir().join(format!(
            "lsharp_metadata_frontend_semantics_{}_{}.ls",
            std::process::id(),
            nonce
        ));
        fs::write(&path, source).expect("一時 L# source を書き込めるべき");
        Self { path }
    }

    fn path(&self) -> &Path {
        &self.path
    }
}

impl Drop for TempSourceFile {
    fn drop(&mut self) {
        let _ = fs::remove_file(&self.path);
    }
}

fn only_defn_metadata(program: &Program) -> &Metadata {
    let [
        Decl::Defn {
            metadata: Some(metadata),
            ..
        },
    ] = program.decls.as_slice()
    else {
        panic!("fixture は metadata 付き defn 一つだけであるべき");
    };
    metadata
}

fn ordered_forms(source: &str, metadata: &Metadata) -> Vec<Value> {
    metadata
        .forms
        .iter()
        .map(|form| {
            let span = form.span();
            let directive = source
                .get(span.start..span.end)
                .expect("metadata form span は source 内であるべき");
            match &form.kind {
                MetadataFormKind::LegacyExample { expressions } => json!({
                    "kind": "LegacyExample",
                    "expression_count": expressions.len(),
                    "directive": directive,
                }),
                MetadataFormKind::LegacyInvariant { .. } => json!({
                    "kind": "LegacyInvariant",
                    "directive": directive,
                }),
                // fixture は legacy example / invariant しか持たないので、この arm は
                // 実行時には通らない。MetadataFormKind が増えても inventory が
                // コンパイルエラーで落ちないようにするための受け皿である。
                other => json!({
                    "kind": "Unexpected",
                    "directive": directive,
                    "debug": format!("{other:?}"),
                }),
            }
        })
        .collect()
}

#[test]
fn rust_frontend_metadata_semantics_are_snapshotted() {
    let program = parse(LEGACY_CONTRACTS).expect("legacy contract fixture は parse できるべき");
    let metadata = only_defn_metadata(&program);
    let diagnostics = lsharp_types::metadata_check::check_metadata(&program);

    let formatted =
        lsharp_tooling::fmt::format_source(LEGACY_CONTRACTS).expect("fixture は format できるべき");
    let formatted_program = parse(&formatted).expect("format 後も parse できるべき");
    let formatted_metadata = only_defn_metadata(&formatted_program);

    let source_file = TempSourceFile::new(LEGACY_CONTRACTS);
    let html = lsharp_tooling::doc_html::render_doc_html(source_file.path())
        .expect("fixture から HTML docs を生成できるべき");
    let api_doc =
        lsharp_tooling::api_doc::build_api_doc_for_file("inventory", "0.1.0", source_file.path())
            .expect("fixture から API docs を生成できるべき");
    let api_function = api_doc
        .modules
        .first()
        .and_then(|module| module.functions.first())
        .expect("API docs は succ function を含むべき");
    let api_function_json =
        serde_json::to_value(api_function).expect("API function を JSON 化できるべき");

    let snapshot = json!({
        "inventory_status": "current_behavior_not_final_v0_2_contract",
        "parser_current_legacy_projection": {
            "aggregate_example_count": metadata.example.len(),
            "aggregate_has_invariant": metadata.invariant.is_some(),
            "ordered_forms": ordered_forms(LEGACY_CONTRACTS, metadata),
        },
        "checker_current_structural_semantics": {
            "diagnostic_count": diagnostics.len(),
            "non_bool_direct_example_accepted": diagnostics.is_empty(),
        },
        "formatter_current_roundtrip": {
            "reparse_succeeded": true,
            "aggregate_example_count": formatted_metadata.example.len(),
            "aggregate_has_invariant": formatted_metadata.invariant.is_some(),
            "ordered_forms": ordered_forms(&formatted, formatted_metadata),
        },
        "docs_current_rendering_boundary": {
            "html_renderer": {
                "doc_visible": html.contains("DOC_MARKER_contract_inventory"),
                "parameter_doc_visible": html.contains("PARAM_MARKER_input"),
                "return_doc_visible": html.contains("RETURN_MARKER_output"),
                "legacy_example_source_visible": html.contains("(succ 0)"),
                "legacy_invariant_source_visible": html.contains("(= result (+ x 1))"),
            },
            "api_doc_projection": {
                "doc": api_function.doc.as_deref(),
                "parameter_doc": api_function.params.first().and_then(|param| param.doc.as_deref()),
                "return_doc": api_function.returns.doc.as_deref(),
                "first_legacy_example": api_function.example.as_deref(),
                "has_invariant_field": api_function_json.get("invariant").is_some(),
            },
        },
    });

    insta::assert_snapshot!(
        "rust_frontend_metadata_semantics",
        serde_json::to_string_pretty(&snapshot).expect("snapshot JSON を serialize できるべき")
    );
}
