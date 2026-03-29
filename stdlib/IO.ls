;; IO.ls - L# 標準ライブラリ: I/O 操作
;;
;; ファイル入出力のラッパーを提供する。

;; === ファイル操作 ===

;; ファイルの内容を文字列として読み込む
;; (defn read-file [path] ...) -- ビルトイン

;; ファイルに文字列を書き込む (書き込みバイト数を返す)
;; (defn write-file [path content] ...) -- ビルトイン

;; ファイルが存在するか
;; (defn file-exists? [path] ...) -- ビルトイン

;; === ユーティリティ ===

;; ファイルの内容を読み込み、デフォルト値で返す (ファイルが存在しない場合)
(defn read-file-or
  [path default]
  :doc "ファイルが存在すれば内容を読み込み、存在しなければデフォルト値を返す。"
  :params [ (path "読み込み対象のパス") (default "ファイルが存在しない場合に返す文字列")]
  :returns "ファイル内容、または default"
  :example [ (read-file-or "missing.txt" "fallback")]
  (if (file-exists? path)
    (read-file path)
    default))

;; エントリポイント (テスト用)
(private
  (defn main []
    (do
      (print (file-exists? "nonexistent.txt"))
      0)))
