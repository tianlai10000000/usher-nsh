#!/usr/bin/env python3
"""Run the production NSH macOS host with isolated files and a real watched-source HMR update."""
from __future__ import annotations
import argparse
import json
import os
from pathlib import Path
import shutil
import signal
import socket
import subprocess
import sys
import tempfile
import time
import xml.etree.ElementTree as ET

ROOT = Path(__file__).resolve().parents[1]
FIXTURE = ROOT / 'tests/e2e/native'


def stop(process: subprocess.Popen | None) -> None:
    """Stop only this run's process group, including watcher children."""
    if process is not None and process.poll() is None:
        try:
            os.killpg(process.pid, signal.SIGTERM)
            process.wait(timeout=5)
        except subprocess.TimeoutExpired:
            os.killpg(process.pid, signal.SIGKILL)
            process.wait()
        except ProcessLookupError:
            pass


def isolated_copy(root: Path) -> Path:
    """Copy sources into a new private tree; never mutate the checkout or reuse user data."""
    stage = root / 'source'
    stage.mkdir()
    shutil.copy2(ROOT / 'tsconfig.json', stage / 'tsconfig.json')
    for directory in ('ui', 'i18n'):
        shutil.copytree(ROOT / directory, stage / directory)
    shutil.copytree(FIXTURE, stage / 'tests/e2e/native')
    config = json.loads((ROOT / 'nsh.json').read_text())
    config['id'] = json.loads((FIXTURE / 'config.json').read_text())['appId']
    config['rust'] = str(ROOT)
    config['capabilities'] = {'windows': True, 'statusBar': True}
    config.pop('updater', None)
    (stage / 'App.tsx').write_text('/** Isolated native acceptance entry. */\nimport \"./tests/e2e/native/App\";\n')
    (stage / 'nsh.json').write_text(json.dumps(config, indent=2) + '\n')
    return stage


