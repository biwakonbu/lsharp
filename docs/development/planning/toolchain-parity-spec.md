# P11-4 ツールチェイン parity 仕様書

**最終更新**: 2026-03-30

---

## 概要

L# 製ツールチェイン (CLI, LSP, formatter, linter, docs) を正式化し、エンドユーザーが Rust 開発環境や外部 Wasm ランタイムの事前準備なしに single binary (host launcher + embedded guest component) で開発フローを完走できるようにする。

本仕様は以下に依存する:

- **P11-1** 言語機能 parity
- **P11-2** stdlib parity
- **P11-3** compiler parity

これらが満たされた上で、ツールチェイン全体を L# guest component として供給し、Wasmtime embedding を行う host launcher から利用する自己完結した開発体験を提供する。

---

## P11-4 本体

### T4-1: L# 製 CLI の正式化

L# 製 CLI を正式化し、現行サブコマンド互換の引数仕様と終了コードを固定する。

**受入基準:**

- AC-001: 現行 Rust CLI の全サブコマンドが L# CLI で同一の引数シグネチャで呼び出せる
- AC-002: 終了コード体系 (0=成功, 1=コンパイルエラー, 2=ランタイムエラー, 127=不明コマンド) が固定され、ドキュメント化されている
- AC-003: `--help`, `--version` の出力形式が Rust 版と互換である
- AC-004: stdin/stdout/stderr の使い分けが Rust 版と一致する

### T4-2: L# 製 LSP の正式化

L# 製 LSP を正式化し、以下のメソッドを実装する: `initialize`, `didOpen`, `didChange`, `hover`, `definition`, `references`, `rename`, `formatting`, `completion`, `shutdown`。

**受入基準:**

- AC-005: 上記 10 メソッドが LSP 3.17 仕様に準拠したリクエスト/レスポンスを返す
- AC-006: VSCode 拡張から L# LSP バイナリを spawn して正常に通信できる
- AC-007: Rust LSP と同一の入力に対し、同等のレスポンスを返す (JSON schema 互換)
- AC-008: `shutdown` 後のリクエストに対し適切なエラーレスポンスを返す

### T4-3: L# 製 formatter/linter の AST 全体対応

L# 製 formatter/linter を AST 全体対応に拡張し、CLI と LSP の両経路で同一結果を返す。

**受入基準:**

- AC-009: formatter が全 AST ノード型 (Expr, Decl, Pattern, Literal, Metadata 含む) を処理できる
- AC-010: `lsharp fmt` (CLI) と LSP の `textDocument/formatting` が同一入力に対し同一出力を返す
- AC-011: linter が全 AST ノード型を走査でき、CLI と LSP で同一の診断結果を返す
- AC-012: formatter/linter の処理順序が決定的 (deterministic) である

### T4-4: docs/review/knowledge 等の L# 移植

docs/review/knowledge/doc-check/doc-ack/install/repl を L# 側へ移植し、VSCode 拡張のバックエンドを Rust LSP から host launcher 同梱の L# 実装へ切り替える。

**受入基準:**

- AC-013: `lsharp doc`, `lsharp review`, `lsharp doc-check`, `lsharp doc-ack`, `lsharp install`, `lsharp repl` が L# 実装で動作する
- AC-014: knowledge JSON の schema が Rust 版と互換である
- AC-015: VSCode 拡張が host launcher 同梱の L# バックエンドを使用する
- AC-016: Rust LSP への依存が拡張から除去されている

### T4-5: single binary 配布形式の固定

macOS/Linux/Windows 向けの host launcher + embedded guest component 配布形式、クロスビルド手順、署名/パッケージング方針を固定する。

**受入基準:**

- AC-017: macOS 向け .tar.gz (署名/公証付き)、Linux 向け .tar.gz、Windows 向け .zip + .exe の host launcher 配布物が CI で生成される
- AC-018: クロスビルド手順がドキュメント化され、CI で再現可能である
- AC-019: 全配布物に checksums.txt が同梱される
- AC-020: 署名検証手順がドキュメント化されている

### T4-6: 完了条件

エンドユーザーが Rust 開発環境にも外部 Wasm ランタイムにも触れず、single binary 配布物だけで開発フローを完走できる。

**受入基準:**

- AC-021: Rust/wasmtime/clang 未インストール環境で配布アーカイブを展開し、`lsharp compile` → `lsharp test` → `lsharp doc` が成功する
- AC-022: VSCode 拡張が同梱 host launcher バイナリだけで全機能を提供する
- AC-023: README の Quick Start が single binary 配布物のみで完走する

