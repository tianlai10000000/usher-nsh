import json
import sys
import tempfile
import unittest
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parents[1]))
import release_gate as gate  # noqa: E402

CHANGELOG = """# Changelog

## Unreleased
- work in progress

## [1.2.0] - 2026-11-11
### Added
- A thing.

## 1.1.0
- Older.
"""


def project(version="1.2.0", cargo="1.2.0", build=5, changelog=CHANGELOG):
    root = Path(tempfile.mkdtemp())
    (root / "nsh.json").write_text(json.dumps({"version": version, "build": build}))
    (root / "Cargo.toml").write_text(f'[package]\nname = "x"\nversion = "{cargo}"\n\n[dependencies]\nserde = {{ version = "1" }}\n')
    (root / "CHANGELOG.md").write_text(changelog)
    return root


class Tags(unittest.TestCase):
    def test_accepts_semver_tags_only(self):
        for tag, version in [("v1.2.3", "1.2.3"), ("v0.0.1-alpha.1", "0.0.1-alpha.1"), ("v1.0.0-rc.1+build.5", "1.0.0-rc.1+build.5")]:
            self.assertEqual(gate.parse_tag(tag), version)
        for tag in ["1.2.3", "v1.2", "v01.2.3", "vlatest", "v1.2.3.4", "release-1", "v", "V1.2.3", "v1.2.3-"]:
            with self.assertRaises(gate.GateError, msg=tag):
                gate.parse_tag(tag)

    def test_prerelease_versions_use_the_preview_channel(self):
        self.assertEqual(gate.channel_for("1.2.3"), "stable")
        self.assertEqual(gate.channel_for("1.2.3-alpha.1"), "preview")
        self.assertEqual(gate.channel_for("1.2.3+meta"), "stable")


class Events(unittest.TestCase):
    ok = {"ref": "refs/tags/v1", "created": True, "deleted": False, "forced": False, "before": gate.ZERO}

    def test_only_a_new_tag_passes(self):
        gate.check_event(self.ok, "refs/tags/v1")
        for change in [{"created": False}, {"deleted": True}, {"forced": True}, {"before": "a" * 40}, {"ref": "refs/tags/v2"}, {"created": None}]:
            with self.assertRaises(gate.GateError, msg=str(change)):
                gate.check_event({**self.ok, **change}, "refs/tags/v1")


class Files(unittest.TestCase):
    def test_a_consistent_project_passes_and_returns_its_notes(self):
        result = gate.decide("v1.2.0", project(), previous_build=4)
        self.assertEqual((result["version"], result["channel"], result["build"], result["prerelease"]), ("1.2.0", "stable", "5", "false"))
        self.assertIn("A thing.", result["notes"])
        self.assertNotIn("Older", result["notes"])
        self.assertNotIn("work in progress", result["notes"])

    def test_every_version_mismatch_is_refused(self):
        for kwargs in [{"version": "1.2.1"}, {"cargo": "1.2.1"}, {"version": "1.2.1", "cargo": "1.2.0"}]:
            with self.assertRaises(gate.GateError, msg=str(kwargs)):
                gate.decide("v1.2.0", project(**kwargs), previous_build=None)
        with self.assertRaises(gate.GateError):
            gate.decide("v1.2.1", project(), previous_build=None)

    def test_the_build_number_must_be_a_positive_integer_above_the_previous_one(self):
        for build in [0, -1, "5", 5.5, True, None]:
            root = project()
            (root / "nsh.json").write_text(json.dumps({"version": "1.2.0", "build": build}))
            with self.assertRaises(gate.GateError, msg=repr(build)):
                gate.decide("v1.2.0", root, previous_build=None)
        for previous in [5, 6, 100]:
            with self.assertRaises(gate.GateError, msg=str(previous)):
                gate.decide("v1.2.0", project(build=5), previous_build=previous)
        self.assertEqual(gate.decide("v1.2.0", project(build=5), previous_build=None)["build"], "5")

    def test_the_changelog_must_have_a_nonempty_section_for_this_version(self):
        for text in ["# Changelog\n\n## Unreleased\n- x\n", "## 1.2.0\n\n## 1.1.0\n- y\n", "## [1.2.0]\n   \n"]:
            with self.assertRaises(gate.GateError, msg=text):
                gate.decide("v1.2.0", project(changelog=text), previous_build=None)
        # 1.2.0 must not match 1.2.01, 11.2.0 or 1.2.0-rc.1.
        for other in ["## 1.2.01\n- x\n", "## 11.2.0\n- x\n", "## 1.2.0-rc.1\n- x\n"]:
            with self.assertRaises(gate.GateError, msg=other):
                gate.decide("v1.2.0", project(changelog=other), previous_build=None)

    def test_the_last_section_needs_no_following_heading(self):
        self.assertEqual(gate.changelog_section("## 1.0.0\n- only\n", "1.0.0"), "- only")

    def test_prerelease_tags_are_marked_for_the_preview_channel(self):
        root = project(version="1.3.0-beta.1", cargo="1.3.0-beta.1", changelog="## 1.3.0-beta.1\n- b\n")
        result = gate.decide("v1.3.0-beta.1", root, previous_build=None)
        self.assertEqual((result["channel"], result["prerelease"]), ("preview", "true"))

    def test_cargo_version_ignores_dependency_versions_and_other_sections(self):
        root = Path(tempfile.mkdtemp())
        (root / "Cargo.toml").write_text('[dependencies]\nfoo = { version = "9.9.9" }\n[package]\nname = "a"\nversion = "2.0.0" # note\n[profile.release]\nversion = "7"\n')
        self.assertEqual(gate.read_cargo_version(root / "Cargo.toml"), "2.0.0")
        (root / "Cargo.toml").write_text('[dependencies]\nversion = "1.0.0"\n')
        with self.assertRaises(gate.GateError):
            gate.read_cargo_version(root / "Cargo.toml")

    def test_missing_or_broken_files_are_gate_errors_not_crashes(self):
        root = Path(tempfile.mkdtemp())
        with self.assertRaises(gate.GateError):
            gate.read_nsh_json(root / "nsh.json")
        (root / "nsh.json").write_text("{not json")
        with self.assertRaises(gate.GateError):
            gate.read_nsh_json(root / "nsh.json")


