/** Exercises production animation lifetimes and public NSH overlay transitions without persistence. */
import { cleanup, colorScheme, mount, signal, Text, View } from "nsh";
import { Button, Popover, Progress, Select, Sheet, Slider, Switch, Tooltip } from "nsh/base";
import { resolveTheme } from "../../../ui/theme";
import { galleryTheme } from "./gallery-theme";
import { buttonLook, createProgressLook, createSwitchLook, fieldLook, popupLook, sliderLook } from "../../../ui/theme/look";
import { createSelectMotion, followSpring, popupMotion } from "../../../ui/theme/motion";

function Preview() {
  const palette = () => resolveTheme(galleryTheme, colorScheme() === "dark" ? "dark" : "light");
  const checked = signal(false);
  const mounted = signal(true);
  const indeterminate = signal(true);
  const progressValue = signal(25);
  const popupOpen = signal(false);
  const popupClosed = signal(0);
  const sheetOpen = signal(false);
  const selected = signal("first");
  const samples = signal("");
  const measured = signal(0);
  let probe: ReturnType<typeof followSpring> | undefined;
  let timers: ReturnType<typeof setTimeout>[] = [];
  const clearTimers = () => { timers.forEach(clearTimeout); timers = []; };
  cleanup(clearTimers);
  function SpringControl() {
    const look = createSwitchLook(palette, checked);
    // The probe uses the same lifecycle helper; no per-frame JS drives either animated value.
    probe = followSpring(() => checked() ? 1 : 0, () => palette().motion.switchSpring);
    return <Switch testID="motion-switch" checked={checked()} onChange={({ checked: next }) => checked.set(next)}
      trackStyle={look.trackStyle} thumbStyle={look.thumbStyle} />;
  }
  function ProgressControl() {
    const look = createProgressLook(palette, progressValue, indeterminate);
    return <Progress testID="motion-progress" value={progressValue()} indeterminate={indeterminate()}
      onLayout={look.onLayout} trackStyle={look.trackStyle} indicatorStyle={look.indicatorStyle} />;
  }
  const select = createSelectMotion(palette);
  const popup = () => popupMotion(palette, "bottom");
  const tip = () => popupMotion(palette, "top", true);
  function sampleSpring() {
    clearTimers();
    samples.set("");
    checked.set(true);
    const owner = probe;
    for (const delay of [32, 96, 240, 800]) {
      timers.push(setTimeout(() => { void owner?.getValue().then(value => samples.update(text => `${text}${delay}:${value};`)); }, delay));
    }
  }
  const actionStyle = () => buttonLook(palette).style;
  return <View style={{ padding: palette().metric.cardPad, gap: palette().metric.stackGap }}>
    <If when={mounted()}><SpringControl /><ProgressControl /></If>
    <Text>{`checked:${checked()} mounted:${mounted()} selected:${selected()}`}</Text>
    <Text>{`samples:${samples()}`}</Text><Text>{`measured:${measured()}`}</Text>
    <Button testID="toggle-value" style={actionStyle()} onPress={() => checked.set(!checked())}><Text>Toggle value</Text></Button>
    <Button testID="sample-spring" style={actionStyle()} onPress={sampleSpring}><Text>Sample spring</Text></Button>
    <Button testID="read-spring" style={actionStyle()} onPress={async () => { if (probe) measured.set(await probe.getValue()); }}><Text>Read spring</Text></Button>
    <Button testID="toggle-mount" style={actionStyle()} onPress={() => mounted.set(!mounted())}><Text>Mount controls</Text></Button>
    <Button testID="toggle-progress" style={actionStyle()} onPress={() => { indeterminate.set(!indeterminate()); progressValue.set(75); }}><Text>Progress mode</Text></Button>
    <Slider testID="motion-slider" value={progressValue()} onChange={({ value }) => { if (typeof value === "number") progressValue.set(value); }}
      trackClass={sliderLook(palette).trackClass} trackStyle={sliderLook(palette).trackStyle}
      indicatorStyle={sliderLook(palette).indicatorStyle} rangeStyle={sliderLook(palette).rangeStyle}
      thumbClass={sliderLook(palette).thumbClass} thumbStyle={sliderLook(palette).thumbStyle} />
    <Popover open={popupOpen()} onOpenChange={({ open }) => popupOpen.set(open)}
      popupStyle={{ ...popupLook(palette), ...popup().style, padding: palette().metric.cardPad }}
      popupStartingStyle={popup().starting} popupEndingStyle={popup().ending}
      onClose={() => popupClosed.update(value => value + 1)}
      trigger={<Button testID="popup-trigger" style={actionStyle()}><Text>Popup</Text></Button>}>
      <Button testID="popup-close" style={actionStyle()} onPress={() => popupOpen.set(false)}><Text>Close popup</Text></Button>
    </Popover>
    <Text>{`popup-closed:${popupClosed()}`}</Text>
    <Button testID="reopen-popup" style={actionStyle()} onPress={() => popupOpen.set(true)}><Text>Reopen popup</Text></Button>
    <Button testID="tip-trigger" style={actionStyle()} tooltip={<Tooltip testID="motion-tooltip"
      style={{ ...popupLook(palette), ...tip().style }} startingStyle={tip().starting} endingStyle={tip().ending}><Text>Motion hint</Text></Tooltip>}><Text>Hint</Text></Button>
    <Select testID="motion-select" value={selected()} items={[{ value: "first", title: "First item" }, { value: "second", title: "Second item" }]}
      onChange={({ value }) => { if (value) selected.set(value); }}
      popupStyle={{ ...popupLook(palette), ...select.style }} onOpen={select.show} onClose={select.hide}
      trigger={<View testID="select-trigger" style={fieldLook(palette).style}><Text>Choose item</Text></View>} />
    <Button testID="sheet-trigger" style={actionStyle()} onPress={() => sheetOpen.set(true)}><Text>Sheet</Text></Button>
    <Sheet testID="motion-sheet" side="left" open={sheetOpen()} onOpenChange={({ open }) => sheetOpen.set(open)}
      popupStyle={{ width: palette().metric.menuWidth, backgroundColor: palette().surface }} backdropStyle={{ backgroundColor: palette().scrim }}>
      <Button testID="sheet-close" onPress={() => sheetOpen.set(false)}><Text>Close sheet</Text></Button>
    </Sheet>
  </View>;
}
mount(Preview);
