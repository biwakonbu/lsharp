# リリース運用 仕様 (P11-6c)

## 概要

L# ネイティブ配布物のリリース運用方針を定める。
semver に基づくバージョニング、nightly/stable の 2 チャネル運用、
障害対応手順、互換表生成の 4 軸で構成する。

本仕様は以下に依存する:

- **P11-4** ツールチェイン parity (docs/development/planning/toolchain-parity-spec.md)
- **P11-5** 互換マトリクス (docs/development/planning/compatibility-matrix.md)

---

## P11-6c: リリース運用 (トップレベル方針)

### リリースの原則

- L# ネイティブバイナリを唯一の公式配布物とする (Rust バイナリは配布しない)
- semver に厳密に従い、破壊的変更は major バージョンでのみ許可する
- 全配布物に checksum と署名を付与し、改竄検知を可能にする
- nightly と stable の 2 チャネルで安定性と開発速度を両立する

### 配布対象プラットフォーム

| OS | arch | 形式 | tier |
|----|------|------|------|
| macOS | arm64 (Apple Silicon) | .tar.gz (署名/公証付き) | tier1 |
| macOS | x86_64 (Intel) | .tar.gz (署名/公証付き) | tier1 |
| Linux | x86_64 | .tar.gz | tier1 |
| Linux | aarch64 | .tar.gz | tier2 |
| Windows | x86_64 | .zip + .exe | tier2 |

---

## P11-6c-1: release playbook

### semver ルール

L# は Semantic Versioning 2.0.0 に従う:

| バージョン要素 | 変更条件 |
|---------------|---------|
| **major** (X.0.0) | 後方互換性のない言語仕様変更、CLI 引数の破壊的変更、LSP レスポンス schema の非互換変更 |
| **minor** (0.Y.0) | 新機能追加、新サブコマンド追加、新 lint rule 追加、後方互換な仕様拡張 |
| **patch** (0.0.Z) | バグ修正、ドキュメント修正、性能改善 (挙動変更なし)、セキュリティ修正 |

### pre-release バージョン

- alpha: `X.Y.Z-alpha.N` -- 機能開発中、API 不安定
- beta: `X.Y.Z-beta.N` -- 機能フリーズ、バグ修正のみ
- rc: `X.Y.Z-rc.N` -- リリース候補、重大バグ修正のみ

### artifact 命名規則

**命名パターン**: `lsharp-{version}-{os}-{arch}.{ext}`

| 例 | 対象 |
|----|------|
| `lsharp-1.0.0-macos-arm64.tar.gz` | macOS Apple Silicon |
| `lsharp-1.0.0-macos-x86_64.tar.gz` | macOS Intel |
| `lsharp-1.0.0-linux-x86_64.tar.gz` | Linux x86_64 |
| `lsharp-1.0.0-linux-aarch64.tar.gz` | Linux aarch64 |
| `lsharp-1.0.0-windows-x86_64.zip` | Windows x86_64 |

**アーカイブ内部構成**:
```
lsharp-{version}-{os}-{arch}/
  bin/
    lsharp          -- CLI バイナリ
    lsharp-lsp      -- LSP バイナリ
  LICENSE
  README.md
  checksums.txt
  CHANGELOG.md      -- このバージョンの変更点 (抜粋)
```

### checksum

- 全 artifact に対して SHA-256 checksum を生成する
- checksum ファイル名: `checksums-{version}.txt`
- checksum フォーマット: `{sha256}  {filename}` (GNU coreutils 互換)
- checksum ファイル自体も GitHub Release の asset として公開する

**検証コマンド例**:
```bash
# macOS / Linux
sha256sum -c checksums-1.0.0.txt

# macOS (BSD shasum)
shasum -a 256 -c checksums-1.0.0.txt
```

### changelog

- CHANGELOG.md は Keep a Changelog 1.1.0 形式に従う
- セクション: Added, Changed, Deprecated, Removed, Fixed, Security
- 各エントリに対応する PR 番号または commit hash を記載する
- 破壊的変更には `BREAKING:` プレフィックスを付与する

**changelog 生成の自動化**:
- `scripts/generate-changelog.sh` で git log から draft を生成する
- draft はリリースマネージャが手動で編集・承認する
- 承認後 `CHANGELOG.md` にマージし、tag を打つ

### 署名

- macOS: Apple Developer ID による codesign + notarization
- Linux: GPG 署名 (detached signature `.sig` ファイル)
- Windows: 署名なし (v1 スコープ外、v2 で Authenticode 対応予定)

