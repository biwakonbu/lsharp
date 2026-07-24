# Semantic Contract System 実装仕様

状態: 実装上の正本
言語契約: [`../../../language/semantic-contract-system.md`](../../../language/semantic-contract-system.md)
source form: [`../../../language/semantic-contract-language.md`](../../../language/semantic-contract-language.md)
実装順序: [`implementation-plan.md`](./implementation-plan.md)
受入試験: [`test-matrix.md`](./test-matrix.md)
運用例: [`operation-example.md`](./operation-example.md)
agent 実行契約: [`agent-execution-guide.md`](./agent-execution-guide.md)

## 1. Reader contract

本書は GPT-5.6 Luna を含む実装 agent が、architecture や acceptance rule を独自解釈せず実装できる粒度で
記述する。

実装 agent は次を MUST とする。

1. production file を編集する前に、上記 5 文書を読む。
2. `implementation-plan.md` の work package 順を守る。
3. 各 production change より先に、対応する RED test を追加する。
4. Rust oracle lane と self-host/native lane の evidence を分離する。
5. 本文にない semantic choice が必要なら、実装で暗黙選択せず規範文書の review change とする。
6. 各 MUST と test ID を acceptance requirement として扱う。
7. generated artifact を source of truth にしない。

本機能を ontology database、doc generator、prompt rule、best-effort metadata freshness として実装しては
ならない。

## 2. Final architecture

authority は一方向である。

```text
L# source
  -> parser AST
  -> resolved / inferred program
  -> canonical contract
  -> canonical semantic snapshot
  -> fingerprint
  -> trusted-baseline delta
  -> obligation
  -> validated evidence
  -> acceptance result
  -> projection (docs / API / graph / LSP / MCP)
```

projection から source への reverse synchronization は設けない。

### 2.1 Crate responsibility

| crate | responsibility |
|---|---|
| `lsharp-syntax` | source contract form と span の lossless parse |
| `lsharp-types` | canonical contract、resolved semantic model、canonicalization、fingerprint、delta、policy、obligation、evidence validation |
| `lsharp-ir` | implementation fingerprint に必要な normalized typed body data の提供。policy は持たない |
| `lsharp-wasm` | deterministic contract sandbox、dynamic trace、runtime evidence。delta classification は行わない |
| `lsharp-driver` | package context、trusted baseline、command、artifact、exit status の composition |
| `lsharp-docs` | read-only specification / ontology projection |
| `lsharp-lsp` | source diagnostic と structured read-only semantic request |
| self-host source | behaviorally equivalent な実装と canonical test-vector output |

Markdown を parse して semantics を復元する crate を作ってはならない。

### 2.2 Required module layout

次の module を作る。repository の file size guidance を超える前に分割する。

```text
crates/lsharp-types/src/semantic/
  mod.rs
  model.rs
  symbol.rs
  canonical.rs
  fingerprint.rs
  diff.rs
  policy.rs
  obligation.rs
  evidence.rs
  diagnostic.rs
  builder.rs

crates/lsharp-docs/src/semantic/
  mod.rs
  specification.rs
  ontology.rs

crates/lsharp-driver/src/semantic/
  mod.rs
  artifact.rs
  baseline.rs
  command.rs
  trust.rs

crates/lsharp-lsp/src/semantic.rs
```

`mcp_server.rs`、`infer.rs`、`metadata_check.rs`、`tracker.rs` へ全機能を押し込まない。

self-host mirror:

```text
selfhost/src/Types/Semantic/
  Model.ls
  Symbol.ls
  Canonical.ls
  Fingerprint.ls
  Diff.ls
  Policy.ls
  Obligation.ls
  Evidence.ls

selfhost/src/Tools/Doc/SemanticSpecification.ls
selfhost/src/Tools/Doc/SemanticOntology.ls
selfhost/src/Tools/Lsp/Semantic.ls
```

## 3. Immutable design decision

実装は次を変更せず適用する。

