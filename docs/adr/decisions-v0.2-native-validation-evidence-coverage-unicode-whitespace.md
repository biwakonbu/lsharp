# ADR: v0.2 evidence coverage bucket の Unicode whitespace boundary

- Status: Accepted (verified partial slice)
- Date: 2026-07-29
- Scope: source `:evidence` の optional `:coverage` bucket
- Related: `EC-M2-02`、`EC-M3-01`、`docs/adr/decisions-v0.2-native-validation-evidence-coverage-whitespace.md`

## Context

Rust の canonical `SamplingPlan` と source adapter は coverage bucket を `trim().is_empty()` で
検査するため、NBSP など Unicode White_Space だけの bucket を空値として拒否する。一方、selfhost
Evidence consumer の coverage boundary は ASCII whitespace の回帰しか持たず、source parser 経路で
Unicode whitespace-only bucket を受け入れてしまう可能性があった。

## Decision

- coverage bucket の non-blank 判定は `Tools.Validation.Whitespace` の共有 UTF-8 byte helper を
  `IntentSource` 経由で利用し、Rust の Unicode White_Space policy と揃える。
- selfhost の既存 empty-field code `4`、raw bucket value、evidence directive/form span を維持する。
- bucket count/cases の整合性、duplicate bucket、manifest/validate input、native stage0 artifact/runtime
  parity はこの slice の対象外として残す。

## Evidence

- Rust source adapter に Unicode NBSP coverage bucket fixtureを追加し、
  `InvalidEvidenceField { field: "coverage" }` と元 value/span を確認した。
- selfhost actual Wasm の source parser→Evidence registry 経路で同一 fixtureを実行し、status `0`、
  code `4`、raw bucket value、非空 diagnostic span を確認した。
- native source-file smoke に Unicode coverage bucket fixtureを追加し、stable
  `source validation error:4`、exit `1`、report/manifestなしの fail-closed contract と source
  provenance wrapper を通過した。
- focused gates: Rust source adapter test、selfhost Unicode coverage E2E、
  `bash scripts/ci/test-native-linux-x86-native-stage0-source-file-smoke.sh`、`bash -n`。

## Boundary and follow-up

これは coverage bucket の Unicode non-blank policy に限定した verified partial slice である。
実 stage0 artifact/runtime、Mac/Linux artifact matrix、manifest/MCP parity、coverage count/cases、
EC-M2-02/EC-M3 aggregate は未完了であり、TODO の `[~]` を維持する。
