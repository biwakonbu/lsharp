use super::support::*;

#[test]
fn test_e2e_runtime_allocator_reuses_small_blocks_without_linear_scan() {
    let (_stdout, series) = compile_and_capture_runtime_telemetry_series(
        r#"
        (defn churn [n]
          (if (<= n 0)
            0
            (let [value (string-concat "left" "right")]
              (do
                (string-length value)
                (churn (- n 1))))))
        (defn main []
          (do
            (churn 128)
            0))
        "#,
        8,
    );
    let telemetry = series
        .last()
        .copied()
        .expect("size-class allocator telemetry は 1 件以上必要");

    assert!(
        telemetry.gc_freed_count > 0,
        "fixture は再利用可能な小サイズ block を回収すべき: {telemetry:?}"
    );
    assert_eq!(
        telemetry.gc_free_list_scan_steps, 0,
        "小サイズ class の再利用は線形探索を行わないべき: {telemetry:?}"
    );
}

#[test]
fn test_e2e_runtime_allocator_uses_oversize_fallback_scan() {
    let (_stdout, series) = compile_and_capture_runtime_telemetry_series(
        r#"
        (defn churn [n]
          (if (<= n 0)
            0
            (let [value (__alloc 2048)]
              (do
                (- value value)
                (churn (- n 1))))))
        (defn main []
          (do
            (churn 8)
            0))
        "#,
        4,
    );
    let telemetry = series
        .last()
        .copied()
        .expect("oversize allocator telemetry は 1 件以上必要");

    assert!(
        telemetry.gc_freed_count > 0,
        "fixture は oversize block を回収すべき: {telemetry:?}"
    );
    assert!(
        telemetry.gc_free_list_scan_steps > 0,
        "oversize class は free-list を走査して再利用すべき: {telemetry:?}"
    );
}
