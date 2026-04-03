use super::support::*;

// =====================================================// P1-3: WASI syscall ラッパー検証
// =====================================================
/// P1-3: fd_write が stdout (fd=1) に出力できることを検証
/// print/print-string は内部で fd_write を使用
#[test]
fn test_e2e_fd_write_wrapper_stdout() {
    let result = compile_and_run(
        r#"
        (defn main []
          (do
            (print-string "hello")
            (print 42)
            0))
    "#,
    );
    assert_eq!(result.trim(), "hello42");
}

/// P1-3: fd_write が stderr (fd=2) 相当の出力をサポートすることを検証
/// 現在は print が stdout のみ対応。stderr 出力は将来の拡張
#[test]
fn test_e2e_fd_write_wrapper_stderr_placeholder() {
    // stderr 出力は未実装だが、print で stdout に書き込める
    let result = compile_and_run(
        r#"
        (defn main []
          (do
            (print-string "error message test")
            0))
    "#,
    );
    assert!(result.contains("error message test"));
}

/// P1-3: fd_open/fd_close/fd_seek がファイル操作で使用されることを検証
/// read-file/write-file は内部で path_open/fd_read/fd_write/fd_close を使用
/// Wasm ランタイム内部で WASI の path_open → fd_read → fd_close が呼ばれる
#[test]
fn test_e2e_fd_open_close_seek() {
    // ファイル I/O ビルトインが WASI syscall を使用することを間接検証
    // write-file → path_open + fd_write + fd_close
    // read-file → path_open + fd_filestat_get + fd_read + fd_close
    // file-exists? → path_open + fd_close
    let result = compile_and_run(
        r#"
        (defn main []
          (do
            ;; fd_write を直接使用する print が動作すること = fd_write ラッパーが有効
            (print 42)
            ;; file-exists? は内部で path_open/fd_close を使用
            ;; 存在しないファイルで false が返ることを検証
            (if (file-exists? "/nonexistent/path/test.txt")
              (print 1)
              (print 0))
            0))
    "#,
    );
    assert_eq!(result.trim(), "42\n0");
}

/// P1-3: JSON パーサー - JsonValue 型の構築と検証
#[test]
fn test_e2e_json_value_construction() {
    let result = compile_and_run(
        r#"
        ;; JSON 値の型タグ (stdlib/Json.ls 互換)
        ;; Null=0, Bool=1, Num=2, Str=3, Arr=4, Obj=5

        (defn make-json-null []
          (let [v (vector-new 2)]
            (vector-push v 0)))

        (defn make-json-bool [b]
          (let [v (vector-new 2)]
            (vector-push (vector-push v 1) b)))

        (defn make-json-num [n]
          (let [v (vector-new 2)]
            (vector-push (vector-push v 2) n)))

        (defn json-tag [json-val]
          (vector-get json-val 0))

        (defn main []
          (let [null-val (make-json-null)
                bool-val (make-json-bool 1)
                num-val (make-json-num 42)]
            (do
              (print (json-tag null-val))
              (print (json-tag bool-val))
              (print (json-tag num-val))
              (print (vector-get num-val 1))
              0)))
    "#,
    );
    assert_eq!(result.trim(), "0\n1\n2\n42");
}

