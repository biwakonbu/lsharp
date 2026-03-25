# 最終撤去条件 仕様 (P11-6d)

## 概要

bootstrap oracle (Rust 実装) および legacy reference の完全撤去に向けた最終条件を定義する。
撤去は段階的に行い、build / test / release / editor integration の全経路で
legacy 依存がゼロになった時点をもって完了とする。
本仕様は docs/completion-criteria.md P11-2e-3 (撤去前ゲート) の具体化であり、
P11-6b (legacy reference 隔離) 完了後に適用する。

---

## P11-6d-1: 依存除去チェックリスト

bootstrap oracle / legacy reference への依存が以下の全経路で除去されていることを検証する。

### build 経路

| 検証項目 | 検証方法 | 合格条件 |
|----------|----------|----------|
| `Cargo.toml` workspace が不在 | `test ! -f Cargo.toml` | ファイルが存在しない |
| `crates/` ディレクトリが不在 | `test ! -d crates` | ディレクトリが存在しない |
| Makefile / build script に `cargo` 呼び出しがない | `grep -r 'cargo ' Makefile* scripts/` | ヒット 0 件 |
| bootstrap 手順が L# のみで完結する | `make bootstrap` が Rust toolchain なしで成功 | exit code 0 |
| stage0 生成が事前ビルド済みバイナリまたは L# native で行われる | stage0 の README 記述確認 | Rust compiler への言及なし |

### test 経路

| 検証項目 | 検証方法 | 合格条件 |
|----------|----------|----------|
| `cargo test` を呼び出す CI job がない | `.github/workflows/*.yml` の検索 | ヒット 0 件 |
| `cargo clippy` / `cargo fmt` を呼び出す CI job がない | 同上 | ヒット 0 件 |
| テスト実行が `lsharp test` で完結する | CI ログ確認 | 全テスト job が `lsharp test` ベース |
| E2E テスト数が撤去前と同等以上 | テスト件数カウント比較 | 減少なし |

### release 経路

| 検証項目 | 検証方法 | 合格条件 |
|----------|----------|----------|
| release workflow に `cargo build --release` がない | workflow ファイル検索 | ヒット 0 件 |
| release artifact が native binary のみ | artifact 一覧確認 | Rust 由来バイナリなし |
| checksum / signing が native artifact に対して実行される | release playbook 確認 | 全 artifact に checksum + 署名 |
| changelog に Rust 依存の記述がない | CHANGELOG.md 検索 | 新規エントリに Rust 言及なし |

### editor integration 経路

| 検証項目 | 検証方法 | 合格条件 |
|----------|----------|----------|
| VSCode 拡張が native LSP binary を使用 | extension 設定確認 | `lsharp-lsp` (native) を起動 |
| LSP server の起動に Rust toolchain が不要 | `which rustc` なし環境でテスト | LSP 正常起動 |
| REPL が native binary で動作 | `lsharp repl` の実行確認 | Rust 経由なしで起動 |

---

## P11-6d-2: fresh clone 再現手順

リポジトリを clone した直後から native release 生成までを
bootstrap oracle (Rust 実装) なしで再現できることを検証する。

### 前提条件

- Rust toolchain がインストールされていない環境
- wasmtime がインストール済み (Wasm 実行用)
- 事前ビルド済み stage0 バイナリが release asset として配布済み

### 手順

```bash
# 1. clone
git clone https://github.com/biwakonbu/lsharp.git
cd lsharp

# 2. stage0 の取得
# 事前ビルド済みバイナリを release asset からダウンロード
make fetch-stage0
# -> stage0 バイナリが bin/stage0 に配置される

# 3. bootstrap (stage0 -> stage1 -> stage2)
make bootstrap
# -> stage1.wasm, stage2.wasm が生成される
# -> stage2 と stage3 の fixed-point 検証が実行される

# 4. native release ビルド
make native-release
# -> target/release/lsharp が生成される

# 5. テスト実行
make test
# -> unit, golden, e2e, bootstrap テストが全て実行される

# 6. release artifact 生成
make release-bundle
# -> dist/ に署名済み release artifact が生成される
```

