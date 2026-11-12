/** Presents the image picker, image opacity and shared content background transparency. */
import { Text, View } from "nsh";
import { Slider } from "nsh/base";
import type { AppearanceStore } from "../../state/appearance";
import { ThemeButton } from "../../components/ThemeControls";
import { groupLook, noteLook, rowLook, sectionTitleLook, sliderLook, spacing } from "../../theme/look";
import backgroundConfig from "../../config/background.json";

export function BackgroundSettings(props: { appearance: AppearanceStore }) {
  const { palette, t, busy } = props.appearance;
  const disabled = () => busy() || props.appearance.pickingBackground();
  const slider = () => sliderLook(palette);

  return (
    <View testID="background-settings" style={spacing(palette).group}>
      <Text role="header" style={sectionTitleLook(palette)}>{t("background.title")}</Text>
      <View style={groupLook(palette, props.appearance.contentBackgroundTransparency())}>
        <View class="flex-row flex-wrap items-center" style={rowLook(palette)}>
          <View class="min-w-0 flex-1" style={spacing(palette).text}>
            <Text style={{ color: palette().text }}>{t("background.image")}</Text>
            <Text style={noteLook(palette)}>{t("background.description")}</Text>
            <Text testID="background-image-name" style={noteLook(palette)}>
              {props.appearance.backgroundImage()?.name ?? t("background.none")}
            </Text>
            <If when={props.appearance.backgroundProblem()}>
              <Text testID="background-image-problem" style={noteLook(palette)}>
                {t(props.appearance.backgroundProblem()!)}
              </Text>
            </If>
          </View>
          <View class="flex-row flex-wrap justify-end" style={spacing(palette).wrap}>
            <ThemeButton palette={palette} testID="background-image-pick" label={t("background.choose")}
              disabled={disabled()} onPress={() => void props.appearance.chooseBackgroundImage()} />
            <ThemeButton palette={palette} testID="background-image-clear" label={t("background.clear")}
              disabled={disabled() || !props.appearance.backgroundImage()}
              onPress={() => void props.appearance.clearBackgroundImage()} />
          </View>
        </View>
        <View testID="background-image-opacity-control" class="flex-row flex-wrap items-center"
          style={rowLook(palette, { divider: true })}>
          <View class="min-w-0 flex-1" style={spacing(palette).text}>
            <Text style={{ color: palette().text }}>{t("background.opacity")}</Text>
            <Text style={noteLook(palette)}>{t("background.opacityDescription")}</Text>
          </View>
          <View class="min-w-0 flex-1" style={spacing(palette).text}>
            <Slider testID="background-image-opacity" accessibilityLabel={t("background.opacity")}
              value={props.appearance.backgroundImageOpacity()} disabled={disabled()}
              min={backgroundConfig.minOpacity} max={backgroundConfig.maxOpacity} step={backgroundConfig.opacityStep}
              trackStyle={slider().trackStyle}
              indicatorStyle={slider().indicatorStyle}
              rangeStyle={slider().rangeStyle}
              thumbClass="rounded-full focus-visible:outline focus-visible:outline-ring"
              thumbStyle={slider().thumbStyle}
              onChange={({ value }) => {
                if (typeof value === "number") props.appearance.previewBackgroundImageOpacity(value);
              }}
              onChangeEnd={({ value }) => {
                if (typeof value === "number") void props.appearance.setBackgroundImageOpacity(value);
              }} />
            <Text testID="background-image-opacity-value" style={noteLook(palette)}>
              {t("background.opacityValue", { value: props.appearance.backgroundImageOpacity() })}
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
            <Slider testID="content-background-transparency" accessibilityLabel={t("background.contentTransparency")}
              value={props.appearance.contentBackgroundTransparency()} disabled={disabled()}
              min={backgroundConfig.contentTransparency.min} max={backgroundConfig.contentTransparency.max}
              step={backgroundConfig.contentTransparency.step}
              trackStyle={slider().trackStyle}
              indicatorStyle={slider().indicatorStyle}
              rangeStyle={slider().rangeStyle}
              thumbClass="rounded-full focus-visible:outline focus-visible:outline-ring"
              thumbStyle={slider().thumbStyle}
              onChange={({ value }) => {
                if (typeof value === "number") props.appearance.previewContentBackgroundTransparency(value);
              }}
              onChangeEnd={({ value }) => {
                if (typeof value === "number") void props.appearance.setContentBackgroundTransparency(value);
              }} />
            <Text testID="content-background-transparency-value" style={noteLook(palette)}>
              {t("background.opacityValue", { value: props.appearance.contentBackgroundTransparency() })}
            </Text>
          </View>
        </View>
      </View>
    </View>
  );
}
