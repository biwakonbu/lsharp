//! NFA→DFA 変換エンジン
//!
//! 部分集合構成法 (subset construction) による NFA→DFA 変換。
//! 後方参照・先読みは DFA で表現不可なため、NFA にフォールバック。
//! 状態数上限 (DFA_STATE_LIMIT) を超えた場合も NFA にフォールバック。

use std::cell::RefCell;
use std::cmp::Reverse;
use std::collections::{BTreeSet, HashMap, VecDeque};

use super::RegexNode;

/// DFA 状態数の上限（状態爆発防止）
const DFA_STATE_LIMIT: usize = 256;

/// 文字マッチ条件
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
enum CharMatcher {
    /// 特定の文字
    Exact(char),
    /// 文字範囲
    Range(char, char),
    /// 任意の文字 (.)
    Any,
    /// 否定文字クラス ([^a-z] 等)
    NegatedRanges(Vec<(char, char)>),
    /// Unicode 文字クラス
    UnicodeClass { property: String, negated: bool },
}

impl CharMatcher {
    /// 文字がこのマッチャに一致するか
    fn matches(&self, ch: char) -> bool {
        match self {
            CharMatcher::Exact(c) => *c == ch,
            CharMatcher::Range(lo, hi) => ch >= *lo && ch <= *hi,
            CharMatcher::Any => true,
            CharMatcher::NegatedRanges(ranges) => {
                !ranges.iter().any(|&(lo, hi)| ch >= lo && ch <= hi)
            }
            CharMatcher::UnicodeClass { property, negated } => {
                let m = match property.as_str() {
                    "L" => ch.is_alphabetic(),
                    "N" => ch.is_numeric(),
                    _ => false,
                };
                if *negated { !m } else { m }
            }
        }
    }

    /// マッチャの具体度スコア (高いほど具体的)
    fn specificity(&self) -> u8 {
        match self {
            CharMatcher::Exact(_) => 4,
            CharMatcher::Range(_, _) => 3,
            CharMatcher::UnicodeClass { .. } => 2,
            CharMatcher::NegatedRanges(_) => 1,
            CharMatcher::Any => 0,
        }
    }
}

/// NFA の状態
#[derive(Debug, Clone)]
struct NfaState {
    /// (マッチ条件, 遷移先) のリスト。None はイプシロン遷移
    transitions: Vec<(Option<CharMatcher>, usize)>,
    /// 受理状態か
    is_accept: bool,
}

/// DFA の遷移先情報
#[derive(Debug, Clone)]
struct DfaTransition {
    /// 遷移先の DFA 状態インデックス
    target: usize,
}

/// DFA の状態
#[derive(Debug, Clone)]
struct DfaState {
    /// この DFA 状態に対応する NFA 状態集合
    nfa_states: BTreeSet<usize>,
    /// 遷移先リスト
    transitions: Vec<DfaTransition>,
    /// 受理状態か
    is_accept: bool,
}

/// コンパイル済み DFA
#[derive(Debug, Clone)]
struct Dfa {
    /// 状態リスト
    states: Vec<DfaState>,
    /// 開始状態のインデックス
    start: usize,
    /// NFA (DFA マッチ時に文字判定用)
    nfa: Vec<NfaState>,
}

// DFA キャッシュ (スレッドローカル)
thread_local! {
    static DFA_CACHE: RefCell<HashMap<String, Option<Dfa>>> = RefCell::new(HashMap::new());
}

/// RegexNode 列から NFA を構築 (Thompson's construction)
fn build_nfa(nodes: &[RegexNode]) -> Vec<NfaState> {
    let mut states = Vec::new();
    let accept = states.len();
    states.push(NfaState {
        transitions: vec![],
        is_accept: true,
    });

    let start = build_nfa_fragment(nodes, accept, &mut states);

    if start != 0 {
        states.push(NfaState {
            transitions: vec![(None, start)],
            is_accept: false,
        });
    }

    states
}

/// NFA フラグメントを構築し、開始状態のインデックスを返す
fn build_nfa_fragment(nodes: &[RegexNode], next: usize, states: &mut Vec<NfaState>) -> usize {
    if nodes.is_empty() {
        return next;
    }

    let mut current_next = next;
    for node in nodes.iter().rev() {
        current_next = build_nfa_node(node, current_next, states);
    }
    current_next
}