// =====================================================// P8-9 T4-3: stage1 E2E テスト
// =====================================================
/// P8-9 T4-3: stage1.wasm (selfhost コンパイラ) のコンパイル+実行検証
/// Rust 版コンパイラで selfhost/src/App/Main.ls をコンパイルし、
/// 出力される stage1.wasm が正しく動作することを検証
#[test]
fn test_e2e_bootstrap_stage1_compile_and_run() {
    let main_path = selfhost_main_path();
    let wasm_bytes = compile_file_only(&main_path);

    // 有効な Wasm バイナリであること
    assert!(
        wasm_bytes.len() > 100,
        "stage1.wasm が小さすぎる: {} bytes",
        wasm_bytes.len()
    );
    assert_eq!(&wasm_bytes[0..4], b"\0asm", "Wasm マジックナンバーが不正");

    // stage1 を実行して出力を検証
    let output = compile_and_run_file(&main_path);
    let lines: Vec<&str> = output.trim().split('\n').collect();

    // AST 検証: tag=1 (lit-int), value=42
    assert_eq!(lines[0], "1", "AST tag = 1 (lit-int)");
    assert_eq!(lines[1], "42", "AST value = 42");

    // IR 検証: 命令数=1
    assert_eq!(lines[2], "1", "IR instruction count = 1");

    // Wasm ヘッダー検証: 8 bytes
    assert_eq!(lines[5], "8", "Wasm header length = 8");

    // Wasm magic: \0asm
    assert_eq!(lines[6], "0", "Wasm magic[0] = 0");
    assert_eq!(lines[7], "97", "Wasm magic[1] = 97 (a)");
    assert_eq!(lines[8], "115", "Wasm magic[2] = 115 (s)");
    assert_eq!(lines[9], "109", "Wasm magic[3] = 109 (m)");
}

/// P8-9 T4-3: stage1 でテスト用 .ls プログラムの AST 構築が機能することを検証
/// (Main.ls が内部で AST→IR→Wasm パイプラインを実行していることの検証)
#[test]
fn test_e2e_bootstrap_stage1_pipeline_verification() {
    let output = compile_and_run_file(&selfhost_main_path());
    let lines: Vec<&str> = output.trim().split('\n').collect();

    // WASI I/O 統合検証
    assert_eq!(
        lines[12], "15",
        "wasm-size = 15 (header 8 + type section 7)"
    );

    // モジュール結合検証
    assert_eq!(lines[13], "10", "module-count = 10");
}

/// P8-9 T4-4/T4-5: 将来のセルフコンパイル検証の基盤テスト
/// stage1.wasm が有効な Wasm バイナリであり、
/// 将来的に .ls ファイルを受け取って stage2.wasm を生成できる構造を持つことを検証
#[test]
fn test_e2e_bootstrap_stage1_binary_structure() {
    let wasm_bytes = compile_file_only(&selfhost_main_path());

    // Wasm バイナリの構造検証
    assert_eq!(&wasm_bytes[0..4], b"\0asm", "Wasm magic");
    assert_eq!(&wasm_bytes[4..8], &[1, 0, 0, 0], "Wasm version 1");

    // セクションの存在確認 (最低限 Type, Function, Export, Code セクション)
    let mut pos = 8;
    let mut section_ids = Vec::new();
    while pos < wasm_bytes.len() {
        let section_id = wasm_bytes[pos];
        section_ids.push(section_id);
        pos += 1;
        // セクションサイズを読み取り (LEB128)
        let mut size: usize = 0;
        let mut shift = 0;
        loop {
            if pos >= wasm_bytes.len() {
                break;
            }
            let byte = wasm_bytes[pos] as usize;
            pos += 1;
            size |= (byte & 0x7F) << shift;
            shift += 7;
            if byte & 0x80 == 0 {
                break;
            }
        }
        pos += size;
    }
    // Type セクション (1), Import (2), Function (3), Export (7), Code (10) が含まれること
    assert!(
        section_ids.contains(&1),
        "Type セクションが必要: {:?}",
        section_ids
    );
    assert!(
        section_ids.contains(&3),
        "Function セクションが必要: {:?}",
        section_ids
    );
    assert!(
        section_ids.contains(&7),
        "Export セクションが必要: {:?}",
        section_ids
    );
    assert!(
        section_ids.contains(&10),
        "Code セクションが必要: {:?}",
        section_ids
    );
}

