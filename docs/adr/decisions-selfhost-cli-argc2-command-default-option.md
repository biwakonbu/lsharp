# argc 2 の command 既定 option は command ごとに決める (compile target を流用しない)

- **Status**: accepted
- **Date**: 2026-08-23
- **Scope**: `selfhost/src/App/Cli.ls:2682` と `selfhost/src/App/EmbeddedCli.ls:1730` の
  「option を伴わない command (`argc` 2) の fallthrough」が `run-command` へ渡す opts の決め方。
  option 解析経路 (`parse-cli-options` / `parse-test-cli-option` / `parse-check-cli-option`) の
  文法、`compile` / `build` の target option の意味、出力形式そのものは変えない。
- **Related**:
  [`I-66`](../../ISSUES.md#i-66) (本 ADR が解く問題),
  [`I-64`](../../ISSUES.md#i-64) (この食い違いが観測されずに残っていた理由),
  [decisions-selfhost-example-coverage-count.md](decisions-selfhost-example-coverage-count.md)
  (同じ 2 系統 CLI の契約差を扱った直前の slice)

## 問題

`Cli.ls` / `EmbeddedCli.ls` はどちらも、option を伴わない command を最後の else 節で

```
(run-command cmd-name file-path (default-compile-target))
```

と処理する。`run-command` の第 3 引数は command ごとに意味が違う整数であり、
`compile` では compile target、`test` では出力形式、`check` でも出力形式である。
**番号空間が重なっている。**

| command | 0 | 1 |
|---|---|---|
| `compile` / `build` | `compile-target-preview1` | `compile-target-component` |
| `test` | text | `test-option-json` |
| `check` | text | `check-option-json` |

したがって fallthrough が渡す `(default-compile-target)` の値が、そのまま
`test` / `check` の出力形式になる。2 系統で既定が違う:

| | `default-compile-target` | `lsharp test f.ls` | `lsharp check f.ls` |
|---|---|---|---|
| `Cli.ls:46` | `compile-target-preview1` = 0 | text | text |
| `EmbeddedCli.ls:44` | `compile-target-component` = 1 | **JSON** | **JSON** |

`EmbeddedCli` では `--format json` を付けていないのに assurance JSON が出て、
`run-test-source-text` (`:1201`) は `test` command から到達しない。

**`I-66` は `test` だけを記録していたが、`check` も同じ機構で同じように割れている。**
`check-option-json` (`EmbeddedCli.ls:86`) も 1 だからである。

### 同じ fallthrough を通るが影響を受けない command

`run-command` (`EmbeddedCli.ls:1677`) が受ける command は 10 個あり、argc 2 では
**その全部**が `(default-compile-target)` を第 3 引数として受け取る。ただし option 値を
実際に見ているのは以下だけで、比較先の定数が 0/1 と重ならないものは影響を受けない。

| command | 見ている定数 | 値 | 0/1 と重なるか |
|---|---|---|---|
| `test` | `test-option-json` | 1 | **重なる** |
| `check` | `check-option-json` | 1 | **重なる** |
| `review` | `review-option-json` | 2 | 重ならない |
| `doc-ack` | `doc-option-trailer-only` | 10 | 重ならない |
| `doc-check` | `doc-option-strict-check` | 11 | 重ならない |
| `compile` / `build` | compile target | 0 / 1 | 本来の受け手 |
| `parse` / `fmt` / `validate` | 参照しない | -- | -- |

`review` / `doc-ack` / `doc-check` は 0 でも 1 でも既定枝 (`print-doc-payload` /
非 JSON review) へ落ちるので、**本 ADR の scope は `test` と `check` の 2 command に限る**。
番号空間を 10 番台へ離した `doc-*` が無傷である事実は、決定 3 の「離せば解ける」根拠でもある。

## 決定

### 1. 正は text lane とする

参照実装が決めている。`crates/lsharp-driver/src/main.rs:201` の `Test.format` は

```rust
#[arg(long, value_enum, default_value = "text")]
```

であり、`lsharp test input.ls` は text を出す。`Cli.ls` は既にこれと一致する。
**割れているのは `EmbeddedCli.ls` 側だけであり、そちらを寄せる。**

`check` は公開 CLI に無い (`CLAUDE.md`: `parse` / `check` / `fmt` は LSP / MCP の内部 API)。
参照が無いので、`test` と揃える方を採る。2 系統 2 command で既定が 1 つになる。

### 2. 直す場所は `default-compile-target` ではなく fallthrough 側

`EmbeddedCli` の既定 target が component なのは意図であり、
`(default-compile-target)` を 0 に変えると `lsharp compile f.ls` の出力形式が変わる。
**compile の既定は触らない。**

fallthrough を command ごとの既定 option へ差し替える:

```
(defn default-option-for-command [cmd-name]
  (if (string-eq cmd-name "test") (test-option-text)
    (if (string-eq cmd-name "check") (check-option-text)
      (default-compile-target))))
```

`test-option-text` / `check-option-text` を 0 として明示的に定義する。
現状 0 は「`-json` でない方」という暗黙値でしかなく、名前が無いこと自体が
番号空間の重なりを見えなくしていた。

### 3. 番号空間の重なりそのものは解かない

command ごとに別 enum を切る案 (下記却下 C) は採らない。重なりが実害になるのは
「compile target の値を test へ渡す」経路が存在するからで、決定 2 でその経路が消える。
消えた後の重なりは、`run-command` の第 3 引数が多義であるという事実の反映にすぎない。

ただし**消えたことを検査に変える**。`lsharp test f.ls` / `lsharp check f.ls` が
両系統で text を出すことを live な e2e (`#[ignore]` 無し) で固定する。
これが無いと、将来 `default-compile-target` を動かしたときに同じ形で黙って戻る。

## 却下した案

- **A. `EmbeddedCli` の `default-compile-target` を 0 にする。**
  `test` / `check` は直るが `compile` の既定 target が preview1 に変わる。
  EmbeddedCli が component を既定にしているのは意図的な設計であり、
  片方の bug を直すために別 command の契約を壊す。**副作用の方が大きい。**

- **B. `EmbeddedCli` を JSON 既定のままとし、`Cli.ls` を JSON へ寄せる。**
  「JSON の方が情報量が多い」は `I-66` に書いたとおり事実だが、
  参照実装 (rust driver) が text 既定である以上、2 系統が揃っても 3 系統目と割れる。
  **揃える先を参照実装以外に置く理由が無い。**

- **C. command ごとに option enum を分ける。**
  構造としては正しいが、`run-command` の signature と全呼び出し (`compile` / `build` /
  `test` / `check` / `doc-ack` / `doc-check` / `fmt` / `validate`) に波及する。
  `I-66` は影響度**低**であり、投じる変更量に見合わない。
  決定 2 で実害の経路は消えるので、**今は決定 3 の検査で足りる。**
  再検討の引き金は「`run-command` の第 3 引数を必要とする command がもう 1 つ増えたとき」。

- **D. fallthrough を廃し、argc 2 でも option 解析器を通す。**
  `parse-test-cli-option` は `argc > 2` を前提に書かれており、
  引数が無い場合の戻り値を新たに決める必要がある。
  「解析器を通す」ことにした瞬間、解析器の既定値が今の `default-compile-target` と
  同じ役割を負う。**問題の置き場所が変わるだけで解けていない。**

## Evidence

<!-- doc-GREEN: 実装後に埋める。RED の test 名 / 両系統の実測出力 / 受入判定 -->

## 満たしていないこと

<!-- doc-GREEN: 実装後に埋める -->
