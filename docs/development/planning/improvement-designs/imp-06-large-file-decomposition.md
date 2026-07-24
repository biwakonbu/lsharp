# imp-06: 大規模ファイル分割 (Rust 側)

> 対象 issue: [I-01](../../../../ISSUES.md#i-01) (ファイルサイズ規約超過)、[I-08](../../../../ISSUES.md#i-08) (テスト配置の偏り、一部)
> ロードマップ: [improvement-roadmap.md](../improvement-roadmap.md) Phase A-2 / D-4
>
> **先行事例**: selfhost 側は ADR-168 (STR-01〜03) で同種の分割を完了している
> (TypeInfer.ls 1093 → 290 行 + Apply/Block/Pattern/Record 切り出し、LspServer.ls 1303 → 80 行、
> Formatter.ls 930 → 84 行、全ファイル 800 行未満)。Rust 側も同じ「責務軸での切り出し +
> 親は再エクスポートと統合のみ」方式を踏襲する。

## 概要

プロジェクト規約 (1 ファイル 500-800 行、CLAUDE.md) を大幅に超えるソースが多数ある
(2026-06-12 実測、詳細は [ISSUES.md I-01](../../../../ISSUES.md#i-01) の表を参照)。
最大の `crates/lsharp-wasm/src/wasi.rs` は 4175 行で規約の 5.2 倍。
インラインテスト (`#[cfg(test)]`) の肥大が行数の 15-20% を占めるファイルもあり、
テスト分離だけでも効果がある。

## 設計

### 1. 分割の一般規則

- 親ファイルは「モジュール宣言 + 再エクスポート + 横断ロジックの統合」のみ残し、800 行未満にする
- 公開 API (pub シグネチャ) は不変。`pub use` で従来パスを維持し、利用側の変更をゼロにする
- インラインテストは `src/<module>/tests.rs` (または `tests/` 配下) へ移し、本体と分離する
- 1 ファイルの分割 = 1 PR 相当の独立変更とし、各分割後に `cargo test` / `cargo clippy` 全件 green を確認
- 分割は「移動のみ」とし、ロジック変更 (リファクタリング) を同一コミットに混ぜない

### 2. 主要ファイルの分割軸

セクション境界は 2026-06-12 に実測済み。分割線はこの境界に沿って引く:

| 対象 | 行数 | 実測セクション境界 | 分割案 |
|------|------|--------------------|--------|
| `crates/lsharp-wasm/src/wasi.rs` | 4175 | :1-100 定数・構造体 / :102 `emit_wasm_wasi()` / :742 `emit_wasm_wasi_p2()` | `wasi/layout.rs` (メモリレイアウト定数・ヘッダ)、`wasi/gc_runtime.rs` (root_push/pop/set・mark・sweep・free list の emit)、`wasi/io.rs` (fd_write 系 WASI 連携)、`wasi/emit_p1.rs` / `wasi/emit_p2.rs` (preview1 / component 入口)、`wasi/tests.rs` |
| `crates/lsharp-driver/src/main.rs` | 3928 | :1-237 CLI 定義 / :238 `main()` / :246-461 Command マッチ | `cli.rs` (clap 定義)、`commands/<command>.rs` (compile / test / review / doc / package 系をコマンド単位)、`main.rs` はディスパッチのみ |
| `crates/lsharp-types/src/infer.rs` | 3783 | :21-99 TypeError / :177-245 Infer struct / :308-435 `infer_program` / :2500-2621 instantiate・generalize・unify / :2652 tests 開始 (約 1130 行 = 30%) | `infer/error.rs` (TypeError)、`infer/unify.rs` (unify + occur check)、`infer/generalize.rs`、`infer/expr.rs` (式推論)、`infer/decl.rs` (`infer_program` と宣言処理)、`infer/tests.rs` |
| `crates/lsharp-ir/src/lib.rs` | 3640 | :1-441 構造体定義 / :442 `link_modules()` / :1647 `compile_multi_file_with_mode` / :1777 `compile_multi_file` / :1792 incremental 系 / :2211 tests 開始 (約 1430 行 = 39%) | `ir.rs` (Module / Function / Instruction / IrType 定義)、`linker.rs` (`link_modules`)、`compile.rs` (compile_multi_file 系 + incremental 系、imp-04 と接続)、`tests.rs` |
| `crates/lsharp-syntax/src/parser.rs` | 2242 | `parser/expr.rs` / `parser/decl.rs` / `parser/tests.rs` |
| `crates/lsharp-syntax/src/hygiene.rs` | 546 | :1-294 Sets of Scopes production API / :295-546 inline tests | inline tests を `hygiene/tests.rs` へ移し、production API と回帰 fixture の ownership を分離 |
| `crates/lsharp-types/src/constraints.rs` | 1961 | `constraints/def.rs` (定義・登録)、`constraints/eval.rs` (評価)、`constraints/pattern.rs` (簡易正規表現)、`constraints/tests.rs`。まず inline tests を責務別 test files へ分離し、production 分割は後続 |
| `crates/lsharp-types/src/regex/dfa.rs` | 699 | :1-580 NFA/DFA production + cache / :581-699 inline tests | inline tests を `regex/dfa_tests.rs` へ移し、DFA backend の test-only fixture を分離 |
| `crates/lsharp-ir/src/lower/expr.rs` | 1897 | パターンマッチ lowering / 計算式 lowering を切り出し |
| `crates/lsharp-ir/src/lower/decl.rs` | 823 | :1-690 宣言 lowering / :691 Self-TCO helper | Self-TCO helper を `lower/decl/self_tco.rs` へ移し、親は 800 行未満へする。残る宣言 lowering の責務分割は後続 |
| `crates/lsharp-driver/src/doc_site.rs` | 840 | :1-573 production doc-site generation / :574-840 inline tests | inline tests を `doc_site/tests.rs` へ移し、production parent を 800 行未満へする。サイト生成責務の分割は後続 |
| `crates/lsharp-syntax/src/macro_expand.rs` | 1681 | 展開器本体 / 組み込みマクロ / tests |
| `crates/lsharp-ir/src/module_graph.rs` | 1597 | グラフ構築 / 解決 / (imp-04 の SCC・キャッシュ導入前に分割) |
| `crates/lsharp-lsp/src/lib.rs` | 1397 | ハンドラ単位 (hover / completion / definition...) は既に別ファイルがあるため、残る統合部から診断変換等を切り出し |

`crates/lsharp-ir/src/lower/tests.rs` (3913 行) は production 変更と分離した test-only slice として、
helper/定数だけを親へ残し、WasmGC/root、core、allocating call、self-TCO、language/trait、record/ADT、
module/lambda、closure call、heap/ADT の9 moduleへ分割する (I-08 の切り分け性改善)。

`crates/lsharp-ir/src/lower/decl.rs` (823 行) は Self-TCO helper を `decl/self_tco.rs` (139 行) へ分離し、親を 692 行へ縮小した。

`crates/lsharp-driver/src/doc_site.rs` (840 行) は inline test 8 件を `doc_site/tests.rs` (266 行) へ移動し、production parent を 575 行へ縮小した。`doc_site::tests` focused 8 件、driver unit 132 件、clippy、rustfmt、`git diff --check` が pass した。full driver integration lane の `default_path_delegation` 12 件は embedded component / selfhost artifact の今回の差分外 failure boundary として残る。

`crates/lsharp-syntax/src/hygiene.rs` (546 行) は inline test 17 件を `hygiene/tests.rs` (249 行) へ移動し、production parent を 297 行へ縮小した。`hygiene::tests` focused 17 件、`lsharp-syntax` package 175 件、clippy、rustfmt、`git diff --check` が pass した。

`crates/lsharp-types/src/regex/dfa.rs` (699 行) は inline test 13 件を `regex/dfa_tests.rs` (107 行) へ移動し、production parent を 594 行へ縮小した。`regex::dfa::tests` focused 13 件、`lsharp-types` package 258 件、clippy、rustfmt、`git diff --check` が pass した。

### 3. 優先順位

1. **imp-02 (エラー統一) の対象になるファイルを先に分割しない** — A-1 のエラー型変更を
   先に済ませ、診断が安定した後に分割する (コンフリクト最小化)。例外として
   テスト分離 (`#[cfg(test)]` の別ファイル化) はいつでも安全に実施できるため先行可
2. wasi.rs (imp-03 の改修対象) と module_graph.rs (imp-04 の改修対象) は、
   それぞれの機能改修**前**に分割しておく — 改修の diff が読める粒度になる
3. main.rs はユーザー影響がなく独立性が高いため、並行作業の隙間で進める

### 4. 機械検査

CI (または契約テスト) に「`crates/**/src/**/*.rs` の行数が 800 を超えるファイル一覧」を
出す検査を追加し、現状の超過リストを許容リストとして固定 → 分割の進捗に応じて
許容リストを縮め、最終的に空にする (リグレッション防止)。

## 影響範囲

- 公開 API 不変・ロジック移動のみのため、機能リスクは低い
- git blame の追跡性が下がるため、分割コミットは「移動のみ」と明記する
- 進行中ブランチとのコンフリクトが最大のコスト。各ファイルの分割タイミングは
  当該ファイルを触る作業の合間を選ぶ

## 検証済み部分実装 (2026-07-25)

`module_graph.rs` の inline unit test 43 件を `src/module_graph/` 配下の 4 test module へ移動し、続けて `ModuleSearchPaths`、path resolver、entry graph builder を `src/module_graph/resolve.rs` へ分離した。さらに Rust parser の inline test 61 件を `src/parser/` 配下の 3 test module、constraints の inline test 43 件を `src/constraints/` 配下の 4 test module、macro expand の inline test 35 件を `src/macro_expand/` 配下の 3 test module、regex の inline test 25 件を `src/regex/tests.rs`、lexer の inline test 37 件を `src/lexer/` 配下の 3 test module、metadata_check の inline test 31 件を `src/metadata_check/` 配下の 2 test moduleへ移動し、macro expand production を `error.rs` / `builtins.rs` / `expand.rs`、constraints production を `eval.rs` / `hierarchy.rs` / `conversion.rs` / `runtime.rs`、regex production を `node.rs` / `parser.rs` / `matcher.rs` へ分離した。さらに `lsharp-ir/src/lower/tests.rs` の inline test 143 件を helper/定数だけの親 133 行と、WasmGC/root・core・allocating call・self-TCO・language/trait・record/ADT・module/lambda・closure call・heap/ADT の9 test module（129/692/414/531/531/228/598/290/390 行）へ移動した。加えて `lsharp-syntax/src/hygiene.rs` の inline test 17 件を production parent 297 行と `hygiene/tests.rs` 249 行、`lsharp-types/src/regex/dfa.rs` の inline test 13 件を production parent 594 行と `regex/dfa_tests.rs` 107 行へ分離した。公開 API・production semantics・test body は変更していない。focused lower 143 tests、Self-TCO 8 tests、hygiene 17 tests、DFA 13 tests、large-stack `lsharp-ir` 257 tests、`lsharp-syntax` package 175 tests、`lsharp-types` package 258 tests、該当 crate の clippy/rustfmt、`git diff --check` が passし、default stack の Formatter incremental fixture overflow は imp-04 C-1n の既知境界として分離した。詳細は各 `decisions-legacy-*split.md` ADR に記録する。

## ステータス

設計 + module graph のテスト/path-resolution 分離、parser・constraints・macro expand・regex・lexer・metadata_check・hygiene・lower のテスト分離、lower/decl Self-TCO production split、doc_site test split、regex DFA test split、macro expand・constraints・regex production split、lexer/metadata_check test split verified partial slice (2026-07-25)。lower tests は parent 133 行 + 9 module（最大 692 行）、lower/decl は parent 692 行 + `decl/self_tco.rs` 139 行、doc_site は parent 575 行 + `doc_site/tests.rs` 266 行、hygiene は parent 297 行 + `hygiene/tests.rs` 249 行、regex DFA は parent 594 行 + `regex/dfa_tests.rs` 107 行となった。lower expr/mod の production 分割は未着手である。parser の expr/decl production 分割、lexer/metadata production の責務分割、doc_site の production 責務分割、regex parser/matcher の追加分割、graph/SCC core の追加分割、`wasi.rs` / `main.rs` / `infer.rs` / `ir/lib.rs` の責務分割、I-01 / I-08 aggregate 完了は未着手または未完了である。着手時は TODO.md に Phase A-2 / D-4 としてファイル単位の項目を作成する。
