# Semantic Contract System Agent 実行契約

状態: 実装 agent 向け規範的 playbook
対象: GPT-5.6 Luna を含む coding agent
architecture: [`README.md`](./README.md)
実装順序: [`implementation-plan.md`](./implementation-plan.md)
受入試験: [`test-matrix.md`](./test-matrix.md)
運用例: [`operation-example.md`](./operation-example.md)

本書は、agent が Semantic Contract System を独自設計へ変形せず、TDD と証跡を保ったまま実装するための
入出力・停止条件を固定する。prompt の一般的な「最善を尽くす」指示より本書の具体要件を優先する。

## 1. Authority order

矛盾を発見した場合は次の順で解決する。下位文書を都合よく解釈して上位契約を弱めてはならない。

1. repository root と対象 directory の `AGENTS.md`。
2. `docs/language/semantic-contract-system.md`。
3. `docs/language/semantic-contract-language.md`。
4. 本 directory の `README.md`。
5. `test-matrix.md`。
6. `implementation-plan.md`。
7. `operation-example.md` と本書。
8. current production implementation。

production implementation が規範文書と異なる場合、current behavior を仕様と推定しない。RED test で差を固定し、
規範へ合わせる。規範同士が矛盾する場合は実装を開始せず、矛盾箇所・選択肢・影響 test ID を報告する。

## 2. Session input

各実装 session は最低限次を入力として持つ。

```yaml
semantic_contract_session:
  spec: lsharp-semantic-snapshot-v1
  work_package: WP0
  target_test_ids:
    - SCS-CAN-001
    - SCS-CAN-002
  repository_root: .
  baseline_ref: origin/main
  rust_lane: required
  native_lane: fixture-first
  supported_targets:
    - aarch64-apple-darwin
    - x86_64-unknown-linux-gnu
```

`work_package` または `target_test_ids` がない場合、agent は `implementation-plan.md` の最初の未完 package から、
依存を持たない最小 test slice を選ぶ。複数 work package を同時実装しない。

## 3. Session start audit

最初の production edit より前に次を実行・記録する。

```bash
git status --short --branch
git rev-parse --show-toplevel
git rev-parse HEAD
git rev-parse --verify origin/main
find .. -name AGENTS.md -print
```

続いて次を確認する。

- user の既存差分と task 対象差分を区別した。
- current branch / upstream / merge-base を確認した。
- relevant `AGENTS.md` を読んだ。
- target test ID が `test-matrix.md` に存在する。
- prerequisite work package が完了しているか、明示的な fixture-only dependency である。
- native stage0、target artifact、VM job の current provenance を確認した。
- stale artifact や過去の報告を current evidence として再利用していない。

既存差分を reset、stash、reformat、削除してはならない。task と無関係な failure を修正する場合は別 task として
分離する。

## 4. Traceability table

実装前に session 内で次の表を作る。

| requirement | source | test ID | production boundary | expected failure |
|---|---|---|---|---|
| canonical field order | language §7.2 | `SCS-CAN-001` | `semantic/canonical.rs` | byte mismatch |

一つの production change は少なくとも一つの requirement と test ID に追跡可能でなければならない。
「将来必要そう」「きれいになる」という理由だけの変更を含めない。

## 5. Mandatory execution loop

### 5.1 RED

1. fixture を追加または既存 canonical fixture を選ぶ。
2. Rust oracle test を追加する。
3. self-host/native が同 fixture を読める registration を production 実装より先に追加する。
4. focused command を実行する。
5. failure value、diagnostic code/span、exit code、artifact boundary を記録する。

compile failure、test not found、fixture missing、panic だけでは semantic RED にならない。対象 requirement が未実装である
ことを示す failure を得る。

### 5.2 GREEN

1. RED を閉じる最小 production code を実装する。
2. test expectation を current output へ合わせて変更しない。
3. unknown / unsupported は explicit diagnostic にする。
4. Rust fallback、host equality、prose judge、generated file parse で shortcut しない。
5. focused Rust test を通す。
6. scope が存在する場合は self-host source `check` と native fixture を通す。

### 5.3 REFACTOR

- behavior と canonical bytes を保持する。
- file size guidance を超える前に規定 module へ分割する。
- semantic rule を driver、docs、MCP adapter へ複製しない。
- shared service と typed model を使う。

### 5.4 EVIDENCE

実行した command ごとに次を記録する。

```yaml
- test_id: SCS-CAN-001
  command: cargo test -p lsharp-types semantic_format_is_canonical
  lane: rust-oracle
  target: host
  result: pass
  fallback_used: false
  commit: <current-head>
  fixture_digest: <sha256>
```

native lane が未実装なら `pass` と書かず、`unsupported` と diagnostic を記録する。WP8 より前の Rust GREEN は
work-package oracle completion であり subsystem conformance ではない。

## 6. Semantic decisions agent must not invent

次は固定済みであり、別案を実装しない。

