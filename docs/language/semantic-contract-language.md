# Semantic Contract Language Forms

状態: 規範的な言語契約
親契約: [`semantic-contract-system.md`](./semantic-contract-system.md)

本書は Semantic Contract System が受理する source form の正確な意味を定義する。code example は
normative target syntax である。parser が既に form を受理できても、本文の semantics と test が未実装なら
conforming ではない。

## 1. General rule

version 1 は第二の ontology DSL や contract DSL を追加しない。typed L# declaration と既存 canonical
metadata form へ operational semantics を与える。

canonical semantic model に入る source item は次に限定する。

- compiler-derived static fact;
- well-formed executable contract;
- 定義済み field にある authored intent / presentation content。

unknown metadata を free-form semantic edge に変換してはならない。

## 2. Typed declaration

```lisp
(defn remaining-balance
  [(: balance Int) (: amount Int)]
  : Int
  (- balance amount))
```

compiler は prose を重複させず次を導出する。

```text
symbol: remaining-balance
kind: function
parameters:
  - balance: Int
  - amount: Int
returns: Int
```

`:params` と prose `:returns` は legacy duplicated metadata である。semantic strict mode では
`LS3240` として拒否する。

## 3. Complete checked public function

```lisp
(module Bank)

(defn remaining-balance
  [(: balance Int) (: amount Int)]
  : Int
  :doc "Return the balance remaining after a permitted withdrawal."
  :rationale "The caller establishes that amount is positive and does not exceed balance."
  :case [
    (expect (remaining-balance 1000 200) 800)
    (expect (remaining-balance 1 1) 0)
  ]
  :assert [
    (= (remaining-balance 10 3) 7)
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
```

この declaration は独立した 5 category を持つ。

1. static signature fact;
2. executable contract claim;
3. authored purpose / rationale;
4. implementation body;
5. optional presentation content。

各 category は別 fingerprint を持つ。

## 4. `:case`

grammar:

```lisp
:case [
  (expect actual-expression expected-expression)
  ...
]
```

規則:

1. version 1 の `:case` は `defn` にだけ付与できる。
2. list は 1 件以上の `expect` を必要とする。
3. 両 expression は declaration の module scope で resolve する。
4. 両 expression は同じ canonical type を持たなければならない。
5. その type は contract runner の equality capability を満たさなければならない。
6. resolved `actual-expression` の call graph が owner function へ到達可能な場合だけ、static coverage candidate とする。
7. checked owner coverage は passing execution evidence の dynamic trace が owner invocation を 1 回以上含む場合だけ
   成立する。dead branch や未実行 helper にある reference は coverage ではない。
8. passing evidence は owner の API・contract・effective implementation closure fingerprint へ binding する。
9. equality unsupported は `LS3235` であり、skip success ではない。

valid:

```lisp
:case [
  (expect (remaining-balance 1000 200) 800)
]
```

type mismatch:

```lisp
:case [
  (expect (remaining-balance 1000 200) true)
]
```

well-formed でも owner を呼ばないため coverage にはならない例:

```lisp
:case [
  (expect (+ 1 1) 2)
]
```

## 5. `:assert`

grammar:

```lisp
:assert [
  boolean-expression
  ...
]
```

規則:

1. version 1 の `:assert` は `defn` にだけ付与できる。
2. assertion は module scope にある zero-argument expression として評価する。
3. function parameter と `result` は implicit binding されない。
4. 各 expression は `Bool` でなければならない。
5. list は 1 件以上の assertion を必要とする。
6. 本書の closed constant evaluator が `true` と評価する assertion は vacuous として拒否する。
7. resolved assertion call graph が owner へ到達可能な場合だけ static coverage candidate とする。
8. checked owner coverage は passing execution evidence の dynamic trace が owner invocation を含む場合だけ成立する。

valid:

```lisp
:assert [
  (= (remaining-balance 10 3) 7)
]
```

`balance` は module scope にないため invalid:

```lisp
:assert [
  (>= balance 0)
]
```

vacuous:

```lisp
:assert [true]
```

## 6. `:property`

grammar は維持する。

```lisp
:property [
  (for-all [name Type ...]
    :precondition [boolean-expression ...]
    :postcondition boolean-expression
    :cases non-negative-integer
    :seed non-negative-integer
    :shrink boolean)
  ...
]
```

version 1 の property はすべて owner-bound である。次の function に対し、

```lisp
(defn f [(: p0 T0) (: p1 T1)] : R ...)
```

次の property は、

```lisp
(for-all [x T0 y T1]
  :precondition [pre]
  :postcondition post)
```

次の実行意味を持つ。

```text
generated x:T0, y:T1 が pre を満たす各 case について:
  result = f(x, y)
  post を要求する
```

規則:

