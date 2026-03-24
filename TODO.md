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

#### P11-1a: 監査対象の固定
- [ ] 監査対象文書を `TODO.md`, `README.md`, `book/ch15-selfhosting.md`, `docs/CI.md`, `docs/memory-management-roadmap.md`, `editors/vscode/*` に固定する
- [ ] 監査対象実装を `selfhost/*`, `stdlib/*`, `crates/*`, `examples/*`, `.github/workflows/*` に固定する
- [ ] 「完了済み」「部分実装」「PoC」「設計のみ」の 4 区分で各項目を再ラベル付けする
- [ ] 監査の正本出力先を `TODO.md` と ADR に限定し、別ドキュメントへ状態を分散させない

#### P11-1b: 互換マトリクス
- [ ] 行方向を公開機能、列方向を `Rust status`, `L# status`, `parity test`, `default path`, `deletion gate` とする
- [ ] CLI はサブコマンド単位、LSP はメソッド単位、formatter/linter/docs は出力 schema 単位で棚卸しする
- [ ] 「実装ありだが未接続」「部分動作」「本番使用可」を別状態として区別し、単純な yes/no で潰さない
- [ ] 互換マトリクスは Phase 11 完了まで PR ごとに更新必須にする

#### P11-1c: 差分判定規則
- [ ] 差分は `仕様差分`, `実装欠落`, `出力差分`, `性能差分`, `運用差分` の 5 種に分類する
- [ ] 仕様差分は TODO/README/book の記述不一致、実装欠落はコード不在、出力差分はテスト不一致として扱う
- [ ] 性能差分は Phase 11 の blocking 条件にせず、正しさ差分を優先して解消する
- [ ] 運用差分には CI、配布、署名、VSCode 連携、インストール手順を含める

#### P11-1d: 受け入れ基準
- [ ] 完了表示の各項目に一次エビデンスを紐付け、テスト名、ADR、ファイルパスのいずれかを必須にする
- [ ] `README` と `book` に書かれた導入手順が現行 mainline で再現できることを smoke test で確認する
- [ ] `TODO.md` 上で Phase 11 の各完了条件が、それぞれ具体的なテスト/ドキュメント/CI gate に接続されている
- [ ] 監査完了後は「Rust 完全撤去」に関する曖昧な用語を禁止し、`bootstrap oracle`, `legacy reference`, `native release` など定義済み語彙へ統一する

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

##### P11-2b-1: 内部 ABI
- [ ] L# 関数の v1 ABI を「引数と戻り値は machine word 単位、複合値は pointer 参照、複数戻り値なし」に固定する
- [ ] immediate 値と heap pointer の表現は現行 linear-memory runtime のタグ付き word 表現を維持し、backend ごとの値表現分岐を禁止する
- [ ] 呼出規約は caller-save 優先で固定し、再帰・相互再帰・高階関数の呼出しが同一規約で通ることを acceptance criteria にする
- [ ] tail call は v1 では非対応に固定し、通常 call + return で正しさを先に取る

##### P11-2b-2: 外部 ABI
- [ ] エントリポイントは `main(argc, argv)` 互換ではなく、L# の `main` をランタイム初期化後に呼ぶ薄いネイティブ stub で包む方式に固定する
- [ ] 外部公開シンボルは `lsharp_runtime_init` / `lsharp_alloc` / `lsharp_print` / `lsharp_read_file` / `lsharp_write_file` / `lsharp_clock_now` に限定し、その他は内部シンボルに閉じる
- [ ] CLI/LSP/REPL は同一 compiler core を呼ぶ別エントリにし、プロセス境界で API を分けるが codegen/runtime ABI は共通化する
- [ ] C ABI 互換が必要な箇所は runtime boundary だけに限定し、ユーザー関数の直接 FFI 公開は scope 外にする

