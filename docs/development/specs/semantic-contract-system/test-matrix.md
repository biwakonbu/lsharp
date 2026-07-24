# Semantic Contract System 受入試験マトリクス

状態: 規範的な実装受入条件
architecture: [`README.md`](./README.md)
実装順序: [`implementation-plan.md`](./implementation-plan.md)

## 1. 読み方

各 test ID は、行に将来 version と明記したものを除き mandatory である。未実装、ignored、skipped、flaky、
fallback 経由の pass が一つでもあれば、対応 work package は完了ではない。

layer:

| code | layer |
|---|---|
| `SYN` | parser / AST |
| `TYP` | Rust type / semantic checker |
| `RUN` | Rust canonical contract runner |
| `DRV` | Rust driver / CLI E2E |
| `DOC` | generated specification / ontology |
| `LSP` | LSP adapter |
| `MCP` | MCP adapter |
| `NAT` | self-host / native stage0 |
| `MAC` | Mac Apple Silicon native gate |
| `LNX` | Linux x86_64 native / VM gate |

expected outcome:

- `PASS`: fallback なしの explicit success。
- `ERR(code)`: stable diagnostic code と relevant span を持つ explicit error。
- `OPEN(rule)`: `accepted=false` で指定 obligation が open。
- `CLOSED(rule)`: output に示した evidence ID により指定 obligation が closed。
- `BYTE-EQUAL`: canonical bytes が一致。
- `STRUCT-EQUAL`: decode 後の typed response structure が一致。

## 2. Fixture catalog

`tests/fixtures/semantic/v1/` に次を作る。

| fixture | 目的 |
|---|---|
| `bank_checked.ls` | 完全な `Bank.remaining-balance` |
| `bank_body_bug.ls` | contract は同じで body だけ誤る |
| `bank_missing_doc.ls` | checked public function に `:doc` がない |
| `bank_missing_contract.ls` | owner coverage がない |
| `bank_unrelated_case.ls` | case が owner を呼ばない |
| `bank_dead_reference.ls` | static graph には owner があるが実行 path では呼ばない |
| `bank_effectful_property.ls` | nondeterministic / unsupported capability を要求する property |
| `bank_helper_base.ls` | public owner が private helper を呼ぶ baseline |
| `bank_helper_changed.ls` | private helper body だけを変更 |
| `bank_property_arity.ls` | property binder arity が owner と異なる |
| `bank_property_type.ls` | binder type が owner parameter と異なる |
| `bank_property_result.ls` | correctly typed `result` が必要な postcondition |
| `bank_property_unsupported.ls` | unsupported generator type |
| `bank_property_filter.ls` | filter が attempt limit を超える |
| `bank_assert_scope.ls` | assertion が function parameter を不正参照 |
| `bank_vacuous.ls` | empty / tautological contract |
| `percentage.ls` | range predicate を持つ constrained type |
| `percentage_changed.ls` | upper bound を 100 から 200 へ変更 |
| `order_typestate.ls` | `Order Draft -> Submitted -> Accepted` |
| `legacy_metadata.ls` | `:params`、`:returns`、`:invariant`、`:transitions` |
| `presentation_only.ls` | `:example`、`:since`、`:see-also` のみ |
| `semantic_format_a.ls` | canonical source A |
| `semantic_format_b.ls` | A の formatting / comment-only variant |
| `semantic_alpha_a.ls` | binder naming A |
| `semantic_alpha_b.ls` | A の alpha-renamed equivalent |
| `public_add_base.ls` | public symbol 追加前 |
| `public_add_current.ls` | public symbol 追加後 |
| `public_remove_base.ls` | public symbol 削除前 |
| `public_remove_current.ls` | public symbol 削除後 |
| `record_base.ls` | public record baseline |
| `record_field_changed.ls` | field の追加・削除・型変更 |
| `adt_base.ls` | public ADT baseline |
| `adt_variant_changed.ls` | variant の追加・削除 |
| `intent_base.ls` | initial `:doc` / `:rationale` |
| `intent_changed.ls` | intent だけ変更 |
| `checked_contract_changed.ls` | checked public function の contract と behavior を変更 |
| `presentation_changed.ls` | presentation だけ変更 |
| `private_body_changed.ls` | private implementation だけ変更 |
| `unknown_schema.json` | unsupported snapshot schema |
| `malformed_snapshot.json` | malformed / duplicate field |
| `base_snapshot.json` | trusted baseline vector |
| `current_snapshot.json` | current snapshot vector |
| `stale_evidence.json` | old fingerprint に bind した evidence |
| `agent_attestation.json` | current checked delta に bind した agent reconciliation |
| `human_attestation.json` | trusted human signature |
| `untrusted_human_attestation.json` | untrusted key の human signature |
| `fallback_evidence.json` | `fallback_used=true` の passed result |
| `migration_note.md` | non-empty reviewed migration text |
| `ontology_expected.json` | canonical read-only graph |
| `spec_expected.md` | generated symbol specification |

