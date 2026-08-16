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
| [I-10](#i-10) | `cargo test --workspace` の pre-existing 97 FAIL が台帳未記載 | 高 | open | -- |
| [I-11](#i-11) | ビルド再現性の綻び (`Cargo.lock` 非追跡 / dead test file) | 低-中 | open | -- |

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
| [DOC-08](#doc-08) | 陳腐化した記述と重複節 (legacy-rust-bootstrap README / TODO の v0.3 節) | 低-中 | open | -- |

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

### I-10: `cargo test --workspace` の pre-existing 97 FAIL が台帳未記載

- **影響度**: 高 / **状態**: open
- **内容**: workspace 全体のテストは常時 97 件 FAIL する。この状態が台帳にも TODO にも記録されて
  いなかったため、「workspace GREEN」を受入条件に置いた作業がそのままでは受入判定できない。
  個々の失敗はおそらく既知の未完項目 (`LEGACY-ROOT-01` / `LEGACY-BOOT-01` 等) の帰結だが、
  test 名から未完項目への対応付けが誰も持っていないのが問題である。
- **根拠**: 2026-08-16 実測。pristine な `a3ae4551` と Track 0 適用後の worktree で、test 名の集合
  `diff` が空、pass/fail 件数と snapshot の byte diff も一致する。内訳は
  `e2e` 以外 37 件 + `lsharp-wasm --test e2e` 内 60 件。
  baseline の e2e evidence 行: `test result: FAILED. 0 passed; 60 failed; 0 ignored; 0 measured;
  2961 filtered out; finished in 704.97s`。
- **既知のクラスタ (未確定、triage で確定させる)**:
  - `runtime_allocator_closures` 17 件 + lib `RootSetWithoutActiveSlot` -- `LEGACY-ROOT-01` 系と推定
  - `selfhost_native_stage_chain` 19 件 + `selfhost_native_stage23_gap` 9 件 -- ローカルに `stage0/` が
    無いことに起因 (`LEGACY-BOOT-01` 系)
  - `test_support_selfhost_typeinfer_runtime_bundle_cached` -- `tests/e2e/support.rs` の共有により
    5 binary へ重複計上される
  - `default_path_delegation` 12 件 -- embedded guest default path の selfhost 出力不一致
  - `LS0102` ペア -- `lsharp-lsp` と `lsharp-tooling` に跨る
  - insta snapshot 陳腐化 14 件 (`snapshot__wasm_*`)
- **なぜ問題か**: 全 FAIL を既知として扱うと新規の regression が埋もれる。逆に全 GREEN を要求すると
  どの slice も受入できない。クラスタごとに「どの未完項目の帰結か」を確定し、期待される FAIL の
  baseline を固定する必要がある。
- **関連**: TODO.md の triage 項目。`LEGACY-ROOT-01` / `LEGACY-BOOT-01` / I-08。
  計測記録は [`rust-boundary-reduction.md`](docs/development/operations/rust-boundary-reduction.md) の
  「Track 0 全体の workspace 検証 (2026-08-16)」節。

<a id="i-11"></a>
### I-11: ビルド再現性の綻び (`Cargo.lock` 非追跡 / dead test file)

- **影響度**: 低-中 / **状態**: open
- **内容**: 二つの独立した綻び。
  - `Cargo.lock` が `.gitignore:4` で除外されている。fresh clone / CI のたびに依存解決がやり直され、
    解決結果が日によって変わるため cold build のキャッシュヒット率が下がり、bootstrap/oracle lane の
    再現性も担保できない。application workspace として追跡対象にするのが Cargo の推奨である。
  - root の `tests/meta_validation.rs` はルート `Cargo.toml` に `[package]` が無いためコンパイル
    されていない dead file である。
- **根拠**: 2026-08-16 実測 (Track 0 調査の副産物)。
- **関連**: I-10 (どちらも「テストが本当に何を検証しているか」の可視性を下げる)。

---

## ドキュメント上の問題

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
  (a) 決定の根拠がコミットメッセージにしか残らない、(b) 台帳に無い既知問題が生まれる (I-10 が実例)、
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

- **影響度**: 低-中 / **状態**: open
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
- **根拠**: 2026-08-16 実測 (Track 0 調査の副産物)。
- **関連**: DOC-05 (正本の二重管理リスク)。

---

## 更新規則

- 新しい問題は該当カテゴリの次番号で追記する (欠番は再利用しない)
- 問題が解消されたら削除せず `状態: resolved` に変更し、解消根拠 (コミット / テスト) を追記する
- 着手タスク化する場合は TODO.md (正本) に項目を作り、本台帳からは ID 参照のみ行う
- file:line の根拠は記載時点の実測とし、大きくずれた場合は検証日とともに更新する
