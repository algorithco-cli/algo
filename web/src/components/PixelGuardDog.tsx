/**
 * PixelGuardDog — pixel guard dog (security mascot).
 *
 * Usage:
 *   import PixelGuardDog from "./PixelGuardDog";
 *   <PixelGuardDog size={260} />
 *   <PixelGuardDog size={160} alert accent="#8e77ff" onBark={() => console.log("woof")} />
 *
 * Props:
 *   size    — width (px), default 240
 *   alert   — true keeps the dog "alert" (ears up, bright eyes, "!" mark)
 *   accent  — collar color
 *   label   — speech bubble ("WOOF!") and aria-label text
 *   onBark  — called when clicked
 *
 * Animations (CSS only, no React re-render):
 *   breathing, blinking, ear twitches, tail wagging.
 *   Hover/focus: alert state. Click: bark.
 *   prefers-reduced-motion is respected.
 *
 * NOTE: hex values below are the pixel-sprite palette (fixed artwork, not the
 * Variant 1 theme) — exempted in src/lib/site.test.ts like the other
 * component-local visuals.
 */
import { memo, useCallback, useEffect, useMemo, useRef, useState } from "react";
import type * as React from "react";

/* ---------- Palette ---------- */
const PALETTE: Record<string, string> = {
  o: "#2d3250", // outline
  w: "#ffffff", // white fur
  l: "#e3e9f4", // light shade
  s: "#c3cde1", // deep shade
  n: "#1c1f33", // nose
  m: "#1c1f33", // mouth
  p: "#f5a8b8", // inner ear / tongue
  y: "#fbbf24", // shield
  k: "#1e3a8a", // shield mark
};

interface Rect {
  x: number;
  y: number;
  w: number;
  c: string;
}

/* ---------- Helpers ---------- */
const mirror = (half: string[]): string[] =>
  half.map((row) => row + [...row].reverse().join(""));

// Light shade on white pixels touching the outline
const shade = (grid: string[]): string[] =>
  grid.map((row, y) =>
    [...row]
      .map((ch, x) => {
        if (ch !== "w") return ch;
        const near =
          row[x - 1] === "o" || row[x + 1] === "o" || grid[y + 1]?.[x] === "o";
        return near ? "l" : ch;
      })
      .join(""),
  );

// Merge horizontal runs of the same color into single rects
const toRects = (
  grid: string[],
  ox: number,
  oy: number,
  flipX = false,
  pal: Record<string, string> = PALETTE,
): Rect[] => {
  const out: Rect[] = [];
  grid.forEach((raw, r) => {
    const row = flipX ? [...raw].reverse().join("") : raw;
    const baseX = flipX ? 32 - ox - raw.length : ox;
    let x = 0;
    while (x < row.length) {
      const ch = row[x];
      if (ch === "." || !pal[ch]) {
        x++;
        continue;
      }
      let end = x;
      while (end + 1 < row.length && row[end + 1] === ch) end++;
      out.push({ x: baseX + x, y: oy + r, w: end - x + 1, c: pal[ch] });
      x = end + 1;
    }
  });
  return out;
};

/* ---------- Sprite data (dog coords: 0..31) ---------- */
const HEAD = shade(
  [
    ".......ooooooooo",
    ".....oowwwwwwwww",
    "....owwwwwwwwwww",
    "....owwwwwwwwwww",
    "....owwwwwwwwwww",
    "....owwwwwwwwwww",
    "....owwwwwwwwwww",
    "....owwwwwwwssss",
    "....owwwwwwswnnn",
    "....owwwwwswwwnn",
    "....owwwwwswwwwm",
    "....owwwwwswmwwm",
    "....owwwwwwswmmm",
    "....owwwwwwwssss",
    ".....owwwwwwwwww",
    "......oooooooooo",
  ].map((row) => row + [...row].reverse().join("")),
);

const EAR = shade([
  "..oo..",
  ".owwo.",
  ".owwwo",
  "owpwwo",
  "owpwwo",
  "owpwwo",
  "owwwwo",
  "owwwww",
]);

const BODY = shade(
  [
    "........owwwwwww",
    "........owwwwwww",
    ".......owwwwwwww",
    "......owwwwwwwww",
    ".....owwwwwwwwww",
    ".....owwwwwwwwww",
    "....owwwwwwwwwww",
    "....owwwwwwwwwww",
    "....owwwwwwwwwww",
    "....owwwwswwwwwo",
    "...owwwwswwwwwwo",
    "...owwwswwwwswwo",
    "...ooooooooooooo",
  ].map((row) => row + [...row].reverse().join("")),
);

