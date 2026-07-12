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
>
> **改善方針**: [docs/development/planning/improvement-roadmap.md](docs/development/planning/improvement-roadmap.md)
> **新設計**: [docs/development/planning/improvement-designs/](docs/development/planning/improvement-designs/README.md)

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
| [D-07](#d-07) | 相互再帰モジュールが一括型推論前提 | 中 | in-design | [imp-04](docs/development/planning/improvement-designs/imp-04-module-system-strengthening.md) |
| [D-08](#d-08) | Native backend research scope (self-regeneration / 差分ゼロ) | 中-高 | deferred | V2-08 / V2-09 |
| [D-09](#d-09) | セルフホスト ADT が整数タグ + Vector 表現 | 中 | in-design | imp-01 |
| [D-10](#d-10) | GC sentinel 判別の理論的 edge case (G1) | 低-中 | documented-limitation | [imp-03](docs/development/planning/improvement-designs/imp-03-dynamic-memory-layout.md) |

### 実装 (I)

| ID | 問題 | 影響度 | 状態 | 設計 doc |
|----|------|--------|------|----------|
| [I-01](#i-01) | ファイルサイズ規約 (500-800 行) 超過 10+ ファイル | 高 | in-design | [imp-06](docs/development/planning/improvement-designs/imp-06-large-file-decomposition.md) |
| [I-02](#i-02) | エラーハンドリング不統一 (miette が driver 限定) | 高 | in-design | [imp-02](docs/development/planning/improvement-designs/imp-02-error-handling-unification.md) |
| [I-03](#i-03) | GC メモリレイアウトの固定スロット上限 | 高 | in-design | imp-03 |
| [I-04](#i-04) | GC フリーリストが線形探索 | 中 | in-design | imp-03 |
| [I-05](#i-05) | CLI コンパイル経路が解析キャッシュ未使用・SCC 検出なし | 中 | in-design | imp-04 |
| [I-06](#i-06) | Fuzz / メモリリーク / 性能限界テストの欠落 | 中 | in-design | [imp-07](docs/development/planning/improvement-designs/imp-07-test-verification-infrastructure.md) |
| [I-07](#i-07) | selfhost parser の rooting 修正が頻発 | 中 | in-design | imp-07 |
| [I-08](#i-08) | テストカバレッジの偏り (lsharp-wasm に集中) | 中 | in-design | imp-07 |

### ドキュメント (DOC)

| ID | 問題 | 影響度 | 状態 | 設計 doc |
|----|------|--------|------|----------|
| [DOC-01](#doc-01) | ユーザーガイドの主要範囲不足 | 高 | resolved | [imp-05](docs/development/planning/improvement-designs/imp-05-docs-restructure.md) |
| [DOC-02](#doc-02) | book/ がユーザー向けと実装者向けの混在 | 中 | resolved | imp-05 |
| [DOC-03](#doc-03) | ドキュメント鮮度追跡 (.lsharp-doc-status) が未運用 | 中 | resolved | imp-05 |
| [DOC-04](#doc-04) | examples/ とドキュメントの連携不足 | 低-中 | resolved | imp-05 |
| [DOC-05](#doc-05) | language-guide テンプレートと docs/ の二重管理リスク | 低 | resolved | imp-05 |
| [DOC-06](#doc-06) | エラーコード体系が docs 未定義 (MCP に E0001-E0005 のみ) | 中 | resolved | imp-02 |

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
### D-07: 相互再帰モジュールが一括型推論前提でインクリメンタル化を阻む

- **影響度**: 中 / **状態**: in-design
- **内容**: `Tools.Text.FormatterExpr` / `FormatterDecl` / `Formatter` は相互再帰のため、
  `compile_multi_file` が 3 モジュールをまとめて 1 回で型推論する必要がある
  (個別モジュール順だと `format-expr` が未束縛になる)。モジュール単位の独立推論が
  できず、インクリメンタルコンパイルの設計が成立しない。
- **根拠**:
  - `docs/development/planning/completion-criteria.md:18` -- 「`lsharp_ir::compile_multi_file` が当該 3 モジュールをまとめて 1 回型推論する（個別モジュール順だと `format-expr` が未束縛になる）」
- **関連**: I-05 / V2-01 (LSP incremental sync)。改善設計は [imp-04](docs/development/planning/improvement-designs/imp-04-module-system-strengthening.md)。

<a id="d-08"></a>
### D-08: Native backend research scope (self-regeneration / 差分ゼロは V2 へ deferred)

- **影響度**: 中-高 / **状態**: deferred (公式状態を尊重)
- **内容**: 2026-03-30 の方針転換で Wasmtime embedding + Component Model が正式配布モデルとなり、
  native self-regeneration (旧条件 1-2) は V2-08、Wasm/native 差分ゼロ (旧条件 3) は V2-09 へ移動した。
  native-only official replacement の V2-13/V2-15 は完了しており、Linux x86_64 actual stage1 → stage2 → stage3 と stable release smoke は release blocker ではない。
- **根拠**:
  - `docs/development/planning/completion-criteria.md:9` -- 方針転換と V2-08/V2-09 への移動
  - `docs/development/planning/completion-criteria.md:59-65` -- 旧条件 1-3 の deferred 整理
- **関連**: V2-08 / V2-09 (TODO.md がタスク正本)。本台帳は deferred research scope のみを記録する。

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
  多数あり、エージェント解析精度・レビュー容易性・責務分離を損なっている。
  主要超過ファイル (src のみ、2026-06-12 実測):

  | ファイル | 行数 | 規約比 |
  |---------|------|--------|
  | `crates/lsharp-wasm/src/wasi.rs` | 4175 | 5.2x |
  | `crates/lsharp-driver/src/main.rs` | 3928 | 4.9x |
  | `crates/lsharp-types/src/infer.rs` | 3783 | 4.7x |
  | `crates/lsharp-ir/src/lib.rs` | 3640 | 4.6x |
  | `crates/lsharp-syntax/src/parser.rs` | 2242 | 2.8x |
  | `crates/lsharp-types/src/constraints.rs` | 1961 | 2.5x |
  | `crates/lsharp-ir/src/lower/expr.rs` | 1897 | 2.4x |
  | `crates/lsharp-syntax/src/macro_expand.rs` | 1681 | 2.1x |
  | `crates/lsharp-ir/src/module_graph.rs` | 1597 | 2.0x |
  | `crates/lsharp-lsp/src/lib.rs` | 1397 | 1.7x |
  | `crates/lsharp-wasm/src/host_bridge.rs` | 1032 | 1.3x |
  | `crates/lsharp-wasm/src/wasi_runner.rs` | 941 | 1.2x |

  テストでは `crates/lsharp-ir/src/lower/tests.rs` (3400 行) も肥大。
- **根拠**: `wc -l` 実測 (上表)。規約は `CLAUDE.md` ファイルサイズ制限の節。
- **関連**: selfhost 側は ADR-168 (STR-01〜03) で分割実績あり (TypeInfer.ls 1093 → 290 行など)。
  Rust 側の分割設計は [imp-06](docs/development/planning/improvement-designs/imp-06-large-file-decomposition.md)。

<a id="i-02"></a>
### I-02: エラーハンドリング戦略の不統一

- **影響度**: 高 / **状態**: in-design
- **内容**: miette によるリッチ診断は lsharp-driver の最上層のみで、下層クレート
  (lsharp-syntax / lsharp-types / lsharp-ir / lsharp-wasm) の src には miette 利用が存在しない
  (thiserror ベースのエラー型のみ)。エラー型間で span 保持も不揃いで、`LowerError` と
  `CodegenError` は span 情報を一切持たない。本番経路にファイル I/O 失敗で panic する箇所があり、
  LSP のエラー診断はソース位置を持たず固定 `Range(0,0)` で報告される。
  エラーコード体系も整備されていない (DOC-06 と関連)。
- **根拠**:
  - `crates/lsharp-ir/src/lib.rs:3609`, `:3611` -- `unwrap_or_else(|err| panic!("{} を読めませんでした: {err}", ...))` によるファイル I/O panic
  - 下層 4 クレートの `src/` に対する `grep -rn "miette"` がヒットなし (2026-06-12 確認)
  - `crates/lsharp-ir/src/lower/mod.rs:19-25` (LowerError)、`crates/lsharp-wasm/src/codegen.rs:11-14` (CodegenError) -- span フィールドなし
  - `crates/lsharp-lsp/src/util.rs:356-364` -- `diagnostic_error` が固定 `Range::new(Position::new(0,0), Position::new(0,0))` を設定、`Diagnostic.code` は未設定
- **関連**: DOC-06 (エラーコード)。改善設計は [imp-02](docs/development/planning/improvement-designs/imp-02-error-handling-unification.md)。

<a id="i-03"></a>
### I-03: GC メモリレイアウトの固定スロット上限

- **影響度**: 高 / **状態**: in-design
- **内容**: GC ランタイムのリニアメモリレイアウトが定数でハードコードされており、
  GC オブジェクトテーブル 4096 スロット / フリーリスト 4096 スロット / root stack 32768 スロットを
  超えるワークロードで容量が枯渇する。大規模プログラムや長寿命プロセスのスケール限界が
  コンパイル時に固定されている。
- **根拠**:
  - `crates/lsharp-wasm/src/wasi.rs:21` -- `const ROOT_STACK_SLOT_CAPACITY: i32 = 32768;`
  - `crates/lsharp-wasm/src/wasi.rs:23` -- `const GC_OBJECT_SLOT_CAPACITY: i32 = 4096;`
  - `crates/lsharp-wasm/src/wasi.rs:26` -- `const GC_FREE_LIST_SLOT_CAPACITY: i32 = 4096;`
- **関連**: memory-management-roadmap.md (GC 実装の正本)。改善設計は [imp-03](docs/development/planning/improvement-designs/imp-03-dynamic-memory-layout.md)。

<a id="i-04"></a>
### I-04: GC フリーリストが線形探索

- **影響度**: 中 / **状態**: in-design
- **内容**: フリーリスト管理が線形走査 (worst case O(n)) で、割り当て頻度の高い
  ワークロードでアロケーションコストが増大する。サイズクラス別リスト等の高速化が未実装。
- **根拠**: `crates/lsharp-wasm/src/wasi.rs` のフリーリスト実装 (GC_FREE_LIST 関連、`:26` 周辺の定数に基づく単一リスト構成)。
- **関連**: I-03 と同じレイアウトに依存。imp-03 参照。

<a id="i-05"></a>
### I-05: CLI コンパイル経路が解析キャッシュを使わず、モジュールグラフに SCC 検出がない

- **影響度**: 中 / **状態**: in-design
- **内容**: インクリメンタル解析キャッシュ自体は実装済みで
  (`CompilationCache` + `SourceFingerprint`、LSP は `analyze_single_file_incremental` 経由で利用)、
  問題は次の 2 点に絞られる:
  1. CLI の `compile_multi_file` (`crates/lsharp-ir/src/lib.rs:1777`) はキャッシュを受け取らず、
     実行ごとに `ModuleGraph::build_from_entry` でグラフ構築と全モジュール再解析を行う
  2. `ModuleGraph` は DFS トポロジカルソート (`module_graph.rs:221-243`) と循環検出
     (`module_graph.rs:168-216`) のみで SCC 検出がなく、相互再帰モジュール (D-07) は
     一括 merged 推論への特別扱いで処理される
- **根拠**:
  - `crates/lsharp-ir/src/cache.rs:215-256` -- `CompilationCache` (実装済み)
  - `crates/lsharp-ir/src/lib.rs:1792-1820` -- `analyze_single_file_incremental` (fingerprint 一致で再解析スキップ)
  - `crates/lsharp-lsp/src/lib.rs:37-38` -- LSP がキャッシュを保持
  - `crates/lsharp-ir/src/lib.rs:1777` -- `compile_multi_file(entry_file: &Path) -> Result<Module, String>` はキャッシュ非対応
- **注記**: 本台帳の初版 (2026-06-12) は「キャッシュ機構なし」と記載していたが、再調査で訂正した。
- **関連**: D-07 / V2-01 (LSP incremental sync)。imp-04 参照。

<a id="i-06"></a>
### I-06: Fuzz テスト・メモリリークテスト・性能限界テストの欠落

- **影響度**: 中 / **状態**: open
- **内容**: E2E / スナップショット / GC メトリクス検証は充実している一方、
  (1) パーサー・型推論へのファズ入力、(2) 長時間運転でのメモリリーク検出、
  (3) 固定スロット上限 (I-03) や再帰深度などのスケール限界を計測するテストが存在しない。
  unification の occur check 性能 (深いネスト型・巨大レコードでの計算量) も未測定。
- **根拠**: リポジトリ内に cargo-fuzz / proptest / quickcheck の依存・ターゲット定義なし
  (2026-06-12 確認)。ベンチは criterion (`crates/lsharp-wasm/benches/compiler_pipeline.rs`) のみ。
- **関連**: I-03 (限界値が未知のまま固定されている)。改善設計は [imp-07](docs/development/planning/improvement-designs/imp-07-test-verification-infrastructure.md)。

<a id="i-07"></a>
### I-07: selfhost parser / x86 backend の GC rooting 修正が頻発

- **影響度**: 中 / **状態**: open
- **内容**: 直近の履歴で selfhost parser の defn body の GC rooting (生存時間管理) を巡る修正が
  反復しており (`dbdd448`, `93074c8`, `9c40998` など)、x86 ネイティブ系では Trace 目的の
  診断コミット (`e5d60de`, `f330419`, `559c630`, `2fb6d79` など) が main 履歴に直接混入している。
  「GC 中に値が回収されないよう手動で root する」規律がコード規約として明文化されておらず、
  同型のバグが繰り返し発生する構造になっている。
- **根拠**: `git log --oneline -20` (2026-06-12) -- 上記コミット群。
  selfhost 側の rooting イディオムは `root_push` / `root_pop` / `root_set`
  (生成コード側ランタイム関数、`crates/lsharp-wasm/src/wasi.rs:154-156` 周辺)。
- **関連**: D-08 (native backend research scope)。改善設計は [imp-07](docs/development/planning/improvement-designs/imp-07-test-verification-infrastructure.md) (rooting 規約の明文化と guard test 拡張)。

<a id="i-08"></a>
### I-08: テストカバレッジの偏り

- **影響度**: 中 / **状態**: open
- **内容**: テストコードが lsharp-wasm の E2E (selfhost 系 tests/ 配下、数万行規模) に集中し、
  lsharp-syntax / lsharp-types / lsharp-driver はインラインテスト主体で相対的に薄い。
  E2E の失敗からレイヤ単体の原因へ切り分けるコストが高い。
- **根拠**: `wc -l` 実測 -- `crates/lsharp-wasm/tests/e2e/selfhost_native_stage_chain.rs` 48941 行、
  `selfhost_native_differential.rs` 12409 行、`selfhost_bootstrap_four_layer.rs` 11750 行 ほか。
- **関連**: I-01 (テストのインライン配置がファイル肥大の一因)。
  改善設計は [imp-07](docs/development/planning/improvement-designs/imp-07-test-verification-infrastructure.md) (増強方針) と imp-06 (分割方針)。

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
  - `TODO.md` -- `DOC-04` 完了メモと focused test evidence
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

---

## 更新規則

- 新しい問題は該当カテゴリの次番号で追記する (欠番は再利用しない)
- 問題が解消されたら削除せず `状態: resolved` に変更し、解消根拠 (コミット / テスト) を追記する
- 着手タスク化する場合は TODO.md (正本) に項目を作り、本台帳からは ID 参照のみ行う
- file:line の根拠は記載時点の実測とし、大きくずれた場合は検証日とともに更新する