source fixture は規範言語文書の syntax をそのまま使う。owner binding や policy を迂回する簡略 syntax に
置き換えない。

## 3. Parser / source-form

| ID | layer | input | expected |
|---|---|---|---|
| `SCS-SYN-001` | SYN,NAT | complete `:case` | ordered `Case` form と span を保持 |
| `SCS-SYN-002` | SYN,NAT | complete `:assert` | ordered assertion form と span を保持 |
| `SCS-SYN-003` | SYN,NAT | complete `:property` | binder / option / source order を保持 |
| `SCS-SYN-004` | SYN,NAT | property option 省略 | AST は省略を保持し canonicalization が default 適用 |
| `SCS-SYN-005` | SYN,NAT | legacy form | aggregate field と ordered form を lossless に保持 |
| `SCS-SYN-006` | SYN,NAT | postcondition なし | closing form 位置で parser error |
| `SCS-SYN-007` | SYN,NAT | negative cases / seed | exact span の parser error |
| `SCS-SYN-008` | SYN,NAT | invalid property option | accepted option を示す parser error |
| `SCS-SYN-009` | SYN,NAT | typed parameter / return | AST と round-trip representation に型を保持 |
| `SCS-SYN-010` | SYN,NAT | GADT typestate | variant return type を保持 |

## 4. Canonical contract semantics

| ID | layer | input | expected |
|---|---|---|---|
| `SCS-CON-001` | TYP,NAT | `bank_checked.ls` | 全 canonical form well-formed |
| `SCS-CON-002` | TYP,NAT | empty `:case` | error。0 test success を禁止 |
| `SCS-CON-003` | TYP,NAT | actual / expected type mismatch | expected expression の error |
| `SCS-CON-004` | TYP,RUN,NAT | comparable ADT case | structural comparison `PASS` |
| `SCS-CON-005` | TYP,NAT | function-valued case | `ERR(LS3235)` |
| `SCS-CON-006` | TYP,NAT | empty `:assert` | error。0 assertion success を禁止 |
| `SCS-CON-007` | TYP,NAT | non-Bool assertion | predicate の type error |
| `SCS-CON-008` | TYP,NAT | `bank_assert_scope.ls` | implicit parameter binding なし |
| `SCS-CON-009` | TYP,NAT | static tautology | vacuity error |
| `SCS-CON-010` | TYP,NAT | static false precondition | unreachable / vacuity error |
| `SCS-CON-011` | TYP,NAT | zero property cases | error |
| `SCS-CON-012` | TYP,NAT | duplicate binder | duplicate 位置の error |
| `SCS-CON-013` | TYP,NAT | binder `result` | reserved-name error |
| `SCS-CON-014` | TYP,NAT | arity mismatch | `ERR(LS3234)` |
| `SCS-CON-015` | TYP,NAT | binder type mismatch | `ERR(LS3234)` と両 type |
| `SCS-CON-016` | TYP,NAT | non-Bool postcondition | postcondition の type error |
| `SCS-CON-017` | TYP,NAT | Bool return を Int として使う | owner return type により reject |
| `SCS-CON-018` | TYP,RUN,NAT | `bank_property_result.ls` | owner を呼び実 result を check |
| `SCS-CON-019` | RUN,NAT | runner が owner を 2 回呼ぶ mutation | exactly-once 違反を検出 |
| `SCS-CON-020` | TYP,NAT | non-function owner の canonical contract | `ERR(LS3236)` |
| `SCS-CON-021` | TYP,NAT | closed constant evaluator 外の expression | `Unknown` とし tautology に誤分類しない |
| `SCS-CON-022` | TYP,RUN,NAT | unsupported/nondeterministic capability | `ERR(LS3237)`、実行/fallback なし |

