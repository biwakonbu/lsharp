# V2-08: Native backend self-regeneration

## 概要

Wasmtime embedding + Component Model を正式配布モデルに据える方針転換により、native backend self-regeneration は Phase 11 の completion gate から外れた。  
ただし `selfhost/src/Backend/Native/` 配下の backend 実装と既存 E2E evidence は維持しており、このページでは Deferred/v2 として再開する際の正本条件を定義する。

## 前提条件

- Wasm bootstrap (`BOOT-04`) を mainline の正本として維持する
- native backend の deterministic slice (`NATIVE-04`) を壊さない
- 公式配布が host launcher + embedded guest component であることを維持する

## 現状 evidence

- `test_e2e_native_self_regeneration_functional_equivalence`
- `test_e2e_native_stage_chain_structure`
- `test_native_pipeline_complete_chain`
- `test_native_codegen_emit_standalone_execution`
- `test_native_codegen_real_execution`
- `test_native_codegen_emits_full_const_instruction_bytes`
- `test_e2e_native_linker_exposes_canonical_stage_artifact_paths`
- `test_e2e_native_linker_generates_canonical_response_file_text`
- `test_e2e_selfhost_main_native_bundle_summary_matches_canonical_contract`
- `test_e2e_stage1_native_observation_summary_two_run_determinism`
- `test_e2e_native_host_bundle_uses_canonical_artifact_contract`
- `test_e2e_stage23_native_host_bundle_proxy_observations_match`

これらは native module skeleton / execution slice / structural parity に加えて、representative build entry における canonical artifact 名 / response file text / bundle summary 契約、tiny host-target program に対する canonical bundle materialization、および host-side proxy の `stage2-native` / `stage3-native` compare loop を示す partial evidence である。
ただし `stage1-native -> stage2-native -> stage3-native` の true self-regeneration 完了証跡ではない。

## 設計

### build entry の固定

- `selfhost/src/App/Main.ls` compile を representative entry に固定する
- `program.o`, `runtime.o`, `linker-response.txt`, `program.native` の artifact 契約を target ごとに固定する
- `selfhost/src/App/PipelineSmoke.ls` から canonical artifact 名と response-file text を hash/length summary として観測できるようにする

### stage chain

- `stage1-native` で selfhost compiler を生成する
- `stage2-native` を `stage1-native` で再生成する
- `stage3-native` を `stage2-native` で再生成する
- 現時点では `selfhost/src/App/Main.ls` smoke 出力から `native summary + bundle summary` をまとめた observation surface を比較面として使う
- さらに現時点では tiny host-target program に限り、canonical artifact bundle を host で materialize して `stage2-native` / `stage3-native` compare の proxy とする
- representative input set に対して exit code / stdout / stderr / artifact hash を比較する

### tier1 target matrix

- `x86_64-apple-darwin`
- `aarch64-apple-darwin`
- `x86_64-unknown-linux-gnu`

## 正本参照

- plan 正本: [`../phase11-implementation-plan.md#v2-08-native-backend-self-regeneration`](../phase11-implementation-plan.md#v2-08-native-backend-self-regeneration)
- completion gate 境界: [`../completion-criteria.md`](../completion-criteria.md)
- native backend 契約: [`../../../language/native-backend-spec.md`](../../../language/native-backend-spec.md)

## ステータス

Deferred。Phase 11 完了判定には含めない。既存テスト群は partial evidence として維持し、true self-regeneration が成立するまでは `[x]` に上げない。
