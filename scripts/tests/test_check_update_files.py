"""Tests for scripts/release/check_update_files.py: what makes a release's update files refusable.
Fixtures are written by hand in the shape `nsh update-manifest` produces, so no `nsh` is needed."""

import hashlib
import json
import sys
import tempfile
import unittest
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parents[1] / "release"))
import check_update_files as check  # noqa: E402

APP = "com.example.app"
ARCHIVE = "App-1.2.0-macos-aarch64.tar.gz"
PACKAGE = "com.example.app-7.nshpkg"


def sha(data: bytes) -> str:
    return hashlib.sha256(data).hexdigest()


def release(*, package=True, signature=True, version="1.2.0", build=7, mutate=None):
    """A consistent release directory. `mutate(manifest, directory)` breaks it in one place."""
    directory = Path(tempfile.mkdtemp())
    archive, package_bytes = b"archive bytes", b"package bytes"
    (directory / ARCHIVE).write_bytes(archive)
    manifest = {
        "id": APP,
        "version": version,
        "platforms": {"macos-aarch64": {"url": ARCHIVE, "sha256": sha(archive), "size": len(archive)}},
    }
    if package:
        (directory / PACKAGE).write_bytes(package_bytes)
        manifest["package"] = {
            "version": version, "build": build, "url": PACKAGE,
            "sha256": sha(package_bytes), "size": len(package_bytes), "minRuntime": "0.5.0",
        }
    if mutate:
        mutate(manifest, directory)
    (directory / "latest.json").write_text(json.dumps(manifest))
    if signature:
        (directory / "latest.json.sig").write_text("c2lnbmF0dXJl")
    return directory


def problems(directory, shell_changed=False, version="1.2.0", build=7):
    return check.check_files(directory, app_id=APP, version=version, build=build, shell_changed=shell_changed)


class Consistent(unittest.TestCase):
    def test_a_release_with_a_package_passes(self):
        self.assertEqual(problems(release()), [])

    def test_a_whole_app_only_release_passes(self):
        self.assertEqual(problems(release(package=False), shell_changed=True), [])


class Signature(unittest.TestCase):
    def test_a_missing_or_empty_signature_is_refused(self):
        self.assertTrue(any("sig" in p for p in problems(release(signature=False))))
        directory = release()
        (directory / "latest.json.sig").write_text("")
        self.assertTrue(any("sig" in p for p in problems(directory)))

    def test_a_missing_manifest_is_refused(self):
        directory = release()
        (directory / "latest.json").unlink()
        self.assertEqual(problems(directory), ["latest.json is missing."])


class Identity(unittest.TestCase):
    def test_another_app_or_version_is_refused(self):
        self.assertTrue(any("another" in p or "'com.other'" in p for p in problems(release(mutate=lambda m, d: m.update(id="com.other")))))
        self.assertTrue(any("9.9.9" in p for p in problems(release(mutate=lambda m, d: m.update(version="9.9.9")))))


class Archive(unittest.TestCase):
    def test_a_changed_archive_is_refused(self):
        directory = release()
        (directory / ARCHIVE).write_bytes(b"swapped after signing")
        found = problems(directory)
        self.assertTrue(any("sha256" in p for p in found))
        self.assertTrue(any("size" in p for p in found))

    def test_a_missing_archive_is_refused(self):
        directory = release()
        (directory / ARCHIVE).unlink()
        self.assertTrue(any("not in the release" in p for p in problems(directory)))

    def test_only_the_one_platform_may_be_listed(self):
        def add(manifest, directory):
            manifest["platforms"]["linux-x86_64"] = manifest["platforms"]["macos-aarch64"]
        self.assertTrue(any("exactly" in p for p in problems(release(mutate=add))))

    def test_urls_must_be_bare_file_names(self):
        for url in ["https://evil.example/a.tar.gz", "../a.tar.gz", "sub/a.tar.gz", "..", "C:\\a"]:
            def set_url(manifest, directory, url=url):
                manifest["platforms"]["macos-aarch64"]["url"] = url
            self.assertTrue(any("file name" in p for p in problems(release(mutate=set_url))), url)


class Package(unittest.TestCase):
    def test_a_changed_package_is_refused(self):
        directory = release()
        (directory / PACKAGE).write_bytes(b"swapped")
        self.assertTrue(any("package sha256" in p for p in problems(directory)))

    def test_the_package_must_carry_this_build_and_version(self):
        self.assertTrue(any("build" in p for p in problems(release(), build=8)))
        self.assertTrue(any("version" in p for p in problems(release(mutate=lambda m, d: m["package"].update(version="0.0.1")))))

    def test_a_package_without_min_runtime_is_refused(self):
        self.assertTrue(any("minRuntime" in p for p in problems(release(mutate=lambda m, d: m["package"].update(minRuntime="")))))

    def test_a_release_that_must_have_a_package_but_has_none_is_refused(self):
        self.assertTrue(any("must list a code package" in p for p in problems(release(package=False))))

    def test_a_stray_second_package_is_refused(self):
        directory = release()
        (directory / "other-8.nshpkg").write_bytes(b"x")
        self.assertTrue(any("Exactly one" in p for p in problems(directory)))


class ShellChanged(unittest.TestCase):
    def test_a_whole_app_release_must_not_list_or_contain_a_package(self):
        found = problems(release(package=True), shell_changed=True)
        self.assertTrue(any("lists a code package" in p for p in found))
        self.assertTrue(any("may be released" in p for p in found))

    def test_a_package_file_without_a_manifest_entry_is_still_refused(self):
        directory = release(package=False)
        (directory / PACKAGE).write_bytes(b"x")
        self.assertTrue(any("may be released" in p for p in problems(directory, shell_changed=True)))


class Command(unittest.TestCase):
    def test_the_command_reads_the_project_and_reports_every_problem(self):
        import subprocess
        directory = release(package=False)
        entry = Path(tempfile.mkdtemp())
        (entry / "nsh.json").write_text(json.dumps({"id": APP, "version": "1.2.0", "build": 7}))
        run = lambda flag: subprocess.run([sys.executable, str(Path(check.__file__)), str(directory), "--entry", str(entry), "--shell-changed", flag], capture_output=True, text=True)
        ok = run("true")
        self.assertEqual(ok.returncode, 0, ok.stderr)
        bad = run("false")
        self.assertEqual(bad.returncode, 1)
        self.assertIn("must list a code package", bad.stderr)


if __name__ == "__main__":
    unittest.main()
