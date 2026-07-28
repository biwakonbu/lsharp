# L# Active Backlog

このファイルは、**未完了タスクだけ**を持つ単一正本である。完了した項目は判断・結果・代表 evidence を
[`docs/adr/`](docs/adr/) または対応する仕様・運用記録へ残し、このファイルから削除する。

状態:

- `[ ]`: 未着手。次の RED と observable contract をまだ固定していない
- `[~]`: verified slice はあるが、項目全体の completion boundary を満たしていない
- `[BLOCKED: 理由]`: 外部状態または明示的な依存待ち

`[x]` は使わない。日付別の進捗ログ、個別 test 名、artifact hash、完了済み phase はここへ蓄積せず、
設計、ADR、test、artifact、運用記録を参照する。

## Current priority — v0.2 Milestone 2

正本:

- [Milestone 2 — Intent and evidence graph](docs/development/planning/v0.2-milestone-02.md)
- [Intent AST と stable ID](docs/development/planning/v0.2-intent-ast.md)
- [Evidence graph](docs/development/planning/v0.2-evidence-graph.md)
- [Intent validation model](docs/development/planning/v0.2-validation-model.md)

- [~] `EC-M2-01` intent AST と stable ID — Rust canonical model、source の
  `:intent` / `:claim` / `:assumption` / `:open-question`、`motivates` /
  `constrained-by` / `tested-by`、fail-closed な typed ID は verified。ID 省略時の命名規則、
  project-level duplicate 検査、selfhost/native parity を閉じる。selfhost parser の ADT/record
  定義 metadata 保持と `IntentSource` の node/typed-edge projection は Rust-host actual Wasm
  の verified slice として ADR に記録したが、native stage0 parity は残る。
- [~] `EC-M2-02` evidence graph — required provenance を持つ evidence record、
  `supports` / `contradicts` の registry closure、source の `shrinks` / `coverage`、
  `evaluates` / `invalidates` の Rust source typed-edge projection、Rust CLI の source→report/manifest
  projection、canonical manifest projection、optional review provenance registry の Rust CLI
  input/output roundtrip と未登録 review edge の fail-closed 検査、
  source `:review` の opaque registry producer と `validate --source --emit-manifest` の review
  registry projection、未知 visibility/空 digest/duplicate ID の span 付き fail-closed 検査、
  Rust MCP の review registry input/output schema と inline manifest projection、
  `public` / `redacted` privacy boundary は verified。実行 trace と generator policy、
  review provenance authentication、review/evaluates/invalidates lifecycle、外部 provenance と
  selfhost/native parity を閉じる。selfhost parser の `:review` kind 16 triple payload と
  `IntentSource` の opaque review registry projection は Rust-host actual Wasm の verified slice
  として追加済みであり、kind 17/18 の `evaluates` / `invalidates` edge projection と review
  registry closure/error boundary も verified slice として追加した。selfhost Evidence consumer の
  review registry、typed review/change edge、登録済み Evidence subject closure、optional `reviews`
  registry と manifest JSON projection も Rust-host actual Wasm で verified slice として追加した。
  ただし review lifecycle/authentication、native stage0 producer/runtime parity は残る。Rust
  canonical `IntentGraph::stale_subjects` による invalidated review → evaluated evidence の
  deterministic stale propagation も verified slice として追加したが、外部 lifecycle/provider
  auth/selfhost-native parity は未完了である。さらに `ValidationReport` が stale review/evidence
  件数を JSON/text facts として投影し、stale が残る場合に `unknown` へ fail-closed する Rust
  canonical report slice を追加した。公開 CLI/MCP、selfhost/native parity は未完了である。
  selfhost `Tools.Validation.Stale` の source graph projection と `App.Cli` /
  `EmbeddedCli` の stale report/unknown wiring は Rust-host actual Wasm で verified したが、
  native stage0 parity は未完了である。
  さらに review の visibility が未知値となる source fixture を native smoke に追加し、stable な
  `source validation error:8`、exit `1`、report/manifestなしの fail-closed 境界を要求した。
  既存 Rust source adapter の `InvalidReviewField`/invalid review 診断と同じ review metadata
  boundary を native contract へ接続する verified sliceだが、current source-commit に一致する
  実 stage0 artifact/runtime の evidence ではない。
  さらに review の `provenance_digest` が whitespace-only となる source fixtureも native smokeへ
  追加し、stable な `source validation error:8`、exit `1`、report/manifestなしの fail-closed 境界を
  要求した。既存 Rust source adapter の空 digest `InvalidReviewField` 診断と同じ metadata validation
  を native contractへ接続する verified sliceだが、実 stage0 artifact/runtime の evidence ではない。
  さらに review ID の wire shape が不正な source fixture を native smokeへ追加し、stable な
  `source validation error:2`、exit `1`、report/manifestなしの fail-closed 境界を要求した。既存 Rust
  source adapter の review ID parse 診断と同じ stable-ID validation を native contractへ接続する
  verified sliceだが、実 stage0 artifact/runtime の evidence ではない。
  さらに review ID が空文字となる source fixtureも native smokeへ追加し、stable な
  `source validation error:8`、exit `1`、report/manifestなしの fail-closed 境界を要求した。これは
  Rust source adapter の必須 review field 検査と同じ invalid-review metadata boundary を native
  contractへ接続する verified sliceだが、実 stage0 artifact/runtime の evidence ではない。
  さらに Rust source adapter が空 review ID を wire-format error へ落とさず `InvalidReviewField` の
  code `8` として返すよう修正し、selfhost actual Wasm E2E でも同じ review metadata boundary を固定した。
  native source-file smoke の既存 empty-review-ID fixtureと Rust/selfhost direct consumer を揃えた verified
  sliceだが、current source-commit に一致する packaged stage0 artifact/runtime の evidence ではない。
  さらに malformed review ID と whitespace-only provenance digest が同時にある場合も、required digest
  の `InvalidReviewField` code `8` を stable-ID wire error より先に返すよう Rust source adapter を修正し、
  selfhost actual Wasm と native source-file smoke の同じ precedence fixtureを追加した verified sliceだが、
  current source-commit に一致する packaged stage0 artifact/runtime の evidence ではない。
  さらに review payload の引数不足となる malformed source fixtureも native smokeへ追加し、stable な
  `source validation error:1`、exit `1`、report/manifestなしの fail-closed 境界を要求した。既存 Rust
  source adapter の malformed review metadata 診断と同じ parser boundary を native contractへ接続する
  verified sliceだが、実 stage0 artifact/runtime の evidence ではない。
  さらに review payload の引数過多となる malformed source fixtureも native smokeへ追加し、stable な
  `source validation error:1`、exit `1`、report/manifestなしの fail-closed 境界を要求した。Rust syntax
  oracle の `LS0101` arity rejection と同じ parser boundary を native contractへ接続する verified
  sliceだが、実 stage0 artifact/runtime の evidence ではない。
  さらに review edge の endpoint が不足する malformed source fixtureも native smokeへ追加し、stable な
  `source validation error:1`、exit `1`、report/manifestなしの fail-closed 境界を `evaluates` /
  `invalidates` の双方で要求した。既存 Rust source adapter の malformed review-edge 診断と同じ
  parser boundary を native contractへ接続する verified sliceだが、実 stage0 artifact/runtime の
  evidence ではない。
  さらに review edge の endpoint が余分になる malformed source fixtureも native smokeへ追加し、stable な
  `source validation error:1`、exit `1`、report/manifestなしの fail-closed 境界を `evaluates` /
  `invalidates` の双方で要求した。Rust syntax oracle の `LS0101` arity rejection と同じ parser
  boundary を native contractへ接続する verified sliceだが、実 stage0 artifact/runtime の evidence
  ではない。
  さらに `validate --source` の negative sampling を `App.Cli` / `EmbeddedCli` の Rust-host actual
  Wasm へ接続し、`source validation error:11`、exit `1`、report/manifestなしの fail-closed boundary
  を両 surface で固定した。current-source stage0、sampling の実行意味論は残る。
