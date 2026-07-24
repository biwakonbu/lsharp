# ADR: WASI GC rooting regression lane

- Status: Accepted (verified slice)
- Date: 2026-07-25
- Scope: `LEGACY-TEST-01` / imp-07 B-4 + D-3 / Rust-oracle WASI runtime

## Context

GC rooting の regression fixture は、direct string、ref、map、closure、heap parameter、let local、opaque nested call、pattern field、legacy sentinel に分散していた。個別 E2E の存在だけでは、allocation を跨ぐ root preservation を同じ再現可能な gate として監査しにくい。

## Decision

`scripts/ci/test-gc-rooting.sh` を9件の exact E2E を直列実行する local lane とする。lane は次を確認する。

- direct/rooted ref/rooted map/rooted closure の transitive retention
- non-self-recursive heap parameter と let local の allocation crossing
- opaque nested call と pattern-bound heap field の preservation
- legacy zero-root sentinel が rooted object を保持しないこと

この lane は既存の actual WASI forced-collection fixture を束ねるだけで、全 allocation の GC stress mode、selfhost source の static lint、Component/HTTP、native stage0 の完了を主張しない。

## Evidence

- RED: contract script を先に追加し、未作成 lane script の missing-file failure を確認。
- Contract: `bash scripts/ci/test-gc-rooting-contract.sh` → passed。
- Exact lane: `scripts/ci/test-gc-rooting.sh` → 9 tests passed。
- Fixtures: `crates/lsharp-wasm/tests/e2e/runtime_allocator_closures.rs`

## Consequences

- allocation crossing の root regression を一つの deterministic local gate で再実行できる。
- 失敗時は fixture category から root leak/retention の原因層を切り分けやすい。
- full GC stress、static rooting lint、Linux x86_64/Mac native parity、公開 command parity は残件として維持する。
