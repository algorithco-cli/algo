import React from 'react';
import {interpolate, useCurrentFrame} from 'remotion';
import {C, F} from '../theme';
import {BlurWords, Kicker, Pill, SceneFrame, clamp, useFadeUp} from '../ui';

const STAGES = [
  {
    title: 'Hook',
    sub: 'fires first',
    body: "Your agent's hook sends every tool call to a tiny client (~1ms). Daemon down? It asks — never blocks you.",
  },
  {
    title: 'Parse + Redact',
    sub: 'on-machine',
    body: 'Tree-sitter parses the shell, the redactor masks secrets, fingerprinting prepares the cache lookup.',
  },
  {
    title: 'Decide',
    sub: 'L0 / L1 / L3 / L4',
    body: 'Hard-deny → cache (24h) → Jev judgment only if you opted in → ask. Errors resolve to ask.',
    accent: true,
  },
  {
    title: 'Render + Audit',
    sub: 'logged',
    body: 'The adapter replies approve / block / ask. Everything lands in ~/.algo/audit.db.',
  },
];

const ROW = 1680;
const GAP = 32;
const W = (ROW - GAP * 3) / 4;
const center = (i: number) => i * (W + GAP) + W / 2;
const P0 = 46;
const P1 = 176;

export const Pipeline: React.FC = () => {
  const frame = useCurrentFrame();
  const x = interpolate(frame, [P0, P1], [center(0), center(3)], clamp);
  const railOn = interpolate(frame, [P0 - 10, P0], [0, 1], clamp);
  const band = useFadeUp(196, 18, 16);
  return (
    <SceneFrame>
      <Kicker>How it works</Kicker>
      <BlurWords
        lines={[{text: 'One pipeline,'}, {text: 'milliseconds.', accent: true}]}
        fontSize={72}
        start={6}
      />

      <div style={{position: 'relative', width: ROW, marginTop: 46}}>
        {/* rail */}
        <div style={{position: 'relative', height: 28, opacity: railOn}}>
          <div
            style={{
              position: 'absolute',
              top: 12,
              left: center(0),
              width: center(3) - center(0),
              height: 4,
              background: C.border,
              borderRadius: 4,
            }}
          />
          <div
            style={{
              position: 'absolute',
              top: 12,
              left: center(0),
              width: x - center(0),
              height: 4,
              background: C.brand,
              borderRadius: 4,
            }}
          />
          <div
            style={{
              position: 'absolute',
              top: 4,
              left: x - 10,
              width: 20,
              height: 20,
              borderRadius: 99,
              background: C.brandStrong,
              boxShadow: `0 0 24px ${C.brand}`,
            }}
          />
        </div>

        <div style={{display: 'flex', gap: GAP, marginTop: 8}}>
          {STAGES.map((s, i) => {
            const at = P0 + ((P1 - P0) / 3) * i;
            return <Stage key={s.title} i={i} {...s} at={at} appear={20 + i * 8} />;
          })}
        </div>
      </div>

      <div
        style={{
          ...band,
          marginTop: 36,
          display: 'flex',
          alignItems: 'center',
          gap: 22,
          fontSize: 24,
          color: C.muted,
        }}
      >
        <span style={{fontFamily: F.mono}}>
          local <b style={{color: C.text}}>p50 &lt;3ms</b> · p99 &lt;10ms
        </span>
        <span>·</span>
        <span style={{fontFamily: F.mono}}>
          Jev (opt-in) p50 &lt;250ms
        </span>
        <span style={{flex: 1}} />
        <Pill color={C.ask} size={22}>
          any error → ask. never allow.
        </Pill>
      </div>
    </SceneFrame>
  );
};

const Stage: React.FC<{
  i: number;
  title: string;
  sub: string;
  body: string;
  accent?: boolean;
  at: number;
  appear: number;
}> = ({i, title, sub, body, accent, at, appear}) => {
  const frame = useCurrentFrame();
  const s = useFadeUp(appear, 18, 24);
  const on = interpolate(frame, [at - 4, at + 8], [0, 1], clamp);
  const border = accent ? C.brand : C.border;
  return (
    <div style={{...s, width: W}}>
      <div
        style={{
          height: 372,
          padding: 28,
          borderRadius: 20,
          background: C.surface,
          border: `1px solid ${on > 0.5 || accent ? border : C.border}`,
          boxShadow: on > 0.5 ? `0 0 ${accent ? 70 : 36}px ${C.brand}${accent ? '55' : '2a'}` : 'none',
          transform: `translateY(${-8 * on}px)`,
        }}
      >
        <div
          style={{
            width: 44,
            height: 44,
            borderRadius: 99,
            display: 'grid',
            placeItems: 'center',
            fontFamily: F.mono,
            fontWeight: 700,
            fontSize: 22,
            color: on > 0.5 ? C.bg : C.muted,
            background: on > 0.5 ? C.brandStrong : C.raised,
            border: `1px solid ${C.border}`,
          }}
        >
          {i + 1}
        </div>
        <div style={{fontSize: 36, fontWeight: 700, marginTop: 20, letterSpacing: '-0.02em'}}>{title}</div>
        <div style={{fontFamily: F.mono, fontSize: 19, color: C.brandStrong, marginTop: 4}}>{sub}</div>
        <div style={{fontSize: 23, lineHeight: 1.5, color: C.muted, marginTop: 18}}>{body}</div>
      </div>
    </div>
  );
};
