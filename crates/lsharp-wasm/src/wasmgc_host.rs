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