1. source form は既存 `:case`、`:assert`、`:property` を使う。
2. `:property` を owner-bound とし、`result` は owner の実戻り値にする。
3. `:assert` は implicit binding のない module scope expression のままにする。
4. legacy form は lossless に保持するが strict mode では duplicated semantics を拒否する。
5. parameter、return、variant、field、constraint、trait、typestate fact は導出する。
6. authored ontology triple を追加しない。
7. API、contract、intent、implementation、presentation の 5 fingerprint を独立させる。
8. dedicated canonical writer が出す canonical JSON bytes を hash する。
9. `serde_json::to_vec` や `Display` output を hash input にしない。
10. SHA-256 と fixed domain prefix を使う。
11. delta は axis ごとに持ち、exclusive enum にしない。
12. obligation closure は obligation と current evidence の pure function にする。
13. baseline は current change 外の provenance を検証できる場合だけ trusted とする。
14. version 1 human attestation は Ed25519 とする。
15. policy profile は `typed`、`checked`、`reviewed` の固定 3 種類とする。
16. public UX は `compile`、`test`、`doc` と例外的な `attest` を中心にする。
17. CLI、LSP、MCP は同じ semantic service result を返す。
18. owner coverage は static reference ではなく passing dynamic trace で確定する。
19. implementation fingerprint は execution dependency の SCC closure を含む。
20. checked API / contract change は agent または human reconciliation、reviewed change は human attestation を要求する。
21. unknown、unsupported、timeout、nondeterministic effect、fallback、provenance mismatch は fail-closed とする。

これらを弱めた実装は partial conformance ではなく contract violation である。

## 4. Core data model

### 4.1 Identifier

validated newtype として実装する。

```rust
pub struct PackageId(String);
pub struct SymbolId(String);
pub struct ObligationId([u8; 32]);
pub struct EvidenceId([u8; 32]);
pub struct Digest([u8; 32]);
```

canonical `SymbolId` text:

```text
lsharp://<percent-encoded-package>/<percent-encoded-module>/<kind>/<percent-encoded-name>
```

規則:

- package version は `SymbolId` に含めない。
- root module は `_root` とする。
- kind は `function`、`type`、`constructor`、`record`、`trait`、`impl`、`alias`、
  `constrained-type` のいずれか。
- percent encoding は UTF-8 byte を uppercase hex で表す。
- malformed / non-canonical encoding を拒否する。
- path、span、timestamp を含めない。

### 4.2 Snapshot type

untyped `serde_json::Value` field を使わず、次の logical model を実装する。

```rust
pub struct SemanticPackageSnapshot {
    pub schema_version: SchemaVersion,
    pub semantics_version: SemanticsVersion,
    pub package: PackageIdentity,
    pub producer: ToolchainIdentity,
    pub symbols: BTreeMap<SymbolId, SemanticSymbol>,
    pub package_fingerprint: Digest,
}

pub struct SemanticSymbol {
    pub id: SymbolId,
    pub kind: SymbolKind,
    pub visibility: Visibility,
    pub static_facts: StaticFacts,
    pub contracts: CanonicalContractSuite,
    pub intent: IntentContent,
    pub presentation: PresentationContent,
    pub relations: Vec<SemanticRelation>,
    pub fingerprints: SymbolFingerprints,
    pub provenance: SourceProvenance,
}

pub struct SymbolFingerprints {
    pub api: Digest,
    pub contract: Digest,
    pub intent: Digest,
    pub implementation: Digest,
    pub presentation: Digest,
}
```

`SourceProvenance` は diagnostic 用 path/span を持ってよいが、全 semantic fingerprint から除外する。

### 4.3 Static fact

closed enum と typed structure を使う。

```rust
pub enum StaticFacts {
    Function(FunctionFacts),
    AlgebraicType(AlgebraicTypeFacts),
    Record(RecordFacts),
    Trait(TraitFacts),
    TraitImplementation(TraitImplementationFacts),
    Alias(AliasFacts),
    ConstrainedType(ConstrainedTypeFacts),
    Constructor(ConstructorFacts),
}

pub struct FunctionFacts {
    pub type_scheme: CanonicalTypeScheme,
    pub parameters: Vec<ParameterFact>,
    pub return_type: CanonicalType,
    pub where_requirements: Vec<TraitRequirement>,
    pub public_errors: Vec<CanonicalType>,
}
```

内部 type を display string で保持してはならない。resolved `Type` を mirror する closed `CanonicalType`
tree を作り、type variable を alpha-normalize する。

### 4.4 Contract model change

現在の canonical contract IR を拡張し、第二 inventory を作らない。

```rust
pub struct CanonicalProperty {
    pub owner: SymbolId,
    pub binders: Vec<Binder>,
    pub invocation: OwnerInvocation,
    pub preconditions: Vec<Predicate>,
    pub postcondition: Predicate,
    pub sampling: SamplingPlan,
    pub source_span: Span,
}

pub struct OwnerInvocation {
    pub argument_binders: Vec<BinderId>,
    pub result_type: CanonicalType,
}
```

