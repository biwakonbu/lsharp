# imp-04: モジュールシステム強化 (SCC 推論と CLI キャッシュ統合)

> 対象 issue: [D-07](../../../../ISSUES.md#d-07) (相互再帰モジュールの一括推論)、[I-05](../../../../ISSUES.md#i-05) (CLI 経路の未キャッシュ・SCC なし)
> ロードマップ: [improvement-roadmap.md](../improvement-roadmap.md) Phase C-1 / C-2
> 関連: [v2-designs/v2-01-lsp-incremental-sync.md](../v2-designs/v2-01-lsp-incremental-sync.md) (LSP 側の受け皿)

## 現状の正確な把握 (2026-06-12 コード検証済み)

> 注: 本書の初版は「キャッシュ機構なし」を前提としていたが、コード検証で訂正した。
> インクリメンタルキャッシュは**既に存在し LSP で稼働している**。残る問題は
> (a) CLI 経路の未統合、(b) SCC 検出の不在、の 2 点である。

### 既にあるもの

| 機構 | 場所 | 内容 |
|------|------|------|
| `SourceFingerprint([u8; 32])` | `crates/lsharp-ir/src/lib.rs:23` | ソースのハッシュ。`from_source()` で生成 |
| `CompilationCache` | `crates/lsharp-ir/src/cache.rs:215-256` | module 名 → `ModuleCacheEntry` の HashMap + `LinkedModuleCache` |
| `ModuleCacheEntry` / `ModuleIrSegments` | `cache.rs` | parse 済み Program (Arc)、型サーフェス (`ModuleTypeSurface`)、IR 7 セグメント (defns/accessors/trait_impls/constraints/ctors/lifted 系) |
| `analyze_single_file_incremental` | `lib.rs:1792-1820` | fingerprint 一致なら再解析スキップ。不一致なら parse → `infer_program` → cache 更新 |
| `analyze_multi_file_incremental_with_overrides` | `lib.rs:1822+` | マルチファイル版 (LSP の未保存バッファ override 対応) |
| LSP での利用 | `crates/lsharp-lsp/src/lib.rs:37-38` | `compilation_cache: Arc<RwLock<CompilationCache>>` を常駐保持 |
| ベンチ | `crates/lsharp-wasm/tests/e2e/incremental_benchmark.rs` | キャッシュ効果の計測が既にある |

### 足りないもの

1. **CLI 経路**: `compile_multi_file(entry_file: &Path) -> Result<Module, String>`
   (`lib.rs:1777-1779`) はキャッシュを受け取らない。実処理 `compile_multi_file_with_mode`
   (`lib.rs:1647-1775`) は毎回 `ModuleGraph::build_from_entry()` (`module_graph.rs:578-582`) で
   グラフ構築 → トポロジカル順に各モジュールを `infer_program` する
2. **SCC 検出**: `ModuleGraph` にあるのは DFS トポロジカルソート (`module_graph.rs:221-243`) と
   循環検出 `detect_cycles` (`:168-216`、循環は `ModuleGraphError::CyclicDependency`) のみ。
   相互再帰モジュール群 (Formatter 3 モジュール) は SCC として扱われず、
   merged 一括推論 (`lower_multi_file_merged`) への特別フォールバックで処理される
   (`completion-criteria.md:18` の制約)
3. **グラフレベルの fingerprint**: import 構造の変化検知はなく、グラフは毎回再構築

## 設計

### 1. SCC (強連結成分) 単位の型推論 (Phase C-1)

1. `module_graph.rs` に Tarjan の SCC アルゴリズムを追加:
   `pub fn scc_groups(&self) -> Vec<Vec<String>>` (逆トポロジカル順の SCC リスト)。
   既存の `detect_cycles` / `topological_sort` は当面残し、SCC 導入後に
   「サイズ > 1 の SCC を許容するか循環エラーにするか」を import 種別で判定する
   (現状 `CyclicDependency` がエラーになる経路との互換に注意 — Formatter 3 モジュールが
   現状どう通っているかをテストで固定してから変更する)
2. `compile_multi_file_with_mode` の「モジュールごとに `infer_program`」ループ
   (`lib.rs:1703-1718`) を「SCC ごとに 1 つの `Program` に宣言を連結して `infer_program`」へ変更。
   `Infer::infer_program` (`crates/lsharp-types/src/infer.rs:308-435`) は複数モジュール分の
   宣言を 1 回で処理できる既存能力をそのまま使う (シグネチャ変更なし)
3. サイズ 1 の SCC は従来の単独推論と完全に同じ経路になるため、既存挙動を自然に包含する
4. 完了条件: Formatter 3 モジュール (`Tools.Text.FormatterExpr` / `FormatterDecl` / `Formatter`)
   がサイズ 3 の SCC として検出され、merged 特別扱いなしで
   `SELFHOST_LSP_RUNTIME_MODULES` fixture の既存経路が green になる

### 2. CLI キャッシュ統合 (Phase C-2)

1. `compile_multi_file_with_cache(entry_file, cache: &mut CompilationCache)` を公開し、
   既存 `analyze_multi_file_incremental_with_overrides` の解析部 + lowering 再利用
   (`ModuleIrSegments`) を CLI コンパイルへ接続する。`compile_multi_file` は
   空キャッシュ移譲のラッパとして互換維持
2. キャッシュキーを「自モジュールの fingerprint」から
   「自 fingerprint + 依存 SCC のキー」の合成へ拡張する (依存の公開型サーフェスが
   変わったら下流を無効化)。`ModuleCacheEntry` に `deps_key: u64` を追加
3. ディスク永続化 (CLI 再実行間のキャッシュ) は、上記のプロセス内統合の効果を
   `incremental_benchmark.rs` で計測した後に判断する (本書では設計しない)

### 3. テスト戦略 (TDD)

1. RED: 相互再帰 2 モジュールの最小 fixture を `compile_multi_file` に渡すテスト
   (現状の挙動 — merged 特別扱いが要る/CyclicDependency になる — を先に固定)
2. GREEN: SCC 導入で特別扱いなしに通す。IR スナップショット不変を確認
3. キャッシュ: 同一入力 2 回目で `note_incremental_type_infer` 系カウンタ
   (`lib.rs:1808` 参照) が増えないことを検証。dirty change 後の cached compile が
   fresh compile と一致する既存 E2E
   (`test_e2e_selfhost_cli_compile_functions_data_with_cache_matches_fresh_compile_after_change`)
   を CLI 経路でも流用
4. 依存変更の無効化: モジュール A の公開シグネチャ変更で依存先 B が再解析されるテスト

## 検証済み部分実装 (2026-07-24)

Phase C-1 の最初の狭い slice として、`ModuleGraph::scc_groups()` を追加した。
Tarjan 法で強連結成分を求め、モジュール名と import の走査順をソートして、結果を
依存先が先に来る deterministic order で返す。SCC 内のモジュール名もソートするため、
同じグラフを複数回処理しても結果が変わらない。グラフにない import は暗黙に追加せず、
既存の `check_imports` による未解決 import 検証を維持する。

`test_scc_groups_are_stable_and_dependency_first` で、`Base` → 相互再帰する
`CycleA`/`CycleB` → `Consumer` の順序と反復安定性を固定した。これは SCC 検出 API の
契約を閉じた verified partial slice であり、推論経路の変更ではない。

### 未完了の後続作業

- `compile_multi_file_with_mode` を SCC ごとの宣言連結・一括 `infer_program` に変更する。
- Formatter 3 モジュールの merged 特別扱いを除去し、相互再帰 fixture の IR/runtime parity を確認する。
- CLI の `CompilationCache` 統合、依存 SCC key、selfhost compiler への移植を行う。
- `LEGACY-MODULE-01` の aggregate 完了条件（両対応 target、native stage0、公開 command）を満たすまで、
  TODO の active item は完了扱いにしない。

## 影響範囲

- `module_graph.rs` (1597 行) と `lib.rs` の compile 系が主対象。imp-06 の分割を
  **先に**行う (module_graph → graph 構築 / 解決 / SCC、lib.rs → compile.rs 切り出し)
- `Infer::infer_program` は変更しない。selfhost コンパイラ側にも同じ SCC 方式を
  移植する後続タスクを TODO 化する (selfhost の compile_multi_file 相当に同じ制約がある)

## ステータス

設計 (2026-06-12 起草、同日コード検証に基づき大幅訂正・具体化)。2026-07-24 に
Phase C-1a の deterministic SCC 検出 API と unit test を検証済み部分実装として反映した。
一括推論、CLI キャッシュ、selfhost 移植は未着手のため、Phase C-1 / C-2 の aggregate 完了とは扱わない。
着手時は TODO.md に Phase C-1 / C-2 として項目を作成する。
