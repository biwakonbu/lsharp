//! 制約評価エンジン
//!
//! 制約付き型 (type-constrained) の制約を評価する。
//! コンパイル時にリテラル値の制約チェックを行い、
//! ランタイム検証コードの生成を支援する。

use crate::types::ConstraintDef;

/// 制約評価エラー
#[derive(Debug, Clone, thiserror::Error)]
pub enum ConstraintError {
    #[error("制約違反: {constraint} (値: {value})")]
    Violation { constraint: String, value: String },

    #[error("型不一致: {expected} が期待されましたが {actual} が渡されました")]
    TypeMismatch { expected: String, actual: String },
}

/// 制約評価結果
#[derive(Debug, Clone, PartialEq)]
pub enum ConstraintResult {
    /// 制約を満たす
    Satisfied,
    /// 制約を満たさない
    Violated(String),
    /// コンパイル時に評価不能（ランタイムで検証が必要）
    Deferred,
}

/// 整数値に対して制約を評価
pub fn eval_int_constraint(value: i64, constraint: &ConstraintDef) -> ConstraintResult {
    match constraint {
        ConstraintDef::Gte(min) => {
            if value >= *min {
                ConstraintResult::Satisfied
            } else {
                ConstraintResult::Violated(format!(
                    "値 {value} は >= {min} を満たしません"
                ))
            }
        }
        ConstraintDef::Lte(max) => {
            if value <= *max {
                ConstraintResult::Satisfied
            } else {
                ConstraintResult::Violated(format!(
                    "値 {value} は <= {max} を満たしません"
                ))
            }
        }
        ConstraintDef::Range(lo, hi) => {
            if value >= *lo && value <= *hi {
                ConstraintResult::Satisfied
            } else {
                ConstraintResult::Violated(format!(
                    "値 {value} は範囲 [{lo}, {hi}] に含まれません"
                ))
            }
        }
        ConstraintDef::OneOf(values) => {
            if values.contains(&value) {
                ConstraintResult::Satisfied
            } else {
                ConstraintResult::Violated(format!(
                    "値 {value} は許可された値リストに含まれません"
                ))
            }
        }
        // 整数値に対して文字列制約は型不一致
        ConstraintDef::Matches(_) | ConstraintDef::MinLength(_) | ConstraintDef::MaxLength(_) => {
            ConstraintResult::Violated("整数値に文字列制約は適用できません".to_string())
        }
        // satisfies は常に実行時評価
        ConstraintDef::Satisfies(_) => ConstraintResult::Deferred,
    }
}

/// 文字列値に対して制約を評価
pub fn eval_string_constraint(value: &str, constraint: &ConstraintDef) -> ConstraintResult {
    match constraint {
        ConstraintDef::MinLength(min) => {
            if value.len() >= *min {
                ConstraintResult::Satisfied
            } else {
                ConstraintResult::Violated(format!(
                    "文字列長 {} は最小長 {min} を満たしません",
                    value.len()
                ))
            }
        }
        ConstraintDef::MaxLength(max) => {
            if value.len() <= *max {
                ConstraintResult::Satisfied
            } else {
                ConstraintResult::Violated(format!(
                    "文字列長 {} は最大長 {max} を超えています",
                    value.len()
                ))
            }
        }
        ConstraintDef::Matches(pattern) => {
            // MVP: 簡易パターンマッチ（正規表現エンジン完成前のフォールバック）
            // 完全な正規表現サポートは WG-2 で実装
            if simple_pattern_match(value, pattern) {
                ConstraintResult::Satisfied
            } else {
                ConstraintResult::Violated(format!(
                    "文字列 \"{value}\" はパターン \"{pattern}\" にマッチしません"
                ))
            }
        }
        // 文字列値に対して整数制約は型不一致
        ConstraintDef::Gte(_) | ConstraintDef::Lte(_) | ConstraintDef::Range(_, _) | ConstraintDef::OneOf(_) => {
            ConstraintResult::Violated("文字列値に整数制約は適用できません".to_string())
        }
        ConstraintDef::Satisfies(_) => ConstraintResult::Deferred,
    }
}

/// 複数の制約を全て評価
pub fn eval_int_constraints(value: i64, constraints: &[ConstraintDef]) -> Vec<ConstraintResult> {
    constraints.iter().map(|c| eval_int_constraint(value, c)).collect()
}

/// 複数の文字列制約を全て評価
pub fn eval_string_constraints(value: &str, constraints: &[ConstraintDef]) -> Vec<ConstraintResult> {
    constraints.iter().map(|c| eval_string_constraint(value, c)).collect()
}

/// 全制約が満たされているか
pub fn all_satisfied(results: &[ConstraintResult]) -> bool {
    results.iter().all(|r| matches!(r, ConstraintResult::Satisfied))
}

/// 違反した制約のメッセージを収集
pub fn collect_violations(results: &[ConstraintResult]) -> Vec<String> {
    results
        .iter()
        .filter_map(|r| {
            if let ConstraintResult::Violated(msg) = r {
                Some(msg.clone())
            } else {
                None
            }
        })
        .collect()
}

/// 境界値テストケースの自動生成（整数制約用）
pub fn generate_boundary_test_cases(constraints: &[ConstraintDef]) -> Vec<(i64, bool)> {
    let mut cases = Vec::new();

    for constraint in constraints {
        match constraint {
            ConstraintDef::Gte(min) => {
                cases.push((*min, true));          // 境界: ちょうど min
                cases.push((*min - 1, false));     // 境界外: min - 1
                cases.push((*min + 1, true));      // 境界内: min + 1
            }
            ConstraintDef::Lte(max) => {
                cases.push((*max, true));           // 境界: ちょうど max
                cases.push((*max + 1, false));      // 境界外: max + 1
                cases.push((*max - 1, true));       // 境界内: max - 1
            }
            ConstraintDef::Range(lo, hi) => {
                cases.push((*lo, true));            // 下限境界
                cases.push((*lo - 1, false));       // 下限外
                cases.push((*hi, true));            // 上限境界
                cases.push((*hi + 1, false));       // 上限外
                if lo < hi {
                    cases.push(((*lo + *hi) / 2, true)); // 中間値
                }
            }
            ConstraintDef::OneOf(values) => {
                for &v in values {
                    cases.push((v, true));
                }
                // リストにない値
                if let Some(&max) = values.iter().max() {
                    cases.push((max + 1000, false));
                }
            }
            _ => {} // 文字列制約・satisfies はスキップ
        }
    }

    cases
}

