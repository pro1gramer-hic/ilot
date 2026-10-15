// îlot's demo, about 55 s: Mochi says hello, the island takes shape, then each thing it
// does, recorded from the browser preview with fake data, and the name to finish.
import { loadFont } from "@remotion/google-fonts/Inter";
import { Video } from "@remotion/media";
import { AbsoluteFill, Easing, interpolate, Sequence, spring, staticFile, useCurrentFrame, useVideoConfig } from "remotion";
import { Mochi } from "./Mochi";

const { fontFamily } = loadFont("normal", { weights: ["500", "600", "700"], subsets: ["latin", "latin-ext"] });

// îlot's theme (src/lib/tokens.css): one violet accent on a near-black ground.
const BG = "#08080b";
const INK = "#f5f5f7";
const INK_2 = "#9a9aa5";
const ACCENT = "#7c56ff";
const ACCENT_TEXT = "#a48fff";
const EASE_OUT = Easing.bezier(0.16, 1, 0.3, 1);
/** The island's spring, close to SPRINGS.island: lively, settles without a wobble. */
const SPRING = { damping: 18, stiffness: 140, mass: 1 };

/** Frames shared by two scenes while one fades into the next. */
const OVERLAP = 10;

/** A part of a recording, in the preview's CSS pixels (the recordings are at twice that). */
type Crop = { x: number; y: number; w: number; h: number };

type Scene =
  | { kind: "intro" | "walk" | "end"; frames: number }
  | { kind: "clip"; frames: number; src: string; crop: Crop; trim?: number; title: string };

const SCENES: Scene[] = [
  { kind: "intro", frames: 105 },
  { kind: "clip", frames: 135, src: "salut", crop: { x: 310, y: 0, w: 360, h: 90 }, title: "Une petite île en haut de ton écran." },
  { kind: "clip", frames: 175, src: "agents", crop: { x: 180, y: 0, w: 620, h: 125 }, trim: 0.2, title: "Ce que font tes agents, sans changer de fenêtre." },
  { kind: "clip", frames: 240, src: "live", crop: { x: 125, y: 0, w: 730, h: 490 }, trim: 0.3, title: "Chaque modification, ligne par ligne." },
  { kind: "clip", frames: 205, src: "permission", crop: { x: 190, y: 0, w: 600, h: 250 }, trim: 0.4, title: "Tu décides. îlot n'autorise jamais seul." },
  { kind: "clip", frames: 215, src: "message", crop: { x: 210, y: 0, w: 560, h: 250 }, trim: 0.4, title: "Réponds sans quitter ce que tu fais." },
  { kind: "clip", frames: 170, src: "music", crop: { x: 220, y: 0, w: 540, h: 120 }, trim: 0.3, title: "Ta musique, toujours à portée." },
  { kind: "walk", frames: 165 },
  { kind: "clip", frames: 155, src: "pastilles", crop: { x: 125, y: 0, w: 730, h: 490 }, trim: 0.3, title: "GitHub, Vercel, Stripe… en un coup d'œil." },
  { kind: "end", frames: 150 },
];

const starts = SCENES.reduce<number[]>((list, scene, i) => [...list, i === 0 ? 0 : list[i - 1] + SCENES[i - 1].frames - OVERLAP], []);
export const DEMO_FRAMES = starts.at(-1)! + SCENES.at(-1)!.frames;

const clamp = { extrapolateLeft: "clamp", extrapolateRight: "clamp" } as const;

/** Fades a scene in and out over the overlap, so that two scenes cross. */
const Fade = ({ frames, children }: { frames: number; children: React.ReactNode }) => {
  const frame = useCurrentFrame();
  return <AbsoluteFill style={{ opacity: interpolate(frame, [0, OVERLAP, frames - OVERLAP, frames], [0, 1, 1, 0], clamp) }}>{children}</AbsoluteFill>;
};

const Backdrop = () => (
  <AbsoluteFill
    style={{
      background: `radial-gradient(1400px 900px at 85% -10%, rgba(124, 92, 255, 0.16) 0%, transparent 60%),
        radial-gradient(1200px 800px at 0% 110%, rgba(66, 133, 244, 0.07) 0%, transparent 60%), ${BG}`,
    }}
  />
);

