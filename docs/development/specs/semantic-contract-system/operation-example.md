# Semantic Contract System 運用例

状態: 規範的な運用 playbook
architecture: [`README.md`](./README.md)
言語 form: [`../../../language/semantic-contract-language.md`](../../../language/semantic-contract-language.md)

本書は、developer / LLM agent / compiler / reviewer が何を更新し、何を自動導出し、どの時点で変更が
受理されるかを end-to-end で示す。

## 1. 誰が何を管理するか

| 対象 | author / owner | 更新方法 |
|---|---|---|
| type、body、canonical contract | developer または LLM | `.ls` source を編集 |
| `:doc`、`:rationale` | developer または LLM | `.ls` source を編集 |
| parameter / return / variant / field / transition | compiler | type-checked source から導出 |
| semantic snapshot / fingerprint | compiler | command 実行ごとに再生成 |
| obligation | compiler | trusted baseline と current snapshot から導出 |
| executable evidence | contract runner | current contract を実行して生成 |
| human attestation | trusted reviewer | external private key で署名 |
| Markdown spec / API JSON / ontology graph | projection tool | snapshot から一方向生成 |

LLM は source と authored intent を編集してよい。snapshot、obligation status、generated spec、human
signature を編集して変更を通過させてはならない。

## 2. Project policy

`lsharp.toml`:

```toml
[semantic-contracts]
private-profile = "typed"
public-profile = "checked"
reviewed-symbols = ["lsharp://bank/Bank/function/withdraw"]
strict-legacy-metadata = true
```

ここでは通常 public function は `checked`、`Bank.withdraw` だけは `reviewed` である。

## 3. Baseline source

```lisp
(module Bank)

(defn remaining-balance
  [(: balance Int) (: amount Int)]
  : Int
  :doc "Return the balance remaining after a permitted withdrawal."
  :case [
    (expect (remaining-balance 1000 200) 800)
    (expect (remaining-balance 1 1) 0)
  ]
  :property [
    (for-all [balance Int amount Int]
      :precondition [(>= balance 0) (> amount 0) (<= amount balance)]
      :postcondition (= result (- balance amount))
      :cases 256
      :seed 0
      :shrink true)
  ]
  (- balance amount))

(defn withdraw
  [(: balance Int) (: amount Int)]
  : Int
  :doc "Apply an authorized withdrawal to an account balance."
  :rationale "Authorization and persistence are separate boundary operations."
  :case [
    (expect (withdraw 1000 200) 800)
  ]
  :property [
    (for-all [balance Int amount Int]
      :precondition [(>= balance 0) (> amount 0) (<= amount balance)]
      :postcondition (= result (- balance amount)))
  ]
  (- balance amount))
```

baseline を current worktree とは別 checkout で生成する。

```bash
lsharp compile src/Bank.ls \
  --semantic-profile checked \
  --emit-semantic target/lsharp/base.semantic.json
```

PR/change verification 側は、この artifact の digest と provenance を trusted input として受け取る。
current branch が同 path を上書きしても trust は得られない。

## 4. Case A — implementation-only change

### 4.1 LLM が source を refactor する

```lisp
(defn remaining-balance
  [(: balance Int) (: amount Int)]
  : Int
  :doc "Return the balance remaining after a permitted withdrawal."
  :case [
    (expect (remaining-balance 1000 200) 800)
    (expect (remaining-balance 1 1) 0)
  ]
  :property [
    (for-all [balance Int amount Int]
      :precondition [(>= balance 0) (> amount 0) (<= amount balance)]
      :postcondition (= result (- balance amount))
      :cases 256
      :seed 0
      :shrink true)
  ]
  (let [(next (- balance amount))]
    next))
```

API、contract、intent は同じで implementation fingerprint だけが変わる。

### 4.2 Evidence なしで verification する

```bash
lsharp compile src/Bank.ls \
  --verify-against target/lsharp/base.semantic.json \
  --semantic-profile checked \
  --emit-obligations target/lsharp/obligations.json
```

exit は nonzero。概念上の output:

```json
{
  "accepted": false,
  "open": [
    {
      "rule": "SCS.RerunContracts.v1",
      "subject": "lsharp://bank/Bank/function/remaining-balance",
      "diagnostic": "LS3231",
      "requiredEvidence": ["CasesPassed", "PropertiesPassed"]
    }
  ]
}
```

compiler は「contract text が変わっていないから正しい」とは判断しない。

### 4.3 Current contract を実行する

```bash
lsharp test src/Bank.ls \
  --semantic-profile checked \
  --emit-evidence target/lsharp/contracts.evidence.json
```

runner は case と property を current implementation に対して実行し、current API / contract /
implementation fingerprint に bind した evidence を出す。

### 4.4 Evidence を使って再 verification する