builder は inferred owner signature から `result_type` を取得する。synthetic probe の `result` を
unconstrained にしてはならない。

executable contract へ resolved call index を追加する。

```rust
pub struct ContractReferenceIndex {
    pub reachable_symbols: BTreeSet<SymbolId>,
}

pub struct ContractExecutionTrace {
    pub invoked_symbols: Vec<SymbolId>,
    pub imported_capabilities: BTreeSet<RuntimeCapability>,
    pub fallback_used: bool,
}

pub enum RuntimeCapability {
    DeterministicMemory,
    Clock,
    Random,
    FileSystem,
    Network,
    Process,
    HostFallback,
    UnknownImport(String),
}
```

version 1 contract execution が許可する capability は `DeterministicMemory` だけである。その他は `LS3237` とする。
reference index は static candidate と dependency closure に使う。checked owner coverage は passing
`ContractExecutionTrace` が owner invocation を含む場合だけ成立し、string matching や dead reference を使わない。

### 4.5 Delta type

```rust
pub struct PackageDelta {
    pub baseline: SnapshotIdentity,
    pub current: SnapshotIdentity,
    pub symbols: BTreeMap<SymbolId, SemanticDelta>,
}

pub struct SemanticDelta {
    pub subject: SymbolId,
    pub api: ApiDelta,
    pub contract: ContentDelta,
    pub intent: ContentDelta,
    pub implementation: ContentDelta,
    pub presentation: ContentDelta,
    pub confidence: DeltaConfidence,
}
```

API change と implementation/intent change が同時に発生しても全 axis を保持する。

### 4.6 Obligation / evidence type

規範文書の shape を typed structure で実装する。最低限の enum:

```rust
pub enum EvidenceKind {
    TypeChecked,
    CasesPassed,
    AssertionsPassed,
    PropertiesPassed,
    CompatibilityChecked,
    TargetParityChecked,
    MigrationNoteAttested,
    HumanIntentAttested,
    AgentIntentAttested,
}

pub enum ProducerKind {
    Compiler,
    ContractRunner,
    Human,
    Agent,
    Ci,
}

pub enum EvidenceResult {
    Passed,
    Failed { diagnostic_codes: Vec<String> },
}
```

`Skipped` success state は設けない。unsupported / skipped は failure diagnostic として表す。

## 5. Semantic builder

### 5.1 Input

resolved input だけを受け取る。

```rust
pub struct SemanticBuildInput<'a> {
    pub package: &'a PackageIdentity,
    pub program: &'a Program,
    pub inferred: &'a InferredProgram,
    pub contracts: &'a [ContractSuite],
    pub module_graph: &'a ResolvedModuleGraph,
    pub typed_bodies: &'a TypedBodyIndex,
}
```

既存 API が `InferredProgram`、`ResolvedModuleGraph`、`TypedBodyIndex` を公開していない場合は narrow な
read-only structure を追加する。formatted source から再構築してはならない。

### 5.2 Build order

順序を固定する。

1. module / visibility context 付きで top-level declaration を列挙する。
2. canonical `SymbolId` を割り当てる。
3. inferred result から canonical type と static fact を作る。
4. contract expression の reference を `SymbolId` または local binding へ resolve する。
5. property を owner signature へ bind し、`result` を owner return type にする。
6. legacy / intent / presentation form を分類する。
7. semantic relation を導出する。
8. typed body を normalize する。
9. resolved execution dependency graph を SCC へ縮約し、effective implementation closure digest を計算する。
10. axis 別 canonical payload と fingerprint を計算する。
11. symbol を sort し package fingerprint を計算する。
12. snapshot invariant を検証する。
13. snapshot または diagnostic を返す。partial trusted snapshot は返さない。

### 5.3 Normalized implementation body

expression tree は次の形式にする。

- span / comment を除外する。
- global reference を `SymbolId` へ置換する。
- local reference を lexical binder index で表す。
- local name を除外する。
- type variable を alpha-normalize する。
- pattern constructor reference を resolve する。
- operational な declaration / expression order を保持する。
- unordered set を sort する。

目的は conservative change detection であり semantic equivalence proof ではない。等価な rewrite でも
implementation fingerprint が変わり、reverification が必要になってよい。

### 5.4 Effective implementation closure

