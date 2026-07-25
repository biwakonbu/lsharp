# ADR: selfhost evidence registry consumer の wire boundary

- Status: Accepted (verified partial)
- Date: 2026-07-25
- Scope: EC-M2-02 selfhost evidence registry input boundary
- Related: `decisions-v0.2-source-evidence-record.md`, `decisions-v0.2-source-evidence-boundary.md`

## Context

Rust の canonical validation model は、source の `:evidence` record を全 node の登録後に
Evidence registry へ追加し、`supports` / `contradicts` edge をその registry へ解決する。
selfhost 側では parser がまだ `:evidence` metadata を生成していないため、parser と registry を
同時に変更すると、source order、required field、duplicate、edge の failure boundary が一つの
差分に埋もれる。まず parser が将来渡せる値の形と fail-closed consumer を独立して固定する。

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

この wire shape は parser の source order や named-field syntax を決めるものではない。parser は
後続 task で同じ payload と directive span を生成し、registry consumer に渡す。

## Evidence

- `selfhost/src/Tools/Validation/Evidence.ls`
- `crates/lsharp-wasm/tests/e2e/selfhost_evidence_registry.rs`
- `CARGO_PROFILE_DEV_DEBUG=0 cargo test -p lsharp-wasm --test e2e selfhost_evidence_registry -- --nocapture`
  （4 passed）

## Consequences and residual work

- selfhost parser はまだ `:evidence` form を生成しないため、source metadata の parse parity は
  未完了である。
- `validate --source` / manifest serializer / EmbeddedCli・MCP、Contract registry、generator policy、
  Mac Apple Silicon / Linux x86_64 native stage0 の実行証跡はこの slice に含めない。
- これらが揃うまで EC-M2-02 は TODO の `[~]` を維持し、この ADR を completed task の移動先には
  扱わない。