- editable ontology store や arbitrary triple authoring を追加しない。
- `:params` / prose `:returns` を第二の signature にしない。
- `:property` の `result` を unconstrained synthetic variable にしない。
- case/assert owner coverage を static reference だけで確定しない。
- helper implementation change後に caller evidence を再利用しない。
- checked API / contract change の intent reconciliation を省略しない。
- agent reconciliation で reviewed human obligation を閉じない。
- obligation に mutable `closed` state を保存しない。
- source/current snapshot を trusted baseline として self-approve しない。
- `serde_json` の任意 serialization や `Display` text を hash しない。
- timeout、skip、unsupported effect、fallback を pass と表現しない。

## 7. Intent update protocol

### 7.1 Checked symbol

API または contract が変わった場合:

1. current delta を取得する。
2. `:doc` が新しい public purpose と矛盾しないか読む。
3. 必要なら `:doc` を更新する。
4. current snapshot を生成する。
5. `updated` または `affirmed` disposition の agent reconciliation evidence を出す。
6. evidence を付けて verification を再実行する。

`affirmed` は「変更不要と判断した」という explicit evidence であり、自動 default にしない。summary は semantic delta
との関係を短く記述する。空文字、定型文だけ、current fingerprint に bind しない record は `LS3223` とする。

### 7.2 Reviewed symbol

agent は source、contract、`:doc`、`:rationale`、migration note の候補を更新してよい。しかし
`SCS.ReviewIntent.v1` が残った時点で停止し、subject、delta、必要 evidence、実行済み machine evidence を人間へ渡す。
agent key、repository 内 key、current patch で追加した key で human requirement を閉じない。

## 8. Per-work-package work order

| WP | first implementation unit | required initial tests |
|---|---|---|
| WP0 | schema newtype と canonical artifact decoder | `SCS-CAN-007`, `SCS-ART-004` |
| WP1 | owner-bound property type check | `SCS-CON-014`, `SCS-CON-017`, `SCS-CON-018` |
| WP2 | `SymbolId` と function static fact | `SCS-MOD-001`, `SCS-MOD-007`, `SCS-CAN-001` |
| WP3 | axis payload と effective implementation closure | `SCS-FP-004`, `SCS-FP-013`, `SCS-FP-016` |
| WP4 | pure obligation derivation と evidence validation | `SCS-OBL-001`, `SCS-EVD-004`, `SCS-EVD-010` |
| WP5 | `--emit-semantic` atomic artifact | `SCS-CLI-001`, `SCS-ART-001` |
| WP6 | assurance-separated symbol page | `SCS-DOC-001`, `SCS-DOC-009`, `SCS-ONT-002` |
| WP7 | shared service response parity | `SCS-LSP-002`, `SCS-MCP-008` |
| WP8 | shared vectors の self-host/native parity | `SCS-NAT-001`, `SCS-NAT-002`, `SCS-NAT-006` |
| WP9 | strict legacy rejection と migration suggestion | `SCS-MIG-001`, `SCS-MIG-004`, `SCS-MIG-007` |

表の unit を飛ばして high-level UI や graph から実装しない。

## 9. Required session output

各 session の終了時に次を返す。

```markdown
## 実装結果

- Work package: WPx
- Implemented test IDs: ...
- Changed production boundaries: ...

## RED evidence

- Command: ...
- Observed failure: ...

## GREEN evidence

- Rust oracle: ...
- Self-host source check: ...
- Native stage0: ...
- Mac target: ...
- Linux target: ...
- Fallback used: false/true

## Remaining obligations

- ...

## Unsupported / unverified boundary

- ...

## Next exact RED

- Test ID: ...
- Command: ...
- Expected failure: ...
```

実行していない command を success と書かない。環境上実行不能なら、理由、未検証 scope、再現 command を記録する。

## 10. Stop conditions

次の場合は推測で継続せず停止境界を明示する。

- 規範文書同士が矛盾し、observable behavior が一意に決まらない。
- reviewed human attestation が必要になった。
- trusted baseline / trust store がなく change acceptance を評価できない。
- supported target の native evidence を要求されるが target が利用できない。
- current user diff と task diff を安全に分離できない。
- required dependency edge や effect capability を compiler が解決できず `Unknown` になる。

停止は task 完了を意味しない。partial implementation を完了扱いせず、次の exact RED と blocker を出す。

## 11. Prompt template for Luna

```text
Semantic Contract System の <WPx / test IDs> を実装してください。

必ず次を正本として順に読んでください:
1. AGENTS.md
2. docs/language/semantic-contract-system.md
3. docs/language/semantic-contract-language.md
4. docs/development/specs/semantic-contract-system/README.md
5. docs/development/specs/semantic-contract-system/test-matrix.md
6. docs/development/specs/semantic-contract-system/implementation-plan.md
7. docs/development/specs/semantic-contract-system/agent-execution-guide.md

production edit 前に traceability table と RED を作り、focused Rust oracle、self-host/native lane、
fallback boundary を別々に報告してください。仕様にない semantic choice を発見した場合は暗黙に決めず停止し、
該当箇所と影響 test ID を提示してください。task-relevant files 以外を変更しないでください。
```

## 12. Completion criterion

agent の説明がもっともらしいこと、graph が表示できること、document が生成できることは completion ではない。
対象 test ID の RED/GREEN、fingerprint/evidence provenance、必要 lane/target の結果、open obligation の監査が揃った場合だけ
対象 slice を完了とする。subsystem 全体の完了は `implementation-plan.md` の Definition of Done に従う。