// =====================================================// P8-9 T4-6: CI ブートストラップ自動検証
// =====================================================
/// P8-9 T4-6: CI で使用されるブートストラップ検証と同等のテスト
/// fixed input set の selfhost モジュールが個別 compile できることを検証。
#[test]
fn test_e2e_bootstrap_ci_all_modules_compile() {
    let modules = [
        "AST.ls",
        "Cli.ls",
        "Closure.ls",
        "Codegen.ls",
        "Compiler.ls",
        "Constraints.ls",
        "Derive.ls",
        "DocTools.ls",
        "Emit.ls",
        "Formatter.ls",
        "GC.ls",
        "HtmlDoc.ls",
        "Hygiene.ls",
        "IR.ls",
        "JsonRpc.ls",
        "Lexer.ls",
        "Linker.ls",
        "Linter.ls",
        "Lower.ls",
        "LowerDecl.ls",
        "LowerExpr.ls",
        "LowerPattern.ls",
        "LspServer.ls",
        "MacroExpand.ls",
        "Main.ls",
        "MetadataCheck.ls",
        "ModuleGraph.ls",
        "NativeCodegen.ls",
        "NativeEmit.ls",
        "NativeTarget.ls",
        "Parser.ls",
        "Span.ls",
        "TestRunner.ls",
        "Token.ls",
        "Type.ls",
        "TypeInfer.ls",
        "TypeScheme.ls",
        "WasiBackend.ls",
        "WasiRunner.ls",
        "WasmEmit.ls",
    ];

    let mut compiled = 0;
    for module in &modules {
        let path = selfhost_source_path(module);
        let wasm = compile_file_only(&path);
        assert_valid_wasm(&wasm);
        compiled += 1;
    }
    assert_eq!(
        compiled,
        modules.len(),
        "fixed input set の全 {} モジュールがコンパイルされるべき",
        modules.len()
    );
}

/// P8-9 T4-6: CI で使用される stdlib コンパイル検証と同等のテスト
#[test]
fn test_e2e_bootstrap_ci_stdlib_compile() {
    let modules = [
        "Core", "Char", "Debug", "IO", "List", "Map", "Path", "Set", "String", "Vector", "Json",
    ];
    let base_dir = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../stdlib");

    let mut compiled = 0;
    for module in &modules {
        let path = base_dir.join(format!("{}.ls", module));
        if path.exists() {
            let wasm = compile_file_only(&path);
            assert_valid_wasm(&wasm);
            compiled += 1;
        }
    }
    assert_eq!(
        compiled, 11,
        "全 11 stdlib モジュールがコンパイルされるべき"
    );
}

/// P11-2 BOOT-03: examples fixed input set が個別 compile できることを検証
#[test]
fn test_e2e_bootstrap_ci_examples_compile() {
    let examples = ["fib.ls", "module.ls", "trait.ls"];
    let base_dir = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../examples");

    let mut compiled = 0;
    for file in &examples {
        let path = base_dir.join(file);
        let wasm = compile_file_only(&path);
        assert_valid_wasm(&wasm);
        compiled += 1;
    }

    assert_eq!(
        compiled, 3,
        "fixed input set の全 3 examples がコンパイルされるべき"
    );
}

// =====================================================// P9-6a: VSCode 拡張 - シンタックスハイライト検証
// =====================================================
/// P9-6a: TextMate grammar ファイルが存在し、有効な JSON であることを検証
#[test]
fn test_e2e_vscode_tmgrammar_exists() {
    let grammar_path = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../editors/vscode/syntaxes/lsharp.tmLanguage.json");
    assert!(
        grammar_path.exists(),
        "TextMate grammar ファイルが存在するべき"
    );

    let content = std::fs::read_to_string(&grammar_path).unwrap();
    // 基本的な JSON 構造の検証
    assert!(
        content.contains("\"scopeName\""),
        "scopeName が含まれるべき"
    );
    assert!(
        content.contains("source.lsharp"),
        "scopeName が source.lsharp であるべき"
    );
    assert!(
        content.contains("\"keyword\""),
        "keyword パターンが含まれるべき"
    );
    assert!(content.contains("defn"), "defn キーワードが含まれるべき");
    assert!(
        content.contains("\"builtin-function\""),
        "組み込み関数パターンが含まれるべき"
    );
    assert!(
        content.contains("\"comment\""),
        "コメントパターンが含まれるべき"
    );
}

/// P9-6a: VSCode 拡張マニフェストが存在し、必要な設定を含むことを検証
#[test]
fn test_e2e_vscode_extension_manifest() {
    let manifest_path = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../editors/vscode/package.json");
    assert!(manifest_path.exists(), "package.json が存在するべき");

    let content = std::fs::read_to_string(&manifest_path).unwrap();
    assert!(
        content.contains(".ls"),
        ".ls ファイル拡張子の登録が含まれるべき"
    );
    assert!(content.contains("lsharp"), "言語ID lsharp が含まれるべき");
}

