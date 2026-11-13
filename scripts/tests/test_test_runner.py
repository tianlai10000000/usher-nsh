"""Verify the test command cannot turn empty, partial or failed execution into success."""

import importlib.util
import json
import subprocess
import tempfile
import unittest
from pathlib import Path
from unittest.mock import Mock, patch

SPEC = importlib.util.spec_from_file_location("suite_runner", Path(__file__).resolve().parents[1] / "test.py")
runner = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(runner)


def output(passed=1, ignored=0, filtered=0):
    return ("test reference::contract ... ok\n"
            f"test result: ok. {passed} passed; 0 failed; {ignored} ignored; "
            f"0 measured; {filtered} filtered out; finished in 0.00s\n")


class Evidence(unittest.TestCase):
    def test_accepts_executed_required_tests(self):
        self.assertEqual(runner.check_result(output(), 0, ["reference::"])["passed"], 1)

    def test_rejects_false_successes(self):
        cases = [(output(), 1, ["reference::"]), ("", 0, ["reference::"]),
                 (output(passed=0), 0, ["reference::"]), (output(ignored=1), 0, ["reference::"]),
                 (output(filtered=1), 0, ["reference::"]), (output(), 0, ["watch::"])]
        for case in cases:
            with self.subTest(case=case), self.assertRaises(runner.SuiteError):
                runner.check_result(*case)

    def test_real_config_has_both_feature_combinations(self):
        config = runner.read_config(runner.CONFIG)
        commands = [run["command"] for run in config["suites"]["state"]["runs"]]
        self.assertTrue(any("--no-default-features" in command for command in commands))
        self.assertTrue(any("backend-watch" in command for command in commands))

    def test_generator_checks_are_not_counted_as_test_cases(self):
        marker = "checked route contracts: fixture.ts"
        self.assertEqual(runner.check_output(marker + "\n", 0, [marker]), {"checks": [marker]})
        for output, code in [(marker, 1), ("", 0), ("prefix " + marker, 0)]:
            with self.subTest(output=output, code=code), self.assertRaises(runner.SuiteError):
                runner.check_output(output, code, [marker])

    def test_invalid_and_empty_config_is_rejected(self):
        config = runner.read_config(runner.CONFIG)
        variants = [{**config, "schema_version": -1}, {**config, "suites": {}},
                    {**config, "report_directory": "../escape"},
                    {**config, "suites": {"state": {"runs": []}}}]
        with tempfile.TemporaryDirectory() as directory:
            path = Path(directory) / "config.json"
            for variant in variants:
                path.write_text(json.dumps(variant), encoding="utf-8")
                with self.subTest(variant=variant), self.assertRaises(runner.SuiteError):
                    runner.read_config(path)

    def test_unknown_suite_never_starts_a_process(self):
        with patch.object(runner, "execute") as process:
            with self.assertRaises(runner.SuiteError):
                runner.run_suite("not-implemented", runner.read_config(runner.CONFIG))
            process.assert_not_called()

    def test_failure_keeps_logs_and_still_runs_other_combinations(self):
        config = runner.read_config(runner.CONFIG)
        results = [subprocess.CompletedProcess([], 1, "compile failure"),
                   subprocess.CompletedProcess([], 0, output())]
        with tempfile.TemporaryDirectory() as directory, patch.object(runner, "execute", side_effect=results) as process, patch("builtins.print"):
            root = Path(directory)
            self.assertFalse(runner.run_suite("state", config, root))
            self.assertEqual(process.call_count, len(config["suites"]["state"]["runs"]))
            report = json.loads((root / config["report_directory"] / "state.json").read_text())
            self.assertEqual(report["status"], "failed")
            self.assertTrue(all((root / run["log"]).exists() for run in report["runs"]))

    def test_timeout_replaces_stale_success_report(self):
        config = runner.read_config(runner.CONFIG)
        with tempfile.TemporaryDirectory() as directory, patch.object(runner, "execute", side_effect=subprocess.TimeoutExpired("cargo", 1, output=b"partial log")), patch("builtins.print"):
            root = Path(directory)
            report = root / config["report_directory"] / "state.json"
            report.parent.mkdir(parents=True)
            report.write_text('{"status": "passed"}', encoding="utf-8")
            self.assertFalse(runner.run_suite("state", config, root))
            self.assertEqual(json.loads(report.read_text())["status"], "failed")

    def test_timeout_terminates_children_and_preserves_output(self):
        process = Mock()
        process.communicate.side_effect = [subprocess.TimeoutExpired("cargo", 1), ("partial log", None)]
        with patch.object(runner.subprocess, "Popen", return_value=process), patch.object(runner, "terminate_process_tree") as terminate:
            with self.assertRaises(subprocess.TimeoutExpired) as caught:
                runner.execute(["cargo", "test"], runner.ROOT, 1)
            terminate.assert_called_once_with(process)
            self.assertEqual(caught.exception.stdout, "partial log")


