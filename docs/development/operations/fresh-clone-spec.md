# Rust 非必須 Fresh Clone 仕様

Rust ツールチェーンを必要としない、L# プロジェクトの fresh clone から **host launcher + guest Wasm component** の取得・検証・配布までの手順を定義する。

## 目的

Phase 11 / Phase 13 移行後、エンドユーザーは Rust をインストールせずに L# コンパイラを取得・ビルド・利用できるようにする。Rust workspace は削除せず、開発者向け host launcher / component tooling context として残存する。

## 手順

### 1. リポジトリ取得

```bash
git clone https://github.com/<org>/lsharp.git
cd lsharp
```

Rust ツールチェーン（`rustc`, `cargo`）は不要。

### 2. Stage0 パッケージ取得

```bash
# プリビルト stage0 host launcher package を GitHub Releases から取得
./scripts/fetch-stage0.sh
```

- OS / アーキテクチャを自動検出
- `stage0/lsharp` として配置
- stage0 package には起動可能な host launcher と、その launcher が実行する guest compiler component が含まれる
- チェックサム検証を実施

### 3. ブートストラップ

```bash
# stage0 launcher → stage1 component → stage2 component の 3 段階ブートストラップ
./scripts/bootstrap.sh
```

| ステージ | 入力 | 出力 | 説明 |
|----------|------|------|------|
| stage0 → stage1 | selfhost/src/**/*.ls | stage1/lsharp.component.wasm | プリビルト host launcher 上の guest compiler component で selfhost 正本 source root をコンパイル |
| stage1 → stage2 | selfhost/src/**/*.ls | stage2/lsharp.component.wasm | stage1 component を host launcher に載せて selfhost 正本 source root を再コンパイル |
| stage2 検証 | — | — | stage1 と stage2 の component 出力が一致することを確認 |

### 4. Host launcher パッケージ生成

```bash
# host launcher に guest component を埋め込んだ配布パッケージを生成
./scripts/release-bundle.sh
```

- 主要成果物は `dist/lsharp`（single-binary host launcher）
- 必要に応じて検証・再埋め込み用の `dist/lsharp.component.wasm` を sidecar として同梱してよいが、主配布物は host launcher とする

### 5. テスト実行

```bash
# テストスイート実行
./dist/lsharp test
```

- selfhost テストランナーによるテスト実行
- smoke の主眼は、配布物に含まれる guest component が host launcher 経由で正常起動すること
- Rust の `cargo test` は使用しない

### 6. 配布パッケージ生成

```bash
# 配布用アーカイブ生成
./scripts/release-bundle.sh
```

- `dist/lsharp-<version>-<os>-<arch>.tar.gz` を生成
- アーカイブには single-binary host launcher を含める
- SHA-256 チェックサムを付与

## CI ジョブ

### `test-fresh-clone`

```yaml
fresh-clone-artifact:
  name: Fresh clone artifact
  runs-on: ubuntu-latest
  steps:
    - uses: actions/checkout@v4
    - uses: dtolnay/rust-toolchain@stable
    - run: bash scripts/release.sh
    - uses: actions/upload-artifact@v4
      with:
        name: fresh-clone-archive-${{ github.sha }}
        path: dist/*.tar.gz

test-fresh-clone:
  name: Test fresh clone (binary-only)
  runs-on: ubuntu-latest
  needs: fresh-clone-artifact
  steps:
    - uses: actions/checkout@v4
    - uses: actions/download-artifact@v4
      with:
        name: fresh-clone-archive-${{ github.sha }}
        path: dist/
    # Rust ツールチェーン setup なし
    - name: Binary-only smoke
      run: bash scripts/ci/test-fresh-clone.sh dist/<archive>.tar.gz
```

