"""Offline installed-package projection for the native MCP shim."""

import importlib.util
import json
import os
import pathlib

from native_selfhost_json import strict_json_loads

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
        if not path.is_dir():
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


_NATIVE_DOC_HELPER = None


def _native_doc_helper():
    global _NATIVE_DOC_HELPER
    if _NATIVE_DOC_HELPER is None:
        helper_path = pathlib.Path(__file__).resolve().with_name("native-selfhost-doc.py")
        spec = importlib.util.spec_from_file_location("native_selfhost_doc", helper_path)
        if spec is None or spec.loader is None:
            raise PackageLookupError(f"native doc helper を読み込めません: {helper_path}")
        module = importlib.util.module_from_spec(spec)
        spec.loader.exec_module(module)
        _NATIVE_DOC_HELPER = module
    return _NATIVE_DOC_HELPER


def _run_native_doc(program, source):
    helper = _native_doc_helper()
    try:
        _, document = helper.run_native_doc(program, source)
    except helper.NativeDocError as error:
        detail = error.message
        if error.child_stderr:
            child_stderr = error.child_stderr.decode("utf-8", "replace").strip()
            if child_stderr:
                detail = f"{detail}: {child_stderr}"
        raise PackageLookupError(f"{source}: native doc の生成に失敗しました: {detail}") from error
    return document


def _package_source_files(package_dir):
    source_root = package_dir / "src"
    if not source_root.is_dir():
        return []
    return sorted(
        path
        for path in source_root.rglob("*.ls")
        if path.is_file() and not path.is_symlink()
    )


def _optional_doc_text(value):
    return value if value else None


def _module_name(document, source):
    name = document["module"]
    if name.startswith("module-"):
        name = name[len("module-") :]
    return source.stem if name in {"", "global"} else name


def _module_doc(source):
    try:
        lines = source.read_text(encoding="utf-8").splitlines()
    except OSError:
        return None
    comments = []
    for line in lines:
        trimmed = line.lstrip()
        if trimmed.startswith(";;"):
            comments.append(trimmed[2:].strip())
            continue
        if not trimmed:
            continue
        break
    if comments and (".ls -" in comments[0] or ".ls:" in comments[0]):
        comments.pop(0)
    text = "\n".join(line for line in comments if line)
    return text or None


def _api_type_kind(kind):
    return {
        "recorddef": "record",
        "type": "adt",
        "typealias": "alias",
        "typeconstrained": "trait",
    }.get(kind, kind)


def _generated_module(document, source):
    functions = []
    for function in document["functions"]:
        params = function["params"]
        return_entry = function["returns"]
        signature = " -> ".join([param["type"] for param in params] + [return_entry["type"]])
        functions.append(
            {
                "name": function["name"],
                "signature": signature,
                "params": [
                    {
                        "name": param["name"],
                        "type": param["type"],
                        "doc": _optional_doc_text(param["doc"]),
                    }
                    for param in params
                ],
                "returns": {
                    "type": return_entry["type"],
                    "doc": _optional_doc_text(return_entry["doc"]),
                },
                "doc": _optional_doc_text(function["doc"]),
                "example": _optional_doc_text(function["example"]),
            }
        )
    return {
        "name": _module_name(document, source),
        "doc": _module_doc(source),
        "functions": functions,
        "types": [
            {"name": type_entry["name"], "kind": _api_type_kind(type_entry["kind"])}
            for type_entry in document["types"]
        ],
    }


def _generate_package_api(program, package_dir, package, version, api_path):
    source_files = _package_source_files(package_dir)
    if not source_files:
        raise PackageLookupError(
            f"{api_path}: api.json が無く、生成対象の src/**/*.ls が見つかりません"
        )
    modules = [
        _generated_module(_run_native_doc(program, source), source)
        for source in source_files
    ]
    modules.sort(key=lambda module: module["name"])
    value = {"package": package, "version": version, "modules": modules}
    _validate_package_api(value, api_path)
    return value


def call_package_api(program, arguments):
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
    if api_path.exists():
        try:
            value = strict_json_loads(api_path.read_text(encoding="utf-8"))
        except (OSError, json.JSONDecodeError, ValueError) as error:
            raise PackageLookupError(f"{api_path}: api.json を読み込めません: {error}") from error
        _validate_package_api(value, api_path)
        return value

    config = _project_config(package_dir)
    package = config["name"] or package_dir.name
    return _generate_package_api(program, package_dir, package, config["version"], api_path)


