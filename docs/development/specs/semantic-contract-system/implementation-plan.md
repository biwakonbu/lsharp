# Semantic Contract System 実装計画

状態: 実装上の正本
architecture: [`README.md`](./README.md)
受入試験: [`test-matrix.md`](./test-matrix.md)
agent 実行契約: [`agent-execution-guide.md`](./agent-execution-guide.md)

本書は実装順序と evidence requirement を固定する。code が存在することや graph が生成できることではなく、
指定 test と evidence が揃った時点で work package を完了とする。

## 1. Work package

### WP0 — Schema・diagnostic・fixture の固定

対象:

- 本 directory の規範文書;
- `tests/fixtures/semantic/v1/`;
- diagnostic registry;
- snapshot / obligation / evidence の JSON schema。

RED:

- `SCS-CAN-001`、`SCS-CAN-002`、`SCS-DIA-001`、`SCS-ART-001`。

GREEN criteria:

- declared schema version だけを受理する。
- reserved diagnostic code が一つの stable category に対応する。
- malformed / duplicate canonical field を拒否する。

### WP1 — Canonical source-contract semantics

対象:

- `crates/lsharp-types/src/metadata_contract.rs`;
- `crates/lsharp-types/src/canonical_contract_check.rs`;
- focused test module;
- existing `test` command が使う contract runner path。

RED:

- owner-bound `result` type;
- binder arity/type mismatch;
- static candidate と dynamic owner coverage の分離;
- deterministic effect boundary;
- structural case comparison;
- closed constant vacuity evaluator;
- unsupported generator、shrink、attempt-limit failure。

GREEN criteria:

- `SCS-CON-*`、`SCS-COV-*`、`SCS-GEN-*` が pass。
- legacy inventory が lossless のまま。
- host fallback を property success に数えない。

### WP2 — Semantic snapshot / canonicalization

対象:

- `semantic/model.rs`、`symbol.rs`、`builder.rs`、`canonical.rs`;
- fixture snapshot。

RED:

- function、ADT/GADT、record、constrained type、trait の static fact extraction;
- fully qualified symbol identity;
- formatting / comment / alpha rename stability;
- path / span / timestamp exclusion;
- deterministic ordering;
- helper / recursive SCC change による transitive caller fingerprint invalidation。

GREEN criteria:

- `SCS-MOD-*` と `SCS-CAN-*` が byte-for-byte match。
- error 時に partial trusted snapshot を返さない。

### WP3 — Fingerprint / delta classifier

対象:

- `semantic/fingerprint.rs`;
- `semantic/diff.rs`。

RED:

- one-axis change;
- multi-axis change;
- public compatibility table;
- unknown schema/type behavior。

GREEN criteria:

- `SCS-FP-*`、`SCS-DIFF-*` が pass。
- unchanged symbol が evidence identity を保持する。

### WP4 — Policy / obligation / evidence / trust

対象:

- `semantic/policy.rs`、`obligation.rs`、`evidence.rs`、`diagnostic.rs`;
- driver trust-store loader。

RED:

- 3 profile の requirement;
- checked `ReconcileIntent` と reviewed `ReviewIntent` の権限分離;
- deterministic obligation ID;
- stale fingerprint rejection;
- agent/human privilege separation;
- trust-root replacement attack;
- baseline replacement attack;
- fallback evidence rejection。

GREEN criteria:

- `SCS-POL-*`、`SCS-OBL-*`、`SCS-EVD-*`、`SCS-SEC-*` が pass。

### WP5 — Driver / artifact integration

対象:

- compile/test command flag;
- `crates/lsharp-driver/src/semantic/*`;
- end-to-end fixture。

RED:

- compile snapshot output;
- trusted verification exit status;
- atomic artifact failure;
- open-obligation JSON;
- failed contract evidence;
- unmanaged project status。

GREEN criteria:

- `SCS-CLI-*`、`SCS-ART-*` が pass。
- acceptance を要求しない既存 compile path は observable behavior を維持する。
- requested acceptance failure を output success と報告しない。

### WP6 — Specification / ontology projection

対象:

- `crates/lsharp-docs/src/semantic/*`;
- generated snapshot fixture。

RED:

- assurance label;
- derived signature / constraint;
- typestate graph;
- unaccepted banner / open obligation;
- arbitrary graph input rejection。

GREEN criteria:

- `SCS-DOC-*`、`SCS-ONT-*` が pass。

### WP7 — LSP / MCP parity

対象:

