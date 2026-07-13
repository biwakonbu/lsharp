#!/usr/bin/env python3
"""Rust host launcher を使わずに native selfhost documentation を生成する。"""

import argparse
import html
import json
import os
import pathlib
import subprocess
import sys
import tempfile


class NativeDocError(Exception):
    def __init__(self, message, child_stderr=b""):
        super().__init__(message)
        self.message = message
        self.child_stderr = child_stderr


def validate_program(program_text):
    program = pathlib.Path(program_text)
    if not program.is_file():
        raise NativeDocError(f"program is not a regular file: {program}")
    if not os.access(program, os.X_OK):
        raise NativeDocError(f"program is not executable: {program}")
    return program.resolve()


def validate_source(source_text):
    source = pathlib.Path(source_text)
    if not source.is_file():
        raise NativeDocError(f"source file is not a regular file: {source}")
    return source.resolve()


def duplicate_key_rejecting_object(pairs):
    value = {}
    for key, item in pairs:
        if key in value:
            raise ValueError(f"duplicate object key: {key}")
        value[key] = item
    return value


def reject_nonstandard_json_constant(value):
    raise ValueError(f"non-standard JSON constant: {value}")


def parse_native_json(output):
    try:
        value = json.loads(
            output,
            object_pairs_hook=duplicate_key_rejecting_object,
            parse_constant=reject_nonstandard_json_constant,
        )
    except (UnicodeDecodeError, json.JSONDecodeError, ValueError) as error:
        raise NativeDocError(f"malformed native JSON: {error}") from error
    validate_document(value)
    return value


def schema_error(path, message):
    raise NativeDocError(f"native JSON schema violation at {path}: {message}")


def require_object(value, path, required, optional=()):
    if type(value) is not dict:
        schema_error(path, "expected object")
    allowed = set(required) | set(optional)
    missing = sorted(set(required) - set(value))
    unsupported = sorted(set(value) - allowed)
    if missing:
        schema_error(path, f"missing required keys: {', '.join(missing)}")
    if unsupported:
        schema_error(path, f"unsupported keys: {', '.join(unsupported)}")


def require_array(value, path):
    if type(value) is not list:
        schema_error(path, "expected array")


def require_string(value, path):
    if type(value) is not str:
        schema_error(path, "expected string")


def require_integer(value, path):
    if type(value) is not int:
        schema_error(path, "expected integer")


def require_non_negative_integer(value, path):
    require_integer(value, path)
    if value < 0:
        schema_error(path, "expected non-negative integer")


def validate_param(param, path):
    require_object(param, path, ("name", "type", "doc"))
    require_string(param["name"], f"{path}.name")
    require_string(param["type"], f"{path}.type")
    require_string(param["doc"], f"{path}.doc")


def validate_returns(returns, path):
    require_object(returns, path, ("type", "doc"))
    require_string(returns["type"], f"{path}.type")
    require_string(returns["doc"], f"{path}.doc")


def validate_function(function, path):
    require_object(
        function,
        path,
        ("name", "arity", "params", "returns", "doc", "example"),
    )
    require_string(function["name"], f"{path}.name")
    require_non_negative_integer(function["arity"], f"{path}.arity")
    require_array(function["params"], f"{path}.params")
    for index, param in enumerate(function["params"]):
        validate_param(param, f"{path}.params[{index}]")
    validate_returns(function["returns"], f"{path}.returns")
    require_string(function["doc"], f"{path}.doc")
    require_string(function["example"], f"{path}.example")


def validate_type(type_entry, path):
    require_object(type_entry, path, ("name", "kind"))
    require_string(type_entry["name"], f"{path}.name")
    require_string(type_entry["kind"], f"{path}.kind")


def validate_html_section(section, path):
    require_object(section, path, ("id", "count"))
    require_string(section["id"], f"{path}.id")
    if section["id"] not in ("functions", "types"):
        schema_error(f"{path}.id", "expected 'functions' or 'types'")
    require_non_negative_integer(section["count"], f"{path}.count")


def validate_document(document):
    require_object(document, "root", ("module", "functions", "types", "html"))
    require_string(document["module"], "root.module")

    require_array(document["functions"], "root.functions")
    for index, function in enumerate(document["functions"]):
        validate_function(function, f"root.functions[{index}]")

    require_array(document["types"], "root.types")
    for index, type_entry in enumerate(document["types"]):
        validate_type(type_entry, f"root.types[{index}]")

    html_data = document["html"]
    require_object(html_data, "root.html", ("title", "sections"))
    require_string(html_data["title"], "root.html.title")
    require_array(html_data["sections"], "root.html.sections")
    for index, section in enumerate(html_data["sections"]):
        validate_html_section(section, f"root.html.sections[{index}]")


def run_native_doc(program, source):
    try:
        completed = subprocess.run(
            [str(program), "doc", str(source), "--json"],
            stdout=subprocess.PIPE,
            stderr=subprocess.PIPE,
            check=False,
        )
    except OSError as error:
        raise NativeDocError(f"failed to execute native program: {error}") from error

    if completed.returncode != 0:
        raise NativeDocError(
            f"native program exited with status {completed.returncode}", completed.stderr
        )
    if completed.stderr:
        raise NativeDocError("native program wrote to stderr", completed.stderr)
    return completed.stdout, parse_native_json(completed.stdout)


def escape(value):
    return html.escape(value, quote=True)


