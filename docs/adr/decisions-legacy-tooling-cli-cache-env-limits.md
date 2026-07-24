# ADR: CLI artifact cache maintenance limit の環境変数

- Status: Accepted (verified partial slice)
- Date: 2026-07-24
- Scope: `crates/lsharp-driver/src/main.rs`
- Related: `decisions-legacy-tooling-cli-cache-env.md`, `decisions-legacy-tooling-cli-cache-maintenance.md`,
  `decisions-legacy-tooling-cli-cache-byte-budget.md`

## Context

C-2p で cache root を `LSHARP_ARTIFACT_CACHE_DIR` から opt-in できるようになった。root だけを環境変数で共有すると、entry/byte
budget を script ごとに argv へ重複して書く必要が残る。一方、値の parse を曖昧にして invalid limit を無視すると、cache の容量契約が
呼び出し方によって変わるため、CLI と環境変数の precedence と error boundary を固定する必要がある。

## Decision

- `LSHARP_ARTIFACT_CACHE_MAX_ENTRIES` / `LSHARP_ARTIFACT_CACHE_MAX_BYTES` は、それぞれ対応する CLI flag が未指定の場合だけ使用する。CLI flag が常に優先され、環境変数の invalid value も explicit flag で shadow される。
- 未設定は limit 無効を意味する。空値、非 UTF-8、数値でない値は compile 前に stable error とし、負数や overflow も parse error とする。
- 解決済み root（CLI または `LSHARP_ARTIFACT_CACHE_DIR`）に対して entry trim、続けて byte trim を従来どおり成功後に適用する。root が得られない場合の併用エラー契約は維持する。
- limit env のいずれかが設定された compile/build では built-in embedded component delegation を抑止し、host-only maintenance が guest の成功へ隠れないようにする。外部 `LSHARP_PATH` delegation、Native target、`emit_ir`、automatic eviction は変更しない。

## Evidence

- RED: limit resolver と delegation guard を先にテストし、未実装時に resolver 未定義および引数数不一致の compile error を確認した。
- GREEN: CLI precedence、env parse、unset、invalid entry、empty byte の driver test 5 件と delegation guard test 1 件が通過した。
- `cargo test -p lsharp-driver --bin lsharp -- --nocapture --test-threads=1` は 124 件、既存 C-2p を含めて全件成功した。tooling cache 7 件、
  driver clippy (`-D warnings`)、rustfmt、`scripts/audit_docs.sh` も成功した。
- manual smoke では root と limits の env を同時に指定し、entry limit `0` を CLI の `2` で上書きした compile は artifact 1 件を残し、
  続く env entry/byte budget (`1` / `1`) の compile は artifact 0 件になった。不正な byte env は exit `1` と parse error になった。

## Consequences

host script は root と bounded maintenance policy を同じ explicit env contract で再現できる。env は user-wide default や automatic
eviction ではなく caller-owned opt-in のままであり、公開 cache key の SCC 公開 surface 統合、Native/selfhost persistence、両対応 target の
Rust-free evidence は未完了として残る。
