# imp-04: モジュールシステム強化 (SCC 推論とインクリメンタル基盤)

> 対象 issue: [D-07](../../../../ISSUES.md#d-07) (相互再帰モジュールの一括推論)、[I-05](../../../../ISSUES.md#i-05) (モジュールグラフ毎回再構築)
> ロードマップ: [improvement-roadmap.md](../improvement-roadmap.md) Phase C-1 / C-2
> 関連: [v2-designs/v2-01-lsp-incremental-sync.md](../v2-designs/v2-01-lsp-incremental-sync.md) (LSP 側の受け皿)

## 概要

現状のマルチファイル型検査には 2 つの構造的制約がある:

1. **相互再帰の特別扱い** (D-07): `Tools.Text.FormatterExpr` / `FormatterDecl` / `Formatter` は
   相互再帰のため、`lsharp_ir::compile_multi_file` が 3 モジュールをまとめて 1 回で型推論する
   必要がある (個別モジュール順だと `format-expr` が未束縛になる。
   `docs/development/planning/completion-criteria.md:18`)。この特別扱いは呼び出し側の知識に
   依存しており、新たな相互再帰モジュール群を追加するたびに同じ罠を踏む。
2. **解析結果の使い捨て** (I-05): モジュール依存グラフ (`crates/lsharp-ir/src/module_graph.rs`)
   と各モジュールの推論結果がコンパイル実行ごとに再構築され、無変更モジュールの結果を
   再利用できない。

## 設計

### 1. SCC (強連結成分) 単位の型推論 (Phase C-1)

- モジュール依存グラフ上で Tarjan 等の SCC 検出を行い、推論単位を
  「単一モジュール」から「SCC」へ一般化する
- SCC のトポロジカル順に推論する。サイズ 1 の SCC (相互再帰なし) は従来どおり
  単独推論となるため、既存挙動は自然に包含される
- Formatter 3 モジュールはサイズ 3 の SCC として自動検出され、呼び出し側の
  特別扱い (一括で渡す前提) が不要になる
- 公開インターフェース: `compile_multi_file` のシグネチャは維持し、内部で SCC 分割する。
  個別モジュール推論の入口 (LSP 用) は「対象モジュールが属する SCC」を解決して推論する

### 2. モジュールグラフ / 推論結果のキャッシュ (Phase C-2)

- **キー**: モジュールソースの fingerprint (selfhost 側に既存の source-fingerprint 概念があるため
  同等のハッシュを Rust 側でも採用) + 依存する SCC のキャッシュキー (推移的に伝播)
- **値**: モジュールの公開シグネチャ (TypeEnv への寄与分) と lowering 結果
- **無効化**: fingerprint 不一致、または依存 SCC のキー変化で該当 SCC 以降のみ再計算。
  グラフの辺が変わった場合 (import 追加/削除) はグラフ再構築から行う
- **保存先**: 第 1 段階はプロセス内キャッシュ (LSP の常駐プロセスで効く)。
  ディスク永続化 (CLI の再実行間で効く) は効果測定後に判断する
- V2-01 (LSP incremental sync) はテキスト同期の増分化であり、本書は解析の増分化。
  両者が揃って LSP の応答性目標が達成される

### 3. テスト戦略 (TDD)

1. RED: 「相互再帰 2 モジュールを compile_multi_file に**個別順で**渡しても成功する」テストを追加
   (現行では未束縛エラーになることを先に固定)
2. GREEN: SCC 推論の実装で green 化。Formatter 3 モジュールの既存経路
   (`SELFHOST_LSP_RUNTIME_MODULES` fixture) の無回帰を確認
3. キャッシュ: 同一入力の 2 回目コンパイルで推論呼び出し回数が減ることをカウンタで検証する
   ユニットテスト + 変更時に正しく無効化されることのテスト (dirty module change 後の
   cached compile が fresh compile と一致する既存 E2E 群を流用)
4. スナップショット: IR スナップショットが SCC 導入前後で不変であることを確認

## 影響範囲

- `crates/lsharp-ir/src/module_graph.rs` (1597 行) が主対象。imp-06 のファイル分割と
  同時期に行う場合は、分割 → SCC 導入の順にする (切断面が明確になるため)
- 型推論器 (`crates/lsharp-types/src/infer.rs`) には「複数モジュールの宣言を 1 つの
  推論コンテキストで処理する」既存能力をそのまま使い、変更を最小化する
- selfhost コンパイラ側のマルチファイル推論にも同じ制約があるため、Rust 側で確立した
  SCC 方式を selfhost へ移植する後続タスクを TODO 化する

## ステータス

設計のみ (2026-06-12 起草)。着手時は TODO.md に Phase C-1 / C-2 として項目を作成する。