resolved call / trait-dispatch / constructor / runtime dependency graph を SCC へ縮約する。各 SCC digest は sorted
member `(SymbolId, own-body-digest)` と sorted outgoing SCC digest から末端順に計算する。symbol の
implementation payload は own-body digest と所属 SCC digest を含む。したがって helper や recursive member の変更は
全 transitive caller の executable evidence を stale にする。解決不能な dynamic edge は `Unknown` とし再利用しない。

## 6. Canonical writer と fingerprint

### 6.1 Writer API

```rust
pub trait CanonicalEncode {
    fn encode(&self, writer: &mut CanonicalJsonWriter) -> Result<(), CanonicalError>;
}

pub struct CanonicalJsonWriter<W: Write> {
    output: W,
    state: WriterState,
}
```

writer の要件:

- insignificant whitespace を出さない。
- object field は schema-defined order で出す。
- duplicate field を拒否する。
- map key を emit 前に sort する。
- Rust/self-host vector が共有する単一 escaping rule を使う。
- floating-point を拒否する。
- bytes を SHA-256 hasher へ直接渡せる。
- `Debug` / `Display` output を使わない。

### 6.2 Domain-separated hash

```rust
fn fingerprint(axis: FingerprintAxis, payload: &impl CanonicalEncode) -> Digest {
    sha256(
        b"lsharp-semantic-v1\0"
        + axis.domain_tag()
        + b"\0"
        + payload.canonical_bytes()
    )
}
```

axis tag は次の lowercase text で固定する。

```text
api
contract
intent
implementation
presentation
package
obligation
evidence
```

### 6.3 Package root

canonical `SymbolId` と 5 digest の pair を sort して encode する。どの symbol axis が変わっても package
root は変わり、semantics-free な source enumeration order では変わらない。

## 7. Contract checking / execution

### 7.1 Owner-bound property checker

現在の unconstrained synthetic `result` probe を次へ置き換える。

1. owner function scheme を resolve する。
2. property 単位で一度 instantiate する。
3. binder count と parameter count を比較する。
4. 各 binder type と対応 parameter type を unify する。
5. binder を property lexical scope へ追加する。
6. instantiated return type を持つ `result` を scope へ追加する。
7. 全 precondition / postcondition が `Bool` と確認する。
8. duplicate binder、reserved `result`、empty property、zero cases、static false precondition、
   static tautological postcondition を拒否する。
9. instantiated owner invocation を canonical IR へ保存する。

runtime は accepted generated input ごとに owner をちょうど 1 回呼び、その実 result で postcondition を
評価する。

### 7.2 Contract equality

host equality fallback ではなく internal `ContractComparable` decision を実装する。

version 1 対応:

- `Int`、`Bool`、`String`、unit;
- base が comparable な constrained type;
- 全 field が comparable な concrete ADT;
- 全 field が comparable な concrete record。

function、unresolved type variable、resource、unsupported runtime value は `LS3235`。polymorphic case は
concrete monomorphic instantiation を必要とする。

### 7.3 Generator

`type-directed-splitmix64-v1` を次のとおり固定する。

- seed state は SplitMix64。
- `Bool` は generated word 1 個の least significant bit を使う。
- `Int` は generated word 1 個を signed 64-bit two's-complement として扱う。
- shrink 順序は規範言語文書 6.2 の algorithm を byte-for-byte 実装する。
- constrained type は全 runtime-evaluable constraint で base value を filter する。
- attempt limit は `max(cases * 100, 10_000)`。
- replay は seed、case index、accepted value、generator version、minimized counterexample を持つ。

Rust/self-host は statistical test ではなく fixed vector fixture を共有する。

### 7.4 Coverage

static candidate は resolved contract reference、verified coverage は runtime trace から計算する。

```rust
pub struct OwnerCoverage {
    pub cases: usize,
    pub assertions: usize,
    pub properties: usize,
}
```

case/assertion は passing trace に owner が 1 回以上現れた場合だけ数える。property は runner が owner を exactly once
呼んだ trace だけを数える。`checked` profile は verified total coverage > 0 と、存在する各 contract kind の current
passing evidence を要求する。unsupported capability、fallback、trace 欠落は `LS3237` / `LS3238` で拒否する。

## 8. Delta classifier

pure function とする。

```rust
pub fn diff(
    baseline: &SemanticPackageSnapshot,
    current: &SemanticPackageSnapshot,
) -> Result<PackageDelta, Vec<SemanticDiagnostic>>;
```

rule は table-driven / versioned とし、text similarity から compatibility を推測しない。

最低限の exact rule:

