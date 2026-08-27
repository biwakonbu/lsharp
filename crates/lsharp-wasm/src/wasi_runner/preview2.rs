//! WASI Preview2 (Component Model) の実行経路。

use super::{
    DEFAULT_STDOUT_CAPTURE_BYTES, ExecutionOutput, StdinMode, configured_engine,
    decode_stdout_bytes, extract_i32_exit, format_component_trap_with_stdout,
};
use wasmtime::component::{Component, Linker as ComponentLinker, ResourceTable};
use wasmtime::*;
use wasmtime_wasi::{WasiCtx, WasiCtxBuilder, WasiView};

/// Preview2 Component Model 用の状態。
struct ComponentState {
    ctx: WasiCtx,
    table: ResourceTable,
}

impl WasiView for ComponentState {
    fn ctx(&mut self) -> &mut WasiCtx {
        &mut self.ctx
    }

    fn table(&mut self) -> &mut ResourceTable {
        &mut self.table
    }
}

/// Component Wasm (.component.wasm) を WASI Preview2 環境で実行し、stdout 出力を返す。
pub fn run_wasm_component(component_bytes: &[u8]) -> Result<String, String> {
    run_wasm_component_with_dir_args_and_stdin(component_bytes, None, &[], "")
}

/// Component Wasm を WASI Preview2 環境で実行 (argv・stdin 付き)。
pub fn run_wasm_component_with_args_and_stdin(
    component_bytes: &[u8],
    args: &[&str],
    stdin_data: &str,
) -> Result<String, String> {
    let output = run_wasm_component_with_dir_args_and_stdin_capture(
        component_bytes,
        None,
        args,
        stdin_data,
    )?;
    if output.exit_code == 0 {
        Ok(output.stdout)
    } else {
        Err(super::format_nonzero_exit_error(
            "Component 実行に失敗",
            output.exit_code,
            &output.stdout,
        ))
    }
}

/// Component Wasm を WASI Preview2 環境で実行 (ファイルシステム・argv・stdin 付き)。
pub fn run_wasm_component_with_dir_args_and_stdin(
    component_bytes: &[u8],
    dir: Option<&std::path::Path>,
    args: &[&str],
    stdin_data: &str,
) -> Result<String, String> {
    let output =
        run_wasm_component_with_dir_args_and_stdin_capture(component_bytes, dir, args, stdin_data)?;
    if output.exit_code == 0 {
        Ok(output.stdout)
    } else {
        Err(super::format_nonzero_exit_error(
            "Component 実行に失敗",
            output.exit_code,
            &output.stdout,
        ))
    }
}

/// Component Wasm を WASI Preview2 環境で実行し、stdout と exit code を返す。
pub fn run_wasm_component_with_dir_args_and_stdin_capture(
    component_bytes: &[u8],
    dir: Option<&std::path::Path>,
    args: &[&str],
    stdin_data: &str,
) -> Result<ExecutionOutput, String> {
    run_wasm_component_capture(component_bytes, dir, args, StdinMode::Memory(stdin_data))
}

/// Component Wasm を実行し、親 stdin を継承した stdout/exit code を返す。
pub fn run_wasm_component_with_dir_and_args_inherit_stdin_capture(
    component_bytes: &[u8],
    dir: Option<&std::path::Path>,
    args: &[&str],
) -> Result<ExecutionOutput, String> {
    run_wasm_component_capture(component_bytes, dir, args, StdinMode::Inherit)
}

