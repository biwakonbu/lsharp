(module Cli)
(import AST)
(import Compiler)
(import DocTools)
(import Formatter)
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

(defn parse-first-body-tag [program]
  (if (> (vector-length program) 0)
    (let [decl0 (vector-get program 0)]
      (if (> (vector-length decl0) 3)
        (vector-get (vector-get decl0 3) 0)
        0))
    0))

(defn run-parse-source [src opts]
  (let [program (parse-program src)]
    (do
      (print (vector-length program))
      (print (parse-first-decl-tag program))
      (print (parse-first-body-tag program))
      (exit-success))))

(defn run-check-source [src opts]
  (let [program (parse-program src)
        ty (infer program)]
    (do
      (print (ty-tag ty))
      (print (ty-name ty))
      (exit-success))))

(defn run-fmt-source [src opts]
  (let [program (parse-program src)
        formatted (format-program program opts)]
    (do
      (print formatted)
      (exit-success))))

(defn run-compile-source [src opts]
  (let [program (parse-program src)
        ir (lower program)
        wasm-size (emit-wasm ir)]
    (do
      (print wasm-size)
      (exit-success))))

(defn run-test-source [src opts]
  (let [program (parse-program src)
        suite (generate-tests program)]
    (do
      (print (vector-length (vector-get suite 0)))
      (print (vector-length (vector-get suite 1)))
      (exit-success))))

(defn review-unused-let-count [node]
  (if (= (vector-get node 0) 7)
    (let [name-hash (vector-get node 1)
          body (vector-get node 3)]
      (if (= (ast-contains-var body name-hash) 0) 1 0))
    0))

(defn review-empty-do-count [node]
  (if (= (vector-get node 0) 9)
    (if (= (vector-get node 1) 0) 1 0)
    0))

(defn review-program-count [program]
  (if (> (vector-length program) 0)
    (let [decl0 (vector-get program 0)]
      (if (> (vector-length decl0) 3)
        (let [body (vector-get decl0 3)]
          (+ (review-unused-let-count body)
             (review-empty-do-count body)))
        0))
    0))

(defn run-review-source [src opts]
  (let [program (parse-program src)
        review-count (review-program-count program)]
    (do
      (print review-count)
      (exit-success))))

(defn run-doc-source [src opts]
  (let [program (parse-program src)
        doc-size (doc-summary-size program opts)]
    (do
      (print doc-size)
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
    (let [doc-size (doc-file-summary-size file-path opts)]
      (do
        (print doc-size)
        (exit-success)))
    (exit-compile-error)))

;; doc-check サブコマンド: ドキュメント整合性チェック
(defn run-doc-check [file-path opts]
  (if (file-exists? file-path)
    (let [doc-size (doc-file-summary-size file-path opts)]
      (do
        (print doc-size)
        (exit-success)))
    (exit-compile-error)))

;; install サブコマンド: パッケージインストール
(defn run-install [package opts]
  (if (> (string-length package) 0)
    (do
      (print (string-length package))
      (exit-success))
    (exit-compile-error)))

(defn repl-warmup-type-name []
  (ty-name (infer (parse-program "(defn main [] 42)"))))

;; repl サブコマンド: 対話的実行環境
(defn run-repl [opts]
  (do
    (print (repl-warmup-type-name))
    (exit-success)))

(defn lsp-capability-count []
  4)

;; lsp サブコマンド: LSP サーバー起動
(defn run-lsp [opts]
  (do
    (print (lsp-capability-count))
    (exit-success)))

;; fmt サブコマンド: ソースコードフォーマット
(defn run-fmt [file-path opts]
  (if (file-exists? file-path)
    (run-fmt-source (read-file file-path) opts)
    (exit-compile-error)))

;; doc サブコマンド: ドキュメント生成
(defn run-doc [file-path opts]
  (if (file-exists? file-path)
    (let [doc-size (doc-file-summary-size file-path opts)]
      (do
        (print doc-size)
        (exit-success)))
    (exit-compile-error)))

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
