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
| [I-01](#i-01) | ファイルサイズ規約 (500-800 行) を 39 ファイルが超過 (src 6 / tests 33) | 高 | in-design | [imp-06](docs/development/planning/improvement-designs/imp-06-large-file-decomposition.md) |
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
| [I-23](#i-23) | `aarch64-selfhost-helper-trailer-size` の pin が 2026-08-03 から陳腐化したまま気付かれていない | 中 | resolved | -- |
| [I-24](#i-24) | 診断の「重複」定義が spec 文言 / test / 実装の 3 者で食い違い、文言どおりに直すと lint 指摘が消える | 中 | resolved | [lint dedup identity ADR](docs/adr/decisions-lint-diagnostic-dedup-identity.md) |
| [I-25](#i-25) | `NativeCodegen.ls` に呼び出し元 0 の defn が 64 個。うち 1 群は使用中の実装と乖離している | 低-中 | open | -- |
| [I-26](#i-26) | x86 lane は helper trailer の補正を持たず、`x86-selfhost-helper-trailer-size` は呼び出し元 0 のまま | 中 | open | -- |
| [I-27](#i-27) | x86 native の hot path で user call を挟むと local/引数が壊れる。回避策だけが test に pin され、欠陥そのものが台帳に無い | 中 | open | -- |
| [I-28](#i-28) | x86 native の int-to-string import 呼び出しが rdi を書かない (harness が import placeholder の param-count を 0 で種まきするため) | 中 | open | -- |
| [I-29](#i-29) | aarch64 native の文字列表現が bit 32 を判別子に使うため、heap offset が 4 GiB を越えると base 相対 offset が絶対番地として strlen される | 中 | open | -- |
| [I-30](#i-30) | selfhost TestRunner に legacy scanner 2 本と canonical inventory が並存し、実行対象の正本が二つある | 中 | open | [worktree 取り込み判定](docs/adr/decisions-worktree-absorption-2026-08-20.md) |
| [I-31](#i-31) | `cargo clippy -p lsharp-types -- -D warnings` が main で既に落ちる (`collapsible_if` 1 件が 3 経路へ波及) | 低 | resolved | [occur-check 深さ境界](docs/adr/decisions-infer-occur-check-depth-bound.md) |
| [I-33](#i-33) | analysis-only cache の直後に compile すると空の IR が返る (clean-hit が IR readiness を見ていない) | 中 | resolved | [analysis/compile cache 境界](docs/adr/decisions-legacy-module-analysis-compile-cache-boundary.md) |
| [I-34](#i-34) | `cargo fmt --check -p lsharp-ir` が main で既に落ちる (`lower/mod.rs` の mod 宣言順 8 箇所) | 低 | resolved | -- |
| [I-35](#i-35) | allocator の到達不能 free-list search に誤った `Br(0)` が残り、path を再有効化すると無限 loop する | 低 | resolved | [到達不能 free-list の削除](docs/adr/decisions-allocator-dead-free-list-removal.md) |
| [I-36](#i-36) | AST / token の Display が string escape と型注釈・`:where` を落とし、pretty-print が re-parse できない | 中 | open | -- |
| [I-37](#i-37) | 別 module の同名 top-level function が診断なしに衝突し、誤った wasm を出す (silent miscompilation) | 高 | open | [worktree 取り込み判定](docs/adr/decisions-worktree-absorption-2026-08-20.md) |
| [I-38](#i-38) | import した module の `type-alias` が展開されず `expected String, found Text` になる | 中 | open | [worktree 取り込み判定](docs/adr/decisions-worktree-absorption-2026-08-20.md) |
| [I-39](#i-39) | block 形式の module body が lowering されず、沈黙して無出力バイナリを出す | 高 | resolved | [block 形式 module body の reject](docs/adr/decisions-module-body-form-rejection.md) |
| [I-40](#i-40) | DocTools の metadata 契約が parser の出力 slot 数と食い違う | 低 | open | [worktree 取り込み判定](docs/adr/decisions-worktree-absorption-2026-08-20.md) |
| [I-41](#i-41) | compile cache の hit/miss を集計する手段が無い | 低 | open | [worktree 取り込み判定](docs/adr/decisions-worktree-absorption-2026-08-20.md) |
| [I-42](#i-42) | 静的 contract 判定が `if` / `let` / `do` / `match` を貫通しない | 中 | open | [worktree 取り込み判定](docs/adr/decisions-worktree-absorption-2026-08-20.md) |
| [I-43](#i-43) | `:example` / `:invariant` / `:doc` の識別子検査が false positive を出す | 中 | open | [worktree 取り込み判定](docs/adr/decisions-worktree-absorption-2026-08-20.md) |
| [I-44](#i-44) | 未定義の computation builder が型検査を通る | 中 | resolved | [computation builder の診断](docs/adr/decisions-computation-builder-diagnostics.md) |
| [I-45](#i-45) | selfhost の canonical `:case` preflight が 0 引数 `defn` の呼び出しを型エラーにする | 中 | resolved | [0 引数 defn の型](docs/adr/decisions-selfhost-zero-arity-defn-type.md) |
| [I-46](#i-46) | 前方参照された呼び出しは引数型も arity も検査されていない | 高 | open | [computation builder の診断](docs/adr/decisions-computation-builder-diagnostics.md) |
| [I-47](#i-47) | `cargo fmt --check` が 5 crate で落ちる (`I-34` は `lsharp-ir` しか見ていなかった) | 低 | resolved | -- |
| [I-48](#i-48) | selfhost のソースが `I-46` の穴に依存しており、vector をタプルとして使っている | 高 | open | -- |
| [I-49](#i-49) | selfhost の `:assert` lane は predicate を型検査していない | 中 | resolved | -- |
| [I-50](#i-50) | `lsharp compile` の入力ソース整形上書きが利用者へ通知されない | 中 | resolved | -- |
| [I-51](#i-51) | `compile -o <ディレクトリ成分の無いファイル名>` が artifact 同期で落ちる | 低 | resolved | -- |
| [I-52](#i-52) | LSP stdio 補完の e2e が 2 系統の理由で全滅 (位置規約の食い違い / snapshot 形式のドリフト) | 中 | open (facet A のみ resolved) | -- |
| [I-53](#i-53) | `lsp_stdio` lane 93 本のうち 64 本が赤で、`I-52` の補完 9 本では説明できない | 中 | open | -- |
| [I-54](#i-54) | LSP の response 側の位置が wire 変換前の内部値で fixture に固定されている | 中 | open | -- |
| [I-55](#i-55) | hover / definition / references / rename の fixture が内部 1 始まり座標のまま止まっている | 中 | open (原因判別済み) | -- |

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
| [DOC-10](#doc-10) | 設計ドキュメントと TODO に完了済み項目が蓄積し、残作業が読めない | 中 | resolved | [worktree 取り込み判定](docs/adr/decisions-worktree-absorption-2026-08-20.md) |

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
- **内容**: CLAUDE.md のファイルサイズ規約 (1 ファイル 500-800 行) を超えるソースが
  **39 ファイル** (`src/` 6 / `tests/` 33) あり、エージェント解析精度・レビュー容易性・
  責務分離を損なっている。

  `src/` 超過 (2026-08-22 実測):

  | ファイル | 行数 | 規約比 |
  |---------|------|--------|
  | `crates/lsharp-driver/src/main.rs` | 3254 | 4.1x |
  | `crates/lsharp-driver/src/main_tests.rs` | 3086 | 3.9x |
  | `crates/lsharp-driver/src/mcp_tests.rs` | 1949 | 2.4x |
  | `crates/lsharp-types/src/infer_tests.rs` | 1384 | 1.7x |
  | `crates/lsharp-driver/src/mcp_review_registry_tests.rs` | 1223 | 1.5x |
  | `crates/lsharp-types/src/validation.rs` | 825 | 1.0x |

  `tests/` 超過の上位 (同実測、全 33 件):

  | ファイル | 行数 | 規約比 |
  |---------|------|--------|
  | `crates/lsharp-wasm/tests/e2e/selfhost_native_stage_chain.rs` | 62990 | 78.7x |
  | `crates/lsharp-wasm/tests/e2e/selfhost_cli_core.rs` | 19412 | 24.3x |
  | `crates/lsharp-wasm/tests/e2e/selfhost_lsp_docs_ops.rs` | 6334 | 7.9x |
  | `crates/lsharp-wasm/tests/e2e/strings_patterns_compiler_integration.rs` | 5354 | 6.7x |
  | `crates/lsharp-wasm/tests/e2e/runtime_allocator_closures.rs` | 3061 | 3.8x |

  **2026-07-25 実測の 16 件からの変化。** 当時の表に挙げた 8 ファイルのうち
  `wasi.rs` (4568) / `lsharp-ir/src/lib.rs` (3080) / `compile.rs` (2870) /
  `lower/expr.rs` (2833) / `infer.rs` (2789) / `parser.rs` (2259) / `lsp/lib.rs` (1397) の
  **7 件は分割で解消した**。残る `main.rs` は **2568 → 3254 行へ増えている**。
  現在の `src/` 超過 6 件のうち **4 件は `*_tests.rs`**、つまり production から test を
  切り出した先が今度は超過している。**重心は `src/` から `tests/` へ移った** ので、
  以後の分割対象は `crates/**/tests/**` である。
- **根拠**: 2026-08-22 実測。比較可能性のため取得条件を固定する。

  ```bash
  find crates -path "*/src/*" -name "*.rs" | xargs wc -l | grep -v total | awk '$1>800' | sort -rn
  find crates -path "*/tests/*" -name "*.rs" | xargs wc -l | grep -v total | awk '$1>800' | sort -rn
  ```

  規約は AGENTS.md のファイルサイズ制限。
- **gate の不在**: workspace 全域を走査する行数 gate は無く、per-file の targeted guard が
  7 本あるだけである (`*_file_size.rs`)。`RUST-FILE-SIZE-GATE-01` (`TODO.md`) が引き取る。
  gate を入れると allowlist が初期 39 件になる。
- **関連**: selfhost 側は ADR-168 (STR-01〜03) で分割実績あり (TypeInfer.ls 1093 → 290 行など)。
  Rust 側の分割設計は [imp-06](docs/development/planning/improvement-designs/imp-06-large-file-decomposition.md)。
  分割そのものは `LEGACY-MAINT-01`、gate は `RUST-FILE-SIZE-GATE-01`。
  **超過 13 file 分の分割軸は設計済みのものがある** — `codex/legacy-maintenance-docs-active-only`
  (2026-07-24) が同じ file を分割しており、追加された test 本体は全件 main にある
  (ミスは file-size guard 関数 1 個のみ)。軸の一覧は
  [worktree 取り込み判定](docs/adr/decisions-worktree-absorption-2026-08-20.md)。
  分割案が読めなくなっている件は `DOC-10`。

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
- **未解消部分の具体箇所** (2026-08-22 に `WORKTREE-ABSORB-02` の branch 判定中に特定):
  - `crates/lsharp-tooling/src/api_doc.rs:188-193` -- `build_api_doc_for_file` は
    `miette::miette!("[{}] ... {e}", e.code())` で code を文字列へ埋めるだけで、
    `NamedSource` も `LabeledSpan` も添えない。同 workspace の
    `crates/lsharp-driver/src/main.rs:1739` / `:1776` は添えており、方式が揃っていない
  - `crates/lsharp-lsp/src/util.rs:515 diagnostic_span_from_message` -- LSP は `LS3102` の
    span を診断メッセージの日本語文言 (`"モジュール '"` / `"' が見つかりません"`) を文字列検索して
    復元している。`:493 stable_code_from_message` も `[LSxxxx]` を文字列から抜き出す。
    **文言を変えると span 復元が黙って壊れる**
  - 上の直接原因は API 境界にある。`analyze_single_file_incremental` /
    `analyze_multi_file_incremental_with_overrides` が `Result<(), String>` を返すため、
    `ModuleGraphError` が `code()` (`module_graph.rs:85`) と `span()` (`:94`) を持っているのに
    LSP へ渡る時点で構造が捨てられている (`util.rs:616` / `:640`)
  - `crates/lsharp-ir/src/module_graph.rs:94-98 ModuleGraphError::span()` は
    `ModuleNotFoundAt` にしか `Some` を返さない。**`CyclicDependency` (`LS3101`) と
    `ModuleNotExported` (`LS3103`) は span を持てない。** つまり循環依存と package 非公開 import は
    code こそ安定しているが、surface で位置を指せない
  - 参照実装が `codex/v0.2-diag-api-doc-forwarding-rebased` の 10 commit にある
    -- `feat: forward ... diagnostics` 8 件 (cli source / module graph / lowering / codegen / io、
    repl、lsp、api-doc) と、cycle / export の span を埋める 2 件
    (`be65dcce` / `2e94896c`)。branch 単位では取り込まず、本 issue の設計に沿って経路ごとに閉じる
  - ただし後者 2 件は `import_spans: HashMap<(String,String), Span>` を graph に持たせる方式で、
    main が採った `ModuleNotFoundAt` + `find_import_span` 方式と競合する。**どちらに寄せるかは
    本 issue で決める必要がある** (`imp-02` の設計判断)
  - なお `module_graph/resolve.rs:314 extract_imports` が span を捨てるのは defect ではない。
    error 時にだけ `find_import_span` で読み直す設計 (`:260`) で、hot path に span を載せない取捨選択
- **関連**: DOC-06 は error-reference と MCP lookup まで解消済み。残る貫通作業は
  [imp-02](docs/development/planning/improvement-designs/imp-02-error-handling-unification.md) を参照。
  branch 判定は [decisions-worktree-absorption-2026-08-20.md](docs/adr/decisions-worktree-absorption-2026-08-20.md)。

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
  | `bootstrap_selfhost_lsp_integration` | 2 | selfhost formatter の compile が `UndefinedVar { name: "ast-defn-signature" }`。2 件とも同一 span。**2026-08-22 解消** -- 「未実装」は誤診断で、fixture が `AST.ls` を連結していなかっただけだった。fixture へ足して GREEN、expected FAIL から削除した。現在は 0 件 |
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

- **影響度**: 中 / **状態**: resolved
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
  **この未検証は 2026-08-20 に解消した (下の「`main` 実体での再実行」)。**
- **台帳化 (2026-08-19)**: 受入条件 (c) を
  [`docs/development/validation/ignored-lane-expected-failures.txt`](docs/development/validation/ignored-lane-expected-failures.txt)
  で満たした。`workspace-expected-failures.txt` と同じ `<binary-id> <test-name>` の粒度で
  117 件すべてを分類つきで並べ、`#[ignore]` の理由文字列も併記してある。
  **`scripts/ci/check-workspace-baseline.sh` の入力にはしていない** — 非 ignored lane の
  baseline へ混ぜると「実測に現れない expected」として必ず非 0 になるためで、
  自動検証は付いていない。ここは満たせなかった点として明示しておく。
  台帳は当初 `8a20cfe2` 時点の集合であったが、2026-08-20 に `main` 実体で
  再実行して全 116 行を確認した (下の「`main` 実体での再実行」)。
- **`main` 実体での再実行 (2026-08-20)** — `IGNLANE-01`。
  **615 test / 499 passed / 116 failed / 22,210.06s**。宣言数 615 == 結果行ユニーク数 615、
  重複行 0 で完走判定 OK。突合は `scripts/compare_ignored_lane.py`。

  | 分類 | 件数 |
  |---|---|
  | 新規 FAIL (実測で落ちたが台帳に無い) | **0** |
  | 解消 (台帳にあるが実測で pass) | **0** |
  | 未出現 (台帳にあるが結果行が無い) | **0** |
  | 台帳外の結果行 | 499 (うち FAILED **0**) |

  **したがって台帳 116 件は `main` 実体でそのまま再現し、
  `8a20cfe2..main` の 3 branch はこの lane に 1 件も影響を与えていない。**
  懸念していた `5e992d52` / `1855fa0b` の `LspServerNav.ls` 22 行削除
  (bundle 構成モジュール、116 件のうち 78 件が bundle を組む系) による
  サイズ・オフセット assertion のずれは**起きなかった**。
  未測定だった `test_e2e_selfhost_pipeline_smoke_root_set_keeps_shadowed_slot_during_allocating_value`
  (`939e4ec9` / `TESTGATE-03` が追加) は **pass** で、台帳へ足す必要は無い。
- **上記再実行の取得条件**: worktree `/Users/biwakonbu/github/tmp/lsharp-ignlane-main` の
  `35ea7c32` でビルドした `target/debug/deps/e2e-68ea5703bbb19562` を
  `--ignored --nocapture` で実行。`os.setsid()` で切り離し (6 時間強かかりハーネスの停止を受けるため)。
  `35ea7c32` と現 `main` のコード差分は `scripts/` へ足した解析スクリプト 2 本だけで、
  test binary に入るコードは同一である。Lima VM `lsharp-linux-x86` は Stopped、
  `LSHARP_NATIVE_*` は全て未設定 — 分類 (a) 60 件 / (b) 4 件が到達不能である前提は
  前回と同じである。
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
- **`STALE-PIN-03` の裁定 (2026-08-19)**: 上で保留した 2 件について、
  **実装と test のどちらが正しいかを両方とも確定させた。結論は 1 件ずつ逆である。**
  - `..._x86_int_to_string_import_sets_rdi` — `I-28` として起票した。当初は **実装側の欠陥**と
    書いたが、2026-08-19 のソース読解で **harness の import placeholder 種まき**が原因と確定し、
    `I-28` 本文で訂正した。
  - `..._x86_function_size_matches_generated_length_diagnostic` — **test の harness が陳腐化**。
    診断が出す `777000` レコードは `idx=3, opcode=41 (if), operand=0, depth=1, expected=11, actual=8`
    で、差 `11 - 8 = 3` は `native-drop-bundle-size-x86(1)` と**厳密に一致**する。
    サイズモデル `native-conditional-control-instr-size-x86(depth) = 8 + drop(depth)` は
    実経路 (`append-control-if-instr-x86` -> `emit-control-if-bundle-x86`、`:11975-11978`) と
    整合しており、食い違うのは **harness が bundle 分割前の
    `emit-control-instr-x86` を呼んでいるから**である
    (`selfhost_native_stage_chain.rs:43891`)。
    同じ診断の関数レベル出力が `expected-size 466 == actual-size 466` /
    `expected-start 0 == actual-start 0` であり、**トリガは instr-status のみ**だったことが
    これを裏付ける。日付も一致する — harness は `3eae652c` (2026-05-11)、
    bundle 分割は `4bd9ee9f` (2026-07-15) で、harness の方が 2 ヶ月古い。
  - **production の legacy 経路は欠陥ではない**ことも確認した。
    `emit-native` -> `generate-native` -> `generate-native-x86-64` -> `generate-native-instr-loop-x86`
    -> `emit-control-instr-x86` は非 bundle だが、offset 側も
    `collect-native-offsets-x86` -> `native-plain-instr-size-x86` (drop bundle を含まない) を使うため
    **内部整合している**。bundle 経路も `collect-native-bundle-offsets-x86` ->
    `native-instr-size-x86` で整合する。**両者を混ぜているのは harness だけ**である。
  - 裁定は cargo 非依存で閉じた (診断値は `STALE-PIN-02` の run で取得済み、
    残りはソース読解のみ)。**harness の修正は含めない** — `TODO.md` の
    `STALE-HARNESS-01` が持つ。
- **`STALE-HARNESS-01` を閉じた (2026-08-22)**: `STALE-PIN-03` 裁定 [B] の修正である。
  `selfhost_native_stage_chain.rs:43891` の harness `check-instr-sizes` を、production の
  `codegen-x86-control-loop-fallback-native` (`NativeCodegen.ls:11216`) と同じ
  `emit-control-instr-bundle-x86 ir meta offsets idx frame-base-slot-count depth` へ揃えた。
  `frame-base-slot-count` / `depth` はどちらも harness のスコープに既にあり、
  **裁定で決着済みの形へ寄せるだけだったので ADR は起こしていない** (`STALE-PIN-02` と同じ扱い)。

  | test | 修正前 | 修正後 |
  |---|---|---|
  | `..._representative_x86_function_size_matches_generated_length_diagnostic` | FAILED (52.78s) | ok (64.82s) |
  | `..._linux_x86_actual_seed_function_size_matches_generated_length_diagnostic` | FAILED (70.98s) | ok |
  | `..._linux_x86_actual_seed_segmented_function_size_matches_generated_length_diagnostic` | FAILED (50.23s) | ok |

  RED の診断レコードは `[777000, 3, 41, 0, 1, 11, 8, 10, 0, 0, 466, 466, 4, 4, 48, 96]` で、
  **裁定時に記録した `idx=3 / opcode=41 (if) / operand=0 / depth=1 / expected=11 / actual=8` と
  逐語で一致した**。差 `3` は `native-drop-bundle-size-x86(1)`、function レベルは
  `expected-size 466 == actual-size 466` で、トリガが instr-status のみである点も一致する。
  裁定はソース読解だけで下したが、**実測が後から同じ数を出したので裁定の前提が裏付けられた**。
- **受入条件 (2) は 1 回の GREEN で満たせる**: `check-function-sizes` は mismatch を見つけると
  診断レコードを出して**その場で再帰を止める** (`:44002-44003` の `0` 復帰。次の
  `check-function-sizes` 呼び出しへ進まない)。したがって出力の先頭が `-1` であることは
  「最初の mismatch が無い」ではなく「全 function / 全 instr を走り切って mismatch が 0」を意味する。
  「診断は最初の mismatch で止まるので 1 回 GREEN を見て終わりにしない」という受入条件の懸念は、
  **sentinel の位置を読めば 1 回で discharge できる**ものだった。
- **同族 2 件の分類が誤っていた (2026-08-22 訂正)**: `ignored-lane-expected-failures.txt` は
  `..._linux_x86_actual_seed_*` の 2 件を分類 (a) 「Lima VM 依存のため未実測」としていたが、
  **macOS ローカルでそのまま実行でき、修正前 RED / 修正後 GREEN を実測できた**。
  test 名の `linux_x86` は seed source の名前であって実行環境の要求ではない。
  分類は「test 名の prefix ではなく関数本体を読んで分けた」と手続きを明記してあり、
  その手続き自体は正しいが、**seed 名と env 依存の区別までは救えていなかった**。
  分類を (a) 60 → 58 / (c) 52 → 51、一覧を 116 → 113 件へ更新した。
- **含めなかったもの**: `test_e2e_native_aarch64_map_insert_instr_size_matches_emitted_length`
  (`:16762`、harness `:18007`) は同じ書き方 — `emit-control-instr-x86` を末尾 2 引数なしで呼ぶ —
  をしているが、**現に pass しており** (回帰確認 `ok` 19.08s)、台帳にも載っていない。
  こちらの harness は `exact` を size モデルと突き合わせず emit 列そのものの検査に使っており、
  bundle 版へ替えると**検査対象が変わる**。落ちていないものを「同じ形だから」で書き換えるのは
  根拠の無い変更なので触っていない。**「形が同じ」は「同じ欠陥がある」を含意しない。**
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
  一律の末尾 slot は採れない。
  **訂正 (2026-08-20)**: ここに続けて「加えて selfhost に offset → line/col 変換が存在しないので、
  本項目は span を載せるだけでは閉じない」と書いていたが、**この前提は誤りである**。
  変換は `lsp-position-from-offset` (`LspServerNav.ls:285`) / `lsp-range-from-offsets` (`:288`)
  として既にあり、呼び出し元も 7 箇所ある。合わせて「`review-collect-node` がソースを受け取らないので
  走査の signature を変える必要がある」という重さの見積もりも誤りだった。投影境界
  `lsp-source-lint-diagnostics [src]` (`Cli.ls:1681`) が既に `src` を持っているため、
  `src` を要するのは `lsp-review-diagnostic-to-lsp` (`Cli.ls:1660`) とその loop だけで、
  これは兄弟 2 本 (`:1400` / `:1450`) が既に取っている引数である。
  観測値 `0:0..0:0` の機構も特定した。`DocTools.ls:713` / `:732` が line/column を **1 1 で直書き**し、
  `render-standard-diagnostic-json` (`LspServerCore.ls:613-616`) が JSON 境界で 1 を引いて
  0-based にする。1 − 1 = 0 である。詳細と、review 診断 slot 4/5 を line/col のまま残す判断
  (第 2 の消費者 `DocJson.ls:111` と snapshot `tests/snapshots/doctools/review-payload.json` が
  `line: 1, column: 1` を pin しており、その経路には `src` が無い) は
  [lint span の AST 表現 ADR](docs/adr/decisions-lint-span-ast-representation.md) の Evidence 節。
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
  `0x34` 以降の 63 word が異なる。

  当初ここに「`git log -S` で追うと **chunk 群 (`e9f761cb`, 2026-04-29) の方が
  本体 (`901c10d8`, 2026-04-22) より後**であり、新しい実装を書いたが呼び出し側を
  差し替え忘れた形に見える」と書いた。**この読みは 2026-08-19 に撤回した。**
  `git log -S` が拾ったのは名前の初出であって実装の新旧ではない。実際は逆で、
  chunk 群が**置き換えられた旧実装**である (根拠は下の「どちらが正しいかは決着した」)。
  以下 2 段落の構造差の記述は観測事実としては正しいが、
  「chunk が新しい」という前提で読まないこと。

  逆アセンブルすると構造差は 2 点。
  (a) 本体は 2 引数それぞれに **`tbz x23/x24, #32`** (`0xb6000137` / `0xb6000138`) を持つが
  chunk には無い。(b) chunk は `0x74` / `0xbc` に「長さ 0 / ポインタ 0」を書き込む
  default 経路を持ち、本体はそこを 1 命令で通過する。

  **(a) の bit 番号を 2026-08-19 に訂正した。当初「`tbz ..., #63`」と書いたが誤りで、
  `0xb6000137` は `TBZ x23, #32` である。** TBZ/TBNZ の bit 番号は
  `b5` (bit 31) と `b40` (bits 23-19) の連結で、`0xb6000137` は `b5=1, b40=0` なので 32。
  bit 63 を見る `TBNZ` は `0xb7f80137` の方で、**こちらは chunk にもある**。
  つまり chunk に無いのは bit 63 の判定ではなく **bit 32 の判定**である。
- **どちらが正しいかは決着した (2026-08-19。実行は不要だった)**:
  **本体 (bit 32 判定を持つ方) が新しく、chunk 群は置き換えられた旧実装の分割コピーである。**

  決め手は byte 一致である。`e9f761cb^` 時点の `emit-aarch64-selfhost-string-concat-helper`
  の 77 word と、現在の chunk1-4 を連結した 77 word が**完全に一致する**。

  ```
  git show e9f761cb^:selfhost/src/Backend/Native/NativeCodegen.ls   # 旧本体を取り出す
  # 旧本体の append-encoded-u32-rooted 引数列 == chunk1..4 の連結 (77 word、差分 0)
  ```

  `e9f761cb` (2026-04-29) 単独で **旧本体の削除・chunk1-4 の追加・新本体の追加**が
  同時に起きている (`git show e9f761cb -- .../NativeCodegen.ls` の `-(defn ...helper []`
  1 箇所と `+(defn ...helper-chunk1..4 []` / `+(defn ...helper []` 5 箇所)。
  `git log -S` が示す「本体 `901c10d8` / chunk `e9f761cb`」は**名前の初出**であって
  実装の新旧ではない。**「新しい実装を書いたが呼び出し側を差し替え忘れた」という
  当初の読みは撤回する。** 実体は「本体を書き換えたが、旧版を分割して置き去りにした」である。

  **両版が食い違う入力も特定できた。** 引数ごとの分岐はこうなっている
  (`x21` は遅延初期化される heap base、`0x008` の `CBNZ x21` が初期化を跳ばす)。

  | 入力 | 現本体 | chunk (旧本体) |
  |---|---|---|
  | 0 | default 経路 | default 経路 (同じ) |
  | bit 63 が立つ | tag を落として `x21 + v` を base 相対の長さ前置文字列として読む | 同じ |
  | bit 63 clear / **bit 32 clear** | **`x21 + v` を base 相対の長さ前置文字列として読む** | **`v` を絶対番地の NUL 終端文字列として strlen する** |
  | bit 63 clear / bit 32 set | `v` を絶対番地の NUL 終端文字列として strlen する | 同じ (同一) |

  つまり差が出るのは **bit 63 と bit 32 が両方 clear の非 0 値**だけで、
  当初書いた「bit 63 が立っていないポインタ」よりも狭い。
  そしてこれは **tag の無い base 相対 offset そのもの**である。
  同じ 3 分岐は生きている `emit-aarch64-selfhost-string-char-at-helper` も持つ
  (`TBNZ#63/x9` と `TBNZ#32/x9`) ので、bit 32 の判別は runtime の文字列表現契約の一部であり、
  chunk 側はその case を扱えない。

  **本裁定のスコープ外だが、この場で気づいたこと**: bit 32 を判別に使う以上、
  base 相対 offset が **4 GiB を越えた時点で誤読が起きる** (offset の bit 32 が立ち、
  絶対番地の NUL 終端文字列として strlen される)。`I-13` の macOS materializer は
  heap を 8 GiB へ倍増しても同じ位置で落ちる実測を持っており、
  4 GiB 超の offset は仮定の話ではない。ただし本 issue の裁定は
  「chunk と本体のどちらが旧実装か」という byte 同一性だけに依っており、
  どの表現が実際に現れるかには依らないので、受入条件 (2) は再開しない。
  この観測は 2026-08-20 に `I-29` / `NATIVE-STR-TAG-01` として独立採番した。
  なお `x21` が mmap した heap base、`movz x22, #1, lsl #16` = 0x10000 は
  確保 frontier の**初期 offset** である (当初この 2 つを混同して書いていたのを訂正)。
- **残るのは削除だけ**: 裁定は済んだので、chunk1-4 を削除して本体を残す。
  `crates/lsharp-wasm/tests` からの参照も無いので test は壊れない。
  ただし `selfhost/src` の編集は source fingerprint を動かすため cargo と stage0 再生成が要る。
  作業項目は `TODO.md` の `NATIVE-DEAD-01` 受入条件 (3)。
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

<a id="i-28"></a>
### I-28: x86 native の int-to-string import 呼び出しが rdi を書かない (harness seed の import placeholder が param-count 0 のため)

- **影響度**: 中 / **状態**: open / **発見**: 2026-08-19 (`STALE-PIN-03` の裁定から)
- **内容**: `(int-to-string 42)` を **当該 test harness の seed で** x86-64 native codegen すると、
  生成される call site は **引数を rdi へ移さない**。helper 側は rdi から読むので、
  変換されるのは caller のレジスタに偶然残っていた値である。
  production の生成器が同じ結果になるかは**未確定** (下の「production 側の帰趨は未確定」を見よ)。
- **裁定 (2026-08-19 に訂正)**: 当初は **「実装側が誤り」** と書いたが、後続のソース読解で
  **harness の import placeholder 種まきが原因**であることが判明した。下の
  「**根本原因 (2026-08-19 に確定)**」節が正本であり、上の一文は撤回する。
  `test_e2e_selfhost_x86_int_to_string_import_sets_rdi`
  (`crates/lsharp-wasm/tests/e2e/selfhost_native_stage_chain.rs:20364`) は
  `48 89 c7 51 e8 .. .. .. .. 59` (= `mov rdi,rax; push rcx; call rel32; pop rcx`) を
  entry から 128 byte の窓で探し、見つからずに落ちている。
- **証拠 (cargo 非依存。`NativeCodegen.ls` の byte リテラル復元のみで閉じる)**:

  | # | 位置 | 事実 |
  |---|---|---|
  | 1 | `:10365` `emit-x86-selfhost-int-to-string-helper` | 40 個の `append-encoded-u32-rooted` リテラルを LE 復元すると先頭は `41 56 53 48 89 fb 4d 8b 16 ...` = `push r14; push rbx; mov rbx, rdi; ...`。**helper は引数を rdi から読む** |
  | 2 | `:10968-10986` `codegen-x86-non-one-arg-call-bundle` | `target-param-count = 0` かつ `current-depth = 1` の分岐は `(emit-push-rax) ++ call-rel-bytes ++ (emit-pop-rcx)` = `50 e8 .. .. .. .. 59`。**観測バイト列と一致し、rdi へは何も書かない** |
  | 3 | `:7179-7197` `emit-one-arg-call-x86-core-with-call-bytes` | `mov-rdi` ++ `push-rcx` ++ `call-rel` ++ `pop-rcx`。**test が要求する列はこの経路の出力そのもの**であり、test 側に古い定数は 1 つも無い |

  1 と 2 から「helper が読む register を call site が書かない」が確定し、
  3 から「test の期待は現在の実装内に存在する正しい呼び出し規約である」が確定する。
  したがって 2 つの候補 — pin の陳腐化 / 別系統だが正しい規約 — はどちらも排除される。
- **根本原因 (2026-08-19 に確定。cargo 非依存のソース読解のみ)**:
  **harness が `int-to-string` の import placeholder に param-count 0 を種まきしている。**
  したがって `codegen-x86-opcode-call-bundle` の `(= target-param-count 1)` 分岐に入れず、
  `48 89 c7` は**構造的に生成されえない**。assertion は当該 harness では原理的に満たせない。

  | # | 位置 | 事実 |
  |---|---|---|
  | 4 | `CompilerBase.ls:428` `ftable-with-native-runtime-imports` | `int-to-string` を **func-idx 6** に登録する。call codegen の `(= operand 6)` 特別扱いと一致する |
  | 5 | `NativeCodegen.ls:14244` `generate-native-x86-64-bundle-with-import-count` | `n = (- (vector-length functions) import-count)`。**先頭 import-count 個が import meta である**という契約を持つ |
  | 6 | `selfhost_native_stage_chain.rs:350-365` `push-import-placeholders` (既定版) | `(make-function-meta 0 0 (vector-new 0))` を **10 個一様に**積む。idx 6 も param-count 0 |
  | 7 | 同 `:348` `root_linux_x86_seed_int_to_string_import_arity` | 既定版を `(if (= idx 6) 1 0)` へ書き換える rewriter。**修正は既にリポジトリ内にある** |
  | 8 | 同 `:663` | 7 の**唯一の呼び出し元**。Linux-x86 root seed 専用で、失敗 test が使う `run_selfhost_main_native_x86_segmented_host_bytes_harness_with_payload_and_args` (`:19572`) には適用されない |

  当初の候補 (a) (b) はいずれも**否定された**。
  - (a) 「`function-metas` を生 operand で引くのが不整合」 -- **否定**。証拠 5 のとおり
    `functions` は先頭に import meta を持つ設計であり、`function-starts` を
    `(- operand import-count)` で引くのと整合する。不整合ではない。
  - (b) 「`wrap-ir-functions-as-meta-loop` が param-count 0 を固定する」 -- **本経路では否定**。
    失敗 test は `emit-native-bundle` を通らず、`compile-file-functions-payload-with-cache` +
    harness の `push-import-placeholders` を通る。ただし (b) 自体は
    `emit-native-bundle` 経路に残る別個の欠陥であり、取り消さない。
- **production 側の帰趨は未確定**: `selfhost/src/**.ls` には `push-import-placeholders` に
  相当する import meta 構築点が**存在しない** (`make-native-function-meta` の呼び出しは
  `:20581` の wrap loop と `:20673` の normalizer の 2 箇所のみで、前者は 0 固定、
  後者は既存値の保存)。したがって「production も同じ症状を持つか」は本読解では決まらない。
  **harness を直すことと production を直すことは別の作業である。**
- **なぜ気付かれなかったか**: `I-23` と同じ構造である。本 test は `#[ignore]` で
  `--ignored` lane からしか走らず、CI 自動実行は 2026-07-12 から停止している (`I-19`)。
  非 ignored lane の baseline (`workspace-expected-failures.txt`) にも載らない。
- **含めない範囲**: 修正そのもの。`TODO.md` の `NATIVE-IMPORT-ABI-01` が持つ。
- **関連**: I-23 (裁定の親)、I-27 (同じ x86 call site 周辺の値破壊)、I-19 (CI 停止)、
  I-21 (x86-64 の root API 未適合)、I-25 (呼び出し元 0 の defn 棚卸し)。

---

<a id="i-29"></a>
### I-29: aarch64 native の文字列表現が bit 32 を判別子に使い、4 GiB 超の heap offset で誤読する

- **影響度**: 中 / **状態**: open
- **内容**: aarch64 native の文字列値は 1 word に 3 形式を詰めており、判別を
  **bit 63 と bit 32 の 2 本の test 命令**で行っている。

  | bit 63 | bit 32 | 解釈 |
  |---|---|---|
  | 1 | -- | tag 付きの base 相対 offset |
  | 0 | 0 | untagged な base 相対 offset。実体は `x21 + v` の長さ前置文字列 |
  | 0 | 1 | **絶対番地**の NUL 終端ポインタ。strlen ループへ入る |

  実際に稼働する `emit-aarch64-selfhost-string-concat-helper`
  (`selfhost/src/Backend/Native/NativeCodegen.ls:15057`) は 2 引数それぞれについて
  `tbnz xN, #63` (`3086483767` / `3086483768` を `:15077` / `:15095` で emit) に続けて
  `tbz xN, #32` (`3053453623` / `3053453624` を `:15080` / `:15100` で emit) を出す。

  **したがって base 相対 offset が 4 GiB (2^32) に達した時点で bit 32 が立ち、
  同じ値が絶対番地として strlen される。** offset は `x21` (mmap した heap base) 起点で、
  確保 frontier `x22` は `movz x22, #1, lsl #16` = 0x10000 から単調増加する。
- **これが仮定の話でない根拠**: heap 確保サイズは
  `native_host_bundle_alloc_size` (`crates/lsharp-wasm/tests/e2e/selfhost_native_stage_chain.rs:37642`)
  が `data_frontier.max(0x10000) + 0x1_0000_0000` を返す形で、**最小でも 4 GiB、
  data_frontier がある分だけ 4 GiB を越える**。加えて `I-13` の macOS materializer は
  heap を 8 GiB へ倍増しても拡大分をちょうど使い切って落ちる実測を持っており、
  4 GiB 超の offset は到達しうる領域である。
- **未確定なこと**: 上記 alloc size 式は **test harness 側にしか無く**
  (`grep` で production 側に同名関数は 0 件)、production materializer が実際に何 byte
  確保するかは未確認である。よって「production で今すぐ踏む」とは断定しない。
  bit 32 が実際に立つ offset で `string-concat` が呼ばれる経路の有無も未確認。
  **判別子の設計が 4 GiB で破綻する**という静的事実だけが確定している。
- **見つかった経緯**: `I-25` (chunk 群と本体の byte 突き合わせ) の副産物。
  `I-25` の裁定自体は byte 同一性だけに依り、どの表現が実際に現れるかには依らないので、
  `I-25` の受入条件は本件では再開しない。
- **失敗の仕方**: 落ちるのではなく**誤読する**。base 相対 offset を絶対番地とみなして
  strlen するので、運が良ければ SIGSEGV、悪ければ無関係なメモリを文字列として読む。
  `I-25` で却下した「一律の長さ変更」や `LINT-SPAN-01` の長さ probe と同じ、
  **判別子の値域を暗黙に仮定する**という失敗の類型である。
- **次の一手**: production materializer の heap 確保サイズを確定させ、4 GiB を越えるなら
  (a) 判別子を bit 32 から heap 上限に依らない別の位置へ移す、(b) heap を 4 GiB 未満に
  抑える、のいずれかを選ぶ。`NATIVE-HEAP-02` の回収機構が入れば (b) が現実的になる。
- **関連**: `I-13` (heap に回収機構が無い)、`I-25` (発見経緯)、
  `TODO.md` の `NATIVE-HEAP-01` / `NATIVE-HEAP-02`。

---

<a id="i-30"></a>
### I-30: selfhost TestRunner に legacy scanner 2 本と canonical inventory が並存し、実行対象の正本が二つある

- **影響度**: 中 / **状態**: open / **発見**: 2026-08-20 (滞留 worktree の取り込み判定から)
- **内容**: `selfhost/src/Tools/Test/TestRunner.ls` (5019 行) には、metadata から
  test case を取り出す経路が **2 系統**ある。

  | 経路 | 実体 | 性質 |
  |---|---|---|
  | canonical | `extract-parser-contract-suites` とその一族 (`:275-357`) | ordered form / span / owner を保つ inventory |
  | legacy | `collect-defn-metadata-loop` / `extract-test-cases-loop` | source を独自に再走査する旧 scanner |

  canonical 側は後から入ったが、**legacy 2 本は削除されていない**。
  source order / grouping / span の正本が inventory なのに、実行対象を決めるのが
  どちらなのかがソースからは一意に読めない。

- **経緯**: この drift は 2026-07-17 の `986ac1e3`
  (`refactor: generate selfhost tests from contract forms`) が既に予告していた。
  当該 commit は legacy 2 本を削除して converter へ収束させる設計だったが、
  main はその後 `extract-parser-contract-suites` という**別の**canonical 経路を
  入れたため、7 ヶ月前のパッチはそのままでは当たらない。
  したがって `986ac1e3` は却下し、**予告された問題の方を本台帳へ移した**
  (判断は [worktree 取り込み判定 ADR](docs/adr/decisions-worktree-absorption-2026-08-20.md))。
- **根拠**: 2026-08-20 実測。`grep -c` で `collect-defn-metadata-loop` 4 hit /
  `extract-test-cases-loop` 6 hit、`contract-forms-to-test-cases` 0 hit。
- **含めない範囲**: legacy scanner の削除そのもの。削除には現 runner の
  result shape と invariant-first suite shape を保つ確認が要るため、独立の slice にする。
- **関連**: `LEGACY-TEST-01` (runner 系の後始末)、DOC-05 (正本の二重管理リスク)。

---

<a id="i-40"></a>
### I-40: DocTools の metadata 契約が parser の出力 slot 数と食い違う

- **影響度**: 低 / **状態**: open / **発見**: 2026-08-22 (`codex/legacy-maint-native-differential-split-audit` の判定中)
- **内容**: `selfhost/src/Syntax/Parser.ls:1140` は defn metadata を **6 slot**
  (`[doc-string, example-text, params-vector, returns-string, invariant-expr, ordered-forms]`) で返すが、
  `selfhost/src/Tools/Doc/DocTools.ls:120` のコメントは **5 slot** を契約として宣言したままで、
  `extract-defn-metadata` は raw vector をそのまま返す。**公開 accessor から slot 5 が漏れる**。
- **根拠**: 現状は観測可能な破綻を起こさない。DocTools の consumer 4 件
  (`:135` / `:144` / `:153` / `:162`) はいずれも `(> (vector-length meta) N)` で
  index guard しており、余分な slot を読まない。したがって**契約文書と実装の不一致**であって
  実害はまだ無い。「気づいたがスコープ外」として捨てないために起票する。
- **`I-37` との絡み**: `Tools.Doc.DocTools` と `Tools.Text.FormatterDecl` は
  **どちらも `extract-defn-metadata` を定義していて衝突している**。
  辞書順で `Tools.Text.FormatterDecl` が勝つため、DocTools の consumer が実際に呼ぶのは
  FormatterDecl 側である。両者の本文は補助関数名 (`doc-defn-signature-node?` /
  `formatter-defn-signature-node?`) が違うだけで、その補助関数自体が逐語同一なので
  **現状この衝突は無害**。ただし **DocTools 側だけを 5 slot へ切り詰める修正は効かない** —
  切り詰めた定義は衝突に負けて呼ばれない。さらに `FormatterDecl.ls:323` / `:414` は
  slot 5 を実際に読むので、切り詰めた側が勝つ順序になれば formatter が壊れる。
- **含めない範囲**: 重複名そのものの解消は `MODULE-DUP-FN-01` / `I-37`。
- **関連**: 参照実装は `codex/legacy-maint-native-differential-split-audit` の `67624ca7`
  (`extract-defn-metadata-raw` への rename + `project-doc-defn-metadata` による slot 5 の切り落とし)。
  **この patch は main には当てない** — 上記のとおり衝突に負けて projection が発火しないため。
  判定は [worktree 取り込み判定](docs/adr/decisions-worktree-absorption-2026-08-20.md)。

<a id="i-41"></a>
### I-41: compile cache の hit/miss を集計する手段が無い

- **影響度**: 低 / **状態**: open / **発見**: 2026-08-22 (`codex/legacy-module-cache-format-identity` の判定中)
- **内容**: main の compile cache は 2 層ある。`lsharp-ir` の in-memory `ModuleCache`
  (`crates/lsharp-ir/src/cache.rs`) と、process 間で共有する
  `ArtifactCache` (`crates/lsharp-tooling/src/artifact_cache.rs`) である。
  **どちらも累積 counter を持たない。** 1 回の compile が cache から来たかは
  `CompileArtifacts::from_cache` で分かるが、**module 単位の hit/miss、link を
  full build したか、cache が何回無効化されたかは観測できない**。
- **根拠**: 2026-08-22 実測。`grep -rn "note_module_hit\|CompileStats\|cache_hits" crates/*/src/`
  が 0 件。`crates/lsharp-ir/src/cache.rs` の `pub fn` は
  `fingerprint` / `deps_key` / `ast` / `imports` / `ir` / `has_ir` / `type_result_len` /
  `len` / `prepare_for_entry` / `get` / `remove_module` のみで counter は無い。
- **なぜ問題か**: dev-loop 高速化の評価が壁時計でしかできない。
  「速くなったのは cache が効いたからか、単に対象が減ったからか」を切り分けられないため、
  `LEGACY-MODULE-01` の残作業 (segment reuse、自動 eviction) の効果測定が
  **測定条件の作り込みに依存してしまう**。
- **含めない範囲**: cache 実装そのものの変更。counter を足すだけで、
  `ArtifactCache` の envelope 形式や `ModuleCache` の invalidation 規則は変えない。
- **関連**: 参照実装は `codex/legacy-module-cache-format-identity` の `bd7d540b`
  (`note_compile_call` / `note_module_hit` / `note_module_miss` / `note_link_cache_hit` /
  `note_link_full_build` / `reset_stats`)。ただし branch の counter は
  同 branch の `PersistentCompileCache` に載っており、**main にその型は無い**ので
  API 面はそのままでは移植できない。`CACHE-TELEMETRY-01` (`TODO.md`) が引き取る。
  判定は [worktree 取り込み判定](docs/adr/decisions-worktree-absorption-2026-08-20.md)。

<a id="i-42"></a>
### I-42: 静的 contract 判定が `if` / `let` / `do` / `match` を貫通しない

- **影響度**: 中 / **状態**: open / **発見**: 2026-08-22 (`WORKTREE-ABSORB-02` の判定中)
- **内容**: `crates/lsharp-types/src/canonical_contract_check/non_vacuity.rs` の
  `static_boolean_result` (`:321`) が扱うのは Ann の unwrap / Bool literal / 単項 `not` /
  二項 `and` `or` / Int literal 比較だけである。`if` / `let` / `do` / `match` で包むと
  静的判定が届かず、**何も検査しない contract が gate を通過する**。

  ```lisp
  (defn checked [] :assert [(if true true false)] true)   ; 診断 0 件で通る
  (defn checked [] :assert [true] true)                   ; 「静的に true」で拒否される
  ```

  `:assert` と `:property` の precondition の両方で同じ穴が開く。
- **根拠**: 2026-08-22、`check_metadata(&parse(src))` を直接呼ぶ一時 integration test で実測。
  CLI (`lsharp test`) は selfhost runner を経由するため、vacuous な fixture と正当な fixture の
  両方が `firstErrorCode:3002` を返して判別できない。**probe は Rust API を直接叩くこと。**

  | 入力 | 診断 |
  |---|---|
  | `:assert [true]` (control) | 1 件 「:assert predicate は静的に true で検査を識別できず vacuous です」 |
  | `:assert [(= 1 1)]` (control) | 1 件 同上 |
  | `:assert [(if true true false)]` | **0 件** |
  | `:assert [(let [a true] a)]` | **0 件** |
  | `:assert [(do true)]` | **0 件** |
  | `:assert [(match true (true true))]` | **0 件** |
  | `:property` の precondition (Int 比較, control) | 1 件 「:property の precondition は到達不能で vacuous です」 |
  | `:property` の precondition (`if` / `let` / `do` / `match`) | **各 0 件** |
- **範囲外**: String 比較版は main では再現しない。`(= "a" "b")` は非空虚性の判定より先に
  型推論が `[E0004] 型の不一致: expected Int, found String` で落ちるため、
  branch の `8f12109a` / `badf2181` に対応する穴は main には無い。
- **関連**: 参照実装は `codex/v0.2-ec-m1-02-integration` の `17685bab` (conditional) /
  `6771ca26` (sequenced) / `0593e1a6` (let) / `acd75035` (match) / `60a7e736` (checked
  static predicates)、および先行する `550f1851` / `b102e4f7` / `41ab5e44` / `ce5563bc`。
  `STATIC-CONTRACT-01` (`TODO.md`) が引き取る。
  判定は [worktree 取り込み判定](docs/adr/decisions-worktree-absorption-2026-08-20.md)。

<a id="i-43"></a>
### I-43: `:example` / `:invariant` / `:doc` の識別子検査が false positive を出す

- **影響度**: 中 / **状態**: open / **発見**: 2026-08-22 (`WORKTREE-ABSORB-02` の判定中)
- **内容**: `crates/lsharp-types/src/metadata_check.rs:64` が組み立てる `all_names` は
  top-level の `Defn` / `TypeDef` / `RecordDef` / `TypeAlias` / `TraitDef` の**宣言名だけ**で、
  ADT の variant 名・trait の method 名・quote されたシンボルを含まない。
  `metadata_check/diagnostics.rs` の `check_invariant` (`:105`) と `check_example` (`:139`) は
  この集合に無い識別子を **Error** で拒否するため、**正当なプログラムが弾かれる**。
  `:doc` のバッククォート識別子検査 (`:62`) は `is_builtin` を通さないので builtin も警告になる。
- **根拠**: 2026-08-22、`check_metadata(&parse(src))` を直接呼ぶ一時 integration test で実測。

  | 入力 | main の診断 | 期待 |
  |---|---|---|
  | `(defn helper [x] x)` + `:example [(helper 1)]` (control) | 0 件 | 0 件 |
  | `(type Color Red Green)` + `:example [(f Red)]` | 1 件 Error 「未定義の識別子 'Red'」 | 0 件 |
  | `(type Color Red Green)` + `:invariant (= c Red)` | 1 件 Error 同上 | 0 件 |
  | `(trait (Show a) (defn show [self] 0))` + `:example [(show x)]` | 1 件 Error 「未定義の識別子 'show'」 | 0 件 |
  | `:invariant (= 'sym 'sym)` | **2 件** Error 「未定義の識別子 'sym'」 | 0 件 |
  | `:example [(f 'sym)]` | 1 件 Error 同上 | 0 件 |
  | `:doc "uses \`println\` and \`+\`"` | 2 件 Warning 「プログラム中に見つかりません」 | 0 件 |
  | `(type Point (record ...))` + `:example [(f (Point 1 2))]` | 0 件 | 0 件 |

  quote が 2 件になるのは `references.rs:108` / `:209` が `Expr::Quote` の内部式へ
  そのまま降りるためで、quote 深度を持たない構造上の帰結である。
- **範囲外**: `(module ...)` 本体の contract。main の `check_metadata` は `program.decls` の
  top-level しか走査せず module 本体へ降りないので、module 内の `:example` は**検査自体が走らない**。
  false positive も出ないが検査もされない。ここは `I-39` の側の問題であり、本 issue では扱わない。
  `I-39` は compile 経路での reject までを閉じた ([ADR](docs/adr/decisions-module-body-form-rejection.md))
  が、metadata 検査経路が module 本体へ降りない点は**そのまま残っている**。
- **関連**: 参照実装は `codex/v0.2-ec-m1-02-integration` の `e4fab504` (ADT constructor) /
  `95bcfc53` (trait method) / `5da4d83c` (quote 境界) / `3ac2227a` (builtin doc 参照) /
  `420b2eaa` (macro / builder symbol) / `971840ac` (constrained type member)。
  ただし branch 側はこれらを `check_metadata_from_contract_inventory` という**別の入口**へ
  実装しており、main の `check_metadata` へはそのままでは当たらない。
  `CONTRACT-SCOPE-01` (`TODO.md`) が引き取る。
  判定は [worktree 取り込み判定](docs/adr/decisions-worktree-absorption-2026-08-20.md)。

<a id="i-44"></a>
### I-44: 未定義の computation builder が型検査を通る

- **影響度**: 中 / **状態**: resolved / **発見**: 2026-08-22 (`WORKTREE-ABSORB-02` の判定中)
- **内容**: `(computation missing (return 42))` の `missing` がどこにも定義されていなくても
  `Infer::infer_program` が `Ok` を返す。builder 名に fresh type variable が割り当てられ、
  `UndefinedVar` にならない。**typo した builder 名が compile を通り、実行時まで残る。**
- **根拠**: 2026-08-22 実測。

  ```
  INFER unknown-builder: OK (no error)   # (defn main [] (computation missing (return 42)))
  ```

  期待は `TypeError::UndefinedVar { name: "missing" }` (code `LS1001`) で、
  span は `(computation missing (return 42))` 全体を指すべきである。
- **関連**: 参照実装は `codex/v0.2-ec-m1-02-integration` の `2d116f69` (unknown builder) /
  `5730cfe2` (incomplete builder) / `e8f7ba83` (computation expression の結果型保持)。
  test は同 branch の `crates/lsharp-types/tests/computation_builder_diagnostics.rs`。
  判定は [worktree 取り込み判定](docs/adr/decisions-worktree-absorption-2026-08-20.md)。
- **解決** (2026-08-22): `Expr::Computation` arm
  (`crates/lsharp-types/src/infer/expr.rs:289`) で未登録 builder を `UndefinedVar` に
  したうえで、**同じ arm に重なっていた 2 つの欠陥も併せて閉じた**。判断は
  [computation builder の診断](docs/adr/decisions-computation-builder-diagnostics.md)。

  | # | 欠陥 | 直し方 |
  |---|---|---|
  | 1 | 未登録の builder 名を通す | `get()` が `None` なら `UndefinedVar { name: builder_name }` |
  | 2 | member (`bind_fn` / `return_fn`) の存在を検査しない | use-site の env から引けなければ `UndefinedVar { name: <member 名> }` |
  | 3 | `result_ty == self.var_gen.fresh()` が sentinel として機能しない | `Option<Type>` に置き換え |

  3 は起票時に見えていなかった。`fresh()` は毎回新しい id を作るので比較は**常に false** であり、
  「未登録なら最後のステップの型を返す」という fallback は一度も発火していない。
  `(computation identity (+ 1 2))` の結果型が束縛されない型変数のままになる。
  1 / 2 だけ直しても残るので同じ slice で閉じた。
- **incomplete builder の「別診断」は variant ではなく `name` で分けた**: `TODO.md` の
  受入条件の文言は「別診断で拒否すること」だったが、新 variant + 新 error code は採らなかった。
  builder 宣言が指す `missing-return` は実際にどこにも定義されていないので `UndefinedVar` が
  意味的に正しく、新 code は `error_codes.rs` と error-reference の二重更新を強制する。
  却下理由は ADR に書いた。
- **member 検査を decl-site に置けなかった**: 登録 pass (`infer/decl.rs:95`) は関数の型環境が
  できる前に走るため、builder より後ろに書かれた member が必ず未定義に見える。
  use-site なら `infer_decl_functions` のパス 1 が全 defn を仮登録済みなので前方参照が通る。
  `computation_builder_members_resolve_when_declared_after_use` がこれを固定する。
- **検証**: `crates/lsharp-types/tests/computation_builder_diagnostics.rs` 5 件が
  RED 5/5 → GREEN 5/5。回帰は
  `cargo test --no-fail-fast -p lsharp-types -p lsharp-ir -p lsharp-tooling` で
  960 passed / 1 failed、唯一の FAIL は `workspace-expected-failures.txt:139` に
  既収載の `api_doc::tests::test_build_api_doc_for_file_preserves_parse_error_code`。
  e2e は `core_language_semantics::test_e2e_computation` と `..._let_bang_typecheck` が ok。
  `cargo clippy -p lsharp-types --all-targets` は警告なし。
- **範囲外を 1 件起票した**: 前方参照下の結果型は直っていない (`I-46`)。
  後の実測で **computation builder 固有ではなく plain な `defn` で再現し**、さらに
  **前方参照された呼び出しは引数型も arity も検査されていない**ことが分かった。
  `I-46` は同じ ID のまま範囲を広げてある。selfhost がこの穴に依存している件は `I-48`。

<a id="i-45"></a>
### I-45: selfhost の canonical `:case` preflight が 0 引数 `defn` の呼び出しを型エラーにする

- **影響度**: 中 / **状態**: resolved / **発見**: 2026-08-22 (`WORKTREE-ABSORB-02` の判定中)
- **内容**: `:case [(expect (zero) 1)]` のように **引数を取らない `defn`** を `expect` 内で
  呼ぶと、`cases:0` / `executed:0` のまま `status:"fail"` / exit 1 になる。
  **期待値が正しいか誤っているかに関係なく同じ結果になる。**
- **原因**: evaluator の穴ではなく、**selfhost 型推論の内部不整合**である。

  | 箇所 | 0 引数をどう扱うか |
  |---|---|
  | `selfhost/src/Types/TypeInfer.ls:466-486` `infer-defn-predeclared` | param-count 0 の `defn` を **body の型そのもの** (`zero : Int`) で env へ登録する |
  | `selfhost/src/Types/TypeInferApply.ls:688-716` `infer-apply-legacy-raw` | argc 0 の apply に **`Unit -> a`** を要求する |

  この 2 つが食い違うため unify が落ち、`check-case-expectation`
  (`selfhost/src/Types/TypeInferAssertions.ls:1481-1535`) が `infer-expr` の失敗を
  一律に `canonical-case-type-error-code` = **1001** へ潰す。
  `selfhost/src/App/EmbeddedCli.ls:1065-1078` の preflight
  (`check-canonical-cases-with-analysis`) がこれを見て suite 生成前に短絡するので
  `cases:0` になる。**1001 は `LS1001` (`UndefinedVar`) ではない** — 旧本文の
  「selfhost runner が `LS1001` を出す」という記述は誤りだった。
- **ずれているのは `defn` 側 1 箇所**: 同じ selfhost の `infer-lambda`
  (`TypeInferApply.ls:33-45`) は param-count 0 の lambda を `Unit -> body` にしており、
  Rust 実装も 0 引数 `defn` を `Fun([], Int)` として持つ (`:case [(expect zero 1)]` を
  Rust lane に食わせると `actual=() -> Int, expected=Int` と報告する)。
  apply 側と lambda 側と Rust が一致していて、`infer-defn-predeclared` だけが外れている。
- **根拠**: 2026-08-22 実測。`lsharp test` の既定 (text) は embedded selfhost component へ
  委譲され (`crates/lsharp-driver/src/main.rs:1080`
  `should_delegate_test_to_embedded_component_args`)、`--format json` と
  `LSHARP_DISABLE_EMBEDDED_COMPONENT=1` は Rust 実装を走らせる。両 lane の対比:

  | fixture | selfhost lane | Rust lane |
  |---|---|---|
  | `(defn zero [] 1)` + `:case [(expect (zero) 1)]` | `status=fail cases=0 exec=0 code=1001` / exit 1 | `status=pass cases=1 exec=1` / exit 0 |
  | `(defn incr [x] ...)` + `:case [(expect (incr 1) 2)]` | `status=pass cases=2 exec=2` / exit 0 | `status=pass cases=1 exec=1` / exit 0 |

  **`--format json` を付けると同じソースが緑になる。** つまり Rust 実装は既に正しく、
  収束先は Rust 側である。
- **arity 以外の変数は効かない**: 宣言順を入れ替えても同じ (前方参照固有ではないので
  `I-46` の別の顔ではない)。`(expect 1 (zero))` / `(expect (+ (zero) 0) 1)` も同じ 1001。
  `(expect zero 1)` (呼ばずに参照) は 1001 を出さないので、名前解決は成功している。
- **`lsharp compile` は同じソースを通す**: `compile <file> -o <out>` も selfhost へ委譲される
  (`main.rs:1129` `should_delegate_compile_build_to_embedded_component_args`) が、
  `(zero)` を含むプログラムは compile が成功する。`(+ 1 true)` は
  `[LS1004] [E0004]` で落とすので型検査自体は走っている。
  差は「preflight だけが `infer-program-analysis` の**確定した** env を見る」ことにあり、
  compile 経路は pass-1 の生 placeholder を unify するので矛盾が顕在化しないと見られる
  (`I-46` / `I-48` と同じ穴。`TypeInfer.ls:485`)。**この含意は未証明で、
  本 issue の修正判断には使わない** — preflight が確定 env を見ている事実は
  `(expect zero 1)` の Rust 側報告 `() -> Int` と selfhost 側の挙動で直接示せている。
- **CI を素通りしない**: 失敗側へ倒れる (`status:"fail"` / exit 1) ので、
  誤った contract が緑で通ることはない。**危険なのは逆向き** — 正しい contract が
  永久に赤のままになり、`:example` から `:case` への移行が機械的にはできない。
- **範囲外**: `(module ...)` 本体に置いた場合。main は module 本体へ降りないため
  そもそも検査が走らない (`I-39`。compile 経路の reject は
  [ADR](docs/adr/decisions-module-body-form-rejection.md) で閉じたが、metadata 検査経路は未対応)。
- **解決 (2026-08-22)**: `infer-defn-predeclared` の param-count 0 分岐で
  `(mk-fun (mk-unit) body-ty)` を登録するようにした。判断と却下した選択肢、影響範囲の実測は
  [0 引数 defn の型](docs/adr/decisions-selfhost-zero-arity-defn-type.md)。
  contract は `crates/lsharp-driver/tests/metadata_test_selfhost_case_arity.rs` の 5 test
  (0 引数 actual 側 / expected 側 / 不一致、arity 1 の control 2 件)。
  非 e2e 6 crate は 1592 passed / 15 failed で、失敗 15 件は baseline と完全一致した。
  **stage chain の自己適用 lane (`#[ignore]`) と workspace e2e lane は回していない。**
- **本修正が閉じないもの**: pass-1 の生 placeholder の穴 (`I-46` / `I-48`)。
  前方参照経由の呼び出しは従来どおり検査されない。
- **関連**: `:assert` lane が対照にならない理由は `I-49`。canonical case lane の取り込み判定は
  [worktree 取り込み判定](docs/adr/decisions-worktree-absorption-2026-08-20.md) の群 3。

<a id="i-46"></a>
### I-46: 前方参照された呼び出しは引数型も arity も検査されていない

- **影響度**: 高 / **状態**: open / **発見**: 2026-08-22 (`I-44` の実装中。当初は
  computation builder 固有と書き、次に「呼び出し側の結果型が汎化される」と書いたが、
  実測でどちらも症状の一部でしかないことが分かったので、同じ ID のまま範囲を広げた)
- **内容**: 呼び出し先が呼び出し元より**後ろ**に定義されていると、その呼び出しは
  **一切型検査されない**。引数の型が違っても、引数の**個数**が違っても通る。
  呼び出し元の結果型が `forall a. () -> a` に汎化されるのは、この穴の帰結の一つである。
  computation expression は関与しない。**宣言順だけが違う同じ program が、
  片方は型エラーで落ち、片方は通る。**

  現れ方は 2 つあり、方向が逆なので片方を直しても他方は残る。

  | 現れ方 | 症状 | 危険度 |
  |---|---|---|
  | 健全性 (unsoundness) | 誤用が**通ってしまう** (引数型・arity・呼び出し元の結果型) | 高 |
  | 完全性 (incompleteness) | 正しい program が**誤って落ちる** (前方参照した多相関数を 2 型で使えない) | 中 |

- **根拠 (健全性 / 呼び出しが検査されない)**: 2026-08-22 実測。`Infer::infer_program` の
  戻り値を直接読み、後述の修正パッチを当てた版と並べたもの。

  ```
  --- 修正なし (HEAD) ---
  callsite-forward: Ok ["main:Fun([], Var(26))", "helper:Fun([Con(\"String\")], Con(\"Int\"))"]
  callsite-ordered: Err Mismatch { expected: Con("String"), found: Con("Int"),
                                   span: 83..93, error_code: ArgMismatch }
  arity-forward:    Ok ["main:Fun([], Var(26))", "helper:Fun([Var(28)], Var(28))"]

  --- 修正あり ---
  callsite-forward: Err Mismatch { expected: Con("Int"), found: Con("String"),
                                   span: 59..94, error_code: General }
  callsite-ordered: Err Mismatch { expected: Con("String"), found: Con("Int"),
                                   span: 83..93, error_code: ArgMismatch }
  arity-forward:    Err Mismatch { expected: Fun([Con("Int"), Con("Int")], Var(26)),
                                   found: Fun([Var(28)], Var(28)),
                                   span: 61..80, error_code: General }
  ```

  ```lisp
  ;; callsite-forward — Int を String 引数へ渡しているのに通る
  (defn main [] (helper 1))
  (defn helper [x] (string-length x))

  ;; arity-forward — 1 引数の関数を 2 引数で呼んでいるのに通る
  (defn main [] (helper 1 2))
  (defn helper [x] x)
  ```

  **arity 不一致が codegen まで無検査で届く。** 呼び出し順を入れ替えれば
  `callsite-ordered` のとおり `ArgMismatch` で落ちるので、検査経路そのものは存在する。
  前方参照の場合だけそこに到達しない。

- **根拠 (健全性 / 結果型の汎化)**: 同日実測。上の穴の帰結として、
  caller の結果型が束縛されないまま量化される。

  ```
  plain-forward: Ok ["main:Fun([], Var(27))", "helper:Fun([Var(29)], Var(29))",
                     "misuse:Fun([], Con(\"Int\"))"]
  plain-ordered: Err Mismatch { expected: Con("String"), found: Con("Int"),
                                span: 62..84, error_code: ArgMismatch }
  fwd-noargs:    Ok ["main:Fun([], Var(27))", "helper:Fun([], Con(\"Int\"))",
                     "misuse:Fun([], Con(\"Int\"))"]
  ```

  ```lisp
  ;; plain-forward — 通ってしまう
  (defn main [] (helper 1))
  (defn helper [x] x)
  (defn misuse [] (string-length (main)))   ; Int を String として使えてしまう

  ;; plain-ordered — helper を前に出しただけで Mismatch になる
  (defn helper [x] x)
  (defn main [] (helper 1))
  (defn misuse [] (string-length (main)))
  ```

  `fwd-noargs` が示すのは、**callee 自身は正しく解決される** (`helper:Fun([], Con("Int"))`)
  のに caller だけが `Var(27)` に取り残されるということである。caller は callee の型が
  確定する前に generalize されている。

- **根拠 (完全性)**: 同日実測。前方参照した多相関数を 2 つの型で使うと、
  最初の使用が placeholder を単相化してしまい、2 番目が落ちる。

  ```
  poly-forward: Err Mismatch { expected: Con("Int"), found: Con("String"),
                               span: 41..49, error_code: ArgMismatch }
  poly-ordered: Ok ["id:Fun([Var(27)], Var(27))", "f:Fun([], Con(\"Int\"))",
                    "g:Fun([], Con(\"String\"))"]
  ```

  ```lisp
  (defn f [] (id 1))
  (defn g [] (id "s"))   ; ← ここが落ちる。id を前に出せば通る
  (defn id [x] x)
  ```

- **原因**: `infer_decl_functions` のパス 1 は全 defn を placeholder 型変数で仮登録し、
  パス 2 が宣言順に本推論して**その場で generalize する** (`infer/decl.rs:298-333`)。
  前方参照された呼び出しは `env` の placeholder を `Fun([Int], r)` の形へ束縛するが、
  パス 2 は callee 本体を推論したあと**生の `placeholder_ty` の方**を unify 相手に使う
  (`decl.rs:317` の `placeholder_ty.apply_subst(&subst)`)。呼び出し側が作った形は
  `env` 側にしか無いので捨てられ、**呼び出し側の要求と callee の実型は一度も突き合わされない**。
  さらに `generalize` は `env_for_gen` から未推論の pending 名を除くため
  (`decl.rs:325-328`)、`r` は「env に現れない自由変数」と判定されて量化される。
- **事後 pass 仮説は実測で否定された**: 当初は「束縛は `self.global_subst`
  (`infer/unify.rs:115`) に累積されているので、ループ後に各 scheme へ最終代入を適用すれば
  閉じられる」と書いた。**これは成立しない。** `global_subst` は `compose` ではなく生の
  `insert` で積むため同じ変数の再束縛で上書きされ、さらに誤用の検査 (`misuse`) は
  ループの**途中**で、既に量化済みの scheme に対して走るので、事後 pass では届かない。
  実際に閉じたのは下記のパッチ (unify 相手を `env` 側の登録型に変える + pending 名の
  除外を「裸の型変数のときだけ」に絞る) の 2 箇所の同時変更で、
  **片方だけでは 3 本の RED のうち 3 本とも RED のまま**である (2026-08-22 bisect)。

  ```diff
  --- a/crates/lsharp-types/src/infer/decl.rs
  +++ b/crates/lsharp-types/src/infer/decl.rs
  -            let resolved_placeholder = placeholder_ty.apply_subst(&subst);
  +            let registered_ty = env
  +                .get(&qualified_name)
  +                .filter(|scheme| scheme.vars.is_empty())
  +                .map(|scheme| scheme.ty.clone())
  +                .unwrap_or_else(|| placeholder_ty.clone());
  +            let resolved_placeholder = registered_ty.apply_subst(&subst);
   @@
               for pending_name in pending_names.iter().skip(index) {
  -                env_for_gen.remove(pending_name);
  +                let is_bare_placeholder = matches!(
  +                    env_for_gen.get(pending_name).map(|scheme| &scheme.ty),
  +                    None | Some(Type::Var(_))
  +                );
  +                if is_bare_placeholder {
  +                    env_for_gen.remove(pending_name);
  +                }
               }
  ```

  完全性側はこれでは閉じない。宣言順に依存しない generalize (依存グラフの SCC 順) が要る。
- **修正を当てられない理由**: 上のパッチは新規 test 6 件を GREEN にするが、
  **selfhost のソースが同じ穴に依存している**ため当てられない。2026-08-22 実測で、
  推論に失敗する selfhost の defn が **0 件 → 262 件**へ増える
  (`Mismatch` 177 / `UndefinedVar` 89、error code は `ArgMismatch` 170 / `IfBranch` 6 /
  `General` 1。`UndefinedVar` の一部は計測 harness が失敗 defn を `continue` で
  読み飛ばす副作用の可能性があり、内訳より桁が信号である)。詳細は `I-48`。
- **selfhost 側の checker も同じ形である**: `selfhost/src/Types/TypeInfer.ls:485` は
  `(unify placeholder body-ty annotated-subst)` と生の placeholder を使い、
  `:912` の `typeinfer-pending-env-vars-loop` も `placeholders` map の生の値を読む。
  **Rust 側だけを直すと 2 つの checker が食い違う。** 修正は両側同時に要る。
- **発見経路**: computation builder の member を前方参照した fixture
  (`(computation-builder identity identity-bind identity-return)` の member を
  `(defn main ...)` より後ろに置く) で最初に観測した。computation expression 固有ではないので、
  そちらは instance として扱う。
- **`I-44` の修正で入った欠陥ではない**: 修正前の RED でも同じ `Fun([], Var(28))` が出ていた。
  `I-44` は「誤って incomplete 扱いにしないこと」までを閉じ、結果型は範囲外として残した。
- **契約の保存先**: `crates/lsharp-types/tests/forward_reference_generalization.rs`。
  検出側 5 件 (引数型 / arity / 結果型の汎化 / 0 引数 callee / computation builder) は
  `#[ignore]` で残し、退行防止側 3 件 (宣言順が正しい場合 / 相互再帰 / 正しい前方参照) は
  live で回る。`#[ignore]` lane の台帳
  (`docs/development/validation/ignored-lane-expected-failures.txt`) は `lsharp-wasm` の
  stage chain e2e だけを測っているので、この 5 件はどちらの baseline にも現れない。
- **関連**: `INFER-FORWARD-GEN-01` (健全性) と `INFER-FORWARD-POLY-01` (完全性) が
  `TODO.md` で引き取る。selfhost 側の依存は `I-48`。発見経路の判断は
  [computation builder の診断](docs/adr/decisions-computation-builder-diagnostics.md)。

<a id="i-47"></a>
### I-47: `cargo fmt --check` が 5 crate で落ちる (`I-34` は `lsharp-ir` しか見ていなかった)

- **影響度**: 低 / **状態**: open / **発見**: 2026-08-22 (`I-44` の commit 前検査中)
- **内容**: `I-34` は `cargo fmt --check -p lsharp-ir` を解消して resolved にしたが、
  **workspace の他 crate は測っていなかった**。実際には 5 crate が落ちる。
- **根拠**: 2026-08-22 実測。`cargo fmt --check -p <crate>` の `Diff in` 行数。

  | crate | `Diff in` |
  |---|---|
  | `lsharp-wasm` | 346 |
  | `lsharp-types` | 66 |
  | `lsharp-driver` | 27 |
  | `lsharp-syntax` | 9 |
  | `lsharp-tooling` | 4 |
  | `lsharp-ir` / `lsharp-lsp` / `lsharp-docs` | 0 |

- **実害が出た経路**: `I-44` の作業中に `cargo fmt -p lsharp-types` を走らせたところ、
  意図した 1 file に加えて**無関係な 20 file**が書き換わり、HEAD の内容へ戻す手間が発生した。
  **crate 単位の `cargo fmt` が commit の粒度を壊す。**
- **`I-34` を再 open にしない理由**: `I-34` の主張 (`lsharp-ir` の `mod` 宣言順) は
  実際に解消済みで、その範囲では正しい。誤っていたのは**範囲の取り方**であり、
  「1 crate で緑になったことを workspace の状態として書いた」ことである。
  別 ID で起票して範囲を明示する。
- **CI の扱いは決めていない**: `I-31` / `I-34` と同じく、gate を緑にすることと
  CI で強制することは別の判断である。
- **解決 (2026-08-22)**: `cargo fmt --all` を適用し、crate ごとに 5 commit へ分けた
  (`e38726d1` / `f78b45ea` / `fb0e33d2` / `f3d9bd52` / `28a62f46`)。
  59 file / 452 hunk。`rustfmt 1.8.0-stable`、`rustfmt.toml` は追加していない
  (Cargo.toml の edition 2024 が決める既定 style_edition で緑になる)。
  適用後 `cargo fmt --all -- --check` は exit 0。
- **満たせなかった受入条件**: `FMT-WORKSPACE-01` は「適用後に
  `cargo test --no-fail-fast --workspace` の FAIL 集合が増えないこと」を求めていたが、
  **`lsharp-wasm` の e2e lane 全体は回していない** (`I-11` の実測で 5 時間超)。
  実際に回したのは `cargo build --workspace --tests` (エラーなし) と、
  `lsharp-types` / `lsharp-ir` / `lsharp-syntax` / `lsharp-tooling` / `lsharp-docs` /
  `lsharp-driver` / `lsharp-lsp` の全 test、および e2e のうち
  **ソース本文へ文字列 assertion する gate** (`file_size` 4 件 / `bounded_fragments` /
  `ops03` 5 件 / `support::` 7 件) である。rustfmt は文字列リテラル内部を書き換えないが、
  行数と `fn` の行頭位置を見る gate は存在するので、そこだけは直接確認した。
  FAIL は 15 件でいずれも `workspace-expected-failures.txt` に既収載
  (driver 11 + 1 / lsp 1 / syntax 1 / tooling 1)。**新規 FAIL 0 / 解消 0。**
- **関連**: `I-34` / `I-31`。

<a id="i-48"></a>
### I-48: selfhost のソースが `I-46` の穴に依存しており、vector をタプルとして使っている

- **影響度**: 高 / **状態**: open / **発見**: 2026-08-22 (`I-46` の修正を当てた際の実測)
- **内容**: selfhost は複数の値をまとめて返すのに **`Vector` を異種タプルとして**使う。
  `push-int-vector` と `push-object-vector` はどちらも `forall a. Vector a -> a -> Vector a`
  なので、`(Int, Int, Map, Int)` のような組を作ると同じ `a` に `Int` と `Map` が来る。
  **HM では型が付かない。** 現在 selfhost が型検査を通っているのは、
  これらの構築関数が呼び出し元より後ろに定義されていて `I-46` の穴で検査されないからである。
  つまり `I-46` を直すと selfhost が自分自身をコンパイルできなくなる。
- **根拠**: 2026-08-22 実測。`I-46` の修正パッチを当て、selfhost 全ソースを推論して
  失敗した defn を一意名で数えた。

  | | 修正なし (HEAD) | 修正あり |
  |---|---|---|
  | 推論に失敗する selfhost の defn (一意名) | **0** | **262** |

  内訳は `Mismatch` 177 / `UndefinedVar` 89、error code は `ArgMismatch` 170 /
  `IfBranch` 6 / `General` 1。`UndefinedVar` の一部は計測 harness が失敗した defn を
  `continue` で読み飛ばす副作用の可能性があり、**内訳ではなく桁が信号**である。

  代表例 2 件。どちらも異種 vector を組み立てている。

  ```lisp
  ;; selfhost/src/Backend/Wasm/CompilerBase.ls:498 — (Int, Int, Map, Int) を作る
  (defn make-bind-node-params-state [done next-param-idx next-env next-local-idx]
    ... (push-int-vector (push-object-vector (push-int-vector
          (push-int-vector (vector-new 4) done) next-param-idx) next-env) next-local-idx) ...)
  ;; → Mismatch { expected: Con("Map"), found: Con("Vector"), span: 17904..17964 }

  ;; selfhost/src/Syntax/Parser.ls:1142 — (String, String, Vector, String, Int, Vector) を作る
  (defn make-empty-defn-metadata-v3 []
    ... (vector-push-quad-rooted-v3 (vector-new 4) "" "" params0 "")
        (vector-push-single-rooted-v3 meta5 0) ...)
  ;; → Mismatch { expected: Con("Int"), found: Con("String"), span: 37534..37594 }
  ```

- **観測できる実害**: `lsharp-ir` の
  `incremental_analysis_tests::test_compile_multi_file_incremental_clean_formatter_trio_cache_hit_succeeds`
  が修正パッチ下で
  `selfhost/src/Backend/Wasm/CompilerBase.ls: [LS1004] [E0004] 型の不一致: expected Map, found Vector`
  で落ちる。HEAD では通る。
- **なぜ `I-46` と別 ID にするか**: `I-46` は Rust 側 checker の欠陥で、修正は
  `infer/decl.rs` の 2 箇所である。本件は **selfhost のデータ表現**の問題で、
  修正はレコード型への移行 (L# はレコードを持つ) と、selfhost 自身の `TypeInfer.ls` を
  Rust 側と同時に直すことになる。成果物も修正経路も別なので ID を分ける。
  `I-46` の修正は本件に **blocked** される。
- **一時回避で済ませない理由**: 「異種 vector を許す型付け」を入れると
  `Vector` の要素型がどこでも `Any` に潰れ、`I-46` を直した意味が無くなる。
  レコードへの移行が本筋である。
- **関連**: `I-46` (checker 側の穴)。`TODO.md` の `INFER-FORWARD-GEN-01` が
  `[BLOCKED: I-48]` で待つ。selfhost の分割は `TYPEINFER-SPLIT-01`。

<a id="i-49"></a>
### I-49: selfhost の `:assert` lane は predicate を型検査していない

- **影響度**: 中 / **状態**: resolved / **発見**: 2026-08-22 (`I-45` の対照実験中)
- **内容**: canonical `:assert` の predicate を selfhost lane に食わせると、
  **未定義の変数を呼んでいても型診断が 1 件も出ない**。runtime 評価まで進んで
  「述語が偽」として落ちるだけなので、診断としては型エラーと区別できない。
- **根拠**: 2026-08-22 実測。fixture は `(defn caller [] :assert [(> (nope) 0)] 0)` で
  `nope` はどこにも定義されていない。

  | lane | 結果 |
  |---|---|
  | selfhost (既定 text) | `status=fail cases=1 executed=1 failed=1 diagnostics.count=0` |
  | Rust (`LSHARP_DISABLE_EMBEDDED_COMPONENT=1`) | `[LS1001] [error] caller: :assert predicate の型推論に失敗しました: [E0001] 未定義の変数 (undefined): nope (29..33)` |

  `EmbeddedCli.ls:1065-1078` の preflight は `check-canonical-cases-with-analysis` を
  `:case` にしか適用しない。`check-canonical-assertions-with-analysis`
  (`TypeInferAssertions.ls:2183`) は存在するが、この経路からは呼ばれていない。
- **`I-45` への含意**: 旧 `I-45` 本文は「同じ 0 引数呼び出しを `:assert` は解決できる」ことを
  `:case` 固有性の根拠にしていたが、**`:assert` は解決しているのではなく検査していない**。
  対照群として無効である。`I-45` の根拠表からこの行を落とした。
- **なぜ危険か**: `:case` (`I-45`) は正しい contract を赤にする「安全側の壊れ方」だが、
  こちらは**誤った contract の型エラーを緑で通し得る**向きである。predicate が
  たまたま真になれば pass する。
- **解決** (2026-08-22): `run-test-source-json` / `run-test-source-text` の preflight へ
  `check-canonical-assertions-with-analysis` を接続した (`EmbeddedCli.ls` / `Cli.ls` の両方)。
  優先順位は `check` lane と同じ base → assertion → case → property。
  同じ fixture で `diagnostics.count=1` / `firstErrorCode=1001` / `executed=0` / rc=1 となり、
  Rust oracle と同じ向きへ揃った。健全な predicate は `executed=1` / rc=0 のまま。
  **欠けていたのは実装ではなく接続**で、検査関数自体は `check` lane が既に使っていた。
  判断と却下理由は [decisions-selfhost-assert-preflight-typecheck.md](docs/adr/decisions-selfhost-assert-preflight-typecheck.md)。
- **残る差分**: 診断 `message` は空文字列のまま。Rust oracle は
  `[LS1001] [error] caller: :assert predicate の型推論に失敗しました: ...` を返す。
  selfhost の preflight は case / property でも message を空で返す設計なので、
  これは assert 固有ではない。`ASSERT-DIAG-MESSAGE-01` (`TODO.md`) が引き取る。
- **関連**: `I-45` と同じ preflight の話。

<a id="i-50"></a>
### I-50: `lsharp compile` の入力ソース整形上書きが利用者へ通知されない

- **影響度**: 中 / **状態**: resolved / **発見**: 2026-08-22 (`CASE-ZERO-ARITY-01` の影響範囲計測中)
  / **訂正**: 2026-08-22 (下記「起票時の記述の訂正」)
- **内容**: `lsharp compile` の **Rust host 経路**は、コンパイル前に entry file を formatter に
  かけ、差分があれば**入力ファイルへ書き戻す**。この書き戻し自体は
  `prepare_source_for_compile` (`crates/lsharp-tooling/src/compile.rs:222-234`) の仕様であり、
  契約テスト `test_prepare_source_for_compile_rewrites_file_when_format_diff_exists`
  (`crates/lsharp-tooling/src/compile_tests_outputs.rs:172`) が固定している。
  **live な欠陥は書き戻しそのものではなく、それが利用者へ一切通知されない点**である。
  `CompileArtifacts` は結果を `pub formatted: bool` (`compile.rs:30`) で運んでいるが、
  `crates/lsharp-driver/src` 全域を grep しても **consumer が 0 件**で、
  stdout にも stderr にも現れない。利用者から見るとコンパイラが黙ってソースを書き換える。
  実際、本 slice の計測中に `selfhost/src/App/EmbeddedCli.ls` と
  `selfhost/src/App/Cli.ls` が副作用として書き換わった (どちらも復元済み)。
- **根拠**: 2026-08-22 実測。最小再現:

  ```bash
  printf '(defn f [x]\n  (let [a\n          (+ x 1)]\n    a))\n(defn main [] (f 1))\n' > input.ls
  md5 -q input.ls                       # d8158dd08c7169e4436faea89bea1722
  lsharp compile input.ls -o out.wasm   # stdout に整形の言及は無い
  md5 -q input.ls                       # 9b18b91c5af42308c0ae8b44a6c166bb
  ```

  差分は `let` の束縛値のインデント (10 空白 → 4 空白) で、内容としては formatter 出力。

- **経路別の切り分け** (同一 fixture、2026-08-22 再実測)。
  **stdout でどちらの lane を踏んだか判別する** — `wasm-size:` なら guest が完遂、
  `コンパイル成功: ...` (`crates/lsharp-driver/src/main.rs:679`、Rust にしか存在しない文字列)
  なら host が実行している:

  | 実行 | 実際に走った lane | 入力ファイル |
  |---|---|---|
  | `compile input.ls -o out.wasm` (既定 = `wasi-component`) | host (guest が拒否し fallback) | **書き換わる** |
  | `compile input.ls -o out.wasm` (`LSHARP_DISABLE_EMBEDDED_COMPONENT=1`) | host | **書き換わる** |
  | `compile input.ls --emit-ir` | host | **書き換わる** |
  | `compile input.ls --target wasi-preview1 -o out.wasm` | **guest が完遂** (`wasm-size:`) | 変わらない |
  | `check` / `test` / `fmt` (引数なし) | -- | 変わらない |

  既定の `compile -o` が書き換わるのは guest が書いているからではなく、
  guest が `wasi-component` を無条件に拒否して host compile へ落ちるためである
  (`I-15` に設計として記録済み、`main.rs:900-928`)。
  **guest が実際に compile を完遂する唯一のセル (`--target wasi-preview1`) では入力は不変**なので、
  書き戻しは Rust host 経路にしか存在しない。ただし既定コマンドが常に host へ落ちる以上、
  実利用ではほぼ常に書き換わる。
- **起票時の記述の訂正** (2026-08-22): 初版の切り分け表は
  `LSHARP_DISABLE_EMBEDDED_COMPONENT=1` と `--emit-ir` を「変わらない」と記録し、
  そこから「書き換えているのは embedded selfhost component 側」と結論していた。
  **どちらも誤り**である。原因は計測手順で、セル間で fixture を復元しておらず、
  先行セルが既に整形済みにしたファイルを後続セルが「変わらない」と読んでいた。
  同じ初版が置いた「**未特定**: 書き込んでいる正確な箇所」も解消済みで、
  上記の `compile.rs:222-234` が唯一の書き込み箇所 (production の呼び出しは
  `compile.rs:368` の 1 箇所、対象は entry file のみで import 先 module は通らない)。
- **既に記録されていた**: この副作用と「host / guest 双方に及ぶ」という判定は
  [`decisions-dev-loop-rust-lane-speedup.md`](docs/adr/decisions-dev-loop-rust-lane-speedup.md)
  の 106-109 行と
  [`rust-boundary-reduction.md`](docs/development/operations/rust-boundary-reduction.md)
  の 4999-5018 行に先行して存在した (`scripts/dev-loop.sh` が entry を退避・復元して
  回避している)。本 issue はそれを未知の挙動として再発見したものではない。
  なお ops 記録の「両方」は上表の再測により **host のみ**へ訂正される。
- **書き戻しの二次被害**: 入力が read-only (`chmod 444`) の場合、compile は
  `[LS5001] <path>: Permission denied (os error 13)` で **rc=1 になる** (host / guest 双方の
  呼び出しで再現。guest 呼び出しは host へ落ちるため)。型検査以前に書き込みで落ちるので、
  診断が出ない。また書き込みは compile の**前**に走るため、整形差分があって型エラーもある
  ソースは「書き換えられたうえで失敗する」。
- **なぜ気付かれていなかったか**: 整形結果が入力と同一なら差分が出ない。
  selfhost 自身のソースは formatter の現行出力と一致していないため露見した。
- **解決** (2026-08-22): driver 側の成功出力 choke point
  `print_compile_artifacts_success` (`crates/lsharp-driver/src/main.rs:674`) に
  `artifacts.formatted` ガード付きの stderr 通知を 1 行足した。
  compile の成功出力はこの関数が唯一の経路で、呼び出し元 2 箇所
  (`main.rs:419` の直接 host、`main.rs:930` の guest 拒否 fallback) を 1 箇所で覆える。
  書き込み元である `prepare_source_for_compile` (library) 側には置かない
  — library が利用者向け出力を持つと LSP / MCP など stderr を持たない consumer に漏れる。
  通知は stderr なので、`compile` の stdout 契約 (`コンパイル成功: ...`) は不変。
- **検証**:

  | test | 位置 | RED | GREEN |
  |---|---|---|---|
  | `test_compile_reports_source_rewrite_when_format_diff_exists` | `lsharp-driver/tests/default_path_delegation.rs` | 通知 0 行で FAIL (`:2826`) | pass |
  | `test_compile_stays_silent_when_source_is_already_formatted` | 同上 | (対照 fixture。修正前から pass) | pass |

  両 test は `LSHARP_DISABLE_EMBEDDED_COMPONENT=1` で host lane を固定する。
  既定 lane も結局 host へ落ちるが (上表)、fallback 判定に依存させると環境差で false GREEN になる。
  沈黙側 fixture は「整形差分が無いこと」をファイル本文の不変 assertion で自ら pin する
  — pin しないと、実は差分があって通知を見落としているだけの空虚な test になる。
- **残渣 (この slice で直していない)**:
  - **失敗経路は依然として無言**。整形差分があって型エラーもあるソースは
    「書き換えられたうえで失敗する」が、driver は失敗時に `CompileArtifacts` を受け取らないため
    通知経路が無い。覆うには library 側で出力するか、エラー型に整形結果を載せる必要があり、
    どちらもこの slice の受入条件 (成功経路の 1 行) を越える設計判断を含む。
  - **`--emit-ir` も無言**。`main.rs:418` の `if !emit_ir` で成功出力ごと抑止されるため、
    整形して書き戻しても通知は出ない。受入条件が `compile -o` に限定されているため意図的に範囲外。
- **関連**: formatter 出力そのものの正しさは `FMT-ROUNDTRIP-01`。fallback の設計は `I-15`。

<a id="i-51"></a>
### I-51: `compile -o <ディレクトリ成分の無いファイル名>` が artifact 同期で落ちる

- **影響度**: 低 / **状態**: resolved / **発見**: 2026-08-22 (`I-50` の経路別切り分け中)
- **内容**: `-o` にディレクトリ成分を持たないファイル名 (`out.wasm`) を渡すと、
  component adapter が親ディレクトリを空文字列として開こうとして失敗する。
  `-o ./out.wasm` や絶対パスでは起きない。
- **根拠**: 2026-08-22 実測。

  ```
  $ LSHARP_DISABLE_EMBEDDED_COMPONENT=1 lsharp compile zz_probe.ls -o zz_out.wasm
  Error: × [LS5001] zz_out.wasm: component adapter エラー:
    artifact parent directory の同期対象を開けません (): No such file or directory (os error 2)
  ```

  エラーメッセージ中の `()` が空のパスで、`Path::parent()` が `""` を返すケースを
  そのまま open している形。`I-50` の書き戻しはこの失敗の**前**に済んでいるため、
  入力は書き換わったうえで rc=1 になる。
- **再現条件は host lane に限られる**: 既定の embedded component lane は WASI guest 側が
  出力を書くのでこの経路を通らず、bare `-o` でも成功する。落ちるのは
  `LSHARP_DISABLE_EMBEDDED_COMPONENT=1` (Rust host codegen) と native lane
  (`lsharp-tooling/src/native.rs:68`) の 2 つ。**artifact 自体は書けており
  (rename まで成功)、rc だけが 1 になる**ため「出力はあるのに失敗した」形に見える。
- **解決** (2026-08-22): `component_adapter::artifact_parent_dir()` を追加し、
  `None` と空文字列の両方を `.` へ正規化する 1 箇所へ集約した。
  `sync_artifact_parent()` (`component_adapter.rs:280`) と
  `write_wasm_artifact()` (`:314`) の 2 箇所が同じ直書きを持っていたので、
  個別に直さず choke point 経由へ寄せた。同型の正しい実装は
  `lsharp-driver/src/atomic_write.rs:14-16` に既にあり、そちらへ形を合わせている。

  検証 (2026-08-22 実測):

  | test | 位置 | RED | GREEN |
  |---|---|---|---|
  | `test_sync_artifact_parent_accepts_bare_file_name_without_directory_component` | `component_adapter_tests.rs` | 本文と同一メッセージで FAIL | pass |
  | `test_artifact_parent_dir_normalizes_bare_and_dotted_and_absolute_forms` | 同上 | (GREEN 期で追加) | pass |
  | `test_compile_output_path_accepts_bare_dotted_and_absolute_forms_identically` | `lsharp-driver/tests/default_path_delegation.rs` | 本文と同一メッセージで FAIL | pass |

  driver test は host lane を `LSHARP_DISABLE_EMBEDDED_COMPONENT=1` で強制する。
  **既定 lane のままでは fix 前から pass する偽 RED になる**ので、lane の固定が受入の前提である。
  なお `default_path_delegation` の embedded lane 系 11 件は
  `workspace-expected-failures.txt:120-131` にある恒常 FAIL で、本 test はそれとは別 lane。
- **直していない同型**: `lsharp-driver/src/atomic_write.rs:70` の `sync_path()` が symlink 分岐で
  同じ直書きを持つ。唯一の呼び出し元 `main.rs:2506` の `sync_install_path()` は
  package root 配下の path しか渡さないので**実行経路上の失敗にならない**。
  test を書くには process 全体の cwd を書き換える必要があり並列実行と両立しないため、
  無検証の修正は避けて記録に留める。`lsharp-tooling/src/native.rs:290` も直書きだが、
  空 parent の `join()` は相対 temp path として機能するので失敗経路ではない。
- **関連**: `I-50` (整形書き戻しの通知 -- resolved。失敗経路の無言は残渣として同 issue に記録)。

<a id="i-52"></a>
### I-52: LSP stdio 補完の e2e が 2 系統の理由で全滅している (位置規約の食い違い / snapshot 形式のドリフト)

- **影響度**: 中 / **状態**: open / **発見**: 2026-08-22 (`I-49` の slice を閉じる際の sweep)
- **範囲 (実測)**: `cargo test -p lsharp-wasm --test e2e -- --ignored lsp_stdio_completion`
  で 10 本が走り、**1 passed / 9 failed** (`finished in 839.52s`, 2026-08-22)。
  緑なのは `..._lsp_stdio_completion` のみ。失敗 9 本は **signature が 2 系統**に分かれ、
  片方だけを直しても残りは緑にならない。

  | test | A: 位置規約 | B: snapshot 形式 |
  |---|---|---|
  | `..._completion_uses_open_document` | ○ | -- |
  | `..._completion_uses_open_document_spec_params` | ○ | -- |
  | `..._completion_uses_changed_document` | ○ | -- |
  | `..._completion_latest_reopened_schema_snapshot` | ○ | ○ |
  | `..._completion_changed_document_schema_snapshot` | ○ | ○ |
  | `..._completion_filesystem_import_schema_snapshot` | ○ | ○ |
  | `..._completion_uses_spec_changed_document_with_escaped_newline` | ○ | ○ |
  | `..._completion_uses_spec_changed_document_with_unicode_escaped_newline` | ○ | ○ |
  | `..._completion_schema_snapshot` | -- | ○ |

  最後の 1 本は request が `"params":0` で **位置を一切送らない**ため、A では説明できない。
  「同じ family が同じ理由で落ちている」と丸めると、この 1 本の原因が消える。

#### A: wire の位置正規化と fixture の col 規約が食い違う

- **内容**: `lsharp lsp` の stdio 経路は wire 上の `line` / `col` を **0-indexed (LSP 3.17 準拠)**
  として受け取り、内部の 1-indexed 規約へ `+1` して渡す。一方 fixture は **1-indexed のまま**で
  書かれており、実効カーソル位置が 1 文字後ろへずれる。ずれた先が `)` などの非シンボル文字だと
  `lsp-prefix-at` が `""` を返し、`lsp-prefix-matches` は空 prefix を**全一致として扱う**ため、
  prefix 絞り込みが丸ごと無効になる。補完結果に L# キーワード 7 件が常に混ざる。
- **根拠**: `..._completion_uses_changed_document` の実測。

  ```
  left  (実測): Content-Length: 407 ... "result":[{"label":"helper",...},{"label":"defn",...},
                {"label":"let",...},{"label":"if",...},{"label":"match",...},
                {"label":"do",...},{"label":"fn",...},{"label":"module",...}]
  right (期待): Content-Length:  86 ... "result":[{"label":"helper","kind":3,"insertText":"helper"}]
  ```

  **item の形は左右で一致している**。差は「余計な keyword 7 件が付く」ことだけで、
  これが A の signature である。

  索引の突き合わせ — `..._repeated_didopen_keeps_latest_source` (`selfhost_cli_core.rs:17665`) の
  latest source は `"(defn beta [] 1) (be)"` (21 byte)、要求は `"col":21`:

  | 経路 | 内部 col | offset | `idx = offset - 1` | 文字 | prefix |
  |---|---|---|---|---|---|
  | 正規化前 (〜2026-08-02) | 21 | 20 | 19 | `e` | `"be"` |
  | 正規化後 (2026-08-03〜) | 22 | 21 (= 文字列長) | 20 | `)` | `""` |

  該当実装は `selfhost/src/App/Cli.ls:2010-2020` (`lsp-stdio-nav-params` の `+1`)、
  `selfhost/src/Tools/Lsp/LspServerNav.ls:275-276` (`lsp-offset-from-line-col`、`line`/`col` とも 1 始まり)、
  `:686-694` (`lsp-prefix-at` / `lsp-prefix-matches`)。
- **いつ壊れたか**: `+1` 正規化は `9175c6e5` 「fix: normalize native lsp wire positions」(2026-08-03)。
  fixture 側は `d32f9e91` (2026-04-01) 由来で、正規化に追随していない。
- **解決** (2026-08-22): **実装ではなく fixture を直した。** `9175c6e5` の `+1` 正規化が
  LSP 3.17 準拠であり、1-indexed のまま止まっていた fixture の側が誤りだったため。
  `selfhost_cli_core.rs` の 4 行を wire 規約 (0-indexed) へ書き換えた。

  | 行 | test | 旧 | 新 |
  |---|---|---|---|
  | `:17075` | `..._completion_uses_open_document` | `"line":1,"col":23` | `"line":0,"col":22` |
  | `:17118` | `..._completion_uses_open_document_spec_params` | `"line":1,"character":23` | `"line":0,"character":22` |
  | `:17445` | `..._completion_uses_changed_document` | `"line":1,"col":23` | `"line":0,"col":22` |
  | `:17678` | `..._repeated_didopen_keeps_latest_source` | `"line":1,"col":21` | `"line":0,"col":20` |

  値の求め方は「シンボル末尾の次の 0-based offset」。`(defn helper [] 1) (he)` は 23 byte で
  `he` が 0-based 20..21 なので 22、`(defn beta [] 1) (be)` は 21 byte で `be` が 18..19 なので 20。
  **`(b) 空 prefix を「補完しない」へ変える` は採らなかった** — params 無しの既定 keyword 補完
  (`LspServerNav.ls:1105-1113`) と衝突し、実装側の契約を変えることになる。fixture の誤りを
  実装の変更で覆うと、規約の正本がどちらなのかが永久に決まらない。
- **検証** (`cargo test -p lsharp-wasm --test e2e -- --ignored --test-threads=1
  lsp_stdio_completion_uses lsp_stdio_repeated_didopen`、2026-08-22):

  | | RED | GREEN |
  |---|---|---|
  | 結果 | 0 passed / 6 failed (`1217.42s`) | **4 passed / 2 failed** (`1234.19s`) |
  | 受入 4 本 | 全滅 (keyword 7 件混入、`Content-Length` 86 → 407 / 403) | 全緑 |
  | 残り 2 本 | FAILED | FAILED (理由は下記) |

  緑になったのは `..._completion_uses_open_document` / `..._completion_uses_open_document_spec_params` /
  `..._completion_uses_changed_document` / `..._repeated_didopen_keeps_latest_source` の 4 本。
- **規約の正本**: `AGENTS.md` の「LSP stdio wire の位置規約 (fixture の正本)」節
  (「テスト構成」直後) に 1 箇所だけ置いた。`docs/language/` には LSP を扱うファイルが無く、
  新規ファイルを孤立させるより既存の作業手順正本へ寄せる方が発見可能性が高い。
- **残渣の内訳**: 位置を 1-indexed のまま残していた fixture は 5 本あった。
  うち **inline 期待値の 2 本は同日の第 2 slice で解決** (下記)、
  snapshot file を読む 3 本は facet B の解決待ちである。

  | 行 | test | 旧 | 新 | 状態 |
  |---|---|---|---|---|
  | `:17533` | `..._uses_spec_changed_document_with_escaped_newline` | `"line": 2, "character": 4` | `"line": 1, "character": 3` | 適用済・検証済 |
  | `:17614` | 同 `_unicode_` 版 | `"line": 2, "character": 4` | `"line": 1, "character": 3` | 適用済・検証済 |
  | `:18966` | `..._completion_changed_document_schema_snapshot` | `"line":1,"col":23` | `"line":0,"col":22` | 導出済 (未検証) |
  | `:19232` | `..._completion_latest_reopened_schema_snapshot` | `"line":1,"col":21` | `"line":0,"col":20` | 導出済 (未検証) |
  | `:19490` | `..._completion_filesystem_import_schema_snapshot` | `find(..) + len + 1`、`"line":1` | `find(..) + len`、`"line":0` | 導出済 (未検証) |

  下 3 本の値は同じ導出規則で求めただけで、GREEN で検証したのは別の 6 本である。
  引き取り先は `TODO.md` の `LSP-COL-CONV-03`。
- **inline 期待値 2 本の解決** (2026-08-22、第 2 slice): この 2 本は facet A と B の**両方**を
  踏むが、期待値が snapshot file ではなく **inline の `serde_json::json!`**
  (`selfhost_cli_core.rs:17573` / `:17654`) で `assert_lsp_stdio_snapshot` を一切経由しない。
  よって facet B の設計決定 (縮約器を入れるか否か) を待たずに単独で緑にできる。

  - 位置: 上表の `:17533` / `:17614` を wire 規約へ
  - 期待値: `"result": [["helper", 3, "helper"]]` →
    `"result": [{"label": "helper", "kind": 3, "insertText": "helper"}]`

  期待値の書き換えを正当化する根拠は facet A と同じで、object 形を契約にしたのは
  `5db1c2a4` (2026-08-03) であり、三要素配列で書かれた test 側が陳腐化していたためである。
  既に緑の `..._completion_uses_open_document` (`:17087`) が object 形を期待していることで、
  現行契約が object 形であることは実測で裏が取れている。

  検証 (`cargo test -p lsharp-wasm --test e2e -- --ignored
  lsp_stdio_completion_uses_spec_changed_document`、2026-08-22):
  **2 passed / 0 failed** (`221.10s`)。RED は同日の facet A GREEN run で
  同 2 本が FAILED (左辺に keyword 7 件 + 右辺が三要素配列) であることを実測している。
- **lane 全体の監査** (2026-08-23、`I-53` へ分離): 上記の「未監査」は
  `cargo test -p lsharp-wasm --test e2e -- --ignored lsp_stdio` (filter は lane 全体、93 本一致) で解消した。
  結果は **29 passed / 64 failed** (`4346.02s`)。`..._lsp_stdio_wire_repeated_sequence` は
  **FAILED** で、原因は位置ではなく最初の frame の initialize capabilities の形
  (実測 object 形 vs 期待 `[1,1,1,1,1,1,1]`) — facet B と同型の inline 期待値の陳腐化である。
  この test の completion が送る `"col":23` は 2 行目 `(defn main []  (he))` の範囲外を指しており、
  initialize を直した後の再測で初めて表面化する。**多段になる前提で扱う。**
  lane 全体の内訳と、そこで見つかった 2 系統の別問題は `I-53` / `I-54` / `I-55` が持つ。

#### B: snapshot file が 2026-08-03 以前の縮約形のまま残っている

- **内容**: `assert_lsp_stdio_snapshot` (`selfhost_cli_core.rs:80-84`) は
  `parse_lsp_stdio_frames` の**生の JSON をそのまま** snapshot file と `assert_eq!` する。
  正規化器は挟まっていない。ところが `tests/snapshots/lsp/stdio/*.json` は
  completion item を `["label", kind, "insertText"]` の**三要素配列**で、diagnostics を
  `{"col":1,"line":1,"messageHash":1,"rule":1,"severity":1,"source":2}` の**型タグ**で持つ、
  実出力には存在しない縮約形で書かれている。したがって **A を直しても、snapshot file を読む test は緑にならない**。 **規模は当初の 6 本ではなく、lane 全体の実測で `assert_lsp_stdio_snapshot` (`:84`) 内で落ちるものが 31 本ある** (`I-53`)。ただし snapshot file が全て陳腐化しているわけではない — echo 系の 4 本 (`..._document_sequence_` / `..._publish_diagnostics_` / `..._request_after_shutdown_` / `..._unknown_method_schema_snapshot`) は緑である。ただし inline 期待値の 2 本は `assert_lsp_stdio_snapshot` を経由しないので B の対象外であり、既に解決済みである (facet A の節を参照)。
- **根拠**: `..._completion_schema_snapshot` の実測。左右の内容は同一 (keyword 7 件) で、形だけが違う。

  ```
  left  (実測): "result":[{"label":"defn","kind":14,"insertText":"defn"}, ...]
  right (file): "result":[["defn",14,"defn"], ...]
  ```

- **いつ壊れたか**: completion item の emit を LSP 準拠の object 形へ変えたのは
  `5db1c2a4` 「fix: project native lsp completion and formatting」(2026-08-03、
  `LspServerNav.ls:140-155`)。snapshot file 側は `78813333` (2026-04-03) で止まっている。
  A と B は**同じ日の別 commit**で入っており、どちらも fixture / snapshot の追随が漏れた。
- **解決の第一段** (2026-08-23): `tests/snapshots/lsp/stdio/completion.json` を object 形へ転記した。
  実測の左辺と snapshot の右辺を突き合わせ、**値 (label / kind / insertText の 7 件) が
  完全に一致し、形だけが違う**ことを確認したうえで転記した = レビュー済みの転記である。
  検証: `cargo test -p lsharp-wasm --test e2e -- --ignored lsp_stdio_completion_schema_snapshot`
  で **1 passed / 0 failed** (`206.74s`、2026-08-23)。
- **転記できるのは 31 本中 3 本だけだった**。`I-53` の lane ログの左右を機械比較したところ、
  値まで一致する (= 純粋な形式ドリフト) のは以下だけである。

  | 分類 | 本数 | 引き取り先 |
  |---|---|---|
  | 形式のみ | 3 (`completion_schema_snapshot` / `initialize_schema_snapshot` / `initialize_shutdown_schema_snapshot`) | `LSP-SNAPSHOT-SHAPE-02` |
  | 位置起因で値が違う | 20 (nav 系 17 + completion 3) | `I-55` / `LSP-COL-CONV-03` |
  | diagnostics の内容差 | 8 (`document_sequence_*` 6 + `filesystem_document_sequence_*` 2) | `I-54` (判別待ち) |

  initialize 2 本は値の比較では差が出る (`[1,1,1,1,1,1,1]` 対 6 個の `Bool(true)` +
  `textDocumentSync:1` + `completionProvider:{}`) が、**能力の集合としては同一**であることを
  目視で確認したので形式ドリフトに分類した。機械比較は一次選別にすぎず、目視が要る。
  **残り 28 本は「転記すれば緑になる」ものではない。** 原因を先に解く。

#### 共通

- **`I-49` の slice が持ち込んだものではない** — `I-49` の diff は
  `Cli.ls` / `EmbeddedCli.ls` の `run-test-source-json` / `run-test-source-text` にしか触れておらず、
  補完経路に一行も入っていない。原因 commit はいずれも 2026-08-03 で、`I-49` の 19 日前。
  ただし **HEAD での再現は未実施** (working tree 復元が auto-mode classifier に阻まれたため)。
  根拠は diff の局所性・履歴・出力の意味的一致の 3 点。
- **なぜ台帳に無かったか**: これらは `#[ignore]` test であり、
  `docs/development/validation/ignored-lane-expected-failures.txt` は宣言スコープが
  `selfhost_native_stage_chain.rs` 単体に限られる。`selfhost_cli_core.rs` の `--ignored` lane は
  **どちらの台帳の対象でもない**。「どちらの表にも無い = 落ちていない」とは読めない。
- **修正方針**: A は (a) fixture の col を 0-indexed へ直す、で決着した (2026-08-22、上記)。
  B は **(d) snapshot file を現行の object 形へ書き直す** を採る (2026-08-22 決定)。

  **(e) `assert_lsp_stdio_snapshot` に縮約器を入れて生出力を縮約形へ落とす、は却下した。**
  理由は 2 つ。第一に、縮約器を置くと「何を縮約対象とするか」がそのまま LSP 出力の契約になり、
  snapshot が wire の真値を写さなくなる。回帰検知の対象が「実際に送られる bytes」から
  「縮約後の何か」へすり替わり、縮約器の穴に落ちた差分は永久に検知できない。
  第二に、縮約形は 2026-04-03 の実装出力をたまたま写しただけの形式であり、
  保存する価値のある設計ではない。維持コストを払う理由が無い。

  **(d) は「無検討な一括再生成」ではない。** これらの snapshot は insta 管理ではなく
  手管理の JSON ファイルで、`cargo insta accept` 相当の機構自体が無い。
  実測した左辺を読み、item の中身が期待どおりであることを確認したうえで転記する
  = レビュー済みの転記である。**中身が変わっている snapshot があれば、それは
  形式ドリフトではなく別の回帰なので、転記せず個別に issue を切る。**
- **状態の内訳**: facet A は resolved。**facet B が未解決なので issue 全体は open のまま**である。
- **関連**: `I-49` (発見の経緯)。引き取り先は `TODO.md` の `LSP-SNAPSHOT-SHAPE-01` (B) と
  `LSP-COL-CONV-03` (snapshot file を読む 3 本の位置修正。B の解消と同時にしか検証できない)。
  wire 位置規約の正本は `AGENTS.md`。

<a id="i-53"></a>

### I-53: `lsp_stdio` lane 93 本のうち 64 本が赤で、`I-52` の補完 9 本では説明できない

- **影響度**: 中 / **状態**: open / **発見**: 2026-08-23 (`I-52` の「未監査」を潰すための lane 全体監査)
- **実測**: `cargo test -p lsharp-wasm --test e2e -- --ignored lsp_stdio`
  (filter は lane 全体。`nohup` 切り離しで実行、2026-08-23)

  ```
  test result: FAILED. 29 passed; 64 failed; 0 ignored; 0 measured; 2981 filtered out;
  finished in 4346.02s
  ```

  **分母は 93 本である。** 事前の grep 見積もり 73 本は `main_with_` 付きだけを数えており、
  `test_e2e_selfhost_cli_lsp_stdio_*` 系を取りこぼしていた。分母を 73 と書かない。
- **証拠の所在**: 生ログと分類は `/Users/biwakonbu/github/tmp/lsp-stdio-lane-red/`
  (`lsp_stdio_full_red.log` / `lsp_class.txt`)。再取得には 72 分かかるため、
  **以後の slice はこのログの左辺を RED 証拠として再利用する** (再 run は GREEN 側だけでよい)。
- **内訳** (panic 位置で機械的に分けたのち、左右を読んで分類):

  | 系統 | 本数 | panic 位置 | 引き取り先 |
  |---|---|---|---|
  | B: snapshot 形式のドリフト | 31 | `selfhost_cli_core.rs:84` (`assert_lsp_stdio_snapshot` 内) | `I-52` facet B |
  | C: nav 系が退化した結果を返す (原因は fixture の座標系。2026-08-23 判別済み) | 22 | 各 test の inline assert | `I-55` |
  | D: response 側の位置が内部値のまま fixture に固定 | 7 | 同上 | `I-54` |
  | diagnostics refresh 3 本 (`Content-Length` 250/259/256 実測 vs 167/164/168 期待) | 3 | `:5817` / `:6034` / `:6105` | `I-54` (形式判別を含む) |
  | `..._lsp_stdio_wire_repeated_sequence` (initialize capabilities の inline 期待値) | 1 | `:6286` | `I-52` facet B |

  C の 22 本は `hover` 6 / `definition` 5 / `references` 5 / `rename` 5 / `goto_definition` 1。
  D の 7 本は `formatting` 5 / `body_hover_spec_position_character_params` /
  `body_rename_spec_position_character_params`。
- **緑 29 本から読めること**: `..._lsp_stdio_zero_based_position_contract` と
  `..._lsp_stdio_standard_uri_navigation_contract` は **pass している**。
  すなわち wire 規約そのものは実装側で成立している。また `*_schema_snapshot` のうち
  `document_sequence` / `publish_diagnostics` / `request_after_shutdown` / `unknown_method` の
  4 本も緑で、**snapshot file が一律に陳腐化しているわけではない**。
  ただしこの 4 本は入力 body をそのまま返す echo 系であり、出力形式の証拠としては弱い
  (`publish-diagnostics.json` の中身は request body の逐語コピーである)。
- **回帰していないことの傍証**: 直近 2 slice で緑にした 6 本
  (`e24bc0f6` の 4 本 + `6af8b52f` の 2 本) は、この lane 全体 run でも全て `ok` だった。
- **なぜ 1 issue にまとめないか**: `I-52` は「補完が 2 系統の理由で落ちる」という
  機構の issue で、facet A は既に resolved である。ここへ lane 全体を後付けで流し込むと
  A の解決追跡が濁る。機構 (A / B) は `I-52` に残し、**lane 全体の実測と未分類の系統は本 issue が持つ**。
- **関連**: `I-52` (機構 A / B)、`I-54` (D)、`I-55` (C)。
  引き取り先は `TODO.md` の `LSP-SNAPSHOT-SHAPE-02` (B の残り約 30 本)。

<a id="i-54"></a>

### I-54: LSP の response 側の位置が wire 変換前の内部値で fixture に固定されている

- **影響度**: 中 / **状態**: open / **発見**: 2026-08-23 (`I-53` の lane 監査)
- **内容**: `I-52` facet A は **request 側**の位置規約のずれだった。同じずれが
  **response 側**にもあり、しかも向きが test ごとに違う。どちらも fixture が
  `9175c6e5` (2026-08-03) の wire 正規化に追随していないことの表れである。

  | test | 実測 (left) | 期待 (right) | 向き |
  |---|---|---|---|
  | `..._lsp_stdio_body_hover_spec_position_character_params` | `["42","2","39"]` | `["42","1","38"]` | 実装が内部値 (1 始まり) を返す |
  | `..._lsp_stdio_body_rename_spec_position_character_params` | `["42","2","39","cube"]` | `["42","1","38","cube"]` | 同上 |
  | `..._main_with_lsp_stdio_formatting` | `line:0,character:0` 〜 `0,16` | `line:1,character:1` 〜 `1,17` | 期待が 1 始まりで陳腐化 |

  前 2 本は `lsp-stdio-nav-params` が返す**変換後の内部値**を test が直接見ており、
  後者は response range が wire (0 始まり) へ正規化されたのに fixture が追随していない。
  **「どちらも off-by-one」で丸めると向きが逆であることが消える**ので、行ごとに向きを記録する。
- **diagnostics refresh 3 本**: `..._body_document_sequence_spec_params_publishes_{,type_,lint_}diagnostics_refresh`
  は `Content-Length` が 250/259/256 (実測) 対 167/164/168 (期待) で、
  位置だけでなく **frame の形そのものが違う**。位置の問題なのか `I-52` facet B と同型の
  形式ドリフトなのかは未判別。**推測で分類しない。**
- **修正方針**: 未決。`I-52` facet A と同じく「実装ではなく fixture を直す」が既定線だが、
  向きが逆の 2 群を同じ理由で直せるかは検算するまで決めない。
- **関連**: `I-52` (request 側の同型問題、facet A で resolved)、`I-53` (実測の出所)。
  引き取り先は `TODO.md` の `LSP-COL-CONV-04`。

<a id="i-55"></a>

### I-55: hover / definition / references / rename の fixture が内部 1 始まり座標のまま止まっている

- **影響度**: 中 / **状態**: open (原因は判別済み、fixture 修正が未了) / **発見**: 2026-08-23 (`I-53` の lane 監査)
- **内容**: nav 系 22 本が、単なる形式差ではなく**シンボル解決に失敗した形**の結果を返す。

  | test | 実測 (left) | 期待 (right) |
  |---|---|---|
  | `..._hover_uses_open_document` | `range` が `line:-1,character:-1`、`contents:"type-info:2:39"` | `range` が `1,36`〜`1,42`、`contents:"defn helper"` |
  | `..._references` | `[[10,2,39]]` (1 件) | `[[10,1,7],[10,1,36],[10,1,47]]` (3 件) |
  | `..._rename` | `[[0,[]]]` (編集 0 件) | `[[10,[[1,7,1,13,"cube"],[1,36,1,42,"cube"],[1,47,1,53,"cube"]]]]` |
  | `..._goto_definition` | `[10,1,0]` | `[10,1,7]` |

- **判別の結果** (2026-08-23、cargo を回さず source と緑の契約 test を読んで検算):
  **実装退行ではない。`I-52` facet A と同じ「fixture が内部 1 始まり座標のまま」が
  request 側と response 側の両方で起きているだけである。** 実装側の wire 契約は
  **入力も出力も 0 始まり**で確定しており、それを緑の contract test 2 本が押さえている。

  | 緑の contract test | 送る位置 | 返る位置 | source |
  |---|---|---|---|
  | `..._lsp_stdio_zero_based_position_contract` (`:5081`) | `line:0,character:6` | `start 0,6` / `end 0,12` | `(defn helper [x] x)` (1 行) |
  | `..._lsp_stdio_standard_uri_navigation_contract` (`:5141`) | `line:1,character:15` | `start 0,6` / `end 0,6` | `(defn helper [x] x)\n(defn main [] (helper 1))` |

  前者は `helper` が 0-based 6..11 の位置で `character:6` を送って当たっており、
  `end` は排他で 12。後者は 2 行目 (`line:1`) の `character:15` が `helper` の内側に当たり、
  定義位置を `line:0,character:6` で返す。**両方向とも 0 始まりである。**

  これに対し `..._hover_uses_open_document_spec_params` (`:16723`) は
  source が `(defn helper [x] x) (defn main [] (helper 1))` の **1 行 45 byte** なのに
  `"line":1,"character":38` を送る。wire が 0 始まりなら 1 行の文書に `line:1` は存在せず、
  実装の `+1` を経て内部 `(2,39)` となって lookup が外れる。実測の
  `range: -1,-1` と `contents:"type-info:2:39"` は、その miss 時の fallback が
  内部位置をそのまま埋めた形である。期待値 `start 1,36` / `end 1,42` も、
  2 個目の `helper` の 0-based 35..40 を **1 始まりへ直した値**であり、
  request と response の両方が内部座標で書かれている。
- **したがって残る作業は fixture の書き換えだけ**だが、**多段になる**:
  request 側を wire 規約へ直すまで、成功時の response が実際にどの座標で返るかは
  測れない。特に `"uri":10` のような数値 uri を使う旧経路
  (`..._references` の `[[10,1,7],...]` 形) は緑の contract test が覆っていないので、
  **response の期待値は request を直したあとに実測して決める。推測で埋めない。**
- **修正方針**: fixture を wire 規約 (0 始まり) へ直す。実装には触らない。response 側の期待値は request 修正後の実測で決める。
- **関連**: `I-52` (facet A、帰結仮説の元)、`I-53` (実測の出所)、`I-54` (response 側の位置)。
  引き取り先は `TODO.md` の `LSP-NAV-DEGRADE-01`。

<a id="doc-01"></a>
<a id="i-31"></a>
### I-31: `cargo clippy -p lsharp-types -- -D warnings` が main で既に落ちる

- **影響度**: 低 / **状態**: resolved / **発見**: 2026-08-22 (`INFER-DEPTH-01` の検証中)
- **内容**: `crates/lsharp-types/src/review_trust_store.rs:120` の nested `if` が
  `clippy::collapsible_if` に当たる。`-D warnings` を付けると **lib / lib test / all-targets の
  3 経路で compile error になる**ため、この crate では clippy を gate として使えない。
  **lint 指摘は 1 件で、3 経路すべてが同じ 1 件を再検出していた** (サマリー行の旧記述
  「3 件」は経路数を件数と取り違えたもの。2026-08-22 に訂正)。
- **根拠**: 2026-08-22 実測。変更を `git stash` した状態でも同じ 3 件が出る。
  したがって `INFER-DEPTH-01` の変更が持ち込んだものではない。
- **経緯**: `fix-clippy-collapsible-match` という branch が存在したが `main` の祖先で、
  当該箇所は覆っていない。lint 債務として残っていた。
- **解決** (2026-08-22): clippy の suggest どおり let chain へ畳んだ
  (`if key.is_active() && let Some(existing) = ...`)。edition 2024 / rustc 1.93 では
  let chain が stable なので `#[allow]` による握り潰しは選ばなかった。
  `--lib` / `--lib --tests` / `--all-targets` の 3 経路が exit 0。
  `cargo test -p lsharp-types` は全 suite pass (`test result: ok` のみ、`failed` 非 0 なし)。
  当該 file は `cargo fmt --check` の diff にも現れない。
- **CI の扱いは決めていない**: 起票時は「修正を入れるなら同時に `-D warnings` を CI で
  常時要求するかを決める必要がある」としていたが、`TODO.md` の `LINT-CLIPPY-01` が
  受入条件を「3 経路が exit 0」に限定し、CI 常時要求の判断を明示的に範囲外へ置いた。
  **gate を緑にすることと、gate を CI で強制することは別の判断である**ため、
  前者だけを実施した。後者は未決のまま残る。
- **関連**: `INFER-DEPTH-01`、`I-34` (同時に解消した format gate)。

<a id="i-33"></a>
### I-33: analysis-only cache の直後に compile すると空の IR が返る

- **影響度**: 中 / **状態**: resolved / **発見**: 2026-08-22 (`WORKTREE-ABSORB-02` の
  `codex/legacy-module-scc-cache-contract` 判定中)
- **内容**: `analyze_multi_file_incremental_with_overrides` は AST と型 surface だけを cache へ入れ、
  IR は `build_module_cache_entry` (`compile_support.rs:108`) が全 field 空の placeholder を置く。
  compile 側の clean-hit 判定は fingerprint 一致だけを見ていたため、**analyze の直後に
  `compile_multi_file_with_cache` を呼ぶと空 module がそのまま返る**。
  `crates/lsharp-tooling/src/compile.rs:259` から呼ばれる公開経路である。
- **根拠**: RED test `test_compile_multi_file_with_cache_materializes_ir_after_analyze_only_cache`
  が `compiled.functions` 空で落ちた (2026-08-22 実測)。
- **解決** (2026-08-22): `ModuleCacheEntry` に `ir_ready` を持たせ、`set_ir` 時のみ true にする。
  compile 側の clean-hit 2 箇所 (`compile_incremental.rs:486` / `:540`) に `has_ir()` を足した。
  analysis 側の判定と SCC 経路の `linked_module` guard は変更していない。
  判断は [analysis/compile cache 境界 ADR](docs/adr/decisions-legacy-module-analysis-compile-cache-boundary.md)。
- **経緯**: `codex/legacy-module-scc-cache-contract` @ `265a42c5` (2026-07-24) が同じ問題を
  branch 側で直していた。main は `lib.rs` を分割済みで diff は当たらないため、
  指摘だけを取り込んで main の構造へ移植した。
- **関連**: `LEGACY-MODULE-01`、[worktree 取り込み判定](docs/adr/decisions-worktree-absorption-2026-08-20.md)。

<a id="i-34"></a>
### I-34: `cargo fmt --check -p lsharp-ir` が main で既に落ちる

- **影響度**: 低 / **状態**: resolved / **発見**: 2026-08-22 (`I-33` の commit 前検査中)
- **内容**: `crates/lsharp-ir/src/lower/mod.rs` の `mod` 宣言順が rustfmt の期待と食い違い、
  8 箇所の diff が出る。`#[cfg(test)] mod *_tests;` が対応する `mod *;` の直後ではなく
  ブロック末尾へ寄せられているのが原因。
- **根拠**: 2026-08-22 実測。`cargo fmt --check -p lsharp-ir` の `Diff in` は
  `lower/mod.rs` の 8 箇所だけで、他のファイルには出ない。`I-33` の変更以前から出ている
  (触ったのは `cache.rs` / `compile_incremental.rs` / `lib_tests/incremental_compile.rs` の 3 本)。
- **解決** (2026-08-22): `cargo fmt -p lsharp-ir` を適用した。差分は `lower/mod.rs` 1 file の
  7 挿入 / 8 削除で、`#[cfg(test)] mod *_tests;` が対応する `mod *;` の直後へ移り、
  `pub use` の並びが辞書順へ揃い、末尾の余分な空行が 1 行落ちただけである。
  `cargo fmt --check -p lsharp-ir` が exit 0、`cargo test -p lsharp-ir` は 301 passed / 0 failed。
- **起票時の懸念は空振りだった**: 「test 分割の並び順を機械的に崩す」ことを理由に保留していたが、
  実際に動いたのは宣言順だけで `include!` の順序にも test の内容にも触れていない。
  **並び順が変わる**ことと**分割の構造が壊れる**ことを同一視した誤りである。
- **CI の扱いは決めていない**: `I-31` と同じ理由。gate を緑にすることと CI で強制することは
  別の判断で、前者だけを実施した。
- **関連**: `I-31` (同じく main で既に落ちていた gate。同じ slice で解消)。

<a id="i-35"></a>
### I-35: allocator の到達不能 free-list search に誤った `Br(0)` が残っている

- **影響度**: 低 / **状態**: resolved / **発見**: 2026-08-22 (`codex/legacy-maintenance-stage-chain-integration` の判定中)
- **内容**: `emit_alloc_func` の legacy free-list first-fit search は、size-class heads を
  導入した際に `crates/lsharp-wasm/src/wasi/allocator.rs:140` の無条件 `Br(0)` で
  丸ごと skip されるようになった。skip される区間の `:172` にある `Br(0)` は
  **内側の `if` を抜けるだけで search loop の次 iteration へ進まない**という誤りを
  抱えたまま残っている。現状は到達不能なので挙動に影響しない。
- **根拠**: 2026-08-22 実測。`allocator.rs:137-140` が
  「旧 table は新しい class heads と併用しない。コードは ABI 差分を小さく保つため残すが、
  常に bump/class path へ進む。」というコメント付きで `Br(0)` を出している。
  `:167-173` の内側 `if` 末尾が `Br(0)` で、`Br(1)` であるべき。
  `codex/legacy-maintenance-stage-chain-integration` の `8be951e4`
  (`fix(wasm): skip undersized free-list entries`) が同じ箇所を `Br(1)` へ直しているが、
  main では dead path なので取り込みは却下した。
- **解決** (2026-08-22): **区間ごと削除した。** `Br(1)` へ直して残す案は却下した。
  ABI は function signature と heap layout であって instruction 列ではないので、
  「ABI 差分を小さく保つため残す」という当初の理由が根拠を欠いていた。
  削除は emitter 91 行と、この区間が唯一の参照だった
  `AllocatorGlobals::free_list_base_global_idx` (struct field と構築 4 箇所)。
  判断は [到達不能 free-list の削除](docs/adr/decisions-allocator-dead-free-list-removal.md)。
- **再発防止**: `allocator_body_has_no_unreachable_block_prologue` が、`__alloc` の
  encode 済み body に `block (empty)` + `br 0` のバイト列が現れないことを検査する。
  この形は「区間を丸ごと到達不能にして残す」ときにだけ出るので、**同じ形の再発を
  実行に依存せず禁止できる**。到達不能命令は wasm validator を通ってしまうため、
  中の誤りは behavioural test では原理的に検出できない。
- **`I-04` は閉じていない**: 起票時は「`I-04` の範囲で決める」としていたが、`I-04` が問うのは
  「線形探索をどう速くするか」であり、その答えである size-class heads は既に入っている。
  今回決めたのは「置き換え済みの旧実装を保持しない」という一点で、class 境界の選び方や
  oversize の扱いには触れていない。`I-04` は自身の範囲で open のまま残す。
- **関連**: `I-04`。起票時の判定は
  [worktree 取り込み判定](docs/adr/decisions-worktree-absorption-2026-08-20.md)。

<a id="i-36"></a>
### I-36: AST / token の Display が string escape と型注釈を落とす

- **影響度**: 中 / **状態**: open / **発見**: 2026-08-22 (`codex/legacy-maintenance-stage-chain-integration` の判定中)
- **内容**: pretty-print した結果が元の source として re-parse できない箇所が 2 種類ある。
  1. **string literal の escape が復元されない。** `crates/lsharp-syntax/src/ast.rs:602` と
     `crates/lsharp-syntax/src/token.rs:71` がどちらも文字列を quote で挟むだけで、
     lexer が解釈済みの `"` / backslash / 改行 / tab / CR を生文字のまま出す。
     `"` を含む文字列は出力が壊れ、改行を含む文字列は行が割れる。
  2. **宣言の型注釈が落ちる。** `ast.rs:325` (defn) / `:519` (lambda) / trait method /
     defmacro のいずれも parameter を `p.name` だけで出し、`Param::ty` を無視する。
     defn の `where_clauses`、trait method の `return_ty`、defmacro の `macro_type` も
     出力されない。
- **根拠**: 2026-08-22 実測 (コード読み)。加えて **既存の roundtrip gate が構造上この 2 件を
  見られない**ことを確認した。`crates/lsharp-syntax/src/lib.rs:141`
  `roundtrip_property_tests::pretty_printed_ast_reparses_to_the_same_source` は
  `test_gen.rs` の generator を使うが、
  - `safe_string()` (`test_gen.rs:23`) は `a`-`z` 6 文字までしか生成せず、
    escape が要る文字を一切含まない
  - `arb_expr()` が作る `Param` は `ty: None` 固定 (`test_gen.rs:59-63`) で、
    `Decl` を生成しないため `where_clauses` にも届かない

  gate は存在するが、生成器の側で盲点が作られている。
- **含めない範囲**: 修正そのもの。`codex/legacy-maintenance-stage-chain-integration` の
  `05b98847` / `fe5ed3c1` が参照実装になるが、1510 commit 越しに patch を当てず
  main の現行 file 構成 (`lexer/` / `parser/` 分割後) の上で書き直す。
  metadata projection の Display は別契約なので含めない。
- **関連**: `FMT-ROUNDTRIP-01` (`TODO.md`)。判定は
  [worktree 取り込み判定](docs/adr/decisions-worktree-absorption-2026-08-20.md)。
  公開 CLI では `fmt` は LSP / MCP の内部 API 扱いなので、影響面は IDE 経路である。

<a id="i-37"></a>
### I-37: 別 module の同名 top-level function が衝突し、診断なしに誤った wasm を出す

- **影響度**: 高 / **状態**: open / **発見**: 2026-08-22 (`codex/legacy-maint-native-stage-chain-split` の判定中)
- **内容**: 複数 module を import して build したとき、**module をまたいで同名の top-level
  function があると 1 つに潰れる**。診断は出ず、コンパイルは成功し、
  出力された wasm が黙って誤った値を返す。silent miscompilation であり、
  本台帳で最も影響度が高い種類の欠陥である。

  勝者は import 順ではなく **module 名の辞書順で決まり、後ろの名前が勝つ**。
  ただし **entry module だけは最後に merge され、常に勝つ**。
  `crates/lsharp-ir/src/compile_surface.rs:34` の
  `results.sort_by(|left, right| left.0.cmp(&right.0));` が module 名で並べ替え、
  entry をその後ろへ置く構造がそのまま観測されている。
- **根拠**: 2026-08-22 実測。`target/debug/lsharp` (main `12a351ce` を含む build) で再現する。

  ```
  A.ls    (module A)
          (defn helper [] : Int 1)
          (defn a [] : Int (helper))
  B.ls    (module B)
          (defn helper [] : Int 20)
          (defn b [] : Int (helper))
  Main.ls (module Main)
          (import A)
          (import B)
          (defn main [] (print (+ (a) (b))))
  ```

  ```bash
  lsharp compile Main.ls -o Main.wasm && wasmtime Main.wasm
  ```

  | fixture | 期待 | 実測 |
  |---|---|---|
  | `helper` を `hA` / `hB` と別名にする | 21 | **21** |
  | 両方 `helper` (`import A` → `import B` の順) | 21 | **40** |
  | 両方 `helper` (`import B` → `import A` の順) | 21 | **40** |

  `1 + 20` が `20 + 20` になっている。**import 順を入れ替えても 40** なので
  「後から register した方が勝つ」ではない。勝敗規則は追加 fixture で切り分けた。
  以下はいずれも `dup` を複数 module が定義し、呼び出し側だけを変えたもの。

  | fixture | 呼び出し元 | 期待 | 実測 | 読み方 |
  |---|---|---|---|---|
  | `Zeta.dup`=1 / `Alpha.dup`=20、`Main` が両方を呼ぶ | `Main` | 21 | **2** | 辞書順で後ろの `Zeta` が両方を奪う |
  | 同上で `import` 順を反転 | `Main` | 21 | **2** | 順序非依存 |
  | `Alpha.dup`=20 / `Mid.dup`=100 / `Zeta.dup`=1、`Mid.viaM` が自 module の `dup` を呼ぶ | `Mid` | 100 | **1** | **自 module 内の呼び出しすら奪われる** |
  | 上に加えて `Zeta` も `main` を持つ | `Main` (entry) | — | `Main` の `main` が生存 | entry は奪われない |
  | `Main` (entry) 自身が `dup`=100 を定義、`Zeta.dup`=1 | `Main` | 100 | **100** | entry の定義は常に勝つ |

  つまり **登録先の key が module で修飾されておらず、merge 順の最後が勝つ**。
- **selfhost がこの挙動に依存している**: 2026-08-22 の走査で、`selfhost/src` の top-level `defn`
  6748 件のうち **310 個の名前が複数 file に重複**している (すべて file をまたぐ重複)。
  entry ごとの import 閉包で数えると `App.Cli` 65 / `App.Main` 62 / `App.PipelineSmoke` 62 /
  `App.EmbeddedCli` 61 / `App.SmokeCli` 56 / `App.CompilerMode` 21 / `App.ModuleResolver` 0 件。
  `App.Cli` 閉包の 65 件は **本文一致 38 / 本文相違 27**。

  ```bash
  # 重複名の総数
  grep -rhoE '^\(defn [a-z0-9?!*<>=+/-]+' selfhost/src --include=*.ls |
    awk '{print $2}' | sort | uniq -d | wc -l
  ```

  相違する 27 件のうち `infer-*` 一族は **意図的な上書き**である。
  `selfhost/src/Types/TypeInfer.ls:219-225` は
  `;; --- Block グループ (TypeInferBlock.ls が上書き) ---` というコメント付きで
  `(defn infer-apply [node env subst counter] (make-result subst (fresh-type-var counter)))` のような
  stub を置き、実体は `TypeInferApply.ls:731` などにある。
  `Types.TypeInfer` < `Types.TypeInferApply` / `TypeInferBlock` なので辞書順で実体が勝つ。
  **現状の正しさは命名規約の偶然** — sub module が親より後ろへ並ぶという規約に依存している。

  したがって `MODULE-DUP-FN-01` の修正は selfhost を巻き込む。
  qualify する設計では `TypeInfer.ls` 内部の呼び出しが自 module の stub へ解決して
  **型推論が黙って劣化**し、重複を診断で reject する設計では 65 件が一斉に落ちる。
  どちらへ倒すにせよ **selfhost 側の重複整理が前提**になる。
- **含めない範囲**: 修正方法の選択 (呼び出し側を module 修飾するか、
  lowering 時に function 名を qualify するか) は `MODULE-DUP-FN-01` (`TODO.md`) で決める。
  block 形式 module body での同種の衝突は `I-39` の範囲。
  selfhost の重複整理そのものは `TYPEINFER-SPLIT-01` / `LEGACY-MAINT-01` の範囲。
- **関連**: `MODULE-DUP-FN-01` (`TODO.md`)、`I-38` / `I-39` (同じ module 名前解決の欠落)、
  `I-40` (この衝突の上に載っている DocTools の契約ずれ)。
  参照実装は `codex/legacy-maint-native-stage-chain-split` の `f5a343a8`
  (`scoped_visibility` による root body function の qualify)。判定は
  [worktree 取り込み判定](docs/adr/decisions-worktree-absorption-2026-08-20.md)。

<a id="i-38"></a>
### I-38: import した module の `type-alias` が展開されず型不一致になる

- **影響度**: 中 / **状態**: open / **発見**: 2026-08-22 (`codex/legacy-maint-native-stage-chain-split` の判定中)
- **内容**: `type-alias` は**同一 file 内でしか展開されない**。別 module で定義された alias を
  import 先の signature で使うと、alias 名が未展開の `Type::Con` のまま残り
  `型の不一致: expected String, found Text` になる。修飾しても同じで、
  `A.Text` と書けば `found A.Text` になるだけである。
  型名としては受理されるので、**「そんな型は無い」という診断は出ない**。
- **根拠**: 2026-08-22 実測。`target/debug/lsharp` で再現する。

  | fixture | 実測 |
  |---|---|
  | 同一 file の `(type-alias Text String)` を signature で使う | **成功** |
  | `A.ls` で定義し、`A` 内部の関数だけが使う (import 側は触れない) | **成功** |
  | `A.ls` で定義し、import 側の signature で `Text` と書く | `expected String, found Text` |
  | 同上、`A.Text` と修飾して書く | `expected String, found A.Text` |

  実装側では `crates/lsharp-types/src/infer.rs:86` の `type_aliases` が単一 map で、
  multi-file 経路 (`infer/decl.rs:160`) は `{module_name}.{name}` で register する一方、
  展開側 (`infer.rs:262` / `:276` / `:293`) は解決した名前をそのまま lookup している。
  main に **cross-module の type-alias を張る test は 1 件も無い**
  (`fn test.*alias` の hit はすべて import alias (`:as`) か同一 file の alias)。
- **含めない範囲**: alias を module 越しに公開するか (公開するなら `:only` / `:open` の
  可視性とどう組むか) は設計判断で、`MODULE-ALIAS-EXPORT-01` (`TODO.md`) で決める。
  「公開しない」に倒す場合も、**現状の型不一致診断は誤りなので直す対象は残る**。
- **関連**: `MODULE-ALIAS-EXPORT-01` (`TODO.md`)、`I-37` / `I-39`。
  参照実装は `codex/legacy-maint-native-stage-chain-split` の `cfcb19a7`
  (`incremental/scoped_type_alias.rs`)。判定は
  [worktree 取り込み判定](docs/adr/decisions-worktree-absorption-2026-08-20.md)。

<a id="i-39"></a>
### I-39: block 形式の module body が parse されるだけで、lowering されない

- **影響度**: 高 / **状態**: resolved / **発見**: 2026-08-22 (`codex/legacy-maint-native-stage-chain-split` の判定中)
- **内容**: `(module M (defn f ...) (defn g ...))` という **body を括弧内に持つ形**は
  parser が受理し (`Decl::ModuleDecl { name, body }`)、型推論と検査系も body を走査する
  (`crates/lsharp-ir/src/compile_pipeline.rs:229` / `:282`、metadata contract 検査)。
  しかし **lowering は `Decl::ModuleDecl` を一度も見ない**。
  `crates/lsharp-ir/src/lower/program.rs` は `Decl::Defn` だけを拾い、
  multi-file 経路の `crates/lsharp-ir/src/compile_entrypoints.rs:79-80` は
  `Decl::ModuleDecl { .. } | Decl::ImportDecl { .. } => {}` で丸ごと捨てる。

  症状は 2 つに分かれ、**後者の方が重い**。

  1. body 内に sibling 参照があると `未定義の変数` / `型の不一致` という**誤診断**で止まる。
     利用者からは自分のコードの誤りに見える
  2. sibling 参照が無いと **compile が成功し、何も起きないバイナリが exit 0 で完成する**。
     失敗を知らせるものが何も無い
- **根拠**: 2026-08-22 実測。`target/debug/lsharp` + `wasmtime` で再現する。

  | fixture | compile | 実行 |
  |---|---|---|
  | `(module Main (defn helper [] 1) (defn main [] (print (helper))))` | `[LS1001] [E0001] 未定義の変数 (undefined): helper` | -- |
  | `(module Main (defn main [] (print 42)))` | **成功** 6472 bytes | **無出力 / exit 0** |
  | `(module Main)` + `(defn main [] (print 42))` (flat 形) | 成功 6498 bytes | `42` / exit 0 |
  | `(module App (module Sub (defn succ [x] (+ x 1))))` + top-level `main` | **成功** | -- |

  block 形式を使う `.ls` は追跡下 133 件のうち **2 件**ある
  (`crates/lsharp-types/tests/fixtures/metadata/nested_contract_forms.ls`、
  `tests/fixtures/validation/ec-m2-project-duplicate-source.ls`)。
  Rust test 内の inline source では 58 箇所。いずれも metadata / validation 検査か
  selfhost の parser / assertion checker へ渡す文字列で、**compile 経路には入らない**。
  よって live な regression ではないが、**block 形式は検査系が意図的に受理している surface**
  であり、「使用 0 件だから何を壊してもよい」わけではない。

  > **訂正 (2026-08-22)**: 本項は当初「この形を使っている `.ls` は repo 内に 1 件も無い」
  > と書いていたが誤りで、上記 2 件がある。また症状を「誤診断で落ちる」とだけ書いていたが、
  > 沈黙して誤答する経路を落としていた。影響度を 中 → 高 へ改めた。
- **解消根拠**: compile 経路の parse 直後・infer 直前で「未対応の構文」として reject する。
  判断と却下案 (実装 / parser で reject / lowering で reject) は
  [block 形式 module body の reject](docs/adr/decisions-module-body-form-rejection.md)。
  body の宣言は lowering に到達しないため、**この形に依存して正しく動いていた program は
  原理的に存在しない**。error 化で壊れるのは既に沈黙して誤答していた program だけである。
- **含めない範囲**: block 形式の実装そのもの。入れ子 module の可視性が `I-37` / `I-38` と
  併せて決まった時点で ADR の reject を撤回し、参照実装
  (`a5e5929c` / `fa7b4c51` / `68849d55`) を当てなおす。
- **関連**: `I-37` / `I-38`。判定は
  [worktree 取り込み判定](docs/adr/decisions-worktree-absorption-2026-08-20.md)。

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

<a id="doc-10"></a>
### DOC-10: 設計ドキュメントと TODO に完了済み項目が蓄積し、残作業が読めない

- **影響度**: 中 / **状態**: resolved / **発見**: 2026-08-22 (`codex/legacy-maintenance-docs-active-only` の判定中)
- **内容**: `TODO.md` には「**未完了タスクだけ**を持つ単一正本」という規約があるが、
  同じ規律が設計ドキュメントと一部の TODO 項目本文には効いていない。
  完了済みの分割・module 名が本文へ追記され続け、**残っている作業が読めなくなっている**。
- **根拠**: 2026-08-22 実測。

  | 対象 | 実測 |
  |---|---|
  | `docs/development/planning/improvement-designs/imp-06-large-file-decomposition.md` | 全 299 行のうち「## ステータス」節が **20,594 バイト**。完了済み module 名の列挙で、`infer.rs` が **8 回**、`lib.rs` が 4 回登場する |
  | `TODO.md` の `LEGACY-MAINT-01` 本文 | 完了済み seam の列挙が 1 行に連結され、**「残る責務分割を続ける」以外に何が残っているかが書かれていない** |

  ```bash
  awk '/^## ステータス/{f=1} f' docs/development/planning/improvement-designs/imp-06-large-file-decomposition.md | wc -c
  awk '/^## ステータス/{f=1} f' docs/development/planning/improvement-designs/imp-06-large-file-decomposition.md \
    | grep -oE '[a-z_]+\.rs' | sort | uniq -c | sort -rn | head
  ```

  完了の記録そのものは必要だが、置き場所は ADR / 運用記録であって設計ドキュメントの
  ステータス節ではない (`.claude/rules/docs-organization.md`)。
- **`I-01` との関係**: `I-01` は「39 file が 800 行超過」という**事実**を持ち、
  こちらは「その分割計画が読めない」という**記述の問題**である。分割の実作業は
  `LEGACY-MAINT-01`、gate は `RUST-FILE-SIZE-GATE-01` が持つ。
- **解決** (2026-08-22): imp-06 の「## 検証済み部分実装」「## ステータス」節 (計 23,486 バイト) を
  [Rust 側ファイル分割の完了記録](docs/development/operations/rust-file-decomposition-record-2026-08-22.md)
  へ逐語で移し、imp-06 には「## 残っている作業 (2026-08-22 実測)」だけを残した。
  `TODO.md` の `LEGACY-MAINT-01` 本文からも完了済み seam の列挙 4 行を落とし、
  運用記録と ADR 群への参照へ置き換えた。設計済み分割軸の表は残作業なので残した。
- **archive は破棄ではない**: 移した文章は 1 文字も削っていない。個別の分割の判断は
  もともと `docs/adr/decisions-legacy-*split*.md` (2026-08-22 時点で 166 本) にあり、
  imp-06 の列挙はその要約が累積したものだった。**同じ内容が 2 箇所にあることが問題**であって、
  内容そのものが不要だったわけではない。運用記録の側に「追記しないこと」と明記した
- **`I-01` の数値を 1 件訂正した**: `mcp_tests.rs` を 1889 行としていたが、同日の再実測で 1949 行。
  取得条件は同じコマンドなので、起票時の転記ミスである
- **含めなかった範囲**: 分割そのもの (`LEGACY-MAINT-01`)、gate (`RUST-FILE-SIZE-GATE-01`)、
  `TODO.md` 全体の再編。imp-06 の「### 3. 優先順位」「### 4. 機械検査」は設計であって
  完了記録ではないので触っていない。
- **関連**: `DOC-09` (完了 TODO の根拠が ADR へ移らない) と同じ病理の別の面。
  `codex/legacy-maintenance-docs-active-only` の `5348570e` は同じ問題を branch 側で
  直そうとしたが、**2026-07-24 時点の実測値を現在値として書く**形だったので却下した
  ([worktree 取り込み判定](docs/adr/decisions-worktree-absorption-2026-08-20.md))。
  方針は正しいので、main の実測に基づいてやり直す。