##### P11-2b-3: スタックとレジスタ
- [ ] 各ターゲットで使用する引数レジスタ、戻り値レジスタ、callee-save/caller-save の一覧を仕様化し、コード生成器に表として持たせる
- [ ] stack frame は `return address / saved regs / local slots / spill slots / outgoing arg area` の順で固定し、debug 情報なしでも決定的に生成する
- [ ] GC-safe point を call 前後と loop backedge に限定し、その時点で root になる stack slot と callee-save register を列挙できるようにする
- [ ] prologue/epilogue の生成規約を固定し、stack alignment は target ABI の要求に合わせて 16-byte 単位に揃える

##### P11-2b-4: object emitter
- [ ] object emitter v1 は relocation 付き `.o` 生成までを責務とし、static archive/shared library 生成は後段の配布タスクへ送る
- [ ] Mach-O/ELF の両対応では section 名、symbol visibility、relocation type を target descriptor に切り出し、命令選択と分離する
- [ ] ランタイム本体は別 object として出力し、compiler が生成した object と linker で束ねる構成に固定する
- [ ] 生成物は `program.o`, `runtime.o`, `linker-response.txt`, `program.native` の 4 点を標準 artifact とし、CI で保存対象を固定する

##### P11-2b-5: deterministic codegen
- [ ] 関数順、静的データ順、シンボル番号、relocation 順を source order かつ stable sort に固定する
- [ ] ビルド時刻、ホストパス、ランダム ID を object/binary へ埋め込まない方針を明文化する
- [ ] デバッグ情報は v1 では無効化し、固定点成立後に opt-in feature として追加する
- [ ] native artifact の再現性検証として同一 commit を 2 回ビルドし、`program.o` と最終バイナリのハッシュ一致を CI 条件にする

#### P11-2c: ランタイム接続
- [ ] Wasm 専用 runtime helper を抽象化し、Native backend から呼べる `alloc/print/read/write/path/clock` の共通 runtime API を定義する
- [ ] ネイティブ runtime v1 は selfhost compiler 実行に必要な最小機能だけを持たせ、スレッド、async、動的ロード、JIT は scope 外にする
- [ ] GC 導入前は bump allocator 互換 runtime で selfhost を成立させ、GC 導入後に同一 runtime API の実装だけを差し替える

##### P11-2c-1: 値表現とメモリ契約
- [ ] runtime API の入出力値はすべて `LsharpWord` で統一し、immediate と heap pointer のタグ付き表現を Wasm/native で共通化する
- [ ] 文字列、Vector、ADT、Closure、Ref Cell のヒープヘッダを runtime の公開契約として固定し、backend ごとの独自レイアウトを禁止する
- [ ] ネイティブ runtime は `alloc_words(size, tag)` と `alloc_bytes(size, tag)` を最小確保 API とし、compiler 側は直接 `malloc` 相当を呼ばない
- [ ] オブジェクトの所有権モデルは「すべてランタイム管理、ユーザーコードに free は露出しない」に固定する

##### P11-2c-2: GC と root 管理
- [ ] runtime API に `root_push`, `root_pop`, `root_set` を導入し、compiler は GC-safe point の前後で必ず root 集合を明示管理する
- [ ] call site、loop backedge、runtime call の直前を GC-safe point とし、それ以外では collector が走らない前提を v1 契約にする
- [ ] GC 導入前の bump allocator 実装でも同じ root API を no-op 互換で提供し、compiler 側に条件分岐を持ち込まない
- [ ] 例外・異常終了経路でも root stack が破壊されないよう、runtime abort パスと compiler 生成 epilogue の整合条件を決める

##### P11-2c-3: 文字列・パス・環境
- [ ] 文字列 ABI は UTF-8 bytes + length を保持する heap object に固定し、ネイティブ側で NUL 終端へ変換するのは runtime boundary のみとする
- [ ] ファイルパス、環境変数、CLI 引数は runtime で OS 文字列から L# 文字列へ正規化し、compiler core には L# 文字列だけを渡す
- [ ] `argv` / `env` / `cwd` / `tempdir` / `homedir` は runtime service として切り出し、直接 OS syscall を compiler core に露出しない
- [ ] path 操作は既存 stdlib/Path.ls を正本とし、OS 差分は separator と canonicalize 挙動だけ runtime で吸収する

