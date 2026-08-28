# representative native code の tail を指す offset の求め方

- **Status**: doc-GREEN (focused 3 本まで / lane 未了 / 2026-08-28)
- **Date**: 2026-08-28 (doc-RED)
- **Scope**: `selfhost_native_stage_chain.rs` の base64 tail probe 3 件
  (`:21689` / `:21734` / `:25576` 付近) が `code` のどこを読むかの決め方。
- **含めない範囲**: `byte-at-or-zero` / `build-base64-chunk-text` の実装そのもの。
  `ci-artifacts/**/seed.ls` に焼き込まれた同一 helper (生成物なので直接直さない)。
  native backend で同じ read をしたときの挙動 (`I-13` の範疇)。
- **Related**: `ISSUES.md` の `I-99` / `TODO.md` の `NATIVE-TAIL-OFFSET-PIN-01`

## 何が問題か

`..._base64_tail_slice_stays_decodable` は `start` に **10,174,680 という絶対 offset を
直書き**しており、representative build の native code が selfhost の source 変更で
伸縮することを考慮していない。

`build-base64-chunk-text` の bound は vector 長ではなく **caller が渡した `end`** である
(`byte-at-or-zero bytes idx end`)。したがって `(vector-length code)` が `end` より短いと
guard は素通りし、`vector-get` が vector の外を読んで
`wasm trap: out of bounds memory access` になる。`I-99` に機構の全経路が書いてある。

同じ offset 帯を触る緑 2 件 (`:21734` / `:25576`) は `len` に真の vector 長を渡すので
trap しないが、**範囲外なら 0 が返るだけなので assertion が無内容**である
(`lines.len()` と「digit が 0..63」しか見ておらず、全部 0 でも通る)。

## 判断

**tail を指す offset は `(vector-length code)` から導く。3 件とも直書き定数を捨てる。**

同一ファイルの `..._entrypoint_slice_base64_matches_raw_bytes` (`:21771` 付近) が
`run_selfhost_main_representative_aarch64_layout_harness().entrypoint_offset` から
offset を導いて緑であり、**同じ commit `ae24e1f6` (2026-04-26) 生まれ**である。
正しい書き方は最初から隣にあった。本 ADR はその方式へ寄せるだけで、新しい設計はしない。

| test | 旧 | 新 |
|---|---|---|
| `..._base64_tail_slice_stays_decodable` | `start 10174680` / `end (+ start 48)` | `end (vector-length code)` / `start (- end 48)` |
| `..._tail_code_bytes_reveal_signed_values` | `10174692`..`10174695` | `(- len 4)`..`(- len 1)` |
| `..._tail_base64_quad_intermediates_are_bounded` | `idx 10174692` | `idx (- len 3)` |

### これは却下案 (真の vector 長を渡すだけ) ではない

`TODO.md` の受入条件 (c) は「`byte-at-or-zero` に真の vector 長を渡すだけの修正で
済ませないこと。trap は消えるが範囲外が静かに 0 で埋まり、主張が無内容になる」と定めていた。

**本案はそれに当たらない。** `start = len - 48` / `end = len` なので、
読む index はすべて `len` 未満であり、**0 埋めは 1 byte も起きない**。
読んでいるのは実在する native code の末尾 48 byte そのものである。
「tail が decodable」という主張は内容を持つ。

## 判別力の無い緑 2 件をどうするか (受入条件 (d))

offset を直しただけでは、この 2 件は「範囲外でも通る」から
「範囲内だが値を見ていない」へ変わるだけである。**そこで assertion を足す。**

### `..._tail_code_bytes_reveal_signed_values`

`(byte-at-or-zero code len len)` を 1 本足し、**`idx == len` で guard が効いて 0 になる**
ことを pin する。これは `I-99` が名指しした「bound が何であるか」を境界で直接固定する
assertion であり、値が何であっても成立を要求する。加えて `len` を印字して
`len >= 4` と 4 byte が `0..255` に収まることを見る。

### `..._tail_base64_quad_intermediates_are_bounded`

