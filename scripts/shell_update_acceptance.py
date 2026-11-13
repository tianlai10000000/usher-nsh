#!/usr/bin/env python3
"""Exercise real macOS archive replacement through the production controller in owned sandboxes."""
from __future__ import annotations

import argparse
from collections import deque
import fcntl
import hashlib
import importlib.util
import json
import os
from pathlib import Path
import platform
import shutil
import signal
import subprocess
import sys
import tarfile
import tempfile
import threading
import time
import uuid
import xml.etree.ElementTree as ET

ROOT = Path(__file__).resolve().parents[1]
FIXTURE = ROOT / 'tests/e2e/shell-update'
spec = importlib.util.spec_from_file_location('update_sim', ROOT / 'scripts/update-sim/sim.py')
sim = importlib.util.module_from_spec(spec)
spec.loader.exec_module(sim)
from macos_test_data import AppDataNamespace


def inventory(app: Path) -> dict:
    """Capture bytes, executable modes and links, rather than trusting only Info.plist's version."""
    result = {}
    for path in sorted(app.rglob('*')):
        name = str(path.relative_to(app))
        if path.is_symlink():
            result[name] = {'link': os.readlink(path)}
        elif path.is_file():
            result[name] = {'sha256': hashlib.sha256(path.read_bytes()).hexdigest(),
                            'mode': path.stat().st_mode & 0o777}
    return result


def archive_app(app: Path, target: Path) -> None:
    """Archive the actual signed bundle, keeping executable modes and internal symlinks."""
    with tarfile.open(target, 'w:gz') as archive:
        archive.add(app, arcname=app.name)


def stage_sources(target: Path, version: str, build: int, keys: dict, port: int) -> Path:
    source = sim.project_copy(target, version, build, keys, f'http://127.0.0.1:{port}/latest.json')
    shutil.copytree(FIXTURE, source / 'tests/e2e/shell-update')
    native = source / 'tests/e2e/native/config.json'
    native.parent.mkdir(parents=True)
    shutil.copy2(ROOT / 'tests/e2e/native/config.json', native)
    # Keep a typed empty scheme list without registering any real URL scheme.
    manifest = json.loads((source / 'nsh.json').read_text())
    manifest['schemes'] = []
    (source / 'nsh.json').write_text(json.dumps(manifest, indent=2) + '\n')
    entry = source / 'App.tsx'
    text = entry.read_text()
    anchor = '  const updates = createUpdates(appearance);'
    if text.count(anchor) != 1:
        raise RuntimeError('production update composition anchor missing or ambiguous')
    header, body = text.split('\n', 1)
    entry.write_text(header + '\nimport { driveShellAcceptance } from "./tests/e2e/shell-update/driver";\n'
                     + body.replace(anchor, anchor + '\n  driveShellAcceptance(updates);'))
    return source


def build_app(source: Path, output: Path, app_id: str, timeout: int) -> Path:
    sim.run(['nsh', 'package', '--target', 'macos', '--arch', 'arm64', '-o', str(output)],
            cwd=source, env=sim.build_environment(), timeout=timeout)
    apps = list(output.glob('*.app'))
    if len(apps) != 1:
        raise RuntimeError('expected exactly one real application bundle')
    sim.validate_app(apps[0], app_id)
    sim.run(['codesign', '--verify', '--deep', '--strict', str(apps[0])])
    return apps[0]


class Evidence:
    """Fence commands to a boot session; a restarted process must introduce itself again."""
    def __init__(self):
        self.lock = threading.Lock()
        self.session = 0
        self.commands = deque()
        self.events = []
        self.latest = None
        self.closed = False

    def record(self, event: dict) -> int:
        with self.lock:
            if self.closed:
                raise ValueError('acceptance session closed')
            if event.get('kind') == 'boot':
                self.session += 1
                self.commands.clear()
                event['session'] = self.session
            elif event.get('session') != self.session or not self.session:
                raise ValueError('stale acceptance session')
            self.events.append(event)
            if event.get('kind') in ('boot', 'state'):
                self.latest = event['value']
            return self.session

    def next(self, session: int) -> dict:
        with self.lock:
            if self.closed or session != self.session or not self.commands:
                return {}
            return self.commands.popleft()

    def command(self, action: str) -> int:
        if action not in ('check', 'download', 'install'):
            raise ValueError('unknown update action')
        with self.lock:
            if self.closed or not self.session:
                raise ValueError('no active application')
            self.commands.append({'action': action})
            return len(self.events)

    def snapshot(self):
        with self.lock:
            return self.session, self.latest

    def close(self):
        with self.lock:
            self.closed = True
            self.commands.clear()


