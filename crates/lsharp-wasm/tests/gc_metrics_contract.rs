use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::atomic::{AtomicUsize, Ordering};

static GC_METRICS_FIXTURE_COUNTER: AtomicUsize = AtomicUsize::new(0);

fn project_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..")
}

#[cfg(unix)]
fn gc_metrics_fixture_dir(label: &str) -> PathBuf {
    let root = project_root().join("target/ci/e2e-fixtures");
    let dir = root.join(format!(
        "lsharp-{label}-{}-{}",
        std::process::id(),
        GC_METRICS_FIXTURE_COUNTER.fetch_add(1, Ordering::Relaxed)
    ));
    std::fs::create_dir_all(&dir).expect("gc metrics fixture directory の作成に失敗");
    dir
}

fn write_gc_metrics_fixture(path: &Path, payload: serde_json::Value) {
    std::fs::write(
        path,
        serde_json::to_string_pretty(&payload).expect("fixture payload の JSON 化に失敗"),
    )
    .expect("gc metrics fixture の書き込みに失敗");
}

#[cfg(unix)]
#[test]
fn test_gc_metrics_script_accepts_fixture_payload_with_pass_s14() {
    let project_root = project_root();
    let script = project_root.join("scripts/ci/collect-gc-metrics.sh");
    let fixture_dir = gc_metrics_fixture_dir("gc-metrics-pass");
    let artifact_path = fixture_dir.join("summary.json");
    write_gc_metrics_fixture(
        &artifact_path,
        serde_json::json!({
            "allocator_mode": "collector",
            "ci_level": "nightly",
            "gate_status": "accepted",
            "s14_status": "pass",
            "s15_status": "blocked",
            "s16_status": "blocked",
            "heap_bytes_series": [10, 20, 30, 40, 50, 60, 70, 80, 90, 90],
            "proxy_workloads": {
                "compile_run_light_loop": {
                    "status": "pass",
                    "iterations": 48,
                    "last_stdout": "1"
                },
                "repl_soak_50_eval": {
                    "status": "pass",
                    "iterations": 50,
                    "eval_count": 50
                },
                "repl_stateful_long_session": {
                    "status": "pass",
                    "iterations": 200,
                    "eval_count": 200,
                    "total_input_bytes": 4000,
                    "last_type_tag": 100
                },
                "repl_stateful_single_session": {
                    "status": "pass",
                    "iterations": 50,
                    "eval_count": 50,
                    "total_input_bytes": 1125,
                    "last_type_tag": 100
                },
                "lsp_actual_stdio_repeated_sequence": {
                    "status": "pass",
                    "iterations": 12,
                    "response_frames": 61
                }
            },
            "peak_alloc_bytes": 90,
            "total_alloc_count": 10,
            "live_alloc_count": 10,
            "max_single_alloc": 16,
            "alloc_span": 80,
            "leak_growing_count": 9,
            "leak_total": 10,
            "leak_suspect": 0
        }),
    );

    let output = Command::new("bash")
        .arg(&script)
        .env("PATH", "/usr/bin:/bin:/usr/sbin:/sbin:/opt/homebrew/bin")
        .env("LSHARP_GC_METRICS_INPUT", &artifact_path)
        .current_dir(&project_root)
        .output()
        .expect("collect-gc-metrics.sh の実行に失敗");

    std::fs::remove_dir_all(&fixture_dir).ok();

    assert!(
        output.status.success(),
        "collect-gc-metrics.sh は validate-only fixture を受理するべき: status={:?}, stdout={}, stderr={}",
        output.status.code(),
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("s14_status: pass")
            && stdout.contains("s15_status: blocked")
            && stdout.contains("s16_status: blocked"),
        "collect-gc-metrics.sh は fixture gate 状態を出力するべき: {}",
        stdout
    );
    assert!(
        stdout.contains(&format!("gc-metrics-artifact:{}", artifact_path.display())),
        "collect-gc-metrics.sh は validate した artifact path を出力するべき: {}",
        stdout
    );
}

#[cfg(unix)]
#[test]
fn test_gc_metrics_script_rejects_fixture_payload_with_mismatched_s14() {
    let project_root = project_root();
    let script = project_root.join("scripts/ci/collect-gc-metrics.sh");
    let fixture_dir = gc_metrics_fixture_dir("gc-metrics-mismatch");
    let artifact_path = fixture_dir.join("summary.json");
    write_gc_metrics_fixture(
        &artifact_path,
        serde_json::json!({
            "allocator_mode": "collector",
            "ci_level": "nightly",
            "gate_status": "accepted",
            "s14_status": "pass",
            "s15_status": "blocked",
            "s16_status": "blocked",
            "heap_bytes_series": [10, 20, 30, 40, 50, 60, 70, 80, 90, 100],
            "proxy_workloads": {
                "compile_run_light_loop": {
                    "status": "pass",
                    "iterations": 48,
                    "last_stdout": "1"
                },
                "repl_soak_50_eval": {
                    "status": "pass",
                    "iterations": 50,
                    "eval_count": 50
                },
                "repl_stateful_long_session": {
                    "status": "pass",
                    "iterations": 200,
                    "eval_count": 200,
                    "total_input_bytes": 4000,
                    "last_type_tag": 100
                },
                "repl_stateful_single_session": {
                    "status": "pass",
                    "iterations": 50,
                    "eval_count": 50,
                    "total_input_bytes": 1125,
                    "last_type_tag": 100
                },
                "lsp_actual_stdio_repeated_sequence": {
                    "status": "pass",
                    "iterations": 12,
                    "response_frames": 61
                }
            },
            "peak_alloc_bytes": 100,
            "total_alloc_count": 10,
            "live_alloc_count": 10,
            "max_single_alloc": 16,
            "alloc_span": 90,
            "leak_growing_count": 10,
            "leak_total": 10,
            "leak_suspect": 1
        }),
    );

    let output = Command::new("bash")
        .arg(&script)
        .env("PATH", "/usr/bin:/bin:/usr/sbin:/sbin:/opt/homebrew/bin")
        .env("LSHARP_GC_METRICS_INPUT", &artifact_path)
        .current_dir(&project_root)
        .output()
        .expect("collect-gc-metrics.sh の実行に失敗");

    std::fs::remove_dir_all(&fixture_dir).ok();

    assert!(
        !output.status.success(),
        "collect-gc-metrics.sh は s14_status mismatch fixture を reject するべき: stdout={}, stderr={}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("AR-04: s14_status is 'pass' but computed 'fail'"),
        "collect-gc-metrics.sh は mismatch 理由を報告するべき: {}",
        stderr
    );
}

