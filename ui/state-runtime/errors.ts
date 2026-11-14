/** Identifies malformed wire data without classifying domain command errors as binding failures. */
export class SnapshotDecodeError extends Error {
  constructor() { super("state_invalid_snapshot"); }
}
