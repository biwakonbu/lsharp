//! 正規表現エンジン
//!
//! NFA ベースのバックトラッキングマッチャと、
//! NFA→DFA 変換による高速マッチを提供する。
//! 後方参照・先読みは DFA で表現不可なため NFA にフォールバック。

pub(crate) mod dfa;
mod matcher;
mod node;
mod parser;

#[allow(unused_imports)]
pub(crate) use matcher::has_advanced_features;
pub(crate) use matcher::simple_pattern_match;
pub(crate) use node::{RegexNode, node_matches_char};
pub(crate) use parser::parse_regex;

#[cfg(test)]
mod tests;
