# ADR: v0.3 provider review-lifecycle review ID wire format parity

- Status: Accepted (verified partial slice)
- Date: 2026-08-02
- Scope: native provider lifecycle snapshot と Rust canonical `ReviewId::parse` の review ID wire format
- Related: [`decisions-v0.3-provider-lifecycle-review-id-required.md`](decisions-v0.3-provider-lifecycle-review-id-required.md)、
  [`decisions-v0.3-review-lifecycle-wire-ordering.md`](decisions-v0.3-review-lifecycle-wire-ordering.md)

## Context

先行 slice は native lifecycle record の `review_id` 欠落・空値を拒否したが、非空でも `review:checkout` のように key segmentが
欠けた値を受理していた。Rust canonical model の `ReviewId::parse` は `review:namespace/key` を要求し、namespace/keyを
ASCIIの英数字、`_`、`-`、`.` の segmentに限定しているため、native provider routeとRust oracleの形式境界が不一致だった。

## Decision

native provider lifecycle preflightも `^review:[A-Za-z0-9_.-]+/[A-Za-z0-9_.-]+$` の wire formatを要求する。`review:checkout`、
wrong-kind、空 segment、禁止文字は `review lifecycle review_id must use review:<namespace>/<key>` として fail-closed に拒否する。
required-field、sequence、state transition、attestation/MCPの別 schema boundaryはこの sliceで変更しない。

## Evidence

native testに同じ malformed `review:checkout` fixtureを追加し、実装前は exit 0 の RED、実装後は wire-format診断の GREENを確認した。
Rust `ReviewLifecycleEvent::new` も同じ値を `LifecycleError::InvalidReviewId` として拒否する differential testを追加した。
focused Rust lifecycle 6件、review wire 8件、native identity 27件と関連 offline harnessを通過した。

## Boundary and follow-up

これは native provider lifecycle routeとRust canonical review IDの wire-format parityに限る verified partial sliceである。selfhost/MCPの
全入力 route、live provider API/auth取得・意味検証、full transition matrix/reducer、current-source Linux runtime、Mac/Linux両 targetの
packaged provenance/rollback bytes parityは未検証のため、M3-04-N1 / M3-05-N2 / M3-05-N7 / M3-05-N9 は `[~]` のまま残す。

current-source manifest/expected replay lockが現HEADに一致せず、別セッション所有のLima/QEMU/replaydも稼働しているため、Linux replay・
stage regeneration・full buildは実行しない。blockerの再現 commandは次のとおり。

```bash
current_head="$(git rev-parse --verify HEAD)"
find /tmp /Users/biwakonbu/github/tmp -maxdepth 5 -type f -name manifest.json -path '*lsharp*'
find /tmp /Users/biwakonbu/github/tmp -maxdepth 4 \( -name 'lsharp-native-linux-x86-hostgen-vm-*' -o -name '*.lock' \)
```
