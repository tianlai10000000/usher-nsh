/** Presents diagnostic level and export controls through the shared appearance store. */
import { signal, Text, View } from "nsh";
import { AppKitPopup } from "../../components/AppKitControls";
import { ThemeButton } from "../../components/ThemeControls";
import { diagnosticLevels, type AppearanceStore, type DiagnosticLevel } from "../../state/appearance";
import { groupLook, headingLook, noteLook, rowLook, sectionTitleLook, spacing } from "../../theme/look";

const ranges = [
  { id: "today", days: 1 },
  { id: "week", days: 7 },
  { id: "all", days: 14 },
] as const;

function isDiagnosticLevel(value: string): value is DiagnosticLevel {
  return (diagnosticLevels as readonly string[]).includes(value);
}

export function DiagnosticsSettings(props: { appearance: AppearanceStore }) {
  const { palette, t, busy, exportState } = props.appearance;
  const range = signal<(typeof ranges)[number]["id"]>("week");
  const days = () => ranges.find((item) => item.id === range())?.days ?? 7;

  return (
    <View testID="diagnostics-settings" style={spacing(palette).group}>
      <Text role="header" style={sectionTitleLook(palette)}>
        {t("diagnostics.title")}
      </Text>
      <View style={groupLook(palette, props.appearance.contentBackgroundTransparency())}>
        <View class="flex-row items-center" style={rowLook(palette)}>
          <View class="min-w-0 flex-1" style={spacing(palette).text}>
            <Text style={{ color: palette().text }}>{t("diagnostics.level")}</Text>
            <Text style={noteLook(palette)}>{t("diagnostics.description")}</Text>
            <If when={props.appearance.diagnosticProblem() !== null}>
              <Text testID="diagnostic-problem" style={noteLook(palette, "text")}>
                {t(props.appearance.diagnosticProblem() === "diagnostics_directory_unavailable"
                  ? "diagnostics.error.diagnostics_directory_unavailable"
                  : "diagnostics.error.diagnostics_write_failed")}
              </Text>
            </If>
            <If when={props.appearance.diagnosticDropped() > 0}>
              <Text testID="diagnostic-dropped" style={noteLook(palette)}>
                {t("diagnostics.dropped", { count: props.appearance.diagnosticDropped() })}
              </Text>
            </If>
          </View>
          <View class="min-w-0 flex-1 items-end">
            <If
              when={palette().nativeAppKit}
              else={
                <View class="flex-row flex-wrap justify-end" style={spacing(palette).wrap}>
                  <Each of={diagnosticLevels}>
                    {(level) => (
                      <ThemeButton palette={palette} testID={`diagnostic-level-${level()}`}
                        label={t(`diagnostics.level.${level()}`)}
                        selected={props.appearance.diagnosticLevel() === level()} disabled={busy()}
                        onPress={() => void props.appearance.chooseDiagnosticLevel(level())} />
                    )}
                  </Each>
                </View>
              }
            >
              <AppKitPopup
                testID="diagnostic-level-native"
                style={{ width: "100%", height: 28 }}
                label={t("diagnostics.level")}
                items={diagnosticLevels.map((level) => ({ id: level, label: t(`diagnostics.level.${level}`) }))}
                selectedId={props.appearance.diagnosticLevel()}
                enabled={!busy()}
                onChange={({ id }) => {
                  if (isDiagnosticLevel(id)) void props.appearance.chooseDiagnosticLevel(id);
                }}
              />
            </If>
          </View>
        </View>
        <View class="flex-row items-center" style={rowLook(palette, { divider: true })}>
          <View class="min-w-0 flex-1" style={spacing(palette).text}>
            <Text style={{ color: palette().text }}>{t("diagnostics.range")}</Text>
            <Text style={noteLook(palette)}>{t("diagnostics.exportHint")}</Text>
            <If when={exportState().kind === "done"}>
              <Text testID="diagnostic-export-done" style={noteLook(palette, "text")}>
                {t("diagnostics.exported", { name: exportState().name ?? "" })}
              </Text>
            </If>
            <If when={exportState().kind === "error" && exportState().text}>
              <Text testID="diagnostic-export-error" style={noteLook(palette, "text")}>
                {t(exportState().text ?? "diagnostics.error.diagnostics_export_failed")}
              </Text>
            </If>
          </View>
          <View class="min-w-0 flex-1 flex-row flex-wrap items-center justify-end" style={spacing(palette).wrap}>
            <Each of={ranges}>
              {(item) => (
                <ThemeButton palette={palette} testID={`diagnostic-range-${item().id}`}
                  label={t(`diagnostics.range.${item().id}`)} selected={range() === item().id}
                  onPress={() => range.set(item().id)} />
              )}
            </Each>
            <ThemeButton palette={palette} prominent testID="diagnostic-export"
              label={t(exportState().kind === "running" ? "diagnostics.exporting" : "diagnostics.export")}
              disabled={exportState().kind === "running"}
              onPress={() => void props.appearance.exportDiagnostics(days())} />
          </View>
        </View>
      </View>
    </View>
  );
}
