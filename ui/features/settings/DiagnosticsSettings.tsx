/** Presents output switches, a compact level picker and diagnostic export using theme controls. */
import { signal, Text, View } from "nsh";
import { Select, Switch } from "nsh/base";
import { AppKitPopup } from "../../components/AppKitControls";
import { ThemeButton } from "../../components/ThemeControls";
import { SystemIcon } from "../../components/SystemIcon";
import { diagnosticLevels, type AppearanceStore, type DiagnosticLevel, type DiagnosticOutput } from "../../state/appearance";
import type { MessageKey } from "../../i18n";
import config from "../../config/diagnostics.json";
import { fieldLook, groupLook, headingLook, noteLook, rowLook, sectionTitleLook, selectLook, spacing, switchLook } from "../../theme/look";

function Choice(props: { appearance: AppearanceStore; id: string; label: string; value: string; items: { id: string; label: string }[]; disabled?: boolean; change: (value: string) => void }) {
  const { palette } = props.appearance;
  const title = () => props.items.find(item => item.id === props.value)?.label ?? "";
  return <If when={palette().nativeAppKit} else={
    <Select accessibilityLabel={props.label} disabled={props.disabled}
      items={props.items.map(item => ({ value: item.id, title: item.label }))} value={props.value}
      popupClass="border" popupStyle={selectLook(palette).popupStyle}
      itemClass={selectLook(palette).itemClass} itemStyle={selectLook(palette).itemStyle}
      onChange={({ value }) => { if (value !== null) props.change(value); }}
      trigger={<View testID={props.id} accessibilityLabel={props.label} class="flex-row items-center justify-between" style={fieldLook(palette).style} hoverStyle={fieldLook(palette).hoverStyle}>
        <Text style={{ color: palette().text }}>{title()}</Text>
        <SystemIcon name="chevron.up.chevron.down" fallback="⌄" size={palette().metric.fontNote} color={palette().muted} />
      </View>} />
  }>
    <AppKitPopup testID={`${props.id}-native`} style={{ width: "100%", height: palette().metric.buttonHeight }}
      label={props.label} items={props.items} selectedId={props.value} enabled={!props.disabled} onChange={({ id }) => props.change(id)} />
  </If>;
}

function OutputRow(props: { appearance: AppearanceStore; output: string; divider: boolean; available: boolean }) {
  const { palette, t, busy } = props.appearance;
  const checked = () => props.available && props.appearance.diagnosticOutput(props.output as DiagnosticOutput);
  const label = () => t(`diagnostics.output.${props.output}` as MessageKey);
  return <View class="flex-row items-center" style={rowLook(palette, { divider: props.divider })}>
    <View class="min-w-0 flex-1" style={spacing(palette).text}>
      <Text style={{ color: palette().text }}>{label()}</Text>
      <Text style={{ ...noteLook(palette), numberOfLines: 0 }}>{t(`diagnostics.output.${props.output}.description` as MessageKey)}</Text>
      <If when={!props.available}><Text style={noteLook(palette)}>{t("diagnostics.unavailable")}</Text></If>
    </View>
    {/* Temporary save protection must not dim every connected output. */}
    <Switch testID={`diagnostic-output-${props.output}`} accessibilityLabel={label()} checked={checked()}
      disabled={!props.available || busy()} disabledStyle={props.available ? undefined : switchLook(palette, checked()).disabledStyle}
      trackStyle={switchLook(palette, checked()).trackStyle} thumbStyle={switchLook(palette, checked()).thumbStyle}
      focusStyle={switchLook(palette, checked()).focusStyle}
      onChange={({ checked }) => { if (props.available) void props.appearance.chooseDiagnosticOutput(props.output as DiagnosticOutput, checked); }} />
  </View>;
}

export function DiagnosticsSettings(props: { appearance: AppearanceStore }) {
  const { palette, t, busy, exportState } = props.appearance;
  const range = signal(config.initialRange);
  const days = () => config.ranges.find(item => item.id === range())!.days;
  const group = () => groupLook(palette, props.appearance.contentBackgroundTransparency());
  return <View testID="diagnostics-settings" style={spacing(palette).page}>
    <View style={spacing(palette).text}>
      <Text role="header" style={headingLook(palette)}>{t("diagnostics.title")}</Text>
      <Text style={{ ...noteLook(palette), numberOfLines: 0 }}>{t("diagnostics.description")}</Text>
      <If when={props.appearance.diagnosticProblem() !== null}>
        <Text testID="diagnostic-problem" style={{ ...noteLook(palette, "text"), numberOfLines: 0 }}>
          {t(props.appearance.diagnosticProblem() === "diagnostics_directory_unavailable" ? "diagnostics.error.diagnostics_directory_unavailable" : "diagnostics.error.diagnostics_write_failed")}
        </Text>
      </If>
      <If when={props.appearance.diagnosticDropped() > 0}>
        <Text testID="diagnostic-dropped" style={noteLook(palette)}>{t("diagnostics.dropped", { count: props.appearance.diagnosticDropped() })}</Text>
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
          <Choice appearance={props.appearance} id="diagnostic-level-picker" label={t("diagnostics.level")} value={props.appearance.diagnosticLevel()} disabled={busy()}
            items={diagnosticLevels.map(level => ({ id: level, label: t(`diagnostics.level.${level}`) }))}
            change={value => { if ((diagnosticLevels as readonly string[]).includes(value)) void props.appearance.chooseDiagnosticLevel(value as DiagnosticLevel); }} />
          <Text testID="diagnostic-level-hint" style={{ ...noteLook(palette), numberOfLines: 0 }}>
            {t(props.appearance.diagnosticLevel() === "OFF" ? "diagnostics.offHint" : "diagnostics.levelHint")}
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
            <ThemeButton palette={palette} prominent testID="diagnostic-export" label={t(exportState().kind === "running" ? "diagnostics.exporting" : "diagnostics.export")}
              disabled={exportState().kind === "running"} onPress={() => void props.appearance.exportDiagnostics(days())} />
          </View>
          <If when={exportState().kind === "done"}><Text testID="diagnostic-export-done" style={{ ...noteLook(palette, "text"), numberOfLines: 0 }}>{t("diagnostics.exported", { name: exportState().name ?? "" })}</Text></If>
          <If when={exportState().kind === "error" && exportState().text}><Text testID="diagnostic-export-error" style={{ ...noteLook(palette), numberOfLines: 0 }}>{t(exportState().text ?? "diagnostics.error.diagnostics_export_failed")}</Text></If>
        </View>
      </View>
    </View>
  </View>;
}
