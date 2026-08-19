# ADR: native linear heap の回収機構

- Status: doc-RED / in-design (実測した事実のみ確定。方式は未決定)
- Date: 2026-08-19
- Scope: `NATIVE-HEAP-02` / `I-13`
- Related: [rust-free daily lane](decisions-dev-loop-rust-free-daily-lane.md)、
  `selfhost/src/Backend/Native/NativeCodegen.ls`、
  `scripts/ci/materialize-native-macos-aarch64-bundle.py`、
  `crates/lsharp-wasm/src/wasi/gc_collect_core.rs` / `gc_mark.rs` / `free_list.rs`、
  [`phase11-implementation-plan.md`](../development/planning/phase11-implementation-plan.md) S14-S16

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

**ただし「リテラルを出現順に並べる」だけでは誤る。** emitter は
`(concat-three-byte-vectors-rooted (byte-vector-2 ...) heap-base (byte-vector-3 ...))` のように
`let` 束縛を引数順で並べ替えるので、grep の出現順とバイト順が一致しない。実際に一度これで
`x86 map-new には limit 比較が無い` という逆の結論を出しかけた。また `read-stdin` /
`int-to-string` / `string-concat` の chunk 群は `(ref-new (vector-new N))` へ
`append-encoded-u32-rooted` を積む形式で、リテラル抽出では 1 word も拾えない。
**S 式を評価してバイト列を組み立てる**必要がある (`byte-vector-N` / `encode-u32-le` /
`concat-*-rooted` / `vector-new` / `ref-new` / `ref-get` / `append-encoded-u32-rooted` /
`emit-aarch64-bl` の 8 形だけで全 helper が評価できる)。
本 ADR の数字は `scripts/native_codegen_bytes.py --list` で再現できる
(手順は `AGENTS.md` の「native emitter のバイト列を cargo 無しで読む」)。

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
   書いているが、実際には確保系 helper 全般に及ぶ。次節で全列挙した。

### 確保系 helper の全列挙 (2026-08-19)

「3 helper」と一度書いたが、これは 3 つ調べて 3 つとも該当しただけの undercount だった。
S 式評価で **全 selfhost helper のバイト列を組み立て、heap frontier を進める命令を機械的に検出**
して数え直す。判定は aarch64 が `add x22, x22, xN`、x86 が `mov [r14], rN`。

| lane | frontier を進める helper | bump 箇所 | limit を参照する箇所 |
|---|---|---|---|
| x86-64 | 9 | 9 | **9 / 9** |
| aarch64 | 10 (うち生存 9) | 11 (うち生存 10) | **0 / 11** |

内訳。両 lane に共通するのは 8 つ (`vector-new` / `vector-push` / `ref-new` / `substring` /
`string-concat` / `map-new` / `read-file` / `read-stdin`) で、`read-file` だけ aarch64 が
2 箇所 bump する。x86 のみが `int-to-string`、aarch64 のみが `alloc` と
`string-concat-helper-chunk3` を持つ。

**ただし `string-concat-helper-chunk3` は呼び出し元 0 の死んだ実装である** (`I-25`)。
これを除くと**両 lane とも生きている確保系 helper は 9 つ**で、bump 箇所は x86 9 / aarch64 10
(aarch64 の `read-file` だけが 2 箇所) になる。`NATIVE-HEAP-01` が bounds check を入れる対象は
この生存 9 つであって、11 でも 10 でもない。数え上げの際は
`python3 -c` による未参照 defn 走査 (`I-25` 参照) と突き合わせること。

x86 の参照形は 2 通りある。`vector-push` / `int-to-string` は `cmp rdi, [r14+8]` の直接メモリ比較、
残る 7 つは `mov rcx, [r14+8]` でロードしてからレジスタ比較する。どちらも到達点は同じで、
超過時は `ja` で `xor eax, eax; ret` の失敗パスへ抜ける。

aarch64 側で検出された `cmp` は 3 つあるが、いずれも limit 比較ではない。
`vector-push` の `cmp w2, w3` は length vs capacity、`cmp w4, #4` は倍化後の下限クランプ。
`substring` の `cmp x1, x22` は **frontier との比較**で、タグ付きポインタか raw かの判別である。
残る 7 helper には `cmp` 自体が 1 つも無い。
**limit そのものを保持する場所が aarch64 lane に無い** — x86 が heap 先頭 16 bytes に
cursor/limit を置くのに対し、aarch64 は `x21` (base) と `x22` (frontier) しか持たない。
`NATIVE-HEAP-01` は「比較を足す」ではなく「**上限値をどこに置くか**」から始まる。

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

## 既存 collector の所在 (2026-08-19、上記 decode の後に判明)

**「native に GC が無いので設計する」という当初の構図は誤りだった。** 本 ADR の初版はそう書いたが、
`crates/lsharp-wasm/src/wasi/` を見ると **mark-sweep collector は wasm lane に実装済み**である。

- `gc_collect_core.rs` / `gc_mark.rs` / `free_list.rs` — mark / sweep / size-class free list
- size class は 7 段 (`GC_FREE_CLASS_LIMITS = [16, 32, 64, 128, 256, 512, 1024]`、`wasi.rs:96`)。
  **64 KiB の map は全段を外れて oversize class 行き**になる
- `TAGGED_POINTER_MASK = 1 << 63` (`wasi.rs:97`) — 私が aarch64 helper から decode した
  bit 63 タグと同一。object header (tag/capacity/length) も共通
- 観測用 export が既にある: `__lsharp_heap_ptr` / `__lsharp_alloc_count` /
  `__lsharp_gc_live_alloc_count` / `__lsharp_gc_freed_count` / `__lsharp_gc_collection_count` ほか

