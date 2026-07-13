#!/usr/bin/env python3

import os
import pathlib
import json
import subprocess
import sys
import tempfile
import unittest


ROOT = pathlib.Path(__file__).resolve().parents[2]
INSTALLER = ROOT / "scripts" / "native-selfhost-install.py"


class NativeSelfhostInstallTest(unittest.TestCase):
    def write_package(self, root, name, version, modules, exports=None):
        root.mkdir(parents=True, exist_ok=True)
        config = ["[project]", f'name = "{name}"', f'version = "{version}"']
        if exports is not None:
            config.extend(
                [
                    "",
                    "[project.exports]",
                    "modules = [",
                    *[f'  "{module}",' for module in exports],
                    "]",
                ]
            )
        (root / "lsharp.toml").write_text("\n".join(config) + "\n", encoding="utf-8")
        for relative, content in modules.items():
            source = root / "src" / relative
            source.parent.mkdir(parents=True, exist_ok=True)
            source.write_text(content, encoding="utf-8")

    def poison_host_commands(self, root, command_names=("cargo", "lsharp")):
        poison_bin = root / "poison-bin"
        poison_bin.mkdir()
        marker = root / "host-command-ran"
        for command_name in command_names:
            command = poison_bin / command_name
            command.write_text(
                "#!/usr/bin/env python3\n"
                "import os\n"
                "import pathlib\n"
                "pathlib.Path(os.environ['HOST_COMMAND_RAN']).touch()\n"
                "raise SystemExit(97)\n",
                encoding="ascii",
            )
            os.chmod(command, 0o755)
        environment = os.environ.copy()
        environment.update(
            {
                "HOST_COMMAND_RAN": str(marker),
                "PATH": str(poison_bin) + os.pathsep + environment["PATH"],
            }
        )
        return environment, marker

    def run_installer(self, project, environment=None):
        return subprocess.run(
            [sys.executable, str(INSTALLER), "--project-dir", str(project)],
            capture_output=True,
            text=True,
            env=environment,
            check=False,
        )

    def run_git(self, directory, *args):
        subprocess.run(
            ["git", *args],
            cwd=directory,
            capture_output=True,
            text=True,
            check=True,
        )

    def read_lock(self, project):
        entries = []
        current = None
        for raw_line in (project / ".lsharp" / "lock.toml").read_text(encoding="utf-8").splitlines():
            line = raw_line.strip()
            if line == "[[package]]":
                current = {}
                entries.append(current)
                continue
            if not line or current is None:
                continue
            key, value = line.split("=", 1)
            current[key.strip()] = json.loads(value.strip())
        return entries

    def test_installs_path_dependency_with_exported_module_index_and_lock(self):
        with tempfile.TemporaryDirectory() as temporary_directory:
            root = pathlib.Path(temporary_directory)
            project = root / "project"
            project.mkdir()
            dependency = root / "geometry"
            self.write_package(
                dependency,
                "geometry",
                "1.4.0",
                {
                    "Geometry.ls": "(module Geometry)\n",
                    "Geometry/Vec2.ls": "(module Geometry.Vec2)\n",
                    "Hidden.ls": "(module Hidden)\n",
                },
                exports=("Geometry", "Geometry.Vec2"),
            )
            (project / "lsharp.toml").write_text(
                "[dependencies.geometry]\npath = \"../geometry\"\n",
                encoding="utf-8",
            )
            environment, marker = self.poison_host_commands(
                root, command_names=("cargo", "lsharp", "git")
            )

            result = self.run_installer(project, environment)

            self.assertEqual(result.returncode, 0, result.stderr)
            self.assertFalse(marker.exists(), "Cargo/lsharp fallback must not run")
            installed = next((project / ".lsharp" / "packages").glob("geometry-*"))
            self.assertTrue(installed.is_symlink())
            self.assertEqual(installed.resolve(), dependency.resolve())
            index_root = project / ".lsharp" / "module-index"
            self.assertEqual(
                (index_root / "Geometry.path").read_text(encoding="utf-8").strip(),
                f".lsharp/packages/{installed.name}/src/Geometry.ls",
            )
            self.assertEqual(
                (index_root / "Geometry" / "Vec2.path").read_text(encoding="utf-8").strip(),
                f".lsharp/packages/{installed.name}/src/Geometry/Vec2.ls",
            )
            self.assertFalse((index_root / "Hidden.path").exists())
            self.assertEqual(
                self.read_lock(project),
                [
                    {
                        "name": "geometry",
                        "version": "1.4.0",
                        "source": f"path:{dependency.resolve()}",
                    }
                ],
            )

    def test_clones_local_git_dependencies_with_branch_and_tag(self):
        with tempfile.TemporaryDirectory() as temporary_directory:
            root = pathlib.Path(temporary_directory)
            repository = root / "repository"
            repository.mkdir()
            self.run_git(repository, "init", "-q")
            self.run_git(repository, "config", "user.email", "native-install@example.invalid")
            self.run_git(repository, "config", "user.name", "Native Install Test")
            self.write_package(
                repository,
                "git-package",
                "1.0.0",
                {"Tagged.ls": "(module Tagged)\n"},
            )
            self.run_git(repository, "add", ".")
            self.run_git(repository, "commit", "-qm", "initial package")
            self.run_git(repository, "tag", "v1.0.0")
            self.run_git(repository, "checkout", "-qb", "feature")
            self.write_package(
                repository,
                "git-package",
                "1.1.0",
                {"Feature.ls": "(module Feature)\n"},
            )
            self.run_git(repository, "add", ".")
            self.run_git(repository, "commit", "-qm", "feature package")

            project = root / "project"
            project.mkdir()
            git_url = repository.as_uri()
            (project / "lsharp.toml").write_text(
                "[dependencies.branch-lib]\n"
                f'git = "{git_url}"\n'
                'branch = "feature"\n\n'
                "[dependencies.tag-lib]\n"
                f'git = "{git_url}"\n'
                'tag = "v1.0.0"\n',
                encoding="utf-8",
            )
            environment, marker = self.poison_host_commands(root)

            result = self.run_installer(project, environment)

            self.assertEqual(result.returncode, 0, result.stderr)
            self.assertFalse(marker.exists(), "Cargo/lsharp fallback must not run")
            packages = project / ".lsharp" / "packages"
            branch_package = next(packages.glob("branch-lib-*"))
            tag_package = next(packages.glob("tag-lib-*"))
            self.assertIn('version = "1.1.0"', (branch_package / "lsharp.toml").read_text(encoding="utf-8"))
            self.assertIn('version = "1.0.0"', (tag_package / "lsharp.toml").read_text(encoding="utf-8"))
            self.assertEqual(
                self.read_lock(project),
                [
                    {
                        "name": "branch-lib",
                        "version": "1.1.0",
                        "source": f"git:{git_url}?branch=feature",
                    },
                    {
                        "name": "tag-lib",
                        "version": "1.0.0",
                        "source": f"git:{git_url}?tag=v1.0.0",
                    },
                ],
            )

    def test_path_dependency_without_project_version_locks_zero_version(self):
        with tempfile.TemporaryDirectory() as temporary_directory:
            root = pathlib.Path(temporary_directory)
            project = root / "project"
            project.mkdir()
            dependency = root / "no-version"
            self.write_package(
                dependency,
                "no-version",
                "1.0.0",
                {"NoVersion.ls": "(module NoVersion)\n"},
            )
            (dependency / "lsharp.toml").write_text(
                "[project]\nname = \"no-version\"\n", encoding="utf-8"
            )
            (project / "lsharp.toml").write_text(
                "[dependencies.no-version]\npath = \"../no-version\"\n",
                encoding="utf-8",
            )
            environment, marker = self.poison_host_commands(
                root, command_names=("cargo", "lsharp", "git")
            )

            result = self.run_installer(project, environment)

            self.assertEqual(result.returncode, 0, result.stderr)
            self.assertFalse(marker.exists(), "Cargo/lsharp fallback must not run")
            self.assertEqual(
                self.read_lock(project),
                [
                    {
                        "name": "no-version",
                        "version": "0.0.0",
                        "source": f"path:{dependency.resolve()}",
                    }
                ],
            )

    def test_resolves_cached_plain_exact_and_minimum_semver_constraints(self):
        with tempfile.TemporaryDirectory() as temporary_directory:
            root = pathlib.Path(temporary_directory)
            project = root / "project"
            packages = project / ".lsharp" / "packages"
            self.write_package(packages / "compat-a", "compat", "1.2.0", {"Compat.ls": ""})
            self.write_package(packages / "compat-b", "compat", "1.9.9", {"Compat.ls": ""})
            self.write_package(packages / "compat-c", "compat", "2.0.0", {"Compat.ls": ""})
            self.write_package(packages / "exact-a", "exact", "1.4.0", {"Exact.ls": ""})
            self.write_package(packages / "exact-b", "exact", "1.4.1", {"Exact.ls": ""})
            self.write_package(packages / "minimum-a", "minimum", "1.5.0", {"Minimum.ls": ""})
            self.write_package(packages / "minimum-b", "minimum", "3.0.0", {"Minimum.ls": ""})
            self.write_package(packages / "precompat-a", "precompat", "0.2.4", {"Precompat.ls": ""})
            self.write_package(packages / "precompat-b", "precompat", "0.3.0", {"Precompat.ls": ""})
            (project / "lsharp.toml").write_text(
                "[dependencies]\n"
                'compat = "1.2.0"\n'
                'exact = "=1.4.0"\n'
                'minimum = ">=1.5.0"\n'
                'precompat = "0.2.0"\n',
                encoding="utf-8",
            )
            environment, marker = self.poison_host_commands(
                root, command_names=("cargo", "lsharp", "git")
            )

            result = self.run_installer(project, environment)

            self.assertEqual(result.returncode, 0, result.stderr)
            self.assertFalse(marker.exists(), "Cargo/lsharp fallback must not run")
            self.assertEqual(
                self.read_lock(project),
                [
                    {"name": "compat", "version": "1.9.9", "source": "registry:default"},
                    {"name": "exact", "version": "1.4.0", "source": "registry:default"},
                    {"name": "minimum", "version": "3.0.0", "source": "registry:default"},
                    {"name": "precompat", "version": "0.2.4", "source": "registry:default"},
                ],
            )

    def test_empty_dependencies_rebuilds_module_index_and_writes_empty_lock(self):
        with tempfile.TemporaryDirectory() as temporary_directory:
            root = pathlib.Path(temporary_directory)
            project = root / "project"
            cached = project / ".lsharp" / "packages" / "cached-12345678"
            self.write_package(
                cached,
                "cached",
                "1.0.0",
                {"Existing.ls": "(module Existing)\n"},
            )
            index_root = project / ".lsharp" / "module-index"
            index_root.mkdir(parents=True)
            (index_root / "Stale.path").write_text("stale\n", encoding="utf-8")
            (project / "lsharp.toml").write_text("[dependencies]\n", encoding="utf-8")
            environment, marker = self.poison_host_commands(root)

            result = self.run_installer(project, environment)

            self.assertEqual(result.returncode, 0, result.stderr)
            self.assertFalse(marker.exists(), "Cargo/lsharp fallback must not run")
            self.assertFalse((index_root / "Stale.path").exists())
            self.assertEqual(
                (index_root / "Existing.path").read_text(encoding="utf-8").strip(),
                ".lsharp/packages/cached-12345678/src/Existing.ls",
            )
            self.assertEqual(self.read_lock(project), [])

    def test_refuses_a_symlinked_lsharp_directory_without_touching_external_state(self):
        with tempfile.TemporaryDirectory() as temporary_directory:
            root = pathlib.Path(temporary_directory)
            project = root / "project"
            project.mkdir()
            external = root / "external-state"
            external.mkdir()
            sentinel = external / "sentinel"
            sentinel.write_text("keep\n", encoding="utf-8")
            (project / ".lsharp").symlink_to(external, target_is_directory=True)
            (project / "lsharp.toml").write_text("[dependencies]\n", encoding="utf-8")
            environment, marker = self.poison_host_commands(root)

            result = self.run_installer(project, environment)

            self.assertNotEqual(result.returncode, 0)
            self.assertIn("refusing symlinked managed directory", result.stderr)
            self.assertFalse(marker.exists(), "Cargo/lsharp fallback must not run")
            self.assertEqual(sentinel.read_text(encoding="utf-8"), "keep\n")

    def test_requires_an_explicit_existing_project_dir(self):
        result = subprocess.run(
            [sys.executable, str(INSTALLER)],
            capture_output=True,
            text=True,
            check=False,
        )

        self.assertNotEqual(result.returncode, 0)
        self.assertIn("--project-dir", result.stderr)


if __name__ == "__main__":
    unittest.main(verbosity=2)
