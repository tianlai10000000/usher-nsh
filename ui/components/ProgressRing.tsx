/** Draws a themed progress ring through the public Canvas API, respecting reduced motion. */
import { Canvas, reduceMotion } from "nsh";
import type { ThemeAccessor } from "../theme/index";

/** A null progress denotes indeterminate work; numeric progress is a fraction from zero to one. */
export function ProgressRing(props: {
  palette: ThemeAccessor;
  size: number;
  progress: number | null;
  label?: string;
  testID?: string;
}) {
  return (
    <Canvas
      testID={props.testID}
      role="progressbar"
      accessibilityLabel={props.label}
      style={{
        width: props.size,
        height: props.size,
        flexShrink: 0,
        animationName: props.progress === null && !reduceMotion() ? "spin" : "none",
        animationDuration: props.palette().metric.updateSpinMs,
        animationTimingFunction: "linear",
        animationIterationCount: "infinite",
      }}
      draw={(cx, size) => {
        const palette = props.palette();
        const radius = (Math.min(size.width, size.height) - palette.metric.borderWidth * 2) / 2;
        cx.lineWidth = palette.metric.borderWidth * 2;
        cx.lineCap = "round";
        cx.beginPath();
        cx.strokeStyle = palette.border;
        cx.arc(size.width / 2, size.height / 2, radius, 0, Math.PI * 2);
        cx.stroke();
        cx.beginPath();
        cx.strokeStyle = palette.muted;
        cx.arc(
          size.width / 2,
          size.height / 2,
          radius,
          -Math.PI / 2,
          -Math.PI / 2 + (props.progress ?? palette.metric.updateSegment) * Math.PI * 2,
        );
        cx.stroke();
      }}
    />
  );
}
