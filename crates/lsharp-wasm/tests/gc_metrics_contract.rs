use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::atomic::{AtomicUsize, Ordering};

static GC_METRICS_FIXTURE_COUNTER: AtomicUsize = AtomicUsize::new(0);

#[cfg(unix)]
const S14_BLOCKED_REASON: &str = "collector_heap_series_missing";
#[cfg(unix)]
const S15_BLOCKED_REASON: &str = "collector_fixed_point_artifact_missing";
#[cfg(unix)]
const S16_BLOCKED_REASON: &str = "collector_workload_artifact_missing";

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
fn read_gc_metrics_fixture(path: &Path, context: &str) -> serde_json::Value {
    serde_json::from_str(
        &std::fs::read_to_string(path)
            .unwrap_or_else(|e| panic!("{context} の再読込に失敗 {}: {}", path.display(), e)),
    )
    .unwrap_or_else(|e| panic!("{context} の JSON parse に失敗 {}: {}", path.display(), e))
}

#[cfg(unix)]
fn default_proxy_workloads() -> serde_json::Value {
    serde_json::json!({
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
    })
}

#[cfg(unix)]
fn base_collector_payload() -> serde_json::Value {
    serde_json::json!({
        "allocator_mode": "collector",
        "ci_level": "nightly",
        "gate_status": "accepted",
        "s14_status": "pass",
        "s14_reason": null,
        "s15_status": "blocked",
        "s16_status": "blocked",
        "s15_reason": S15_BLOCKED_REASON,
        "s16_reason": S16_BLOCKED_REASON,
        "s15_proof": null,
        "s16_proof": null,
        "heap_bytes_series": [10, 20, 30, 40, 50, 60, 70, 80, 90, 90],
        "proxy_workloads": default_proxy_workloads(),
        "peak_alloc_bytes": 90,
        "total_alloc_count": 10,
        "live_alloc_count": 10,
        "max_single_alloc": 16,
        "alloc_span": 80,
        "leak_growing_count": 9,
        "leak_total": 10,
        "leak_suspect": 0
    })
}

#[cfg(unix)]
fn run_gc_metrics_script_with_fixture(
    label: &str,
    payload: serde_json::Value,
) -> std::process::Output {
    let project_root = project_root();
    let script = project_root.join("scripts/ci/collect-gc-metrics.sh");
    let fixture_dir = gc_metrics_fixture_dir(label);
    let artifact_path = fixture_dir.join("summary.json");
    write_gc_metrics_fixture(&artifact_path, payload);

    let output = Command::new("bash")
        .arg(&script)
        .env("PATH", "/usr/bin:/bin:/usr/sbin:/sbin:/opt/homebrew/bin")
        .env("LSHARP_GC_METRICS_INPUT", &artifact_path)
        .current_dir(&project_root)
        .output()
        .expect("collect-gc-metrics.sh の実行に失敗");

    std::fs::remove_dir_all(&fixture_dir).ok();
    output
}

#[cfg(unix)]
fn run_gc_metrics_script_with_fixture_and_proof_bundle(
    label: &str,
    payload: serde_json::Value,
    proof_bundle: serde_json::Value,
) -> (std::process::Output, serde_json::Value, serde_json::Value) {
    let project_root = project_root();
    let script = project_root.join("scripts/ci/collect-gc-metrics.sh");
    let fixture_dir = gc_metrics_fixture_dir(label);
    let artifact_path = fixture_dir.join("summary.json");
    let proof_bundle_path = fixture_dir.join("collector-proof.json");
    write_gc_metrics_fixture(&artifact_path, payload);
    write_gc_metrics_fixture(&proof_bundle_path, proof_bundle);

    let output = Command::new("bash")
        .arg(&script)
        .env("PATH", "/usr/bin:/bin:/usr/sbin:/sbin:/opt/homebrew/bin")
        .env("LSHARP_GC_METRICS_INPUT", &artifact_path)
        .env("LSHARP_GC_PROOF_BUNDLE_INPUT", &proof_bundle_path)
        .current_dir(&project_root)
        .output()
        .expect("collect-gc-metrics.sh の実行に失敗");

    let normalized_payload = read_gc_metrics_fixture(&artifact_path, "merged gc metrics fixture");
    let normalized_sidecar =
        read_gc_metrics_fixture(&proof_bundle_path, "merged collector proof sidecar");

    std::fs::remove_dir_all(&fixture_dir).ok();
    (output, normalized_payload, normalized_sidecar)
}

