#!/usr/bin/env python3

import argparse
import pathlib
import stat
import sys
import tarfile


ALLOWED_NAMES = (
    "program.native",
    "manifest.json",
    "smoke-stdout.txt",
    "smoke-stderr.txt",
)


def parse_arguments():
    parser = argparse.ArgumentParser()
    parser.add_argument("--output", required=True, type=pathlib.Path)
    parser.add_argument("--program", required=True, type=pathlib.Path)
    parser.add_argument("--manifest", required=True, type=pathlib.Path)
    parser.add_argument("--smoke-stdout", required=True, type=pathlib.Path)
    parser.add_argument("--smoke-stderr", required=True, type=pathlib.Path)
    return parser.parse_args()


def require_regular_file(path, label, require_nonempty):
    try:
        file_stat = path.lstat()
    except FileNotFoundError as error:
        raise ValueError(f"{label} does not exist: {path}") from error
    if not stat.S_ISREG(file_stat.st_mode):
        raise ValueError(f"{label} must be a regular file: {path}")
    if require_nonempty and file_stat.st_size == 0:
        raise ValueError(f"{label} must not be empty: {path}")
    return file_stat


def verify_bundle(bundle_path):
    with tarfile.open(bundle_path, "r:gz") as bundle:
        members = bundle.getmembers()
    names = [member.name for member in members]
    if names != list(ALLOWED_NAMES):
        raise ValueError(f"native release input bundle entries are invalid: {names}")
    if any(not member.isfile() for member in members):
        raise ValueError("native release input bundle contains a non-regular entry")


def create_bundle(output, source_paths):
    source_stats = {
        name: require_regular_file(
            source_paths[name],
            name,
            name in {"program.native", "manifest.json"},
        )
        for name in ALLOWED_NAMES
    }

    output.parent.mkdir(parents=True, exist_ok=True)
    with tarfile.open(output, "w:gz", format=tarfile.USTAR_FORMAT) as bundle:
        for name in ALLOWED_NAMES:
            source_path = source_paths[name]
            source_stat = source_stats[name]
            member = tarfile.TarInfo(name=name)
            member.size = source_stat.st_size
            member.mode = stat.S_IMODE(source_stat.st_mode)
            member.mtime = int(source_stat.st_mtime)
            member.type = tarfile.REGTYPE
            with source_path.open("rb") as source_file:
                bundle.addfile(member, source_file)
    verify_bundle(output)


def main():
    arguments = parse_arguments()
    source_paths = {
        "program.native": arguments.program,
        "manifest.json": arguments.manifest,
        "smoke-stdout.txt": arguments.smoke_stdout,
        "smoke-stderr.txt": arguments.smoke_stderr,
    }
    try:
        create_bundle(arguments.output, source_paths)
    except (OSError, tarfile.TarError, ValueError) as error:
        print(f"ERROR: {error}", file=sys.stderr)
        return 1
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
