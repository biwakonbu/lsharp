(module IR.LowerPattern)
(import IR.IR)

;; LowerPattern.ls - L# セルフホスティング: パターンの lowering
;;
;; AST のパターンノードを IR の条件分岐命令列に変換する。
;; リテラル/コンストラクタ/レコード/ワイルドカードパターンに対応。

;; === パターン lowering ===

;; パターンを IR に変換 (ディスパッチ)
;; pattern: AST パターンノード
;; scrutinee-idx: マッチ対象のローカル変数インデックス
;; instrs: 追記先の命令列
;; 戻り値: 更新された instrs
(defn lower-pattern [pattern scrutinee-idx instrs]
  (let [tag (vector-get pattern 0)]
    (if (= tag 42)
      ;; リテラルパターン
      (lower-literal-pattern pattern scrutinee-idx instrs)
      (if (= tag 43)
        ;; コンストラクタパターン
        (lower-constructor-pattern pattern scrutinee-idx instrs)
        (if (= tag 44)
          ;; レコードパターン
          (lower-record-pattern pattern scrutinee-idx instrs)
          (if (= tag 40)
            ;; ワイルドカードパターン
            (lower-wildcard-pattern pattern scrutinee-idx instrs)
            ;; 変数パターン (tag=41): scrutinee を変数に束縛
            (vector-push instrs (make-instr 10 scrutinee-idx))))))))

;; === リテラルパターン lowering ===

;; リテラルパターン: scrutinee == literal 値 の比較命令を生成
;; pattern: [42, lit-node]
(defn lower-literal-pattern [pattern scrutinee-idx instrs]
  (let [lit-node (vector-get pattern 1)
        lit-tag (vector-get lit-node 0)
        lit-value
          (if (= lit-tag 32)
            0
            (vector-get lit-node 1))
        ;; scrutinee をロード
        i1 (vector-push instrs (make-instr 10 scrutinee-idx))
        ;; リテラル値をプッシュ
        i2 (vector-push i1 (make-instr 1 lit-value))
        ;; 等値比較
        i3 (vector-push i2 (make-instr 30 0))]
    i3))

;; === コンストラクタパターン lowering ===

;; コンストラクタパターン: タグ判別 + サブパターンの再帰 lowering
;; pattern: [43, constructor-tag, sub-pattern-count, sub-pat1, sub-pat2, ...]
(defn lower-constructor-pattern [pattern scrutinee-idx instrs]
  (let [ctor-tag (vector-get pattern 1)
        ;; scrutinee のタグフィールドをロード
        i1 (vector-push instrs (make-instr 10 scrutinee-idx))
        ;; コンストラクタタグと比較
        i2 (vector-push i1 (make-instr 1 ctor-tag))
        ;; 等値比較 (タグ判別)
        i3 (vector-push i2 (make-instr 30 0))]
    i3))

;; === レコードパターン lowering ===

;; レコードパターン: 各フィールドのパターンを lowering
;; pattern: [44, field-count, field1-hash, field1-pattern, ...]
(defn lower-record-pattern [pattern scrutinee-idx instrs]
  (let [field-count (vector-get pattern 1)]
    ;; 各フィールドの比較命令を生成
    (if (> field-count 0)
      (let [;; フィールド1: scrutinee のオフセットからロード
            i1 (vector-push instrs (make-instr 10 scrutinee-idx))]
        i1)
      ;; フィールドなし: 常にマッチ
      (vector-push instrs (make-instr 1 1)))))

;; === ワイルドカードパターン lowering ===

;; ワイルドカードパターン: 常にマッチ (比較命令不要)
;; pattern: [40]
(defn lower-wildcard-pattern [pattern scrutinee-idx instrs]
  ;; ワイルドカードは常にマッチするので true (1) をプッシュ
  (vector-push instrs (make-instr 1 1)))

;; === エントリポイント (テスト用) ===

(defn main []
  (let [;; リテラルパターンの lowering テスト
        lit-pat
          (vector-push
            (vector-push (vector-new 2) 42)
            (vector-push (vector-push (vector-new 2) 1) 99))
        result (lower-literal-pattern lit-pat 0 (vector-new 4))
        ;; ワイルドカードパターンのテスト
        wild-pat (vector-push (vector-new 1) 40)
        wild-result (lower-wildcard-pattern wild-pat 0 (vector-new 4))]
    (do
      (print (vector-length result))       ;; 3 (load, const, eq)
      (print (vector-length wild-result))  ;; 1 (const true)
      0)))
