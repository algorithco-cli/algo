# algorithco guard — website ad (Remotion)

A ~53 second, 1920×1080, 30 fps product ad that explains what algorithco guard is
and how to use it. Built from the real website copy, brand tokens (`design-tokens.css`)
and fonts (Inter + JetBrains Mono) in the `algorithco-guard` repo. Every npm
script regenerates the video palette from the root `design-tokens.css`, which
remains the single source of truth.

## Two videos in this project

| Composition | Size | Length | Use |
|---|---|---|---|
| `GuardAd` | 1920x1080 (16:9) | ~53s | website hero / landing page |
| `GuardX` | 1080x1080 (1:1) | ~21s | X (Twitter) feed |

`npm run build` renders the website ad; `npm run build:x` renders the X ad (`out/guard-x.mp4`);
`npm run still:x` renders a cover image (`out/guard-x-cover.png`).
The X ad is built to work with the sound off: every idea is on screen as big text.

## Run it

Requires Node 20+.

```bash
npm install
npm run dev        # opens Remotion Studio (live preview, scrub the timeline)
npm run build      # renders out/guard-ad.mp4  (H.264, good for websites)
```

Other outputs:

```bash
npm run build:webm   # out/guard-ad.webm (VP9)
npm run still        # out/thumbnail.png (poster frame)
```

The first render downloads Remotion's headless Chrome automatically (needs internet once).

## Storyboard

| Scene | Length | What it shows |
|---|---|---|
| Intro | 4.0s | "Ship agents you don't have to babysit." + logo |
| Problem | 6.3s | Agent runs `curl … \| sh` / `rm -rf /` with no checkpoint |
| What it is | 7.3s | Agents → guard → shell, allow / ask / deny, 4 pillars |
| How it works | 8.0s | Hook → Parse + Redact → Decide → Render + Audit |
| Live demo | 11.3s | 4 commands typed and judged (values from the site's own demo logic) |
| How to use | 11.3s | install → `algo init` → shadow mode → `algo why` / `algo enforce on` |
| CTA | 7.3s | Stats + "Start free." |

## Things to edit

- `src/config.ts` — **site URL** (the repo currently only has `http://127.0.0.1:3007`,
  the production domain is still "TBD"), CTA label, and whether to show the URL.
- `src/durations.ts` — scene lengths (change one number, the total updates itself).
- `src/scenes/*.tsx` — copy and layout, one file per scene.
- `src/theme.ts` — semantic aliases for colors generated from the repo's dark tokens.

## Notes

- The terminal outputs in the "How to use" scene (`algo status` numbers, `algo why`
  confidence/latency) are illustrative mock-ups, not captured from a real run.
- No audio is included. To add music: put an mp3 in `public/`, then in
  `src/Video.tsx` add `<Audio src={staticFile('music.mp3')} />` (imports from `remotion`).