```bash
lsharp compile src/Bank.ls \
  --verify-against target/lsharp/base.semantic.json \
  --semantic-profile checked \
  --evidence target/lsharp/contracts.evidence.json \
  --emit-obligations target/lsharp/obligations.json
```

結果:

```json
{
  "accepted": true,
  "open": [],
  "closed": [
    {
      "rule": "SCS.RerunContracts.v1",
      "evidenceKinds": ["CasesPassed", "PropertiesPassed"]
    }
  ]
}
```

parameter table、return type、spec page、ontology graph は compiler projection なので manual update は不要。

## 5. Case B — implementation bug

LLM が body を誤って次へ変える。

```lisp
(+ balance amount)
```

verification 前は Case A と同じ `RerunContracts` が open になる。`lsharp test` を実行すると concrete case と
property が fail し、failed evidence が生成される。

次は概念上の失敗 evidence である。actual replay record は fixed generator vector から runner が生成する。

```json
{
  "kind": "PropertiesPassed",
  "result": {
    "status": "failed",
    "diagnostics": ["property counterexample"]
  },
  "replay": {
    "seed": 0,
    "generatorVersion": "type-directed-splitmix64-v1",
    "counterexampleRecorded": true
  }
}
```

failed evidence は obligation を閉じない。LLM が contract を `(+ balance amount)` に合わせて変更すれば
contract fingerprint も変わり、reviewed symbol なら `ReviewIntent` が追加される。単に test を都合よく
書き換えることは silent success にならない。

## 6. Case C — checked contract change と LLM reconciliation

`remaining-balance` の仕様を「overdraft は 0 へ clamp する」へ変更する。signature は同じだが、checked claim、
implementation、authored purpose が変わる。

```lisp
(defn remaining-balance
  [(: balance Int) (: amount Int)]
  : Int
  :doc "Return a non-negative balance after applying a requested withdrawal."
  :case [
    (expect (remaining-balance 1000 200) 800)
    (expect (remaining-balance 100 120) 0)
  ]
  :property [
    (for-all [balance Int amount Int]
      :precondition [(>= balance 0) (> amount 0)]
      :postcondition
        (= result (if (> amount balance) 0 (- balance amount))))
  ]
  (if (> amount balance)
      0
      (- balance amount)))
```

current contract を実行し、current snapshot を出す。

```bash
lsharp test src/Bank.ls \
  --semantic-profile checked \
  --emit-evidence target/lsharp/contracts.evidence.json

lsharp compile src/Bank.ls \
  --semantic-profile checked \
  --emit-semantic target/lsharp/current.semantic.json \
  --verify-against target/lsharp/base.semantic.json \
  --evidence target/lsharp/contracts.evidence.json \
  --emit-obligations target/lsharp/obligations.json
```

contract evidence が pass しても、結果は `accepted=false` で `SCS.ReconcileIntent.v1` が残る。
これは prose の真偽を compiler が推測するためではなく、API / contract 変更後に authored intent を無検討のまま
fresh と扱わないためである。

LLM agent は delta と `:doc` を照合し、今回は実際に `:doc` を更新したため `updated` disposition を出す。

```bash
lsharp attest intent \
  --semantic target/lsharp/current.semantic.json \
  --subject 'lsharp://bank/Bank/function/remaining-balance' \
  --reviewer-kind agent \
  --disposition updated \
  --agent-run-id "$LUNA_RUN_ID" \
  --summary "overdraft clamp contract と purpose を同期" \
  --out target/lsharp/remaining-balance.reconciliation.json
```

再 verification:

```bash
lsharp compile src/Bank.ls \
  --semantic-profile checked \
  --verify-against target/lsharp/base.semantic.json \
  --evidence target/lsharp/contracts.evidence.json \
  --evidence target/lsharp/remaining-balance.reconciliation.json \
  --emit-obligations target/lsharp/obligations.json
```

この evidence は `ReconcileIntent` を閉じるが、generated specification は purpose を
`agent-reconciled` と表示し、`attested` や `checked prose` とは表示しない。`reviewed` symbol に同じ evidence を
渡しても human obligation は閉じない。

## 7. Case D — breaking reviewed API change

### 7.1 LLM が fee parameter を追加する

```lisp
(defn withdraw
  [(: balance Int) (: amount Int) (: fee Int)]
  : Int
  :doc "Apply an authorized withdrawal and fee to an account balance."
  :rationale "Fee application is part of the same atomic balance transition."
  :case [
    (expect (withdraw 1000 200 10) 790)
  ]
  :property [
    (for-all [balance Int amount Int fee Int]
      :precondition [
        (>= balance 0)
        (> amount 0)
        (>= fee 0)
        (<= (+ amount fee) balance)
      ]
      :postcondition (= result (- balance (+ amount fee))))
  ]
  (- balance (+ amount fee)))
```

compiler は次を同時に検出する。

