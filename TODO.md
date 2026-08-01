# L# Active Backlog

このファイルは、**未完了タスクだけ**を持つ単一正本である。完了した項目は判断・結果・代表 evidence を
[`docs/adr/`](docs/adr/) または対応する仕様・運用記録へ残し、このファイルから削除する。

状態:

- `[ ]`: 未着手。次の RED と observable contract をまだ固定していない
- `[~]`: verified slice はあるが、項目全体の completion boundary を満たしていない
- `[BLOCKED: 理由]`: 外部状態または明示的な依存待ち

`[x]` は使わない。日付別の進捗ログ、個別 test 名、artifact hash、完了済み phase はここへ蓄積せず、
設計、ADR、test、artifact、運用記録を参照する。

## Next milestone — v0.3 review provenance / lifecycle

正本: [`v0.3-review-provenance-lifecycle.md`](docs/development/planning/v0.3-review-provenance-lifecycle.md)

次版の設計と task 分解（v0.3 completion 後に active backlog へ昇格）:
[`v0.4-lsharp-next-shape.md`](docs/development/planning/v0.4-lsharp-next-shape.md)、
[`v0.4-milestone-01.md`](docs/development/planning/v0.4-milestone-01.md)。

v0.4 M1-01 の semantic fixture matrix では、artifact を要求する fixture が `compile` または
`build` を commands に宣言しない場合を validator が拒否する command/artifact scope boundary を
verified partial として追加した。実 Rust/native artifact、runtime、Mac/Linux matrix は未接続のため
v0.4 の完了項目には移さない。さらに artifact/runtime の期待値と `codegen`/`wasm`、
`runtime`/`runtime` の layer・observable 対を validator が拒否する scope boundary を verified
partial として追加した。実 Rust/native artifact、runtime、Mac/Linux matrix は未接続のため
v0.4 の完了項目には移さない。さらに evidence index の report/comparison/ADR 参照で symlink
traversal と project root 外への resolve を拒否する safe-reference boundary を verified partial として
追加した。実 Rust/native artifact、runtime、Mac/Linux matrix は未接続のため v0.4 の完了項目には
移さない。ADR: `docs/adr/decisions-v0.4-m1-01-artifact-command-boundary.md`、
`docs/adr/decisions-v0.4-m1-01-scope-boundary.md`、
`docs/adr/decisions-v0.4-m1-06-evidence-reference-boundary.md`。さらに evidence index の
`task` と fixture matrix `suite` の identity mismatch を拒否する task-scope boundary を verified
partial として追加した。実 Rust/native artifact、runtime、Mac/Linux matrix は未接続のため v0.4 の
完了項目には移さない。さらに artifact を要求する evidence index の command を `compile`/`build`
に限定し、`check` route で artifact/runtime を宣言できない command-scope boundary を verified
partial として追加した。実 Rust/native artifact、runtime、Mac/Linux matrix は未接続のため v0.4 の
完了項目には移さない。ADR: `docs/adr/decisions-v0.4-m1-06-task-scope-boundary.md`、
`docs/adr/decisions-v0.4-m1-06-command-scope-boundary.md`。さらに oracle/native report の producer
role (`rust-oracle` / `native-stage0`) の取り違えを拒否する producer-scope boundary を verified partial
として追加した。実 Rust/native artifact、runtime、Mac/Linux matrix は未接続のため v0.4 の完了項目には
移さない。ADR: `docs/adr/decisions-v0.4-m1-06-report-producer-boundary.md`。さらに report/index の
`source_commit` を指定 root の current `HEAD` に束縛し、同じ stale commit を全入力へコピーしても
pass しない current-source boundary を verified partial として追加した。実 Rust/native artifact、runtime、
Mac/Linux matrix は未接続のため v0.4 の完了項目には移さない。ADR:
`docs/adr/decisions-v0.4-m1-06-current-source-boundary.md`。さらに evidence index の `adr` を
`docs/adr/*.md` に限定する ADR-reference boundary を verified partial として追加した。実 Rust/native
artifact、runtime、Mac/Linux matrix は未接続のため v0.4 の完了項目には移さない。ADR:
`docs/adr/decisions-v0.4-m1-06-adr-reference-boundary.md`。
同じ `docs/adr/*.md` path shape を JSON Schema にも反映する schema-parity boundary を verified partial
として追加した。実 Rust/native artifact、runtime、Mac/Linux matrix は未接続のため v0.4 の完了項目には
移さない。ADR: `docs/adr/decisions-v0.4-m1-06-adr-schema-boundary.md`。
さらに JSON Schema の `task` を `V4-M1-01` constant に固定する task-schema parity boundary を verified
partial として追加した。実 Rust/native artifact、runtime、Mac/Linux matrix は未接続のため v0.4 の完了項目には
移さない。ADR: `docs/adr/decisions-v0.4-m1-06-task-schema-boundary.md`。

さらに evidence index の oracle/native report と comparison の参照を task-owned `ci-artifacts/`
配下に限定する artifact-namespace boundary を schema と executable audit に揃えた verified partial
として追加した。実 Rust/native artifact、runtime、Mac/Linux matrix は未接続のため v0.4 の完了項目には
移さない。ADR: `docs/adr/decisions-v0.4-m1-06-artifact-namespace-boundary.md`。

さらに evidence artifact の report/comparison path を
`ci-artifacts/v4-m1-01/<source_commit>/<target>/` に分離し、同一 source commit の Mac/Linux
結果の上書きと target 取り違えを拒否する target-scoped artifact boundary を executable audit に接続した
verified partial として追加した。実 Rust/native artifact、runtime、Mac/Linux matrix は未接続のため v0.4 の
完了項目には移さない。ADR: `docs/adr/decisions-v0.4-m1-06-target-artifact-boundary.md`。

さらに evidence index 自体も同じ target-scoped artifact namespace 配下の regular file に限定し、
bundle 外の index や symlink index を report/comparison と結び付けられない index-ownership boundary を
executable audit に追加した verified partial として記録する。実 Rust/native artifact、runtime、Mac/Linux
matrix は未接続のため v0.4 の完了項目には移さない。ADR:
`docs/adr/decisions-v0.4-m1-06-index-artifact-boundary.md`。

さらに Mac Apple Silicon と Linux x86_64 の target index を再監査して束ねる two-target aggregate
audit/schema を追加した。片側 pending/mismatch や Mac-only pass を aggregate pass に昇格させない
verified partial として記録する。実 target artifact/runtime、rollback、provider parity は未接続のため
v0.4 の完了項目には移さない。ADR: `docs/adr/decisions-v0.4-m1-06-two-target-aggregate.md`。

- [~] `EC-M3-01` attestation model / canonical bytes — Rust model、strict timestamp、明示 clock、
  canonical bytes、signature encoding boundary は verified partial slice。source/native producer、
  trust store、署名検証、両対応 target の runtime evidence は残る。
- [~] `EC-M3-02` lifecycle transition — append-only registry と stale/revoked 境界の Rust verified
  slice。selfhost reducerにも deterministic ordering、transition、sequence rollback、`effective_at`
  rollback（code `8` と前後 timestamp payload）、explicit clock 以下の最新 `event_at` 選択を接続し、
  Rust `review_lifecycle` 6件・clock gate 1件と selfhost lifecycle E2E 2件で parityを確認した。
  source/native report parity、provider snapshot、release evidence は残る。
- [~] `EC-M3-03` CLI/MCP explicit inputs — explicit context、clock、trust/lifecycle input の Rust
  CLI/MCP boundary は verified partial slice。MCP input schema の subject/source/now/artifact all-or-none
  も `dependentRequired` で runtime boundaryへ接続した。native MCP subset の `review_now` canonical UTC
  lexical schema と実行前 reject も verified partial として追加した。selfhost/native MCP と target artifact
  parity は残る。ADR: `docs/adr/decisions-v0.3-native-mcp-review-clock-schema.md`。
- [~] `EC-M3-04` source / selfhost / native producer parity — `:review` 互換を維持した named-field
  `:review-attestation` の Rust parser/source adapter、selfhost kind 20、`unverified` state、span、
  canonical bytes parity、invalid algorithm/signature/timestamp/time-window の fail-closed contract を verified。native
  source-file smoke、Evidence/manifest projection、current-source と packaged stage0 の provenance は残る。
  2026-07-31 に `tests/fixtures/validation/ec-m3-review-attestation-source.ls` を canonical fixture とし、
  Rust parser/Rust-host selfhost/Mac smoke/Linux stage0 wrapperの同一入力と Linux fake provenance harness の
  fixture copy 契約を追加した。Linux current-source runtime、Mac/Linux matrix、packaged provenance は残る。
  同日、current `f6a6da30` の Mac Apple Silicon stage0 producer/package と actual App.Cli E2E、source-file
  smoke を実行し、Mac current-source runtime の named-field/attestation/validation smoke を pass させた。
  既定 macOS `TMPDIR` の task-owned producer artifact path拒否も再現し、`TMPDIR_ROOT/lsharp-*` のみを
  許可する safe cleanup boundaryを追加した。末尾 `/` を含む既定 `TMPDIR` の `T//lsharp-*` 連結も
  stage0 wrapperで正規化し、actual producer/package（484.89秒）と生成 stage0の source-file smokeを通過した。
  さらに current `3e1b2690` の Mac Apple Silicon producer/package（actual `App.Cli` E2E 510.15秒）と
  `aarch64-apple-darwin` stage0 manifest/source-file evidence smokeを実行し、source commit/target、
  named-field attestation、期限付き/期限なし canonical bytes、`unverified`、directive span、invalid
  source error code `8`、stdout/stderr、exit code、Wasm digest/sizeの一致を確認した。Linux x86_64
  current-source runtime、fetch後の packaged provenance と両 target matrixは残る。
  source-file evidence writer は stage0 manifest の `source_commit` を小文字40桁 hexadecimal に限定し、
  uppercase input を証跡作成前に拒否する contract test を追加した。これは provenance input boundary の
  verified partial sliceであり、Linux current-source runtime、packaged provenance、両 target matrixは残る。
  同じ writer は work directory 内の symlink も staging 前に拒否し、外部 path を参照する証跡を保存しない
  contract test を追加した。これは task-owned evidence の安全境界に限る verified partial sliceであり、Linux
  current-source runtime、packaged provenance、両 target matrixは残る。
