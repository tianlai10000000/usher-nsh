/**
 * The contract between the frame layout and the update feature.
 *
 * The rail shows an update button and marks it when an update is available. The layout only
 * reads this projection and reports the press; the update state machine behind it lives
 * elsewhere, so the frame does not depend on how updates are checked or installed.
 */

export type UpdateIndicator = {
  /** Whether an update is ready to be installed; the rail button shows a badge while true. */
  available: () => boolean;
  /** The user pressed the update button. */
  request: () => void;
};
