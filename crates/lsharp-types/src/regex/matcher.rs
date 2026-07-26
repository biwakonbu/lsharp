use super::{RegexNode, dfa, node_matches_char, parse_regex};

/// 複雑なノード（Group/Alternation）の繰り返しマッチ
#[allow(clippy::too_many_arguments)]
pub(super) fn try_repeat_complex(
    inner: &RegexNode,
    rest_nodes: &[RegexNode],
    text: &[char],
    ni: usize,
    start: usize,
    require_end: bool,
    min_count: usize,
) -> bool {
    // inner を展開してマッチさせ、マッチした位置から再帰
    fn try_inner(
        inner: &RegexNode,
        rest_nodes: &[RegexNode],
        text: &[char],
        ni: usize,
        pos: usize,
        count: usize,
        min_count: usize,
        require_end: bool,
    ) -> bool {
        // 最小回数を満たしたら、残りのパターンとマッチ試行
        if count >= min_count && regex_match(rest_nodes, text, ni + 1, pos, require_end) {
            return true;
        }

        if pos >= text.len() {
            return false;
        }

        // inner をもう1回マッチさせる
        match inner {
            RegexNode::Group(group_nodes) | RegexNode::NonCapturingGroup(group_nodes) => {
                for end in (pos + 1)..=text.len() {
                    if regex_match(group_nodes, text, 0, pos, false)
                        && group_full_match(group_nodes, text, pos, end)
                        && try_inner(
                            inner,
                            rest_nodes,
                            text,
                            ni,
                            end,
                            count + 1,
                            min_count,
                            require_end,
                        )
                    {
                        return true;
                    }
                }
            }
            RegexNode::Alternation(alternatives) => {
                for alt in alternatives {
                    for end in (pos + 1)..=text.len() {
                        if group_full_match(alt, text, pos, end)
                            && try_inner(
                                inner,
                                rest_nodes,
                                text,
                                ni,
                                end,
                                count + 1,
                                min_count,
                                require_end,
                            )
                        {
                            return true;
                        }
                    }
                }
            }
            _ => {}
        }
        false
    }

    try_inner(
        inner,
        rest_nodes,
        text,
        ni,
        start,
        0,
        min_count,
        require_end,
    )
}

/// グループノードがテキストの [start, end) にフルマッチするか
pub(super) fn group_full_match(
    nodes: &[RegexNode],
    text: &[char],
    start: usize,
    end: usize,
) -> bool {
    let sub_text = &text[start..end];
    regex_match(nodes, sub_text, 0, 0, true)
}

pub(super) fn node_full_match_ends(node: &RegexNode, text: &[char], start: usize) -> Vec<usize> {
    let mut ends = Vec::new();
    for end in start..=text.len() {
        if regex_match(std::slice::from_ref(node), &text[start..end], 0, 0, true) {
            ends.push(end);
        }
    }
    ends
}

#[allow(clippy::too_many_arguments)]
fn try_repeat_bounded(
    inner: &RegexNode,
    nodes: &[RegexNode],
    text: &[char],
    ni: usize,
    start: usize,
    require_end: bool,
    min_count: usize,
    max_count: Option<usize>,
) -> bool {
    fn dfs(
        inner: &RegexNode,
        rest_nodes: &[RegexNode],
        text: &[char],
        pos: usize,
        count: usize,
        min_count: usize,
        max_limit: usize,
        require_end: bool,
    ) -> bool {
        if count >= min_count && regex_match(rest_nodes, text, 0, pos, require_end) {
            return true;
        }
        if count >= max_limit {
            return false;
        }

        for end in node_full_match_ends(inner, text, pos) {
            if end == pos {
                continue;
            }
            if dfs(
                inner,
                rest_nodes,
                text,
                end,
                count + 1,
                min_count,
                max_limit,
                require_end,
            ) {
                return true;
            }
        }
        false
    }

    if matches!(max_count, Some(max) if max < min_count) {
        return false;
    }
    let max_limit = max_count.unwrap_or_else(|| {
        text.len()
            .saturating_sub(start)
            .saturating_add(min_count)
            .saturating_add(1)
    });
    dfs(
        inner,
        &nodes[ni + 1..],
        text,
        start,
        0,
        min_count,
        max_limit,
        require_end,
    )
}

/// 正規表現マッチのステップ上限 (NC-11: 病的入力対策)
const REGEX_STEP_LIMIT: usize = 100_000;

thread_local! {
    /// 正規表現マッチのステップカウンター (NC-11)
    static REGEX_STEPS: std::cell::Cell<usize> = const { std::cell::Cell::new(0) };
}