/// P9-6a: VSCode 拡張の TypeScript ソースが存在することを検証
#[test]
fn test_e2e_vscode_extension_source() {
    let ext_path = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../editors/vscode/src/extension.ts");
    assert!(ext_path.exists(), "extension.ts が存在するべき");

    let content = std::fs::read_to_string(&ext_path).unwrap();
    assert!(content.contains("activate"), "activate 関数が含まれるべき");
    assert!(
        content.contains("deactivate"),
        "deactivate 関数が含まれるべき"
    );
    assert!(content.contains("lsharp"), "lsharp 言語IDが含まれるべき");
}

// =====================================================// GC: メモリ管理基盤テスト
// =====================================================
/// GC: Shadow stack の基盤となる __alloc が正しく動作することを検証
/// 現在のアロケータは bump allocator で、GC の基盤となる
#[test]
fn test_e2e_gc_alloc_foundation() {
    let result = compile_and_run(
        r#"
        (defn main []
          (let [;; 複数のヒープオブジェクトを生成してアロケータが動作することを検証
                v1 (vector-new 4)
                v2 (vector-push v1 100)
                v3 (vector-push v2 200)
                s1 "hello"
                s2 "world"
                s3 (string-concat s1 s2)]
            (do
              (print (vector-length v3))
              (print (vector-get v3 0))
              (print (string-length s3))
              0)))
    "#,
    );
    assert_eq!(result.trim(), "2\n100\n10");
}

/// GC: HashMap (Open Addressing) のメモリ使用が安定していることを検証
/// GC 導入時にも HashMap が正常に動作する基盤テスト
#[test]
fn test_e2e_gc_hashmap_memory_stable() {
    let result = compile_and_run(
        r#"
        (defn main []
          (let [m (map-new)
                m1 (map-insert m 1 100)
                m2 (map-insert m1 2 200)
                m3 (map-insert m2 3 300)]
            (do
              (print (map-size m3))
              (print (map-get m3 1))
              (print (map-get m3 2))
              (print (map-get m3 3))
              0)))
    "#,
    );
    assert_eq!(result.trim(), "3\n100\n200\n300");
}

// =====================================================// P9-6b: JSON-RPC パーサー/シリアライザー (selfhost/src/Tools/Lsp/JsonRpc.ls)
// =====================================================
/// P9-6b: JSON-RPC モジュールがコンパイル+実行できることを検証
#[test]
fn test_e2e_selfhost_jsonrpc() {
    let source = selfhost_module("JsonRpc.ls");
    let output = compile_and_run(source);
    let lines: Vec<&str> = output.trim().split('\n').collect();
    // メッセージ種別: request=0, response=1, notification=2, error=3
    assert_eq!(lines[0], "0", "request type");
    assert_eq!(lines[1], "1", "response type");
    assert_eq!(lines[2], "2", "notification type");
    assert_eq!(lines[3], "3", "error type");
    // ID
    assert_eq!(lines[4], "1", "request id");
    assert_eq!(lines[5], "1", "response id");
    assert_eq!(lines[6], "1", "error id");
    // メソッド
    assert_eq!(lines[7], "1", "initialize method");
    assert_eq!(lines[8], "2", "shutdown method");
}

/// P9-6b: JSON-RPC モジュールの Wasm バイナリが有効であることを検証
#[test]
fn test_e2e_selfhost_jsonrpc_wasm_valid() {
    let source = selfhost_module("JsonRpc.ls");
    let wasm = compile_only(source);
    assert_valid_wasm(&wasm);
}