- [~] `EC-M2-03` `lsharp validate` — version 1 manifest parser、source adapter、
  `--emit-manifest` の atomic/durable file boundary、deterministic text/JSON report、
  optional `reviews` registry の Rust CLI roundtrip と未登録 review edge の non-zero/
  no-output contract、
  Rust MCP `lsharp_validate` の `reviews` input/output schema と `include_manifest` projection、
  `pass=0` / `fail=1` / `unknown=2` の Rust CLI は verified。selfhost `App.Cli` と
  `EmbeddedCli` の source/report/pass-fail-unknown/exit、EmbeddedCli の
  `--emit-manifest` による report と version 1 manifest file の分離出力は Rust-host
  actual Wasm で verified。さらに EmbeddedCli は manifest write failure 時に
  no-report/no-file と exit `1` を返す fail-closed boundary も Rust-host actual Wasm で
  verified。EC-M3-01 の Rust canonical source/manifest fixture、native
  source-file smoke の report/exit/bytes 契約、duplicate node の exit `1` /
  diagnostic-only fail-closed 契約、Cargo/Rust/host lsharp を成功経路から遮断する harness
  contract も verified。selfhost/native の manifest producer/parser、native canonical parity、
  atomic/durable emission、native stage0 の write/provenance failure、release-level
  provenance、両 supported target の runtime evidence を閉じる。Rust canonical report の
  `stale_reviews` / `stale_evidence` facts と stale→`unknown` policy、Rust MCP schema/output
  も verified slice として追加した。selfhost `App.Cli` / `EmbeddedCli` の stale report/exit
  parity は Rust-host actual Wasm で verified したが、native stage0 report parity、native MCP、
  両 supported target の runtime evidence は未完了である。さらに `--format text` の
  deterministic source report（trace gap、status、件数、stale facts）と `--format json` の
  option/exit parity を両 selfhost surface の Rust-host actual Wasm で verified した。Cli の
  complete graph（独立 review 付き）`pass=0` / `status: pass` も同じ text/json status projection
  として verified した。EmbeddedCli の complete graph でも `--format text` の `status: pass` /
  exit `0` と同じ deterministic line projection を verified した。native source-file smoke の
  inner runnerにも canonical fixtureの `--format text` unknown report（6行の固定順、exit `2`）を
  要求する contract を追加したが、これは fake Lima/provenance harness の verified sliceであり、
  current source-commit に一致する実 stage0 artifact/runtime の証拠ではない。さらに native
  source-file smoke は `--emit-manifest` の親ディレクトリ欠落を stable write failure、exit `1`、
  no-report/no-file として要求する contract を持つが、atomic/durable writer と実 stage0 runtime
  の証拠ではない。さらに independent review を含む complete graph fixtureの JSON `status: pass` /
  exit `0` と text 6行 projection を native smoke contract に追加したが、current source-commit に
  一致する stage0 artifact/runtime での実行 evidence は未取得である。さらに contradiction
  fixture の JSON `status: fail` / exit `1`（`independent_reviews: 1`、
  `contradicting_observations: 1`）と同じ text 6行 projectionを native smoke contract に追加した。
  これは report を保持する判定 failure と parse/graph/write の diagnostic-only failure を分離する
  fake Lima/provenance harness の verified sliceであり、current source-commit に一致する実 stage0
  artifact/runtime の evidence ではない。さらに invalidation された review と、それを
  `evaluates` する evidence の stale propagation fixture を native smoke に追加し、JSON/text の
  `status: unknown` / exit `2`、`stale_reviews: 1`、`stale_evidence: 1`、stderr 空を要求した。
  これは Rust-host/selfhost source projection と native contract を揃える verified sliceだが、
  current source-commit に一致する実 stage0 artifact/runtime の evidence ではない。さらに
  orphan edge が存在しない node を参照する source fixture を native smoke に追加し、stable な
  `source validation error:5`、exit `1`、report/manifestなしの fail-closed 境界を要求した。
  これは既存 Rust source adapter の `MissingNodeReference` と同じ observable error code を
  native contract へ接続する verified sliceだが、current source-commit に一致する実 stage0
  artifact/runtime の evidence ではない。
  さらに edge payload が不足する malformed source fixture を native smoke に追加し、stable な
  `source validation error:1`、exit `1`、report/manifestなしの fail-closed 境界を要求した。
  既存 Rust source adapter の malformed edge/span 診断と同じ入力拒否境界を native contract
  へ接続する verified sliceだが、current source-commit に一致する実 stage0 artifact/runtime
  の evidence ではない。
  さらに wire prefix が不正な edge endpoint の source fixture を native smoke に追加し、stable な
  `source validation error:2`、exit `1`、report/manifestなしの fail-closed 境界を要求した。
  既存 Rust source adapter の malformed ID 診断と同じ入力拒否境界を native contract へ接続する
  verified sliceだが、current source-commit に一致する実 stage0 artifact/runtime の evidence ではない。
  さらに metadata kind と wire prefix が不一致な node fixture を native smoke に追加し、stable な
  `source validation error:3`、exit `1`、report/manifestなしの fail-closed 境界を要求した。
  既存 Rust source adapter の typed kind mismatch 診断と同じ入力拒否境界を native contract へ接続する
  verified sliceだが、current source-commit に一致する実 stage0 artifact/runtime の evidence ではない。
  さらに未登録 evidence を `supports` する source fixture を native smoke に追加し、stable な
  `source validation error:6`、exit `1`、report/manifestなしの fail-closed 境界を要求した。
  既存 Rust source adapter の `EvidenceRegistryRequired` 診断と同じ registry closure 境界を native
  contract へ接続する verified sliceだが、current source-commit に一致する実 stage0 artifact/runtime
  の evidence ではない。
  さらに required field が不足する malformed evidence source fixtureを native smoke に追加し、stable な
  `source validation error:1`、exit `1`、report/manifestなしの fail-closed 境界を要求した。Rust
  syntax oracle の evidence metadata required-field `LS0101` 診断と同じ parser boundary を native
  contractへ接続する verified sliceだが、current source-commit に一致する実 stage0 artifact/runtime
  の evidence ではない。
  さらに evidence の method enum が未定義となる source fixture を native smoke に追加し、stable な
  `source validation error:8`、exit `1`、report/manifestなしの fail-closed 境界を要求した。Rust
  source adapter の `InvalidEvidenceField` 診断と同じ typed evidence field boundary を native
  contractへ接続する verified sliceだが、current source-commit に一致する実 stage0 artifact/runtime
  の evidence ではない。
  さらに evidence の outcome、independence、subject kind が不正となる source fixtureも native smoke
  に追加し、いずれも stable な `source validation error:8`、exit `1`、report/manifestなしの
  fail-closed 境界を要求した。Rust source adapter の `InvalidEvidenceField` 診断と同じ typed
  evidence enum matrix を native contractへ接続する verified sliceだが、current source-commit に
  一致する実 stage0 artifact/runtime の evidence ではない。
  さらに selfhost Evidence consumer の invalid `:method` が native source contract と同じ code `8` を
  返し、field/value を保持する Rust-host E2E を追加した。native stage0 artifact/runtime parity は未完了の
  ままだが、selfhost direct consumer の typed-field code drift を閉じる verified sliceである。
  さらに selfhost Evidence consumer の invalid `:independence` も native source contract と同じ code `8` を
  返し、field/value を保持する Rust-host E2E を追加した。native stage0 artifact/runtime parity は未完了の
  ままだが、selfhost direct consumer の typed-field code drift を閉じる verified sliceである。
  さらに selfhost Evidence consumer の unsupported `:subject` kind も native source contract と同じ code `8` を
  返し、field/value を保持する Rust-host E2E を追加した。native stage0 artifact/runtime parity は未完了の
  ままだが、selfhost direct consumer の typed-field code drift を閉じる verified sliceである。
  さらに selfhost Evidence consumer の invalid `:outcome` も native source contract と同じ code `8` を返し、
  field/value を保持する Rust-host E2E を追加した。既存の selfhost enum 判定が native contract と一致する
  ことを直接固定する verified sliceであり、native stage0 artifact/runtime parity は未完了のままである。
  さらに empty `:method` / `:outcome` / `:independence` を selfhost の required-field code `4` から
  native enum-invalid code `8` へ揃え、3フィールドの Rust oracle、selfhost E2E、native source-file smoke を
  追加した。enum は enum validator、runner/target/provenance は required-field validator が担当する
  boundary を固定した verified sliceだが、native stage0 artifact/runtime parity は未完了のままである。
  さらに同じ evidence ID を二度宣言する source fixtureも native smoke に追加し、stable な
  `source validation error:3`、exit `1`、report/manifestなしの fail-closed 境界を要求した。Rust
  source adapter / selfhost registry の duplicate evidence rejection と同じ identity boundary を
  native contractへ接続する verified sliceだが、current source-commit に一致する実 stage0
  artifact/runtime の evidence ではない。
  さらに evidence named field を同じ record 内で二度宣言する malformed source fixtureも native
  smoke に追加し、stable な `source validation error:1`、exit `1`、report/manifestなしの
  fail-closed 境界を要求した。Rust syntax oracle の duplicate named field `LS0101` 診断と同じ
  parser boundary を native contractへ接続する verified sliceだが、current source-commit に一致する
  実 stage0 artifact/runtime の evidence ではない。
  さらに evidence の required sampling generator が空文字となる source fixtureも native smoke に
  追加し、stable な `source validation error:4`、exit `1`、report/manifestなしの fail-closed 境界を
  要求した。Rust `EvidenceGraph` の `EvidenceValidationError::EmptyField { field: "generator" }` と
  selfhost Evidence registry の required-field code `4` を source contractへ接続する verified slice
  だが、current source-commit に一致する実 stage0 artifact/runtime の evidence ではない。
  さらに evidence の required execution identity runner が空文字となる source fixtureも native
  smoke に追加し、stable な `source validation error:4`、exit `1`、report/manifestなしの fail-closed
  境界を要求した。Rust `EvidenceValidationError::EmptyField { field: "runner" }` と selfhost Evidence
  registry の required-field code `4` を source contractへ接続する verified sliceだが、current
  source-commit に一致する実 stage0 artifact/runtime の evidence ではない。
  さらに invalid evidence ID と空の required runner が同時にある場合も、runner の
  `EvidenceValidationError::EmptyField` code `4` を stable-ID wire error より先に返すよう Rust source
  adapter を修正し、selfhost actual Wasm と native source-file smoke の同じ precedence fixtureを追加した
  verified sliceだが、current source-commit に一致する実 stage0 artifact/runtime の evidence ではない。
  さらに evidence の required execution target が空文字となる source fixtureも native smoke に追加し、
  stable な `source validation error:4`、exit `1`、report/manifestなしの fail-closed 境界を要求した。
  Rust `EvidenceValidationError::EmptyField { field: "target" }` と selfhost Evidence registry の
  required-field code `4` を source contractへ接続する verified sliceだが、current source-commit に
  一致する実 stage0 artifact/runtime の evidence ではない。
  さらに evidence の required source commit が空文字となる source fixtureも native smoke に追加し、
  stable な `source validation error:4`、exit `1`、report/manifestなしの fail-closed 境界を要求した。
  Rust `EvidenceValidationError::EmptyField { field: "source_commit" }` と selfhost Evidence registry の
  wire field `source-commit` / required-field code `4` を source contractへ接続する verified sliceだが、
  current source-commit に一致する実 stage0 artifact/runtime の evidence ではない。
  さらに evidence の required artifact digest が空文字となる source fixtureも native smoke に追加し、
  stable な `source validation error:4`、exit `1`、report/manifestなしの fail-closed 境界を要求した。
  Rust `EvidenceValidationError::EmptyField { field: "artifact_digest" }` と selfhost Evidence registry の
  wire field `artifact-digest` / required-field code `4` を source contractへ接続する verified sliceだが、
  current source-commit に一致する実 stage0 artifact/runtime の evidence ではない。
  さらに evidence の required provenance producer が空文字となる source fixtureも native smoke に追加し、
  stable な `source validation error:4`、exit `1`、report/manifestなしの fail-closed 境界を要求した。
  Rust `EvidenceValidationError::EmptyField { field: "producer" }` と selfhost Evidence registry の
  required-field code `4` を source contractへ接続する verified sliceだが、current source-commit に
  一致する実 stage0 artifact/runtime の evidence ではない。
  さらに evidence の required provenance tool version が空文字となる source fixtureも native smoke に
  追加し、stable な `source validation error:4`、exit `1`、report/manifestなしの fail-closed 境界を要求した。
  Rust `EvidenceValidationError::EmptyField { field: "tool_version" }` と selfhost Evidence registry の
  wire field `tool-version` / required-field code `4` を source contractへ接続する verified sliceだが、
  current source-commit に一致する実 stage0 artifact/runtime の evidence ではない。
  さらに evidence の required provenance timestamp が空文字となる source fixtureも native smoke に
  追加し、stable な `source validation error:4`、exit `1`、report/manifestなしの fail-closed 境界を要求した。
  Rust `EvidenceValidationError::EmptyField { field: "timestamp" }` と selfhost Evidence registry の
  required-field code `4` を source contractへ接続する verified sliceだが、current source-commit に
  一致する実 stage0 artifact/runtime の evidence ではない。
  さらに evidence の required execution runner が空白だけとなる source fixtureも native smoke に
  追加し、stable な `source validation error:4`、exit `1`、report/manifestなしの fail-closed 境界を要求した。
  Rust canonical の `trim().is_empty()` と selfhost Evidence registry の nonblank 判定を同じ
  `runner` fieldへ接続する verified sliceだが、current source-commit に一致する実 stage0
  artifact/runtime の evidence ではない。
  さらに source node の stable ID 欠落と本文欠落の fixtureを native smoke に追加し、stable な
  `source validation error:1`、exit `1`、report/manifestなしの fail-closed 境界を要求した。Rust
  syntax oracle の `LS0101` required-operand 診断と同じ ID 明示 / non-empty text policy を native
  contractへ接続する verified sliceだが、current source-commit に一致する実 stage0 artifact/runtime
  の evidence ではない。
  さらに source node の本文が空白だけとなる fixtureも native smoke に追加し、stable な
  `source validation error:1`、exit `1`、report/manifestなしの fail-closed 境界を要求した。Rust
  canonical の `text.trim().is_empty()` と selfhost `IntentSource` の nonblank 判定を同じ node
  text policyへ接続する verified sliceだが、current source-commit に一致する実 stage0
  artifact/runtime の evidence ではない。
  さらに invalid stable ID と whitespace-only node text が同時にある場合も、本文の
  `NodeTextError::EmptyText` / malformed code `1` を stable-ID wire error より先に返すよう Rust source
  adapter を修正し、selfhost actual Wasm と native source-file smoke の同じ precedence fixtureを追加した
  verified sliceだが、current source-commit に一致する実 stage0 artifact/runtime の evidence ではない。
  さらに evidence の `:subject` が空白だけとなる source fixtureも native smoke に追加し、stable な
  `source validation error:2`、exit `1`、report/manifestなしの fail-closed 境界を要求した。Rust
  canonical の stable ID parser と selfhost `source-wire-shape-valid?` を同じ subject wire policyへ
  接続する verified sliceだが、current source-commit に一致する実 stage0 artifact/runtime の
  evidence ではない。
  さらに evidence の ID が空白だけとなる source fixtureも native smoke に追加し、stable な
  `source validation error:2`、exit `1`、report/manifestなしの fail-closed 境界を要求した。Rust
  canonical の `EvidenceId::parse` と selfhost の evidence ID wire validation を同じ stable-ID
  policyへ接続する verified sliceだが、current source-commit に一致する実 stage0 artifact/runtime の
  evidence ではない。
  さらに evidence の ID が空文字となる source fixtureも native smoke に追加し、stable な
  `source validation error:2`、exit `1`、report/manifestなしの fail-closed 境界を要求した。Rust
  canonical の `EvidenceId::parse` と selfhost の evidence ID wire validation を同じ empty stable-ID
  policyへ接続する verified sliceだが、current source-commit に一致する実 stage0 artifact/runtime の
  evidence ではない。
  さらに review registry が存在する状態で未登録 review を `evaluates` する source fixture を native
  smoke に追加し、stable な `source validation error:10`、exit `1`、report/manifestなしの fail-closed
  境界を要求した。既存 Rust source adapter の `MissingReview` 診断と同じ review registry closure
  境界を native contract へ接続する verified sliceだが、current source-commit に一致する実 stage0
  artifact/runtime の evidence ではない。
  さらに同じ review ID を二度宣言する source fixture を native smoke に追加し、stable な
  `source validation error:7`、exit `1`、report/manifestなしの fail-closed 境界を要求した。
  既存 Rust source adapter の `DuplicateReview` 診断と同じ registry uniqueness 境界を native
  contract へ接続する verified sliceだが、current source-commit に一致する実 stage0 artifact/runtime
  の evidence ではない。
  さらに review edge が review 自体を `evaluates` の subject に指定する source fixture を native
  smoke に追加し、stable な `source validation error:9`、exit `1`、report/manifestなしの fail-closed
  境界を要求した。既存 Rust source adapter の `EdgeSubjectKindMismatch` 診断と同じ review edge
  subject-kind 境界を native contract へ接続する verified sliceだが、current source-commit に一致する
  実 stage0 artifact/runtime の evidence ではない。
  さらに `invalidates` の subject に claim を指定する source fixtureも native smokeへ追加し、
  同じ stable code `9`、exit `1`、report/manifestなしの fail-closed を要求した。`evaluates` と
  `invalidates` の typed review-edge subject boundary を双方の native contractへ接続する verified
  sliceだが、current source-commit に一致する実 stage0 artifact/runtime の evidence ではない。
  さらに登録済み review と未登録 review を同じ source に置き、`invalidates` が未登録 review を参照
  する fixtureも native smokeへ追加した。stable な `source validation error:10`、exit `1`、
  report/manifestなしの fail-closed を要求し、review registry closure の両 edge relation を native
  contractへ接続する verified sliceとしたが、実 stage0 artifact/runtime の evidence ではない。
  さらに明示 review registry の未登録 `evaluates` review と不正な review subject kind を同時に
  持つ fixtureを追加し、subject-kind code `9` より先に missing-review code `10` を返す precedence を
  Rust source adapter、selfhost、native smoke で統一した verified sliceだが、current source-commit に
  一致する実 stage0 artifact/runtime の evidence ではない。
  さらに review の `evaluates` / `invalidates` が未登録 evidence を subject にする source fixtureも
  native smokeへ追加し、stable な `source validation error:6`、exit `1`、report/manifestなしの
  fail-closed を双方の relation で要求した。既存 Rust source adapter の `EvidenceRegistryRequired`
  と同じ review-edge evidence registry boundary を native contractへ接続する verified sliceだが、
  current source-commit に一致する実 stage0 artifact/runtime の evidence ではない。
  さらに `supports` / `contradicts` が未登録で wire shape も不正な evidence を参照する fixtureを
  追加し、stable な `source validation error:6`、exit `1`、report/manifestなしの fail-closed を
  両 relation で要求した。Rust source adapter も stable-ID parse より先に registry closure を
  判定するよう修正し、selfhost/native と同じ evidence-edge precedence を固定した verified sliceだが、
  current source-commit に一致する実 stage0 artifact/runtime の evidence ではない。