- [~] `EC-M3-05` release / evidence gate — Rust CLI/MCP と manifest の入出力 roundtrip が、明示した
  subject/source/artifact/clock と trust-store/lifecycle component digest を `review_evidence_identity`
  として deterministic JSON/text/MCP/manifest へ投影し、競合を fail-closed に拒否する verified partial
  slice。native source-file smoke の explicit identity JSON/text/manifest、nullable digest、all-or-none
  input boundaryも verified partial として追加した。native MCP subset の provider/output/input と
  manifest nested schema parity、canonical `review_now` lexical preflight、explicit identity の
  report/manifest postflight一致検証、identity optionsなし source/file routeの no-implicit boundaryも
  verified partial として追加した。
  selfhost `App.Cli` の explicit identity
  JSON/text/manifest parity も Rust-host actual Wasm で verified partial として追加した。offline
  release identity verifier を native-only
  archive / packaged stage0 の optional identity projection と release smoke の再検証へ接続したが、
  明示 artifact/trust-store/lifecycle bytes を canonical identity へ変換する offline input preparer の
  verified partial sliceを追加した。preparer/verifier の `now` は Rust canonical timestamp と同じ閏年・暦日・時分秒範囲を
  共有し、lexical shapeだけでは通る不正日時を fail-closed にする。verifier は明示 snapshot の raw bytes を任意に再計算し、trust-store/lifecycle digest mismatch と片側指定を fail-closed にする。
  2026-08-01 に `--manifest` の JSON root shape も検証し、配列・null・数値を traceback ではなく
  return code `1` の `review_evidence_identity` 入力エラーへ変換する contract test を追加した。これは
  manifest runtime validation の verified partial sliceであり、native MCP の
  LSP/package API/provider semantics、current-source target runtime、
  packaged bytes parity、manifest runtime validation の残りは未完了である。
  同日、native MCP `lsharp_validate` の inline `manifest` / `manifest_file` についても、malformed JSON と
  配列・null・数値 root を native 実行前に `JSON object` 入力エラーへ変換する preflight を追加した。
  54件の native MCP suite と fake native no-execution contract で確認した verified partial sliceである。
  ADR: `docs/adr/decisions-v0.3-native-mcp-manifest-input-root.md`。
  さらに native MCP が出力する manifest について、malformed JSON、非 object root、schema version不一致、
  required field欠落、未知 top-level field、nodes/evidence/edges の非配列を wrapper 側で fail-closed にした。
  native MCP 55件で traceback なしの出力境界を確認した verified partial sliceである。
  ADR: `docs/adr/decisions-v0.3-native-mcp-manifest-output-root.md`。
  さらに native `lsharp_validate` report の root、required fields、status、count、collection、optional field
  境界を wrapper 側で検証し、array/null/malformed/missing/unknown/invalid status/boolean count を traceback
  なしで拒否した。native MCP 56件で確認した verified partial sliceである。
  ADR: `docs/adr/decisions-v0.3-native-mcp-validate-report-root.md`。
  `lsharp_check` の native output についても root、required fields、boolean `ok`、diagnostics 配列を
  wrapper 側で fail-closed にし、array/null/malformed/missing/unknown/type mismatch を traceback なしで拒否した。
  native MCP 57件で確認した verified partial sliceである。
  ADR: `docs/adr/decisions-v0.3-native-mcp-check-output-root.md`。
  2026-07-31 に native source-file smoke の identity options なし JSON/manifest が
  `review_evidence_identity` を暗黙生成しない no-implicit boundary を追加した。実 native MCP、provider、
  current-source/packaged runtime は残る。
  positional version 1 manifest input の既存 `review_evidence_identity` についても、同値 caller
  identity の再 attach と conflicting identity の `source validation error:14` / exit `1` / no-report・
  no-manifest fail-closed を `ManifestInput` / `App.Cli`、Rust-host actual Wasm、Linux fake provenance
  harness で verified partial とした。Linux current-source native stage0、native MCP、provider、両 target
  runtime は残る。
  さらに Mac current-source stage0 packageを実際に source-file smokeへ渡し、identity source fixtureの
  trace (`motivates` / `tested-by`) と explicit identity JSON/text/manifest、同値再 attach、conflict
  fail-closed を actual runtimeで確認した。Linux runtime、native MCP/provider、fetch後の packaged parityは残る。
  native-only `scripts/release.sh` から snapshot path を verifier へ伝播し、packaging 前の digest mismatch を拒否する wiring も verified partial として追加した。
  stage0 package は `--review-trust-store` / `--review-lifecycle` を同じ verifier へ渡し、release smoke は
  `RELEASE_REVIEW_TRUST_STORE` / `RELEASE_REVIEW_LIFECYCLE` から native-only archive の identity を再検証する
  offline propagation まで verified partial として追加した。さらに
  `native-official-release-local.sh` が同じ snapshot を両 target の release、stage0 package、Mac smoke、
  Linux VM smoke へ伝播する offline orchestrator wiring も verified partial として追加した。さらに
  stage0 fetch 後の directory を Mac host / Linux Lima の既存 source-file runtime smoke へ渡す wiringを
  fake two-target harness で固定した。provider helperの実取得、selfhost/native MCP parity、
  current-source と packaged stage0 の provenance、Mac Apple Silicon / Linux x86_64 の実 release
  artifact/runtime gate は残る。さらに Mac `App.Cli` producer、transport、materializer、stage0 package
  builderを接続する current-source producer wrapper と既存 output の fail-closed を fake harnessで
  固定した。実 Mac/Linux runtime、provider取得、bytes parity は残る。ADR:
  `docs/adr/decisions-v0.3-release-identity-gate.md`,
  `docs/adr/decisions-v0.3-provider-input-identity-preparer.md`,
  `docs/adr/decisions-v0.3-provider-snapshot-digest-verification.md`,
  `docs/adr/decisions-v0.3-native-release-snapshot-wiring.md`,
  `docs/adr/decisions-v0.3-stage0-release-smoke-snapshot-wiring.md`,
  `docs/adr/decisions-v0.3-native-official-multitarget-snapshot-wiring.md`,
  `docs/adr/decisions-v0.3-native-official-stage0-runtime-smoke.md`,
  `docs/adr/decisions-v0.3-native-macos-stage0-producer.md`。さらに
  `NATIVE_SELFHOST_SOURCE_SMOKE_EVIDENCE_DIR` による source-file smoke の一時 work directory、
  stdout/stderr、Wasm bytes、exit code の保存と digest/size manifest を contract test で固定した。
  これは実 target gate の operator evidence を保持する verified partial sliceであり、実 Mac/Linux
  runtime、provider取得、packaged bytes parity は残る。ADR:
  `docs/adr/decisions-v0.3-native-source-smoke-evidence.md`。さらに official stage0 runtime gateへ
  `NATIVE_OFFICIAL_SOURCE_SMOKE_EVIDENCE_ROOT` を接続し、Mac hostと Linux/Limaの target別 source-file
  smoke evidenceを別 leafへ保持する wiringを追加した。Linux側は evidence writerをVMへコピーし、source
  smoke終了後に stdout/stderr、Wasm digest/size、exit code、stage0 manifestをhostへ再帰コピーする。
  fake `limactl` の成功/失敗 copy-out contractで、失敗時の元 exit code保持も確認した。
  official gateだけでなく直接の Linux source-file smoke入口も hostgen replay lockを
  `limactl` 前に検査し、live ownerでは exit `90`、owner metadata出力、外部呼び出しなしで停止する
  fake contractを追加した。これは operator safetyの verified partial sliceであり、Linux runtimeや
  packaged provenanceの completion evidenceではない。
  2026-07-31 に fresh current `e44ca727` の Mac Apple Silicon stage0 producer（actual App.Cli E2E
  491.75秒）→stage0 archive→local HTTP `fetch-stage0.sh`→fetched package source-file smokeを完走し、
  source commit/target、stage0 payload、`compile.wasm` / `build.wasm` digest/size、App.Cli
  `--version` / `--help` の exit/stderrを確認した。これは証跡伝播と Mac current-source/fetch runtime の
  verified partial sliceであり、実 Linux runtime、provider取得、両 target packaged bytes/rollback parityは残る。ADR:
  `docs/adr/decisions-v0.3-native-official-stage0-runtime-smoke.md`。2026-08-01 に current
  `origin/main` `be17567f35f2f688a51652efc0bb6ba31ed12582` の Mac Apple Silicon stage0 producerを
  actual App.Cli E2E（489.04秒）で完走させ、manifest digest
  `574f4fedbf3a0bd7d034c9443b99fa2137eed3199ad06320f581c583606a4f48`（259 bytes）、source commit/target/payloadを確認した。
  relative `--output-dir` の stage0 release archive生成失敗を RED→GREEN の regression testで閉じ、
  package→local file `fetch-stage0.sh` checksum/provenance→fetched source-file smokeを通過させた。
  producer/fetched smokeとも exit `0`、`compile.wasm` / `build.wasm` は digest
  `afd1638e444a7e8c371dc1d17550479fcc5e4efbbb9e9dbdffa8551933d71a00`（2,559 bytes）で一致した。
  これは Mac current-source/fetch runtimeの verified partial sliceであり、Linux runtime、provider取得、両 target packaged bytes/rollback parityは残る。
  さらに native manifest の
  node/evidence/subject ID fields を一つの serializer helper へ統合し、EC-M3-01 canonical
  manifest bytes の working-tree preflight parity を確認した。これは serializer の verified
  sliceであり、fresh producer と両 target runtime の証拠は残る。ADR:
  `docs/adr/decisions-v0.3-ec-m3-native-id-serializer.md`。さらに native parser/CLI の
  review-attestation span、missing/extra metadata の malformed boundary、evidence registry
  precedence、positional manifest input、manifest write failure、stderr/exit propagation を
  Rust contract に揃え、Mac source-file smoke の全 negative/manifest/report cases を blocked
  tool preflight で通過した。これは working-tree source と旧 stage0 を使った verified preflight
  であり、fresh current-source producer、packaged bytes、Linux x86_64 runtime、provider/rollback
  evidence は残る。ADR:
  `docs/adr/decisions-v0.3-native-validation-boundary-followups.md`。

2026-08-01 の current `origin/main` `1cdbe555f63c909fbfb3940c8462cf4b08ba442d` では、Mac/Linux
App.Cli producer、Linux hostgen fixed point、protocol stage0 compiler、provider snapshot identity、
rollback archive、stage0 package/fetch、official release smoke を同一 source commit で確認した。
Mac producer/fetched source-file smoke は exit `0` で Wasm bytes が一致し、Linux current-source
source-file smoke は別タスクの live hostgen replay lock により exit `90` で待機中である。native
MCP と provider API/auth adapter、および Linux runtime は未完了のため `[~]` を維持する。ADR:
`docs/adr/decisions-v0.3-current-source-two-target-runtime.md`。