1. version 1 の `:property` は `defn` にだけ付与できる。
2. binder arity は owner parameter arity と一致する。
3. binder order が owner invocation argument order である。
4. type-variable instantiate 後、各 binder type は対応 parameter type と canonical equal である。
5. binder name は owner parameter name と異なってよい。
6. binder name は unique とし、`result` は reserved word とする。
7. `result` は owner invocation の実戻り値へ implicit binding し、owner return type を持つ。
8. precondition と postcondition は `Bool` である。
9. postcondition は必須で non-vacuous とする。
10. `:cases` は 1 以上。default は `256`。
11. `:seed` default は `0`。
12. `:shrink` default は `true`。
13. sampling は evidence に記録した versioned generator を使う。
14. unsupported binder type は `LS3232`。untyped / host fallback を使わない。
15. versioned attempt limit 内に requested accepted case 数を作れなければ `LS3233`。
16. owner invocation と contract expression は deterministic contract runtime の capability boundary 内で実行する。
    clock、OS random、network、unrecorded file I/O、host fallback などを要求する場合は `LS3237`。
17. timeout、crash、shrink failure、replay mismatch は failure とする。

canonical example:

```lisp
:property [
  (for-all [balance Int amount Int]
    :precondition [(>= balance 0) (> amount 0) (<= amount balance)]
    :postcondition (= result (- balance amount))
    :cases 256
    :seed 0
    :shrink true)
]
```

arity mismatch:

```lisp
:property [
  (for-all [balance Int]
    :postcondition (= result balance))
]
```

reserved binder:

```lisp
:property [
  (for-all [result Int amount Int]
    :postcondition (= result amount))
]
```

### 6.1 Closed constant evaluator

static vacuity 判定は一般の定理証明ではなく、次の closed expression だけを評価する total evaluator で行う。

- `Int` / `Bool` / `String` literal;
- literal だけを operand とする `+`、`-`、`*`、整数比較、Boolean connective、equality;
- result が上記へ閉じる `if`;
- compiler が `const` として確定した binding。

function call、parameter、`result`、unknown operator、overflow、division by zero、effectful expression を含む場合は
`Unknown` とし、tautology / contradiction と決め付けない。evaluator の result は `True`、`False`、`Unknown` の
3 値である。`True` の assertion / postcondition、`False` の precondition だけを version 1 の static vacuity として
拒否する。

### 6.2 Version 1 generator surface

version 1 が必須対応する generator type は意図的に小さくする。

- `Int`;
- `Bool`;
- base type が対応済みで、constraint predicate を contract runtime が評価できる constrained alias。

record、tuple、recursive ADT、String、user generator、polymorphic function への対応は version 1 の必須ではない。
新しい type を追加する場合は generator-version fixed vector を追加する。それまでは concrete `:case` を使うか
`LS3232` を返す。

constrained value は base value を生成し、全 constraint を評価し、pass した値だけを accepted case として数える。
attempt limit は次で固定する。

```text
max(cases * 100, 10_000)
```

この limit は `type-directed-splitmix64-v1` の identity に含まれる。

shrink は次の deterministic algorithm とする。

1. binder を declaration order で一つずつ処理する。
2. `Int x` の candidate は、`x != 0` のとき `0`、続いて `x / 2`、`x / 4`、... を truncation toward zero
   で生成し、duplicate と `0` を除く。最後に `x - sign(x)` を未出力なら追加する。
3. `Bool true` の candidate は `false` だけ、`Bool false` は candidate なし。
4. constrained value は constraint を満たす candidate だけを残す。
5. candidate が全 precondition を満たし、owner を 1 回実行した postcondition が引き続き fail する場合、その
   candidate を採用し binder 0 から再開する。
6. 全 binder で採用がなければ minimized counterexample とする。

整数演算 overflow、owner execution failure、replay mismatch は shrink の候補棄却ではなく contract failure である。

## 7. Constrained type

```lisp
(type-constrained Percentage Int
  :constraints [(>= 0) (<= 100)])
```

次を semantic model へ入れる。

- nominal semantic identity `Percentage`;
- base type `Int`;
- lower-bound predicate;
- upper-bound predicate。

predicate は API と contract の両 fingerprint に含める。`100` から `200` への変更は API change かつ
behavioral change である。

各 predicate は次の verification mode を持つ。

- `static`: type checking で discharge した。
- `runtime`: inserted runtime guard で enforcement した。
- `sampled`: generated input を使う contract だけで exercise した。
- `unsupported`: `checked` / `reviewed` policy では受理しない。

generated specification は mode を表示する。sampled check を proof と表現してはならない。

## 8. Type から導出する typestate

state transition は function type から導出し、`:transitions` へ重複記述しない。