## 5. Owner coverage

| ID | layer | scenario | expected |
|---|---|---|---|
| `SCS-COV-001` | TYP,RUN,NAT | case actual が owner を直接 call | passing dynamic trace で case coverage = 1 |
| `SCS-COV-002` | TYP,RUN,NAT | helper 経由で owner call | passing dynamic trace により coverage = 1 |
| `SCS-COV-003` | TYP,NAT | unrelated case | coverage = 0、`OPEN(SCS.AddOwnerCoverage.v1)` |
| `SCS-COV-004` | TYP,RUN,NAT | assertion が owner call | passing dynamic trace で assertion coverage = 1 |
| `SCS-COV-005` | TYP,RUN,NAT | owner-bound property | exactly-once trace で property coverage = 1 |
| `SCS-COV-006` | TYP,NAT | docs-only `:example` | coverage = 0 |
| `SCS-COV-007` | TYP,NAT | stale passing evidence | structure はあっても verification open |
| `SCS-COV-008` | TYP,NAT | owner body の recursion だけ | contract coverage に数えない |
| `SCS-COV-009` | TYP,RUN,NAT | dead branch にだけ owner reference | static candidate でも `ERR(LS3238)`、verified coverage = 0 |

## 6. Generator / replay

| ID | layer | scenario | expected |
|---|---|---|---|
| `SCS-GEN-001` | RUN,NAT | seed 0 の先頭 16 Int | fixed vector match |
| `SCS-GEN-002` | RUN,NAT | seed 0 の先頭 16 Bool | fixed vector match |
| `SCS-GEN-003` | RUN,NAT | 同 seed/cases を 2 回 | accepted values / result identical |
| `SCS-GEN-004` | RUN,NAT | 別 seed | deterministic な別 fixed vector |
| `SCS-GEN-005` | RUN,NAT | failing Int property | specified shrink order の minimized value |
| `SCS-GEN-006` | RUN,NAT | constrained Int | accepted value が全 constraint を満たす |
| `SCS-GEN-007` | TYP,RUN,NAT | unsupported type | `ERR(LS3232)`、fallback なし |
| `SCS-GEN-008` | RUN,NAT | filter exhaustion | exact attempt limit で `ERR(LS3233)` |
| `SCS-GEN-009` | RUN,NAT | runner timeout | failed evidence、nonzero、skip success なし |
| `SCS-GEN-010` | RUN,NAT | replay evidence | resampling せず failing case を再現 |
| `SCS-GEN-011` | RUN,NAT | generator-version mismatch | evidence reject |
| `SCS-GEN-012` | RUN,NAT | shrink crash | failed evidence。original pass にしない |
| `SCS-GEN-013` | RUN,NAT | 2 binder counterexample | declaration-order restart の fixed shrink vector |
| `SCS-GEN-014` | RUN,NAT | constrained shrink candidate | precondition/constraint を満たす deterministic minimum |

## 7. Semantic model

