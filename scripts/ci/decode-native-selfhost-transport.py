#!/usr/bin/env python3
"""Decode the marker transport emitted by native self-host generation."""

import argparse
import pathlib
from typing import List, Optional, Tuple


DEFAULT_TARGET = "x86_64-unknown-linux-gnu"
SEGMENT_MARKER = 9000000010
HEADER_MARKER = 9000000005
HEADER_END_MARKER = 9000000006
CODE_MARKER = 9000000001
CODE_PAYLOAD_MARKER = 9000000002
DATA_MARKER = 9000000003
DATA_PAYLOAD_MARKER = 9000000004


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser(
        description="Decode native self-host marker transport into stage artifacts."
    )
    parser.add_argument(
        "--target",
        default=DEFAULT_TARGET,
        help=f"target triple recorded in manifest.json (default: {DEFAULT_TARGET})",
    )
    parser.add_argument("transport_path", type=pathlib.Path)
    parser.add_argument("out_dir", type=pathlib.Path)
    return parser.parse_args()


def parse_int(line: bytes) -> int:
    return int(line.decode("utf-8"))


def decode_packed_flat(packed_lines: List[bytes], declared_len: int) -> bytes:
    expected_line_count = packed_line_count(declared_len)
    if len(packed_lines) != expected_line_count:
        raise SystemExit(
            "packed payload line count mismatch: "
            f"declared={declared_len} expected={expected_line_count} "
            f"actual={len(packed_lines)}"
        )
    decoded = bytearray()
    mask = (1 << 64) - 1
    for raw in packed_lines:
        packed = parse_int(raw) & mask
        for byte_idx in range(8):
            if len(decoded) >= declared_len:
                return bytes(decoded)
            decoded.append((packed >> (byte_idx * 8)) & 0xFF)
    if len(decoded) != declared_len:
        raise SystemExit(
            f"decoded length mismatch: declared={declared_len} actual={len(decoded)}"
        )
    return bytes(decoded)


def packed_line_count(byte_len: int) -> int:
    if byte_len < 0:
        raise SystemExit(f"payload length must be non-negative: {byte_len}")
    return (byte_len + 7) // 8


def decode_packed_payload_at(
    lines: List[bytes], index: int, declared_len: int, end_sentinel: Optional[int]
) -> Tuple[bytes, int, List[Tuple[int, int, int, bytes]]]:
    if declared_len < 0:
        raise SystemExit(f"payload length must be non-negative: {declared_len}")
    if index < len(lines) and parse_int(lines[index]) == SEGMENT_MARKER:
        decoded = bytearray()
        segments = []
        segment_index = 0
        while index < len(lines) and len(decoded) < declared_len:
            if parse_int(lines[index]) != SEGMENT_MARKER:
                raise SystemExit(f"missing segment marker at line {index}")
            index += 1
            if index >= len(lines):
                raise SystemExit("missing segment length after segment marker")
            segment_len = parse_int(lines[index])
            index += 1
            count = packed_line_count(segment_len)
            segment = decode_packed_flat(lines[index:index + count], segment_len)
            segments.append((segment_index, len(decoded), segment_len, bytes(segment[:32])))
            decoded.extend(segment)
            index += count
            segment_index += 1
        if len(decoded) != declared_len:
            raise SystemExit(
                "decoded segmented length mismatch: "
                f"declared={declared_len} actual={len(decoded)}"
            )
        return bytes(decoded), index, segments

    if end_sentinel is None:
        payload = decode_packed_flat(lines[index:], declared_len)
        return payload, len(lines), [(0, 0, declared_len, payload[:32])]

    start = index
    while index < len(lines) and parse_int(lines[index]) != end_sentinel:
        index += 1
    payload = decode_packed_flat(lines[start:index], declared_len)
    return payload, index, [(0, 0, declared_len, payload[:32])]


def expect(lines: List[bytes], index: int, sentinel: int) -> int:
    if index >= len(lines) or parse_int(lines[index]) != sentinel:
        got = lines[index][:80] if index < len(lines) else b"<eof>"
        raise SystemExit(f"missing sentinel {sentinel} at line {index}: {got!r}")
    return index + 1


def write_code_segment_table(
    out_path: pathlib.Path,
    segments: List[Tuple[int, int, int, bytes]],
    function_start_len: int,
) -> None:
    rows = ["segment_index\tfunction_idx\tkind\tstart\tlen\tend\tfirst_32_bytes"]
    for segment_index, start, segment_len, first_bytes in segments:
        function_idx = segment_index + 10 if segment_index < function_start_len else -1
        kind = "function" if segment_index < function_start_len else "trailer"
        first_32_bytes = " ".join(f"{byte:02x}" for byte in first_bytes)
        rows.append(
            f"{segment_index}\t{function_idx}\t{kind}\t{start}\t{segment_len}\t"
            f"{start + segment_len}\t{first_32_bytes}"
        )
    out_path.write_text("\n".join(rows) + "\n")


def main() -> None:
    args = parse_args()
    args.out_dir.mkdir(parents=True, exist_ok=True)
    lines = [
        line
        for line in args.transport_path.read_bytes().replace(b"\0", b"\n").splitlines()
        if line
    ]

    index = 0
    while index < len(lines) and parse_int(lines[index]) != HEADER_MARKER:
        index += 1
    index = expect(lines, index, HEADER_MARKER)
    function_start_len = parse_int(lines[index])
    index += 1
    main_func_idx = parse_int(lines[index])
    index += 1
    entrypoint_offset = parse_int(lines[index])
    index += 1
    index = expect(lines, index, HEADER_END_MARKER)
    index = expect(lines, index, CODE_MARKER)
    code_len = parse_int(lines[index])
    index += 1
    index = expect(lines, index, CODE_PAYLOAD_MARKER)
    code, index, code_segments = decode_packed_payload_at(
        lines, index, code_len, DATA_MARKER
    )
    index = expect(lines, index, DATA_MARKER)
    data_len = parse_int(lines[index])
    index += 1
    index = expect(lines, index, DATA_PAYLOAD_MARKER)
    data, _index, _data_segments = decode_packed_payload_at(
        lines, index, data_len, None
    )

    (args.out_dir / "stage-code.bin").write_bytes(code)
    (args.out_dir / "stage-data.bin").write_bytes(data)
    (args.out_dir / "entrypoint-offset.txt").write_text(f"{entrypoint_offset}\n")
    (args.out_dir / "function-start-len.txt").write_text(f"{function_start_len}\n")
    (args.out_dir / "main-func-idx.txt").write_text(f"{main_func_idx}\n")
    write_code_segment_table(
        args.out_dir / "stage-code-segments.tsv", code_segments, function_start_len
    )
    (args.out_dir / "manifest.json").write_text(
        "{\n"
        f'  "target": "{args.target}",\n'
        f'  "code_len": {len(code)},\n'
        f'  "data_len": {len(data)},\n'
        f'  "entrypoint_offset": {entrypoint_offset},\n'
        f'  "function_start_len": {function_start_len},\n'
        f'  "main_func_idx": {main_func_idx}\n'
        "}\n"
    )


if __name__ == "__main__":
    main()
