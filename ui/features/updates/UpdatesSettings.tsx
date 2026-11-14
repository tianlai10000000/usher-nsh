/** The update settings section reads a shared projection and dispatches semantic commands. */
import { Text, View } from "nsh";
import { Progress, Switch } from "nsh/base";
import type { AppState } from "../../state/app";
import type { UpdateActions } from "./index";
import { ThemeButton } from "../../components/ThemeControls";
import {
  groupLook,
  noteLook,
  rowLook,
  spacing,
  switchLook,
  createProgressLook,
  createSwitchLook,
} from "../../theme/look";

export function UpdateDetails(
  props: {
    appearance: AppState;
    updates: UpdateActions;
    testID?: string;
  },
) {
  const palette = props.appearance.appearance.palette;
  const t = props.appearance.preferences.t;
  const updates = props.updates;
  const view = updates.view;
  const downloading = () => updates.data().domain?.phase.type === "downloading";
  const progress = createProgressLook(palette, () => (view().progress ?? 0) * 100,
    () => downloading() && view().progress === null);
  const installing = () => ["installing", "awaiting_first_launch"].includes(updates.data().domain?.phase.type ?? "");
  return (
    <View
      testID={props.testID ?? "update-details"}
      style={{ ...spacing(palette).stack, width: "100%", minWidth: 0 }}
    >
      <Text
        testID="update-status"
        style={{ color: palette().text, width: "100%", numberOfLines: 0 }}
      >
        {t(view().key, { version: view().candidate?.version ?? "" })}
      </Text>
      <If when={updates.error()}>
        <Text
          testID="update-error"
          style={{ ...noteLook(palette), width: "100%", numberOfLines: 0 }}
        >
          {t(updates.error()!)}
        </Text>
      </If>
      <If when={updates.data().domain?.phase.type === "downloading"}>
        <Progress
          testID="update-progress"
          value={(view().progress ?? 0) * 100}
          indeterminate={view().progress === null}
          accessibilityLabel={t("update.downloading", {
            version: view().candidate?.version ?? "",
          })}
          onLayout={progress.onLayout}
          trackStyle={progress.trackStyle}
          indicatorStyle={progress.indicatorStyle}
        />
      </If>
      <If when={view().ready && view().candidate?.kind === "package"}>
        <Text
          testID="update-restart-note"
          style={{ ...noteLook(palette), width: "100%", numberOfLines: 0 }}
        >
          {t("update.nextStart")}
        </Text>
      </If>
      <View
        testID="update-actions"
        class="flex-row items-center"
        style={{ ...spacing(palette).wrap, width: "100%" }}
      >
        <If when={view().canDownload || downloading()}>
          <ThemeButton
            palette={palette}
            testID="update-download" loadingFeedback
            prominent
            busy={downloading()}
            interactionBlocked={view().working}
            label={t("update.download")}
            onPress={() => updates.download()}
          />
        </If>
        <If when={view().ready || installing()}>
          <ThemeButton
            palette={palette}
            testID="update-install" loadingFeedback
            prominent
            busy={installing()}
            label={t("update.restart")}
            onPress={() => updates.install()}
          />
        </If>
        <If when={!view().ready && !installing()}>
          <ThemeButton
            palette={palette}
            testID="update-check" loadingFeedback
            native={false}
            busy={updates.data().domain?.phase.type === "checking"}
            label={t("update.check")}
            disabled={view().unavailable || view().ready}
            interactionBlocked={view().working}
            onPress={() => updates.check()}
          />
        </If>
        <If
          when={!!updates.data().info.manual_url &&
            (view().failure !== undefined ||
              updates.data().domain?.phase.outcome?.type === "manual_upgrade")}
        >
          <ThemeButton
            palette={palette}
            testID="update-manual" loadingFeedback
            label={t("update.manualDownload")}
            onPress={() => updates.manual()}
          />
        </If>
      </View>
    </View>
  );
}
export function UpdatesSettings(
  props: { appearance: AppState; updates: UpdateActions },
) {
  const palette = props.appearance.appearance.palette;
  const t = props.appearance.preferences.t;
  const data = props.updates.data;
  const enabled = () => data().domain?.auto_check ?? true;
  const animatedSwitch = createSwitchLook(palette, enabled);
  const unknown = () => t("update.unknown");
  return (
    <View testID="settings-updates" style={spacing(palette).stack}>
      <View
        style={groupLook(
          palette,
          props.appearance.background.contentBackgroundTransparency(),
        )}
      >
        <View class="flex-row items-center" style={rowLook(palette)}>
          <View class="min-w-0 flex-1" style={spacing(palette).text}>
            <Text style={{ color: palette().text }}>
              {t("update.autoCheck")}
            </Text>
            <Text
              style={{ ...noteLook(palette), width: "100%", numberOfLines: 0 }}
            >
              {t("update.autoDescription")}
            </Text>
          </View>
          <Switch
            testID="update-auto-check"
            accessibilityLabel={t("update.autoCheck")}
            checked={enabled()}
            disabled={!data().domain}
            trackClass={animatedSwitch.trackClass}
            trackStyle={animatedSwitch.trackStyle}
            thumbStyle={animatedSwitch.thumbStyle}
            focusStyle={switchLook(palette, enabled()).focusStyle}
            onChange={({ checked }) => void props.updates.autoCheck(checked)}
          />
        </View>
        <View style={rowLook(palette, { divider: true })}>
          <UpdateDetails {...props} />
        </View>
      </View>
      <Text testID="update-shell-version" style={noteLook(palette)}>
        {t("update.shellVersion", {
          version: data().domain?.capability === "unsupported"
            ? unknown()
            : data().domain?.shell ?? unknown(),
          build: data().info.shell_build ?? unknown(),
        })}
      </Text>
      <Text style={noteLook(palette)}>
        {t("update.packageVersion", {
          version: data().info.package_build
            ? data().domain?.running ?? unknown()
            : t("update.bundled"),
          build: data().info.package_build ?? unknown(),
        })}
      </Text>
      <Text testID="update-last-check" style={noteLook(palette)}>
        {t("update.lastCheck", {
          time: data().info.last_checked
            ? new Date(data().info.last_checked!).toLocaleString()
            : t("update.never"),
        })}
      </Text>
      <If when={data().info.last_result}>
        <Text style={noteLook(palette)}>
          {t(
            `update.result.${data().info.last_result}` as import("../../i18n").MessageKey,
          )}
        </Text>
      </If>
    </View>
  );
}
