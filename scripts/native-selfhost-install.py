#!/usr/bin/env python3

import argparse
import json
import os
import pathlib
import platform
import re
import shutil
import subprocess
import sys
import uuid

try:
    import tomllib
except ModuleNotFoundError:
    tomllib = None


class InstallError(Exception):
    pass


def parse_arguments(argv):
    parser = argparse.ArgumentParser(
        description="Install L# dependencies without a Rust host launcher."
    )
    parser.add_argument(
        "--project-dir",
        required=True,
        metavar="DIR",
        help="Directory containing lsharp.toml.",
    )
    return parser.parse_args(argv)


def ensure_supported_host():
    system = platform.system()
    machine = platform.machine().lower()
    if system == "Darwin" and machine in {"arm64", "aarch64"}:
        return
    if system == "Linux" and machine in {"x86_64", "amd64"}:
        return
    raise InstallError(
        "native selfhost install supports macOS arm64 and Linux x86_64 only; "
        f"found {system} {platform.machine()}"
    )


def require_project_dir(value):
    project_dir = pathlib.Path(value).expanduser()
    if not project_dir.is_dir():
        raise InstallError(f"project directory does not exist: {project_dir}")
    project_dir = project_dir.resolve(strict=True)
    manifest = project_dir / "lsharp.toml"
    if not manifest.is_file():
        raise InstallError(f"lsharp.toml is required: {manifest}")
    return project_dir


def strip_toml_comment(line):
    quote = None
    escaped = False
    for index, character in enumerate(line):
        if quote is not None:
            if quote == '"' and escaped:
                escaped = False
            elif quote == '"' and character == "\\":
                escaped = True
            elif character == quote:
                quote = None
            continue
        if character in {"'", '"'}:
            quote = character
        elif character == "#":
            return line[:index]
    return line


def split_top_level(value, delimiter):
    parts = []
    start = 0
    quote = None
    escaped = False
    depth = 0
    for index, character in enumerate(value):
        if quote is not None:
            if quote == '"' and escaped:
                escaped = False
            elif quote == '"' and character == "\\":
                escaped = True
            elif character == quote:
                quote = None
            continue
        if character in {"'", '"'}:
            quote = character
        elif character in "[{":
            depth += 1
        elif character in "]}":
            depth -= 1
            if depth < 0:
                raise ValueError("unbalanced TOML value")
        elif character == delimiter and depth == 0:
            parts.append(value[start:index])
            start = index + 1
    if quote is not None or depth != 0:
        raise ValueError("unterminated TOML value")
    parts.append(value[start:])
    return parts


def toml_value_is_complete(value):
    quote = None
    escaped = False
    depth = 0
    for character in value:
        if quote is not None:
            if quote == '"' and escaped:
                escaped = False
            elif quote == '"' and character == "\\":
                escaped = True
            elif character == quote:
                quote = None
            continue
        if character in {"'", '"'}:
            quote = character
        elif character in "[{":
            depth += 1
        elif character in "]}":
            depth -= 1
            if depth < 0:
                raise ValueError("unbalanced TOML value")
    return quote is None and depth == 0


def split_toml_assignment(line):
    quote = None
    escaped = False
    for index, character in enumerate(line):
        if quote is not None:
            if quote == '"' and escaped:
                escaped = False
            elif quote == '"' and character == "\\":
                escaped = True
            elif character == quote:
                quote = None
            continue
        if character in {"'", '"'}:
            quote = character
        elif character == "=":
            return line[:index], line[index + 1 :]
    return None


def parse_toml_key_path(value):
    keys = []
    for raw_key in split_top_level(value.strip(), "."):
        key = raw_key.strip()
        if not key:
            raise ValueError(f"invalid TOML key path: {value}")
        if key.startswith('"') and key.endswith('"'):
            try:
                key = json.loads(key)
            except json.JSONDecodeError as error:
                raise ValueError(f"invalid TOML quoted key: {error}") from error
        elif key.startswith("'") and key.endswith("'"):
            key = key[1:-1]
        elif not re.fullmatch(r"[A-Za-z0-9_-]+", key):
            raise ValueError(f"invalid TOML key: {key}")
        keys.append(key)
    return keys


