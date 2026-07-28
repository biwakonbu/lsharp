# ADR: v0.2 native validation empty evidence ID

- Status: Accepted (verified partial slice)
- Date: 2026-07-28
- Scope: `selfhost/src/Tools/Validation/Evidence.ls`, `scripts/ci/native-selfhost-dev-source-file-smoke.sh`
- Related: `EC-M2-02`、`docs/adr/decisions-v0.2-evidence-graph.md`

## Context

Evidence の `:evidence` ID は stable graph identity である。Rust canonical source adapter は
`EvidenceId::parse` を先に実行するため、空文字の ID は stable ID wire format error（code `2`）になる。
Selfhost は ID を required-field chain で先に検査していたため、空文字を empty field（code `4`）として
扱う境界が残っていた。

## Decision

- Evidence ID を通常の required-field empty-string chain から外す。
- Evidence ID は `source-wire-valid?` で stable-ID wire shape を検証し、空文字は invalid ID code `2` として
  field/value を保持して拒否する。
- runner、target、provenance、sampling generator などの実行事実は required-field code `4` を維持する。

## Evidence

- RED: native source-file smoke の static contract は fixture variables がない状態で失敗した。
- Rust oracle: `cargo test -p lsharp-types --test validation_source evidence::source_adapter_rejects_empty_evidence_id_as_invalid_id`
  が `StableIdError::InvalidWireFormat { value: "" }` を検証する。
- Selfhost oracle: empty ID の E2E が `['0', '2', 'id', '[]']` を検証する。
- Native contract: source-file smoke が `source validation error:2`、exit `1`、report/manifestなしを要求する。
- Native provenance smoke、docs audit、format/syntax checks を完了条件とする。

## Boundary and follow-up

この ADR は empty evidence ID の wire contract のみを閉じる。current source-commit に一致する packaged
stage0 execution、Mac/Linux artifact/runtime parity、manifest bytes、fallback exclusion は証明しない。
EC-M2-02 および M2/M3 aggregate は `[~]` のまま維持する。