def render_function(function):
    parts = ["<section class=\"function\">"]
    parts.append(f"<h3>{escape(function['name'])}</h3>")
    parts.append(f"<p><strong>Arity:</strong> {function['arity']}</p>")
    parts.append("<h4>Parameters</h4>")
    if function["params"]:
        parts.append("<dl>")
        for param in function["params"]:
            parts.append(
                "<dt><code>{}</code> <span>{}</span></dt>".format(
                    escape(param["name"]), escape(param["type"])
                )
            )
            if "doc" in param:
                parts.append(f"<dd>{escape(param['doc'])}</dd>")
        parts.append("</dl>")
    else:
        parts.append("<p>None</p>")

    returns = function["returns"]
    parts.append("<h4>Returns</h4>")
    parts.append(f"<p><code>{escape(returns['type'])}</code></p>")
    if "doc" in returns:
        parts.append(f"<p>{escape(returns['doc'])}</p>")
    if "doc" in function:
        parts.append("<h4>Documentation</h4>")
        parts.append(f"<p>{escape(function['doc'])}</p>")
    if "example" in function:
        parts.append("<h4>Example</h4>")
        parts.append(f"<pre><code>{escape(function['example'])}</code></pre>")
    parts.append("</section>")
    return "\n".join(parts)


def render_type(type_entry):
    return "\n".join(
        (
            "<section class=\"type\">",
            f"<h3>{escape(type_entry['name'])}</h3>",
            f"<p><strong>Kind:</strong> {escape(type_entry['kind'])}</p>",
            "</section>",
        )
    )


def render_html(document):
    title = document["html"]["title"]
    parts = [
        "<!DOCTYPE html>",
        "<html lang=\"ja\">",
        "<head>",
        "<meta charset=\"utf-8\">",
        "<meta name=\"viewport\" content=\"width=device-width, initial-scale=1\">",
        f"<title>{escape(title)}</title>",
        "</head>",
        "<body>",
        "<main>",
        f"<h1>{escape(title)}</h1>",
        f"<p><strong>Module:</strong> <code>{escape(document['module'])}</code></p>",
        "<h2>Functions</h2>",
    ]
    if document["functions"]:
        parts.extend(render_function(function) for function in document["functions"])
    else:
        parts.append("<p>None</p>")
    parts.append("<h2>Types</h2>")
    if document["types"]:
        parts.extend(render_type(type_entry) for type_entry in document["types"])
    else:
        parts.append("<p>None</p>")
    parts.extend(("</main>", "</body>", "</html>"))
    return "\n".join(parts) + "\n"


def nearest_project_root(source):
    current = source.parent
    while True:
        if (current / "lsharp.toml").is_file():
            return current
        if current.parent == current:
            return source.parent
        current = current.parent


def default_json_output(source):
    return nearest_project_root(source) / "docs" / "api.json"


def atomic_write(destination, content):
    destination = pathlib.Path(destination)
    if destination.exists() and destination.is_dir():
        raise NativeDocError(f"output path is a directory: {destination}")
    try:
        destination.parent.mkdir(parents=True, exist_ok=True)
    except OSError as error:
        raise NativeDocError(
            f"failed to create output directory {destination.parent}: {error}"
        ) from error

    temporary_path = None
    try:
        descriptor, temporary_name = tempfile.mkstemp(
            prefix=f".{destination.name}.", suffix=".tmp", dir=destination.parent
        )
        temporary_path = pathlib.Path(temporary_name)
        with os.fdopen(descriptor, "wb") as output:
            output.write(content)
            output.flush()
            os.fsync(output.fileno())
        os.replace(temporary_path, destination)
    except OSError as error:
        raise NativeDocError(f"failed to write output {destination}: {error}") from error
    finally:
        if temporary_path is not None:
            try:
                temporary_path.unlink()
            except FileNotFoundError:
                pass
            except OSError:
                pass


def parse_args(argv):
    parser = argparse.ArgumentParser(
        description="Generate documentation through a native selfhost program."
    )
    parser.add_argument("--program", required=True, metavar="PATH")
    parser.add_argument("file", metavar="FILE")
    parser.add_argument("-o", "--output", metavar="OUT")
    output_format = parser.add_mutually_exclusive_group()
    output_format.add_argument("--json", action="store_true", dest="json_output")
    output_format.add_argument("--format", choices=("json",), dest="format_name")
    return parser.parse_args(argv)


def write_error(error):
    stderr = sys.stderr.buffer
    stderr.write(b"native-selfhost-doc: ")
    stderr.write(error.message.encode("utf-8", "replace"))
    stderr.write(b"\n")
    if error.child_stderr:
        stderr.write(error.child_stderr)
        if not error.child_stderr.endswith(b"\n"):
            stderr.write(b"\n")
    stderr.flush()


def main(argv=None):
    args = parse_args(argv)
    try:
        program = validate_program(args.program)
        source = validate_source(args.file)
        native_json, document = run_native_doc(program, source)
        json_output = args.json_output or args.format_name == "json"
        if json_output:
            destination = pathlib.Path(args.output) if args.output else default_json_output(source)
            atomic_write(destination, native_json)
            print(destination)
        else:
            rendered = render_html(document).encode("utf-8")
            if args.output:
                destination = pathlib.Path(args.output)
                atomic_write(destination, rendered)
                print(destination)
            else:
                sys.stdout.write(rendered.decode("utf-8"))
                sys.stdout.flush()
    except NativeDocError as error:
        write_error(error)
        return 1
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
