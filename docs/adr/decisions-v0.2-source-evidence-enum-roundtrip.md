# ADR: v0.2 source evidence enum の graph/manifest round-trip

- Status: Accepted (verified partial slice)
- Date: 2026-07-26
- Scope: `crates/lsharp-types/src/validation_source.rs` and its source/manifest adapters
- Related: `EC-M2-02`, `docs/adr/decisions-v0.2-evidence-enum-roundtrip.md`

## Context

Source metadata の `:evidence` は method 8 種、outcome 5 種、independence 3 種を文字列で受け、
canonical `Evidence` へ投影する。manifest wire 側の全 variant は別の contract test で固定したが、
source adapter の parser mapping は代表値の fixture しかなく、source → graph → manifest → graph
の境界で全 variant が lossless であることを確認できていなかった。

## Decision

- source metadata の全 Evidence enum wire value を canonical enum へ fail-closed に投影する。
- source adapter の graph は version 1 manifest serializer と input parser を通過しても、enum value
  を含め元の graph と完全一致することを契約とする。
- source/native stage0 や実行 generator policy の証跡はこの Rust adapter contract の範囲外として、
  M2 aggregate の完了条件へ拡大解釈しない。

## Evidence

- Contract test: `source_adapter_preserves_every_evidence_enum_variant` は method 8 種、outcome 5 種、
  independence 3 種を source `:evidence` forms から構築し、typed enum の順序、manifest round-trip、
  graph equality を検証する。
- RED/GREEN: source fixture を先に追加し、現行 adapter で focused test が pass することを確認した。
  production semantics の変更はない。

## Boundary

これは Rust source metadata adapter の enum coverage に限定した verified slice である。
selfhost parser/native stage0、CLI write boundary、EmbeddedCli/MCP、Mac/Linux runtime、EC-M2 aggregate の
完了を意味しない。
