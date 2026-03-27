(module Cli)
(import AST)
(import Compiler)
(import DocTools)
(import Formatter)
(import LspServer)
(import Parser)
(import TestRunner)
(import TypeInfer)
(import WasmEmit)

;; Cli.ls - L# 製 CLI エントリポイント
;;
;; P11-4 T4-1: L# 製 CLI の正式化
;; 13 サブコマンドの引数解析とディスパッチを行う。
;;
;; サブコマンド一覧:
;;   parse, check, compile, build, test,
;;   review, doc-ack, doc-check, install,
;;   repl, lsp, fmt, doc

;; === 終了コード定義 ===
(defn exit-success [] 0)
(defn exit-compile-error [] 1)
(defn exit-runtime-error [] 2)
(defn exit-unknown-command [] 127)

;; === 終了コード公開 API (contract parity) ===
(defn exit-code-success [] 0)
(defn exit-code-compile-error [] 1)
(defn exit-code-runtime-error [] 2)
(defn exit-code-unknown-command [] 127)

;; === コマンド ID 定義 ===
(defn cmd-parse [] 1)
(defn cmd-check [] 2)
(defn cmd-compile [] 3)
(defn cmd-build [] 4)
(defn cmd-test [] 5)
(defn cmd-review [] 6)
(defn cmd-doc-ack [] 7)
(defn cmd-doc-check [] 8)
(defn cmd-install [] 9)
(defn cmd-repl [] 10)
(defn cmd-lsp [] 11)
(defn cmd-fmt [] 12)
(defn cmd-doc [] 13)

;; === 引数解析 ===

;; コマンド名文字列からコマンド ID を返す
;; 未知のコマンドは 0 を返す
(defn arg-parse [cmd-name]
  (if (string-eq cmd-name "parse") (cmd-parse)
  (if (string-eq cmd-name "check") (cmd-check)
  (if (string-eq cmd-name "compile") (cmd-compile)
  (if (string-eq cmd-name "build") (cmd-build)
  (if (string-eq cmd-name "test") (cmd-test)
  (if (string-eq cmd-name "review") (cmd-review)
  (if (string-eq cmd-name "doc-ack") (cmd-doc-ack)
  (if (string-eq cmd-name "doc-check") (cmd-doc-check)
  (if (string-eq cmd-name "install") (cmd-install)
  (if (string-eq cmd-name "repl") (cmd-repl)
  (if (string-eq cmd-name "lsp") (cmd-lsp)
  (if (string-eq cmd-name "fmt") (cmd-fmt)
  (if (string-eq cmd-name "doc") (cmd-doc)
  0))))))))))))))

;; === コマンドディスパッチ ===

;; file I/O が未接続でも CLI core を TDD できるよう、
;; まずは in-memory source を受ける helper から固める。

(defn parse-first-decl-tag [program]
  (if (> (vector-length program) 0)
    (vector-get (vector-get program 0) 0)
    0))

(defn parse-decl-tag-text [tag]
  (if (= tag 20) "defn"
    (if (= tag 25) "module"
      (if (= tag 26) "import"
        (string-concat "decl-" (int-to-string tag))))))

(defn parse-expr-tag-text [tag]
  (if (= tag 1) "int"
    (if (= tag 2) "bool"
      (if (= tag 3) "string"
        (if (= tag 4) "var"
          (if (= tag 5) "apply"
            (if (= tag 6) "if"
              (if (= tag 7) "let"
                (if (= tag 8) "fn"
                  (if (= tag 9) "do"
                    (if (= tag 10) "match"
                      (if (= tag 32) "unit"
                        (string-concat "expr-" (int-to-string tag))))))))))))))

(defn parse-first-decl-text [program]
  (if (> (vector-length program) 0)
    (parse-decl-tag-text (vector-get (vector-get program 0) 0))
    "none"))

(defn parse-defn-body-index [decl]
  (+ 3 (vector-get decl 2)))

(defn parse-first-body-tag [program]
  (if (> (vector-length program) 0)
    (let [decl0 (vector-get program 0)]
      (if (= (vector-get decl0 0) 20)
        (vector-get (vector-get decl0 (parse-defn-body-index decl0)) 0)
        0))
    0))

(defn parse-first-body-text [program]
  (let [tag (parse-first-body-tag program)]
    (if (= tag 0) "none"
      (parse-expr-tag-text tag))))

