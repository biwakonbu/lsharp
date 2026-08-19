# ADR: native linear heap の回収機構

- Status: doc-RED / in-design (実測した事実のみ確定。方式は未決定)
- Date: 2026-08-19
- Scope: `NATIVE-HEAP-02` / `I-13`
- Related: [rust-free daily lane](decisions-dev-loop-rust-free-daily-lane.md)、
  `selfhost/src/Backend/Native/NativeCodegen.ls`、
  `scripts/ci/materialize-native-macos-aarch64-bundle.py`

## 問題

`I-13` は「115 KB の入力 (`App/Cli.ls`) を native stage0 に食わせると heap を使い切って
exit 139」を確定させ、その原因を「**消費量は生存データ量ではなく累積確保回数に比例する**」と
書いた。4 GiB → 8 GiB へ倍増しても拡大分をちょうど使い切って落ちる実測がこれを支える。

ただし `I-13` はこの命題を **materializer 側 (`calloc` 1 回 + bump、`free` なし)** からしか
導いていない。「では何がその累積確保を作っているのか」は書かれておらず、
回収機構を設計する前にそこを埋める必要がある。本 ADR はその埋め戻しである。

## lane を分けて実測した確保系 helper (2026-08-19)

**wasm lane の数字を native lane の根拠にしてはならない。** `I-13` 自身が
「x86 には limit があるのに aarch64 に無い」という lane parity 崩れの台帳なので、
parity を仮定した時点で同型の誤りになる。よって native 側の機械語を直接 decode した。

decode は cargo を使わず、`NativeCodegen.ls` のバイト列リテラルを python で並べて行った。

### 確保 1 回あたりのサイズと limit 比較の有無

| helper | lane | 1 回の確保 | capacity | limit 比較 |
|---|---|---|---|---|
| `map-new` | wasm (`application_map_allocation.rs:20-60`) | 65,552 | 4096 | 線形メモリの grow に委譲 |
| `map-new` | native x86-64 (`:9931`) | 65,296 | 4080 | **あり** (`cmp rdi, [r14+8]`) |
| `map-new` | native aarch64 (`:15171`) | **65,536 固定** | 4080 | **なし** |
| `vector-push` (grow) | wasm (`application_vector.rs:190`) | `16 + cap*8` | `max(cap*2, 4)` | 同上 |
| `vector-push` (grow) | native x86-64 (`:9711`) | `16 + cap*8` | `max(cap*2, 1)` | **あり** (`cmp rdi, [r14+8]`) |
| `vector-push` (grow) | native aarch64 (`:14671`) | `16 + cap*8` | `max(cap*2, 4)` | **なし** |
| `string-concat` | native x86-64 (`:9876`) | `align16(8 + len1 + len2)` | -- | **あり** (`cmp rcx, rdx`) |

aarch64 `vector-push` の該当箇所 (`0x24`〜`0x30`) は `add w4, w3, w3` → `cmp w4, #4` →
`movz w4, #4` で、**倍化して下限 4** である。x86 は `add r13d, r13d` → 0 なら 1。
aarch64 `map-new` は `movz x2, #1, lsl #16` → `add x22, x22, x2` で、
**入力によらず 65,536 を無条件に bump する**。frontier `x22` の前進と `x21` 加算だけで、
比較も分岐も無い。

### この decode が変えた 2 つの判断

1. **O(N²) 仮説は復活しない。** 「`concat-byte-vectors` が `vector-push` を 1 要素ずつ呼ぶので
   総確保が O(N²)」という筋を一度立てた。(115 KB)²/2 ≈ 6.9 GiB は観測 8 GiB に近く、
   数字としては魅力的だった。**しかし wasm・x86・aarch64 の 3 lane すべてで倍化であることを
   確認した以上、これは棄却である。** 数値の一致は偶然として扱う。
   再確保のたびに旧 buffer が放棄されるので総確保量は最終サイズの約 2 倍だが、依然 O(N)。

2. **`NATIVE-HEAP-01` のスコープは 1 helper では済まない。** `I-13` と `TODO.md` は
   bounds check の欠落を `emit-aarch64-selfhost-alloc-helper` (`:14513`) 単独の問題として
   書いているが、実際には `vector-push` (`:14671`) と `map-new` (`:15171`) にも limit 比較が無い。
   x86 は同じ 3 つすべてに持っている。**aarch64 の欠落は確保系 helper 全般に及ぶ。**

