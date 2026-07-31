# ADR: v0.3 native-official multi-target の provider snapshot wiring

- Status: Accepted (verified partial slice)
- Date: 2026-07-31
- Scope: `native-official-release-local.sh` の Mac Apple Silicon / Linux x86_64 propagation
- Related: [`decisions-v0.3-stage0-release-smoke-snapshot-wiring.md`](decisions-v0.3-stage0-release-smoke-snapshot-wiring.md)、
  [`v0.3-milestone-01.md`](../development/planning/v0.3-milestone-01.md)

## Context

N7 で stage0 package と release-smoke は explicit snapshot を検証できるようになったが、
multi-target local gate が同じ snapshot を targetごとに渡さなければ、Mac と Linux の release
evidence が別の provider input を見る余地が残る。Linux smoke は Lima VM 内で実行されるため、
host path をそのまま渡さず、検証済みの bytes を VM へ明示コピーする必要もある。

## Decision

- orchestrator は `NATIVE_OFFICIAL_REVIEW_TRUST_STORE` と
  `NATIVE_OFFICIAL_REVIEW_LIFECYCLE` を all-or-none の任意入力として受け取る。指定時は両方とも
  non-empty regular file であることを開始時に確認する。
- 同じ snapshot path を両 target の `scripts/release.sh` へ
  `NATIVE_ONLY_REVIEW_TRUST_STORE` / `NATIVE_ONLY_REVIEW_LIFECYCLE` として渡し、stage0 package へ
  `--review-trust-store` / `--review-lifecycle` として渡す。identity file がない場合は各下流 gateの
  fail-closed 契約に委ね、snapshotを黙って無視しない。
- Mac の release-smoke には `RELEASE_REVIEW_TRUST_STORE` /
  `RELEASE_REVIEW_LIFECYCLE` を同じ host path で渡す。Linux は snapshot bytes を VM 内の固定名へ
  `limactl copy` し、VM 内の release-smoke には VM path を渡す。verifier とその timestamp helperも
  同じ VM work directoryへコピーする。
- rollback compatibility archive には snapshot を渡さない。provider adapter、network、implicit
  trust root は成功経路に入れない。

## Evidence

- RED: `test-native-release-identity.py` の release-surface contract が orchestrator の snapshot
  env/option wiring 不在で失敗した。
- GREEN: `test-native-official-release-snapshots.sh` の fake two-target gate が両 targetの release、
  stage0 package、Mac smoke、Linux VM copy/env、stage0 fetchへ同じ snapshotを伝播すること、片側指定を
  rejectすることを確認した。
- `bash -n`、identity Python suite（7 tests）、fake orchestrator gate、`git diff --check` が passした。

## Boundary and follow-up

これは multi-target offline propagation の verified partial sliceである。実 provider取得/authentication、
current-source の Mac Apple Silicon / Linux x86_64 stage0再生成、両 target の実 packaged runtime、
snapshot digestとsource/artifact provenanceの実 runtime比較は未完了であり、`EC-M3-05` は `[~]` のまま残す。
