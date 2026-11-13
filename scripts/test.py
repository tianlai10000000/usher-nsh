#!/usr/bin/env python3
"""Run configured test suites with existing runners and preserve execution evidence.

Suite definitions live in tests/config.json. Cargo and NSH own test execution;
this wrapper rejects failed, empty, ignored or incomplete required test runs.
"""

from __future__ import annotations

import argparse
import json
import os
import re
import signal
import subprocess
import sys
import xml.etree.ElementTree as ET
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
CONFIG = ROOT / "tests" / "config.json"
SUMMARY = re.compile(
    r"^test result: ok\. (\d+) passed; (\d+) failed; (\d+) ignored; "
    r"(\d+) measured; (\d+) filtered out;", re.MULTILINE
)
PASSED_TEST = re.compile(r"^test (.+) \.\.\. ok$", re.MULTILINE)


class SuiteError(Exception):
    """An invalid configuration or incomplete test run."""


def read_config(path: Path) -> dict:
    """Validate the supported configuration before starting any subprocess."""
    config = json.loads(path.read_text(encoding="utf-8"))
    if config.get("schema_version") != 1:
        raise SuiteError("unsupported test configuration schema")
    report = Path(config.get("report_directory", ""))
    if not report.parts or report.is_absolute() or ".." in report.parts:
        raise SuiteError("report_directory must be a nonempty repository-relative path")
    suites = config.get("suites")
    if not isinstance(suites, dict) or not suites:
        raise SuiteError("no test suites configured")
    identifiers = set()
    for name, suite in suites.items():
        if not re.fullmatch(r"[a-z][a-z0-9-]*", name):
            raise SuiteError("invalid suite name")
        runs = suite.get("runs")
        if not isinstance(runs, list) or not runs:
            raise SuiteError(f"{name}: no test runs configured")
        for run in runs:
            identifier = run.get("id", "")
            if not re.fullmatch(r"[a-z][a-z0-9-]*", identifier) or identifier in identifiers:
                raise SuiteError("run identifiers must be safe and unique")
            identifiers.add(identifier)
            command = run.get("command")
            if not isinstance(command, list) or not command or not all(isinstance(arg, str) and arg for arg in command):
                raise SuiteError(f"{identifier}: command must be a nonempty argument array")
            timeout = run.get("timeout_seconds")
            if type(timeout) not in (int, float) or timeout <= 0:
                raise SuiteError(f"{identifier}: positive timeout required")
            prefixes = run.get("required_output" if run.get("runner") == "check" else "required_test_prefixes")
            if not isinstance(prefixes, list) or not prefixes or not all(isinstance(p, str) and p for p in prefixes):
                raise SuiteError(f"{identifier}: required test groups must be specified")
            if run.get("runner", "cargo") not in ("cargo", "nsh", "native", "check"):
                raise SuiteError(f"{identifier}: unsupported runner")
            if run.get("runner") in ("nsh", "native"):
                paths = run.get("test_paths")
                if not isinstance(paths, list) or not paths or any(
                    not isinstance(path, str) or not Path(path).parts or Path(path).is_absolute()
                    or ".." in Path(path).parts for path in paths
                ):
                    raise SuiteError(f"{identifier}: repository-relative test_paths required")
    return config


def check_result(output: str, returncode: int, prefixes: list[str]) -> dict:
    """Check execution evidence, without reimplementing any domain assertion."""
    if returncode:
        raise SuiteError(f"test process exited with {returncode}")
    summaries = [tuple(map(int, groups)) for groups in SUMMARY.findall(output)]
    if not summaries or sum(row[0] for row in summaries) == 0:
        raise SuiteError("no executed tests reported")
    if any(any(row[index] for index in (1, 2, 3, 4)) for row in summaries):
        raise SuiteError("failed, ignored, measured or filtered tests cannot satisfy a full suite")
    passed = PASSED_TEST.findall(output)
    for prefix in prefixes:
        if not any(name.startswith(prefix) for name in passed):
            raise SuiteError(f"required test group did not execute: {prefix}")
    return {"passed": sum(row[0] for row in summaries), "tests": passed}


