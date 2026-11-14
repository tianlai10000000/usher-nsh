/** Presents output switches, a compact level picker and diagnostic export using theme controls. */
import { signal, Text, View } from "nsh";
import { InteractionSelect, ThemeSwitch } from "../../components/ValueControls";
import { AppKitPopup } from "../../components/AppKitControls";
import { ThemeButton } from "../../components/ThemeControls";
import { SystemIcon } from "../../components/SystemIcon";
import { diagnosticLevels, type AppState, type DiagnosticLevel, type DiagnosticOutput } from "../../state/app";
import type { MessageKey } from "../../i18n/index";
import { createSelectMotion } from "../../theme/motion";
import config from "../../config/diagnostics.json";
import { fieldLook, groupLook, headingLook, noteLook, rowLook, sectionTitleLook, selectLook, spacing } from "../../theme/look";

function Choice(props: { appearance: AppState; id: string; label: string; value: string; items: { id: string; label: string }[]; interactionBlocked?: boolean; change: (value: string) => void }) {
  const palette = props.appearance.appearance.palette;
  const popup = createSelectMotion(palette);
  const title = () => props.items.find(item => item.id === props.value)?.label ?? "";
  return <If when={palette().nativeAppKit} else={
    <InteractionSelect accessibilityLabel={props.label} interactionBlocked={props.interactionBlocked}
      items={props.items.map(item => ({ value: item.id, title: item.label }))} value={props.value}
      popupClass="border" popupStyle={{ ...selectLook(palette).popupStyle, ...popup.style }}
      onOpen={popup.show} onClose={popup.hide}
      itemClass={selectLook(palette).itemClass} itemStyle={selectLook(palette).itemStyle}
      onChange={({ value }) => { if (value !== null) props.change(value); }}
      trigger={<View testID={props.id} accessibilityLabel={props.label} class="flex-row items-center justify-between" style={fieldLook(palette).style} hoverStyle={fieldLook(palette).hoverStyle}>
        <Text style={{ color: palette().text }}>{title()}</Text>
        <SystemIcon name="chevron.up.chevron.down" fallback="⌄" size={palette().metric.fontNote} color={palette().muted} />
      </View>} />
  }>
    <AppKitPopup testID={`${props.id}-native`} style={{ width: "100%", height: palette().metric.buttonHeight }}
      label={props.label} items={props.items} selectedId={props.value} enabled interactionBlocked={props.interactionBlocked} onChange={({ id }) => props.change(id)} />
  </If>;
}

function OutputRow(props: { appearance: AppState; output: string; divider: boolean; available: boolean }) {
  const palette = props.appearance.appearance.palette;
  const t = props.appearance.preferences.t;
  const busy = props.appearance.settings.busy;
  const checked = () => props.available && props.appearance.diagnostics.diagnosticOutput(props.output as DiagnosticOutput);
  const label = () => t(`diagnostics.output.${props.output}` as MessageKey);
  return <View class="flex-row items-center" style={rowLook(palette, { divider: props.divider })}>
    <View class="min-w-0 flex-1" style={spacing(palette).text}>
      <Text style={{ color: palette().text }}>{label()}</Text>
      <Text style={{ ...noteLook(palette), numberOfLines: 0 }}>{t(`diagnostics.output.${props.output}.description` as MessageKey)}</Text>
      <If when={!props.available}><Text style={noteLook(palette)}>{t("diagnostics.unavailable")}</Text></If>
    </View>
    {/* Temporary save protection must not dim every connected output. */}
    <ThemeSwitch palette={palette} testID={`diagnostic-output-${props.output}`} accessibilityLabel={label()} checked={checked()}
      disabled={!props.available} interactionBlocked={busy()}
      onChange={({ checked }) => { if (props.available) void props.appearance.diagnostics.chooseDiagnosticOutput(props.output as DiagnosticOutput, checked); }} />
  </View>;
}

