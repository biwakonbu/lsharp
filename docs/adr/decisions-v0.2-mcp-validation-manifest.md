# ADR: v0.2 MCP `lsharp_validate` manifest input

- Status: Accepted (verified partial slice)
- Date: 2026-07-25
- Scope: `crates/lsharp-driver/src/mcp_server.rs`
- Related: `EC-M2-03`, `decisions-v0.2-mcp-validation-tool.md`,
  `decisions-v0.2-validation-input-parser.md`

## Context

Rust MCP の `lsharp_validate` は source/file から intent graph report を返せるようになったが、
CLI と同じ version 1 JSON manifest を MCP consumer が直接検証する入力境界は未接続だった。
manifest の parser を MCP 専用に複製すると、schema version、referential closure、fail-closed
diagnostic が CLI と乖離する。

## Decision

- `lsharp_validate` の入力は `source`、`file`、`manifest`、`manifest_file` のいずれか一つに限定する。
- `manifest` は version 1 JSON object または JSON string、`manifest_file` は JSON file path とする。
- manifest input は `lsharp_types::validation_input::parse_intent_graph_json` へ渡し、source input と
  同じ `IntentGraph::validate()` / `ValidationReport::to_json_value()` を使う。
- `tools/list` の input schema は入力を `oneOf` で表し、manifest の object/string variant と file path
  の型を公開する。複数入力、未対応 schema version、unknown field、referential error は tool error
  (`isError: true`) として返し、成功 report に変換しない。
- この slice では MCP からの manifest emission は追加せず、validation report と manifest artifact を
  別の出力境界として維持する。

## Evidence

- RED: manifest 未対応時に direct object/file、schema、error boundary の 4 tests が失敗することを確認。
- GREEN: source/file 回帰を含む `mcp_server::tests` 27 tests が pass。direct object/string、
  `manifest_file`、JSON-RPC、複数 input rejection、schema version error を固定した。
- Follow-up GREEN: `tools/list` の `manifest` object schema が version 1 の required envelope
  (`schema_version` / `nodes` / `evidence` / `edges`)、optional `reviews`、unknown top-level field
  rejection を公開することを `test_validate_tool_manifest_input_schema_declares_versioned_graph_fields`
  で固定した。input/output schema は同じ helper を共有し、MCP consumer が parser の必須境界を
  schema だけで欠落させない。
- Presence follow-up GREEN: `reviews` を省略した `evaluates` edge は opaque endpoint として
  `status: unknown` を返し、`reviews: []` を明示した同じ未登録 edge は tool error として拒否する
  対照を `mcp_server::tests` へ追加した。CLI の presence semantics と MCP の report/error boundary
  を同じ parser policyで固定する。
- Gate: `cargo test -p lsharp-driver --bin lsharp mcp_server::tests -- --nocapture`（41 tests）、
  対象ファイルの rustfmt、`git diff --check`。
- Presence gate: 同 focused suite 43 tests（review registry 6 tests）と `cargo clippy -p lsharp-driver
  --bin lsharp --tests -- -D warnings`。
- Numeric schema follow-up: `tools/list` の manifest object schema に `nodes[].span` と
  `evidence[].execution.sampling` の unsigned fields（`start` / `end` / `cases` / `seed` /
  `shrinks[]` / `coverage.*`）を `type: integer` と `minimum: 0` で宣言した。入力と出力で同じ
  helper を共有し、MCP consumer が小数や負数を静的契約上受け付けない境界を parser の typed
  serde 契約と同期する。
- Numeric schema gate: 新規 schema boundary test と既存 `mcp_server::tests` 44 tests、対象 binary
  の `cargo clippy --tests -- -D warnings`、rustfmt、`git diff --check` を pass した。
- Numeric runtime follow-up: MCP の direct `manifest` string input でも、`span.start/end` と
  `sampling.cases/seed/shrinks[]/coverage.*` の fractional、`null`、`u64::MAX + 1` を全6 fieldで
  `validation manifest の parse` error として fail-closed にする回帰 matrix（18 cases）を追加した。
  MCP は report JSON や canonical manifest を返さず、`isError: true` の tool result に留める。
