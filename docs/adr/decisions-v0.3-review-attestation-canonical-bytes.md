# ADR: v0.3 review attestation の canonical bytes

- Status: Accepted (verified partial slice)
- Date: 2026-07-29
- Scope: `lsharp-types` の review attestation input model と署名対象 bytes
- Related: `EC-M3-01`、[`v0.3-review-provenance-lifecycle.md`](../development/planning/v0.3-review-provenance-lifecycle.md)、
  [`decisions-v0.2-review-provenance-redaction.md`](decisions-v0.2-review-provenance-redaction.md)

## Context

M2 の `ReviewRecord` は `ReviewId`、opaque `provenance_digest`、visibility を保持するが、
review がどの graph/source に対して発行されたかを署名対象として再現する契約はなかった。
provider ごとの JSON をそのまま署名すると key order や target ごとの serializer によって
Rust/selfhost/native の結果が変わり、署名 material を manifest に混ぜると既存の privacy
boundary を壊す。

## Decision

- `ReviewAttestation` を M2 の `ReviewRecord` と別の optional model として追加する。
- 必須 identity は `review_id`、`subject_digest`、`source_commit`、`provenance_digest`、
  `provider`、`key_id`、`issued_at`、`sequence` とする。
- 初期 allowlist は `ed25519` のみとし、unknown algorithm は implicit fallback せず拒否する。
- 署名対象は `lsharp.review-attestation.v1\0` の domain separator と、設計文書の field order に
  従う UTF-8 byte length-prefixed fields とする。`sequence` は unsigned decimal、optional
  `expires_at` は zero-length field とする。
- `signature` は canonical bytes に含めない。provider/key rotation で署名表現だけが変わっても、
  review の対象 identity は変わらない。
- `ReviewId` は既存の typed stable ID parser で検査し、空 field、空署名、unknown algorithm、
  invalid review ID は constructor/setter で fail-closed に拒否する。
- `ReviewVerificationState` (`verified` / `unverified` / `stale` / `revoked` / `invalid`) は
  状態の vocabulary だけを先に固定する。暗号学的署名検証、trusted key 解決、lifecycle reducer、
  CLI/MCP/source/native 接続は後続の RED として残す。

## Evidence

- RED: `crates/lsharp-types/tests/review_attestation.rs` を先に追加し、未公開の
  `lsharp_types::intent::review_attestation` import が解決できないことを確認した。
- GREEN: `cargo test -p lsharp-types --test review_attestation`（4 passed）。
- Regression: `cargo test -p lsharp-types --test intent_ast`（4 passed）、
  `intent_node_wire`（3 passed）、`review_provenance`（9 passed）。
- Formatting/contract: 新規 Rust files の `rustfmt --check` と `git diff --check` を通過した。
  `intent.rs` 全体の rustfmt は既存の長い error attribute の差分を含むため実行結果へ拡大解釈しない。
- Documentation: `bash scripts/audit_docs.sh`（0 errors）を別の docs-only commit で通過済み。

## Boundary

これは canonical input model と署名対象 bytes の verified partial slice である。
signature の decode/verify、trusted key distribution、append-only lifecycle、manifest schema、
CLI/MCP、selfhost/native producer、Mac Apple Silicon/Linux x86_64 runtime evidence
は未完了であり、`EC-M3-01`〜`EC-M3-05` の後続タスクとして設計文書に残す。M2 の opaque registry、
privacy field、既存 manifest bytes はこの sliceだけでは変更しない。

attestation の strict timestamp と明示 clock は
[`decisions-v0.3-review-attestation-expiry-clock.md`](decisions-v0.3-review-attestation-expiry-clock.md)
で別の verified partial slice として追加済みである。