さらに明示 review registry の未登録 `evaluates` / `invalidates` review endpoint を source-local
`MissingReviewReference` として保持し、relation、ID、directive span を Rust source adapter の
diagnostic に残すようにした。selfhost の code `10` / span と native source-file smoke の
no-report/no-manifest boundary に対応する verified sliceだが、driver の EmbeddedCli build、実
stage0 artifact/runtime、Mac/Linux matrix、native fallback exclusion の evidence ではない。

さらに evidence の required field (`runner` / `target` / `source-commit` / `artifact-digest` /
`generator` / `producer` / `tool-version` / `timestamp`) が空または whitespace-only の場合を
`InvalidEvidenceRequiredField` として field、value、directive span 付きで返すようにした。selfhost
の stable code `4` / span と native source-file smoke の no-report/no-manifest boundary に対応する
verified sliceだが、driver の EmbeddedCli build、実 stage0 artifact/runtime、Mac/Linux matrix、
native fallback exclusion の evidence ではない。

さらに source node の `text` が空または whitespace-only の場合を `InvalidNodeField` として
field、value、directive span 付きで返すようにした。selfhost source adapter の stable code `1` /
span E2E と native source-file smoke の no-report/no-manifest boundary に対応する verified sliceだが、
driver の EmbeddedCli build、実 stage0 artifact/runtime、Mac/Linux matrix、native fallback exclusion
の evidence ではない。