## 容疑者 (帰属は未確定)

8 GiB ÷ 65,536 = **131,072 回**の `map-new` で heap を使い切る。

| # | 容疑 | 支持する事実 | 反証 / 未確定 |
|---|---|---|---|
| A | `map-new` の 64 KiB 固定確保 | 確保が入力サイズに依存しない。resize path が無い。`map-insert` / `map-remove` は in-place で 0 確保 (`application_map_mutation.rs:22,143`) ので、増幅は **map-new の呼び出し回数だけ**で決まる | 静的な数え上げでは届かない (下記) |
| B | `vector-push` 再確保時の旧 buffer 放棄 | 倍化でも旧 buffer は回収されない。総確保 ≈ 最終サイズ×2 | O(N) なので単独では 8 GiB を作れない |
| C | `string-concat` の左畳み込み | 毎回 `len1+len2` の新規バッファを bump し、入力 2 本を放棄する。**growth policy と無関係に構造的 O(N²)** | 使用分布がパイプライン外に偏る。`selfhost/src` 全体で 723 箇所だが `Parser.ls` 1 / `Lexer.ls` 0 / `TypeInferCore.ls` 0 / `NativeCodegen.ls` 17 で、上位は LSP (`LspServerCore.ls` 84) / CLI (`Cli.ls` 78) / doc 生成 |

**静的数え上げは容疑 A を説明しきれない。** `selfhost/src` の `(defn ` は 6,656 個、
`typeinfer-defn-type-param-env` (`TypeInferFunctions.ls:311-315`) は trivial 分岐でも
`map-new` を 1 回呼ぶ。全 defn 分でも 6,656 × 64 KiB ≈ 416 MiB にしかならない。
crash 対象の `App/Cli.ls` 単体ならさらに 1 桁小さい。
131,072 回に届くには**式ごと・単一化ごとといった内側のループで呼ばれている**必要があり、
それは静的には決まらない。**帰属には動的計測が要る。**

## 選択肢

| 案 | 内容 | 現時点の評価 |
|---|---|---|
| A | mark-sweep GC | root set は列挙可能 (`x27`..`x28` の連続配列 8 MiB)、object は自己記述 (tag@0 / capacity@4 / length@8、payload は 16 から)、pointer は bit 63 タグ付きで整数と区別できる。**mark は原理的に可能**。ただし GC は `NativeCodegen.ls` 自身が emit する機械語になるうえ、frontier が x86 は heap 先頭 16 bytes、aarch64 は `x22` レジスタと**非対称**で、共通実装を書けない |
| B | phase 境界での arena reset | 実装は最も軽い。`I-13` の「累積確保回数に比例」という性質に直接効く。ただし phase を跨いで生存するデータの識別が要る |
| C | 確保量の削減 (`map-new` の固定 64 KiB を縮める / 遅延確保) | **回収機構を入れずに問題が消える可能性がある。** 容疑 A が主因なら、これが最小の変更。案 A の実装重量を考えると先に検討する価値がある |
| D | heap 拡大 | **却下。** 4 GiB → 8 GiB の実測で拡大分をちょうど使い切って落ちた (`I-13`)。env knob も緩和策にならないことが同 issue に記録済み |

**A / B / C は現時点で決めない。** 帰属が未確定のまま方式を選ぶと、
効かない機構を実装する危険がある (案 D が実測で潰れたのと同じ失敗の型)。

## 受入条件

1. **確保の帰属を動的に計測する。** どの call site が何回 `map-new` / `vector-push` grow /
   `string-concat` を呼ぶかを、`App/Cli.ls` を入力とする実行で数える。
   **計測は wasm lane で行ってよい** — 呼び出し回数は program logic の性質であり、
   lane によらない。native stage 再生成より安い経路である。
   (確保**サイズ**は lane で違うので、そちらは上表を使う。)
2. 計測結果を本 ADR の Evidence 節へ戻し、容疑 A / B / C の寄与率を確定させる。
3. その上で案を選ぶ。**選ばなかった案とその理由を本 ADR に残す。**

## 含めない範囲

- `NATIVE-HEAP-01` (bounds check)。ただし上記のとおり対象 helper が 3 つに増える事実は
  `I-13` / `TODO.md` へ反映する。
- x86 / aarch64 の frontier 表現の統一。案 A を選んだ場合にのみ必要になる。

## Evidence

未取得 (受入条件 1 が先)。
