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
- `test_e2e_native_host_binary_local_roundtrip_link_and_execute`
- `test_e2e_native_host_binary_i32_add_link_and_execute`
- `test_e2e_native_host_binary_i32_mul_link_and_execute`
- `test_e2e_native_host_binary_drop_restores_previous_value`
- `test_e2e_native_host_binary_direct_call_bundle_link_and_execute`
- `test_e2e_native_host_binary_direct_call_arg_bundle_link_and_execute`
- `test_e2e_native_host_binary_direct_call_two_arg_bundle_link_and_execute`
- `test_e2e_native_host_binary_direct_call_three_arg_bundle_link_and_execute`
- `test_e2e_native_host_binary_direct_call_four_arg_bundle_link_and_execute`
- `test_e2e_native_host_binary_direct_call_five_arg_bundle_link_and_execute`
- `test_e2e_native_host_binary_direct_call_six_arg_bundle_link_and_execute`
- `test_e2e_native_host_binary_direct_call_seven_arg_bundle_link_and_execute`
- `test_e2e_native_host_binary_direct_call_eight_arg_bundle_link_and_execute`
- `test_e2e_native_host_binary_direct_call_nine_arg_bundle_link_and_execute`
- `test_native_codegen_emits_x86_i32_core_instruction_bytes`
- `test_native_codegen_emits_x86_i32_mul_bytes`
- `test_native_codegen_emits_x86_direct_call_bundle_bytes`
- `test_native_codegen_emits_aarch64_direct_call_bundle_bytes`
- `test_native_codegen_emits_x86_direct_call_arg_bundle_bytes`
- `test_native_codegen_emits_x86_direct_call_two_arg_bundle_bytes`
- `test_native_codegen_emits_x86_direct_call_three_arg_bundle_bytes`
- `test_native_codegen_emits_x86_direct_call_four_arg_bundle_bytes`
- `test_native_codegen_emits_x86_direct_call_five_arg_bundle_bytes`
- `test_native_codegen_emits_x86_direct_call_six_arg_bundle_bytes`
- `test_native_codegen_emits_x86_direct_call_seven_arg_bundle_bytes`
- `test_native_codegen_emits_x86_direct_call_eight_arg_bundle_bytes`
- `test_native_codegen_emits_x86_direct_call_nine_arg_bundle_bytes`
- `test_e2e_native_host_binary_direct_call_ten_arg_bundle_link_and_execute`
- `test_native_codegen_emits_x86_direct_call_ten_arg_bundle_bytes`
- `test_e2e_native_host_binary_direct_call_eleven_arg_bundle_link_and_execute`
- `test_native_codegen_emits_x86_direct_call_eleven_arg_bundle_bytes`
- `test_e2e_native_host_binary_direct_call_twelve_arg_bundle_link_and_execute`
- `test_native_codegen_emits_x86_direct_call_twelve_arg_bundle_bytes`
- `test_e2e_native_host_binary_direct_call_thirteen_arg_bundle_link_and_execute`
- `test_native_codegen_emits_x86_direct_call_thirteen_arg_bundle_bytes`
- `test_e2e_native_host_bundle_uses_canonical_artifact_contract`
- `test_e2e_stage23_native_host_bundle_proxy_observations_match`

これらは native module skeleton / execution slice / structural parity に加えて、representative build entry における canonical artifact 名 / response file text / bundle summary 契約、tiny host-target program に対する canonical bundle materialization、および host-side proxy の `stage2-native` / `stage3-native` compare loop を示す partial evidence である。
加えて `scripts/ci/build-native.sh` により、Darwin arm64 ホストでは `stage1-native` / `stage2-native` / `stage3-native` の proxy bundle artifact を `ci-artifacts/native-proxy/<id>/` に materialize でき、representative build entry の IR opcode gap を `actual-stage23-gap.json` として出力できる。
また `selfhost/src/Backend/Native/NativeCodegen.ls` は x86_64 / aarch64 ともに `LocalGet` / `LocalSet` を stack slot として emit できるようになり、host-target の local roundtrip は `test_e2e_native_host_binary_local_roundtrip_link_and_execute` で固定済みである。
加えて i32 arithmetic core (`I32Const`, `I32Add`, `I32Mul`, `I32WrapI64`, `I64ExtendI32S`, `I64ExtendI32U`) も両 arch の backend に追加され、AArch64 host execution と x86_64 byte invariant で固定された。`Drop` も `local.get` 前段値への limited restore に加え、1-arg direct call 後の previous value、2-arg direct call 後の one-deeper spilled previous、さらに one-spill 3-value window 上の `drop; drop` で bottom value まで戻る path までは host test で固定済みだが、representative stage23 全面からはまだ blocker を外していない。
さらに `emit-native-bundle` による direct intra-bundle call の最小 slice を追加し、x86_64 では rel32 call bytes、AArch64 では `fp/lr` save/restore を伴う direct call bundle host execution を固定した。加えて `emit-native-function-meta-bundle` により function-meta (`[param-count, local-count, ir]`) を受ける 1-arg〜52-arg direct call まで追加され、callee parameter を local slots へ spill して host execution できるようになった。1-arg call では x86_64 `push/pop rcx`、AArch64 `x10` 退避で single previous-value preservation を入れ、続く 3-value〜52-value window 用 spill slot を導入して 3-arg〜52-arg marshaling と deeper previous restore の limited path を固定した。52-arg top の証跡として `test_e2e_native_host_binary_direct_call_fifty_two_arg_bundle_link_and_execute`、`test_native_codegen_emits_x86_direct_call_fifty_two_arg_bundle_bytes`、`test_e2e_native_host_binary_fifty_two_i32_const_window_keeps_latest_value`、`test_e2e_native_host_binary_fifty_two_arg_local_get_49_roundtrip`、`test_e2e_native_host_binary_fifty_two_arg_local_get_51_roundtrip` を追加した。
これは no-arg / 1-arg〜52-arg / intra-bundle の call-frame discipline を示す partial evidence であり、representative stage23 の `Call`（53+ args、import/runtime 境界、52-value window を超える deeper stack preservation、実 LoweredModule function plumbing）をまだ置き換えてはいない。
その結果 `actual-stage23-gap.json` の先頭 blocker は引き続き representative `Call`、`Drop`、`I32Store`、`I32Load`、`I64Load/Store`、control-flow / memory ops 群である。
ただし `stage1-native -> stage2-native -> stage3-native` の true self-regeneration 完了証跡ではない。

## 設計

### build entry の固定

- `selfhost/src/App/Main.ls` compile を representative entry に固定する
- `program.o`, `runtime.o`, `linker-response.txt`, `program.native` の artifact 契約を target ごとに固定する
- `selfhost/src/App/PipelineSmoke.ls` から canonical artifact 名と response-file text を hash/length summary として観測できるようにする
- `scripts/ci/build-native.sh` は現時点では true native compiler build ではなく、canonical proxy bundle を artifact 化する build entry として扱う
- `actual-stage23-gap.json` は representative build entry を actual stage23 へ進めるための blocker report として扱う

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
