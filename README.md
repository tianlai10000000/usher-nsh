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
- Updates settings, automatic checks, code-package downloads, whole-app confirmation, progress and restart controls; Rust owns the shared state and NSH verifies signatures.

Local whole-app replacement and the public 0.1.1 update from a 0.1.0 standard app are verified on macOS arm64. Remote code-package acceptance, system tray support, and remaining platform acceptance are tracked in the implementation plan. Diagnostic export currently writes to the application's `exports/` directory and reveals that folder. System save dialogs and mobile export integration are pending.

## Supported targets

| Target | Project status |
| --- | --- |
| macOS | Current development platform, with AppKit integration and local packaging verification |
| Windows and Linux | Planned desktop targets, pending project-level build and runtime acceptance |
| Android and iOS | Planned mobile targets, pending simulator, device and packaging acceptance |

Implementation status and acceptance criteria are recorded in [iteration backlog](docs/todo.md).

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

Buttons for tasks that require waiting, such as file operations, theme scanning and saving, diagnostic export and updates, show an internal loading icon and remain disabled without changing size. Immediate theme, color mode, language and navigation changes do not show loading icons. Icon-only buttons replace their icon while loading; the leftmost rail keeps its existing interaction.

Navigation supports back, forward and breadcrumbs. Desktop shortcuts are `⌘+[` and `⌘+]` on macOS, or `Ctrl+[` and `Ctrl+]` on other desktop targets. The window is a full-width title bar (menu toggle, history, breadcrumbs and a light/dark switch), a narrow fixed rail (home, then update, settings and account buttons at the bottom), a secondary menu that can be shown or hidden, the content panel and an optional full-width bottom status bar. Hiding the secondary menu animates the content panel to the left. Narrow windows show the secondary menu as a drawer instead. The update button shows availability, progress and restart readiness; it is disabled when updates are unsupported (including development runs) or a visible download is in progress. The account button remains disabled. Menu docking and bottom status bar visibility are saved locally.

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

Bundled themes are discovered at build time from [ui/theme/builtin/](ui/theme/builtin) and [ui/theme/presets/](ui/theme/presets). Adding a file there adds a theme, and its `order` field places it in the list. [default](ui/theme/builtin/default/theme.json) is the root: it defines every token, so other themes override only what differs.

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

