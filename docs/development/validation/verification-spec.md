# 検証と固定点 仕様 (P11-2d)

## 概要
L# selfhost compiler の正しさを保証するための検証戦略。
bootstrap 固定点、**guest Wasm component / host launcher differential test**、テスト行列、性能ゲートの 4 軸で構成する。

---

## P11-2d: 検証と固定点 (トップレベル方針)

### 固定点検証
- `stage1.wasm -> stage2.wasm -> stage3.wasm` の 3 段固定点検証を bootstrap の正本とする
- stage2 と stage3 の出力が bit-identical であることが固定点成立の条件
- 固定点が成立しない場合、そのコミットは CI で reject する

### guest component / host launcher differential test
- `stageN.wasm` と、それを host launcher 経由で実行した配布経路が同じソースに対して同値な観測結果を返すことを検証する
- 観測結果の定義: 終了コード、stdout、生成物ハッシュ、型エラー出力

### 両実行経路比較
- `selfhost/src/**/*.ls`, `stdlib/*.ls`, `examples/` の全ソースに対して guest component 単体出力と host launcher 経由の配布経路を比較する
- 終了コード、stdout、生成物ハッシュ、型エラー出力の 4 点を比較する

### 最適化レベルの固定
- deferred native backend v1 は非最適化 (`-O0` 相当) で固定する
- 性能最適化は固定点と互換性が安定した後に別 Phase で扱う
- `-O0` 固定の理由: 最適化パスによる非決定性を排除し、固定点検証の信頼性を確保するため

---

## P11-2d-1: bootstrap 固定点

### 正本入力集合
固定点検証に使用するソースファイル群を以下に固定する:
- `selfhost/src/**/*.ls` -- selfhost compiler 本体の正本
- `stdlib/*.ls` -- 標準ライブラリ
- `examples/fib.ls` -- 再帰・数値計算の代表例
- `examples/module.ls` -- モジュールシステムの代表例
- `examples/trait.ls` -- トレイトシステムの代表例

entrypoint は `selfhost/src/App/Main.ls` を基準にする。検証入力の selfhost ソースは `selfhost/src/**/*.ls` のみを正本として扱う。

入力集合の変更は ADR を経由し、CI の正本入力リストを一箇所で管理する。

### 3 段比較
```
stage0 (Rust compiler) --[selfhost src]--> stage1.wasm
stage1.wasm            --[selfhost src]--> stage2.wasm
stage2.wasm            --[selfhost src]--> stage3.wasm
```

- stage0 は Rust 実装の L# compiler (crates/ 配下)
- stage1 は stage0 が生成した selfhost compiler の Wasm バイナリ
- stage2 は stage1 が生成した selfhost compiler の Wasm バイナリ
- stage3 は stage2 が生成した selfhost compiler の Wasm バイナリ
- 固定点条件: `stage2.wasm == stage3.wasm` (byte-identical)

### 比較対象 (4 点分離)
固定点不一致時の原因特定を容易にするため、比較を 4 層に分ける:

| 比較層 | 内容 | 不一致時の示唆 |
|--------|------|----------------|
| raw wasm bytes | バイナリ全体の SHA-256 | いずれかの層にズレあり |
| exported symbol list | export section のシンボル一覧 | codegen のシンボル生成に差異 |
| data section bytes | data section のバイト列 | 定数/文字列リテラルの差異 |
| compiler diagnostics | warning/error の出力テキスト | 型推論・パース段階の差異 |

各層を独立に比較し、どの層で最初にズレが生じたかをレポートする。

### 失敗時の diff 保存
- binary diff ではなく section diff と symbol/data diff を保存する
- diff フォーマット: テキストベースで人間可読なもの (wasm-tools dump 形式)
- CI artifact として回収し、PR comment にサマリを貼る
- 保存先: `ci-artifacts/bootstrap-diff/{commit_sha}/`

---

## P11-2d-2: guest component / host launcher differential test

### 観測点 (5 点)
differential test で比較する観測点を以下に固定する:

| 観測点 | 比較方法 |
|--------|----------|
| exit code | 数値一致 |
| stdout | テキスト完全一致 (末尾改行正規化あり) |
| stderr | テキスト完全一致 (末尾改行正規化あり) |
| generated file bytes | SHA-256 ハッシュ一致 |
| diagnostics JSON | JSON deep equal (順序無視) |

### テストカテゴリ (7 種)
比較対象プログラムを以下の 7 カテゴリに分類する:

| カテゴリ | 代表的な入力 | 検証の主眼 |
|----------|-------------|-----------|
| 正常系 | fib.ls, module.ls | 正常コンパイル + 実行結果 |
| parse error | 構文不正の .ls | エラーメッセージの同値性 |
| type error | 型不整合の .ls | 型エラー出力の同値性 |
| module import | 複数ファイルプロジェクト | モジュール解決の一致 |
| file I/O | ファイル読み書きを含む .ls | runtime I/O の同値性 |
| macro expansion | マクロを含む .ls | マクロ展開結果の同値性 |
| formatter/linter | formatter 入力 | 整形結果の同値性 |