同日、native selfhost `mcp-server` を JSON-RPC shim経由の `lsharp_check` / `lsharp_validate` /
`lsharp_format` deterministic subsetへ接続した。native program以外の fallbackは呼ばず、provider
snapshot pathは明示した2つのファイルを offline で raw bytes の SHA-256 digestへ変換して native
validateへ渡す。片側指定、欠損、非 regular、空、明示 digestとの不一致は native 実行前に fail-closed
とし、ネットワーク・provider helper・署名/lifecycle意味検証は呼ばない。focused runner/protocol
tools/list の JSON Schema に provider path と review identity 4-field の all-or-none
`dependentRequired` を公開し、`additionalProperties: false` と shim自身の unknown-field拒否も
native 実行前に揃えた。focused
testsは通過したが、Rust MCP 全tool parity、provider取得/auth、署名/lifecycle検証、Linux runtimeは
未完了のため `[~]` を維持する。`lsharp_check` の migrationDiagnostics output schema（code/owner/
semantics/disposition/range）と `lsharp_validate` report の trace gap/review identity/
review verification schemaも Rust contractへ揃え、manifest top-level（version/nodes/evidence/edges）
も明示した。ADR:
`docs/adr/decisions-v0.3-native-mcp-subset-shim.md`,
`docs/adr/decisions-v0.3-native-mcp-provider-snapshot-adapter.md`,
`docs/adr/decisions-v0.3-native-mcp-review-identity-contract.md`,
`docs/adr/decisions-v0.3-native-mcp-validate-input-closed-world.md`,
`docs/adr/decisions-v0.3-native-mcp-check-output-schema.md`,
`docs/adr/decisions-v0.3-native-mcp-validate-output-schema.md`,
`docs/adr/decisions-v0.3-native-mcp-manifest-output-schema.md`。

2026-08-01 に native MCP の lsharp_errors、lsharp_search、lsharp_project_context、lsharp_package_api、lsharp_stdlib_api を追加した。canonical
crates/lsharp-driver/src/error_codes.rs の read-only table projection として LS####、
E0001-E0005（E0003 は LS1002 へ統合）、未知コード、docs link を返し、Rust/native schema の
必須・空文字拒否・additionalProperties: false を揃えた。fake native program を実行しないことを
native MCP 17 tests と Rust schema/error focused tests で確認した。lsharp_search は `.lsharp/packages`
下の `lsharp.toml` を読む offline installed-package projection として、query、決定的順序、closed-world
schema、fake native program 非実行を native MCP 19 tests と Rust MCP focused 56 tests で確認した。
lsharp_project_context は `lsharp.toml` の project/exports/dependencies と `.lsharp/packages` を offline
projection し、依存名の決定的順序、closed-world schema、引数 fail-closed、fake native program 非実行を
native MCP 21 tests と Rust MCP focused 59 tests で確認した。lsharp_package_api は deterministic な
installed package の既存 `docs/api.json` projection に加え、artifact が無い場合の sorted `src/**/*.ls`
ごとの native `doc --json` in-memory projection を追加した。full closed-world schema、nested malformed
field と malformed native doc の fail-closed、引数 fail-closed、既存 artifact route の fake native program
非実行、生成 route の `docs/api.json` 非作成を native MCP 47 tests と native LSP relay 5 tests、Rust `mcp_server::tests` 87 tests
（package API filter 6 tests、driver unit 214 tests）で確認した。lsharp_stdlib_api は Rust canonical `doc --json` から生成した
`.lsharp/packages` の file/dangling symlink は package として列挙せず、directory symlink は path dependency 互換のため保持する Rust/native
discovery contract も追加した。ADR: `docs/adr/decisions-v0.3-native-mcp-package-entry-boundary.md`。
`stdlib/api.json` を読む offline projectionに加え、artifact が無い場合の sorted direct `stdlib/*.ls`
ごとの native `doc --json` in-memory projection を追加した。module 絞り込み、閉世界 schema、引数
fail-closed、artifact と Rust canonical output の一致、malformed native doc の fail-closed、artifact
非作成を確認した。native `lsharp_hover` は source/file、line/character と `col` alias を native
`lsp --stdio` へ渡す scalar signature/doc projection として、malformed frame・child failure・missing
response の fail-closed を確認した verified partial である。
続いて native `lsharp_definition` の source/file/position relay、単一 location の closed `{start,end}`
projection、invalid range/ambiguous location の fail-closed を native MCP 37 tests で確認した verified partial である。
さらに native `lsharp_references` の `includeDeclaration: true` relay、複数 location の closed `{count,ranges}`
projection、native `null` の空集合化、invalid range の fail-closed を native MCP 42 tests で確認した verified partial である。
さらに native `lsharp_completion` の array/CompletionList relay、numeric completion kind の Rust 名称投影、
native `null` の空候補化、invalid item の fail-closed を native MCP 47 tests で確認した verified partial である。
さらに `lsharp_compile_run` の source/file route、native `compile` と外部 `wasmtime` の明示境界、
compile/runtime failure・欠落 artifact・runtime 未設定の fail-closed、stdout の分離、task-owned temporary
source/Wasm の cleanup を native MCP 52 tests で確認した。これは local external-runtime の verified partial
であり、supported-target stage0/packaged runtime parity ではない。package-install semantics、provider
semantics、target runtime、manifest runtime validation の全 surface は [~] のまま残す。ADR:
docs/adr/decisions-v0.3-native-mcp-subset-shim.md。
compile/run boundary の判断は
`docs/adr/decisions-v0.3-native-mcp-compile-run-boundary.md` に記録した。
さらに native `lsharp_format` の出力を `formatted` だけの closed-world schema に固定し、native `fmt`
が stdout を返しても non-zero exit なら stderr/status 付き MCP error とする fail-closed boundary を
native MCP 60 tests で確認した。ADR:
`docs/adr/decisions-v0.3-native-mcp-format-output.md`。これは source text の意味を解釈しない
offline relay の verified partial slice であり、target runtime と Rust/native full parity は残る。
さらに native MCP の `lsharp_hover` / `lsharp_definition` / `lsharp_references` /
`lsharp_completion` で `character` と `col` の oneOf schema を揃え、両方指定した曖昧な入力を
native LSP 実行前に拒否する contract を native MCP 62 tests で確認した。ADR:
`docs/adr/decisions-v0.3-native-mcp-lsp-position-alias.md`。単独 `col` alias は互換性のため維持し、
target runtime と Rust/native full parity は残る。
さらに native MCP `lsharp_check` / `lsharp_format` の `source`/`file` input schema と runtime
preflight を closed-world に揃え、未知引数・source/file 同時指定を native 実行前に拒否する contract を
native MCP 65 tests で確認した。ADR:
`docs/adr/decisions-v0.3-native-mcp-check-format-input.md`。target runtime と Rust/native full parity は残る。
さらに共通 `source` input schema に `minLength: 1` を設定し、native LSP 4 tools、`check`、`validate`、
`format`、`compile_run` の `tools/list` と runtime preflight の非空契約を揃えた。空文字・空白のみの
`source` は native 実行前に拒否し、native MCP 68 tests と Python compile を通過した verified partial
である。ADR: `docs/adr/decisions-v0.3-native-mcp-source-input-schema.md`。target runtime、provider semantics、
Rust/native full parity は残る。
さらに native MCP `lsharp_check` の `migrationDiagnostics` item を postflight 検証し、required field、
code/semantics/disposition enum、range、非負 position、optional message の不正値を native 成功として
返さない contract を追加した。完全な item はそのまま通し、native MCP 69 tests と Python compile を
通過した verified partial である。ADR: `docs/adr/decisions-v0.3-native-mcp-check-output-items.md`。
target runtime、provider semantics、Rust/native full parity は残る。
さらに native MCP `lsharp_validate` の `trace_gaps` / `review_verifications` item を postflight 検証し、
required field、closed field、trace-gap code、review ID pattern、lifecycle state の不正値を native 成功
として返さない contract を追加した。完全な nested report はそのまま通し、native MCP 70 tests と
Python compile を通過した verified partial である。ADR: `docs/adr/decisions-v0.3-native-mcp-validate-output-items.md`。
identity、manifest runtime、target runtime、provider semantics、Rust/native full parity は残る。
さらに native MCP `lsharp_validate` の `review_evidence_identity` report object を postflight 検証し、
required field、unknown field、非 nullable string、nullable provider digest の型境界を揃えた。manifest
input 経由の malformed identity を native 成功として返さず、完全な identity は保持することを native
MCP 72 tests と Python compile で確認した verified partial である。ADR:
`docs/adr/decisions-v0.3-native-mcp-review-identity-output.md`。manifest nested runtime、target runtime、
provider semantics、Rust/native full parity は残る。
さらに native MCP emitted manifest の `edges` を全 relation variant（`motivates`、`constrained-by`、
`tested-by`、`supports`、`contradicts`、`evaluates`、`invalidates`）ごとに postflight 検証し、relation
固有 field、ID、subject kind、unknown field の境界を揃えた。valid edge manifest は保持し、malformed edge
は native 成功として返さないことを native MCP 74 tests と Python compile で確認した verified partial
である。ADR: `docs/adr/decisions-v0.3-native-mcp-manifest-edges.md`。evidence nested、referential integrity、
target runtime、provider semantics、Rust/native full parity は残る。
さらに native MCP emitted manifest の `nodes` / `reviews` item を postflight 検証し、closed field、
required identifier、kind/visibility/state enum、non-empty text、span offset、provenance digest の境界を
揃えた。valid node/review manifest は保持し、malformed item は native 成功として返さないことを native
MCP 73 tests と Python compile で確認した verified partial である。ADR:
`docs/adr/decisions-v0.3-native-mcp-manifest-items.md`。evidence/edges nested runtime、target runtime、
provider semantics、Rust/native full parity は残る。

さらに native MCP emitted manifest の `evidence` item を postflight 検証し、closed field、required
identifier、subject kind、method/outcome/independence enum、execution/sampling、provenance の境界を揃えた。
valid evidence record は保持し、malformed record は native 成功として返さないことを native MCP 75 tests
と Python compile で確認した verified partial である。ADR:
`docs/adr/decisions-v0.3-native-mcp-manifest-evidence.md`。referential integrity、target runtime、provider
semantics、Rust/native full parity は残る。

さらに native MCP emitted manifest の referential integrity を postflight 検証し、node/evidence/explicit
review の duplicate ID、evidence subject、全 edge relation の graph-owned endpoint、node kind mismatch を
fail-closed にした。explicit `reviews` registry がある場合だけ review endpoint を閉じ、contract/change と
省略された review registry は opaque external boundary として保持する。全7 relation variant と opaque
boundary の valid fixture を保持し、malformed closure を拒否することを native MCP 76 tests と Python
compile で確認した verified partial である。ADR:
`docs/adr/decisions-v0.3-native-mcp-manifest-referential-closure.md`。target runtime、provider semantics、
Rust/native full parity は残る。

