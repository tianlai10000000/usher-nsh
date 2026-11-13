/** Renders production component looks without persistence and exposes their resolved style contract. */
import { colorScheme, mount, signal, Text, View } from "nsh";
import { Button, Input, Progress, Slider, Switch } from "nsh/base";
import { galleryTheme } from "./gallery-theme";
import { resolveTheme } from "../../../ui/theme";
import { buttonLook, fieldLook, popupLook, progressLook, sliderLook, switchLook, tabLook } from "../../../ui/theme/look";

function Preview() {
  const accent = signal<string | null>(null);
  const checked = signal(true);
  const value = signal(50);
  const palette = () => resolveTheme(galleryTheme, colorScheme() === "dark" ? "dark" : "light", { accent: accent() });
  const contract = () => ({
    switch: switchLook(palette, checked()), slider: sliderLook(palette),
    progress: progressLook(palette), field: fieldLook(palette),
    selected: buttonLook(palette, { selected: true }), primary: buttonLook(palette, { kind: "primary" }),
    tab: tabLook(palette), popup: popupLook(palette),
  });
  return <View style={{ padding: palette().metric.cardPad, gap: palette().metric.stackGap, backgroundColor: palette().background }}>
    <Text style={{ color: palette().text }}>NSH Gallery component finishes</Text>
    <Switch testID="sample-switch" checked={checked()} onChange={event => checked.set(event.checked)}
      trackStyle={switchLook(palette, checked()).trackStyle} thumbStyle={switchLook(palette, checked()).thumbStyle} />
    <Slider testID="sample-slider" value={value()} onChange={event => { if (typeof event.value === "number") value.set(event.value); }}
      trackStyle={sliderLook(palette).trackStyle} indicatorStyle={sliderLook(palette).indicatorStyle}
      rangeStyle={sliderLook(palette).rangeStyle} thumbStyle={sliderLook(palette).thumbStyle} />
    <Progress value={value()} trackStyle={progressLook(palette).trackStyle} indicatorStyle={progressLook(palette).indicatorStyle} />
    <Input value="Gallery input" style={fieldLook(palette).style} />
    <Button style={buttonLook(palette, { selected: true }).style}><Text style={buttonLook(palette, { selected: true }).label}>Selected neutral button</Text></Button>
    <Button style={buttonLook(palette, { kind: "primary" }).style}><Text style={buttonLook(palette, { kind: "primary" }).label}>Primary button</Text></Button>
    <View style={{ ...popupLook(palette), padding: palette().metric.cardPad }}><Text style={{ color: palette().popoverText }}>Popup surface</Text></View>
    <Button testID="override-accent" onPress={() => accent.set("#b42318")}><Text>Override accent</Text></Button>
    <Text style={{ numberOfLines: 1 }}>{`contract:${JSON.stringify(contract())}`}</Text>
  </View>;
}
mount(Preview);