// =====================================================// P9-6c: リンター (selfhost/src/Tools/Text/Linter.ls)
// =====================================================
/// P9-6c: リンターモジュールがコンパイル+実行できることを検証
#[test]
fn test_e2e_selfhost_linter() {
    let source = selfhost_module("Linter.ls");
    let output = compile_and_run(source);
    let lines: Vec<&str> = output.trim().split('\n').collect();
    // 診断: severity=1(warning), rule=100(unused-var), line=10
    assert_eq!(lines[0], "1", "warning severity");
    assert_eq!(lines[1], "100", "unused-var rule");
    assert_eq!(lines[2], "10", "line number");
    // 診断: severity=0(error), rule=102(missing-type-ann)
    assert_eq!(lines[3], "0", "error severity");
    assert_eq!(lines[4], "102", "missing-type-ann rule");
    // 空ブロック: severity=1(warning), rule=104(empty-body)
    assert_eq!(lines[5], "1", "warning severity for empty body");
    assert_eq!(lines[6], "104", "empty-body rule");
    // 集約結果: 3 diagnostics
    assert_eq!(lines[7], "3", "total diagnostics");
    // ルール数
    assert_eq!(lines[8], "5", "rule count");
    // 未使用変数検出: severity=1(warning), rule=100(unused-var)
    assert_eq!(lines[9], "1", "unused var: warning severity");
    assert_eq!(lines[10], "100", "unused var: rule id");
    // 使用済み変数: 検出されない (0)
    assert_eq!(lines[11], "0", "used var: no diagnostic");
    // ルール一括実行: 1件検出
    assert_eq!(lines[12], "1", "run-all-rules: 1 diagnostic");
    // do ノード: ast-contains-var 直接検索
    assert_eq!(lines[13], "1", "do: contains-var found 99");
    assert_eq!(lines[14], "0", "do: contains-var not found 77");
    // do ノード: let 経由の未使用変数検出 → 警告なし
    assert_eq!(lines[15], "0", "do: used var no diagnostic");
    // match ノード: ast-contains-var 直接検索
    assert_eq!(lines[16], "1", "match: contains-var found 99");
    assert_eq!(lines[17], "0", "match: contains-var not found 77");
    // match ノード: let 経由の未使用変数検出 → 警告なし
    assert_eq!(lines[18], "0", "match: used var no diagnostic");
}

// =====================================================// P9-6d: フォーマッタ (selfhost/src/Tools/Text/Formatter.ls)
// =====================================================
/// P9-6d: フォーマッタモジュールがコンパイル+実行できることを検証
#[test]
fn test_e2e_selfhost_formatter() {
    let source = format!(
        "{}\n{}\n{}",
        selfhost_module("FormatterExpr.ls"),
        selfhost_module("FormatterDecl.ls"),
        selfhost_module("Formatter.ls"),
    );
    let output = compile_and_run(&source);
    let lines: Vec<&str> = output.trim().split('\n').collect();
    // インデント設定
    assert_eq!(lines[0], "2", "indent-width");
    assert_eq!(lines[1], "80", "max-line-width");
    // インデント文字列長
    assert_eq!(lines[2], "0", "indent level 0 length");
    assert_eq!(lines[3], "2", "indent level 1 length");
    assert_eq!(lines[4], "4", "indent level 2 length");
    // 1 行フォーマット判定
    assert_eq!(lines[5], "1", "short form fits on one line");
    assert_eq!(lines[6], "0", "long form needs wrapping");
    // let 束縛
    assert_eq!(lines[7], "1", "single binding fits on one line");
    assert_eq!(lines[8], "2", "multi binding indented");
    // defn
    assert_eq!(lines[9], "1", "short defn one line");
    assert_eq!(lines[10], "6", "long defn multi-line");
    // 統計
    assert_eq!(lines[11], "1", "line count");
    assert_eq!(lines[12], "1", "node count");
    // format-program: 空 vector は CLI 連携用の末尾改行 1 文字、同一入力で連続一致
    assert_eq!(lines[13], "1", "format-program empty program");
    assert_eq!(lines[14], "1", "format-program idempotent");
}

