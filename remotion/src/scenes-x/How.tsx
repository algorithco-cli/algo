import React from 'react';
import {AbsoluteFill, interpolate, useCurrentFrame} from 'remotion';
import {C, F} from '../theme';
import {BlurWords, Code, VerdictPill, clamp, useFadeUp} from '../ui';

const NODES = [
  {t: 'Hook', b: 'Every tool call goes to a tiny client (~1ms).'},
  {t: 'Decide', b: 'Hard-deny → cache → ask. Errors always resolve to ask.'},
  {t: 'Audit', b: 'Every decision logged to ~/.algo/audit.db.'},
];
const W = 288;
const GAP = 36;
const center = (i: number) => i * (W + GAP) + W / 2;
const P0 = 46;
const P1 = 108;

const Node: React.FC<{i: number; t: string; b: string; appear: number}> = ({i, t, b, appear}) => {
  const frame = useCurrentFrame();
  const s = useFadeUp(appear, 16, 22);
  const at = P0 + ((P1 - P0) / 2) * i;
  const on = interpolate(frame, [at - 4, at + 6], [0, 1], clamp);
  const accent = i === 1;
  return (
    <div style={{...s, width: W}}>
      <div
        style={{
          height: 250,
          padding: 24,
          borderRadius: 20,
          background: C.surface,
          border: `1px solid ${on > 0.5 || accent ? C.brand : C.border}`,
          boxShadow: on > 0.5 ? `0 0 50px ${C.brand}44` : 'none',
          transform: `translateY(${-8 * on}px)`,
        }}
      >
        <div
          style={{
            width: 42,
            height: 42,
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
        <div style={{fontSize: 36, fontWeight: 700, marginTop: 14, letterSpacing: '-0.02em'}}>{t}</div>
        <div style={{fontSize: 22, lineHeight: 1.45, color: C.muted, marginTop: 8}}>{b}</div>
      </div>
    </div>
  );
};

export const XHow: React.FC = () => {
  const frame = useCurrentFrame();
  const x = interpolate(frame, [P0, P1], [center(0), center(2)], clamp);
  const railOn = interpolate(frame, [P0 - 12, P0], [0, 1], clamp);
  const denied = frame >= P1 + 4;
  const result = useFadeUp(P1 + 6, 16, 16);
  const foot = useFadeUp(P1 + 26, 16, 10);
  return (
    <AbsoluteFill style={{padding: '84px 72px', justifyContent: 'center', gap: 34}}>
      <div style={{fontSize: 24, fontWeight: 600, letterSpacing: '0.18em', textTransform: 'uppercase', color: C.brand, ...useFadeUp(0, 14, 8)}}>
        How it works
      </div>
      <BlurWords lines={[{text: 'Hook. Decide. Audit.', accent: true}]} fontSize={80} start={4} />

      <div style={{position: 'relative', width: 3 * W + 2 * GAP}}>
        {/* rail + traveling command */}
        <div style={{position: 'relative', height: 64, opacity: railOn, marginBottom: 6}}>
          <div style={{position: 'absolute', top: 44, left: center(0), width: center(2) - center(0), height: 4, background: C.border, borderRadius: 4}} />
          <div style={{position: 'absolute', top: 44, left: center(0), width: x - center(0), height: 4, background: denied ? C.deny : C.brand, borderRadius: 4}} />
          <div
            style={{
              position: 'absolute',
              top: 0,
              left: x,
              transform: 'translateX(-50%)',
              fontFamily: F.mono,
              fontSize: 22,
              padding: '6px 14px',
              borderRadius: 10,
              color: denied ? C.deny : C.text,
              background: C.surface,
              border: `1px solid ${denied ? C.deny : C.brand}`,
              boxShadow: `0 0 26px ${denied ? C.deny : C.brand}66`,
              whiteSpace: 'nowrap',
            }}
          >
            curl … | sh
          </div>
          <div style={{position: 'absolute', top: 36, left: x - 10, width: 20, height: 20, borderRadius: 99, background: denied ? C.deny : C.brandStrong, boxShadow: `0 0 22px ${denied ? C.deny : C.brand}`}} />
        </div>
        <div style={{display: 'flex', gap: GAP}}>
          {NODES.map((n, i) => (
            <Node key={n.t} i={i} {...n} appear={18 + i * 8} />
          ))}
        </div>
      </div>

      <div style={{...result, display: 'flex', alignItems: 'center', gap: 18}}>
        {denied ? <VerdictPill verdict="deny" size={28} /> : null}
        <Code size={26} color={C.deny}>DENY_CURL_PIPE_SH</Code>
        <span style={{fontSize: 24, color: C.muted}}>blocked before it runs</span>
      </div>
      <div style={{...foot, fontSize: 26, color: C.muted}}>
        Ask <span style={{fontFamily: F.mono, color: C.brandStrong}}>algo why</span> — action, reason, confidence, source, latency.
      </div>
    </AbsoluteFill>
  );
};
