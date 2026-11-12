#!/usr/bin/env python3
"""Local update simulation: runs the app's real update path against a mock update source.

It builds an "old" app whose `updater.url` points at a local HTTP server, serves a manifest
and packages the scenario describes, launches the app in an isolated HOME and asserts what
ended up in the app's update directory. Nothing here touches GitHub or a real key: every run
generates one-off keys under `.nsh/update-sim/`.

Subcommands: list, run <scenario>|--all, serve <scenario>, clean. See docs/update.md, section 12.

The slow step is `nsh package` (about five minutes, it compiles the Rust crate), so the old
app is cached by a hash of the shell inputs. Packages, manifests and signatures are cheap and
are rebuilt for every scenario.

NSH accepts plain HTTP only for 127.0.0.1, localhost and [::1] (nsh-updater `check_url`); the
rest of the verification path is the same as with a real HTTPS source.
"""

from __future__ import annotations

import argparse
import hashlib
import http.server
import io
import json
import os
import shutil
import signal
import socket
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

# Paths hashed to decide whether the cached old app is still valid. Mirrors the shell inputs
# of docs/update.md section 4.2; `nsh.json` is read separately because its version and build
# change on every release.
SHELL_INPUTS = ["src", "platforms", "crates", "build.rs", "Cargo.toml", "Cargo.lock", "ui/theme", "ui/config/background.json"]


def load_config() -> dict:
    return json.loads(CONFIG.read_text())


def fail(message: str) -> "None":
    print(f"error: {message}", file=sys.stderr)
    raise SystemExit(1)


def run(cmd: list[str], **kwargs) -> subprocess.CompletedProcess:
    result = subprocess.run(cmd, capture_output=True, text=True, **kwargs)
    if result.returncode != 0:
        fail(f"{' '.join(map(str, cmd))} failed:\n{result.stdout}{result.stderr}")
    return result


def require_nsh() -> None:
    if shutil.which("nsh") is None:
        fail("the nsh command line is not installed (scripts/ci/install-nsh.sh)")


def shell_hash() -> str:
    digest = hashlib.sha256()
    for entry in SHELL_INPUTS:
        base = ROOT / entry
        files = [base] if base.is_file() else sorted(p for p in base.rglob("*") if p.is_file()) if base.exists() else []
        for file in files:
            digest.update(str(file.relative_to(ROOT)).encode())
            digest.update(file.read_bytes())
    # The NSH version is part of the shell, and so is everything the old app is configured with.
    digest.update(subprocess.run(["nsh", "--version"], capture_output=True, text=True).stdout.encode())
    digest.update(CONFIG.read_bytes())
    return digest.hexdigest()[:16]


def free_port() -> int:
    with socket.socket() as sock:
        sock.bind(("127.0.0.1", 0))
        return sock.getsockname()[1]


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

    The repository's nsh.json is never modified: the simulation works on copies in .nsh/update-sim/.
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
    manifest["version"] = version
    manifest["build"] = build
    if url is not None:
        manifest["updater"] = {"url": url, "publicKey": keys["manifest_pub"], "packages": {"roots": [keys["root_pub"]]}}
    (destination / "nsh.json").write_text(json.dumps(manifest, indent=2))
    return destination


# --- the old app (cached) -------------------------------------------------------------------------


def old_app(keys: dict, config: dict, port: int) -> Path:
    """The installed "old" app. Built once per shell hash; its source URL is the fixed sim port."""
    key = shell_hash()
    cache = SIM / "apps" / key
    app = cache / "Usher NSH.app"
    if app.exists():
        return app
    print(f"building the old app (cache miss {key}); this compiles the Rust crate and takes minutes", flush=True)
    project = project_copy(SIM / "work" / "old-src", config["oldVersion"], config["oldBuild"], keys,
                           f"http://127.0.0.1:{config['port']}/latest.json")
    cache.mkdir(parents=True, exist_ok=True)
    run(["nsh", "package", "--target", "macos", "-o", str(cache)], cwd=project)
    if not app.exists():
        fail("nsh package produced no .app")
    return app


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
             "-o", str(package)], cwd=project)
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
        data = bytearray(data.replace(b"com.tianlai10000000.usher.nsh", b"com.example.other.app....."))
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


def updates_dir(home: Path, app_id: str) -> Path:
    return home / "Library" / "Application Support" / app_id / "updates"


def state_of(directory: Path) -> dict:
    """What the update directory holds, as the assertions see it."""
    names = sorted(p.name for p in directory.iterdir()) if directory.exists() else []
    return {"files": names, "has": {name: name in names for name in ["next.nshpkg", "trying.nshpkg", "current.nshpkg", "trust"]},
            "partial": [n for n in names if n.endswith(".part")]}


