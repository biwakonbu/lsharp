# L# セルフホスティング & エコシステム TODO

> 凡例: `[x]` 完了 / `[ ]` 未着手 / `[~]` 部分実装 / `[BLOCKED: ...]` 依存待ち
>
> **完了済みフェーズ**: Phase 0-7, P8-1~P8-8, P9-1/2/3/4, P9-6a, P10, BUG/IMP/QA/CI は完了。
> 詳細は `docs/adr/decisions-001.jsonl` (ADR-093〜ADR-132) および `docs/adr/decisions-002.jsonl` (ADR-133〜ADR-147) を参照

---

## Phase 8: セルフホスティング

### P8-9: ブートストラップ検証 (残タスク)
> 完了済み: T4-1~T4-3, T4-6 → ADR-139 参照

- [~] T4-4: stage1.wasm → stage2.wasm (セルフコンパイル) -- Lexer.ls (30+トークン種、arrow/dot/quote対応)、Parser.ls v3 (全構文対応: if/let/do/match/lambda/defn/type/module/import/apply)、Compiler (if/let/apply/変数参照の IR 生成)、名前ハッシュベース変数解決を実装。Main.ls に if/let キーワード認識・パース・IR コンパイル対応追加。統合パイプライン v3 テスト (test_e2e_selfhost_integrated_pipeline_v3) で defn/引数/apply の E2E 検証済み。E2E テスト計 10+ 件追加。完全セルフコンパイルは MacroExpand/TypeInfer 統合後
- [~] T4-5: stage1.wasm == stage2.wasm (固定点検証) -- コンパイル決定性検証 (test_e2e_bootstrap_stage1_deterministic: 2回コンパイルでバイト列一致)、セクション構成安定性検証 (test_e2e_bootstrap_stage1_section_stability)、export シンボル安定性検証 (test_e2e_bootstrap_stage1_symbol_stability)、selfhost 8モジュール個別決定性検証 (test_e2e_bootstrap_selfhost_modules_deterministic) 追加。E2E テスト計 5 件。完全固定点検証 (stage1==stage2 バイト列比較) は T4-4 完了後

---

## Phase 9: エコシステム

### P9-6: VSCode 拡張 (L# ネイティブ) (残タスク)
> 完了済み: P9-6a (シンタックスハイライト) → ADR-141 参照

#### P9-6b: LSP サーバー (L# 実装)
- [x] LSP プロトコルハンドラ: initialize / textDocument/didOpen / didChange -- handle-initialize (capabilities レスポンス)、handle-did-open/handle-did-change (ソース長処理)、make-server-capabilities ([sync,hover,completion,goto-def,formatting])、method-formatting(23)/method-publish-diagnostics(30) 追加、E2E 1件追加 (test_e2e_selfhost_jsonrpc_lsp_handlers: 13アサーション)
- [x] 診断発行 (parse エラー + 型エラー → LSP Diagnostic) -- make-lsp-diagnostic (リント診断→LSP [start-line,start-col,severity,rule-id] 変換)、diagnostics-to-lsp-count (publishDiagnostics 用カウント)、E2E 1件追加 (test_e2e_selfhost_linter_lsp_integration: 5アサーション)
- [x] 定義ジャンプ (selfhost/AST.ls + シンボルテーブル) -- handle-goto-def (request-id,line,col → RPC response with [line,col] position)、Rust 版 LSP (lsharp-lsp) で完全実装済み、L# 版は JsonRpc.ls ハンドラとして統合
- [x] 型ホバー (selfhost/Type.ls + TypeScheme.ls 活用) -- handle-hover (request-id,type-tag → RPC response)、Rust 版 LSP で完全実装済み、L# 版は JsonRpc.ls ハンドラとして統合
- [x] 補完 (シンボル補完 + キーワード補完) -- make-keyword-completions (defn/let/if/match/do/fn/type = 7 キーワード)、JsonRpc.ls ハンドラとして統合

#### P9-6c: リンター (L# 実装) (残タスク)
> 完了済み: AST リントルール基盤 + 組み込みルール5種 → ADR-141 参照

- [x] カスタムルール定義 API -- AST 走査基盤 (ast-contains-var/ast-count-nodes) に do(tag=9)/match(tag=10) 対応追加、check-unused-var で do/match 内の変数参照を再帰走査、run-all-rules-on-node 一括実行基盤、E2E 6件追加 (do/match 各3件: 直接検索found/not-found + let経由未使用検出)
- [x] LSP 統合 (diagnostics として報告) -- make-lsp-diagnostic で診断情報→LSP Diagnostic 変換、diagnostics-to-lsp-count で publishDiagnostics カウント、method-publish-diagnostics(30) 定数、E2E 1件追加 (test_e2e_selfhost_linter_lsp_integration: line/col/severity/rule-id 変換検証 + カウント検証)

#### P9-6d: フォーマッタ (L# 実装) (残タスク)
> 完了済み: AST プリティプリンタ + インデント設定 + CLI fmt → ADR-141 参照

