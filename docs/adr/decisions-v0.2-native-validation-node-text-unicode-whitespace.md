# ADR: v0.2 source node text の Unicode whitespace boundary

- Status: Accepted (verified partial slice)
- Date: 2026-07-29
- Scope: selfhost source `:intent` / `:claim` / `:assumption` / `:open-question` の本文
- Related: `EC-M2-01`、`EC-M3-01`、`docs/adr/decisions-v0.2-native-validation-review-unicode-whitespace.md`

## Context

Rust source adapter は node 本文を `trim().is_empty()` で検査するため、NBSP など Unicode
White_Space だけの本文を `InvalidNodeField` として拒否する。一方、selfhost `IntentSource` の
`source-node-nonblank?` は space/tab/LF/CR だけを認識し、Unicode whitespace-only の node 本文を
有効な graph node として受け入れていた。

## Decision

- node 本文の non-blank 判定は `Tools.Validation.Whitespace` の共有 UTF-8 byte helper を使う。
- selfhost の既存 malformed code `1`、node kind/ID、directive span の error boundary は変更しない。
- node ID、review provenance、manifest input、native stage0 artifact/runtime parity はこの slice の
  対象外として残す。

## Evidence

- Rust source adapter に Unicode NBSP node text fixtureを追加し、`InvalidNodeField { field: "text" }`
  と元 value/span を確認した。
- selfhost actual Wasm の同一 fixtureで status `0` / code `1` / kind `7` / node ID / directive span
  を確認した。
- 既存 ASCII whitespace node text、empty-text precedence、type metadata source adapter 回帰を通過した。
- native source-file smoke に Unicode node text fixtureを追加し、stable `source validation error:1`、
  exit `1`、report/manifestなしの fail-closed contract と provenance wrapper を通過した。
- focused gates: `cargo test -p lsharp-types --test validation_source -- --nocapture`（58件）、
  selfhost node source tests、`bash scripts/ci/test-native-linux-x86-native-stage0-source-file-smoke.sh`、
  `bash -n scripts/ci/native-selfhost-dev-source-file-smoke.sh`、`git diff --check`。

## Boundary and follow-up

これは source node text の Unicode non-blank policy に限定した verified partial sliceである。
実 stage0 artifact/runtime、Mac/Linux artifact matrix、manifest/MCP parity、全 node/public surface、
EC-M2-01/EC-M3 aggregate は未完了であり、TODO の `[~]` を維持する。