さらに source node の stable ID wire format／segment が不正な場合を `NodeIdAt` として directive
span 付きで返すようにした。selfhost source adapter の stable code `2` / span E2E と native
source-file smoke の no-report/no-manifest boundary に対応する verified sliceだが、driver の
EmbeddedCli build、実 stage0 artifact/runtime、Mac/Linux matrix、native fallback exclusion の evidence
ではない。

さらに evidence record 自体の stable ID wire format／segment が不正な場合を `EvidenceIdAt` として
directive span 付きで返すようにした。selfhost Evidence consumer の stable code `2`、field/value、
span E2E と native source-file smoke の no-report/no-manifest boundary に対応する verified sliceだが、
driver の EmbeddedCli build、実 stage0 artifact/runtime、Mac/Linux matrix、native fallback exclusion
の evidence ではない。

さらに evidence の `subject` ID wire format／segment が不正な場合を `EvidenceSubjectIdAt` として
directive span 付きで返すようにした。selfhost Evidence consumer の stable code `2`、field/value、
span E2E と native source-file smoke の no-report/no-manifest boundary に対応する verified sliceだが、
driver の EmbeddedCli build、実 stage0 artifact/runtime、Mac/Linux matrix、native fallback exclusion
の evidence ではない。

さらに evidence の `:coverage` に空 bucket が含まれる場合を Rust source adapter の
`InvalidEvidenceField`（field/value/directive span）として拒否し、selfhost Evidence consumer の
empty-field code `4` と malformed／empty／negative／duplicate coverage の directive span を揃えた。
Rust source suite 54件、selfhost evidence registry 39件、source adapter 30件、Linux x86_64 native
stage0 source-file smoke/provenance gate を通過した verified partial sliceである。whitespace-only
bucket policy、coverage count、parser/manifest/validate CLI 全体、current-source artifact/runtime、
Mac/Linux matrix、EC-M2-02 aggregate は残件。ADR: `docs/adr/decisions-v0.2-native-validation-evidence-coverage-bucket.md`。

