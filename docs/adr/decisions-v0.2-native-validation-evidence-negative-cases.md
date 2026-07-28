# ADR: v0.2 evidence `cases` の負値 boundary

- Status: Accepted (verified partial slice)
- Date: 2026-07-29
- Scope: source `:evidence` の sampling `:cases`
- Related: `EC-M2-02`、`docs/adr/decisions-v0.2-native-validation-evidence-negative-coverage-count.md`

## Context

evidence source metadata の `:cases` は canonical sampling の非負整数だが、Rust parser、selfhost
source parser→Evidence registry、native `validate` の同一負値 fixtureが未接続だった。manifest decoder
では既に unsigned JSON boundary が固定されている。coverage count の合計と `cases` の関係は既存仕様で
決まっていないため、この slice では意味論を追加しない。

## Decision

- Rust source parser は `:cases -1` を parser `Unexpected` の stable code `LS0101` で拒否する。
- selfhost source parser→Evidence registry は `cases < 0` を invalid-sampling code `11`、field `cases`、
  empty raw value、non-empty directive/form span 付きで拒否する。
- native source-file validation は exit `1`、`source validation error:11`、report/manifestなしを要求する。
- `sum(coverage counts) == cases`、count/cases の上限、manifest input、current-source stage0
  artifact/runtime、supported target matrix は未決定または未検証のため対象外とする。

## Evidence

- Rust `intent_edges` に負の cases fixtureを追加し、`LS0101` を確認した。
- selfhost actual Wasm の source parser→Evidence registry 経路で負値 fixtureを実行し、status `0`、
  code `11`、field `cases`、empty raw value、非空 span を確認した。
- native source-file smoke に負値 fixture、exit/report/manifest assertions を追加した。
- `bash -n` 両 smoke script と `bash scripts/ci/test-native-linux-x86-native-stage0-source-file-smoke.sh`
  を通過した。

## Boundary and follow-up

これは evidence `cases` の parser/consumer boundary に限定した verified partial sliceである。
coverage count/cases の意味論、非負値の上限、manifest/validate、current-source stage0 artifact/runtime、
Mac/Linux matrix、EC-M2-02 aggregate は未完了であり、TODO の `[~]` を維持する。