def parse_toml_value(value):
    value = value.strip()
    if not value:
        raise ValueError("missing TOML value")
    if value.startswith('"'):
        try:
            parsed = json.loads(value)
        except json.JSONDecodeError as error:
            raise ValueError(f"invalid TOML string: {error}") from error
        if not isinstance(parsed, str):
            raise ValueError("TOML string must decode to a string")
        return parsed
    if value.startswith("'"):
        if not value.endswith("'") or len(value) < 2:
            raise ValueError("invalid TOML literal string")
        return value[1:-1]
    if value.startswith("["):
        if not value.endswith("]"):
            raise ValueError("unterminated TOML array")
        inner = value[1:-1].strip()
        if not inner:
            return []
        return [parse_toml_value(item) for item in split_top_level(inner, ",") if item.strip()]
    if value.startswith("{"):
        if not value.endswith("}"):
            raise ValueError("unterminated TOML inline table")
        inner = value[1:-1].strip()
        table = {}
        if not inner:
            return table
        for item in split_top_level(inner, ","):
            assignment = split_top_level(item, "=")
            if len(assignment) != 2:
                raise ValueError(f"invalid TOML inline table entry: {item}")
            assign_toml_value(
                table,
                parse_toml_key_path(assignment[0]),
                parse_toml_value(assignment[1]),
            )
        return table
    if value == "true":
        return True
    if value == "false":
        return False
    if re.fullmatch(r"[+-]?[0-9]+", value):
        return int(value)
    return value


def assign_toml_value(table, keys, value):
    current = table
    for key in keys[:-1]:
        existing = current.get(key)
        if existing is None:
            existing = {}
            current[key] = existing
        if not isinstance(existing, dict):
            raise ValueError(f"TOML key is not a table: {key}")
        current = existing
    leaf = keys[-1]
    if leaf in current:
        raise ValueError(f"duplicate TOML key: {leaf}")
    current[leaf] = value


def ensure_toml_table(table, keys):
    current = table
    for key in keys:
        existing = current.get(key)
        if existing is None:
            existing = {}
            current[key] = existing
        if not isinstance(existing, dict):
            raise ValueError(f"TOML key is not a table: {key}")
        current = existing
    return current


def parse_toml_fallback(content):
    root = {}
    current = root
    pending = None
    for line_number, raw_line in enumerate(content.splitlines(), start=1):
        line = strip_toml_comment(raw_line).strip()
        if pending is not None:
            pending[2] += "\n" + line
            if not toml_value_is_complete(pending[2]):
                continue
            assign_toml_value(pending[0], pending[1], parse_toml_value(pending[2]))
            pending = None
            continue
        if not line:
            continue
        if line.startswith("["):
            if not line.endswith("]") or line.startswith("[["):
                raise ValueError(f"line {line_number}: unsupported TOML table syntax")
            current = ensure_toml_table(root, parse_toml_key_path(line[1:-1]))
            continue
        assignment = split_toml_assignment(line)
        if assignment is None:
            raise ValueError(f"line {line_number}: expected TOML assignment")
        keys = parse_toml_key_path(assignment[0])
        if not toml_value_is_complete(assignment[1]):
            pending = [current, keys, assignment[1], line_number]
            continue
        assign_toml_value(current, keys, parse_toml_value(assignment[1]))
    if pending is not None:
        raise ValueError(f"line {pending[3]}: unterminated TOML value")
    return root


def load_toml(path):
    try:
        content = path.read_text(encoding="utf-8")
    except OSError as error:
        raise InstallError(f"cannot read TOML file {path}: {error}") from error
    try:
        if tomllib is not None:
            return tomllib.loads(content)
        return parse_toml_fallback(content)
    except (ValueError, TypeError) as error:
        raise InstallError(f"invalid TOML in {path}: {error}") from error