##### P11-2c-4: I/O と時刻
- [ ] v1 runtime API を `print`, `eprint`, `read_file`, `write_file`, `file_exists`, `read_dir`, `clock_now_millis` に固定する
- [ ] 標準入力、watch mode、socket、subprocess は v1 scope 外にし、必要になった時点で別 Phase を切る
- [ ] LSP/REPL 用の stdin/stdout ストリームは compiler core 共通 API ではなく、ツールチェイン層の adapter として実装する
- [ ] 失敗しうる I/O API は `Result` 相当のタグ付きオブジェクトを返し、native runtime が errno/OS error を L# エラー値へ写像する

##### P11-2c-5: エラーと診断
- [ ] runtime error を `panic`, `io_error`, `alloc_error`, `internal_error` に分類し、終了コードと標準エラー出力の規約を固定する
- [ ] compiler 診断と runtime 例外は別経路にし、型エラー・構文エラーは L# 診断値、runtime 障害は runtime error 値で表現する
- [ ] ネイティブ配布物の CLI は `stdout=通常出力`, `stderr=診断/障害`, `exit code=0/1/2` の 3 区分に固定する
- [ ] selfhost/native differential test ではエラー時も stdout/stderr/exit code が同値であることを比較対象に含める

##### P11-2c-6: 起動シーケンス
- [ ] ネイティブバイナリ起動時は `runtime_init -> argv/env/path 正規化 -> GC 初期化 -> compiler main 呼出し -> runtime_shutdown` の順に固定する
- [ ] CLI, LSP, REPL, formatter, doc generator は同一 runtime 初期化経路を共有し、ツール別の差分は main 以降に閉じ込める
- [ ] stageN-native が selfhost compiler として別プロセスを起動せずに再帰的自己コンパイルできるよう、runtime 再初期化不可の前提を避ける
- [ ] profiling/statistics は v1 では内部フラグに限定し、ユーザー向けデフォルト出力へ混ぜない

#### P11-2d: 検証と固定点
- [ ] `stage1.wasm -> stage2.wasm -> stage3.wasm` の固定点検証を bootstrap の正本とする
- [ ] `stageN.wasm` と `stageN-native` が同じソースに対して同値な観測結果を返す differential test を追加する
- [ ] selfhost/stdlib/examples の Wasm/native 両コンパイル結果に対して、終了コード、stdout、生成物ハッシュ、型エラー出力を比較する
- [ ] Native backend はまず非最適化 (`-O0` 相当) で固定し、性能最適化は固定点と互換性が安定した後に別 Phase で扱う

##### P11-2d-1: bootstrap 固定点
- [ ] 固定点の正本入力集合を `selfhost/*.ls + stdlib/*.ls + examples/fib.ls + examples/module.ls + examples/trait.ls` に固定する
- [ ] `stage1.wasm` は stage0(Rust) が生成、`stage2.wasm` は stage1 が生成、`stage3.wasm` は stage2 が生成する 3 段比較に固定する
- [ ] 比較対象は raw wasm bytes, exported symbol list, data section bytes, compiler diagnostics の 4 点に分け、どれがズレたか即判別できるようにする
- [ ] fixed-point 失敗時は binary diff ではなく section diff と symbol/data diff を保存し、CI artifact で回収する

##### P11-2d-2: Wasm/native differential test
- [ ] differential test の観測点を `exit code`, `stdout`, `stderr`, `generated file bytes`, `diagnostics JSON` に固定する
- [ ] 比較対象プログラムを `正常系`, `parse error`, `type error`, `module import`, `file I/O`, `macro expansion`, `formatter/linter` の 7 カテゴリに分ける
- [ ] nondeterministic 要素を含む時計・一時ファイル・絶対パスは test fixture 側で固定入力を与え、観測値に混ぜない
- [ ] native-only/Wasm-only の既知差分がある場合は allowlist 化し、TODO/ADR に理由と解消条件を記録する

