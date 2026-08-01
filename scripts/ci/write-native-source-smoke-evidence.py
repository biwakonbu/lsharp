#!/usr/bin/env python3
"""Persist one native source-file smoke run without overwriting prior evidence."""

from __future__ import annotations

import argparse
import hashlib
import json
import os
from pathlib import Path
import re
import shutil
import tempfile
from typing import NoReturn


TARGETS = {"aarch64-apple-darwin", "x86_64-unknown-linux-gnu"}
SOURCE_COMMIT = re.compile(r"^[0-9a-f]{40}$")


def fail(message: str) -> NoReturn:
    raise SystemExit(message)


def sha256(path: Path) -> str:
    digest = hashlib.sha256()
    with path.open("rb") as stream:
        for chunk in iter(lambda: stream.read(1024 * 1024), b""):
            digest.update(chunk)
    return digest.hexdigest()


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--evidence-dir", required=True)
    parser.add_argument("--work-dir", required=True)
    parser.add_argument("--stage0-manifest", required=True)
    parser.add_argument("--target", required=True, choices=sorted(TARGETS))
    parser.add_argument("--exit-code", required=True, type=int)
    return parser.parse_args()


def load_stage0_manifest(path: Path, target: str) -> tuple[dict[str, object], str]:
    if path.is_symlink() or not path.is_file() or not path.stat().st_size:
        fail(f"stage0 manifest is not a non-empty regular file: {path}")
    try:
        manifest = json.loads(path.read_text(encoding="utf-8"))
    except (OSError, UnicodeDecodeError, json.JSONDecodeError) as error:
        fail(f"stage0 manifest is invalid: {error}")
    if not isinstance(manifest, dict):
        fail("stage0 manifest must be a JSON object")
    if manifest.get("kind") != "lsharp-native-selfhost-stage0":
        fail("stage0 manifest kind is invalid")
    if manifest.get("target") != target:
        fail(
            "stage0 manifest target does not match source smoke target: "
            f"manifest={manifest.get('target')!r} target={target!r}"
        )
    source_commit = manifest.get("source_commit")
    if not isinstance(source_commit, str) or not SOURCE_COMMIT.fullmatch(source_commit):
        fail(
            "stage0 manifest source_commit must be 40 lowercase hexadecimal characters"
        )
    return manifest, sha256(path)


def artifact_metadata(work_dir: Path) -> dict[str, dict[str, int | str]]:
    artifacts: dict[str, dict[str, int | str]] = {}
    for name in ("compile.wasm", "build.wasm"):
        path = work_dir / name
        if path.is_file() and not path.is_symlink():
            artifacts[name] = {"sha256": sha256(path), "size": path.stat().st_size}
    return artifacts


def command_outputs(work_dir: Path) -> list[str]:
    return sorted(
        str(path.relative_to(work_dir))
        for path in work_dir.rglob("*")
        if path.is_file() and path.suffix in {".stdout", ".stderr"}
    )


def first_symlink(path: Path) -> Path | None:
    for root, directories, files in os.walk(path, topdown=True, followlinks=False):
        for name in sorted((*directories, *files)):
            candidate = Path(root) / name
            if candidate.is_symlink():
                return candidate
    return None


def main() -> int:
    args = parse_args()
    if not 0 <= args.exit_code <= 255:
        fail(f"exit code is outside the shell range: {args.exit_code}")

    evidence_dir = Path(args.evidence_dir)
    if not evidence_dir.is_absolute() or evidence_dir == Path("/"):
        fail(f"evidence directory must be an absolute non-root path: {evidence_dir}")
    if evidence_dir.exists() or evidence_dir.is_symlink():
        fail(f"evidence directory already exists: {evidence_dir}")

    work_dir = Path(args.work_dir)
    if work_dir.is_symlink() or not work_dir.is_dir():
        fail(f"source smoke work directory is unavailable: {work_dir}")
    work_symlink = first_symlink(work_dir)
    if work_symlink is not None:
        fail(f"source smoke work directory contains a symlink: {work_symlink}")

    stage0_manifest = Path(args.stage0_manifest)
    manifest, stage0_digest = load_stage0_manifest(stage0_manifest, args.target)

    parent = evidence_dir.parent
    parent.mkdir(parents=True, exist_ok=True)
    staging = Path(tempfile.mkdtemp(prefix=f".{evidence_dir.name}.", dir=parent))
    try:
        shutil.copytree(work_dir, staging / "work", symlinks=True)
        staged_symlink = first_symlink(staging / "work")
        if staged_symlink is not None:
            fail(f"source smoke evidence contains a symlink: {staged_symlink}")
        shutil.copy2(stage0_manifest, staging / "stage0-manifest.json")
        (staging / "exit.code").write_text(f"{args.exit_code}\n", encoding="ascii")
        evidence = {
            "kind": "lsharp-native-selfhost-source-smoke-evidence",
            "target": args.target,
            "source_commit": manifest["source_commit"],
            "stage0_manifest_sha256": stage0_digest,
            "exit_code": args.exit_code,
            "artifacts": artifact_metadata(work_dir),
            "command_outputs": command_outputs(work_dir),
        }
        (staging / "manifest.json").write_text(
            json.dumps(evidence, ensure_ascii=False, indent=2, sort_keys=True) + "\n",
            encoding="utf-8",
        )
        try:
            os.rename(staging, evidence_dir)
        except FileExistsError:
            fail(f"evidence directory appeared during capture: {evidence_dir}")
    except Exception:
        shutil.rmtree(staging, ignore_errors=True)
        raise
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
