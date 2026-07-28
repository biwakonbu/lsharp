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

## Follow-up: unsigned numeric overflow (2026-07-29)

負数に加えて、`u64::MAX + 1` (`18446744073709551616`) を
`span.start` / `span.end`、`sampling.cases` / `sampling.seed` /
`sampling.shrinks[*]` / `sampling.coverage[*]` の全 unsigned field に入力する回帰テストを追加した。
全て graph 構築前の `ValidationInputError::Json` となり、値の丸め、切り詰め、部分 manifest の生成を
行わない。既存の `usize` / `u64` typed serde decode が上限超過を fail-closed にするため production
code の変更は不要だった。

- Test: `parse_manifest_rejects_unsigned_numeric_overflow`
- Evidence: `cargo test -p lsharp-types --test validation_input parse_manifest_rejects_unsigned_numeric_overflow -- --nocapture`

## Follow-up: fractional unsigned numbers (2026-07-29)

JSON の小数値も unsigned integer field へ暗黙変換しない契約を追加した。`0.5` / `1.5` を
`span.start` / `span.end`、`sampling.cases` / `sampling.seed` / `sampling.shrinks[*]` /
`sampling.coverage[*]` の全 field に入力し、全て graph 構築前の `ValidationInputError::Json` として
reject されることを確認した。既存の typed serde decode に委譲し、切り捨てや丸めによる sampling
semantics の変質を許可しない。

- Test: `parse_manifest_rejects_fractional_unsigned_numeric_fields`
- Evidence: `cargo test -p lsharp-types --test validation_input parse_manifest_rejects_fractional_unsigned_numeric_fields -- --nocapture`

## Boundary

これは Rust manifest input decoder の数値型境界に限定した verified slice である。source syntax
adapter の sampling diagnostics、selfhost/native stage0、CLI/MCP の exit/report parity、sampling の
semantic policy、Mac Apple Silicon / Linux x86_64 の artifact/runtime evidence、EC-M2 aggregate の
完了を意味しない。