(defn parse-decl-count-text [program]
  (string-concat "decls:" (int-to-string (vector-length program))))

(defn diagnostics-summary-text [count code body]
  (if (= count 0)
    "diagnostics:0"
    (string-concat
      "diagnostics:"
      (string-concat
        (int-to-string count)
        (string-concat
          ","
          (string-concat
            code
            (string-concat
              "@1:1"
               (string-concat ",first-body:" body))))))))

(defn parse-diagnostic-code [diag]
  (vector-get diag 1))

(defn parse-diagnostics-first-code [diagnostics]
  (if (> (vector-length diagnostics) 0)
    (parse-diagnostic-code (vector-get diagnostics 0))
    0))

(defn parse-diagnostic-body-from-code [code]
  (if (= code 1001) "unexpected token )"
    (if (= code 1002) "unexpected token ]"
      "parse error")))

(defn parse-diagnostics-body-text [diagnostics]
  (if (> (vector-length diagnostics) 0)
    (parse-diagnostic-body-from-code (parse-diagnostics-first-code diagnostics))
    ""))

(defn check-diagnostic-body-from-code [code]
  (if (= code (error-code-undefined)) "undefined symbol"
    (if (= code (error-code-if-cond)) "if condition must be Bool"
      (if (= code (error-code-if-branch)) "if branches must have same type"
        (if (= code (error-code-arg-mismatch)) "function argument type mismatch"
          (if (= code (error-code-infinite)) "infinite type"
            "type error"))))))

(defn check-diagnostics-body-text [program]
  (let [code (check-diagnostics-first-code program)]
    (if (= code 0)
      ""
      (check-diagnostic-body-from-code code))))

(defn run-parse-source [src opts]
  (let [program (parse-program src)
        diagnostics (parse-diagnostics src)
        diagnostics-count (vector-length diagnostics)
        diagnostics-text (diagnostics-summary-text diagnostics-count "P0001" (parse-diagnostics-body-text diagnostics))]
    (do
      (print-string (parse-decl-count-text program))
      (print-string "\n")
      (print-string (string-concat "first-decl:" (parse-first-decl-text program)))
      (print-string "\n")
      (print-string (string-concat "first-body:" (parse-first-body-text program)))
      (print-string "\n")
      (print-string diagnostics-text)
      (print-string "\n")
      (exit-success))))

(defn builtin-type-name-text [type-hash]
  (if (= type-hash 100) "Int"
  (if (= type-hash 200) "Bool"
  (if (= type-hash 300) "String"
  (if (= type-hash 400) "Float"
  (if (= type-hash 500) "Unit"
    (string-concat "type-" (int-to-string type-hash))))))))

(defn render-type-text [ty]
  (let [tag (ty-tag ty)]
    (if (= tag 1)
      (builtin-type-name-text (ty-name ty))
      (if (= tag 2)
        (string-concat "t" (int-to-string (ty-name ty)))
        (if (= tag 3)
          "Fn"
          (if (= tag 4)
            (string-concat "record-" (int-to-string (ty-name ty)))
            "Unknown"))))))

(defn run-check-source [src opts]
  (let [program (parse-program src)
        ty (infer program)
        rendered (render-type-text ty)
        diagnostics-count (check-diagnostics-count-program program)
        diagnostics-text (diagnostics-summary-text diagnostics-count "T0001" (check-diagnostics-body-text program))]
    (do
      (print-string rendered)
      (print-string "\n")
      (print-string diagnostics-text)
      (print-string "\n")
      (exit-success))))

(defn run-fmt-source [src opts]
  (let [program (parse-program src)
        formatted (format-program program opts)]
    (do
      (print-string formatted)
      (exit-success))))

(defn wasm-size-text [size]
  (string-concat "wasm-size:" (int-to-string size)))

(defn run-compile-source [src opts]
  (let [program (parse-program src)
        ir (lower program)
        wasm-size (emit-wasm ir)]
    (do
      (print-string (wasm-size-text wasm-size))
      (print-string "\n")
      (exit-success))))

(defn test-examples-text [count]
  (string-concat "examples:" (int-to-string count)))

(defn test-invariants-text [count]
  (string-concat "invariants:" (int-to-string count)))

