/** Composes general, theme, diagnostic and update sections over shared Rust projections. */
import appearanceConfig from "../../config/appearance.json";
import { UpdatesSettings } from "../updates/UpdatesSettings";
import type { Updates } from "../updates/index";
import { signal, Text, View } from "nsh";
import { ThemeSwitch } from "../../components/ValueControls";
import { isLocalePreference, type LocalePreference } from "../../i18n/index";
import { isComponentFamily, type AppState, type ColorMode } from "../../state/app";
import { AppKitPopup, AppKitSegments } from "../../components/AppKitControls";
import type { ComponentFamily } from "../../theme/index";
import { EffectiveAppearance } from "./EffectiveAppearance";
import { ThemeButton } from "../../components/ThemeControls";
import { DiagnosticsSettings } from "./DiagnosticsSettings";
import { SettingsNavigation, SettingsSectionPicker } from "./SettingsNavigation";
import { ThemeSettings } from "./ThemeSettings";
import { BackgroundSettings } from "./BackgroundSettings";
import { groupLook, headingLook, noteLook, rowLook, sectionTitleLook, spacing } from "../../theme/look";

const modes = appearanceConfig.modes as readonly ColorMode[];
const languages: readonly LocalePreference[] = ["system", "en", "zh-Hans"];

function GeneralSettings(props: { appearance: AppState }) {
  const palette = props.appearance.appearance.palette;
  const t = props.appearance.preferences.t;
  const busy = props.appearance.settings.busy;
  const componentFamilies: readonly ComponentFamily[] = props.appearance.appearance.appKitAvailable
    ? ["nsh", "appkit"]
    : ["nsh"];

  return (
    <View testID="settings-general" style={spacing(palette).page}>
      <View style={spacing(palette).group}>
        <Text role="header" style={sectionTitleLook(palette)}>
          {t("settings.appearance")}
        </Text>
        <View style={groupLook(palette, props.appearance.background.contentBackgroundTransparency())}>
          <View class="flex-row items-center" style={rowLook(palette)}>
            <View class="min-w-0 flex-1" style={spacing(palette).text}>
              <Text style={{ color: palette().text }}>{t("settings.appearance")}</Text>
              <Text style={noteLook(palette)}>{t("settings.appearanceDescription")}</Text>
              <EffectiveAppearance appearance={props.appearance} />
            </View>
            <View class="min-w-0 flex-1 items-end">
              <If
                when={palette().nativeAppKit}
                else={
                  <View class="flex-row flex-wrap justify-end" style={spacing(palette).wrap}>
                    <Each of={modes}>
                      {(mode) => (
                        <ThemeButton palette={palette} testID={`appearance-${mode()}`} label={t(`mode.${mode()}`)}
                          selected={props.appearance.appearance.mode() === mode()} interactionBlocked={busy()}
                          onPress={() => props.appearance.appearance.choose(mode())} />
                      )}
                    </Each>
                  </View>
                }
              >
                <AppKitSegments
                  testID="appearance-native"
                  style={{ width: "100%", height: 28 }}
                  label={t("settings.appearance")}
                  items={modes.map((mode) => ({ id: mode, label: t(`mode.${mode}`) }))}
                  selectedId={props.appearance.appearance.mode()}
                  enabled interactionBlocked={busy()}
                  onChange={({ id }) => {
                    const mode = modes.find((candidate) => candidate === id);
                    if (mode !== undefined) void props.appearance.appearance.choose(mode);
                  }}
                />
              </If>
            </View>
          </View>
          <View class="flex-row items-center" style={rowLook(palette, { divider: true })}>
            <View class="min-w-0 flex-1" style={spacing(palette).text}>
              <Text style={{ color: palette().text }}>{t("settings.componentFamily")}</Text>
              <If when={props.appearance.appearance.componentFamily() === "appkit"}>
                <Text testID="appkit-status" style={noteLook(palette)}>
                  {t(palette().nativeAppKit ? "theme.appKitActive" : "theme.appKitFallback")}
                </Text>
              </If>
            </View>
            <View class="min-w-0 flex-1 items-end">
              <If
                when={palette().nativeAppKit}
                else={
                  <View class="flex-row flex-wrap justify-end" style={spacing(palette).wrap}>
                    <Each of={componentFamilies}>
                      {(family) => (
                        <ThemeButton
                          palette={palette}
                          testID={`component-family-${family()}`}
                          label={t(`settings.family.${family()}`)}
                          selected={props.appearance.appearance.componentFamily() === family()}
                          interactionBlocked={busy()}
                          onPress={() => props.appearance.appearance.chooseComponentFamily(family())}
                        />
                      )}
                    </Each>
                  </View>
                }
              >
                <AppKitSegments
                  testID="component-family-native"
                  style={{ width: "100%", height: 28 }}
                  label={t("settings.componentFamily")}
                  items={componentFamilies.map((id) => ({ id, label: t(`settings.family.${id}`) }))}
                  selectedId={props.appearance.appearance.componentFamily()}
                  enabled interactionBlocked={busy()}
                  onChange={({ id }) => {
                    if (isComponentFamily(id)) void props.appearance.appearance.chooseComponentFamily(id);
                  }}
                />
              </If>
            </View>
          </View>
          <View class="flex-row items-center" style={rowLook(palette, { divider: true })}>
            <View class="min-w-0 flex-1">
              <Text style={{ color: palette().text }}>{t("settings.showStatusBar")}</Text>
            </View>
            <View class="min-w-0 flex-1 items-end">
              <If
                when={palette().nativeAppKit}
                else={
                  <ThemeSwitch palette={palette}
                    testID="show-status-bar"
                    accessibilityLabel={t("settings.showStatusBar")}
                    checked={props.appearance.preferences.showStatusBar()}
                    interactionBlocked={busy()}
                    onChange={({ checked }) => void props.appearance.preferences.setShowStatusBar(checked)}
                  />
                }
              >
                <AppKitSegments
                  testID="show-status-bar-native"
                  style={{ width: "100%", height: 28 }}
                  label={t("settings.showStatusBar")}
                  items={[
                    { id: "show", label: t("settings.statusBarShown") },
                    { id: "hide", label: t("settings.statusBarHidden") },
                  ]}
                  selectedId={props.appearance.preferences.showStatusBar() ? "show" : "hide"}
                  enabled interactionBlocked={busy()}
                  onChange={({ id }) => {
                    if (id === "show" || id === "hide") void props.appearance.preferences.setShowStatusBar(id === "show");
                  }}
                />
              </If>
            </View>
          </View>
        </View>
      </View>
      <BackgroundSettings appearance={props.appearance} />
      <View style={spacing(palette).group}>
        <Text role="header" style={sectionTitleLook(palette)}>
          {t("settings.language")}
        </Text>
        <View style={groupLook(palette, props.appearance.background.contentBackgroundTransparency())}>
          <View class="flex-row items-center" style={rowLook(palette)}>
            <View class="min-w-0 flex-1" style={spacing(palette).text}>
              <Text style={{ color: palette().text }}>{t("settings.language")}</Text>
              <Text style={noteLook(palette)}>{t("settings.languageDescription")}</Text>
            </View>
            <View class="min-w-0 flex-1 items-end">
              <If
                when={palette().nativeAppKit}
                else={
                  <View class="flex-row flex-wrap justify-end" style={spacing(palette).wrap}>
                    <Each of={languages}>
                      {(language) => (
                        <ThemeButton palette={palette} testID={`locale-${language()}`} label={t(`locale.${language()}`)}
                          selected={props.appearance.preferences.language() === language()} interactionBlocked={busy()}
                          onPress={() => props.appearance.preferences.chooseLanguage(language())} />
                      )}
                    </Each>
                  </View>
                }
              >
                <AppKitPopup
                  testID="locale-native"
                  style={{ width: "100%", height: 28 }}
                  label={t("settings.language")}
                  items={languages.map((language) => ({ id: language, label: t(`locale.${language}`) }))}
                  selectedId={props.appearance.preferences.language()}
                  enabled interactionBlocked={busy()}
                  onChange={({ id }) => {
                    if (isLocalePreference(id)) void props.appearance.preferences.chooseLanguage(id);
                  }}
                />
              </If>
            </View>
          </View>
        </View>
      </View>
    </View>
  );
}

