# ADR: Rust コードベース撤去

## ステータス

提案

## コンテキスト

L# セルフホストコンパイラが Phase 11 完了基準を満たした後、Rust 実装を撤去する。撤去は不可逆な操作であるため、明確なスコープ・順序・ロールバック手順を事前に定義する。

## 決定

### 撤去前提条件

以下の全条件を満たした後に撤去を開始する:

1. `ci-gate-v2` が native-only パイプラインで 2 週間安定
2. fresh clone テスト（OPS-07）が Rust 無しで pass
3. パフォーマンスベンチマークで Rust 版比 2x 以内
4. ステークホルダーによる ADR レビュー完了
5. `v0.x.y-rust-final` タグの作成

### 撤去スコープ

以下を **削除順** に処理する。依存度の低い周辺クレートから先に削除し、コアコンパイラは最後に削除する。

| 順序 | 対象 | 説明 |
|------|------|------|
| 1 | `crates/lsharp-docs/` | ドキュメントツール |
| 2 | `crates/lsharp-lsp/` | LSP サーバー |
| 3 | `crates/lsharp-driver/` | CLI ドライバー |
| 4 | `crates/lsharp-wasm/` | Wasm コード生成 |
| 5 | `crates/lsharp-ir/` | 中間表現 (IR) |
| 6 | `crates/lsharp-types/` | 型推論・制約解決 |
| 7 | `crates/lsharp-syntax/` | 字句解析・構文解析 |
| 8 | `Cargo.toml`, `Cargo.lock` | Rust ワークスペース定義 |
| 9 | `rust-toolchain.toml`, `.cargo/` | Rust ツールチェーン設定 |

### 段階的削除の手順

各ステップで以下を実施する:

1. 対象クレートを削除
2. `Cargo.toml` の workspace members から除外
3. CI ジョブが pass することを確認
4. コミット・プッシュ

最終ステップ（#8, #9）では `Cargo.toml` 自体を削除するため、CI パイプラインは事前に selfhost ベースに完全移行済みであること。

### ロールバック手順

#### 即時ロールバック（撤去中）

撤去途中で問題が発見された場合:

```bash
git revert <removal-commit>
cargo build   # Rust コンパイラ復元
```

#### 完全ロールバック（撤去完了後）

撤去完了後にロールバックが必要な場合:

1. `v0.x.y-rust-final` タグからチェックアウト
   ```bash
   git checkout v0.x.y-rust-final
   ```
2. Rust コンパイラをビルド
   ```bash
   cargo build --release
   ```
3. CI ジョブを Rust パスに切り替え
   ```bash
   bash scripts/rollback.sh
   ```
4. テスト実行で復元を検証
   ```bash
   cargo test
   ```

#### `legacy-rust-bootstrap/` からの復元

`legacy-rust-bootstrap/` ディレクトリにスナップショットが保存されている場合:

```bash
# rollback-procedure.md の手順に従う
bash scripts/rollback.sh --dry-run  # シミュレーション
bash scripts/rollback.sh            # 実行
```

### ロールバックが必要なシナリオ

| シナリオ | 重大度 | 対応 |
|----------|--------|------|
| L# コンパイラの致命的バグ発見 | Critical | 即時ロールバック + ホットフィックス |
| パフォーマンス回帰 (>2x) | High | ベンチマーク調査 + 条件付きロールバック |
| プラットフォーム互換性問題 | High | 対象プラットフォームのみ Rust fallback |
| ブートストラップ破損 | Critical | stage0 バイナリから復元 |
| CI パイプライン障害 | Medium | CI 設定の修正で対応 |

### CI リカバリー手順

撤去後の CI で問題が発生した場合:

1. `scripts/rollback.sh` スクリプト実行
   ```bash
   bash scripts/rollback.sh
   ```
2. `.github/workflows/ci.yml` を Rust パスに切り替え
   - `cargo test` / `cargo clippy` / `cargo fmt` ジョブを復元
   - `dtolnay/rust-toolchain` ステップを追加
3. `ci-gate` の必須ジョブを更新
   - selfhost ベースのジョブを Rust ベースに差し替え
4. Branch Protection Rules を更新

## 結果

### メリット

- リポジトリサイズの削減（Rust ソース + `target/` キャッシュ）
- ビルド依存の簡素化（Rust ツールチェーン不要）
- メンテナンスコストの削減（2 つのコンパイラ実装を維持しない）

### リスク

- ロールバック時の復元コスト
- Rust エコシステムツール（clippy, miri 等）の喪失
- 過渡期の CI 不安定性

### 緩和策

- `v0.x.y-rust-final` タグによる完全復元ポイント
- `legacy-rust-bootstrap/` スナップショット保持
- 段階的削除による影響範囲の最小化

## 証跡

- `scripts/rollback.sh`
- `docs/development/operations/rollback-procedure.md`
- `legacy-rust-bootstrap/`
- `crates/lsharp-wasm/tests/e2e/selfhost_lsp_docs_ops.rs` (`test_e2e_ops08_final_removal_rollback`)