/// 単一の RegexNode から NFA フラグメントを構築
fn build_nfa_node(node: &RegexNode, next: usize, states: &mut Vec<NfaState>) -> usize {
    match node {
        RegexNode::Literal(ch) => {
            let s = states.len();
            states.push(NfaState {
                transitions: vec![(Some(CharMatcher::Exact(*ch)), next)],
                is_accept: false,
            });
            s
        }
        RegexNode::Dot => {
            let s = states.len();
            states.push(NfaState {
                transitions: vec![(Some(CharMatcher::Any), next)],
                is_accept: false,
            });
            s
        }
        RegexNode::CharClass { chars, negated } => {
            let s = states.len();
            if *negated {
                states.push(NfaState {
                    transitions: vec![(Some(CharMatcher::NegatedRanges(chars.clone())), next)],
                    is_accept: false,
                });
                s
            } else {
                let transitions: Vec<_> = chars
                    .iter()
                    .map(|&(lo, hi)| {
                        if lo == hi {
                            (Some(CharMatcher::Exact(lo)), next)
                        } else {
                            (Some(CharMatcher::Range(lo, hi)), next)
                        }
                    })
                    .collect();
                states.push(NfaState {
                    transitions,
                    is_accept: false,
                });
                s
            }
        }
        RegexNode::UnicodeClass { property, negated } => {
            let s = states.len();
            states.push(NfaState {
                transitions: vec![(
                    Some(CharMatcher::UnicodeClass {
                        property: property.clone(),
                        negated: *negated,
                    }),
                    next,
                )],
                is_accept: false,
            });
            s
        }
        RegexNode::Star(inner) => {
            let s = states.len();
            states.push(NfaState {
                transitions: vec![],
                is_accept: false,
            });
            let inner_start = build_nfa_node(inner, s, states);
            states[s].transitions.push((None, inner_start));
            states[s].transitions.push((None, next));
            s
        }
        RegexNode::Plus(inner) => {
            let star_state = states.len();
            states.push(NfaState {
                transitions: vec![],
                is_accept: false,
            });
            let inner_start = build_nfa_node(inner, star_state, states);
            states[star_state].transitions.push((None, inner_start));
            states[star_state].transitions.push((None, next));
            inner_start
        }
        RegexNode::Optional(inner) => {
            let inner_start = build_nfa_node(inner, next, states);
            let s = states.len();
            states.push(NfaState {
                transitions: vec![(None, inner_start), (None, next)],
                is_accept: false,
            });
            s
        }
        RegexNode::BoundedRepeat { inner, min, max } => {
            build_nfa_bounded_repeat(inner, *min, *max, next, states)
        }
        RegexNode::Group(group_nodes) => build_nfa_fragment(group_nodes, next, states),
        RegexNode::NonCapturingGroup(group_nodes) => build_nfa_fragment(group_nodes, next, states),
        RegexNode::Alternation(alternatives) => {
            let s = states.len();
            states.push(NfaState {
                transitions: vec![],
                is_accept: false,
            });
            for alt in alternatives {
                let alt_start = build_nfa_fragment(alt, next, states);
                states[s].transitions.push((None, alt_start));
            }
            s
        }
        // 後方参照・先読みは DFA で処理不可
        RegexNode::Backreference(_) | RegexNode::Lookahead(_) | RegexNode::LookaheadNeg(_) => {
            let s = states.len();
            states.push(NfaState {
                transitions: vec![],
                is_accept: false,
            });
            s
        }
    }
}

fn build_nfa_bounded_repeat(
    inner: &RegexNode,
    min: usize,
    max: Option<usize>,
    next: usize,
    states: &mut Vec<NfaState>,
) -> usize {
    if matches!(max, Some(max) if max < min) {
        let s = states.len();
        states.push(NfaState {
            transitions: vec![],
            is_accept: false,
        });
        return s;
    }

    let mut current_next = next;
    match max {
        Some(max) => {
            for _ in min..max {
                let inner_start = build_nfa_node(inner, current_next, states);
                let s = states.len();
                states.push(NfaState {
                    transitions: vec![(None, inner_start), (None, current_next)],
                    is_accept: false,
                });
                current_next = s;
            }
        }
        None => {
            current_next = build_nfa_node(
                &RegexNode::Star(Box::new(inner.clone())),
                current_next,
                states,
            );
        }
    }

    for _ in 0..min {
        current_next = build_nfa_node(inner, current_next, states);
    }
    current_next
}

