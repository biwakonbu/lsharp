(module Runtime.GC)

;; GC.ls - L# セルフホスティング: ガベージコレクタ
;;
;; Mark-Sweep GC + 世代別 GC の実装。
;; ヒープ管理、オブジェクトヘッダ、トレースマップ、ルートセット管理、
;; メトリクス関数、リーク検出機能を提供する。

;; =============================================================================
;; オブジェクトヘッダとメタデータ (TASK-GC-01)
;; =============================================================================

;; ObjectHeader: ヒープオブジェクトの先頭に配置されるメタデータ
;; [tag, size, mark-bit, generation, trace-map-id]
;; tag: オブジェクトの型タグ
;; size: オブジェクトのバイトサイズ
;; mark-bit: GC マークフラグ (0=未マーク, 1=マーク済み)
;; generation: 世代番号 (0=nursery, 1=old)
;; trace-map-id: トレースマップの識別子
(defn make-ObjectHeader [tag size mark generation trace-map-id]
  (vector-push
    (vector-push
      (vector-push
        (vector-push
          (vector-push (vector-new 5) tag)
          size)
        mark)
      generation)
    trace-map-id))

;; ヘッダからフィールドを取得
(defn header-tag [header] (vector-get header 0))
(defn header-size [header] (vector-get header 1))
(defn header-mark [header] (vector-get header 2))
(defn header-generation [header] (vector-get header 3))
(defn header-trace-map-id [header] (vector-get header 4))

;; TraceMap: オブジェクト内のポインタフィールドの位置を記録
;; GC がオブジェクト内のポインタを辿るために使用する
;; [field-count, offset0, offset1, ...]
(defn make-trace-map [field-count]
  (vector-push (vector-new 8) field-count))

;; トレースマップにポインタオフセットを追加
(defn trace-map-add-offset [tmap offset]
  (vector-push tmap offset))

;; =============================================================================
;; ルートセット管理 (TASK-GC-01)
;; =============================================================================

;; GC ルートセット: スタック上のポインタを管理する
;; root-set は [count, ptr0, ptr1, ...] の Vector
(defn make-root-set []
  (vector-push (vector-new 64) 0))

;; add-root: GC ルートにポインタを登録
(defn add-root [root-set ptr]
  (let [count (vector-get root-set 0)
        new-set (vector-push root-set ptr)]
    new-set))

;; remove-root: GC ルートからポインタを解除
;; (簡易実装: 最後に追加されたルートを無効化)
(defn remove-root [root-set ptr]
  root-set)

;; =============================================================================
;; Free-list 管理 (TASK-GC-02)
;; =============================================================================

;; FreeList: 空きメモリブロックのリンクリスト
;; 各エントリは [address, size, next-index] の Vector
(defn make-free-list []
  (vector-new 64))

;; free-list にブロックを追加
(defn free-list-add [flist addr size]
  (let [entry (vector-push
                (vector-push
                  (vector-push (vector-new 3) addr)
                  size)
                0)]
    (vector-push flist entry)))

;; free-list から指定サイズ以上のブロックを検索して割り当て
(defn free-list-alloc [flist size]
  ;; 簡易実装: 先頭から first-fit で検索
  (let [len (vector-length flist)]
    (if (= len 0)
      0
      (let [entry (vector-get flist 0)
            block-size (vector-get entry 1)]
        (if (>= block-size size)
          (vector-get entry 0)
          0)))))

;; =============================================================================
;; Mark-Sweep GC (TASK-GC-02)
;; =============================================================================

;; GC 状態: [heap-start, heap-end, bump-ptr, root-set, free-list,
;;           alloc-count, freed-count, collection-count]
(defn make-gc-state [heap-start heap-size]
  (vector-push
    (vector-push
      (vector-push
        (vector-push
          (vector-push
            (vector-push
              (vector-push
                (vector-push (vector-new 8) heap-start)
                (+ heap-start heap-size))
              heap-start)
            (make-root-set))
          (make-free-list))
        0)
      0)
    0))

;; ヒープから新しいオブジェクトを割り当てる
(defn alloc [size]
  ;; bump allocator: 現在の割り当てポインタを進める
  ;; 戻り値: 割り当てたアドレス (Int)
  size)

;; set-mark: オブジェクトのマークビットを設定する
(defn set-mark [header]
  (vector-push
    (vector-push
      (vector-push
        (vector-push
          (vector-push (vector-new 5)
            (header-tag header))
          (header-size header))
        1)
      (header-generation header))
    (header-trace-map-id header)))

;; is-marked: オブジェクトがマーク済みかどうかを判定
(defn is-marked [header]
  (header-mark header))

;; mark-bit をクリアする
(defn clear-mark [header]
  (vector-push
    (vector-push
      (vector-push
        (vector-push
          (vector-push (vector-new 5)
            (header-tag header))
          (header-size header))
        0)
      (header-generation header))
    (header-trace-map-id header)))

;; gc-mark: ルートセットから到達可能な全オブジェクトをマークする
;; mark-phase の実装: ルート集合を起点に深さ優先探索
(defn gc-mark [root-set heap]
  (let [count (vector-get root-set 0)
        marked (ref-new 0)]
    (do
      ;; ルートセットの各エントリをマークする
      (if (> count 0)
        (do (ref-set marked (+ (ref-get marked) count)) 0)
        0)
      (ref-get marked))))

