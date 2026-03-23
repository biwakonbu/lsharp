# 実装指示

## メタ情報
- 更新日時: 2026-03-23 15:00:00
- イテレーション: 1
- 対象グループ: Group A (Red Phase - Phase 0 テスト作成)
- 並列ワーカー数: 3

## 1. 並列実行指示

### 1.1 実行モード
**並列実行**: 以下の 3 タスクを同時に実行してください。

### 1.2 Worker 別タスク割り当て

| Worker | タスク ID | タスク内容 | 見積 |
|--------|----------|-----------|------|
| Worker-1 | TEST-001 | P0-0 lower.rs リファクタリング用テストハーネス | small |
| Worker-2 | TEST-002 | P0-1 Bump Allocator 用 E2E テスト | small |
| Worker-3 | TEST-003 | P0-2 メモリ操作 IR 命令用ユニットテスト | small |

### 1.3 並列実行の注意
- 各 Worker のタスクは独立して実行可能
- Worker-1 は `crates/lsharp-ir/src/lower.rs` 内のテストモジュールのみ操作
- Worker-2 は `crates/lsharp-wasm/tests/e2e.rs` にテスト追加
- Worker-3 は `crates/lsharp-ir/src/lib.rs` と `crates/lsharp-wasm/src/emit.rs` にテスト追加
- ファイル競合なし (Worker-2 と Worker-3 は異なるファイルを操作)
- 全 Worker の完了を待ってから Group B へ進む

### 1.4 TDD 原則
**Red Phase**: テストのみ作成する。実装コードは変更しない。テストが FAIL することを確認する。

---

## 2. Worker-1: TEST-001 -- lower.rs リファクタリング用テストハーネス

### 2.1 目標
`lower.rs` を `lower/` ディレクトリに分割した後も既存 422 テストが全パスすることを保証するテストハーネスを作成する。

### 2.2 背景
- 現在の `crates/lsharp-ir/src/lower.rs` は 1996 行で、ファイルサイズ制限 (500-800行) を大幅に超過
- 分割先: `lower/mod.rs`, `lower/expr.rs`, `lower/pattern.rs`, `lower/decl.rs`, `lower/tests.rs`
- 分割後も `Lower` struct の公開 API (`lower_program`) は変更なし
- insta スナップショットテストのパス更新が必要

### 2.3 変更内容

#### ファイル: `crates/lsharp-ir/src/lower.rs`

既存のテストモジュール (`#[cfg(test)] mod tests`) 内に、分割後の構造を検証するテストを追加。

追加するテスト:
1. **`test_lower_module_structure`** -- `Lower::new()` と `lower_program()` が正しく動作し、既存のシンプルなプログラム (整数リテラル、関数定義、if式) が従来通り IR に変換されることを検証
2. **`test_lower_expr_basic`** -- 式の IR 変換 (算術演算、比較演算、let 束縛) が正しく動作することを検証
3. **`test_lower_pattern_match_basic`** -- パターンマッチの IR 生成 (リテラルパターン、変数パターン) が正しく動作することを検証
4. **`test_lower_decl_function`** -- 関数宣言、アクセサ生成、コンストラクタ生成が正しく動作することを検証

これらのテストは現在の lower.rs で PASS するように書く（リファクタリング対象なので、既存コードで動作する形式で書く）。ただし、分割後のモジュール構造 (`lower::expr`, `lower::pattern` 等) からの import パスを使ったテストも 1 つ追加する。この import パステストは分割前は FAIL する。

```rust
// 分割後のモジュール構造を検証するテスト (分割前は FAIL)
#[test]
#[should_panic] // 分割前はモジュールが存在しないため panic
fn test_lower_submodules_exist() {
    // 分割後、このテストから #[should_panic] を除去して GREEN にする
    // lower/mod.rs, lower/expr.rs, lower/pattern.rs, lower/decl.rs が存在し
    // Lower struct が pub(crate) でアクセス可能であることを検証
    panic!("lower/ ディレクトリへの分割が未実装");
}
```

