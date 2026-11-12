/** Presents theme selection, local theme management and palette editing with shared settings. */
import { computed, effect, platform, signal, Text, View } from "nsh";
import { defaultThemeId, editorPalette, fallbackTheme, type CustomPalette, type CustomTheme, type Scheme } from "../../theme";
import type { MessageKey } from "../../i18n";
import type { AppearanceStore } from "../../state/appearance";
import { ThemeButton, ThemeInput } from "../../components/ThemeControls";
import { AppKitPopup, AppKitSegments } from "../../components/AppKitControls";
import { groupLook, headingLook, noteLook, rowLook, sectionTitleLook, spacing } from "../../theme/look";

const fields = ["background", "surface", "text", "accent"] as const;
const schemes = ["light", "dark"] as const;
const themeIssueReasons: Partial<Record<string, MessageKey>> = {
  style_theme_duplicate_id: "theme.issue.duplicateId",
  style_theme_file_too_large: "theme.issue.tooLarge",
  style_theme_invalid_color: "theme.issue.invalidColor",
  style_theme_invalid_format: "theme.issue.invalidFormat",
  style_theme_invalid_id: "theme.issue.invalidId",
  style_theme_invalid_json: "theme.issue.invalidJson",
  style_theme_invalid_name: "theme.issue.invalidName",
  style_theme_invalid_radius: "theme.issue.invalidRadius",
  style_theme_invalid_token: "theme.issue.invalidToken",
  style_theme_too_many_tokens: "theme.issue.tooManyTokens",
  style_theme_invalid_base: "theme.issue.invalidBase",
  style_theme_invalid_reference: "theme.issue.invalidReference",
  style_theme_legacy_conflict: "theme.issue.legacyConflict",
  style_theme_not_regular_file: "theme.issue.notRegular",
  style_theme_read_failed: "theme.issue.readFailed",
  style_theme_unsupported_version: "theme.issue.unsupportedVersion",
};

function copyPalette({ background, surface, text, accent }: CustomPalette): CustomPalette {
  return { background, surface, text, accent };
}

