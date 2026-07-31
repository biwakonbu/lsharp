"""Offline installed-package projection for the native MCP shim."""

import json
import pathlib

try:
    import tomllib
except ModuleNotFoundError:  # pragma: no cover - Python 3.10 fallback
    tomllib = None


class PackageLookupError(Exception):
    """Invalid offline package/context arguments are reported as MCP errors."""


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

PACKAGE_ITEM_SCHEMA = SEARCH_OUTPUT_SCHEMA["properties"]["packages"]["items"]

PROJECT_CONTEXT_OUTPUT_SCHEMA = {
    "$schema": "https://json-schema.org/draft/2020-12/schema",
    "type": "object",
    "additionalProperties": False,
    "required": ["project", "dependencies", "installedPackages"],
    "properties": {
        "project": {
            "type": "object",
            "additionalProperties": False,
            "required": ["name", "version", "description", "exports"],
            "properties": {
                "name": {"type": "string"},
                "version": {"type": "string"},
                "description": {"type": "string"},
                "exports": {"type": "array", "items": {"type": "string"}},
            },
        },
        "dependencies": {
            "type": "array",
            "items": {
                "type": "object",
                "additionalProperties": False,
                "required": ["name", "source"],
                "properties": {
                    "name": {"type": "string", "minLength": 1},
                    "source": {"type": "string", "enum": ["registry", "path", "git"]},
                    "version": {"type": "string"},
                    "path": {"type": "string", "minLength": 1},
                    "git": {"type": "string", "minLength": 1},
                    "branch": {"type": ["string", "null"]},
                    "tag": {"type": ["string", "null"]},
                },
            },
        },
        "installedPackages": {"type": "array", "items": PACKAGE_ITEM_SCHEMA},
    },
}

