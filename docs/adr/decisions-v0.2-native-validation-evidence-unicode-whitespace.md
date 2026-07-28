# ADR: v0.2 evidence required fields の Unicode whitespace boundary

- Status: Accepted (verified partial slice)
- Date: 2026-07-29
- Scope: selfhost source evidence の required execution/provenance fields
- Related: `EC-M2-02`、`EC-M3-01`、`docs/adr/decisions-v0.2-native-validation-evidence-coverage-whitespace.md`

## Context

Rust canonical/source adapter は `str::trim().is_empty()` を使うため、ASCII whitespace だけでなく
Unicode White_Space（NBSP、Ogham、各種 quad、line/paragraph separator、ideographic space など）だけの
required value を拒否する。一方、selfhost の `string-char-at` は UTF-8 byte を返すため、従来の
`source-evidence-whitespace?` は space/tab/LF/CR の 4 byte だけを認識し、Unicode whitespace-only
`runner` を有効な evidence として登録していた。

## Decision

- `Tools.Validation.Whitespace` に UTF-8 byte sequence の幅を返す non-blank 判定を追加する。
- evidence required execution/provenance fields は同じ helper を使い、ASCII 9..13/space と Rust の
  Unicode White_Space 相当の UTF-8 sequence を空値として扱う。
- 元の field value と evidence directive span は source adapter の既存 error boundary で保持し、
  stable code `4` を変更しない。
- node text、review provenance、manifest input、coverage count/cases はこの slice の対象外として
  残す。native stage0 artifact/runtime の current-source parity も別 gate として扱う。

## Evidence

- Rust source adapter の Unicode NBSP `runner` fixture を追加し、
  `InvalidEvidenceRequiredField { field: "runner" }` を確認した。
- selfhost actual Wasm の同一 NBSP fixtureで status `0` / code `4` / field `runner` を確認した。
- selfhost helper の Rust White_Space 主要 10 種（ASCII vertical tab/form feed、U+0085、U+00A0、
  U+1680、U+2000、U+2028、U+202F、U+205F、U+3000）を direct runtime で全て non-blank `0` と確認した。
- native source-file smoke に Unicode `runner` fixtureを追加し、stable `source validation error:4`、
  exit `1`、report/manifestなしの fail-closed contract と provenance wrapper を通過した。
- focused gates: `cargo test -p lsharp-types --test validation_source source_adapter_rejects_unicode_whitespace_only_required_execution_runner -- --nocapture`、
  selfhost Unicode validation tests、`bash scripts/ci/test-native-linux-x86-native-stage0-source-file-smoke.sh`、
  `bash -n scripts/ci/native-selfhost-dev-source-file-smoke.sh`、`git diff --check`。

## Boundary and follow-up

これは evidence required fields の Unicode non-blank policy に限定した verified partial sliceである。
App.Cli/EmbeddedCli の full bundle runtime、current-source packaged stage0、Mac/Linux artifact matrix、
node/review/manifest の Unicode parity、EC-M2-02/EC-M3 aggregate は未完了であり、TODO の `[~]` を維持する。
