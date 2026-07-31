# ADR: native official gate の provider identity preflight

- Status: Accepted (verified partial slice)
- Date: 2026-07-31
- Scope: `EC-M3-05-N9` / `scripts/ci/native-official-release-local.sh`
- Related: [`decisions-v0.3-native-official-source-commit-preflight.md`](decisions-v0.3-native-official-source-commit-preflight.md)

## Context

provider の trust-store と lifecycle snapshot は、review evidence identity と組でなければ
digest を release artifact へ結び付けられない。従来の multi-target orchestrator は snapshot
の all-or-none だけを開始時に検査し、Mac/Linux の App.Cli または stage0 input に identity file
が欠けていても一方の packaging を開始してから下流の verifier で失敗していた。

## Decision

trust-store/lifecycle snapshot が明示された場合、orchestrator の packaging 前に次の4入力を
すべて non-empty regular file として検査する。

- Mac Apple Silicon App.Cli artifact の `review-evidence-identity.json`
- Linux x86_64 App.Cli artifact の `review-evidence-identity.json`
- Mac Apple Silicon stage0 package input の `review-evidence-identity.json`
- Linux x86_64 stage0 package input の `review-evidence-identity.json`

一つでも欠ける場合は `review evidence identity is required when provider snapshots are supplied`
で fail-closed に停止し、release、stage0 package、provider helper、release smoke、Lima VM を
開始しない。snapshot 未指定時の旧 archive は従来どおり unverified compatibility boundary
として受理する。

## Evidence

- RED: fake two-target gate で Mac App.Cli identity を一つ外すと、実装前は release/package/smoke
  invocationへ進んだ。
- GREEN: 同じ gate が identity 欠落を開始時に拒否し、invocation log が増えないことを確認する。
- `bash -n scripts/ci/native-official-release-local.sh scripts/ci/test-native-official-release-snapshots.sh`
  と `bash scripts/ci/test-native-official-release-snapshots.sh` が pass。

## Boundary

これは provider snapshot と identity の入力 closure だけを閉じる operator preflight であり、
provider API/authentication、actual Mac/Linux stage0 runtime、artifact digest の target parity、
rollback、Wasm runtime の evidenceではない。`EC-M3-05-N9` は引き続き partial のまま維持する。