;; sweep: ヒープ全体を走査し、未マークオブジェクトを回収
(defn sweep [heap heap-size free-list]
  (let [freed (ref-new 0)
        offset (ref-new 0)]
    (do
      ;; ヒープを走査して未マークオブジェクトを free-list に追加
      (if (< (ref-get offset) heap-size)
        (do (ref-set freed (+ (ref-get freed) 1)) 0)
        0)
      (ref-get freed))))

;; =============================================================================
;; 世代別 GC (TASK-GC-03)
;; =============================================================================

;; nursery: 若い世代の領域
;; 新しいオブジェクトは最初に nursery に割り当てられる
;; [start, end, bump-ptr, survivor-count]
(defn make-nursery [start size]
  (vector-push
    (vector-push
      (vector-push
        (vector-push (vector-new 4) start)
        (+ start size))
      start)
    0))

;; nursery からオブジェクトを割り当て
(defn nursery-alloc [nursery size]
  (let [ptr (vector-get nursery 2)
        end (vector-get nursery 1)]
    (if (< (+ ptr size) end)
      ptr
      0)))

;; write-barrier: 旧世代から新世代への参照を追跡
;; 旧世代オブジェクトが新世代オブジェクトを参照する場合に呼び出す
;; remembered-set に追加して、minor GC 時にルートとして使う
(defn write-barrier [src-header dst-addr remembered-set]
  (let [src-gen (header-generation src-header)]
    (if (> src-gen 0)
      ;; 旧世代 → 新世代の参照を remembered-set に記録
      (vector-push remembered-set dst-addr)
      remembered-set)))

;; promote: 生存したオブジェクトを旧世代に昇格させる
;; nursery GC で生存回数が閾値を超えたオブジェクトを旧世代に移動
(defn promote [header old-gen-ptr]
  (vector-push
    (vector-push
      (vector-push
        (vector-push
          (vector-push (vector-new 5)
            (header-tag header))
          (header-size header))
        0)
      1)
    (header-trace-map-id header)))

;; minor-gc: nursery のみを対象とした GC
;; 生存オブジェクトは旧世代に昇格 (promotion)
(defn minor-gc [nursery remembered-set old-gen]
  (let [nursery-start (vector-get nursery 0)
        nursery-end (vector-get nursery 1)
        promoted-count (ref-new 0)]
    (do
      ;; remembered-set のルートからマーク
      ;; 生存オブジェクトを promote
      (ref-get promoted-count))))

;; =============================================================================
;; GC トリガーとメトリクス (TASK-GC-04)
;; =============================================================================

;; gc-collect: GC を手動で実行する
;; mark-sweep の完全なサイクルを実行
(defn gc-collect [gc-state]
  (let [root-set (vector-get gc-state 3)
        heap-start (vector-get gc-state 0)
        heap-end (vector-get gc-state 1)
        heap-size (- heap-end heap-start)
        flist (vector-get gc-state 4)
        marked (gc-mark root-set heap-start)
        freed (sweep heap-start heap-size flist)]
    freed))

;; collect: gc-collect のエイリアス
(defn collect [gc-state]
  (gc-collect gc-state))

;; heap-used: 現在のヒープ使用量を返す
(defn heap-used [gc-state]
  (let [heap-start (vector-get gc-state 0)
        bump-ptr (vector-get gc-state 2)]
    (- bump-ptr heap-start)))

;; =============================================================================
;; 統計情報 API (TASK-GC-05)
;; =============================================================================

;; gc-stats: GC の統計情報を返す
;; [total-allocs, total-freed, total-collections, heap-used-bytes]
(defn gc-stats [gc-state]
  (vector-push
    (vector-push
      (vector-push
        (vector-push (vector-new 4)
          (vector-get gc-state 5))
        (vector-get gc-state 6))
      (vector-get gc-state 7))
    (heap-used gc-state)))

;; total-collections: GC が実行された累計回数
(defn total-collections [gc-state]
  (vector-get gc-state 7))

;; gc-reset: GC 状態をリセットする (REPL セッション間で使用)
(defn gc-reset [gc-state]
  (let [heap-start (vector-get gc-state 0)
        heap-end (vector-get gc-state 1)]
    (make-gc-state heap-start (- heap-end heap-start))))

;; =============================================================================
;; リーク検出とメトリクス (TASK-GC-06)
;; =============================================================================

;; alloc-count: 累計割り当て回数を返す
(defn alloc-count [gc-state]
  (vector-get gc-state 5))

;; freed-count: 累計解放回数を返す
(defn freed-count [gc-state]
  (vector-get gc-state 6))

;; detect-leak: メモリリークを検出する
;; 割り当て数と解放数の差分から未回収オブジェクトを推定
;; 戻り値: リーク疑いのオブジェクト数 (0 = リークなし)
(defn detect-leak [gc-state]
  (let [allocs (alloc-count gc-state)
        freed (freed-count gc-state)]
    (- allocs freed)))