さらに native MCP emitted manifest の `execution.sampling.coverage` を Rust `SamplingPlan` と同じ invariant
へ揃え、非空 bucket 合計の `cases` 一致、checked u64 overflow 拒否、空/省略 coverage の後方互換を固定した。
canonical、mismatch、overflow、empty coverage の fixture を native MCP 77 tests と Python compile で確認した
verified partial である。ADR: `docs/adr/decisions-v0.3-native-mcp-manifest-sampling.md`。provider semantics、
target runtime、Rust/native full parity は残る。

さらに native MCP `lsharp_validate` report の5つの counter
（`open_questions` / `independent_reviews` / `contradicting_observations` / `stale_reviews` /
`stale_evidence`）を共有 `U64_MAX` 境界まで postflight 検証し、`u64::MAX + 1` を native 成功として
返さない contract を追加した。schema の maximum assertion と overflow fixture を native MCP 77 tests、
Python compile、docs audit、diff check で確認した verified partial である。provider semantics、target runtime、
Rust/native full parity は残る。ADR:
`docs/adr/decisions-v0.3-native-mcp-validate-report-counters.md`。

さらに native MCP shim が直接読む JSON-RPC request、native report、direct/file manifest input、emitted
manifest を strict object-pairs parserで読み、nestedを含む duplicate JSON object key を最後の値へ黙って
上書きせず fail-closedにした。duplicate `id` / `schema_version` / report `status` fixtureを native MCP
78 tests、Python compile、docs audit、diff checkで確認した verified partial である。provider semantics、target
runtime、Rust/native full parity は残る。ADR:
`docs/adr/decisions-v0.3-native-mcp-json-duplicate-keys.md`。

さらに native MCP の LSP response frame と package/stdlib `api.json` artifact も shared strict JSON
decoderへ統合し、duplicate `result` / `package` key を last-value 化せず relay-specific error として
拒否する contract を追加した。LSP、package、stdlib の duplicate fixtureを native MCP 79 tests、Python
compile、docs audit、diff checkで確認した verified partial である。provider semantics、target runtime、
Rust/native full parity は残る。ADR:
`docs/adr/decisions-v0.3-native-mcp-json-relays.md`。

さらに shared strict JSON decoder の `parse_constant` で `NaN` / `Infinity` / `-Infinity` を拒否し、
JSON-RPC、native report、LSP、package/stdlib artifact の非標準 JSON constant を成功値として返さない
boundaryを固定した。native MCP 79 tests、Python compile、docs audit、diff checkで確認した verified partial
である。provider semantics、target runtime、Rust/native full parity は残る。ADR:
`docs/adr/decisions-v0.3-native-mcp-json-constants.md`。

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
  canonical manifest と MCP input/output schema の coverage bucket 名 non-blank 境界、
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
  positional manifest と `--source` の読み込み失敗を driver I/O code `LS5001`、空 stdout、
  no-report として返す Rust CLI boundary、
  Rust MCP `lsharp_validate` の `reviews` input/output schema、input top-level unknown field の
  `additionalProperties: false` boundary、空の manifest/path string を schema で拒否する
  `minLength: 1` boundary、coverage bucket 名の non-blank schema boundary と
  `include_manifest` projection、
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
  exit `0` と同じ deterministic line projection を verified した。
  独立 review gate は `EvidenceOutcome::Pass` かつ `IndependentReview` の evidence だけを数える。
  failed/unknown/stale/contradicted review evidence を独立 review として扱わない Rust canonical
  report boundary を追加し、failed review の complete graph が `unknown` になる RED→GREEN を固定した。
  これは Rust-host canonical report の verified partial sliceであり、selfhost/native parity、MCP、
  current-source artifact/runtime、supported 2 targets は残件である。native source-file smoke の
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

2026-07-31 に Rust MCP `lsharp_validate` の input schema を runtime の empty-string boundary と同期した。
`manifest` の JSON string variant、`file`、`manifest_file`、`trust_store`、`review_lifecycle` に
`minLength: 1` を追加し、runtime が空文字を parse/path error として拒否する routeを static schemaでも
fail-closed にした。空 `source` は既存 semantics（空 programを受理し得る）を保つため変更していない。
`test_validate_tool_input_schema_rejects_empty_manifest_and_path_strings` の Draft 2020-12 validator matrix
と `mcp_server::tests` 70件を通過した。selfhost/native MCP、current-source artifact/runtime、対応2 target、
EC-M2-03 aggregate は残件。ADR: `docs/adr/decisions-v0.2-mcp-validation-manifest.md`。

2026-07-31 に coverage bucket 名の non-blank policy を canonical manifest schema と MCP input/output schemaへ
接続した。runtime の `trim().is_empty()` boundary と同じく、空文字・ASCII空白・NBSP-only の property nameを
Draft 2020-12 `propertyNames.pattern: "\\S"` で rejectする。canonical schema、MCP input、MCP outputの
validator matrixを `test_manifest_schemas_use_draft202012_validator_for_valid_and_invalid_fixtures` に追加し、
既存の valid fixtureを保ったまま3種類の空 bucketを全validatorで拒否することを固定した。Rust-host schema
verified partial sliceであり、selfhost/native manifest parser、current-source artifact/runtime、対応2 target、
EC-M2-02/03 aggregate は残件。ADR: `docs/adr/decisions-v0.2-mcp-validation-manifest.md`。

2026-07-31 に review provenance wire schema の `lifecycle.effective_at` を、Rust
`ReviewLifecycleEvent` が要求する canonical UTC timestamp 定義へ接続した。これまで schema は
`non_empty_string` のみを参照していたため、offset付き・非canonicalな時刻を schema consumer が受理できた。
`review_provenance_schema_requires_canonical_timestamp_for_lifecycle_effective_at` の RED→GREEN と、既存の
`wire_rejects_noncanonical_lifecycle_effective_timestamp` を含む `validation_schema` 8件 / `review_wire` 5件で
lexical schema/parser boundary を固定した。Rust-host schema verified partial sliceであり、暦日の実在性を含む
validator matrix、provider/authentication、selfhost/native producer、対応2 target、EC-M3 aggregate は残件。
ADR: `docs/adr/decisions-v0.3-review-wire-schema-timestamp.md`。

2026-07-31 に MCP `lsharp_validate` input schema の review verification context を runtime の all-or-none
policyへ接続した。`review_subject_digest` / `review_source_commit` / `review_now` の相互依存と、
`review_artifact_digest` 指定時の4 field依存を Draft 2020-12 `dependentRequired` で宣言し、complete contextを
保ったまま5種類の partial inputを `test_validate_tool_input_schema_requires_complete_review_context` で拒否する
validator matrixを追加した。Rust-host MCP schema/runtime verified partial sliceであり、selfhost/native MCP、
current-source artifact/runtime、provider/authentication、対応2 target、EC-M3 aggregate は残件。ADR:
`docs/adr/decisions-v0.3-review-explicit-context.md`。

2026-07-31 に MCP `lsharp_validate` の `review_now` schemaを runtime の canonical UTC timestamp lexical
boundaryへ接続した。`minLength` だけだった input propertyへ `^[0-9]{4}-[0-9]{2}-[0-9]{2}T[0-9]{2}:[0-9]{2}:[0-9]{2}Z$`
patternを追加し、offset付き、空白区切り、fractional seconds、任意文字列を
`test_validate_tool_input_schema_requires_canonical_review_now` の Draft 2020-12 validator matrixで拒否した。
暦日の実在性は既存 Rust canonical timestamp validatorの責務として維持する。Rust-host MCP schema/runtime
verified partial sliceであり、selfhost/native MCP、current-source artifact/runtime、provider/authentication、
対応2 target、EC-M3 aggregateは残件。ADR: `docs/adr/decisions-v0.3-review-explicit-context.md`。

次の実装は `EC-M2-01`〜`EC-M2-03` の未接続入力を一つの RED に絞る。current plan の
acceptance と依存順を確認し、完了 slice の履歴を TODO へ再展開しない。

## Next milestone — v0.3 Review provenance lifecycle

正本: [`v0.3-review-provenance-lifecycle.md`](docs/development/planning/v0.3-review-provenance-lifecycle.md)

- [~] `EC-M3-01` attestation の canonical bytes、strict UTC timestamp、Ed25519 signature、
  current subject/source/provenance binding と explicit report、および
  `reviews[].verification_state` manifest projection を Rust canonical model で検証する。
  attestation input wiring、selfhost/native parity、両 supported target の artifact/runtime evidence
  を閉じる。
- [~] `EC-M3-02` append-only lifecycle を deterministic に reduce し、active sequence、superseded、
  revoked、stale を report の事実へ接続する。Rust/selfhost reducerの sequence/transition/
  `effective_at` rollback（selfhost code `8`）と explicit clock `event_at` 選択は verified partial
  sliceだが、provider snapshot の取得、report projection、native parity は残る。
- [~] `EC-M3-03` CLI/MCP の trust store/lifecycle explicit input と project-root boundary を維持し、
  attestation verification state と no-report/no-manifest の失敗境界を CLI/MCP の共通 projectionへ接続した。
  `--review-subject-digest` / `--review-source-commit` / `--review-now` の all-or-none context、
  expiry/binding の Rust CLI/MCP fixture、malformed clock の no-report contract も verified partial
  とした。selfhost/native parity は残る。ADR:
  `docs/adr/decisions-v0.3-review-explicit-context.md`。
- [~] `EC-M3-04` source と selfhost/native producer の attestation named-field、canonical bytes、
  state、span、exit code の Rust/selfhost 同一 fixture parity は verified partial。JSON report の
  field order、nullable `expires_at`、canonical bytes、span と native source-file smoke の
  report/manifest fixture contract を追加検証した。Mac current-source stage0 producer/package/source-file
  smoke は実行済みで、Linux current-source stage0、packaged artifact provenance、Mac/Linux runtime parityを
  `v0.3-milestone-01.md` の M3-04-N1 で閉じる。
- [~] `EC-M3-05` keyset/lifecycle/source/artifact digest の Rust CLI/MCP/manifest と selfhost identity
  projection、nullable field order、conflict rejection は verified partial。offline release identity
  verifier、native-only archive / packaged stage0 の optional projection、artifact/source mismatch の
  release smoke rejectionを追加した。native text/JSON/MCP と release gate の
  `verified/unverified/stale/revoked/invalid` ordering、provider adapter、両 target runtimeを
  M3-05-N1/N2 で閉じる。official gate の task-owned cleanup path traversal (`.` / `..`) 拒否は
  verified partial として追加したが、actual provider/auth、current-source/packaged runtime、
  rollback/Wasm parity は残る。ADR:
  `docs/adr/decisions-v0.3-native-official-cleanup-path.md`。

この milestone の verified slice は ADR に残すが、項目全体の completion boundary を満たすまで
`[~]` を維持する。次の RED と validation gate は
[`v0.3-milestone-01.md`](docs/development/planning/v0.3-milestone-01.md) の M3-04-N1 に従い、
現在の `lsharp-types` clock contract を暗黙に拡張しない。