class PreviousBuild(unittest.TestCase):
    @staticmethod
    def release(build=None, draft=False, name="build.txt"):
        assets = [] if build is None else [{"name": name, "browser_download_url": f"u/{build}"}]
        return {"draft": draft, "assets": assets}

    @staticmethod
    def fetch(url):
        return url.split("/")[-1].encode()

    def test_takes_the_highest_build_across_prereleases_and_stable(self):
        releases = [self.release(7), self.release(9), self.release(3), self.release(None)]
        self.assertEqual(gate.highest_build(releases, self.fetch), 9)

    def test_drafts_and_other_assets_are_ignored(self):
        releases = [self.release(99, draft=True), self.release(5, name="notes.txt"), self.release(4)]
        self.assertEqual(gate.highest_build(releases, self.fetch), 4)

    def test_no_previous_release_means_no_constraint(self):
        self.assertIsNone(gate.highest_build([], self.fetch))
        self.assertIsNone(gate.highest_build([self.release(None)], self.fetch))

    def test_an_unreadable_or_invalid_build_file_is_an_error_not_a_free_pass(self):
        for body in [b"abc", b"", b"0", b"-3", b"5.5"]:
            with self.assertRaises(gate.GateError, msg=repr(body)):
                gate.highest_build([self.release(1)], lambda url, body=body: body)

        def broken(url):
            raise OSError("down")

        with self.assertRaises(gate.GateError):
            gate.highest_build([self.release(1)], broken)

    def test_only_recent_releases_are_read(self):
        many = [self.release(i + 1) for i in range(gate.RECENT_RELEASES + 5)]
        # Builds beyond the window are never fetched, so the maximum is within it.
        self.assertEqual(gate.highest_build(many, self.fetch), gate.RECENT_RELEASES)


class Rehearsal(unittest.TestCase):
    """A rehearsal exists to test the build chain at any time, so only what would make the
    build itself meaningless is fatal; what only matters to publishing is a warning."""

    def test_a_consistent_project_has_no_warnings_in_either_mode(self):
        for strict in (True, False):
            result = gate.decide("v1.2.0", project(), previous_build=4, strict=strict)
            self.assertEqual(result["warnings"], [])

    def test_a_version_mismatch_is_fatal_even_in_a_rehearsal(self):
        with self.assertRaises(gate.GateError):
            gate.decide("v1.2.0", project(cargo="9.9.9"), previous_build=None, strict=False)
        with self.assertRaises(gate.GateError):
            gate.decide("v1.2.0", project(build=0), previous_build=None, strict=False)

    def test_a_missing_changelog_is_a_warning_with_placeholder_notes(self):
        result = gate.decide("v1.2.0", project(changelog="# nothing\n"), previous_build=None, strict=False)
        self.assertEqual(len(result["warnings"]), 1)
        self.assertIn("CHANGELOG", result["warnings"][0])
        self.assertIn("1.2.0", result["notes"])
        with self.assertRaises(gate.GateError):
            gate.decide("v1.2.0", project(changelog="# nothing\n"), previous_build=None, strict=True)

    def test_a_build_number_that_is_not_higher_is_a_warning(self):
        result = gate.decide("v1.2.0", project(build=5), previous_build=5, strict=False)
        self.assertEqual(len(result["warnings"]), 1)
        self.assertIn("build", result["warnings"][0])
        with self.assertRaises(gate.GateError):
            gate.decide("v1.2.0", project(build=5), previous_build=5, strict=True)

    def test_every_soft_problem_is_reported_not_just_the_first(self):
        result = gate.decide("v1.2.0", project(build=5, changelog="x\n"), previous_build=9, strict=False)
        self.assertEqual(len(result["warnings"]), 2)