#[cfg(unix)]
fn run_gc_metrics_script_with_fixture_and_adjacent_proof_bundle(
    label: &str,
    payload: serde_json::Value,
    proof_bundle: serde_json::Value,
) -> (std::process::Output, serde_json::Value, serde_json::Value) {
    let project_root = project_root();
    let script = project_root.join("scripts/ci/collect-gc-metrics.sh");
    let fixture_dir = gc_metrics_fixture_dir(label);
    let artifact_path = fixture_dir.join("summary.json");
    let proof_bundle_path = fixture_dir.join("collector-proof.json");
    write_gc_metrics_fixture(&artifact_path, payload);
    write_gc_metrics_fixture(&proof_bundle_path, proof_bundle);

    let output = Command::new("bash")
        .arg(&script)
        .env("PATH", "/usr/bin:/bin:/usr/sbin:/sbin:/opt/homebrew/bin")
        .env("LSHARP_GC_METRICS_INPUT", &artifact_path)
        .current_dir(&project_root)
        .output()
        .expect("collect-gc-metrics.sh の実行に失敗");

    let normalized_payload =
        read_gc_metrics_fixture(&artifact_path, "auto-merged gc metrics fixture");
    let normalized_sidecar =
        read_gc_metrics_fixture(&proof_bundle_path, "auto-merged collector proof sidecar");

    std::fs::remove_dir_all(&fixture_dir).ok();
    (output, normalized_payload, normalized_sidecar)
}

#[cfg(unix)]
fn run_gc_metrics_script_with_fixture_and_capture_artifacts(
    label: &str,
    payload: serde_json::Value,
) -> (std::process::Output, serde_json::Value, serde_json::Value) {
    let project_root = project_root();
    let script = project_root.join("scripts/ci/collect-gc-metrics.sh");
    let fixture_dir = gc_metrics_fixture_dir(label);
    let artifact_path = fixture_dir.join("summary.json");
    let proof_sidecar_path = fixture_dir.join("collector-proof.json");
    write_gc_metrics_fixture(&artifact_path, payload);

    let output = Command::new("bash")
        .arg(&script)
        .env("PATH", "/usr/bin:/bin:/usr/sbin:/sbin:/opt/homebrew/bin")
        .env("LSHARP_GC_METRICS_INPUT", &artifact_path)
        .current_dir(&project_root)
        .output()
        .expect("collect-gc-metrics.sh の実行に失敗");

    let normalized_payload = read_gc_metrics_fixture(&artifact_path, "captured gc metrics fixture");
    let normalized_sidecar =
        read_gc_metrics_fixture(&proof_sidecar_path, "captured collector proof sidecar");

    std::fs::remove_dir_all(&fixture_dir).ok();
    (output, normalized_payload, normalized_sidecar)
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
            "s14_reason": null,
            "s15_status": "blocked",
            "s16_status": "blocked",
            "s15_reason": S15_BLOCKED_REASON,
            "s16_reason": S16_BLOCKED_REASON,
            "s15_proof": null,
            "s16_proof": null,
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
            "s14_reason": null,
            "s15_status": "blocked",
            "s16_status": "blocked",
            "s15_reason": S15_BLOCKED_REASON,
            "s16_reason": S16_BLOCKED_REASON,
            "s15_proof": null,
            "s16_proof": null,
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
            "s14_reason": null,
            "s15_status": "blocked",
            "s16_status": "blocked",
            "s15_reason": S15_BLOCKED_REASON,
            "s16_reason": S16_BLOCKED_REASON,
            "s15_proof": null,
            "s16_proof": null,
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
            "s14_reason": null,
            "s15_status": "blocked",
            "s16_status": "blocked",
            "s15_reason": S15_BLOCKED_REASON,
            "s16_reason": S16_BLOCKED_REASON,
            "s15_proof": null,
            "s16_proof": null,
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

#[cfg(unix)]
#[test]
fn test_gc_metrics_script_rejects_fixture_payload_without_s15_s16_proof_keys() {
    let project_root = project_root();
    let script = project_root.join("scripts/ci/collect-gc-metrics.sh");
    let fixture_dir = gc_metrics_fixture_dir("gc-metrics-missing-proof-keys");
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
        !output.status.success(),
        "collect-gc-metrics.sh は s15/s16 proof key 欠落 fixture を reject するべき: stdout={}, stderr={}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("AR-04: missing GC metrics keys: s14_reason, s15_reason, s16_reason, s15_proof, s16_proof"),
        "collect-gc-metrics.sh は proof key 欠落を報告するべき: {}",
        stderr
    );
}

