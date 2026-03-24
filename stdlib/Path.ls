;; Path.ls - L# 標準ライブラリ: パス操作ユーティリティ
;;
;; ファイルパスの操作を提供する。
;; 内部的に string-char-at / substring / string-length を使用。

;; === パス操作 ===

;; パスを結合する ("/tmp" "file.txt" -> "/tmp/file.txt")
(defn path-join [dir file]
  (string-concat (string-concat dir "/") file))

;; ファイルの拡張子を取得する ("file.txt" -> ".txt")
;; ドットが見つからない場合は空文字列を返す
(defn path-extension [path]
  (let [len (string-length path)
        dot-pos (path-find-last-dot path len)]
    (if (= dot-pos -1)
      ""
      (substring path dot-pos len))))

;; ベースネームを取得する ("/tmp/file.txt" -> "file.txt")
(defn path-basename [path]
  (let [len (string-length path)
        sep-pos (path-find-last-sep path len)]
    (if (= sep-pos -1)
      path
      (substring path (+ sep-pos 1) len))))

;; ディレクトリ名を取得する ("/tmp/file.txt" -> "/tmp")
(defn path-dirname [path]
  (let [len (string-length path)
        sep-pos (path-find-last-sep path len)]
    (if (= sep-pos -1)
      "."
      (substring path 0 sep-pos))))

;; === 内部ヘルパー ===

;; パス内の最後の '.' の位置を見つける (-1 = 見つからない)
(defn path-find-last-dot [path len]
  (let [result (ref-new -1)
        i (ref-new 0)]
    (do
      (if (< (ref-get i) len)
        (do
          (if (= (string-char-at path (ref-get i)) 46)
            (do (ref-set result (ref-get i)) 0)
            0)
          (ref-set i (+ (ref-get i) 1))
          (if (< (ref-get i) len)
            (do
              (if (= (string-char-at path (ref-get i)) 46)
                (do (ref-set result (ref-get i)) 0)
                0)
              (ref-set i (+ (ref-get i) 1))
              (if (< (ref-get i) len)
                (do
                  (if (= (string-char-at path (ref-get i)) 46)
                    (do (ref-set result (ref-get i)) 0)
                    0)
                  (ref-set i (+ (ref-get i) 1))
                  (if (< (ref-get i) len)
                    (do
                      (if (= (string-char-at path (ref-get i)) 46)
                        (do (ref-set result (ref-get i)) 0)
                        0)
                      (ref-set i (+ (ref-get i) 1))
                      (if (< (ref-get i) len)
                        (do
                          (if (= (string-char-at path (ref-get i)) 46)
                            (do (ref-set result (ref-get i)) 0)
                            0)
                          (ref-set i (+ (ref-get i) 1))
                          (if (< (ref-get i) len)
                            (do
                              (if (= (string-char-at path (ref-get i)) 46)
                                (do (ref-set result (ref-get i)) 0)
                                0)
                              (ref-set i (+ (ref-get i) 1))
                              (if (< (ref-get i) len)
                                (do
                                  (if (= (string-char-at path (ref-get i)) 46)
                                    (do (ref-set result (ref-get i)) 0)
                                    0)
                                  (ref-set i (+ (ref-get i) 1))
                                  (if (< (ref-get i) len)
                                    (do
                                      (if (= (string-char-at path (ref-get i)) 46)
                                        (do (ref-set result (ref-get i)) 0)
                                        0)
                                      0)
                                    0))
                                0))
                            0))
                        0))
                    0))
                0))
            0))
        0)
      (ref-get result))))

;; パス内の最後の '/' の位置を見つける (-1 = 見つからない)
(defn path-find-last-sep [path len]
  (let [result (ref-new -1)
        i (ref-new 0)]
    (do
      (if (< (ref-get i) len)
        (do
          (if (= (string-char-at path (ref-get i)) 47)
            (do (ref-set result (ref-get i)) 0)
            0)
          (ref-set i (+ (ref-get i) 1))
          (if (< (ref-get i) len)
            (do
              (if (= (string-char-at path (ref-get i)) 47)
                (do (ref-set result (ref-get i)) 0)
                0)
              (ref-set i (+ (ref-get i) 1))
              (if (< (ref-get i) len)
                (do
                  (if (= (string-char-at path (ref-get i)) 47)
                    (do (ref-set result (ref-get i)) 0)
                    0)
                  (ref-set i (+ (ref-get i) 1))
                  (if (< (ref-get i) len)
                    (do
                      (if (= (string-char-at path (ref-get i)) 47)
                        (do (ref-set result (ref-get i)) 0)
                        0)
                      (ref-set i (+ (ref-get i) 1))
                      (if (< (ref-get i) len)
                        (do
                          (if (= (string-char-at path (ref-get i)) 47)
                            (do (ref-set result (ref-get i)) 0)
                            0)
                          (ref-set i (+ (ref-get i) 1))
                          (if (< (ref-get i) len)
                            (do
                              (if (= (string-char-at path (ref-get i)) 47)
                                (do (ref-set result (ref-get i)) 0)
                                0)
                              (ref-set i (+ (ref-get i) 1))
                              (if (< (ref-get i) len)
                                (do
                                  (if (= (string-char-at path (ref-get i)) 47)
                                    (do (ref-set result (ref-get i)) 0)
                                    0)
                                  (ref-set i (+ (ref-get i) 1))
                                  (if (< (ref-get i) len)
                                    (do
                                      (if (= (string-char-at path (ref-get i)) 47)
                                        (do (ref-set result (ref-get i)) 0)
                                        0)
                                      0)
                                    0))
                                0))
                            0))
                        0))
                    0))
                0))
            0))
        0)
      (ref-get result))))

;; エントリポイント (テスト用)
(defn main []
  (do
    ;; path-join テスト
    (print (string-length (path-join "/tmp" "file.txt")))  ;; 14
    ;; path-extension テスト
    (print (string-length (path-extension "file.txt")))    ;; 4 (.txt)
    ;; path-basename テスト
    (print (string-length (path-basename "/tmp/file.txt"))) ;; 8 (file.txt)
    ;; path-dirname テスト
    (print (string-length (path-dirname "/tmp/file.txt")))  ;; 4 (/tmp)
    0))
