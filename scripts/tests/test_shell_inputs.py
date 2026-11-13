"""Tests for scripts/release/shell_inputs.py: which files make a release whole-app only, and the
guard that keeps that list complete. `changed` is exercised against throwaway Git repositories."""

import json
import subprocess
import sys
import tempfile
import unittest
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parents[1] / "release"))
import shell_inputs as shell  # noqa: E402

INPUTS = ["src", "build.rs", "ui/theme", "ui/config/background.json", "nsh.json", "Cargo.toml", "Cargo.lock"]


def write(root: Path, relative: str, text: str) -> None:
    path = root / relative
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text(text, encoding="utf-8")


def git(root: Path, *args: str) -> str:
    result = subprocess.run(["git", "-C", str(root), *args], capture_output=True, text=True, check=True)
    return result.stdout.strip()


def repository() -> Path:
    root = Path(tempfile.mkdtemp())
    git(root, "init", "-q", "-b", "main")
    git(root, "config", "user.email", "t@example.com")
    git(root, "config", "user.name", "t")
    write(root, "src/lib.rs", "// shell\n")
    write(root, "build.rs", "fn main() {}\n")
    write(root, "ui/theme/builtin/default/theme.json", "{}\n")
    write(root, "ui/config/background.json", "{}\n")
    write(root, "ui/features/Page.tsx", "export {};\n")
    write(root, "i18n/messages/en.json", "{}\n")
    write(root, "App.tsx", "export {};\n")
    write(root, "Cargo.toml", '[package]\nname = "example"\nversion = "1.0.0"\n[dependencies]\ndep = "2"\n')
    write(root, "Cargo.lock", 'version = 4\n[[package]]\nname = "example"\nversion = "1.0.0"\ndependencies = ["dep"]\n[[package]]\nname = "dep"\nversion = "2.0.0"\nsource = "registry+https://example.com"\nchecksum = "abc"\n')
    write(root, "nsh.json", json.dumps({"id": "com.example.app", "version": "1.0.0", "build": 1, "rust": "."}))
    git(root, "add", "-A")
    git(root, "commit", "-qm", "base")
    git(root, "tag", "v1.0.0")
    return root


class Reading(unittest.TestCase):
    def test_the_real_list_is_readable_and_names_the_known_embedded_files(self):
        inputs = shell.read_inputs()
        for required in ["src", "build.rs", "ui/theme", "ui/config/background.json", "nsh.json"]:
            self.assertIn(required, inputs)

    def test_comments_blanks_and_trailing_slashes_are_ignored(self):
        path = Path(tempfile.mkdtemp()) / "list.txt"
        path.write_text("# c\n\nsrc/\n  build.rs  \n", encoding="utf-8")
        self.assertEqual(shell.read_inputs(path), ["src", "build.rs"])

    def test_an_empty_list_is_an_error_not_an_empty_shell(self):
        path = Path(tempfile.mkdtemp()) / "list.txt"
        path.write_text("# nothing\n", encoding="utf-8")
        with self.assertRaises(shell.ShellInputsError):
            shell.read_inputs(path)

    def test_a_directory_covers_its_children_but_not_a_sibling_with_the_same_prefix(self):
        self.assertTrue(shell.covered("src/a/b.rs", ["src"]))
        self.assertTrue(shell.covered("src", ["src"]))
        self.assertFalse(shell.covered("src2/a.rs", ["src"]))
        self.assertFalse(shell.covered("ui/theme.json", ["ui/theme"]))