def require_string(value, label):
    if not isinstance(value, str) or not value:
        raise InstallError(f"{label} must be a non-empty string")
    return value


def safe_package_name(name):
    if not isinstance(name, str) or not name:
        raise InstallError("dependency name must be a non-empty string")
    if name in {".", ".."} or any(character in name for character in "/\\\x00\r\n"):
        raise InstallError(f"dependency name is unsafe for .lsharp/packages: {name!r}")
    return name


def dependency_specs(config):
    dependencies = config.get("dependencies", {})
    if not isinstance(dependencies, dict):
        raise InstallError("[dependencies] must be a TOML table")
    specs = []
    for name in sorted(dependencies):
        value = dependencies[name]
        safe_package_name(name)
        if isinstance(value, str):
            specs.append((name, "version", value, None, None))
            continue
        if not isinstance(value, dict):
            raise InstallError(f"dependency {name!r} must be a version string or table")
        if "git" in value:
            git = require_string(value["git"], f"dependency {name!r}.git")
            branch = value.get("branch")
            tag = value.get("tag")
            if branch is not None:
                branch = require_string(branch, f"dependency {name!r}.branch")
            if tag is not None:
                tag = require_string(tag, f"dependency {name!r}.tag")
            specs.append((name, "git", git, branch, tag))
            continue
        if "path" in value:
            path = require_string(value["path"], f"dependency {name!r}.path")
            specs.append((name, "path", path, None, None))
            continue
        raise InstallError(f"dependency {name!r} must define path or git")
    return specs


def ensure_managed_directory(parent, name):
    path = parent / name
    if os.path.lexists(path):
        if path.is_symlink():
            raise InstallError(f"refusing symlinked managed directory: {path}")
        if not path.is_dir():
            raise InstallError(f"managed path is not a directory: {path}")
        return path
    try:
        path.mkdir()
    except OSError as error:
        raise InstallError(f"cannot create directory {path}: {error}") from error
    return path


def managed_child(parent, name):
    child = parent / name
    if child.parent != parent:
        raise InstallError(f"managed path escapes its parent: {child}")
    return child


def remove_managed_path(path, parent):
    if path.parent != parent:
        raise InstallError(f"refusing to remove path outside managed directory: {path}")
    if not os.path.lexists(path):
        return
    if path.is_symlink() or path.is_file():
        path.unlink()
        return
    if not path.is_dir():
        raise InstallError(f"refusing to remove unsupported managed path: {path}")
    shutil.rmtree(path)


def temporary_path(parent, stem):
    return managed_child(parent, f".{stem}.tmp-{uuid.uuid4().hex}")


def rollback_promotions(promoted, packages_dir):
    for destination, backup in reversed(promoted):
        if os.path.lexists(destination):
            remove_managed_path(destination, packages_dir)
        if backup is not None and os.path.lexists(backup):
            os.replace(backup, destination)


def rollback_install_state(promoted, metadata_backups, packages_dir, lsharp_dir):
    for path, backup in reversed(metadata_backups):
        if os.path.lexists(path):
            remove_managed_path(path, lsharp_dir)
        if backup is not None and os.path.lexists(backup):
            os.replace(backup, path)
    rollback_promotions(promoted, packages_dir)


def backup_metadata(path, backup):
    if not os.path.lexists(path):
        return None
    try:
        os.replace(path, backup)
    except OSError as error:
        raise InstallError(f"cannot back up install metadata {path}: {error}") from error
    return backup


def sync_install_path(path, failpoint):
    if os.environ.get("LSHARP_TEST_INSTALL_FAILPOINT") == failpoint:
        raise InstallError(f"test-only {failpoint} sync failpoint")
    sync_path = pathlib.Path(path)
    if sync_path.is_symlink():
        sync_path = sync_path.parent
    try:
        descriptor = os.open(sync_path, os.O_RDONLY)
    except OSError as error:
        raise InstallError(f"cannot open {sync_path} for {failpoint} sync: {error}") from error
    try:
        os.fsync(descriptor)
    except OSError as error:
        raise InstallError(f"cannot sync {sync_path} for {failpoint}: {error}") from error
    finally:
        os.close(descriptor)