#[cfg(unix)]
#[test]
fn test_gc_metrics_script_rejects_fixture_payload_with_invalid_blocked_s14_reason() {
    let mut payload = base_collector_payload();
    payload["s14_status"] = serde_json::json!("blocked");
    payload["s14_reason"] = serde_json::json!(S15_BLOCKED_REASON);
    payload["heap_bytes_series"] = serde_json::json!([]);

    let output = run_gc_metrics_script_with_fixture("gc-metrics-s14-invalid-reason", payload);

    assert!(
        !output.status.success(),
        "collect-gc-metrics.sh は invalid blocked s14 reason を reject するべき: stdout={}, stderr={}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains(&format!(
            "AR-04: s14_reason must be one of: {S14_BLOCKED_REASON} when s14_status is 'blocked'"
        )),
        "collect-gc-metrics.sh は invalid blocked s14 reason を報告するべき: {}",
        stderr
    );
}

#[cfg(unix)]
#[test]
fn test_gc_metrics_script_rejects_fixture_payload_with_invalid_blocked_s15_reason() {
    let mut payload = base_collector_payload();
    payload["s15_reason"] = serde_json::json!(S16_BLOCKED_REASON);

    let output = run_gc_metrics_script_with_fixture("gc-metrics-s15-invalid-reason", payload);

    assert!(
        !output.status.success(),
        "collect-gc-metrics.sh は invalid blocked s15 reason を reject するべき: stdout={}, stderr={}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains(
            "AR-04: s15_reason must be one of: collector_fixed_point_artifact_missing when s15_status is 'blocked'"
        ),
        "collect-gc-metrics.sh は invalid blocked reason を報告するべき: {}",
        stderr
    );
}

#[cfg(unix)]
#[test]
fn test_gc_metrics_script_rejects_fixture_payload_with_noncollector_s15_gc_mode() {
    let mut payload = base_collector_payload();
    payload["s15_status"] = serde_json::json!("pass");
    payload["s15_reason"] = serde_json::Value::Null;
    payload["s15_proof"] = serde_json::json!({
        "gc_mode": "none",
        "stage_pair": ["stage1", "stage2"],
        "bytes_identical": true,
        "exports_identical": true,
        "data_sections_identical": true,
        "diagnostics_identical": true
    });

    let output = run_gc_metrics_script_with_fixture("gc-metrics-s15-noncollector-mode", payload);

    assert!(
        !output.status.success(),
        "collect-gc-metrics.sh は non-collector gc_mode の S15 proof を reject するべき: stdout={}, stderr={}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("AR-04: s15_proof.gc_mode must be one of: mark-sweep, generational"),
        "collect-gc-metrics.sh は collector gc_mode 制約を報告するべき: {}",
        stderr
    );
}

