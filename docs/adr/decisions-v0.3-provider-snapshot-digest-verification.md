# ADR: v0.3 offline provider snapshot digest verification

- Status: Accepted (verified partial slice)
- Date: 2026-07-31
- Scope: release identity verifier の明示 provider snapshot 再計算境界
- Related: [`decisions-v0.3-release-identity-gate.md`](decisions-v0.3-release-identity-gate.md)、
  [`decisions-v0.3-provider-input-identity-preparer.md`](decisions-v0.3-provider-input-identity-preparer.md)、
  [`v0.3-milestone-01.md`](../development/planning/v0.3-milestone-01.md)

## Context

`prepare-review-evidence-identity.py` は trust store と lifecycle snapshot の raw bytes を digest
へ変換するが、release verifier は受け取った digest の形状だけを検証していた。provider adapter
から渡された snapshot が release 時点の bytes と異なっていても、artifact と source commit 以外の
identity 差分を検出できない境界だった。

## Decision

- `verify-native-release-identity.py` は `--trust-store` と `--review-lifecycle` を任意の明示入力として
 受け取る。
- 片方だけの指定は fail-closed に拒否し、両方が指定された場合は raw bytes の SHA-256 を再計算して
  `trust_store_digest` / `lifecycle_digest` と照合する。
- snapshot が指定されない既存呼び出しは、provider adapter が外部で digest を確定する offline boundary
  として従来どおり動作する。verifier は network、environment、current checkout、implicit trust root
  を参照しない。
- `--require-provider-input` は従来どおり digest の `null` を exit `2` の `unverified` とし、snapshot
  の再計算成功だけで review verification state を `verified` へ昇格させない。

## Evidence

RED は raw snapshot オプションを受け付けない verifier に対する 7-case release identity test の失敗。
GREEN は trust store/lifecycle の一致、各 digest mismatch、片側指定、preparer output の再検証を含む
Python focused suite である。既存の Rust canonical identity と docs audit は別境界として維持する。

## Boundary

この変更は provider API の取得・認証、署名検証、selfhost/native MCP parity、current-source の Mac/Linux
runtime を実装しない。caller/provider adapter が用意した snapshot bytes を release verifier が再計算
できる verified partial slice であり、`EC-M3-05` は `[~]` のまま残す。