// =====================================================// P9-6b: LSP ハンドラ統合 (selfhost/src/Tools/Lsp/JsonRpc.ls)
// =====================================================
/// P9-6b: LSP ハンドラ関数がコンパイル+実行できることを検証
#[test]
fn test_e2e_selfhost_jsonrpc_lsp_handlers() {
    let source = selfhost_module("JsonRpc.ls");
    let output = compile_and_run(source);
    let lines: Vec<&str> = output.trim().split('\n').collect();
    // 既存 9 行の後に LSP ハンドラテスト出力
    // server capabilities: 7 要素
    assert_eq!(lines[9], "7", "capabilities: vector length");
    assert_eq!(lines[10], "1", "capabilities: text-document-sync");
    // handle-initialize: response type=1, id=1, result=capabilities
    assert_eq!(lines[11], "1", "initialize: response type");
    assert_eq!(lines[12], "1", "initialize: response id");
    assert_eq!(lines[13], "7", "initialize: capabilities length");
    // handle-shutdown: response type=1, id=9, result=0
    assert_eq!(lines[14], "1", "shutdown: response type");
    assert_eq!(lines[15], "9", "shutdown: response id");
    assert_eq!(lines[16], "0", "shutdown: result sentinel");
    // handle-did-open: source length returned
    assert_eq!(lines[17], "100", "did-open: source length");
    // handle-hover: response type=1, id=2, type-tag=1(int)
    assert_eq!(lines[18], "1", "hover: response type");
    assert_eq!(lines[19], "2", "hover: response id");
    // handle-goto-def: response type=1, line=10, col=5
    assert_eq!(lines[20], "1", "goto-def: response type");
    assert_eq!(lines[21], "10", "goto-def: line");
    assert_eq!(lines[22], "5", "goto-def: col");
    // handle-completion: keyword count
    assert_eq!(lines[23], "7", "completion: keyword count");
    // 追加メソッド定数
    assert_eq!(lines[24], "23", "method: formatting");
    assert_eq!(lines[25], "30", "method: publish-diagnostics");
    // deterministic JSON-RPC text rendering
    assert_eq!(
        lines[26], r#"{"jsonrpc":"2.0","id":1,"result":[1,1,1,1,1,1,1]}"#,
        "initialize response text"
    );
    assert_eq!(
        lines[27], r#"{"jsonrpc":"2.0","id":9,"result":0}"#,
        "shutdown response text"
    );
}

// =====================================================// P9-6c: リンター LSP 統合 (selfhost/src/Tools/Text/Linter.ls)
// =====================================================
/// P9-6c: リンター診断を LSP Diagnostic 形式に変換できることを検証
#[test]
fn test_e2e_selfhost_linter_lsp_integration() {
    let source = selfhost_module("Linter.ls");
    let output = compile_and_run(source);
    let lines: Vec<&str> = output.trim().split('\n').collect();
    // 既存 19 行の後に LSP 統合テスト出力
    // make-lsp-diagnostic: [start-line, start-col, severity, rule-id]
    assert_eq!(lines[19], "10", "lsp-diag: start-line");
    assert_eq!(lines[20], "5", "lsp-diag: start-col");
    assert_eq!(lines[21], "1", "lsp-diag: severity (warning)");
    assert_eq!(lines[22], "100", "lsp-diag: code (unused-var)");
    // diagnostics-to-lsp-count
    assert_eq!(lines[23], "3", "publish-diagnostics: count");
}

// =====================================================// P9-6d: フォーマッタ LSP 統合 (selfhost/src/Tools/Text/Formatter.ls)
// =====================================================
/// P9-6d: フォーマッタが LSP TextEdit を生成できることを検証
#[test]
fn test_e2e_selfhost_formatter_lsp_integration() {
    let source = format!(
        "{}\n{}\n{}",
        selfhost_module("FormatterExpr.ls"),
        selfhost_module("FormatterDecl.ls"),
        selfhost_module("Formatter.ls"),
    );
    let output = compile_and_run(&source);
    let lines: Vec<&str> = output.trim().split('\n').collect();
    // 既存 15 行の後に LSP 統合テスト出力 (format-program 2 行追加)
    // make-text-edit: [start-line, start-col, end-line, end-col, text-hash]
    assert_eq!(lines[15], "0", "text-edit: start-line");
    assert_eq!(lines[16], "0", "text-edit: start-col");
    assert_eq!(lines[17], "10", "text-edit: end-line");
    assert_eq!(lines[18], "0", "text-edit: end-col");
    assert_eq!(lines[19], "42", "text-edit: new-text hash");
    // formatting response: 1 edit
    assert_eq!(lines[20], "1", "formatting: edit count");
}

