# Component Model 仕様

## 目的

本書は、L# の正式配布モデルとして採用する Wasmtime embedding + Component Model の契約を定義する。
L# compiler の成果物を guest component (`.component.wasm`) として配布し、Wasmtime を内包した host launcher (Rust binary) に埋め込んで single binary として配布する。

## 適用範囲

本書が扱うのは次の領域である。

- WIT world 定義と host/guest 境界
- Component Model adapter の post-processing 方式
- Single binary embedding 仕様
- WASI preview1 から preview2 への移行戦略
- Host/guest 境界の粒度ルール

以下は本書の対象外とする。

- native backend の ABI や object 形式 (deferred, `native-backend-spec.md` 参照)
- GC 内部アルゴリズムの詳細 (`memory-management-roadmap.md` 参照)
- Phase 単位の実装計画やタスク管理

## 設計原則

1. **共通成果物は Wasm component**: OS 差は host launcher に閉じ込め、guest (L# code) は platform-agnostic
2. **Host boundary は粗粒度**: batch API を基本とし、細粒度な cross-boundary 呼び出しは禁止
3. **重い処理は guest 側**: compiler, runtime, stdlib の pure/core 部分は全て guest 内で実行
4. **WIT による契約**: host が提供する capability は WIT interface で明示的に定義

## 用語

本書では、正式配布モデルの主要語彙を次のように固定する。

- **host launcher**: Wasmtime と埋め込み資産を含む配布用バイナリ。OS ごとの差分はここに閉じ込める
- **guest component**: L# compiler / runtime / stdlib を含む `.component.wasm`。WIT world 契約に従って host launcher から instantiate される
- **single binary distribution**: `host launcher + embedded guest component + embedded stdlib` を 1 つの配布物として届ける方式

## アーキテクチャ

### パイプライン

```text
L# Source (.ls)
  -> Frontend (Lexer -> Parser -> MacroExpand -> TypeInfer)
  -> Lowering
  -> LoweredModule
  -> Wasm Codegen (core .wasm)
  -> Component Adapter (post-processing)
  -> .component.wasm
```

selfhost emitter は core Wasm のみを出力し、Component Model wrapping は host 側 (Rust) で行う。

### Single Binary 構成

```text
lsharp (host launcher)
  = Rust binary
  + Wasmtime engine
  + embedded guest component (.component.wasm)
  + embedded stdlib
```

host launcher は `include_bytes!` 等で guest component を埋め込み、起動時に `Component::new` で instantiate する。現行 Rust driver は既定で `selfhost/src/App/EmbeddedCli.ls` を build-time に `.component.wasm` 化して埋め込み、`parse` / `check` / `fmt` の default path として起動する。build-time 環境変数 `LSHARP_EMBED_COMPONENT_PATH` を与えれば custom guest で override でき、runtime `LSHARP_DISABLE_EMBEDDED_COMPONENT=1` を与えれば embedded guest を明示的に無効化して shadow-command / built-in path へ戻せる。core `.wasm` を直接配布の正本として扱わず、配布境界では常に guest component を正とする。

### 配布成果物

正式配布モデルの成果物は次で固定する。

| 成果物 | 役割 |
|--------|------|
| `guest component` (`.component.wasm`) | WIT world 契約を実装する platform-agnostic な guest |
| `host launcher` | Wasmtime embedding, capability wiring, CLI / server 起動導線 |
| `single binary distribution` | host launcher に guest component と stdlib を埋め込んだ最終配布物 |

## WIT World 定義

### lsharp-compiler (CLI 向け)

```wit
world lsharp-compiler {
  import wasi:io/streams@0.2.3;
  import wasi:filesystem/types@0.2.3;
  import wasi:cli/environment@0.2.3;
  import wasi:cli/exit@0.2.3;

  export wasi:cli/run@0.2.3;
}
```

`lsharp-compiler` world は、host launcher が CLI capability を guest component へ束ねて渡すための最小 world とする。CLI 実行の入口は host launcher 側にあり、guest は world 経由で必要な capability のみを利用する。現行実装は wasmtime-wasi 29 系の WIT set に合わせて `@0.2.3` を使用し、core module 側では canonical ABI export `wasi:cli/run@0.2.3#run` を生成する。

### lsharp-http-handler (HTTP server 向け)

```wit
world lsharp-http-handler {
  import wasi:http/types@0.2.3;
  export wasi:http/incoming-handler@0.2.3;
}
```

guest は `(defn handle [request] response)` で HTTP handler を記述し、host が accept loop / TLS / connection management を担当する。

現状の Rust 側は `crates/lsharp-wasm/build.rs` で `lsharp-http-handler` world を single-package staging directory へ複製し、stable Preview2 imports を `wasmtime_wasi::bindings::sync::*` へ remap した generated bindings と `host_bridge::link_http_handler_world()` を用意済み。host linker では stable WASI 部分を `WasiView`/`WasiImpl` で再利用しつつ、custom `wasi:http` traits だけを state 側で実装できる。加えて `wit-component` の `dummy-module` feature を使い、staged world から生成した dummy guest component を synthetic host bridge に instantiate できる smoke test まで固定した。canonical ABI 調査で得た export/import 名に合わせて、`emit_wasm_wasi_p2()` は `main` のない `handle` 1 引数モジュールを自動で HTTP handler component 化し、core export `cm32p2|wasi:http/incoming-handler@0.2|handle` / `handle_post` と `cm32p2_memory` / `cm32p2_realloc` / `cm32p2_initialize` を生成する。現行の response semantics は最小実装で、request は opaque handle として guest `handle` に渡し、guest 実行後に default `200` / empty body response を `response-outparam.set` で返す。`host_bridge::tests::test_http_handler_world_calls_lsharp_handle_and_sets_response_outparam` と `compile::tests::test_compile_file_handle_only_emits_http_handler_component_export` により、実 source からの compile/instantiate path まで固定済みである。

### World 定義ルール

WIT world 定義は次のルールに従う。

1. host launcher が提供する capability は全て import として明示する
2. guest component が外部へ見せる入口は world export に限定する
3. world はユースケース単位 (`lsharp-compiler`, `lsharp-http-handler`) に分割し、platform ごとの差を混在させない
4. preview1 互換 import を world の正本へ残さず、preview2 名称で固定する

## Host/Guest 境界ルール

### 粗粒度 batch API

host が提供する capability は batch 操作を基本とする。

許容される API 設計:

| API | 説明 |
|-----|------|
| `read-files(paths[]) -> map<path, bytes>` | 複数ファイルの一括読み込み |
| `write-outputs(outputs[])` | 複数出力の一括書き込み |
| `handle-http(req) -> resp` | 単一 HTTP request/response |
| `stat-many(paths[]) -> stats[]` | 複数ファイルの一括 stat |

避けるべき API 設計:

| API | 理由 |
|-----|------|
| `read-byte(fd)` | 1 byte ずつの cross-boundary 呼び出し |
| `get-header(name)` を多数回 | 細粒度な host 問い合わせ |
| `symbol-lookup(name)` を逐次 | 解析中の host 依存 |

理由: component 越しのデータ受け渡しには canonical ABI の lift/lower が入るため、細粒度呼び出しは構造的に不利。

### Guest 内部に閉じる処理

以下は全て guest 側 (Wasm 内) で完結する。

- bytecode VM / evaluator
- parser / type checker / optimizer
- stdlib の pure/core 部分
- scheduler / task system
- メモリ管理 (alloc / GC)

### Host Object 禁止

host 側のミュータブルオブジェクトを guest に渡してはならない。全てのデータは component 境界で値として受け渡す。

### Boundary 正規化ルール

- 文字列・パス・設定値は host launcher が preview2/WIT の値へ正規化して guest component に渡す
- guest component は OS 固有ハンドルや file descriptor 番号を内部表現として保持しない
- capability 追加が必要な場合は、先に WIT world を拡張し、runtime API と docs を同期してから host 実装を増やす

現状の host capability bridge は `crates/lsharp-wasm/src/host_bridge.rs` にあり、`wit/lsharp-core.wit` を `wasmtime::component::bindgen!` で束ねた `HostCapabilities` / `HostCapabilitiesView` / `link_host_capabilities()` を提供する。これにより host launcher は `host-fs` / `host-process` の batch API を Wasmtime component linker へ一括登録し、guest component 側へ coarse-grained capability を渡せる。

## 2 Target Compilation

| Target | 用途 | 成果物 |
|--------|------|--------|
| `wasi-component` (default) | CLI, server, 配布 | `.component.wasm` |
| `web-wasm` | ブラウザ | core `.wasm` (WASI import なし) |

`lsharp compile --target wasi-component` がデフォルト。`--target wasm` は `wasi-component` の alias。

## WASI Preview1 -> Preview2 移行戦略

### 段階的移行

1. **Phase 1 (dual-mode)**: `wasi_runner.rs` に preview2 execution path を追加。既存 tests は preview1 のまま維持
2. **Phase 2 (preview2 codegen)**: `wasi.rs` に `emit_wasm_wasi_p2()` を追加。9 WASI preview1 imports を Component Model interface へ変換
3. **Phase 3 (selfhost migration)**: `WasiBackend.ls` に target flag を追加。selfhost emitter が preview1 / preview2 を切り替え

移行期間中も正式配布物は host launcher + guest component を維持し、preview1 core `.wasm` は bootstrap / 移行検証用の中間成果物としてのみ扱う。

### WASI Import 対応表

| WASI preview1 | Component Model interface |
|----------------|--------------------------|
| `fd_write` | `wasi:io/streams.output-stream.blocking-write-and-flush` |
| `fd_read` | `wasi:io/streams.input-stream.blocking-read` |
| `args_get` / `args_sizes_get` | `wasi:cli/environment.get-arguments` |
| `path_open` / `fd_close` / `fd_seek` / `fd_filestat_get` | `wasi:filesystem/types.*` |
| `proc_exit` | `wasi:cli/exit.exit` |

## Component Adapter 方式

core Wasm module を component へ変換する際は post-processing approach を採用する。

```text
selfhost emitter -> core .wasm -> component_adapter (wit-component) -> .component.wasm
```

これにより selfhost emitter は core Wasm binary format のみを理解すればよく、Component Model binary format を直接出力する必要がない。

現状は `crates/lsharp-wasm/src/component_adapter.rs` の generic helper がこの責務を担う。

- `embed_component_metadata_for_world()` -- core module / adapter module に指定 world の `component-type` metadata を埋め込む
- `componentize_core_module()` -- metadata 埋め込み済み main module と named adapter 群から guest component bytes を生成する

official WASI Preview2 import mapping や selfhost compiler world への適用は `P13-A-2` / `P13-A-3` の責務として分離し、B-2 では host 側の generic post-processing layer を正本化する。

## 関連文書

- [`backend-boundary.md`](./backend-boundary.md) -- backend 境界仕様
- [`runtime-spec.md`](./runtime-spec.md) -- runtime 共通契約
- [`native-backend-spec.md`](./native-backend-spec.md) -- native backend と current supported target
- `docs/adr/decisions-002.jsonl` ADR-167 -- Phase 13 の完了判断