export function ThemeSettings(props: { appearance: AppearanceStore; busy: () => boolean }) {
  const { palette, t } = props.appearance;
  const accentDraft = signal("");
  const draft = signal<CustomTheme | null>(null);
  const copiedStyle = signal<string | undefined>();
  const scheme = signal<Scheme>("light");
  const activeDraft = computed(() => draft()?.[scheme()]);
  // The registry lists every selectable theme, bundled, file-based and editor-made, in order.
  const themeChoices = computed(() =>
    props.appearance.styleThemes().map((theme) => ({ id: theme.id, label: props.appearance.themeName(theme) })),
  );
  const selectedStyle = computed(() =>
    themeChoices().find(({ id }) => id === props.appearance.themeId()),
  );
  const baseTheme = () => props.appearance.styleThemes().find((theme) => theme.id === defaultThemeId) ?? fallbackTheme;
  const canOpenDirectory = platform.desktop && platform.os !== "web";
  effect(() => accentDraft.set(props.appearance.accent() ?? ""));

  function edit(theme: CustomTheme | undefined) {
    scheme.set("light");
    if (theme) draft.set(theme);
  }

  function createDraft() {
    // Editing uses a local draft. Saving commits both palettes and their selection together.
    edit({
      id: `custom-${Date.now().toString(36)}-${Math.random().toString(36).slice(2, 8)}`,
      name: "",
      light: copyPalette(editorPalette(baseTheme(), "light")),
      dark: copyPalette(editorPalette(baseTheme(), "dark")),
    });
  }

  async function duplicateStyle() {
    const source = selectedStyle();
    if (!source) return;

    // Match the settings schema's character limit without splitting surrogate pairs.
    const name = Array.from(t("theme.copyName", { name: source.label })).slice(0, 80).join("").trimEnd();
    copiedStyle.set(undefined);
    if (await props.appearance.cloneStyleTheme(source.id, name)) {
      copiedStyle.set(props.appearance.themeId());
    }
  }

  function changeColor(field: keyof CustomPalette, value: string) {
    const current = draft();
    if (current) draft.set({ ...current, [scheme()]: { ...current[scheme()], [field]: value } });
  }

  async function save() {
    const current = draft();
    if (current && await props.appearance.saveTheme(current)) draft.set(null);
  }

  function previewColor(field: keyof CustomPalette) {
    const value = activeDraft()?.[field];
    return value && /^#[0-9a-f]{6}$/i.test(value) ? value : editorPalette(baseTheme(), scheme())[field];
  }

  return (
    <View testID="theme-settings" style={spacing(palette).page}>
      <View style={spacing(palette).group}>
        <Text role="header" style={sectionTitleLook(palette)}>
          {t("theme.title")}
        </Text>
        <View style={groupLook(palette, props.appearance.contentBackgroundTransparency())}>
          <View class="flex-row items-center" style={rowLook(palette)}>
            <View class="min-w-0 flex-1" style={spacing(palette).text}>
              <Text style={{ color: palette().text }}>{t("theme.title")}</Text>
              <If when={palette().glass}>
                <Text testID="glass-status" style={noteLook(palette)}>
                  {t("theme.glassActive", { name: selectedStyle()?.label ?? "" })}
                </Text>
              </If>
              <If when={selectedStyle() === undefined}>
                <Text testID="theme-unavailable" style={noteLook(palette)}>
                  {t("theme.unavailable")}
                </Text>
              </If>
              <If when={copiedStyle() === props.appearance.themeId()}>
                <Text testID="theme-copied" style={noteLook(palette)}>
                  {t("theme.copyDone")}
                </Text>
              </If>
            </View>
            <View class="min-w-0 flex-1 items-end">
              <If
                when={palette().nativeAppKit}
                else={
                  <View class="flex-row flex-wrap justify-end" style={spacing(palette).wrap}>
                    <Each of={themeChoices()} key={(choice) => choice.id}>
                      {(choice) => (
                        <ThemeButton
                          palette={palette}
                          testID={`theme-${choice().id}`}
                          label={choice().label}
                          selected={props.appearance.themeId() === choice().id}
                          disabled={props.busy()}
                          onPress={() => void props.appearance.chooseTheme(choice().id)}
                        />
                      )}
                    </Each>
                  </View>
                }
              >
                <AppKitPopup
                  testID="theme-native"
                  style={{ width: "100%", height: 28 }}
                  label={t("theme.title")}
                  items={themeChoices()}
                  selectedId={props.appearance.themeId()}
                  enabled={!props.busy()}
                  onChange={({ id }) => void props.appearance.chooseTheme(id)}
                />
              </If>
            </View>
          </View>
          <View class="flex-row items-center" style={rowLook(palette, { divider: true })}>
            <View class="min-w-0 flex-1" style={spacing(palette).text}>
              <Text style={{ color: palette().text }}>{t("theme.accent")}</Text>
            </View>
            <View class="min-w-0 flex-1" style={spacing(palette).group}>
              <ThemeInput palette={palette} label={t("theme.accent")} testID="accent-input"
                placeholder={t("theme.accentPlaceholder")} value={accentDraft()} onChange={(value) => accentDraft.set(value)} />
              <View class="flex-row flex-wrap justify-end" style={spacing(palette).wrap}>
                <ThemeButton palette={palette} testID="accent-apply" label={t("theme.applyAccent")}
                  disabled={props.busy()} onPress={() => void props.appearance.setAccent(accentDraft().trim())} />
                <ThemeButton palette={palette} testID="accent-reset" label={t("theme.accentDefault")}
                  disabled={props.busy()} onPress={() => {
                    accentDraft.set("");
                    void props.appearance.setAccent(null);
                  }} />
              </View>
            </View>
          </View>
        </View>
      </View>
      <View style={spacing(palette).group}>
        <Text role="header" style={sectionTitleLook(palette)}>
          {t("theme.custom")}
        </Text>
        <View style={groupLook(palette, props.appearance.contentBackgroundTransparency())}>
          <View class="flex-row items-center" style={rowLook(palette)}>
            <View class="min-w-0 flex-1" style={spacing(palette).text}>
              <If when={canOpenDirectory}>
                <Text style={{ color: palette().text }}>{t("theme.openDirectory")}</Text>
              </If>
              <If when={props.appearance.directoryProblem()}>{(problem) => (
                <Text testID="theme-directory-problem" style={noteLook(palette)}>{t(problem())}</Text>
              )}</If>
              <Each of={props.appearance.themeFileIssues()}>
                {(issue) => (
                  <Text testID="theme-file-issue" style={noteLook(palette)}>
                    {issue().code === "style_theme_directory_unavailable"
                      ? t("theme.directoryUnavailable")
                      : t("theme.fileIssue", {
                        name: issue().fileName,
                        reason: t(themeIssueReasons[issue().code] ?? "theme.issue.invalidFormat"),
                      })}
                  </Text>
                )}
              </Each>
            </View>
            <View class="min-w-0 flex-1 flex-row flex-wrap items-center justify-end" style={spacing(palette).wrap}>
              <If when={canOpenDirectory}>
                <ThemeButton palette={palette} testID="theme-directory-open" label={t("theme.openDirectory")}
                  disabled={props.busy() || props.appearance.openingDirectory()}
                  onPress={() => void props.appearance.openThemeDirectory()} />
              </If>
              <ThemeButton palette={palette} testID="theme-directory-rescan" label={t("theme.rescan")}
                disabled={props.busy()} onPress={() => void props.appearance.rescanThemes()} />
            </View>
          </View>
          <View class="flex-row flex-wrap items-center justify-end" style={{ ...rowLook(palette, { divider: true }), ...spacing(palette).wrap }}>
            <ThemeButton palette={palette} testID="theme-add" label={t("theme.add")}
              disabled={props.busy()} onPress={() => createDraft()} />
            <If when={selectedStyle() !== undefined && props.appearance.selectedCustomTheme() === undefined}>
              <ThemeButton palette={palette} testID="theme-clone" label={t("theme.cloneStyle")}
                disabled={props.busy()} onPress={() => void duplicateStyle()} />
            </If>
            <If when={props.appearance.selectedCustomTheme() !== undefined}>
              <ThemeButton palette={palette} testID="theme-edit" label={t("theme.edit")}
                disabled={props.busy()} onPress={() => edit(props.appearance.selectedCustomTheme())} />
              <ThemeButton palette={palette} testID="theme-remove" label={t("theme.remove")}
                disabled={props.busy()} onPress={() => void props.appearance.removeTheme(props.appearance.themeId())} />
            </If>
          </View>
          <If when={draft() !== null}>
            <View style={{ ...rowLook(palette, { divider: true }), ...spacing(palette).stack }}>
              <Text style={{ color: palette().text }}>{t("theme.customName")}</Text>
              <ThemeInput palette={palette} label={t("theme.customName")} testID="theme-name"
                placeholder={t("theme.namePlaceholder")} value={draft()?.name ?? ""}
                onChange={(name) => draft.update((current) => current && { ...current, name })} />
              <If
                when={palette().nativeAppKit}
                else={
                  <View class="flex-row flex-wrap" style={spacing(palette).wrap}>
                    <Each of={schemes}>
                      {(item) => <ThemeButton palette={palette} testID={`palette-${item()}`}
                        label={t(item() === "light" ? "theme.chooseLightPalette" : "theme.chooseDarkPalette")}
                        selected={scheme() === item()} onPress={() => scheme.set(item())} />}
                    </Each>
                  </View>
                }
              >
                <AppKitSegments
                  testID="palette-scheme-native"
                  style={{ width: "100%", height: 28 }}
                  label={t("theme.title")}
                  items={schemes.map((item) => ({
                    id: item,
                    label: t(item === "light" ? "theme.chooseLightPalette" : "theme.chooseDarkPalette"),
                  }))}
                  selectedId={scheme()}
                  enabled
                  onChange={({ id }) => {
                    if (id === "light" || id === "dark") scheme.set(id);
                  }}
                />
              </If>
              <Each of={fields}>
                {(field) => (
                  <View style={spacing(palette).text}>
                    <Text style={{ color: palette().text }}>{t(`theme.${field()}`)}</Text>
                    <ThemeInput palette={palette} label={t(`theme.${field()}`)} testID={`palette-${field()}`}
                      value={activeDraft()?.[field()] ?? ""} onChange={(value) => changeColor(field(), value)} />
                  </View>
                )}
              </Each>
              <View style={{ padding: palette().metric.stackGap, borderRadius: palette().metric.radiusCard, backgroundColor: previewColor("background") }}>
                <View style={{ ...spacing(palette).group, padding: palette().metric.stackGap, borderRadius: palette().metric.radiusCard, backgroundColor: previewColor("surface") }}>
                  <Text style={{ color: previewColor("text") }}>{t("theme.preview")}</Text>
                  <View style={{ height: palette().metric.wrapGap, width: palette().metric.rowPadX * 3, borderRadius: palette().metric.wrapGap, backgroundColor: previewColor("accent") }} />
                </View>
              </View>
              <View class="flex-row flex-wrap justify-end" style={spacing(palette).wrap}>
                <ThemeButton palette={palette} testID="theme-save" label={t("theme.save")}
                  prominent disabled={props.busy()} onPress={() => void save()} />
                <ThemeButton palette={palette} testID="theme-cancel" label={t("theme.cancel")}
                  onPress={() => draft.set(null)} />
              </View>
            </View>
          </If>
        </View>
      </View>
    </View>
  );
}