See the [NSH Gallery component audit](docs/todo.md#evidence-theme-audit-0) for coverage and remaining differences. Switches, sliders, progress, selection indicators and popups read distinct component finishes; custom accents replace fixed accent gradients. Native switch springs, slider hover feedback, progress sweeps and popup transitions respect reduced motion. Select dismissal retains the current framework behavior; see the audit for remaining boundaries.


The script rewrites the theme file and the pinned revision, and nothing else. Review the diff before committing. The theme carries the Gallery colors, radii and shadows, and the Gallery's control metrics: 28 point buttons, 32 point fields, 7 point control corners, a 3 point focus halo, a 97% press sink and a 5% hover wash. Each metric in the mapping names the upstream file and symbol it was read from. Sizes live in TypeScript source (`look/*.ts`) rather than a token file, so after a sync check those symbols in the new revision. The Gallery's own component structure is not reproduced: its tab strips, split buttons, menus and sheets are not part of this application.

[Slate](ui/theme/presets/slate.json), [Primer](ui/theme/presets/primer.json) and [Radix Colors · Sand](ui/theme/presets/radix-sand.json) state the sizes of shadcn/ui, Primer and Radix Themes in the same way. [Geist](ui/theme/presets/geist.json) states none, because Vercel does not publish its component dimensions, so it uses the root theme's sizes with its own colors.

## Diagnostics

Settings lists Updates before Diagnostics. Diagnostics groups output switches, detail level, additional diagnostics and archive export. Application file logs are enabled by default; console and structured diagnostics are separately switchable and off by default. Terminal details and memory diagnostics are disabled and labelled “Not connected yet”.

The shared level supports `ERROR`, `WARN`, `INFO`, `DEBUG`, `TRACE` and `OFF`, defaulting to `INFO`. Level and output preferences apply immediately and persist across launches; OFF preserves the output preferences. Structured diagnostics is a separate event stream, independent of the TRACE level, currently covering diagnostic configuration and update transitions.

A background worker writes JSONL to local-date directories: application events in `usher.log`, structured events in `usher_tracing.log`. Files split at 8 MiB and share 14-day / 256 MiB retention cleanup; the current day is not deleted to enforce the quota. Credential values are masked and the home directory is shortened to `~`. Exports cover today, the last 7 days or the last 14 days, including historical logs from disabled outputs. Archives use `.tar.gz`, skip symbolic links and limit each source file to 8 MiB and total source data to 64 MiB. The export directory retains the five newest archives. The independent logging facade, backend contracts and test matrix are documented in [the logging design](docs/log.md); remaining platform acceptance is listed in [batch 05](docs/todo.md#logging).

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

Run the relevant checks from the project root. The workspace command includes the update, state, router and test-support crates. Application dependencies enable the watch and matchit adapters; dedicated suites also test the cores without those features.

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

For test helpers, state and routing cores, application integration and headless journeys, run:

```sh
python3 scripts/test.py test-support
python3 scripts/test.py logging
python3 scripts/test.py logging-mock
python3 scripts/test.py state
python3 scripts/test.py router
python3 scripts/test.py routes-check
python3 scripts/test.py update-state
python3 scripts/test.py state-targets
python3 scripts/test.py integration
python3 scripts/test.py state-ui
python3 scripts/test.py router-ui
python3 scripts/test.py e2e --target headless
python3 scripts/test.py ui
```

The `test-support`, `logging`, `logging-mock`, `state` and `router` suites require no NSH. Logging unit tests stay beside the rules; public contracts capture every kind and level through injected backends and compare the file adapter with the capture backend. State and router suites run the same consumer contracts with reference and production backends; router properties use fixed fixture seeds. `integration` runs 11 settings, 13 window/router, 17 update and 3 diagnostics tests against real services and isolated data. `state-ui` covers TS projections and real subscription bridges; `router-ui` covers the router facade and real page lifecycle. `e2e --target headless` runs four journeys through the actual App and Rust services using isolated memory sessions. The macOS native acceptance command also verifies same-VM windows sharing persisted preferences, isolated window sessions, actual source HMR, and close/reopen identity.

Window state is committed by `WindowService`, with page session rules in `src/window_session.rs`. Rust services retain typed errors; the bridge emits stable codes. Generated route contracts include literal metadata, checked against page and message types. The legacy session bridge remains compatible.

Reports are written to `target/test-reports/`; failed, empty, skipped or incomplete required test groups fail the command. NSH suites recursively discover source files and validate fresh JUnit reports. Run NSH commands sequentially within one checkout, and run `nsh check` separately for type checking. `routes-check` verifies generation tests and the actual catalog's generated contract without modifying files; the latter is reported as a check, not an additional test. After changing `ui/pages/routes.json`, regenerate and verify:

```sh
cargo run --locked -p usher-router --features matcher-matchit --bin usher-routes -- generate ui/pages/routes.json ui/pages/routes.generated.ts
python3 scripts/test.py routes-check
nsh check
```

`state-targets` compiles seven configured Rust targets; install their rust-std components first. CI reads and installs the targets from tests/config.json. This checks pure libraries, not native hosts on those platforms.

Native macOS checks open real windows and require an unlocked graphical session. The state acceptance command stages a private source copy and starts the official host with an explicit temporary data/cache root. It modifies only that copy to trigger HMR, verifies the saved file, and removes the temporary tree afterward:

```sh
python3 scripts/test.py e2e --target macos
```

This command runs three lifecycle probes, one native back/guard test diagnostic layout/control and pending-save rendering checks, and seven button lifecycle journeys using production state/router adapters and RouterView. The button suite stages a test-only host extension that cancels a real AppKit image picker and verifies closure, state recovery and another successful cancellation; it does not ship in the application. It does not cover process restart or every native input path. Reports include `e2e-macos.json`, `native-state.xml`, `native-isolation.json`, and per-run logs. CI exposes it through the manual `native_state` option on a self-hosted macOS runner labelled `usher-native`; that runner must already be configured with an unlocked desktop. Remote execution remains unverified.

To run only the isolated native button journeys:

```sh
python3 scripts/native_button_acceptance.py --junit target/test-reports/native-buttons.xml
```

Packaged deep-link acceptance uses an ad-hoc signed release app with a unique test ID. Its data/cache namespaces point to temporary directories; the product entry receives only a read-only route observer in the isolated copy. The command verifies LaunchServices registration, external URL cold launch and same-process wake, then unregisters the app and removes its isolated data:

```sh
python3 scripts/test.py packaged-routing
```

This also runs under the manual `native_state` CI option. It does not verify Developer ID notarization or Gatekeeper's downloaded-app flow.

The macOS VM reload check uses real Cmd+R to replace the VM, restores persisted preferences, resets window history/session data and rejects obsolete callbacks. It uses the official runner’s isolated data/cache roots and is separate from HMR and process restart.

```sh
python3 scripts/test.py native-reload
```

Local update launch simulations use a unique test application ID and separate temporary data/cache roots for each scenario, without changing HOME. Twelve scenarios cover checking, signed package download and next-launch promotion, interrupted downloads, trust failures, and startup rollback. Run on an unlocked Apple Silicon macOS host:

```sh
scripts/update-sim/sim.sh run --all
```

Logs and cleanup evidence are retained under `target/test-reports/update-sim-*/`. `serve <scenario>` owns the isolated app until Ctrl+C, then stops it and cleans up. This covers controller-driven checks, signed package download and next-launch promotion, plus preloaded package recovery. Real whole-app replacement is covered by the separate `shell-update` command below. See the [update simulation design](docs/update.md#12-本地更新模拟d14).

Whole-app archive acceptance builds real old/new standard apps and an old AppKit app and drives production check / download / install actions against a signed loopback source. It checks file replacement, NSH-initiated restart, settings retention, corrupt archives, inner identity/signature rejection and installation permission recovery:

```sh
python3 scripts/test.py shell-update
```

Run serially on an unlocked Apple Silicon Mac running macOS 26 or later (required by the AppKit case). Every run uses a unique app ID, temporary source/app/data roots and disposable signing keys. Reports remain under `target/test-reports/shell-update-*/`; a failed cleanup retains the isolation tree and fails the suite. The command is included in the manual `native_state` CI job.

The suite also covers AppKit-to-standard conversion, a uniquely owned temporary installation under `/Applications`, read-only installation parents, and local boundary probes for interrupted replacement, mislabeled architecture and synthetic quarantine. Boundary probes record known limitations separately in each case report; a passing probe does not mean automatic crash recovery or Gatekeeper approval. Developer ID notarization, actual browser distribution and public releases remain outside this command. Known NSH 0.5.0 limits: interruption between installation renames requires external recovery, and a validly signed archive can contain a mismatched executable architecture. Synthetic quarantine assessment rejects the ad hoc bundle. See the update design for remaining acceptance work.

Existing native UI checks:

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
| `crates/usher-update/` | Pure code-package / whole-app update policy |
| `crates/usher-logging/` | Independent logging, redaction, retention and export |
| `crates/usher-state/`, `crates/usher-router/` | Replaceable state backend and independent routing rules |
| `crates/usher-test-support/` | Generic development-only test helpers |
| `tests/integration/`, `tests/e2e/` | Application contracts and real headless journeys |
| `ui/layout/Frame.tsx` | Window frame composition entry |
| `ui/layout/components/` | Title bar, menus, content panel, status bar and background layers |
| `ui/layout/controllers/`, `ui/layout/contracts/` | Menu and window control logic, and interfaces consumed by the frame |
| `ui/router/`, `ui/pages/` | Router facade, RouterView, route catalog and page registration |
| `ui/features/`, `ui/state/`, `ui/components/` | Feature views, presentation state and reusable controls |
| `ui/theme/` | Bundled theme files, the Gallery mapping and token projection |
| `i18n/messages/`, `ui/i18n/` | Translation catalogs and UI localization |
| `platforms/macos/` | AppKit window material, controls and symbols |
| `tests/ui/` | NSH UI checks |
| `scripts/` | Packaging and Gallery theme sync commands |
| `.nsh/`, `target/` | Generated build and test output |

The [architecture](architecture.md) describes module responsibilities, dependency boundaries, state ownership and data flow. The [batch plan](docs/todo.md) records implementation status, the [contributing guidelines](CONTRIBUTING.md) define development and contribution requirements, and the [interaction design guidelines](docs/interaction-design.md) define layout, button loading states, themes and interface copy.

The [shared state design](docs/state-management.md) documents SM1–SM3 and the SM4 migration. Settings retain the existing bridge and persistence formats. Shared navigation, sections, feedback, previews, exports and counters now use Rust stores; diagnostic health has its own live subscription. Headless regressions and isolated macOS multi-window/HMR acceptance pass. SM4 is complete for the supported macOS development host; other platforms and release acceptance remain separate work.

The [routing design](docs/routing.md) records RT1–RT3 and current RT4 progress: standalone `usher-router`, replaceable matchit matching, typed generation from `ui/pages/routes.json`, central page registration and `ui/router`/RouterView. History is committed through `usher-state`; the former writable `ui/navigation` implementation is removed. RT4 adds `usher://app/…` deep links, native back through asynchronous guards, and catalog replacement/HMR preserving history and session data. Change the identity and regenerate types when changing routes.json content. The macOS development host and the local arm64 ad-hoc release package are verified for OS URL registration, external cold launch and same-process wake. Other platform hosts and distribution/notarization remain unverified. Interactive iOS swipe-back is not implemented.

The [testing design](docs/testing.md) records upstream GitHub references, fixtures and actual results. UT1 is complete; integration, bridge and headless E2E coverage are implemented. Generic Rust helpers live in `usher-test-support` as development dependencies; business fixtures stay with their modules. Unit and crate contracts stay beside their owners; application integration and E2E are centralized. CI configuration includes the new commands, but remote GitHub runs have not been verified.

Updates now use A initialization: SettingsService owns an initially unconfigured service; Configure supplies versions once. Shared views do not close it. Rust UpdateState exposes snapshots, commands and single-executor claims; `[updates].auto_check` defaults to enabled and is omitted at its default. The controller and Updates settings section are connected: the first automatic check runs after 30 seconds, then every 6 hours; compatible code packages download quietly, whole-app downloads require confirmation, and restarts are always explicit. The rail indicator and popover share progress, retry and ready states. The rail entry is hidden until a check starts or an update is available; checking uses a neutral spinning ring without a center icon. Up-to-date results and background check failures hide it again. Manual check failures retain a warning entry and the reason and retry action in Settings or an already-open popover; closing the popover keeps the entry. Automatic package downloads also show a progress ring without opening the popover. Ready views use a compact system download icon, wrapped descriptions and explicit restart actions. The popover closes through a themed × icon button in its top-right corner. There is no executor handover; destroying its VM requires an application restart. `usher-logging` provides independent logging with an application health observer. `update-state` and real bridge tests use isolated data and controlled native I/O. Local update simulation separately validates signed package downloads against a loopback source; a separate real-archive suite verifies local whole-app replacement, restart, settings retention and failure recovery. The public 0.1.1 release and standard whole-app upgrade from 0.1.0 are verified on macOS arm64; remote code-package distribution and Gatekeeper acceptance remain pending.

SM6 local regression passes; the latest RT4 checks include 229 Rust tests, 36 application integration tests, all UI/headless journeys and four isolated native checks. Pure libraries compile for seven targets, and the application library compiles for four Apple targets. Complete builds and hosts on other platforms, and remote CI, remain unverified, so SM6 is partially complete. Earlier SM6 isolated release builds show 1.10% more application file bytes than 06ed67e (not remeasured for RT4); measurement conditions and evidence are in the audit above.

## License

The Cargo packages declare `LicenseRef-Proprietary` and are marked `publish = false`. Third-party dependencies retain their respective licenses.
