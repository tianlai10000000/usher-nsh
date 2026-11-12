# Usher NSH

[简体中文](README.zh.md) · [Architecture](architecture.md) · [Contributing guidelines](CONTRIBUTING.md)

Usher NSH is a desktop and mobile application foundation built with Longbridge Native Shell (NSH), Rust and TSX. Rust owns shared application data and system services, while TSX provides views and presentation state. The home page currently contains a Hello World example.

## Current capabilities

- Shared Rust settings with local TOML persistence, migrations, recovery and save status.
- Light, Dark and System appearance, configurable accent colors, bundled themes including NSH Gallery, and local JSON themes.
- English and Simplified Chinese interfaces, with a system-language preference.
- Custom desktop window chrome, collapsible sidebar, navigation history, breadcrumbs and an optional bottom status bar.
- AppKit window material on macOS, plus a selectable native settings control family in supported enhanced builds.
- Structured diagnostics with configurable log levels, daily files, retention and archive export.
- A separate Rust crate for update state transitions and release signature validation.

Update UI, network and installation integration, system tray support, and remaining platform acceptance are tracked in the implementation plan. Diagnostic export currently writes to the application's `exports/` directory and reveals that folder. System save dialogs and mobile export integration are pending.

## Supported targets

| Target | Project status |
| --- | --- |
| macOS | Current development platform, with AppKit integration and local packaging verification |
| Windows and Linux | Planned desktop targets, pending project-level build and runtime acceptance |
| Android and iOS | Planned mobile targets, pending simulator, device and packaging acceptance |

Implementation status and acceptance criteria are recorded in [scaffolding.md](scaffolding.md).

## Requirements