/** The scene's line, rising into place once the picture has settled. */
const Caption = ({ title, delay = 10, top }: { title: string; delay?: number; top: number }) => {
  const frame = useCurrentFrame();
  return (
    <div
      style={{
        position: "absolute",
        top,
        left: 0,
        right: 0,
        textAlign: "center",
        fontFamily,
        color: INK,
        fontSize: 64,
        fontWeight: 600,
        letterSpacing: "-0.025em",
        opacity: interpolate(frame, [delay, delay + 14], [0, 1], { ...clamp, easing: EASE_OUT }),
        translate: interpolate(frame, [delay, delay + 20], ["0px 18px", "0px 0px"], { ...clamp, easing: EASE_OUT }),
      }}
    >
      {title}
    </div>
  );
};

/** A recorded scene: the part of the preview that matters, framed, with its caption. */
const Clip = ({ scene }: { scene: Extract<Scene, { kind: "clip" }> }) => {
  const frame = useCurrentFrame();
  const { fps } = useVideoConfig();
  const { crop } = scene;
  // The frame fits in a 1560 × 700 box above the caption.
  const scale = Math.min(1560 / (crop.w * 2), 700 / (crop.h * 2));
  const width = crop.w * 2 * scale;
  const height = crop.h * 2 * scale;
  const enter = spring({ frame, fps, config: SPRING });
  return (
    <AbsoluteFill>
      <div
        style={{
          position: "absolute",
          left: (1920 - width) / 2,
          top: 90 + (700 - height) / 2,
          width,
          height,
          overflow: "hidden",
          borderRadius: 16,
          boxShadow: "0 40px 120px rgba(0, 0, 0, 0.6)",
          opacity: interpolate(enter, [0, 1], [0, 1]),
          scale: interpolate(enter, [0, 1], [0.94, 1]),
          translate: interpolate(enter, [0, 1], ["0px 30px", "0px 0px"]),
        }}
      >
        <div
          style={{
            position: "absolute",
            left: -crop.x * 2 * scale,
            top: -crop.y * 2 * scale,
            width: 1960 * scale,
            height: 1440 * scale,
            transformOrigin: `${(crop.x + crop.w / 2) * 2 * scale}px ${(crop.y + crop.h / 2) * 2 * scale}px`,
            scale: interpolate(frame, [0, scene.frames], [1, 1.035], clamp),
          }}
        >
          <Video src={staticFile(`raw/${scene.src}.mp4`)} trimBefore={Math.round((scene.trim ?? 0) * fps)} muted style={{ width: "100%", height: "100%" }} />
        </div>
      </div>
      <Caption title={scene.title} top={850} />
    </AbsoluteFill>
  );
};

/** Mochi pops up and waves, then shrinks toward the top of the screen, where the island is. */
const Intro = ({ frames }: { frames: number }) => {
  const frame = useCurrentFrame();
  const { fps } = useVideoConfig();
  const pop = spring({ frame, fps, config: { damping: 12, stiffness: 160, mass: 0.9 } });
  const leave = spring({ frame: frame - (frames - 28), fps, config: SPRING });
  return (
    <AbsoluteFill>
      <div
        style={{
          position: "absolute",
          left: 960 - 170,
          top: 230,
          scale: interpolate(pop, [0, 1], [0.4, 1]) * interpolate(leave, [0, 1], [1, 0.22]),
          translate: interpolate(leave, [0, 1], ["0px 0px", "0px -330px"]),
          opacity: interpolate(pop, [0, 0.4], [0, 1], clamp),
        }}
      >
        <Mochi move="hello" size={340} from={10} />
      </div>
      <div style={{ opacity: interpolate(leave, [0, 0.5], [1, 0], clamp) }}>
        <Caption title="Salut, moi c'est Mochi." top={640} delay={14} />
      </div>
    </AbsoluteFill>
  );
};

