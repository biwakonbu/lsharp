# ADR: v0.2 native validation node text span

- Status: Accepted (verified partial slice)
- Date: 2026-07-29
- Scope: `crates/lsharp-types/src/validation_source.rs`, `crates/lsharp-types/src/validation_source/source_nodes.rs`, `crates/lsharp-types/tests/validation_source/nodes.rs`, `crates/lsharp-wasm/tests/e2e/selfhost_intent_source_adapter.rs`
- Related: `EC-M2-01`、`docs/adr/decisions-v0.2-native-validation-evidence-required-span.md`

## Context

source metadata の node `text` が空または whitespace-only の場合、Rust adapter は
`SourceGraphError::Node(IntentNodeError::NodeText(...))` に直接落としていた。そのため selfhost が
返す stable malformed code `1` と directive span に対応する source-local diagnostic がなく、空本文を
不正 stable ID より先に拒否する precedence も source span 付きで表現できなかった。

## Decision

- `SourceGraphError::InvalidNodeField` を追加し、field、value、node directive span を保持する。
- node `text` の trim-empty 検査は `IntentNode` construction 前に行い、field は `text` とする。
- existing `SourceGraphError::Node` は graph-owned node construction の canonical error boundary として残し、
  source metadata の blank text だけを source-local variant へ投影する。

## Evidence

- RED: source node tests が `InvalidNodeField` と directive span を要求し、variant 未実装で compile error になった。
- GREEN: `cargo test -p lsharp-types --test validation_source -- --nocapture`（52 tests）。
- Selfhost actual Wasm: whitespace-only node text の stable code `1`、kind/ID、directive span E2E が通過した。
- Native contract: `bash scripts/ci/test-native-linux-x86-native-stage0-source-file-smoke.sh`
  が通過し、空／whitespace-only node text の code `1`、exit `1`、no-report/no-manifest boundary を維持した。
- Touched Rust files の `rustfmt --edition 2021 --check` と `git diff --check` が通過した。

## Boundary and follow-up

これは Rust source adapter の node text diagnostic と既存 selfhost/native contract の verified
partial sliceである。driver の EmbeddedCli build は current `origin/main` の
`selfhost/src/Tools/Validation/Stale.ls` にある既存 `vector-push-single-rooted-v3` undefined error
で停止するため未検証とした。実 stage0 artifact/runtime、Mac/Linux matrix、native fallback exclusion、
M2 aggregate completion は未完了であり、TODO の `[~]` を維持する。
