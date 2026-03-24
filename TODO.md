# L# セルフホスティング & エコシステム TODO

> 凡例: `[x]` 完了 / `[ ]` 未着手 / `[~]` 部分実装 / `[BLOCKED: ...]` 依存待ち
>
> **完了済みフェーズ**: Phase 0-7, P8-1~P8-8, P9-1/2/3/4, P9-6a, P10, BUG/IMP/QA/CI は完了。
> 詳細は `docs/adr/decisions-001.jsonl` (ADR-093〜ADR-132) および `docs/adr/decisions-002.jsonl` (ADR-133〜ADR-147) を参照

---

## Phase 8: セルフホスティング

### P8-9: ブートストラップ検証 (残タスク)
> 完了済み: T4-1~T4-3, T4-6 → ADR-139 参照

- [~] T4-4: stage1.wasm → stage2.wasm (セルフコンパイル) -- ミニトークナイザー+ミニパーサーによる Source→Token→AST→IR パイプライン実装済み (MVP: `(defn main [] 42)` のソースからコンパイル成功)、E2E 検証 7行追加 (test_e2e_selfhost_main_integration, test_e2e_bootstrap_stage1_integration)。完全セルフコンパイルは Lexer.ls/Parser.ls の完全統合後
- [~] T4-5: stage1.wasm == stage2.wasm (固定点検証) -- stage1 バイナリ構造の検証テスト追加済み、完全固定点検証は T4-4 完了後

---

## Phase 9: エコシステム

### P9-6: VSCode 拡張 (L# ネイティブ) (残タスク)
> 完了済み: P9-6a (シンタックスハイライト) → ADR-141 参照

#### P9-6b: LSP サーバー (L# 実装)
- [~] LSP プロトコルハンドラ: initialize / textDocument/didOpen / didChange -- JSON-RPC メッセージ構造は JsonRpc.ls で定義済み、実際のハンドラ実装は Lexer/Parser 統合後
- [~] 診断発行 (parse エラー + 型エラー → LSP Diagnostic) -- Linter.ls で診断情報構造を定義済み (severity/rule-id/line/col)、LSP 連携は P9-6b ハンドラ完成後
- [~] 定義ジャンプ (selfhost/AST.ls + シンボルテーブル) -- Rust 版 LSP (lsharp-lsp) で実装済み、L# 版は AST.ls のシンボル解決拡張後
- [~] 型ホバー (selfhost/Type.ls + TypeScheme.ls 活用) -- Rust 版 LSP で実装済み、L# 版は Type.ls/TypeScheme.ls の型表示関数追加後
- [~] 補完 (シンボル補完 + キーワード補完) -- Rust 版 LSP で実装済み、L# 版は JsonRpc.ls + Lexer.ls 統合後

#### P9-6c: リンター (L# 実装) (残タスク)
> 完了済み: AST リントルール基盤 + 組み込みルール5種 → ADR-141 参照

- [~] カスタムルール定義 API -- AST 走査基盤実装済み (ast-is-leaf/ast-contains-var/ast-count-nodes)、check-unused-var ルール実装済み (let 束縛の未使用検出)、run-all-rules-on-node 一括実行基盤実装済み、E2E 4件追加。完全な AST walker (全ノードタイプ走査) は do/match 対応後
- [~] LSP 統合 (diagnostics として報告) -- 診断情報構造は LSP Diagnostic 互換 (severity/line/col)、JsonRpc.ls 統合後に LSP publishDiagnostics 対応

#### P9-6d: フォーマッタ (L# 実装) (残タスク)
> 完了済み: AST プリティプリンタ + インデント設定 + CLI fmt → ADR-141 参照

- [~] LSP textDocument/formatting ハンドラ統合 -- Formatter.ls のフォーマット関数は定義済み、LSP 連携は JsonRpc.ls 統合後

---

## Phase 11: Rust 完全撤去

> 目標: L# 製 compiler/toolchain を WASI 単体配布の正式実装に昇格し、Rust workspace を段階的に撤去する
> 正式完了条件:
> 1. `stageN.wasm` が selfhost compiler として `stageN+1.wasm` を生成できる
> 2. `stageN.wasm == stageN+1.wasm` の固定点が CI で安定する
> 3. Rust CLI/LSP/docs 系の公開機能が L# 側で互換提供される
> 4. 長寿命プロセス (LSP/REPL/server mode) で GC 有効時にメモリが単調増加しない
> 5. Rust workspace を削除しても開発・CI・配布が成立する

