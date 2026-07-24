# Semantic Contract System

状態: 規範的な言語契約
スキーマ: `lsharp-semantic-snapshot-v1`
正規化: `lsharp-semantic-canonical-v1`
fingerprint: `lsharp-semantic-sha256-v1`
property generator: `type-directed-splitmix64-v1`

## 1. 目的

Semantic Contract System は、L# プログラムの意味を表現・検査・可視化・レビューするための
正本である。目的は次の 4 点に限定する。

1. 型検査後に確定する事実を、重複した prose ではなく compiler-owned data として保持する。
2. 振る舞いに関する主張を executable contract とし、その実行証跡を対象プログラムへ結び付ける。
3. 人間の意図を machine-checked truth と混同せず、意味変更後の stale な意図を検出する。
4. 変更から deterministic な obligation を導出し、必要な evidence が揃うまで変更を accepted としない。

本システムは汎用 ontology reasoner ではない。L# の型・契約・変更受理に operational semantics を
与える closed な semantic model である。

## 2. 規範用語

本文の **MUST**、**MUST NOT**、**SHOULD**、**SHOULD NOT**、**MAY** は規範要件を表す。

- **static fact**: parse・名前解決・型検査済み source から compiler が導出した事実。
- **checked claim**: current evidence によって実行結果が確認された canonical contract。
- **reconciled intent**: API / contract 変更後に、現在の authored intent を更新または再確認した agent/human 証跡。
- **attested intent**: semantic fingerprint に結び付いた trusted human review 証跡を持つ自然言語の意図。
- **presentation metadata**: 生成表示にだけ使う authored information。
- **semantic snapshot**: package 全体の canonical semantic representation。
- **trusted baseline**: current change が書き換えられない経路から得た変更前 snapshot。
- **semantic delta**: baseline と current snapshot の deterministic な差分。
- **obligation**: delta と policy から導出される必要作業。
- **evidence**: obligation rule が受理できる再現可能な証跡。
- **accepted change**: 必須 obligation がすべて valid evidence で閉じた変更。

## 3. 不変条件

実装は次をすべて満たさなければ conforming ではない。

1. machine-readable semantics の正本は compiler-owned semantic snapshot だけである。
2. 仕様書、API 記述、diagram、LLM context、ontology graph はすべて projection である。
3. projection を編集して source や semantic snapshot を更新してはならない。
4. prose の存在を correctness や proof として扱ってはならない。
5. executable contract は current execution evidence がない限り `checked` と表示してはならない。
6. evidence rule が参照する fingerprint が変わった時点で、旧 evidence は stale になる。
7. obligation に mutable な `closed` flag を持たせてはならない。closure は毎回導出する。
8. unknown、unsupported、timeout、fallback、provenance mismatch は fail-closed とする。
9. LLM・agent・human prose judge を acceptance oracle にしてはならない。
10. semantic model の relation は operational semantics を持つ導出関係に限定する。
11. version 1 では free-form な `subject/predicate/object` triple を source から author できない。
12. public API または checked claim が変わった場合、authored intent は reconciliation evidence が得られるまで fresh と扱わない。
13. contract evidence は owner 本体だけでなく、実行に影響する依存閉包の fingerprint に bind する。
14. 型検査成功、contract 成功、change acceptance、release readiness を別の状態として扱う。

## 4. 意味情報の 3 層と presentation

### 4.1 Static fact

型検査後に compiler が確定する。対象は次を含む。

- fully qualified symbol identity、kind、visibility;
- alpha-normalized type scheme;
- parameter と return type;
- ADT variant、GADT constructor return type、record field;
- constrained type の base type、predicate、enforcement mode;
- trait definition、requirement、implementation;
- module dependency;
- function type から導出できる typestate transition;
- return type が表す public error surface;
- diagnostic 用 source provenance。

これらを `:params`、`:returns`、別 schema、手書き graph へ重複記述してはならない。

### 4.2 Checked claim

canonical executable contract は次の 3 形式である。

- `:case`: concrete な actual / expected comparison;
- `:assert`: module scope の Boolean proposition;
- `:property`: owner function を実際に呼ぶ sampled universal property。

claim には次の 2 状態がある。

- **well-formed**: 構造・scope・型が妥当である。
- **verified**: current fingerprint に対する contract runner evidence が pass している。

