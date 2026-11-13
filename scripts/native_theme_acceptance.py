#!/usr/bin/env python3
"""Verify native theme animation in disposable source roots without an application state bridge."""
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
ENTRY = 'tests/e2e/theme-motion-native.test.tsx'


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--junit', type=Path, required=True)
    args = parser.parse_args()
    signal.signal(signal.SIGTERM, interrupt)
    report = args.junit.resolve()
    report.parent.mkdir(parents=True, exist_ok=True)
    report.unlink(missing_ok=True)
    suite = json.loads((ROOT / 'tests/config.json').read_text())['suites']['e2e-macos']
    run = next(item for item in suite['runs'] if item['id'] == 'native-theme-motion')
    with tempfile.TemporaryDirectory(prefix='usher-theme-motion-') as temporary:
        stage = Path(temporary)
        for folder in ('ui', 'tests/ui'):
            shutil.copytree(ROOT / folder, stage / folder)
        (stage / 'tests/e2e').mkdir(parents=True)
        shutil.copy2(ROOT / ENTRY, stage / ENTRY)
        app = json.loads((ROOT / 'nsh.json').read_text())
        app['id'] += '.' + stage.name.replace('_', '-')
        app.pop('rust', None)
        app.pop('updater', None)
        (stage / 'nsh.json').write_text(json.dumps(app, indent=2) + '\n')
        (stage / 'App.tsx').write_text('/** Isolated theme animation fixture. */\nimport "./tests/ui/fixtures/ThemeMotion";\n')
        process = None
        try:
            process = subprocess.Popen(['nsh', 'test', ENTRY, '--target', 'macos', '--junit', str(report)],
                                       cwd=stage, env=os.environ.copy(), start_new_session=True)
            return process.wait(timeout=run['timeout_seconds'])
        finally:
            stop(process)
            screenshots = report.parent / (report.stem + '-screenshots')
            screenshots.mkdir(exist_ok=True)
            for path in stage.glob('*.png'):
                shutil.copy2(path, screenshots / path.name)


if __name__ == '__main__':
    raise SystemExit(main())
