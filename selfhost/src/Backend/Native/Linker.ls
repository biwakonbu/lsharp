(module Backend.Native.Linker)
(import Backend.Native.NativeTarget)

;; Linker.ls - L# セルフホスティング: リンカー呼び出し
;;
;; オブジェクトファイル群をリンクして実行可能バイナリを生成する。
;; response file (@file) 方式でリンカー引数を渡す。

;; === リンカーコマンド構築 ===

;; ターゲットに応じたリンカー種別を選択
(defn select-linker [target]
  (target-linker-flavor target))

;; representative build entry で使う native artifact 名を固定する。
;; 現状は tier1 target すべてで同じ canonical 名を使う。
(defn default-program-object-path [target]
  "program.o")

(defn default-runtime-object-path [target]
  "runtime.o")

(defn default-linker-response-path [target]
  "linker-response.txt")

(defn default-program-binary-path [target]
  "program.native")

;; canonical response file では tier1 target すべてで -o を使う。
(defn linker-output-flag [target]
  "-o")

;; リンカー引数をリスト形式で構築
;; objects: オブジェクトファイルパスの Vector
;; output: 出力ファイルパス (文字列ハッシュ)
;; target: ターゲット記述子
;; 戻り値: 引数の Vector
(defn append-linker-objects [args objects idx n]
  (if (>= idx n)
    (ref-get args)
    (do
      (ref-set args (vector-push (ref-get args) (vector-get objects idx)))
      (append-linker-objects args objects (+ idx 1) n))))

(defn build-linker-args [objects output target]
  (let [args (ref-new (vector-new 16))
    linker-kind (select-linker target)]
    (do
      ;; 出力ファイル指定
      (ref-set args (vector-push (ref-get args) 1)) ;; -o フラグ
      (ref-set args (vector-push (ref-get args) output))
      ;; オブジェクトファイルを追加
      (let [i (ref-new 0)
        n (vector-length objects)]
        (append-linker-objects args objects (ref-get i) n)))))

;; representative build entry で使う string ベースの response file 引数を構築する。
(defn build-linker-response-args [objects output target]
  (let [args (ref-new (vector-new 16))]
    (do
      (ref-set args (vector-push (ref-get args) (linker-output-flag target)))
      (ref-set args (vector-push (ref-get args) output))
      (append-linker-objects args objects 0 (vector-length objects)))))

;; === Response File 生成 ===

;; response file のコンテンツを生成
;; リンカー引数を改行区切りのバイト列として出力
;; args: 引数の Vector (各要素は整数値)
;; 戻り値: バイト列 (改行区切りの引数リスト)
(defn append-response-args [result args idx n]
  (if (>= idx n)
    (ref-get result)
    (do
      ;; 引数を追加 (簡易版: 整数値としてエンコード)
      (ref-set result (vector-push (ref-get result) (vector-get args idx)))
      ;; 改行 (0x0A)
      (ref-set result (vector-push (ref-get result) 10))
      (append-response-args result args (+ idx 1) n))))

(defn generate-response-file [args]
  (let [result (ref-new (vector-new 64))
    n (vector-length args)]
    (append-response-args result args 0 n)))

;; canonical response file のテキスト版を生成する。
(defn append-response-text-lines [result args idx n]
  (if (>= idx n)
    result
    (append-response-text-lines
      (string-concat result (string-concat (vector-get args idx) "\n"))
      args
      (+ idx 1)
      n)))

(defn generate-response-file-text [args]
  (append-response-text-lines "" args 0 (vector-length args)))

;; response file を書き出す (将来の実装: ファイル I/O)
;; 現在はバイト列を返すのみ
(defn write-response-file [path args]
  (generate-response-file args))

;; === リンカー呼び出し ===

;; リンクを実行
;; objects: オブジェクトファイルの Vector
;; output: 出力パス
;; target: ターゲット記述子
;; 戻り値: 0 (成功) / 1 (失敗)
(defn link-objects [objects output target]
  (let [args (build-linker-args objects output target)
    response (generate-response-file args)]
    ;; 将来: response file を書き出してリンカーを exec する
    ;; 現在は引数構築の検証のみ
    (vector-length response)))

;; リンカーを直接呼び出す (将来の実装: exec syscall)
;; linker-path: リンカーの実行パス
;; response-path: response file のパス
;; 戻り値: 終了コード (0 = 成功)
(defn invoke-linker [linker-path response-path]
  ;; 将来: exec(linker-path, ["@" ++ response-path]) を実行
  ;; 現在はスタブ: 常に 0 (成功) を返す
  0)

;; === エントリポイント (テスト用) ===

(defn main []
  (let [target (make-target 1) ;; x86_64-apple-darwin
    objects (vector-push (vector-push (vector-new 4) 100) 200)
    result (link-objects objects 42 target)
    linker (select-linker target)]
    (do
      (print linker) ;; 1 (ld64)
      (print result) ;; response file のバイト数
      0)))
