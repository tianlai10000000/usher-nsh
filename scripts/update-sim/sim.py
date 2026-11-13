#!/usr/bin/env python3
"""Local update simulation: runs the app's real update path against a mock update source.

It builds an "old" app whose `updater.url` points at a local HTTP server, serves a manifest
and packages the scenario describes, launches a unique test application in isolated data
roots and asserts what ended up in its update directory. Nothing here touches GitHub or a real key: every run
generates one-off keys in a temporary tree and removes them after the run.

Subcommands: list, run <scenario>|--all, serve <scenario>, clean. See docs/update.md, section 12.

The slow step is `nsh package`. Each invocation builds its own app identity and reuses only
Cargo compilation artifacts; source copies, application and keys are temporary.

NSH accepts plain HTTP only for 127.0.0.1, localhost and [::1] (nsh-updater `check_url`); the
rest of the verification path is the same as with a real HTTPS source.
"""

from __future__ import annotations

import argparse
import http.server
import io
import json
import os
import plistlib
import uuid
import fcntl
import shutil
import signal
import subprocess
import sys
import tarfile
import tempfile
import threading
import time
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
SIM = ROOT / ".nsh" / "update-sim"
SCENARIOS = Path(__file__).resolve().parent / "scenarios"
CONFIG = Path(__file__).resolve().parent / "config.json"

sys.path.insert(0, str(ROOT / 'scripts'))
from macos_test_data import AppDataNamespace


def load_config() -> dict:
    return json.loads(CONFIG.read_text())


def fail(message: str) -> "None":
    print(f"error: {message}", file=sys.stderr)
    raise SystemExit(1)


COMMAND_LOG: Path | None = None


def run(cmd: list[str], **kwargs) -> subprocess.CompletedProcess:
    """Capture command evidence and stop its process group on timeout or interruption."""
    timeout = kwargs.pop('timeout', None)
    process = subprocess.Popen(cmd, stdout=subprocess.PIPE, stderr=subprocess.STDOUT,
                               text=True, start_new_session=True, **kwargs)
    try:
        output, _ = process.communicate(timeout=timeout)
    except (subprocess.TimeoutExpired, KeyboardInterrupt):
        try:
            os.killpg(process.pid, signal.SIGKILL)
        except ProcessLookupError:
            pass
        output, _ = process.communicate()
        if COMMAND_LOG:
            with COMMAND_LOG.open('a') as log:
                log.write(f'{cmd[0]} interrupted: {output}\n')
        raise
    if COMMAND_LOG:
        with COMMAND_LOG.open('a') as log:
            log.write(json.dumps(cmd) + '\n' + output + '\n')
    if process.returncode:
        raise RuntimeError(f'{cmd[0]} failed ({process.returncode}): {output}')
    return subprocess.CompletedProcess(cmd, process.returncode, output)


def require_nsh() -> None:
    if shutil.which("nsh") is None:
        fail("the nsh command line is not installed (scripts/ci/install-nsh.sh)")


# --- keys -------------------------------------------------------------------------------------


def make_keys(directory: Path, config: dict) -> dict:
    """One-off manifest, root and release keys plus a certificate, in `directory`."""
    directory.mkdir(parents=True, exist_ok=True)
    for name in ("manifest", "root", "release"):
        if not (directory / f"{name}.secret").exists():
            run(["nsh", "keygen", str(directory / name)])
    cert = directory / "release.cert"
    if not cert.exists():
        run(["nsh", "certify", "--root", str(directory / "root.secret"), "--key", str(directory / "release.pub"),
             "--apps", config["appIdPattern"], "--days", str(config["certificateDays"]), "--out", str(cert)])
    read = lambda name: (directory / name).read_text().strip()
    return {"manifest_pub": read("manifest.pub"), "root_pub": read("root.pub"), "dir": directory}


# --- project copies -----------------------------------------------------------------------------


