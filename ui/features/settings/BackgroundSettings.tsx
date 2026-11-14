/** Presents the image picker, image opacity and shared content background transparency. */
import { Text, View } from "nsh";
import { ThemeSlider } from "../../components/ValueControls";
import type { AppState } from "../../state/app";
import { ThemeButton } from "../../components/ThemeControls";
import { groupLook, noteLook, rowLook, sectionTitleLook, spacing } from "../../theme/look";
import backgroundConfig from "../../config/background.json";

export function BackgroundSettings(props: { appearance: AppState }) {
  const palette = props.appearance.appearance.palette;
  const t = props.appearance.preferences.t;
  const busy = props.appearance.settings.busy;
  const disabled = () => busy() || props.appearance.background.pickingBackground();

  return (
    <View testID="background-settings" style={spacing(palette).group}>
      <Text role="header" style={sectionTitleLook(palette)}>{t("background.title")}</Text>
      <View style={groupLook(palette, props.appearance.background.contentBackgroundTransparency())}>
        <View class="flex-row flex-wrap items-center" style={rowLook(palette)}>
          <View class="min-w-0 flex-1" style={spacing(palette).text}>
            <Text style={{ color: palette().text }}>{t("background.image")}</Text>
            <Text style={noteLook(palette)}>{t("background.description")}</Text>
            <Text testID="background-image-name" style={noteLook(palette)}>
              {props.appearance.background.backgroundImage()?.name ?? t("background.none")}
            </Text>
            <If when={props.appearance.background.backgroundProblem()}>
              <Text testID="background-image-problem" style={noteLook(palette)}>
                {t(props.appearance.background.backgroundProblem()!)}
              </Text>
            </If>
          </View>
          <View class="flex-row flex-wrap justify-end" style={spacing(palette).wrap}>
            <ThemeButton palette={palette} testID="background-image-pick" loadingFeedback label={t("background.choose")}
              busy={props.appearance.background.pickingBackground()}
              interactionBlocked={disabled()} onPress={() => props.appearance.background.chooseBackgroundImage()} />
            <ThemeButton palette={palette} testID="background-image-clear" loadingFeedback label={t("background.clear")}
              interactionBlocked={disabled()} disabled={!props.appearance.background.backgroundImage()}
              onPress={() => props.appearance.background.clearBackgroundImage()} />
          </View>
        </View>
        <View testID="background-image-opacity-control" class="flex-row flex-wrap items-center"
          style={rowLook(palette, { divider: true })}>
          <View class="min-w-0 flex-1" style={spacing(palette).text}>
            <Text style={{ color: palette().text }}>{t("background.opacity")}</Text>
            <Text style={noteLook(palette)}>{t("background.opacityDescription")}</Text>
          </View>
          <View class="min-w-0 flex-1" style={spacing(palette).text}>
            <ThemeSlider palette={palette} testID="background-image-opacity" accessibilityLabel={t("background.opacity")}
              value={props.appearance.background.backgroundImageOpacity()} interactionBlocked={disabled()}
              min={backgroundConfig.minOpacity} max={backgroundConfig.maxOpacity} step={backgroundConfig.opacityStep}
              onChange={({ value }) => {
                if (typeof value === "number") props.appearance.background.previewBackgroundImageOpacity(value);
              }}
              onChangeEnd={({ value }) => {
                if (typeof value === "number") void props.appearance.background.setBackgroundImageOpacity(value);
              }} />
            <Text testID="background-image-opacity-value" style={noteLook(palette)}>
              {t("background.opacityValue", { value: props.appearance.background.backgroundImageOpacity() })}
            </Text>
          </View>
        </View>
        <View testID="content-background-transparency-control" class="flex-row flex-wrap items-center"
          style={rowLook(palette, { divider: true })}>
          <View class="min-w-0 flex-1" style={spacing(palette).text}>
            <Text style={{ color: palette().text }}>{t("background.contentTransparency")}</Text>
            <Text style={noteLook(palette)}>{t("background.contentTransparencyDescription")}</Text>
          </View>
          <View class="min-w-0 flex-1" style={spacing(palette).text}>
            <ThemeSlider palette={palette} testID="content-background-transparency" accessibilityLabel={t("background.contentTransparency")}
              value={props.appearance.background.contentBackgroundTransparency()} interactionBlocked={disabled()}
              min={backgroundConfig.contentTransparency.min} max={backgroundConfig.contentTransparency.max}
              step={backgroundConfig.contentTransparency.step}
              onChange={({ value }) => {
                if (typeof value === "number") props.appearance.background.previewContentBackgroundTransparency(value);
              }}
              onChangeEnd={({ value }) => {
                if (typeof value === "number") void props.appearance.background.setContentBackgroundTransparency(value);
              }} />
            <Text testID="content-background-transparency-value" style={noteLook(palette)}>
              {t("background.opacityValue", { value: props.appearance.background.contentBackgroundTransparency() })}
            </Text>
          </View>
        </View>
      </View>
    </View>
  );
}
