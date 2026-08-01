# ADR: v0.3 review-lifecycle sequence gap parity

- Status: Accepted (verified partial slice)
- Date: 2026-08-02
- Scope: Rust `ReviewLifecycleRegistry` と native provider identity preflight の同一 review ID に対する sequence continuity
- Related: [`decisions-v0.3-provider-lifecycle-sequence-gap-preflight.md`](decisions-v0.3-provider-lifecycle-sequence-gap-preflight.md)

## Context

先行する native provider boundary は、同じ `review_id` の `proposed` sequence `1` の次に `active` sequence `3` が来る
gapを fail-closed で拒否していた。一方、Rust canonical `ReviewLifecycleRegistry` は duplicate と rollback は拒否するものの、
同じ入力を受理していたため、Rust oracle/bootstrap と native verifier の observable boundary が一致していなかった。

## Decision

Rust reducerにも `LifecycleError::SequenceGap` を追加し、同じ review IDの直前 sequenceから `+1` を超える eventを拒否する。
duplicate、rollback、effective-time rollback、state transitionの診断は混ぜず、sequenceが直前値のちょうど `+1` の eventだけを
既存の state transition 検証へ進める。

## Evidence

Rust test `lifecycle_rejects_duplicate_or_rollback_sequences_and_invalid_transitions` に同じ `proposed(1) → active(3)` fixtureを追加し、
実装前は `SequenceGap` variantの未実装でRED、実装後は `LifecycleError::SequenceGap` でGREENとなることを確認した。native側の
`test-native-release-identity.py` も同じ fixtureを `review lifecycle sequence gap` として拒否するため、Rust error variant と native
fail-closed診断が同じ sequence continuity boundaryを表す。

## Boundary and follow-up

これは Rust/native の sequence gap parity に限る verified partial sliceである。full transition matrix/reducer、live provider API/auth取得・
意味検証、current-source Linux runtime、Mac/Linux両 targetの packaged provenance/rollback bytes parityは未検証のため、
M3-04-N1 / M3-05-N2 / M3-05-N7 / M3-05-N9 は `[~]` のまま残す。

current-source manifest/expected replay lockが現HEADに一致せず、別セッション所有のLima/QEMU/replay processも稼働しているため、
Linux replay・stage regeneration・full buildは実行しない。blockerの再現 commandは次のとおり。

```bash
current_head="$(git rev-parse --verify HEAD)"
find /tmp /Users/biwakonbu/github/tmp -maxdepth 5 -type f -name manifest.json -path '*lsharp*'
find /tmp /Users/biwakonbu/github/tmp -maxdepth 4 \( -name 'lsharp-native-linux-x86-hostgen-vm-*' -o -name '*.lock' \)
```