#[cfg(unix)]
#[test]
fn test_gc_metrics_script_rejects_fixture_payload_with_incomplete_s16_workload_set() {
    let mut payload = base_collector_payload();
    payload["s16_status"] = serde_json::json!("pass");
    payload["s16_reason"] = serde_json::Value::Null;
    payload["s16_proof"] = serde_json::json!({
        "gc_mode": "mark-sweep",
        "completed_workloads": ["compile_run_light_loop", "repl_soak_50_eval"],
        "all_workloads_completed": true,
        "sigsegv_count": 0,
        "trap_count": 0,
        "unreachable_count": 0,
        "dangling_pointer_count": 0
    });

    let output =
        run_gc_metrics_script_with_fixture("gc-metrics-s16-incomplete-workload-set", payload);

    assert!(
        !output.status.success(),
        "collect-gc-metrics.sh は incomplete S16 workload set を reject するべき: stdout={}, stderr={}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains(
            "AR-04: s16_proof.completed_workloads must equal the required workload set when s16_status is 'pass'"
        ),
        "collect-gc-metrics.sh は required workload set 制約を報告するべき: {}",
        stderr
    );
}

#[cfg(unix)]
#[test]
fn test_gc_metrics_script_accepts_fixture_payload_with_valid_s15_s16_proofs() {
    let mut payload = base_collector_payload();
    payload["s15_status"] = serde_json::json!("pass");
    payload["s15_reason"] = serde_json::Value::Null;
    payload["s15_proof"] = serde_json::json!({
        "gc_mode": "mark-sweep",
        "stage_pair": ["stage1", "stage2"],
        "bytes_identical": true,
        "exports_identical": true,
        "data_sections_identical": true,
        "diagnostics_identical": true
    });
    payload["s16_status"] = serde_json::json!("pass");
    payload["s16_reason"] = serde_json::Value::Null;
    payload["s16_proof"] = serde_json::json!({
        "gc_mode": "mark-sweep",
        "completed_workloads": [
            "compile_run_light_loop",
            "repl_soak_50_eval",
            "repl_stateful_long_session",
            "repl_stateful_single_session",
            "lsp_actual_stdio_repeated_sequence"
        ],
        "all_workloads_completed": true,
        "sigsegv_count": 0,
        "trap_count": 0,
        "unreachable_count": 0,
        "dangling_pointer_count": 0
    });
    assert_eq!(payload["s14_reason"], serde_json::Value::Null);

    let output = run_gc_metrics_script_with_fixture("gc-metrics-valid-s15-s16-proofs", payload);

    assert!(
        output.status.success(),
        "collect-gc-metrics.sh は valid S15/S16 proof fixture を受理するべき: status={:?}, stdout={}, stderr={}",
        output.status.code(),
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("s14_status: pass")
            && stdout.contains("s15_status: pass")
            && stdout.contains("s16_status: pass"),
        "collect-gc-metrics.sh は valid fixture の gate 状態を出力するべき: {}",
        stdout
    );
}

