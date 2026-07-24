# ADR: `regex/mod.rs` の production 責務分離

- Status: Accepted (verified partial slice)
- Date: 2026-07-25
- Scope: `crates/lsharp-types/src/regex/`
- Related: `I-01`, `I-08`, `imp-06-large-file-decomposition.md`

## Context

テスト分離後も `regex/mod.rs` に AST (`RegexNode`)、parser、文字判定、NFA backtracking matcher、capture/backreference、DFA 選択の入口が混在していた。production の責務境界を明確にし、次の parser/matcher 改修が同じファイルへ集中しない構造が必要だった。

## Decision

- `node.rs` に `RegexNode` と単一ノードの文字判定を置く。
- `parser.rs` に `parse_regex` と quantifier/group/alternation の parser helpers を置く。
- `matcher.rs` に NFA matcher、capture/backreference、step limit、advanced-feature 判定、DFA/NFA の公開入口を置く。
- `mod.rs` は `dfa` と各 production module の宣言、既存 crate-private API の re-export、test module 宣言だけを担当する。
- `dfa.rs` の sibling API と既存の `crate::regex::*` 呼び出し経路は維持し、挙動・型・診断契約は変更しない。

## Evidence

- focused `cargo test -p lsharp-types regex:: -- --nocapture`: 38 passed（regex 25、DFA 13）。
- `cargo test -p lsharp-types`: unit 209、integration 3/4/1/6/30/3/2 が全て pass。
- 行数は `mod.rs` 19、`node.rs` 74、`parser.rs` 294、`matcher.rs` 699（既存 `dfa.rs` 699、`tests.rs` 200）。
- `cargo clippy -p lsharp-types --all-targets -- -D warnings`、changed files の rustfmt check、`git diff --check` が pass。

## Consequences

parser と matcher を独立してレビュー・変更でき、`mod.rs` は module boundary と API re-export に縮小した。parser の expr/decl 分割、matcher algorithm の改善、`I-01` / `I-08` aggregate、native/selfhost parity は後続であり、この verified slice だけで完了扱いにはしない。