class Entry(unittest.TestCase):
    def run_main(self, env, root):
        import contextlib
        import io
        import os
        out, err = io.StringIO(), io.StringIO()
        saved = dict(os.environ)
        try:
            for key in ("GITHUB_REF", "GITHUB_EVENT_NAME", "GITHUB_REPOSITORY", "GITHUB_EVENT_PATH"):
                os.environ.pop(key, None)
            os.environ.update(env)
            with contextlib.redirect_stdout(out), contextlib.redirect_stderr(err):
                code = gate.main(["--root", str(root), "--skip-git"])
        finally:
            os.environ.clear()
            os.environ.update(saved)
        return code, out.getvalue(), err.getvalue()

    push = {"GITHUB_EVENT_NAME": "push", "GITHUB_REF": "refs/tags/v1.2.0"}

    def test_a_pushed_tag_is_the_only_way_in(self):
        code, out, _ = self.run_main(self.push, project())
        self.assertEqual(code, 0)
        self.assertIn("version=1.2.0", out)
        self.assertIn("rehearsal=false", out)

    manual = {"GITHUB_EVENT_NAME": "workflow_dispatch", "GITHUB_REF": "refs/heads/main"}

    def test_a_manual_run_is_a_rehearsal_that_takes_its_version_from_the_files(self):
        code, out, _ = self.run_main(self.manual, project())
        self.assertEqual(code, 0)
        self.assertIn("version=1.2.0", out)
        self.assertIn("rehearsal=true", out)
        self.assertIn("release_ready=true", out)
        # A real release says it is not one.
        self.assertIn("rehearsal=false", self.run_main(self.push, project())[1])

    def test_a_rehearsal_tells_you_when_a_tag_push_would_be_refused(self):
        code, out, err = self.run_main(self.manual, project(changelog="# nothing\n"))
        self.assertEqual(code, 0, "a rehearsal still runs")
        self.assertIn("release_ready=false", out)
        self.assertIn("::warning::", err)
        self.assertIn("CHANGELOG", err)

    def test_a_rehearsal_is_still_stopped_by_an_inconsistent_version(self):
        code, _, err = self.run_main(self.manual, project(cargo="9.9.9"))
        self.assertEqual(code, 1)
        self.assertIn("version differs", err)

    def test_a_rehearsal_may_run_on_a_tag_ref_and_is_still_a_rehearsal(self):
        env = {"GITHUB_EVENT_NAME": "workflow_dispatch", "GITHUB_REF": "refs/tags/v1.2.0"}
        self.assertIn("rehearsal=true", self.run_main(env, project())[1])

    def test_a_pull_request_branch_push_schedule_or_nothing_is_refused(self):
        for env in [
            {"GITHUB_EVENT_NAME": "push", "GITHUB_REF": "refs/heads/main"},
            {"GITHUB_EVENT_NAME": "pull_request", "GITHUB_REF": "refs/tags/v1.2.0"},
            {"GITHUB_EVENT_NAME": "schedule", "GITHUB_REF": "refs/heads/main"},
            {},
        ]:
            code, out, err = self.run_main(env, project())
            self.assertEqual(code, 1, str(env))
            self.assertEqual(out, "", str(env))
            self.assertIn("pushed tag or a manual rehearsal", err)

    def test_a_pushed_tag_still_has_to_agree_with_the_files(self):
        code, _, err = self.run_main({**self.push, "GITHUB_REF": "refs/tags/v9.9.9"}, project())
        self.assertEqual(code, 1)
        self.assertIn("version differs", err)
        code, _, err = self.run_main(self.push, project(changelog="# nothing\n"))
        self.assertEqual(code, 1)


if __name__ == "__main__":
    unittest.main()