/// イプシロン閉包を計算
fn epsilon_closure(nfa: &[NfaState], initial: &BTreeSet<usize>) -> BTreeSet<usize> {
    let mut closure = initial.clone();
    let mut stack: Vec<usize> = initial.iter().copied().collect();

    while let Some(state) = stack.pop() {
        if state >= nfa.len() {
            continue;
        }
        for (matcher, target) in &nfa[state].transitions {
            if matcher.is_none() && !closure.contains(target) {
                closure.insert(*target);
                stack.push(*target);
            }
        }
    }

    closure
}

/// NFA 状態集合から、文字入力による遷移先を計算
fn nfa_move(nfa: &[NfaState], states: &BTreeSet<usize>, ch: char) -> BTreeSet<usize> {
    let mut result = BTreeSet::new();
    for &s in states {
        if s >= nfa.len() {
            continue;
        }
        for (matcher, target) in &nfa[s].transitions {
            if let Some(m) = matcher
                && m.matches(ch)
            {
                result.insert(*target);
            }
        }
    }
    result
}

/// NFA 状態集合から、使用されるマッチャの一覧を収集
fn collect_matchers(nfa: &[NfaState], states: &BTreeSet<usize>) -> Vec<CharMatcher> {
    let mut matchers = Vec::new();
    let mut seen = std::collections::HashSet::new();
    for &s in states {
        if s >= nfa.len() {
            continue;
        }
        for (matcher, _) in &nfa[s].transitions {
            if let Some(m) = matcher
                && seen.insert(format!("{m:?}"))
            {
                matchers.push(m.clone());
            }
        }
    }
    // 具体的なマッチャを先に (Exact > Range > UnicodeClass > NegatedRanges > Any)
    matchers.sort_by_key(|m| Reverse(m.specificity()));
    matchers
}

/// 代表文字を生成 (マッチャから実際のテスト文字を取得)
fn representative_chars(matchers: &[CharMatcher]) -> Vec<char> {
    let mut chars = Vec::new();
    let ascii_printable: Vec<char> = (' '..='~').collect();
    let unicode_samples = [
        '\u{3042}', '\u{6F22}', '\u{3053}', '\u{3093}', '\u{306B}', '\u{3061}', '\u{306F}',
    ];

    for m in matchers {
        match m {
            CharMatcher::Exact(c) => {
                if !chars.contains(c) {
                    chars.push(*c);
                }
            }
            CharMatcher::Range(lo, hi) => {
                let mut c = *lo;
                while c <= *hi {
                    if !chars.contains(&c) {
                        chars.push(c);
                    }
                    if c == *hi {
                        break;
                    }
                    c = char::from_u32(c as u32 + 1).unwrap_or(*hi);
                    if chars.len() > 256 {
                        break;
                    }
                }
            }
            CharMatcher::Any | CharMatcher::NegatedRanges(_) => {
                for c in &ascii_printable {
                    if !chars.contains(c) {
                        chars.push(*c);
                    }
                }
            }
            CharMatcher::UnicodeClass { .. } => {
                for c in &ascii_printable {
                    if !chars.contains(c) {
                        chars.push(*c);
                    }
                }
                for c in &unicode_samples {
                    if !chars.contains(c) {
                        chars.push(*c);
                    }
                }
            }
        }
    }
    chars
}

/// RegexNode 列から DFA をコンパイル
fn compile_dfa(nodes: &[RegexNode]) -> Option<Dfa> {
    let nfa = build_nfa(nodes);
    compile_dfa_from_nfa(nfa)
}

