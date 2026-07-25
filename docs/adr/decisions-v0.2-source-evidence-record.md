# ADR: source `:evidence` record の required-field registry

- Status: Accepted (partial)
- Date: 2026-07-25
- Scope: EC-M2-02 / EC-M2-03
- Supersedes: `decisions-v0.2-source-evidence-boundary.md` の未接続 registry 境界を、登録済み record に限って更新する

## Context

`supports` / `contradicts` は evidence ID の存在を必要とするが、source から evidence record を
登録する入力契約がなければ、edge を安全に接続できない。canonical `Evidence` は runner/target、
source commit、artifact digest、sampling、provenance、independence を required field として持つため、
source でも値を補完せず明示的に受け取る必要がある。

## Decision

宣言 metadata に次の named-field form を追加する。

```lisp
:evidence "evidence:checkout/cancel-observation"
  :subject "claim:checkout/cancel-rejects-shipped"
  :method "case" :outcome "pass"
  :runner "cargo-test" :target "aarch64-apple-darwin"
  :source-commit "0123456789abcdef" :artifact-digest "sha256:abc123"
  :cases 1 :seed 42 :generator "checkout-cancel-fixture"
  :shrinks [8 3 1] :coverage [("negative" 2) ("positive" 1)]
  :producer "lsharp-test" :tool-version "0.2.0"
  :timestamp "2026-07-25T00:00:00Z" :independence "same-author"
```

- parser は source order と directive span を保ち、raw string enum を暗黙に補正しない。
- source adapter は全 node を登録した後に evidence record を登録し、その後で edge を解決する。
  これにより declaration order に依存しない。
- subject は `IntentId` / `ClaimId` / `ContractId` の wire prefix を検証し、Intent/Claim subject は
  source node registry に存在することを要求する。Contract registry は別境界として残す。
- `supports` / `contradicts` は登録済み `EvidenceId` にだけ接続する。record がない場合は
  `EvidenceRegistryRequired`、enum/value/required field が不正な場合は入力エラーとして返す。
- 同じ `EvidenceId` を複数の source record が宣言した場合は graph 登録前に検出し、最初の record
  span と重複 record span を含む source-level diagnostic として fail-closed にする。
- optional `shrinks` / `coverage` は source でも named field として明示し、非負値・重複 bucket を
  fail-closed に検査した上で canonical `SamplingPlan` と manifest へ投影する。selfhost/native parity
  と generator policy の実行証跡は後続 task とする。

## Consequences

- evidence edge が未登録 record を黙って参照したり、空の record を自動生成したりしない。
- `validate --source` は required-field evidence が揃った source では graph/report へ進み、未登録 edge では
  report status と混同しない入力エラーを返す。
- source と JSON manifest は同じ canonical `Evidence` / `Edge` model を共有し、optional sampling
  fields の source→canonical→manifest projection まで閉じる。ただし selfhost/native parity はまだ
  閉じていない。

## Evidence

- `crates/lsharp-syntax/tests/intent_edges.rs`
- `crates/lsharp-types/tests/validation_source.rs`
- `source_adapter_reports_duplicate_evidence_with_both_source_spans`
- `crates/lsharp-driver/tests/validate_cli.rs`
- `cargo test -p lsharp-syntax`
- `cargo test -p lsharp-types`
- `cargo test -p lsharp-driver --test validate_cli validate_source`
- `cargo clippy -p lsharp-syntax -p lsharp-types --lib -- -D warnings`

## Follow-up: optional sampling projection

`:shrinks [8 3 1]` と `:coverage [("negative" 2) ("positive" 1)]` を optional field として追加した。
省略時は空の plan を使い、値を補完しない。coverage bucket は同一 record 内で重複できない。
parser、source adapter、manifest value の同値性は次で固定する。

- `cargo test -p lsharp-syntax --test intent_edges evidence_record_metadata`
- `cargo test -p lsharp-types --test validation_source source_adapter_projects_optional_sampling_fields`

この slice は Rust source adapter と既存 manifest serializer の境界に限る。selfhost/native parser parity、
generator/shrink policy の実行、supported 2 target の artifact/runtime evidence は TODO の `[~]` として残す。