def check_output(output: str, returncode: int, markers: list[str]) -> dict:
    """Record generator checks separately from executed test cases."""
    if returncode or any(marker not in output.splitlines() for marker in markers):
        raise SuiteError("generator check failed or did not confirm every required artifact")
    return {"checks": markers}


def discover_nsh(paths: list[str], root: Path) -> set[str]:
    """Discover module-local TS tests recursively, excluding supporting fixtures."""
    files = set()
    for name in paths:
        path = root / name
        candidates = path.rglob("*") if path.is_dir() else [path]
        found = {str(candidate.relative_to(root)).replace("\\", "/") for candidate in candidates
                 if candidate.is_file() and candidate.name.endswith((".test.ts", ".test.tsx"))
                 and not {".nsh", "node_modules", "target"}.intersection(candidate.relative_to(root).parts)}
        if not found:
            raise SuiteError(f"no tests discovered at {name}")
        files.update(found)
    return files


def check_nsh_result(path: Path, returncode: int, prefixes: list[str], files: set[str]) -> dict:
    """Require fresh JUnit evidence for every discovered file and every required group."""
    if returncode:
        raise SuiteError(f"test process exited with {returncode}")
    try:
        report = ET.parse(path).getroot()
    except (OSError, ET.ParseError) as error:
        raise SuiteError(f"missing or invalid JUnit report: {error}") from error
    cases = list(report.iter("testcase"))
    if not cases or report.tag not in ("testsuite", "testsuites"):
        raise SuiteError("no executed tests reported")
    if any(next(report.iter(tag), None) is not None for tag in ("failure", "error", "skipped")):
        raise SuiteError("failed or skipped tests cannot satisfy a full suite")
    for suite in report.iter():
        if suite.tag in ("testsuite", "testsuites"):
            if any(int(suite.get(field, "0")) for field in ("failures", "errors", "skipped")):
                raise SuiteError("JUnit reports failed or skipped tests")
    if int(report.get("tests", "-1")) != len(cases):
        raise SuiteError("inconsistent JUnit test count")
    identities = {(case.get("classname"), case.get("name")) for case in cases}
    if len(identities) != len(cases) or any(not name for _, name in identities):
        raise SuiteError("missing or duplicate JUnit test identities")
    reported = {case.get("classname", "").replace("\\", "/") for case in cases}
    if reported != files:
        raise SuiteError(f"JUnit discovery mismatch: expected {sorted(files)}, got {sorted(reported)}")
    names = [case.get("name", "") for case in cases]
    for prefix in prefixes:
        if not any(name.startswith(prefix) for name in names):
            raise SuiteError(f"required test group did not execute: {prefix}")
    return {"passed": len(cases), "tests": names, "files": sorted(files)}


def terminate_process_tree(process: subprocess.Popen) -> None:
    """Stop Cargo and its test children, including a hung test binary."""
    if os.name == "posix":
        try:
            os.killpg(process.pid, signal.SIGTERM)
            try:
                process.wait(timeout=15)
            except subprocess.TimeoutExpired:
                os.killpg(process.pid, signal.SIGKILL)
        except ProcessLookupError:
            pass
    else:
        subprocess.run(["taskkill", "/PID", str(process.pid), "/T", "/F"],
                       stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL, check=False)
        if process.poll() is None:
            process.kill()


def execute(command: list[str], root: Path, timeout: float) -> subprocess.CompletedProcess:
    """Capture one existing runner, releasing its process tree on interruption."""
    process = subprocess.Popen(command, cwd=root, stdout=subprocess.PIPE,
                               stderr=subprocess.STDOUT, text=True,
                               start_new_session=os.name == "posix")
    try:
        output, _ = process.communicate(timeout=timeout)
    except (subprocess.TimeoutExpired, KeyboardInterrupt) as error:
        terminate_process_tree(process)
        output, _ = process.communicate()
        if isinstance(error, subprocess.TimeoutExpired):
            raise subprocess.TimeoutExpired(command, timeout, output=output) from error
        raise
    return subprocess.CompletedProcess(command, process.returncode, output)