def launch(app: Path, home: Path, seconds: float, log: Path) -> None:
    """Starts the app in an isolated HOME for `seconds`, then stops it the way a crash would."""
    executable = next((app / "Contents" / "MacOS").iterdir())
    env = {**os.environ, "HOME": str(home)}
    with log.open("ab") as out:
        process = subprocess.Popen([str(executable)], env=env, stdout=out, stderr=out, start_new_session=True)
        try:
            time.sleep(seconds)
        finally:
            try:
                os.killpg(process.pid, signal.SIGKILL)
            except ProcessLookupError:
                pass
            process.wait()


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


def preload(scenario: dict, served: Path, home: Path, app_id: str) -> None:
    """Puts the package into `next.nshpkg` the way a finished download would, for scenarios that
    start from a downloaded package (the launch path), which needs no network at all."""
    if scenario.get("preload") != "next":
        return
    package = next(served.glob("*.nshpkg"))
    target = updates_dir(home, app_id)
    target.mkdir(parents=True, exist_ok=True)
    shutil.copy2(package, target / "next.nshpkg")


def run_scenario(name: str, scenario: dict, keys: dict, config: dict) -> bool | None:
    """True: passed. False: failed. None: skipped, the scenario is marked pending."""
    print(f"\n== {name}: {scenario['description']}", flush=True)
    pending = scenario.get("pending")
    if pending:
        # A pending scenario needs something the app does not do yet. Running it would only
        # prove that nothing happened, which must not be reported as a pass.
        print(f"   SKIP pending: {pending}")
        return None
    directory = SIM / "runs" / name
    if directory.exists():
        shutil.rmtree(directory)
    directory.mkdir(parents=True)
    app = old_app(keys, config, config["port"])
    served = build_artifacts(scenario, keys, config, directory)
    home = directory / "home"
    home.mkdir()
    app_id = json.loads((ROOT / "nsh.json").read_text())["id"]
    preload(scenario, served, home, app_id)

    source = start_source(served, scenario.get("source", {}), config["port"])
    log = directory / "app.log"
    try:
        for step in scenario.get("launches", [{"seconds": config["launchSeconds"]}]):
            launch(app, home, step["seconds"], log)
    finally:
        requests = list(source.requests) if source else []
        if source:
            source.shutdown()
            source.server_close()
    state = state_of(updates_dir(home, app_id))
    app_log = log.read_text(errors="replace") if log.exists() else ""
    problems = check_expectations(name, scenario["expect"], state, {"log": app_log, "requests": requests})
    for problem in problems:
        print(f"   FAIL {problem}")
    if not problems:
        print(f"   PASS ({len(scenario['expect'])} expectations; state {state['files']})")
    return not problems


def main(argv: list[str] | None = None) -> int:
    parser = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    sub = parser.add_subparsers(dest="command", required=True)
    sub.add_parser("list")
    run_parser = sub.add_parser("run")
    run_parser.add_argument("scenario", nargs="?")
    run_parser.add_argument("--all", action="store_true")
    serve_parser = sub.add_parser("serve")
    serve_parser.add_argument("scenario")
    sub.add_parser("clean")
    args = parser.parse_args(argv)

    scenarios = load_scenarios()
    if args.command == "list":
        for name, scenario in scenarios.items():
            mark = "  [pending]" if scenario.get("pending") else ""
            print(f"{name:20} {scenario['description']}{mark}")
        return 0
    if args.command == "clean":
        shutil.rmtree(SIM, ignore_errors=True)
        print(f"removed {SIM}")
        return 0

    require_nsh()
    config = load_config()
    keys = make_keys(SIM / "keys", config)
    if args.command == "serve":
        scenario = scenarios.get(args.scenario) or fail(f"unknown scenario {args.scenario}")
        directory = SIM / "runs" / args.scenario
        served = build_artifacts(scenario, keys, config, directory)
        source = start_source(served, scenario.get("source", {}), config["port"])
        app = old_app(keys, config, config["port"])
        print(f"serving {served} on http://127.0.0.1:{config['port']}/latest.json")
        print(f'open the old app in an isolated HOME:\n  HOME="{directory}/home" "{next((app / "Contents" / "MacOS").iterdir())}"')
        try:
            while True:
                time.sleep(3600)
        except KeyboardInterrupt:
            if source:
                source.shutdown()
        return 0

    if args.all:
        names = list(scenarios)
    elif args.scenario:
        if args.scenario not in scenarios:
            fail(f"unknown scenario {args.scenario}")
        names = [args.scenario]
    else:
        fail("name a scenario or pass --all")
    results = {name: run_scenario(name, scenarios[name], keys, config) for name in names}
    passed = sum(1 for result in results.values() if result is True)
    failed = sum(1 for result in results.values() if result is False)
    skipped = sum(1 for result in results.values() if result is None)
    print(f"\n{passed} passed, {failed} failed, {skipped} skipped (pending)")
    return 0 if failed == 0 else 1


if __name__ == "__main__":
    raise SystemExit(main())
