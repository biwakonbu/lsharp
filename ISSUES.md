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
| [I-42](#i-42) | 静的 contract 判定が `if` / `let` / `do` / `match` を貫通しない | 中 | resolved | [worktree 取り込み判定](docs/adr/decisions-worktree-absorption-2026-08-20.md) |
| [I-43](#i-43) | `:example` / `:invariant` / `:doc` の識別子検査が false positive を出す | 中 | resolved | [worktree 取り込み判定](docs/adr/decisions-worktree-absorption-2026-08-20.md) |
| [I-44](#i-44) | 未定義の computation builder が型検査を通る | 中 | resolved | [computation builder の診断](docs/adr/decisions-computation-builder-diagnostics.md) |
| [I-45](#i-45) | selfhost の canonical `:case` preflight が 0 引数 `defn` の呼び出しを型エラーにする | 中 | resolved | [0 引数 defn の型](docs/adr/decisions-selfhost-zero-arity-defn-type.md) |
| [I-46](#i-46) | 前方参照された呼び出しは引数型も arity も検査されていない | 高 | open | [computation builder の診断](docs/adr/decisions-computation-builder-diagnostics.md) |
| [I-47](#i-47) | `cargo fmt --check` が 5 crate で落ちる (`I-34` は `lsharp-ir` しか見ていなかった) | 低 | resolved | -- |
| [I-48](#i-48) | selfhost のソースが `I-46` の穴に依存しており、vector をタプルとして使っている | 高 | open | -- |
| [I-49](#i-49) | selfhost の `:assert` lane は predicate を型検査していない | 中 | resolved | -- |
| [I-50](#i-50) | `lsharp compile` の入力ソース整形上書きが利用者へ通知されない | 中 | resolved | -- |
| [I-51](#i-51) | `compile -o <ディレクトリ成分の無いファイル名>` が artifact 同期で落ちる | 低 | resolved | -- |
| [I-52](#i-52) | LSP stdio 補完の e2e が 2 系統の理由で全滅 (位置規約の食い違い / snapshot 形式のドリフト) | 中 | resolved | -- |
| [I-53](#i-53) | `lsp_stdio` lane 93 本のうち 64 本が赤で、`I-52` の補完 9 本では説明できない | 中 | resolved | -- |
| [I-54](#i-54) | LSP の response 側の位置が wire 変換前の内部値で fixture に固定されている | 中 | resolved | -- |
| [I-55](#i-55) | hover / definition / references / rename の fixture が内部 1 始まり座標のまま止まっている | 中 | resolved | -- |
| [I-56](#i-56) | `source` を持たない document request で params の slot がずれ、open document state が参照されない | 中 | resolved | -- |
| [I-57](#i-57) | `definition` / `references` の response だけ LSP Location ではなく縮約 array で、line / col とも内部の 1 始まりが漏れている | 中 | resolved | -- |
| [I-58](#i-58) | lint 診断の dedup 意味論を pin する test が、real span 導入と同時に前提を失う | 低 | resolved | -- |
| [I-59](#i-59) | `:invariant` の型推論が quote を扱えず、識別子検査を直しても診断が残る | 低 | resolved | -- |
| [I-60](#i-60) | 0 引数 defn の型を pin する e2e 5 本が `I-45` の契約変更で赤のまま放置されている | 中 | resolved | -- |
| [I-61](#i-61) | `definition` / `references` の wire 形式が request の URI の送り方で分岐する (縮約 array / `Location` object) | 中 | resolved | 2026-08-23 |
| [I-62](#i-62) | `:example` は quote を含んでも診断 0 件で通り、`lsharp test` が message 無しで落ちる | 低 | resolved | 2026-08-23 |
| [I-63](#i-63) | `rename` の wire 形式も先頭要素の uri text だけで list 全体が切り替わる | 低 | resolved | 2026-08-23 |
| [I-64](#i-64) | `#[ignore]` の e2e が陳腐化した期待値を抱えたまま誰にも観測されない | 中 | resolved | 2026-08-24 |
| [I-65](#i-65) | selfhost runner は contract metadata の quote 契約を持たず、`:invariant` + quote を `pass` と報告する | 中 | resolved | 2026-08-23 |
| [I-66](#i-66) | EmbeddedCli の既定 test option が `--format json` と同値で、`run-test-source-text` が到達不能になっている | 低 | open | -- |
| [I-67](#i-67) | selfhost runner の `cases` / `coverage.executed` は pass 数を数えており、失敗時に rust runner と食い違う | 低 | resolved | 2026-08-23 |
| [I-68](#i-68) | `:invariant` / property の `cases` はサンプル数を載せており、rust oracle の contract 数と食い違う | 低 | open | -- |
| [I-69](#i-69) | `repl-session-last-type-name` が型タグを見ずにスロット 1 を読み、`lsharp repl` が壊れた型名を出す | 中 | open | -- |
| [I-70](#i-70) | ADR の Evidence 節が `#[ignore]` 下の test を根拠にしており、赤に転じても訂正されない | 中 | resolved | 2026-08-24 |
| [I-71](#i-71) | stage-N 生成 Wasm が 3 つの固定 offset で `expected i64 but nothing on stack` になる | 高 | resolved | 空 do が値を積まないため。3 経路に `i64.const 0` を emit。**症状は消えたが赤は減らず、72 行は `I-72` へ付け替え** |
| [I-72](#i-72) | stage-N 生成 Wasm の import 数が 1 つ足りない (`expected 11 imports, found 10`) | 高 | resolved | 2026-08-27。harness を 11-import へ統一。台帳 88 行中 80 行が緑 |
| [I-73](#i-73) | native differential の exact-byte pin 33 件が一律にずれている | 中 | open | -- |
| [I-74](#i-74) | root lifetime verifier が `main` 以外の helper の `depth: 1` を拒否する | 中 | open | -- |
| [I-75](#i-75) | sweep で露出した未分類の赤 11 件 | 中 | open | 8 件を 2026-08-27 に `I-72` / `I-76` / `I-78` / `I-80` / `I-84` / `I-90` へ移管。残り 11 件は症状を実測して台帳へ書いた |
| [I-76](#i-76) | `check` の型名出力は program 型を返すので、式の型を検査する test が成立しない | 中 | open | -- |
| [I-77](#i-77) | e2e の Wasm 検証ヘルパーが関数本体を一つも検証していない | 高 | open | -- |
| [I-78](#i-78) | stage1 compiler が `src/App/Cli.ls` の self-feed compile で `integer divide by zero` trap する | 中 | open | `I-75` から分離 (2026-08-27)。`I-72` 解決後に赤 3 件 |
| [I-79](#i-79) | 実行失敗で assertion が skip される test が 3 件あり、緑のまま何も検査していなかった | 中 | resolved | 2026-08-27 解決。起票時の「8 件」は分類が誤っていた |
| [I-80](#i-80) | target-defn probe が AST の形を添字直打ちで辿り陳腐化している | 中 | open (実装は 2026-08-27 に完了。lane 再計測待ち) | test 側 3 件を是正。marker 129 以降の初回評価で新規赤は 0。副産物が `I-88` |
| [I-81](#i-81) | `local_bound_violation_indices` が 0 件になり、violation 前提の診断足場が落ちる | 中 | open | `I-72` 解決後に露出 (2026-08-27)。同日、極性を反転し改名。**lane 再計測待ち** |
| [I-82](#i-82) | test 名が主張する主題を検査していない probe test が 13 件あり、常に緑になる | 中 | open | `I-79` の全数調査で発見 (2026-08-27)。**同日 12 件を是正。残るは #13 の 1 件** |
| [I-83](#i-83) | compiler-mode が生成した wasm が stack 不整合で load できない | 高 | open | `I-79` の是正で初めて実測 (2026-08-27) |
| [I-84](#i-84) | 構造上必ず赤くなる test が 5 件、台帳に恒久的な赤として載っている | 中 | open | `I-81` の裁定中に走査で発見 (2026-08-27)。うち 1 件は `I-75` が誤分類。**5 件のうち 1 件 (#1) は `I-81` として同日決着**。残り 4 件 |
| [I-85](#i-85) | `test_debug_boot04_*` 12 件の主題 assertion が `!output.trim().is_empty()` だけ | 中 | open | `I-82` の裁定 5 を書く途中で発見 (2026-08-27)。同日 12 件とも実質化。**lane 再計測待ち**。副産物が `I-87` |
| [I-86](#i-86) | selfhost parser が Rust reference より緩く、不正な構文を `diagnostics:0` で受理する | 中 | open | `I-82` の #7 を実測して発見 (2026-08-27)。2 引数 `if` と top-level のゴミ atom の 2 形 |
| [I-87](#i-87) | WASI 経路の `read-file` が preopen 外のパスに対しエラーではなく空文字列を返す | 中 | open | `I-85` の是正中に発見 (2026-08-27)。fixture が読めていないのに test が緑になっていた |
| [I-88](#i-88) | target-defn probe の body ナビゲーションが旧 shape 前提のままで、下流 marker が「壊れていること」を pin している | 低 | deferred | `I-80` の却下案 B の代償を記録したもの (2026-08-27) |
| [I-89](#i-89) | x86 の 20 引数以上 param spill テーブル約 680 行が到達不能で、旧 slot 規約のまま残っている | 低 | open | `I-73` の受入条件 (a) を調べる途中で発見 (2026-08-27)。aarch64 側の同型 chain は生きている |
| [I-90](#i-90) | selfhost LSP の framed response が 0 origin Position を返すのに、test 2 件の期待値が 1 origin になっている | 中 | open | `I-75` の 14 件を分類する中で診断確定 (2026-08-27)。line も character も一律 +1 |

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
- **gate** (2026-08-23 に導入。`RUST-FILE-SIZE-GATE-01` の完了):
  `crates/lsharp-wasm/tests/rust_file_size_contract.rs` が `crates/**/src/**` と
  `crates/**/tests/**` を走査し、800 行超の実測集合が
  `tests/rust-file-size-allowlist.txt` (src 6 件) /
  `tests/rust-test-file-size-allowlist.txt` (tests 33 件) と
  **双方向で一致する**ことを要求する。分割で 800 行以下になった file を list から
  消し忘れても落ちるので、**list は単調減少しかしない**。
  導入前は per-file の targeted guard 8 本 (`*_file_size.rs`) だけで、
  新しく超過した file を検知できなかった。

  検証は 4 段階 (RED / GREEN / 負の対照 2 本) を実測した。判断と却下理由は
  [`decisions-rust-file-size-gate.md`](docs/adr/decisions-rust-file-size-gate.md)。
  **allowlist への追加を機械的には禁止できていない** — 受入条件の後半を満たせなかった
  事実として同 ADR に記録した。
- **状態が `in-design` のままである理由**: gate は入ったが、**超過 39 件そのものは
  1 件も減っていない**。分割は `LEGACY-MAINT-01` が持つ。gate の導入を
  「file-size 問題の解決」と読まない。
- **関連**: selfhost 側は ADR-168 (STR-01〜03) で分割実績あり (TypeInfer.ls 1093 → 290 行など)。
  Rust 側の分割設計は [imp-06](docs/development/planning/improvement-designs/imp-06-large-file-decomposition.md)。
  分割そのものは `LEGACY-MAINT-01`。gate は 2026-08-23 に導入済み (上記)。
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
- **`LINT-SPAN-01` の実装は 2026-08-23 に完了した**: `let` は束縛識別子、`do` は `do` トークンの
  byte offset を末尾 pair で持ち、投影境界 `lsp-review-diagnostic-to-lsp` が
  `lsp-position-from-offset` で実 range を作る。fixture `(defn main [] (let [unused (do)] 0))` の
  2 診断は `0:20..0:26` と `0:28..0:30` になり、受入条件 1 を満たした。
  焦点 test は 13 本緑 (`683.42s`)、判別力のある 11 本は事前に旧実装で赤を観測している。
  **受入条件 2 は宣言どおり vacuous になった** — `..._preserves_distinct_same_start_diagnostics` は
  pass するが、この fixture の 2 診断はもはや同一開始位置ではないので dedup 意味論の pin ではない。
  pin の再建は `I-58` が持つ。詳細は同 ADR の「受入条件 1 / 2 / 4」節。
  なお実装中に、受入条件 3 の survey が **Rust 側 test の長さ pin を対象にしていなかった**ことが
  判明した (`selfhost_doctools_cli_diagnostics.rs:547/597` が review diagnostic の長さ 7 を pin していた)。
  survey は `selfhost/src/**.ls` だけを走査していた。同種の survey を再び行うときは
  test 側の pin も対象に含める。
  広域 sweep (511 本) では 4 本赤が出たが、`.ls` を `HEAD` へ戻した再実行で左右の実測値まで
  一致したので、**本 slice の回帰は 0 件**である。未登録の 3 本は `I-45` の未計測 fallout として
  `I-60` へ切り出した。
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

- **影響度**: 中 / **状態**: resolved (2026-08-23) / **発見**: 2026-08-22 (`WORKTREE-ABSORB-02` の判定中)
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
  `STATIC-CONTRACT-01` が引き取り、2026-08-23 に完了して `TODO.md` からは削除した。
  判定は [worktree 取り込み判定](docs/adr/decisions-worktree-absorption-2026-08-20.md)。
- **解決** (2026-08-23): `static_boolean_result` を `static_boolean_result_in(expr, env)` へ
  作り替え、`if` / `let` / `do` / `match` を貫通させた。`env` は「静的に値が決まる束縛」の
  スタックで、後ろから引くことで内側の再束縛が外側を覆う。

  設計上の判断が 3 つある。

  1. **`if` は条件が静的に決まらなくても、両枝が同じ値へ落ちるなら確定させる。**
     片枝だけを見ると `(if flag true true)` が漏れる。`match` には評価できる条件が無いので
     **常に全 arm の一致**を要求する。
  2. **`let` / `match` が演算子名 (`not` `and` `or` `=` `==` `!=` `<` `>` `<=` `>=`) を
     再束縛したら判定を諦める** (`None` を返す)。builtin の意味で計算すると
     `(let [not f] (not false))` を vacuous と誤診する。
  3. **pattern が束縛する名前は「値不明」として env へ積む。**
     積まないと外側の静的束縛が arm body へ漏れる。

  test は `crates/lsharp-types/src/canonical_contract_check/tests.rs` に 22 件
  (`cargo test -p lsharp-types --lib canonical_contract_check` → 23 passed / 0 failed)。
  内訳は control 2 / `:assert` 貫通 4 / precondition 貫通 4 / 非空虚性の negative control 4 /
  shadowing の negative control 2 / 束縛値追跡 5 / fixture 訂正 1。
  非空虚性は `lookup_static_binding` の `.rev()` を外す破壊で
  `..._let_rebinding_shadows_outer_static_value` と
  `..._match_arm_binding_shadows_outer_static_value` の 2 件が落ちることを実測して確認した。
- **証拠表の訂正** (2026-08-23): 上表の `:assert [(match true (true true))]` は
  **fixture の不備**であって穴ではない。`parse_match` (`crates/lsharp-syntax/src/parser/expr.rs:249`)
  は arm に `[` を要求するので、この式はそもそも parse できず、診断が 0 件なのは当然だった。
  正しい形は `(match true [_ true])`。この訂正は
  `static_contract_issue_table_paren_match_arm_fixture_does_not_parse` で固定してある。
  `:precondition` も同様に vector を取る (`crates/lsharp-syntax/src/parser/metadata.rs:483`)
  ので、`:precondition false` と書いた probe は parse error になり RED の意味を持たない。

<a id="i-43"></a>
### I-43: `:example` / `:invariant` / `:doc` の識別子検査が false positive を出す

- **影響度**: 中 / **状態**: resolved (2026-08-23) / **発見**: 2026-08-22 (`WORKTREE-ABSORB-02` の判定中)
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
  `CONTRACT-SCOPE-01` が引き取り、2026-08-23 に完了して `TODO.md` からは削除した。
  判定は [worktree 取り込み判定](docs/adr/decisions-worktree-absorption-2026-08-20.md)。
- **解決** (2026-08-23): branch の別入口 (`check_metadata_from_contract_inventory`) は使わず、
  main の `check_metadata` へ直接 3 点を当てた。

  | 症状 | 直した場所 |
  |---|---|
  | ADT variant / trait method が未定義扱い | `metadata_check.rs` に `collect_nested_names` を足し、`TypeDef` の variant 名と `TraitDef` の method 名を `all_names` へ入れた。`private` で包まれていても辿る |
  | quote されたシンボルが参照扱い | `references.rs` に `collect_unquoted_references` を足し、`Expr::Quote` の内側は `~` / `~@` で戻された部分式だけを参照として拾うようにした。入れ子の quote は素通しする |
  | builtin の `:doc` バッククォート参照が Warning | `diagnostics.rs` の doc 検査に `is_builtin` の skip を足し、`:invariant` / `:example` と扱いを揃えた |

  test は `crates/lsharp-types/src/metadata_check/diagnostics_tests.rs` に 11 件
  (control 1 / 受入 6 / negative control 4)。crate 全体は 255 passed / 0 failed、
  `cargo clippy -p lsharp-types --all-targets` は clean。
  非空虚性は 3 点をそれぞれ破壊して確認した — doc の skip を外すと 1 件、
  `collect_nested_names` を空にすると 3 件、quote の内側を全部拾うと 3 件が落ちる。
  `all_names` を広げた影響範囲も測った — `未定義の識別子` の文字列で分岐している
  `crates/lsharp-tooling/src/metadata_test.rs:65` を含む `cargo test -p lsharp-tooling --lib` は
  145 passed / 1 failed で、その 1 件は既知の `api_doc::tests::test_build_api_doc_for_file_preserves_parse_error_code`
  (`docs/development/validation/workspace-expected-failures.txt:139` に記録済み) のみ。
  `cargo test -p lsharp-types` (integration binary 込み) は exit 0。
  測ったのはこの 2 crate だけで、driver / wasm 側の metadata 消費者は含まない。
- **証拠表の訂正** (2026-08-23): 上表の `:invariant (= c Red)` は識別子スコープを直しても
  0 件にはならない。`=` が Int 比較なので `(= c Red)` は**型推論**で落ちる。
  これは `I-43` の穴ではないので、test 側の fixture を
  `(= (code Red) 0)` (variant を関数へ渡す形) へ改めた。
  同様に `:invariant (= 'sym 'sym)` も識別子エラー 2 件は消えるが、
  quote を扱えない型推論のエラーが 1 件残る。これは `I-59` として別に立てた。

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
- **残る差分の解消** (2026-08-23): preflight 3 経路 (assertion / case / property) の
  診断 `message` を `(code, span)` から合成するようにした。selfhost は識別子を name-hash で
  持ち hash から文字列へ戻せないため、シンボル名は **span でソース本文を切り出して**載せている。
  実測は `[LS1001] contract の述語が型検査を通りません: (> (nope) 0) (25..37)`。
  判断と却下理由は [decisions-selfhost-preflight-diagnostic-message.md](docs/adr/decisions-selfhost-preflight-diagnostic-message.md)。
  **なお残るもの**: message の文字列は Rust oracle
  (`[LS1001] [error] caller: :assert predicate の型推論に失敗しました: ...`) と逐語一致しない。
  逐語一致は当該 ADR で明示的に却下している。`run-test-source-text` lane の message も空のまま。
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

- **影響度**: 中 / **状態**: resolved / **発見**: 2026-08-22 (`I-49` の slice を閉じる際の sweep)
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
  (2026-08-23 に解決済み — facet B の節を参照。`"col":23` の疑義は、同 test が反復間比較で
  あるため問題にならないことが緑で確認できた。)
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
- **解決の第二段** (2026-08-23): initialize の capabilities を object 形へ転記した。
  `initialize.json` / `initialize-shutdown-sequence.json` の `[1,1,1,1,1,1,1]` と、
  `..._lsp_stdio_wire_repeated_sequence` の inline 期待値 (`selfhost_cli_core.rs:6245`) を
  `{"capabilities":{...}}` へ書き換えた。7 個の `1` は
  `completionProvider` / `definitionProvider` / `documentFormattingProvider` /
  `hoverProvider` / `referencesProvider` / `renameProvider` / `textDocumentSync` に
  一対一で対応し、**能力の集合としては同一**である。
  検証: `cargo test -p lsharp-wasm --test e2e -- --ignored lsp_stdio_initialize
  lsp_stdio_wire_repeated_sequence` で **5 passed / 0 failed** (`310.86s`)。
  `..._wire_repeated_sequence` もこれで緑になった — 同 test は frame0 の initialize だけを
  絶対値で比較し、hover / completion は**反復 1 回目と 2 回目以降を突き合わせる**構造なので、
  位置の絶対値を要求していなかった。`I-52` で疑義として残していた `col:23` は問題ではない。
- **解決の第三段** (2026-08-23): diagnostics の縮約形を object 形へ転記した。
  snapshot 3 ファイル (`document-sequence-diagnostics-refresh.json` /
  `document-sequence-type-diagnostics-refresh.json` /
  `document-sequence-lint-diagnostics-refresh.json`) と inline 6 箇所
  (`selfhost_cli_core.rs` の `open_diagnostics` 定義) を書き換えた。
  **inline は当初 3 本と見積もっていたが実際は 6 箇所だった** — `lsp_transport_document_sequence_*`
  3 本が同じ縮約形のまま残っており、lane 監査の filter (`lsp_stdio`) に掛からないので
  `I-53` の 64 FAIL に現れていなかった。転記前に 9 本すべての左右をログで突き合わせ、
  位置の値が緑の契約 test と一致することを確認した
  (`:18664` の type `character:14`、`:18721` の lint `0,0`)。
  検証: `cargo test -p lsharp-wasm --test e2e -- --ignored lsp_stdio_document_sequence
  lsp_stdio_body_document_sequence lsp_transport_document_sequence` で
  **15 passed / 0 failed** (`738.62s`、2026-08-23)。
  これで facet B のうち**転記だけで解決する系統は打ち止め**である。残る 22 本は位置起因なので
  `I-55` / `LSP-COL-CONV-03` が先に要る。
  (このうち 5 本 = completion 3 + `filesystem_document_sequence` 2 は第四段で解決した。
  nav 系 17 本は `I-55` の第一〜第三段で解決済み。)
- **解決の第四段** (2026-08-23): 位置起因と形式ドリフトが重畳していた 5 本を、
  **位置を先に直し、実測してから転記した**。`LSP-COL-CONV-03` の完了。

  | test | 位置の修正 | snapshot file |
  |---|---|---|
  | `..._completion_changed_document_schema_snapshot` | `"line":1,"col":23` → `"line":0,"col":22` | `completion-changed-document.json` |
  | `..._completion_latest_reopened_schema_snapshot` | `"line":1,"col":21` → `"line":0,"col":20` | `completion-latest-reopened.json` |
  | `..._completion_filesystem_import_schema_snapshot` | `find+len+1`, `"line":1` → `find+len`, `"line":0` | `completion-filesystem-import.json` |
  | `..._filesystem_document_sequence_schema_snapshot` | 5 frame の `"line":1` → `0`、`symbol_col` = `find+1`、`completion_col` = `find+len` | `filesystem-document-sequence.json` |
  | `..._filesystem_document_sequence_spec_style_snapshot` | 同上 (spec 形の `"position"` 5 箇所) | 同上 (**兄弟と同一 file を共有**) |

  **兄弟 2 本が同じ snapshot file を読む**ので、`completion_col` の式が食い違っていた
  (`+ 1` の有無) のは単なるバグである。片方を直して他方を放置すると、どちらの式が
  正しくても必ず一方が落ちる。
  転記した差分は 4 file 分あり、**いずれも item の集合は左右で一致し、形だけが違う**ことを
  機械比較で確認した — completion `["helper",3,"helper"]` → object、
  diagnostics の型タグ → LSP Diagnostic、hover の `range:[1,51,1,58]` → object `0,50`〜`0,57`、
  rename の 5-tuple → TextEdit object。
- **転記中に見つかった別系統** (`I-57` へ分離): `definition` / `references` の response だけが
  縮約 array のままで、しかも line / col とも内部の 1 始まりが漏れている。
  **fixture と実装が一致しているので assert は緑であり、転記では捕まらない。**
  「転記は規約の検査にはならない」ことの実例なので、黙って写さず issue を切った。
- **第四段の検証** (`cargo test -p lsharp-wasm --test e2e -- --ignored
  lsp_stdio_completion_changed_document_schema_snapshot
  lsp_stdio_completion_latest_reopened_schema_snapshot
  lsp_stdio_completion_filesystem_import_schema_snapshot
  lsp_stdio_filesystem_document_sequence`、2026-08-23):
  **5 passed / 0 failed** (`477.09s`)。RED は同日の `I-56` GREEN run
  (`13 passed; 5 failed`) で同 5 本が `selfhost_cli_core.rs:84` で落ちることを実測している。
- **転記できるのは 31 本中 3 本だけだった**。`I-53` の lane ログの左右を機械比較したところ、
  値まで一致する (= 純粋な形式ドリフト) のは以下だけである。

  | 分類 | 本数 | 引き取り先 |
  |---|---|---|
  | 形式のみ (completion / initialize) | 3 | 2026-08-23 転記済 (`I-52` 第一段 / 第二段) |
  | 形式のみ (diagnostics の縮約形) | 6 (`document_sequence_*_diagnostics_refresh_snapshot`) | 2026-08-23 転記済 (`I-52` 第三段) |
  | 位置起因で値が違う | 20 (nav 系 17 + completion 3) | `I-55` (nav 17) / `LSP-COL-CONV-03` (completion 3、第四段で解決) |
  | 位置と形式の重畳 | 2 (`filesystem_document_sequence_*`) | `LSP-COL-CONV-03` (第四段で解決) |

  initialize 2 本は値の比較では差が出る (`[1,1,1,1,1,1,1]` 対 6 個の `Bool(true)` +
  `textDocumentSync:1` + `completionProvider:{}`) が、**能力の集合としては同一**であることを
  目視で確認したので形式ドリフトに分類した。機械比較は一次選別にすぎず、目視が要る。
  diagnostics 6 本は当初「内容差」に分類したが、2026-08-23 の再検で**形式ドリフトと確定した**
  (`I-54` 参照)。**残り 22 本は「転記すれば緑になる」ものではない。** 原因を先に解く。

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
- **状態の内訳**: facet A / facet B ともに resolved (2026-08-23)。
  **`assert_lsp_stdio_snapshot` を経由する 31 本は全て決着した** —
  形式のみ 9 本 (第一〜第三段)、位置と形式の重畳 5 本 (第四段)、
  残る 17 本は nav 系で `I-55` が持つ。
- **関連**: `I-49` (発見の経緯)。引き取り先は `TODO.md` の `LSP-SNAPSHOT-SHAPE-01` (B) と
  `LSP-COL-CONV-03` (snapshot file を読む 3 本の位置修正。B の解消と同時にしか検証できない)。
  wire 位置規約の正本は `AGENTS.md`。

<a id="i-53"></a>

### I-53: `lsp_stdio` lane 93 本のうち 64 本が赤で、`I-52` の補完 9 本では説明できない

- **影響度**: 中 / **状態**: resolved / **発見**: 2026-08-23 (`I-52` の「未監査」を潰すための lane 全体監査)
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
  引き取り先は `TODO.md` の `LSP-COL-CONV-03` / `LSP-COL-CONV-04` / `LSP-NAV-DEGRADE-01`。
  B のうち転記だけで解決する 9 本は 2026-08-23 に完了した (`I-52` の第一〜第三段)。
- **解決** (2026-08-23、`LSP-LANE-REVERIFY-01`): lane 全体を同じ filter で再計測し、
  **93 passed / 0 failed** を実測した。64 FAIL は 4 issue への分解と個別修正で全て消えている。

  ```
  cargo test -p lsharp-wasm --test e2e -- --ignored lsp_stdio
  test result: ok. 93 passed; 0 failed; 0 ignored; 0 measured; 2981 filtered out;
  finished in 5076.88s
  ```

  ログは `/Users/biwakonbu/github/tmp/lsp-stdio-lane-reverify/lane.log` (`EXIT=0`)。
  **計測したのは HEAD `94f54bb7` 時点の code state である。** run 中に landed した
  `3216ace2` は docs のみで test binary に影響しない。一方、run 中に working tree へ入れた
  `LINT-SPAN-01` の未 commit 変更 (`selfhost_cli_core.rs` の期待値 7 箇所 + `.ls` 3 本) は
  **この lane の結果には含まれない** — test binary は launch 時点で build 済みだったため。
  したがって本 lane は `LINT-SPAN-01` 適用後の状態に対する保証ではない。
  所要は 4346.02s → 5076.88s に伸びたが、これは同時に走っていた別 job の CPU 競合による。

<a id="i-54"></a>

### I-54: LSP の response 側の位置が wire 変換前の内部値で fixture に固定されている

- **影響度**: 中 / **状態**: resolved / **発見**: 2026-08-23 (`I-53` の lane 監査)
- **内容**: `I-52` facet A は **request 側**の位置規約のずれだった。同じずれが
  **response 側**にもあり、しかも向きが test ごとに違う。どちらも fixture が
  `9175c6e5` (2026-08-03) の wire 正規化に追随していないことの表れである。

  | test | 実測 (left) | 期待 (right) | 向き |
  |---|---|---|---|
  | `..._lsp_stdio_body_hover_spec_position_character_params` | `["42","2","39"]` | `["42","1","38"]` | 実装が内部値 (1 始まり) を返す |
  | `..._lsp_stdio_body_rename_spec_position_character_params` | `["42","2","39","cube"]` | `["42","1","38","cube"]` | 同上 |
  | `..._main_with_lsp_stdio_formatting` | `line:0,character:0` 〜 `0,16` | `line:1,character:1` 〜 `1,17` | 期待が 1 始まりで陳腐化 |

- **body params 2 本の帰結** (2026-08-23、**fixture を直した / 実装は触っていない**):
  この 2 本が見ているのは `lsp-stdio-nav-params` の変換**後**の内部値である。
  fixture は `line` については変換後の値 (`1`) を、`col` については変換前の wire 値 (`38`) を
  期待しており、**同じ vector の中で座標系が混ざっていた**。`+1` は line と col の双方に
  等しくかかるので、内部整合する読みは `["42","1","39"]` の一方しかない。
  `+1` が wire → 内部の正規変換であることは緑の contract test 2 本
  (`..._zero_based_position_contract` / `..._standard_uri_navigation_contract`) が押さえている。
  よって **fixture 側が陳腐化している**と判定し、`38` → `39` へ直した。
  request の `"line":1` → `"line":0` は `I-55` 第一段で同時に直っている。
  検証は `I-55` の 40 本ランに含まれる (`ok. 40 passed; 0 failed`)。
- **formatting 系の帰結** (2026-08-23): 当初 5 本と数えていたが、実測で **2 群に割れた**。
  `..._main_with_lsp_stdio_formatting` (inline `source` を送る 1 本) だけが純粋な fixture の
  陳腐化で、残る 5 本 (didOpen 済み document を `uri` だけで参照するもの) は
  **実装バグだった** — `I-56` へ分離した。

  前 2 本は `lsp-stdio-nav-params` が返す**変換後の内部値**を test が直接見ており、
  後者は response range が wire (0 始まり) へ正規化されたのに fixture が追随していない。
  **「どちらも off-by-one」で丸めると向きが逆であることが消える**ので、行ごとに向きを記録する。
- **diagnostics 系 9 本の判別** (2026-08-23、実測ログの左右を読んで確定):
  **位置の問題ではなく `I-52` facet B と同じ形式ドリフトである。** 期待値が
  2026-04-03 時点の縮約形 (型タグ) のまま止まっており、実出力は LSP 準拠の
  Diagnostic object になっている。

  ```
  left  (実測): {"range":{"start":{"line":0,"character":0},"end":{"line":0,"character":1}},
                 "severity":1,"code":"LS0101","source":"lsharp","message":"unexpected token )"}
  right (期待): {"source":1,"severity":1,"rule":1001,"line":1,"col":1,"messageHash":0}
  ```

  対応は取れている — `rule:1001` は `code:"LS0101"`、`source:1` は `"lsharp"`、
  `line:1,col:1` (1 始まり) は `start {line:0,character:0}` (0 始まり) に対応する。
  `end` は縮約形が持っていなかった情報である。
  **現行契約が object 形であることは緑の test が押さえている** —
  `..._lsp_stdio_didopen_publishes_standard_parse_diagnostic` (`:18757`) は
  `{"range":..,"severity":1,"code":"LS0101","source":"lsharp","message":..}` を
  逐語で期待して pass している。
  内訳は inline 3 本 (`..._body_document_sequence_spec_params_publishes_*_refresh`) と
  snapshot 6 本 (`..._document_sequence_*_diagnostics_refresh_snapshot`)。
  **解決** (2026-08-23): 転記前に 9 本すべての左右をログで突き合わせ、object 形へ書き換えた。
  併せて `lsp_transport_document_sequence_*` 3 本の同型ドリフトも見つかり、計 12 本を修正した。
  検証は `I-52` facet B の「解決の第三段」に記録した (15 passed / 0 failed)。
- **`..._main_with_lsp_stdio_formatting` の帰結** (2026-08-23、**fixture を直した**):
  inline `source` を送る 1 本。response range は wire (0 始まり) へ正規化されているのに
  期待だけが `1,1`〜`1,17` で止まっていた。`start 0,0` / `end 0,16` へ直し、併せて
  5-tuple の期待を LSP TextEdit object へ転記した。
  検証は `I-56` の解決節のランに含まれる (`..._lsp_stdio_formatting` は `ok`)。
- **修正方針**: 位置の 7 本は fixture を直す。ただし**向きが逆の 2 群を同じ理由では直せない**ので、
  行ごとにどちらが正本かを決める。diagnostics 9 本は形式ドリフトとして転記する。
- **各行の帰結** (どちらを正本としたか):

  | 対象 | 直した側 | 理由 |
  |---|---|---|
  | `..._body_hover_spec_position_character_params` | fixture | 変換**後**の内部値を見る test。同じ vector 内で座標系が混ざっていた |
  | `..._body_rename_spec_position_character_params` | fixture | 同上 |
  | `..._main_with_lsp_stdio_formatting` | fixture | 実装の response は既に wire (0 始まり)。期待だけが陳腐化 |
  | formatting 5 本 (didOpen 参照) | **実装** | 陳腐化ではなく params slot のずれ (`I-56`) |
  | diagnostics 9 本 | fixture | 位置ではなく形式ドリフト。値の対応は取れている |
- **状態**: 位置 7 本 / diagnostics 9 本ともに決着した。**resolved**。
- **関連**: `I-52` (request 側の同型問題、facet A で resolved)、`I-53` (実測の出所)、
  `I-56` (formatting 5 本の実装バグ)。
  引き取り先だった `TODO.md` の `LSP-COL-CONV-04` は削除済み。

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
- **したがって残る作業は fixture の書き換えだけ**である。当初は「request を直すまで
  response の座標が測れないので多段になる」と見ていたが、**references / rename については
  多段が要らないことが 2026-08-23 に分かった**。lane ログの左右を並べると:

  ```
  left  (実測): "result":[[42,2,39]]
  right (期待): "result":[[42,1,7],[42,1,36],[42,1,47]]
  ```

  応答は **triple 形のまま**で形式ドリフトしておらず、期待値の `col` (7 / 36 / 47) も
  source `(defn square [x] x) (defn main [] (square 1) (square 2))` の
  0-based 6 / 35 / 46 に一対一で対応する内部 1 始まり値である。つまり**期待値は既に正しい**。
  request 側も `col:38` は `+1` を経て内部 39 = 0-based 38 となり `square` (0-based 35..40) の
  内側に当たる。**外れているのは `"line":1` だけ**で、1 行の文書に対して内部 line 2 を
  指してしまっている。修正は `"line":1` → `"line":0` の 1 箇所である。
  hover は fallback が `contents:"type-info:2:39"` という別形を返すので、**別に確認する**。
  なお実測ログから抽出した nav 系 40 本の左右は
  `/Users/biwakonbu/github/tmp/lsp-stdio-lane-red/nav_left_right.txt` にある。
- **修正方針**: fixture を wire 規約 (0 始まり) へ直す。実装には触らない。response 側の期待値は request 修正後の実測で決める。
- **解決** (2026-08-23): 上記の方針どおり fixture だけを直した。**実装は 1 行も触っていない。**
  二段で進めた。

  | 段 | 直したもの | 件数 | 結果 |
  |---|---|---|---|
  | 第一段 | request の `"line":1` → `"line":0` | 40 箇所 | 40 本中 15 本 (definition / references 系) が緑へ |
  | 第一段 | nav 4 本に残っていた diagnostics の縮約形 → object 形 | 4 箇所 | -- |
  | 第二段 | hover の response 期待値 (`1,36`〜`1,42` → `0,35`〜`0,41`) | 6 箇所 | -- |
  | 第二段 | rename の response 期待値 (5-tuple → LSP TextEdit object) | 5 箇所 | -- |
  | 第二段 | `definition` / `hover` の open-document 2 本に欠けていた publishDiagnostics frame | 2 箇所 | -- |
  | 第二段 | snapshot file の転記 | 11 file | -- |

  **hover の response は推測で書かなかった。** 第一段の実測で `contents:"type-info:2:39"` /
  `range -1,-1` という miss 時の fallback が消え、`contents:"defn helper"` と正しい span が
  返ることを確認してから期待値を決めた。すなわち hover の退化も request の座標系に
  起因しており、**実装退行ではなかった**ことが実測で裏付けられた。

  snapshot 11 file は転記前に左右を機械比較し、**全件が「同じ span を 0 始まり + LSP object 形へ
  書き直しただけ」**であることを確認した。値そのものが動いたものは 1 件も無い。
  `definition-open-document.json` にだけ frame が 1 つ増えるが、これは main document
  `(helper 1)` に対する `LS0103 unknown form` (0,1〜0,7) で、top-level が defn でない以上
  正当な出力である。helper document (uri 11) 側に frame が出ないのも、
  「空 → 空は publish しない」という既存の clear 意味論と整合する。
- **検証** (2026-08-23): `cargo test -p lsharp-wasm --test e2e -- --ignored lsp_stdio_hover
  lsp_stdio_definition lsp_stdio_references lsp_stdio_rename lsp_stdio_goto_definition
  lsp_stdio_body_hover lsp_stdio_body_rename` →
  **`ok. 40 passed; 0 failed; 0 ignored; 3034 filtered out; finished in 3608.39s`**。
  第一段時点は `15 passed; 25 failed; 1693.35s` だった。
- **関連**: `I-52` (facet A、帰結仮説の元)、`I-53` (実測の出所)、`I-54` (response 側の位置)。
  引き取り先だった `TODO.md` の `LSP-NAV-DEGRADE-01` は完了につき削除した。

<a id="i-56"></a>
### I-56: `source` を持たない document request で params の slot がずれ、open document state が参照されない

- **影響度**: 中 / **状態**: resolved / **発見**: 2026-08-23 (`I-54` の formatting 5 本を潰す過程)
- **内容**: `lsp --stdio` に `{"uri":42}` だけの `textDocument/formatting` を送ると、
  直前の `didOpen` で登録した source を使わず、空の TextEdit
  (`range` が `line:-1,character:-1`、`newText:""`) を返す。
  同じ request に `"source"` を inline で載せた場合は正しく整形される。

  | test | 送る params | 実測 (left) |
  |---|---|---|
  | `..._lsp_stdio_formatting` | `uri` + `source` | `start 0,0` / `end 0,16` / 整形済み text |
  | `..._lsp_stdio_formatting_uses_open_document` | `uri` のみ (didOpen 済み) | `range -1,-1` / `newText:""` |
  | `..._formatting_uses_spec_document_text_with_escaped_quote` | 同上 | 同上 |
  | `..._formatting_uses_spec_document_text_with_unicode_escaped_quote` | 同上 | 同上 |
  | `..._formatting_preserves_defn_metadata` | 同上 | 同上 |
  | `..._formatting_open_document_schema_snapshot` | 同上 | 同上 |

- **原因** (source を読んで確定):
  `lsp-stdio-document-params` (`selfhost/src/App/Cli.ls:2022-2035`) は
  `source` も `text` も無いとき **source slot を詰めずに** path / uriText を push する。
  結果 params は `[uri, path, uriText]` となり、`[uri, source, path, uriText]` を前提とする
  以下の読み出しが 1 つずつずれる。

  - `lsp-has-document-param` (`LspServerCore.ls:305-306`) は要素数 > 1 だけを見るので、
    source が無い params でも 1 を返す
  - `lsp-document-src` (`:323-324`) は index 1 を返すので、**path (空文字列) を source として読む**
  - `lsp-session-document-src` (`:338-340`) はそのため `server-state-source-for-uri` の
    fallback へ入らず、`handle-formatting` (`LspServerNav.ls:1078-1088`) が空 source と判断して
    `handle-formatting-mock` を返す

  nav 系 (hover / definition / references / rename) は `lsp-stdio-nav-params` という
  別の parser を使い、source slot を常に固定位置へ詰めるので同じずれを踏まない。
  **したがってこれは formatting の request 経路に固有の実装バグである。**
- **fixture の陳腐化とは別物**: 同じ 6 本は response 形式のドリフト
  (`[[1,1,1,17,"..."]]` の 5-tuple → LSP TextEdit object) も同時に踏んでいる。
  形式だけ転記しても `range -1,-1` は消えないので、**実装の修正が先**である。
- **修正方針**: params の slot を常に固定長で詰め、`lsp-session-document-src` は
  inline source が空なら session state へ落ちる。実装を直す (fixture ではない)。
- **解決** (2026-08-23、**実装を直した**): 2 箇所。

  | file | 変更 |
  |---|---|
  | `selfhost/src/App/Cli.ls` (`lsp-stdio-document-params`) | `source` も `text` も無い body で `""` を push し、slot を常に固定長で詰める |
  | `selfhost/src/Tools/Lsp/LspServerCore.ls` (`lsp-session-document-src`) | slot の有無ではなく **inline source の長さ**で判定し、空なら `server-state-source-for-uri` へ落ちる |

  2 つ目が要るのは、1 つ目だけでは slot が `""` で埋まるため
  `lsp-document-src` が空文字列を「inline source あり」として返してしまい、
  didOpen 済みの内容が依然として無視されるからである。
  **slot のずれ (原因) と、空 source の扱い (帰結) は別の bug であり、片方だけでは緑にならない。**
- **検証** (`cargo test -p lsharp-wasm --test e2e -- --ignored lsp_stdio_formatting
  lsp_stdio_completion_changed lsp_stdio_completion_latest lsp_stdio_completion_filesystem
  lsp_stdio_filesystem_document_sequence`、2026-08-23):

  ```
  test result: FAILED. 13 passed; 5 failed; 0 ignored; 0 measured; 3056 filtered out; finished in 976.73s
  ```

  受入 5 本 (`..._formatting_uses_open_document` /
  `..._formatting_uses_spec_document_text_with_escaped_quote` /
  `..._formatting_uses_spec_document_text_with_unicode_escaped_quote` /
  `..._formatting_preserves_defn_metadata` /
  `..._formatting_open_document_schema_snapshot`) は**全て緑**。
  残る 5 failed は `I-52` 第四段の転記対象であり、本 issue とは別系統である
  (いずれも `selfhost_cli_core.rs:84` = `assert_lsp_stdio_snapshot` 内で落ちる)。
- **関連**: `I-54` (formatting 5 本の出所)、`I-53` (実測の出所)。
  引き取り先だった `TODO.md` の `LSP-DOC-PARAM-SLOT-01` は削除済み。

<a id="i-57"></a>
### I-57: `definition` / `references` の response だけ LSP Location ではなく縮約 array で、line / col とも内部の 1 始まりが漏れている

- **影響度**: 中 / **状態**: open / **発見**: 2026-08-23 (`LSP-COL-CONV-03` の snapshot 転記中)
- **内容**: `lsp --stdio` の response は method によって形式と座標系が割れている。
  同じ 1 つの document に対する同じ 1 slice の実測 (`filesystem-document-sequence.json`) で
  以下が並んで出る。

  | method | 実測 response | 形式 | line |
  |---|---|---|---|
  | `textDocument/hover` | `{"contents":"defn mid-val","range":{"start":{"line":0,"character":50},...}}` | LSP object | **0 始まり** |
  | `textDocument/rename` | `[[200,[{"range":{"start":{"line":0,...}},"newText":"mid-next"}]],...]` | LSP TextEdit | **0 始まり** |
  | `textDocument/formatting` | `[{"range":{...0 始まり...},"newText":"..."}]` | LSP TextEdit | **0 始まり** |
  | `textDocument/publishDiagnostics` | `{"range":{"start":{"line":0,"character":50},...},"code":"LS1001",...}` | LSP Diagnostic | **0 始まり** |
  | `textDocument/definition` | `[8091858770804166904,1,50]` | 縮約 array | **1 始まり** |
  | `textDocument/references` | `[[200,1,51],[8091858770804166904,1,50]]` | 縮約 array | **1 始まり** |

  (line 列と同様に **col も 1 始まり**である。下表を参照。)

  1 行の文書なので `line:1` は wire 規約 (0 始まり、`AGENTS.md`) では 2 行目を指す。
  **col も同じく 1 始まりである。** 同じ snapshot 内の rename が返す TextEdit と突き合わせると
  ずれが 1 文字ぶんであることが確定する。

  | document | rename TextEdit の start (wire) | definition / references の col |
  |---|---|---|
  | `uri:200` の `mid-val` 呼び出し | `character: 50` | `references` が **51** |
  | `Support.Mid` の `mid-val` 定義 | `character: 49` | `definition` / `references` が **50** |

  すなわち縮約 array は **内部表現 (line 1 始まり / col 1 始まり) が無変換で漏れている**。
  「col は 0 始まりと一致している」と読めるのは数値の偶然で、
  definition の内部 col 50 (定義側、wire 49) と hover の wire character 50 (呼び出し側) が
  たまたま同じ値になるだけである。**別の document の別の token を比べている。**
  形式も LSP の `Location` (`{uri, range}`) ではない。
- **根拠**: 2026-08-23 の実測。`tests/snapshots/lsp/stdio/definition-filesystem-import.json` /
  `references-filesystem-import.json` / `filesystem-document-sequence.json` の 3 file に
  この形が入っている。いずれも **fixture と実装が一致している** (assert は緑) ので、
  test では捕まらない。`I-55` / `I-52` の座標修正でも触れていない。
- **なぜ今まで出なかったか**: `I-52` facet A / `I-55` は **request 側**の座標系の話で、
  response 側は「実測に合わせて転記する」方針だったため、実測そのものが規約に反していても
  そのまま snapshot へ入る。転記は規約の検査にはならない。
- **`I-54` と何が違うか**: `I-54` は「fixture が内部値のまま陳腐化している」= 実装は正しい。
  本 issue は逆で、**fixture は実測どおりだが実装の出力が規約に反している**。
  したがって修正対象は実装であり、fixture は実装修正後に転記し直す。
- **原因** (2026-08-23、source を読んで確定。cargo は回していない):
  **`Location` object を出す経路は既にあり、しかも正しい。** 分岐しているのは
  `uri-text` (client が送った URI 文字列) の有無である。

  | 条件 | 経路 | 出力 |
  |---|---|---|
  | `server-state-uri-text-for-uri` が非空 | `lsp-render-location-json-with-uri` (`LspServerNav.ls:85-93`) | `{"uri":"..","range":{..}}`。`lsp-render-wire-range-json` を通るので **0 始まり** |
  | 同 が空 (uri が int のまま) | `lsp-render-location-frame` / `lsp-render-locations-frame` (`:76-83`, `LspServerCore.ls:505-506`) | `[uri, line, col]`。**変換なし = 内部 1 始まり** |

  snapshot の request は `"uri":42` / `"uri":200` のように **int を送る**ので、
  常に後者へ落ちる。`lsp-render-location-frame` は汎用の
  `render-rpc-int-vector-response-frame` (`JsonRpc.ls:200`) をそのまま呼んでおり、
  location という意味を持たない int vector として出力する。
  `lsp-render-location-json` (`:55-65`) も `vector-get` の値を素通しする。

  **したがって「wire 形式が client の送り方で変わる」という、より重い問題が下にある。**
  同じ document / 同じ token に対して、URI を文字列で送れば 0 始まりの `Location`、
  int で送れば 1 始まりの縮約 array が返る。座標系が request の書き方に依存する。
- **修正の切り分け**: 座標の漏れは `lsp-render-location-json`
  (`selfhost/src/Tools/Lsp/LspServerNav.ls`) と `lsp-render-location-frame`
  (`selfhost/src/Tools/Lsp/LspServerCore.ls`) の
  2 箇所で wire 変換 (`- 1`) を入れれば閉じる。**`make-location` 側では直せない** —
  `handle-rename` (`LspServerNav.ls:993-1000`) が同じ location vector の
  line / col を内部値として読んで TextEdit を組み立てるため、生成時に変換すると
  rename が壊れる。**変換は render 境界に置く**。
- **判断が要る点**: 縮約 array を廃して `Location` object へ一本化するかどうかは
  互換性の判断を含む。上記のとおり object 経路は既に実装されているので、
  「新形式の追加」ではなく「fallback の廃止」である。
  `definition` / `references` の consumer は現状 test しかないが、
  `lsp-offset-from-line-col` 等の内部 helper がこの形を前提にしていないかを先に見る必要がある。
  **本 issue では形式変更まで決めない。** 座標の 1 始まり漏れは規約違反として確定させ、
  形式は ADR で決める。
- **解決** (2026-08-23): render 境界 2 箇所へ wire 変換 (`- 1`) を入れた。
  `lsp-render-location-json` (`selfhost/src/Tools/Lsp/LspServerNav.ls:55-68`) は
  line / col を読む時点で減算する。`lsp-render-location-frame`
  (`selfhost/src/Tools/Lsp/LspServerCore.ls:504-511`) は 3 要素を先に読んでから
  変換済み vector を組み立てて `render-rpc-int-vector-response-frame` へ渡す
  (汎用の int vector 応答 helper に location の意味を持たせないため、そちらは触っていない)。
  `lsp-render-wire-range-json` と同じ規約に揃え、**clamp はしない**。
- **起票時に把握できていなかった波及範囲**: 起票時は snapshot 3 file と書いたが、実測では
  `tests/snapshots/lsp/stdio/` の **8 file / 10 frame** と、
  `crates/lsharp-wasm/tests/e2e/selfhost_cli_core.rs` に**インラインで直書きされた
  期待 wire 文字列 13 箇所**が同じ値を pin していた。後者には snapshot を持たない
  request id (7 / 10 / 61 / 67 / 68) が含まれる。判断ではなく同一問題の footprint なので
  起票し直さず、実測としてここに記録する。
- **ずれが 1 文字ぶんであることの裏取り**: 変換後、`references` の col 51→50 が
  同じ snapshot の rename TextEdit の `character:50` と一致し、`definition` の 50→49 が
  rename の 49 と一致する。上の「rename TextEdit の start (wire)」表の予測どおりで、
  単に数値を 1 引いただけではなく skew が閉じたことを示す。
- **検証**: `cargo test -p lsharp-wasm --test e2e -- --ignored lsp_stdio_definition
  lsp_stdio_references lsp_stdio_filesystem_document_sequence
  lsp_transport_goto_definition_frame lsp_transport_references_frame`。
  実装前 (fixture だけ wire へ書き換えた状態) が `0 passed; 22 failed` (1170.90s、
  差分はすべて line / col の 1 ずれのみ)、実装後が `22 passed; 0 failed` (1197.76s)。
  22 本はすべて `#[ignore]` 付きなので `--ignored` が要る。
- **残した範囲**: 縮約 array と `Location` object の**形式の分岐そのもの**は閉じていない。
  `I-61` へ分離した。
- **関連**: `I-52` (request 側の同型問題)、`I-54` (逆向きの不一致)、`I-55` (nav 系の座標)、
  `I-61` (本 issue が残した形式の分岐)。

<a id="i-58"></a>

### I-58: lint 診断の dedup 意味論を pin する test が、real span 導入と同時に前提を失う

- **影響度**: 低 / **状態**: resolved (2026-08-23) / **発見**: 2026-08-23 (`LINT-SPAN-01` の doc-RED)
- **内容**: `I-24` は「同一開始位置でも rule が異なる lint 診断は dedup しない」を裁定し、
  `test_e2e_selfhost_cli_lsp_stdio_didopen_preserves_distinct_same_start_diagnostics`
  (`crates/lsharp-wasm/tests/e2e/selfhost_cli_core.rs:18762`) がそれを pin している。
  この pin が成立しているのは、fixture `(defn main [] (let [unused (do)] 0))` に対して
  **全ての lint 診断が `0:0..0:0` へ潰れている**ためであり、
  「同一開始位置」は仕様ではなく `LINT-SPAN-01` が直そうとしている**バグの副作用**である。

  `LINT-SPAN-01` で real span を載せると、同じ fixture で

  | rule | span (offset) | wire range |
  |---|---|---|
  | `L0001` unused-let | 束縛識別子 `unused` = 20..26 | `0:20..0:26` |
  | `L0002` empty-do | `do` トークン = 28..30 | `0:28..0:30` |

  となり開始位置が一致しなくなる。test は expected 文字列を更新すれば pass するが、
  **検査していた意味論には触れなくなる**。`I-24` の裁定は pin を 1 本失う。

- **なぜ同じ形で作り直せないか**: `review-*-diagnostic` は 2 rule しかなく
  (`DocTools.ls:713` / `:729`)、それぞれ `let` (tag 7) / `do` (tag 9) という
  **互いに素な kind** に紐づく。したがって lint 同士で同一 span になる fixture は
  原理的に構成できない。残る手は type 診断と lint 診断を同一開始位置に置くことだが、
  `LS1002` の span は実測で if 式**全体**を指す (`selfhost_cli_core.rs:18754` が
  `(defn main [] (let [unused 42] (if 1 true false)))` に対し `0:31..0:48` を pin) ので、
  同一開始位置を作れるかは span 決定規則側の調査を要する。

- **根拠**:
  - 裁定の正本: [診断 dedup の rule identity](docs/adr/decisions-lint-diagnostic-dedup-identity.md)
  - dedup 実装: `selfhost/src/Tools/Lsp/LspServerNav.ls:1225-1245` (AC-209)
  - span 化の正本: [lint span の AST 表現](docs/adr/decisions-lint-span-ast-representation.md) 決定 6
- **これは `LINT-SPAN-01` を止める理由にはならない**。`0:0..0:0` は実利用者に見える不具合で、
  pin の副作用のほうを保存するのは本末転倒である。**pin の再建を別項目へ分けて追跡する**
  ことでカバレッジの黙殺を防ぐ。追跡は `LINT-DEDUP-PIN-01`。
- **2026-08-23 の追加調査: 当初の受入条件は満たせない**。`dedup-diag-same-span` は
  2 引数が非対称で、`dedup-find-span` (`LspServerNav.ls:1241-1246`) は
  「既に result にある診断」を第 1 引数、「これから入れる診断」を第 2 引数に渡す。
  第 1 引数が lint でない場合は **無条件に 1 (重複) を返す** ので、
  同一開始位置の type + lint ペアでは type が lint を吸収する。
  さらに `sort-diagnostics` の order key は `source*100000000 + ...` で
  type (`source=2`) が lint (`source=3`) より必ず先に並ぶため、実運用経路
  (`Cli.ls:1474` / `:1739` / `:1754`) ではこの吸収が**決定的に起きる**。
  したがって「type 診断と lint 診断を突き合わせる」攻め筋で書いた test は
  `I-24` の裁定と**逆の契約**を pin することになり、採れない。
  lint 同士で同一 span を作れないことと合わせ、**e2e ソース fixture 経由で
  この裁定を pin する手段は存在しない**。
  代替として `dedup-diagnostics` を直接呼ぶ関数レベル pin を採る。
  判断と却下理由、受入条件との差の扱いは
  [診断 dedup の rule identity](docs/adr/decisions-lint-diagnostic-dedup-identity.md)
  の追記節が正本。

- **解決** (2026-08-23): `dedup-diagnostics` を直接呼ぶ関数レベル pin
  `test_e2e_selfhost_lsp_dedup_diagnostics_keeps_distinct_lint_rules`
  (`crates/lsharp-wasm/tests/e2e/selfhost_cli_core.rs`) を 1 本置いた。
  rule 相違 → 2 件 / 完全一致 → 1 件 / end 相違 → 2 件 の 3 分岐を検査する。
  実測は `1 passed; 0 failed` (255.30s)。**RED は取っていない** — 実装は既に正しく、
  本 slice は pin の再建だからである。失敗力は分岐の向きを揃えないことで確保した。
  AN32j は削除せず、「開始位置が異なる 2 lint が順序どおりに publish される」検査として残す。

<a id="i-59"></a>
### I-59: `:invariant` の型推論が quote を扱えず、識別子検査を直しても診断が残る

- **影響度**: 低 / **状態**: resolved (2026-08-23) / **発見**: 2026-08-23 (`CONTRACT-SCOPE-01` の実装中)
- **内容**: `I-43` で `:invariant` の識別子スコープ検査から quote されたシンボルを外したが、
  その後段に走る型推論 (`check_legacy_invariant_types`) が quote を扱えず、
  `(defn caller [x] :invariant (= 'sym 'sym) x)` は
  `[E0001] 未定義の変数 (undefined): quote/unquote はマクロ展開後に使用できません` を
  **1 件出したままになる**。`:example` 側は型推論を走らせないので 0 件で通る。
- **根拠**: 2026-08-23、`crates/lsharp-types/src/metadata_check/diagnostics_tests.rs` の
  `contract_scope_quoted_symbol_in_invariant_is_accepted` で実測。
  この test は「識別子スコープ由来のエラーが残らないこと」だけを assert しており、
  型推論由来の 1 件は意図的に許している。
- **なぜ低いか**: `:invariant` に quote を書く実プログラムを確認できていない。
  診断メッセージ自体は正確で、黙って通るわけではない。
- **判断が要る点**: 直し方が 2 つあり、どちらも contract の意味論に関わる。
  (a) `:invariant` の型推論を quote 対応させる。(b) `:invariant` は `:example` と同じく
  型推論の対象外とする。**どちらが正しいかは本 issue では決めない。**
- **裁定** (2026-08-23): (a) も (b) も却下し、metadata 固有の Error を出す (c) を採った。
  決め手は「`:invariant` は `test_runner.rs:85` で生成ソースへ差し込まれて**実行される**」
  ことで、どちらの案も診断を消すのではなく `lsharp test` の lowering へ**後ろ倒しする**だけ
  になる。判断と却下理由は
  [`:invariant` に書かれた quote の扱い](docs/adr/decisions-invariant-quote-handling.md) が正本。
- **解決** (2026-08-23): `find_quote_span` (`metadata_check/references.rs`) を足し、
  `check_legacy_invariant_types` が `has_unknown_reference` skip と同じ位置で quote を検出して
  probe を組み立てずに metadata 固有の Error を 1 件返すようにした。
  見出しは「未定義の変数」ではなく
  「`:invariant` に quote/unquote は書けません (実行可能な contract であり、quote はマクロ展開後に残らないため)」
  になり、span は生成ソースではなく元の `:invariant` を指す。
  test は `contract_scope_quoted_symbol_in_invariant_reports_metadata_error` へ改名して
  受入条件の 3 つの assert (件数ちょうど 1 / 旧見出しを含まない / `:invariant` と quote に言及) を置いた。
  RED `0 passed; 1 failed` → GREEN `1 passed; 0 failed`、`cargo test -p lsharp-types` 全 binary 緑。
- **関連**: `I-43` の解決節、`CONTRACT-INVARIANT-QUOTE-01` (`TODO.md`)。

<a id="i-60"></a>
### I-60: 0 引数 defn の型を pin する e2e 5 本が `I-45` の契約変更で赤のまま放置されている

- **影響度**: 中 / **状態**: resolved / **発見**: 2026-08-23 (`LINT-SPAN-01` の広域 sweep 中)
- **内容**: `914bd9f1` (`I-45`) が `infer-defn-predeclared` の param-count 0 分岐を
  `(mk-fun (mk-unit) body-ty)` へ変え、0 引数 `defn` は `Unit -> body` として env へ登録される
  ようになった。**これは意図した契約変更である**が、変更前の型を pin していた e2e 5 本が
  赤に転じたまま、どの正本にも載っていない。

  | test | left (実測) | right (期待) |
  |---|---|---|
  | `e2e::selfhost_lexer_parser::test_e2e_selfhost_gadt_constructor_registers_refined_return_type` | `["0","3","0","16777216","0"]` | `["0","5","1","1","100"]` |
  | `e2e::selfhost_lexer_parser::test_e2e_selfhost_program_analysis_preserves_first_defn_type` | `["3","-9223372036853747496"]` | `["1","100"]` |
  | `e2e::selfhost_typeinfer_pipeline_bootstrap::test_e2e_selfhost_pipeline_complete_stages` | `3` | `1` |
  | `e2e::selfhost_main_module_determinism::test_e2e_selfhost_pipeline_macroexpand_typeinfer_integration` | `3` | `1` |
  | `e2e::strings_patterns_compiler_integration::test_e2e_selfhost_main_integration` | `"3"` | `"1"` |

  後半 2 本は起票後に見つけた。3 本目と同じく `Main.ls` の 5 要素 summary の
  `lines[28]` を読んでおり、`#[ignore]` も付いていない。
  5 本の fixture はいずれも 0 引数 defn (`(defn make-int [] (IntLit 1))` /
  `(defn main [] 42)` ×2)。tag 3 は `Fun` (`selfhost/src/Types/Type.ls:52-57` の
  `(vector-push base 3)`) で、`ty-name` / `type-app-arg` が Fun ノードの pointer slot を
  読むため `16777216` / `-9223372036853747496` という値が出ている。
- **根拠**: 2026-08-23、`LINT-SPAN-01` の広域 sweep (511 本, `507 passed; 4 failed`) で検出。
  `.ls` 3 本を `HEAD` へ戻した再実行 (`/Users/biwakonbu/github/tmp/lint-span-01/base4.log`,
  `0 passed; 4 failed; 80.07s`) でも左右の実測値まで一致したので、`LINT-SPAN-01` の回帰ではない。
- **なぜ台帳に無いか**: `914bd9f1` のコミットメッセージ自身が
  「**回していない**: stage chain の `#[ignore]` lane と workspace e2e lane (実測 5h38m)」と
  記録している。`workspace-expected-failures.txt` は 2026-08-16/17 の計測が正本で、
  その時点では 3 本とも緑だった。**したがって同ファイルへ追記してはならない** —
  baseline の意味 (「その計測時点で赤だった集合」) が壊れる。
- **`I-11` との違い**: `I-11` は「計測はしたが台帳に写していない」欠落。本件は
  「契約を変えたあと計測していない」欠落で、原因も直し方も別である。
- **直し方**: 3 本の期待値を `Unit -> body` の新契約へ張り直す。
  `decisions-selfhost-zero-arity-defn-type.md` が契約の正本なので、これは
  「実装に合わせて期待値を変える」禁止則の例外 (契約変更に追随する書き換え) に当たる。
  ADR の Evidence 節へ、この 3 本を追随させた事実を戻すこと。
- **これは下限である**: sweep が覆ったのは e2e 約 3,075 本のうち 511 本にすぎず、
  `914bd9f1` 以降 full lane は一度も回っていない。**5 本は確定した下限で、全数は未了**。
  次に full lane を回したときに確定させる。
- **解決** (2026-08-23): 5 本を新契約へ張り直し、`5 passed; 0 failed; 112.24s` を確認した
  (`/Users/biwakonbu/github/tmp/i60/green5.log`)。inline harness の 2 本は `ty-fr` /
  `type-fun-ret` で `Fun` を剥がしてから戻り型を pin し、**tag 3 も pin に残した**。
  summary を読む 3 本については `PipelineSmoke.ls:98-103` で slot 1 の意味を
  「値の型 (Fun なら戻り型) の名前ハッシュ」へ変えた。**slot 数は 5 のまま**である
  (print 回数を変えると `lines[30]` / `lines[31]` を読む別 test がずれる)。
  判断と実測値は `decisions-selfhost-zero-arity-defn-type.md` の Evidence 節が正本。
  `workspace-expected-failures.txt` へは予告どおり追記していない。
- **6 本目** (2026-08-23): `ASSERT-DIAG-MESSAGE-01` の回帰 lane
  (`cargo test -p lsharp-wasm --test e2e selfhost_cli_actual_main_args`) で
  `test_e2e_selfhost_cli_main_check_json_aliases` (`EC-M1-03`) が
  `left: "Fn" / right: "Int"` で落ちた。fixture は `(defn main [] 42)` で、
  `check --json` の `type` フィールドを読んでいる。`render-type-text`
  (`Cli.ls:715` / `EmbeddedCli.ls:114`) は ty-fun (tag 3) を `"Fn"` へ潰すため、
  `Unit -> Int` になった `main` は `"Fn"` を返す。
  **本件が `ASSERT-DIAG-MESSAGE-01` の回帰でないことの根拠**: 当該 slice の編集を
  一切含まない凍結済み `target/debug/lsharp` が既に
  `{"command":"check","type":"Fn",...}` を返す。また当該 slice の `Cli.ls` 差分は
  preflight 4 関数だけで `check` 経路に触れていない。
  起票時に書いた「5 本は確定した下限で、全数は未了」のとおりの追加分である。
  **前 5 本と違い、この 1 本は型そのものではなく `check --json` の利用者向け出力を
  見ている** — つまり `I-45` の契約変更は user-visible な出力まで変えていた。
  **解決** (2026-08-23): 期待値を `"Fn"` へ張り直し、`ok. 1 passed; 0 failed; 262.18s`
  を確認した (`/Users/biwakonbu/github/tmp/i60b/green.log`)。
  実測は `decisions-selfhost-zero-arity-defn-type.md` の「6 本目」節が正本。
  **6 本もまだ下限である**ことは変わらない。全数確定は full lane に委ねる。
- **関連**: `I-45`、`docs/adr/decisions-selfhost-zero-arity-defn-type.md`。

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

<a id="i-61"></a>

### I-61: `definition` / `references` の wire 形式が request の URI の送り方で分岐する

- **影響度**: 中 / **状態**: resolved (2026-08-23) / **発見**: 2026-08-23 (`I-57` の修正中に分離)
- **内容**: 同じ document の同じ token に対して、client が `"uri"` を**文字列**で送れば
  `lsp-render-location-json-with-uri` (`selfhost/src/Tools/Lsp/LspServerNav.ls:88-96`) を通って
  LSP の `Location` object (`{"uri":"..","range":{..}}`) が返り、**int** で送れば
  `lsp-render-location-json` / `lsp-render-location-frame` を通って縮約 array
  (`[uri, line, col]`) が返る。分岐条件は `server-state-uri-text-for-uri` が非空かどうかだけで、
  method でも capability でもない。
- **根拠**: `I-57` の「原因」節の表。`I-57` の修正で**座標系は両経路とも 0 始まりに揃った**が、
  **形式の分岐は残っている**。snapshot の request は int を送るので、
  `tests/snapshots/lsp/stdio/definition-*.json` / `references-*.json` /
  `filesystem-document-sequence.json` はすべて縮約 array 側を実測している。
- **なぜ `I-57` で閉じないか**: 縮約 array を廃するか残すかは互換性の判断であり、
  座標のバグ修正とは別種の決定である。`I-57` 本文の「判断が要る点」で
  「**本 issue では形式変更まで決めない。形式は ADR で決める**」と切り分けた。
- **決める前に見るもの**: `definition` / `references` の consumer は現状 test だけだが、
  `lsp-offset-from-line-col` 等の内部 helper が縮約 array の形を前提にしていないかを先に確認する。
  object 経路は既に実装されているので、決定は「新形式の追加」ではなく「fallback の廃止」になる。
- **裁定** (2026-08-23、doc-RED): 縮約 array を廃止し、常に LSP `Location` object を返す。
  uri 文字列は「client が送った uri text → `file://` + 絶対 path → `lsharp://document/<hash>`」の
  3 段 fallback で決める。決め手は 3 つ。(1) 縮約 array は LSP 3.17 に無く、読める client が無い。
  (2) `lsp-virtual-uri-for-path` (`LspServerNav.ls:509-511`) 経由で**実 client から到達する** —
  開いていないファイルへの定義ジャンプで踏み、しかもその時点で path は既知である。
  (3) `lsp-render-locations-frame-with-state` の guard は**先頭要素しか見ない**ため、
  uri text を持たない同一 location が先頭なら縮約 array、2 番目以降なら
  `lsharp://document/<hash>` の object になる — **描画が位置に依存する**。
  判断と却下理由は
  [`definition` / `references` の wire 形式](docs/adr/decisions-lsp-location-wire-shape.md) が正本。
- **解決** (2026-08-23): 縮約 array のレンダラを実装から削除した — `LspServerNav.ls` の
  `lsp-render-location-json` / `lsp-render-locations-json-loop` / `lsp-render-locations-frame` と
  `LspServerCore.ls` の `lsp-render-location-frame`。`lsp-render-location-frame-with-state` /
  `lsp-render-locations-frame-with-state` の guard も外したので、**形式が 2 つに戻る経路が実装に無い**。
  uri 文字列は 3 段 fallback (`lsp-register-file-uri-text` が 2 段目を担う)。
  pin は `test_e2e_selfhost_lsp_locations_frame_always_renders_location_objects`
  (`crates/lsharp-wasm/tests/e2e/selfhost_lsp_docs_ops.rs`) で、RED では
  uri text を持つ 3 番目の location まで巻き込んで縮約 array へ落ちることを実測した。
  書き換えた期待値は snapshot 9 file / 10 frame + インライン 13 箇所。
  **`I-57` 由来の見積もり (snapshot 8 file) は 1 file 少なく、`references.json` が漏れていた。**
  実測値と受入判定は
  [ADR の Evidence](docs/adr/decisions-lsp-location-wire-shape.md#evidence) が正本。
- **残渣**: `rename` に同じ先頭要素依存が残っていた (`I-63`)。ADR が scope を
  definition / references に限ったためで、しかも本 issue の 2 段目追加により
  uri text を持つ document が増えたので**踏みやすくなっていた**。
  **2026-08-23 に `I-63` で解決済み** —
  [ADR: `rename` の wire 形式](docs/adr/decisions-lsp-rename-wire-shape.md)。
- **関連**: `I-57` (座標の漏れ。解決済み)、`I-52` / `I-55` (座標規約の同系統)、
  `I-63` (`rename` 側。解決済み)。

<a id="i-62"></a>
### I-62: `:example` は quote を含んでも診断 0 件で通り、`lsharp test` が message 無しで落ちる

- **影響度**: 低 / **状態**: resolved (2026-08-23) / **発見**: 2026-08-23 (`I-59` の裁定中)
- **内容**: `I-59` で `:invariant` 側は quote を `lsharp check` の段階で弾くようにしたが、
  `:example` 側は素通しのままである。実測 (2026-08-23):

  ```
  (defn caller [x] :example [(caller 'sym)] x)
  ```

  | コマンド | 結果 |
  |---|---|
  | `lsharp check` | exit 0、`diagnostics.count = 0` |
  | `lsharp test` | exit 1、`status = "fail"` / `executed 0, failed 1` / **`message` は空文字列** |

- **なぜ穴なのか**: `:example` も `crates/lsharp-wasm/src/test_runner.rs:78` で生成ソースへ
  差し込まれて**実行される**。quote は `ir/lower/expr/quote_expr.rs:9` が拒否するので、
  `:invariant` とまったく同じ理由で落ちる。違うのは落ちる場所と、
  **落ちた理由がどこにも出ないこと**である。`lsharp test` は `failed 1` とだけ言い、
  診断も message も空なので、利用者は原因に辿り着けない。
- **`:example` が 0 件で通ることは対称性の根拠にならない**。
  [`:invariant` に書かれた quote の扱い](docs/adr/decisions-invariant-quote-handling.md)
  の案 (b) 却下理由で「正しさの証拠ではなく同種の穴」と判定済みで、
  揃えるなら `:invariant` を緩める方向ではなく `:example` を締める方向である。
- **本 issue で決めないこと**: 締め方が 2 つある。(a) `check_metadata` に `:example` 用の
  quote 検出を足して `I-59` と同型の Error を出す。(b) `lsharp test` の失敗 message を
  埋めて lowering の理由を伝搬させる。(a) は早く落ちるが `:example` の式は
  `:invariant` と違って任意の呼び出し列なので検出範囲の設計が要る。
  (b) は quote 以外の lowering 失敗も一緒に救うが、落ちる位置は後ろのままである。
  **どちらか一方で足りるのか両方要るのかは、この issue では決めない。**
- **2026-08-23 追記**: 上の実測表は**既定の runner 1 経路だけ**を見ていた。
  rust runner を分けて測ると (b) は既に成立しており、空 message は selfhost runner 側の
  一般の欠落だと分かった。裁定は
  [`:example` に書かれた quote の扱い](docs/adr/decisions-example-quote-handling.md) が正本で、
  **(a) だけを採る**。分解した残りは `I-49` 残差分 (preflight の空 message。2026-08-23 解消) と
  `I-65` (`:example` の suite 経路。`EXAMPLE-FAIL-REASON-01`。どちらも 2026-08-23 解消) が引き取る。
- **解決** (2026-08-23): 案 (a) を実装した。`check_example`
  (`crates/lsharp-types/src/metadata_check/diagnostics.rs`) が `find_quote_span` で式全体を走査し、
  quote があれば `:invariant` と同型の metadata 固有 Error を 1 件返す。既存の識別子スコープ検査は
  据え置いたので `'(a ~nonexistent)` は 2 件になる。
  pin は `contract_scope_quoted_symbol_in_example_reports_metadata_error` と、
  書き換えた `contract_scope_unquoted_reference_inside_quote_still_errors`
  (`metadata_check/diagnostics_tests.rs`)。
  rust runner の `lsharp test --format json` は `firstErrorCode` 1001 → 1002、
  `message` が `[E0001] 未定義の変数 (undefined)` から
  `[LS1002] [error] caller: :example に quote/unquote は書けません …` へ置き換わり、
  span も生成ソース `63..67` から元ソース `37..41` へ移った。
  `cargo test -p lsharp-types` は test binary 41 本すべて緑、clippy 警告 0。
  判断と却下案の正本は
  [`:example` に書かれた quote の扱い](docs/adr/decisions-example-quote-handling.md)。
- **残渣**: 既定経路 (selfhost runner) の挙動は変えていない。本 issue の冒頭の実測表が
  そちらを見ていたので、**表の症状そのものは既定 CLI では残っていた**。引き取り先は `I-65`
  (`:invariant` + quote が緑になる方。2026-08-23 解消) と `EXAMPLE-FAIL-REASON-01`
  (`:example` 側の空 message。2026-08-23 解消。正本は
  [`:example` の失敗理由](docs/adr/decisions-selfhost-example-fail-reason.md))。
  **既定 CLI の空 message は解消した** — 同じ fixture が
  `:example 式が偽を返しました: (caller 'sym)` 相当の message を返す。
  残るのは `cases` / `coverage.executed` の数え方で、`I-67` が引き取る。
- **関連**: `I-59` (解決済み。`:invariant` 側)、`I-43`、`I-65` (selfhost runner の quote 契約不在)、
  `I-49` (selfhost preflight の空 message。2026-08-23 解消)。

<a id="i-63"></a>
### I-63: `rename` の wire 形式も先頭要素の uri text だけで list 全体が切り替わる

- **影響度**: 低 / **状態**: resolved (2026-08-23) / **発見**: 2026-08-23 (`I-61` の実装中)
- **内容**: `lsp-render-rename-frame-with-state`
  (`selfhost/src/Tools/Lsp/LspServerNav.ls:222`) は `changes` の**先頭要素の uri text だけ**を見て、
  非空なら `{"changes":{..}}` (LSP の `WorkspaceEdit`)、空なら `[[uri,[TextEdit..]],..]` という
  縮約形へ list 全体を落とす。`I-61` が `definition` / `references` で潰した位置依存と
  まったく同じ形が、method 違いでもう 1 箇所残っている。
- **`I-61` で一緒に直さなかった理由**:
  [ADR](docs/adr/decisions-lsp-location-wire-shape.md) は scope を `textDocument/definition` と
  `textDocument/references` の応答形式に限り、`rename` を明示的に対象外と書いている。
  `WorkspaceEdit` は `Location` と別の型で、縮約形を廃止するときに
  **`changes` へ寄せるか `documentChanges` へ寄せるか**という別の判断が要る。
  uri 文字列の決め方 (`I-61` の 3 段 fallback) は流用できるが、それだけでは閉じない。
- **今すぐ壊れているわけではない**: 現行 e2e fixture は uri を int で送るので常に縮約形へ落ち、
  期待値もそれで pin されている。位置依存が観測されるのは、**uri text を持つ document と
  持たない document が同じ rename 結果に混ざったとき**である。
- **根拠**: source 読み (2026-08-23)。`I-61` の実装で `lsp-virtual-uri-for-path` が
  絶対 path の uri text を state へ登録するようになったため、混在は起きやすくなっている。
- **解決** (2026-08-23): guard を削除して `lsp-render-rename-frame-with-state` を単一経路にし、
  縮約側 3 関数 (`lsp-render-rename-frame` / `lsp-render-workspace-changes-json-loop` /
  `lsp-render-workspace-change-json`) を削除した。空 `changes` は `{"changes":{}}` を返す。
  判断と却下理由は
  [ADR: `rename` の wire 形式](docs/adr/decisions-lsp-rename-wire-shape.md) が正本。
  位置依存が消えたことは
  `test_e2e_selfhost_lsp_rename_frame_always_renders_workspace_edit` が pin する
  (uri text を持たない document を**先頭に**置いた混在ケース)。
  回帰 lane は `--ignored` 32 test が `ok. 32 passed; 0 failed` / 1638.95s。
  実測は [ADR の Evidence](docs/adr/decisions-lsp-rename-wire-shape.md#evidence) が正本。
- **残渣**: `test_e2e_selfhost_cli_lsp_transport_rename_frame` が本変更より**前から**赤だった
  ことがこの作業中に判明した。期待値は本 slice で直したが、`#[ignore]` の赤が
  どの台帳にも載らない仕組みのほうは `I-64` へ切り出した。
- **関連**: `I-61` (`definition` / `references` 側。解決済み)、`I-57` (座標系)、
  `I-64` (この作業で見つかった `#[ignore]` の陳腐化 pin)。

<a id="i-64"></a>
### I-64: `#[ignore]` の e2e が陳腐化した期待値を抱えたまま誰にも観測されない

- **影響度**: 中 / **状態**: resolved (2026-08-24) / **発見**: 2026-08-23 (`I-63` の影響範囲 grep 中)
- **内容**: `crates/lsharp-wasm/tests/e2e/selfhost_cli_core.rs:5290`
  `test_e2e_selfhost_cli_lsp_transport_rename_frame` は `#[ignore]` が付いており、
  **`I-63` の作業で単独実行するまで誰も回していなかった。実行すると FAIL する。**
  期待値は TextEdit を `[1,7,1,13,"cube"]` という縮約 array で pin しているが、
  現行 renderer (`lsp-render-text-edit-json`) は `{"range":{..},"newText":".."}` の
  object を出す。期待値は 2026-03-27 の Phase 12 commit `9deab1ce` 以来据え置かれており、
  その後 renderer 側だけが変わった。
- **実測** (2026-08-23):

  ```
  test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 3076 filtered out; finished in 259.16s
    left: ...,"result":[[99,[{"range":{"start":{"line":0,"character":6},...},"newText":"cube"},...]]]
   right: ...,"result":[[99,[[1,7,1,13,"cube"],[2,16,2,22,"cube"],[2,27,2,33,"cube"]]]]
  ```

  **この FAIL は `I-63` の変更より前から存在する。** `I-63` の実装で期待値は
  `WorkspaceEdit` 形へ書き換えるので、この 1 本自体は同時に GREEN になる。
- **問題は 1 本ではなく仕組みのほう**: `#[ignore]` の e2e は既定 lane で走らず、
  `workspace-expected-failures.txt` にも載っていないので、**赤いことすら台帳に無い**。
  `I-60` (0 引数 defn の pin 5 本が契約変更で赤のまま放置) と同じ壊れ方であり、
  同じことが再発している。1 本ずつ見つけるのではなく、
  **`#[ignore]` 付き e2e を一度全部走らせて赤を列挙し、expected-failure か修正かを決める**必要がある。
- **今回この issue で直さない理由**: `--ignored` の全量は e2e 単独で 5 時間規模
  (`I-11` の実測)。`I-63` の slice に載せると計測も判断も混ざる。
- **解決 (2026-08-24)**: `#[ignore]` lane を **18 module / 1,431 件**全量実行し、
  赤 274 件を 1 本残らず台帳へ振り分けた。実測は
  [`ignored-lane-sweep-2026-08-23.md`](docs/development/operations/ignored-lane-sweep-2026-08-23.md)、
  台帳は [`ignored-lane-expected-failures.txt`](docs/development/validation/ignored-lane-expected-failures.txt) (274 行)。
  `compare_ignored_lane.py` を 18 ログへ流し
  **完走判定 OK / 新規 FAIL 0 / 解消 0 / 未出現 0 (exit 0)**。所要は通算 41,043s (約 11.4 時間)。

  **この sweep が実際に捕まえたもの**:

  | 収量 | 件数 |
  |---|---:|
  | 新規に露出した赤 | 145 |
  | うち新規 issue を要した cluster | 5 (`I-71`〜`I-75`) |
  | 既存項目の射程が広がったもの | `check` の型名 pin +3 / `REPL-TYPE-TAG-01` +1 |
  | 緑に転じて台帳から外したもの | 1 |

  **懸念そのものが実測で裏付けられた。** `I-45` の契約変更 (`914bd9f1`、2026-08-22) が
  取り残した型名 pin は、翌日の専用修復パス `13a505b2` を経てなお **6 本**残っていた。
  赤を狙った修復パスですら取りこぼす。**網羅は「気を付ける」では達成されず、
  lane を回すことでしか達成されない。**
- **残った未診断は本 issue に持たせない。** 145 件の原因追及は `I-71`〜`I-75` が持つ。
  本 issue の受入条件は「全量実行して振り分ける」であり、それは満たした。
- **関連**: `I-63` (この test の期待値を書き換える側)、`I-60` (同型の放置)、
  `I-11` (workspace 恒常 FAIL の baseline)、`I-70` (ADR の Evidence 側)、
  `I-71`〜`I-75` (振り分け先)。

<a id="i-65"></a>
### I-65: selfhost runner は contract metadata の quote 契約を持たず、`:invariant` + quote を `pass` と報告する

- **影響度**: 中 / **状態**: resolved (2026-08-23) / **発見**: 2026-08-23 (`I-62` の裁定中)
- **内容**: `lsharp test` は既定で selfhost runner へ委譲される
  (`provenance.runner = "selfhost"`)。この runner は contract metadata の quote 検査を持たず、
  `I-59` / `I-62` が rust 側へ入れた診断が既定経路からは**一切見えない**。
  実測 (2026-08-23、`./target/debug/lsharp`、fixture は 1 ファイル 1 defn):

  | fixture | runner | 結果 |
  |---|---|---|
  | `(defn caller [x] :invariant (= 'sym 'sym) x)` | selfhost (既定) | **`status pass` / `executed 5, failed 0` / exit 0** |
  | 同上 | rust (`LSHARP_DISABLE_EMBEDDED_COMPONENT=1`) | `[LS1002] [error] caller: :invariant に quote/unquote は書けません (…) (33..37)` / exit 1 |
  | `(defn caller [x] :example [(caller 'sym)] x)` | selfhost (既定) | `status fail` / `executed 0, failed 1` / **`message` 空** / `count 0` |
  | 同上 | rust | `[LS1001] テストプログラムの型チェックに失敗: [E0001] 未定義の変数 (undefined): quote/unquote はマクロ展開後に使用できません (63..67)` |

- **なぜ message 欠落より重いか**: `:example` 側は「落ちるが理由が空」なので、
  利用者は少なくとも失敗に気付ける。これは `EXAMPLE-FAIL-REASON-01` の担当範囲で、
  2026-08-23 に解消した (正本は
  [`:example` の失敗理由](docs/adr/decisions-selfhost-example-fail-reason.md))。
  preflight 側の空 message も `I-49` 残差分として 2026-08-23 に解消済みである。**`:invariant` 側は緑を返す。** 実行できないはずの contract が
  「5 件実行して 0 件失敗」と報告されるので、利用者は穴の存在に気付けない。
  原因は `selfhost/src/Types/TypeInfer.ls:199` の「quote/unquote 系は現状すべて inner expr へ
  委譲する」という扱いで、`'sym` が中身の型として通ってしまうことにある。
- **`lsharp check` も同様**: `check` は selfhost shadow command (`main.rs:809`) なので、
  rust の `check_metadata` は既定経路では走らない。両 fixture とも
  `diagnostics.count = 0` / exit 0 を返す (`migration` 配列に `LS2002` / `LS2003` は載る)。
  [`:invariant` ADR](docs/adr/decisions-invariant-quote-handling.md) の
  「`lsharp test` まで待たずに `lsharp check` の段階で分かる」は
  **rust 側の層についての記述**であり、既定 CLI には当てはまらない。
- **本 issue で決めないこと**: selfhost 側に metadata contract 検査をどう載せるか
  (TypeInfer で弾く / TestRunner の preflight で弾く / rust の診断を委譲経路で運ぶ)。
  parity の取り方は設計判断なので ADR が要る。
- **設計判断** (2026-08-23): [decisions-selfhost-contract-quote-parity.md](docs/adr/decisions-selfhost-contract-quote-parity.md)
  が正本。TestRunner の preflight に `check-contract-quote` を足し、contract directive の
  **ソース範囲をトークン列で**走査して quote / unquote / splice-unquote を弾く案を採った。
  payload を AST として走査する案は、`:example` と `:property` の payload が AST ではなく
  **ソース文字列**であるため (`Parser.ls:1364` / `:1846`) 上表 2 行目の fixture に届かず却下した。
  `TypeInfer` を一般に厳しくする案は `I-48` の 262 defn 巻き添えと同じ性質なので別 slice へ送った。
- **解決** (2026-08-23): `TestRunner.ls` に `check-contract-quote` を新設し、
  contract directive のソース範囲をトークン列で走査して quote / unquote / splice-unquote を
  `LS2008` で弾くようにした。`Cli.ls` / `EmbeddedCli.ls` の両方の
  `run-test-source-json` / `run-test-source-text` へ差し込んである。
  **上表 1 行目の `status pass` は再現しない** — 既定経路で `status fail` / exit 2 /
  `firstErrorCode 2008` を返し、message に directive 範囲が載る。上表は発見時の記録として残す。
  実測と受入判定は [decisions-selfhost-contract-quote-parity.md](docs/adr/decisions-selfhost-contract-quote-parity.md)
  の Evidence 節が正本。**`Cli.ls` 側の分岐を実行した test は無い** (該当 e2e が 2 本とも
  `#[ignore]`。`I-64` の範囲) ことも同節に明記した。
- **関連**: `I-59` / `I-62` (rust 側でこの契約を実装した側)、`I-49` (selfhost preflight の
  空 message。2026-08-23 解消)、`I-66` (本 issue の GREEN 中に見つけた既定 lane の食い違い)。
  `:example` の quote **以外**の失敗理由は `EXAMPLE-FAIL-REASON-01` が引き取り、
  2026-08-23 に解消した (正本は
  [`:example` の失敗理由](docs/adr/decisions-selfhost-example-fail-reason.md))。

<a id="i-66"></a>
### I-66: EmbeddedCli の既定 test option が `--format json` と同値で、`run-test-source-text` が到達不能になっている

- **影響度**: 低 / **状態**: open / **発見**: 2026-08-23 (`SELFHOST-QUOTE-PARITY-01` の GREEN 中)
- **内容**: `EmbeddedCli.ls` の `main` は `argc` が 2 のとき (`lsharp test input.ls`)
  option 解析を通らず `(default-compile-target)` をそのまま opts として `run-command` へ渡す
  (`EmbeddedCli.ls:1730`)。EmbeddedCli の既定 target は `(compile-target-component)` = **1**
  (`:42,:44`) で、これが `(test-option-json)` = **1** (`:86`) と同値である。
  結果として `run-test-source` (`:1293`) の JSON 分岐に入り、
  **`--format json` を付けていないのに assurance JSON が出る**。
  `run-test-source-text` (`:1201`) は `test` command からは到達しない。
- **実測** (2026-08-23、e2e bundle `selfhost_embedded_cli_runtime_bundle()` に
  `&["test", "input.ls"]`): 出力は 1 行の assurance JSON。`diagnostics.message` も載る。
  `run-test-source-text` が出す `examples:N` / `failures:N` 形式は一切現れない。
- **`Cli.ls` は違う**: `Cli.ls` の既定 target は `(compile-target-preview1)` = 0 (`Cli.ls:46`)
  なので、同じ argv で `run-test-source-text` に入る。**2 系統で既定 lane が食い違っている。**
  ただし `Cli.ls` の `test` を argv 経由で叩く e2e は 2 本とも `#[ignore]` なので
  (`selfhost_cli_actual_main_args.rs` の `test_e2e_selfhost_cli_main_with_args_test_file` /
  `..._test_format_json_file`)、この食い違いは live なテストでは観測されていない (`I-64`)。
- **なぜ低いか**: 現状の出力は JSON なので情報量は多く、利用者が損をしていない。
  `SELFHOST-QUOTE-PARITY-01` の受入判定にも影響しない (既定 lane は非緑になる)。
  問題は「option の番号空間が 2 つ重なっている」という設計上の危うさで、
  `compile-target` 側に値を足すと `test` の挙動が黙って変わる。
- **直し方の方向** (未確定): option enum を command ごとに分ける、または
  `test` command のとき `default-compile-target` ではなく明示の `test-option-text` を渡す。
  後者は `Cli.ls` / `EmbeddedCli.ls` の既定 lane を揃える判断を伴うので ADR が要る。
- **`check` も同じ**  (2026-08-23 追記): `check-option-json` (`EmbeddedCli.ls:86`) も 1 なので、
  `lsharp check input.ls` (argc 2) も `EmbeddedCli` では JSON lane、`Cli.ls` では text lane に入る。
  fallthrough (`EmbeddedCli.ls:1730` / `Cli.ls:2682`) は 1 箇所で両 command を賄っており、
  機構が同じである。**本 issue の範囲に `check` を含める。**
- **方向は確定した** (2026-08-23): 参照実装 `crates/lsharp-driver/src/main.rs:201` の
  `Test.format` が `default_value = "text"` なので、**text lane が正**であり
  寄せるのは `EmbeddedCli` 側である。`default-compile-target` は触らない
  (component 既定は意図であり、変えると `compile` の契約が壊れる)。
  判断と却下理由は
  [argc 2 の command 既定 option](docs/adr/decisions-selfhost-cli-argc2-command-default-option.md)。
- **関連**: `I-64` (`#[ignore]` により観測されない)、`I-65` (発見の経緯)。
  引き取り先は `TODO.md` の `EMBEDDED-CLI-OPTION-SPACE-01`。

<a id="i-67"></a>
### I-67: selfhost runner の `cases` / `coverage.executed` は pass 数を数えており、失敗時に rust runner と食い違う

- **影響度**: 低 / **状態**: resolved / **発見**: 2026-08-23 (`EXAMPLE-FAIL-REASON-01` の原因調査中)
- **内容**: `:example` が 1 件あって偽を返す fixture で、2 つの runner の assurance JSON が
  `message` 以外にも 3 箇所食い違う。実測 (2026-08-23、
  `(defn abs [x] :example [(= (abs 5) 6)] (if (< x 0) (- 0 x) x))`、
  fixture と同じ dir から相対パスで `./target/debug/lsharp test ex_fail.ls`):

  | フィールド | selfhost (既定) | rust (`--format json`) |
  |---|---|---|
  | `cases` | **0** | 1 |
  | `coverage.executed` | **0** | 1 |
  | `coverage.failed` | 1 | 1 |
  | exit code | **1** | 2 |

- **原因**: `EmbeddedCli.ls:846` の `assurance-result-actual-loop` が結果 vector の
  index 2 (`actual`) を合算して `executed` を作る。ところが
  `TestRunner.ls:4000` の `run-examples-loop` は `(make-test-result name passed passed)` と
  書いており、**`actual` に `passed` を入れている**。したがって失敗した `:example` は
  `actual = 0` を寄与し、`executed` にも `cases` にも数えられない。
  「実行した数」ではなく「通った数」を数えている。
- **なぜ低いか**: `coverage.failed` と `status` は正しいので、成否の判定自体は誤らない。
  食い違うのは分母側の 2 つと exit code である。ただし
  [zero-arity defn の型](docs/adr/decisions-selfhost-zero-arity-defn-type.md) のように
  **`coverage.executed` を受入判定の gate に使う運用が既にある**ので、
  失敗を含む fixture でこれを gate にすると意図しない値を見ることになる。
- **exit code の 1 / 2 は別の話かもしれない**。selfhost は `exit-runtime-error` で 1、
  rust runner は 2 を返す。これが意図的な使い分けなのかは未確認で、本 issue では決めない。
  **本 issue の解決後も残る**。実測として記録するだけで、引き取り先は未定である。
- **解決** (2026-08-23): `actual` の意味を「実行した contract 数」に確定させ、
  `run-examples-loop` を `:assert` (`actual = 1`) と同じ形へ揃えた。集計側は無変更。
  他の kind の `actual` を洗った結果は ADR の表にある。実測で
  `cases` / `coverage.executed` / `coverage.failed` が 3 fixture すべて 2 runner 一致になった。
  live e2e は `test_e2e_selfhost_embedded_cli_test_format_json_example_failure_message`。
  正本は [`cases` / `coverage.executed` の意味](docs/adr/decisions-selfhost-example-coverage-count.md)。
- **切り出したもの**: `:invariant` / property / `:case` の `actual` はサンプル数や式の値を
  入れたままで、rust oracle (contract 数) と食い違う。`I-68` へ分けた。
- **関連**: `I-62` (同じ fixture 系列)、`EXAMPLE-FAIL-REASON-01` (`message` 側。
  こちらは本 issue と切り離して解決した。正本は
  [`:example` の失敗理由](docs/adr/decisions-selfhost-example-fail-reason.md))、
  `I-68` (残りの kind)。

<a id="i-68"></a>
### I-68: `:invariant` / property の `cases` はサンプル数を載せており、rust oracle の contract 数と食い違う

- **影響度**: 低 / **状態**: open / **発見**: 2026-08-23 (`I-67` の原因調査中)
- **内容**: assurance JSON の `cases` / `coverage.executed` が何を数えるかについて、
  2 つの runner が別の契約を持っている。rust は
  `MetadataTestRun::total()` (`crates/lsharp-tooling/src/metadata_test.rs:19`) が
  `results.len()` であり、**contract 1 件を 1 と数える**。サンプルを何本回したかは載らない。
  selfhost は結果 vector の `actual` slot を総和するので、
  `:invariant` は `sample-count`、property は実行サンプル数を載せる。
- **根拠**: `TestRunner.ls` の `materialize-invariant` は
  `actual (if (= diagnostic-code 0) sample-count 0)`、
  `materialize-property-with-span` は `actual (if (= diagnostic-code 0) actual-count 0)` と書く。
  結果として `:cases 5` の property は selfhost で `cases 5` / `executed 5`
  (`crates/lsharp-wasm/tests/e2e/selfhost_cli_actual_main_args.rs:436`,
  `crates/lsharp-wasm/tests/native_cli_output.rs:438` が固定) になるが、
  同じ fixture の rust runner は `1` を返す。
- **なぜ低いか**: 分母の意味の食い違いであって、`status` と `coverage.failed` は
  どちらの runner でも一致する。成否の判定は誤らない。
- **`I-67` との違い**: `I-67` は `:example` が `actual` に `passed` を入れていた
  「実行数を通過数と取り違えた」バグで、rust に寄せる向きが一意に決まる。
  こちらは**どちらの契約を正とするかがまだ決まっていない**。
  「サンプル数を出したい」という要求そのものは妥当であり、
  だとすれば載せる先が `cases` でよいのか (別フィールドを立てるべきか) を先に決める必要がある。
- **`:case` はもっと悪い** (2026-08-23 追記): `:case` の `actual` には**式の値**が入る。
  `(expect (f) 3)` は `executed` へ 3 を寄与し、`(expect (f) 0)` は 0 を寄与して
  実行数から消える。サンプル数でも contract 数でもなく、値と個数を混同している。
  **本 issue の範囲に `:case` を含める。**
- **設計時の記述とも割れている** (2026-08-23 追記): `docs/development/planning/v0.2-evidence-contracts.md:159,170`
  は `"cases": 256` とサンプル数を書く。つまり 3 者 (設計記述 / rust / selfhost) が
  別のことを言っている状態である。
- **決着** (2026-08-23): (a) を採る。集計 report の `cases` / `coverage.executed` は
  **実行した contract 数**とし、全 kind で `actual` に 1 を入れる。
  設計記述の「サンプル数」は **per-contract Evidence レコード**のフィールドであって
  ファイル単位の集計ではない (kind をまたいで足せる量は contract 数しかない)。
  サンプル数用のフィールドは足さない。判断と却下 4 案は
  [集計 assurance report の `cases`](docs/adr/decisions-assurance-cases-contract-count.md)。
  上の pin 2 件は契約変更に追随させる (実装に合わせるのではない)。
- **関連**: `I-67` (`:example` 側。切り離して解決)。正本は
  [`cases` / `coverage.executed` の意味](docs/adr/decisions-selfhost-example-coverage-count.md)
  の「却下した案 / 案 D」。引き取り先は `TODO.md` の `SAMPLE-COVERAGE-CONTRACT-01`。

<a id="i-69"></a>
### I-69: `repl-session-last-type-name` が型タグを見ずにスロット 1 を読み、`lsharp repl` が壊れた型名を出す

- **影響度**: 中 / **状態**: open / **発見**: 2026-08-23 (`I-64` の `--ignored` 全量 sweep)
- **内容**: `selfhost/src/App/Cli.ls:1463` の `repl-session-eval` が
  `(ty-name ty)` を**型タグを確かめずに**呼び、その結果を session slot 1 へ格納する。
  `ty-name` は `Type.ls:249` で `(vector-get ty 1)` なので、`ty` が Con (`[1, name-hash]`)
  でない限りスロット 1 は名前ハッシュではない。Fun (`[3, param-type, ret-type]`) なら
  slot 1 は**引数型オブジェクトそのもの**であり、tagged handle が漏れる。
- **根拠**: `--ignored` sweep で **2 module / 6 test** が同一の値 `-9223372036718940184`
  (`0x800000000818afe8`) を出す。bit 63 が立っており整数タグではない。
  - `selfhost_gc_stateful_soak.rs:83` (Int=100 期待) / `:118` (**Bool=200 期待**) /
    `:172` / `:232` / `:307`
  - `runtime_allocator_closures.rs:313` (Int=100 期待)
  - **Bool を期待する行まで同じ値になる**ことが決定的である。入力 (`(defn main [] 42)` /
    `(defn main [] true)`) が違うのに同値になるのは、読んでいるのが戻り値型ではなく
    引数型スロットだからと考えると整合する。
- **user 影響**: test だけの問題ではない。`run-repl` (`Cli.ls:1472`) は CLI dispatch
  `:2275` から到達可能で、`repl-summary-type-text` → `builtin-type-name-text` (`:714`) を通る。
  `builtin-type-name-text` は 100/200/300/400/500 以外を
  `(string-concat "type-" (int-to-string type-hash))` へ落とすので、
  `lsharp repl` も同型の壊れた型名を印字する。**2026-08-23 に CLI 経路で実測した** —
  `selfhost_cli_actual_main_args::..._repl_summary` (argv `["repl"]`) が
  `type:type--9223372036853734056` を出した。gc_stateful_soak 側の
  `-9223372036718940184` とは値が違う。**値そのものはアドレスなので pin できない**が、
  経路も症状も CLI で再現する。**握り潰されて気付けない。**
- **層が 2 つある (2026-08-23 追記)**。同 sweep の `..._check_file` /
  `..._check_json_file` が `(defn main [] 42)` に対し `"Int"` の pin へ `"Fn"` を返した。
  `render-type-text` (`Cli.ls:715`) は tag 3 を `"Fn"` と印字する正しい実装なので、
  これは **`infer` が Fun を返している**ことの直接証拠であり、本 issue 記載の
  「`infer` が Fun を返すようになったのが変化かどうか未確定」はこれで埋まった。
    - **L1**: `infer` (`TypeInfer.ls:1712`) は `infer-program-analysis-type` =
      最初の decl の型を返す。`(defn main [] 42)` なら `Unit -> Int`。由来は
      `fd786316` (2026-07-14) の program-level analysis 化。
      **ただし L1 は infer のバグではない (2026-08-23 再訂正)。** `(defn main [] 42)` が
      `Unit -> Int` になるのは
      [`decisions-selfhost-zero-arity-defn-type.md`](docs/adr/decisions-selfhost-zero-arity-defn-type.md)
      (2026-08-22 accepted / `914bd9f1` / `I-45`) が **Rust 実装 (`Fun([], _)`) との parity を根拠に
      意図して選んだ契約**であり、同 ADR は「Rust 側を selfhost に合わせる」案を
      「正しい方を壊す向き」として明示的に却下している。したがって `check` が `"Fn"` を
      印字するのは正しい。
      赤 2 件の正体は **`914bd9f1` (2026-08-22) が契約を変えた際に更新し漏らした陳腐化 pin** である。

      **機構の訂正 (2026-08-23、3 度目)。** 当初「同 commit が生きている pin だけ直した」と
      書いたが誤り。`..._check_format_json` の pin を `"Fn"` へ直したのは `914bd9f1` ではなく
      **`13a505b2` (2026-08-23、`I-60` の陳腐化 pin 修復パス)** であり、その test 自身も
      `#[ignore]` 下にある (sweep ログに `... ok` として出る)。つまり
      `914bd9f1` は同一ファイルの pin を**全部**取り残し、翌日の専用修復パス `13a505b2` が
      **4 兄弟のうち 1 本しか直さなかった**。所見はこの訂正で**弱まるのではなく強まる**:
      赤を狙って走らせた修復パスですら、既定 lane で回らない兄弟を取りこぼしている。

      取り残された兄弟は確認できただけで 3 本ある。

      | pin 位置 | 期待値 | 引き取り先 |
      |---|---|---|
      | `selfhost_cli_actual_main_args.rs` `..._check_file` | `Int` → `Fn` | 2026-08-27 解決 |
      | `selfhost_cli_actual_main_args.rs` `..._check_json_file` | `Int` → `Fn` | 2026-08-27 解決 |
      | `selfhost_cli_actual_main_args.rs:1786` `..._repl_summary` | `type:Int` → `type:Fn` | `REPL-TYPE-TAG-01` |
      | `selfhost_cli_core.rs:4792` (repl 系、未 sweep) | `type:Int` → `type:Fn` | `REPL-TYPE-TAG-01` |

      いずれも `input-bytes:17` = `(defn main [] 42)` ちょうど 17 byte で、同一 fixture である。
      **`I-64` (`IGNORED-STALE-PIN-01`) が想定していた壊れ方そのもので、混入から 1 日で観測された。**
      陳腐化しているのはもう 1 件ある —
      [`decisions-v0.3-native-cli-check-file-e2e.md`](docs/adr/decisions-v0.3-native-cli-check-file-e2e.md)
      の Evidence 節が `..._check_file` は `Int` を返すと書いている。同 ADR は
      「output contract が green になったら normal test にする」と書いており、
      **ignore 下の test の自称期待値をそのまま Evidence にした**ため再検証されていない。
      引き取り先は `check` 型名 pin の追随 slice で、`REPL-TYPE-TAG-01` からは外す。
      **2026-08-27 に解決した。** pin 5 本を `"Fn"` へ追随させ、当該 ADR の Evidence も訂正した。
      判別の結果 `render-type-text` のバグではなく `I-45` 契約そのものであることが確定している
      (下記 `I-76`)。経緯は [`decisions-selfhost-zero-arity-defn-type.md`](docs/adr/decisions-selfhost-zero-arity-defn-type.md) の「7〜11 本目」節。
    - **L2**: `repl-session-eval` が tag を見ずに `ty-name` を呼ぶ (本 issue の当初の指摘)。
  **L1 を直せば L2 の症状も消えるが、L2 は独立の欠陥である。** 型が本当に Fun である
  program を評価した瞬間に同じアドレス印字が再発する。
- **直し方はリポジトリ内にある**: 同じ `Cli.ls:715` の `render-type-text` は
  `(ty-tag ty)` で分岐し、Con のときだけ `builtin-type-name-text` を通す。
  `repl-session-eval` をこの形へ寄せればよい。新規設計は要らない。
- **L2 だけが本 issue の欠陥である (2026-08-23 確定)。** `type:type--9223372036853734056` は
  どの契約の下でもゴミであり、`repl-session-eval` を `ty-tag` 分岐へ寄せる修正は L1 と独立に正しい。
  ただし **repl の受入値は `type:Int` ではなく `type:Fn`** になる —
  `(defn main [] 42)` は `I-45` の契約で `Unit -> Int` であり、`render-type-text` は tag 3 を
  `"Fn"` と印字するのが正しい実装だからである。
- **なぜ今まで観測されなかったか**: 該当 6 test は全て `#[ignore]` 付きで、
  `I-64` が指摘した「検査は生きているが中身が観測されない」領域にあった。
  `workspace-expected-failures.txt` にも `--ignored` 台帳にも載っていない。
- **関連**: `I-64` (発見経路)。引き取り先は `TODO.md` の `REPL-TYPE-TAG-01`。

<a id="i-70"></a>
### I-70: ADR の Evidence 節が `#[ignore]` 下の test を根拠にしており、赤に転じても訂正されない

- **状態**: resolved (2026-08-24)
- **発見**: 2026-08-23 (`I-64` の `#[ignore]` 全量 sweep の副産物)

`docs/adr/decisions-v0.3-native-cli-check-file-e2e.md` の Evidence 節は
`test_e2e_selfhost_cli_main_with_args_check_file` が `Int` / `diagnostics:0` を返すと書いている。
同 ADR の Decision 節は「output contract が green になったら normal test にする」と書いており、
つまり **ADR を書いた時点でその test は `#[ignore]` 下にあった**。sweep で実測すると `Fn` を返す。
Evidence として書かれていたのは実測値ではなく、test の自称期待値だった。

- **一般化して数えた (2026-08-23、cargo 非依存の grep)**。`docs/adr/*.md` の `## Evidence` 節から
  `test_*` を拾い、`#[ignore]` 付き test 名 (1,493 件) と突き合わせた結果、
  **14 ADR が計 36 件の `#[ignore]` 下 test を Evidence に引いている**。
  内訳の最大は `decisions-native-root-pop-empty-guard.md` の 19 件。
- **「引いている」だけでは欠陥ではない。** heavy CI gate は意図的に `#[ignore]` を付けて
  `scripts/ci/*` から回す運用 (`ops03c`) があり、その場合は別経路で検証されている。
  **問題なのは「どの script も回しておらず、かつ赤」の組み合わせ**である。
- **母数を測り直した (2026-08-24)。36 件 / 14 ADR ではなく 43 件 / 15 ADR だった。**
  2026-08-23 の grep は `#[ignore]` の直後に別の属性が続く形 (`#[ignore]` → `#[cfg(..)]` → `fn`) を
  取りこぼしていた。**古い数字を静かに置き換えず、なぜ増えたかをここに残す。**
  なお `#[cfg_attr(.., ignore)]` 形は実測 0 件なので、43 が全量である。
- **`I-60` / `I-64` と同型だが層が違う。** あちらは test の pin が陳腐化する話、
  こちらは **ADR という判断の正本が陳腐化する**話である。ADR は後続の判断の根拠として
  引かれるので、誤った Evidence は test 1 本より遠くまで伝播する。
- 引き取り先は `TODO.md` の `ADR-EVIDENCE-IGNORED-01` だった (解決に伴い削除済み)。

**解決** (2026-08-24): 43 件を実測 verdict と突き合わせ、赤・曖昧な 17 件 (8 ADR) を
3 つに分類して、矛盾するものだけを訂正した。

**件数は citation 単位で数える** (母数の 43 と同じ基数)。同じ test を 2 ADR が引く例が
あるため、test 単位の数とは一致しない。両方を併記する。

| 分類 | citation | 一意 test | 扱い |
|---|---|---|---|
| 緑 (Evidence どおり) | 26 | -- | 訂正不要 |
| **一致** — ADR 自身が「赤である」と主張している | 11 | 10 | 訂正不要。裏付けとして記録 |
| **環境ゲート** — 前提 (env / Lima VM) が sweep で未充足 | 3 | 2 | 前提を明記する補足節を追加 |
| **矛盾** — ADR が pass と書き、実測が FAILED | 3 | 2 | 訂正節を追加 |

**数え違いを 1 件訂正した (2026-08-24)。** 初版は「17 件 (11 ADR) / 一致 10 / 矛盾 4 (3 test)」と
書いたが、ADR 数は 8 が正しく、一致の 10 は citation ではなく test を数えたものだった。
基数を混ぜた結果、差分が矛盾側へ吸われて 4 になっていた。**この slice はまさに
「数字を静かに直さない」ために存在するので、何を数え違えたかを残す。**
訂正した ADR の実数は 3 節 (下表 3 行) で、初版から変わっていない。

**最大の発見は「赤は陳腐化の証拠にならない」ことだった。**
`decisions-native-root-pop-empty-guard.md` の 9 件はすべて赤だが、同 ADR の当該節は
**失敗分類表**であり「どれが赤で、なぜ赤か」を書いている。赤で色分けして一括訂正していれば、
正しい Evidence を 9 件壊すところだった。**verdict の色ではなく ADR の主張文で判定する**のが
正しい手順である。

**当初「曖昧」と記録した 1 件は誤りだったので取り消す。**
`decisions-test-gate-staleness-repair.md` が引く `test_e2e_bootstrap_fixed_point_stage2_stage3` は
確かに 2 module に同名で存在するが、ADR は**裸の test 名ではなく panic メッセージを引用**しており、
その本文が `crates/lsharp-wasm/tests/e2e/selfhost_bootstrap_acceptance.rs` とファイル名を含んでいる。
参照は一意に解決するので訂正不要である。「Evidence が参照として成立していない」という
新しい失敗モードは、**この sweep では 1 件も観測されなかった**。

受入条件が求めた **(1) どの script も回していない / (2) script が回している の分割は、
実測してみると判別軸として機能しなかった**。43 citation を一意化した 39 test のうち、

| 分類 | 件数 |
|---|---|
| (2) `scripts/**` が test 名を直接書いている | 3 |
| (2') `scripts/**` が prefix で拾う (`--ignored` lane の prefix filter) | 35 |
| (1) どの script も回していない | **1** |

その 1 件は `test_e2e_selfhost_x86_int_to_string_import_sets_rdi` で、
`decisions-native-root-pop-empty-guard.md` が「赤である」と名指ししている分類表の一員、
すなわち上の**一致**にあたる。**つまり (1) から欠陥は 1 件も出なかった。**

**軸を「script が回しているか」から「ADR が何を主張しているか」へ変えたのはこのためである。**
前者で切ると 38/39 が (2) に落ちて何も分からない。受入条件の文言どおりの分割は行ったが、
判定はそれでは付かなかった、という事実をここに残す。

なお (2') の prefix 一致は **「script が prefix を名指ししている」以上のことは言わない**。
その script が当該 test を実際に走らせ、緑を要求しているかまでは確かめていない。

矛盾 4 件の内訳と、訂正の限界:

| ADR | test | 主張 | 実測 |
|---|---|---|---|
| `decisions-v0.3-native-cli-check-file-e2e.md` | `..._main_with_args_check_file` | `Int` / `diagnostics:0` | `Fn` / `diagnostics:0` |
| `decisions-v0.2-selfhost-evidence-parser-duplicate.md` | `..._validate_source_json_reports_contradicting_evidence` | pass (293.49s) | FAILED |
| `decisions-v0.2-selfhost-source-validation-cli.md` | 同上 | `independent_reviews=1` | `0` |

1 件目は**本 ADR の観測ミスではなく、後続の契約変更を反映しなかった**もので、
`914bd9f1` (`I-45`) が 0-arity `defn` を `Unit -> body` にした結果である。訂正は
`docs/adr/decisions-selfhost-zero-arity-defn-type.md` を指すだけで足りる。
**ADR の陳腐化は「古くなる」より「後続の判断に追い越される」形で起きる。**

2・3 件目は `independent_reviews` が `1` ではなく `0` になっている点まで特定したが、
**原因は未診断なので「正しい値」へは書き換えていない**。`I-75` が診断を引き取る。
分からないことを分かったことにして書き換えれば、それは訂正ではなく捏造になる。

**残した宿題** (本 issue では答えない): 環境ゲート 3 件のうち 2 件は `scripts/ci/*` が
回している。その script が現在の CI 環境で通っているかは確認していない。CI の確認は
本作業のスコープ外である。

<a id="i-71"></a>
### I-71: stage-N 生成 Wasm が 3 つの固定 offset で `expected i64 but nothing on stack` になる

- **影響度**: 高 / **状態**: resolved (2026-08-27)
- **発見**: 2026-08-24 (`I-64` の `#[ignore]` 全量 sweep。**本 sweep の最大収量**)
- **内容**: selfhost compiler が出力した Wasm を wasmtime が読めない。
  `WebAssembly translation error / Invalid input WebAssembly code at offset N:
  type mismatch: expected i64 but nothing on stack`。**赤 72 件**。

  | module | 件数 |
  |---|---|
  | `selfhost_bootstrap_four_layer` | 68 |
  | `selfhost_bootstrap_acceptance` | 4 |

  **distinct な offset は 3 つしかない** (メッセージは 3 つとも完全に同一)。

  | offset | 出現 (延べ) |
  |---|---|
  | 329391 | 101 |
  | 457947 | 76 |
  | 310805 | 1 |

- **原因**: selfhost compiler が空 do (`(do)`、expr-count = 0) に対して
  **何も emit しない**。`(if cond (do ...) (do))` は blockty = i64 の `if` を作るので、
  else 腕が空だと `end` の時点で値が積まれておらず型検査に落ちる。
  Rust host (`crates/lsharp-ir/src/lower/expr/do_expr.rs`) は同じ入力に対して
  `I64Const(0)` を unit として積んでおり、**selfhost 側だけが host の契約から外れていた**。
- **起票時の見立ては誤りだった。** 当初は「ftable / function index の誤解決」を疑っていた。
  同一メッセージの先行事例が 2 件あり、どちらも index の誤解決だったためである
  (`workspace-expected-failures.txt:102` の `selfhost_standalone_io` offset 2456、
  `rust-boundary-reduction.md:3106` の `EC-M1-01` offset 2929)。
  **この推論は棄却された。** 症状のメッセージが同じでも原因は同じではない。
  訂正を消すと同じ見立てが再発するので、誤りだった事実をここに残す。
- **3 offset は 3 事象ではなかった。** 実測すると 329391 / 457947 は `src/App/Main.ls` stage2 の
  `func[1231]` / `func[1623]`、310805 は `src/App/CompilerMode.ls` stage2 の `func[1231]` である。
  **同じ 2 関数が、大きさの違う 2 モジュールの別の絶対位置に現れていただけ**だった。
  さらに sweep が一度も挙げなかった 4 つ目 (439361 = `CompilerMode.ls` の `func[1623]`) が存在する。
  **offset の集合は原因の集合ではない**という反証がここにある。
- **これが 3 日見つからなかった理由は `I-77`。** e2e の Wasm 検証 helper が
  関数本体を一つも検証しないため、sweep log は wasmtime が同じモジュールを蹴る 1 行手前で
  `BOOT-04 stage2: wasmparser validation PASSED` を出していた。
- **修正**: `selfhost/src/Backend/Wasm/Compiler.ls` の do 三経路
  (tag 9 dispatch `:1249` / `compile-do-with-source` `:1353` /
  `compile-do-with-source-normal-setup-diagnostic` `:1436`) で `(emit-to instrs 1 0)` を emit する。
  判断と却下した選択肢は
  [`docs/adr/decisions-selfhost-empty-do-unit-value.md`](docs/adr/decisions-selfhost-empty-do-unit-value.md)。
  実測 (壊れている関数 2 → 0、`wasm-tools validate` OK、新規 test 2 件 RED → GREEN) も同 ADR の Evidence 節。
- **fix 後の再測定 (2026-08-27): 症状は消えたが、赤は 1 件も減らなかった。**
  `runtime_allocator_closures` / `selfhost_bootstrap_acceptance` / `selfhost_bootstrap_four_layer`
  の 3 module を同条件で測り直した (180 test / 3 module とも完走 / 赤 88 件)。
  `expected i64 but nothing on stack` の出現は **3 module で 0 件**になり、本件の症状は消滅した。
  一方で **赤の集合は 88 件のまま 1 件も動かなかった**。本件の症状を出していた 75 test は、
  そのまま `expected 11 imports, found 10` (`I-72`) で落ちるようになっただけである。
  **`I-71` は `I-72` を隠していた。** 72 件は `I-71` の帰結ではなく、`I-71` が先に当たる壁だっただけだった。
- **受入条件「GREEN 後、台帳の該当 72 行を削除する」は満たせない。**
  削除すると `scripts/compare_ignored_lane.py` が新規 FAIL 72 件を報告して非 0 になる。
  実測が赤である行を台帳から消すのは台帳を壊す操作なので、**削除せず引き取り先を
  `I-71` → `I-72` へ付け替えた**。条件を静かに緩めたのではなく、条件そのものが
  「1 つの原因が 1 つの赤に対応する」という誤った前提に立っていた。
- **関連**: `I-64` (発見経路)、`I-72` (**本件が隠していた真の壁**。72 行の引き取り先)、
  `I-75` (3 行を再測定で移管。うち 2 行は本件の症状を経て `I-72` へ。
  残り 1 行 (`..._cli_module`) は本件の症状を一度も出しておらず `I-78` へ — 同原因ではない)、
  `I-77` (本件を隠していた検証の穴)。

<a id="i-72"></a>
### I-72: stage-N 生成 Wasm の import 数が 1 つ足りない (`expected 11 imports, found 10`)

- **影響度**: 高 / **状態**: resolved (2026-08-27)
- **発見**: 2026-08-24 (`I-64` の `#[ignore]` 全量 sweep)
- **内容**: 生成された Wasm は**読めるが、インスタンス化できない**。
  `インスタンス化に失敗: expected 11 imports, found 10`。**数値は全件 `11` / `10` で完全に一致**する。
- **`I-71` の fix で赤 8 件 → 82 件になった (2026-08-27)。**
  起票時に見えていた 8 件は「`I-71` に当たらずここまで到達できた」ものだけだった。
  `I-71` (translation) を直すと、そこで止まっていた 74 件がこの壁まで進み、同じ症状で落ちる。
  **本件は `I-71` の後ろに隠れていた真の壁である。** 台帳 82 行が正本だった。

  | module | 件数 |
  |---|---|
  | `selfhost_bootstrap_four_layer` | 76 |
  | `selfhost_bootstrap_acceptance` | 6 |

- **`I-71` とは層が違う。** `I-71` は translation (バイナリが不正)、本件は
  instantiation (バイナリは正しいが host が渡す import 集合と食い違う)。
  同じ module に同居しているので混ぜやすいが、疑う場所が違う。
  `I-71` は codegen、本件は **host 側の import 表と compiler 側の import 宣言の同期**である。
- **原因 (2026-08-27 診断)**: compiler 側 (`selfhost/src/App/CompilerMode.ls:6093,6140`) は
  11 import を宣言する `emit-import-section-alloc-print-read-arg-concat-sub-print-string`
  だけを呼ぶのに対し、e2e harness の共通ヘルパーは 10 import しか渡していなかった。
  **ずれていたのは compiler 側ではなく host 側**である。10-import 版の emitter
  (`WasmEmit.ls:2004`) は production から到達不能で、test 埋め込みの L# ソースにだけ残っていた。
- **解決 (2026-08-27)**: harness の共通ヘルパーを 11 import へ統一した
  (`run_wasm_with_six_imports_compiler_mode*` → `run_wasm_with_eleven_imports_compiler_mode*`)。
  採用案と却下案は
  [`decisions-selfhost-eleven-import-abi-harness.md`](docs/adr/decisions-selfhost-eleven-import-abi-harness.md)
  が正本。fix commit は `12c41d58`。
- **実測 (3 module の部分再測定 / 2026-08-27)**: 3 module とも宣言数 == 結果行ユニーク数で完走判定 OK。
  `expected 11 imports, found 10` は **3 ログとも 0 件**。逆向きの
  `expected 10 imports, found 11` も 0 件。台帳外の新規 FAIL も 0 件。
  台帳 88 行のうち **80 行が緑に転じたので削除**し、8 行が別原因で残った
  (`I-78` 3 件 / `I-80` 2 件 / `I-81` 1 件 / `REPL-TYPE-TAG-01` 1 件 / `[d]` 1 件)。
  取得条件は [`ignored-lane-sweep-2026-08-23.md`](docs/development/operations/ignored-lane-sweep-2026-08-23.md)。
- **判定に使ったのは行数ではなく症状数である。** 行が残っていても、残った理由が
  import 数不一致でなければ本件は解決している。`I-71` で「症状が消えたのに赤が減らなかった」
  前例がある以上、赤の増減は fix の効果の指標にならない。
  本件では赤が 80 行減ったが、**それも判定の根拠にはしていない**。
- **関連**: `I-64` (発見経路)、`I-71` (**本件を隠していた**。72 行の移管元)、
  `I-75` (2 行を移管)、`I-79` (本件の呼び出し元全数調査で発見)、
  `I-80` / `I-81` (本件の解決で下から出てきた層)。

<a id="i-73"></a>
### I-73: native differential の exact-byte pin 33 件が一律にずれている

- **影響度**: 中 / **状態**: open
- **発見**: 2026-08-24 (`I-64` の `#[ignore]` 全量 sweep)
- **内容**: `selfhost_native_differential` の赤 **33 件**。x86-64 / aarch64 の生成バイト列を
  literal で pin している test 群で、**ずれ方に規則性がある**。観測された形は 3 つ。

  1. **frame displacement が一律 8 少ない。** 実測が `240,255,255,255` (= -16) を出す位置で
     pin は `248,255,255,255` (= -8) を期待する。以降も 8 ずつずれたまま最後まで並走する
     (`part_010.rs:314` / `part_011.rs:178` で確認)
  2. **epilogue 最終 byte が `92` (0x5C) vs `93` (0x5D)。** pop 対象 register が違う
  3. **長さ assertion では実測が payload 長ではなく桁違いに大きい値を返す。**
     `left "2563" / right "63"` (`part_003.rs:300`)、`left "3488" / right "28"` (`:217`)、
     `left "3070" / right "584"` (`part_007.rs:184`)、`left "2539" / right "25"` (`:165`)。
     payload の切り出しに失敗して bundle 全体を測っている可能性がある

  **1 と 2 は「一律のずれ」であって、値がランダムに壊れているのではない。**
  frame layout を 8 byte 動かす変更が入り、pin が追随していない形と整合する。
- **受入条件 (a) の答え (2026-08-27、cargo 不使用の git 考古学)。**
  **該当 commit は `361d0d99` "wip: advance linux x86 selfhost native path" (2026-05-17)。
  対応する ADR は無い。**

  | 対象 | commit | 日付 |
  |---|---|---|
  | pin の初出 | `a8fb4914` "Support native direct calls with fifteen args" | 2026-04-13 |
  | 分割で `part_010.rs` へ移動 | `197bf027` "refactor(wasm): split native differential tests" | 2026-07-27 |
  | frame layout を動かした変更 | **`361d0d99`** | **2026-05-17** |

  `361d0d99` は `NativeCodegen.ls` に次を新設し、既存の
  `(local-slot-offset param-index)` を機械的に置換した:

  ```lisp
  ;; selfhost IR は slot 0 を scratch として予約するため、x86 引数は slot 1 から退避する
  (defn native-param-slot-offset-x86 [param-index]
    (local-slot-offset (+ param-index 1)))
  ```

  `local-slot-offset [idx] = (* (+ idx 1) 8)` なので、これは **x86 の param slot を
  一律 1 slot = 8 byte ずらす**。ずれの向きと大きさが実測と一致する。

  **ADR が無いことの確認**: `361d0d99` の commit message は 1 行の `wip:` で body が空。
  `docs/adr/` 全体を `native-param-slot-offset` / `slot 0 を scratch` で grep して 0 件。
  `x86` と frame/stack slot を同時に含む ADR 2 件はいずれも別主題である。
  同 commit は 10 ファイル / +5,391 / -1,325 の大きな wip で、
  `selfhost_native_differential.rs` に +817 行を足しながら**既存 pin の追随はしていない**。
- **(a) の限界を明記する。** `361d0d99` が当該 33 件のバイト列を実際に変えたことは
  **測定では確認していない**。根拠は (i) x86 の frame を明示的に 1 slot ずらす commit が
  履歴上これ 1 本だけ、(ii) pin より後、(iii) ずれの向きと大きさが一致する、
  という状況証拠である。確定には `361d0d99^` と `361d0d99` の両方で emitter を build して
  出力バイト列を差分する必要があり、**それは cargo が要る**。受入条件 (b) の作業に属する。
- **したがって「意図的な変更か regression か」はまだ決まっていない。**
  ADR が無いことは分かったが、ADR の不在は regression の証明ではない。
  決めるのは引き取り先の仕事のままである。
- **33 件の内訳には assertion 以外も 2 件ある**。
  `..._nine_arg_bundle_bytes` は `Os { code: 2, kind: NotFound }`、
  `..._fifty_nine_arg_bundle_bytes` は `support.rs:1685` で panic。**どちらも未診断**。
- **`NATIVE-I32SUB-01` とは別**。あちらは i32 減算の値そのものの誤りで、
  本件は byte 列の配置のずれである。
- **関連**: `I-64` (発見経路)、`I-89` ((a) の調査中に見つかった x86 の到達不能な spill テーブル。
  原因 commit が同じ `361d0d99`)。引き取り先は `TODO.md` の `NATIVE-DIFF-PIN-01`。

<a id="i-74"></a>
### I-74: root lifetime verifier が `main` 以外の helper の `depth: 1` を拒否する

- **影響度**: 中 / **状態**: open
- **発見**: 2026-08-24 (`I-64` の `#[ignore]` 全量 sweep)
- **内容**: `selfhost_cli_core` の赤 **9 件**が
  `RootLifetime { error: ImbalancedExit { function: ..., depth: 1 } }` で落ちる。
  対象関数は fixture 内の helper 2 本のみ。

  | 関数 | 件数 |
  |---|---|
  | `compile-file-state` | 6 |
  | `compile-pair-state` | 3 |

- **`I-14` の案 E では覆えない。** [`decisions-root-lifetime-main-exit-exemption.md`](docs/adr/decisions-root-lifetime-main-exit-exemption.md)
  が免除するのは `function.is_export && function.name == "main"` だけである。
  本件の 2 本は export でも `main` でもない。**案 E の射程外に同じ形の赤が残っていた**。
- **fixture 自身には明示的な `root_push` が無い** (`selfhost_cli_core.rs:256-265`)。
  helper が呼ぶ `push-object-vector` ([`App/CompilerMode.ls:83`](selfhost/src/App/CompilerMode.ls))
  は `root_push` 2 / `root_pop` 2 で均衡しており、verifier は intra-procedural なので
  そもそも算入しない。**したがって `depth: 1` は lowering が挿入した root に由来する疑いが濃い**。
  もしそうなら本件は「verifier が厳しすぎる」ではなく「lowering が root を漏らしている」、
  すなわち **verifier が本物の欠陥を捕まえている**側になる。`I-14` とは向きが逆になり得る。
  **どちらかは未確定であり、判別測定 (`I-14` が使った verifier 無効化対照) が要る。**
- **関連**: `I-14` (同じ verifier、別の射程)、`I-64` (発見経路)、`LEGACY-ROOT-01`。
  引き取り先は `TODO.md` の `ROOT-IMBALANCED-HELPER-01`。

<a id="i-75"></a>
### I-75: sweep で露出した未分類の赤 11 件

- **影響度**: 中 / **状態**: open
- **発見**: 2026-08-24 (`I-64` の `#[ignore]` 全量 sweep)
- **内容**: 新規赤 145 件のうち、`I-71`〜`I-74` /
  `check` の型名 pin (2026-08-27 解決済み) / `REPL-TYPE-TAG-01` のいずれにも収まらない **11 件**
  (起票時 19 件。2026-08-27 の再測定で 4 件に原因が付き、
  2 件を `I-72`、1 件を `I-78`、1 件を `I-80` へ移管した。
  さらに同日 `..._full_inline_mismatch_probe` 1 件を `I-84` へ、
  LSP Position の origin ずれ 2 件を `I-90` へ、
  `..._check_reports_invalid_canonical_case` 1 件を `I-76` へ移管した)。
  症状が 1 件ずつ違い、まとめると嘘になるので**個別に台帳へ載せた上で本 issue が保持する**。
  内訳は [`ignored-lane-expected-failures.txt`](docs/development/validation/ignored-lane-expected-failures.txt)
  の `引き取り先: I-75` 行が正本。

  | module | 件数 | 実測した症状 |
  |---|---|---|
  | `selfhost_cli_core` | 8 | `main-with-args` 系 5 (UTF-8 不正 2 / `exit code 1` 3) / check の import 解決 1 / self-feed compile の OOB trap 1 / `validate-source-json` の数値 1 |
  | `selfhost_native_stage_chain` | 2 | OOB trap 1 / native と selfhost の hash 不一致 1 |
  | `selfhost_lsp_docs_ops` | 1 | formatter の module body canonical text |

  **旧版の表は `selfhost_cli_core` を 11 と書いており、合計が見出しの 19 に対して 18 だった。**
  台帳を数え直した実測は 12 で、移管後の合計 15 と一致した (2026-08-27 前半)。
  **同日さらに 1 件が `I-84` へ移り、`selfhost_cli_core` は 11、合計は 14 になった。**
  移った `..._full_inline_mismatch_probe` は「原因未診断の赤」ではなく
  **構造上必ず赤くなる診断ダンプ**で、分類そのものが誤っていた。


- **2026-08-27 の分類パス (cargo 不使用)。** `I-64` sweep のログ
  (`/Users/biwakonbu/github/tmp/i64/mod-*.log`) から 14 件すべての失敗出力を取り出し、
  **台帳の注記を「原因未診断」から実測症状へ差し替えた**。結果:

  | 判定 | 件数 | 内訳 |
  |---|---|---|
  | 原因が確定して移管 | 3 | LSP Position の origin ずれ 2 (`I-90`) / `check` 型名が `Fn` へ潰れる 1 (`I-76`) |
  | 症状は取れたが原因未確定 | 8 | 下表 |
  | ログから追加情報が取れない | 1 | `..._validate_source_json_reports_contradicting_evidence` (`left Number(0)` / `right 1` だけ) |

  **原因未確定の 8 件のうち 5 件は `main-with-args` の 1 群**で、
  `-o` / `--target` を渡す経路だけが落ちている。2 件は
  `InvalidData: stream did not contain valid UTF-8` で、**stdout に wasm binary が
  出ている疑いがある** (= `-o` が効いていない形)。残り 3 件は `exit code 1` で
  **stderr が捨てられているので読めない**。
  **この 5 件が同一原因かどうかは、stderr が取れるまで決めない。**
  形が似ていることは同一原因の証拠ではない。
- **`EMBEDDED-CLI-OPTION-SPACE-01` との関係は未確認である。** 名前は近いが、
  当該項目が扱うのは option とその値の間の空白の扱いであり、本件の 5 件が
  そこに落ちるかは測っていない。**近そうという理由で束ねない。**

- **`exit code 1` 3 件と `NotFound` 系は stderr が台帳に残っていない。**
  `support.rs:188` が exit code だけを文字列化して捨てている。
  **再現時に stderr を拾えるようにすることが診断の前提**になる。
- **本 issue は保持であって診断ではない。** 残り 11 件それぞれの原因が付いた時点で
  該当分を別 issue へ移し、本 issue から減らす。全部移り終わったら resolved にする。
- **関連**: `I-64` (発見経路)、`I-84` (1 件を移管。誤分類の是正)、
  `I-90` / `I-76` (2026-08-27 の分類パスで移管した先)。
  引き取り先は `TODO.md` の `SWEEP-UNCLASSIFIED-01`。
<a id="i-76"></a>
### I-76: `check` の型名出力は program 型を返すので、式の型を検査する test が成立しない

- **影響度**: 中 / **状態**: open
- **発見**: 2026-08-27 (`check` 型名 pin の追随 slice での判別作業)
- **内容**: `run-check-program` (`selfhost/src/App/Cli.ls:744-745`) は
  `infer-program-analysis-type` が返す **program 全体の型**を `render-type-text` へ渡す。
  program の末尾が `defn` である限り、これは常に関数型である。
  `I-45` (`decisions-selfhost-zero-arity-defn-type.md`、`914bd9f1`) が 0 引数 `defn` を
  `Unit -> body` にしたので、**引数の有無にかかわらず `check` の型名は `"Fn"` に潰れる**。

  影響を受けたのは `test_e2e_selfhost_cli_check_source_builtin_application_type_contract`
  (`selfhost_cli_core.rs:1138`)。fixture は `(defn probe [] (not true))` で、
  assertion は `"Bool"` を要求し「builtin not の戻り値型は Bool であるべき」と書いてある。
  `I-45` 以前は 0 引数 `defn` の型が body 型そのものだったのでこの assertion は成立していたが、
  現在は `Unit -> Bool` になり `"Fn"` が返る。

- **pin を `"Fn"` へ追随させると test は緑になるが、test 名が主張する検査は消える。**
  `"Fn"` はどんな `defn` に対しても返る値なので、builtin `not` の戻り値型を
  1 ビットも区別しない。**緑になることと検査していることは別である。**
  当該 slice では契約追随として pin を動かしたが、
  **失われた coverage を本 issue が保持する**。
- **これは `render-type-text` のバグではない。** 判別済み: `render-type-text` は
  適用結果を関数型へ潰しているのではなく、そもそも適用結果を渡されていない。
  `Cli.ls:715` の tag 分岐は仕様どおり動いている。**実装は触らない。**
- **式の型を検査する経路が `check` にあるべきかは未決**。`check` の契約は
  「program を型検査して型名と診断数を返す」であり、任意の式の型を問う口ではない。
  builtin の戻り値型は `Types/TypeInferBuiltins` 側の unit test で見るのが筋かもしれない。
  **どちらへ寄せるかは本 issue では決めない。**
- **関連**: `I-45` (原因となった契約変更)、`I-69` (同じ `914bd9f1` の別の取り残し)。
  引き取り先は `TODO.md` の `CHECK-BUILTIN-RET-COV-01`。

<a id="i-77"></a>
### I-77: e2e の Wasm 検証ヘルパーが関数本体を一つも検証していない

- **影響度**: 高 / **状態**: open
- **発見**: 2026-08-27 (`I-71` の原因追及中。sweep log の読み違いから)
- **内容**: `crates/lsharp-wasm/tests/e2e` には Wasm を「検証する」名前のヘルパーが 2 つあるが、
  **どちらも関数本体の型検査をしていない**。

  | ヘルパー | 実体 | 実際に見ているもの |
  |---|---|---|
  | `assert_valid_wasm` (`support.rs:692`) | `len > 8` と先頭 4 バイトの `\0asm` | マジックバイトのみ |
  | `validate_wasm_detailed` (`selfhost_bootstrap_four_layer/part_000.rs:139`) | `Validator::payload()` の戻り値を捨てる | section 構造のみ |

  後者が問題である。wasmparser 0.221 の `Validator::payload()` は
  `ValidPayload::Func(FuncToValidate, FunctionBody)` を返し、
  **呼び出し側が `f.into_validator(allocs).validate(&body)` を回して初めて本体が検証される**。
  `validate_wasm_detailed` は `ValidPayload` を `_` で捨てているので、
  code section entry を 1 つも型検査しない。

- **これが `I-71` の発見を 3 日遅らせた。** sweep log には

  ```
  BOOT-04 stage2: wasmparser validation PASSED
  ```

  が、**同じ module を wasmtime が `expected i64 but nothing on stack` で蹴る直前の行**に出る。
  「wasmparser は通るのに wasmtime だけ落ちる」= ランタイム側の差異、と読めてしまう。
  実際には wasmparser が何も見ていなかっただけである。
- **`I-76` と同じ失敗様式である。** 緑になることと検査していることは別である。
  ヘルパー名 (`validate_wasm_detailed` の `detailed`) が検査範囲を偽って伝える分、
  こちらの方が誤読を招きやすい。
- **既存呼び出し箇所の付け替えは本 issue では決めない。** `validate_wasm_detailed` は
  `#[ignore]` 下の多数の test から呼ばれており、本体検証を有効化すると
  それらの verdict が一斉に変わる。`I-71` の slice では
  **新規 test 用に `support.rs` へ `validate_wasm_function_bodies` を足すに留めた**。
  どの呼び出し箇所をいつ付け替えるかは `I-71` の GREEN 後に別途決める。
- **関連**: `I-71` (発見経路。本件が隠していた不具合)、`I-76` (同じ失敗様式)、
  `I-70` (`#[ignore]` 下の根拠が腐る話)。
  引き取り先は `TODO.md` の `WASM-BODY-VALIDATION-01`。

<a id="i-78"></a>
### I-78: stage1 compiler が `src/App/Cli.ls` の self-feed compile で `integer divide by zero` trap する

- **影響度**: 中 / **状態**: open
- **発見**: 2026-08-24 の `#[ignore]` 全量 sweep (当時は `I-75` の未分類バケツに入っていた)。
  2026-08-27 の `I-71` fix 後の再測定で症状を読み出し、独立の issue として分離した。
- **内容**: stage1 compiler (wasm) に `src/App/Cli.ls` を食わせると、
  translation でも instantiation でもなく **実行中に trap する**。

  ```
  stage1 compiler run failed: 実行に失敗: error while executing at wasm backtrace:
    ...
    24: 0x49dd1 - <wasm function 1144>
    25..32: 0x49e61 - <wasm function 1144>   (同一 offset の自己再帰 8 段)
    33: 0x4bb2c - <wasm function 1161>
    34: 0x9087d - <wasm function 1951>
    35: 0x91240 - <wasm function 1954>
    36: 0x184b25 - <wasm function 4172>
    37: 0x184b55 - <wasm function 4174>: wasm trap: integer divide by zero
  ```

- **`I-71` の fix とは無関係**。2026-08-24 の sweep log にも同じ trap が 2 件出ており、
  fix 前後で出現数は 2 のまま変わらない。fix が持ち込んだ regression ではない。
- **`I-71` / `I-72` とは層が違う。** `I-71` は translation、`I-72` は instantiation、
  本件は**実行**。同じ test 群に 3 層が積み重なっているので、
  上の層を直すたびに下の層が新しく見えるだけで、赤の数は減らない。
- **未診断**。除数がどこで 0 になるかは特定していない。`func[1144]` の自己再帰 8 段は
  compiler 内のリスト走査に見えるが、逆アセンブルで確認していない。
- **赤 3 件** (`I-72` 解決後の 2026-08-27 実測)。3 件とも `selfhost_bootstrap_acceptance` で、
  いずれも `src/App/Cli.ls` を食わせた時点で落ちる。

  | test | 位置 | 備考 |
  |---|---|---|
  | `test_e2e_bootstrap_fixed_input_set_stage_chain_match_cli_module` | `part_002.rs:318:18` | 起票時から赤。`wasm trap: integer divide by zero` |
  | `test_e2e_bootstrap_fixed_input_set_stage_chain_match` | `part_002.rs:515:9` | `I-72` との複合だったが、`I-72` 解決後は本件単独 |
  | `test_e2e_bootstrap_stage2_self_feed_fixed_input_set` | `part_002.rs:295:9` | stage2 側。**同一原因とは断定していない** (下記) |

- **3 件目の trap 種別は不明のままである。** harness が `{e}` で整形するのでエラー文字列が
  `<wasm function 4157>; printed=""` で終わり、trap kind が落ちる。
  trace 形状と対象 path は 1・2 件目と一致するが、それは「同じ場所を通った」証拠であって
  「同じ原因である」証拠ではない。`{e:?}` へ変えれば拾える
  (`I-79` と同じ「harness が情報を握り潰す」類型)。
- **関連**: `I-75` (分離元)、`I-71` / `I-72` (同じ test 群の別の層)、`I-64` (発見経路)。
  引き取り先は `TODO.md` の `CLI-SELFFEED-DIVZERO-01`。

<a id="i-79"></a>
### I-79: 実行失敗で assertion が skip される test が 3 件あり、緑のまま何も検査していなかった

- **影響度**: 中 / **状態**: **resolved (2026-08-27)**
- **発見**: 2026-08-27 (`I-72` の呼び出し元全数調査)
- **内容**: 実行が `Err` を返したときに `if let Ok(..)` / `match` の `Ok` 側だけに書かれた
  assertion が丸ごと skip され、test は緑のまま何も検査しない。

  ```rust
  if let Ok(output) = result {          // Err なら以降が丸ごと消える
      assert_valid_wasm(&modules[0]);
      assert_ne!(run_output, "7\n", "...");
  }
  ```

- **起票時に書いた「8 件」は分類が誤っていた。** 全 `*.rs` を brace matching で走査した結果、
  1 つの形として書いていたものは 4 つの異なる形の混合だった。

  | 形 | 定義 | 実測 | 起票時に挙げた 8 件のうち |
  |---|---|---|---|
  | (b) | Ok 側に assertion があり、`Err` で skip される | 5 箇所 / 3 test | **1 件だけ** |
  | (c) | `Result` を束縛して `{:?}` 表示のみ。assertion が最初から無い | 9 箇所 / 6 test | 4 件 |
  | (a') | `Err` 腕が `eprintln!`、かつ `Ok` 腕にも assertion が無い | 2 箇所 / 2 test | 2 件 |
  | (d) | `assert!` はあるが構造上恒真 | 1 箇所 / 1 test | 1 件 |

  **本 issue が扱うのは形 (b) だけである。** 形 (b) は「書かれた assertion が実行されない」ので
  直し方に判断の余地が無い。形 (c)/(a')/(d) は assertion がそもそも無く、直すには
  「この probe は何を保証すべきか」を新たに決める必要がある。`I-82` へ移した。
- **「8 件は全て `selfhost_bootstrap_four_layer` にある」も誤りだった。** 形 (b) の 3 test は
  3 つの別 module に散っている。

  | test | module | 是正後 |
  |---|---|---|
  | `test_e2e_boot04_compiler_mode_ignores_dotted_flat_file` | `selfhost_bootstrap_four_layer` (`part_015`) | **FAILED** → `I-83` |
  | `test_native_codegen_real_execution` | `selfhost_native_differential` (`part_001`) | ok (挙動不変) |
  | `test_e2e_selfhost_type_error_parity` | `selfhost_type_parser_parity` | FAILED → fixture 修正で ok |

- **RED は「走らせて赤くなること」では取れなかった。** `I-72` の fix 後、3 件はいずれも
  実際に走って緑である。**入力を意図的に壊しても緑のままであること**を RED の証拠とした。
  `test_native_codegen_real_execution` に壊れた S 式を注入すると、結果は `ok` のまま、
  所要は **19.68s → 0.06s** に落ちた。**所要時間の落差が、何もしていないことの証拠である。**
- **`I-64` の sweep がこれを拾えなかった理由**もここにある。sweep は落ちた test を数える。
  落ちない test は、検査していなくても数に入らない。
  **「緑になることと検査していることは別である」の実例**である。
  同型は `I-77` (検証ヘルパーが関数本体を一つも見ていない) と
  `I-70` (ADR の Evidence が `#[ignore]` 下の test を根拠にしていた) に既にある。
- **解決** (2026-08-27): 3 件の `Err` 腕を `panic!` へ置き換えた。skip カウンタは導入していない
  (「n 件までは skip してよい」という閾値は、閾値以下の恒常 skip を正常として固定するため)。
  判断と却下理由は [`decisions-harness-swallowed-error-arms.md`](docs/adr/decisions-harness-swallowed-error-arms.md)。
- **方法論**: **「同じ形が何件あるか」を数える前に、その形の定義を実物で確かめること。**
  定義を確かめずに数えると、数は合っているのに中身が違うという誤りが台帳に残る。
  これは件数の検算では発見できない種類の誤りである。
- **関連**: `I-72` (発見経路)、`I-82` (本 issue が範囲外とした形)、`I-83` (是正で表に出た実バグ)、
  `I-77` / `I-70` (同じ「緑だが検査していない」類型)、`I-64` (sweep が拾えなかった)。

<a id="i-80"></a>
### I-80: target-defn probe が AST の形を添字直打ちで辿り、`make-type-constrained` の refactor に追随していない

- **影響度**: 中 / **状態**: open (実装は 2026-08-27 に完了。lane 再計測待ち)
- **発見**: 2026-08-27 (`I-72` の fix 後の部分再測定で露出)
- **内容**: `selfhost_bootstrap_four_layer` の target-defn parity probe 2 件が、
  marker の値が期待値に届かずに落ちる。どちらも対象 defn は `make-type-constrained`
  (台帳が長く使ってきた `ast-make-type-constrained` という名前は**ソースのどこにも存在しない**)。

  | test | marker | 実測 | 期待 | 位置 |
  |---|---|---|---|---|
  | `test_e2e_boot04_stage1_target_defn_parity_reports_ast_make_type_constrained_lengths` | 126 | 5 | 7 | `part_009.rs:411:5` |
  | `test_e2e_boot04_self_hosted_stage2_target_defn_parity_reaches_ast_make_type_constrained` | 127 | 0 | 5 | `part_009.rs:302:5` |

- **`I-72` の下から出てきた層である。** どちらの test も `I-72` の解決前は
  インスタンス化で止まっており、この assertion まで到達していなかった。
  したがって **`I-72` の fix が持ち込んだ regression ではない**。
  ただし「fix 前は緑だった」ことの実証でもない — fix 前は assertion が走っていないので
  比較できる過去の値が存在しない。
- **診断済み (2026-08-27)。compiler の regression ではなく probe の陳腐化である。**
  probe (`selfhost/src/App/CompilerMode.ls` の `target-defn` モード) は
  `make-type-constrained` の AST を**添字直打ち**で辿り、body が
  `(let [v (vector-new 2)] (vector-push (vector-push v ...) ...))` の形をしていることを
  前提にしている。現在の定義 (`selfhost/src/Syntax/AST.ls:260`) は
  `(vector-push-pair-rooted (vector-new 2) (ast-typeconstrained) name-hash)` で
  **`let` が無い**。よって marker 126 (`body[0]`) は `ast-let` (7) ではなく
  `ast-apply` (5) になり、marker 127 (`body[3][3][4]` の tag) は平坦化された AST の
  外を指す。**2 つの実測値がこれ 1 つで説明できる。**

  probe 本体 (`CompilerMode.ls:6323-6327`) のナビゲーションは次の 5 行である。
  `let` を前提にしていることがそのまま読める。

  ```lisp
  decl        (vector-get decls target-idx)
  body        (vector-get decl (+ 3 (vector-get decl 2)))
  outer-expr  (vector-get body 3)
  inner-call  (vector-get (vector-get outer-expr 3) 4)
  inner-func  (vector-get inner-call 1)
  ```

- **範囲外読み出しの値は stage1 と stage2 で違う (2026-08-27 の再計測で判明。上の記述の訂正)。**
  本 issue は当初「marker 127 は…外を指して **0 になる**」と書いたが、これは stage2 側だけの話だった。
  stage1 側は 126 で落ちるので 127 を assert しておらず、値が見えていなかった。
  full dump を取ると **stage1 の 127 は `4294967296` (= 2^32)、128 は `72057594054705152`
  (= 2^56 + 2^24)** で、stage2 の `0` / `0` とは違う。

  | marker | 意味 (emitter より) | stage1 実測 | stage2 実測 | 期待 |
  |---|---|---|---|---|
  | 124 | `decl[0]` (decl tag) | 20 | 20 | 20 |
  | 125 | `decl[2]` (param 数) | 1 | 1 | 1 |
  | 126 | `body[0]` (body tag) | **5** | **5** | 7 |
  | 127 | `inner-call[0]` | **4294967296** | **0** | 5 |
  | 128 | `inner-func[0]` | **72057594054705152** | **0** | 4 |
  | 129 | `inner-func[1]` (use-site hash) | 0 | 0 | `== 131` |
  | 130 | `ftable-lookup ftable 129` | 0 | 0 | `> 0` |
  | 131 | `decls[31][1]` (def-site hash) | -5490128408457682031 | 同左 | -- |

  **範囲外読み出しは 0 を返すとは限らない。** 同じ probe を同じ入力で走らせても、
  stage1 (Rust) と stage2 (self-hosted) で違う値が出る。片方だけ見て
  「0 が返る」と一般化したのが誤りだった。
- **126/127 を直しても緑にならない可能性が、予測から実測へ変わった。**
  full dump では 129 (=0) と 131 (=-5490128408457682031) が一致せず、130 も 0 である。
  現在の assertion 順ではここまで到達しないが、**到達すれば落ちる値が今そこにある**。
  ただしこれらは壊れた navigation の下流の値なので、
  **navigation を直せば再計算される。「129 は直後に落ちる」と読むのは誤りである**
  (「offset の集合は原因の集合ではない」)。確定しているのは
  「126/127 の緑を完了条件にしてはいけない」という一点。
- **時系列が裏付ける。** probe 本体と test 名はどちらも `357f261d` (2026-04-11) 生まれ。
  `AST.ls:260` の `vector-push-pair-rooted` 化は `901c10d8` (2026-04-22)。
  **refactor は probe より 11 日新しく、probe は追随していない。**
  旧 shape は `part_009.rs:456` の minimal fixture 文字列に残骸として残っている。
- **当初の原因候補はどちらも外れていた。** 「対象 defn が短く切れている」でも
  「別の defn を見ている」でもない。同じ probe の marker 124 (`decl[0]` = 20 = `ast-defn`) と
  marker 125 (param 数 = 1) は**通っている**。probe は**正しい defn を正しく見つけており**、
  壊れているのは body 内ナビゲーションだけである。
- **stage1 側と stage2 側は同一原因である (当初の見立ての訂正)。**
  「marker が別なので原因が同じとは限らない」と書いたが、実物は
  **同じ `target-defn` probe を stage1 の binary と stage2 の binary で走らせているだけ**で、
  落ちる marker が違うのは assertion の並び順の差にすぎない
  (stage1 側は 126 を `== 7` で見るので 126 で、stage2 側は 126 を `> 0` でしか見ないので 127 で落ちる)。
  **慎重さは実物を読むまでの態度であって、読んだ後まで持ち越すものではない。**
- **未検証の層が残っている。** marker 129 以降 (`129 == 131` の hash 一致、`130`/`132`/`133` の
  lookup 非空) は 126/127 で落ちるため**一度も評価されていない**。
  probe 本体は marker 131 以降のために `(vector-get decls 31)` も添字直打ちしている。
  **126/127 を直すと下から新しい赤が出る可能性がある** (`I-72` → `I-80` と同じ構造)。
  「126/127 が緑になった」を完了条件にしないこと。
- **裁定は済んでいる**: `docs/adr/decisions-target-defn-probe-shape-drift.md`。
  stage2 側はリテラル pin をやめ stage1 出力との parity 比較にする。
  stage1 側は shape pin として残しリテラルを実測へ更新する。minimal fixture は現在の shape へ更新する。
- **是正 (2026-08-27)**: 裁定 1〜3 をすべて実装した。stage2 側は期待値リテラルを全廃して
  stage1 出力との parity 比較にし、stage1 側は shape pin として 126 を 5 へ更新、
  minimal fixture は現在の `vector-push-pair-rooted` 形へ差し替えた
  (302 が 7 → 5 になるという予測を立ててから測り、そのとおりになった)。
  3 件とも個別実行 `===EXIT 0`。**lane 再計測待ち。**
- **marker 129 以降の初回評価では新しい赤は出なかった。ただし元の assertion は成立し得なかった。**
  `129 == 131` / `130 > 0` / `133 > 0` はナビゲーションが壊れている以上、前提が偽である。
  assertion を「壊れている状態を pin する」側 (`== 0`) へ付け替え、
  **probe 本体が直されたら赤くなる向き**に置き直した。本来の assertion を復元すべきことは
  `I-88` へ引き取らせた。
- **裁定 2 の文言のうち 1 点は実行しなかった。** 「127 を現在の shape での実測へ更新する」は、
  実測値 (stage1 `4294967296` / stage2 `0`) が**範囲外読み出しの binary 依存なゴミ**だったため、
  pin せず parity の比較対象からも外した。判断と根拠は ADR の「満たせなかったこと」に書いた。
- **関連**: `I-72` (これを隠していた)、`I-75` (`..._lengths` の移管元)、`I-64` (発見経路)、
  `I-82` (probe が主題を検査していない類型)、`I-84` (構造上必ず赤くなる probe)、
  `I-88` (却下案 B の代償)。
  引き取り先は `TODO.md` の `TARGET-DEFN-PARITY-01`。

<a id="i-81"></a>
### I-81: `local_bound_violation_indices` が 0 件になり、violation 前提の診断足場が落ちる

- **影響度**: 中 / **状態**: open (実装は 2026-08-27 に完了。lane 再計測待ち)
- **発見**: 2026-08-27 (`I-72` の fix 後の部分再測定で露出)
- **内容**: `test_v2_12_self_hosted_stage2_reports_compiler_mode_first_violation_body_diff`
  (`part_014.rs:205:10`) は stage3 の Wasm から `local_bound_violation_indices` を集め、
  **最初の violation の body diff を出す**診断足場である。
  収集結果が空になり、`first violation` を取り出す時点で落ちる。
- **前提が違っていた (2026-08-27 訂正)。この test は一度も緑になったことがない。**
  当初「改善した結果として足場が成立しなくなった」と書いたが、実物を読むと
  **body に分岐も `return` も無く、最後に無条件 `panic!` する**。
  つまり violation が在っても無くても赤で、違いは「どこで落ちるか」だけである。
  是正前は末尾の `panic!` で詳細ダンプを出し、今は手前の `.expect` で落ちる。
  `local_bound_violation_indices` が 0 件になったのは**良い状態**であり、
  「足場が壊れた」のではなく**足場が最初から test として成立していなかった**。
- **裁定は「極性を反転して恒久的な regression guard にする」**。
  violation 0 件を緑とし、再発したときに従来の full dump を assertion の失敗メッセージとして出す。
  当初案 (b) (violation を含む fixture を与えて足場のまま保つ) は却下した —
  恒久的に赤い test は台帳に永久の 1 件を積むだけで、情報を運ばない。
  当初案 (a) への懸念「violation 再発に気付けなくなる」は、
  **極性を反転すれば消える** (0 件が正常、再発が赤)。
  詳細は [`docs/adr/decisions-always-failing-diagnostic-probes.md`](docs/adr/decisions-always-failing-diagnostic-probes.md)。
- **`I-72` の下から出てきた層である。** 解決前はインスタンス化で止まっており、
  この収集まで到達していなかった。
- **解決 (2026-08-27)**: 裁定どおり極性を反転した。末尾の無条件 `panic!` を
  `let Some(&first_bad) = bad_indices.first() else { return; };` で包み、violation 0 件を緑にした。
  ダンプは失敗メッセージとして残してある。test 名は
  `test_v2_12_self_hosted_stage2_compiler_mode_has_no_local_bound_violation` へ改めた。
  **検出力は別に証明した** — 非 ignore の
  `test_local_bound_violation_indices_detects_out_of_range_local` (`part_017.rs`) が、
  範囲外の local を仕込んだ入力で `local_bound_violation_indices` が実際に検出することを固定する。
  これを置かずに反転すると `I-82` と同じ「常に緑で何も見ていない」test になる。
  個別実行は緑 (`--exact ... --ignored`、exit 0)。
  `docs/development/validation/ignored-lane-expected-failures.txt` の該当行は削除した。
  **`selfhost_bootstrap_four_layer` の lane 再計測は未了** (`AGENTS.md` の改名再計測規約)。
- **関連**: `I-72` (これを隠していた)、`I-79` (「緑だが検査していない」の裏返しで、
  こちらは「赤だが欠陥は無いかもしれない」)、`I-64` (発見経路)、
  `I-84` (本 issue の裁定中に見つかった同型 4 件)。
  引き取り先は `TODO.md` の `VIOLATION-PROBE-STALE-01`。

<a id="i-82"></a>
### I-82: test 名が主張する主題を検査していない probe test が 13 件あり、常に緑になる

- **影響度**: 中 / **状態**: open
- **発見**: 2026-08-27 (`I-79` の全数調査)
- **内容**: 実行結果を `eprintln!` / `println!` するだけで、**test 名が主張する主題を
  一度も検査していない** test が **13 件 / 16 箇所**ある。`I-79` (assertion が skip される) とは別で、
  こちらは**最初から無い**。したがって入力が何であれ、実行が成功しようが失敗しようが、常に緑になる。

  **基準はこうである** (当初の記述を 2026-08-27 に訂正した。下記「枠組みを直した」を見よ)。

  > 1. test 名またはコメントが**主題**を宣言している
  > 2. その主題について、**結果を検査する assertion が無い** — 表示するだけ、または恒真な `assert!`
  >
  > 中間結果に assertion があっても、**主題が未検査なら対象**とする。

  | 形 | 位置 | test | ignore |
  |---|---|---|---|
  | (c) | `four_layer/part_008.rs:344` | `..._stage2_reports_main_again_cache_pairs_progress` | yes |
  | (c) | `four_layer/part_008.rs:455` | `..._stage2_reports_main_again_progress` | yes |
  | (c) | `four_layer/part_011.rs:390` | `..._stage2_reports_module_resolver_progress` | yes |
  | (c) | `four_layer/part_011.rs:434` | `..._stage2_reports_string_length_if_progress` | yes |
  | (c) | `four_layer/part_015.rs:587` | `test_i64_if_condition_validity` | **no** |
  | (c) | `stage_chain.rs:54969-54984` (4 箇所) | `..._representative_const_only_entrypoint_helper_offsets` | yes |
  | (a') | `four_layer/part_015.rs:620` | `test_parse_compiler_ls` | **no** |
  | (a') | `four_layer/part_015.rs:633` | `test_parse_caws_standalone` | **no** |
  | (a') | `four_layer/part_015.rs:674` | `test_debug_stage2_output_minimal` | yes |
  | (a') | `four_layer/part_015.rs:707` | `test_validate_stage2_wasm` | yes |
  | (a') | `four_layer/part_016.rs:296` | `test_debug_stage3_output_chars` | yes |
  | (a') | `four_layer/part_016.rs:382` | `test_debug_stage3_main_again_output_chars` | yes |
  | (d) | `four_layer/part_014.rs:596` | `..._stage2_classifies_chunked_lexer_failure_band` | yes |

- **3 件は `#[ignore]` を持たない。** `test_i64_if_condition_validity` /
  `test_parse_compiler_ls` / `test_parse_caws_standalone` は通常 lane で毎回走りながら、
  実行結果に対する assertion を 1 つも持たない。
  とくに `test_parse_compiler_ls` は「`Compiler.ls` をパースして構文エラーを検出する」と
  コメントに書きながら、**パース結果を `eprintln!` するだけ**である。
- **枠組みを直した。件数と定義を合わせて 3 度動いている。** 最初の手作業で 9 件と数え、
  走査を `scripts/sweep_unchecked_result.py` として書き直したときに `part_015` の 4 件が追加で出た。
  **手で数えた 9 という数は、走査の網羅性ではなく目視の到達範囲を表していた。**
  さらに 13 件の実物を全部読んだ結果、**当初の見出し「assertion を 1 つも持たない」が
  13 件中 6 件で成り立たない**ことが分かった。それらは `.expect(...)` や `assert_valid_wasm(...)` で
  中間結果を検査しており、検査していないのは主題の方である。
  1・2 度目は数え漏れだが、**3 度目は数える対象の定義が間違っていた**。
  これは件数の検算では発見できない種類の誤りである。
- **形 (d) は「恒真な assertion」である。** `assert!(matches!(classification, A | B | C | D | E))`
  と書かれているが、`classification` はその 5 値のいずれかにしか成りえない構造で作られている。
  **assertion があることと、検査していることは別である。**
- **裁定は済んでいる。実装は 13 件中 12 件まで進んだ (2026-08-27)。**
  [`docs/adr/decisions-probe-subject-unchecked.md`](docs/adr/decisions-probe-subject-unchecked.md) が
  test ごとの裁定を確定した。内訳は **assertion 追加 8 件 / 削除 4 件 (+ 基準外の隣接 1 件) /
  恒真 assert の実質化 1 件**。一括削除・一括 `panic!`・一括 `#[ignore]` の 3 案はいずれも却下した。
  削除 4 件は実施済み (`grep` 0 hit)。実質化は #1〜#4 / #5〜#7 / #9 / #12 の 9 件が完了した。
  **残るのは #13 `..._const_only_entrypoint_helper_offsets` の 1 件**で、これは
  `selfhost_native_stage_chain` に属するため four_layer の lane では覆えない。
  実測値と pin の型は ADR の Evidence 節が正本。**four_layer の lane 再計測も未了。**
- **`test_i64_if_condition_validity` は極性が逆だった。** fixture
  `tests/fixtures/selfhost-debug/test_i64_if.wasm` は**仕様上不正な wasm** で
  (`if` 条件が i64 / `if (result i64)` に else が無い)、正しい契約は
  「wasmparser と wasmtime の**両方が reject する**」である。
  名前から `is_ok()` を assert すると赤になり、赤を消そうとして fixture を壊す連鎖に入る。
  なお 2 つ目の欠陥が出すエラー形 (`expected [i64] but got []`) は `I-83` の
  `expected i64 but nothing on stack` と同じだが、**offset の集合は原因の集合ではない**ので
  参照に留める。
- **実装には four_layer の再計測が 1 本要る** (前回実測 6748s ≈ 112 分)。
  13 件中 12 件が `selfhost_bootstrap_four_layer` に属し、test の追加・削除は
  ignored lane の母集団を変えるため、`AGENTS.md` の partial-lane 規約 (`d29cb5a1`) が再計測を要求する。
  **four_layer に触る裁定は 1 つの slice に束ねること。**
- **走査は `scripts/sweep_unchecked_result.py` にある。** 現状 18 件を出力し、
  うち 2 件は既知の偽陽性である (`part_007.rs:264` は直後に `assert_valid_wasm` があり、
  `part_014.rs:651` は `summarize` クロージャの中)。
  **出力をそのまま件数として使わないこと。** 判定は必ず該当箇所を開いて行う。
- **関連**: `I-79` (本 issue の親。形 (b) だけが解決済み)、`I-81` (同種の probe 裁定だが対象は別 test)、
  `I-83` (`test_i64_if.wasm` と同型の症状)、`I-77` / `I-70` (「緑だが検査していない」類型)。
  裁定は [`docs/adr/decisions-probe-subject-unchecked.md`](docs/adr/decisions-probe-subject-unchecked.md)。
  引き取り先は `TODO.md` の `PROBE-ASSERTS-NOTHING-01`。

<a id="i-83"></a>
### I-83: compiler-mode が生成した wasm が stack 不整合で load できない

- **影響度**: 高 / **状態**: open
- **発見**: 2026-08-27 (`I-79` の是正で初めて実測)
- **内容**: `test_e2e_boot04_compiler_mode_ignores_dotted_flat_file` は、
  `src/App/Main.ls` と `src/Syntax.Token.ls` (dotted 名の flat file) を置いた fixture を
  stage1 の compiler-mode に食わせ、**生成 wasm を実行して** 出力が `"7\n"` でないこと
  (= dotted flat file を module source に採っていないこと) を確かめる test である。
  生成された wasm が wasmtime で load できない。

  ```
  Wasm モジュールの読み込みに失敗: WebAssembly translation error
  Caused by:
      Invalid input WebAssembly code at offset 270: type mismatch: expected i64 but nothing on stack
  ```

- **これは `I-79` の是正で新たに壊れたのではない。** 是正前は実行失敗が
  `if let Ok(..)` に握り潰され、**この test が検証しようとしていた契約は一度も測られていなかった**。
  したがって「いつから壊れているか」は不明であり、**回帰ではなく初回計測**として扱う。
- **`assert_ne!` はまだ一度も実行されていない。** load 段階で落ちるため、
  「compiler-mode が dotted flat file を module source に採るか」という**本来の問いは未解決**である。
  `I-83` を直しても、その先に別の赤がある可能性がある。
- **`I-72` の 11-import 統一とは無関係である。** import 数の不一致ではなく、
  生成コードの stack balance の問題であり、offset 270 は code section 内を指す。
  ただし同型の症状 (`values remaining on stack at end of block`) は
  `part_015` の step512 診断が別経路で固定しており、**同じ根が疑われる**。
  ただし **offset の集合は原因の集合ではない**ので、同一と決めつけないこと。
- **関連**: `I-79` (発見経路)、`I-78` (compiler-mode self-feed の別の赤)、
  `I-77` (wasm body validation)。
  引き取り先は `TODO.md` の `COMPILER-MODE-STACK-01`。

<a id="i-84"></a>
### I-84: 構造上必ず赤くなる test が 5 件、台帳に恒久的な赤として載っている

- **影響度**: 中 / **状態**: open
- **発見**: 2026-08-27 (`I-81` の裁定中に `scripts/sweep_always_failing_tests.py` を書いて走査)
- **内容**: body に分岐も `return` も無く、**最後に無条件 `panic!` する** `#[test]` が **5 件**ある。
  入力が何であれ必ず赤になる。調査中に書いた診断ダンプがそのまま checked in された形である。

  | 位置 | test | 台帳の現状 |
  |---|---|---|
  | `four_layer/part_014.rs:154` | `..._reports_compiler_mode_first_violation_body_diff` | 引き取り先 `I-81` |
  | `selfhost_cli_core.rs:2870` | `..._direct_module_resolver_full_inline_mismatch_probe` | 引き取り先 `I-75` (**誤分類**) |
  | `stage_chain.rs:26574` | `..._representative_crash_offset_maps_to_rust_function` | `# diagnostic:` 注記 |
  | `stage_chain.rs:26623` | `..._representative_post_entry_call_targets_map_to_source_order` | `# diagnostic:` 注記 |
  | `stage_chain.rs:26660` | `..._representative_crash_x8_offset_maps_to_source_order` | `# diagnostic:` 注記 |

  5 件とも `#[ignore]` を持ち、5 件とも
  [`ignored-lane-expected-failures.txt`](docs/development/validation/ignored-lane-expected-failures.txt) に載っている。
- **台帳の契約と噛み合っていない。** `scripts/compare_ignored_lane.py` は
  「緑に転じた台帳行は削除する」ことを前提に作られているが、**この 5 件は緑に転じ得ない**。
  結果として恒久的に 5 行を占め、未解決の欠陥と同じ見え方をする。
- **`I-75` は 1 件を誤分類していた。** `..._full_inline_mismatch_probe` は
  「原因未診断の赤」として `I-75` が保持していたが、実際には原因も何も
  **成功経路が存在しない**。`I-75` は 15 → 14 件になる。
- **3 件は crash アドレスを直書きしている。** `0x6200d0` / `0x621700` /
  `0x1674bc` 他 / `0x106d24` は特定の native crash 調査で得た生アドレスで、
  codegen が動けば意味を失う。**契約ではない。**
- **2 件は極性を反転できる。** `..._first_violation_body_diff` と
  `..._full_inline_mismatch_probe` はどちらも「無いことが正常」な性質
  (local bound violation / 2 回コンパイルの Wasm mismatch) を probe しており、
  現在の実測は「無い」。ダンプを assertion の失敗メッセージへ移せば、
  **情報を 1 bit も失わずに恒久的な guard になる。**
- **反転する前に、検出器が本当に検出できることを確かめる必要がある。**
  確かめずに反転すると `I-82` と同じ「常に緑で何も見ていない」test になる。
  RED の取り方は `I-79` 形 (b) と同じで、**入力を意図的に壊して赤になることを見る**。
- **走査は `scripts/sweep_always_failing_tests.py` にある。**
  `#[test]` の body を brace matching し、最後の top-level 文が
  `panic!` / `unreachable!` / `todo!` / `unimplemented!` のものを出す。
  **出力をそのまま件数として使わないこと。** マクロ内や `cfg` 分岐は判定しきれない。
- **関連**: `I-81` (発見経路。5 件のうち 1 件)、`I-75` (誤分類していた 1 件)、
  `I-82` (「緑だが検査していない」の裏返し)。
  裁定は [`docs/adr/decisions-always-failing-diagnostic-probes.md`](docs/adr/decisions-always-failing-diagnostic-probes.md)。
  引き取り先は `TODO.md` の `ALWAYS-RED-PROBE-01`。

<a id="i-85"></a>

### I-85: `test_debug_boot04_*` 12 件の主題 assertion が「空でないこと」だけを見ている

- **影響度**: 中 / **状態**: open
- **発見**: 2026-08-27 (`I-82` の裁定 5 を書く途中、削除 4 件の引き取り先を静的に確かめていて発見)
- **内容**: `selfhost_bootstrap_four_layer` の `test_debug_boot04_*` **12 件**が、
  probe が出した値そのものを `eprintln!` へ流し、主題の assertion は
  **`assert!(!output.trim().is_empty())` 1 行だけ**という同一構造を持つ。

  | fragment | 件数 |
  |---|---|
  | `part_009.rs` | 4 (`:507` / `:577` / `:635` / `:696`) |
  | `part_010.rs` | 7 (`:71` / `:166` / `:261` / `:356` / `:468` / `:584` / `:703`) |
  | `part_011.rs` | 1 (`:114`) |

  12 件とも構造が同じで、`assert_valid_wasm(...)` 1〜2 個 (中間結果) +
  `eprintln!("... values = {:?}", output)` + 上記 1 行、で終わる。
- **`I-82` の 13 件には含まない。件数を動かさない。**
  `I-82` の基準は「主題を検査する assertion が**無い**」であり、この 12 件には有る。
  literal に恒真でもない (出力が空なら落ちる)。**基準の外にある。**
  `I-82` の件数はこれまで 3 度動いており、そのうち 1 回は数える対象の定義が誤っていたことによる。
  **基準を後から広げて件数を動かすのは、その 4 回目になる。**
- **それでも問題である理由**: probe の名前が主張しているのは
  「first-defn の値」「build compile progress の marker 列」といった**値の内容**であって、
  出力が存在することではない。値が全部 0 に化けても、marker の順序が崩れても、この test は緑のままである。
  `I-82` の #12 (恒真 `assert!(matches!(...))`) と同じ帯にいる。
- **是正の型は既にリポジトリ内にある。**
  `four_layer/part_008.rs:471` の `..._reports_main_again_build_progress` は
  同じ debug 出力を `Vec<i64>` へ parse し、`values[0] == 50` / marker 50..=67 の順序を
  `ordered_marker_positions(...)` で検査し、末尾の wasm size 一致まで見ている。
  **新規設計は要らない。この形へ寄せる。**
- **12 件は全部 `#[ignore]` 側なので、是正には `selfhost_bootstrap_four_layer` の
  再計測 1 本 (前回実測 6748s ≈ 112 分) が要る。**
  `I-82` の実装 slice と同じ module なので、**束ねて lane 1 本で覆うのが安い**。
- **是正 (2026-08-27。lane 再計測は未了)**: 12 件とも実質化した。pin の強さは入力の由来で分けた —
  test 内リテラルが入力なら全値 `assert_eq!`、実在の `.ls` が入力なら marker は exact で
  数値は下限と関係式 (ADR `decisions-probe-subject-unchecked.md` の 裁定 7 が正本)。
  共通の構造検査は `part_018.rs` の `assert_build_compile_progress_shape` /
  `assert_debug_progress_shape` に集約した。
  `grep -rn 'trim().is_empty()' .../selfhost_bootstrap_four_layer/` の hit は
  無関係な `.filter()` だけになった。
- **是正の過程で本物のバグが 2 件出た。「空でない」で通っていたものは、実際に壊れていた。**
  - `..._first_defn_ir_parity_on_minimal_demo_main_shape` は probe 名を arg18 に置いており、
    実際には `cache-compile-phase-probe` が走っていた (`App/Main.ls` の dispatch は
    **arg スロットだけで probe を選び、probe 名の文字列は読まない**)。arg13 へ直した
  - `..._first_defn_probe_on_minimal_make_type_constrained_shape` の fixture は
    preopen 外に置かれており **1 バイトも読まれていなかった** (`301,-1` = defn 0 件)。
    fixture の置き場を直して stage1 / stage2 とも `[301, 0, 302, 7]` になった。
    空文字列に潰れる `read-file` 自体は `I-87` として登録した
- **関連**: `I-82` (発見経路。基準の外という判定も含む)、`I-84` (「常に赤い probe」の裏返し)、
  `I-87` (是正中に露出した本体側の欠陥)。
  引き取り先は `TODO.md` の `WEAK-SUBJECT-ASSERT-01`。

<a id="i-86"></a>

### I-86: selfhost parser が Rust reference parser より緩く、不正な構文を `diagnostics:0` で受理する

- **影響度**: 中 / **状態**: open
- **発見**: 2026-08-27 (`I-82` の #7 `test_parse_caws_standalone` に assertion を足すため、
  fixture が本当にパースできるのかを実測して発見)
- **内容**: 同じソースに対して Rust reference parser (`lsharp_syntax::parse`) と
  selfhost parser (`lsharp parse` は native selfhost へ委譲される) の判定が割れる。

  | 入力 | Rust reference | selfhost `parse` |
  |---|---|---|
  | `(defn main [] (if (> 1 0) 42))` -- 2 引数 `if` | `Err` (`expected "式", found ")"`) | `decls:2 diagnostics:0` |
  | `(defn main [] (if (> 1 0) 42 0))` -- 3 引数 `if` | `Ok decls=2` | `decls:2 diagnostics:0` |
  | `(module T)` + `@@@ ###` | `Err` (`Lex UnexpectedChar '@'`) | `decls:7 diagnostics:0` |
  | `(defn main [] (if (> 1 0) 42 0)` -- 閉じ括弧不足 | (未計測) | `diagnostics:1,P0001@1:1` |

  L# の `if` は 3 引数である (`crates/lsharp-syntax/src/parser/expr.rs:194` の `parse_if` は
  cond / then / else を順に必須で読む)。**Rust 側が正しく、selfhost 側が緩い。**
- **`diagnostics` チャネル自体は生きている。** 閉じ括弧不足は `P0001` として報告される。
  報告経路が無いのではなく、**この 2 形が検査されていない**。
- **top-level のゴミが decl として数えられる方が重い。** `@@@ ###` が `decls:7` になるということは、
  selfhost の top-level が `(` 以外のトークンを decl 境界として受け入れている。
  構文エラーが decl 数の水増しとして通過する。
- **発見経路**: `tests/fixtures/selfhost-debug/test_caws.ls` は `compile-apply-with-source` の
  古い inline 版を抜き出した fixture で、offset 1795 の `(if (> arg-count 0) (do ...))` が
  else 節を欠いていた。Rust parser はこれを拒否し (末尾に余った `)` 6 個としてエラーが出る)、
  selfhost CLI は `diagnostics:0` で受理していた。fixture 側は `0` を補って修復済み
  (`I-82` の #7 の是正に含む)。**修復したので、この乖離を検出する test は現存しない。**
- **本 issue では直さない。** selfhost parser の arity 検査追加は `I-82` の probe 実質化 slice の
  範囲外である。`I-73` (native differential の pin) と同じ帯の、Rust/selfhost 差分の問題として扱う。
- **関連**: `I-82` (発見経路)、`I-73` (Rust/selfhost 差分の pin)。
  引き取り先は `TODO.md` の `SELFHOST-PARSE-LENIENT-01`。

<a id="i-87"></a>

### I-87: WASI 経路の `read-file` が preopen 外のパスに対しエラーではなく空文字列を返す

- **影響度**: 中 / **状態**: open
- **発見**: 2026-08-27 (`I-85` の `test_debug_boot04_stage2_first_defn_probe_on_minimal_make_type_constrained_shape`
  を実質化する際、stage1 側の probe が `301,-1` = 「defn が 1 つも無い」を返すのを実測して発見)
- **内容**: stage1 を動かす WASI runner は
  `crates/lsharp-wasm/src/wasi_runner/preview1.rs:109-117` で
  `builder.preopened_dir(dir_path, ".", DirPerms::all(), FilePerms::all())` を張るだけである。
  つまり guest から見える root は selfhost ルート 1 つで、それ以外のパスは**原理的に開けない**。
  ところが開けなかったとき `read-file` は**エラーを返さず空文字列を返す**。
  guest 側はそれを「中身が空のファイル」として扱い、そのまま先へ進む。
- **これが test を無言で骨抜きにする。** 実例:
  `std::env::temp_dir()` 配下へ書いた fixture を絶対パスで stage1 に渡していた test は、
  fixture が 1 バイトも読まれていないのに probe 出力自体は空でないため
  `assert!(!output.trim().is_empty())` を通過していた。probe が出していたのは
  「入力が空だったときの値」(`301,-1`) であって、test 名が主張する parity ではない。
  `I-85` / `I-82` の「主題を検査していない test」が緑であり続けた原因の一つがこれである。
- **eleven-import 版は挙動が違う。** `crates/lsharp-wasm/tests/e2e/selfhost_bootstrap_four_layer/part_002.rs:42-60`
  の host func は `root_dir.join(rel_path)` を使うため絶対パスも解決でき、失敗時は panic する。
  **同じ `read-file` という名前で、経路によって「絶対パスが通るか」も「失敗が見えるか」も違う。**
- **直し方の方向**: 失敗を空文字列に潰さない。`read-file` に成否を返させるか、
  少なくとも guest 側で「長さ 0」と「読めなかった」を区別できるようにする。
  test 側の回避 (fixture を preopen 配下へ置く) は `I-85` の是正で既に入れたが、
  **それは回避であって是正ではない**。
- **関連**: `I-85` (発見経路。fixture 配置で回避済み)、`I-82` (同じ「緑だが何も見ていない」帯)。
  引き取り先は `TODO.md` の `SELFHOST-READFILE-SILENT-01`。

<a id="i-88"></a>
### I-88: target-defn probe の body ナビゲーションが旧 shape 前提のままで、下流 marker の assertion が「壊れていること」を pin している

- **影響度**: 低 / **状態**: deferred (`I-80` の却下案 B を実行するときに解消する)
- **発見**: 2026-08-27 (`I-80` の test 側是正の副産物)
- **内容**: `selfhost/src/App/CompilerMode.ls` の `compile-file-mode-target-defn-parity-probe` は
  `make-type-constrained` の body を **添字直打ち**で辿る:

  ```
  body        = (vector-get decl (+ 3 (vector-get decl 2)))
  outer-expr  = (vector-get body 3)
  inner-call  = (vector-get (vector-get outer-expr 3) 4)
  inner-func  = (vector-get inner-call 1)
  ```

  この経路は body が `let` + 二重 `vector-push` である前提で書かれている。
  現在の body は `vector-push-pair-rooted` 単一呼び出し (`ast-apply`) なので、
  `outer-expr` 以下は **AST の外を読んでいる**。`I-80` はこれを診断済みで、
  probe 本体を作り替える案 (却下案 B) は「selfhost 側の変更で stage0 再生成と native lane に
  波及するため、test 2 件の赤を直す規模ではない」として**意図的に却下**した。
- **本 issue が記録するのは、その却下の代償である。** `I-80` の是正で
  `part_009.rs` の stage1 側 shape pin は、壊れた状態を明示的に pin する形になった:

  | marker | 本来の assertion | 現在の assertion | 理由 |
  |---|---|---|---|
  | 129 | `== 131` (use-site と def-site の hash 一致) | `== 0` | `inner-func` がゴミなので hash が 0 |
  | 130 | `> 0` (ftable lookup が空でない) | `== 0` | hash 0 は ftable に無い |
  | 133 | `> 0` (chunked ftable lookup が空でない) | `== 0` | 同上 |
  | 135 | (未評価) | `== 0` | 同上 |
  | 127 / 128 | -- | **pin しない** | 範囲外読み出しで binary 依存。stage1 は `4294967296` / `72057594054705152`、stage2 は `0` / `0` |

  **これは緑にするための書き換えではない。** 現在の assertion は probe 本体が直された瞬間に
  赤くなる向きに置いてあり、コメントで「赤くなったらそれは正しい挙動である」と明示している。
  ただし**「本来何を見るべきだったか」はコードからは復元できない**ので、ここに残す。
- **解消の条件**: 案 B (probe を shape 非依存の構造走査へ作り替える) を実行したら、
  上表の「本来の assertion」へ戻す。`(vector-get decls 31)` の hardcode も同時に扱う。
  案 B の再検討トリガは `I-80` の ADR が定めている
  (「`(vector-get decls 31)` の hardcode が実際に問題を起こした時点」)。
- **関連**: `I-80` (親。test 側は是正済み)、`I-82` / `I-85` (主題を検査していない test の帯)、
  `I-84` (恒常赤にしないための判断根拠)。
  引き取り先は `TODO.md` の `TARGET-DEFN-NAV-STALE-01`。

<a id="i-89"></a>
### I-89: x86 の 20 引数以上 param spill テーブル約 680 行が到達不能で、旧 slot 規約のまま残っている

- **影響度**: 低 / **状態**: open
- **発見**: 2026-08-27 (`I-73` の受入条件 (a) の git 考古学の副産物)
- **内容**: `selfhost/src/Backend/Native/NativeCodegen.ls` の x86 param spill には
  経路が 2 つあるが、**片方に caller が無い**。

  | 経路 | 入口 | slot 規約 | 到達性 |
  |---|---|---|---|
  | 汎用 loop | `spill-native-function-params-x86-loop` (`:13971`) -> `spill-native-function-param-x86` (`:13949`) | `native-param-slot-offset-x86` = **1 origin** | **生きている** (`:14086` から全 param 数がここを通る) |
  | hand-unrolled テーブル | `spill-native-function-params-x86-twenty-to-sixty-one` (`:13988`) | `local-slot-offset` = **0 origin** | **到達不能** |

  後者の chain は `...-twenty-to-twenty-two` (`:13269`) から `...-twenty-to-sixty-one` (`:13988`)
  まで約 680 行あるが、chain の根に caller が 1 つも無い
  (`grep -rn spill-native-function-params-x86-twenty-to-sixty-one selfhost/src` が定義行だけを返す)。
- **死んだテーブルは `361d0d99` 以前の 0 origin 規約のままである。**
  `361d0d99` (2026-05-17) が「selfhost IR は slot 0 を scratch として予約する」として
  x86 の param slot を 1 origin へ移したが、この置換は汎用 loop 側にしか適用されなかった。
  さらに `spill-native-function-params-x86-twenty-plus` (`:13978`) は
  **同一関数内で不整合**である — register 側 (param 0-5) が 0 origin、
  stack 側 (`spill-native-function-stack-params-x86-loop`) が 1 origin。
- **aarch64 側の同型 chain は生きている** (`:20056` から呼ばれる)。
  死んでいるのは x86 側だけで、汎用 loop へ書き直した際の消し忘れである。
- **現状では動作上のバグではない。** 到達不能なので実行されない。
  記録する理由は、**旧 ABI を体現した 680 行が「正しそうな顔で」残っていること**にある。
  将来ここへ分岐を戻すと params が 1 slot ずれる。
- **削除の前に確認が要る**: `spill-native-function-params-aarch64-*` と対称に見えるので、
  x86 側だけを消すと「片方だけ無い」状態になる。**汎用 loop が 20 引数以上でも
  aarch64 側と同じバイト列を出すことを先に確かめること。**
  確認せずに消すのは、動く可能性のある実装を捨てることになる。
- **関連**: `I-73` (発見経路。ずれの原因 commit が同じ `361d0d99`)。
  引き取り先は `TODO.md` の `NATIVE-X86-SPILL-DEAD-01`。

<a id="i-90"></a>
### I-90: selfhost LSP の framed response が 0 origin Position を返すのに、test 2 件の期待値が 1 origin になっている

- **影響度**: 中 / **状態**: open
- **発見**: 2026-08-27 (`I-75` の 14 件を分類する作業。`I-64` sweep のログの読み直しだけで確定した)
- **内容**: `selfhost_cli_core` の赤 2 件が、**LSP `Position` の origin ずれ**という同一原因で落ちている。

  | test | 実装が返した range | test の期待 |
  |---|---|---|
  | `..._lsp_transport_hover_frame` | `{line:1,character:15}` - `{line:1,character:21}` | `{line:2,character:16}` - `{line:2,character:22}` |
  | `..._lsp_transport_formatting_frame` | `{line:0,character:0}` - `{line:1,character:3}` | `{line:1,character:1}` - `{line:2,character:4}` |

  **line も character も、4 つの数値すべてがちょうど +1 ずれている。** ずれ方に例外が無い。
- **どちらが正しいかは LSP 仕様で決まる。** LSP の `Position` は `line` / `character` とも
  **zero-based** である。hover の fixture は
  `"(defn square [x] x)\n(defn main [] (square 1) (square 2))"` で、2 行目の `square` は
  0 origin で 15..21 に載る。**実装が返した `{line:1,character:15}`-`{line:1,character:21}` が
  仕様どおりで、test の期待が 1 origin である。**
- **したがって本件は「実装のバグ」ではなく「test の期待が仕様と違う」側の疑いが濃い。**
  ただし *断定はしない*。理由は次のとおり:
  - hover の request 側は `(99, 2, 17, source)` を渡しており、`line=2` は 2 行しかない文書では
    0 origin だと範囲外である。**request 側は 1 origin で書かれている可能性がある。**
    もし実装が request を 1 origin で受けて response を 0 origin で返しているなら、
    **実装自身が入口と出口で origin を混ぜている**ことになり、そちらが本体の欠陥になる
  - `character=17` は `square` の内側なので 0 origin でも 1 origin でも成立し、
    **この fixture だけでは request 側の origin を判別できない**
- **判別の方法**: 文書の 1 行目 (`(defn square [x] x)`) の `square` を狙う request を足す。
  0 origin なら `line=0`、1 origin なら `line=1` で当たる。**どちらか一方しか当たらない**ので
  1 回の測定で決まる。`selfhost/src/App/Cli.ls` の `run-lsp-transport-request` が正本。
- **`I-75` から移管した 2 件である。** 移管前の注記は「原因未診断」だった。
- **関連**: `I-75` (発見経路)、`I-64` (sweep の元)。
  引き取り先は `TODO.md` の `LSP-POSITION-ORIGIN-01`。
