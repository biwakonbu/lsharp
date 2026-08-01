# ADR: native MCP provider snapshot semantic boundary

- Status: Accepted (verified partial slice)
- Date: 2026-08-02
- Scope: `scripts/native-selfhost-mcp.py` の `lsharp_validate` provider snapshot adapter

## Context

native MCP は明示された trust-store / review-lifecycle snapshot を offline で raw bytes として
digest化するだけであり、署名検証や lifecycle の意味検証を実行しない。従来は provider snapshot
を渡した native report が `verified`、`stale`、または `revoked` を返しても、その semantic stateを
そのまま MCP 応答へ通せる境界だった。

## Decision

provider snapshot path が指定された場合、native MCP は `review_verifications` の `unverified` だけを
受理する。`verified` / `stale` / `revoked` のような semantic state は、native verifier が存在しない
ため `provider semantic verification is unavailable; review_verifications must remain unverified` で
fail-closed にする。snapshot bytesの読み込み、digest照合、既存の schema / role binding、provider API
の実取得はこの境界では変更しない。

## Evidence

- RED: provider snapshotを渡した fake native report の `verified` state が既存 shim で受理された。
- GREEN: 同じ fixtureを使い、provider snapshot使用時の semantic stateをMCP応答前に拒否する test と
  `scripts/ci/test-native-selfhost-mcp.py` の focused suite が通過した。

## Boundary

これは native MCP が未実装の provider semantic verification を成功扱いしない verified partial slice
である。live provider/auth acquisition、署名・lifecycleの実 semantic verifier、current-source Linux
runtime、両 target packaged/rollback parity は未検証であり、M3-04-N1 / M3-05-N2 / M3-05-N7 / M3-05-N9 は
`[~]` のまま維持する。