/// NFA ベースの正規表現マッチ（バックトラッキング）
/// require_end = true の場合、テキスト末尾まで消費されることを要求
pub(super) fn regex_match(
    nodes: &[RegexNode],
    text: &[char],
    ni: usize,
    ti: usize,
    require_end: bool,
) -> bool {
    // ステップ制限チェック (NC-11)
    let exceeded = REGEX_STEPS.with(|steps| {
        let current = steps.get();
        if current >= REGEX_STEP_LIMIT {
            return true;
        }
        steps.set(current + 1);
        false
    });
    if exceeded {
        return false;
    }

    if ni >= nodes.len() {
        return if require_end { ti >= text.len() } else { true };
    }

    match &nodes[ni] {
        RegexNode::Group(group_nodes) => {
            let mut expanded = group_nodes.clone();
            expanded.extend_from_slice(&nodes[ni + 1..]);
            regex_match(&expanded, text, 0, ti, require_end)
        }
        RegexNode::NonCapturingGroup(group_nodes) => {
            let mut expanded = group_nodes.clone();
            expanded.extend_from_slice(&nodes[ni + 1..]);
            regex_match(&expanded, text, 0, ti, require_end)
        }
        RegexNode::Alternation(alternatives) => {
            for alt in alternatives {
                let mut expanded = alt.clone();
                expanded.extend_from_slice(&nodes[ni + 1..]);
                if regex_match(&expanded, text, 0, ti, require_end) {
                    return true;
                }
            }
            false
        }
        RegexNode::Star(inner) => {
            if regex_match(nodes, text, ni + 1, ti, require_end) {
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
                    if regex_match(nodes, text, ni + 1, pos, require_end) {
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
                    if regex_match(nodes, text, ni + 1, pos, require_end) {
                        return true;
                    }
                }
                false
            }
        }
        RegexNode::Optional(inner) => {
            if regex_match(nodes, text, ni + 1, ti, require_end) {
                return true;
            }
            match inner.as_ref() {
                RegexNode::Group(group_nodes) | RegexNode::NonCapturingGroup(group_nodes) => {
                    let mut expanded = group_nodes.clone();
                    expanded.extend_from_slice(&nodes[ni + 1..]);
                    regex_match(&expanded, text, 0, ti, require_end)
                }
                RegexNode::Alternation(alternatives) => {
                    for alt in alternatives {
                        let mut expanded = alt.clone();
                        expanded.extend_from_slice(&nodes[ni + 1..]);
                        if regex_match(&expanded, text, 0, ti, require_end) {
                            return true;
                        }
                    }
                    false
                }
                _ => {
                    if ti < text.len() && node_matches_char(inner, text[ti]) {
                        return regex_match(nodes, text, ni + 1, ti + 1, require_end);
                    }
                    false
                }
            }
        }
        RegexNode::BoundedRepeat { inner, min, max } => {
            try_repeat_bounded(inner, nodes, text, ni, ti, require_end, *min, *max)
        }
        RegexNode::Lookahead(lookahead_nodes) => {
            let remaining_text = &text[ti..];
            if regex_match(lookahead_nodes, remaining_text, 0, 0, false) {
                regex_match(nodes, text, ni + 1, ti, require_end)
            } else {
                false
            }
        }
        RegexNode::LookaheadNeg(lookahead_nodes) => {
            let remaining_text = &text[ti..];
            if !regex_match(lookahead_nodes, remaining_text, 0, 0, false) {
                regex_match(nodes, text, ni + 1, ti, require_end)
            } else {
                false
            }
        }
        RegexNode::Backreference(_) => false,
        node => {
            if ti < text.len() && node_matches_char(node, text[ti]) {
                regex_match(nodes, text, ni + 1, ti + 1, require_end)
            } else {
                false
            }
        }
    }
}

/// 後方参照・先読みを含むパターンかどうか判定
pub(crate) fn has_advanced_features(nodes: &[RegexNode]) -> bool {
    for node in nodes {
        match node {
            RegexNode::Backreference(_) | RegexNode::Lookahead(_) | RegexNode::LookaheadNeg(_) => {
                return true;
            }
            RegexNode::NonCapturingGroup(inner) => {
                if has_advanced_features(inner) {
                    return true;
                }
            }
            RegexNode::Group(inner) => {
                if has_advanced_features(inner) {
                    return true;
                }
            }
            RegexNode::Alternation(alts) => {
                for alt in alts {
                    if has_advanced_features(alt) {
                        return true;
                    }
                }
            }
            RegexNode::Star(inner) | RegexNode::Plus(inner) | RegexNode::Optional(inner) => {
                if has_advanced_features(&[inner.as_ref().clone()]) {
                    return true;
                }
            }
            RegexNode::BoundedRepeat { inner, .. } => {
                if has_advanced_features(&[inner.as_ref().clone()]) {
                    return true;
                }
            }
            _ => {}
        }
    }
    false
}

/// パターンマッチのエントリポイント
///
/// 後方参照・先読みがないパターンは DFA を使用し、
/// それ以外は NFA バックトラッキングにフォールバック。
pub(crate) fn simple_pattern_match(text: &str, pattern: &str) -> bool {
    // ステップカウンターをリセット (NC-11)
    REGEX_STEPS.with(|steps| steps.set(0));

    if pattern.is_empty() {
        return true;
    }

    let (anchored_start, pattern) = if let Some(stripped) = pattern.strip_prefix('^') {
        (true, stripped)
    } else {
        (false, pattern)
    };
    let (anchored_end, pattern) = if let Some(stripped) = pattern.strip_suffix('$') {
        (true, stripped)
    } else {
        (false, pattern)
    };

    let nodes = parse_regex(pattern);
    let text_chars: Vec<char> = text.chars().collect();
    let use_captures = has_advanced_features(&nodes);

    // DFA マッチを試行 (後方参照・先読みがないパターンのみ)
    if !use_captures
        && let Some(result) =
            dfa::try_dfa_match(pattern, &nodes, &text_chars, anchored_start, anchored_end)
    {
        return result;
    }

    // NFA バックトラッキングマッチ（フォールバック）
    let do_match = |start: usize, require_end: bool| -> bool {
        if use_captures {
            let mut captures = Vec::new();
            super::matcher_advanced::regex_match_with_captures(
                &nodes,
                &text_chars,
                0,
                start,
                require_end,
                &mut captures,
            )
        } else {
            regex_match(&nodes, &text_chars, 0, start, require_end)
        }
    };

    if anchored_start && anchored_end {
        do_match(0, true)
    } else if anchored_start {
        do_match(0, false)
    } else if anchored_end {
        for start in 0..=text_chars.len() {
            if do_match(start, true) {
                return true;
            }
        }
        false
    } else {
        for start in 0..=text_chars.len() {
            if do_match(start, false) {
                return true;
            }
        }
        false
    }
}
