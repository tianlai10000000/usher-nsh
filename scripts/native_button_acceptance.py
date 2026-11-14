#!/usr/bin/env python3
"""Run shared button journeys in an isolated host with a real AppKit picker cancel driver."""
from __future__ import annotations

import argparse
import json
import os
from pathlib import Path
import shutil
import signal
import subprocess
import tempfile

from native_acceptance import interrupt, stop

ROOT = Path(__file__).resolve().parents[1]
HOST = Path('tests/e2e/native-buttons/host')
ENTRY = 'tests/e2e/buttons-native.test.tsx'


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--junit', type=Path, required=True)
    parser.add_argument('--filter', help='Run a focused diagnostic; full acceptance omits this option.')
    args = parser.parse_args()
    signal.signal(signal.SIGTERM, interrupt)
    report = args.junit.resolve()
    report.parent.mkdir(parents=True, exist_ok=True)
    report.unlink(missing_ok=True)
    screenshots = report.parent / (report.stem + "-screenshots")
    if screenshots.exists():
        shutil.rmtree(screenshots)
    screenshots.mkdir()
    config = json.loads((ROOT / 'tests/e2e/native/config.json').read_text())
    suite = json.loads((ROOT / 'tests/config.json').read_text())['suites']['e2e-macos']
    run = next(item for item in suite['runs'] if item['id'] == 'native-button-interactions')
    with tempfile.TemporaryDirectory(prefix='usher-native-buttons-') as temporary:
        stage = Path(temporary)
        for folder in ('ui', 'i18n', 'tests/ui', 'tests/e2e/native-buttons'):
            shutil.copytree(ROOT / folder, stage / folder, ignore=shutil.ignore_patterns('target', 'Cargo.lock'))
        (stage / 'tests/e2e/native').mkdir()
        shutil.copy2(ROOT / 'tests/e2e/native/config.json', stage / 'tests/e2e/native/config.json')
        shutil.copy2(ROOT / ENTRY, stage / ENTRY)
        shutil.copy2(ROOT / 'tsconfig.json', stage / 'tsconfig.json')
        shutil.copy2(ROOT / 'App.tsx', stage / 'ProductionApp.tsx')
        manifest = stage / HOST / 'Cargo.toml'
        manifest.write_text(manifest.read_text().replace('path = "../../../.."', f'path = {json.dumps(str(ROOT))}'))
        app = json.loads((ROOT / 'nsh.json').read_text())
        app['id'] = config['appId'] + '.buttons'
        app['rust'] = str(HOST)
        app.pop('updater', None)
        (stage / 'nsh.json').write_text(json.dumps(app, indent=2) + '\n')
        (stage / 'App.tsx').write_text('/** Isolated native button acceptance entry. */\nimport "./tests/ui/fixtures/AsyncButtons";\n')
        (stage / '.nsh/rust').mkdir(parents=True)
        lock = ROOT / '.nsh/rust/Cargo.lock'
        if lock.exists():
            shutil.copy2(lock, stage / '.nsh/rust/Cargo.lock')
        env = {**os.environ, 'CARGO_TARGET_DIR': str(ROOT / 'target')}
        process = None
        try:
            process = subprocess.Popen(
                ['nsh', 'test', ENTRY, '--target', 'macos', '--junit', str(report)] +
                (['--filter', args.filter] if args.filter else []),
                cwd=stage, env=env, start_new_session=True,
            )
            result = process.wait(timeout=run['timeout_seconds'])
        finally:
            stop(process)
            for path in stage.glob("*.png"):
                shutil.copy2(path, screenshots / path.name)
        if result != 0 or args.filter:
            return result
        return subprocess.run(["swift", str(ROOT / "scripts/compare_interaction_frames.swift"), str(screenshots)],
                              cwd=ROOT, check=False).returncode


if __name__ == '__main__':
    raise SystemExit(main())