- Numeric runtime gate: `mcp_server::tests` 45 tests と対象 binary の `cargo clippy --tests -- -D warnings`
  を pass。既存 typed serde parser の境界を MCP tool まで接続した Rust-host verified sliceであり、
  production code の変更はない。
- Typed edge schema follow-up: 公開 `docs/schemas/intent-graph.schema.json` と同じ6 relation variant
  （`motivates` / `constrained-by` / `tested-by` / `supports|contradicts` / `evaluates` /
  `invalidates`）を `edges[].oneOf` へ追加し、stable ID の namespace/key pattern と、evidence /
  review / invalidation subject の kind enum を MCP input/output schema に反映した。input/output は
  同じ manifest helper を共有する。
- Typed edge schema gate: 新規 schema parity test と `mcp_server::tests` 46 tests、対象 binary の
  `cargo clippy --tests -- -D warnings`、rustfmt、`git diff --check`、docs audit（0 errors/warnings）を
  passした。
- Manifest-file numeric runtime follow-up: MCP の `manifest_file` input でも、`span.start/end` と
  `sampling.cases/seed/shrinks[]/coverage.*` の fractional、`null`、`u64::MAX + 1` を全6 fieldで
  `validation manifest の parse` error として fail-closed にする回帰 matrix（18 cases）を追加した。
  JSON-RPC は report/canonical manifest を返さず、`isError: true` かつ `structuredContent` なしの
  text tool error に留める。
- Manifest-file numeric runtime gate: `mcp_server::tests` 47 tests、対象 binary の
  `cargo clippy --tests -- -D warnings`、rustfmt、`git diff --check`、docs audit（0 errors/warnings）を
  passした。既存 typed serde parser の `manifest_file` route まで接続した Rust-host verified sliceであり、
  production code の変更はない。
- Draft 2020-12 validator follow-up: canonical `docs/schemas/intent-graph.schema.json` と MCP の
  `lsharp_validate` input/output schema を実 validator で compile し、canonical fixture の valid
  roundtrip と fractional、`null`、`u64::MAX + 1`、typed subject kind mismatch の reject を input/output
  両面で固定した。Rust `u64` と JSON Schema の整数境界を一致させるため、manifest の unsigned 6 fields に
  `maximum: 18446744073709551615` を canonical schema と MCP schema helper の双方へ追加した。
- Draft 2020-12 validator gate: `jsonschema` dev dependency、meta-schema validation、valid 1 case と
  invalid 4 cases を含む `mcp_server::tests` 48 tests、対象 binary の `cargo clippy --tests -- -D warnings`、
  rustfmt、`git diff --check`、docs audit（0 errors/warnings）を passした。これは Rust-host の schema
  contract verified sliceであり、native/selfhost MCP producer、current-source artifact/runtime、対応2 target
  の完了証拠ではない。
- Validation report follow-up: `intent-validation.schema.json` の external `$ref` を canonical
  `intent-graph.schema.json` resource として登録し、実際の `call_tool("lsharp_validate", include_manifest: true)`
  が返す report と inline manifest を Draft 2020-12 validator で検証した。valid canonical fixture の
  roundtrip と未知の `status` の reject を固定し、schema の report envelope が static schema だけでなく
  MCP の actual output も覆うことを確認した。
- Validation report gate: `$ref` 解決付き validator test と `mcp_server::tests` 49 tests、対象 binary の
  `cargo clippy --tests -- -D warnings`、rustfmt、`git diff --check`、docs audit（0 errors/warnings）を
  passした。これは Rust-host の report schema verified sliceであり、native/selfhost MCP producer、
  current-source artifact/runtime、対応2 target、EC-M3 全体の完了証拠ではない。
- Output schema parity follow-up: MCP `tools/list` の validation output schema を canonical
  `intent-validation.schema.json` と同期し、unknown top-level field、trace gap の未知 code / 空
  `subject_id`、64-bit `usize` counter 上限超過を reject する boundary を追加した。report counter 5項目に
  `maximum: 18446744073709551615`、trace gap の code enum / `minLength: 1`、object の
  `additionalProperties: false` を canonical/MCP 両方へ反映した。