def fnv1a64(value):
    hashed = 0xCBF29CE484222325
    for byte in value.encode("utf-8"):
        hashed ^= byte
        hashed = (hashed * 0x100000001B3) & 0xFFFFFFFFFFFFFFFF
    return f"{hashed:016x}"


def installed_package_dir(packages_dir, name, source):
    return managed_child(packages_dir, f"{name}-{fnv1a64(source)[:8]}")


def path_source(project_dir, dependency_path):
    source = pathlib.Path(dependency_path).expanduser()
    if not source.is_absolute():
        source = project_dir / source
    try:
        source = source.resolve(strict=True)
    except OSError as error:
        raise InstallError(f"path dependency does not exist: {source} ({error})") from error
    if not source.is_dir():
        raise InstallError(f"path dependency is not a directory: {source}")
    manifest = source / "lsharp.toml"
    if not manifest.is_file():
        raise InstallError(f"path dependency has no lsharp.toml: {manifest}")
    return source


def git_source(git, branch, tag):
    if branch is not None:
        return f"git:{git}?branch={branch}"
    if tag is not None:
        return f"git:{git}?tag={tag}"
    return f"git:{git}"


def package_version_text(package_dir):
    manifest = package_dir / "lsharp.toml"
    if not manifest.is_file():
        return "0.0.0"
    config = load_toml(manifest)
    project = config.get("project")
    if not isinstance(project, dict):
        return "0.0.0"
    version = project.get("version")
    if not isinstance(version, str) or not version:
        return "0.0.0"
    return version


def install_path_dependency(packages_dir, staging_dir, name, dependency_path, project_dir):
    source = path_source(project_dir, dependency_path)
    source_id = f"path:{source}"
    destination = installed_package_dir(packages_dir, name, source_id)
    if os.path.lexists(destination) and not destination.is_symlink():
        raise InstallError(f"refusing to replace non-symlink path package: {destination}")
    staged = managed_child(staging_dir, destination.name)
    try:
        os.symlink(str(source), str(staged), target_is_directory=True)
    except OSError as error:
        raise InstallError(f"cannot create path dependency symlink for {name!r}: {error}") from error
    return (
        {
            "name": name,
            "version": package_version_text(source),
            "source": source_id,
        },
        (staged, destination, f"{name} -> {source}"),
    )


def run_git_clone(git, branch, tag, destination):
    arguments = ["git", "clone", "--depth", "1"]
    reference = branch if branch is not None else tag
    if reference is not None:
        arguments.extend(["--branch", reference])
    arguments.extend([git, str(destination)])
    try:
        result = subprocess.run(arguments, capture_output=True, text=True, check=False)
    except OSError as error:
        raise InstallError(f"cannot execute git clone for {git}: {error}") from error
    if result.returncode == 0:
        return
    details = result.stderr.strip() or result.stdout.strip() or f"exit status {result.returncode}"
    raise InstallError(f"git clone failed for {git}: {details}")


def install_git_dependency(packages_dir, staging_dir, name, git, branch, tag):
    source_id = git_source(git, branch, tag)
    destination = installed_package_dir(packages_dir, name, source_id)
    promotion = None
    if os.path.lexists(destination):
        if destination.is_symlink():
            raise InstallError(f"refusing symlinked git package destination: {destination}")
        if not destination.is_dir():
            raise InstallError(f"git package destination is not a directory: {destination}")
        if not (destination / "lsharp.toml").is_file():
            raise InstallError(f"existing git package has no lsharp.toml: {destination}")
        package_dir = destination
    else:
        staged = managed_child(staging_dir, destination.name)
        run_git_clone(git, branch, tag, staged)
        if not (staged / "lsharp.toml").is_file():
            raise InstallError(f"cloned dependency has no lsharp.toml: {staged}")
        package_dir = staged
        promotion = (staged, destination, f"{name} (git: {git})")
    return (
        {
            "name": name,
            "version": package_version_text(package_dir),
            "source": source_id,
        },
        promotion,
    )


