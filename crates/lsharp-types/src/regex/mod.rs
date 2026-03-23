//! 正規表現エンジン
//!
//! NFA ベースのバックトラッキングマッチャと、
//! NFA→DFA 変換による高速マッチを提供する。
//! 後方参照・先読みは DFA で表現不可なため NFA にフォールバック。

pub(crate) mod dfa;

/// 正規表現パターン要素
///
/// サポートする構文:
/// - リテラル文字
/// - `^` 先頭アンカー
/// - `$` 末尾アンカー
/// - `.` 任意の1文字
/// - `*` 直前の0回以上
/// - `+` 直前の1回以上
/// - `[...]` 文字クラス（ネストなし、`^` 否定あり）
/// - `\p{L}` Unicode Letter, `\p{N}` Unicode Number
/// - `\P{L}` Unicode Letter 以外, `\P{N}` Unicode Number 以外
#[derive(Debug, Clone)]
pub(crate) enum RegexNode {
    /// リテラル文字
    Literal(char),
    /// 任意の1文字 (.)
    Dot,
    /// 文字クラス ([a-z], [abc])
    CharClass { chars: Vec<(char, char)>, negated: bool },
    /// 0回以上の繰り返し (*)
    Star(Box<RegexNode>),
    /// 1回以上の繰り返し (+)
    Plus(Box<RegexNode>),
    /// 0回または1回 (?)
    Optional(Box<RegexNode>),
    /// グループ ((...))
    Group(Vec<RegexNode>),
    /// 選択肢 (a|b)
    Alternation(Vec<Vec<RegexNode>>),
    /// 後方参照 (\1, \2, ...)
    Backreference(usize),
    /// 先読み ((?=...))
    Lookahead(Vec<RegexNode>),
    /// 否定先読み ((?!...))
    LookaheadNeg(Vec<RegexNode>),
    /// Unicode 文字クラス (\p{L}, \p{N}, \P{L}, \P{N})
    UnicodeClass { property: String, negated: bool },
}

/// 正規表現パターンをパース
pub(crate) fn parse_regex(pattern: &str) -> Vec<RegexNode> {
    let chars: Vec<char> = pattern.chars().collect();
    let (nodes, _) = parse_regex_inner(&chars, 0);
    // トップレベルの | を処理
    let has_alt = nodes.iter().any(|n| matches!(n, RegexNode::Literal('|')));
    if has_alt {
        let mut alternatives = vec![vec![]];
        for node in nodes {
            if matches!(node, RegexNode::Literal('|')) {
                alternatives.push(vec![]);
            } else {
                alternatives.last_mut().unwrap().push(node);
            }
        }
        if alternatives.len() > 1 {
            return vec![RegexNode::Alternation(alternatives)];
        }
        // 1つしかない場合はそのまま返す
        return alternatives.into_iter().flatten().collect();
    }
    nodes
}

