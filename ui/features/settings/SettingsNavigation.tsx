/** Uses framework tabs or native AppKit navigation to select settings sections. */
import { Text, View } from "nsh";
import { Select, Tab, Tabs } from "nsh/base";
import { AppKitPopup, AppKitSidebar } from "../../components/AppKitControls";
import { SystemIcon } from "../../components/SystemIcon";
import type { MessageKey } from "../../i18n";
import { createSelectMotion } from "../../theme/motion";
import { fieldLook, selectLook, spacing, tabLook } from "../../theme/look";
import { isSettingsSection, settingsSections, type AppearanceStore } from "../../state/appearance";

/** Native rows are 32 points tall (see `SIDEBAR_ROW` in `platforms/macos/controls.rs`). */
const nativeRow = 32;

function useSections(appearance: AppearanceStore) {
  const { t, settingsSection } = appearance;
  const items = () =>
    settingsSections.map((id) => ({ id, label: t(`settings.section.${id}` as MessageKey) }));
  const choose = (id: string) => {
    if (isSettingsSection(id)) appearance.setSettingsSection(id);
  };
  const currentLabel = () => items().find((item) => item.id === settingsSection())?.label ?? "";
  return { items, choose, currentLabel };
}

/** Wide windows: the sections down the left side. */
export function SettingsNavigation(props: { appearance: AppearanceStore }) {
  const { palette, t, settingsSection } = props.appearance;
  const { items, choose } = useSections(props.appearance);

  return (
    <View testID="settings-navigation" role="navigation" accessibilityLabel={t("settings.sections")}
      class="w-[180px]" style={spacing(palette).text}>
      <If
        when={palette().nativeAppKit}
        else={
          <Tabs testID="settings-tabs" style={spacing(palette).text}
            indicatorStyle={tabLook(palette).indicatorStyle}>
            <Each of={items()}>
              {(item) => {
                const selected = () => settingsSection() === item().id;
                const look = () => tabLook(palette, selected());
                return (
                  <Tab testID={`settings-section-${item().id}`} selected={selected()}
                    style={look().style}
                    hoverStyle={look().hoverStyle}
                    activeStyle={look().activeStyle}
                    focusVisibleStyle={look().focusVisibleStyle}
                    onPress={() => choose(item().id)}>
                    <Text style={look().label}>
                      {item().label}
                    </Text>
                  </Tab>
                );
              }}
            </Each>
          </Tabs>
        }
      >
        {/* The liquid glass family draws the list with AppKit's own buttons. */}
        <AppKitSidebar
          testID="settings-section-native"
          style={{ width: 180, height: items().length * nativeRow }}
          label={t("settings.sections")}
          items={items()}
          selectedId={settingsSection()}
          enabled
          onChange={({ id }) => choose(id)}
        />
      </If>
    </View>
  );
}

/** Narrow windows: one drop-down in place of the list. */
export function SettingsSectionPicker(props: { appearance: AppearanceStore }) {
  const { palette, t, settingsSection } = props.appearance;
  const { items, choose, currentLabel } = useSections(props.appearance);
  const popup = createSelectMotion(palette);

  return (
    <If
      when={palette().nativeAppKit}
      else={
        <Select
          accessibilityLabel={t("settings.sections")}
          items={items().map((item) => ({ value: item.id, title: item.label }))}
          value={settingsSection()}
          onChange={({ value }) => {
            if (value !== null) choose(value);
          }}
          popupClass="border"
          popupStyle={{ ...selectLook(palette).popupStyle, ...popup.style }}
          onOpen={popup.show} onClose={popup.hide}
          itemClass={selectLook(palette).itemClass}
          itemStyle={selectLook(palette).itemStyle}
          trigger={
            <View testID="settings-section-picker" accessibilityLabel={t("settings.sections")}
              class="flex-row items-center justify-between"
              style={{ ...fieldLook(palette).style, backgroundColor: palette().surface }}
              hoverStyle={fieldLook(palette).hoverStyle}>
              <Text style={{ color: palette().text }}>{currentLabel()}</Text>
              <SystemIcon name="chevron.up.chevron.down" fallback="⌄" size={12} color={palette().muted} />
            </View>
          }
        />
      }
    >
      {/* AppKit's own pop-up button for the liquid glass family. */}
      <AppKitPopup
        testID="settings-section-native-picker"
        style={{ width: "100%", height: 28 }}
        label={t("settings.sections")}
        items={items()}
        selectedId={settingsSection()}
        enabled
        onChange={({ id }) => choose(id)}
      />
    </If>
  );
}