def parse_semver(value):
    original = value
    normalized = value.strip().lstrip("v")
    parts = normalized.split(".")
    if len(parts) != 3 or any(not part.isascii() or not part.isdigit() for part in parts):
        raise InstallError(f"invalid semver: {original}")
    return tuple(int(part) for part in parts)


def parse_version_requirement(value):
    if not isinstance(value, str):
        raise InstallError("version dependency must be a string")
    trimmed = value.strip()
    if trimmed.startswith(">="):
        return "minimum", parse_semver(trimmed[2:])
    if trimmed.startswith("="):
        return "exact", parse_semver(trimmed[1:])
    return "compatible", parse_semver(trimmed)


def semver_matches(kind, required, candidate):
    if kind == "exact":
        return candidate == required
    if kind == "minimum":
        return candidate >= required
    if candidate < required:
        return False
    major, minor, patch = required
    candidate_major, candidate_minor, candidate_patch = candidate
    if major > 0:
        return candidate_major == major
    if minor > 0:
        return candidate_major == 0 and candidate_minor == minor
    return candidate_major == 0 and candidate_minor == 0 and candidate_patch == patch


def resolve_cached_version_dependency(packages_dir, name, requirement):
    kind, required = parse_version_requirement(requirement)
    prefix = f"{name}-"
    candidates = []
    try:
        package_paths = sorted(packages_dir.iterdir(), key=lambda path: path.name)
    except OSError as error:
        raise InstallError(f"cannot scan package cache {packages_dir}: {error}") from error
    for package_dir in package_paths:
        if not package_dir.name.startswith(prefix):
            continue
        if not package_dir.is_dir() and not package_dir.is_symlink():
            continue
        manifest = package_dir / "lsharp.toml"
        if not manifest.is_file():
            continue
        version_text = package_version_text(package_dir)
        try:
            version = parse_semver(version_text)
        except InstallError:
            continue
        if semver_matches(kind, required, version):
            candidates.append((version, package_dir.name, version_text))
    if not candidates:
        raise InstallError(
            f"no cached package matches dependency {name!r} version requirement {requirement!r}"
        )
    _, _, version_text = max(candidates)
    return {"name": name, "version": version_text, "source": "registry:default"}


def exported_modules(package_dir):
    manifest = package_dir / "lsharp.toml"
    if not manifest.is_file():
        return None
    config = load_toml(manifest)
    project = config.get("project")
    if not isinstance(project, dict):
        return None
    exports = project.get("exports")
    if exports is None:
        return None
    if not isinstance(exports, dict):
        raise InstallError(f"project.exports must be a table: {manifest}")
    modules = exports.get("modules", [])
    if not isinstance(modules, list) or not all(isinstance(module, str) for module in modules):
        raise InstallError(f"project.exports.modules must be a string array: {manifest}")
    return set(modules) if modules else None


def module_index_path(index_root, module_name):
    parts = module_name.split(".")
    if not all(part and part not in {".", ".."} and "/" not in part and "\\" not in part for part in parts):
        return None
    return index_root.joinpath(*parts).with_suffix(".path")


