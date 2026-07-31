"""Offline installed-package projection for the native MCP shim."""

import json
import pathlib

try:
    import tomllib
except ModuleNotFoundError:  # pragma: no cover - Python 3.10 fallback
    tomllib = None


class PackageLookupError(Exception):
    """Invalid lsharp_search arguments are reported as MCP tool errors."""


SEARCH_OUTPUT_SCHEMA = {
    "$schema": "https://json-schema.org/draft/2020-12/schema",
    "type": "object",
    "additionalProperties": False,
    "required": ["packages"],
    "properties": {
        "packages": {
            "type": "array",
            "items": {
                "type": "object",
                "additionalProperties": False,
                "required": ["name", "version", "path"],
                "properties": {
                    "name": {"type": "string", "minLength": 1},
                    "version": {"type": "string"},
                    "path": {"type": "string", "minLength": 1},
                },
            },
        }
    },
}


def _decode_string(value):
    if value.startswith('"') and value.endswith('"'):
        try:
            return json.loads(value)
        except json.JSONDecodeError:
            return ""
    if value.startswith("'") and value.endswith("'"):
        return value[1:-1]
    return ""


def _fallback_project(text):
    in_project = False
    values = {}
    for raw_line in text.splitlines():
        line = raw_line.strip()
        if line.startswith("["):
            in_project = line == "[project]"
            continue
        if not in_project or "=" not in line:
            continue
        key, raw_value = (part.strip() for part in line.split("=", 1))
        if key in {"name", "version"}:
            values[key] = _decode_string(raw_value)
    return values


def _project_config(package_dir):
    defaults = {"name": "", "version": "0.1.0"}
    config_path = package_dir / "lsharp.toml"
    try:
        text = config_path.read_text(encoding="utf-8")
    except OSError:
        return defaults
    if tomllib is not None:
        try:
            project = tomllib.loads(text).get("project", {})
            if isinstance(project, dict):
                return {
                    "name": project.get("name") if isinstance(project.get("name"), str) else "",
                    "version": project.get("version") if isinstance(project.get("version"), str) else "0.1.0",
                }
        except (tomllib.TOMLDecodeError, ValueError):
            return defaults
    values = _fallback_project(text)
    return {**defaults, **{key: value for key, value in values.items() if value}}


def _installed_packages(project_dir):
    packages_dir = project_dir / ".lsharp" / "packages"
    try:
        entries = sorted(packages_dir.iterdir(), key=lambda path: path.name)
    except OSError:
        return []
    packages = []
    for path in entries:
        if not path.is_dir() and not path.is_symlink():
            continue
        config = _project_config(path)
        name = config["name"] or path.name or "package"
        packages.append({"name": name, "version": config["version"], "path": str(path)})
    return sorted(packages, key=lambda package: (package["name"], package["path"]))


def call_search(arguments):
    unknown = sorted(set(arguments).difference({"project_dir", "query"}))
    if unknown:
        raise PackageLookupError(f"lsharp_search の未知の引数: {', '.join(unknown)}")
    project_dir = arguments.get("project_dir")
    if project_dir is None:
        project_path = pathlib.Path.cwd()
    elif isinstance(project_dir, str) and project_dir.strip():
        project_path = pathlib.Path(project_dir)
    else:
        raise PackageLookupError("project_dir は空でない文字列が必要です")
    query = arguments.get("query", "")
    if not isinstance(query, str):
        raise PackageLookupError("query は文字列が必要です")
    packages = [package for package in _installed_packages(project_path) if query in package["name"]]
    return {"packages": packages}