## v0.2 Milestone 1 closure

個別 slice の履歴と current boundary は
[Rust 依存境界の縮小](docs/development/operations/rust-boundary-reduction.md) を正本とする。

- [~] `EC-M1-01` Rust/selfhost observable parity — invariant scope、computation/match、
  diagnostics、module/import、qualified/private record の parser/type/runtime slice と
  両 supported target の current-source core stage0 smoke は verified。constructor/record/GADT の
  残る semantics、全 diagnostic/span、standalone source check、full cross-target aggregate を閉じる。
  2026-07-31 に selfhost `Types.TypeInferApply` の 3-64 引数 application を共通の 64 要素
  bounded rooted continuationへ通し、8/64 の成功、65 の fail-closed、failure propagation、既存
  curried applyを合わせた focused E2E 13件を通した。Linux x86_64 fixed point は status `pass`、
  stage2/stage3 code length 各 `11332908`、stdout SHA-256 は両方
  `aa5cee91b5f47dd54a7da64492859bb1b9eede381059051713e85310115ba7ad` で一致した。これは
  TypeInferApply の 0-64 bounded arity と current-source Linux native self-regeneration の
  verified sliceであり、65以上の診断/span契約、全 diagnostic/span、Mac/Linux aggregate parity は
  残る。
  2026-07-31 に selfhost `Types.TypeInferPattern` の constructor/record child pattern inference を
  64 要素 bounded rooted continuationへ集約し、65 要素 cross-chunk contract/runtime 2件、
  match error 10件、quote/pattern 12件を通した。Linux x86_64 fixed point は status `pass`、
  stage2/stage3 code length 各 `11168596`、stdout SHA-256 は両方
  `dad391cd36df64b6354b1f4429aaf7a4c410697b7ca74606fbb2865dc2186bb1` で一致した。これは
  TypeInferPattern child traversal と current-source Linux native self-regeneration の verified
  sliceであり、record schema inference、全 diagnostic/span、Mac/Linux aggregate parity は残る。
- [~] `EC-M1-02` canonical metadata IR — canonical case/assert/property inventory、
  typed binder、precondition/postcondition、directive span の slice は verified。一般 `TypeExpr`、
  全 `ContractSuite` evaluator、binder/predicate 個別 span、formatter/docs、2 target evidence を閉じる。
- [~] `EC-M1-03` form separation and migration — canonical form と legacy migration report の
  text/JSON slice は verified。全 form evaluator、schema、formatter/docs/MCP、2 target evidence を閉じる。
- [~] `EC-M1-04` strict predicate and non-vacuity — Bool preflight、zero-case、
  static reachability/vacuity の slice は verified。動的・compound predicate、全 diagnostic/span、
  evaluator/runtime、2 target aggregate を閉じる。2026-07-31 に selfhost
  `Types.TypeInferAssertions` の string scanner、assertion/case form、program/module/property
  aggregation を 64 要素単位の bounded rooted continuationへ揃え、65要素境界を跨ぐ focused E2E と
  parser diagnostics `0` を確認した。Linux x86_64 fixed point は status `pass`、stage2/stage3
  code length 各 `11491724`、stdout SHA-256 は両方
  `bfff156740a634e25a4fc968ca2a83c9ce62227ed3846d70a3d59658fd6d1d76` で一致した。
  これは TypeInferAssertions の bounded traversal verified sliceであり、全 diagnostic/span、
  evaluator/runtime、Mac/Linux aggregate parityは残る。
- [~] `EC-M1-05` reproducible type-directed sampling — Int/Bool/String の deterministic prefix は
  verified。一般 `TypeExpr`、constraint generator、seed/shrink/coverage、2 target evidence を閉じる。
- [~] `EC-M1-06` structured assurance report — implementation conformance と intent validation を
  混同しない text/JSON report の slice は verified。Rust driver の `test --format json` にも
  canonical `:case`/`:assert` の pass/fail と preflight failure、exit `0/2` を接続した。全 form、
  EmbeddedCli、Rust/selfhost report field differential、provenance、2 target evidence を閉じる。
- [~] `EC-M1-07` native parity and migration closure — current-source native fixed-point と
  source-file smoke は両 target の verified slice を持つ。Rust oracle、standalone Wasm、
  full public surface、guide/schema/MCP/migration docs を同じ observable contract へ揃える。

## V2-16 — Rust dependency boundary reduction

`V2-16a` no-Cargo development loop と `V2-16d` native development E2E は完了履歴へ移動済み。
残る aggregate は次のとおり。

- [~] `LEGACY-LANG-01` record pattern parity — source/ftable の direct/nested pattern、
  nominal marker、field binding は verified。2026-07-31 に `TypeInferRecord.ls` の
  record field/value inference、record literal field lookup、declared literal/update
  inference を 64 要素 bounded/rooted scan へ移行し、65 要素 fixture と既存 record
  computation regression、Linux x86_64 stage2/stage3 fixed-point を確認した。一般 Map API、
  `Type.ls` の `type-record-field-type` / `type-record-fields-eq` も 64 要素 bounded/rooted
  scan へ移行し、65 field lookup/equality fixture、private visibility、record computation
  regression、Linux x86_64 fixed-pointを確認した。
  `TypeInferRecordDecl.ls` の `:only` 判定と accessor filtering も 64 要素 bounded/rooted
  scan へ移行し、65 要素 export filtering fixture と既存 private visibility regression を
  確認した。
  `Syntax.Parser.ls` の name hash、qualified symbol dot、整数桁、string literal hash scanも
  64 要素 bounded/rooted scanへ移行し、65文字境界、正負整数、escape hash、parser formsの
  regression、Linux x86_64 fixed-pointを確認した。
  `vector-set-at` と `defn-signature-param-present-v3` の collection scanも 64 要素
  bounded/rooted continuationへ移行し、tokenizerが生成した64要素超のvectorで setter と
  signature presence の chunk境界を確認した。`selfhost_parser_collection_scanners` の
  3 tests、parser forms 22 tests、metadata forms 29 tests、Linux x86_64 fixed-pointを確認した。
  Parser の structural/recovery scan (`parse-skip-bracket-v3`、`parse-skip-brace-v3`、
  `scan-defn-param-form-end-v3`、delimiter balance、`recover-to-next`) も 64 要素
  bounded/rooted continuationへ移行し、129 token の bracket/brace skip、parameter span end、
  未閉鎖 delimiter code、recovery cursor を `selfhost_parser_structural_scanners` で確認した。
  parser forms 22 tests、metadata forms 29 tests、Linux x86_64 fixed-pointも通過した。
  `collect-example-expression-spans-v3-loop` も 64 要素 bounded/rooted continuationへ移行し、
  129 expression の `(start,end)` 順序と bracket boundary を
  `selfhost_parser_expression_spans` の2 testsで確認した。parser forms 22 tests、metadata forms
  29 tests、Linux x86_64 stage2/stage3 fixed-pointも通過した。
  `parse-defn-meta-case-loop-v3` と `parse-defn-meta-assert-loop-v3` も 64 要素
  bounded/rooted continuationへ移行し、65 case / 65 assert の件数と EOF cursorを
  `selfhost_parser_metadata_scanners` で確認した。metadata scanner 2 tests、metadata forms 29 tests、
  parser forms 22 tests、assertion span 4 tests、case span 5 tests、Linux x86_64 fixed-pointも通過した。
  `parse-source-evidence-shrinks-loop-v3` と `parse-source-evidence-coverage-loop-v3` も item単位の
  step、64要素 bounded loop、rooted continuation、public wrapperへ移行し、65 shrinks / 65 coverage
  entries の件数と EOF cursorを `selfhost_parser_metadata_sequences` で確認した。sequence scanner 2 tests、
  parser filter 114 tests（既知の projection baseline 1件を除く）、evidence registry 59 tests、Linux
  x86_64 fixed-pointも通過した。
  `parse-defn-param-signature-loop-v3` も `[done, next-index, next-signature]` stateを返す step、64要素
  bounded loop、rooted continuation、public wrapperへ移行し、65 typed paramsの先頭/末尾/return typeと
  signature countを `selfhost_parser_signature_scanners` で確認した。signature scanner 2 tests、metadata
  forms 29 tests、parser forms 22 tests、Linux x86_64 fixed-pointも通過した。
  `parse-defn-metadata-loop-v3` の各 directive handler も一 directive の metadata を返す single-step に分離し、
  outer loopを64 directive bounded/rooted continuationへ移行した。65個の `:property []` で metadata form
  件数65とEOF cursorを `selfhost_parser_metadata_outer_scanners` のstatic/runtime 2 testsで確認し、
  `selfhost_parser_` grouped filter 68 tests、Linux x86_64 stage2/stage3 fixed-pointを通過した。summaryは
  `ci-artifacts/native-linux-x86-hostgen-vm/7b7a4f24-parser-metadata-outer/actual-selfregen-summary.json` に保存し、
  stage2/stage3 code lengthは双方 `11297622`、stdout SHA-256は双方
  `6082b4494b46a2244b1d1e84c4faf3824f9b6ff664ca6859785acc8c73240a3c` で一致した。
  evidence/review metadataの field handler と field loopも、各 fieldを一つ処理する step、64 field
  bounded loop、rooted continuation、public wrapperへ移行した。65個の重複 `:subject` / `:subject-digest`
  fieldで evidence/review payload length `82` / `75` と body cursor `10` を
  `selfhost_parser_metadata_fields` のstatic/runtime 2 testsで確認し、`selfhost_parser_` grouped filter
  70 tests、scoped rustfmt、`git diff --check`、Linux x86_64 stage2/stage3 fixed-pointを通過した。summaryは
  `ci-artifacts/native-linux-x86-hostgen-vm/7f1e21d8-parser-metadata-fields/actual-selfregen-summary.json` に保存し、
  stage2/stage3 code lengthは双方 `11314848`、stdout SHA-256は双方
  `63466a74503a2e979f7bb805b8d99f91a848f883ea5c0f4124ac8ffdc0a288a7` で一致した。固定長の
  `source-evidence-seen-new-v3-loop`、required-field check、`source-review-attestation-seen-new-loop` は
  このbatchの対象外として残る。gate後に `origin/main` へ入った差分はMCP専用で、gate時の
  `7f1e21d8` と統合後の対象3ファイルは byte-for-byte 一致したため、同じLinux replayは重複実行していない。
  evidence/review metadataの fixed-size seen vector初期化と evidence required-field 判定も、各要素を一つ処理する
  step、64要素 bounded loop、rooted continuationへ移行した。evidence seen length `17`、空seenの required 判定
  `0`、required fieldを全て埋めた判定 `1`、review seen length `12` を
  `selfhost_parser_metadata_initializers` のstatic/runtime 2 testsで確認し、`selfhost_parser_` grouped filter
  72 tests、scoped rustfmt、`git diff --check`、Linux x86_64 stage2/stage3 fixed-pointを通過した。summaryは
  `ci-artifacts/native-linux-x86-hostgen-vm/0064a5bb-parser-metadata-initializers/actual-selfregen-summary.json` に保存し、
  stage2/stage3 code lengthは双方 `11332908`、stdout SHA-256は双方
  `aa5cee91b5f47dd54a7da64492859bb1b9eede381059051713e85310115ba7ad` で一致した。
  record schema pattern の semantic parity、全 pattern、import target、Rust ABI parity を
  actual E2E で閉じる。既知の `test_e2e_selfhost_typeinfer_record_pattern_uses_declared_field_type`
  の `1` vs `0` は変更前 baseline と同じで残件。