// Collar (b = light, c = base). Base is replaced with `accent`.
const COLLAR = [".......obbbbbbbb", ".......occcccccc", ".......ooooooooo"].map(
  (row) => row + [...row].reverse().join(""),
);

const SHIELD = [
  "oooooooo",
  "oyyyyyko",
  "oyyyykyo",
  "oykykyyo",
  "oyykyyyo",
  ".oyyyyo.",
  "..oyyo..",
  "...oo...",
];

const TIPS: Record<string, string[]> = {
  mid: ["...oo.", "..owwo", "..owwo", ".owwwo"],
  right: ["....oo", "...owo", "..owwo", ".owwwo"],
  left: ["..oo..", ".owwo.", ".owwo.", ".owwwo"],
};
const TAIL_BASE = [".owwwo", ".owwo.", "owwwo.", "owwo..", "owwo..", "oooo.."];
const TAIL_ORDER = ["mid", "right", "mid", "left"];

/* Static layers computed once */
const R = {
  head: toRects(HEAD, 0, 6),
  earL: toRects(EAR, 4, 0),
  earR: toRects(EAR, 4, 0, true),
  body: toRects(BODY, 0, 22),
  shield: toRects(SHIELD, 12, 23),
  tail: TAIL_ORDER.map((t) =>
    toRects(shade([...TIPS[t], ...TAIL_BASE]), 28, 25),
  ),
};

const Px = memo(function Px({ rects }: { rects: Rect[] }): JSX.Element {
  return (
    <>
      {rects.map((r) => (
        <rect
          key={`${r.x},${r.y},${r.w},${r.c}`}
          x={r.x}
          y={r.y}
          width={r.w}
          height={1}
          fill={r.c}
        />
      ))}
    </>
  );
});

/* ---------- Styles ---------- */
const CSS = `
.pd-root{--pd-wag:.8s;--pd-eye:#1b1e2e;position:relative;display:inline-block;padding:0;border:0;background:none;cursor:pointer;line-height:0;-webkit-tap-highlight-color:transparent}
.pd-root[data-alert="true"]{--pd-wag:.4s;--pd-eye:#38bdf8}
.pd-root[data-bark="true"]{--pd-wag:.25s}
.pd-root:focus-visible{outline:3px solid #38bdf8;outline-offset:6px;border-radius:6px}
.pd-svg{display:block;width:100%;height:auto;image-rendering:pixelated;overflow:visible}

.pd-head{animation:pd-breath 1.8s infinite}
.pd-root[data-bark="true"] .pd-head{animation:pd-shake .1s 6}
@keyframes pd-breath{0%,49.99%{transform:translateY(0)}50%,100%{transform:translateY(1px)}}
@keyframes pd-shake{0%{transform:translateX(0)}25%{transform:translateX(-1px)}75%{transform:translateX(1px)}100%{transform:translateX(0)}}

.pd-eyes{transform-box:fill-box;transform-origin:center;animation:pd-blink 4.6s infinite}
@keyframes pd-blink{0%,93%{transform:scaleY(1)}94%,97%{transform:scaleY(.2)}98%,100%{transform:scaleY(1)}}

.pd-ears{transition:transform 0s}
.pd-root[data-alert="true"] .pd-ears{transform:translateY(-1px)}
.pd-ear-l{animation:pd-twitch 5.3s infinite}
.pd-ear-r{animation:pd-twitch 7.1s 1.7s infinite}
@keyframes pd-twitch{0%,92%{transform:translateY(0)}93%,96%{transform:translateY(1px)}97%,100%{transform:translateY(0)}}

.pd-frame{visibility:hidden;animation:pd-frame var(--pd-wag) infinite}
@keyframes pd-frame{0%{visibility:visible}25%,100%{visibility:hidden}}

.pd-mouth{visibility:hidden}
.pd-root[data-bark="true"] .pd-mouth{visibility:visible}

.pd-excl{opacity:0}
.pd-root[data-alert="true"] .pd-excl{opacity:1;animation:pd-hop .5s steps(1) infinite}
@keyframes pd-hop{0%{transform:translateY(0)}50%{transform:translateY(-1px)}}

.pd-bubble{position:absolute;top:2%;left:76%;z-index:1;padding:.3em .55em;background:#fff;color:#2d3250;border:2px solid #2d3250;box-shadow:3px 3px 0 #2d3250;font:800 var(--pd-fs,14px)/1 ui-monospace,"SF Mono",Menlo,Consolas,monospace;letter-spacing:.04em;white-space:nowrap;opacity:0;pointer-events:none;transform:translateY(4px)}
.pd-root[data-bark="true"] .pd-bubble{opacity:1;transform:translateY(0)}

@media (prefers-reduced-motion:reduce){
  .pd-head,.pd-eyes,.pd-ear-l,.pd-ear-r,.pd-excl{animation:none!important}
  .pd-frame{animation:none}
  .pd-frame:first-child{visibility:visible}
}
`;