##### P11-2d-3: テスト行列
- [ ] tier1 matrix を `macOS arm64`, `macOS x86_64`, `Linux x86_64` に固定し、各 OS で bootstrap/Wasm/native を全実行する
- [ ] tier2 matrix を `Windows x86_64` とし、native artifact 起動と CLI smoke test を最優先、fixed-point は後段対応にする
- [ ] リポジトリ内テストは `unit`, `golden`, `e2e`, `bootstrap`, `release-smoke` の 5 種へ分類し、CI job 名もそれに揃える
- [ ] failure triage を容易にするため、frontend/type/IR/backend/runtime/link/package のどこで落ちたかをテスト名に埋め込む

##### P11-2d-4: 性能・回帰ゲート
- [ ] native backend v1 は正しさ優先だが、`fib`, `selfhost compile`, `LSP initialize`, `formatter on stdlib` のベンチマークを基準点として保存する
- [ ] peak RSS、compile latency、binary size を記録し、急激な回帰のみを fail、微小回帰は警告扱いにする
- [ ] release build と debug build の両方で smoke test を実行し、debug 専用の UB 隠蔽を避ける
- [ ] PGO/LTO/高度最適化は Phase 11 の gate に含めず、正しさ固定後の別最適化フェーズへ明示的に送る

#### P11-2e: 完了条件
- [ ] Rust を使うのは stage0 生成だけで、stage1 以降の生成・検証・ネイティブ成果物生成は L# 単独で閉じる
- [ ] selfhost compiler が自分自身を native binary として再生成でき、同じ commit 上で bootstrap 経路と native 経路の両方が CI を通る
- [ ] AOT backend の仕様が README/book/TODO で矛盾なく説明されている

##### P11-2e-1: 技術完了条件
- [ ] stage1-native が selfhost/stdlib/examples を単独でコンパイルできる
- [ ] stage1-native が自分自身のソースから stage2-native を生成できる
- [ ] stageN.wasm と stageN-native の観測結果差分が allowlist なしでゼロになる
- [ ] AOT backend 導入後も既存 Wasm backend の E2E が回帰しない

##### P11-2e-2: ドキュメント完了条件
- [ ] README のアーキテクチャ図が Wasm 単一 backend 前提から multi-backend 前提へ更新されている
- [ ] `book/` の selfhosting 章が native backend/bootstrap/fixed-point の現行方針を反映している
- [ ] CI/配布/署名/クロスビルドの手順が docs に一本化されている

##### P11-2e-3: 撤去前ゲート
- [ ] Rust 実装を無効化した状態で 2 週間以上 mainline CI が安定する
- [ ] リリース候補を少なくとも 1 回 native 配布物だけで作成し、VSCode 拡張と CLI が動作する
- [ ] rollback 用の最後の Rust ベース release tag を確定し、削除範囲と復旧手順を ADR に記録する

### P11-3: コンパイラ中核の Rust parity
- [ ] `crates/lsharp-syntax` 相当の機能を L# に移植する。対象は span/token/AST/衛生マクロ/derive/macro expansion を含む
- [ ] `crates/lsharp-types` 相当の機能を L# に移植する。対象は HM 推論、制約、高度型、metadata check、型表示まで含む
- [ ] `crates/lsharp-ir` 相当の機能を L# に移植する。対象は multi-file/module graph、lowering、closure 変換、pattern lowering を含む
- [ ] `crates/lsharp-wasm` 相当の機能を L# に移植する。対象は codegen、WASI runtime、test runner、snapshot 対応を含む
- [ ] Rust 実装との比較はフェーズごとの golden test で維持し、削除直前に全差分を解消する
- [ ] 完了条件: Rust crate 群を参照しなくても既存 examples/stdlib/selfhost が同一意味で通る

