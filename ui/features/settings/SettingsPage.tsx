/** Composes general, theme, diagnostic and update sections over shared Rust projections. */
import { UpdatesSettings } from "../updates/UpdatesSettings";
import type { Updates } from "../updates";
import { signal, Text, View } from "nsh";
import { Switch } from "nsh/base";
import { isLocalePreference, type LocalePreference } from "../../i18n";
import { isComponentFamily, type AppearanceStore, type ColorMode } from "../../state/appearance";
import { AppKitPopup, AppKitSegments } from "../../components/AppKitControls";
import type { ComponentFamily } from "../../theme";
import { EffectiveAppearance } from "./EffectiveAppearance";
import { ThemeButton } from "../../components/ThemeControls";
import { DiagnosticsSettings } from "./DiagnosticsSettings";
import { SettingsNavigation, SettingsSectionPicker } from "./SettingsNavigation";
import { ThemeSettings } from "./ThemeSettings";
import { BackgroundSettings } from "./BackgroundSettings";
import { groupLook, headingLook, noteLook, rowLook, sectionTitleLook, spacing, switchLook, createSwitchLook } from "../../theme/look";

const modes: readonly ColorMode[] = ["light", "dark", "system"];
const languages: readonly LocalePreference[] = ["system", "en", "zh-Hans"];

function GeneralSettings(props: { appearance: AppearanceStore }) {
  const { palette, t, busy } = props.appearance;
  const statusSwitch = createSwitchLook(palette, props.appearance.showStatusBar);
  const componentFamilies: readonly ComponentFamily[] = props.appearance.appKitAvailable
    ? ["nsh", "appkit"]
    : ["nsh"];

  return (
    <View testID="settings-general" style={spacing(palette).page}>
      <View style={spacing(palette).group}>
        <Text role="header" style={sectionTitleLook(palette)}>
          {t("settings.appearance")}
        </Text>
        <View style={groupLook(palette, props.appearance.contentBackgroundTransparency())}>
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
                          selected={props.appearance.mode() === mode()} disabled={busy()}
                          onPress={() => props.appearance.choose(mode())} />
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
                  selectedId={props.appearance.mode()}
                  enabled={!busy()}
                  onChange={({ id }) => {
                    const mode = modes.find((candidate) => candidate === id);
                    if (mode !== undefined) void props.appearance.choose(mode);
                  }}
                />
              </If>
            </View>
          </View>
          <View class="flex-row items-center" style={rowLook(palette, { divider: true })}>
            <View class="min-w-0 flex-1" style={spacing(palette).text}>
              <Text style={{ color: palette().text }}>{t("settings.componentFamily")}</Text>
              <If when={props.appearance.componentFamily() === "appkit"}>
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
                          selected={props.appearance.componentFamily() === family()}
                          disabled={busy()}
                          onPress={() => props.appearance.chooseComponentFamily(family())}
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
                  selectedId={props.appearance.componentFamily()}
                  enabled={!busy()}
                  onChange={({ id }) => {
                    if (isComponentFamily(id)) void props.appearance.chooseComponentFamily(id);
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
                  <Switch
                    testID="show-status-bar"
                    accessibilityLabel={t("settings.showStatusBar")}
                    checked={props.appearance.showStatusBar()}
                    disabled={busy()}
                    disabledStyle={switchLook(palette, false).disabledStyle}
                    focusStyle={switchLook(palette, false).focusStyle}
                    trackClass={statusSwitch.trackClass}
                    trackStyle={statusSwitch.trackStyle}
                    thumbStyle={statusSwitch.thumbStyle}
                    onChange={({ checked }) => void props.appearance.setShowStatusBar(checked)}
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
                  selectedId={props.appearance.showStatusBar() ? "show" : "hide"}
                  enabled={!busy()}
                  onChange={({ id }) => {
                    if (id === "show" || id === "hide") void props.appearance.setShowStatusBar(id === "show");
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
        <View style={groupLook(palette, props.appearance.contentBackgroundTransparency())}>
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
                          selected={props.appearance.language() === language()} disabled={busy()}
                          onPress={() => props.appearance.chooseLanguage(language())} />
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
                  selectedId={props.appearance.language()}
                  enabled={!busy()}
                  onChange={({ id }) => {
                    if (isLocalePreference(id)) void props.appearance.chooseLanguage(id);
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

export function SettingsPage(props: { appearance: AppearanceStore; updates?: Updates }) {
  const { palette, t, busy, retryable, settingsSection } = props.appearance;
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
          onPress={() => props.appearance.retry()} />
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