canonical `SamplingPlan` / `EvidenceGraph` でも空 coverage bucket を登録前に
`EvidenceValidationError::EmptyField { field: "coverage" }` として拒否するよう揃えた。
`evidence_required_fields` 6件、`evidence_graph` 5件、source suite 54件、Linux x86_64 native
stage0 source-file smoke/provenance gate を通過した verified partial sliceである。duplicate/
whitespace policy、coverage count/cases、manifest/validate CLI、current-source artifact/runtime、
Mac/Linux matrix、EC-M2-02 aggregate は残件。ADR: `docs/adr/decisions-v0.2-native-validation-evidence-canonical-sampling.md`。

続く coverage whitespace policy では、canonical `SamplingPlan` / `EvidenceGraph`、Rust source
adapter、selfhost Evidence consumer の whitespace-only bucket を共通の empty-field policy へ揃えた。
元の bucket value と directive/form span を保持し、selfhost/native の stable code `4`、exit `1`、
report/manifestなしを固定した。evidence required fields 8件、evidence graph 5件、source suite 55件、
selfhost evidence registry 40件、source adapter 31件、Linux x86_64 native source-file smoke/provenance
gateを通過した verified partial sliceである。Unicode whitespace、duplicate/count/cases、manifest/
validate、current-source artifact/runtime、Mac/Linux matrix、EC-M2-02 aggregate は残件。ADR:
`docs/adr/decisions-v0.2-native-validation-evidence-coverage-whitespace.md`。