#### P11-3a: syntax parity
- [ ] `span`, `token`, `lexer`, `parser`, `ast`, `hygiene`, `macro_expand`, `derive` を移植対象の固定範囲にする
- [ ] 既存 Rust parser test を golden fixture 化し、L# parser が同じ AST/診断を返すことを確認する
- [ ] macro 展開トレースバック、gensym、衛生スコープ集合の表現を selfhost 側へ統合し、旧簡略表現を廃止する
- [ ] parser recovery と複数診断の並列報告を parity 条件に含める

#### P11-3b: types parity
- [ ] HM 推論、constraint compatibility、metadata check、type display を Rust と同じ公開挙動へ揃える
- [ ] 高度型機能は HKT/GADT/trait/where/type alias/record update を最小完了集合に含める
- [ ] type error のメッセージ本文まで byte-to-byte 一致は要求せず、error code・span・主要説明文の一致を parity 条件にする
- [ ] inference 結果の deterministic ordering を定義し、hover/knowledge/doc 出力の差分源を潰す

#### P11-3c: IR parity
- [ ] module graph、multi-file compile、closure conversion、pattern lowering、trait dispatch lowering を L# 実装へ移植する
- [ ] lower 済み IR の snapshot format を仕様化し、Wasm/native backend の共通入力として固定する
- [ ] IR 生成順の安定化を priority にし、hash map 依存の出力順非決定性を禁止する
- [ ] Rust IR snapshot と L# IR snapshot の比較ジョブを native backend 完成まで維持する

#### P11-3d: backend parity
- [ ] Wasm backend は既存 Rust 実装の feature parity を先に取り、その後 native backend と共通 codegen 契約へ整理する
- [ ] test runner, wasi helper, snapshot generator を L# 実装へ移植し、生成物検証を Rust ツールに依存させない
- [ ] runtime helper の仕様変更は Wasm/native 同時変更を原則とし、片系だけ先行しない
- [ ] backend ごとの差分は target descriptor と runtime adapter だけへ閉じ込める

#### P11-3e: parity 移行順
- [ ] 移植順を `syntax -> types -> IR -> Wasm backend -> Native backend -> tools` に固定する
- [ ] 各段で Rust 実装を削除せず shadow mode で比較し、2 段連続で CI 緑になってから切替える
- [ ] 切替単位は crate 単位ではなく公開機能単位にし、partial parity でもユーザーに見える挙動が安定したところから既定経路を更新する
- [ ] parity 進捗は TODO だけでなく ADR にも残し、撤去判断の監査証跡にする

#### P11-3f: 完了条件
- [ ] `cargo run -- ...` 相当の既存コマンド群が L# 実装だけで同値動作する
- [ ] Rust 実装を外した状態で parser/type/IR/backend の golden test が全通する
- [ ] examples/stdlib/selfhost の全主要ケースで Rust/L# の差分報告が空になる

### P11-4: ツールチェイン parity
- [ ] L# 製 CLI を正式化し、現行サブコマンド互換の引数仕様と終了コードを固定する
- [ ] L# 製 LSP を正式化し、`initialize/didOpen/didChange/hover/definition/references/rename/formatting/completion/shutdown` を実装する
- [ ] L# 製 formatter/linter を AST 全体対応に拡張し、CLI と LSP の両経路で同一結果を返す
- [ ] docs/review/knowledge/doc-check/doc-ack/install/repl を L# 側へ移植し、VSCode 拡張のバックエンドを Rust LSP からネイティブな L# 実装へ切り替える
- [ ] macOS/Linux/Windows 向けのネイティブ配布形式、クロスビルド手順、署名/パッケージング方針を固定する
- [ ] 完了条件: エンドユーザーが Rust バイナリにも Wasm ランタイムにも触れずにネイティブ配布物だけで開発フローを完走できる

#### P11-4a: CLI parity
- [ ] `parse/check/compile/build/test/review/doc-ack/doc-check/install/repl/lsp/fmt/doc` の引数、標準入出力、終了コードを仕様化する
- [ ] help/version 出力も互換対象に含め、ドキュメント例が壊れないようにする
- [ ] config/lockfile/project init/install は OS 依存 path を吸収した共通 service 経由で実装する
- [ ] CLI smoke test を配布アーカイブ展開後に実行する