**署名検証手順**:
```bash
# macOS (codesign)
codesign --verify --deep --strict lsharp

# Linux (GPG)
gpg --verify lsharp-1.0.0-linux-x86_64.tar.gz.sig lsharp-1.0.0-linux-x86_64.tar.gz
```

### release playbook (手順書)

リリース作業は以下の手順で実施する:

```
1. リリースブランチ作成
   git checkout -b release/v{version} main

2. バージョン番号更新
   - selfhost/Version.ls の VERSION 定数を更新
   - CHANGELOG.md に release date を記入

3. CI テスト実行
   - test-unit, test-golden, test-e2e, test-bootstrap 全 PASS を確認
   - test-release-smoke (全 tier のビルド + smoke test) を実行

4. artifact 生成
   - CI が全プラットフォーム向けバイナリをビルド
   - checksum 生成
   - 署名実施

5. リリース候補の検証
   - 配布アーカイブの展開テスト
   - Quick Start の完走テスト
   - VSCode 拡張の動作確認

6. tag 発行
   git tag -a v{version} -m "Release v{version}"

7. GitHub Release 公開
   - artifact + checksum + changelog をアップロード
   - release notes を記入

8. post-release
   - main ブランチに release ブランチをマージ
   - CHANGELOG.md の Unreleased セクションをリセット
   - 互換表 (../planning/compatibility-matrix.md) を更新
```

---

## P11-6c-2: チャネル管理

### 2 チャネル運用

| チャネル | 目的 | 更新頻度 | 品質保証レベル |
|---------|------|---------|--------------|
| **stable** | 一般ユーザー向け安定版 | 4-8 週間ごと | 全テスト PASS + 1 週間の安定期間 |
| **nightly** | 開発者・早期採用者向け | 毎日 (main ブランチの最新) | CI テスト PASS のみ |

### stable チャネル

**リリース条件**:
- main ブランチの CI が全 green
- bootstrap 固定点が成立 (stage2 == stage3)
- 全 tier1 プラットフォームで smoke test PASS
- 重大バグ (P0/P1) が未解決でないこと
- リリースブランチで 1 週間以上の安定期間を経過

**サポートポリシー**:
- 最新 stable + 1 つ前の stable をサポート対象とする
- セキュリティ修正は 2 世代前まで backport する
- EOL (End of Life) はリリースノートで告知する

### nightly チャネル

**生成条件**:
- main ブランチへの merge が成功するたびに自動ビルド
- CI の `test-unit`, `test-golden`, `test-e2e` が全 PASS
- ビルド失敗時は前回の成功ビルドを維持 (nightly は常に利用可能な状態を保つ)

**命名規則**: `lsharp-nightly-{YYYYMMDD}-{os}-{arch}.{ext}`

**保持期間**: 過去 30 日分の nightly ビルドを保持し、古いものは自動削除する

**免責事項**:
- nightly は API の安定性を保証しない
- nightly 間の互換性は保証しない
- nightly のバグ報告は歓迎するが、修正の優先度は stable より低い

### チャネル切替え

ユーザーは以下の方法でチャネルを切替える:

```bash
# stable (デフォルト)
lsharp --version
# => lsharp 1.0.0

# nightly
lsharp +nightly --version
# => lsharp 1.1.0-nightly.20260325

# 特定 nightly
lsharp +nightly-20260320 --version
```

### hotfix フロー

stable リリース後に重大バグが発見された場合:

```
1. main で修正コミット作成
2. release/v{version} ブランチに cherry-pick
3. patch バージョンを bump (v1.0.0 -> v1.0.1)
4. 通常の release playbook を実行 (安定期間は短縮可、最低 2 日)
5. GitHub Release に hotfix であることを明記
```

---

## P11-6c-3: 障害対応

### crash report 収集