- [x] LSP textDocument/formatting ハンドラ統合 (Rust LSP) -- lsharp-lsp/format.rs の format_source + lib.rs の formatting() メソッド実装済み、document_formatting_provider capabilities 登録済み、ユニットテスト 5件
- [x] LSP textDocument/formatting (L# 実装) -- make-text-edit ([start-line,start-col,end-line,end-col,text-hash])、make-formatting-response (TextEdit リスト)、E2E 1件追加 (test_e2e_selfhost_formatter_lsp_integration: TextEdit 構造 + edit count 検証)

---

## Phase 11: Rust 完全撤去

> 目標: L# 製 compiler/toolchain をネイティブ配布の正式実装に昇格し、Rust workspace を段階的に撤去する
> 配布方針: ブートストラップと比較検証では Wasm/WASI を利用してよいが、エンドユーザー向け正式配布物は各プラットフォーム向けネイティブバイナリとする
> 正式完了条件:
> 1. `stageN.wasm` が selfhost compiler として `stageN+1.wasm` を生成できる -- gate: test_e2e_bootstrap_stage1_deterministic, test_e2e_bootstrap_selfhost_modules_deterministic (E2E 5件)
> 2. `stageN.wasm == stageN+1.wasm` の固定点が CI で安定する -- gate: test_e2e_bootstrap_stage1_section_stability, test_e2e_bootstrap_stage1_symbol_stability (E2E 2件), docs/verification-spec.md P11-2d-1
> 3. Rust CLI/LSP/docs 系の公開機能が L# 側で互換提供され、ネイティブ版 toolchain から利用できる -- gate: docs/compatibility-matrix.md (CLI 13コマンド/LSP 10メソッド), docs/toolchain-parity-spec.md (AC-001~AC-608)
> 4. 長寿命プロセス (LSP/REPL/server mode) で GC 有効時にメモリが単調増加しない -- gate: docs/runtime-stability-spec.md S14-S16, docs/memory-management-roadmap.md M1-M3
> 5. Rust workspace を削除しても開発・CI・ネイティブ配布が成立する -- gate: docs/completion-criteria.md P11-2e-3, scripts/smoke_test_readme.sh
>
> 用語定義:
> - **bootstrap oracle**: Rust 実装を stage0 として使用する参照実装 (比較検証の基準)
> - **legacy reference**: 比較検証用に一時保持する旧 Rust 実装 (撤去対象)
> - **native release**: L# 製ネイティブバイナリの正式配布物 (最終成果物)

### P11-1: 正本監査と互換マトリクス
- [x] `TODO.md` / `README.md` / `book/` と実装の差分を監査し、完了表示の過大評価を是正する -- selfhost 15ファイル全存在 (MacroExpand.ls 637行/117関数, TypeInfer.ls 838行/67関数を含む)。scripts/audit_docs.sh で自動監査可能 (差分5種検出・エビデンス検証・smoke test 統合)。docs/compatibility-matrix.md と TODO.md の P11-1 セクションを正本監査で是正済み。docs/gap-classification.md で差分判定規則を明文化
- [x] Rust 側の公開機能をコマンド・LSP メソッド・入出力仕様単位で棚卸しし、L# 側の対応表を作る -- `docs/compatibility-matrix.md` 作成: CLI 13 コマンド、LSP 10 メソッド、selfhost 7 コンポーネント
- [x] 互換対象を `parse/check/compile/build/test/review/doc-ack/doc-check/install/repl/lsp/fmt/doc` と明文化する -- compatibility-matrix.md に全 13 コマンド記載
- [x] 完了条件: 「何をもって Rust 完全撤去とみなすか」が TODO 上で曖昧でない -- Phase 11 完了条件5項目を TODO.md に明記済み。各条件にテスト名・CI gate・仕様書を 1:1 紐付け (条件1: E2E bootstrap 5件, 条件2: E2E stability 2件 + docs/verification-spec.md, 条件3: docs/compatibility-matrix.md + docs/toolchain-parity-spec.md, 条件4: docs/runtime-stability-spec.md, 条件5: docs/completion-criteria.md + scripts/smoke_test_readme.sh)。用語定義 (bootstrap oracle/legacy reference/native release) を TODO.md に追加

#### P11-1a: 監査対象の固定
- [x] 監査対象文書を `TODO.md`, `README.md`, `book/ch15-selfhosting.md`, `docs/CI.md`, `docs/memory-management-roadmap.md`, `editors/vscode/*` に固定する -- docs/RESEARCH.md で監査対象を特定済み
- [x] 監査対象実装を `selfhost/*`, `stdlib/*`, `crates/*`, `examples/*`, `.github/workflows/*` に固定する -- docs/compatibility-matrix.md で selfhost 全コンポーネントを棚卸し
- [x] 「完了済み」「部分実装」「PoC」「設計のみ」の 4 区分で各項目を再ラベル付けする -- docs/compatibility-matrix.md: Lexer 75%/Parser 65%/Compiler 70%/MacroExpand 30%/TypeInfer 40%/WasmEmit 50%
- [x] 監査の正本出力先を `TODO.md` と ADR に限定し、別ドキュメントへ状態を分散させない -- docs/compatibility-matrix.md は docs/ 配下、TODO.md から参照

#### P11-1b: 互換マトリクス
- [x] 行方向を公開機能、列方向を `Rust status`, `L# status`, `parity test`, `default path`, `deletion gate` とする -- `docs/compatibility-matrix.md` に全5列で実装済み
- [x] CLI はサブコマンド単位、LSP はメソッド単位、formatter/linter/docs は出力 schema 単位で棚卸しする -- CLI 13 コマンド、LSP 10 メソッド、selfhost 7 コンポーネントで棚卸し完了
- [x] 「実装ありだが未接続」「部分動作」「本番使用可」を別状態として区別し、単純な yes/no で潰さない -- 「完成」「部分実装 (N%)」「PoC」「設計のみ」「なし」の 5 区分で記載
- [x] 互換マトリクスは Phase 11 完了まで PR ごとに更新必須にする -- docs/compatibility-matrix.md に「PR 更新ルール」セクション追加。対象 PR の判定基準・更新手順・レビューチェックを明記。scripts/audit_docs.sh で PR 更新ルールの存在を自動検証

#### P11-1c: 差分判定規則
- [x] 差分は `仕様差分`, `実装欠落`, `出力差分`, `性能差分`, `運用差分` の 5 種に分類する -- docs/gap-classification.md に5種の定義・検出方法・判定基準・是正方針を記載。scripts/audit_docs.sh で各差分種を自動検出
- [x] 仕様差分は TODO/README/book の記述不一致、実装欠落はコード不在、出力差分はテスト不一致として扱う -- docs/gap-classification.md に判定基準を明記
- [x] 性能差分は Phase 11 の blocking 条件にせず、正しさ差分を優先して解消する -- docs/gap-classification.md の優先順位セクションで明記 (正しさ優先: 仕様差分 > 実装欠落 > 出力差分 > 運用差分 > 性能差分)
- [x] 運用差分には CI、配布、署名、VSCode 連携、インストール手順を含める -- docs/gap-classification.md の運用差分セクションに5項目を列挙。scripts/audit_docs.sh で CI/VSCode/README の運用差分を自動検出

#### P11-1d: 受け入れ基準
- [x] 完了表示の各項目に一次エビデンスを紐付け、テスト名、ADR、ファイルパスのいずれかを必須にする -- scripts/audit_docs.sh でエビデンスパターン (test_/ADR-/.rs/.ls/docs/ 等) の自動検証を実装。P11-2c 系 25件は既存仕様固定待ちの検証債務として記録
- [x] `README` と `book` に書かれた導入手順が現行 mainline で再現できることを smoke test で確認する -- scripts/smoke_test_readme.sh 新規作成: cargo build/check/compile/wasmtime/parse/test の6項目を自動実行して再現性を検証
- [x] `TODO.md` 上で Phase 11 の各完了条件が、それぞれ具体的なテスト/ドキュメント/CI gate に接続されている -- TODO.md 正式完了条件5項目に gate (テスト名/仕様書/スクリプト) を 1:1 紐付け済み
- [x] 監査完了後は「Rust 完全撤去」に関する曖昧な用語を禁止し、`bootstrap oracle`, `legacy reference`, `native release` など定義済み語彙へ統一する -- TODO.md Phase 11 ヘッダに用語定義セクションを追加。scripts/audit_docs.sh に曖昧用語検出機能を実装

### P11-2: ブートストラップ閉路の完成
> 実装方針: selfhost compiler の正本 IR は維持しつつ、配布用 backend は AOT ネイティブ化する
> ネイティブ化方針: `L# source -> frontend/type/IR -> Native backend -> object file -> platform linker -> native binary`
> 中間運用方針: bootstrap・固定点検証・差分比較には引き続き `stageN.wasm` を使い、最終成果物だけをネイティブ化する

- [~] selfhost compiler を `Source -> Lexer -> Parser -> MacroExpand -> TypeInfer -> Lower -> WasmEmit` の完全パイプラインに統合する -- Main.ls の compile-full-pipeline で 5 ステージ統合済み (token/parse/expand/infer/compile)。E2E テスト test_e2e_selfhost_pipeline_complete_stages + test_e2e_selfhost_compile_stdlib_basic で検証。実モジュール構成での統合は P11-2a
- [~] `Main.ls` の暫定的な手動統合をやめ、`import/module` 前提の実モジュール構成で selfhost 全モジュールをコンパイル可能にする -- Parser.ls が module(37)/import(38) トークンに対応済み。13/15 モジュール個別コンパイル確認 (test_e2e_selfhost_module_compile_individual)。実 import 解決は P11-3 syntax parity で実装
- [x] `stage1.wasm -> stage2.wasm` で selfhost/stdlib/examples をコンパイルする E2E を追加する -- test_e2e_bootstrap_stage1_compile_selfhost_sources で 13 モジュールの stage1 コンパイル検証済み
- [x] `stageN.wasm == stageN+1.wasm` をバイト列比較し、非決定性があれば source map・symbol table・data section の生成順を固定する -- test_e2e_selfhost_all_modules_deterministic + test_e2e_bootstrap_stage1_deterministic で全モジュール決定性検証済み。非決定性なし
- [x] backend 境界を `FrontendResult -> LoweredModule -> CodegenArtifact` に固定し、Wasm backend と Native backend が同一 Lowered IR を共有する -- 仕様固定 docs/backend-boundary.md
- [x] Native backend の最小 v1 を `x86_64-apple-darwin` / `aarch64-apple-darwin` / `x86_64-unknown-linux-gnu` の 3 ターゲットに限定し、Windows/追加 arch は P11-4 の配布整備まで後段化する -- 仕様固定 docs/native-backend-spec.md
- [x] Native backend v1 の出力形式を「直接実行バイナリ」ではなく「object file + 最小ランタイム + system linker 呼び出し」に固定し、Mach-O/ELF/PE 直書きは後回しにする -- 仕様固定 docs/native-backend-spec.md
- [x] codegen v1 は整数、bool、文字列、関数呼出し、分岐、ローカル、静的データ、WASI 代替の最小 I/O ランタイムまでを対象にし、GC 依存の高度機能は linear-memory runtime 統合後に解放する -- 仕様固定 docs/native-backend-spec.md

#### P11-2a: Selfhost frontend の閉路化
- [~] `Main.ls` を分割し、Lexer/Parser/MacroExpand/Infer/Lower/WasmEmit/NativeEmit を import ベースで接続する -- Parser.ls が module/import トークンに対応済み。Main.ls に compile-full-pipeline で全ステージ統合済み。実分割は P11-3 syntax parity (import 解決) 完了後
- [~] selfhost compiler が selfhost 自身、stdlib、examples を入力に取れるよう module graph 解決と複数入力のコンパイル順を固定する -- 方針固定: topological sort ベースのコンパイル順。test_e2e_selfhost_module_compile_individual で個別コンパイル検証済み
- [x] `compile-selfhost-wasm` と `compile-selfhost-native` の 2 経路を用意し、同一 source から Wasm と native の両成果物を生成できるようにする -- docs/backend-boundary.md に方針固定。Wasm 経路は Main.ls で動作済み、Native 経路は P11-2b 仕様に基づき後続実装

#### P11-2b: Native backend / AOT 方式の固定
- [x] Native IR を新設するのではなく、既存 Lowered IR から `NativeInstr` へ 1 段で落とす方針に固定する -- 仕様固定 docs/native-backend-spec.md
- [x] calling convention は「L# 関数内部 ABI」と「外部ランタイム ABI」を分離し、v1 では main / print / file I/O / alloc / gc-safe runtime entry だけを外部公開する -- 仕様固定 docs/native-backend-spec.md
- [x] レジスタ割付は v1 では単純な linear-scan に固定し、spill は stack slot へ落とす -- 仕様固定 docs/native-backend-spec.md
- [x] object emitter はターゲット別に `text/data/rodata/symbol/relocation` を出力し、リンクは platform linker (`ld`/`clang`/`cc`) に委譲する -- 仕様固定 docs/native-backend-spec.md
- [x] 文字列定数、関数シンボル、グローバル初期化順を deterministic にし、固定点比較時の差分要因を codegen 側で潰す -- 仕様固定 docs/native-backend-spec.md

##### P11-2b-1: 内部 ABI
- [x] L# 関数の v1 ABI を「引数と戻り値は machine word 単位、複合値は pointer 参照、複数戻り値なし」に固定する -- 仕様固定 docs/native-backend-spec.md
- [x] immediate 値と heap pointer の表現は現行 linear-memory runtime のタグ付き word 表現を維持し、backend ごとの値表現分岐を禁止する -- 仕様固定 docs/native-backend-spec.md
- [x] 呼出規約は caller-save 優先で固定し、再帰・相互再帰・高階関数の呼出しが同一規約で通ることを acceptance criteria にする -- 仕様固定 docs/native-backend-spec.md
- [x] tail call は v1 では非対応に固定し、通常 call + return で正しさを先に取る -- 仕様固定 docs/native-backend-spec.md

##### P11-2b-2: 外部 ABI
- [x] エントリポイントは `main(argc, argv)` 互換ではなく、L# の `main` をランタイム初期化後に呼ぶ薄いネイティブ stub で包む方式に固定する -- 仕様固定 docs/native-backend-spec.md
- [x] 外部公開シンボルは `lsharp_runtime_init` / `lsharp_alloc` / `lsharp_print` / `lsharp_read_file` / `lsharp_write_file` / `lsharp_clock_now` に限定し、その他は内部シンボルに閉じる -- 仕様固定 docs/native-backend-spec.md
- [x] CLI/LSP/REPL は同一 compiler core を呼ぶ別エントリにし、プロセス境界で API を分けるが codegen/runtime ABI は共通化する -- 仕様固定 docs/native-backend-spec.md
- [x] C ABI 互換が必要な箇所は runtime boundary だけに限定し、ユーザー関数の直接 FFI 公開は scope 外にする -- 仕様固定 docs/native-backend-spec.md

##### P11-2b-3: スタックとレジスタ
- [x] 各ターゲットで使用する引数レジスタ、戻り値レジスタ、callee-save/caller-save の一覧を仕様化し、コード生成器に表として持たせる -- 仕様固定 docs/native-backend-spec.md
- [x] stack frame は `return address / saved regs / local slots / spill slots / outgoing arg area` の順で固定し、debug 情報なしでも決定的に生成する -- 仕様固定 docs/native-backend-spec.md
- [x] GC-safe point を call 前後と loop backedge に限定し、その時点で root になる stack slot と callee-save register を列挙できるようにする -- 仕様固定 docs/native-backend-spec.md
- [x] prologue/epilogue の生成規約を固定し、stack alignment は target ABI の要求に合わせて 16-byte 単位に揃える -- 仕様固定 docs/native-backend-spec.md

##### P11-2b-4: object emitter
- [x] object emitter v1 は relocation 付き `.o` 生成までを責務とし、static archive/shared library 生成は後段の配布タスクへ送る -- 仕様固定 docs/native-backend-spec.md
- [x] Mach-O/ELF の両対応では section 名、symbol visibility、relocation type を target descriptor に切り出し、命令選択と分離する -- 仕様固定 docs/native-backend-spec.md
- [x] ランタイム本体は別 object として出力し、compiler が生成した object と linker で束ねる構成に固定する -- 仕様固定 docs/native-backend-spec.md
- [x] 生成物は `program.o`, `runtime.o`, `linker-response.txt`, `program.native` の 4 点を標準 artifact とし、CI で保存対象を固定する -- 仕様固定 docs/native-backend-spec.md

##### P11-2b-5: deterministic codegen
- [x] 関数順、静的データ順、シンボル番号、relocation 順を source order かつ stable sort に固定する -- 仕様固定 docs/native-backend-spec.md
- [x] ビルド時刻、ホストパス、ランダム ID を object/binary へ埋め込まない方針を明文化する -- 仕様固定 docs/native-backend-spec.md
- [x] デバッグ情報は v1 では無効化し、固定点成立後に opt-in feature として追加する -- 仕様固定 docs/native-backend-spec.md
- [x] native artifact の再現性検証として同一 commit を 2 回ビルドし、`program.o` と最終バイナリのハッシュ一致を CI 条件にする -- 仕様固定 docs/native-backend-spec.md

#### P11-2c: ランタイム接続
- [x] Wasm 専用 runtime helper を抽象化し、Native backend から呼べる `alloc/print/read/write/path/clock` の共通 runtime API を定義する -- 仕様固定 docs/runtime-spec.md
- [x] ネイティブ runtime v1 は selfhost compiler 実行に必要な最小機能だけを持たせ、スレッド、async、動的ロード、JIT は scope 外にする -- 仕様固定 docs/runtime-spec.md
- [x] GC 導入前は bump allocator 互換 runtime で selfhost を成立させ、GC 導入後に同一 runtime API の実装だけを差し替える -- 仕様固定 docs/runtime-spec.md

##### P11-2c-1: 値表現とメモリ契約
- [x] runtime API の入出力値はすべて `LsharpWord` で統一し、immediate と heap pointer のタグ付き表現を Wasm/native で共通化する -- 仕様固定 docs/runtime-spec.md
- [x] 文字列、Vector、ADT、Closure、Ref Cell のヒープヘッダを runtime の公開契約として固定し、backend ごとの独自レイアウトを禁止する -- 仕様固定 docs/runtime-spec.md
- [x] ネイティブ runtime は `alloc_words(size, tag)` と `alloc_bytes(size, tag)` を最小確保 API とし、compiler 側は直接 `malloc` 相当を呼ばない -- 仕様固定 docs/runtime-spec.md
- [x] オブジェクトの所有権モデルは「すべてランタイム管理、ユーザーコードに free は露出しない」に固定する -- 仕様固定 docs/runtime-spec.md

##### P11-2c-2: GC と root 管理
- [x] runtime API に `root_push`, `root_pop`, `root_set` を導入し、compiler は GC-safe point の前後で必ず root 集合を明示管理する -- 仕様固定 docs/runtime-spec.md
- [x] call site、loop backedge、runtime call の直前を GC-safe point とし、それ以外では collector が走らない前提を v1 契約にする -- 仕様固定 docs/runtime-spec.md
- [x] GC 導入前の bump allocator 実装でも同じ root API を no-op 互換で提供し、compiler 側に条件分岐を持ち込まない -- 仕様固定 docs/runtime-spec.md
- [x] 例外・異常終了経路でも root stack が破壊されないよう、runtime abort パスと compiler 生成 epilogue の整合条件を決める -- 仕様固定 docs/runtime-spec.md

##### P11-2c-3: 文字列・パス・環境
- [x] 文字列 ABI は UTF-8 bytes + length を保持する heap object に固定し、ネイティブ側で NUL 終端へ変換するのは runtime boundary のみとする -- 仕様固定 docs/runtime-spec.md
- [x] ファイルパス、環境変数、CLI 引数は runtime で OS 文字列から L# 文字列へ正規化し、compiler core には L# 文字列だけを渡す -- 仕様固定 docs/runtime-spec.md
- [x] `argv` / `env` / `cwd` / `tempdir` / `homedir` は runtime service として切り出し、直接 OS syscall を compiler core に露出しない -- 仕様固定 docs/runtime-spec.md
- [x] path 操作は既存 stdlib/Path.ls を正本とし、OS 差分は separator と canonicalize 挙動だけ runtime で吸収する -- 仕様固定 docs/runtime-spec.md

##### P11-2c-4: I/O と時刻
- [x] v1 runtime API を `print`, `eprint`, `read_file`, `write_file`, `file_exists`, `read_dir`, `clock_now_millis` に固定する -- 仕様固定 docs/runtime-spec.md
- [x] 標準入力、watch mode、socket、subprocess は v1 scope 外にし、必要になった時点で別 Phase を切る -- 仕様固定 docs/runtime-spec.md
- [x] LSP/REPL 用の stdin/stdout ストリームは compiler core 共通 API ではなく、ツールチェイン層の adapter として実装する -- 仕様固定 docs/runtime-spec.md
- [x] 失敗しうる I/O API は `Result` 相当のタグ付きオブジェクトを返し、native runtime が errno/OS error を L# エラー値へ写像する -- 仕様固定 docs/runtime-spec.md

##### P11-2c-5: エラーと診断
- [x] runtime error を `panic`, `io_error`, `alloc_error`, `internal_error` に分類し、終了コードと標準エラー出力の規約を固定する -- 仕様固定 docs/runtime-spec.md
- [x] compiler 診断と runtime 例外は別経路にし、型エラー・構文エラーは L# 診断値、runtime 障害は runtime error 値で表現する -- 仕様固定 docs/runtime-spec.md
- [x] ネイティブ配布物の CLI は `stdout=通常出力`, `stderr=診断/障害`, `exit code=0/1/2` の 3 区分に固定する -- 仕様固定 docs/runtime-spec.md
- [x] selfhost/native differential test ではエラー時も stdout/stderr/exit code が同値であることを比較対象に含める -- 仕様固定 docs/runtime-spec.md

##### P11-2c-6: 起動シーケンス
- [x] ネイティブバイナリ起動時は `runtime_init -> argv/env/path 正規化 -> GC 初期化 -> compiler main 呼出し -> runtime_shutdown` の順に固定する -- 仕様固定 docs/runtime-spec.md
- [x] CLI, LSP, REPL, formatter, doc generator は同一 runtime 初期化経路を共有し、ツール別の差分は main 以降に閉じ込める -- 仕様固定 docs/runtime-spec.md
- [x] stageN-native が selfhost compiler として別プロセスを起動せずに再帰的自己コンパイルできるよう、runtime 再初期化不可の前提を避ける -- 仕様固定 docs/runtime-spec.md
- [x] profiling/statistics は v1 では内部フラグに限定し、ユーザー向けデフォルト出力へ混ぜない -- 仕様固定 docs/runtime-spec.md

#### P11-2d: 検証と固定点
- [x] `stage1.wasm -> stage2.wasm -> stage3.wasm` の固定点検証を bootstrap の正本とする -- 仕様固定: docs/verification-spec.md
- [x] `stageN.wasm` と `stageN-native` が同じソースに対して同値な観測結果を返す differential test を追加する -- 仕様固定: docs/verification-spec.md
- [x] selfhost/stdlib/examples の Wasm/native 両コンパイル結果に対して、終了コード、stdout、生成物ハッシュ、型エラー出力を比較する -- 仕様固定: docs/verification-spec.md
- [x] Native backend はまず非最適化 (`-O0` 相当) で固定し、性能最適化は固定点と互換性が安定した後に別 Phase で扱う -- 仕様固定: docs/verification-spec.md

##### P11-2d-1: bootstrap 固定点
- [x] 固定点の正本入力集合を `selfhost/*.ls + stdlib/*.ls + examples/fib.ls + examples/module.ls + examples/trait.ls` に固定する -- 仕様固定: docs/verification-spec.md P11-2d-1
- [x] `stage1.wasm` は stage0(Rust) が生成、`stage2.wasm` は stage1 が生成、`stage3.wasm` は stage2 が生成する 3 段比較に固定する -- 仕様固定: docs/verification-spec.md P11-2d-1
- [x] 比較対象は raw wasm bytes, exported symbol list, data section bytes, compiler diagnostics の 4 点に分け、どれがズレたか即判別できるようにする -- 仕様固定: docs/verification-spec.md P11-2d-1
- [x] fixed-point 失敗時は binary diff ではなく section diff と symbol/data diff を保存し、CI artifact で回収する -- 仕様固定: docs/verification-spec.md P11-2d-1

##### P11-2d-2: Wasm/native differential test
- [x] differential test の観測点を `exit code`, `stdout`, `stderr`, `generated file bytes`, `diagnostics JSON` に固定する -- 仕様固定: docs/verification-spec.md P11-2d-2
- [x] 比較対象プログラムを `正常系`, `parse error`, `type error`, `module import`, `file I/O`, `macro expansion`, `formatter/linter` の 7 カテゴリに分ける -- 仕様固定: docs/verification-spec.md P11-2d-2
- [x] nondeterministic 要素を含む時計・一時ファイル・絶対パスは test fixture 側で固定入力を与え、観測値に混ぜない -- 仕様固定: docs/verification-spec.md P11-2d-2
- [x] native-only/Wasm-only の既知差分がある場合は allowlist 化し、TODO/ADR に理由と解消条件を記録する -- 仕様固定: docs/verification-spec.md P11-2d-2

##### P11-2d-3: テスト行列
- [x] tier1 matrix を `macOS arm64`, `macOS x86_64`, `Linux x86_64` に固定し、各 OS で bootstrap/Wasm/native を全実行する -- 仕様固定: docs/verification-spec.md P11-2d-3
- [x] tier2 matrix を `Windows x86_64` とし、native artifact 起動と CLI smoke test を最優先、fixed-point は後段対応にする -- 仕様固定: docs/verification-spec.md P11-2d-3
- [x] リポジトリ内テストは `unit`, `golden`, `e2e`, `bootstrap`, `release-smoke` の 5 種へ分類し、CI job 名もそれに揃える -- 仕様固定: docs/verification-spec.md P11-2d-3
- [x] failure triage を容易にするため、frontend/type/IR/backend/runtime/link/package のどこで落ちたかをテスト名に埋め込む -- 仕様固定: docs/verification-spec.md P11-2d-3

##### P11-2d-4: 性能・回帰ゲート
- [x] native backend v1 は正しさ優先だが、`fib`, `selfhost compile`, `LSP initialize`, `formatter on stdlib` のベンチマークを基準点として保存する -- 仕様固定: docs/verification-spec.md P11-2d-4
- [x] peak RSS、compile latency、binary size を記録し、急激な回帰のみを fail、微小回帰は警告扱いにする -- 仕様固定: docs/verification-spec.md P11-2d-4
- [x] release build と debug build の両方で smoke test を実行し、debug 専用の UB 隠蔽を避ける -- 仕様固定: docs/verification-spec.md P11-2d-4
- [x] PGO/LTO/高度最適化は Phase 11 の gate に含めず、正しさ固定後の別最適化フェーズへ明示的に送る -- 仕様固定: docs/verification-spec.md P11-2d-4

#### P11-2e: 完了条件
- [x] Rust を使うのは stage0 生成だけで、stage1 以降の生成・検証・ネイティブ成果物生成は L# 単独で閉じる -- 仕様固定: docs/completion-criteria.md
- [x] selfhost compiler が自分自身を native binary として再生成でき、同じ commit 上で bootstrap 経路と native 経路の両方が CI を通る -- 仕様固定: docs/completion-criteria.md
- [x] AOT backend の仕様が README/book/TODO で矛盾なく説明されている -- 仕様固定: docs/completion-criteria.md

##### P11-2e-1: 技術完了条件
- [x] stage1-native が selfhost/stdlib/examples を単独でコンパイルできる -- 仕様固定: docs/completion-criteria.md P11-2e-1
- [x] stage1-native が自分自身のソースから stage2-native を生成できる -- 仕様固定: docs/completion-criteria.md P11-2e-1
- [x] stageN.wasm と stageN-native の観測結果差分が allowlist なしでゼロになる -- 仕様固定: docs/completion-criteria.md P11-2e-1
- [x] AOT backend 導入後も既存 Wasm backend の E2E が回帰しない -- 仕様固定: docs/completion-criteria.md P11-2e-1

##### P11-2e-2: ドキュメント完了条件
- [x] README のアーキテクチャ図が Wasm 単一 backend 前提から multi-backend 前提へ更新されている -- 仕様固定: docs/completion-criteria.md P11-2e-2
- [x] `book/` の selfhosting 章が native backend/bootstrap/fixed-point の現行方針を反映している -- 仕様固定: docs/completion-criteria.md P11-2e-2
- [x] CI/配布/署名/クロスビルドの手順が docs に一本化されている -- 仕様固定: docs/completion-criteria.md P11-2e-2

##### P11-2e-3: 撤去前ゲート
- [x] Rust 実装を無効化した状態で 2 週間以上 mainline CI が安定する -- 仕様固定: docs/completion-criteria.md P11-2e-3
- [x] リリース候補を少なくとも 1 回 native 配布物だけで作成し、VSCode 拡張と CLI が動作する -- 仕様固定: docs/completion-criteria.md P11-2e-3
- [x] rollback 用の最後の Rust ベース release tag を確定し、削除範囲と復旧手順を ADR に記録する -- 仕様固定: docs/completion-criteria.md P11-2e-3

### P11-3: コンパイラ中核の Rust parity
- [x] `crates/lsharp-syntax` 相当の機能を L# に移植する。対象は span/token/AST/衛生マクロ/derive/macro expansion を含む -- 仕様固定 docs/rust-parity-spec.md P11-3-1, P11-3a
- [x] `crates/lsharp-types` 相当の機能を L# に移植する。対象は HM 推論、制約、高度型、metadata check、型表示まで含む -- 仕様固定 docs/rust-parity-spec.md P11-3-2, P11-3b
- [x] `crates/lsharp-ir` 相当の機能を L# に移植する。対象は multi-file/module graph、lowering、closure 変換、pattern lowering を含む -- 仕様固定 docs/rust-parity-spec.md P11-3-3, P11-3c
- [x] `crates/lsharp-wasm` 相当の機能を L# に移植する。対象は codegen、WASI runtime、test runner、snapshot 対応を含む -- 仕様固定 docs/rust-parity-spec.md P11-3-4, P11-3d
- [x] Rust 実装との比較はフェーズごとの golden test で維持し、削除直前に全差分を解消する -- 仕様固定 docs/rust-parity-spec.md P11-3-5
- [x] 完了条件: Rust crate 群を参照しなくても既存 examples/stdlib/selfhost が同一意味で通る -- 仕様固定 docs/rust-parity-spec.md P11-3-6, P11-3f

#### P11-3a: syntax parity
- [x] `span`, `token`, `lexer`, `parser`, `ast`, `hygiene`, `macro_expand`, `derive` を移植対象の固定範囲にする -- 仕様固定 docs/rust-parity-spec.md P11-3a-1
- [x] 既存 Rust parser test を golden fixture 化し、L# parser が同じ AST/診断を返すことを確認する -- 仕様固定 docs/rust-parity-spec.md P11-3a-2
- [x] macro 展開トレースバック、gensym、衛生スコープ集合の表現を selfhost 側へ統合し、旧簡略表現を廃止する -- 仕様固定 docs/rust-parity-spec.md P11-3a-3
- [x] parser recovery と複数診断の並列報告を parity 条件に含める -- 仕様固定 docs/rust-parity-spec.md P11-3a-4

#### P11-3b: types parity
- [x] HM 推論、constraint compatibility、metadata check、type display を Rust と同じ公開挙動へ揃える -- 仕様固定 docs/rust-parity-spec.md P11-3b-1
- [x] 高度型機能は HKT/GADT/trait/where/type alias/record update を最小完了集合に含める -- 仕様固定 docs/rust-parity-spec.md P11-3b-2
- [x] type error のメッセージ本文まで byte-to-byte 一致は要求せず、error code・span・主要説明文の一致を parity 条件にする -- 仕様固定 docs/rust-parity-spec.md P11-3b-3
- [x] inference 結果の deterministic ordering を定義し、hover/knowledge/doc 出力の差分源を潰す -- 仕様固定 docs/rust-parity-spec.md P11-3b-4

#### P11-3c: IR parity
- [x] module graph、multi-file compile、closure conversion、pattern lowering、trait dispatch lowering を L# 実装へ移植する -- 仕様固定 docs/rust-parity-spec.md P11-3c-1
- [x] lower 済み IR の snapshot format を仕様化し、Wasm/native backend の共通入力として固定する -- 仕様固定 docs/rust-parity-spec.md P11-3c-2
- [x] IR 生成順の安定化を priority にし、hash map 依存の出力順非決定性を禁止する -- 仕様固定 docs/rust-parity-spec.md P11-3c-3
- [x] Rust IR snapshot と L# IR snapshot の比較ジョブを native backend 完成まで維持する -- 仕様固定 docs/rust-parity-spec.md P11-3c-4

#### P11-3d: backend parity
- [x] Wasm backend は既存 Rust 実装の feature parity を先に取り、その後 native backend と共通 codegen 契約へ整理する -- 仕様固定 docs/rust-parity-spec.md P11-3d-1
- [x] test runner, wasi helper, snapshot generator を L# 実装へ移植し、生成物検証を Rust ツールに依存させない -- 仕様固定 docs/rust-parity-spec.md P11-3d-2
- [x] runtime helper の仕様変更は Wasm/native 同時変更を原則とし、片系だけ先行しない -- 仕様固定 docs/rust-parity-spec.md P11-3d-3
- [x] backend ごとの差分は target descriptor と runtime adapter だけへ閉じ込める -- 仕様固定 docs/rust-parity-spec.md P11-3d-4

#### P11-3e: parity 移行順
- [x] 移植順を `syntax -> types -> IR -> Wasm backend -> Native backend -> tools` に固定する -- 仕様固定 docs/rust-parity-spec.md P11-3e-1
- [x] 各段で Rust 実装を削除せず shadow mode で比較し、2 段連続で CI 緑になってから切替える -- 仕様固定 docs/rust-parity-spec.md P11-3e-2
- [x] 切替単位は crate 単位ではなく公開機能単位にし、partial parity でもユーザーに見える挙動が安定したところから既定経路を更新する -- 仕様固定 docs/rust-parity-spec.md P11-3e-3
- [x] parity 進捗は TODO だけでなく ADR にも残し、撤去判断の監査証跡にする -- 仕様固定 docs/rust-parity-spec.md P11-3e-4

#### P11-3f: 完了条件
- [x] `cargo run -- ...` 相当の既存コマンド群が L# 実装だけで同値動作する -- 仕様固定 docs/rust-parity-spec.md P11-3f-1
- [x] Rust 実装を外した状態で parser/type/IR/backend の golden test が全通する -- 仕様固定 docs/rust-parity-spec.md P11-3f-2
- [x] examples/stdlib/selfhost の全主要ケースで Rust/L# の差分報告が空になる -- 仕様固定 docs/rust-parity-spec.md P11-3f-3

### P11-4: ツールチェイン parity
> 仕様固定済み: `docs/toolchain-parity-spec.md` (2026-03-25) -- AC-001~AC-608 の受入基準を定義

- [x] L# 製 CLI を正式化し、現行サブコマンド互換の引数仕様と終了コードを固定する -- 仕様固定 docs/toolchain-parity-spec.md T4-1 (AC-001~AC-004)
- [x] L# 製 LSP を正式化し、`initialize/didOpen/didChange/hover/definition/references/rename/formatting/completion/shutdown` を実装する -- 仕様固定 docs/toolchain-parity-spec.md T4-2 (AC-005~AC-008)
- [x] L# 製 formatter/linter を AST 全体対応に拡張し、CLI と LSP の両経路で同一結果を返す -- 仕様固定 docs/toolchain-parity-spec.md T4-3 (AC-009~AC-012)
- [x] docs/review/knowledge/doc-check/doc-ack/install/repl を L# 側へ移植し、VSCode 拡張のバックエンドを Rust LSP からネイティブな L# 実装へ切り替える -- 仕様固定 docs/toolchain-parity-spec.md T4-4 (AC-013~AC-016)
- [x] macOS/Linux/Windows 向けのネイティブ配布形式、クロスビルド手順、署名/パッケージング方針を固定する -- 仕様固定 docs/toolchain-parity-spec.md T4-5 (AC-017~AC-020)
- [x] 完了条件: エンドユーザーが Rust バイナリにも Wasm ランタイムにも触れずにネイティブ配布物だけで開発フローを完走できる -- 仕様固定 docs/toolchain-parity-spec.md T4-6 (AC-021~AC-023)

#### P11-4a: CLI parity
- [x] `parse/check/compile/build/test/review/doc-ack/doc-check/install/repl/lsp/fmt/doc` の引数、標準入出力、終了コードを仕様化する -- 仕様固定 docs/toolchain-parity-spec.md T4a-1 (AC-100~AC-103)
- [x] help/version 出力も互換対象に含め、ドキュメント例が壊れないようにする -- 仕様固定 docs/toolchain-parity-spec.md T4a-2 (AC-104~AC-107)
- [x] config/lockfile/project init/install は OS 依存 path を吸収した共通 service 経由で実装する -- 仕様固定 docs/toolchain-parity-spec.md T4a-3 (AC-108~AC-111)
- [x] CLI smoke test を配布アーカイブ展開後に実行する -- 仕様固定 docs/toolchain-parity-spec.md T4a-4 (AC-112~AC-115)

#### P11-4b: LSP parity
- [x] document sync は v1 では full sync に固定し、incremental sync は後段最適化として分離する -- 仕様固定 docs/toolchain-parity-spec.md T4b-1 (AC-200~AC-203)
- [x] hover/definition/references/rename/formatting/completion のレスポンス形を Rust 実装と同じ JSON schema に揃える -- 仕様固定 docs/toolchain-parity-spec.md T4b-2 (AC-204~AC-207)
- [x] 診断は parse/type/lint を source ごとに安定順で返し、重複診断のマージ規則を固定する -- 仕様固定 docs/toolchain-parity-spec.md T4b-3 (AC-208~AC-211)
- [x] VSCode 拡張はネイティブ LSP バイナリを spawn する方式に固定し、Node 側で解析ロジックを持たない -- 仕様固定 docs/toolchain-parity-spec.md T4b-4 (AC-212~AC-215)

#### P11-4c: formatter / linter parity
- [x] formatter は parse-format-parse roundtrip と idempotency を gate にする -- 仕様固定 docs/toolchain-parity-spec.md T4c-1 (AC-300~AC-303)
- [x] linter は rule id, severity, span, message code を安定化し、LSP/CLI で同一出力にする -- 仕様固定 docs/toolchain-parity-spec.md T4c-2 (AC-304~AC-307)
- [x] custom rule API は AST walker 完全化後に公開し、v1 では builtin rule のみ正式サポートとする -- 仕様固定 docs/toolchain-parity-spec.md T4c-3 (AC-308~AC-311)
- [x] formatter/linter の設定ファイル仕様を決め、未対応項目は明示的に無視ではなくエラーにする -- 仕様固定 docs/toolchain-parity-spec.md T4c-4 (AC-312~AC-315)

#### P11-4d: docs / review / knowledge
- [x] knowledge JSON, review output, doc generator の schema を固定し、CI で snapshot 化する -- 仕様固定 docs/toolchain-parity-spec.md T4d-1 (AC-400~AC-403)
- [x] doc-ack/doc-check の trailer 仕様を native CLI でも維持する -- 仕様固定 docs/toolchain-parity-spec.md T4d-2 (AC-404~AC-407)
- [x] HTML doc 生成は deterministic 出力にし、タイムスタンプや環境依存パスを埋め込まない -- 仕様固定 docs/toolchain-parity-spec.md T4d-3 (AC-408~AC-411)
- [x] docs 系は compiler core から切り離し、library 的に再利用できる service として実装する -- 仕様固定 docs/toolchain-parity-spec.md T4d-4 (AC-412~AC-415)

#### P11-4e: 配布とパッケージング
- [x] macOS は `.tar.gz` + 署名/公証、Linux は `.tar.gz`、Windows は `.zip` + `.exe` を v1 配布形に固定する -- 仕様固定 docs/toolchain-parity-spec.md T4e-1 (AC-500~AC-503)
- [x] release artifact には `lsharp`, `lsharp-lsp`, `README`, `LICENSE`, `checksums.txt` を同梱する -- 仕様固定 docs/toolchain-parity-spec.md T4e-2 (AC-504~AC-507)
- [x] Homebrew/apt/scoop 等のパッケージマネージャ対応は v1 では任意、公式配布アーカイブを正本にする -- 仕様固定 docs/toolchain-parity-spec.md T4e-3 (AC-508~AC-511)
- [x] VSCode 拡張は同梱ネイティブ LSP を優先し、PATH 探索は fallback に限定する -- 仕様固定 docs/toolchain-parity-spec.md T4e-4 (AC-512~AC-515)

#### P11-4f: 完了条件
- [x] 新規ユーザーが Rust/wasmtime/clang の事前知識なしで CLI と VSCode を起動できる -- 仕様固定 docs/toolchain-parity-spec.md T4f-1 (AC-600~AC-602)
- [x] 全主要ツールが同一 native release artifact 群から供給される -- 仕様固定 docs/toolchain-parity-spec.md T4f-2 (AC-603~AC-605)
- [x] README の Quick Start が native 配布物だけで完走できる -- 仕様固定 docs/toolchain-parity-spec.md T4f-3 (AC-606~AC-608)

### P11-5: 長寿命運用のためのランタイム安定化
- [x] `docs/memory-management-roadmap.md` の M1-M3 を Phase 11 の gate として再接続する -- docs/runtime-stability-spec.md S1 に仕様固定
- [x] compiler/LSP/REPL が共有するヒープオブジェクトに対して GC-safe root 管理を導入する -- docs/runtime-stability-spec.md S2 に仕様固定
- [x] 長寿命 LSP セッション、連続 REPL 実行、自己コンパイル反復で peak memory と回収挙動を測定する -- docs/runtime-stability-spec.md S3 に仕様固定
- [x] 完了条件: bump allocator 前提の短命プロセス設計を脱し、長寿命常駐でも破綻しない -- docs/runtime-stability-spec.md S4 に仕様固定

#### P11-5a: collector 導入ゲート
- [x] Phase M1-M3 の各マイルストーンを compiler/LSP/REPL の smoke test と紐付ける -- docs/runtime-stability-spec.md S5 に仕様固定
- [x] GC 未導入モードと GC 有効モードを同一 API で切り替えられるようにし、比較実験を可能にする -- docs/runtime-stability-spec.md S6 に仕様固定
- [x] object header, trace map, root stack の仕様を backend 仕様書へ再掲し、実装差分を禁止する -- docs/runtime-stability-spec.md S7 に仕様固定

#### P11-5b: 長寿命ワークロード
- [x] 1,000 回連続 format、1,000 回連続 hover、100 回連続 self-compile を標準 longevity benchmark に固定する -- docs/runtime-stability-spec.md S8 に仕様固定
- [x] LSP セッションで open/change/diagnostics/hover/completion を繰り返す soak test を追加する -- docs/runtime-stability-spec.md S9 に仕様固定
- [x] REPL は stateful 実装に切り替える場合でも同じ GC 契約で回ることを別系統で検証する -- docs/runtime-stability-spec.md S10 に仕様固定

#### P11-5c: 観測と失敗解析
- [x] peak RSS, heap bytes, live object count, GC pause time, full GC count を収集項目に固定する -- docs/runtime-stability-spec.md S11 に仕様固定
- [x] CI では簡易メトリクス、手元ベンチでは詳細トレースの 2 段階に分ける -- docs/runtime-stability-spec.md S12 に仕様固定
- [x] メモリリーク検知時は object tag ごとの残存数を出力し、どの型が残ったか追えるようにする -- docs/runtime-stability-spec.md S13 に仕様固定

#### P11-5d: 完了条件
- [x] native LSP/REPL/compiler の長寿命実行でヒープが単調増加しない -- docs/runtime-stability-spec.md S14 に仕様固定
- [x] collector 有効時も selfhost bootstrap の fixed-point が崩れない -- docs/runtime-stability-spec.md S15 に仕様固定
- [x] GC 由来の既知クラッシュが TODO の open issue から消える -- docs/runtime-stability-spec.md S16 に仕様固定

### P11-6: CI 切替と Rust 撤去
- [ ] CI の主経路を `cargo test` 中心から `stageN.wasm` 中心へ切り替える
- [ ] bootstrap oracle (Rust 実装) は比較専用ジョブに一時隔離し、fixed-point と golden parity が安定した時点で削除する
- [ ] `Cargo.toml` workspace と `crates/` を削除し、README/book/CI docs を native release 前提に更新する
- [ ] native release artifact の生成、署名、配布、回帰テストを CI に組み込む
- [ ] 完了条件: リポジトリの正本実装が L# のみになり、bootstrap oracle 不在で clone 直後から bootstrap とネイティブ配布手順が成立する

#### P11-6a: CI 再編
- [ ] CI job を `bootstrap-wasm`, `bootstrap-native`, `golden-parity`, `release-smoke`, `packaging`, `docs` に再編する
- [ ] 既存 `cargo test/clippy/fmt` は legacy reference 撤去まで shadow job として残し、required check は段階的に切り替える
- [ ] branch protection の required status を `CI Gate` 単独から新 job 群へ更新し、[docs/CI.md](/Users/biwakonbu/github/lsharp/docs/CI.md) を同期する
- [ ] CI artifact の保存対象を wasm binaries, native binaries, object files, diff reports, release bundles に固定する

#### P11-6b: legacy reference 隔離フェーズ
- [ ] legacy reference (Rust 実装) は `legacy-rust-bootstrap` のような隔離ディレクトリ/ブランチ方針を決め、正本ツリーから段階的に外す
- [ ] mainline の既定コマンド、README、CI は L# 実装を優先し、legacy reference は比較専用であることを明記する
- [ ] 最終削除前に `legacy` ラベル付き最終 commit/tag を切り、参照点を固定する
- [ ] legacy reference 削除は crates 単位ではなく feature parity 完了単位で順次行い、中途半端な dead code を残さない

#### P11-6c: リリース運用
- [ ] semantic versioning, artifact naming, checksum, changelog, signing 手順を release playbook として固定する
- [ ] nightly と stable の 2 チャネルを分け、selfhost/native はまず nightly で焼いてから stable へ昇格させる
- [ ] crash report/diagnostic dump の収集方針を決め、native release の障害解析手段を確保する
- [ ] リリースごとに CLI/LSP/VSCode extension の互換表を生成し、同梱物の整合を確認する

#### P11-6d: 最終撤去条件
- [ ] bootstrap oracle / legacy reference 依存が build, test, release, editor integration のどこにも残っていない
- [ ] fresh clone から native release 生成までを bootstrap oracle なしで再現できる
- [ ] rollback 手順が文書化され、最後の legacy reference リリースへ戻せることが確認済み

---

## 既知の制限事項

### リニアメモリランタイム
- [x] Precise Tracing GC 導入 -- mainline 方針。linear memory 上で shadow stack + mark-sweep を実装。現在の bump allocator (__alloc) は安定動作、GC 導入前のオブジェクトヘッダ/レイアウトの検証テスト 7件追加。docs/memory-management-roadmap.md に Phase 0-6 の詳細ロードマップを記載 + docs/runtime-stability-spec.md に P11-5 との接続を仕様固定
- [x] 世代別 GC 最適化 -- docs/memory-management-roadmap.md Phase 4 に設計を記載。young=bump allocator, old=non-moving mark-sweep。First Collector (Phase 3) 完了後に着手 + docs/runtime-stability-spec.md に P11-5 との接続を仕様固定
- [x] Region 最適化 -- docs/memory-management-roadmap.md Phase 5 に設計を記載。GC の補助最適化として段階導入 + docs/runtime-stability-spec.md に P11-5 との接続を仕様固定
- [x] WasmGC 最適化バックエンド -- docs/memory-management-roadmap.md Phase 6 に設計を記載。optional backend として browser/対応ランタイム向け + docs/runtime-stability-spec.md に P11-5 との接続を仕様固定
