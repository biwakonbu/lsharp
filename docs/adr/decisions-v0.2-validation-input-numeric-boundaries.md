# ADR: v0.2 validation manifest の unsigned 数値境界

- Status: Accepted (verified partial slice)
- Date: 2026-07-26
- Scope: `crates/lsharp-types/src/validation_input.rs` の JSON manifest decoder
- Related: `EC-M2-02`, `EC-M2-03`, `docs/adr/decisions-v0.2-validation-input-parser.md`

## Context

version 1 manifest の span と sampling は JSON 上の非負整数である。既存の input parser は
`usize` / `u64` として値を decode し、span の逆順を別の graph-level error で拒否していたが、
負数が全ての unsigned field で JSON decode 境界に fail-closed することを回帰テストで列挙して
いなかった。`cases`、`seed`、`shrinks`、`coverage` に負数が紛れ込むと、source adapter の
sampling 契約と manifest input 契約の境界がずれる。

## Decision

- manifest の `span.start` / `span.end`、`sampling.cases` / `sampling.seed`、
  `sampling.shrinks[*]`、`sampling.coverage[*]` は unsigned JSON number として decode する。
- これらの field に負数が現れた場合、graph を部分構築せず `ValidationInputError::Json` として
  reject する。負数を 0 へ丸めたり、符号を捨てたり、sampling を省略扱いにしない。
- この境界は serde の typed decode に委譲し、既存の `InvalidSpan`（非負値で start > end）と
  `SamplingPlan` の semantic validation を混同しない。

## Evidence

- `parse_manifest_rejects_negative_unsigned_numeric_fields` が上記6 fieldを同じ complete manifest
  fixtureで変異させ、全て `ValidationInputError::Json` になることを検証する。
- RED のテスト追加後、production code を変更せず focused test が GREEN になった。これは既存の
  `usize` / `u64` serde decode が負数を受理しないことを確認する契約固定である。
- 実行: `cargo test -p lsharp-types --test validation_input parse_manifest_rejects_negative_unsigned_numeric_fields -- --nocapture`

## Boundary

これは Rust manifest input decoder の数値型境界に限定した verified slice である。source syntax
adapter の sampling diagnostics、selfhost/native stage0、CLI/MCP の exit/report parity、sampling の
semantic policy、Mac Apple Silicon / Linux x86_64 の artifact/runtime evidence、EC-M2 aggregate の
完了を意味しない。