fn parse_regex_inner(chars: &[char], start: usize) -> (Vec<RegexNode>, usize) {
    let mut nodes = Vec::new();
    let mut i = start;

    while i < chars.len() {
        if chars[i] == ')' {
            return (nodes, i + 1); // グループ終了
        }
        let node = match chars[i] {
            '|' => {
                i += 1;
                // 選択肢の区切りとしてリテラル '|' を一旦挿入
                nodes.push(RegexNode::Literal('|'));
                continue;
            }
            '(' => {
                i += 1;
                // 先読み構文のチェック: (?=...) または (?!...)
                if i < chars.len() && chars[i] == '?' {
                    if i + 1 < chars.len() && chars[i + 1] == '=' {
                        // 肯定先読み (?=...)
                        i += 2;
                        let (group_nodes, end) = parse_regex_inner(chars, i);
                        i = end;
                        RegexNode::Lookahead(group_nodes)
                    } else if i + 1 < chars.len() && chars[i + 1] == '!' {
                        // 否定先読み (?!...)
                        i += 2;
                        let (group_nodes, end) = parse_regex_inner(chars, i);
                        i = end;
                        RegexNode::LookaheadNeg(group_nodes)
                    } else {
                        // 通常のグループとして処理
                        let (group_nodes, end) = parse_regex_inner(chars, i);
                        i = end;
                        if group_nodes.iter().any(|n| matches!(n, RegexNode::Literal('|'))) {
                            let mut alternatives = vec![vec![]];
                            for node in group_nodes {
                                if matches!(node, RegexNode::Literal('|')) {
                                    alternatives.push(vec![]);
                                } else {
                                    alternatives.last_mut().unwrap().push(node);
                                }
                            }
                            RegexNode::Alternation(alternatives)
                        } else {
                            RegexNode::Group(group_nodes)
                        }
                    }
                } else {
                    let (group_nodes, end) = parse_regex_inner(chars, i);
                    i = end;
                    // グループ内の | を処理
                    if group_nodes.iter().any(|n| matches!(n, RegexNode::Literal('|'))) {
                        let mut alternatives = vec![vec![]];
                        for node in group_nodes {
                            if matches!(node, RegexNode::Literal('|')) {
                                alternatives.push(vec![]);
                            } else {
                                alternatives.last_mut().unwrap().push(node);
                            }
                        }
                        RegexNode::Alternation(alternatives)
                    } else {
                        RegexNode::Group(group_nodes)
                    }
                }
            }
            '.' => {
                i += 1;
                RegexNode::Dot
            }
            '[' => {
                i += 1;
                let negated = i < chars.len() && chars[i] == '^';
                if negated { i += 1; }
                let mut ranges = Vec::new();
                while i < chars.len() && chars[i] != ']' {
                    let start = chars[i];
                    if i + 2 < chars.len() && chars[i + 1] == '-' && chars[i + 2] != ']' {
                        let end = chars[i + 2];
                        ranges.push((start, end));
                        i += 3;
                    } else {
                        ranges.push((start, start));
                        i += 1;
                    }
                }
                if i < chars.len() { i += 1; } // ']'
                RegexNode::CharClass { chars: ranges, negated }
            }
            '\\' => {
                i += 1;
                if i < chars.len() {
                    let ch = chars[i];
                    i += 1;
                    match ch {
                        'd' => RegexNode::CharClass { chars: vec![('0', '9')], negated: false },
                        'w' => RegexNode::CharClass {
                            chars: vec![('a', 'z'), ('A', 'Z'), ('0', '9'), ('_', '_')],
                            negated: false,
                        },
                        's' => RegexNode::CharClass {
                            chars: vec![(' ', ' '), ('\t', '\t'), ('\n', '\n'), ('\r', '\r')],
                            negated: false,
                        },
                        // Unicode 文字クラス: \p{L}, \p{N}
                        'p' | 'P' => {
                            let negated = ch == 'P';
                            if i < chars.len() && chars[i] == '{' {
                                i += 1; // '{'
                                let prop_start = i;
                                while i < chars.len() && chars[i] != '}' {
                                    i += 1;
                                }
                                let property: String = chars[prop_start..i].iter().collect();
                                if i < chars.len() { i += 1; } // '}'
                                RegexNode::UnicodeClass { property, negated }
                            } else {
                                // \p / \P の後に { がない場合はリテラル
                                RegexNode::Literal(ch)
                            }
                        }
                        // 後方参照: \1, \2, ...
                        '1'..='9' => RegexNode::Backreference((ch as u8 - b'0') as usize),
                        _ => RegexNode::Literal(ch),
                    }
                } else {
                    RegexNode::Literal('\\')
                }
            }
            ch => {
                i += 1;
                RegexNode::Literal(ch)
            }
        };

        // 量指定子のチェック
        let node = if i < chars.len() {
            match chars[i] {
                '*' => { i += 1; RegexNode::Star(Box::new(node)) }
                '+' => { i += 1; RegexNode::Plus(Box::new(node)) }
                '?' => { i += 1; RegexNode::Optional(Box::new(node)) }
                _ => node,
            }
        } else {
            node
        };

        nodes.push(node);
    }

    (nodes, i)
}

