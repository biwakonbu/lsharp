# ADR: 型推論限界値 regression lane

- Status: Accepted (verified slice)
- Date: 2026-07-25
- Scope: `LEGACY-TEST-01` / imp-07 D-3 / Rust-oracle `lsharp-types`

## Context

occurs-check、深い型注釈、巨大レコードの限界値 fixture は
`crates/lsharp-types/tests/infer_limits.rs` に存在するが、個別に実行する必要があり、
型推論の failure boundary と panic-safety を同じ再現可能な local gate として監査しにくかった。

## Decision

[`scripts/ci/test-type-inference-limits.sh`](../../scripts/ci/test-type-inference-limits.sh) を
型推論限界値の local lane とする。各 integration test を `--exact` で直列実行し、
`--dry-run` で command 集合を確認できるようにする。

対象は次の 3 test に限定する。

- `self_application_reports_infinite_type`: `InfiniteType` / `LS1003` の occurs-check 境界
- `wide_record_type_annotations_do_not_panic`: 128 / 256 fields の型注釈
- `deeply_nested_type_annotations_do_not_panic`: `Box` 32 / 64 / 128 段の型注釈

`cargo bench --bench infer_limits` の性能比較、式全体の full-program parity、native stage0、
Linux x86_64、runtime の recursion/GC 上限はこの lane の完了条件へ混ぜない。

## Evidence

- RED: contract script を先に追加し、未作成の lane script で実行失敗を確認した。
- Contract: `bash scripts/ci/test-type-inference-limits-contract.sh` → passed。
- Dry-run: `scripts/ci/test-type-inference-limits.sh --dry-run` → 3 exact command を出力。
- Exact lane: `scripts/ci/test-type-inference-limits.sh` → 3 tests passed。
- Script syntax: `bash -n scripts/ci/test-type-inference-limits.sh scripts/ci/test-type-inference-limits-contract.sh` → passed。
- Docs: `scripts/audit_docs.sh` → 0 errors / 0 warnings。

## Consequences

occurs-check の安定診断と深い型構造の panic-safety を一つの軽量 gate で再実行できる。
一方、これは `LEGACY-TEST-01` の verified slice に留まり、性能回帰の閾値、生成式全体の型推論、
GC/runtime 限界、supported 2 targets の native evidence、full aggregate 完了は残件である。