class Changed(unittest.TestCase):
    def changed(self, root: Path) -> list[str]:
        return shell.changed_files(root, "v1.0.0", INPUTS)

    def test_a_tsx_only_change_is_not_a_shell_change(self):
        root = repository()
        write(root, "ui/features/Page.tsx", "export const x = 1;\n")
        write(root, "i18n/messages/en.json", '{"a":"b"}\n')
        write(root, "App.tsx", "export const y = 2;\n")
        git(root, "add", "-A")
        git(root, "commit", "-qm", "tsx")
        self.assertEqual(self.changed(root), [])

    def test_each_kind_of_shell_input_counts(self):
        for relative in ["src/lib.rs", "build.rs", "ui/theme/builtin/default/theme.json", "ui/config/background.json"]:
            root = repository()
            write(root, relative, "// changed\n")
            git(root, "add", "-A")
            git(root, "commit", "-qm", "shell")
            self.assertEqual(self.changed(root), [relative], relative)

    def test_a_new_rust_file_counts(self):
        root = repository()
        write(root, "src/new.rs", "// new\n")
        git(root, "add", "-A")
        git(root, "commit", "-qm", "new")
        self.assertEqual(self.changed(root), ["src/new.rs"])

    def test_a_deleted_shell_file_counts(self):
        root = repository()
        (root / "src/lib.rs").unlink()
        git(root, "add", "-A")
        git(root, "commit", "-qm", "delete")
        self.assertEqual(self.changed(root), ["src/lib.rs"])

    def test_nsh_json_version_and_build_alone_are_not_a_shell_change(self):
        root = repository()
        write(root, "nsh.json", json.dumps({"id": "com.example.app", "version": "1.1.0", "build": 2, "rust": "."}))
        git(root, "add", "-A")
        git(root, "commit", "-qm", "bump")
        self.assertEqual(self.changed(root), [])

    def test_any_other_nsh_json_field_is_a_shell_change(self):
        root = repository()
        write(root, "nsh.json", json.dumps({"id": "com.example.app", "version": "1.1.0", "build": 2, "rust": ".", "macos": {"x": 1}}))
        git(root, "add", "-A")
        git(root, "commit", "-qm", "permissions")
        self.assertEqual(self.changed(root), ["nsh.json"])

    def test_patch_release_with_all_version_files_can_ship_a_code_package(self):
        root = repository()
        for name in ("Cargo.toml", "Cargo.lock", "nsh.json"):
            path = root / name
            path.write_text(path.read_text().replace("1.0.0", "1.0.1"))
        config = json.loads((root / "nsh.json").read_text())
        config["build"] += 1
        write(root, "nsh.json", json.dumps(config))
        write(root, "App.tsx", "export const changed = true;\n")
        git(root, "add", "-A")
        git(root, "commit", "-qm", "patch release")
        self.assertEqual(self.changed(root), [])
        result = subprocess.run([sys.executable, str(Path(shell.__file__)), "changed", "--version", "1.0.1",
                                 "--strict", "--root", str(root)], capture_output=True, text=True)
        self.assertEqual(result.returncode, 0, result.stderr)
        self.assertIn("shell_changed=false", result.stdout)

    def test_cargo_dependency_and_configuration_changes_still_change_the_shell(self):
        for name, before, after in [("Cargo.toml", 'dep = "2"', 'dep = "3"'),
                                    ("Cargo.toml", 'name = "example"', 'name = "renamed"'),
                                    ("Cargo.lock", 'version = "2.0.0"', 'version = "2.0.1"'),
                                    ("Cargo.lock", 'checksum = "abc"', 'checksum = "def"'),
                                    ("Cargo.lock", 'dependencies = ["dep"]', 'dependencies = []')]:
            with self.subTest(name=name, after=after):
                root = repository()
                path = root / name
                path.write_text(path.read_text().replace(before, after))
                self.assertEqual(self.changed(root), [name])

    def test_invalid_or_inconsistent_cargo_identity_is_rejected(self):
        for name, content in [("Cargo.toml", "invalid ["), ("Cargo.lock", 'version = 4\n')]:
            with self.subTest(name=name):
                root = repository()
                write(root, name, content)
                with self.assertRaises(shell.ShellInputsError):
                    self.changed(root)

    def test_field_order_in_nsh_json_does_not_matter(self):
        root = repository()
        write(root, "nsh.json", json.dumps({"rust": ".", "build": 1, "version": "1.0.0", "id": "com.example.app"}))
        git(root, "add", "-A")
        git(root, "commit", "-qm", "reorder")
        self.assertEqual(self.changed(root), [])

    def test_an_unknown_base_is_an_error_not_a_free_pass(self):
        with self.assertRaises(shell.ShellInputsError):
            shell.changed_files(repository(), "v9.9.9", INPUTS)


class Epoch(unittest.TestCase):
    def test_the_previous_tag_is_the_highest_formal_tag_below_the_version(self):
        tags = ["v0.9.0", "v0.10.0", "v0.1.5", "v0.10.1", "v1.0.0-rc.1", "latest", "v0.10"]
        self.assertEqual(shell.previous_release_tag(tags, "0.10.1"), "v0.10.0")
        self.assertEqual(shell.previous_release_tag(tags, "0.10.2"), "v0.10.1")
        self.assertEqual(shell.previous_release_tag(tags, "0.9.0"), "v0.1.5")
        self.assertIsNone(shell.previous_release_tag(tags, "0.1.0"))
        self.assertIsNone(shell.previous_release_tag([], "1.0.0"))

    def test_pre_release_and_malformed_tags_are_never_the_previous_release(self):
        # A pre-release tag is not a release (docs/update.md, D7); using one as the base would
        # compare against code nobody was ever offered.
        for tag in ["v0.5.0-alpha.1", "v0.5.0-rc.2", "v0.5.0+build", "v0.5.0.1", "v0.5", "V0.5.0", "v-0.5.0"]:
            self.assertIsNone(shell.previous_release_tag([tag], "9.0.0"), tag)
        self.assertEqual(shell.previous_release_tag(["v0.5.0-rc.2", "v0.4.0"], "9.0.0"), "v0.4.0")

    def test_numeric_not_text_order(self):
        self.assertEqual(shell.previous_release_tag(["v0.9.0", "v0.10.0"], "0.11.0"), "v0.10.0")

    def test_a_shell_change_must_raise_the_major_or_minor_line(self):
        self.assertIsNone(shell.epoch_problem("v0.1.4", "0.2.0"))
        self.assertIsNone(shell.epoch_problem("v0.9.3", "1.0.0"))
        for version in ["0.1.5", "0.1.4", "0.1.0"]:
            self.assertIn("minor", shell.epoch_problem("v0.1.4", version), version)

    def test_a_version_that_is_not_a_version_is_an_error(self):
        with self.assertRaises(shell.ShellInputsError):
            shell.version_key("1.2")