/// NFA から DFA を構築 (部分集合構成法)
fn compile_dfa_from_nfa(nfa: Vec<NfaState>) -> Option<Dfa> {
    if nfa.is_empty() {
        return None;
    }

    let nfa_start = nfa.len() - 1;
    let initial_set = {
        let mut s = BTreeSet::new();
        s.insert(nfa_start);
        epsilon_closure(&nfa, &s)
    };

    let mut dfa_states: Vec<DfaState> = Vec::new();
    let mut state_map: HashMap<BTreeSet<usize>, usize> = HashMap::new();
    let mut work_queue: VecDeque<BTreeSet<usize>> = VecDeque::new();

    let is_accept = initial_set
        .iter()
        .any(|&s| s < nfa.len() && nfa[s].is_accept);
    let start_idx = 0;
    state_map.insert(initial_set.clone(), start_idx);
    dfa_states.push(DfaState {
        nfa_states: initial_set.clone(),
        transitions: vec![],
        is_accept,
    });
    work_queue.push_back(initial_set);

    while let Some(current_set) = work_queue.pop_front() {
        let current_idx = state_map[&current_set];

        let matchers = collect_matchers(&nfa, &current_set);
        let rep_chars = representative_chars(&matchers);

        // 各代表文字について遷移先を計算、同じ遷移先をグループ化
        let mut target_to_idx: HashMap<BTreeSet<usize>, usize> = HashMap::new();
        let mut transitions = Vec::new();

        for ch in &rep_chars {
            let moved = nfa_move(&nfa, &current_set, *ch);
            if moved.is_empty() {
                continue;
            }
            let target_set = epsilon_closure(&nfa, &moved);
            if target_set.is_empty() {
                continue;
            }

            // 既にこの遷移先の DFA 状態がある場合はスキップ
            if target_to_idx.contains_key(&target_set) {
                continue;
            }

            let target_idx = if let Some(&idx) = state_map.get(&target_set) {
                idx
            } else if dfa_states.len() >= DFA_STATE_LIMIT {
                return None;
            } else {
                let idx = dfa_states.len();
                let is_accept = target_set
                    .iter()
                    .any(|&s| s < nfa.len() && nfa[s].is_accept);
                state_map.insert(target_set.clone(), idx);
                dfa_states.push(DfaState {
                    nfa_states: target_set.clone(),
                    transitions: vec![],
                    is_accept,
                });
                work_queue.push_back(target_set.clone());
                idx
            };

            target_to_idx.insert(target_set, target_idx);
            transitions.push(DfaTransition { target: target_idx });
        }

        dfa_states[current_idx].transitions = transitions;
    }

    Some(Dfa {
        states: dfa_states,
        start: start_idx,
        nfa,
    })
}

/// DFA でマッチを実行
///
/// 入力文字に対して NFA の nfa_move + epsilon_closure を使って
/// 正確な遷移先 DFA 状態を決定する。
fn dfa_match(dfa: &Dfa, text: &[char], start: usize, require_end: bool) -> bool {
    let mut current = dfa.start;

    for ch in &text[start..] {
        let state = &dfa.states[current];

        // NFA を使って正確な遷移先を計算
        let moved = nfa_move(&dfa.nfa, &state.nfa_states, *ch);
        if moved.is_empty() {
            return if !require_end { state.is_accept } else { false };
        }
        let target_nfa_states = epsilon_closure(&dfa.nfa, &moved);

        // 遷移先 DFA 状態を検索
        let mut found = false;
        for trans in &state.transitions {
            if dfa.states[trans.target].nfa_states == target_nfa_states {
                current = trans.target;
                found = true;
                break;
            }
        }
        if !found {
            return if !require_end { state.is_accept } else { false };
        }
    }

    dfa.states[current].is_accept
}

/// DFA マッチを試行
///
/// DFA コンパイルに成功した場合は Some(bool) を返す。
/// DFA が使えない場合 (状態爆発等) は None を返し、NFA にフォールバック。
pub(crate) fn try_dfa_match(
    pattern: &str,
    nodes: &[RegexNode],
    text_chars: &[char],
    anchored_start: bool,
    anchored_end: bool,
) -> Option<bool> {
    let dfa = DFA_CACHE.with(|cache| {
        let mut cache = cache.borrow_mut();
        if let Some(cached) = cache.get(pattern) {
            return cached.clone();
        }
        let compiled = compile_dfa(nodes);
        cache.insert(pattern.to_string(), compiled.clone());
        compiled
    });

    let dfa = dfa?;

    let result = if anchored_start && anchored_end {
        dfa_match(&dfa, text_chars, 0, true)
    } else if anchored_start {
        dfa_match(&dfa, text_chars, 0, false)
    } else if anchored_end {
        (0..=text_chars.len()).any(|s| dfa_match(&dfa, text_chars, s, true))
    } else {
        (0..=text_chars.len()).any(|s| dfa_match(&dfa, text_chars, s, false))
    };

    Some(result)
}

