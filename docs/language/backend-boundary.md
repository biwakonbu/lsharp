# Backend 境界仕様

## 目的

本書は、L# compiler の frontend、lowering、codegen の間にある境界契約を定義する。
目的は、Wasm backend、Component Model backend、Native backend (deferred) が同一の Lowered IR を共有し、backend 固有の判断を codegen に閉じ込めることにある。

## 適用範囲

本書が扱うのは次の領域である。

- source から `FrontendResult` までの frontend の責務
- `FrontendResult` から `LoweredModule` を生成する lowering の責務
- `LoweredModule` から `CodegenArtifact` を生成する codegen の責務
- backend 間で共有される IR 契約

以下は本書の対象外とする。

- runtime API の詳細
- native backend 固有の ABI や object 形式
- Phase 単位の実装計画やタスク管理

## パイプライン全体像

```text
Source (.ls)
  -> Frontend (Lexer -> Parser -> MacroExpand -> TypeInfer)
  -> FrontendResult
  -> Lowering
  -> LoweredModule
  -> Codegen (Wasm / Component / Native (deferred))
  -> CodegenArtifact
```

この構造により、frontend は言語意味の解決に集中し、codegen は backend 固有の表現へ変換することに集中できる。

## 境界で受け渡す成果物

### FrontendResult

`FrontendResult` は frontend の最終成果物であり、型検査済みのプログラム表現を lowering に渡す。

| フィールド | 内容 | 要件 |
|------------|------|------|
| `program` | 型検査済み AST (`Program`) | lowering はこの AST を唯一の構文入力として扱う |
| `type_results` | 式ごとの推論型情報 (`TypeResults`) | lowering は追加の型推論を行わず、この結果を参照する |

`FrontendResult` は次の条件を満たさなければならない。

- 構文エラーと型エラーは frontend の時点で解決済みである
- backend 固有の calling convention やレジスタ情報を含まない
- lowering に必要な型情報を欠落なく保持する

### LoweredModule

`LoweredModule` は backend 非依存の IR モジュールであり、Wasm backend、Component Model backend、Native backend (deferred) の共通入力となる。

| フィールド | 内容 |
|------------|------|
| `functions` | 関数定義列 |
| `globals` | グローバル変数 |
| `data_segments` | 文字列定数などの静的データ |
| `gc_types` | GC 管理対象の型情報 |
| `imports` | runtime や外部環境から取り込む定義 |
| `exports` | backend 成果物に公開する定義 |

`LoweredModule` は次の条件を満たさなければならない。

- backend 間で共有できる表現であること
- レジスタ割付、スタックレイアウト、section 名など backend 固有の情報を含まないこと
- codegen が安定した順序で処理できるよう、関数・静的データ・シンボル列の順序が決定的であること

### CodegenArtifact

`CodegenArtifact` は codegen の最終成果物である。形式自体は backend ごとに異なるが、`LoweredModule` から生成される最終出力という役割は共通とする。

| 種別 | 内容 |
|------|------|
| `WasmArtifact` | `.wasm` バイナリと、その生成に付随する補助情報 |
| `ComponentArtifact` | `.component.wasm` — core Wasm に Component Model adapter を適用した成果物。WIT world 契約に従う |
| `NativeArtifact` | `.o` などのネイティブ成果物と、リンクに必要な補助情報 (deferred) |

codegen は `CodegenArtifact` を生成する責務のみを持ち、frontend や lowering の意味解析を再実装してはならない。

## 境界ごとの責務

### Frontend の責務

frontend は次を担当する。

- 字句解析、構文解析、マクロ展開、型推論
- AST の意味解決
- lowering が必要とする型情報の確定

frontend は runtime のレイアウトや target ABI を知る必要がない。

### Lowering の責務

lowering は次を担当する。

- 型検査済み AST を backend 非依存の IR へ写像する
- 制御構造、呼び出し、データ定義を明示的な IR へ変換する
- backend ごとの codegen が参照できる import / export / data 情報を整理する

lowering は次を行ってはならない。

- 特定ターゲットのレジスタや命令セットに依存した最適化
- Wasm / Component / Native (deferred) の artifact 形式の決定

