(module Cli)
(import AST)
(import Compiler)
(import Formatter)
(import Linter)

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
(defn arg-parse [cmd-hash]
  (if (= cmd-hash 1) (cmd-parse)
  (if (= cmd-hash 2) (cmd-check)
  (if (= cmd-hash 3) (cmd-compile)
  (if (= cmd-hash 4) (cmd-build)
  (if (= cmd-hash 5) (cmd-test)
  (if (= cmd-hash 6) (cmd-review)
  (if (= cmd-hash 7) (cmd-doc-ack)
  (if (= cmd-hash 8) (cmd-doc-check)
  (if (= cmd-hash 9) (cmd-install)
  (if (= cmd-hash 10) (cmd-repl)
  (if (= cmd-hash 11) (cmd-lsp)
  (if (= cmd-hash 12) (cmd-fmt)
  (if (= cmd-hash 13) (cmd-doc)
  0))))))))))))))

;; === コマンドディスパッチ ===

;; parse サブコマンド: ソースファイルをパースして AST を出力
(defn run-parse [file-path opts]
  (exit-success))

;; check サブコマンド: 型チェックを実行
(defn run-check [file-path opts]
  (exit-success))

;; compile サブコマンド: Wasm バイナリを生成
(defn run-compile [file-path opts]
  (exit-success))

;; build サブコマンド: プロジェクト全体をビルド
(defn run-build [dir opts]
  (exit-success))

;; test サブコマンド: メタデータテストを実行
(defn run-test [file-path opts]
  (exit-success))

;; review サブコマンド: コードレビューを実行
(defn run-review [file-path opts]
  (exit-success))

;; doc-ack サブコマンド: ドキュメント確認
(defn run-doc-ack [file-path opts]
  (exit-success))

;; doc-check サブコマンド: ドキュメント整合性チェック
(defn run-doc-check [file-path opts]
  (exit-success))

;; install サブコマンド: パッケージインストール
(defn run-install [package opts]
  (exit-success))

;; repl サブコマンド: 対話的実行環境
(defn run-repl [opts]
  (exit-success))

;; lsp サブコマンド: LSP サーバー起動
(defn run-lsp [opts]
  (exit-success))

;; fmt サブコマンド: ソースコードフォーマット
(defn run-fmt [file-path opts]
  (exit-success))

;; doc サブコマンド: ドキュメント生成
(defn run-doc [file-path opts]
  (exit-success))

;; メインディスパッチャ
;; コマンド ID に基づいて適切なハンドラを呼び出す
(defn dispatch-command [cmd-id file-path opts]
  (if (= cmd-id (cmd-parse)) (run-parse file-path opts)
  (if (= cmd-id (cmd-check)) (run-check file-path opts)
  (if (= cmd-id (cmd-compile)) (run-compile file-path opts)
  (if (= cmd-id (cmd-build)) (run-build file-path opts)
  (if (= cmd-id (cmd-test)) (run-test file-path opts)
  (if (= cmd-id (cmd-review)) (run-review file-path opts)
  (if (= cmd-id (cmd-doc-ack)) (run-doc-ack file-path opts)
  (if (= cmd-id (cmd-doc-check)) (run-doc-check file-path opts)
  (if (= cmd-id (cmd-install)) (run-install file-path opts)
  (if (= cmd-id (cmd-repl)) (run-repl opts)
  (if (= cmd-id (cmd-lsp)) (run-lsp opts)
  (if (= cmd-id (cmd-fmt)) (run-fmt file-path opts)
  (if (= cmd-id (cmd-doc)) (run-doc file-path opts)
  (exit-unknown-command)))))))))))))))

;; === ヘルプ・バージョン ===

;; --help フラグ: 使用方法を表示
(defn show-help []
  0)

;; --version フラグ: バージョン情報を表示
(defn show-version []
  0)

;; 検証用 main
(defn main []
  (let [;; コマンド ID テスト
        p (cmd-parse)
        c (cmd-check)
        b (cmd-build)

        ;; ディスパッチテスト
        r1 (dispatch-command (cmd-parse) 0 0)
        r2 (dispatch-command (cmd-lsp) 0 0)
        r3 (dispatch-command 0 0 0)]
    (do
      (print p)   ;; 1
      (print c)   ;; 2
      (print b)   ;; 4
      (print r1)  ;; 0 (success)
      (print r2)  ;; 0 (success)
      (print r3)  ;; 127 (unknown)
      0)))