#### P11-4b: LSP parity
- [ ] document sync は v1 では full sync に固定し、incremental sync は後段最適化として分離する
- [ ] hover/definition/references/rename/formatting/completion のレスポンス形を Rust 実装と同じ JSON schema に揃える
- [ ] 診断は parse/type/lint を source ごとに安定順で返し、重複診断のマージ規則を固定する
- [ ] VSCode 拡張はネイティブ LSP バイナリを spawn する方式に固定し、Node 側で解析ロジックを持たない

#### P11-4c: formatter / linter parity
- [ ] formatter は parse-format-parse roundtrip と idempotency を gate にする
- [ ] linter は rule id, severity, span, message code を安定化し、LSP/CLI で同一出力にする
- [ ] custom rule API は AST walker 完全化後に公開し、v1 では builtin rule のみ正式サポートとする
- [ ] formatter/linter の設定ファイル仕様を決め、未対応項目は明示的に無視ではなくエラーにする

#### P11-4d: docs / review / knowledge
- [ ] knowledge JSON, review output, doc generator の schema を固定し、CI で snapshot 化する
- [ ] doc-ack/doc-check の trailer 仕様を native CLI でも維持する
- [ ] HTML doc 生成は deterministic 出力にし、タイムスタンプや環境依存パスを埋め込まない
- [ ] docs 系は compiler core から切り離し、library 的に再利用できる service として実装する

#### P11-4e: 配布とパッケージング
- [ ] macOS は `.tar.gz` + 署名/公証、Linux は `.tar.gz`、Windows は `.zip` + `.exe` を v1 配布形に固定する
- [ ] release artifact には `lsharp`, `lsharp-lsp`, `README`, `LICENSE`, `checksums.txt` を同梱する
- [ ] Homebrew/apt/scoop 等のパッケージマネージャ対応は v1 では任意、公式配布アーカイブを正本にする
- [ ] VSCode 拡張は同梱ネイティブ LSP を優先し、PATH 探索は fallback に限定する

#### P11-4f: 完了条件
- [ ] 新規ユーザーが Rust/wasmtime/clang の事前知識なしで CLI と VSCode を起動できる
- [ ] 全主要ツールが同一 native release artifact 群から供給される
- [ ] README の Quick Start が native 配布物だけで完走できる

### P11-5: 長寿命運用のためのランタイム安定化
- [ ] `docs/memory-management-roadmap.md` の M1-M3 を Phase 11 の gate として再接続する
- [ ] compiler/LSP/REPL が共有するヒープオブジェクトに対して GC-safe root 管理を導入する
- [ ] 長寿命 LSP セッション、連続 REPL 実行、自己コンパイル反復で peak memory と回収挙動を測定する
- [ ] 完了条件: bump allocator 前提の短命プロセス設計を脱し、長寿命常駐でも破綻しない

#### P11-5a: collector 導入ゲート
- [ ] Phase M1-M3 の各マイルストーンを compiler/LSP/REPL の smoke test と紐付ける
- [ ] GC 未導入モードと GC 有効モードを同一 API で切り替えられるようにし、比較実験を可能にする
- [ ] object header, trace map, root stack の仕様を backend 仕様書へ再掲し、実装差分を禁止する

#### P11-5b: 長寿命ワークロード
- [ ] 1,000 回連続 format、1,000 回連続 hover、100 回連続 self-compile を標準 longevity benchmark に固定する
- [ ] LSP セッションで open/change/diagnostics/hover/completion を繰り返す soak test を追加する
- [ ] REPL は stateful 実装に切り替える場合でも同じ GC 契約で回ることを別系統で検証する

#### P11-5c: 観測と失敗解析
- [ ] peak RSS, heap bytes, live object count, GC pause time, full GC count を収集項目に固定する
- [ ] CI では簡易メトリクス、手元ベンチでは詳細トレースの 2 段階に分ける
- [ ] メモリリーク検知時は object tag ごとの残存数を出力し、どの型が残ったか追えるようにする

