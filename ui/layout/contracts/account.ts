/**
 * The contract between the frame layout and the account feature.
 *
 * The rail's avatar button reads the signed-in user's display name and picture and reports the
 * press. Sign-in and profile screens live elsewhere; the frame only shows who is signed in.
 */

export type AccountEntry = {
  /** Display name for the avatar's initials and accessibility label, or null when signed out. */
  name: () => string | null;
  /** Picture URL, or null to show the fallback. */
  picture: () => string | null;
  /** The user pressed the avatar. */
  open: () => void;
};