def project_copy(destination: Path, version: str, build: int, keys: dict, url: str | None) -> Path:
    """A copy of the project with `version`, `build` and a simulation `updater` in nsh.json.

    The repository's nsh.json is never modified; source copies live in the temporary run tree.
    """
    if destination.exists():
        shutil.rmtree(destination)
    destination.mkdir(parents=True)
    for name in ["App.tsx", "src", "ui", "i18n", "platforms", "crates", "build.rs", "Cargo.toml", "Cargo.lock", "tsconfig.json", ".cargo"]:
        source = ROOT / name
        if source.is_dir():
            shutil.copytree(source, destination / name, symlinks=True)
        elif source.exists():
            shutil.copy2(source, destination / name)
    manifest = json.loads((ROOT / "nsh.json").read_text())
    manifest["id"] = keys["app_id"]
    manifest.pop("schemes", None)
    manifest["rust"] = str(ROOT)
    manifest["version"] = version
    manifest["build"] = build
    if url is not None:
        manifest["updater"] = {"url": url, "publicKey": keys["manifest_pub"], "packages": {"roots": [keys["root_pub"]]}}
    (destination / "nsh.json").write_text(json.dumps(manifest, indent=2))
    return destination


# --- the temporary old app -------------------------------------------------------------------------


def build_environment() -> dict:
    """Share compiler artifacts across shell and package builds, without sharing runtime data."""
    return {**os.environ, 'CARGO_TARGET_DIR': str(ROOT / 'target/update-sim-build'),
            'CARGO_INCREMENTAL': '0', 'USHER_MACOS_APPKIT': '0'}


def old_app(keys: dict, config: dict, directory: Path) -> Path:
    """Build one temporary application; reuse only Cargo artifacts, never an old identity."""
    project = project_copy(directory / "old-src", config["oldVersion"], config["oldBuild"], keys,
                           f"http://127.0.0.1:{config['port']}/latest.json")
    output = directory / 'app'
    print('building the isolated old app; reusing Cargo compilation artifacts', flush=True)
    run(['nsh', 'package', '--target', 'macos', '--arch', 'arm64', '-o', str(output)],
        cwd=project, env=build_environment(), timeout=config['buildTimeoutSeconds'])
    apps = list(output.glob('*.app'))
    if len(apps) != 1:
        raise RuntimeError('nsh package did not produce exactly one app')
    validate_app(apps[0], keys['app_id'])
    return apps[0]


def validate_app(app: Path, app_id: str) -> Path:
    """Reject a different application or URL registration before starting any process."""
    info = plistlib.loads((app / 'Contents/Info.plist').read_bytes())
    if info['CFBundleIdentifier'] != app_id or info.get('CFBundleURLTypes'):
        raise RuntimeError('test app identity or URL registration mismatch')
    executable = app / 'Contents/MacOS' / info['CFBundleExecutable']
    if not executable.is_file() or executable.parent != app / 'Contents/MacOS':
        raise RuntimeError('invalid test app executable')
    return executable


# --- artifacts of one scenario --------------------------------------------------------------------


def fake_archive(directory: Path, name: str, version: str) -> Path:
    """A minimal .tar.gz named by NSH's convention. The shell-update scenario only inspects its
    hash and size on the way in; a real replacement needs `real_archive` instead."""
    path = directory / name
    with tarfile.open(path, "w:gz") as archive:
        data = f"simulated shell {version}".encode()
        info = tarfile.TarInfo("Usher NSH.app/Contents/placeholder")
        info.size = len(data)
        archive.addfile(info, io.BytesIO(data))
    return path