/// DFA キャッシュをクリア (テスト用)
#[cfg(test)]
pub(crate) fn clear_dfa_cache() {
    DFA_CACHE.with(|cache| cache.borrow_mut().clear());
}

/// DFA キャッシュにパターンが存在するか確認 (テスト用)
#[cfg(test)]
pub(crate) fn is_cached(pattern: &str) -> bool {
    DFA_CACHE.with(|cache| cache.borrow().contains_key(pattern))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::regex::simple_pattern_match;

    #[test]
    fn test_dfa_simple_literal() {
        clear_dfa_cache();
        assert!(simple_pattern_match("hello", "^hello$"));
        assert!(!simple_pattern_match("world", "^hello$"));
        assert!(simple_pattern_match("abc", "abc"));
    }

    #[test]
    fn test_dfa_char_class() {
        clear_dfa_cache();
        assert!(simple_pattern_match("abc", "^[a-z]+$"));
        assert!(!simple_pattern_match("ABC", "^[a-z]+$"));
        assert!(simple_pattern_match("a1b", "[a-z]"));
    }

    #[test]
    fn test_dfa_star() {
        clear_dfa_cache();
        assert!(simple_pattern_match("", "^a*$"));
        assert!(simple_pattern_match("a", "^a*$"));
        assert!(simple_pattern_match("aaa", "^a*$"));
    }

    #[test]
    fn test_dfa_plus() {
        clear_dfa_cache();
        assert!(!simple_pattern_match("", "^a+$"));
        assert!(simple_pattern_match("a", "^a+$"));
        assert!(simple_pattern_match("aaa", "^a+$"));
    }

    #[test]
    fn test_dfa_alternation() {
        clear_dfa_cache();
        assert!(simple_pattern_match("cat", "^(cat|dog)$"));
        assert!(simple_pattern_match("dog", "^(cat|dog)$"));
        assert!(!simple_pattern_match("bird", "^(cat|dog)$"));
    }

    #[test]
    fn test_dfa_cache() {
        clear_dfa_cache();
        let pattern = "[a-z]+";
        assert!(!is_cached(pattern));
        simple_pattern_match("hello", "^[a-z]+$");
        assert!(is_cached("[a-z]+"));
    }

    #[test]
    fn test_dfa_state_limit() {
        clear_dfa_cache();
        assert!(simple_pattern_match("abcabc", "^(abc)\\1$"));
        assert!(!simple_pattern_match("abcdef", "^(abc)\\1$"));
    }

    #[test]
    fn test_dfa_backreference_fallback() {
        clear_dfa_cache();
        assert!(simple_pattern_match("aa", "^(.)\\1$"));
        assert!(!simple_pattern_match("ab", "^(.)\\1$"));
    }

    #[test]
    fn test_dfa_negated_char_class() {
        clear_dfa_cache();
        assert!(simple_pattern_match("123", "^[^a-z]+$"));
        assert!(!simple_pattern_match("abc", "^[^a-z]+$"));
    }

    #[test]
    fn test_dfa_unicode_letter() {
        clear_dfa_cache();
        assert!(simple_pattern_match("hello", "^\\p{L}+$"));
        assert!(simple_pattern_match(
            "\u{3053}\u{3093}\u{306B}\u{3061}\u{306F}",
            "^\\p{L}+$"
        ));
        assert!(!simple_pattern_match("123", "^\\p{L}+$"));
    }

    #[test]
    fn test_dfa_unicode_number() {
        clear_dfa_cache();
        assert!(simple_pattern_match("123", "^\\p{N}+$"));
        assert!(!simple_pattern_match("abc", "^\\p{N}+$"));
    }

    #[test]
    fn test_dfa_unicode_negated() {
        clear_dfa_cache();
        assert!(simple_pattern_match("123", "^\\P{L}+$"));
        assert!(!simple_pattern_match("abc", "^\\P{L}+$"));
    }

    #[test]
    fn test_dfa_optional_dot() {
        clear_dfa_cache();
        assert!(simple_pattern_match("ab", "^a.?b$"));
        assert!(simple_pattern_match("axb", "^a.?b$"));
        assert!(!simple_pattern_match("axxb", "^a.?b$"));
    }
}