class Handler(sim.Handler):
    """A test-only command/evidence endpoint beside the normal signed update source."""
    def json_response(self, status: int, value: dict):
        payload = json.dumps(value).encode()
        self.send_response(status)
        self.send_header('Content-Type', 'application/json')
        self.send_header('Content-Length', str(len(payload)))
        self.end_headers()
        self.wfile.write(payload)

    def do_GET(self):
        from urllib.parse import urlsplit, parse_qs
        url = urlsplit(self.path)
        if url.path != self.server.endpoint:
            return super().do_GET()
        try:
            session = int(parse_qs(url.query)['session'][0])
            self.json_response(200, self.server.evidence.next(session))
        except (KeyError, ValueError):
            self.json_response(400, {'error': 'invalid session'})

    def do_POST(self):
        if self.path != self.server.endpoint:
            return self.json_response(404, {'error': 'unknown endpoint'})
        try:
            length = int(self.headers.get('Content-Length', '0'))
            if not 0 < length <= self.server.body_limit:
                raise ValueError('invalid evidence length')
            event = json.loads(self.rfile.read(length))
            session = self.server.evidence.record(event)
            self.json_response(200, {'session': session})
        except (ValueError, KeyError, TypeError):
            self.json_response(400, {'error': 'invalid evidence'})


def wait_for(read, timeout: float, interval: float, description: str):
    deadline = time.monotonic() + timeout
    while time.monotonic() < deadline:
        value = read()
        if value:
            return value
        time.sleep(interval)
    raise RuntimeError(f'timed out: {description}')


def check_boot(value: dict, version: str, build: int, preferences: dict):
    """Validate evidence from the running process, including settings read after replacement."""
    if value['shell']['version'] != version or value['shell']['build'] != build:
        raise RuntimeError('running shell version/build differs from the expected artifact')
    if value['package'] is not None:
        raise RuntimeError('whole-app acceptance unexpectedly ran a code package')
    settings = value['settings']
    if (not settings['saved'] or settings['colorMode'] != preferences['colorMode']
            or settings['localePreference'] != preferences['localePreference']
            or value['update']['domain']['auto_check'] != preferences['autoCheck']):
        raise RuntimeError('saved preferences were not retained by the running process')


def live_processes(app: Path) -> list[int]:
    """Include NSH's detached restart shell/open helper, which NSRunningApplication cannot see."""
    output = subprocess.check_output(['ps', '-axo', 'uid=,pid=,command='], text=True)
    rows = [line.strip().split(None, 2) for line in output.splitlines()]
    return [int(pid) for uid, pid, command in rows
            if int(uid) == os.getuid() and int(pid) != os.getpid() and str(app.resolve()) in command]


def stop_owned(app: Path, app_id: str, process, host, timeout: float, interval: float):
    """Stop only this bundle's app/restart processes, then prove a quiet interval before detaching data."""
    deadline = time.monotonic() + timeout * 2
    quiet_since = None
    denied = set()
    while time.monotonic() < deadline:
        owned = set(live_processes(app)) | {row['pid'] for row in host('process', app_id)}
        for pid in owned:
            try:
                os.kill(pid, signal.SIGKILL)
            except ProcessLookupError:
                pass
            except PermissionError:
                # A process already exiting can briefly remain in the host inventory.
                # Only a subsequent quiet interval, never this error, proves it stopped.
                denied.add(pid)
        if process and process.poll() is None:
            try:
                os.killpg(process.pid, signal.SIGKILL)
            except ProcessLookupError:
                pass
            except PermissionError:
                denied.add(process.pid)
        if process:
            try:
                process.wait(timeout=interval)
            except subprocess.TimeoutExpired:
                continue
        if not owned:
            quiet_since = quiet_since or time.monotonic()
            if time.monotonic() - quiet_since >= timeout:
                return
        else:
            quiet_since = None
        time.sleep(interval)
    raise RuntimeError(f'could not prove the application and restart helpers stopped; retain isolation; denied PIDs: {sorted(denied)}')


def write_settings(path: Path, config: dict) -> bytes:
    text = (f'schemaVersion = {config["schemaVersion"]}\n\n[appearance]\ncolorMode = {json.dumps(config["colorMode"])}\n'
            f'\n[general]\nlocalePreference = {json.dumps(config["localePreference"])}\n'
            f'\n[updates]\nauto_check = {str(config["autoCheck"]).lower()}\n')
    path.write_text(text)
    return text.encode()