well-formed だけの claim を verified と表示してはならない。

### 4.3 Attested intent

自然言語の目的・背景は次へ格納する。

- `:doc`: public purpose の簡潔な説明。
- `:rationale`: 境界判断、trade-off、non-obvious な設計理由。

compiler が検査できるのは presence、content hash、reconciliation/attestation の provenance、signature、
fingerprint binding までである。内容の真偽は証明できないため、generated output は次を区別する。

- `authored`: current API / contract との reconciliation evidence がない。
- `agent-reconciled`: agent が current API / contract / intent fingerprint に対して更新または再確認した。
- `attested`: trusted human が current API / contract / intent fingerprint を review した。

いずれも prose 自体を `checked` や `proved` と表示してはならない。

### 4.4 Presentation metadata

次は presentation 専用である。

- `:example`;
- `:since`;
- `:see-also`。

`:example` は parse・type-check するが expected outcome を持たない。そのため executable contract
coverage には数えない。

## 5. Source language contract

canonical source form、owner binding、constrained type、typestate、legacy migration の正確な意味は
[`semantic-contract-language.md`](./semantic-contract-language.md) が定義する。

実装は同文書の form をそのまま semantic model へ取り込む。別の contract DSL を追加したり、
generated prose から意味を復元したりしてはならない。

## 6. Canonical Semantic Model

### 6.1 Package snapshot

logical model は次である。

```rust
struct SemanticPackageSnapshot {
    schema_version: String,
    semantics_version: String,
    package: PackageIdentity,
    compiler: ToolchainIdentity,
    symbols: BTreeMap<SymbolId, SemanticSymbol>,
    package_fingerprint: Digest,
}

struct SemanticSymbol {
    id: SymbolId,
    kind: SymbolKind,
    visibility: Visibility,
    static_facts: StaticFacts,
    contracts: ContractSuite,
    intent: IntentContent,
    presentation: PresentationContent,
    relations: Vec<SemanticRelation>,
    fingerprints: SymbolFingerprints,
    provenance: SourceProvenance,
}
```

canonical model は parser-only name や raw source text ではなく、resolved symbol と inferred type を
保持しなければならない。

### 6.2 Symbol identity

`SymbolId` は次を含む。

- package identity;
- fully qualified module path;
- symbol name;
- namespace が衝突し得る場合の symbol kind。

absolute path、span、timestamp は identity に含めない。public symbol の rename / move は explicit な
compatibility alias がない限り breaking change である。heuristic rename detection は advisory にだけ使い、
compatibility 判定を緩めてはならない。

### 6.3 Semantic relation

version 1 で許可する relation kind は次だけである。

```rust
enum SemanticRelation {
    HasType { symbol: SymbolId, ty: CanonicalType },
    Accepts { function: SymbolId, position: u32, ty: CanonicalType },
    Returns { function: SymbolId, ty: CanonicalType },
    Constructs { constructor: SymbolId, ty: SymbolId },
    Implements { ty: SymbolId, trait_id: SymbolId },
    RequiresTrait { symbol: SymbolId, trait_id: SymbolId },
    Refines { constrained: SymbolId, base: CanonicalType },
    Transitions { function: SymbolId, from: CanonicalType, to: CanonicalType },
    DependsOn { symbol: SymbolId, dependency: SymbolId },
}
```

各 relation は、どの static fact または checked claim から導出したかを provenance として持つ。
source から arbitrary relation edge を記述する API は設けない。

### 6.4 Closed-world boundary

snapshot は「compiler と accepted contract system が package について把握している内容」だけを表す。
edge が存在しないことは現実世界で false であることを意味しない。ontology projection は必ず
`closedWorld: true` とこの境界を表示する。

## 7. Canonicalization と fingerprint

### 7.1 Fingerprint axis

symbol ごとに 5 種類を独立して保持する。

```rust
struct SymbolFingerprints {
    api: Digest,
    contract: Digest,
    intent: Digest,
    implementation: Digest,
    presentation: Digest,
}
```

`api` に含めるもの:

- identity、kind、visibility;
- alpha-normalized type scheme;
- public ADT / record shape;
- trait requirements;
- constrained predicate と enforcement mode;
- derived typestate surface;
- public error surface。

`contract` に含めるもの:

- canonical `:case`、`:assert`、`:property` AST;
- owner binding;
- sampling parameter と generator version;
- behavioral claim として使う constraint predicate。