### 2.4 完了条件
- [ ] 既存 422 テストが全て PASS
- [ ] `test_lower_submodules_exist` が `#[should_panic]` 付きで PASS (= 分割前なので panic する)
- [ ] `cargo test -p lsharp-ir` で全テスト PASS

---

## 3. Worker-2: TEST-002 -- Bump Allocator 用 E2E テスト

### 3.1 目標
Bump Allocator (`__alloc` 関数) の動作を検証する E2E テストを作成する。`__alloc` は未実装なのでテストは FAIL する。

### 3.2 背景
- 現在メモリ管理は未実装: 固定アドレスレイアウトのみ (0-511: 予約領域、512~: 文字列定数)
- `__alloc(size: i32) -> i32` はグローバル `$heap_ptr` を使った Bump Allocator
- 8 バイトアラインメント、`memory.grow` による自動ページ拡張
- `__alloc` は wasi.rs にインライン Wasm 関数として埋め込む予定

### 3.3 変更内容

#### ファイル: `crates/lsharp-wasm/tests/e2e.rs`

以下の E2E テストを追加:

```rust
// --- Phase 0: Bump Allocator テスト ---

#[test]
fn test_e2e_alloc_basic() {
    // __alloc を呼び出してメモリアドレスを取得
    // 返り値がヒープ領域の先頭 (文字列定数データの後) であることを検証
    let result = compile_and_run(r#"
        (defn main () Int
          (let ((addr (__alloc 16)))
            (print addr)
            addr))
    "#);
    // __alloc が返すアドレスは文字列定数データ末尾以降
    // 具体的なアドレスは文字列定数の有無に依存するが、
    // 少なくとも 512 以上であることを期待
    let addr: i64 = result.trim().parse().unwrap();
    assert!(addr >= 512, "heap address should be >= 512, got {}", addr);
}

#[test]
fn test_e2e_alloc_alignment() {
    // 複数の __alloc 呼び出しで 8 バイトアラインメントを検証
    let result = compile_and_run(r#"
        (defn main () Int
          (let ((a1 (__alloc 1))
                (a2 (__alloc 1)))
            (print a1)
            (print a2)
            (- a2 a1)))
    "#);
    let lines: Vec<&str> = result.trim().lines().collect();
    let a1: i64 = lines[0].parse().unwrap();
    let a2: i64 = lines[1].parse().unwrap();
    // 8 バイトアラインメント: サイズ 1 でも 8 バイト分確保
    assert_eq!(a2 - a1, 8, "allocations should be 8-byte aligned");
}

#[test]
fn test_e2e_alloc_memory_grow() {
    // 大量のメモリ確保で memory.grow が正しく動作することを検証
    // 1ページ = 64KB = 65536 バイト
    let result = compile_and_run(r#"
        (defn main () Int
          (let ((addr (__alloc 65536)))
            (print addr)
            addr))
    "#);
    let addr: i64 = result.trim().parse().unwrap();
    assert!(addr >= 512, "large allocation should succeed");
}
```

**注意**: `__alloc` はまだビルトイン関数として認識されないため、コンパイルエラーまたはリンクエラーで FAIL する。

### 3.4 完了条件
- [ ] 3 つの E2E テスト (`test_e2e_alloc_basic`, `test_e2e_alloc_alignment`, `test_e2e_alloc_memory_grow`) が追加されている
- [ ] これらのテストが FAIL する (Red Phase)
- [ ] 既存テストは全て PASS のまま
- [ ] `cargo test -p lsharp-wasm test_e2e_alloc` で 3 テストが FAIL

---

## 4. Worker-3: TEST-003 -- メモリ操作 IR 命令用テスト

### 4.1 目標
`Instruction` enum に追加するメモリ操作命令 (I32Load/I32Store/I64Load/I64Store 等) の IR 構築テストと emit 変換テストを作成する。

