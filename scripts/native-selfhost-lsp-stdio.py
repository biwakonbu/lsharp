#!/usr/bin/env python3
"""Replay an EOF-only native LSP command as a persistent stdio server."""

import argparse
import os
import pathlib
import subprocess
import sys


HEADER_END = b"\r\n\r\n"
HEADER_NAME_PUNCTUATION = b"!#$%&'*+-.^_|~" + bytes((96,))


class ShimError(Exception):
    def __init__(self, message, child_stderr=b""):
        super().__init__(message)
        self.message = message
        self.child_stderr = child_stderr


class FrameError(Exception):
    pass


def is_header_name(name):
    return bool(name) and all(
        ord("0") <= byte <= ord("9")
        or ord("A") <= byte <= ord("Z")
        or ord("a") <= byte <= ord("z")
        or byte in HEADER_NAME_PUNCTUATION
        for byte in name
    )


def parse_content_length(header, source):
    content_length = None
    if not header:
        raise FrameError(f"malformed {source} frame header: missing Content-Length")

    for line in header.split(b"\r\n"):
        if b":" not in line:
            raise FrameError(f"malformed {source} frame header")
        name, value = line.split(b":", 1)
        if not is_header_name(name):
            raise FrameError(f"malformed {source} frame header")
        if name.lower() != b"content-length":
            continue
        if content_length is not None:
            raise FrameError(f"malformed {source} frame header: duplicate Content-Length")
        value = value.strip(b" \t")
        if not value or any(byte < ord("0") or byte > ord("9") for byte in value):
            raise FrameError(f"invalid Content-Length in {source} frame")
        content_length = int(value)

    if content_length is None:
        raise FrameError(f"malformed {source} frame header: missing Content-Length")
    return content_length


class LspFrameParser:
    def __init__(self, source):
        self.source = source
        self.buffer = bytearray()
        self.header_size = None
        self.content_length = None

    def feed(self, data):
        self.buffer.extend(data)
        frames = []
        while True:
            if self.content_length is None:
                header_end = self.buffer.find(HEADER_END)
                if header_end < 0:
                    if b"\n\n" in self.buffer:
                        raise FrameError(
                            f"malformed {self.source} frame header: expected CRLF"
                        )
                    return frames
                self.content_length = parse_content_length(
                    bytes(self.buffer[:header_end]), self.source
                )
                self.header_size = header_end + len(HEADER_END)

            frame_size = self.header_size + self.content_length
            if len(self.buffer) < frame_size:
                return frames
            frames.append(bytes(self.buffer[:frame_size]))
            del self.buffer[:frame_size]
            self.header_size = None
            self.content_length = None

    def finish(self):
        if self.content_length is not None:
            body_size = len(self.buffer) - self.header_size
            raise FrameError(
                f"truncated {self.source} frame: declared Content-Length "
                f"{self.content_length}, received {body_size} body bytes"
            )
        if self.buffer:
            if b"\n\n" in self.buffer:
                raise FrameError(
                    f"malformed {self.source} frame header: expected CRLF"
                )
            raise FrameError(f"truncated {self.source} frame header")


def parse_complete_frames(data, source):
    parser = LspFrameParser(source)
    frames = parser.feed(data)
    parser.finish()
    return frames


def validate_program(program_text):
    program = pathlib.Path(program_text)
    if not program.is_file():
        raise ShimError(f"program is not a regular file: {program}")
    if not os.access(program, os.X_OK):
        raise ShimError(f"program is not executable: {program}")
    return str(program.resolve())


def run_program(program, program_args, aggregate):
    command = [program, "lsp", "--stdio", *program_args]
    try:
        completed = subprocess.run(
            command,
            input=aggregate,
            stdout=subprocess.PIPE,
            stderr=subprocess.PIPE,
            check=False,
        )
    except OSError as error:
        raise ShimError(f"failed to execute program: {error}") from error

    if completed.returncode != 0:
        raise ShimError(
            f"program exited with status {completed.returncode}", completed.stderr
        )
    if completed.stderr:
        raise ShimError("program wrote to stderr", completed.stderr)
    try:
        return parse_complete_frames(completed.stdout, "child output")
    except FrameError as error:
        raise ShimError(f"malformed child output: {error}") from error


def read_chunks(stream):
    read1 = getattr(stream, "read1", None)
    if read1 is None:
        while True:
            chunk = stream.read(1)
            if not chunk:
                return
            yield chunk
    else:
        while True:
            chunk = read1(65536)
            if not chunk:
                return
            yield chunk


def write_error(error):
    stderr = sys.stderr.buffer
    stderr.write(b"native-selfhost-lsp-stdio: ")
    stderr.write(error.message.encode("utf-8", "replace"))
    stderr.write(b"\n")
    if error.child_stderr:
        stderr.write(error.child_stderr)
        if not error.child_stderr.endswith(b"\n"):
            stderr.write(b"\n")
    stderr.flush()


def parse_args(argv):
    parser = argparse.ArgumentParser(
        description="Replay an EOF-only native LSP command over persistent stdio."
    )
    parser.add_argument("--program", required=True, metavar="PATH")
    parser.add_argument("program_args", nargs=argparse.REMAINDER, metavar="ARGS")
    args = parser.parse_args(argv)
    if args.program_args[:1] == ["--"]:
        args.program_args = args.program_args[1:]
    return args


def main(argv=None):
    args = parse_args(argv)
    try:
        program = validate_program(args.program)
        parser = LspFrameParser("inbound")
        aggregate = bytearray()
        previous_responses = []

        for chunk in read_chunks(sys.stdin.buffer):
            for request in parser.feed(chunk):
                aggregate.extend(request)
                responses = run_program(program, args.program_args, bytes(aggregate))
                previous_count = len(previous_responses)
                if len(responses) < previous_count:
                    raise ShimError(
                        "response frame count regressed during replay: "
                        f"previous={previous_count} current={len(responses)}"
                    )
                if responses[:previous_count] != previous_responses:
                    raise ShimError("response replay prefix changed")
                new_responses = responses[previous_count:]
                if new_responses:
                    sys.stdout.buffer.write(b"".join(new_responses))
                    sys.stdout.buffer.flush()
                previous_responses = responses

        parser.finish()
    except ShimError as error:
        write_error(error)
        return 1
    except FrameError as error:
        write_error(ShimError(str(error)))
        return 1
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
