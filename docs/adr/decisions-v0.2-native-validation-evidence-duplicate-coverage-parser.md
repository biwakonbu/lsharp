# ADR: v0.2 native validation duplicate coverage parser boundary

- Status: Accepted (verified partial slice)
- Date: 2026-07-29
- Scope: Rust `:evidence` coverage parser contract and native source-file smoke
- Related: `EC-M2-02`、`docs/adr/decisions-v0.2-selfhost-evidence-parser-duplicate.md`、`docs/adr/decisions-v0.2-native-validation-evidence-coverage-whitespace.md`

## Context

coverage bucket の重複は source parser の時点で拒否する既存契約がある。Rust parser は
`LS0101` として duplicate bucket を返し、selfhost Evidence consumer の直接 registry 境界は
code `10` で拒否する。一方、native stage0 source-file smoke にはこの parser-owned boundary の
fixtureがなく、source → validate の no-report/no-manifest 契約を target runtime で確認できていなかった。

## Decision

- duplicate coverage bucket は parser-owned boundary とし、source adapter に後段の `BTreeMap`
  duplicate 検査を追加しない。parser を通過した canonical source record には duplicate bucket が
存在しないため、source adapter の責務を重複再判定へ拡張しない。
- native source-file smoke は duplicate coverage fixtureに対して stable parser error code `1`、
  exit `1`、report/manifestなしを要求する。
- selfhost direct Evidence registry の vector payload duplicate policy（code `10`）は既存の
  parser-independent consumer boundary として維持する。

## Evidence

- RED/識別: Rust source adapter に duplicate fixtureを追加して実行すると、adapter ではなく
  parser が `unique :evidence coverage buckets` / `LS0101` で先に拒否することを確認した。
- Rust parser: `cargo test -p lsharp-syntax --test intent_edges duplicate -- --nocapture`（1 passed）。
  同テスト内で duplicate coverage bucket の `LS0101` を検証する。
- Selfhost actual Wasm: `test_e2e_selfhost_evidence_registry_rejects_duplicate_coverage_bucket`（1 passed、
  code `10`、field `coverage`、bucket value、form span）。
- Native: `bash scripts/ci/test-native-linux-x86-native-stage0-source-file-smoke.sh` が
  Linux x86_64 source-file smoke と provenance gate を通過した。

## Boundary and follow-up

この判断は parser-owned duplicate coverage の native contract に限定した verified partial sliceである。
canonical `SamplingPlan` の直接 map は duplicate を表現できず、coverage count/cases の意味論、
Unicode whitespace、manifest/validate 全体、current-source artifact/runtime、Mac/Linux supported
matrix、EC-M2-02 aggregate は未完了であり、TODO の `[~]` を維持する。
