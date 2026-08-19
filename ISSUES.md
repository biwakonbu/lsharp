# L# 問題台帳 (ISSUES)

> **本ファイルの役割**: 現バージョンの設計・実装・ドキュメント上の問題を一元管理する**問題台帳**。
> 「何が問題か・根拠・現在の状態」を記録する。「何をやるか」のタスク正本は [TODO.md](TODO.md) であり、
> 本台帳にチェックボックスは置かない。参照は ISSUES → TODO / ADR / 設計ドキュメントの一方向とする。
>
> **採番**: `D-NN` (設計) / `I-NN` (実装) / `DOC-NN` (ドキュメント)。
> 本台帳の `DOC-NN` は TODO.md / ADR-169 のタスク ID `DOC-02` 等とは**別体系**である。
>
> **状態**: `open` (未着手) / `in-design` (設計ドキュメントあり) / `deferred` (V2 等へ委譲済み) /
> `documented-limitation` (既知の制限として公式整理済み) / `resolved` (解消済み、履歴として保持)。
>
> **根拠検証日**: 2026-06-12 (記載の file:line はこの日時点の実測)。
> **状態・タスク対応の再監査日**: 2026-08-12。
>
> **改善方針**: [docs/development/planning/improvement-roadmap.md](docs/development/planning/improvement-roadmap.md)
> **新設計**: [docs/development/planning/improvement-designs/](docs/development/planning/improvement-designs/README.md)

## 2026-08-02 現在の checkpoint（作業再開用）

この節は、次の作業を再開するときに最初に読む現在地である。問題 ID の正本は以下の既存台帳であり、
日付ごとの細かなログや完了判定は ADR・仕様・検証記録へ分離する。

### 現在の事実

- 確認時点の `origin/main` は `89de36805439fda34040ab63f0919c7f4ed34a2e`
  (`fix: reject symlinked installed package manifests`)。この commit は、Cloud で作成した検証済み
  slice を task-owned worktree へ適用し、Rust/native parity と focused gate を通したうえで公開したもの。
- canonical な問題台帳は本ファイル（`ISSUES.md`）、未完タスクの正本は [`TODO.md`](TODO.md) である。
  `ISSUE.md` という別名の台帳は作らず、二重管理を避ける。
- `/Users/biwakonbu/github/lsharp` の root checkout は複数セッションが共有しており、確認時点で競合中の差分と
  未追跡ファイルがある。通常作業では編集・pull・reset の対象にせず、専用 worktree を
  `/Users/biwakonbu/github/tmp/<task>/` に作り、完了後に自分が所有するものだけを削除する。
- L# の正本実装は Rust と native selfhost stage0 の二系統で、Rust は oracle/bootstrap/rollback のために残す。
  Rust-free の完了判定には parser → 型推論 → lowering → codegen → runtime → 公開 command の境界と、
  Mac Apple Silicon / Linux x86_64 の必要な実行証跡が要る。focused test、summary、stale artifact、Rust host
  fallback の成功だけでは完了としない。

### 目指す次版と運用の判断

- 現在の active milestone は v0.3 review provenance / lifecycle であり、次版の形は
  [`v0.4-lsharp-next-shape.md`](docs/development/planning/v0.4-lsharp-next-shape.md) と
  [`v0.4-milestone-01.md`](docs/development/planning/v0.4-milestone-01.md) に分けて設計する。
- Cloud は実装・RED→GREEN・task-only commit の場所、local の task-owned worktree は結果を main へ適用し、
  まとめて検証して push する場所とする。Cloud の HTTPS credential がなくても、commit は捨てず、SHA・
  parent・remote・merge-base・left/right・検証結果を残して local 適用へ進む。
- 完了項目は ADR / 仕様 / evidence に移し、`TODO.md` から削除する。partial parity、Rust-only、
  external boundary、未検証 ABI は `[~]` のまま残し、`[x]` は使わない。

### 直近で閉じた installed-package ownership の境界

以下は package root の外部 path を Rust/native MCP が package-owned として投影しないための verified partial である。
いずれも installer、live provider/auth、実 target runtime、Mac/Linux packaged parity まで閉じたものではない。

| 境界 | 現在の契約 | 記録 |
|---|---|---|
| `docs/api.json` | 既存 metadata は regular non-symlink file のみ読む | [`decisions-v0.3-native-mcp-package-api-regular-file-boundary.md`](docs/adr/decisions-v0.3-native-mcp-package-api-regular-file-boundary.md) |
| `.lsharp/packages/<entry>` | package entry 自体は regular non-symlink directory のみ列挙する | [`decisions-v0.3-native-mcp-installed-package-directory-ownership.md`](docs/adr/decisions-v0.3-native-mcp-installed-package-directory-ownership.md) |
| `lsharp.toml` | regular package directory 内で manifest が存在する場合、symlink は無視し、explicit API は既存 not-found で fail-closed | [`decisions-v0.3-native-mcp-package-manifest-symlink-boundary.md`](docs/adr/decisions-v0.3-native-mcp-package-manifest-symlink-boundary.md) |

### 未完了の境界と再開点

- `src/` ディレクトリ自体が外部 directory への symlink の場合の source traversal / in-memory package API generation は
  まだ実装していない。次の RED は、同一 fixture で外部 source の name/content/metadata が応答へ投影されないことと、
  Rust/native の既存 not-found または empty contract を固定すること。Cloud への follow-up はこの checkpoint 指示で停止した。
- 個別 source file、`docs/` directory、その他 nested entry 全体、installer が取得した tree、provider/auth、
  current-source runtime、Mac/Linux packaged/rollback parity は別の残件である。ひとつの slice を越えて一括完了扱いにしない。
- Linux replay / stage regeneration / full build は、current-source manifest・expected replay lock・VM ownership が揃わない間は起動しない。
  同様に live provider/auth と実 Ed25519 は offline contract の証拠へ拡大解釈しない。

### 作業を再開するときの gate

1. `git fetch origin main`、root/worktree/status、TODO の active item、対象 artifact/VM 状態を確認する。
2. 一つの observable contract の RED を Rust/native の同一 fixture で追加し、失敗値を確認してから実装する。
3. focused GREEN、必要な Rust/native/fake batch、rustfmt/Python syntax、docs audit、`git diff --check` をまとめて実行する。
4. evidence を ADR と TODO に反映し、task-relevant files だけを commit/push する。push 後に `HEAD == origin/main`、
   worktree、remote branch、残タスクを再監査する。

## 2026-08-12 現在の checkpoint（作業再開用）

### 現在の事実

- `origin/main` と task-owned worktree の code checkpoint は `47743365897b5b30416f0bc11f63b36025ef6229`
  (`feat(driver): aggregate source validation projects`) で一致している。`4e2d0cf3` では default
  EmbeddedCli build を阻害していた selfhost validation serializer の異種 state 型を分離し、`47743365` では
  Rust `validate --source` が regular file または directory を deterministic に収集して project graphへ集約する。
- duplicate intent node の cross-file boundary は、duplicate code `2`、first/duplicate path と span、exit `1`、
  stdout空、manifest未生成を `crates/lsharp-driver/tests/validate_cli.rs` で検証済みである。single-file pathも同じ
  aggregatorを通るため、既存の file contract は維持している。
- Mac Apple Silicon の current-source native App.Cli release は source commit `47743365` と fixed-point manifest を
  持ち、native core runtime matrix `44 cases` が passした。Linux x86_64 replayは stage1後に actual stage2/stage3
  summaryを回収できず、Linux fixed-point、Linux App.Cli target-only、directory validationの native runtime evidenceは未取得である。
- Linux VM内の hostgen process、tmux、task-owned workdir、replay lockは終了後に残っていない。`lsharp-linux-x86` は
  停止済みで、VM使用量は約 `3.5 GiB`、空きは約 `7.2 GiB` だった。次回は VM を再利用する前に current-source
  manifest / lock / process ownership を確認する。

### 未完了の境界と再開点

- `EC-M2-01` / `EC-M2-03` の Rust project aggregate は valid cross-file edge の report/manifest、duplicate
  evidence/review の source-specific diagnostics、MCP/public surfaceまで閉じていない。次の RED は TODO.md にある
  `validate_accepts_project_directory_with_cross_file_edge` で、2 fileの node/edge/source provenanceを固定する。
- `selfhost/src` の directory input、native App.Cli/EmbeddedCli/MCP parity、Linux current-source stage2/stage3
  fixed point、Mac/Linux packaged provenance parityは未完了である。Macの44-case成功をLinuxやRust-free aggregateへ拡大解釈しない。
- `ISSUE.md` という別名は作らず、本ファイルを問題台帳、`TODO.md` を未完タスク正本として維持する。

### 再開 gate

1. `git fetch origin main`、worktree、TODO、VM/process/lock/artifactを確認する。
2. cross-file edge の Rust RED/GREEN と report/manifest contractを一つの semantic batchで検証する。
3. selfhost/native parityが必要になった時点で、Mac/Linux heavy replayを同時起動せず、既存 stage2とVM-side lockを再利用する。
4. docs audit、`git diff --check`、intentional filesだけの commit/push、remote SHAとVM停止状態を再確認する。

---

## サマリー

### 設計 (D)