さらに duplicate coverage bucket の parser-owned boundary を native source-file smokeへ接続した。
Rust parser は `LS0101`、selfhost direct Evidence registry は code `10` で重複を拒否し、native stage0
 source-file smoke は parser error code `1`、exit `1`、report/manifestなしを要求する。Rust source
 adapter に後段の `BTreeMap` duplicate 再判定は追加せず、parser を通過した canonical source record
 の責務を維持した verified partial sliceである。canonical map が duplicate を表現できない点、
 coverage count/cases、Unicode whitespace、manifest/validate、current-source artifact/runtime、
 Mac/Linux matrix、EC-M2-02 aggregate は残件。ADR:
`docs/adr/decisions-v0.2-native-validation-evidence-duplicate-coverage-parser.md`。

version 1 JSON manifest input でも whitespace-only coverage bucket を canonical graph 登録前に
`GraphError::InvalidEvidence { source: EvidenceValidationError::EmptyField { field: "coverage" } }`
として拒否する回帰テストを追加した。`validation_input` 17件、rustfmt、diff checkを通過した
verified partial sliceである。manifest の native source/runtime parity、report/atomic writer、
coverage count/cases、Unicode whitespace、Mac/Linux matrix、EC-M2-03 aggregate は残件。ADR:
`docs/adr/decisions-v0.2-native-validation-manifest-coverage-whitespace.md`。

次の実装は `EC-M2-01`〜`EC-M2-03` の未接続入力を一つの RED に絞る。current plan の
acceptance と依存順を確認し、完了 slice の履歴を TODO へ再展開しない。

## v0.2 Milestone 1 closure

個別 slice の履歴と current boundary は
[Rust 依存境界の縮小](docs/development/operations/rust-boundary-reduction.md) を正本とする。

- [~] `EC-M1-01` Rust/selfhost observable parity — invariant scope、computation/match、
  diagnostics、module/import、qualified/private record の parser/type/runtime slice と
  両 supported target の current-source core stage0 smoke は verified。constructor/record/GADT の
  残る semantics、全 diagnostic/span、standalone source check、full cross-target aggregate を閉じる。
- [~] `EC-M1-02` canonical metadata IR — canonical case/assert/property inventory、
  typed binder、precondition/postcondition、directive span の slice は verified。一般 `TypeExpr`、
  全 `ContractSuite` evaluator、binder/predicate 個別 span、formatter/docs、2 target evidence を閉じる。
- [~] `EC-M1-03` form separation and migration — canonical form と legacy migration report の
  text/JSON slice は verified。全 form evaluator、schema、formatter/docs/MCP、2 target evidence を閉じる。
- [~] `EC-M1-04` strict predicate and non-vacuity — Bool preflight、zero-case、
  static reachability/vacuity の slice は verified。動的・compound predicate、全 diagnostic/span、
  evaluator/runtime、2 target aggregate を閉じる。
- [~] `EC-M1-05` reproducible type-directed sampling — Int/Bool/String の deterministic prefix は
  verified。一般 `TypeExpr`、constraint generator、seed/shrink/coverage、2 target evidence を閉じる。
- [~] `EC-M1-06` structured assurance report — implementation conformance と intent validation を
  混同しない text/JSON report の slice は verified。全 form、EmbeddedCli、Rust/selfhost differential、
  provenance、2 target evidence を閉じる。
- [~] `EC-M1-07` native parity and migration closure — current-source native fixed-point と
  source-file smoke は両 target の verified slice を持つ。Rust oracle、standalone Wasm、
  full public surface、guide/schema/MCP/migration docs を同じ observable contract へ揃える。

## V2-16 — Rust dependency boundary reduction

`V2-16a` no-Cargo development loop と `V2-16d` native development E2E は完了履歴へ移動済み。
残る aggregate は次のとおり。

- [~] `LEGACY-LANG-01` record pattern parity — source/ftable の direct/nested pattern、
  nominal marker、field binding は verified。一般 Map API、全 pattern、import target、
  Rust ABI parity を actual E2E で閉じる。
- [~] `LEGACY-LANG-02` ADT/GADT execution parity — ordinary ADT の direct/nested constructor と
  GADT parser/type refinement は verified。nominal/exhaustiveness、full ftable/import、
  linear-memory/WasmGC runtime parity を閉じる。
- [~] `LEGACY-COMP-01` full-program compiler closure — 主要 CLI builder は full-program 化済み。
  diagnostic-only legacy `lower`、no-arg pipeline runtime/native E2E、component sidecar の
  artifact boundary を閉じる。
- [~] `V2-16b` / `LEGACY-IO-01` native artifact I/O — bounded argv/file/raw-byte Preview1 と
  4096 bytes 超 read の slice は verified。全 fd error semantics、dynamic root/data/heap layout、
  component sidecar、target 別 release artifact を閉じる。
- [~] `V2-16c` / `LEGACY-TOOL-01` public command closure — `install` / `repl` / `lsp --stdio` /
  `doc` / component helper の routing contract は verified。実 stage0 と外部 tool の E2E、
  Rust-only flag/target の明示境界、target 別 release evidence を閉じる。
- [~] `V2-16e` / `LEGACY-BOOT-01` bootstrap/oracle/rollback isolation — source commit と
  fingerprint を検証する stage0 package と両 target の daily Rust-free core slice は verified。
  public acquisition、current-checkout regeneration、release asset、rollback 実行、
  Rust oracle/host integration の隔離を閉じる。

## ISSUES-derived quality and runtime work

[ISSUES.md](ISSUES.md) の active issue を実装可能な aggregate へまとめる。issue の問題定義と
根拠は ISSUES、作業順と completion boundary は本節を正本とする。

- [~] `LEGACY-DIAG-01` stable diagnostics — Issue `I-02`。syntax/types/IR/codegen と主要
  CLI/LSP/MCP forwarding は verified。compile multi-file、REPL、doc、metadata、native linker、
  LSP incremental/module/codegen の code/span forwarding を閉じる。
- [~] `LEGACY-RUNTIME-01` dynamic GC layout and allocator — Issues `I-03` / `I-04` / `D-10`。
  core WASI の object/free/root table growth と allocation failure の stable `LS4002` は verified。
  free-list size class、sentinel precise discrimination、HTTP/component/selfhost/native parity を
  actual runtime/metrics と両 supported target で閉じる。