`b0` / `b1` / `b2` から `s0`..`s3` を **Rust 側で組み直して突き合わせる**。
L# 側の quad 分解と Rust 側の再計算が一致することを要求するので、
全部 0 でも通る形ではなくなる (0 なら 0 同士で一致するが、
非 0 のときに分解が狂えば落ちる)。従来の「digit が 0..63」は残す。

**正直に書いておく**: この 2 件の assertion が見ているのは
「helper の bound と算術が正しいこと」であって、
「native code の tail の中身が正しいこと」ではない。後者を見るには
同じ byte を独立な経路でもう一度読む必要があり、本 slice では harness 1 本を
追加していない。**強くはなったが、code の中身を検査する test にはなっていない。**

## 却下した案

### 案 A: `start` を別の定数へ更新する

**却下。** 今日の code 長に合わせても、selfhost の source が変わればまた陳腐化する。
`I-99` が問題にしているのは値ではなく「絶対 offset を直書きしている」ことである。

### 案 B: `byte-at-or-zero` の signature を変えて bound を必ず vector 長にする

**却下 (本 slice では)。** helper は 2 つの harness template に重複定義されており
(`:16996` と `:21580`)、さらに `ci-artifacts/**/seed.ls` に焼き込まれている。
呼び出し元 `print-base64-chunks` / `write-base64-chunks` は既に真の vector 長を
渡しているので (`end = min(idx+48, len)`)、**罠に掛かっていたのは harness 側の 1 件だけ**である。
helper の契約変更は影響範囲が広い割に、直る赤は同じ 1 件である。

ただし **`byte-at-or-zero` という名前が「範囲外なら 0」を約束しているように読める**
という `I-99` の指摘は残る。名前と契約の食い違いは `ISSUES.md` の `I-99` が持ち続ける。

## 全経路の走査 (受入条件 (e))

`build-base64-chunk-text` の呼び出しは **4 箇所**しかない
(`ci-artifacts` の生成物を除く)。

| 呼び出し元 | 渡す `end` | 判定 |
|---|---|---|
| `print-base64-chunks` (`:19242`) | `(if (< (+ idx 48) len) (+ idx 48) len)` | 真の vector 長で clamp。安全 |
| `write-base64-chunks` (`:19252`) | `(if (< (+ idx 768) len) (+ idx 768) len)` | 同上。安全 |
| `..._base64_tail_slice_stays_decodable` (`:21695`) | 直書き `10174680 + 48` | **本件** |
| `..._entrypoint_slice_base64_matches_raw_bytes` (`:21803`) | `entrypoint_offset + 48` (実測由来) | 緑。対照実験 |

**`I-99` が「2 経路しか見ていない」と書いた範囲を全 4 経路へ広げた。同じ罠は他に無い。**

## Evidence

### (a) `(vector-length code)` の実測値 -- **旧定数は末尾の遥か外だった**

測定のために `..._base64_tail_slice_stays_decodable` へ `(print len)` と
`println!("representative native code len: {code_len}")` を足した
(in-repo 先例: 同ファイル `:25572` の `println!("latest owner decl: {lines:?}")`)。

| 項目 | 値 |
|---|---|
| 起動 | `/Users/biwakonbu/github/tmp/i98/run_len.py` を `os.setsid()` で切り離し。pid 49817 |
| ログ | `/Users/biwakonbu/github/tmp/i98/len.log` |
| 実測 | `representative native code len: 5746740` |
| 結果 | `test result: ok. 1 passed; 0 failed` / `3080 filtered out` / `RUNEXIT=0` / `ELAPSED=68.71` |

| | byte |
|---|---|
| representative native code の実長 | **5,746,740** |
| 旧 test が読もうとした `start` | 10,174,680 |
| 末尾を超えていた量 | **+4,427,940** (実長の約 0.77 倍ぶん外側) |

