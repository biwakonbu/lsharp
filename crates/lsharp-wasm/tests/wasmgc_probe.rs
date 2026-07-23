use wasmtime::{Config, Engine, Instance, Module, Store};

#[test]
fn wasm_gc_struct_probe_executes_with_wasmtime_29() {
    let bytes = lsharp_wasm::wasmgc::emit_minimal_struct_probe();

    let mut config = Config::new();
    config.wasm_gc(true);
    let engine = Engine::new(&config).expect("WasmGC を有効化した engine を作成できる");
    let module = Module::new(&engine, bytes).expect("WasmGC struct module を検証できる");
    let mut store = Store::new(&engine, ());
    let instance =
        Instance::new(&mut store, &module, &[]).expect("WasmGC module を instantiate できる");
    let read_field = instance
        .get_typed_func::<(), i64>(&mut store, "read-field")
        .expect("read-field export が存在する");

    let value = read_field
        .call(&mut store, ())
        .expect("struct.new と struct.get が実行できる");
    assert_eq!(value, 42);
}

#[test]
fn wasm_gc_struct_probe_is_rejected_without_gc_feature() {
    let bytes = lsharp_wasm::wasmgc::emit_minimal_struct_probe();
    let engine = Engine::default();

    assert!(Module::new(&engine, bytes).is_err());
}
