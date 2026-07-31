# ADR: official multi-target gate の Linux VM smoke cleanup

- Date: 2026-07-31
- Status: Accepted (verified partial slice)
- Scope: `M3-05-N9` / `EC-M3-05` / `scripts/ci/native-official-release-local.sh`

## Context

Linux x86_64 の release smoke は Lima VM 内に archive、rollback archive、helper、snapshot を置く
一時 work directory を作る。従来は smoke command が終了した後の正常経路でだけ `rm -rf` を実行して
いたため、archive/helper の copy 失敗や VM 内 setup 失敗では task-owned directory が残り得た。
これは N9 の「実行後に VM・temporary archive・process を回収する」operator boundary と一致しない。

## Decision

Linux smoke の VM work directory を専用 subshell の `EXIT` trap で回収する。

- work directory の作成後から archive/helper/snapshot copy、smoke command の成否にかかわらず
  `limactl shell <vm> -- rm -rf <work-dir>` を一度実行する。
- cleanup 自体の失敗は元の smoke/setup failure を隠さず、元の non-zero status を親関数へ返す。
- Mac host smoke、外部 provider の取得/authentication、実 target runtime の証拠には変更を加えない。

## Evidence

- RED: fake `limactl` の archive copy を失敗させたとき、copy 後の VM work directory cleanup invocation
  が存在せずテストが失敗した。
- GREEN: 同じ fake two-target harness が copy failure を non-zero で保持し、copy invocation の後に
  同じ VM work directoryを削除する invocation があることを確認する。
- `bash -n scripts/ci/native-official-release-local.sh scripts/ci/test-native-official-release-snapshots.sh`
  と `bash scripts/ci/test-native-official-release-snapshots.sh` が pass。

この証拠は offline fake operator cleanup contract に限られる。実 Lima VM、current-source stage0、
provider snapshot digest、packaged App.Cli、rollback/Wasm parity は未取得であり、N9 と EC-M3-05 は
`[~]` のまま残す。

## Consequences

copy/setup/smoke の途中失敗でも task-owned VM work directory が残りにくくなる。実 target gate では
既存の VM-side lock と artifact reuse を維持し、別セッションの VM、worktree、temporary directory を
cleanup 対象へ含めない。