def write_module_index(project_dir, packages_dir, index_root):
    for package_dir in sorted(packages_dir.iterdir(), key=lambda path: path.name):
        if not package_dir.is_dir() and not package_dir.is_symlink():
            continue
        source_root = package_dir / "src"
        if not source_root.is_dir():
            continue
        exports = exported_modules(package_dir)
        for directory, directories, filenames in os.walk(source_root, followlinks=False):
            directories.sort()
            for filename in sorted(filenames):
                if not filename.endswith(".ls"):
                    continue
                source_file = pathlib.Path(directory) / filename
                relative = source_file.relative_to(source_root).with_suffix("")
                module_name = ".".join(relative.parts)
                if exports is not None and module_name not in exports:
                    continue
                destination = module_index_path(index_root, module_name)
                if destination is None or destination.exists():
                    continue
                try:
                    target = source_file.relative_to(project_dir).as_posix()
                except ValueError as error:
                    raise InstallError(
                        f"package source is not lexically inside project: {source_file}"
                    ) from error
                try:
                    destination.parent.mkdir(parents=True, exist_ok=True)
                    destination.write_text(target, encoding="utf-8")
                    sync_install_path(destination, "index-sync")
                except OSError as error:
                    raise InstallError(f"cannot write module index {destination}: {error}") from error


def rebuild_module_index(project_dir, lsharp_dir, packages_dir):
    index_root = managed_child(lsharp_dir, "module-index")
    if os.path.lexists(index_root):
        if index_root.is_symlink():
            raise InstallError(f"refusing symlinked module index: {index_root}")
        if not index_root.is_dir():
            raise InstallError(f"module index is not a directory: {index_root}")
    temporary = temporary_path(lsharp_dir, "module-index")
    try:
        temporary.mkdir()
        write_module_index(project_dir, packages_dir, temporary)
        sync_install_path(temporary, "index-sync")
        if os.path.lexists(index_root):
            remove_managed_path(index_root, lsharp_dir)
        os.replace(temporary, index_root)
        sync_install_path(lsharp_dir, "index-sync")
    finally:
        if os.path.lexists(temporary):
            remove_managed_path(temporary, lsharp_dir)


def toml_string(value):
    return json.dumps(value, ensure_ascii=True)


def write_lockfile(lsharp_dir, entries):
    lock_path = managed_child(lsharp_dir, "lock.toml")
    if os.path.lexists(lock_path) and lock_path.is_symlink():
        raise InstallError(f"refusing symlinked lockfile: {lock_path}")
    if lock_path.exists() and not lock_path.is_file():
        raise InstallError(f"lockfile is not a regular file: {lock_path}")
    lines = ["# .lsharp/lock.toml -- generated; do not edit manually.", ""]
    for entry in sorted(entries, key=lambda entry: entry["name"]):
        lines.extend(
            [
                "[[package]]",
                f"name = {toml_string(entry['name'])}",
                f"version = {toml_string(entry['version'])}",
                f"source = {toml_string(entry['source'])}",
                "",
            ]
        )
    temporary = temporary_path(lsharp_dir, "lock.toml")
    try:
        temporary.write_text("\n".join(lines) + "\n", encoding="utf-8")
        sync_install_path(temporary, "lock-sync")
        os.replace(temporary, lock_path)
        sync_install_path(lsharp_dir, "lock-sync")
    except OSError as error:
        raise InstallError(f"cannot write lockfile {lock_path}: {error}") from error
    finally:
        if os.path.lexists(temporary):
            remove_managed_path(temporary, lsharp_dir)


