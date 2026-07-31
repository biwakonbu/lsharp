# ADR: v0.3 native source-file smoke の evidence identity projection

- Status: Accepted (verified partial slice)
- Date: 2026-07-31
- Scope: `scripts/ci/native-selfhost-dev-source-file-smoke.sh` の `validate` identity contract
- Related: [`decisions-v0.3-review-evidence-identity.md`](decisions-v0.3-review-evidence-identity.md)、
  [`v0.3-milestone-01.md`](../development/planning/v0.3-milestone-01.md)、`EC-M3-05`

## Context

Rust CLI/MCP と selfhost EmbeddedCli には、明示した subject、source commit、artifact、clock と
任意の trust/lifecycle digest を `review_evidence_identity` へ投影する契約がある。しかし native
source-file smoke がこの境界を確認しなければ、native producer が identity を落としたり、暗黙の
digestを補ったりしても検知できない。

さらに positional version 1 manifest input は、これまで graph の trace metrics だけを再投影し、
manifest に既存 identity があっても report へ戻していなかった。caller が渡す identity と既存
manifest identity の不一致を検知できないまま続行すると、evidence の owner/context が別物へ
差し替わる。

## Decision

- 既存の `:review-attestation` fixtureへ explicit review identity options を渡し、JSON report と
  manifest が同じ field order/value を返すことを要求する。
- trust-store/lifecycle digest を省略した場合は `null` を JSON/manifest へ投影し、text では `-`
  と表示する。system clock、environment、current checkout、manifestからの推測は行わない。
- subject/source/artifact/now の一部だけを指定した場合は、stable な all-or-none diagnostic で
  exit `1`、stdout 空、manifestなしに fail-closed する。
- review 自体に verification input がない fixtureは identity を付けても `unverified` のまま
  `unknown` (exit `2`) とし、identity を `verified` shortcut として扱わない。
- identity options を一つも渡さない後方互換 route は、JSON report / manifest に
  `review_evidence_identity` を暗黙生成せず、system clock・environment・checkout・manifestから
  値を補わない。明示 context route だけが identity を投影する。
- positional manifest input は canonical identity object の required/nullable fields を
  fail-closed に読み取り、既存 identity と caller の明示 identity が byte-equivalent なら
  report へ再 attach する。片側だけの identity は欠落側を補うが、両方が異なる場合は stable
  `source validation error:14`、exit `1`、stdout 空、manifest 出力なしで停止する。

## Evidence

- RED: `test_native_review_identity_is_never_implicit_without_explicit_context` を先に追加し、
  identity options のない native JSON/manifest に暗黙 identity が混入しないことを示す marker が
  source smoke にない状態で失敗した。
- GREEN: identity options なしの report/manifest の両方で `review_evidence_identity` の不在を検査し、
  Linux fake Lima/provenance harness が `Linux native stage0 source-file provenance tests: OK` で通過した。
- RED: `scripts/ci/test-native-linux-x86-native-stage0-source-file-smoke.sh` が identity markerを
  先に要求し、実装前に `VALIDATION_IDENTITY_MANIFEST` 欠落で失敗した。
- GREEN: native source smokeへ full/optional identity の JSON+manifest、text projection、partial
  identity rejection を追加し、fake Lima/provenance harness が
  `Linux native stage0 source-file provenance tests: OK` で通過した。
- RED: `test_native_review_identity_manifest_input_reattaches_and_rejects_conflicts` を先に追加し、
  positional manifest の既存 identity を再 attach し、conflicting caller identity を拒否する
  marker が source smoke にない状態で失敗した。
- GREEN: `ManifestInput.ls` が canonical identity object を fixed-wire fields として scoped に
  読み取り、`App.Cli` が同値 identity の再 attach、conflict の code `14` fail-closed、report/
  manifest no-output を実装した。Rust-host の `test_e2e_selfhost_manifest_input_retrieves_existing_review_identity`
  と Linux fake Lima/provenance harness が通過した。
- `bash -n scripts/ci/native-selfhost-dev-source-file-smoke.sh scripts/ci/test-native-linux-x86-native-stage0-source-file-smoke.sh`
  と `git diff --check` が通過した。

## Boundary

これは native source-file smoke の contract/harness evidence であり、current checkoutとsource
commitが一致する packaged stage0の実 runtime evidenceではない。Mac Apple Silicon / Linux x86_64
の current-source artifact、native MCP、release identity gate、verified/stale/revoked/invalid の
target matrix は未完了であり、`EC-M3-05` は `[~]` のまま維持する。