#[cfg(unix)]
#[test]
fn test_gc_metrics_script_merges_external_collector_proof_bundle_into_summary() {
    let payload = base_collector_payload();
    let proof_bundle = serde_json::json!({
        "s15_status": "pass",
        "s15_reason": null,
        "s15_proof": {
            "gc_mode": "mark-sweep",
            "stage_pair": ["stage2", "stage3"],
            "bytes_identical": true,
            "exports_identical": true,
            "data_sections_identical": true,
            "diagnostics_identical": true
        },
        "s16_status": "pass",
        "s16_reason": null,
        "s16_proof": {
            "gc_mode": "mark-sweep",
            "completed_workloads": [
                "compile_run_light_loop",
                "repl_soak_50_eval",
                "repl_stateful_long_session",
                "repl_stateful_single_session",
                "lsp_actual_stdio_repeated_sequence"
            ],
            "all_workloads_completed": true,
            "sigsegv_count": 0,
            "trap_count": 0,
            "unreachable_count": 0,
            "dangling_pointer_count": 0
        }
    });

    let (output, normalized_payload, normalized_sidecar) =
        run_gc_metrics_script_with_fixture_and_proof_bundle(
            "gc-metrics-proof-bundle-merge",
            payload,
            proof_bundle,
        );

    assert!(
        output.status.success(),
        "collect-gc-metrics.sh は external collector proof bundle を merge して受理するべき: status={:?}, stdout={}, stderr={}",
        output.status.code(),
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("s15_status: pass") && stdout.contains("s16_status: pass"),
        "collect-gc-metrics.sh は merge 済み gate 状態を出力するべき: {}",
        stdout
    );
    assert_eq!(normalized_payload["s15_status"], "pass");
    assert_eq!(normalized_payload["s16_status"], "pass");
    assert_eq!(normalized_payload["s14_reason"], serde_json::Value::Null);
    assert_eq!(normalized_payload["s15_reason"], serde_json::Value::Null);
    assert_eq!(normalized_payload["s16_reason"], serde_json::Value::Null);
    assert_eq!(
        normalized_payload["s15_proof"]["stage_pair"],
        serde_json::json!(["stage2", "stage3"])
    );
    assert_eq!(
        normalized_payload["s16_proof"]["completed_workloads"],
        serde_json::json!([
            "compile_run_light_loop",
            "repl_soak_50_eval",
            "repl_stateful_long_session",
            "repl_stateful_single_session",
            "lsp_actual_stdio_repeated_sequence"
        ])
    );
    assert_eq!(normalized_sidecar["s15_status"], "pass");
    assert_eq!(normalized_sidecar["s16_status"], "pass");
    assert_eq!(normalized_sidecar["s15_reason"], serde_json::Value::Null);
    assert_eq!(normalized_sidecar["s16_reason"], serde_json::Value::Null);
    assert_eq!(normalized_sidecar["s16_proof"]["gc_mode"], "mark-sweep");
}

#[cfg(unix)]
#[test]
fn test_gc_metrics_script_rejects_invalid_external_collector_proof_bundle() {
    let payload = base_collector_payload();
    let proof_bundle = serde_json::json!({
        "s15_status": "pass",
        "s15_proof": null
    });

    let (output, _, _) = run_gc_metrics_script_with_fixture_and_proof_bundle(
        "gc-metrics-proof-bundle-invalid",
        payload,
        proof_bundle,
    );

    assert!(
        !output.status.success(),
        "collect-gc-metrics.sh は invalid external collector proof bundle を reject するべき: stdout={}, stderr={}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("AR-04: s15_proof must be a JSON object when s15_status is 'pass'"),
        "collect-gc-metrics.sh は invalid merged proof を報告するべき: {}",
        stderr
    );
}