---

## P11-4a CLI parity

### T4a-1: サブコマンド引数・入出力・終了コードの仕様化

`parse`, `check`, `compile`, `build`, `test`, `review`, `doc-ack`, `doc-check`, `install`, `repl`, `lsp`, `fmt`, `doc` の引数、標準入出力、終了コードを仕様化する。

**受入基準:**

- AC-100: 各サブコマンドの引数仕様 (必須/オプション/デフォルト値) がテーブル形式でドキュメント化されている
- AC-101: stdout にはプログラム出力のみ、stderr には診断メッセージのみが出力される
- AC-102: 終了コード表が全サブコマンドに対して定義されている
- AC-103: `--output` / `-o` フラグの挙動が全生成系コマンドで統一されている

### T4a-2: help/version 出力の互換性

help/version 出力も互換対象に含め、ドキュメント例が壊れないようにする。

**受入基準:**

- AC-104: `lsharp --help` の出力がドキュメント記載の例と一致する
- AC-105: `lsharp --version` の出力形式が `lsharp x.y.z` に固定されている
- AC-106: 各サブコマンドの `--help` がサブコマンド固有のオプションを表示する
- AC-107: help テキストのスナップショットテストが CI に含まれている

### T4a-3: OS 依存パス吸収

config/lockfile/project init/install は OS 依存 path を吸収した共通 service 経由で実装する。

**受入基準:**

- AC-108: `$XDG_CONFIG_HOME` (Linux), `~/Library/Application Support` (macOS), `%APPDATA%` (Windows) を共通 API で吸収する
- AC-109: lockfile のパス解決が OS に依存しない正規化済みパスで行われる
- AC-110: `lsharp init` が OS 固有のテンプレートを正しく生成する
- AC-111: パス解決ロジックのユニットテストが 3 OS 分存在する

### T4a-4: CLI smoke test

CLI smoke test を配布アーカイブ展開後に実行する。

**受入基準:**

- AC-112: 配布アーカイブ展開後に `lsharp --version`, `lsharp --help`, `lsharp check examples/hello.ls` が成功する
- AC-113: smoke test スクリプトが配布アーカイブに同梱されている
- AC-114: CI のリリースパイプラインで smoke test が gate として実行される
- AC-115: smoke test の失敗がリリースをブロックする

---

## P11-4b LSP parity

### T4b-1: document sync 方式

document sync は v1 では full sync に固定し、incremental sync は後段最適化として分離する。

**受入基準:**

- AC-200: `TextDocumentSyncKind.Full` を initialize レスポンスで返す
- AC-201: `didChange` で受け取った全文をパースし、診断を返す
- AC-202: incremental sync は v1 スコープ外であることがドキュメントに明記されている
- AC-203: full sync でのレイテンシが 1000 行ファイルで 500ms 以内である

### T4b-2: LSP メソッドのレスポンス互換

hover/definition/references/rename/formatting/completion のレスポンス形を Rust 実装と同じ JSON schema に揃える。

**受入基準:**

- AC-204: 各メソッドのレスポンス JSON が Rust LSP と同一の schema に準拠する
- AC-205: hover のマークダウン形式が Rust 版と一致する
- AC-206: definition/references が返す Location の uri/range が同一入力で一致する
- AC-207: completion の item 一覧が Rust 版と同一のソート順・フィルタ結果を返す

### T4b-3: 診断の安定順序

診断は parse/type/lint を source ごとに安定順で返し、重複診断のマージ規則を固定する。

**受入基準:**

- AC-208: 診断は `source` フィールド (`parse`, `type`, `lint`) でグルーピングされ、各グループ内で行番号昇順に返される
- AC-209: 同一 span に対する重複診断は severity の高い方のみ残す
- AC-210: 診断の `code` フィールドが安定した識別子 (例: `E0001`, `L0001`) を持つ
- AC-211: 同一ファイルに対して再パースしても診断の順序・内容が変化しない

### T4b-4: VSCode 拡張の spawn 方式

VSCode 拡張は host launcher 形式の LSP バイナリを spawn する方式に固定し、Node 側で解析ロジックを持たない。

**受入基準:**

- AC-212: 拡張は `lsharp-lsp` バイナリを `child_process.spawn` で起動する
- AC-213: Node 側に parse/type-check/lint のロジックが一切存在しない
- AC-214: LSP バイナリが見つからない場合に明確なエラーメッセージを表示する
- AC-215: 拡張設定で LSP バイナリパスをオーバーライドできる

