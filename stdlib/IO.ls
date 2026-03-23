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
(defn read-file-or [path default]
  (if (file-exists? path)
    (read-file path)
    default))

;; エントリポイント (テスト用)
(defn main []
  (do
    (print (file-exists? "nonexistent.txt"))
    0))