- baseline にない current private symbol: package API unchanged、content axis は added。
- baseline にない current public symbol: `AddedCompatible`。
- baseline public symbol が current にない: `Breaking`。
- public `SymbolId` change: removal + addition で breaking。
- identical API canonical bytes: `Unchanged`。
- public function signature change: `Breaking`。
- public ADT / record shape change: `Breaking`。
- public constraint change: later versioned proof rule がない限り `Breaking`。
- private-only API bytes change: package public API unchanged。
- unrecognized schema / type shape: `Unknown` + `LS3211`。

## 9. Policy / obligation engine

### 9.1 Configuration

規範文書の fixed TOML section を parse する。section がない project は `legacy-unmanaged` とする。

- ordinary compile は可能。
- `ChangeAccepted` は出さない。
- `--verify-against` は explicit profile または config を必要とする。
- generated spec は `acceptance: unmanaged` を表示する。

new project template は private `typed`、public `checked`、strict legacy metadata、empty reviewed glob を生成する。
version 1 の reviewed glob は function だけを選択でき、non-function match は configuration error とする。

### 9.2 Obligation derivation

```rust
pub fn derive_obligations(
    policy: &ResolvedPolicy,
    delta: &PackageDelta,
    current: &SemanticPackageSnapshot,
) -> Vec<Obligation>;
```

| condition | obligation |
|---|---|
| checked/reviewed public symbol に `:doc` がない | `ProvidePurpose` |
| reviewed symbol に `:rationale` がない | `ProvideRationale` |
| checked/reviewed function に owner coverage がない | `AddOwnerCoverage` |
| implementation changed | `RerunContracts` |
| checked contract changed | `RerunContracts`、`ReconcileIntent` |
| reviewed contract changed | `RerunContracts`、`ReviewIntent` |
| compatible checked public API changed | `VerifyCompatibility`、`ReconcileIntent` |
| compatible reviewed public API changed | `VerifyCompatibility`、`ReviewIntent` |
| breaking checked public API changed | `VerifyCompatibility`、`DocumentMigration`、`ReconcileIntent` |
| breaking reviewed public API changed | `VerifyCompatibility`、`DocumentMigration`、`ReviewIntent` |
| reviewed symbol の intent changed | `ReviewIntent` |
| strict mode の legacy form | `ResolveLegacyMetadata` |
| any unknown axis | `ResolveAmbiguousDelta` |
| runtime/public target behavior changed | repository target policy に従い `VerifyTargetParity` |

obligation は subject、rule ID、obligation digest で sort する。

### 9.3 Closure

```rust
pub fn evaluate_obligation(
    obligation: &Obligation,
    evidence: &[EvidenceRecord],
    trust: &TrustContext,
) -> ObligationEvaluation;
```

result は `Closed { evidence_ids }` または `Open { reasons }`。mutable state を持たない。

## 10. Evidence / trust

### 10.1 Executable evidence

`lsharp test --emit-evidence` は symbol / contract kind ごとに record を出す。record は次を含む。

- normalized command argument;
- toolchain / generator version;
- target triple;
- Rust/native lane;
- replay data と dynamic execution trace;
- imported capability と `fallback_used`。native evidence では必ず false;
- result diagnostic;
- rule が要求する current fingerprint と effective implementation closure digest。

failed run も audit 用 failed evidence を出してよいが obligation は閉じない。

### 10.2 Agent reconciliation

```bash
lsharp attest intent \
  --semantic target/lsharp/current.semantic.json \
  --subject 'lsharp://bank/Bank/function/remaining-balance' \
  --reviewer-kind agent \
  --disposition updated \
  --agent-run-id "$LUNA_RUN_ID" \
  --summary "契約変更に合わせて purpose を更新" \
  --out target/lsharp/intent.reconciliation.json
```

`checked` API / contract change では、agent は `updated` または `affirmed` disposition を持つ
`AgentIntentAttested` evidence を出す。payload は baseline/current の API・contract・intent fingerprint、agent/run
identity、summary digest を含む。これは `ReconcileIntent` だけを閉じ、generated prose や human review の代用にしない。

### 10.3 Human attestation

```bash
lsharp attest intent \
  --semantic target/lsharp/current.semantic.json \
  --subject 'lsharp://bank/Bank/function/remaining-balance' \
  --reviewer-kind human \
  --disposition reviewed \
  --key ~/.config/lsharp/reviewer.ed25519 \
  --out target/lsharp/intent.attestation.json
```

signed payload:

- subject;
- API / contract / intent fingerprint;
- reviewer ID;
- reviewer kind `human`;
- attestation purpose;
- schema/version;
- optional migration-note digest。

