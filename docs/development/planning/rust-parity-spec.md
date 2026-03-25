# P11-3: コンパイラ中核の Rust Parity 仕様

## 概要

P11-3 は L# セルフホスティングにおいて、Rust で実装されたコンパイラ中核
(`lsharp-syntax`, `lsharp-types`, `lsharp-ir`, `lsharp-wasm`) を L# 自身で
再実装し、既存 Rust 実装と同一の観測挙動を達成するフェーズである。

## 目標

- Rust crate 群を参照しなくても、既存 examples/stdlib/selfhost が同一意味で通ること
- golden test によるフェーズごとの挙動比較を維持し、削除直前に全差分を解消すること
- 移植順を固定し、shadow mode 比較で段階的に切替えること

## 移植対象モジュール対応表

| Rust クレート | Rust モジュール | L# 移植先 (想定) |
|--------------|----------------|-----------------|
| `lsharp-syntax` | `span.rs`, `token.rs`, `lexer.rs`, `parser.rs`, `ast.rs`, `hygiene.rs`, `macro_expand.rs`, `derive.rs` | `selfhost/Span.ls`, `selfhost/Token.ls`, `selfhost/Lexer.ls`, `selfhost/Parser.ls`, `selfhost/Ast.ls`, `selfhost/Hygiene.ls`, `selfhost/MacroExpand.ls`, `selfhost/Derive.ls` |
| `lsharp-types` | `types.rs`, `infer.rs`, `constraints.rs`, `metadata_check.rs` | `selfhost/Types.ls`, `selfhost/TypeInfer.ls`, `selfhost/Constraints.ls`, `selfhost/MetadataCheck.ls` |
| `lsharp-ir` | `lib.rs`, `lower/mod.rs`, `lower/expr.rs`, `lower/decl.rs`, `lower/pattern.rs`, `closure.rs`, `module_graph.rs` | `selfhost/Ir.ls`, `selfhost/Lower.ls`, `selfhost/LowerExpr.ls`, `selfhost/LowerDecl.ls`, `selfhost/LowerPattern.ls`, `selfhost/Closure.ls`, `selfhost/ModuleGraph.ls` |
| `lsharp-wasm` | `wasi.rs`, `codegen.rs`, `emit.rs`, `test_runner.rs`, `wasi_runner.rs` | `selfhost/WasiBackend.ls`, `selfhost/Codegen.ls`, `selfhost/Emit.ls`, `selfhost/TestRunner.ls`, `selfhost/WasiRunner.ls` |

---

## P11-3 本体 (6 件)

### P11-3-1: syntax 移植方針

L# の `selfhost/Lexer.ls`, `selfhost/Parser.ls` は既に v3 まで拡張されているが、
Rust 側の `lsharp-syntax` が持つ以下の機能との parity を完了条件とする:

- span 付きトークン生成 (ソース位置追跡)
- 全 AST ノード型の表現 (Expr/Decl/Pattern/Literal/Metadata)
- 衛生マクロ (gensym, scope set, macro expansion trace)
- derive マクロ展開
- parser recovery と複数診断の並列報告

### P11-3-2: types 移植方針

HM 型推論、制約解決、metadata 検証、型表示を L# で再実装する。

- `TypeEnv`, `Substitution`, `TypeScheme` の等価表現
- `unify`, `generalize`, `instantiate` の中核アルゴリズム
- `TraitConstraint`, `ConstrainedTypeInfo` の制約系
- 高度型機能 (HKT/GADT/trait/where/type alias/record update) の最小完了集合
- type error の error code + span + 主要説明文の一致

### P11-3-3: IR 移植方針

AST から lowered IR への変換を L# で再実装する。

- `Module`, `Function`, `Instruction`, `IrType` の等価表現
- multi-file compile と module graph 解決
- closure conversion (自由変数解析、環境キャプチャ)
- pattern lowering (ADT タグ分岐、ネストパターン)
- trait dispatch lowering (辞書引数の静的挿入)

### P11-3-4: Wasm backend 移植方針

IR から Wasm バイナリを生成するバックエンドを L# で再実装する。

- `wasm-encoder` 相当のバイナリエミッタ
- WASI runtime helper (print, read_file, write_file, clock_now)
- test runner (`:example`, `:invariant` からのテスト自動生成)
- snapshot 生成と比較

### P11-3-5: golden test 戦略

各フェーズで Rust 実装と L# 実装の出力を比較する golden test を維持する。