/**
 * Below this page width the sections move from a column on the left into one drop-down:
 * the column takes 180 + 32 points and the cards need about 350 to stay readable.
 */
const wideBreakpoint = 560;

export function SettingsPage(props: { appearance: AppState; updates?: Updates }) {
  const palette = props.appearance.appearance.palette;
  const t = props.appearance.preferences.t;
  const busy = props.appearance.settings.busy;
  const retryable = props.appearance.settings.retryable;
  const settingsSection = props.appearance.settings.settingsSection;
  // The page's own width, not the window's: the docked sidebar takes room too.
  const width = signal(0);
  const wide = () => width() >= wideBreakpoint;

  const content = () => (
    <View class="min-w-0 flex-1" style={spacing(palette).stack}>
      <If when={settingsSection() === "general"}>
        <GeneralSettings appearance={props.appearance} />
      </If>
      <If when={settingsSection() === "theme"}>
        <ThemeSettings appearance={props.appearance} busy={busy} />
      </If>
      <If when={settingsSection() === "updates" && !!props.updates}>
        <UpdatesSettings appearance={props.appearance} updates={props.updates!} />
      </If>
      <If when={settingsSection() === "diagnostics"}>
        <DiagnosticsSettings appearance={props.appearance} />
      </If>
    </View>
  );

  return (
    <View class="w-full max-w-[960px] self-center" style={spacing(palette).stack}
      onLayout={({ bounds }) => width.set(bounds.size.width)}>
      <Text role="header" style={headingLook(palette, "display")}>
        {t("settings.title")}
      </Text>
      <If when={retryable()}>
        <ThemeButton palette={palette} label={t("settings.retry")} testID="settings-retry" loadingFeedback
          onPress={() => props.appearance.settings.retry()} />
      </If>
      <If
        when={wide()}
        else={
          <View testID="settings-layout-compact" style={spacing(palette).stack}>
            <SettingsSectionPicker appearance={props.appearance} />
            {content()}
          </View>
        }
      >
        <View testID="settings-layout-wide" class="flex-row items-start" style={{ gap: palette().metric.pageGap + palette().metric.groupGap }}>
          <SettingsNavigation appearance={props.appearance} />
          {content()}
        </View>
      </If>
    </View>
  );
}
