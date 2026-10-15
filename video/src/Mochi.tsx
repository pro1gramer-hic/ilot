// Mochi drawn by îlot's own engine, one frame of the video at a time: the move's time is
// the video's, so every render gives the same Mochi.
import { useEffect, useRef } from "react";
import { useCurrentFrame, useVideoConfig } from "remotion";
import { drawMochi } from "../../src/lib/mochi/engine";
import type { Expression } from "../../src/lib/mochi/moves";
import { framePoseAt, type MochiMove } from "../../src/lib/mochi/poses";

type Props = {
  move: MochiMove;
  /** Width and height on screen, in pixels. */
  size: number;
  /** Frame of the video where the move starts. */
  from?: number;
  expression?: Expression;
  style?: React.CSSProperties;
};

export const Mochi = ({ move, size, from = 0, expression, style }: Props) => {
  const canvas = useRef<HTMLCanvasElement>(null);
  const frame = useCurrentFrame();
  const { fps } = useVideoConfig();
  const t = Math.max(0, frame - from) / fps;

  useEffect(() => {
    const ctx = canvas.current?.getContext("2d");
    if (!ctx) return;
    const params = { ...framePoseAt(move, t, expression), sphere: true };
    drawMochi(ctx, size, params, t * 1000);
  }, [move, size, t, expression]);

  return <canvas ref={canvas} width={size} height={size} style={{ width: size, height: size, ...style }} />;
};
