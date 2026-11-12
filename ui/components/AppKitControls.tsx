import { nativeComponent } from "nsh";

type Choice = { id: string; label: string };
type ChoiceProps = {
  items: readonly Choice[];
  selectedId: string;
  enabled: boolean;
  label: string;
};
type ChoiceEvents = { change: { id: string } };

export const AppKitSegments = nativeComponent<ChoiceProps, ChoiceEvents>("usher.settings.segments", {
  props: ["items", "selectedId", "enabled", "label"],
  events: ["change"],
});

export const AppKitPopup = nativeComponent<ChoiceProps, ChoiceEvents>("usher.settings.popup", {
  props: ["items", "selectedId", "enabled", "label"],
  events: ["change"],
});

export const AppKitButton = nativeComponent<
  { label: string; enabled: boolean },
  { activate: object }
>("usher.settings.button", {
  props: ["label", "enabled"],
  events: ["activate"],
});


/** A vertical list of native buttons; the selected one shows pressed. Its height is `items * 32`. */
export const AppKitSidebar = nativeComponent<ChoiceProps, ChoiceEvents>("usher.settings.sidebar", {
  props: ["items", "selectedId", "enabled", "label"],
  events: ["change"],
});