- Output schema parity gate: canonical report validator と MCP output validator の valid report / 4 reject
  matrix を新規テストで固定し、`mcp_server::tests` 50 tests、対象 binary の `cargo clippy --tests -- -D warnings`、
  rustfmt、`git diff --check`、docs audit（0 errors/warnings）を passした。Rust-host schema contract の
  verified sliceであり、native/selfhost MCP producer、current-source artifact/runtime、対応2 target、
  EC-M3 全体の完了証拠ではない。
- Provenance schema parity follow-up: canonical `intent-graph.schema.json` が要求する evidence provenance
  の `producer` / `tool_version` / `timestamp` 非空境界を MCP input/output manifest schema に同期した。
  3 fields の空文字を canonical、MCP input、MCP output の Draft 2020-12 validator matrix で reject し、
  Rust parser の required provenance contract を schema consumer が弱めないことを固定した。
- Provenance schema parity gate: valid fixture と既存 numeric/typed subject matrixを含む
  `mcp_server::tests` 50 tests、対象 binary の `cargo clippy --tests -- -D warnings`、rustfmt、
  `git diff --check`、docs audit（0 errors/warnings）を passした。Rust-host manifest schema verified slice
  であり、selfhost/native MCP、current-source artifact/runtime、対応2 target、EC-M3 全体の完了証拠ではない。
- Provenance runtime follow-up: canonical fixture の `producer` / `tool_version` / `timestamp` を空文字へ
  変異させ、direct `manifest` と `manifest_file` の MCP route がいずれも `isError: true`、
  `structuredContent` なし、field名付き text error で fail-closed になる6ケースを追加した。schema static
  parityだけでなく、実際の Rust MCP parser route が required provenance contract を保持することを固定した。
- Provenance runtime gate: `mcp_server::tests::review_registry_tests` 14 tests、対象 binary の
  `cargo clippy --tests -- -D warnings`、rustfmt、`git diff --check`、docs audit（0 errors/warnings）を
  passした。全 `mcp_server::tests` では既存の `test_compile_run_tool_uses_wasi_default_for_wasm_output`
  が embedded artifact の `Invalid argument (os error 22)` で失敗したため、今回の MCP schema/runtime gate には
  focused suite を採用した。native/selfhost MCP、current-source artifact/runtime、対応2 target、EC-M3 全体
  の完了証拠ではない。
- Compile-run isolation follow-up: `lsharp_compile_run` が固定の共有 temp directory を毎回削除していたため、
  別 process / test の同時呼び出しが入力・Wasm artifact を相互に消し、`[LS5001] Invalid argument (os error 22)`
  を返し得る境界を修正した。呼び出しごとに PID・時刻・単調 sequence を含む専用 directory を作り、RAII の
  `Drop` で成功・失敗を問わず削除する。unique path と cleanup を `test_compile_run_temp_dirs_are_unique_and_cleaned_up`
  で先に RED、実装後 GREEN として固定した。
- Compile-run isolation gate: `mcp_server::tests` 52 tests（compile-run 3 tests を含む）、対象 binary の
  `cargo clippy --tests -- -D warnings`、rustfmt、`git diff --check`、docs audit（0 errors/warnings）を passした。
  `LSHARP_EMBED_COMPONENT_PATH` で既存 component artifact を明示した Rust-host laneであり、selfhost/native
  MCP producer、current-source stage0 artifact/runtime、対応2 target、EC-M3 全体の完了証拠には数えない。
- Duplicate top-level key runtime follow-up: version 1 manifest の `schema_version` を重複させた同一 JSON
  string を direct `manifest` と `manifest_file` の両 routeへ入力し、parser の duplicate-field rejection
  が `isError: true`、`structuredContent` なし、`validation manifest の parse` / `duplicate` /
  `schema_version` を含む text error へ伝播する2ケースを追加した。JSON object inputではなく string/file
  routeを使い、MCP transport前の `serde_json::Value` 上書きで duplicate key が消えないことも固定する。