def run_suite(name: str, config: dict, root: Path = ROOT) -> bool:
    """Run every configured feature combination and write logs plus a JSON report."""
    if name not in config["suites"]:
        raise SuiteError(f"unknown or unimplemented suite: {name}")
    directory = root / config["report_directory"]
    directory.mkdir(parents=True, exist_ok=True)
    report_path = directory / f"{name}.json"
    # Remove a prior success before doing work, including when execution is interrupted.
    report_path.unlink(missing_ok=True)
    records = []
    expanded = []
    definitions = config["suites"][name]["runs"]
    for run in definitions:
        if run.get("runner") != "nsh":
            expanded.append(run)
            continue
        try:
            files = sorted(discover_nsh(run["test_paths"], root))
            for index, file in enumerate(files):
                expanded.append({**run, "id": f"{run['id']}-{index}", "group": run["id"],
                                 "command": [*run["command"], file], "test_paths": [file],
                                 "required_test_prefixes": []})
        except (OSError, SuiteError) as error:
            records.append({"id": run["id"], "group": run["id"], "status": "failed", "error": str(error)})
    for run in expanded:
        command = list(run["command"])
        junit = directory / f"{run['id']}.xml" if run.get("runner") in ("nsh", "native") else None
        if junit is not None:
            junit.unlink(missing_ok=True)
            command.extend(["--junit", str(junit.relative_to(root))])
        print(f"Running {run['id']}: {' '.join(command)}", flush=True)
        record = {"id": run["id"], "group": run.get("group", run["id"]), "command": command, "status": "failed"}
        output = ""
        try:
            files = (set(run["test_paths"]) if run.get("runner") == "native" else discover_nsh(run["test_paths"], root)) if junit is not None else set()
            result = execute(command, root, run["timeout_seconds"])
            output = result.stdout
            record["returncode"] = result.returncode
            if junit is not None:
                record["junit"] = str(junit.relative_to(root))
                record.update(check_nsh_result(junit, result.returncode, run["required_test_prefixes"], files))
            elif run.get("runner") == "check":
                record.update(check_output(output, result.returncode, run["required_output"]))
            else:
                record.update(check_result(output, result.returncode, run["required_test_prefixes"]))
            record["status"] = "passed"
        except subprocess.TimeoutExpired as error:
            output = error.stdout or ""
            if isinstance(output, bytes):
                output = output.decode("utf-8", errors="replace")
            record["error"] = f"test timed out after {run['timeout_seconds']} seconds"
        except (OSError, ValueError, SuiteError) as error:
            record["error"] = str(error)
        log = directory / f"{run['id']}.log"
        log.write_text(output, encoding="utf-8")
        record["log"] = str(log.relative_to(root))
        records.append(record)
        print(output, end="" if output.endswith("\n") else "\n")
        if "error" in record:
            print(record["error"], file=sys.stderr)
    # A logical NSH group may span files. Validate its required cases after every
    # file ran, so adding a nested test automatically enters the same CI command.
    for definition in definitions:
        if definition.get("runner") != "nsh":
            continue
        group = [record for record in records if record.get("group") == definition["id"]]
        names = [test for record in group for test in record.get("tests", [])]
        missing = [prefix for prefix in definition["required_test_prefixes"]
                   if not any(test.startswith(prefix) for test in names)]
        if missing:
            group[0]["status"] = "failed"
            group[0]["error"] = f"required test groups did not execute: {missing}"
            print(group[0]["error"], file=sys.stderr)
    success = bool(records) and all(record["status"] == "passed" for record in records)
    report_path.write_text(json.dumps({"schema_version": 1, "suite": name,
                                     "status": "passed" if success else "failed",
                                     "runs": records}, indent=2) + "\n", encoding="utf-8")
    print(f"Report: {report_path}")
    return success


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("suite", help="suite name configured in tests/config.json")
    parser.add_argument("--target", choices=("headless", "macos"), help="currently supported E2E host")
    args = parser.parse_args()
    if args.target and args.suite != "e2e":
        parser.error("--target applies only to e2e")
    try:
        suite = "e2e-macos" if args.target == "macos" else args.suite
        return 0 if run_suite(suite, read_config(CONFIG)) else 1
    except (OSError, ValueError, AttributeError, TypeError, SuiteError) as error:
        print(f"test suite error: {error}", file=sys.stderr)
        return 1


if __name__ == "__main__":
    raise SystemExit(main())