Install Rust 1.97 or later (edition 2024), Xcode command-line tools, and NSH CLI v0.5.0. The Rust NSH dependencies are pinned to the same version. NSH release downloads are available from the upstream project. [NSH v0.5.0](https://github.com/longbridge/nsh/releases/tag/v0.5.0)

## Quick start

Run from the project root.

```sh
nsh --version
nsh run --target macos
```

The first run builds the Rust module and macOS development host. Press `Ctrl+C` to stop the development server. The development workflow supports TSX hot reload. Rust and host changes require rebuilding; HMR state and lifecycle acceptance is tracked in the implementation plan.

In **Settings → Appearance**, choose appearance, style theme and control family independently. The macOS development build includes AppKit settings controls by default. They are available on macOS 26 and later when the native components are registered.

Navigation supports back, forward and breadcrumbs. Desktop shortcuts are `⌘+[` and `⌘+]` on macOS, or `Ctrl+[` and `Ctrl+]` on other desktop targets. The window is a full-width title bar (menu toggle, history, breadcrumbs and a light/dark switch), a narrow fixed rail (home, then update, settings and account buttons at the bottom), a secondary menu that can be shown or hidden, the content panel and an optional full-width bottom status bar. Hiding the secondary menu animates the content panel to the left. Narrow windows show the secondary menu as a drawer instead. The update and account buttons stay disabled until those features connect to the contracts in `ui/layout/contracts/`. Menu docking and bottom status bar visibility are saved locally.

The eight title-bar and rail icon buttons use NSH `Tooltip` hints on hover or keyboard focus, including disabled buttons. Labels follow the selected language and current action. Hints prefer the bottom of title-bar buttons and the right of rail buttons; NSH draws them above the window content and flips or shifts them within the safe area, with long labels wrapping to the available width. Breadcrumbs have no tooltip.

Settings uses the framework’s `Tab` / `Tabs` for its wide section list and `Select` in compact layouts. The selected section has a themed fill and edge; its indicator is positioned and animated by NSH. Keyboard focus has a separate ring: arrow keys and Home/End change sections, while Tab enters or leaves the list in one step. The AppKit control family uses native navigation controls.

The macOS window starts at 800 × 600 points and has the same minimum frame size. TSX hot reload preserves the current window size.

## Local data

With the default macOS host and application ID, the data directories are

| Build | Directory |
| --- | --- |
| Development | `~/Library/Application Support/com.tianlai10000000.usher.nsh/development/` |
| Release | `~/Library/Application Support/com.tianlai10000000.usher.nsh/` |

Each directory contains `settings.toml`, a `themes/` directory for local theme files, `backgrounds/` for imported background images, `logs/YYYY-MM-DD/usher.log` for diagnostics, and `exports/` for generated archives. The host supplies the base directory; isolated tests can use a separate data root.

## Style themes

Every theme is one JSON file in the same format, whether it ships with the application or is added locally. The Rust registry in [src/style_themes.rs](src/style_themes.rs) loads them, merges each theme with the theme it `extends`, substitutes `var(--name)` references and sends the interface a complete token table. The interface reads tokens by name, so a new theme needs no code change.

Bundled themes are discovered at build time from [ui/theme/builtin/](ui/theme/builtin/) and [ui/theme/presets/](ui/theme/presets/). Adding a file there adds a theme, and its `order` field places it in the list. [default](ui/theme/builtin/default/theme.json) is the root: it defines every token, so other themes override only what differs.

To add a local theme, open the theme directory from Settings, add `themes/<name>.json` or `themes/<folder>/theme.json`, then select **Rescan theme files**. The settings page also creates a theme or copies an existing one. Keep a theme's ID stable when renaming it so saved selections can be restored.

```json
{
  "schemaVersion": 1,
  "id": "user.warm-frost",
  "name": "Warm Frost",
  "names": { "zh-Hans": "暖灰磨砂" },
  "extends": "default",
  "cssVars": {
    "light": { "sidebar": "#EFEDE9B3", "radius": 12 },
    "dark": { "sidebar": "#242322B3", "radius": 12 }
  }
}
```

| Field | Meaning |
| --- | --- |
| `id` | `user.<name>` or `custom-<name>` for local files. Bundled themes use a plain ID or `builtin.<name>`. |
| `names` | Optional display names by UI locale. `name` is used when a locale has no entry. |
| `extends` | The ID of another theme. Omitted means `default`. Chains stop at `default`, up to eight levels. |
| `material` | `solid` or `glass`. Inherited from the base theme when omitted. |
| `order` | Position in the theme list. Local themes without one follow the bundled themes. |
| `source` | Optional origin of a vendored theme: name, URL, license and revision. |
| `cssVars.shared` | Tokens that do not depend on the color scheme: sizes, spacing, type, radii, focus and motion. |
| `cssVars.light`, `cssVars.dark` | Colors and other scheme-specific tokens. A scheme value wins over `shared`. Up to 192 tokens per scheme. |

A token value is checked by its name family or, outside the families, by its shape. Colors are `#RRGGBB` or `#RRGGBBAA`, a CSS function such as `color-mix(...)` or `rgba(...)`, a shadow list, or a `var(--token)` reference. The metric families hold plain numbers:

| Family | Meaning | Range |
| --- | --- | --- |
| `size-*` | Heights, widths and line widths in points | 0 to 64 |
| `layout-*` | Frame widths, insets, title bar height and the wide-window breakpoint, in points | 0 to 1200 |
| `space-*` | Padding and gaps in points | 0 to 48 |
| `font-*` | Font sizes in points | 0 to 32 |
| `radius`, `radius-*` | Corner radii in points | 0 to 32 |
| `focus-width`, `focus-offset` | Keyboard focus halo width and offset, in points. The offset may be negative to draw the halo inside. | 0 to 8, offset ±8 |
| `motion-*` | Durations in milliseconds | 0 to 1000 |
| `feedback-*` | Ratios: press scale, hover wash strength, disabled opacity | 0 to 1 |

The root theme [default](ui/theme/builtin/default/theme.json) lists every metric. Controls read them through [ui/theme/look.ts](ui/theme/look.ts), so a theme changes a button's height, padding, corner, focus halo, press sink and hover wash by changing numbers. Views contain no sizes or colors of their own. A theme with an unknown base, a reference loop or an invalid value is skipped and reported in Settings; other themes are unaffected.

The bundled themes are [Default](ui/theme/builtin/default/theme.json), [Apple Liquid Glass](ui/theme/builtin/apple-glass/theme.json), [NSH Gallery](ui/theme/presets/nsh-gallery.json), [Primer · GitHub](ui/theme/presets/primer.json), [Geist · Vercel](ui/theme/presets/geist.json), [Slate · shadcn/ui](ui/theme/presets/slate.json) and [Radix Colors · Sand](ui/theme/presets/radix-sand.json).

Default and NSH Gallery use a neutral light-gray window tint over the macOS backdrop in light mode. The NSH Gallery content panel uses its card surface, which is white in light mode. The docked secondary menu draws its edge shadow from `shell-shadow`; NSH Gallery inherits that shadow and the window tint from Default.

Primary rail icons use `size-rail-icon` (24 points by default). The bottom update, Settings and default account icons use `size-rail-utility-icon` (18 points by default), independently of toolbar icons. The default light-mode menu selection fill is `menu-selected: #eaeaeb`.

On macOS, Home and Settings use system SF Symbols: filled while selected and outlined otherwise, consistently across the rail, secondary menu and title-bar Settings button. The update and default account icons also use system symbols. All rail symbols are defined in [navigation-icons.json](ui/config/navigation-icons.json); other hosts retain the existing glyph fallback. The compact Settings section picker uses Select's native highlight state for both pointer hover and keyboard navigation, with the current theme's `item-hover` fill.

In **Settings → General → Window background**, choose an image with the system image picker, remove it, or adjust its opacity from 0% (transparent) to 100% (opaque); the initial opacity is 30%. The image covers the window between the frosted background and all interface elements. Dragging the slider previews changes immediately and releasing it saves the value. Images are copied locally before their references are saved, so moving the original does not affect the background. Settings store only image metadata and opacity. The picker scales images to a maximum side of 2560 pixels; imports are limited to 16 MiB and 16,777,216 pixels. Limits and defaults are shared by Rust and TSX through [background.json](ui/config/background.json). Missing or unreadable images fall back to the themed backdrop and can be replaced or removed in Settings. Hosts without a data directory keep the selection for the current session. Background composition is managed by [WindowBackdrop.tsx](ui/layout/components/WindowBackdrop.tsx).

**Content background transparency** in the same section controls the secondary menu (including its drawer), right content panel and cards across all Settings sections together. It defaults to 0%, preserving each theme’s fill; 100% removes those fills so the custom background image shows through. Text and controls retain their appearance. The value previews while dragging and is saved on release, independently of image opacity.

### NSH Gallery theme

[nsh-gallery.json](ui/theme/presets/nsh-gallery.json) is generated from the design tokens of the NSH Gallery UI ([examples/nsh-gallery-ui](https://github.com/longbridge/nsh/tree/main/examples/nsh-gallery-ui), Apache-2.0; see [NOTICE](NOTICE)). That layer is an internal part of the NSH repository, so the theme is a snapshot pinned to a commit rather than a dependency. The mapping from upstream tokens to this application's roles is [ui/theme/sources/nsh-gallery.json](ui/theme/sources/nsh-gallery.json). To follow upstream changes, from the project root:

```sh
python3 -I scripts/sync-gallery-theme.py --revision main
cargo test --locked --lib
```

The script rewrites the theme file and the pinned revision, and nothing else. Review the diff before committing. The theme carries the Gallery colors, radii and shadows, and the Gallery's control metrics: 28 point buttons, 32 point fields, 7 point control corners, a 3 point focus halo, a 97% press sink and a 5% hover wash. Each metric in the mapping names the upstream file and symbol it was read from. Sizes live in TypeScript source (`look/*.ts`) rather than a token file, so after a sync check those symbols in the new revision. The Gallery's own component structure is not reproduced: its tab strips, split buttons, menus and sheets are not part of this application.

[Slate](ui/theme/presets/slate.json), [Primer](ui/theme/presets/primer.json) and [Radix Colors · Sand](ui/theme/presets/radix-sand.json) state the sizes of shadcn/ui, Primer and Radix Themes in the same way. [Geist](ui/theme/presets/geist.json) states none, because Vercel does not publish its component dimensions, so it uses the root theme's sizes with its own colors.

## Diagnostics

The Diagnostics section in Settings supports `ERROR`, `WARN`, `INFO`, `DEBUG`, `TRACE` and `OFF`. The default is `INFO`; changing the level applies immediately and is saved for the next launch.

Logs are written by a background worker into local-date directories, split at 8 MiB and retained for 14 days subject to a 256 MiB disk quota. Export supports today, the last 7 days or the last 14 days. Archives use `.tar.gz`, skip symbolic links and limit each source file to 8 MiB and total source data to 64 MiB. The export directory retains the five newest archives. Platform acceptance gaps are listed in [batch 05](scaffolding.md#批次-05--诊断日志--部分完成).

## Package for macOS

```sh
sh scripts/package-macos.sh standard
sh scripts/package-macos.sh appkit
```

| Variant | Contents | Output |
| --- | --- | --- |
| `standard` | AppKit window material, shared themes and NSH controls | `.nsh/out/macos-standard/` |
| `appkit` | Standard capabilities plus selectable AppKit settings controls on supported systems | `.nsh/out/macos-appkit/` |

Omitting the script argument selects `standard`. Both variants share the application ID and release settings directory. The script produces local ad hoc signed artifacts; distribution signing and notarization remain release work.

Release optimization is defined in [.cargo/config.toml](.cargo/config.toml). Running `nsh package --target macos` directly from the project root also uses that configuration and writes to `.nsh/out/` by default. Verify the configuration on each remaining target as it is brought up.

## Verify

Run the relevant checks from the project root. The workspace command includes the independent update crate.

```sh
cargo test --locked --workspace
nsh check
nsh test tests/ui/appearance.test.tsx
nsh test tests/ui/style-themes.test.tsx
nsh test tests/ui/background.test.tsx
nsh test tests/ui/background-layout.test.tsx
nsh test tests/ui/window-shell.test.tsx
nsh test tests/ui/tooltips.test.tsx
nsh test tests/ui/diagnostics.test.tsx
```

For changes confined to update logic, use

```sh
cargo test --locked -p usher-update
```

Native macOS checks open real windows and require a graphical session.

```sh
nsh test tests/ui/window-shell.test.tsx --target macos
nsh test tests/ui/tooltips.test.tsx --target macos
nsh test tests/ui/background-layout.test.tsx --target macos
nsh test tests/ui/appkit.test.tsx --target macos
USHER_MACOS_APPKIT=0 nsh test tests/ui/appkit.test.tsx --target macos
```

Headless UI checks use session settings and cannot validate disk persistence or native material. See the implementation plan for remaining integration, benchmark and performance work.

## Repository guide

| Location | Responsibility |
| --- | --- |
| `App.tsx` | Application mount and shared view composition |
| `src/` | Rust settings, theme files, diagnostics and NSH bridge |
| `crates/usher-update/` | Update state machine and release signature logic |
| `ui/layout/Frame.tsx` | Window frame composition entry |
| `ui/layout/components/` | Title bar, menus, content panel, status bar and background layers |
| `ui/layout/controllers/`, `ui/layout/contracts/` | Menu and window control logic, and interfaces consumed by the frame |
| `ui/navigation/` | Routes and navigation history |
| `ui/features/`, `ui/state/`, `ui/components/` | Feature views, presentation state and reusable controls |
| `ui/theme/` | Bundled theme files, the Gallery mapping and token projection |
| `i18n/messages/`, `ui/i18n/` | Translation catalogs and UI localization |
| `platforms/macos/` | AppKit window material, controls and symbols |
| `tests/ui/` | NSH UI checks |
| `scripts/` | Packaging and Gallery theme sync commands |
| `.nsh/`, `target/` | Generated build and test output |

The [architecture](architecture.md) describes module responsibilities, dependency boundaries, state ownership and data flow. The [batch plan](scaffolding.md) records implementation status, and the [contributing guidelines](CONTRIBUTING.md) define development and contribution requirements.

## License

The Cargo packages declare `LicenseRef-Proprietary` and are marked `publish = false`. Third-party dependencies retain their respective licenses.