`intent` に含めるもの:

- normalized `:doc`;
- normalized `:rationale`;
- 将来 schema が導入する intent identifier。

`implementation` に含めるもの:

- symbol 自身の normalized typed body digest;
- execution に影響する resolved dependency graph;
- dependency graph の SCC を用いて計算した effective implementation closure digest。

補助関数、trait implementation、constructor、runtime-visible dependency の実装が変われば、それを実行し得る
caller の effective implementation fingerprint も変わらなければならない。単に dependency edge が同じであることを
理由に旧 contract evidence を再利用してはならない。

`presentation` に含めるもの:

- `:example`;
- `:since`;
- `:see-also`。

### 7.2 Canonical byte encoding

hash input は `lsharp-semantic-canonical-v1` で生成する。

1. UTF-8 を使う。
2. field order は schema が固定し、serializer 任せにしない。
3. map key は canonical UTF-8 bytes で sort する。
4. semantics を持つ list order は保持する。
5. set、trait requirement set、unordered relation set は sort する。
6. type variable は first occurrence 順に `t0`, `t1`, ... へ alpha-normalize する。
7. whitespace、comment、formatting、span、timestamp、absolute path を除外する。
8. integer は canonical base-10 text にする。
9. version 1 fingerprint payload に floating-point value を入れない。
10. string escaping は JSON-compatible な単一実装を使う。
11. payload の前に `lsharp-semantic-v1\0` と axis domain tag を付ける。
12. digest は SHA-256 とする。

human-readable JSON は projection である。任意の pretty-printed JSON bytes を hash してはならない。

### 7.3 Effective implementation closure

execution dependency graph を strongly connected component (SCC) へ縮約し、condensation DAG の末端から
effective digest を計算する。各 SCC の digest は次を canonical encode して SHA-256 する。

1. SCC member の `SymbolId` と own-body digest の sorted pair;
2. outgoing SCC の effective digest の sorted set;
3. runtime / trait dispatch で解決済みの execution dependency identity。

各 symbol の implementation axis は own-body digest と所属 SCC の effective digestを含む。再帰 SCC 内のどの
member を変更しても SCC 全 member と全 transitive caller の implementation fingerprint が変わる。動的 dispatch
または外部実行境界を閉じられない場合は dependency closure を `Unknown` とし、旧 evidence を再利用しない。

### 7.4 Package fingerprint

package fingerprint は `SymbolId` 順に並べた各 symbol の 5 digest から Merkle-style に生成する。
unchanged symbol の evidence reuse は、evidence rule が参照する全 fingerprint と producer/toolchain rule が
一致した場合だけ許可する。

## 8. Semantic delta

一つの変更が複数 axis を変更し得るため、exclusive enum に畳み込まない。

```rust
struct SemanticDelta {
    subject: SymbolId,
    api: ApiDelta,
    contract: ContentDelta,
    intent: ContentDelta,
    implementation: ContentDelta,
    presentation: ContentDelta,
    confidence: DeltaConfidence,
}

enum ApiDelta {
    Unchanged,
    AddedCompatible,
    ChangedCompatible,
    Breaking,
    Unknown,
}

enum ContentDelta {
    Unchanged,
    Added,
    Removed,
    Changed,
    Unknown,
}
```

最低限の API classification は次である。

| 変更 | classification |
|---|---|
| private symbol 追加 | API unchanged |
| public symbol 追加 | added compatible |
| public symbol 削除 | breaking |
| public symbol rename / move | breaking |
| public parameter の arity / order / type 変更 | breaking |
| public return type 変更 | breaking |
| ADT variant の追加・削除 | breaking |
| public record field の追加・削除・型変更 | breaking |
| public trait requirement 変更 | 専用 proof rule がない限り breaking |
| constrained predicate 変更 | breaking または unknown。compatible と仮定しない |
| formatting / comment だけ | 全 semantic axis unchanged |
| body だけ | implementation changed |

`Unknown` は unresolved obligation を発生させ、自動受理しない。

## 9. Policy profile

### 9.1 `typed`

必須:

- parse と type check;
- valid canonical snapshot;
- strict mode で unknown / legacy semantic form がないこと。

不要:

- owner contract coverage;
- intent prose;
- human attestation。

### 9.2 `checked`

