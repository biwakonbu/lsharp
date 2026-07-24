/// 正規表現パターン要素
///
/// サポートする構文:
/// - リテラル文字
/// - `^` 先頭アンカー
/// - `$` 末尾アンカー
/// - `.` 任意の1文字
/// - `*` 直前の0回以上
/// - `+` 直前の1回以上
/// - `{n}` / `{n,m}` / `{n,}` 回数指定
/// - `*?` / `+?` / `??` / `{...}?` 非貪欲サフィックス
/// - `[...]` 文字クラス（ネストなし、`^` 否定あり）
/// - `\d` / `\w` / `\s` と否定形 `\D` / `\W` / `\S`
/// - `\p{L}` Unicode Letter, `\p{N}` Unicode Number
/// - `\P{L}` Unicode Letter 以外, `\P{N}` Unicode Number 以外
#[derive(Debug, Clone)]
pub(crate) enum RegexNode {
    /// リテラル文字
    Literal(char),
    /// 任意の1文字 (.)
    Dot,
    /// 文字クラス ([a-z], [abc])
    CharClass {
        chars: Vec<(char, char)>,
        negated: bool,
    },
    /// 0回以上の繰り返し (*)
    Star(Box<RegexNode>),
    /// 1回以上の繰り返し (+)
    Plus(Box<RegexNode>),
    /// 0回または1回 (?)
    Optional(Box<RegexNode>),
    /// min 回以上 max 回以下の繰り返し。max=None は上限なし。
    BoundedRepeat {
        inner: Box<RegexNode>,
        min: usize,
        max: Option<usize>,
    },
    /// グループ ((...))
    Group(Vec<RegexNode>),
    /// 非キャプチャグループ ((?:...))
    NonCapturingGroup(Vec<RegexNode>),
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
