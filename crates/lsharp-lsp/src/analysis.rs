use tower_lsp::lsp_types::{Hover, HoverContents, MarkedString, Position, Range};

pub fn hover(source: &str, position: Position) -> Option<Hover> {
    let offset = crate::util::position_to_offset(source, position)?;
    let (name, start, end) = symbol_range_near_offset(source, offset)?;
    let program = lsharp_syntax::parse(source).ok()?;
    let mut infer = lsharp_types::infer::Infer::new();
    let type_results = infer.infer_program(&program).ok()?;
    let signature = type_results
        .iter()
        .find(|(candidate, _)| candidate == &name)
        .map(|(_, scheme)| render_signature(&scheme.ty))
        .unwrap_or_else(|| "unknown".to_string());
    let doc = find_function_doc(&program, &name);
    let contents = if let Some(doc) = doc {
        format!("{name} : {signature}\n\n{doc}")
    } else {
        format!("{name} : {signature}")
    };

    Some(Hover {
        contents: HoverContents::Scalar(MarkedString::String(contents)),
        range: Some(Range::new(
            crate::util::offset_to_position(source, start),
            crate::util::offset_to_position(source, end),
        )),
    })
}

fn symbol_range_near_offset(source: &str, offset: usize) -> Option<(String, usize, usize)> {
    let offsets = [
        offset,
        offset.saturating_sub(1),
        offset.saturating_add(1).min(source.len().saturating_sub(1)),
    ];
    for candidate in offsets {
        if let Some(symbol) = crate::util::symbol_range_at_position(source, candidate) {
            return Some(symbol);
        }
    }
    None
}

fn find_function_doc(program: &lsharp_syntax::ast::Program, name: &str) -> Option<String> {
    use lsharp_syntax::ast::Decl;

    for decl in &program.decls {
        let decl = match decl {
            Decl::Private { inner, .. } => inner.as_ref(),
            other => other,
        };
        if let Decl::Defn { name: decl_name, metadata, .. } = decl
            && decl_name == name {
                return metadata.as_ref().and_then(|metadata| metadata.doc.clone());
            }
    }
    None
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_hover_returns_type_and_doc_for_toplevel_function() {
        let source = r#"
(defn add
  [x y]
  :doc "整数加算"
  (+ x y))

(defn main []
  (add 1 2))
"#;

        let hover = hover(source, Position::new(7, 3)).expect("hover が必要");
        let HoverContents::Scalar(MarkedString::String(text)) = hover.contents else {
            panic!("hover contents は string markdown を返すべき");
        };

        assert!(text.contains("add"));
        assert!(text.contains("Int -> Int -> Int"));
        assert!(text.contains("整数加算"));
    }
}
