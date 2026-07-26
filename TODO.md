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
  project-level duplicate 検査、selfhost/native parity を閉じる。
- [~] `EC-M2-02` evidence graph — required provenance を持つ evidence record、
  `supports` / `contradicts` の registry closure、source の `shrinks` / `coverage`、
  canonical manifest projection は verified。実行 trace と generator policy、
  review/evaluates/invalidates、外部 provenance と privacy policy、selfhost/native parity を閉じる。
- [~] `EC-M2-03` `lsharp validate` — version 1 manifest parser、source adapter、
  `--emit-manifest` の atomic/durable file boundary、deterministic text/JSON report、
  `pass=0` / `fail=1` / `unknown=2` の Rust CLI は verified。selfhost/native の manifest
  producer/parser/report/exit code、release-level provenance、EmbeddedCli/MCP、
  両 supported target の runtime evidence を閉じる。

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
  `Module` / `Function` / GC model、linker seam、`validation_source` evidence seam は verified。`wasi.rs`、`lsharp-ir/src/lib.rs`、`lsharp-tooling/src/compile.rs`、
  `infer.rs`、parser/lower/driver/LSP の責務分割を、型・focused test・snapshot parity を保って完了する。
- [~] `LEGACY-TEST-01` property/fuzz/limit coverage — Issues `I-06` / `I-08`。syntax/types
  property test と複数の GC/type/runtime limit lane は verified。再利用可能な generator、
  leak/rooting stress、performance threshold、full fuzz target、native stage0 evidence を閉じる。

## Scheduling rules

- current milestone は `EC-M2-01` → `EC-M2-02` → `EC-M2-03`。同時に一つの observable contract だけを進める。
- product/release completion target は `aarch64-apple-darwin` と `x86_64-unknown-linux-gnu` に限定する。
- Linux VM / stage regeneration は共有 lock と既存 artifact を使い、同じ heavy replay を重複起動しない。
- Rust は bootstrap、oracle/differential、rollback、未移行 host integration の明示境界として保持する。
- verified slice は `[~]` のまま残し、aggregate completion 後に evidence を ADR/仕様へ移して本ファイルから削除する。
