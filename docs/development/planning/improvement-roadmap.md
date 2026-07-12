# 改善ロードマップ (Improvement Roadmap)

> **役割**: [ISSUES.md](../../../ISSUES.md) (問題台帳) に記録された全 24 件の問題を、
> 改善フェーズへマッピングし、各フェーズの完了条件 (exit criteria) を定める。
> タスク化して着手する際は TODO.md (タスク正本) に項目を作成し、本書とは ID で相互参照する。
>
> **新設計の正本**: [improvement-designs/](improvement-designs/README.md) (テーマ別 6 本)。
> WasmGC バックエンドの設計正本は [v2-designs/v2-07](v2-designs/v2-07-wasmgc-optional-backend.md) であり、
> imp-01 はその補遺として扱う。
>
> **既存計画との関係**: completion-criteria.md / runtime-stability-spec.md /
> memory-management-roadmap.md の公式状態 (deferred / documented limitation / 仕様固定) は
> 本書で上書きしない。本書は「品質改善」の観点でそれらを横断的に束ねる。

---

## フェーズ構成

```
Phase A: 基盤健全化        -- コード組織・エラー処理・メモリ上限 (他フェーズの前提)
Phase B: 型システム・実行系 -- WasmGC 移行と高度型機能の実行対応、GC 性能
Phase C: モジュール・配布   -- インクリメンタル化、native backend の追跡
Phase D: ドキュメント・品質 -- ユーザー導線、テスト体系
```

Phase A は B〜D の作業効率と安全性を底上げするため最優先とする。
B 以降は依存が薄いものから並行着手してよい。

---

## Issue → フェーズ マッピング (全 24 件)

| Issue | 問題 (要約) | フェーズ | 設計 doc | 備考 |
|-------|------------|---------|----------|------|
| I-01 | ファイルサイズ規約超過 | A | [imp-06](improvement-designs/imp-06-large-file-decomposition.md) | |
| I-02 | エラーハンドリング不統一 | A | [imp-02](improvement-designs/imp-02-error-handling-unification.md) | |
| I-03 | GC 固定スロット上限 | A | [imp-03](improvement-designs/imp-03-dynamic-memory-layout.md) | |
| DOC-06 | エラーコード体系未定義 | A | imp-02 | error-reference / MCP lookup は完了、診断貫通は I-02 |
| D-01 | WasmGC i64 フォールバック | B | [imp-01](improvement-designs/imp-01-wasmgc-full-migration.md) | v2-07 補遺 |
| D-02 | GADT 実行未検証 | B | imp-01 | D-01 依存 |
| D-03 | HKT 実行未対応 | B | imp-01 | D-01 依存 |
| D-04 | Computation Expression MVP | B | imp-01 | D-01 依存 |
| D-05 | 正規表現制約が簡易版 | B | [imp-08](improvement-designs/imp-08-regex-constraint-engine.md) | resolved |
| D-06 | 動的ディスパッチなし | B | imp-01 | D-01 依存 |
| D-09 | selfhost ADT 整数タグ表現 | B | imp-01 | D-01 依存 |
| D-10 | GC sentinel edge case (G1) | B | imp-03 | documented limitation 維持、精密判別は任意 |
| I-04 | GC フリーリスト線形探索 | B | imp-03 | |
| I-07 | rooting 修正の頻発 | B | [imp-07](improvement-designs/imp-07-test-verification-infrastructure.md) | rooting 規約の明文化 (下記 B-4) |
| D-07 | 相互再帰モジュール一括推論 | C | [imp-04](improvement-designs/imp-04-module-system-strengthening.md) | |
| I-05 | CLI 経路の未キャッシュ・SCC なし | C | imp-04 | V2-01 と接続 |
| D-08 | Native backend research scope | C | -- | V2-08/V2-09 へ委譲済み、追跡のみ |
| DOC-01 | ユーザーガイド不足 | D | [imp-05](improvement-designs/imp-05-docs-restructure.md) | |
| DOC-02 | book/ 読者層混在 | D | imp-05 | |
| DOC-03 | doc-status 未運用 | D | imp-05 | |
| DOC-04 | examples 連携不足 | D | imp-05 | |
| DOC-05 | language-guide 二重管理 | D | imp-05 | |
| I-06 | fuzz/リーク/限界テスト欠落 | D | imp-07 | 下記 D-3 |
| I-08 | テストカバレッジ偏り | D | imp-07, imp-06 | テスト分割と同時 |