def prepare_case(case: str, directory: Path, new: Path, source: Path, keys: dict, version: str):
    """Sign the manifest after inner-bundle faults, but before a transport hash fault."""
    bundle = directory / 'payload' / new.name
    bundle.parent.mkdir()
    shutil.copytree(new, bundle, symlinks=True)
    metadata = bundle / 'Contents/Resources/nsh/app.json'
    if case in ('invalid-signature', 'wrong-identity'):
        data = json.loads(metadata.read_text())
        if case == 'wrong-identity':
            data['id'] += '.foreign'
        else:
            data['acceptanceTampered'] = True
        metadata.write_text(json.dumps(data))
    if case == 'wrong-identity':
        sim.run(['codesign', '--force', '--sign', '-', '--timestamp=none', str(bundle)])
    served = directory / 'served'
    served.mkdir()
    archive = served / f'Usher-NSH-{version}-macos-aarch64.tar.gz'
    archive_app(bundle, archive)
    sim.run(['nsh', 'update-manifest', '--entry', str(source), '--key', str(keys['dir'] / 'manifest.secret'),
             '-o', str(served), str(archive)])
    pristine = directory / 'pristine.tar.gz'
    if case == 'archive-tampered':
        shutil.copy2(archive, pristine)
        sim.apply_tamper(served, {'file': archive.name, 'kind': 'flip-byte'})
    return served, archive, pristine