### 検証項目

| ステップ | 期待結果 | 失敗時の対処 |
|----------|----------|-------------|
| fetch-stage0 | stage0 バイナリ取得成功 | release asset の URL / checksum を確認 |
| bootstrap | fixed-point 成立 (stage2 == stage3) | bootstrap oracle の残存依存を調査 |
| native-release | native binary 生成成功 | native backend のリンク設定を確認 |
| test | 全テスト pass | 失敗テストの原因を調査 |
| release-bundle | artifact 生成 + checksum 付与 | release playbook を確認 |

### CI での自動検証

- `test-fresh-clone` job を追加し、上記手順を Docker コンテナ内で自動実行する
- コンテナイメージには Rust toolchain を含めない
- main merge ごとに実行し、bootstrap oracle への暗黙依存の混入を防止する

---

## P11-6d-3: rollback 手順

最終撤去後に致命的な問題が発見された場合、最後の legacy reference リリースへ復帰する手順。

### 前提

- 撤去前に `v0.x.y-rust-final` タグを切っておく (P11-6b-3 で固定)
- legacy reference の最終 commit が tag で参照可能であること
- CI workflow の legacy 版が tag から復元可能であること

### rollback 手順

```bash
# 1. rollback ブランチの作成
git checkout -b rollback/rust-final v0.x.y-rust-final

# 2. legacy CI workflow の復元確認
# v0.x.y-rust-final 時点の .github/workflows/ が有効であることを確認
git diff v0.x.y-rust-final HEAD -- .github/workflows/

# 3. Rust toolchain の復元
# v0.x.y-rust-final の rust-toolchain.toml に記載されたバージョンをインストール
rustup install $(cat rust-toolchain.toml | grep channel | cut -d'"' -f2)

# 4. legacy ビルドの実行
cargo build --release
cargo test

# 5. legacy release の作成 (必要な場合)
# release playbook の legacy 版手順に従う
make legacy-release

# 6. hotfix の適用 (必要な場合)
# rollback ブランチ上で修正を行い、legacy CI で検証
git commit -m "hotfix: ..."
```

### rollback が必要になるシナリオ

| シナリオ | 判断基準 | rollback 範囲 |
|----------|----------|--------------|
| native binary のランタイムクラッシュ | crash report が一定数を超過 | release のみ rollback |
| bootstrap の fixed-point 崩壊 | CI で fixed-point fail が連続 | main ブランチを rollback |
| LSP/editor の重大不具合 | ユーザー報告 + 再現確認 | extension + LSP binary を rollback |
| セキュリティ脆弱性 | CVE 発行 or 脆弱性報告 | 該当コンポーネントを rollback |

### rollback 後の CI 復旧手順

1. rollback ブランチの CI が全件 pass することを確認
2. branch protection の required status を legacy job 群に戻す
3. release workflow を legacy 版に切り替え
4. 影響を受けたユーザーへの通知 (release notes + advisory)
5. 原因調査と修正計画の策定
6. 修正完了後、再度撤去手順を実行

### rollback の判断フロー

```
問題発見
  -> 重大度評価 (critical / major / minor)
  -> critical: 即座に rollback + hotfix
  -> major: 24h 以内に修正可能か判断 -> 不可なら rollback
  -> minor: 次回リリースで修正 (rollback 不要)
```

---

## 補足: 撤去完了の最終確認

全条件を満たした後、以下の最終確認を実施する:

1. **docs/ の整合性**: 全ドキュメントから Rust 実装への参照が撤去 or 歴史的記述に変更されている
2. **CI の安定性**: 撤去後 2 週間以上 CI が安定している (P11-2e-3 ゲート 1 準拠)
3. **ADR の記録**: 撤去の経緯、判断、rollback 手順が ADR として記録されている
4. **tag の固定**: `v0.x.y-rust-final` tag が保護され、削除不可に設定されている
