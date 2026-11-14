/** Separates business availability from temporary submission locks for all control adapters. */
export type InteractionProps = {
  /** Business unavailability retains the control's disabled appearance. */
  disabled?: boolean;
  /** Temporarily rejects interaction without dimming unrelated controls. */
  interactionBlocked?: boolean;
};

/** Read inside reactive bindings; the business executor must retain its own concurrency guard. */
export function controlInteraction(props: InteractionProps) {
  return {
    disabled: () => props.disabled === true || props.interactionBlocked === true,
    dimmed: () => props.disabled === true,
  };
}