#### P11-5d: 完了条件
- [ ] native LSP/REPL/compiler の長寿命実行でヒープが単調増加しない
- [ ] collector 有効時も selfhost bootstrap の fixed-point が崩れない
- [ ] GC 由来の既知クラッシュが TODO の open issue から消える

### P11-6: CI 切替と Rust 撤去
- [ ] CI の主経路を `cargo test` 中心から `stageN.wasm` 中心へ切り替える
- [ ] Rust 実装は比較専用ジョブに一時隔離し、fixed-point と golden parity が安定した時点で削除する
- [ ] `Cargo.toml` workspace と `crates/` を削除し、README/book/CI docs を L# ネイティブ正式版前提に更新する
- [ ] ネイティブ release artifact の生成、署名、配布、回帰テストを CI に組み込む
- [ ] 完了条件: リポジトリの正本実装が L# のみになり、Rust 不在で clone 直後から bootstrap とネイティブ配布手順が成立する

#### P11-6a: CI 再編
- [ ] CI job を `bootstrap-wasm`, `bootstrap-native`, `golden-parity`, `release-smoke`, `packaging`, `docs` に再編する
- [ ] 既存 `cargo test/clippy/fmt` は Rust 撤去まで shadow job として残し、required check は段階的に切り替える
- [ ] branch protection の required status を `CI Gate` 単独から新 job 群へ更新し、[docs/CI.md](/Users/biwakonbu/github/lsharp/docs/CI.md) を同期する
- [ ] CI artifact の保存対象を wasm binaries, native binaries, object files, diff reports, release bundles に固定する

#### P11-6b: Rust 隔離フェーズ
- [ ] Rust 実装は `legacy-rust-bootstrap` のような隔離ディレクトリ/ブランチ方針を決め、正本ツリーから段階的に外す
- [ ] mainline の既定コマンド、README、CI は L# 実装を優先し、Rust 実装は比較専用であることを明記する
- [ ] 最終削除前に `legacy` ラベル付き最終 commit/tag を切り、参照点を固定する
- [ ] Rust 削除は crates 単位ではなく feature parity 完了単位で順次行い、中途半端な dead code を残さない

#### P11-6c: リリース運用
- [ ] semantic versioning, artifact naming, checksum, changelog, signing 手順を release playbook として固定する
- [ ] nightly と stable の 2 チャネルを分け、selfhost/native はまず nightly で焼いてから stable へ昇格させる
- [ ] crash report/diagnostic dump の収集方針を決め、native 配布物の障害解析手段を確保する
- [ ] リリースごとに CLI/LSP/VSCode extension の互換表を生成し、同梱物の整合を確認する

#### P11-6d: 最終撤去条件
- [ ] Rust 依存が build, test, release, editor integration のどこにも残っていない
- [ ] fresh clone から native release 生成までを Rust なしで再現できる
- [ ] rollback 手順が文書化され、最後の Rust リリースへ戻せることが確認済み

---

## 既知の制限事項

### リニアメモリランタイム
- [~] Precise Tracing GC 導入 -- mainline 方針。linear memory 上で shadow stack + mark-sweep を実装。現在の bump allocator (__alloc) は安定動作、GC 導入前のオブジェクトヘッダ/レイアウトの検証テスト 7件追加。docs/memory-management-roadmap.md に Phase 0-6 の詳細ロードマップを記載
- [~] 世代別 GC 最適化 -- docs/memory-management-roadmap.md Phase 4 に設計を記載。young=bump allocator, old=non-moving mark-sweep。First Collector (Phase 3) 完了後に着手
- [~] Region 最適化 -- docs/memory-management-roadmap.md Phase 5 に設計を記載。GC の補助最適化として段階導入
- [~] WasmGC 最適化バックエンド -- docs/memory-management-roadmap.md Phase 6 に設計を記載。optional backend として browser/対応ランタイム向け
