#!/usr/bin/env python3
"""Correlate Linux x86 entrypoint metadata rows with emitted rel32 calls."""

from __future__ import annotations

import argparse
import json
import sys
from pathlib import Path


HEADER_MARKER = 9_000_000_020
METADATA_ROW_MARKER = 9_000_000_021
USER_CALL_REPLAY_MARKER = 9_000_000_046
WORD64 = 1 << 64
SIGN_BIT64 = 1 << 63


class DiagnosticError(ValueError):
    pass


def read_values(path: Path) -> list[int]:
    try:
        text = path.read_text(encoding="utf-8")
    except OSError as error:
        raise DiagnosticError(f"metadata read failed: {path}: {error}") from error
    try:
        return [int(token) for token in text.split()]
    except ValueError as error:
        raise DiagnosticError(f"metadata contains a non-integer token: {path}") from error


def signed_word(value: int) -> int:
    if value >= SIGN_BIT64:
        return value - WORD64
    return value


def byte_values(values: list[int], label: str) -> list[int]:
    if any(value < 0 or value > 255 for value in values):
        raise DiagnosticError(f"{label} contains a non-byte value: {values}")
    return values


def parse_metadata_rows(values: list[int]) -> tuple[list[int], list[dict[str, object]], list[dict[str, object]]]:
    function_indices: list[int] = []
    metadata_rows: list[dict[str, object]] = []
    replay_rows: list[dict[str, object]] = []
    for index, marker in enumerate(values):
        if marker == HEADER_MARKER:
            if index + 12 > len(values):
                raise DiagnosticError(f"metadata header is truncated at value {index}")
            function_indices.append(values[index + 2])
        elif marker == METADATA_ROW_MARKER:
            if index + 15 > len(values):
                raise DiagnosticError(f"metadata row is truncated at value {index}")
            metadata_rows.append(
                {
                    "instr_idx": values[index + 1],
                    "opcode": values[index + 2],
                    "operand": values[index + 3],
                    "depth": values[index + 4],
                    "offset": values[index + 5],
                    "size": values[index + 6],
                    "bytes": byte_values(values[index + 7 : index + 15], "metadata row bytes"),
                }
            )
        elif marker == USER_CALL_REPLAY_MARKER:
            if index + 21 > len(values):
                raise DiagnosticError(f"user call replay row is truncated at value {index}")
            replay_rows.append(
                {
                    "instr_idx": values[index + 1],
                    "opcode": values[index + 2],
                    "operand": values[index + 3],
                    "depth": values[index + 4],
                    "offset": values[index + 5],
                    "size": values[index + 6],
                    "param_count": values[index + 7],
                    "call_next_offset": values[index + 8],
                    "target_offset": signed_word(values[index + 9]),
                    "call_rel": signed_word(values[index + 10]),
                    "direct_len": values[index + 11],
                    "direct_target": signed_word(values[index + 12]),
                    "bytes": byte_values(values[index + 13 : index + 21], "replay row bytes"),
                }
            )
    if not function_indices:
        raise DiagnosticError("metadata header is missing")
    if not metadata_rows:
        raise DiagnosticError("metadata rows are missing")
    if not replay_rows:
        raise DiagnosticError("user call replay rows are missing")
    return function_indices, metadata_rows, replay_rows


def correlate(values: list[int], requested_function_index: int | None) -> dict[str, object]:
    function_indices, metadata_rows, replay_rows = parse_metadata_rows(values)
    if requested_function_index is not None and requested_function_index not in function_indices:
        raise DiagnosticError(
            "requested function index is absent: "
            f"requested={requested_function_index} headers={function_indices}"
        )

    calls: list[dict[str, object]] = []
    for replay in replay_rows:
        if replay["opcode"] != 40:
            continue
        matches = [
            row
            for row in metadata_rows
            if all(row[field] == replay[field] for field in ("instr_idx", "opcode", "operand", "depth", "offset", "size"))
        ]
        if len(matches) != 1:
            raise DiagnosticError(
                "user call replay row does not map to exactly one metadata row: "
                f"row={replay} matches={len(matches)}"
            )
        metadata = matches[0]
        size = min(int(metadata["size"]), len(metadata["bytes"]))
        emitted = list(metadata["bytes"][:size])
        call_offsets = [offset for offset, byte in enumerate(emitted) if byte == 0xE8]
        if len(call_offsets) != 1 or call_offsets[0] + 5 > size:
            raise DiagnosticError(f"opcode 40 row does not contain one complete rel32 call: row={metadata}")
        call_offset = call_offsets[0]
        rel32 = int.from_bytes(bytes(emitted[call_offset + 1 : call_offset + 5]), "little", signed=True)
        target_offset = int(metadata["offset"]) + call_offset + 5 + rel32
        expected_bytes = list(replay["bytes"][:size])
        calls.append(
            {
                "instr_idx": replay["instr_idx"],
                "opcode": replay["opcode"],
                "operand": replay["operand"],
                "depth": replay["depth"],
                "offset": replay["offset"],
                "size": replay["size"],
                "emitted_bytes": bytes(emitted).hex(),
                "expected_bytes": bytes(expected_bytes).hex(),
                "bytes_match": emitted == expected_bytes,
                "rel32": rel32,
                "expected_rel32": replay["call_rel"],
                "rel32_match": rel32 == replay["call_rel"],
                "target_offset": target_offset,
                "expected_target_offset": replay["target_offset"],
                "target_match": target_offset == replay["target_offset"],
            }
        )
    if not calls:
        raise DiagnosticError("opcode 40 user call rows are missing")
    return {
        "function_index": requested_function_index if requested_function_index is not None else function_indices[0],
        "call_count": len(calls),
        "all_calls_match": all(
            call["bytes_match"] and call["rel32_match"] and call["target_match"] for call in calls
        ),
        "calls": calls,
    }


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("metadata", type=Path)
    parser.add_argument("--function-index", type=int)
    arguments = parser.parse_args()
    try:
        report = correlate(read_values(arguments.metadata), arguments.function_index)
    except DiagnosticError as error:
        print(f"error: {error}", file=sys.stderr)
        return 2
    print(json.dumps(report, sort_keys=True, separators=(",", ":")))
    return 0 if report["all_calls_match"] else 1


if __name__ == "__main__":
    raise SystemExit(main())
