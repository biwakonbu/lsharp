# ADR: v0.3 selfhost review lifecycle reducer

- Status: Accepted (verified partial slice)
- Date: 2026-07-31
- Scope: selfhost `Tools.Validation.Lifecycle` の review lifecycle event reducer
- Related: [`decisions-v0.3-review-lifecycle.md`](decisions-v0.3-review-lifecycle.md)、
  [`decisions-v0.3-review-lifecycle-wire-ordering.md`](decisions-v0.3-review-lifecycle-wire-ordering.md)、
  [`v0.3-review-provenance-lifecycle.md`](../development/planning/v0.3-review-provenance-lifecycle.md)、
  `EC-M3-02`

## Context

Rust の canonical reducer は review lifecycle を declaration order に依存させず、review ID と
sequence の deterministic order で再生する。selfhost 側には attestation/source の validation
primitive がある一方、lifecycle event の受理、並べ替え、transition 拒否を実行する reducer が
なく、Rust-host の成功だけでは selfhost parity を証明できなかった。

特に sequence 2 が先に到着する provider response、同一 sequence の payload 差し替え、
`revoked` / `superseded` 後の resurrection は、順序依存や fail-open を生むため、同じ observable
error boundary を selfhost に固定する必要がある。

## Decision

- `selfhost/src/Tools/Validation/Lifecycle.ls` に、
  `[review-id, sequence, state, effective-at, reason-digest]` event と
  `[code, review-id, sequence, previous-sequence, state, previous-state]` error を定義する。
  `effective_at` の巻き戻しには code `8` として、入力側/既存側の timestamp を末尾 payload に付加する。
- event は既存の source review wire と strict UTC timestamp、nonblank field policy を再利用し、
  review ID、sequence、state、optional reason digest を fail-closed に検証する。
- registry の view は review ID の byte-wise lexical order、同一 review 内の sequence order へ
  insertion sort で正規化する。入力配列の順序を結果や digest の意味論にしない。
- 初期 state は `proposed` / `active` のみ、遷移は `proposed → active`、
  `active → superseded`、`active → revoked` のみとする。terminal state からの resurrection、
  duplicate sequence、sequence rollback、同一 review 内の `effective_at` rollback は明示 error code で拒否する。
- timestamp は event construction 時に strict canonical UTC へ検証済みなので、Rust canonical reducer と
  同じ UTF-8 byte-wise lexical order を chronological order として比較する。既存の sequence/transition
  error precedence は維持する。
- この module は lifecycle state の純粋な reducer とし、provider 取得、signature/trust、
  report projection、CLI/MCP の入力解決は外部境界として残す。

## Evidence

- RED: focused E2E を先に追加し、canonical `Lifecycle.ls` が存在しない状態で
  `canonical Lifecycle.ls が読み込めない` と失敗することを確認した。
- GREEN: `CARGO_TARGET_DIR=/Users/biwakonbu/github/tmp/lsharp-m3-selfhost-lifecycle/target cargo test -p lsharp-wasm --test e2e 'e2e::selfhost_evidence_registry::lifecycle::selfhost_lifecycle_reducer_orders_events_and_rejects_invalid_transitions' -- --exact --nocapture`（1 passed）。
  out-of-order events の正規化、terminal state、duplicate、initial state、rollback を同じ
  Wasm runtime で検証した。
- Rust oracle: `CARGO_TARGET_DIR=/Users/biwakonbu/github/tmp/lsharp-m3-selfhost-lifecycle/target cargo test -p lsharp-types --test review_lifecycle`（5 passed）。
- RED: selfhost に `effective_at` rollback fixture を追加すると、sequence/transition だけでは拒否できず
  既存 boundaryへ誤って流れることを確認した。
- GREEN: code `8` の `EffectiveTimeRollback` と入力/既存 timestamp payloadを selfhost Wasm で確認し、
  Rust `lifecycle_rejects_effective_time_rollback` と同じ fixture意味論へ揃えた。
- Regression: `cargo test -p lsharp-types --test review_lifecycle`（6 passed）と
  selfhost lifecycle E2E（1 passed）を通過した。
- Regression: selfhost evidence registry 全体（51 tests）が passed。
- Contract: 対象 Rust files の `rustfmt --edition 2024 --check`、`git diff --check`、
  `bash scripts/audit_docs.sh`（0 errors, 0 warnings）が passed。

## Boundary

これは macOS の Rust host が生成・実行する selfhost Wasm における lifecycle reducer の
verified partial slice である。native stage0 の source/package provenance、App.Cli /
EmbeddedCli への reducer wiring、stale/revoked report projection、Mac Apple Silicon /
Linux x86_64 の native artifact/runtime gate、provider snapshot の取得は未完了であり、
`EC-M3-02` の `[~]` を維持する。