**fixture 構成**:
```
tests/golden/
  syntax/   -- AST JSON + 診断メッセージ
  types/    -- 推論結果 (型注釈付き AST) + type error
  ir/       -- lowered IR snapshot (insta 互換 format)
  wasm/     -- Wasm バイナリの section hash + 実行結果
```

**比較ルール**:
- AST: ノード構造とスパン位置の構造的等価
- 型推論: 型変数の名前は正規化して比較 (alpha-equivalence)
- IR: snapshot format での行単位比較 (hash map 順序は安定化済み)
- Wasm: 実行結果の stdout/stderr/exit code の完全一致

**運用**:
- golden fixture は Rust 実装から生成し、L# 実装が同一出力を返すことを CI で検証
- Rust 実装の削除前に全 golden test の差分がゼロであることを確認
- 差分が発生した場合は L# 側を修正 (golden fixture を変更しない)

### P11-3-6: 完了条件

- Rust crate 群を参照しなくても既存 examples/stdlib/selfhost が同一意味で通る
- `cargo run -- compile/check/test` 相当のコマンドが L# 実装だけで同値動作する
- golden test の全差分がゼロ

---

## P11-3a: syntax parity (4 件)

### P11-3a-1: 移植対象の固定範囲

以下のモジュールを移植対象として固定する:

| Rust モジュール | 責務 | L# 移植先 |
|----------------|------|-----------|
| `span.rs` | ソース位置 (byte offset, line, column) | `selfhost/Span.ls` |
| `token.rs` | トークン種別の定義 | `selfhost/Token.ls` (既存拡張) |
| `lexer.rs` | 字句解析 | `selfhost/Lexer.ls` (既存拡張) |
| `parser.rs` | 構文解析 → AST 生成 | `selfhost/Parser.ls` (既存拡張) |
| `ast.rs` | AST ノード型定義 | `selfhost/Ast.ls` |
| `hygiene.rs` | 衛生マクロのスコープ管理 | `selfhost/Hygiene.ls` |
| `macro_expand.rs` | マクロ展開エンジン | `selfhost/MacroExpand.ls` (既存拡張) |
| `derive.rs` | derive マクロ展開 | `selfhost/Derive.ls` |

### P11-3a-2: golden fixture 化

- 既存 Rust parser テスト (`lsharp-syntax` の `#[cfg(test)]`) から入力ソースを抽出
- 各入力に対する期待 AST (JSON 形式) と診断メッセージを fixture ファイルとして出力
- L# parser が同じ入力に対して同一の AST 構造と診断を返すことを CI で検証
- fixture format: `{ "input": "...", "ast": {...}, "diagnostics": [...] }`

### P11-3a-3: 衛生マクロ統合

- macro 展開トレースバック (展開元 → 展開先のスパンチェーン) を selfhost 側で表現
- gensym カウンタをモジュールスコープで管理し、展開間の名前衝突を防止
- 衛生スコープ集合 (scope set) を `Set<ScopeId>` として表現
- 旧簡略表現 (P11-1 で導入した簡易版) を廃止し、Rust 実装と同一の衛生モデルへ統合

### P11-3a-4: parser recovery

- parser recovery (エラー発生後の解析続行) を parity 条件に含める
- 複数診断の並列報告: 1 回のパースで複数のエラー/警告を収集
- recovery 戦略: 閉じ括弧までスキップ、次のトップレベル宣言まで同期
- 診断の severity (error/warning/info) と error code を Rust 実装と一致させる

---

## P11-3b: types parity (4 件)

### P11-3b-1: HM 推論 parity

L# の型推論を Rust `lsharp-types/infer.rs` と同じ公開挙動へ揃える。

**中核アルゴリズム**:
- `infer_expr`: 式の型推論 (リテラル、変数、関数適用、λ抽象、let、match、if)
- `unify`: 型の単一化 (occurs check 含む)
- `generalize`: 環境に自由でない型変数を全称量化
- `instantiate`: 型スキームから新しい型変数でインスタンス化

**制約互換性**:
- `TraitConstraint` の解決順序
- `ConstrainedTypeInfo` の伝播ルール
- 制約未解決時のエラー報告

### P11-3b-2: 高度型機能の最小完了集合

以下を最小完了集合として parity 条件に含める:

| 機能 | Rust 実装箇所 | parity 条件 |
|------|-------------|------------|
| HKT (Higher-Kinded Types) | `types.rs` Type::App | kind チェックと適用の等価性 |
| GADT | `infer.rs` match 分岐 | コンストラクタ型の特殊化 |
| trait | `constraints.rs` | 辞書引数の挿入と解決 |
| where 節 | `constraints.rs` | 制約の伝播と検証 |
| type alias | `types.rs` | 展開と表示の等価性 |
| record update | `infer.rs` | フィールド型の推論と更新 |