| ID | layer | input | expected |
|---|---|---|---|
| `SCS-MOD-001` | TYP,NAT | typed function | canonical parameter / return / requirement |
| `SCS-MOD-002` | TYP,NAT | ordinary ADT | variant / field |
| `SCS-MOD-003` | TYP,NAT | GADT | constructor return / state parameter |
| `SCS-MOD-004` | TYP,NAT | record | source semantics に従う field order |
| `SCS-MOD-005` | TYP,NAT | constrained type | base / predicate / enforcement mode |
| `SCS-MOD-006` | TYP,NAT | trait / impl | requirement / implementation relation |
| `SCS-MOD-007` | TYP,NAT | nested module | unique canonical fully qualified ID |
| `SCS-MOD-008` | TYP,NAT | private declaration | fact は出すが public API から除外 |
| `SCS-MOD-009` | TYP,NAT | `order_typestate.ls` | derived transition edge が正確に 2 件 |
| `SCS-MOD-010` | TYP,NAT | unrelated input/output ADT | transition edge なし |
| `SCS-MOD-011` | TYP,NAT | unresolved reference | builder failure、partial trusted snapshot なし |
| `SCS-MOD-012` | TYP,NAT | duplicate canonical SymbolId | deterministic hard error |

## 8. Canonicalization

| ID | layer | input | expected |
|---|---|---|---|
| `SCS-CAN-001` | TYP,NAT | format A/B | canonical `BYTE-EQUAL` |
| `SCS-CAN-002` | TYP,NAT | alpha A/B | API/body canonical `BYTE-EQUAL` |
| `SCS-CAN-003` | TYP,NAT | checkout path が異なる | 全 fingerprint equal |
| `SCS-CAN-004` | TYP,NAT | span / line ending が異なる | 全 fingerprint equal |
| `SCS-CAN-005` | TYP,NAT | non-semantic enumeration shuffle | package bytes equal |
| `SCS-CAN-006` | TYP,NAT | ordered case list reverse | contract bytes differ |
| `SCS-CAN-007` | TYP,NAT | duplicate canonical field | `ERR(LS3201)` |
| `SCS-CAN-008` | TYP,NAT | floating payload | `ERR(LS3201)` |
| `SCS-CAN-009` | TYP,NAT | malformed SymbolId encoding | `ERR(LS3201)` |
| `SCS-CAN-010` | TYP,NAT | Rust/self-host vector | `BYTE-EQUAL` |
| `SCS-CAN-011` | TYP,NAT | pretty JSON / canonical writer | decode equal、hash は canonical bytes のみ |
| `SCS-CAN-012` | TYP,NAT | unsupported schema | `ERR(LS3203)` |

## 9. Fingerprint axis

| ID | layer | change | changed axis |
|---|---|---|---|
| `SCS-FP-001` | TYP,NAT | formatting / comment | none |
| `SCS-FP-002` | TYP,NAT | public parameter type | API と構造上影響する implementation |
| `SCS-FP-003` | TYP,NAT | public return type | API と構造上影響する implementation |
| `SCS-FP-004` | TYP,NAT | body only | implementation |
| `SCS-FP-005` | TYP,NAT | case only | contract |
| `SCS-FP-006` | TYP,NAT | property cases/seed/generator | contract |
| `SCS-FP-007` | TYP,NAT | `:doc` only | intent |
| `SCS-FP-008` | TYP,NAT | `:rationale` only | intent |
| `SCS-FP-009` | TYP,NAT | `:example` only | presentation |
| `SCS-FP-010` | TYP,NAT | `:since` / `:see-also` only | presentation |
| `SCS-FP-011` | TYP,NAT | constrained predicate | API + contract |
| `SCS-FP-012` | TYP,NAT | typestate signature | API + implementation |
| `SCS-FP-013` | TYP,NAT | private helper body | helper と transitive caller の implementation |
| `SCS-FP-014` | TYP,NAT | any symbol axis | package fingerprint changes |
| `SCS-FP-015` | TYP,NAT | changed package 内の independent symbol | symbol fingerprint stable |
| `SCS-FP-016` | TYP,NAT | recursive SCC member body | SCC 全 member と transitive caller implementation が変わる |
| `SCS-FP-017` | TYP,NAT | unresolved dynamic dependency | implementation confidence unknown、evidence reuse 禁止 |

## 10. Delta classification

