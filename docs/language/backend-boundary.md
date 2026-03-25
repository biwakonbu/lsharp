# Backend 境界仕様書

> P11-2: ブートストラップ閉路の基盤

## 概要

L# コンパイラの backend 境界を `FrontendResult -> LoweredModule -> CodegenArtifact` の 3 層に固定し、Wasm backend と将来の Native backend が同一の Lowered IR を共有する方針を定める。

```
Source (.ls)
  -> Frontend (Lexer -> Parser -> MacroExpand -> TypeInfer)
  -> FrontendResult (AST + 型情報)
  -> Lowering
  -> LoweredModule (IR)
  -> Codegen (Wasm / Native)
  -> CodegenArtifact (Wasm bytes / Native object)
```

---

## FrontendResult

Frontend が出力する中間表現。AST と型情報のペア。

### 定義

```
FrontendResult = {
  program: Program       -- 型チェック済み AST (Vec<Decl>)
  type_results: TypeResults  -- 各式の推論型マップ
}
```

### Rust 側の対応

- `lsharp_syntax::ast::Program` -- パース結果の AST
- `lsharp_types::infer::TypeResults` -- 型推論結果 (`HashMap<ExprId, Type>`)

### Selfhost 側の対応 (Main.ls)

- `compile-full-pipeline` の Step 1-4 が Frontend に対応
  - Step 1: `mini-tokenize` (Lexer)
  - Step 2: `mini-parse-defn` (Parser)
  - Step 3: `expand-macros-mini` (MacroExpand)
  - Step 4: `ti-infer-expr` (TypeInfer)
- 出力: `[tokens, defn-ast, expanded-body, ti-result, ir-instrs]` の vector

---

## LoweredModule

Frontend の出力を IR に変換した中間表現。Backend 非依存。

### 定義

```
LoweredModule = {
  functions: Vec<Function>   -- 関数定義列
  globals: Vec<Global>       -- グローバル変数
  data_segments: Vec<Data>   -- 静的データ (文字列定数等)
  gc_types: Vec<GcType>      -- GC 管理型 (ADT, レコード)
  imports: Vec<Import>       -- 外部インポート (WASI 等)
  exports: Vec<Export>       -- エクスポート定義
}
```

### Rust 側の対応

- `lsharp_ir::Module` -- IR モジュール (`lower.rs` の `Lower::lower_program` が生成)
- `lsharp_ir::Function` -- 関数定義 (名前, 引数型, 戻り値型, ローカル, 命令列)
- `lsharp_ir::Instruction` -- IR 命令 (I64Const, LocalGet, Call, If, Block, etc.)

### Selfhost 側の対応 (Main.ls)

- `compile-full-pipeline` の Step 5 が Lowering に対応
  - `compile-expr` が AST ノードを IR 命令列 (`[opcode, operand]` の vector) に変換
- IR opcodes: `ir-i64-const(1)`, `ir-local-get(10)`, `ir-local-set(11)`, `ir-call(40)`, `ir-if(41)`, etc.

---

## CodegenArtifact

Codegen が出力する最終成果物。Backend ごとに異なるバイナリ形式。

### 定義

```
CodegenArtifact = WasmArtifact | NativeArtifact

WasmArtifact = {
  bytes: Vec<u8>            -- WebAssembly バイナリ (.wasm)
  source_map: Option<SourceMap>  -- デバッグ用ソースマップ
}

NativeArtifact = {
  object: Vec<u8>           -- オブジェクトファイル (.o)
  target: TargetTriple      -- ターゲットトリプル
  runtime_deps: Vec<String> -- リンク時に必要なランタイムライブラリ
}
```

### Rust 側の対応 (Wasm backend)

- `lsharp_wasm::wasi::emit_wasm_wasi(&Module) -> Result<Vec<u8>>` -- Wasm バイナリ生成
- `wasm-encoder` クレートで Type/Function/Export/Code セクションを構築

### Selfhost 側の対応 (Main.ls)

- `emit-header` -- Wasm マジックバイト + バージョン (8 bytes)
- `emit-type-section-main` -- Type セクション
- `leb128-u` -- LEB128 エンコーディング

### Native backend (未実装 -- P11-2b で設計)

- 出力形式: object file + 最小ランタイム + system linker 呼び出し
- ターゲット v1: `x86_64-apple-darwin`, `aarch64-apple-darwin`, `x86_64-unknown-linux-gnu`
- Mach-O/ELF 直書きは後回し、`cc`/`ld` に委譲

---

## Wasm backend と Native backend の IR 共有方針

### 原則

1. **同一 LoweredModule を入力**: Wasm backend も Native backend も `LoweredModule` を受け取る
2. **Backend 固有の変換は Codegen 内に閉じる**: IR → Wasm 命令、IR → NativeInstr の変換はそれぞれの Codegen モジュール内で行う
3. **IR に Backend 固有の情報を混ぜない**: レジスタ割付、calling convention、ABI 情報は IR に含めない

### パイプライン分岐点

```
FrontendResult
  -> Lowering -> LoweredModule
                    |
                    +-> WasmCodegen -> WasmArtifact (.wasm)
                    |
                    +-> NativeCodegen -> NativeArtifact (.o) -> Linker -> Binary
```

### Bootstrap と配布の使い分け

| 用途 | Backend | 成果物 |
|------|---------|--------|
| Bootstrap (stageN.wasm) | Wasm | `.wasm` (wasmtime で実行) |
| 固定点検証 | Wasm | `stageN.wasm == stageN+1.wasm` |
| 差分比較 | Wasm | Rust compiler vs selfhost compiler |
| エンドユーザー配布 | Native | プラットフォーム別バイナリ |
| CI テスト | Wasm + Native | 両方で同一テスト結果 |

---

## 現状と今後

### 実装済み (P11-1 時点)

- [x] Rust 側: Frontend (parse/infer) -> Lowering (lower) -> WasmCodegen (wasi) のフルパイプライン
- [x] Selfhost 側: Main.ls の `compile-full-pipeline` で 5 ステージ統合 (token/parse/expand/infer/compile)
- [x] E2E テスト: `test_e2e_selfhost_pipeline_complete_stages` で全ステージ通過を検証

### 未実装 (P11-2 以降)

- [ ] Selfhost 側の Lowering を IR.ls/Compiler.ls の正式版に統合
- [ ] Selfhost 側の WasmCodegen を WasmEmit.ls の正式版に統合
- [ ] Native backend の設計・実装 (P11-2b)
- [ ] Backend 境界の型安全性を L# の型システムで保証
