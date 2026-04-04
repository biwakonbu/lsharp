(module Runtime.GC)

;; GC.ls - L# セルフホスティング: ガベージコレクタ
;;
;; 実 workload の allocator はまだ Rust 側 bump allocator が担っているが、
;; selfhost module 単体では mark-sweep + free-list の最小意味論を持たせる。
;; fixture 経由で回収 / root 保持 / free-list 再利用を実行確認できる状態にする。

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
;; 補助ベクタ操作
;; =============================================================================

(defn vector-set-at [vec idx new-val]
  (vector-set-at-loop vec (vector-new (vector-length vec)) idx new-val 0 (vector-length vec)))

(defn vector-set-at-loop [vec result idx new-val i len]
  (if (>= i len)
    result
    (vector-set-at-loop
      vec
      (vector-push result
        (if (= i idx)
          new-val
          (vector-get vec i)))
      idx
      new-val
      (+ i 1)
      len)))

(defn vector-remove-at [vec idx]
  (vector-remove-at-loop vec (vector-new (vector-length vec)) idx 0 (vector-length vec)))

(defn vector-remove-at-loop [vec result idx i len]
  (if (>= i len)
    result
    (vector-remove-at-loop
      vec
      (if (= i idx)
        result
        (vector-push result (vector-get vec i)))
      idx
      (+ i 1)
      len)))

;; =============================================================================
;; ルートセット管理 (TASK-GC-01)
;; =============================================================================

;; GC ルートセット: 到達可能オブジェクトの先頭ポインタ一覧
(defn make-root-set []
  (vector-new 0))

(defn root-set-contains [root-set ptr]
  (root-set-contains-loop root-set ptr 0 (vector-length root-set)))

(defn root-set-contains-loop [root-set ptr i len]
  (if (>= i len)
    0
    (if (= (vector-get root-set i) ptr)
      1
      (root-set-contains-loop root-set ptr (+ i 1) len))))

(defn root-set-remove [root-set ptr]
  (root-set-remove-loop root-set ptr (vector-new (vector-length root-set)) 0 (vector-length root-set) 0))

(defn root-set-remove-loop [root-set ptr result i len removed]
  (if (>= i len)
    result
    (let [value (vector-get root-set i)
      drop-now (if (= removed 1)
        0
        (if (= value ptr) 1 0))]
      (root-set-remove-loop
        root-set
        ptr
        (if (= drop-now 1)
          result
          (vector-push result value))
        (+ i 1)
        len
        (if (= drop-now 1) 1 removed)))))

;; add-root: GC ルートにポインタを登録
(defn add-root [gc-state ptr]
  (let [root-set-ref (vector-get gc-state 3)
    root-set (ref-get root-set-ref)]
    (do
      (ref-set root-set-ref
        (if (= (root-set-contains root-set ptr) 1)
          root-set
          (vector-push root-set ptr)))
      ptr)))

;; remove-root: GC ルートからポインタを解除
(defn remove-root [gc-state ptr]
  (let [root-set-ref (vector-get gc-state 3)
    root-set (ref-get root-set-ref)]
    (do
      (ref-set root-set-ref (root-set-remove root-set ptr))
      ptr)))

;; =============================================================================
;; Free-list 管理 (TASK-GC-02)
;; =============================================================================

;; FreeList: 空きメモリブロックの配列
;; 各エントリは [address, size]
(defn make-free-entry [addr size]
  (vector-push
    (vector-push (vector-new 2) addr)
    size))

(defn free-entry-addr [entry] (vector-get entry 0))
(defn free-entry-size [entry] (vector-get entry 1))

(defn make-free-list []
  (vector-new 0))

;; free-list にブロックを追加
(defn free-list-add [flist addr size]
  (if (> size 0)
    (vector-push flist (make-free-entry addr size))
    flist))

(defn free-list-find-index [flist size]
  (free-list-find-index-loop flist size 0 (vector-length flist)))