| ID | layer | baseline/current | expected |
|---|---|---|---|
| `SCS-DIFF-001` | TYP,NAT | identical | 全 axis unchanged |
| `SCS-DIFF-002` | TYP,NAT | private symbol add | package public API unchanged |
| `SCS-DIFF-003` | TYP,NAT | public symbol add | `AddedCompatible` |
| `SCS-DIFF-004` | TYP,NAT | public symbol remove | `Breaking` |
| `SCS-DIFF-005` | TYP,NAT | public rename/move | breaking removal + compatible addition |
| `SCS-DIFF-006` | TYP,NAT | function arity/order/type | `Breaking` |
| `SCS-DIFF-007` | TYP,NAT | return type | `Breaking` |
| `SCS-DIFF-008` | TYP,NAT | ADT variant add/remove | `Breaking` |
| `SCS-DIFF-009` | TYP,NAT | record field change | `Breaking` |
| `SCS-DIFF-010` | TYP,NAT | constraint change | fixed rule の Breaking/Unknown。compatible 禁止 |
| `SCS-DIFF-011` | TYP,NAT | body only | implementation changed |
| `SCS-DIFF-012` | TYP,NAT | contract only | contract changed |
| `SCS-DIFF-013` | TYP,NAT | intent only | intent changed |
| `SCS-DIFF-014` | TYP,NAT | presentation only | presentation changed |
| `SCS-DIFF-015` | TYP,NAT | unsupported type/schema | `ERR(LS3211)` + unknown delta |
| `SCS-DIFF-016` | TYP,NAT | API/contract/body 同時変更 | 全 affected axis を保持 |

## 11. Policy / obligation

| ID | layer | scenario | expected |
|---|---|---|---|
| `SCS-POL-001` | TYP,DRV,NAT | private `typed` helper | purpose/coverage obligation なし |
| `SCS-POL-002` | TYP,DRV,NAT | checked public missing doc | `ERR(LS3220)` + `OPEN(ProvidePurpose)` |
| `SCS-POL-003` | TYP,DRV,NAT | checked public missing coverage | `ERR(LS3230)` + `OPEN(AddOwnerCoverage)` |
| `SCS-POL-004` | TYP,DRV,NAT | reviewed missing rationale | `ERR(LS3221)` + `OPEN(ProvideRationale)` |
| `SCS-POL-005` | TYP,DRV,NAT | overlapping profile | strongest profile |
| `SCS-POL-006` | DRV,NAT | invalid reviewed glob | config error |
| `SCS-POL-007` | DRV,NAT | config absent | `legacy-unmanaged`、accepted claim なし |
| `SCS-POL-008` | DRV,NAT | explicit checked flag | checked policy enforcement |
| `SCS-POL-009` | DRV,NAT | reviewed glob が non-function に一致 | configuration error |
| `SCS-OBL-001` | TYP,NAT | same input twice | obligation ID/order identical |
| `SCS-OBL-002` | TYP,NAT | body changed | `RerunContracts` |
| `SCS-OBL-003` | TYP,NAT | checked contract changed | `RerunContracts` + `ReconcileIntent` |
| `SCS-OBL-004` | TYP,NAT | reviewed contract changed | `RerunContracts` + `ReviewIntent` |
| `SCS-OBL-005` | TYP,NAT | compatible checked API changed | `VerifyCompatibility` + `ReconcileIntent` |
| `SCS-OBL-011` | TYP,NAT | breaking checked API | compatibility + migration + reconciliation |
| `SCS-OBL-012` | TYP,NAT | breaking reviewed API | compatibility + migration + human review |
| `SCS-OBL-006` | TYP,NAT | unknown delta | `ResolveAmbiguousDelta`、accepted false |
| `SCS-OBL-007` | TYP,NAT | strict legacy form | `ResolveLegacyMetadata` |
| `SCS-OBL-008` | TYP,NAT | runtime/public change | target policy の `VerifyTargetParity` |
| `SCS-OBL-009` | TYP,NAT | presentation-only | semantic review なし、projection rebuild のみ |
| `SCS-OBL-010` | TYP,NAT | evidence order shuffle | closure result identical |

