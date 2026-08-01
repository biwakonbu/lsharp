# ADR: native official gate の provider identity source commit binding

- Status: Accepted (verified partial slice)
- Date: 2026-08-02
- Scope: `EC-M3-05-N2` / `EC-M3-05-N9` / `scripts/ci/native-official-release-local.sh`
- Related: [`decisions-v0.3-native-official-provider-identity-preflight.md`](decisions-v0.3-native-official-provider-identity-preflight.md)

## Context

provider snapshot が指定された native official gate は、4つの
`review-evidence-identity.json` の存在を packaging 前に検査していた。しかし identity の
`source_commit` が current checkout と異なる場合、provider identity は別 source の証跡のまま
release/package boundaryへ到達し得た。

## Decision

provider snapshot が指定された場合、Mac/Linux の App.Cli と stage0 各 identity を JSON object として
読み、`source_commit` が current checkout の lowercase 40桁 HEAD と一致することを packaging 前に検査する。
不正 JSON、非 object、欠落または不一致は明示的な入力エラーとして拒否し、release、provider helper、
stage0 fetch、source smoke、release smoke、Lima VMを開始しない。

## Evidence

- RED: fake two-target harness で App.Cli identity の `source_commit` を別 commit にすると、実装前は
  provider identity が受理され、下流 invocationへ進んだ。
- GREEN: 同じ harness が `review evidence identity source_commit mismatch` を返し、invocation logを
  不変に保った。
- `bash scripts/ci/test-native-official-release-snapshots.sh` と shell syntax gate が pass。

## Boundary

これは provider identity と current source の早期 bindingだけを閉じる verified partial sliceである。
provider API/authentication の実取得・意味検証、target artifact の実 runtime、Linux current-source replay、
packaged bytes parity、rollback/Wasm parity は未検証であり、M3-04-N1 / M3-05-N2 / M3-05-N7 / M3-05-N9 は
`[~]` のまま維持する。
