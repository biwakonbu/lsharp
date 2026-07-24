# ADR: `regex/dfa.rs` inline test 分離

- Status: Accepted (verified partial slice)
- Date: 2026-07-25
- Scope: `crates/lsharp-types/src/regex/dfa.rs`
- Related: `I-01`, `I-08`, `imp-06-large-file-decomposition.md`, `decisions-legacy-regex-production-split.md`

## Context

`regex/dfa.rs` は NFA→DFA backend、thread-local cache、backreference fallback の production code と 13 件の DFA 回帰テストを同じファイルに保持していた。DFA の production implementation と regex feature fixture を分離すると、regex backend の差分を小さくレビューできる。

## Decision

- `CharMatcher`、NFA/DFA construction、cache、`simple_pattern_match` への接続は変更しない。
- `#[cfg(test)] mod tests` の 13 件を `crates/lsharp-types/src/regex/dfa_tests.rs` へ移動する。
- `dfa.rs` は `#[path = "dfa_tests.rs"] mod tests;` で既存の `regex::dfa::tests` namespace を維持する。
- regex parser/matcher production split、その他の type inference/constraints split、I-01 / I-08 aggregate は別タスクとして残す。

## Evidence

- 分離前後の `regex::dfa::tests` focused gate: 13 passed。
- `RUST_MIN_STACK=33554432 cargo test -p lsharp-types`: 209 unit tests、49 integration/property tests passed。
- `dfa.rs` は 699 行から 594 行へ、`regex/dfa_tests.rs` は 107 行となった。
- `RUST_MIN_STACK=33554432 cargo clippy -p lsharp-types --all-targets -- -D warnings`、対象 files の rustfmt check、`git diff --check`: pass。

## Consequences

regex DFA の production/cache boundary と test-only feature coverage が明確になり、13 件の回帰テストを単独で再実行できる。regex parser/matcher、infer、constraints、その他の大規模 Rust file と I-01 / I-08 aggregate は未完了であるため、TODO の partial slice を維持する。