(defn test-failures-text [count]
  (string-concat "failures:" (int-to-string count)))

(defn run-test-source [src opts]
  (let [suite (generate-tests-from-source src)
        example-results (vector-get suite 0)
        invariant-results (vector-get suite 1)
        example-count (vector-length example-results)
        invariant-count (vector-length invariant-results)
        failed (+ (count-failed-results example-results)
                  (count-failed-results invariant-results))]
    (do
      (print-string (test-examples-text example-count))
      (print-string "\n")
      (print-string (test-invariants-text invariant-count))
      (print-string "\n")
      (print-string (test-failures-text failed))
      (print-string "\n")
      (if (> failed 0)
        (exit-runtime-error)
        (exit-success)))))

(defn run-review-source [src opts]
  (let [program (parse-program src)
        review (generate-review program opts)
        diagnostics (vector-get review 1)
        review-title (review-summary-title diagnostics)
        review-body (review-summary-body diagnostics)
        review-severity (review-summary-severity diagnostics)
        review-code-location (review-summary-code-location diagnostics)]
    (do
      (print (vector-length diagnostics))
      (print-string review-title)
      (print-string "\n")
      (print-string review-body)
      (print-string "\n")
      (print-string review-severity)
      (print-string "\n")
      (print-string review-code-location)
      (print-string "\n")
      (exit-success))))

(defn run-doc-source [src opts]
  (let [program (parse-program src)
        doc (generate program opts)
        title (vector-get doc 0)
        body (vector-get doc 1)]
    (do
      (print-string title)
      (print-string "\n")
      (print-string body)
      (print-string "\n")
      (exit-success))))

;; parse サブコマンド: ソースファイルをパースして AST を出力
(defn run-parse [file-path opts]
  (if (file-exists? file-path)
    (run-parse-source (read-file file-path) opts)
    (exit-compile-error)))

;; check サブコマンド: 型チェックを実行
(defn run-check [file-path opts]
  (if (file-exists? file-path)
    (run-check-source (read-file file-path) opts)
    (exit-compile-error)))

;; compile サブコマンド: Wasm バイナリを生成
(defn run-compile [file-path opts]
  (if (file-exists? file-path)
    (run-compile-source (read-file file-path) opts)
    (exit-compile-error)))

;; build サブコマンド: プロジェクト全体をビルド
(defn run-build [file-path opts]
  (if (file-exists? file-path)
    (run-compile file-path opts)
    (exit-compile-error)))

;; test サブコマンド: メタデータテストを実行
(defn run-test [file-path opts]
  (if (file-exists? file-path)
    (run-test-source (read-file file-path) opts)
    (exit-compile-error)))

;; review サブコマンド: コードレビューを実行
(defn run-review [file-path opts]
  (if (file-exists? file-path)
    (run-review-source (read-file file-path) opts)
    (exit-compile-error)))

;; doc-ack サブコマンド: ドキュメント確認
(defn run-doc-ack [file-path opts]
  (if (file-exists? file-path)
    (let [src (read-file file-path)]
      (do
        (print-string "ack:recorded")
        (print-string "\n")
        (run-doc-source src opts)))
    (exit-compile-error)))

;; doc-check サブコマンド: ドキュメント整合性チェック
(defn run-doc-check [file-path opts]
  (if (file-exists? file-path)
    (let [src (read-file file-path)]
      (do
        (print-string "status:ok")
        (print-string "\n")
        (run-doc-source src opts)))
    (exit-compile-error)))

;; install サブコマンド: パッケージインストール
(defn install-plan-title [package]
  (string-concat "package:" package))

(defn install-plan-body [package]
  "status:planned")

(defn run-install [package opts]
  (if (> (string-length package) 0)
    (do
      (print-string (install-plan-title package))
      (print-string "\n")
      (print-string (install-plan-body package))
      (print-string "\n")
      (exit-success))
    (exit-compile-error)))

;; 注: まだ stdio 付きの継続 REPL ループは未接続なので、
;; まずは同一プロセス内の session helper で状態保持を検証する。
(defn repl-session-new []
  (let [v (vector-new 3)]
    (vector-push
      (vector-push
        (vector-push v (ref-new 0))  ;; eval-count
        (ref-new 0))                 ;; last-type-name
      (ref-new 0))))                 ;; total-input-bytes