### 4.2 背景

**既に存在する命令** (lib.rs Instruction enum):
- `I32Const(i32)`, `I32WrapI64`, `I64ExtendI32S`, `I32Eqz`, `I32And`, `I32Or`

**追加が必要な命令**:
- メモリ操作: `I32Load`, `I32Store`, `I32Load8U`, `I32Store8`, `I64Load`, `I64Store`
- 型変換: `I64ExtendI32U` (符号なし拡張。既存の `I64ExtendI32S` は符号付き)
- 算術: `I32Add`, `I32Sub`, `I32Mul`
- 比較: `I32GtU`, `I32GeU`
- ビット操作: `I32Shl`, `I32ShrU`
- メモリ管理: `MemoryGrow`, `MemorySize`

### 4.3 変更内容

#### ファイル: `crates/lsharp-ir/src/lib.rs`

`#[cfg(test)] mod tests` セクションに以下のテストを追加:

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_memory_load_store_instructions() {
        // メモリ操作命令が IR として構築できることを検証
        let instructions = vec![
            Instruction::I32Const(100),      // アドレス
            Instruction::I32Load { offset: 0 },  // 読み出し
            Instruction::I32Const(200),      // アドレス
            Instruction::I32Const(42),       // 値
            Instruction::I32Store { offset: 0 }, // 書き込み
        ];
        assert_eq!(instructions.len(), 5);
        // I32Load/I32Store が Instruction enum に存在することの型レベル検証
    }

    #[test]
    fn test_i64_memory_instructions() {
        let instructions = vec![
            Instruction::I32Const(100),
            Instruction::I64Load { offset: 0 },
            Instruction::I32Const(200),
            Instruction::I64Const(12345),
            Instruction::I64Store { offset: 0 },
        ];
        assert_eq!(instructions.len(), 5);
    }

    #[test]
    fn test_byte_memory_instructions() {
        let instructions = vec![
            Instruction::I32Const(100),
            Instruction::I32Load8U { offset: 0 },
            Instruction::I32Const(200),
            Instruction::I32Const(65),  // 'A'
            Instruction::I32Store8 { offset: 0 },
        ];
        assert_eq!(instructions.len(), 5);
    }

    #[test]
    fn test_i32_arithmetic_instructions() {
        let instructions = vec![
            Instruction::I32Const(10),
            Instruction::I32Const(20),
            Instruction::I32Add,
            Instruction::I32Sub,
            Instruction::I32Mul,
        ];
        assert_eq!(instructions.len(), 5);
    }

    #[test]
    fn test_i32_comparison_instructions() {
        let instructions = vec![
            Instruction::I32Const(10),
            Instruction::I32Const(20),
            Instruction::I32GtU,
            Instruction::I32GeU,
        ];
        assert_eq!(instructions.len(), 4);
    }

    #[test]
    fn test_i32_bitwise_instructions() {
        let instructions = vec![
            Instruction::I32Const(0xFF),
            Instruction::I32Const(4),
            Instruction::I32Shl,
            Instruction::I32ShrU,
        ];
        assert_eq!(instructions.len(), 4);
    }

    #[test]
    fn test_memory_management_instructions() {
        let instructions = vec![
            Instruction::MemorySize,
            Instruction::I32Const(1),
            Instruction::MemoryGrow,
        ];
        assert_eq!(instructions.len(), 3);
    }

    #[test]
    fn test_i64_extend_i32_unsigned() {
        let instructions = vec![
            Instruction::I32Const(42),
            Instruction::I64ExtendI32U,
        ];
        assert_eq!(instructions.len(), 2);
    }

    #[test]
    fn test_instruction_display_memory_ops() {
        // Display trait の検証
        assert_eq!(
            format!("{}", Instruction::I32Load { offset: 0 }),
            "i32.load offset=0"
        );
        assert_eq!(
            format!("{}", Instruction::I32Store { offset: 4 }),
            "i32.store offset=4"
        );
        assert_eq!(
            format!("{}", Instruction::MemoryGrow),
            "memory.grow"
        );
    }
}
```

#### ファイル: `crates/lsharp-wasm/src/emit.rs`

`#[cfg(test)] mod tests` セクションに以下のテストを追加 (emit.rs にテストモジュールがない場合は新規作成):

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use lsharp_ir::Instruction;

    #[test]
    fn test_emit_memory_load() {
        // I32Load が正しい Wasm 命令に変換されることを検証
        let mut func = wasm_encoder::Function::new(vec![]);
        let instructions = vec![
            Instruction::I32Const(100),
            Instruction::I32Load { offset: 0 },
        ];
        let result = emit_instructions_common(
            &mut func, &instructions, |_, _| Ok(())
        );
        assert!(result.is_ok());
    }

    #[test]
    fn test_emit_memory_store() {
        let mut func = wasm_encoder::Function::new(vec![]);
        let instructions = vec![
            Instruction::I32Const(100),
            Instruction::I32Const(42),
            Instruction::I32Store { offset: 0 },
        ];
        let result = emit_instructions_common(
            &mut func, &instructions, |_, _| Ok(())
        );
        assert!(result.is_ok());
    }

    #[test]
    fn test_emit_memory_grow() {
        let mut func = wasm_encoder::Function::new(vec![]);
        let instructions = vec![
            Instruction::I32Const(1),
            Instruction::MemoryGrow,
        ];
        let result = emit_instructions_common(
            &mut func, &instructions, |_, _| Ok(())
        );
        assert!(result.is_ok());
    }

    #[test]
    fn test_emit_i32_arithmetic() {
        let mut func = wasm_encoder::Function::new(vec![]);
        let instructions = vec![
            Instruction::I32Const(10),
            Instruction::I32Const(20),
            Instruction::I32Add,
            Instruction::I32Sub,
            Instruction::I32Mul,
        ];
        let result = emit_instructions_common(
            &mut func, &instructions, |_, _| Ok(())
        );
        assert!(result.is_ok());
    }
}
```

### 4.4 完了条件
- [ ] `crates/lsharp-ir/src/lib.rs` に 9 個のユニットテストが追加されている
- [ ] `crates/lsharp-wasm/src/emit.rs` に 4 個のユニットテストが追加されている
- [ ] これらのテストがコンパイルエラーで FAIL する (命令バリアントが未定義)
- [ ] 既存テストは全て PASS のまま

---

## 5. 共通参照ファイル

### 5.1 必須参照
| ファイル | 参照目的 |
|---------|---------|
| `crates/lsharp-ir/src/lib.rs` | Instruction enum の現在の定義を確認 (I32Const, I32WrapI64, I32And, I32Or は既に存在) |
| `crates/lsharp-ir/src/lower.rs` | Lower struct、lower_program() の現在の構造を確認 |
| `crates/lsharp-wasm/src/emit.rs` | emit_instructions_common() の現在の実装を確認 |
| `crates/lsharp-wasm/tests/e2e.rs` | compile_and_run() ヘルパーの使い方を確認 |
| `crates/lsharp-wasm/src/wasi.rs` | メモリレイアウト定数 (NEWLINE_ADDR, IOV_ADDR 等) を確認 |

### 5.2 パターン参照
| ファイル | 参照目的 |
|---------|---------|
| `crates/lsharp-wasm/tests/e2e.rs` の既存テスト | E2E テストの書き方パターン |
| `crates/lsharp-ir/src/lib.rs` の既存 Instruction バリアント | 新しい命令バリアントの命名規約 |

## 6. 制約・注意点

### 6.1 技術的制約
- Rust Edition 2024 を使用
- `wasm-encoder 0.245`, `wasmtime 29` に依存
- ファイルサイズは 500-800 行以内に収める
- テストファイルへの追加は既存テストの後に配置する

### 6.2 並列実行時の注意
- Worker-1 は `crates/lsharp-ir/src/lower.rs` のテストモジュールのみ操作
- Worker-2 は `crates/lsharp-wasm/tests/e2e.rs` のみ操作
- Worker-3 は `crates/lsharp-ir/src/lib.rs` と `crates/lsharp-wasm/src/emit.rs` のテストモジュールのみ操作
- 共有ファイルの同時編集は発生しない

### 6.3 禁止事項
- 実装コードの変更 (Red Phase のためテストのみ)
- 既存テストの修正・削除
- `Instruction` enum への新バリアント追加 (Worker-3 はテストのみ。実装は Group B で行う)
- `__alloc` 関数の実装 (Worker-2 はテストのみ)

### 6.4 命名規約
- テスト関数名: `test_e2e_` プレフィックス (E2E テスト)、`test_` プレフィックス (ユニットテスト)
- コメント: 日本語
- 変数・関数名: 英語 (snake_case)

### 6.5 lib.rs の既存 Instruction バリアント (Worker-3 用)
以下のバリアントは **既に存在する** ため、テストで使用可能:
- `I32Const(i32)`, `I32WrapI64`, `I64ExtendI32S`, `I32Eqz`, `I32And`, `I32Or`

以下のバリアントは **未定義** のため、テストはコンパイルエラーになることを期待:
- `I32Load { offset: u32 }`, `I32Store { offset: u32 }`
- `I32Load8U { offset: u32 }`, `I32Store8 { offset: u32 }`
- `I64Load { offset: u32 }`, `I64Store { offset: u32 }`
- `I64ExtendI32U`, `I32Add`, `I32Sub`, `I32Mul`
- `I32GtU`, `I32GeU`, `I32Shl`, `I32ShrU`
- `MemoryGrow`, `MemorySize`

## 7. 追加課題の報告指示

**全 Worker に以下の報告を必須とする:**

タスク実行中に発見した課題は必ず結果に含めること。

### 7.1 報告対象
- バグや不整合
- 追加で必要な変更
- テスト不足
- リファクタリングが必要な箇所
- セキュリティ上の懸念

### 7.2 報告形式
```yaml
discovered_issues:
  - id: "ISSUE-XXX"
    description: "課題の説明"
    severity: "critical" | "major" | "minor"
    related_task: "TEST-XXX"