```lisp
(type Draft DraftState)
(type Submitted SubmittedState)
(type Accepted AcceptedState)

(type (Order state)
  (: (DraftOrder Int Int) (Order Draft))
  (: (SubmittedOrder Int Int) (Order Submitted))
  (: (AcceptedOrder Int Int) (Order Accepted)))

(defn submit
  [(: order (Order Draft))]
  : (Order Submitted)
  :doc "Submit a draft order."
  :case [
    (expect
      (submit (DraftOrder 100 1200))
      (SubmittedOrder 100 1200))
  ]
  (match order
    [(DraftOrder id total) (SubmittedOrder id total)]))

(defn accept
  [(: order (Order Submitted))]
  : (Order Accepted)
  :doc "Accept a submitted order."
  :case [
    (expect
      (accept (SubmittedOrder 100 1200))
      (AcceptedOrder 100 1200))
  ]
  (match order
    [(SubmittedOrder id total) (AcceptedOrder id total)]))
```

導出結果:

```text
Order: Draft -> Submitted by submit
Order: Submitted -> Accepted by accept
```

input と return type が同じ resolved type constructor を持ち、state position の argument が異なる場合だけ
transition edge を作る。無関係な ADT 間の function を typestate transition にしてはならない。

## 9. Intent と presentation

```lisp
(defn normalize-id
  [(: value String)]
  : String
  :doc "Normalize an external identifier before comparison."
  :rationale "Normalization is kept at the boundary so internal IDs remain nominal."
  :since "0.3.0"
  :see-also [parse-id validate-id]
  :example [
    (normalize-id " A-01 ")
  ]
  ...)
```

規則:

- `:doc` / `:rationale` は intent であり machine-proved ではない。
- `:example` は presentation-only で type-check を必須とする。
- `:since` / `:see-also` は presentation fingerprint だけを変更する。
- expected outcome のない example は contract coverage を満たさない。
- `:see-also` target は symbol resolve を必須とする。

## 10. Legacy metadata

| form | version 1 treatment |
|---|---|
| `:params` | strict mode で reject。name/type は導出する |
| `:returns` | strict mode で reject。return type は導出する |
| `:invariant` | strict mode で reject。constraint / `:assert` / `:property` へ migrate |
| `:transitions` | strict mode で reject。typestate signature から導出 |
| `:example` | presentation-only。evidence にはしない |
| `:doc` | authored intent |
| `:rationale` | authored intent |
| `:since` | presentation metadata |
| `:see-also` | presentation metadata |

migration tool が explicit に変換するまでは legacy form を parser/AST で lossless に保持する。silent に
canonical fact や passing contract へ変換してはならない。

## 11. Coverage calculation

`checked` / `reviewed` function の owner coverage は次のいずれかで満たす。

- passing `:case` の dynamic execution trace が owner invocation を含む。
- passing `:assert` の dynamic execution trace が owner invocation を含む。
- passing owner-bound `:property` が owner を exactly once 実行した。

次は coverage ではない。

- docs-only example;
- 他 symbol の contract;
- owner への static reference はあるが実行されない case/assertion;
- owner と無関係な case/assertion;
- empty form;
- vacuous predicate;
- stale / failed evidence。

owner body 内の recursive call 自体は coverage ではない。call は contract execution trace で観測されるか、
owner-bound property execution により implicit に行われる必要がある。static call graph は candidate の事前検査と
依存閉包計算に使うが、checked coverage の最終 oracle にはしない。

## 12. Source-level acceptance example

### 12.1 `typed` private helper

```lisp
(private
  (defn clamp-low
    [(: value Int) (: minimum Int)]
    : Int
    (if (< value minimum) minimum value)))
```

`typed` では prose / contract なしで受理できる。

### 12.2 `checked` public function without purpose

```lisp
(defn add [(: x Int) (: y Int)] : Int
  :case [(expect (add 1 2) 3)]
  (+ x y))
```

type-correct だが `checked` では `LS3220` により accepted にならない。

### 12.3 Stale evidence

body を `(+ x y)` から `(- x y)` へ変えると implementation fingerprint が変わる。以前の passing evidence は
一致しないため `LS3231` が open になる。current contract を実行すると case が fail し、stale evidence で
通過できない。

### 12.4 `checked` contract change

checked public function の API または contract を変更すると、contract execution が pass しても
`SCS.ReconcileIntent.v1` が open になる。agent は current delta と `:doc` を照合し、`updated` または `affirmed`
disposition を current API / contract / intent fingerprint に bind した evidence として出す。これがない限り
`LS3223` は open のままである。

### 12.5 `reviewed` semantic change

public return type、contract、intent のいずれかを変更すると以前の human attestation は stale になる。
machine contract が pass しても、current API / contract / intent fingerprint に対する trusted human signature が
得られるまで `LS3222` は open のままである。agent reconciliation はこの requirement を閉じない。