---

## P11-4c formatter/linter parity

### T4c-1: formatter の roundtrip と idempotency

formatter は parse-format-parse roundtrip と idempotency を gate にする。

**受入基準:**

- AC-300: 任意の valid L# ソースに対し `parse(format(parse(src)))` が `parse(src)` と同一 AST を返す
- AC-301: `format(format(src)) == format(src)` が全テストケースで成立する
- AC-302: roundtrip テストが CI gate に含まれている
- AC-303: 空白・改行・コメントの保持規則がドキュメント化されている

### T4c-2: linter 出力の安定化

linter は rule id, severity, span, message code を安定化し、LSP/CLI で同一出力にする。

**受入基準:**

- AC-304: 各 lint rule に一意の rule id (例: `L0001`) が付与されている
- AC-305: severity は `error`, `warning`, `info`, `hint` の 4 段階に固定されている
- AC-306: CLI の `lsharp lint` 出力と LSP の診断が同一の rule id/severity/span/message を含む
- AC-307: rule id と severity のマッピングテーブルがドキュメント化されている

### T4c-3: custom rule API のスコープ

custom rule API は AST walker 完全化後に公開し、v1 では builtin rule のみ正式サポートとする。

**受入基準:**

- AC-308: v1 では builtin rule のみが有効であり、custom rule の設定はエラーになる
- AC-309: AST walker の公開 API 設計が RFC としてドキュメント化されている
- AC-310: builtin rule の一覧がドキュメント化され、各 rule にテストケースが存在する
- AC-311: custom rule API は v2 ロードマップに記載されている

### T4c-4: formatter/linter 設定ファイル仕様

formatter/linter の設定ファイル仕様を決め、未対応項目は明示的に無視ではなくエラーにする。

**受入基準:**

- AC-312: 設定ファイル形式 (TOML) と schema がドキュメント化されている
- AC-313: 未知のキーが設定ファイルに含まれる場合、エラーメッセージと共に終了コード 1 を返す
- AC-314: 設定ファイルが存在しない場合のデフォルト値がドキュメント化されている
- AC-315: 設定ファイルの JSON Schema が配布物に同梱される

---

## P11-4d docs/review/knowledge

### T4d-1: schema の固定と CI snapshot 化

knowledge JSON, review output, doc generator の schema を固定し、CI で snapshot 化する。

**受入基準:**

- AC-400: knowledge JSON の JSON Schema が `docs/schemas/` に配置されている
- AC-401: review output の JSON Schema が同ディレクトリに配置されている
- AC-402: doc generator の出力 schema が固定されている
- AC-403: 各 schema に対する snapshot テストが CI に含まれている

### T4d-2: doc-ack/doc-check trailer 仕様の維持

doc-ack/doc-check の trailer 仕様を host launcher CLI でも維持する。

**受入基準:**

- AC-404: `doc-ack` が生成する trailer コメントが Rust 版と同一の形式である
- AC-405: `doc-check` が trailer の存在・形式を検証し、不正な場合にエラーを返す
- AC-406: trailer 仕様がドキュメント化されている
- AC-407: trailer 操作のユニットテストが存在する

### T4d-3: HTML doc 生成の deterministic 出力

HTML doc 生成は deterministic 出力にし、タイムスタンプや環境依存パスを埋め込まない。

**受入基準:**

- AC-408: 同一入力に対し `lsharp doc` を 2 回実行して diff が空になる
- AC-409: 生成 HTML にタイムスタンプ、ホスト名、絶対パスが含まれない
- AC-410: CI で deterministic 出力の回帰テストが実行されている
- AC-411: ビルド環境に依存するメタデータはオプトイン方式のみで埋め込み可能である

### T4d-4: docs 系の service 分離

docs 系は compiler core から切り離し、library 的に再利用できる service として実装する。

**受入基準:**

- AC-412: docs 系モジュールが compiler core (parser/type-checker/codegen) に直接依存しない
- AC-413: docs service が独立した API として呼び出し可能である
- AC-414: docs service の依存グラフに compiler core が含まれない
- AC-415: docs service の公開 API がドキュメント化されている

---

## P11-4e 配布パッケージング

### T4e-1: OS 別配布形式の固定

macOS は .tar.gz + 署名/公証、Linux は .tar.gz、Windows は .zip + .exe を v1 配布形に固定する。

**受入基準:**

