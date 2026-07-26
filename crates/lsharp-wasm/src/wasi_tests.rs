#[cfg(test)]
mod tests {
    use super::*;
    use lsharp_ir::lower::Lower;
    use lsharp_types::infer::Infer;

    fn compile_wasi(source: &str) -> Vec<u8> {
        let program = lsharp_syntax::parse(source).unwrap();
        let mut infer = Infer::new();
        let type_results = infer.infer_program(&program).unwrap();
        let mut lower = Lower::new();
        let module = lower.lower_program(&program, &type_results).unwrap();
        emit_wasm_wasi(&module).unwrap()
    }

    fn compile_wasi_p2(source: &str) -> Vec<u8> {
        let program = lsharp_syntax::parse(source).unwrap();
        let mut infer = Infer::new();
        let type_results = infer.infer_program(&program).unwrap();
        let mut lower = Lower::new();
        let module = lower.lower_program(&program, &type_results).unwrap();
        emit_wasm_wasi_p2(&module).unwrap()
    }

    fn run_wasi(wasm_bytes: &[u8]) -> String {
        use wasmtime::*;
        use wasmtime_wasi::{WasiCtxBuilder, preview1::WasiP1Ctx};

        let engine = Engine::default();
        let mut linker = Linker::<WasiP1Ctx>::new(&engine);
        wasmtime_wasi::preview1::add_to_linker_sync(&mut linker, |t| t).unwrap();

        let stdout = wasmtime_wasi::pipe::MemoryOutputPipe::new(1024);
        let wasi = WasiCtxBuilder::new().stdout(stdout.clone()).build_p1();

        let mut store = Store::new(&engine, wasi);
        let module = wasmtime::Module::new(&engine, wasm_bytes).unwrap();
        let instance = linker.instantiate(&mut store, &module).unwrap();

        let start = instance
            .get_typed_func::<(), ()>(&mut store, "_start")
            .unwrap();
        start.call(&mut store, ()).unwrap();

        drop(store);
        let bytes = stdout.try_into_inner().unwrap();
        String::from_utf8(bytes.to_vec()).unwrap()
    }

    fn run_wasi_with_root_slot_failure_ledger(wasm_bytes: &[u8]) -> (String, i32, i32, i32) {
        use wasmtime::*;
        use wasmtime_wasi::{WasiCtxBuilder, preview1::WasiP1Ctx};

        let engine = Engine::default();
        let mut linker = Linker::<WasiP1Ctx>::new(&engine);
        wasmtime_wasi::preview1::add_to_linker_sync(&mut linker, |t| t).unwrap();
        let stdout = wasmtime_wasi::pipe::MemoryOutputPipe::new(1024);
        let wasi = WasiCtxBuilder::new().stdout(stdout).build_p1();
        let mut store = Store::new(&engine, wasi);
        let module = wasmtime::Module::new(&engine, wasm_bytes).unwrap();
        let instance = linker.instantiate(&mut store, &module).unwrap();
        let start = instance
            .get_typed_func::<(), ()>(&mut store, "_start")
            .unwrap();
        let error = start.call(&mut store, ()).unwrap_err();
        let failure_slot = instance
            .get_global(&mut store, "__lsharp_root_slot_failure_slot")
            .unwrap()
            .get(&mut store)
            .i32()
            .unwrap();
        let failure_top = instance
            .get_global(&mut store, "__lsharp_root_slot_failure_top")
            .unwrap()
            .get(&mut store)
            .i32()
            .unwrap();
        let failure_count = instance
            .get_global(&mut store, "__lsharp_root_slot_failure_count")
            .unwrap()
            .get(&mut store)
            .i32()
            .unwrap();
        (
            format!("{error:#}"),
            failure_slot,
            failure_top,
            failure_count,
        )
    }

    include!("wasi_tests/core.rs");
    include!("wasi_tests/preview2.rs");
}