- `fresh-clone-artifact` が Linux 用 release-style archive を同一 workflow で作成し、`test-fresh-clone` はそれを download して検証する
- `test-fresh-clone` 側は Rust ツールチェーン無し runner を維持する（`dtolnay/rust-toolchain` ステップなし）
- `scripts/ci/test-fresh-clone.sh <archive>` が `scripts/ci/release-smoke.sh` / `scripts/ci/default-path-smoke.sh` / `scripts/smoke_test_readme.sh` を順に再利用し、downloaded artifact だけで `verify checksum -> packaged binary smoke -> default path smoke -> README Quick Start smoke` を通す
- これは stage0 package 配布前の **closest viable binary-only gate** であり、true no-Rust end-state では GitHub Releases / stage0 fetch に置き換える

### `fresh-clone-smoke`（現行の暫定 gate）

`test-fresh-clone` と並走する暫定 gate として、現在も Rust 依存の **clean checkout 由来ビルド回帰** を継続検知する `fresh-clone-smoke` を運用する。

- `scripts/ci/test-fresh-clone.sh` が `target/ci/fresh-clone-smoke/` に clean checkout 相当のコピーを作る
- そのコピー上で `cargo build -p lsharp-driver -q` を実行し、ビルド済み `lsharp` バイナリを得る
- `scripts/ci/default-path-smoke.sh` を再利用して `check` / `compile` の default-path smoke を再実行する
- 追加で `selfhost/src/Syntax/Token.ls` と `stdlib/Core.ls` をコンパイルし、selfhost / stdlib の代表 slice が clean checkout でも壊れていないことを確認する

このジョブは **Rust 非依存化の完了を主張しない**。あくまで `test-fresh-clone` と役割分担しながら、clean checkout 経路の regressions を CI gate に載せ続ける暫定措置である。

### `release-smoke`（downloaded artifact の中間 gate）

`test-fresh-clone` の前段として、release workflow では build 済み archive を `actions/download-artifact` で集約し、`scripts/ci/release-smoke.sh` で **download release artifact -> checksum verify -> packaged binary smoke** を再実行する。

- Rust toolchain setup を追加せずに `scripts/ci/release-smoke.sh` を回す
- `.tar.gz` / `.zip` archive を展開し、`checksums.txt` を検証する
- packaged `lsharp` binary の `--version` / `check` / `fmt` / `compile` を smoke する

これは true no-Rust `test-fresh-clone` の代替ではないが、release artifact download 後の binary-only 経路 regressions を早めに捕捉する中間 gate である。

## 前提条件

| 条件 | 依存タスク | 説明 |
|------|-----------|------|
| stage0 パッケージ配布 | BOOT-04 | GitHub Releases で OS / arch 別 host launcher package を配布済み |
| ブートストラップ閉包 | BOOT-04 | stage0 → stage1 → stage2 の component 生成が完全に閉じている |
| host launcher packaging | P13-3 | guest component の埋め込みまたは package 化パイプラインが動作 |
| テストランナー | CLI-02 | `lsharp test` コマンドが機能 |

## 現状

2026 年時点では以下の制約がある:

- stage0 host launcher package の GitHub Releases 配布は未実装
- ブートストラップの完全閉包（BOOT-04）は proxy 段階
- `test-fresh-clone` は workflow-local downloaded artifact を使う closest viable binary-only gate までは接続されたが、GitHub Releases / stage0 fetch を起点とする true no-Rust end-state ではない
- `fresh-clone-smoke` は clean checkout の smoke を継続検知する暫定 gate として並走する

これらが解決された後、現行の workflow-local artifact ベース `test-fresh-clone` を GitHub Releases / stage0 fetch ベースへ置き換え、`fresh-clone-smoke` は段階的に retire する。

## 証跡

- `scripts/smoke_test_readme.sh`（host launcher + component 配布 smoke への更新対象）
- `scripts/ci/test-fresh-clone.sh`（clean checkout smoke + downloaded artifact binary-only smoke）
- `scripts/ci/release-smoke.sh`（release artifact 展開 + checksum + packaged binary smoke）
- `.github/workflows/ci.yml` (`fresh-clone-artifact`, `test-fresh-clone`, `fresh-clone-smoke`)
- `crates/lsharp-wasm/tests/e2e/selfhost_lsp_docs_ops.rs` (`test_e2e_ops07_fresh_clone_no_rust`)
