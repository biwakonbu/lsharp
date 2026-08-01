# ADR: native official gate の provider identity schema preflight

- Status: Accepted (verified partial slice)
- Date: 2026-08-02
- Scope: `EC-M3-05-N2` / `EC-M3-05-N9` / `scripts/ci/native-official-release-local.sh`
- Related: [`decisions-v0.3-native-official-provider-source-commit-binding.md`](decisions-v0.3-native-official-provider-source-commit-binding.md)

## Context

source commit binding は provider identity を current checkout に結び付けるが、identity が
`source_commit` だけの不完全 JSON でも downstream packaging へ進める余地が残っていた。
canonical field order、provider snapshot digest、strict timestamp などの意味検証は既存の
`verify-native-release-identity.py` が正本として持っている。

## Decision

provider snapshot が指定された official gate は、Mac/Linux の App.Cli と stage0 4入力について、
既存 verifier を `--source-commit`、trust-store、lifecycle snapshot、`--require-provider-input` 付きで
packaging 前に呼び出す。verifier が canonical identity schema、provider digest、時刻、source commit を
拒否した場合は release、provider helper、stage0 fetch、source smoke、release smoke、Lima VMを開始しない。

## Evidence

- RED: fake two-target harness で current `source_commit` だけを持つ不完全 identity を渡すと、実装前は
  official gate が downstream invocationへ進んだ。
- GREEN: 同じ harness が既存 verifier の canonical schema errorを返し、invocation logを不変に保った。
- `bash scripts/ci/test-native-official-release-snapshots.sh` と shell syntax gate が pass。

## Boundary

これは provider identity の早期 schema/digest preflightを既存 verifierへ接続する verified partial sliceである。
provider API/authentication の実取得・意味検証、target artifact の実 runtime、Linux current-source replay、
packaged bytes parity、rollback/Wasm parity は未検証であり、M3-04-N1 / M3-05-N2 / M3-05-N7 / M3-05-N9 は
`[~]` のまま維持する。
