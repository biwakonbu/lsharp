# ADR: SCC incremental compile の clean linked-IR fast path

- Status: Accepted (verified partial slice)
- Date: 2026-07-24
- Scope: `lsharp-ir::compile_multi_file_incremental_scc`
- Related: `decisions-legacy-module-incremental-scc.md`

## Context

SCC-aware incremental compile は correctness bridge として全 SCC を一括推論していたが、cycle が 1 つでも
ある graph では warm rebuild でも singleton SCC を含む全 module を再推論していた。canonical `Cli.ls` は
38 module / 36 SCC の graph で、`Backend.Wasm.Compiler` の merged infer + revalidation だけで約 25 秒、
`Syntax.Parser` でも約 10 秒かかり、clean-cache probe が 90 秒を超えていた。

## Decision

- SCC compile 入口で現在の全 module fingerprint と module order を確認する。
- fingerprint が全て cache hit で linked module order も一致する場合は、cached linked IR を即時返す。
- dirty module、linked order 不一致、linked IR 未生成の場合は従来どおり SCC 一括推論と modular lowering を行う。
- SCC 内の segment reuse と dirty SCC の局所再推論は後続の別 slice とする。

## Evidence

- RED: `test_compile_multi_file_incremental_infers_mutual_recursive_scc` に SCC inference tracker を追加し、
  warm rebuild で 2 group の再推論が発生することを確認した。
- GREEN: 同テストは warm rebuild の SCC inference count 0、A の source 変更後は count > 0、両方の linked IR
  parity を確認する。
- lsharp-ir regression 244 tests（既知の canonical Formatter probe 1 件を skip）、clippy、rustfmt、diff check を通過。

## Residual risk

初回 full inference の性能、dirty SCC の局所再推論、segment reuse、source override clean-hit、Formatter
canonical runtime parity、Mac/Linux native stage0 evidence は未完了である。`LEGACY-MODULE-01` aggregate 完了
とは扱わない。