---

## Phase A: 基盤健全化

**目的**: 後続フェーズの大規模変更 (WasmGC 移行、モジュールシステム改修) を安全に進めるため、
コードの可読性・診断品質・メモリ上限という土台を先に整える。

| # | 作業 | 対象 issue | 設計 |
|---|------|-----------|------|
| A-1 | エラーハンドリング統一: 下層クレートへ miette Diagnostic を貫通させ、本番経路の panic を排除。`LS####` エラーコード体系を導入 | I-02, DOC-06 | imp-02 |
| A-2 | 大規模ファイル分割: wasi.rs / main.rs / infer.rs / ir/lib.rs を筆頭に規約 (500-800 行) へ分割 | I-01 | imp-06 |
| A-3 | GC メモリレイアウト動的化: 固定スロット (4096 / 32768) の grow 戦略導入 | I-03 | imp-03 |

**順序**: A-1 を最初に行う (エラー型のシグネチャ変更がファイル分割の切断面に影響するため)。
A-2 と A-3 は独立に並行可。

**Exit criteria**:
- [pending] 下層 4 クレートの本番経路に `panic!` / `unwrap()` / `expect()` による異常終了経路がない (テストコードは除く)
- [pending] 全診断に `LS####` コードが付与され、CLI / LSP / MCP が同一コードを返す
- [pending] `crates/**/src/*.rs` の全ファイルが 800 行以下 (`wc -l` で機械検査可能)
- [pending] GC オブジェクトテーブル / root stack が初期容量超過時に grow し、上限到達時は panic ではなく診断付きエラーになる
- [pending] 既存テストが全件 green を維持 (`cargo test`)

## Phase B: 型システム・実行系

**目的**: 「型チェックのみ」に留まる高度型機能 (GADT / HKT / Computation Expression) を
実行可能にし、レコード/ADT の型情報を Wasm 層まで保持する。GC の実行性能を改善する。

| # | 作業 | 対象 issue | 設計 |
|---|------|-----------|------|
| B-1 | WasmGC バックエンド実装 (v2-07 の段階移行: Records/ADT → Strings → Closures)。i64 フォールバック TODO (emit.rs) の解消 | D-01, D-02, D-03, D-04, D-06, D-09 | imp-01 |
| B-2 | GC フリーリストのサイズクラス化 (線形探索の解消) | I-04 | imp-03 |
| B-3 | 正規表現エンジン (WG-2) による `matches` 制約の完全化 | D-05 | imp-08 (done) |
| B-4 | GC rooting 規約の明文化と lint 化: 「heap 値を helper 呼び出しを跨いで保持する場合は root する」規律を selfhost コード規約 + 契約テストとして固定 | I-07 | imp-07 |
| B-5 | (任意) G1 precise discrimination の再評価 | D-10 | imp-03 |

**Exit criteria**:
- [done] `matches` 制約が shared regex engine を使い、bounded quantifier / shorthand negation / non-capturing group / lazy suffix / Unicode class を docs と focused tests で固定。Evidence: `test_regex_bounded_quantifiers`, `test_string_constraint_uses_shared_regex_extended_features`
- [pending] `--backend=wasmgc` で examples/gadt.ls, hkt.ls, computation.ls が「型チェックのみ」注記なしで実行され、期待出力を返す E2E がある
- [pending] `crates/lsharp-wasm/src/emit.rs` の「WasmGC 本格実装時に削除」TODO が 0 件
- [pending] トレイトの動的ディスパッチ (vtable) の最小ケースが E2E で通る
- [pending] アロケーション heavy ベンチで フリーリスト探索が割り当て件数に対して定数時間相当になる計測証跡
- [pending] rooting 規約が docs に明文化され、違反検出テスト (既存 guard test の拡張) が CI にある

## Phase C: モジュール・配布

**目的**: モジュール単位の独立解析を可能にし、インクリメンタルコンパイル / LSP 応答性の
基盤を作る。native backend は V2 トラックの進捗を追跡する。