- [~] `LEGACY-LANG-02` ADT/GADT execution parity — ordinary ADT の direct/nested constructor と
  GADT parser/type refinement は verified。2026-07-31 に selfhost
  `Types.TypeInferAdt` の type parameter、constructor field、variant、type declaration scan を
  64 要素単位の bounded rooted continuationへ揃え、65 要素で chunk 境界を跨ぐ focused E2E
  (`selfhost_typeinfer_adt_scanners`) を通した。Linux x86_64 fixed point は status `pass`、
  stage2/stage3 code length 各 `11168596`、stdout SHA-256 は両方
  `dad391cd36df64b6354b1f4429aaf7a4c410697b7ca74606fbb2865dc2186bb1` で一致した。
  これは ordinary ADT の bounded type-inference traversal と current-source Linux native
  self-regeneration の verified sliceであり、nominal/exhaustiveness、full ftable/import、
  linear-memory/WasmGC runtime parity、Mac/Linux aggregate は残る。
- [~] `LEGACY-COMP-01` full-program compiler closure — 主要 CLI builder は full-program 化済み。
  `TypeInferBlock.ls` の大きな do/computation 子要素走査は 64 要素 bounded/rooted scanへ移行し、
  Linux x86_64 stage2/stage3 fixed-pointを確認した。full-program compiler closure、
  diagnostic-only legacy `lower`、no-arg pipeline runtime/native E2E、component sidecar の
  artifact boundary を閉じる。
- [~] `V2-16b` / `LEGACY-IO-01` native artifact I/O — bounded argv/file/raw-byte Preview1 と
  4096 bytes 超 read の slice は verified。`valid/io-read-file` は manifest の明示的な
  UTF-8 runtime input snapshot と task-owned preopen まで Rust oracle/native producer contract
  で固定した。`valid/io-read-file-empty` は zero-byte file と EOF の区別、
  `valid/io-read-file-missing` は明示的な空 directory と missing-path fd error の fail-closed 境界、
  `valid/io-read-stdin` は明示的な UTF-8 stdin snapshot と producer 境界を固定する。
  全 fd error semantics、dynamic root/data/heap layout、component sidecar、target 別
  release artifact を閉じる。
- [~] `V2-16c` / `LEGACY-TOOL-01` public command closure — `install` / `repl` / `lsp --stdio` /
  `doc` / component helper の routing contract は verified。`install` は実 installer helper を
  fake stage0 から public runner 経由で呼び、path dependency、lockfile、module-index、cargo/host
  `lsharp` fallback 不使用まで integration test で確認した。実 stage0 と外部 tool の E2E、
  Rust-only flag/target の明示境界、target 別 release evidence を閉じる。ADR:
  `docs/adr/decisions-v0.3-native-install-runner-e2e.md`。
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

2026-07-29 に version 1 JSON manifest の `reviews[].provenance_digest` Unicode whitespace boundary を追加した。
manifest の opaque review digest を NBSP-only に変異させ、registry 登録前に
`ValidationInputError::Graph(GraphError::InvalidReview)` / field `review_provenance_digest` として拒否する
ことを `review_registry_rejects_unicode_whitespace_only_provenance_digest_in_manifest_input` で固定した。
既存の canonical `str::trim()` policy を manifest review registry へ拡張した Rust verified partial sliceであり、
review lifecycle/authentication、manifest の coverage parity、selfhost/native manifest parser、current-source
artifact/runtime、supported 2 targets、EC-M2-02/EC-M3 aggregate は残件。Evidence:
`docs/adr/decisions-v0.2-validation-manifest-review-unicode-whitespace.md`。

2026-07-29 に version 1 JSON manifest の `sampling.coverage` Unicode whitespace boundary を追加した。
coverage key を NBSP-only に変異させ、canonical `SamplingPlan` の graph 登録前に
`ValidationInputError::Graph(GraphError::InvalidEvidence)` / field `coverage` として拒否することを
`parse_manifest_rejects_unicode_whitespace_only_coverage_bucket_before_registration` で固定した。source
adapter と同じ non-blank policyを manifest inputへ拡張した Rust verified partial sliceであり、coverage
count/cases の意味論、selfhost/native manifest parser、current-source artifact/runtime、supported 2
targets、EC-M2-02/EC-M3 aggregate は残件。Evidence:
`docs/adr/decisions-v0.2-validation-manifest-coverage-unicode-whitespace.md`。

2026-07-29 に version 1 JSON manifest の duplicate review identity boundary を追加した。同一の
`review:checkout/reviewer-001` IDに異なる digest/visibilityを持つ2 recordsを入力し、registry 登録前に
`ValidationInputError::Graph(GraphError::DuplicateReview)` として拒否することを
`review_registry_rejects_duplicate_review_ids_in_manifest_input` で固定した。canonical review registryの
identity policyを manifest inputへ拡張した Rust verified partial sliceであり、review lifecycle/authentication、
other duplicate keys、selfhost/native manifest parser、current-source artifact/runtime、supported 2 targets、
EC-M2-02/EC-M3 aggregate は残件。Evidence:
`docs/adr/decisions-v0.2-validation-manifest-duplicate-review.md`。

2026-07-29 に version 1 JSON manifest の edge payload unknown-field boundary を追加した。tagged
`EdgeInput` の relation variant ごとに許可 field を検査し、`motivates` / `constrained-by` /
`tested-by` / `supports` / `contradicts` / `evaluates` / `invalidates` の未知 field と duplicate
field を graph 登録前の `ValidationInputError::Json` として拒否することを全 variant fixture で固定した。
Rust canonical manifest input の verified partial sliceであり、selfhost/native manifest parser、source
producer、current-source artifact/runtime、supported 2 targets、EC-M2-02/EC-M2-03/EC-M3 aggregate は残件。
Evidence: `docs/adr/decisions-v0.2-validation-manifest-edge-unknown-fields.md`。

2026-07-29 に明示 `reviews: []` の review registry presence boundary を追加した。manifest の
省略 (`None`) と empty (`Some([])`) を区別し、空でも `evaluates` / `invalidates` の未登録 review を
`GraphError::MissingReview` として拒否する。canonical output は明示 empty registry を `reviews: []` として
保持し、parse/emit/parse で closure policy を失わない Rust verified partial sliceである。review lifecycle/
authentication、selfhost/native manifest parser、current-source artifact/runtime、supported 2 targets、
EC-M2-02/EC-M2-03/EC-M3 aggregate は残件。Evidence:
`docs/adr/decisions-v0.2-validation-manifest-explicit-empty-review-registry.md`。

2026-07-29 に version 1 manifest の明示 `reviews: null` boundary を追加した。省略 (`None`) と
配列 (`Some`) だけを受理し、schema 外の null が registry なしへ変換されないよう custom deserializer で
`ValidationInputError::Json` にする Rust canonical input verified partial sliceである。selfhost/native
manifest parser、CLI/MCP parity、current-source artifact/runtime、supported 2 targets、review lifecycle/
authentication、EC-M2-02/EC-M2-03/EC-M3 aggregate は残件。Evidence:
`docs/adr/decisions-v0.2-validation-manifest-null-review-registry.md`。

2026-07-29 に MCP `lsharp_validate` の manifest input schema を version 1 envelope と同期した。
`tools/list` が `schema_version` / `nodes` / `evidence` / `edges` を required、`reviews` を optional として
公開し、unknown top-level field を schema 上で拒否することを `mcp_server::tests` 41件で固定した。
Rust parser と MCP input/output schema の必須境界を揃えた verified partial sliceであり、selfhost/native
MCP、current-source artifact/runtime、supported 2 targets、EC-M2-03/EC-M3 aggregate は残件。Evidence:
`docs/adr/decisions-v0.2-mcp-validation-manifest.md`。

2026-07-29 に EC-M2-02/03 の evidence coverage count invariant を追加した。`coverage` を宣言した
evidence は bucket count の合計を `cases` と完全一致させ、checked-add overflow と不一致を canonical
`SamplingPlan`、Rust source adapter（directive/form span 付き）、version 1 manifest input、公開
`lsharp validate` の全入力境界で fail-closed にする。coverage を省略する既存 observational evidence
は互換性のため受理する。canonical partition、不一致、overflow、source span、manifest/CLI の exit `1`・
空 stdout・manifestなしを focused test で固定した Rust-host verified partial sliceである。coverage の
実行生成/generator policy、MCP/selfhost/native parity、current-source artifact/runtime、Mac/Linux matrix、
EC-M2-02/03 aggregate は残件。Evidence: `docs/adr/decisions-v0.2-validation-evidence-coverage-count.md`。

さらに同じ coverage count invariant を Rust MCP `lsharp_validate` の `manifest` object、JSON string、
`manifest_file` routeへ接続した。`cases=3` に対する `coverage.smoke=2` を direct JSON-RPC に入力すると、
3 routeすべてが `isError: true`、`structuredContent` なし、coverage/cases/covered 付き text error を返す
ことを固定した。さらに source/file route も `sum=1,cases=2` の source adapter diagnostic を同じ
`isError` / no-structured-content boundaryへ投影することを固定した。Rust-host MCP 全 input-route verified
partial sliceであり、selfhost/native MCP producer、current-source artifact/runtime、Mac/Linux matrix、
EC-M2-02/03 aggregate は残件。
Evidence: `docs/adr/decisions-v0.2-validation-evidence-coverage-count.md`。

2026-07-29 に MCP `lsharp_validate` でも review registry の presence semantics を対照固定した。
`reviews` 省略時の未登録 `evaluates` edge は opaque endpoint として `status: unknown` を返し、明示
`reviews: []` の同じ edge は review ID error として拒否することを review registry 6件・MCP suite 43件で確認した。
CLI と MCP の parser policy を揃えた verified partial sliceであり、selfhost/native MCP、current-source
artifact/runtime、supported 2 targets、EC-M2-03/EC-M3 aggregate は残件。Evidence:
`docs/adr/decisions-v0.2-mcp-validation-manifest.md`。

