# ADR: provider snapshot の role binding

- Status: Accepted (verified partial slice)
- Date: 2026-08-02
- Scope: `EC-M3-05` / `prepare-review-evidence-identity.py` / `verify-native-release-identity.py`
- Related: [`decisions-v0.3-provider-snapshot-nonempty-preflight.md`](decisions-v0.3-provider-snapshot-nonempty-preflight.md)

## Context

explicit provider input は trust-store と review-lifecycle の2つの roleを持つが、同じ pathを両方へ
渡しても producer / verifier は同じ bytesを別々の digestとして受理できた。これは provider adapterの
配線ミスを意味検証へ到達する前に検出できない境界だった。

## Decision

両 provider inputが指定された場合、pathを lexical-normalizeした値が同一なら fail-closed で拒否する。
distinct pathの bytes、provider API/authentication、署名・lifecycle意味検証、既存の regular-file/nonempty/
canonical identity検証はそのまま維持する。これは role bindingだけを検査し、snapshot内容を解釈しない。

## Evidence

- RED: 同じ snapshot pathを trust-store と review-lifecycle に渡す producer / verifier fixtureが成功していた。
- GREEN: 両方が `must be different files` で拒否し、producer outputは生成されない。
- producer 7件、release identity 11件、official snapshot/replay-lock/provider-snapshot、stage0 release-package
  focused harnessと shell syntax/diff/docs gateを通過。

## Boundary

これは explicit provider inputの role bindingに限る verified partial sliceである。live provider/auth取得、
署名意味検証、current-source Linux runtime、両 target packaged/rollback bytes parityは未検証であり、
M3-04-N1 / M3-05-N2 / M3-05-N7 / M3-05-N9は`[~]`のまま維持する。