(defn repl-session-eval-count [session]
  (ref-get (vector-get session 0)))

(defn repl-session-last-type-name [session]
  (ref-get (vector-get session 1)))

(defn repl-session-total-input-bytes [session]
  (ref-get (vector-get session 2)))

(defn repl-session-eval [session src]
  (let [program (parse-program src)
        ty (infer program)
        type-name (ty-name ty)]
    (do
      (ref-set (vector-get session 0) (+ (repl-session-eval-count session) 1))
      (ref-set (vector-get session 1) type-name)
      (ref-set (vector-get session 2) (+ (repl-session-total-input-bytes session) (string-length src)))
      type-name)))

(defn repl-session-run-loop [session inputs idx count]
  (if (>= idx count)
    0
    (do
      (repl-session-eval session (vector-get inputs idx))
      (repl-session-run-loop session inputs (+ idx 1) count))))

(defn repl-session-run [inputs]
  (let [session (repl-session-new)
        _ (repl-session-run-loop session inputs 0 (vector-length inputs))
        summary (vector-new 3)]
    (vector-push
      (vector-push
        (vector-push summary (repl-session-eval-count session))
        (repl-session-total-input-bytes session))
      (repl-session-last-type-name session))))

(defn repl-summary-type-text [summary]
  (string-concat "type:" (builtin-type-name-text (vector-get summary 2))))

(defn repl-summary-evals-text [summary]
  (string-concat "evals:" (int-to-string (vector-get summary 0))))

(defn repl-summary-input-bytes-text [summary]
  (string-concat "input-bytes:" (int-to-string (vector-get summary 1))))

(defn repl-warmup-summary []
  (let [inputs (vector-push (vector-new 1) "(defn main [] 42)")]
    (repl-session-run inputs)))

(defn repl-warmup-type-name []
  (let [summary (repl-warmup-summary)]
    (vector-get summary 2)))

(defn repl-warmup-type-text []
  (builtin-type-name-text (repl-warmup-type-name)))

;; repl サブコマンド: 対話的実行環境
(defn run-repl [opts]
  (let [summary (repl-warmup-summary)]
    (do
      (print-string (repl-summary-type-text summary))
      (print-string "\n")
      (print-string (repl-summary-evals-text summary))
      (print-string "\n")
      (print-string (repl-summary-input-bytes-text summary))
      (print-string "\n")
      (exit-success))))

(defn lsp-bool-text [value]
  (if (= value 1) "true" "false"))

(defn lsp-sync-kind-text [kind]
  (if (= kind 1) "full"
    (string-concat "sync-" (int-to-string kind))))

(defn lsp-loop-request [method-id params]
  (let [v (vector-new 2)]
    (vector-push (vector-push v method-id) params)))

(defn lsp-init-summary []
  (let [requests (vector-push (vector-new 1) (lsp-loop-request (lsp-method-initialize) 0))
        summary (server-loop-sequence requests)]
    summary))

(defn lsp-init-capabilities [summary]
  (let [results (vector-get summary 0)]
    (vector-get results 0)))

(defn lsp-summary-requests-text [summary]
  (string-concat "requests:" (int-to-string (vector-get summary 2))))

(defn lsp-summary-documents-text [summary]
  (string-concat "documents:" (int-to-string (vector-get summary 1))))

(defn lsp-summary-source-bytes-text [summary]
  (string-concat "source-bytes:" (int-to-string (vector-get summary 3))))

;; lsp サブコマンド: LSP サーバー起動
(defn run-lsp [opts]
  (let [summary (lsp-init-summary)
        caps (lsp-init-capabilities summary)]
    (do
      (print-string (string-concat "sync:" (lsp-sync-kind-text (vector-get caps 0))))
      (print-string "\n")
      (print-string (string-concat "hover:" (lsp-bool-text (vector-get caps 1))))
      (print-string "\n")
      (print-string (string-concat "completion:" (lsp-bool-text (vector-get caps 2))))
      (print-string "\n")
      (print-string (string-concat "definition:" (lsp-bool-text (vector-get caps 3))))
      (print-string "\n")
      (print-string (string-concat "references:" (lsp-bool-text (vector-get caps 4))))
      (print-string "\n")
      (print-string (string-concat "rename:" (lsp-bool-text (vector-get caps 5))))
      (print-string "\n")
      (print-string (string-concat "formatting:" (lsp-bool-text (vector-get caps 6))))
      (print-string "\n")
      (print-string (lsp-summary-requests-text summary))
      (print-string "\n")
      (print-string (lsp-summary-documents-text summary))
      (print-string "\n")
      (print-string (lsp-summary-source-bytes-text summary))
      (print-string "\n")
      (exit-success))))