- Duplicate top-level key runtime gate: `mcp_server::tests::review_registry_tests` 15 tests、対象 binary の
  clippy、`git diff --check`、docs auditを passした。対象テストファイル全体の rustfmt check は変更前から
  同じ既存差分で失敗するため、追加ブロックは rustfmt 出力へ合わせ、ベースライン差分を広げていない。
  これは Rust-host verified partial sliceであり、selfhost/native MCP producer、current-source stage0
  artifact/runtime、対応2 target、EC-M3 全体の完了証拠ではない。
- Unknown top-level field runtime follow-up: version 1 manifest の `unexpected` fieldを direct object、
  JSON string、`manifest_file` の3 routeへ入力し、canonical parserの unknown-field rejection が
  `isError: true`、`structuredContent` なし、`validation manifest の parse` と field名付き text errorへ
  伝播することを固定した。static schemaの `additionalProperties: false` だけでなく、実MCP transportの
  object serializationとstring/file parser routeを同じ fail-closed contractへ揃えた。
- Unknown top-level field runtime gate: `mcp_server::tests::review_registry_tests` 16 tests、対象 binaryの
  clippy、追加ブロックを含む対象rustfmt確認、`git diff --check`、docs auditを通過した。Rust-host
  verified partial sliceであり、selfhost/native MCP producer、current-source stage0 artifact/runtime、
  対応2 target、EC-M3 全体の完了証拠ではない。

### Input envelope closure follow-up (2026-07-31)

`lsharp_validate` の MCP `tools/list` input schema は、runtime の manifest parser が unknown
top-level field を fail-closed に拒否する一方、schema object 自体に `additionalProperties: false`
を宣言していなかった。consumer が static schema を正本として入力を検証する場合に、runtime と
schema の boundary がずれるため、input envelope を strict object として固定する。

- `validate_input_schema()` の top-level object に `additionalProperties: false` を追加する。
- `source` / `file` / `manifest` / `manifest_file` の `oneOf`、optional review context、manifest
  内部の version 1 schema は変更しない。
- RED: `test_validate_tool_declares_source_input_and_report_output_schema` が missing field を
  `null` として検出して失敗した。
- GREEN: 同 test と `test_manifest_schemas_use_draft202012_validator_for_valid_and_invalid_fixtures`
  で unknown top-level input を reject することを確認した。`mcp_server::tests` は69件 pass。

これは Rust-host MCP の静的 input envelope に限定した verified partial sliceであり、selfhost/native
MCP producer、current-source stage0 artifact/runtime、対応2 target、EC-M2-03 aggregate は未完了である。

### Non-empty route strings follow-up (2026-07-31)

`lsharp_validate` の runtime は、空の `manifest` JSON string を parser error、空の `file` / `manifest_file`
を I/O error、空の `trust_store` / `review_lifecycle` を path argument error として拒否している。一方、
MCP `tools/list` の input schema はこれらを単なる `string` として公開していたため、schema consumer と
runtime の入力契約がずれていた。

- `manifest` の string variant、`file`、`manifest_file`、`trust_store`、`review_lifecycle` に
  `minLength: 1` を追加する。
- 空 `source` は既存の空 program semanticsを保つため、同じ制約を追加しない。
- RED: `test_validate_tool_input_schema_rejects_empty_manifest_and_path_strings` が `file` などの
  `minLength` 欠落と、Draft 2020-12 validator が空 routeを受理することを検出した。
- GREEN: schemaの各 `minLength: 1` と、manifest/file/path の5つの空入力 rejectを同じテストで固定し、
  `mcp_server::tests` 70件を通過した。

これは Rust-host MCP input schema の static/runtime parity に限定した verified partial sliceであり、
selfhost/native MCP producer、current-source stage0 artifact/runtime、対応2 target、EC-M2-03 aggregate の
完了証拠ではない。

### Coverage bucket name schema closure (2026-07-31)

`SamplingPlan` の runtime validation は coverage bucket 名を `trim().is_empty()` で検査するが、canonical
`intent-graph.schema.json` と MCP `lsharp_validate` input/output schema の `coverage` object は
`additionalProperties` だけを指定していた。そのため空文字、ASCII空白、NBSP-only の bucket 名が static
schemaでは有効に見えていた。