/// 単一ノードが文字にマッチするか
pub(crate) fn node_matches_char(node: &RegexNode, ch: char) -> bool {
    match node {
        RegexNode::Literal(c) => *c == ch,
        RegexNode::Dot => true,
        RegexNode::CharClass { chars, negated } => {
            let in_class = chars.iter().any(|&(lo, hi)| ch >= lo && ch <= hi);
            if *negated { !in_class } else { in_class }
        }
        RegexNode::UnicodeClass { property, negated } => {
            let matches = match property.as_str() {
                "L" => ch.is_alphabetic(),
                "N" => ch.is_numeric(),
                _ => false,
            };
            if *negated { !matches } else { matches }
        }
        _ => false, // Star, Plus, Optional は直接比較しない
    }
}

/// 複雑なノード（Group/Alternation）の繰り返しマッチ
#[allow(clippy::too_many_arguments)]
fn try_repeat_complex(
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
        if count >= min_count
            && regex_match(rest_nodes, text, ni + 1, pos, require_end) {
                return true;
            }

        if pos >= text.len() {
            return false;
        }

        // inner をもう1回マッチさせる
        match inner {
            RegexNode::Group(group_nodes) => {
                for end in (pos + 1)..=text.len() {
                    if regex_match(group_nodes, text, 0, pos, false)
                        && group_full_match(group_nodes, text, pos, end)
                        && try_inner(inner, rest_nodes, text, ni, end, count + 1, min_count, require_end) {
                            return true;
                        }
                }
            }
            RegexNode::Alternation(alternatives) => {
                for alt in alternatives {
                    for end in (pos + 1)..=text.len() {
                        if group_full_match(alt, text, pos, end)
                            && try_inner(inner, rest_nodes, text, ni, end, count + 1, min_count, require_end) {
                                return true;
                            }
                    }
                }
            }
            _ => {}
        }
        false
    }

    try_inner(inner, rest_nodes, text, ni, start, 0, min_count, require_end)
}

/// グループノードがテキストの [start, end) にフルマッチするか
fn group_full_match(nodes: &[RegexNode], text: &[char], start: usize, end: usize) -> bool {
    let sub_text = &text[start..end];
    regex_match(nodes, sub_text, 0, 0, true)
}

/// 正規表現マッチのステップ上限 (NC-11: 病的入力対策)
const REGEX_STEP_LIMIT: usize = 100_000;

thread_local! {
    /// 正規表現マッチのステップカウンター (NC-11)
    static REGEX_STEPS: std::cell::Cell<usize> = const { std::cell::Cell::new(0) };
}