**そして、この collector は実行中に一度も走らない。**
`grep -rn 'gc_collect_idx' crates/lsharp-wasm/src/` を crate 全体に当てると `Call` は 2 箇所しかなく、
どちらも `compiler_world/code.rs` (`:131`, `:150`) である。内訳は

- `_start`: `main` を呼んで **return した直後**
- `__proc_exit_with_collect`: 引数が 0 (正常終了) のときだけ

つまり回収は**プログラムが終わった後**にしか起きず、実行中の heap 圧力を一切下げない。

**http handler world はさらに徹底していて、呼び出し元が 0 である。**
`http_handler_core.rs:450` は `emit_gc_collect_func` で collector を emit するが、
その関数 index を受ける変数は `:42` で `let _gc_collect_idx` と **underscore 付き**、
すなわち一度も使われない。リクエスト境界で回す等の運用は入っていない。

**第 3 の実体として `selfhost/src/Runtime/GC.ls` (484 行) がある。** mark / sweep / free-list /
世代別の骨格が L# で書かれているが、**実 workload からは呼ばれない**。
`(gc-collect ...)` の呼び出しは同ファイル `:423` の alias 定義 1 箇所だけで、
`Runtime.GC` を `import` しているのは e2e fixture (`selfhost_gc_runtime_bootstrap.rs`) のみ。
ファイル冒頭コメント自身が「実 workload の allocator はまだ Rust 側 bump allocator が担っている」
「selfhost module 単体では mark-sweep + free-list の最小意味論を持たせる」と断っている。
**回収機構の実体は 3 つあり、そのうち実行中に走るものは 0 である。**
`phase11-implementation-plan.md:713` はこれを telemetry の固定として記録しており、
同じ行の末尾に「S14-S16 を閉じるには selfhost 側の broader closure/indirect heap 値判定と
compiler-side GC-safe point spill / shadow stack 完全列挙がなお必要」と書いている。
**実行中に回せないのは safe point が未完だからで、これは native 固有の欠落ではない。**

### 構図の訂正

`NATIVE-HEAP-02` の本質は「native lane に回収機構が無い」ではない。

> **両 lane とも実行中は回収しない設計であり、線形メモリを grow できない native lane でだけ
> それが crash として顕在化している。**

wasm lane が落ちないのは回収しているからではなく、memory.grow できるからである。
この違いを取り違えると「wasm から native へ collector を移植すれば直る」と読めてしまうが、
**移植しても呼ぶ場所が無いので何も変わらない。**

## 選択肢

| 案 | 内容 | 現時点の評価 |
|---|---|---|
| A | safe point を完備し、collector を確保パスから起動できるようにする | **根治。ただし wasm lane を含む共通課題** (`phase11` S14-S16)。native への port はその後の話で、port 自体は tagged pointer / object header / root stack が両 lane で共通なぶん見通しが良い。frontier 表現だけが非対称 (x86 は heap 先頭 16 bytes、aarch64 は `x22` レジスタ) |
| B | phase 境界での arena reset | safe point の完備を待たずに `I-13` の「累積確保回数に比例」へ直接効く。phase を跨いで生存するデータの識別が要る。**A の前段として単独で成立する** |
| C | 確保量の削減 (`map-new` の固定 64 KiB を縮める / 遅延確保) | **回収機構を入れずに問題が消える可能性がある。** 容疑 A が主因なら最小の変更。既存 collector の size class が 1024 bytes までしか無く 64 KiB が oversize 行きになることも、この確保が設計の想定外である傍証 |
| D | heap 拡大 | **却下。** 4 GiB → 8 GiB の実測で拡大分をちょうど使い切って落ちた (`I-13`)。env knob も緩和策にならないことが同 issue に記録済み |
| E | wasm の collector を native へ移植するだけ | **却下。** 実行中の呼び出し元が両 lane に存在しないため、移植しても回収は起きない。案 A の一部として初めて意味を持つ |

**A / B / C は現時点で決めない。** 帰属が未確定のまま方式を選ぶと、
効かない機構を実装する危険がある (案 D が実測で潰れたのと同じ失敗の型)。

## 受入条件

1. **確保の帰属を動的に計測する。** `App/Cli.ls` を入力とする実行で、
   `map-new` / `vector-push` grow / `string-concat` がそれぞれ何回・何 bytes 確保するかを数える。
   **計測は wasm lane で行ってよい** — 呼び出し回数は program logic の性質であり、
   lane によらない。native stage 再生成より安い経路である。
   (確保**サイズ**は lane で違うので、そちらは上表を使う。)
   **計装を新規に書く必要は無い**: 総量は `__lsharp_heap_ptr` と `__lsharp_heap_start` の差、
   総回数は `__lsharp_alloc_count` で取れる (`wasi.rs:124-126`、読み出し方は
   `tests/e2e/support.rs:585` の `read_i32_global` が実例)。
   内訳は `application_map_allocation.rs:20` の `default_cap` を変えた版との差分で切り分ける。
2. 計測結果を本 ADR の Evidence 節へ戻し、容疑 A / B / C の寄与率を確定させる。
3. その上で案を選ぶ。**選ばなかった案とその理由を本 ADR に残す。**

## 含めない範囲

- `NATIVE-HEAP-01` (bounds check)。ただし上記のとおり対象が aarch64 の確保系 helper 9 つ
  全部であり、かつ上限値の置き場所から設計が要る事実は `I-13` / `TODO.md` へ反映する。
- x86 / aarch64 の frontier 表現の統一。案 A を選んだ場合にのみ必要になる。
- safe point の完備 (`phase11` S14-S16)。案 A の前提だが本項目より広く、wasm lane も対象である。

## Evidence

未取得 (受入条件 1 が先)。