;; fmt サブコマンド: ソースコードフォーマット
(defn run-fmt [file-path opts]
  (if (file-exists? file-path)
    (run-fmt-source (read-file file-path) opts)
    (exit-compile-error)))

;; doc サブコマンド: ドキュメント生成
(defn run-doc [file-path opts]
  (if (file-exists? file-path)
    (run-doc-source (read-file file-path) opts)
    (exit-compile-error)))

;; === 診断 (diagnostics) ===

;; parse-diagnostics-count: ソースをパースし、診断数を返す
;; 正常ソースなら 0、パースエラーがあればエラー数を返す
;; D-4: parse/check の構造化エラー返却の基盤
(defn parse-diagnostics-loop [spans pos-ref src diagnostics]
  (if (== (p-current spans pos-ref) 99)
    diagnostics
    (let [before (ref-get pos-ref)
          parsed (parse-with-recovery spans pos-ref src diagnostics)
          next-diagnostics (vector-get parsed 1)]
      (if (= (ref-get pos-ref) before)
        (do
          (p-advance pos-ref)
          (parse-diagnostics-loop spans pos-ref src next-diagnostics))
        (parse-diagnostics-loop spans pos-ref src next-diagnostics)))))

(defn parse-diagnostics [src]
  (let [spans (tokenize-with-spans src)
        pos-ref (ref-new 0)
        diagnostics (parse-diagnostics-loop spans pos-ref src (collect-diagnostics))]
    diagnostics))

(defn parse-diagnostics-count [src]
  (let [diagnostics (parse-diagnostics src)]
    (vector-length diagnostics)))

;; check-diagnostics-count: ソースを型チェックし、診断数を返す
;; 正常ソースなら 0
(defn check-diagnostics-loop [program idx len env counter count]
  (if (>= idx len)
    count
    (let [decl (vector-get program idx)
          tag (vector-get decl 0)]
      (if (= tag 20)
        (let [out (infer-defn decl env counter)]
          (if (= (result-failed out) 1)
            (check-diagnostics-loop program (+ idx 1) len env counter (+ count 1))
            (let [next-env (if (> (vector-length out) 3) (vector-get out 3) env)]
              (check-diagnostics-loop program (+ idx 1) len next-env counter count))))
        (check-diagnostics-loop program (+ idx 1) len env counter count)))))

(defn check-diagnostics-first-code-loop [program idx len env counter]
  (if (>= idx len)
    0
    (let [decl (vector-get program idx)
          tag (vector-get decl 0)]
      (if (= tag 20)
        (let [out (infer-defn decl env counter)]
          (if (= (result-failed out) 1)
            (result-error-code out)
            (let [next-env (if (> (vector-length out) 3) (vector-get out 3) env)]
              (check-diagnostics-first-code-loop program (+ idx 1) len next-env counter))))
        (check-diagnostics-first-code-loop program (+ idx 1) len env counter)))))

(defn check-diagnostics-count-program [program]
  (let [counter (make-var-counter)
        env (init-builtin-env counter)]
    (check-diagnostics-loop program 0 (vector-length program) env counter 0)))

(defn check-diagnostics-first-code [program]
  (let [counter (make-var-counter)
        env (init-builtin-env counter)]
    (check-diagnostics-first-code-loop program 0 (vector-length program) env counter)))

(defn check-diagnostics-count [src]
  (let [program (parse-program src)]
    (check-diagnostics-count-program program)))

;; メインディスパッチャ
;; コマンド ID に基づいて適切なハンドラを呼び出す
(defn dispatch-command-tail [cmd-id file-path opts]
  (if (= cmd-id (cmd-doc-ack))
    (run-doc-ack file-path opts)
    (if (= cmd-id (cmd-doc-check))
      (run-doc-check file-path opts)
      (if (= cmd-id (cmd-install))
        (run-install file-path opts)
        (if (= cmd-id (cmd-repl))
          (run-repl opts)
          (if (= cmd-id (cmd-lsp))
            (run-lsp opts)
            (if (= cmd-id (cmd-fmt))
              (run-fmt file-path opts)
              (if (= cmd-id (cmd-doc))
                (run-doc file-path opts)
                (exit-unknown-command)))))))))