PACKAGE_API_OUTPUT_SCHEMA = {
    "$schema": "https://json-schema.org/draft/2020-12/schema",
    "type": "object",
    "additionalProperties": False,
    "required": ["package", "version", "modules"],
    "properties": {
        "package": {"type": "string", "minLength": 1},
        "version": {"type": "string", "minLength": 1},
        "modules": {
            "type": "array",
            "items": {
                "type": "object",
                "additionalProperties": False,
                "required": ["name", "doc", "functions", "types"],
                "properties": {
                    "name": {"type": "string", "minLength": 1},
                    "doc": {"type": ["string", "null"]},
                    "functions": {
                        "type": "array",
                        "items": {
                            "type": "object",
                            "additionalProperties": False,
                            "required": ["name", "signature", "params", "returns", "doc", "example"],
                            "properties": {
                                "name": {"type": "string", "minLength": 1},
                                "signature": {"type": "string", "minLength": 1},
                                "params": {
                                    "type": "array",
                                    "items": {
                                        "type": "object",
                                        "additionalProperties": False,
                                        "required": ["name", "type", "doc"],
                                        "properties": {
                                            "name": {"type": "string", "minLength": 1},
                                            "type": {"type": "string", "minLength": 1},
                                            "doc": {"type": ["string", "null"]},
                                        },
                                    },
                                },
                                "returns": {
                                    "type": "object",
                                    "additionalProperties": False,
                                    "required": ["type", "doc"],
                                    "properties": {
                                        "type": {"type": "string", "minLength": 1},
                                        "doc": {"type": ["string", "null"]},
                                    },
                                },
                                "doc": {"type": ["string", "null"]},
                                "example": {"type": ["string", "null"]},
                            },
                        },
                    },
                    "types": {
                        "type": "array",
                        "items": {
                            "type": "object",
                            "additionalProperties": False,
                            "required": ["name", "kind"],
                            "properties": {
                                "name": {"type": "string", "minLength": 1},
                                "kind": {"type": "string", "minLength": 1},
                            },
                        },
                    },
                },
            },
        },
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


def _decode_toml_value(value):
    value = value.split(" #", 1)[0].strip()
    if value.startswith("[") and value.endswith("]"):
        try:
            decoded = json.loads(value)
            return decoded if isinstance(decoded, list) else []
        except json.JSONDecodeError:
            return []
    return _decode_string(value)


def _fallback_config_data(text):
    section = ""
    project = {}
    dependencies = {}
    for raw_line in text.splitlines():
        line = raw_line.strip()
        if line.startswith("["):
            section = line[1:-1].strip()
            continue
        if not line or line.startswith("#") or "=" not in line:
            continue
        key, raw_value = (part.strip() for part in line.split("=", 1))
        value = _decode_toml_value(raw_value)
        if section == "project" and key in {"name", "version", "description"}:
            if isinstance(value, str):
                project[key] = value
        elif section == "project.exports" and key == "modules" and isinstance(value, list):
            project.setdefault("exports", {})["modules"] = [item for item in value if isinstance(item, str)]
        elif section == "dependencies" and isinstance(value, str):
            dependencies[key] = value
        elif section.startswith("dependencies.") and key in {"path", "git", "branch", "tag"}:
            name = section[len("dependencies.") :]
            spec = dependencies.setdefault(name, {})
            if isinstance(spec, dict) and isinstance(value, str):
                spec[key] = value
    return {"project": project, "dependencies": dependencies}


def _load_config_data(project_dir):
    config_path = project_dir / "lsharp.toml"
    try:
        text = config_path.read_text(encoding="utf-8")
    except OSError:
        return {}
    if tomllib is not None:
        try:
            value = tomllib.loads(text)
            return value if isinstance(value, dict) else {}
        except (tomllib.TOMLDecodeError, ValueError):
            return {}
    return _fallback_config_data(text)


def _project_config(package_dir):
    defaults = {"name": "", "version": "0.1.0"}
    project = _load_config_data(package_dir).get("project", {})
    if not isinstance(project, dict):
        return defaults
    return {
        "name": project.get("name") if isinstance(project.get("name"), str) else "",
        "version": project.get("version") if isinstance(project.get("version"), str) else "0.1.0",
    }


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


def _dependency_summary(name, spec, project_dir):
    if isinstance(spec, str):
        return {"name": name, "version": spec, "source": "registry"}
    if not isinstance(spec, dict):
        return None
    if isinstance(spec.get("path"), str):
        path = spec["path"]
        resolved_path = str(pathlib.Path(path)) if pathlib.Path(path).is_absolute() else f"{project_dir}/{path}"
        return {"name": name, "source": "path", "path": resolved_path}
    if isinstance(spec.get("git"), str):
        return {
            "name": name,
            "source": "git",
            "git": spec["git"],
            "branch": spec.get("branch") if isinstance(spec.get("branch"), str) else None,
            "tag": spec.get("tag") if isinstance(spec.get("tag"), str) else None,
        }
    return None


def call_project_context(arguments):
    unknown = sorted(set(arguments).difference({"project_dir"}))
    if unknown:
        raise PackageLookupError(f"lsharp_project_context の未知の引数: {', '.join(unknown)}")
    project_dir = arguments.get("project_dir")
    if project_dir is None:
        project_path = pathlib.Path.cwd()
    elif isinstance(project_dir, str) and project_dir.strip():
        project_path = pathlib.Path(project_dir)
    else:
        raise PackageLookupError("lsharp_project_context の project_dir は空でない文字列が必要です")

    data = _load_config_data(project_path)
    project = data.get("project", {})
    if not isinstance(project, dict):
        project = {}
    exports = project.get("exports", {})
    modules = exports.get("modules", []) if isinstance(exports, dict) else []
    if not isinstance(modules, list) or not all(isinstance(module, str) for module in modules):
        modules = []
    dependencies = data.get("dependencies", {})
    if not isinstance(dependencies, dict):
        dependencies = {}
    summaries = [
        summary
        for name, spec in sorted(dependencies.items())
        if isinstance(name, str) and (summary := _dependency_summary(name, spec, project_path)) is not None
    ]
    return {
        "project": {
            "name": project.get("name") if isinstance(project.get("name"), str) else "",
            "version": project.get("version") if isinstance(project.get("version"), str) else "0.1.0",
            "description": project.get("description") if isinstance(project.get("description"), str) else "",
            "exports": modules,
        },
        "dependencies": summaries,
        "installedPackages": _installed_packages(project_path),
    }


def call_package_api(arguments):
    unknown = sorted(set(arguments).difference({"name", "project_dir"}))
    if unknown:
        raise PackageLookupError(f"lsharp_package_api の未知の引数: {', '.join(unknown)}")
    name = arguments.get("name")
    if not isinstance(name, str) or not name.strip():
        raise PackageLookupError("lsharp_package_api の name は空でない文字列が必要です")
    project_dir = arguments.get("project_dir")
    if project_dir is None:
        project_path = pathlib.Path.cwd()
    elif isinstance(project_dir, str) and project_dir.strip():
        project_path = pathlib.Path(project_dir)
    else:
        raise PackageLookupError("lsharp_package_api の project_dir は空でない文字列が必要です")
    packages_dir = project_path / ".lsharp" / "packages"
    try:
        package_dir = next(
            path
            for path in sorted(packages_dir.iterdir(), key=lambda path: path.name)
            if path.name.startswith(f"{name}-") and path.is_dir()
        )
    except (OSError, StopIteration):
        raise PackageLookupError(f"インストール済みパッケージ '{name}' が見つかりません") from None
    api_path = package_dir / "docs" / "api.json"
    try:
        value = json.loads(api_path.read_text(encoding="utf-8"))
    except (OSError, json.JSONDecodeError) as error:
        raise PackageLookupError(f"{api_path}: api.json を読み込めません: {error}") from error
    if not isinstance(value, dict):
        raise PackageLookupError(f"{api_path}: api.json の root は object が必要です")
    return value
