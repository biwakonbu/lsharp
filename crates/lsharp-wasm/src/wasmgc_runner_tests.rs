use super::*;

use wasmtime::component::Val as ComponentVal;

#[test]
fn component_output_module_decodes_cli_exit_results() {
    assert_eq!(
        component_output::decode_wasmgc_component_run_result(Some(&ComponentVal::Bool(false)))
            .expect("false result は正常終了を表す"),
        0
    );
    assert_eq!(
        component_output::decode_wasmgc_component_run_result(Some(&ComponentVal::Bool(true)))
            .expect("true result は失敗終了を表す"),
        1
    );
    assert_eq!(
        component_output::decode_wasmgc_component_run_result(Some(&ComponentVal::Result(
            Ok(None,)
        )))
        .expect("ok result は正常終了を表す"),
        0
    );
    assert!(
        component_output::decode_wasmgc_component_run_result(Some(&ComponentVal::S64(2))).is_err()
    );
}
