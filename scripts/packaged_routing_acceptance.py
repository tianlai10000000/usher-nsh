#!/usr/bin/env python3
"""Verify real macOS release scheme registration and URL launch with isolated test namespaces."""
from __future__ import annotations

import argparse
import hashlib
import json
import os
from pathlib import Path
import plistlib
import shutil
import signal
import subprocess
import sys
import tempfile
import time
import uuid
import xml.etree.ElementTree as ET

ROOT = Path(__file__).resolve().parents[1]
FIXTURE = ROOT / 'tests/e2e/packaged'

sys.path.insert(0, str(Path(__file__).resolve().parent))
from macos_test_data import create_link, remove_link


def stage_sources(destination: Path, app_id: str) -> Path:
    """Copy the working sources, adding only a read-only route observation in the test copy."""
    destination.mkdir()
    inventory = subprocess.check_output(['git', 'ls-files', '-z', '-c', '-o', '--exclude-standard'], cwd=ROOT)
    for relative in set(inventory.decode().split('\0')):
        source = ROOT / relative
        if relative and source.is_file():
            target = destination / relative
            target.parent.mkdir(parents=True, exist_ok=True)
            shutil.copy2(source, target)
    manifest = json.loads((destination / 'nsh.json').read_text())
    manifest['id'] = app_id
    # Rust sources are read-only and unchanged by this fixture; retain their stable
    # path so repeated UI-only acceptance does not relink a new Rust dependency graph.
    manifest['rust'] = str(ROOT)
    manifest.pop('updater', None)
    (destination / 'nsh.json').write_text(json.dumps(manifest, indent=2) + '\n')
    entry = destination / 'App.tsx'
    text = entry.read_text()
    anchor = '  bindNativeRouting(navigation);'
    if text.count(anchor) != 1:
        raise RuntimeError('product routing observation anchor is missing or ambiguous')
    header, body = text.split('\n', 1)
    text = header + '\nimport { observePackagedRouter } from "./tests/e2e/packaged/observe";\n' + body
    entry.write_text(text.replace(anchor, anchor + '\n  observePackagedRouter(navigation);'))
    return destination


def wait_for(read, timeout: float, description: str):
    deadline = time.monotonic() + timeout
    while time.monotonic() < deadline:
        value = read()
        if value:
            return value
        time.sleep(0.1)
    raise RuntimeError(f'timed out: {description}')


def route_events(path: Path, marker: str) -> list[dict]:
    events = []
    if path.exists():
        for line in path.read_text(errors='replace').splitlines():
            if marker in line:
                try:
                    events.append(json.JSONDecoder().raw_decode(line.split(marker, 1)[1])[0])
                except ValueError:
                    pass
    return events