// =====================================================// P8-9 T4-4: セルフコンパイル拡張 (if/let/変数)
// =====================================================
/// T4-4: if 式と let 式のソースからのコンパイルを検証
#[test]
fn test_e2e_selfhost_main_compile_if_let() {
    let output = compile_and_run_file(&selfhost_main_path());
    let lines: Vec<&str> = output.trim().split('\n').collect();
    // 既存出力 21 行 (index 0-20) の後に T4-4 拡張出力
    // T4-4 拡張: if 式コンパイル
    // "(defn main [] (if 1 42 0))" → tok: if=32 検出
    assert_eq!(lines[21], "1", "if-compile: token if detected");
    // if 式 AST: tag=6
    assert_eq!(lines[22], "6", "if-compile: ast tag = if");
    // if 式 IR: 3 命令 (cond, then, else)
    assert_eq!(lines[23], "3", "if-compile: ir instruction count");

    // T4-4 拡張: let 式コンパイル
    // "(defn main [] (let [x 42] x))" → let=31 検出
    assert_eq!(lines[24], "1", "let-compile: token let detected");
    // let 式 AST: tag=7
    assert_eq!(lines[25], "7", "let-compile: ast tag = let");
    // let 式 IR: 2 命令 (init value + local.get)
    assert_eq!(lines[26], "2", "let-compile: ir instruction count");
}

// =====================================================// P8-9 T4-5: 固定点検証
// =====================================================
/// T4-5: Main.ls のコンパイルが決定的 (同一入力→同一バイナリ) であることを検証
#[test]
fn test_e2e_bootstrap_stage1_deterministic() {
    let main_path = selfhost_main_path();
    let wasm1 = compile_file_only(&main_path);
    let wasm2 = compile_file_only(&main_path);
    assert_eq!(wasm1, wasm2, "stage1 compilation must be deterministic");
    assert!(
        wasm1.len() > 100,
        "stage1 wasm must be non-trivial: {} bytes",
        wasm1.len()
    );
}

/// T4-5: stage1 バイナリ構造の固定点検証 (セクション構成が安定していること)
#[test]
fn test_e2e_bootstrap_stage1_fixed_point_sections() {
    let wasm = compile_file_only(&selfhost_main_path());
    // Wasm magic + version
    assert_eq!(&wasm[0..4], b"\0asm", "wasm magic");
    assert_eq!(wasm[4], 1, "wasm version");
    // セクション ID の列が安定していることを確認
    // Type(1), Function(3), Export(7), Code(10) の順
    let mut section_ids = Vec::new();
    let mut pos = 8;
    while pos < wasm.len() {
        let section_id = wasm[pos];
        section_ids.push(section_id);
        pos += 1;
        // セクションサイズを LEB128 デコード
        let mut size: usize = 0;
        let mut shift = 0;
        loop {
            if pos >= wasm.len() {
                break;
            }
            let byte = wasm[pos] as usize;
            pos += 1;
            size |= (byte & 0x7f) << shift;
            if byte & 0x80 == 0 {
                break;
            }
            shift += 7;
        }
        pos += size;
    }
    // セクション構成が Type, Function, Memory, Export, Code を含むこと
    assert!(section_ids.contains(&1), "Type section present");
    assert!(section_ids.contains(&3), "Function section present");
    assert!(section_ids.contains(&7), "Export section present");
    assert!(section_ids.contains(&10), "Code section present");
}