### Codegen の責務

codegen は次を担当する。

- `LoweredModule` を backend 固有のバイナリ表現へ変換する
- Wasm / Component / Native (deferred) の artifact 分岐を `CodegenArtifact` 境界で完結させる
- target ごとの ABI、section、relocation、linker 連携を解決する
- runtime との接続点を具体化する
- Component Model backend では core Wasm 生成後の post-processing を管理し、WIT world 契約に沿った Component artifact を確定する

codegen は IR 自体の意味を変更してはならない。意味変換が必要な場合は lowering 境界へ戻して設計する。

## IR 共有方針

Wasm backend、Component Model backend、Native backend (deferred) は、次の原則に従って同一 IR を共有する。

1. 各 backend は同じ `LoweredModule` を入力とする
2. backend 固有の変換は codegen の内部に閉じる
3. calling convention、レジスタ割付、ABI などの情報は IR へ持ち込まない
4. runtime 依存は import / runtime 契約として明示し、暗黙の host 依存を作らない

この方針により、backend の追加や差し替えが frontend / lowering の設計を汚染しないようにする。

## 実装との対応

現在の主要な対応関係は次のとおりである。

| 層 | Rust 側の主な対応 | Selfhost 側の主な対応 |
|----|-------------------|------------------------|
| Frontend | `lsharp_syntax::ast::Program`, `lsharp_types::infer::TypeResults` | `mini-tokenize`, `mini-parse-defn`, `expand-macros-mini`, `ti-infer-expr` |
| Lowering | `lsharp_ir::Module`, `Lower::lower_program` | `compile-expr` を中心とする IR 生成 |
| Wasm Codegen | `lsharp_wasm::wasi::emit_wasm_wasi` | `emit-header`, `emit-type-section-main`, `leb128-u` など |

この表は現状の実装への対応を示す参考情報であり、仕様上の責務分割そのものは上記の境界契約で定義する。

## 適用例

この境界仕様は、ビルドや配布の文脈で次のように使い分ける。

| 用途 | 利用する backend | 主な成果物 |
|------|------------------|------------|
| bootstrap | Wasm | `stageN.wasm` |
| 固定点検証 | Wasm | 同一入力からの再現可能な `.wasm` |
| エンドユーザー配布 (正式) | Component | host launcher に embedded `.component.wasm` |
| HTTP server | Component | `wasi:http/incoming-handler` world の component |
| ブラウザ配布 | Wasm | browser 向け core `.wasm` |
| エンドユーザー配布 (deferred) | Native | プラットフォーム別バイナリ |

## 進化方針

境界契約を拡張する場合は、次の方針を守る。

- 既存 `LoweredModule` の意味を壊す変更は避ける
- 新しい backend 固有要件は、まず codegen 側の契約として局所化できるかを検討する
- frontend と lowering の境界を変更する場合は、型情報と意味解析の責務分担を明文化してから適用する

## Component Model 向け post-processing

Component Model backend は、core Wasm codegen の出力に post-processing を適用する形で実装する。

```text
LoweredModule
  -> Wasm Codegen (既存)
  -> core .wasm
  -> Component Adapter (wasm-tools / wit-component)
  -> .component.wasm
```

selfhost emitter は core Wasm のみを出力する責務を持ち、Component Model wrapping は host 側 (Rust) で行う。これにより selfhost emitter の複雑化を避ける。

現状の host 側 adapter layer は `crates/lsharp-wasm/src/component_adapter.rs` にあり、`embed_component_metadata_for_world()` で core/adaptor module へ `component-type` metadata を埋め込み、`componentize_core_module()` で `wit-component` を使って guest component を生成する。これにより adapter bytes の供給元と WIT world を切り替えつつ、selfhost emitter 側は core Wasm binary format に専念できる。

WIT world 定義と host/guest 境界の詳細は [`component-model-spec.md`](./component-model-spec.md) を参照。

## 関連文書

- [`runtime-spec.md`](./runtime-spec.md)
- [`component-model-spec.md`](./component-model-spec.md)
- [`native-backend-spec.md`](./native-backend-spec.md) (deferred)