2026-07-29 に review registry の presence semantics を公開 `lsharp validate` で対照固定した。
`reviews: []` の未登録 `evaluates` edge は exit `1`・空 stdout・manifest なしで拒否し、`reviews` 省略時の
同じ edge は opaque endpoint として `status: unknown`・exit `2` を返す。selfhost/native parser、MCP、
current-source artifact/runtime、supported 2 targets、EC-M2-02/EC-M3 aggregate は残件。Evidence:
`docs/adr/decisions-v0.2-validation-manifest-explicit-empty-review-registry.md`。

2026-07-29 に明示 `reviews: []` の unregistered review edge closure を公開 `lsharp validate` へ接続した。
未登録 `evaluates` review は exit `1`、空 stdout、manifest file なし、review/missing/identity 診断の
diagnostic-only 結果となることを `manifest_input_cli` と既存 CLI の全30テストで固定した。review lifecycle/
authentication、selfhost/native parser、MCP、current-source artifact/runtime、supported 2 targets、
EC-M2-02/EC-M3 aggregate は残件。Evidence:
`docs/adr/decisions-v0.2-validation-manifest-explicit-empty-review-registry.md`。

2026-07-29 に公開 `lsharp validate` の manifest input matrix を拡張した。duplicate review identity は
exit `1`、空 stdout、manifest file なし、review ID/重複診断となり、unknown edge field は同じ
diagnostic-only boundary で unknown field/edge 診断を返すことを新規 `manifest_input_cli` test で固定した。
selfhost/native parser、MCP、current-source artifact/runtime、supported 2 targets、EC-M2-02/EC-M3 aggregate
は残件。Evidence: `docs/adr/decisions-v0.2-validation-manifest-duplicate-review.md`、
`docs/adr/decisions-v0.2-validation-manifest-edge-unknown-fields.md`。

2026-07-29 に `sampling.coverage` duplicate bucket key の wire boundary を公開 `lsharp validate` へ接続した。
同一 `coverage` key を持つ manifest は exit `1`、空 stdout、manifest file なし、`coverage` と duplicate-key
分類を含む stderr の diagnostic-only 結果となることを Rust driver CLI 全27件で固定した。selfhost/native
manifest parity、report/atomic writer、current-source artifact/runtime、supported 2 targets、EC-M2-03
aggregate は残件。Evidence: `docs/adr/decisions-v0.2-validation-manifest-duplicate-coverage.md`。

2026-07-29 に version 1 manifest の subject kind schema parity を追加した。evidence、`evaluates`、
`invalidates` の subject を relation ごとの JSON Schema 定義へ分離し、Rust parser と同じ
`intent`/`claim`/`contract`、`intent`/`claim`/`evidence`、`evidence`/`review` の kind enum を固定した。
これは schema consumer が Rust parser より広い subject を受理しないための static verified partial sliceで
あり、JSON Schema 実 validator、selfhost/native manifest parser、current-source artifact/runtime、
supported 2 targets、EC-M2-02/EC-M3 aggregate は残件。Evidence:
`docs/adr/decisions-v0.2-validation-manifest-typed-subject-schema.md`。

2026-07-29 に version 1 manifest の typed subject 診断を追加した。`evaluates.subject` / `invalidates.subject`
の不正 kind を、欠落 node と同じ `MissingNodeReference` にせず relation/kind/stable ID を保持する
`ValidationInputError::InvalidSubjectKind` として fail-closed にする Rust canonical parser verified partial
sliceである。source/native diagnostic parity、selfhost/native manifest parser、CLI/MCP、current-source
artifact/runtime、supported 2 targets、EC-M2-02/EC-M3 aggregate は残件。Evidence:
`docs/adr/decisions-v0.2-validation-manifest-subject-kind-diagnostic.md`。

2026-07-29 に `lsharp validate <manifest> --format json` の typed subject input error boundary を追加した。
`evaluates.subject` / `invalidates.subject` の `InvalidSubjectKind` は exit `1`、空 stdout、manifest file なし、
relation path/kind/stable ID を含む stderr の diagnostic-only 結果となり、report JSON と混ざらないことを
Rust driver CLI test で固定した。default EmbeddedCli build
は origin/main の既存 selfhost source に残る `vector-push-single-rooted-v3` 未定義で停止したため、
既存 component artifact を指定した focused test で manifest input 経路だけを検証した。source/native CLI
の同一診断、MCP、current-source artifact/runtime、supported 2 targets、EC-M2-02/EC-M3 aggregate は残件。
Evidence: `docs/adr/decisions-v0.2-validation-cli-subject-kind-diagnostic.md`。

2026-07-29 に `reviews: null` の optional registry type boundary を公開 `lsharp validate` へ接続した。
manifest input error は exit `1`、空 stdout、manifest file なし、`reviews` / `null` を含む stderr の
diagnostic-only 結果となることを Rust driver CLI 全26件で固定した。selfhost/native parser、MCP parity、
current-source artifact/runtime、supported 2 targets、EC-M2-02/EC-M3 aggregate は残件。Evidence:
`docs/adr/decisions-v0.2-validation-manifest-null-review-registry.md`。

2026-07-29 に EC-M3-01 version 1 manifest の unsigned numeric overflow boundary を追加した。
`u64::MAX + 1` を span と sampling の全 unsigned fieldへ入力し、Rust canonical parser が graph 構築前に
`ValidationInputError::Json` として fail-closed にすることを
`parse_manifest_rejects_unsigned_numeric_overflow` で固定した。既存の `usize` / `u64` typed serde
decode により production code の変更は不要だった。selfhost/native manifest parser、source producer、
current-source stage0 artifact/runtime、Mac/Linux matrix、EC-M2-02/EC-M3 aggregate は残件。Evidence:
`docs/adr/decisions-v0.2-validation-input-numeric-boundaries.md`。

2026-07-29 に EC-M3-01 version 1 manifest の fractional unsigned numeric boundary を追加した。
`0.5` / `1.5` を span と sampling の全 unsigned fieldへ入力し、Rust canonical parser が graph 構築前に
`ValidationInputError::Json` として reject することを
`parse_manifest_rejects_fractional_unsigned_numeric_fields` で固定した。typed serde decode に委譲し、
小数の丸め・切り捨てによる sampling semantics の変質を許可しない。selfhost/native manifest parser、
source producer、current-source stage0 artifact/runtime、Mac/Linux matrix、EC-M2-02/EC-M3 aggregate は残件。
Evidence: `docs/adr/decisions-v0.2-validation-input-numeric-boundaries.md`。

さらに公開 `lsharp validate <manifest> --format json --emit-manifest` へ fractional unsigned numeric
boundary を接続した。span と sampling の全6 fieldで exit `1`、空 stdout、manifest file 未生成、
`manifest` / `floating point` を含む stderr を固定した。default EmbeddedCli は既存 selfhost source の
`vector-push-single-rooted-v3` 未定義で停止するため、既存 component artifact を指定した Rust driver CLI
laneで検証した。selfhost/native CLI、MCP、current-source stage0 artifact/runtime、Mac/Linux matrix、
EC-M2-03/EC-M3 aggregate は残件。Evidence: `docs/adr/decisions-v0.2-validation-input-numeric-boundaries.md`。

同じ公開 CLI laneで `u64::MAX + 1` の unsigned numeric overflow も span / sampling の全6 fieldに対して
exit `1`、空 stdout、manifest file 未生成、manifest input error の stderr となることを固定した。
default EmbeddedCli blockerを避けた既存 component artifact 指定の Rust driver CLI evidenceであり、
selfhost/native CLI、MCP、current-source stage0 artifact/runtime、Mac/Linux matrix、EC-M2-03/EC-M3 aggregate
は残件。Evidence: `docs/adr/decisions-v0.2-validation-input-numeric-boundaries.md`。

2026-07-29 に version 1 manifest の unsigned numeric `null` boundary を追加した。span / sampling の全6
fieldで `null` を 0 や省略へ変換せず、Rust canonical parser が graph 構築前に
`ValidationInputError::Json` として拒否することを `parse_manifest_rejects_null_unsigned_numeric_fields`
で固定した。selfhost/native manifest parser、公開 CLI/MCP、current-source stage0 artifact/runtime、
Mac/Linux matrix、EC-M2-02/EC-M3 aggregate は残件。Evidence:
`docs/adr/decisions-v0.2-validation-input-numeric-boundaries.md`。

2026-07-29 に MCP `lsharp_validate` の manifest schema へ unsigned numeric boundary を反映した。
`tools/list` が `nodes[].span.start/end` と `evidence[].execution.sampling.cases/seed/shrinks[]/coverage.*`
を `type: integer`・`minimum: 0` として公開し、input/output で同じ schema helper を共有することを
新規 schema test と MCP suite 44件で固定した。これは static schema と Rust MCP の verified partial
sliceであり、JSON Schema 実 validator、selfhost/native MCP、current-source stage0 artifact/runtime、
Mac/Linux matrix、EC-M3 aggregate は未完了である。Evidence:
`docs/adr/decisions-v0.2-mcp-validation-manifest.md`。

2026-07-29 に EC-M3-01 の MCP manifest runtime numeric boundary を追加した。direct `manifest` string
input の span / sampling 全6 fieldで fractional、`null`、`u64::MAX + 1` を `isError: true` の
`validation manifest の parse` error として拒否し、report/canonical manifestを成功値として返さない
18ケースを `mcp_server::tests` 45件で固定した。既存 Rust typed serde parser を MCP toolまで接続した
verified partial sliceであり、selfhost/native MCP、current-source stage0 artifact/runtime、Mac/Linux
matrix、JSON Schema実 validator、EC-M3 aggregate は未完了である。Evidence:
`docs/adr/decisions-v0.2-mcp-validation-manifest.md`。

2026-07-29 に EC-M3-01 の MCP typed edge schema boundary を追加した。`tools/list` の manifest
`edges[]` が公開 intent-graph schema と同じ6 relation variantを `oneOf` で宣言し、stable IDの
namespace/key pattern と evidence/review/invalidation subject kind enum を input/output 共通 schema
へ反映することを新規 parity test と MCP suite 46件で固定した。static schema と Rust MCP の verified
partial sliceであり、JSON Schema 実 validator、selfhost/native MCP、current-source stage0 artifact/runtime、
Mac/Linux matrix、EC-M3 aggregate は未完了である。Evidence:
`docs/adr/decisions-v0.2-mcp-validation-manifest.md`。

2026-07-29 に EC-M3-01 の MCP `manifest_file` runtime numeric boundary を追加した。file input の
`span.start/end` と `sampling.cases/seed/shrinks[]/coverage.*` 全6 fieldで fractional、`null`、
`u64::MAX + 1` を `isError: true` の `validation manifest の parse` error として拒否し、report/
canonical manifestを返さない18ケースを `mcp_server::tests` 47件で固定した。既存 Rust typed serde
parser を `manifest_file` routeまで接続した verified partial sliceであり、selfhost/native MCP、
current-source stage0 artifact/runtime、JSON Schema実 validator、Mac/Linux matrix、EC-M3 aggregate は
未完了である。Evidence: `docs/adr/decisions-v0.2-mcp-validation-manifest.md`。