## 12. Evidence / security

| ID | layer | scenario | expected |
|---|---|---|---|
| `SCS-EVD-001` | TYP,DRV,NAT | current passing case evidence | case requirement の `CLOSED(RerunContracts)` |
| `SCS-EVD-002` | TYP,DRV,NAT | old API fingerprint | `ERR(LS3212)` |
| `SCS-EVD-003` | TYP,DRV,NAT | old contract fingerprint | `ERR(LS3212)` |
| `SCS-EVD-004` | TYP,DRV,NAT | old implementation fingerprint | `ERR(LS3231)` |
| `SCS-EVD-005` | TYP,DRV,NAT | failed result | obligation open |
| `SCS-EVD-006` | TYP,DRV,NAT | target mismatch | `ERR(LS3213)` |
| `SCS-EVD-007` | TYP,DRV,NAT | generator/toolchain incompatible | evidence reject |
| `SCS-EVD-008` | TYP,DRV,NAT | trusted human signature | reviewed obligation closed |
| `SCS-EVD-009` | TYP,DRV,NAT | untrusted human key | `ERR(LS3222)` |
| `SCS-EVD-010` | TYP,DRV,NAT | current agent reconciliation | `CLOSED(ReconcileIntent)` |
| `SCS-EVD-011` | TYP,DRV,NAT | stale agent reconciliation | `ERR(LS3223)` |
| `SCS-EVD-012` | TYP,DRV,NAT | `affirmed` without current delta binding | `ERR(LS3223)` |
| `SCS-SEC-001` | TYP,DRV,NAT | agent で human requirement | open + `ERR(LS3222)` |
| `SCS-SEC-002` | DRV,NAT | current snapshot を baseline にする | trust を証明できず `ERR(LS3202)` |
| `SCS-SEC-003` | DRV,NAT | current patch だけで trust key 追加 | 同 change の key として不受理 |
| `SCS-SEC-004` | DRV,NAT | evidence fingerprint 手編集 | signature/digest validation failure |
| `SCS-SEC-005` | DRV,NAT | passed fallback evidence | `ERR(LS3213)` |
| `SCS-SEC-006` | DRV,NAT | contract 削除 + old evidence | coverage/rerun を閉じない |
| `SCS-SEC-007` | DRV,NAT | unrelated case 追加 | owner coverage open |
| `SCS-SEC-008` | DRV,NAT | timeout を skipped success と表す | representation reject |
| `SCS-SEC-009` | DOC | generated Markdown 手編集 | acceptance に影響せず再生成で上書き |
| `SCS-SEC-010` | DRV,NAT | obligation JSON に `closed=true` 追加 | field reject/ignore、closure 再計算 |
| `SCS-SEC-011` | TYP,DRV,NAT | private helper changed + old caller evidence | effective closure mismatch で `ERR(LS3231)` |
| `SCS-SEC-012` | TYP,DRV,NAT | static-only owner reference + passed unrelated path | `ERR(LS3238)`、coverage open |

## 13. CLI / artifact

| ID | layer | scenario | expected |
|---|---|---|---|
| `SCS-CLI-001` | DRV,NAT | `--emit-semantic` | deterministic snapshot |
| `SCS-CLI-002` | DRV,NAT | passing `--emit-evidence` | evidence + exit 0 |
| `SCS-CLI-003` | DRV,NAT | contract failure | failed evidence + nonzero |
| `SCS-CLI-004` | DRV,NAT | all evidence valid | accepted true、open error なし |
| `SCS-CLI-005` | DRV,NAT | open obligation | `ERR(LS3210)`、accepted false |
| `SCS-CLI-006` | DRV,NAT | baseline なし current compile | `SourceValid` のみ |
| `SCS-CLI-007` | DRV,NAT | unmanaged project | text/JSON に status 表示 |
| `SCS-CLI-008` | DRV,NAT | acceptance fail + Wasm request | nonzero、accepted artifact と報告しない |
| `SCS-CLI-009` | DRV,NAT | duplicate evidence path | EvidenceId で deterministic dedup |
| `SCS-CLI-010` | DRV,NAT | malformed evidence | fatal、partial acceptance なし |
| `SCS-CLI-011` | DRV,NAT | agent `attest intent` | digest は snapshot から取得し `ReconcileIntent` evidence を生成 |
| `SCS-CLI-012` | DRV,NAT | caller が fingerprint を直接指定 | option 不在または explicit reject |
| `SCS-ART-001` | DRV,NAT | output write fail | partial final file なし |
| `SCS-ART-002` | DRV,NAT | interrupted temp write | old final artifact 保持 |
| `SCS-ART-003` | DRV,NAT | snapshot round-trip | typed structure 保持 |
| `SCS-ART-004` | DRV,NAT | schema/version missing | `ERR(LS3203)` |
| `SCS-ART-005` | DRV,NAT | symlink replacement | safe reject または non-follow atomic behavior |