/** Mochi leaves the island, grows as he drops to the desktop, hops along, then waves. */
const Walk = () => {
  const frame = useCurrentFrame();
  const { fps } = useVideoConfig();
  const drop = spring({ frame: frame - 14, fps, config: { damping: 11, stiffness: 120, mass: 1 } });
  const along = interpolate(frame, [44, 120], [0, 1], { ...clamp, easing: Easing.inOut(Easing.cubic) });
  const size = 230;
  // He starts in the pill's slot, at its size, and lands on the desktop's ground line.
  const x = interpolate(along, [0, 1], [interpolate(drop, [0, 1], [690, 960 - size / 2]), 1380 - size / 2]);
  const y = interpolate(drop, [0, 1], [-96, 516]);
  const walking = frame >= 44 && frame < 120;
  const move = frame < 44 ? "fly" : walking ? "bounce" : "hello";
  return (
    <AbsoluteFill style={{ fontFamily }}>
      <div
        style={{
          position: "absolute",
          left: 960 - 200,
          top: 60,
          width: 400,
          height: 84,
          borderRadius: 42,
          background: "#121218",
          boxShadow: "0 20px 60px rgba(0, 0, 0, 0.5)",
          display: "flex",
          alignItems: "center",
          justifyContent: "center",
          gap: 26,
          color: INK_2,
          fontSize: 30,
          fontWeight: 500,
        }}
      >
        <span style={{ marginLeft: 40 }}>En veille</span>
        <span style={{ display: "flex", gap: 10 }}>
          {["#d97757", "#4285f4", "#10a37f"].map((color) => (
            <i key={color} style={{ width: 12, height: 12, borderRadius: 6, background: color, opacity: 0.4 }} />
          ))}
        </span>
      </div>
      <div style={{ position: "absolute", left: x, top: y, scale: interpolate(drop, [0, 1], [0.42, 1]), transformOrigin: "50% 100%" }}>
        <Mochi move={move} size={size} from={move === "hello" ? 120 : move === "bounce" ? 44 : 0} expression={walking ? { look: 1 } : undefined} />
      </div>
      <div style={{ position: "absolute", left: 160, right: 160, top: 742, height: 2, background: "linear-gradient(90deg, transparent, rgba(255,255,255,0.08), transparent)" }} />
      <Caption title="Il sort se balader sur ton bureau." top={850} />
    </AbsoluteFill>
  );
};

/** Mochi and the name, then what îlot is in one line. */
const End = () => {
  const frame = useCurrentFrame();
  const { fps } = useVideoConfig();
  const name = spring({ frame: frame - 6, fps, config: SPRING });
  return (
    <AbsoluteFill style={{ alignItems: "center", justifyContent: "center", fontFamily }}>
      <Mochi move="smile" size={250} from={4} />
      <div
        style={{
          color: INK,
          fontSize: 170,
          fontWeight: 700,
          letterSpacing: "-0.04em",
          lineHeight: 1,
          marginTop: 10,
          opacity: name,
          translate: interpolate(name, [0, 1], ["0px 26px", "0px 0px"]),
        }}
      >
        îlot
      </div>
      <div style={{ color: INK_2, fontSize: 38, fontWeight: 500, marginTop: 30, opacity: interpolate(frame, [20, 36], [0, 1], clamp) }}>
        Tes agents IA, tes messages et ta musique, dans une petite île.
      </div>
      <div style={{ display: "flex", gap: 14, marginTop: 40, opacity: interpolate(frame, [34, 50], [0, 1], clamp) }}>
        {["Windows", "Open source", "Licence MIT"].map((label) => (
          <span key={label} style={{ color: ACCENT_TEXT, fontSize: 24, fontWeight: 600, padding: "10px 22px", borderRadius: 999, background: "rgba(124, 92, 255, 0.14)", border: `1px solid ${ACCENT}55` }}>
            {label}
          </span>
        ))}
      </div>
    </AbsoluteFill>
  );
};

export const Demo = () => {
  const { fps } = useVideoConfig();
  return (
    <AbsoluteFill style={{ background: BG }}>
      <Backdrop />
      {SCENES.map((scene, i) => (
        <Sequence key={i} from={starts[i]} durationInFrames={scene.frames} premountFor={fps}>
          <Fade frames={scene.frames}>
            {scene.kind === "clip" ? <Clip scene={scene} /> : scene.kind === "intro" ? <Intro frames={scene.frames} /> : scene.kind === "walk" ? <Walk /> : <End />}
          </Fade>
        </Sequence>
      ))}
    </AbsoluteFill>
  );
};
