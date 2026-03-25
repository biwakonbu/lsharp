# TODO残タスク完了 (P8/P11) - 設計書

> 最終更新: 2026-03-25

## 概要

27 件のタスクを 3 つの実行トラックに分割して並行推進する設計。
コード実装 6 件と仕様固定 21 件を効率的に完了させる。

## アーキテクチャ

### 実行トラック構成

```
Track 1: コード実装 (6 件) -- 依存チェーンに沿って順次実装
  P11-2a (import 解決) -> P11-2 (パイプライン統合) -> P8-9 (セルフコンパイル/固定点)

Track 2: 仕様固定 -- CI/運用 (13 件) -- Track 1 と並行して仕様書作成
  P11-6 + P11-6a + P11-6b

Track 3: 仕様固定 -- リリース/撤去 (8 件) -- Track 2 と並行して仕様書作成
  P11-6c + P11-6d
```

### トラック間依存

```
                    [即時開始]
                        |
         +--------------+--------------+
         |                             |
    Track 2/3                     Track 1
    (仕様固定)                   (コード実装)
         |                             |
   ci-migration-spec.md          Phase 1: P11-2a
   legacy-isolation-spec.md        import/module 解決
   release-operations-spec.md          |
   final-removal-spec.md          Phase 2: P11-2
         |                       パイプライン統合
   TODO.md 21 件更新                   |
                                  Phase 3: P8-9
                                  セルフコンパイル + 固定点
                                       |
                                  TODO.md 6 件更新
```

## コンポーネント

### Track 1: コード実装

#### Phase 1: import/module 解決 (P11-2a)

##### module 宣言の追加

各 selfhost ファイルに module 宣言を追加する。

対象 15 ファイル:

| ファイル | module 名 | import 先 |
|---------|-----------|-----------|
| Token.ls | Token | (なし) |
| IR.ls | IR | (なし) |
| AST.ls | AST | Token |
| Lexer.ls | Lexer | Token |
| Parser.ls | Parser | Token, AST, Lexer |
| Type.ls | Type | (なし) |
| TypeScheme.ls | TypeScheme | Type |
| TypeInfer.ls | TypeInfer | AST, Type, TypeScheme |
| MacroExpand.ls | MacroExpand | AST |
| Compiler.ls | Compiler | AST, IR |
| WasmEmit.ls | WasmEmit | IR |
| Linter.ls | Linter | AST |
| Formatter.ls | Formatter | AST |
| JsonRpc.ls | JsonRpc | (なし) |
| Main.ls | Main | 全モジュール |

##### module graph 解決

Rust 側の `module_graph.rs` の topological sort を参考に、コンパイル順を固定:

```
コンパイル順 (依存先から):
1. Token.ls, IR.ls, Type.ls
2. AST.ls, Lexer.ls, TypeScheme.ls, JsonRpc.ls
3. Parser.ls, TypeInfer.ls, MacroExpand.ls, Compiler.ls, WasmEmit.ls, Linter.ls, Formatter.ls
4. Main.ls
```

##### Main.ls のリファクタリング

Main.ls (780 行) から以下を除去し、import ベースに移行:

- Token 定数の再定義 -> `(import Token)` に置換
- AST タグの再定義 -> `(import AST)` に置換
- IR オペコードの再定義 -> `(import IR)` に置換
- Compiler のインラインコピー -> `(import Compiler)` に置換
- WasmEmit のインラインコピー -> `(import WasmEmit)` に置換
- expand-macros-mini -> `(import MacroExpand)` に置換
- ti-infer-expr 簡易版 -> `(import TypeInfer)` に置換

リファクタリング後の Main.ls は 200 行以下を目標とする (パイプライン接続 + CLI エントリのみ)。

#### Phase 2: パイプライン統合 (P11-2)

compile-full-pipeline を完全版に更新:

```lisp
;; 更新前
(defn compile-full-pipeline (source)
  (let tokens (tokenize source))
  (let ast (parse tokens))
  (let expanded (expand-macros-mini ast))    ;; パススルー
  (let typed (ti-infer-expr ast))            ;; 簡易版
  (let ir (compile-to-ir typed))
  (emit-wasm ir))

;; 更新後
(defn compile-full-pipeline (source)
  (let tokens (tokenize source))
  (let ast (parse tokens))
  (let expanded (expand-macros ast))          ;; MacroExpand.ls の完全版
  (let typed (infer-program expanded))        ;; TypeInfer.ls の完全版
  (let ir (compile-to-ir typed))
  (emit-wasm ir))
```