def require_unlocked(session: dict) -> None:
    """Reject an unavailable desktop before interpreting focus assertions as product failures."""
    if session.get('ready') is not True:
        raise RuntimeError('requires an unlocked foreground macOS session; unlock the Mac and rerun')


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--junit', type=Path, required=True)
    args = parser.parse_args()
    args.junit.parent.mkdir(parents=True, exist_ok=True)
    args.junit.unlink(missing_ok=True)
    output = Path(tempfile.mkdtemp(prefix='packaged-routing-', dir=args.junit.parent.resolve()))
    config = json.loads((FIXTURE / 'config.json').read_text())
    native = json.loads((ROOT / 'tests/e2e/native/config.json').read_text())
    manifest = json.loads((ROOT / 'nsh.json').read_text())
    policy = json.loads((ROOT / 'ui/config/routing.json').read_text())
    catalog = json.loads((ROOT / 'ui/pages/routes.json').read_text())
    app_id = native['appId'] + '.' + uuid.uuid4().hex
    origin = f"{manifest['schemes'][0]}://{policy['hosts'][0]}"
    targets = {route['id']: origin + route['path'] for route in catalog['routes']}
    events: dict = {}
    error = None
    cleanup_errors = []
    started = time.monotonic()
    def interrupt(_signum, _frame):
        raise KeyboardInterrupt('packaged acceptance interrupted')
    previous_term = signal.signal(signal.SIGTERM, interrupt)
    report = {'app_id': app_id, 'logs': str(output), 'events': events}
    links: list[tuple[Path, Path]] = []
    registered = False
    app = None
    original_handler = None
    host = None
    temporary = None
    try:
        if sys.platform != 'darwin':
            raise RuntimeError('packaged acceptance requires macOS with an unlocked graphical session')
        temporary = tempfile.TemporaryDirectory(prefix='isolated-', dir=output)
        sandbox = Path(temporary.name).resolve()
        source = stage_sources(sandbox / 'source', app_id)
        helper = sandbox / 'host'
        with (output / 'helper-build.log').open('w') as log:
            subprocess.run(['swiftc', '-module-cache-path', str(sandbox / 'swift-cache'), str(FIXTURE / 'host.swift'), '-o', str(helper)], stdout=log, stderr=subprocess.STDOUT, check=True)
        def host(*arguments):
            return json.loads(subprocess.check_output([str(helper), *arguments], text=True))
        report['session_before_build'] = host('session')
        require_unlocked(report['session_before_build'])
        directories = host('directories')
        for key in ('data', 'cache'):
            link = Path(directories[key]) / app_id
            target = sandbox / key
            create_link(link, target)
            links.append((link, target))
        state_link = Path(directories['library']) / 'Saved Application State' / (app_id + '.savedState')
        create_link(state_link, sandbox / 'saved-state')
        links.append((state_link, sandbox / 'saved-state'))
        report['isolated_roots'] = {str(link): str(target) for link, target in links}
        original_handler = host('handlers', manifest['schemes'][0], targets['settings'])['default']
        report['original_handler'] = original_handler
        env = {**os.environ, 'USHER_MACOS_APPKIT': '0', 'CARGO_INCREMENTAL': '0',
               'CARGO_TARGET_DIR': str(ROOT / 'target/packaged-routing-build')}
        command = ['nsh', 'package', '--target', 'macos', '--arch', 'arm64', '--out', str(sandbox / 'out')]
        with (output / 'package.log').open('w') as log:
            subprocess.run(command, cwd=source, env=env, stdout=log, stderr=subprocess.STDOUT, check=True, timeout=config['buildTimeoutSeconds'])
        report['build_command'] = command
        apps = list((sandbox / 'out').glob('*.app'))
        if len(apps) != 1:
            raise RuntimeError('packaging did not produce exactly one application')
        app = apps[0]
        info = plistlib.loads((app / 'Contents/Info.plist').read_bytes())
        schemes = [scheme for entry in info.get('CFBundleURLTypes', []) for scheme in entry.get('CFBundleURLSchemes', [])]
        if info['CFBundleIdentifier'] != app_id or schemes != manifest['schemes']:
            raise RuntimeError('packaged Info.plist does not declare the expected identity and schemes')
        subprocess.run(['codesign', '--verify', '--deep', '--strict', str(app)], check=True, capture_output=True)
        report['artifact_sha256'] = {str(path.relative_to(app)): hashlib.sha256(path.read_bytes()).hexdigest()
                                     for path in app.rglob('*') if path.is_file() and not path.is_symlink()}
        report['session_before_launch'] = host('session')
        require_unlocked(report['session_before_launch'])
        host('register', str(app)); registered = True
        registration = host('handlers', manifest['schemes'][0], targets['settings'])
        if app_id not in registration['handlers'] or str(app) not in registration['applications']:
            raise RuntimeError('LaunchServices did not register the packaged URL handler')
        if original_handler and registration['default'] == app_id:
            host('restore', manifest['schemes'][0], original_handler)
        events['registration'] = {'schemes': schemes, 'handlers': registration, 'signature': 'verified ad hoc release'}
        if host('process', app_id):
            raise RuntimeError('cold-launch precondition failed: test app is already running')
        route_log = output / 'application.log'
        # Use bare URLs only when LaunchServices already selects this test app.
        # Otherwise explicit selection protects the user's existing scheme handler.
        selected = host('handlers', manifest['schemes'][0], targets['settings'])['default']
        selection = [] if selected == app_id else ['-a', str(app)]
        report['dispatch'] = 'system-default' if not selection else 'explicit-test-application'
        subprocess.run(['open', *selection, '--stderr', str(route_log), targets['settings']], check=True)
        wait_for(lambda: any(e['route'] == 'settings' and e['pending'] is None for e in route_events(route_log, config['marker'])), config['timeoutSeconds'], 'cold URL reaches settings')
        processes = host('process', app_id)
        if len(processes) != 1:
            raise RuntimeError('cold launch did not create exactly one process')
        pid = processes[0]['pid']
        events['cold_launch'] = {'pid': pid, 'url': targets['settings'], 'routes': route_events(route_log, config['marker'])}
        # Keep cold URL launch and warm activation independent: the latter starts
        # with an ordinary explicitly opened foreground application.
        host('terminate', app_id)
        wait_for(lambda: not host('process', app_id), config['timeoutSeconds'], 'cold-launch process exits')
        route_log = output / 'warm-application.log'
        subprocess.run(['open', '-a', str(app), '--stderr', str(route_log)], check=True)
        wait_for(lambda: any(e['route'] == 'home' and e['pending'] is None for e in route_events(route_log, config['marker'])),
                 config['timeoutSeconds'], 'ordinary app launch reaches home')
        processes = wait_for(lambda: (p := host('process', app_id)) and len(p) == 1 and p[0]['active'] and p,
                             config['timeoutSeconds'], 'ordinary launch is foreground')
        pid = processes[0]['pid']
        report['warm_setup'] = processes[0]
        host('hide', app_id)
        wait_for(lambda: (p := host('process', app_id)) and p[0]['hidden'], config['timeoutSeconds'], 'test app hidden')
        previous = len(route_events(route_log, config['marker']))
        selected = host('handlers', manifest['schemes'][0], targets['settings'])['default']
        selection = [] if selected == app_id else ['-a', str(app)]
        subprocess.run(['open', *selection, targets['settings']], check=True)
        wait_for(lambda: any(e['route'] == 'settings' and e['pending'] is None for e in route_events(route_log, config['marker'])[previous:]), config['timeoutSeconds'], 'warm URL reaches settings')
        restored = wait_for(lambda: (p := host('process', app_id)) and len(p) == 1 and p[0]['pid'] == pid and not p[0]['hidden'] and p,
                            config['timeoutSeconds'], 'bare URL restores the same hidden process')
        report['bare_url_wake'] = {'process': restored[0], 'routes': route_events(route_log, config['marker'])[previous:]}
        # macOS may decline activation requested by a background URL sender. An
        # explicit application open additionally requests foreground activation.
        host('hide', app_id)
        wait_for(lambda: (p := host('process', app_id)) and p[0]['hidden'], config['timeoutSeconds'], 'test app hidden again')
        previous = len(route_events(route_log, config['marker']))
        subprocess.run(['open', '-a', str(app), targets['home']], check=True)
        wait_for(lambda: any(e['route'] == 'home' and e['pending'] is None for e in route_events(route_log, config['marker'])[previous:]),
                 config['timeoutSeconds'], 'explicit application URL reaches home')
        def awakened():
            processes = host('process', app_id)
            report['last_wake_processes'] = processes
            return (processes if len(processes) == 1 and processes[0]['pid'] == pid
                    and not processes[0]['hidden'] and processes[0]['active'] else None)
        processes = wait_for(awakened, config['timeoutSeconds'], 'same process wakes and becomes active')
        events['warm_wake'] = {'dispatch': 'explicit-test-application', 'url': targets['home'], 'process': processes[0], 'routes': route_events(route_log, config['marker'])[previous:]}
        report['data_files'] = sorted(str(path.relative_to(sandbox)) for root in (sandbox / 'data', sandbox / 'cache') for path in root.rglob('*') if path.is_file())
        report['status'] = 'passed'
    except (OSError, ValueError, RuntimeError, subprocess.SubprocessError, KeyboardInterrupt) as failure:
        error = str(failure)
        if host:
            try:
                report['session_at_failure'] = host('session')
            except (OSError, ValueError, subprocess.SubprocessError):
                pass
        report['status'] = 'failed'
        report['error'] = error
    finally:
        safe_to_remove = True
        if host:
            try:
                host('terminate', app_id)
                wait_for(lambda: not host('process', app_id), config['timeoutSeconds'], 'test process exits')
            except Exception as failure:
                try:
                    host('force', app_id)
                    wait_for(lambda: not host('process', app_id), config['timeoutSeconds'], 'test process force exits')
                except Exception as force_failure:
                    safe_to_remove = False
                    cleanup_errors.extend([str(failure), str(force_failure)])
        if registered and app:
            try:
                subprocess.run([config['lsregister'], '-u', str(app)], check=True, capture_output=True)
                current = host('handlers', manifest['schemes'][0], targets['settings'])
                if original_handler and current['default'] == app_id:
                    host('restore', manifest['schemes'][0], original_handler)
                remaining = host('handlers', manifest['schemes'][0], targets['settings'])
                if app_id in remaining['handlers'] or remaining['default'] == app_id:
                    raise RuntimeError('test handler still registered after cleanup')
                report['handler_after_cleanup'] = remaining['default']
            except Exception as failure:
                cleanup_errors.append(str(failure))
        for link, target in reversed(links) if safe_to_remove else []:
            try:
                remove_link(link, target)
            except Exception as failure:
                cleanup_errors.append(str(failure))
        if temporary and safe_to_remove:
            temporary.cleanup()
        elif temporary:
            # Keep isolation attached if the OS cannot terminate the test app.
            temporary._finalizer.detach()
        report['cleanup_errors'] = cleanup_errors
        report['cleanup'] = ('temporary source, app, data and namespace links removed' if safe_to_remove and not cleanup_errors
                             else 'cleanup incomplete; inspect cleanup_errors and retained namespace paths')
        signal.signal(signal.SIGTERM, previous_term)
        if cleanup_errors:
            error = '; '.join(cleanup_errors)
            report['status'] = 'failed'
        (output / 'report.json').write_text(json.dumps(report, indent=2) + '\n')
        cases = [('packaged scheme registered in LaunchServices', 'registration'),
                 ('packaged external URL cold launches settings', 'cold_launch'),
                 ('packaged external URL wakes the same hidden process', 'warm_wake')]
        suite = ET.Element('testsuite', name='packaged-routing', tests=str(len(cases)), failures='0', errors='0', skipped='0', time=str(time.monotonic() - started))
        for index, (name, event) in enumerate(cases):
            case = ET.SubElement(suite, 'testcase', classname='tests/e2e/packaged/observe.ts', name=name)
            if event not in events or (error and index == 0):
                ET.SubElement(case, 'failure', message=error or f'missing evidence: {event}')
        suite.set('failures', str(len(list(suite.iter('failure')))))
        ET.ElementTree(suite).write(args.junit, encoding='unicode', xml_declaration=True)
    print(f'Packaged routing evidence: {output}', flush=True)
    return 1 if report['status'] != 'passed' else 0


if __name__ == '__main__':
    raise SystemExit(main())