fn run_wasm_component_capture(
    component_bytes: &[u8],
    dir: Option<&std::path::Path>,
    args: &[&str],
    stdin_mode: StdinMode<'_>,
) -> Result<ExecutionOutput, String> {
    let engine = configured_engine()?;

    let mut linker = ComponentLinker::<ComponentState>::new(&engine);
    wasmtime_wasi::add_to_linker_sync(&mut linker)
        .map_err(|e| format!("WASI Preview2 リンクに失敗: {e}"))?;

    let stdout = wasmtime_wasi::pipe::MemoryOutputPipe::new(DEFAULT_STDOUT_CAPTURE_BYTES);
    let mut builder = WasiCtxBuilder::new();
    builder.stdout(stdout.clone());
    match stdin_mode {
        StdinMode::Memory(stdin_data) => {
            let stdin = wasmtime_wasi::pipe::MemoryInputPipe::new(stdin_data.as_bytes().to_vec());
            builder.stdin(stdin);
        }
        StdinMode::Inherit => {
            builder.inherit_stdin();
        }
    }
    builder.args(args);
    if let Some(dir_path) = dir {
        builder
            .preopened_dir(
                dir_path,
                ".",
                wasmtime_wasi::DirPerms::all(),
                wasmtime_wasi::FilePerms::all(),
            )
            .map_err(|e| format!("component preopened_dir に失敗: {e}"))?;
    }

    let state = ComponentState {
        ctx: builder.build(),
        table: ResourceTable::new(),
    };
    let mut store = Store::new(&engine, state);

    let component = Component::new(&engine, component_bytes)
        .map_err(|e| format!("Component の読み込みに失敗: {e}"))?;

    let instance = linker
        .instantiate(&mut store, &component)
        .map_err(|e| format!("Component インスタンス化に失敗: {e}"))?;

    let execution = if let Some(run_export) =
        find_component_run_func(&component, &instance, &mut store)
    {
        call_component_run(&mut store, run_export)
    } else {
        return Err(
            "Component に run 関数が見つかりません (wasi:cli/run@0.2.x#run または run export が必要)"
                .to_string(),
        );
    };

    drop(store);
    let bytes = stdout
        .try_into_inner()
        .ok_or_else(|| "stdout の取得に失敗".to_string())?;
    let exit_code = execution.map_err(|error| format_component_trap_with_stdout(error, &bytes))?;
    let stdout = decode_stdout_bytes(&bytes)?;
    Ok(ExecutionOutput { stdout, exit_code })
}

struct ComponentRunExport {
    func: wasmtime::component::Func,
    returns_exit_bool: bool,
}

fn find_component_run_func(
    component: &Component,
    instance: &wasmtime::component::Instance,
    store: &mut Store<ComponentState>,
) -> Option<ComponentRunExport> {
    for export_name in ["wasi:cli/run@0.2.3#run", "wasi:cli/run@0.2.0#run"] {
        if let Some(run_func) = instance.get_func(&mut *store, export_name) {
            return Some(ComponentRunExport {
                func: run_func,
                returns_exit_bool: true,
            });
        }
    }

    if let Some(run_func) = instance.get_func(&mut *store, "run") {
        return Some(ComponentRunExport {
            func: run_func,
            returns_exit_bool: false,
        });
    }

    for interface_name in ["wasi:cli/run@0.2.3", "wasi:cli/run@0.2.0"] {
        if let Some((_, run_instance_index)) = component.export_index(None, interface_name)
            && let Some((_, run_func_index)) =
                component.export_index(Some(&run_instance_index), "run")
            && let Some(run_func) = instance.get_func(&mut *store, run_func_index)
        {
            return Some(ComponentRunExport {
                func: run_func,
                returns_exit_bool: true,
            });
        }
    }

    None
}

fn call_component_run(
    store: &mut Store<ComponentState>,
    run_export: ComponentRunExport,
) -> Result<i32, String> {
    if run_export.returns_exit_bool {
        let mut results = [wasmtime::component::Val::Bool(false)];
        let execution = run_export.func.call(&mut *store, &[], &mut results);
        match execution {
            Ok(()) => decode_component_run_result(&results[0]),
            Err(e) => {
                if let Some(exit) = extract_i32_exit(&e) {
                    Ok(exit)
                } else {
                    Err(format!("Component 実行に失敗: {e}"))
                }
            }
        }
    } else {
        let execution = run_export.func.call(&mut *store, &[], &mut []);
        match execution {
            Ok(()) => Ok(0),
            Err(e) => {
                if let Some(exit) = extract_i32_exit(&e) {
                    Ok(exit)
                } else {
                    Err(format!("Component 実行に失敗: {e}"))
                }
            }
        }
    }
}

fn decode_component_run_result(result: &wasmtime::component::Val) -> Result<i32, String> {
    match result {
        wasmtime::component::Val::Bool(false) => Ok(0),
        wasmtime::component::Val::Bool(true) => Ok(1),
        wasmtime::component::Val::Result(Ok(None)) => Ok(0),
        wasmtime::component::Val::Result(Err(None)) => Ok(1),
        _ => Err("Component run の戻り値型が想定外です".to_string()),
    }
}