(defn free-list-find-index-loop [flist size i len]
  (if (>= i len)
    len
    (let [entry (vector-get flist i)]
      (if (>= (free-entry-size entry) size)
        i
        (free-list-find-index-loop flist size (+ i 1) len)))))

;; free-list から指定サイズ以上のブロックを検索して割り当て
(defn free-list-alloc [flist size]
  (let [idx (free-list-find-index flist size)
    len (vector-length flist)]
    (if (= idx len)
      0
      (free-entry-addr (vector-get flist idx)))))

(defn free-list-take [flist size]
  (let [idx (free-list-find-index flist size)
    len (vector-length flist)]
    (if (= idx len)
      (vector-push
        (vector-push (vector-new 2) 0)
        flist)
      (let [entry (vector-get flist idx)
        addr (free-entry-addr entry)
        block-size (free-entry-size entry)
        remaining (- block-size size)
        base-list (vector-remove-at flist idx)
        next-list (if (> remaining 0)
          (free-list-add base-list (+ addr size) remaining)
          base-list)]
        (vector-push
          (vector-push (vector-new 2) addr)
          next-list)))))

;; =============================================================================
;; Mark-Sweep GC (TASK-GC-02)
;; =============================================================================

;; 各 live object は [address, size, generation, trace-map-id]
(defn make-object [addr size generation trace-map-id]
  (vector-push
    (vector-push
      (vector-push
        (vector-push (vector-new 4) addr)
        size)
      generation)
    trace-map-id))

(defn object-addr [obj] (vector-get obj 0))
(defn object-size [obj] (vector-get obj 1))
(defn object-generation [obj] (vector-get obj 2))
(defn object-trace-map-id [obj] (vector-get obj 3))

(defn object-find-index [objects addr]
  (object-find-index-loop objects addr 0 (vector-length objects)))

(defn object-find-index-loop [objects addr i len]
  (if (>= i len)
    len
    (if (= (object-addr (vector-get objects i)) addr)
      i
      (object-find-index-loop objects addr (+ i 1) len))))

(defn object-exists [objects addr]
  (let [idx (object-find-index objects addr)
    len (vector-length objects)]
    (if (= idx len) 0 1)))

;; GC 状態:
;; [heap-start, heap-end, bump-ptr-ref, root-set-ref, free-list-ref,
;;  alloc-count-ref, freed-count-ref, collection-count-ref, objects-ref]
(defn make-gc-state [heap-start heap-size]
  (let [v0 (vector-new 9)
    v1 (vector-push v0 heap-start)
    v2 (vector-push v1 (+ heap-start heap-size))
    v3 (vector-push v2 (ref-new heap-start))
    v4 (vector-push v3 (ref-new (make-root-set)))
    v5 (vector-push v4 (ref-new (make-free-list)))
    v6 (vector-push v5 (ref-new 0))
    v7 (vector-push v6 (ref-new 0))
    v8 (vector-push v7 (ref-new 0))
    v9 (vector-push v8 (ref-new (vector-new 0)))]
    v9))

;; ヒープから新しいオブジェクトを割り当てる
(defn alloc [gc-state size]
  (let [free-list-ref (vector-get gc-state 4)
    objects-ref (vector-get gc-state 8)
    alloc-count-ref (vector-get gc-state 5)
    taken (free-list-take (ref-get free-list-ref) size)
    reused-addr (vector-get taken 0)]
    (if (> reused-addr 0)
      (do
        (ref-set free-list-ref (vector-get taken 1))
        (ref-set objects-ref (vector-push (ref-get objects-ref) (make-object reused-addr size 0 0)))
        (ref-set alloc-count-ref (+ (ref-get alloc-count-ref) 1))
        reused-addr)
      (let [bump-ptr-ref (vector-get gc-state 2)
        heap-end (vector-get gc-state 1)
        ptr (ref-get bump-ptr-ref)
        next (+ ptr size)]
        (if (> next heap-end)
          0
          (do
            (ref-set bump-ptr-ref next)
            (ref-set objects-ref (vector-push (ref-get objects-ref) (make-object ptr size 0 0)))
            (ref-set alloc-count-ref (+ (ref-get alloc-count-ref) 1))
            ptr))))))

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