| ID | 問題 | 影響度 | 状態 | 設計 doc |
|----|------|--------|------|----------|
| [D-01](#d-01) | WasmGC codegen が i64 フォールバックのまま | 高 | in-design | [imp-01](docs/development/planning/improvement-designs/imp-01-wasmgc-full-migration.md) |
| [D-02](#d-02) | GADT が型チェックのみで実行未検証 | 中-高 | in-design | imp-01 |
| [D-03](#d-03) | HKT が型チェックのみで実行未対応 | 中 | in-design | imp-01 |
| [D-04](#d-04) | Computation Expression がビルダー登録のみの MVP | 中 | in-design | imp-01 |
| [D-05](#d-05) | 正規表現制約が簡易パターンのみ | 低-中 | resolved | [imp-08](docs/development/planning/improvement-designs/imp-08-regex-constraint-engine.md) |
| [D-06](#d-06) | トレイトが静的ディスパッチのみ (vtable なし) | 中 | in-design | imp-01 |
| [D-07](#d-07) | SCC 推論は部分実装、canonical/native parity が未完 | 中 | in-design | [imp-04](docs/development/planning/improvement-designs/imp-04-module-system-strengthening.md) |
| [D-08](#d-08) | Native backend self-regeneration / differential track | 中-高 | resolved | [native backend 仕様](docs/language/native-backend-spec.md) |
| [D-09](#d-09) | セルフホスト ADT が整数タグ + Vector 表現 | 中 | in-design | imp-01 |
| [D-10](#d-10) | GC sentinel 判別の理論的 edge case (G1) | 低-中 | documented-limitation | [imp-03](docs/development/planning/improvement-designs/imp-03-dynamic-memory-layout.md) |

### 実装 (I)

| ID | 問題 | 影響度 | 状態 | 設計 doc |
|----|------|--------|------|----------|
| [I-01](#i-01) | ファイルサイズ規約 (500-800 行) を 16 ファイルが超過 | 高 | in-design | [imp-06](docs/development/planning/improvement-designs/imp-06-large-file-decomposition.md) |
| [I-02](#i-02) | 診断 code/span が全 surface に未貫通 | 高 | in-design | [imp-02](docs/development/planning/improvement-designs/imp-02-error-handling-unification.md) |
| [I-03](#i-03) | GC 容量 grow が全 runtime/backend に未貫通 | 高 | in-design | imp-03 |
| [I-04](#i-04) | GC フリーリストが線形探索 | 中 | in-design | imp-03 |
| [I-05](#i-05) | Rust host cache は部分実装、selfhost/native persistence が未完 | 中 | in-design | imp-04 |
| [I-06](#i-06) | Property/limit slice はあるが full fuzz/leak/perf gate が未完 | 中 | in-design | [imp-07](docs/development/planning/improvement-designs/imp-07-test-verification-infrastructure.md) |
| [I-07](#i-07) | rooting guard は部分実装、全 source/runtime 境界が未完 | 中 | in-design | imp-07 |
| [I-08](#i-08) | テスト配置が巨大 E2E に集中 | 中 | in-design | imp-07 |
| [I-09](#i-09) | installed package の nested source ownership が未完 | 中 | in-design | v0.3 MCP package ownership ADR |
| [I-10](#i-10) | `validate --source` project aggregate の selfhost/native parity が未完 | 中-高 | in-design | v0.2 validation model |
| [I-11](#i-11) | `cargo test --workspace` の恒常 FAIL が台帳未記載 | 高 | resolved | -- |
| [I-12](#i-12) | ビルド再現性の綻び (`Cargo.lock` 非追跡 / dead test file) | 低-中 | resolved | -- |
| [I-13](#i-13) | native aarch64 の linear heap に回収機構と bounds check が無い | 高 | documented-limitation | -- |
| [I-14](#i-14) | root lifetime verifier が公開 runtime API の合法な使用を拒否する | 中-高 | resolved (17/17 解消) | [main exit 免除 ADR](docs/adr/decisions-root-lifetime-main-exit-exemption.md) / [意図的不均衡の注釈 ADR](docs/adr/decisions-root-lifetime-intentional-imbalance-annotation.md) |
| [I-15](#i-15) | `default-path-smoke` が guest 経路を前提に書かれ、既定経路を検査していない | 中 | resolved | [default-path-smoke 決定論化 ADR](docs/adr/decisions-default-path-smoke-determinism.md) |
| [I-16](#i-16) | embedded component cache の key が build 入力 (`wit/` / `stdlib/`) を覆いきっていない | 中 | resolved | [cache key 被覆 ADR](docs/adr/decisions-embedded-component-cache-key-coverage.md) |
| [I-17](#i-17) | runtime spec が root 管理 API の戻り値と境界挙動を定義していない | 中 | resolved | [root API 契約 ADR](docs/adr/decisions-runtime-spec-root-api-contract.md) |
| [I-18](#i-18) | metadata directive の allowlist が 3 系統で二重管理されている | 中 | open | [directive allowlist parity ADR](docs/adr/decisions-parser-directive-allowlist-parity.md) |
| [I-19](#i-19) | CI 自動実行の停止で `ci.yml` の 17 job が 1 ヶ月以上まったく観測されていない | 中 | documented-limitation | [default-path-smoke 決定論化 ADR](docs/adr/decisions-default-path-smoke-determinism.md) |
| [I-20](#i-20) | selfhost parser が受理した 6 directive の payload を黙って捨てている | 中 | open | [directive allowlist parity ADR](docs/adr/decisions-parser-directive-allowlist-parity.md) |
| [I-21](#i-21) | native backend の root API が runtime spec の tier 1 契約に適合していない (aarch64 は解決済、x86-64 が残件) | 高 | open | [空 stack ガード ADR](docs/adr/decisions-native-root-pop-empty-guard.md) |
| [I-22](#i-22) | heavy e2e 164 件が `#[ignore]` 契約を満たしていない。案 A で裁定し 2026-08-19 に実装した | 中 | resolved | [ignore 契約 ADR](docs/adr/decisions-test-gate-ignore-contract.md) |
| [I-23](#i-23) | `aarch64-selfhost-helper-trailer-size` の pin が 2026-08-03 から陳腐化したまま気付かれていない | 中 | open | -- |
| [I-24](#i-24) | 診断の「重複」定義が spec 文言 / test / 実装の 3 者で食い違い、文言どおりに直すと lint 指摘が消える | 中 | resolved | [lint dedup identity ADR](docs/adr/decisions-lint-diagnostic-dedup-identity.md) |
| [I-25](#i-25) | `NativeCodegen.ls` に呼び出し元 0 の defn が 64 個。うち 1 群は使用中の実装と乖離している | 低-中 | open | -- |
| [I-26](#i-26) | x86 lane は helper trailer の補正を持たず、`x86-selfhost-helper-trailer-size` は呼び出し元 0 のまま | 中 | open | -- |
| [I-27](#i-27) | x86 native の hot path で user call を挟むと local/引数が壊れる。回避策だけが test に pin され、欠陥そのものが台帳に無い | 中 | open | -- |

### ドキュメント (DOC)

| ID | 問題 | 影響度 | 状態 | 設計 doc |
|----|------|--------|------|----------|
| [DOC-01](#doc-01) | ユーザーガイドの主要範囲不足 | 高 | resolved | [imp-05](docs/development/planning/improvement-designs/imp-05-docs-restructure.md) |
| [DOC-02](#doc-02) | book/ がユーザー向けと実装者向けの混在 | 中 | resolved | imp-05 |
| [DOC-03](#doc-03) | ドキュメント鮮度追跡 (.lsharp-doc-status) が未運用 | 中 | resolved | imp-05 |
| [DOC-04](#doc-04) | examples/ とドキュメントの連携不足 | 低-中 | resolved | imp-05 |
| [DOC-05](#doc-05) | language-guide テンプレートと docs/ の二重管理リスク | 低 | resolved | imp-05 |
| [DOC-06](#doc-06) | エラーコード体系が docs 未定義 (MCP に E0001-E0005 のみ) | 中 | resolved | imp-02 |
| [DOC-07](#doc-07) | ドキュメント更新が実装の後追いになり、依頼駆動でしか走らない | 中 | in-design | [doc-sync rule](.claude/rules/doc-sync.md) |
| [DOC-08](#doc-08) | 陳腐化した記述と重複節 (legacy-rust-bootstrap README / TODO の v0.3 節) | 低-中 | resolved | -- |
| [DOC-09](#doc-09) | 完了 TODO を削除する際に根拠が ADR へ移されず、原因究明の記録ごと消えている | 中 | resolved | [x86 値 liveness の却下案](docs/adr/decisions-native-x86-value-liveness-rejected-approaches.md) |

---

## 設計上の問題

<a id="d-01"></a>
### D-01: WasmGC codegen が MVP の i64 フォールバックのまま

- **影響度**: 高 / **状態**: in-design
- **内容**: レコード型・ADT は設計上 WasmGC struct へマップされる想定だが、現行 codegen は
  リニアメモリ + i64 表現のフォールバックで動作している。Wasm 層で型情報が消失し、
  レコード/ADT の実行時型安全性と後段最適化の余地が失われている。
- **根拠**:
  - `crates/lsharp-wasm/src/emit.rs:199`, `:203`, `:205`, `:211` -- 「TODO: WasmGC 本格実装時に削除。スタック操作はフォールバック用。」
  - `docs/development/planning/v2-designs/v2-07-wasmgc-optional-backend.md` -- WasmGC backend は「Phase 11 後に実装予定」のまま
- **関連**: V2-07 (WasmGC optional backend、設計正本)。改善設計は [imp-01](docs/development/planning/improvement-designs/imp-01-wasmgc-full-migration.md) (v2-07 の補遺)。

<a id="d-02"></a>
### D-02: GADT が型チェックのみで実行未検証

- **影響度**: 中-高 / **状態**: in-design
- **内容**: GADT 構文 (`Variant.return_type`) のパースと型チェックは実装済みだが、
  GC struct 型の wasmtime 未サポートを理由にサンプルは実行を伴わない。
  パターンマッチ時の型絞り込み (type refinement) の実行時挙動が未検証。
- **根拠**:
  - `examples/gadt.ls:2` -- 「GC struct 型は wasmtime で未サポートのため、型チェックのみ検証。main は print のスタブ。」
- **関連**: D-01 (WasmGC 移行が前提)。imp-01 参照。

<a id="d-03"></a>
### D-03: HKT (高カインド型) が型チェックのみで実行未対応

- **影響度**: 中 / **状態**: in-design
- **内容**: `Kind` (Star/Arrow) は型システムに定義されているが、HKT を使うサンプルは
  実行を伴わず型チェックのみ。HKT ベースの Functor/Monad 抽象が実用段階にない。
- **根拠**:
  - `examples/hkt.ls:2` -- 「GC struct 型は wasmtime で未サポートのため、型チェックのみ検証。main は print のスタブ。」
- **関連**: D-01 / D-04。imp-01 参照。

<a id="d-04"></a>
### D-04: Computation Expression がビルダー登録のみの MVP

- **影響度**: 中 / **状態**: in-design
- **内容**: `let!` / `do!` / `return` の構文 (`ComputationStep`) は AST にあるが、
  MVP 段階ではビルダー登録のみで、let!/return の Wasm 実行は未対応。
  モナディックな計算式が実用化されていない。
- **根拠**:
  - `examples/computation.ls:2` -- 「MVP 段階ではビルダー登録のみ。let!/return の Wasm 実行は GC 型の wasmtime サポート後に完全対応予定。」
- **関連**: D-01 (GC 型の実行サポートが前提)。imp-01 参照。

<a id="d-05"></a>
### D-05: 正規表現制約が簡易パターンのみ

- **影響度**: 低-中 / **状態**: resolved
- **内容**: 制約付き型の `matches` 制約は `crates/lsharp-types/src/regex/` の共有 engine で評価する。
  `constraints.rs` 側の重複 matcher は削除し、`{n}` / `{n,m}` / `{n,}`、否定 shorthand class、
  non-capturing group、lazy quantifier suffix、Unicode letter/number class を利用者向け reference に明記した。
- **解消根拠**:
  - `crates/lsharp-types/src/regex/mod.rs` -- bounded quantifier、否定 shorthand、non-capturing group、lazy suffix を実装
  - `crates/lsharp-types/src/regex/dfa.rs` -- bounded repeat / non-capturing group を DFA 側の NFA fragment へ接続
  - `crates/lsharp-types/src/constraints.rs` -- `matches` 制約が shared regex engine を参照
  - `docs/guides/language-reference.md` -- `type-constrained` と `matches` regex syntax の利用者向け表
- **検証**:
  - `test_regex_bounded_quantifiers`
  - `test_regex_shorthand_negated_classes`
  - `test_regex_non_capturing_group_does_not_shift_backreference`
  - `test_regex_lazy_quantifier_suffix_is_accepted`
  - `test_string_constraint_uses_shared_regex_extended_features`
- **関連**: 改善設計は [imp-08](docs/development/planning/improvement-designs/imp-08-regex-constraint-engine.md)。

<a id="d-06"></a>
### D-06: トレイトが静的ディスパッチのみ (動的ディスパッチなし)

- **影響度**: 中 / **状態**: in-design
- **内容**: トレイトメソッド呼び出しは lowering 時にマングル名
  (`TraitName_TypeName_methodName` 形式) で具象実装関数へ静的に解決される。
  vtable による動的ディスパッチ・存在型 (trait object 相当) が表現できない。
  WasmGC vtable による動的ディスパッチは未実装と book にも明記されている。
- **根拠**:
  - `book/ch10-traits.md:3` -- WasmGC vtable による動的ディスパッチは未実装
  - lowering のマングル名解決 (crates/lsharp-ir/src/lower/ のトレイト処理、2026-06-12 確認)
- **関連**: D-01 (WasmGC struct が実装基盤)。imp-01 参照。

<a id="d-07"></a>
### D-07: SCC 推論は部分実装、canonical/native parity が未完

- **影響度**: 中 / **状態**: in-design
- **内容**: `ModuleGraph::scc_groups()` と SCC 単位の compile / incremental / source override
  推論は実装済みで、Formatter 3 モジュールも明示 import と一般 SCC 経路を使う。旧
  `try_infer_formatter_trio_batch` 特例は除去済み。一方、dirty-set には Formatter 固有の
  atomic expansion が残り、canonical `App.Cli` の runtime、native stage0、両 supported target の
  parity は閉じていない。
- **検証済み部分**:
  - `crates/lsharp-ir/src/module_graph.rs` -- deterministic SCC と Formatter dirty-set expansion
  - `crates/lsharp-ir/src/lib.rs` -- SCC compile / incremental / source override inference
  - `test_compile_multi_file_infers_mutual_recursive_scc`
  - `test_compile_multi_file_incremental_infers_mutual_recursive_scc`
- **関連**: I-05 / V2-01 (LSP incremental sync)。改善設計は [imp-04](docs/development/planning/improvement-designs/imp-04-module-system-strengthening.md)。

<a id="d-08"></a>
### D-08: Native backend self-regeneration / differential track

- **影響度**: 中-高 / **状態**: resolved
- **内容**: V2-08〜V2-10 で Darwin arm64 の actual native self-regeneration、
  Wasm/native differential、experimental native-only RC を完了した。V2-13〜V2-15 では
  Mac Apple Silicon と Linux x86_64 の target matrix、official artifact layout、
  actual `App.Cli` / rollback / stable archive smoke を固定した。旧 deferred track 自体に
  残件はなく、完了履歴は TODO ではなく仕様・設計・release 運用文書へ保持する。
- **解消根拠**:
  - `docs/language/native-backend-spec.md` -- 両 supported target の actual self-regeneration と
    official release smoke の current contract
  - `docs/development/planning/v2-designs/v2-08-native-backend-self-regeneration.md`
  - `docs/development/planning/v2-designs/v2-09-wasm-native-differential-zero.md`
  - `docs/development/planning/v2-designs/v2-10-native-only-rc-distribution.md`
  - `test_e2e_native_ops03_official_native_only_replacement_backlog_contract`
  - `test_e2e_native_ops04_linux_x86_server_target_contract`
- **関連**: bootstrap provenance、rooting、public surface の未完境界は D-08 の再オープンではなく、
  TODO.md の `LEGACY-BOOT-01` / `LEGACY-ROOT-01` / `LEGACY-TOOL-01` で追跡する。

<a id="d-09"></a>
### D-09: セルフホストコンパイラの ADT が整数タグ + Vector 表現

- **影響度**: 中 / **状態**: in-design
- **内容**: セルフホストコンパイラでは ADT を WasmGC struct ではなく整数タグ + Vector で
  表現している。ブートストラップ初期の簡略化としては妥当だが、フィールドアクセスの間接化と
  タグ判定コストにより、本来の struct 表現より実行効率が低い。
- **根拠**:
  - `book/ch15-selfhosting.md` -- 整数タグ方式の採用理由 (「WasmGC の struct/subtyping は複雑で、ブートストラップの初期段階では使いにくい」)
- **関連**: D-01 (WasmGC 移行で解消の道筋)。imp-01 参照。

<a id="d-10"></a>
### D-10: GC sentinel/handle 判別の理論的 edge case (G1)

- **影響度**: 低-中 / **状態**: documented-limitation (公式状態を尊重)
- **内容**: ユーザーが `i64::MIN + N` (`heap_start <= N < heap_ptr`) という値を意図的に計算して
  保持すると、subtract 後に heap range へ入り collector に false-mark される。実用上の発生確率は
  ゼロに近く、現状は documented limitation として整理済み。なお S14/S15/S16 (GC 有効 runtime
  stability) は CI artifact による machine-readable 証跡でゲート close 済み。
- **根拠**:
  - `docs/development/planning/runtime-stability-spec.md:278-282` -- G1 の定義と documented limitation 整理
  - `docs/development/planning/completion-criteria.md:121-123` -- S14/S15/S16 gate close の現況
- **関連**: precise discrimination の将来選択肢は runtime-stability-spec.md が正本。[imp-03](docs/development/planning/improvement-designs/imp-03-dynamic-memory-layout.md) で言及。

---

## 実装上の問題

<a id="i-01"></a>
### I-01: ファイルサイズ規約 (500-800 行) の大幅超過

- **影響度**: 高 / **状態**: in-design
- **内容**: CLAUDE.md のファイルサイズ規約 (1 ファイル 500-800 行) を大幅に超えるソースが
  16 ファイルあり、エージェント解析精度・レビュー容易性・責務分離を損なっている。
  主要超過ファイル (src のみ、2026-07-25 実測):

  | ファイル | 行数 | 規約比 |
  |---------|------|--------|
  | `crates/lsharp-wasm/src/wasi.rs` | 4568 | 5.7x |
  | `crates/lsharp-ir/src/lib.rs` | 3080 | 3.9x |
  | `crates/lsharp-tooling/src/compile.rs` | 2870 | 3.6x |
  | `crates/lsharp-ir/src/lower/expr.rs` | 2833 | 3.5x |
  | `crates/lsharp-types/src/infer.rs` | 2789 | 3.5x |
  | `crates/lsharp-driver/src/main.rs` | 2568 | 3.2x |
  | `crates/lsharp-syntax/src/parser.rs` | 2259 | 2.8x |
  | `crates/lsharp-lsp/src/lib.rs` | 1397 | 1.7x |

  残り 8 件は `src/` 配下の test module と driver/tooling/wasmgc surface。`constraints.rs`、
  `macro_expand.rs`、`module_graph.rs`、`host_bridge.rs`、`wasi_runner.rs`、`lower/tests.rs` は
  分割により 800 行以下へ縮小済み。
- **根拠**: `find crates -path '*/src/*.rs' -type f -print0 | xargs -0 wc -l`。
  規約は AGENTS.md のファイルサイズ制限。
- **関連**: selfhost 側は ADR-168 (STR-01〜03) で分割実績あり (TypeInfer.ls 1093 → 290 行など)。
  Rust 側の分割設計は [imp-06](docs/development/planning/improvement-designs/imp-06-large-file-decomposition.md)。

<a id="i-02"></a>
### I-02: 診断 code/span が全 surface に未貫通

- **影響度**: 高 / **状態**: in-design
- **内容**: parser / type inference / lowering / module graph / codegen の `LS####` と
  error-reference、代表的な CLI / LSP / MCP forwarding は実装済み。LSP も parser/type/module の
  representative case では実 range と code を返す。残る問題は、`CodegenError` と一部
  `LowerError` が span を持たないこと、multi-file / REPL / doc / metadata / native linker /
  incremental module・codegen の経路で文字列化や span 消失が残ること、下層本番経路の
  panic-free 契約が閉じていないこと。
- **検証済み部分**:
  - `crates/lsharp-lsp/src/util.rs` -- `diagnostic_error_at` と span → range 変換
  - `syntax_diagnostics_expose_stable_code_and_source_range`
  - `type_diagnostics_expose_stable_code_and_non_empty_source_range`
  - `incremental_module_diagnostics_forward_stable_code`
- **関連**: DOC-06 は error-reference と MCP lookup まで解消済み。残る貫通作業は
  [imp-02](docs/development/planning/improvement-designs/imp-02-error-handling-unification.md) を参照。

<a id="i-03"></a>
### I-03: GC 容量 grow が全 runtime/backend に未貫通

- **影響度**: 高 / **状態**: in-design
- **内容**: `4096` / `32768` は core WASI runtime では初期容量になり、object table /
  free-list / root stack は容量到達時に grow する。memory grow failure の fail-closed trap と
  stable `LS4002` も verified。残る問題は HTTP/component/selfhost/native runtime への同一契約の
  貫通、standalone の dynamic root/data/heap layout、両 supported target の native stage0 evidence。
- **検証済み部分**:
  - `test_e2e_runtime_object_table_grows_past_initial_capacity`
  - `test_e2e_runtime_free_list_grows_past_initial_capacity`
  - `test_e2e_runtime_root_stack_grows_past_initial_capacity`
  - `test_e2e_alloc_memory_grow_failure_reports_ls4002`
- **関連**: memory-management-roadmap.md (GC 実装の正本)。改善設計は [imp-03](docs/development/planning/improvement-designs/imp-03-dynamic-memory-layout.md)。

<a id="i-04"></a>
### I-04: GC フリーリストが線形探索

- **影響度**: 中 / **状態**: in-design
- **内容**: フリーリスト管理が線形走査 (worst case O(n)) で、割り当て頻度の高い
  ワークロードでアロケーションコストが増大する。サイズクラス別リスト等の高速化が未実装。
- **根拠**: `crates/lsharp-wasm/src/wasi.rs` のフリーリスト実装 (GC_FREE_LIST 関連、`:26` 周辺の定数に基づく単一リスト構成)。
- **関連**: I-03 と同じレイアウトに依存。imp-03 参照。

<a id="i-05"></a>
### I-05: Rust host cache は部分実装、selfhost/native persistence が未完

- **影響度**: 中 / **状態**: in-design
- **内容**: SCC-aware `CompilationCache`、dependency surface invalidation、tooling/driver
  `CompileSession`、明示 root の process 間 `ArtifactCache`、Wasm validation/runtime、
  CLI/env opt-in と entry/byte budget は Rust host で verified。残る問題は source override の
  segment/disk persistence、自動 eviction policy、Native artifact、selfhost/native compiler への移植、
  public command と両 supported target の evidence。
- **検証済み部分**:
  - `compile_multi_file_with_cache`
  - `CompileSession::with_artifact_cache`
  - `test_compile_multi_file_with_cache_matches_fresh_and_warm_compile`
  - `test_compile_session_reuses_default_cache_for_multi_file_compile`
- **関連**: D-07 / V2-01 (LSP incremental sync)。imp-04 参照。

<a id="i-06"></a>
### I-06: Property/limit slice はあるが full fuzz/leak/perf gate が未完

- **影響度**: 中 / **状態**: in-design
- **内容**: syntax/types の bounded proptest、固定 seed 4096-case lane、occur-check /
  deep type / wide record、GC capacity、recursion、repeated-start collector の limit lane は
  verified。残る問題は再利用可能な cross-layer generator、常設 full fuzz target、
  長時間 leak/rooting stress と static lint、performance regression threshold、
  selfhost/native stage0 と両 supported target の evidence。
- **根拠**:
  - `scripts/ci/test-property-nightly.sh`
  - `scripts/ci/test-type-inference-limits.sh`
  - `scripts/ci/test-runtime-limits.sh`
  - `scripts/ci/test-runtime-recursion-limits.sh`
- **関連**: I-03 / I-07。改善設計は [imp-07](docs/development/planning/improvement-designs/imp-07-test-verification-infrastructure.md)。

<a id="i-07"></a>
### I-07: rooting guard は部分実装、全 source/runtime 境界が未完

- **影響度**: 中 / **状態**: in-design
- **内容**: selfhost rooting 規約、runtime failure ledger、compiler root-lifetime ledger、
  shadowed `root_set` と allocation-crossing の Rust/Wasm guard は verified。残る問題は
  全 selfhost source の static lint、GC stress mode、stateful REPL/LSP、indirect control flow、
  Mac/Linux native stage0 の同一契約。
- **根拠**:
  - `scripts/ci/test-gc-rooting.sh`
  - `scripts/ci/test-selfhost-rooting-guards.sh`
  - `docs/development/planning/memory-management-roadmap.md`
- **関連**: TODO.md の `LEGACY-ROOT-01`。改善設計は
  [imp-07](docs/development/planning/improvement-designs/imp-07-test-verification-infrastructure.md)
  (rooting 規約の明文化と guard test 拡張)。

<a id="i-08"></a>
### I-08: テストカバレッジの偏り

- **影響度**: 中 / **状態**: in-design
- **内容**: syntax/types の property test、layer-focused test、test distribution report、
  production/test module の分割は進んだが、selfhost native E2E は依然として数万行規模に集中し、
  失敗から原因 layer へ切り分けるコストが高い。
- **根拠**: `wc -l` 実測 (2026-07-25) --
  `selfhost_native_stage_chain.rs` 62327 行、`selfhost_native_differential.rs` 12620 行、
  `selfhost_bootstrap_four_layer.rs` 11779 行。分布の機械可視化は `scripts/test-distribution.py`。
- **関連**: I-01 (テストのインライン配置がファイル肥大の一因)。
  改善設計は [imp-07](docs/development/planning/improvement-designs/imp-07-test-verification-infrastructure.md) (増強方針) と imp-06 (分割方針)。

<a id="i-09"></a>
### I-09: installed package の nested source ownership が未完

- **影響度**: 中 / **状態**: in-design
- **内容**: package entry、`lsharp.toml`、`docs/api.json` の直下 ownership boundary は Rust/native MCP で
  verified partial になったが、regular な `.lsharp/packages/<entry>` 内の `src/` directory、個別 source、
  `docs/` directory、その他 nested tree を package-owned input として扱う規則は閉じていない。外部 directory を
  symlink 経由で source traversal / in-memory package API generation が辿れば、package 外の source identity、content、
  metadata が search / project context / package API へ投影される可能性がある。
- **根拠**:
  - `docs/adr/decisions-v0.3-native-mcp-package-api-regular-file-boundary.md`
  - `docs/adr/decisions-v0.3-native-mcp-installed-package-directory-ownership.md`
  - `docs/adr/decisions-v0.3-native-mcp-package-manifest-symlink-boundary.md`
  - `TODO.md` の `EC-M3-05` / `M3-05-N9` verified partial 記録
- **次の識別実験**: regular package directory 内の `src/` directory symlink と外部 source fixture を一つだけ追加し、
  実際の source traversal surface の既存 not-found / empty / ignore 契約を Rust/native の RED で固定する。個別 source、
  `docs/`、installer、provider、runtime の完了へ拡張しない。
- **残る境界**: installer が取得した tree、live provider/auth、current-source Mac/Linux runtime、packaged/rollback parity、
  実 Ed25519 は未検証であり、`EC-M3-05` と `M3-05-N9` は `[~]` を維持する。

<a id="i-10"></a>
### I-10: `validate --source` project aggregate の selfhost/native parity が未完

- **影響度**: 中-高 / **状態**: in-design
- **内容**: Rust driverは `47743365` で複数の regular `.ls` fileを deterministic な project graphへ集約し、
  file境界を越えた duplicate intent nodeを code `2` と source span付きで fail-closedにできる。一方、validな
  cross-file edgeの report/manifest、duplicate evidence/reviewの source-specific diagnostics、directory inputの
  selfhost App.Cli / EmbeddedCli / MCP投影は未実装または未検証である。Mac Apple Siliconの current-source App.Cli
  runtimeは成功したが、Linux x86_64の current-source fixed-point summaryとdirectory validation runtimeは未取得である。
- **根拠**:
  - `crates/lsharp-driver/src/validation_source_project.rs` -- deterministic `.ls` collection、全 node先行登録、cross-file duplicate検査
  - `crates/lsharp-driver/src/main.rs` -- `--source` file/directory routing と `source validation error:2` diagnostic
  - `crates/lsharp-driver/tests/validate_cli.rs` -- `validate_rejects_project_duplicate_across_source_files`
  - `TODO.md` の `EC-M2-01` / `EC-M2-03` current checkpoint
- **次の識別実験**: 異なる `:intent` IDを持つ2 fileとcross-file edgeを一つの projectとして受理し、deterministic report、
  manifest、node/edge/source provenanceをRust CLIで固定する。selfhost/native directory parityやLinux replayへ同時に拡張しない。
- **残る境界**: valid project aggregate、manifest/MCP/public surface parity、selfhost/native producer、Linux current-source
  fixed point、Mac/Linux packaged provenance parity、Rust-free aggregateは未完了であり、`EC-M2-01` / `EC-M2-03` と
  `V2-16b` / `V2-16c` / `V2-16e` は `[~]` を維持する。

<a id="i-11"></a>
### I-11: `cargo test --workspace` の恒常 FAIL が台帳未記載

- **影響度**: 高 / **状態**: resolved
- **内容**: workspace 全体のテストは恒常的に FAIL を出す。この集合が台帳にも TODO にも
  記録されていなかったため、「workspace GREEN」を受入条件に置いた作業が受入判定できず、
  新規 regression が既知 FAIL に埋もれる状態だった。
  **test 名の完全リストを `docs/development/validation/workspace-expected-failures.txt` に
  正本として固定し、`scripts/ci/check-workspace-baseline.sh` で機械照合できるようにした。**
- **測定方法**: `cargo nextest run --workspace --profile baseline` (`.config/nextest.toml`)。
  nextest を選んだ理由は 3 つ -- process-per-test なので 1 件の crash が binary 全体を
  巻き込まない (`cargo test` では `runtime_allocator_closures` の panic が同一 binary の
  後続を道連れにし、FAIL 集合が実行順に依存していた) / JUnit XML から test 名を機械抽出できる /
  `fail-fast = false` で全 target を完走する。`retries = 0` は必須 -- flaky を成功として
  吸収すると baseline が「安定して落ちるもの」ではなく「たまたま通ったもの」になる。
  doctest は nextest の対象外なので `cargo test --doc` を別途回す。
- **実測 (2026-08-16〜17)**:

  | 区分 | 実行 | FAIL | 所要 |
  |---|---|---|---|
  | 非 e2e (`not binary_id(lsharp-wasm::e2e)`) | 2067 | 38 | 1398.826s |
  | e2e partition 1/6 | 289 | 12 | 3762.748s |
  | e2e partition 2/6 | 316 | 14 | 6945.190s |
  | e2e partition 3/6 | 290 | 12 | 5459.507s |
  | e2e partition 4/6 | 287 | 7 | 3977.506s |
  | e2e partition 5/6 | 302 | 11 | 2609.981s |
  | e2e partition 6/6 | 315 | 14 | 3080.154s |
  | e2e 小計 | **1799** | **70** | 25835.086s (7 時間 11 分) |
  | **合計** | **3866** | **108** | -- |

  e2e を 6 分割したのは、1 プロセスで回すと途中で中断されるため (`cargo test` 実測で
  `1702 passed; 60 failed; 1259 ignored` / 20,275.13s = 5 時間 38 分。出典は
  [`rust-boundary-reduction.md`](docs/development/operations/rust-boundary-reduction.md) の
  `lsharp-wasm --test e2e` 節で、同節には本項が数値と FAIL 帰属の正本である旨を注記した)。`--partition hash:i/6` を i=1..6 で回し、JUnit を結合した。
  分割しても総所要は縮まない (むしろ増える) -- 利点は「途中で殺されても部分成果が残る」点だけ。

- **完走判定 (checker では代替できない)**: `check-workspace-baseline.sh` は
  「どの partition にも入らなかった test」を検出できない。実行されなかった test は
  集合上 pass と区別が付かず、baseline が静かに壊れる。そのため別途、
  `cargo nextest list --workspace --profile baseline -E 'binary_id(lsharp-wasm::e2e)'` の
  run 集合と 6 partition の JUnit 実行集合を**名前の集合として**突き合わせた。

  結果は **expected 1799 / actual 1799 / 取りこぼし 0 / 余剰 0 / 重複 0**。
  各 partition の Summary 行の `run + skipped` も全 6 本で 3059 に一致する。

  内訳の算術: e2e binary の `#[test]` 総数 **3059** = run **1799** + `#[ignore]` **1260**
  (`--run-ignored ignored-only` の実数)。

  なお `grep -rc '#\[ignore'` は **1265** を返し、5 件過大である。差の 5 件はすべて
  `selfhost_lsp_docs_ops.rs` 自身が持つ**文字列リテラル**
  (`prev.starts_with("#[ignore")` 3 箇所 + 検査メッセージ 1 箇所 + それに巻き込まれた 1 件) で、
  属性ではない。**`#[ignore]` ゲートを検査する test 自身が `#[ignore]` の grep を汚す**という
  構図なので、件数の権威は常に `cargo nextest list` 側に置くこと。

  中断された run の成果物は混ぜてはならない。
  `<failure type="test abort" message="... signal 15 (SIGTERM)">` を検出して弾いており、
  今回の 6 本には**中断 0 件**。

- **旧記述 (60 FAIL) との橋渡し**: 本 issue は当初 `cargo test` 実測の **97 FAIL**
  (非 e2e 37 + e2e 60) として起票された。nextest の **108 FAIL** (非 e2e 38 + e2e 70) は
  regression ではない。e2e 側 60 → 70 の +10 の帰属は次のとおり:

  - **2 件**は pristine `a3ae4551` に**定義自体が存在しない**。rebase で upstream から
    入った test である (`test_e2e_selfhost_lsp_type_diagnostics_use_standard_projection` /
    `test_e2e_selfhost_standalone_user_call_after_preview1_import`)。
    e2e binary の `#[test]` 総数も rebase で 3021 → 3059 に増えている。
  - **8 件**は pristine にも定義があるが、旧 60 件には数えられていなかった
    (クラスタ別に `selfhost_native_stage_chain` +4 / `selfhost_lsp_docs_ops` +2 /
    `selfhost_native_stage23_gap` +1 / `strings_patterns_compiler_integration` +1)。
    **この 8 件を個別に特定することはできない** -- 旧計測はクラスタ表しか残しておらず、
    60 件の test 名を記録していないためである。**名前を正本に固定した理由がこれ**。

  候補となる機構は 2 つあり、どちらも実測していない: (a) rebase で入った upstream 変更が
  既存 test を落とした (b) `cargo test` は 1 binary 1 プロセスなので先行 panic が後続を
  道連れにし FAIL として計上されなかった分が、process-per-test の nextest で表面化した
  (`.config/nextest.toml` 冒頭に記録のある既知の挙動)。
  **本ブランチ起因ではないことは構成的に言える** -- `git diff origin/main..HEAD -- crates/ selfhost/`
  は新規ファイル追加 (`embedded_component_cache*` / 移設した `meta_validation.rs`) と
  `lsharp-driver/build.rs` / `lsharp-wasm/src/lib.rs` のみで、
  **既存の e2e test も `selfhost/` も 1 行も変更していない**。
  旧 60 件のクラスタで**件数が減ったものは 1 つも無い**ことも確認済み。

- **旧記述のうち実測で否定された 1 件、および含意だけが誤っていた 1 件**:

  1. **「`selfhost_native_stage_chain` 19 + `selfhost_native_stage23_gap` 9 はローカルに
     `stage0/` が無いことに起因」は誤り。** 実際に落ちているのは `selfhost/src/**.ls` を
     `read_to_string` してソース本文へ文字列 assertion する test で、stage0 も実行も関与しない
     (`selfhost_source_path()` は `tests/e2e/support.rs:717-` で `selfhost/src/...` 固定)。
     `tests/` 内で `./stage0` を参照するのは `selfhost_native_stage_chain.rs:2406` の 1 箇所のみで、
     これは markdown の中身に `"./stage0"` という文字列が載っていることを要求する doc assertion。
     stage0 を実際に食う test は `LSHARP_NATIVE_*` env 経由でのみ artifact を受け取る。
     **正しい環境前提は「`./stage0` 不在」ではなく「`LSHARP_NATIVE_*` が全て未設定」。**
  2. **「`test_support_selfhost_typeinfer_runtime_bundle_cached` は `tests/e2e/support.rs` の
     `mod` 共有で 5 binary へ重複計上される」-- 重複計上そのものは正しい。**
     非 e2e ブロックに実際 5 行あり (`doctools_parity` / `lsp_diagnostic_parity` /
     `lsp_edge_case_parity` / `lsp_stateful_parity` / `property_probe_diagnostic`)、
     `mod support` を持つ binary 数と一致する。e2e を足して**計 6 binary**。
     **誤っていたのは「計上の仕方の問題 = 実害なし」という含意の方**で、6 件はいずれも
     本物の assertion 失敗 (`bundle.contains(selfhost_module("TypeInferApply.ls").trim())`)。
     原因が決定的なソース不一致である以上、`support` mod を取り込む全 binary で
     落ちるのは必然である。原因は `support.rs:1426` の
     `.replace("(import Types.TypeInfer)\n", "")` -- bundle 側は正規化するのに test は生ソースの
     verbatim 包含を要求する。assert は 2026-03-27 `7f9bdbb4` 由来、`replace()` と当該 import 行は
     2026-07-20 `2b0c54b1` が同時に入れたもので、**上流変更に検査が追随しなかった陳腐化**。

- **確定したクラスタ帰属** (件数は今回の実測。注記の全文は
  `workspace-expected-failures.txt` の各 `# [<cluster>]` 行にある):

  | クラスタ | 件数 | 性質 |
  |---|---|---|
  | `selfhost_native_stage_chain` | 23 | 3 系統。`selfhost/src` ソース本文 assertion 16 / x86_64 native codegen の byte 実測 (rel32 解決・function segment metadata marker) 6 / release gate contract の文面 1 |
  | `runtime_allocator_closures` | 17 | `RootLifetime { RootPopUnderflow / ImbalancedExit / BranchDepthMismatch }` -- `LEGACY-ROOT-01`。旧記述と完全に一致した唯一のクラスタ |
  | `default_path_delegation` | 12 | embedded guest default path の selfhost 出力不一致 |
  | `selfhost_native_stage23_gap` | 10 | selfhost codegen の未達。式深度上限 (8 > 7) / harness fixture 不在 / native helper emitter の offset・trailer・prologue 不一致 |
  | insta snapshot | 14 | insta が 2026-05-31 で停止、codegen は 2026-07-27 まで進行 |
  | `selfhost_lsp_docs_ops` | 5 | 4 要因。`TESTGATE-01` / `DIAG-DEDUP-01` (2 件) / 標準 LSP Diagnostic 配列投影の未実装 / release-smoke.sh の boundary 未検証。うち `DIAG-DEDUP-01` の 2 件は **2026-08-18 に解消** (`I-24`)、`TESTGATE-01` 由来の 1 件 (`ops03c`) は **2026-08-19 に解消** (`TESTGATE-03`)。現在は 2 件 / 2 要因 |
  | `strings_patterns_compiler_integration` | 5 | codegen が host `alloc` へ**負の size** を渡す 4 件 (`RootLifetime` とも I-13 の heap 枯渇とも別) + WasmEmit が native 専用 opcode 88 を黙って破棄する 1 件 |
  | `selfhost_cli_core` | 4 | selfhost CLI の未実装挙動への RED。contract suite の canonical/legacy 分離 / unsupported type の実行前報告 / contradicting evidence / import 先 helper の診断。後ろ 3 件は **2026-08-19 の `TESTGATE-03` で当該 test が `#[ignore]` へ移り** default lane から外れたため expected FAIL から削除した (**挙動が直ったわけではない**。phase11 lane で走る)。現在は 1 件 / 1 要因 |
  | `bootstrap_selfhost_lsp_integration` | 2 | selfhost formatter の compile が `UndefinedVar { name: "ast-defn-signature" }`。2 件とも同一 span |
  | `LS0102` | 2 | `lsharp-lsp` と `lsharp-tooling` に跨る |
  | `support` | 6 | 上記の陳腐化 -- `TESTGATE-02`。`mod` 共有により非 e2e 5 binary + e2e の計 6 binary で同一に落ちる (**2026-08-18 解消**) |
  | その他 e2e 単発 | 3 | module graph の topological sort 未達 / preview1 import 後の user call が不正な Wasm / nested module decl の body-count |
  | 非 e2e 単発 | 5 | `lsharp-wasm --lib` の `RootSetWithoutActiveSlot` (`LEGACY-ROOT-01`) / `doctools_parity` の typed metadata 6 vs 5 / `e2e_selfhost_syntax` の nested module decl (e2e 側と同因、binary-id 違い) / `validate_source_review_edges` / `selfhost_cli_validation_contract` (upstream 由来) |
  | **合計** | **108** | e2e 70 + 非 e2e 38 |

  **`#[ignore]` 検査の陳腐化と `support` の陳腐化は production バグではない。**
  CLAUDE.md の 500-800 行制限に沿った mod 分割 (`include!("<name>/part_NNN.rs")`) を
  親ファイルの `read_to_string` で検査する形が壊れたもので、修正は安価。
  **follow-up は `TODO.md` に ID 付きで登録した** -- `TESTGATE-01` / `TESTGATE-02` /
  `DIAG-DEDUP-01`。それ以外はすべて未実装挙動への RED であり、新規 ID は切っていない。
  **3 件とも解消済みで `TODO.md` からは削除した** (`TESTGATE-02` / `DIAG-DEDUP-01` は 2026-08-18、
  `TESTGATE-01` は切り出し先の `TESTGATE-03` を含めて 2026-08-19)。証拠は
  [test gate 是正 ADR](docs/adr/decisions-test-gate-staleness-repair.md) と
  [ignore 契約 ADR](docs/adr/decisions-test-gate-ignore-contract.md) にある。

  **`DIAG-DEDUP-01` は 2026-08-18 に解消した** (`TODO.md` から削除済み)。3 者間の衝突を
  `I-24` として採番し、spec AC-209 の文言のほうが陳腐化していると裁定した。
  `workspace-expected-failures.txt` から 2 エントリを削除している (e2e 52 -> 50 /
  `selfhost_lsp_docs_ops` 5 -> 3)。

  **`TESTGATE-02` は 2026-08-18 に解消した** (`TODO.md` から削除済み)。bundle 正規化を
  単一関数へ寄せ、検査側もそれを通す形へ直した。6 binary すべてで pass を実測し、
  `workspace-expected-failures.txt` から 6 エントリを削除している (e2e 53 -> 52 /
  非 e2e 36 -> 31)。判断と却下案は
  [test gate 陳腐化是正 ADR](docs/adr/decisions-test-gate-staleness-repair.md) が正本。
  なお**上の表は 2026-08-16〜17 時点の実測記録であり、後追いで数値は書き換えない**。

  `TESTGATE-01` については**落ちている 1 件よりも、落ちない側の方が重い**。
  `selfhost_lsp_docs_ops.rs` の検査は 2 モードあり、厳密名モード (`:3761-3769`) は
  一致 0 件で `panic!` するので気付ける (今回の FAIL がこれ) が、**prefix モード
  (`:3791-3794`) は一致 0 件でも何もせず pass する**。実測で、prefix ルール 6 本
  (`selfhost_bootstrap_four_layer` の `test_e2e_boot04_` 71 / `test_e2e_bootstrap_` 45 /
  `test_v2_12_self_hosted_` 9 / `test_v2_11_` 1、`selfhost_native_differential` の
  `test_native_codegen_emits_` 86 / `test_native_emit_object_` 3) が親では 0 件一致、
  fragment 側では**計 215 関数**に当たる。
  **`cargo nextest list` の run 集合 1799 にこの 6 prefix は 1 件も現れない**ことを確認済みで、
  215 件は全て `#[ignore]` 側にある。**live な regression は無い**が、ゲートは空回りしており
  今後の追加漏れを一切検出しない。`include!` のみになっている親は `tests/e2e/` に 5 件あり、
  今後の分割で同じ壊れ方が増える。直し方は既にリポジトリ内にある --
  `selfhost_bootstrap_acceptance_file_size.rs:23-53` が fragment ディレクトリを `read_dir` で
  列挙し、親の `include!` マニフェストとの整合を検証したうえで各 fragment を見る正しい形を
  実装している。

  **2026-08-18 の追記**: `TESTGATE-01` で上記の構造的破損を是正した (厳密名 / prefix の
  両モードを `file_size` 方式へ寄せ、一致 0 件の prefix を `dead_prefix_rules` として落とす)。
  是正後 `dead_prefix_rules` は 0 件で、215 関数の無言無効化は解消した。
  ただし**「live な regression は無い」は分割済み 4 親だけを見た測定だった**。
  検査が復活すると、非分割ファイルに隠れていた**本物の違反 164 件**が現れた
  (`selfhost_cli_core.rs` 158 / `selfhost_cli_actual_main_args.rs` 5 /
  `selfhost_native_stage_chain.rs` 1)。この 164 件をどう扱うかは規約側の判断なので
  `I-22` / `TESTGATE-03` へ切り出した。**本作業 (`TESTGATE-01`) の前後で baseline の
  FAIL 集合は変わらない** — `ops03c` は expected FAIL のまま残った。
  その後 2026-08-19 に `TESTGATE-03` が案 A で実装され、`ops03c` は GREEN になって
  expected FAIL からも外れている。

- **個々の FAIL の修正はスコープ外。** 特に snapshot 14 件は `cargo insta accept` 一発で
  消えるが、2 ヶ月分の未レビューな codegen 出力を追認することになるので**やらない**。
  expected-failure として理由付きで記録するに留める。

- **測定中の HEAD 跨ぎについて**: partition 2 は走行中に HEAD が `9b35413f` → `b69d9b0d` へ動いた
  (レビュー由来の doc 修正 2 件)。この repo には markdown の literal を assert する test が
  実在する (`selfhost_native_stage_chain.rs:2406`) ため確認が必要だったが、
  `git grep -ln 'AGENTS\.md\|\.config/nextest\.toml\|workspace-expected-failures' -- 'crates/*/tests' 'crates/*/src'`
  は **0 件**で、変更した 2 ファイルを読む test は存在しない。測定値に対して無害。

- **関連**: `LEGACY-ROOT-01` / `LEGACY-BOOT-01` / I-08 / `TESTGATE-01` / `TESTGATE-02` /
  `DIAG-DEDUP-01` (= `I-24`)。
  正本は [`workspace-expected-failures.txt`](docs/development/validation/workspace-expected-failures.txt)、
  照合は `scripts/ci/check-workspace-baseline.sh`。

<a id="i-12"></a>
### I-12: ビルド再現性の綻び (`Cargo.lock` 非追跡 / dead test file)

- **影響度**: 低-中 / **状態**: resolved
- **内容**: 二つの独立した綻び。
  - `Cargo.lock` が `.gitignore:9` で除外されている。fresh clone / CI のたびに依存解決がやり直され、
    解決結果が日によって変わるため cold build のキャッシュヒット率が下がり、bootstrap/oracle lane の
    再現性も担保できない。application workspace として追跡対象にするのが Cargo の推奨である。
  - root の `tests/meta_validation.rs` はルート `Cargo.toml` に `[package]` が無いためコンパイル
    されていない dead file である。
- **根拠**: 2026-08-16 実測 (Track 0 調査の副産物)。
- **解決** (2026-08-16):
  - `.gitignore` から `Cargo.lock` の行を除去し追跡対象にした。`adr-rust-removal.md:55` の維持スコープ表が
    `Cargo.lock` を「物理削除しない」と宣言していることとも整合する。
  - dead file は**削除ではなく移設**した。root の `tests/meta_validation.rs` が持っていた
    `TEST-META-02` (completion marker の 3状態管理) は、生きている
    `crates/lsharp-wasm/tests/meta_validation.rs` (TEST-META-01/03/04/05/06 を保持) に**存在しなかった**。
    削除すれば検証が 1 つ失われるため、`CARGO_MANIFEST_DIR` 起点の path 解決へ書き換えたうえで
    live 側へ移し、root の dead file を落とした。移設後に実行して 6 件全て pass することを確認済み
    (移設前は 5 件。`TEST-META-02` はこれまで一度も実行されていなかった)。
- **関連**: I-11 (どちらも「テストが本当に何を検証しているか」の可視性を下げる)。

---

<a id="i-13"></a>
### I-13: native aarch64 の linear heap に回収機構と bounds check が無く、実務サイズの入力で segfault する

- **影響度**: 高 / **状態**: documented-limitation
- **内容**: native stage0 に実務サイズの入力 (`selfhost/src/App/Cli.ls`) を食わせると、
  stage bootstrap (約 49s) が成功したあと materialize 済み program が exit 139 で落ちる。
  初回の切り分けは `App/Cli.ls` が 2,288 行 / 115 KB、stage0 が `d87cd5d1` の時点で行った。
  その後 `origin/main` へ rebase して `App/Cli.ls` は **2,556 行** (+268) になり、stage0 も
  `d55159b6` で作り直したが、**再現性は変わらず exit 139 のまま**である (2026-08-16 再確認)。

  ```
  bash scripts/native-selfhost-dev.sh \
    --stage0-dir ci-artifacts/native-stage0/aarch64-apple-darwin/current \
    check selfhost/src/App/Cli.ls
  ```

  ```
  scripts/native-selfhost-dev.sh: line 449: 65772 Segmentation fault: 11  "$STAGE_DIR/program.native" "$@"
  [exited with code 139]
  ```

  (`line 449` は当時の実測ログの引用。現在の該当行は `scripts/native-selfhost-dev.sh:492`。)

- **原因 (確定)**: **linear heap (bump allocator) の枯渇**。当初の 3 仮説のうち 2 つは実測で棄却した。

  | 仮説 | 判定 | 根拠 |
  |---|---|---|
  | root stack overflow (8 MiB) | 棄却 | `x27-x28` = 1,249,744 / 8,388,608 = **14.9%** しか使っていない |
  | native thread stack overflow (128 MiB) | 棄却 | fault は SP から約 20 GB 離れた位置への **byte read** (`esr 0x92000007`) |
  | linear heap (4 GiB) の枯渇 | **確定** | fault address が heap 終端の **+1 byte**。`vmRegionInfo` も `MALLOC_LARGE` 直後の未マップ域 |

  既定 4 GiB での crash: `KERN_INVALID_ADDRESS at 0xc48010001`、heap base `x21 = 0xb48000000`、
  `alloc_size = 0x1_0001_0000` (= 4 GiB + 64 KiB) より heap 終端 `0xc48010000` — 差は **+1 byte**。
  `alloc_size` は `scripts/ci/materialize-native-macos-aarch64-bundle.py:44-47` の式と
  `stage-data.bin` の実測 29,052 bytes から正確に再現できる。

- **heap 拡大では解決しない (実測)**: stage0 同梱 materializer の `0x1_0000_0000` を `0x2_0000_0000` (8 GiB) に
  patch し、生成された `program.s` の `_lsharp_alloc_size: .quad 8590000128` で反映を確認したうえで再実行しても、
  **拡大した heap をちょうど使い切って同じ様態で落ちた** (fault address − heap 終端 = **0**)。
  構造的な理由がある。
  - materializer に `free` / `munmap` / frontier の reset が**一切無い**。`calloc` 1 回 + bump のみの
    純粋な bump allocator で**回収機構が存在しない**。消費量は生存データ量ではなく
    **累積確保回数**に比例するため、heap 拡大は先送りにしかならない。
  - **2026-08-19 追記: これは native lane 固有の欠落ではない。** wasm lane には mark-sweep
    collector が実装済みだが (`crates/lsharp-wasm/src/wasi/gc_collect_core.rs` /
    `gc_mark.rs` / `free_list.rs`)、その唯一の呼び出し元は
    `wasi/compiler_world/code.rs:131,150` — **`main` から return した後**と
    `proc_exit(0)` の中だけである。実行中には一度も走らず、heap 圧力を下げない。
    **wasm lane が落ちないのは回収しているからではなく `memory.grow` できるからで**、
    両 lane に共通する「実行中は回収しない」設計が、grow できない native lane でだけ
    crash として顕在化している。実行中に回せない理由は compiler-side の GC safe point が
    未完だからで (`phase11-implementation-plan.md:713` の S14-S16)、これも両 lane 共通である。
- **bounds check も無い**: `selfhost/src/Backend/Native/NativeCodegen.ls:14512-14548`
  `emit-aarch64-selfhost-alloc-helper` は 18 word / 72 bytes ちょうど (`:20361` の
  `append-native-bytes-rooted ... 72` と一致)。全 word を decode しても **limit 比較も条件分岐も無い**。
  唯一の分岐は heap base の非ゼロ判定 `CBNZ x21` のみ。対照的に x86 側の
  `emit-x86-selfhost-vector-new-helper:9635` / `emit-x86-selfhost-string-concat-helper:9861` は
  「先頭 16 bytes は cursor/limit」「cursor/limit の範囲へ bump allocate」とコメントで limit を明示しており、
  **aarch64 側だけ非対称**である。よって heap 終端を越えた確保が検出されず、未マップページへ触れて SIGSEGV になる。

  **2026-08-19 追記: 欠落は `alloc-helper` 単独ではない。** x86 側を「コメントで limit を明示している」
  としか確認していなかったので、両 lane の機械語を直接 decode し直した。まず 3 helper を調べて
  3 つとも該当したので「対象は 3 helper」と書いたが、これは undercount だった。
  emitter の S 式を評価して**全 selfhost helper のバイト列を組み立て**、frontier を進める命令
  (aarch64 `add x22, x22, xN` / x86 `mov [r14], rN`) を機械的に数え直した結果
  (`python3 scripts/native_codegen_bytes.py --list` で再現できる):

  | lane | frontier を進める helper | bump 箇所 | limit を参照する箇所 |
  |---|---|---|---|
  | x86-64 | 9 | 9 | **9 / 9** |
  | aarch64 | 10 | 11 | **0 / 11** |

  なお aarch64 の 10 のうち `string-concat-helper-chunk3` は呼び出し元 0 ([I-25](#i-25)) なので、
  **実際に bounds check を入れる対象は両 lane とも 9 helper** である。

  **aarch64 は確保系 helper の全部に limit 比較が無い。** 検出された `cmp` 3 つは
  length vs capacity / 倍化の下限クランプ / frontier とのタグ判別で、いずれも上限比較ではない。
  そもそも **aarch64 lane には上限値を保持する場所が無い** — x86 が heap 先頭 16 bytes に
  cursor/limit を置くのに対し、aarch64 は `x21` (base) と `x22` (frontier) しか持たない。
  よって `NATIVE-HEAP-01` は「比較を足す」ではなく「**上限値をどこに置くか**」から始まる。
  確保サイズ・成長ポリシーの lane 別実測と全列挙の内訳は
  [`decisions-native-heap-reclamation.md`](docs/adr/decisions-native-heap-reclamation.md) にある。
  なお同じ走査で呼び出し元 0 の defn が 64 個あることも分かった ([I-25](#i-25))。
  最初は 4 つと書いたが、それは string-concat chunk 群だけを見た undercount だった。
- **切り分け済みの周辺事実**:
  - stage bootstrap 自体は成功する。stage0 package は壊れていない。
  - 小さい fixture (`tests/fixtures/validation/*.ls`) を使う
    `scripts/ci/native-selfhost-dev-source-file-smoke.sh` は exit 0 で通る。
  - `materialize-native-macos-aarch64-bundle.py` には heap / root stack / native stack の
    env override が無い。Linux x86 側は `materialize-native-linux-x86-bundle.py:9` に
    `LSHARP_NATIVE_LINUX_X86_ACTUAL_HEAP_BYTES` を持つ。ただし上記実測より、
    **env knob は本ワークロードの緩和策にならない**。
- **根拠**: 2026-08-16 実測 (T1-1 の副産物 + 2 回の crash report 解析)。
- **帰結する未完項目**: TODO.md の `NATIVE-HEAP-01` (aarch64 の確保系 helper 全部の bounds check) と
  `NATIVE-HEAP-02` (回収機構の設計)。前者は症状を診断メッセージへ変えるだけで、
  「115 KB の入力に 8 GiB 超」という増幅そのものは解消しない。
  後者の設計は [`decisions-native-heap-reclamation.md`](docs/adr/decisions-native-heap-reclamation.md)
  で in-design。**「累積確保回数に比例」に機構を与える主要容疑は aarch64 `map-new` の
  無条件 65,536 bytes 確保** (resize path 無し、`map-insert` / `map-remove` は in-place で 0 確保)
  だが、静的な数え上げでは 8 GiB に届かないため帰属は動的計測待ちである。
- **関連**: `LEGACY-IO-01` (dynamic root/data/heap layout)、`LEGACY-ROOT-01` (rooting discipline)、
  [`decisions-dev-loop-rust-free-daily-lane.md`](docs/adr/decisions-dev-loop-rust-free-daily-lane.md)。

---

<a id="i-14"></a>
### I-14: root lifetime verifier が公開 runtime API の合法な使用を拒否する

- **影響度**: 中-高 / **状態**: resolved
- **内容**: `crates/lsharp-ir/src/root_lifetime.rs` は「関数の出口で root stack が空であること」を
  無条件に要求する (`ImbalancedExit`)。しかし `root_push` / `root_pop` / `root_set` は
  [`docs/language/runtime-spec.md:78-79,84,142`](docs/language/runtime-spec.md) が定める**公開 runtime API**
  であり、**均衡を要求する記述はどこにも無い**。結果として、仕様どおりに書かれたユーザーコードが
  lowering 段階で `LS3003` に落ちる。
- **判別測定 (2026-08-17)**: `runtime_allocator_closures` の恒常 FAIL 17 件について、
  verifier を一時無効化した build と対照した。

  | 条件 | 結果 |
  |---|---|
  | verifier 有効 (現状) | `17 failed` / rc=101 |
  | verifier 無効 (`validate_module` を no-op 化) | **`94 passed; 0 failed`** / rc=0 |

  **17 件すべてが verifier 起因で、その裏に隠れた本物の欠陥は 0 件**である。
  patch は測定用スクリプト内で適用し、終了時に必ず逆適用した (tracked diff 空を事後確認済み)。

- **内訳** (`validate_module` は fail-fast なので、各 test の error kind は第一報告のみ):

  | error kind | 関数 | 件数 | 対応 |
  |---|---|---|---|
  | `ImbalancedExit` | `main` | 13 | 案 E (本 ADR) |
  | `RootPopUnderflow` | `main` | 1 | 案 B1 (別スライス) |
  | `BranchDepthMismatch` | `push-roots` ×2 / `alloc-rooted` ×1 | 3 | 案 B1 (別スライス) |

- **原因**: **verifier が拒否する形と、fixture が意図して書いている形は構文が同一で、期待だけが正反対**である。
  自動判別できる信号は IR 上に存在しない。これが本件の核心である。

  ```lisp
  (defn main []
    (let [keep (string-concat "keep" "!")
          _slot (root_push keep)]     ; プログラム終了まで保持する。pop しないのが仕様
      (churn 200)
      (print-string keep)))
  ```

  ```lisp
  (defn alloc-rooted [n]              ; object table を溢れさせるため意図的に root を積み増す
    (if (> n 0)
        (let [value (alloc 8) _ (root_push value)] (alloc-rooted (- n 1)))
        0))
  ```

- **verifier を消してはいけない根拠**: `f8234503` はこの verifier で
  `selfhost/src/Backend/Wasm/Compiler.ls` の手書き `(root_pop)` 個数バグを **4 件**捕まえている
  (`compile-user-call-arg-instrs-step-with-source:119` / `compile-recordupdate-with-ftable:1789` /
  `register-adt-variants:3522` / `compile-let-with-ftable-impl-body-impl-3:4368`)。
  selfhost の 38 ファイルが root API を直接使っており、検査を外すと**この 4 件の再発を検出できなくなる**。
- **第 1 段の結果 (2026-08-18)**: `main` の出口検査だけを免除する案 E を実施し、
  **17 件中 13 件を解消**した (`90 passed; 4 failed`)。baseline は 108 → 95 entry、
  `scripts/ci/test-gc-rooting.sh` は rc=101 → rc=0。判定・却下理由・実測は
  [`decisions-root-lifetime-main-exit-exemption.md`](docs/adr/decisions-root-lifetime-main-exit-exemption.md)。
- **第 2 段の結果 (2026-08-18)**: 残る 4 件は `:roots-unbalanced "<理由>"` を
  `defn` の metadata directive として追加し、fixture 側が意図を宣言する形で解消した。
  免除集合は IR の struct field ではなく `validate_module` の第 2 引数で渡す。
  実測 **`94 passed; 0 failed`** で、`scripts/ci/test-runtime-limits.sh` は rc=101 → rc=0。
  baseline は 94 → 90 entry。判定・却下理由・満たせなかった受入条件は
  [意図的不均衡の注釈 ADR](docs/adr/decisions-root-lifetime-intentional-imbalance-annotation.md)
  が正本。**これで本項の 17 件は全て解消した。**
- **副産物**: 注釈を要したのは 4 fixture / **5 関数**で、1 つ多いのは
  `..._root_stack_growth_preserves_root_api` の `main` が関数間 lease を消費していたため。
  既存の lease helper 2 件と同じ形がユーザーコード側にも現れることの実証で、
  helper の名前ハードコードを注釈へ寄せる動機になる (未着手)。
- **派生**: `RootPopUnderflow` の位置づけを決める過程で、runtime spec が root API の
  境界挙動を定義していないことが判明した (I-17)。空 stack への `root_pop` の 1 項目だけ
  本スライスで spec へ引き上げた。残る 3 項目は 2026-08-18 に I-17 で解決済み。
- **関連**: `LEGACY-ROOT-01` (TODO.md)、I-07 (rooting guard の未完)、I-11 (baseline の由来)。

---

<a id="i-15"></a>
### I-15: `default-path-smoke` が guest 経路を前提に書かれ、既定経路を検査していない

- **影響度**: 中 / **状態**: resolved
- **内容**: `scripts/ci/default-path-smoke.sh:38-43,53-58` は `lsharp compile` /
  `lsharp build` の**既定 target** (`wasi-component`) の stdout に `wasm-size:` を要求する。
  しかし `wasm-size:` を出すのは embedded guest だけであり、**guest は既定 target を
  一切遂行しない**。`selfhost/src/App/EmbeddedCli.ls:1230` の `run-compile-output` は
  target が `preview1` でなければ**無条件に**
  `error: wasi-component output requires external component packaging` を出して非 0 終了する
  (`:1215` の `component-output-boundary-message`)。`run-build-output` (`:1231`) はその別名なので
  `build` も同一挙動。
- **含意**: このゲートは「既定経路が壊れたら落ちる」ものではなく、**書かれた条件が
  最初から成立しない**。`crates/lsharp-driver/src/main.rs:900-928` の設計どおり
  driver は host compile へ fallback し `コンパイル成功: ... (18506 bytes)` を出すので、
  assertion は常に落ちる。結果として `scripts/ci/test-fresh-clone.sh:132-134` の
  clean-checkout 経路も rc=1 になる。
- **実測 (2026-08-18)**: worktree `codex/gate-fixes-root-lifetime` の `target/debug/lsharp` で 4 経路を測定した。

  | 呼び出し | stdout | 経路 | 出力先頭 4 byte |
  |---|---|---|---|
  | `compile examples/fib.ls -o <out>.component.wasm` | `コンパイル成功: ... (18506 bytes)` | host fallback | `0061736d` |
  | `compile examples/fib.ls --target wasi-preview1 -o <out>.wasm` | `wasm-size:2904` | guest | `0061736d` |
  | `build examples/fib.ls --output <out>.component.wasm` | `コンパイル成功: ... (18506 bytes)` | host fallback | `0061736d` |
  | `build examples/fib.ls --target wasi-preview1 --output <out>.wasm` | `wasm-size:2904` | guest | `0061736d` |

  4 経路とも rc=0 で有効な Wasm を書き出す。**壊れているのは製品ではなくゲートの条件式**である。
  `--target wasi-preview1` は `CliCompileTarget` の正規の variant で、`build` も受理する。
- **取り下げた旧仮説**: 「CI が緑なのは `LSHARP_EMBED_COMPONENT_PATH` 由来の packaged component を
  積んだバイナリで回しているから」と見ていたが、**誤り**。boundary は guest 側で無条件なので
  packaged binary でも同じ結果になる。CI が緑だった実際の理由は
  **`default-path-smoke` job が走っていなかった**こと — 直近 5 run はいずれも 2026-07-12 で、
  問題の 2 commit より前であり、job は skipped だった。
- **CI 側の欠落**: `.github/workflows/ci.yml` の `default-path-smoke` job には
  `cargo build --bin lsharp` step が無く、`LSHARP_BIN` も渡していない。
  `needs: [test]` は順序を作るだけで `target/` を共有しない
  (`Swatinem/rust-cache` は workspace member の binary を保持しない)。
  つまり job が実際に走っても、script は `:24-28` の「binary が無い」で落ちる。
- **腐敗は 3 層あった (2026-08-18、script を完走させて判明)**: 当初は既定経路の
  assertion 1 件と見ていたが、ゲートが一度も走らないあいだに次の 3 層が積み上がっていた。

  | 層 | 箇所 | 症状 |
  |---|---|---|
  | 1 | `:39` / `:54` の既定経路 assertion | 条件が原理的に成立しない |
  | 2 | guest `check` / `test` の assertion | 期待が plain text のままで、製品は structured JSON へ移行済み |
  | 3 | `selfhost/src/App/SmokeCli.ls` | `(import App.CompilerMode)` 欠落でソースがコンパイルできない |

  第 2 層の契約の正本は `crates/lsharp-wasm/tests/e2e/selfhost_bootstrap_contracts.rs:234-235`
  (check JSON の `failureKinds`) と `docs/development/planning/v0.2-evidence-contracts.md:165-180`
  (test の v0.2 assurance report)。script 側の期待は 2026-03-31 `bc752767` 由来で、
  **契約側が後から動いたのに script が追随していなかった**。
  第 3 層は `build-wasm-bytes-wasi` (`selfhost/src/App/CompilerMode.ls:6089`) を
  import 無しで呼んでおり、`App/Main.ls` / `App/Cli.ls` / `App/EmbeddedCli.ls` /
  `App/PipelineSmoke.ls` の 4 兄弟は同 import を持つ。`SmokeCli.ls` だけが欠けていた。
- **解消 (2026-08-18)**: 3 層すべてを修正し、`scripts/ci/default-path-smoke.sh` が **rc=0** で
  完走することを実測した。`ci.yml` の `default-path-smoke` job に `cargo build --bin lsharp` を追加。
  第 3 層の修正で `lsharp-driver::default_path_delegation
  test_driver_delegates_to_wasm_cli_artifact_via_lsharp_path` が pass へ転じたため、
  baseline から 1 行削除 (95 → 94)。判断と却下理由、満たせなかった受入条件は
  [default-path-smoke 決定論化 ADR](docs/adr/decisions-default-path-smoke-determinism.md) が正本。
- **skipped の原因 (2026-08-18 解明)**: job 側の条件ではなく、**workflow 全体が起動しない**
  のが答えだった。`6651109b` (2026-07-12) が `ci.yml` から `push` / `pull_request` トリガを外し、
  `workflow_dispatch` 限定にしている (`name: CI (manual only)`)。
  `needs: [test]` の上流状態でも path filter でもない。
  本ブランチを push しても run は 0 件で、リポジトリ全体の最新 run は 3 件とも 2026-07-12 だった。
  **「直近 5 run で skipped」という当初の読み自体が不正確**で、正しくは
  「その 5 run より後、job は一度も実行対象になっていない」である。
  停止方針そのものは意図的かつ記録済みで、観測されない 17 job という副作用は I-19 が正本。
- **残る未確定**: job が実際に緑になることは依然として未確認である。CI 停止中は push で
  確かめられないため、確認には `workflow_dispatch` の手動起動が要る (`SMOKE-GATE-03`)。
- **関連**: I-19 (CI 停止の副作用)、`SMOKE-GATE-03` (TODO.md、1 run 観測の残件)、
  `OPS-05` (job の由来)、I-11 (baseline の由来)。

---

<a id="i-16"></a>
### I-16: embedded component cache の key が build 入力を覆いきっていない

- **影響度**: 中 / **状態**: resolved
- **内容**: `crates/lsharp-driver/build.rs:43-59` の `cached_default_embedded_component` は
  cache key を `collect_source_entries("selfhost/src", ...)` と
  `current_executable_fingerprint()` の 2 つだけから導く。しかし埋め込む component の
  生成には他の入力がある。
- **`wit/` — 実入力なのに key に無い**: `build_default_embedded_component` が呼ぶ
  `emit_wasm_wasi_p2` (`crates/lsharp-wasm/src/wasi.rs:239-243`) は
  `wit/lsharp-compiler.wit` を読み、`component_adapter::resolve_world`
  (`crates/lsharp-wasm/src/component_adapter.rs:50-70`) がその WIT workspace
  (`wit/deps` を含む) を解決する。build.rs は `:120-123` で `wit/` に
  `rerun-if-changed` を張っているので、**wit だけを編集すると build script は再実行され、
  しかし key が変わらないので古い bytes を hit させる**。これが最も現実的な stale 経路である。
- **`stdlib/` — 潜在入力で、しかも `rerun-if-changed` すら無い**:
  `ModuleGraph::resolve_module_file_with_search_paths`
  (`crates/lsharp-ir/src/module_graph/resolve.rs:138-161`) の探索順は
  source_root -> packages -> stdlib で、`default_stdlib_root()` (`:486-497`) が
  bundled `stdlib/` を既定で載せる。key に入っていないうえ、build.rs は `stdlib/` に
  `rerun-if-changed` を**張っていない**。したがって key に足すだけでは不十分で、
  build script が再実行される保証も同時に要る (再実行されなければ OUT_DIR の
  古い artifact がそのまま使われる)。
- **実測 (2026-08-18) — 旧記述の訂正**: `TODO.md` の `EMBEDCACHE-01` は
  「`compile_multi_file` は `stdlib/` も読む」と書いていたが、**この tree では成立しない**。
  `selfhost/src` 配下の `(import ...)` / `(open ...)` は 254 箇所・異なるモジュール 56 件で、
  **56 件すべてが `selfhost/src/` 配下で解決する** (`stdlib/` にしか無いものは 0 件、
  どちらにも無いものも 0 件)。現時点で stdlib は 1 ファイルも読まれていない。
  ただし探索パスには載っているので、selfhost 側の module が 1 つ消える、あるいは
  stdlib にしか無い module を import した瞬間に実入力へ変わる。
- **共通の原因**: 「build script を再実行させる集合 (`rerun-if-changed`)」と
  「key に入る集合」が**別々に書かれており、一致を保つ仕組みが無い**。
  `wit/` は前者にだけ、`selfhost/src` は両方に、`stdlib/` はどちらにも無い、
  という食い違いが現に起きている。
- **実害**: 未観測。cache を退避した fresh build と cache hit build のバイト一致は確認済み。
  観測されていないのは、これまで `wit/` を触る変更と selfhost を触る変更が
  たまたま同時に入っていたためと見る。
- **解消 (2026-08-18)**: 入力 root の一覧を
  `lsharp_wasm::embedded_component_cache::EMBEDDED_COMPONENT_KEY_ROOTS`
  (`["selfhost/src", "stdlib", "wit"]`) に集約し、key 導出 (`embedded_component_key_sources`) と
  `build.rs` の `rerun-if-changed` の**両方をそこから導く**ようにした。
  これで「片方にだけ root が載っている」状態が構造的に作れなくなる。
  実測: `stdlib/List.ls` の mtime だけを触ると build script が再実行され (新設の
  `rerun-if-changed` が効いている証拠) 内容不変なので cache hit (3.3s)。
  `wit/lsharp-compiler.wit` を 1 行変えると key が `b3c50215...` から `e2326cd3...` へ分岐し、
  復元すると元の key へ戻る。`stdlib/List.ls` の内容変更では miss (1m27s の再コンパイル)。
  unit test 6 本を追加 (`cargo test -p lsharp-wasm --lib embedded_component_cache` →
  22 passed)。判断・却下理由・満たせなかった受入条件は
  [cache key 被覆 ADR](docs/adr/decisions-embedded-component-cache-key-coverage.md) が正本。
  **stale hit が実際に起きる様子は再現していない** (旧 build.rs との対照は emitter fingerprint が
  変わるため成立しない)。`packages/` 探索パスは引き続き key の外にある。
- **関連**: [cache key 被覆 ADR](docs/adr/decisions-embedded-component-cache-key-coverage.md)。

---

<a id="i-17"></a>
### I-17: runtime spec が root 管理 API の戻り値と境界挙動を定義していない

- **影響度**: 中 / **状態**: resolved (2026-08-18)
- **内容**: [runtime spec](docs/language/runtime-spec.md) の root 管理 API は
  一行の役割表と「GC 未導入段階では no-op 互換実装を許容する」だけで、
  **戻り値も境界挙動も定義していなかった**。一方
  `crates/lsharp-wasm/src/wasi/root.rs` の emitter はどちらも具体的に決めている。

  | 事項 | 実装 (`wasi/root.rs`) | 2026-08-18 時点の spec |
  |---|---|---|
  | 空 stack への `root_pop` | `top == 0` を分岐し top を動かさず `0` を返す | tier 1 項目 3 |
  | `root_push` の戻り値 | push 前の top を i64 で返す | tier 1 項目 1 |
  | `root_set` の戻り値 | 書き込んだ slot を i64 で返す | tier 1 項目 2 |
  | root stack の容量上限と grow | 初期 32768 slot / 倍々拡張 / 確保失敗で trap | tier 1 項目 4 (数値は契約外) |
  | `root_set` 失敗の観測 | failure ledger + trap | tier 2 項目 5/6 |

- **なぜ問題だったか**: 契約が実装より薄いので、**test が spec の無い挙動を pin している**
  状態になっていた。実例が `test_e2e_root_runtime_api_tracks_slots_and_values` で、
  空 stack への `root_pop` が `0` を返すことを assertion に含む。
- **解決 (2026-08-18)**: 残っていた 3 項目を spec へ書いた。契約は
  **tier 1 (全 backend 必須) / tier 2 (観測可能性、backend 任意)** の二段に分けてある。
  判断と却下案は [root API 契約 ADR](docs/adr/decisions-runtime-spec-root-api-contract.md) が正本。
- **本項の記述のうち、実測で不正確だと分かったもの (訂正済み)**:

  1. 「`root_set` の失敗 → **failure ledger へ記録してから trap する**」は不正確だった。
     `failure_slot` / `failure_top` は bounds check の**前に、成功・失敗を問わず毎回**
     `GlobalSet` される scratch であり、失敗時にのみ増えるのは `failure_count` だけである
     (`crates/lsharp-wasm/src/wasi/root.rs:190-224`)。
     このため spec は「`failure_count > 0` のときにのみ意味を持つ」という形で契約化し、
     無条件書き込みは契約へ昇格させなかった (ADR の却下案 C)。
  2. 「**native backend は現時点で `lsharp_root_pop` を実装していない**ため、
     この食い違いは backend 間の非互換として顕在化していない」は
     **シンボルとしては真だが、挙動としては偽**だった。aarch64 lane は IR opcode 74/75/76 を
     インライン展開し、8 MiB の bss root stack を実際に持っている。しかも
     その `root_pop` は空 stack ガードを持たない。**非互換は既に顕在化している。**
     詳細は I-21 が正本。
- **関連**: I-14 (verifier と公開 API の食い違い)、I-07 (rooting guard の未完)、
  I-21 (native backend の非適合)。

---

<a id="i-18"></a>
### I-18: metadata directive の allowlist が 3 系統で二重管理されている

- **影響度**: 中 / **状態**: open (parity test で見張っているが、二重管理そのものは残る)
- **内容**: `:` で始まる metadata directive を受理するかの判定表が手書きで複数箇所に存在する。
  当初 2 箇所と記録していたが、2026-08-18 の調査で **3 系統**あることが分かった。

  | 系統 | 場所 | 役割 | 件数 |
  |---|---|---|---|
  | `decl` | `crates/lsharp-syntax/src/parser/decl.rs` の `is_colon_directive` | directive として受理するか | 29 |
  | `metadata` | `crates/lsharp-syntax/src/parser/metadata.rs` の `try_parse_metadata` | 受理したものをどう読むか | 27 |
  | `selfhost` | `selfhost/src/Syntax/Parser.ls` の `directive-symbol-v3` + `source-directive-symbol-v3` | selfhost front end の受理判定 | 28 |

  片方だけに directive を足すと、同じソースが front end によって通ったり落ちたりする。
  directive でない `:` は戻り値型注釈として読まれるため、食い違いは「未知の directive」ではなく
  **型注釈の parse error** として現れ、原因が読み取りにくい。
- **顕在化した実例**: 2026-08-18 に追加した `:roots-unbalanced` は Rust parser にだけ入れた
  (selfhost source を編集すると embedded component の再ビルドと cache key の再計算を巻き込むため、
  [意図的不均衡の注釈 ADR](docs/adr/decisions-root-lifetime-intentional-imbalance-annotation.md)
  の「含めない範囲」に置いた)。現時点で本 directive を使うのは Rust 側の e2e fixture だけなので
  実害は出ていないが、**divergence が 1 件ある状態が既に始まっている**。
- **3 者は正しく運用していても一致しない**: `where` / `constraints` は lexer が専用トークン
  (`TokenKind::Where` / `TokenKind::Constraints`) へ落とすため、`is_colon_directive` で実際に効くのは
  トークン側の腕であり、`matches!` 内の文字列腕は**到達しない死んだ枝**である。
  `try_parse_metadata` は `Some(TokenKind::Symbol(_))` しか見ないので、この 2 つがそちらに
  無いのは正しい。**完全一致を要求する検査は実装が正しいまま赤くなる。**
- **手当て (2026-08-18)**: `crates/lsharp-syntax/tests/metadata_directive_parity.rs` を置き、
  3 系統をすべて text 抽出して**ペアごとの差分**を pin した。新しい片側追加は検出できる。
  判断と却下案は [directive allowlist parity ADR](docs/adr/decisions-parser-directive-allowlist-parity.md) が正本。
  **二重管理そのものは解消していない** — 一覧は 3 系統のまま残り、正本化は未着手。
- **直し方の方向**: 一覧を単一の正本 (data file か、片方から生成) に寄せる。
  ただし正本化は lexer の予約語経路と selfhost の payload 処理の差異を先に整理しないと設計できない。
- **関連**: I-14 (`:roots-unbalanced` の導入経緯)、I-20 (受理と読み取りの乖離)、
  `LEGACY-MODULE-01` (selfhost 側の変更コスト)。

---

<a id="i-19"></a>
### I-19: CI 自動実行の停止で `ci.yml` の 17 job が観測されない状態が続いている

- **影響度**: 中 / **状態**: documented-limitation
- **内容**: `6651109b` (2026-07-12) が `.github/workflows/ci.yml` から `push` / `pull_request`
  トリガを外し、`workflow_dispatch` 限定にした。**方針そのものは意図的**で、
  [`CI.md`](docs/development/operations/CI.md) の冒頭と
  [branch protection checklist](docs/development/operations/branch-protection-checklist.md) に
  「Temporary policy (2026-07-12): CI 自動実行は停止」として記録済みである。
  台帳に無かったのは**その副作用**のほうで、`ci.yml` が定義する **17 job が誰にも観測されない**
  状態がそれ以来続いている。

  ```
  test / doc-status / lint / format / bootstrap / default-path-smoke /
  fresh-clone-artifact / test-fresh-clone / fresh-clone-smoke / gc-metrics-artifact /
  native-proxy-artifact / native-linux-x86-smoke / editor-extension-build /
  audit-docs / ci-gate / ci-gate-v2 / shadow-oracle
  ```
- **実測 (2026-08-18)**: `codex/gate-fixes-root-lifetime` を push しても
  `gh run list --branch codex/gate-fixes-root-lifetime` は **0 件**。
  `gh run list --limit 3` の最新 3 件はいずれも **2026-07-12** で、`6651109b` が入った当日である。
- **実害の実例**: `I-15` の「3 層の腐敗」がこれである。`default-path-smoke` の assertion は
  第 2 層 (structured JSON への移行) と第 3 層 (`SmokeCli.ls` の import 欠落) を
  誰にも見られないまま積み上げた。**gate が落ちないのではなく、gate が走らない**ので
  baseline にも載らず、`workspace-expected-failures.txt` からも見えない。
- **代替手段の被覆範囲**: `6651109b` が代替として立てたのは
  `native-official-release-local.sh` による **release の手元 gate** であり、
  上記 17 job を代替するものではない。`scripts/ci/test-*.sh` の 41 本は手元で回せるが、
  「CI job として組まれた形」(runner の clean 環境 / job 間の `needs` / artifact 受け渡し) は
  手元では再現されない。
- **含意**: CI 停止中は **`ci.yml` に書かれた内容の正しさが検証されない**。
  本ブランチが `127f0d3d` で `default-path-smoke` job に足した `cargo build --bin lsharp` も、
  YAML の構文妥当性 (`yaml.safe_load`) までしか確認できていない。
- **関連**: I-15 (実害の実例)、`SMOKE-GATE-03` (TODO.md、1 run 観測の残件)、
  `OPS-05` (job の由来)。

---

## ドキュメント上の問題

<a id="i-20"></a>
### I-20: selfhost parser が受理した 6 directive の payload を黙って捨てている

- **影響度**: 中 / **状態**: open
- **内容**: selfhost parser は `directive-symbol-v3` で **28 件**の directive を受理するが、
  `parse-defn-metadata-step-v3` (`selfhost/src/Syntax/Parser.ls:1223`) が実際に読むのは
  **22 件**である。名前で分岐する 8 件 (`doc` / `example` / `params` / `returns` /
  `invariant` / `case` / `assert` / `property`) と、`source-metadata-form-kind-v3` が
  非 0 を返す 14 件がそれで、残る **6 件は fall through して
  `skip-directive-payload-v3` へ落ち、`meta` を更新せずに返る**。

  ```
  where / rationale / since / see-also / transitions / constraints
  ```

  Rust の `try_parse_metadata` はこのうち `rationale` / `since` / `see-also` / `transitions` を
  `Metadata` へ格納する。**同じソースから front end によって異なるメタデータが出る。**
- **パース error にはならない**: 受理はされ payload も読み飛ばされるので、
  構文としては通る。**落ちないので baseline にも載らない**種類の欠落である。
- **4 つ目の sync point**: `source-metadata-form-kind-v3` (`Parser.ls:1865`) は
  `source-directive-symbol-v3` と同じ 14 件を kind コード付きで再度並べている。
  I-18 が数えた 3 系統に加え、これが 4 つ目の手書き表になる。
- **検出できていない理由**: `metadata_directive_parity.rs` は allowlist だけを比較する。
  「受理したものをどう読むか」は比較対象外だと ADR に明記してある。
- **関連**: I-18 (allowlist の二重管理)、`LEGACY-MODULE-01` (selfhost 側の変更コスト)。

---

<a id="i-21"></a>
### I-21: native backend の root API が runtime spec の tier 1 契約に適合していない

- **影響度**: 高 / **状態**: open
- **内容**: [runtime spec](docs/language/runtime-spec.md) の root 管理 API tier 1 は
  全 backend 必須の契約だが、native backend はこれを満たしていない。
  wasm backend と挙動が食い違っており、**同じプログラムが backend によって異なる結果を出す**。

  | lane | 実装 | tier 1 適合 |
  |---|---|---|
  | wasm (WASI) | `crates/lsharp-wasm/src/wasi/root.rs` | 満たす |
  | native aarch64 | `selfhost/src/Backend/Native/NativeCodegen.ls` で IR opcode 74/75/76 をインライン展開 | 項目 1/2/3 は満たす (2026-08-18 に是正。当初は**項目 3 に違反**)。**項目 4 に違反** (下記) |
  | native x86-64 | 同上だが stub | **未実装** |

- **aarch64 の違反 (最も重い)**: `emit-root-pop-aarch64` は空 stack のガードを持たず、
  無条件に `sub x27, x27, #8` してから `ldr` する。root stack が空のときに呼ぶと
  stack pointer が base を下回り、**bss 領域の手前を読む**。
  spec の tier 1 項目 3 (「空の root stack に対する `root_pop` は trap せず、
  root stack を変更せずに `0` を返す」) に対する直接の違反である。
  **2026-08-18 に是正した** (`NATIVE-ROOT-01`)。`emit-root-pop-aarch64` に
  `cmp x27, x28` / `b.eq` の空判定を inline で入れ、空のときは `x27` を動かさず `0` を返す。
  判断と却下した選択肢は
  [空 stack ガード ADR](docs/adr/decisions-native-root-pop-empty-guard.md) が正本。
  **是正前の実害は実測で確認済み**: 空 pop を含む host binary は exit code `-1`
  (異常終了) を返していた。是正後は期待どおり `7` を返す。
- **項目 4 (容量が動的) の違反 — 2026-08-18 追記**: native lane の root stack は
  **固定 8 MiB の BSS ブロック**であり、拡張しない。`emit-root-push-aarch64`
  (`NativeCodegen.ls:17105-17112`) は `str x0, [x27]` / `add x27, x27, #8` を無条件に出すだけで
  **容量検査を持たない**。上限を超えた `root_push` は trap せず、隣接する `__DATA,__bss` を
  黙って壊す。tier 1 項目 4 は「容量は動的で固定上限を定めない。確保できなくなった時点で trap する」
  と定めており ([root API 契約 ADR](docs/adr/decisions-runtime-spec-root-api-contract.md))、
  これに正面から反する。wasm 側は `ROOT_STACK_SLOT_CAPACITY = 32768` から倍々に拡張し
  `memory.grow` 失敗で `Unreachable` するので、**同じプログラムが backend で異なる結果を出す**
  という本 issue の主題がここにも当てはまる。
  `NATIVE-ROOT-01` の是正時にこの行を「満たす」と書いたのは項目 3 だけを見た誤りで、
  2026-08-18 に上表を訂正した。

- **root stack の実体は codegen ではなく link 時の entry stub にある — 2026-08-18 追記**:
  `NativeCodegen.ls` は x27 (root stack pointer) / x28 (root stack base) を
  **一度も初期化しない**。`grep 'x27\|x28'` で当たるのは root_push/pop/set 自身の
  `add` / `sub` / `cmp` だけで、base を設定する `emit-aarch64-mov-x28-x27` (`:16145`) と
  `emit-aarch64-mov-x0-x27` (`:16141`) は **定義だけあって呼び出しが 0 件**である。
  実際の確保と初期化は link 時に足される entry stub が行う。

  | 経路 | 場所 | root stack |
  |---|---|---|
  | e2e host binary (aarch64) | `crates/lsharp-wasm/tests/e2e/selfhost_native_stage_chain.rs:37368-37377` / `:37867-37906` | `adrp x27, _lsharp_root_stack@PAGE` / `mov x28, x27` + `.zerofill __DATA,__bss,_lsharp_root_stack,0x800000,3` |
  | 製品 (macOS aarch64) | `scripts/ci/materialize-native-macos-aarch64-bundle.py:131-133,170` | 同上 |
  | 製品 (Linux x86-64) | `scripts/ci/materialize-native-linux-x86-bundle.py` | **無い** (`root_stack` / `.comm` / `.zerofill` が 0 hit) |

  つまり x86-64 lane を tier 1 適合にするには codegen の 3 命令だけでは足りず、
  **entry stub 側に root stack の確保を足す**必要がある。この事実は `NATIVE-ROOT-02` の
  受入条件に反映した。`_lsharp_root_stack` シンボルはリポジトリ全体で上記 2 経路にしか無い。

- **x86-64 の stub**: `emit-root-push-x86` は `xor eax, eax` を出すだけで (引数を捨てて常に 0 を返す)、tier 1 項目 1 (push した slot の index を返す) を満たさない。
  `root_pop` には emitter そのものが無く定数 0 を返す。`root_set` は store 命令を出さない。
- **シンボルは存在しない**: `lsharp_root_push` / `lsharp_root_pop` / `lsharp_root_set` という
  シンボルは実装のどこにも無い。root 操作は呼び出し側へ直接展開される。
  [native backend spec](docs/language/native-backend-spec.md) の
  「v1 で想定する代表的な公開シンボル」がこれらを挙げているのは**想定であって実態ではない**。
  spec 側は 2026-08-18 に「ABI シンボルの有無は問わない」と明記して食い違いを解いたが、
  **挙動の非適合は残る**。
- **なぜ顕在化していないか**: native lane は GC を導入しておらず、root stack を実際に
  使い切る経路がまだ無い。`I-17` が「native は実装していないので非互換は顕在化していない」と
  書いていたのは**シンボルの有無を見た誤読**で、挙動としては既に食い違っている。
- **着手の追跡**: aarch64 の項目 3 は `NATIVE-ROOT-01` で閉じた。残る x86-64 lane は
  `TODO.md` の `NATIVE-ROOT-02`、両 lane 共通の項目 4 (動的容量) は `NATIVE-ROOT-03` が持つ。契約を書いた `RUNTIME-SPEC-01` は
  native 実装を「含めない範囲」に明記していたため、非適合の記録である本項が先に立った。
- **状態を open のままにしている理由**: tier 1 は全 backend 必須の契約であり、
  x86-64 が未実装である以上「native backend が適合した」とは言えない。
  aarch64 だけを見て resolved にすると、残った非適合がどの台帳にも載らなくなる。
- **関連**: I-17 (契約側)、I-13 (native heap の回収機構と bounds check の欠如)。

---

<a id="i-22"></a>
### I-22: heavy e2e 164 件が `#[ignore]` 契約を満たしておらず、規約と実態のどちらが陳腐化しているか未決

- **影響度**: 中 / **状態**: resolved
- **解決 (2026-08-19)**: **案 A** (164 件に `#[ignore]` を付ける) を採って実装した。
  `ops03c` は GREEN、`ops03` / `ops03b` / `ops03d` も巻き添えなく ok。
  default で走る test は **1,800 -> 1,636** (合計 3,062 は不変、ignored が 1,262 -> 1,426 と
  ちょうど 164 増えた)。`workspace-expected-failures.txt` からは 4 行を外した — 裁定時に
  数えていた 3 行に加え、GREEN になった `ops03c` 自身の行も外さないと
  「expected が pass に転じた」条件が発火するためである。
  **phase11 lane の selection は 2026-08-19 に実測済み** -- 該当 4 filter の `--ignored --list` 和集合 585 件に 164 件が **164/164 含まれる** (漏れ 0)。
  ただし**実走して pass/fail を見てはいない** (全件 5 時間規模)。
  判断・実測・満たせなかった条件は
  [ignore 契約 ADR](docs/adr/decisions-test-gate-ignore-contract.md) の Evidence 節が正本。
- **内容**: `TESTGATE-01` で `test_e2e_ops03c_heavy_ci_gates_are_ignored_and_scripted` の
  構造的破損 (fragment 未追随 / prefix モードの無言無効化) を直したところ、
  検査が**本物の違反 164 件**を報告した。従来はこれらが見えていなかった。

  | ファイル | 違反数 |
  |---|---|
  | `crates/lsharp-wasm/tests/e2e/selfhost_cli_core.rs` | 158 |
  | `crates/lsharp-wasm/tests/e2e/selfhost_cli_actual_main_args.rs` | 5 |
  | `crates/lsharp-wasm/tests/e2e/selfhost_native_stage_chain.rs` | 1 |

- **いつ・なぜ増えたか (git 履歴からの実測)**: 「ルールが一度も効いていなかった」ではない。
  ルールは効いていた時期があり、**enforcement が止まった後に積み上がった**。

  | 日付 | 事象 | `selfhost_cli_core.rs` の違反数 |
  |---|---|---|
  | 2026-05-07 `e9bb354e` | prefix ルール導入 (`Move heavyweight e2e gates behind scripts`) | 0 |
  | 2026-05-15 〜 2026-07-12 | -- | 0 のまま |
  | 2026-07-12 | CI 自動実行が停止 (`I-19`) | 0 |
  | 2026-07-17 `60f4ef8f` | 増加の開始 | 0 → 2 |
  | 2026-07-17 〜 2026-07-27 | selfhost contract/diagnostics の TDD 連打が 1 commit あたり 1〜3 件ずつ追加 | 141 |
  | 2026-07-27 `a7edffe1` | `split bootstrap acceptance tests` で親が `include!` のみになり検査が panic 停止 | 141 |
  | 2026-08-18 (現在) | -- | 158 |

  つまり破壊は **2 段**である。(1) 2026-07-12 の CI 停止で誰も検査を回さなくなり、
  5 日後から違反が積み上がった。(2) 2026-07-27 の fragment 分割で、たとえ手元で回しても
  厳密名モードが先に panic して違反一覧に到達しなくなった。

- **偽陽性ではない根拠**: `crates/lsharp-wasm/tests/` に `#[cfg_attr(..., ignore)]` 形は
  **0 件**であり、行スキャン方式が `#[ignore]` を取りこぼす経路は無い。
- **決めるべきこと (本 issue の本体)**: 規約と実態のどちらが陳腐化しているか。

  - **案 A: 164 件に `#[ignore]` を付ける** — 既に `#[ignore]` を持つ 218 件の兄弟と
    整合し、`scripts/ci/compile-phase11-inputs.sh` が `--ignored` で回す設計にも合う。
    ただし **CI は 2026-07-12 から止まったままなので (`I-19`)、今日 `#[ignore]` を付けることは
    「現在 run されて pass している 158 件が、どこでも走らなくなる」ことを意味する**。
    さらに `I-11` が正本として記録した run set (run 1,799 / ignored 1,260 / 全 3,059) の
    算術がずれる。
  - **案 B: prefix ルールを絞る** — 違反の中身は 2026-07-17 以降の selfhost CLI check 系の
    細粒度 test であり (例: `selfhost_cli_core.rs:973` `test_e2e_selfhost_cli_check_file_resolves_imported_definition`)、
    heavy artifact gate という当初の対象像から外れている可能性が高い。
    採ると run set は 1,799 のまま動かず、`I-11` の測定 anchor が全て有効なまま残る。

  **裁定 (2026-08-18)**: **案 A** を採った。判断と却下理由は
  [test gate ignore 契約 ADR](docs/adr/decisions-test-gate-ignore-contract.md)。
  決め手は案 A 側の損失が実測で消えたこと — `scripts/ci/compile-phase11-inputs.sh` は
  prefix 起動なので、`#[ignore]` を付けても 164/164 が phase11 lane で走り続ける
  (当初 0/164 と測ったのは厳密名 grep による誤測)。以下の「案 A / 案 B」は裁定前の記述として残す。

  どちらを採るかは**規約側の意図の判断**であり、gate を green にするために片方へ寄せない。
  `DIAG-DEDUP-01` と同じ形 (規約 vs 蓄積した実態) なので、同様に裁定してから直す。

  **先例が出た**: `DIAG-DEDUP-01` は 2026-08-18 に `I-24` として裁定し、
  「**規約の文言のほうが、実運用の要求を取りこぼしていた**」と結論した
  ([lint dedup identity ADR](docs/adr/decisions-lint-diagnostic-dedup-identity.md))。
  ただしこれは「実態が正しい」という一般則ではない。あの件で決め手になったのは
  「規約どおりに直すと利用者から見える指摘が消える」という**具体的な損失**であって、
  実態が蓄積していたことそのものではない。本件で同じ問いを立てるなら
  「案 A を採ると 158 件がどこでも走らなくなる」という損失を同じ天秤に載せる。
- **現在の扱い**: 解決済み。`TESTGATE-01` 直後は `ops03c` が `workspace-expected-failures.txt` の
  expected FAIL として残っていた (同 slice の前後で baseline の FAIL 集合は不変) が、
  2026-08-19 の `TESTGATE-03` 実装で GREEN になり、当該行は削除した。

---

<a id="i-23"></a>
### I-23: `aarch64-selfhost-helper-trailer-size` の pin が 2026-08-03 から陳腐化したまま気付かれていない

- **影響度**: 中 / **状態**: open
- **内容**: `test_e2e_native_aarch64_bundle_initial_capacity_includes_full_helper_trailer`
  (`crates/lsharp-wasm/tests/e2e/selfhost_native_stage_chain.rs`) は
  `(aarch64-selfhost-helper-trailer-size 10)` == `2520` と
  `(aarch64-bundle-initial-capacity 1000 10)` == `3520` を pin しているが、
  現在の実測は `3492` / `4492` で、**恒常的に FAIL している**。
- **実測の取り方 (2026-08-18)**: 当該 test を単体で実行し、panic が
  `.expect(...)` (環境要因) ではなく `assert_eq!` 側であることまで確認した。

  ```
  assertion `left == right` failed: AArch64 bundle result capacity は helper trailer 全体を含むべき: [3492, 4492]
    left: [3492, 4492]
   right: [2520, 3520]
  ```

  **この確認を分けて記録するのは、環境要因の FAIL を「陳腐化した pin」と読み違えると
  本 issue の原因説明そのものが崩れるからである。**
- **いつずれたか (git 履歴からの実測)**:

  | 日付 | commit | 事象 |
  |---|---|---|
  | 2026-05-04 | `cf41069e` | 期待値 `vec![2520, 3520]` を pin |
  | 2026-08-03 | `1ee26eef` | `aarch64-selfhost-read-stdin-helper-offset` の定数を `2324` → `3296` へ拡大 |

  helper trailer size は `read-stdin helper offset + 156`、`base(0,10)` は
  `import-stub-count(10) * 4 = 40` なので、現在の値は `40 + 3296 + 156 = 3492` である。
  期待値 `2520` は `read-stdin` helper が増える前の世界を pin しており、
  **どの import-stub-count を与えても現在のコードでは再現できない**。
- **なぜ気付かれなかったか**: 本 test は `#[ignore]` であり、
  `scripts/ci/compile-phase11-inputs.sh` の `--ignored` lane からのみ走る。
  CI 自動実行は 2026-07-12 から停止している (`I-19`) ため、
  定数を拡大した 2026-08-03 以降、誰もこの lane を回していない。
  `#[ignore]` test は `docs/development/validation/workspace-expected-failures.txt` の
  baseline (非 ignored が対象) にも載らないので、**どの台帳にも残らなかった**。
- **どちらが陳腐化しているか**: pin 側である。`1ee26eef` は read-stdin runtime opcode の
  追加という機能変更で、helper trailer が伸びること自体は正しい。
  ただし**期待値を実装に合わせて書き換えるだけで済ませない** — 同じ形の pin が
  他にも眠っている可能性が高く、そちらを先に洗い出す。
- **lane 完走の実測 (2026-08-19)**: 受入条件 (a) の
  「`--ignored` lane を一度完走させて FAIL 集合を確定させる」を満たした。
  **614 test / 497 passed / 117 failed / 18,756.35s**。
  分類は test 名 prefix ではなく関数本体を読んで行い、
  **(a) Lima VM 依存 60 件 / (b) `LSHARP_NATIVE_*` env 依存 4 件 / (c) それ以外 53 件 /
  帰属不能 0 件**。(c) 53 件のうち 37 件は `wasm trap: out of bounds memory access` で
  representative 破損調査の harness 族に属し、22 件は
  `#[ignore = "diagnostic: ..."]` と理由文字列に失敗が既知である旨が書いてある。
  数字と原因クラスタの全量は
  [root_pop 空 stack ガード ADR](docs/adr/decisions-native-root-pop-empty-guard.md) の
  「完走後の全件結果」節が正本。
  **1 回目の run はハーネスに 328/614 で停止されたため採用していない** —
  走っていない 286 件を pass と区別できないためである。
- **baseline との前後比較 (2026-08-19)**: 受入条件 (b) の残りだった
  「`origin/main` での baseline」を実測した。`origin/main` `8475b00a` で
  **612 test / 495 passed / 117 failed / 19,005.96s**。
  **積集合 612 件の上で FAIL 集合は sweep2 と完全に一致し、新規 FAIL 0 / 解消 0。**
  sweep2 側にだけある 2 件は `NATIVE-ROOT-01` が追加した root_pop ガードの test で、
  どちらも pass する。したがって **117 件はすべて `origin/main` 時点で既に FAIL しており**、
  `NATIVE-ROOT-01` 由来のものは無い。分類 (a) 60 / (b) 4 / (c) 53 / 帰属不能 0 も
  baseline 側から独立に再現した。
- **比較の射程を訂正した (2026-08-19)**: 上の 1 行目を当初「`main` (32 commit ahead)」と
  書いていたが誤りである。sweep2 を取ったのは merge 前の worktree HEAD `8a20cfe2` で、
  ADR 側の取得条件には `8a20cfe2` と書いてあったので文書内で矛盾していた。
  比較が証明したのは **`8475b00a` ≡ `8a20cfe2`** — つまり `NATIVE-ROOT-01` 由来 0 まで
  であって、merge 済みの他 3 branch は覆っていない。`8a20cfe2..main` のうちこの lane に
  届くのは 2 つ:
  - `939e4ec9` (`TESTGATE-03`) が `selfhost_native_stage_chain.rs` に `#[ignore]` を 1 個追加した。
    **`main` での lane の分母は 615** で、増えた
    `test_e2e_selfhost_pipeline_smoke_root_set_keeps_shadowed_slot_during_allocating_value`
    は本測定に含まれていない (未測定であって pass ではない)。
  - `5e992d52` / `1855fa0b` が `LspServerNav.ls` から 22 行削除した。同ファイルは
    selfhost bundle の構成モジュール (`crates/lsharp-wasm/tests/e2e/support.rs:37-39`) で、
    117 件のうち 79 件が bundle を組む系なので、サイズ・オフセットを pin する assertion が
    ずれる可能性は排除できない。

  分母は revision ごとに実測した (`8475b00a` 612 / `8a20cfe2` 614 / `main` 615)。
  **`main` 実体での lane 再実行は未実施**で、これは満たせなかった条件である。
  再実行は `TODO.md` の `IGNLANE-01` が持つ。
- **台帳化 (2026-08-19)**: 受入条件 (c) を
  [`docs/development/validation/ignored-lane-expected-failures.txt`](docs/development/validation/ignored-lane-expected-failures.txt)
  で満たした。`workspace-expected-failures.txt` と同じ `<binary-id> <test-name>` の粒度で
  117 件すべてを分類つきで並べ、`#[ignore]` の理由文字列も併記してある。
  **`scripts/ci/check-workspace-baseline.sh` の入力にはしていない** — 非 ignored lane の
  baseline へ混ぜると「実測に現れない expected」として必ず非 0 になるためで、
  自動検証は付いていない。ここは満たせなかった点として明示しておく。
  台帳の 117 行は `8a20cfe2` 時点の集合であり、`main` では未検証である
  (上記「比較の射程を訂正した」を参照)。
- **pin を更新した (2026-08-19)**: `main` 実体で RED を確認し
  (`left: [3492, 4492]` / `right: [2520, 3520]`)、
  `crates/lsharp-wasm/tests/e2e/selfhost_native_stage_chain.rs:26444` の期待値を
  `vec![3492, 4492]` へ更新して GREEN (`test result: ok. 1 passed`, 43.13s)。
  算術の由来をコメントとして同じ位置に残したので、次に定数が動いたときは
  `git log -S` ではなく test 本文から辿れる。
  `ignored-lane-expected-failures.txt` の該当行も削除した (117 -> 116 件)。
  実行は `CARGO_TARGET_DIR=/Users/biwakonbu/github/tmp/stale-pin-02/target` で行い、
  root checkout に `target/` を作っていない。
  なお **pin 側の陳腐化と確定していたのは本 issue の 1 件だけ**である。
  同じ (c) 53 件に含まれる `..._x86_function_size_matches_generated_length_diagnostic` と
  `..._x86_int_to_string_import_sets_rdi` は、panic message を読むと数値 pin の陳腐化ではなく
  **実装側の欠陥を検出している可能性が高い**ため、`STALE-PIN-03` で裁定してから扱う。
  「(c) に入った = pin が古い」と一括りにしない。
- **発見の経緯**: `NATIVE-ROOT-01` (aarch64 root_pop の空 stack ガード) の回帰確認で
  `selfhost_native_stage_chain` の `--ignored` lane を通したときに検出した。
  `NATIVE-ROOT-01` の変更とは無関係であることは上記の算術で確定している
  (`2520 < 3296 + 156` なので、opcode 75 の命令長に関わらず再現不能)。
  加えて実測値 `3492` は算術値と一致しており、算術の前提そのものも裏が取れている。
- **関連**: I-19 (CI 停止)、I-22 (同じく CI 停止期間に積み上がった `#[ignore]` 契約違反)、
  I-11 (baseline が非 ignored 限定であること)。

---

<a id="i-24"></a>
### I-24: 診断の「重複」定義が spec 文言 / test / 実装の 3 者で食い違っている

- **影響度**: 中 / **状態**: resolved
- **内容**: 診断の重複除去について、3 つの正本が互いに違うことを言っている。
  2 者間の drift ではなく **3 者間の衝突**である。

  | # | 正本 | 主張 | 状態 |
  |---|---|---|---|
  | 1 | spec AC-209 (`docs/development/planning/toolchain-parity-spec.md`) | 同一 span なら severity の高い方のみ残す。rule の例外は書かれていない | 文言のみ |
  | 2 | `test_e2e_selfhost_lsp_render_sorted_deduped_diagnostics_json` / `..._snapshot` | AC-209 の文言どおり。同一 start span の lint 2 件を 1 件へ潰すことを要求 | **FAIL** (baseline に 2 件) |
  | 3 | `LspServerNav.ls:1225-1245` の `dedup-diag-same-lint-identity` と、それを pin する 2 test | lint 同士は rule と start/end が全一致した場合だけ重複 | pass |

- **決め手**: 3 の側の pin は 2026-08-11 `c00368ad` が入れた LSP wire レベルの e2e
  `test_e2e_selfhost_cli_lsp_stdio_didopen_preserves_distinct_same_start_diagnostics`
  (TEST-CLI-02-AN32j) である。実ソース `(defn main [] (let [unused (do)] 0))` に対し
  `L0001` と `L0002` が**どちらも** publish されることを要求しており、両者は
  range `0:0..0:0` / severity 2 で start も end も一致する。
  **AC-209 の文言をそのまま実装すると、この 2 件のうち片方が利用者から黙って消える。**
- **どちらが陳腐化しているか**: spec の文言 (1) である。AC-209 は「重複」を span だけで
  定義しており、rule identity を勘定していなかった。文言を是正し、test (2) の期待値を
  それに合わせる。判断と却下理由は
  [lint dedup identity ADR](docs/adr/decisions-lint-diagnostic-dedup-identity.md) に記録した。
  **「実装が正しいから test を直す」ではない。** test が写している文言のほうが実運用の要求を
  取りこぼしている、という判定である。
- **span 精密化では解けない**: 現状 `L0001` / `L0002` がともに `0:0..0:0` へ落ちるのは
  lint span の投影が未実装だからだが、span が精密になっても「同一識別子に別 rule が 2 件」は
  正当に起こりうる。したがって rule identity による判別は**恒久的に**必要で、
  本判断は暫定免除ではない。span 精密化は独立の品質課題として `LINT-SPAN-01` に登録した。
- **派生して見つかった重複実装**: `merge-duplicate-diagnostics` (`LspServerNav.ls:1169`) は
  同一 start span を rule を問わず潰す、`dedup-diagnostics` と逆の意味論を持つ。
  呼び出し元は `LspServer.ls:144` の検証用 `main` と parity test 3 本だけで、
  **実運用の publish 経路 (`Cli.ls:1440` / `:1687` / `:1702`) には入っていない**。
  同じ概念の実装が 2 つあり片方だけが正しい状態なので、単一正本化の対象として
  `TODO.md` の `LSP-DEDUP-MERGE-01` に登録した。2026-08-18 に呼び出し元を実測で全数確認し、
  さらに `merge-duplicate-diagnostics` が `len == 2` の入力しか扱わない (3 件以上は素通し) ことが
  判明したため、削除する方向で決着させた。判断と却下理由は
  [dedup 単一正本化 ADR](docs/adr/decisions-lsp-dedup-single-source.md)。
  2026-08-19 に実装まで完了し、受入条件 3 件 (非 docs 参照 0 / parity test 3 本が期待値据え置きで pass /
  `Cli.ls` 経路の pin 7 件が pass) をすべて実測で満たした。
- **`LINT-SPAN-01` の設計判断は 2026-08-19 に確定した**:
  [lint span の AST 表現 ADR](docs/adr/decisions-lint-span-ast-representation.md)。
  併せて「selfhost の AST は位置情報を持っていない」という当初の見立てを訂正した。
  var / string / float / apply / if / module-decl / import-decl / type-alias は既に
  byte offset の span を持ち、欠けているのは `let` (tag 7) と `do` (tag 9) だけである。
  ただし AST ノードの**長さが意味の判別子として使われている** (`TypeInfer.ls:60` / `:114-115`) ため
  一律の末尾 slot は採れず、加えて selfhost に offset → line/col 変換が存在しないので、
  本項目は span を載せるだけでは閉じない。
- **関連**: I-11 (baseline)、`ISSUES.md` の I-22 (同じ「規約 vs 実態」の形。
  `:1201` が本件の裁定に倣うと書いている)。

---

<a id="i-25"></a>
### I-25: `NativeCodegen.ls` に呼び出し元 0 の defn が 64 個ある

- **影響度**: 低-中 / **状態**: open
- **内容**: `selfhost/src/Backend/Native/NativeCodegen.ls` (20,937 行 / 1,382 defn) のうち
  **64 defn / 449 行が selfhost 全体から一度も呼ばれない**。
  行数は「その `defn` 行から次の `defn` 行の直前まで」で数えている (間の空行・コメントを含む)。
  449 / 282 / 167 はいずれもこの同一方式による実測で、449 = 282 + 167 が成り立つ。
  最初は `string-concat-helper-chunk1`〜`chunk4` の 4 つだけを見つけて「4 つ」と書いたが、
  `selfhost/src` 全体を走査すると 64 個あった。内訳は 3 種類に分かれる。

  **危険なのは 1 群だけである (2026-08-19 に全 40 件の本体を確認)。**
  `crates` test からも参照が無い 40 defn / 282 行の内訳:

  | 種類 | 件数 / 行数 | 例 | 危険度 |
  |---|---|---|---|
  | 使用中の実装と**乖離した別実装** | 4 / 113 | `emit-aarch64-selfhost-string-concat-helper-chunk1`〜`4` | **高** (下記) |
  | 定数・単一命令 wrapper の網羅定義 | 24 / 73 | `reg-rcx`、`emit-aarch64-mov-x0-x1` | 低 |
  | 引数を足した新名へ移行した旧名 | 4 / 21 | `generate-native-control-instr-bundle-loop-x86` → `-with-context` (5 呼び出し) | 低 |
  | 生きた関数へ委譲するだけの adapter | 3 / 9 | `collect-function-starts-aarch64` は `(collect-callable-function-starts-aarch64 functions 0)` の 1 行 | 低 |
  | 未使用の変種 (生きた primitive を組む本体を持つ) | 5 / 66 | `emit-mov-forty-fourth-stack-from-rcx` (stack slot 直書き)、`emit-consume-four-produce-one-bundle-aarch64` | 低 |

  残る 24 defn / 167 行は test に pin されている。内訳は
  「test が名前ごと参照する」16 件と「assertion だけ」8 件で、性質は次のとおり。

  **16 件の大半は既定引数 facade である (2026-08-19 に全件実読)。**
  短い arity の名前が `import-count 0` / `import-stub-offset 0` / `function-start` を
  埋めて、生きた full-arity 実装へ委譲する。production は常に import count を持つので
  短い方を使わず、test だけが使う。

  | 形 | 件数 | 例 |
  |---|---|---|
  | 既定引数 facade (`-with-import-count` / `-and-base` 版へ委譲) | 6 | `codegen-ir-instr-bundle-x86`、`generate-native-function-aarch64-bundle` |
  | module 末尾の公開 API facade (`emit-native-*`) | 5 | `emit-native-function-meta-bundle` (`generate-` 版へ 1 行委譲、test 呼び出し 138) |
  | 生きた primitive を並べた arity dispatcher | 3 | `emit-call-bundle-x86-one-to-nine`、`spill-native-function-params-x86-twenty-to-sixty-one`、`emit-sixty-one-arg-call-x86` |
  | context を組み立てて委譲 | 1 | `native-function-body-size-x86-loop` (`make-x86-body-size-context` → `x86-body-size-loop-ctx`) |
  | 独立した判断を持つ | 1 | `x86-selfhost-helper-trailer-size` (`I-26`) |

  **当初この表は 9 / 5 / 2 と書いていたが (2026-08-19 に訂正)、合計 16 は合っていても
  `x86-selfhost-helper-trailer-size` の居場所が無かった。** 同じ節の 2 行下で
  「独立した判断を持つのは trailer-size だけ」と書いておきながら、
  それを既定引数 facade に数えていたことになる。全件を分類し直した結果が上表である。

  **乖離した別実装は 1 件も無い。** 独立した判断を持つのは
  `x86-selfhost-helper-trailer-size` だけで、それは `I-26` が追っている。
  つまり `NATIVE-DEAD-01` が裁定すべき対象は 64 件中 chunk 群 4 件のみで確定する。
  **chunk 群を除く 36 件は、既存の生きたコードへ委譲するか単一命令を返すかのどちらかで、
  失われる意味論を持たない。** 「移行残り」に見えたものの実体はこれである。

  再現は `python3 scripts/native_codegen_dead_defn.py` (cargo 非依存)。
- **「呼び出し元 0」は `.ls` に限った話である (2026-08-19 に追加走査)**:
  64 個のうち **25 個は `crates/lsharp-wasm/tests` から参照されている**。
  production 未使用でも test からは生きているので、削除可否は 3 つに分かれる。

  | 区分 | 件数 | 削除したら | 例 |
  |---|---|---|---|
  | test からも参照なし | 40 | 壊れない | `reg-rcx`、chunk1〜4、`emit-aarch64-mov-x0-x1` |
  | test が名前ごと参照する (L# 呼び出し / ソース走査表) | 16 | **壊れる** | `emit-native-function-meta-bundle` (呼び出し 138)、`x86-selfhost-helper-trailer-size` (呼び出し 7 + 走査表 1) |
  | ソース文字列 assertion だけ | 8 | 7 件は壊れない (否定 assertion `!body.contains(...)` のため)。`compile-and-run-native` の 1 件だけ肯定 assertion で壊れる | `x86-function-emit-layout-*` 4 件 |

  つまり **削除が test を壊さないのは 47 個、壊すのは 17 個**である。
  「未参照だから消してよい」と `.ls` の走査結果だけで判断すると 17 件で転ぶ。

  **「assertion だけ」8 件のうち 7 件は移行残りではない (2026-08-19 に訂正)。**
  否定 assertion の中身は「この defn を hot path で呼ぶな」であり、
  呼ばれないこと自体が pin された仕様である (`x86-function-emit-layout-*` 4 件、
  `native-call-rel-x86`、`emit-map-new-bundle-x86`、`emit-four-arg-call-x86-core`)。
  理由は x86 native 実行時に user call を跨いで local / 引数が壊れることで、
  **その欠陥は台帳のどこにも書かれていない**。`I-27` として起票した。
  消しても assertion は通るが、hot path がこの書き方である理由の記録が失われるので、
  `NATIVE-DEAD-01` / `NATIVE-INLINE-01` のどちらでも削除対象に含めない。

  **残る 1 件 `compile-and-run-native` は無効化された差分 test の placeholder である。**
  本体は `(compile-to-native ir target)` への一行委譲。pin しているのは
  `selfhost_gc_runtime_bootstrap.rs:733-739` の
  `contains("(defn compile-and-run-native") || contains("(defn native-run") ||
  contains("(defn emit-and-execute")` という 3 択の名前存在チェックで、
  「Wasm と native の実行結果が一致すること」という**本来の assertion は同 test の
  `:741-747` にコメントアウトされたまま**である。つまり生きているのは
  「native 実行関数と呼べる名前が 1 つある」という契約表面だけで、その裏の比較は動いていない。
  test コメントが参照する `NATIVE-06` は `ISSUES.md` / `TODO.md` / `docs/adr/` には
  無いが、`docs/development/planning/phase11-implementation-plan.md:526`
  「NATIVE-06 Wasm/native differential」として実在する
  (当初「どの正本にも存在しない」と書いたが、`grep` を上記 3 つにしか当てていなかった。
  2026-08-19 に `docs/` 全体 + `AGENTS.md` へ広げて訂正)。
  ただしその Current state は「differential harness、`tests/differential-allowlist.yaml`、
  5 観測点 proxy test は追加済み」と書くだけで、**本 test の実行比較が
  コメントアウトされている事実には触れていない**。ID は登録されているが、
  無効化されている個所は登録されていない。
  `DOC-07` と同じ形の抜けだが、欠陥ではなく未実装なので新規 ID は切らずここに記録する。

  **これで 64 件すべての性質が実測で確定した (2026-08-19)。**

  参照の種類は 3 つある。**呼び出し**と **assertion** に加え、`("<name>", <上限>)` の形で
  Rust 側の走査表に名前が載っているものがある (ネスト深さ上限表など)。これは L# の呼び出しでは
  ないが、defn が消えれば走査が空振りするので削除は同じく壊れる側に入る。
  16 件中この形を含むのは `native-function-body-size-x86-loop` と
  `x86-selfhost-helper-trailer-size` の 2 件。
- **なぜ問題か**: chunk3 は heap frontier を bump するコード (`add x22, x22, x2`) を含む。
  つまり「確保系 helper を数える」「bounds check を入れる」といった作業の対象に**見えてしまう**。
  実際 `NATIVE-HEAP-01` のスコープ確定で 1 件これを拾った。生きているコードと死んでいるコードが
  同じ命名規則で並んでいる限り、同じ取り違えが再発する。
  この棚卸し自体も同じ罠を 3 回踏んでいる。
  (1) 「3 helper」→ 実数 10、(2) 「chunk 4 つ」→ 実数 64、
  (3) test 参照を**部分一致**で数えて 25 件中 3 件を誤分類した
  (`generate-native-control-instr-bundle-loop-x86` の「92 呼び出し」は全て
  `-with-context` / `-with-import-count-and-base` という別関数だった)。
  **数を書くときは全走査してから書き、識別子は語境界で照合する。**
- **突き合わせ結果 (2026-08-19)**: **単なる残骸ではなく、乖離した別実装だった。**
  chunk1-4 を連結すると 308 bytes / 77 word で本体と**長さは完全に一致**するが、
  `0x34` 以降の 63 word が異なる。しかも `git log -S` で追うと
  **chunk 群 (`e9f761cb`, 2026-04-29) の方が本体 (`901c10d8`, 2026-04-22) より後**である。
  「新しい実装を書いたが呼び出し側を差し替え忘れた」形に見える。

  逆アセンブルすると構造差は 2 点。
  (a) 本体は 2 引数それぞれに `tbz x23/x24, #63` (`0xb6000137` / `0xb6000138`) を持つが
  chunk には無い。(b) chunk は `0x74` / `0xbc` に「長さ 0 / ポインタ 0」を書き込む
  default 経路を持ち、本体はそこを 1 命令で通過する。
- **判断していないこと**: **どちらが正しいかは機械語の読みだけでは決まらない。**
  両版が違う結果を出すのは bit 63 が立っていないポインタを渡したときで、
  その入力が実際に発生しうるかは呼び出し側の契約に依る。判定には実行が要る。
  それまで削除しない。作業項目は `TODO.md` の `NATIVE-DEAD-01`。
- **関連**: I-13 (同じ helper 群の bounds check 欠落)、
  I-27 (「assertion だけ」7 件の実体。呼ばれないことが仕様である理由)、
  [native heap 回収機構 ADR](docs/adr/decisions-native-heap-reclamation.md) の
  「確保系 helper の全列挙」節。

---

<a id="i-26"></a>
### I-26: x86 lane は helper trailer の補正を持たない (aarch64 だけが持つ)

- **影響度**: 中 / **状態**: open
- **内容**: `x86-selfhost-helper-trailer-size` (`NativeCodegen.ls:10645`) は
  **selfhost 内の呼び出し元が 0**。定義だけがあり、production の bundle 生成経路から一度も呼ばれない。
  aarch64 版 (`:16012`) は 2 箇所で使われている。用途は 2 つある。

  | 用途 | aarch64 | x86-64 |
  |---|---|---|
  | bundle の初期 capacity | `:16016` で `import-stub-offset + trailer-size` | `:10561` で `import-stub-offset + stub + **2048 の直書き**` |
  | 末尾関数の entrypoint offset 補正 | `:20828-20831`。`(vector-length bundle) - trailer-length - entrypoint-length` で再計算 | **無い**。`function-starts` の値をそのまま使う (`:20791-20795`) |

  後者が本質である。両 lane とも通常は `function-starts` の静的 offset を使うが、
  **aarch64 だけが「entrypoint が最後の callable 関数のとき」に実測 bundle 長から
  trailer を差し引いて offset を引き直す**。x86 にはこの分岐自体が無い。
- **経緯 (2026-08-19 に確認)**: aarch64 の補正は 2026-05-04 の `bf35168d`
  "Fix AArch64 entrypoint payload offset" で入った。当時は trailer 長を
  `(+ (aarch64-selfhost-map-new-fixed-helper-offset 0 import-count) 92)` と式で直書きしており、
  同日の `cf41069e` で `aarch64-selfhost-helper-trailer-size` へ切り出されている
  (直書き時代の `aarch64-selfhost-map-new-fixed-helper-offset` は現在 呼び出し元 0。`I-25` の 64 件に入る)。
  **x86 版が入ったのは 4 日後の 2026-05-08 `f56fcabd` "Close native stage23 helper gaps"** で、
  同じ commit が `selfhost_native_stage23_gap.rs` を +210 行している。
  つまり **test と一緒に追加され、production への接続だけが行われないまま残った**。
- **決定的な非対称**: aarch64 の補正は `collect-callable-actual-layout-aarch64` (`:18562`) が
  返す**実測 layout** を前提にしている。x86 にはこれに相当するものが無い。
  紛らわしいので明示すると、**x86 にも `layout` という名前のものはある** —
  `make-x86-function-emit-layout` (`:13993`) は 5 箇所から呼ばれる生きた構造体だが、
  中身は `import-count` / `import-stub-offset` / `function-start-base` / `emit-start-base` の
  **静的な emit パラメータ束**であって、実際に emit したバイト長を測ったものではない。
  実測に当たる `measure-native-function-aarch64-bundle-with-import-count` (`:18325`) と
  `collect-callable-function-lengths-aarch64` (`:18416`) には x86 版が存在せず、
  x86 は静的な `collect-callable-function-starts-x86` (`:9264`) しか持たない
  (`defn` 名に `measure` / `actual` / `layout` を含むものを `selfhost/src` 全体で走査して確認)。
  よって x86 へ補正を移植するには、先に「実際に emit した長さを測る」機構から要る。
- **未確認 (判定していない)**: この非対称が
  (a) x86 の静的 layout が正確なので補正が要らない、なのか
  (b) x86 にも同じズレがあるが補正が入っていない、なのか。
  aarch64 で補正が必要だった理由 (measure と emit のズレ) が x86 にも当てはまるかは、
  末尾関数を entrypoint にした bundle を実際に生成して offset を突き合わせないと決まらない。
- **副次的に見つかったこと**: x86 の初期 capacity の直書き `2048` は、
  helper size を実際に合計すると **2,486 bytes** (27 helper) で **438 bytes 足りない**。
  `vector-push` は容量超過時に倍化するので即座の破綻ではないが、
  「2048 は当て推量で、`x86-selfhost-helper-trailer-size` はそれを置き換えるために
  書かれたまま接続されていない」と読める。
- **test の状況 (誤読しやすい)**: `selfhost_native_stage23_gap/part_000.rs:571` の
  `("x86-selfhost-helper-trailer-size", 24)` は **戻り値ではなく `defn` の最大ネスト深さ**の
  上限である。値そのものを pin する test は無い。
  `selfhost_native_stage_chain.rs` の 6 箇所は test 内に埋め込んだ L# スニペットが
  `code-len (+ user-total (x86-selfhost-helper-trailer-size 10))` として呼んでおり、
  **test だけがこの関数を「本来の用途」で使っている**。
  従って**この関数は単純には削除できない** — `.ls` の呼び出し元は 0 だが test 側の
  参照が 8 箇所ある (呼び出し 7 + 走査表 1。`I-25` の「test が名前ごと参照する」16 件の 1 つ)。
  接続するか、test ごと畳むかのどちらかになる。
- **関連**: I-23 (aarch64 側 trailer-size pin の陳腐化)、I-21 (x86-64 の root API 未適合。
  x86 lane が aarch64 に追随できていないという同じ筋)、I-25 (同じ走査で見つかった)、
  I-27 (同じ x86 lane の、台帳に無い制約)。

---

<a id="i-27"></a>
### I-27: x86 native の hot path で user call を挟むと値が壊れる。回避策だけが test に pin されている

- **影響度**: 中 / **状態**: open / **発見**: 2026-08-19 (`I-25` の未参照 defn 棚卸しから)
- **内容**: `NativeCodegen.ls` の x86 codegen には「hot path で他の `defn` を呼んではならない」
  という制約が複数箇所にある。理由は **native 実行時に呼び出しを跨いで local / 引数 / ref が
  壊れるから**で、回避策は「wrapper を呼ばずに式を inline 展開する」。
  この制約は **test の assertion message にしか書かれていない**。
- **証拠**: `crates/lsharp-wasm/tests/e2e/selfhost_native_stage_chain.rs` の否定 assertion 群。

  | 位置 | 呼んではならない対象 | assertion が挙げる理由 |
  |---|---|---|
  | `:8552` | `emit-map-new-bundle-x86` | wrapper に `rel` を渡すと native 実行時に引数が壊れる |
  | `:8548` | `(let [rel ...])` そのもの | native 実行時に `rel` local が壊れる |
  | `:12376` | `native-call-rel-x86` | `target-offset` local が `current-offset` に化けうる |
  | `:12755` | `emit-four-arg-call-x86-core` (rel 版) | rel32 を integer で渡すと root されない。`(ref-new rel)` を `root_push` して `-with-rel-ref` 版へ渡すこと |
  | `:14386` | `x86-function-emit-layout-*` 4 accessor | accessor を user call すると stage1 native の rel32 が壊れる |

  同種の文言 (「破壊」「化けうる」「失われないよう」) を含む行は e2e 全体で **24 行**
  (当初「20 箇所、すべて x86」と書いたが、それは `grep -oE 'x86|aarch64'` による
  **token 出現数**であり行数ではなかった。2026-08-19 に行単位で数え直した)。
  内訳は x86 を明示 **17 行** / aarch64 を明示 **2 行** / lane を書かない **5 行**。
  aarch64 の 2 行は `write-file` helper 内で base register を書込件数で潰すなという話で、
  **user call を跨ぐ破壊とは別種**。lane 無しの 5 行は `test_wasm_compiler_*` 系で
  「native 実行時」とだけ書く。**user call を跨ぐ破壊として lane を明示しているものは
  x86 のみ**である。
- **なぜ問題か**: 3 つある。

  1. **欠陥そのものが台帳に無い。** 記録されているのは回避策だけで、
     「なぜ user call で壊れるのか」「今も壊れるのか」はどこにも書かれていない。
  2. **回避前の形が defn として残り、呼び出し元 0 になっている。**
     `I-25` の「ソース文字列 assertion だけ」8 件のうち **7 件がこれ**である
     (4 accessor + `native-call-rel-x86` + `emit-map-new-bundle-x86` +
     `emit-four-arg-call-x86-core`)。名前だけ見ると「移行残り」だが、実体は
     **意図的に呼ばれない形**で、test が「呼ぶな」を pin している。
     消しても否定 assertion は通るが、**なぜ hot path がこの書き方なのかの記録が失われる**。
  3. **同じ罠を将来また踏む。** 制約が台帳に無いので、hot path をリファクタして
     wrapper に括り出す変更が「読みやすくなった」として通りうる。止めるのは test だけで、
     test message を読むまで理由が分からない。
- **由来**: 5 件のうち **4 件**が 2026-05-17 の `361d0d99`
  「wip: advance linux x86 selfhost native path」で一括導入された。
  **commit body は空**、変更は 10 ファイル 5,391 insertions。
  判断の根拠がコミットメッセージにすら残っていない典型例で、`DOC-07` が指す
  「ドキュメント更新が実装の後追いになる」の実害がここに出ている。

  **残る 1 件 (`:12755` の four-arg) だけは別 commit で、しかも根拠が残っている**
  (当初「5 件すべて `361d0d99`」と書いたが、`git log -S` を 3 件にしか当てていなかった。
  2026-08-19 に残り 2 件へも当てて訂正した)。2026-05-23 の `b9d5d4e5`
  「Restore x86 four-arg call rel rooting」は commit body こそ空だが、
  同 commit の `TODO.md` 進捗ログに **`map-new depth=1` の bad rel32 target は
  直前の stage2 entry (`opcode=40 target-param-count=4`) が `call-rel-bytes` を失って
  cursor を崩す下流症状だった**と書き残している。この記述は後の `TODO.md` 整理で
  失われており、現在の `TODO.md` には残っていない。
- **同じ class の後続事例 (2026-08-19 に git 履歴から追加)**: 「native 実行時に local /
  引数 / ref が別の値に化ける」現象は 2026-05〜08 に繰り返し現れ、**毎回 L# ソース側で
  回避されている**。回避の形は 3 通りあるが、症状は同じである。

  | 事例 | 日付 | 観測 | 回避 |
  |---|---|---|---|
  | 本エントリの 5 件 | 05-17 / 05-23 | hot path の user call を跨いで値が壊れる | wrapper を呼ばず inline 展開 / `ref-new` + `root_push` |
  | `f5fe89bb` register state ref shadowing | 06-07 | 同一 function 内の branch 間で同名 `state-ref` が shadow し、`ref-get` が古い branch の local 6 を読む | branch ごとに別名の binding へ分離 |
  | `7f9fd01c` defn body branch shadowing | 06-15 | **stage2-debug の function 1313 disassembly で `[rbp-0x78]` に `local.set` した直後、`body` の `local.get` が `[rbp-0x70]` を読んでいる** | branch 固有の local 名へ変更 |
  | `50a2ad3c` one-argument builtin inference | 08-03 | `infer-apply-legacy-raw` の 1 引数分岐で **未使用の外側束縛が native local slot を衝突させる** (`TODO.md` の `V2-16b` に記録) | 当該分岐の束縛を整理 (`TypeInferApply.ls` のみ 1 ファイル修正) |

  `7f9fd01c` の disassembly は「root / call の問題ではなく **selfhost lowering の local slot
  解決の取り違え**」と明記しており、これが現時点で最も直接的な実測である。
  **いずれの事例も lowering 側は直っておらず、呼び出し側の L# を書き換えて避けているだけ**である。
- **未確定 (cargo が要る)**: 根本原因の特定。候補は
  (a) x86 の register/stack window 割り当てが呼び出しを跨いで caller の slot を潰す、
  (b) ref allocation が呼び出し先で走り caller の未 root な ref を無効化する
  (`:12364` の assertion は root 順序を pin しているので、こちらの筋もある)、
  (c) **selfhost lowering が同一 function 内で local slot を取り違える** (分岐間の同名束縛の
  shadow、分岐内の未使用な外側束縛による衝突)。
  `b9d5d4e5` の修正が「値を `ref-new` して `root_push` する」だったことは (b) を、
  上表の 3 事例は (c) を支持する。**(c) だけが disassembly レベルの実測を持つ。**
  ただし本エントリの 5 件が (c) と同一原因かは未確認 (5 件は上表の他 3 件より 1〜3 ヶ月古い)。
  **判定には native 実行が要るため、本エントリでは決めない。**

  **2026-08-19 追記 (`DOC-09-01` の救出による)**: `0bd8bd47` が削除していた
  2026-05〜07 の調査記録から、当時到達していた所在が復活した。

  > `call-rel` は helper 内の `push rax` 後ではなく、`append-zero-arg-call-bundle-x86` への
  > **関数呼び出し境界で既に崩れている**。

  同じ調査で **27 案が却下**されており、その落ち方には規則性がある。
  新規 helper 追加 / rooted-ref 化 / control-loop 本体への分岐追加はいずれも
  selfhost 自身の `parse` / `check` を壊すか artifact gate で Wasm OOB を起こし、
  rel32 の算出位置を動かす案は static gate を通っても VM metadata の bytes / target が
  不変だった。通ったのは既存分岐への合流だけである。
  詳細は
  [`docs/adr/decisions-native-x86-value-liveness-rejected-approaches.md`](docs/adr/decisions-native-x86-value-liveness-rejected-approaches.md)。
  **これは候補 (b) と (c) の双方を支持し、(a) を積極的に支持する記述は無い。**
  この「無い」は救出対象 93 行すべてと、走査を削除行全域 (1,364 行) へ広げた結果に対する
  判定である (`DOC-09` の解決節を参照)。部分読みで書いた判断ではない。
- **関連**: I-26 (同じ x86 lane の未接続)、I-25 (この 7 件を含む棚卸し)、
  I-07 (rooting guard の未完)、I-21 (x86-64 の root API 未適合)、DOC-07 (後追い更新)、
  DOC-09 (原因記録の消失)、
  `docs/adr/decisions-native-x86-value-liveness-rejected-approaches.md` (却下 27 案)。

---

<a id="doc-01"></a>
### DOC-01: ユーザーガイドの主要範囲不足

- **影響度**: 高 / **状態**: resolved
- **内容**: `docs/guides/` に利用者向けの主要 guide を追加し、metadata 駆動開発、
  IDE / LSP セットアップ、デプロイメントターゲット選択、stdlib API の探し方を
  公開サイトの `start` section へ登録した。エラーコードリファレンスは `LS####`
  体系導入に依存するため、引き続き DOC-06 の範囲として扱う。
- **解消根拠**:
  - `docs/guides/metadata-driven-development.md`
  - `docs/guides/ide-setup.md`
  - `docs/guides/deployment-targets.md`
  - `docs/guides/stdlib-guide.md`
  - `docs/site.toml` -- 新規 guide を `guides/*.html` として公開対象へ追加
  - `docs/guides/README.md` -- guide hub と読む順序を更新
- **検証**:
  - `test_doc_site_manifest_exposes_user_guide_expansion`
  - `test_cmd_doc_site_generates_guides_and_api_site`
  - `git diff --check`
- **関連**: 改善設計は [imp-05](docs/development/planning/improvement-designs/imp-05-docs-restructure.md)。エラーコードは DOC-06 / imp-02。

<a id="doc-02"></a>
### DOC-02: book/ がユーザー向けと実装者向けの混在

- **影響度**: 中 / **状態**: resolved
- **内容**: `book/` は L# コンパイラ実装を読む開発者向けの読み物として位置付け、
  `docs/guides/` を L# でアプリやライブラリを書く利用者向けの正面玄関として分離した。
  `docs/site.toml` の book section audience も「コンパイラ実装を読む開発者」に統一した。
- **解消根拠**:
  - `book/preface.md` -- book の読者層と `docs/guides/` との分担を明記
  - `docs/site.toml` -- book section の audience を実装読者向けに統一
  - `docs/guides/README.md` -- 利用者向け guide と book の境界を明記
- **検証**:
  - `test_doc_site_manifest_separates_user_guides_from_implementation_book`
  - `test_cmd_doc_site_generates_guides_and_api_site`
  - `git diff --check`
- **関連**: imp-05 (読者層別の目次再構成)。

<a id="doc-03"></a>
### DOC-03: ドキュメント鮮度追跡 (.lsharp-doc-status) が実装済みだが未運用

- **影響度**: 中 / **状態**: resolved
- **内容**: `.lsharp-doc-status` を repo root に追加し、`examples/metadata.ls` の `abs`
  metadata entry を初回 Fresh ack 済みの代表 fixture として登録した。CI には
  `scripts/ci/doc-status-check.sh` を追加し、`lsharp doc-check examples/metadata.ls --emit-trailers`
  が `.lsharp-doc-status` から reviewer を読んで `Doc-Reviewed-By: docs-maintainers` を返すことを
  gate 化した。運用手順も docs site の operations section へ公開対象として追加した。
- **解消根拠**:
  - `.lsharp-doc-status` -- `abs` entry を `Fresh` / `docs-maintainers` で登録
  - `scripts/ci/doc-status-check.sh` -- CI で `doc-check --emit-trailers` を実行
  - `.github/workflows/ci.yml` -- `Documentation freshness` job を追加
  - `docs/development/operations/documentation-freshness.md` -- ack / check / 更新手順
  - `docs/site.toml` -- operations page として公開対象へ追加
- **検証**:
  - `test_repo_doc_status_dogfooding_is_wired_for_metadata_fixture`
  - `bash scripts/ci/doc-status-check.sh`
  - `test_cmd_doc_site_generates_manifest_pages_and_publish_assets`
  - `git diff --check`
- **関連**: imp-05 (運用フロー設計)。

<a id="doc-04"></a>
### DOC-04: examples/ とドキュメントの連携不足

- **影響度**: 低-中 / **状態**: resolved
- **内容**: `examples/` の tracked な 15 個の `.ls` サンプルは
  `docs/guides/examples.md` の機能マトリクスに登録済み。各サンプルが示す言語機能、
  実行状態、関連ドキュメントを一覧化し、`gadt.ls` / `hkt.ls` / `computation.ls` は
  「型チェックのみ / stub main」、`metadata.ls` は metadata 用サンプルとして区別した。
  `examples/README.md` からも同マトリクスへ導線を張り、`examples/*.wasm` は生成物で
  `.gitignore` 対象であることを明示した。
- **解消根拠**:
  - `docs/guides/examples.md` -- tracked な `examples/*.ls` 15 件の機能マトリクス
  - `examples/README.md` -- source directory 側からマトリクスへの導線
  - `docs/site.toml` -- `Examples Matrix` を `guides/examples.html` として公開対象へ追加
  - `docs/guides/README.md` -- 利用者向け guide hub からの導線
- **検証**:
  - `test_doc_site_manifest_exposes_examples_matrix`
  - `test_cmd_doc_site_generates_guides_and_api_site`
  - `git diff --check`
- **関連**: imp-05 (examples ↔ 機能マトリクス)。

<a id="doc-05"></a>
### DOC-05: language-guide テンプレートと docs/ の二重管理リスク

- **影響度**: 低 / **状態**: resolved
- **内容**: `docs/guides/` を人間向け guide の正本、`docs/site.toml` を公開サイト構成の
  正本として明記し、`crates/lsharp-driver/templates/lsharp-language-guide.md` は AI セッション向けの
  要約として扱う同期方針を追加した。`lsharp language-guide` はこの template を標準出力へ出す
  公開 CLI として維持する。
- **解消根拠**:
  - `crates/lsharp-driver/templates/lsharp-language-guide.md` -- `docs/guides/` / `docs/site.toml` の SSOT を明記
  - `crates/lsharp-driver/src/claude_plugin.rs` -- template の SSOT 文言と主要 guide path を focused test で固定
  - `docs/guides/metadata-driven-development.md`, `docs/guides/deployment-targets.md`, `docs/guides/stdlib-guide.md` -- template と重複していた主要内容を利用者向け docs へ移動
- **検証**:
  - `test_lsharp_language_guide_template_points_to_docs_guides_as_ssot`
  - `test_lsharp_language_guide_template_covers_user_development_workflows`
  - `git diff --check`
- **関連**: imp-05 (正本一本化の方針)。

<a id="doc-06"></a>
### DOC-06: エラーコード体系が docs に未定義

- **影響度**: 中 / **状態**: resolved
- **内容**: `docs/guides/error-reference.md` を `LS####` error code reference の利用者向け正本として
  追加し、MCP `lsharp_errors` も driver 内の共有 `ERROR_CODES` table から説明を返すようにした。
  legacy `E0001`〜`E0005` は互換 alias として `LS1001` / `LS1002` / `LS1004` / `LS1003` へ解決する。
  CLI / LSP / MCP の全診断へ `LS####` を貫通させる作業は引き続き I-02 / imp-02 の範囲に残す。
- **解消根拠**:
  - `docs/guides/error-reference.md` -- `LS####` range、legacy alias、code 一覧、MCP lookup を定義
  - `crates/lsharp-driver/src/error_codes.rs` -- MCP と docs 契約の共有 table
  - `crates/lsharp-driver/src/mcp_server.rs` -- `lsharp_errors` を共有 table 参照へ変更
  - `docs/site.toml` / `docs/guides/README.md` -- error reference を公開 guide へ追加
- **検証**:
  - `test_errors_tool_returns_ls_error_code_reference_and_legacy_alias`
  - `test_errors_tool_accepts_legacy_error_code_alias`
  - `test_error_reference_doc_mentions_all_mcp_error_codes`
  - `test_doc_site_manifest_exposes_user_guide_expansion`
  - `git diff --check`
- **関連**: I-02 (診断統一と `LS####` 貫通)。改善設計は [imp-02](docs/development/planning/improvement-designs/imp-02-error-handling-unification.md)。

<a id="doc-07"></a>
### DOC-07: ドキュメント更新が実装の後追いになり、依頼駆動でしか走らない

- **影響度**: 中 / **状態**: in-design
- **内容**: 実装を先に書き、ドキュメント (ISSUES / TODO / ADR / 運用記録) はユーザーからの明示的な
  依頼があったときだけ後追いで更新される、という運用になっていた。結果として、
  (a) 決定の根拠がコミットメッセージにしか残らない、(b) 台帳に無い既知問題が生まれる (I-11 が実例)、
  (c) 依頼のたびに同じ指示を繰り返す手間が発生する、という三つの損失が出ている。
- **根拠**: 2026-08-16 のユーザー指摘 --「今後毎回ドキュメント更新を依頼するのは面倒」。
  同日時点のハーネス実測では、TDD (実装前にテスト) は `.claude/hooks/tdd-guard.sh` で機械的に
  警告されるのに対し、ドキュメントには同等の仕組みが一切無かった。
- **設計**: TDD と同じ「先に書く」規律をドキュメントへ適用する。
  - `.claude/rules/doc-sync.md` -- 変更種別 → 更新すべき正本の対応表と、doc-RED → 実装 → doc-GREEN の順序
  - `.claude/hooks/doc-guard.sh` -- PreToolUse (Edit|Write)。実装ファイル編集時に台帳/docs が
    未変更なら stderr へ警告する。`tdd-guard.sh` と同じく **非ブロック (`exit 0`)**。
    正当なリファクタや調査を止めないため、ブロック型は採らない
  - `.claude/skills/doc-sync/SKILL.md` -- slice を閉じる時の同期チェックリスト
- **残る問題**: 二点。
  - hook は「何かを書いたか」しか見ない。書いた内容が正しい正本に、正しい粒度で
    入っているかは判定できない。そこは skill のチェックリストと人のレビューに委ねる。
  - 判定が working tree の dirty 状態に依存するため、**commit が事実上のリセット境界**になる。
    未コミットのまま複数 slice を跨ぐと 2 番目以降の slice では警告が出ない。
    slice を閉じたら commit する運用が前提となる。
- **関連**: DOC-05 (二重管理リスク)、`.claude/rules/docs-organization.md` (配置規則の正本)。

<a id="doc-08"></a>
### DOC-08: 陳腐化した記述と重複節

- **影響度**: 低-中 / **状態**: resolved
- **内容**: 二件。
  - `legacy-rust-bootstrap/README.md` は「移行完了時に `crates/` を配置予定」と書くが、
    `docs/development/operations/adr-rust-removal.md` は Rust workspace の物理削除を **withdrawn**
    としている。README が現行 ADR と矛盾しており、ディレクトリの実体は README のみで空である。
  - `TODO.md` に v0.3 milestone 節が 2 箇所ある。二重計上の温床になる。
    行番号は編集のたびにずれるため見出しで指す:
    `## Next milestone — v0.3 review provenance / lifecycle` と
    `## Next milestone — v0.3 Review provenance lifecycle` (`grep -n '^## Next milestone — v0.3' TODO.md`)。
- **是正済み (2026-08-16)**: `CLAUDE.md` の TDD 節が「TODO.md の項目を `[x]` に更新」と書いており、
  `TODO.md` 冒頭の凡例および `AGENTS.md` の「`[x]` は使わない」と正面から矛盾していた。
  DOC-07 のハーネス整備の一環で `[ ]` / `[~]` / `[BLOCKED:]` の 3 状態へ修正した。
- **解決** (2026-08-16): 二件とも是正した。

  1. **`legacy-rust-bootstrap/README.md` を全面書き換え**。「`crates/` / `Cargo.toml` / `Cargo.lock` を
     配置予定」(旧 L13-17) と「ロールバックが必要な場合にのみ参照する」(旧 L23) を撤回し、
     ADR (`docs/development/operations/adr-rust-removal.md:41` の withdrawn / `:55` の維持スコープ表 /
     `:104` の primary rollback path 否定) と整合する内容へ差し替えた。
     現在は「比較・監査用のスナップショット置き場であり、いまは空。Rust workspace は `crates/` に残る。
     rollback の正本は `docs/development/operations/rollback-procedure.md` と `scripts/rollback.sh`」と明記する。

  2. **`TODO.md` の v0.3 milestone 節 2 つを 1 つへ統合**。`## Next milestone — v0.3 Review provenance lifecycle`
     (小文字表記でない側) を、`## Next milestone — v0.3 review provenance / lifecycle` へ merge した。
     **どちらを残すか / どちらを捨てるかの編集判断はしていない**。統合スクリプトが旧節の
     `EC-M3-01`〜`05` の本文を、統合先の対応する `- [~]` bullet 直下へ字下げ継続として verbatim で移し、
     旧節は「上記へ統合した」旨のポインタへ縮めてある (旧節末尾の地の文も保存)。
     - 検証: `EC-M3-0N` ごとに `^- \[~\] \`EC-M3-0N\`` が 1 回ずつであること、旧節の全行が統合先に
       残存すること、文字多重集合の差分が「剥がした 5 つの `EC-M3-0N` ラベルと重複していた正本行 1 行」で
       過不足なく説明できること (net 34 文字) を機械的に確認した。
     - 統合を選んだ理由: `git log -L` で両節を追うと **どちらも upstream で更新が続いており**、
       「片方が陳腐化しているので消す」という選択肢は最初から無かった。
     - **注意**: 旧節はポインタとして**見出しを保持している**ので、上記 内容 欄が挙げる
       `grep -n '^## Next milestone — v0.3' TODO.md` は統合後も 2 hit する。
       二重計上の有無は見出し数ではなく `grep -cE '^- \[~\] \`EC-M3-0N\`' TODO.md` が
       各 ID につき 1 であることで判定すること。

- **`LEGACY-MODULE-01` の二重は誤読であり、是正対象ではない** (2026-08-16 判定):
  `TODO.md` に `LEGACY-MODULE-01` は 2 回現れるが、前者は
  「`- [~] \`LEGACY-MODULE-01\` selfhost/native module cache — 上記 \`C\`。既存項目 (本ファイル後段) を参照。」
  と自ら後段を指す **意図的な前方参照**であり、後者 (`SCC inference and cache generalization`) が本体である。
  v0.3 節のような二重計上ではないので統合しない。
  **同じ誤読の再発を防ぐためここに記録する** — `grep 'LEGACY-MODULE-01' TODO.md` の 2 hit を見て
  「重複だ」と判断してはならない。
- **根拠**: 2026-08-16 実測 (Track 0 調査の副産物)。是正の実施も同日。
- **関連**: DOC-05 (正本の二重管理リスク)。

<a id="doc-09"></a>
### DOC-09: 完了 TODO を削除する際に根拠が ADR へ移されず、原因究明の記録ごと消えている

- **影響度**: 中 / **状態**: resolved / **発見**: 2026-08-19 (`I-27` の由来調査から)
- **内容**: `TODO.md` は「未完了タスクだけを持つ」正本であり、完了項目は
  **ADR / 運用記録へ移してから削除する**と `.claude/rules/doc-sync.md` が定めている。
  実際には移送されず削除だけが行われた実例があり、そこには
  **native 実行時の値破壊の原因究明の記録**が含まれていた。
- **根拠**: `0bd8bd47` 「docs: keep active-only TODO backlog」(2026-07-25) は
  `TODO.md` から **1,364 行**を削除している (同時に 128 行追加。
  `git show --numstat --format='' 0bd8bd47 -- TODO.md` が `128 / 1364`)。
  一方、同 commit は `docs/adr/` を 1 ファイルも追加・変更していない
  (`git show --stat 0bd8bd47`)。移送先が存在しない。

  **当初「1,492 行を削除」と書いていたが (2026-08-19 に訂正)、それは `--stat` の
  変更行数 (追加 + 削除) であって削除行数ではなかった。** `--stat` の数字は
  insertions と deletions の和なので、削除だけを数えるには `--numstat` が要る。
  `I-25` / `I-27` で潰したのと同じ「数を全走査せずに書く」誤りである。

  消えた内容の具体例が `b9d5d4e5` (2026-05-23) の追加進捗である。commit body は空だが
  `TODO.md` 側に **「`map-new depth=1` の bad rel32 target は、直前の stage2 entry
  (`opcode=40 target-param-count=4`) が `call-rel-bytes` を失って cursor を崩す下流症状だった」**
  という原因の記述と、それを突き止めた Rust 側 metadata 診断の手順が書かれていた。
  現在の `TODO.md` を `grep` しても残っておらず (`map-new-four-arg-relref-full-v2` 0 hit)、
  ADR にも無い。**`git log -S` で掘らない限り到達できない状態**になっている。
- **なぜ問題か**: `DOC-07` は「ドキュメントが後追いになる」問題を扱うが、これは逆で
  **一度は正しく書かれた知見が、正本整理の過程で失われる**という別種の損失である。
  後追いは遅れるだけだが、こちらは書いた分が消えるので、同じ調査を後からやり直すことになる。
  実際 `I-27` の起票時、5 件の否定 assertion のうち 1 件だけ原因が判っていたのに、
  台帳からは「原因不明」に見えていた。
- **解決 (2026-08-19)**: `0bd8bd47` の削除 1,364 行を棚卸しし、救出対象を ADR へ移した
  (`DOC-09-01`)。
  - **棚卸しの結果**: 救出すべき記述は **93 行** (原因の絞り込み 61 / 却下した案 35 / 重複 3)。
    該当行を持つ項目は **5 件**で、内訳は `V2-13a-5b` 45 行 (root gap の追跡) /
    `V2-13a-5h` 44 行 (x86 call-rel の 27 却下案) / `V2-13a-5` 2 行 / `V2-15` 1 行 /
    `V2-16` 1 行。**89 行 (96%) が 5b と 5h の 2 項目に集中する。**
    他の項目と、`EC-M2` / 責務分離 30 節 / Phase 11・14・15 の計画節は 0 件である。

    **当初「すべて 2 項目に集中。他 45 項目は 0 件」と書いていたが (2026-08-19 に訂正)、
    項目別の集計には 5 項目が出ていた。** 集計出力を先頭 60 行しか読まずに要約したのが原因で、
    `I-25` / `I-27` / 本 issue の削除行数で潰したのと同じ「全走査せずに数を書く」誤りを、
    その再発防止を書いている当の節でやっていた。再現手順:

    ```bash
    git show --format='' -U0 0bd8bd47 -- TODO.md \
      | awk '/^--- a\//{h=1;next} /^\+\+\+ b\//{next} /^-/ && h {print substr($0,2)}' > deleted.txt
    # 項目ごとの該当行数を数える (先頭だけ見ない)
    grep -cE '原因|判明|症状|下流|真因|突き止|に絞|絞られ|崩れている|化ける|却下|見送|採用しない|断念|不採用' deleted.txt
    ```

    あわせて、救出語の走査を残タスク節 (削除行 356..928) の外側 — `EC-M2` / evidence-refresh 節
    (0..356) と Phase 11・14・15 の計画節 (928..1364) — へも広げ、**該当 0 件**を確認した。
    93 行 / 移送しない 1,271 行の切り分けはこれで全 1,364 行を覆っている。
  - **移送先**:
    [`docs/adr/decisions-native-x86-value-liveness-rejected-approaches.md`](docs/adr/decisions-native-x86-value-liveness-rejected-approaches.md)。
    3 主題 (返却値の root gap / x86 call-rel の helper 呼び出し境界での消失 / IR サイズ別
    fallback の廃止) にまとめた。挙げた test 名 11 件は削除行と現在の test source の
    双方で照合済みで、改名されていたのは 1 件だけだった。
  - **移送しなかった 1,271 行**とその理由は同 ADR の「移送しなかったもの」節に書いた。
  - **再発防止**: `.claude/skills/doc-sync/SKILL.md` の doc-GREEN 手順 4 に、
    削除前の救出走査 (`git diff -U0 -- TODO.md` の削除行を原因/却下語で grep) を足した。
    1 行でも出たら ADR を先に作る。
  - **`I-27` への効果**: 起票時に「原因不明」に見えていた件について、
    「`call-rel` は helper 内ではなく `append-zero-arg-call-bundle-x86` への
    **関数呼び出し境界で既に崩れている**」という当時の到達点が復活した。
    候補 (b) と (c) の裏付けにあたる。
- **関連**: DOC-07 (後追い更新)、DOC-05 (二重管理)、I-27 (この損失で原因不明に見えていた件)、
  `.claude/rules/doc-sync.md` (「ADR / 運用記録へ移してから」の正本)、
  `docs/adr/decisions-native-x86-value-liveness-rejected-approaches.md` (移送先)。

---

## 更新規則

- 新しい問題は該当カテゴリの次番号で追記する (欠番は再利用しない)
- 問題が解消されたら削除せず `状態: resolved` に変更し、解消根拠 (コミット / テスト) を追記する
- 着手タスク化する場合は TODO.md (正本) に項目を作り、本台帳からは ID 参照のみ行う
- file:line の根拠は記載時点の実測とし、大きくずれた場合は検証日とともに更新する
