#!/usr/bin/env python3
"""Regenerates ui/theme/presets/nsh-gallery.json from the NSH Gallery UI design tokens.

Usage (from the project root; `-I` keeps the working directory off the import path):

    python3 -I scripts/sync-gallery-theme.py                 # the revision pinned in the map
    python3 -I scripts/sync-gallery-theme.py --revision main # the latest upstream
    python3 -I scripts/sync-gallery-theme.py --source DIR    # a local nsh-gallery-ui checkout

The mapping lives in ui/theme/sources/nsh-gallery.json. This script holds no color: it only
merges the upstream scheme tables the way CSS does, converts pixel lengths to points, appends
the mapped roles and writes the metrics. Each metric names the upstream symbol it came from;
after a sync, check those symbols in the pinned revision, because sizes live in TypeScript
source (look/*.ts) rather than in a token file and so cannot be extracted mechanically. Review the diff, run `cargo test --locked --lib` and commit.
"""
import argparse
import json
import pathlib
import re
import sys
import urllib.request

ROOT = pathlib.Path(__file__).resolve().parent.parent
MAP_FILE = ROOT / "ui/theme/sources/nsh-gallery.json"
OUTPUT = ROOT / "ui/theme/presets/nsh-gallery.json"
PIXELS = re.compile(r"^(\d+)px$")


def read_upstream(mapping, revision, local):
    names = mapping["source"]["files"]
    if local:
        return {key: json.loads((local / name).read_text()) for key, name in names.items()}
    base = f'{mapping["source"]["rawBase"]}/{revision}/{mapping["source"]["directory"]}'
    files = {}
    for key, name in names.items():
        with urllib.request.urlopen(f"{base}/{name}", timeout=30) as response:
            files[key] = json.load(response)
    return files


def resolve_revision(mapping, requested):
    if re.fullmatch(r"[0-9a-f]{40}", requested):
        return requested
    api = "https://api.github.com/repos/longbridge/nsh/commits/" + requested
    with urllib.request.urlopen(api, timeout=30) as response:
        return json.load(response)["sha"]


def scheme_tokens(tokens, scheme):
    """General tokens, then the scheme's own: a dark theme starts from the light values, as in CSS."""
    merged = dict(tokens["cssVars"].get("theme", {}))
    merged.update(tokens["cssVars"]["light"])
    if scheme == "dark":
        merged.update(tokens["cssVars"]["dark"])
    return {name: pixels_to_points(value) for name, value in merged.items()}


def pixels_to_points(value):
    match = PIXELS.match(value)
    return int(match.group(1)) if match else value


def role_value(value, finishes, scheme):
    if value.startswith("@finishes."):
        name, prop = value[len("@finishes."):].split(".", 1)
        return finishes[scheme][name][prop]
    return value


def main():
    parser = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    parser.add_argument("--revision", help="a commit SHA, branch or tag; defaults to the pinned revision")
    parser.add_argument("--source", type=pathlib.Path, help="a local examples/nsh-gallery-ui directory")
    args = parser.parse_args()

    mapping = json.loads(MAP_FILE.read_text())
    revision = mapping["source"]["revision"]
    if args.source:
        revision = args.revision or revision
    else:
        revision = resolve_revision(mapping, args.revision or revision)
    upstream = read_upstream(mapping, revision, args.source)

    css_vars = {}
    for scheme in ("light", "dark"):
        tokens = scheme_tokens(upstream["tokens"], scheme)
        for role, value in mapping["roles"].items():
            tokens[role] = role_value(value, upstream["finishes"], scheme)
        css_vars[scheme] = dict(sorted(tokens.items()))

    theme = {"schemaVersion": 1, **{k: v for k, v in mapping["theme"].items() if v != {}}}
    theme["source"] = {k: mapping["source"][k] for k in ("name", "url", "license")} | {"revision": revision}
    # Sizes do not depend on the color scheme, so they are written once, under `shared`.
    shared = {name: spec[0] for name, spec in mapping.get("metrics", {}).items()}
    touch = {name: spec[0] for name, spec in mapping.get("touch", {}).items()}
    theme["cssVars"] = {"shared": dict(sorted(shared.items())), "touch": dict(sorted(touch.items())), **css_vars}
    OUTPUT.write_text(json.dumps(theme, indent=2, ensure_ascii=False) + "\n")
    print(f"wrote {OUTPUT.relative_to(ROOT)} from {mapping['source']['name']} @ {revision[:10]}")
    return 0


if __name__ == "__main__":
    sys.exit(main())
