# ADR: Linux native stage0 source-file smoke の VM lifecycle cleanup

- Date: 2026-07-31
- Status: Accepted (verified partial slice)
- Scope: `M3-05-N9` / `EC-M3-05` / `scripts/ci/native-linux-x86-native-stage0-source-file-smoke.sh`

## Context

official multi-target gate の Linux stage0 runtime smoke は、停止中の Lima VM を `ensure_vm_running`
で起動する。しかし従来は、その VM をこの smoke が所有したかを記録せず、source-file smoke 終了時に
VM を stop していなかった。既に Running の共有 VM を無条件に stop することも、別セッションの
native replay を壊すため許されない。

## Decision

stage0 source-file smoke は orchestrator と同じ ownership contract を持つ。

- `limactl list` が `Running` 以外で、smoke 自身の `limactl start` が成功した場合だけ
  `VM_STARTED_BY_SMOKE=1` とする。
- `EXIT` cleanup で、所有して起動した VM だけを stop する。既に Running の VM は停止しない。
- work directory cleanup と VM stop のどちらかが失敗した場合、元の smoke failure があればその
  status を保持し、smoke が成功していた場合は cleanup failure として non-zero を返す。
- `KEEP_NATIVE_STAGE0_SOURCE_SMOKE_WORK_DIR=1` は work directory の保持だけを許可し、owned VM の
  lifecycle cleanup は省略しない。

## Evidence

- RED: fake `limactl` の Stopped 状態で、stage0 smoke が gate-owned VM の stop invocation を出さず
  テストが失敗した。
- GREEN: Stopped/Running 両状態を fake harness で実行し、owned VM の start→stop、共有 Running VM
  の no-stop を確認した。
- fake stop failure は smoke success を success と報告せず、cleanup error と non-zero を返す。
- `bash -n scripts/ci/native-linux-x86-native-stage0-source-file-smoke.sh scripts/ci/test-native-linux-x86-native-stage0-source-file-smoke.sh`
  と `bash scripts/ci/test-native-linux-x86-native-stage0-source-file-smoke.sh` が pass。

この証拠は fake Lima operator contract に限られる。現在実行中の Linux x86_64 VM replay、current-source
stage0 の実 runtime、provider/authentication、packaged artifact、rollback/Wasm parity は未取得であり、
N9 と EC-M3-05 は `[~]` のまま残す。

## Consequences

stage0 source-file smoke が起動した VM は run 終了時に解放され、共有 VM は保護される。実 gate では
既存の VM-side lock、free-space gate、artifact reuse を維持し、他セッションの VM/worktree/temp を
cleanup 対象に含めない。
