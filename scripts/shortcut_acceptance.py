#!/usr/bin/env python3
"""Run isolated local/global shortcut journeys in an isolated host with test-only module factories."""
from __future__ import annotations
import argparse
import json
import os
from pathlib import Path
import shutil
import signal
import subprocess
import tempfile
import uuid
from native_acceptance import interrupt, stop

ROOT = Path(__file__).resolve().parents[1]
ENTRY = 'tests/e2e/shortcuts/shortcuts.test.tsx'


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--system-input', action='store_true')
    parser.add_argument('--global-shortcuts', action='store_true')
    parser.add_argument('--target', choices=('headless', 'macos'), required=True)
    parser.add_argument('--junit', type=Path, required=True)
    args = parser.parse_args()
    if args.global_shortcuts and args.target != 'macos':
        parser.error('global native registration acceptance requires macos')
    helper = None
    if args.system_input:
        if not args.global_shortcuts:
            parser.error('--system-input requires --global-shortcuts')
        from shortcut_input import build, check
        helper = build()
        if not check(helper):
            parser.error(f'system input permission required for {helper}; run scripts/shortcut_input.py --request-permission')
    signal.signal(signal.SIGTERM, interrupt)
    report = args.junit.resolve()
    report.parent.mkdir(parents=True, exist_ok=True)
    report.unlink(missing_ok=True)
    folder = 'tests/e2e/global-shortcuts' if args.global_shortcuts else 'tests/e2e/shortcuts'
    host = Path(folder) / 'host'
    config = json.loads((ROOT / folder / 'config.json').read_text())
    with tempfile.TemporaryDirectory(prefix='usher-shortcuts-') as temporary:
        stage = Path(temporary)
        for source in ('ui', 'i18n', folder):
            shutil.copytree(ROOT / source, stage / source, ignore=shutil.ignore_patterns('target', 'Cargo.lock'))
        if args.global_shortcuts:
            config['previewPath'] = str(report.with_suffix('.png'))
            (stage / folder / 'config.json').write_text(json.dumps(config, indent=2) + '\n')
        if helper:
            config.update(inputHelper=str(helper), inputConfig=str(stage / folder / 'config.json'), inputReceipt=str(stage / 'input-receipt.json'))
            (stage / folder / 'config.json').write_text(json.dumps(config, indent=2) + '\n')
        shutil.copy2(ROOT / 'tsconfig.json', stage / 'tsconfig.json')
        manifest = stage / host / 'Cargo.toml'
        manifest.write_text(manifest.read_text().replace('path = "../../../.."', f'path = {json.dumps(str(ROOT))}'))
        app = json.loads((ROOT / 'nsh.json').read_text())
        app['id'] += '.' + config['appIdSuffix'] + '.' + uuid.uuid4().hex
        app['rust'] = str(host)
        app.pop('updater', None)
        (stage / 'nsh.json').write_text(json.dumps(app, indent=2) + '\n')
        (stage / 'App.tsx').write_text(f'/** Isolated shortcut test entry. */\nimport "./{folder}/Harness";\n')
        (stage / '.nsh/rust').mkdir(parents=True)
        lock = ROOT / '.nsh/rust/Cargo.lock'
        if lock.exists():
            shutil.copy2(lock, stage / '.nsh/rust/Cargo.lock')
        env = {**os.environ, 'CARGO_TARGET_DIR': str(ROOT / 'target')}
        entry = ENTRY
        if args.target == 'macos':
            entry = 'tests/e2e/shortcuts/suite.test.ts'
        if args.global_shortcuts:
            entry = folder + '/native.test.tsx'
        suite = ('global-shortcuts-' if args.global_shortcuts else 'shortcuts-') + args.target
        if args.system_input:
            entry = folder + '/input.test.tsx'
            suite = 'global-shortcuts-input-macos'
        process = None
        try:
            process = subprocess.Popen(['nsh', 'test', entry] + ['--junit', str(report)] + (['--target', 'macos'] if args.target == 'macos' else []), cwd=stage, env=env, start_new_session=True)
            return process.wait(timeout=json.loads((ROOT / 'tests/config.json').read_text())['suites'][suite]['runs'][0]['timeout_seconds'])
        finally:
            stop(process)


if __name__ == '__main__':
    raise SystemExit(main())
