# ADR: selfhost evidence registry consumer の wire boundary

- Status: Accepted (verified partial)
- Date: 2026-07-25
- Scope: EC-M2-02 selfhost evidence registry input boundary
- Related: `decisions-v0.2-source-evidence-record.md`, `decisions-v0.2-source-evidence-boundary.md`

## Context

Rust の canonical validation model は、source の `:evidence` record を全 node の登録後に
Evidence registry へ追加し、`supports` / `contradicts` edge をその registry へ解決する。
selfhost 側では parser と registry consumer の初期 source-form 経路が未接続だったため、source order、
required field、sampling、duplicate、edge の failure boundaryを一つの差分に埋めないよう、まず
parserの17-field payload生成と fail-closed consumerを同じ runtime contractで固定する。

## Decision

`Tools.Validation.Evidence` は、次の tagged vector を selfhost の入力契約とする。

```text
form    = [15, payload, span-start, span-end]
payload = [id, subject, method, outcome, runner, target, source-commit,
           artifact-digest, cases, seed, generator, shrinks, coverage,
           producer, tool-version, timestamp, independence]
coverage = [[bucket, count], ...]
```

- required string と subject wire kind（Intent / Claim / Contract）を検査し、Intent/Claim は
  先に構築された node registry に存在することを要求する。
- method、outcome、independence は canonical enum の値だけを受理する。cases、shrinks、coverage
  count は非負値、coverage bucket は record 内で一意であることを要求する。
- registry への登録は evidence ID を一意にする。同じ ID は current span と first span を含む
  error record として返し、登録を行わない。
- `supports` / `contradicts` は evidence ID と Claim ID を検証し、registry に登録済みの evidence
  と存在する Claim にだけ typed edge を追加する。未登録 evidence は registry-required error と
  して返し、外部 Contract registry は別境界に残す。
- error record は `[code, field, value, start, end, related-start, related-end]` とし、既存の
  selfhost source adapter の result shape `[status, value-or-error]` を再利用する。

parser の初期 source slice は named fieldsをsource orderから同じpayloadへ投影し、directive spanと
optional sampling vectorを保持してregistry consumerへ渡す。source edge/manifestのsyntaxや全診断
parityはこの決定の範囲外である。

## Evidence

- `selfhost/src/Tools/Validation/Evidence.ls`
- `selfhost/src/Syntax/Parser.ls`
- `crates/lsharp-wasm/tests/e2e/selfhost_evidence_registry.rs`
- `cargo test -p lsharp-wasm --test e2e selfhost_evidence_registry -- --nocapture`
  （10 passed）、`selfhost_intent_source_adapter`（既存 8 passed）。負の shrink 値、重複
  coverage bucket、duplicate evidence ID は同じ selfhost registry error contract として確認した。

## Consequences and residual work

- selfhost parserのrequired fieldsとsamplingを含む初期 `:evidence` form、および
  `source-evidence-graph-from-program` の registry後 `supports` / `contradicts` 投影は検証済みで、
  負の shrink 値も `invalid-sampling` code `11` / field `shrinks` として fail-closed に固定したが、
  malformed/unknown fieldの診断 parity、既存 validate graph/CLI との接続、nested source graphの全要件は
  未完了である。
- `validate --source` / manifest serializer / EmbeddedCli・MCP、Contract registry、generator policy、
  Mac Apple Silicon / Linux x86_64 native stage0 の実行証跡はこの slice に含めない。
- これらが揃うまで EC-M2-02 は TODO の `[~]` を維持し、この ADR を completed task の移動先には
  扱わない。