## 14. Projection

| ID | layer | scenario | expected |
|---|---|---|---|
| `SCS-DOC-001` | DOC,NAT | accepted checked symbol | static / checked / authored label |
| `SCS-DOC-002` | DOC,NAT | accepted reviewed symbol | attested + reviewer/evidence summary |
| `SCS-DOC-003` | DOC,NAT | constrained type | predicate + verification mode |
| `SCS-DOC-004` | DOC,NAT | typestate | derived transition 2 件 |
| `SCS-DOC-005` | DOC,NAT | unaccepted、opt-in なし | `ERR(LS3250)` |
| `SCS-DOC-006` | DOC,NAT | unaccepted、opt-in あり | prominent status + open obligation |
| `SCS-DOC-007` | DOC,NAT | `:params`/`:returns` なし | parameter/return を型から導出 |
| `SCS-DOC-008` | DOC,NAT | docs-only example | example label、checked ではない |
| `SCS-DOC-009` | DOC,NAT | valid agent reconciliation | `agent-reconciled`。`attested` とは表示しない |
| `SCS-ONT-001` | DOC,NAT | ontology fixture | expected canonical nodes/edges |
| `SCS-ONT-002` | DOC,NAT | every edge | provenance + assurance |
| `SCS-ONT-003` | DOC,NAT | graph root | `closedWorld: true` |
| `SCS-ONT-004` | DOC,NAT | arbitrary authored edge | API 不在または explicit reject |
| `SCS-ONT-005` | DOC,NAT | source declaration shuffle | canonical graph order stable |

## 15. LSP / MCP parity

| ID | layer | scenario | expected |
|---|---|---|---|
| `SCS-LSP-001` | LSP | snapshot request | compiler service と同じ typed snapshot |
| `SCS-LSP-002` | LSP | open obligation | CLI と code/ID/span equal |
| `SCS-LSP-003` | LSP | baseline unavailable | `LS3202`、guess delta なし |
| `SCS-LSP-004` | LSP | cancel/timeout | failure propagation、stale result を current にしない |
| `SCS-LSP-005` | LSP | contract skeleton | edit suggestion のみ、evidence なし |
| `SCS-MCP-001` | MCP | `lsharp_semantic_snapshot` | service と `STRUCT-EQUAL` |
| `SCS-MCP-002` | MCP | `lsharp_semantic_diff` | service と `STRUCT-EQUAL` |
| `SCS-MCP-003` | MCP | `lsharp_semantic_obligations` | service と `STRUCT-EQUAL` |
| `SCS-MCP-004` | MCP | `lsharp_verify_change` | accepted Boolean は compiler-derived |
| `SCS-MCP-005` | MCP | tool failure | structured failure、prose success なし |
| `SCS-MCP-006` | MCP | human obligation close request | required human evidence を返し close しない |
| `SCS-MCP-007` | MCP | skeleton だけ適用 | obligation open |
| `SCS-MCP-008` | MCP,LSP,DRV | same fixture | diagnostic/obligation `STRUCT-EQUAL` |

## 16. Migration