#### Phase 3: セルフコンパイル + 固定点 (P8-9)

- stage1.wasm (Rust が生成) が selfhost ソースを入力として stage2.wasm を生成
- stage1.wasm == stage2.wasm のバイト列一致を検証

### Track 2/3: 仕様固定

#### 仕様書構成

| 仕様書 | カバー範囲 |
|--------|-----------|
| `docs/ci-migration-spec.md` | P11-6 (5 件) + P11-6a (4 件) |
| `docs/legacy-isolation-spec.md` | P11-6b (4 件) |
| `docs/release-operations-spec.md` | P11-6c (4 件) |
| `docs/final-removal-spec.md` | P11-6d (3 件) |

## データ設計

### module graph データ構造

module 間の依存関係をグラフとして表現し、topological sort でコンパイル順を決定する。

```
Token (依存なし)
IR (依存なし)
Type (依存なし)
  |
  +-> AST (Token に依存)
  +-> Lexer (Token に依存)
  +-> TypeScheme (Type に依存)
  +-> JsonRpc (依存なし)
       |
       +-> Parser (Token, AST, Lexer に依存)
       +-> TypeInfer (AST, Type, TypeScheme に依存)
       +-> MacroExpand (AST に依存)
       +-> Compiler (AST, IR に依存)
       +-> WasmEmit (IR に依存)
       +-> Linter (AST に依存)
       +-> Formatter (AST に依存)
            |
            +-> Main (全モジュールに依存)
```

## エラーハンドリング

### import 解決失敗

module が見つからない場合:

```
Error: Module 'Foo' not found
  at Main.ls:3:1
  hint: Expected file 'selfhost/Foo.ls' or 'stdlib/Foo.ls'
```

### 循環依存

module graph に循環がある場合:

```
Error: Circular dependency detected
  Lexer -> Parser -> Lexer
```

### 固定点不一致

stage1.wasm != stage2.wasm の場合:

- section diff を出力
- symbol diff を出力
- data section diff を出力
- CI artifact として保存

## テスト戦略

### Track 1 のテスト

#### E2E テスト (5 件追加)

- `test_e2e_selfhost_module_declarations`: selfhost ファイルの module/import 宣言検証
- `test_e2e_selfhost_topological_sort`: module graph の topological sort 検証
- `test_e2e_selfhost_main_structure`: Main.ls の構造化リファクタリング検証
- `test_e2e_selfhost_pipeline_integration`: 完全版パイプライン統合検証
- `test_e2e_bootstrap_deterministic`: bootstrap の決定性検証

#### 回帰テスト

- 既存テスト 852 件が全て通ること (追加 5 件含む)

### Track 2/3 のテスト

仕様書のみのため追加テストは不要。仕様書内に記載する CI job 名と実際の CI 構成の整合性を確認する。

## 実装優先順位

1. Track 2/3: 仕様固定 21 件 (即座に完了可能、依存なし)
2. Track 1 Phase 1: P11-2a import/module 解決
3. Track 1 Phase 2: P11-2 パイプライン統合
4. Track 1 Phase 3: P8-9 セルフコンパイル + 固定点

## 成果物一覧

### 仕様書 (4 本)

- `docs/ci-migration-spec.md` (16,732 bytes)
- `docs/legacy-isolation-spec.md` (11,381 bytes)
- `docs/release-operations-spec.md` (16,744 bytes)
- `docs/final-removal-spec.md` (7,875 bytes)

### コード変更

- selfhost 15 ファイルに module/import 宣言追加
- Main.ls 構造化リファクタリング (import ベース)
- E2E テスト 5 件追加
- テスト総数: 852 件全通過

## 関連ドキュメント

- [要件定義書](./requirements.md)
- [CI 移行仕様](../../ci-migration-spec.md)
- [legacy 隔離仕様](../../legacy-isolation-spec.md)
- [リリース運用仕様](../../release-operations-spec.md)
- [最終撤去仕様](../../final-removal-spec.md)