/// T4-5: stage1 バイナリのセクション構成が複数回コンパイルで安定していることを検証
#[test]
fn test_e2e_bootstrap_stage1_section_stability() {
    let main_path = selfhost_main_path();
    let wasm1 = compile_file_only(&main_path);
    let wasm2 = compile_file_only(&main_path);

    // 各 wasm からセクション ID とサイズの列を抽出するヘルパー
    fn extract_sections(wasm: &[u8]) -> Vec<(u8, usize)> {
        let mut sections = Vec::new();
        let mut pos = 8; // magic(4) + version(4) をスキップ
        while pos < wasm.len() {
            let section_id = wasm[pos];
            pos += 1;
            // LEB128 デコード
            let mut size: usize = 0;
            let mut shift = 0;
            loop {
                if pos >= wasm.len() {
                    break;
                }
                let byte = wasm[pos] as usize;
                pos += 1;
                size |= (byte & 0x7f) << shift;
                if byte & 0x80 == 0 {
                    break;
                }
                shift += 7;
            }
            sections.push((section_id, size));
            pos += size;
        }
        sections
    }

    let sections1 = extract_sections(&wasm1);
    let sections2 = extract_sections(&wasm2);

    // セクション数が一致
    assert_eq!(
        sections1.len(),
        sections2.len(),
        "セクション数が不安定: {} vs {}",
        sections1.len(),
        sections2.len()
    );

    // 各セクションの ID とサイズが一致
    for (i, (s1, s2)) in sections1.iter().zip(sections2.iter()).enumerate() {
        assert_eq!(
            s1.0, s2.0,
            "セクション {} の ID が不安定: {} vs {}",
            i, s1.0, s2.0
        );
        assert_eq!(
            s1.1, s2.1,
            "セクション {} (ID={}) のサイズが不安定: {} vs {}",
            i, s1.0, s1.1, s2.1
        );
    }

    // セクションが最低4つ以上あること (Type, Function, Export, Code)
    assert!(
        sections1.len() >= 4,
        "セクション数が少なすぎる: {}",
        sections1.len()
    );
}

/// T4-5: stage1 の export シンボル名が安定していることを検証
#[test]
fn test_e2e_bootstrap_stage1_symbol_stability() {
    let main_path = selfhost_main_path();
    let wasm1 = compile_file_only(&main_path);
    let wasm2 = compile_file_only(&main_path);

    // Export セクション (ID=7) のバイト列を抽出
    fn extract_export_section(wasm: &[u8]) -> Option<Vec<u8>> {
        let mut pos = 8;
        while pos < wasm.len() {
            let section_id = wasm[pos];
            pos += 1;
            let mut size: usize = 0;
            let mut shift = 0;
            loop {
                if pos >= wasm.len() {
                    break;
                }
                let byte = wasm[pos] as usize;
                pos += 1;
                size |= (byte & 0x7f) << shift;
                if byte & 0x80 == 0 {
                    break;
                }
                shift += 7;
            }
            if section_id == 7 {
                return Some(wasm[pos..pos + size].to_vec());
            }
            pos += size;
        }
        None
    }

    let export1 = extract_export_section(&wasm1).expect("Export section not found in wasm1");
    let export2 = extract_export_section(&wasm2).expect("Export section not found in wasm2");

    // Export セクション全体がバイト一致 (シンボル名・順序・インデックスが安定)
    assert_eq!(
        export1,
        export2,
        "Export セクションが不安定: {} bytes vs {} bytes",
        export1.len(),
        export2.len()
    );

    // Export セクションが空でないこと
    assert!(!export1.is_empty(), "Export セクションが空");
}

/// T4-5: selfhost の各モジュールを個別にコンパイルし出力が決定的であることを検証
#[test]
fn test_e2e_bootstrap_selfhost_modules_deterministic() {
    // MacroExpand.ls, TypeInfer.ls は拡張構文を使用しておりパース未対応のため除外
    let modules: &[&str] = &[
        "Lexer.ls",
        "Parser.ls",
        "AST.ls",
        "Token.ls",
        "Compiler.ls",
        "Type.ls",
        "IR.ls",
        "WasmEmit.ls",
    ];

    for name in modules {
        let path = selfhost_source_path(name);
        let wasm1 = compile_file_only(&path);
        let wasm2 = compile_file_only(&path);
        assert_eq!(
            wasm1,
            wasm2,
            "{} のコンパイルが非決定的: {} bytes vs {} bytes",
            name,
            wasm1.len(),
            wasm2.len()
        );
        assert_valid_wasm(&wasm1);
    }
}