def interrupt(_signum, _frame):
    """Let timeout termination unwind temporary directories and owned native hosts."""
    raise KeyboardInterrupt("native acceptance interrupted")


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--junit', type=Path, required=True)
    args = parser.parse_args()
    args.junit.parent.mkdir(parents=True, exist_ok=True)
    args.junit.unlink(missing_ok=True)
    report = args.junit.parent / 'native-isolation.json'
    report.unlink(missing_ok=True)
    config = json.loads((FIXTURE / 'config.json').read_text())
    events: dict = {}
    server = host = None
    error = None
    started = time.monotonic()
    logs = Path(tempfile.mkdtemp(prefix='native-run-', dir=args.junit.parent.resolve()))
    server_log = logs / 'native-dev-server.log'
    host_log = logs / 'native-host.log'
    previous_term = signal.signal(signal.SIGTERM, interrupt)
    try:
        if sys.platform != 'darwin':
            raise RuntimeError('native acceptance requires macOS and an unlocked graphical session')
        binary = ROOT / '.nsh/rust/target/debug/nsh-app-usher-nsh-macos-host'
        with (logs / 'native-root-check.log').open('w') as output:
            subprocess.run(['nsh', 'check'], cwd=ROOT, stdout=output, stderr=subprocess.STDOUT, check=True, timeout=config['timeoutSeconds'])
        # Build the maintained generated host, not a modified framework or a simulated window runner.
        with (logs / 'native-host-build.log').open('w') as output:
            subprocess.run(['cargo', 'build', '--locked', '--manifest-path', str(ROOT / '.nsh/rust/Cargo.toml'), '-p', 'nsh-app-usher-nsh-macos-host'], cwd=ROOT, stdout=output, stderr=subprocess.STDOUT, check=True)
        with tempfile.TemporaryDirectory(prefix='usher-native-', dir=logs.resolve()) as temporary:
            sandbox = Path(temporary).resolve()
            stage = isolated_copy(sandbox)
            stage_env = {**os.environ, 'CARGO_TARGET_DIR': str(ROOT / '.nsh/rust/target')}
            with (logs / 'native-staged-check.log').open('w') as output:
                subprocess.run(['nsh', 'check', str(stage)], cwd=stage, env=stage_env, stdout=output, stderr=subprocess.STDOUT, check=True, timeout=config['timeoutSeconds'])
            data = sandbox / 'data'
            data.mkdir(mode=0o700)
            with socket.socket() as reserved:
                reserved.bind(('127.0.0.1', 0))
                port = reserved.getsockname()[1]
            with server_log.open('w') as server_output, host_log.open('w') as host_output:
                try:
                    server = subprocess.Popen(['nsh', 'dev', str(stage / 'App.tsx'), '--server-only', '--port', str(port), '-v'], cwd=stage, env=stage_env, stdout=server_output, stderr=subprocess.STDOUT, start_new_session=True)
                    # This host owns the entire data and cache roots, before any application code loads.
                    host_command = [str(binary), '--dev-server', f'ws://127.0.0.1:{port}/ws', '--data-dir', str(data), '--background', '--name', 'Usher native state acceptance', '--window-size', f"{config['windowSize']['width']}x{config['windowSize']['height']}"]
                    host = subprocess.Popen(host_command, cwd=stage, stdout=host_output, stderr=subprocess.STDOUT, start_new_session=True)
                    deadline = time.monotonic() + config['timeoutSeconds']
                    changed = False
                    while time.monotonic() < deadline:
                        for line in server_log.read_text(errors='replace').splitlines():
                            if config['marker'] not in line:
                                continue
                            raw = line.split(config['marker'], 1)[1]
                            try:
                                event = json.JSONDecoder().raw_decode(raw)[0]
                            except ValueError:
                                continue
                            events[event['event']] = event['data']
                        if 'bundle failed:' in server_log.read_text(errors='replace'):
                            raise RuntimeError('native bundle failed; inspect dev-server log')
                        if 'failed' in events:
                            raise RuntimeError(events['failed'])
                        if 'before_hmr' in events and not changed:
                            source = stage / 'tests/e2e/native/Probe.tsx'
                            text = source.read_text()
                            if text.count('const generation = 1;') != 1:
                                raise RuntimeError('HMR source mutation anchor is missing or ambiguous')
                            source.write_text(text.replace('const generation = 1;', 'const generation = 2;'))
                            changed = True
                        if 'complete' in events:
                            break
                        if host.poll() is not None or server.poll() is not None:
                            raise RuntimeError('native host or watcher exited before acceptance completed')
                        time.sleep(0.1)
                    else:
                        raise RuntimeError('native acceptance timed out; inspect host and dev-server logs')
                finally:
                    stop(host)
                    stop(server)
            expected = data / config['appId'] / 'development'
            settings = expected / 'settings.toml'
            import tomllib
            saved = tomllib.loads(settings.read_text())
            if saved.get('appearance', {}).get('colorMode') != config['colorMode']:
                raise RuntimeError('persisted test preference is absent from the isolated directory')
            files = []
            for path in data.rglob('*'):
                if path.is_symlink() or not path.resolve().is_relative_to(data):
                    raise RuntimeError('test data escaped its isolated root')
                if path.is_file():
                    files.append(str(path.relative_to(data)))
            report.write_text(json.dumps({'schema_version': 1, 'status': 'passed', 'logs': str(logs), 'host_command': host_command, 'data_root': str(data), 'settings': str(settings), 'files': files, 'events': events, 'cleanup': 'temporary tree removed after run'}, indent=2) + '\n')
    except (OSError, RuntimeError, ValueError, subprocess.SubprocessError, KeyboardInterrupt) as failure:
        error = str(failure)
        report.write_text(json.dumps({'status': 'failed', 'error': error, 'logs': str(logs), 'events': events}, indent=2) + '\n')
    finally:
        stop(host)
        stop(server)
        signal.signal(signal.SIGTERM, previous_term)
    cases = [('native windows isolate sessions and share persisted settings', 'before_hmr'), ('native HMR retains stores and rejects old owners and guards', 'hmr_passed'), ('native close and reopen replaces the window identity', 'complete')]
    suite = ET.Element('testsuite', name='native-state', tests=str(len(cases)), failures=str(1 if error else 0), errors='0', skipped='0', time=str(time.monotonic()-started))
    for index, (name, event) in enumerate(cases):
        case = ET.SubElement(suite, 'testcase', classname='tests/e2e/native/App.tsx', name=name)
        if error and index == 0:
            ET.SubElement(case, 'failure', message=error).text = error
        elif event not in events:
            ET.SubElement(case, 'failure', message=f'missing evidence: {event}')
    suite.set('failures', str(len(list(suite.iter('failure')))))
    ET.ElementTree(suite).write(args.junit, encoding='unicode', xml_declaration=True)
    if error:
        print(error, file=sys.stderr)
        return 1
    print('Native acceptance: 3 passed; isolated data removed; real HMR and window lifecycle verified')
    return 0

if __name__ == '__main__':
    raise SystemExit(main())