- [~] `LEGACY-EXEC-01` advanced runtime — Issues `D-01` / `D-02` / `D-03` / `D-04` /
  `D-06` / `D-09`。WasmGC の record/ADT/string/closure slice は verified。GADT/HKT/
  computation expression、trait vtable、selfhost representation、supported 2 target を閉じる。
- [~] `LEGACY-ROOT-01` rooting discipline — Issue `I-07`。runtime failure ledger と
  compiler root-lifetime ledger の slice は verified。全 selfhost source、stateful REPL/LSP、
  indirect control flow、Mac/Linux native stage0 を閉じる。
- [~] `LEGACY-MODULE-01` SCC inference and cache generalization — Issues `D-07` / `I-05`。
  SCC detection/inference、Formatter batch 特例除去、Rust host の process/artifact cache、
  validation/runtime と明示 maintenance は verified。Formatter 固有 dirty-set、canonical runtime、
  source override の segment/disk persistence、自動 eviction、selfhost/native compiler、
  public command と両 supported target の evidence を閉じる。
- [~] `LEGACY-MAINT-01` large-file decomposition — Issues `I-01` / `I-08`。多数の test/
  production split と `lsharp-ir/src/lib.rs` の `Instruction` / `IrType` および
  `Module` / `Function` / GC model、linker seam、compile surface seam、compile/incremental orchestration seam、`validation_source` node/evidence/typed edge seam、validation source adapter test seam、selfhost evidence registry runtime/validation test seam、selfhost evidence parser duplicate-field seam、selfhost native differential test seam、selfhost bootstrap four-layer test seam、selfhost bootstrap acceptance test seam、selfhost typeinfer E2E test seam、selfhost lexer/parser parity E2E test seam、WasmGC probe test seam、selfhost native stage23 gap test seam、validation input manifest/reference seam、native emitter memory seam、atomic/durable writer cleanup test seam、validation output manifest wire seam、native selfhost transport strict payload-length seam、WASI runner Preview1/Preview2 mode seam は verified。`wasi.rs`、`lsharp-ir/src/lib.rs`、`lsharp-tooling/src/compile.rs`、
  `infer.rs`、parser/lower/driver/LSP の責務分割を、型・focused test・snapshot parity を保って完了する。WasmGC emitter の instruction lowering / Component output seam / Preview2・CLI runner seam、WASI HTTP handler core seam、WASI GC collector seam、WASI tests core seam も verified とし、残る責務分割を続ける。
- [~] `LEGACY-TEST-01` property/fuzz/limit coverage — Issues `I-06` / `I-08`。syntax/types
  property test と複数の GC/type/runtime limit lane、bounded regex repeat の 64-case property
  lane は verified。再利用可能な generator、leak/rooting stress、performance threshold、
  full fuzz target、native stage0 evidence を閉じる。

## Scheduling rules

- current milestone は `EC-M2-01` → `EC-M2-02` → `EC-M2-03`。同時に一つの observable contract だけを進める。
- product/release completion target は `aarch64-apple-darwin` と `x86_64-unknown-linux-gnu` に限定する。
- Linux VM / stage regeneration は共有 lock と既存 artifact を使い、同じ heavy replay を重複起動しない。
- Rust は bootstrap、oracle/differential、rollback、未移行 host integration の明示境界として保持する。
- verified slice は `[~]` のまま残し、aggregate completion 後に evidence を ADR/仕様へ移して本ファイルから削除する。

2026-07-29 に EC-M2-02 evidence required-field Unicode whitespace boundary を追加した。Rust
source adapter の `str::trim()` と selfhost `Tools.Validation.Whitespace` の UTF-8 byte 判定を
揃え、NBSP を含む Unicode White_Space-only `runner` を stable code `4`、directive/form span 付きで
拒否する。Rust source test、selfhost actual Wasm の同一 fixture、主要 Unicode White_Space 10 種の
direct runtime、Linux x86_64 native source-file smoke/provenance gate を通過した verified partial slice
である。node text、review provenance、manifest input、coverage count/cases、current-source
artifact/runtime、Mac/Linux matrix、EC-M2-02/EC-M3 aggregate は残件。ADR:
`docs/adr/decisions-v0.2-native-validation-evidence-unicode-whitespace.md`。

2026-07-29 に EC-M2-02 review provenance digest の Unicode whitespace boundary を追加した。Rust
source adapter の `trim().is_empty()` と selfhost `IntentSource` の共有 UTF-8 byte helper を揃え、
NBSP-only `provenance_digest` を stable code `8`、review ID、directive/form span 付きで拒否する。
Rust source suite 57件、selfhost actual Wasm の同一 fixtureと既存 review precedence、Linux x86_64
native source-file smoke/provenance gateを通過した verified partial sliceである。visibility、review
lifecycle/authentication、manifest/MCP、current-source artifact/runtime、Mac/Linux matrix、EC-M2-02/
EC-M3 aggregate は残件。ADR: `docs/adr/decisions-v0.2-native-validation-review-unicode-whitespace.md`。

2026-07-29 に EC-M2-01 node text の Unicode whitespace boundary を追加した。Rust source adapter の
`trim().is_empty()` と selfhost `IntentSource` の共有 UTF-8 byte helper を揃え、NBSP-only node本文を
stable code `1`、kind/ID、directive/form span 付きで拒否する。Rust source suite 58件、selfhost actual
Wasm の同一 fixtureと既存 whitespace/precedence/type-source 回帰、Linux x86_64 native source-file
smoke/provenance gateを通過した verified partial sliceである。manifest/MCP、current-source
artifact/runtime、Mac/Linux matrix、EC-M2-01/EC-M3 aggregate は残件。ADR:
`docs/adr/decisions-v0.2-native-validation-node-text-unicode-whitespace.md`。

2026-07-29 に EC-M2-02 coverage bucket の Unicode whitespace boundary を追加した。Rust source
adapter の `trim().is_empty()` と selfhost `Tools.Validation.Whitespace` の UTF-8 byte 判定を揃え、
NBSP-only coverage bucket を stable code `4`、raw value、directive/form span 付きで拒否する。Rust
source suite 59件、selfhost actual Wasm の parser→registry 同一 fixture、Linux x86_64 native source-file
smoke/provenance gateを通過した verified partial sliceである。duplicate/count/cases、manifest/MCP、
current-source artifact/runtime、Mac/Linux matrix、EC-M2-02/EC-M3 aggregate は残件。ADR:
`docs/adr/decisions-v0.2-native-validation-evidence-coverage-unicode-whitespace.md`。