**旧定数は末尾の「少し先」ではなく、code 全体をもう一本ぶん近く超えた位置だった。**
`build-base64-chunk-text` の bound は caller の `end` なので、`byte-at-or-zero` の
guard は効かず、この read はそのまま trap する。`I-99` の読みどおりである。

出所は selfhost の source が縮んだことではなく、**そもそも一度でも当たっていたのか
確かめられない値だった**という点にある。10,174,680 が有効だった時点の code 長は
本 slice では特定していない (下記「満たせなかったこと」)。

### (b)(c) 3 本の再測定 -- すべて緑

| test | 結果 |
|---|---|
| `..._base64_tail_slice_stays_decodable` | ok |
| `..._tail_code_bytes_reveal_signed_values` | ok |
| `..._tail_base64_quad_intermediates_are_bounded` | ok |

前者 3 本は 4 本まとめた run (`/Users/biwakonbu/github/tmp/i98/focused.log`) で緑、
`..._base64_tail_slice_stays_decodable` は上記の単独 run でも緑。

**48 byte は 3 の倍数なので pad が入らず、base64 は 64 文字ちょうどになる**という
`assert_eq!(len, 64, ...)` も通った。これは「`build-base64-chunk-text` が
`start`..`end` の全 byte を実際に符号化した」ことの証拠であり、
0 埋めで誤魔化されていないことを示す。

### (d) 判別力の補強も緑

- `..._tail_code_bytes_reveal_signed_values`: `idx == len` を 1 本撃ち、
  `byte-at-or-zero` の guard が **真の vector 長**で効くことを境界で pin した。
  `lines[4] == 0` が通った
- `..._tail_base64_quad_intermediates_are_bounded`: L# 側の quad 分解を
  Rust 側で `[b0/4, (b0%4)*16 + b1/16, (b1%16)*4 + b2/64, b2%64]` と組み直して
  突き合わせた。**全部 0 でも通る形にはなっていない**

### 測定中に起きた SIGKILL (記録)

4 本を 1 プロセスにまとめた最初の run で、4 本目の
`..._typeinfer_program_apply_matches_selfhost` が `signal: 9` で殺された
(`RUNEXIT=101` / `ELAPSED=304.65`)。assertion 失敗ではない。

**結果を見る前に予測を書き** (`/Users/biwakonbu/github/tmp/i98/prediction.md`)、
同じ binary・同じ test を 1 本だけ回して判別した。**単独では完走して緑**
(`test result: ok. 1 passed; 0 failed` / `finished in 119.20s`)。
したがって原因は harness 変更ではなく、1 プロセスに 4 本詰めたことによる常駐量の蓄積である。

**kill の主体までは示せていない。** `log show` に jetsam / memorystatus の記録は
見つからなかった。示せたのは「4 本同居なら死に、単独なら死なない」という相関だけである。

**運用への含意**: `selfhost_native_stage_chain` の重い test を focused run で
複数まとめない。`SWEEP-LANE-RERUN-01` の lane が落ちたときは、
`MODEXIT` が `-9` (SIGKILL) か `101` (libtest の通常の test 失敗) かを先に見ること。

## 満たせなかったこと

- **lane を回していない。** focused 3 本の緑は lane 1 本の完走ではない。
  台帳 (`ignored-lane-expected-failures.txt:411`) の行はまだ落としていない。
  `SWEEP-LANE-RERUN-01` が 7 項目まとめて引き取る。
- **10,174,680 が有効だった時点を特定していない。** 「いつ陳腐化したか」は
  `git log -S"10174680"` で辿れるはずだが、本 slice では追っていない。
  `(vector-length code)` 相対にした以上、再発しないので追う実益が薄いと判断した。
- **`code` の中身は依然として検査していない。** 本 slice が強めたのは
  「範囲外を読まない」「分解が正しい」の 2 点であって、
  末尾 48 byte が native code として何であるべきかは定めていない。
  これは元の test の設計がそうであり、本件で変えていない。
- **`println!` の追加は測定のためであり、恒久的な出力である。** lane では
  `--nocapture` を付けないので出力されないが、付けた場合は 1 行増える。
