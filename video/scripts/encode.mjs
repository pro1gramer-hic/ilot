// Turns each recorded scene (JPEG frames with their timestamps, see record.mjs) into a
// 30 frames/s H.264 clip next to it, with the ffmpeg that ships with Remotion.
//   node video/scripts/encode.mjs

import { execFileSync } from "node:child_process";
import { readdirSync, readFileSync, writeFileSync } from "node:fs";
import { join } from "node:path";

const RAW = "video/public/raw";
const FFMPEG = "video/node_modules/@remotion/compositor-win32-x64-msvc/ffmpeg.exe";

for (const entry of readdirSync(RAW, { withFileTypes: true })) {
  if (!entry.isDirectory()) continue;
  const dir = join(RAW, entry.name);
  const frames = JSON.parse(readFileSync(join(dir, "frames.json"), "utf8"));
  const lines = frames.flatMap((frame, i) => {
    const next = frames[i + 1];
    return [`file '${frame.file}'`, `duration ${next ? (next.t - frame.t).toFixed(4) : "0.5"}`];
  });
  lines.push(`file '${frames.at(-1).file}'`);
  writeFileSync(join(dir, "concat.txt"), lines.join("\n"));
  const out = join(RAW, `${entry.name}.mp4`);
  execFileSync(FFMPEG, [
    "-v", "error", "-y", "-f", "concat", "-safe", "0", "-i", join(dir, "concat.txt"),
    "-r", "30", "-fps_mode", "cfr", "-pix_fmt", "yuv420p", "-c:v", "libx264", "-crf", "14", "-preset", "slow", out,
  ]);
  console.log(out);
}