2026-07-29 に EC-M2-02 coverage count の負値 boundary を追加した。Rust source parser は負の
coverage count を parse error 集約 code `LS0104` で拒否し、selfhost actual Wasm の parser→Evidence
registry は invalid-sampling code `11`、field `coverage`、bucket value、directive/form span を保持
して拒否する。native source-file smoke には `source validation error:11`、exit `1`、report/manifestなし
の fixture/assertion を追加し、provenance gate と `bash -n` を通過した verified partial sliceである。
`sum(coverage counts) == cases` の意味論、count 上限、manifest/validate、current-source artifact/runtime、
Mac/Linux matrix、EC-M2-02 aggregate は残件。ADR:
`docs/adr/decisions-v0.2-native-validation-evidence-negative-coverage-count.md`。

2026-07-29 に EC-M2-02 evidence `cases` の負値 boundary を追加した。Rust source parser は
`:cases -1` を stable code `LS0101`、selfhost actual Wasm の source parser→Evidence registry は
invalid-sampling code `11`、field `cases`、empty raw value、directive/form span 付きで拒否する。native
source-file smoke には `source validation error:11`、exit `1`、report/manifestなしの fixture/assertionを
追加し、provenance gate と `bash -n` を通過した verified partial sliceである。`sum(coverage counts) ==
cases` の意味論、非負値上限、manifest/validate、current-source artifact/runtime、Mac/Linux matrix、
EC-M2-02 aggregate は残件。ADR:
`docs/adr/decisions-v0.2-native-validation-evidence-negative-cases.md`。

2026-07-29 に EC-M2-02 evidence `seed` の負値 boundary を追加した。Rust source parser の既存
negative-seed testは `LS0101`、selfhost actual Wasm の source parser→Evidence registry は
invalid-sampling code `11`、field `seed`、empty raw value、directive/form span 付きで拒否する。native
source-file smoke には `source validation error:11`、exit `1`、report/manifestなしの fixture/assertionを
追加し、provenance gate と `bash -n` を通過した verified partial sliceである。generator/shrink/coverage
の実行意味論、manifest/validate、current-source artifact/runtime、Mac/Linux matrix、EC-M2-02 aggregate
は残件。ADR: `docs/adr/decisions-v0.2-native-validation-evidence-negative-seed.md`。

2026-07-29 に EC-M2-02 evidence `shrinks` の負値 boundary を追加した。Rust source parser の既存
negative-shrink testは `LS0101`、selfhost actual Wasm の source parser→Evidence registry は
invalid-sampling code `11`、field `shrinks`、empty raw value、directive/form span 付きで拒否する。native
source-file smoke には `source validation error:11`、exit `1`、report/manifestなしの fixture/assertionを
追加し、provenance gate と `bash -n` を通過した verified partial sliceである。generator/shrink の実行
意味論、manifest/validate、current-source artifact/runtime、Mac/Linux matrix、EC-M2-02 aggregate は残件。
ADR: `docs/adr/decisions-v0.2-native-validation-evidence-negative-shrinks.md`。

2026-07-29 に `validate --source` の negative sampling CLI boundary を `EmbeddedCli` へ拡張した。
`App.Cli` と同じ `:cases -1` fixtureを EmbeddedCli の Rust-host actual Wasm で実行し、
`source validation error:11`、exit `1`、validation reportなし、`--emit-manifest` の出力なしを固定した。
これで両 selfhost CLI surface の source/report fail-closed boundary が verified partial slice となったが、
current-source stage0 artifact/runtime、sampling の実行意味論、supported 2 targets、EC-M2-02/03 aggregate
は残件。ADR: `docs/adr/decisions-v0.2-native-validation-cli-negative-sampling.md`。

さらに `validate --source` の `seed` / `shrinks` 負値 boundary を EmbeddedCli の Rust-host actual Wasm
へ拡張した。`:seed -1` と `:shrinks [-1]` の両 fixtureで `source validation error:11`、exit `1`、
validation reportなし、`--emit-manifest` の出力なしを同一 bundle の実行で固定した。current-source
stage0 artifact/runtime、sampling の実行意味論、supported 2 targets、EC-M2-02/03 aggregate は残件。
ADR: `docs/adr/decisions-v0.2-native-validation-cli-negative-sampling.md`。

さらに version 1 JSON manifest の `sampling.coverage` duplicate key を map 化前の serde visitor で
拒否し、後続値への黙った上書きを防ぐ Rust canonical input boundary を verified slice として追加した。
selfhost/native manifest parity、coverage count/cases の意味論、current-source artifact/runtime は残る。
ADR: `docs/adr/decisions-v0.2-validation-manifest-duplicate-coverage.md`。

2026-07-29 に version 1 JSON manifest の evidence required-field Unicode whitespace boundary を追加した。
execution の `runner` / `target` / `source_commit` / `artifact_digest` と sampling/provenance の
`generator` / `producer` / `tool_version` / `timestamp` を NBSP-only に変異させ、canonical graph 登録前に
`GraphError::InvalidEvidence` / `EvidenceValidationError::EmptyField` として field名を保持して拒否する
ことを `parse_manifest_rejects_unicode_whitespace_only_required_evidence_fields` で固定した。既存の
`str::trim()` policyを manifest inputにも適用する Rust verified partial sliceであり、manifest の
node/review/coverage Unicode parity、selfhost/native manifest parser、current-source artifact/runtime、
supported 2 targets、EC-M2-02/EC-M3 aggregate は残件。Evidence:
`docs/adr/decisions-v0.2-validation-manifest-evidence-unicode-whitespace.md`。

2026-07-29 に version 1 JSON manifest の node text Unicode whitespace boundary を追加した。complete
manifest の intent node text を NBSP-only に変異させ、canonical `IntentNode` の登録前に
`ValidationInputError::Node(NodeTextError::EmptyText)` として拒否することを
`parse_manifest_rejects_unicode_whitespace_only_node_text` で固定した。Rust source adapter と同じ
`str::trim()` policyを manifest inputへ拡張した verified partial sliceであり、manifest の review/
coverage Unicode parity、selfhost/native manifest parser、current-source artifact/runtime、supported
2 targets、EC-M2-01/EC-M3 aggregate は残件。Evidence:
`docs/adr/decisions-v0.2-validation-manifest-node-text-unicode-whitespace.md`。
