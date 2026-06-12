# imp-05: ドキュメント再構成 (ユーザー導線の整備)

> 対象 issue: [DOC-01](../../../../ISSUES.md#doc-01) (ガイド不足)、[DOC-02](../../../../ISSUES.md#doc-02) (book 読者層混在)、
> [DOC-03](../../../../ISSUES.md#doc-03) (doc-status 未運用)、[DOC-04](../../../../ISSUES.md#doc-04) (examples 連携不足)、
> [DOC-05](../../../../ISSUES.md#doc-05) (language-guide 二重管理)
> ロードマップ: [improvement-roadmap.md](../improvement-roadmap.md) Phase D-1 / D-2
> 関連: [imp-02](imp-02-error-handling-unification.md) (LS#### エラーコード体系がエラーリファレンスの入力)

## 概要

ユーザー向けドキュメントの正面玄関 `docs/guides/` は 4 文書 238 行に留まり、
metadata 駆動開発・IDE 統合・デプロイターゲット選択・stdlib・エラーリファレンスの
ガイドが存在しない。一方で実装解説 (book/、5000 行超) と AI 向けテンプレート
(`crates/lsharp-driver/templates/lsharp-language-guide.md`) は充実しており、
**情報は存在するのに人間の新規ユーザーへ届く形になっていない**のが本質的な問題である。

## 設計

### 1. docs/guides/ の拡張 (DOC-01, Phase D-1)

以下 5 ページを新設する。記述の種は language-guide テンプレート (後述 4) と book から移植する:

| 新設ページ | 内容 | 主な種 |
|-----------|------|--------|
| `docs/guides/metadata-driven-development.md` | `:doc` / `:params` / `:returns` / `:example` / `:invariant` / `:transitions` の完全仕様、`lsharp test` / `lsharp doc` での活用手順 | templates/lsharp-language-guide.md、book/ch13 |
| `docs/guides/ide-setup.md` | `lsharp lsp` のエディタ別セットアップ (VS Code / Vim / Emacs)、提供機能一覧 (hover / completion / definition / references / rename / formatting / diagnostics) と既知の制限 (FULL sync 等) | book/ch16 |
| `docs/guides/deployment-targets.md` | `lsharp compile` のターゲット選択ガイド (wasi-component / preview1 / web-wasm / native の使い分け、native の現況は TODO.md 正本を参照) | templates/lsharp-language-guide.md、docs/language/backend-boundary.md |
| `docs/guides/stdlib-guide.md` | stdlib モジュール一覧と探し方 (`lsharp doc-site` での生成、MCP `lsharp_stdlib_api`) | book/ch14 |
| `docs/guides/error-reference.md` | LS#### エラーコードの正本一覧 (コード / 説明 / 対処)。imp-02 の体系導入後に実コードと同期 | imp-02、mcp_server.rs の既存 5 件 |

合わせて `docs/guides/README.md` を導線ハブへ拡張する
(「最初に読む順序」「book のどの章がユーザー向けか」を明記)。
公開サイトに載せるページは `docs/site.toml` (公開ページの正本) への登録を忘れない。

### 2. book/ の読者層分離 (DOC-02, Phase D-2)

章の中身は動かさず、`book/README.md` (目次) に読者層ラベルを付ける:

- **言語ユーザー向け**: ch01 (概要), ch04 (型システムの使い方相当部), ch07-09 (レコード/ADT/モジュール), ch10 (トレイト), ch13 (テスト), ch14 (stdlib)
- **コンパイラ実装者向け**: ch02-03 (lexer/parser), ch05-06 (IR/codegen), ch11-12 (高度型/制約の内部), ch15 (selfhost), ch16 (LSP 実装)

公開 CLI で `parse` / `check` / `fmt` を内部 API とする方針 (README) と整合させ、
ユーザー向け章では内部コマンドを使った手順を書かない。

### 3. examples ↔ 機能マトリクス (DOC-04, Phase D-2)

`docs/guides/README.md` (または examples/README.md 新設) に対応表を置く:

- 各 .ls サンプル → デモする言語機能 → 関連ガイド/章
- **実行可能** / **型チェックのみ** (gadt.ls / hkt.ls / computation.ls) の区別を明示
  (imp-01 の Stage 達成で「型チェックのみ」が解消されたら更新)
- ビルド成果物 (examples/*.wasm) は .gitignore 対象とするか確認し、ソースと分離する

### 4. language-guide テンプレートの正本一本化 (DOC-05, Phase D-2)

`crates/lsharp-driver/templates/lsharp-language-guide.md` (Claude Skill 用) と
docs/guides/ の二重管理を避ける方針:

- **正本は docs/guides/** とする (人間向けが正、AI 向けは派生)
- テンプレートには「正本は docs/guides/ であり、本ファイルは AI セッション向けの要約」と
  明記し、内容更新時は docs/guides/ → テンプレートの順で同期する
- 将来案 (任意): テンプレートを docs/guides/ から生成するスクリプト化。
  まずは手動同期 + 同期チェックの契約テスト (主要見出しの存在確認) で十分
- 注意: 本設計の起草時点でテンプレート拡張は未コミットの作業ツリー変更であり、
  その変更自体には本設計から手を入れない

### 5. doc-status の運用開始 (DOC-03, Phase D-2)

実装済みの鮮度追跡 (`lsharp review` / `doc-ack` / `doc-check`、
`crates/lsharp-driver/src/main.rs:286` 付近) を自プロジェクトで運用する:

1. `.lsharp-doc-status` を生成してコミットし、selfhost/stdlib の `:doc` 群を初回 ack する
2. CI に `lsharp doc-check` ジョブを追加し、コード変更でドキュメントが Stale になったら
   fail (または warning) にする
3. 運用手順 (ack のタイミング、Stale 時のフロー) を `docs/development/operations/` に 1 ページ追加する

### 6. 実装順序

1. error-reference.md 以外の guides 4 ページ (imp-02 に依存しないため先行可)
2. guides/README.md ハブ化 + book 目次ラベル + examples マトリクス
3. doc-status 運用開始 (CI 統合)
4. error-reference.md (imp-02 の LS#### 導入後)
5. language-guide テンプレート同期 (テンプレート変更がコミットされた後)

## 影響範囲

- docs/guides/ の増設と site.toml 登録のみで、コード変更は doc-check の CI 追加と
  同期契約テストに限られる
- book/ は目次 (README) のみ変更し、章本文は動かさない

## ステータス

設計のみ (2026-06-12 起草)。着手時は TODO.md に Phase D-1 / D-2 として項目を作成する。
