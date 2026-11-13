/** The update settings section reads a shared projection and dispatches semantic commands. */
import { reduceMotion, Text, View } from "nsh";
import { Progress, Switch } from "nsh/base";
import type { AppearanceStore } from "../../state/appearance";
import type { UpdateActions } from "./index";
import { ThemeButton } from "../../components/ThemeControls";
import {
  groupLook,
  noteLook,
  rowLook,
  spacing,
  switchLook,
} from "../../theme/look";

export function UpdateDetails(
  props: {
    appearance: AppearanceStore;
    updates: UpdateActions;
  },
) {
  const { palette, t } = props.appearance;
  const updates = props.updates;
  const view = updates.view;
  return (
    <View
      testID="update-details"
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
          trackStyle={{
            height: palette().metric.sliderTrack,
            backgroundColor: palette().border,
          }}
          indicatorStyle={{
            height: palette().metric.sliderTrack,
            backgroundColor: palette().accent,
            ...(view().progress === null
              ? {
                width: `${
                  palette().metric.updateSegment * 100
                }%` as `${number}%`,
                animationName: reduceMotion()
                  ? "none" as const
                  : "pulse" as const,
                animationDuration: palette().metric.updateSpinMs,
                animationIterationCount: "infinite" as const,
              }
              : {}),
          }}
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
        class="flex-row flex-wrap items-center"
        style={{ ...spacing(palette).wrap, width: "100%" }}
      >
        <If when={view().canDownload}>
          <ThemeButton
            palette={palette}
            testID="update-download"
            prominent
            label={t("update.download")}
            onPress={() => void updates.download()}
          />
        </If>
        <If when={view().ready}>
          <ThemeButton
            palette={palette}
            testID="update-install"
            prominent
            label={t("update.restart")}
            onPress={() => void updates.install()}
          />
        </If>
        <If when={!view().ready}>
          <ThemeButton
            palette={palette}
            testID="update-check"
            label={t(view().canRetryCheck ? "update.retry" : "update.check")}
            disabled={!view().canCheck}
            onPress={() => void updates.check()}
          />
        </If>
        <If
          when={!!updates.data().info.manual_url &&
            (view().failure !== undefined ||
              updates.data().domain?.phase.outcome?.type === "manual_upgrade")}
        >
          <ThemeButton
            palette={palette}
            testID="update-manual"
            label={t("update.manualDownload")}
            onPress={() => void updates.manual()}
          />
        </If>
      </View>
    </View>
  );
}
export function UpdatesSettings(
  props: { appearance: AppearanceStore; updates: UpdateActions },
) {
  const { palette, t } = props.appearance;
  const data = props.updates.data;
  const enabled = () => data().domain?.auto_check ?? true;
  const unknown = () => t("update.unknown");
  return (
    <View testID="settings-updates" style={spacing(palette).stack}>
      <View
        style={groupLook(
          palette,
          props.appearance.contentBackgroundTransparency(),
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
            trackStyle={switchLook(palette, enabled()).trackStyle}
            thumbStyle={switchLook(palette, enabled()).thumbStyle}
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
