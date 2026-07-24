use super::RegexNode;

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
                // 先読み構文のチェック: (?=...), (?!...), (?:...)
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
                    } else if i + 1 < chars.len() && chars[i + 1] == ':' {
                        // 非キャプチャグループ (?:...)
                        i += 2;
                        let (group_nodes, end) = parse_regex_inner(chars, i);
                        i = end;
                        if group_nodes
                            .iter()
                            .any(|n| matches!(n, RegexNode::Literal('|')))
                        {
                            RegexNode::Alternation(split_alternatives(group_nodes))
                        } else {
                            RegexNode::NonCapturingGroup(group_nodes)
                        }
                    } else {
                        // 通常のグループとして処理
                        let (group_nodes, end) = parse_regex_inner(chars, i);
                        i = end;
                        if group_nodes
                            .iter()
                            .any(|n| matches!(n, RegexNode::Literal('|')))
                        {
                            RegexNode::Alternation(split_alternatives(group_nodes))
                        } else {
                            RegexNode::Group(group_nodes)
                        }
                    }
                } else {
                    let (group_nodes, end) = parse_regex_inner(chars, i);
                    i = end;
                    // グループ内の | を処理
                    if group_nodes
                        .iter()
                        .any(|n| matches!(n, RegexNode::Literal('|')))
                    {
                        RegexNode::Alternation(split_alternatives(group_nodes))
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
                if negated {
                    i += 1;
                }
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
                if i < chars.len() {
                    i += 1;
                } // ']'
                RegexNode::CharClass {
                    chars: ranges,
                    negated,
                }
            }
            '\\' => {
                i += 1;
                if i < chars.len() {
                    let ch = chars[i];
                    i += 1;
                    match ch {
                        'd' => RegexNode::CharClass {
                            chars: vec![('0', '9')],
                            negated: false,
                        },
                        'D' => RegexNode::CharClass {
                            chars: vec![('0', '9')],
                            negated: true,
                        },
                        'w' => RegexNode::CharClass {
                            chars: vec![('a', 'z'), ('A', 'Z'), ('0', '9'), ('_', '_')],
                            negated: false,
                        },
                        'W' => RegexNode::CharClass {
                            chars: vec![('a', 'z'), ('A', 'Z'), ('0', '9'), ('_', '_')],
                            negated: true,
                        },
                        's' => RegexNode::CharClass {
                            chars: vec![(' ', ' '), ('\t', '\t'), ('\n', '\n'), ('\r', '\r')],
                            negated: false,
                        },
                        'S' => RegexNode::CharClass {
                            chars: vec![(' ', ' '), ('\t', '\t'), ('\n', '\n'), ('\r', '\r')],
                            negated: true,
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
                                if i < chars.len() {
                                    i += 1;
                                } // '}'
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

        let node = parse_quantifier(chars, &mut i, node);

        nodes.push(node);
    }

    (nodes, i)
}

fn split_alternatives(nodes: Vec<RegexNode>) -> Vec<Vec<RegexNode>> {
    let mut alternatives = vec![vec![]];
    for node in nodes {
        if matches!(node, RegexNode::Literal('|')) {
            alternatives.push(vec![]);
        } else {
            alternatives.last_mut().unwrap().push(node);
        }
    }
    alternatives
}

fn parse_quantifier(chars: &[char], i: &mut usize, node: RegexNode) -> RegexNode {
    if *i >= chars.len() {
        return node;
    }

    let quantified = match chars[*i] {
        '*' => {
            *i += 1;
            RegexNode::Star(Box::new(node))
        }
        '+' => {
            *i += 1;
            RegexNode::Plus(Box::new(node))
        }
        '?' => {
            *i += 1;
            RegexNode::Optional(Box::new(node))
        }
        '{' => {
            if let Some((min, max)) = parse_bounded_quantifier(chars, i) {
                RegexNode::BoundedRepeat {
                    inner: Box::new(node),
                    min,
                    max,
                }
            } else {
                return node;
            }
        }
        _ => return node,
    };

    if *i < chars.len() && chars[*i] == '?' {
        *i += 1;
    }
    quantified
}

fn parse_bounded_quantifier(chars: &[char], i: &mut usize) -> Option<(usize, Option<usize>)> {
    let original = *i;
    *i += 1; // '{'
    let min_start = *i;
    while *i < chars.len() && chars[*i].is_ascii_digit() {
        *i += 1;
    }
    if min_start == *i {
        *i = original;
        return None;
    }
    let min: usize = chars[min_start..*i]
        .iter()
        .collect::<String>()
        .parse()
        .ok()?;

    let max = if *i < chars.len() && chars[*i] == ',' {
        *i += 1;
        let max_start = *i;
        while *i < chars.len() && chars[*i].is_ascii_digit() {
            *i += 1;
        }
        if max_start == *i {
            None
        } else {
            Some(
                chars[max_start..*i]
                    .iter()
                    .collect::<String>()
                    .parse()
                    .ok()?,
            )
        }
    } else {
        Some(min)
    };

    if *i < chars.len() && chars[*i] == '}' {
        *i += 1;
        Some((min, max))
    } else {
        *i = original;
        None
    }
}