def run_case(case, root, output, old, new, source, keys, config, native, fixture, host, port, record):
    directory = root / case
    directory.mkdir()
    evidence_dir = output / case
    evidence_dir.mkdir()
    installed = directory / 'installed'
    installed.mkdir()
    app = installed / old.name
    shutil.copytree(old, app, symlinks=True)
    before = inventory(app)
    expected = inventory(new)
    namespace = AppDataNamespace(host('directories'), keys['app_id'], directory / 'runtime')
    evidence = Evidence()
    server = process = None
    interval = fixture['pollIntervalMs'] / 1000
    timeout = native['timeoutSeconds']
    prefs = fixture['settings']
    target = json.loads((source / 'nsh.json').read_text())
    platform_config = json.loads((ROOT / 'tests/e2e/packaged/config.json').read_text())
    def state():
        return evidence.snapshot()[1]
    def phase(name):
        value = state()
        return value if value and value['update']['domain']['phase']['type'] == name else None
    def wait(read, message):
        return wait_for(read, timeout, interval, message)
    def launch():
        nonlocal process
        previous = evidence.snapshot()[0]
        with (evidence_dir / 'app.log').open('ab') as log:
            process = subprocess.Popen([str(sim.validate_app(app, keys['app_id']))], stdout=log, stderr=log, start_new_session=True)
        wait(lambda: evidence.snapshot()[0] > previous, 'boot evidence')
        check_boot(state(), config['oldVersion'], config['oldBuild'], prefs)
        return process.pid
    def find_update():
        evidence.command('check')
        value = wait(lambda: phase('checked'), 'whole-app offer')
        outcome = value['update']['domain']['phase']['outcome']
        if outcome['type'] != 'shell_update' or outcome['candidate']['version'] != target['version']:
            raise RuntimeError(f'expected an installable whole-app offer: {outcome}')
    def download():
        evidence.command('download')
        wait(lambda: phase('ready_to_install'), 'real archive download/verification')
    def confirm_failure(value):
        if value['update']['domain']['phase']['failure'] != fixture['failureClasses'][case]:
            raise RuntimeError('native failure was classified into the wrong product category')
    def assert_unchanged():
        if inventory(app) != before:
            raise RuntimeError('failed update changed the installed application')
        if list(installed.glob('.nsh-updater-*')):
            raise RuntimeError('installer left a staging or backup directory')
    print(f'Running shell {case}', flush=True)
    try:
        namespace.create()
        settings = namespace.root / 'data/settings.toml'
        saved = write_settings(settings, prefs)
        served, archive, pristine = prepare_case(case, directory, new, source, keys, target['version'])
        server = sim.Source(served, {}, port)
        server.RequestHandlerClass = Handler
        server.endpoint = fixture['endpoint']
        server.body_limit = fixture['evidenceLimitBytes']
        server.evidence = evidence
        threading.Thread(target=server.serve_forever, daemon=True).start()
        initial_pid = launch()
        record['initial_pid'] = initial_pid
        find_update()
        if any('.tar.gz' in path for path in server.requests):
            raise RuntimeError('whole-app archive downloaded without explicit confirmation')
        if case == 'archive-tampered':
            evidence.command('download')
            failed = wait(lambda: phase('install_failed'), 'corrupt archive refusal')
            confirm_failure(failed)
            assert_unchanged()
            record['failure'] = failed
            stop_owned(app, keys['app_id'], process, host, config['processTimeoutSeconds'], interval)
            launch()
            shutil.copy2(pristine, archive)
            find_update()
        download()
        record['ready'] = state()
        if case == 'install-denied':
            installed.chmod(0o555)
        prior_session = evidence.snapshot()[0]
        evidence.command('install')
        if case in ('invalid-signature', 'wrong-identity', 'install-denied'):
            failed = wait(lambda: phase('install_failed'), 'installation refusal')
            confirm_failure(failed)
            installed.chmod(0o755)
            assert_unchanged()
            record['failure'] = failed
            stop_owned(app, keys['app_id'], process, host, config['processTimeoutSeconds'], interval)
            launch()
            record['old_relaunch'] = state()
            if case == 'install-denied':
                find_update()
                download()
                prior_session = evidence.snapshot()[0]
                evidence.command('install')
        if case not in ('invalid-signature', 'wrong-identity'):
            # Poll/reap the old child so NSH's detached restart helper sees its PID disappear.
            wait(lambda: process.poll() is not None, 'old process exit')
            wait(lambda: evidence.snapshot()[0] > prior_session, 'NSH-initiated new process boot')
            check_boot(state(), target['version'], target['build'], prefs)
            running = wait(lambda: host('process', keys['app_id']), 'new registered native process')
            if any(row['pid'] == initial_pid for row in running):
                raise RuntimeError('the old process is still registered')
            if inventory(app) != expected:
                raise RuntimeError('installed files differ from the actual new application')
            sim.run(['codesign', '--verify', '--deep', '--strict', str(app)])
            record['new_processes'] = running
            record['new_boot'] = state()
            record['installed_inventory'] = expected
        if settings.read_bytes() != saved:
            raise RuntimeError('update changed persisted settings bytes')
        if list(installed.glob('.nsh-updater-*')):
            raise RuntimeError('staging or old installation remains')
        if any('.nshpkg' in path for path in server.requests):
            raise RuntimeError('whole-app path unexpectedly downloaded a code package')
        record['settings_sha256'] = hashlib.sha256(saved).hexdigest()
        record['archive_sha256'] = hashlib.sha256(archive.read_bytes()).hexdigest()
        record['status'] = 'passed'
    except (Exception, KeyboardInterrupt) as error:
        record['error'] = str(error) or type(error).__name__
        record['interrupted'] = isinstance(error, KeyboardInterrupt)
    finally:
        installed.chmod(0o755)
        evidence.close()
        record['events'] = evidence.events
        record['isolated_roots'] = namespace.evidence()
        record['requests'] = list(server.requests) if server else []
        try:
            stop_owned(app, keys['app_id'], process, host, config['processTimeoutSeconds'], interval)
            sim.run([platform_config['lsregister'], '-u', str(app)])
            if server:
                server.shutdown()
                server.server_close()
            logs = '\n'.join(p.read_text(errors='replace') for p in sorted((namespace.root / 'data').rglob('usher*.log')))
            (evidence_dir / 'diagnostics.log').write_text(logs)
            expected_failure = fixture.get('failureDetails', {}).get(case)
            if record['status'] == 'passed' and expected_failure and not any(
                    '"native_failure"' in line and any(fragment in line.lower() for fragment in expected_failure)
                    for line in logs.splitlines()):
                record['status'] = 'failed'
                record['error'] = 'native diagnostic did not confirm the injected fault'
            namespace.cleanup(lambda: bool(host('process', keys['app_id']) or live_processes(app)))
            record['cleanup_pending'] = False
        except (Exception, KeyboardInterrupt) as error:
            record['cleanup_error'] = str(error) or type(error).__name__
            record['status'] = 'failed'
        (evidence_dir / 'report.json').write_text(json.dumps(record, indent=2) + '\n')
    print(f'shell {case}: {record["status"]}', flush=True)
    return record


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--junit', type=Path, required=True)
    args = parser.parse_args()
    args.junit.parent.mkdir(parents=True, exist_ok=True)
    args.junit.unlink(missing_ok=True)
    output = Path(tempfile.mkdtemp(prefix='shell-update-', dir=args.junit.parent.resolve()))
    root = Path(tempfile.mkdtemp(prefix='isolated-', dir=output))
    fixture = json.loads((FIXTURE / 'config.json').read_text())
    config = sim.load_config()
    native = json.loads((ROOT / 'tests/e2e/native/config.json').read_text())
    release = json.loads((ROOT / 'scripts/update-sim/scenarios/shell-update.json').read_text())['manifest']
    report = {'cases': [], 'status': 'failed', 'sandbox': str(root)}
    sim.COMMAND_LOG = output / 'commands.log'
    started = time.monotonic()
    def interrupt(_signum, _frame):
        raise KeyboardInterrupt('shell acceptance interrupted')
    previous = signal.signal(signal.SIGTERM, interrupt)
    try:
        if sys.platform != 'darwin' or platform.machine() != 'arm64':
            raise RuntimeError('requires an unlocked macOS arm64 host')
        helper = root / 'host'
        sim.run(['swiftc', '-module-cache-path', str(root / 'swift-cache'), str(ROOT / 'tests/e2e/packaged/host.swift'), '-o', str(helper)])
        def host(*args):
            return json.loads(sim.run([str(helper), *args]).stdout)
        if not host('session')['ready']:
            raise RuntimeError('requires an unlocked graphical session')
        keys = sim.make_keys(root / 'keys', config)
        keys['app_id'] = native['appId'] + '.' + uuid.uuid4().hex
        report['app_id'] = keys['app_id']
        # Reserve a free loopback port before building the immutable updater URL into both apps.
        with sim.socket.socket() as reserved:
            reserved.bind(('127.0.0.1', 0))
            port = reserved.getsockname()[1]
        old_source = stage_sources(root / 'old-src', config['oldVersion'], config['oldBuild'], keys, port)
        new_source = stage_sources(root / 'new-src', release['version'], release['build'], keys, port)
        print('Building two real isolated applications', flush=True)
        old = build_app(old_source, root / 'old-out', keys['app_id'], config['buildTimeoutSeconds'])
        # A valid obsolete resource proves the swap does not merely overlay the new files.
        (old / 'Contents/Resources' / fixture['retiredResource']).write_text(config['oldVersion'])
        sim.run(['codesign', '--force', '--sign', '-', '--timestamp=none', str(old)])
        new = build_app(new_source, root / 'new-out', keys['app_id'], config['buildTimeoutSeconds'])
        report['old_inventory'] = inventory(old)
        report['new_inventory'] = inventory(new)
        for case in fixture['cases']:
            # Register ownership before entering the case, including interruption during cleanup.
            result = {'name': case, 'status': 'failed', 'cleanup_pending': True}
            report['cases'].append(result)
            run_case(case, root, output, old, new, new_source, keys, config, native, fixture, host, port, result)
            if result.get('cleanup_pending') or result.get('interrupted'):
                break
        if len(report['cases']) == len(fixture['cases']) and all(c['status'] == 'passed' for c in report['cases']):
            report['status'] = 'passed'
    except (Exception, KeyboardInterrupt) as error:
        report['error'] = str(error)
    finally:
        signal.signal(signal.SIGTERM, previous)
        for case in report['cases']:
            if case.get('cleanup_pending'):
                case['status'] = 'failed'
        report['retained'] = any(c.get('cleanup_pending') for c in report['cases'])
        if not report['retained']:
            shutil.rmtree(root)
        (output / 'report.json').write_text(json.dumps(report, indent=2) + '\n')
        results = {c['name']: c for c in report['cases']}
        suite = ET.Element('testsuite', name='shell-update', tests=str(len(fixture['cases'])),
                           time=str(time.monotonic() - started))
        for name in fixture['cases']:
            case = ET.SubElement(suite, 'testcase', classname=str((FIXTURE / 'driver.ts').relative_to(ROOT)), name='shell archive ' + name)
            result = results.get(name, {})
            if result.get('status') != 'passed':
                message = result.get('error') or result.get('cleanup_error') or report.get('error') or 'scenario did not execute'
                ET.SubElement(case, 'failure', message=message).text = message
        suite.set('failures', str(len(list(suite.iter('failure')))))
        ET.ElementTree(suite).write(args.junit, encoding='unicode', xml_declaration=True)
        print(f'Shell acceptance: {report["status"]}; evidence: {output}', flush=True)
    return 0 if report['status'] == 'passed' else 1


def locked_main():
    """Share the local simulator lock so native builds and namespace owners run serially."""
    sim.SIM.parent.mkdir(parents=True, exist_ok=True)
    with (sim.SIM.parent / (sim.SIM.name + '.lock')).open('a') as lock:
        try:
            fcntl.flock(lock, fcntl.LOCK_EX | fcntl.LOCK_NB)
        except BlockingIOError:
            print('another local update acceptance command is running', file=sys.stderr)
            return 1
        return main()


if __name__ == '__main__':
    raise SystemExit(locked_main())
