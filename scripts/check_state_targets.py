#!/usr/bin/env python3
"""Verify Cargo JSON artifact evidence for every configured library/target combination."""
import json
from pathlib import Path
import subprocess
import sys
import tomllib

ROOT = Path(__file__).resolve().parents[1]


def verified_artifacts(messages: list[dict], packages: list[str], targets: list[str]) -> set[tuple[str, str]]:
    """Reject success summaries without real library artifacts for the complete matrix."""
    finished = [message for message in messages if message.get('reason') == 'build-finished']
    if not finished or not all(message.get('success') is True for message in finished):
        raise ValueError('Cargo did not finish successfully')
    found = set()
    for message in messages:
        if message.get('reason') != 'compiler-artifact' or 'lib' not in message.get('target', {}).get('kind', []):
            continue
        manifest = Path(message.get('manifest_path', ''))
        if not manifest.is_file():
            continue
        package = tomllib.loads(manifest.read_text())['package']['name']
        if package not in packages:
            continue
        for filename in message.get('filenames', []):
            path = Path(filename)
            for target in targets:
                if target in path.parts and path.is_file():
                    found.add((package, target))
    required = {(package, target) for package in packages for target in targets}
    if not required or found != required:
        raise ValueError(f'missing compiled artifacts: {sorted(required - found)}')
    return found


def main() -> int:
    config = json.loads((ROOT / 'tests/config.json').read_text())['platform_compile']
    packages, targets = config['packages'], config['targets']
    for items in (packages, targets):
        if not items or len(items) != len(set(items)) or any(not isinstance(item, str) or not item for item in items):
            raise ValueError('platform matrix must contain unique nonempty names')
    command = ['cargo', 'check', '--locked', '--message-format=json']
    for package in packages:
        command.extend(['-p', package])
    command.extend(['--features', ','.join(config['features'])])
    for target in targets:
        command.extend(['--target', target])
    print('Command: ' + ' '.join(command), flush=True)
    result = subprocess.run(command, cwd=ROOT, stdout=subprocess.PIPE, stderr=subprocess.STDOUT, text=True)
    reports = ROOT / 'target/test-reports'
    reports.mkdir(parents=True, exist_ok=True)
    (reports / 'state-target-artifacts.jsonl').write_text(result.stdout)
    if result.returncode:
        print(result.stdout, file=sys.stderr)
        return result.returncode
    messages = []
    for line in result.stdout.splitlines():
        try:
            message = json.loads(line)
            if isinstance(message, dict):
                messages.append(message)
        except ValueError:
            pass
    artifacts = verified_artifacts(messages, packages, targets)
    for package, target in sorted(artifacts):
        print(f'verified {package} {target}')
    print('state platform artifacts verified')
    return 0


if __name__ == '__main__':
    try:
        raise SystemExit(main())
    except (OSError, ValueError, KeyError) as error:
        print(error, file=sys.stderr)
        raise SystemExit(1)
