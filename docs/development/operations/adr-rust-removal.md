# ADR: Rust ワークスペース維持と配布モデル転換

## ステータス

提案 (レビュー待ち)

> **関連ドキュメント**
> - Phase 13 移行前ゲート定義: [`docs/development/planning/completion-criteria.md` § P11-2e-3](../planning/completion-criteria.md)
> - ロールバック手順詳細: [`docs/development/operations/rollback-procedure.md`](./rollback-procedure.md)
> - 配布・署名: [`docs/development/operations/release-distribution-signing.md`](./release-distribution-signing.md)

## コンテキスト

当初は、L# セルフホストコンパイラが Phase 11 完了基準を満たした後に Rust 実装を物理撤去する方針だった。しかし 2026-03-30 の Component Model pivot により、正式配布モデルは **Wasmtime embedding + guest Wasm component + host launcher single binary** に変更された。これに伴い、Rust workspace は host launcher / component tooling context として継続利用し、物理削除を完了条件から外す必要がある。

## 決定

### 方針転換の前提条件

以下の全条件を満たした後に、新配布モデルへの運用移行を正式化する。各条件の現在ステータスを示す。

| # | 条件 | ステータス | 備考 |
|---|------|-----------|------|
| 1 | `ci-gate-v2` が host launcher + component パイプラインで 2 週間安定 | **PENDING** | 安定期間未開始 |
| 2 | fresh clone テスト（OPS-07）がエンドユーザー視点で Rust 無しで pass | **PENDING** | 現行 OPS-07 は clean checkout smoke の暫定 gate |
| 3 | host launcher 経由の component smoke が release gate に固定 | **PENDING** | release / README smoke の更新途中 |
| 4 | ステークホルダーによる ADR レビュー完了 | **PENDING** | レビュー証跡なし (下記「レビュー記録」参照) |
| 5 | rollback 手順が「embedded compiler component の巻き戻し」として確定 | **PENDING** | 文書更新は完了。last-known-good release tag / package 運用の固定が残る |

> ※ 上記は `completion-criteria.md` の P11-2e-3 ゲートと対応する。条件が満たされた時点で各行を更新し、evidence (CI run URL / tag URL / reviewer) を追記する。

### 取り下げた前提

以下の前提は **withdrawn** とする。

- Phase 11 の完了が Rust workspace の物理削除を含む
- fresh clone / release / rollback の主経路が native-only artifact である
- rollback の最終到達点が「Rust 実装の復元」である

### 維持スコープ

以下を **維持対象** として扱う。削除順ではなく、Phase 13 配布モデルでの責務を明示する。

| 対象 | 役割 | 備考 |
|------|------|------|
| `crates/lsharp-driver/` | host launcher / CLI エントリポイント | Wasmtime embedding と配布単一バイナリの中心 |
| `crates/lsharp-wasm/` | guest compiler component の codegen / packaging 補助 | component 出力の検証と build bridge を担う |
| `crates/lsharp-lsp/`, `crates/lsharp-docs/` | host 側の運用・IDE・文書 tooling | guest component と同じ配布面に接続 |
| `crates/lsharp-ir/`, `crates/lsharp-types/`, `crates/lsharp-syntax/` | stage0 / tooling / oracle 比較 | selfhost bootstrap と検証文脈で継続利用 |
| `Cargo.toml`, `Cargo.lock`, `rust-toolchain.toml`, `.cargo/` | Rust ワークスペース定義 | 物理削除しない |

### 運用移行の手順

各ステップで以下を実施する:

1. host launcher + embedded component を正本配布物として扱う
2. fresh clone / release / rollback / verification 文書を同じ用語で揃える
3. CI ジョブが component smoke を blocking で実行することを確認
4. コミット・プッシュ

Rust workspace 自体は削除しないため、最終ステップは「workspace の責務を host launcher / component tooling へ限定したことの確認」とする。

### ロールバック手順

#### 即時ロールバック（リリース運用中）

host launcher / embedded component の配布切替中に問題が発見された場合:

```bash
git revert <cutover-commit>
cargo build   # host launcher が再ビルドできることを確認
```

#### 配布物ロールバック（公開済みリリース）

公開済みリリースにロールバックが必要な場合:

1. 直前の正常タグからチェックアウト
   ```bash
   git checkout v<last-known-good>
   ```
2. host launcher をビルド
   ```bash
   cargo build --release
   ```
3. 正常な guest component を再埋め込み、または前回の component package を再採用
   ```bash
   bash scripts/rollback.sh
   ```
4. Wasm component smoke で復元を検証
   ```bash
   cargo test
   ```

#### source snapshot の参照

`legacy-rust-bootstrap/` ディレクトリが存在しても、それは比較・監査用スナップショットであり primary rollback path ではない。必要時のみ差分確認に使う。

```bash
# rollback-procedure.md の手順に従う
bash scripts/rollback.sh --dry-run  # シミュレーション
bash scripts/rollback.sh            # 実行
```

### ロールバックが必要なシナリオ

| シナリオ | 重大度 | 対応 |
|----------|--------|------|
| embedded compiler component の致命的バグ発見 | Critical | 直前の正常 component へ巻き戻し + ホットフィックス |
| host launcher と component の ABI 不整合 | Critical | 同一タグの組へ再パッケージ |
| プラットフォーム互換性問題 | High | 対象プラットフォームの host launcher package を差し替え |
| ブートストラップ破損 | Critical | stage0 package から再構成 |
| CI パイプライン障害 | Medium | CI 設定の修正で対応 |

### CI リカバリー手順

新配布モデル移行後の CI で問題が発生した場合:

1. `scripts/rollback.sh` スクリプト実行
   ```bash
   bash scripts/rollback.sh
   ```
2. `.github/workflows/ci.yml` で前回正常な host launcher / component package を使う経路に戻す
   - `cargo test` / `cargo clippy` / `cargo fmt` は継続利用する
   - release / smoke job が正常な component smoke に向くことを確認する
3. `ci-gate` の必須ジョブを更新
   - 失敗している新配布経路を last-known-good package に差し替え
4. Branch Protection Rules を更新

## 結果

### メリット

- host launcher / guest component という正式配布モデルを明示できる
- Rust workspace を削らず、tooling / rollback / packaging の安全弁を維持できる
- fresh clone / release / rollback 文書を同じアーキテクチャ前提に揃えられる

### リスク

- host launcher と guest component の責務境界が曖昧だと文書が再び分岐する
- Rust workspace 維持により「完全 selfhost 完了」と誤解される可能性がある
- 過渡期の CI 不安定性

### 緩和策

- rollback 対象を host launcher / embedded component の組み合わせとして固定する
- `legacy-rust-bootstrap/` を監査用スナップショットとして保持する
- Phase 13 完了条件と運用 runbook を相互参照にする

## 証跡

### 実装済み証跡

- `scripts/rollback.sh`
- `docs/development/operations/rollback-procedure.md`
- `legacy-rust-bootstrap/`
- `crates/lsharp-wasm/tests/e2e/selfhost_lsp_docs_ops.rs`（OPS 系 rollback E2E）

### レビュー記録

> **PENDING**: ADR レビューはまだ完了していない。レビュー完了時に以下を記録する。

| 項目 | 内容 |
|------|------|
| レビュアー | (未定) |
| レビュー日 | (未完了) |
| レビュー形式 | PR コメント / 対面 / async |
| 指摘事項 | (なし or 箇条書き) |
| 承認コミット/PR | (URL) |

### 運用移行承認

> **PENDING**: 方針転換の前提条件が全て満たされた後、ここに承認記録を追記する。

| 項目 | 内容 |
|------|------|
| last-known-good release tag | (URL / SHA) |
| CI 安定期間開始日 | (未開始) |
| CI 安定期間終了日 | (未完了) |
| CHANGELOG / ADR 記録箇所 | (未記録) |
| host-launcher RC バージョン | (未作成) |
| RC 検証者 | (未定) |