| ID | layer | scenario | expected |
|---|---|---|---|
| `SCS-MIG-001` | TYP,DRV,NAT | strict `:params` | `ERR(LS3240)` + deterministic fix category |
| `SCS-MIG-002` | TYP,DRV,NAT | strict prose `:returns` | `ERR(LS3240)` |
| `SCS-MIG-003` | TYP,DRV,NAT | strict `:invariant` | `ERR(LS3240)`、silent conversion なし |
| `SCS-MIG-004` | TYP,DRV,NAT | strict `:transitions` | `ERR(LS3240)` + derived transition guidance |
| `SCS-MIG-005` | TYP,NAT | non-strict legacy | lossless pending migration、evidence ではない |
| `SCS-MIG-006` | DRV,NAT | migration suggestion | source edit のみ。source から acceptance 再実行 |
| `SCS-MIG-007` | DRV,NAT | new project template | private typed / public checked config |
| `SCS-MIG-008` | DRV,NAT | old project no config | compile 可、acceptance は unmanaged |

## 17. Native / target evidence

| ID | layer | scenario | expected |
|---|---|---|---|
| `SCS-NAT-001` | TYP,NAT | all canonical vector | Rust/self-host `BYTE-EQUAL` |
| `SCS-NAT-002` | RUN,NAT | generator vector | values / shrink path equal |
| `SCS-NAT-003` | TYP,NAT | diagnostic | code / subject / normalized span equal |
| `SCS-NAT-004` | DRV,NAT | acceptance | obligation / closure equal |
| `SCS-NAT-005` | NAT | unsupported self-host slice | explicit diagnostic、Rust fallback なし |
| `SCS-NAT-006` | NAT | fallback probe | evidence が fallback を記録し reject |
| `SCS-NAT-007` | MAC | claimed Mac public surface | actual native compile/test/artifact pass |
| `SCS-NAT-008` | LNX | claimed Linux public surface | actual native/VM compile/test/artifact pass |
| `SCS-NAT-009` | MAC,LNX | same fixed input | snapshot/fingerprint equal |
| `SCS-NAT-010` | NAT | stale stage0 fingerprint | evidence production 前に stage0 reject |

## 18. Diagnostic registry

| ID | layer | scenario | expected |
|---|---|---|---|
| `SCS-DIA-001` | TYP,DRV,NAT | reserved code | code ごとに一つの stable category/severity |
| `SCS-DIA-002` | TYP,DRV,NAT | obligation diagnostic | subject/span/obligation ID/remediation category |
| `SCS-DIA-003` | TYP,DRV,NAT | source span なし | provenance-only location を明示 |
| `SCS-DIA-004` | LSP,MCP,DRV | same failure | code/category identical |
| `SCS-DIA-005` | TYP,NAT | unknown internal error | pass/closed へ変換しない |

## 19. Required command evidence

各 implementation PR は導入した focused test の exact output を記録する。work package 完了前に relevant subset と
repository gate を実行する。

```bash
cargo test -p lsharp-syntax semantic
cargo test -p lsharp-types semantic
cargo test -p lsharp-driver semantic
cargo test -p lsharp-docs semantic
cargo test -p lsharp-lsp semantic
cargo test -p lsharp-wasm --test e2e semantic_contract
scripts/native-selfhost-dev.sh check tests/fixtures/semantic/v1
scripts/native-selfhost-dev.sh test tests/fixtures/semantic/v1
cargo fmt --all -- --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
git diff --check
```

target-specific test は claimed native/public surface を変えた work package で追加する。fixture-level Rust pass を
native / target evidence の代用にしない。

## 20. Final acceptance audit

subsystem completion 前に test ID ごとの machine-readable report を作る。

- implementation commit;
- fixture digest;
- command;
- layer / target;
- producer / toolchain identity;
- pass/fail;
- fallback-used;
- artifact / evidence path;
- required な Rust/native matching vector。

mandatory ID の欠落、conflicting duplicate、skip、stale、fallback、matrix より狭い scope の evidence があれば
audit failure とする。