**自動 crash report**:
- L# バイナリが panic / segfault した場合、crash report を生成する
- crash report はローカルファイルに保存される (自動送信しない)
- 保存先: `$XDG_STATE_HOME/lsharp/crash-reports/` (Linux/macOS)
  または `%LOCALAPPDATA%\lsharp\crash-reports\` (Windows)

**crash report の内容**:

| フィールド | 内容 |
|-----------|------|
| version | L# バージョン (semver + commit hash) |
| os | OS 名とバージョン |
| arch | CPU アーキテクチャ |
| command | 実行されたサブコマンドと引数 (ファイルパスはハッシュ化) |
| backtrace | スタックトレース (シンボル付き) |
| timestamp | ISO 8601 形式のタイムスタンプ |
| exit_signal | シグナル番号 (SIGSEGV, SIGABRT 等) |

**プライバシー保護**:
- ソースコードの内容は crash report に含めない
- ファイルパスはハッシュ化して記録する
- crash report の送信はユーザーの明示的な opt-in が必要

### diagnostic dump

ユーザーがバグ報告時に添付する diagnostic dump を生成するコマンド:

```bash
lsharp diagnostic-dump --output dump.json
```

**dump の内容**:

| セクション | 内容 |
|-----------|------|
| environment | OS, arch, L# version, 利用可能なツール |
| config | 設定ファイルの内容 (パスはマスク) |
| recent_crashes | 直近 5 件の crash report サマリ |
| installed_extensions | VSCode 拡張のバージョン |
| build_info | コンパイラのビルド情報 (commit hash, build date) |

### 障害解析手段

**ログレベル**:

| レベル | 環境変数 | 用途 |
|-------|---------|------|
| error | `LSHARP_LOG=error` | エラーのみ (デフォルト) |
| warn | `LSHARP_LOG=warn` | 警告以上 |
| info | `LSHARP_LOG=info` | 一般的な情報 |
| debug | `LSHARP_LOG=debug` | デバッグ情報 (パイプライン各段階の入出力サマリ) |
| trace | `LSHARP_LOG=trace` | 全詳細 (型推論の各ステップ、IR 命令の生成過程) |

**デバッグ用サブコマンド**:

| コマンド | 出力 |
|---------|------|
| `lsharp parse --ast --json` | AST の JSON 表現 |
| `lsharp check --verbose` | 型推論の詳細ログ |
| `lsharp compile --emit-ir` | lowered IR の dump |
| `lsharp compile --emit-wasm-text` | Wasm テキスト形式 (WAT) |

### 障害分類と対応 SLA

| 優先度 | 定義 | 対応目標 |
|--------|------|---------|
| P0 (critical) | コンパイラ crash、データ損失、セキュリティ脆弱性 | 24 時間以内に hotfix |
| P1 (high) | 誤コンパイル (wrong code)、LSP crash | 1 週間以内に修正 |
| P2 (medium) | 診断メッセージの誤り、性能退行 | 次回 stable リリースまでに修正 |
| P3 (low) | ドキュメント誤り、UI の軽微な問題 | best-effort |

### インシデント対応フロー

```
1. 報告受付
   - GitHub Issues で受付
   - P0/P1 は即座にトリアージ

2. 再現確認
   - diagnostic dump を収集
   - 最小再現ケースを作成

3. 原因特定
   - crash report のスタックトレースを解析
   - debug/trace ログで原因箇所を特定

4. 修正
   - テストを先に追加 (TDD)
   - 修正コミット作成

5. リリース
   - P0: hotfix リリース
   - P1: hotfix または次回 stable
   - P2/P3: 次回 stable