/// NFA ベースの正規表現マッチ（バックトラッキング）
/// require_end = true の場合、テキスト末尾まで消費されることを要求
fn regex_match(nodes: &[RegexNode], text: &[char], ni: usize, ti: usize, require_end: bool) -> bool {
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
            if matches!(inner.as_ref(), RegexNode::Group(_) | RegexNode::Alternation(_)) {
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
            if matches!(inner.as_ref(), RegexNode::Group(_) | RegexNode::Alternation(_)) {
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
                RegexNode::Group(group_nodes) => {
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
        RegexNode::Backreference(_) => {
            false
        }
        node => {
            if ti < text.len() && node_matches_char(node, text[ti]) {
                regex_match(nodes, text, ni + 1, ti + 1, require_end)
            } else {
                false
            }
        }
    }
}

/// キャプチャ付き正規表現マッチ（後方参照サポート）
fn regex_match_with_captures(
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
                    if regex_match_with_captures(nodes, text, ni + 1, end_pos, require_end, captures) {
                        return true;
                    }
                }
            }
            captures[group_idx] = None;
            false
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
                    regex_match_with_captures(nodes, text, ni + 1, ti + cap_len, require_end, captures)
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
            if matches!(inner.as_ref(), RegexNode::Group(_) | RegexNode::Alternation(_)) {
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
            if matches!(inner.as_ref(), RegexNode::Group(_) | RegexNode::Alternation(_)) {
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
                RegexNode::Group(group_nodes) => {
                    let mut expanded = group_nodes.clone();
                    expanded.extend_from_slice(&nodes[ni + 1..]);
                    regex_match_with_captures(&expanded, text, 0, ti, require_end, captures)
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
                _ => {
                    if ti < text.len() && node_matches_char(inner, text[ti]) {
                        return regex_match_with_captures(nodes, text, ni + 1, ti + 1, require_end, captures);
                    }
                    false
                }
            }
        }
        node => {
            if ti < text.len() && node_matches_char(node, text[ti]) {
                regex_match_with_captures(nodes, text, ni + 1, ti + 1, require_end, captures)
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
            RegexNode::Backreference(_)
            | RegexNode::Lookahead(_)
            | RegexNode::LookaheadNeg(_) => return true,
            RegexNode::Group(inner) => {
                if has_advanced_features(inner) { return true; }
            }
            RegexNode::Alternation(alts) => {
                for alt in alts {
                    if has_advanced_features(alt) { return true; }
                }
            }
            RegexNode::Star(inner) | RegexNode::Plus(inner) | RegexNode::Optional(inner) => {
                if has_advanced_features(&[inner.as_ref().clone()]) { return true; }
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
        && let Some(result) = dfa::try_dfa_match(
            pattern, &nodes, &text_chars,
            anchored_start, anchored_end,
        )
    {
        return result;
    }

    // NFA バックトラッキングマッチ（フォールバック）
    let do_match = |start: usize, require_end: bool| -> bool {
        if use_captures {
            let mut captures = Vec::new();
            regex_match_with_captures(&nodes, &text_chars, 0, start, require_end, &mut captures)
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

#[cfg(test)]
mod tests {
    use super::*;

    // === 既存の正規表現テスト (constraints.rs から移動) ===

    #[test]
    fn test_simple_pattern_match_literal() {
        assert!(simple_pattern_match("hello@world.com", "@"));
    }

    #[test]
    fn test_simple_pattern_match_anchored() {
        assert!(simple_pattern_match("hello", "^hello$"));
        assert!(!simple_pattern_match("hello world", "^hello$"));
    }

    #[test]
    fn test_regex_char_class() {
        assert!(simple_pattern_match("abc", "^[a-z]+$"));
        assert!(!simple_pattern_match("ABC", "^[a-z]+$"));
        assert!(simple_pattern_match("a1b", "[a-z]"));
        assert!(!simple_pattern_match("123", "^[a-z]+$"));
    }

    #[test]
    fn test_regex_star() {
        assert!(simple_pattern_match("", "^a*$"));
        assert!(simple_pattern_match("a", "^a*$"));
        assert!(simple_pattern_match("aaa", "^a*$"));
        assert!(simple_pattern_match("bbb", "^a*b+$"));
    }

    #[test]
    fn test_regex_plus() {
        assert!(!simple_pattern_match("", "^a+$"));
        assert!(simple_pattern_match("a", "^a+$"));
        assert!(simple_pattern_match("aaa", "^a+$"));
    }

    #[test]
    fn test_regex_question() {
        assert!(simple_pattern_match("", "^a?$"));
        assert!(simple_pattern_match("a", "^a?$"));
        assert!(!simple_pattern_match("aa", "^a?$"));
    }

    #[test]
    fn test_regex_dot() {
        assert!(simple_pattern_match("a", "^.$"));
        assert!(simple_pattern_match("z", "^.$"));
        assert!(!simple_pattern_match("", "^.$"));
        assert!(simple_pattern_match("abc", "^.+$"));
    }

    #[test]
    fn test_regex_combined() {
        assert!(simple_pattern_match("hello123", "^[a-z]+[0-9]+$"));
        assert!(!simple_pattern_match("123hello", "^[a-z]+[0-9]+$"));
        assert!(simple_pattern_match("test@example.com", "[a-z]+@[a-z]+"));
        assert!(simple_pattern_match("ab", "^a.?b$"));
    }

    #[test]
    fn test_regex_negated_class() {
        assert!(simple_pattern_match("123", "^[^a-z]+$"));
        assert!(!simple_pattern_match("abc", "^[^a-z]+$"));
    }

    #[test]
    fn test_regex_alternation() {
        assert!(simple_pattern_match("cat", "^(cat|dog)$"));
        assert!(simple_pattern_match("dog", "^(cat|dog)$"));
        assert!(!simple_pattern_match("bird", "^(cat|dog)$"));
    }

    #[test]
    fn test_regex_group() {
        assert!(simple_pattern_match("abab", "^(ab)+$"));
        assert!(!simple_pattern_match("abc", "^(ab)+$"));
    }

    #[test]
    fn test_regex_alternation_partial() {
        assert!(simple_pattern_match("hello world", "hello|goodbye"));
        assert!(simple_pattern_match("goodbye world", "hello|goodbye"));
        assert!(!simple_pattern_match("hi there", "^(hello|goodbye)$"));
    }

    #[test]
    fn test_regex_backreference() {
        assert!(simple_pattern_match("abcabc", "^(abc)\\1$"));
        assert!(!simple_pattern_match("abcdef", "^(abc)\\1$"));
        assert!(simple_pattern_match("aa", "^(.)\\1$"));
        assert!(!simple_pattern_match("ab", "^(.)\\1$"));
    }

    #[test]
    fn test_regex_lookahead() {
        assert!(simple_pattern_match("foobar", "^foo(?=bar)"));
        assert!(!simple_pattern_match("foobaz", "^foo(?=bar)"));
        assert!(simple_pattern_match("foobar", "^foo(?=bar)bar$"));
    }

    #[test]
    fn test_regex_lookahead_neg() {
        assert!(simple_pattern_match("foobaz", "^foo(?!bar)"));
        assert!(!simple_pattern_match("foobar", "^foo(?!bar)"));
        assert!(simple_pattern_match("foobaz", "^foo(?!bar)baz$"));
    }

    #[test]
    fn test_regex_pathological_input_terminates() {
        let pattern = "^(a+)+b$";
        let input = "aaaaaaaaaaaaaaaaaaaac";
        assert!(!simple_pattern_match(input, pattern));
    }

    #[test]
    fn test_regex_step_limit_prevents_hang() {
        let long_input: String = "a".repeat(100);
        let pattern = "^(a*)*b$";
        assert!(!simple_pattern_match(&long_input, pattern));
    }

    // === Unicode 文字クラステスト ===

    #[test]
    fn test_unicode_letter() {
        assert!(simple_pattern_match("hello", "^\\p{L}+$"));
        assert!(simple_pattern_match("\u{3053}\u{3093}\u{306B}\u{3061}\u{306F}", "^\\p{L}+$"));
        assert!(!simple_pattern_match("123", "^\\p{L}+$"));
    }

    #[test]
    fn test_unicode_number() {
        assert!(simple_pattern_match("123", "^\\p{N}+$"));
        assert!(!simple_pattern_match("abc", "^\\p{N}+$"));
    }

    #[test]
    fn test_unicode_letter_number_combined() {
        assert!(simple_pattern_match("\u{540D}\u{524D}42", "^\\p{L}+\\p{N}+$"));
        assert!(!simple_pattern_match("42\u{540D}\u{524D}", "^\\p{L}+\\p{N}+$"));
    }

    #[test]
    fn test_unicode_negated() {
        assert!(simple_pattern_match("123", "^\\P{L}+$"));
        assert!(!simple_pattern_match("abc", "^\\P{L}+$"));
        assert!(simple_pattern_match("42!", "^\\P{L}+$"));
    }
}