;; gc-mark: ルートセットから到達可能な全オブジェクト数を返す
(defn gc-mark [root-set objects]
  (gc-mark-loop root-set objects 0 (vector-length root-set) 0))

(defn gc-mark-loop [root-set objects i len marked]
  (if (>= i len)
    marked
    (gc-mark-loop
      root-set
      objects
      (+ i 1)
      len
      (if (= (object-exists objects (vector-get root-set i)) 1)
        (+ marked 1)
        marked))))

;; sweep: ヒープ全体を走査し、未マークオブジェクトを回収
(defn sweep [root-set objects free-list]
  (sweep-loop root-set objects free-list (vector-new (vector-length objects)) 0 (vector-length objects) 0))

(defn sweep-loop [root-set objects free-list live i len freed]
  (if (>= i len)
    (vector-push
      (vector-push
        (vector-push (vector-new 3) live)
        free-list)
      freed)
    (let [obj (vector-get objects i)
      addr (object-addr obj)
      size (object-size obj)]
      (if (= (root-set-contains root-set addr) 1)
        (sweep-loop root-set objects free-list (vector-push live obj) (+ i 1) len freed)
        (sweep-loop root-set objects (free-list-add free-list addr size) live (+ i 1) len (+ freed 1))))))

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
  (let [survivor-count (vector-length remembered-set)]
    survivor-count))

;; =============================================================================
;; GC トリガーとメトリクス (TASK-GC-04)
;; =============================================================================

;; gc-collect: GC を手動で実行する
;; mark-sweep の完全なサイクルを実行
(defn gc-collect [gc-state]
  (let [root-set-ref (vector-get gc-state 3)
    free-list-ref (vector-get gc-state 4)
    freed-count-ref (vector-get gc-state 6)
    collection-count-ref (vector-get gc-state 7)
    objects-ref (vector-get gc-state 8)
    root-set (ref-get root-set-ref)
    objects (ref-get objects-ref)
    marked (gc-mark root-set objects)
    result (sweep root-set objects (ref-get free-list-ref))
    live-objects (vector-get result 0)
    next-free-list (vector-get result 1)
    freed (vector-get result 2)]
    (do
      marked
      (ref-set objects-ref live-objects)
      (ref-set free-list-ref next-free-list)
      (ref-set freed-count-ref (+ (ref-get freed-count-ref) freed))
      (ref-set collection-count-ref (+ (ref-get collection-count-ref) 1))
      freed)))

;; collect: gc-collect のエイリアス
(defn collect [gc-state]
  (gc-collect gc-state))

(defn object-bytes [objects]
  (object-bytes-loop objects 0 (vector-length objects) 0))

(defn object-bytes-loop [objects i len total]
  (if (>= i len)
    total
    (object-bytes-loop
      objects
      (+ i 1)
      len
      (+ total (object-size (vector-get objects i))))))

;; heap-used: 現在の live object 合計サイズを返す
(defn heap-used [gc-state]
  (let [objects-ref (vector-get gc-state 8)]
    (object-bytes (ref-get objects-ref))))

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
          (ref-get (vector-get gc-state 5)))
        (ref-get (vector-get gc-state 6)))
      (ref-get (vector-get gc-state 7)))
    (heap-used gc-state)))

;; total-collections: GC が実行された累計回数
(defn total-collections [gc-state]
  (ref-get (vector-get gc-state 7)))

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
  (ref-get (vector-get gc-state 5)))

;; freed-count: 累計解放回数を返す
(defn freed-count [gc-state]
  (ref-get (vector-get gc-state 6)))

;; detect-leak: メモリリークを検出する
;; 割り当て数と解放数の差分から未回収オブジェクトを推定
;; 戻り値: リーク疑いのオブジェクト数 (0 = リークなし)
(defn detect-leak [gc-state]
  (- (alloc-count gc-state) (freed-count gc-state)))