def _validate_package_api(value, api_path):
    """Validate the closed-world shape returned by ``lsharp_package_api``."""

    def fail(path, message):
        raise PackageLookupError(f"{api_path}: api.json の {path} {message}")

    def object_at(item, path, fields):
        if not isinstance(item, dict):
            fail(path, "は object が必要です")
        unknown = sorted(set(item).difference(fields))
        if unknown:
            fail(f"{path}.{unknown[0]}", "は未知のフィールドです")
        missing = [field for field in fields if field not in item]
        if missing:
            fail(f"{path}.{missing[0]}", "は必須です")
        return item

    def non_empty_string(item, path):
        if not isinstance(item, str) or not item.strip():
            fail(path, "は空でない文字列が必要です")

    def nullable_string(item, path):
        if item is not None and not isinstance(item, str):
            fail(path, "は文字列または null が必要です")

    if not isinstance(value, dict):
        fail("root", "は object が必要です")
    root = object_at(value, "root", {"package", "version", "modules"})
    non_empty_string(root["package"], "package")
    non_empty_string(root["version"], "version")
    if not isinstance(root["modules"], list):
        fail("modules", "は配列が必要です")

    for module_index, raw_module in enumerate(root["modules"]):
        module_path = f"modules[{module_index}]"
        module = object_at(raw_module, module_path, {"name", "doc", "functions", "types"})
        non_empty_string(module["name"], f"{module_path}.name")
        nullable_string(module["doc"], f"{module_path}.doc")
        for collection in ("functions", "types"):
            if not isinstance(module[collection], list):
                fail(f"{module_path}.{collection}", "は配列が必要です")

        for function_index, raw_function in enumerate(module["functions"]):
            function_path = f"{module_path}.functions[{function_index}]"
            function = object_at(
                raw_function,
                function_path,
                {"name", "signature", "params", "returns", "doc", "example"},
            )
            non_empty_string(function["name"], f"{function_path}.name")
            non_empty_string(function["signature"], f"{function_path}.signature")
            nullable_string(function["doc"], f"{function_path}.doc")
            nullable_string(function["example"], f"{function_path}.example")
            if not isinstance(function["params"], list):
                fail(f"{function_path}.params", "は配列が必要です")
            returns_path = f"{function_path}.returns"
            returns = object_at(function["returns"], returns_path, {"type", "doc"})
            non_empty_string(returns["type"], f"{returns_path}.type")
            nullable_string(returns["doc"], f"{returns_path}.doc")

            for param_index, raw_param in enumerate(function["params"]):
                param_path = f"{function_path}.params[{param_index}]"
                param = object_at(raw_param, param_path, {"name", "type", "doc"})
                non_empty_string(param["name"], f"{param_path}.name")
                non_empty_string(param["type"], f"{param_path}.type")
                nullable_string(param["doc"], f"{param_path}.doc")

        for type_index, raw_type in enumerate(module["types"]):
            type_path = f"{module_path}.types[{type_index}]"
            type_info = object_at(raw_type, type_path, {"name", "kind"})
            non_empty_string(type_info["name"], f"{type_path}.name")
            non_empty_string(type_info["kind"], f"{type_path}.kind")


def _stdlib_api_path():
    explicit = os.environ.get("LSHARP_STDLIB_API_PATH", "")
    if explicit.strip():
        return pathlib.Path(explicit)
    root_value = os.environ.get("LSHARP_STDLIB_PATH", "")
    if root_value.strip():
        root = pathlib.Path(root_value)
        if root.exists():
            return root if root.is_file() else root / "api.json"
    return pathlib.Path(__file__).resolve().parent.parent / "stdlib" / "api.json"


def _stdlib_source_root(api_path):
    root_value = os.environ.get("LSHARP_STDLIB_PATH", "")
    if root_value.strip():
        root = pathlib.Path(root_value)
        return root.parent if root.is_file() else root
    return api_path.parent


def _stdlib_source_files(api_path):
    source_root = _stdlib_source_root(api_path)
    try:
        return sorted(
            path
            for path in source_root.iterdir()
            if path.suffix == ".ls" and path.is_file() and not path.is_symlink()
        )
    except OSError:
        return []


def _generate_stdlib_api(program, api_path):
    source_files = _stdlib_source_files(api_path)
    if not source_files:
        raise PackageLookupError(
            f"{api_path}: api.json が無く、生成対象の stdlib/*.ls が見つかりません"
        )
    modules = [
        _generated_module(_run_native_doc(program, source), source)
        for source in source_files
    ]
    modules.sort(key=lambda module: module["name"])
    value = {"package": "stdlib", "version": "0.1.0", "modules": modules}
    _validate_package_api(value, api_path)
    return value


def call_stdlib_api(program, arguments):
    unknown = sorted(set(arguments).difference({"module"}))
    if unknown:
        raise PackageLookupError(f"lsharp_stdlib_api の未知の引数: {', '.join(unknown)}")
    module = arguments.get("module")
    if module is not None and not isinstance(module, str):
        raise PackageLookupError("lsharp_stdlib_api の module は文字列が必要です")
    if isinstance(module, str) and not module.strip():
        raise PackageLookupError("lsharp_stdlib_api の module は空でない文字列が必要です")
    api_path = _stdlib_api_path()
    if api_path.exists():
        try:
            value = strict_json_loads(api_path.read_text(encoding="utf-8"))
        except (OSError, json.JSONDecodeError, ValueError) as error:
            raise PackageLookupError(f"{api_path}: api.json を読み込めません: {error}") from error
        _validate_package_api(value, api_path)
    else:
        value = _generate_stdlib_api(program, api_path)
    modules = value["modules"]
    if module is not None:
        modules = [item for item in modules if item.get("name") == module]
    return {"package": value["package"], "version": value["version"], "modules": modules}