export function DiagnosticsSettings(props: { appearance: AppState }) {
  const palette = props.appearance.appearance.palette;
  const t = props.appearance.preferences.t;
  const busy = props.appearance.settings.busy;
  const exportState = props.appearance.diagnostics.exportState;
  const range = signal(config.initialRange);
  const days = () => config.ranges.find(item => item.id === range())!.days;
  const group = () => groupLook(palette, props.appearance.background.contentBackgroundTransparency());
  return <View testID="diagnostics-settings" style={spacing(palette).page}>
    <View style={spacing(palette).text}>
      <Text role="header" style={headingLook(palette)}>{t("diagnostics.title")}</Text>
      <Text style={{ ...noteLook(palette), numberOfLines: 0 }}>{t("diagnostics.description")}</Text>
      <If when={props.appearance.diagnostics.diagnosticProblem() !== null}>
        <Text testID="diagnostic-problem" style={{ ...noteLook(palette, "text"), numberOfLines: 0 }}>
          {t(props.appearance.diagnostics.diagnosticProblem() === "diagnostics_directory_unavailable" ? "diagnostics.error.diagnostics_directory_unavailable" : "diagnostics.error.diagnostics_write_failed")}
        </Text>
      </If>
      <If when={props.appearance.diagnostics.diagnosticDropped() > 0}>
        <Text testID="diagnostic-dropped" style={noteLook(palette)}>{t("diagnostics.dropped", { count: props.appearance.diagnostics.diagnosticDropped() })}</Text>
      </If>
    </View>
    <View style={spacing(palette).group}>
      <Text style={sectionTitleLook(palette)}>{t("diagnostics.outputs")}</Text>
      <View style={group()}>
        <Each of={config.outputs}>{(output, index) => <OutputRow appearance={props.appearance} output={output()} divider={index() > 0} available />}</Each>
      </View>
    </View>
    <View style={spacing(palette).group}>
      <Text style={sectionTitleLook(palette)}>{t("diagnostics.detail")}</Text>
      <View style={group()}>
        <View style={rowLook(palette)}>
          <Text style={{ color: palette().text }}>{t("diagnostics.level")}</Text>
          <Choice appearance={props.appearance} id="diagnostic-level-picker" label={t("diagnostics.level")} value={props.appearance.diagnostics.diagnosticLevel()} interactionBlocked={busy()}
            items={diagnosticLevels.map(level => ({ id: level, label: t(`diagnostics.level.${level}`) }))}
            change={value => { if ((diagnosticLevels as readonly string[]).includes(value)) void props.appearance.diagnostics.chooseDiagnosticLevel(value as DiagnosticLevel); }} />
          <Text testID="diagnostic-level-hint" style={{ ...noteLook(palette), numberOfLines: 0 }}>
            {t(props.appearance.diagnostics.diagnosticLevel() === "OFF" ? "diagnostics.offHint" : "diagnostics.levelHint")}
          </Text>
        </View>
      </View>
    </View>
    <View style={spacing(palette).group}>
      <Text style={sectionTitleLook(palette)}>{t("diagnostics.collectors")}</Text>
      <View style={group()}>
        <Each of={config.unavailable}>{(output, index) => <OutputRow appearance={props.appearance} output={output()} divider={index() > 0} available={false} />}</Each>
      </View>
    </View>
    <View style={spacing(palette).group}>
      <Text style={sectionTitleLook(palette)}>{t("diagnostics.exportTitle")}</Text>
      <View style={group()}>
        <View style={rowLook(palette)}>
          <Text style={{ ...noteLook(palette), numberOfLines: 0 }}>{t("diagnostics.exportHint")}</Text>
          <View class="flex-row flex-wrap items-center" style={spacing(palette).wrap}>
            <View class="min-w-0 flex-1">
              <Choice appearance={props.appearance} id="diagnostic-range-picker" label={t("diagnostics.range")} value={range()}
                items={config.ranges.map(item => ({ id: item.id, label: t(`diagnostics.range.${item.id}` as MessageKey) }))}
                change={value => { if (config.ranges.some(item => item.id === value)) range.set(value); }} />
            </View>
            <ThemeButton palette={palette} prominent testID="diagnostic-export" loadingFeedback label={t("diagnostics.export")}
              busy={exportState().kind === "running"}
              disabled={exportState().kind === "running"} onPress={() => props.appearance.diagnostics.exportDiagnostics(days())} />
          </View>
          <If when={exportState().kind === "done"}><Text testID="diagnostic-export-done" style={{ ...noteLook(palette, "text"), numberOfLines: 0 }}>{t("diagnostics.exported", { name: exportState().name ?? "" })}</Text></If>
          <If when={exportState().kind === "error" && exportState().text}><Text testID="diagnostic-export-error" style={{ ...noteLook(palette), numberOfLines: 0 }}>{t(exportState().text ?? "diagnostics.error.diagnostics_export_failed")}</Text></If>
        </View>
      </View>
    </View>
  </View>;
}
