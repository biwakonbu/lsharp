# ADR: v0.2 review provenance digest の Unicode whitespace boundary

- Status: Accepted (verified partial slice)
- Date: 2026-07-29
- Scope: selfhost source `:review` の `provenance_digest` required field
- Related: `EC-M2-02`、`EC-M3-01`、`docs/adr/decisions-v0.2-native-validation-evidence-unicode-whitespace.md`

## Context

Rust source adapter は review の `provenance_digest` を `trim().is_empty()` で検査するため、
NBSP など Unicode White_Space だけの digest を拒否する。一方、selfhost `IntentSource` の
`source-review-nonblank?` は space/tab/LF/CR だけを認識し、Unicode whitespace-only digest を
有効な opaque review registry entry として受け入れていた。

## Decision

- review provenance digest の non-blank 判定は `Tools.Validation.Whitespace` の共有 helper を使う。
- selfhost source adapter の既存 invalid-review code `8`、review ID、directive span の error boundary
  は変更しない。
- review visibility、review lifecycle/authentication、native stage0 artifact/runtime parity はこの
  sliceの対象外として残す。

## Evidence

- Rust source adapter に Unicode NBSP digest fixtureを追加し、
  `InvalidReviewField { field: "provenance_digest" }` を確認した。
- selfhost actual Wasm の同一 NBSP digest fixtureで status `0` / code `8` / review ID を確認した。
- 既存の空 digest、invalid visibility、invalid review ID precedence の selfhost testsも helper
  注入後に通過した。
- native source-file smoke に Unicode digest fixtureを追加し、stable `source validation error:8`、
  exit `1`、report/manifestなしの fail-closed contract と provenance wrapper を通過した。
- focused gates: `cargo test -p lsharp-types --test validation_source -- --nocapture`（57件）、
  selfhost review source tests、`bash scripts/ci/test-native-linux-x86-native-stage0-source-file-smoke.sh`、
  `bash -n scripts/ci/native-selfhost-dev-source-file-smoke.sh`、`git diff --check`。

## Boundary and follow-up

これは review provenance digest の Unicode non-blank policy に限定した verified partial sliceである。
実 stage0 artifact/runtime、Mac/Linux artifact matrix、provider authentication、review lifecycle、
manifest/MCP parity、EC-M2-02/EC-M3 aggregate は未完了であり、TODO の `[~]` を維持する。
