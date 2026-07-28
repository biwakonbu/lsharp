# ADR: v0.2 native validation node stable ID span

- Status: Accepted (verified partial slice)
- Date: 2026-07-29
- Scope: `crates/lsharp-types/src/validation_source.rs`, `crates/lsharp-types/src/validation_source/source_nodes.rs`, `crates/lsharp-types/tests/validation_source/nodes.rs`, `crates/lsharp-wasm/tests/e2e/selfhost_intent_source_adapter.rs`
- Related: `EC-M2-01`、`docs/adr/decisions-v0.2-native-validation-node-text-span.md`

## Context

source metadata の node stable ID が wire format／segment 規則に違反した場合、Rust adapter は
`SourceGraphError::Node(IntentNodeError::StableId(...))` に直接落としていた。そのため selfhost の
stable code `2` と directive span に対応する source-local diagnostic がなく、不正 ID を source
metadata の位置へ結び付けられなかった。

## Decision

- `SourceGraphError::NodeIdAt` を追加し、`StableIdError` と node directive span を保持する。
- `IntentNode::from_wire_parts` の `IntentNodeError::StableId` だけを `NodeIdAt` へ投影する。
- `KindMismatch` と graph-owned `Node` error は既存の境界を維持し、node text の `InvalidNodeField` と
  stable-ID parse failure を別診断として扱う。

## Evidence

- RED: source node test が `NodeIdAt` と directive span を要求し、variant 未実装で compile error になった。
- GREEN: `cargo test -p lsharp-types --test validation_source -- --nocapture`（53 tests）。
- Selfhost actual Wasm: malformed node ID の stable code `2`、ID、directive span E2E が通過した。
- Native contract: `bash scripts/ci/test-native-linux-x86-native-stage0-source-file-smoke.sh`
  が通過し、invalid node ID の code `2`、exit `1`、no-report/no-manifest boundary を維持した。
- Touched Rust files の `rustfmt --edition 2021 --check` と `git diff --check` が通過した。

## Boundary and follow-up

これは Rust source adapter の node stable-ID diagnostic と既存 selfhost/native contract の verified
partial sliceである。driver の EmbeddedCli build は current `origin/main` の
`selfhost/src/Tools/Validation/Stale.ls` にある既存 `vector-push-single-rooted-v3` undefined error
で停止するため未検証とした。実 stage0 artifact/runtime、Mac/Linux matrix、native fallback exclusion、
M2 aggregate completion は未完了であり、TODO の `[~]` を維持する。
