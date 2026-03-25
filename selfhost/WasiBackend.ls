(module WasiBackend)

;; WasiBackend.ls - L# セルフホスティング: WASI wiring
;;
;; WASI インポート定義と WASI ヘルパー関数。
;; fd_write, fd_read, clock_time_get 等の WASI API をラップ。

;; === WASI インポート定義 ===

;; WASI snapshot preview1 のモジュール名エンコーディング
;; "wasi_snapshot_preview1" = 21 バイト
(defn wasi-module-name-length [] 21)

;; WASI fd_write の型インデックス
;; (i32, i32, i32, i32) -> i32
(defn wasi-fd-write-type [] 0)

;; WASI fd_read の型インデックス
;; (i32, i32, i32, i32) -> i32
(defn wasi-fd-read-type [] 1)

;; WASI clock_time_get の型インデックス
;; (i32, i64, i32) -> i32
(defn wasi-clock-time-get-type [] 2)

;; === WASI メモリレイアウト ===

;; WASI 用メモリ: 最低 1 ページ (64KB)
(defn wasi-memory-pages [] 1)

;; iov バッファのベースオフセット
(defn wasi-iov-base [] 0)

;; 書き込みバッファのベースオフセット
(defn wasi-write-buffer-base [] 1024)

;; === WASI ヘルパー関数 ===

;; print: fd_write を使って stdout に出力
;; value を文字列に変換して stdout (fd=1) に書き込む
(defn wasi-print [value]
  ;; fd_write(fd=1, iovs_ptr, iovs_len=1, nwritten_ptr)
  ;; iov: [buf_ptr, buf_len]
  (let [fd 1
        iovs-ptr 0
        iovs-len 1
        nwritten-ptr 8]
    (do
      ;; 実際の WASI 呼び出しはランタイムが処理
      ;; ここでは引数構造の定義のみ
      (print value)
      0)))

;; read-file: fd_read を使ってファイルから読み込み
;; fd: ファイルディスクリプタ
;; buf-ptr: 読み込みバッファのポインタ
;; buf-len: バッファサイズ
(defn wasi-read-file [fd buf-ptr buf-len]
  ;; fd_read(fd, iovs_ptr, iovs_len=1, nread_ptr)
  (let [iovs-ptr 0
        iovs-len 1
        nread-ptr 8]
    ;; 読み込みバイト数を返す (暫定: 0)
    0))

;; write-file: fd_write を使ってファイルに書き込み
;; fd: ファイルディスクリプタ
;; buf-ptr: 書き込みデータのポインタ
;; buf-len: データサイズ
(defn wasi-write-file [fd buf-ptr buf-len]
  ;; fd_write(fd, iovs_ptr, iovs_len=1, nwritten_ptr)
  (let [iovs-ptr 0
        iovs-len 1
        nwritten-ptr 8]
    ;; 書き込みバイト数を返す (暫定: buf-len)
    buf-len))

;; clock-now: clock_time_get で現在時刻を取得
;; 戻り値: ナノ秒単位のタイムスタンプ (暫定: 0)
(defn wasi-clock-now []
  ;; clock_time_get(clock_id=0 (realtime), precision=0, timestamp_ptr)
  (let [clock-id 0
        precision 0
        timestamp-ptr 16]
    ;; タイムスタンプを返す (暫定: 0)
    0))

;; === WASI インポートセクション生成 ===

;; WASI fd_write インポートの生成
(defn wasi-imports []
  ;; [module-name, func-name, type-idx] の Vector
  (vector-push
    (vector-push
      (vector-push (vector-new 3) 0)   ;; module: wasi_snapshot_preview1
      0)                                ;; func: fd_write
    0))                                 ;; type index

;; WASI メモリ定義
(defn wasi-memory []
  ;; [initial-pages, max-pages] の Vector
  (vector-push
    (vector-push (vector-new 2) 1)      ;; initial: 1 page
    256))                               ;; max: 256 pages (16MB)

;; エントリポイント (テスト用)
(defn main []
  (do
    (wasi-print 42)
    (print (wasi-clock-now))            ;; 0
    0))