- AC-500: macOS 配布物が .tar.gz 形式であり、Apple Developer ID で署名・公証されている
- AC-501: Linux 配布物が .tar.gz 形式であり、x86_64 と aarch64 の 2 アーキテクチャを提供する
- AC-502: Windows 配布物が .zip 形式であり、.exe バイナリを含む
- AC-503: 各配布物のファイル構成がドキュメント化されている

### T4e-2: release artifact の同梱物

release artifact には host launcher としての `lsharp`, `lsharp-lsp`, `README.md`, `LICENSE`, `checksums.txt` を同梱する。

**受入基準:**

- AC-504: 全配布アーカイブに `lsharp`, `lsharp-lsp`, `README.md`, `LICENSE`, `checksums.txt` が含まれる
- AC-505: `checksums.txt` が SHA-256 ハッシュを含む
- AC-506: `checksums.txt` の検証スクリプトが README に記載されている
- AC-507: CI が artifact 同梱物の完全性を検証する

### T4e-3: パッケージマネージャ対応

Homebrew/apt/scoop 等のパッケージマネージャ対応は v1 では任意、公式配布アーカイブを正本にする。

**受入基準:**

- AC-508: v1 では公式配布アーカイブが唯一の正式配布チャネルである
- AC-509: Homebrew formula のテンプレートがリポジトリに存在する (best-effort)
- AC-510: パッケージマネージャ対応は v2 ロードマップに記載されている
- AC-511: サードパーティパッケージとの差異についてドキュメントで注意喚起されている

### T4e-4: VSCode 拡張の LSP バイナリ探索

VSCode 拡張は同梱 host launcher LSP を優先し、PATH 探索は fallback に限定する。

**受入基準:**

- AC-512: 拡張は `globalStoragePath` 内の同梱バイナリを最優先で探索する
- AC-513: 同梱バイナリが見つからない場合のみ `$PATH` から `lsharp-lsp` を探索する
- AC-514: fallback が発生した場合、ユーザーに通知メッセージを表示する
- AC-515: 探索優先順位がドキュメント化されている

---

## P11-4f 完了条件

### T4f-1: 事前知識なしでの起動

新規ユーザーが Rust/wasmtime/clang の事前知識なしで CLI と VSCode を起動できる。

**受入基準:**

- AC-600: Rust/wasmtime/clang 未インストール環境で `lsharp --version` が成功する
- AC-601: VSCode に L# 拡張をインストール後、`.ls` ファイルを開いてシンタックスハイライトと診断が表示される
- AC-602: セットアップに必要な手順が 5 ステップ以内である

### T4f-2: 同一 artifact からの供給

全主要ツールが同一 single-binary release artifact 群から供給される。

**受入基準:**

- AC-603: `lsharp`, `lsharp-lsp`, formatter, linter が同一アーカイブに含まれる
- AC-604: 追加ダウンロードなしで全ツールが利用可能である
- AC-605: バージョン番号が全バイナリで一致する

### T4f-3: Quick Start の完走

README の Quick Start が single binary 配布物だけで完走できる。

**受入基準:**

- AC-606: Quick Start の全ステップが配布アーカイブの展開とパス設定のみで完了する
- AC-607: Quick Start に Rust/wasmtime/clang のインストール手順が含まれない
- AC-608: CI で Quick Start の自動実行テストが通過する

---

## CLI コマンド入出力契約テーブル (AC-100/AC-101/AC-102)

### サブコマンド引数・入出力・終了コード一覧

| サブコマンド | 必須引数 | オプション引数 | stdout | stderr | Exit Code (成功) | Exit Code (エラー) |
|-------------|---------|---------------|--------|--------|-----------------|-------------------|
| parse | `<file>` | `--ast`, `--tokens` | AST/トークン列 | 診断メッセージ | 0 | 1 |
| check | `<file>` | `--verbose` | 型チェック結果 | 型エラー診断 | 0 | 1 |
| compile | `<file>` | `-o <output>`, `--target` | (なし) | コンパイルエラー | 0 | 1 |
| build | `[dir]` | `-o <output>`, `--release` | ビルド進捗 | ビルドエラー | 0 | 1 |
| test | `<file>` | `--filter`, `--verbose` | テスト結果 | テスト失敗詳細 | 0 | 1 |
| review | `<file>` | `--format json` | レビュー結果 JSON | 診断メッセージ | 0 | 1 |
| doc-ack | `<file>` | `--trailer` | 確認メッセージ | エラー | 0 | 1 |
| doc-check | `<file>` | `--strict` | チェック結果 | 不整合エラー | 0 | 1 |
| install | `<package>` | `--global`, `--path` | インストール結果 | エラー | 0 | 1 |
| repl | (なし) | `--no-color` | REPL 出力 | エラー | 0 | 2 |
| lsp | (なし) | `--stdio`, `--port` | JSON-RPC stdout | ログ | 0 | 2 |
| fmt | `<file>` | `--check`, `--write` | フォーマット済みソース | 診断メッセージ | 0 | 1 |
| doc | `<file>` | `-o <dir>`, `--format` | 生成結果パス | エラー | 0 | 1 |