- API: breaking;
- contract: changed;
- intent: changed;
- implementation: changed。

### 7.2 Machine evidence だけを揃える

`lsharp test` が pass しても、`withdraw` は reviewed symbol なので次が残る。

```json
{
  "accepted": false,
  "open": [
    {"rule": "SCS.VerifyCompatibility.v1"},
    {"rule": "SCS.DocumentMigration.v1"},
    {"rule": "SCS.ReviewIntent.v1"}
  ]
}
```

compatibility checker は breaking である事実を evidence として記録するが、breaking change 自体を
自動承認しない。

### 7.3 Migration note を作る

`docs/migrations/withdraw-fee.md`:

```markdown
# Bank.withdraw fee parameter

`Bank.withdraw` の第 3 引数に non-negative fee を追加した。
既存 caller は fee を明示し、fee がない operation では `0` を渡す。
```

note digest は attestation payload に入る。non-empty file の存在だけでは review evidence にならない。

### 7.4 Human reviewer が署名する

まず current source から snapshot を生成する。open human obligation があっても `SourceValid` snapshot は出せる。

```bash
lsharp compile src/Bank.ls \
  --semantic-profile reviewed \
  --emit-semantic target/lsharp/current.semantic.json
```

```bash
lsharp attest intent \
  --semantic target/lsharp/current.semantic.json \
  --subject 'lsharp://bank/Bank/function/withdraw' \
  --reviewer-kind human \
  --disposition reviewed \
  --migration-note docs/migrations/withdraw-fee.md \
  --key ~/.config/lsharp/reviewer.ed25519 \
  --out target/lsharp/withdraw.attestation.json
```

LLM はこの private key を持たない。agent attestation を作っても `ReviewIntent` は閉じない。

### 7.5 Final verification

```bash
lsharp compile src/Bank.ls \
  --verify-against target/lsharp/base.semantic.json \
  --semantic-profile reviewed \
  --evidence target/lsharp/contracts.evidence.json \
  --evidence target/lsharp/compatibility.evidence.json \
  --evidence target/lsharp/withdraw.attestation.json \
  --trust-store /etc/lsharp/reviewer-keys \
  --emit-obligations target/lsharp/obligations.json
```

valid signature、migration-note digest、current fingerprint が一致して初めて accepted になる。

## 8. Generated specification

```bash
lsharp doc \
  --semantic target/lsharp/current.semantic.json \
  --out target/lsharp/spec
```

`Bank.withdraw` page は次を分離表示する。

- signature / parameter / return / transition: `static`;
- case / property と replay summary: `checked`;
- `:doc` / `:rationale`: `attested`;
- breaking API と migration note: reviewed change evidence;
- open obligation があれば `unaccepted` banner。

LLM が `target/lsharp/spec/Bank.withdraw.md` を直接編集しても source、snapshot、acceptance は変化しない。

## 9. PR / change gate

change gate は次の順で行う。

1. merge-base を separate checkout し baseline snapshot を生成する。
2. current checkout で snapshot を生成する。
3. current contract を実行し evidence を生成する。
4. baseline/current delta と policy から obligation を導出する。
5. executable / compatibility / target / attestation evidence を検証する。
6. error-severity open obligation が 0 件なら `ChangeAccepted`。
7. repository-wide native target / artifact / provenance gate が別途 pass して初めて `ReleaseReady`。

base/current を同じ mutable worktree から作らない。local manual gate でも CI gate でも trust boundary は同じである。

## 10. LLM agent loop

agent に渡す machine response は次を含む。

```json
{
  "accepted": false,
  "subject": "lsharp://bank/Bank/function/withdraw",
  "delta": {
    "api": "breaking",
    "contract": "changed",
    "intent": "changed",
    "implementation": "changed"
  },
  "openObligations": [
    {
      "rule": "SCS.RerunContracts.v1",
      "acceptedEvidence": ["CasesPassed", "PropertiesPassed"],
      "sourceSpans": [{"path": "src/Bank.ls", "start": 120, "end": 480}]
    },
    {
      "rule": "SCS.ReviewIntent.v1",
      "acceptedEvidence": ["HumanIntentAttested"],
      "agentMayClose": false
    }
  ]
}
```

agent は machine obligation を source edit と command execution で閉じる。`agentMayClose=false` に達したら
停止し、人間へ具体的な review subject と delta を提示する。prompt 上の「必ず metadata を更新すること」ではなく、
accepted evidence がなければ command が nonzero になることで enforcement する。

## 11. 運用上の禁止事項

- generated doc / graph を manual source として review し、source snapshot を検証しない。
- `obligations.json` の status を編集する。
- current branch で baseline を再生成して差分を消す。
- failed / timeout / unsupported contract を skip success にする。
- agent signature を human signature として扱う。
- type-check success を contract pass や accepted change と言い換える。
- native lane の fallback success を native evidence に数える。
