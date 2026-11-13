# Changelog

Each release needs a `## <version>` section here: the release workflow refuses a tag
whose version has no section, and publishes that section as the release notes.
Keep work in progress under `Unreleased`.

## Unreleased

## 0.1.0

First release, installed by hand. Subsequent releases are discovered through signed GitHub update manifests; compatible code packages and whole-app upgrades use the application's Updates controls.

### Added

- Window frame: a full-width title bar with menu toggle, back and forward history, breadcrumbs and a light/dark switch; a fixed rail with home, update, settings and account buttons; a secondary menu that docks beside the content on wide windows and becomes a drawer on narrow ones; and an optional status bar. Docking and status bar visibility are saved.
- Settings with General, Theme, Updates and Diagnostics sections, laid out as grouped cards. Wide windows list the sections on the left; narrow windows use a drop-down.
- Appearance: Light, Dark or System mode, a configurable accent color, and a choice of style theme. Bundled themes are Default, Apple Liquid Glass, NSH Gallery, Primer · GitHub, Geist · Vercel, Radix Colors · Sand and Slate · shadcn/ui. You can add your own themes as local JSON files.
- A window background image with adjustable opacity, and adjustable content-area transparency.
- English and Simplified Chinese, with an option to follow the system language.
- Tooltips on the title-bar and rail icon buttons, including disabled ones.
- macOS: native window material, and AppKit settings controls on macOS 26 and later (the Standard build uses NSH controls on every supported macOS).
- Diagnostics: a log level from `OFF` to `TRACE` (default `INFO`) that applies immediately, daily log files kept for 14 days, and an export of the last day, 7 days or 14 days as a `.tar.gz` in the app's `exports/` folder.
- Preferences are saved to a local file with schema migration. A damaged file is backed up, and a file written by a newer version is left untouched.
- Downloads: a `standard` and an `appkit` build for macOS on Apple Silicon, as `.zip` files.

### Known limitations

- macOS on Apple Silicon only. There is no Intel Mac, Windows, Linux, Android or iOS build.
- The home page is still a Hello World example.
- Packages use ad hoc macOS signatures without Developer ID notarization. Gatekeeper may prevent first launch. Whole-app replacement interrupted between renames requires external recovery, and the current installer does not independently reject a mislabeled executable architecture.
- Diagnostic export has no system save dialog: the archive goes to the `exports/` folder, which the app then reveals.

## 0.0.1-alpha.1

- First alpha: macOS shell with sidebar navigation, themes and AppKit controls.
- Diagnostic logging with a level setting and log export.
- Signed update manifests with a trusted key set and a recovery key.