#[cfg(unix)]
#[test]
fn test_gc_metrics_script_rejects_fixture_payload_without_repl_stateful_long_session() {
    let project_root = project_root();
    let script = project_root.join("scripts/ci/collect-gc-metrics.sh");
    let fixture_dir = gc_metrics_fixture_dir("gc-metrics-missing-long-session");
    let artifact_path = fixture_dir.join("summary.json");
    write_gc_metrics_fixture(
        &artifact_path,
        serde_json::json!({
            "allocator_mode": "collector",
            "ci_level": "nightly",
            "gate_status": "accepted",
            "s14_status": "pass",
            "s15_status": "blocked",
            "s16_status": "blocked",
            "heap_bytes_series": [10, 20, 30, 40, 50, 60, 70, 80, 90, 90],
            "proxy_workloads": {
                "compile_run_light_loop": {
                    "status": "pass",
                    "iterations": 48,
                    "last_stdout": "1"
                },
                "repl_soak_50_eval": {
                    "status": "pass",
                    "iterations": 50,
                    "eval_count": 50
                },
                "repl_stateful_single_session": {
                    "status": "pass",
                    "iterations": 50,
                    "eval_count": 50,
                    "total_input_bytes": 1125,
                    "last_type_tag": 100
                },
                "lsp_actual_stdio_repeated_sequence": {
                    "status": "pass",
                    "iterations": 12,
                    "response_frames": 61
                }
            },
            "peak_alloc_bytes": 90,
            "total_alloc_count": 10,
            "live_alloc_count": 10,
            "max_single_alloc": 16,
            "alloc_span": 80,
            "leak_growing_count": 9,
            "leak_total": 10,
            "leak_suspect": 0
        }),
    );

    let output = Command::new("bash")
        .arg(&script)
        .env("PATH", "/usr/bin:/bin:/usr/sbin:/sbin:/opt/homebrew/bin")
        .env("LSHARP_GC_METRICS_INPUT", &artifact_path)
        .current_dir(&project_root)
        .output()
        .expect("collect-gc-metrics.sh の実行に失敗");

    std::fs::remove_dir_all(&fixture_dir).ok();

    assert!(
        !output.status.success(),
        "collect-gc-metrics.sh は long-session proxy workload 欠落 fixture を reject するべき: stdout={}, stderr={}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("AR-04: missing proxy_workloads entries: repl_stateful_long_session"),
        "collect-gc-metrics.sh は long-session proxy workload 欠落を報告するべき: {}",
        stderr
    );
}

#[cfg(unix)]
#[test]
fn test_gc_metrics_script_rejects_fixture_payload_without_proxy_workloads() {
    let project_root = project_root();
    let script = project_root.join("scripts/ci/collect-gc-metrics.sh");
    let fixture_dir = gc_metrics_fixture_dir("gc-metrics-missing-proxy-workloads");
    let artifact_path = fixture_dir.join("summary.json");
    write_gc_metrics_fixture(
        &artifact_path,
        serde_json::json!({
            "allocator_mode": "collector",
            "ci_level": "nightly",
            "gate_status": "accepted",
            "s14_status": "pass",
            "s15_status": "blocked",
            "s16_status": "blocked",
            "heap_bytes_series": [10, 20, 30, 40, 50, 60, 70, 80, 90, 90],
            "peak_alloc_bytes": 90,
            "total_alloc_count": 10,
            "live_alloc_count": 10,
            "max_single_alloc": 16,
            "alloc_span": 80,
            "leak_growing_count": 9,
            "leak_total": 10,
            "leak_suspect": 0
        }),
    );

    let output = Command::new("bash")
        .arg(&script)
        .env("PATH", "/usr/bin:/bin:/usr/sbin:/sbin:/opt/homebrew/bin")
        .env("LSHARP_GC_METRICS_INPUT", &artifact_path)
        .current_dir(&project_root)
        .output()
        .expect("collect-gc-metrics.sh の実行に失敗");

    std::fs::remove_dir_all(&fixture_dir).ok();

    assert!(
        !output.status.success(),
        "collect-gc-metrics.sh は proxy_workloads 欠落 fixture を reject するべき: stdout={}, stderr={}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("AR-04: missing GC metrics keys: proxy_workloads"),
        "collect-gc-metrics.sh は proxy_workloads 欠落を報告するべき: {}",
        stderr
    );
}
