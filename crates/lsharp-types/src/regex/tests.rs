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
fn test_regex_bounded_quantifiers() {
    assert!(simple_pattern_match("aaa", "^a{3}$"));
    assert!(!simple_pattern_match("aa", "^a{3}$"));

    assert!(simple_pattern_match("aa", "^a{2,4}$"));
    assert!(simple_pattern_match("aaaa", "^a{2,4}$"));
    assert!(!simple_pattern_match("a", "^a{2,4}$"));
    assert!(!simple_pattern_match("aaaaa", "^a{2,4}$"));

    assert!(simple_pattern_match("aaaaa", "^a{2,}$"));
    assert!(!simple_pattern_match("a", "^a{2,}$"));
}

#[test]
fn test_regex_shorthand_negated_classes() {
    assert!(simple_pattern_match("abc", "^\\D+$"));
    assert!(!simple_pattern_match("123", "^\\D+$"));

    assert!(simple_pattern_match("!?", "^\\W+$"));
    assert!(!simple_pattern_match("abc_123", "^\\W+$"));

    assert!(simple_pattern_match("visible", "^\\S+$"));
    assert!(!simple_pattern_match("has space", "^\\S+$"));
}

#[test]
fn test_regex_non_capturing_group_does_not_shift_backreference() {
    assert!(simple_pattern_match("abcdcd", "^(?:ab)(cd)\\1$"));
    assert!(!simple_pattern_match("abcdab", "^(?:ab)(cd)\\1$"));
}

#[test]
fn test_regex_lazy_quantifier_suffix_is_accepted() {
    assert!(simple_pattern_match("aaa", "^a+?$"));
    assert!(simple_pattern_match("", "^a*?$"));
    assert!(simple_pattern_match("a", "^a??$"));
    assert!(simple_pattern_match("aaa", "^a{2,4}?$"));
    assert!(!simple_pattern_match("aaaaa", "^a{2,4}?$"));
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
    assert!(simple_pattern_match(
        "\u{3053}\u{3093}\u{306B}\u{3061}\u{306F}",
        "^\\p{L}+$"
    ));
    assert!(!simple_pattern_match("123", "^\\p{L}+$"));
}

#[test]
fn test_unicode_number() {
    assert!(simple_pattern_match("123", "^\\p{N}+$"));
    assert!(!simple_pattern_match("abc", "^\\p{N}+$"));
}

#[test]
fn test_unicode_letter_number_combined() {
    assert!(simple_pattern_match(
        "\u{540D}\u{524D}42",
        "^\\p{L}+\\p{N}+$"
    ));
    assert!(!simple_pattern_match(
        "42\u{540D}\u{524D}",
        "^\\p{L}+\\p{N}+$"
    ));
}

#[test]
fn test_unicode_negated() {
    assert!(simple_pattern_match("123", "^\\P{L}+$"));
    assert!(!simple_pattern_match("abc", "^\\P{L}+$"));
    assert!(simple_pattern_match("42!", "^\\P{L}+$"));
}
