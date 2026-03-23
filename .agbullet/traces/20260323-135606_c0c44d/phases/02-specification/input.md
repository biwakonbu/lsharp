# Phase 2: 仕様策定 - Input

## 1. From Previous Phase
- Source: Phase 1 (information_gathering)
- Reference: .agbullet/tasks/20260323-135606_c0c44d/RESEARCH.md

## 2. Task
TODO.md の全Phase (0-9) の実装を完了する。
- Phase 0: 基盤整備 (lower.rsリファクタリング、Bump Allocator、メモリ操作IR、タグ付きワード)
- Phase 1: 文字列操作
- Phase 2: 動的コレクション (ADT、Vector、HashMap)
- Phase 3: クロージャ
- Phase 4: エラー処理 & ミュータビリティ
- Phase 5: File I/O & WASI拡張
- Phase 6: マルチファイルコンパイル
- Phase 7: 標準ライブラリ
- Phase 8: セルフホスティング
- Phase 9: エコシステム

## 3. Target Files
- crates/lsharp-ir/src/lower.rs (1996行、分割対象)
- crates/lsharp-ir/src/lib.rs (IR定義、メモリ操作命令追加先)
- crates/lsharp-wasm/src/wasi.rs (Bump Allocator、WASI import追加先)
- crates/lsharp-wasm/src/emit.rs (命令変換追加先)
- crates/lsharp-wasm/tests/e2e.rs (E2Eテスト)
- crates/lsharp-ir/src/module_graph.rs (モジュールグラフ)
- crates/lsharp-wasm/src/wasi_runner.rs (WASIランナー)
- crates/lsharp-syntax/src/ast.rs (AST定義)
- crates/lsharp-syntax/src/parser.rs (パーサー)
- crates/lsharp-types/src/infer.rs (型推論)

## 4. Architecture
6クレートの Cargo ワークスペース。コンパイラパイプライン: Lexer -> Parser -> 型推論 -> IR Lowering -> Wasm Codegen。
リニアメモリベースのランタイムを段階的に構築。

---
Created at: 2026-03-23T13:58:00Z