`typed` に加えて、対象 public function へ次を要求する。public type / trait は static fact と
compatibility rule の対象だが、version 1 では executable owner coverage を要求しない。

- non-empty `:doc`;
- owner を cover する non-vacuous executable contract が 1 件以上;
- implementation または contract 変更後の current passing evidence;
- public API change に対する compatibility evidence;
- API または contract 変更後の `AgentIntentAttested` または `HumanIntentAttested` reconciliation evidence;
- error severity の unresolved obligation が 0 件。

### 9.3 `reviewed`

`checked` に加えて次を要求する。

- non-empty `:rationale`;
- API、contract、intent 変更後の trusted human attestation;
- breaking public API change に対する signed migration note;
- attestation producer の external trust-root validation。

### 9.4 Package configuration

形は固定する。

```toml
[semantic-contracts]
private-profile = "typed"
public-profile = "checked"
reviewed-symbols = ["Payment.*", "Auth.*"]
strict-legacy-metadata = true
```

`reviewed-symbols` は fully qualified `SymbolId` に対して照合する。複数 rule に一致した function には
最強 profile を適用する。version 1 の `reviewed-symbols` が non-function symbol に一致した場合は
configuration error とする。

section が存在しない既存 project は `legacy-unmanaged` である。通常 compile は許可するが、
`ChangeAccepted` を名乗れない。new project template は上記 section を生成しなければならない。

## 10. Obligation と evidence

### 10.1 Obligation model

```rust
struct Obligation {
    id: ObligationId,
    subject: SymbolId,
    rule: ObligationRuleId,
    cause: SemanticDelta,
    required_evidence: Vec<EvidenceRequirement>,
    severity: ObligationSeverity,
    source_spans: Vec<Span>,
}
```

`ObligationId` は rule identifier/version、subject、baseline/current fingerprint、policy profile から
決定的に生成する。

version 1 の rule:

- `SCS.RerunContracts.v1`;
- `SCS.AddOwnerCoverage.v1`;
- `SCS.ResolveLegacyMetadata.v1`;
- `SCS.VerifyCompatibility.v1`;
- `SCS.DocumentMigration.v1`;
- `SCS.ReconcileIntent.v1`;
- `SCS.ReviewIntent.v1`;
- `SCS.ProvidePurpose.v1`;
- `SCS.ProvideRationale.v1`;
- `SCS.ResolveAmbiguousDelta.v1`;
- `SCS.VerifyTargetParity.v1`。

### 10.2 Evidence model

```rust
struct EvidenceRecord {
    schema_version: String,
    evidence_id: EvidenceId,
    subject: SymbolId,
    kind: EvidenceKind,
    fingerprints: EvidenceFingerprints,
    producer: ProducerIdentity,
    toolchain: ToolchainIdentity,
    result: EvidenceResult,
    replay: ReplayDescriptor,
    signature: Option<SignatureEnvelope>,
}
```

version 1 の kind:

- `TypeChecked`;
- `CasesPassed`;
- `AssertionsPassed`;
- `PropertiesPassed`;
- `CompatibilityChecked`;
- `TargetParityChecked`;
- `MigrationNoteAttested`;
- `HumanIntentAttested`;
- `AgentIntentAttested`。

agent attestation は `checked` profile の `SCS.ReconcileIntent.v1` だけを閉じてよい。
`reviewed` profile の `SCS.ReviewIntent.v1`、human migration approval、その他の human evidence requirement を
閉じてはならない。

intent reconciliation evidence は baseline/current の API・contract・intent fingerprint、`updated` または
`affirmed` disposition、producer identity、summary digest を含む。`affirmed` は intent text が変わらないことを
許すが、現在の semantic delta を読んだという explicit evidence を必要とする。

### 10.3 Evidence validity

evidence が valid である条件:

1. schema が supported である。
2. subject が current snapshot に存在する。
3. requirement が参照する fingerprint が current value と一致する。
4. result が explicit success である。
5. producer と toolchain が policy を満たす。
6. executable evidence の replay information と dynamic execution trace が完全である。
7. executable evidence が参照する effective implementation closure fingerprint が current value と一致する。
8. 必要な signature が trusted key で検証できる。
9. fallback / unsupported path を使っていない。
10. target identity が requirement と一致する。

### 10.4 Attestation trust