#[cfg(unix)]
#[test]
fn test_gc_metrics_script_autoloads_adjacent_collector_proof_bundle() {
    let payload = base_collector_payload();
    let proof_bundle = serde_json::json!({
        "s15_status": "pass",
        "s15_reason": null,
        "s15_proof": {
            "gc_mode": "mark-sweep",
            "stage_pair": ["stage2", "stage3"],
            "bytes_identical": true,
            "exports_identical": true,
            "data_sections_identical": true,
            "diagnostics_identical": true
        },
        "s16_status": "pass",
        "s16_reason": null,
        "s16_proof": {
            "gc_mode": "mark-sweep",
            "completed_workloads": [
                "compile_run_light_loop",
                "repl_soak_50_eval",
                "repl_stateful_long_session",
                "repl_stateful_single_session",
                "lsp_actual_stdio_repeated_sequence"
            ],
            "all_workloads_completed": true,
            "sigsegv_count": 0,
            "trap_count": 0,
            "unreachable_count": 0,
            "dangling_pointer_count": 0
        }
    });

    let (output, normalized_payload, normalized_sidecar) =
        run_gc_metrics_script_with_fixture_and_adjacent_proof_bundle(
            "gc-proof-bundle-auto",
            payload,
            proof_bundle,
        );

    assert!(
        output.status.success(),
        "collect-gc-metrics.sh は隣接 collector-proof.json を自動検出して受理するべき: status={:?}, stdout={}, stderr={}",
        output.status.code(),
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );

    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("s15_status: pass") && stdout.contains("s16_status: pass"),
        "collect-gc-metrics.sh は auto-merged gate 状態を出力するべき: {}",
        stdout
    );
    assert_eq!(normalized_payload["s15_status"], "pass");
    assert_eq!(normalized_payload["s16_status"], "pass");
    assert_eq!(normalized_payload["s14_reason"], serde_json::Value::Null);
    assert_eq!(normalized_payload["s15_reason"], serde_json::Value::Null);
    assert_eq!(normalized_payload["s16_reason"], serde_json::Value::Null);
    assert_eq!(
        normalized_payload["s15_proof"]["stage_pair"],
        serde_json::json!(["stage2", "stage3"])
    );
    assert_eq!(normalized_payload["s16_proof"]["gc_mode"], "mark-sweep");
    assert_eq!(normalized_sidecar["s15_status"], "pass");
    assert_eq!(normalized_sidecar["s16_status"], "pass");
    assert_eq!(normalized_sidecar["s15_reason"], serde_json::Value::Null);
    assert_eq!(normalized_sidecar["s16_reason"], serde_json::Value::Null);
}

