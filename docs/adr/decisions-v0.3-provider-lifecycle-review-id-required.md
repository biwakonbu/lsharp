# ADR: v0.3 provider review-lifecycle review ID required

- Status: Accepted (verified partial slice)
- Date: 2026-08-02
- Scope: native provider lifecycle snapshot の `review_id` required input boundary
- Related: [`decisions-v0.3-review-lifecycle-sequence-gap-parity.md`](decisions-v0.3-review-lifecycle-sequence-gap-parity.md)

## Context

native provider identity preflight は `sequence` と state の意味検証を行っていたが、record の `review_id` が欠落していても
その recordを lifecycle semantics の対象外として exit 0 で受理していた。Rust canonical `ReviewLifecycleEvent::new` は
`review_id` を必須の typed input として扱うため、provider snapshot の required-field boundary が一致していなかった。

## Decision

native lifecycle snapshot の各 recordは、非空の string `review_id` を必須とする。欠落、null、空文字、別型は
`review lifecycle review_id is required` として意味検証前に fail-closed で拒否する。sequence、state、transition、stable ID wire format の
既存/別 boundaryはこの sliceで変更しない。

## Evidence

`test-native-release-identity.py` に `review_id` 欠落・`sequence: 1`・`state: proposed` の fixtureを追加した。実装前は verifierが
exit 0となる RED、実装後は required診断で拒否する GREENを確認した。Rust `ReviewLifecycleEvent::new` の空 `review_id` 拒否も
focused lifecycle testで確認し、typed canonical inputとnative required boundaryを対応付けた。

## Boundary and follow-up

これは provider lifecycle record の required `review_id` に限る verified partial sliceであり、full transition matrix/reducer、selfhost/MCPの
全入力 route、live provider API/auth取得・意味検証、current-source Linux runtime、Mac/Linux両 targetの
packaged provenance/rollback bytes parityは未検証である。M3-04-N1 / M3-05-N2 / M3-05-N7 / M3-05-N9 は `[~]` のまま残す。

current-source manifest/expected replay lockが現HEADに一致せず、別セッション所有のLima/QEMU/replaydも稼働しているため、Linux replay・
stage regeneration・full buildは実行しない。blockerの再現 commandは次のとおり。

```bash
current_head="$(git rev-parse --verify HEAD)"
find /tmp /Users/biwakonbu/github/tmp -maxdepth 5 -type f -name manifest.json -path '*lsharp*'
find /tmp /Users/biwakonbu/github/tmp -maxdepth 4 \( -name 'lsharp-native-linux-x86-hostgen-vm-*' -o -name '*.lock' \)
```
