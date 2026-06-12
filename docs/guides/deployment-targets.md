# Deployment Targets

`lsharp compile` は出力先や `--target` に応じて Wasm component、WASI preview1 wasm、web wasm、native artifact を扱います。通常は既定の `wasi-component` から始めます。

## Default: wasi-component

```bash
lsharp compile src/Main.ls -o main.component.wasm
```

- 既定の公開 compile target です。
- host launcher + embedded guest component の通常導線と整合します。
- README や release smoke の primary path として扱います。

## WASI Preview1

```bash
lsharp compile src/Main.ls --target wasi-preview1 -o main.wasm
```

- WASI preview1 runtime で動かす `.wasm` が必要な場合に使います。
- 既存の preview1 host や wasmtime smoke と合わせる用途です。

## Web Wasm

```bash
lsharp compile src/Main.ls --target web-wasm -o main.web.wasm
```

- browser 向け core wasm を出す経路です。
- 現時点では Rust host fallback 側の扱いを含みます。
- browser integration の host glue は利用側で用意します。

## Native

```bash
lsharp compile src/Main.ls --target native -o main.native
```

- native backend 経路です。
- product / release support target は Linux x86_64 と Mac Apple Silicon を通常対象にします。
- Windows native と Intel Mac native は通常サポート対象として案内しません。
- native backend の self-regeneration や Linux x86 stage chain は TODO.md の native track を正本として追跡します。

## Choosing A Target

- CLI と release smoke の最初の確認は `wasi-component` を使います。
- preview1 runtime と接続する必要がある場合だけ `wasi-preview1` を選びます。
- browser 用の core wasm が必要な場合は `web-wasm` を選びます。
- native artifact は supported product/release target 上で必要な場合に限定します。

## Related Pages

- [Quick Start](./quick-start.md)
- [Package Layout](./package-layout.md)
- [Native Backend Spec](../language/native-backend-spec.md)