- canonical schema と MCP の共有 `sampling_schema()` に `propertyNames.pattern: "\\S"` を追加する。
- bucket の count type・non-negative・maximum policyは変更しない。
- RED: `test_manifest_schemas_use_draft202012_validator_for_valid_and_invalid_fixtures` に空文字、ASCII空白、
  NBSP-only の coverage property nameを追加し、canonical/input/output validatorが空 bucketを受理することを
  確認した。
- GREEN: 同じ3ケースを canonical manifest schema、MCP input schema、MCP output schemaの全validatorで拒否し、
  valid canonical fixtureは維持した。focused `mcp_server::tests` はこの変更を含めて70件を通過した。

これは Rust-host の canonical/MCP static schema parity に限定した verified partial sliceであり、selfhost/native
manifest parser、current-source stage0 artifact/runtime、対応2 target、EC-M2-02/03 aggregateの完了証拠ではない。

## Boundary and follow-up

これは Rust MCP の manifest input/report wiring に限定した verified slice である。EmbeddedCli の
Rust-host actual Wasm manifest output wiringは別ADRで接続済みだが、native manifest emission、
selfhost/native report parity、Mac Apple Silicon / Linux x86_64 artifact/runtime evidence は
未完了のため、`TODO.md` の `EC-M2-03` は `[~]` のまま維持する。今回の numeric schema は static
`tools/list` 契約と Rust MCP lane に限定され、JSON Schema validator 実行、selfhost/native MCP、
current-source artifact/runtime、supported 2 targets の完了証拠には数えない。
MCP runtime matrixも同じく Rust-host lane に限られ、native/selfhost の診断・target parity は未検証である。
今回の typed edge schema も static `tools/list` contract と Rust MCP lane に限定され、JSON Schema 実
validator、selfhost/native MCP producer、Mac/Linux artifact/runtime の完了証拠には数えない。
今回の `manifest_file` numeric matrix も Rust-host JSON-RPC input boundary に限定され、selfhost/native
MCP、current-source artifact/runtime、JSON Schema 実 validator、Mac/Linux の target parity の完了証拠には
数えない。
今回の Draft 2020-12 validator は canonical/MCP schema の構文・valid fixture・主要 reject matrix を
 Rust-host で実行したが、selfhost/native producer、current-source stage0 artifact/runtime、Mac/Linux
 target parity、EC-M3 全体の完了証拠には数えない。
今回の validation report validator は `intent-validation.schema.json` の `$ref` と actual MCP report の
接続を Rust-host で検証したが、native/selfhost report producer、current-source stage0 artifact/runtime、
Mac/Linux target parity、EC-M3 全体の完了証拠には数えない。
今回の output schema parity は canonical report の strict boundary と MCP `tools/list` の静的 contract を
Rust-host で同期したが、native/selfhost producer、current-source stage0 artifact/runtime、Mac/Linux target
parity、EC-M3 全体の完了証拠には数えない。
今回の provenance schema parity は canonical manifest の required non-empty fields と MCP static schema を
Rust-host で同期したが、selfhost/native producer、current-source stage0 artifact/runtime、Mac/Linux target
parity、EC-M3 全体の完了証拠には数えない。
今回の provenance runtime matrix は Rust-host の direct/file MCP route に限定され、selfhost/native MCP
producer、current-source stage0 artifact/runtime、Mac/Linux target parity、EC-M3 全体の完了証拠には数えない。
今回の compile-run isolation は Rust MCP の temp directory ownership / cleanup 境界に限定され、Wasm
compiler の target parity、selfhost/native MCP producer、current-source stage0 artifact/runtime、Mac/Linux
target parity、EC-M3 全体の完了証拠には数えない。
今回の duplicate top-level key runtime matrix は JSON string/file の Rust MCP input route に限定され、JSON
object transport、selfhost/native MCP producer、current-source stage0 artifact/runtime、対応2 target、EC-M3
全体の完了証拠には数えない。