class NshEvidence(unittest.TestCase):
    XML = '<testsuites tests="1"><testsuite><testcase classname="nested/projection.test.ts" name="state ordering"/></testsuite></testsuites>'

    def test_discovers_nested_ts_and_tsx_and_rejects_missing_paths(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            (root / "nested/deeper").mkdir(parents=True)
            (root / "nested/projection.test.ts").write_text("")
            (root / "nested/deeper/binding.test.tsx").write_text("")
            (root / "nested/deeper/Fixture.tsx").write_text("")
            (root / "nested/deeper/.nsh/test").mkdir(parents=True)
            (root / "nested/deeper/.nsh/test/Fixture.steps.test.ts").write_text("")
            self.assertEqual(runner.discover_nsh(["nested"], root), {
                "nested/projection.test.ts", "nested/deeper/binding.test.tsx"})
            with self.assertRaises(runner.SuiteError):
                runner.discover_nsh(["absent"], root)

    def test_junit_requires_successful_cases_complete_discovery_and_groups(self):
        with tempfile.TemporaryDirectory() as directory:
            path = Path(directory) / "report.xml"
            path.write_text(self.XML)
            self.assertEqual(runner.check_nsh_result(path, 0, ["state "], {"nested/projection.test.ts"})["passed"], 1)
            for code, prefixes, files in [(1, ["state "], {"nested/projection.test.ts"}),
                                           (0, ["missing"], {"nested/projection.test.ts"}),
                                           (0, ["state "], {"nested/projection.test.ts", "new.test.ts"})]:
                with self.subTest(code=code, prefixes=prefixes, files=files), self.assertRaises(runner.SuiteError):
                    runner.check_nsh_result(path, code, prefixes, files)

    def test_junit_rejects_leaf_failure_skip_empty_malformed_and_inconsistent_counts(self):
        invalid = ["", "not xml", '<testsuites tests="0"/>', self.XML.replace('tests="1"', 'tests="2"')]
        for tag in ("failure", "error", "skipped"):
            invalid.append(self.XML.replace('name="state ordering"/>', f'name="state ordering"><{tag}/></testcase>'))
        with tempfile.TemporaryDirectory() as directory:
            path = Path(directory) / "report.xml"
            for xml in invalid:
                path.write_text(xml)
                with self.subTest(xml=xml), self.assertRaises(runner.SuiteError):
                    runner.check_nsh_result(path, 0, ["state "], {"nested/projection.test.ts"})
            path.unlink()
            with self.assertRaises(runner.SuiteError):
                runner.check_nsh_result(path, 0, ["state "], {"nested/projection.test.ts"})

    def test_old_junit_is_removed_and_cannot_mask_a_missing_new_report(self):
        config = {"report_directory": "reports", "suites": {"ui": {"runs": [{
            "id": "ui", "runner": "nsh", "command": ["nsh", "test"],
            "test_paths": ["nested"], "timeout_seconds": 1, "required_test_prefixes": ["state "]}]}}}
        with tempfile.TemporaryDirectory() as directory, patch("builtins.print"):
            root = Path(directory)
            (root / "nested").mkdir()
            (root / "nested/projection.test.ts").write_text("")
            (root / "reports").mkdir()
            xml = root / "reports/ui-0.xml"
            xml.write_text(self.XML)
            with patch.object(runner, "execute", return_value=subprocess.CompletedProcess([], 0, "no report")):
                self.assertFalse(runner.run_suite("ui", config, root))
            self.assertFalse(xml.exists())
            def execute(command, _, timeout):
                self.assertIn("--junit", command)
                xml.write_text(self.XML)
                return subprocess.CompletedProcess(command, 0, "completed")
            with patch.object(runner, "execute", side_effect=execute):
                self.assertTrue(runner.run_suite("ui", config, root))

    def test_nsh_configuration_rejects_escaping_paths_and_unknown_runners(self):
        config = runner.read_config(runner.CONFIG)
        with tempfile.TemporaryDirectory() as directory:
            path = Path(directory) / "config.json"
            for changes in ({"test_paths": ["../escape"]}, {"test_paths": []}, {"runner": "unknown"}):
                run = dict(config["suites"]["state-ui"]["runs"][0], **changes)
                invalid = dict(config, suites={"ui": {"runs": [run]}})
                path.write_text(json.dumps(invalid))
                with self.subTest(changes=changes), self.assertRaises(runner.SuiteError):
                    runner.read_config(path)

    def test_runs_each_discovered_file_and_checks_required_groups_across_files(self):
        config = {"report_directory": "reports", "suites": {"ui": {"runs": [{
            "id": "ui", "runner": "nsh", "command": ["nsh", "test"],
            "test_paths": ["nested"], "timeout_seconds": 1,
            "required_test_prefixes": ["state ordering", "state cleanup"]}]}}}
        with tempfile.TemporaryDirectory() as directory, patch("builtins.print"):
            root = Path(directory)
            (root / "nested/deeper").mkdir(parents=True)
            files = {"nested/projection.test.ts": "state ordering",
                     "nested/deeper/binding.test.tsx": "state cleanup"}
            for file in files:
                (root / file).write_text("")

            def execute(command, _, timeout):
                file = command[2]
                self.assertIn(file, files)
                report = root / command[command.index("--junit") + 1]
                report.write_text(self.XML.replace("nested/projection.test.ts", file)
                                  .replace("state ordering", files[file]))
                return subprocess.CompletedProcess(command, 0, "completed")

            with patch.object(runner, "execute", side_effect=execute) as process:
                self.assertTrue(runner.run_suite("ui", config, root))
                self.assertEqual(process.call_count, len(files))
                config["suites"]["ui"]["runs"][0]["required_test_prefixes"].append("missing group")
                self.assertFalse(runner.run_suite("ui", config, root))


if __name__ == "__main__":
    unittest.main()