def build_artifacts(scenario: dict, keys: dict, config: dict, directory: Path) -> Path:
    """Writes the files the mock source serves: manifest, signature, package and archive."""
    served = directory / "served"
    if served.exists():
        shutil.rmtree(served)
    served.mkdir(parents=True)
    manifest_spec = scenario["manifest"]
    version = manifest_spec["version"]
    build = manifest_spec["build"]
    url = f"http://127.0.0.1:{config['port']}/latest.json"
    project = project_copy(directory / "new-src", version, build, keys, url)

    archive_name = f"Usher-NSH-{version}-macos-aarch64.tar.gz"
    archive = fake_archive(served, archive_name, version)
    notes = served / "notes.txt"
    notes.write_text(manifest_spec.get("notes", f"Simulated release {version}."))

    args = ["nsh", "update-manifest", "--entry", str(project), "--key", str(keys["dir"] / "manifest.secret"),
            "--notes", str(notes), "-o", str(served)]
    if manifest_spec.get("package", True):
        package = served / f"pkg-{build}.nshpkg"
        run(["nsh", "publish", "--key", str(keys["dir"] / "release.secret"), "--cert", str(keys["dir"] / "release.cert"),
             "-o", str(package)], cwd=project, env=build_environment(), timeout=config["buildTimeoutSeconds"])
        args += ["--package", str(package)]
    args.append(str(archive))
    run(args)
    notes.unlink()

    tamper = scenario.get("tamper")
    if tamper:
        apply_tamper(served, tamper)
    return served


