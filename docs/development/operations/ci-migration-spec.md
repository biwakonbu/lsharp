# CI 移行仕様 (P11-6 / P11-6a)

## 概要
CI の主経路を Rust (cargo test / clippy / rustfmt) から L# selfhost (stageN.wasm / native) へ移行し、
最終的に Cargo.toml / crates/ を削除して L# のみで bootstrap + native 配布が成立する状態を達成する。

本仕様は以下の 2 グループ計 9 件のタスクをカバーする:
- **P11-6** (CI 切替と Rust 撤去): 5 件 (P11-6-1 ~ P11-6-5)
- **P11-6a** (CI 再編): 4 件 (P11-6a-1 ~ P11-6a-4)

### 前提条件
- bootstrap 固定点 (stage2 == stage3) が CI で安定していること (P11-2d-1)
- native backend が tier1 全プラットフォームで動作すること (P11-2b)
- Wasm/native differential test が pass していること (P11-2d-2)

### 現行 CI 構成 (移行前)
現行の `.github/workflows/ci.yml` は以下の job で構成される:

| ジョブ | 内容 |
|--------|------|
| Test | `cargo test` -- 全テスト実行 |
| Lint (clippy) | `cargo clippy -- -D warnings` |
| Format (rustfmt) | `cargo fmt --check` |
| Bootstrap (selfhost) | selfhost/*.ls, stdlib/*.ls のコンパイル検証 |
| CI Gate | 上記ジョブの成功を集約 (branch protection の required check) |

---

## P11-6-1: CI 主経路切替

### 目的
`cargo test` 中心の CI を `stageN.wasm` / native バイナリ中心の CI に切り替える。

### 移行手順

#### Phase 1: 並走期間 (shadow mode)
1. 新しい L# ベースの job を ci.yml に追加する (既存 job は残す)
2. 新 job は `stageN.wasm` でコンパイル + テスト実行する
3. 新 job の失敗は warning 扱い (CI Gate に含めない)
4. 並走期間は最低 2 週間、全 tier1 で安定するまで延長する

#### Phase 2: 主経路切替
1. 新 job が 2 週間以上安定したことを確認する
2. CI Gate の `needs` を新 job 群に切り替える
3. 旧 job (cargo test / clippy / fmt) を `ci-gate` の必須依存から除外する
4. 旧 job は shadow job として残す (P11-6a-2 参照)

#### Phase 3: 旧 job 撤去
1. shadow job のエラーが 4 週間以上発生していないことを確認する
2. shadow job を ci.yml から削除する
3. docs/development/operations/CI.md を更新する

### 新 CI の主要テスト
| テスト | 実行方法 | 判定基準 |
|--------|----------|----------|
| bootstrap 固定点 | stage1 -> stage2 -> stage3 で byte-identical | stage2 == stage3 |
| unit test | stageN.wasm で selfhost テストスイート実行 | 全 pass |
| golden test | スナップショット比較 | 差分なし |
| e2e test | examples/ の compile + run | 期待出力一致 |
| differential test | Wasm vs native で 5 点比較 | 全一致 (allowlist 除く) |

---

## P11-6-2: bootstrap oracle 隔離

### 目的
Rust 実装 (stage0) を「比較専用のオラクル」として一時的に隔離し、L# selfhost の正しさ検証に活用する。

### 隔離方針

#### oracle job の定義
```yaml
# ci.yml 内
oracle-parity:
  name: Oracle Parity (Rust reference)
  runs-on: ubuntu-latest
  needs: [bootstrap-wasm]
  steps:
    - name: Rust 版 (stage0) でコンパイル
      run: cargo run -- compile selfhost/*.ls -o /tmp/stage0-output/
    - name: L# 版 (stage1) の出力と比較
      run: scripts/ci/compare-oracle.sh /tmp/stage0-output/ /tmp/stage1-output/
```

#### 比較内容
| 比較項目 | 不一致時の扱い |
|----------|---------------|
| 生成 Wasm の export symbol list | warning (Phase 1), fail (Phase 2) |
| compiler diagnostics | warning のみ (メッセージ形式は異なりうる) |
| exit code | fail |

#### oracle 撤去条件
- bootstrap 固定点が 4 週間以上安定
- L# 版のテストカバレッジが Rust 版と同等以上
- P11-6-3 (crates/ 削除) の前提条件が全て満たされている

---

## P11-6-3: Cargo.toml / crates/ 削除

### 前提条件 (全て満たすこと)
1. bootstrap 固定点が 4 週間以上安定
2. oracle parity job で exit code 不一致が 0 件
3. L# selfhost の全テスト (unit / golden / e2e / differential) が pass
4. native backend の tier1 全プラットフォームで release smoke test pass
5. チーム合意 (ADR 作成)

### 段階的削除手順

#### Step 1: 依存排除確認
1. `crates/` 内の各クレートが他ツールから参照されていないことを確認する
2. CI script 内の `cargo` コマンド呼び出しを全て列挙する
3. README.md, book/, docs/ 内の `cargo` 関連記述を列挙する

#### Step 2: CI からの cargo 参照除去
1. ci.yml から全ての `cargo` コマンドを含む step を削除する
2. `dtolnay/rust-toolchain` action を削除する
3. `Swatinem/rust-cache` action を削除する
4. oracle parity job を削除する (shadow job も含む)

#### Step 3: ファイル削除
1. `Cargo.toml` (workspace root) を削除する
2. `Cargo.lock` を削除する
3. `crates/` ディレクトリを削除する
4. `rust-toolchain.toml` が存在する場合は削除する
5. `.cargo/` ディレクトリが存在する場合は削除する

#### Step 4: ドキュメント更新
1. `README.md` のビルド手順を L# ベースに書き換える
2. `docs/development/operations/CI.md` を新 CI 構成に書き換える
3. `CLAUDE.md` のビルド・テストコマンドを更新する
4. `CONTRIBUTING.md` が存在する場合は更新する

#### Step 5: .gitignore 更新
1. `target/` ディレクトリのエントリを削除する
2. L# の build artifact ディレクトリを追加する

---

## P11-6-4: native release CI 組込み

### artifact 生成

#### ビルド matrix
| OS | arch | artifact 名 |
|----|------|-------------|
| macOS | arm64 | lsharp-darwin-arm64 |
| macOS | x86_64 | lsharp-darwin-x64 |
| Linux | x86_64 | lsharp-linux-x64 |

#### ビルド手順
```yaml
release-build:
  name: Release Build (${{ matrix.target }})
  runs-on: ${{ matrix.runner }}
  strategy:
    matrix:
      include:
        - target: darwin-arm64
          runner: macos-latest
        - target: darwin-x64
          runner: macos-13
        - target: linux-x64
          runner: ubuntu-latest
  steps:
    - uses: actions/checkout@v4
    - name: Bootstrap compiler
      run: scripts/ci/bootstrap.sh
    - name: Build native binary
      run: scripts/ci/build-native.sh --target ${{ matrix.target }}
    - name: Smoke test
      run: scripts/ci/release-smoke.sh ./build/lsharp-${{ matrix.target }}
    - uses: actions/upload-artifact@v4
      with:
        name: lsharp-${{ matrix.target }}
        path: build/lsharp-${{ matrix.target }}
```

### 署名
- macOS: `codesign` による ad-hoc 署名 (v1 では Apple Developer ID なし)
- Linux: 署名なし (将来的に GPG 署名を検討)
- 署名検証スクリプト: `scripts/ci/verify-signature.sh`

### 配布
- GitHub Releases に artifact をアップロードする
- tag push (`v*`) をトリガーとする
- SHA-256 チェックサム (`checksums.txt`) を同梱する
- リリースノートは CHANGELOG.md から自動生成する

### 回帰テスト
リリース artifact に対して以下の smoke test を実行する:

| テスト | 内容 | 判定基準 |
|--------|------|----------|
| version | `lsharp --version` | 正常出力 + 正しいバージョン文字列 |
| compile | `lsharp compile examples/fib.ls` | exit code 0 |
| run | `lsharp run examples/fib.ls` | 期待出力一致 |
| selfhost | `lsharp compile selfhost/Main.ls` | exit code 0 |
| determinism | 同一ソースを 2 回コンパイル | 出力ハッシュ一致 |

---

## P11-6-5: 完了条件

### L# のみで bootstrap が成立する条件
以下の全てを満たすとき、P11-6 を完了とする:

1. **Rust コード不要**: リポジトリに `Cargo.toml`, `crates/`, `*.rs` ファイルが存在しない
2. **bootstrap 自立**: stage0 なしで stage1 -> stage2 -> stage3 の固定点が成立する
   - stage1 は前回の release binary (もしくは CI cache) を使用する
   - 新規クローンでも bootstrap 可能な手順が README に記載されている
3. **CI 完全移行**: ci.yml に `cargo` コマンドが存在しない
4. **native 配布成立**: tier1 全プラットフォームの release artifact が CI で自動生成される
5. **テストカバレッジ維持**: Rust 版で実行していたテスト (unit / golden / e2e) の等価なテストが L# 版で全て pass する
6. **ドキュメント整合**: README, docs/development/operations/CI.md, CLAUDE.md が L# ベースの手順に更新されている

### bootstrap 自立の初回手順
初回 (stage0 が存在しない環境) の bootstrap 手順:
1. GitHub Releases から最新の `lsharp-{platform}` をダウンロードする
2. ダウンロードした binary を `stage0` として使用する
3. `stage0 compile selfhost/Main.ls -o stage1.wasm` で stage1 を生成する
4. 以降は通常の 3 段 bootstrap を実行する

---

## P11-6a-1: CI job 再編

### 新 job 構成
現行の 4 job (Test, Lint, Format, Bootstrap) + CI Gate を以下の 6 job + gate に再編する:

| 新 job 名 | 内容 | 実行条件 | 依存 |
|-----------|------|----------|------|
| `bootstrap-wasm` | stage1 -> stage2 -> stage3 固定点検証 (Wasm) | 全 PR, main push | なし |
| `bootstrap-native` | native binary での bootstrap 検証 | 全 PR, main push | `bootstrap-wasm` |
| `golden-parity` | Wasm/native differential test + golden test | 全 PR, main push | `bootstrap-wasm`, `bootstrap-native` |
| `release-smoke` | tier1 全プラットフォームの release smoke test | main push, tag push | `bootstrap-native` |
| `packaging` | release artifact 生成 + checksums | tag push (`v*`) | `release-smoke` |
| `docs` | ドキュメントビルド + リンク検証 | 全 PR, main push | なし |

#### job 依存グラフ
```
bootstrap-wasm ──┬── bootstrap-native ──┬── golden-parity
                 │                      └── release-smoke ── packaging
                 └── docs (独立)

ci-gate: bootstrap-wasm + bootstrap-native + golden-parity + docs
```

#### 各 job の詳細

**bootstrap-wasm**:
- stage0 (前回 release binary or CI cache) で stage1.wasm を生成
- stage1.wasm で stage2.wasm を生成
- stage2.wasm で stage3.wasm を生成
- stage2.wasm == stage3.wasm を検証
- 失敗時: section diff を artifact に保存

**bootstrap-native**:
- stage1.wasm から native binary を生成
- native binary で selfhost をコンパイルし、Wasm 版と出力を比較
- tier1 matrix (macOS arm64, macOS x64, Linux x64) で並列実行

**golden-parity**:
- P11-2d-2 の 7 カテゴリ全てで Wasm/native differential test 実行
- スナップショットテスト (insta 相当) の検証
- allowlist に登録された既知差分は skip

**release-smoke**:
- release build の smoke test (P11-6-4 の回帰テスト参照)
- debug build との比較 (UB 検出)

**packaging**:
- tier1 全プラットフォームの native binary を生成
- SHA-256 checksums.txt を生成
- GitHub Releases にアップロード

**docs**:
- ドキュメントのビルド検証
- 内部リンクの dead link チェック
- CLAUDE.md のコマンド例が実際に動作するかの検証 (optional)

---

## P11-6a-2: shadow job

### 目的
既存の `cargo test` / `cargo clippy` / `cargo fmt` job を即座に削除せず、
legacy reference として一定期間維持し、移行の安全性を担保する。

### shadow job 定義
```yaml
shadow-legacy:
  name: Shadow (Rust legacy)
  runs-on: ubuntu-latest
  # CI Gate に含めない -- 失敗してもマージ可能
  if: github.event_name == 'pull_request'
  continue-on-error: true
  steps:
    - uses: actions/checkout@v4
    - uses: dtolnay/rust-toolchain@stable
    - uses: Swatinem/rust-cache@v2
    - name: cargo test (shadow)
      run: cargo test 2>&1 | tee /tmp/shadow-test.log || true
    - name: cargo clippy (shadow)
      run: cargo clippy -- -D warnings 2>&1 | tee /tmp/shadow-clippy.log || true
    - name: cargo fmt (shadow)
      run: cargo fmt --check 2>&1 | tee /tmp/shadow-fmt.log || true
    - uses: actions/upload-artifact@v4
      with:
        name: shadow-legacy-logs
        path: /tmp/shadow-*.log
        retention-days: 7
```

### 維持方針
| 期間 | 状態 | アクション |
|------|------|-----------|
| 移行後 0-2 週間 | 全 PR で実行 | shadow 結果を監視、不一致があれば調査 |
| 移行後 2-4 週間 | main push のみ実行 | 頻度を下げて継続監視 |
| 移行後 4 週間以降 | 削除可能 | P11-6-3 の一環で削除 |

### 撤去条件
- 4 週間以上 shadow job と新 job の結果に有意な差異がない
- P11-6-3 の前提条件が全て満たされている
- チーム合意

---

## P11-6a-3: branch protection 更新

### 現行設定
- Required status check: `CI Gate` のみ
- `CI Gate` は test / lint / format / bootstrap の成功を集約

### 切替手順

#### Step 1: 新 CI Gate の導入
1. ci.yml に新しい `ci-gate-v2` job を追加する
2. `ci-gate-v2` は新 job 群 (`bootstrap-wasm`, `bootstrap-native`, `golden-parity`, `docs`) の成功を集約する
3. `ci-gate-v2` と旧 `ci-gate` を並走させる

```yaml
ci-gate-v2:
  name: CI Gate v2
  runs-on: ubuntu-latest
  needs: [bootstrap-wasm, bootstrap-native, golden-parity, docs]
  if: always()
  steps:
    - name: 全ジョブの結果を検証
      run: |
        results=("${{ needs.bootstrap-wasm.result }}"
                 "${{ needs.bootstrap-native.result }}"
                 "${{ needs.golden-parity.result }}"
                 "${{ needs.docs.result }}")
        for r in "${results[@]}"; do
          if [[ "$r" != "success" ]]; then
            echo "CI Gate v2 failed: one or more jobs did not succeed"
            exit 1
          fi
        done
        echo "CI Gate v2: All checks passed."
```

#### Step 2: branch protection 切替
1. GitHub Settings > Branches > main の protection rule を編集
2. Required status checks に `CI Gate v2` を追加
3. 1-2 日間は `CI Gate` と `CI Gate v2` の両方を required にする (安全期間)
4. 安全期間後、`CI Gate` (旧) を required から除外する

#### Step 3: 旧 gate 撤去
1. ci.yml から旧 `ci-gate` job を削除する
2. `ci-gate-v2` を `ci-gate` にリネームする (名前の一貫性)
3. branch protection の required check を `ci-gate` に更新する

### ロールバック手順
新 gate で問題が発生した場合:
1. branch protection の required check を旧 `CI Gate` に戻す
2. 新 job 群の問題を調査・修正する
3. 再度切替を試みる

---

## P11-6a-4: CI artifact

### 保存対象

| artifact 名 | 内容 | 生成 job |
|-------------|------|----------|
| `bootstrap-stages` | stage1.wasm, stage2.wasm, stage3.wasm | `bootstrap-wasm` |
| `bootstrap-diff` | 固定点不一致時の section diff | `bootstrap-wasm` (失敗時のみ) |
| `native-binaries` | tier1 全プラットフォームの native binary | `bootstrap-native` |
| `differential-report` | Wasm/native 差分レポート | `golden-parity` |
| `release-artifacts` | リリース用 native binary + checksums.txt | `packaging` |
| `shadow-legacy-logs` | shadow job のログ | `shadow-legacy` |
| `benchmark-results` | 性能ベンチマーク結果 (JSON) | `release-smoke` |

### 保持期間

| artifact カテゴリ | PR | main push | tag push |
|-------------------|-----|-----------|----------|
| bootstrap-stages | 7 日 | 30 日 | 永続 (release) |
| bootstrap-diff | 7 日 | 30 日 | N/A |
| native-binaries | 7 日 | 30 日 | 永続 (release) |
| differential-report | 7 日 | 30 日 | 永続 (release) |
| release-artifacts | N/A | N/A | 永続 (GitHub Releases) |
| shadow-legacy-logs | 7 日 | N/A | N/A |
| benchmark-results | 7 日 | 90 日 | 永続 (release) |

### artifact 設定例
```yaml
- uses: actions/upload-artifact@v4
  with:
    name: bootstrap-stages
    path: |
      build/stage1.wasm
      build/stage2.wasm
      build/stage3.wasm
    retention-days: ${{ github.ref == 'refs/heads/main' && 30 || 7 }}
```

### ストレージ管理
- GitHub Actions の artifact ストレージ上限に注意する (Free plan: 500 MB)
- 大きな artifact (native binary) は圧縮してからアップロードする
- 不要な artifact は retention-days で自動削除する
- release tag の artifact は GitHub Releases に移動し、Actions artifact からは削除する

---

## 関連仕様
- [検証と固定点 仕様 (P11-2d)](../validation/verification-spec.md) -- テスト分類、固定点検証の詳細
- [Native Backend 仕様 (v1)](../../language/native-backend-spec.md) -- native binary の ABI、codegen 仕様
- [CI / ブランチ保護設定](CI.md) -- 現行 CI 構成 (移行前)
