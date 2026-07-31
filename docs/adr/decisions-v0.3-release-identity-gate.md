# ADR: v0.3 native release evidence identity gate

- Status: Accepted (verified partial slice)
- Date: 2026-07-31
- Scope: native-only release archive と packaged stage0 の明示 `review_evidence_identity` 入力
- Related: [`decisions-v0.3-review-evidence-manifest-identity.md`](decisions-v0.3-review-evidence-manifest-identity.md)、
  [`v0.3-milestone-01.md`](../development/planning/v0.3-milestone-01.md)

## Context

Rust/selfhost の manifest identity は確定しているが、native release が source commit や artifact
bytes と別の identity を受け取ると、packaged artifact の provenance を再現できない。provider の
network/helper を release smoke へ暗黙に入れることも、offline な target gate の意味を壊す。

## Decision

- [`verify-native-release-identity.py`](../../scripts/ci/verify-native-release-identity.py) を共通の
  offline verifier とする。`subject_digest`、`source_commit`、`artifact_digest`、
  `trust_store_digest`、`lifecycle_digest`、`now` の順序と型を fail-closed に検証し、source commit と
  actual artifact の SHA-256 を明示入力へ突き合わせる。
- `--require-provider-input` を指定した release gate では keyset/lifecycle digest の `null` を
  `unverified` として exit `2` にする。provider adapter はこの JSON を作る外部入力境界であり、verifier
  は network、environment、current checkout、Rust/host helper から digest を補完しない。
- native-only `scripts/release.sh` は、`NATIVE_ONLY_REVIEW_EVIDENCE_IDENTITY` が指定された場合に
  verifier の canonical projectionを `review-evidence-identity.json` と archive manifest の
  `review_evidence_identity` へ同じ値で格納する。`release-smoke.sh` はその payload が存在する
  archive で identity、source commit、program bytes、manifest の再 attach を検証し、manifestだけ
  の宣言や digest mismatch を拒否する。未指定の旧 archive は後方互換の unverified boundary として
  identity検証を行わない。
- `package-native-stage0-release.sh` は同じ explicit identity を optional input として stage0
  manifest/packageへ投影する。`native-official-release-local.sh` は artifact/stage0 directoryに
  `review-evidence-identity.json` がある場合だけこの入力を伝播し、明示された provider snapshot は
  下流の release/package/smoke verifierへ渡す。

## Evidence

- RED: `scripts/ci/test-native-release-identity.py` が verifier未実装時に失敗し、release、release
  smoke、stage0 package の共通 gate marker と identity conflict を先に固定した。
- GREEN: 同テスト（5 tests）、native release packaging fixture、
  `scripts/ci/test-native-stage0-release-package.sh`、`scripts/ci/test-native-stage0-package.sh`、
  `scripts/ci/test-native-release-input-bundle.py` が passした。`bash -n` で変更 shell scriptも検査した。
- identity field order、artifact SHA-256 mismatch、provider digest 欠落、manifest/file conflict は
  offline fixtureで再現可能な failure boundary として固定した。

## Boundary and follow-up

これは release identity の producer/packaging/smoke contract の verified partial sliceである。multi-target
orchestrator の offline snapshot propagation は [`decisions-v0.3-native-official-multitarget-snapshot-wiring.md`](decisions-v0.3-native-official-multitarget-snapshot-wiring.md)
へ分離して記録した。
provider adapter の実取得、current-source の Mac Apple Silicon / Linux x86_64 native stage0 replay、
両 target の packaged runtime、`verified/unverified/stale/revoked/invalid` の実行時 matrix は未完了。
active Linux native replay と競合しないため、この runでは heavy replayを起動していない。`TODO.md` の
`EC-M3-04` / `EC-M3-05` は `[~]` のまま維持する。
