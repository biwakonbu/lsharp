# imp-01: WasmGC 完全移行 (v2-07 補遺)

> 対象 issue: [D-01](../../../../ISSUES.md#d-01) (i64 フォールバック)、[D-02](../../../../ISSUES.md#d-02) (GADT)、
> [D-03](../../../../ISSUES.md#d-03) (HKT)、[D-04](../../../../ISSUES.md#d-04) (Computation Expression)、
> [D-06](../../../../ISSUES.md#d-06) (動的ディスパッチ)、[D-09](../../../../ISSUES.md#d-09) (selfhost ADT 表現)
> ロードマップ: [improvement-roadmap.md](../improvement-roadmap.md) Phase B-1
>
> **正本**: [v2-designs/v2-07-wasmgc-optional-backend.md](../v2-designs/v2-07-wasmgc-optional-backend.md)。
> 本書は v2-07 の方針 (バックエンド構成・型マッピング・優先順位・切り替えフラグ) を変更せず、
> 「現行コードのどこを、どの順で、どう無回帰に置き換えるか」の補遺を与える。

## 概要

現行 codegen はレコード/ADT を i64 フォールバックで表現しており
(`crates/lsharp-wasm/src/emit.rs:199-211` の TODO 群)、その帰結として:

- examples/gadt.ls / hkt.ls / computation.ls は「GC struct 型は wasmtime で未サポートのため、型チェックのみ検証」のまま (D-02/D-03/D-04)
- トレイトの動的ディスパッチ (vtable) が実装基盤を持たない (D-06)
- selfhost コンパイラは ADT を整数タグ + Vector で表現し続けている (D-09)

v2-07 の WasmGC バックエンドを実装し、これら 6 issue を一括で解消の道筋に乗せる。

## 設計

### 1. 段階移行 (v2-07 の優先順位に準拠)

| Stage | 対象 | 解消 issue | 完了の証跡 |
|-------|------|-----------|-----------|
| 1 | Records / ADT → `struct` 型 (tagged struct) | D-01 (主要部), D-02 | examples/gadt.ls がスタブなしで実行される E2E |
| 2 | Strings → `array i8` (UTF-8) | D-01 (残部) | 文字列系 E2E が wasmgc backend で green |
| 3 | Closures → funcref + env struct | D-03, D-04 | examples/hkt.ls / computation.ls が実行される E2E |
| 4 | トレイト vtable (メソッドテーブル struct + funcref) | D-06 | 動的ディスパッチの最小 E2E |
| 5 | selfhost コンパイラの ADT 表現切替 | D-09 | bootstrap 固定点が wasmgc backend でも一致 |

各 Stage は独立に main へマージ可能な単位とし、`--backend=wasmgc` フラグの背後で開発する
(デフォルトはリニアメモリバックエンドのまま。v2-07 の切り替え方針どおり)。

### 2. wasmtime 側の前提確認

examples の注記「GC struct 型は wasmtime で未サポート」は記載当時の制約である。
Stage 1 着手前に、現在 workspace で使用している wasmtime バージョンの GC proposal
サポート状況 (`Config::wasm_gc(true)`) を確認し、不足する場合は wasmtime の更新を
先行タスクとする (Cargo.toml の wasmtime / wasmtime-wasi を更新し、既存 E2E の全件 green を確認)。

### 3. コード上の置き換え地点

- `crates/lsharp-wasm/src/emit.rs:199-211` -- 「TODO: WasmGC 本格実装時に削除」とマークされた
  スタック操作フォールバック。Stage 1 で `struct.new` / `struct.get` / `struct.set` 系へ置換し、
  TODO を削除する
- `crates/lsharp-ir` -- `IrType::Ref` の lowering が i64 へ落ちている経路を、backend 選択に応じて
  GC ref 型を保持する経路へ分岐させる (IR 自体は v2-07 のとおり共有レイヤーとして不変)
- GC ランタイム (mark-sweep、`crates/lsharp-wasm/src/wasi.rs`) は wasmgc backend では不要になる。
  リニアメモリバックエンドが残る間は両立させ、backend ごとにランタイム注入を切り替える

### 4. 無回帰戦略

- 既存 E2E は全件リニアメモリバックエンドで実行し続ける (デフォルト不変のため変更不要)
- wasmgc backend 用に同一シナリオの E2E をパラメタライズして追加し、stdout の byte 一致を
  両バックエンドで比較する差分テストを置く
- bootstrap 固定点 (stage chain) は Stage 5 まではリニアメモリバックエンドのまま触れない
- 「型チェックのみ」の examples 3 件は Stage 達成ごとに注記を外し、実行アサーション付き
  E2E に昇格させる (ISSUES.md の D-02/D-03/D-04 を resolved に遷移)

### 5. GADT / HKT の追加作業 (codegen 以外)

D-02 / D-03 は codegen だけでなく型推論側の検証も必要:

- GADT: パターンマッチ時の型絞り込みの網羅テストを `crates/lsharp-types/src/infer.rs` の
  テストへ追加 (実行前に型レベルの正しさを固定する)
- HKT: 型適用の未実装経路 (selfhost 側に「HKT の型適用を実装してください」系の診断が残る) を
  洗い出し、Stage 3 のスコープに含めるか判定する

## 影響範囲

- デフォルトバックエンドは変更しないため、既存ユーザーへの影響なし
- wasmtime バージョン更新が入る場合は全 E2E の回帰確認が必要
- Stage 5 (selfhost 切替) は bootstrap 固定点の再検証を伴う最大の変更点であり、単独で計画する

## ステータス

設計のみ (2026-06-12 起草)。v2-07 の「Phase 11 後に実装予定」を引き継ぎ、
着手時は TODO.md に Phase B-1 として Stage 単位の項目を作成する。
