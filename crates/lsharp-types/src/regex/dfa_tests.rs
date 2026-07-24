//! DFA backend の回帰テスト

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
