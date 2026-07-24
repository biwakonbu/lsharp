# ADR: selfhost rooting IR guard lane

- Status: Accepted (verified slice)
- Date: 2026-07-25
- Scope: `LEGACY-ROOT-01` / imp-07 B-4 / Rust-oracle Wasm compiler IR

## Context

`selfhost_rooting_parity.rs` には、allocating call を跨ぐ heap 値の保護と lowering 順序を確認する
guard fixture が存在するが、個別の E2E として分散していた。GC runtime lane だけでは、
root slot の挿入位置や後続引数の lowering 前に保持する契約を一括で再実行できない。

## Decision

[`scripts/ci/test-selfhost-rooting-guards.sh`](../../scripts/ci/test-selfhost-rooting-guards.sh) を
selfhost compiler の rooting IR guard lane とする。既存の Rust-oracle E2E を `--exact` で直列実行し、
`--dry-run` で対象 command 集合を固定する。

対象は次の 13 test に限定する。

- string concat / substring: 引数保護と lhs/source の lowering 順序
- ref-new / vector-push: wrapped value、reallocation receiver、後続 value の保護
- map insert / map get: receiver、key、value の保護
- user call: 先行引数の保護と call までの root lifetime
- let / let-chain: heap binding と最終 body の保護

この lane は生成 IR の root push/pop/order を検査する。全 selfhost source の static lint、
alloc 毎の GC stress、Mac Apple Silicon/Linux x86_64 native stage0、Component/HTTP parity は
この ADR の完了条件へ混ぜない。

## Evidence

- RED: contract script を先に追加し、未作成の lane script で実行失敗を確認した。
- Contract: `bash scripts/ci/test-selfhost-rooting-guards-contract.sh` → passed。
- Dry-run: `scripts/ci/test-selfhost-rooting-guards.sh --dry-run` → 13 exact command を出力。
- Exact lane: `scripts/ci/test-selfhost-rooting-guards.sh` → 13 tests passed。
- Script syntax: `bash -n scripts/ci/test-selfhost-rooting-guards.sh scripts/ci/test-selfhost-rooting-guards-contract.sh` → passed。

## Consequences

selfhost compiler の代表的な allocating operation に対する root slot の挿入位置を、軽量な一つの local gate で再実行できる。
一方、これは `LEGACY-ROOT-01` の verified slice であり、一般 root 規律の static lint、GC stress、
Linux native/VM evidence、aggregate 完了は残件である。