def install(project_dir):
    config = load_toml(project_dir / "lsharp.toml")
    specs = dependency_specs(config)
    lsharp_dir = ensure_managed_directory(project_dir, ".lsharp")
    packages_dir = ensure_managed_directory(lsharp_dir, "packages")
    staging_dir = temporary_path(packages_dir, "install-txn")
    try:
        staging_dir.mkdir()
    except OSError as error:
        raise InstallError(f"cannot create install transaction staging {staging_dir}: {error}") from error
    try:
        entries = []
        pending_promotions = []
        for name, kind, value, branch, tag in specs:
            if kind == "path":
                entry, promotion = install_path_dependency(
                    packages_dir, staging_dir, name, value, project_dir
                )
                entries.append(entry)
                pending_promotions.append(promotion)
            elif kind == "git":
                entry, promotion = install_git_dependency(
                    packages_dir, staging_dir, name, value, branch, tag
                )
                entries.append(entry)
                if promotion is not None:
                    pending_promotions.append(promotion)
            else:
                entries.append(resolve_cached_version_dependency(packages_dir, name, value))

        promoted = []
        for index, (staged, destination, description) in enumerate(pending_promotions):
            sync_install_path(staged, "promotion-before-sync")
            backup = temporary_path(staging_dir, f"backup-{index}")
            backup_path = None
            if os.path.lexists(destination):
                try:
                    os.replace(destination, backup)
                except OSError as error:
                    rollback_promotions(promoted, packages_dir)
                    raise InstallError(
                        f"cannot back up package destination {destination}: {error}"
                    ) from error
                backup_path = backup
            if os.environ.get("LSHARP_TEST_INSTALL_FAILPOINT") == f"promotion:{index}":
                if backup_path is not None and os.path.lexists(backup_path):
                    os.replace(backup_path, destination)
                rollback_promotions(promoted, packages_dir)
                raise InstallError(f"test-only package promotion failpoint at index {index}")
            try:
                os.replace(staged, destination)
            except OSError as error:
                if backup_path is not None and os.path.lexists(backup_path):
                    os.replace(backup_path, destination)
                rollback_promotions(promoted, packages_dir)
                raise InstallError(f"cannot promote package {destination}: {error}") from error
            try:
                sync_install_path(destination, "promotion-after-sync")
                sync_install_path(destination.parent, "promotion-after-sync")
            except InstallError:
                remove_managed_path(destination, packages_dir)
                if backup_path is not None and os.path.lexists(backup_path):
                    os.replace(backup_path, destination)
                rollback_promotions(promoted, packages_dir)
                raise
            promoted.append((destination, backup_path))
            print(f"installed {description}")
        lock_path = managed_child(lsharp_dir, "lock.toml")
        index_root = managed_child(lsharp_dir, "module-index")
        if lock_path.is_symlink():
            raise InstallError(f"refusing symlinked lockfile: {lock_path}")
        if lock_path.exists() and not lock_path.is_file():
            raise InstallError(f"lockfile is not a regular file: {lock_path}")
        if os.path.lexists(index_root):
            if index_root.is_symlink():
                raise InstallError(f"refusing symlinked module index: {index_root}")
            if not index_root.is_dir():
                raise InstallError(f"module index is not a directory: {index_root}")

        metadata_backups = []
        try:
            metadata_backups.append(
                (lock_path, backup_metadata(lock_path, temporary_path(staging_dir, "metadata-lock")))
            )
            metadata_backups.append(
                (
                    index_root,
                    backup_metadata(index_root, temporary_path(staging_dir, "metadata-index")),
                )
            )
        except InstallError:
            rollback_install_state(promoted, metadata_backups, packages_dir, lsharp_dir)
            raise

        if os.environ.get("LSHARP_TEST_INSTALL_FAILPOINT") == "lock":
            rollback_install_state(promoted, metadata_backups, packages_dir, lsharp_dir)
            raise InstallError("test-only lockfile commit failpoint")
        try:
            write_lockfile(lsharp_dir, entries)
        except InstallError:
            rollback_install_state(promoted, metadata_backups, packages_dir, lsharp_dir)
            raise

        if os.environ.get("LSHARP_TEST_INSTALL_FAILPOINT") == "index":
            rollback_install_state(promoted, metadata_backups, packages_dir, lsharp_dir)
            raise InstallError("test-only module-index commit failpoint")
        try:
            rebuild_module_index(project_dir, lsharp_dir, packages_dir)
        except InstallError:
            rollback_install_state(promoted, metadata_backups, packages_dir, lsharp_dir)
            raise
        print(f"installed {len(entries)} dependency entries")
    finally:
        if os.path.lexists(staging_dir):
            remove_managed_path(staging_dir, packages_dir)


def main(argv=None):
    arguments = parse_arguments(argv)
    try:
        ensure_supported_host()
        install(require_project_dir(arguments.project_dir))
    except InstallError as error:
        print(f"error: {error}", file=sys.stderr)
        return 1
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