### nondeterministic 要素の固定
以下の非決定性要素は test fixture 側で固定し、観測値に混入させない:

- **時計**: `lsharp_clock_now_millis` を固定値 (epoch 0) を返す stub に差し替える
- **一時ファイル**: テスト用の deterministic な tmpdir を使用する
- **絶対パス**: ソースパスを相対パスに正規化してから比較する
- **ハッシュ/乱数**: seed を固定する (該当機能がある場合)

### 既知差分の allowlist
- host-launcher-only または component-only の既知差分が発生する場合、allowlist ファイルで管理する
- allowlist の各エントリに理由と解消条件を記載する
- allowlist に追加する場合は ADR を作成し、TODO に解消タスクを登録する
- allowlist のエントリ数が 10 を超えた場合は技術負債として優先対応する

allowlist フォーマット:
```yaml
# tests/differential-allowlist.yaml
- id: "launcher-stderr-path-format"
  category: "file I/O"
  observation: "stderr"
  reason: "host launcher は OS パス、guest component は WASI パスを出力"
  resolve_condition: "パス正規化レイヤーを統一"
  tracking_issue: null
```

---

## P11-2d-3: テスト行列

### Supported product/release target matrix
supported target は全テストを実行する最優先プラットフォーム:

| OS | arch | 実行内容 |
|----|------|----------|
| macOS | arm64 (Mac Apple Silicon) | bootstrap + host launcher / component differential 全テスト |
| Linux | x86_64 | bootstrap + host launcher / component differential 全テスト |

Mac Apple Silicon または Linux x86_64 で 1 つでも失敗した場合、PR はマージ不可。

### Out of support scope

| OS | arch | 実行内容 |
|----|------|----------|
| macOS | x86_64 (Intel) | out of support scope。Rosetta / Mach-O smoke は internal diagnostic のみ |
| Windows | x86_64 | out of support scope。host launcher / Authenticode は archived design |
| Linux | aarch64 | out of support scope。将来再評価事項 |

- out of support scope の target は product/release gate から外す。再導入する場合は supported product/release target matrix の変更として扱う。

### テスト分類 (5 種)
リポジトリ内のテストを以下の 5 種に分類し、CI job 名もこれに揃える:

| 分類 | CI job 名 | 内容 | 実行タイミング |
|------|-----------|------|---------------|
| unit | `test-unit` | クレート内ユニットテスト | 全 PR |
| golden | `test-golden` | スナップショット/ゴールデンテスト | 全 PR |
| e2e | `test-e2e` | フルパイプライン E2E テスト | 全 PR |
| bootstrap | `test-bootstrap` | 3 段固定点検証 | main merge + release |
| release-smoke | `test-release-smoke` | リリースビルドの smoke test | release branch |

### failure triage
テスト名にコンパイラパイプラインの段階を埋め込み、失敗箇所の特定を容易にする:

- 命名規則: `test_{分類}_{段階}_{内容}`
- 段階一覧: `frontend`, `type`, `ir`, `backend`, `runtime`, `link`, `package`
- 例: `test_e2e_backend_component_fib`, `test_bootstrap_link_stage2_symbols`

---

## P11-2d-4: 性能・回帰ゲート

### ベンチマーク基準点
host launcher + embedded component の release 経路は正しさ優先だが、以下のベンチマークを基準点として保存する:

| ベンチマーク | 入力 | 計測内容 |
|-------------|------|----------|
| fib | examples/fib.ls | 実行時間 (wall clock) |
| selfhost compile | selfhost/src/**/*.ls 全体 | コンパイル時間 |
| LSP initialize | VSCode 拡張初期化 | 応答時間 |
| formatter on stdlib | stdlib/*.ls 全体 | 整形完了時間 |

### メトリクス (3 点)
以下のメトリクスを記録し、回帰検知に使用する:

| メトリクス | 単位 | 回帰閾値 (fail) | 警告閾値 |
|-----------|------|-----------------|---------|
| peak RSS | MiB | +50% | +20% |
| compile latency | ms | +100% | +30% |
| binary size | KiB | +50% | +20% |

- 閾値を超えた場合: fail 閾値は CI fail、警告閾値は PR comment で通知
- ベースラインは main ブランチの最新値を使用し、PR ごとに比較する

### release/debug smoke test
- release build (`--release`) と debug build の両方で smoke test を実行する
- debug ビルドでのみ発生する UB や未定義動作の隠蔽を防ぐため
- smoke test 内容: 全 examples のコンパイル + 実行 + 結果検証

### PGO/LTO の扱い
- PGO (Profile-Guided Optimization) は Phase 11 の gate に含めない
- LTO (Link-Time Optimization) は Phase 11 の gate に含めない
- これらは正しさが固定点で保証された後の別最適化フェーズで扱う
- 最適化フェーズの開始条件: 固定点が 2 週間以上安定していること