2026-07-29 に EC-M3-01 の JSON Schema 実 validator boundary を追加した。canonical intent graph schema と
MCP `lsharp_validate` input/output schemaを Draft 2020-12 validatorで実行し、canonical fixtureの valid
roundtrip、fractional、`null`、`u64::MAX + 1`、typed subject kind mismatch の4 rejectを固定した。
Rust `u64` と schemaの整数上限を揃えるため、unsigned 6 fieldへ `maximum: 18446744073709551615` を追加した。
`mcp_server::tests` 48件で verifiedした Rust-host schema sliceであり、selfhost/native MCP、current-source
stage0 artifact/runtime、Mac/Linux matrix、EC-M3 aggregate は未完了である。Evidence:
`docs/adr/decisions-v0.2-mcp-validation-manifest.md`。

2026-07-29 に EC-M3-01 の MCP validation report schema boundary を追加した。`intent-validation.schema.json`
の external `$ref` を canonical intent graph schema resource として解決し、実際の
`lsharp_validate` `include_manifest: true` output（report と inline manifest）を Draft 2020-12 validator
で検証する valid roundtrip と、未知 `status` の reject を固定した。MCP suite 49件、clippy、rustfmt、
diff check、docs auditを通過した Rust-host verified partial sliceであり、native/selfhost MCP producer、
current-source stage0 artifact/runtime、Mac/Linux matrix、EC-M3 aggregate は残件である。Evidence:
`docs/adr/decisions-v0.2-mcp-validation-manifest.md`。

2026-07-29 に EC-M3-01 の MCP output schema parity boundary を追加した。canonical report schema と
`tools/list` の output schema を同期し、unknown top-level field、trace gap の未知 code / 空 subject ID、
64-bit counter 上限超過を両 validator が reject することを新規 matrix で固定した。report counter 5項目の
`maximum`、trace gap の code enum / non-empty ID、strict object boundary を canonical/MCP に反映し、MCP
suite 50件、clippy、rustfmt、diff check、docs auditを通過した Rust-host verified partial sliceである。
selfhost/native MCP producer、current-source stage0 artifact/runtime、Mac/Linux matrix、EC-M3 aggregate は
残件。Evidence: `docs/adr/decisions-v0.2-mcp-validation-manifest.md`。

2026-07-29 に EC-M3-01 の MCP provenance schema parity boundary を追加した。canonical manifest が要求する
evidence provenance の `producer` / `tool_version` / `timestamp` を MCP input/output schema でも
`minLength: 1` とし、3 fields の空文字を canonical/MCP 両 validator が reject する matrix を固定した。
MCP suite 50件、clippy、rustfmt、diff check、docs auditを通過した Rust-host verified partial sliceであり、
selfhost/native MCP producer、current-source stage0 artifact/runtime、Mac/Linux matrix、EC-M3 aggregate は
残件。Evidence: `docs/adr/decisions-v0.2-mcp-validation-manifest.md`。

2026-07-29 に EC-M3-01 の MCP provenance runtime boundary を追加した。`producer` / `tool_version` /
`timestamp` を空文字へ変異させた canonical fixtureを direct `manifest` と `manifest_file` の両 routeへ入力し、
6ケースすべてで `isError: true`、`structuredContent` なし、field名付き text error の fail-closed を固定した。
focused `review_registry_tests` 14件、clippy、rustfmt、diff check、docs auditを通過した Rust-host verified
partial sliceであり、全 MCP suite は既存 compile-run artifact `Invalid argument (os error 22)` 1件を含むため
今回の gateから除外した。selfhost/native MCP、current-source stage0 artifact/runtime、Mac/Linux matrix、
EC-M3 aggregate は残件。Evidence: `docs/adr/decisions-v0.2-mcp-validation-manifest.md`。

2026-07-29 に MCP `lsharp_compile_run` の共有 temp directory race を閉じた。固定名の directory を呼び出しごとに
削除していたため、別 process / test の同時実行で `Main.ls` / `Main.wasm` が相互に消え、`[LS5001] Invalid argument
(os error 22)` になり得た。呼び出しごとに PID・時刻・sequence を含む専用 directory を作り、RAII cleanupで
成功・失敗を問わず回収する。unique path / cleanup の RED→GREEN と実 MCP compile/run を含む `mcp_server::tests`
52件、clippy、rustfmt、diff check、docs auditを通過した Rust-host verified partial sliceである。selfhost/native
MCP、current-source artifact/runtime、Mac/Linux matrix、EC-M3 aggregate は残件。Evidence:
`docs/adr/decisions-v0.2-mcp-validation-manifest.md`。

2026-07-29 に version 1 manifest envelope の duplicate top-level key boundary を追加した。
`schema_version` / `nodes` / `reviews` / `evidence` / `edges` の同名 JSON key を最後の値や空配列へ
上書きせず、parser 入口の `ValidationInputError::Json` として拒否する5ケースを
`parse_manifest_rejects_duplicate_top_level_fields` で固定した。既存 serde decode の fail-closed
behaviorを回帰テストと ADR に昇格した Rust canonical verified partial sliceであり、production codeの
変更はない。selfhost/native manifest parser、CLI/MCP parity、current-source artifact/runtime、Mac/Linux
matrix、EC-M2/EC-M3 aggregate は残件。Evidence:
`docs/adr/decisions-v0.2-validation-manifest-duplicate-top-level.md`。

2026-07-29 に同じ duplicate top-level key boundary を公開 `lsharp validate` へ接続した。
`manifest_input_cli` の fixture が exit `1`、空 stdout、`--emit-manifest` file 未生成、stderr の
`duplicate` / `schema_version` 診断を返すことを固定した。Rust-host CLI verified partial sliceであり、
selfhost/native manifest parser、MCP report/exit parity、current-source stage0 artifact/runtime、
Mac/Linux matrix、EC-M2/EC-M3 aggregate は残件。Evidence:
`docs/adr/decisions-v0.2-validation-manifest-duplicate-top-level.md`。

2026-07-29 に同じ duplicate top-level key boundary を Rust MCP `lsharp_validate` へ接続した。
`schema_version` の duplicate JSON string を direct `manifest` と `manifest_file` の両 routeへ入力し、
`isError: true`、`structuredContent` なし、`validation manifest の parse` / `duplicate` /
`schema_version` を含む text errorを2ケースで固定した。Rust-host MCP verified partial sliceであり、
selfhost/native MCP producer、current-source stage0 artifact/runtime、Mac/Linux matrix、EC-M3 aggregate は
残件。Evidence: `docs/adr/decisions-v0.2-mcp-validation-manifest.md`。

2026-07-29 に project config 由来の manifest path safety を公開 `lsharp validate` へ接続した。
`[validation].manifest` の absolute path と project root外を指す symlink を、`..` と同じく
non-zero exit・空 stdout・path boundary診断で拒否する2ケースを追加し、`validate_cli` 全29件で
root/nested discovery、missing config、relative/absolute/traversal/symlink boundaryを固定した。
Rust-host CLI verified partial sliceであり、selfhost/native、current-source stage0 artifact/runtime、
Mac/Linux matrix、EC-M2-03 aggregate は残件。Evidence: `docs/adr/decisions-v0.2-validation-config.md`。

2026-07-29 に project config manifest path safety の残るCLI境界も接続した。empty path、missing file、
directory targetを report生成前に non-zero exit・空 stdout・個別診断で拒否する3ケースを追加し、
`validate_cli` 全32件で project-relative、`..`、absolute、empty、missing、regular-file、外部symlinkを
固定した。Rust-host CLI verified partial sliceであり、selfhost/native、current-source stage0 artifact/runtime、
Mac/Linux matrix、EC-M2-03 aggregate は残件。Evidence: `docs/adr/decisions-v0.2-validation-config.md`。

2026-07-29 に MCP `lsharp_validate` の manifest unknown top-level field runtime boundaryを追加した。
`unexpected` fieldを direct object、JSON string、`manifest_file` の3 routeへ入力し、`isError: true`、
`structuredContent` なし、parse errorとfield名を返す fail-closed contractを固定した。MCP
`review_registry_tests` 16件のRust-host verified partial sliceであり、selfhost/native MCP producer、
current-source stage0 artifact/runtime、Mac/Linux matrix、EC-M3 aggregate は残件。Evidence:
`docs/adr/decisions-v0.2-mcp-validation-manifest.md`。

2026-07-29 に failed independent review の complete graph source fixture を native smoke に追加した。
`method=review`、`outcome=fail`、`independence=independent-review` は JSON/text とも `status: unknown`、
exit `2`、`independent_reviews: 0`、stderr 空となることを固定した。Rust canonical の
RED→GREEN を native source-file contract へ接続する fake Lima/provenance harness の verified partial
sliceであり、current source-commit に一致する実 stage0 artifact/runtime、selfhost/native/MCP parity、
Mac/Linux matrix、EC-M2-03 aggregate は残件。Evidence:
`docs/adr/decisions-v0.2-native-validation-failed-independent-review.md`。

2026-07-29 に native source-file smoke の manifest roundtrip boundary を追加した。source fixtureから
`--emit-manifest` した version 1 manifestを positional `validate <manifest> --format json` へ再入力し、
source report と manifest report の byte-for-byte 一致、`unknown` / exit `2`、stderr 空を要求する。
これは Rust CLI の source/emitted-manifest parity を native contract へ接続する fake Lima/provenance
harness の verified partial sliceであり、current source-commit に一致する実 stage0 artifact/runtime、
selfhost/native/MCP parity、Mac/Linux matrix、EC-M2-03 aggregate は残件。Evidence:
`docs/adr/decisions-v0.2-native-validation-manifest-roundtrip.md`。Rust oracle focused test は
default EmbeddedCli build の既存 `vector-push-single-rooted-v3` 未定義で停止し、GREEN evidence には
算入していない。

2026-07-29 に `selfhost/src/Tools/Validation/Stale.ls` の rooted vector helper owner importを修正した。
`Syntax.Parser` の直接 import を追加し、default EmbeddedCli build が `vector-push-single-rooted-v3`
undefinedで停止する既存 blockerを解消した。RED→GREEN の selfhost stale validation test、Rust driver
の `validate_cli` 32件、`validate_review_registry` 2件、`manifest_input_cli` 8件が passした verified
sliceである。native stage0 current-source/runtime、MCP、Mac/Linux matrix、EC-M2-03 aggregate は残件。
Evidence: `docs/adr/decisions-v0.2-selfhost-stale-parser-import.md`。
