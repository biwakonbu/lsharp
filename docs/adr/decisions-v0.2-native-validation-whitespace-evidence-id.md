# ADR: v0.2 native validation whitespace-only evidence ID

- Status: Accepted (verified partial slice)
- Date: 2026-07-28
- Scope: `selfhost/src/Tools/Validation/Evidence.ls`, `scripts/ci/native-selfhost-dev-source-file-smoke.sh`
- Related: `EC-M2-02`、`docs/adr/decisions-v0.2-evidence-graph.md`

## Context

Evidence の `:evidence` ID は stable graph identity であり、通常の provenance string ではない。
Rust canonical source adapter は `EvidenceId::parse` を先に実行するため、空白だけの ID は stable ID
wire format error（code `2`）になる。Selfhost は以前、ID を required-field nonblank chain で先に検査し、
空白だけの値を empty field（code `4`）として扱っていた。

## Decision

- Evidence ID を通常の required-field empty-string chain から外す。
- Evidence ID は `source-wire-valid?` で stable-ID wire shape を検証し、空白だけを含む malformed value は
  invalid ID code `2` として field/value を保持して拒否する。
- runner、target、provenance、sampling generator などの実行事実は従来どおり required-field code `4` を
  使用する。

## Evidence

- RED: Rust oracle は `SourceGraphError::EdgeId(StableIdError::InvalidWireFormat { value: "  " })` を要求し、
  selfhost E2E は修正前に `['0', '4', 'id', '[]']` を返した。
- Rust oracle: `cargo test -p lsharp-types --test validation_source evidence::source_adapter_rejects_whitespace_only_evidence_id_as_invalid_id`
  が stable-ID boundary を検証する。
- Selfhost oracle: whitespace-only ID の E2E が `['0', '2', 'id', '[  ]']` を検証する。
- Native contract: source-file smoke が `source validation error:2`、exit `1`、report/manifestなしを要求する。
- Full evidence validation、native source-file smoke、docs/format checks を完了条件とする。

## Boundary and follow-up

この ADR は whitespace-only evidence ID の wire contract のみを閉じる。current source-commit に一致する
packaged stage0 execution、Mac/Linux artifact/runtime parity、manifest bytes、fallback exclusion は証明しない。
EC-M2-02 および M2/M3 aggregate は `[~]` のまま維持する。
