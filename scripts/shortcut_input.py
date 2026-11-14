#!/usr/bin/env python3
"""Build and check a stable macOS input helper; permission is requested only with an explicit flag."""
from __future__ import annotations
import argparse
import hashlib
import json
import plistlib
import subprocess
import tempfile
import time
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
CONFIG = ROOT / 'tests/e2e/global-shortcuts/config.json'
SOURCE = ROOT / 'tests/e2e/global-shortcuts/input/main.swift'


def build() -> Path:
    """Keep the signed helper unchanged when its source/identity are unchanged, preserving TCC identity."""
    identity = json.loads(CONFIG.read_text())['input']['helper']
    bundle = ROOT / 'target/test-tools' / (identity['name'] + '.app')
    executable = bundle / 'Contents/MacOS' / identity['executable']
    digest = hashlib.sha256(SOURCE.read_bytes() + json.dumps(identity, sort_keys=True).encode()).hexdigest()
    stamp = bundle.parent / 'shortcut-input.sha256'
    if executable.exists() and stamp.exists() and stamp.read_text() == digest:
        return bundle
    executable.parent.mkdir(parents=True, exist_ok=True)
    plist = {'CFBundleIdentifier': identity['identifier'], 'CFBundleName': identity['name'],
             'CFBundleDisplayName': identity['name'], 'CFBundleExecutable': identity['executable'],
             'CFBundleVersion': identity['version'], 'CFBundlePackageType': 'APPL', 'LSUIElement': True}
    (bundle / 'Contents/Info.plist').write_bytes(plistlib.dumps(plist))
    subprocess.run(['swiftc', '-module-cache-path', str(ROOT / 'target/swift-module-cache'), str(SOURCE), '-o', str(executable), '-framework', 'AppKit', '-framework', 'CoreGraphics'], check=True)
    subprocess.run(['codesign', '--force', '--sign', '-', '--identifier', identity['identifier'], str(bundle)], check=True)
    stamp.write_text(digest)
    return bundle


def check(bundle: Path, request: bool = False) -> bool:
    """Launch by bundle identity so macOS grants input access to the stable test tool."""
    with tempfile.TemporaryDirectory(prefix='usher-input-check-') as temporary:
        receipt = Path(temporary) / 'permission.json'
        subprocess.run(['open', '-g', '-n', str(bundle), '--args',
                        '--request' if request else '--check', str(receipt)], check=True)
        config = json.loads(CONFIG.read_text())['input']
        deadline = time.monotonic() + config['timeoutMs'] / 1000
        while not receipt.exists() and time.monotonic() < deadline:
            time.sleep(config['pollMs'] / 1000)
        if not receipt.exists():
            raise RuntimeError('input helper did not return a permission receipt')
        return json.loads(receipt.read_text()).get('permitted') is True


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--request-permission', action='store_true')
    args = parser.parse_args()
    bundle = build()
    allowed = check(bundle, args.request_permission)
    print(json.dumps({'helper': str(bundle), 'permitted': allowed}, ensure_ascii=False))
    return 0 if allowed else 2


if __name__ == '__main__':
    raise SystemExit(main())
