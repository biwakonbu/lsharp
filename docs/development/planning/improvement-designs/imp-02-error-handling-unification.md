# imp-02: エラーハンドリング統一と LS#### エラーコード体系

> 対象 issue: [I-02](../../../../ISSUES.md#i-02) (エラーハンドリング不統一)、[DOC-06](../../../../ISSUES.md#doc-06) (エラーコード体系未定義)
> ロードマップ: [improvement-roadmap.md](../improvement-roadmap.md) Phase A-1

## 概要

エラーハンドリングの方針を全クレートで統一する。現状は:

- miette のリッチ診断 (ソーススパン付き) は lsharp-driver の最上層のみ
- 下層クレート (syntax / types / ir / wasm) は thiserror ベースのエラー型のみで、span 情報の伝播が層ごとに途切れる
- 本番経路に panic がある (`crates/lsharp-ir/src/lib.rs:3609`, `:3611` のファイル I/O panic)
- エラーコード体系が存在せず、MCP の `lsharp_errors` は E0001〜E0005 のハードコード (`crates/lsharp-driver/src/mcp_server.rs:438-462`)

これを「全層 Diagnostic 貫通 + LS#### コード体系」へ統一する。

## 設計

### 1. エラー型の層別方針

| 層 | エラー型 | 方針 |
|----|---------|------|
| lsharp-syntax | `ParseError` | 既存 thiserror 型に `#[derive(Diagnostic)]` を追加し、`#[diagnostic(code(...))]` で LS コードを付与 |
| lsharp-types | `TypeError` | 同上 |
| lsharp-ir | `LowerError` | 同上。ファイル I/O 系 panic は `LowerError::Io` バリアントへ置換 |
| lsharp-wasm | `CodegenError` / ランタイムエラー | 同上。wasmtime 実行エラーは exit code + 診断メッセージへ整形 |
| lsharp-driver | `miette::Report` | 既存どおり最上層で集約。変更最小 |

実装は miette の `Diagnostic` derive を下層クレートに追加するだけで済み、
既存の `thiserror::Error` 定義と共存できる (miette は thiserror の上に被せられる)。
既存のエラーバリアント名・メッセージは変更しない (スナップショットテストへの影響を最小化)。

### 2. panic 許容基準

| 区分 | panic | 備考 |
|------|-------|------|
| テストコード (`#[cfg(test)]`, tests/) | 許容 | `expect("理由")` を推奨 |
| コンパイラ内部不変条件 (到達不能分岐) | `unreachable!("不変条件の説明")` のみ許容 | 入力起因で到達し得る分岐は不可 |
| 入力起因の失敗 (I/O、パース、型、codegen) | 禁止 | `Result` で伝播する |

`crates/lsharp-ir/src/lib.rs:3609-3611` の `unwrap_or_else(|err| panic!(...))` は
入力起因 (ファイル読み込み・パース失敗) のため `LowerError` への置換対象。

機械検査: `grep -rn "panic!\|\.unwrap()\|\.expect(" crates/*/src --include="*.rs"` の結果から
`#[cfg(test)]` ブロックを除いた残数を CI で監視する (まず現状値を固定し、増加を禁止 →
段階的にゼロへ)。

### 3. LS#### エラーコード体系

既存 MCP の `E0001`〜`E0005` は L# 固有であることが分かりにくく、Rust の `E####` と紛らわしい。
プレフィックスを `LS` とし、層ごとに番台を割り当てる:

| 範囲 | 層 | 例 |
|------|----|----|
| LS0001-LS0999 | 字句・構文 (lsharp-syntax) | LS0001 予期しないトークン / LS0002 予期しない EOF / LS0003 未知のフォーム |
| LS1001-LS1999 | 型 (lsharp-types) | LS1001 未定義の識別子 / LS1002 型不一致 / LS1003 無限型 / LS1004 if 条件が Bool でない / LS1005 引数型不一致 |
| LS2001-LS2999 | 制約・メタデータ (lsharp-types) | LS2001 制約不成立 / LS2002 :example 失敗 |
| LS3001-LS3999 | lowering / モジュール (lsharp-ir) | LS3001 未サポート構文 / LS3002 未定義関数 / LS3003 モジュール解決失敗 / LS3004 ソース読込失敗 |
| LS4001-LS4999 | codegen / ランタイム (lsharp-wasm) | LS4001 codegen 失敗 / LS4002 GC 容量超過 (imp-03 の grow 失敗診断) |
| LS5001-LS5999 | CLI / プロジェクト (lsharp-driver) | LS5001 lsharp.toml 不正 / LS5002 パッケージ解決失敗 |

運用規則:

- コードは一度割り当てたら意味を変えない (欠番は再利用しない)
- 正本一覧は `docs/guides/error-reference.md` (imp-05 で新設するページ) に置き、
  コード・説明・対処を 1 コード 1 節で記載する
- 既存 `E0001`〜`E0005` は対応表 (E0001→LS1001 等) を 1 リリースの間 MCP 応答に併記して移行する

### 4. CLI / LSP / MCP の診断統一

3 つのフロントエンドが同じ診断ソースから同じコードを返すようにする:

- **CLI**: miette のレンダリングに `code` が含まれる (derive の `#[diagnostic(code(...))]` で自動)
- **LSP**: `Diagnostic.code` フィールドに LS コードを設定 (`crates/lsharp-lsp/src/lib.rs` の診断変換部)
- **MCP**: `lsharp_errors` のハードコード表を廃止し、エラーコード定義の単一ソース
  (例: `crates/lsharp-driver/src/error_codes.rs` に定数表) から引く。
  `lsharp_check` 等の応答にも LS コードを含める

### 5. 実装順序 (TDD)

1. エラーコード定数表 + 対応表のユニットテスト (RED → GREEN)
2. lsharp-syntax へ Diagnostic derive + LS コード付与 (スナップショット維持確認)
3. lsharp-types → lsharp-ir → lsharp-wasm の順に同様
4. ir/lib.rs の panic 置換 (E2E で診断出力を検証)
5. LSP / MCP の code 配線
6. panic 残数の CI 監視テスト追加

## 影響範囲

- 下層クレートに `miette` 依存が追加される (workspace 依存は既存)
- エラーメッセージ文言は不変のため、既存スナップショットは原則無変更
- MCP の `lsharp_errors` 応答形式に `ls_code` フィールドが増える (後方互換)

## ステータス

設計のみ (2026-06-12 起草)。着手時は TODO.md に Phase A-1 として項目を作成する。
