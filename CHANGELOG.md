# Changelog

Each release needs a `## <version>` section here: the release workflow refuses a tag
whose version has no section, and publishes that section as the release notes.
Keep work in progress under `Unreleased`.

## Unreleased

- Frame layout module (`ui/layout/`): a full-width title bar with breadcrumbs and a light/dark switch, a fixed rail with update, settings and account buttons, a secondary menu that can be hidden (the content panel grows left with an animation), and a status bar as wide as the window. Sizes and the animation duration are theme metrics (`layout-*`, `motion-layout`).
- Settings sections use grouped cards: a heading above a rounded card whose rows put the name and description on the left and the control on the right.

## 0.0.1-alpha.1

- First alpha: macOS shell with sidebar navigation, themes and AppKit controls.
- Diagnostic logging with a level setting and log export.
- Signed update manifests with a trusted key set and a recovery key.