### P11-1: 正本監査と互換マトリクス
- [ ] `TODO.md` / `README.md` / `book/` と実装の差分を監査し、完了表示の過大評価を是正する
- [ ] Rust 側の公開機能をコマンド・LSP メソッド・入出力仕様単位で棚卸しし、L# 側の対応表を作る
- [ ] 互換対象を `parse/check/compile/build/test/review/doc-ack/doc-check/install/repl/lsp/fmt/doc` と明文化する
- [ ] 完了条件: 「何をもって Rust 完全撤去とみなすか」が TODO 上で曖昧でない

### P11-2: ブートストラップ閉路の完成
- [ ] selfhost compiler を `Source -> Lexer -> Parser -> MacroExpand -> TypeInfer -> Lower -> WasmEmit` の完全パイプラインに統合する
- [ ] `Main.ls` の暫定的な手動統合をやめ、`import/module` 前提の実モジュール構成で selfhost 全モジュールをコンパイル可能にする
- [ ] `stage1.wasm -> stage2.wasm` で selfhost/stdlib/examples をコンパイルする E2E を追加する
- [ ] `stageN.wasm == stageN+1.wasm` をバイト列比較し、非決定性があれば source map・symbol table・data section の生成順を固定する
- [ ] 完了条件: Rust を使うのは stage0 生成だけで、stage1 以降の生成と検証は L# 単独で閉じる

### P11-3: コンパイラ中核の Rust parity
- [ ] `crates/lsharp-syntax` 相当の機能を L# に移植する。対象は span/token/AST/衛生マクロ/derive/macro expansion を含む
- [ ] `crates/lsharp-types` 相当の機能を L# に移植する。対象は HM 推論、制約、高度型、metadata check、型表示まで含む
- [ ] `crates/lsharp-ir` 相当の機能を L# に移植する。対象は multi-file/module graph、lowering、closure 変換、pattern lowering を含む
- [ ] `crates/lsharp-wasm` 相当の機能を L# に移植する。対象は codegen、WASI runtime、test runner、snapshot 対応を含む
- [ ] Rust 実装との比較はフェーズごとの golden test で維持し、削除直前に全差分を解消する
- [ ] 完了条件: Rust crate 群を参照しなくても既存 examples/stdlib/selfhost が同一意味で通る

### P11-4: ツールチェイン parity
- [ ] L# 製 CLI を正式化し、現行サブコマンド互換の引数仕様と終了コードを固定する
- [ ] L# 製 LSP を正式化し、`initialize/didOpen/didChange/hover/definition/references/rename/formatting/completion/shutdown` を実装する
- [ ] L# 製 formatter/linter を AST 全体対応に拡張し、CLI と LSP の両経路で同一結果を返す
- [ ] docs/review/knowledge/doc-check/doc-ack/install/repl を L# 側へ移植し、VSCode 拡張のバックエンドを Rust LSP から `stageN.wasm` 起動へ切り替える
- [ ] 完了条件: エンドユーザーが Rust バイナリを一切触らずに開発フローを完走できる

### P11-5: 長寿命運用のためのランタイム安定化
- [ ] `docs/memory-management-roadmap.md` の M1-M3 を Phase 11 の gate として再接続する
- [ ] compiler/LSP/REPL が共有するヒープオブジェクトに対して GC-safe root 管理を導入する
- [ ] 長寿命 LSP セッション、連続 REPL 実行、自己コンパイル反復で peak memory と回収挙動を測定する
- [ ] 完了条件: bump allocator 前提の短命プロセス設計を脱し、長寿命常駐でも破綻しない

### P11-6: CI 切替と Rust 撤去
- [ ] CI の主経路を `cargo test` 中心から `stageN.wasm` 中心へ切り替える
- [ ] Rust 実装は比較専用ジョブに一時隔離し、fixed-point と golden parity が安定した時点で削除する
- [ ] `Cargo.toml` workspace と `crates/` を削除し、README/book/CI docs を L# 正式版前提に更新する
- [ ] 完了条件: リポジトリの正本実装が L# のみになり、Rust 不在で clone 直後から bootstrap 手順が成立する

---

## 既知の制限事項

### リニアメモリランタイム
- [~] Precise Tracing GC 導入 -- mainline 方針。linear memory 上で shadow stack + mark-sweep を実装。現在の bump allocator (__alloc) は安定動作、GC 導入前のオブジェクトヘッダ/レイアウトの検証テスト 7件追加。docs/memory-management-roadmap.md に Phase 0-6 の詳細ロードマップを記載
- [~] 世代別 GC 最適化 -- docs/memory-management-roadmap.md Phase 4 に設計を記載。young=bump allocator, old=non-moving mark-sweep。First Collector (Phase 3) 完了後に着手
- [~] Region 最適化 -- docs/memory-management-roadmap.md Phase 5 に設計を記載。GC の補助最適化として段階導入
- [~] WasmGC 最適化バックエンド -- docs/memory-management-roadmap.md Phase 6 に設計を記載。optional backend として browser/対応ランタイム向け