```

### 7.3 報告がない場合
課題がなければ `discovered_issues: []` を返却。

## 8. 全体完了条件

### 8.1 各 Worker の完了条件
- [ ] Worker-1: TEST-001 -- lower.rs テストハーネス追加、`test_lower_submodules_exist` が `#[should_panic]` で PASS
- [ ] Worker-2: TEST-002 -- Bump Allocator E2E テスト 3 個追加、全て FAIL (Red Phase)
- [ ] Worker-3: TEST-003 -- メモリ操作 IR テスト 9 個 (lib.rs) + emit テスト 4 個 (emit.rs) 追加、全てコンパイルエラーで FAIL

### 8.2 グループ完了条件
- [ ] 全 Worker のタスクが完了
- [ ] 既存 422 テストが全て PASS
- [ ] 新規テストが意図通りに FAIL (Red Phase 確認)

## 9. 実行後の報告

完了後、Worker 毎に以下を報告:
1. 完了した Worker ID とタスク ID
2. 変更したファイルのリスト
3. 各変更の概要
4. 発生した問題 (あれば)
5. **発見した追加課題** (discovered_issues)

**報告フォーマット**:
```yaml
worker_results:
  - worker: Worker-1
    task_id: TEST-001
    status: completed
    files_changed: [...]
    discovered_issues: []
  - worker: Worker-2
    task_id: TEST-002
    status: completed
    files_changed: [...]
    discovered_issues: []
  - worker: Worker-3
    task_id: TEST-003
    status: completed
    files_changed: [...]
    discovered_issues: []
```