def apply_tamper(served: Path, tamper: dict) -> None:
    """Corrupts one served file the way the scenario names, after everything was signed."""
    target = served / tamper["file"] if "file" in tamper else next(served.glob(tamper["glob"]))
    data = bytearray(target.read_bytes())
    kind = tamper["kind"]
    if kind == "flip-byte":
        data[len(data) // 2] ^= 0xFF
    elif kind == "truncate":
        data = data[: len(data) // 2]
    elif kind == "replace-id":
        manifest = json.loads(data)
        manifest["id"] = str(uuid.uuid4())
        data = bytearray(json.dumps(manifest).encode())
    else:
        fail(f"unknown tamper kind {kind}")
    target.write_bytes(bytes(data))


# --- the mock source ------------------------------------------------------------------------------


class Source(http.server.ThreadingHTTPServer):
    """Serves `root`, with the scenario's behaviour: rate limit, early cut, offline."""

    daemon_threads = True
    allow_reuse_address = True

    def __init__(self, root: Path, behaviour: dict, port: int):
        self.root = root
        self.behaviour = behaviour
        self.requests: list[str] = []
        super().__init__(("127.0.0.1", port), Handler)


class Handler(http.server.BaseHTTPRequestHandler):
    server: Source

    def log_message(self, *args) -> None:  # noqa: D401 - the simulator prints its own summary
        pass

    def do_GET(self) -> None:
        self.server.requests.append(self.path)
        path = (self.server.root / self.path.lstrip("/").split("?")[0]).resolve()
        if self.server.root.resolve() not in path.parents and path != self.server.root.resolve():
            self.send_error(403)
            return
        if not path.is_file():
            self.send_error(404)
            return
        data = path.read_bytes()
        behaviour = self.server.behaviour
        self.send_response(200)
        self.send_header("Content-Length", str(len(data)))
        self.end_headers()
        is_package = path.suffix in (".nshpkg", ".gz")
        cut = behaviour.get("cutAfterBytes") if is_package else None
        rate = behaviour.get("bytesPerSecond") if is_package else None
        sent = 0
        chunk = 16 * 1024
        try:
            while sent < len(data):
                if cut is not None and sent >= cut:
                    self.connection.shutdown(socket.SHUT_RDWR)  # a dropped connection, not a clean end
                    return
                piece = data[sent:sent + chunk]
                self.wfile.write(piece)
                sent += len(piece)
                if rate:
                    time.sleep(len(piece) / rate)
        except (BrokenPipeError, ConnectionResetError):
            pass


def start_source(root: Path, behaviour: dict, port: int) -> Source | None:
    if behaviour.get("offline"):
        return None
    source = Source(root, behaviour, port)
    threading.Thread(target=source.serve_forever, daemon=True).start()
    return source


# --- running the app --------------------------------------------------------------------------------


def updates_dir(data: Path) -> Path:
    return data / 'updates'


def state_of(directory: Path) -> dict:
    """What the update directory holds, as the assertions see it."""
    names = sorted(p.name for p in directory.iterdir()) if directory.exists() else []
    return {"files": names, "has": {name: name in names for name in ["next.nshpkg", "trying.nshpkg", "current.nshpkg", "trust"]},
            "partial": [n for n in names if n.endswith(".part")]}


class AppProcess:
    """Own a directly launched process group; never override HOME and always reap before cleanup."""

    def __init__(self, app: Path, app_id: str, log: Path, timeout: float):
        self.executable = validate_app(app, app_id)
        self.log = log
        self.timeout = timeout
        self.process = None

    def start(self) -> None:
        with self.log.open('ab') as out:
            self.process = subprocess.Popen([str(self.executable)], stdout=out, stderr=out,
                                            start_new_session=True)

    def stop(self) -> None:
        if self.process is None:
            return
        if self.process.poll() is None:
            try:
                os.killpg(self.process.pid, signal.SIGKILL)
            except ProcessLookupError:
                pass
        self.process.wait(timeout=self.timeout)

    def launch(self, seconds: float) -> None:
        self.start()
        try:
            try:
                code = self.process.wait(timeout=seconds)
            except subprocess.TimeoutExpired:
                return
            raise RuntimeError(f'test app exited prematurely with {code}')
        finally:
            self.stop()


# --- scenarios ----------------------------------------------------------------------------------------


def load_scenarios() -> dict[str, dict]:
    return {path.stem: json.loads(path.read_text()) for path in sorted(SCENARIOS.glob("*.json"))}


def check_expectations(name: str, expectations: list[dict], state: dict, extra: dict) -> list[str]:
    """Returns the failed expectations as messages; an empty list means the scenario passed."""
    problems: list[str] = []
    for expectation in expectations:
        kind = expectation["kind"]
        if kind == "has":
            if not state["has"].get(expectation["name"]):
                problems.append(f"expected {expectation['name']} to exist; found {state['files']}")
        elif kind == "absent":
            if state["has"].get(expectation["name"]):
                problems.append(f"expected {expectation['name']} to be absent; found {state['files']}")
        elif kind == "no-partial":
            if state["partial"]:
                problems.append(f"a partial download was left behind: {state['partial']}")
        elif kind == "log-contains":
            if expectation["text"] not in extra.get("log", ""):
                problems.append(f"the app log lacks {expectation['text']!r}")
        elif kind == "requested":
            if not any(expectation["path"] in path for path in extra.get("requests", [])):
                problems.append(f"the source never saw a request for {expectation['path']}")
        elif kind == "not-requested":
            if any(expectation["path"] in path for path in extra.get("requests", [])):
                problems.append(f"the source saw an unexpected request for {expectation['path']}")
        else:
            problems.append(f"unknown expectation kind {kind}")
    return problems


def preload(scenario: dict, served: Path, data: Path) -> None:
    """Puts the package into `next.nshpkg` the way a finished download would, for scenarios that
    start from a downloaded package (the launch path), which needs no network at all."""
    if scenario.get("preload") != "next":
        return
    package = next(served.glob("*.nshpkg"))
    target = updates_dir(data)
    target.mkdir(parents=True, exist_ok=True)
    shutil.copy2(package, target / "next.nshpkg")


def run_scenario(name: str, scenario: dict, keys: dict, config: dict, app: Path,
                 sandbox: Path, output: Path, host, interactive: bool = False, record: dict | None = None) -> dict:
    """Run one scenario with fresh data; preserve evidence even when launch or cleanup fails."""
    if record is None:
        record = {'name': name, 'status': 'failed'}
    record['cleanup_pending'] = True
    directory = sandbox / name
    directory.mkdir()
    evidence = output / name
    evidence.mkdir()
    namespace = AppDataNamespace(host('directories'), keys['app_id'], directory / 'runtime')
    process = AppProcess(app, keys['app_id'], evidence / 'app.log', config['processTimeoutSeconds'])
    source = None
    print(f"Running {name}: {scenario['description']}", flush=True)
    try:
        if host('process', keys['app_id']):
            raise RuntimeError('test identity already has a running app')
        namespace.create()
        served = build_artifacts(scenario, keys, config, directory)
        preload(scenario, served, namespace.root / 'data')
        source = start_source(served, scenario.get('source', {}), config['port'])
        if interactive:
            process.start()
            print('Isolated app and source running; press Ctrl+C to stop and clean up.', flush=True)
            while process.process.poll() is None:
                time.sleep(1)
            raise RuntimeError('interactive app exited before the session ended')
        for step in scenario.get('launches', [{'seconds': config['launchSeconds']}]):
            process.launch(step['seconds'])
        state = state_of(updates_dir(namespace.root / 'data'))
        record['state'] = state
        record['expectations'] = len(scenario['expect'])
        problems = check_expectations(name, scenario['expect'], state,
                                      {'log': process.log.read_text(errors='replace'),
                                       'requests': list(source.requests) if source else []})
        if problems:
            raise RuntimeError('; '.join(problems))
        record['status'] = 'passed'
    except KeyboardInterrupt:
        record['status'] = 'observed' if interactive else 'interrupted'
    except (Exception, SystemExit) as error:
        record['error'] = str(error)
    finally:
        record['isolated_roots'] = namespace.evidence()
        record['requests'] = list(source.requests) if source else []
        errors = []
        try:
            if source:
                source.shutdown()
                source.server_close()
        except Exception as error:
            errors.append(str(error))
        try:
            process.stop()
            namespace.cleanup(lambda: bool(host('process', keys['app_id'])))
        except Exception as error:
            errors.append(str(error))
        record['cleanup_errors'] = errors
        record['cleanup_pending'] = bool(errors)
        if errors:
            record['status'] = 'failed'
        (evidence / 'report.json').write_text(json.dumps(record, indent=2) + '\n')
    print(f"{name}: {record['status']}", flush=True)
    return record


def summarize(records: list[dict]) -> dict:
    """A nonempty successful executed set is required; pending scenarios never count as passes."""
    passed = sum(record['status'] == 'passed' for record in records)
    pending = sum(record['status'] == 'pending' for record in records)
    failed = sum(record['status'] not in ('passed', 'pending') for record in records)
    return {'passed': passed, 'pending': pending, 'failed': failed,
            'status': 'passed' if passed and not failed else 'failed'}


def execute_simulation(names: list[str], scenarios: dict, interactive: bool) -> int:
    global COMMAND_LOG
    config = load_config()
    reports = ROOT / json.loads((ROOT / 'tests/config.json').read_text())['report_directory']
    reports.mkdir(parents=True, exist_ok=True)
    output = Path(tempfile.mkdtemp(prefix='update-sim-', dir=reports))
    COMMAND_LOG = output / 'commands.log'
    sandbox = Path(tempfile.mkdtemp(prefix='isolated-', dir=output))
    native = json.loads((ROOT / 'tests/e2e/native/config.json').read_text())
    app_id = native['appId'] + '.' + uuid.uuid4().hex
    report = {'app_id': app_id, 'sandbox': str(sandbox), 'scenarios': [], 'status': 'failed'}
    def interrupt(_signum, _frame):
        raise KeyboardInterrupt('update simulation interrupted')
    previous_term = signal.signal(signal.SIGTERM, interrupt)
    try:
        if sys.platform != 'darwin':
            raise RuntimeError('update simulation requires a macOS graphical host')
        require_nsh()
        helper = sandbox / 'host'
        run(['swiftc', '-module-cache-path', str(sandbox / 'swift-cache'),
             str(ROOT / 'tests/e2e/packaged/host.swift'), '-o', str(helper)])
        def host(*arguments):
            return json.loads(run([str(helper), *arguments]).stdout)
        report['session'] = host('session')
        if report['session'].get('ready') is not True:
            raise RuntimeError('requires an unlocked macOS graphical session')
        keys = make_keys(sandbox / 'keys', config)
        keys['app_id'] = app_id
        active = [name for name in names if not scenarios[name].get('pending') or interactive]
        app = old_app(keys, config, sandbox) if active else None
        if app:
            run(['codesign', '--verify', '--deep', '--strict', str(app)])
        for name in names:
            pending = scenarios[name].get('pending')
            if pending and not interactive:
                report['scenarios'].append({'name': name, 'status': 'pending', 'reason': pending})
                print(f'{name}: pending ({pending})', flush=True)
                continue
            result = {'name': name, 'status': 'failed', 'cleanup_pending': True}
            report['scenarios'].append(result)
            run_scenario(name, scenarios[name], keys, config, app, sandbox, output, host, interactive, result)
            if result['cleanup_errors'] or result['status'] == 'interrupted':
                break
        report.update(summarize(report['scenarios']))
        if interactive and len(report['scenarios']) == 1 and report['scenarios'][0]['status'] == 'observed':
            report.update(status='observed', failed=0, observed=1)
    except (Exception, SystemExit, KeyboardInterrupt) as error:
        report['error'] = str(error)
        report['status'] = 'failed'
        report['failed'] = report.get('failed', 0) + 1
    finally:
        # A surviving process or altered namespace must keep its complete isolation tree.
        retained = any(record.get('cleanup_pending') or record.get('cleanup_errors') for record in report['scenarios'])
        report['retained'] = retained
        if not retained:
            try:
                shutil.rmtree(sandbox)
            except OSError as error:
                report['cleanup_error'] = str(error)
                report['status'] = 'failed'
                report['retained'] = retained = True
        report['cleanup'] = 'retained for inspection' if retained else 'temporary source, app, keys and data removed'
        signal.signal(signal.SIGTERM, previous_term)
        (output / 'report.json').write_text(json.dumps(report, indent=2) + '\n')
        print(f'Update simulation evidence: {output}', flush=True)
        print(f"{report['status']}: {report.get('passed', 0)} passed, {report.get('failed', 0)} failed, {report.get('pending', 0)} pending", flush=True)
    return 0 if report['status'] in ('passed', 'observed') else 1


def main(argv: list[str] | None = None) -> int:
    parser = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    sub = parser.add_subparsers(dest='command', required=True)
    sub.add_parser('list')
    run_parser = sub.add_parser('run')
    run_parser.add_argument('scenario', nargs='?')
    run_parser.add_argument('--all', action='store_true')
    serve_parser = sub.add_parser('serve')
    serve_parser.add_argument('scenario')
    sub.add_parser('clean')
    args = parser.parse_args(argv)
    scenarios = load_scenarios()
    if args.command == 'list':
        for name, scenario in scenarios.items():
            print(f"{name:20} {scenario['description']}" + (' [pending]' if scenario.get('pending') else ''))
        return 0
    # The lock is outside the removable legacy cache; clean cannot remove an active run's lock.
    SIM.parent.mkdir(parents=True, exist_ok=True)
    with (SIM.parent / (SIM.name + '.lock')).open('a') as lock:
        try:
            fcntl.flock(lock, fcntl.LOCK_EX | fcntl.LOCK_NB)
        except BlockingIOError:
            parser.error('another update simulation or cleanup is running')
        if args.command == 'clean':
            shutil.rmtree(SIM, ignore_errors=True)
            print(f'removed legacy simulation cache {SIM}; reports and retained isolation are preserved')
            return 0
        names = list(scenarios) if getattr(args, 'all', False) else [args.scenario]
        if any(name not in scenarios for name in names):
            parser.error('name an existing scenario or pass --all')
        return execute_simulation(names, scenarios, args.command == 'serve')


if __name__ == '__main__':
    raise SystemExit(main())
