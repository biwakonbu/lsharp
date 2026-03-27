# Rust 不要 Fresh Clone 仕様

Rust ツールチェーンを必要としない、L# プロジェクトの fresh clone からビルド・テスト・配布までの手順を定義する。

## 目的

Phase 11 完了後、エンドユーザーは Rust をインストールせずに L# コンパイラを取得・ビルド・利用できるようにする。

## 手順

### 1. リポジトリ取得

```bash
git clone https://github.com/<org>/lsharp.git
cd lsharp
```

Rust ツールチェーン（`rustc`, `cargo`）は不要。

### 2. Stage0 バイナリ取得

```bash
# プリビルト stage0 バイナリを GitHub Releases から取得
./scripts/fetch-stage0.sh
```

- OS / アーキテクチャを自動検出
- `stage0/lsharp` として配置
- チェックサム検証を実施

### 3. ブートストラップ

```bash
# stage0 → stage1 → stage2 の 3 段階ブートストラップ
./scripts/bootstrap.sh
```

| ステージ | 入力 | 出力 | 説明 |
|----------|------|------|------|
| stage0 → stage1 | selfhost/*.ls | stage1/lsharp.wasm | プリビルトコンパイラで selfhost をコンパイル |
| stage1 → stage2 | selfhost/*.ls | stage2/lsharp.wasm | stage1 出力で selfhost を再コンパイル |
| stage2 検証 | — | — | stage1 と stage2 の出力が一致することを確認 |

### 4. ネイティブリリースビルド

```bash
# Wasm → ネイティブバイナリ生成
./scripts/native-release.sh
```

- wasmtime AOT コンパイルまたは同等の手段でネイティブバイナリを生成
- 出力: `dist/lsharp` (実行可能バイナリ)

### 5. テスト実行

```bash
# テストスイート実行
./dist/lsharp test
```

- selfhost テストランナーによるテスト実行
- Rust の `cargo test` は使用しない

### 6. 配布パッケージ生成

```bash
# 配布用アーカイブ生成
./scripts/release-bundle.sh
```

- `dist/lsharp-<version>-<os>-<arch>.tar.gz` を生成
- SHA-256 チェックサムを付与

## CI ジョブ

### `test-fresh-clone`

```yaml
test-fresh-clone:
  name: Fresh clone (no Rust)
  runs-on: ubuntu-latest
  # main マージ毎に実行
  if: github.event_name == 'push' && github.ref == 'refs/heads/main'
  steps:
    - uses: actions/checkout@v4
    # Rust ツールチェーン setup なし
    - name: Fetch stage0
      run: ./scripts/fetch-stage0.sh
    - name: Bootstrap
      run: ./scripts/bootstrap.sh
    - name: Native release
      run: ./scripts/native-release.sh
    - name: Test
      run: ./dist/lsharp test
    - name: Release bundle
      run: ./scripts/release-bundle.sh
```

- Rust ツールチェーン無しコンテナ使用（`dtolnay/rust-toolchain` ステップなし）
- 全ステップの成功を検証

### `fresh-clone-smoke`（現行の暫定 gate）

`test-fresh-clone` を mainline に入れる前段として、現在は Rust 依存のままでも **clean checkout 由来のビルド回帰** を継続検知する `fresh-clone-smoke` を運用する。

- `scripts/ci/test-fresh-clone.sh` が `target/ci/fresh-clone-smoke/` に clean checkout 相当のコピーを作る
- そのコピー上で `cargo build -p lsharp-driver -q` を実行し、ビルド済み `lsharp` バイナリを得る
- `scripts/ci/default-path-smoke.sh` を再利用して `check` / `compile` の default-path smoke を再実行する
- 追加で `selfhost/Token.ls` と `stdlib/Core.ls` をコンパイルし、selfhost / stdlib の代表 slice が clean checkout でも壊れていないことを確認する

このジョブは **Rust 非依存化の完了を主張しない**。あくまで `test-fresh-clone` の前段で、clean checkout 経路の regressions を CI gate に載せるための暫定措置である。

## 前提条件

| 条件 | 依存タスク | 説明 |
|------|-----------|------|
| stage0 バイナリ配布 | BOOT-04 | GitHub Releases で OS / arch 別プリビルトを配布済み |
| ブートストラップ閉包 | BOOT-04 | stage0 → stage1 → stage2 が完全に閉じている |
| ネイティブバイナリ生成 | NATIVE-05 | Wasm → ネイティブ変換パイプラインが動作 |
| テストランナー | CLI-02 | `lsharp test` コマンドが機能 |

## 現状

2026 年時点では以下の制約がある:

- stage0 プリビルトバイナリの GitHub Releases 配布は未実装
- ブートストラップの完全閉包（BOOT-04）は proxy 段階
- `scripts/smoke_test_readme.sh` は Rust 前提の smoke テスト
- `fresh-clone-smoke` は clean checkout の smoke までであり、Rust 非依存 `test-fresh-clone` の代替ではない

これらが解決された後、本仕様に基づく `test-fresh-clone` ジョブを CI に追加し、`fresh-clone-smoke` は置き換える。

## 証跡

- `scripts/smoke_test_readme.sh`（現行の Rust 依存 smoke）
- `scripts/ci/test-fresh-clone.sh`（clean checkout 回帰の暫定 smoke）
- `crates/lsharp-wasm/tests/e2e/selfhost_lsp_docs_ops.rs` (`test_e2e_ops07_fresh_clone_no_rust`)
