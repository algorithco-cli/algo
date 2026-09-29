export const FPS = 30;
export const T = 12; // cross-fade length between scenes (frames)

export const SCENES = {
  intro: 120,
  problem: 190,
  what: 220,
  pipeline: 240,
  demo: 340,
  usage: 340,
  cta: 220,
} as const;

export type SceneKey = keyof typeof SCENES;
const KEYS = Object.keys(SCENES) as SceneKey[];

export const START = {} as Record<SceneKey, number>;
{
  let acc = 0;
  for (const k of KEYS) {
    START[k] = acc;
    acc += SCENES[k] - T;
  }
}

export const TOTAL =
  KEYS.reduce((sum, k) => sum + SCENES[k], 0) - T * (KEYS.length - 1);

// ── X / square ad (1080x1080) ──
export const X_T = 8;
export const X_SCENES = {
  hook: 105,
  guard: 135,
  how: 165,
  trust: 105,
  cta: 165,
} as const;
export const X_START = {} as Record<keyof typeof X_SCENES, number>;
{
  let acc = 0;
  for (const k of Object.keys(X_SCENES) as Array<keyof typeof X_SCENES>) {
    X_START[k] = acc;
    acc += X_SCENES[k] - X_T;
  }
}
export const X_TOTAL =
  Object.values(X_SCENES).reduce((a, b) => a + b, 0) -
  X_T * (Object.keys(X_SCENES).length - 1);
