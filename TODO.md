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

- [x] カスタムルール定義 API -- AST 走査基盤 (ast-contains-var/ast-count-nodes) に do(tag=9)/match(tag=10) 対応追加、check-unused-var で do/match 内の変数参照を再帰走査、run-all-rules-on-node 一括実行基盤、E2E 6件追加 (do/match 各3件: 直接検索found/not-found + let経由未使用検出)
- [~] LSP 統合 (diagnostics として報告) -- 診断情報構造は LSP Diagnostic 互換 (severity/line/col)、JsonRpc.ls 統合後に LSP publishDiagnostics 対応

#### P9-6d: フォーマッタ (L# 実装) (残タスク)
> 完了済み: AST プリティプリンタ + インデント設定 + CLI fmt → ADR-141 参照

- [x] LSP textDocument/formatting ハンドラ統合 (Rust LSP) -- lsharp-lsp/format.rs の format_source + lib.rs の formatting() メソッド実装済み、document_formatting_provider capabilities 登録済み、ユニットテスト 5件
- [~] LSP textDocument/formatting (L# 実装) -- Formatter.ls のフォーマット関数は定義済み、L# 製 LSP (JsonRpc.ls) 統合は P9-6b ハンドラ完成後

---

## Phase 11: Rust 完全撤去

> 目標: L# 製 compiler/toolchain をネイティブ配布の正式実装に昇格し、Rust workspace を段階的に撤去する
> 配布方針: ブートストラップと比較検証では Wasm/WASI を利用してよいが、エンドユーザー向け正式配布物は各プラットフォーム向けネイティブバイナリとする
> 正式完了条件:
> 1. `stageN.wasm` が selfhost compiler として `stageN+1.wasm` を生成できる
> 2. `stageN.wasm == stageN+1.wasm` の固定点が CI で安定する
> 3. Rust CLI/LSP/docs 系の公開機能が L# 側で互換提供され、ネイティブ版 toolchain から利用できる
> 4. 長寿命プロセス (LSP/REPL/server mode) で GC 有効時にメモリが単調増加しない
> 5. Rust workspace を削除しても開発・CI・ネイティブ配布が成立する

### P11-1: 正本監査と互換マトリクス
- [ ] `TODO.md` / `README.md` / `book/` と実装の差分を監査し、完了表示の過大評価を是正する
- [ ] Rust 側の公開機能をコマンド・LSP メソッド・入出力仕様単位で棚卸しし、L# 側の対応表を作る
- [ ] 互換対象を `parse/check/compile/build/test/review/doc-ack/doc-check/install/repl/lsp/fmt/doc` と明文化する
- [ ] 完了条件: 「何をもって Rust 完全撤去とみなすか」が TODO 上で曖昧でない

### P11-2: ブートストラップ閉路の完成
> 実装方針: selfhost compiler の正本 IR は維持しつつ、配布用 backend は AOT ネイティブ化する
> ネイティブ化方針: `L# source -> frontend/type/IR -> Native backend -> object file -> platform linker -> native binary`
> 中間運用方針: bootstrap・固定点検証・差分比較には引き続き `stageN.wasm` を使い、最終成果物だけをネイティブ化する

- [ ] selfhost compiler を `Source -> Lexer -> Parser -> MacroExpand -> TypeInfer -> Lower -> WasmEmit` の完全パイプラインに統合する
- [ ] `Main.ls` の暫定的な手動統合をやめ、`import/module` 前提の実モジュール構成で selfhost 全モジュールをコンパイル可能にする
- [ ] `stage1.wasm -> stage2.wasm` で selfhost/stdlib/examples をコンパイルする E2E を追加する
- [ ] `stageN.wasm == stageN+1.wasm` をバイト列比較し、非決定性があれば source map・symbol table・data section の生成順を固定する
- [ ] backend 境界を `FrontendResult -> LoweredModule -> CodegenArtifact` に固定し、Wasm backend と Native backend が同一 Lowered IR を共有する
- [ ] Native backend の最小 v1 を `x86_64-apple-darwin` / `aarch64-apple-darwin` / `x86_64-unknown-linux-gnu` の 3 ターゲットに限定し、Windows/追加 arch は P11-4 の配布整備まで後段化する
- [ ] Native backend v1 の出力形式を「直接実行バイナリ」ではなく「object file + 最小ランタイム + system linker 呼び出し」に固定し、Mach-O/ELF/PE 直書きは後回しにする
- [ ] codegen v1 は整数、bool、文字列、関数呼出し、分岐、ローカル、静的データ、WASI 代替の最小 I/O ランタイムまでを対象にし、GC 依存の高度機能は linear-memory runtime 統合後に解放する

#### P11-2a: Selfhost frontend の閉路化
- [ ] `Main.ls` を分割し、Lexer/Parser/MacroExpand/Infer/Lower/WasmEmit/NativeEmit を import ベースで接続する
- [ ] selfhost compiler が selfhost 自身、stdlib、examples を入力に取れるよう module graph 解決と複数入力のコンパイル順を固定する
- [ ] `compile-selfhost-wasm` と `compile-selfhost-native` の 2 経路を用意し、同一 source から Wasm と native の両成果物を生成できるようにする

#### P11-2b: Native backend / AOT 方式の固定
- [ ] Native IR を新設するのではなく、既存 Lowered IR から `NativeInstr` へ 1 段で落とす方針に固定する
- [ ] calling convention は「L# 関数内部 ABI」と「外部ランタイム ABI」を分離し、v1 では main / print / file I/O / alloc / gc-safe runtime entry だけを外部公開する
- [ ] レジスタ割付は v1 では単純な linear-scan に固定し、spill は stack slot へ落とす
- [ ] object emitter はターゲット別に `text/data/rodata/symbol/relocation` を出力し、リンクは platform linker (`ld`/`clang`/`cc`) に委譲する
- [ ] 文字列定数、関数シンボル、グローバル初期化順を deterministic にし、固定点比較時の差分要因を codegen 側で潰す

#### P11-2c: ランタイム接続
- [ ] Wasm 専用 runtime helper を抽象化し、Native backend から呼べる `alloc/print/read/write/path/clock` の共通 runtime API を定義する
- [ ] ネイティブ runtime v1 は selfhost compiler 実行に必要な最小機能だけを持たせ、スレッド、async、動的ロード、JIT は scope 外にする
- [ ] GC 導入前は bump allocator 互換 runtime で selfhost を成立させ、GC 導入後に同一 runtime API の実装だけを差し替える

#### P11-2d: 検証と固定点
- [ ] `stage1.wasm -> stage2.wasm -> stage3.wasm` の固定点検証を bootstrap の正本とする
- [ ] `stageN.wasm` と `stageN-native` が同じソースに対して同値な観測結果を返す differential test を追加する
- [ ] selfhost/stdlib/examples の Wasm/native 両コンパイル結果に対して、終了コード、stdout、生成物ハッシュ、型エラー出力を比較する
- [ ] Native backend はまず非最適化 (`-O0` 相当) で固定し、性能最適化は固定点と互換性が安定した後に別 Phase で扱う

#### P11-2e: 完了条件
- [ ] Rust を使うのは stage0 生成だけで、stage1 以降の生成・検証・ネイティブ成果物生成は L# 単独で閉じる
- [ ] selfhost compiler が自分自身を native binary として再生成でき、同じ commit 上で bootstrap 経路と native 経路の両方が CI を通る
- [ ] AOT backend の仕様が README/book/TODO で矛盾なく説明されている

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
- [ ] docs/review/knowledge/doc-check/doc-ack/install/repl を L# 側へ移植し、VSCode 拡張のバックエンドを Rust LSP からネイティブな L# 実装へ切り替える
- [ ] macOS/Linux/Windows 向けのネイティブ配布形式、クロスビルド手順、署名/パッケージング方針を固定する
- [ ] 完了条件: エンドユーザーが Rust バイナリにも Wasm ランタイムにも触れずにネイティブ配布物だけで開発フローを完走できる

### P11-5: 長寿命運用のためのランタイム安定化
- [ ] `docs/memory-management-roadmap.md` の M1-M3 を Phase 11 の gate として再接続する
- [ ] compiler/LSP/REPL が共有するヒープオブジェクトに対して GC-safe root 管理を導入する
- [ ] 長寿命 LSP セッション、連続 REPL 実行、自己コンパイル反復で peak memory と回収挙動を測定する
- [ ] 完了条件: bump allocator 前提の短命プロセス設計を脱し、長寿命常駐でも破綻しない

### P11-6: CI 切替と Rust 撤去
- [ ] CI の主経路を `cargo test` 中心から `stageN.wasm` 中心へ切り替える
- [ ] Rust 実装は比較専用ジョブに一時隔離し、fixed-point と golden parity が安定した時点で削除する
- [ ] `Cargo.toml` workspace と `crates/` を削除し、README/book/CI docs を L# ネイティブ正式版前提に更新する
- [ ] ネイティブ release artifact の生成、署名、配布、回帰テストを CI に組み込む
- [ ] 完了条件: リポジトリの正本実装が L# のみになり、Rust 不在で clone 直後から bootstrap とネイティブ配布手順が成立する

---

## 既知の制限事項

### リニアメモリランタイム
- [~] Precise Tracing GC 導入 -- mainline 方針。linear memory 上で shadow stack + mark-sweep を実装。現在の bump allocator (__alloc) は安定動作、GC 導入前のオブジェクトヘッダ/レイアウトの検証テスト 7件追加。docs/memory-management-roadmap.md に Phase 0-6 の詳細ロードマップを記載
- [~] 世代別 GC 最適化 -- docs/memory-management-roadmap.md Phase 4 に設計を記載。young=bump allocator, old=non-moving mark-sweep。First Collector (Phase 3) 完了後に着手
- [~] Region 最適化 -- docs/memory-management-roadmap.md Phase 5 に設計を記載。GC の補助最適化として段階導入
- [~] WasmGC 最適化バックエンド -- docs/memory-management-roadmap.md Phase 6 に設計を記載。optional backend として browser/対応ランタイム向け