class Embedded(unittest.TestCase):
    def project(self, source: str, inputs_covering: bool = True) -> Path:
        root = Path(tempfile.mkdtemp())
        write(root, "src/lib.rs", source)
        write(root, "ui/config/background.json", "{}\n")
        write(root, "ui/extra/data.json", "{}\n")
        return root

    def test_an_embedded_file_outside_the_list_is_reported(self):
        root = self.project('const X: &str = include_str!("../ui/extra/data.json");\n')
        problems = shell.check_embedded(root, ["src", "ui/config/background.json"])
        self.assertEqual(len(problems), 1)
        self.assertIn("ui/extra/data.json", problems[0])

    def test_an_embedded_file_inside_the_list_passes(self):
        root = self.project('const X: &str = include_str!("../ui/config/background.json");\n')
        self.assertEqual(shell.check_embedded(root, ["src", "ui/config/background.json"]), [])

    def test_include_bytes_and_include_are_recognised(self):
        root = self.project('static A: &[u8] = include_bytes!("../ui/extra/data.json");\ninclude!("../ui/extra/data.json");\n')
        self.assertEqual(len(shell.check_embedded(root, ["src"])), 2)

    def test_test_only_embeds_are_not_shipped_and_are_ignored(self):
        root = self.project('fn f() {}\n#[cfg(test)]\nmod tests { const X: &str = include_str!("../ui/extra/data.json"); }\n')
        self.assertEqual(shell.check_embedded(root, ["src"]), [])

    def test_the_real_repository_is_fully_covered(self):
        root = Path(__file__).resolve().parents[2]
        self.assertEqual(shell.check_embedded(root, shell.read_inputs()), [])


class Command(unittest.TestCase):
    def run_cli(self, *args: str):
        return subprocess.run([sys.executable, str(Path(shell.__file__)), *args], capture_output=True, text=True)

    def test_no_previous_release_means_a_whole_app_release(self):
        result = self.run_cli("changed", "--base", "")
        self.assertEqual(result.returncode, 0)
        self.assertIn("shell_changed=true", result.stdout)

    def release_with_shell_change(self, version):
        root = repository()
        write(root, "src/lib.rs", "// rust changed\n")
        git(root, "add", "-A")
        git(root, "commit", "-qm", "rust")
        return self.run_cli("changed", "--version", version, "--strict", "--root", str(root)), root

    def test_a_shell_change_on_the_same_line_fails_a_release_and_only_warns_a_rehearsal(self):
        result, root = self.release_with_shell_change("1.0.1")
        self.assertEqual(result.returncode, 1)
        self.assertIn("shell_changed=true", result.stdout)
        self.assertIn("::error::", result.stderr)
        rehearsal = self.run_cli("changed", "--version", "1.0.1", "--root", str(root))
        self.assertEqual(rehearsal.returncode, 0)
        self.assertIn("::warning::", rehearsal.stderr)

    def test_a_shell_change_that_raises_the_minor_passes(self):
        result, _ = self.release_with_shell_change("1.1.0")
        self.assertEqual(result.returncode, 0, result.stderr)
        self.assertIn("previous_tag=v1.0.0", result.stdout)

    def test_a_tsx_only_change_needs_no_minor_bump(self):
        root = repository()
        write(root, "ui/features/Page.tsx", "export const x = 1;\n")
        git(root, "add", "-A")
        git(root, "commit", "-qm", "tsx")
        result = self.run_cli("changed", "--version", "1.0.1", "--strict", "--root", str(root))
        self.assertEqual(result.returncode, 0, result.stderr)
        self.assertIn("shell_changed=false", result.stdout)

    def test_the_command_prints_a_flag_the_workflow_can_read(self):
        root = repository()
        write(root, "ui/features/Page.tsx", "export const x = 1;\n")
        git(root, "add", "-A")
        git(root, "commit", "-qm", "tsx")
        result = self.run_cli("changed", "--base", "v1.0.0", "--root", str(root))
        self.assertEqual(result.returncode, 0, result.stderr)
        self.assertEqual(result.stdout.split(), ["previous_tag=v1.0.0", "shell_changed=false"])


if __name__ == "__main__":
    unittest.main()