### 終了コード体系 (exit code)

| exit_code | 意味 | 適用コマンド |
|-----------|------|-------------|
| 0 | 成功 | 全コマンド |
| 1 | コンパイルエラー / 入力エラー | parse, check, compile, build, test, review, doc-ack, doc-check, install, fmt, doc |
| 2 | ランタイムエラー | repl, lsp |
| 127 | 不明コマンド | (dispatcher) |

### stdout/stderr の使い分け (AC-101)

- **stdout**: プログラムの正規出力 (AST, 型情報, フォーマット結果, JSON-RPC 等)
- **stderr**: 診断メッセージ (エラー, 警告, 進捗ログ)
- LSP モードでは stdout は JSON-RPC 専用、stderr はサーバーログ専用

---

## 依存関係

| 依存先 | 前提条件 | 理由 |
|--------|----------|------|
| **P11-1** 言語機能 parity | L# のセルフホスト言語機能が Rust 実装と同等 | ツールチェイン自体を L# で記述するために必要 |
| **P11-2** stdlib parity | 標準ライブラリが IO, ファイルシステム, プロセス起動を提供 | CLI/LSP が OS とやり取りするために必要 |
| **P11-3** compiler parity | L# 製コンパイラが `wasi-component` / `web-wasm` target を安定生成できる | guest component を host launcher 配布へ組み込むために必要 |

**P11-1 → P11-2 → P11-3 → P11-4** の順序依存があり、各フェーズの完了が次のフェーズの着手条件となる。

---

## リスクと制約

| リスク | 影響 | 緩和策 |
|--------|------|--------|
| host launcher のクロスビルドまたは component embedding が不安定 | 配布物の品質低下 | CI で 3 OS x 2 arch の host launcher ビルドと component smoke を維持 |
| Apple 公証の審査遅延 | macOS リリースの遅延 | 公証をリリースパイプラインの早期ステージに配置 |
| LSP full sync のパフォーマンス不足 | 大規模ファイルでの応答遅延 | 1000 行以下を v1 ターゲットとし、incremental sync を v2 で対応 |
| formatter/linter の AST 全ノード対応の工数 | スケジュール超過 | builtin rule を最小セットに絞り段階的に拡張 |
| Rust LSP → L# LSP 移行時の互換性破壊 | 既存ユーザーの開発体験悪化 | 移行期間中は両方を選択可能にし、段階的に移行 |
| Windows 環境での動作検証不足 | Windows ユーザーへの影響 | CI に Windows ランナーを追加し smoke test を必須化 |

---

## 用語定義

| 用語 | 定義 |
|------|------|
| **host launcher** | Wasmtime を内包し、L# 製 guest component を実行する配布用バイナリ |
| **guest component** | L# で実装された compiler/toolchain を Wasm Component Model 形式で包んだ成果物 |
| **single binary 配布** | host launcher に guest component を同梱した正式配布形 |
| **配布アーカイブ** | リリース時に提供される .tar.gz / .zip 形式のアーカイブ |
| **CLI parity** | L# 製 CLI が Rust 製 CLI と同一の引数・出力・終了コードを持つこと |
| **LSP parity** | L# 製 LSP が Rust 製 LSP と同一の JSON レスポンスを返すこと |
| **full sync** | LSP の TextDocumentSyncKind.Full -- 変更時にドキュメント全文を送信する方式 |
| **incremental sync** | LSP の TextDocumentSyncKind.Incremental -- 変更差分のみを送信する方式 |
| **roundtrip** | ソースコードを parse → format → parse した結果が元の AST と同一であること |
| **idempotency** | formatter を 2 回適用しても出力が変わらないこと |
| **公証 (notarization)** | Apple の公証サービスによるマルウェアスキャンと承認プロセス |
| **smoke test** | 配布物の基本動作を確認する最小限のテスト群 |
| **gate** | CI パイプラインで後続ステージの実行を許可する条件 |
| **trailer** | ソースファイル末尾に付与される doc-ack/doc-check 用のメタデータコメント |
| **schema** | JSON Schema 等による構造化データの型定義 |
