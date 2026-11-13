#!/usr/bin/env python3
"""Measure two isolated macOS release builds with the same profile; never launch or install them."""
from __future__ import annotations

import argparse
import hashlib
import io
import json
import os
from pathlib import Path
import shutil
import subprocess
import tarfile
import tempfile
import time

ROOT = Path(__file__).resolve().parents[1]


def git(*args: str) -> bytes:
    return subprocess.check_output(['git', *args], cwd=ROOT)


def measure(source: Path, label: str, output: Path) -> dict:
    inputs = {}
    for path in sorted(source.rglob('*')):
        if path.is_file():
            inputs[str(path.relative_to(source))] = hashlib.sha256(path.read_bytes()).hexdigest()
    (output / f'{label}-inputs.json').write_text(json.dumps(inputs, indent=2) + '\n')
    env = {**os.environ, 'USHER_MACOS_APPKIT': '0', 'CARGO_INCREMENTAL': '0',
           'CARGO_TARGET_DIR': str(source / 'build-target')}
    command = ['nsh', 'package', '--target', 'macos', '--arch', 'arm64', '--out', str(source / 'out')]
    started = time.monotonic()
    with (output / f'{label}-package.log').open('w') as log:
        subprocess.run(command, cwd=source, env=env, stdout=log, stderr=subprocess.STDOUT, check=True)
    seconds = time.monotonic() - started
    apps = list((source / 'out').glob('*.app'))
    if len(apps) != 1:
        raise RuntimeError(f'expected one app, found {apps}')
    app = apps[0]
    files = [path for path in app.rglob('*') if path.is_file() and not path.is_symlink()]
    executables = [path for path in (app / 'Contents/MacOS').iterdir() if path.is_file()]
    result = {'seconds': seconds, 'app_bytes': sum(path.stat().st_size for path in files),
              'executables': {path.name: path.stat().st_size for path in executables},
              'profile': (source / '.cargo/config.toml').read_text(), 'command': command,
              'source_digest': hashlib.sha256(json.dumps(inputs, sort_keys=True).encode()).hexdigest()}
    # Preserve a per-build result even if the second build fails.
    (output / f'{label}-cost.json').write_text(json.dumps(result, indent=2) + '\n')
    return result


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--baseline', required=True, help='reviewed Git revision to compare')
    args = parser.parse_args()
    revision = git('rev-parse', '--verify', args.baseline + '^{commit}').decode().strip()
    reports = ROOT / 'target/test-reports'
    reports.mkdir(parents=True, exist_ok=True)
    output = Path(tempfile.mkdtemp(prefix='state-cost-', dir=reports))
    print(f'Cost evidence: {output}', flush=True)
    result = {'baseline_revision': revision, 'conditions': 'sequential single cold-target builds; cached registry; standard arm64; no launch', 'builds': {}}
    try:
        with tempfile.TemporaryDirectory(prefix='sources-', dir=output) as temporary:
            source_root = Path(temporary)
            baseline = source_root / 'baseline'
            baseline.mkdir()
            with tarfile.open(fileobj=io.BytesIO(git('archive', revision))) as archive:
                archive.extractall(baseline, filter='data')
            current = source_root / 'current'
            current.mkdir()
            # Git's file inventory excludes build caches and private ignored files.
            for relative in set(git('ls-files', '-z', '-c', '-o', '--exclude-standard').decode().split('\0')):
                path = ROOT / relative
                if relative and path.is_file():
                    destination = current / relative
                    destination.parent.mkdir(parents=True, exist_ok=True)
                    shutil.copy2(path, destination)
            for label, source in [('baseline', baseline), ('current', current)]:
                print(f'Building {label}', flush=True)
                result['builds'][label] = measure(source, label, output)
                shutil.rmtree(source)
        result['status'] = 'passed'
    except (OSError, RuntimeError, subprocess.SubprocessError) as error:
        result['status'] = 'failed'
        result['error'] = str(error)
    (output / 'comparison.json').write_text(json.dumps(result, indent=2) + '\n')
    print(json.dumps(result, indent=2), flush=True)
    return 0 if result['status'] == 'passed' else 1


if __name__ == '__main__':
    raise SystemExit(main())