human-required evidence は current change が置き換えられない trust root を使う。
version 1 は Ed25519 signed evidence envelope を採用する。trusted public key は次のいずれかから供給する。

- PR 外の CI trust store;
- signed baseline artifact;
- explicit local `--trust-store` path。

current worktree に追加した public key だけで、その同じ変更を self-approve してはならない。

## 11. Trusted baseline

change verification は trusted baseline を必須とする。

受理できる source:

- separate checkout の merge-base から生成した snapshot;
- signed release snapshot;
- trusted artifact store から取得し digest/signature を検証した artifact。

受理できない source:

- current worktree だけから再生成した snapshot;
- trust root 自体も current patch で変更した unsigned file;
- source/toolchain provenance が一致しない cache。

baseline なしの通常 compile は current source validity を示せるが、`main` に対する change acceptance を
示してはならない。

## 12. Acceptance pipeline

```text
source
  -> parse / resolve
  -> infer / type check
  -> canonical semantic snapshot
  -> canonical contract validation
  -> trusted baseline comparison
  -> multidimensional delta
  -> policy obligation derivation
  -> evidence validation / current contract execution
  -> open obligation derivation
  -> accept or fail-closed
  -> specification / API / graph / LLM context projection
```

状態を次のように区別する。

- `SourceValid`: current source が parse・type-check できる。
- `CurrentVerified`: current policy が要求する contract が pass した。
- `ChangeAccepted`: trusted baseline に対する obligation がすべて閉じた。
- `ReleaseReady`: repository-wide target、artifact、provenance gate まで pass した。

下位状態から上位状態を推論してはならない。

## 13. CLI contract

public flow は compile-centered とする。

current snapshot:

```bash
lsharp compile src/Main.ls \
  --semantic-profile checked \
  --emit-semantic target/lsharp/current.semantic.json
```

contract evidence:

```bash
lsharp test src/Main.ls \
  --semantic-profile checked \
  --emit-evidence target/lsharp/contracts.evidence.json
```

trusted baseline に対する verification:

```bash
lsharp compile src/Main.ls \
  --semantic-profile checked \
  --verify-against target/lsharp/base.semantic.json \
  --evidence target/lsharp/contracts.evidence.json \
  --emit-obligations target/lsharp/obligations.json
```

checked intent reconciliation:

```bash
lsharp attest intent \
  --semantic target/lsharp/current.semantic.json \
  --subject 'lsharp://example/Main/function/run' \
  --reviewer-kind agent \
  --disposition affirmed \
  --agent-run-id "$LUNA_RUN_ID" \
  --summary "API/contract delta に対して purpose を再確認" \
  --out target/lsharp/intent.reconciliation.json
```

projection:

```bash
lsharp doc \
  --semantic target/lsharp/current.semantic.json \
  --out target/lsharp/spec
```

snapshot が accepted でない場合、generated output は status と open obligation を明示する。
accepted output と同じ見た目にしてはならない。

## 14. LSP・MCP・LLM contract

LSP/MCP は compiler-owned result の adapter である。source text から独自に semantics を復元してはならない。

必須 operation:

- `lsharp_semantic_snapshot`;
- `lsharp_semantic_diff`;
- `lsharp_semantic_obligations`;
- `lsharp_contract_skeleton`;
- `lsharp_verify_change`。

response は schema version、fingerprint、baseline provenance、span 付き diagnostic、obligation、accepted
Boolean を含む。`accepted` は compiler が導出し、agent が指定できない。

LLM の正規 loop:

1. open obligation を読む。
2. source / contract / intent を変更する。
3. 指定された verification command を実行する。
4. machine-readable diagnostic を読む。
5. `ReconcileIntent` が open なら semantic delta と authored intent を照合し、更新または `affirmed` disposition を
   agent reconciliation evidence として出す。
6. machine / agent-closeable obligation が 0 件になるまで繰り返す。
7. human obligation が残ったら停止し、人間の attestation を要求する。

LLM は次をしてはならない。

- generated specification を直接修正する。
- obligation status を編集する。
- 自分の prose review を human evidence とする。
- current snapshot を trusted baseline に置き換える。
- unknown / unsupported result を隠す。
- type-check success だけで acceptance を宣言する。

## 15. Generated specification

symbol page は evidence strength を分離して表示する。

