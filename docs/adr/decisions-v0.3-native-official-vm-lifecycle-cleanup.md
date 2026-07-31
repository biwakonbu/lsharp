# ADR: official multi-target gate の Lima VM lifecycle ownership

- Date: 2026-07-31
- Status: Accepted (verified partial slice)
- Scope: `M3-05-N9` / `EC-M3-05` / `scripts/ci/native-official-release-local.sh`

## Context

Linux x86_64 release smoke は、対象 VM が停止している場合に `limactl start` を実行するが、
従来は smoke 完了後も VM を起動状態のまま残していた。逆に、既に Running の共有 VM を gate が
無条件に停止すると、別の native replay や別セッションの作業を壊す。

## Decision

gate は VM lifecycle の所有権を一 run 内で記録する。

- `limactl list` が `Running` 以外で、gate 自身の `limactl start` が成功した場合だけ
  `vm_started_by_gate=1` とする。
- VM work directory の cleanup と同じ `EXIT` trap で、所有して起動した VM だけを
  `limactl stop` する。
- 既に Running の VM は停止しない。
- cleanup の失敗は stderr に明示し、元の setup/smoke が失敗していればその exit status を隠さない。
  smoke が成功していて cleanup に失敗した場合は gate 自体を non-zero にする。

## Evidence

- RED: fake Lima が `Stopped` を返す正常 two-target gateで、`limactl stop` が記録されずテストが失敗した。
- GREEN: 同じ fake gate が自分で起動した VM の stop を記録し、`FAKE_LIMA_RUNNING=1` の再実行では
  stop を記録しないことを確認した。
- 既存の copy failure fixtureでも non-zero status と VM work directory cleanup invocation を維持した。
- fake `limactl stop` failure では、smoke 成功を success と報告せず cleanup failure を stderr/exit `1`
  へ投影することを確認した。
- `bash -n scripts/ci/native-official-release-local.sh scripts/ci/test-native-official-release-snapshots.sh`
  と `bash scripts/ci/test-native-official-release-snapshots.sh` が pass。

この証拠は fake operator lifecycle contract に限られる。実 Lima VM、current-source stage0、provider
取得/authentication、packaged artifact、rollback/Wasm parity は未取得であり、N9 と EC-M3-05 は
`[~]` のまま残す。

## Consequences

gate が所有した VM は run 終了時に解放され、共有中の VM は保護される。cleanup 失敗を success と
して隠さないため、operator は残置状態を再実行前に扱える。実 target gate では既存の VM-side
lock/artifact reuse を維持し、他セッション所有の VM・worktree・temporary directory を所有物として
扱わない。