private key を repository から読んではならない。verification は external trust store または trusted
baseline key を使う。ローカル Ed25519 command は参照フローであり、実運用では SCM / CI が認証済み human review を
同じ signed envelope へ変換してよい。ただし adapter は reviewer identity、対象 fingerprint、review purpose を保持し、
agent identity を human へ昇格させてはならない。

## 11. CLI / artifact behavior

### 11.1 `compile`

既存 compile command へ次を追加する。

```text
--semantic-profile <typed|checked|reviewed>
--emit-semantic <path>
--verify-against <path>
--evidence <path>        # repeatable
--trust-store <path>
--emit-obligations <path>
--allow-unaccepted-projection
```

behavior:

1. current internal inconsistency は artifact emission 前に fail する。
2. `--emit-semantic` は deterministic pretty JSON projection を atomic write する。
3. `--verify-against` は provenance validation、delta、obligation、evidence validation を行い、
   error-severity open obligation があれば nonzero。
4. `--emit-obligations` は audit 用に open/closed evaluation の両方を出す。
5. output path failure で partial final file を残さない。
6. semantic acceptance を要求して fail した場合、Wasm output を accepted success と報告しない。
7. source が `SourceValid` なら、受理失敗時でも `status: unaccepted` の current snapshot を明示的な出力先へ
   emit してよい。これは trusted baseline や accepted artifact として再利用できない。

### 11.2 `test`

canonical case、assertion、owner-bound property を実行する。test failure 時も failed evidence を atomic に
出してよいが command は nonzero。

### 11.3 `doc`

`lsharp doc --semantic <snapshot> --out <directory>` を正規形とする。source を再 parse して意味推論しない。出力:

- symbol Markdown page;
- package index;
- machine JSON;
- ontology JSON;
- ontology JSON から生成する optional DOT view。

unaccepted output は `--allow-unaccepted-projection` を必要とし、status と open obligation を目立つ形で示す。

### 11.4 `attest`

`attest intent` は agent reconciliation と human attestation の typed payload を作る。agent mode は secret key を
要求しないが producer/run identity を必須とし、human mode は external Ed25519 private key を必須とする。どちらも
semantic snapshot から fingerprint を読み、caller が digest を直接指定できないようにする。

### 11.5 Atomic artifact

sibling temporary file へ write、`fsync`、rename の順に行う。schema と producer identity を含める。
platform API が可能なら symlink replacement を拒否する。artifact parse error は fatal とする。

## 12. LSP / MCP

両 adapter が使う一つの compiler service を実装する。

```rust
pub trait SemanticService {
    fn snapshot(&self, request: SnapshotRequest) -> SemanticResponse;
    fn diff(&self, request: DiffRequest) -> SemanticResponse;
    fn obligations(&self, request: ObligationRequest) -> SemanticResponse;
    fn contract_skeleton(&self, request: SkeletonRequest) -> SemanticResponse;
    fn verify_change(&self, request: VerifyRequest) -> SemanticResponse;
}
```

MCP name は言語契約どおり固定する。LSP は `lsharp/semantic/*` custom request を使ってよい。両者は同じ
response type と diagnostic code を serialize する。

`contract_skeleton` は source edit suggestion を返してよいが evidence を生成しない。

## 13. Projection implementation

### 13.1 Specification

`lsharp-docs` は snapshot と obligation evaluation を受け取り、次の label を使う。

- compiler fact: `static`;
- passing contract: `checked` + evidence summary;
- reconciliation なしの intent: `authored`;
- valid agent reconciliation: `agent-reconciled`;
- trusted human review: `attested`;
- obligation remaining: `unaccepted`。

snapshot test は label を assertion する。prose content があるだけでは不十分。

### 13.2 Ontology

read-only graph schema:

```json
{
  "schema": "lsharp-semantic-ontology-v1",
  "closedWorld": true,
  "nodes": [],
  "edges": []
}
```

全 edge は `kind`、`source`、`target`、`provenance`、`assurance` を持つ。version 1 で arbitrary graph
edit を受ける API は実装しない。

## 14. Implementation execution

mandatory な work package、TDD command、PR slicing、Definition of Done は
[`implementation-plan.md`](./implementation-plan.md) が定義する。agent は
[`agent-execution-guide.md`](./agent-execution-guide.md) の入出力・停止条件にも従う。本書と同じ authority を持つため、
architecture だけ実装して plan/evidence requirement を省略してはならない。
