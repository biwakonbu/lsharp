#!/usr/bin/env bash
set -euo pipefail

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
decoder="${repo_root}/scripts/ci/decode-native-selfhost-transport.py"
tmp_dir="$(mktemp -d)"
trap 'rm -rf "${tmp_dir}"' EXIT

write_fixture() {
  local fixture_kind="$1"
  local fixture_path="$2"

  python3 - "${fixture_kind}" "${fixture_path}" <<'PY'
import pathlib
import sys

fixture_kind = sys.argv[1]
fixture_path = pathlib.Path(sys.argv[2])


def packed_lines(payload: bytes) -> list[str]:
    return [str(int.from_bytes(payload[index:index + 8], "little")) for index in range(0, len(payload), 8)]


def segmented_lines(segments: list[bytes]) -> list[str]:
    lines = []
    for segment in segments:
        lines.extend(["9000000010", str(len(segment)), *packed_lines(segment)])
    return lines


if fixture_kind == "flat":
    code = b"flat-code!"
    data = b"flat-data"
    function_start_len = 1
    main_func_idx = 10
    entrypoint_offset = 4
    code_lines = packed_lines(code)
    data_lines = packed_lines(data)
elif fixture_kind == "segmented":
    code = b"ABCdefghiJK"
    data = b"data-12345"
    function_start_len = 2
    main_func_idx = 11
    entrypoint_offset = 9
    code_lines = segmented_lines([b"ABC", b"defghi", b"JK"])
    data_lines = segmented_lines([b"data", b"-12345"])
else:
    raise SystemExit(f"unknown fixture kind: {fixture_kind}")

lines = [
    "9000000005",
    str(function_start_len),
    str(main_func_idx),
    str(entrypoint_offset),
    "9000000006",
    "9000000001",
    str(len(code)),
    "9000000002",
    *code_lines,
    "9000000003",
    str(len(data)),
    "9000000004",
    *data_lines,
]
fixture_path.write_text("\n".join(lines) + "\n")
PY
}

assert_output() {
  local fixture_kind="$1"
  local output_dir="$2"
  local expected_target="$3"

  python3 - "${fixture_kind}" "${output_dir}" "${expected_target}" <<'PY'
import json
import pathlib
import sys

fixture_kind = sys.argv[1]
output_dir = pathlib.Path(sys.argv[2])
expected_target = sys.argv[3]

if fixture_kind == "flat":
    expected_code = b"flat-code!"
    expected_data = b"flat-data"
    expected_layout = (4, 1, 10)
    expected_segments = [
        "segment_index\tfunction_idx\tkind\tstart\tlen\tend\tfirst_32_bytes",
        "0\t10\tfunction\t0\t10\t10\t66 6c 61 74 2d 63 6f 64 65 21",
    ]
elif fixture_kind == "segmented":
    expected_code = b"ABCdefghiJK"
    expected_data = b"data-12345"
    expected_layout = (9, 2, 11)
    expected_segments = [
        "segment_index\tfunction_idx\tkind\tstart\tlen\tend\tfirst_32_bytes",
        "0\t10\tfunction\t0\t3\t3\t41 42 43",
        "1\t11\tfunction\t3\t6\t9\t64 65 66 67 68 69",
        "2\t-1\ttrailer\t9\t2\t11\t4a 4b",
    ]
else:
    raise SystemExit(f"unknown fixture kind: {fixture_kind}")

assert (output_dir / "stage-code.bin").read_bytes() == expected_code
assert (output_dir / "stage-data.bin").read_bytes() == expected_data
assert int((output_dir / "entrypoint-offset.txt").read_text()) == expected_layout[0]
assert int((output_dir / "function-start-len.txt").read_text()) == expected_layout[1]
assert int((output_dir / "main-func-idx.txt").read_text()) == expected_layout[2]
assert (output_dir / "stage-code-segments.tsv").read_text().splitlines() == expected_segments

manifest = json.loads((output_dir / "manifest.json").read_text())
assert manifest == {
    "target": expected_target,
    "code_len": len(expected_code),
    "data_len": len(expected_data),
    "entrypoint_offset": expected_layout[0],
    "function_start_len": expected_layout[1],
    "main_func_idx": expected_layout[2],
}
PY
}

flat_fixture="${tmp_dir}/flat.transport"
flat_output="${tmp_dir}/flat-output"
write_fixture flat "${flat_fixture}"
python3 "${decoder}" "${flat_fixture}" "${flat_output}"
assert_output flat "${flat_output}" x86_64-unknown-linux-gnu

segmented_fixture="${tmp_dir}/segmented.transport"
segmented_output="${tmp_dir}/segmented-output"
write_fixture segmented "${segmented_fixture}"
python3 "${decoder}" --target aarch64-unknown-linux-gnu "${segmented_fixture}" "${segmented_output}"
assert_output segmented "${segmented_output}" aarch64-unknown-linux-gnu

printf 'decode-native-selfhost-transport: PASS\n'