```

---

## P11-6c-4: 互換表生成

### リリースごとの互換表

各 stable リリースで以下の互換表を生成し、ドキュメントに含める。

**CLI 互換表**:

| CLI version | サブコマンド | 追加/変更/廃止 | 備考 |
|-------------|------------|--------------|------|
| v1.0.0 | parse, check, compile, build, test, fmt, doc, lsp, review, doc-ack, doc-check, install, repl | 初回リリース | - |
| v1.1.0 | + lint | 追加 | 新サブコマンド |
| v2.0.0 | - install (deprecated) | 廃止 | パッケージマネージャに移行 |

**LSP 互換表**:

| LSP version | 対応メソッド | LSP spec version | 備考 |
|-------------|------------|-----------------|------|
| v1.0.0 | initialize, shutdown, didOpen, didChange, hover, definition, references, rename, formatting, completion | LSP 3.17 | full sync のみ |
| v1.1.0 | + codeAction, codeLens | LSP 3.17 | - |

**VSCode 拡張互換表**:

| 拡張 version | 対応 LSP version | 対応 VSCode version | 備考 |
|-------------|-----------------|-------------------|------|
| v1.0.0 | v1.0.0 | 1.80+ | - |
| v1.1.0 | v1.0.0 - v1.1.0 | 1.80+ | LSP 後方互換 |

### 互換表の生成方法

**自動生成**:
- `scripts/generate-compat-table.sh` でリリース tag から互換表を生成する
- CLI: 各 tag で `lsharp --help` を実行し、サブコマンド一覧を抽出
- LSP: initialize レスポンスの capabilities から対応メソッドを抽出
- VSCode: `package.json` の `engines.vscode` から対応バージョンを抽出

**手動補完**:
- 破壊的変更の説明は手動で記入する
- 移行ガイドへのリンクを追加する

### 互換表の配置

| 配置先 | 内容 |
|--------|------|
| `docs/development/planning/compatibility-matrix.md` | 最新版の互換マトリクス (開発用) |
| `docs/releases/v{version}-compat.md` | リリース固有の互換表 |
| GitHub Release notes | 互換表のサマリ (主要変更のみ) |
| `book/appendix/compatibility.md` | ユーザー向け互換表 (全履歴) |

### 互換性ポリシー

**後方互換性の保証**:
- minor バージョン間: CLI の引数・出力形式は後方互換
- major バージョン間: 破壊的変更あり (移行ガイド提供)
- LSP: minor バージョン間でレスポンス schema の後方互換を保証

**廃止 (deprecation) ポリシー**:
- 機能廃止は 1 minor バージョン前に `deprecated` 警告を出力する
- 廃止警告は `--no-deprecation-warnings` で抑制可能
- 実際の除去は次の major バージョンで実施する
- 廃止予定の機能は CHANGELOG と互換表に明記する

### CI での互換表検証

- リリース CI で互換表の自動生成と diff 検出を実行する
- 手動記入の互換表と自動生成結果に差分がある場合は warning を出力する
- 互換表が未更新のままリリースされることを防ぐため、CI gate に含める

---

## 依存関係

| 依存先 | 前提条件 | 理由 |
|--------|----------|------|
| **P11-4** toolchain parity | ネイティブ配布物が生成可能 | リリース対象物の前提 |
| **P11-4e** 配布パッケージング | OS 別配布形式が固定 | artifact 命名と形式の基盤 |
| **P11-2d** 検証と固定点 | bootstrap 固定点が成立 | stable リリースの品質保証 |
| **P11-6b** legacy 隔離 | Rust 実装が隔離済み | L# のみの配布を実現 |

---

## リスクと制約

| リスク | 影響 | 緩和策 |
|--------|------|--------|
| Apple 公証の審査遅延 | macOS リリースの遅延 | 公証をパイプライン早期に配置、審査失敗時の再提出手順を整備 |
| nightly ビルドの不安定 | 早期採用者の信頼低下 | ビルド失敗時は前回成功版を維持、nightly 品質ダッシュボードを提供 |
| hotfix の品質リスク | 安定期間短縮による退行 | hotfix でも最低 2 日の安定期間を確保、影響範囲を限定した回帰テスト |
| 互換表の更新漏れ | ユーザーの混乱 | CI gate で互換表更新を強制 |
| crash report のプライバシー | ユーザー情報の漏洩 | opt-in 方式、ソースコード非収集、パスのハッシュ化 |

---

## 用語定義

| 用語 | 定義 |
|------|------|
| **semver** | Semantic Versioning 2.0.0。`MAJOR.MINOR.PATCH` 形式のバージョニング規則 |
| **stable** | 一般ユーザー向けの安定版リリースチャネル |
| **nightly** | main ブランチの最新ビルドを毎日提供する開発者向けチャネル |
| **hotfix** | stable リリース後に重大バグを修正するための緊急パッチリリース |
| **artifact** | CI が生成するリリース用のビルド成果物 (バイナリアーカイブ) |
| **checksum** | ファイルの完全性を検証するための SHA-256 ハッシュ値 |
| **公証 (notarization)** | Apple のサービスによるマルウェアスキャンと承認プロセス |
| **codesign** | Apple Developer ID によるバイナリ署名 |
| **diagnostic dump** | バグ報告時に添付する環境・設定・クラッシュ情報の集約ファイル |
| **crash report** | バイナリ異常終了時に自動生成される障害情報ファイル |
| **tier1** | 全テストを実行し、リリースをブロックする最優先プラットフォーム |
| **tier2** | 段階的にカバレッジを拡大するプラットフォーム。失敗は warning 扱い |
| **deprecation** | 機能の廃止予告。次の major バージョンで実際に除去される |
| **EOL** | End of Life。サポート終了を意味する |
| **release playbook** | リリース作業の手順書 |
| **互換表** | リリースごとの CLI/LSP/VSCode 拡張の互換性を記録した表 |
