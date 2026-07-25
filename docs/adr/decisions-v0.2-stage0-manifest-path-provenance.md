# ADR: stage0 manifest の payload path provenance boundary

- Status: Accepted (verified partial slice)
- Date: 2026-07-25
- Scope: `scripts/native-selfhost-dev.sh`, `scripts/ci/test-native-selfhost-dev.sh`
- Related: `v0.2-milestone-03.md`, `docs/development/operations/rust-boundary-reduction.md`

## Context

native stage0 は `manifest.json` の `compiler`、`transport_driver`、`materializer` を
package 内の payload として読み込む。source commit の一致だけを検査しても、manifest の
相対 path が package 外を指すと、stage0 の provenance boundary を越えて任意の host file を
実行できる。

## Decision

- stage0 manifest の payload path は非空の相対 path とし、絶対 path、`.`、`..` を含む path、
  package 外へ正規化される path を native runner の実行前に拒否する。
- 拒否時は `error: stage0 manifest <field> must be a relative path` を stderr に返し、transport
  driver、materializer、`program.native` を起動しない。
- この boundary は source commit / target / stage0 file fingerprint の検証に加わる防御線であり、
  実 target の artifact/runtime evidence や rollback archive の検証を代替しない。

## Evidence

- `bash scripts/ci/test-native-selfhost-dev.sh`
  （`native selfhost dev runner tests: OK`）。fixture の `compiler: ../outside/compiler` を
  source commit が一致する stage0 に投入し、明示拒否と program 未起動を確認した。
- `bash -n scripts/ci/test-native-selfhost-dev.sh scripts/native-selfhost-dev.sh`
  と `git diff --check` を通過した。

## Boundary and follow-up

これは fake stage0 fixture による provenance/path の verified slice である。Mac Apple Silicon
と Linux x86_64 の current-source stage0、public acquisition、release artifact/runtime、
emergency rollback、Rust oracle/differential は引き続き未完了であり、M3 aggregate の完了判定を
変更しない。