| # | 作業 | 対象 issue | 設計 |
|---|------|-----------|------|
| C-1 | SCC (強連結成分) 単位の型推論: 相互再帰モジュール群を SCC として検出し、SCC 単位で推論。単一モジュールの個別推論を可能にする | D-07 | imp-04 |
| C-2 | モジュールグラフ / 解析結果のキャッシュ: fingerprint ベースの再利用。V2-01 (LSP incremental sync) の前提を提供 | I-05 | imp-04 |
| C-3 | Native backend 追跡: V2-13a-5 (Linux x86_64 stage chain) / V2-08 / V2-09 は TODO.md を正本として進捗を追う。本ロードマップでは新規作業を定義しない | D-08 | -- |

**Exit criteria**:
- [pending] Formatter 3 モジュール (FormatterExpr / FormatterDecl / Formatter) が SCC として自動検出され、`compile_multi_file` の特別扱いコメント (completion-criteria.md:18 記載の制約) が不要になる
- [pending] 無変更モジュールの再コンパイルがキャッシュヒットする計測証跡 (2 回目コンパイルの解析スキップ)
- [pending] D-08 は V2-08 / V2-09 / V2-13 の close をもって ISSUES.md 上で resolved に遷移 (本フェーズの成果物ではない)

## Phase D: ドキュメント・品質

**目的**: 新規ユーザーの導線を整備し、テスト体系の偏りを正す。

| # | 作業 | 対象 issue | 設計 |
|---|------|-----------|------|
| D-1 | docs/guides/ 拡張: metadata 仕様 / IDE セットアップ / デプロイターゲット / stdlib ガイドを追加。エラーコードリファレンスは LS#### 体系導入後に DOC-06 として追加 | DOC-01, DOC-06 | imp-05 |
| D-2 | book/ 読者層分離、examples ↔ 機能マトリクス整備、language-guide テンプレートの正本一本化、doc-status の CI 運用開始 | DOC-02, DOC-03, DOC-04, DOC-05 | imp-05 |
| D-3 | テスト体系強化: パーサー/型推論の property-based テスト (proptest) 導入、GC リーク検出テスト、スロット上限・再帰深度の限界値テスト、occur check 性能計測 | I-06 | imp-07 |
| D-4 | テスト配置の再編: 巨大インラインテストの分離、syntax/types のユニットテスト増強 | I-08 | imp-06, imp-07 |

**Exit criteria**:
- [done] docs/guides/ に metadata / IDE / deployment / stdlib guide が存在し、site.toml 経由で公開サイトに載る。Evidence: `test_doc_site_manifest_exposes_user_guide_expansion`, `test_cmd_doc_site_generates_guides_and_api_site`
- [done] error-reference.md が LS#### 体系導入後に DOC-06 / imp-02 と同期して追加される。Evidence: `test_error_reference_doc_mentions_all_mcp_error_codes`, `test_errors_tool_returns_ls_error_code_reference_and_legacy_alias`
- [done] book/ の読者層がコンパイラ実装を読む開発者向けに分離され、docs/guides/ が利用者向け入口として明示される。Evidence: `test_doc_site_manifest_separates_user_guides_from_implementation_book`
- [done] examples の全 .ls がドキュメントの機能マトリクスから参照され、「型チェックのみ」サンプルが明示される。Evidence: `test_doc_site_manifest_exposes_examples_matrix`
- [done] language-guide テンプレートが docs/guides/ と docs/site.toml を SSOT として明記する。Evidence: `test_lsharp_language_guide_template_points_to_docs_guides_as_ssot`
- [done] `.lsharp-doc-status` がリポジトリで運用され、CI で doc-check が走る。Evidence: `test_repo_doc_status_dogfooding_is_wired_for_metadata_fixture`, `scripts/ci/doc-status-check.sh`
- [pending] fuzz ターゲットが CI (または定期ジョブ) で実行される
- [pending] I-06 記載の限界値 (GC スロット / 再帰深度) が計測され、ドキュメント化される

---

## 運用規則

- フェーズ内の作業に着手する際は TODO.md に項目を作成し、本書の `A-1` 等の ID を記載する
- exit criteria の `[pending]` は達成時に `[done]` + 証跡 (テスト名 / コミット) へ更新する
- 対象 issue が resolved になったら ISSUES.md 側の状態も更新する (本書からは状態を二重管理しない)
- 完了条件の正本が他文書にあるもの (D-08 → completion-criteria.md / TODO.md、D-10 → runtime-stability-spec.md) は本書で再定義しない