### P11-3b-3: type error parity

- error code (E0001 形式) の一致を要求
- span (エラー位置) の一致を要求
- 主要説明文 (primary message) の意味的一致を要求
- 補助説明文 (secondary label) とヘルプメッセージは byte-to-byte 一致を要求しない
- miette 互換のエラー表示構造を L# 側で再現

### P11-3b-4: inference 結果の deterministic ordering

- 型変数の割り当て順序を source order に固定
- 型環境の走査順を挿入順に固定 (hash map → ordered map)
- hover/knowledge/doc 出力で型情報を表示する際の順序を安定化
- golden test で比較可能な正規化形式を定義

---

## P11-3c: IR parity (4 件)

### P11-3c-1: IR 移植対象

以下のモジュールを L# へ移植する:

| Rust モジュール | 責務 | 依存 |
|----------------|------|------|
| `module_graph.rs` | モジュール依存グラフの構築とトポロジカルソート | syntax (import 解析) |
| `lower/mod.rs` | lowering エントリポイント | types (型情報), syntax (AST) |
| `lower/expr.rs` | 式の lowering | lower/mod |
| `lower/decl.rs` | 宣言の lowering | lower/mod |
| `lower/pattern.rs` | パターンの lowering (ADT タグ分岐) | lower/mod |
| `closure.rs` | closure conversion (自由変数解析、環境キャプチャ) | lower |

**multi-file compile**: 複数 .ls ファイルのコンパイルにおいて、module graph で依存順序を
解決し、トポロジカル順に lowering を実行する。循環依存はエラーとする。

### P11-3c-2: IR snapshot format

lowered IR の snapshot format を以下のように仕様化する:

```
; module: <module_name>
; function: <function_name> (<param_types>) -> <return_type>
  <instruction>
  <instruction>
  ...
```

- 命令は indent 2 spaces
- 関数はセミコロンコメントで区切り
- 型は正規化表記 (型変数は t0, t1, ... で付番)
- この format は `insta` snapshot と互換

### P11-3c-3: IR 生成順の安定化

- 関数の出力順は source order (ソースコード中の出現順) に固定
- 静的データの出力順はモジュール内の宣言順に固定
- hash map に依存する出力順非決定性を禁止
- `BTreeMap` または `IndexMap` 相当の ordered collection を使用
- IR snapshot の比較が非決定的にならないことを CI で検証

### P11-3c-4: Rust/L# IR snapshot 比較