/* ---------- Component ---------- */
export interface PixelGuardDogProps {
  size?: number;
  alert?: boolean;
  accent?: string;
  label?: string;
  onBark?: () => void;
}

export default function PixelGuardDog({
  size = 240,
  alert = false,
  accent = "#8e77ff",
  label = "WOOF!",
  onBark,
}: PixelGuardDogProps): JSX.Element {
  const [hover, setHover] = useState(false);
  const [barking, setBarking] = useState(false);
  const timer = useRef<ReturnType<typeof setTimeout> | null>(null);

  useEffect(
    () => () => {
      if (timer.current) clearTimeout(timer.current);
    },
    [],
  );

  const bark = useCallback(() => {
    setBarking(true);
    if (timer.current) clearTimeout(timer.current);
    timer.current = setTimeout(() => setBarking(false), 700);
    onBark?.();
  }, [onBark]);

  const isAlert = alert || hover || barking;
  const collarRects = useMemo(
    () =>
      toRects(COLLAR, 0, 21, false, { ...PALETTE, c: accent, b: mix(accent) }),
    [accent],
  );

  return (
    <button
      type="button"
      className="pd-root"
      aria-label={`Guard dog. ${label}`}
      data-alert={isAlert}
      data-bark={barking}
      style={
        {
          width: size,
          "--pd-fs": `${Math.max(10, size * 0.065)}px`,
        } as React.CSSProperties
      }
      onClick={bark}
      onPointerEnter={() => setHover(true)}
      onPointerLeave={() => setHover(false)}
      onFocus={() => setHover(true)}
      onBlur={() => setHover(false)}
    >
      <style>{CSS}</style>
      <span className="pd-bubble" aria-hidden="true">
        {label}
      </span>

      <svg
        className="pd-svg"
        viewBox="-2 0 36 37"
        shapeRendering="crispEdges"
        role="presentation"
        aria-hidden="true"
      >
        {/* ground shadow */}
        <rect x="4" y="35" width="24" height="1" fill="#2d3250" opacity=".16" />
        <rect x="8" y="36" width="16" height="1" fill="#2d3250" opacity=".16" />

        {/* tail (4 frames) */}
        <g>
          {R.tail.map((rects, i) => (
            <g
              // biome-ignore lint/suspicious/noArrayIndexKey: keyframes are static and never reorder
              key={i}
              className="pd-frame"
              style={{ animationDelay: `calc(var(--pd-wag) * ${i / 4})` }}
            >
              <Px rects={rects} />
            </g>
          ))}
        </g>

        <Px rects={R.body} />

        {/* head */}
        <g className="pd-head">
          <g className="pd-ears">
            <g className="pd-ear-l">
              <Px rects={R.earL} />
            </g>
            <g className="pd-ear-r">
              <Px rects={R.earR} />
            </g>
          </g>
          <Px rects={R.head} />

          <g className="pd-eyes" fill="var(--pd-eye)">
            <rect x="9" y="9" width="2" height="4" />
            <rect x="21" y="9" width="2" height="4" />
          </g>
          <rect x="9" y="9" width="1" height="1" fill="#fff" />
          <rect x="21" y="9" width="1" height="1" fill="#fff" />

          {/* barking mouth */}
          <g className="pd-mouth">
            <rect x="12" y="17" width="8" height="2" fill={PALETTE.m} />
            <rect x="13" y="19" width="6" height="1" fill={PALETTE.m} />
            <rect x="14" y="20" width="4" height="1" fill={PALETTE.p} />
          </g>
        </g>

        {/* collar and shield */}
        <Px rects={collarRects} />
        <Px rects={R.shield} />

        {/* "!" mark */}
        <g className="pd-excl" fill="#f43f5e">
          <rect x="15" y="0" width="2" height="3" />
          <rect x="15" y="4" width="2" height="1" />
        </g>
      </svg>
    </button>
  );
}

// Lightens a color a bit (hex -> hex)
function mix(hex: string): string {
  const n = Number.parseInt(hex.replace("#", ""), 16);
  const ch = (v: number): number =>
    Math.min(255, Math.round(v + (255 - v) * 0.35));
  const r = ch((n >> 16) & 255);
  const g = ch((n >> 8) & 255);
  const b = ch(n & 255);
  return `#${[r, g, b].map((v) => v.toString(16).padStart(2, "0")).join("")}`;
}
