//! WasmGC backend の host-side external boundary。
//!
//! `StringBytes` は WasmGC の concrete packed array reference なので、linear-memory の
//! i64 pointer として扱わず、Wasmtime の GC API で明示的に bytes へ読み出す。

use wasmtime::{Caller, Func, FuncType, HeapType, StorageType, Store, Val, ValType};

/// WasmGC `env.print-string` import 用の host function を作成する。
///
/// `func_type` は emitter が生成した `(ref null (concrete array i8)) -> ()` である必要がある。
/// callback には packed array の bytes が渡され、callback のエラーは Wasm trap として返る。
pub fn create_print_string_import<T, F>(
    store: &mut Store<T>,
    func_type: FuncType,
    sink: F,
) -> Result<Func, String>
where
    F: Fn(&[u8]) -> Result<(), String> + Send + Sync + 'static,
{
    validate_print_string_type(&func_type)?;

    Ok(Func::new(
        store,
        func_type,
        move |mut caller: Caller<'_, T>, params, _results| {
            let anyref = match params.first().and_then(Val::anyref) {
                Some(Some(anyref)) => anyref,
                Some(None) => return Err(wasmtime::Error::msg("print-string に null reference")),
                None => {
                    return Err(wasmtime::Error::msg(
                        "print-string の引数が anyref ではありません",
                    ));
                }
            };
            let array = anyref
                .as_array(&caller)
                .map_err(|error| {
                    wasmtime::Error::msg(format!("print-string の array downcast に失敗: {error}"))
                })?
                .ok_or_else(|| {
                    wasmtime::Error::msg("print-string の引数が array reference ではありません")
                })?;
            let array_type = array.ty(&caller).map_err(|error| {
                wasmtime::Error::msg(format!(
                    "print-string の array type を取得できません: {error}"
                ))
            })?;
            if !matches!(array_type.element_type(), StorageType::I8) {
                return Err(wasmtime::Error::msg(format!(
                    "print-string の array element type が i8 ではありません: {}",
                    array_type.element_type()
                )));
            }

            let len = array.len(&caller).map_err(|error| {
                wasmtime::Error::msg(format!(
                    "print-string の array length を取得できません: {error}"
                ))
            })?;
            let mut bytes = Vec::with_capacity(len as usize);
            for index in 0..len {
                let value = array.get(&mut caller, index).map_err(|error| {
                    wasmtime::Error::msg(format!(
                        "print-string の byte {index} を取得できません: {error}"
                    ))
                })?;
                let Val::I32(value) = value else {
                    return Err(wasmtime::Error::msg(format!(
                        "print-string の byte {index} が i32 ではありません: {value:?}"
                    )));
                };
                let value = u8::try_from(value).map_err(|_| {
                    wasmtime::Error::msg(format!(
                        "print-string の byte {index} が unsigned i8 範囲外です: {value}"
                    ))
                })?;
                bytes.push(value);
            }

            sink(&bytes).map_err(|error| {
                wasmtime::Error::msg(format!("print-string host sink failed: {error}"))
            })
        },
    ))
}

/// Component Model canonical `list<u8>` output import 用の host function を作成する。
///
/// core module 側の signature は `(ptr: i32, len: i32) -> ()` で、exported `memory` を
/// 呼び出し中だけ借用する。範囲外、負の値、write sink のエラーはすべて trap へ変換する。
pub fn create_component_output_import<T, F>(
    store: &mut Store<T>,
    func_type: FuncType,
    sink: F,
) -> Result<Func, String>
where
    F: Fn(&[u8]) -> Result<(), String> + Send + Sync + 'static,
{
    validate_component_output_type(&func_type)?;

    Ok(Func::new(
        store,
        func_type,
        move |mut caller: Caller<'_, T>, params, _results| {
            let Some(Val::I32(pointer)) = params.first() else {
                return Err(wasmtime::Error::msg(
                    "component output write の pointer が i32 ではありません",
                ));
            };
            let Some(Val::I32(length)) = params.get(1) else {
                return Err(wasmtime::Error::msg(
                    "component output write の length が i32 ではありません",
                ));
            };
            let pointer = usize::try_from(*pointer).map_err(|_| {
                wasmtime::Error::msg(format!(
                    "component output write の pointer が負値です: {pointer}"
                ))
            })?;
            let length = usize::try_from(*length).map_err(|_| {
                wasmtime::Error::msg(format!(
                    "component output write の length が負値です: {length}"
                ))
            })?;
            let Some(memory) = caller
                .get_export("memory")
                .and_then(wasmtime::Extern::into_memory)
            else {
                return Err(wasmtime::Error::msg(
                    "component output write に exported memory がありません",
                ));
            };
            let memory_size = memory.data_size(&caller);
            let end = pointer.checked_add(length).ok_or_else(|| {
                wasmtime::Error::msg(
                    "component output write の pointer+length が overflow しました",
                )
            })?;
            if end > memory_size {
                return Err(wasmtime::Error::msg(format!(
                    "component output write の範囲が linear memory 外です: ptr={pointer}, len={length}, memory={memory_size}"
                )));
            }
            let mut bytes = vec![0_u8; length];
            memory.read(&caller, pointer, &mut bytes).map_err(|error| {
                wasmtime::Error::msg(format!(
                    "component output write の linear memory 読み出しに失敗: {error}"
                ))
            })?;
            sink(&bytes).map_err(|error| {
                wasmtime::Error::msg(format!("component output host sink failed: {error}"))
            })
        },
    ))
}

fn validate_print_string_type(func_type: &FuncType) -> Result<(), String> {
    let params = func_type.params().collect::<Vec<_>>();
    let results = func_type.results().collect::<Vec<_>>();
    if params.len() != 1 || !results.is_empty() {
        return Err(format!(
            "print-string import の signature が不正です: params={}, results={}",
            params.len(),
            results.len()
        ));
    }

    let ValType::Ref(reference) = &params[0] else {
        return Err(format!(
            "print-string import の parameter が reference ではありません: {}",
            params[0]
        ));
    };
    if !reference.is_nullable() {
        return Err(
            "print-string import の StringBytes reference は nullable である必要があります".into(),
        );
    }
    let HeapType::ConcreteArray(array_type) = reference.heap_type() else {
        return Err(format!(
            "print-string import の parameter が concrete array ではありません: {}",
            reference.heap_type()
        ));
    };
    if !matches!(array_type.element_type(), StorageType::I8) {
        return Err(format!(
            "print-string import の array element type が i8 ではありません: {}",
            array_type.element_type()
        ));
    }
    Ok(())
}

fn validate_component_output_type(func_type: &FuncType) -> Result<(), String> {
    let params = func_type.params().collect::<Vec<_>>();
    let results = func_type.results().collect::<Vec<_>>();
    if params.len() != 2
        || !matches!(params.first(), Some(ValType::I32))
        || !matches!(params.get(1), Some(ValType::I32))
        || !results.is_empty()
    {
        return Err(format!(
            "component output write import の signature が不正です: params={params:?}, results={results:?}"
        ));
    }
    Ok(())
}