/// 簡易パターンマッチ（正規表現のサブセット）
///
/// サポートする構文:
/// - リテラル文字
/// - `^` 先頭アンカー
/// - `$` 末尾アンカー
/// - `.` 任意の1文字
/// - `*` 直前の0回以上
/// - `+` 直前の1回以上
/// - `[...]` 文字クラス（ネストなし、`^` 否定あり）
/// 正規表現パターン要素
#[derive(Debug, Clone)]
enum RegexNode {
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
}

/// 正規表現パターンをパース
fn parse_regex(pattern: &str) -> Vec<RegexNode> {
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
fn node_matches_char(node: &RegexNode, ch: char) -> bool {
    match node {
        RegexNode::Literal(c) => *c == ch,
        RegexNode::Dot => true,
        RegexNode::CharClass { chars, negated } => {
            let in_class = chars.iter().any(|&(lo, hi)| ch >= lo && ch <= hi);
            if *negated { !in_class } else { in_class }
        }
        _ => false, // Star, Plus, Optional は直接比較しない
    }
}

/// 複雑なノード（Group/Alternation）の繰り返しマッチ
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
        if count >= min_count {
            if regex_match(rest_nodes, text, ni + 1, pos, require_end) {
                return true;
            }
        }

        if pos >= text.len() {
            return false;
        }

        // inner をもう1回マッチさせる
        match inner {
            RegexNode::Group(group_nodes) => {
                // グループをマッチさせて、消費した文字数を取得
                for end in (pos + 1)..=text.len() {
                    if regex_match(group_nodes, text, 0, pos, false)
                        && group_full_match(group_nodes, text, pos, end)
                    {
                        if try_inner(inner, rest_nodes, text, ni, end, count + 1, min_count, require_end) {
                            return true;
                        }
                    }
                }
            }
            RegexNode::Alternation(alternatives) => {
                for alt in alternatives {
                    for end in (pos + 1)..=text.len() {
                        if group_full_match(alt, text, pos, end) {
                            if try_inner(inner, rest_nodes, text, ni, end, count + 1, min_count, require_end) {
                                return true;
                            }
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

/// NFA ベースの正規表現マッチ（バックトラッキング）
/// require_end = true の場合、テキスト末尾まで消費されることを要求
fn regex_match(nodes: &[RegexNode], text: &[char], ni: usize, ti: usize, require_end: bool) -> bool {
    if ni >= nodes.len() {
        return if require_end { ti >= text.len() } else { true };
    }

    match &nodes[ni] {
        RegexNode::Group(group_nodes) => {
            // グループ内のノードをフラットに展開してマッチ
            let mut expanded = group_nodes.clone();
            expanded.extend_from_slice(&nodes[ni + 1..]);
            regex_match(&expanded, text, 0, ti, require_end)
        }
        RegexNode::Alternation(alternatives) => {
            // いずれかの選択肢がマッチすればOK
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
            // 0回マッチ
            if regex_match(nodes, text, ni + 1, ti, require_end) {
                return true;
            }
            // Group/Alternation の繰り返し
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
            // 肯定先読み: 位置を消費せずにマッチを確認
            let remaining_text = &text[ti..];
            if regex_match(lookahead_nodes, remaining_text, 0, 0, false) {
                regex_match(nodes, text, ni + 1, ti, require_end)
            } else {
                false
            }
        }
        RegexNode::LookaheadNeg(lookahead_nodes) => {
            // 否定先読み: マッチしないことを確認
            let remaining_text = &text[ti..];
            if !regex_match(lookahead_nodes, remaining_text, 0, 0, false) {
                regex_match(nodes, text, ni + 1, ti, require_end)
            } else {
                false
            }
        }
        RegexNode::Backreference(_) => {
            // 後方参照はキャプチャ付きマッチでのみサポート
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
/// captures: グループ番号 -> キャプチャされたテキスト
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
            // キャプチャグループ: マッチ結果を記録
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
            // 後方参照: \n で n 番目のキャプチャグループにマッチ
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
            // 肯定先読み: 位置を消費せずにマッチを確認
            let remaining_text = &text[ti..];
            if regex_match(lookahead_nodes, remaining_text, 0, 0, false) {
                regex_match_with_captures(nodes, text, ni + 1, ti, require_end, captures)
            } else {
                false
            }
        }
        RegexNode::LookaheadNeg(lookahead_nodes) => {
            // 否定先読み: マッチしないことを確認
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
fn has_advanced_features(nodes: &[RegexNode]) -> bool {
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

fn simple_pattern_match(text: &str, pattern: &str) -> bool {
    if pattern.is_empty() {
        return true;
    }

    let (anchored_start, pattern) = if pattern.starts_with('^') {
        (true, &pattern[1..])
    } else {
        (false, pattern)
    };
    let (anchored_end, pattern) = if pattern.ends_with('$') {
        (true, &pattern[..pattern.len() - 1])
    } else {
        (false, pattern)
    };

    let nodes = parse_regex(pattern);
    let text_chars: Vec<char> = text.chars().collect();
    let use_captures = has_advanced_features(&nodes);

    // 後方参照・先読みがある場合はキャプチャ付きマッチを使用
    let do_match = |start: usize, require_end: bool| -> bool {
        if use_captures {
            let mut captures = Vec::new();
            regex_match_with_captures(&nodes, &text_chars, 0, start, require_end, &mut captures)
        } else {
            regex_match(&nodes, &text_chars, 0, start, require_end)
        }
    };

    if anchored_start && anchored_end {
        // 完全マッチ: 先頭から開始、末尾まで消費
        do_match(0, true)
    } else if anchored_start {
        // 先頭マッチ
        do_match(0, false)
    } else if anchored_end {
        // 末尾マッチ: 各位置から試行、末尾まで消費
        for start in 0..=text_chars.len() {
            if do_match(start, true) {
                return true;
            }
        }
        false
    } else {
        // 部分マッチ
        for start in 0..=text_chars.len() {
            if do_match(start, false) {
                return true;
            }
        }
        false
    }
}

use std::collections::HashMap;
use crate::types::ConstrainedTypeInfo;

/// 制約階層の解決
///
/// 制約付き型の基底型が別の制約付き型の場合、
/// 基底型の制約も自動的に継承する。
///
/// 例: Natural (>= 0), Percentage base=Natural (:constraints [(<= 100)])
/// -> Percentage は (>= 0) と (<= 100) の両方の制約を持つ
pub fn resolve_constraint_hierarchy(
    type_name: &str,
    constrained_types: &HashMap<String, ConstrainedTypeInfo>,
) -> Vec<ConstraintDef> {
    let mut all_constraints = Vec::new();
    let mut visited = Vec::new();
    collect_constraints_recursive(type_name, constrained_types, &mut all_constraints, &mut visited);
    all_constraints
}

/// 再帰的に基底型の制約を収集
fn collect_constraints_recursive(
    type_name: &str,
    constrained_types: &HashMap<String, ConstrainedTypeInfo>,
    constraints: &mut Vec<ConstraintDef>,
    visited: &mut Vec<String>,
) {
    // 循環参照検出
    if visited.contains(&type_name.to_string()) {
        return;
    }
    visited.push(type_name.to_string());

    if let Some(info) = constrained_types.get(type_name) {
        // 基底型が別の制約付き型か確認
        let base_name = match &info.base_type {
            crate::types::Type::Con(name) => Some(name.clone()),
            _ => None,
        };

        // 基底型の制約を先に収集（階層の上位が先）
        if let Some(ref base) = base_name {
            collect_constraints_recursive(base, constrained_types, constraints, visited);
        }

        // 自身の制約を追加
        constraints.extend(info.constraints.clone());
    }
}

/// 制約の互換性チェック
///
/// 子の制約が親の制約を満たすか検証する。
/// 例: 親 (>= 0, <= 200), 子 (>= 0, <= 100) -> OK (子は親の範囲内)
/// 例: 親 (>= 0, <= 100), 子 (>= -10, <= 100) -> NG (子が親の下限を超える)
pub fn check_constraint_compatibility(
    parent_constraints: &[ConstraintDef],
    child_constraints: &[ConstraintDef],
) -> Vec<String> {
    let mut errors = Vec::new();

    // 親の各制約に対して、子の制約が親を満たすか検証
    for parent in parent_constraints {
        match parent {
            ConstraintDef::Gte(parent_min) => {
                // 子の Gte が親の Gte 以上であるか
                let child_min = child_constraints.iter().find_map(|c| match c {
                    ConstraintDef::Gte(v) => Some(*v),
                    ConstraintDef::Range(lo, _) => Some(*lo),
                    _ => None,
                });
                if let Some(cm) = child_min {
                    if cm < *parent_min {
                        errors.push(format!(
                            "子の下限 ({cm}) が親の下限 ({parent_min}) より小さい"
                        ));
                    }
                }
            }
            ConstraintDef::Lte(parent_max) => {
                // 子の Lte が親の Lte 以下であるか
                let child_max = child_constraints.iter().find_map(|c| match c {
                    ConstraintDef::Lte(v) => Some(*v),
                    ConstraintDef::Range(_, hi) => Some(*hi),
                    _ => None,
                });
                if let Some(cm) = child_max {
                    if cm > *parent_max {
                        errors.push(format!(
                            "子の上限 ({cm}) が親の上限 ({parent_max}) より大きい"
                        ));
                    }
                }
            }
            ConstraintDef::Range(parent_lo, parent_hi) => {
                // 子の範囲が親の範囲内か（Range、Gte、Lte のいずれかから推定）
                let child_range = child_constraints.iter().find_map(|c| match c {
                    ConstraintDef::Range(lo, hi) => Some((*lo, *hi)),
                    _ => None,
                });
                if let Some((clo, chi)) = child_range {
                    if clo < *parent_lo {
                        errors.push(format!(
                            "子の下限 ({clo}) が親の範囲下限 ({parent_lo}) より小さい"
                        ));
                    }
                    if chi > *parent_hi {
                        errors.push(format!(
                            "子の上限 ({chi}) が親の範囲上限 ({parent_hi}) より大きい"
                        ));
                    }
                } else {
                    // Range がない場合、Gte/Lte から推定して範囲チェック
                    let child_lo = child_constraints.iter().find_map(|c| match c {
                        ConstraintDef::Gte(v) => Some(*v),
                        _ => None,
                    });
                    let child_hi = child_constraints.iter().find_map(|c| match c {
                        ConstraintDef::Lte(v) => Some(*v),
                        _ => None,
                    });
                    if let Some(clo) = child_lo {
                        if clo < *parent_lo {
                            errors.push(format!(
                                "子の下限 ({clo}) が親の範囲下限 ({parent_lo}) より小さい"
                            ));
                        }
                    }
                    if let Some(chi) = child_hi {
                        if chi > *parent_hi {
                            errors.push(format!(
                                "子の上限 ({chi}) が親の範囲上限 ({parent_hi}) より大きい"
                            ));
                        }
                    }
                }
            }
            _ => {}
        }
    }

    errors
}

/// 制約階層間の型変換情報
///
/// 2つの制約付き型間で安全な型変換が可能かを判定し、
/// 変換関数の情報を返す。
#[derive(Debug, Clone)]
pub struct ConversionInfo {
    /// 変換元の型名
    pub from_type: String,
    /// 変換先の型名
    pub to_type: String,
    /// 変換方向
    pub kind: ConversionKind,
    /// 追加で必要な制約チェック（アップキャスト時は不要、ダウンキャスト時は必要）
    pub extra_checks: Vec<ConstraintDef>,
}

/// 型変換の種別
#[derive(Debug, Clone, PartialEq)]
pub enum ConversionKind {
    /// 安全な拡大変換（子 -> 親: 追加チェック不要）
    Upcast,
    /// 制約付き縮小変換（親 -> 子: ランタイムチェック必要）
    Downcast,
    /// 変換不可
    Incompatible,
}

/// 2つの制約付き型間の変換情報を生成
pub fn analyze_conversion(
    from_type: &str,
    to_type: &str,
    constrained_types: &HashMap<String, ConstrainedTypeInfo>,
) -> ConversionInfo {
    let from_constraints = resolve_constraint_hierarchy(from_type, constrained_types);
    let to_constraints = resolve_constraint_hierarchy(to_type, constrained_types);

    // from が to の部分型か判定: from の制約が to の制約を全て包含（同等以上に厳しい）
    let from_subsumes_to = is_subtype_constraints(&from_constraints, &to_constraints);
    // to が from の部分型か判定
    let to_subsumes_from = is_subtype_constraints(&to_constraints, &from_constraints);

    if from_subsumes_to {
        // from は to の子型（より厳しい制約）→ to への変換はアップキャスト（安全）
        return ConversionInfo {
            from_type: from_type.to_string(),
            to_type: to_type.to_string(),
            kind: ConversionKind::Upcast,
            extra_checks: vec![],
        };
    }

    if to_subsumes_from {
        // to は from の子型（より厳しい制約）→ to への変換はダウンキャスト（チェック必要）
        let extra_checks: Vec<ConstraintDef> = to_constraints
            .iter()
            .filter(|c| !from_constraints.contains(c))
            .cloned()
            .collect();

        return ConversionInfo {
            from_type: from_type.to_string(),
            to_type: to_type.to_string(),
            kind: ConversionKind::Downcast,
            extra_checks,
        };
    }

    // どちらの方向も互換性なし
    ConversionInfo {
        from_type: from_type.to_string(),
        to_type: to_type.to_string(),
        kind: ConversionKind::Incompatible,
        extra_checks: vec![],
    }
}

/// child_constraints が parent_constraints の全制約を少なくとも同等に満たすか判定
///
/// 子型の条件: 親の各制約について、子が同等以上に厳しい制約を持つこと。
/// 例: 親が Gte(0) なら子は Gte(0) 以上を持つ必要がある。
fn is_subtype_constraints(child: &[ConstraintDef], parent: &[ConstraintDef]) -> bool {
    // 親の制約が空なら、どの子も部分型
    if parent.is_empty() {
        return true;
    }
    // 子の制約が空で親の制約があるなら、子は部分型ではない
    if child.is_empty() && !parent.is_empty() {
        return false;
    }

    // 互換性チェック（子が親の制約を違反しないか）
    let compat_errors = check_constraint_compatibility(parent, child);
    if !compat_errors.is_empty() {
        return false;
    }

    // 追加条件: 親の各制約の「種類」について、子が対応する制約を持つか確認
    for p in parent {
        let has_corresponding = child.iter().any(|c| constraints_same_dimension(p, c));
        if !has_corresponding {
            return false;
        }
    }

    true
}

/// 2つの制約が同じ次元（同じ種類のチェック）に属するか判定
fn constraints_same_dimension(a: &ConstraintDef, b: &ConstraintDef) -> bool {
    use ConstraintDef::*;
    matches!(
        (a, b),
        (Gte(_), Gte(_))
            | (Gte(_), Range(_, _))
            | (Lte(_), Lte(_))
            | (Lte(_), Range(_, _))
            | (Range(_, _), Range(_, _))
            | (Range(_, _), Gte(_))
            | (Range(_, _), Lte(_))
            | (OneOf(_), OneOf(_))
            | (MinLength(_), MinLength(_))
            | (MaxLength(_), MaxLength(_))
            | (Matches(_), Matches(_))
            | (Satisfies(_), Satisfies(_))
    )
}

/// ランタイム検証用の制約チェック式を IR 風の擬似コードとして生成
///
/// 制約付き型のスマートコンストラクタ（Name.new）で使用される
/// ランタイム検証ロジックの記述。
#[derive(Debug, Clone)]
pub struct RuntimeCheck {
    /// チェック対象の型名
    pub type_name: String,
    /// チェック条件のリスト（全て AND で結合）
    pub conditions: Vec<RuntimeCondition>,
}

/// ランタイム検証条件
#[derive(Debug, Clone)]
pub enum RuntimeCondition {
    /// value >= threshold
    IntGte(i64),
    /// value <= threshold
    IntLte(i64),
    /// lo <= value <= hi
    IntRange(i64, i64),
    /// value が指定値のいずれかと一致
    IntOneOf(Vec<i64>),
    /// 文字列長が最小長以上
    StrMinLength(usize),
    /// 文字列長が最大長以下
    StrMaxLength(usize),
    /// パターンマッチ
    StrMatches(String),
}

/// 制約付き型からランタイムチェック情報を生成
pub fn generate_runtime_checks(
    type_name: &str,
    constrained_types: &HashMap<String, ConstrainedTypeInfo>,
) -> RuntimeCheck {
    let constraints = resolve_constraint_hierarchy(type_name, constrained_types);
    let conditions: Vec<RuntimeCondition> = constraints
        .iter()
        .filter_map(|c| constraint_to_runtime_condition(c))
        .collect();

    RuntimeCheck {
        type_name: type_name.to_string(),
        conditions,
    }
}

/// ConstraintDef を RuntimeCondition に変換
fn constraint_to_runtime_condition(constraint: &ConstraintDef) -> Option<RuntimeCondition> {
    match constraint {
        ConstraintDef::Gte(v) => Some(RuntimeCondition::IntGte(*v)),
        ConstraintDef::Lte(v) => Some(RuntimeCondition::IntLte(*v)),
        ConstraintDef::Range(lo, hi) => Some(RuntimeCondition::IntRange(*lo, *hi)),
        ConstraintDef::OneOf(values) => Some(RuntimeCondition::IntOneOf(values.clone())),
        ConstraintDef::MinLength(v) => Some(RuntimeCondition::StrMinLength(*v)),
        ConstraintDef::MaxLength(v) => Some(RuntimeCondition::StrMaxLength(*v)),
        ConstraintDef::Matches(p) => Some(RuntimeCondition::StrMatches(p.clone())),
        ConstraintDef::Satisfies(_) => None, // satisfies は静的に変換不能
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_gte_satisfied() {
        assert_eq!(
            eval_int_constraint(5, &ConstraintDef::Gte(0)),
            ConstraintResult::Satisfied
        );
    }

    #[test]
    fn test_gte_boundary() {
        assert_eq!(
            eval_int_constraint(0, &ConstraintDef::Gte(0)),
            ConstraintResult::Satisfied
        );
    }

    #[test]
    fn test_gte_violated() {
        assert!(matches!(
            eval_int_constraint(-1, &ConstraintDef::Gte(0)),
            ConstraintResult::Violated(_)
        ));
    }

    #[test]
    fn test_lte_satisfied() {
        assert_eq!(
            eval_int_constraint(50, &ConstraintDef::Lte(100)),
            ConstraintResult::Satisfied
        );
    }

    #[test]
    fn test_lte_violated() {
        assert!(matches!(
            eval_int_constraint(101, &ConstraintDef::Lte(100)),
            ConstraintResult::Violated(_)
        ));
    }

    #[test]
    fn test_range_satisfied() {
        assert_eq!(
            eval_int_constraint(50, &ConstraintDef::Range(0, 100)),
            ConstraintResult::Satisfied
        );
    }

    #[test]
    fn test_range_violated() {
        assert!(matches!(
            eval_int_constraint(-1, &ConstraintDef::Range(0, 100)),
            ConstraintResult::Violated(_)
        ));
    }

    #[test]
    fn test_one_of_satisfied() {
        assert_eq!(
            eval_int_constraint(2, &ConstraintDef::OneOf(vec![1, 2, 3])),
            ConstraintResult::Satisfied
        );
    }

    #[test]
    fn test_one_of_violated() {
        assert!(matches!(
            eval_int_constraint(4, &ConstraintDef::OneOf(vec![1, 2, 3])),
            ConstraintResult::Violated(_)
        ));
    }

    #[test]
    fn test_min_length_satisfied() {
        assert_eq!(
            eval_string_constraint("hello", &ConstraintDef::MinLength(3)),
            ConstraintResult::Satisfied
        );
    }

    #[test]
    fn test_min_length_violated() {
        assert!(matches!(
            eval_string_constraint("hi", &ConstraintDef::MinLength(3)),
            ConstraintResult::Violated(_)
        ));
    }

    #[test]
    fn test_max_length_satisfied() {
        assert_eq!(
            eval_string_constraint("hi", &ConstraintDef::MaxLength(10)),
            ConstraintResult::Satisfied
        );
    }

    #[test]
    fn test_max_length_violated() {
        assert!(matches!(
            eval_string_constraint("hello world", &ConstraintDef::MaxLength(5)),
            ConstraintResult::Violated(_)
        ));
    }

    #[test]
    fn test_satisfies_deferred() {
        assert_eq!(
            eval_int_constraint(42, &ConstraintDef::Satisfies("is-even".to_string())),
            ConstraintResult::Deferred
        );
    }

    #[test]
    fn test_multiple_constraints() {
        let constraints = vec![
            ConstraintDef::Gte(0),
            ConstraintDef::Lte(100),
        ];
        let results = eval_int_constraints(50, &constraints);
        assert!(all_satisfied(&results));
    }

    #[test]
    fn test_multiple_constraints_violated() {
        let constraints = vec![
            ConstraintDef::Gte(0),
            ConstraintDef::Lte(100),
        ];
        let results = eval_int_constraints(101, &constraints);
        assert!(!all_satisfied(&results));
        let violations = collect_violations(&results);
        assert_eq!(violations.len(), 1);
    }

    #[test]
    fn test_boundary_test_generation() {
        let constraints = vec![ConstraintDef::Range(0, 100)];
        let cases = generate_boundary_test_cases(&constraints);
        // 5 ケース: lo, lo-1, hi, hi+1, mid
        assert_eq!(cases.len(), 5);

        // 境界値が含まれているか確認
        assert!(cases.iter().any(|(v, ok)| *v == 0 && *ok));
        assert!(cases.iter().any(|(v, ok)| *v == -1 && !*ok));
        assert!(cases.iter().any(|(v, ok)| *v == 100 && *ok));
        assert!(cases.iter().any(|(v, ok)| *v == 101 && !*ok));
    }

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
        // [a-z] 文字クラス
        assert!(simple_pattern_match("abc", "^[a-z]+$"));
        assert!(!simple_pattern_match("ABC", "^[a-z]+$"));
        assert!(simple_pattern_match("a1b", "[a-z]"));
        assert!(!simple_pattern_match("123", "^[a-z]+$"));
    }

    #[test]
    fn test_regex_star() {
        // * (0回以上)
        assert!(simple_pattern_match("", "^a*$"));
        assert!(simple_pattern_match("a", "^a*$"));
        assert!(simple_pattern_match("aaa", "^a*$"));
        assert!(simple_pattern_match("bbb", "^a*b+$"));
    }

    #[test]
    fn test_regex_plus() {
        // + (1回以上)
        assert!(!simple_pattern_match("", "^a+$"));
        assert!(simple_pattern_match("a", "^a+$"));
        assert!(simple_pattern_match("aaa", "^a+$"));
    }

    #[test]
    fn test_regex_question() {
        // ? (0回または1回)
        assert!(simple_pattern_match("", "^a?$"));
        assert!(simple_pattern_match("a", "^a?$"));
        assert!(!simple_pattern_match("aa", "^a?$"));
    }

    #[test]
    fn test_regex_dot() {
        // . (任意の1文字)
        assert!(simple_pattern_match("a", "^.$"));
        assert!(simple_pattern_match("z", "^.$"));
        assert!(!simple_pattern_match("", "^.$"));
        assert!(simple_pattern_match("abc", "^.+$"));
    }

    #[test]
    fn test_regex_combined() {
        // 複合パターン
        assert!(simple_pattern_match("hello123", "^[a-z]+[0-9]+$"));
        assert!(!simple_pattern_match("123hello", "^[a-z]+[0-9]+$"));
        assert!(simple_pattern_match("test@example.com", "[a-z]+@[a-z]+"));
        assert!(simple_pattern_match("ab", "^a.?b$"));
    }

    #[test]
    fn test_regex_negated_class() {
        // [^...] 否定文字クラス
        assert!(simple_pattern_match("123", "^[^a-z]+$"));
        assert!(!simple_pattern_match("abc", "^[^a-z]+$"));
    }

    #[test]
    fn test_regex_alternation() {
        // | (選択)
        assert!(simple_pattern_match("cat", "^(cat|dog)$"));
        assert!(simple_pattern_match("dog", "^(cat|dog)$"));
        assert!(!simple_pattern_match("bird", "^(cat|dog)$"));
    }

    #[test]
    fn test_regex_group() {
        // () グループ
        assert!(simple_pattern_match("abab", "^(ab)+$"));
        assert!(!simple_pattern_match("abc", "^(ab)+$"));
    }

    #[test]
    fn test_regex_alternation_partial() {
        // | 部分マッチ
        assert!(simple_pattern_match("hello world", "hello|goodbye"));
        assert!(simple_pattern_match("goodbye world", "hello|goodbye"));
        assert!(!simple_pattern_match("hi there", "^(hello|goodbye)$"));
    }

    #[test]
    fn test_type_mismatch_int_string_constraint() {
        assert!(matches!(
            eval_int_constraint(42, &ConstraintDef::MinLength(3)),
            ConstraintResult::Violated(_)
        ));
    }

    #[test]
    fn test_type_mismatch_string_int_constraint() {
        assert!(matches!(
            eval_string_constraint("hello", &ConstraintDef::Gte(0)),
            ConstraintResult::Violated(_)
        ));
    }
}

    #[test]
    fn test_regex_backreference() {
        // (abc) の後に \1 で同じテキストをマッチ
        assert!(simple_pattern_match("abcabc", "^(abc)\\1$"));
        assert!(!simple_pattern_match("abcdef", "^(abc)\\1$"));
        // 単語の繰り返し
        assert!(simple_pattern_match("aa", "^(.)\\1$"));
        assert!(!simple_pattern_match("ab", "^(.)\\1$"));
    }

    #[test]
    fn test_regex_lookahead() {
        // 肯定先読み: foo の後に bar が続く位置をマッチ
        assert!(simple_pattern_match("foobar", "^foo(?=bar)"));
        assert!(!simple_pattern_match("foobaz", "^foo(?=bar)"));
        // 先読みは位置を消費しないので、後続のマッチが可能
        assert!(simple_pattern_match("foobar", "^foo(?=bar)bar$"));
    }

    #[test]
    fn test_regex_lookahead_neg() {
        // 否定先読み: foo の後に bar が続かない場合にマッチ
        assert!(simple_pattern_match("foobaz", "^foo(?!bar)"));
        assert!(!simple_pattern_match("foobar", "^foo(?!bar)"));
        // 否定先読みの後に別のパターン
        assert!(simple_pattern_match("foobaz", "^foo(?!bar)baz$"));
    }

#[cfg(test)]
mod hierarchy_tests {
    use super::*;
    use crate::types::{ConstrainedTypeInfo, Type};

    fn make_registry() -> HashMap<String, ConstrainedTypeInfo> {
        let mut registry = HashMap::new();

        // Natural: Int (>= 0)
        registry.insert("Natural".to_string(), ConstrainedTypeInfo {
            name: "Natural".to_string(),
            base_type: Type::Con("Int".to_string()),
            constraints: vec![ConstraintDef::Gte(0)],
        });

        // Percentage: Natural (<= 100)
        // -> 継承後は (>= 0, <= 100)
        registry.insert("Percentage".to_string(), ConstrainedTypeInfo {
            name: "Percentage".to_string(),
            base_type: Type::Con("Natural".to_string()),
            constraints: vec![ConstraintDef::Lte(100)],
        });

        // Priority: Percentage (<= 10)
        // -> 継承後は (>= 0, <= 100, <= 10)
        registry.insert("Priority".to_string(), ConstrainedTypeInfo {
            name: "Priority".to_string(),
            base_type: Type::Con("Percentage".to_string()),
            constraints: vec![ConstraintDef::Lte(10)],
        });

        registry
    }

    #[test]
    fn test_resolve_simple_hierarchy() {
        let registry = make_registry();
        let constraints = resolve_constraint_hierarchy("Natural", &registry);
        assert_eq!(constraints.len(), 1);
        assert!(matches!(constraints[0], ConstraintDef::Gte(0)));
    }

    #[test]
    fn test_resolve_two_level_hierarchy() {
        let registry = make_registry();
        let constraints = resolve_constraint_hierarchy("Percentage", &registry);
        // Natural の (>= 0) + Percentage の (<= 100)
        assert_eq!(constraints.len(), 2);
        assert!(matches!(constraints[0], ConstraintDef::Gte(0)));
        assert!(matches!(constraints[1], ConstraintDef::Lte(100)));
    }

    #[test]
    fn test_resolve_three_level_hierarchy() {
        let registry = make_registry();
        let constraints = resolve_constraint_hierarchy("Priority", &registry);
        // Natural (>= 0) + Percentage (<= 100) + Priority (<= 10)
        assert_eq!(constraints.len(), 3);
        assert!(matches!(constraints[0], ConstraintDef::Gte(0)));
        assert!(matches!(constraints[1], ConstraintDef::Lte(100)));
        assert!(matches!(constraints[2], ConstraintDef::Lte(10)));
    }

    #[test]
    fn test_resolve_nonexistent_type() {
        let registry = make_registry();
        let constraints = resolve_constraint_hierarchy("Unknown", &registry);
        assert!(constraints.is_empty());
    }

    #[test]
    fn test_resolve_base_type_not_constrained() {
        let mut registry = HashMap::new();
        // Port: Int (range 1 65535) - 基底型 Int は制約付き型ではない
        registry.insert("Port".to_string(), ConstrainedTypeInfo {
            name: "Port".to_string(),
            base_type: Type::Con("Int".to_string()),
            constraints: vec![ConstraintDef::Range(1, 65535)],
        });
        let constraints = resolve_constraint_hierarchy("Port", &registry);
        assert_eq!(constraints.len(), 1);
        assert!(matches!(constraints[0], ConstraintDef::Range(1, 65535)));
    }

    #[test]
    fn test_compatibility_valid() {
        let parent = vec![ConstraintDef::Gte(0), ConstraintDef::Lte(200)];
        let child = vec![ConstraintDef::Gte(0), ConstraintDef::Lte(100)];
        let errors = check_constraint_compatibility(&parent, &child);
        assert!(errors.is_empty());
    }

    #[test]
    fn test_compatibility_child_exceeds_parent_lower() {
        let parent = vec![ConstraintDef::Gte(0)];
        let child = vec![ConstraintDef::Gte(-10)];
        let errors = check_constraint_compatibility(&parent, &child);
        assert_eq!(errors.len(), 1);
    }

    #[test]
    fn test_compatibility_child_exceeds_parent_upper() {
        let parent = vec![ConstraintDef::Lte(100)];
        let child = vec![ConstraintDef::Lte(200)];
        let errors = check_constraint_compatibility(&parent, &child);
        assert_eq!(errors.len(), 1);
    }

    #[test]
    fn test_compatibility_range_check() {
        let parent = vec![ConstraintDef::Range(0, 100)];
        let child = vec![ConstraintDef::Range(-5, 50)];
        let errors = check_constraint_compatibility(&parent, &child);
        assert_eq!(errors.len(), 1); // 子の下限が親の範囲外
    }
}

#[cfg(test)]
mod conversion_tests {
    use super::*;
    use crate::types::{ConstrainedTypeInfo, Type};

    fn make_registry() -> HashMap<String, ConstrainedTypeInfo> {
        let mut registry = HashMap::new();

        // Natural: Int (>= 0)
        registry.insert("Natural".to_string(), ConstrainedTypeInfo {
            name: "Natural".to_string(),
            base_type: Type::Con("Int".to_string()),
            constraints: vec![ConstraintDef::Gte(0)],
        });

        // Percentage: Natural (<= 100)
        // 継承後: (>= 0, <= 100)
        registry.insert("Percentage".to_string(), ConstrainedTypeInfo {
            name: "Percentage".to_string(),
            base_type: Type::Con("Natural".to_string()),
            constraints: vec![ConstraintDef::Lte(100)],
        });

        // Priority: Percentage (<= 10)
        // 継承後: (>= 0, <= 100, <= 10)
        registry.insert("Priority".to_string(), ConstrainedTypeInfo {
            name: "Priority".to_string(),
            base_type: Type::Con("Percentage".to_string()),
            constraints: vec![ConstraintDef::Lte(10)],
        });

        // Port: Int (range 1 65535)
        registry.insert("Port".to_string(), ConstrainedTypeInfo {
            name: "Port".to_string(),
            base_type: Type::Con("Int".to_string()),
            constraints: vec![ConstraintDef::Range(1, 65535)],
        });

        registry
    }

    #[test]
    fn test_upcast_child_to_parent() {
        // Percentage -> Natural: Percentage は Natural の子型なのでアップキャスト（安全）
        let registry = make_registry();
        let info = analyze_conversion("Percentage", "Natural", &registry);
        assert_eq!(info.kind, ConversionKind::Upcast);
        assert!(info.extra_checks.is_empty());
    }

    #[test]
    fn test_upcast_grandchild_to_grandparent() {
        // Priority -> Natural: Priority は Natural の孫型なのでアップキャスト（安全）
        let registry = make_registry();
        let info = analyze_conversion("Priority", "Natural", &registry);
        assert_eq!(info.kind, ConversionKind::Upcast);
    }

    #[test]
    fn test_downcast_parent_to_child() {
        // Natural -> Percentage: ダウンキャスト（制約追加チェックが必要）
        let registry = make_registry();
        let info = analyze_conversion("Natural", "Percentage", &registry);
        assert_eq!(info.kind, ConversionKind::Downcast);
        assert!(!info.extra_checks.is_empty());
    }

    #[test]
    fn test_downcast_parent_to_grandchild() {
        // Natural -> Priority: ダウンキャスト（さらに制約が厳しい）
        let registry = make_registry();
        let info = analyze_conversion("Natural", "Priority", &registry);
        assert_eq!(info.kind, ConversionKind::Downcast);
    }

    #[test]
    fn test_incompatible_unrelated_types() {
        // Port -> Priority: 無関係な制約付き型は変換不可
        let registry = make_registry();
        let info = analyze_conversion("Port", "Priority", &registry);
        assert_eq!(info.kind, ConversionKind::Incompatible);
    }

    #[test]
    fn test_same_type_conversion() {
        // Natural -> Natural: 同じ型はアップキャスト（自明に安全）
        let registry = make_registry();
        let info = analyze_conversion("Natural", "Natural", &registry);
        assert_eq!(info.kind, ConversionKind::Upcast);
    }

    #[test]
    fn test_unknown_type_conversion() {
        // Unknown -> Natural: 不明型はアップキャスト（制約なし -> 制約なし相当）
        let registry = make_registry();
        let info = analyze_conversion("Unknown", "Natural", &registry);
        // 不明型は制約が空で、Naturalの制約をパスしない → ダウンキャスト or Incompatible
        // 空制約の子がNaturalの親制約を満たせるかどうかで判定
        assert!(matches!(
            info.kind,
            ConversionKind::Downcast | ConversionKind::Incompatible
        ));
    }
}

#[cfg(test)]
mod runtime_check_tests {
    use super::*;
    use crate::types::{ConstrainedTypeInfo, Type};

    fn make_registry() -> HashMap<String, ConstrainedTypeInfo> {
        let mut registry = HashMap::new();

        // Natural: Int (>= 0)
        registry.insert("Natural".to_string(), ConstrainedTypeInfo {
            name: "Natural".to_string(),
            base_type: Type::Con("Int".to_string()),
            constraints: vec![ConstraintDef::Gte(0)],
        });

        // Percentage: Natural (<= 100)
        registry.insert("Percentage".to_string(), ConstrainedTypeInfo {
            name: "Percentage".to_string(),
            base_type: Type::Con("Natural".to_string()),
            constraints: vec![ConstraintDef::Lte(100)],
        });

        // Email: String (min-length 5, matches "@")
        registry.insert("Email".to_string(), ConstrainedTypeInfo {
            name: "Email".to_string(),
            base_type: Type::Con("String".to_string()),
            constraints: vec![
                ConstraintDef::MinLength(5),
                ConstraintDef::Matches("@".to_string()),
            ],
        });

        // Status: Int (one-of [0, 1, 2])
        registry.insert("Status".to_string(), ConstrainedTypeInfo {
            name: "Status".to_string(),
            base_type: Type::Con("Int".to_string()),
            constraints: vec![ConstraintDef::OneOf(vec![0, 1, 2])],
        });

        registry
    }

    #[test]
    fn test_runtime_checks_natural() {
        let registry = make_registry();
        let check = generate_runtime_checks("Natural", &registry);
        assert_eq!(check.type_name, "Natural");
        assert_eq!(check.conditions.len(), 1);
        assert!(matches!(check.conditions[0], RuntimeCondition::IntGte(0)));
    }

    #[test]
    fn test_runtime_checks_percentage_inherits() {
        // Percentage は Natural の (>= 0) + 自身の (<= 100) を継承
        let registry = make_registry();
        let check = generate_runtime_checks("Percentage", &registry);
        assert_eq!(check.type_name, "Percentage");
        assert_eq!(check.conditions.len(), 2);
        assert!(matches!(check.conditions[0], RuntimeCondition::IntGte(0)));
        assert!(matches!(check.conditions[1], RuntimeCondition::IntLte(100)));
    }

    #[test]
    fn test_runtime_checks_email_string_constraints() {
        let registry = make_registry();
        let check = generate_runtime_checks("Email", &registry);
        assert_eq!(check.type_name, "Email");
        assert_eq!(check.conditions.len(), 2);
        assert!(matches!(check.conditions[0], RuntimeCondition::StrMinLength(5)));
        assert!(matches!(check.conditions[1], RuntimeCondition::StrMatches(_)));
    }

    #[test]
    fn test_runtime_checks_one_of() {
        let registry = make_registry();
        let check = generate_runtime_checks("Status", &registry);
        assert_eq!(check.conditions.len(), 1);
        match &check.conditions[0] {
            RuntimeCondition::IntOneOf(values) => {
                assert_eq!(values, &vec![0, 1, 2]);
            }
            _ => panic!("IntOneOf が期待される"),
        }
    }

    #[test]
    fn test_runtime_checks_unknown_type() {
        let registry = make_registry();
        let check = generate_runtime_checks("Unknown", &registry);
        assert_eq!(check.type_name, "Unknown");
        assert!(check.conditions.is_empty());
    }

    #[test]
    fn test_constraint_to_runtime_satisfies_returns_none() {
        let result = constraint_to_runtime_condition(
            &ConstraintDef::Satisfies("is-valid".to_string()),
        );
        assert!(result.is_none());
    }

    #[test]
    fn test_constraint_to_runtime_range() {
        let result = constraint_to_runtime_condition(&ConstraintDef::Range(1, 100));
        match result {
            Some(RuntimeCondition::IntRange(lo, hi)) => {
                assert_eq!(lo, 1);
                assert_eq!(hi, 100);
            }
            _ => panic!("IntRange が期待される"),
        }
    }
}
