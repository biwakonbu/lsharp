# ADR: v0.2 evidence coverage count の負値 boundary

- Status: Accepted (verified partial slice)
- Date: 2026-07-29
- Scope: source `:evidence` の optional `:coverage` entry count
- Related: `EC-M2-02`、`docs/adr/decisions-v0.2-native-validation-evidence-coverage-bucket.md`

## Context

現在の契約で確定している coverage count の数値条件は非負である。一方、Rust parser の
`parse_evidence_coverage`、selfhost source parser→Evidence registry、native source-file smoke の
同一 fixtureによる負値回帰が不足していた。`coverage` の全 count と `cases` の合計を一致させる
意味論は既存仕様で決まっていないため、この slice では導入しない。

## Decision

- Rust source parser は負の coverage count を fail-closed にする。複数 parse error の集約結果として
  stable code `LS0104` を返す。
- selfhost source parser→Evidence registry は負の coverage count を invalid-sampling code `11`、
  field `coverage`、bucket value、non-empty directive/form span 付きで拒否する。
- native source-file validation は exit `1`、`source validation error:11`、report/manifestなしを要求する。
- `sum(coverage counts) == cases`、count の上限、manifest input、current-source stage0 artifact/runtime、
  supported target matrix は未決定または未検証のため、この slice の対象外とする。

## Evidence

- Rust `intent_edges` に負の coverage count fixtureを追加し、`LS0104` を確認した。
- selfhost actual Wasm の source parser→Evidence registry 経路で負値 fixtureを実行し、status `0`、
  code `11`、field `coverage`、`[negative]`、非空 span を確認した。
- native source-file smoke に負値 fixture、exit/report/manifest assertions を追加した。
- `bash -n scripts/ci/native-selfhost-dev-source-file-smoke.sh`、
  `bash -n scripts/ci/test-native-linux-x86-native-stage0-source-file-smoke.sh`、
  `bash scripts/ci/test-native-linux-x86-native-stage0-source-file-smoke.sh` を通過した。

## Boundary and follow-up

これは負の coverage count の parser/consumer boundary に限定した verified partial sliceである。
coverage count/cases の意味論、非負 count の上限、manifest/validate、current-source stage0
artifact/runtime、Mac/Linux matrix、EC-M2-02 aggregate は未完了であり、TODO の `[~]` を維持する。