- shared semantic service;
- LSP custom request;
- MCP handler。既存 server file が size guidance を超える場合は先に分割する。

RED:

- response schema;
- source span;
- CLI と diagnostic/obligation parity;
- skeleton が evidence でないこと;
- unknown/tool failure propagation。

GREEN criteria:

- `SCS-LSP-*`、`SCS-MCP-*` が structurally equal response で pass。

### WP8 — Self-host parity / native evidence

対象:

- architecture 文書で指定した self-host module;
- shared canonical vector fixture;
- native stage0 test。

RED:

- 実装前の native lane が同じ fixture で fail、または explicit unsupported diagnostic を返す。
- fallback による fake pass を negative test で検出する。

GREEN criteria:

- canonical bytes / fingerprint が Rust oracle と equal。
- contract result / diagnostic が Rust oracle と equal。
- native evidence の成功経路に Rust/host fallback がない。
- claimed public surface の Mac Apple Silicon / Linux x86_64 gate が pass。

### WP9 — Strict migration / default project template

対象:

- metadata migration diagnostic/fix;
- new project configuration template;
- language reference / example;
- active TODO entry と verified completion 後の ADR。

RED:

- strict `:params`、prose `:returns`、`:invariant`、`:transitions` rejection;
- explicit migration output;
- unmanaged project が accepted change を名乗れないこと。

GREEN criteria:

- `SCS-MIG-*` が pass。
- new project は managed config を default で持つ。
- existing unmanaged project は compile できるが migration 前は visibly unaccepted。

## 2. Required TDD execution pattern

WP0〜WP7 の GREEN は Rust oracle lane の work-package completion を意味する。NAT/MAC/LNX column を含む
subsystem conformance は WP8 で閉じる。それ以前に native parity を完了と表示してはならない。

各 work package で次を順番どおり実行する。

1. test matrix と同じ fixture を使う Rust oracle RED を追加する。
2. production implementation 前に native fixture を追加・登録する。
3. focused Rust test を実行し、期待する failure value/boundary を記録する。
4. native stage0 を実行し、同じ failure または explicit unsupported diagnostic を記録する。
5. RED を閉じる最小 production behavior を実装する。
6. focused Rust GREEN を実行する。
7. scope に応じて self-host source `check` と native stage0 GREEN を実行する。
8. Rust/self-host differential output を比較する。
9. public behavior を変更した場合は artifact/runtime/support target gate を実行する。
10. verified evidence だけを docs / ADR / active TODO truth へ反映する。
11. format、lint、workspace test、`git diff --check` を実行する。
12. task-relevant file だけを commit する。

baseline command。実際の test target 名に合わせる以外は scope を狭めない。

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

Rust-only pass は native evidence ではない。fallback を使用した native command も native evidence ではない。

## 3. Pull-request slicing

WP0〜WP9 を一つの code PR へまとめない。次の順で分割する。

1. schema / fixture と contract semantics;
2. snapshot / canonicalization / effective dependency fingerprint;
3. delta / policy / obligation / evidence;
4. driver artifact / CLI;
5. docs projection;
6. LSP / MCP;
7. self-host / native parity;
8. strict migration / default rollout。

各 PR body は次を明記する。

- 追加した test ID;
- RED command と observed failure;
- GREEN command;
- Rust evidence と native evidence の区別;
- unsupported boundary;
- generated artifact が source truth でないこと;
- remaining work package ID。

## 4. Definition of Done

subsystem 全体の完了条件:

- `test-matrix.md` の mandatory test が必要 layer ですべて実装・pass。
- 5 fingerprint と effective implementation closure に Rust/self-host canonical vector がある。
- owner-bound property が実 owner を呼び、`result` を owner return type として検査する。
- dynamic owner coverage、deterministic effect boundary、3 profile、全 anti-bypass rule が enforcement される。
- checked change の agent reconciliation と reviewed change の human attestation を混同しない。
- current change が trusted baseline を置換できない。
- agent attestation が human review を閉じられない。
- CLI / LSP / MCP / docs / ontology が同じ compiler snapshot を使う。
- generated spec が assurance strength と open obligation を表示する。
- supported slice で Rust/self-host/native output が equivalent。
- 必要な Mac Apple Silicon / Linux x86_64 evidence がある。
- active TODO は remaining work だけを保持し、completed decision は ADR へ移動済み。
- approved split なしに file size guidance を超えない。
- fallback、skipped test、stale artifact、prose assertion を proof として数えない。

successful demo、生成 graph、もっともらしい LLM output は completion evidence ではない。
