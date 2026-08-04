#!/usr/bin/env python3
"""Check the native MCP shim through an actual selfhost App.Cli program."""

import argparse
import json
import pathlib
import subprocess
import sys
import tempfile


SCRIPTS_DIR = pathlib.Path(__file__).resolve().parents[1]
SHIM = SCRIPTS_DIR / "native-selfhost-mcp.py"
SOURCE = "(defn main [] 42)\n"


def request(request_id, method, params=None):
    payload = {"jsonrpc": "2.0", "id": request_id, "method": method}
    if params is not None:
        payload["params"] = params
    return (json.dumps(payload, separators=(",", ":")) + "\n").encode("utf-8")


def run_shim(program, payload, root):
    result = subprocess.run(
        [sys.executable, str(SHIM), "--program", str(program)],
        cwd=root,
        input=payload,
        capture_output=True,
        check=False,
    )
    if result.returncode != 0 or result.stderr:
        raise AssertionError(
            f"native MCP runtime failed: exit={result.returncode} stderr={result.stderr!r}"
        )
    return [json.loads(line) for line in result.stdout.splitlines() if line]


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("--program", required=True, type=pathlib.Path)
    args = parser.parse_args()
    program = args.program.expanduser().resolve()
    if not program.is_file() or not program.stat().st_mode & 0o111:
        raise SystemExit(f"native program is not executable: {program}")
    if not SHIM.is_file():
        raise SystemExit(f"native MCP shim is missing: {SHIM}")

    payload = b"".join(
        [
            request(1, "initialize"),
            request(2, "tools/list"),
            request(
                3,
                "tools/call",
                {"name": "lsharp_check", "arguments": {"source": SOURCE}},
            ),
            request(
                4,
                "tools/call",
                {"name": "lsharp_format", "arguments": {"source": SOURCE}},
            ),
            request(
                5,
                "tools/call",
                {"name": "lsharp_install", "arguments": {"name": "demo"}},
            ),
        ]
    )
    with tempfile.TemporaryDirectory(prefix="lsharp-native-mcp-runtime-") as directory:
        responses = run_shim(program, payload, pathlib.Path(directory))

    if len(responses) != 5:
        raise AssertionError(f"MCP response count mismatch: {responses!r}")
    if responses[0]["result"]["serverInfo"] != {
        "name": "lsharp",
        "version": "0.1.0",
    }:
        raise AssertionError(f"initialize response mismatch: {responses[0]!r}")
    tool_names = {tool["name"] for tool in responses[1]["result"]["tools"]}
    if not {"lsharp_check", "lsharp_format", "lsharp_install"} <= tool_names:
        raise AssertionError(f"MCP tool list is incomplete: {sorted(tool_names)!r}")

    check = responses[2]["result"]
    if check.get("isError") is not False or check["structuredContent"] != {
        "ok": True,
        "diagnostics": [],
        "migrationDiagnostics": [],
    }:
        raise AssertionError(f"lsharp_check response mismatch: {check!r}")

    formatted = responses[3]["result"]
    if formatted.get("isError") is not False or formatted["structuredContent"] != {
        "formatted": SOURCE,
    }:
        raise AssertionError(f"lsharp_format response mismatch: {formatted!r}")

    install = responses[4]["result"]
    if install.get("isError") is not True or install["content"] != [
        {
            "type": "text",
            "text": "native MCP package installation requires an explicit external provider adapter",
        }
    ]:
        raise AssertionError(f"lsharp_install boundary mismatch: {install!r}")

    print("native MCP runtime contract passed: 5 requests")


if __name__ == "__main__":
    main()