#[cfg(unix)]
#[test]
fn test_gc_metrics_script_writes_default_collector_proof_sidecar() {
    let payload = base_collector_payload();

    let (output, normalized_payload, normalized_sidecar) =
        run_gc_metrics_script_with_fixture_and_capture_artifacts(
            "gc-proof-sidecar-default",
            payload,
        );

    assert!(
        output.status.success(),
        "collect-gc-metrics.sh は proof bundle 未指定でも collector-proof.json sidecar を生成するべき: status={:?}, stdout={}, stderr={}",
        output.status.code(),
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(normalized_payload["s15_status"], "blocked");
    assert_eq!(normalized_payload["s16_status"], "blocked");
    assert_eq!(normalized_payload["s14_reason"], serde_json::Value::Null);
    assert_eq!(normalized_payload["s15_reason"], S15_BLOCKED_REASON);
    assert_eq!(normalized_payload["s16_reason"], S16_BLOCKED_REASON);
    assert_eq!(
        normalized_sidecar,
        serde_json::json!({
            "s15_status": "blocked",
            "s15_reason": S15_BLOCKED_REASON,
            "s15_proof": null,
            "s16_status": "blocked",
            "s16_reason": S16_BLOCKED_REASON,
            "s16_proof": null
        })
    );
}

#[cfg(unix)]
#[test]
fn test_gc_metrics_script_adjacent_proof_bundle_does_not_override_actual_payload_proofs() {
    let mut payload = base_collector_payload();
    payload["s15_status"] = serde_json::json!("pass");
    payload["s15_reason"] = serde_json::Value::Null;
    payload["s15_proof"] = serde_json::json!({
        "gc_mode": "mark-sweep",
        "stage_pair": ["stage2", "stage3"],
        "bytes_identical": true,
        "exports_identical": true,
        "data_sections_identical": true,
        "diagnostics_identical": true
    });
    payload["s16_status"] = serde_json::json!("pass");
    payload["s16_reason"] = serde_json::Value::Null;
    payload["s16_proof"] = serde_json::json!({
        "gc_mode": "mark-sweep",
        "completed_workloads": [
            "compile_run_light_loop",
            "repl_soak_50_eval",
            "repl_stateful_long_session",
            "repl_stateful_single_session",
            "lsp_actual_stdio_repeated_sequence"
        ],
        "all_workloads_completed": true,
        "sigsegv_count": 0,
        "trap_count": 0,
        "unreachable_count": 0,
        "dangling_pointer_count": 0
    });

    let stale_proof_bundle = serde_json::json!({
        "s15_status": "blocked",
        "s15_reason": S15_BLOCKED_REASON,
        "s15_proof": null,
        "s16_status": "blocked",
        "s16_reason": S16_BLOCKED_REASON,
        "s16_proof": null
    });

    let (output, normalized_payload, normalized_sidecar) =
        run_gc_metrics_script_with_fixture_and_adjacent_proof_bundle(
            "gc-proof-bundle-preserve-actual",
            payload,
            stale_proof_bundle,
        );

    assert!(
        output.status.success(),
        "collect-gc-metrics.sh は actual payload proof を stale adjacent sidecar で上書きしないべき: status={:?}, stdout={}, stderr={}",
        output.status.code(),
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(normalized_payload["s15_status"], "pass");
    assert_eq!(normalized_payload["s16_status"], "pass");
    assert_eq!(normalized_payload["s15_reason"], serde_json::Value::Null);
    assert_eq!(normalized_payload["s16_reason"], serde_json::Value::Null);
    assert_eq!(normalized_payload["s15_proof"]["gc_mode"], "mark-sweep");
    assert_eq!(normalized_payload["s16_proof"]["gc_mode"], "mark-sweep");
    assert_eq!(normalized_sidecar["s15_status"], "pass");
    assert_eq!(normalized_sidecar["s16_status"], "pass");
}

#[cfg(unix)]
#[test]
fn test_gc_metrics_script_normalizes_collector_proof_sidecar_after_merge() {
    let payload = base_collector_payload();
    let proof_bundle = serde_json::json!({
        "s15_status": "pass",
        "s15_reason": null,
        "s15_proof": {
            "gc_mode": "mark-sweep",
            "stage_pair": ["stage2", "stage3"],
            "bytes_identical": true,
            "exports_identical": true,
            "data_sections_identical": true,
            "diagnostics_identical": true
        },
        "s16_status": "pass",
        "s16_reason": null,
        "s16_proof": {
            "gc_mode": "mark-sweep",
            "completed_workloads": [
                "compile_run_light_loop",
                "repl_soak_50_eval",
                "repl_stateful_long_session",
                "repl_stateful_single_session",
                "lsp_actual_stdio_repeated_sequence"
            ],
            "all_workloads_completed": true,
            "sigsegv_count": 0,
            "trap_count": 0,
            "unreachable_count": 0,
            "dangling_pointer_count": 0
        }
    });

    let (output, normalized_payload, normalized_sidecar) =
        run_gc_metrics_script_with_fixture_and_proof_bundle(
            "gc-proof-sidecar-merge",
            payload,
            proof_bundle,
        );

    assert!(
        output.status.success(),
        "collect-gc-metrics.sh は merge 後 proof sidecar を正規化出力するべき: status={:?}, stdout={}, stderr={}",
        output.status.code(),
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(
        normalized_payload["s15_status"],
        normalized_sidecar["s15_status"]
    );
    assert_eq!(
        normalized_payload["s15_reason"],
        normalized_sidecar["s15_reason"]
    );
    assert_eq!(
        normalized_payload["s15_proof"],
        normalized_sidecar["s15_proof"]
    );
    assert_eq!(
        normalized_payload["s16_status"],
        normalized_sidecar["s16_status"]
    );
    assert_eq!(
        normalized_payload["s16_reason"],
        normalized_sidecar["s16_reason"]
    );
    assert_eq!(
        normalized_payload["s16_proof"],
        normalized_sidecar["s16_proof"]
    );
}
