use super::matcher::{group_full_match, node_full_match_ends, regex_match, try_repeat_complex};
use super::{RegexNode, node_matches_char};

#[allow(clippy::too_many_arguments)]
fn try_repeat_bounded_with_captures(
    inner: &RegexNode,
    nodes: &[RegexNode],
    text: &[char],
    ni: usize,
    start: usize,
    require_end: bool,
    min_count: usize,
    max_count: Option<usize>,
    captures: &mut Vec<Option<Vec<char>>>,
) -> bool {
    if matches!(max_count, Some(max) if max < min_count) {
        return false;
    }
    let max_limit = max_count.unwrap_or_else(|| {
        text.len()
            .saturating_sub(start)
            .saturating_add(min_count)
            .saturating_add(1)
    });

    fn dfs(
        inner: &RegexNode,
        rest_nodes: &[RegexNode],
        text: &[char],
        pos: usize,
        count: usize,
        min_count: usize,
        max_limit: usize,
        require_end: bool,
        captures: &mut Vec<Option<Vec<char>>>,
    ) -> bool {
        if count >= min_count {
            let mut rest_captures = captures.clone();
            if regex_match_with_captures(rest_nodes, text, 0, pos, require_end, &mut rest_captures)
            {
                *captures = rest_captures;
                return true;
            }
        }
        if count >= max_limit {
            return false;
        }

        for end in node_full_match_ends(inner, text, pos) {
            if end == pos {
                continue;
            }
            let mut next_captures = captures.clone();
            if dfs(
                inner,
                rest_nodes,
                text,
                end,
                count + 1,
                min_count,
                max_limit,
                require_end,
                &mut next_captures,
            ) {
                *captures = next_captures;
                return true;
            }
        }
        false
    }

    dfs(
        inner,
        &nodes[ni + 1..],
        text,
        start,
        0,
        min_count,
        max_limit,
        require_end,
        captures,
    )
}

/// キャプチャ付き正規表現マッチ（後方参照サポート）
pub(super) fn regex_match_with_captures(
    nodes: &[RegexNode],
    text: &[char],
    ni: usize,
    ti: usize,
    require_end: bool,
    captures: &mut Vec<Option<Vec<char>>>,
) -> bool {
    if ni >= nodes.len() {
        return if require_end { ti >= text.len() } else { true };
    }

    match &nodes[ni] {
        RegexNode::Group(group_nodes) => {
            let group_idx = captures.len();
            captures.push(None);
            for end_pos in ti..=text.len() {
                let sub = &text[ti..end_pos];
                if group_full_match(group_nodes, text, ti, end_pos) {
                    captures[group_idx] = Some(sub.to_vec());
                    if regex_match_with_captures(
                        nodes,
                        text,
                        ni + 1,
                        end_pos,
                        require_end,
                        captures,
                    ) {
                        return true;
                    }
                }
            }
            captures[group_idx] = None;
            false
        }
        RegexNode::NonCapturingGroup(group_nodes) => {
            let mut expanded = group_nodes.clone();
            expanded.extend_from_slice(&nodes[ni + 1..]);
            regex_match_with_captures(&expanded, text, 0, ti, require_end, captures)
        }
        RegexNode::Backreference(n) => {
            if *n == 0 || *n > captures.len() {
                return false;
            }
            if let Some(Some(captured)) = captures.get(*n - 1) {
                let captured = captured.clone();
                let cap_len = captured.len();
                if ti + cap_len > text.len() {
                    return false;
                }
                if text[ti..ti + cap_len] == captured[..] {
                    regex_match_with_captures(
                        nodes,
                        text,
                        ni + 1,
                        ti + cap_len,
                        require_end,
                        captures,
                    )
                } else {
                    false
                }
            } else {
                false
            }
        }
        RegexNode::Lookahead(lookahead_nodes) => {
            let remaining_text = &text[ti..];
            if regex_match(lookahead_nodes, remaining_text, 0, 0, false) {
                regex_match_with_captures(nodes, text, ni + 1, ti, require_end, captures)
            } else {
                false
            }
        }
        RegexNode::LookaheadNeg(lookahead_nodes) => {
            let remaining_text = &text[ti..];
            if !regex_match(lookahead_nodes, remaining_text, 0, 0, false) {
                regex_match_with_captures(nodes, text, ni + 1, ti, require_end, captures)
            } else {
                false
            }
        }
        RegexNode::Alternation(alternatives) => {
            for alt in alternatives {
                let mut expanded = alt.clone();
                expanded.extend_from_slice(&nodes[ni + 1..]);
                if regex_match_with_captures(&expanded, text, 0, ti, require_end, captures) {
                    return true;
                }
            }
            false
        }
        RegexNode::Star(inner) => {
            if regex_match_with_captures(nodes, text, ni + 1, ti, require_end, captures) {
                return true;
            }
            if matches!(
                inner.as_ref(),
                RegexNode::Group(_) | RegexNode::NonCapturingGroup(_) | RegexNode::Alternation(_)
            ) {
                try_repeat_complex(inner, nodes, text, ni, ti, require_end, 0)
            } else {
                let mut pos = ti;
                while pos < text.len() && node_matches_char(inner, text[pos]) {
                    pos += 1;
                    if regex_match_with_captures(nodes, text, ni + 1, pos, require_end, captures) {
                        return true;
                    }
                }
                false
            }
        }
        RegexNode::Plus(inner) => {
            if matches!(
                inner.as_ref(),
                RegexNode::Group(_) | RegexNode::NonCapturingGroup(_) | RegexNode::Alternation(_)
            ) {
                try_repeat_complex(inner, nodes, text, ni, ti, require_end, 1)
            } else {
                let mut pos = ti;
                while pos < text.len() && node_matches_char(inner, text[pos]) {
                    pos += 1;
                    if regex_match_with_captures(nodes, text, ni + 1, pos, require_end, captures) {
                        return true;
                    }
                }
                false
            }
        }
        RegexNode::Optional(inner) => {
            if regex_match_with_captures(nodes, text, ni + 1, ti, require_end, captures) {
                return true;
            }
            match inner.as_ref() {
                RegexNode::Group(group_nodes) | RegexNode::NonCapturingGroup(group_nodes) => {
                    let mut expanded = group_nodes.clone();
                    expanded.extend_from_slice(&nodes[ni + 1..]);
                    regex_match_with_captures(&expanded, text, 0, ti, require_end, captures)
                }
                RegexNode::Alternation(alternatives) => {
                    for alt in alternatives {
                        let mut expanded = alt.clone();
                        expanded.extend_from_slice(&nodes[ni + 1..]);
                        if regex_match_with_captures(&expanded, text, 0, ti, require_end, captures)
                        {
                            return true;
                        }
                    }
                    false
                }
                _ => {
                    if ti < text.len() && node_matches_char(inner, text[ti]) {
                        return regex_match_with_captures(
                            nodes,
                            text,
                            ni + 1,
                            ti + 1,
                            require_end,
                            captures,
                        );
                    }
                    false
                }
            }
        }
        RegexNode::BoundedRepeat { inner, min, max } => try_repeat_bounded_with_captures(
            inner,
            nodes,
            text,
            ni,
            ti,
            require_end,
            *min,
            *max,
            captures,
        ),
        node => {
            if ti < text.len() && node_matches_char(node, text[ti]) {
                regex_match_with_captures(nodes, text, ni + 1, ti + 1, require_end, captures)
            } else {
                false
            }
        }
    }
}