- Rust 実装と L# 実装の両方で IR snapshot を生成する比較ジョブを CI に追加
- native backend 完成まで比較ジョブを維持 (Rust 実装削除後は L# 単独に移行)
- 差分が発生した場合は CI を red にし、L# 側を修正
- 比較対象: examples/ + stdlib/ + selfhost/ の全 .ls ファイル

---

## P11-3d: backend parity (4 件)

### P11-3d-1: Wasm backend feature parity

Rust `lsharp-wasm` の以下の機能を L# で再実装する:

**codegen 機能**:
- 整数/浮動小数点演算
- 関数呼び出し (直接/間接)
- 制御フロー (if/loop/block/br)
- メモリ操作 (load/store/grow)
- テーブル操作 (closure の関数ポインタ)
- グローバル変数
- 複数戻り値 (multi-value)
- WASI import (fd_write, proc_exit, args_get, environ_get 等)

**emit 機能**:
- Wasm バイナリの section 構成 (type, import, function, table, memory, global, export, start, element, code, data)
- `wasm-encoder` 相当のバイナリエンコーダ

### P11-3d-2: test runner / wasi helper 移植

- `:example` メタデータからのテスト自動生成
- `:invariant` メタデータからのプロパティテスト生成
- WASI runtime helper (stdin/stdout/stderr, ファイル I/O, clock, environ)
- テスト実行結果の集計と報告 (passed/failed/skipped)
- L# 実装のテスト検証を Rust ツールに依存させない

### P11-3d-3: runtime helper 同時変更原則

- runtime helper の仕様変更は Wasm backend と native backend の同時変更を原則とする
- 片系だけ先行する変更を禁止
- runtime helper の公開 API:
  - `lsharp_runtime_init`: ランタイム初期化
  - `lsharp_alloc`: メモリ割り当て
  - `lsharp_print`: 標準出力
  - `lsharp_read_file` / `lsharp_write_file`: ファイル I/O
  - `lsharp_clock_now`: 時刻取得

### P11-3d-4: backend 差分の閉じ込め

backend 間の差分を以下の 2 つのレイヤーに閉じ込める:

**target descriptor**: ターゲット固有の定数とレイアウト情報
- word size, endianness, alignment
- 呼び出し規約 (Wasm: stack machine, native: register-based)
- section 名と visibility のマッピング

**runtime adapter**: ターゲット固有のランタイムバインディング
- Wasm: WASI import
- native: libc / OS syscall

共通 codegen 契約 (IR → backend 命令) はターゲット非依存とし、
target descriptor と runtime adapter だけで切替え可能にする。

---

## P11-3e: parity 移行順 (4 件)

### P11-3e-1: 移植順の固定

移植順を以下に固定する (依存関係のトポロジカル順):

```
1. syntax (Lexer, Parser, AST, 衛生マクロ, derive)
2. types (型推論, 制約解決, metadata 検証, 型表示)
3. IR (lowering, closure conversion, pattern lowering, module graph)
4. Wasm backend (codegen, emit, WASI)
5. Native backend (object emitter, linker integration)
6. tools (CLI, LSP, formatter, linter, test runner)
```

各段は前段の L# 実装に依存するため、順序の入替えは不可。

### P11-3e-2: shadow mode 比較

各段で Rust 実装を削除せず、shadow mode で比較する:

**shadow mode の動作**:
1. 入力を Rust 実装と L# 実装の両方に渡す
2. 両方の出力を比較
3. 差分があれば CI で報告 (red にはしない、最初は warning)
4. 2 段 (2 つの連続するサブフェーズ) で CI 緑になってから既定経路を L# に切替え

**切替え手順**:
1. shadow mode で差分ゼロを確認
2. L# を既定経路に設定、Rust をフォールバックに変更
3. 1 週間の安定期間
4. Rust フォールバックを無効化
5. (P11-3f 完了条件クリア後) Rust 実装を削除

### P11-3e-3: 公開機能単位の切替え

切替え単位は crate 単位ではなく公開機能単位とする。

例:
- parser の基本構文解析は parity 達成済みでも、衛生マクロは未達の場合:
  - 基本構文解析は L# を既定経路にする
  - 衛生マクロは Rust を既定経路に残す

partial parity でもユーザーに見える挙動が安定したところから既定経路を更新する。

**機能単位の粒度**:
- syntax: 基本パース / マクロ展開 / derive / parser recovery
- types: 基本推論 / 制約解決 / 高度型 / metadata check
- IR: 基本 lowering / closure conversion / pattern lowering / module graph
- backend: codegen / emit / test runner / WASI runtime

### P11-3e-4: ADR 記録

parity 進捗は TODO.md だけでなく ADR (Architecture Decision Record) にも記録する。

記録内容:
- 各機能単位の parity 達成日
- shadow mode 比較の結果サマリ
- 切替え判断の理由と影響範囲
- 撤去判断の監査証跡 (誰が、いつ、何の根拠で承認したか)

ADR 格納先: `docs/adr/decisions-003.jsonl` (P11 系列)

---

## P11-3f: 完了条件 (3 件)

### P11-3f-1: cargo run 互換

`cargo run -- ...` 相当の既存コマンド群が L# 実装だけで同値動作すること:

| コマンド | 検証内容 |
|---------|---------|
| `parse --ast` | AST の構造的等価 |
| `check` | 型チェック結果 (成功/失敗) と診断メッセージ |
| `compile -o` | 生成 Wasm の実行結果一致 |
| `test` | `:example` / `:invariant` テスト結果一致 |
| `build` | プロジェクトビルド結果一致 |

### P11-3f-2: golden test 全通過

Rust 実装を外した状態で以下の golden test が全通過すること:

- parser golden test (AST + 診断)
- type inference golden test (型注釈付き AST + type error)
- IR snapshot test (lowered IR)
- backend golden test (Wasm 実行結果)
- E2E test (`crates/lsharp-wasm/tests/e2e.rs` 相当)

### P11-3f-3: 差分報告空

examples/stdlib/selfhost の全主要ケースで Rust/L# の差分報告が空になること:

**検証対象**:
- `examples/` 配下の全 .ls ファイル
- `stdlib/` 配下の全 .ls ファイル
- `selfhost/` 配下の全 .ls ファイル

**比較項目**:
- parse 結果 (AST)
- type check 結果 (型注釈)
- IR 生成結果 (snapshot)
- Wasm 生成結果 (実行 stdout/stderr/exit code)

全ての比較項目で差分がゼロであることを以て P11-3 完了とする。