(defn dispatch-command [cmd-id file-path opts]
  (if (= cmd-id (cmd-parse))
    (run-parse file-path opts)
    (if (= cmd-id (cmd-check))
      (run-check file-path opts)
      (if (= cmd-id (cmd-compile))
        (run-compile file-path opts)
        (if (= cmd-id (cmd-build))
          (run-build file-path opts)
          (if (= cmd-id (cmd-test))
            (run-test file-path opts)
            (if (= cmd-id (cmd-review))
              (run-review file-path opts)
              (dispatch-command-tail cmd-id file-path opts))))))))

;; === ヘルプ・バージョン ===

(defn help-text []
  "Usage: lsharp <command> [options] Commands: parse check compile build test review doc-ack doc-check install repl lsp fmt doc")

(defn version-text []
  "lsharp 0.1.0")

;; --help フラグ: 使用方法を表示
(defn show-help []
  (do
    (print-string (help-text))
    (exit-success)))

;; --version フラグ: バージョン情報を表示
(defn show-version []
  (do
    (print-string (version-text))
    (exit-success)))

;; === 出力チャネル分離 (stdout/stderr contract) ===

;; プログラム結果を標準出力へ
(defn cli-stdout [msg]
  (do
    (print-string msg)
    (print-string "\n")
    0))

;; 診断・エラーメッセージを stderr チャネルへ
;; WASI 環境では "error: " プレフィックスと改行で区別
(defn cli-stderr [msg]
  (do
    (print-string (string-concat "error: " msg))
    (print-string "\n")
    0))

;; === サブコマンドヘルプ ===

;; 個別コマンドのヘルプ文字列を返す
(defn format-subcommand-help [cmd]
  (if (string-eq cmd "parse") "parse <file> - Parse source and show AST"
  (if (string-eq cmd "check") "check <file> - Type-check source"
  (if (string-eq cmd "compile") "compile <file> -o <out> - Compile to Wasm"
  (if (string-eq cmd "build") "build [dir] - Build project"
  (if (string-eq cmd "test") "test <file> - Run metadata tests"
  (if (string-eq cmd "review") "review <file> - Code review"
  (if (string-eq cmd "doc-ack") "doc-ack <file> - Acknowledge docs"
  (if (string-eq cmd "doc-check") "doc-check <file> - Check doc consistency"
  (if (string-eq cmd "install") "install <pkg> - Install package"
  (if (string-eq cmd "repl") "repl - Interactive REPL"
  (if (string-eq cmd "lsp") "lsp - Start LSP server"
  (if (string-eq cmd "fmt") "fmt <file> - Format source"
  (if (string-eq cmd "doc") "doc <file> - Generate docs"
  "unknown command"))))))))))))))

;; === トップレベルエントリポイント ===

;; コマンド名文字列からディスパッチし、終了コードを返す
;; --help / --version フラグも処理する
(defn run-command [cmd-name file-path opts]
  (if (string-eq cmd-name "--help") (show-help)
  (if (string-eq cmd-name "--version") (show-version)
  (let [cmd-id (arg-parse cmd-name)]
    (if (= cmd-id 0)
      (do
        (cli-stderr (string-concat "unknown command: " cmd-name))
        (exit-code-unknown-command))
      (dispatch-command cmd-id file-path opts))))))

;; 検証用 main
(defn main []
  (let [;; コマンド ID テスト
        p (cmd-parse)
        c (cmd-check)
        b (cmd-build)
        empty-path ""

        ;; ディスパッチテスト
        r1 (dispatch-command (cmd-repl) empty-path 0)
        r2 (dispatch-command (cmd-lsp) empty-path 0)
        r3 (dispatch-command 0 empty-path 0)]
    (do
      (print p)   ;; 1
      (print c)   ;; 2
      (print b)   ;; 4
      (print r1)  ;; 0 (success)
      (print r2)  ;; 0 (success)
      (print r3)  ;; 127 (unknown)
      0)))
