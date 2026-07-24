use tower_lsp::lsp_types::{CompletionItem, CompletionItemKind, Position};

pub fn complete(
    source: &str,
    position: Position,
    module_candidates: &[String],
) -> Vec<CompletionItem> {
    let Some(offset) = crate::util::position_to_offset(source, position) else {
        return Vec::new();
    };

    let prefix = prefix_at_offset(source, offset);
    if in_import_context(source, offset) {
        let mut items = Vec::new();
        for candidate in module_candidates {
            if candidate.starts_with(&prefix) {
                items.push(module_item(candidate));
            }
        }
        return items;
    }

    let mut items = Vec::new();
    if let Ok(program) = lsharp_syntax::parse(source) {
        for decl in &program.decls {
            if let lsharp_syntax::ast::Decl::Defn { name, .. } = decl
                && name.starts_with(&prefix)
            {
                items.push(function_item(name));
            }
        }
    }

    for keyword in ["defn", "let", "if", "match", "do", "fn", "module", "import"] {
        if keyword.starts_with(&prefix) {
            items.push(keyword_item(keyword));
        }
    }

    items
}

pub(crate) fn keyword_item(label: &str) -> CompletionItem {
    CompletionItem {
        label: label.to_string(),
        kind: Some(CompletionItemKind::KEYWORD),
        insert_text: Some(label.to_string()),
        ..Default::default()
    }
}

fn function_item(label: &str) -> CompletionItem {
    CompletionItem {
        label: label.to_string(),
        kind: Some(CompletionItemKind::FUNCTION),
        insert_text: Some(label.to_string()),
        ..Default::default()
    }
}

fn module_item(label: &str) -> CompletionItem {
    CompletionItem {
        label: label.to_string(),
        kind: Some(CompletionItemKind::MODULE),
        insert_text: Some(label.to_string()),
        ..Default::default()
    }
}

fn prefix_at_offset(source: &str, offset: usize) -> String {
    fn is_symbol_char(c: char) -> bool {
        c.is_alphanumeric() || c == '_' || c == '-' || c == '?' || c == '!'
    }

    let mut start = offset.min(source.len());
    while start > 0 {
        let prev = source[..start].chars().next_back().unwrap_or(' ');
        if !is_symbol_char(prev) {
            break;
        }
        start -= prev.len_utf8();
    }
    source[start..offset.min(source.len())].to_string()
}

fn in_import_context(source: &str, offset: usize) -> bool {
    let start = source[..offset.min(source.len())]
        .rfind('\n')
        .map(|idx| idx + 1)
        .unwrap_or(0);
    let line_prefix = source[start..offset.min(source.len())].trim_start();
    line_prefix.starts_with("(import ")
}

#[cfg(test)]
#[path = "completion_tests.rs"]
mod tests;