```markdown
# Bank.remaining-balance

Acceptance: accepted (`checked`)

## Signature — static

`(Int, Int) -> Int`

| Parameter | Type |
|---|---|
| `balance` | `Int` |
| `amount` | `Int` |

## Purpose — authored

Return the balance remaining after a permitted withdrawal.

## Behavior — checked

- `amount <= balance` のとき `result = balance - amount`。
- 256 cases、seed 0、generator `type-directed-splitmix64-v1`。

## Change status

- API: unchanged
- Contract: unchanged
- Implementation: changed and reverified
- Open obligations: none
```

ontology projection も同じ fact を node/edge で表してよい。ただし全 edge に provenance と assurance を
付与し、編集可能な model として扱わない。

## 16. Diagnostic

version 1 は次の stable code を予約する。

| code | 意味 |
|---|---|
| `LS3201` | deterministic な semantic snapshot を構築できない |
| `LS3202` | trusted baseline がない、または provenance が invalid |
| `LS3203` | snapshot schema / toolchain が incompatible |
| `LS3210` | unresolved semantic obligation |
| `LS3211` | semantic delta が ambiguous / unknown |
| `LS3212` | obligation と evidence の fingerprint mismatch |
| `LS3213` | producer、target、fallback provenance が invalid |
| `LS3220` | required public `:doc` がない / empty |
| `LS3221` | required public `:rationale` がない / empty |
| `LS3222` | human attestation がない / stale / untrusted |
| `LS3223` | checked intent reconciliation がない / stale |
| `LS3230` | owner executable contract coverage がない |
| `LS3231` | executable evidence がない / stale / failed |
| `LS3232` | property generator が binder type を support しない |
| `LS3233` | property sampling が required coverage / attempt を満たせない |
| `LS3234` | property binder が owner signature と一致しない |
| `LS3235` | case comparison に canonical equality support がない |
| `LS3236` | canonical executable contract の owner kind が version 1 で unsupported |
| `LS3237` | contract execution に unsupported / nondeterministic effect がある |
| `LS3238` | passing evidence の dynamic trace が owner coverage を証明しない |
| `LS3240` | strict mode で legacy duplicated metadata を使用した |
| `LS3250` | opt-in なしに unaccepted snapshot の projection を要求した |

diagnostic は subject `SymbolId`、可能な場合の source span、obligation ID、machine-readable remediation
category を含む。free-form message だけを API contract にしてはならない。

## 17. Anti-bypass requirement

次を negative test で拒否する。

- current snapshot で baseline bytes を置換する。
- current patch で trust store を変更して self-sign する。
- producer を再実行せず evidence fingerprint だけ書き換える。
- agent attestation で human obligation を閉じる。
- fallback path の結果を native evidence とする。
- body、contract、type、generator、policy 変更後に stale evidence を使う。
- contract を削除して以前の evidence を保持する。
- owner への dead-code/static-only reference を含む case で coverage を満たす。
- transitive helper implementation を変え、caller の旧 evidence を再利用する。
- API / contract 変更後に intent reconciliation を省略する。
- empty / tautological contract を coverage とする。
- timeout / unsupported generator を skipped success とする。
- generated Markdown を編集して open obligation を隠す。

## 18. Conformance criteria

1. 同一 input/schema に対して Rust と self-host が byte-equal な canonical vector を出す。
2. formatting / comment-only change は全 semantic fingerprint を保持する。
3. 各 axis は本文で定義した入力だけに反応する。
4. delta と obligation は deterministic かつ conservative である。
5. requirement が一致しない evidence は obligation を閉じない。
6. `typed`、`checked`、`reviewed` が定義どおりの acceptance outcome を返す。
7. generated document は provenance と evidence strength を表示する。
8. CLI、LSP、MCP は同じ snapshot・diagnostic・obligation を返す。
9. unsupported / unknown は Rust oracle と native lane の双方で明示的に fail-closed となる。
10. helper / trait implementation の変更は transitive caller の executable evidence を stale にする。
11. checked API / contract change は agent または human reconciliation、reviewed change は human attestation を要求する。
12. claimed public surface は必要な Mac Apple Silicon / Linux x86_64 evidence を持つ。

実装の絶対方針は
[`../development/specs/semantic-contract-system/README.md`](../development/specs/semantic-contract-system/README.md)、
検証項目は同 directory の `test-matrix.md` が定義する。
